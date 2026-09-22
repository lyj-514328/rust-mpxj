//! Corresponds to Java: `org/mpxj/ProjectProperties.java`
//!
//! Property collection for the entire project (MPX header record / MPP project properties). Java declares it as
//! `public final class ProjectProperties extends AbstractFieldContainer<ProjectProperties> implements TimeUnitDefaultsContainer`,
//! This file makes the following trade-offs following this crate's established conventions.
//!
//! # Structural trade-offs: inheritance -> composition + forwarding
//!
//! Java obtains `set` / `get` / `get_cached_value` / by **inheriting** `AbstractFieldContainer`,
//! `addFieldListener` / `removeFieldListener` / `disableEvents` / `enableEvents`.
//! while the Rust side [`ProjectProperties`] holds one by field composition
//! [`crate::abstract_field_container::AbstractFieldContainer`] and forwards each of the above methods;
//! The three Java abstract methods are implemented in this class itself per the conventions of `src/abstract_field_container.rs`:
//!
//! | Java | Rust |
//! |---|---|
//! | `handleFieldChange(field, old, new)` | inside [`ProjectProperties::set`], calls the private function `handle_field_change` via the closure of `AbstractFieldContainer::update` |
//! | `getAlwaysCalculatedField(field)` | private function `is_always_calculated_field` (used by [`ProjectProperties::get`]) |
//! | `getCalculationMethod(field)` (`Function<ProjectProperties, Object>`) | private method `calculate_field` (dispatches with `match`, equivalent to Java's `CALCULATED_FIELD_MAP`) |
//!
//! # Differences from Java (structural)
//!
//! - **Parent file back-reference**: Java uses the strong reference `private final ProjectFile m_parentFile;`; the Rust side, following
//! this crate's established conventions, uses `Weak<RefCell<ProjectFile>>` instead (`ProjectFile` strongly references this class, and a reverse strong reference
//! would form an `Rc` cycle), and [`ProjectProperties::get_parent_file`] returns `None` when the parent file has been dropped.
//! - **Default calendar / activity default calendar**: Java stores only the unique ID in the field table and resolves the value via
//! `m_parentFile.getCalendars().getByUniqueID(...)`. Besides following this path, the Rust side
//! additionally holds a `Weak<RefCell<ProjectCalendar>>` (see [`ProjectProperties::set_default_calendar`])
//! as a fallback when the parent file is unavailable (not wired up / already dropped / borrow conflict), so that even in an environment without a parent file
//! the "set -> get" round trip can still be completed. Note: when only
//! [`ProjectProperties::set_default_calendar_unique_id`] is called without providing a calendar object, the value still comes from
//! the parent file's calendar container (consistent with Java; the cached value in the field table does not participate in reads because it is an "always calculated field").
//! - **`get` does not write back the calculated result**: Java's `AbstractFieldContainer.get`, when "the field is not always calculated and
//! the cache is empty", writes the calculated result back to the field table with `set(field, result)`. The Rust side's
//! [`crate::field_container::FieldContainer::get`] only accepts `&self` and cannot write back; the semantics are equivalent
//! (every `get` recalculates), only `get_cached_value` cannot observe that cache.
//! - **Representation of field values**: this crate's [`crate::field_value::FieldValue`] can only carry scalar
//!   (Integer / Short / Double / Boolean / String / Date / Time / DateTime / Duration /
//! Rate). The `Character` / enum / `UUID` / `Notes` / `Map` values in Java's field table are therefore
//! mapped as follows:
//! * `Character` (date / time / thousands / decimal separator, mpx delimiter) is stored as a single-character
//! `FieldValue::String`;
//! * enums with an int value (`TimeUnit`, `DateOrder`, `ProjectTimeFormat`, `ProjectDateFormat`,
//!     `CurrencySymbolPosition`,`ScheduleFrom`,`EarnedValueMethod`,`AccrueType`,
//! `FileVersion`) are stored as `FieldValue::Integer(getValue())`;
//! enums without an int value (`TaskType`, `CriticalActivityType`, `TotalSlackCalculationType`,
//! `RelationshipLagCalendar`, `SchedulingProgressedActivities`) are stored as the "declaration-order ordinal";
//! `CodePage` is stored as the MPX code page name (`get_value()`);
//! * `DayOfWeek` is stored as Java's 1 (Monday) ... 7 (Sunday);
//! * `UUID` is stored as a standard string;
//! * `Map<String, Object>` (`CUSTOM_PROPERTIES`) and `Map<ProjectCode, ProjectCodeValue>`
//! (`PROJECT_CODE_VALUES`) cannot fit into `FieldValue`; the former is carried by a separate field
//! [`ProjectProperties::get_custom_properties`] instead;
//! * `Notes` is a trait (the Java base class); this class still uniformly stores plain text,
//! and [`ProjectProperties::get_notes_object`] returns `PlainNotes`
//! (`Task` / `ResourceAssignment` have switched to [`crate::field_value::FieldValue::Notes`]
//! to faithfully preserve the subtype; this class keeps the original approach to avoid widening the change surface).
//! - **Overloads**: Rust has no overloading, so Java's `getBaselineDate(int)` / `setBaselineDate(int, LocalDateTime)`
//! are given the `_for_index` suffix per this crate's convention.
//! - **Null semantics**: Java dereferences possibly-`null` dates / containers in many places (throwing NPE), while the Rust side
//! does not panic and degrades gracefully to returning `None` / an empty string / Java's default value; see each method's docs for details.
//!
//! # Skipped Java methods and reasons
//!
//! - `getLocation()` / `setLocation(Location)`: `Location` is not declared in `src/lib.rs`
//! (`ProjectContext` likewise does not port `getLocations()`). `getLocationUniqueID()` /
//! `setLocationUniqueID(...)` has been translated.
//! - `getProjectCodeValues()` / `addProjectCodeValue(ProjectCodeValue)`:
//! `ProjectCode` / `ProjectCodeValue` are not declared in `src/lib.rs`.
//! - `getPopulatedFields()` has been ported by [`ProjectProperties::get_populated_fields`];
//! it returns a [`crate::field_key::FieldKey`] collection, using the custom field's unique ID as an equivalent identity key,
//! without including display metadata such as field names.
//! - `getResourcePoolObject()`: **cannot be ported** (see the next subsection of this section, "Reasons it cannot be ported").
//! `getResourcePoolFile()` / `setResourcePoolFile(...)` have been translated.
//! - `removeFieldListener(FieldListener)`: Java removes the listener by object identity, which Rust cannot express,
//! so per the conventions of [`crate::field_container::FieldContainer`] it is changed to `clear_field_listeners()`.
//! - Java-specific mechanisms (reflection, JDBC, XML/JAXB, GUI, `javax.*`, `Serializable`): none appear in this class
//! (the only related item is the `EnumHelper.createTypeArray` table construction for enums such as `TimeUnit`, which has been replaced with
//! `match` in each enum module).
//!
//! # Reasons it cannot be ported: `getResourcePoolObject()`
//!
//! Java's implementation is `return m_parentFile.readExternalProject(getResourcePoolFile());`,
//! i.e. "get the parent file -> read the external project (caching the result)". This path has two hard limitations on the Rust side:
//!
//! 1. **`&mut ProjectFile` cannot be obtained**: `readExternalProject` writes to
//! the cache of `ExternalProjectContainer` (`m_fileMap` / `m_projectMap`), corresponding to this crate's
//! [`ProjectFile::read_external_project`] (`&mut self`). However, this type's `parent_file` is
//! `Weak<RefCell<ProjectFile>>`, and obtaining `&mut ProjectFile` requires `try_borrow_mut`;
//! yet **the very precondition for calling this method is that the `ProjectFile` is already borrowed** (`ProjectProperties`
//! is a field of `ProjectFile`, obtainable only via `&ProjectFile` / `&mut ProjectFile`),
//! so except for a "detached `ProjectProperties` instance" (belonging to no file, a concept Java does not have)
//! `try_borrow_mut` necessarily fails => the method could only ever return `None`, which does not match Java's behavior.
//! This is a limitation imposed by the `RefCell` borrow model (Java's stable references do not have this problem).
//! 2. **External file reading itself is not wired up**: the read action depends on `UniversalProjectReader` (`Class.forName`
//! reflection dispatch), which this project categorically skips; `ExternalProjectContainer` can only rely on the caller
//!    [`crate::external_project_container::ExternalProjectContainer::set_project_reader`]
//! injecting a callback, and when none is injected the read always fails (returns `None`, equivalent to Java's `catch (MPXJException)`).
//!
//! When this feature is needed, hold `&mut ProjectFile` (for example the call form of `Task::expand_subproject`)
//! and then call [`ProjectFile::read_external_project`] directly; its semantics match Java.
//!
//! # Model-layer interfaces used (all already in place; listed here only as an index)
//!
//! `ProjectFile` / `Task` are used with the following signatures:
//!
//! ```text
//! ProjectFile::get_project_context(&self) -> Rc<RefCell<ProjectContext>>
//! ProjectFile::get_child_tasks(&self) -> Vec<Rc<RefCell<Task>>>
//! ProjectFile::get_earliest_start_date(&self) -> Option<NaiveDateTime>   // Java: getEarliestStartDate()
//! ProjectFile::get_latest_finish_date(&self) -> Option<NaiveDateTime>    // Java: getLatestFinishDate()
//! ProjectFile::get_actual_start(&self) -> Option<NaiveDateTime>          // Java: getActualStart()
//! ProjectFile::get_actual_finish(&self) -> Option<NaiveDateTime>         // Java: getActualFinish()
//! Task: FieldContainer (used for Java's getValueFromSummaryTask)
//! ```
//!
//! The default calendar and time unit defaults are both accessed via `ProjectContext` (already in place), matching Java's
//! `m_parentFile.getCalendars()` / `m_parentFile.getProjectContext().getTimeUnitDefaults()`
//! value retrieval path.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use chrono::{Local, NaiveDateTime, NaiveTime, Weekday};
use uuid::Uuid;

use crate::abstract_field_container::AbstractFieldContainer;
use crate::accrue_type::AccrueType;
use crate::code_page::CodePage;
use crate::critical_activity_type::CriticalActivityType;
use crate::currency_symbol_position::CurrencySymbolPosition;
use crate::date_order::DateOrder;
use crate::duration::Duration;
use crate::earned_value_method::EarnedValueMethod;
use crate::field_container::FieldContainer;
use crate::field_key::FieldKey;
use crate::field_value::FieldValue;
use crate::file_version::FileVersion;
use crate::listener::field_listener::FieldListener;
use crate::local_date_time_range::LocalDateTimeRange;
use crate::mpxj_enum::MpxjEnum;
use crate::notes::{Notes, PlainNotes};
use crate::project_calendar::ProjectCalendar;
use crate::project_context::ProjectContext;
use crate::project_date_format::ProjectDateFormat;
use crate::project_field::ProjectField;
use crate::project_file::ProjectFile;
use crate::project_time_format::ProjectTimeFormat;
use crate::rate::Rate;
use crate::relationship_lag_calendar::RelationshipLagCalendar;
use crate::schedule_from::ScheduleFrom;
use crate::scheduling_progressed_activities::SchedulingProgressedActivities;
use crate::task_field::TaskField;
use crate::task_type::TaskType;
use crate::time_unit::TimeUnit;
use crate::time_unit_defaults::TimeUnitDefaults;
use crate::time_unit_defaults_container::TimeUnitDefaultsContainer;
use crate::total_slack_calculation_type::TotalSlackCalculationType;

/// Default time separator.
///
/// Corresponds to Java: `private static final char DEFAULT_TIME_SEPARATOR = ':';`
const DEFAULT_TIME_SEPARATOR: char = ':';

/// Default date separator.
///
/// Corresponds to Java: `private static final char DEFAULT_DATE_SEPARATOR = '/';`
const DEFAULT_DATE_SEPARATOR: char = '/';

/// Default thousands separator.
///
/// Corresponds to Java: `private static final char DEFAULT_THOUSANDS_SEPARATOR = ',';`
const DEFAULT_THOUSANDS_SEPARATOR: char = ',';

/// Default decimal separator.
///
/// Corresponds to Java: `private static final char DEFAULT_DECIMAL_SEPARATOR = '.';`
const DEFAULT_DECIMAL_SEPARATOR: char = '.';

/// Default currency symbol.
///
/// Corresponds to Java: `private static final String DEFAULT_CURRENCY_SYMBOL = "$";`
const DEFAULT_CURRENCY_SYMBOL: &str = "$";

/// Default currency decimal places.
///
/// Corresponds to Java: `private static final Integer DEFAULT_CURRENCY_DIGITS = Integer.valueOf(2);`
const DEFAULT_CURRENCY_DIGITS: i32 = 2;

/// Default MPX delimiter.
///
/// Corresponds to Java: `private static final char DEFAULT_MPX_DELIMITER = ',';`
const DEFAULT_MPX_DELIMITER: char = ',';

/// Default earned value baseline number.
///
/// Corresponds to Java: `private static final Integer DEFAULT_BASELINE_FOR_EARNED_VALUE = Integer.valueOf(0);`
const DEFAULT_BASELINE_FOR_EARNED_VALUE: i32 = 0;

/// Default fiscal year start month.
///
/// Corresponds to Java: `private static final Integer DEFAULT_FISCAL_YEAR_START_MONTH = Integer.valueOf(1);`
const DEFAULT_FISCAL_YEAR_START_MONTH: i32 = 1;

/// Default value for the other project assignment priority.
///
/// Corresponds to Java: `private static final Integer DEFAULT_OTHER_PROJECT_ASSIGNMENT_PRIORITY = Integer.valueOf(5);`
const DEFAULT_OTHER_PROJECT_ASSIGNMENT_PRIORITY: i32 = 5;

/// Default value for the resource overallocation percentage.
///
/// Corresponds to Java: `private static final Double DEFAULT_OVERALLOCATION = Double.valueOf(25.0);`
const DEFAULT_OVERALLOCATION: f64 = 25.0;

/// Default value for the maximum number of slack paths.
///
/// Corresponds to Java: `private static final Integer DEFAULT_FLOAT_PATHS = Integer.valueOf(10);`
const DEFAULT_FLOAT_PATHS: i32 = 10;

/// Default value for the Activity ID suffix.
///
/// Corresponds to Java: `private static final Integer DEFAULT_ACTIVITY_ID_SUFFIX = Integer.valueOf(1000);`
const DEFAULT_ACTIVITY_ID_SUFFIX: i32 = 1000;

/// Default value for the Activity ID auto-increment step.
///
/// Corresponds to Java: `private static final Integer DEFAULT_ACTIVITY_ID_INCREMENT = Integer.valueOf(10);`
const DEFAULT_ACTIVITY_ID_INCREMENT: i32 = 10;

/// Default value for the MPX program name.
///
/// Corresponds to Java: `CALCULATED_FIELD_MAP.put(ProjectField.MPX_PROGRAM_NAME, p -> "Microsoft Project for Windows");`
const DEFAULT_MPX_PROGRAM_NAME: &str = "Microsoft Project for Windows";

/// Default value for the AM text.
///
/// Corresponds to Java: `CALCULATED_FIELD_MAP.put(ProjectField.AM_TEXT, p -> "am");`
const DEFAULT_AM_TEXT: &str = "am";

/// Default value for the PM text.
///
/// Corresponds to Java: `CALCULATED_FIELD_MAP.put(ProjectField.PM_TEXT, p -> "pm");`
const DEFAULT_PM_TEXT: &str = "pm";

/// Default value for the WBS code separator.
///
/// Corresponds to Java: `CALCULATED_FIELD_MAP.put(ProjectField.WBS_CODE_SEPARATOR, p -> ".");`
const DEFAULT_WBS_CODE_SEPARATOR: &str = ".";

/// Default value for the leveling priority expression.
///
/// Corresponds to Java: `CALCULATED_FIELD_MAP.put(ProjectField.LEVELING_PRIORITIES, p -> "(0||priority_type(sort_type|ASC)())");`
const DEFAULT_LEVELING_PRIORITIES: &str = "(0||priority_type(sort_type|ASC)())";

/// Default value for the Activity ID prefix.
///
/// Corresponds to Java: `CALCULATED_FIELD_MAP.put(ProjectField.ACTIVITY_ID_PREFIX, p -> "A");`
const DEFAULT_ACTIVITY_ID_PREFIX: &str = "A";

/// Java's `ProjectFieldLists.BASELINE_DATES`.
///
/// Corresponds to Java: `org/mpxj/common/ProjectFieldLists.java` (that helper class is not ported, so the constant is replicated here).
const BASELINE_DATES: [ProjectField; 10] = [
    ProjectField::Baseline1Date,
    ProjectField::Baseline2Date,
    ProjectField::Baseline3Date,
    ProjectField::Baseline4Date,
    ProjectField::Baseline5Date,
    ProjectField::Baseline6Date,
    ProjectField::Baseline7Date,
    ProjectField::Baseline8Date,
    ProjectField::Baseline9Date,
    ProjectField::Baseline10Date,
];

/// Default start time (08:00).
///
/// Corresponds to Java: `private static final LocalTime DEFAULT_START_TIME = LocalTime.of(8, 0);`
fn default_start_time() -> NaiveTime {
    NaiveTime::from_hms_opt(8, 0, 0).unwrap_or(NaiveTime::MIN)
}

/// Default critical slack threshold (0 days).
///
/// Corresponds to Java: `private static final Duration DEFAULT_CRITICAL_SLACK_LIMIT = Duration.getInstance(0, TimeUnit.DAYS);`
fn default_critical_slack_limit() -> Duration {
    Duration::get_instance_int(0, TimeUnit::Days)
}

/// Minimum slack to preserve during leveling (1 hour).
///
/// Corresponds to Java: `private static final Duration DEFAULT_MINIMUM_FLOAT = Duration.getInstance(1, TimeUnit.HOURS);`
fn default_minimum_float() -> Duration {
    Duration::get_instance_int(1, TimeUnit::Hours)
}

/// Project properties (corresponding to the MPX header record / MPP project properties).
///
/// Corresponds to Java: `public final class ProjectProperties extends AbstractFieldContainer<ProjectProperties> implements TimeUnitDefaultsContainer`
pub struct ProjectProperties {
/// Parent project file (Java uses a strong reference; the Rust side uses a weak reference to break the cycle, see the file header).
    ///
    /// Corresponds to Java field: `private final ProjectFile m_parentFile;`
    parent_file: Weak<RefCell<ProjectFile>>,

/// Field storage (corresponding to Java's inheritance relationship `extends AbstractFieldContainer<ProjectProperties>`).
    fields: AbstractFieldContainer,

/// Default calendar (used only as a fallback when the parent file is unavailable, see the file header).
    ///
/// Corresponds to Java: no such field; Java resolves it via `ProjectField.DEFAULT_CALENDAR_UNIQUE_ID` +
/// `m_parentFile.getCalendars()`.
    default_calendar: Weak<RefCell<ProjectCalendar>>,

/// Activity default calendar (used only as a fallback when the parent file is unavailable, see the file header).
    ///
/// Corresponds to Java: no such field; Java resolves it via `ProjectField.ACTIVITY_DEFAULT_CALENDAR_UNIQUE_ID` +
/// `m_parentFile.getCalendars()`.
    activity_default_calendar: Weak<RefCell<ProjectCalendar>>,

/// Custom document properties.
    ///
/// Corresponds to Java field: the `Map<String, Object>` stored in `ProjectField.CUSTOM_PROPERTIES`.
    ///
/// `FieldValue` cannot carry a `Map`, so this crate carries it in a separate field; `get(ProjectField.CUSTOM_PROPERTIES)`
/// therefore returns `None` (see the file header).
    custom_properties: HashMap<String, FieldValue>,
}

impl ProjectProperties {
/// Constructor.
    ///
    /// Corresponds to Java: `ProjectProperties(ProjectFile file)`
    pub fn new(parent_file: Weak<RefCell<ProjectFile>>) -> ProjectProperties {
        ProjectProperties {
            parent_file,
            fields: AbstractFieldContainer::new(),
            default_calendar: Weak::new(),
            activity_default_calendar: Weak::new(),
            custom_properties: HashMap::new(),
        }
    }

/// Get the parent project file (returns `None` when the parent file has been dropped).
    ///
    /// Corresponds to Java: `public ProjectFile getParentFile()`
    pub fn get_parent_file(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.parent_file.upgrade()
    }

    //
// Default duration / work units, rates
    //

/// Get the default duration unit; uses Java's default value `DAYS` when unset.
    ///
    /// Corresponds to Java: `public TimeUnit getDefaultDurationUnits()`
    pub fn get_default_duration_units(&self) -> TimeUnit {
        self.opt_int(ProjectField::DefaultDurationUnits.into())
            .map_or(TimeUnit::Days, TimeUnit::get_instance)
    }

/// Set the default duration unit.
    ///
    /// Corresponds to Java: `public void setDefaultDurationUnits(TimeUnit units)`
    pub fn set_default_duration_units(&mut self, units: Option<TimeUnit>) {
        self.set_field(
            ProjectField::DefaultDurationUnits.into(),
            units.map(|units| FieldValue::Integer(units.value())),
        );
    }

/// Get the "default duration type is fixed" flag.
    ///
    /// Corresponds to Java: `public boolean getDefaultDurationIsFixed()`
    pub fn get_default_duration_is_fixed(&self) -> bool {
        self.bool_value(ProjectField::DefaultDurationIsFixed.into())
    }

/// Set the "default duration type is fixed" flag.
    ///
    /// Corresponds to Java: `public void setDefaultDurationIsFixed(boolean fixed)`
    pub fn set_default_duration_is_fixed(&mut self, fixed: bool) {
        self.set_boolean(ProjectField::DefaultDurationIsFixed, fixed);
    }

/// Get the default work unit; uses Java's default value `HOURS` when unset.
    ///
    /// Corresponds to Java: `public TimeUnit getDefaultWorkUnits()`
    pub fn get_default_work_units(&self) -> TimeUnit {
        self.opt_int(ProjectField::DefaultWorkUnits.into())
            .map_or(TimeUnit::Hours, TimeUnit::get_instance)
    }

/// Set the default work unit.
    ///
    /// Corresponds to Java: `public void setDefaultWorkUnits(TimeUnit units)`
    pub fn set_default_work_units(&mut self, units: Option<TimeUnit>) {
        self.set_field(
            ProjectField::DefaultWorkUnits.into(),
            units.map(|units| FieldValue::Integer(units.value())),
        );
    }

/// Get the default standard rate.
    ///
    /// Corresponds to Java: `public Rate getDefaultStandardRate()`
    pub fn get_default_standard_rate(&self) -> Option<Rate> {
        self.opt_rate(ProjectField::DefaultStandardRate.into())
    }

/// Set the default standard rate.
    ///
    /// Corresponds to Java: `public void setDefaultStandardRate(Rate rate)`
    pub fn set_default_standard_rate(&mut self, rate: Option<Rate>) {
        self.set_field(
            ProjectField::DefaultStandardRate.into(),
            rate.map(FieldValue::Rate),
        );
    }

/// Get the default overtime rate.
    ///
    /// Corresponds to Java: `public Rate getDefaultOvertimeRate()`
    pub fn get_default_overtime_rate(&self) -> Option<Rate> {
        self.opt_rate(ProjectField::DefaultOvertimeRate.into())
    }

/// Set the default overtime rate.
    ///
    /// Corresponds to Java: `public void setDefaultOvertimeRate(Rate rate)`
    pub fn set_default_overtime_rate(&mut self, rate: Option<Rate>) {
        self.set_field(
            ProjectField::DefaultOvertimeRate.into(),
            rate.map(FieldValue::Rate),
        );
    }

/// Get the "update resource status when updating task status" flag.
    ///
    /// Corresponds to Java: `public boolean getUpdatingTaskStatusUpdatesResourceStatus()`
    pub fn get_updating_task_status_updates_resource_status(&self) -> bool {
        self.bool_value(ProjectField::UpdatingTaskStatusUpdatesResourceStatus.into())
    }

/// Set the "update resource status when updating task status" flag.
    ///
    /// Corresponds to Java: `public void setUpdatingTaskStatusUpdatesResourceStatus(boolean flag)`
    pub fn set_updating_task_status_updates_resource_status(&mut self, flag: bool) {
        self.set_boolean(ProjectField::UpdatingTaskStatusUpdatesResourceStatus, flag);
    }

/// Get the "split in-progress tasks" flag.
    ///
    /// Corresponds to Java: `public boolean getSplitInProgressTasks()`
    pub fn get_split_in_progress_tasks(&self) -> bool {
        self.bool_value(ProjectField::SplitInProgressTasks.into())
    }

/// Set the "split in-progress tasks" flag.
    ///
    /// Corresponds to Java: `public void setSplitInProgressTasks(boolean flag)`
    pub fn set_split_in_progress_tasks(&mut self, flag: bool) {
        self.set_boolean(ProjectField::SplitInProgressTasks, flag);
    }

