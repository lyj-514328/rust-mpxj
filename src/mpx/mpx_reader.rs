//! Corresponds to Java: `org/mpxj/mpx/MPXReader.java`
//!
//! The reader that constructs a [`ProjectFile`] by reading an MPX file — the closing class of the
//! MPX read chain.
//!
//! # Wiring (Java inheritance chain → Rust traits)
//!
//! Java is `public final class MPXReader extends AbstractProjectStreamReader`. Rust has no
//! inheritance, so it is wired per the conventions in the header of
//! [`crate::reader::abstract_project_reader`]:
//!
//! - `impl ProjectReader for MPXReader`: each interface method is given individually. Among them,
//!   `add_project_listener` forwards to [`AbstractProjectReader::add_project_listener_default`], and
//!   `read_file` / `read_all_file` / `read_all_reader` forward to
//!   [`AbstractProjectStreamReader`]'s `*_default` (the still-"abstract" `read_reader` at this level,
//!   i.e. Java's `read(InputStream)`, is implemented by this class itself);
//! - `impl AbstractProjectReader for MPXReader`: provides listener storage
//!   ([`MPXReader::project_listeners`] / [`MPXReader::project_listeners_mut`]);
//! - `impl AbstractProjectStreamReader for MPXReader {}`: this level has no methods left to implement.
//!
//! The entry point [`MPXReader::read`] corresponds one-to-one to Java's `read(InputStream)`;
//! `read_reader` only forwards (in Java `read(InputStream)` is itself this class's implementation;
//! on the Rust side [`MPXReader::read`] is kept separate to keep "the entry method name consistent
//! with Java").
//!
//! # File-header probe → byte / character switch (Java's `BufferedInputStream` + `mark` / `reset`)
//!
//! Java's flow is:
//!
//! 1. Read the first 4 bytes with `BufferedInputStream` as **bytes** and validate the `MPX` prefix;
//!    the 4th byte is the field delimiter;
//! 2. `bis.mark(1024)` is set **after reading the 1st byte**, so `bis.reset()` can only return to
//!    offset 1 (the Java comment states plainly: the first field of the first record yields `PX`
//!    rather than `MPX`);
//! 3. Read the "file creation record" with `InputStreamTokenizer` (byte-wise, no lookahead) and take
//!    the code page from it;
//! 4. Read the remaining records as **characters** with
//!    `new InputStreamReader(bis, codePage.getCharset())` + `ReaderTokenizer`.
//!
//! Rust's [`std::io::Read`] has no `mark` / `reset`, so per the advice of
//! [`crate::reader::abstract_project_stream_reader::read_stream_into_cursor`] this class first reads
//! the whole stream into memory to obtain a [`std::io::Cursor`] (`Cursor` implements [`Seek`], and
//! `seek` is `reset`):
//!
//! | Java | Rust |
//! |---|---|
//! | `new BufferedInputStream(is)` | [`read_stream_into_cursor`] → `Cursor<Vec<u8>>` |
//! | Byte-wise `bis.read()` probing 4 bytes | `cursor.read(...)` reads 4 bytes |
//! | `bis.mark(1024)` / `bis.reset()` | `cursor.seek(SeekFrom::Start(1))` |
//! | `new InputStreamTokenizer(bis)` | `Tokenizer::new(ByteSource::new(cursor))` |
//! | `new ReaderTokenizer(new InputStreamReader(bis, charset))` | `Tokenizer::new(CharSource::with_encoding(cursor, encoding))` |
//! | `codePage.getCharset()` | [`crate::CodePage::get_charset`] (falls back to [`encoding_rs::WINDOWS_1252`] when `None`, see below) |
//!
//! The key to the handoff is that both
//! [`crate::common::input_stream_tokenizer::ByteSource`] /
//! [`crate::common::reader_tokenizer::CharSource`] support "taking back the underlying data source"
//! (`Tokenizer::into_source()` → `ByteSource::into_inner()`), so after the byte stage the same
//! `Cursor` is handed unchanged to the character-stage decoder — corresponding to the same
//! `BufferedInputStream` being used first by `InputStreamTokenizer` and then by `InputStreamReader`
//! in Java.
//!
//! Trade-off note: `Record::new` (Java's `new Record(file, locale, tk, m_formats)`) needs a
//! `&mut Tokenizer<S>`, whereas
//! [`crate::common::input_stream_tokenizer::InputStreamTokenizer`] /
//! [`crate::common::reader_tokenizer::ReaderTokenizer`] only forward methods such as `nextToken` and
//! do not expose the inner `Tokenizer`; so here the `ByteSource` / `CharSource` used internally by
//! both are combined directly to construct a `Tokenizer<S>`; the semantics are exactly the same as
//! using the two wrapper types above (they are themselves thin wrappers around `Tokenizer<S>`).
//!
//! `CodePage::get_charset()` returns `None` for CP437 / CP850 (`encoding_rs` does not implement
//! these two encodings, see [`crate::CodePage`]), whereas the Java charset is always non-null; Rust
//! falls back to [`encoding_rs::WINDOWS_1252`] (also a single-byte Western encoding, closer to the
//! original file's semantics than UTF-8).
//!
//! # Record dispatch list (Java `switch (recordNumber)`, Rust `match`)
//!
//! | Record number (constant) | Handling |
//! |---|---|
//! | `PROJECT_NAMES`(80) / `DDE_OLE_CLIENT_LINKS`(81) / `COMMENTS`(0) | Silently ignored |
//! | `CURRENCY_SETTINGS`(10) | [`MPXReader::populate_currency_settings`] + `m_formats.update()` |
//! | `DEFAULT_SETTINGS`(11) | [`MPXReader::populate_default_settings`] + `m_formats.update()` |
//! | `DATE_TIME_SETTINGS`(12) | [`MPXReader::populate_date_time_settings`] + `m_formats.update()` |
//! | `BASE_CALENDAR`(20) | `addCalendar()` + [`MPXReader::populate_calendar`] (base calendar) |
//! | `BASE_CALENDAR_HOURS`(25) | [`MPXReader::populate_calendar_hours`] |
//! | `BASE_CALENDAR_EXCEPTION`(26) | [`MPXReader::populate_calendar_exception`] |
//! | `PROJECT_HEADER`(30) | [`MPXReader::populate_project_header`] + `m_formats.update()` |
//! | `RESOURCE_MODEL_TEXT`(40) / `RESOURCE_MODEL_NUMERIC`(41) | `ResourceModel::update` |
//! | `RESOURCE_RECORD`(50) | `addResource()` + [`MPXReader::populate_resource`] + events |
//! | `RESOURCE_NOTES`(51) | `setNotes` |
//! | `RESOURCE_CALENDAR`(55) / `_HOURS`(56) / `_EXCEPTION`(57) | As above (acting on the resource calendar) |
//! | `TASK_MODEL_TEXT`(60) / `TASK_MODEL_NUMERIC`(61) | `TaskModel::update` |
//! | `TASK_RECORD`(70) | `addTask()` + [`MPXReader::populate_task`] + hierarchy attachment + events |
//! | `TASK_NOTES`(71) | `setNotes` |
//! | `RECURRING_TASK`(72) | `setRecurring(true)` + `addRecurringTask()` + [`MPXReader::populate_recurring_task`] |
//! | `RESOURCE_ASSIGNMENT`(75) | `addResourceAssignment` + [`MPXReader::populate_resource_assignment`] |
//! | `RESOURCE_ASSIGNMENT_WORKGROUP_FIELDS`(76) | `addWorkgroupAssignment()` + [`MPXReader::populate_resource_assignment_workgroup_fields`] |
//! | `FILE_CREATION`(999) | [`MPXReader::populate_file_creation_record`] |
//! | Other | `Err(INVALID_RECORD)` |
//!
//! # Fields written directly to entities, bypassing the field table (Rust-specific trade-off)
//!
//! [`crate::FieldValue`] has only scalar variants (`Integer` / `Short` / `Double` / `Boolean` /
//! `String` / `Date` / `Time` / `DateTime` / `Duration` / `Rate`) and **no entity-reference or
//! collection variants**. Fields into which Java writes a "`Resource` object" or
//! "`List<Relation>`", Rust instead bypasses the field table and calls the entity's setters /
//! builders directly:
//!
//! | Java field | Java form | Rust form |
//! |---|---|---|
//! | `AssignmentField.RESOURCE` | Written through the field internally by `Task.addResourceAssignment(resource)` | [`crate::Task::add_resource_assignment`] (associated by `resource unique ID`) |
//! | `TaskField.PREDECESSORS` / `UNIQUE_ID_PREDECESSORS` | `populateRelationList` collects, then a `Relation` is built | [`MPXReader::populate_relation_list`] + [`crate::Task::add_predecessor`] (stored in the deferred list, not written to the field table) |
//! | `ResourceField.MAX_UNITS` | `resource.getAvailability().add(new Availability(...))` | [`crate::Resource::add_availability`] (a Rust-specific write path with the same semantics as Java's `getAvailability().add(...)`, see item 3 of "Differences from Java") |
//! | Enum fields (`CONSTRAINT_TYPE` / `PRIORITY` / `TYPE`) | `task.set(taskField, ConstraintType/Priority/TaskType)` | The field table encodes an `Integer` (the enum's `getValue()`), or `set_type` is called directly (see item 4 of "Differences from Java") |
//!
//! `TaskField.NOTES` / `TaskField.SUCCESSORS` are explicitly ignored in Java (MS Project does not
//! write a notes column, and MPX files usually contain only predecessor relations), and Rust
//! likewise ignores them.
//!
//! # Differences from Java
//!
//! 1. **`m_formats` is not a struct member**: `m_formats` is Java's **temporary working data** (set to
//!    null in `finally`). On the Rust side `Record` borrows `MPXJFormats` (`Record<'a>`'s lifetime
//!    parameter); if it were also a member of `self`, then `parse_record(&mut self, &record)` would
//!    simultaneously require a "field borrow" and a "whole-value mutable borrow" and fail borrow
//!    checking. So during reading it is carried in a local variable, and `m_formats.update()` is
//!    hoisted into the loop of [`MPXReader::read`]: `parse_record` returns "whether this record
//!    triggers `m_formats.update()`" (the 4 call sites that immediately follow populate in Java), and
//!    the caller performs the update after `Record` is released, exactly matching Java's semantics.
//! 2. **`m_projectConfig` is not a struct member**: Java's `ProjectConfig` is held by
//!    `ProjectContext`; Rust can only borrow it temporarily via a `Ref`
//!    (`ProjectFile::get_project_config`) and cannot store it long-term (`ProjectConfig` implements
//!    neither `Clone` nor `Rc`). So it is read on demand: [`MPXReader::with_project_config`] fetches
//!    the config fresh each time, equivalent to Java's `m_projectConfig.getXxx()`.
//! 3. **`ResourceField.MAX_UNITS`**: Java goes through `resource.getAvailability().add(new Availability(...))`.
//!    This repo's `Resource::get_availability()` only lends a read-only `Ref`, so it writes to the
//!    availability table via [`crate::Resource::add_availability`] (exactly as in Java: the value
//!    does not go into the field table, and `Resource::get_max_units()` is computed from the
//!    availability table).
//! 4. **Enum fields**: `FieldValue` has no enum variant, so Java's `task.set(taskField, enumValue)` is
//!    uniformly written to the field table on the Rust side as "enum's `getValue()` → `FieldValue::Integer`"
//!    (consistent with the internal encoding of `Task::set_priority` / `Task::set_constraint_type`,
//!    restored on read by `Task::get_priority` / `get_constraint_type`).
//! 5. **`TaskModel::get_model` / `ResourceModel::get_model` need `&mut self`** (Java returns the
//!    internal array directly), and `populate_task` / `populate_resource` also call `&mut self`
//!    methods in the loop (such as `populate_relation_list`). Rust instead **copies** the model into
//!    a local `Vec<i32>` and iterates that; the semantics are unchanged (the model has at most 154
//!    entries).
//! 6. **`m_projectFile.addTask()`**: Rust's `ProjectFile::add_task` re-enters the borrow while
//!    holding `borrow_mut()`, so the borrow-free entry point it provides,
//!    [`crate::ProjectFile::add_task_shared`], is used (same semantics as Java's `addTask()`).
//! 7. **Collections and entities**: `List` → `Vec`, `Map` → `HashMap`; entities are always shared as
//!    `Rc<RefCell<..>>` (this crate's convention). The key of `resourceCalendarMap` is `Integer`
//!    (may be null), expressed on the Rust side as `HashMap<Option<i32>, usize>`.
//! 8. **`Resource` ID / unique ID access goes through traits**: `get_id` / `set_id` /
//!    `get_unique_id` / `set_unique_id` on [`crate::Resource`] are methods of the
//!    `ProjectEntityWithID` / `ProjectEntityWithMutableUniqueID` / `ProjectEntityWithUniqueID`
//!    traits, so this module imports these three traits.
//! 9. **Exception wrapping in `read`**: Java's `catch (Exception ex)` re-wraps **all** exceptions
//!    (including `INVALID_FILE` / `INVALID_RECORD`) as
//!    `MPXJException(READ_ERROR + " (failed at line N)", ex)`; Rust does the same
//!    ([`crate::MpxjError::with_cause`]), with a message like `Error reading file (failed at line 3): ...`.
//! 10. **Cleanup in `finally`**: Java nulls out all temporary members; on the Rust side
//!     [`MPXReader::reset_transient`] performs the same cleanup (the return value has already been
//!     cloned, so it is unaffected).
//! 11. **`get_time_from_integer`**: Java's `LocalTime.ofSecondOfDay` normalizes seconds beyond one day
//!     modulo 86400; Rust explicitly `rem_euclid(86_400)` before constructing, giving the same behavior.
//! 12. **`DayOfWeekHelper.getInstance` returns `Option<Weekday>`** (may be null in Java): when Rust
//!     cannot resolve the weekday it skips that "calendar working hours" record (Java passes null to
//!     `addCalendarHours`).
//! 13. **Splitting the relation string**: Java uses `String.indexOf(char, int)` / `substring`
//!     (UTF-16 indices), while Rust uses byte indices (`str::find` / slicing), which are equivalent
//!     for ASCII separators and ASCII relation strings (the `index + 2` spot instead "takes two
//!     `char`s", which is also safe for supplementary-plane characters).
//! 14. **How `m_projectFile.updateStructure()` is called**: as in Java, it is called in place,
//!     [`crate::ProjectFile::update_structure`]. The previous workaround of "temporarily moving the
//!     task container out of `ProjectFile` and rebuilding it" has been removed —
//!     `TaskContainer::update_structure` now accepts the automatic-numbering switch passed in by the
//!     caller, as does `Task::add_child_task_with_auto_outline_level`, and the container side no
//!     longer calls back into the parent file, so calling it while holding a mutable borrow of
//!     `ProjectFile` does not re-enter a borrow conflict. The "number of top-level tasks" counter for
//!     WBS / outline numbering is passed in by the container as its own `child_tasks.len()`, matching
//!     both the timing and the result of Java reading `getChildTasks().size()`.
//! 15. **Wiring of the task ID → task map**: [`crate::Task::set_id`], like Java's `Task.setID`, calls
//!     back into [`crate::TaskContainer`]'s `unmap_id` / `map_id` (that callback relies on
//!     `Task::set_self_reference` being wired when the task enters the container), so this class needs
//!     no extra compensation. When the task is not yet wired, or the parent file is being borrowed,
//!     the callback degrades to "does not maintain the ID index" without panicking.
//! 16. **Unmapped MPX field numbers**: in task records Java explicitly checks `taskField == null` and
//!     skips; in resource records Java does not check (`MPXResourceField.getMpxjField` returning null
//!     falls into the `default` branch and NPEs). Rust skips the field in both cases (no panic).
//! 17. **When field text cannot be parsed as an integer** (a task's `Objects` / `OutlineLevel` / `ID` /
//!     `UniqueID`): Java's `Integer.valueOf` throws `NumberFormatException`, which `read`'s catch wraps
//!     as `READ_ERROR (failed at line N)`; Rust returns an equivalent-text [`crate::MpxjError`]
//!     (`For input string: "..."`), and [`MPXReader::read`] performs the same wrapping.
//! 18. **Format parsing in `populateTask` goes through [`Record`]'s accessors**: Java in
//!     `populateTask` writes `m_formats.getPercentageDecimalFormat().parse(field)` /
//!     `parseDateTime(field)` etc. directly, while Rust uses `record.get_percentage(i)` /
//!     `record.get_currency(i)` / `record.get_float(i)` / `record.get_date_time(i)` — the internals of
//!     these [`Record`] accessors call **the same set** of `MPXJFormats` format objects, so parse
//!     results and error messages match Java (only the message for `NUMBER1..5` is rewritten to Java's
//!     `"Failed to parse number"`, since `Record::get_float`'s message is `"Failed to parse float"`).
//!     This way `MPXJFormats` need not be passed down through every populate method; only call sites
//!     that need the format object **itself** are exceptions: `populate_task` retrieves it via
//!     [`Record::get_formats`], and `populate_relation` (which does not receive a `Record`) has it
//!     passed down through the layers by [`MPXReader::read_impl`] (both are used for
//!     `DurationUtility.getInstance(..., m_formats.getDurationDecimalFormat(), ...)`).
//!
//! # Skipped Java code and rationale
//!
//! | Java | Reason |
//! |---|---|
//! | `MPXWriter` | Not ported per the task requirements (this file contains only the read chain) |
//! | Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable` | Not used by this class |
//!
//! Skipped Java members: none (the members listed in the table above have their reasons stated).

