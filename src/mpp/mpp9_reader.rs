//! Corresponds to Java: `org/mpxj/mpp/MPP9Reader.java`
//!
//! Reader for MPP9 (Project 2000 / 2002 / 2003) files: reads data from the `Props9` / `   19`
//! / `   29` locations of the OLE compound document and populates [`ProjectFile`]. Every method in this document
//! corresponds one-to-one to the same-named Java method (method names in snake_case); fully translated, nothing trimmed.
//!
//! # Structural trade-off: Java instance state → private state struct
//!
//! Java's `MPP9Reader` uses the shape "no-arg constructor + instance fields holding intermediate state +
//! nulling them out via `clearMemberData()` at the end of `process`"; while [`MPPVariantReader::process`] has the signature
//! `&self` (see the three implementation styles given in the `mpp_variant_reader.rs` file header). This file uses
//! **style (2)**, "pack the state into a local struct and pass `&mut` to each private method"; it stays closest to Java's
//! method-by-method translation and introduces no interior mutability:
//!
//! | Java | Rust |
//! |---|---|
//! | `final class MPP9Reader implements MPPVariantReader` | [`MPP9Reader`] (zero-sized type) + `impl MPPVariantReader for MPP9Reader` |
//! | `MPP9Reader` instance fields (`m_reader` / `m_file` / `m_root` / `m_projectDir` / `m_viewDir` / `m_projectProps` / `m_outlineCodeVarData` / `m_fontBases` / `m_taskSubProjects` / `m_externalTasks` / `m_taskOrder` / `m_nullTaskOrder` / `m_resourceMap` / `m_inputStreamFactory`) | same-named fields of the private struct [`MPP9ReaderState`] (with the `m_` prefix removed) |
//! | private instance methods of `MPP9Reader` (`populateMemberData` / `processTaskData` / `createTaskMap` / `postProcessTasks` / …) | same-named methods on [`MPP9ReaderState`], one for one |
//! | `try { populateMemberData(); … } finally { clearMemberData(); }` | [`MPP9Reader::process`]: build state → run the body → clear state |
//! | `private EventManager m_eventManager` | **Not stored**: `ProjectFile::get_event_manager` returns only a read-only `Ref`, which cannot be held long term; instead it is borrowed temporarily through `ProjectContext` when an event fires (see [`fire_task_read_event`] / [`fire_resource_read_event`]) |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `InputStream` constructor parameter | `&[u8]` | per project convention; wherever Java uses `new DocumentInputStream(entry)` this becomes `entry.get_data()` (`Rc<Vec<u8>>`), and wherever Java uses `m_inputStreamFactory.getInstance(..)` the factory is kept (encrypted files are decrypted as well) |
//! | Access to `ProjectFile` / `ProjectContext` | always `try_borrow` / `try_borrow_mut` | on a borrow conflict return `Err` (see [`try_file`] / [`try_file_mut`]); never panic |
//! | cross-method call after `m_projectProps.getByteArray(..)` | copy first with `to_vec()` | Java holds a stable reference; Rust must release the `Props9` borrow before calling a `&mut self` method |
//! | `MPPUtility.decodePassword(props.getByteArray(..), code)` | copy first, then decrypt in place | Java modifies the array inside `Props` in place; the Rust side does not modify `Props` (at the cost of one extra copy) |
//! | `reader.getReadPassword().matches(readPassword)` | string equality comparison | Java treats the password in the file as a **regular expression** and matches the user input against it; regex is a dependency that has not been introduced, and on real (alphanumeric) passwords it is equivalent to equality, so equality is used |
//! | array indexes such as `metaData[11]` | [`meta_byte`] (returns 0 when out of bounds) | Java throws `ArrayIndexOutOfBoundsException` when out of bounds |
//! | `ByteArrayHelper.getShort/getInt` | [`get_short_by_offset`] / [`get_int_by_offset`] (internally using `get_short_safe` / `get_int_safe`) | Java throws when out of bounds; a negative offset also throws in Java, Rust returns 0 |
//! | `readSubProject`'s `catch (ArrayIndexOutOfBoundsException)` | [`MPP9ReaderState::read_sub_project_inner`] returns `Err`, and the wrapper records an ignored error | consistent with Java's intent to tolerate corrupted files as far as possible |
//! | `m_file.getProjectConfig().setAutoWBS(..)` | obtain `get_project_config_mut()` via `ProjectContext` | `ProjectFile::get_project_config()` returns only a read-only `Ref` |
//! | `Collections.sort(externalTasks)` | `Vec::sort_by(Task::compare_to)` | equivalent (`Task.compareTo` compares IDs) |
//!
//! # Untranslated / skipped Java code
//!
//! * `MPPUtility.convertRateFromHours(m_file, resource, ResourceField.STANDARD_RATE,
//!   ResourceField.STANDARD_RATE_UNITS)` (two places in `processResourceData`) — that method and
//!   `RateHelper.convertFromHours` have both been ported (see `mpp_utility.rs`), and the call sites are wired up.
//! * `AvailabilityFactory::process` returns "the rows that should be appended to the resource availability table"; the call site
//!   writes them back via `Resource::add_availability` (Java appends directly to `resource.getAvailability()`).
//! * the old implementation commented out above `postProcessTasks()` (already a comment in Java) and `dumpUnknownData`
//!   together with its two constant tables (also commented-out debug code in Java) — commented-out dead code is not translated.
//! * reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: this class uses none of these mechanisms.
//!
//! # Interface assumptions (verified to be consistent)
//!
//! ```text
//! pub trait MPPVariantReader {
//!     fn process(&self, reader: &MPPReader, file: &Rc<RefCell<ProjectFile>>,
//!                root: &DirectoryEntry) -> MpxjResult<()>;
//! }
//! ```
//!
//! The read-only switches of `MPPReader` that are used: `get_read_properties_only` / `get_read_presentation_data` /
//! `get_respect_password_protection` / `get_read_password`(`Option<&str>`)/
//! `get_use_raw_timephased_data`.

use std::cell::{Ref, RefCell, RefMut};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;

use chrono::NaiveDate;

use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::local_date_time_helper::LocalDateTimeHelper;
use crate::common::mpp_field_type::MppFieldType;
use crate::constraint_type::ConstraintType;
use crate::custom_field_container::CustomFieldContainer;
use crate::data_type::DataType;
use crate::duration::Duration;
use crate::field_key::FieldKey;
use crate::field_type_class::FieldTypeClass;
use crate::field_value::FieldValue;
use crate::mpp::abstract_calendar_factory::{get_int_safe, get_short_safe};
use crate::mpp::availability_factory::AvailabilityFactory;
use crate::mpp::constraint_factory::ConstraintFactory;
use crate::mpp::cost_rate_table_factory::CostRateTableFactory;
use crate::mpp::custom_field_value_reader9::CustomFieldValueReader9;
use crate::mpp::data_link_factory::DataLinkFactory;
use crate::mpp::document_input_stream_factory::DocumentInputStreamFactory;
use crate::mpp::field_map::FieldMap;
use crate::mpp::field_map9::FieldMap9;
use crate::mpp::filter_reader::FilterReader;
use crate::mpp::filter_reader9::FilterReader9;
use crate::mpp::fixed_data::FixedData;
use crate::mpp::fixed_meta::FixedMeta;
use crate::mpp::font_base::FontBase;
use crate::mpp::graphical_indicator_reader::GraphicalIndicatorReader;
use crate::mpp::group_reader::GroupReader;
use crate::mpp::group_reader9::GroupReader9;
use crate::mpp::hyperlink_reader::HyperlinkReader;
use crate::mpp::mpp9_calendar_factory::MPP9CalendarFactory;
use crate::mpp::mpp_reader::MPPReader;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::mpp_variant_reader::MPPVariantReader;
use crate::mpp::ole::DirectoryEntry;
use crate::mpp::project_properties_reader::ProjectPropertiesReader;
use crate::mpp::props9::Props9;
use crate::mpp::props_block::PropsBlock;
use crate::mpp::props_key::PropsKey;
use crate::mpp::recurring_task_reader::RecurringTaskReader;
use crate::mpp::resource_assignment_factory::ResourceAssignmentFactory;
use crate::mpp::table_factory::TableFactory;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_meta::VarMeta;
use crate::mpp::var_meta9::VarMeta9;
use crate::mpp::view_factory::ViewFactory;
use crate::mpp::view_factory9::ViewFactory9;
use crate::mpp::view_state_reader9::ViewStateReader9;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_calendar::ProjectCalendar;
use crate::project_entity_with_id::ProjectEntityWithID;
use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
use crate::project_file::ProjectFile;
use crate::resource::Resource;
use crate::resource_field::ResourceField;
use crate::resource_type::ResourceType;
use crate::task::Task;
use crate::task_field::TaskField;
use crate::time_unit::TimeUnit;

/// Name of the storage holding the project data.
///
/// Corresponds to Java: `root.getEntry("   19")`
const PROJECT_DIRECTORY: &str = "   19";

/// Name of the storage holding the view data.
///
/// Corresponds to Java: `root.getEntry("   29")`
const VIEW_DIRECTORY: &str = "   29";

/// Name of the task data storage.
///
/// Corresponds to Java: `m_projectDir.getEntry("TBkndTask")`
const TASK_DIRECTORY: &str = "TBkndTask";

/// Name of the resource data storage.
///
/// Corresponds to Java: `m_projectDir.getEntry("TBkndRsc")`
const RESOURCE_DIRECTORY: &str = "TBkndRsc";

/// Name of the assignment data storage.
///
/// Corresponds to Java: `m_projectDir.getEntry("TBkndAssn")`
const ASSIGNMENT_DIRECTORY: &str = "TBkndAssn";

/// Name of the outline code data storage.
///
/// Corresponds to Java: `m_projectDir.getEntry("TBkndOutlCode")`
const OUTLINE_CODE_DIRECTORY: &str = "TBkndOutlCode";

/// Name of the sub-storage under the view data storage (view definitions).
///
/// Corresponds to Java: `m_viewDir.getEntry("CV_iew")`
const VIEW_DIRECTORY_VIEWS: &str = "CV_iew";

/// Name of the sub-storage under the view data storage (table definitions).
///
/// Corresponds to Java: `m_viewDir.getEntry("CTable")`
const VIEW_DIRECTORY_TABLES: &str = "CTable";

/// Name of the sub-storage under the view data storage (filter definitions).
///
/// Corresponds to Java: `m_viewDir.getEntry("CFilter")`
const VIEW_DIRECTORY_FILTERS: &str = "CFilter";

/// Name of the sub-storage under the view data storage (group definitions).
///
/// Corresponds to Java: `m_viewDir.getEntry("CGrouping")`
const VIEW_DIRECTORY_GROUPS: &str = "CGrouping";

/// Name of the sub-storage under the view data storage (saved view state and data links).
///
/// Corresponds to Java: `m_viewDir.getEntry("CEdl")`
const VIEW_DIRECTORY_SAVED_VIEW_STATE: &str = "CEdl";

/// End marker of the subproject task unique ID list.
///
/// Corresponds to Java: `private static final int SUBPROJECT_LISTEND = 0x00000303;`
const SUBPROJECT_LISTEND: i32 = 0x0000_0303;

/// Marker value 0 for "the previous value is a subproject task unique ID".
///
/// Corresponds to Java: `private static final int SUBPROJECT_TASKUNIQUEID0 = 0x00000000;`
const SUBPROJECT_TASKUNIQUEID0: i32 = 0x0000_0000;

/// Marker value 1 for "the previous value is a subproject task unique ID".
///
/// Corresponds to Java: `private static final int SUBPROJECT_TASKUNIQUEID1 = 0x0B340000;`
const SUBPROJECT_TASKUNIQUEID1: i32 = 0x0B34_0000;

/// Marker value 2 for "the previous value is a subproject task unique ID".
///
/// Corresponds to Java: `private static final int SUBPROJECT_TASKUNIQUEID2 = 0x0ABB0000;`
const SUBPROJECT_TASKUNIQUEID2: i32 = 0x0ABB_0000;

/// Marker value 3 for "the previous value is a subproject task unique ID".
///
/// Corresponds to Java: `private static final int SUBPROJECT_TASKUNIQUEID3 = 0x05A10000;`
const SUBPROJECT_TASKUNIQUEID3: i32 = 0x05A1_0000;

/// Marker value 4 for "the previous value is a subproject task unique ID".
///
/// Corresponds to Java: `private static final int SUBPROJECT_TASKUNIQUEID4 = 0x02F70000;`
const SUBPROJECT_TASKUNIQUEID4: i32 = 0x02F7_0000;

/// Marker value 5 for "the previous value is a subproject task unique ID".
///
/// Corresponds to Java: `private static final int SUBPROJECT_TASKUNIQUEID5 = 0x07010000;`
const SUBPROJECT_TASKUNIQUEID5: i32 = 0x0701_0000;

/// Standard key for table column data.
///
/// Corresponds to Java: `private static final Integer TABLE_COLUMN_DATA_STANDARD = Integer.valueOf(1);`
const TABLE_COLUMN_DATA_STANDARD: i32 = 1;

/// Enterprise key for table column data.
///
/// Corresponds to Java: `private static final Integer TABLE_COLUMN_DATA_ENTERPRISE = Integer.valueOf(2);`
const TABLE_COLUMN_DATA_ENTERPRISE: i32 = 2;

/// Baseline key for table column data (Java uses `null`).
///
/// Corresponds to Java: `private static final Integer TABLE_COLUMN_DATA_BASELINE = null;`
const TABLE_COLUMN_DATA_BASELINE: Option<i32> = None;

/// Data type key for outline code values in the variable-length data block.
///
/// Corresponds to Java: `private static final Integer OUTLINECODE_DATA = Integer.valueOf(1);`
const OUTLINE_CODE_DATA: i32 = 1;

/// Mask for the "confirmed" flag in the duration units field.
///
/// Corresponds to Java: `private static final int DURATION_CONFIRMED_MASK = 0x20;`
const DURATION_CONFIRMED_MASK: i32 = 0x20;

/// Fixed offset of the unique ID of deleted / empty tasks in the fixed-length data block.
///
/// Corresponds to Java: `private static final int TASK_UNIQUE_ID_FIXED_OFFSET = 0;`
const TASK_UNIQUE_ID_FIXED_OFFSET: usize = 0;

/// Fixed offset of the ID of deleted / empty tasks in the fixed-length data block.
///
/// Corresponds to Java: `private static final int TASK_ID_FIXED_OFFSET = 4;`
const TASK_ID_FIXED_OFFSET: usize = 4;

/// Length of the "empty task" data block.
///
/// Corresponds to Java: `private static final int NULL_TASK_BLOCK_SIZE = 8;`
const NULL_TASK_BLOCK_SIZE: usize = 8;

/// Epoch value representing "no date" as 1983-12-31 00:00.
///
/// Corresponds to Java: `MicrosoftProjectConstants.EPOCH_DATE`
/// (this class is not ported; see the file header of `mpp_utility.rs` for how the same name is handled).
fn epoch_date() -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(1983, 12, 31)
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .unwrap_or(chrono::NaiveDateTime::MIN)
}

/// MPP9 file reader.
///
/// Corresponds to Java: `final class MPP9Reader implements MPPVariantReader`
///
/// This type is zero-sized: all Java instance state lives in [`MPP9ReaderState`] (see the file header).
pub struct MPP9Reader;

impl MPP9Reader {
    /// Constructor.
    ///
    /// Corresponds to Java: implicit no-arg constructor (`new MPP9Reader()`)
    pub fn new() -> Self {
        Self
    }
}

impl Default for MPP9Reader {
    /// Corresponds to Java: implicit no-arg constructor, equivalent to [`MPP9Reader::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl MPPVariantReader for MPP9Reader {
    /// Read an MPP9 file and populate `file`.
    ///
    /// Corresponds to Java: `@Override public void process(MPPReader reader, ProjectFile file, DirectoryEntry root)`
    fn process(
        &self,
        reader: &MPPReader,
        file: &Rc<RefCell<ProjectFile>>,
        root: &DirectoryEntry,
    ) -> MpxjResult<()> {
        // Java:
        // try { populateMemberData(reader, file, root); processProjectProperties(); … }
        // finally { clearMemberData(); }
        //
        // When `populateMemberData` fails, the state has not been established yet (in Java the fields are
        // already partially assigned at that point, but `clearMemberData` in `finally` just nulls the references); the Rust side has no state to clear, so it returns the error directly.
        let mut state = MPP9ReaderState::populate_member_data(reader, file, root)?;
        let result = state.process();
        state.clear_member_data();
        result
    }
}

/// Instance state of `MPP9Reader` (Java's instance fields).
///
/// Corresponds to Java: the set of instance fields of `MPP9Reader` (see "Structural trade-off" in the file header).
struct MPP9ReaderState<'a> {
    /// Parent reader (used only to read the 5 switches).
    ///
    /// Corresponds to Java field: `private MPPReader m_reader`
    reader: &'a MPPReader,

    /// Project file to be populated.
    ///
    /// Corresponds to Java field: `private ProjectFile m_file`
    file: Rc<RefCell<ProjectFile>>,

    /// Root storage of the compound document.
    ///
    /// Corresponds to Java field: `private DirectoryEntry m_root`
    root: DirectoryEntry,

    /// Resource unique ID → resource calendar.
    ///
    /// Corresponds to Java field: `private HashMap<Integer, ProjectCalendar> m_resourceMap`
    resource_map: HashMap<i32, Rc<RefCell<ProjectCalendar>>>,

    /// Outline code value data.
    ///
    /// Corresponds to Java field: `private Var2Data m_outlineCodeVarData`
    outline_code_var_data: Var2Data,

    /// Project `Props` data stream.
    ///
    /// Corresponds to Java field: `private Props9 m_projectProps`
    project_props: Props9,

    /// Base font table: font index → base font.
    ///
    /// Corresponds to Java field: `private Map<Integer, FontBase> m_fontBases`
    font_bases: HashMap<i32, FontBase>,

    /// Task unique ID → owning subproject file name.
    ///
    /// Corresponds to Java field: `private Map<Integer, String> m_taskSubProjects`
    task_sub_projects: HashMap<i32, String>,

    /// Set of external task unique IDs.
    ///
    /// Corresponds to Java field: `private Set<Integer> m_externalTasks`
    external_tasks: BTreeSet<i32>,

    /// Project data storage (`   19`).
    ///
    /// Corresponds to Java field: `private DirectoryEntry m_projectDir`
    project_dir: DirectoryEntry,

    /// View data storage (`   29`).
    ///
    /// Corresponds to Java field: `private DirectoryEntry m_viewDir`
    view_dir: DirectoryEntry,

    /// Task ID → task unique ID (used for reordering in `postProcessTasks`).
    ///
    /// Corresponds to Java field: `private Map<Integer, Integer> m_taskOrder`
    task_order: BTreeMap<i32, i32>,

    /// Empty task ID → task unique ID.
    ///
    /// Corresponds to Java field: `private Map<Integer, Integer> m_nullTaskOrder`
    null_task_order: BTreeMap<i32, i32>,

    /// Factory that creates plain / decrypting streams according to the password protection state.
    ///
    /// Corresponds to Java field: `private DocumentInputStreamFactory m_inputStreamFactory`
    input_stream_factory: DocumentInputStreamFactory,
}

impl<'a> MPP9ReaderState<'a> {
    /// Populate the data this reader needs and parse the file-level properties.
    ///
    /// Corresponds to Java: `private void populateMemberData(MPPReader reader, ProjectFile file, DirectoryEntry root)`
    ///
    /// Rust-specific: in Java this method writes the arguments and parse results into instance fields (no return value); this file merges
    /// "build the state" and "fill the fields" into one associated function returning `Self` (see "Structural trade-off" in the file header).
    fn populate_member_data(
        reader: &'a MPPReader,
        file: &Rc<RefCell<ProjectFile>>,
        root: &DirectoryEntry,
    ) -> MpxjResult<Self> {
        // Java: m_reader = reader; m_file = file; m_eventManager = file.getEventManager(); m_root = root;
        let file = Rc::clone(file);
        let root = root.clone();

        //
        // Retrieve the high level document properties (never encoded)
        //
        // Java: Props props = new Props9(new DocumentInputStream(((DocumentEntry) root.getEntry("Props9"))));
        let props = {
            let data = root
                .get_document("Props9")
                .ok_or_else(|| missing_entry("Props9"))?
                .get_data();
            Props9::new(&data)?
        };

        // Java: file.getProjectProperties().setProjectFilePath(props.getUnicodeString(PropsKey.PROJECT_FILE_PATH));
        let project_file_path = props.get_unicode_string(PropsKey::ProjectFilePath);
        try_file_mut(&file)?
            .get_project_properties_mut()
            .set_project_file_path(project_file_path);

        // Java: m_inputStreamFactory = new DocumentInputStreamFactory(props);
        let input_stream_factory = DocumentInputStreamFactory::new(&props);

        //
        // Test for password protection. In the single byte retrieved here:
        //
        // 0x00 = no password
        // 0x01 = protection password has been supplied
        // 0x02 = write reservation password has been supplied
        // 0x03 = both passwords have been supplied
        //
        // Java: byte passwordProtectionFlag = props.getByte(PropsKey.PASSWORD_FLAG);
        //       boolean passwordRequiredToRead = (passwordProtectionFlag & 0x1) != 0;
        let password_protection_flag = props.get_byte(PropsKey::PasswordFlag);
        let password_required_to_read = (password_protection_flag & 0x1) != 0;

        if password_required_to_read && reader.get_respect_password_protection() {
            // File is password protected for reading, let's read the password
            // and see if the correct read password was given to us.
            //
            // Java: String readPassword = MPPUtility.decodePassword(props.getByteArray(
            //          PropsKey.PROTECTION_PASSWORD_HASH), m_inputStreamFactory.getEncryptionCode());
            // (Java decrypts the array inside `Props` in place; the Rust side makes a copy to avoid modifying `Props`)
            let mut password_hash = props
                .get_byte_array(PropsKey::ProtectionPasswordHash)
                .map(<[u8]>::to_vec)
                .unwrap_or_default();
            let read_password =
                MPPUtility::decode_password(&mut password_hash, input_stream_factory.get_encryption_code());

            // It looks like it is possible for a project file to have the password protection flag on
            // without a password. In this case MS Project treats the file as NOT protected.
            // See https://github.com/joniles/mpxj/issues/xxx — see the Java source for the original comment.
            if let Some(read_password) = read_password {
                if !read_password.is_empty() {
                    // See if the correct read password was given
                    // Java: if (reader.getReadPassword() == null || !reader.getReadPassword().matches(readPassword))
                    let matches = match reader.get_read_password() {
                        None => false,
                        // Java uses `String.matches` (regex); the regex dependency has not been introduced, so equality is used (see the file header)
                        Some(given) => given == read_password.as_str(),
                    };

                    if !matches {
                        // Passwords don't match
                        return Err(MpxjError::new(MpxjError::PASSWORD_PROTECTED_ENTER_PASSWORD));
                    }
                }
            }
            // Passwords matched so let's allow the reading to continue.
        }

        // Java: m_resourceMap = new HashMap<>();
        //       m_projectDir = (DirectoryEntry) root.getEntry("   19");
        //       m_viewDir = (DirectoryEntry) root.getEntry("   29");
        let project_dir = root
            .get_directory(PROJECT_DIRECTORY)
            .ok_or_else(|| missing_entry(PROJECT_DIRECTORY))?;
        let view_dir = root
            .get_directory(VIEW_DIRECTORY)
            .ok_or_else(|| missing_entry(VIEW_DIRECTORY))?;

        // Java: DirectoryEntry outlineCodeDir = (DirectoryEntry) m_projectDir.getEntry("TBkndOutlCode");
        let outline_code_dir = project_dir
            .get_directory(OUTLINE_CODE_DIRECTORY)
            .ok_or_else(|| missing_entry(OUTLINE_CODE_DIRECTORY))?;

        // Java: VarMeta outlineCodeVarMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) outlineCodeDir.getEntry("VarMeta"))));
        let outline_code_var_meta: Rc<dyn VarMeta> = {
            let data = outline_code_dir
                .get_document("VarMeta")
                .ok_or_else(|| missing_entry("VarMeta"))?
                .get_data();
            Rc::new(VarMeta9::new(&data)?)
        };

        // Java: m_outlineCodeVarData = new Var2Data(m_file, outlineCodeVarMeta, new DocumentInputStream(((DocumentEntry) outlineCodeDir.getEntry("Var2Data"))));
        let outline_code_var_data = {
            let data = outline_code_dir
                .get_document("Var2Data")
                .ok_or_else(|| missing_entry("Var2Data"))?
                .get_data();
            Var2Data::new(&file, outline_code_var_meta, &data)
        };

        // Java: m_projectProps = new Props9(m_inputStreamFactory.getInstance(m_projectDir, "Props"));
        let project_props = {
            let mut stream = input_stream_factory.get_instance(&project_dir, "Props")?;
            let data = stream.to_bytes();
            Props9::new(&data)?
        };

        // Java: m_fontBases = new HashMap<>(); m_taskSubProjects = new HashMap<>(); m_externalTasks = new HashSet<>();
        //       m_taskOrder = new TreeMap<>(); m_nullTaskOrder = new TreeMap<>();
        //
        // Java: m_file.getProjectProperties().setMppFileType(Integer.valueOf(9));
        //       m_file.getProjectProperties().setAutoFilter(props.getBoolean(PropsKey.AUTO_FILTER));
        let auto_filter = props.get_boolean(PropsKey::AutoFilter);
        {
            let mut file_ref = try_file_mut(&file)?;
            let properties = file_ref.get_project_properties_mut();
            properties.set_mpp_file_type(Some(9));
            properties.set_auto_filter(auto_filter);
        }

        Ok(Self {
            reader,
            file,
            root,
            resource_map: HashMap::new(),
            outline_code_var_data,
            project_props,
            font_bases: HashMap::new(),
            task_sub_projects: HashMap::new(),
            external_tasks: BTreeSet::new(),
            project_dir,
            view_dir,
            task_order: BTreeMap::new(),
            null_task_order: BTreeMap::new(),
            input_stream_factory,
        })
    }