    //
// Date / time formats
    //

/// Get the date order (DMY / MDY / YMD); uses Java's default value `DMY` when unset.
    ///
    /// Corresponds to Java: `public DateOrder getDateOrder()`
    pub fn get_date_order(&self) -> DateOrder {
        self.opt_int(ProjectField::DateOrder.into())
            .map_or(DateOrder::Dmy, DateOrder::get_instance)
    }

/// Set the date order.
    ///
    /// Corresponds to Java: `public void setDateOrder(DateOrder dateOrder)`
    pub fn set_date_order(&mut self, date_order: Option<DateOrder>) {
        self.set_field(
            ProjectField::DateOrder.into(),
            date_order.map(|value| FieldValue::Integer(value.value())),
        );
    }

/// Get the time format; uses Java's default value `TWELVE_HOUR` when unset.
    ///
    /// Corresponds to Java: `public ProjectTimeFormat getTimeFormat()`
    pub fn get_time_format(&self) -> ProjectTimeFormat {
        self.opt_int(ProjectField::TimeFormat.into())
            .map_or(ProjectTimeFormat::TwelveHour, ProjectTimeFormat::get_instance)
    }

/// Set the time format.
    ///
    /// Corresponds to Java: `public void setTimeFormat(ProjectTimeFormat timeFormat)`
    pub fn set_time_format(&mut self, time_format: Option<ProjectTimeFormat>) {
        self.set_field(
            ProjectField::TimeFormat.into(),
            time_format.map(|value| FieldValue::Integer(value.value())),
        );
    }

/// Get the default start time; uses Java's default value 08:00 when unset.
    ///
    /// Corresponds to Java: `public LocalTime getDefaultStartTime()`
    pub fn get_default_start_time(&self) -> NaiveTime {
        self.opt_time(ProjectField::DefaultStartTime.into())
            .unwrap_or_else(default_start_time)
    }

/// Set the default start time.
    ///
    /// Corresponds to Java: `public void setDefaultStartTime(LocalTime defaultStartTime)`
    pub fn set_default_start_time(&mut self, default_start_time: Option<NaiveTime>) {
        self.set_field(
            ProjectField::DefaultStartTime.into(),
            default_start_time.map(FieldValue::Time),
        );
    }

/// Get the default end time.
    ///
    /// Corresponds to Java: `public LocalTime getDefaultEndTime()`
    pub fn get_default_end_time(&self) -> Option<NaiveTime> {
        self.opt_time(ProjectField::DefaultEndTime.into())
    }

/// Set the default end time.
    ///
    /// Corresponds to Java: `public void setDefaultEndTime(LocalTime date)`
    pub fn set_default_end_time(&mut self, date: Option<NaiveTime>) {
        self.set_field(ProjectField::DefaultEndTime.into(), date.map(FieldValue::Time));
    }

/// Get the date separator; uses Java's default value `'/'` when unset.
    ///
    /// Corresponds to Java: `public char getDateSeparator()`
    ///
/// Java dereferences `null` and throws NPE when the field is `null`; the Rust side falls back to the default value.
    pub fn get_date_separator(&self) -> char {
        self.char_value(ProjectField::DateSeparator.into(), DEFAULT_DATE_SEPARATOR)
    }

/// Set the date separator.
    ///
    /// Corresponds to Java: `public void setDateSeparator(char dateSeparator)`
    ///
/// `FieldValue` has no character variant, so it is stored as a single-character string (see the file header).
    pub fn set_date_separator(&mut self, date_separator: char) {
        self.set_char(ProjectField::DateSeparator, date_separator);
    }

/// Get the time separator; uses Java's default value `':'` when unset.
    ///
    /// Corresponds to Java: `public char getTimeSeparator()`
    pub fn get_time_separator(&self) -> char {
        self.char_value(ProjectField::TimeSeparator.into(), DEFAULT_TIME_SEPARATOR)
    }

/// Set the time separator.
    ///
    /// Corresponds to Java: `public void setTimeSeparator(char timeSeparator)`
    pub fn set_time_separator(&mut self, time_separator: char) {
        self.set_char(ProjectField::TimeSeparator, time_separator);
    }

/// Get the AM text; uses Java's default value `"am"` when unset.
    ///
    /// Corresponds to Java: `public String getAMText()`
    pub fn get_am_text(&self) -> String {
        self.string_value(ProjectField::AmText.into(), DEFAULT_AM_TEXT)
    }

/// Set the AM text.
    ///
    /// Corresponds to Java: `public void setAMText(String amText)`
    pub fn set_am_text(&mut self, am_text: Option<String>) {
        self.set_field(ProjectField::AmText.into(), am_text.map(FieldValue::String));
    }

/// Get the PM text; uses Java's default value `"pm"` when unset.
    ///
    /// Corresponds to Java: `public String getPMText()`
    pub fn get_pm_text(&self) -> String {
        self.string_value(ProjectField::PmText.into(), DEFAULT_PM_TEXT)
    }

/// Set the PM text.
    ///
    /// Corresponds to Java: `public void setPMText(String pmText)`
    pub fn set_pm_text(&mut self, pm_text: Option<String>) {
        self.set_field(ProjectField::PmText.into(), pm_text.map(FieldValue::String));
    }

/// Get the date format; uses Java's default value `DD_MM_YYYY` when unset.
    ///
    /// Corresponds to Java: `public ProjectDateFormat getDateFormat()`
    pub fn get_date_format(&self) -> ProjectDateFormat {
        self.opt_int(ProjectField::DateFormat.into())
            .map_or(ProjectDateFormat::DdMmYyyy, ProjectDateFormat::get_instance)
    }

/// Set the date format.
    ///
    /// Corresponds to Java: `public void setDateFormat(ProjectDateFormat dateFormat)`
    pub fn set_date_format(&mut self, date_format: Option<ProjectDateFormat>) {
        self.set_field(
            ProjectField::DateFormat.into(),
            date_format.map(|value| FieldValue::Integer(value.value())),
        );
    }

/// Get the bar text date format; uses Java's default value `DD_MM_YYYY` when unset.
    ///
    /// Corresponds to Java: `public ProjectDateFormat getBarTextDateFormat()`
    pub fn get_bar_text_date_format(&self) -> ProjectDateFormat {
        self.opt_int(ProjectField::BarTextDateFormat.into())
            .map_or(ProjectDateFormat::DdMmYyyy, ProjectDateFormat::get_instance)
    }

/// Set the bar text date format.
    ///
    /// Corresponds to Java: `public void setBarTextDateFormat(ProjectDateFormat dateFormat)`
    pub fn set_bar_text_date_format(&mut self, date_format: Option<ProjectDateFormat>) {
        self.set_field(
            ProjectField::BarTextDateFormat.into(),
            date_format.map(|value| FieldValue::Integer(value.value())),
        );
    }

    //
// Title / company / manager
    //

/// Set the project title.
    ///
    /// Corresponds to Java: `public void setProjectTitle(String projectTitle)`
    pub fn set_project_title(&mut self, project_title: Option<String>) {
        self.set_field(
            ProjectField::ProjectTitle.into(),
            project_title.map(FieldValue::String),
        );
    }

/// Get the project title.
    ///
    /// Corresponds to Java: `public String getProjectTitle()`
    pub fn get_project_title(&self) -> Option<String> {
        self.opt_string(ProjectField::ProjectTitle.into())
    }

/// Set the company name.
    ///
    /// Corresponds to Java: `public void setCompany(String company)`
    pub fn set_company(&mut self, company: Option<String>) {
        self.set_field(ProjectField::Company.into(), company.map(FieldValue::String));
    }

/// Get the company name.
    ///
    /// Corresponds to Java: `public String getCompany()`
    pub fn get_company(&self) -> Option<String> {
        self.opt_string(ProjectField::Company.into())
    }

/// Set the manager name.
    ///
    /// Corresponds to Java: `public void setManager(String manager)`
    pub fn set_manager(&mut self, manager: Option<String>) {
        self.set_field(ProjectField::Manager.into(), manager.map(FieldValue::String));
    }

/// Get the manager name.
    ///
    /// Corresponds to Java: `public String getManager()`
    pub fn get_manager(&self) -> Option<String> {
        self.opt_string(ProjectField::Manager.into())
    }

    //
// Default calendar / activity default calendar
    //

/// Set the default calendar unique ID.
    ///
    /// Corresponds to Java: `public void setDefaultCalendarUniqueID(Integer id)`
    pub fn set_default_calendar_unique_id(&mut self, id: Option<i32>) {
        self.set_field(
            ProjectField::DefaultCalendarUniqueId.into(),
            id.map(FieldValue::Integer),
        );
    }

/// Get the default calendar unique ID (uses the value from the parent file's calendar container when the parent file is available).
    ///
    /// Corresponds to Java: `public Integer getDefaultCalendarUniqueID()`
    ///
/// In Java this field is an "always calculated field"; when the parent file is unavailable the Rust side falls back to
/// the unique ID of the calendar pointed to by the [`ProjectProperties::default_calendar`] weak reference.
    pub fn get_default_calendar_unique_id(&self) -> Option<i32> {
        if let Some(id) = self.calculate_default_calendar_unique_id() {
            return Some(id);
        }

        self.default_calendar
            .upgrade()
            .and_then(|calendar| calendar.try_borrow().ok().and_then(|item| item.get_unique_id()))
    }

/// Set the default calendar.
    ///
    /// Corresponds to Java: `public void setDefaultCalendar(ProjectCalendar calendar)`
    ///
/// In addition to writing the unique ID field, it also records the weak-reference fallback value in
/// [`ProjectProperties::default_calendar`] (see the file header).
    pub fn set_default_calendar(&mut self, calendar: Option<Rc<RefCell<ProjectCalendar>>>) {
        let unique_id = calendar
            .as_ref()
            .and_then(|calendar| calendar.try_borrow().ok().and_then(|item| item.get_unique_id()));
        self.set_default_calendar_unique_id(unique_id);
        self.default_calendar = match &calendar {
            Some(calendar) => Rc::downgrade(calendar),
            None => Weak::new(),
        };
    }

/// Get the default calendar.
    ///
    /// Corresponds to Java: `public ProjectCalendar getDefaultCalendar()`
    ///
/// First resolves by unique ID in the parent file's calendar container; falls back to
/// the [`ProjectProperties::default_calendar`] weak reference when the parent file is unavailable or there is no match.
    pub fn get_default_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        if let Some(context) = self.parent_context() {
            let id = self.calculate_default_calendar_unique_id();
            if let Some(calendar) = self.find_calendar(&context, id) {
                return Some(calendar);
            }
        }

        self.default_calendar.upgrade()
    }

/// Set the activity default calendar unique ID.
    ///
    /// Corresponds to Java: `public void setActivityDefaultCalendarUniqueID(Integer id)`
    pub fn set_activity_default_calendar_unique_id(&mut self, id: Option<i32>) {
        self.set_field(
            ProjectField::ActivityDefaultCalendarUniqueId.into(),
            id.map(FieldValue::Integer),
        );
    }

/// Get the activity default calendar unique ID.
    ///
    /// Corresponds to Java: `public Integer getActivityDefaultCalendarUniqueID()`
    pub fn get_activity_default_calendar_unique_id(&self) -> Option<i32> {
        let result = self.opt_int(ProjectField::ActivityDefaultCalendarUniqueId.into());
        if result.is_some() {
            return result;
        }

        self.activity_default_calendar
            .upgrade()
            .and_then(|calendar| calendar.try_borrow().ok().and_then(|item| item.get_unique_id()))
    }

/// Set the activity default calendar.
    ///
    /// Corresponds to Java: `public void setActivityDefaultCalendar(ProjectCalendar calendar)`
    ///
/// Java does not null-check `calendar` (passing `null` throws NPE); the Rust side treats `None` as
/// "clear the activity default calendar".
    pub fn set_activity_default_calendar(&mut self, calendar: Option<Rc<RefCell<ProjectCalendar>>>) {
        let unique_id = calendar
            .as_ref()
            .and_then(|calendar| calendar.try_borrow().ok().and_then(|item| item.get_unique_id()));
        self.set_activity_default_calendar_unique_id(unique_id);
        self.activity_default_calendar = match &calendar {
            Some(calendar) => Rc::downgrade(calendar),
            None => Weak::new(),
        };
    }

/// Get the activity default calendar.
    ///
    /// Corresponds to Java: `public ProjectCalendar getActivityDefaultCalendar()`
    pub fn get_activity_default_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        if let Some(context) = self.parent_context() {
            let id = self.get_activity_default_calendar_unique_id();
            if let Some(calendar) = self.find_calendar(&context, id) {
                return Some(calendar);
            }
        }

        self.activity_default_calendar.upgrade()
    }

    //
// Project start / finish dates
    //

/// Set the project start date.
    ///
    /// Corresponds to Java: `public void setStartDate(LocalDateTime startDate)`
    pub fn set_start_date(&mut self, start_date: Option<NaiveDateTime>) {
        self.set_field(ProjectField::StartDate.into(), start_date.map(FieldValue::DateTime));
    }

/// Get the project start date (uses the earliest task start date in the file when not explicitly set).
    ///
    /// Corresponds to Java: `public LocalDateTime getStartDate()`
    pub fn get_start_date(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::StartDate.into())
    }

/// Get the project finish date (uses the latest task finish date in the file when not explicitly set).
    ///
    /// Corresponds to Java: `public LocalDateTime getFinishDate()`
    pub fn get_finish_date(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::FinishDate.into())
    }

/// Set the project finish date.
    ///
    /// Corresponds to Java: `public void setFinishDate(LocalDateTime finishDate)`
    pub fn set_finish_date(&mut self, finish_date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::FinishDate.into(),
            finish_date.map(FieldValue::DateTime),
        );
    }

/// Get the scheduling direction (schedule from start date / finish date); uses Java's default value `START` when unset.
    ///
    /// Corresponds to Java: `public ScheduleFrom getScheduleFrom()`
    pub fn get_schedule_from(&self) -> ScheduleFrom {
        self.opt_int(ProjectField::ScheduleFrom.into())
            .map_or(ScheduleFrom::Start, ScheduleFrom::get_instance)
    }

/// Set the scheduling direction.
    ///
    /// Corresponds to Java: `public void setScheduleFrom(ScheduleFrom scheduleFrom)`
    pub fn set_schedule_from(&mut self, schedule_from: Option<ScheduleFrom>) {
        self.set_field(
            ProjectField::ScheduleFrom.into(),
            schedule_from.map(|value| FieldValue::Integer(value.value())),
        );
    }

/// Get the current date; uses Java's default value "the current moment" when unset.
    ///
    /// Corresponds to Java: `public LocalDateTime getCurrentDate()`
    pub fn get_current_date(&self) -> NaiveDateTime {
        self.opt_date_time(ProjectField::CurrentDate.into())
            .unwrap_or_else(|| Local::now().naive_local())
    }

/// Set the current date.
    ///
    /// Corresponds to Java: `public void setCurrentDate(LocalDateTime currentDate)`
    pub fn set_current_date(&mut self, current_date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::CurrentDate.into(),
            current_date.map(FieldValue::DateTime),
        );
    }

/// Get the notes text.
    ///
    /// Corresponds to Java: `public String getComments()`
    pub fn get_comments(&self) -> Option<String> {
        self.opt_string(ProjectField::Comments.into())
    }

/// Set the notes text.
    ///
    /// Corresponds to Java: `public void setComments(String comments)`
    pub fn set_comments(&mut self, comments: Option<String>) {
        self.set_field(ProjectField::Comments.into(), comments.map(FieldValue::String));
    }

    //
// Cost / work / duration / percent complete
    //

/// Get the project cost.
    ///
    /// Corresponds to Java: `public Number getCost()`
    ///
/// Java's `Number` is unified to `f64` on the Rust side (`FieldValue::as_double` accepts both integers and floats).
    pub fn get_cost(&self) -> Option<f64> {
        self.opt_double(ProjectField::Cost.into())
    }

/// Set the project cost.
    ///
    /// Corresponds to Java: `public void setCost(Number cost)`
    pub fn set_cost(&mut self, cost: Option<f64>) {
        self.set_field(ProjectField::Cost.into(), cost.map(FieldValue::Double));
    }

/// Set the baseline project cost.
    ///
    /// Corresponds to Java: `public void setBaselineCost(Number baselineCost)`
    pub fn set_baseline_cost(&mut self, baseline_cost: Option<f64>) {
        self.set_field(
            ProjectField::BaselineCost.into(),
            baseline_cost.map(FieldValue::Double),
        );
    }

/// Get the baseline project cost.
    ///
    /// Corresponds to Java: `public Number getBaselineCost()`
    pub fn get_baseline_cost(&self) -> Option<f64> {
        self.opt_double(ProjectField::BaselineCost.into())
    }

/// Set the actual project cost.
    ///
    /// Corresponds to Java: `public void setActualCost(Number actualCost)`
    pub fn set_actual_cost(&mut self, actual_cost: Option<f64>) {
        self.set_field(
            ProjectField::ActualCost.into(),
            actual_cost.map(FieldValue::Double),
        );
    }

/// Get the actual project cost.
    ///
    /// Corresponds to Java: `public Number getActualCost()`
    pub fn get_actual_cost(&self) -> Option<f64> {
        self.opt_double(ProjectField::ActualCost.into())
    }

/// Set the project work.
    ///
    /// Corresponds to Java: `public void setWork(Duration work)`
    pub fn set_work(&mut self, work: Option<Duration>) {
        self.set_field(ProjectField::Work.into(), work.map(FieldValue::Duration));
    }

/// Get the project work.
    ///
    /// Corresponds to Java: `public Duration getWork()`
    pub fn get_work(&self) -> Option<Duration> {
        self.opt_duration(ProjectField::Work.into())
    }

/// Set the baseline project work.
    ///
    /// Corresponds to Java: `public void setBaselineWork(Duration baselineWork)`
    pub fn set_baseline_work(&mut self, baseline_work: Option<Duration>) {
        self.set_field(
            ProjectField::BaselineWork.into(),
            baseline_work.map(FieldValue::Duration),
        );
    }

/// Get the baseline project work.
    ///
    /// Corresponds to Java: `public Duration getBaselineWork()`
    pub fn get_baseline_work(&self) -> Option<Duration> {
        self.opt_duration(ProjectField::BaselineWork.into())
    }

/// Set the actual project work.
    ///
    /// Corresponds to Java: `public void setActualWork(Duration actualWork)`
    pub fn set_actual_work(&mut self, actual_work: Option<Duration>) {
        self.set_field(
            ProjectField::ActualWork.into(),
            actual_work.map(FieldValue::Duration),
        );
    }

/// Get the actual project work.
    ///
    /// Corresponds to Java: `public Duration getActualWork()`
    pub fn get_actual_work(&self) -> Option<Duration> {
        self.opt_duration(ProjectField::ActualWork.into())
    }

/// Get the project's "Work 2" property.
    ///
    /// Corresponds to Java: `public Number getWork2()`
    pub fn get_work2(&self) -> Option<f64> {
        self.opt_double(ProjectField::Work2.into())
    }

/// Set the project's "Work 2" property.
    ///
    /// Corresponds to Java: `public void setWork2(Number work2)`
    pub fn set_work2(&mut self, work2: Option<f64>) {
        self.set_field(ProjectField::Work2.into(), work2.map(FieldValue::Double));
    }

/// Get the project duration.
    ///
    /// Corresponds to Java: `public Duration getDuration()`
    pub fn get_duration(&self) -> Option<Duration> {
        self.opt_duration(ProjectField::Duration.into())
    }

/// Set the project duration.
    ///
    /// Corresponds to Java: `public void setDuration(Duration duration)`
    pub fn set_duration(&mut self, duration: Option<Duration>) {
        self.set_field(ProjectField::Duration.into(), duration.map(FieldValue::Duration));
    }

/// Get the baseline project duration.
    ///
    /// Corresponds to Java: `public Duration getBaselineDuration()`
    pub fn get_baseline_duration(&self) -> Option<Duration> {
        self.opt_duration(ProjectField::BaselineDuration.into())
    }

/// Set the baseline project duration.
    ///
    /// Corresponds to Java: `public void setBaselineDuration(Duration baselineDuration)`
    pub fn set_baseline_duration(&mut self, baseline_duration: Option<Duration>) {
        self.set_field(
            ProjectField::BaselineDuration.into(),
            baseline_duration.map(FieldValue::Duration),
        );
    }

/// Get the actual project duration.
    ///
    /// Corresponds to Java: `public Duration getActualDuration()`
    pub fn get_actual_duration(&self) -> Option<Duration> {
        self.opt_duration(ProjectField::ActualDuration.into())
    }

/// Set the actual project duration.
    ///
    /// Corresponds to Java: `public void setActualDuration(Duration actualDuration)`
    pub fn set_actual_duration(&mut self, actual_duration: Option<Duration>) {
        self.set_field(
            ProjectField::ActualDuration.into(),
            actual_duration.map(FieldValue::Duration),
        );
    }

