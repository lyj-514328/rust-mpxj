//! Corresponds to Java: `org/mpxj/mpp/MPP12Reader.java`
//!
//! Variant reader for Project 12 (MSP 2007) MPP files: reads in the exact order used by Java
//! "project properties → subprojects → graphical indicators → custom fields → calendars → resources → tasks →
//! relations → assignments → post-processing → data links → presentation layer", writing the results into [`ProjectFile`].
//!
//! # Differences from Java (trade-offs are collected here; the body only marks key points)
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `final class MPP12Reader implements MPPVariantReader` | `pub struct MPP12Reader` (no fields) + `impl MPPVariantReader` | Java's 15 member fields are all "temporary state for the duration of one `process`"; on the Rust side they are collected into the private [`ReaderState`] (`populateMemberData` creates it, `clearMemberData` clears it, and it is released when it leaves scope), while `MPP12Reader` itself is a stateless unit struct (the same style as `ConstraintFactory` / `ResourceAssignmentFactory` / `AvailabilityFactory` in the same package) |
//! | `private MPPReader m_reader` | the `reader: &'a MPPReader` field of [`ReaderState`] | the trait's `process` already passes the reader in as a parameter, and the state struct stores it by borrow, equivalent to Java's member reference |
//! | `private ProjectFile m_file` | `Rc<RefCell<ProjectFile>>` | crate convention: `ProjectFile` is shared via `Rc<RefCell<..>>`; all access uses `try_borrow` / `try_borrow_mut` (Java has a stable reference; on a Rust borrow conflict the code skips / returns `Err` and documents it in a comment) |
//! | `file.getEventManager()` | `file.get_project_context().borrow_mut().get_event_manager_mut()` | `ProjectFile::get_event_manager()` only returns an immutable `Ref`, so event methods taking `&mut self` cannot be called (the same existing approach as `ConstraintFactory` / `ResourceAssignmentFactory`) |
//! | `m_projectDir` / `m_viewDir` / `m_root` (`DirectoryEntry`) | same-typed fields (`DirectoryEntry` is an `Rc` handle on the Rust side and can be `Clone`d freely) | no difference |
//! | `new DocumentInputStream((DocumentEntry) dir.getEntry(name))` | `dir.get_document(name)?.get_data()` (helper [`read_document`]) | per task convention, constructors that take an `InputStream` take `&[u8]` instead; when the entry is missing Java throws `FileNotFoundException` and Rust returns `Err` |
//! | `m_inputStreamFactory.getInstance(dir, name)` | [`DocumentInputStreamFactory::get_instance`] + `MppInputStream::to_bytes()` | encrypted files return the decrypted bytes, matching Java's behavior |
//! | `new FixedData(meta, is, 768, fieldMap.getMaxFixedDataSize(0))` | `FixedData::with_limits(&meta, data, 768, max)` (`max` comes from `get_max_fixed_data_size(0)`, and is `None` when out of bounds → 0) | `FieldMap::get_max_fixed_data_size` returns `None` when out of bounds (Java throws an exception) |
//! | `new FixedData(48, is)` / `(138, is)` / `(230, is)` | [`FixedData::with_fixed_size`] | corresponds one-to-one with Java's three "fixed length" overloads |
//! | `new FixedMeta(is, n)` | [`FixedMeta::with_item_size`] | on a bad magic / insufficient data Java throws `IOException` and Rust returns `Err` |
//! | `TreeMap` / `HashMap` / `HashSet` | `BTreeMap` / `HashMap` / `BTreeSet` | per crate convention (`BTreeMap`'s ascending iteration matches Java's `TreeMap`, and `postProcessTasks` relies on that order) |
//! | `Integer` (nullable) | `Option<i32>` | `createTaskMap`'s "deleted task" uses `None` as a placeholder, corresponding to Java's `taskMap.put(key, null)` |
//! | after `taskMap.get(uniqueID)`, directly `offset.intValue()` | `task_map.get(&unique_id).copied().flatten()` | deleted tasks do not take part in the following loop, equivalent to Java's `isValidOffset(null)` returning false |
//! | Java array out of bounds (`ByteArrayHelper.getInt` / `data[i]` / `metaData[i]`) throws `ArrayIndexOutOfBoundsException` | [`opt_int`] / [`opt_short`] / [`opt_long`] / [`opt_u8`] return `None`, and call sites `break` / `continue` / use 0 | follows the "library code never panics" convention |
//! | when `task.getConstraintType()` is `null`, Java's `switch` throws an NPE | the `_` branch of the `match` skips | same as above |
//! | `metaData[i]` (Java's `byte` is signed) | [`opt_u8`] + bit-mask comparison | a single-bit test is insensitive to sign extension, so the result matches Java |
//! | `m_outlineCodeVarData` is given both to `CustomFieldValueReader12` (stored by reference) and to outline code value lookup | split into two equivalent instances: field `outline_code_var_data` for value lookup, and `custom_field_outline_code_var_data` for `processCustomFields` to take ownership of | `CustomFieldValueReader12::new` takes `Var2Data` by value, and `Var2Data` is not `Clone`; both instances are built from the same bytes and have identical content |
//! | `m_outlineCodeFixedData` / `m_outlineCodeFixedData2` | field type is `Option<FixedData>`, with `take()` in `processCustomFields` | same as above: constructors take by value |
//! | `getCustomFieldOutlineCodeValue` (two overloads: 4-arg / 3-arg) | [`ReaderState::get_custom_field_outline_code_value`] (4-arg) and [`ReaderState::get_custom_field_outline_code_value_by_id`] (3-arg) | Rust has no overloading, so the 3-arg version gets the `_by_id` suffix; internally there is also `outline_code_value_by_id` with a depth limit (see below) |
//! | recursive lookup of parent outline codes (Java relies on the data being acyclic; a cycle gives `StackOverflowError`) | recursive depth limit [`MAX_OUTLINE_CODE_DEPTH`], returning `None` when exceeded | an extension of the "no panic" convention (stack overflow cannot be caught) |
//! | when `item.getParentUniqueID()` is `null` Java throws an NPE | treated as "no parent", no recursion | same as above |
//! | `map.put(lookupTableGuid, field)` (`field` may be `null`) | not inserted when `field` is `None` | `CustomFieldValueReader12` receives `HashMap<Uuid, MppFieldType>` (no `null` values), see [`ReaderState::populate_lookup_table_map`] |
//! | `task.getCachedValue(TaskField.CALENDAR_UNIQUE_ID)` | `task.get_cached_value(FieldKey::Task(TaskField::CalendarUniqueId))` + `as_integer()` | field keys use [`FieldKey`] (crate convention) |
//! | `task.set(TaskField.ENTERPRISE_FLAG1, Boolean)` / `resource.set(...)` | `set(FieldKey, Some(FieldValue::Boolean(..)))` | same as above (`Resource`'s `set` comes from [`FieldContainer`]) |
//! | `resource.setFlag(n, v)` / `resource.setOutlineCode(n, v)` | `set_flag_at` / `set_outline_code_at` | the existing naming for indexed setters on the Rust side |
//! | `resource.setUniqueID` / `setID` | `ProjectEntityWithMutableUniqueID::set_unique_id` / `ProjectEntityWithID::set_id` | these two methods are trait methods on the Rust side and must be in scope |
//! | `NumberHelper.getInt(Integer)` | private helper [`number_get_int`] | `NumberHelper::get_int` takes `Option<f64>`, whereas the argument here is `Option<i32>` |
//! | `MicrosoftProjectConstants.EPOCH_DATE` / `MAX_UNIQUE_ID` | [`epoch_date`] / [`MAX_UNIQUE_ID`] | constant values come from the same-named Java constants (1983-12-31T00:00, 0x1FFFFF); `MicrosoftProjectConstants` is not present in this crate |
//! | `Collections.sort(externalTasks)` (`Task implements Comparable`) | `sort_by(\|a, b\| a.compare_to(&b))` | uses `Task::compare_to` (Java's `compareTo`) |
//! | `throw new MPXJException("Unable to fix task order")` | `Err(MpxjError::new("Unable to fix task order"))` | unified error type |
//! | `recurringTaskReader.processRecurringTask(task, recurringData)` (writes back to task in place) | `reader.process_recurring_task(data)`, then writes the result via `task.add_recurring_task()` | writing to a `Task` on the Rust side requires `&mut Task`, so "read" and "attach" are split into two steps (see the file header of `crate::mpp::recurring_task_reader`) |
//! | `finally { clearMemberData(); }` | in [`MPP12Reader::process`]: "[`ReaderState::process`] runs the body → `clear_member_data()` → return the result" | the body is exactly all statements inside Java's `try` block, and `clearMemberData` runs on both the success and failure paths |
//! | `toString` / `equals` / `hashCode` | none | not overridden by this class |
//! | `MPPUtility.convertRateFromHours(m_file, resource, STANDARD_RATE/OvertimeRate, …_UNITS)` takes `TimeUnit` from `resource.getCachedValue(unitsField)` | [`MPPUtility::convert_rate_from_hours`] (reconstructs `TimeUnit` internally from an integer) | `FieldValue` has no variant carrying a `TimeUnit`, so the units field is stored as an integer; the conversion logic is provided by `RateHelper::convert_from_hours` |
//! | `AvailabilityFactory.process(resource, data)` appends directly to `resource.getAvailability()` | the factory returns "the rows to append" and the call site writes them back via [`crate::Resource::add_availability`] | `get_availability()` only lends a read-only `Ref` |
//!
//! # Interfaces with related modules (verified consistent)
//!
//! `mpp_variant_reader.rs` / `mpp_reader.rs` were landed at the same time as this file; the actual signatures are:
//!
//! ```ignore
//! // crate::mpp::mpp_variant_reader
//! pub trait MPPVariantReader {
//!     fn process(
//!         &self,
//!         reader: &MPPReader,
//!         file: &Rc<RefCell<ProjectFile>>,
//!         root: &DirectoryEntry,
//!     ) -> MpxjResult<()>;
//! }
//!
//! // crate::mpp::mpp_reader
//! impl MPPReader {
//!     pub fn get_read_properties_only(&self) -> bool;      // Java: getReadPropertiesOnly()
//!     pub fn get_read_presentation_data(&self) -> bool;    // Java: getReadPresentationData()
//!     pub fn get_use_raw_timephased_data(&self) -> bool;   // Java: getUseRawTimephasedData()
//! }
//! ```
//!
//! This file uses only the 4 symbols above; the contemporaneous `MPP9Reader` also uses the same "zero-sized reader + private
//! `ReaderState`" structure (see [`crate::mpp::mpp9_reader`]), making side-by-side comparison easy.
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;

use chrono::NaiveDateTime;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::local_date_time_helper::LocalDateTimeHelper;
use crate::common::mpp_field_type::MppFieldType;
use crate::common::number_helper::NumberHelper;
use crate::constraint_type::ConstraintType;
use crate::custom_field_lookup_table::CustomFieldValueItem;
use crate::field_key::FieldKey;
use crate::field_type_class::FieldTypeClass;
use crate::field_value::FieldValue;
use crate::mpp::availability_factory::AvailabilityFactory;
use crate::mpp::constraint_factory::ConstraintFactory;
use crate::mpp::cost_rate_table_factory::CostRateTableFactory;
use crate::mpp::custom_field_reader12::CustomFieldReader12;
use crate::mpp::custom_field_value_reader12::CustomFieldValueReader12;
use crate::mpp::data_link_factory::DataLinkFactory;
use crate::mpp::document_input_stream_factory::DocumentInputStreamFactory;
use crate::mpp::field_map::FieldMap;
use crate::mpp::field_map12::FieldMap12;
use crate::mpp::filter_reader::FilterReader;
use crate::mpp::filter_reader12::FilterReader12;
use crate::mpp::fixed_data::FixedData;
use crate::mpp::fixed_meta::FixedMeta;
use crate::mpp::font_base::FontBase;
use crate::mpp::graphical_indicator_reader::GraphicalIndicatorReader;
use crate::mpp::group_reader::GroupReader;
use crate::mpp::group_reader12::GroupReader12;
use crate::mpp::hyperlink_reader::HyperlinkReader;
use crate::mpp::mpp12_calendar_factory::MPP12CalendarFactory;
use crate::mpp::mpp_reader::MPPReader;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::mpp_variant_reader::MPPVariantReader;
use crate::mpp::ole::DirectoryEntry;
use crate::mpp::project_properties_reader::ProjectPropertiesReader;
use crate::mpp::props12::Props12;
use crate::mpp::props14::Props14;
use crate::mpp::props_key::PropsKey;
use crate::mpp::recurring_task_reader::RecurringTaskReader;
use crate::mpp::resource_assignment_factory::ResourceAssignmentFactory;
use crate::mpp::table_factory::TableFactory;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_meta::VarMeta;
use crate::mpp::var_meta12::VarMeta12;
use crate::mpp::view_factory::ViewFactory;
use crate::mpp::view_factory12::ViewFactory12;
use crate::mpp::view_state_reader12::ViewStateReader12;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_calendar::ProjectCalendar;
use crate::project_entity_with_id::ProjectEntityWithID;
use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
use crate::project_file::ProjectFile;
use crate::resource::Resource;
use crate::resource_field::ResourceField;
use crate::resource_type::ResourceType;
use crate::task::Task;
use crate::task_field::TaskField;

// ---------------------------------------------------------------------------
// Constants (corresponding to Java's private static final members)
// ---------------------------------------------------------------------------

/// Signal: the previous value is a subproject task unique ID.
///
/// Corresponds to Java: `private static final int SUBPROJECT_TASKUNIQUEID0 = 0x00000000;`
const SUBPROJECT_TASKUNIQUEID0: i32 = 0x0000_0000;
/// Corresponds to Java: `private static final int SUBPROJECT_TASKUNIQUEID1 = 0x0B340000;`
const SUBPROJECT_TASKUNIQUEID1: i32 = 0x0B34_0000;
/// Corresponds to Java: `private static final int SUBPROJECT_TASKUNIQUEID2 = 0x0ABB0000;`
const SUBPROJECT_TASKUNIQUEID2: i32 = 0x0ABB_0000;
/// Corresponds to Java: `private static final int SUBPROJECT_TASKUNIQUEID3 = 0x05A10000;`
const SUBPROJECT_TASKUNIQUEID3: i32 = 0x05A1_0000;
/// Corresponds to Java: `private static final int SUBPROJECT_TASKUNIQUEID4 = 0x0BD50000;`
const SUBPROJECT_TASKUNIQUEID4: i32 = 0x0BD5_0000;
/// Corresponds to Java: `private static final int SUBPROJECT_TASKUNIQUEID5 = 0x03D60000;`
const SUBPROJECT_TASKUNIQUEID5: i32 = 0x03D6_0000;
/// Corresponds to Java: `private static final int SUBPROJECT_TASKUNIQUEID6 = 0x07010000;`
const SUBPROJECT_TASKUNIQUEID6: i32 = 0x0701_0000;

/// Table column data key: standard column.
///
/// Corresponds to Java: `private static final Integer TABLE_COLUMN_DATA_STANDARD = Integer.valueOf(6);`
const TABLE_COLUMN_DATA_STANDARD: i32 = 6;
/// Table column data key: enterprise column.
///
/// Corresponds to Java: `private static final Integer TABLE_COLUMN_DATA_ENTERPRISE = Integer.valueOf(7);`
const TABLE_COLUMN_DATA_ENTERPRISE: i32 = 7;
/// Table column data key: baseline column.
///
/// Corresponds to Java: `private static final Integer TABLE_COLUMN_DATA_BASELINE = Integer.valueOf(8);`
const TABLE_COLUMN_DATA_BASELINE: i32 = 8;

/// Data type of outline code data in the variable-length block.
///
/// Corresponds to Java: `private static final Integer OUTLINECODE_DATA = Integer.valueOf(22);`
const OUTLINECODE_DATA: i32 = 22;

/// Mask for the custom value list.
///
/// Corresponds to Java: `private static final int VALUE_LIST_MASK = 0x0700;`
const VALUE_LIST_MASK: i32 = 0x0700;

/// Mask for the "duration confirmed" flag in the duration units field.
///
/// Corresponds to Java: `private static final int DURATION_CONFIRMED_MASK = 0x20;`
const DURATION_CONFIRMED_MASK: i32 = 0x20;

/// Fixed offset of the unique ID of a deleted / empty task.
///
/// Corresponds to Java: `private static final int TASK_UNIQUE_ID_FIXED_OFFSET = 0;`
const TASK_UNIQUE_ID_FIXED_OFFSET: i32 = 0;
/// Fixed offset of the ID of a deleted / empty task.
///
/// Corresponds to Java: `private static final int TASK_ID_FIXED_OFFSET = 4;`
const TASK_ID_FIXED_OFFSET: i32 = 4;
/// Byte size of the "empty task" fixed-length data block.
///
/// Corresponds to Java: `private static final int NULL_TASK_BLOCK_SIZE = 16;`
const NULL_TASK_BLOCK_SIZE: usize = 16;

/// Maximum unique ID allowed by MS Project.
///
/// Corresponds to Java: `org.mpxj.common.MicrosoftProjectConstants.MAX_UNIQUE_ID` (that constants class is not present in this crate).
const MAX_UNIQUE_ID: i32 = 0x1F_FFFF;

/// Maximum depth of parent outline code recursion (Rust-specific, see the file header difference table).
const MAX_OUTLINE_CODE_DEPTH: usize = 64;

/// Returns the epoch time used by the MPP date algorithm (1983-12-31T00:00).
///
/// Corresponds to Java: `org.mpxj.common.MicrosoftProjectConstants.EPOCH_DATE`
fn epoch_date() -> Option<NaiveDateTime> {
    chrono::NaiveDate::from_ymd_opt(1983, 12, 31)?.and_hms_opt(0, 0, 0)
}

/// Wraps a built-in task field as an [`MppFieldType`] (Rust-specific, a private helper like those in `field_map12`).
fn task_field(field: TaskField) -> MppFieldType {
    MppFieldType::Builtin(FieldKey::Task(field))
}

/// Wraps a built-in resource field as an [`MppFieldType`] (Rust-specific).
fn resource_field(field: ResourceField) -> MppFieldType {
    MppFieldType::Builtin(FieldKey::Resource(field))
}

/// Returns the integer value of an `Integer` (`null` → 0).
///
/// Corresponds to Java: `NumberHelper.getInt(Number)` -- Rust's [`NumberHelper::get_int`] takes
/// `Option<f64>`, whereas the argument in this file is `Option<i32>`, so the equivalent logic is inlined here.
fn number_get_int(value: Option<i32>) -> i32 {
    value.unwrap_or(0)
}

/// Reader for MPP12 (Project 2007) files.
///
/// Corresponds to Java: `final class MPP12Reader implements MPPVariantReader`
pub struct MPP12Reader;

