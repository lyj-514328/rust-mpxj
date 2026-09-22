//! Corresponds to Java: `org/mpxj/mpp/MPP8Reader.java`
//!
//! Reader for Microsoft Project 98 (MPP8) files: reads project properties, calendars,
//! resources, tasks, constraints, and assignment data, as well as (optional) presentation-layer data such as views / tables, writing them into [`ProjectFile`].
//!
//! # The actual `MPPVariantReader` signature used
//!
//! Exactly matches the trait already implemented in [`crate::mpp::mpp_variant_reader`] (this file implements that contract):
//!
//! ```ignore
//! pub trait MPPVariantReader {
//!     fn process(
//!         &self,
//!         reader: &MPPReader,
//!         file: &Rc<RefCell<ProjectFile>>,
//!         root: &DirectoryEntry,
//!     ) -> MpxjResult<()>;
//! }
//! ```
//!
//! That is, Java's `void process(MPPReader reader, ProjectFile file, DirectoryEntry root)` →
//! `&MPPReader` / `&Rc<RefCell<ProjectFile>>` / `&DirectoryEntry` (matching the comparison table given in the trait file header).
//!
//! # Java instance state → Rust private state struct
//!
//! Java's `MPP8Reader` uses "no-argument constructor + assigning `m_file` / `m_root` /
//! `m_projectDir` / `m_viewDir` / `m_projectProps` / `m_calendarMap` to instance fields inside `populateMemberData`,
//! and clearing them with `clearMemberData` when `process` ends" mutable instance state. The trait's `process` is fixed to `&self`
//! (see the three implementation styles given in the trait file header); this file adopts **style (2)** among them: wrapping this mutable state in
//! the private struct [`MPP8ReaderState`]; `process` constructs it, calls the private methods, and finally clears it.
//! This way each private method (`populateMemberData` / `processCalendarData` / …) stays 1:1 with Java method by method,
//! and no interior mutability needs to be introduced on [`MPP8Reader`].
//!
//! # Differences from Java (summary of trade-offs)
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `new DocumentInputStream((DocumentEntry) dir.getEntry(name))` | `dir.get_document(name)` → `get_data()` → `&[u8]` | Per this project's convention, `InputStream` is always replaced with `&[u8]` |
//! | Throws `FileNotFoundException` when `root.getEntry("   1")` is not found | Returns `None` → [`MpxjError`] | No panic; if `m_viewDir` is missing, presentation-layer data is skipped (Java would NPE) |
//! | Field `m_eventManager` | Not stored; instead obtained on demand via `ProjectContext` (`with_event_manager`) | `ProjectFile::get_event_manager` returns `Ref`, while `fire*` needs `&mut`, so it cannot be held long-term |
//! | `m_calendarMap = new HashMap<>()` | `HashMap<i32, Rc<RefCell<ProjectCalendar>>>` | Calendars are held as shared handles |
//! | `Pair<ProjectCalendar, Integer>` | [`crate::common::pair::Pair`] | A type of the same name has been ported |
//! | `NumberHelper.getDouble(double)` | Pass `Some(f64)` directly | This Java overload merely boxes the primitive `double`, with no extra semantics |
//! | `data[268] & data[303]` (`byte`) | `data[268] & data[303]` (`u8`) | Bitwise AND is fully equivalent for signed and unsigned, so `& 0xFF` need not be kept |
//! | `resource.getAvailability().add(new Availability(...))` | `resource.add_availability(Availability::new(...))` | `get_availability()` only lends a read-only `Ref`, so append via `Resource`'s write entry point (Java directly holds a mutable object) |
//! | `FileNotFoundException` → `addIgnoredError` | [`MpxjError`] + `add_ignored_error` | A missing `TBkndCons` is tolerable (Java explicitly commented on this) |
//! | `throws MPXJException, IOException` | `MpxjResult<()>` | All private methods uniformly return `MpxjResult<()>` |
//! | `StringBuilder` (stripping `&` from table names) | `chars().filter()` | Semantics are identical |
//! | Out-of-bounds reads (`ByteArrayHelper.getInt/getShort`) | `get_int_safe` / `get_short_safe` | Java throws `ArrayIndexOutOfBoundsException`; Rust returns 0 |
//! | `LocalTime.plus(duration, ChronoUnit.MILLIS)` | `plus_milliseconds` | chrono has no method that wraps around by milliseconds |
//! | Indexed setters such as `task.setCost(1, …)` | `set_cost_at(1, …)` etc. | This crate uniformly adds an `_at` suffix for such overloads (`Task`'s `setDuration`/`setDate`/`setStart`/`setFinish`/`setText`/`setNumber`/`setFlag` still use their original names; see the comments at each site) |
//! | `FieldTypeHelper.getInstance` returns `FieldType` | `Option<MppFieldType>` → `to_field_key()` | See `crate::common::mpp_field_type` |
//! | `RecurringTaskReader.processRecurringTask(task, data)` (internally `task.addRecurringTask()`) | After returning the result, this class writes `task.add_recurring_task()` | Writing to `Task` on the Rust side requires `&mut Task`, so "reading" and "attaching" are split into two steps (see the `crate::mpp::recurring_task_reader` file header) |
//!
//! ## Explicitly skipped Java content
//!
//! | Java | Reason |
//! |---|---|
//! | The commented-out `dumpUnknownData(...)` calls and `UNKNOWN_*_DATA` constants, `System.out.println(table)` | Already comments in the Java source (debugging tools); not involved in parsing |
//! | Streams that only exist in MPP12/14, such as `__propertiesVersion` | MPP8Reader (Java) does not read them; MPP8 properties come from the `Props` stream |
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: not used by this class.
//!
//! # Parts handled by `MPPReader` and not duplicated in this class
//!
//! `updateStructure()`, summary task marking, relation validation, baseline backfill, derived calendar cleanup, `readComplete()`
//! etc. are all performed uniformly by [`MPPReader`] after `process` returns (see the trait file header).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use chrono::Weekday;

use crate::accrue_type::AccrueType;
use crate::availability::Availability;
use crate::column::Column;
use crate::common::day_of_week_helper::DayOfWeekHelper;
use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::local_date_time_helper::LocalDateTimeHelper;
use crate::common::pair::Pair;
use crate::constraint_type::ConstraintType;
use crate::cost_rate_table::CostRateTable;
use crate::cost_rate_table_entry::CostRateTableEntry;
use crate::duration::Duration;
use crate::event_manager::EventManager;
use crate::field_type_class::FieldTypeClass;
use crate::local_time_range::LocalTimeRange;
use crate::mpp::abstract_calendar_factory::{get_int_safe, get_short_safe, plus_milliseconds};
use crate::mpp::extended_data::ExtendedData;
use crate::mpp::fix_defer_fix::FixDeferFix;
use crate::mpp::fix_fix::FixFix;
use crate::mpp::hyperlink_reader::HyperlinkReader;
use crate::mpp::mpp_reader::MPPReader;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::mpp_variant_reader::MPPVariantReader;
use crate::mpp::ole::DirectoryEntry;
use crate::mpp::project_properties_reader::ProjectPropertiesReader;
use crate::mpp::props8::Props8;
use crate::mpp::props_key::PropsKey;
use crate::mpp::recurring_task_reader::RecurringTaskReader;
use crate::mpp::task_type_helper::TaskTypeHelper;
use crate::mpp::view8::View8;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::notes::Notes;
use crate::priority::Priority;
use crate::project_calendar::ProjectCalendar;
use crate::project_calendar_hours::ProjectCalendarHours;
use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
use crate::project_entity_with_id::ProjectEntityWithID;
use crate::project_file::ProjectFile;
use crate::rate::Rate;
// Corresponds to Java: `org.mpxj.Relation.Builder` (this crate's `relation` module flattens the nested class
// `Builder` into `crate::relation::Builder`; an alias is made here to correspond to the Java form)
use crate::relation::Builder as RelationBuilder;
use crate::relation_type::RelationType;
use crate::rtf_notes::RtfNotes;
use crate::table::Table;
use crate::task::Task;
use crate::time_unit::TimeUnit;
use crate::view::View;

// ---------------------------------------------------------------------------
// Task var data type numbers
// ---------------------------------------------------------------------------

/// Corresponds to Java: `TASK_WBS`,`TASK_CONTACT`
const TASK_WBS: i32 = 104;
const TASK_CONTACT: i32 = 105;

/// Strip the trailing NUL terminator from an MPP8 record.
///
/// Corresponds to Java: the text after `new RtfNotes(notes)` is converted by `StringTextConverter`;
/// The raw bytes of `FixDeferFix` may carry out newlines and NULs together; the Java converter ignores them.
fn normalize_mpp8_notes(notes: String) -> String {
    let had_nul = notes.ends_with('\0');
    let mut notes = notes.trim_end_matches('\0').to_owned();
    if had_nul && notes.ends_with('\n') {
        notes.pop();
    }
    notes
}

/// Corresponds to Java: `TASK_TEXT1` … `TASK_TEXT10` (106..115) and `TASK_TEXT11` … `TASK_TEXT30` (184..203)
const TASK_TEXT: [i32; 30] = [
    106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 184, 185, 186, 187, 188, 189, 190, 191, 192,
    193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203,
];

/// Corresponds to Java: `TASK_START1` … `TASK_START10`
const TASK_START: [i32; 10] = [116, 118, 120, 122, 124, 126, 128, 130, 132, 134];

/// Corresponds to Java: `TASK_FINISH1` … `TASK_FINISH10`
const TASK_FINISH: [i32; 10] = [117, 119, 121, 123, 125, 127, 129, 131, 133, 135];

/// Corresponds to Java: `TASK_NUMBER1` … `TASK_NUMBER10` (137..146) and `TASK_NUMBER11` … `TASK_NUMBER20` (204..213)
const TASK_NUMBER: [i32; 20] = [
    137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 204, 205, 206, 207, 208, 209, 210, 211, 212,
    213,
];

/// Corresponds to Java: `TASK_DURATION1` … `TASK_DURATION10`
const TASK_DURATION: [i32; 10] = [147, 149, 151, 153, 155, 157, 159, 161, 163, 165];

/// Corresponds to Java: `TASK_DURATION1_UNITS` … `TASK_DURATION10_UNITS`
const TASK_DURATION_UNITS: [i32; 10] = [148, 150, 152, 154, 156, 158, 160, 162, 164, 166];

/// Corresponds to Java: `TASK_RECURRING_DATA`
const TASK_RECURRING_DATA: i32 = 168;

/// Corresponds to Java: `TASK_DATE1` … `TASK_DATE10`
const TASK_DATE: [i32; 10] = [174, 175, 176, 177, 178, 179, 180, 181, 182, 183];

/// Corresponds to Java: `TASK_HYPERLINK`
const TASK_HYPERLINK: i32 = 236;

/// Corresponds to Java: `TASK_COST1` … `TASK_COST10`
const TASK_COST: [i32; 10] = [237, 238, 239, 240, 241, 242, 243, 244, 245, 246];

/// Corresponds to Java: `TASK_NOTES`
const TASK_NOTES: i32 = 247;

// ---------------------------------------------------------------------------
// Resource var data type numbers
// ---------------------------------------------------------------------------

/// Corresponds to Java: `RESOURCE_GROUP`,`RESOURCE_CODE`,`RESOURCE_EMAIL`
const RESOURCE_GROUP: i32 = 61;
const RESOURCE_CODE: i32 = 62;
const RESOURCE_EMAIL: i32 = 63;

/// Corresponds to Java: `RESOURCE_TEXT1` … `RESOURCE_TEXT30`
const RESOURCE_TEXT: [i32; 30] = [
    64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86,
    87, 88, 89, 90, 91, 92, 93,
];

/// Corresponds to Java: `RESOURCE_START1` … `RESOURCE_START10`
const RESOURCE_START: [i32; 10] = [94, 95, 96, 97, 98, 99, 100, 101, 102, 103];

/// Corresponds to Java: `RESOURCE_FINISH1` … `RESOURCE_FINISH10`
const RESOURCE_FINISH: [i32; 10] = [104, 105, 106, 107, 108, 109, 110, 111, 112, 113];

/// Corresponds to Java: `RESOURCE_NUMBER1` … `RESOURCE_NUMBER20`
const RESOURCE_NUMBER: [i32; 20] = [
    114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132,
    133,
];

/// Corresponds to Java: `RESOURCE_DURATION1` … `RESOURCE_DURATION10`
const RESOURCE_DURATION: [i32; 10] = [134, 135, 136, 137, 138, 139, 140, 141, 142, 143];

/// Corresponds to Java: `RESOURCE_DURATION1_UNITS` … `RESOURCE_DURATION10_UNITS`
const RESOURCE_DURATION_UNITS: [i32; 10] = [144, 145, 146, 147, 148, 149, 150, 151, 152, 153];

/// Corresponds to Java: `RESOURCE_DATE1` … `RESOURCE_DATE10`
const RESOURCE_DATE: [i32; 10] = [157, 158, 159, 160, 161, 162, 163, 164, 165, 166];

/// Corresponds to Java: `RESOURCE_NOTES`
const RESOURCE_NOTES: i32 = 169;

/// Corresponds to Java: `RESOURCE_COST1` … `RESOURCE_COST10`
const RESOURCE_COST: [i32; 10] = [170, 171, 172, 173, 174, 175, 176, 177, 178, 179];

/// Corresponds to Java: `DEFAULT_WORKING_WEEK` (index 0 = Sunday)
const DEFAULT_WORKING_WEEK: [bool; 7] = [false, true, true, true, true, true, false];

/// Length of each table fixed-data record.
///
/// Corresponds to Java: `new FixFix(126, …)`
const TABLE_ITEM_SIZE: i32 = 126;

/// Length of each column definition record.
///
/// Corresponds to Java: `index += 12;`
const COLUMN_BLOCK_SIZE: usize = 12;

/// Reader for MPP8 (Microsoft Project 98) files.
///
/// Corresponds to Java: `final class MPP8Reader implements MPPVariantReader`
///
/// Java's implicit default constructor corresponds to [`MPP8Reader::new`]; this class itself holds no state,
/// and the mutable state during parsing is kept in the private struct `MPP8ReaderState`.
#[derive(Debug, Default)]
pub struct MPP8Reader;

impl MPP8Reader {
    /// Constructor.
    ///
    /// Corresponds to Java: implicit default constructor `MPP8Reader()`
    pub fn new() -> Self {
        Self
    }

    /// Contents of the `try` block of Java's `process`.
    ///
    /// Rust-specific: Java uses `try { … } finally { clearMemberData(); }`; the Rust side extracts the `try` block into
    /// this function so that the `finally` equivalent (`clear_member_data`) still runs when `?` returns early in `process`.
    fn run_process(
        state: &mut MPP8ReaderState,
        reader: &MPPReader,
        file: &Rc<RefCell<ProjectFile>>,
        root: &DirectoryEntry,
    ) -> MpxjResult<()> {
        // Java: populateMemberData(file, root);
        state.populate_member_data(file, root)?;
        // Java: processProjectProperties(); followed by the various data blocks (trimmed according to the reader's switches)
        state.run(
            reader.get_read_properties_only(),
            reader.get_read_presentation_data(),
        )
    }
}

impl MPPVariantReader for MPP8Reader {
    /// Process one MPP8 file (the format used by Project 98).
    ///
    /// Corresponds to Java: `@Override public void process(MPPReader reader, ProjectFile file, DirectoryEntry root)`
    fn process(
        &self,
        reader: &MPPReader,
        file: &Rc<RefCell<ProjectFile>>,
        root: &DirectoryEntry,
    ) -> MpxjResult<()> {
        // Java: try { … } finally { clearMemberData(); }
        let mut state = MPP8ReaderState::new();
        let result = MPP8Reader::run_process(&mut state, reader, file, root);
        state.clear_member_data();
        result
    }
}

/// Mutable state during [`MPP8Reader`] parsing.
///
/// Rust-specific: corresponds to the instance fields of Java's `MPP8Reader`
/// `m_file` / `m_eventManager` / `m_calendarMap` / `m_root` / `m_projectDir` / `m_viewDir` /
/// `m_projectProps` (`m_eventManager` is not stored; see the file header difference table).
struct MPP8ReaderState {
    /// Corresponds to Java: `private ProjectFile m_file`
    file: Rc<RefCell<ProjectFile>>,
    /// Corresponds to Java: `private DirectoryEntry m_root`
    root: Option<DirectoryEntry>,
    /// Corresponds to Java: `private DirectoryEntry m_projectDir`
    project_dir: Option<DirectoryEntry>,
    /// Corresponds to Java: `private DirectoryEntry m_viewDir`
    view_dir: Option<DirectoryEntry>,
    /// Corresponds to Java: `private Props8 m_projectProps`
    project_props: Option<Props8>,
    /// Corresponds to Java: `private HashMap<Integer, ProjectCalendar> m_calendarMap`
    calendar_map: HashMap<i32, Rc<RefCell<ProjectCalendar>>>,
}

impl MPP8ReaderState {
    /// Construct empty state (all Java-side field initial values are `null`).
    fn new() -> Self {
        Self {
            file: ProjectFile::new(),
            root: None,
            project_dir: None,
            view_dir: None,
            project_props: None,
            calendar_map: HashMap::new(),
        }
    }

    /// Execute the reading of each data block in the order of Java's `process`.
    ///
    /// Rust-specific: extracts the skeleton of Java's `process` so that [`MPPReader`] and unit tests can share it
    /// (the two switches are taken from `reader` in Java and passed in explicitly here).
    fn run(&mut self, read_properties_only: bool, read_presentation_data: bool) -> MpxjResult<()> {
        self.process_project_properties()?;

        if !read_properties_only {
            self.process_calendar_data()?;
            self.process_resource_data()?;
            self.process_task_data()?;
            self.process_constraint_data()?;
            self.process_assignment_data()?;

            if read_presentation_data {
                self.process_view_property_data()?;
                self.process_view_data()?;
                self.process_table_data()?;
            }
        }

        Ok(())
    }

    /// Populate the member data required by subsequent reads.
    ///
    /// Corresponds to Java: `private void populateMemberData(ProjectFile file, DirectoryEntry root) throws IOException`
    fn populate_member_data(
        &mut self,
        file: &Rc<RefCell<ProjectFile>>,
        root: &DirectoryEntry,
    ) -> MpxjResult<()> {
        // Java: m_root = root; m_file = file;
        self.root = Some(root.clone());
        self.file = Rc::clone(file);

        // Java: m_eventManager = file.getEventManager();
        // (Not stored on the Rust side; see the file header difference table)

        // Java: m_calendarMap = new HashMap<>();
        self.calendar_map = HashMap::new();

        // Java: m_projectDir = (DirectoryEntry) root.getEntry("   1");
        self.project_dir = root.get_directory(PROJECT_DIRECTORY);
        // Java: m_viewDir = (DirectoryEntry) root.getEntry("   2");
        self.view_dir = root.get_directory(VIEW_DIRECTORY);

        // Java: m_projectProps = new Props8(m_file, new DocumentInputStream(((DocumentEntry) m_projectDir.getEntry("Props"))));
        let props_data = self
            .project_dir
            .as_ref()
            .and_then(|dir| dir.get_document(PROPS_STREAM))
            .map(|document| document.get_data());

        match props_data {
            Some(data) => self.project_props = Some(Props8::new(file, &data)),
            // Java: FileNotFoundException → MPXJException (the entire read fails)
            None => {
                return Err(MpxjError::with_cause(
                    MpxjError::READ_ERROR,
                    &MpxjError::new(format!("Entry not found: {PROJECT_DIRECTORY}/{PROPS_STREAM}")),
                ));
            }
        }

        // Java: m_file.getProjectProperties().setMppFileType(Integer.valueOf(8));
        file.borrow_mut()
            .get_project_properties_mut()
            .set_mpp_file_type(Some(8));

        Ok(())
    }