/// Get the project percent complete.
    ///
    /// Corresponds to Java: `public Number getPercentageComplete()`
    pub fn get_percentage_complete(&self) -> Option<f64> {
        self.opt_double(ProjectField::PercentageComplete.into())
    }

/// Set the project percent complete.
    ///
    /// Corresponds to Java: `public void setPercentageComplete(Number percentComplete)`
    pub fn set_percentage_complete(&mut self, percent_complete: Option<f64>) {
        self.set_field(
            ProjectField::PercentageComplete.into(),
            percent_complete.map(FieldValue::Double),
        );
    }

    //
// Baseline / actual start and finish dates, variance
    //

/// Set the baseline project start date.
    ///
    /// Corresponds to Java: `public void setBaselineStart(LocalDateTime baselineStartDate)`
    pub fn set_baseline_start(&mut self, baseline_start_date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::BaselineStart.into(),
            baseline_start_date.map(FieldValue::DateTime),
        );
    }

/// Get the baseline project start date.
    ///
    /// Corresponds to Java: `public LocalDateTime getBaselineStart()`
    pub fn get_baseline_start(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::BaselineStart.into())
    }

/// Set the baseline project finish date.
    ///
    /// Corresponds to Java: `public void setBaselineFinish(LocalDateTime baselineFinishDate)`
    pub fn set_baseline_finish(&mut self, baseline_finish_date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::BaselineFinish.into(),
            baseline_finish_date.map(FieldValue::DateTime),
        );
    }

/// Get the baseline project finish date.
    ///
    /// Corresponds to Java: `public LocalDateTime getBaselineFinish()`
    pub fn get_baseline_finish(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::BaselineFinish.into())
    }

/// Set the actual project start date.
    ///
    /// Corresponds to Java: `public void setActualStart(LocalDateTime actualStartDate)`
    pub fn set_actual_start(&mut self, actual_start_date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::ActualStart.into(),
            actual_start_date.map(FieldValue::DateTime),
        );
    }

/// Get the actual project start date.
    ///
    /// Corresponds to Java: `public LocalDateTime getActualStart()`
    pub fn get_actual_start(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::ActualStart.into())
    }

/// Set the actual project finish date.
    ///
    /// Corresponds to Java: `public void setActualFinish(LocalDateTime actualFinishDate)`
    pub fn set_actual_finish(&mut self, actual_finish_date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::ActualFinish.into(),
            actual_finish_date.map(FieldValue::DateTime),
        );
    }

/// Get the actual project finish date.
    ///
    /// Corresponds to Java: `public LocalDateTime getActualFinish()`
    pub fn get_actual_finish(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::ActualFinish.into())
    }

/// Get the start date variance.
    ///
    /// Corresponds to Java: `public Duration getStartVariance()`
    pub fn get_start_variance(&self) -> Option<Duration> {
        self.opt_duration(ProjectField::StartVariance.into())
    }

/// Set the start date variance.
    ///
    /// Corresponds to Java: `public void setStartVariance(Duration startVariance)`
    pub fn set_start_variance(&mut self, start_variance: Option<Duration>) {
        self.set_field(
            ProjectField::StartVariance.into(),
            start_variance.map(FieldValue::Duration),
        );
    }

/// Get the finish date variance.
    ///
    /// Corresponds to Java: `public Duration getFinishVariance()`
    pub fn get_finish_variance(&self) -> Option<Duration> {
        self.opt_duration(ProjectField::FinishVariance.into())
    }

/// Set the finish date variance.
    ///
    /// Corresponds to Java: `public void setFinishVariance(Duration finishVariance)`
    pub fn set_finish_variance(&mut self, finish_variance: Option<Duration>) {
        self.set_field(
            ProjectField::FinishVariance.into(),
            finish_variance.map(FieldValue::Duration),
        );
    }

    //
// Document information
    //

/// Get the project subject text.
    ///
    /// Corresponds to Java: `public String getSubject()`
    pub fn get_subject(&self) -> Option<String> {
        self.opt_string(ProjectField::Subject.into())
    }

/// Set the project subject text.
    ///
    /// Corresponds to Java: `public void setSubject(String subject)`
    pub fn set_subject(&mut self, subject: Option<String>) {
        self.set_field(ProjectField::Subject.into(), subject.map(FieldValue::String));
    }

/// Get the project author text.
    ///
    /// Corresponds to Java: `public String getAuthor()`
    pub fn get_author(&self) -> Option<String> {
        self.opt_string(ProjectField::Author.into())
    }

/// Set the project author text.
    ///
    /// Corresponds to Java: `public void setAuthor(String author)`
    pub fn set_author(&mut self, author: Option<String>) {
        self.set_field(ProjectField::Author.into(), author.map(FieldValue::String));
    }

/// Get the project keywords text.
    ///
    /// Corresponds to Java: `public String getKeywords()`
    pub fn get_keywords(&self) -> Option<String> {
        self.opt_string(ProjectField::Keywords.into())
    }

/// Set the project keywords text.
    ///
    /// Corresponds to Java: `public void setKeywords(String keywords)`
    pub fn set_keywords(&mut self, keywords: Option<String>) {
        self.set_field(ProjectField::Keywords.into(), keywords.map(FieldValue::String));
    }

    //
// Currency
    //

/// Set the currency symbol.
    ///
    /// Corresponds to Java: `public void setCurrencySymbol(String symbol)`
    pub fn set_currency_symbol(&mut self, symbol: Option<String>) {
        self.set_field(
            ProjectField::CurrencySymbol.into(),
            symbol.map(FieldValue::String),
        );
    }

/// Get the currency symbol; uses the Java default `"$"` when unset.
    ///
    /// Corresponds to Java: `public String getCurrencySymbol()`
    pub fn get_currency_symbol(&self) -> String {
        self.string_value(ProjectField::CurrencySymbol.into(), DEFAULT_CURRENCY_SYMBOL)
    }

/// Set the currency symbol position.
    ///
    /// Corresponds to Java: `public void setSymbolPosition(CurrencySymbolPosition value)`
    pub fn set_symbol_position(&mut self, value: Option<CurrencySymbolPosition>) {
        self.set_field(
            ProjectField::CurrencySymbolPosition.into(),
            value.map(|value| FieldValue::Integer(value.value())),
        );
    }

/// Get the currency symbol position; uses the Java default `BEFORE` when unset.
    ///
    /// Corresponds to Java: `public CurrencySymbolPosition getSymbolPosition()`
    pub fn get_symbol_position(&self) -> CurrencySymbolPosition {
        self.opt_int(ProjectField::CurrencySymbolPosition.into())
            .map_or(CurrencySymbolPosition::Before, CurrencySymbolPosition::get_instance)
    }

/// Set the number of currency decimal places.
    ///
    /// Corresponds to Java: `public void setCurrencyDigits(Integer currDigs)`
    pub fn set_currency_digits(&mut self, curr_digs: Option<i32>) {
        self.set_field(
            ProjectField::CurrencyDigits.into(),
            curr_digs.map(FieldValue::Integer),
        );
    }

/// Get the number of currency decimal places; uses the Java default 2 when unset.
    ///
    /// Corresponds to Java: `public Integer getCurrencyDigits()`
    pub fn get_currency_digits(&self) -> i32 {
        self.int_value(ProjectField::CurrencyDigits.into(), DEFAULT_CURRENCY_DIGITS)
    }

/// Set the thousands separator.
    ///
    /// Corresponds to Java: `public void setThousandsSeparator(char sep)`
    pub fn set_thousands_separator(&mut self, sep: char) {
        self.set_char(ProjectField::ThousandsSeparator, sep);
    }

/// Get the thousands separator; uses the Java default `','` when unset.
    ///
    /// Corresponds to Java: `public char getThousandsSeparator()`
    pub fn get_thousands_separator(&self) -> char {
        self.char_value(
            ProjectField::ThousandsSeparator.into(),
            DEFAULT_THOUSANDS_SEPARATOR,
        )
    }

/// Set the decimal separator.
    ///
    /// Corresponds to Java: `public void setDecimalSeparator(char decSep)`
    pub fn set_decimal_separator(&mut self, dec_sep: char) {
        self.set_char(ProjectField::DecimalSeparator, dec_sep);
    }

/// Get the decimal separator; uses the Java default `'.'` when unset.
    ///
    /// Corresponds to Java: `public char getDecimalSeparator()`
    pub fn get_decimal_separator(&self) -> char {
        self.char_value(ProjectField::DecimalSeparator.into(), DEFAULT_DECIMAL_SEPARATOR)
    }

/// Get the currency code.
    ///
    /// Corresponds to Java: `public String getCurrencyCode()`
    pub fn get_currency_code(&self) -> Option<String> {
        self.opt_string(ProjectField::CurrencyCode.into())
    }

/// Set the currency code.
    ///
    /// Corresponds to Java: `public void setCurrencyCode(String currencyCode)`
    pub fn set_currency_code(&mut self, currency_code: Option<String>) {
        self.set_field(
            ProjectField::CurrencyCode.into(),
            currency_code.map(FieldValue::String),
        );
    }

    //
// Other project flags
    //

/// Get the "project was edited externally" flag.
    ///
    /// Corresponds to Java: `public boolean getProjectExternallyEdited()`
    pub fn get_project_externally_edited(&self) -> bool {
        self.bool_value(ProjectField::ProjectExternallyEdited.into())
    }

/// Set the "project was edited externally" flag.
    ///
    /// Corresponds to Java: `public void setProjectExternallyEdited(boolean projectExternallyEdited)`
    pub fn set_project_externally_edited(&mut self, project_externally_edited: bool) {
        self.set_boolean(ProjectField::ProjectExternallyEdited, project_externally_edited);
    }

/// Get the category text.
    ///
    /// Corresponds to Java: `public String getCategory()`
    pub fn get_category(&self) -> Option<String> {
        self.opt_string(ProjectField::Category.into())
    }

/// Set the category text.
    ///
    /// Corresponds to Java: `public void setCategory(String category)`
    pub fn set_category(&mut self, category: Option<String>) {
        self.set_field(ProjectField::Category.into(), category.map(FieldValue::String));
    }

    //
// Time unit defaults
    //

/// Get the number of days per month (this field is one of Java's "always calculated fields"; the value comes from the project context).
    ///
    /// Corresponds to Java: `@Override public Integer getDaysPerMonth()`
    pub fn get_days_per_month(&self) -> Option<i32> {
        self.opt_int(ProjectField::DaysPerMonth.into())
    }

/// Set the number of days per month.
    ///
    /// Corresponds to Java: `public void setDaysPerMonth(Integer daysPerMonth)`
    pub fn set_days_per_month(&mut self, days_per_month: Option<i32>) {
        self.set_field(
            ProjectField::DaysPerMonth.into(),
            days_per_month.map(FieldValue::Integer),
        );
    }

/// Get the number of minutes per day (this field is one of Java's "always calculated fields"; the value comes from the project context).
    ///
    /// Corresponds to Java: `@Override public Integer getMinutesPerDay()`
    pub fn get_minutes_per_day(&self) -> Option<i32> {
        self.opt_int(ProjectField::MinutesPerDay.into())
    }

/// Set the number of minutes per day.
    ///
    /// Corresponds to Java: `public void setMinutesPerDay(Integer minutesPerDay)`
    pub fn set_minutes_per_day(&mut self, minutes_per_day: Option<i32>) {
        self.set_field(
            ProjectField::MinutesPerDay.into(),
            minutes_per_day.map(FieldValue::Integer),
        );
    }

/// Get the number of minutes per week (this field is one of Java's "always calculated fields"; the value comes from the project context).
    ///
    /// Corresponds to Java: `@Override public Integer getMinutesPerWeek()`
    pub fn get_minutes_per_week(&self) -> Option<i32> {
        self.opt_int(ProjectField::MinutesPerWeek.into())
    }

/// Set the number of minutes per week.
    ///
    /// Corresponds to Java: `public void setMinutesPerWeek(Integer minutesPerWeek)`
    pub fn set_minutes_per_week(&mut self, minutes_per_week: Option<i32>) {
        self.set_field(
            ProjectField::MinutesPerWeek.into(),
            minutes_per_week.map(FieldValue::Integer),
        );
    }

/// Get the number of minutes per month (this field is one of Java's "always calculated fields"; the value comes from the project context).
    ///
    /// Corresponds to Java: `@Override public Integer getMinutesPerMonth()`
    pub fn get_minutes_per_month(&self) -> Option<i32> {
        self.opt_int(ProjectField::MinutesPerMonth.into())
    }

/// Set the number of minutes per month.
    ///
    /// Corresponds to Java: `public void setMinutesPerMonth(Integer minutesPerMonth)`
    pub fn set_minutes_per_month(&mut self, minutes_per_month: Option<i32>) {
        self.set_field(
            ProjectField::MinutesPerMonth.into(),
            minutes_per_month.map(FieldValue::Integer),
        );
    }

/// Get the number of minutes per year (this field is one of Java's "always calculated fields"; the value comes from the project context).
    ///
    /// Corresponds to Java: `@Override public Integer getMinutesPerYear()`
    pub fn get_minutes_per_year(&self) -> Option<i32> {
        self.opt_int(ProjectField::MinutesPerYear.into())
    }

/// Set the number of minutes per year.
    ///
    /// Corresponds to Java: `public void setMinutesPerYear(Integer minutesPerYear)`
    pub fn set_minutes_per_year(&mut self, minutes_per_year: Option<i32>) {
        self.set_field(
            ProjectField::MinutesPerYear.into(),
            minutes_per_year.map(FieldValue::Integer),
        );
    }

/// Get the "fiscal year start" flag.
    ///
    /// Corresponds to Java: `public boolean getFiscalYearStart()`
    pub fn get_fiscal_year_start(&self) -> bool {
        self.bool_value(ProjectField::FiscalYearStart.into())
    }

/// Set the "fiscal year start" flag.
    ///
    /// Corresponds to Java: `public void setFiscalYearStart(boolean fiscalYearStart)`
    pub fn set_fiscal_year_start(&mut self, fiscal_year_start: bool) {
        self.set_boolean(ProjectField::FiscalYearStart, fiscal_year_start);
    }

/// Get the default task earned value method; uses the Java default `PERCENT_COMPLETE` when unset.
    ///
    /// Corresponds to Java: `public EarnedValueMethod getDefaultTaskEarnedValueMethod()`
    pub fn get_default_task_earned_value_method(&self) -> EarnedValueMethod {
        self.opt_int(ProjectField::DefaultTaskEarnedValueMethod.into())
            .map_or(EarnedValueMethod::PercentComplete, EarnedValueMethod::get_instance)
    }

/// Set the default task earned value method.
    ///
    /// Corresponds to Java: `public void setDefaultTaskEarnedValueMethod(EarnedValueMethod defaultTaskEarnedValueMethod)`
    pub fn set_default_task_earned_value_method(&mut self, value: Option<EarnedValueMethod>) {
        self.set_field(
            ProjectField::DefaultTaskEarnedValueMethod.into(),
            value.map(|value| FieldValue::Integer(value.value())),
        );
    }

/// Get the "remove file properties" flag.
    ///
    /// Corresponds to Java: `public boolean getRemoveFileProperties()`
    pub fn get_remove_file_properties(&self) -> bool {
        self.bool_value(ProjectField::RemoveFileProperties.into())
    }

/// Set the "remove file properties" flag.
    ///
    /// Corresponds to Java: `public void setRemoveFileProperties(boolean removeFileProperties)`
    pub fn set_remove_file_properties(&mut self, remove_file_properties: bool) {
        self.set_boolean(ProjectField::RemoveFileProperties, remove_file_properties);
    }

    //
// Progress update / scheduling flags
    //

/// Get the "move completed ends back" flag.
    ///
    /// Corresponds to Java: `public boolean getMoveCompletedEndsBack()`
    pub fn get_move_completed_ends_back(&self) -> bool {
        self.bool_value(ProjectField::MoveCompletedEndsBack.into())
    }

/// Set the "move completed ends back" flag.
    ///
    /// Corresponds to Java: `public void setMoveCompletedEndsBack(boolean moveCompletedEndsBack)`
    pub fn set_move_completed_ends_back(&mut self, move_completed_ends_back: bool) {
        self.set_boolean(ProjectField::MoveCompletedEndsBack, move_completed_ends_back);
    }

/// Get the "new tasks are estimated" flag.
    ///
    /// Corresponds to Java: `public boolean getNewTasksEstimated()`
    pub fn get_new_tasks_estimated(&self) -> bool {
        self.bool_value(ProjectField::NewTasksEstimated.into())
    }

/// Set the "new tasks are estimated" flag.
    ///
    /// Corresponds to Java: `public void setNewTasksEstimated(boolean newTasksEstimated)`
    pub fn set_new_tasks_estimated(&mut self, new_tasks_estimated: bool) {
        self.set_boolean(ProjectField::NewTasksEstimated, new_tasks_estimated);
    }

/// Get the "split actual cost" flag.
    ///
    /// Corresponds to Java: `public boolean getSpreadActualCost()`
    pub fn get_spread_actual_cost(&self) -> bool {
        self.bool_value(ProjectField::SpreadActualCost.into())
    }

/// Set the "split actual cost" flag.
    ///
    /// Corresponds to Java: `public void setSpreadActualCost(boolean spreadActualCost)`
    pub fn set_spread_actual_cost(&mut self, spread_actual_cost: bool) {
        self.set_boolean(ProjectField::SpreadActualCost, spread_actual_cost);
    }

/// Get the "multiple critical paths" flag.
    ///
    /// Corresponds to Java: `public boolean getMultipleCriticalPaths()`
    pub fn get_multiple_critical_paths(&self) -> bool {
        self.bool_value(ProjectField::MultipleCriticalPaths.into())
    }

/// Set the "multiple critical paths" flag.
    ///
    /// Corresponds to Java: `public void setMultipleCriticalPaths(boolean multipleCriticalPaths)`
    pub fn set_multiple_critical_paths(&mut self, multiple_critical_paths: bool) {
        self.set_boolean(ProjectField::MultipleCriticalPaths, multiple_critical_paths);
    }

/// Get the "automatically add new resources and tasks" flag.
    ///
    /// Corresponds to Java: `public boolean getAutoAddNewResourcesAndTasks()`
    pub fn get_auto_add_new_resources_and_tasks(&self) -> bool {
        self.bool_value(ProjectField::AutoAddNewResourcesAndTasks.into())
    }

/// Set the "automatically add new resources and tasks" flag.
    ///
    /// Corresponds to Java: `public void setAutoAddNewResourcesAndTasks(boolean autoAddNewResourcesAndTasks)`
    pub fn set_auto_add_new_resources_and_tasks(&mut self, auto_add_new_resources_and_tasks: bool) {
        self.set_boolean(
            ProjectField::AutoAddNewResourcesAndTasks,
            auto_add_new_resources_and_tasks,
        );
    }

/// Get the last saved time.
    ///
    /// Corresponds to Java: `public LocalDateTime getLastSaved()`
    pub fn get_last_saved(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::LastSaved.into())
    }

/// Set the last saved time.
    ///
    /// Corresponds to Java: `public void setLastSaved(LocalDateTime lastSaved)`
    pub fn set_last_saved(&mut self, last_saved: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::LastSaved.into(),
            last_saved.map(FieldValue::DateTime),
        );
    }

/// Get the status date.
    ///
    /// Corresponds to Java: `public LocalDateTime getStatusDate()`
    pub fn get_status_date(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::StatusDate.into())
    }

/// Set the status date.
    ///
    /// Corresponds to Java: `public void setStatusDate(LocalDateTime statusDate)`
    pub fn set_status_date(&mut self, status_date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::StatusDate.into(),
            status_date.map(FieldValue::DateTime),
        );
    }

/// Get the last scheduled date.
    ///
    /// Corresponds to Java: `public LocalDateTime getLastScheduledDate()`
    pub fn get_last_scheduled_date(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::LastScheduledDate.into())
    }

/// Set the last scheduled date.
    ///
    /// Corresponds to Java: `public void setLastScheduledDate(LocalDateTime value)`
    pub fn set_last_scheduled_date(&mut self, value: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::LastScheduledDate.into(),
            value.map(FieldValue::DateTime),
        );
    }

/// Set the "activity percent complete based on activity steps" flag.
    ///
    /// Corresponds to Java: `public void setActivityPercentCompleteBasedOnActivitySteps(boolean value)`
    pub fn set_activity_percent_complete_based_on_activity_steps(&mut self, value: bool) {
        self.set_boolean(
            ProjectField::ActivityPercentCompleteBasedOnActivitySteps,
            value,
        );
    }

/// Get the "activity percent complete based on activity steps" flag (the Java method name is `getStepCompleteFlag`).
    ///
    /// Corresponds to Java: `public boolean getStepCompleteFlag()`
    pub fn get_step_complete_flag(&self) -> bool {
        self.bool_value(ProjectField::ActivityPercentCompleteBasedOnActivitySteps.into())
    }

/// Get the "move remaining starts back" flag.
    ///
    /// Corresponds to Java: `public boolean getMoveRemainingStartsBack()`
    pub fn get_move_remaining_starts_back(&self) -> bool {
        self.bool_value(ProjectField::MoveRemainingStartsBack.into())
    }

/// Set the "move remaining starts back" flag.
    ///
    /// Corresponds to Java: `public void setMoveRemainingStartsBack(boolean moveRemainingStartsBack)`
    pub fn set_move_remaining_starts_back(&mut self, move_remaining_starts_back: bool) {
        self.set_boolean(ProjectField::MoveRemainingStartsBack, move_remaining_starts_back);
    }

/// Get the "auto link" flag.
    ///
    /// Corresponds to Java: `public boolean getAutolink()`
    pub fn get_autolink(&self) -> bool {
        self.bool_value(ProjectField::AutoLink.into())
    }

/// Set the "auto link" flag.
    ///
    /// Corresponds to Java: `public void setAutolink(boolean autolink)`
    pub fn set_autolink(&mut self, autolink: bool) {
        self.set_boolean(ProjectField::AutoLink, autolink);
    }

/// Get the "Microsoft Project Server URL" flag.
    ///
    /// Corresponds to Java: `public boolean getMicrosoftProjectServerURL()`
    pub fn get_microsoft_project_server_url(&self) -> bool {
        self.bool_value(ProjectField::MicrosoftProjectServerUrl.into())
    }

/// Set the "Microsoft Project Server URL" flag.
    ///
    /// Corresponds to Java: `public void setMicrosoftProjectServerURL(boolean microsoftProjectServerURL)`
    pub fn set_microsoft_project_server_url(&mut self, microsoft_project_server_url: bool) {
        self.set_boolean(
            ProjectField::MicrosoftProjectServerUrl,
            microsoft_project_server_url,
        );
    }

/// Get the "honor constraints" flag.
    ///
    /// Corresponds to Java: `public boolean getHonorConstraints()`
    pub fn get_honor_constraints(&self) -> bool {
        self.bool_value(ProjectField::HonorConstraints.into())
    }