impl MPP12Reader {
/// Constructor.
    ///
/// Corresponds to Java: implicit default constructor (on the Java side `MPPReader` directly does `new MPP12Reader()`).
    pub fn new() -> Self {
        Self
    }

/// Processes one MPP12 file.
    ///
    /// Corresponds to Java: `@Override public void process(MPPReader reader, ProjectFile file, DirectoryEntry root)`
    ///
/// Java's `try { ... } finally { clearMemberData(); }` is expressed on the Rust side as
/// "create state → run the body → clear state → return the result", with clearing performed on both paths.
    pub fn process(
        &self,
        reader: &MPPReader,
        file: &Rc<RefCell<ProjectFile>>,
        root: &DirectoryEntry,
    ) -> MpxjResult<()> {
        // Java:
        // try { populateMemberData(reader, file, root); processProjectProperties(); ... }
        // finally { clearMemberData(); }
        //
// When `populateMemberData` fails the state has not been created yet (at that point Java has partially assigned its fields, but the
// `clearMemberData` in `finally` only nulls the references); on the Rust side there is no state to clear, so it returns the error directly.
        let mut state = ReaderState::populate_member_data(reader, file, root)?;
        let result = state.process();
        state.clear_member_data();
        result
    }
}

impl Default for MPP12Reader {
/// Corresponds to Java's implicit default constructor, equivalent to [`MPP12Reader::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl MPPVariantReader for MPP12Reader {
    /// Corresponds to Java: `MPP12Reader#process(MPPReader, ProjectFile, DirectoryEntry)`
    fn process(
        &self,
        reader: &MPPReader,
        file: &Rc<RefCell<ProjectFile>>,
        root: &DirectoryEntry,
    ) -> MpxjResult<()> {
        MPP12Reader::process(self, reader, file, root)
    }
}

/// All temporary state for the duration of one `process`.
///
/// Corresponds to Java: the 15 member fields of `MPP12Reader` (`m_reader` … `m_inputStreamFactory`).
///
/// Java nulls them all out in `clearMemberData()`; on the Rust side [`ReaderState::clear_member_data`]
/// clears the collection fields and the remaining fields are released when the struct leaves scope (equivalent).
struct ReaderState<'a> {
    /// Corresponds to Java: `private MPPReader m_reader;`
    reader: &'a MPPReader,
    /// Corresponds to Java: `private ProjectFile m_file;`
    file: Rc<RefCell<ProjectFile>>,
    /// Corresponds to Java: `private DirectoryEntry m_root;`
    root: DirectoryEntry,
    /// Corresponds to Java: `private HashMap<Integer, ProjectCalendar> m_resourceMap;`
    resource_map: HashMap<i32, Rc<RefCell<ProjectCalendar>>>,
    /// Corresponds to Java: `private Var2Data m_outlineCodeVarData;`
    ///
/// "The instance used for value lookup" (used by `getCustomFieldOutlineCodeValue`).
    outline_code_var_data: Var2Data,
/// Corresponds to Java: the same `m_outlineCodeVarData`, used by `processCustomFields` to take ownership.
    ///
/// Rust-specific: `CustomFieldValueReader12::new` takes `Var2Data` by value and `Var2Data` is not `Clone`,
/// so an equivalent instance is rebuilt from the same bytes (see the file header difference table).
    custom_field_outline_code_var_data: Option<Var2Data>,
    /// Corresponds to Java: `private VarMeta m_outlineCodeVarMeta;`
    outline_code_var_meta: Rc<VarMeta12>,
    /// Corresponds to Java: `private FixedData m_outlineCodeFixedData;`
    outline_code_fixed_data: Option<FixedData>,
    /// Corresponds to Java: `private FixedData m_outlineCodeFixedData2;`
    outline_code_fixed_data2: Option<FixedData>,
    /// Corresponds to Java: `private Props m_projectProps;`
    project_props: Props12,
    /// Corresponds to Java: `private Map<Integer, FontBase> m_fontBases;`
    font_bases: HashMap<i32, FontBase>,
    /// Corresponds to Java: `private Map<Integer, String> m_taskSubProjects;`
    task_sub_projects: HashMap<i32, String>,
    /// Corresponds to Java: `private Set<Integer> m_externalTasks;`
    external_tasks: BTreeSet<i32>,
    /// Corresponds to Java: `private DirectoryEntry m_projectDir;`
    project_dir: DirectoryEntry,
    /// Corresponds to Java: `private DirectoryEntry m_viewDir;`
    view_dir: DirectoryEntry,
    /// Corresponds to Java: `private Map<Long, Integer> m_taskOrder;`
    task_order: BTreeMap<i64, i32>,
    /// Corresponds to Java: `private Map<Integer, Integer> m_nullTaskOrder;`
    null_task_order: BTreeMap<i32, i32>,
    /// Corresponds to Java: `private DocumentInputStreamFactory m_inputStreamFactory;`
    input_stream_factory: DocumentInputStreamFactory,
}