    /// Clear the temporary member data.
    ///
    /// Corresponds to Java: `private void clearMemberData()`
    fn clear_member_data(&mut self) {
        // Java: m_root = null; m_eventManager = null; m_file = null; m_calendarMap = null;
        //       m_projectDir = null; m_viewDir = null; m_projectProps = null;
        self.root = None;
        self.project_dir = None;
        self.view_dir = None;
        self.project_props = None;
        self.calendar_map = HashMap::new();
        // Rust-specific: file is non-nullable, so an empty file is swapped back in to prevent further use of the state (Java sets null)
        self.file = ProjectFile::new();
    }

    /// Process the project properties data.
    ///
    /// Corresponds to Java: `private void processProjectProperties() throws MPXJException`
    fn process_project_properties(&mut self) -> MpxjResult<()> {
        // Java: ProjectPropertiesReader reader = new ProjectPropertiesReader();
        //       reader.process(m_file, m_projectProps, m_root);
        let Some(root) = self.root.as_ref() else {
            // This branch cannot occur in Java (m_root is always assigned)
            return Err(MpxjError::new("MPP8Reader: root is not populated"));
        };
        let Some(props) = self.project_props.as_ref() else {
            // This branch cannot occur in Java (m_projectProps is always assigned)
            return Err(MpxjError::new(
                "MPP8Reader: project properties are not populated",
            ));
        };

        ProjectPropertiesReader.process(&self.file, props, root)
    }