/// Set the "honor constraints" flag.
    ///
    /// Corresponds to Java: `public void setHonorConstraints(boolean honorConstraints)`
    pub fn set_honor_constraints(&mut self, honor_constraints: bool) {
        self.set_boolean(ProjectField::HonorConstraints, honor_constraints);
    }

/// Get the "managed project" flag.
    ///
    /// Corresponds to Java: `public boolean getAdminProject()`
    pub fn get_admin_project(&self) -> bool {
        self.bool_value(ProjectField::AdminProject.into())
    }

/// Set the "managed project" flag.
    ///
    /// Corresponds to Java: `public void setAdminProject(boolean adminProject)`
    pub fn set_admin_project(&mut self, admin_project: bool) {
        self.set_boolean(ProjectField::AdminProject, admin_project);
    }

/// Get the "inserted projects are treated as summary tasks" flag.
    ///
    /// Corresponds to Java: `public boolean getInsertedProjectsLikeSummary()`
    pub fn get_inserted_projects_like_summary(&self) -> bool {
        self.bool_value(ProjectField::InsertedProjectsLikeSummary.into())
    }

/// Set the "inserted projects are treated as summary tasks" flag.
    ///
    /// Corresponds to Java: `public void setInsertedProjectsLikeSummary(boolean insertedProjectsLikeSummary)`
    pub fn set_inserted_projects_like_summary(&mut self, inserted_projects_like_summary: bool) {
        self.set_boolean(
            ProjectField::InsertedProjectsLikeSummary,
            inserted_projects_like_summary,
        );
    }

/// Get the project name.
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> Option<String> {
        self.opt_string(ProjectField::Name.into())
    }

/// Set the project name.
    ///
    /// Corresponds to Java: `public void setName(String name)`
    pub fn set_name(&mut self, name: Option<String>) {
        self.set_field(ProjectField::Name.into(), name.map(FieldValue::String));
    }

/// Get the "split percent complete" flag.
    ///
    /// Corresponds to Java: `public boolean getSpreadPercentComplete()`
    pub fn get_spread_percent_complete(&self) -> bool {
        self.bool_value(ProjectField::SpreadPercentComplete.into())
    }

/// Set the "split percent complete" flag.
    ///
    /// Corresponds to Java: `public void setSpreadPercentComplete(boolean spreadPercentComplete)`
    pub fn set_spread_percent_complete(&mut self, spread_percent_complete: bool) {
        self.set_boolean(ProjectField::SpreadPercentComplete, spread_percent_complete);
    }

/// Get the "move completed ends forward" flag.
    ///
    /// Corresponds to Java: `public boolean getMoveCompletedEndsForward()`
    pub fn get_move_completed_ends_forward(&self) -> bool {
        self.bool_value(ProjectField::MoveCompletedEndsForward.into())
    }

/// Set the "move completed ends forward" flag.
    ///
    /// Corresponds to Java: `public void setMoveCompletedEndsForward(boolean moveCompletedEndsForward)`
    pub fn set_move_completed_ends_forward(&mut self, move_completed_ends_forward: bool) {
        self.set_boolean(
            ProjectField::MoveCompletedEndsForward,
            move_completed_ends_forward,
        );
    }

/// Get the "editable actual cost" flag.
    ///
    /// Corresponds to Java: `public boolean getEditableActualCosts()`
    pub fn get_editable_actual_costs(&self) -> bool {
        self.bool_value(ProjectField::EditableActualCosts.into())
    }

/// Set the "editable actual cost" flag.
    ///
    /// Corresponds to Java: `public void setEditableActualCosts(boolean editableActualCosts)`
    pub fn set_editable_actual_costs(&mut self, editable_actual_costs: bool) {
        self.set_boolean(ProjectField::EditableActualCosts, editable_actual_costs);
    }

    //
// Identity
    //

/// Get the project unique ID.
    ///
    /// Corresponds to Java: `public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.opt_int(ProjectField::UniqueId.into())
    }

/// Set the project unique ID.
    ///
    /// Corresponds to Java: `public void setUniqueID(Integer uniqueID)`
    pub fn set_unique_id(&mut self, unique_id: Option<i32>) {
        self.set_field(ProjectField::UniqueId.into(), unique_id.map(FieldValue::Integer));
    }

/// Get the project GUID.
    ///
    /// Corresponds to Java: `public UUID getGUID()`
    ///
/// `FieldValue` has no UUID variant, so it is stored as a plain string (see the file header notes); returns `None` when it cannot be parsed.
    pub fn get_guid(&self) -> Option<Uuid> {
        self.opt_string(ProjectField::Guid.into())
            .and_then(|value| Uuid::parse_str(&value).ok())
    }

/// Set the project GUID.
    ///
    /// Corresponds to Java: `public void setGUID(UUID guid)`
    pub fn set_guid(&mut self, guid: Option<Uuid>) {
        self.set_field(
            ProjectField::Guid.into(),
            guid.map(|guid| FieldValue::String(guid.to_string())),
        );
    }

/// Get the project revision number.
    ///
    /// Corresponds to Java: `public Integer getRevision()`
    pub fn get_revision(&self) -> Option<i32> {
        self.opt_int(ProjectField::Revision.into())
    }

/// Set the project revision number.
    ///
    /// Corresponds to Java: `public void setRevision(Integer revision)`
    pub fn set_revision(&mut self, revision: Option<i32>) {
        self.set_field(ProjectField::Revision.into(), revision.map(FieldValue::Integer));
    }

/// Get the "new tasks are effort driven" flag.
    ///
    /// Corresponds to Java: `public boolean getNewTasksEffortDriven()`
    pub fn get_new_tasks_effort_driven(&self) -> bool {
        self.bool_value(ProjectField::NewTasksEffortDriven.into())
    }

/// Set the "new tasks are effort driven" flag.
    ///
    /// Corresponds to Java: `public void setNewTasksEffortDriven(boolean newTasksEffortDriven)`
    pub fn set_new_tasks_effort_driven(&mut self, new_tasks_effort_driven: bool) {
        self.set_boolean(ProjectField::NewTasksEffortDriven, new_tasks_effort_driven);
    }

/// Get the "move remaining starts forward" flag.
    ///
    /// Corresponds to Java: `public boolean getMoveRemainingStartsForward()`
    pub fn get_move_remaining_starts_forward(&self) -> bool {
        self.bool_value(ProjectField::MoveRemainingStartsForward.into())
    }

/// Set the "move remaining starts forward" flag.
    ///
    /// Corresponds to Java: `public void setMoveRemainingStartsForward(boolean moveRemainingStartsForward)`
    pub fn set_move_remaining_starts_forward(&mut self, move_remaining_starts_forward: bool) {
        self.set_boolean(
            ProjectField::MoveRemainingStartsForward,
            move_remaining_starts_forward,
        );
    }

/// Get the "actuals synchronized" flag.
    ///
    /// Corresponds to Java: `public boolean getActualsInSync()`
    pub fn get_actuals_in_sync(&self) -> bool {
        self.bool_value(ProjectField::ActualsInSync.into())
    }

/// Set the "actuals synchronized" flag.
    ///
    /// Corresponds to Java: `public void setActualsInSync(boolean actualsInSync)`
    pub fn set_actuals_in_sync(&mut self, actuals_in_sync: bool) {
        self.set_boolean(ProjectField::ActualsInSync, actuals_in_sync);
    }

/// Get the default task type; uses the Java default `FIXED_UNITS` when unset.
    ///
    /// Corresponds to Java: `public TaskType getDefaultTaskType()`
    pub fn get_default_task_type(&self) -> TaskType {
        self.opt_int(ProjectField::DefaultTaskType.into())
            .and_then(TaskType::get_instance)
            .unwrap_or(TaskType::FixedUnits)
    }

/// Set the default task type.
    ///
    /// Corresponds to Java: `public void setDefaultTaskType(TaskType defaultTaskType)`
    pub fn set_default_task_type(&mut self, default_task_type: Option<TaskType>) {
        self.set_field(
            ProjectField::DefaultTaskType.into(),
            default_task_type.map(|value| FieldValue::Integer(ordinal_of(&TaskType::ALL, value))),
        );
    }

/// Get the earned value method.
    ///
    /// Corresponds to Java: `public EarnedValueMethod getEarnedValueMethod()`
    pub fn get_earned_value_method(&self) -> Option<EarnedValueMethod> {
        self.opt_int(ProjectField::EarnedValueMethod.into())
            .map(EarnedValueMethod::get_instance)
    }

/// Set the earned value method.
    ///
    /// Corresponds to Java: `public void setEarnedValueMethod(EarnedValueMethod earnedValueMethod)`
    pub fn set_earned_value_method(&mut self, earned_value_method: Option<EarnedValueMethod>) {
        self.set_field(
            ProjectField::EarnedValueMethod.into(),
            earned_value_method.map(|value| FieldValue::Integer(value.value())),
        );
    }

/// Get the project creation date.
    ///
    /// Corresponds to Java: `public LocalDateTime getCreationDate()`
    pub fn get_creation_date(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::CreationDate.into())
    }

/// Set the project creation date.
    ///
    /// Corresponds to Java: `public void setCreationDate(LocalDateTime creationDate)`
    pub fn set_creation_date(&mut self, creation_date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::CreationDate.into(),
            creation_date.map(FieldValue::DateTime),
        );
    }

/// Get the extended creation date.
    ///
    /// Corresponds to Java: `public LocalDateTime getExtendedCreationDate()`
    pub fn get_extended_creation_date(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::ExtendedCreationDate.into())
    }

/// Set the extended creation date.
    ///
    /// Corresponds to Java: `public void setExtendedCreationDate(LocalDateTime creationDate)`
    pub fn set_extended_creation_date(&mut self, creation_date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::ExtendedCreationDate.into(),
            creation_date.map(FieldValue::DateTime),
        );
    }

/// Get the default fixed cost accrual method; uses the Java default `END` when unset.
    ///
    /// Corresponds to Java: `public AccrueType getDefaultFixedCostAccrual()`
    pub fn get_default_fixed_cost_accrual(&self) -> AccrueType {
        self.opt_int(ProjectField::DefaultFixedCostAccrual.into())
            .and_then(AccrueType::get_instance)
            .unwrap_or(AccrueType::End)
    }

/// Set the default fixed cost accrual method.
    ///
    /// Corresponds to Java: `public void setDefaultFixedCostAccrual(AccrueType defaultFixedCostAccrual)`
    pub fn set_default_fixed_cost_accrual(&mut self, default_fixed_cost_accrual: Option<AccrueType>) {
        self.set_field(
            ProjectField::DefaultFixedCostAccrual.into(),
            default_fixed_cost_accrual.map(|value| FieldValue::Integer(value.value())),
        );
    }

/// Get the critical slack limit; uses the Java default "0 days" when unset.
    ///
    /// Corresponds to Java: `public Duration getCriticalSlackLimit()`
    pub fn get_critical_slack_limit(&self) -> Duration {
        self.opt_duration(ProjectField::CriticalSlackLimit.into())
            .unwrap_or_else(default_critical_slack_limit)
    }

/// Set the critical slack limit.
    ///
    /// Corresponds to Java: `public void setCriticalSlackLimit(Duration criticalSlackLimit)`
    pub fn set_critical_slack_limit(&mut self, critical_slack_limit: Option<Duration>) {
        self.set_field(
            ProjectField::CriticalSlackLimit.into(),
            critical_slack_limit.map(FieldValue::Duration),
        );
    }

/// Get the baseline number used for earned value calculations; uses the Java default 0 when unset.
    ///
    /// Corresponds to Java: `public Integer getBaselineForEarnedValue()`
    pub fn get_baseline_for_earned_value(&self) -> i32 {
        self.int_value(
            ProjectField::BaselineForEarnedValue.into(),
            DEFAULT_BASELINE_FOR_EARNED_VALUE,
        )
    }

/// Set the baseline number used for earned value calculations.
    ///
    /// Corresponds to Java: `public void setBaselineForEarnedValue(Integer baselineForEarnedValue)`
    pub fn set_baseline_for_earned_value(&mut self, baseline_for_earned_value: Option<i32>) {
        self.set_field(
            ProjectField::BaselineForEarnedValue.into(),
            baseline_for_earned_value.map(FieldValue::Integer),
        );
    }

/// Get the fiscal year start month (January = 1, December = 12); uses the Java default 1 when unset.
    ///
    /// Corresponds to Java: `public Integer getFiscalYearStartMonth()`
    pub fn get_fiscal_year_start_month(&self) -> i32 {
        self.int_value(
            ProjectField::FiscalYearStartMonth.into(),
            DEFAULT_FISCAL_YEAR_START_MONTH,
        )
    }

/// Set the fiscal year start month.
    ///
    /// Corresponds to Java: `public void setFiscalYearStartMonth(Integer fiscalYearStartMonth)`
    pub fn set_fiscal_year_start_month(&mut self, fiscal_year_start_month: Option<i32>) {
        self.set_field(
            ProjectField::FiscalYearStartMonth.into(),
            fiscal_year_start_month.map(FieldValue::Integer),
        );
    }

/// Get the "new task start dates default to the project start date" flag.
    ///
    /// Corresponds to Java: `public boolean getNewTaskStartIsProjectStart()`
    pub fn get_new_task_start_is_project_start(&self) -> bool {
        self.bool_value(ProjectField::NewTaskStartIsProjectStart.into())
    }

/// Set the "new task start dates default to the project start date" flag.
    ///
    /// Corresponds to Java: `public void setNewTaskStartIsProjectStart(boolean newTaskStartIsProjectStart)`
    pub fn set_new_task_start_is_project_start(&mut self, new_task_start_is_project_start: bool) {
        self.set_boolean(
            ProjectField::NewTaskStartIsProjectStart,
            new_task_start_is_project_start,
        );
    }

/// Get the "new tasks default to manually scheduled" flag.
    ///
    /// Corresponds to Java: `public boolean getNewTasksAreManual()`
    pub fn get_new_tasks_are_manual(&self) -> bool {
        self.bool_value(ProjectField::NewTasksAreManual.into())
    }

/// Set the "new tasks default to manually scheduled" flag.
    ///
    /// Corresponds to Java: `public void setNewTasksAreManual(boolean newTasksAreManual)`
    pub fn set_new_tasks_are_manual(&mut self, new_tasks_are_manual: bool) {
        self.set_boolean(ProjectField::NewTasksAreManual, new_tasks_are_manual);
    }

/// Get the week start day; uses the Java default `MONDAY` when unset.
    ///
    /// Corresponds to Java: `public DayOfWeek getWeekStartDay()`
    ///
/// Rust's [`Weekday`] matches Java's `DayOfWeek` (Monday … Sunday); the field stores Java's
/// 1 (Monday) … 7 (Sunday).
    pub fn get_week_start_day(&self) -> Weekday {
        self.opt_int(ProjectField::WeekStartDay.into())
            .and_then(weekday_from_java_day)
            .unwrap_or(Weekday::Mon)
    }

/// Set the week start day.
    ///
    /// Corresponds to Java: `public void setWeekStartDay(DayOfWeek weekStartDay)`
    pub fn set_week_start_day(&mut self, week_start_day: Option<Weekday>) {
        self.set_field(
            ProjectField::WeekStartDay.into(),
            week_start_day.map(|value| FieldValue::Integer(java_day_of_week(value))),
        );
    }

    //
// Custom document properties / hyperlinks
    //

/// Set the custom document properties.
    ///
    /// Corresponds to Java: `public void setCustomProperties(Map<String, Object> customProperties)`
    ///
/// `FieldValue` cannot hold a `Map`, so this crate stores it in a dedicated field (see the file header notes).
    pub fn set_custom_properties(&mut self, custom_properties: HashMap<String, FieldValue>) {
        self.custom_properties = custom_properties;
    }

/// Get the custom document properties.
    ///
    /// Corresponds to Java: `public Map<String, Object> getCustomProperties()`
    pub fn get_custom_properties(&self) -> &HashMap<String, FieldValue> {
        &self.custom_properties
    }

/// Get a mutable reference to the custom document properties (Rust-specific: Java returns a mutable Map directly).
    pub fn get_custom_properties_mut(&mut self) -> &mut HashMap<String, FieldValue> {
        &mut self.custom_properties
    }

/// Set the hyperlink base.
    ///
    /// Corresponds to Java: `public void setHyperlinkBase(String hyperlinkBase)`
    pub fn set_hyperlink_base(&mut self, hyperlink_base: Option<String>) {
        self.set_field(
            ProjectField::HyperlinkBase.into(),
            hyperlink_base.map(FieldValue::String),
        );
    }

/// Get the hyperlink base.
    ///
    /// Corresponds to Java: `public String getHyperlinkBase()`
    pub fn get_hyperlink_base(&self) -> Option<String> {
        self.opt_string(ProjectField::HyperlinkBase.into())
    }

/// Get the "show project summary task" flag.
    ///
    /// Corresponds to Java: `public boolean getShowProjectSummaryTask()`
    pub fn get_show_project_summary_task(&self) -> bool {
        self.bool_value(ProjectField::ShowProjectSummaryTask.into())
    }

/// Set the "show project summary task" flag.
    ///
    /// Corresponds to Java: `public void setShowProjectSummaryTask(boolean value)`
    pub fn set_show_project_summary_task(&mut self, value: bool) {
        self.set_boolean(ProjectField::ShowProjectSummaryTask, value);
    }

    //
// Baseline dates
    //

/// Get the baseline date.
    ///
    /// Corresponds to Java: `public LocalDateTime getBaselineDate()`
    pub fn get_baseline_date(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::BaselineDate.into())
    }

/// Set the baseline date.
    ///
    /// Corresponds to Java: `public void setBaselineDate(LocalDateTime value)`
    pub fn set_baseline_date(&mut self, value: Option<NaiveDateTime>) {
        self.set_field(ProjectField::BaselineDate.into(), value.map(FieldValue::DateTime));
    }

/// Get the baseline date with the given number (number 1-10).
    ///
    /// Corresponds to Java: `public LocalDateTime getBaselineDate(int baselineNumber)`
    ///
/// Java throws `IllegalArgumentException` for an out-of-bounds number; the Rust side returns `None` (without panicking),
/// so it is given the `_for_index` suffix to distinguish it from the no-argument overload.
    pub fn get_baseline_date_for_index(&self, baseline_number: i32) -> Option<NaiveDateTime> {
        let field = select_field(&BASELINE_DATES, baseline_number)?;
        self.opt_date_time(field.into())
    }

/// Set the baseline date with the given number (number 1-10).
    ///
    /// Corresponds to Java: `public void setBaselineDate(int baselineNumber, LocalDateTime value)`
    ///
/// Java throws `IllegalArgumentException` for an out-of-bounds number; the Rust side degrades gracefully by not writing.
    pub fn set_baseline_date_for_index(&mut self, baseline_number: i32, value: Option<NaiveDateTime>) {
        let Some(field) = select_field(&BASELINE_DATES, baseline_number) else {
            return;
        };
        self.set_field(field.into(), value.map(FieldValue::DateTime));
    }

    //
// Document summary information
    //

/// Get the template property.
    ///
    /// Corresponds to Java: `public String getTemplate()`
    pub fn get_template(&self) -> Option<String> {
        self.opt_string(ProjectField::Template.into())
    }

/// Set the template property.
    ///
    /// Corresponds to Java: `public void setTemplate(String template)`
    pub fn set_template(&mut self, template: Option<String>) {
        self.set_field(ProjectField::Template.into(), template.map(FieldValue::String));
    }

/// Get the "last modified by" property.
    ///
    /// Corresponds to Java: `public String getLastAuthor()`
    pub fn get_last_author(&self) -> Option<String> {
        self.opt_string(ProjectField::LastAuthor.into())
    }

/// Set the "last modified by" property.
    ///
    /// Corresponds to Java: `public void setLastAuthor(String projectUser)`
    pub fn set_last_author(&mut self, project_user: Option<String>) {
        self.set_field(
            ProjectField::LastAuthor.into(),
            project_user.map(FieldValue::String),
        );
    }

/// Get the last printed time.
    ///
    /// Corresponds to Java: `public LocalDateTime getLastPrinted()`
    pub fn get_last_printed(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::Lastprinted.into())
    }

/// Set the last printed time.
    ///
    /// Corresponds to Java: `public void setLastPrinted(LocalDateTime lastPrinted)`
    pub fn set_last_printed(&mut self, last_printed: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::Lastprinted.into(),
            last_printed.map(FieldValue::DateTime),
        );
    }

/// Get the application short name.
    ///
    /// Corresponds to Java: `public String getShortApplicationName()`
    pub fn get_short_application_name(&self) -> Option<String> {
        self.opt_string(ProjectField::ShortApplicationName.into())
    }

/// Set the application short name.
    ///
    /// Corresponds to Java: `public void setShortApplicationName(String application)`
    pub fn set_short_application_name(&mut self, application: Option<String>) {
        self.set_field(
            ProjectField::ShortApplicationName.into(),
            application.map(FieldValue::String),
        );
    }

/// Get the edit time property.
    ///
    /// Corresponds to Java: `public Integer getEditingTime()`
    pub fn get_editing_time(&self) -> Option<i32> {
        self.opt_int(ProjectField::EditingTime.into())
    }

/// Set the edit time property.
    ///
    /// Corresponds to Java: `public void setEditingTime(Integer editingTime)`
    pub fn set_editing_time(&mut self, editing_time: Option<i32>) {
        self.set_field(
            ProjectField::EditingTime.into(),
            editing_time.map(FieldValue::Integer),
        );
    }

/// Get the format property.
    ///
    /// Corresponds to Java: `public String getPresentationFormat()`
    pub fn get_presentation_format(&self) -> Option<String> {
        self.opt_string(ProjectField::PresentationFormat.into())
    }

/// Set the format property.
    ///
    /// Corresponds to Java: `public void setPresentationFormat(String format)`
    pub fn set_presentation_format(&mut self, format: Option<String>) {
        self.set_field(
            ProjectField::PresentationFormat.into(),
            format.map(FieldValue::String),
        );
    }

/// Get the content type property.
    ///
    /// Corresponds to Java: `public String getContentType()`
    pub fn get_content_type(&self) -> Option<String> {
        self.opt_string(ProjectField::ContentType.into())
    }

/// Set the content type property.
    ///
    /// Corresponds to Java: `public void setContentType(String contentType)`
    pub fn set_content_type(&mut self, content_type: Option<String>) {
        self.set_field(
            ProjectField::ContentType.into(),
            content_type.map(FieldValue::String),
        );
    }

/// Get the content status property.
    ///
    /// Corresponds to Java: `public String getContentStatus()`
    pub fn get_content_status(&self) -> Option<String> {
        self.opt_string(ProjectField::ContentStatus.into())
    }