impl<'a> ReaderState<'a> {
/// Populates the member data needed throughout the read.
    ///
    /// Corresponds to Java: `private void populateMemberData(MPPReader reader, ProjectFile file, DirectoryEntry root) throws MPXJException, IOException`
    fn populate_member_data(
        reader: &'a MPPReader,
        file: &Rc<RefCell<ProjectFile>>,
        root: &DirectoryEntry,
    ) -> MpxjResult<Self> {
        // Java: m_reader = reader; m_file = file; m_eventManager = file.getEventManager(); m_root = root;
        let root = root.clone();

        //
        // Retrieve the high level document properties
        //
        // Java: Props props = new Props12(new DocumentInputStream(((DocumentEntry) root.getEntry("Props12"))));
        let props_document = root
            .get_document("Props12")
            .ok_or_else(|| MpxjError::new("Entry not found: Props12"))?;
        let props = Props12::new(&props_document.get_data())?;

        // Java: file.getProjectProperties().setProjectFilePath(props.getUnicodeString(PropsKey.PROJECT_FILE_PATH));
        let project_file_path = props.get_unicode_string(PropsKey::ProjectFilePath);
        if let Ok(mut file_ref) = file.try_borrow_mut() {
            file_ref
                .get_project_properties_mut()
                .set_project_file_path(project_file_path);
        }

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

        // Java: if (passwordRequiredToRead) { throw new MPXJException(MPXJException.PASSWORD_PROTECTED); }
        if password_required_to_read {
            return Err(MpxjError::new(MpxjError::PASSWORD_PROTECTED));
        }

        // Java: m_projectDir = (DirectoryEntry) root.getEntry("   112");
        //       m_viewDir = (DirectoryEntry) root.getEntry("   212");
        let project_dir = root
            .get_directory("   112")
            .ok_or_else(|| MpxjError::new("Entry not found:    112"))?;
        let view_dir = root
            .get_directory("   212")
            .ok_or_else(|| MpxjError::new("Entry not found:    212"))?;

        //
// Outline code block
        //
        // Java: DirectoryEntry outlineCodeDir = (DirectoryEntry) m_projectDir.getEntry("TBkndOutlCode");
        let outline_code_dir = project_dir
            .get_directory("TBkndOutlCode")
            .ok_or_else(|| MpxjError::new("Entry not found: TBkndOutlCode"))?;

        // Java: m_outlineCodeVarMeta = new VarMeta12(new DocumentInputStream(((DocumentEntry) outlineCodeDir.getEntry("VarMeta"))));
        let outline_code_var_meta =
            Rc::new(VarMeta12::new(&read_document(&outline_code_dir, "VarMeta")?)?);

        // Java: m_outlineCodeVarData = new Var2Data(m_file, m_outlineCodeVarMeta, new DocumentInputStream(...("Var2Data")));
        let outline_code_var_data_bytes = read_document(&outline_code_dir, "Var2Data")?;
        let outline_code_var_data = Var2Data::new(
            file,
            Rc::clone(&outline_code_var_meta) as Rc<dyn VarMeta>,
            &outline_code_var_data_bytes,
        );
// Rust-specific: build another equivalent instance for CustomFieldValueReader12 to take by value (see the field comment).
        let custom_field_outline_code_var_data = Some(Var2Data::new(
            file,
            Rc::clone(&outline_code_var_meta) as Rc<dyn VarMeta>,
            &outline_code_var_data_bytes,
        ));

        // Java: FixedMeta outlineCodeFixedMeta = new FixedMeta(new DocumentInputStream(...("FixedMeta")), 10);
        //       m_outlineCodeFixedData = new FixedData(outlineCodeFixedMeta, new DocumentInputStream(...("FixedData")));
        let outline_code_fixed_meta =
            FixedMeta::with_item_size(&read_document(&outline_code_dir, "FixedMeta")?, 10)?;
        let outline_code_fixed_data = Some(FixedData::new(
            &outline_code_fixed_meta,
            &read_document(&outline_code_dir, "FixedData")?,
        ));

        // Java: FixedMeta outlineCodeFixedMeta2 = new FixedMeta(new DocumentInputStream(...("Fixed2Meta")), 10);
        //       m_outlineCodeFixedData2 = new FixedData(outlineCodeFixedMeta2, new DocumentInputStream(...("Fixed2Data")));
        let outline_code_fixed_meta2 =
            FixedMeta::with_item_size(&read_document(&outline_code_dir, "Fixed2Meta")?, 10)?;
        let outline_code_fixed_data2 = Some(FixedData::new(
            &outline_code_fixed_meta2,
            &read_document(&outline_code_dir, "Fixed2Data")?,
        ));

        // Java: m_projectProps = new Props12(m_inputStreamFactory.getInstance(m_projectDir, "Props"));
        let mut props_stream = input_stream_factory.get_instance(&project_dir, "Props")?;
        let project_props = Props12::new(&props_stream.to_bytes())?;

        //
// Collection initialization + project properties
        //
        // Java: m_fontBases = new HashMap<>(); m_taskSubProjects = new HashMap<>(); m_externalTasks = new HashSet<>();
        //       m_taskOrder = new TreeMap<>(); m_nullTaskOrder = new TreeMap<>();
        // Java: m_file.getProjectProperties().setMppFileType(Integer.valueOf(12));
        //       m_file.getProjectProperties().setAutoFilter(props.getBoolean(PropsKey.AUTO_FILTER));
        let auto_filter = props.get_boolean(PropsKey::AutoFilter);
        if let Ok(mut file_ref) = file.try_borrow_mut() {
            let properties = file_ref.get_project_properties_mut();
            properties.set_mpp_file_type(Some(12));
            properties.set_auto_filter(auto_filter);
        }

        Ok(Self {
            reader,
            file: Rc::clone(file),
            root,
            resource_map: HashMap::new(),
            outline_code_var_data,
            custom_field_outline_code_var_data,
            outline_code_var_meta,
            outline_code_fixed_data,
            outline_code_fixed_data2,
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

/// Clears the temporary member data.
    ///
    /// Corresponds to Java: `private void clearMemberData()`
    ///
/// Java sets all fields to `null`; on the Rust side the collection fields are cleared and the remaining fields are released with the struct
/// (`clear_member_data` is called on `process`'s return path, after which `state` immediately leaves scope).
    fn clear_member_data(&mut self) {
        self.resource_map.clear();
        self.custom_field_outline_code_var_data = None;
        self.outline_code_fixed_data = None;
        self.outline_code_fixed_data2 = None;
        self.font_bases.clear();
        self.task_order.clear();
        self.null_task_order.clear();
        self.task_sub_projects.clear();
        self.external_tasks.clear();
    }

/// The body of `process`: all steps after `populateMemberData` and before `clearMemberData`.
    ///
/// Corresponds to Java: all statements after `populateMemberData(...)` in `MPP12Reader#process`
    fn process(&mut self) -> MpxjResult<()> {
        // Java: processProjectProperties();
        self.process_project_properties()?;

        // Java: if (!reader.getReadPropertiesOnly()) { ... }
        if !self.reader.get_read_properties_only() {
            // Java: processSubProjectData();
            self.process_sub_project_data();

            // Java: processGraphicalIndicators();
            self.process_graphical_indicators();

            // Java: processCustomFields();
            self.process_custom_fields()?;

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

            // Java: if (reader.getReadPresentationData()) { ... }
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

/// Reads the custom field definitions.
    ///
    /// Corresponds to Java: `private void processCustomFields() throws IOException`
    fn process_custom_fields(&mut self) -> MpxjResult<()> {
        // Ensure we have read the custom field definitions ...
        //
        // Java: DirectoryEntry taskDir = (DirectoryEntry) m_projectDir.getEntry("TBkndTask");
        //       if (taskDir.hasEntry("Props")) { Props12 props = new Props12(m_inputStreamFactory.getInstance(taskDir, "Props"));
        //          new CustomFieldReader12(m_file, props.getByteArray(PropsKey.CUSTOM_FIELDS)).process(); }
        if let Some(task_dir) = self.project_dir.get_directory("TBkndTask") {
            if has_entry(&task_dir, "Props") {
                let mut stream = self.input_stream_factory.get_instance(&task_dir, "Props")?;
                let props = Props12::new(&stream.to_bytes())?;
                CustomFieldReader12::new(&self.file, props.get_byte_array(PropsKey::CustomFields))
                    .process();
            }
        }

        // Java: DirectoryEntry rscDir = (DirectoryEntry) m_projectDir.getEntry("TBkndRsc");
        //       if (rscDir.hasEntry("Props")) { ... }
        if let Some(rsc_dir) = self.project_dir.get_directory("TBkndRsc") {
            if has_entry(&rsc_dir, "Props") {
                let mut stream = self.input_stream_factory.get_instance(&rsc_dir, "Props")?;
                let props = Props12::new(&stream.to_bytes())?;
                CustomFieldReader12::new(&self.file, props.get_byte_array(PropsKey::CustomFields))
                    .process();
            }
        }

        // ... before we add values lists
        // Java: Map<UUID, FieldType> lookupTableMap = new HashMap<>();
        //       populateLookupTableMap(lookupTableMap, (DirectoryEntry) m_projectDir.getEntry("TBkndTask"));
        //       populateLookupTableMap(lookupTableMap, (DirectoryEntry) m_projectDir.getEntry("TBkndRsc"));
        let mut lookup_table_map: HashMap<uuid::Uuid, MppFieldType> = HashMap::new();
        if let Some(task_dir) = self.project_dir.get_directory("TBkndTask") {
            self.populate_lookup_table_map(&mut lookup_table_map, &task_dir);
        }
        if let Some(rsc_dir) = self.project_dir.get_directory("TBkndRsc") {
            self.populate_lookup_table_map(&mut lookup_table_map, &rsc_dir);
        }

        // Java: CustomFieldValueReader12 reader = new CustomFieldValueReader12(m_file, lookupTableMap,
        //          m_outlineCodeVarMeta, m_outlineCodeVarData, m_outlineCodeFixedData, m_outlineCodeFixedData2);
        //       reader.process();
        //
// Rust-specific: Var2Data / FixedData are handed over by value (see the file header difference table); when unavailable they are skipped (no such case in Java).
        let (Some(outline_code_var_data), Some(outline_code_fixed_data), Some(outline_code_fixed_data2)) = (
            self.custom_field_outline_code_var_data.take(),
            self.outline_code_fixed_data.take(),
            self.outline_code_fixed_data2.take(),
        ) else {
            return Ok(());
        };

        let reader = CustomFieldValueReader12::new(
            &self.file,
            lookup_table_map,
            Rc::clone(&self.outline_code_var_meta) as Rc<dyn VarMeta>,
            outline_code_var_data,
            outline_code_fixed_data,
            outline_code_fixed_data2,
        );
        reader.process();

        Ok(())
    }

/// Writes "lookup table GUID → field type" into the map.
    ///
    /// Corresponds to Java: `private void populateLookupTableMap(Map<UUID, FieldType> map, DirectoryEntry dir) throws IOException`
    fn populate_lookup_table_map(
        &self,
        map: &mut HashMap<uuid::Uuid, MppFieldType>,
        dir: &DirectoryEntry,
    ) {
        // Java: if (!dir.hasEntry("Props")) { return; }
        if !has_entry(dir, "Props") {
            return;
        }

        // Java: Props props = new Props14(m_file, m_inputStreamFactory.getInstance(dir, "Props"));
        let Ok(mut stream) = self.input_stream_factory.get_instance(dir, "Props") else {
            return;
        };
        let Ok(props) = Props14::new(&self.file, &stream.to_bytes()) else {
            return;
        };

        // Java: byte[] data = props.getByteArray(PropsKey.CUSTOM_FIELDS); if (data == null) { return; }
        let Some(data) = props
            .get_byte_array(PropsKey::CustomFields)
            .map(<[u8]>::to_vec)
        else {
            return;
        };

        // Java: int length = ByteArrayHelper.getInt(data, 0); int index = length + 36;
// When data is insufficient Java throws an exception; the Rust side treats it as "no records".
        let Some(length) = opt_int(&data, 0) else {
            return;
        };
// Java's int arithmetic can wrap; it is received in an i64 and range-checked uniformly (see below).
        let mut index = i64::from(length) + 36;

        // 4 byte record count
        // Java: int recordCount = ByteArrayHelper.getInt(data, index); index += 4; index += (8 * recordCount);
        let Some(record_count) = read_int_i64(&data, index) else {
            return;
        };
        index += 4 + 8 * i64::from(record_count);

        // Java: while (index + 176 <= data.length)
// The Rust side adds an `index >= 0` check: Java would also enter the loop with a negative index and throw an exception.
        while index >= 0 && index + 176 <= data.len() as i64 {
            let index_usize = index as usize;

            // Java: int blockLength = ByteArrayHelper.getInt(data, index);
            //       if (blockLength <= 0 || index + blockLength > data.length) { break; }
            let block_length = ByteArrayHelper::get_int(&data, index_usize);
            if block_length <= 0 || index + i64::from(block_length) > data.len() as i64 {
                break;
            }

            // Java: int customFieldID = ByteArrayHelper.getInt(data, index + 4);
            //       FieldType field = FieldTypeHelper.getInstance(m_file, customFieldID);
            //       UUID lookupTableGuid = MPPUtility.getGUID(data, index + 160);
            //       if (lookupTableGuid != null) { map.put(lookupTableGuid, field); }
            //
// The Rust side does not insert when `field` is `None` (null on the Java side) -- the consumer's map has no null values.
            let custom_field_id = ByteArrayHelper::get_int(&data, index_usize + 4);
            let field = FieldTypeHelper::get_instance(&self.file, custom_field_id);
            let lookup_table_guid = MPPUtility::get_guid(&data, index_usize + 160);
            if let (Some(guid), Some(field)) = (lookup_table_guid, field) {
                map.insert(guid, field);
            }

            index += i64::from(block_length);
        }
    }

/// Reads the project properties data.
    ///
    /// Corresponds to Java: `private void processProjectProperties() throws MPXJException`
    fn process_project_properties(&self) -> MpxjResult<()> {
        // Java: ProjectPropertiesReader reader = new ProjectPropertiesReader();
        //       reader.process(m_file, m_projectProps, m_root);
        ProjectPropertiesReader.process(&self.file, &self.project_props, &self.root)
    }

/// Reads the graphical indicators data.
    ///
    /// Corresponds to Java: `private void processGraphicalIndicators()`
    fn process_graphical_indicators(&mut self) {
        // Java: GraphicalIndicatorReader graphicalIndicatorReader = new GraphicalIndicatorReader();
        //       graphicalIndicatorReader.process(m_file, m_projectProps);
        let mut graphical_indicator_reader = GraphicalIndicatorReader::new();
        graphical_indicator_reader.process(&self.file, &self.project_props);
    }

/// Reads the subproject data and indexes it by task ID.
    ///
/// Project saves, in order, all subprojects ever inserted into this project, and this is also the basis for
/// computing each subproject's unique ID offset.
    ///
    /// Corresponds to Java: `private void processSubProjectData()`
    fn process_sub_project_data(&mut self) {
        // Java: byte[] subProjData = m_projectProps.getByteArray(PropsKey.SUBPROJECT_DATA); if (subProjData == null) { return; }
        //
// Rust-specific: copy the properties data into a local buffer to avoid overlapping borrows between "reading project_props" and "modifying task_sub_projects"
// borrows.
        let Some(sub_proj_data) = self
            .project_props
            .get_byte_array(PropsKey::SubprojectData)
            .map(<[u8]>::to_vec)
        else {
            return;
        };
        let data: &[u8] = &sub_proj_data;

        let mut index: i32 = 0;
        let mut offset: i32 = 0;

        /*int blockSize = MPPUtility.getInt(subProjData, offset);*/
        offset += 4;

        /*int unknown = MPPUtility.getInt(subProjData, offset);*/
        offset += 4;

        // Java: int itemCountOffset = ByteArrayHelper.getInt(subProjData, offset); offset += 4;
        let Some(item_count_offset) = opt_int(data, offset) else {
            return;
        };
        offset += 4;

        // Java: while (offset < itemCountOffset)
        while offset < item_count_offset {
            index += 1;

            // Java: itemHeaderOffset = ByteArrayHelper.getInt(subProjData, offset) & 0x1FFFF; offset += 4;
            let Some(value) = opt_int(data, offset) else {
                break;
            };
            let item_header_offset = value & 0x1FFFF;
            offset += 4;

            // 20 byte header: 16 bytes GUID, 4 bytes flags
            // Java: byte subProjectType = subProjData[itemHeaderOffset + 16];
            let Some(sub_project_type) = opt_u8(data, item_header_offset + 16) else {
                break;
            };

            match sub_project_type {
                //
                // Subproject that is no longer inserted. This is a placeholder in order to be
                // able to always guarantee unique unique ids.
                //
                // deleted entry?
                0x00 | 0x10 => {
                    offset += 8;
                }

                // task unique ID, 8 bytes, path, file name
                0x0b | 0x99 | 0x09 | 0x0D => {
                    let Some(unique_id_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    // sometimes offset of a task ID?
                    if opt_int(data, offset).is_none() {
                        break;
                    }
                    offset += 4;
                    let Some(file_path_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_name_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    self.read_sub_projects(
                        data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // task unique ID, 8 bytes, path, file name
                0x03 | 0x11 | 0x91 => {
                    let Some(unique_id_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_path_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_name_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    // Unknown offset
                    offset += 4;
                    self.read_sub_projects(
                        data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // task unique ID, path, unknown, file name
                0x81 | 0x83 | 0x41 => {
                    let Some(unique_id_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_path_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    // unknown offset to 2 bytes of data?
                    offset += 4;
                    let Some(file_name_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    self.read_sub_projects(
                        data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // task unique ID, path, file name
                0x01 | 0x08 => {
                    let Some(unique_id_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_path_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_name_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    self.read_sub_projects(
                        data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // task unique ID, path, file name
                0xC0 => {
                    let unique_id_offset = item_header_offset;
                    let Some(file_path_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_name_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    // unknown offset
                    offset += 4;
                    self.read_sub_projects(
                        data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // resource, task unique ID, path, file name
                0x05 => {
                    let Some(unique_id_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_path_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_name_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    self.read_sub_project(
                        data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                0x45 => {
                    let Some(unique_id_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_path_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_name_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    offset += 4;
                    self.read_sub_project(
                        data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // path, file name
                0x02 => {
                    //filePathOffset = MPPUtility.getInt(subProjData, offset) & 0x1FFFF;
                    offset += 4;

                    //fileNameOffset = MPPUtility.getInt(subProjData, offset) & 0x1FFFF;
                    offset += 4;

                    // sp = readSubProject(subProjData, -1, filePathOffset, fileNameOffset, index);
                    // 0x02 looks to be the link FROM the resource pool to a project that is using it.
                }

                0x04 => {
                    let Some(file_path_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_name_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    self.read_sub_project(
                        data,
                        item_header_offset,
                        -1,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // task unique ID, 4 bytes, path, 4 bytes, file name
                0x89 | 0x8D => {
                    let Some(unique_id_offset) = opt_short(data, offset) else {
                        break;
                    };
                    offset += 8;
                    let Some(file_path_offset) = opt_short(data, offset) else {
                        break;
                    };
                    offset += 8;
                    let Some(file_name_offset) = opt_short(data, offset) else {
                        break;
                    };
                    offset += 4;
                    self.read_sub_projects(
                        data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // task unique ID, path, file name
                0x0A => {
                    let Some(unique_id_offset) = opt_short(data, offset) else {
                        break;
                    };
                    offset += 4;
                    let Some(file_path_offset) = opt_short(data, offset) else {
                        break;
                    };
                    offset += 4;
                    let Some(file_name_offset) = opt_short(data, offset) else {
                        break;
                    };
                    offset += 4;
                    self.read_sub_projects(
                        data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // new resource pool entry
                0x44 => {
                    let Some(file_path_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    offset += 4;
                    let Some(file_name_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    self.read_sub_projects(
                        data,
                        item_header_offset,
                        -1,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // Appears when a subproject is collapsed
                0x80 => {
                    offset += 12;
                }

                // Any other value, assume 12 bytes to handle old/deleted data?
                _ => {
                    offset += 12;
                }
            }
        }
    }

/// Reads a set of subprojects.
    ///
    /// Corresponds to Java: `private void readSubProjects(byte[] data, int itemHeaderOffset, int uniqueIDOffset, int filePathOffset, int fileNameOffset, int subprojectIndex)`
    fn read_sub_projects(
        &mut self,
        data: &[u8],
        item_header_offset: i32,
        mut unique_id_offset: i32,
        file_path_offset: i32,
        file_name_offset: i32,
        mut subproject_index: i32,
    ) {
        // Java: while (uniqueIDOffset < filePathOffset) { readSubProject(...); uniqueIDOffset += 4; }
        while unique_id_offset < file_path_offset {
            self.read_sub_project(
                data,
                item_header_offset,
                unique_id_offset,
                file_path_offset,
                file_name_offset,
                subproject_index,
            );
            unique_id_offset += 4;
            subproject_index += 1;
        }
    }

/// Reads the details of a single subproject from the byte array.
    ///
    /// Corresponds to Java: `private void readSubProject(byte[] data, int headerOffset, int uniqueIDOffset, int filePathOffset, int fileNameOffset, int subprojectIndex)`
    ///
/// On an out-of-bounds access (`ArrayIndexOutOfBoundsException`) Java records an ignored error and continues;
/// on the Rust side out of bounds is expressed by [`ReaderState::read_sub_project_path`] returning `None`, and an ignored error is likewise recorded
/// -- this is the only branch in this file that "swallows" an out-of-bounds access, corresponding one-to-one with Java's `catch`.
    fn read_sub_project(
        &mut self,
        data: &[u8],
        _header_offset: i32,
        unique_id_offset: i32,
        file_path_offset: i32,
        _file_name_offset: i32,
        _subproject_index: i32,
    ) {
// Java's `headerOffset` / `fileNameOffset` / `subprojectIndex` are not used in this method body.

        // Java: try { ... sp = ...; processUniqueIdValues(sp, data, uniqueIDOffset); }
        //       catch (ArrayIndexOutOfBoundsException ex) { m_file.addIgnoredError(ex); }
        let Some(path) = self.read_sub_project_path(data, unique_id_offset, file_path_offset) else {
            //
            // Admit defeat at this point - we have probably stumbled upon a data format we
            // don't understand, so we'll fail gracefully here. This will now be reported as a
            // missing sub project error by end users of the library, rather than as an
            // exception being thrown.
            //
            if let Ok(mut file) = self.file.try_borrow_mut() {
                file.add_ignored_error(MpxjError::new(
                    "Unable to read sub project data (array index out of bounds)",
                ));
            }
            return;
        };

        self.process_unique_id_values(path, data, unique_id_offset);
    }

/// Extracts the project path / file name from a subproject entry.
    ///
/// Corresponds to Java: the "compute `sp`" part of `readSubProject` (inlined inside Java's `try` block).
    fn read_sub_project_path(
        &self,
        data: &[u8],
        unique_id_offset: i32,
        file_path_offset: i32,
    ) -> Option<String> {
        // We have a 20 byte header.
        // First 16 bytes are (most of the time) the GUID of the target project
        // Remaining 4 bytes are believed to be flags
        // Java: int type = uniqueIDOffset == -1 ? SUBPROJECT_TASKUNIQUEID0 : ByteArrayHelper.getInt(data, uniqueIDOffset + 4);
        let type_ = if unique_id_offset == -1 {
            SUBPROJECT_TASKUNIQUEID0
        } else {
            opt_int(data, unique_id_offset + 4)?
        };

        // Generate the unique id offset for this subproject
        //int offset = 0x00800000 + ((subprojectIndex - 1) * 0x00400000);

        if type_ == SUBPROJECT_TASKUNIQUEID4 {
            // Java: sp = MPPUtility.getUnicodeString(data, filePathOffset);
            return Some(MPPUtility::get_unicode_string(
                data,
                usize::try_from(file_path_offset).ok()?,
            ));
        }

        //
        // First block header
        //
        let mut file_path_offset = file_path_offset + 18;

        //
        // String size as a 4 byte int
        //
        file_path_offset += 4;

        //
        // Full DOS path
        //
        // Java: String dosFullPath = MPPUtility.getString(data, filePathOffset);
        //       filePathOffset += (dosFullPath.length() + 1);
        let dos_full_path = MPPUtility::get_string(data, usize::try_from(file_path_offset).ok()?);
        file_path_offset += dos_full_path.chars().count() as i32 + 1;

        //
        // 24 byte block
        //
        file_path_offset += 24;

        //
        // 4 byte block size
        //
        // Java: int size = ByteArrayHelper.getInt(data, filePathOffset); filePathOffset += 4;
        let mut size = opt_int(data, file_path_offset)?;
        file_path_offset += 4;

        if size == 0 {
            // Java: sp = dosFullPath;
            Some(dos_full_path)
        } else {
            //
            // 4 byte unicode string size in bytes
            //
            size = opt_int(data, file_path_offset)?;
            file_path_offset += 4;

            //
            // 2 byte data
            //
            file_path_offset += 2;

            //
            // Unicode string
            //
            // Java: sp = MPPUtility.getUnicodeString(data, filePathOffset, size);
            let size = usize::try_from(size).ok()?;
            Some(MPPUtility::get_unicode_string_with_max_length(
                data,
                usize::try_from(file_path_offset).ok()?,
                size,
            ))
        }
    }

/// Records the value of the subproject task unique ID.
    ///
    /// Corresponds to Java: `private void processUniqueIdValues(String sp, byte[] data, int uniqueIDOffset)`
    fn process_unique_id_values(&mut self, sp: String, data: &[u8], unique_id_offset: i32) {
        // Java: if (uniqueIDOffset == -1) { return; }
        if unique_id_offset == -1 {
            return;
        }

        // Java: int value = ByteArrayHelper.getInt(data, uniqueIDOffset);
        //       int type = ByteArrayHelper.getInt(data, uniqueIDOffset + 4);
        let (Some(value), Some(type_)) = (
            opt_int(data, unique_id_offset),
            opt_int(data, unique_id_offset + 4),
        ) else {
            return;
        };

        // Java: Integer taskUniqueID = value == 0 || value > MAX_UNIQUE_ID ? null : Integer.valueOf(value);
        //       if (taskUniqueID != null) { switch (type) { ... } }
        if value == 0 || value > MAX_UNIQUE_ID {
            return;
        }

        match type_ {
            SUBPROJECT_TASKUNIQUEID0
            | SUBPROJECT_TASKUNIQUEID1
            | SUBPROJECT_TASKUNIQUEID2
            | SUBPROJECT_TASKUNIQUEID3
            | SUBPROJECT_TASKUNIQUEID4
            | SUBPROJECT_TASKUNIQUEID5
            | SUBPROJECT_TASKUNIQUEID6 => {
                self.task_sub_projects.insert(value, sp);
            }

            _ => {
                if value != 0 {
                    self.external_tasks.insert(value);
                    self.task_sub_projects.insert(value, sp);
                }
            }
        }
    }

/// Handles properties related to the appearance of project data.
    ///
    /// Corresponds to Java: `private void processViewPropertyData() throws IOException`
    fn process_view_property_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("Props")) { ... }
        if has_entry(&self.view_dir, "Props") {
            // Java: Props props = new Props12(m_inputStreamFactory.getInstance(m_viewDir, "Props"));
            let mut stream = self.input_stream_factory.get_instance(&self.view_dir, "Props")?;
            let props = Props12::new(&stream.to_bytes())?;

            // Java: byte[] data = props.getByteArray(PropsKey.FONT_BASES); if (data != null) { processBaseFonts(data); }
            if let Some(data) = props.get_byte_array(PropsKey::FontBases).map(<[u8]>::to_vec) {
                self.process_base_fonts(&data);
            }

            // Java: ProjectProperties properties = m_file.getProjectProperties();
            //       properties.setShowProjectSummaryTask(props.getBoolean(PropsKey.SHOW_PROJECT_SUMMARY_TASK));
            let show_project_summary_task = props.get_boolean(PropsKey::ShowProjectSummaryTask);
            if let Ok(mut file) = self.file.try_borrow_mut() {
                file.get_project_properties_mut()
                    .set_show_project_summary_task(show_project_summary_task);
            }
        }

        Ok(())
    }

/// Builds an index of "font number → font".
    ///
    /// Corresponds to Java: `private void processBaseFonts(byte[] data)`
    fn process_base_fonts(&mut self, data: &[u8]) {
        // Java: int offset = 0; int blockCount = ByteArrayHelper.getShort(data, 0); offset += 2;
        let Some(block_count) = opt_short(data, 0) else {
            return;
        };
        let mut offset: i32 = 2;

        for r#loop in 0..block_count {
            /*unknownAttribute = MPPUtility.getShort(data, offset);*/
            offset += 2;

            // Java: size = ByteArrayHelper.getShort(data, offset); offset += 2;
            let Some(size) = opt_short(data, offset) else {
                break;
            };
            offset += 2;

            // Java: name = MPPUtility.getUnicodeString(data, offset); offset += 64;
            let name = MPPUtility::get_unicode_string(data, usize::try_from(offset).unwrap_or(usize::MAX));
            offset += 64;

            // Java: if (!name.isEmpty()) { FontBase fontBase = new FontBase(Integer.valueOf(loop), name, size); m_fontBases.put(fontBase.getIndex(), fontBase); }
            if !name.is_empty() {
                let font_base = FontBase::new(Some(r#loop), name, size);
                if let Some(index) = font_base.get_index() {
                    self.font_bases.insert(index, font_base);
                }
            }
        }
    }

/// Builds a map of "task unique ID → FixedData block index".
    ///
    /// Corresponds to Java: `private TreeMap<Integer, Integer> createTaskMap(FieldMap fieldMap, FixedMeta taskFixedMeta, FixedData taskFixedData, FixedData taskFixed2Data, Var2Data taskVarData)`
    ///
/// A `None` value in the return value means "deleted task" (Java's `taskMap.put(key, null)`).
    fn create_task_map(
        field_map: &FieldMap,
        task_fixed_meta: &FixedMeta,
        task_fixed_data: &FixedData,
        task_fixed2_data: &FixedData,
        task_var_data: &Var2Data,
    ) -> BTreeMap<i32, Option<i32>> {
        let mut task_map: BTreeMap<i32, Option<i32>> = BTreeMap::new();
        let unique_id_offset = field_map.get_fixed_data_offset(&task_field(TaskField::UniqueId));
        let item_count = task_fixed_meta.get_adjusted_item_count();

        //
        // First three items are not tasks, so let's skip them.
        // Note we're working backwards: where we have duplicate tasks the later ones
        // appear to be the correct versions (https://github.com/joniles/mpxj/issues/152)
        //
        // Java: for (int loop = itemCount - 1; loop > 2; loop--)
        let mut r#loop = item_count - 1;
        while r#loop > 2 {
            let data = task_fixed_data.get_byte_array_value(r#loop);
            let data2 = task_fixed2_data.get_byte_array_value(r#loop);

            if let (Some(data), Some(_data2)) = (data, data2) {
                // Java: byte[] metaData = taskFixedMeta.getByteArrayValue(loop);
// int flags = ByteArrayHelper.getInt(metaData, 0); (Java throws an NPE when metaData is null)
                let flags = task_fixed_meta
                    .get_byte_array_value(r#loop)
                    .and_then(|meta_data| opt_int(meta_data, 0))
                    .unwrap_or(0);

                //
                // Check for the deleted task flag
                //
                if (flags & 0x02) != 0 {
                    // Project stores the deleted tasks unique id's into the fixed data as well
                    // and at least in one case the deleted task was listed twice in the list
                    // the second time with data with it causing a phantom task to be shown.
                    // See CalendarErrorPhantomTasks.mpp
                    //
                    // So let's add the unique id for the deleted task into the map so we don't
                    // accidentally include the task later.
                    //
                    // Java: uniqueID = ByteArrayHelper.getShort(data, TASK_UNIQUE_ID_FIXED_OFFSET);
                    //       taskMap.put(key, null);
                    let unique_id = opt_short(data, TASK_UNIQUE_ID_FIXED_OFFSET).unwrap_or(0);
                    task_map.insert(unique_id, None);
                } else {
                    //
                    // Do we have a null task?
                    //
                    if data.len() == NULL_TASK_BLOCK_SIZE {
                        // Java: uniqueID = ByteArrayHelper.getInt(data, TASK_UNIQUE_ID_FIXED_OFFSET);
                        //       if (!taskMap.containsKey(key)) { taskMap.put(key, Integer.valueOf(loop)); }
                        if let Some(unique_id) = opt_int(data, TASK_UNIQUE_ID_FIXED_OFFSET) {
                            task_map.entry(unique_id).or_insert(Some(r#loop));
                        }
                    } else {
                        //
                        // We apply a heuristic here - if we have more than 75% of the data, we assume
                        // the task is valid.
                        //
                        // Java: int maxSize = fieldMap.getMaxFixedDataSize(0);
                        //       if (maxSize == 0 || ((data.length * 100) / maxSize) > 75)
                        let max_size = field_map.get_max_fixed_data_size(0).unwrap_or(0);
// Java's int arithmetic does not overflow in this scenario; i64 is used to avoid a panic in debug builds.
                        let passes_heuristic = max_size == 0
                            || (data.len() as i64 * 100) / i64::from(max_size) > 75;

                        if passes_heuristic {
                            if let Some(unique_id) = opt_int(data, unique_id_offset) {
                                // Accept this task if it does not have a deleted unique ID or it has a
                                // deleted unique ID but it has var data entries
                                let has_var_data = !task_var_data
                                    .get_var_meta()
                                    .get_types(Some(unique_id))
                                    .is_empty();
                                if !task_map.contains_key(&unique_id) || has_var_data {
                                    // If a task is already in the map, overwrite it as long as flags is not 0x04.
                                    // TODO: gain a better understanding of why this works.
                                    if !task_map.contains_key(&unique_id) || (flags & 0x04) == 0 {
                                        task_map.insert(unique_id, Some(r#loop));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            r#loop -= 1;
        }

        task_map
    }

/// Builds a map of "resource unique ID → FixedData block index".
    ///
    /// Corresponds to Java: `private TreeMap<Integer, Integer> createResourceMap(FieldMap fieldMap, FixedMeta rscFixedMeta, FixedData rscFixedData)`
    fn create_resource_map(
        field_map: &FieldMap,
        rsc_fixed_meta: &FixedMeta,
        rsc_fixed_data: &FixedData,
    ) -> BTreeMap<i32, i32> {
        let mut resource_map: BTreeMap<i32, i32> = BTreeMap::new();
        let item_count = rsc_fixed_meta.get_adjusted_item_count();
        let max_fixed_data_size = field_map.get_max_fixed_data_size(0).unwrap_or(0);
        let unique_id_offset = 0;

        for r#loop in 0..item_count {
            // Java: byte[] data = rscFixedData.getByteArrayValue(loop);
            //       if (data == null || data.length < maxFixedDataSize) { continue; }
            let Some(data) = rsc_fixed_data.get_byte_array_value(r#loop) else {
                continue;
            };
            if (data.len() as i64) < i64::from(max_fixed_data_size) {
                continue;
            }

            // Java: Integer uniqueID = Integer.valueOf(ByteArrayHelper.getShort(data, uniqueIdOffset));
            //       if (!resourceMap.containsKey(uniqueID)) { resourceMap.put(uniqueID, Integer.valueOf(loop)); }
            if let Some(unique_id) = opt_short(data, unique_id_offset) {
                resource_map.entry(unique_id).or_insert(r#loop);
            }
        }

        resource_map
    }

/// Reads the calendar data.
    ///
/// The format is a 4-byte block header + 7 60-byte blocks (seven days a week), optionally followed by several
/// 64-byte exception blocks.
    ///
    /// Corresponds to Java: `private void processCalendarData() throws IOException`
    fn process_calendar_data(&mut self) -> MpxjResult<()> {
        // Java: CalendarFactory factory = new MPP12CalendarFactory(m_file);
        //       factory.processCalendarData(m_projectDir, m_projectProps, m_inputStreamFactory, m_resourceMap);
        let factory = MPP12CalendarFactory::new(Rc::clone(&self.file));
        factory.process_calendar_data(
            &self.project_dir,
            &self.project_props,
            &self.input_stream_factory,
            &mut self.resource_map,
        )
    }

/// Reads and organizes the task data.
    ///
/// The code below reads data from the MPP file in alphabetical order of [`Task`] setters; fields for which no corresponding
/// data can be obtained (computed at runtime, or whose location in the file has not yet been decoded) are handled
/// as skipped, following Java's comments.
    ///
/// Undecoded boolean properties in the task data `Props` section are skipped.
    ///
    /// Corresponds to Java: `private void processTaskData() throws IOException`
    fn process_task_data(&mut self) -> MpxjResult<()> {
        // Java: FieldMap fieldMap = new FieldMap12(m_file); fieldMap.createTaskFieldMap(m_projectProps);
        let mut field_map = FieldMap12::new(&self.file);
        field_map.create_task_field_map(&self.project_props);

        // Java: FieldMap enterpriseCustomFieldMap = new FieldMap12(m_file);
        //       enterpriseCustomFieldMap.createEnterpriseCustomFieldMap(m_projectProps, FieldTypeClass.TASK);
        let mut enterprise_custom_field_map = FieldMap12::new(&self.file);
        enterprise_custom_field_map
            .create_enterprise_custom_field_map(&self.project_props, FieldTypeClass::Task);

        // Java: DirectoryEntry taskDir = (DirectoryEntry) m_projectDir.getEntry("TBkndTask");
        let task_dir = self
            .project_dir
            .get_directory("TBkndTask")
            .ok_or_else(|| MpxjError::new("Entry not found: TBkndTask"))?;

        // Java: VarMeta taskVarMeta = new VarMeta12(new DocumentInputStream(((DocumentEntry) taskDir.getEntry("VarMeta"))));
        //       Var2Data taskVarData = new Var2Data(m_file, taskVarMeta, new DocumentInputStream(...("Var2Data")));
        let task_var_meta = Rc::new(VarMeta12::new(&read_document(&task_dir, "VarMeta")?)?);
        let task_var_data = Var2Data::new(
            &self.file,
            Rc::clone(&task_var_meta) as Rc<dyn VarMeta>,
            &read_document(&task_dir, "Var2Data")?,
        );

        // Java: FixedMeta taskFixedMeta = new FixedMeta(new DocumentInputStream(...("FixedMeta")), 47);
        //       FixedData taskFixedData = new FixedData(taskFixedMeta, new DocumentInputStream(...("FixedData")), 768, fieldMap.getMaxFixedDataSize(0));
        let task_fixed_meta = FixedMeta::with_item_size(&read_document(&task_dir, "FixedMeta")?, 47)?;
        let task_fixed_data = FixedData::with_limits(
            &task_fixed_meta,
            &read_document(&task_dir, "FixedData")?,
            768,
            field_map.get_max_fixed_data_size(0).unwrap_or(0),
        );

        // Java: FixedMeta taskFixed2Meta = new FixedMeta(new DocumentInputStream(...("Fixed2Meta")), 86);
        //       FixedData taskFixed2Data = new FixedData(taskFixed2Meta, new DocumentInputStream(...("Fixed2Data")));
        let task_fixed2_meta =
            FixedMeta::with_item_size(&read_document(&task_dir, "Fixed2Meta")?, 86)?;
        let task_fixed2_data =
            FixedData::new(&task_fixed2_meta, &read_document(&task_dir, "Fixed2Data")?);

        // Java: TreeMap<Integer, Integer> taskMap = createTaskMap(fieldMap, taskFixedMeta, taskFixedData, taskFixed2Data, taskVarData);
        let task_map = Self::create_task_map(
            &field_map,
            &task_fixed_meta,
            &task_fixed_data,
            &task_fixed2_data,
            &task_var_data,
        );

        // The var data may not contain all the tasks as tasks with no var data assigned will
        // not be saved in there. Most notably these are tasks with no name. So use the task map
        // which contains all the tasks.
        //
        // Java: Integer[] uniqueIdArray = taskMap.keySet().toArray(new Integer[0]);
        let unique_id_array: Vec<i32> = task_map.keys().copied().collect();

        let mut auto_wbs = true;
        let mut external_tasks: Vec<Rc<RefCell<Task>>> = Vec::new();
        let mut recurring_task_reader: Option<RecurringTaskReader> = None;
        let hyperlink_reader = HyperlinkReader::new();

        let id_offset = field_map.get_fixed_data_offset(&task_field(TaskField::Id));
        let max_fixed_data_size = field_map.get_max_fixed_data_size(0).unwrap_or(0).max(0) as usize;

        for unique_id in unique_id_array {
            // Java: offset = taskMap.get(uniqueID); if (!taskFixedData.isValidOffset(offset)) { continue; }
            let Some(offset) = task_map.get(&unique_id).copied().flatten() else {
                continue;
            };
            if !task_fixed_data.is_valid_offset(offset) {
                continue;
            }

            // Java: data = taskFixedData.getByteArrayValue(offset.intValue());
            //       Integer id = Integer.valueOf(ByteArrayHelper.getInt(data, fieldMap.getFixedDataOffset(TaskField.ID)));
            let Some(raw_data) = task_fixed_data.get_byte_array_value(offset) else {
                continue;
            };
            let id = opt_int(raw_data, id_offset);

            if raw_data.len() == NULL_TASK_BLOCK_SIZE {
                // Java: Integer nullTaskID = Integer.valueOf(ByteArrayHelper.getShort(data, TASK_ID_FIXED_OFFSET));
                //       if (!m_nullTaskOrder.containsKey(nullTaskID)) { task = m_file.addTask(); ... }
                if let Some(null_task_id) = opt_short(raw_data, TASK_ID_FIXED_OFFSET) {
                    if !self.null_task_order.contains_key(&null_task_id) {
                        let unique = opt_short(raw_data, TASK_UNIQUE_ID_FIXED_OFFSET).unwrap_or(0);
                        self.create_null_task(Some(null_task_id), unique);
                    }
                }
                continue;
            }

// Java: if (data.length < fieldMap.getMaxFixedDataSize(0)) { if (uniqueID == 0) { pad to maxFixedDataSize + 8 } else { continue; } }
            let padded_data;
            let data: &[u8] = if raw_data.len() < max_fixed_data_size {
                if unique_id != 0 {
                    continue;
                }
                let mut new_data = vec![0u8; max_fixed_data_size + 8];
                new_data[..raw_data.len()].copy_from_slice(raw_data);
                padded_data = new_data;
                &padded_data
            } else {
                raw_data
            };

            // Java: metaData = taskFixedMeta.getByteArrayValue(offset.intValue());
            //       metaData2 = taskFixed2Meta.getByteArrayValue(offset.intValue());
            //       byte[] data2 = taskFixed2Data.getByteArrayValue(offset.intValue());
            let meta_data = task_fixed_meta.get_byte_array_value(offset);
            let meta_data2 = task_fixed2_meta.get_byte_array_value(offset);
            let data2 = task_fixed2_data.get_byte_array_value(offset);

            // Java: byte[] recurringData = taskVarData.getByteArray(uniqueID, fieldMap.getVarDataKey(TaskField.RECURRING_DATA));
            let recurring_data = task_var_data.get_byte_array(
                Some(unique_id),
                field_map.get_var_data_key(&task_field(TaskField::RecurringData)),
            );

            // Java: Task temp = m_file.getTaskByID(id); if (temp != null) { ... }
            let existing = self
                .file
                .try_borrow()
                .ok()
                .and_then(|file| file.get_task_by_id(id));
            if let Some(temp) = existing {
                // Task with this id already exists... determine if this is the 'real' task by seeing
                // if this task has some var data. This is sort of hokey, but it's the best method i have
                // been able to see.
                if !task_var_meta.get_unique_identifier_set().contains(&unique_id) {
                    // Sometimes Project contains phantom tasks that coexist on the same id as a valid
                    // task. In this case don't want to include the phantom task. Seems to be a very rare case.
                    continue;
                }

                // Java: if (temp.getName() == null) { m_file.removeTask(temp); }
                // Ok, this looks valid. Remove the previous instance since it is most likely not a valid task.
                // At worst case this removes a task with an empty name.
                let name_is_null = temp
                    .try_borrow()
                    .map(|task| task.get_name().is_none())
                    .unwrap_or(false);
                if name_is_null {
                    if let Ok(mut file) = self.file.try_borrow_mut() {
                        file.remove_task(&temp);
                    }
                }
            }

            // Java: task = m_file.addTask();
            let Ok(task) = self.file.try_borrow_mut().map(|mut file| file.add_task()) else {
                continue;
            };

            {
                let Ok(mut task_ref) = task.try_borrow_mut() else {
                    continue;
                };

                // Java: task.disableEvents();
                task_ref.disable_events();

                // Java: fieldMap.populateContainer(FieldTypeClass.TASK, task, uniqueID, new byte[][]{data, data2}, taskVarData);
                field_map.populate_container(
                    FieldTypeClass::Task,
                    &mut *task_ref,
                    Some(unique_id),
                    &[Some(data), data2],
                    &task_var_data,
                );

                // Java: enterpriseCustomFieldMap.populateContainer(FieldTypeClass.TASK, task, uniqueID, null, taskVarData);
                enterprise_custom_field_map.populate_container(
                    FieldTypeClass::Task,
                    &mut *task_ref,
                    Some(unique_id),
                    &[],
                    &task_var_data,
                );

                // Java: task.enableEvents();
                task_ref.enable_events();

                // Java: task.setEffortDriven((metaData[11] & 0x10) != 0);
                task_ref.set_effort_driven(meta_byte(meta_data, 11) & 0x10 != 0);
                // Java: task.setEstimated(getDurationEstimated(ByteArrayHelper.getShort(data, fieldMap.getFixedDataOffset(TaskField.ACTUAL_DURATION_UNITS))));
                let actual_duration_units =
                    field_map.get_fixed_data_offset(&task_field(TaskField::ActualDurationUnits));
                let estimated = opt_short(data, actual_duration_units)
                    .map_or(false, Self::get_duration_estimated);
                task_ref.set_estimated(estimated);
                // Java: task.setExpanded(((metaData[12] & 0x02) == 0));
                task_ref.set_expanded(meta_byte(meta_data, 12) & 0x02 == 0);

                // Java: if (NumberHelper.getInt(task.getSubprojectTaskID()) != 0) { task.setExternalTask(true); externalTasks.add(task); }
                let task_unique_id = task_ref.get_unique_id();
                if number_get_int(task_ref.get_subproject_task_id()) != 0 {
                    task_ref.set_external_task(true);
                    external_tasks.push(Rc::clone(&task));
                }

                // Java: task.setFlag(1, (metaData[37] & 0x20) != 0); ... task.setFlag(20, (metaData[40] & 0x01) != 0);
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

                // Java: hyperlinkReader.read(task, taskVarData.getByteArray(uniqueID, fieldMap.getVarDataKey(TaskField.HYPERLINK_DATA)));
// The Rust side's `HyperlinkReader::read` only takes `&[u8]`, so it is skipped when the data does not exist
// (Java's read also returns immediately for null).
                if let Some(bytes) = task_var_data.get_byte_array(
                    Some(unique_id),
                    field_map.get_var_data_key(&task_field(TaskField::HyperlinkData)),
                ) {
                    hyperlink_reader.read(FieldTypeClass::Task, &mut *task_ref, &bytes);
                }

                // Java: task.setID(id);
                task_ref.set_id(id);

                // Java: task.setIgnoreResourceCalendar((metaData[10] & 0x02) != 0);
                task_ref.set_ignore_resource_calendar(meta_byte(meta_data, 10) & 0x02 != 0);
                // Java: task.setLevelAssignments((metaData[13] & 0x04) != 0);
                task_ref.set_level_assignments(meta_byte(meta_data, 13) & 0x04 != 0);
                // Java: task.setLevelingCanSplit((metaData[13] & 0x02) != 0);
                task_ref.set_leveling_can_split(meta_byte(meta_data, 13) & 0x02 != 0);
                // Java: task.setMarked((metaData[9] & 0x40) != 0);
                task_ref.set_marked(meta_byte(meta_data, 9) & 0x40 != 0);
                // Java: task.setMilestone((metaData[8] & 0x20) != 0);
                task_ref.set_milestone(meta_byte(meta_data, 8) & 0x20 != 0);

                // Java: task.setOutlineCode(1..10, getCustomFieldOutlineCodeValue(taskVarData, m_outlineCodeVarData, uniqueID, fieldMap.getVarDataKey(TaskField.OUTLINE_CODE1_INDEX..10)));
                for index in 1..=10 {
                    let field = outline_code_index_field(index);
                    let var_data_key = field_map.get_var_data_key(&task_field(field));
                    let value =
                        self.get_custom_field_outline_code_value(&task_var_data, Some(unique_id), var_data_key);
                    task_ref.set_outline_code(index, value);
                }

                // Java: task.setRollup((metaData[10] & 0x08) != 0);
                task_ref.set_rollup(meta_byte(meta_data, 10) & 0x08 != 0);
                // Java: task.setUniqueID(uniqueID);
                task_ref.set_unique_id(Some(unique_id));

                // Java: switch (task.getConstraintType()) { ... }
                match task_ref.get_constraint_type() {
                    //
                    // Adjust the start and finish dates if the task
                    // is constrained to start as late as possible.
                    //
                    Some(ConstraintType::AsLateAsPossible) => {
                        if LocalDateTimeHelper::compare(task_ref.get_start(), task_ref.get_late_start())
                            < 0
                        {
                            let late_start = task_ref.get_late_start();
                            task_ref.set_start(late_start);
                        }
                        if LocalDateTimeHelper::compare(
                            task_ref.get_finish(),
                            task_ref.get_late_finish(),
                        ) < 0
                        {
                            let late_finish = task_ref.get_late_finish();
                            task_ref.set_finish(late_finish);
                        }
                    }

                    Some(ConstraintType::StartNoLaterThan) | Some(ConstraintType::FinishNoLaterThan) => {
                        if LocalDateTimeHelper::compare(task_ref.get_finish(), task_ref.get_start())
                            < 0
                        {
                            let late_finish = task_ref.get_late_finish();
                            task_ref.set_finish(late_finish);
                        }
                    }

                    _ => {}
                }

                //
                // Retrieve task recurring data
                //
                if let Some(recurring_data) = recurring_data.as_deref() {
                    // Java: if (recurringTaskReader == null) { recurringTaskReader = new RecurringTaskReader(m_file.getProjectProperties()); }
                    if recurring_task_reader.is_none() {
                        recurring_task_reader = self
                            .file
                            .try_borrow()
                            .ok()
                            .map(|file| RecurringTaskReader::new(file.get_project_properties()));
                    }
                    if let Some(reader) = &recurring_task_reader {
                        // Java: recurringTaskReader.processRecurringTask(task, recurringData);
                        let recurring = reader.process_recurring_task(recurring_data);
                        *task_ref.add_recurring_task() = recurring;
                        // Java: task.setRecurring(true);
                        task_ref.set_recurring(true);
                    }
                }

                //
                // Configure the calendar
                //
                // Java: Integer calendarID = (Integer) task.getCachedValue(TaskField.CALENDAR_UNIQUE_ID);
                let calendar_id = task_ref
                    .get_cached_value(FieldKey::Task(TaskField::CalendarUniqueId))
                    .and_then(FieldValue::as_integer);
                if let Some(calendar_id) = calendar_id {
                    if calendar_id == -1 {
                        // Java: task.setCalendarUniqueID(null);
                        task_ref.set_calendar_unique_id(None);
                    } else {
                        // Java: ProjectCalendar calendar = m_file.getCalendarByUniqueID(calendarID);
                        //       if (calendar != null) { task.setCalendar(calendar); }
                        let calendar = self
                            .file
                            .try_borrow()
                            .ok()
                            .and_then(|file| file.get_calendar_by_unique_id(Some(calendar_id)));
                        if let Some(calendar) = calendar {
                            task_ref.set_calendar(Some(&calendar));
                        }
                    }
                }

                //
                // Set the subproject and external task flag
                //
                // Java: String sp = m_taskSubProjects.get(task.getUniqueID()); if (sp != null) { ... }
                let sp = task_unique_id
                    .and_then(|unique_id| self.task_sub_projects.get(&unique_id).cloned());
                if let Some(sp) = sp {
                    task_ref.set_subproject_file(Some(sp));
                    // Java: Integer subprojectTaskUniqueID = task.getSubprojectTaskUniqueID();
                    //       if (subprojectTaskUniqueID != null) { task.setSubprojectTaskUniqueID(Integer.valueOf(subprojectTaskUniqueID.intValue() & 0xFFFF)); }
                    if let Some(subproject_task_unique_id) = task_ref.get_subproject_task_unique_id() {
                        task_ref
                            .set_subproject_task_unique_id(Some(subproject_task_unique_id & 0xFFFF));
                    }
                }

                // Java: if (m_externalTasks.contains(task.getUniqueID()) && NumberHelper.getInt(task.getSubprojectTaskUniqueID()) != 0) { task.setExternalTask(...); }
                let is_external = task_unique_id.map_or(false, |id| self.external_tasks.contains(&id));
                if is_external && number_get_int(task_ref.get_subproject_task_unique_id()) != 0 {
                    // The condition preserves external tasks which no longer have an associated subproject filename
                    let value = task_unique_id.map_or(false, |id| self.external_tasks.contains(&id));
                    task_ref.set_external_task(value);
                }

                //
                // If we have a WBS value from the MPP file, don't autogenerate
                //
                if task_ref.get_wbs().is_some() {
                    auto_wbs = false;
                }

                //
                // Process any enterprise columns
                //
                Self::process_task_enterprise_columns(&mut task_ref, meta_data2);
            }

            // Unfortunately it looks like 'null' tasks sometimes make it through. So let's check for to see if we
            // need to mark this task as a null task after all.
            let is_null_task = {
                let Ok(task_ref) = task.try_borrow() else {
                    continue;
                };
                let epoch = epoch_date();
                let start = task_ref.get_start();
                let finish = task_ref.get_finish();
                task_ref.get_name().is_none()
                    && ((start.is_none() || start == epoch) || (finish.is_none() || finish == epoch))
            };

            if is_null_task {
                // Java: Integer nullTaskID = Integer.valueOf(ByteArrayHelper.getInt(data, TASK_ID_FIXED_OFFSET));
                let Some(null_task_id) = opt_int(data, TASK_ID_FIXED_OFFSET) else {
                    continue;
                };

                // Java: m_file.removeTask(task);
                if let Ok(mut file) = self.file.try_borrow_mut() {
                    file.remove_task(&task);
                }

                // Java: if (!m_nullTaskOrder.containsKey(nullTaskID)) { task = m_file.addTask(); ... }
                if !self.null_task_order.contains_key(&null_task_id) {
                    let unique = opt_int(data, TASK_UNIQUE_ID_FIXED_OFFSET).unwrap_or(0);
                    self.create_null_task(Some(null_task_id), unique);
                }
                continue;
            }

            // Java: if (data2 == null || data2.length < 24) { m_nullTaskOrder.put(task.getID(), task.getUniqueID()); }
            //       else { Long key = Long.valueOf(ByteArrayHelper.getLong(data2, 16)); m_taskOrder.put(key, task.getUniqueID()); }
            let (task_id, task_unique_id) = {
                let Ok(task_ref) = task.try_borrow() else {
                    continue;
                };
                (task_ref.get_id(), task_ref.get_unique_id())
            };

            if data2.map_or(true, |data2| data2.len() < 24) {
                if let (Some(task_id), Some(task_unique_id)) = (task_id, task_unique_id) {
                    self.null_task_order.insert(task_id, task_unique_id);
                }
            } else if let (Some(data2), Some(task_unique_id)) = (data2, task_unique_id) {
                if let Some(key) = opt_long(data2, 16) {
                    self.task_order.insert(key, task_unique_id);
                }
            }

            // Java: m_eventManager.fireTaskReadEvent(task);
            fire_task_read_event(&self.file, &task);
            //dumpUnknownData(task.getUniqueID().toString(), UNKNOWN_TASK_DATA, data);
        }

        //
        // Enable auto WBS if necessary
        //
        // Java: m_file.getProjectConfig().setAutoWBS(autoWBS);
        if let Ok(file) = self.file.try_borrow() {
            file.get_project_context()
                .borrow_mut()
                .get_project_config_mut()
                .set_auto_wbs(auto_wbs);
        }

        //
        // We have now read all of the task, so we are in a position
        // to perform post-processing to set up the relevant details
        // for each external task.
        //
        if !external_tasks.is_empty() {
            self.process_external_tasks(&mut external_tasks);
        }

        Ok(())
    }

/// MPP files occasionally contain duplicate IDs, causing the task structure to be reported incorrectly; this method uses the
/// sorted data in the file to rebuild the correct order of task IDs.
    ///
    /// Corresponds to Java: `private void postProcessTasks() throws MPXJException`
    fn post_process_tasks(&mut self) -> MpxjResult<()> {
        //
        // Renumber ID values using a large increment to allow
        // space for later inserts.
        //
        let mut task_map: BTreeMap<i32, i32> = BTreeMap::new();
        // Java: int nextIDIncrement = ((m_nullTaskOrder.size() / 1000) + 1) * 2000;
        let next_id_increment = ((self.null_task_order.len() as i32 / 1000) + 1) * 2000;
        // Java: int nextID = (m_file.getTaskByUniqueID(Integer.valueOf(0)) == null ? nextIDIncrement : 0);
        let no_zero_task = self
            .file
            .try_borrow()
            .ok()
            .and_then(|file| file.get_task_by_unique_id(Some(0)))
            .is_none();
        let mut next_id = if no_zero_task { next_id_increment } else { 0 };

        // Java: for (Map.Entry<Long, Integer> entry : m_taskOrder.entrySet()) { taskMap.put(Integer.valueOf(nextID), entry.getValue()); nextID += nextIDIncrement; }
        for value in self.task_order.values() {
            task_map.insert(next_id, *value);
// Java's int overflow wraps, and the Rust side uses wrapping_add to stay consistent without panicking.
            next_id = next_id.wrapping_add(next_id_increment);
        }

        //
        // Insert any null tasks into the correct location
        //
        let mut insertion_count: i32 = 0;
        let mut offset_map: HashMap<i32, i32> = HashMap::new();
        for (id_value, value) in &self.null_task_order {
            // Java: int baseTargetIdValue = (idValue - insertionCount) * nextIDIncrement;
            let base_target_id_value = id_value
                .wrapping_sub(insertion_count)
                .wrapping_mul(next_id_increment);
            let mut target_id_value = base_target_id_value;
            // Java: Integer previousOffset = offsetMap.get(previousOffsetKey); int offset = previousOffset == null ? 0 : previousOffset + 1;
            let previous_offset = offset_map.get(&base_target_id_value).copied();
            let mut offset = previous_offset.map_or(0, |previous| previous + 1);
            insertion_count += 1;

            // Java: while (taskMap.containsKey(Integer.valueOf(targetIDValue))) { ++offset; if (offset == nextIDIncrement) { throw ... } targetIDValue = baseTargetIdValue - (nextIDIncrement - offset); }
            while task_map.contains_key(&target_id_value) {
                offset += 1;
                if offset == next_id_increment {
                    return Err(MpxjError::new("Unable to fix task order"));
                }
                target_id_value = base_target_id_value - (next_id_increment - offset);
            }

            offset_map.insert(base_target_id_value, offset);
            task_map.insert(target_id_value, *value);
        }

        //
        // Finally, we can renumber the tasks
        //
        // Java: nextID = (m_file.getTaskByUniqueID(Integer.valueOf(0)) == null ? 1 : 0);
        let mut next_id = if no_zero_task { 1 } else { 0 };
        for unique_id in task_map.values() {
            // Java: Task task = m_file.getTaskByUniqueID(entry.getValue()); if (task != null) { task.setID(Integer.valueOf(nextID)); }
            let task = self
                .file
                .try_borrow()
                .ok()
                .and_then(|file| file.get_task_by_unique_id(Some(*unique_id)));
            if let Some(task) = task {
                if let Ok(mut task_ref) = task.try_borrow_mut() {
                    task_ref.set_id(Some(next_id));
                }
            }
            next_id += 1;
        }

        Ok(())
    }

/// Retrieves the task's enterprise column values.
    ///
    /// Corresponds to Java: `private void processTaskEnterpriseColumns(Task task, byte[] metaData2)`
    fn process_task_enterprise_columns(task: &mut Task, meta_data2: Option<&[u8]>) {
        if let Some(meta_data2) = meta_data2 {
            // Java: int bits = ByteArrayHelper.getInt(metaData2, 59);
            let bits = opt_int(meta_data2, 59).unwrap_or(0);

// Java: task.set(TaskField.ENTERPRISE_FLAG1, Boolean.valueOf((bits & 0x00001) != 0)); ... (20 in total)
            for index in 1..=20 {
                let field = enterprise_flag_field(index);
                let mask = 1i32 << (index - 1);
                task.set(
                    FieldKey::Task(field),
                    Some(FieldValue::Boolean((bits & mask) != 0)),
                );
            }
        }
    }

/// Retrieves the resource's enterprise column data.
    ///
    /// Corresponds to Java: `private void processResourceEnterpriseColumns(Resource resource, byte[] metaData2)`
    fn process_resource_enterprise_columns(resource: &mut Resource, meta_data2: Option<&[u8]>) {
        if let Some(meta_data2) = meta_data2 {
            // Java: int bits = ByteArrayHelper.getInt(metaData2, 16);
// resource.set(ResourceField.ENTERPRISE_FLAG1, Boolean.valueOf((bits & 0x00010) != 0)); ... (20 in total)
            let bits = opt_int(meta_data2, 16).unwrap_or(0);

            for index in 1..=20 {
                let field = resource_enterprise_flag_field(index);
                let mask = 1i32 << (index + 3);
                resource.set(
                    FieldKey::Resource(field),
                    Some(FieldValue::Boolean((bits & mask) != 0)),
                );
            }

            // Java: bits = ByteArrayHelper.getInt(metaData2, 32); resource.set(ResourceField.GENERIC, Boolean.valueOf((bits & 0x04000000) != 0));
            let bits = opt_int(meta_data2, 32).unwrap_or(0);
            resource.set(
                FieldKey::Resource(ResourceField::Generic),
                Some(FieldValue::Boolean((bits & 0x0400_0000) != 0)),
            );

            // Java: bits = MPPUtility.getByte(metaData2, 48); resource.set(ResourceField.ENTERPRISE, Boolean.valueOf((bits & 0x10) != 0));
            let bits = MPPUtility::get_byte(meta_data2, 48);
            resource.set(
                FieldKey::Resource(ResourceField::Enterprise),
                Some(FieldValue::Boolean((bits & 0x10) != 0)),
            );
        }
    }

/// The project file containing external tasks is not recorded per task; the MPP file stores the "current"
/// external task file (that is, the one last used). This method iterates over tasks marked as external
/// and associates the correct external project data (in the form of a subproject path) with them where possible.
    ///
    /// Corresponds to Java: `private void processExternalTasks(List<Task> externalTasks)`
    fn process_external_tasks(&self, external_tasks: &mut [Rc<RefCell<Task>>]) {
        //
        // Sort the list of tasks into ID order
        //
        // Java: Collections.sort(externalTasks);
        external_tasks.sort_by(|left, right| {
            let left = left.try_borrow();
            let right = right.try_borrow();
            match (left, right) {
                (Ok(left), Ok(right)) => left.compare_to(&right),
                _ => std::cmp::Ordering::Equal,
            }
        });

        //
        // Find any external tasks which don't have a sub project
        // object, and set this attribute using the most recent
        // value.
        //
        let mut current_sub_project: Option<String> = None;

        for current_task in external_tasks.iter() {
            let Ok(mut current_task) = current_task.try_borrow_mut() else {
                continue;
            };

            // Java: String sp = currentTask.getSubprojectFile(); if (sp == null) { ... } else { currentSubProject = sp; }
            if let Some(sp) = current_task.get_subproject_file() {
                current_sub_project = Some(sp);
            } else if let Some(sp) = &current_sub_project {
                current_task.set_subproject_file(Some(sp.clone()));
            }

            // Java: if (currentSubProject != null) { currentTask.setProject(currentSubProject); }
            if let Some(sp) = &current_sub_project {
                current_task.set_project(Some(sp.clone()));
            }
        }
    }

/// Reads and organizes the relation (constraint) data.
    ///
    /// Corresponds to Java: `private void processConstraintData() throws IOException`
    fn process_constraint_data(&mut self) -> MpxjResult<()> {
        // Java: ConstraintFactory factory = new ConstraintFactory();
        //       factory.process(m_projectDir, m_file, m_inputStreamFactory);
        ConstraintFactory.process(&self.project_dir, &self.file, &self.input_stream_factory)
    }

/// Reads and organizes the resource data.
    ///
    /// Corresponds to Java: `private void processResourceData() throws IOException`
    fn process_resource_data(&mut self) -> MpxjResult<()> {
        // Java: FieldMap fieldMap = new FieldMap12(m_file); fieldMap.createResourceFieldMap(m_projectProps);
        let mut field_map = FieldMap12::new(&self.file);
        field_map.create_resource_field_map(&self.project_props);

        // Java: FieldMap enterpriseCustomFieldMap = new FieldMap12(m_file);
        //       enterpriseCustomFieldMap.createEnterpriseCustomFieldMap(m_projectProps, FieldTypeClass.RESOURCE);
        let mut enterprise_custom_field_map = FieldMap12::new(&self.file);
        enterprise_custom_field_map
            .create_enterprise_custom_field_map(&self.project_props, FieldTypeClass::Resource);

        // Java: DirectoryEntry rscDir = (DirectoryEntry) m_projectDir.getEntry("TBkndRsc");
        let rsc_dir = self
            .project_dir
            .get_directory("TBkndRsc")
            .ok_or_else(|| MpxjError::new("Entry not found: TBkndRsc"))?;

        // Java: VarMeta rscVarMeta = new VarMeta12(new DocumentInputStream(...("VarMeta")));
        //       Var2Data rscVarData = new Var2Data(m_file, rscVarMeta, new DocumentInputStream(...("Var2Data")));
        let rsc_var_meta = Rc::new(VarMeta12::new(&read_document(&rsc_dir, "VarMeta")?)?);
        let rsc_var_data = Var2Data::new(
            &self.file,
            Rc::clone(&rsc_var_meta) as Rc<dyn VarMeta>,
            &read_document(&rsc_dir, "Var2Data")?,
        );

        // Java: FixedMeta rscFixedMeta = new FixedMeta(new DocumentInputStream(...("FixedMeta")), 37);
        //       FixedData rscFixedData = new FixedData(rscFixedMeta, m_inputStreamFactory.getInstance(rscDir, "FixedData"));
        let rsc_fixed_meta = FixedMeta::with_item_size(&read_document(&rsc_dir, "FixedMeta")?, 37)?;
        let mut fixed_data_stream = self.input_stream_factory.get_instance(&rsc_dir, "FixedData")?;
        let rsc_fixed_data = FixedData::new(&rsc_fixed_meta, &fixed_data_stream.to_bytes());

        // Java: FixedMeta rscFixed2Meta = new FixedMeta(new DocumentInputStream(...("Fixed2Meta")), 49);
        //       FixedData rscFixed2Data = new FixedData(rscFixed2Meta, m_inputStreamFactory.getInstance(rscDir, "Fixed2Data"));
        let rsc_fixed2_meta = FixedMeta::with_item_size(&read_document(&rsc_dir, "Fixed2Meta")?, 49)?;
        let mut fixed2_data_stream = self.input_stream_factory.get_instance(&rsc_dir, "Fixed2Data")?;
        let rsc_fixed2_data = FixedData::new(&rsc_fixed2_meta, &fixed2_data_stream.to_bytes());

        // Java: TreeMap<Integer, Integer> resourceMap = createResourceMap(fieldMap, rscFixedMeta, rscFixedData);
        let resource_map = Self::create_resource_map(&field_map, &rsc_fixed_meta, &rsc_fixed_data);

        // Java: Integer[] uniqueid = rscVarMeta.getUniqueIdentifierArray();
        let uniqueid = rsc_var_meta.get_unique_identifier_array();

        let hyperlink_reader = HyperlinkReader::new();

        for id in uniqueid {
            // Java: offset = resourceMap.get(id); if (offset == null) { continue; }
            let Some(offset) = resource_map.get(&id).copied() else {
                continue;
            };

            // Java: data = rscFixedData.getByteArrayValue(offset.intValue());
            //       byte[] metaData2 = rscFixed2Meta.getByteArrayValue(offset.intValue());
            //       byte[] data2 = rscFixed2Data.getByteArrayValue(offset.intValue());
            let data = rsc_fixed_data.get_byte_array_value(offset);
            let meta_data2 = rsc_fixed2_meta.get_byte_array_value(offset);
            let data2 = rsc_fixed2_data.get_byte_array_value(offset);

            // Java: resource = m_file.addResource();
            let Ok(resource) = self.file.try_borrow_mut().map(|mut file| file.add_resource()) else {
                continue;
            };

            {
                let Ok(mut resource_ref) = resource.try_borrow_mut() else {
                    continue;
                };

                // Java: resource.disableEvents();
                resource_ref.disable_events();

                // Java: fieldMap.populateContainer(FieldTypeClass.RESOURCE, resource, id, new byte[][]{data, data2}, rscVarData);
                field_map.populate_container(
                    FieldTypeClass::Resource,
                    &mut *resource_ref,
                    Some(id),
                    &[data, data2],
                    &rsc_var_data,
                );

                // Java: enterpriseCustomFieldMap.populateContainer(FieldTypeClass.RESOURCE, resource, id, null, rscVarData);
                enterprise_custom_field_map.populate_container(
                    FieldTypeClass::Resource,
                    &mut *resource_ref,
                    Some(id),
                    &[],
                    &rsc_var_data,
                );

                // Java: resource.enableEvents();
                resource_ref.enable_events();

                // Java: resource.setBudget((metaData2[8] & 0x20) != 0);
                resource_ref.set_budget(meta_byte(meta_data2, 8) & 0x20 != 0);

                // Java: resource.setGUID(MPPUtility.getGUID(data2, 0));
                resource_ref.set_guid(data2.and_then(|data2| MPPUtility::get_guid(data2, 0)));

                // Java: hyperlinkReader.read(resource, rscVarData.getByteArray(id, fieldMap.getVarDataKey(ResourceField.HYPERLINK_DATA)));
                if let Some(bytes) = rsc_var_data.get_byte_array(
                    Some(id),
                    field_map.get_var_data_key(&resource_field(ResourceField::HyperlinkData)),
                ) {
                    hyperlink_reader.read(FieldTypeClass::Resource, &mut *resource_ref, &bytes);
                }

                // Java: resource.setID(Integer.valueOf(ByteArrayHelper.getInt(data, 4)));
                resource_ref.set_id(data.and_then(|data| opt_int(data, 4)));

                // Java: resource.setOutlineCode(1..10, getCustomFieldOutlineCodeValue(rscVarData, m_outlineCodeVarData, id, fieldMap.getVarDataKey(ResourceField.OUTLINE_CODE1_INDEX..10)));
                for index in 1..=10 {
                    let field = resource_outline_code_index_field(index);
                    let var_data_key = field_map.get_var_data_key(&resource_field(field));
                    let value =
                        self.get_custom_field_outline_code_value(&rsc_var_data, Some(id), var_data_key);
                    resource_ref.set_outline_code_at(index, value);
                }

                // Java: resource.setUniqueID(id);
                ProjectEntityWithMutableUniqueID::set_unique_id(&mut *resource_ref, Some(id));

                // Java: metaData = rscFixedMeta.getByteArrayValue(offset.intValue());
                let meta_data = rsc_fixed_meta.get_byte_array_value(offset);

                // Java: resource.setFlag(1, (metaData[28] & 0x40) != 0); ... resource.setFlag(20, (metaData[31] & 0x01) != 0);
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

                //
                // Configure the resource calendar
                //
                // Java: resource.setCalendar(m_resourceMap.get(id));
                resource_ref.set_calendar(self.resource_map.get(&id));

                //
                // Process any enterprise columns
                //
                Self::process_resource_enterprise_columns(&mut resource_ref, meta_data2);

                //
                // Convert rate units
                //
                // Java:
                //   MPPUtility.convertRateFromHours(m_file, resource, ResourceField.STANDARD_RATE, ResourceField.STANDARD_RATE_UNITS);
                //   MPPUtility.convertRateFromHours(m_file, resource, ResourceField.OVERTIME_RATE, ResourceField.OVERTIME_RATE_UNITS);
// Note: the units field stores a `TimeUnit` as an integer, and [`MPPUtility::convert_rate_from_hours`] uses it
// to convert the "amount per hour" into the target unit via `RateHelper::convert_from_hours`.
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

                //
                // Process cost rate tables
                //
                // Java: CostRateTableFactory crt = new CostRateTableFactory(m_file);
                //       crt.process(resource, 0, rscVarData.getByteArray(id, fieldMap.getVarDataKey(ResourceField.COST_RATE_A))); ...
                let cost_rate_table_factory = CostRateTableFactory::new(Rc::clone(&self.file));
                let cost_rate_fields = [
                    ResourceField::CostRateA,
                    ResourceField::CostRateB,
                    ResourceField::CostRateC,
                    ResourceField::CostRateD,
                    ResourceField::CostRateE,
                ];
                for (index, field) in cost_rate_fields.iter().enumerate() {
                    let data = rsc_var_data
                        .get_byte_array(Some(id), field_map.get_var_data_key(&resource_field(*field)));
                    cost_rate_table_factory.process(&mut resource_ref, index as i32, data.as_deref());
                }

                //
                // Process availability table
                //
                // Java: AvailabilityFactory af = new AvailabilityFactory();
                //       af.process(resource, rscVarData.getByteArray(id, fieldMap.getVarDataKey(ResourceField.AVAILABILITY_DATA)));
// Note: `AvailabilityFactory::process` by convention returns "the rows to append to the availability table"; here they are
// written back via `Resource::add_availability` (Java appends directly to `resource.getAvailability()`).
                let availability_factory = AvailabilityFactory::new();
                let availability_data = rsc_var_data.get_byte_array(
                    Some(id),
                    field_map.get_var_data_key(&resource_field(ResourceField::AvailabilityData)),
                );
                let availability_entries =
                    availability_factory.process(&resource_ref, availability_data.as_deref());
                for entry in availability_entries {
                    resource_ref.add_availability(entry);
                }

                //
                // Process resource type
                //
                // Java: if ((metaData[9] & 0x02) != 0) { WORK } else if ((metaData2[8] & 0x10) != 0) { COST } else { MATERIAL }
                if meta_byte(meta_data, 9) & 0x02 != 0 {
                    resource_ref.set_type(ResourceType::Work);
                } else if meta_byte(meta_data2, 8) & 0x10 != 0 {
                    resource_ref.set_type(ResourceType::Cost);
                } else {
                    resource_ref.set_type(ResourceType::Material);
                }
            }

            // Java: m_eventManager.fireResourceReadEvent(resource);
            fire_resource_read_event(&self.file, &resource);
        }

        Ok(())
    }

/// Reads and organizes the resource assignment data.
    ///
    /// Corresponds to Java: `private void processAssignmentData() throws IOException`
    fn process_assignment_data(&mut self) -> MpxjResult<()> {
        // Java: FieldMap fieldMap = new FieldMap12(m_file); fieldMap.createAssignmentFieldMap(m_projectProps);
        let mut field_map = FieldMap12::new(&self.file);
        field_map.create_assignment_field_map(&self.project_props);

        // Java: FieldMap enterpriseCustomFieldMap = new FieldMap12(m_file);
        //       enterpriseCustomFieldMap.createEnterpriseCustomFieldMap(m_projectProps, FieldTypeClass.ASSIGNMENT);
        let mut enterprise_custom_field_map = FieldMap12::new(&self.file);
        enterprise_custom_field_map
            .create_enterprise_custom_field_map(&self.project_props, FieldTypeClass::Assignment);

        // Java: DirectoryEntry assnDir = (DirectoryEntry) m_projectDir.getEntry("TBkndAssn");
        let assn_dir = self
            .project_dir
            .get_directory("TBkndAssn")
            .ok_or_else(|| MpxjError::new("Entry not found: TBkndAssn"))?;

        // Java: VarMeta assnVarMeta = new VarMeta12(new DocumentInputStream(...("VarMeta")));
        //       Var2Data assnVarData = new Var2Data(m_file, assnVarMeta, new DocumentInputStream(...("Var2Data")));
        let assn_var_meta = Rc::new(VarMeta12::new(&read_document(&assn_dir, "VarMeta")?)?);
        let assn_var_data = Var2Data::new(
            &self.file,
            Rc::clone(&assn_var_meta) as Rc<dyn VarMeta>,
            &read_document(&assn_dir, "Var2Data")?,
        );

        // MSP 2007 seems to write 142 byte blocks, MSP 2010 writes 110 byte blocks
        // We need to identify any cases where the metadata count does not correctly identify the block size
        // Java: FixedMeta assnFixedMeta = new FixedMeta(new DocumentInputStream(...("FixedMeta")), 34);
        //       FixedData assnFixedData = new FixedData(assnFixedMeta, m_inputStreamFactory.getInstance(assnDir, "FixedData"));
        let assn_fixed_meta = FixedMeta::with_item_size(&read_document(&assn_dir, "FixedMeta")?, 34)?;
        let mut assn_fixed_data_stream = self.input_stream_factory.get_instance(&assn_dir, "FixedData")?;
        let assn_fixed_data = FixedData::new(&assn_fixed_meta, &assn_fixed_data_stream.to_bytes());

        // Java: FixedData assnFixedData2 = new FixedData(48, m_inputStreamFactory.getInstance(assnDir, "Fixed2Data"));
        let mut assn_fixed_data2_stream =
            self.input_stream_factory.get_instance(&assn_dir, "Fixed2Data")?;
        let assn_fixed_data2 = FixedData::with_fixed_size(48, &assn_fixed_data2_stream.to_bytes());

        // Java: ResourceAssignmentFactory factory = new ResourceAssignmentFactory();
        //       factory.process(m_file, fieldMap, enterpriseCustomFieldMap, null, m_reader.getUseRawTimephasedData(),
        //          assnVarMeta, assnVarData, assnFixedMeta, assnFixedData, assnFixedData2, assnFixedMeta.getAdjustedItemCount());
        ResourceAssignmentFactory.process(
            &self.file,
            &field_map,
            Some(&enterprise_custom_field_map),
            None,
            self.reader.get_use_raw_timephased_data(),
            &*assn_var_meta,
            &assn_var_data,
            &assn_fixed_meta,
            &assn_fixed_data,
            Some(&assn_fixed_data2),
            assn_fixed_meta.get_adjusted_item_count(),
        );

        Ok(())
    }

/// Determines whether the duration is confirmed.
    ///
    /// Corresponds to Java: `private boolean getDurationEstimated(int type)`
    fn get_duration_estimated(type_: i32) -> bool {
        (type_ & DURATION_CONFIRMED_MASK) != 0
    }

/// Reads the view data.
    ///
    /// Corresponds to Java: `private void processViewData() throws IOException`
    fn process_view_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CV_iew")) { ... }
        if !has_entry(&self.view_dir, "CV_iew") {
            return Ok(());
        }

        let dir = self
            .view_dir
            .get_directory("CV_iew")
            .ok_or_else(|| MpxjError::new("Entry not found: CV_iew"))?;

        // Java: VarMeta viewVarMeta = new VarMeta12(new DocumentInputStream(...("VarMeta")));
        //       Var2Data viewVarData = new Var2Data(m_file, viewVarMeta, new DocumentInputStream(...("Var2Data")));
        let view_var_meta = Rc::new(VarMeta12::new(&read_document(&dir, "VarMeta")?)?);
        let view_var_data = Var2Data::new(
            &self.file,
            Rc::clone(&view_var_meta) as Rc<dyn VarMeta>,
            &read_document(&dir, "Var2Data")?,
        );

        // Java: FixedMeta fixedMeta = new FixedMeta(new DocumentInputStream(...("FixedMeta")), 10);
        //       FixedData fixedData = new FixedData(138, m_inputStreamFactory.getInstance(dir, "FixedData"));
        let fixed_meta = FixedMeta::with_item_size(&read_document(&dir, "FixedMeta")?, 10)?;
        let mut fixed_data_stream = self.input_stream_factory.get_instance(&dir, "FixedData")?;
        let fixed_data = FixedData::with_fixed_size(138, &fixed_data_stream.to_bytes());

        // Java: int items = fixedMeta.getAdjustedItemCount();
        let items = fixed_meta.get_adjusted_item_count();
        // Java: ViewFactory factory = new ViewFactory12();
        let factory = ViewFactory12::new();

        // Java: int lastOffset = -1;
        let mut last_offset: i32 = -1;

        for r#loop in 0..items {
            // Java: byte[] fm = fixedMeta.getByteArrayValue(loop);
            let Some(fm) = fixed_meta.get_byte_array_value(r#loop) else {
                continue;
            };
            // Java: int offset = ByteArrayHelper.getShort(fm, 4);
            let Some(offset) = opt_short(fm, 4) else {
                continue;
            };

            if offset > last_offset {
                // Java: byte[] fd = fixedData.getByteArrayValue(fixedData.getIndexFromOffset(offset));
                let fd = fixed_data.get_byte_array_value(fixed_data.get_index_from_offset(offset));
                if let Some(fd) = fd {
                    // Java: view = factory.createView(m_file, fm, fd, viewVarData, m_fontBases);
                    //       m_file.getViews().add(view);
                    let view =
                        factory.create_view(&self.file, fm, fd, &view_var_data, &self.font_bases)?;
                    if let Ok(mut file) = self.file.try_borrow_mut() {
                        file.get_views_mut().add(view);
                    }
                }
                last_offset = offset;
            }
        }

        Ok(())
    }

/// Reads the table data.
    ///
    /// Corresponds to Java: `private void processTableData() throws IOException`
    fn process_table_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CTable")) { ... }
        if !has_entry(&self.view_dir, "CTable") {
            return Ok(());
        }

        let dir = self
            .view_dir
            .get_directory("CTable")
            .ok_or_else(|| MpxjError::new("Entry not found: CTable"))?;

        // Java: VarMeta varMeta = new VarMeta12(new DocumentInputStream(...("VarMeta")));
        //       Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(...("Var2Data")));
        let var_meta = Rc::new(VarMeta12::new(&read_document(&dir, "VarMeta")?)?);
        let var_data = Var2Data::new(
            &self.file,
            Rc::clone(&var_meta) as Rc<dyn VarMeta>,
            &read_document(&dir, "Var2Data")?,
        );

        // Java: FixedData fixedData = new FixedData(230, new DocumentInputStream(((DocumentEntry) dir.getEntry("FixedData"))));
        let fixed_data = FixedData::with_fixed_size(230, &read_document(&dir, "FixedData")?);

        // Java: TableContainer container = m_file.getTables();
        //       TableFactory factory = new TableFactory(TABLE_COLUMN_DATA_STANDARD, TABLE_COLUMN_DATA_ENTERPRISE, TABLE_COLUMN_DATA_BASELINE);
        let factory = TableFactory::new(
            Some(TABLE_COLUMN_DATA_STANDARD),
            Some(TABLE_COLUMN_DATA_ENTERPRISE),
            Some(TABLE_COLUMN_DATA_BASELINE),
        );

        // Java: int items = fixedData.getItemCount();
        let items = fixed_data.get_item_count();

        for r#loop in 0..items {
            // Java: byte[] data = fixedData.getByteArrayValue(loop);
            let Some(data) = fixed_data.get_byte_array_value(r#loop) else {
                continue;
            };

            // Java: Table table = factory.createTable(m_file, data, varMeta, varData); container.add(table);
            let table = factory.create_table(&self.file, data, &*var_meta, &var_data);
            if let Ok(mut file) = self.file.try_borrow_mut() {
                file.get_tables_mut().add(table);
            }
        }

        Ok(())
    }

/// Reads the filter definitions.
    ///
    /// Corresponds to Java: `private void processFilterData() throws IOException`
    fn process_filter_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CFilter")) { ... }
        if !has_entry(&self.view_dir, "CFilter") {
            return Ok(());
        }

        let dir = self
            .view_dir
            .get_directory("CFilter")
            .ok_or_else(|| MpxjError::new("Entry not found: CFilter"))?;

        // Java: FixedMeta fixedMeta = new FixedMeta(new DocumentInputStream(...("FixedMeta")), 10);
        //       FixedData fixedData = new FixedData(fixedMeta, m_inputStreamFactory.getInstance(dir, "FixedData"));
        let fixed_meta = FixedMeta::with_item_size(&read_document(&dir, "FixedMeta")?, 10)?;
        let mut fixed_data_stream = self.input_stream_factory.get_instance(&dir, "FixedData")?;
        let fixed_data = FixedData::new(&fixed_meta, &fixed_data_stream.to_bytes());

        // Java: VarMeta varMeta = new VarMeta12(new DocumentInputStream(...("VarMeta")));
        //       Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(...("Var2Data")));
        let var_meta = Rc::new(VarMeta12::new(&read_document(&dir, "VarMeta")?)?);
        let var_data = Var2Data::new(
            &self.file,
            Rc::clone(&var_meta) as Rc<dyn VarMeta>,
            &read_document(&dir, "Var2Data")?,
        );

        // Java: FilterReader reader = new FilterReader12(); reader.process(m_file, fixedData, varData);
        let reader = FilterReader12::new();
        reader.process(&self.file, &fixed_data, &var_data);

        Ok(())
    }

/// Reads the saved view states.
    ///
    /// Corresponds to Java: `private void processSavedViewState() throws IOException`
    fn process_saved_view_state(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CEdl")) { ... }
        if !has_entry(&self.view_dir, "CEdl") {
            return Ok(());
        }

        let dir = self
            .view_dir
            .get_directory("CEdl")
            .ok_or_else(|| MpxjError::new("Entry not found: CEdl"))?;

        // Java: VarMeta varMeta = new VarMeta12(new DocumentInputStream(...("VarMeta")));
        //       Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(...("Var2Data")));
        let var_meta = Rc::new(VarMeta12::new(&read_document(&dir, "VarMeta")?)?);
        let var_data = Var2Data::new(
            &self.file,
            Rc::clone(&var_meta) as Rc<dyn VarMeta>,
            &read_document(&dir, "Var2Data")?,
        );

        // Java: InputStream is = new DocumentInputStream(((DocumentEntry) dir.getEntry("FixedData")));
        //       byte[] fixedData = InputStreamHelper.readAvailable(is); is.close();
        let fixed_data = read_document(&dir, "FixedData")?;

        // Java: ViewStateReader reader = new ViewStateReader12(); reader.process(m_file, varData, fixedData);
        let reader = ViewStateReader12::new();
        reader.process(&self.file, &var_data, &fixed_data)
    }

/// Reads the group definitions.
    ///
    /// TODO: Doesn't work correctly with MPP12 files saved by Project 2007 and 2010
    ///
    /// Corresponds to Java: `private void processGroupData() throws IOException`
    fn process_group_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CGrouping")) { ... }
        if !has_entry(&self.view_dir, "CGrouping") {
            return Ok(());
        }

        let dir = self
            .view_dir
            .get_directory("CGrouping")
            .ok_or_else(|| MpxjError::new("Entry not found: CGrouping"))?;

        // Java: if (dir.hasEntry("FixedMeta")) { ... }
        if !has_entry(&dir, "FixedMeta") {
            return Ok(());
        }

        // Java: FixedMeta fixedMeta = new FixedMeta(new DocumentInputStream(...("FixedMeta")), 10);
        //       FixedData fixedData = new FixedData(fixedMeta, m_inputStreamFactory.getInstance(dir, "FixedData"));
        let fixed_meta = FixedMeta::with_item_size(&read_document(&dir, "FixedMeta")?, 10)?;
        let mut fixed_data_stream = self.input_stream_factory.get_instance(&dir, "FixedData")?;
        let fixed_data = FixedData::new(&fixed_meta, &fixed_data_stream.to_bytes());

        // Java: VarMeta varMeta = new VarMeta12(new DocumentInputStream(...("VarMeta")));
        //       Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(...("Var2Data")));
        let var_meta = Rc::new(VarMeta12::new(&read_document(&dir, "VarMeta")?)?);
        let var_data = Var2Data::new(
            &self.file,
            Rc::clone(&var_meta) as Rc<dyn VarMeta>,
            &read_document(&dir, "Var2Data")?,
        );

        // Java: GroupReader reader = new GroupReader12(); reader.process(m_file, fixedData, varData, m_fontBases);
        let reader = GroupReader12::new();
        reader.process(&self.file, &fixed_data, &var_data, &self.font_bases);

        Ok(())
    }

/// Reads the data link definitions.
    ///
    /// Corresponds to Java: `private void processDataLinks() throws IOException`
    fn process_data_links(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CEdl")) { ... }
        if !has_entry(&self.view_dir, "CEdl") {
            return Ok(());
        }

        let dir = self
            .view_dir
            .get_directory("CEdl")
            .ok_or_else(|| MpxjError::new("Entry not found: CEdl"))?;

        // Java: FixedMeta fixedMeta = new FixedMeta(new DocumentInputStream(...("FixedMeta")), 11);
        //       FixedData fixedData = new FixedData(fixedMeta, m_inputStreamFactory.getInstance(dir, "FixedData"));
        let fixed_meta = FixedMeta::with_item_size(&read_document(&dir, "FixedMeta")?, 11)?;
        let mut fixed_data_stream = self.input_stream_factory.get_instance(&dir, "FixedData")?;
        let fixed_data = FixedData::new(&fixed_meta, &fixed_data_stream.to_bytes());

        // Java: VarMeta varMeta = new VarMeta12(new DocumentInputStream(...("VarMeta")));
        //       Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(...("Var2Data")));
        let var_meta = Rc::new(VarMeta12::new(&read_document(&dir, "VarMeta")?)?);
        let var_data = Var2Data::new(
            &self.file,
            Rc::clone(&var_meta) as Rc<dyn VarMeta>,
            &read_document(&dir, "Var2Data")?,
        );

        // Java: DataLinkFactory factory = new DataLinkFactory(m_file, fixedData, varData); factory.process();
        let factory = DataLinkFactory::new(&self.file, fixed_data, var_data);
        factory.process()
    }

/// Gets a custom field value (4-arg version).
    ///
    /// Corresponds to Java: `private String getCustomFieldOutlineCodeValue(Var2Data varData, Var2Data outlineCodeVarData, Integer id, Integer type)`
    fn get_custom_field_outline_code_value(
        &self,
        var_data: &Var2Data,
        id: Option<i32>,
        type_: Option<i32>,
    ) -> Option<String> {
        // Java: int mask = varData.getShort(id, type);
        let mask = var_data.get_short(id, type_);

        if (mask & 0xFF00) != VALUE_LIST_MASK {
            // Java: result = outlineCodeVarData.getUnicodeString(Integer.valueOf(varData.getInt(id, 2, type)), OUTLINECODE_DATA);
            let unique_id = var_data.get_int_at(id, 2, type_);
            self.outline_code_var_data
                .get_unicode_string(Some(unique_id), Some(OUTLINECODE_DATA))
        } else {
            // Java: int uniqueId = varData.getInt(id, 2, type);
            //       CustomFieldValueItem item = m_file.getCustomFields().getCustomFieldValueItemByUniqueID(uniqueId);
            let unique_id = var_data.get_int_at(id, 2, type_);
            let item = self.custom_field_value_item(unique_id)?;

            // Java: Object value = item.getValue(); if (value instanceof String) { result = (String) value; }
            let (result, parent_unique_id) = {
                let Ok(item_ref) = item.try_borrow() else {
                    return None;
                };
                (
                    item_ref
                        .get_value()
                        .and_then(|value| value.as_string())
                        .map(str::to_owned),
                    item_ref.get_parent_unique_id(),
                )
            };

            let mut result = result;

            // Java: String result2 = getCustomFieldOutlineCodeValue(varData, outlineCodeVarData, item.getParentUniqueID());
            //       if (result != null && result2 != null && !result2.isEmpty()) { result = result2 + "." + result; }
            //
// Note: Java throws an NPE for `parentUniqueID == null`; the Rust side treats it as "no parent" (no recursion).
            let result2 = parent_unique_id
                .and_then(|parent| self.get_custom_field_outline_code_value_by_id(var_data, Some(parent)));

            if result.is_some() && result2.as_deref().is_some_and(|text| !text.is_empty()) {
                let prefix = result2.unwrap_or_default();
                let suffix = result.unwrap_or_default();
                result = Some(format!("{prefix}.{suffix}"));
            }

            result
        }
    }

/// Gets a custom field value (3-arg version, Java's same-named overload).
    ///
    /// Corresponds to Java: `private String getCustomFieldOutlineCodeValue(Var2Data varData, Var2Data outlineCodeVarData, Integer id)`
    fn get_custom_field_outline_code_value_by_id(
        &self,
        var_data: &Var2Data,
        id: Option<i32>,
    ) -> Option<String> {
        self.outline_code_value_by_id(var_data, id, 0)
    }

/// Recursive implementation of the 3-arg version (with a depth limit, see the file header difference table).
    fn outline_code_value_by_id(
        &self,
        var_data: &Var2Data,
        id: Option<i32>,
        depth: usize,
    ) -> Option<String> {
        if depth >= MAX_OUTLINE_CODE_DEPTH {
            return None;
        }

        // Java: int uniqueId = id.intValue(); if (uniqueId == 0) { return ""; }
// (Java throws an NPE for id == null; the Rust side returns None)
        let unique_id = id?;
        if unique_id == 0 {
            return Some(String::new());
        }

        // Java: CustomFieldValueItem item = m_file.getCustomFields().getCustomFieldValueItemByUniqueID(uniqueId);
        let item = self.custom_field_value_item(unique_id)?;

        let (mut result, parent_unique_id) = {
            let Ok(item_ref) = item.try_borrow() else {
                return None;
            };
            (
                item_ref
                    .get_value()
                    .and_then(|value| value.as_string())
                    .map(str::to_owned),
                item_ref.get_parent_unique_id(),
            )
        };

        // Java: if (result != null && !NumberHelper.equals(id, item.getParentUniqueID())) { ... }
        let equals_parent =
            NumberHelper::equals(id.map(f64::from), parent_unique_id.map(f64::from));
        if result.is_some() && !equals_parent {
            // Java: String result2 = getCustomFieldOutlineCodeValue(varData, outlineCodeVarData, item.getParentUniqueID());
            let result2 = parent_unique_id
                .and_then(|parent| self.outline_code_value_by_id(var_data, Some(parent), depth + 1));

            // Java: if (result2 != null && !result2.isEmpty()) { result = result2 + "." + result; }
            if let Some(result2) = result2.filter(|text| !text.is_empty()) {
                let suffix = result.unwrap_or_default();
                result = Some(format!("{result2}.{suffix}"));
            }
        }

        result
    }

/// Gets a custom field value item.
    ///
    /// Corresponds to Java: `m_file.getCustomFields().getCustomFieldValueItemByUniqueID(uniqueId)`
    ///
/// Rust-specific: `ProjectFile` / `CustomFieldContainer` are both `RefCell`, and on a borrow conflict it is treated as
/// "not found" (no such case in Java).
    fn custom_field_value_item(&self, unique_id: i32) -> Option<Rc<RefCell<CustomFieldValueItem>>> {
        let custom_fields = self.file.try_borrow().ok()?.get_custom_fields();
        let item = custom_fields
            .try_borrow()
            .ok()?
            .get_custom_field_value_item_by_unique_id(unique_id);
        item
    }

/// Creates an "empty task" and registers it in `m_nullTaskOrder`.
    ///
/// Corresponds to Java: the inline code that creates an empty task in `processTaskData`
    /// (`task = m_file.addTask(); task.setNull(true); task.setUniqueID(...); task.setID(...); m_nullTaskOrder.put(...)`).
    fn create_null_task(&mut self, null_task_id: Option<i32>, unique_id: i32) {
        let Ok(task) = self.file.try_borrow_mut().map(|mut file| file.add_task()) else {
            return;
        };

        if let Ok(mut task_ref) = task.try_borrow_mut() {
            // Java: task.setNull(true);
            task_ref.set_null(true);
            // Java: task.setUniqueID(Integer.valueOf(ByteArrayHelper.getShort(data, TASK_UNIQUE_ID_FIXED_OFFSET)));
            task_ref.set_unique_id(Some(unique_id));
            // Java: task.setID(nullTaskID);
            task_ref.set_id(null_task_id);
        }

        // Java: m_nullTaskOrder.put(task.getID(), task.getUniqueID());
        if let Some(null_task_id) = null_task_id {
            self.null_task_order.insert(null_task_id, unique_id);
        }
    }
}

// ---------------------------------------------------------------------------
// Bounds-safe read helpers (corresponding to Java's ByteArrayHelper / MPPUtility static methods)
// ---------------------------------------------------------------------------

/// Determines whether an entry with the given name exists under the storage.
///
/// Corresponds to Java: `DirectoryEntry.hasEntry(String)`
fn has_entry(dir: &DirectoryEntry, name: &str) -> bool {
    dir.get_entry(name).is_some()
}

/// Reads all bytes of a document stream.
///
/// Corresponds to Java: `new DocumentInputStream((DocumentEntry) dir.getEntry(name))` read to the end
/// (POI throws `FileNotFoundException` when the entry is not found; Rust returns `Err`).
fn read_document(dir: &DirectoryEntry, name: &str) -> MpxjResult<Vec<u8>> {
    match dir.get_document(name) {
        Some(document) => Ok(document.get_data().as_ref().clone()),
        None => Err(MpxjError::new(format!("Entry not found: {name}"))),
    }
}

/// Reads a 4-byte little-endian integer; returns `None` when out of bounds (Java throws `ArrayIndexOutOfBoundsException`).
fn opt_int(data: &[u8], offset: i32) -> Option<i32> {
    let start = usize::try_from(offset).ok()?;
    if start.checked_add(4)? > data.len() {
        return None;
    }
    Some(ByteArrayHelper::get_int(data, start))
}

/// Reads a 2-byte little-endian short (0..=65535); returns `None` when out of bounds.
fn opt_short(data: &[u8], offset: i32) -> Option<i32> {
    let start = usize::try_from(offset).ok()?;
    if start.checked_add(2)? > data.len() {
        return None;
    }
    Some(ByteArrayHelper::get_short(data, start))
}

/// Reads an 8-byte little-endian integer; returns `None` when out of bounds.
fn opt_long(data: &[u8], offset: i32) -> Option<i64> {
    let start = usize::try_from(offset).ok()?;
    if start.checked_add(8)? > data.len() {
        return None;
    }
    Some(ByteArrayHelper::get_long(data, start))
}

/// Reads a single byte; returns `None` when out of bounds.
fn opt_u8(data: &[u8], offset: i32) -> Option<u8> {
    let start = usize::try_from(offset).ok()?;
    data.get(start).copied()
}

/// Reads a 4-byte little-endian integer (with an `i64` offset, for offset calculations that may wrap); returns `None` when out of bounds.
fn read_int_i64(data: &[u8], offset: i64) -> Option<i32> {
    let start = usize::try_from(offset).ok()?;
    if start.checked_add(4)? > data.len() {
        return None;
    }
    Some(ByteArrayHelper::get_int(data, start))
}

/// Reads an "offset" field (4 bytes, taking the low 17 bits) and advances the cursor by 4 bytes.
///
/// Corresponds to Java: `XXXOffset = ByteArrayHelper.getInt(subProjData, offset) & 0x1FFFF; offset += 4;`
fn read_masked_offset(data: &[u8], offset: &mut i32) -> Option<i32> {
    let value = opt_int(data, *offset)? & 0x1FFFF;
    *offset += 4;
    Some(value)
}

/// Reads one byte from the metadata block; treated as 0 when the data is absent or out of bounds.
///
/// Corresponds to Java: `metaData[n]` (Java throws an exception for both `null` and out-of-bounds).
fn meta_byte(data: Option<&[u8]>, offset: usize) -> u8 {
    data.and_then(|data| data.get(offset)).copied().unwrap_or(0)
}

/// Gets a task outline code index field (`TaskField.OUTLINE_CODE1_INDEX` … `OUTLINE_CODE10_INDEX`).
fn outline_code_index_field(index: i32) -> TaskField {
    match index {
        1 => TaskField::OutlineCode1Index,
        2 => TaskField::OutlineCode2Index,
        3 => TaskField::OutlineCode3Index,
        4 => TaskField::OutlineCode4Index,
        5 => TaskField::OutlineCode5Index,
        6 => TaskField::OutlineCode6Index,
        7 => TaskField::OutlineCode7Index,
        8 => TaskField::OutlineCode8Index,
        9 => TaskField::OutlineCode9Index,
        _ => TaskField::OutlineCode10Index,
    }
}

/// Gets a resource outline code index field (`ResourceField.OUTLINE_CODE1_INDEX` … `OUTLINE_CODE10_INDEX`).
fn resource_outline_code_index_field(index: i32) -> ResourceField {
    match index {
        1 => ResourceField::OutlineCode1Index,
        2 => ResourceField::OutlineCode2Index,
        3 => ResourceField::OutlineCode3Index,
        4 => ResourceField::OutlineCode4Index,
        5 => ResourceField::OutlineCode5Index,
        6 => ResourceField::OutlineCode6Index,
        7 => ResourceField::OutlineCode7Index,
        8 => ResourceField::OutlineCode8Index,
        9 => ResourceField::OutlineCode9Index,
        _ => ResourceField::OutlineCode10Index,
    }
}

/// Gets a task enterprise flag field (index 1..=20).
fn enterprise_flag_field(index: usize) -> TaskField {
    match index {
        1 => TaskField::EnterpriseFlag1,
        2 => TaskField::EnterpriseFlag2,
        3 => TaskField::EnterpriseFlag3,
        4 => TaskField::EnterpriseFlag4,
        5 => TaskField::EnterpriseFlag5,
        6 => TaskField::EnterpriseFlag6,
        7 => TaskField::EnterpriseFlag7,
        8 => TaskField::EnterpriseFlag8,
        9 => TaskField::EnterpriseFlag9,
        10 => TaskField::EnterpriseFlag10,
        11 => TaskField::EnterpriseFlag11,
        12 => TaskField::EnterpriseFlag12,
        13 => TaskField::EnterpriseFlag13,
        14 => TaskField::EnterpriseFlag14,
        15 => TaskField::EnterpriseFlag15,
        16 => TaskField::EnterpriseFlag16,
        17 => TaskField::EnterpriseFlag17,
        18 => TaskField::EnterpriseFlag18,
        19 => TaskField::EnterpriseFlag19,
        _ => TaskField::EnterpriseFlag20,
    }
}

/// Gets a resource enterprise flag field (index 1..=20).
fn resource_enterprise_flag_field(index: usize) -> ResourceField {
    match index {
        1 => ResourceField::EnterpriseFlag1,
        2 => ResourceField::EnterpriseFlag2,
        3 => ResourceField::EnterpriseFlag3,
        4 => ResourceField::EnterpriseFlag4,
        5 => ResourceField::EnterpriseFlag5,
        6 => ResourceField::EnterpriseFlag6,
        7 => ResourceField::EnterpriseFlag7,
        8 => ResourceField::EnterpriseFlag8,
        9 => ResourceField::EnterpriseFlag9,
        10 => ResourceField::EnterpriseFlag10,
        11 => ResourceField::EnterpriseFlag11,
        12 => ResourceField::EnterpriseFlag12,
        13 => ResourceField::EnterpriseFlag13,
        14 => ResourceField::EnterpriseFlag14,
        15 => ResourceField::EnterpriseFlag15,
        16 => ResourceField::EnterpriseFlag16,
        17 => ResourceField::EnterpriseFlag17,
        18 => ResourceField::EnterpriseFlag18,
        19 => ResourceField::EnterpriseFlag19,
        _ => ResourceField::EnterpriseFlag20,
    }
}

/// Fires the "task read" event.
///
/// Corresponds to Java: `m_eventManager.fireTaskReadEvent(task)`
fn fire_task_read_event(file: &Rc<RefCell<ProjectFile>>, task: &Rc<RefCell<Task>>) {
    if let Ok(file_ref) = file.try_borrow() {
        let context = file_ref.get_project_context();
        context
            .borrow_mut()
            .get_event_manager_mut()
            .fire_task_read_event(task);
    }
}

/// Fires the "resource read" event.
///
/// Corresponds to Java: `m_eventManager.fireResourceReadEvent(resource)`
fn fire_resource_read_event(file: &Rc<RefCell<ProjectFile>>, resource: &Rc<RefCell<Resource>>) {
    if let Ok(file_ref) = file.try_borrow() {
        let context = file_ref.get_project_context();
        context
            .borrow_mut()
            .get_event_manager_mut()
            .fire_resource_read_event(resource);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    use cfb::CompoundFile;
    use chrono::Days;

    use crate::mpp::ole::PoifsFileSystem;
    use crate::mpp::props::Props;
    use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;

    // -----------------------------------------------------------------------
// Byte-level helpers needed to hand-build the MPP12 structure
    // -----------------------------------------------------------------------

/// Writes a stream into the compound document.
    fn write_stream(file: &mut CompoundFile<Cursor<Vec<u8>>>, path: &str, data: &[u8]) {
        file.create_stream(path).unwrap().write_all(data).unwrap();
    }

/// Builds a `Props12` stream (16-byte block header + several "length/key/unknown + data" records).
    fn props12(entries: &[(i32, Vec<u8>)]) -> Vec<u8> {
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&(entries.len() as u16).to_le_bytes());

        for (key, value) in entries {
            data.extend_from_slice(&(value.len() as i32).to_le_bytes());
            data.extend_from_slice(&key.to_le_bytes());
            data.extend_from_slice(&0i32.to_le_bytes());
            data.extend_from_slice(value);
            if value.len() % 2 != 0 {
                data.push(0);
            }
        }

        data
    }

/// Builds a `VarMeta12` stream: block header + each "unique ID(4) + offset(4) + type(2) + unknown(2)" entry.
    fn var_meta12(items: &[(i32, i32, i32)], data_size: i32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(items.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&data_size.to_le_bytes());

        for (unique_id, offset, type_) in items {
            data.extend_from_slice(&unique_id.to_le_bytes());
            data.extend_from_slice(&offset.to_le_bytes());
            data.extend_from_slice(&(*type_ as i16).to_le_bytes());
            data.extend_from_slice(&0i16.to_le_bytes());
        }

        data
    }

/// Builds a `FixedMeta` stream: 16-byte block header + several fixed-length records (bytes 4..8 point to the data body offset).
    fn fixed_meta(item_size: usize, data_offsets: &[i32]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(data_offsets.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());

        for offset in data_offsets {
            let mut item = vec![0u8; item_size];
            if item_size >= 8 {
                item[4..8].copy_from_slice(&offset.to_le_bytes());
            }
            data.extend_from_slice(&item);
        }

        data
    }

/// Builds an empty `FixedMeta` stream (0 records).
    fn empty_fixed_meta() -> Vec<u8> {
        fixed_meta(10, &[])
    }

/// Builds an empty `VarMeta12` stream (0 records).
    fn empty_var_meta12() -> Vec<u8> {
        var_meta12(&[], 0)
    }

/// Concatenates several variable-length data items into a `Var2Data` body and returns each item's offset.
    fn var2_block(values: &[Vec<u8>]) -> (Vec<u8>, Vec<i32>) {
        let mut data = Vec::new();
        let mut offsets = Vec::new();

        for value in values {
            offsets.push(data.len() as i32);
            data.extend_from_slice(&(value.len() as i32).to_le_bytes());
            data.extend_from_slice(value);
        }

        (data, offsets)
    }

/// UTF-16LE encoding + NUL terminator (the string form in variable-length blocks).
    fn utf16z(text: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in text.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00]);
        data
    }

/// Timestamp bytes (the low 2 bytes are tenths of a minute within the day, the high 2 bytes are days since 1983-12-31).
    fn timestamp_bytes(days: u16) -> [u8; 4] {
        let mut bytes = [0u8; 4];
        bytes[0..2].copy_from_slice(&0u16.to_le_bytes());
        bytes[2..4].copy_from_slice(&days.to_le_bytes());
        bytes
    }

/// The `days`-th day after the epoch date (consistent with the algorithm of `MPPUtility::get_timestamp`).
    fn epoch_plus_days(days: u64) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(1983, 12, 31)
            .and_then(|epoch| epoch.checked_add_days(Days::new(days)))
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .expect("valid epoch date")
    }

/// Offsets used by this test in the MPP12 default task field layout.
    struct TaskLayout {
        unique_id: i32,
        id: i32,
        start: i32,
        finish: i32,
        percent_complete: i32,
        name_key: i32,
    }

/// Gets task field offsets from the `FieldMap12` default table (also validating MPP12's known layout).
    fn task_layout() -> TaskLayout {
        let file = ProjectFile::new();
        let mut field_map = FieldMap12::new(&file);
// The default layout table is only written into the map in `createTaskFieldMap`: `Props` empty → use the FieldMap12 default table directly
        field_map.create_task_field_map(&Props::new());

        TaskLayout {
            unique_id: field_map.get_fixed_data_offset(&task_field(TaskField::UniqueId)),
            id: field_map.get_fixed_data_offset(&task_field(TaskField::Id)),
            start: field_map.get_fixed_data_offset(&task_field(TaskField::Start)),
            finish: field_map.get_fixed_data_offset(&task_field(TaskField::Finish)),
            percent_complete: field_map
                .get_fixed_data_offset(&task_field(TaskField::PercentComplete)),
            name_key: field_map
                .get_var_data_key(&task_field(TaskField::Name))
                .expect("NAME field should have a variable-length data key"),
        }
    }

/// Builds one task fixed-length data entry (with a fixed `length`).
    fn task_data(layout: &TaskLayout, unique_id: i32, id: i32, days: u16, percent: u16, length: usize) -> Vec<u8> {
        let mut data = vec![0u8; length];
        data[layout.unique_id as usize..layout.unique_id as usize + 4]
            .copy_from_slice(&unique_id.to_le_bytes());
        data[layout.id as usize..layout.id as usize + 4].copy_from_slice(&id.to_le_bytes());
        data[layout.start as usize..layout.start as usize + 4]
            .copy_from_slice(&timestamp_bytes(days));
        data[layout.finish as usize..layout.finish as usize + 4]
            .copy_from_slice(&timestamp_bytes(days + 10));
        data[layout.percent_complete as usize..layout.percent_complete as usize + 2]
            .copy_from_slice(&percent.to_le_bytes());
        data
    }

/// Byte size of the task fixed-length data block (200 bytes for each of the two tasks).
    const TASK_BLOCK_SIZE: usize = 200;

/// Switches used when building the MPP12 compound document.
    #[derive(Default)]
    struct FixtureOptions {
/// Whether to create the `TBkndTask` storage.
        include_task_dir: bool,
/// Whether the task block's VarMeta / FixedMeta use the "empty but valid" (0 records) version.
        empty_task_streams: bool,
    }

/// Builds a complete MPP12 compound document (with 2 tasks, 1 resource, and 1 relation).
    fn build_mpp12() -> Vec<u8> {
        build_mpp12_with(FixtureOptions {
            include_task_dir: true,
            empty_task_streams: false,
        })
    }

/// Builds the MPP12 compound document using the given switches.
    fn build_mpp12_with(options: FixtureOptions) -> Vec<u8> {
        let layout = task_layout();

// Task names (variable-length block, keys taken from the default field table)
        let (task_var_data, task_var_offsets) =
            var2_block(&[utf16z("Task 1"), utf16z("Task 2")]);
        let task_var_meta = var_meta12(
            &[
                (1, task_var_offsets[0], layout.name_key),
                (2, task_var_offsets[1], layout.name_key),
            ],
            task_var_data.len() as i32,
        );

// Task fixed-length block: 5 records (Java walks backwards from itemCount-1 and processes only the two with index > 2)
        let mut task1 = task_data(&layout, 1, 1, 100, 50, TASK_BLOCK_SIZE);
        let mut task2 = task_data(&layout, 2, 2, 110, 25, TASK_BLOCK_SIZE);
        task1[0..4].copy_from_slice(&1i32.to_le_bytes());
        task2[0..4].copy_from_slice(&2i32.to_le_bytes());

// Task fixed-length metadata: index 3 points to task 1, index 4 points to task 2
        let mut task_meta = fixed_meta(47, &[0, 0, 0, 0, TASK_BLOCK_SIZE as i32]);
// Byte 8 of record 3 is set → task 1's milestone is true
        let item3 = 16 + 47 * 3;
        task_meta[item3 + 8] = 0x20;

        let mut task_fixed_data = task1.clone();
        task_fixed_data.extend_from_slice(&task2);

// Task Fixed2 block: `data2[16..24]` is the task sort key
        let mut task2_record = vec![0u8; 86];
        task2_record[16..24].copy_from_slice(&2i64.to_le_bytes());
        let mut task1_record = vec![0u8; 86];
        task1_record[16..24].copy_from_slice(&1i64.to_le_bytes());

        let mut task_fixed2_meta = fixed_meta(86, &[0, 0, 0, 0, 86]);
// Byte 59 of record 3 (task 1) is 0x01 → ENTERPRISE_FLAG1 (enterprise columns are in the "metadata" block)
        task_fixed2_meta[16 + 86 * 3 + 59] = 0x01;
        let mut task_fixed2_data = task1_record;
        task_fixed2_data.extend_from_slice(&task2_record);

// Resources (1)
        let rsc_var_meta = var_meta12(&[(1, 0, 14)], 0);
        let mut rsc_fixed_data = vec![0u8; 120];
        rsc_fixed_data[0..2].copy_from_slice(&1u16.to_le_bytes());
        rsc_fixed_data[4..8].copy_from_slice(&1i32.to_le_bytes());

        let mut rsc_meta = fixed_meta(37, &[0]);
// metaData[9] & 0x02 → resource type WORK
        rsc_meta[16 + 9] = 0x02;

// The resource's Fixed2Meta entry (Java reads the budget flag and enterprise columns from the "metadata" block)
        let mut rsc_meta2 = fixed_meta(49, &[0]);
        // metaData2[8] & 0x20 → budget
        rsc_meta2[16 + 8] = 0x20;
        // metaData2[16] & 0x00010 → ENTERPRISE_FLAG1
        rsc_meta2[16 + 16] = 0x10;
// metaData2[32] & 0x04000000 → GENERIC (0x04 in the 3rd byte)
        rsc_meta2[16 + 35] = 0x04;
        // metaData2[48] & 0x10 → ENTERPRISE
        rsc_meta2[16 + 48] = 0x10;

        let mut rsc_fixed2_data = vec![0u8; 100];
// Non-zero GUID (`MPPUtility::get_guid` returns None for all zeros)
        rsc_fixed2_data[0..16].copy_from_slice(&[
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F, 0x10,
        ]);

// Relation: 1 → 2, StartStart, lag 1 day
        let mut cons_meta = Vec::new();
        cons_meta.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        cons_meta.extend_from_slice(&0i32.to_le_bytes());
        cons_meta.extend_from_slice(&1i32.to_le_bytes());
        cons_meta.extend_from_slice(&0i32.to_le_bytes());
        let mut cons_item = vec![0u8; 10];
        cons_item[4..8].copy_from_slice(&0i32.to_le_bytes());
        cons_meta.extend_from_slice(&cons_item);

        let mut cons_data = vec![0u8; 20];
        cons_data[0..4].copy_from_slice(&77i32.to_le_bytes());
        cons_data[4..8].copy_from_slice(&1i32.to_le_bytes());
        cons_data[8..12].copy_from_slice(&2i32.to_le_bytes());
        cons_data[12..14].copy_from_slice(&3u16.to_le_bytes());
        cons_data[14..16].copy_from_slice(&7u16.to_le_bytes());
        cons_data[16..20].copy_from_slice(&4800i32.to_le_bytes());

// ---- Assemble the compound document ----
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).unwrap();

        write_stream(&mut file, "/Props12", &props12(&[]));

        file.create_storage("/   112").unwrap();
        file.create_storage("/   212").unwrap();
        write_stream(&mut file, "/   112/Props", &props12(&[]));

        file.create_storage("/   112/TBkndOutlCode").unwrap();
        write_stream(&mut file, "/   112/TBkndOutlCode/VarMeta", &empty_var_meta12());
        write_stream(&mut file, "/   112/TBkndOutlCode/Var2Data", &[]);
        write_stream(&mut file, "/   112/TBkndOutlCode/FixedMeta", &empty_fixed_meta());
        write_stream(&mut file, "/   112/TBkndOutlCode/FixedData", &[]);
        write_stream(&mut file, "/   112/TBkndOutlCode/Fixed2Meta", &empty_fixed_meta());
        write_stream(&mut file, "/   112/TBkndOutlCode/Fixed2Data", &[]);

        if options.include_task_dir {
            file.create_storage("/   112/TBkndTask").unwrap();
            if options.empty_task_streams {
                write_stream(&mut file, "/   112/TBkndTask/VarMeta", &empty_var_meta12());
                write_stream(&mut file, "/   112/TBkndTask/Var2Data", &[]);
                write_stream(&mut file, "/   112/TBkndTask/FixedMeta", &fixed_meta(47, &[]));
                write_stream(&mut file, "/   112/TBkndTask/FixedData", &[]);
                write_stream(&mut file, "/   112/TBkndTask/Fixed2Meta", &fixed_meta(86, &[]));
                write_stream(&mut file, "/   112/TBkndTask/Fixed2Data", &[]);
            } else {
                write_stream(&mut file, "/   112/TBkndTask/VarMeta", &task_var_meta);
                write_stream(&mut file, "/   112/TBkndTask/Var2Data", &task_var_data);
                write_stream(&mut file, "/   112/TBkndTask/FixedMeta", &task_meta);
                write_stream(&mut file, "/   112/TBkndTask/FixedData", &task_fixed_data);
                write_stream(&mut file, "/   112/TBkndTask/Fixed2Meta", &task_fixed2_meta);
                write_stream(&mut file, "/   112/TBkndTask/Fixed2Data", &task_fixed2_data);
            }
        }

        file.create_storage("/   112/TBkndRsc").unwrap();
        write_stream(&mut file, "/   112/TBkndRsc/VarMeta", &rsc_var_meta);
        write_stream(&mut file, "/   112/TBkndRsc/Var2Data", &[]);
        write_stream(&mut file, "/   112/TBkndRsc/FixedMeta", &rsc_meta);
        write_stream(&mut file, "/   112/TBkndRsc/FixedData", &rsc_fixed_data);
        write_stream(&mut file, "/   112/TBkndRsc/Fixed2Meta", &rsc_meta2);
        write_stream(&mut file, "/   112/TBkndRsc/Fixed2Data", &rsc_fixed2_data);

        file.create_storage("/   112/TBkndAssn").unwrap();
        write_stream(&mut file, "/   112/TBkndAssn/VarMeta", &empty_var_meta12());
        write_stream(&mut file, "/   112/TBkndAssn/Var2Data", &[]);
        write_stream(&mut file, "/   112/TBkndAssn/FixedMeta", &empty_fixed_meta());
        write_stream(&mut file, "/   112/TBkndAssn/FixedData", &[]);
        write_stream(&mut file, "/   112/TBkndAssn/Fixed2Data", &[]);

        file.create_storage("/   112/TBkndCal").unwrap();
        write_stream(&mut file, "/   112/TBkndCal/VarMeta", &empty_var_meta12());
        write_stream(&mut file, "/   112/TBkndCal/Var2Data", &[]);
        write_stream(&mut file, "/   112/TBkndCal/FixedMeta", &empty_fixed_meta());
        write_stream(&mut file, "/   112/TBkndCal/FixedData", &[]);

        file.create_storage("/   112/TBkndCons").unwrap();
        write_stream(&mut file, "/   112/TBkndCons/FixedMeta", &cons_meta);
        write_stream(&mut file, "/   112/TBkndCons/FixedData", &cons_data);

        file.into_inner().into_inner()
    }

/// Opens the hand-built compound document and returns the root storage.
    fn open(bytes: &[u8]) -> PoifsFileSystem {
        PoifsFileSystem::new(Cursor::new(bytes.to_vec())).expect("compound document can be opened")
    }

    // -----------------------------------------------------------------------
// Tests
    // -----------------------------------------------------------------------

/// Key offsets of the MPP12 default task field layout (pinning down the Java-side data layout).
    #[test]
    fn default_task_layout_matches_mpp12() {
        let layout = task_layout();

        assert_eq!(layout.unique_id, 0);
        assert_eq!(layout.id, 4);
        assert_eq!(layout.start, 88);
        assert_eq!(layout.finish, 92);
        assert_eq!(layout.percent_complete, 122);
        assert_eq!(layout.name_key, 14);
    }

/// Drives a full parse: key fields for tasks / resources / calendars / relations can all be retrieved.
    #[test]
    fn parses_tasks_resources_relations_and_calendar() {
        let bytes = build_mpp12();
        let fs = open(&bytes);
        let root = fs.get_root();

        let file = ProjectFile::new();
        let reader = MPPReader::default();
        MPP12Reader::new()
            .process(&reader, &file, &root)
            .expect("MPP12 parsed successfully");

// ---- Project properties ----
        {
            let file_ref = file.borrow();
            assert_eq!(file_ref.get_project_properties().get_mpp_file_type(), Some(12));
        }

// ---- Tasks ----
        let tasks = file.borrow().get_tasks().as_slice().to_vec();
        assert_eq!(tasks.len(), 2, "should have 2 tasks");

        let task1 = file
            .borrow()
            .get_task_by_unique_id(Some(1))
            .expect("task 1");
        {
            let task1 = task1.borrow();
            assert_eq!(task1.get_name(), Some("Task 1".to_owned()));
            assert_eq!(task1.get_unique_id(), Some(1));
// postProcessTasks reorders IDs based on data2's sort key: task 1 gets ID 1
            assert_eq!(task1.get_id(), Some(1));
            assert_eq!(task1.get_percentage_complete(), Some(50.0));
            assert_eq!(task1.get_start(), Some(epoch_plus_days(100)));
            assert_eq!(task1.get_finish(), Some(epoch_plus_days(110)));
            // metaData[8] & 0x20
            assert!(task1.get_milestone());
// metaData2[59] & 0x01 (enterprise columns are read from the task Fixed2Meta block)
            assert_eq!(
                task1.get(FieldKey::Task(TaskField::EnterpriseFlag1)),
                Some(FieldValue::Boolean(true))
            );
// Custom field bits in the field table (false when not read)
            assert!(!task1.get_external_task());
        }

        let task2 = file
            .borrow()
            .get_task_by_unique_id(Some(2))
            .expect("task 2");
        {
            let task2 = task2.borrow();
            assert_eq!(task2.get_name(), Some("Task 2".to_owned()));
            assert_eq!(task2.get_id(), Some(2));
            assert_eq!(task2.get_percentage_complete(), Some(25.0));
            assert_eq!(task2.get_start(), Some(epoch_plus_days(110)));
            assert!(!task2.get_milestone());
            assert_eq!(
                task2.get(FieldKey::Task(TaskField::EnterpriseFlag1)),
                Some(FieldValue::Boolean(false))
            );
        }

// ---- Resources ----
        let resources = file.borrow().get_resources().as_slice().to_vec();
        assert_eq!(resources.len(), 1, "should have 1 resource");
        {
            let resource = resources[0].borrow();
            assert_eq!(resource.get_unique_id(), Some(1));
            assert_eq!(resource.get_id(), Some(1));
            // metaData2[8] & 0x20
            assert!(resource.get_budget());
            // metaData[9] & 0x02
            assert_eq!(resource.get_type(), ResourceType::Work);
            // data2[0..16]
            assert!(resource.get_guid().is_some());
// Enterprise columns in metaData2
            assert_eq!(
                resource.get(FieldKey::Resource(ResourceField::EnterpriseFlag1)),
                Some(FieldValue::Boolean(true))
            );
            assert!(resource.get_generic());
            assert!(resource.get_enterprise());
        }

// ---- Calendars ----
        assert!(
            file.borrow().get_default_calendar().is_some(),
            "project default calendar should exist after parsing"
        );

// ---- Relations ----
        let relations = file.borrow().get_relations().as_slice().to_vec();
        assert_eq!(relations.len(), 1, "should have 1 relation");
        {
            let relation = relations[0].borrow();
            assert_eq!(relation.get_unique_id(), Some(77));
        }
        assert_eq!(task2.borrow().get_predecessors().len(), 1);

// ---- All blocks except the missing TBkndCons are provided ----
        let ignored = file.borrow().get_ignored_errors().to_vec();
        assert!(
            ignored.iter().all(|error| !error.message().contains("TBkndCons")),
            "this test provides TBkndCons, so this ignored error should not be recorded"
        );
    }

/// Parses the given compound document bytes (reused by "no panic" assertions).
    fn parse(bytes: &[u8]) -> MpxjResult<()> {
        let fs = open(bytes);
        let root = fs.get_root();
        let file = ProjectFile::new();
        let reader = MPPReader::default();
        MPP12Reader::new().process(&reader, &file, &root)
    }

/// Builds a compound document containing only `Props12`.
    fn bytes_with_props12_only(props12_bytes: &[u8]) -> Vec<u8> {
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        write_stream(&mut file, "/Props12", props12_bytes);
        file.into_inner().into_inner()
    }

/// Returns `Err` without panicking when the `Props12` stream is missing.
    #[test]
    fn missing_props12_stream_returns_error() {
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        file.create_storage("/   112").unwrap();
        file.create_storage("/   212").unwrap();

        let result = parse(&file.into_inner().into_inner());

        assert!(result.is_err(), "missing Props12 should return Err");
        assert_eq!(
            result.err().map(|error| error.message().to_owned()),
            Some("Entry not found: Props12".to_owned())
        );
    }

/// Returns `Err` without panicking when the `   112` project storage is missing.
    #[test]
    fn missing_project_directory_returns_error() {
        let result = parse(&bytes_with_props12_only(&props12(&[])));

        assert!(result.is_err(), "missing    112 should return Err");
    }

/// Truncated data does not panic: it returns `Err` (the block header cannot be fully read) or degrades to "no data".
    #[test]
    fn truncated_data_does_not_panic() {
// 1) Props12 has only 8 bytes (not even the full 16-byte block header can be read)
        assert!(parse(&bytes_with_props12_only(&[0u8; 8])).is_err());

// 2) The outline code block's VarMeta is empty and FixedMeta has only 4 bytes (the block header cannot be fully read)
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        write_stream(&mut file, "/Props12", &props12(&[]));
        file.create_storage("/   112").unwrap();
        file.create_storage("/   212").unwrap();
        write_stream(&mut file, "/   112/Props", &props12(&[]));
        file.create_storage("/   112/TBkndOutlCode").unwrap();
        write_stream(&mut file, "/   112/TBkndOutlCode/VarMeta", &[]);
        write_stream(&mut file, "/   112/TBkndOutlCode/Var2Data", &[]);
        write_stream(&mut file, "/   112/TBkndOutlCode/FixedMeta", &[0u8; 4]);
        write_stream(&mut file, "/   112/TBkndOutlCode/FixedData", &[]);
        write_stream(&mut file, "/   112/TBkndOutlCode/Fixed2Meta", &[0u8; 4]);
        write_stream(&mut file, "/   112/TBkndOutlCode/Fixed2Data", &[]);
// Truncated FixedMeta (insufficient header) → Err, no panic
        assert!(parse(&file.into_inner().into_inner()).is_err());
    }

/// Parsing succeeds when the task block is "empty but valid" (0 records), and no tasks are produced.
    #[test]
    fn empty_task_streams_parse_as_no_tasks() {
        let bytes = build_mpp12_with(FixtureOptions {
            include_task_dir: true,
            empty_task_streams: true,
        });

        parse(&bytes).expect("empty task block should parse successfully");

        let fs = open(&bytes);
        let root = fs.get_root();
        let file = ProjectFile::new();
        let reader = MPPReader::default();
        MPP12Reader::new()
            .process(&reader, &file, &root)
            .expect("empty task block should parse successfully");

        assert!(file.borrow().get_tasks().as_slice().is_empty());
// Resources and calendars are unaffected
        assert_eq!(file.borrow().get_resources().as_slice().len(), 1);
        assert!(file.borrow().get_default_calendar().is_some());
// A relation needs two tasks to be established, so there are no relations at this point
        assert!(file.borrow().get_relations().as_slice().is_empty());
    }

/// Returns `Err` without panicking when the `TBkndTask` storage is missing.
    #[test]
    fn missing_task_directory_returns_error() {
        let bytes = build_mpp12_with(FixtureOptions {
            include_task_dir: false,
            empty_task_streams: false,
        });

        let result = parse(&bytes);

        assert!(result.is_err(), "missing TBkndTask should return Err");
        assert_eq!(
            result.err().map(|error| error.message().to_owned()),
            Some("Entry not found: TBkndTask".to_owned())
        );
    }
}