    /// Process the data in the `Props` file related to the project data's appearance.
    ///
    /// Corresponds to Java: `private void processViewPropertyData() throws IOException`
    fn process_view_property_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("Props"))
        let Some(data) = self
            .view_dir
            .as_ref()
            .and_then(|dir| dir.get_document(PROPS_STREAM))
            .map(|document| document.get_data())
        else {
            return Ok(());
        };

        // Java: Props8 props = new Props8(m_file, new DocumentInputStream(((DocumentEntry) m_viewDir.getEntry("Props"))));
        let props = Props8::new(&self.file, &data);
        // Java: ProjectProperties properties = m_file.getProjectProperties();
        //       properties.setShowProjectSummaryTask(props.getBoolean(PropsKey.SHOW_PROJECT_SUMMARY_TASK));
        let show_project_summary_task = props.get_boolean(PropsKey::ShowProjectSummaryTask);
        self.file
            .borrow_mut()
            .get_project_properties_mut()
            .set_show_project_summary_task(show_project_summary_task);

        Ok(())
    }

    /// Extract and organize the calendar data.
    ///
    /// Corresponds to Java: `private void processCalendarData() throws IOException`
    fn process_calendar_data(&mut self) -> MpxjResult<()> {
        // Java: DirectoryEntry calDir = (DirectoryEntry) m_projectDir.getEntry("TBkndCal");
        let Some(cal_dir) = self.project_directory(CALENDAR_DIRECTORY) else {
            return Err(MpxjError::with_cause(
                MpxjError::READ_ERROR,
                &MpxjError::new(format!("Entry not found: {CALENDAR_DIRECTORY}")),
            ));
        };

        // Java: FixFix calendarFixedData = new FixFix(36, new DocumentInputStream(((DocumentEntry) calDir.getEntry("FixFix   0"))));
        let Some(fix_fix_data) = read_document_data(&cal_dir, FIX_FIX_STREAM, CALENDAR_DIRECTORY)?
        else {
            return Err(MpxjError::new(
                "Entry not found: TBkndCal/FixFix   0",
            ));
        };
        let calendar_fixed_data = FixFix::new(36, &fix_fix_data);

        // Java: FixDeferFix calendarVarData = new FixDeferFix(new DocumentInputStream(((DocumentEntry) calDir.getEntry("FixDeferFix   0"))));
        let Some(fix_defer_data) =
            read_document_data(&cal_dir, FIX_DEFER_FIX_STREAM, CALENDAR_DIRECTORY)?
        else {
            return Err(MpxjError::new(
                "Entry not found: TBkndCal/FixDeferFix   0",
            ));
        };
        let calendar_var_data = FixDeferFix::new(&fix_defer_data);

        // Java: List<Pair<ProjectCalendar, Integer>> baseCalendars = new ArrayList<>();
        let mut base_calendars: Vec<Pair<Rc<RefCell<ProjectCalendar>>, i32>> = Vec::new();

        // Java: int calendars = calendarFixedData.getItemCount();
        let calendars = calendar_fixed_data.get_item_count();

        for loop_index in 0..calendars {
            // Java: baseData = calendarFixedData.getByteArrayValue(loop);
            let Some(base_data) = calendar_fixed_data.get_byte_array_value(loop_index) else {
                // Java would throw ArrayIndexOutOfBoundsException; the Rust side skips this entry
                continue;
            };

            // Java: calendarID = ByteArrayHelper.getInt(baseData, 0);
            let calendar_id = get_int_safe(base_data, 0);
            // Java: baseCalendarID = ByteArrayHelper.getInt(baseData, 4);
            let base_calendar_id = get_int_safe(base_data, 4);
            // Java: name = calendarVarData.getUnicodeString(getOffset(baseData, 20));
            let name = calendar_var_data.get_unicode_string(get_offset(base_data, 20));

            //
            // Java: skip calendars with a negative ID
            //
            if calendar_id < 0 {
                continue;
            }

            //
            // Java: ExtendedData ed = new ExtendedData(calendarVarData, getOffset(baseData, 32));
            //       offset = -1 - ed.getInt(Integer.valueOf(8));
            let extended = ExtendedData::new(&calendar_var_data, get_offset(base_data, 32));
            let offset = -1 - extended.get_int(8);

            let cal: Rc<RefCell<ProjectCalendar>>;

            if offset == -1 {
                //
                // Working calendar: there is no explicit working-hours data
                //
                if base_calendar_id > 0 {
                    // Java: cal = m_file.addDefaultDerivedCalendar();
                    //       baseCalendars.add(new Pair<>(cal, Integer.valueOf(baseCalendarID)));
                    cal = self.file.borrow_mut().add_default_derived_calendar();
                    base_calendars.push(Pair::new(Rc::clone(&cal), base_calendar_id));
                } else {
                    // Java: cal = m_file.addDefaultBaseCalendar(); cal.setName(name);
                    cal = self.file.borrow_mut().add_default_base_calendar();
                    set_calendar_name(&cal, name);
                }

                // Java: cal.setUniqueID(Integer.valueOf(calendarID));
                set_calendar_unique_id(&cal, calendar_id);
            } else {
                if base_calendar_id > 0 {
                    // Java: cal = m_file.addCalendar();
                    //       baseCalendars.add(new Pair<>(cal, Integer.valueOf(baseCalendarID)));
                    cal = self.file.borrow_mut().add_calendar();
                    base_calendars.push(Pair::new(Rc::clone(&cal), base_calendar_id));
                } else {
                    // Java: cal = m_file.addCalendar(); cal.setName(name);
                    cal = self.file.borrow_mut().add_calendar();
                    set_calendar_name(&cal, name);
                }

                // Java: cal.setUniqueID(Integer.valueOf(calendarID));
                set_calendar_unique_id(&cal, calendar_id);

                // Java: extData = calendarVarData.getByteArray(offset);
                match calendar_var_data.get_byte_array(offset) {
                    Some(ext_data) => self.populate_calendar_days(&cal, &ext_data),
                    None => {
                        // In Java, indexing a null here throws NullPointerException (the entire read fails);
                        // the Rust side degrades gracefully to "not writing working hours" without panicking.
                    }
                }
            }

            // Java: m_calendarMap.put(Integer.valueOf(calendarID), cal);
            self.calendar_map.insert(calendar_id, Rc::clone(&cal));
            // Java: m_eventManager.fireCalendarReadEvent(cal);
            self.with_event_manager(|manager| manager.fire_calendar_read_event(&cal));
        }

        // Java: updateBaseCalendarNames(baseCalendars);
        self.update_base_calendar_names(&base_calendars);

        //
        // Java:
        // ProjectCalendar projectDefaultCalendar = m_file.getCalendars().getByName(m_projectProps.getUnicodeString(PropsKey.DEFAULT_CALENDAR_NAME));
        // if (projectDefaultCalendar == null) { projectDefaultCalendar = m_file.getCalendars().findOrCreateDefaultCalendar(); }
        // m_file.getProjectProperties().setDefaultCalendar(projectDefaultCalendar);
        //
        let default_name = self
            .project_props
            .as_ref()
            .and_then(|props| props.get_unicode_string(PropsKey::DefaultCalendarName));

        let found = match self.file.try_borrow() {
            Ok(file) => file.get_calendars().get_by_name(default_name.as_deref()),
            // This branch cannot occur in Java (the file reference is stable)
            Err(_) => None,
        };

        let project_default_calendar = match found {
            Some(calendar) => Some(calendar),
            None => self.find_or_create_default_calendar(),
        };

        if let Some(project_default_calendar) = project_default_calendar {
            self.file.borrow_mut().set_default_calendar(Some(project_default_calendar));
        }

        Ok(())
    }

    /// Write the default working hours and exceptions for each day of a single calendar.
    ///
    /// Rust-specific: Java inlines this logic in `processCalendarData`'s `for (index = 0; index < 7; index++)`
    /// and exception loops; the Rust side extracts this method to avoid excessive nesting, with 1:1 semantics (`baseCalendarID` is re-read from the fixed data block
    /// , corresponding to Java using that local variable directly inside the loop).
    fn populate_calendar_days(&self, cal: &Rc<RefCell<ProjectCalendar>>, ext_data: &[u8]) {
        //
        // Java: for (index = 0; index < 7; index++) { offset = 4 + (40 * index); … }
        //
        for index in 0..7usize {
            let offset = 4 + (40 * index);

            // Java: defaultFlag = ByteArrayHelper.getShort(extData, offset);
            let default_flag = get_short_safe(ext_data, offset);
            // Java: day = DayOfWeekHelper.getInstance(index + 1);
            let Some(day) = DayOfWeekHelper::get_instance(index as i32 + 1) else {
                // Java's index + 1 is always within 1..=7 and never out of bounds; the Rust side skips defensively
                continue;
            };

            if default_flag == 1 {
                // Java: cal.setWorkingDay(day, DEFAULT_WORKING_WEEK[index]);
                set_working_day(cal, day, DEFAULT_WORKING_WEEK[index]);

                if is_working_day(cal, day) {
                    // Java: hours = cal.addCalendarHours(day);
                    //       hours.add(new LocalTimeRange(LocalTime.of(8, 0), LocalTime.of(12, 0)));
                    //       hours.add(new LocalTimeRange(LocalTime.of(13, 0), LocalTime.of(17, 0)));
                    add_calendar_hours(
                        cal,
                        day,
                        &[
                            LocalTimeRange::new(time(8, 0), time(12, 0)),
                            LocalTimeRange::new(time(13, 0), time(17, 0)),
                        ],
                    );
                } else if base_calendar_is_base(ext_data) {
                    // Java: if (baseCalendarID <= 0) { cal.addCalendarHours(day); }
                    add_calendar_hours(cal, day, &[]);
                }
            } else {
                // Java: periodCount = ByteArrayHelper.getShort(extData, offset + 2);
                //       hours = cal.addCalendarHours(day);
                let period_count = get_short_safe(ext_data, offset + 2);

                if period_count == 0 {
                    // Java: cal.setWorkingDay(day, false);
                    set_working_day(cal, day, false);
                    add_calendar_hours(cal, day, &[]);
                } else {
                    // Java: cal.setWorkingDay(day, true);
                    set_working_day(cal, day, true);

                    let mut ranges = Vec::new();
                    for period_index in 0..period_count {
                        let period_index = period_index as usize;
                        // Java: LocalTime start = MPPUtility.getTime(extData, offset + 8 + (periodIndex * 2));
                        let start = MPPUtility::get_time(ext_data, offset + 8 + (period_index * 2));
                        // Java: duration = MPPUtility.getDuration(extData, offset + 16 + (periodIndex * 4));
                        let duration =
                            MPPUtility::get_duration(ext_data, offset + 16 + (period_index * 4));
                        // Java: LocalTime end = start.plus(duration, ChronoUnit.MILLIS);
                        let end = plus_milliseconds(start, duration);
                        ranges.push(LocalTimeRange::new(Some(start), Some(end)));
                    }

                    add_calendar_hours(cal, day, &ranges);
                }
            }
        }

        //
        // Java: exceptionCount = ByteArrayHelper.getShort(extData, 0);
        //
        let exception_count = get_short_safe(ext_data, 0);
        if exception_count == 0 {
            return;
        }

        for index in 0..exception_count {
            let index = index as usize;
            let offset = 4 + (40 * 7) + (index * 44);

            // Java: LocalDate fromDate = MPPUtility.getDate(extData, offset);
            let from_date = MPPUtility::get_date(ext_data, offset);
            // Java: LocalDate toDate = MPPUtility.getDate(extData, offset + 2);
            let to_date = MPPUtility::get_date(ext_data, offset + 2);

            // Java: periodCount = ByteArrayHelper.getShort(extData, offset + 6);
            let period_count = get_short_safe(ext_data, offset + 6);

            let mut ranges = Vec::new();
            for exception_period_index in 0..period_count {
                let exception_period_index = exception_period_index as usize;
                // Java: LocalTime start = MPPUtility.getTime(extData, offset + 12 + (exceptionPeriodIndex * 2));
                let start =
                    MPPUtility::get_time(ext_data, offset + 12 + (exception_period_index * 2));
                // Java: duration = MPPUtility.getDuration(extData, offset + 20 + (exceptionPeriodIndex * 4));
                let duration =
                    MPPUtility::get_duration(ext_data, offset + 20 + (exception_period_index * 4));
                // Java: LocalTime end = start.plus(duration, ChronoUnit.MILLIS);
                let end = plus_milliseconds(start, duration);
                ranges.push(LocalTimeRange::new(Some(start), Some(end)));
            }

            // Java: exception = cal.addCalendarException(fromDate, toDate); … exception.add(...)
            if let Ok(mut cal_ref) = cal.try_borrow_mut() {
                let exception = cal_ref.add_calendar_exception(from_date, to_date);
                for range in ranges {
                    exception.add(range);
                }
            }
        }
    }

    /// Convert the "base calendar ID" placeholder into a real parent-child relationship.
    ///
    /// In MPP8 files, a derived calendar's reference to its base calendar may be a forward reference, so the base calendar ID is stored temporarily
    /// and replaced with the real parent-child relationship in this method after all calendars have been read.
    ///
    /// Corresponds to Java: `private void updateBaseCalendarNames(List<Pair<ProjectCalendar, Integer>> baseCalendars)`
    fn update_base_calendar_names(
        &self,
        base_calendars: &[Pair<Rc<RefCell<ProjectCalendar>>, i32>],
    ) {
        for pair in base_calendars {
            // Java: ProjectCalendar cal = pair.getFirst(); Integer baseCalendarID = pair.getSecond();
            let cal = pair.get_first();
            let base_calendar_id = *pair.get_second();

            // Java: ProjectCalendar baseCal = m_calendarMap.get(baseCalendarID);
            //       if (baseCal != null) { cal.setParent(baseCal); }
            if let Some(base_cal) = self.calendar_map.get(&base_calendar_id) {
                if let Ok(mut cal_ref) = cal.try_borrow_mut() {
                    cal_ref.set_parent(Some(Rc::clone(base_cal)));
                }
            }
        }
    }

    /// Extract and organize the task data.
    ///
    /// Corresponds to Java: `private void processTaskData() throws IOException`
    fn process_task_data(&mut self) -> MpxjResult<()> {
        // Java: DirectoryEntry taskDir = (DirectoryEntry) m_projectDir.getEntry("TBkndTask");
        let Some(task_dir) = self.project_directory(TASK_DIRECTORY) else {
            return Err(MpxjError::new("Entry not found: TBkndTask"));
        };

        // Java: FixFix taskFixedData = new FixFix(316, new DocumentInputStream(((DocumentEntry) taskDir.getEntry("FixFix   0"))));
        let Some(fix_fix_data) = read_document_data(&task_dir, FIX_FIX_STREAM, TASK_DIRECTORY)? else {
            return Err(MpxjError::new("Entry not found: TBkndTask/FixFix   0"));
        };
        let mut task_fixed_data = FixFix::new(316, &fix_fix_data);

        // Java: if (taskFixedData.getDiff() != 0) { taskFixedData = new FixFix(366, …); }
        if task_fixed_data.get_diff() != 0 {
            task_fixed_data = FixFix::new(366, &fix_fix_data);
        }

        // Java: FixDeferFix taskVarData = null;
        let mut task_var_data: Option<FixDeferFix> = None;

        // Java: int tasks = taskFixedData.getItemCount();
        let tasks = task_fixed_data.get_item_count();

        // Java: boolean autoWBS = true;
        let mut auto_wbs = true;

        // Java: RecurringTaskReader recurringTaskReader = null;
        let mut recurring_task_reader: Option<RecurringTaskReader> = None;

        // Java: ProjectProperties properties = m_file.getProjectProperties();
        //       TimeUnit defaultProjectTimeUnits = properties.getDefaultDurationUnits();
        let default_project_time_units = self.default_duration_units();

        // Java: HyperlinkReader hyperlinkReader = new HyperlinkReader();
        //       hyperlinkReader.setHasScreenTip(false);
        let mut hyperlink_reader = HyperlinkReader::new();
        hyperlink_reader.set_has_screen_tip(false);

        for loop_index in 0..tasks {
            // Java: data = taskFixedData.getByteArrayValue(loop);
            let Some(data) = task_fixed_data.get_byte_array_value(loop_index) else {
                continue;
            };

            //
            // Validate the unique ID
            //
            // Java: uniqueID = ByteArrayHelper.getInt(data, 0);
            let unique_id = get_int_safe(data, 0);
            if unique_id < 1 {
                continue;
            }

            //
            // Validate whether the task is deleted. This appears to be a set of flag bits rather than a single value: deleted tasks have shown
            // 0x0001 and 0x0002; valid tasks have shown 0x0000, 0x0914, 0x0040, 0x004A, 0x203D, 0x0031.
            //
            // Java: deleted = ByteArrayHelper.getShort(data, 272);
            let deleted = get_short_safe(data, 272);
            if (deleted & 0xC0) == 0
                && (deleted & 0x03) != 0
                && deleted != 0x0031
                && deleted != 0x203D
            {
                continue;
            }

            //
            // Load var data only when first needed
            //
            // Java: if (taskVarData == null) { taskVarData = new FixDeferFix(…); }
            if task_var_data.is_none() {
                let Some(fix_defer_data) =
                    read_document_data(&task_dir, FIX_DEFER_FIX_STREAM, TASK_DIRECTORY)?
                else {
                    return Err(MpxjError::new("Entry not found: TBkndTask/FixDeferFix   0"));
                };
                task_var_data = Some(FixDeferFix::new(&fix_defer_data));
            }
            let Some(task_var_data) = task_var_data.as_ref() else {
                continue;
            };

            //
            // MPP files may contain blank rows: this flag indicates the current row is blank and should be ignored.
            //
            // Java: if ((data[8] & 0x01) != 0) { continue; }
            if (get_byte_safe(data, 8) & 0x01) != 0 {
                continue;
            }

            // Java: taskExtData = new ExtendedData(taskVarData, getOffset(data, 312));
            let task_ext_data = ExtendedData::new(task_var_data, get_offset(data, 312));
            // Java: byte[] recurringData = taskExtData.getByteArray(TASK_RECURRING_DATA);
            let recurring_data = task_ext_data
                .get_byte_array(TASK_RECURRING_DATA)
                .map(<[u8]>::to_vec);

            // Java: id = ByteArrayHelper.getInt(data, 4);
            let id = get_int_safe(data, 4);

            // Java: flags[0] = (byte) (data[268] & data[303]); and three such groups
            // (Java's `byte` is signed, but the bitwise-AND result is identical to unsigned, so Rust uses u8 directly)
            let flags = [
                get_byte_safe(data, 268) & get_byte_safe(data, 303),
                get_byte_safe(data, 269) & get_byte_safe(data, 304),
                get_byte_safe(data, 270) & get_byte_safe(data, 305),
            ];

            // Java: task = m_file.addTask();
            let task = self.file.borrow_mut().add_task();

            {
                let Ok(mut task_ref) = task.try_borrow_mut() else {
                    // This branch cannot occur in Java (the task has just been created, with no other borrows)
                    continue;
                };

                // Java: task.setActualCost(NumberHelper.getDouble(((double) MPPUtility.getLong6(data, 234)) / 100));
                task_ref.set_actual_cost(Some(to_money(get_long6_safe(data, 234))));
                // Java: task.setActualDuration(MPPUtility.getAdjustedDuration(properties, ByteArrayHelper.getInt(data, 74), MPPUtility.getDurationTimeUnits(ByteArrayHelper.getShort(data, 72), defaultProjectTimeUnits)));
                task_ref.set_actual_duration(self.adjusted_duration(
                    get_int_safe(data, 74),
                    MPPUtility::get_duration_time_units_with_default(
                        get_short_safe(data, 72),
                        Some(default_project_time_units),
                    ),
                ));
                // Java: task.setActualFinish(MPPUtility.getTimestamp(data, 108));
                task_ref.set_actual_finish(MPPUtility::get_timestamp(data, 108));
                // Java: task.setActualOvertimeCost(NumberHelper.getDouble(((double) MPPUtility.getLong6(data, 210)) / 100));
                task_ref.set_actual_overtime_cost(Some(to_money(get_long6_safe(data, 210))));
                // Java: task.setActualOvertimeWork(MPPUtility.getDuration(((double) MPPUtility.getLong6(data, 192)) / 100, TimeUnit.HOURS));
                task_ref.set_actual_overtime_work(Some(MPPUtility::get_duration_double(
                    to_money(get_long6_safe(data, 192)),
                    TimeUnit::Hours,
                )));
                // Java: task.setActualStart(MPPUtility.getTimestamp(data, 104));
                task_ref.set_actual_start(MPPUtility::get_timestamp(data, 104));
                // Java: task.setActualWork(MPPUtility.getDuration(((double) MPPUtility.getLong6(data, 180)) / 100, TimeUnit.HOURS));
                task_ref.set_actual_work(Some(MPPUtility::get_duration_double(
                    to_money(get_long6_safe(data, 180)),
                    TimeUnit::Hours,
                )));
                // Java: task.setACWP() / setAssignment() / setAssignmentDelay() / setAssignmentUnits()
                //       —— all are computed values; skip.

                // Java: task.setBaselineCost(NumberHelper.getDouble((double) MPPUtility.getLong6(data, 246) / 100));
                task_ref.set_baseline_cost(Some(to_money(get_long6_safe(data, 246))));
                // Java: task.setBaselineDuration(MPPUtility.getAdjustedDuration(properties, ByteArrayHelper.getInt(data, 82), MPPUtility.getDurationTimeUnits(ByteArrayHelper.getShort(data, 72), defaultProjectTimeUnits)));
                task_ref.set_baseline_duration(self.adjusted_duration(
                    get_int_safe(data, 82),
                    MPPUtility::get_duration_time_units_with_default(
                        get_short_safe(data, 72),
                        Some(default_project_time_units),
                    ),
                ));
                // Java: task.setBaselineFinish(MPPUtility.getTimestamp(data, 116));
                task_ref.set_baseline_finish(MPPUtility::get_timestamp(data, 116));
                // Java: task.setBaselineStart(MPPUtility::getTimestamp(data, 112));
                task_ref.set_baseline_start(MPPUtility::get_timestamp(data, 112));
                // Java: task.setBaselineWork(MPPUtility.getDuration(((double) MPPUtility.getLong6(data, 174)) / 100, TimeUnit.HOURS));
                task_ref.set_baseline_work(Some(MPPUtility::get_duration_double(
                    to_money(get_long6_safe(data, 174)),
                    TimeUnit::Hours,
                )));
                // Java: task.setBCWP() / setBCWS() / setConfirmed() —— computed values; skip.

                // Java: task.setConstraintDate(MPPUtility.getTimestamp(data, 120));
                task_ref.set_constraint_date(MPPUtility::get_timestamp(data, 120));
                // Java: task.setConstraintType(ConstraintType.getInstance(ByteArrayHelper.getShort(data, 88)));
                task_ref.set_constraint_type(Some(ConstraintType::get_instance(get_short_safe(
                    data, 88,
                ))));
                // Java: task.setContact(taskExtData.getUnicodeString(TASK_CONTACT));
                task_ref.set_contact(task_ext_data.get_unicode_string(TASK_CONTACT));
                // Java: task.setCost(NumberHelper.getDouble(((double) MPPUtility.getLong6(data, 222)) / 100));
                task_ref.set_cost(Some(to_money(get_long6_safe(data, 222))));
                // Java: task.setCost(1..10, …taskExtData.getLong(TASK_COSTn) / 100);
                for index in 1..=10i32 {
                    let value = task_ext_data.get_long(TASK_COST[(index - 1) as usize]);
                    task_ref.set_cost_at(index, Some(to_money(value)));
                }
                // Java: task.setCostRateTable() / setCostVariance() —— computed values; skip.

                // Java: task.setCreateDate(MPPUtility.getTimestamp(data, 138));
                task_ref.set_create_date(MPPUtility::get_timestamp(data, 138));
                // Java: task.setCritical() / setCV() —— computed values; skip.

                // Java: task.setDate(1..10, taskExtData.getTimestamp(TASK_DATEn));
                for index in 1..=10i32 {
                    let value = task_ext_data.get_timestamp(TASK_DATE[(index - 1) as usize]);
                    task_ref.set_date(index, value);
                }

                // Java: task.setDelay() —— no longer supported by MS Project; skip.

                // Java: task.setDuration(MPPUtility.getAdjustedDuration(properties, ByteArrayHelper.getInt(data, 68), MPPUtility.getDurationTimeUnits(ByteArrayHelper.getShort(data, 72), defaultProjectTimeUnits)));
                task_ref.set_duration(self.adjusted_duration(
                    get_int_safe(data, 68),
                    MPPUtility::get_duration_time_units_with_default(
                        get_short_safe(data, 72),
                        Some(default_project_time_units),
                    ),
                ));
                // Java: task.setDuration(1..10, MPPUtility.getAdjustedDuration(properties, taskExtData.getInt(TASK_DURATIONn), MPPUtility.getDurationTimeUnits(taskExtData.getShort(TASK_DURATIONn_UNITS), defaultProjectTimeUnits)));
                for index in 1..=10i32 {
                    let slot = (index - 1) as usize;
                    let value = self.adjusted_duration(
                        task_ext_data.get_int(TASK_DURATION[slot]),
                        MPPUtility::get_duration_time_units_with_default(
                            task_ext_data.get_short(TASK_DURATION_UNITS[slot]),
                            Some(default_project_time_units),
                        ),
                    );
                    task_ref.set_duration_at(index, value);
                }

                // Java: task.setDurationVariance() —— computed value; skip.

                // Java: task.setEarlyFinish(MPPUtility.getTimestamp(data, 20));
                task_ref.set_early_finish(MPPUtility::get_timestamp(data, 20));
                // Java: task.setEarlyStart(MPPUtility.getTimestamp(data, 96));
                task_ref.set_early_start(MPPUtility::get_timestamp(data, 96));
                // Java: task.setEffortDriven((data[17] & 0x08) != 0);
                task_ref.set_effort_driven((get_byte_safe(data, 17) & 0x08) != 0);

                // Java: task.setExternalTask() —— computed value; skip.

                // Java: task.setFinish(MPPUtility.getTimestamp(data, 20));
                task_ref.set_finish(MPPUtility::get_timestamp(data, 20));
                // Java: task.setFinish(1..10, taskExtData.getTimestamp(TASK_FINISHn));
                for index in 1..=10i32 {
                    let value = task_ext_data.get_timestamp(TASK_FINISH[(index - 1) as usize]);
                    task_ref.set_finish_at(index, value);
                }
                // Java: task.setFinishVariance() —— computed value; skip.

                // Java: task.setFixedCost(NumberHelper.getDouble(((double) MPPUtility.getLong6(data, 228)) / 100));
                task_ref.set_fixed_cost(Some(to_money(get_long6_safe(data, 228))));
                // Java: task.setFixedCostAccrual(AccrueType.getInstance(ByteArrayHelper.getShort(data, 136)));
                task_ref.set_fixed_cost_accrual(AccrueType::get_instance(get_short_safe(data, 136)));

                // Java: task.setFlag(1..20, (flags[…] & bit) != 0);
                // Note that flag bit 20 is "not correct" according to Java's comment; translated as-is here.
                task_ref.set_flag(1, (flags[0] & 0x02) != 0);
                task_ref.set_flag(2, (flags[0] & 0x04) != 0);
                task_ref.set_flag(3, (flags[0] & 0x08) != 0);
                task_ref.set_flag(4, (flags[0] & 0x10) != 0);
                task_ref.set_flag(5, (flags[0] & 0x20) != 0);
                task_ref.set_flag(6, (flags[0] & 0x40) != 0);
                task_ref.set_flag(7, (flags[0] & 0x80) != 0);
                task_ref.set_flag(8, (flags[1] & 0x01) != 0);
                task_ref.set_flag(9, (flags[1] & 0x02) != 0);
                task_ref.set_flag(10, (flags[1] & 0x04) != 0);
                task_ref.set_flag(11, (flags[1] & 0x08) != 0);
                task_ref.set_flag(12, (flags[1] & 0x10) != 0);
                task_ref.set_flag(13, (flags[1] & 0x20) != 0);
                task_ref.set_flag(14, (flags[1] & 0x40) != 0);
                task_ref.set_flag(15, (flags[1] & 0x80) != 0);
                task_ref.set_flag(16, (flags[2] & 0x01) != 0);
                task_ref.set_flag(17, (flags[2] & 0x02) != 0);
                task_ref.set_flag(18, (flags[2] & 0x04) != 0);
                task_ref.set_flag(19, (flags[2] & 0x08) != 0);
                task_ref.set_flag(20, (flags[2] & 0x10) != 0);

                // Java: task.setFreeSlack() —— computed value; skip.

                // Java: task.setHideBar((data[16] & 0x01) != 0);
                task_ref.set_hide_bar((get_byte_safe(data, 16) & 0x01) != 0);

                // Java: hyperlinkReader.read(task, taskVarData.getByteArray(-1 - taskExtData.getInt(TASK_HYPERLINK)));
                let hyperlink_data =
                    task_var_data.get_byte_array(-1 - task_ext_data.get_int(TASK_HYPERLINK));
                if let Some(hyperlink_data) = hyperlink_data.as_deref() {
                    // Java: hyperlinkReader.read(task, …) (`task_ref` is a `RefMut`, which must be dereferenced to `&mut Task`
                    // to actually obtain `&mut dyn FieldContainer`)
                    hyperlink_reader.read(FieldTypeClass::Task, &mut *task_ref, hyperlink_data);
                }

                // Java: task.setID(Integer.valueOf(id));
                task_ref.set_id(Some(id));
                // Java: task.setIndicators() —— computed value; skip.

                // Java: task.setLateFinish(MPPUtility.getTimestamp(data, 160));
                task_ref.set_late_finish(MPPUtility::get_timestamp(data, 160));
                // Java: task.setLateStart(MPPUtility.getTimestamp(data, 24));
                task_ref.set_late_start(MPPUtility::get_timestamp(data, 24));
                // Java: task.setLevelAssignments((data[19] & 0x10) != 0);
                task_ref.set_level_assignments((get_byte_safe(data, 19) & 0x10) != 0);
                // Java: task.setLevelingCanSplit((data[19] & 0x08) != 0);
                task_ref.set_leveling_can_split((get_byte_safe(data, 19) & 0x08) != 0);
                // Java: task.setLevelingDelay(MPPUtility.getDuration(((double) ByteArrayHelper.getInt(data, 90)) / 3, MPPUtility.getDurationTimeUnits(ByteArrayHelper.getShort(data, 94), defaultProjectTimeUnits)));
                task_ref.set_leveling_delay(Some(MPPUtility::get_duration_double(
                    f64::from(get_int_safe(data, 90)) / 3.0,
                    MPPUtility::get_duration_time_units_with_default(
                        get_short_safe(data, 94),
                        Some(default_project_time_units),
                    ),
                )));
                // Java: task.setLinkedFields() —— computed value; skip.

                // Java: task.setMarked((data[13] & 0x02) != 0);
                task_ref.set_marked((get_byte_safe(data, 13) & 0x02) != 0);
                // Java: task.setMilestone((data[12] & 0x01) != 0);
                task_ref.set_milestone((get_byte_safe(data, 12) & 0x01) != 0);

                // Java: task.setName(taskVarData.getUnicodeString(getOffset(data, 264)));
                let name = task_var_data.get_unicode_string(get_offset(data, 264));
                task_ref.set_name(name);

                // Java: task.setNumber(1..20, NumberHelper.getDouble(taskExtData.getDouble(TASK_NUMBERn)));
                for index in 1..=20i32 {
                    let value = task_ext_data.get_double(TASK_NUMBER[(index - 1) as usize]);
                    task_ref.set_number(index, Some(value));
                }

                // Java: task.setObjects() —— computed value; skip.

                // Java: task.setOutlineLevel(Integer.valueOf(ByteArrayHelper.getShort(data, 48)));
                task_ref.set_outline_level(Some(get_short_safe(data, 48)));
                // Java: task.setOutlineNumber() / setOverallocated() —— computed values; skip.

                // Java: task.setOvertimeCost(NumberHelper.getDouble(((double) MPPUtility.getLong6(data, 204)) / 100));
                task_ref.set_overtime_cost(Some(to_money(get_long6_safe(data, 204))));
                // Java: task.setOvertimeWork() —— computed value; skip.
                // Java: task.getPredecessors() —— computed value; skip.

                // Java: task.setPercentageComplete(MPPUtility.getPercentage(data, 130));
                task_ref.set_percentage_complete(MPPUtility::get_percentage(data, 130));
                // Java: task.setPercentageWorkComplete(MPPUtility.getPercentage(data, 132));
                task_ref.set_percentage_work_complete(MPPUtility::get_percentage(data, 132));
                // Java: task.setPreleveledFinish(MPPUtility.getTimestamp(data, 148));
                task_ref.set_preleveled_finish(MPPUtility::get_timestamp(data, 148));
                // Java: task.setPreleveledStart(MPPUtility.getTimestamp(data, 144));
                task_ref.set_preleveled_start(MPPUtility::get_timestamp(data, 144));
                // Java: task.setPriority(Priority.getInstance((ByteArrayHelper.getShort(data, 128) + 1) * 100));
                task_ref.set_priority(Some(Priority::get_instance(
                    (get_short_safe(data, 128) + 1) * 100,
                )));
                // Java: task.setProject() —— computed value; skip.

                // Java: task.setRecurring(ByteArrayHelper.getShort(data, 142) != 0);
                task_ref.set_recurring(get_short_safe(data, 142) != 0);
                // Java: task.setRegularWork() —— computed value; skip.

                // Java: task.setRemainingCost(NumberHelper.getDouble(((double) MPPUtility.getLong6(data, 240)) / 100));
                task_ref.set_remaining_cost(Some(to_money(get_long6_safe(data, 240))));
                // Java: task.setRemainingDuration(MPPUtility.getAdjustedDuration(properties, ByteArrayHelper.getInt(data, 78), MPPUtility.getDurationTimeUnits(ByteArrayHelper.getShort(data, 72), defaultProjectTimeUnits)));
                task_ref.set_remaining_duration(self.adjusted_duration(
                    get_int_safe(data, 78),
                    MPPUtility::get_duration_time_units_with_default(
                        get_short_safe(data, 72),
                        Some(default_project_time_units),
                    ),
                ));
                // Java: task.setRemainingOvertimeCost(NumberHelper.getDouble(((double) MPPUtility.getLong6(data, 216)) / 100));
                task_ref.set_remaining_overtime_cost(Some(to_money(get_long6_safe(data, 216))));
                // Java: task.setRemainingOvertimeWork(MPPUtility.getDuration(((double) MPPUtility.getLong6(data, 198)) / 100, TimeUnit.HOURS));
                task_ref.set_remaining_overtime_work(Some(MPPUtility::get_duration_double(
                    to_money(get_long6_safe(data, 198)),
                    TimeUnit::Hours,
                )));
                // Java: task.setRemainingWork(MPPUtility.getDuration(((double) MPPUtility.getLong6(data, 186)) / 100, TimeUnit.HOURS));
                task_ref.set_remaining_work(Some(MPPUtility::get_duration_double(
                    to_money(get_long6_safe(data, 186)),
                    TimeUnit::Hours,
                )));
                // Java: setResourceGroup / setResourceInitials / setResourceNames / setResourcePhonetics /
                //       setResponsePending —— computed value; skip.

                // Java: task.setResume(MPPUtility.getTimestamp(data, 32));
                task_ref.set_resume(MPPUtility::get_timestamp(data, 32));
                // Java: task.setResumeNoEarlierThan() —— not present in MSP98; skip.

                // Java: task.setRollup((data[15] & 0x04) != 0);
                task_ref.set_rollup((get_byte_safe(data, 15) & 0x04) != 0);

                // Java: task.setStart(MPPUtility.getTimestamp(data, 96));
                task_ref.set_start(MPPUtility::get_timestamp(data, 96));
                // Java: task.setStart(1..10, taskExtData.getTimestamp(TASK_STARTn));
                for index in 1..=10i32 {
                    let value = task_ext_data.get_timestamp(TASK_START[(index - 1) as usize]);
                    task_ref.set_start_at(index, value);
                }
                // Java: task.setStartVariance() —— computed value; skip.

                // Java: task.setStop(MPPUtility.getTimestamp(data, 124));
                task_ref.set_stop(MPPUtility::get_timestamp(data, 124));

                // Java: setSubprojectFile / setSubprojectReadOnly / setSuccessors / setSummary / setSV /
                //       teamStatusPending —— computed value or handled by another module; skip.

                // Java: task.setText(1..30, taskExtData.getUnicodeString(TASK_TEXTn));
                for index in 1..=30i32 {
                    let value = task_ext_data.get_unicode_string(TASK_TEXT[(index - 1) as usize]);
                    task_ref.set_text(index, value);
                }

                // Java: task.setTotalSlack() —— computed value; skip.

                // Java: task.setType(TaskTypeHelper.getInstance(ByteArrayHelper.getShort(data, 134)));
                task_ref.set_type(Some(TaskTypeHelper::get_instance(get_short_safe(data, 134))));
                // Java: task.setUniqueID(Integer.valueOf(uniqueID));
                task_ref.set_unique_id(Some(unique_id));
                // Java: setUniqueIDPredecessors / setUniqueIDSuccessors / setUpdateNeeded —— computed values; skip.

                // Java: task.setWBS(taskExtData.getUnicodeString(TASK_WBS));
                task_ref.set_wbs(task_ext_data.get_unicode_string(TASK_WBS));
                // Java: task.setWork(MPPUtility.getDuration(((double) MPPUtility.getLong6(data, 168)) / 100, TimeUnit.HOURS));
                task_ref.set_work(Some(MPPUtility::get_duration_double(
                    to_money(get_long6_safe(data, 168)),
                    TimeUnit::Hours,
                )));
                // Java: setWorkContour (derived from resources) / setWorkVariance (computed value); skip.

                //
                // Recurring task data
                //
                // Java: if (recurringData != null) { if (recurringTaskReader == null) { … }
                //       recurringTaskReader.processRecurringTask(task, recurringData); }
                //
                if let Some(recurring_data) = recurring_data.as_deref() {
                    if recurring_task_reader.is_none() {
                        recurring_task_reader = match self.file.try_borrow() {
                            Ok(file) => Some(RecurringTaskReader::new(file.get_project_properties())),
                            // This branch cannot occur in Java (the file reference is stable)
                            Err(_) => None,
                        };
                    }

                    if let Some(reader) = recurring_task_reader.as_ref() {
                        let recurring_task = reader.process_recurring_task(recurring_data);
                        // Java: RecurringTaskReader.processRecurringTask internally calls `task.addRecurringTask()` directly;
                        // on the Rust side that reader returns a result of the same type; here it just needs to be attached to the task
                        *task_ref.add_recurring_task() = recurring_task;
                    }
                }

                //
                // Task notes
                //
                // Java: setTaskNotes(task, data, taskExtData, taskVarData);
                self.set_task_notes(&mut task_ref, data, &task_ext_data, task_var_data);

                //
                // If the MPP file provides a WBS, do not generate one automatically
                //
                // Java: if (task.getWBS() != null) { autoWBS = false; }
                if task_ref.get_wbs().is_some() {
                    auto_wbs = false;
                }
            }

            // Java: m_eventManager.fireTaskReadEvent(task);
            self.with_event_manager(|manager| manager.fire_task_read_event(&task));
        }

        //
        // Enable automatic WBS generation when necessary
        //
        // Java: m_file.getProjectConfig().setAutoWBS(autoWBS);
        self.set_auto_wbs(auto_wbs);

        Ok(())
    }

    /// Task notes appear to have two representations in MPP8 files; this method determines which one is used.
    ///
    /// Corresponds to Java: `private void setTaskNotes(Task task, byte[] data, ExtendedData taskExtData, FixDeferFix taskVarData)`
    fn set_task_notes(
        &self,
        task: &mut Task,
        data: &[u8],
        task_ext_data: &ExtendedData<'_>,
        task_var_data: &FixDeferFix,
    ) {
        // Java: String notes = taskExtData.getString(TASK_NOTES);
        let mut notes = task_ext_data.get_string(TASK_NOTES);

        if notes.is_none() && data.len() == 366 {
            // Java: byte[] offsetData = taskVarData.getByteArray(getOffset(data, 362));
            if let Some(offset_data) = task_var_data.get_byte_array(get_offset(data, 362)) {
                if offset_data.len() >= 12 {
                    // Java: notes = taskVarData.getString(getOffset(offsetData, 8));
                    notes = task_var_data.get_string(get_offset(&offset_data, 8));

                    // This approach picks up some random content, and our understanding of the file format is insufficient to determine
                    // when it should be ignored, so a heuristic rule is used here: content that does not look like RTF is always ignored.
                    // Java: if (notes != null && notes.indexOf('{') == -1) { notes = null; }
                    if let Some(value) = notes.as_ref() {
                        if !value.contains('{') {
                            notes = None;
                        }
                    }
                }
            }
        }

        // Java: if (notes != null) { task.setNotesObject(new RtfNotes(notes)); }
        if let Some(notes) = notes {
            let notes = normalize_mpp8_notes(notes);
            let rtf_notes = RtfNotes::new(Some(notes));
            task.set_notes_object(Some(&rtf_notes as &dyn Notes));
        }
    }

    /// Extract and organize the constraint (inter-task relation) data.
    ///
    /// Corresponds to Java: `private void processConstraintData() throws IOException`
    fn process_constraint_data(&mut self) -> MpxjResult<()> {
        //
        // Locate the directory holding the constraints
        //
        // Java:
        // try { consDir = (DirectoryEntry) m_projectDir.getEntry("TBkndCons"); }
        // catch (FileNotFoundException ex) { m_file.addIgnoredError(ex); consDir = null; }
        //
        let Some(cons_dir) = self.project_directory(CONSTRAINT_DIRECTORY) else {
            // A valid MPP8 file may lack this directory; in that case only record an "ignored error" and continue
            self.file.borrow_mut().add_ignored_error(MpxjError::new(
                format!("Entry not found: {CONSTRAINT_DIRECTORY}"),
            ));
            return Ok(());
        };

        //
        // Process constraints only when the directory exists
        //
        // Java: FixFix consFixedData = new FixFix(36, new DocumentInputStream(((DocumentEntry) consDir.getEntry("FixFix   0"))));
        let Some(fix_fix_data) = read_document_data(&cons_dir, FIX_FIX_STREAM, CONSTRAINT_DIRECTORY)?
        else {
            return Ok(());
        };
        let cons_fixed_data = FixFix::new(36, &fix_fix_data);
        let count = cons_fixed_data.get_item_count();

        for loop_index in 0..count {
            // Java: byte[] data = consFixedData.getByteArrayValue(loop);
            let Some(data) = cons_fixed_data.get_byte_array_value(loop_index) else {
                continue;
            };

            // Java: if (ByteArrayHelper.getInt(data, 28) == 0)
            if get_int_safe(data, 28) != 0 {
                continue;
            }

            // Java: int taskID1 = ByteArrayHelper.getInt(data, 12);
            let task_id1 = get_int_safe(data, 12);
            // Java: int taskID2 = ByteArrayHelper.getInt(data, 16);
            let task_id2 = get_int_safe(data, 16);

            if task_id1 == task_id2 {
                continue;
            }

            // Java: Task task1 = m_file.getTaskByUniqueID(Integer.valueOf(taskID1));
            //       Task task2 = m_file.getTaskByUniqueID(Integer.valueOf(taskID2));
            let task1 = self.file.borrow().get_task_by_unique_id(Some(task_id1));
            let task2 = self.file.borrow().get_task_by_unique_id(Some(task_id2));

            let (Some(task1), Some(task2)) = (task1, task2) else {
                continue;
            };

            //
            // Java:
            // Relation relation = task2.addPredecessor(new Relation.Builder()
            //    .predecessorTask(task1)
            //    .type(RelationType.getInstance(ByteArrayHelper.getShort(data, 20)))
            //    .lag(MPPUtility.getDuration(ByteArrayHelper.getInt(data, 24), MPPUtility.getDurationTimeUnits(ByteArrayHelper.getShort(data, 22))))
            //    .uniqueID(Integer.valueOf(ByteArrayHelper.getInt(data, 0))));
            //
            let builder = RelationBuilder::new()
                .predecessor_task(&task1)
                .r#type(RelationType::get_instance(get_short_safe(data, 20)))
                .lag(Some(MPPUtility::get_duration_int(
                    get_int_safe(data, 24),
                    MPPUtility::get_duration_time_units(get_short_safe(data, 22)),
                )))
                .unique_id(Some(get_int_safe(data, 0)));

            // Java: m_eventManager.fireRelationReadEvent(relation);
            if let Some(relation) = Task::add_predecessor(&task2, builder) {
                self.with_event_manager(|manager| manager.fire_relation_read_event(&relation));
            }
        }

        Ok(())
    }

    /// Extract and organize the resource data.
    ///
    /// Corresponds to Java: `private void processResourceData() throws IOException`
    fn process_resource_data(&mut self) -> MpxjResult<()> {
        // Java: DirectoryEntry rscDir = (DirectoryEntry) m_projectDir.getEntry("TBkndRsc");
        let Some(rsc_dir) = self.project_directory(RESOURCE_DIRECTORY) else {
            return Err(MpxjError::new("Entry not found: TBkndRsc"));
        };

        // Java: FixFix rscFixedData = new FixFix(196, new DocumentInputStream(((DocumentEntry) rscDir.getEntry("FixFix   0"))));
        let Some(fix_fix_data) = read_document_data(&rsc_dir, FIX_FIX_STREAM, RESOURCE_DIRECTORY)?
        else {
            return Err(MpxjError::new("Entry not found: TBkndRsc/FixFix   0"));
        };
        let rsc_fixed_data = FixFix::new(196, &fix_fix_data);

        // Java: FixDeferFix rscVarData = null;
        let mut rsc_var_data: Option<FixDeferFix> = None;

        // Java: int resources = rscFixedData.getItemCount();
        let resources = rsc_fixed_data.get_item_count();

        for loop_index in 0..resources {
            // Java: data = rscFixedData.getByteArrayValue(loop);
            let Some(data) = rsc_fixed_data.get_byte_array_value(loop_index) else {
                continue;
            };

            //
            // Validate the unique ID
            //
            // Java: id = ByteArrayHelper.getInt(data, 0);
            let id = get_int_safe(data, 0);
            if id < 1 {
                continue;
            }

            //
            // MPP files may contain blank rows: this flag indicates the current row is blank and should be ignored.
            //
            // Java: if ((data[8] & 0x01) != 0) { continue; }
            if (get_byte_safe(data, 8) & 0x01) != 0 {
                continue;
            }

            //
            // Validate whether the resource is deleted: this may be a set of flag bits like the task records, but the data seen so far
            // is not yet sufficient to support that inference, so the simple "non-zero means deleted" check is kept.
            //
            // Java: if (ByteArrayHelper.getShort(data, 164) != 0) { continue; }
            if get_short_safe(data, 164) != 0 {
                continue;
            }

            //
            // Load var data only when first needed
            //
            // Java: if (rscVarData == null) { rscVarData = new FixDeferFix(…); }
            if rsc_var_data.is_none() {
                let Some(fix_defer_data) =
                    read_document_data(&rsc_dir, FIX_DEFER_FIX_STREAM, RESOURCE_DIRECTORY)?
                else {
                    return Err(MpxjError::new("Entry not found: TBkndRsc/FixDeferFix   0"));
                };
                rsc_var_data = Some(FixDeferFix::new(&fix_defer_data));
            }
            let Some(rsc_var_data) = rsc_var_data.as_ref() else {
                continue;
            };

            // Java: rscExtData = new ExtendedData(rscVarData, getOffset(data, 192));
            let rsc_ext_data = ExtendedData::new(rsc_var_data, get_offset(data, 192));

            // Java: resource = m_file.addResource();
            let resource = self.file.borrow_mut().add_resource();

            {
                let Ok(mut resource_ref) = resource.try_borrow_mut() else {
                    // This branch cannot occur in Java (the resource has just been created, with no other borrows)
                    continue;
                };

                // Java: resource.setAccrueAt(AccrueType.getInstance(ByteArrayHelper.getShort(data, 20)));
                // (Java's `AccrueType` is non-null; falls back to `START` when it cannot be obtained)
                let accrue_at = AccrueType::get_instance(get_short_safe(data, 20));
                resource_ref.set_accrue_at(accrue_at.unwrap_or(AccrueType::Start));

                // Java: resource.setActualCost(NumberHelper.getDouble(((double) MPPUtility.getLong6(data, 114)) / 100));
                resource_ref.set_actual_cost(Some(to_money(get_long6_safe(data, 114))));
                // Java: resource.setActualOvertimeCost(NumberHelper.getDouble(((double) MPPUtility.getLong6(data, 144)) / 100));
                resource_ref.set_actual_overtime_cost(Some(to_money(get_long6_safe(data, 144))));
                // Java: resource.setActualWork(MPPUtility.getDuration(((double) MPPUtility.getLong6(data, 62)) / 100, TimeUnit.HOURS));
                resource_ref.set_actual_work(Some(MPPUtility::get_duration_double(
                    to_money(get_long6_safe(data, 62)),
                    TimeUnit::Hours,
                )));
                // Java: resource.setBaseCalendar() —— computed value; skip.

                // Java: resource.setBaselineCost(NumberHelper.getDouble(((double) MPPUtility.getLong6(data, 126)) / 100));
                resource_ref.set_baseline_cost(Some(to_money(get_long6_safe(data, 126))));
                // Java: resource.setBaselineWork(MPPUtility.getDuration(((double) MPPUtility.getLong6(data, 68)) / 100, TimeUnit.HOURS));
                resource_ref.set_baseline_work(Some(MPPUtility::get_duration_double(
                    to_money(get_long6_safe(data, 68)),
                    TimeUnit::Hours,
                )));

                // Java: resource.setCode(rscExtData.getUnicodeString(RESOURCE_CODE));
                resource_ref.set_code(rsc_ext_data.get_unicode_string(RESOURCE_CODE));
                // Java: resource.setCost(NumberHelper.getDouble(((double) MPPUtility.getLong6(data, 120)) / 100));
                resource_ref.set_cost(Some(to_money(get_long6_safe(data, 120))));
                // Java: resource.setCost(1..10, NumberHelper.getDouble(((double) rscExtData.getLong(RESOURCE_COSTn)) / 100));
                for index in 1..=10i32 {
                    let value = rsc_ext_data.get_long(RESOURCE_COST[(index - 1) as usize]);
                    resource_ref.set_cost_at(index, Some(to_money(value)));
                }

                // Java: resource.setDate(1..10, rscExtData.getTimestamp(RESOURCE_DATEn));
                for index in 1..=10i32 {
                    let value = rsc_ext_data.get_timestamp(RESOURCE_DATE[(index - 1) as usize]);
                    resource_ref.set_date_at(index, value);
                }

                // Java: resource.setDuration(1..10, MPPUtility.getDuration(rscExtData.getInt(RESOURCE_DURATIONn), MPPUtility.getDurationTimeUnits(rscExtData.getShort(RESOURCE_DURATIONn_UNITS))));
                for index in 1..=10i32 {
                    let slot = (index - 1) as usize;
                    let value = MPPUtility::get_duration_int(
                        rsc_ext_data.get_int(RESOURCE_DURATION[slot]),
                        MPPUtility::get_duration_time_units(
                            rsc_ext_data.get_short(RESOURCE_DURATION_UNITS[slot]),
                        ),
                    );
                    resource_ref.set_duration_at(index, Some(value));
                }

                // Java: resource.setEmailAddress(rscExtData.getUnicodeString(RESOURCE_EMAIL));
                resource_ref.set_email_address(rsc_ext_data.get_unicode_string(RESOURCE_EMAIL));

                // Java: resource.setFinish(1..10, rscExtData.getTimestamp(RESOURCE_FINISHn));
                for index in 1..=10i32 {
                    let value = rsc_ext_data.get_timestamp(RESOURCE_FINISH[(index - 1) as usize]);
                    resource_ref.set_finish_at(index, value);
                }

                // Java: resource.setGroup(rscExtData.getUnicodeString(RESOURCE_GROUP));
                resource_ref.set_group(rsc_ext_data.get_unicode_string(RESOURCE_GROUP));
                // Java: resource.setID(Integer.valueOf(ByteArrayHelper.getInt(data, 4)));
                resource_ref.set_id(Some(get_int_safe(data, 4)));
                // Java: resource.setInitials(rscVarData.getUnicodeString(getOffset(data, 160)));
                resource_ref.set_initials(rsc_var_data.get_unicode_string(get_offset(data, 160)));
                // Java: resource.setLinkedFields() —— computed value; skip.
                // Java: resource.setName(rscVarData.getUnicodeString(getOffset(data, 156)));
                resource_ref.set_name(rsc_var_data.get_unicode_string(get_offset(data, 156)));

                // Java: resource.setNumber(1..20, NumberHelper.getDouble(rscExtData.getDouble(RESOURCE_NUMBERn)));
                for index in 1..=20i32 {
                    let value = rsc_ext_data.get_double(RESOURCE_NUMBER[(index - 1) as usize]);
                    resource_ref.set_number_at(index, Some(value));
                }
                // Java: resource.setObjects() / setOverallocated() —— computed values; skip.

                // Java: resource.setOvertimeCost(NumberHelper.getDouble(((double) MPPUtility.getLong6(data, 138)) / 100));
                resource_ref.set_overtime_cost(Some(to_money(get_long6_safe(data, 138))));
                // Java: resource.setOvertimeWork(MPPUtility.getDuration(((double) MPPUtility.getLong6(data, 74)) / 100, TimeUnit.HOURS));
                resource_ref.set_overtime_work(Some(MPPUtility::get_duration_double(
                    to_money(get_long6_safe(data, 74)),
                    TimeUnit::Hours,
                )));
                // Java: resource.setPeakUnits(NumberHelper.getDouble(((double) ByteArrayHelper.getInt(data, 110)) / 100));
                resource_ref.set_peak_units(Some(f64::from(get_int_safe(data, 110)) / 100.0));
                // Java: resource.setPercentageWorkComplete() —— computed value; skip.
                // Java: resource.setRegularWork(MPPUtility.getDuration(((double) MPPUtility.getLong6(data, 92)) / 100, TimeUnit.HOURS));
                resource_ref.set_regular_work(Some(MPPUtility::get_duration_double(
                    to_money(get_long6_safe(data, 92)),
                    TimeUnit::Hours,
                )));
                // Java: resource.setRemainingCost(NumberHelper.getDouble(((double) MPPUtility.getLong6(data, 132)) / 100));
                resource_ref.set_remaining_cost(Some(to_money(get_long6_safe(data, 132))));
                // Java: resource.setRemainingOvertimeCost(NumberHelper.getDouble(((double) MPPUtility.getLong6(data, 150)) / 100));
                resource_ref.set_remaining_overtime_cost(Some(to_money(get_long6_safe(data, 150))));
                // Java: resource.setRemainingWork(MPPUtility.getDuration(((double) MPPUtility.getLong6(data, 86)) / 100, TimeUnit.HOURS));
                resource_ref.set_remaining_work(Some(MPPUtility::get_duration_double(
                    to_money(get_long6_safe(data, 86)),
                    TimeUnit::Hours,
                )));

                // Java: resource.setStart(1..10, rscExtData.getTimestamp(RESOURCE_STARTn));
                for index in 1..=10i32 {
                    let value = rsc_ext_data.get_timestamp(RESOURCE_START[(index - 1) as usize]);
                    resource_ref.set_start_at(index, value);
                }

                // Java: resource.setText(1..30, rscExtData.getUnicodeString(RESOURCE_TEXTn));
                for index in 1..=30i32 {
                    let value = rsc_ext_data.get_unicode_string(RESOURCE_TEXT[(index - 1) as usize]);
                    resource_ref.set_text_at(index, value);
                }

                // Java: resource.setUniqueID(Integer.valueOf(id));
                resource_ref.set_unique_id(Some(id));
                // Java: resource.setWork(MPPUtility.getDuration(((double) MPPUtility.getLong6(data, 56)) / 100, TimeUnit.HOURS));
                resource_ref.set_work(Some(MPPUtility::get_duration_double(
                    to_money(get_long6_safe(data, 56)),
                    TimeUnit::Hours,
                )));

                //
                // Associate the resource calendar
                //
                // Java: calendar = m_calendarMap.get(Integer.valueOf(ByteArrayHelper.getInt(data, 24)));
                //       resource.setCalendar(calendar);
                let calendar_id = get_int_safe(data, 24);
                let calendar = self.calendar_map.get(&calendar_id);
                resource_ref.set_calendar(calendar);

                //
                // Resource notes
                //
                // Java: notes = rscExtData.getString(RESOURCE_NOTES);
                //       if (notes != null) { resource.setNotesObject(new RtfNotes(notes)); }
                if let Some(notes) = rsc_ext_data.get_string(RESOURCE_NOTES) {
                    let notes = normalize_mpp8_notes(notes);
                    let rtf_notes = RtfNotes::new(Some(notes));
                    resource_ref.set_notes_object(Some(&rtf_notes as &dyn Notes));
                }

                //
                // Rate tables
                //
                // Java:
                // Number costPerUse = NumberHelper.getDouble(((double) MPPUtility.getLong6(data, 80)) / 100);
                // Rate standardRate = new Rate(MPPUtility.getDouble(data, 36), TimeUnit.HOURS);
                // Rate overtimeRate = new Rate(MPPUtility.getDouble(data, 44), TimeUnit.HOURS);
                // CostRateTable costRateTable = new CostRateTable();
                // costRateTable.add(new CostRateTableEntry(LocalDateTimeHelper.START_DATE_NA,
                //     LocalDateTimeHelper.END_DATE_NA, costPerUse, standardRate, overtimeRate));
                // resource.setCostRateTable(0, costRateTable);
                //
                let cost_per_use = to_money(get_long6_safe(data, 80));
                let standard_rate = Rate::new(MPPUtility::get_double(data, 36), TimeUnit::Hours);
                let overtime_rate = Rate::new(MPPUtility::get_double(data, 44), TimeUnit::Hours);

                let mut cost_rate_table = CostRateTable::new();
                cost_rate_table.add(CostRateTableEntry::new(
                    Some(LocalDateTimeHelper::start_date_na()),
                    Some(LocalDateTimeHelper::end_date_na()),
                    Some(cost_per_use),
                    &[standard_rate, overtime_rate],
                ));
                resource_ref.set_cost_rate_table(0, cost_rate_table);

                //
                // Availability table
                //
                // Java:
                // LocalDateTime availableFrom = MPPUtility.getTimestamp(data, 28);
                // LocalDateTime availableTo = MPPUtility.getTimestamp(data, 32);
                // availableFrom = availableFrom == null ? LocalDateTimeHelper.START_DATE_NA : availableFrom;
                // availableTo = availableTo == null ? LocalDateTimeHelper.END_DATE_NA : availableTo;
                // resource.getAvailability().add(new Availability(availableFrom, availableTo,
                //     NumberHelper.getDouble(((double) ByteArrayHelper.getInt(data, 52)) / 100)));
                //
                // Note: `NumberHelper.getDouble(double)` merely boxes the primitive `double` (see the file header difference table),
                //       so `Some(f64)` is passed directly here; the write entry point is `Resource::add_availability`
                //       (Java appends directly to `resource.getAvailability()`).
                let available_from = MPPUtility::get_timestamp(data, 28)
                    .unwrap_or_else(LocalDateTimeHelper::start_date_na);
                let available_to = MPPUtility::get_timestamp(data, 32)
                    .unwrap_or_else(LocalDateTimeHelper::end_date_na);
                resource_ref.add_availability(Availability::new(
                    Some(available_from),
                    Some(available_to),
                    Some(f64::from(get_int_safe(data, 52)) / 100.0),
                ));
            }

            // Java: m_eventManager.fireResourceReadEvent(resource);
            self.with_event_manager(|manager| manager.fire_resource_read_event(&resource));
        }

        Ok(())
    }

    /// Extract and organize the resource assignment data.
    ///
    /// Corresponds to Java: `private void processAssignmentData() throws IOException`
    fn process_assignment_data(&mut self) -> MpxjResult<()> {
        // Java: DirectoryEntry assnDir = (DirectoryEntry) m_projectDir.getEntry("TBkndAssn");
        let Some(assn_dir) = self.project_directory(ASSIGNMENT_DIRECTORY) else {
            return Err(MpxjError::new("Entry not found: TBkndAssn"));
        };

        // Java: FixFix assnFixedData = new FixFix(204, new DocumentInputStream(((DocumentEntry) assnDir.getEntry("FixFix   0"))));
        let Some(fix_fix_data) =
            read_document_data(&assn_dir, FIX_FIX_STREAM, ASSIGNMENT_DIRECTORY)?
        else {
            return Err(MpxjError::new("Entry not found: TBkndAssn/FixFix   0"));
        };
        let mut assn_fixed_data = FixFix::new(204, &fix_fix_data);

        // Java:
        // if (assnFixedData.getDiff() != 0 || (assnFixedData.getSize() % 238 == 0 && !testAssignmentTasks(assnFixedData)))
        // { assnFixedData = new FixFix(238, …); }
        if assn_fixed_data.get_diff() != 0
            || (assn_fixed_data.get_size() % 238 == 0
                && !self.test_assignment_tasks(&assn_fixed_data))
        {
            assn_fixed_data = FixFix::new(238, &fix_fix_data);
        }

        // Java: int count = assnFixedData.getItemCount();
        let count = assn_fixed_data.get_item_count();

        // Java: FixDeferFix assnVarData = null;
        let mut assn_var_data: Option<FixDeferFix> = None;

        for loop_index in 0..count {
            // Java: if (assnVarData == null) { assnVarData = new FixDeferFix(…); }
            if assn_var_data.is_none() {
                let Some(fix_defer_data) =
                    read_document_data(&assn_dir, FIX_DEFER_FIX_STREAM, ASSIGNMENT_DIRECTORY)?
                else {
                    return Err(MpxjError::new("Entry not found: TBkndAssn/FixDeferFix   0"));
                };
                assn_var_data = Some(FixDeferFix::new(&fix_defer_data));
            }

            // Java: byte[] data = assnFixedData.getByteArrayValue(loop);
            let Some(data) = assn_fixed_data.get_byte_array_value(loop_index) else {
                continue;
            };

            //
            // Check whether the deleted flag is set
            //
            // Java: if (MPPUtility.getByte(data, 168) != 0x02)
            if MPPUtility::get_byte(data, 168) == 0x02 {
                continue;
            }

            // Java: Task task = m_file.getTaskByUniqueID(Integer.valueOf(ByteArrayHelper.getInt(data, 16)));
            //       Resource resource = m_file.getResourceByUniqueID(Integer.valueOf(ByteArrayHelper.getInt(data, 20)));
            let task = self
                .file
                .borrow()
                .get_task_by_unique_id(Some(get_int_safe(data, 16)));
            let resource = self
                .file
                .borrow()
                .get_resource_by_unique_id(Some(get_int_safe(data, 20)));

            let (Some(task), Some(resource)) = (task, resource) else {
                continue;
            };

            // Java: ResourceAssignment assignment = task.getExistingResourceAssignment(resource);
            //       if (assignment == null) { assignment = task.addResourceAssignment(resource); }
            let existing = {
                let Ok(task_ref) = task.try_borrow() else {
                    continue;
                };
                task_ref.get_existing_resource_assignment(Some(&resource))
            };

            let assignment = match existing {
                Some(assignment) => assignment,
                None => {
                    let created = match task.try_borrow_mut() {
                        Ok(mut task_ref) => task_ref.add_resource_assignment(Some(&resource)),
                        // This branch cannot occur in Java (the task is not borrowed)
                        Err(_) => None,
                    };
                    match created {
                        Some(assignment) => assignment,
                        None => continue,
                    }
                }
            };

            {
                let Ok(mut assignment_ref) = assignment.try_borrow_mut() else {
                    continue;
                };

                // Java: assignment.setActualCost(NumberHelper.getDouble(((double) MPPUtility.getLong6(data, 138)) / 100));
                assignment_ref.set_actual_cost(Some(to_money(get_long6_safe(data, 138))));
                // Java: assignment.setActualWork(MPPUtility.getDuration(((double) MPPUtility.getLong6(data, 96)) / 100, TimeUnit.HOURS));
                assignment_ref.set_actual_work(Some(MPPUtility::get_duration_double(
                    to_money(get_long6_safe(data, 96)),
                    TimeUnit::Hours,
                )));
                // Java: assignment.setCost(NumberHelper.getDouble(((double) MPPUtility.getLong6(data, 132)) / 100));
                assignment_ref.set_cost(Some(to_money(get_long6_safe(data, 132))));
                // Java: assignment.setDelay() —— unclear which MSP field this corresponds to; skip.
                // Java: assignment.setFinish(MPPUtility.getTimestamp(data, 28));
                assignment_ref.set_finish(MPPUtility::get_timestamp(data, 28));
                // Java: assignment.setOvertimeWork(MPPUtility.getDuration(((double) MPPUtility.getLong6(data, 90)) / 100, TimeUnit.HOURS));
                assignment_ref.set_overtime_work(Some(MPPUtility::get_duration_double(
                    to_money(get_long6_safe(data, 90)),
                    TimeUnit::Hours,
                )));
                // Java: assignment.setPlannedCost() / setPlannedWork() —— unclear which field this corresponds to; skip.
                // Java: assignment.setRemainingWork(MPPUtility.getDuration(((double) MPPUtility.getLong6(data, 114)) / 100, TimeUnit.HOURS));
                assignment_ref.set_remaining_work(Some(MPPUtility::get_duration_double(
                    to_money(get_long6_safe(data, 114)),
                    TimeUnit::Hours,
                )));
                // Java: assignment.setStart(MPPUtility.getTimestamp(data, 24));
                assignment_ref.set_start(MPPUtility::get_timestamp(data, 24));
                // Java: assignment.setUniqueID(Integer.valueOf(ByteArrayHelper.getInt(data, 0)));
                assignment_ref.set_unique_id(Some(get_int_safe(data, 0)));
                // Java: assignment.setUnits(Double.valueOf(((double) ByteArrayHelper.getShort(data, 80)) / 100));
                assignment_ref.set_units(Some(f64::from(get_short_safe(data, 80)) / 100.0));
                // Java: assignment.setWork(MPPUtility.getDuration(((double) MPPUtility.getLong6(data, 84)) / 100, TimeUnit.HOURS));
                assignment_ref.set_work(Some(MPPUtility::get_duration_double(
                    to_money(get_long6_safe(data, 84)),
                    TimeUnit::Hours,
                )));
            }

            // Java: m_eventManager.fireAssignmentReadEvent(assignment);
            self.with_event_manager(|manager| manager.fire_assignment_read_event(&assignment));
        }

        Ok(())
    }

    /// MPP8's task assignment data block has two lengths: 204 and 238 bytes. In most cases, dividing the whole block length by these two values
    /// is enough to determine which one to use (the one giving a remainder of 0).
    ///
    /// However, there are cases where the whole block length is divisible by both; in that case this method makes an "empirical" judgment: from observation,
    /// assignment data is always associated with a valid resource or task; if both are invalid, the record length is deemed to have been chosen incorrectly.
    ///
    /// As noted above, this is only an empirical judgment and may well be wrong in some cases; a more reliable way of deciding must still be sought.
    ///
    /// Corresponds to Java: `private boolean testAssignmentTasks(FixFix assnFixedData)`
    fn test_assignment_tasks(&self, assn_fixed_data: &FixFix) -> bool {
        // Java: boolean result = true;
        let mut result = true;
        let count = assn_fixed_data.get_item_count();

        for loop_index in 0..count {
            // Java: data = assnFixedData.getByteArrayValue(loop);
            let Some(data) = assn_fixed_data.get_byte_array_value(loop_index) else {
                continue;
            };

            // Java: task = m_file.getTaskByUniqueID(Integer.valueOf(ByteArrayHelper.getInt(data, 16)));
            //       resource = m_file.getResourceByUniqueID(Integer.valueOf(ByteArrayHelper.getInt(data, 20)));
            let task = self
                .file
                .borrow()
                .get_task_by_unique_id(Some(get_int_safe(data, 16)));
            let resource = self
                .file
                .borrow()
                .get_resource_by_unique_id(Some(get_int_safe(data, 20)));

            if task.is_none() && resource.is_none() {
                result = false;
                break;
            }
        }

        // Java: return (result);
        result
    }

    /// Extract view data from the MPP file.
    ///
    /// Corresponds to Java: `private void processViewData() throws IOException`
    fn process_view_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CV_iew"))
        let Some(dir) = self
            .view_dir
            .as_ref()
            .and_then(|dir| dir.get_directory(VIEW_DATA_DIRECTORY))
        else {
            return Ok(());
        };

        // Java: FixFix ff = new FixFix(138, new DocumentInputStream(((DocumentEntry) dir.getEntry("FixFix   0"))));
        let Some(fix_fix_data) = read_document_data(&dir, FIX_FIX_STREAM, VIEW_DATA_DIRECTORY)? else {
            return Ok(());
        };
        let ff = FixFix::new(138, &fix_fix_data);
        let items = ff.get_item_count();

        for loop_index in 0..items {
            // Java: data = ff.getByteArrayValue(loop);
            let Some(data) = ff.get_byte_array_value(loop_index) else {
                continue;
            };
            // Java: view = new View8(m_file, data); m_file.getViews().add(view);
            let view: Rc<RefCell<dyn View>> = Rc::new(RefCell::new(View8::new(&self.file, data)));
            self.file.borrow_mut().get_views_mut().add(view);
        }

        Ok(())
    }

    /// Extract table data from the MPP file.
    ///
    /// Corresponds to Java: `private void processTableData() throws IOException`
    fn process_table_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CTable"))
        let Some(dir) = self
            .view_dir
            .as_ref()
            .and_then(|dir| dir.get_directory(TABLE_DIRECTORY))
        else {
            return Ok(());
        };

        // Java: FixFix ff = new FixFix(126, new DocumentInputStream(((DocumentEntry) dir.getEntry("FixFix   0"))));
        let Some(fix_fix_data) = read_document_data(&dir, FIX_FIX_STREAM, TABLE_DIRECTORY)? else {
            return Ok(());
        };
        let ff = FixFix::new(TABLE_ITEM_SIZE, &fix_fix_data);

        // Java: FixDeferFix fdf = new FixDeferFix(new DocumentInputStream(((DocumentEntry) dir.getEntry("FixDeferFix   0"))));
        let Some(fix_defer_data) = read_document_data(&dir, FIX_DEFER_FIX_STREAM, TABLE_DIRECTORY)?
        else {
            return Ok(());
        };
        let fdf = FixDeferFix::new(&fix_defer_data);

        let items = ff.get_item_count();

        for loop_index in 0..items {
            // Java: byte[] data = ff.getByteArrayValue(loop);
            let Some(data) = ff.get_byte_array_value(loop_index) else {
                continue;
            };

            // Java: Table table = new Table();
            let table = Rc::new(RefCell::new(Table::new()));

            // Java: table.setID(ByteArrayHelper.getInt(data, 0));
            table.borrow_mut().set_id(get_int_safe(data, 0));

            // Java: String name = MPPUtility.getUnicodeString(data, 4);
            let mut name = MPPUtility::get_unicode_string(data, 4);
            // Java: if (name.indexOf('&') != -1) { … StringBuilder strips all '&' … name = sb.toString(); }
            if name.contains('&') {
                name = name.chars().filter(|c| *c != '&').collect();
            }

            // Java: table.setName(MPPUtility.removeAmpersands(name));
            table
                .borrow_mut()
                .set_name(MPPUtility::remove_ampersands(Some(name.as_str())));

            // Java: container.add(table);
            self.file
                .borrow_mut()
                .get_tables_mut()
                .add(Rc::clone(&table));

            // Java: byte[] extendedData = fdf.getByteArray(getOffset(data, 122));
            if let Some(extended_data) = fdf.get_byte_array(get_offset(data, 122)) {
                // Java: byte[] columnData = fdf.getByteArray(getOffset(extendedData, 8));
                let column_data = fdf.get_byte_array(get_offset(&extended_data, 8));
                // Java: processColumnData(table, columnData); (Java would NPE when `columnData` is null)
                self.process_column_data(&table, column_data.as_deref());
            }
        }

        Ok(())
    }

    /// Process the column data associated with the current table.
    ///
    /// Corresponds to Java: `private void processColumnData(Table table, byte[] data)`
    ///
    /// Java's `data` is allowed to be `null` (in which case `ByteArrayHelper.getShort` throws an NPE); the Rust side
    /// accepts it with `Option` and returns directly.
    fn process_column_data(&self, table: &Rc<RefCell<Table>>, data: Option<&[u8]>) {
        // Java: int columnCount = ByteArrayHelper.getShort(data, 4) + 1;
        let Some(data) = data else {
            return;
        };
        let column_count = get_short_safe(data, 4) + 1;

        // Java: int index = 8;
        let mut index = 8usize;

        for loop_index in 0..column_count {
            // Java: Column column = new Column(m_file);
            let mut column = Column::new(Rc::downgrade(&self.file));

            // Java: if (loop == 0) { table.setResourceFlag(ByteArrayHelper.getShort(data, index) == 0); }
            if loop_index == 0 {
                if let Ok(mut table_ref) = table.try_borrow_mut() {
                    table_ref.set_resource_flag(get_short_safe(data, index) == 0);
                }
            }

            // Java: column.setFieldType(FieldTypeHelper.getInstance(m_file, ByteArrayHelper.getInt(data, index)));
            let field_type = FieldTypeHelper::get_instance(&self.file, get_int_safe(data, index));
            column.set_field_type(field_type.as_ref().map(|field| field.to_field_key()));

            // Java: column.setWidth(MPPUtility.getByte(data, index + 4));
            column.set_width(MPPUtility::get_byte(data, index + 4));

            // Java: columnTitleOffset = ByteArrayHelper.getShort(data, index + 6);
            //       if (columnTitleOffset != 0) { column.setTitle(MPPUtility.getUnicodeString(data, columnTitleOffset)); }
            let column_title_offset = get_short_safe(data, index + 6);
            if column_title_offset != 0 {
                let title = MPPUtility::get_unicode_string(data, column_title_offset as usize);
                column.set_title(Some(title));
            }

            // Java:
            // alignment = MPPUtility.getByte(data, index + 8);
            // if (alignment == 32) { column.setAlignTitle(Column.ALIGN_LEFT); }
            // else if (alignment == 33) { column.setAlignTitle(Column.ALIGN_CENTER); }
            // else { column.setAlignTitle(Column.ALIGN_RIGHT); }
            column.set_align_title(mpp8_column_alignment(MPPUtility::get_byte(data, index + 8)));

            // Java: same as above, replacing with index + 10 and setAlignData
            column.set_align_data(mpp8_column_alignment(MPPUtility::get_byte(data, index + 10)));

            // Java: table.addColumn(column);
            if let Ok(mut table_ref) = table.try_borrow_mut() {
                table_ref.add_column(column);
            }

            // Java: index += 12;
            index += COLUMN_BLOCK_SIZE;
        }
    }

    // -----------------------------------------------------------------------
    // Internal helpers (Rust-specific)
    // -----------------------------------------------------------------------

    /// Get the sub-storage under the project directory (`   1`).
    ///
    /// Rust-specific: Java directly does `(DirectoryEntry) m_projectDir.getEntry(name)`;
    /// here an **owning** handle is returned to avoid a long-lived borrow of `self` (otherwise subsequent
    /// mutable borrows of `self.calendar_map` would conflict in Rust).
    fn project_directory(&self, name: &str) -> Option<DirectoryEntry> {
        let project_dir = self.project_dir.as_ref()?;
        project_dir.get_directory(name)
    }

    /// Read the "duration adjusted according to project settings".
    ///
    /// Corresponds to Java: `MPPUtility.getAdjustedDuration(m_file.getProjectProperties(), duration, timeUnit)`
    fn adjusted_duration(&self, duration: i32, time_unit: TimeUnit) -> Option<Duration> {
        let Ok(file) = self.file.try_borrow() else {
            // This branch cannot occur in Java (the file reference is stable)
            return None;
        };

        MPPUtility::get_adjusted_duration(file.get_project_properties(), duration, time_unit)
    }

    /// Get the project's default duration unit.
    ///
    /// Corresponds to Java: `m_file.getProjectProperties().getDefaultDurationUnits()`
    fn default_duration_units(&self) -> TimeUnit {
        match self.file.try_borrow() {
            Ok(file) => file.get_project_properties().get_default_duration_units(),
            // This branch cannot occur in Java (the file reference is stable); `TimeUnit::Days` matches
            // the default value when `ProjectProperties` is unset
            Err(_) => TimeUnit::Days,
        }
    }

    /// Set the automatic WBS generation flag.
    ///
    /// Corresponds to Java: `m_file.getProjectConfig().setAutoWBS(autoWBS)`
    ///
    /// `ProjectFile::get_project_config` returns a read-only `Ref`, so a mutable borrow is obtained via `ProjectContext`.
    fn set_auto_wbs(&self, auto_wbs: bool) {
        let Ok(file) = self.file.try_borrow() else {
            return;
        };
        let context = file.get_project_context();
        drop(file);

        if let Ok(mut context_ref) = context.try_borrow_mut() {
            context_ref.get_project_config_mut().set_auto_wbs(auto_wbs);
        }
    }

    /// Get the calendar named `Standard`, creating a new default base calendar if none exists.
    ///
    /// Corresponds to Java: `m_file.getCalendars().findOrCreateDefaultCalendar()`
    ///
    /// `ProjectFile::get_calendars` returns a read-only `Ref`, so a mutable borrow is obtained via `ProjectContext`.
    fn find_or_create_default_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        let file = self.file.try_borrow().ok()?;
        let context = file.get_project_context();
        drop(file);

        let mut context_ref = context.try_borrow_mut().ok()?;
        context_ref.find_or_create_default_calendar()
    }

    /// Fire a project event.
    ///
    /// Corresponds to Java: `m_eventManager.fireXxxEvent(...)`
    ///
    /// Rust-specific: Java's `m_eventManager` comes from `m_file.getEventManager()`; on the Rust side
    /// the `EventManager` is attached to `ProjectContext` and `fire*` needs `&mut`, so it is obtained on demand via a
    /// mutable borrow of the context (dispatch is skipped on a borrow conflict, which does not happen in Java).
    fn with_event_manager(&self, fire: impl FnOnce(&mut EventManager)) {
        let Ok(file) = self.file.try_borrow() else {
            return;
        };
        let context = file.get_project_context();
        drop(file);

        if let Ok(mut context_ref) = context.try_borrow_mut() {
            fire(context_ref.get_event_manager_mut());
        }
    }
}