/// Set the content status property.
    ///
    /// Corresponds to Java: `public void setContentStatus(String contentStatus)`
    pub fn set_content_status(&mut self, content_status: Option<String>) {
        self.set_field(
            ProjectField::ContentStatus.into(),
            content_status.map(FieldValue::String),
        );
    }

/// Get the language property.
    ///
    /// Corresponds to Java: `public String getLanguage()`
    pub fn get_language(&self) -> Option<String> {
        self.opt_string(ProjectField::Language.into())
    }

/// Set the language property.
    ///
    /// Corresponds to Java: `public void setLanguage(String language)`
    pub fn set_language(&mut self, language: Option<String>) {
        self.set_field(ProjectField::Language.into(), language.map(FieldValue::String));
    }

/// Get the document version property.
    ///
    /// Corresponds to Java: `public String getDocumentVersion()`
    pub fn get_document_version(&self) -> Option<String> {
        self.opt_string(ProjectField::DocumentVersion.into())
    }

/// Set the document version property.
    ///
    /// Corresponds to Java: `public void setDocumentVersion(String documentVersion)`
    pub fn set_document_version(&mut self, document_version: Option<String>) {
        self.set_field(
            ProjectField::DocumentVersion.into(),
            document_version.map(FieldValue::String),
        );
    }

    //
// MPX-specific properties
    //

/// Set the MPX delimiter (default `','`).
    ///
    /// Corresponds to Java: `public void setMpxDelimiter(char delimiter)`
    pub fn set_mpx_delimiter(&mut self, delimiter: char) {
        self.set_char(ProjectField::MpxDelimiter, delimiter);
    }

/// Get the MPX delimiter; uses the Java default `','` when unset.
    ///
    /// Corresponds to Java: `public char getMpxDelimiter()`
    pub fn get_mpx_delimiter(&self) -> char {
        self.char_value(ProjectField::MpxDelimiter.into(), DEFAULT_MPX_DELIMITER)
    }

/// Set the name of the program that created the file.
    ///
    /// Corresponds to Java: `public void setMpxProgramName(String programName)`
    pub fn set_mpx_program_name(&mut self, program_name: Option<String>) {
        self.set_field(
            ProjectField::MpxProgramName.into(),
            program_name.map(FieldValue::String),
        );
    }

/// Get the name of the program that created the file; uses the Java default `"Microsoft Project for Windows"` when unset.
    ///
    /// Corresponds to Java: `public String getMpxProgramName()`
    pub fn get_mpx_program_name(&self) -> String {
        self.string_value(ProjectField::MpxProgramName.into(), DEFAULT_MPX_PROGRAM_NAME)
    }

/// Set the MPX file version.
    ///
    /// Corresponds to Java: `public void setMpxFileVersion(FileVersion version)`
    pub fn set_mpx_file_version(&mut self, version: Option<FileVersion>) {
        self.set_field(
            ProjectField::MpxFileVersion.into(),
            version.map(|value| FieldValue::Integer(value.value())),
        );
    }

/// Get the MPX file version; uses the Java default `VERSION_4_0` when unset.
    ///
    /// Corresponds to Java: `public FileVersion getMpxFileVersion()`
    pub fn get_mpx_file_version(&self) -> FileVersion {
        match self.opt_int(ProjectField::MpxFileVersion.into()) {
            Some(1) => FileVersion::Version1_0,
            Some(3) => FileVersion::Version3_0,
            _ => FileVersion::Version4_0,
        }
    }

/// Set the MPX code page.
    ///
    /// Corresponds to Java: `public void setMpxCodePage(CodePage codePage)`
    pub fn set_mpx_code_page(&mut self, code_page: Option<CodePage>) {
        self.set_field(
            ProjectField::MpxCodePage.into(),
            code_page.map(|value| FieldValue::String(value.get_value().to_owned())),
        );
    }

/// Get the MPX code page; uses the Java default `ANSI` when unset.
    ///
    /// Corresponds to Java: `public CodePage getMpxCodePage()`
    pub fn get_mpx_code_page(&self) -> CodePage {
        self.opt_string(ProjectField::MpxCodePage.into())
            .map_or(CodePage::Ansi, |value| CodePage::get_instance(&value))
    }

/// Set the project file path.
    ///
    /// Corresponds to Java: `public void setProjectFilePath(String projectFilePath)`
    pub fn set_project_file_path(&mut self, project_file_path: Option<String>) {
        self.set_field(
            ProjectField::ProjectFilePath.into(),
            project_file_path.map(FieldValue::String),
        );
    }

/// Get the project file path.
    ///
    /// Corresponds to Java: `public String getProjectFilePath()`
    pub fn get_project_file_path(&self) -> Option<String> {
        self.opt_string(ProjectField::ProjectFilePath.into())
    }

/// Get the full name of the application that created this project data.
    ///
    /// Corresponds to Java: `public String getFullApplicationName()`
    pub fn get_full_application_name(&self) -> Option<String> {
        self.opt_string(ProjectField::FullApplicationName.into())
    }

/// Set the full name of the application that created this project data.
    ///
    /// Corresponds to Java: `public void setFullApplicationName(String name)`
    pub fn set_full_application_name(&mut self, name: Option<String>) {
        self.set_field(
            ProjectField::FullApplicationName.into(),
            name.map(FieldValue::String),
        );
    }

/// Get the version of the application that created this project.
    ///
    /// Corresponds to Java: `public Integer getApplicationVersion()`
    pub fn get_application_version(&self) -> Option<i32> {
        self.opt_int(ProjectField::ApplicationVersion.into())
    }

/// Set the version of the application that created this project.
    ///
    /// Corresponds to Java: `public void setApplicationVersion(Integer version)`
    pub fn set_application_version(&mut self, version: Option<i32>) {
        self.set_field(
            ProjectField::ApplicationVersion.into(),
            version.map(FieldValue::Integer),
        );
    }

/// Get the type of the MPP file that was read (8 = Project 98, 9 = Project 2000/2002, 12 = Project 2003/2007, 14 = Project 2010/2013).
    ///
    /// Corresponds to Java: `public Integer getMppFileType()`
    pub fn get_mpp_file_type(&self) -> Option<i32> {
        self.opt_int(ProjectField::MppFileType.into())
    }

/// Set the MPP file type (internal use).
    ///
    /// Corresponds to Java: `public void setMppFileType(Integer fileType)`
    pub fn set_mpp_file_type(&mut self, file_type: Option<i32>) {
        self.set_field(
            ProjectField::MppFileType.into(),
            file_type.map(FieldValue::Integer),
        );
    }

/// Get the "auto filter enabled" flag.
    ///
    /// Corresponds to Java: `public boolean getAutoFilter()`
    pub fn get_auto_filter(&self) -> bool {
        self.bool_value(ProjectField::Autofilter.into())
    }

/// Set the "auto filter enabled" flag.
    ///
    /// Corresponds to Java: `public void setAutoFilter(boolean autoFilter)`
    pub fn set_auto_filter(&mut self, auto_filter: bool) {
        self.set_boolean(ProjectField::Autofilter, auto_filter);
    }

/// Get the provider of the file that generated this `ProjectFile`.
    ///
    /// Corresponds to Java: `public String getFileApplication()`
    pub fn get_file_application(&self) -> Option<String> {
        self.opt_string(ProjectField::FileApplication.into())
    }

/// Set the provider of the file that generated this `ProjectFile`.
    ///
    /// Corresponds to Java: `public void setFileApplication(String type)`
    pub fn set_file_application(&mut self, r#type: Option<String>) {
        self.set_field(
            ProjectField::FileApplication.into(),
            r#type.map(FieldValue::String),
        );
    }

/// Get the file type that generated this `ProjectFile`.
    ///
    /// Corresponds to Java: `public String getFileType()`
    pub fn get_file_type(&self) -> Option<String> {
        self.opt_string(ProjectField::FileType.into())
    }

/// Set the file type that generated this `ProjectFile`.
    ///
    /// Corresponds to Java: `public void setFileType(String type)`
    pub fn set_file_type(&mut self, r#type: Option<String>) {
        self.set_field(ProjectField::FileType.into(), r#type.map(FieldValue::String));
    }

/// Get the P6 export flag (used to distinguish projects that were explicitly selected for export from those exported incidentally because of external relationships).
    ///
    /// Corresponds to Java: `public boolean getExportFlag()`
    pub fn get_export_flag(&self) -> bool {
        self.bool_value(ProjectField::ExportFlag.into())
    }

/// Set the P6 export flag.
    ///
    /// Corresponds to Java: `public void setExportFlag(boolean value)`
    pub fn set_export_flag(&mut self, value: bool) {
        self.set_boolean(ProjectField::ExportFlag, value);
    }

/// Get the baseline project unique ID.
    ///
    /// Corresponds to Java: `public Integer getBaselineProjectUniqueID()`
    pub fn get_baseline_project_unique_id(&self) -> Option<i32> {
        self.opt_int(ProjectField::BaselineProjectUniqueId.into())
    }

/// Set the baseline project unique ID.
    ///
    /// Corresponds to Java: `public void setBaselineProjectUniqueID(Integer uniqueID)`
    pub fn set_baseline_project_unique_id(&mut self, unique_id: Option<i32>) {
        self.set_field(
            ProjectField::BaselineProjectUniqueId.into(),
            unique_id.map(FieldValue::Integer),
        );
    }

/// Get the project ID.
    ///
    /// Corresponds to Java: `public String getProjectID()`
    pub fn get_project_id(&self) -> Option<String> {
        self.opt_string(ProjectField::ProjectId.into())
    }

/// Set the project ID.
    ///
    /// Corresponds to Java: `public void setProjectID(String id)`
    pub fn set_project_id(&mut self, id: Option<String>) {
        self.set_field(ProjectField::ProjectId.into(), id.map(FieldValue::String));
    }

/// Get the critical activity determination method; falls back to the Java default `TOTAL_FLOAT` when unset.
    ///
    /// Corresponds to Java: `public CriticalActivityType getCriticalActivityType()`
    pub fn get_critical_activity_type(&self) -> CriticalActivityType {
        self.opt_int(ProjectField::CriticalActivityType.into())
            .and_then(CriticalActivityType::get_instance)
            .unwrap_or(CriticalActivityType::TotalFloat)
    }

/// Set the critical activity determination method.
    ///
    /// Corresponds to Java: `public void setCriticalActivityType(CriticalActivityType value)`
    pub fn set_critical_activity_type(&mut self, value: Option<CriticalActivityType>) {
        self.set_field(
            ProjectField::CriticalActivityType.into(),
            value.map(|value| FieldValue::Integer(value.value())),
        );
    }

/// Set the "must finish by" date.
    ///
    /// Corresponds to Java: `public void setMustFinishBy(LocalDateTime date)`
    pub fn set_must_finish_by(&mut self, date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::MustFinishBy.into(),
            date.map(FieldValue::DateTime),
        );
    }

/// Get the "must finish by" date.
    ///
    /// Corresponds to Java: `public LocalDateTime getMustFinishBy()`
    pub fn get_must_finish_by(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::MustFinishBy.into())
    }

/// Set the "scheduled finish" date.
    ///
    /// Corresponds to Java: `public void setScheduledFinish(LocalDateTime date)`
    pub fn set_scheduled_finish(&mut self, date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::ScheduledFinish.into(),
            date.map(FieldValue::DateTime),
        );
    }

/// Get the "scheduled finish" date.
    ///
    /// Corresponds to Java: `public LocalDateTime getScheduledFinish()`
    pub fn get_scheduled_finish(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::ScheduledFinish.into())
    }

/// Set the "scheduled start" date.
    ///
    /// Corresponds to Java: `public void setPlannedStart(LocalDateTime date)`
    pub fn set_planned_start(&mut self, date: Option<NaiveDateTime>) {
        self.set_field(ProjectField::PlannedStart.into(), date.map(FieldValue::DateTime));
    }

/// Get the "scheduled start" date.
    ///
    /// Corresponds to Java: `public LocalDateTime getPlannedStart()`
    pub fn get_planned_start(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::PlannedStart.into())
    }

/// Get the location unique ID.
    ///
    /// Corresponds to Java: `public Integer getLocationUniqueID()`
    pub fn get_location_unique_id(&self) -> Option<i32> {
        self.opt_int(ProjectField::LocationUniqueId.into())
    }

/// Set the location unique ID.
    ///
    /// Corresponds to Java: `public void setLocationUniqueID(Integer uniqueID)`
    pub fn set_location_unique_id(&mut self, unique_id: Option<i32>) {
        self.set_field(
            ProjectField::LocationUniqueId.into(),
            unique_id.map(FieldValue::Integer),
        );
    }

/// Get the resource pool file.
    ///
    /// Corresponds to Java: `public String getResourcePoolFile()`
    pub fn get_resource_pool_file(&self) -> Option<String> {
        self.opt_string(ProjectField::ResourcePoolFile.into())
    }

/// Set the resource pool file.
    ///
    /// Corresponds to Java: `public void setResourcePoolFile(String file)`
    pub fn set_resource_pool_file(&mut self, file: Option<String>) {
        self.set_field(
            ProjectField::ResourcePoolFile.into(),
            file.map(FieldValue::String),
        );
    }

/// Set the total slack calculation method.
    ///
    /// Corresponds to Java: `public void setTotalSlackCalculationType(TotalSlackCalculationType type)`
    pub fn set_total_slack_calculation_type(&mut self, r#type: Option<TotalSlackCalculationType>) {
        self.set_field(
            ProjectField::TotalSlackCalculationType.into(),
            r#type.map(|value| FieldValue::Integer(ordinal_of(&TotalSlackCalculationType::ALL, value))),
        );
    }

/// Get the total slack calculation method; falls back to the Java default `SMALLEST_SLACK` when unset.
    ///
    /// Corresponds to Java: `public TotalSlackCalculationType getTotalSlackCalculationType()`
    pub fn get_total_slack_calculation_type(&self) -> TotalSlackCalculationType {
        self.opt_int(ProjectField::TotalSlackCalculationType.into())
            .and_then(TotalSlackCalculationType::get_instance)
            .unwrap_or(TotalSlackCalculationType::SmallestSlack)
    }

/// Set the calendar used for relationship lag.
    ///
    /// Corresponds to Java: `public void setRelationshipLagCalendar(RelationshipLagCalendar calendar)`
    pub fn set_relationship_lag_calendar(&mut self, calendar: Option<RelationshipLagCalendar>) {
        self.set_field(
            ProjectField::RelationshipLagCalendar.into(),
            calendar.map(|value| {
                FieldValue::Integer(ordinal_of(&RelationshipLagCalendar::ALL, value))
            }),
        );
    }

/// Get the calendar used for relationship lag; falls back to the Java default `PREDECESSOR` when unset.
    ///
    /// Corresponds to Java: `public RelationshipLagCalendar getRelationshipLagCalendar()`
    pub fn get_relationship_lag_calendar(&self) -> RelationshipLagCalendar {
        self.opt_int(ProjectField::RelationshipLagCalendar.into())
            .and_then(RelationshipLagCalendar::get_instance)
            .unwrap_or(RelationshipLagCalendar::Predecessor)
    }

/// Get the WBS code separator; falls back to the Java default `"."` when unset.
    ///
    /// Corresponds to Java: `public String getWbsCodeSeparator()`
    pub fn get_wbs_code_separator(&self) -> String {
        self.string_value(ProjectField::WbsCodeSeparator.into(), DEFAULT_WBS_CODE_SEPARATOR)
    }

/// Set the WBS code separator.
    ///
    /// Corresponds to Java: `public void setWbsCodeSeparator(String value)`
    pub fn set_wbs_code_separator(&mut self, value: Option<String>) {
        self.set_field(
            ProjectField::WbsCodeSeparator.into(),
            value.map(FieldValue::String),
        );
    }

/// Get the "consider assignments in other projects when leveling" flag.
    ///
    /// Corresponds to Java: `public boolean getConsiderAssignmentsInOtherProjects()`
    pub fn get_consider_assignments_in_other_projects(&self) -> bool {
        self.bool_value(ProjectField::ConsiderAssignmentsInOtherProjects.into())
    }

/// Set the "consider assignments in other projects when leveling" flag.
    ///
    /// Corresponds to Java: `public void setConsiderAssignmentsInOtherProjects(boolean value)`
    pub fn set_consider_assignments_in_other_projects(&mut self, value: bool) {
        self.set_boolean(ProjectField::ConsiderAssignmentsInOtherProjects, value);
    }

/// Get the minimum priority of assignments from other projects to consider when leveling; falls back to the Java default 5 when unset.
    ///
    /// Corresponds to Java: `public Integer getConsiderAssignmentsInOtherProjectsWithPriorityEqualHigherThan()`
    pub fn get_consider_assignments_in_other_projects_with_priority_equal_higher_than(&self) -> i32 {
        self.int_value(
            ProjectField::ConsiderAssignmentsInOtherProjectsWithPriorityEqualHigherThan.into(),
            DEFAULT_OTHER_PROJECT_ASSIGNMENT_PRIORITY,
        )
    }

/// Set the minimum priority of assignments from other projects to consider when leveling.
    ///
    /// Corresponds to Java: `public void setConsiderAssignmentsInOtherProjectsWithPriorityEqualHigherThan(Integer value)`
    pub fn set_consider_assignments_in_other_projects_with_priority_equal_higher_than(
        &mut self,
        value: Option<i32>,
    ) {
        self.set_field(
            ProjectField::ConsiderAssignmentsInOtherProjectsWithPriorityEqualHigherThan.into(),
            value.map(FieldValue::Integer),
        );
    }

/// Get the "retain scheduled earliest / latest dates" flag.
    ///
    /// Corresponds to Java: `public boolean getPreserveScheduledEarlyAndLateDates()`
    pub fn get_preserve_scheduled_early_and_late_dates(&self) -> bool {
        self.bool_value(ProjectField::PreserveScheduledEarlyAndLateDates.into())
    }

/// Set the "retain scheduled earliest / latest dates" flag.
    ///
    /// Corresponds to Java: `public void setPreserveScheduledEarlyAndLateDates(boolean value)`
    pub fn set_preserve_scheduled_early_and_late_dates(&mut self, value: bool) {
        self.set_boolean(ProjectField::PreserveScheduledEarlyAndLateDates, value);
    }

/// Get the "level all resources" flag.
    ///
    /// Corresponds to Java: `public boolean getLevelAllResources()`
    pub fn get_level_all_resources(&self) -> bool {
        self.bool_value(ProjectField::LevelAllResources.into())
    }

/// Set the "level all resources" flag.
    ///
    /// Corresponds to Java: `public void setLevelAllResources(boolean value)`
    pub fn set_level_all_resources(&mut self, value: bool) {
        self.set_boolean(ProjectField::LevelAllResources, value);
    }

/// Get the "level resources only within activity total slack" flag.
    ///
    /// Corresponds to Java: `public boolean getLevelResourcesOnlyWithinActivityTotalFloat()`
    pub fn get_level_resources_only_within_activity_total_float(&self) -> bool {
        self.bool_value(ProjectField::LevelResourcesOnlyWithinActivityTotalFloat.into())
    }

/// Set the "level resources only within activity total slack" flag.
    ///
    /// Corresponds to Java: `public void setLevelResourcesOnlyWithinActivityTotalFloat(boolean value)`
    pub fn set_level_resources_only_within_activity_total_float(&mut self, value: bool) {
        self.set_boolean(ProjectField::LevelResourcesOnlyWithinActivityTotalFloat, value);
    }

/// Get the minimum slack to retain when leveling; falls back to the Java default "1 hour" when unset.
    ///
    /// Corresponds to Java: `public Duration getPreserveMinimumFloatWhenLeveling()`
    pub fn get_preserve_minimum_float_when_leveling(&self) -> Duration {
        self.opt_duration(ProjectField::PreserveMinimumFloatWhenLeveling.into())
            .unwrap_or_else(default_minimum_float)
    }

/// Set the minimum slack to retain when leveling.
    ///
    /// Corresponds to Java: `public void setPreserveMinimumFloatWhenLeveling(Duration value)`
    pub fn set_preserve_minimum_float_when_leveling(&mut self, value: Option<Duration>) {
        self.set_field(
            ProjectField::PreserveMinimumFloatWhenLeveling.into(),
            value.map(FieldValue::Duration),
        );
    }

/// Get the maximum percentage of resource over-allocation; falls back to the Java default 25.0 when unset.
    ///
    /// Corresponds to Java: `public Number getMaxPercentToOverallocateResources()`
    pub fn get_max_percent_to_overallocate_resources(&self) -> f64 {
        self.opt_double(ProjectField::MaxPercentToOverallocateResources.into())
            .unwrap_or(DEFAULT_OVERALLOCATION)
    }

/// Set the maximum percentage of resource over-allocation.
    ///
    /// Corresponds to Java: `public void setMaxPercentToOverallocateResources(Number value)`
    pub fn set_max_percent_to_overallocate_resources(&mut self, value: Option<f64>) {
        self.set_field(
            ProjectField::MaxPercentToOverallocateResources.into(),
            value.map(FieldValue::Double),
        );
    }

/// Get the leveling priority expression; falls back to the Java default when unset.
    ///
    /// Corresponds to Java: `public String getLevelingPriorities()`
    pub fn get_leveling_priorities(&self) -> String {
        self.string_value(
            ProjectField::LevelingPriorities.into(),
            DEFAULT_LEVELING_PRIORITIES,
        )
    }

/// Set the leveling priority expression.
    ///
    /// Corresponds to Java: `public void setLevelingPriorities(String value)`
    pub fn set_leveling_priorities(&mut self, value: Option<String>) {
        self.set_field(
            ProjectField::LevelingPriorities.into(),
            value.map(FieldValue::String),
        );
    }

/// Get the "set data date and scheduled start date to the project forecast start date" flag.
    ///
    /// Corresponds to Java: `public boolean getDataDateAndPlannedStartSetToProjectForecastStart()`
    pub fn get_data_date_and_planned_start_set_to_project_forecast_start(&self) -> bool {
        self.bool_value(ProjectField::DataDateAndPlannedStartSetToProjectForecastStart.into())
    }

/// Set the "set data date and scheduled start date to the project forecast start date" flag.
    ///
    /// Corresponds to Java: `public void setDataDateAndPlannedStartSetToProjectForecastStart(boolean value)`
    pub fn set_data_date_and_planned_start_set_to_project_forecast_start(&mut self, value: bool) {
        self.set_boolean(
            ProjectField::DataDateAndPlannedStartSetToProjectForecastStart,
            value,
        );
    }

/// Get the "ignore logical relationships with other projects" flag.
    ///
    /// Corresponds to Java: `public boolean getIgnoreRelationshipsToAndFromOtherProjects()`
    pub fn get_ignore_relationships_to_and_from_other_projects(&self) -> bool {
        self.bool_value(ProjectField::IgnoreRelationshipsToAndFromOtherProjects.into())
    }