use std::any::TypeId;
use std::cell::RefCell;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::rc::Rc;

use chrono::{NaiveTime, Weekday};

use crate::availability::Availability;
use crate::common::day_of_week_helper::DayOfWeekHelper;
use crate::common::input_stream_tokenizer::ByteSource;
use crate::common::local_date_helper::LocalDateHelper;
use crate::common::local_date_time_helper::LocalDateTimeHelper;
use crate::common::number_helper::NumberHelper;
use crate::common::reader_tokenizer::CharSource;
use crate::common::slack_helper::SlackHelper;
use crate::common::tokenizer::{Tokenizer, TT_EOF};
use crate::cost_rate_table::CostRateTable;
use crate::cost_rate_table_entry::CostRateTableEntry;
use crate::duration::Duration;
use crate::listener::project_listener::ProjectListener;
use crate::mpx::constraint_type_utility::ConstraintTypeUtility;
use crate::mpx::duration_utility::DurationUtility;
use crate::mpx::locale_data::LocaleData;
use crate::mpx::locale_utility::{equals_ignore_case, LocaleUtility, MpxLocale};
use crate::mpx::mpx_constants::MPXConstants;
use crate::mpx::mpx_resource_field::MPXResourceField;
use crate::mpx::mpx_task_field::MPXTaskField;
use crate::mpx::mpxj_formats::MPXJFormats;
use crate::mpx::priority_utility::PriorityUtility;
use crate::mpx::record::Record;
use crate::mpx::recurrence_utility::RecurrenceUtility;
use crate::mpx::relation_type_utility::RelationTypeUtility;
use crate::mpx::resource_model::ResourceModel;
use crate::mpx::task_model::TaskModel;
use crate::mpxj_enum::MpxjEnum;
use crate::project_calendar::ProjectCalendar;
use crate::project_calendar_days::ProjectCalendarDays;
use crate::project_calendar_exception::ProjectCalendarException;
use crate::project_calendar_hours::ProjectCalendarHours;
use crate::project_config::ProjectConfig;
use crate::project_entity_with_id::ProjectEntityWithID;
use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
use crate::project_file::ProjectFile;
use crate::project_properties::ProjectProperties;
use crate::rate::Rate;
use crate::reader::abstract_project_reader::{AbstractProjectReader, SharedProjectListener};
use crate::reader::abstract_project_stream_reader::{
    read_stream_into_cursor, AbstractProjectStreamReader,
};
use crate::reader::project_reader::ProjectReader;
use crate::recurrence_type::RecurrenceType;
use crate::relation::Builder as RelationBuilder;
use crate::resource::Resource;
use crate::resource_assignment::ResourceAssignment;
use crate::resource_assignment_workgroup_fields::ResourceAssignmentWorkgroupFields;
use crate::resource_field::ResourceField;
use crate::task::Task;
use crate::task_field::TaskField;
use crate::task_type::TaskType;
use crate::time_unit::TimeUnit;
use crate::{CalendarType, DayType, EventManager, FieldKey, FieldValue, LocalTimeRange, MpxjError, MpxjResult};

/// Task relationship data awaiting deferred processing.
///
/// Corresponds to Java: `protected static class DeferredRelationship`
///
/// Why deferred: predecessor relationships in an MPX file may be **forward references** (pointing to
/// tasks that appear later in the file), so the relationship text is collected first and parsed
/// uniformly after the whole file is read, building [`Relation`]s.
pub struct DeferredRelationship {
    /// The (successor) task the relationship belongs to.
    ///
    /// Corresponds to Java: `private Task m_task;`
    task: Option<Rc<RefCell<Task>>>,
    /// The source field (`PREDECESSORS` looks up by ID, `UNIQUE_ID_PREDECESSORS` by unique ID).
    ///
    /// Corresponds to Java: `private TaskField m_field;`
    field: Option<TaskField>,
    /// Relationship text (may contain multiple relationships separated by the field delimiter).
    ///
    /// Corresponds to Java: `private String m_data;`
    data: Option<String>,
}

impl DeferredRelationship {
    /// Construct an empty deferred relationship (an implicit no-arg constructor in Java).
    ///
    /// Corresponds to Java: `DeferredRelationship()`
    pub fn new() -> Self {
        Self {
            task: None,
            field: None,
            data: None,
        }
    }

    /// Get the successor task.
    ///
    /// Corresponds to Java: `public Task getTask()`
    pub fn get_task(&self) -> Option<Rc<RefCell<Task>>> {
        self.task.clone()
    }

    /// Set the successor task.
    ///
    /// Corresponds to Java: `public void setTask(Task task)`
    pub fn set_task(&mut self, task: Rc<RefCell<Task>>) {
        self.task = Some(task);
    }

    /// Get the source field.
    ///
    /// Corresponds to Java: `public TaskField getField()`
    pub fn get_field(&self) -> Option<TaskField> {
        self.field
    }

    /// Set the source field.
    ///
    /// Corresponds to Java: `public void setField(TaskField field)`
    pub fn set_field(&mut self, field: TaskField) {
        self.field = Some(field);
    }

    /// Get the relationship text.
    ///
    /// Corresponds to Java: `public String getData()`
    pub fn get_data(&self) -> Option<&str> {
        self.data.as_deref()
    }

    /// Set the relationship text.
    ///
    /// Corresponds to Java: `public void setData(String data)`
    pub fn set_data(&mut self, data: String) {
        self.data = Some(data);
    }
}

impl Default for DeferredRelationship {
    /// Corresponds to Java: implicit no-arg constructor.
    fn default() -> Self {
        Self::new()
    }
}

/// Constructs a [`ProjectFile`] instance by reading an MPX file.
///
/// Corresponds to Java: `public final class MPXReader extends AbstractProjectStreamReader`
pub struct MPXReader {
    /// Registered listeners (Rust-specific: corresponds to the `m_projectListeners` field of Java's abstract base class).
    ///
    /// Corresponds to Java: `AbstractProjectReader.m_projectListeners`
    listeners: Vec<SharedProjectListener>,

    /// Target locale.
    ///
    /// Corresponds to Java: `private Locale m_locale = Locale.ENGLISH;`
    m_locale: MpxLocale,

    /// Whether to ignore text-form task / resource table definition records.
    ///
    /// Corresponds to Java: `private boolean m_ignoreTextModels = true;`
    m_ignore_text_models: bool,

    /*
     * The following is temporary working data, corresponding to the same-named members in Java
     * (Java nulls them all out in `finally`; Rust performs the same cleanup via
     * [`MPXReader::reset_transient`]).
     */

    /// The project file being read.
    ///
    /// Corresponds to Java: `private ProjectFile m_projectFile;`
    m_project_file: Option<Rc<RefCell<ProjectFile>>>,

    /// The task produced by the most recent task record.
    ///
    /// Corresponds to Java: `private Task m_lastTask;`
    m_last_task: Option<Rc<RefCell<Task>>>,

    /// The resource produced by the most recent resource record.
    ///
    /// Corresponds to Java: `private Resource m_lastResource;`
    m_last_resource: Option<Rc<RefCell<Resource>>>,

    /// The calendar produced by the most recent resource calendar record.
    ///
    /// Corresponds to Java: `private ProjectCalendar m_lastResourceCalendar;`
    m_last_resource_calendar: Option<Rc<RefCell<ProjectCalendar>>>,

    /// The assignment produced by the most recent resource assignment record.
    ///
    /// Corresponds to Java: `private ResourceAssignment m_lastResourceAssignment;`
    m_last_resource_assignment: Option<Rc<RefCell<ResourceAssignment>>>,

    /// The calendar produced by the most recent base calendar record.
    ///
    /// Corresponds to Java: `private ProjectCalendar m_lastBaseCalendar;`
    m_last_base_calendar: Option<Rc<RefCell<ProjectCalendar>>>,

    /// Whether a resource table definition record has been read.
    ///
    /// Corresponds to Java: `private boolean m_resourceTableDefinition;`
    m_resource_table_definition: bool,

    /// Whether a task table definition record has been read.
    ///
    /// Corresponds to Java: `private boolean m_taskTableDefinition;`
    m_task_table_definition: bool,

    /// Task field model (column position → MPX field number).
    ///
    /// Corresponds to Java: `private TaskModel m_taskModel;`
    m_task_model: Option<TaskModel>,

    /// Resource field model (column position → MPX field number).
    ///
    /// Corresponds to Java: `private ResourceModel m_resourceModel;`
    m_resource_model: Option<ResourceModel>,

    /// Field delimiter (taken from the 4th byte of the file header).
    ///
    /// Corresponds to Java: `private char m_delimiter;`
    m_delimiter: char,

    /// Pending (possibly forward-referencing) task relationships.
    ///
    /// Corresponds to Java: `private List<DeferredRelationship> m_deferredRelationships;`
    m_deferred_relationships: Vec<DeferredRelationship>,

    /// The default calendar name specified in the project header record.
    ///
    /// Corresponds to Java: `private String m_defaultCalendarName;`
    m_default_calendar_name: Option<String>,

    /// The outline level used by the first task record in the file.
    ///
    /// Corresponds to Java: `private int m_baseOutlineLevel;`
    m_base_outline_level: i32,
}