// ---------------------------------------------------------------------------
// Module-level helpers
// ---------------------------------------------------------------------------

/// Name of the storage holding the project data.
///
/// Corresponds to Java: `root.getEntry("   1")`
const PROJECT_DIRECTORY: &str = "   1";

/// Name of the storage holding the presentation-layer data.
///
/// Corresponds to Java: `root.getEntry("   2")`
const VIEW_DIRECTORY: &str = "   2";

/// Name of the properties stream.
///
/// Corresponds to Java: `dir.getEntry("Props")`
const PROPS_STREAM: &str = "Props";

/// Name of the fixed-length data stream.
///
/// Corresponds to Java: `dir.getEntry("FixFix   0")`
const FIX_FIX_STREAM: &str = "FixFix   0";

/// Name of the variable-length data stream.
///
/// Corresponds to Java: `dir.getEntry("FixDeferFix   0")`
const FIX_DEFER_FIX_STREAM: &str = "FixDeferFix   0";

/// Name of the calendar data storage.
///
/// Corresponds to Java: `m_projectDir.getEntry("TBkndCal")`
const CALENDAR_DIRECTORY: &str = "TBkndCal";
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
/// Name of the constraint data storage.
///
/// Corresponds to Java: `m_projectDir.getEntry("TBkndCons")`
const CONSTRAINT_DIRECTORY: &str = "TBkndCons";
/// Name of the view data storage.
///
/// Corresponds to Java: `m_viewDir.getEntry("CV_iew")`
const VIEW_DATA_DIRECTORY: &str = "CV_iew";
/// Name of the table data storage.
///
/// Corresponds to Java: `m_viewDir.getEntry("CTable")`
const TABLE_DIRECTORY: &str = "CTable";