/// Set the "ignore logical relationships with other projects" flag.
    ///
    /// Corresponds to Java: `public void setIgnoreRelationshipsToAndFromOtherProjects(boolean value)`
    pub fn set_ignore_relationships_to_and_from_other_projects(&mut self, value: bool) {
        self.set_boolean(ProjectField::IgnoreRelationshipsToAndFromOtherProjects, value);
    }

/// Get the "mark open-ended activities as critical" flag.
    ///
    /// Corresponds to Java: `public boolean getMakeOpenEndedActivitiesCritical()`
    pub fn get_make_open_ended_activities_critical(&self) -> bool {
        self.bool_value(ProjectField::MakeOpenEndedActivitiesCritical.into())
    }

/// Set the "mark open-ended activities as critical" flag.
    ///
    /// Corresponds to Java: `public void setMakeOpenEndedActivitiesCritical(boolean value)`
    pub fn set_make_open_ended_activities_critical(&mut self, value: bool) {
        self.set_boolean(ProjectField::MakeOpenEndedActivitiesCritical, value);
    }

/// Get the "use expected finish date" flag.
    ///
    /// Corresponds to Java: `public boolean getUseExpectedFinishDates()`
    pub fn get_use_expected_finish_dates(&self) -> bool {
        self.bool_value(ProjectField::UseExpectedFinishDates.into())
    }

/// Set the "use expected finish date" flag.
    ///
    /// Corresponds to Java: `public void setUseExpectedFinishDates(boolean value)`
    pub fn set_use_expected_finish_dates(&mut self, value: bool) {
        self.set_boolean(ProjectField::UseExpectedFinishDates, value);
    }

/// Get the "calculate start-to-start lag from the earliest start date" flag.
    ///
    /// Corresponds to Java: `public boolean getComputeStartToStartLagFromEarlyStart()`
    pub fn get_compute_start_to_start_lag_from_early_start(&self) -> bool {
        self.bool_value(ProjectField::ComputeStartToStartLagFromEarlyStart.into())
    }

/// Set the "calculate start-to-start lag from the earliest start date" flag.
    ///
    /// Corresponds to Java: `public void setComputeStartToStartLagFromEarlyStart(boolean value)`
    pub fn set_compute_start_to_start_lag_from_early_start(&mut self, value: bool) {
        self.set_boolean(ProjectField::ComputeStartToStartLagFromEarlyStart, value);
    }

/// Get the "calculate slack against each project's finish date" flag.
    ///
    /// Corresponds to Java: `public boolean getCalculateFloatBasedOnFinishDateOfEachProject()`
    pub fn get_calculate_float_based_on_finish_date_of_each_project(&self) -> bool {
        self.bool_value(ProjectField::CalculateFloatBasedOnFinishDateOfEachProject.into())
    }

/// Set the "calculate slack against each project's finish date" flag.
    ///
    /// Corresponds to Java: `public void setCalculateFloatBasedOnFinishDateOfEachProject(boolean value)`
    pub fn set_calculate_float_based_on_finish_date_of_each_project(&mut self, value: bool) {
        self.set_boolean(ProjectField::CalculateFloatBasedOnFinishDateOfEachProject, value);
    }

/// Get the "calculate multiple float paths" flag.
    ///
    /// Corresponds to Java: `public boolean getCalculateMultipleFloatPaths()`
    pub fn get_calculate_multiple_float_paths(&self) -> bool {
        self.bool_value(ProjectField::CalculateMultipleFloatPaths.into())
    }

/// Set the "calculate multiple float paths" flag.
    ///
    /// Corresponds to Java: `public void setCalculateMultipleFloatPaths(boolean value)`
    pub fn set_calculate_multiple_float_paths(&mut self, value: bool) {
        self.set_boolean(ProjectField::CalculateMultipleFloatPaths, value);
    }

/// Get the "calculate multiple float paths using total slack" flag.
    ///
    /// Corresponds to Java: `public boolean getCalculateMultipleFloatPathsUsingTotalFloat()`
    pub fn get_calculate_multiple_float_paths_using_total_float(&self) -> bool {
        self.bool_value(ProjectField::CalculateMultipleFloatPathsUsingTotalFloat.into())
    }

/// Set the "calculate multiple float paths using total slack" flag.
    ///
    /// Corresponds to Java: `public void setCalculateMultipleFloatPathsUsingTotalFloat(boolean value)`
    pub fn set_calculate_multiple_float_paths_using_total_float(&mut self, value: bool) {
        self.set_boolean(ProjectField::CalculateMultipleFloatPathsUsingTotalFloat, value);
    }

/// Get the "unique ID of the ending activity for multiple float paths".
    ///
    /// Corresponds to Java: `public Integer getDisplayMultipleFloatPathsEndingWithActivityUniqueID()`
    pub fn get_display_multiple_float_paths_ending_with_activity_unique_id(&self) -> Option<i32> {
        self.opt_int(ProjectField::DisplayMultipleFloatPathsEndingWithActivityUniqueId.into())
    }

/// Set the "unique ID of the ending activity for multiple float paths".
    ///
    /// Corresponds to Java: `public void setDisplayMultipleFloatPathsEndingWithActivityUniqueID(Integer value)`
    pub fn set_display_multiple_float_paths_ending_with_activity_unique_id(
        &mut self,
        value: Option<i32>,
    ) {
        self.set_field(
            ProjectField::DisplayMultipleFloatPathsEndingWithActivityUniqueId.into(),
            value.map(FieldValue::Integer),
        );
    }

/// Get the "limit the number of float paths to calculate" flag.
    ///
    /// Corresponds to Java: `public boolean getLimitNumberOfFloatPathsToCalculate()`
    pub fn get_limit_number_of_float_paths_to_calculate(&self) -> bool {
        self.bool_value(ProjectField::LimitNumberOfFloatPathsToCalculate.into())
    }

/// Set the "limit the number of float paths to calculate" flag.
    ///
    /// Corresponds to Java: `public void setLimitNumberOfFloatPathsToCalculate(boolean value)`
    pub fn set_limit_number_of_float_paths_to_calculate(&mut self, value: bool) {
        self.set_boolean(ProjectField::LimitNumberOfFloatPathsToCalculate, value);
    }

/// Get the maximum number of float paths to calculate; falls back to the Java default 10 when unset.
    ///
    /// Corresponds to Java: `public Integer getMaximumNumberOfFloatPathsToCalculate()`
    pub fn get_maximum_number_of_float_paths_to_calculate(&self) -> i32 {
        self.int_value(
            ProjectField::MaximumNumberOfFloatPathsToCalculate.into(),
            DEFAULT_FLOAT_PATHS,
        )
    }

/// Set the maximum number of float paths to calculate.
    ///
    /// Corresponds to Java: `public void setMaximumNumberOfFloatPathsToCalculate(Integer value)`
    pub fn set_maximum_number_of_float_paths_to_calculate(&mut self, value: Option<i32>) {
        self.set_field(
            ProjectField::MaximumNumberOfFloatPathsToCalculate.into(),
            value.map(FieldValue::Integer),
        );
    }

/// Set the scheduling method for progressed activities.
    ///
    /// Corresponds to Java: `public void setSchedulingProgressedActivities(SchedulingProgressedActivities value)`
    pub fn set_scheduling_progressed_activities(
        &mut self,
        value: Option<SchedulingProgressedActivities>,
    ) {
        self.set_field(
            ProjectField::SchedulingProgressedActivities.into(),
            value.map(|value| {
                FieldValue::Integer(ordinal_of(&SchedulingProgressedActivities::ALL, value))
            }),
        );
    }

/// Get the scheduling method for progressed activities; falls back to the Java default `RETAINED_LOGIC` when unset.
    ///
    /// Corresponds to Java: `public SchedulingProgressedActivities getSchedulingProgressedActivities()`
    pub fn get_scheduling_progressed_activities(&self) -> SchedulingProgressedActivities {
        self.opt_int(ProjectField::SchedulingProgressedActivities.into())
            .and_then(SchedulingProgressedActivities::get_instance)
            .unwrap_or(SchedulingProgressedActivities::RetainedLogic)
    }

/// Get the name of the baseline type associated with this project.
    ///
    /// Corresponds to Java: `public String getBaselineTypeName()`
    pub fn get_baseline_type_name(&self) -> Option<String> {
        self.opt_string(ProjectField::BaselineTypeName.into())
    }

/// Set the name of the baseline type associated with this project.
    ///
    /// Corresponds to Java: `public void setBaselineTypeName(String value)`
    pub fn set_baseline_type_name(&mut self, value: Option<String>) {
        self.set_field(
            ProjectField::BaselineTypeName.into(),
            value.map(FieldValue::String),
        );
    }

/// Get the unique ID of the baseline type associated with this project.
    ///
    /// Corresponds to Java: `public Integer getBaselineTypeUniqueID()`
    pub fn get_baseline_type_unique_id(&self) -> Option<i32> {
        self.opt_int(ProjectField::BaselineTypeUniqueId.into())
    }

/// Set the unique ID of the baseline type associated with this project.
    ///
    /// Corresponds to Java: `public void setBaselineTypeUniqueID(Integer value)`
    pub fn set_baseline_type_unique_id(&mut self, value: Option<i32>) {
        self.set_field(
            ProjectField::BaselineTypeUniqueId.into(),
            value.map(FieldValue::Integer),
        );
    }

/// Get the last baseline update time.
    ///
    /// Corresponds to Java: `public LocalDateTime getLastBaselineUpdateDate()`
    pub fn get_last_baseline_update_date(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::LastBaselineUpdateDate.into())
    }

/// Set the last baseline update time.
    ///
    /// Corresponds to Java: `public void setLastBaselineUpdateDate(LocalDateTime value)`
    pub fn set_last_baseline_update_date(&mut self, value: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::LastBaselineUpdateDate.into(),
            value.map(FieldValue::DateTime),
        );
    }

/// Get the prefix used when creating Activity IDs; falls back to the Java default `"A"` when unset.
    ///
    /// Corresponds to Java: `public String getActivityIdPrefix()`
    pub fn get_activity_id_prefix(&self) -> String {
        self.string_value(ProjectField::ActivityIdPrefix.into(), DEFAULT_ACTIVITY_ID_PREFIX)
    }

/// Set the prefix used when creating Activity IDs.
    ///
    /// Corresponds to Java: `public void setActivityIdPrefix(String value)`
    pub fn set_activity_id_prefix(&mut self, value: Option<String>) {
        self.set_field(
            ProjectField::ActivityIdPrefix.into(),
            value.map(FieldValue::String),
        );
    }

/// Get the suffix used when creating Activity IDs; falls back to the Java default 1000 when unset.
    ///
    /// Corresponds to Java: `public Integer getActivityIdSuffix()`
    pub fn get_activity_id_suffix(&self) -> i32 {
        self.int_value(
            ProjectField::ActivityIdSuffix.into(),
            DEFAULT_ACTIVITY_ID_SUFFIX,
        )
    }

/// Set the suffix used when creating Activity IDs.
    ///
    /// Corresponds to Java: `public void setActivityIdSuffix(Integer value)`
    pub fn set_activity_id_suffix(&mut self, value: Option<i32>) {
        self.set_field(
            ProjectField::ActivityIdSuffix.into(),
            value.map(FieldValue::Integer),
        );
    }

/// Get the increment used when creating Activity IDs; falls back to the Java default 10 when unset.
    ///
    /// Corresponds to Java: `public Integer getActivityIdIncrement()`
    pub fn get_activity_id_increment(&self) -> i32 {
        self.int_value(
            ProjectField::ActivityIdIncrement.into(),
            DEFAULT_ACTIVITY_ID_INCREMENT,
        )
    }

/// Set the increment used when creating Activity IDs.
    ///
    /// Corresponds to Java: `public void setActivityIdIncrement(Integer value)`
    pub fn set_activity_id_increment(&mut self, value: Option<i32>) {
        self.set_field(
            ProjectField::ActivityIdIncrement.into(),
            value.map(FieldValue::Integer),
        );
    }

/// Get the "increment Activity IDs based on selected activities" flag.
    ///
    /// Corresponds to Java: `public boolean getActivityIdIncrementBasedOnSelectedActivity()`
    pub fn get_activity_id_increment_based_on_selected_activity(&self) -> bool {
        self.bool_value(ProjectField::ActivityIdIncrementBasedOnSelectedActivity.into())
    }

/// Set the "increment Activity IDs based on selected activities" flag.
    ///
    /// Corresponds to Java: `public void setActivityIdIncrementBasedOnSelectedActivity(boolean value)`
    pub fn set_activity_id_increment_based_on_selected_activity(&mut self, value: bool) {
        self.set_boolean(ProjectField::ActivityIdIncrementBasedOnSelectedActivity, value);
    }

/// Get the baseline calendar name.
    ///
    /// Corresponds to Java: `public String getBaselineCalendarName()`
    pub fn get_baseline_calendar_name(&self) -> Option<String> {
        self.opt_string(ProjectField::BaselineCalendarName.into())
    }

/// Set the baseline calendar name.
    ///
    /// Corresponds to Java: `public void setBaselineCalendarName(String value)`
    pub fn set_baseline_calendar_name(&mut self, value: Option<String>) {
        self.set_field(
            ProjectField::BaselineCalendarName.into(),
            value.map(FieldValue::String),
        );
    }

/// Determine whether this `ProjectFile` represents a baseline.
    ///
    /// Corresponds to Java: `public boolean getProjectIsBaseline()`
    pub fn get_project_is_baseline(&self) -> bool {
        self.bool_value(ProjectField::ProjectIsBaseline.into())
    }

/// Set whether this `ProjectFile` represents a baseline.
    ///
    /// Corresponds to Java: `public void setProjectIsBaseline(boolean value)`
    pub fn set_project_is_baseline(&mut self, value: bool) {
        self.set_boolean(ProjectField::ProjectIsBaseline, value);
    }

/// Get the project website URL.
    ///
    /// Corresponds to Java: `public String getProjectWebsiteUrl()`
    pub fn get_project_website_url(&self) -> Option<String> {
        self.opt_string(ProjectField::ProjectWebsiteUrl.into())
    }

/// Set the project website URL.
    ///
    /// Corresponds to Java: `public void setProjectWebsiteUrl(String value)`
    pub fn set_project_website_url(&mut self, value: Option<String>) {
        self.set_field(
            ProjectField::ProjectWebsiteUrl.into(),
            value.map(FieldValue::String),
        );
    }

    //
// Project notes
    //

/// Get the project notes as plain text (returns an empty string when there are no notes).
    ///
    /// Corresponds to Java: `public String getNotes()`
    ///
/// Java uses `TaskField.NOTES` (rather than `ProjectField.NOTES`) as the field key; the Rust side does the same.
    pub fn get_notes(&self) -> String {
        self.opt_string(FieldKey::Task(TaskField::Notes))
            .unwrap_or_default()
    }

/// Set the project notes.
    ///
    /// Corresponds to Java: `public void setNotes(String notes)`
    pub fn set_notes(&mut self, notes: Option<String>) {
        self.set_field(
            FieldKey::Task(TaskField::Notes),
            notes.map(FieldValue::String),
        );
    }

/// Get the project notes object.
    ///
    /// Corresponds to Java: `public Notes getNotesObject()`
    ///
/// `Notes` is a trait (Java's base class) and cannot be stored in `FieldValue`; the Rust side stores it uniformly as plain text,
/// so this method returns [`PlainNotes`] (see the file header notes).
    pub fn get_notes_object(&self) -> Option<PlainNotes> {
        self.opt_string(FieldKey::Task(TaskField::Notes))
            .map(|text| PlainNotes::new(Some(text)))
    }

/// Set the project notes object (its plain text form is stored in the field table).
    ///
    /// Corresponds to Java: `public void setNotesObject(Notes notes)`
    pub fn set_notes_object(&mut self, notes: Option<&dyn Notes>) {
        self.set_field(
            FieldKey::Task(TaskField::Notes),
            notes.map(|notes| FieldValue::String(notes.to_string())),
        );
    }

/// Get the "publish enabled" flag.
    ///
    /// Corresponds to Java: `public boolean getEnablePublication()`
    pub fn get_enable_publication(&self) -> bool {
        self.bool_value(ProjectField::EnablePublication.into())
    }

/// Set the "publish enabled" flag.
    ///
    /// Corresponds to Java: `public void setEnablePublication(boolean value)`
    pub fn set_enable_publication(&mut self, value: bool) {
        self.set_boolean(ProjectField::EnablePublication, value);
    }

/// Get the "summary enabled" flag.
    ///
    /// Corresponds to Java: `public boolean getEnableSummarization()`
    pub fn get_enable_summarization(&self) -> bool {
        self.bool_value(ProjectField::EnableSummarization.into())
    }

/// Set the "summary enabled" flag.
    ///
    /// Corresponds to Java: `public void setEnableSummarization(boolean value)`
    pub fn set_enable_summarization(&mut self, value: bool) {
        self.set_boolean(ProjectField::EnableSummarization, value);
    }

/// Get the default price per unit for activities without a resource / role price.
    ///
    /// Corresponds to Java: `public Number getActivityDefaultPricePerUnit()`
    pub fn get_activity_default_price_per_unit(&self) -> Option<f64> {
        self.opt_double(ProjectField::ActivityDefaultPricePerUnit.into())
    }

/// Set the default price per unit for activities without a resource / role price.
    ///
    /// Corresponds to Java: `public void setActivityDefaultPricePerUnit(Number value)`
    pub fn set_activity_default_price_per_unit(&mut self, value: Option<f64>) {
        self.set_field(
            ProjectField::ActivityDefaultPricePerUnit.into(),
            value.map(FieldValue::Double),
        );
    }

/// Set the "update units when resource assignment costs change" flag.
    ///
    /// Corresponds to Java: `public void setUpdateUnitsWhenCostsChangeOnResourceAssignments(boolean value)`
    pub fn set_update_units_when_costs_change_on_resource_assignments(&mut self, value: bool) {
        self.set_boolean(
            ProjectField::UpdateUnitsWhenCostsChangeOnResourceAssignments,
            value,
        );
    }

/// Get the "update units when resource assignment costs change" flag.
    ///
    /// Corresponds to Java: `public boolean getUpdateUnitsWhenCostsChangeOnResourceAssignments()`
    pub fn get_update_units_when_costs_change_on_resource_assignments(&self) -> bool {
        self.bool_value(ProjectField::UpdateUnitsWhenCostsChangeOnResourceAssignments.into())
    }

    //
// Field container (public methods inherited from Java's AbstractFieldContainer)
    //

/// Disable dependent field cleanup and event firing on field updates.
    ///
/// Corresponds to Java: `public void disableEvents()` (inherited from `AbstractFieldContainer`)
    pub fn disable_events(&mut self) {
        self.fields.disable_events();
    }

/// Enable dependent field cleanup and event firing on field updates (default state).
    ///
/// Corresponds to Java: `public void enableEvents()` (inherited from `AbstractFieldContainer`)
    pub fn enable_events(&mut self) {
        self.fields.enable_events();
    }

    //
// Private helper methods
    //

/// Get a nullable integer (typed wrapper around `get`).
    fn opt_int(&self, field: FieldKey) -> Option<i32> {
        self.get(field).and_then(|value| value.as_integer())
    }

/// Get a nullable numeric value (Java's `Number`).
    fn opt_double(&self, field: FieldKey) -> Option<f64> {
        self.get(field).and_then(|value| value.as_double())
    }

/// Get a boolean value; Java's `BooleanHelper.getBoolean(null)` returns `false`.
    fn bool_value(&self, field: FieldKey) -> bool {
        self.get(field)
            .and_then(|value| value.as_boolean())
            .unwrap_or(false)
    }

/// Get an integer, falling back to the given default when missing.
    fn int_value(&self, field: FieldKey, default: i32) -> i32 {
        self.opt_int(field).unwrap_or(default)
    }

/// Get a string, falling back to the given default when missing.
    fn string_value(&self, field: FieldKey, default: &str) -> String {
        self.opt_string(field)
            .unwrap_or_else(|| default.to_owned())
    }

/// Get a nullable string.
    fn opt_string(&self, field: FieldKey) -> Option<String> {
        self.get(field)
            .and_then(|value| value.as_string().map(str::to_owned))
    }

/// Get a nullable date-time.
    fn opt_date_time(&self, field: FieldKey) -> Option<NaiveDateTime> {
        self.get(field).and_then(|value| value.as_date_time())
    }

/// Get a nullable time.
    fn opt_time(&self, field: FieldKey) -> Option<NaiveTime> {
        self.get(field).and_then(|value| value.as_time())
    }

/// Get a nullable duration.
    fn opt_duration(&self, field: FieldKey) -> Option<Duration> {
        self.get(field).and_then(|value| value.as_duration())
    }

/// Get a nullable rate.
    fn opt_rate(&self, field: FieldKey) -> Option<Rate> {
        self.get(field).and_then(|value| value.as_rate())
    }

/// Get a character value, falling back to the given default when missing (`FieldValue` has no character variant; see the file header notes).
    fn char_value(&self, field: FieldKey, default: char) -> char {
        self.opt_string(field)
            .and_then(|text| text.chars().next())
            .unwrap_or(default)
    }

/// Write a field value (internal entry point for `FieldContainer::set`).
    fn set_field(&mut self, field: FieldKey, value: Option<FieldValue>) {
        FieldContainer::set(self, field, value);
    }

/// Write a boolean field (corresponds to Java's `private void set(FieldType field, boolean value)`).
    fn set_boolean(&mut self, field: ProjectField, value: bool) {
        self.set_field(field.into(), Some(FieldValue::Boolean(value)));
    }

/// Write a character field (stored as a single-character string; see the file header notes).
    fn set_char(&mut self, field: ProjectField, value: char) {
        self.set_field(field.into(), Some(FieldValue::String(value.to_string())));
    }

/// Upgrade the parent file reference and get its project context (returns `None` when the parent file has been dropped or there is a borrow conflict).
    fn parent_context(&self) -> Option<Rc<RefCell<ProjectContext>>> {
        parent_context_of(&self.parent_file)
    }

/// Get the default calendar unique ID from the given project context (corresponds to Java's `calculateDefaultCalendarUniqueID()`).
    fn calculate_default_calendar_unique_id(&self) -> Option<i32> {
        let context = self.parent_context()?;
        let context_ref = context.try_borrow().ok()?;
        context_ref.get_calendars().get_default_calendar_unique_id()
    }

/// Find a calendar by unique ID in the given context.
    fn find_calendar(
        &self,
        context: &Rc<RefCell<ProjectContext>>,
        unique_id: Option<i32>,
    ) -> Option<Rc<RefCell<ProjectCalendar>>> {
        let context_ref = context.try_borrow().ok()?;
        context_ref.get_calendars().get_by_unique_id(unique_id)
    }

/// Compute the value of a calculated field (corresponds to Java's `CALCULATED_FIELD_MAP` + `getCalculationMethod`).
    ///
/// On the Java side the map values are `Function<ProjectProperties, Object>`; the Rust side dispatches directly with `match`,
/// and uses "return `FieldValue`" in place of Java's `Object`.
    ///