impl MPXReader {
    /// Construct the reader: the default locale is English and text-form table definition records are ignored.
    ///
    /// Corresponds to Java: `MPXReader`'s implicit no-arg constructor (member defaults are in
    /// `private Locale m_locale = Locale.ENGLISH;` and `private boolean m_ignoreTextModels = true;`)
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
            m_locale: MpxLocale::En,
            m_ignore_text_models: true,
            m_project_file: None,
            m_last_task: None,
            m_last_resource: None,
            m_last_resource_calendar: None,
            m_last_resource_assignment: None,
            m_last_base_calendar: None,
            m_resource_table_definition: false,
            m_task_table_definition: false,
            m_task_model: None,
            m_resource_model: None,
            m_delimiter: ',',
            m_deferred_relationships: Vec::new(),
            m_default_calendar_name: None,
            m_base_outline_level: -1,
        }
    }

    /// Get the current locale.
    ///
    /// Corresponds to Java: `public Locale getLocale()`
    pub fn get_locale(&self) -> MpxLocale {
        self.m_locale
    }

    /// Set the locale.
    ///
    /// Corresponds to Java: `public void setLocale(Locale locale)`
    pub fn set_locale(&mut self, locale: MpxLocale) {
        self.m_locale = locale;
    }

    /// Get the locales supported by this class.
    ///
    /// Corresponds to Java: `public static Locale[] getSupportedLocales()`
    pub fn get_supported_locales() -> &'static [MpxLocale] {
        LocaleUtility::get_supported_locales()
    }

    /// Set "whether to ignore text-form task / resource table definition records".
    ///
    /// Corresponds to Java: `public void setIgnoreTextModels(boolean flag)`
    ///
    /// Why ignore these records: in MPX files generated by some tools, the text-form table definition
    /// uses wrong field names (but the field numbers in the numeric-form records are correct), so
    /// ignoring the text form sidesteps the problem.
    pub fn set_ignore_text_models(&mut self, flag: bool) {
        self.m_ignore_text_models = flag;
    }

    /// Get "whether to ignore text-form task / resource table definition records".
    ///
    /// Corresponds to Java: `public boolean getIgnoreTextModels()`
    pub fn get_ignore_text_models(&self) -> bool {
        self.m_ignore_text_models
    }

    /// Read an MPX file (this class's read entry point).
    ///
    /// Corresponds to Java: `@Override public ProjectFile read(InputStream is) throws MPXJException`
    ///
    /// As in Java: any failure (including an invalid file header, an invalid record number, or a field
    /// parse failure) is wrapped as `READ_ERROR + " (failed at line N)"` before returning; temporary
    /// working data is cleaned up at the end regardless of success or failure.
    pub fn read(&mut self, reader: &mut dyn Read) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        let mut line = 1usize;

        let result = self.read_impl(reader, &mut line);

        // Java: finally { m_projectFile = null; ... m_deferredRelationships = null; }
        self.reset_transient();

        // Java: catch (Exception ex) { throw new MPXJException(MPXJException.READ_ERROR
        //       + " (failed at line " + line + ")", ex); }
        match result {
            Ok(file) => Ok(file),
            Err(ex) => Err(MpxjError::with_cause(
                format!("{} (failed at line {})", MpxjError::READ_ERROR, line),
                &ex,
            )),
        }
    }

    /// The implementation body of read (`line` is held by the caller and used for error messages).
    ///
    /// Corresponds to Java: the `try` block body of `read(InputStream)`
    fn read_impl(
        &mut self,
        reader: &mut dyn Read,
        line: &mut usize,
    ) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        // Java: BufferedInputStream bis = new BufferedInputStream(is); byte[] data = new byte[4];
        // Rust has no mark/reset, so instead the whole stream is read into memory and wrapped in a Cursor (see the file header).
        let mut cursor: Cursor<Vec<u8>> = read_stream_into_cursor(reader)?;

        // Java:
        // data[0] = (byte) bis.read(); bis.mark(1024);
        // data[1] = (byte) bis.read(); data[2] = (byte) bis.read(); data[3] = (byte) bis.read();
        let mut data = [0u8; 4];
        let mut count = 0usize;
        while count < data.len() {
            match cursor.read(&mut data[count..])? {
                0 => break,
                read => count += read,
            }
        }

        // Java: if ((data[0] != 'M') || (data[1] != 'P') || (data[2] != 'X'))
        //       { throw new MPXJException(MPXJException.INVALID_FILE); }
        // Java gets -1 (cast to 0xFF) for input shorter than 4 bytes and also falls here;
        // Rust reports INVALID_FILE for both "fewer than 4 bytes" and "prefix mismatch".
        if count < data.len() || &data[0..3] != b"MPX" {
            return Err(MpxjError::new(MpxjError::INVALID_FILE));
        }

        // Java: m_projectFile = new ProjectFile();
        let file = ProjectFile::new();
        self.m_project_file = Some(Rc::clone(&file));

        // Java: m_projectConfig = m_projectFile.getProjectConfig();
        //       m_projectConfig.setAutoTaskID(false); ... setAutoWBS(false);
        {
            let context = file.borrow().get_project_context();
            let mut context_ref = context.borrow_mut();
            let config = context_ref.get_project_config_mut();
            config.set_auto_task_id(false);
            config.set_auto_task_unique_id(false);
            config.set_auto_resource_id(false);
            config.set_auto_resource_unique_id(false);
            config.set_auto_outline_level(false);
            config.set_auto_outline_number(false);
            config.set_auto_wbs(false);
        }

        // Java: addListenersToProject(m_projectFile);
        self.add_listeners_to_project(&file);

        // Java: LocaleUtility.setLocale(m_projectFile.getProjectProperties(), m_locale);
        LocaleUtility::set_locale(file.borrow_mut().get_project_properties_mut(), self.m_locale);

        // Java: m_delimiter = (char) data[3];
        //       m_projectFile.getProjectProperties().setMpxDelimiter(m_delimiter);
        //       ...setFileApplication("Microsoft"); ...setFileType("MPX");
        self.m_delimiter = data[3] as char;
        {
            let mut file_ref = file.borrow_mut();
            let properties = file_ref.get_project_properties_mut();
            properties.set_mpx_delimiter(self.m_delimiter);
            properties.set_file_application(Some("Microsoft".to_owned()));
            properties.set_file_type(Some("MPX".to_owned()));
        }

        // Java: m_taskModel = new TaskModel(m_projectFile, m_locale, null); m_taskModel.setLocale(m_locale);
        let mut task_model = TaskModel::new(Rc::downgrade(&file), self.m_locale);
        task_model.set_locale(self.m_locale);
        self.m_task_model = Some(task_model);

        // Java: m_resourceModel = new ResourceModel(m_projectFile, m_locale, null);
        let mut resource_model = ResourceModel::new(Rc::downgrade(&file), self.m_locale);
        resource_model.set_locale(self.m_locale);
        self.m_resource_model = Some(resource_model);

        // Java: m_baseOutlineLevel = -1;
        self.m_base_outline_level = -1;

        // Java: m_formats = new MPXJFormats(m_locale, LocaleData.getString(m_locale, LocaleData.NA), m_projectFile);
        //       m_deferredRelationships = new ArrayList<>();
        let null_text = LocaleData::get_string(self.m_locale, LocaleData::NA).unwrap_or(LocaleData::NA);
        let mut formats = MPXJFormats::new(self.m_locale, null_text, Rc::downgrade(&file));
        self.m_deferred_relationships.clear();

        // Java: bis.reset();
        // Java's mark is set after reading the 1st byte, so it can only return to offset 1: the first field
        // of the first record is "PX" (see the Java source comment); here it equivalently seeks to 1.
        cursor.seek(SeekFrom::Start(1))?;

        //
        // Java:
        // Tokenizer tk = new InputStreamTokenizer(bis); tk.setDelimiter(m_delimiter);
        // parseRecord(Integer.valueOf(MPXConstants.FILE_CREATION_RECORD_NUMBER),
        //             new Record(m_projectFile, m_locale, tk, m_formats));
        // ++line;
        //
        // At this point reading is byte-wise and the text is not charset-decoded (the file creation record contains only ASCII).
        //
        let mut tk = Tokenizer::new(ByteSource::new(cursor));
        tk.set_delimiter(self.m_delimiter);

        let record = Record::new(&file, self.m_locale, &mut tk, &formats)?;
        self.parse_record(MPXConstants::FILE_CREATION_RECORD_NUMBER, &record)?;
        drop(record);
        *line += 1;

        //
        // Java:
        // InputStreamReader reader = new InputStreamReader(bis,
        //     m_projectFile.getProjectProperties().getMpxCodePage().getCharset());
        // tk = new ReaderTokenizer(reader); tk.setDelimiter(m_delimiter);
        //
        // Take back the same data source from the byte stage and hand it to the character-decoding tokenizer (see the file header).
        //
        let cursor = tk.into_source().into_inner();
        let encoding = file
            .borrow()
            .get_project_properties()
            .get_mpx_code_page()
            .get_charset()
            // The Java charset is always non-null (provided by CharsetHelper); CP437 / CP850 have no implementation
            // in encoding_rs, so this falls back to WINDOWS_1252 as a single-byte Western encoding (see the file header).
            .unwrap_or(encoding_rs::WINDOWS_1252);

        let mut tk = Tokenizer::new(CharSource::with_encoding(cursor, encoding));
        tk.set_delimiter(self.m_delimiter);

        // Java: while (tk.getType() != Tokenizer.TT_EOF) { ... }
        while tk.get_type() != TT_EOF {
            let record = Record::new(&file, self.m_locale, &mut tk, &formats)?;
            let number = record.get_record_number();

            // Java: if (number != null) { parseRecord(number, record); }
            let needs_formats_update = match number {
                Some(number) => self.parse_record(number, &record)?,
                None => false,
            };

            // Java: m_formats.update() is inside parseRecord's branches;
            // Rust must wait until Record is released before mutably borrowing formats (see item 1 of the file-header differences).
            drop(record);
            if needs_formats_update {
                formats.update();
            }

            *line += 1;
        }

        // Java: processDeferredRelationships(); validateCalendars();
        // Rust-specific: Java's `populateRelation` reads the member `m_formats` directly, but this repo's format
        // collection is a local variable (see item 1 of the file-header differences), so it is passed down through the layers.
        self.process_deferred_relationships(&formats)?;
        self.validate_calendars()?;

        // Java: m_projectFile.updateStructure();
        //
        // Called in place as in Java: [`crate::ProjectFile::update_structure`] first reads the automatic-numbering
        // switches from `ProjectContext` and passes them as parameters to the task container; the container side no
        // longer calls back into the parent file, so calling it while holding a mutable borrow of `ProjectFile` does not re-enter a borrow conflict.
        file.borrow_mut().update_structure();

        // Java: m_projectConfig.setAutoCalendarUniqueID(false);
        {
            let context = file.borrow().get_project_context();
            context
                .borrow_mut()
                .get_project_config_mut()
                .set_auto_calendar_unique_id(false);
        }

        // Java: m_projectFile.readComplete(); return m_projectFile;
        file.borrow_mut().read_complete();

        Ok(file)
    }

    /// Clean up temporary working data (corresponding to Java `read`'s `finally` block).
    ///
    /// Corresponds to Java: `finally { m_projectFile = null; ... m_deferredRelationships = null; }`
    fn reset_transient(&mut self) {
        self.m_project_file = None;
        self.m_last_task = None;
        self.m_last_resource = None;
        self.m_last_resource_calendar = None;
        self.m_last_resource_assignment = None;
        self.m_last_base_calendar = None;
        self.m_resource_table_definition = false;
        self.m_task_table_definition = false;
        self.m_task_model = None;
        self.m_resource_model = None;
        self.m_deferred_relationships.clear();
    }

    /// Get the current project file handle (Java uses the member `m_projectFile` directly).
    ///
    /// Rust-specific: Java's member is always non-null; here `Err` expresses the degraded case of "reading has not started yet".
    fn file(&self) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        self.m_project_file
            .clone()
            .ok_or_else(|| MpxjError::new(MpxjError::READ_ERROR))
    }

    /// Access the project configuration read-only.
    ///
    /// Corresponds to Java: `m_projectConfig.getXxx()`
    ///
    /// Returning `None` means the project file has not been created yet or is mutably borrowed (this cannot happen in Java).
    fn with_project_config<R>(&self, read: impl FnOnce(&ProjectConfig) -> R) -> Option<R> {
        let file = self.m_project_file.as_ref()?;
        let file_ref = file.try_borrow().ok()?;
        Some(read(&file_ref.get_project_config()))
    }

    /// Get a boolean flag from the project configuration (falls back to `false` when unreadable, corresponding to Java's "the config always exists").
    ///
    /// Corresponds to Java: reads such as `m_projectConfig.getXxx()`
    fn project_config_flag(&self, read: impl FnOnce(&ProjectConfig) -> bool) -> bool {
        self.with_project_config(read).unwrap_or(false)
    }

    /// Fire a project event.
    ///
    /// Corresponds to Java: `m_eventManager.fireXxxEvent(...)`
    ///
    /// Java's `m_eventManager` is taken from `m_projectFile.getEventManager()` (i.e. the project context's event
    /// manager, see the [`crate::project_file`] file header), and Rust likewise obtains it through the context;
    /// on a borrow conflict (which cannot happen in Java) it skips dispatching without panicking.
    fn with_event_manager(&self, fire: impl FnOnce(&mut EventManager)) {
        let Some(file) = self.m_project_file.as_ref() else {
            return;
        };

        let context = match file.try_borrow() {
            Ok(file) => file.get_project_context(),
            Err(_) => return,
        };

        match context.try_borrow_mut() {
            Ok(mut context) => fire(context.get_event_manager_mut()),
            Err(_) => return,
        }
    }

    /// Parse one MPX record.
    ///
    /// Corresponds to Java: `private void parseRecord(Integer recordNumber, Record record) throws MPXJException`
    ///
    /// The return value indicates whether the `m_formats.update()` that immediately follows populate in Java
    /// needs to be called (Java calls it directly in 4 branches; Rust hoists it to the caller due to borrow
    /// constraints, see item 1 of the file-header differences).
    fn parse_record(&mut self, record_number: i32, record: &Record<'_>) -> MpxjResult<bool> {
        let needs_formats_update = match record_number {
            // Java: case PROJECT_NAMES / DDE_OLE_CLIENT_LINKS / COMMENTS: silently ignored
            MPXConstants::PROJECT_NAMES_RECORD_NUMBER
            | MPXConstants::DDE_OLE_CLIENT_LINKS_RECORD_NUMBER
            | MPXConstants::COMMENTS_RECORD_NUMBER => false,

            MPXConstants::CURRENCY_SETTINGS_RECORD_NUMBER => {
                let file = self.file()?;
                let mut file_ref = file.borrow_mut();
                Self::populate_currency_settings(record, file_ref.get_project_properties_mut());
                // Java: m_formats.update();
                true
            }

            MPXConstants::DEFAULT_SETTINGS_RECORD_NUMBER => {
                let file = self.file()?;
                let mut file_ref = file.borrow_mut();
                Self::populate_default_settings(record, file_ref.get_project_properties_mut())?;
                true
            }

            MPXConstants::DATE_TIME_SETTINGS_RECORD_NUMBER => {
                let file = self.file()?;
                let mut file_ref = file.borrow_mut();
                Self::populate_date_time_settings(record, file_ref.get_project_properties_mut());
                true
            }

            MPXConstants::BASE_CALENDAR_RECORD_NUMBER => {
                let file = self.file()?;
                let calendar = file.borrow_mut().add_calendar();
                self.m_last_base_calendar = Some(Rc::clone(&calendar));
                self.populate_calendar(record, &calendar, true);
                false
            }

            MPXConstants::BASE_CALENDAR_HOURS_RECORD_NUMBER => {
                if let Some(calendar) = self.m_last_base_calendar.clone() {
                    Self::populate_calendar_hours(record, &calendar);
                }
                false
            }

            MPXConstants::BASE_CALENDAR_EXCEPTION_RECORD_NUMBER => {
                if let Some(calendar) = self.m_last_base_calendar.clone() {
                    Self::populate_calendar_exception(record, &calendar);
                }
                false
            }

            MPXConstants::PROJECT_HEADER_RECORD_NUMBER => {
                let file = self.file()?;
                let mut file_ref = file.borrow_mut();
                self.populate_project_header(record, file_ref.get_project_properties_mut())?;
                true
            }

            MPXConstants::RESOURCE_MODEL_TEXT_RECORD_NUMBER => {
                if !self.m_resource_table_definition && !self.m_ignore_text_models {
                    if let Some(model) = self.m_resource_model.as_mut() {
                        model.update(record, true)?;
                    }
                    self.m_resource_table_definition = true;
                }
                false
            }

            MPXConstants::RESOURCE_MODEL_NUMERIC_RECORD_NUMBER => {
                if !self.m_resource_table_definition {
                    if let Some(model) = self.m_resource_model.as_mut() {
                        model.update(record, false)?;
                    }
                    self.m_resource_table_definition = true;
                }
                false
            }

            MPXConstants::RESOURCE_RECORD_NUMBER => {
                let file = self.file()?;
                let resource = file.borrow_mut().add_resource();
                self.m_last_resource = Some(Rc::clone(&resource));

                // Java: m_lastResource.disableEvents(); populateResource(...); enableEvents();
                resource.borrow_mut().disable_events();
                self.populate_resource(&resource, record)?;
                resource.borrow_mut().enable_events();

                self.with_event_manager(|manager| manager.fire_resource_read_event(&resource));
                false
            }

            MPXConstants::RESOURCE_NOTES_RECORD_NUMBER => {
                if let Some(resource) = self.m_last_resource.clone() {
                    resource.borrow_mut().set_notes(record.get_string(0));
                }
                false
            }

            MPXConstants::RESOURCE_CALENDAR_RECORD_NUMBER => {
                if let Some(resource) = self.m_last_resource.clone() {
                    let calendar = resource.borrow_mut().add_calendar()?;
                    self.m_last_resource_calendar = Some(Rc::clone(&calendar));
                    self.populate_calendar(record, &calendar, false);
                }
                false
            }

            MPXConstants::RESOURCE_CALENDAR_HOURS_RECORD_NUMBER => {
                if let Some(calendar) = self.m_last_resource_calendar.clone() {
                    Self::populate_calendar_hours(record, &calendar);
                }
                false
            }

            MPXConstants::RESOURCE_CALENDAR_EXCEPTION_RECORD_NUMBER => {
                if let Some(calendar) = self.m_last_resource_calendar.clone() {
                    Self::populate_calendar_exception(record, &calendar);
                }
                false
            }

            MPXConstants::TASK_MODEL_TEXT_RECORD_NUMBER => {
                if !self.m_task_table_definition && !self.m_ignore_text_models {
                    if let Some(model) = self.m_task_model.as_mut() {
                        model.update(record, true)?;
                    }
                    self.m_task_table_definition = true;
                }
                false
            }

            MPXConstants::TASK_MODEL_NUMERIC_RECORD_NUMBER => {
                if !self.m_task_table_definition {
                    if let Some(model) = self.m_task_model.as_mut() {
                        model.update(record, false)?;
                    }
                    self.m_task_table_definition = true;
                }
                false
            }

            MPXConstants::TASK_RECORD_NUMBER => {
                let file = self.file()?;
                let task = ProjectFile::add_task_shared(&file);
                self.m_last_task = Some(Rc::clone(&task));

                // Java: m_lastTask.disableEvents(); populateTask(record, m_lastTask); enableEvents();
                task.borrow_mut().disable_events();
                self.populate_task(record, &task)?;
                task.borrow_mut().enable_events();

                let outline_level =
                    NumberHelper::get_int(task.borrow().get_outline_level().map(f64::from));

                if self.m_base_outline_level == -1 {
                    self.m_base_outline_level = outline_level;
                }

                if outline_level != self.m_base_outline_level {
                    // Java: List<Task> childTasks = m_projectFile.getChildTasks();
                    let child_tasks = file.borrow().get_child_tasks();

                    let Some(last_child) = child_tasks.last() else {
                        return Err(MpxjError::new(MpxjError::INVALID_OUTLINE));
                    };

                    // Java: childTasks.get(childTasks.size() - 1).addChildTask(m_lastTask, outlineLevel);
                    Task::add_child_task_at_level(last_child, &task, outline_level);
                }

                self.with_event_manager(|manager| manager.fire_task_read_event(&task));
                false
            }

            MPXConstants::TASK_NOTES_RECORD_NUMBER => {
                if let Some(task) = self.m_last_task.clone() {
                    task.borrow_mut().set_notes(record.get_string(0));
                }
                false
            }

            MPXConstants::RECURRING_TASK_RECORD_NUMBER => {
                if let Some(task) = self.m_last_task.clone() {
                    // Java: m_lastTask.setRecurring(true);
                    task.borrow_mut().set_recurring(true);

                    // Java: RecurringTask task = m_lastTask.addRecurringTask();
                    //       populateRecurringTask(record, task);
                    self.populate_recurring_task(record, &task)?;
                }
                false
            }

            MPXConstants::RESOURCE_ASSIGNMENT_RECORD_NUMBER => {
                if let Some(task) = self.m_last_task.clone() {
                    let resource = self.get_resource_for_assignment(record);
                    let assignment = task
                        .borrow_mut()
                        .add_resource_assignment(resource.as_ref());

                    if let Some(assignment) = assignment {
                        self.m_last_resource_assignment = Some(Rc::clone(&assignment));

                        // Java: m_lastResourceAssignment.disableEvents();
                        //       populateResourceAssignment(record, m_lastResourceAssignment);
                        //       m_lastResourceAssignment.enableEvents();
                        assignment.borrow_mut().disable_events();
                        self.populate_resource_assignment(record, &assignment)?;
                        assignment.borrow_mut().enable_events();
                    }
                }
                false
            }

            MPXConstants::RESOURCE_ASSIGNMENT_WORKGROUP_FIELDS_RECORD_NUMBER => {
                // Java:
                // if (m_lastResourceAssignment != null) {
                //    ResourceAssignmentWorkgroupFields workgroup =
                //        m_lastResourceAssignment.addWorkgroupAssignment();
                //    populateResourceAssignmentWorkgroupFields(record, workgroup);
                // }
                if let Some(assignment) = self.m_last_resource_assignment.clone() {
                    let mut assignment_ref = assignment.borrow_mut();
                    let workgroup = assignment_ref.add_workgroup_assignment();
                    Self::populate_resource_assignment_workgroup_fields(record, workgroup);
                }
                false
            }

            MPXConstants::FILE_CREATION_RECORD_NUMBER => {
                let file = self.file()?;
                let mut file_ref = file.borrow_mut();
                Self::populate_file_creation_record(record, file_ref.get_project_properties_mut());
                false
            }

            // Java: default: throw new MPXJException(MPXJException.INVALID_RECORD);
            _ => return Err(MpxjError::new(MpxjError::INVALID_RECORD)),
        };

        Ok(needs_formats_update)
    }

    /// Populate currency settings.
    ///
    /// Corresponds to Java: `private void populateCurrencySettings(Record record, ProjectProperties properties)`
    fn populate_currency_settings(record: &Record<'_>, properties: &mut ProjectProperties) {
        properties.set_currency_symbol(record.get_string(0));
        properties.set_symbol_position(Some(record.get_currency_symbol_position(1)));
        properties.set_currency_digits(record.get_integer(2));

        // Java: Character c = record.getCharacter(3); if (c != null) properties.setThousandsSeparator(c);
        if let Some(c) = record.get_character(3) {
            properties.set_thousands_separator(c);
        }

        // Java: c = record.getCharacter(4); if (c != null) properties.setDecimalSeparator(c);
        if let Some(c) = record.get_character(4) {
            properties.set_decimal_separator(c);
        }
    }

    /// Populate default settings.
    ///
    /// Corresponds to Java: `private void populateDefaultSettings(Record record, ProjectProperties properties) throws MPXJException`
    fn populate_default_settings(
        record: &Record<'_>,
        properties: &mut ProjectProperties,
    ) -> MpxjResult<()> {
        properties.set_default_duration_units(Some(record.get_time_unit(0)));
        properties.set_default_duration_is_fixed(record.get_numeric_boolean(1));
        properties.set_default_work_units(Some(record.get_time_unit(2)));

        // Java: properties.setMinutesPerDay(Integer.valueOf((int) (NumberHelper.getDouble(record.getFloat(3)) * 60)));
        let minutes_per_day = (NumberHelper::get_double(record.get_float(3)?) * 60.0) as i32;
        properties.set_minutes_per_day(Some(minutes_per_day));

        let minutes_per_week = (NumberHelper::get_double(record.get_float(4)?) * 60.0) as i32;
        properties.set_minutes_per_week(Some(minutes_per_week));

        properties.set_default_standard_rate(record.get_rate(5)?);
        properties.set_default_overtime_rate(record.get_rate(6)?);
        properties.set_updating_task_status_updates_resource_status(record.get_numeric_boolean(7));
        properties.set_split_in_progress_tasks(record.get_numeric_boolean(8));

        Ok(())
    }

    /// Populate date-time settings.
    ///
    /// Corresponds to Java: `private void populateDateTimeSettings(Record record, ProjectProperties properties)`
    fn populate_date_time_settings(record: &Record<'_>, properties: &mut ProjectProperties) {
        properties.set_date_order(Some(record.get_date_order(0)));
        properties.set_time_format(Some(record.get_time_format(1)));

        if let Some(time) = Self::get_time_from_integer(record.get_integer(2)) {
            properties.set_default_start_time(Some(time));
        }

        if let Some(c) = record.get_character(3) {
            properties.set_date_separator(c);
        }

        if let Some(c) = record.get_character(4) {
            properties.set_time_separator(c);
        }

        properties.set_am_text(record.get_string(5));
        properties.set_pm_text(record.get_string(6));
        properties.set_date_format(Some(record.get_date_format(7)));
        properties.set_bar_text_date_format(Some(record.get_date_format(8)));
    }

    /// Convert an integer time expressed in minutes into a [`NaiveTime`].
    ///
    /// Corresponds to Java: `private LocalTime getTimeFromInteger(Integer time)`
    ///
    /// Java's `LocalTime.ofSecondOfDay` normalizes seconds beyond one day modulo 86400; kept consistent here.
    fn get_time_from_integer(time: Option<i32>) -> Option<NaiveTime> {
        let time = time?;

        // Java: LocalTime.ofSecondOfDay(time.intValue() * 60L)
        let seconds = i64::from(time) * 60;
        let seconds_of_day = seconds.rem_euclid(86_400);

        NaiveTime::from_num_seconds_from_midnight_opt(seconds_of_day as u32, 0)
    }

    /// Populate the project header (project properties).
    ///
    /// Corresponds to Java: `private void populateProjectHeader(Record record, ProjectProperties properties) throws MPXJException`
    fn populate_project_header(
        &mut self,
        record: &Record<'_>,
        properties: &mut ProjectProperties,
    ) -> MpxjResult<()> {
        properties.set_project_title(record.get_string(0));
        properties.set_company(record.get_string(1));
        properties.set_manager(record.get_string(2));
        properties.set_start_date(record.get_date_time(4));
        properties.set_finish_date(record.get_date_time(5));
        properties.set_schedule_from(Some(record.get_schedule_from(6)));
        properties.set_current_date(record.get_date_time(7));
        properties.set_comments(record.get_string(8));
        properties.set_cost(record.get_currency(9)?);
        properties.set_baseline_cost(record.get_currency(10)?);
        properties.set_actual_cost(record.get_currency(11)?);
        properties.set_work(record.get_duration(12)?);
        properties.set_baseline_work(record.get_duration(13)?);
        properties.set_actual_work(record.get_duration(14)?);
        properties.set_work2(record.get_percentage(15)?);
        properties.set_duration(record.get_duration(16)?);
        properties.set_baseline_duration(record.get_duration(17)?);
        properties.set_actual_duration(record.get_duration(18)?);
        properties.set_percentage_complete(record.get_percentage(19)?);
        properties.set_baseline_start(record.get_date_time(20));
        properties.set_baseline_finish(record.get_date_time(21));
        properties.set_actual_start(record.get_date_time(22));
        properties.set_actual_finish(record.get_date_time(23));
        properties.set_start_variance(record.get_duration(24)?);
        properties.set_finish_variance(record.get_duration(25)?);
        properties.set_subject(record.get_string(26));
        properties.set_author(record.get_string(27));
        properties.set_keywords(record.get_string(28));

        // Java: m_defaultCalendarName = record.getString(3);
        self.m_default_calendar_name = record.get_string(3);

        Ok(())
    }

    /// Populate a calendar working hours (a weekday's working periods) instance.
    ///
    /// Corresponds to Java: `private void populateCalendarHours(Record record, ProjectCalendar calendar)`
    fn populate_calendar_hours(record: &Record<'_>, calendar: &Rc<RefCell<ProjectCalendar>>) {
        // Java: calendar.addCalendarHours(DayOfWeekHelper.getInstance(NumberHelper.getInt(record.getInteger(0))))
        // When Rust cannot resolve the weekday it skips this record (Java passes null to addCalendarHours).
        let day_value = NumberHelper::get_int(record.get_integer(0).map(f64::from));
        let Some(day) = DayOfWeekHelper::get_instance(day_value) else {
            return;
        };

        let mut calendar_ref = calendar.borrow_mut();
        let hours = calendar_ref.add_calendar_hours(day);

        Self::add_date_range(hours, record.get_time(1), record.get_time(2));
        Self::add_date_range(hours, record.get_time(3), record.get_time(4));
        Self::add_date_range(hours, record.get_time(5), record.get_time(6));
    }

    /// Add a time range to the working hours (only when both start and end are non-null).
    ///
    /// Corresponds to Java: `private void addDateRange(ProjectCalendarHours hours, LocalTime start, LocalTime end)`
    ///
    /// Java's comment mentions the "end is midnight" case (where the end should be treated as midnight of the
    /// next day); the time in an MPX record is `HH:mm` text and Java likewise just constructs a `LocalTimeRange`
    /// as-is, so the behavior matches.
    fn add_date_range(
        hours: &mut ProjectCalendarHours,
        start: Option<NaiveTime>,
        end: Option<NaiveTime>,
    ) {
        if let (Some(start), Some(end)) = (start, end) {
            hours.add(LocalTimeRange::new(Some(start), Some(end)));
        }
    }

    /// Populate a calendar exception (non-working day / special working day).
    ///
    /// Corresponds to Java: `private void populateCalendarException(Record record, ProjectCalendar calendar)`
    fn populate_calendar_exception(record: &Record<'_>, calendar: &Rc<RefCell<ProjectCalendar>>) {
        let from_date = record.get_date(0);
        let mut to_date = record.get_date(1);
        let working = record.get_numeric_boolean(2);

        // Java: some MPX files write only the start date for a single-day exception; the end date is then treated as the same as the start date.
        if from_date.is_some() && to_date.is_none() {
            to_date = from_date;
        }

        let mut calendar_ref = calendar.borrow_mut();

        // Java: calendar.addCalendarException(
        //           LocalDateHelper.getLocalDate(fromDate), LocalDateHelper.getLocalDate(toDate));
        let exception = calendar_ref.add_calendar_exception(
            LocalDateHelper::get_local_date(from_date),
            LocalDateHelper::get_local_date(to_date),
        );

        if working {
            Self::add_exception_range(exception, record.get_time(3), record.get_time(4));
            Self::add_exception_range(exception, record.get_time(5), record.get_time(6));
            Self::add_exception_range(exception, record.get_time(7), record.get_time(8));
        }
    }

    /// Add a time range to the exception, avoiding adding empty ranges.
    ///
    /// Corresponds to Java: `private void addExceptionRange(ProjectCalendarException exception, LocalTime start, LocalTime finish)`
    fn add_exception_range(
        exception: &mut ProjectCalendarException,
        start: Option<NaiveTime>,
        finish: Option<NaiveTime>,
    ) {
        if let (Some(start), Some(finish)) = (start, finish) {
            exception.add(LocalTimeRange::new(Some(start), Some(finish)));
        }
    }

    /// Populate a calendar instance.
    ///
    /// Corresponds to Java: `private void populateCalendar(Record record, ProjectCalendar calendar, boolean isBaseCalendar)`
    ///
    /// When `is_base_calendar == true` the calendar is a base calendar (field 0 is the calendar name);
    /// otherwise it is a resource calendar (field 0 is the parent (base) calendar name).
    fn populate_calendar(
        &mut self,
        record: &Record<'_>,
        calendar: &Rc<RefCell<ProjectCalendar>>,
        is_base_calendar: bool,
    ) {
        if is_base_calendar {
            // Java: calendar.setName(record.getString(0));
            calendar.borrow_mut().set_name(record.get_string(0));
        } else {
            // Java: calendar.setParent(m_projectFile.getCalendarByName(record.getString(0)));
            let parent = self
                .m_project_file
                .as_ref()
                .and_then(|file| match file.try_borrow() {
                    Ok(file) => file.get_calendar_by_name(record.get_string(0).as_deref()),
                    // Java has no such branch; Rust avoids a borrow conflict
                    Err(_) => None,
                });

            calendar.borrow_mut().set_parent(parent);
        }

        // Java sets fields 1..7 in DayOfWeek declaration order (Sunday → Saturday)
        {
            let mut calendar_ref = calendar.borrow_mut();
            calendar_ref.set_calendar_day_type(Weekday::Sun, record.get_day_type(1));
            calendar_ref.set_calendar_day_type(Weekday::Mon, record.get_day_type(2));
            calendar_ref.set_calendar_day_type(Weekday::Tue, record.get_day_type(3));
            calendar_ref.set_calendar_day_type(Weekday::Wed, record.get_day_type(4));
            calendar_ref.set_calendar_day_type(Weekday::Thu, record.get_day_type(5));
            calendar_ref.set_calendar_day_type(Weekday::Fri, record.get_day_type(6));
            calendar_ref.set_calendar_day_type(Weekday::Sat, record.get_day_type(7));
        }

        // Java: m_eventManager.fireCalendarReadEvent(calendar);
        self.with_event_manager(|manager| manager.fire_calendar_read_event(calendar));
    }

    /// Populate a resource.
    ///
    /// Corresponds to Java: `private void populateResource(Resource resource, Record record) throws MPXJException`
    fn populate_resource(
        &mut self,
        resource: &Rc<RefCell<Resource>>,
        record: &Record<'_>,
    ) -> MpxjResult<()> {
        let false_text = LocaleData::get_string(self.m_locale, LocaleData::NO);

        let length = record.get_length();
        // Java: int[] model = m_resourceModel.getModel(); (Rust copies it, see item 5 of the file-header differences)
        let model: Vec<i32> = match self.m_resource_model.as_mut() {
            Some(model) => model.get_model().to_vec(),
            None => Vec::new(),
        };

        // Java: Rate standardRate = Rate.ZERO; Rate overtimeRate = Rate.ZERO;
        //       Number costPerUse = NumberHelper.DOUBLE_ZERO;
        let mut standard_rate = Rate::ZERO;
        let mut overtime_rate = Rate::ZERO;
        let mut cost_per_use: Option<f64> = Some(NumberHelper::DOUBLE_ZERO);

        for i in 0..length {
            // Java: int mpxFieldType = model[i]; if (mpxFieldType == -1) break;
            // Terminates as well when the model is too short (Java throws ArrayIndexOutOfBoundsException)
            let mpx_field_type = match model.get(i) {
                Some(value) => *value,
                None => -1,
            };

            if mpx_field_type == -1 {
                break;
            }

            let index = i as i32;
            let field = record.get_string(index);

            let Some(field) = field else {
                continue;
            };

            if field.is_empty() {
                continue;
            }

            // Java: ResourceField resourceField = MPXResourceField.getMpxjField(mpxFieldType);
            // Java does not check for null (an unmapped field number falls into the default branch and NPEs);
            // Rust skips unmapped field numbers.
            let Some(resource_field) = MPXResourceField::get_mpxj_field(mpx_field_type) else {
                continue;
            };

            match resource_field {
                ResourceField::Objects => {
                    // Java: resource.set(resourceField, record.getInteger(i));
                    resource.borrow_mut().set(
                        FieldKey::Resource(resource_field),
                        record.get_integer(index).map(FieldValue::Integer),
                    );
                }

                ResourceField::Id => {
                    // Java: resource.setID(record.getInteger(i));
                    resource.borrow_mut().set_id(record.get_integer(index));
                }

                ResourceField::UniqueId => {
                    // Java: resource.setUniqueID(record.getInteger(i));
                    resource.borrow_mut().set_unique_id(record.get_integer(index));
                }

                ResourceField::MaxUnits => {
                    // Java: resource.getAvailability().add(
                    //           new Availability(START_DATE_NA, END_DATE_NA, record.getUnits(i)));
                    let units = record.get_units(index)?;
                    resource.borrow_mut().add_availability(Availability::new(
                        Some(LocalDateTimeHelper::start_date_na()),
                        Some(LocalDateTimeHelper::end_date_na()),
                        units,
                    ));
                }

                ResourceField::PercentWorkComplete | ResourceField::Peak => {
                    // Java: resource.set(resourceField, record.getPercentage(i));
                    resource.borrow_mut().set(
                        FieldKey::Resource(resource_field),
                        record.get_percentage(index)?.map(FieldValue::Double),
                    );
                }

                ResourceField::Cost
                | ResourceField::CostVariance
                | ResourceField::BaselineCost
                | ResourceField::ActualCost
                | ResourceField::RemainingCost => {
                    // Java: resource.set(resourceField, record.getCurrency(i));
                    resource.borrow_mut().set(
                        FieldKey::Resource(resource_field),
                        record.get_currency(index)?.map(FieldValue::Double),
                    );
                }

                ResourceField::CostPerUse => {
                    // Java: costPerUse = record.getCurrency(i);
                    cost_per_use = record.get_currency(index)?;
                }

                ResourceField::OvertimeRate => {
                    // Java: overtimeRate = record.getRate(i);
                    overtime_rate = record.get_rate(index)?.unwrap_or(Rate::ZERO);
                }

                ResourceField::StandardRate => {
                    // Java: standardRate = record.getRate(i);
                    standard_rate = record.get_rate(index)?.unwrap_or(Rate::ZERO);
                }

                ResourceField::RemainingWork
                | ResourceField::OvertimeWork
                | ResourceField::BaselineWork
                | ResourceField::ActualWork
                | ResourceField::Work
                | ResourceField::WorkVariance => {
                    // Java: resource.set(resourceField, record.getDuration(i));
                    resource.borrow_mut().set(
                        FieldKey::Resource(resource_field),
                        record.get_duration(index)?.map(FieldValue::Duration),
                    );
                }

                ResourceField::AccrueAt => {
                    // Java: resource.set(resourceField, record.getAccrueType(i));
                    // Enums are encoded into the field table via getValue() (see item 4 of the file-header differences).
                    resource.borrow_mut().set(
                        FieldKey::Resource(resource_field),
                        record
                            .get_accrue_type(index)
                            .map(|value| FieldValue::Integer(value.value())),
                    );
                }

                ResourceField::LinkedFields | ResourceField::Overallocated => {
                    // Java: resource.set(resourceField, record.getBoolean(i, falseText));
                    resource.borrow_mut().set(
                        FieldKey::Resource(resource_field),
                        record
                            .get_boolean(index, false_text)
                            .map(FieldValue::Boolean),
                    );
                }

                _ => {
                    // Java: resource.set(resourceField, field);
                    resource.borrow_mut().set(
                        FieldKey::Resource(resource_field),
                        Some(FieldValue::String(field)),
                    );
                }
            }
        }

        // Java: if (m_projectConfig.getAutoResourceUniqueID())
        //          resource.setUniqueID(m_projectFile.getUniqueIdObjectSequence(Resource.class).getNext());
        if self.project_config_flag(|config| config.get_auto_resource_unique_id()) {
            let file = self.file()?;
            let next = file
                .borrow_mut()
                .get_unique_id_object_sequence(TypeId::of::<Resource>())
                .get_next();
            resource.borrow_mut().set_unique_id(Some(next));
        }

        // Java: if (m_projectConfig.getAutoResourceID())
        //          resource.setID(m_projectFile.getResources().getNextID());
        if self.project_config_flag(|config| config.get_auto_resource_id()) {
            let file = self.file()?;
            let next_id = file.borrow().get_resources().get_next_id();
            resource.borrow_mut().set_id(Some(next_id));
        }

        // Java:
        // CostRateTable table = new CostRateTable();
        // table.add(new CostRateTableEntry(LocalDateTimeHelper.START_DATE_NA, LocalDateTimeHelper.END_DATE_NA,
        //                                  costPerUse, standardRate, overtimeRate));
        // resource.setCostRateTable(0, table);
        let mut table = CostRateTable::new();
        table.add(CostRateTableEntry::new(
            Some(LocalDateTimeHelper::start_date_na()),
            Some(LocalDateTimeHelper::end_date_na()),
            cost_per_use,
            &[standard_rate, overtime_rate],
        ));
        resource.borrow_mut().set_cost_rate_table(0, table);

        // Java: handle malformed MPX files — ensure the unique ID is non-null.
        // if (resource.getUniqueID() == null) resource.setUniqueID(resource.getID());
        if resource.borrow().get_unique_id().is_none() {
            let id = resource.borrow().get_id();
            resource.borrow_mut().set_unique_id(id);
        }

        Ok(())
    }

    /// Collect one relationship into the deferred-processing list.
    ///
    /// Corresponds to Java: `private void populateRelationList(Task task, TaskField field, String data)`
    fn populate_relation_list(&mut self, task: &Rc<RefCell<Task>>, field: TaskField, data: String) {
        let mut deferred = DeferredRelationship::new();
        deferred.set_task(Rc::clone(task));
        deferred.set_field(field);
        deferred.set_data(data);

        self.m_deferred_relationships.push(deferred);
    }

    /// Iterate over all deferred relationships and process them one by one.
    ///
    /// Corresponds to Java: `private void processDeferredRelationships() throws MPXJException`
    ///
    /// `formats` is a Rust-specific parameter (see item 1 of the file-header differences): Java's
    /// `populateRelation` reads the member `m_formats` directly.
    fn process_deferred_relationships(&mut self, formats: &MPXJFormats) -> MpxjResult<()> {
        // Java iterates the member list; Rust takes the list out first and then processes it, avoiding a borrow
        // conflict between "iterating a &self field" and "processing with &mut self".
        let deferred = std::mem::take(&mut self.m_deferred_relationships);

        for relationship in &deferred {
            self.process_deferred_relationship(relationship, formats)?;
        }

        Ok(())
    }

    /// Process one deferred relationship (its data may contain multiple relationships).
    ///
    /// Corresponds to Java: `private void processDeferredRelationship(DeferredRelationship dr) throws MPXJException`
    fn process_deferred_relationship(
        &self,
        dr: &DeferredRelationship,
        formats: &MPXJFormats,
    ) -> MpxjResult<()> {
        let Some(data) = dr.get_data().map(|data| data.trim().to_owned()) else {
            return Ok(());
        };

        let Some(task) = dr.get_task() else {
            return Ok(());
        };

        let Some(field) = dr.get_field() else {
            return Ok(());
        };

        let length = data.len();

        if length != 0 {
            let mut start = 0usize;
            let mut end = 0usize;

            while end != length {
                // Java: end = data.indexOf(m_delimiter, start); if (end == -1) { end = length; }
                end = match data[start..].find(self.m_delimiter) {
                    Some(offset) => start + offset,
                    None => length,
                };

                if start != end {
                    // Java: populateRelation(dr.getField(), task, data.substring(start, end).trim());
                    self.populate_relation(field, &task, data[start..end].trim(), formats)?;
                }

                start = end + self.m_delimiter.len_utf8();
            }
        }

        Ok(())
    }

    /// Create and populate a new task relationship.
    ///
    /// Corresponds to Java: `private void populateRelation(TaskField field, Task sourceTask, String relationship) throws MPXJException`
    ///
    /// `formats` is a Rust-specific parameter (see item 1 of the file-header differences): Java reads the
    /// member `m_formats` directly to get the "duration decimal format" for parsing lag time.
    fn populate_relation(
        &self,
        field: TaskField,
        source_task: &Rc<RefCell<Task>>,
        relationship: &str,
        formats: &MPXJFormats,
    ) -> MpxjResult<()> {
        let length = relationship.len();

        //
        // Java: take the leading run of digits as the task ID
        //
        // int index = 0;
        // while ((index < length) && Character.isDigit(relationship.charAt(index))) { ++index; }
        //
        // Java's `Character.isDigit` accepts non-ASCII digits; Rust handles ASCII digits (`Integer.valueOf` /
        // `str::parse` reject non-ASCII digits in both, so the results match).
        let mut index = 0usize;
        for (offset, c) in relationship.char_indices() {
            if c.is_ascii_digit() {
                index = offset + c.len_utf8();
            } else {
                break;
            }
        }

        // Java: try { taskID = Integer.valueOf(relationship.substring(0, index)); }
        //       catch (NumberFormatException ex) { throw new MPXJException(INVALID_FORMAT + " '" + relationship + "'"); }
        let task_id = relationship[..index].parse::<i32>().map_err(|_| {
            MpxjError::new(format!(
                "{} '{}'",
                MpxjError::INVALID_FORMAT,
                relationship
            ))
        })?;

        //
        // Java: choose whether to look up the target task by ID or by unique ID based on the field
        //
        let target_task = match self.m_project_file.as_ref() {
            Some(file) => match file.try_borrow() {
                Ok(file) => {
                    if field == TaskField::Predecessors {
                        file.get_task_by_id(Some(task_id))
                    } else {
                        file.get_task_by_unique_id(Some(task_id))
                    }
                }
                // Java has no such branch; Rust avoids a borrow conflict
                Err(_) => None,
            },
            None => None,
        };

        //
        // Java: if not yet at the end, the next part should be SF / SS / FS / FF
        //
        let (relation_type, lag) = if index == length {
            // Java: type = RelationType.FINISH_START; lag = Duration.getInstance(0, TimeUnit.DAYS);
            (Some(crate::RelationType::FinishStart), Some(Duration::get_instance(0.0, TimeUnit::Days)))
        } else {
            // Java: if ((index + 1) == length) { throw new MPXJException(INVALID_FORMAT ...); }
            // Rust also treats "fewer than two characters" as an invalid format.
            let mut chars = relationship[index..].chars();
            match (chars.next(), chars.next()) {
                (Some(first), Some(second)) => {
                    let type_text: String = [first, second].into_iter().collect();
                    index += first.len_utf8() + second.len_utf8();

                    // Java: type = RelationTypeUtility.getInstance(m_locale, relationship.substring(index, index + 2));
                    let relation_type =
                        RelationTypeUtility::get_instance(self.m_locale, Some(type_text.as_str()));

                    if index == length {
                        // Java: lag = Duration.getInstance(0, TimeUnit.DAYS);
                        (
                            relation_type,
                            Some(Duration::get_instance(0.0, TimeUnit::Days)),
                        )
                    } else {
                        // Java: if (relationship.charAt(index) == '+') { ++index; }
                        if relationship[index..].starts_with('+') {
                            index += '+'.len_utf8();
                        }

                        // Java: lag = DurationUtility.getInstance(
                        //           relationship.substring(index), m_formats.getDurationDecimalFormat(), m_locale);
                        let lag = DurationUtility::get_instance(
                            relationship.get(index..),
                            formats.get_duration_decimal_format(),
                            self.m_locale,
                        );

                        (relation_type, lag)
                    }
                }
                _ => {
                    return Err(MpxjError::new(format!(
                        "{} '{}'",
                        MpxjError::INVALID_FORMAT,
                        relationship
                    )))
                }
            }
        };

        // Java: if (type == null) { throw new MPXJException(INVALID_FORMAT + " '" + relationship + "'"); }
        let Some(relation_type) = relation_type else {
            return Err(MpxjError::new(format!(
                "{} '{}'",
                MpxjError::INVALID_FORMAT,
                relationship
            )));
        };

        // Java: some MPX files contain an invalid task ID; the relationship is then ignored (the rest of the schedule remains valid).
        if let Some(target_task) = target_task {
            let builder = RelationBuilder::new()
                .predecessor_task(&target_task)
                .r#type(relation_type)
                .lag(lag);

            if let Some(relation) = Task::add_predecessor(source_task, builder) {
                // Java: m_eventManager.fireRelationReadEvent(relation);
                self.with_event_manager(|manager| manager.fire_relation_read_event(&relation));
            }
        }

        Ok(())
    }

    /// Populate a task instance.
    ///
    /// Corresponds to Java: `private void populateTask(Record record, Task task) throws MPXJException`
    fn populate_task(&mut self, record: &Record<'_>, task: &Rc<RefCell<Task>>) -> MpxjResult<()> {
        let false_text = LocaleData::get_string(self.m_locale, LocaleData::NO);

        let mut i = 0usize;
        let length = record.get_length();
        // Java: int[] model = m_taskModel.getModel(); (Rust copies it, see item 5 of the file-header differences)
        let model: Vec<i32> = match self.m_task_model.as_mut() {
            Some(model) => model.get_model().to_vec(),
            None => Vec::new(),
        };

        while i < length {
            // Java throws ArrayIndexOutOfBoundsException when the model is too short; Rust terminates the loop
            let mpx_field_id = match model.get(i) {
                Some(value) => *value,
                None => -1,
            };

            if mpx_field_id == -1 {
                break;
            }

            // Java: field = record.getString(i++);
            let index = i as i32;
            let field = record.get_string(index);
            i += 1;

            let Some(field) = field else {
                continue;
            };

            if field.is_empty() {
                continue;
            }

            // Java: TaskField taskField = MPXTaskField.getMpxjField(mpxFieldID);
            //       if (taskField == null) { ignore unknown fields }
            let Some(task_field) = MPXTaskField::get_mpxj_field(mpx_field_id) else {
                continue;
            };

            match task_field {
                TaskField::Notes => {
                    // Java: although a notes property may appear, MS Project ignores it, and it is likewise ignored here.
                }

                TaskField::Summary => {
                    // Java: ignore the summary property in the file, defaulting to false; it is set again later during updateStructure.
                    task.borrow_mut().set_summary(false);
                }

                TaskField::Successors => {
                    // Java: MPX files usually write only predecessor relations, so successor properties are ignored.
                }

                TaskField::Predecessors | TaskField::UniqueIdPredecessors => {
                    // Java: populateRelationList(task, taskField, field);
                    self.populate_relation_list(task, task_field, field);
                }

                TaskField::PercentComplete | TaskField::PercentWorkComplete => {
                    // Java: task.set(taskField, m_formats.getPercentageDecimalFormat().parse(field));
                    //       catch (ParseException ex) { throw new MPXJException("Failed to parse percentage", ex); }
                    // Uses the same format object as Record::get_percentage; error messages match Java.
                    let value = record.get_percentage(index)?;
                    task.borrow_mut().set(
                        FieldKey::Task(task_field),
                        value.map(FieldValue::Double),
                    );
                }

                TaskField::ActualCost
                | TaskField::BaselineCost
                | TaskField::Bcwp
                | TaskField::Bcws
                | TaskField::Cost
                | TaskField::Cost1
                | TaskField::Cost2
                | TaskField::Cost3
                | TaskField::CostVariance
                | TaskField::Cv
                | TaskField::FixedCost
                | TaskField::RemainingCost
                | TaskField::Sv => {
                    // Java: task.set(taskField, m_formats.getCurrencyFormat().parse(field));
                    let value = record.get_currency(index)?;
                    task.borrow_mut().set(
                        FieldKey::Task(task_field),
                        value.map(FieldValue::Double),
                    );
                }

                TaskField::ActualDuration
                | TaskField::ActualWork
                | TaskField::BaselineDuration
                | TaskField::BaselineWork
                | TaskField::Duration
                | TaskField::Duration1
                | TaskField::Duration2
                | TaskField::Duration3
                | TaskField::DurationVariance
                | TaskField::FinishVariance
                | TaskField::FreeSlack
                | TaskField::RemainingDuration
                | TaskField::RemainingWork
                | TaskField::StartVariance
                | TaskField::TotalSlack
                | TaskField::Work
                | TaskField::WorkVariance
                | TaskField::LevelingDelay => {
                    // Java: task.set(taskField, DurationUtility.getInstance(
                    //           field, m_formats.getDurationDecimalFormat(), m_locale));
                    // Rust-specific: the format collection is retrieved via [`Record::get_formats`] (see item 1 of the file-header differences).
                    let value = DurationUtility::get_instance(
                        Some(&field),
                        record.get_formats().get_duration_decimal_format(),
                        self.m_locale,
                    );
                    task.borrow_mut().set(
                        FieldKey::Task(task_field),
                        value.map(FieldValue::Duration),
                    );
                }

                TaskField::ActualFinish
                | TaskField::ActualStart
                | TaskField::BaselineFinish
                | TaskField::BaselineStart
                | TaskField::ConstraintDate
                | TaskField::Created
                | TaskField::EarlyFinish
                | TaskField::EarlyStart
                | TaskField::Finish
                | TaskField::Finish1
                | TaskField::Finish2
                | TaskField::Finish3
                | TaskField::Finish4
                | TaskField::Finish5
                | TaskField::LateFinish
                | TaskField::LateStart
                | TaskField::Resume
                | TaskField::Start
                | TaskField::Start1
                | TaskField::Start2
                | TaskField::Start3
                | TaskField::Start4
                | TaskField::Start5
                | TaskField::Stop => {
                    // Java: task.set(taskField, m_formats.parseDateTime(field));
                    let value = record.get_date_time(index);
                    task.borrow_mut().set(
                        FieldKey::Task(task_field),
                        value.map(FieldValue::DateTime),
                    );
                }

                TaskField::Confirmed
                | TaskField::Critical
                | TaskField::Flag1
                | TaskField::Flag2
                | TaskField::Flag3
                | TaskField::Flag4
                | TaskField::Flag5
                | TaskField::Flag6
                | TaskField::Flag7
                | TaskField::Flag8
                | TaskField::Flag9
                | TaskField::Flag10
                | TaskField::HideBar
                | TaskField::LinkedFields
                | TaskField::Marked
                | TaskField::Milestone
                | TaskField::Rollup
                | TaskField::UpdateNeeded => {
                    // Java: task.set(taskField, (field.equalsIgnoreCase(falseText) ? Boolean.FALSE : Boolean.TRUE));
                    let value = !equals_ignore_case(&field, false_text);
                    task.borrow_mut().set(
                        FieldKey::Task(task_field),
                        Some(FieldValue::Boolean(value)),
                    );
                }

                TaskField::ConstraintType => {
                    // Java: task.set(taskField, ConstraintTypeUtility.getInstance(m_locale, field));
                    // Enums are encoded into the field table via getValue() (see item 4 of the file-header differences).
                    let value = ConstraintTypeUtility::get_instance(self.m_locale, Some(&field));
                    task.borrow_mut().set(
                        FieldKey::Task(task_field),
                        Some(FieldValue::Integer(value.value())),
                    );
                }

                TaskField::Objects | TaskField::OutlineLevel => {
                    // Java: task.set(taskField, Integer.valueOf(field));
                    let value = field.parse::<i32>().map_err(|_| {
                        MpxjError::new(format!("For input string: \"{field}\""))
                    })?;
                    task.borrow_mut()
                        .set(FieldKey::Task(task_field), Some(FieldValue::Integer(value)));
                }

                TaskField::Id => {
                    // Java: task.setID(Integer.valueOf(field));
                    let value = field.parse::<i32>().map_err(|_| {
                        MpxjError::new(format!("For input string: \"{field}\""))
                    })?;
                    task.borrow_mut().set_id(Some(value));
                }

                TaskField::UniqueId => {
                    // Java: task.setUniqueID(Integer.valueOf(field));
                    let value = field.parse::<i32>().map_err(|_| {
                        MpxjError::new(format!("For input string: \"{field}\""))
                    })?;
                    task.borrow_mut().set_unique_id(Some(value));
                }

                TaskField::Number1
                | TaskField::Number2
                | TaskField::Number3
                | TaskField::Number4
                | TaskField::Number5 => {
                    // Java: task.set(taskField, m_formats.getDecimalFormat().parse(field));
                    //       catch (ParseException ex) { throw new MPXJException("Failed to parse number", ex); }
                    // Uses the same format object as Record::get_float; Java's exception message is kept here.
                    let value = record
                        .get_float(index)
                        .map_err(|_| MpxjError::new("Failed to parse number"))?;
                    task.borrow_mut().set(
                        FieldKey::Task(task_field),
                        value.map(FieldValue::Double),
                    );
                }

                TaskField::Priority => {
                    // Java: task.set(taskField, PriorityUtility.getInstance(m_locale, field));
                    let value = PriorityUtility::get_instance(self.m_locale, Some(&field));
                    task.borrow_mut().set(
                        FieldKey::Task(task_field),
                        Some(FieldValue::Integer(value.get_value())),
                    );
                }

                TaskField::Type => {
                    // Java: boolean fixed = !field.equalsIgnoreCase(falseText);
                    //       task.setType(fixed ? TaskType.FIXED_DURATION : TaskType.FIXED_UNITS);
                    let fixed = !equals_ignore_case(&field, false_text);
                    task.borrow_mut().set_type(Some(if fixed {
                        TaskType::FixedDuration
                    } else {
                        TaskType::FixedUnits
                    }));
                }

                _ => {
                    // Java: task.set(taskField, field);
                    task.borrow_mut()
                        .set(FieldKey::Task(task_field), Some(FieldValue::String(field)));
                }
            }
        }

        // Java: if (m_projectConfig.getAutoWBS()) { task.generateWBS(null); }
        if self.project_config_flag(|config| config.get_auto_wbs()) {
            task.borrow_mut().generate_wbs(None);
        }

        // Java: if (m_projectConfig.getAutoOutlineNumber()) { task.generateOutlineNumber(null); }
        if self.project_config_flag(|config| config.get_auto_outline_number()) {
            task.borrow_mut().generate_outline_number(None);
        }

        // Java: if (m_projectConfig.getAutoOutlineLevel()) { task.setOutlineLevel(Integer.valueOf(1)); }
        if self.project_config_flag(|config| config.get_auto_outline_level()) {
            task.borrow_mut().set_outline_level(Some(1));
        }

        // Java: if (m_projectConfig.getAutoTaskUniqueID())
        //          task.setUniqueID(m_projectFile.getUniqueIdObjectSequence(Task.class).getNext());
        if self.project_config_flag(|config| config.get_auto_task_unique_id()) {
            let file = self.file()?;
            let next = file
                .borrow_mut()
                .get_unique_id_object_sequence(TypeId::of::<Task>())
                .get_next();
            task.borrow_mut().set_unique_id(Some(next));
        }

        // Java: if (task.getID() == null || m_projectConfig.getAutoTaskID())
        //          task.setID(m_projectFile.getTasks().getNextID());
        if task.borrow().get_id().is_none() || self.project_config_flag(|config| config.get_auto_task_id())
        {
            let file = self.file()?;
            let next_id = file.borrow().get_tasks().get_next_id();
            task.borrow_mut().set_id(Some(next_id));
        }

        // Java: handle malformed MPX files — ensure the unique ID is non-null.
        // if (task.getUniqueID() == null) task.setUniqueID(task.getID());
        if task.borrow().get_unique_id().is_none() {
            let id = task.borrow().get_id();
            task.borrow_mut().set_unique_id(id);
        }

        //
        // Java: some applications (such as SureTrak) do not write start / finish; when MS Project opens such files it
        // fills them in with early start / early finish (if present).
        //
        if task.borrow().get_start().is_none() {
            if let Some(early_start) = task.borrow().get_early_start() {
                task.borrow_mut().set_start(Some(early_start));
            }
        }

        if task.borrow().get_finish().is_none() {
            if let Some(early_finish) = task.borrow().get_early_finish() {
                task.borrow_mut().set_finish(Some(early_finish));
            }
        }

        //
        // Java: a task whose duration is not 0 is not a milestone even if the milestone flag is true.
        //
        let milestone_with_duration = task.borrow().get_milestone()
            && task
                .borrow()
                .get_duration()
                .map_or(false, |duration| duration.get_duration() != 0.0);
        if milestone_with_duration {
            task.borrow_mut().set_milestone(false);
        }

        //
        // Java: only total slack is written in the file; this assumes it is correct and back-fills start / finish variance from it.
        //
        SlackHelper::infer_slack(&mut task.borrow_mut());

        //
        // Java has no extra action: `Task.setID` itself registers the ID in the container's "ID → task" map.
        // Rust's [`crate::Task::set_id`] now likewise calls back into `TaskContainer::map_id` /
        // `unmap_id` (see that method's docs and item 15 of the file-header differences), so no compensation is needed here.
        //

        Ok(())
    }

    /// Populate a recurring task record (`RECURRING_TASK_RECORD_NUMBER`).
    ///
    /// Corresponds to Java: `private void populateRecurringTask(Record record, RecurringTask task)`
    fn populate_recurring_task(
        &self,
        record: &Record<'_>,
        task: &Rc<RefCell<Task>>,
    ) -> MpxjResult<()> {
        // Java: RecurrenceUtility.getDuration(m_projectFile.getProjectProperties(), ...)
        let file = self.file()?;

        // This repo's convention: shared objects use `try_borrow_mut` (Java has no such branch; on a borrow conflict it skips this record without panicking).
        let Ok(mut task_ref) = task.try_borrow_mut() else {
            return Ok(());
        };

        // Java: RecurringTask task = m_lastTask.addRecurringTask();
        let recurring = task_ref.add_recurring_task();

        // Java: task.setStartDate(LocalDateHelper.getLocalDate(record.getDateTime(1)));
        recurring.set_start_date(LocalDateHelper::get_local_date(record.get_date_time(1)));
        // Java: task.setFinishDate(LocalDateHelper.getLocalDate(record.getDateTime(2)));
        recurring.set_finish_date(LocalDateHelper::get_local_date(record.get_date_time(2)));

        // Java: task.setDuration(RecurrenceUtility.getDuration(
        //           m_projectFile.getProjectProperties(), record.getInteger(3), record.getInteger(4)));
        {
            let Ok(file_ref) = file.try_borrow() else {
                return Ok(());
            };
            recurring.set_duration(RecurrenceUtility::get_duration(
                file_ref.get_project_properties(),
                record.get_integer(3),
                record.get_integer(4),
            ));
        }

        // Java: task.setOccurrences(record.getInteger(5));
        recurring.set_occurrences(record.get_integer(5));
        // Java: task.setRecurrenceType(RecurrenceUtility.getRecurrenceType(record.getInteger(6)));
        recurring.set_recurrence_type(RecurrenceUtility::get_recurrence_type(record.get_integer(6)));

        // Java: task.setUseEndDate(NumberHelper.getInt(record.getInteger(8)) == 1);
        //       task.setWorkingDaysOnly(NumberHelper.getInt(record.getInteger(9)) == 1);
        // This repo's `NumberHelper` only has `get_int(Option<f64>)`; for an integer argument `getInt(x) == 1`
        // is equivalent to `x == Some(1)` (null / other values are all false).
        recurring.set_use_end_date(record.get_integer(8) == Some(1));
        recurring.set_working_days_only(record.get_integer(9) == Some(1));

        // Java: task.setWeeklyDaysFromBitmap(
        //           RecurrenceUtility.getDays(record.getString(10)), RecurrenceUtility.RECURRING_TASK_DAY_MASKS);
        recurring.set_weekly_days_from_bitmap(
            RecurrenceUtility::get_days_from_string(record.get_string(10).as_deref()),
            &RecurrenceUtility::RECURRING_TASK_DAY_MASKS,
        );

        //
        // Java: RecurrenceType type = task.getRecurrenceType();
        //       if (type != null) { switch (task.getRecurrenceType()) { ... } }
        //
        match recurring.get_recurrence_type() {
            Some(RecurrenceType::Daily) => {
                // Java: task.setFrequency(record.getInteger(13));
                recurring.set_frequency(record.get_integer(13));
            }

            Some(RecurrenceType::Weekly) => {
                // Java: task.setFrequency(record.getInteger(14));
                recurring.set_frequency(record.get_integer(14));
            }

            Some(RecurrenceType::Monthly) => {
                // Java: task.setRelative(NumberHelper.getInt(record.getInteger(11)) == 1);
                recurring.set_relative(record.get_integer(11) == Some(1));

                if recurring.get_relative() {
                    // Java: task.setFrequency(record.getInteger(17));
                    //       task.setDayNumber(record.getInteger(15));
                    //       task.setDayOfWeek(RecurrenceUtility.getDay(record.getInteger(16)));
                    recurring.set_frequency(record.get_integer(17));
                    recurring.set_day_number(record.get_integer(15));
                    // This repo's `RecurringData::set_day_of_week` takes a non-null `Weekday`
                    // (Java's `DayOfWeek` may be null); it is skipped when unavailable.
                    if let Some(day) = RecurrenceUtility::get_day(record.get_integer(16)) {
                        recurring.set_day_of_week(day);
                    }
                } else {
                    // Java: task.setFrequency(record.getInteger(19));
                    //       task.setDayNumber(record.getInteger(18));
                    recurring.set_frequency(record.get_integer(19));
                    recurring.set_day_number(record.get_integer(18));
                }
            }

            Some(RecurrenceType::Yearly) => {
                // Java: task.setRelative(NumberHelper.getInt(record.getInteger(12)) != 1);
                recurring.set_relative(record.get_integer(12) != Some(1));

                if recurring.get_relative() {
                    // Java: task.setDayNumber(record.getInteger(20));
                    //       task.setDayOfWeek(RecurrenceUtility.getDay(record.getInteger(21)));
                    //       task.setMonthNumber(record.getInteger(22));
                    recurring.set_day_number(record.get_integer(20));
                    if let Some(day) = RecurrenceUtility::get_day(record.get_integer(21)) {
                        recurring.set_day_of_week(day);
                    }
                    recurring.set_month_number(record.get_integer(22));
                } else {
                    // Java: task.setYearlyAbsoluteFromDate(
                    //           LocalDateHelper.getLocalDate(record.getDateTime(23)));
                    recurring.set_yearly_absolute_from_date(LocalDateHelper::get_local_date(
                        record.get_date_time(23),
                    ));
                }
            }

            // Java: does not enter the switch when type == null
            None => {}
        }

        Ok(())
    }

    /// Get the resource corresponding to a resource assignment record.
    ///
    /// Corresponds to Java: `private Resource getResourceForAssignment(Record record)`
    ///
    /// Handle malformed MPX files: look up by unique ID first, then by ID if not found.
    fn get_resource_for_assignment(&self, record: &Record<'_>) -> Option<Rc<RefCell<Resource>>> {
        let file = self.m_project_file.as_ref()?;
        let file_ref = file.try_borrow().ok()?;

        // Java: Resource resource = m_projectFile.getResourceByUniqueID(record.getInteger(12));
        let resource = file_ref.get_resource_by_unique_id(record.get_integer(12));

        if resource.is_some() {
            return resource;
        }

        // Java: if (resource == null) { resource = m_projectFile.getResourceByID(record.getInteger(0)); }
        file_ref.get_resource_by_id(record.get_integer(0))
    }

    /// Populate a resource assignment.
    ///
    /// Corresponds to Java: `private void populateResourceAssignment(Record record, ResourceAssignment assignment) throws MPXJException`
    fn populate_resource_assignment(
        &self,
        record: &Record<'_>,
        assignment: &Rc<RefCell<ResourceAssignment>>,
    ) -> MpxjResult<()> {
        {
            let mut assignment_ref = assignment.borrow_mut();
            assignment_ref.set_units(record.get_units(1)?);
            assignment_ref.set_work(record.get_duration(2)?);
            assignment_ref.set_baseline_work(record.get_duration(3)?);
            assignment_ref.set_actual_work(record.get_duration(4)?);
            assignment_ref.set_overtime_work(record.get_duration(5)?);
            assignment_ref.set_cost(record.get_currency(6)?);
            assignment_ref.set_baseline_cost(record.get_currency(7)?);
            assignment_ref.set_actual_cost(record.get_currency(8)?);
            assignment_ref.set_start(record.get_date_time(9));
            assignment_ref.set_finish(record.get_date_time(10));
            assignment_ref.set_delay(record.get_duration(11)?);
        }

        //
        // Java: compute remaining work
        //
        // Duration work = assignment.getWork();
        // Duration actualWork = assignment.getActualWork();
        // if (work != null && actualWork != null) {
        //    if (work.getUnits() != actualWork.getUnits()) {
        //       actualWork = actualWork.convertUnits(work.getUnits(), m_projectFile.getProjectProperties());
        //    }
        //    assignment.setRemainingWork(Duration.getInstance(work.getDuration() - actualWork.getDuration(), work.getUnits()));
        // }
        let (work, actual_work) = {
            let assignment_ref = assignment.borrow();
            (assignment_ref.get_work(), assignment_ref.get_actual_work())
        };

        if let (Some(work), Some(actual_work)) = (work, actual_work) {
            let file = self.file()?;

            let actual_work = if work.get_units() != actual_work.get_units() {
                let file_ref = file.borrow();
                actual_work.convert_units(work.get_units(), file_ref.get_project_properties())
            } else {
                actual_work
            };

            assignment.borrow_mut().set_remaining_work(Some(Duration::get_instance(
                work.get_duration() - actual_work.get_duration(),
                work.get_units(),
            )));
        }

        // Java: m_eventManager.fireAssignmentReadEvent(assignment);
        self.with_event_manager(|manager| manager.fire_assignment_read_event(assignment));

        Ok(())
    }

    /// Populate a resource assignment workgroup fields instance (`RESOURCE_ASSIGNMENT_WORKGROUP_FIELDS_RECORD_NUMBER`).
    ///
    /// Corresponds to Java: `private void populateResourceAssignmentWorkgroupFields(Record record, ResourceAssignmentWorkgroupFields workgroup)`
    ///
    /// Trade-off: this method is `private` in Java; Rust makes it `pub` so unit tests can directly cover the per-field logic.
    pub fn populate_resource_assignment_workgroup_fields(
        record: &Record<'_>,
        workgroup: &mut ResourceAssignmentWorkgroupFields,
    ) {
        // Java: workgroup.setMessageUniqueID(record.getString(0));
        workgroup.set_message_unique_id(record.get_string(0));

        // Java: workgroup.setConfirmed(NumberHelper.getInt(record.getInteger(1)) == 1);
        //       workgroup.setResponsePending(NumberHelper.getInt(record.getInteger(1)) == 1);
        // Note: both lines use the same field index (both 1) — copied verbatim from the Java source, not "corrected".
        // For an integer argument `getInt(x) == 1` is equivalent to `x == Some(1)` (see `populate_recurring_task`).
        workgroup.set_confirmed(record.get_integer(1) == Some(1));
        workgroup.set_response_pending(record.get_integer(1) == Some(1));

        // Java: workgroup.setUpdateStart(record.getDateTime(3));
        workgroup.set_update_start(record.get_date_time(3));
        // Java: workgroup.setUpdateFinish(record.getDateTime(4));
        workgroup.set_update_finish(record.get_date_time(4));
        // Java: workgroup.setScheduleID(record.getString(5));
        workgroup.set_schedule_id(record.get_string(5));
    }

    /// Validate the project's calendars and fix problems.
    ///
    /// Corresponds to Java: `private void validateCalendars()`
    fn validate_calendars(&self) -> MpxjResult<()> {
        let file = self.file()?;

        //
        // Java: ensure a default calendar exists
        //
        // ProjectCalendarContainer calendars = m_projectFile.getCalendars();
        // ProjectCalendar defaultCalendar = calendars.getByName(m_defaultCalendarName);
        // if (defaultCalendar == null) { defaultCalendar = calendars.findOrCreateDefaultCalendar(); }
        // m_projectFile.getProjectProperties().setDefaultCalendar(defaultCalendar);
        let default_calendar = {
            let found = file
                .borrow()
                .get_calendars()
                .get_by_name(self.m_default_calendar_name.as_deref());

            match found {
                Some(calendar) => Some(calendar),
                None => file
                    .borrow()
                    .get_project_context()
                    .borrow_mut()
                    .find_or_create_default_calendar(),
            }
        };

        if let Some(default_calendar) = default_calendar {
            file.borrow_mut().set_default_calendar(Some(Rc::clone(&default_calendar)));
        }

        //
        // Java: calendars.forEach(this::validateCalendar);
        //
        // Rust takes a calendar snapshot first and then validates one by one (`for_each` modifies the calendars, requiring a separate mutable borrow).
        let calendars: Vec<Rc<RefCell<ProjectCalendar>>> =
            file.borrow().get_calendars().iter().cloned().collect();

        for calendar in &calendars {
            self.validate_calendar(calendar);
        }

        //
        // Java: post-processing of resource calendars
        //
        // Map<Integer, List<Resource>> resourceCalendarMap = m_projectFile.getResources().stream()
        //     .filter(r -> r.getCalendarUniqueID() != null)
        //     .collect(Collectors.groupingBy(Resource::getCalendarUniqueID));
        let resources: Vec<Rc<RefCell<Resource>>> =
            file.borrow().get_resources().iter().cloned().collect();

        // The key may be null (Java's Integer key), so Option<i32> is used.
        let mut resource_calendar_map: std::collections::HashMap<Option<i32>, usize> =
            std::collections::HashMap::new();

        for resource in &resources {
            if let Ok(resource_ref) = resource.try_borrow() {
                if let Some(unique_id) = resource_ref.get_calendar_unique_id() {
                    *resource_calendar_map.entry(Some(unique_id)).or_insert(0) += 1;
                }
            }
        }

        for resource in &resources {
            // Java: ProjectCalendar calendar = resource.getCalendar(); if (calendar != null) { ... }
            let calendar = match resource.try_borrow() {
                Ok(resource_ref) => resource_ref.get_calendar(),
                Err(_) => None,
            };

            let Some(calendar) = calendar else {
                continue;
            };

            let (derived, unique_id) = match calendar.try_borrow() {
                Ok(calendar_ref) => (calendar_ref.is_derived(), calendar_ref.get_unique_id()),
                Err(_) => continue,
            };

            // Java: if (calendar.isDerived()) { calendar.setType(CalendarType.RESOURCE);
            //          calendar.setPersonal(resourceCalendarMap.computeIfAbsent(calendar.getUniqueID(),
            //              k -> Collections.emptyList()).size() == 1); }
            if derived {
                let count = resource_calendar_map.get(&unique_id).copied().unwrap_or(0);

                if let Ok(mut calendar_ref) = calendar.try_borrow_mut() {
                    calendar_ref.set_type(Some(CalendarType::Resource));
                    calendar_ref.set_personal(count == 1);
                }
            }
        }

        Ok(())
    }

    /// Validate a calendar and fix problems.
    ///
    /// Corresponds to Java: `private void validateCalendar(ProjectCalendar calendar)`
    fn validate_calendar(&self, calendar: &Rc<RefCell<ProjectCalendar>>) {
        //
        // Java: if a calendar has no parent but has a weekday of type DEFAULT, assume it was supposed to have a
        //       parent and set the parent calendar to the project's default calendar.
        //
        // if (calendar.getParent() == null &&
        //     Stream.of(DayOfWeek.values()).anyMatch(d -> calendar.getCalendarDayType(d) == DayType.DEFAULT))
        // {
        //    calendar.setParent(m_projectFile.getDefaultCalendar());
        // }
        let days = [
            Weekday::Sun,
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
            Weekday::Sat,
        ];

        let needs_parent = match calendar.try_borrow() {
            Ok(calendar_ref) => {
                calendar_ref.get_parent().is_none()
                    && days.iter().any(|day| {
                        calendar_ref.get_calendar_day_type(*day) == Some(DayType::Default)
                    })
            }
            Err(_) => false,
        };

        if needs_parent {
            let default_calendar = self.m_project_file.as_ref().and_then(|file| {
                match file.try_borrow() {
                    Ok(file_ref) => file_ref.get_default_calendar(),
                    // Java has no such branch; Rust avoids a borrow conflict
                    Err(_) => None,
                }
            });

            if let Ok(mut calendar_ref) = calendar.try_borrow_mut() {
                calendar_ref.set_parent(default_calendar);
            }
        }

        //
        // Java: add calendar hours for working / non-working days that are missing working periods.
        //
        for day in days {
            let needs_hours = match calendar.try_borrow() {
                Ok(calendar_ref) => calendar_ref.get_calendar_hours_for_day(day).is_none(),
                Err(_) => false,
            };

            if !needs_hours {
                continue;
            }

            let day_type = match calendar.try_borrow() {
                Ok(calendar_ref) => calendar_ref.get_calendar_day_type(day),
                Err(_) => None,
            };

            // Java: if (dayType != DayType.DEFAULT) { ... }
            if day_type == Some(DayType::Default) {
                continue;
            }

            if let Ok(mut calendar_ref) = calendar.try_borrow_mut() {
                let hours = calendar_ref.add_calendar_hours(day);

                // Java: if (dayType == DayType.WORKING) { hours.add(DEFAULT_WORKING_MORNING);
                //                                             hours.add(DEFAULT_WORKING_AFTERNOON); }
                if day_type == Some(DayType::Working) {
                    hours.add(ProjectCalendarDays::default_working_morning());
                    hours.add(ProjectCalendarDays::default_working_afternoon());
                }
            }
        }
    }

    /// Populate the file creation record.
    ///
    /// Corresponds to Java: `static void populateFileCreationRecord(Record record, ProjectProperties properties)`
    ///
    /// It is a `static` method in Java (package-private); Rust likewise makes it an instance-independent associated function.
    fn populate_file_creation_record(record: &Record<'_>, properties: &mut ProjectProperties) {
        properties.set_mpx_program_name(record.get_string(0));
        properties.set_mpx_file_version(Some(crate::FileVersion::get_instance_optional(
            record.get_string(1).as_deref(),
        )));
        properties.set_mpx_code_page(Some(record.get_code_page(2)));
    }
}