/// Read the contents of a stream under a storage.
///
/// Corresponds to Java: `new DocumentInputStream((DocumentEntry) dir.getEntry(name))`
///
/// Returns `Err` when the stream does not exist (Java's `getEntry` throws `FileNotFoundException`);
/// the caller decides whether to report an error or treat it as an "optional stream"; `owner` is only used to build a readable error message.
fn read_document_data(
    directory: &DirectoryEntry,
    name: &str,
    owner: &str,
) -> MpxjResult<Option<Rc<Vec<u8>>>> {
    if let Some(document) = directory.get_document(name) {
        return Ok(Some(document.get_data()));
    }

    Err(MpxjError::new(format!("Entry not found: {owner}/{name}")))
}

/// Get a byte from the fixed data block.
///
/// `FixFix` guarantees a complete record length, so out-of-bounds can only occur when the caller passes a wrong value;
/// out of bounds returns 0 (Java throws `ArrayIndexOutOfBoundsException`).
fn get_byte_safe(data: &[u8], offset: usize) -> u8 {
    data.get(offset).copied().unwrap_or(0)
}

/// Get a six-byte little-endian long integer from the fixed data block.
///
/// Corresponds to Java: `MPPUtility.getLong6(data, offset)` (out of bounds returns 0).
fn get_long6_safe(data: &[u8], offset: usize) -> i64 {
    MPPUtility::get_long6(data, offset)
}