/// In Java, the defaults for `CUSTOM_PROPERTIES` / `PROJECT_CODE_VALUES` are empty maps,
/// which `FieldValue` cannot represent, so `None` is returned (for the actual value of `CUSTOM_PROPERTIES`, see
/// [`ProjectProperties::get_custom_properties`]; see the file header notes).
    fn calculate_field(&self, field: FieldKey) -> Option<FieldValue> {
        let FieldKey::Project(project_field) = field else {
            return None;
        };

        match project_field {
// ---- Values rolled up from tasks ----
            ProjectField::ActualCost => self.get_value_from_summary_task(TaskField::ActualCost),
            ProjectField::ActualDuration => {
                self.get_value_from_summary_task(TaskField::ActualDuration)
            }
            ProjectField::ActualFinish => self.calculate_actual_finish(),
            ProjectField::ActualStart => self.calculate_actual_start(),
            ProjectField::ActualWork => self.get_value_from_summary_task(TaskField::ActualWork),
            ProjectField::BaselineCost => self.get_value_from_summary_task(TaskField::BaselineCost),
            ProjectField::BaselineDuration => {
                self.get_value_from_summary_task(TaskField::BaselineDuration)
            }
            ProjectField::BaselineFinish => {
                self.get_value_from_summary_task(TaskField::BaselineFinish)
            }
            ProjectField::BaselineStart => {
                self.get_value_from_summary_task(TaskField::BaselineStart)
            }
            ProjectField::BaselineWork => self.get_value_from_summary_task(TaskField::BaselineWork),
            ProjectField::Cost => self.get_value_from_summary_task(TaskField::Cost),
            ProjectField::Duration => self.get_value_from_summary_task(TaskField::Duration),
            ProjectField::FinishDate => self.calculate_finish_date(),
            ProjectField::FinishVariance => {
                self.get_value_from_summary_task(TaskField::FinishVariance)
            }
            ProjectField::PercentageComplete => {
                self.get_value_from_summary_task(TaskField::PercentComplete)
            }
            ProjectField::PlannedStart => self.get_value_from_summary_task(TaskField::PlannedStart),
            ProjectField::StartDate => self.calculate_start_date(),
            ProjectField::StartVariance => {
                self.get_value_from_summary_task(TaskField::StartVariance)
            }
            ProjectField::Work => self.get_value_from_summary_task(TaskField::Work),
// Java: no calculation method is registered for ProjectField.WORK2 (comment: meaning to be confirmed)

// ---- Project default values ----
            ProjectField::DefaultCalendarUniqueId => self
                .calculate_default_calendar_unique_id()
                .map(FieldValue::Integer),
            ProjectField::DaysPerMonth => self
                .time_unit_default_value(|defaults| defaults.get_days_per_month())
                .map(FieldValue::Integer),
            ProjectField::MinutesPerDay => self
                .time_unit_default_value(|defaults| defaults.get_minutes_per_day())
                .map(FieldValue::Integer),
            ProjectField::MinutesPerWeek => self
                .time_unit_default_value(|defaults| defaults.get_minutes_per_week())
                .map(FieldValue::Integer),
            ProjectField::MinutesPerMonth => self
                .time_unit_default_value(|defaults| defaults.get_minutes_per_month())
                .map(FieldValue::Integer),
            ProjectField::MinutesPerYear => self
                .time_unit_default_value(|defaults| defaults.get_minutes_per_year())
                .map(FieldValue::Integer),
            ProjectField::CurrencySymbol => {
                Some(FieldValue::String(DEFAULT_CURRENCY_SYMBOL.to_owned()))
            }
            ProjectField::CurrencySymbolPosition => Some(FieldValue::Integer(
                CurrencySymbolPosition::Before.value(),
            )),
            ProjectField::CurrencyDigits => Some(FieldValue::Integer(DEFAULT_CURRENCY_DIGITS)),
            ProjectField::MpxProgramName => {
                Some(FieldValue::String(DEFAULT_MPX_PROGRAM_NAME.to_owned()))
            }
            ProjectField::MpxFileVersion => {
                Some(FieldValue::Integer(FileVersion::Version4_0.value()))
            }
            ProjectField::MpxCodePage => Some(FieldValue::String(
                CodePage::Ansi.get_value().to_owned(),
            )),
            ProjectField::DateOrder => Some(FieldValue::Integer(DateOrder::Dmy.value())),
            ProjectField::TimeFormat => Some(FieldValue::Integer(ProjectTimeFormat::TwelveHour.value())),
            ProjectField::DefaultStartTime => Some(FieldValue::Time(default_start_time())),
            ProjectField::DateSeparator => Some(FieldValue::String(
                DEFAULT_DATE_SEPARATOR.to_string(),
            )),
            ProjectField::TimeSeparator => Some(FieldValue::String(
                DEFAULT_TIME_SEPARATOR.to_string(),
            )),
            ProjectField::AmText => Some(FieldValue::String(DEFAULT_AM_TEXT.to_owned())),
            ProjectField::PmText => Some(FieldValue::String(DEFAULT_PM_TEXT.to_owned())),
            ProjectField::DateFormat => {
                Some(FieldValue::Integer(ProjectDateFormat::DdMmYyyy.value()))
            }
            ProjectField::BarTextDateFormat => {
                Some(FieldValue::Integer(ProjectDateFormat::DdMmYyyy.value()))
            }
            ProjectField::DefaultDurationUnits => Some(FieldValue::Integer(TimeUnit::Days.value())),
            ProjectField::DefaultDurationIsFixed => Some(FieldValue::Boolean(false)),
            ProjectField::DefaultWorkUnits => Some(FieldValue::Integer(TimeUnit::Hours.value())),
            ProjectField::UpdatingTaskStatusUpdatesResourceStatus => {
                Some(FieldValue::Boolean(true))
            }
            ProjectField::SplitInProgressTasks => Some(FieldValue::Boolean(false)),
            ProjectField::ProjectExternallyEdited => Some(FieldValue::Boolean(false)),
            ProjectField::FiscalYearStart => Some(FieldValue::Boolean(false)),
            ProjectField::DefaultTaskEarnedValueMethod => Some(FieldValue::Integer(
                EarnedValueMethod::PercentComplete.value(),
            )),
            ProjectField::NewTasksEstimated => Some(FieldValue::Boolean(true)),
            ProjectField::AutoAddNewResourcesAndTasks => Some(FieldValue::Boolean(true)),
            ProjectField::AutoLink => Some(FieldValue::Boolean(true)),
            ProjectField::MicrosoftProjectServerUrl => Some(FieldValue::Boolean(true)),
            ProjectField::DefaultTaskType => Some(FieldValue::Integer(ordinal_of(
                &TaskType::ALL,
                TaskType::FixedUnits,
            ))),
            ProjectField::DefaultFixedCostAccrual => {
                Some(FieldValue::Integer(AccrueType::End.value()))
            }
            ProjectField::CriticalSlackLimit => {
                Some(FieldValue::Duration(default_critical_slack_limit()))
            }
            ProjectField::BaselineForEarnedValue => Some(FieldValue::Integer(
                DEFAULT_BASELINE_FOR_EARNED_VALUE,
            )),
            ProjectField::FiscalYearStartMonth => {
                Some(FieldValue::Integer(DEFAULT_FISCAL_YEAR_START_MONTH))
            }
            ProjectField::NewTaskStartIsProjectStart => Some(FieldValue::Boolean(true)),
            ProjectField::NewTasksAreManual => Some(FieldValue::Boolean(true)),
            ProjectField::WeekStartDay => Some(FieldValue::Integer(java_day_of_week(Weekday::Mon))),
            ProjectField::CriticalActivityType => Some(FieldValue::Integer(ordinal_of(
                &CriticalActivityType::ALL,
                CriticalActivityType::TotalFloat,
            ))),
            ProjectField::TotalSlackCalculationType => Some(FieldValue::Integer(ordinal_of(
                &TotalSlackCalculationType::ALL,
                TotalSlackCalculationType::SmallestSlack,
            ))),
            ProjectField::RelationshipLagCalendar => Some(FieldValue::Integer(ordinal_of(
                &RelationshipLagCalendar::ALL,
                RelationshipLagCalendar::Predecessor,
            ))),
            ProjectField::ScheduleFrom => Some(FieldValue::Integer(ScheduleFrom::Start.value())),
            ProjectField::CurrentDate => Some(FieldValue::DateTime(Local::now().naive_local())),
            ProjectField::ThousandsSeparator => Some(FieldValue::String(
                DEFAULT_THOUSANDS_SEPARATOR.to_string(),
            )),
            ProjectField::DecimalSeparator => Some(FieldValue::String(
                DEFAULT_DECIMAL_SEPARATOR.to_string(),
            )),
            ProjectField::MpxDelimiter => {
                Some(FieldValue::String(DEFAULT_MPX_DELIMITER.to_string()))
            }
            ProjectField::WbsCodeSeparator => {
                Some(FieldValue::String(DEFAULT_WBS_CODE_SEPARATOR.to_owned()))
            }
            ProjectField::ConsiderAssignmentsInOtherProjects => Some(FieldValue::Boolean(false)),
            ProjectField::ConsiderAssignmentsInOtherProjectsWithPriorityEqualHigherThan => {
                Some(FieldValue::Integer(DEFAULT_OTHER_PROJECT_ASSIGNMENT_PRIORITY))
            }
            ProjectField::PreserveScheduledEarlyAndLateDates => Some(FieldValue::Boolean(true)),
            ProjectField::LevelAllResources => Some(FieldValue::Boolean(true)),
            ProjectField::LevelResourcesOnlyWithinActivityTotalFloat => {
                Some(FieldValue::Boolean(false))
            }
            ProjectField::PreserveMinimumFloatWhenLeveling => {
                Some(FieldValue::Duration(default_minimum_float()))
            }
            ProjectField::MaxPercentToOverallocateResources => {
                Some(FieldValue::Double(DEFAULT_OVERALLOCATION))
            }
            ProjectField::LevelingPriorities => {
                Some(FieldValue::String(DEFAULT_LEVELING_PRIORITIES.to_owned()))
            }
            ProjectField::DataDateAndPlannedStartSetToProjectForecastStart => {
                Some(FieldValue::Boolean(false))
            }
            ProjectField::IgnoreRelationshipsToAndFromOtherProjects => {
                Some(FieldValue::Boolean(false))
            }
            ProjectField::MakeOpenEndedActivitiesCritical => Some(FieldValue::Boolean(false)),
            ProjectField::UseExpectedFinishDates => Some(FieldValue::Boolean(true)),
            ProjectField::ComputeStartToStartLagFromEarlyStart => Some(FieldValue::Boolean(true)),
            ProjectField::CalculateFloatBasedOnFinishDateOfEachProject => {
                Some(FieldValue::Boolean(true))
            }
            ProjectField::CalculateMultipleFloatPaths => Some(FieldValue::Boolean(false)),
            ProjectField::CalculateMultipleFloatPathsUsingTotalFloat => {
                Some(FieldValue::Boolean(true))
            }
            ProjectField::LimitNumberOfFloatPathsToCalculate => Some(FieldValue::Boolean(true)),
            ProjectField::MaximumNumberOfFloatPathsToCalculate => {
                Some(FieldValue::Integer(DEFAULT_FLOAT_PATHS))
            }
            ProjectField::SchedulingProgressedActivities => Some(FieldValue::Integer(ordinal_of(
                &SchedulingProgressedActivities::ALL,
                SchedulingProgressedActivities::RetainedLogic,
            ))),
            ProjectField::ActivityIdPrefix => {
                Some(FieldValue::String(DEFAULT_ACTIVITY_ID_PREFIX.to_owned()))
            }
            ProjectField::ActivityIdSuffix => Some(FieldValue::Integer(DEFAULT_ACTIVITY_ID_SUFFIX)),
            ProjectField::ActivityIdIncrement => {
                Some(FieldValue::Integer(DEFAULT_ACTIVITY_ID_INCREMENT))
            }
            ProjectField::ActivityIdIncrementBasedOnSelectedActivity => {
                Some(FieldValue::Boolean(true))
            }
            ProjectField::ProjectIsBaseline => Some(FieldValue::Boolean(false)),
            ProjectField::ActivityPercentCompleteBasedOnActivitySteps => {
                Some(FieldValue::Boolean(false))
            }
// Java's defaults for CUSTOM_PROPERTIES / PROJECT_CODE_VALUES are empty maps, which cannot be expressed
// with `FieldValue`; no calculation method is registered for the remaining fields.
            _ => None,
        }
    }

/// Get the duration unit default from the project context (corresponds to the Java calculated field's
    /// `p.m_parentFile.getProjectContext().getTimeUnitDefaults().getXxx()`).
    fn time_unit_default_value(
        &self,
        load: impl FnOnce(&TimeUnitDefaults) -> Option<i32>,
    ) -> Option<i32> {
        let context = self.parent_context()?;
        let context_ref = context.try_borrow().ok()?;
        load(context_ref.get_time_unit_defaults())
    }

/// Project earliest start date (Corresponds to Java: `m_parentFile.getEarliestStartDate()`).
    fn calculate_start_date(&self) -> Option<FieldValue> {
        let parent = self.parent_file.upgrade()?;
        let parent_ref = parent.try_borrow().ok()?;
        parent_ref
            .get_earliest_start_date()
            .map(FieldValue::DateTime)
    }

/// Project latest finish date (Corresponds to Java: `m_parentFile.getLatestFinishDate()`).
    fn calculate_finish_date(&self) -> Option<FieldValue> {
        let parent = self.parent_file.upgrade()?;
        let parent_ref = parent.try_borrow().ok()?;
        parent_ref.get_latest_finish_date().map(FieldValue::DateTime)
    }

/// Project actual start date (Corresponds to Java: `m_parentFile.getActualStart()`).
    fn calculate_actual_start(&self) -> Option<FieldValue> {
        let parent = self.parent_file.upgrade()?;
        let parent_ref = parent.try_borrow().ok()?;
        parent_ref.get_actual_start().map(FieldValue::DateTime)
    }

/// Project actual finish date (Corresponds to Java: `m_parentFile.getActualFinish()`).
    fn calculate_actual_finish(&self) -> Option<FieldValue> {
        let parent = self.parent_file.upgrade()?;
        let parent_ref = parent.try_borrow().ok()?;
        parent_ref.get_actual_finish().map(FieldValue::DateTime)
    }

/// Get a value from the single child task (corresponds to Java's `getValueFromSummaryTask(TaskField field)`).
    ///
/// Java comment: `TODO: implement dummy project summary task to allow values to be rolled up`.
/// Returns `None` when the number of child tasks is not 1, the parent file is unavailable, or there is a borrow conflict (Java returns `null`).
    fn get_value_from_summary_task(&self, field: TaskField) -> Option<FieldValue> {
        let parent = self.parent_file.upgrade()?;
        let parent_ref = parent.try_borrow().ok()?;
        let child_tasks = parent_ref.get_child_tasks();
        if child_tasks.len() != 1 {
            return None;
        }

        let task = child_tasks.first()?;
        let task_ref = task.try_borrow().ok()?;
        task_ref.get(FieldKey::Task(field))
    }
}

impl ProjectProperties {
/// Return the fields that hold a non-default cached value in the project properties.
    ///
/// Corresponds to Java: `public Set<FieldType> getPopulatedFields()`; custom fields are passed in by the parent file's
/// caller so that a borrowed project file is not borrowed again in reverse.
    pub fn get_populated_fields(
        &self,
        custom: impl IntoIterator<Item = FieldKey>,
    ) -> std::collections::HashSet<FieldKey> {
        let defaults = crate::common::populated_fields::Defaults::from_project_properties(self);
        crate::common::populated_fields::merge_fields(
            crate::project_field::ProjectField::ALL.map(FieldKey::Project), custom,
        )
        .into_iter()
        .filter(|field| crate::common::populated_fields::is_populated(self, *field, defaults))
        .collect()
    }
}

impl FieldContainer for ProjectProperties {
/// Set a field value and forward the change to Java's `handleFieldChange`.
    ///
    /// Corresponds to Java: `@Override public void set(FieldType field, Object value)`
    fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
        let parent_file = self.parent_file.clone();
        self.fields.update(field, value, move |field, old_value, new_value| {
            handle_field_change(&parent_file, field, old_value, new_value);
        });
    }

/// Read a field value (including calculated fields).
    ///
    /// Corresponds to Java: `@Override public Object get(FieldType field)`
    ///
/// Java writes the calculation result back to the field table; on the Rust side `&self` cannot write back, so see the file header notes.
    fn get(&self, field: FieldKey) -> Option<FieldValue> {
        if !is_always_calculated_field(field) {
            if let Some(value) = self.fields.get_cached_value(field) {
                return Some(value.clone());
            }
        }

        self.calculate_field(field)
    }

/// Read only the stored field value without triggering a calculation.
    ///
    /// Corresponds to Java: `@Override public Object getCachedValue(FieldType field)`
    fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
        self.fields.get_cached_value(field)
    }

/// Get timephased data expressed as durations.
    ///
    /// Corresponds to Java: `@Override public List<Duration> getTimephasedDurationValues(FieldType field, List<LocalDateTimeRange> ranges, TimeUnit units)`
    ///
/// Java's implementation always returns a list of all `null` (project-level fields have no timephased data),
/// consistent with the [`FieldContainer`] default implementation; it is explicitly overridden here to match Java.
    fn get_timephased_duration_values(
        &self,
        field: FieldKey,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        let _ = (field, units);
        vec![None; ranges.len()]
    }

/// Get timephased data expressed as numeric values.
    ///
    /// Corresponds to Java: `@Override public List<Number> getTimephasedNumericValues(FieldType field, List<LocalDateTimeRange> ranges)`
    ///
/// Same as [`FieldContainer::get_timephased_duration_values`]; Java always returns a list of all `null`.
    fn get_timephased_numeric_values(
        &self,
        field: FieldKey,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        let _ = field;
        vec![None; ranges.len()]
    }

/// Register a field change listener.
    ///
    /// Corresponds to Java: `@Override public void addFieldListener(FieldListener listener)`
    fn add_field_listener(&mut self, listener: Box<dyn FieldListener>) {
        self.fields.add_field_listener(listener);
    }

/// Clear the field change listeners.
    ///
    /// Corresponds to Java: `@Override public void removeFieldListener(FieldListener listener)`
    ///
/// Java removes a listener by object identity, which Rust cannot express; by trait convention this clears the listeners instead (see the file header notes).
    fn clear_field_listeners(&mut self) {
        self.fields.clear_field_listeners();
    }
}

impl TimeUnitDefaultsContainer for ProjectProperties {
    /// Corresponds to Java: `@Override public Integer getMinutesPerDay()`
    fn get_minutes_per_day(&self) -> Option<i32> {
        ProjectProperties::get_minutes_per_day(self)
    }

    /// Corresponds to Java: `@Override public Integer getMinutesPerWeek()`
    fn get_minutes_per_week(&self) -> Option<i32> {
        ProjectProperties::get_minutes_per_week(self)
    }

    /// Corresponds to Java: `@Override public Integer getMinutesPerMonth()`
    fn get_minutes_per_month(&self) -> Option<i32> {
        ProjectProperties::get_minutes_per_month(self)
    }

    /// Corresponds to Java: `@Override public Integer getMinutesPerYear()`
    fn get_minutes_per_year(&self) -> Option<i32> {
        ProjectProperties::get_minutes_per_year(self)
    }

    /// Corresponds to Java: `@Override public Integer getDaysPerMonth()`
    fn get_days_per_month(&self) -> Option<i32> {
        ProjectProperties::get_days_per_month(self)
    }
}

/// Upgrade the parent file reference and get its project context (returns `None` when the parent file has been dropped or there is a parent file borrow conflict).
fn parent_context_of(parent_file: &Weak<RefCell<ProjectFile>>) -> Option<Rc<RefCell<ProjectContext>>> {
    let parent = parent_file.upgrade()?;
    let parent_ref = parent.try_borrow().ok()?;
    Some(parent_ref.get_project_context())
}

/// Field change handling, corresponding to Java's `@Override void handleFieldChange(FieldType field, Object oldValue, Object newValue)`.
///
/// In Java this handler synchronizes the "default calendar unique ID" and the "duration unit defaults" to the container held by the parent file; on the Rust side
/// the same synchronization is done through the parent file's `ProjectContext` (silently skipped when the parent file is unavailable or there is a borrow conflict,
/// which unlike Java's strong references will not panic).
fn handle_field_change(
    parent_file: &Weak<RefCell<ProjectFile>>,
    field: FieldKey,
    _old_value: Option<&FieldValue>,
    new_value: Option<&FieldValue>,
) {
    let FieldKey::Project(project_field) = field else {
        return;
    };

    match project_field {
        ProjectField::DefaultCalendarUniqueId => {
            let Some(context) = parent_context_of(parent_file) else {
                return;
            };
            let Ok(mut context_ref) = context.try_borrow_mut() else {
                return;
            };
            context_ref
                .get_calendars_mut()
                .set_default_calendar_unique_id(new_value.and_then(FieldValue::as_integer));
        }
        ProjectField::DaysPerMonth => {
            let value = new_value.and_then(FieldValue::as_integer);
            with_time_unit_defaults(parent_file, |defaults| {
                defaults.set_days_per_month(value)
            });
        }
        ProjectField::MinutesPerDay => {
            let value = new_value.and_then(FieldValue::as_integer);
            with_time_unit_defaults(parent_file, |defaults| {
                defaults.set_minutes_per_day(value)
            });
        }
        ProjectField::MinutesPerWeek => {
            let value = new_value.and_then(FieldValue::as_integer);
            with_time_unit_defaults(parent_file, |defaults| {
                defaults.set_minutes_per_week(value)
            });
        }
        ProjectField::MinutesPerMonth => {
            let value = new_value.and_then(FieldValue::as_integer);
            with_time_unit_defaults(parent_file, |defaults| {
                defaults.set_minutes_per_month(value)
            });
        }
        ProjectField::MinutesPerYear => {
            let value = new_value.and_then(FieldValue::as_integer);
            with_time_unit_defaults(parent_file, |defaults| {
                defaults.set_minutes_per_year(value)
            });
        }
        _ => {}
    }
}

/// Modify the duration unit defaults on the parent file's project context (silently skipped when the parent file is unavailable or there is a borrow conflict).
fn with_time_unit_defaults(
    parent_file: &Weak<RefCell<ProjectFile>>,
    apply: impl FnOnce(&mut TimeUnitDefaults),
) {
    let Some(context) = parent_context_of(parent_file) else {
        return;
    };
    let Ok(mut context_ref) = context.try_borrow_mut() else {
        return;
    };

    apply(context_ref.get_time_unit_defaults_mut());
}