    /// Clear the transient state.
    ///
    /// Corresponds to Java: `private void clearMemberData()`
    ///
    /// Java sets each field to `null` (including non-collection fields such as `m_file` / `m_root` / `m_projectProps`);
    /// the Rust state struct is dropped as a whole when `process` ends, which is equivalent to nulling it, so only the collections are cleared here.
    fn clear_member_data(&mut self) {
        self.resource_map.clear();
        self.font_bases.clear();
        self.task_order.clear();
        self.null_task_order.clear();
        self.task_sub_projects.clear();
        self.external_tasks.clear();
    }

    /// The body of `process`: all the steps after `populateMemberData` and before `clearMemberData`.
    ///
    /// Corresponds to Java: all statements after `populateMemberData(...)` in `MPP9Reader#process`
    fn process(&mut self) -> MpxjResult<()> {
        // Java: processProjectProperties();
        self.process_project_properties()?;

        // Java: if (!reader.getReadPropertiesOnly()) { … }
        if !self.reader.get_read_properties_only() {
            // Java: processSubProjectData();
            self.process_sub_project_data();

            // Java: processGraphicalIndicators();
            self.process_graphical_indicators();

            // Java: processCustomValueLists();
            self.process_custom_value_lists()?;

            // Java: processCalendarData();
            self.process_calendar_data()?;

            // Java: processResourceData();
            self.process_resource_data()?;

            // Java: processTaskData();
            self.process_task_data()?;

            // Java: processConstraintData();
            self.process_constraint_data()?;

            // Java: processAssignmentData();
            self.process_assignment_data()?;

            // Java: postProcessTasks();
            self.post_process_tasks()?;

            // Java: processDataLinks();
            self.process_data_links()?;

            // Java: if (reader.getReadPresentationData()) { … }
            if self.reader.get_read_presentation_data() {
                // Java: processViewPropertyData();
                self.process_view_property_data()?;

                // Java: processTableData();
                self.process_table_data()?;

                // Java: processViewData();
                self.process_view_data()?;

                // Java: processFilterData();
                self.process_filter_data()?;

                // Java: processGroupData();
                self.process_group_data()?;

                // Java: processSavedViewState();
                self.process_saved_view_state()?;
            }
        }

        Ok(())
    }

    /// Process the project property data.
    ///
    /// Corresponds to Java: `private void processProjectProperties() throws MPXJException`
    fn process_project_properties(&mut self) -> MpxjResult<()> {
        // Java: ProjectPropertiesReader reader = new ProjectPropertiesReader();
        //       reader.process(m_file, m_projectProps, m_root);
        ProjectPropertiesReader.process(&self.file, &self.project_props, &self.root)
    }

    /// Process the graphical indicator data.
    ///
    /// Corresponds to Java: `private void processGraphicalIndicators()`
    fn process_graphical_indicators(&mut self) {
        // Java: GraphicalIndicatorReader graphicalIndicatorReader = new GraphicalIndicatorReader();
        //       graphicalIndicatorReader.process(m_file, m_projectProps);
        let mut reader = GraphicalIndicatorReader::new();
        reader.process(&self.file, &self.project_props);
    }