/// Convert a raw "hundredth of a currency unit" value into a currency value.
///
/// Corresponds to Java: `NumberHelper.getDouble(((double) raw) / 100)`
fn to_money(raw: i64) -> f64 {
    raw as f64 / 100.0
}

/// Convert an offset in the fixed data block into an offset in var data.
///
/// Corresponds to Java: `private int getOffset(byte[] data, int offset)`
fn get_offset(data: &[u8], offset: usize) -> i32 {
    // Java: return (-1 - ByteArrayHelper.getInt(data, offset));
    -1 - get_int_safe(data, offset)
}

/// Construct a `LocalTime` (used for default working hours).
///
/// Corresponds to Java: `LocalTime.of(hour, minute)`
fn time(hour: u32, minute: u32) -> Option<chrono::NaiveTime> {
    chrono::NaiveTime::from_hms_opt(hour, minute, 0)
}

/// Write the calendar name (skipped on borrow failure, which does not happen in Java).
fn set_calendar_name(calendar: &Rc<RefCell<ProjectCalendar>>, name: Option<String>) {
    if let Ok(mut calendar_ref) = calendar.try_borrow_mut() {
        calendar_ref.set_name(name);
    }
}

/// Write the calendar unique ID (skipped on borrow failure, which does not happen in Java).
fn set_calendar_unique_id(calendar: &Rc<RefCell<ProjectCalendar>>, unique_id: i32) {
    if let Ok(mut calendar_ref) = calendar.try_borrow_mut() {
        calendar_ref.set_unique_id(Some(unique_id));
    }
}

/// Read the calendar's "is working day" flag (treated as false on borrow failure).
fn is_working_day(calendar: &Rc<RefCell<ProjectCalendar>>, day: Weekday) -> bool {
    match calendar.try_borrow() {
        Ok(calendar_ref) => calendar_ref.is_working_day(day),
        Err(_) => false,
    }
}

/// Set the calendar's "is working day" flag (skipped on borrow failure, which does not happen in Java).
fn set_working_day(calendar: &Rc<RefCell<ProjectCalendar>>, day: Weekday, working: bool) {
    if let Ok(mut calendar_ref) = calendar.try_borrow_mut() {
        calendar_ref.set_working_day(day, working);
    }
}

/// Append working hours to a day of the calendar.
///
/// Corresponds to Java: `hours = cal.addCalendarHours(day); hours.add(...)`
fn add_calendar_hours(
    calendar: &Rc<RefCell<ProjectCalendar>>,
    day: Weekday,
    ranges: &[LocalTimeRange],
) {
    if let Ok(mut calendar_ref) = calendar.try_borrow_mut() {
        let hours: &mut ProjectCalendarHours = calendar_ref.add_calendar_hours(day);
        for range in ranges {
            hours.add(*range);
        }
    }
}

/// Determine whether the "base calendar ID" of the calendar data block is not greater than 0.
///
/// Rust-specific: in `processCalendarData`, Java uses `baseCalendarID` as a local variable inside the loop;
/// this file extracts it into a helper function, so it is re-read here from the fixed data block (offset 4).
fn base_calendar_is_base(data: &[u8]) -> bool {
    get_int_safe(data, 4) <= 0
}