/// Determine whether a field belongs to Java's `ALWAYS_CALCULATED_FIELDS`.
///
/// Corresponds to Java: `@Override boolean getAlwaysCalculatedField(FieldType field)` and
/// `private static final Set<FieldType> ALWAYS_CALCULATED_FIELDS`
fn is_always_calculated_field(field: FieldKey) -> bool {
    matches!(
        field,
        FieldKey::Project(
            ProjectField::DefaultCalendarUniqueId
                | ProjectField::DaysPerMonth
                | ProjectField::MinutesPerDay
                | ProjectField::MinutesPerWeek
                | ProjectField::MinutesPerMonth
                | ProjectField::MinutesPerYear
        )
    )
}

/// Map a field number to a field (Java's `private ProjectField selectField(ProjectField[] fields, int index)`).
///
/// Java throws `IllegalArgumentException` for an out-of-bounds number; the Rust side returns `None` (no panic).
fn select_field(fields: &[ProjectField], index: i32) -> Option<ProjectField> {
    if index < 1 || index > fields.len() as i32 {
        return None;
    }

    fields.get((index - 1) as usize).copied()
}

/// Get the ordinal of an enum value in Java's declaration order (used for int conversion of enums that do not implement `MpxjEnum`).
fn ordinal_of<T: PartialEq>(all: &[T], value: T) -> i32 {
    all.iter().position(|item| *item == value).unwrap_or(0) as i32
}

/// `DayOfWeek` → Java's 1 (Monday) … 7 (Sunday).
///
/// Corresponds to Java: `DayOfWeek.getValue()`
fn java_day_of_week(day: Weekday) -> i32 {
    day.num_days_from_monday() as i32 + 1
}

/// Java's 1 (Monday) … 7 (Sunday) → [`Weekday`] (returns `None` when out of bounds).
///
/// Corresponds to Java: `DayOfWeek.of(int dayOfWeek)`
fn weekday_from_java_day(value: i32) -> Option<Weekday> {
    u8::try_from(value - 1)
        .ok()
        .and_then(|day| Weekday::try_from(day).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

/// A properties object with no parent file (when the parent file is unavailable each calculated field degrades gracefully to `None` / the Java default).
    fn new_properties() -> ProjectProperties {
        ProjectProperties::new(Weak::new())
    }

    fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .expect("valid date")
            .and_hms_opt(hour, minute, 0)
            .expect("valid time")
    }

/// Create a calendar with a unique ID (a temporary calendar that does not access the context).
    fn calendar(unique_id: i32) -> Rc<RefCell<ProjectCalendar>> {
        let mut calendar = ProjectCalendar::new(Weak::new(), true);
        calendar.set_unique_id(Some(unique_id));
        Rc::new(RefCell::new(calendar))
    }

    #[test]
    fn string_and_numeric_fields_round_trip() {
        let mut properties = new_properties();

        assert_eq!(properties.get_name(), None);
        properties.set_name(Some("Sample".to_owned()));
        properties.set_company(Some("Acme".to_owned()));
        properties.set_manager(Some("Jon".to_owned()));
        properties.set_comments(Some("备注".to_owned()));
        properties.set_cost(Some(1234.5));
        properties.set_percentage_complete(Some(25.0));
        properties.set_unique_id(Some(7));
        assert_eq!(properties.get_name(), Some("Sample".to_owned()));
        assert_eq!(properties.get_company(), Some("Acme".to_owned()));
        assert_eq!(properties.get_manager(), Some("Jon".to_owned()));
        assert_eq!(properties.get_comments(), Some("备注".to_owned()));
        assert_eq!(properties.get_cost(), Some(1234.5));
        assert_eq!(properties.get_percentage_complete(), Some(25.0));
        assert_eq!(properties.get_unique_id(), Some(7));

// Write and then clear
        properties.set_name(None);
        assert_eq!(properties.get_name(), None);
        assert_eq!(
            properties.get_cached_value(FieldKey::Project(ProjectField::Name)),
            None
        );
    }

    #[test]
    fn enum_and_date_fields_round_trip() {
        let mut properties = new_properties();

        properties.set_default_duration_units(Some(TimeUnit::Weeks));
        properties.set_default_work_units(Some(TimeUnit::ElapsedHours));
        properties.set_date_order(Some(DateOrder::Ymd));
        properties.set_time_format(Some(ProjectTimeFormat::TwentyFourHour));
        properties.set_date_format(Some(ProjectDateFormat::DdMmYy));
        properties.set_schedule_from(Some(ScheduleFrom::Finish));
        properties.set_default_task_type(Some(TaskType::FixedWork));
        properties.set_earned_value_method(Some(EarnedValueMethod::PhysicalPercentComplete));
        properties.set_total_slack_calculation_type(Some(TotalSlackCalculationType::StartSlack));
        properties.set_relationship_lag_calendar(Some(RelationshipLagCalendar::Successor));
        properties.set_scheduling_progressed_activities(Some(
            SchedulingProgressedActivities::ActualDates,
        ));
        properties.set_mpx_code_page(Some(CodePage::Zh));
        properties.set_week_start_day(Some(Weekday::Sun));
        properties.set_start_date(Some(date_time(2024, 3, 1, 8, 0)));
        properties.set_default_end_time(Some(NaiveTime::from_hms_opt(17, 0, 0).expect("valid time")));

        assert_eq!(properties.get_default_duration_units(), TimeUnit::Weeks);
        assert_eq!(properties.get_default_work_units(), TimeUnit::ElapsedHours);
        assert_eq!(properties.get_date_order(), DateOrder::Ymd);
        assert_eq!(properties.get_time_format(), ProjectTimeFormat::TwentyFourHour);
        assert_eq!(properties.get_date_format(), ProjectDateFormat::DdMmYy);
        assert_eq!(properties.get_schedule_from(), ScheduleFrom::Finish);
        assert_eq!(properties.get_default_task_type(), TaskType::FixedWork);
        assert_eq!(
            properties.get_earned_value_method(),
            Some(EarnedValueMethod::PhysicalPercentComplete)
        );
        assert_eq!(
            properties.get_total_slack_calculation_type(),
            TotalSlackCalculationType::StartSlack
        );
        assert_eq!(
            properties.get_relationship_lag_calendar(),
            RelationshipLagCalendar::Successor
        );
        assert_eq!(
            properties.get_scheduling_progressed_activities(),
            SchedulingProgressedActivities::ActualDates
        );
        assert_eq!(properties.get_mpx_code_page(), CodePage::Zh);
        assert_eq!(properties.get_week_start_day(), Weekday::Sun);
        assert_eq!(properties.get_start_date(), Some(date_time(2024, 3, 1, 8, 0)));
        assert_eq!(
            properties.get_default_end_time(),
            Some(NaiveTime::from_hms_opt(17, 0, 0).expect("valid time"))
        );
    }

    #[test]
    fn default_values_follow_java_calculated_field_map() {
        let properties = new_properties();

// String / character
        assert_eq!(properties.get_currency_symbol(), "$");
        assert_eq!(properties.get_am_text(), "am");
        assert_eq!(properties.get_pm_text(), "pm");
        assert_eq!(properties.get_mpx_program_name(), "Microsoft Project for Windows");
        assert_eq!(properties.get_wbs_code_separator(), ".");
        assert_eq!(properties.get_leveling_priorities(), "(0||priority_type(sort_type|ASC)())");
        assert_eq!(properties.get_activity_id_prefix(), "A");
        assert_eq!(properties.get_date_separator(), '/');
        assert_eq!(properties.get_time_separator(), ':');
        assert_eq!(properties.get_thousands_separator(), ',');
        assert_eq!(properties.get_decimal_separator(), '.');
        assert_eq!(properties.get_mpx_delimiter(), ',');

// Numeric
        assert_eq!(properties.get_currency_digits(), 2);
        assert_eq!(properties.get_baseline_for_earned_value(), 0);
        assert_eq!(properties.get_fiscal_year_start_month(), 1);
        assert_eq!(
            properties.get_consider_assignments_in_other_projects_with_priority_equal_higher_than(),
            5
        );
        assert_eq!(properties.get_maximum_number_of_float_paths_to_calculate(), 10);
        assert_eq!(properties.get_activity_id_suffix(), 1000);
        assert_eq!(properties.get_activity_id_increment(), 10);
        assert_eq!(properties.get_max_percent_to_overallocate_resources(), 25.0);

// Enum / time / duration
        assert_eq!(properties.get_default_duration_units(), TimeUnit::Days);
        assert_eq!(properties.get_default_work_units(), TimeUnit::Hours);
        assert_eq!(properties.get_date_order(), DateOrder::Dmy);
        assert_eq!(properties.get_time_format(), ProjectTimeFormat::TwelveHour);
        assert_eq!(properties.get_symbol_position(), CurrencySymbolPosition::Before);
        assert_eq!(properties.get_schedule_from(), ScheduleFrom::Start);
        assert_eq!(properties.get_mpx_file_version(), FileVersion::Version4_0);
        assert_eq!(properties.get_mpx_code_page(), CodePage::Ansi);
        assert_eq!(properties.get_week_start_day(), Weekday::Mon);
        assert_eq!(
            properties.get_default_task_earned_value_method(),
            EarnedValueMethod::PercentComplete
        );
        assert_eq!(properties.get_default_task_type(), TaskType::FixedUnits);
        assert_eq!(properties.get_default_fixed_cost_accrual(), AccrueType::End);
        assert_eq!(
            properties.get_critical_activity_type(),
            CriticalActivityType::TotalFloat
        );
        assert_eq!(
            properties.get_total_slack_calculation_type(),
            TotalSlackCalculationType::SmallestSlack
        );
        assert_eq!(
            properties.get_relationship_lag_calendar(),
            RelationshipLagCalendar::Predecessor
        );
        assert_eq!(
            properties.get_scheduling_progressed_activities(),
            SchedulingProgressedActivities::RetainedLogic
        );
        assert_eq!(
            properties.get_default_start_time(),
            NaiveTime::from_hms_opt(8, 0, 0).expect("valid time")
        );
        assert_eq!(
            properties.get_critical_slack_limit(),
            Duration::get_instance_int(0, TimeUnit::Days)
        );
        assert_eq!(
            properties.get_preserve_minimum_float_when_leveling(),
            Duration::get_instance_int(1, TimeUnit::Hours)
        );

// Boolean
        assert!(properties.get_updating_task_status_updates_resource_status());
        assert!(!properties.get_split_in_progress_tasks());
        assert!(properties.get_new_tasks_estimated());
        assert!(properties.get_autolink());
        assert!(properties.get_new_task_start_is_project_start());
        assert!(properties.get_new_tasks_are_manual());
        assert!(properties.get_use_expected_finish_dates());
        assert!(properties.get_activity_id_increment_based_on_selected_activity());
        assert!(!properties.get_project_is_baseline());
// Boolean fields with no default yield false via BooleanHelper.getBoolean(null)
        assert!(!properties.get_spread_percent_complete());
        assert!(!properties.get_honor_constraints());

// Project-level values are unavailable when there is no parent file
        assert_eq!(properties.get_start_date(), None);
        assert_eq!(properties.get_finish_date(), None);
        assert_eq!(properties.get_actual_start(), None);
        assert_eq!(properties.get_actual_finish(), None);
        assert_eq!(properties.get_default_calendar_unique_id(), None);
    }

    #[test]
    fn default_calendar_round_trips_via_weak_reference() {
        let mut properties = new_properties();
        let calendar = calendar(42);

        assert!(properties.get_default_calendar().is_none());

        properties.set_default_calendar(Some(Rc::clone(&calendar)));
        assert_eq!(properties.get_default_calendar_unique_id(), Some(42));
        let resolved = properties.get_default_calendar().expect("should retrieve the default calendar");
        assert!(Rc::ptr_eq(&resolved, &calendar));

// When only the unique ID is set (no calendar object) and there is no parent file, an always-calculated field cannot be resolved (consistent with Java:
// Java likewise ignores the cached value in the field table and resolves it through the parent file's calendar container)
        let mut properties = new_properties();
        properties.set_default_calendar_unique_id(Some(9));
        assert_eq!(properties.get_default_calendar_unique_id(), None);
        assert_eq!(
            properties.get_cached_value(FieldKey::Project(ProjectField::DefaultCalendarUniqueId)),
            Some(&FieldValue::Integer(9))
        );

        properties.set_default_calendar(None);
        assert!(properties.get_default_calendar().is_none());
        assert_eq!(properties.get_default_calendar_unique_id(), None);
    }

    #[test]
    fn activity_default_calendar_round_trips() {
        let mut properties = new_properties();
        let calendar = calendar(5);

        assert!(properties.get_activity_default_calendar().is_none());
        properties.set_activity_default_calendar(Some(Rc::clone(&calendar)));
        assert_eq!(properties.get_activity_default_calendar_unique_id(), Some(5));
        let resolved = properties
            .get_activity_default_calendar()
            .expect("should retrieve the active default calendar");
        assert!(Rc::ptr_eq(&resolved, &calendar));

        properties.set_activity_default_calendar(None);
        assert!(properties.get_activity_default_calendar().is_none());
        assert_eq!(properties.get_activity_default_calendar_unique_id(), None);
    }

    #[test]
    fn time_unit_defaults_are_read_from_parent_context() {
        let mut properties = new_properties();

// No parent file: always-calculated fields cannot be resolved, but the raw value can still be written to the field table
        assert_eq!(properties.get_days_per_month(), None);
        assert_eq!(properties.get_minutes_per_day(), None);
        properties.set_days_per_month(Some(25));
        properties.set_minutes_per_day(Some(600));
        assert_eq!(properties.get_days_per_month(), None);
        assert_eq!(
            properties.get_cached_value(FieldKey::Project(ProjectField::DaysPerMonth)),
            Some(&FieldValue::Integer(25))
        );
        assert_eq!(
            properties.get_cached_value(FieldKey::Project(ProjectField::MinutesPerDay)),
            Some(&FieldValue::Integer(600))
        );

// Used as a trait object: behaves the same as Java's TimeUnitDefaultsContainer
        let container: &dyn TimeUnitDefaultsContainer = &properties;
        assert_eq!(container.get_days_per_month(), None);
        assert_eq!(container.get_minutes_per_day(), None);
        assert_eq!(container.get_minutes_per_week(), None);
        assert_eq!(container.get_minutes_per_month(), None);
        assert_eq!(container.get_minutes_per_year(), None);
    }

    #[test]
    fn currency_and_working_day_settings_round_trip() {
        let mut properties = new_properties();

// Currency settings
        properties.set_currency_symbol(Some("€".to_owned()));
        properties.set_currency_code(Some("EUR".to_owned()));
        properties.set_currency_digits(Some(3));
        properties.set_symbol_position(Some(CurrencySymbolPosition::AfterWithSpace));
        properties.set_thousands_separator(' ');
        properties.set_decimal_separator(',');
        assert_eq!(properties.get_currency_symbol(), "€");
        assert_eq!(properties.get_currency_code(), Some("EUR".to_owned()));
        assert_eq!(properties.get_currency_digits(), 3);
        assert_eq!(
            properties.get_symbol_position(),
            CurrencySymbolPosition::AfterWithSpace
        );
        assert_eq!(properties.get_thousands_separator(), ' ');
        assert_eq!(properties.get_decimal_separator(), ',');

// Working day / fiscal year settings
        properties.set_week_start_day(Some(Weekday::Wed));
        properties.set_fiscal_year_start(true);
        properties.set_fiscal_year_start_month(Some(4));
        assert_eq!(properties.get_week_start_day(), Weekday::Wed);
        assert!(properties.get_fiscal_year_start());
        assert_eq!(properties.get_fiscal_year_start_month(), 4);

        properties.set_week_start_day(Some(Weekday::Mon));
        assert_eq!(properties.get_week_start_day(), Weekday::Mon);

// An out-of-bounds weekday value falls back to the default Monday
        properties.set_field(
            FieldKey::Project(ProjectField::WeekStartDay),
            Some(FieldValue::Integer(99)),
        );
        assert_eq!(properties.get_week_start_day(), Weekday::Mon);
    }

    #[test]
    fn guid_notes_and_custom_properties_round_trip() {
        let mut properties = new_properties();

// The GUID is stored as a string
        let guid = Uuid::from_u128(0x1234_5678_9abc_def0_1234_5678_9abc_def0);
        properties.set_guid(Some(guid));
        assert_eq!(properties.get_guid(), Some(guid));

// Notes are stored uniformly as plain text
        assert_eq!(properties.get_notes(), "");
        assert!(properties.get_notes_object().is_none());
        properties.set_notes(Some("项目笔记".to_owned()));
        assert_eq!(properties.get_notes(), "项目笔记");
        assert_eq!(
            properties.get_notes_object().map(|notes| notes.to_string()),
            Some("项目笔记".to_owned())
        );

        let notes = PlainNotes::new(Some("另一段笔记".to_owned()));
        properties.set_notes_object(Some(&notes));
        assert_eq!(properties.get_notes(), "另一段笔记");
        properties.set_notes_object(None);
        assert_eq!(properties.get_notes(), "");

// Custom properties are carried by a separate field
        assert!(properties.get_custom_properties().is_empty());
        let custom_properties = HashMap::from([
            ("Author".to_owned(), FieldValue::String("Jon".to_owned())),
            ("Revision".to_owned(), FieldValue::Integer(3)),
        ]);
        properties.set_custom_properties(custom_properties);
        assert_eq!(properties.get_custom_properties().len(), 2);
        assert_eq!(
            properties.get_custom_properties().get("Revision"),
            Some(&FieldValue::Integer(3))
        );
        properties
            .get_custom_properties_mut()
            .insert("Checked".to_owned(), FieldValue::Boolean(true));
        assert_eq!(properties.get_custom_properties().len(), 3);
    }

    #[test]
    fn baseline_dates_are_indexed_from_one() {
        let mut properties = new_properties();

        properties.set_baseline_date_for_index(1, Some(date_time(2024, 1, 1, 8, 0)));
        properties.set_baseline_date_for_index(10, Some(date_time(2024, 10, 1, 8, 0)));
        assert_eq!(
            properties.get_baseline_date_for_index(1),
            Some(date_time(2024, 1, 1, 8, 0))
        );
        assert_eq!(
            properties.get_baseline_date_for_index(10),
            Some(date_time(2024, 10, 1, 8, 0))
        );
        assert_eq!(properties.get_baseline_date_for_index(2), None);

// Java throws IllegalArgumentException for an out-of-bounds number; the Rust side returns None / does not write
        assert_eq!(properties.get_baseline_date_for_index(0), None);
        assert_eq!(properties.get_baseline_date_for_index(11), None);
        properties.set_baseline_date_for_index(2, Some(date_time(2024, 2, 2, 8, 0)));
        assert_eq!(
            properties.get_baseline_date_for_index(2),
            Some(date_time(2024, 2, 2, 8, 0))
        );

// The no-argument overload uses the BASELINE_DATE field
        properties.set_baseline_date(Some(date_time(2024, 5, 5, 8, 0)));
        assert_eq!(
            properties.get_baseline_date(),
            Some(date_time(2024, 5, 5, 8, 0))
        );
    }

    #[test]
    fn field_listeners_receive_changes() {
        #[derive(Clone)]
        struct Recorder(Rc<RefCell<Vec<FieldKey>>>);

        impl FieldListener for Recorder {
            fn field_change(
                &mut self,
                field: FieldKey,
                _old_value: Option<&FieldValue>,
                _new_value: Option<&FieldValue>,
            ) {
                self.0.borrow_mut().push(field);
            }
        }

        struct Counter(Rc<Cell<i32>>);

        impl FieldListener for Counter {
            fn field_change(
                &mut self,
                _field: FieldKey,
                _old_value: Option<&FieldValue>,
                _new_value: Option<&FieldValue>,
            ) {
                self.0.set(self.0.get() + 1);
            }
        }

        let log = Rc::new(RefCell::new(Vec::new()));
        let mut properties = new_properties();
        properties.add_field_listener(Box::new(Recorder(Rc::clone(&log))));

        properties.set_name(Some("A".to_owned()));
        properties.set_name(Some("A".to_owned()));
        properties.set_name(None);
        assert_eq!(log.borrow().len(), 2);

        properties.clear_field_listeners();
        properties.set_name(Some("B".to_owned()));
        assert_eq!(log.borrow().len(), 2);

// disableEvents only affects dependent field cleanup and not listeners (consistent with Java)
        let fired = Rc::new(Cell::new(0));
        let mut properties = new_properties();
        properties.add_field_listener(Box::new(Counter(Rc::clone(&fired))));
        properties.disable_events();
        properties.enable_events();
        properties.set_unique_id(Some(1));
        assert_eq!(properties.get_unique_id(), Some(1));
        assert_eq!(fired.get(), 1);
    }

    #[test]
    fn set_and_get_delegate_to_the_field_container() {
        let mut properties = new_properties();

        assert!(properties.fields.is_empty());
        properties.set_field(
            FieldKey::Project(ProjectField::Name),
            Some(FieldValue::String("X".to_owned())),
        );
        assert_eq!(properties.fields.len(), 1);
        assert_eq!(
            properties.get_cached_value(FieldKey::Project(ProjectField::Name)),
            Some(&FieldValue::String("X".to_owned()))
        );

// set(None) removes the field from the field table
        properties.set_field(FieldKey::Project(ProjectField::Name), None);
        assert!(properties.fields.is_empty());
    }

    #[test]
    fn no_parent_file_means_no_project_level_roll_up() {
        let properties = new_properties();

        assert!(properties.get_parent_file().is_none());
// Summary fields (from the single child task) cannot be resolved when there is no parent file
        assert_eq!(properties.get_cost(), None);
        assert_eq!(properties.get_work(), None);
        assert_eq!(properties.get_duration(), None);
        assert_eq!(properties.get_actual_cost(), None);
        assert_eq!(properties.get_baseline_work(), None);
        assert_eq!(properties.get_percentage_complete(), None);
    }

    #[test]
    fn resetting_to_java_defaults_after_explicit_values() {
        let mut properties = new_properties();

        properties.set_currency_digits(Some(0));
        assert_eq!(properties.get_currency_digits(), 0);
        properties.set_currency_digits(None);
        assert_eq!(properties.get_currency_digits(), DEFAULT_CURRENCY_DIGITS);

        properties.set_maximum_number_of_float_paths_to_calculate(Some(3));
        assert_eq!(properties.get_maximum_number_of_float_paths_to_calculate(), 3);
        properties.set_maximum_number_of_float_paths_to_calculate(None);
        assert_eq!(
            properties.get_maximum_number_of_float_paths_to_calculate(),
            DEFAULT_FLOAT_PATHS
        );

        properties.set_critical_slack_limit(Some(Duration::get_instance_int(2, TimeUnit::Days)));
        assert_eq!(
            properties.get_critical_slack_limit(),
            Duration::get_instance_int(2, TimeUnit::Days)
        );
        properties.set_critical_slack_limit(None);
        assert_eq!(
            properties.get_critical_slack_limit(),
            Duration::get_instance_int(0, TimeUnit::Days)
        );
    }
}