    /// Read the subproject data and index it by task ID.
    ///
    /// Corresponds to Java: `private void processSubProjectData()`
    ///
    /// Project stores "all subprojects ever inserted into this project" in order, and the `unique id` offsets are derived exactly
    /// from this order.
    fn process_sub_project_data(&mut self) {
        // Java: byte[] subProjData = m_projectProps.getByteArray(PropsKey.SUBPROJECT_DATA);
        //       if (subProjData == null) { return; }
        // (make a copy: the subsequent `&mut self` call must not overlap with the borrow of `m_projectProps`)
        let Some(sub_proj_data) = self
            .project_props
            .get_byte_array(PropsKey::SubprojectData)
            .map(<[u8]>::to_vec)
        else {
            return;
        };
        let sub_proj_data = sub_proj_data.as_slice();

        // Java: int index = 0; int offset = 0;
        let mut index = 0i32;
        let mut offset = 0usize;

        // Java: /*int blockSize = MPPUtility.getInt(subProjData, offset);*/ offset += 4;
        offset += 4;

        // Java: /*int unknown = MPPUtility.getInt(subProjData, offset);*/ offset += 4;
        offset += 4;

        // Java: int itemCountOffset = ByteArrayHelper.getInt(subProjData, offset); offset += 4;
        let item_count_offset = get_int_safe(sub_proj_data, offset);
        offset += 4;

        // Java: while (offset < itemCountOffset)
        while (offset as i64) < i64::from(item_count_offset) {
            // Java: index++;
            index += 1;

            // Java: itemHeaderOffset = ByteArrayHelper.getShort(subProjData, offset); offset += 4;
            let item_header_offset = get_short_safe(sub_proj_data, offset);
            offset += 4;

            // 20-byte header: 16-byte GUID + 4-byte flag
            // Java: byte subProjectType = subProjData[itemHeaderOffset + 16];
            let sub_project_type = usize::try_from(item_header_offset + 16)
                .ok()
                .and_then(|position| sub_proj_data.get(position).copied())
                .unwrap_or(0);

            // Java: switch (subProjectType) { … }
            match sub_project_type {
                //
                // Subproject that is no longer inserted. This is a placeholder in order to be
                // able to always guarantee unique unique ids.
                //
                0x00 => {
                    // Java: offset += 8;
                    offset += 8;
                }

                //
                // task unique ID, 8 bytes, path, file name
                //
                0x99 | 0x09 | 0x0D => {
                    // Java: uniqueIDOffset = ByteArrayHelper.getShort(subProjData, offset); offset += 4;
                    let unique_id_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    // Java: // sometimes offset of a task ID?  offset += 4;
                    offset += 4;

                    // Java: filePathOffset = ByteArrayHelper.getShort(subProjData, offset); offset += 4;
                    let file_path_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    // Java: fileNameOffset = ByteArrayHelper.getShort(subProjData, offset); offset += 4;
                    let file_name_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    self.read_sub_project(
                        sub_proj_data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                //
                // task unique ID, 8 bytes, path, file name
                //
                0x91 => {
                    let unique_id_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    let file_path_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    let file_name_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    // Unknown offset
                    offset += 4;

                    self.read_sub_project(
                        sub_proj_data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                //
                // task unique ID, path, file name
                //
                0x01 | 0x03 | 0x05 | 0x08 | 0x0A | 0x11 => {
                    let unique_id_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    let file_path_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    let file_name_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    self.read_sub_project(
                        sub_proj_data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                //
                // task unique ID, path, unknown, file name
                //
                0x81 | 0x41 => {
                    let unique_id_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    let file_path_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    // unknown offset to 2 bytes of data?
                    offset += 4;

                    let file_name_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    self.read_sub_project(
                        sub_proj_data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                //
                // task unique ID, path, file name
                //
                0xC0 => {
                    let unique_id_offset = item_header_offset;

                    let file_path_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    let file_name_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    // unknown offset
                    offset += 4;

                    self.read_sub_project(
                        sub_proj_data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                0x45 => {
                    // Java: uniqueIDOffset = ByteArrayHelper.getInt(subProjData, offset) & 0x1FFFF;
                    let unique_id_offset = get_int_safe(sub_proj_data, offset) & 0x1FFFF;
                    offset += 4;

                    let file_path_offset = get_int_safe(sub_proj_data, offset) & 0x1FFFF;
                    offset += 4;

                    let file_name_offset = get_int_safe(sub_proj_data, offset) & 0x1FFFF;
                    offset += 4;

                    offset += 4;

                    self.read_sub_project(
                        sub_proj_data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                //
                // path, file name
                //
                0x02 | 0x04 => {
                    let file_path_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    let file_name_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    // Java: readSubProject(subProjData, itemHeaderOffset, -1, filePathOffset, fileNameOffset, index);
                    self.read_sub_project(
                        sub_proj_data,
                        item_header_offset,
                        -1,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                //
                // task unique ID, 4 bytes, path, 4 bytes, file name
                //
                0x8D => {
                    let unique_id_offset = get_short_safe(sub_proj_data, offset);
                    offset += 8;

                    let file_path_offset = get_short_safe(sub_proj_data, offset);
                    offset += 8;

                    let file_name_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    self.read_sub_project(
                        sub_proj_data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                //
                // Appears when a subproject is collapsed
                //
                0x80 => {
                    offset += 12;
                }

                // deleted entry?
                0x10 => {
                    offset += 8;
                }

                // new resource pool entry
                0x44 => {
                    let file_path_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    offset += 4;

                    let file_name_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    // Java: readSubProject(subProjData, itemHeaderOffset, -1, filePathOffset, fileNameOffset, index);
                    self.read_sub_project(
                        sub_proj_data,
                        item_header_offset,
                        -1,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                //
                // Any other value, assume 12 bytes to handle old/deleted data?
                //
                _ => {
                    offset += 12;
                }
            }
        }
    }

    /// Read the subproject details from the byte array.
    ///
    /// Corresponds to Java: `private void readSubProject(byte[] data, int headerOffset, int uniqueIDOffset, int filePathOffset, int fileNameOffset, int subprojectIndex)`
    ///
    /// Java swallows "unrecognized data format" failures with `try/catch (ArrayIndexOutOfBoundsException)`,
    /// recording only an ignored error; the Rust side expresses the same situation through [`MPP9ReaderState::read_sub_project_inner`] returning `Err`
    /// to express the same situation.
    #[allow(clippy::too_many_arguments)]
    fn read_sub_project(
        &mut self,
        data: &[u8],
        header_offset: i32,
        unique_id_offset: i32,
        file_path_offset: i32,
        file_name_offset: i32,
        subproject_index: i32,
    ) {
        //
        // Admit defeat at this point - we have probably stumbled
        // upon a data format we don't understand, so we'll fail
        // gracefully here. This will now be reported as a missing
        // sub project error by end users of the library, rather
        // than as an exception being thrown.
        //
        if let Err(error) = self.read_sub_project_inner(
            data,
            header_offset,
            unique_id_offset,
            file_path_offset,
            file_name_offset,
            subproject_index,
        ) {
            // Java: catch (ArrayIndexOutOfBoundsException ex) { m_file.addIgnoredError(ex); }
            if let Ok(mut file) = self.file.try_borrow_mut() {
                file.add_ignored_error(error);
            }
        }
    }

    /// The body of `readSubProject` (the part wrapped in `try` in Java).
    ///
    /// Returns `Err` when data is out of bounds (corresponding to Java throwing `ArrayIndexOutOfBoundsException`).
    #[allow(clippy::too_many_arguments)]
    fn read_sub_project_inner(
        &mut self,
        data: &[u8],
        header_offset: i32,
        unique_id_offset: i32,
        file_path_offset: i32,
        file_name_offset: i32,
        subproject_index: i32,
    ) -> MpxjResult<()> {
        // Java's `headerOffset`, `fileNameOffset` and `subprojectIndex` are not used in the method body
        // (`subprojectIndex` appears only in a commented-out line); the parameters are kept here to preserve the 1:1 mapping.
        let _ = (header_offset, file_name_offset, subproject_index);

        // We have a 20 byte header.
        // First 16 bytes are (most of the time) the GUID of the target project
        // Remaining 4 bytes are believed to be flags
        // NOTE: actually for MPP9 files this is not the project GUID.
        // MPP9 files don't appear to store the GUID of the subproject.

        // Generate the unique id offset for this subproject
        // int offset = 0x00800000 + ((subprojectIndex - 1) * 0x00400000);

        //
        // First block header
        //
        // Java: filePathOffset += 18;
        let mut offset = checked_offset_add(file_path_offset, 18)?;

        //
        // String size as a 4 byte int
        //
        // Java: filePathOffset += 4;
        offset = checked_offset_add(offset, 4)?;

        //
        // Full DOS path
        //
        // Java: String dosFullPath = MPPUtility.getString(data, filePathOffset);
        let dos_full_path = MPPUtility::get_string(data, checked_offset(offset)?);

        // Java: filePathOffset += (dosFullPath.length() + 1);
        offset = checked_offset_add(offset, (dos_full_path.chars().count() + 1) as i32)?;

        //
        // 24 byte block
        //
        // Java: filePathOffset += 24;
        offset = checked_offset_add(offset, 24)?;

        //
        // 4 byte block size
        //
        // Java: int size = ByteArrayHelper.getInt(data, filePathOffset); filePathOffset += 4;
        let mut size = get_int_checked(data, offset)?;
        offset = checked_offset_add(offset, 4)?;

        let sp;
        if size == 0 {
            sp = dos_full_path;
        } else {
            //
            // 4 byte unicode string size in bytes
            //
            // Java: size = ByteArrayHelper.getInt(data, filePathOffset); filePathOffset += 4;
            size = get_int_checked(data, offset)?;
            offset = checked_offset_add(offset, 4)?;

            //
            // 2 byte data
            //
            // Java: filePathOffset += 2;
            offset = checked_offset_add(offset, 2)?;

            //
            // Unicode string
            //
            // Java: sp = MPPUtility.getUnicodeString(data, filePathOffset, size);
            let max_length = usize::try_from(size).map_err(|_| out_of_bounds())?;
            sp = MPPUtility::get_unicode_string_with_max_length(data, checked_offset(offset)?, max_length);
            // filePathOffset += size;
        }

        // Java: processUniqueIdValues(sp, data, uniqueIDOffset);
        self.process_unique_id_values(&sp, data, unique_id_offset)
    }

    /// Record the mapping between task unique IDs inside a subproject and the owning subproject file.
    ///
    /// Corresponds to Java: `private void processUniqueIdValues(String sp, byte[] data, int uniqueIDOffset)`
    fn process_unique_id_values(
        &mut self,
        sp: &str,
        data: &[u8],
        unique_id_offset: i32,
    ) -> MpxjResult<()> {
        // Java: if (uniqueIDOffset == -1) { return; }
        if unique_id_offset == -1 {
            return Ok(());
        }

        // Java: Integer prev = Integer.valueOf(0);
        let mut prev: i32 = 0;
        let mut offset = unique_id_offset;

        // Java: int value = ByteArrayHelper.getInt(data, uniqueIDOffset);
        let mut value = get_int_checked(data, offset)?;

        // Java: while (value != SUBPROJECT_LISTEND) { … }
        while value != SUBPROJECT_LISTEND {
            match value {
                SUBPROJECT_TASKUNIQUEID0
                | SUBPROJECT_TASKUNIQUEID1
                | SUBPROJECT_TASKUNIQUEID2
                | SUBPROJECT_TASKUNIQUEID3
                | SUBPROJECT_TASKUNIQUEID4
                | SUBPROJECT_TASKUNIQUEID5 => {
                    // Java: m_taskSubProjects.put(prev, sp); prev = Integer.valueOf(0);
                    self.task_sub_projects.insert(prev, sp.to_owned());
                    prev = 0;
                }

                _ => {
                    if prev != 0 {
                        // The previous value was for an external task unique task id
                        // Java: m_externalTasks.add(prev); m_taskSubProjects.put(prev, sp);
                        self.external_tasks.insert(prev);
                        self.task_sub_projects.insert(prev, sp.to_owned());
                    }
                    prev = value;
                }
            }

            // Read the next value
            // Java: uniqueIDOffset += 4; value = ByteArrayHelper.getInt(data, uniqueIDOffset);
            offset = checked_offset_add(offset, 4)?;
            value = get_int_checked(data, offset)?;
        }

        if prev != 0 {
            // The previous value was for an external task unique task id
            self.external_tasks.insert(prev);
            self.task_sub_projects.insert(prev, sp.to_owned());
        }

        Ok(())
    }

    /// Process the parts of the `Props` stream related to the visual appearance of the project data.
    ///
    /// Corresponds to Java: `private void processViewPropertyData() throws IOException`
    fn process_view_property_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("Props")) { … }
        if self.view_dir.get_entry("Props").is_none() {
            return Ok(());
        }

        // Java: Props props = new Props9(m_inputStreamFactory.getInstance(m_viewDir, "Props"));
        let props = {
            let mut stream = self.input_stream_factory.get_instance(&self.view_dir, "Props")?;
            let data = stream.to_bytes();
            Props9::new(&data)?
        };

        // Java: byte[] data = props.getByteArray(PropsKey.FONT_BASES);
        //       if (data != null) { processBaseFonts(data); }
        if let Some(data) = props.get_byte_array(PropsKey::FontBases) {
            let data = data.to_vec();
            self.process_base_fonts(&data);
        }

        // Java: ProjectProperties properties = m_file.getProjectProperties();
        //       properties.setShowProjectSummaryTask(props.getBoolean(PropsKey.SHOW_PROJECT_SUMMARY_TASK));
        let show_project_summary_task = props.get_boolean(PropsKey::ShowProjectSummaryTask);
        try_file_mut(&self.file)?
            .get_project_properties_mut()
            .set_show_project_summary_task(show_project_summary_task);

        Ok(())
    }

    /// Build the index "base font index → base font instance".
    ///
    /// Corresponds to Java: `private void processBaseFonts(byte[] data)`
    fn process_base_fonts(&mut self, data: &[u8]) {
        // Java: int offset = 0;
        let mut offset = 0usize;

        // Java: int blockCount = ByteArrayHelper.getShort(data, 0); offset += 2;
        let block_count = get_short_safe(data, 0);
        offset += 2;

        // Java: for (int loop = 0; loop < blockCount; loop++) { … }
        for loop_index in 0..block_count {
            // Java: /*unknownAttribute = MPPUtility.getShort(data, offset);*/ offset += 2;
            offset += 2;

            // Java: size = ByteArrayHelper.getShort(data, offset); offset += 2;
            let size = get_short_safe(data, offset);
            offset += 2;

            // Java: name = MPPUtility.getUnicodeString(data, offset); offset += 64;
            let name = MPPUtility::get_unicode_string(data, offset);
            offset += 64;

            if !name.is_empty() {
                // Java: FontBase fontBase = new FontBase(Integer.valueOf(loop), name, size);
                //       m_fontBases.put(fontBase.getIndex(), fontBase);
                let font_base = FontBase::new(Some(loop_index), name, size);
                let index = font_base.get_index().unwrap_or(loop_index);
                self.font_bases.insert(index, font_base);
            }
        }
    }

    /// Read the list of task field values defined in the MPP file.
    ///
    /// Corresponds to Java: `private void processCustomValueLists() throws IOException`
    fn process_custom_value_lists(&mut self) -> MpxjResult<()> {
        // Java: CustomFieldValueReader9 reader = new CustomFieldValueReader9(m_projectDir, m_file, m_projectProps);
        //       reader.process();
        CustomFieldValueReader9::new(&self.project_dir, &self.file, &self.project_props).process()
    }

    /// Read the resource / task field aliases defined in the MPP file.
    ///
    /// Corresponds to Java: `private void processFieldNameAliases(Map<Integer, FieldType> map, byte[] data)`
    ///
    /// Java's `Map<Integer, FieldType>` becomes `HashMap<i32, FieldKey>` on the Rust side
    /// (the runtime hashable equivalent of `FieldType`, see the file header of `field_key.rs`).
    fn process_field_name_aliases(&mut self, map: &HashMap<i32, FieldKey>, data: Option<&[u8]>) {
        let Some(data) = data else {
            return;
        };

        // Java: int offset = 0; int index = 0;
        let mut offset = 0usize;
        let mut index = 0i32;

        // Java: CustomFieldContainer fields = m_file.getCustomFields();
        let Ok(file) = try_file(&self.file) else {
            return;
        };
        let fields = file.get_custom_fields();
        drop(file);

        // Java: while (offset < data.length) { … }
        while offset < data.len() {
            // Java: String alias = MPPUtility.getUnicodeString(data, offset);
            let alias = MPPUtility::get_unicode_string(data, offset);

            if !alias.is_empty() {
                // Java: FieldType field = map.get(Integer.valueOf(index));
                //       if (field != null) { fields.getOrCreate(field).setAlias(alias); }
                if let Some(field) = map.get(&index) {
                    let custom = CustomFieldContainer::get_or_create(&fields, *field);
                    if let Ok(mut custom) = custom.try_borrow_mut() {
                        custom.set_alias(Some(alias.clone()));
                    }
                }
            }

            // Java: offset += (alias.length() + 1) * 2;
            offset = offset.saturating_add((alias.chars().count() + 1) * 2);
            index += 1;
        }
    }

    /// Map task unique identifiers to indexes in the fixed-length data block.
    ///
    /// Corresponds to Java: `private TreeMap<Integer, Integer> createTaskMap(FieldMap fieldMap, FixedMeta taskFixedMeta, FixedData taskFixedData, Var2Data taskVarData)`
    ///
    /// Java's `TreeMap` (whose values may be `null`) becomes `BTreeMap<i32, Option<i32>>` on the Rust side:
    /// A value of `None` means "this unique ID belongs to a deleted task".
    fn create_task_map(
        &self,
        field_map: &FieldMap,
        task_fixed_meta: &FixedMeta,
        task_fixed_data: &FixedData,
        task_var_data: &Var2Data,
    ) -> BTreeMap<i32, Option<i32>> {
        let mut task_map: BTreeMap<i32, Option<i32>> = BTreeMap::new();

        // Java: int uniqueIdOffset = fieldMap.getFixedDataOffset(TaskField.UNIQUE_ID);
        let unique_id_offset = field_map.get_fixed_data_offset(&task_field(TaskField::UniqueId));

        // Java: int itemCount = taskFixedMeta.getAdjustedItemCount();
        let item_count = task_fixed_meta.get_adjusted_item_count();

        //
        // First three items are not tasks, so let's skip them.
        // Note we're working backwards: where we have duplicate tasks the later ones
        // appear to be the correct versions (https://github.com/joniles/mpxj/issues/152)
        //
        // Java: for (int loop = itemCount - 1; loop > 2; loop--) { … }
        for loop_index in (3..item_count).rev() {
            // Java: byte[] data = taskFixedData.getByteArrayValue(loop);
            let Some(data) = task_fixed_data.get_byte_array_value(loop_index) else {
                continue;
            };

            // Java: byte[] metaData = taskFixedMeta.getByteArrayValue(loop);
            let meta_data: &[u8] = task_fixed_meta.get_byte_array_value(loop_index).unwrap_or(&[]);

            //
            // Check for the deleted task flag
            //
            // Java: int flags = ByteArrayHelper.getInt(metaData, 0);
            let flags = get_int_safe(meta_data, 0);

            if (flags & 0x02) != 0 {
                //
                // Project stores the deleted tasks unique id's into the fixed data as well
                // and at least in one case the deleted task was listed twice in the list
                // the second time with data with it causing a phantom task to be shown.
                // See CalendarErrorPhantomTasks.mpp
                //
                // Java: uniqueID = ByteArrayHelper.getShort(data, TASK_UNIQUE_ID_FIXED_OFFSET);
                //       taskMap.put(key, null);
                let unique_id = get_short_by_offset(data, TASK_UNIQUE_ID_FIXED_OFFSET as i32);
                task_map.insert(unique_id, None);
            } else {
                //
                // Do we have a null task?
                //
                if data.len() == NULL_TASK_BLOCK_SIZE {
                    let unique_id = get_int_by_offset(data, TASK_UNIQUE_ID_FIXED_OFFSET as i32);
                    if !task_map.contains_key(&unique_id) {
                        task_map.insert(unique_id, Some(loop_index));
                    }
                } else {
                    //
                    // We apply a heuristic here - if we have more than 75% of the data, we assume
                    // the task is valid.
                    //
                    let max_size = field_map.get_max_fixed_data_size(0).unwrap_or(0);
                    if max_size == 0 || ((data.len() as i32 * 100) / max_size) > 75 {
                        let unique_id = get_int_by_offset(data, unique_id_offset);

                        // Accept this task if it does not have a deleted unique ID or it has a
                        // deleted unique ID but it has var data entries
                        if !task_map.contains_key(&unique_id)
                            || !task_var_data.get_var_meta().get_types(Some(unique_id)).is_empty()
                        {
                            // If a task is already in the map, overwrite it as long as flags is not 0x04.
                            // TODO: gain a better understanding of why this works.
                            if !task_map.contains_key(&unique_id) || (flags & 0x04) == 0 {
                                task_map.insert(unique_id, Some(loop_index));
                            }
                        }
                    }
                }
            }
        }

        task_map
    }

    /// Map resource unique identifiers to indexes in the fixed-length data block.
    ///
    /// Corresponds to Java: `private TreeMap<Integer, Integer> createResourceMap(FieldMap fieldMap, FixedMeta rscFixedMeta, FixedData rscFixedData)`
    fn create_resource_map(
        &self,
        field_map: &FieldMap,
        rsc_fixed_meta: &FixedMeta,
        rsc_fixed_data: &FixedData,
    ) -> BTreeMap<i32, i32> {
        let mut resource_map: BTreeMap<i32, i32> = BTreeMap::new();

        // Java: int itemCount = rscFixedMeta.getAdjustedItemCount();
        let item_count = rsc_fixed_meta.get_adjusted_item_count();

        for loop_index in 0..item_count {
            // Java: byte[] data = rscFixedData.getByteArrayValue(loop);
            //       if (data == null || data.length < fieldMap.getMaxFixedDataSize(0)) { continue; }
            let Some(data) = rsc_fixed_data.get_byte_array_value(loop_index) else {
                continue;
            };
            if (data.len() as i32) < field_map.get_max_fixed_data_size(0).unwrap_or(0) {
                continue;
            }

            // Check for the deleted resource flag
            // Java: byte[] metaData = rscFixedMeta.getByteArrayValue(loop);
            //       int flags = ByteArrayHelper.getInt(metaData, 0);
            let meta_data: &[u8] = rsc_fixed_meta.get_byte_array_value(loop_index).unwrap_or(&[]);
            let flags = get_int_safe(meta_data, 0);
            if (flags & 0x02) != 0 {
                continue;
            }

            // Java: Integer uniqueID = Integer.valueOf(ByteArrayHelper.getShort(data, 0));
            //       resourceMap.put(uniqueID, Integer.valueOf(loop));
            let unique_id = get_short_safe(data, 0);
            resource_map.insert(unique_id, loop_index);
        }

        resource_map
    }

    /// Process the calendar data.
    ///
    /// Corresponds to Java: `private void processCalendarData() throws IOException`
    ///
    /// The calendar data format is: a 4-byte block header + 7×60 bytes (the seven days of a week), optionally followed by a group
    /// of 64-byte exception blocks.
    fn process_calendar_data(&mut self) -> MpxjResult<()> {
        // Java: CalendarFactory factory = new MPP9CalendarFactory(m_file);
        //       factory.processCalendarData(m_projectDir, m_projectProps, m_inputStreamFactory, m_resourceMap);
        let factory = MPP9CalendarFactory::new(Rc::clone(&self.file));
        factory.process_calendar_data(
            &self.project_dir,
            &self.project_props,
            &self.input_stream_factory,
            &mut self.resource_map,
        )
    }

    /// Extract and organize the task data.
    ///
    /// Corresponds to Java: `private void processTaskData() throws IOException`
    ///
    /// What follows walks through the `Task` setters in alphabetical order, pulling data out of the MPP file. Places where no mapping is found
    /// (for example the field is computed on the fly, or the data simply cannot be found in the file) are marked with comments in Java.
    fn process_task_data(&mut self) -> MpxjResult<()> {
        // Java: FieldMap fieldMap = new FieldMap9(m_file); fieldMap.createTaskFieldMap(m_projectProps);
        let mut field_map = FieldMap9::new(&self.file);
        field_map.create_task_field_map(&self.project_props);

        // Java: DirectoryEntry taskDir = (DirectoryEntry) m_projectDir.getEntry("TBkndTask");
        let task_dir = self
            .project_dir
            .get_directory(TASK_DIRECTORY)
            .ok_or_else(|| missing_entry(TASK_DIRECTORY))?;

        // Java: VarMeta taskVarMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) taskDir.getEntry("VarMeta"))));
        let task_var_meta: Rc<dyn VarMeta> = {
            let data = task_dir
                .get_document("VarMeta")
                .ok_or_else(|| missing_entry("VarMeta"))?
                .get_data();
            Rc::new(VarMeta9::new(&data)?)
        };

        // Java: Var2Data taskVarData = new Var2Data(m_file, taskVarMeta, new DocumentInputStream(((DocumentEntry) taskDir.getEntry("Var2Data"))));
        let task_var_data = {
            let data = task_dir
                .get_document("Var2Data")
                .ok_or_else(|| missing_entry("Var2Data"))?
                .get_data();
            Var2Data::new(&self.file, Rc::clone(&task_var_meta), &data)
        };

        // Java: FixedMeta taskFixedMeta = new FixedMeta(new DocumentInputStream(((DocumentEntry) taskDir.getEntry("FixedMeta"))), 47);
        let task_fixed_meta = {
            let data = task_dir
                .get_document("FixedMeta")
                .ok_or_else(|| missing_entry("FixedMeta"))?
                .get_data();
            FixedMeta::with_item_size(&data, 47).map_err(MpxjError::from)?
        };

        // Java: FixedData taskFixedData = new FixedData(taskFixedMeta,
        //          m_inputStreamFactory.getInstance(taskDir, "FixedData"), 768, fieldMap.getMaxFixedDataSize(0));
        let task_fixed_data = {
            let mut stream = self.input_stream_factory.get_instance(&task_dir, "FixedData")?;
            let bytes = stream.to_bytes();
            FixedData::with_limits(
                &task_fixed_meta,
                &bytes,
                768,
                field_map.get_max_fixed_data_size(0).unwrap_or(0),
            )
        };

        // Java: processFieldNameAliases(TASK_FIELD_ALIASES, m_projectProps.getByteArray(PropsKey.TASK_FIELD_NAME_ALIASES));
        let aliases = self
            .project_props
            .get_byte_array(PropsKey::TaskFieldNameAliases)
            .map(<[u8]>::to_vec);
        let alias_map = task_field_aliases();
        self.process_field_name_aliases(&alias_map, aliases.as_deref());

        // Java: TreeMap<Integer, Integer> taskMap = createTaskMap(fieldMap, taskFixedMeta, taskFixedData, taskVarData);
        let task_map = self.create_task_map(&field_map, &task_fixed_meta, &task_fixed_data, &task_var_data);

        // The var data may not contain all the tasks as tasks with no var data assigned will
        // not be saved in there. Most notably these are tasks with no name. So use the task map
        // which contains all the tasks.
        // Java: Integer[] uniqueIdArray = taskMap.keySet().toArray(new Integer[0]);
        let unique_id_array: Vec<i32> = task_map.keys().copied().collect();

        // Java: boolean autoWBS = true;
        let mut auto_wbs = true;

        // Java: List<Task> externalTasks = new ArrayList<>();
        let mut external_tasks: Vec<Rc<RefCell<Task>>> = Vec::new();

        // Java: RecurringTaskReader recurringTaskReader = null;
        let mut recurring_task_reader: Option<RecurringTaskReader> = None;

        // Java: HyperlinkReader hyperlinkReader = new HyperlinkReader();
        let hyperlink_reader = HyperlinkReader::new();

        // Java: for (Integer uniqueID : uniqueIdArray) { … }
        for unique_id in unique_id_array {
            // Java: offset = taskMap.get(uniqueID); if (!taskFixedData.isValidOffset(offset)) { continue; }
            let Some(offset) = task_map.get(&unique_id).copied().flatten() else {
                continue;
            };

            // Java: data = taskFixedData.getByteArrayValue(offset.intValue());
            let Some(data) = task_fixed_data.get_byte_array_value(offset) else {
                continue;
            };

            // Java: Integer id = Integer.valueOf(ByteArrayHelper.getInt(data, fieldMap.getFixedDataOffset(TaskField.ID)));
            let id = get_int_by_offset(data, field_map.get_fixed_data_offset(&task_field(TaskField::Id)));

            // Java: if (data.length == NULL_TASK_BLOCK_SIZE) { … continue; }
            if data.len() == NULL_TASK_BLOCK_SIZE {
                let task = try_file_mut(&self.file)?.add_task();
                {
                    let mut task_ref = task.borrow_mut();
                    task_ref.set_null(true);
                    task_ref.set_unique_id(Some(get_short_by_offset(
                        data,
                        TASK_UNIQUE_ID_FIXED_OFFSET as i32,
                    )));
                    task_ref.set_id(Some(get_short_by_offset(data, TASK_ID_FIXED_OFFSET as i32)));
                    self.null_task_order.insert(
                        task_ref.get_id().unwrap_or(0),
                        task_ref.get_unique_id().unwrap_or(0),
                    );
                }
                continue;
            }

            // Java: if (data.length < fieldMap.getMaxFixedDataSize(0)) { continue; }
            if (data.len() as i32) < field_map.get_max_fixed_data_size(0).unwrap_or(0) {
                continue;
            }

            // Java: if (uniqueID.intValue() != 0 && !taskVarMeta.containsKey(uniqueID)) { continue; }
            if unique_id != 0 && !task_var_meta.contains_key(Some(unique_id)) {
                continue;
            }

            // Java: metaData = taskFixedMeta.getByteArrayValue(offset.intValue());
            let meta_data: &[u8] = task_fixed_meta.get_byte_array_value(offset).unwrap_or(&[]);

            // Java: byte[] recurringData = taskVarData.getByteArray(uniqueID, fieldMap.getVarDataKey(TaskField.RECURRING_DATA));
            let recurring_data = task_var_data.get_byte_array(
                Some(unique_id),
                field_map.get_var_data_key(&task_field(TaskField::RecurringData)),
            );

            // Java: Task temp = m_file.getTaskByID(id);
            let temp = try_file(&self.file)?.get_task_by_id(Some(id));
            if let Some(temp) = &temp {
                // Task with this id already exists... determine if this is the 'real' task by seeing
                // if this task has some var data. This is sort of hokey, but it's the best method I have
                // been able to see.
                if !task_var_meta.get_unique_identifier_set().contains(&unique_id) {
                    // Sometimes Project contains phantom tasks that coexist on the same id as a valid
                    // task. In this case don't want to include the phantom task. Seems to be a very rare case.
                    continue;
                }

                if temp.borrow().get_name().is_none() {
                    // Ok, this looks valid. Remove the previous instance since it is most likely not a valid task.
                    // At worst case this removes a task with an empty name.
                    try_file_mut(&self.file)?.remove_task(temp);
                }
            }

            // Java: task = m_file.addTask();
            let task = try_file_mut(&self.file)?.add_task();

            // Java: task.disableEvents();
            //       fieldMap.populateContainer(FieldTypeClass.TASK, task, uniqueID, new byte[][]{data}, taskVarData);
            //       task.enableEvents();
            task.borrow_mut().disable_events();
            field_map.populate_container(
                FieldTypeClass::Task,
                &mut *task.borrow_mut(),
                Some(unique_id),
                &[Some(data)],
                &task_var_data,
            );
            task.borrow_mut().enable_events();

            // Java: task.setEffortDriven((metaData[11] & 0x10) != 0);
            task.borrow_mut()
                .set_effort_driven(meta_byte(meta_data, 11) & 0x10 != 0);

            // Java: task.setEstimated(getDurationEstimated(ByteArrayHelper.getShort(data,
            //          fieldMap.getFixedDataOffset(TaskField.ACTUAL_DURATION_UNITS))));
            let estimated = Self::get_duration_estimated(get_short_by_offset(
                data,
                field_map.get_fixed_data_offset(&task_field(TaskField::ActualDurationUnits)),
            ));
            task.borrow_mut().set_estimated(estimated);

            // Java: task.setExpanded(((metaData[12] & 0x02) == 0));
            task.borrow_mut().set_expanded(meta_byte(meta_data, 12) & 0x02 == 0);

            // Java: if (NumberHelper.getInt(task.getSubprojectTaskID()) != 0) { task.setExternalTask(true); externalTasks.add(task); }
            if task.borrow().get_subproject_task_id().unwrap_or(0) != 0 {
                task.borrow_mut().set_external_task(true);
                external_tasks.push(Rc::clone(&task));
            }

            // Java: task.setFlag(1, (metaData[37] & 0x20) != 0); … task.setFlag(20, (metaData[40] & 0x01) != 0);
            {
                let mut task_ref = task.borrow_mut();
                task_ref.set_flag(1, meta_byte(meta_data, 37) & 0x20 != 0);
                task_ref.set_flag(2, meta_byte(meta_data, 37) & 0x40 != 0);
                task_ref.set_flag(3, meta_byte(meta_data, 37) & 0x80 != 0);
                task_ref.set_flag(4, meta_byte(meta_data, 38) & 0x01 != 0);
                task_ref.set_flag(5, meta_byte(meta_data, 38) & 0x02 != 0);
                task_ref.set_flag(6, meta_byte(meta_data, 38) & 0x04 != 0);
                task_ref.set_flag(7, meta_byte(meta_data, 38) & 0x08 != 0);
                task_ref.set_flag(8, meta_byte(meta_data, 38) & 0x10 != 0);
                task_ref.set_flag(9, meta_byte(meta_data, 38) & 0x20 != 0);
                task_ref.set_flag(10, meta_byte(meta_data, 38) & 0x40 != 0);
                task_ref.set_flag(11, meta_byte(meta_data, 38) & 0x80 != 0);
                task_ref.set_flag(12, meta_byte(meta_data, 39) & 0x01 != 0);
                task_ref.set_flag(13, meta_byte(meta_data, 39) & 0x02 != 0);
                task_ref.set_flag(14, meta_byte(meta_data, 39) & 0x04 != 0);
                task_ref.set_flag(15, meta_byte(meta_data, 39) & 0x08 != 0);
                task_ref.set_flag(16, meta_byte(meta_data, 39) & 0x10 != 0);
                task_ref.set_flag(17, meta_byte(meta_data, 39) & 0x20 != 0);
                task_ref.set_flag(18, meta_byte(meta_data, 39) & 0x40 != 0);
                task_ref.set_flag(19, meta_byte(meta_data, 39) & 0x80 != 0);
                task_ref.set_flag(20, meta_byte(meta_data, 40) & 0x01 != 0);

                // Java: task.setHideBar((metaData[10] & 0x80) != 0);
                task_ref.set_hide_bar(meta_byte(meta_data, 10) & 0x80 != 0);
            }

            // Java: hyperlinkReader.read(task, taskVarData.getByteArray(uniqueID, fieldMap.getVarDataKey(TaskField.HYPERLINK_DATA)));
            if let Some(hyperlink_data) = task_var_data.get_byte_array(
                Some(unique_id),
                field_map.get_var_data_key(&task_field(TaskField::HyperlinkData)),
            ) {
                hyperlink_reader.read(
                    FieldTypeClass::Task,
                    &mut *task.borrow_mut(),
                    &hyperlink_data,
                );
            }

            // Java: task.setID(id);
            task.borrow_mut().set_id(Some(id));

            // Java: task.setIgnoreResourceCalendar(((metaData[10] & 0x02) != 0));
            task.borrow_mut()
                .set_ignore_resource_calendar(meta_byte(meta_data, 10) & 0x02 != 0);

            // Java: task.setLevelAssignments((metaData[13] & 0x04) != 0);
            task.borrow_mut()
                .set_level_assignments(meta_byte(meta_data, 13) & 0x04 != 0);

            // Java: task.setLevelingCanSplit((metaData[13] & 0x02) != 0);
            task.borrow_mut()
                .set_leveling_can_split(meta_byte(meta_data, 13) & 0x02 != 0);

            // Java: task.setMarked((metaData[9] & 0x40) != 0);
            task.borrow_mut().set_marked(meta_byte(meta_data, 9) & 0x40 != 0);

            // Java: task.setMilestone((metaData[8] & 0x20) != 0);
            task.borrow_mut().set_milestone(meta_byte(meta_data, 8) & 0x20 != 0);

            // Java: task.setOutlineCode(1, m_outlineCodeVarData.getUnicodeString((Integer) task.getCachedValue(TaskField.OUTLINE_CODE1_INDEX), OUTLINECODE_DATA)); … 10
            let value = self.task_outline_code(&task, TaskField::OutlineCode1Index);
            task.borrow_mut().set_outline_code(1, value);
            let value = self.task_outline_code(&task, TaskField::OutlineCode2Index);
            task.borrow_mut().set_outline_code(2, value);
            let value = self.task_outline_code(&task, TaskField::OutlineCode3Index);
            task.borrow_mut().set_outline_code(3, value);
            let value = self.task_outline_code(&task, TaskField::OutlineCode4Index);
            task.borrow_mut().set_outline_code(4, value);
            let value = self.task_outline_code(&task, TaskField::OutlineCode5Index);
            task.borrow_mut().set_outline_code(5, value);
            let value = self.task_outline_code(&task, TaskField::OutlineCode6Index);
            task.borrow_mut().set_outline_code(6, value);
            let value = self.task_outline_code(&task, TaskField::OutlineCode7Index);
            task.borrow_mut().set_outline_code(7, value);
            let value = self.task_outline_code(&task, TaskField::OutlineCode8Index);
            task.borrow_mut().set_outline_code(8, value);
            let value = self.task_outline_code(&task, TaskField::OutlineCode9Index);
            task.borrow_mut().set_outline_code(9, value);
            let value = self.task_outline_code(&task, TaskField::OutlineCode10Index);
            task.borrow_mut().set_outline_code(10, value);

            // Java: task.setRollup((metaData[10] & 0x08) != 0);
            task.borrow_mut().set_rollup(meta_byte(meta_data, 10) & 0x08 != 0);

            // Java: task.setUniqueID(uniqueID);
            task.borrow_mut().set_unique_id(Some(unique_id));

            // Java: switch (task.getConstraintType()) { … }
            // Take the value out first and release the borrow: the temporary from the `match` scrutinee lives until the end of the last statement,
            // otherwise the `borrow_mut()` in the branch below would hit an unreleased `borrow()`
            let constraint_type = task.borrow().get_constraint_type();
            match constraint_type {
                //
                // Adjust the start and finish dates if the task
                // is constrained to start as late as possible.
                //
                Some(ConstraintType::AsLateAsPossible) => {
                    let (start, late_start) = {
                        let task_ref = task.borrow();
                        (task_ref.get_start(), task_ref.get_late_start())
                    };
                    // Java: if (LocalDateTimeHelper.compare(task.getStart(), task.getLateStart()) < 0) { task.setStart(task.getLateStart()); }
                    if LocalDateTimeHelper::compare(start, late_start) < 0 {
                        task.borrow_mut().set_start(late_start);
                    }

                    let (finish, late_finish) = {
                        let task_ref = task.borrow();
                        (task_ref.get_finish(), task_ref.get_late_finish())
                    };
                    if LocalDateTimeHelper::compare(finish, late_finish) < 0 {
                        task.borrow_mut().set_finish(late_finish);
                    }
                }

                Some(ConstraintType::StartNoLaterThan) | Some(ConstraintType::FinishNoLaterThan) => {
                    let (finish, start, late_finish) = {
                        let task_ref = task.borrow();
                        (
                            task_ref.get_finish(),
                            task_ref.get_start(),
                            task_ref.get_late_finish(),
                        )
                    };
                    if LocalDateTimeHelper::compare(finish, start) < 0 {
                        task.borrow_mut().set_finish(late_finish);
                    }
                }

                _ => {}
            }

            //
            // Retrieve task recurring data
            //
            // Java: if (recurringData != null) { … }
            if let Some(recurring_data) = recurring_data {
                if recurring_task_reader.is_none() {
                    // Java: recurringTaskReader = new RecurringTaskReader(m_file.getProjectProperties());
                    let reader = {
                        let properties = try_file(&self.file)?;
                        RecurringTaskReader::new(properties.get_project_properties())
                    };
                    recurring_task_reader = Some(reader);
                }

                if let Some(recurring_task_reader) = &recurring_task_reader {
                    // Java: recurringTaskReader.processRecurringTask(task, recurringData);
                    // Java's processRecurringTask writes into the task directly; the Rust side returns the instance and the caller mounts it
                    // (see the file header of `recurring_task_reader.rs`). The returned instance is the same type held by `Task`.
                    let recurring_task = recurring_task_reader.process_recurring_task(&recurring_data);
                    {
                        let mut task_ref = task.borrow_mut();
                        *task_ref.add_recurring_task() = recurring_task;
                    }
                }

                // Java: task.setRecurring(true);
                task.borrow_mut().set_recurring(true);
            }

            //
            // Configure the calendar
            //
            // Java: Integer calendarID = (Integer) task.getCachedValue(TaskField.CALENDAR_UNIQUE_ID);
            let calendar_id = task
                .borrow()
                .get_cached_value(FieldKey::Task(TaskField::CalendarUniqueId))
                .and_then(FieldValue::as_integer);
            if let Some(calendar_id) = calendar_id {
                if calendar_id == -1 {
                    // Java: task.setCalendarUniqueID(null);
                    task.borrow_mut().set_calendar_unique_id(None);
                } else {
                    // Java: ProjectCalendar calendar = m_file.getCalendarByUniqueID(calendarID);
                    //       if (calendar != null) { task.setCalendar(calendar); }
                    let calendar = try_file(&self.file)?.get_calendar_by_unique_id(Some(calendar_id));
                    if let Some(calendar) = calendar {
                        task.borrow_mut().set_calendar(Some(&calendar));
                    }
                }
            }

            //
            // Set the subproject and external task flag
            //
            // Java: String sp = m_taskSubProjects.get(task.getUniqueID());
            let sp = self.task_sub_projects.get(&unique_id).cloned();
            if let Some(sp) = sp {
                // Java: task.setSubprojectFile(sp);
                task.borrow_mut().set_subproject_file(Some(sp));

                // Java: Integer subprojectTaskUniqueID = task.getSubprojectTaskUniqueID();
                //       if (subprojectTaskUniqueID != null) { task.setSubprojectTaskUniqueID(subprojectTaskUniqueID & 0xFFFF); }
                // Take the value out first and release the borrow, to avoid another `borrow_mut()` inside the `if let` scope
                let subproject_task_unique_id = task.borrow().get_subproject_task_unique_id();
                if let Some(subproject_task_unique_id) = subproject_task_unique_id {
                    task.borrow_mut()
                        .set_subproject_task_unique_id(Some(subproject_task_unique_id & 0xFFFF));
                }
            }

            // Java: if (m_externalTasks.contains(task.getUniqueID()) && NumberHelper.getInt(task.getSubprojectTaskUniqueID()) != 0) { … }
            if self.external_tasks.contains(&unique_id)
                && task.borrow().get_subproject_task_unique_id().unwrap_or(0) != 0
            {
                // The condition preserves external tasks which no longer have an associated subproject filename
                let is_external = self.external_tasks.contains(&unique_id);
                task.borrow_mut().set_external_task(is_external);
            }

            //
            // If we have a WBS value from the MPP file, don't autogenerate
            //
            // Java: if (task.getWBS() != null) { autoWBS = false; }
            if task.borrow().get_wbs().is_some() {
                auto_wbs = false;
            }

            //
            // Process any enterprise columns
            //
            // Java: processTaskEnterpriseColumns(fieldMap, task, taskVarData);
            self.process_task_enterprise_columns(&field_map, &task, &task_var_data)?;

            //
            // Unfortunately it looks like 'null' tasks sometimes make it through,
            // so let's check for to see if we need to mark this task as a null
            // task after all.
            //
            let (name, start, finish, create_date) = {
                let task_ref = task.borrow();
                (
                    task_ref.get_name(),
                    task_ref.get_start(),
                    task_ref.get_finish(),
                    task_ref.get_create_date(),
                )
            };
            let epoch = epoch_date();
            let is_epoch = |value: Option<chrono::NaiveDateTime>| {
                value.is_none() || value == Some(epoch)
            };
            if name.is_none() && (is_epoch(start) || is_epoch(finish) || is_epoch(create_date)) {
                // Java: m_file.removeTask(task); task = m_file.addTask(); task.setNull(true); … continue;
                try_file_mut(&self.file)?.remove_task(&task);

                let null_task = try_file_mut(&self.file)?.add_task();
                {
                    let mut task_ref = null_task.borrow_mut();
                    task_ref.set_null(true);
                    task_ref.set_unique_id(Some(get_int_by_offset(
                        data,
                        TASK_UNIQUE_ID_FIXED_OFFSET as i32,
                    )));
                    task_ref.set_id(Some(get_int_by_offset(data, TASK_ID_FIXED_OFFSET as i32)));
                    self.null_task_order.insert(
                        task_ref.get_id().unwrap_or(0),
                        task_ref.get_unique_id().unwrap_or(0),
                    );
                }

                continue;
            }

            // Java: m_taskOrder.put(task.getID(), task.getUniqueID());
            self.task_order.insert(
                task.borrow().get_id().unwrap_or(0),
                task.borrow().get_unique_id().unwrap_or(0),
            );

            //
            // Fire the task read event
            //
            // Java: m_eventManager.fireTaskReadEvent(task);
            fire_task_read_event(&self.file, &task)?;
        }

        //
        // Enable auto WBS if necessary
        //
        // Java: m_file.getProjectConfig().setAutoWBS(autoWBS);
        {
            let context = try_file(&self.file)?.get_project_context();
            context
                .try_borrow_mut()
                .map_err(|_| borrow_error("ProjectContext"))?
                .get_project_config_mut()
                .set_auto_wbs(auto_wbs);
        }

        //
        // We have now read all of the tasks, so we are in a position
        // to perform post-processing to set up the relevant details
        // for each external task.
        //
        // Java: if (!externalTasks.isEmpty()) { processExternalTasks(externalTasks); }
        if !external_tasks.is_empty() {
            self.process_external_tasks(&mut external_tasks);
        }

        Ok(())
    }

    /// Extract the task enterprise column values.
    ///
    /// Corresponds to Java: `private void processTaskEnterpriseColumns(FieldMap fieldMap, Task task, Var2Data taskVarData)`
    ///
    /// Java has no `throws`; the Rust side returns `MpxjResult` because it needs to borrow the project properties (duration conversion).
    fn process_task_enterprise_columns(
        &self,
        field_map: &FieldMap,
        task: &Rc<RefCell<Task>>,
        task_var_data: &Var2Data,
    ) -> MpxjResult<()> {
        // Java: byte[] data = null; Integer varDataKey = fieldMap.getVarDataKey(TaskField.ENTERPRISE_DATA);
        let var_data_key = field_map.get_var_data_key(&task_field(TaskField::EnterpriseData));

        // Java: if (varDataKey != null) { data = taskVarData.getByteArray(task.getUniqueID(), varDataKey); }
        let data = if var_data_key.is_some() {
            task_var_data.get_byte_array(task.borrow().get_unique_id(), var_data_key)
        } else {
            None
        };

        // Java: if (data == null) { return; }
        let Some(data) = data else {
            return Ok(());
        };

        // Java: PropsBlock props = new PropsBlock(data);
        let props = PropsBlock::new(&data);

        // Java: for (Integer key : props.keySet()) { … }
        for key in props.key_set() {
            // Java: FieldType field = FieldTypeHelper.getInstance(m_file, key.intValue());
            //       if (field == null || field.getDataType() == null) { continue; }
            // (Rust's `MppFieldType::get_data_type()` always has a value, so only a null check is needed)
            let Some(field) = FieldTypeHelper::get_instance(&self.file, key) else {
                continue;
            };

            // Java: Object value = null;
            let mut value: Option<FieldValue> = None;

            // Java sets `field` to null in the BOOLEAN branch, so the `task.set(field, value)` at the end of the loop
            // is actually `task.set(null, null)` (writing a null key into the HashMap, with no real effect).
            // Rust has no null key, so a flag is used to skip the write at the end.
            let mut skip_final_set = false;

            match field.get_data_type() {
                DataType::Currency => {
                    // Java: value = Double.valueOf(props.getDouble(key) / 100);
                    value = Some(FieldValue::Double(props.get_double_by_type(key) / 100.0));
                }

                DataType::Date => {
                    // Java: value = props.getTimestamp(key);
                    value = props.get_timestamp_by_type(key).map(FieldValue::DateTime);
                }

                DataType::Work => {
                    // Java: double durationValueInHours = MPPUtility.getDouble(props.getByteArray(key), 0) / 60000;
                    //       value = Duration.getInstance(durationValueInHours, TimeUnit.HOURS);
                    let work_data = props.get_byte_array_by_type(key).unwrap_or(&[]);
                    let duration_value_in_hours = MPPUtility::get_double(work_data, 0) / 60000.0;
                    value = Some(FieldValue::Duration(Duration::get_instance(
                        duration_value_in_hours,
                        TimeUnit::Hours,
                    )));
                }

                DataType::Duration => {
                    // Java: byte[] durationData = props.getByteArray(key);
                    //       double durationValueInHours = ((double) ByteArrayHelper.getInt(durationData, 0)) / 600;
                    let duration_data = props.get_byte_array_by_type(key).unwrap_or(&[]);
                    let duration_value_in_hours =
                        f64::from(get_int_safe(duration_data, 0)) / 600.0;

                    // Java: TimeUnit durationUnits = durationData.length < 6 ? TimeUnit.DAYS
                    //          : MPPUtility.getDurationTimeUnits(ByteArrayHelper.getShort(durationData, 4));
                    let duration_units = if duration_data.len() < 6 {
                        TimeUnit::Days
                    } else {
                        MPPUtility::get_duration_time_units(get_short_safe(duration_data, 4))
                    };

                    // Java: Duration duration = Duration.getInstance(durationValueInHours, TimeUnit.HOURS);
                    //       value = duration.convertUnits(durationUnits, m_file.getProjectProperties());
                    let duration = Duration::get_instance(duration_value_in_hours, TimeUnit::Hours);
                    let properties = try_file(&self.file)?;
                    let properties = Ref::map(properties, |file| file.get_project_properties());
                    value = Some(FieldValue::Duration(
                        duration.convert_units(duration_units, &*properties),
                    ));
                }

                DataType::Boolean => {
                    // Java: field = null; int bits = props.getInt(key);
                    //       task.set(TaskField.ENTERPRISE_FLAG1, Boolean.valueOf((bits & 0x00002) != 0)); … FLAG20
                    skip_final_set = true;

                    let bits = props.get_int_by_type(key);
                    let mut task_ref = task.borrow_mut();
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag1),
                        Some(FieldValue::Boolean(bits & 0x00002 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag2),
                        Some(FieldValue::Boolean(bits & 0x00004 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag3),
                        Some(FieldValue::Boolean(bits & 0x00008 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag4),
                        Some(FieldValue::Boolean(bits & 0x00010 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag5),
                        Some(FieldValue::Boolean(bits & 0x00020 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag6),
                        Some(FieldValue::Boolean(bits & 0x00040 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag7),
                        Some(FieldValue::Boolean(bits & 0x00080 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag8),
                        Some(FieldValue::Boolean(bits & 0x00100 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag9),
                        Some(FieldValue::Boolean(bits & 0x00200 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag10),
                        Some(FieldValue::Boolean(bits & 0x00400 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag11),
                        Some(FieldValue::Boolean(bits & 0x00800 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag12),
                        Some(FieldValue::Boolean(bits & 0x01000 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag13),
                        Some(FieldValue::Boolean(bits & 0x02000 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag14),
                        Some(FieldValue::Boolean(bits & 0x04000 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag15),
                        Some(FieldValue::Boolean(bits & 0x08000 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag16),
                        Some(FieldValue::Boolean(bits & 0x10000 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag17),
                        Some(FieldValue::Boolean(bits & 0x20000 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag18),
                        Some(FieldValue::Boolean(bits & 0x40000 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag19),
                        Some(FieldValue::Boolean(bits & 0x80000 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag20),
                        Some(FieldValue::Boolean(bits & 0x100000 != 0)),
                    );
                }

                DataType::Numeric => {
                    // Java: value = Double.valueOf(props.getDouble(key));
                    value = Some(FieldValue::Double(props.get_double_by_type(key)));
                }

                DataType::String => {
                    // Java: value = props.getUnicodeString(key);
                    value = props
                        .get_unicode_string_by_type(key)
                        .map(FieldValue::String);
                }

                DataType::Percentage => {
                    // Java: value = Integer.valueOf(props.getShort(key));
                    value = Some(FieldValue::Integer(props.get_short_by_type(key)));
                }

                _ => {}
            }

            // Java: task.set(field, value);
            if !skip_final_set {
                task.borrow_mut().set(field.to_field_key(), value);
            }
        }

        Ok(())
    }

    /// Extract the resource enterprise column data.
    ///
    /// Corresponds to Java: `private void processResourceEnterpriseColumns(FieldMap fieldMap, Resource resource, Var2Data resourceVarData)`
    fn process_resource_enterprise_columns(
        &self,
        field_map: &FieldMap,
        resource: &Rc<RefCell<Resource>>,
        resource_var_data: &Var2Data,
    ) -> MpxjResult<()> {
        // Java: byte[] data = null; Integer varDataKey = fieldMap.getVarDataKey(ResourceField.ENTERPRISE_DATA);
        let var_data_key = field_map.get_var_data_key(&resource_field(ResourceField::EnterpriseData));

        // Java: if (varDataKey != null) { data = resourceVarData.getByteArray(resource.getUniqueID(), varDataKey); }
        let data = if var_data_key.is_some() {
            resource_var_data.get_byte_array(resource.borrow().get_unique_id(), var_data_key)
        } else {
            None
        };

        // Java: if (data == null) { return; }
        let Some(data) = data else {
            return Ok(());
        };

        // Java: PropsBlock props = new PropsBlock(data);
        let props = PropsBlock::new(&data);

        // Java: resource.setCreationDate(props.getTimestamp(PropsKey.RESOURCE_CREATION_DATE));
        resource
            .borrow_mut()
            .set_creation_date(props.get_timestamp(PropsKey::ResourceCreationDate));

        // Java: for (Integer key : props.keySet()) { … }
        for key in props.key_set() {
            // Java: FieldType field = FieldTypeHelper.getInstance(m_file, key.intValue());
            let Some(field) = FieldTypeHelper::get_instance(&self.file, key) else {
                continue;
            };

            // Java: Object value = null;
            let mut value: Option<FieldValue> = None;

            // Same as on the task side: Java sets `field` to null in the BOOLEAN branch.
            let mut skip_final_set = false;

            match field.get_data_type() {
                DataType::Currency => {
                    value = Some(FieldValue::Double(props.get_double_by_type(key) / 100.0));
                }

                DataType::Date => {
                    value = props.get_timestamp_by_type(key).map(FieldValue::DateTime);
                }

                DataType::Duration => {
                    let duration_data = props.get_byte_array_by_type(key).unwrap_or(&[]);
                    let duration_value_in_hours =
                        f64::from(get_int_safe(duration_data, 0)) / 600.0;

                    let duration_units = if duration_data.len() < 6 {
                        TimeUnit::Days
                    } else {
                        MPPUtility::get_duration_time_units(get_short_safe(duration_data, 4))
                    };

                    let duration = Duration::get_instance(duration_value_in_hours, TimeUnit::Hours);
                    let properties = try_file(&self.file)?;
                    let properties = Ref::map(properties, |file| file.get_project_properties());
                    value = Some(FieldValue::Duration(
                        duration.convert_units(duration_units, &*properties),
                    ));
                }

                DataType::Boolean => {
                    // Java: if (field == ResourceField.FLAG1) { field = null; …20 ENTERPRISE_FLAG… }
                    if field.to_field_key() == FieldKey::Resource(ResourceField::Flag1) {
                        skip_final_set = true;

                        let bits = props.get_int_by_type(key);
                        let mut resource_ref = resource.borrow_mut();
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag1),
                            Some(FieldValue::Boolean(bits & 0x00002 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag2),
                            Some(FieldValue::Boolean(bits & 0x00004 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag3),
                            Some(FieldValue::Boolean(bits & 0x00008 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag4),
                            Some(FieldValue::Boolean(bits & 0x00010 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag5),
                            Some(FieldValue::Boolean(bits & 0x00020 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag6),
                            Some(FieldValue::Boolean(bits & 0x00040 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag7),
                            Some(FieldValue::Boolean(bits & 0x00080 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag8),
                            Some(FieldValue::Boolean(bits & 0x00100 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag9),
                            Some(FieldValue::Boolean(bits & 0x00200 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag10),
                            Some(FieldValue::Boolean(bits & 0x00400 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag11),
                            Some(FieldValue::Boolean(bits & 0x00800 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag12),
                            Some(FieldValue::Boolean(bits & 0x01000 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag13),
                            Some(FieldValue::Boolean(bits & 0x02000 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag14),
                            Some(FieldValue::Boolean(bits & 0x04000 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag15),
                            Some(FieldValue::Boolean(bits & 0x08000 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag16),
                            Some(FieldValue::Boolean(bits & 0x10000 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag17),
                            Some(FieldValue::Boolean(bits & 0x20000 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag18),
                            Some(FieldValue::Boolean(bits & 0x40000 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag19),
                            Some(FieldValue::Boolean(bits & 0x80000 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag20),
                            Some(FieldValue::Boolean(bits & 0x100000 != 0)),
                        );
                    }

                    // Java: if (field == ResourceField.GENERIC) { field = null; resource.setGeneric(props.getShort(key) != 0); }
                    if field.to_field_key() == FieldKey::Resource(ResourceField::Generic) {
                        skip_final_set = true;
                        let generic = props.get_short_by_type(key) != 0;
                        resource.borrow_mut().set_generic(generic);
                    }
                }

                DataType::Numeric => {
                    value = Some(FieldValue::Double(props.get_double_by_type(key)));
                }

                DataType::String => {
                    value = props
                        .get_unicode_string_by_type(key)
                        .map(FieldValue::String);
                }

                _ => {}
            }

            // Java: resource.set(field, value);
            if !skip_final_set {
                resource.borrow_mut().set(field.to_field_key(), value);
            }
        }

        Ok(())
    }

    /// In a project file, the file that each external task belongs to is not attached to the task; instead there is a notion of the "current external task file"
    /// (that is, the last one used). This method walks the tasks marked as external and associates the correct external project data
    /// with the tasks as far as possible.
    ///
    /// Corresponds to Java: `private void processExternalTasks(List<Task> externalTasks)`
    fn process_external_tasks(&self, external_tasks: &mut [Rc<RefCell<Task>>]) {
        //
        // Sort the list of tasks into ID order
        //
        // Java: Collections.sort(externalTasks);
        external_tasks.sort_by(|left, right| {
            let left = left.borrow();
            let right = right.borrow();
            left.compare_to(&right)
        });

        //
        // Find any external tasks which don't have a sub project
        // object, and set this attribute using the most recent value.
        //
        // Java: String currentSubProject = null;
        let mut current_sub_project: Option<String> = None;

        for current_task in external_tasks.iter() {
            // Java: String sp = currentTask.getSubprojectFile();
            let sp = current_task.borrow().get_subproject_file();
            if sp.is_none() {
                if let Some(current) = &current_sub_project {
                    // Java: currentTask.setSubprojectFile(currentSubProject);
                    current_task.borrow_mut().set_subproject_file(Some(current.clone()));
                }
            } else {
                current_sub_project = sp;
            }

            if let Some(current) = &current_sub_project {
                // Java: currentTask.setProject(currentSubProject);
                current_task.borrow_mut().set_project(Some(current.clone()));
            }
        }
    }

    /// Process the relation (constraint) data.
    ///
    /// Corresponds to Java: `private void processConstraintData() throws IOException`
    fn process_constraint_data(&mut self) -> MpxjResult<()> {
        // Java: ConstraintFactory factory = new ConstraintFactory();
        //       factory.process(m_projectDir, m_file, m_inputStreamFactory);
        let factory = ConstraintFactory;
        factory.process(&self.project_dir, &self.file, &self.input_stream_factory)
    }

    /// Extract and organize the resource data.
    ///
    /// Corresponds to Java: `private void processResourceData() throws IOException`
    fn process_resource_data(&mut self) -> MpxjResult<()> {
        // Java: FieldMap fieldMap = new FieldMap9(m_file); fieldMap.createResourceFieldMap(m_projectProps);
        let mut field_map = FieldMap9::new(&self.file);
        field_map.create_resource_field_map(&self.project_props);

        // Java: DirectoryEntry rscDir = (DirectoryEntry) m_projectDir.getEntry("TBkndRsc");
        let rsc_dir = self
            .project_dir
            .get_directory(RESOURCE_DIRECTORY)
            .ok_or_else(|| missing_entry(RESOURCE_DIRECTORY))?;

        // Java: VarMeta rscVarMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) rscDir.getEntry("VarMeta"))));
        let rsc_var_meta: Rc<dyn VarMeta> = {
            let data = rsc_dir
                .get_document("VarMeta")
                .ok_or_else(|| missing_entry("VarMeta"))?
                .get_data();
            Rc::new(VarMeta9::new(&data)?)
        };

        // Java: Var2Data rscVarData = new Var2Data(m_file, rscVarMeta, new DocumentInputStream(((DocumentEntry) rscDir.getEntry("Var2Data"))));
        let rsc_var_data = {
            let data = rsc_dir
                .get_document("Var2Data")
                .ok_or_else(|| missing_entry("Var2Data"))?
                .get_data();
            Var2Data::new(&self.file, Rc::clone(&rsc_var_meta), &data)
        };

        // Java: FixedMeta rscFixedMeta = new FixedMeta(new DocumentInputStream(((DocumentEntry) rscDir.getEntry("FixedMeta"))), 37);
        let rsc_fixed_meta = {
            let data = rsc_dir
                .get_document("FixedMeta")
                .ok_or_else(|| missing_entry("FixedMeta"))?
                .get_data();
            FixedMeta::with_item_size(&data, 37).map_err(MpxjError::from)?
        };

        // Java: FixedData rscFixedData = new FixedData(rscFixedMeta, m_inputStreamFactory.getInstance(rscDir, "FixedData"));
        let rsc_fixed_data = {
            let mut stream = self.input_stream_factory.get_instance(&rsc_dir, "FixedData")?;
            let bytes = stream.to_bytes();
            FixedData::new(&rsc_fixed_meta, &bytes)
        };

        // Java: processFieldNameAliases(RESOURCE_FIELD_ALIASES, m_projectProps.getByteArray(PropsKey.RESOURCE_FIELD_NAME_ALIASES));
        let aliases = self
            .project_props
            .get_byte_array(PropsKey::ResourceFieldNameAliases)
            .map(<[u8]>::to_vec);
        let alias_map = resource_field_aliases();
        self.process_field_name_aliases(&alias_map, aliases.as_deref());

        // Java: TreeMap<Integer, Integer> resourceMap = createResourceMap(fieldMap, rscFixedMeta, rscFixedData);
        let resource_map = self.create_resource_map(&field_map, &rsc_fixed_meta, &rsc_fixed_data);

        // Java: Integer[] uniqueid = rscVarMeta.getUniqueIdentifierArray();
        let unique_ids = rsc_var_meta.get_unique_identifier_array();

        // Java: HyperlinkReader hyperlinkReader = new HyperlinkReader();
        let hyperlink_reader = HyperlinkReader::new();

        // Java: CostRateTableFactory crt = new CostRateTableFactory(m_file); (Java creates it inside the loop; Rust hoists it outside)
        let cost_rate_factory = CostRateTableFactory::new(Rc::clone(&self.file));

        // Java: AvailabilityFactory af = new AvailabilityFactory(); (same as above)
        let availability_factory = AvailabilityFactory::new();

        // Java: for (Integer id : uniqueid) { … }
        for id in unique_ids {
            // Java: offset = resourceMap.get(id); if (offset == null) { continue; }
            let Some(offset) = resource_map.get(&id).copied() else {
                continue;
            };

            // Java: data = rscFixedData.getByteArrayValue(offset.intValue());
            let Some(data) = rsc_fixed_data.get_byte_array_value(offset) else {
                continue;
            };

            // Java: resource = m_file.addResource();
            let resource = try_file_mut(&self.file)?.add_resource();

            // Java: resource.disableEvents();
            //       fieldMap.populateContainer(FieldTypeClass.RESOURCE, resource, id, new byte[][]{data}, rscVarData);
            //       resource.enableEvents();
            resource.borrow_mut().disable_events();
            field_map.populate_container(
                FieldTypeClass::Resource,
                &mut *resource.borrow_mut(),
                Some(id),
                &[Some(data)],
                &rsc_var_data,
            );
            resource.borrow_mut().enable_events();

            // Java: hyperlinkReader.read(resource, rscVarData.getByteArray(id, fieldMap.getVarDataKey(ResourceField.HYPERLINK_DATA)));
            if let Some(hyperlink_data) = rsc_var_data.get_byte_array(
                Some(id),
                field_map.get_var_data_key(&resource_field(ResourceField::HyperlinkData)),
            ) {
                hyperlink_reader.read(
                    FieldTypeClass::Resource,
                    &mut *resource.borrow_mut(),
                    &hyperlink_data,
                );
            }

            // Java: resource.setID(Integer.valueOf(ByteArrayHelper.getInt(data, 4)));
            resource.borrow_mut().set_id(Some(get_int_safe(data, 4)));

            // Java: resource.setOutlineCode(1, m_outlineCodeVarData.getUnicodeString(
            //          (Integer) resource.getCachedValue(ResourceField.OUTLINE_CODE1_INDEX), OUTLINECODE_DATA)); … 10
            let value = self.resource_outline_code(&resource, ResourceField::OutlineCode1Index);
            resource.borrow_mut().set_outline_code_at(1, value);
            let value = self.resource_outline_code(&resource, ResourceField::OutlineCode2Index);
            resource.borrow_mut().set_outline_code_at(2, value);
            let value = self.resource_outline_code(&resource, ResourceField::OutlineCode3Index);
            resource.borrow_mut().set_outline_code_at(3, value);
            let value = self.resource_outline_code(&resource, ResourceField::OutlineCode4Index);
            resource.borrow_mut().set_outline_code_at(4, value);
            let value = self.resource_outline_code(&resource, ResourceField::OutlineCode5Index);
            resource.borrow_mut().set_outline_code_at(5, value);
            let value = self.resource_outline_code(&resource, ResourceField::OutlineCode6Index);
            resource.borrow_mut().set_outline_code_at(6, value);
            let value = self.resource_outline_code(&resource, ResourceField::OutlineCode7Index);
            resource.borrow_mut().set_outline_code_at(7, value);
            let value = self.resource_outline_code(&resource, ResourceField::OutlineCode8Index);
            resource.borrow_mut().set_outline_code_at(8, value);
            let value = self.resource_outline_code(&resource, ResourceField::OutlineCode9Index);
            resource.borrow_mut().set_outline_code_at(9, value);
            let value = self.resource_outline_code(&resource, ResourceField::OutlineCode10Index);
            resource.borrow_mut().set_outline_code_at(10, value);

            // Java: resource.setUniqueID(id);
            resource.borrow_mut().set_unique_id(Some(id));

            // Java: metaData = rscFixedMeta.getByteArrayValue(offset.intValue());
            let meta_data: &[u8] = rsc_fixed_meta.get_byte_array_value(offset).unwrap_or(&[]);

            // Java: resource.setFlag(1, (metaData[28] & 0x40) != 0); … resource.setFlag(20, (metaData[31] & 0x01) != 0);
            {
                let mut resource_ref = resource.borrow_mut();
                resource_ref.set_flag_at(1, meta_byte(meta_data, 28) & 0x40 != 0);
                resource_ref.set_flag_at(2, meta_byte(meta_data, 28) & 0x80 != 0);
                resource_ref.set_flag_at(3, meta_byte(meta_data, 29) & 0x01 != 0);
                resource_ref.set_flag_at(4, meta_byte(meta_data, 29) & 0x02 != 0);
                resource_ref.set_flag_at(5, meta_byte(meta_data, 29) & 0x04 != 0);
                resource_ref.set_flag_at(6, meta_byte(meta_data, 29) & 0x08 != 0);
                resource_ref.set_flag_at(7, meta_byte(meta_data, 29) & 0x10 != 0);
                resource_ref.set_flag_at(8, meta_byte(meta_data, 29) & 0x20 != 0);
                resource_ref.set_flag_at(9, meta_byte(meta_data, 29) & 0x40 != 0);
                resource_ref.set_flag_at(10, meta_byte(meta_data, 28) & 0x20 != 0);
                resource_ref.set_flag_at(11, meta_byte(meta_data, 29) & 0x80 != 0);
                resource_ref.set_flag_at(12, meta_byte(meta_data, 30) & 0x01 != 0);
                resource_ref.set_flag_at(13, meta_byte(meta_data, 30) & 0x02 != 0);
                resource_ref.set_flag_at(14, meta_byte(meta_data, 30) & 0x04 != 0);
                resource_ref.set_flag_at(15, meta_byte(meta_data, 30) & 0x08 != 0);
                resource_ref.set_flag_at(16, meta_byte(meta_data, 30) & 0x10 != 0);
                resource_ref.set_flag_at(17, meta_byte(meta_data, 30) & 0x20 != 0);
                resource_ref.set_flag_at(18, meta_byte(meta_data, 30) & 0x40 != 0);
                resource_ref.set_flag_at(19, meta_byte(meta_data, 30) & 0x80 != 0);
                resource_ref.set_flag_at(20, meta_byte(meta_data, 31) & 0x01 != 0);
            }

            //
            // Configure the resource calendar
            //
            // Java: resource.setCalendar(m_resourceMap.get(id));
            let calendar = self.resource_map.get(&id).cloned();
            resource.borrow_mut().set_calendar(calendar.as_ref());

            //
            // Process any enterprise columns
            //
            // Java: processResourceEnterpriseColumns(fieldMap, resource, rscVarData);
            self.process_resource_enterprise_columns(&field_map, &resource, &rsc_var_data)?;

            //
            // Convert rate units
            //
            // Java:
            // MPPUtility.convertRateFromHours(m_file, resource, ResourceField.STANDARD_RATE, ResourceField.STANDARD_RATE_UNITS);
            // MPPUtility.convertRateFromHours(m_file, resource, ResourceField.OVERTIME_RATE, ResourceField.OVERTIME_RATE_UNITS);
            // Note: the units field stores a `TimeUnit` as an integer, and [`MPPUtility::convert_rate_from_hours`] uses it internally
            //       to convert the "amount per hour" into the target units via `RateHelper::convert_from_hours`.
            {
                let mut resource_ref = resource.borrow_mut();
                MPPUtility::convert_rate_from_hours(
                    &self.file,
                    &mut resource_ref,
                    ResourceField::StandardRate,
                    ResourceField::StandardRateUnits,
                );
                MPPUtility::convert_rate_from_hours(
                    &self.file,
                    &mut resource_ref,
                    ResourceField::OvertimeRate,
                    ResourceField::OvertimeRateUnits,
                );
            }

            //
            // Process cost rate tables
            //
            // Java: CostRateTableFactory crt = new CostRateTableFactory(m_file);
            //       crt.process(resource, 0, rscVarData.getByteArray(id, fieldMap.getVarDataKey(ResourceField.COST_RATE_A))); … E
            let cost_rate_a = rsc_var_data.get_byte_array(
                Some(id),
                field_map.get_var_data_key(&resource_field(ResourceField::CostRateA)),
            );
            cost_rate_factory.process(&mut resource.borrow_mut(), 0, cost_rate_a.as_deref());

            let cost_rate_b = rsc_var_data.get_byte_array(
                Some(id),
                field_map.get_var_data_key(&resource_field(ResourceField::CostRateB)),
            );
            cost_rate_factory.process(&mut resource.borrow_mut(), 1, cost_rate_b.as_deref());

            let cost_rate_c = rsc_var_data.get_byte_array(
                Some(id),
                field_map.get_var_data_key(&resource_field(ResourceField::CostRateC)),
            );
            cost_rate_factory.process(&mut resource.borrow_mut(), 2, cost_rate_c.as_deref());

            let cost_rate_d = rsc_var_data.get_byte_array(
                Some(id),
                field_map.get_var_data_key(&resource_field(ResourceField::CostRateD)),
            );
            cost_rate_factory.process(&mut resource.borrow_mut(), 3, cost_rate_d.as_deref());

            let cost_rate_e = rsc_var_data.get_byte_array(
                Some(id),
                field_map.get_var_data_key(&resource_field(ResourceField::CostRateE)),
            );
            cost_rate_factory.process(&mut resource.borrow_mut(), 4, cost_rate_e.as_deref());

            //
            // Process availability table
            //
            // Java: AvailabilityFactory af = new AvailabilityFactory();
            //       af.process(resource, rscVarData.getByteArray(id, fieldMap.getVarDataKey(ResourceField.AVAILABILITY_DATA)));
            // Note: `AvailabilityFactory::process` by convention returns "the rows that should be appended to the availability table"; here they are
            //       written back via `Resource::add_availability` (Java appends directly to `resource.getAvailability()`).
            let availability_data = rsc_var_data.get_byte_array(
                Some(id),
                field_map.get_var_data_key(&resource_field(ResourceField::AvailabilityData)),
            );
            let availability_entries =
                availability_factory.process(&resource.borrow(), availability_data.as_deref());
            for entry in availability_entries {
                resource.borrow_mut().add_availability(entry);
            }

            //
            // Process resource type
            //
            // Java: if ((metaData[9] & 0x02) != 0) { resource.setType(ResourceType.WORK); }
            //       else { resource.setType(ResourceType.MATERIAL); }
            if meta_byte(meta_data, 9) & 0x02 != 0 {
                resource.borrow_mut().set_type(ResourceType::Work);
            } else {
                resource.borrow_mut().set_type(ResourceType::Material);
            }

            // Java: m_eventManager.fireResourceReadEvent(resource);
            fire_resource_read_event(&self.file, &resource)?;
        }

        Ok(())
    }

    /// Extract and organize the resource assignment data.
    ///
    /// Corresponds to Java: `private void processAssignmentData() throws IOException`
    fn process_assignment_data(&mut self) -> MpxjResult<()> {
        // Java: FieldMap fieldMap = new FieldMap9(m_file); fieldMap.createAssignmentFieldMap(m_projectProps);
        let mut field_map = FieldMap9::new(&self.file);
        field_map.create_assignment_field_map(&self.project_props);

        // Java: DirectoryEntry assnDir = (DirectoryEntry) m_projectDir.getEntry("TBkndAssn");
        let assn_dir = self
            .project_dir
            .get_directory(ASSIGNMENT_DIRECTORY)
            .ok_or_else(|| missing_entry(ASSIGNMENT_DIRECTORY))?;

        // Java: VarMeta assnVarMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) assnDir.getEntry("VarMeta"))));
        let assn_var_meta: Rc<dyn VarMeta> = {
            let data = assn_dir
                .get_document("VarMeta")
                .ok_or_else(|| missing_entry("VarMeta"))?
                .get_data();
            Rc::new(VarMeta9::new(&data)?)
        };

        // Java: Var2Data assnVarData = new Var2Data(m_file, assnVarMeta, new DocumentInputStream(((DocumentEntry) assnDir.getEntry("Var2Data"))));
        let assn_var_data = {
            let data = assn_dir
                .get_document("Var2Data")
                .ok_or_else(|| missing_entry("Var2Data"))?
                .get_data();
            Var2Data::new(&self.file, Rc::clone(&assn_var_meta), &data)
        };

        // Java: FixedMeta assnFixedMeta = new FixedMeta(new DocumentInputStream(((DocumentEntry) assnDir.getEntry("FixedMeta"))), 34);
        let assn_fixed_meta = {
            let data = assn_dir
                .get_document("FixedMeta")
                .ok_or_else(|| missing_entry("FixedMeta"))?
                .get_data();
            FixedMeta::with_item_size(&data, 34).map_err(MpxjError::from)?
        };

        // Java: FixedData assnFixedData = new FixedData(142, m_inputStreamFactory.getInstance(assnDir, "FixedData"));
        let assn_fixed_data = {
            let mut stream = self.input_stream_factory.get_instance(&assn_dir, "FixedData")?;
            let bytes = stream.to_bytes();
            FixedData::with_fixed_size(142, &bytes)
        };

        // Java: if (assnFixedData.getItemCount() != assnFixedMeta.getAdjustedItemCount()) {
        //          assnFixedData = new FixedData(assnFixedMeta, m_inputStreamFactory.getInstance(assnDir, "FixedData")); }
        let assn_fixed_data = if assn_fixed_data.get_item_count()
            != assn_fixed_meta.get_adjusted_item_count()
        {
            let mut stream = self.input_stream_factory.get_instance(&assn_dir, "FixedData")?;
            let bytes = stream.to_bytes();
            FixedData::new(&assn_fixed_meta, &bytes)
        } else {
            assn_fixed_data
        };

        // Java: ResourceAssignmentFactory factory = new ResourceAssignmentFactory();
        //       factory.process(m_file, fieldMap, null, null, m_reader.getUseRawTimephasedData(),
        //          assnVarMeta, assnVarData, assnFixedMeta, assnFixedData, null, assnFixedMeta.getAdjustedItemCount());
        let factory = ResourceAssignmentFactory;
        factory.process(
            &self.file,
            &field_map,
            None,
            None,
            self.reader.get_use_raw_timephased_data(),
            &*assn_var_meta,
            &assn_var_data,
            &assn_fixed_meta,
            &assn_fixed_data,
            None,
            assn_fixed_meta.get_adjusted_item_count(),
        );

        Ok(())
    }

    /// Determine whether the duration is marked as "confirmed" (not confirmed means estimated).
    ///
    /// Corresponds to Java: `private boolean getDurationEstimated(int type)`
    fn get_duration_estimated(type_: i32) -> bool {
        (type_ & DURATION_CONFIRMED_MASK) != 0
    }

    /// Extract the view data from the MPP file.
    ///
    /// Corresponds to Java: `private void processViewData() throws IOException`
    fn process_view_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CV_iew")) { … }
        if self.view_dir.get_entry(VIEW_DIRECTORY_VIEWS).is_none() {
            return Ok(());
        }

        // Java: DirectoryEntry dir = (DirectoryEntry) m_viewDir.getEntry("CV_iew");
        let dir = self
            .view_dir
            .get_directory(VIEW_DIRECTORY_VIEWS)
            .ok_or_else(|| missing_entry(VIEW_DIRECTORY_VIEWS))?;

        // Java: VarMeta viewVarMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) dir.getEntry("VarMeta"))));
        let view_var_meta: Rc<dyn VarMeta> = {
            let data = dir
                .get_document("VarMeta")
                .ok_or_else(|| missing_entry("VarMeta"))?
                .get_data();
            Rc::new(VarMeta9::new(&data)?)
        };

        // Java: Var2Data viewVarData = new Var2Data(m_file, viewVarMeta, new DocumentInputStream(((DocumentEntry) dir.getEntry("Var2Data"))));
        let view_var_data = {
            let data = dir
                .get_document("Var2Data")
                .ok_or_else(|| missing_entry("Var2Data"))?
                .get_data();
            Var2Data::new(&self.file, Rc::clone(&view_var_meta), &data)
        };

        // Java: FixedMeta fixedMeta = new FixedMeta(new DocumentInputStream(((DocumentEntry) dir.getEntry("FixedMeta"))), 10);
        let fixed_meta = {
            let data = dir
                .get_document("FixedMeta")
                .ok_or_else(|| missing_entry("FixedMeta"))?
                .get_data();
            FixedMeta::with_item_size(&data, 10).map_err(MpxjError::from)?
        };

        // Java: FixedData fixedData = new FixedData(122, m_inputStreamFactory.getInstance(dir, "FixedData"));
        let fixed_data = {
            let mut stream = self.input_stream_factory.get_instance(&dir, "FixedData")?;
            let bytes = stream.to_bytes();
            FixedData::with_fixed_size(122, &bytes)
        };

        // Java: int items = fixedMeta.getAdjustedItemCount();
        let items = fixed_meta.get_adjusted_item_count();

        // Java: ViewFactory factory = new ViewFactory9();
        let factory = ViewFactory9::new();

        // Java: int lastOffset = -1;
        let mut last_offset = -1i32;

        for loop_index in 0..items {
            // Java: byte[] fm = fixedMeta.getByteArrayValue(loop);
            let meta = fixed_meta.get_byte_array_value(loop_index).unwrap_or(&[]);

            // Java: int offset = ByteArrayHelper.getShort(fm, 4);
            let offset = get_short_safe(meta, 4);

            if offset > last_offset {
                // Java: byte[] fd = fixedData.getByteArrayValue(fixedData.getIndexFromOffset(offset));
                let data = fixed_data
                    .get_byte_array_value(fixed_data.get_index_from_offset(offset));
                if let Some(data) = data {
                    // Java: view = factory.createView(m_file, fm, fd, viewVarData, m_fontBases);
                    //       m_file.getViews().add(view);
                    let view =
                        factory.create_view(&self.file, meta, data, &view_var_data, &self.font_bases)?;
                    try_file_mut(&self.file)?.get_views_mut().add(view);
                }
                last_offset = offset;
            }
        }

        Ok(())
    }

    /// Extract the table data from the MPP file.
    ///
    /// Corresponds to Java: `private void processTableData() throws IOException`
    ///
    /// TODO (Java comment): this implementation does not handle MPP9 files saved by a later version of MS Project.
    fn process_table_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CTable")) { … }
        if self.view_dir.get_entry(VIEW_DIRECTORY_TABLES).is_none() {
            return Ok(());
        }

        // Java: DirectoryEntry dir = (DirectoryEntry) m_viewDir.getEntry("CTable");
        let dir = self
            .view_dir
            .get_directory(VIEW_DIRECTORY_TABLES)
            .ok_or_else(|| missing_entry(VIEW_DIRECTORY_TABLES))?;

        // Java: InputStream stream = m_inputStreamFactory.getInstance(dir, "FixedData");
        //       int blockSize = stream.available() % 115 == 0 ? 115 : 110;
        //       FixedData fixedData = new FixedData(blockSize, stream);
        let mut stream = self.input_stream_factory.get_instance(&dir, "FixedData")?;
        let block_size = if stream.available() % 115 == 0 { 115 } else { 110 };
        let fixed_data_bytes = stream.to_bytes();
        let fixed_data = FixedData::with_fixed_size(block_size, &fixed_data_bytes);

        // Java: VarMeta varMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) dir.getEntry("VarMeta"))));
        let var_meta: Rc<dyn VarMeta> = {
            let data = dir
                .get_document("VarMeta")
                .ok_or_else(|| missing_entry("VarMeta"))?
                .get_data();
            Rc::new(VarMeta9::new(&data)?)
        };

        // Java: Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(((DocumentEntry) dir.getEntry("Var2Data"))));
        let var_data = {
            let data = dir
                .get_document("Var2Data")
                .ok_or_else(|| missing_entry("Var2Data"))?
                .get_data();
            Var2Data::new(&self.file, Rc::clone(&var_meta), &data)
        };

        // Java: TableFactory factory = new TableFactory(TABLE_COLUMN_DATA_STANDARD, TABLE_COLUMN_DATA_ENTERPRISE, TABLE_COLUMN_DATA_BASELINE);
        let factory = TableFactory::new(
            Some(TABLE_COLUMN_DATA_STANDARD),
            Some(TABLE_COLUMN_DATA_ENTERPRISE),
            TABLE_COLUMN_DATA_BASELINE,
        );

        // Java: int items = fixedData.getItemCount();
        let items = fixed_data.get_item_count();

        for loop_index in 0..items {
            // Java: byte[] data = fixedData.getByteArrayValue(loop);
            let Some(data) = fixed_data.get_byte_array_value(loop_index) else {
                continue;
            };

            // Java: Table table = factory.createTable(m_file, data, varMeta, varData);
            //       container.add(table);
            let table = factory.create_table(&self.file, data, &*var_meta, &var_data);
            try_file_mut(&self.file)?.get_tables_mut().add(table);
        }

        Ok(())
    }

    /// Read the filter definitions.
    ///
    /// Corresponds to Java: `private void processFilterData() throws IOException`
    ///
    /// TODO (Java comment): does not work correctly on MPP9 files saved by Project 2007 / 2010.
    fn process_filter_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CFilter")) { … }
        if self.view_dir.get_entry(VIEW_DIRECTORY_FILTERS).is_none() {
            return Ok(());
        }

        // Java: DirectoryEntry dir = (DirectoryEntry) m_viewDir.getEntry("CFilter");
        let dir = self
            .view_dir
            .get_directory(VIEW_DIRECTORY_FILTERS)
            .ok_or_else(|| missing_entry(VIEW_DIRECTORY_FILTERS))?;

        // Java: InputStream stream = m_inputStreamFactory.getInstance(dir, "FixedData");
        //       int blockSize = stream.available() % 115 == 0 ? 115 : 110;
        //       FixedData fixedData = new FixedData(blockSize, stream, true);
        let mut stream = self.input_stream_factory.get_instance(&dir, "FixedData")?;
        let block_size = if stream.available() % 115 == 0 { 115 } else { 110 };
        let fixed_data_bytes = stream.to_bytes();
        let fixed_data = FixedData::with_fixed_size_and_remainder(block_size, &fixed_data_bytes, true);

        // Java: VarMeta varMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) dir.getEntry("VarMeta"))));
        let var_meta: Rc<dyn VarMeta> = {
            let data = dir
                .get_document("VarMeta")
                .ok_or_else(|| missing_entry("VarMeta"))?
                .get_data();
            Rc::new(VarMeta9::new(&data)?)
        };

        // Java: Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(((DocumentEntry) dir.getEntry("Var2Data"))));
        let var_data = {
            let data = dir
                .get_document("Var2Data")
                .ok_or_else(|| missing_entry("Var2Data"))?
                .get_data();
            Var2Data::new(&self.file, var_meta, &data)
        };

        // Java: FilterReader reader = new FilterReader9();
        //       reader.process(m_file, fixedData, varData);
        FilterReader9::new().process(&self.file, &fixed_data, &var_data);

        Ok(())
    }

    /// Read the group definitions.
    ///
    /// Corresponds to Java: `private void processGroupData() throws IOException`
    ///
    /// TODO (Java comment): does not work correctly on MPP9 files saved by Project 2007 / 2010.
    fn process_group_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CGrouping")) { … }
        if self.view_dir.get_entry(VIEW_DIRECTORY_GROUPS).is_none() {
            return Ok(());
        }

        // Java: DirectoryEntry dir = (DirectoryEntry) m_viewDir.getEntry("CGrouping");
        let dir = self
            .view_dir
            .get_directory(VIEW_DIRECTORY_GROUPS)
            .ok_or_else(|| missing_entry(VIEW_DIRECTORY_GROUPS))?;

        // Java: FixedData fixedData = new FixedData(110, m_inputStreamFactory.getInstance(dir, "FixedData"));
        let fixed_data = {
            let mut stream = self.input_stream_factory.get_instance(&dir, "FixedData")?;
            let bytes = stream.to_bytes();
            FixedData::with_fixed_size(110, &bytes)
        };

        // Java: VarMeta varMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) dir.getEntry("VarMeta"))));
        let var_meta: Rc<dyn VarMeta> = {
            let data = dir
                .get_document("VarMeta")
                .ok_or_else(|| missing_entry("VarMeta"))?
                .get_data();
            Rc::new(VarMeta9::new(&data)?)
        };

        // Java: Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(((DocumentEntry) dir.getEntry("Var2Data"))));
        let var_data = {
            let data = dir
                .get_document("Var2Data")
                .ok_or_else(|| missing_entry("Var2Data"))?
                .get_data();
            Var2Data::new(&self.file, var_meta, &data)
        };

        // Java: GroupReader reader = new GroupReader9();
        //       reader.process(m_file, fixedData, varData, m_fontBases);
        GroupReader9::new().process(&self.file, &fixed_data, &var_data, &self.font_bases);

        Ok(())
    }

    /// Read the view state saved in the MPP file.
    ///
    /// Corresponds to Java: `private void processSavedViewState() throws IOException`
    fn process_saved_view_state(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CEdl")) { … }
        if self
            .view_dir
            .get_entry(VIEW_DIRECTORY_SAVED_VIEW_STATE)
            .is_none()
        {
            return Ok(());
        }

        // Java: DirectoryEntry dir = (DirectoryEntry) m_viewDir.getEntry("CEdl");
        let dir = self
            .view_dir
            .get_directory(VIEW_DIRECTORY_SAVED_VIEW_STATE)
            .ok_or_else(|| missing_entry(VIEW_DIRECTORY_SAVED_VIEW_STATE))?;

        // Java: VarMeta varMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) dir.getEntry("VarMeta"))));
        let var_meta: Rc<dyn VarMeta> = {
            let data = dir
                .get_document("VarMeta")
                .ok_or_else(|| missing_entry("VarMeta"))?
                .get_data();
            Rc::new(VarMeta9::new(&data)?)
        };

        // Java: Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(((DocumentEntry) dir.getEntry("Var2Data"))));
        let var_data = {
            let data = dir
                .get_document("Var2Data")
                .ok_or_else(|| missing_entry("Var2Data"))?
                .get_data();
            Var2Data::new(&self.file, var_meta, &data)
        };

        // Java: InputStream is = m_inputStreamFactory.getInstance(dir, "FixedData");
        //       byte[] fixedData = InputStreamHelper.readAvailable(is);
        let fixed_data = {
            let mut stream = self.input_stream_factory.get_instance(&dir, "FixedData")?;
            stream.to_bytes()
        };

        // Java: ViewStateReader reader = new ViewStateReader9();
        //       reader.process(m_file, varData, fixedData);
        ViewStateReader9::new().process(&self.file, &var_data, &fixed_data)
    }

    /// Read the data link (DataLink) definitions.
    ///
    /// Corresponds to Java: `private void processDataLinks() throws IOException`
    fn process_data_links(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CEdl")) { … }
        if self
            .view_dir
            .get_entry(VIEW_DIRECTORY_SAVED_VIEW_STATE)
            .is_none()
        {
            return Ok(());
        }

        // Java: DirectoryEntry dir = (DirectoryEntry) m_viewDir.getEntry("CEdl");
        let dir = self
            .view_dir
            .get_directory(VIEW_DIRECTORY_SAVED_VIEW_STATE)
            .ok_or_else(|| missing_entry(VIEW_DIRECTORY_SAVED_VIEW_STATE))?;

        // Java: FixedMeta fixedMeta = new FixedMeta(new DocumentInputStream(((DocumentEntry) dir.getEntry("FixedMeta"))), 10);
        let fixed_meta = {
            let data = dir
                .get_document("FixedMeta")
                .ok_or_else(|| missing_entry("FixedMeta"))?
                .get_data();
            FixedMeta::with_item_size(&data, 10).map_err(MpxjError::from)?
        };

        // Java: FixedData fixedData = new FixedData(fixedMeta, m_inputStreamFactory.getInstance(dir, "FixedData"));
        let fixed_data = {
            let mut stream = self.input_stream_factory.get_instance(&dir, "FixedData")?;
            let bytes = stream.to_bytes();
            FixedData::new(&fixed_meta, &bytes)
        };

        // Java: VarMeta varMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) dir.getEntry("VarMeta"))));
        let var_meta: Rc<dyn VarMeta> = {
            let data = dir
                .get_document("VarMeta")
                .ok_or_else(|| missing_entry("VarMeta"))?
                .get_data();
            Rc::new(VarMeta9::new(&data)?)
        };

        // Java: Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(((DocumentEntry) dir.getEntry("Var2Data"))));
        let var_data = {
            let data = dir
                .get_document("Var2Data")
                .ok_or_else(|| missing_entry("Var2Data"))?
                .get_data();
            Var2Data::new(&self.file, var_meta, &data)
        };

        // Java: DataLinkFactory factory = new DataLinkFactory(m_file, fixedData, varData);
        //       factory.process();
        let factory = DataLinkFactory::new(&self.file, fixed_data, var_data);
        factory.process()
    }

    /// After all other checks pass, make a best effort to catch one more invalid task: validate by task ID.
    ///
    /// Corresponds to Java: `private void postProcessTasks() throws MPXJException`
    fn post_process_tasks(&mut self) -> MpxjResult<()> {
        //
        // Renumber ID values using a large increment to allow
        // space for later inserts.
        //
        // Java: TreeMap<Integer, Integer> taskMap = new TreeMap<>();
        let mut task_map: BTreeMap<i32, i32> = BTreeMap::new();

        // Java: int nextIDIncrement = ((m_nullTaskOrder.size() / 1000) + 1) * 2000;
        let next_id_increment = ((self.null_task_order.len() as i32 / 1000) + 1) * 2000;

        // Java: int nextID = (m_file.getTaskByUniqueID(Integer.valueOf(0)) == null ? nextIDIncrement : 0);
        let has_unique_id_zero = try_file(&self.file)?
            .get_task_by_unique_id(Some(0))
            .is_some();
        let mut next_id = if has_unique_id_zero { 0 } else { next_id_increment };

        // Java: for (Map.Entry<Integer, Integer> entry : m_taskOrder.entrySet()) { taskMap.put(Integer.valueOf(nextID), entry.getValue()); nextID += nextIDIncrement; }
        for value in self.task_order.values() {
            task_map.insert(next_id, *value);
            next_id += next_id_increment;
        }

        //
        // Insert any null tasks into the correct location
        //
        // Java: int insertionCount = 0; Map<Integer, Integer> offsetMap = new HashMap<>();
        let mut insertion_count = 0i32;
        let mut offset_map: HashMap<i32, i32> = HashMap::new();

        for (id_value, unique_id) in &self.null_task_order {
            // Java: int idValue = entry.getKey().intValue();
            let id_value = *id_value;

            // Java: int baseTargetIdValue = (idValue - insertionCount) * nextIDIncrement;
            let base_target_id_value = id_value
                .wrapping_sub(insertion_count)
                .wrapping_mul(next_id_increment);
            let mut target_id_value = base_target_id_value;

            // Java: Integer previousOffsetKey = Integer.valueOf(baseTargetIdValue);
            //       Integer previousOffset = offsetMap.get(previousOffsetKey);
            //       int offset = previousOffset == null ? 0 : previousOffset.intValue() + 1;
            let previous_offset_key = base_target_id_value;
            let previous_offset = offset_map.get(&previous_offset_key).copied();
            let mut offset = previous_offset.map_or(0, |value| value + 1);
            insertion_count += 1;

            // Java: while (taskMap.containsKey(Integer.valueOf(targetIDValue))) { … }
            while task_map.contains_key(&target_id_value) {
                // Java: ++offset;
                offset += 1;

                // Java: if (offset == nextIDIncrement) { throw new MPXJException("Unable to fix task order"); }
                if offset == next_id_increment {
                    return Err(MpxjError::new("Unable to fix task order"));
                }

                // Java: targetIDValue = baseTargetIdValue - (nextIDIncrement - offset);
                target_id_value = base_target_id_value
                    .wrapping_sub(next_id_increment.wrapping_sub(offset));
            }

            // Java: offsetMap.put(previousOffsetKey, Integer.valueOf(offset));
            //       taskMap.put(Integer.valueOf(targetIDValue), entry.getValue());
            offset_map.insert(previous_offset_key, offset);
            task_map.insert(target_id_value, *unique_id);
        }

        //
        // Finally, we can renumber the tasks
        //
        // Java: nextID = (m_file.getTaskByUniqueID(Integer.valueOf(0)) == null ? 1 : 0);
        let mut next_id = if try_file(&self.file)?
            .get_task_by_unique_id(Some(0))
            .is_none()
        {
            1
        } else {
            0
        };

        // Java: for (Map.Entry<Integer, Integer> entry : taskMap.entrySet()) { Task task = m_file.getTaskByUniqueID(entry.getValue()); if (task != null) { task.setID(Integer.valueOf(nextID)); } nextID++; }
        for unique_id in task_map.values() {
            let task = try_file(&self.file)?.get_task_by_unique_id(Some(*unique_id));
            if let Some(task) = task {
                task.borrow_mut().set_id(Some(next_id));
            }
            next_id += 1;
        }

        Ok(())
    }

    /// Get the task outline code value.
    ///
    /// Corresponds to Java: `m_outlineCodeVarData.getUnicodeString((Integer) task.getCachedValue(TaskField.OUTLINE_CODEn_INDEX), OUTLINECODE_DATA)`
    fn task_outline_code(&self, task: &Rc<RefCell<Task>>, field: TaskField) -> Option<String> {
        // In Java a `null` ID throws an NPE via `TreeMap.get(null)`; the Rust side returns None.
        let id = task
            .borrow()
            .get_cached_value(FieldKey::Task(field))
            .and_then(FieldValue::as_integer);
        self.outline_code_var_data
            .get_unicode_string(id, Some(OUTLINE_CODE_DATA))
    }

    /// Get the resource outline code value.
    ///
    /// Corresponds to Java: `m_outlineCodeVarData.getUnicodeString((Integer) resource.getCachedValue(ResourceField.OUTLINE_CODEn_INDEX), OUTLINECODE_DATA)`
    fn resource_outline_code(
        &self,
        resource: &Rc<RefCell<Resource>>,
        field: ResourceField,
    ) -> Option<String> {
        let id = resource
            .borrow()
            .get_cached_value(FieldKey::Resource(field))
            .and_then(FieldValue::as_integer);
        self.outline_code_var_data
            .get_unicode_string(id, Some(OUTLINE_CODE_DATA))
    }
}

// ---------------------------------------------------------------------------
// Private helper functions
// ---------------------------------------------------------------------------

/// Wrap a built-in task field as a [`MppFieldType`] (Rust-specific: Java passes the `TaskField` constant directly).
fn task_field(field: TaskField) -> MppFieldType {
    MppFieldType::Builtin(FieldKey::Task(field))
}

/// Wrap a built-in resource field as a [`MppFieldType`] (Rust-specific).
fn resource_field(field: ResourceField) -> MppFieldType {
    MppFieldType::Builtin(FieldKey::Resource(field))
}

/// Take a read-only borrow of `ProjectFile`.
///
/// Corresponds to Java: use the `m_file` reference directly (the Rust side must `try_borrow` and returns `Err` on a borrow conflict).
fn try_file(file: &Rc<RefCell<ProjectFile>>) -> MpxjResult<Ref<'_, ProjectFile>> {
    file.try_borrow().map_err(|_| borrow_error("ProjectFile"))
}

/// Take a mutable borrow of `ProjectFile`.
///
/// Corresponds to Java: use the `m_file` reference directly (the Rust side must `try_borrow_mut` and returns `Err` on a borrow conflict).
fn try_file_mut(file: &Rc<RefCell<ProjectFile>>) -> MpxjResult<RefMut<'_, ProjectFile>> {
    file.try_borrow_mut()
        .map_err(|_| borrow_error("ProjectFile"))
}

/// Build a "borrow conflict" error.
fn borrow_error(target: &str) -> MpxjError {
    MpxjError::new(format!("Unable to borrow {target}"))
}

/// Build an "entry not found" error.
///
/// Corresponds to Java: POI's `DirectoryEntry.getEntry` throws `FileNotFoundException`.
fn missing_entry(name: &str) -> MpxjError {
    MpxjError::new(format!("Entry not found: {name}"))
}

/// Build an "array out of bounds" error.
///
/// Corresponds to Java's `ArrayIndexOutOfBoundsException` (`readSubProject` catches it and records an ignored error).
fn out_of_bounds() -> MpxjError {
    MpxjError::new("Array index out of bounds")
}

/// Read `metaData[index]`; return 0 when out of bounds.
///
/// Corresponds to Java: `metaData[11]` (throws `ArrayIndexOutOfBoundsException` when out of bounds).
fn meta_byte(meta_data: &[u8], index: usize) -> u8 {
    meta_data.get(index).copied().unwrap_or(0)
}

/// Read two bytes at a Java `int` offset; return 0 when the offset is negative or out of bounds.
///
/// Corresponds to Java: `ByteArrayHelper.getShort(data, offset)` (throws when out of bounds / on a negative offset).
fn get_short_by_offset(data: &[u8], offset: i32) -> i32 {
    match usize::try_from(offset) {
        Ok(offset) => get_short_safe(data, offset),
        Err(_) => 0,
    }
}

/// Read four bytes at a Java `int` offset; return 0 when the offset is negative or out of bounds.
///
/// Corresponds to Java: `ByteArrayHelper.getInt(data, offset)` (throws when out of bounds / on a negative offset).
fn get_int_by_offset(data: &[u8], offset: i32) -> i32 {
    match usize::try_from(offset) {
        Ok(offset) => get_int_safe(data, offset),
        Err(_) => 0,
    }
}

/// Read four bytes; return `Err` when the offset is negative or out of bounds (used for `readSubProject`'s "give up on out of bounds" semantics).
fn get_int_checked(data: &[u8], offset: i32) -> MpxjResult<i32> {
    let offset = usize::try_from(offset).map_err(|_| out_of_bounds())?;
    if offset.checked_add(4).map_or(true, |end| end > data.len()) {
        return Err(out_of_bounds());
    }

    Ok(get_int_safe(data, offset))
}

/// Convert a negative offset to `usize`; a negative value returns `Err` (corresponding to Java throwing).
fn checked_offset(offset: i32) -> MpxjResult<usize> {
    usize::try_from(offset).map_err(|_| out_of_bounds())
}

/// `offset + add`; return `Err` when the result overflows `i32` or is negative.
fn checked_offset_add(offset: i32, add: i32) -> MpxjResult<i32> {
    offset.checked_add(add).ok_or_else(out_of_bounds)
}

/// Fire the "task read complete" event.
///
/// Corresponds to Java: `m_eventManager.fireTaskReadEvent(task)`
///
/// Rust-specific: `ProjectFile::get_event_manager` returns only a read-only `Ref`, so it is obtained through `ProjectContext`
/// as a mutable reference (Java holds a stable reference, see the file header).
fn fire_task_read_event(file: &Rc<RefCell<ProjectFile>>, task: &Rc<RefCell<Task>>) -> MpxjResult<()> {
    let context = try_file(file)?.get_project_context();
    context
        .try_borrow_mut()
        .map_err(|_| borrow_error("ProjectContext"))?
        .get_event_manager_mut()
        .fire_task_read_event(task);
    Ok(())
}

/// Fire the "resource read complete" event.
///
/// Corresponds to Java: `m_eventManager.fireResourceReadEvent(resource)`
fn fire_resource_read_event(
    file: &Rc<RefCell<ProjectFile>>,
    resource: &Rc<RefCell<Resource>>,
) -> MpxjResult<()> {
    let context = try_file(file)?.get_project_context();
    context
        .try_borrow_mut()
        .map_err(|_| borrow_error("ProjectContext"))?
        .get_event_manager_mut()
        .fire_resource_read_event(resource);
    Ok(())
}

/// Resource field name alias table.
///
/// Corresponds to Java: `private static final Map<Integer, FieldType> RESOURCE_FIELD_ALIASES`
fn resource_field_aliases() -> HashMap<i32, FieldKey> {
    use ResourceField as F;

    let mut map = HashMap::new();

    // TEXT1..TEXT30
    for index in 0..30usize {
        map.insert(52 + index as i32, FieldKey::Resource(resource_text_field(index)));
    }

    // START1..START10
    for index in 0..10usize {
        map.insert(82 + index as i32, FieldKey::Resource(resource_start_field(index)));
    }

    // FINISH1..FINISH10
    for index in 0..10usize {
        map.insert(92 + index as i32, FieldKey::Resource(resource_finish_field(index)));
    }

    // NUMBER1..NUMBER20
    for index in 0..20usize {
        map.insert(102 + index as i32, FieldKey::Resource(resource_number_field(index)));
    }

    // DURATION1..DURATION10
    for index in 0..10usize {
        map.insert(
            122 + index as i32,
            FieldKey::Resource(resource_duration_field(index)),
        );
    }

    // DATE1..DATE10
    for index in 0..10usize {
        map.insert(145 + index as i32, FieldKey::Resource(resource_date_field(index)));
    }

    // OUTLINE_CODE1..OUTLINE_CODE10
    for index in 0..10usize {
        map.insert(
            155 + index as i32,
            FieldKey::Resource(resource_outline_code_field(index)),
        );
    }

    // FLAG10, FLAG1..FLAG9, FLAG11..FLAG20 (this is the Java key order)
    map.insert(165, FieldKey::Resource(F::Flag10));
    for index in 0..9usize {
        map.insert(166 + index as i32, FieldKey::Resource(resource_flag_field(index)));
    }
    for index in 10..20usize {
        map.insert(
            165 + index as i32,
            FieldKey::Resource(resource_flag_field(index)),
        );
    }

    // COST1..COST10
    for index in 0..10usize {
        map.insert(207 + index as i32, FieldKey::Resource(resource_cost_field(index)));
    }

    map
}

/// Task field name alias table.
///
/// Corresponds to Java: `private static final Map<Integer, FieldType> TASK_FIELD_ALIASES`
fn task_field_aliases() -> HashMap<i32, FieldKey> {
    let mut map = HashMap::new();

    // TEXT1..TEXT10
    for index in 0..10usize {
        map.insert(118 + index as i32, FieldKey::Task(task_text_field(index)));
    }

    // START1..START10 / FINISH1..FINISH10 (appearing in pairs)
    for index in 0..10usize {
        map.insert(
            128 + (index as i32) * 2,
            FieldKey::Task(task_start_field(index)),
        );
        map.insert(
            129 + (index as i32) * 2,
            FieldKey::Task(task_finish_field(index)),
        );
    }

    // NUMBER1..NUMBER10
    for index in 0..10usize {
        map.insert(149 + index as i32, FieldKey::Task(task_number_field(index)));
    }

    // DURATION1..DURATION10 (spaced one entry apart)
    for index in 0..10usize {
        map.insert(
            159 + (index as i32) * 2,
            FieldKey::Task(task_duration_field(index)),
        );
    }

    // DATE1..DATE10
    for index in 0..10usize {
        map.insert(184 + index as i32, FieldKey::Task(task_date_field(index)));
    }

    // TEXT11..TEXT30
    for index in 0..20usize {
        map.insert(
            194 + index as i32,
            FieldKey::Task(task_text_field(index + 10)),
        );
    }

    // NUMBER11..NUMBER20
    for index in 0..10usize {
        map.insert(
            214 + index as i32,
            FieldKey::Task(task_number_field(index + 10)),
        );
    }

    // OUTLINE_CODE1..OUTLINE_CODE10
    for index in 0..10usize {
        map.insert(
            227 + index as i32,
            FieldKey::Task(task_outline_code_field(index)),
        );
    }

    // FLAG1..FLAG20
    for index in 0..20usize {
        map.insert(237 + index as i32, FieldKey::Task(task_flag_field(index)));
    }

    // COST1..COST10
    for index in 0..10usize {
        map.insert(278 + index as i32, FieldKey::Task(task_cost_field(index)));
    }

    map
}

/// Get the `index`-th (zero-based) text field of the task.
fn task_text_field(index: usize) -> TaskField {
    const FIELDS: [TaskField; 30] = [
        TaskField::Text1,
        TaskField::Text2,
        TaskField::Text3,
        TaskField::Text4,
        TaskField::Text5,
        TaskField::Text6,
        TaskField::Text7,
        TaskField::Text8,
        TaskField::Text9,
        TaskField::Text10,
        TaskField::Text11,
        TaskField::Text12,
        TaskField::Text13,
        TaskField::Text14,
        TaskField::Text15,
        TaskField::Text16,
        TaskField::Text17,
        TaskField::Text18,
        TaskField::Text19,
        TaskField::Text20,
        TaskField::Text21,
        TaskField::Text22,
        TaskField::Text23,
        TaskField::Text24,
        TaskField::Text25,
        TaskField::Text26,
        TaskField::Text27,
        TaskField::Text28,
        TaskField::Text29,
        TaskField::Text30,
    ];
    FIELDS.get(index).copied().unwrap_or(TaskField::Text1)
}

/// Get the `index`-th start date field of the task.
fn task_start_field(index: usize) -> TaskField {
    const FIELDS: [TaskField; 10] = [
        TaskField::Start1,
        TaskField::Start2,
        TaskField::Start3,
        TaskField::Start4,
        TaskField::Start5,
        TaskField::Start6,
        TaskField::Start7,
        TaskField::Start8,
        TaskField::Start9,
        TaskField::Start10,
    ];
    FIELDS.get(index).copied().unwrap_or(TaskField::Start1)
}

/// Get the `index`-th finish date field of the task.
fn task_finish_field(index: usize) -> TaskField {
    const FIELDS: [TaskField; 10] = [
        TaskField::Finish1,
        TaskField::Finish2,
        TaskField::Finish3,
        TaskField::Finish4,
        TaskField::Finish5,
        TaskField::Finish6,
        TaskField::Finish7,
        TaskField::Finish8,
        TaskField::Finish9,
        TaskField::Finish10,
    ];
    FIELDS.get(index).copied().unwrap_or(TaskField::Finish1)
}

/// Get the `index`-th number field of the task.
fn task_number_field(index: usize) -> TaskField {
    const FIELDS: [TaskField; 20] = [
        TaskField::Number1,
        TaskField::Number2,
        TaskField::Number3,
        TaskField::Number4,
        TaskField::Number5,
        TaskField::Number6,
        TaskField::Number7,
        TaskField::Number8,
        TaskField::Number9,
        TaskField::Number10,
        TaskField::Number11,
        TaskField::Number12,
        TaskField::Number13,
        TaskField::Number14,
        TaskField::Number15,
        TaskField::Number16,
        TaskField::Number17,
        TaskField::Number18,
        TaskField::Number19,
        TaskField::Number20,
    ];
    FIELDS.get(index).copied().unwrap_or(TaskField::Number1)
}

/// Get the `index`-th duration field of the task.
fn task_duration_field(index: usize) -> TaskField {
    const FIELDS: [TaskField; 10] = [
        TaskField::Duration1,
        TaskField::Duration2,
        TaskField::Duration3,
        TaskField::Duration4,
        TaskField::Duration5,
        TaskField::Duration6,
        TaskField::Duration7,
        TaskField::Duration8,
        TaskField::Duration9,
        TaskField::Duration10,
    ];
    FIELDS.get(index).copied().unwrap_or(TaskField::Duration1)
}

/// Get the `index`-th date field of the task.
fn task_date_field(index: usize) -> TaskField {
    const FIELDS: [TaskField; 10] = [
        TaskField::Date1,
        TaskField::Date2,
        TaskField::Date3,
        TaskField::Date4,
        TaskField::Date5,
        TaskField::Date6,
        TaskField::Date7,
        TaskField::Date8,
        TaskField::Date9,
        TaskField::Date10,
    ];
    FIELDS.get(index).copied().unwrap_or(TaskField::Date1)
}

/// Get the `index`-th outline code field of the task.
fn task_outline_code_field(index: usize) -> TaskField {
    const FIELDS: [TaskField; 10] = [
        TaskField::OutlineCode1,
        TaskField::OutlineCode2,
        TaskField::OutlineCode3,
        TaskField::OutlineCode4,
        TaskField::OutlineCode5,
        TaskField::OutlineCode6,
        TaskField::OutlineCode7,
        TaskField::OutlineCode8,
        TaskField::OutlineCode9,
        TaskField::OutlineCode10,
    ];
    FIELDS.get(index).copied().unwrap_or(TaskField::OutlineCode1)
}

/// Get the `index`-th flag field of the task.
fn task_flag_field(index: usize) -> TaskField {
    const FIELDS: [TaskField; 20] = [
        TaskField::Flag1,
        TaskField::Flag2,
        TaskField::Flag3,
        TaskField::Flag4,
        TaskField::Flag5,
        TaskField::Flag6,
        TaskField::Flag7,
        TaskField::Flag8,
        TaskField::Flag9,
        TaskField::Flag10,
        TaskField::Flag11,
        TaskField::Flag12,
        TaskField::Flag13,
        TaskField::Flag14,
        TaskField::Flag15,
        TaskField::Flag16,
        TaskField::Flag17,
        TaskField::Flag18,
        TaskField::Flag19,
        TaskField::Flag20,
    ];
    FIELDS.get(index).copied().unwrap_or(TaskField::Flag1)
}

/// Get the `index`-th cost field of the task.
fn task_cost_field(index: usize) -> TaskField {
    const FIELDS: [TaskField; 10] = [
        TaskField::Cost1,
        TaskField::Cost2,
        TaskField::Cost3,
        TaskField::Cost4,
        TaskField::Cost5,
        TaskField::Cost6,
        TaskField::Cost7,
        TaskField::Cost8,
        TaskField::Cost9,
        TaskField::Cost10,
    ];
    FIELDS.get(index).copied().unwrap_or(TaskField::Cost1)
}

/// Get the `index`-th text field of the resource.
fn resource_text_field(index: usize) -> ResourceField {
    const FIELDS: [ResourceField; 30] = [
        ResourceField::Text1,
        ResourceField::Text2,
        ResourceField::Text3,
        ResourceField::Text4,
        ResourceField::Text5,
        ResourceField::Text6,
        ResourceField::Text7,
        ResourceField::Text8,
        ResourceField::Text9,
        ResourceField::Text10,
        ResourceField::Text11,
        ResourceField::Text12,
        ResourceField::Text13,
        ResourceField::Text14,
        ResourceField::Text15,
        ResourceField::Text16,
        ResourceField::Text17,
        ResourceField::Text18,
        ResourceField::Text19,
        ResourceField::Text20,
        ResourceField::Text21,
        ResourceField::Text22,
        ResourceField::Text23,
        ResourceField::Text24,
        ResourceField::Text25,
        ResourceField::Text26,
        ResourceField::Text27,
        ResourceField::Text28,
        ResourceField::Text29,
        ResourceField::Text30,
    ];
    FIELDS.get(index).copied().unwrap_or(ResourceField::Text1)
}

/// Get the `index`-th start date field of the resource.
fn resource_start_field(index: usize) -> ResourceField {
    const FIELDS: [ResourceField; 10] = [
        ResourceField::Start1,
        ResourceField::Start2,
        ResourceField::Start3,
        ResourceField::Start4,
        ResourceField::Start5,
        ResourceField::Start6,
        ResourceField::Start7,
        ResourceField::Start8,
        ResourceField::Start9,
        ResourceField::Start10,
    ];
    FIELDS.get(index).copied().unwrap_or(ResourceField::Start1)
}

/// Get the `index`-th finish date field of the resource.
fn resource_finish_field(index: usize) -> ResourceField {
    const FIELDS: [ResourceField; 10] = [
        ResourceField::Finish1,
        ResourceField::Finish2,
        ResourceField::Finish3,
        ResourceField::Finish4,
        ResourceField::Finish5,
        ResourceField::Finish6,
        ResourceField::Finish7,
        ResourceField::Finish8,
        ResourceField::Finish9,
        ResourceField::Finish10,
    ];
    FIELDS.get(index).copied().unwrap_or(ResourceField::Finish1)
}

/// Get the `index`-th number field of the resource.
fn resource_number_field(index: usize) -> ResourceField {
    const FIELDS: [ResourceField; 20] = [
        ResourceField::Number1,
        ResourceField::Number2,
        ResourceField::Number3,
        ResourceField::Number4,
        ResourceField::Number5,
        ResourceField::Number6,
        ResourceField::Number7,
        ResourceField::Number8,
        ResourceField::Number9,
        ResourceField::Number10,
        ResourceField::Number11,
        ResourceField::Number12,
        ResourceField::Number13,
        ResourceField::Number14,
        ResourceField::Number15,
        ResourceField::Number16,
        ResourceField::Number17,
        ResourceField::Number18,
        ResourceField::Number19,
        ResourceField::Number20,
    ];
    FIELDS.get(index).copied().unwrap_or(ResourceField::Number1)
}

/// Get the `index`-th duration field of the resource.
fn resource_duration_field(index: usize) -> ResourceField {
    const FIELDS: [ResourceField; 10] = [
        ResourceField::Duration1,
        ResourceField::Duration2,
        ResourceField::Duration3,
        ResourceField::Duration4,
        ResourceField::Duration5,
        ResourceField::Duration6,
        ResourceField::Duration7,
        ResourceField::Duration8,
        ResourceField::Duration9,
        ResourceField::Duration10,
    ];
    FIELDS.get(index).copied().unwrap_or(ResourceField::Duration1)
}

/// Get the `index`-th date field of the resource.
fn resource_date_field(index: usize) -> ResourceField {
    const FIELDS: [ResourceField; 10] = [
        ResourceField::Date1,
        ResourceField::Date2,
        ResourceField::Date3,
        ResourceField::Date4,
        ResourceField::Date5,
        ResourceField::Date6,
        ResourceField::Date7,
        ResourceField::Date8,
        ResourceField::Date9,
        ResourceField::Date10,
    ];
    FIELDS.get(index).copied().unwrap_or(ResourceField::Date1)
}

/// Get the `index`-th outline code field of the resource.
fn resource_outline_code_field(index: usize) -> ResourceField {
    const FIELDS: [ResourceField; 10] = [
        ResourceField::OutlineCode1,
        ResourceField::OutlineCode2,
        ResourceField::OutlineCode3,
        ResourceField::OutlineCode4,
        ResourceField::OutlineCode5,
        ResourceField::OutlineCode6,
        ResourceField::OutlineCode7,
        ResourceField::OutlineCode8,
        ResourceField::OutlineCode9,
        ResourceField::OutlineCode10,
    ];
    FIELDS.get(index).copied().unwrap_or(ResourceField::OutlineCode1)
}

/// Get the `index`-th flag field of the resource.
fn resource_flag_field(index: usize) -> ResourceField {
    const FIELDS: [ResourceField; 20] = [
        ResourceField::Flag1,
        ResourceField::Flag2,
        ResourceField::Flag3,
        ResourceField::Flag4,
        ResourceField::Flag5,
        ResourceField::Flag6,
        ResourceField::Flag7,
        ResourceField::Flag8,
        ResourceField::Flag9,
        ResourceField::Flag10,
        ResourceField::Flag11,
        ResourceField::Flag12,
        ResourceField::Flag13,
        ResourceField::Flag14,
        ResourceField::Flag15,
        ResourceField::Flag16,
        ResourceField::Flag17,
        ResourceField::Flag18,
        ResourceField::Flag19,
        ResourceField::Flag20,
    ];
    FIELDS.get(index).copied().unwrap_or(ResourceField::Flag1)
}

/// Get the `index`-th cost field of the resource.
fn resource_cost_field(index: usize) -> ResourceField {
    const FIELDS: [ResourceField; 10] = [
        ResourceField::Cost1,
        ResourceField::Cost2,
        ResourceField::Cost3,
        ResourceField::Cost4,
        ResourceField::Cost5,
        ResourceField::Cost6,
        ResourceField::Cost7,
        ResourceField::Cost8,
        ResourceField::Cost9,
        ResourceField::Cost10,
    ];
    FIELDS.get(index).copied().unwrap_or(ResourceField::Cost1)
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use cfb::CompoundFile;
    use chrono::{NaiveDate, NaiveDateTime};

    use super::*;
    use crate::mpp::ole::PoifsFileSystem;

    /// Magic number of the `FixedMeta` / `VarMeta` block header.
    const MAGIC: u32 = 0xFADF_ADBA;

    /// Length of a task record in the fixed-length data block (for tests).
    const TASK_BLOCK_SIZE: usize = 192;

    /// Length of a resource record in the fixed-length data block (for tests).
    const RESOURCE_BLOCK_SIZE: usize = 192;

    /// Number of entries in the task FixedMeta (the first 3 are skipped by `createTaskMap`, so the tasks are at indexes 3 and 4).
    const TASK_META_ITEMS: usize = 5;

    /// Assemble a `Props9` stream: a 16-byte block header + several entries.
    fn props9(entries: &[(i32, Vec<u8>)]) -> Vec<u8> {
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&(entries.len() as i16).to_le_bytes());

        for (key, value) in entries {
            data.extend_from_slice(&(value.len() as i32).to_le_bytes());
            data.extend_from_slice(&key.to_le_bytes());
            data.extend_from_slice(&0i32.to_le_bytes());
            data.extend_from_slice(value);
            // Java: align to a two-byte boundary
            if value.len() % 2 != 0 {
                data.push(0);
            }
        }

        data
    }

    /// Assemble a `VarMeta9` stream: a 24-byte block header + one "unique ID(3) + type(1) + offset(4)" per entry.
    fn var_meta9(entries: &[(i32, i32, i32)], data_size: i32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&MAGIC.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(entries.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&data_size.to_le_bytes());

        for (unique_id, type_, offset) in entries {
            data.extend_from_slice(&unique_id.to_le_bytes()[..3]);
            data.push(*type_ as u8);
            data.extend_from_slice(&offset.to_le_bytes());
        }

        data
    }

    /// Assemble a `Var2Data` stream, returning (data, per-item offsets). Each item is "4-byte length + data".
    fn var2_data(items: &[Vec<u8>]) -> (Vec<u8>, Vec<i32>) {
        let mut data = Vec::new();
        let mut offsets = Vec::new();

        for item in items {
            offsets.push(data.len() as i32);
            data.extend_from_slice(&(item.len() as i32).to_le_bytes());
            data.extend_from_slice(item);
        }

        (data, offsets)
    }

    /// Assemble a `FixedMeta` stream, returning (data, the start index of each entry).
    ///
    /// Each entry is `item_size` bytes, and bytes 4..8 are the offset of that entry's data in `FixedData`.
    fn fixed_meta(item_size: usize, offsets: &[i32]) -> (Vec<u8>, Vec<usize>) {
        let mut data = Vec::new();
        data.extend_from_slice(&MAGIC.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(offsets.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());

        let mut starts = Vec::new();
        for offset in offsets {
            starts.push(data.len());
            let mut item = vec![0u8; item_size];
            item[4..8].copy_from_slice(&offset.to_le_bytes());
            data.extend_from_slice(&item);
        }

        (data, starts)
    }

    /// NUL-terminated UTF-16LE string.
    fn utf16(text: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in text.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00]);
        data
    }

    /// Write the fixed fields of the task / resource into the data block.
    ///
    /// - offset 0: unique ID (4 bytes)
    /// - offset 4: ID (4 bytes)
    /// - offsets 58/60: duration units and duration (the layout used by `DurationTest9.mpp`)
    /// - offset 88: start time (day count 2 → 1984-01-02)
    /// - offset 160: calendar unique ID
    fn fixed_data_block(unique_id: i32, id: i32, duration: i32, calendar_id: i32) -> Vec<u8> {
        let mut data = vec![0u8; TASK_BLOCK_SIZE];
        data[0..4].copy_from_slice(&unique_id.to_le_bytes());
        data[4..8].copy_from_slice(&id.to_le_bytes());
        // DurationUnits = 7 (days)
        data[58..60].copy_from_slice(&7i16.to_le_bytes());
        data[60..64].copy_from_slice(&duration.to_le_bytes());
        // Start: the low 2 bytes are the time of day (tenths of a minute), the high 2 bytes are the day count
        data[88..92].copy_from_slice(&[0x00, 0x00, 0x02, 0x00]);
        data[160..164].copy_from_slice(&calendar_id.to_le_bytes());
        data
    }

    /// Location where the data block is "truncated / corrupted".
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Broken {
        /// All streams are fine.
        Nothing,
        /// The calendar's `VarMeta` / `FixedMeta` is garbage data.
        Calendar,
        /// The task's `VarMeta` / `FixedMeta` is garbage data.
        Task,
    }

    /// Build an MPP9 compound document for tests.
    ///
    /// Structure (all in the minimal parseable form):
    ///
    /// ```text
    /// /Props9
    /// /   19/Props (containing SUBPROJECT_DATA)
    /// /   19/TBkndOutlCode/{VarMeta,Var2Data}
    /// /   19/TBkndCal/{VarMeta,Var2Data,FixedMeta,FixedData} (1 calendar, unique ID = 1)
    /// /   19/TBkndTask/{VarMeta,Var2Data,FixedMeta,FixedData} (2 tasks)
    /// /   19/TBkndRsc/{VarMeta,Var2Data,FixedMeta,FixedData} (1 resource)
    /// /   19/TBkndAssn/{VarMeta,Var2Data,FixedMeta,FixedData} (no assignments)
    /// /   19/TBkndCons/{FixedMeta,FixedData} (1 relation)
    /// /   29
    /// ```
    fn build_mpp9(broken: Broken) -> Vec<u8> {
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).expect("create compound document");

        // Root: Props9 (an empty table suffices — password flag 0, no file path)
        file.create_stream("/Props9")
            .expect("Props9")
            .write_all(&props9(&[]))
            .expect("write Props9");

        // Project storage
        file.create_storage("/   19").expect("   19");
        file.create_stream("/   19/Props")
            .expect("Props")
            .write_all(&project_props())
            .expect("write Props");

        // Outline codes: just having VarMeta / Var2Data is enough (no FixedMeta → no value list)
        file.create_storage("/   19/TBkndOutlCode").expect("TBkndOutlCode");
        file.create_stream("/   19/TBkndOutlCode/VarMeta")
            .expect("outl VarMeta")
            .write_all(&var_meta9(&[], 0))
            .expect("write outl VarMeta");
        file.create_stream("/   19/TBkndOutlCode/Var2Data")
            .expect("outl Var2Data")
            .write_all(&[])
            .expect("write outl Var2Data");

        // Calendar
        file.create_storage("/   19/TBkndCal").expect("TBkndCal");
        let cal_var_meta = match broken {
            Broken::Calendar => vec![0x00, 0x01, 0x02],
            _ => var_meta9(&[], 0),
        };
        file.create_stream("/   19/TBkndCal/VarMeta")
            .expect("cal VarMeta")
            .write_all(&cal_var_meta)
            .expect("write cal VarMeta");
        file.create_stream("/   19/TBkndCal/Var2Data")
            .expect("cal Var2Data")
            .write_all(&[])
            .expect("write cal Var2Data");
        let (cal_fixed_meta, _) = fixed_meta(10, &[0]);
        let cal_fixed_meta = match broken {
            Broken::Calendar => vec![0xEEu8; 20],
            _ => cal_fixed_meta,
        };
        file.create_stream("/   19/TBkndCal/FixedMeta")
            .expect("cal FixedMeta")
            .write_all(&cal_fixed_meta)
            .expect("write cal FixedMeta");
        // 12 bytes: calendarID = 1, baseCalendarID = 0, resourceID = 0
        let mut cal_fixed_data = vec![0u8; 12];
        cal_fixed_data[0..4].copy_from_slice(&1i32.to_le_bytes());
        file.create_stream("/   19/TBkndCal/FixedData")
            .expect("cal FixedData")
            .write_all(&cal_fixed_data)
            .expect("write cal FixedData");

        // Tasks: two tasks (unique IDs 1, 2), plus 2 placeholder entries for alignment
        file.create_storage("/   19/TBkndTask").expect("TBkndTask");
        let (task_name_data, task_name_offsets) = var2_data(&[utf16("Task 1"), utf16("Task 2")]);
        let task_var_meta = match broken {
            Broken::Task => vec![0x00, 0x01, 0x02],
            _ => var_meta9(
                &[
                    (1, 11, task_name_offsets[0]),
                    (2, 11, task_name_offsets[1]),
                ],
                task_name_data.len() as i32,
            ),
        };
        file.create_stream("/   19/TBkndTask/VarMeta")
            .expect("task VarMeta")
            .write_all(&task_var_meta)
            .expect("write task VarMeta");
        file.create_stream("/   19/TBkndTask/Var2Data")
            .expect("task Var2Data")
            .write_all(&task_name_data)
            .expect("write task Var2Data");

        let task_offsets: Vec<i32> = (0..TASK_META_ITEMS)
            .map(|index| match index {
                4 => TASK_BLOCK_SIZE as i32,
                _ => 0,
            })
            .collect();
        let (task_fixed_meta, task_meta_starts) = fixed_meta(47, &task_offsets);
        let task_fixed_meta = match broken {
            Broken::Task => vec![0xEEu8; 20],
            _ => task_fixed_meta,
        };
        file.create_stream("/   19/TBkndTask/FixedMeta")
            .expect("task FixedMeta")
            .write_all(&task_fixed_meta)
            .expect("write task FixedMeta");

        // Task A (index 3): unique ID 1, ID 1, linked calendar 1
        let mut task_data = fixed_data_block(1, 1, 4800, 1);
        // Task B (index 4): unique ID 2, ID 2, no linked calendar
        task_data.extend_from_slice(&fixed_data_block(2, 2, 4800, 0));

        if broken == Broken::Task {
            // Write only half the data block: neither record gets complete data
            task_data.truncate(TASK_BLOCK_SIZE + 4);
        }

        // Only the task record (index 3) needs its flags set; here "task A's metadata" is used to verify the key flags
        let mut task_fixed_meta_bytes = task_fixed_meta.clone();
        if broken != Broken::Task && task_meta_starts.len() == TASK_META_ITEMS {
            let start = task_meta_starts[3];
            task_fixed_meta_bytes[start + 8] = 0x20; // milestone
            task_fixed_meta_bytes[start + 9] = 0x40; // marked
            task_fixed_meta_bytes[start + 10] = 0x80 | 0x08 | 0x02; // hideBar / rollup / ignoreResourceCalendar
            task_fixed_meta_bytes[start + 11] = 0x10; // effortDriven
            task_fixed_meta_bytes[start + 13] = 0x04 | 0x02; // levelAssignments / levelingCanSplit
            task_fixed_meta_bytes[start + 37] = 0x20; // flag1
            task_fixed_meta_bytes[start + 38] = 0x01; // flag4
        }

        // Overwrite (the compound document does not support in-place rewriting, so the stream is rebuilt)
        let mut file = rebuild_stream(
            file,
            "/   19/TBkndTask/FixedMeta",
            &task_fixed_meta_bytes,
        );
        file.create_stream("/   19/TBkndTask/FixedData")
            .expect("task FixedData")
            .write_all(&task_data)
            .expect("write task FixedData");

        // Resources: unique ID 1, ID 7 (resources are always read before tasks, so the resource should still exist when the task data is corrupted)
        file.create_storage("/   19/TBkndRsc").expect("TBkndRsc");
        let (resource_name_data, resource_name_offsets) = var2_data(&[utf16("Resource 1")]);
        let rsc_var_meta = var_meta9(
            &[(1, 1, resource_name_offsets[0])],
            resource_name_data.len() as i32,
        );
        file.create_stream("/   19/TBkndRsc/VarMeta")
            .expect("rsc VarMeta")
            .write_all(&rsc_var_meta)
            .expect("write rsc VarMeta");
        file.create_stream("/   19/TBkndRsc/Var2Data")
            .expect("rsc Var2Data")
            .write_all(&resource_name_data)
            .expect("write rsc Var2Data");

        let (rsc_fixed_meta, rsc_meta_starts) = fixed_meta(37, &[0]);
        file.create_stream("/   19/TBkndRsc/FixedMeta")
            .expect("rsc FixedMeta")
            .write_all(&rsc_fixed_meta)
            .expect("write rsc FixedMeta");

        let mut rsc_fixed_meta_bytes = rsc_fixed_meta.clone();
        if let Some(start) = rsc_meta_starts.first().copied() {
            rsc_fixed_meta_bytes[start + 9] = 0x02; // WORK
            rsc_fixed_meta_bytes[start + 28] = 0x40 | 0x20; // flag1 / flag10
        }
        let mut file = rebuild_stream(file, "/   19/TBkndRsc/FixedMeta", &rsc_fixed_meta_bytes);

        let mut rsc_data = vec![0u8; RESOURCE_BLOCK_SIZE];
        rsc_data[0..4].copy_from_slice(&1i32.to_le_bytes());
        rsc_data[4..8].copy_from_slice(&7i32.to_le_bytes());
        file.create_stream("/   19/TBkndRsc/FixedData")
            .expect("rsc FixedData")
            .write_all(&rsc_data)
            .expect("write rsc FixedData");

        // Assignments: none (entry count 0 → the factory's loop does not execute)
        file.create_storage("/   19/TBkndAssn").expect("TBkndAssn");
        file.create_stream("/   19/TBkndAssn/VarMeta")
            .expect("assn VarMeta")
            .write_all(&var_meta9(&[], 0))
            .expect("write assn VarMeta");
        file.create_stream("/   19/TBkndAssn/Var2Data")
            .expect("assn Var2Data")
            .write_all(&[])
            .expect("write assn Var2Data");
        let (assn_fixed_meta, _) = fixed_meta(34, &[]);
        file.create_stream("/   19/TBkndAssn/FixedMeta")
            .expect("assn FixedMeta")
            .write_all(&assn_fixed_meta)
            .expect("write assn FixedMeta");
        file.create_stream("/   19/TBkndAssn/FixedData")
            .expect("assn FixedData")
            .write_all(&[])
            .expect("write assn FixedData");

        // Relation: task A (unique ID 1) → task B (unique ID 2), FS + 1 day
        file.create_storage("/   19/TBkndCons").expect("TBkndCons");
        let (cons_fixed_meta, _) = fixed_meta(10, &[0]);
        file.create_stream("/   19/TBkndCons/FixedMeta")
            .expect("cons FixedMeta")
            .write_all(&cons_fixed_meta)
            .expect("write cons FixedMeta");

        let mut cons_data = vec![0u8; 20];
        cons_data[0..4].copy_from_slice(&100i32.to_le_bytes()); // relation unique ID
        cons_data[4..8].copy_from_slice(&1i32.to_le_bytes()); // predecessor task
        cons_data[8..12].copy_from_slice(&2i32.to_le_bytes()); // successor task
        cons_data[12..14].copy_from_slice(&1i16.to_le_bytes()); // FS
        cons_data[14..16].copy_from_slice(&7i16.to_le_bytes()); // days
        cons_data[16..20].copy_from_slice(&4800i32.to_le_bytes()); // 1 day
        file.create_stream("/   19/TBkndCons/FixedData")
            .expect("cons FixedData")
            .write_all(&cons_data)
            .expect("write cons FixedData");

        // View storage (empty → all presentation-layer code is skipped)
        file.create_storage("/   29").expect("   29");

        file.into_inner().into_inner()
    }

    /// Replace a stream in the compound document with new content (`cfb` does not support in-place rewriting).
    fn rebuild_stream(
        file: CompoundFile<Cursor<Vec<u8>>>,
        path: &str,
        data: &[u8],
    ) -> CompoundFile<Cursor<Vec<u8>>> {
        let bytes = file.into_inner().into_inner();
        let mut file = CompoundFile::open(Cursor::new(bytes)).expect("reopen compound document");
        file.create_stream(path)
            .expect("create stream")
            .write_all(data)
            .expect("write stream");
        file
    }

    /// Project `Props` stream: subproject data and the conversion properties for an 8-hour working day.
    fn project_props() -> Vec<u8> {
        let mut data = vec![0u8; 160];

        // 4-byte block size + 4 bytes unknown + 4-byte itemCountOffset
        data[8..12].copy_from_slice(&24i32.to_le_bytes());

        // Entry header: itemHeaderOffset / uniqueIDOffset / filePathOffset / fileNameOffset
        data[12..14].copy_from_slice(&32i16.to_le_bytes());
        data[16..18].copy_from_slice(&120i16.to_le_bytes());
        data[20..22].copy_from_slice(&60i16.to_le_bytes());
        data[24..26].copy_from_slice(&60i16.to_le_bytes());

        // Byte 17 of the 20-byte entry header (32..52) is the subproject type (0x01)
        data[48] = 0x01;

        // File path block: 18-byte header + 4 bytes + DOS path + 24 bytes + 4-byte size (0 → use the DOS path)
        let dos_path = b"C:\\sub.mpp\0";
        data[82..82 + dos_path.len()].copy_from_slice(dos_path);

        // Unique ID list: 0 (TASKUNIQUEID0), 42 (external task), 0x0303 (end)
        data[120..124].copy_from_slice(&0i32.to_le_bytes());
        data[124..128].copy_from_slice(&42i32.to_le_bytes());
        data[128..132].copy_from_slice(&SUBPROJECT_LISTEND.to_le_bytes());

        props9(&[
            (PropsKey::SubprojectData.get_value(), data),
            (PropsKey::MinutesPerDay.get_value(), 480i32.to_le_bytes().to_vec()),
            (PropsKey::MinutesPerWeek.get_value(), 2400i32.to_le_bytes().to_vec()),
            (PropsKey::DaysPerMonth.get_value(), 20i16.to_le_bytes().to_vec()),
        ])
    }

    /// Open the root storage of the test file.
    fn open(bytes: Vec<u8>) -> (PoifsFileSystem, DirectoryEntry) {
        let fs = PoifsFileSystem::new(Cursor::new(bytes)).expect("open compound document");
        let root = fs.get_root();
        (fs, root)
    }

    /// Drive one complete parse.
    fn process(bytes: Vec<u8>, reader: &MPPReader) -> (MpxjResult<()>, Rc<RefCell<ProjectFile>>) {
        let (_fs, root) = open(bytes);
        let file = ProjectFile::new();
        let result = MPP9Reader::new().process(reader, &file, &root);
        (result, file)
    }

    /// Assert that two `NaiveDateTime` values are equal (for easier writing of expected values).
    fn date_time(year: i32, month: u32, day: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .expect("valid date")
            .and_hms_opt(0, 0, 0)
            .expect("valid time")
    }

    /// Full parse: the key fields of tasks, resources, calendars and relations.
    #[test]
    fn parses_tasks_resources_calendar_and_relations() {
        let reader = MPPReader::new();
        let (result, file) = process(build_mpp9(Broken::Nothing), &reader);
        assert!(result.is_ok(), "read failed: {:?}", result.err());

        let file_ref = file.borrow();

        // ---- Tasks ----
        let mut tasks: Vec<Rc<RefCell<Task>>> = file_ref
            .get_tasks()
            .iter()
            .map(Rc::clone)
            .collect();
        tasks.sort_by_key(|task| task.borrow().get_unique_id().unwrap_or(0));
        assert_eq!(tasks.len(), 2, "should read 2 tasks");

        let task_a = file_ref.get_task_by_unique_id(Some(1)).expect("task 1");
        {
            let task = task_a.borrow();
            assert_eq!(task.get_name().as_deref(), Some("Task 1"));
            assert_eq!(task.get_id(), Some(1));
            assert_eq!(task.get_unique_id(), Some(1));
            assert!(!task.get_null());
            // Duration: 4800 tenths of a minute, units in days → 1 day
            let duration = task.get_duration().expect("duration");
            assert_eq!(duration.get_duration(), 1.0);
            assert_eq!(duration.get_units(), TimeUnit::Days);
            // Start time: 1983-12-31 + 2 days
            assert_eq!(task.get_start(), Some(date_time(1984, 1, 2)));
            // Metadata flags
            assert!(task.get_milestone());
            assert!(task.get_marked());
            assert!(task.get_hide_bar());
            assert!(task.get_rollup());
            assert!(task.get_ignore_resource_calendar());
            assert!(task.get_effort_driven());
            assert!(task.get_level_assignments());
            assert!(task.get_leveling_can_split());
            assert!(task.get_flag(1));
            assert!(task.get_flag(4));
            assert!(!task.get_flag(2));
            // `(metaData[12] & 0x02) == 0` → expanded
            assert!(task.get_expanded());
            // Calendar link (fixed-length data block offset 160 = 1)
            assert_eq!(task.get_calendar_unique_id(), Some(1));
            assert!(task.get_calendar().is_some(), "task should be associated with a calendar");
        }

        let task_b = file_ref.get_task_by_unique_id(Some(2)).expect("task 2");
        {
            let task = task_b.borrow();
            assert_eq!(task.get_name().as_deref(), Some("Task 2"));
            assert_eq!(task.get_id(), Some(2));
            // Flags not set
            assert!(!task.get_milestone());
            assert!(!task.get_flag(1));
            assert!(task.get_calendar().is_none());
        }

        // ---- Relations: task B's predecessor is task A ----
        let predecessors = task_b.borrow().get_predecessors();
        assert_eq!(predecessors.len(), 1, "task 2 should have 1 predecessor task");
        {
            let relation = predecessors[0].borrow();
            assert_eq!(
                relation.get_predecessor_task().map(|task| task.borrow().get_unique_id()),
                Some(Some(1))
            );
        }

        // ---- Resources ----
        let resource = file_ref.get_resource_by_unique_id(Some(1)).expect("resource 1");
        {
            let resource = resource.borrow();
            assert_eq!(resource.get_name().as_deref(), Some("Resource 1"));
            assert_eq!(resource.get_id(), Some(7));
            assert_eq!(resource.get_unique_id(), Some(1));
            assert_eq!(resource.get_type(), ResourceType::Work);
            // Metadata flags (0x40 / 0x20 at offset 28)
            assert!(resource.get_flag_at(1));
            assert!(resource.get_flag_at(10));
            assert!(!resource.get_flag_at(2));
        }

        // ---- Calendars ----
        let calendar = file_ref.get_calendar_by_unique_id(Some(1)).expect("calendar 1");
        assert_eq!(calendar.borrow().get_unique_id(), Some(1));

        // ---- Ignored errors: only the "missing summary information stream" kind should occur (the test file has neither of these streams) ----
        let unexpected: Vec<String> = file_ref
            .get_ignored_errors()
            .iter()
            .map(|error| error.message().to_owned())
            .filter(|message| !message.contains("SummaryInformation"))
            .collect();
        assert!(unexpected.is_empty(), "unexpected ignored error: {unexpected:?}");
    }

    /// When `getReadPropertiesOnly` is true, only the project properties are read.
    #[test]
    fn read_properties_only_skips_task_data() {
        let mut reader = MPPReader::new();
        reader.set_read_properties_only(true);

        let (result, file) = process(build_mpp9(Broken::Nothing), &reader);
        assert!(result.is_ok(), "read failed: {:?}", result.err());

        let file_ref = file.borrow();
        assert_eq!(file_ref.get_tasks().len(), 0);
        assert_eq!(file_ref.get_project_properties().get_mpp_file_type(), Some(9));
    }

    /// Returns `Err` when a required storage is missing; does not panic.
    #[test]
    fn missing_project_directory_reports_error() {
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).expect("create compound document");
        file.create_stream("/Props9")
            .expect("Props9")
            .write_all(&props9(&[]))
            .expect("write Props9");
        let bytes = file.into_inner().into_inner();

        let reader = MPPReader::new();
        let (result, _file) = process(bytes, &reader);
        assert!(result.is_err(), "missing `   19` storage should error");
    }

    /// Returns `Err` when `Props9` is missing; does not panic.
    #[test]
    fn missing_props9_reports_error() {
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).expect("create compound document");
        file.create_storage("/   19").expect("   19");
        file.create_stream("/   19/Props")
            .expect("Props")
            .write_all(&props9(&[]))
            .expect("write Props");
        file.create_storage("/   29").expect("   29");
        let bytes = file.into_inner().into_inner();

        let reader = MPPReader::new();
        let (result, _file) = process(bytes, &reader);
        assert!(result.is_err(), "missing Props9 should error");
    }

    /// Returns `Err` when the calendar data is corrupted; does not panic.
    #[test]
    fn broken_calendar_data_does_not_panic() {
        let reader = MPPReader::new();
        let (result, file) = process(build_mpp9(Broken::Calendar), &reader);

        assert!(result.is_err(), "VarMeta with bad magic should cause read failure");
        assert_eq!(file.borrow().get_tasks().len(), 0);
    }

    /// Returns `Err` when the task data is corrupted; does not panic.
    #[test]
    fn broken_task_data_does_not_panic() {
        let reader = MPPReader::new();
        let (result, file) = process(build_mpp9(Broken::Task), &reader);

        assert!(result.is_err(), "task VarMeta with bad magic should cause read failure");
        assert_eq!(file.borrow().get_resources().len(), 1, "resources read before the task should be preserved");
    }

    /// An empty compound document (not even `\1CompObj`) does not panic.
    #[test]
    fn empty_compound_file_does_not_panic() {
        let bytes = CompoundFile::create(Cursor::new(Vec::new()))
            .expect("create compound document")
            .into_inner()
            .into_inner();

        let reader = MPPReader::new();
        let (result, _file) = process(bytes, &reader);
        assert!(result.is_err());
    }

    /// `processUniqueIdValues`: the rules for collecting subproject task unique IDs.
    #[test]
    fn process_unique_id_values_collects_external_tasks() {
        let bytes = build_mpp9(Broken::Nothing);
        let (_fs, root) = open(bytes);
        let reader = MPPReader::new();
        let file = ProjectFile::new();
        let mut state =
            MPP9ReaderState::populate_member_data(&reader, &file, &root).expect("establish read state");

        // List: [TASKUNIQUEID0, 42, LISTEND]
        let mut data = vec![0u8; 16];
        data[0..4].copy_from_slice(&SUBPROJECT_TASKUNIQUEID0.to_le_bytes());
        data[4..8].copy_from_slice(&42i32.to_le_bytes());
        data[8..12].copy_from_slice(&SUBPROJECT_LISTEND.to_le_bytes());

        state
            .process_unique_id_values("sub.mpp", &data, 0)
            .expect("parse unique ID list");

        // Unique ID 0 is recorded by TASKUNIQUEID0; number 42 is the "external task"
        assert_eq!(state.task_sub_projects.get(&0).map(String::as_str), Some("sub.mpp"));
        assert_eq!(state.task_sub_projects.get(&42).map(String::as_str), Some("sub.mpp"));
        assert!(state.external_tasks.contains(&42));
        assert!(!state.external_tasks.contains(&0));

        // Out-of-bounds offset: returns Err (Java throws and readSubProject catches it)
        assert!(state.process_unique_id_values("sub.mpp", &data, 14).is_err());
        // uniqueIDOffset == -1: return immediately
        assert!(state.process_unique_id_values("sub.mpp", &data, -1).is_ok());
    }

    /// `readSubProject`: reads the DOS path and records the mapping; out-of-bounds data does not panic.
    #[test]
    fn read_sub_project_reads_the_dos_path() {
        let bytes = build_mpp9(Broken::Nothing);
        let (_fs, root) = open(bytes);
        let reader = MPPReader::new();
        let file = ProjectFile::new();
        let mut state =
            MPP9ReaderState::populate_member_data(&reader, &file, &root).expect("establish read state");

        let mut data = vec![0u8; 200];
        let dos_path = b"C:\\sub.mpp\0";
        data[22..22 + dos_path.len()].copy_from_slice(dos_path);
        // The 4-byte size of the file name block is 0 → use the DOS path
        data[55..59].copy_from_slice(&0i32.to_le_bytes());
        // Unique ID list
        data[120..124].copy_from_slice(&SUBPROJECT_TASKUNIQUEID0.to_le_bytes());
        data[124..128].copy_from_slice(&7i32.to_le_bytes());
        data[128..132].copy_from_slice(&SUBPROJECT_LISTEND.to_le_bytes());

        state.read_sub_project(&data, 0, 120, 0, 0, 1);

        assert_eq!(
            state.task_sub_projects.get(&7).map(String::as_str),
            Some("C:\\sub.mpp")
        );
        assert!(file.borrow().get_ignored_errors().is_empty());

        // Data too short: records an ignored error but does not panic
        state.read_sub_project(&[0u8; 4], 0, 120, 0, 0, 1);
        assert_eq!(file.borrow().get_ignored_errors().len(), 1);
    }

    /// The subproject data (`SUBPROJECT_DATA`) is parsed by `processSubProjectData`.
    #[test]
    fn process_sub_project_data_reads_the_props_block() {
        let bytes = build_mpp9(Broken::Nothing);
        let (_fs, root) = open(bytes);
        let reader = MPPReader::new();
        let file = ProjectFile::new();
        let mut state =
            MPP9ReaderState::populate_member_data(&reader, &file, &root).expect("establish read state");

        state.process_sub_project_data();

        assert_eq!(
            state.task_sub_projects.get(&42).map(String::as_str),
            Some("C:\\sub.mpp")
        );
        assert!(state.external_tasks.contains(&42));
        assert!(file.borrow().get_ignored_errors().is_empty());
    }

    /// `getDurationEstimated`: the "confirmed" flag in the duration units field.
    #[test]
    fn duration_estimated_uses_the_confirmed_mask() {
        assert!(MPP9ReaderState::get_duration_estimated(0x20));
        assert!(MPP9ReaderState::get_duration_estimated(0x27));
        assert!(!MPP9ReaderState::get_duration_estimated(0x07));
        assert!(!MPP9ReaderState::get_duration_estimated(0));
    }

    /// Size and key mappings of the field name alias tables (copied item by item from Java's two static tables).
    #[test]
    fn field_name_alias_tables_match_java() {
        let resource_aliases = resource_field_aliases();
        assert_eq!(resource_aliases.len(), 130);
        // 52..81 → TEXT1..TEXT30
        assert_eq!(resource_aliases.get(&52), Some(&FieldKey::Resource(ResourceField::Text1)));
        assert_eq!(resource_aliases.get(&81), Some(&FieldKey::Resource(ResourceField::Text30)));
        // 82..91 → START1..START10;92..101 → FINISH1..FINISH10
        assert_eq!(resource_aliases.get(&82), Some(&FieldKey::Resource(ResourceField::Start1)));
        assert_eq!(resource_aliases.get(&101), Some(&FieldKey::Resource(ResourceField::Finish10)));
        // 165 → FLAG10,166..174 → FLAG1..FLAG9,175..184 → FLAG11..FLAG20
        assert_eq!(resource_aliases.get(&165), Some(&FieldKey::Resource(ResourceField::Flag10)));
        assert_eq!(resource_aliases.get(&166), Some(&FieldKey::Resource(ResourceField::Flag1)));
        assert_eq!(resource_aliases.get(&174), Some(&FieldKey::Resource(ResourceField::Flag9)));
        assert_eq!(resource_aliases.get(&175), Some(&FieldKey::Resource(ResourceField::Flag11)));
        assert_eq!(resource_aliases.get(&184), Some(&FieldKey::Resource(ResourceField::Flag20)));
        // 207..216 → COST1..COST10
        assert_eq!(resource_aliases.get(&216), Some(&FieldKey::Resource(ResourceField::Cost10)));

        let task_aliases = task_field_aliases();
        assert_eq!(task_aliases.len(), 130);
        assert_eq!(task_aliases.get(&118), Some(&FieldKey::Task(TaskField::Text1)));
        assert_eq!(task_aliases.get(&127), Some(&FieldKey::Task(TaskField::Text10)));
        // START/FINISH in pairs: 128/129 → START1/FINISH1, 146/147 → START10/FINISH10
        assert_eq!(task_aliases.get(&128), Some(&FieldKey::Task(TaskField::Start1)));
        assert_eq!(task_aliases.get(&129), Some(&FieldKey::Task(TaskField::Finish1)));
        assert_eq!(task_aliases.get(&146), Some(&FieldKey::Task(TaskField::Start10)));
        assert_eq!(task_aliases.get(&147), Some(&FieldKey::Task(TaskField::Finish10)));
        // DURATION keys are spaced one entry apart: 159/161/…/177
        assert_eq!(task_aliases.get(&159), Some(&FieldKey::Task(TaskField::Duration1)));
        assert_eq!(task_aliases.get(&161), Some(&FieldKey::Task(TaskField::Duration2)));
        assert_eq!(task_aliases.get(&177), Some(&FieldKey::Task(TaskField::Duration10)));
        // 194..213 → TEXT11..TEXT30;214..223 → NUMBER11..NUMBER20
        assert_eq!(task_aliases.get(&194), Some(&FieldKey::Task(TaskField::Text11)));
        assert_eq!(task_aliases.get(&213), Some(&FieldKey::Task(TaskField::Text30)));
        assert_eq!(task_aliases.get(&223), Some(&FieldKey::Task(TaskField::Number20)));
        // 227..236 → OUTLINE_CODE1..10;237..256 → FLAG1..FLAG20;278..287 → COST1..COST10
        assert_eq!(task_aliases.get(&236), Some(&FieldKey::Task(TaskField::OutlineCode10)));
        assert_eq!(task_aliases.get(&237), Some(&FieldKey::Task(TaskField::Flag1)));
        assert_eq!(task_aliases.get(&256), Some(&FieldKey::Task(TaskField::Flag20)));
        assert_eq!(task_aliases.get(&287), Some(&FieldKey::Task(TaskField::Cost10)));
    }
}