/// Map MPP8's alignment codes to [`Column`] constants.
///
/// Corresponds to Java: in `processColumnData`
/// `if (alignment == 32) { ALIGN_LEFT } else if (alignment == 33) { ALIGN_CENTER } else { ALIGN_RIGHT }`
/// (MPP8 stores ASCII spaces `0x20` and `0x21`, unlike MPP12/14's `& 0x0F` test).
fn mpp8_column_alignment(alignment: i32) -> i32 {
    match alignment {
        32 => Column::ALIGN_LEFT,
        33 => Column::ALIGN_CENTER,
        _ => Column::ALIGN_RIGHT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::{Cursor, Write};

    use cfb::CompoundFile;
    use chrono::{NaiveDate, NaiveDateTime};

    use crate::common::mpp_field_type::MppFieldType;
    use crate::field_key::FieldKey;
    use crate::mpp::ole::PoifsFileSystem;
    use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
    use crate::task_field::TaskField;

    // -----------------------------------------------------------------------
    // Compound document construction helpers
    // -----------------------------------------------------------------------

    /// Encode `value` as NUL-terminated UTF-16LE bytes.
    fn utf16(value: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in value.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00]);
        data
    }

    /// Write a little-endian `i32`.
    fn put_int(data: &mut [u8], offset: usize, value: i32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// Write a little-endian `i16`.
    fn put_short(data: &mut [u8], offset: usize, value: i16) {
        data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    /// Write a little-endian six-byte long integer.
    fn put_long6(data: &mut [u8], offset: usize, value: i64) {
        for index in 0..6 {
            data[offset + index] = ((value >> (index * 8)) & 0xFF) as u8;
        }
    }

    /// Write a little-endian `f64`.
    fn put_double(data: &mut [u8], offset: usize, value: f64) {
        data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    /// Write a "timestamp": the low two bytes are the time (0), and the high two bytes are the number of days since 1983-12-31.
    fn put_timestamp(data: &mut [u8], offset: usize, days: i16) {
        put_short(data, offset, 0);
        put_short(data, offset + 2, days);
    }

    /// Write an offset pointing into var data (Java's `-1 - offset` convention).
    fn put_var_offset(data: &mut [u8], offset: usize, var_offset: i32) {
        put_int(data, offset, -1 - var_offset);
    }

    /// Read a little-endian `i32` (for test assertions only).
    fn read_short(data: &[u8], offset: usize) -> i16 {
        i16::from_le_bytes([data[offset], data[offset + 1]])
    }

    /// Construct the byte contents of the `Props8` stream.
    ///
    /// See the `crate::mpp::props8` file header for the layout; the entries generated here always take
    /// the "byte2 == 64 → length taken from attrib1" path.
    fn props8_stream(entries: &[(i32, Vec<u8>)]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&1024i32.to_le_bytes());
        data.extend_from_slice(&1024i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(entries.len() as i16).to_le_bytes());
        data.extend_from_slice(&0i16.to_le_bytes());

        for (key, value) in entries {
            data.extend_from_slice(&(value.len() as i32).to_le_bytes());
            let bytes = key.to_le_bytes();
            assert_eq!(bytes[2], 64, "test PropsKey must satisfy byte2 == 64");
            data.extend_from_slice(&bytes);
            data.extend_from_slice(&0i32.to_le_bytes());
            data.extend_from_slice(value);
            if value.len() % 2 != 0 {
                data.push(0);
            }
        }

        data
    }

    /// Allocator for var data (`FixDeferFix`): appends data items sequentially and returns their offset.
    struct VarAllocator {
        buffer: Vec<u8>,
    }

    impl VarAllocator {
        fn new() -> Self {
            // The first 8 bytes are reserved so that the first data item's offset is greater than 0 (`FixDeferFix::get_byte_array` requires offset > 0)
            Self {
                buffer: vec![0u8; 8],
            }
        }

        /// Append a data item and return its offset in var data.
        fn push(&mut self, payload: &[u8]) -> i32 {
            let offset = self.buffer.len();
            self.write_item(offset, payload);
            offset as i32
        }

        /// Append a NUL-terminated UTF-16LE string item.
        fn push_string(&mut self, value: &str) -> i32 {
            let payload = utf16(value);
            self.push(&payload)
        }

        /// Write a data item according to `FixDeferFix`'s block-chain layout.
        ///
        /// First block: `[next block offset:4][item length:4][data (up to 28 bytes)]`,
        /// Subsequent blocks: `[next block offset:4][data (up to 32 bytes)]`.
        fn write_item(&mut self, offset: usize, payload: &[u8]) {
            let size = payload.len();
            let block_count = if size <= 28 {
                1
            } else {
                1 + (size - 28).div_ceil(32)
            };

            let end = offset + block_count * 36;
            if self.buffer.len() < end {
                self.buffer.resize(end, 0);
            }

            let first = size.min(28);
            let next = if block_count == 1 { -1 } else { (offset + 36) as i32 };
            self.buffer[offset..offset + 4].copy_from_slice(&next.to_le_bytes());
            self.buffer[offset + 4..offset + 8].copy_from_slice(&(size as i32).to_le_bytes());
            self.buffer[offset + 8..offset + 8 + first].copy_from_slice(&payload[..first]);

            let mut written = first;
            let mut block = 1;
            while written < size {
                let block_offset = offset + block * 36;
                let remaining = size - written;
                let chunk = remaining.min(32);
                let next = if remaining <= 32 {
                    -1
                } else {
                    (block_offset + 36) as i32
                };

                self.buffer[block_offset..block_offset + 4].copy_from_slice(&next.to_le_bytes());
                self.buffer[block_offset + 4..block_offset + 4 + chunk]
                    .copy_from_slice(&payload[written..written + chunk]);

                written += chunk;
                block += 1;
            }
        }
    }

    /// Construct the contents of an "extended data block": a `[length:4][type:4][data]` sequence.
    fn extended_block(items: &[(i32, Vec<u8>)]) -> Vec<u8> {
        let mut result = Vec::new();
        for (item_type, payload) in items {
            result.extend_from_slice(&(payload.len() as i32).to_le_bytes());
            result.extend_from_slice(&item_type.to_le_bytes());
            result.extend_from_slice(payload);
        }
        result
    }

    /// Construct var data while also extracting the "fixed record" returned by the closure.
    fn build_var_data<T>(build: impl FnOnce(&mut VarAllocator) -> T) -> (Vec<u8>, T) {
        let mut allocator = VarAllocator::new();
        let extra = build(&mut allocator);
        (allocator.buffer, extra)
    }

    /// Construct a 316-byte task fixed record (the MPP8 task record length).
    fn task_record(unique_id: i32, id: i32, name_offset: i32, ext_offset: i32) -> Vec<u8> {
        let mut data = vec![0u8; 316];

        put_int(&mut data, 0, unique_id);
        put_int(&mut data, 4, id);
        data[12] |= 0x01; // milestone
        data[13] |= 0x02; // marked
        data[15] |= 0x04; // rollup
        data[16] |= 0x01; // hide bar
        data[17] |= 0x08; // effort driven
        data[19] |= 0x18; // level assignments + leveling can split

        put_timestamp(&mut data, 20, 100); // finish（= earlyFinish）
        put_timestamp(&mut data, 24, 101); // lateStart
        put_timestamp(&mut data, 32, 102); // resume
        put_short(&mut data, 48, 1); // outline level
        put_int(&mut data, 68, 4800); // duration (1 day, 480 minutes/day)
        put_short(&mut data, 72, 7); // duration units = days
        put_int(&mut data, 74, 2400); // actual duration
        put_int(&mut data, 78, 2400); // remaining duration
        put_int(&mut data, 82, 4800); // baseline duration
        put_short(&mut data, 88, 2); // constraint type = must start on
        put_int(&mut data, 90, 1800); // leveling delay
        put_short(&mut data, 94, 5); // leveling delay units = hours
        put_timestamp(&mut data, 96, 100); // earlyStart（= start）
        put_timestamp(&mut data, 104, 100); // actual start
        put_timestamp(&mut data, 108, 103); // actual finish
        put_timestamp(&mut data, 112, 99); // baseline start
        put_timestamp(&mut data, 116, 101); // baseline finish
        put_timestamp(&mut data, 120, 105); // constraint date
        put_timestamp(&mut data, 124, 106); // stop
        put_short(&mut data, 128, 4); // priority：(4 + 1) * 100 = 500
        put_short(&mut data, 130, 50); // percentage complete
        put_short(&mut data, 132, 25); // percentage work complete
        put_short(&mut data, 134, 0); // task type
        put_short(&mut data, 136, 1); // fixed cost accrual
        put_timestamp(&mut data, 138, 98); // create date
        put_short(&mut data, 142, 1); // recurring
        put_timestamp(&mut data, 144, 100); // preleveled start
        put_timestamp(&mut data, 148, 103); // preleveled finish
        put_timestamp(&mut data, 160, 107); // late finish

        put_long6(&mut data, 168, 480_000); // work -> 4800 -> 8 hours
        put_long6(&mut data, 174, 480_000); // baseline work
        put_long6(&mut data, 180, 240_000); // actual work -> 4 hours
        put_long6(&mut data, 186, 240_000); // remaining work
        put_long6(&mut data, 192, 60_000); // actual overtime work -> 1 hour
        put_long6(&mut data, 198, 60_000); // remaining overtime work
        put_long6(&mut data, 204, 1_000); // overtime cost → 10.0
        put_long6(&mut data, 210, 0); // actual overtime cost
        put_long6(&mut data, 216, 0); // remaining overtime cost
        put_long6(&mut data, 222, 5_000); // cost → 50.0
        put_long6(&mut data, 228, 2_000); // fixed cost → 20.0
        put_long6(&mut data, 234, 12_345); // actual cost → 123.45
        put_long6(&mut data, 240, 1_000); // remaining cost → 10.0
        put_long6(&mut data, 246, 4_000); // baseline cost → 40.0

        put_var_offset(&mut data, 264, name_offset);
        // flags[0] = data[268] & data[303]
        data[268] = 0x02;
        data[303] = 0x02;

        put_var_offset(&mut data, 312, ext_offset);

        data
    }

    /// Construct a 196-byte resource fixed record.
    fn resource_record(
        unique_id: i32,
        id: i32,
        name_offset: i32,
        initials_offset: i32,
        ext_offset: i32,
    ) -> Vec<u8> {
        let mut data = vec![0u8; 196];

        put_int(&mut data, 0, unique_id);
        put_int(&mut data, 4, id);
        put_short(&mut data, 20, 1); // accrue at = start
        put_int(&mut data, 24, 1); // calendar unique id
        put_timestamp(&mut data, 28, 100); // available from
        put_timestamp(&mut data, 32, 200); // available to
        put_double(&mut data, 36, 50.0); // standard rate
        put_double(&mut data, 44, 75.0); // overtime rate
        put_int(&mut data, 52, 10_000); // max units (for availability table)
        put_long6(&mut data, 56, 480_000); // work -> 8 hours
        put_long6(&mut data, 62, 240_000); // actual work -> 4 hours
        put_long6(&mut data, 68, 480_000); // baseline work
        put_long6(&mut data, 74, 60_000); // overtime work -> 1 hour
        put_long6(&mut data, 80, 1_000); // cost per use → 10.0
        put_long6(&mut data, 86, 240_000); // remaining work
        put_long6(&mut data, 92, 0); // regular work
        put_int(&mut data, 110, 5_000); // peak units → 50.0
        put_long6(&mut data, 114, 5_000); // actual cost → 50.0
        put_long6(&mut data, 120, 10_000); // cost → 100.0
        put_long6(&mut data, 126, 10_000); // baseline cost → 100.0
        put_long6(&mut data, 132, 5_000); // remaining cost → 50.0
        put_long6(&mut data, 138, 2_000); // overtime cost → 20.0
        put_long6(&mut data, 144, 0); // actual overtime cost
        put_long6(&mut data, 150, 0); // remaining overtime cost

        put_var_offset(&mut data, 156, name_offset);
        put_var_offset(&mut data, 160, initials_offset);
        put_var_offset(&mut data, 192, ext_offset);

        data
    }

    /// Construct a 204-byte assignment fixed record.
    fn assignment_record(unique_id: i32, task_unique_id: i32, resource_unique_id: i32) -> Vec<u8> {
        let mut data = vec![0u8; 204];

        put_int(&mut data, 0, unique_id);
        put_int(&mut data, 16, task_unique_id);
        put_int(&mut data, 20, resource_unique_id);
        put_timestamp(&mut data, 24, 100); // start
        put_timestamp(&mut data, 28, 103); // finish
        put_short(&mut data, 80, 100); // units = 1.0
        put_long6(&mut data, 84, 480_000); // work -> 8 hours
        put_long6(&mut data, 90, 0); // overtime work
        put_long6(&mut data, 96, 240_000); // actual work -> 4 hours
        put_long6(&mut data, 114, 240_000); // remaining work
        put_long6(&mut data, 132, 5_000); // cost → 50.0
        put_long6(&mut data, 138, 1_000); // actual cost → 10.0
        // Offset 168 stays 0: not deleted

        data
    }

    /// Construct the "working hours per day" payload in the calendar data block (7 days × 40 bytes + 1 exception × 44 bytes).
    fn calendar_hours_payload() -> Vec<u8> {
        let mut data = vec![0u8; 4 + (40 * 7) + 44];

        // Java: exceptionCount = ByteArrayHelper.getShort(extData, 0);
        put_short(&mut data, 0, 1);

        // Monday to Friday: defaultFlag == 1 (reuse the default working hours 8:00-12:00 / 13:00-17:00)
        for index in 1..=5usize {
            put_short(&mut data, 4 + (40 * index), 1);
        }

        // Exception: 1984-01-05 (5 days since 1983-12-31), working hours 01:00 - 02:00
        let offset = 4 + (40 * 7);
        put_short(&mut data, offset, 5);
        put_short(&mut data, offset + 2, 5);
        put_short(&mut data, offset + 6, 1);
        put_short(&mut data, offset + 12, 600); // (600 / 10) * 60 = 3600 seconds = 01:00
        put_short(&mut data, offset + 20, 600); // 600 * 6000 ms = 1 hour

        data
    }

    /// Construct a calendar fixed record plus its var data.
    fn calendar_data() -> (Vec<u8>, Vec<u8>) {
        build_var_data(|allocator| {
            let name_offset = allocator.push_string("Standard");
            let hours_offset = allocator.push(&calendar_hours_payload());
            let ext_block = extended_block(&[(8, (-1 - hours_offset).to_le_bytes().to_vec())]);
            let ext_offset = allocator.push(&ext_block);

            let mut record = vec![0u8; 36];
            put_int(&mut record, 0, 1); // calendar id
            put_int(&mut record, 4, 0); // base calendar id
            put_var_offset(&mut record, 20, name_offset);
            put_var_offset(&mut record, 32, ext_offset);
            record
        })
    }

    /// Construct task fixed data and var data.
    fn task_data() -> (Vec<u8>, Vec<u8>) {
        build_var_data(|allocator| {
            let name1 = allocator.push_string("Task A");
            let name2 = allocator.push_string("Task B");
            let wbs = allocator.push_string("1.1");
            let contact = allocator.push_string("Jon");
            let text1 = allocator.push_string("Text one");
            let notes = allocator.push_string("{\\rtf1 Hello}");
            let hyperlink = {
                let mut payload = vec![0u8; 24];
                payload.extend_from_slice(&utf16("https://example.com"));
                payload.extend_from_slice(&[0xBB; 12]);
                payload.extend_from_slice(&utf16("address"));
                payload.extend_from_slice(&[0xCC; 12]);
                payload.extend_from_slice(&utf16("subaddress"));
                allocator.push(&payload)
            };

            let mut recurring = vec![0u8; 42];
            put_short(&mut recurring, 6, 100); // start date
            put_short(&mut recurring, 10, 110); // end date
            put_int(&mut recurring, 12, 4800); // duration
            put_short(&mut recurring, 16, 7); // duration units = days
            put_short(&mut recurring, 18, 5); // occurrences
            put_short(&mut recurring, 20, 1); // recurrence type
            put_short(&mut recurring, 24, 1); // use end date
            put_short(&mut recurring, 26, 1); // working days only
            put_short(&mut recurring, 28, 0); // Sunday
            put_short(&mut recurring, 30, 1); // Monday

            let items = vec![
                (TASK_WBS, (-1 - wbs).to_le_bytes().to_vec()),
                (TASK_CONTACT, (-1 - contact).to_le_bytes().to_vec()),
                (TASK_TEXT[0], (-1 - text1).to_le_bytes().to_vec()),
                (TASK_NOTES, (-1 - notes).to_le_bytes().to_vec()),
                (TASK_DURATION[0], 4800i32.to_le_bytes().to_vec()),
                (TASK_DURATION_UNITS[0], 7i16.to_le_bytes().to_vec()),
                (TASK_NUMBER[0], 1.5f64.to_le_bytes().to_vec()),
                (TASK_COST[0], {
                    let mut value = vec![0u8; 6];
                    put_long6(&mut value, 0, 12_345);
                    value
                }),
                (TASK_DATE[0], {
                    let mut value = vec![0u8; 4];
                    put_timestamp(&mut value, 0, 110);
                    value
                }),
                (TASK_START[0], {
                    let mut value = vec![0u8; 4];
                    put_timestamp(&mut value, 0, 110);
                    value
                }),
                (TASK_FINISH[0], {
                    let mut value = vec![0u8; 4];
                    put_timestamp(&mut value, 0, 112);
                    value
                }),
                (TASK_HYPERLINK, (-1 - hyperlink).to_le_bytes().to_vec()),
                (TASK_RECURRING_DATA, recurring),
            ];

            let ext_block = extended_block(&items);
            let ext_offset = allocator.push(&ext_block);

            let task1 = task_record(1, 1, name1, ext_offset);
            let task2 = task_record(2, 2, name2, ext_offset);

            [task1, task2].concat()
        })
    }

    /// Construct resource fixed data and var data.
    fn resource_data() -> (Vec<u8>, Vec<u8>) {
        build_var_data(|allocator| {
            let name = allocator.push_string("Resource 1");
            let initials = allocator.push_string("R1");
            let code = allocator.push_string("CODE");
            let email = allocator.push_string("r1@example.com");
            let group = allocator.push_string("Group");
            let notes = allocator.push_string("{\\rtf1 Resource notes}");

            let ext_block = extended_block(&[
                (RESOURCE_GROUP, (-1 - group).to_le_bytes().to_vec()),
                (RESOURCE_CODE, (-1 - code).to_le_bytes().to_vec()),
                (RESOURCE_EMAIL, (-1 - email).to_le_bytes().to_vec()),
                (RESOURCE_NOTES, (-1 - notes).to_le_bytes().to_vec()),
                (RESOURCE_COST[0], {
                    let mut value = vec![0u8; 6];
                    put_long6(&mut value, 0, 12_345);
                    value
                }),
                (RESOURCE_NUMBER[0], 2.5f64.to_le_bytes().to_vec()),
                (RESOURCE_DATE[0], {
                    let mut value = vec![0u8; 4];
                    put_timestamp(&mut value, 0, 110);
                    value
                }),
                (RESOURCE_DURATION[0], 4800i32.to_le_bytes().to_vec()),
                (RESOURCE_DURATION_UNITS[0], 7i16.to_le_bytes().to_vec()),
                (RESOURCE_START[0], {
                    let mut value = vec![0u8; 4];
                    put_timestamp(&mut value, 0, 100);
                    value
                }),
                (RESOURCE_FINISH[0], {
                    let mut value = vec![0u8; 4];
                    put_timestamp(&mut value, 0, 110);
                    value
                }),
            ]);
            let ext_offset = allocator.push(&ext_block);

            resource_record(1, 1, name, initials, ext_offset)
        })
    }

    /// Construct table fixed data and var data.
    fn table_data() -> (Vec<u8>, Vec<u8>) {
        build_var_data(|allocator| {
            let mut columns = vec![0u8; 8 + 2 * COLUMN_BLOCK_SIZE];
            put_short(&mut columns, 4, 1); // columnCount - 1 -> 2 columns

            let mut index = 8usize;
            put_int(
                &mut columns,
                index,
                FieldTypeHelper::get_field_id(Some(&MppFieldType::Builtin(FieldKey::Task(
                    TaskField::Name,
                )))),
            );
            columns[index + 4] = 20; // width
            put_short(&mut columns, index + 6, 40); // title offset
            columns[index + 8] = 32; // ALIGN_LEFT
            columns[index + 10] = 33; // ALIGN_CENTER

            index += COLUMN_BLOCK_SIZE;
            put_int(
                &mut columns,
                index,
                FieldTypeHelper::get_field_id(Some(&MppFieldType::Builtin(FieldKey::Task(
                    TaskField::Cost,
                )))),
            );
            columns[index + 4] = 10;
            columns[index + 8] = 34; // other values -> ALIGN_RIGHT
            columns[index + 10] = 32;

            // The column title is placed at offset 40 (UTF-16LE: "Task")
            let title = utf16("Task");
            columns.resize(40, 0);
            columns.extend_from_slice(&title);

            let column_offset = allocator.push(&columns);

            let mut table_ext = vec![0u8; 12];
            put_int(&mut table_ext, 8, -1 - column_offset);
            let ext_offset = allocator.push(&table_ext);

            let mut record = vec![0u8; 126];
            put_int(&mut record, 0, 1);
            let name = utf16("Entry");
            record[4..4 + name.len()].copy_from_slice(&name);
            put_var_offset(&mut record, 122, ext_offset);
            record
        })
    }

    /// Write a stream to the test compound document, filling in the parent storage hierarchy first if necessary.
    ///
    /// `cfb`'s `create_stream` does not create parent storages automatically, so `create_storage_all` is used first
    /// to build the entire parent path (it skips storages that already exist, so it can be called repeatedly).
    /// Since only directories that have actually had a stream written are created, the "intentionally missing" directories
    /// (such as `TBkndCons`, which [`missing_constraints_directory_is_an_ignored_error`] relies on)
    /// remain non-existent.
    fn add_stream(compound: &mut CompoundFile<Cursor<Vec<u8>>>, path: &str, data: &[u8]) {
        let parent = path.rsplit_once('/').expect("path contains parent directory").0;
        compound.create_storage_all(parent).expect("create parent storage");
        compound
            .create_stream(path)
            .expect("create stream")
            .write_all(data)
            .expect("write stream");
    }

    /// Construct a minimal but complete MPP8 compound document.
    fn build_compound_file() -> Vec<u8> {
        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).expect("create compound document");

        // ---- Project storage: Props ----
        let props = props8_stream(&[
            (PropsKey::DefaultCalendarName.get_value(), utf16("Standard")),
            (
                PropsKey::MinutesPerDay.get_value(),
                480i32.to_le_bytes().to_vec(),
            ),
        ]);
        add_stream(&mut compound, "/   1/Props", &props);

        // ---- Calendars ----
        let (cal_var, cal_fixed) = calendar_data();
        add_stream(&mut compound, "/   1/TBkndCal/FixFix   0", &cal_fixed);
        add_stream(&mut compound, "/   1/TBkndCal/FixDeferFix   0", &cal_var);

        // ---- Tasks ----
        let (task_var, task_fixed) = task_data();
        add_stream(&mut compound, "/   1/TBkndTask/FixFix   0", &task_fixed);
        add_stream(&mut compound, "/   1/TBkndTask/FixDeferFix   0", &task_var);

        // ---- Resources ----
        let (rsc_var, rsc_fixed) = resource_data();
        add_stream(&mut compound, "/   1/TBkndRsc/FixFix   0", &rsc_fixed);
        add_stream(&mut compound, "/   1/TBkndRsc/FixDeferFix   0", &rsc_var);

        // ---- Assignments ----
        let assignment = assignment_record(1, 1, 1);
        add_stream(&mut compound, "/   1/TBkndAssn/FixFix   0", &assignment);
        add_stream(&mut compound, "/   1/TBkndAssn/FixDeferFix   0", &[0u8; 8]);

        // ---- Constraints ----
        let mut cons = vec![0u8; 36];
        put_int(&mut cons, 0, 7); // relation unique id
        put_int(&mut cons, 12, 1); // predecessor unique id
        put_int(&mut cons, 16, 2); // successor unique id
        put_short(&mut cons, 20, 0); // type = finish-finish（Java: FINISH_FINISH = 0）
        put_short(&mut cons, 22, 5); // lag units = hours
        put_int(&mut cons, 24, 600); // lag -> 1 hour
        add_stream(&mut compound, "/   1/TBkndCons/FixFix   0", &cons);

        // ---- View storage: Props / CV_iew / CTable ----
        let view_props = props8_stream(&[(
            PropsKey::ShowProjectSummaryTask.get_value(),
            1i16.to_le_bytes().to_vec(),
        )]);
        add_stream(&mut compound, "/   2/Props", &view_props);

        let mut view_record = vec![0u8; 138];
        put_int(&mut view_record, 0, 1);
        let name = utf16("Gantt Chart");
        view_record[4..4 + name.len()].copy_from_slice(&name);
        add_stream(&mut compound, "/   2/CV_iew/FixFix   0", &view_record);

        let (table_var, table_fixed) = table_data();
        add_stream(&mut compound, "/   2/CTable/FixFix   0", &table_fixed);
        add_stream(&mut compound, "/   2/CTable/FixDeferFix   0", &table_var);

        compound.into_inner().into_inner()
    }

    /// Open the test compound document.
    fn open(bytes: Vec<u8>) -> (PoifsFileSystem, DirectoryEntry) {
        let fs = PoifsFileSystem::new(Cursor::new(bytes)).expect("open compound document");
        let root = fs.get_root();
        (fs, root)
    }

    /// Drive the read flow (equivalent to `process`, but without depending on [`MPPReader`]).
    fn read_file(file: &Rc<RefCell<ProjectFile>>, root: &DirectoryEntry) -> MpxjResult<()> {
        let mut state = MPP8ReaderState::new();
        let result = match state.populate_member_data(file, root) {
            Ok(()) => state.run(false, true),
            Err(error) => Err(error),
        };
        state.clear_member_data();
        result
    }

    /// The `days`-th day since 1983-12-31 (at midnight).
    fn epoch_plus_days(days: i64) -> NaiveDateTime {
        let epoch = NaiveDate::from_ymd_opt(1983, 12, 31)
            .expect("valid date")
            .and_hms_opt(0, 0, 0)
            .expect("valid time");
        epoch + chrono::Duration::days(days)
    }

    /// Get the parsed calendars.
    fn calendar(file: &Rc<RefCell<ProjectFile>>, unique_id: i32) -> Rc<RefCell<ProjectCalendar>> {
        file.borrow()
            .get_calendars()
            .get_by_unique_id(Some(unique_id))
            .expect("calendar exists")
    }

    /// Construct a compound document with "project properties only" (reused by test cases with missing data blocks).
    fn props_only_compound_file() -> Vec<u8> {
        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).expect("create compound document");
        let props = props8_stream(&[(
            PropsKey::DefaultCalendarName.get_value(),
            utf16("Standard"),
        )]);
        add_stream(&mut compound, "/   1/Props", &props);
        compound.into_inner().into_inner()
    }

    #[test]
    fn reads_project_properties_and_calendars() {
        let bytes = build_compound_file();
        let (_fs, root) = open(bytes);
        let file = ProjectFile::new();

        read_file(&file, &root).expect("read succeeded");

        let file_ref = file.borrow();

        // Java: setMppFileType(Integer.valueOf(8))
        assert_eq!(
            file_ref.get_project_properties().get_mpp_file_type(),
            Some(8)
        );
        // Java: properties.setShowProjectSummaryTask(props.getBoolean(PropsKey.SHOW_PROJECT_SUMMARY_TASK))
        assert!(file_ref
            .get_project_properties()
            .get_show_project_summary_task());

        // Calendar: name / unique ID
        let cal = calendar(&file, 1);
        assert_eq!(cal.borrow().get_name(), Some("Standard"));
        assert_eq!(cal.borrow().get_unique_id(), Some(1));

        // Working day flag
        assert!(cal.borrow().is_working_day(Weekday::Mon));
        assert!(cal.borrow().is_working_day(Weekday::Fri));
        assert!(!cal.borrow().is_working_day(Weekday::Sat));
        assert!(!cal.borrow().is_working_day(Weekday::Sun));

        // Default working hours 8:00-12:00 / 13:00-17:00
        let hours = cal.borrow().get_hours(Weekday::Mon).expect("Monday has work hours");
        assert_eq!(hours.size(), 2);
        assert_eq!(hours.get(0).get_start(), time(8, 0));
        assert_eq!(hours.get(0).get_end(), time(12, 0));
        assert_eq!(hours.get(1).get_start(), time(13, 0));
        assert_eq!(hours.get(1).get_end(), time(17, 0));

        // Non-working days also get (empty) working hours registered
        let sunday_hours = cal.borrow().get_hours(Weekday::Sun).expect("Sunday has hours records");
        assert_eq!(sunday_hours.size(), 0);

        // Exception dates and their working hours
        {
            let mut cal_ref = cal.borrow_mut();
            let exceptions = cal_ref.get_calendar_exceptions();
            assert_eq!(exceptions.len(), 1);
            assert_eq!(
                exceptions[0].get_from_date(),
                NaiveDate::from_ymd_opt(1984, 1, 5)
            );
            assert_eq!(exceptions[0].size(), 1);
            assert_eq!(exceptions[0].get(0).get_start(), time(1, 0));
            assert_eq!(exceptions[0].get(0).get_end(), time(2, 0));
        }

        // Java: setDefaultCalendar(getCalendars().getByName(DEFAULT_CALENDAR_NAME in Props))
        let default_calendar = file_ref.get_default_calendar().expect("default calendar exists");
        assert_eq!(default_calendar.borrow().get_name(), Some("Standard"));
    }

    #[test]
    fn reads_tasks_resources_assignments_and_relations() {
        let bytes = build_compound_file();
        let (_fs, root) = open(bytes);
        let file = ProjectFile::new();

        read_file(&file, &root).expect("read succeeded");

        let file_ref = file.borrow();
        assert_eq!(file_ref.get_tasks().len(), 2);
        assert_eq!(file_ref.get_resources().len(), 1);
        assert_eq!(file_ref.get_resource_assignments().len(), 1);

        // ---- Tasks ----
        let task = file_ref
            .get_task_by_unique_id(Some(1))
            .expect("task 1 exists");
        let task2 = file_ref
            .get_task_by_unique_id(Some(2))
            .expect("task 2 exists");

        {
            let task_ref = task.borrow();
            assert_eq!(task_ref.get_name(), Some("Task A".to_owned()));
            assert_eq!(task_ref.get_id(), Some(1));
            assert_eq!(task_ref.get_wbs(), Some("1.1".to_owned()));
            assert_eq!(task_ref.get_contact(), Some("Jon".to_owned()));
            assert_eq!(task_ref.get_text(1), Some("Text one".to_owned()));
            assert!(task_ref.get_milestone());
            assert!(task_ref.get_marked());
            assert!(task_ref.get_rollup());
            assert!(task_ref.get_hide_bar());
            assert!(task_ref.get_effort_driven());
            assert!(task_ref.get_level_assignments());
            assert!(task_ref.get_leveling_can_split());
            assert!(task_ref.get_recurring());
            assert!(task_ref.get_flag(1));
            assert!(!task_ref.get_flag(2));
            assert_eq!(task_ref.get_outline_level(), Some(1));
            assert_eq!(task_ref.get_percentage_complete(), Some(50.0));
            assert_eq!(task_ref.get_percentage_work_complete(), Some(25.0));
            assert_eq!(
                task_ref.get_constraint_type(),
                Some(ConstraintType::MustStartOn)
            );
            assert_eq!(task_ref.get_constraint_date(), Some(epoch_plus_days(105)));
            assert_eq!(task_ref.get_start(), Some(epoch_plus_days(100)));
            assert_eq!(task_ref.get_finish(), Some(epoch_plus_days(100)));
            assert_eq!(task_ref.get_early_start(), Some(epoch_plus_days(100)));
            assert_eq!(task_ref.get_late_start(), Some(epoch_plus_days(101)));
            assert_eq!(task_ref.get_late_finish(), Some(epoch_plus_days(107)));
            assert_eq!(task_ref.get_stop(), Some(epoch_plus_days(106)));
            assert_eq!(task_ref.get_resume(), Some(epoch_plus_days(102)));
            assert_eq!(task_ref.get_preleveled_start(), Some(epoch_plus_days(100)));
            assert_eq!(task_ref.get_preleveled_finish(), Some(epoch_plus_days(103)));
            assert_eq!(task_ref.get_baseline_start(), Some(epoch_plus_days(99)));
            assert_eq!(task_ref.get_baseline_finish(), Some(epoch_plus_days(101)));
            assert_eq!(task_ref.get_actual_start(), Some(epoch_plus_days(100)));
            assert_eq!(task_ref.get_actual_finish(), Some(epoch_plus_days(103)));
            assert_eq!(
                task_ref.get_priority().map(|value| value.get_value()),
                Some(500)
            );
            assert_eq!(
                task_ref.get_type(),
                Some(crate::task_type::TaskType::FixedUnits)
            );
            assert_eq!(task_ref.get_fixed_cost_accrual(), Some(AccrueType::Start));

            // Duration: 4800 / (480 * 10) = 1 day
            let duration = task_ref.get_duration().expect("duration exists");
            assert_eq!(duration.get_units(), TimeUnit::Days);
            assert_eq!(duration.get_duration(), 1.0);
            // Work: long integer 480000 / 100 / 600 = 8 hours
            let work = task_ref.get_work().expect("work exists");
            assert_eq!(work.get_units(), TimeUnit::Hours);
            assert_eq!(work.get_duration(), 8.0);
            assert_eq!(
                task_ref
                    .get_actual_work()
                    .expect("actual work exists")
                    .get_duration(),
                4.0
            );
            assert_eq!(
                task_ref
                    .get_actual_overtime_work()
                    .expect("actual overtime work exists")
                    .get_duration(),
                1.0
            );
            // Currency value: long integer / 100
            assert_eq!(task_ref.get_cost(), Some(50.0));
            assert_eq!(task_ref.get_cost_at(1), Some(123.45));
            assert_eq!(task_ref.get_fixed_cost(), Some(20.0));
            assert_eq!(task_ref.get_actual_cost(), Some(123.45));
            assert_eq!(task_ref.get_remaining_cost(), Some(10.0));
            assert_eq!(task_ref.get_baseline_cost(), Some(40.0));
            assert_eq!(task_ref.get_overtime_cost(), Some(10.0));
            // leveling delay: (1800 / 3) / 600 = 1 hour
            let leveling_delay = task_ref.get_leveling_delay().expect("leveling delay exists");
            assert_eq!(leveling_delay.get_units(), TimeUnit::Hours);
            assert_eq!(leveling_delay.get_duration(), 1.0);
            // Custom fields in the extended data
            let duration1 = task_ref.get_duration_at(1).expect("duration 1 exists");
            assert_eq!(duration1.get_units(), TimeUnit::Days);
            assert_eq!(duration1.get_duration(), 1.0);
            assert_eq!(task_ref.get_number(1), Some(1.5));
            assert_eq!(task_ref.get_date(1), Some(epoch_plus_days(110)));
            assert_eq!(task_ref.get_start_at(1), Some(epoch_plus_days(110)));
            assert_eq!(task_ref.get_finish_at(1), Some(epoch_plus_days(112)));
            // Hyperlink (Project 98 has no screen tip)
            assert_eq!(
                task_ref.get_hyperlink(),
                Some("https://example.com".to_owned())
            );
            assert_eq!(task_ref.get_hyperlink_address(), Some("address".to_owned()));
            assert_eq!(
                task_ref.get_hyperlink_sub_address(),
                Some("subaddress".to_owned())
            );
            assert_eq!(task_ref.get_hyperlink_screen_tip(), None);
            // Notes (RTF text)
            assert!(!task_ref.get_notes().is_empty());
            // Recurring task
            let recurring = task_ref.get_recurring_task().expect("recurring task data exists");
            assert_eq!(recurring.get_recurring_data().get_occurrences(), Some(5));
            assert!(recurring.get_recurring_data().get_use_end_date());
            assert!(recurring.get_recurring_data().get_working_days_only());
        }

        // Task 1 has a WBS → automatic WBS generation is disabled
        assert!(!file_ref.get_project_config().get_auto_wbs());
        assert_eq!(task2.borrow().get_name(), Some("Task B".to_owned()));

        // ---- Resources ----
        let resource = file_ref
            .get_resource_by_unique_id(Some(1))
            .expect("resource exists");
        {
            let resource_ref = resource.borrow();
            assert_eq!(resource_ref.get_name(), Some("Resource 1".to_owned()));
            assert_eq!(resource_ref.get_initials(), Some("R1".to_owned()));
            assert_eq!(resource_ref.get_code(), Some("CODE".to_owned()));
            assert_eq!(
                resource_ref.get_email_address(),
                Some("r1@example.com".to_owned())
            );
            assert_eq!(resource_ref.get_group(), Some("Group".to_owned()));
            assert_eq!(resource_ref.get_id(), Some(1));
            assert_eq!(resource_ref.get_unique_id(), Some(1));
            assert_eq!(resource_ref.get_actual_cost(), Some(50.0));
            assert_eq!(resource_ref.get_cost(), Some(100.0));
            assert_eq!(resource_ref.get_cost_at(1), Some(123.45));
            assert_eq!(resource_ref.get_peak_units(), Some(50.0));
            assert_eq!(resource_ref.get_overtime_cost(), Some(20.0));
            assert_eq!(resource_ref.get_baseline_cost(), Some(100.0));
            assert_eq!(resource_ref.get_remaining_cost(), Some(50.0));
            assert_eq!(resource_ref.get_number_at(1), Some(2.5));
            assert_eq!(resource_ref.get_date_at(1), Some(epoch_plus_days(110)));
            let resource_duration = resource_ref.get_duration_at(1).expect("resource duration 1 exists");
            assert_eq!(resource_duration.get_units(), TimeUnit::Days);
            assert_eq!(resource_duration.get_duration(), 1.0);
            assert_eq!(resource_ref.get_start_at(1), Some(epoch_plus_days(100)));
            assert_eq!(resource_ref.get_finish_at(1), Some(epoch_plus_days(110)));
            assert!(!resource_ref.get_notes().is_empty());
            // Associated calendar
            assert_eq!(
                resource_ref
                    .get_calendar()
                    .map(|calendar| calendar.borrow().get_name().map(str::to_owned)),
                Some(Some("Standard".to_owned()))
            );
            // Rate table: the first row of table 0
            let rate_table = resource_ref.get_cost_rate_table(0).expect("rate table exists");
            let rate_entry = rate_table.get(0).expect("rate entry exists");
            assert_eq!(rate_entry.get_cost_per_use(), Some(10.0));
            assert_eq!(
                rate_entry.get_standard_rate(),
                Rate::new(50.0, TimeUnit::Hours)
            );
            assert_eq!(
                rate_entry.get_overtime_rate(),
                Rate::new(75.0, TimeUnit::Hours)
            );
        }

        // ---- Assignments ----
        let assignment = file_ref
            .get_resource_assignments()
            .get_by_task_unique_id(Some(1))
            .into_iter()
            .next()
            .expect("assignment exists");
        {
            let assignment_ref = assignment.borrow();
            assert_eq!(assignment_ref.get_unique_id(), Some(1));
            assert_eq!(assignment_ref.get_units(), Some(1.0));
            assert_eq!(assignment_ref.get_cost(), Some(50.0));
            assert_eq!(assignment_ref.get_actual_cost(), Some(10.0));
            assert_eq!(
                assignment_ref.get_work().expect("assignment work exists").get_duration(),
                8.0
            );
            assert_eq!(assignment_ref.get_start(), Some(epoch_plus_days(100)));
            assert_eq!(assignment_ref.get_finish(), Some(epoch_plus_days(103)));
        }

        // ---- Relations ----
        let relations = task2.borrow().get_predecessors();
        assert_eq!(relations.len(), 1);
        {
            let relation = relations[0].borrow();
            assert_eq!(relation.get_type(), RelationType::FinishFinish);
            assert_eq!(relation.get_unique_id(), Some(7));
            let lag = relation.get_lag().expect("lag exists");
            assert_eq!(lag.get_units(), TimeUnit::Hours);
            assert_eq!(lag.get_duration(), 1.0);
        }
    }

    #[test]
    fn reads_view_and_table_data() {
        let bytes = build_compound_file();
        let (_fs, root) = open(bytes);
        let file = ProjectFile::new();

        read_file(&file, &root).expect("read succeeded");

        let file_ref = file.borrow();

        // Views
        assert_eq!(file_ref.get_views().len(), 1);
        let view = file_ref.get_views().get(0).expect("view exists");
        assert_eq!(view.borrow().get_id(), Some(1));
        assert_eq!(view.borrow().get_name(), Some("Gantt Chart"));

        // Tables
        assert_eq!(file_ref.get_tables().len(), 1);
        let table = file_ref.get_tables().get(0).expect("table exists");
        let table_ref = table.borrow();
        assert_eq!(table_ref.get_id(), 1);
        assert_eq!(table_ref.get_name(), Some("Entry"));
        assert!(!table_ref.get_resource_flag());

        let columns = table_ref.get_columns();
        assert_eq!(columns.len(), 2);
        assert_eq!(
            columns[0].get_field_type(),
            Some(FieldKey::Task(TaskField::Name))
        );
        assert_eq!(columns[0].get_width(), 20);
        assert_eq!(columns[0].get_title(), Some("Task".to_owned()));
        assert_eq!(columns[0].get_align_title(), Column::ALIGN_LEFT);
        assert_eq!(columns[0].get_align_data(), Column::ALIGN_CENTER);
        assert_eq!(
            columns[1].get_field_type(),
            Some(FieldKey::Task(TaskField::Cost))
        );
        assert_eq!(columns[1].get_width(), 10);
        // Title offset is 0 → no explicit title; `Column.getTitle()` falls back to the field name
        // (Java: m_fieldType.getName(locale))
        assert_eq!(columns[1].get_title(), Some("Cost".to_owned()));
        assert_eq!(columns[1].get_align_title(), Column::ALIGN_RIGHT);
        assert_eq!(columns[1].get_align_data(), Column::ALIGN_LEFT);
    }

    #[test]
    fn missing_props_stream_reports_an_error() {
        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).expect("create compound document");
        compound.create_storage("/   1").expect("create project storage");
        let bytes = compound.into_inner().into_inner();

        let (_fs, root) = open(bytes);
        let file = ProjectFile::new();

        // Java: FileNotFoundException → MPXJException
        let error = read_file(&file, &root).err().expect("missing Props should error");
        assert!(error.message().starts_with(MpxjError::READ_ERROR));
    }

    #[test]
    fn missing_calendar_block_reports_an_error() {
        let (_fs, root) = open(props_only_compound_file());
        let file = ProjectFile::new();

        // Java: missing TBkndCal → FileNotFoundException → MPXJException; the Rust side returns Err instead of panicking
        let error = read_file(&file, &root).err().expect("missing calendar data should error");
        assert!(error.message().contains("TBkndCal"));
    }

    #[test]
    fn missing_constraints_directory_is_an_ignored_error() {
        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).expect("create compound document");
        let props = props8_stream(&[(
            PropsKey::DefaultCalendarName.get_value(),
            utf16("Standard"),
        )]);
        add_stream(&mut compound, "/   1/Props", &props);

        let (cal_var, cal_fixed) = calendar_data();
        add_stream(&mut compound, "/   1/TBkndCal/FixFix   0", &cal_fixed);
        add_stream(&mut compound, "/   1/TBkndCal/FixDeferFix   0", &cal_var);

        // Tasks / resources / assignments: all empty.
        // This test case relies on the `TBkndCons` directory being missing, so no stream is written to it here,
        // and the directory will not be created either (`add_stream` only fills in parent storages for streams that are written).
        for directory in ["TBkndTask", "TBkndRsc", "TBkndAssn"] {
            add_stream(&mut compound, &format!("/   1/{directory}/FixFix   0"), &[]);
            add_stream(
                &mut compound,
                &format!("/   1/{directory}/FixDeferFix   0"),
                &[0u8; 8],
            );
        }

        let bytes = compound.into_inner().into_inner();
        let (_fs, root) = open(bytes);
        let file = ProjectFile::new();

        // Precondition check: the `TBkndCons` directory is indeed missing, otherwise this test case is meaningless
        assert!(root.get_directory("TBkndCons").is_none());

        // A missing TBkndCons is allowed: Java records addIgnoredError and continues
        read_file(&file, &root).expect("missing constraints directory is not an error");

        let file_ref = file.borrow();
        assert_eq!(file_ref.get_relations().len(), 0);
        assert!(file_ref
            .get_ignored_errors()
            .iter()
            .any(|error| error.message().contains("TBkndCons")));
    }

    #[test]
    fn truncated_records_do_not_panic() {
        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).expect("create compound document");
        let props = props8_stream(&[(
            PropsKey::DefaultCalendarName.get_value(),
            utf16("Standard"),
        )]);
        add_stream(&mut compound, "/   1/Props", &props);

        // Calendar fixed data: 40 bytes = 1 complete record + 4 residual bytes (getDiff() != 0)
        let mut cal_fixed = vec![0u8; 40];
        put_int(&mut cal_fixed, 0, 1);
        cal_fixed[36] = 0xAA;
        add_stream(&mut compound, "/   1/TBkndCal/FixFix   0", &cal_fixed);
        // Var data is empty → getByteArray always returns None
        add_stream(&mut compound, "/   1/TBkndCal/FixDeferFix   0", &[]);

        // Task fixed data: one complete record, but var data is insufficient to parse the extended data
        let task_fixed = task_record(1, 1, 8, 8);
        add_stream(&mut compound, "/   1/TBkndTask/FixFix   0", &task_fixed);
        add_stream(&mut compound, "/   1/TBkndTask/FixDeferFix   0", &[0u8; 8]);

        // Resource fixed data: truncated to less than one record (196 bytes)
        add_stream(&mut compound, "/   1/TBkndRsc/FixFix   0", &[0u8; 100]);
        add_stream(&mut compound, "/   1/TBkndRsc/FixDeferFix   0", &[0u8; 8]);

        // Assignment fixed data: length insufficient to hold any record
        add_stream(&mut compound, "/   1/TBkndAssn/FixFix   0", &[0u8; 10]);
        add_stream(&mut compound, "/   1/TBkndAssn/FixDeferFix   0", &[0u8; 8]);

        let bytes = compound.into_inner().into_inner();
        let (_fs, root) = open(bytes);
        let file = ProjectFile::new();

        // No panic; with incomplete data it may return Ok (skipping the missing parts) or Err
        let _ = read_file(&file, &root);

        let file_ref = file.borrow();
        // The calendar is still registered: var data is empty → the extended data offset is -1 → it is treated as a "working calendar",
        // i.e. it degrades to the default base calendar (Java: addDefaultBaseCalendar comes with default working hours).
        let cal = calendar(&file, 1);
        let monday = cal
            .borrow()
            .get_hours(Weekday::Mon)
            .expect("default base calendar has default work hours");
        assert_eq!(monday.size(), 2);
        assert_eq!(monday.get(0).get_start(), time(8, 0));
        assert_eq!(monday.get(0).get_end(), time(12, 0));
        assert_eq!(monday.get(1).get_start(), time(13, 0));
        assert_eq!(monday.get(1).get_end(), time(17, 0));
        // The task exists, but the extended data is empty → name / WBS are both None
        if let Some(task) = file_ref.get_task_by_unique_id(Some(1)) {
            let task_ref = task.borrow();
            assert_eq!(task_ref.get_name(), None);
            assert_eq!(task_ref.get_wbs(), None);
        }
        // Neither the resource nor the assignment can be read
        assert_eq!(file_ref.get_resources().len(), 0);
        assert_eq!(file_ref.get_resource_assignments().len(), 0);
    }

    #[test]
    fn assignment_item_size_falls_back_to_238() {
        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).expect("create compound document");
        let props = props8_stream(&[(
            PropsKey::DefaultCalendarName.get_value(),
            utf16("Standard"),
        )]);
        add_stream(&mut compound, "/   1/Props", &props);

        let (cal_var, cal_fixed) = calendar_data();
        add_stream(&mut compound, "/   1/TBkndCal/FixFix   0", &cal_fixed);
        add_stream(&mut compound, "/   1/TBkndCal/FixDeferFix   0", &cal_var);

        for directory in ["TBkndTask", "TBkndRsc"] {
            add_stream(&mut compound, &format!("/   1/{directory}/FixFix   0"), &[]);
            add_stream(
                &mut compound,
                &format!("/   1/{directory}/FixDeferFix   0"),
                &[0u8; 8],
            );
        }

        // 238 bytes is not divisible by 204 → getDiff() != 0 → switch to the 238 record length
        let mut assignment = vec![0u8; 238];
        put_int(&mut assignment, 0, 1);
        put_int(&mut assignment, 16, 99); // non-existent task
        put_int(&mut assignment, 20, 99); // non-existent resource
        add_stream(&mut compound, "/   1/TBkndAssn/FixFix   0", &assignment);
        add_stream(&mut compound, "/   1/TBkndAssn/FixDeferFix   0", &[0u8; 8]);

        let bytes = compound.into_inner().into_inner();
        let (_fs, root) = open(bytes);
        let file = ProjectFile::new();

        read_file(&file, &root).expect("read succeeded");
        // Neither the task nor the resource exists → no assignment is created
        assert_eq!(file.borrow().get_resource_assignments().len(), 0);
    }

    #[test]
    fn mpp8_column_alignment_matches_java() {
        assert_eq!(mpp8_column_alignment(32), Column::ALIGN_LEFT);
        assert_eq!(mpp8_column_alignment(33), Column::ALIGN_CENTER);
        assert_eq!(mpp8_column_alignment(0), Column::ALIGN_RIGHT);
        assert_eq!(mpp8_column_alignment(255), Column::ALIGN_RIGHT);
    }

    #[test]
    fn get_offset_uses_the_minus_one_convention() {
        let mut data = vec![0u8; 4];
        put_int(&mut data, 0, 7);
        assert_eq!(get_offset(&data, 0), -8);
        // Out of bounds (Java throws AIOOBE) → reads 0 → -1
        assert_eq!(get_offset(&data, 100), -1);
    }

    #[test]
    fn var_data_block_chain_is_understood_by_fix_defer_fix() {
        // Verify that the test constructor and the production code agree on the block-chain layout (across blocks + single block)
        let mut allocator = VarAllocator::new();
        let short = allocator.push(b"short");
        let long = allocator.push(&[0x5Au8; 100]);
        assert_eq!(short, 8);
        assert_eq!(long, 8 + 36);

        let fix = FixDeferFix::new(&allocator.buffer);
        assert_eq!(fix.get_byte_array(short).as_deref(), Some(&b"short"[..]));
        assert_eq!(fix.get_byte_array(long), Some(vec![0x5A; 100]));
    }

    #[test]
    fn reader_implements_the_variant_reader_trait() {
        // Compile-time check: `MPP8Reader` can be used as `dyn MPPVariantReader`
        let reader: Box<dyn MPPVariantReader> = Box::new(MPP8Reader::new());
        assert_eq!(
            std::mem::size_of_val(&reader),
            std::mem::size_of::<usize>() * 2
        );
        // The struct itself is stateless
        assert_eq!(std::mem::size_of::<MPP8Reader>(), 0);
        let _ = read_short(&[1, 0], 0);
    }
}