impl Default for MPXReader {
    /// Corresponds to Java: implicit no-arg constructor.
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectReader for MPXReader {
    /// Register a listener.
    ///
    /// Corresponds to Java: `@Override public void addProjectListener(ProjectListener listener)`
    fn add_project_listener(&mut self, listener: Box<dyn ProjectListener>) {
        self.add_project_listener_default(listener);
    }

    /// Read one schedule by path.
    ///
    /// Corresponds to Java: `@Override public ProjectFile read(File file)` (defined in
    /// `AbstractProjectStreamReader`)
    fn read_file(&mut self, path: &std::path::Path) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        AbstractProjectStreamReader::read_file_default(self, path)
    }

    /// Read all schedules by path (MPX has only a single schedule).
    ///
    /// Corresponds to Java: `@Override public List<ProjectFile> readAll(File file)`
    fn read_all_file(
        &mut self,
        path: &std::path::Path,
    ) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
        AbstractProjectStreamReader::read_all_file_default(self, path)
    }

    /// Read one schedule from a reader.
    ///
    /// Corresponds to Java: `@Override public ProjectFile read(InputStream is)`
    fn read_reader(&mut self, reader: &mut dyn Read) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        self.read(reader)
    }

    /// Read all schedules from a reader (MPX has only a single schedule).
    ///
    /// Corresponds to Java: `@Override public List<ProjectFile> readAll(InputStream inputStream)`
    fn read_all_reader(
        &mut self,
        reader: &mut dyn Read,
    ) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
        AbstractProjectStreamReader::read_all_reader_default(self, reader)
    }
}

impl AbstractProjectReader for MPXReader {
    /// Get the registered listeners.
    ///
    /// Rust-specific (Java's `m_projectListeners` is a private field, see the
    /// [`crate::reader::abstract_project_reader`] file header).
    fn project_listeners(&self) -> &[SharedProjectListener] {
        &self.listeners
    }

    /// Get the registered listeners (mutable).
    ///
    /// Rust-specific (as above).
    fn project_listeners_mut(&mut self) -> &mut Vec<SharedProjectListener> {
        &mut self.listeners
    }
}

impl AbstractProjectStreamReader for MPXReader {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Read a chunk of MPX data from memory.
    fn read(data: &[u8]) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        let mut reader = MPXReader::new();
        reader.read_reader(&mut &data[..])
    }

    #[test]
    fn defaults_match_java() {
        let reader = MPXReader::new();

        // Java: private Locale m_locale = Locale.ENGLISH; private boolean m_ignoreTextModels = true;
        assert_eq!(reader.get_locale(), MpxLocale::En);
        assert!(reader.get_ignore_text_models());

        // Java: getSupportedLocales() delegates to LocaleUtility
        assert_eq!(MPXReader::get_supported_locales(), MpxLocale::ALL);
    }

    #[test]
    fn locale_and_text_model_flags_are_settable() {
        let mut reader = MPXReader::new();
        reader.set_locale(MpxLocale::De);
        reader.set_ignore_text_models(false);

        assert_eq!(reader.get_locale(), MpxLocale::De);
        assert!(!reader.get_ignore_text_models());
    }

    #[test]
    fn invalid_header_is_reported_as_a_read_error() {
        // Java: an invalid file header throws MPXJException(INVALID_FILE), but read's catch wraps it again
        let error = read(b"XXX,bad\r\n")
            .err()
            .expect("invalid file header should error");

        assert!(
            error.message().starts_with(MpxjError::READ_ERROR),
            "actual message: {}",
            error.message()
        );
        assert!(
            error.message().contains(MpxjError::INVALID_FILE),
            "actual message: {}",
            error.message()
        );
    }

    #[test]
    fn truncated_header_is_rejected() {
        assert!(read(b"MP").is_err());
        assert!(read(b"").is_err());
    }

    #[test]
    fn file_creation_record_is_read_through_the_byte_phase() {
        // Only the file creation record: the byte phase reads the code page and other info, then the character phase reads until EOF
        let file = read(b"MPX,Microsoft Project for Windows,4.0,ANSI\r\n").expect("should read successfully");

        let file_ref = file.borrow();
        let properties = file_ref.get_project_properties();

        assert_eq!(properties.get_mpx_program_name(), "Microsoft Project for Windows");
        assert_eq!(properties.get_mpx_code_page(), crate::CodePage::Ansi);
        // Java: m_delimiter = (char) data[3]; here it is ','
        assert_eq!(properties.get_mpx_delimiter(), ',');
        assert_eq!(properties.get_file_application().as_deref(), Some("Microsoft"));
        assert_eq!(properties.get_file_type().as_deref(), Some("MPX"));
    }

    #[test]
    fn custom_delimiter_from_the_header_is_used() {
        // The 4th byte is the delimiter: ';' is used here
        let data = b"MPX;Microsoft Project for Windows;4.0;ANSI\r\n";
        let file = read(data).expect("should read successfully");

        let file_ref = file.borrow();
        assert_eq!(file_ref.get_project_properties().get_mpx_delimiter(), ';');
        assert_eq!(
            file_ref.get_project_properties().get_mpx_code_page(),
            crate::CodePage::Ansi
        );
    }

    #[test]
    fn reader_can_be_reused() {
        let mut reader = MPXReader::new();

        let first = reader
            .read_reader(&mut &b"MPX,Microsoft Project for Windows,4.0,ANSI\r\n"[..])
            .expect("first read succeeded");
        let second = reader
            .read_reader(&mut &b"MPX,Microsoft Project for Windows,4.0,ANSI\r\n"[..])
            .expect("second read succeeded");

        // Each read creates a new ProjectFile (as in Java), and the temporary state has been cleaned up
        assert!(!Rc::ptr_eq(&first, &second));
        assert!(reader.m_project_file.is_none());
        assert!(reader.m_task_model.is_none());
        assert!(reader.m_deferred_relationships.is_empty());
    }

    #[test]
    fn unknown_record_numbers_are_rejected() {
        // 123 is not among any of MPXConstants' record numbers (note 999 is the placeholder value for the "file creation record")
        let data = b"MPX,Microsoft Project for Windows,4.0,ANSI\r\n123\r\n";
        let error = read(data).err().expect("unknown record number should error");

        assert!(
            error.message().contains(MpxjError::INVALID_RECORD),
            "actual message: {}",
            error.message()
        );
    }
}
