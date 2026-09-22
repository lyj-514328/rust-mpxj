//! Corresponds to Java: `org/mpxj/ProjectCalendar.java`
//!
//! Calendar definition: represents both a base calendar and calendars derived from a base calendar. Defines the working
//! and non-working days of each week; the default calendar defines Monday to Friday as working days.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `ManuallyScheduledTaskCalendar extends ProjectCalendar` and overrides the date ranges | manually scheduled rules are stored in the temporary calendar | the Rust caller holds the concrete calendar shared handle, see the `manually_scheduled_task_calendar` file header |
//! | obtain the calendar ID from `ProjectContext` at construction time | `new_with_unique_id` receives the already-obtained ID | when the context is already borrowed, the caller obtains the ID first to avoid re-entrant borrow failure |
//!
//! # Inheritance flattened
//!
//! Java's `ProjectCalendar extends ProjectCalendarDays` cannot be expressed in Rust.
//! It follows the same approach as [`crate::project_calendar_week`]: [`ProjectCalendar`]
//! holds a [`ProjectCalendarDays`] by field composition and **forwards all of its public methods one by one**
//! (see the "Forwarding ProjectCalendarDays" section at the end of the file). In Java, the overrides of `addCalendarHours` /
//! `removeCalendarHours` (clear the cache first, then call `super`) are implemented directly in this file.
//!
//! # Differences from Java (structural)
//!
//! - **Parent calendar back-reference**: Java uses a strong reference `ProjectCalendar m_parent`; the Rust side, following this project's
//! established practice (see `crate::shift` / `crate::shift_period`), uses
//! `Weak<RefCell<ProjectCalendar>>` to avoid a cycle with the strong reference held in the container that would leak memory;
//! [`ProjectCalendar::get_parent`] returns `None` once the parent calendar has been released. In Java, `setParent`'s
//! `calendar != this` guard (which discards malformed data that sets the calendar as its own parent; see the Java source comment)
//!   "I've seen a malformed MSPDI file which sets the parent calendar to itself")
//! is reproduced by **address identity**: instances of this type always live in `Rc<RefCell<ProjectCalendar>>`
//! (the container only stores that form), so `calendar.as_ptr() == self as *const ProjectCalendar`
//! is equivalent to Java's reference identity (the same style as
//! [`crate::project_calendar_container::ProjectCalendarContainer::remove`], which uses
//! `RefCell::as_ptr` for pointer identity comparison).
//! - **Context back-reference**: `m_context` is likewise changed to `Weak<RefCell<ProjectContext>>`
//! (Java uses a strong reference), and [`ProjectCalendar::get_project_context`] returns
//!   `Option<Rc<RefCell<ProjectContext>>>`.
//! - **`m_startTimeCache`**: Java uses a `WeakHashMap<LocalDate, LocalTime>` as a memory-sensitive
//! cache. The Rust side simplifies it to `RefCell<HashMap<NaiveDate, NaiveTime>>`; wrapping it in [`RefCell`]
//! keeps read-only accessors such as [`ProjectCalendar::get_start_time`] as `&self`
//! (in Java this cache is written from read-only methods). Java also caches `null` results,
//! whereas the Rust `HashMap` holds non-null `NaiveTime` values and only caches hits (equivalent result).
//! - **`m_expandedExceptions`**: Java lazily expands and writes back this list inside a read-only accessor. The Rust side
//! uses `RefCell<Vec<ProjectCalendarException>>` so the relevant accessors remain `&self`.
//! - **`m_weeksSorted` / `sortWorkWeeks`**: Java sorts the work weeks and then performs a binary search. The Rust side
//! changes [`ProjectCalendar::get_work_week`] to a linear scan over `m_workWeeks` (equivalent result),
//! which avoids introducing interior mutability for a read-only method; the `m_weeksSorted` flag is therefore no longer kept.
//! - **Collection return values**: Java's `getCalendarExceptions()` / `getExpandedCalendarExceptions()` /
//! `getWorkWeeks()` return an immutable view of the underlying list. Because the list may live inside a [`RefCell`]
//! on the Rust side, they instead return a **snapshot** (`Vec` / slice `to_vec()`) with the same element semantics.
//! - **Overloads**: Java's `addCalendarException` / `getWork` / `getHours` / `getRanges`
//! and similar methods are overloaded; Rust has no overloading (and same-named methods cannot be distinguished by argument count alone), so a suffix is added per argument type:
//! `_for_day` (`getWork(DayOfWeek)`) / `_for_date` / `_for_date_time` /
//! `_recurring`. Those that keep their original name are: `get_work(LocalDateTime, LocalDateTime, TimeUnit)`,
//!   `get_ranges(LocalDate)`,`get_hours(DayOfWeek)`,`add_calendar_exception(from, to)`.
//! - **Null semantics**: Java dereferences possibly-`null` dates/times in several places (throwing an NPE);
//! the Rust side does not panic and degrades to "skip" or an empty collection; see each method's docs for details.
//! - **`TemporaryCalendar` / `ProjectCalendarHelper`**: this repository **replicates them in place rather than as separate modules**
//! -- see the next section.
//!
//! # `TemporaryCalendar` / `ProjectCalendarHelper` (replicated in place)
//!
//! Java's two classes both live in the `org.mpxj` root package: `TemporaryCalendar` is just a
//! subclass of `ProjectCalendar(context, temporaryCalendar = true)` (with no members besides a constructor),
//! and `ProjectCalendarHelper.mergeExceptions` has only a single call site. This repository therefore **does not create separate modules**
//! and instead replicates them in place:
//!
//! - [`ProjectCalendar::get_expanded_calendar_exceptions_with_work_weeks`] directly constructs
//! an instance of this type with `temporary_calendar = true` (equivalent to `new TemporaryCalendar(m_context)`);
//! - the two overloads of `ProjectCalendarHelper.mergeExceptions` are merged into the private function
//! `merge_exceptions` in this file (Rust has no overloading; `mergeExceptions(target, calendar)` in Java simply
//! forwards to `mergeExceptions(target, source.getCalendarExceptions())`, so there is no need to split it into two functions).
//!
//! # Java -> Rust member mapping
//!
//! | Java | Rust |
//! |---|---|
//! | `public ProjectCalendar(ProjectFile file)` | [`ProjectCalendar::new_in_file`] |
//! | `protected ProjectCalendar(ProjectContext, boolean)` | [`ProjectCalendar::new`] |
//! | `public List<Task> getTasks()` | [`ProjectCalendar::get_tasks`] |
//! | `public List<Resource> getResources()` | [`ProjectCalendar::get_resources`] |
//! | `public int getResourceCount()` | [`ProjectCalendar::get_resource_count`] |
//! | `public List<ProjectCalendar> getDerivedCalendarsForProject(ProjectFile)` | [`ProjectCalendar::get_derived_calendars_for_project`] |
//! | `@Override public Integer getUniqueID()` / `setUniqueID(Integer)` | [`ProjectCalendar::get_unique_id`] / [`ProjectCalendar::set_unique_id`], also exposed to the container through [`ProjectEntityWithUniqueID`] / [`ProjectEntityWithMutableUniqueID`] |
//! | `public void remove()` | [`ProjectCalendar::remove`] (the resource / task calendar reference cleanup that Java performs in the container's `removed` is done by this method after releasing the context borrow; see the container file header) |
//!
//! Java-specific mechanisms (reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`):
//! none appear in this class, so nothing needs to be skipped; this class also has **no** skipped Java methods.
//!
//! # Interface assumptions about `ProjectContext` / `ProjectCalendarContainer`
//!
//! This file uses the following interfaces (all implemented and verified):
//!
//! ```text
//! ProjectContext::get_calendars(&self) -> &ProjectCalendarContainer
//! ProjectContext::get_calendars_mut(&mut self) -> &mut ProjectCalendarContainer
//! ProjectContext::get_time_unit_defaults(&self) -> &TimeUnitDefaults
//! ProjectContext::get_project_config(&self) -> &ProjectConfig
//! ProjectContext (as UniqueIdObjectSequenceProvider):
//!   get_unique_id_object_sequence(&mut self, TypeId) -> Rc<ObjectSequence>
//! (ObjectSequence::get_next(&self) -> i32, consistent with the established porting approach of "using TypeId as the key")
//! ProjectContext::get_projects(&self) -> Vec<Rc<RefCell<ProjectFile>>>
//! ProjectContext::get_resources(&self) -> &ResourceContainer
//! ProjectCalendarContainer::get_default_calendar_unique_id(&self) -> Option<i32>
//! ProjectCalendarContainer::set_default_calendar_unique_id(&mut self, Option<i32>)
//! ProjectCalendarContainer::update_unique_id_for_calendar(&mut self, element, old, new)
//! ProjectCalendarContainer::remove(&mut self, calendar: &ProjectCalendar) -> bool
//! ProjectCalendarContainer::as_slice(&self) -> &[Rc<RefCell<ProjectCalendar>>]
//! ```
//!
//! `ProjectCalendar` implements `ProjectEntityWithUniqueID` and `ProjectEntityWithMutableUniqueID`
//! (see the end of the file), so `ProjectCalendarContainer` can compose `ProjectEntityContainer`
//! and reuse its `fixUniqueIdClashes` / `firstUniqueID` / `getSequenceProvider`.

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt::{self, Display, Formatter};
use std::rc::{Rc, Weak};

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Timelike, Weekday};
use uuid::Uuid;

use crate::calendar_type::CalendarType;
use crate::common::local_date_helper::LocalDateHelper;
use crate::common::local_date_time_helper::LocalDateTimeHelper;
use crate::common::local_time_helper::LocalTimeHelper;
use crate::common::number_helper::NumberHelper;
use crate::day_type::DayType;
use crate::duration::Duration;
use crate::local_time_range::LocalTimeRange;
use crate::project_calendar_days::{ProjectCalendarDays, JAVA_DAY_OF_WEEK_ORDER};
use crate::project_calendar_exception::ProjectCalendarException;
use crate::project_calendar_hours::ProjectCalendarHours;
use crate::project_calendar_week::ProjectCalendarWeek;
use crate::project_context::ProjectContext;
use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
use crate::project_file::ProjectFile;
use crate::recurrence_type::RecurrenceType;
use crate::recurring_data::RecurringData;
use crate::resource::Resource;
use crate::task::Task;
use crate::time_unit::TimeUnit;
use crate::time_unit_defaults_container::TimeUnitDefaultsContainer;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;

/// A calendar definition of working / non-working days.
///
/// Corresponds to Java: `public class ProjectCalendar extends ProjectCalendarDays implements ProjectEntityWithMutableUniqueID, TimeUnitDefaultsContainer`
pub struct ProjectCalendar {
    /// Java's `this` is used to update the container's unique ID index; Rust uses a weak reference to break the cycle.
    self_ref: Weak<RefCell<ProjectCalendar>>,
    /// The effective range rule for manually scheduled tasks; Java overrides `getRanges(LocalDate)` in a subclass.
    manual_schedule: Option<crate::manually_scheduled_task_calendar::ManualSchedule>,
    /// Corresponds to Java's inheritance `extends ProjectCalendarDays` (the inheritance is flattened into field composition)
    days: ProjectCalendarDays,

    /// The parent calendar (Java uses a strong reference; the Rust side uses a weak reference to avoid a reference cycle).
    ///
    /// Corresponds to Java field: `private ProjectCalendar m_parent;`
    parent: Weak<RefCell<ProjectCalendar>>,

    /// The calendar unique ID.
    ///
    /// Corresponds to Java field: `private Integer m_uniqueID;`
    unique_id: Option<i32>,

    /// The calendar GUID.
    ///
    /// Corresponds to Java field: `private UUID m_guid;`
    guid: Option<Uuid>,

    /// The list of exception dates.
    ///
    /// Corresponds to Java field: `private final List<ProjectCalendarException> m_exceptions = new ArrayList<>();`
    exceptions: Vec<ProjectCalendarException>,

    /// The list of exception dates (including the expansion of recurring exceptions, generated lazily).
    ///
    /// Corresponds to Java field: `private final List<ProjectCalendarException> m_expandedExceptions = new ArrayList<>();`
    expanded_exceptions: RefCell<Vec<ProjectCalendarException>>,

    /// Whether the exception list has been sorted.
    ///
    /// Corresponds to Java field: `private boolean m_exceptionsSorted;`
    exceptions_sorted: bool,

    /// Start time cache (Java uses a `WeakHashMap` as a memory-sensitive cache; see the file header).
    ///
    /// Corresponds to Java field: `private final Map<LocalDate, LocalTime> m_startTimeCache = new WeakHashMap<>();`
    start_time_cache: RefCell<HashMap<NaiveDate, NaiveTime>>,

    /// The start date-time of the last `get_date` call (cached, to skip repeated computation).
    ///
    /// Corresponds to Java field: `private LocalDateTime m_getDateLastStartDate;`
    get_date_last_start_date: Option<NaiveDateTime>,

    /// The remaining milliseconds of the last `get_date` call.
    ///
    /// Corresponds to Java field: `private long m_getDateLastRemainingMilliseconds;`
    get_date_last_remaining_milliseconds: i64,

    /// The result of the last `get_date` call.
    ///
    /// Corresponds to Java field: `private LocalDateTime m_getDateLastResult;`
    get_date_last_result: Option<NaiveDateTime>,

    /// The list of work week definitions.
    ///
    /// Corresponds to Java field: `private final ArrayList<ProjectCalendarWeek> m_workWeeks = new ArrayList<>();`
    work_weeks: Vec<ProjectCalendarWeek>,

    /// Minutes per day (the value explicitly set on this calendar).
    ///
    /// Corresponds to Java field: `private Integer m_calendarMinutesPerDay;`
    calendar_minutes_per_day: Option<i32>,

    /// Minutes per week (the value explicitly set on this calendar).
    ///
    /// Corresponds to Java field: `private Integer m_calendarMinutesPerWeek;`
    calendar_minutes_per_week: Option<i32>,

    /// Minutes per month (the value explicitly set on this calendar).
    ///
    /// Corresponds to Java field: `private Integer m_calendarMinutesPerMonth;`
    calendar_minutes_per_month: Option<i32>,

    /// Minutes per year (the value explicitly set on this calendar).
    ///
    /// Corresponds to Java field: `private Integer m_calendarMinutesPerYear;`
    calendar_minutes_per_year: Option<i32>,

    /// The calendar type.
    ///
    /// Corresponds to Java field: `private CalendarType m_type = CalendarType.GLOBAL;`
    r#type: CalendarType,

    /// Whether this is a personal calendar.
    ///
    /// Corresponds to Java field: `private boolean m_personal;`
    personal: bool,

    /// The unique ID of the owning project (`None` when not a project-specific calendar).
    ///
    /// Corresponds to Java field: `private Integer m_projectUniqueID;`
    project_unique_id: Option<i32>,

    /// Whether this is a temporary calendar.
    ///
    /// Corresponds to Java field: `private final boolean m_temporaryCalendar;`
    temporary_calendar: bool,

    /// The parent context (Java uses a strong reference; the Rust side uses a weak reference to avoid a reference cycle).
    ///
    /// Corresponds to Java field: `private final ProjectContext m_context;`
    context: Weak<RefCell<ProjectContext>>,
}

impl ProjectCalendar {
    /// Internal constructor.
    ///
    /// Corresponds to Java: `protected ProjectCalendar(ProjectContext context, boolean temporaryCalendar)`
    ///
    /// When this is not a temporary calendar, the context exists, and `ProjectConfig.getAutoCalendarUniqueID()` is true,
    /// takes the next value from the context's unique ID sequence (corresponds to Java's `setUniqueID(...)`;
    /// the calendar has not yet been added to the container during construction, so the value is assigned directly with the same final effect).
    pub fn new(context: Weak<RefCell<ProjectContext>>, temporary_calendar: bool) -> ProjectCalendar {
        let mut unique_id = None;

        if !temporary_calendar {
            if let Some(context_rc) = context.upgrade() {
                let auto_calendar_unique_id = match context_rc.try_borrow() {
                    Ok(context_ref) => {
                        context_ref.get_project_config().get_auto_calendar_unique_id()
                    }
                    Err(_) => false,
                };

                if auto_calendar_unique_id {
                    if let Ok(mut context_ref) = context_rc.try_borrow_mut() {
                        unique_id = Some(
                            context_ref
                                .get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>())
                                .get_next(),
                        );
                    }
                }
            }
        }

        Self::new_with_unique_id(context, temporary_calendar, unique_id)
    }

    /// Construct a calendar with a pre-obtained ID when the caller already holds a borrow of the context.
    ///
    /// Corresponds to Java: `protected ProjectCalendar(ProjectContext context, boolean temporaryCalendar)`
    /// the step of automatically generating a unique ID; on the Rust side this step is performed by the caller that holds the context borrow.
    pub(crate) fn new_with_unique_id(
        context: Weak<RefCell<ProjectContext>>,
        temporary_calendar: bool,
        unique_id: Option<i32>,
    ) -> ProjectCalendar {
        ProjectCalendar {
            self_ref: Weak::new(),
            manual_schedule: None,
            days: ProjectCalendarDays::new(),
            parent: Weak::new(),
            unique_id,
            guid: None,
            exceptions: Vec::new(),
            expanded_exceptions: RefCell::new(Vec::new()),
            exceptions_sorted: false,
            start_time_cache: RefCell::new(HashMap::new()),
            get_date_last_start_date: None,
            get_date_last_remaining_milliseconds: 0,
            get_date_last_result: None,
            work_weeks: Vec::new(),
            calendar_minutes_per_day: None,
            calendar_minutes_per_week: None,
            calendar_minutes_per_month: None,
            calendar_minutes_per_year: None,
            r#type: CalendarType::Global,
            personal: false,
            project_unique_id: None,
            temporary_calendar,
            context,
        }
    }

    /// Install the date range calculation rule for manual scheduling.
    ///
    /// Corresponds to Java: `ManuallyScheduledTaskCalendar extends ProjectCalendar` and overrides
    /// `getRanges(LocalDate)`; Rust's shared handle is fixed to `ProjectCalendar`, so the overriding rule
    /// is stored in the temporary calendar. Called only by the wrapper constructor.
    pub(crate) fn set_manual_schedule(
        &mut self,
        schedule: crate::manually_scheduled_task_calendar::ManualSchedule,
    ) {
        self.manual_schedule = Some(schedule);
    }

    /// Wire up the self handle for the `this` callback in Java's `setUniqueID`.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.updateUniqueID(this, oldID, newID)`.
    pub(crate) fn set_self_ref(&mut self, self_ref: Weak<RefCell<ProjectCalendar>>) {
        self.self_ref = self_ref;
    }

    /// Get the parent `ProjectContext` (returns `None` when the context has been released).
    ///
    /// Corresponds to Java: `public ProjectContext getProjectContext()`
    pub fn get_project_context(&self) -> Option<Rc<RefCell<ProjectContext>>> {
        self.context.upgrade()
    }

    /// Construct a calendar from the parent project file.
    ///
    /// Corresponds to Java: `public ProjectCalendar(ProjectFile file)`
    /// (Java is `this(file.getProjectContext(), false)`)
    ///
    /// When the parent file is already borrowed (which cannot happen in Java), degrades to a calendar without a context and is equivalent to `new`.
    pub fn new_in_file(file: &Rc<RefCell<ProjectFile>>) -> ProjectCalendar {
        let context = match file.try_borrow() {
            Ok(file_ref) => Rc::downgrade(&file_ref.get_project_context()),
            Err(_) => Weak::new(),
        };

        ProjectCalendar::new(context, false)
    }

    /// Determine whether this calendar is the default calendar of a project file / database.
    ///
    /// Corresponds to Java: `public boolean getDefault()`
    pub fn get_default(&self) -> bool {
        if self.temporary_calendar {
            return false;
        }

        let Some(context) = self.context.upgrade() else {
            return false;
        };
        let Ok(context_ref) = context.try_borrow() else {
            return false;
        };

        let default_calendar_unique_id = context_ref
            .get_calendars()
            .get_default_calendar_unique_id();
        default_calendar_unique_id.is_some() && default_calendar_unique_id == self.unique_id
    }

    /// Set this calendar as the default calendar of a project file / database.
    ///
    /// Corresponds to Java: `public void setDefault()`
    pub fn set_default(&self) {
        let Some(context) = self.context.upgrade() else {
            return;
        };
        let Ok(mut context_ref) = context.try_borrow_mut() else {
            return;
        };

        context_ref
            .get_calendars_mut()
            .set_default_calendar_unique_id(self.unique_id);
    }

    /// Get the effective minutes per day for this calendar: first the explicit value on this calendar, then the parent calendar, and finally the context default.
    ///
    /// Corresponds to Java: `@Override public Integer getMinutesPerDay()`
    pub fn get_minutes_per_day(&self) -> Option<i32> {
        if let Some(result) = self.calendar_minutes_per_day {
            return Some(result);
        }

        if let Some(parent) = self.parent.upgrade() {
            return match parent.try_borrow() {
                Ok(parent_ref) => parent_ref.get_minutes_per_day(),
                Err(_) => None,
            };
        }

        let context = self.context.upgrade()?;
        let Ok(context_ref) = context.try_borrow() else {
            return None;
        };
        context_ref.get_time_unit_defaults().get_minutes_per_day()
    }

    /// Get the effective minutes per week for this calendar: first the explicit value on this calendar, then the parent calendar, and finally the context default.
    ///
    /// Corresponds to Java: `@Override public Integer getMinutesPerWeek()`
    pub fn get_minutes_per_week(&self) -> Option<i32> {
        if let Some(result) = self.calendar_minutes_per_week {
            return Some(result);
        }

        if let Some(parent) = self.parent.upgrade() {
            return match parent.try_borrow() {
                Ok(parent_ref) => parent_ref.get_minutes_per_week(),
                Err(_) => None,
            };
        }

        let context = self.context.upgrade()?;
        let Ok(context_ref) = context.try_borrow() else {
            return None;
        };
        context_ref.get_time_unit_defaults().get_minutes_per_week()
    }

    /// Get the effective minutes per month for this calendar: first the explicit value on this calendar, then the parent calendar, and finally the context default.
    ///
    /// Corresponds to Java: `@Override public Integer getMinutesPerMonth()`
    pub fn get_minutes_per_month(&self) -> Option<i32> {
        if let Some(result) = self.calendar_minutes_per_month {
            return Some(result);
        }

        if let Some(parent) = self.parent.upgrade() {
            return match parent.try_borrow() {
                Ok(parent_ref) => parent_ref.get_minutes_per_month(),
                Err(_) => None,
            };
        }

        let context = self.context.upgrade()?;
        let Ok(context_ref) = context.try_borrow() else {
            return None;
        };
        context_ref.get_time_unit_defaults().get_minutes_per_month()
    }

    /// Get the effective minutes per year for this calendar: first the explicit value on this calendar, then the parent calendar, and finally the context default.
    ///
    /// Corresponds to Java: `@Override public Integer getMinutesPerYear()`
    pub fn get_minutes_per_year(&self) -> Option<i32> {
        if let Some(result) = self.calendar_minutes_per_year {
            return Some(result);
        }

        if let Some(parent) = self.parent.upgrade() {
            return match parent.try_borrow() {
                Ok(parent_ref) => parent_ref.get_minutes_per_year(),
                Err(_) => None,
            };
        }

        let context = self.context.upgrade()?;
        let Ok(context_ref) = context.try_borrow() else {
            return None;
        };
        context_ref.get_time_unit_defaults().get_minutes_per_year()
    }

    /// Get the number of days per month.
    ///
    /// Corresponds to Java: `@Override public Integer getDaysPerMonth()`
    ///
    /// Java comment: this value is not currently stored in the calendar; it is read directly from the project properties.
    pub fn get_days_per_month(&self) -> Option<i32> {
        let context = self.context.upgrade()?;
        let Ok(context_ref) = context.try_borrow() else {
            return None;
        };
        context_ref.get_time_unit_defaults().get_days_per_month()
    }

    /// Set the explicit minutes per day for this calendar.
    ///
    /// Corresponds to Java: `public void setCalendarMinutesPerDay(Integer minutes)`
    pub fn set_calendar_minutes_per_day(&mut self, minutes: Option<i32>) {
        self.calendar_minutes_per_day = minutes;
    }

    /// Get the explicitly set minutes per day for this calendar.
    ///
    /// Corresponds to Java: `public Integer getCalendarMinutesPerDay()`
    pub fn get_calendar_minutes_per_day(&self) -> Option<i32> {
        self.calendar_minutes_per_day
    }

    /// Set the explicit minutes per week for this calendar.
    ///
    /// Corresponds to Java: `public void setCalendarMinutesPerWeek(Integer minutes)`
    pub fn set_calendar_minutes_per_week(&mut self, minutes: Option<i32>) {
        self.calendar_minutes_per_week = minutes;
    }

    /// Get the explicitly set minutes per week for this calendar.
    ///
    /// Corresponds to Java: `public Integer getCalendarMinutesPerWeek()`
    pub fn get_calendar_minutes_per_week(&self) -> Option<i32> {
        self.calendar_minutes_per_week
    }

    /// Set the explicit minutes per month for this calendar.
    ///
    /// Corresponds to Java: `public void setCalendarMinutesPerMonth(Integer minutes)`
    pub fn set_calendar_minutes_per_month(&mut self, minutes: Option<i32>) {
        self.calendar_minutes_per_month = minutes;
    }

    /// Get the explicitly set minutes per month for this calendar.
    ///
    /// Corresponds to Java: `public Integer getCalendarMinutesPerMonth()`
    pub fn get_calendar_minutes_per_month(&self) -> Option<i32> {
        self.calendar_minutes_per_month
    }

    /// Set the explicit minutes per year for this calendar.
    ///
    /// Corresponds to Java: `public void setCalendarMinutesPerYear(Integer minutes)`
    pub fn set_calendar_minutes_per_year(&mut self, minutes: Option<i32>) {
        self.calendar_minutes_per_year = minutes;
    }

    /// Get the explicitly set minutes per year for this calendar.
    ///
    /// Corresponds to Java: `public Integer getCalendarMinutesPerYear()`
    pub fn get_calendar_minutes_per_year(&self) -> Option<i32> {
        self.calendar_minutes_per_year
    }

    /// Add an empty work week and return a mutable reference to it.
    ///
    /// Corresponds to Java: `public ProjectCalendarWeek addWorkWeek()`
    pub fn add_work_week(&mut self) -> &mut ProjectCalendarWeek {
        self.work_weeks.push(ProjectCalendarWeek::new());
        // Java also sets m_weeksSorted to false; the Rust side uses a linear scan instead (see the file header).
        let index = self.work_weeks.len() - 1;
        self.clear_working_date_cache();
        &mut self.work_weeks[index]
    }

    /// Remove a work week from the calendar (removes the first match using `ProjectCalendarWeek` equality semantics).
    ///
    /// Corresponds to Java: `public void removeWorkWeek(ProjectCalendarWeek week)`
    pub fn remove_work_week(&mut self, week: &ProjectCalendarWeek) {
        if let Some(index) = self.work_weeks.iter().position(|item| item == week) {
            self.work_weeks.remove(index);
        }
        self.clear_working_date_cache();
    }

    /// Clear all work weeks.
    ///
    /// Corresponds to Java: `public void clearWorkWeeks()`
    pub fn clear_work_weeks(&mut self) {
        self.work_weeks.clear();
        self.clear_working_date_cache();
    }

    /// Get all work weeks (Java returns an immutable view; Rust returns a slice).
    ///
    /// Corresponds to Java: `public List<ProjectCalendarWeek> getWorkWeeks()`
    pub fn get_work_weeks(&self) -> &[ProjectCalendarWeek] {
        &self.work_weeks
    }

    /// Add a single-day exception date.
    ///
    /// Corresponds to Java: `public ProjectCalendarException addCalendarException(LocalDate date)`
    ///
    /// Rust has no overloading, so the `_for_date` suffix is added.
    pub fn add_calendar_exception_for_date(
        &mut self,
        date: Option<NaiveDate>,
    ) -> &mut ProjectCalendarException {
        self.add_calendar_exception_impl(date, date, None)
    }

    /// Add an exception for a start-end date range.
    ///
    /// Corresponds to Java: `public ProjectCalendarException addCalendarException(LocalDate fromDate, LocalDate toDate)`
    pub fn add_calendar_exception(
        &mut self,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
    ) -> &mut ProjectCalendarException {
        self.add_calendar_exception_impl(from_date, to_date, None)
    }

    /// Add a recurring exception.
    ///
    /// Corresponds to Java: `public ProjectCalendarException addCalendarException(RecurringData recurringData)`
    ///
    /// Rust has no overloading, so the `_recurring` suffix is added.
    pub fn add_calendar_exception_recurring(
        &mut self,
        recurring: RecurringData,
    ) -> &mut ProjectCalendarException {
        self.add_calendar_exception_impl(None, None, Some(recurring))
    }

    /// Internal implementation for adding a normal / recurring exception.
    ///
    /// Corresponds to Java: `private ProjectCalendarException addCalendarException(LocalDate fromDate, LocalDate toDate, RecurringData recurringData)`
    fn add_calendar_exception_impl(
        &mut self,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
        recurring: Option<RecurringData>,
    ) -> &mut ProjectCalendarException {
        self.exceptions
            .push(ProjectCalendarException::new_with_recurring(
                from_date, to_date, recurring,
            ));
        let index = self.exceptions.len() - 1;

        self.expanded_exceptions.borrow_mut().clear();
        self.exceptions_sorted = false;
        self.clear_working_date_cache();

        &mut self.exceptions[index]
    }

    /// Remove an exception (removes the first match using `ProjectCalendarException` equality semantics).
    ///
    /// Corresponds to Java: `public void removeCalendarException(ProjectCalendarException exception)`
    ///
    /// Note that Java's `List.remove(Object)` uses `equals`, and `ProjectCalendarException`'s
    /// `equals` comes from `AbstractList` (comparing only the working hours content); the Rust side stays consistent.
    pub fn remove_calendar_exception(&mut self, exception: &ProjectCalendarException) {
        if let Some(index) = self.exceptions.iter().position(|item| item == exception) {
            self.exceptions.remove(index);
        }
        self.expanded_exceptions.borrow_mut().clear();
        self.clear_working_date_cache();
    }

    /// Clear all exceptions.
    ///
    /// Corresponds to Java: `public void clearCalendarExceptions()`
    pub fn clear_calendar_exceptions(&mut self) {
        self.exceptions.clear();
        self.expanded_exceptions.borrow_mut().clear();
        self.exceptions_sorted = false;
        self.clear_working_date_cache();
    }

    /// Get all exceptions (recurring exceptions are returned as a single entry with its recurrence rule), sorted by start date.
    ///
    /// Corresponds to Java: `public List<ProjectCalendarException> getCalendarExceptions()`
    ///
    /// Java returns an immutable view and sorts internally; the Rust side sorts and then returns a slice.
    pub fn get_calendar_exceptions(&mut self) -> &[ProjectCalendarException] {
        self.sort_exceptions();
        &self.exceptions
    }

    /// Get the expanded exception list (recurring exceptions replaced by an explicit exception for each occurrence).
    ///
    /// Corresponds to Java: `public List<ProjectCalendarException> getExpandedCalendarExceptions()`
    ///
    /// Java returns an immutable view; the Rust side returns a snapshot (see the file header).
    pub fn get_expanded_calendar_exceptions(&self) -> Vec<ProjectCalendarException> {
        self.populate_expanded_exceptions();
        match self.expanded_exceptions.try_borrow() {
            Ok(expanded) => expanded.clone(),
            Err(_) => Vec::new(),
        }
    }

    /// Expand the exceptions and also convert the work weeks defined on this calendar into exceptions.
    ///
    /// Corresponds to Java: `public List<ProjectCalendarException> getExpandedCalendarExceptionsWithWorkWeeks()`
    ///
    /// Java implements this via `TemporaryCalendar` + `ProjectCalendarHelper.mergeExceptions`;
    /// both are replicated in place in this repository (not as separate modules), and here a temporary calendar is constructed and
    /// merged via the `merge_exceptions` function in this file (see the file header).
    pub fn get_expanded_calendar_exceptions_with_work_weeks(
        &mut self,
    ) -> Vec<ProjectCalendarException> {
        if self.work_weeks.is_empty() {
            return self.get_expanded_calendar_exceptions();
        }

        let mut temporary_calendar = ProjectCalendar::new(self.context.clone(), true);

        let source_exceptions = self.get_calendar_exceptions().to_vec();
        merge_exceptions(&mut temporary_calendar, &source_exceptions);

        for week in &self.work_weeks {
            if let Some(exceptions) = week.convert_to_recurring_exceptions(None, None) {
                merge_exceptions(&mut temporary_calendar, &exceptions);
            }
        }

        temporary_calendar.get_expanded_calendar_exceptions()
    }

    /// Add (replace with) a new empty set of working hours for the specified day of the week and return a mutable reference to it.
    ///
    /// Corresponds to Java: `@Override public ProjectCalendarHours addCalendarHours(DayOfWeek day)`
    pub fn add_calendar_hours(&mut self, day: Weekday) -> &mut ProjectCalendarHours {
        self.clear_working_date_cache();
        self.days.add_calendar_hours(day)
    }

    /// Remove the working hours for the specified day of the week (set to `null`).
    ///
    /// Corresponds to Java: `@Override public void removeCalendarHours(DayOfWeek day)`
    pub fn remove_calendar_hours(&mut self, day: Weekday) {
        self.clear_working_date_cache();
        self.days.remove_calendar_hours(day);
    }

    /// Set the base calendar from which this calendar is derived.
    ///
    /// Corresponds to Java: `public void setParent(ProjectCalendar calendar)`
    ///
    /// Java uses `calendar != this` to ignore malformed data that sets the calendar as its own parent; the Rust side reproduces this by address identity
    /// (see the file header). When `self` is not yet inside an `Rc<RefCell<ProjectCalendar>>`
    /// (for example, not yet added to the container), there is nothing to compare, so it is set directly, consistent with Java.
    pub fn set_parent(&mut self, calendar: Option<Rc<RefCell<ProjectCalendar>>>) {
        if let Some(candidate) = &calendar {
            // Java: if (calendar != this)
            let is_self = candidate.as_ptr() as *const ProjectCalendar == self as *const ProjectCalendar;
            if is_self {
                return;
            }
        }

        self.parent = match &calendar {
            Some(calendar) => Rc::downgrade(calendar),
            None => Weak::new(),
        };

        for day in JAVA_DAY_OF_WEEK_ORDER {
            if self.days.get_calendar_day_type(day).is_none() {
                self.days.set_calendar_day_type(day, DayType::Default);
            }
        }

        self.clear_working_date_cache();
    }

    /// Get the parent calendar (returns `None` when the parent calendar has been released).
    ///
    /// Corresponds to Java: `public ProjectCalendar getParent()`
    pub fn get_parent(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        self.parent.upgrade()
    }

    /// Get the parent calendar unique ID (returns `None` when there is no parent calendar or it has been released).
    ///
    /// Corresponds to Java: `public Integer getParentUniqueID()`
    pub fn get_parent_unique_id(&self) -> Option<i32> {
        let parent = self.parent.upgrade()?;
        let result = match parent.try_borrow() {
            Ok(parent_ref) => parent_ref.unique_id,
            Err(_) => None,
        };
        result
    }

    /// Compute how much of an absolute time span converts to working-day duration under this calendar.
    ///
    /// Corresponds to Java: `public Duration getDuration(LocalDateTime startDate, LocalDateTime endDate)`
    ///
    /// Returns `None` when any argument is `None` (corresponding to Java's `null`). Java's "same day returns 1 day"
    /// semantics are preserved by `get_days_in_range`.
    pub fn get_duration(
        &self,
        start_date: Option<NaiveDateTime>,
        end_date: Option<NaiveDateTime>,
    ) -> Option<Duration> {
        let (mut start_date, end_date) = match (start_date, end_date) {
            (Some(start_date), Some(end_date)) => (start_date, end_date),
            _ => return None,
        };

        let mut days = get_days_in_range(start_date, end_date);
        let mut duration = 0;

        while days > 0 {
            if self.is_working_date(start_date.date()) {
                duration += 1;
            }

            days -= 1;
            start_date = start_date
                .checked_add_signed(TimeDelta::days(1))
                .unwrap_or(start_date);
        }

        Some(Duration::get_instance_int(duration, TimeUnit::Days))
    }

    /// Get the work start time for the specified date; returns `None` for a non-working day.
    ///
    /// Corresponds to Java: `public LocalTime getStartTime(LocalDate date)`
    pub fn get_start_time(&self, date: NaiveDate) -> Option<NaiveTime> {
        if let Some(result) = self.start_time_cache.borrow().get(&date).copied() {
            return Some(result);
        }

        let ranges = self.get_ranges(date);
        if ranges.size() == 0 {
            return None;
        }

        let result = ranges.get(0).get_start();
        if let Some(result) = result {
            if let Ok(mut cache) = self.start_time_cache.try_borrow_mut() {
                cache.insert(date, result);
            }
        }

        result
    }

    /// Get the work end time for the specified date; returns `None` for a non-working day.
    ///
    /// Corresponds to Java: `public LocalTime getFinishTime(LocalDate date)`
    pub fn get_finish_time(&self, date: NaiveDate) -> Option<NaiveTime> {
        let ranges = self.get_ranges(date);
        if ranges.size() == 0 {
            return None;
        }

        ranges.get(ranges.size() - 1).get_end()
    }

    /// Given a date and a duration, compute the resulting date-time after adding the duration (supports positive and negative durations).
    ///
    /// Corresponds to Java: `public LocalDateTime getDate(LocalDateTime date, Duration duration)`
    ///
    /// Java updates an internal cache, so the Rust side takes `&mut self`.
    pub fn get_date(&mut self, date: NaiveDateTime, duration: Duration) -> NaiveDateTime {
        if duration.get_units().is_elapsed() {
            let elapsed_minutes = duration
                .convert_units(TimeUnit::ElapsedMinutes, self)
                .get_duration();
            return date
                .checked_add_signed(TimeDelta::minutes(elapsed_minutes as i64))
                .unwrap_or(date);
        }

        if duration.get_duration() < 0.0 {
            self.get_date_from_negative_duration(date, duration)
        } else {
            self.get_date_from_positive_duration(date, duration)
        }
    }

    /// Forward duration calculation.
    ///
    /// Corresponds to Java: `private LocalDateTime getDateFromPositiveDuration(LocalDateTime startDate, Duration duration)`
    fn get_date_from_positive_duration(
        &mut self,
        start_date: NaiveDateTime,
        duration: Duration,
    ) -> NaiveDateTime {
        let mut start_date = start_date;
        let mut remaining_milliseconds = java_math_round(
            NumberHelper::round(
                duration
                    .convert_units(TimeUnit::Minutes, &*self)
                    .get_duration(),
                2.0,
            ) * 60000.0,
        );

        if remaining_milliseconds == 0 {
            return start_date;
        }

        // Can the result of the previous call to this method be used to skip part of the calculation?
        let get_date_last_start_date = self.get_date_last_start_date;
        let get_date_last_remaining_milliseconds = self.get_date_last_remaining_milliseconds;

        self.get_date_last_start_date = Some(start_date);
        self.get_date_last_remaining_milliseconds = remaining_milliseconds;

        if self.get_date_last_result.is_some()
            && LocalDateTimeHelper::compare(Some(start_date), get_date_last_start_date) == 0
            && remaining_milliseconds >= get_date_last_remaining_milliseconds
        {
            if let Some(result) = self.get_date_last_result {
                start_date = result;
                remaining_milliseconds -= get_date_last_remaining_milliseconds;
                if remaining_milliseconds == 0 {
                    return start_date;
                }
            }
        }

        let mut current_day_start = start_date;
        let mut current_day_end = LocalDateHelper::get_local_date_time(Some(current_day_start.date()))
            .and_then(|value| value.checked_add_signed(TimeDelta::days(1)))
            .unwrap_or(current_day_start);

        while remaining_milliseconds > 0 {
            let current_date_working_milliseconds = java_math_round(
                self.get_work(current_day_start, current_day_end, TimeUnit::Minutes)
                    .get_duration()
                    * 60000.0,
            );

            if remaining_milliseconds == current_date_working_milliseconds {
                if let Some(finish_time) = self.get_finish_time(current_day_start.date()) {
                    current_day_end =
                        LocalTimeHelper::set_end_time(Some(current_day_start), Some(finish_time))
                            .unwrap_or(current_day_start);
                }
                break;
            }

            if remaining_milliseconds > current_date_working_milliseconds {
                // Deduct the day's work hours
                remaining_milliseconds -= current_date_working_milliseconds;

                // Advance the calendar to the next working day
                let mut current_day = current_day_start;
                let mut non_working_day_count = 0;
                loop {
                    match current_day.checked_add_signed(TimeDelta::days(1)) {
                        Some(next_day) => current_day = next_day,
                        None => {
                            current_day = current_day_start
                                .checked_add_signed(TimeDelta::days(1))
                                .unwrap_or(current_day_start);
                            remaining_milliseconds = 0;
                            break;
                        }
                    }

                    non_working_day_count += 1;
                    if non_working_day_count > MAX_NONWORKING_DAYS {
                        current_day = current_day_start
                            .checked_add_signed(TimeDelta::days(1))
                            .unwrap_or(current_day_start);
                        remaining_milliseconds = 0;
                        break;
                    }

                    if self.is_working_date(current_day.date()) {
                        break;
                    }
                }

                let start_time = self.get_start_time(current_day.date());
                current_day_start =
                    LocalTimeHelper::set_time(Some(current_day), start_time).unwrap_or(current_day);
                let finish_time = self.get_finish_time(current_day_start.date());
                current_day_end =
                    LocalTimeHelper::set_end_time(Some(current_day_start), finish_time)
                        .unwrap_or(current_day_start);
            } else {
                // The day has more work hours than remain: compute the exact time when work ends
                let ranges = self.get_ranges(current_day_start.date());
                let current_day_start_time = current_day_start.time();
                let mut first_range = true;

                for index in 0..ranges.size() {
                    let range = ranges.get(index);
                    let (Some(range_start), Some(range_end)) =
                        (range.get_start(), range.get_end())
                    else {
                        continue;
                    };

                    // Skip this range if its end time is earlier than the current start time
                    if first_range
                        && range_end != NaiveTime::MIN
                        && range_end < current_day_start_time
                    {
                        continue;
                    }

                    let mut range_start = range_start;
                    let mut range_end = range_end;

                    // When the current start time is later than the range start, move the range start forward
                    if first_range && range_start < current_day_start_time {
                        range_start = current_day_start_time;
                    }
                    first_range = false;

                    let range_milliseconds =
                        LocalTimeHelper::get_milliseconds_in_range(Some(range_start), Some(range_end));
                    if remaining_milliseconds > range_milliseconds {
                        remaining_milliseconds -= range_milliseconds;
                    } else {
                        if remaining_milliseconds != range_milliseconds {
                            range_end = range_start
                                + TimeDelta::milliseconds(remaining_milliseconds);
                        }
                        current_day_end =
                            LocalTimeHelper::set_time(Some(current_day_start), Some(range_end))
                                .unwrap_or(current_day_start);
                        remaining_milliseconds = 0;
                        break;
                    }
                }
            }
        }

        let current_day_end = truncate_to_seconds(current_day_end);
        self.get_date_last_result = Some(current_day_end);

        current_day_end
    }

    /// Backward duration calculation.
    ///
    /// Corresponds to Java: `private LocalDateTime getDateFromNegativeDuration(LocalDateTime endDate, Duration duration)`
    fn get_date_from_negative_duration(
        &mut self,
        end_date: NaiveDateTime,
        duration: Duration,
    ) -> NaiveDateTime {
        let mut remaining_milliseconds = -java_math_round(
            NumberHelper::round(
                duration
                    .convert_units(TimeUnit::Minutes, &*self)
                    .get_duration(),
                2.0,
            ) * 60000.0,
        );

        if remaining_milliseconds == 0 {
            return end_date;
        }

        // Compute the start and end date-times of the current day
        let mut current_day_start;
        let mut current_day_end = end_date;

        if current_day_end.time() == NaiveTime::MIN {
            current_day_start =
                LocalDateHelper::get_local_date_time(Some(current_day_end.date()))
                    .and_then(|value| value.checked_sub_signed(TimeDelta::days(1)))
                    .unwrap_or(current_day_end);
        } else {
            current_day_start = LocalDateHelper::get_local_date_time(Some(current_day_end.date()))
                .unwrap_or(current_day_end);
        }

        while remaining_milliseconds > 0 {
            let current_day_working_milliseconds = java_math_round(
                self.get_work(current_day_start, current_day_end, TimeUnit::Minutes)
                    .get_duration()
                    * 60000.0,
            );

            // The remaining work hours exactly equal the day's work hours
            if remaining_milliseconds == current_day_working_milliseconds {
                let start_time = self.get_start_time(current_day_start.date());
                current_day_start =
                    LocalTimeHelper::set_time(Some(current_day_start), start_time)
                        .unwrap_or(current_day_start);
                break;
            }

            if remaining_milliseconds > current_day_working_milliseconds {
                // Deduct the day's work hours
                remaining_milliseconds -= current_day_working_milliseconds;

                // Move the calendar back to the previous working day
                let mut non_working_day_count = 0;
                let mut current_day = current_day_start;
                loop {
                    match current_day.checked_sub_signed(TimeDelta::days(1)) {
                        Some(previous_day) => current_day = previous_day,
                        None => {
                            current_day = current_day_start
                                .checked_sub_signed(TimeDelta::days(1))
                                .unwrap_or(current_day_start);
                            remaining_milliseconds = 0;
                            break;
                        }
                    }

                    non_working_day_count += 1;
                    if non_working_day_count > MAX_NONWORKING_DAYS {
                        current_day = current_day_start
                            .checked_sub_signed(TimeDelta::days(1))
                            .unwrap_or(current_day_start);
                        remaining_milliseconds = 0;
                        break;
                    }

                    if self.is_working_date(current_day.date()) {
                        break;
                    }
                }

                current_day_start = current_day;
                let finish_time = self.get_finish_time(current_day_start.date());
                current_day_end =
                    LocalTimeHelper::set_end_time(Some(current_day_start), finish_time)
                        .unwrap_or(current_day_start);
            } else {
                // The day has more work hours than remain: compute the exact time when work starts
                let ranges = self.get_ranges(current_day_start.date());
                let mut ranges: Vec<_> = (0..ranges.size()).map(|index| ranges.get(index)).collect();
                ranges.reverse();

                let current_day_end_time = current_day_end.time();
                let mut last_range = true;

                for range in &ranges {
                    let (Some(range_start), Some(range_end)) =
                        (range.get_start(), range.get_end())
                    else {
                        continue;
                    };

                    // Skip this range if its start is later than the current end time
                    if current_day_end_time != NaiveTime::MIN && current_day_end_time < range_start {
                        continue;
                    }

                    let mut range_start = range_start;
                    let mut range_end = range_end;

                    // When the current end time is earlier than the range end, move the range end backward
                    if last_range && (range_end == NaiveTime::MIN || range_end > current_day_end_time)
                    {
                        range_end = current_day_end_time;
                    }
                    last_range = false;

                    let range_milliseconds =
                        LocalTimeHelper::get_milliseconds_in_range(Some(range_start), Some(range_end));
                    if remaining_milliseconds > range_milliseconds {
                        remaining_milliseconds -= range_milliseconds;
                    } else {
                        if remaining_milliseconds != range_milliseconds {
                            range_start = range_end
                                - TimeDelta::milliseconds(remaining_milliseconds);
                        }
                        current_day_start =
                            LocalTimeHelper::set_time(Some(current_day_start), Some(range_start))
                                .unwrap_or(current_day_start);
                        remaining_milliseconds = 0;
                        break;
                    }
                }
            }
        }

        truncate_to_seconds(current_day_start)
    }

    /// Get the next work start time after the given date-time.
    ///
    /// Corresponds to Java: `public LocalDateTime getNextWorkStart(LocalDateTime date)`
    pub fn get_next_work_start(&self, date: NaiveDateTime) -> NaiveDateTime {
        let original_date = date;
        let mut date = date;

        let ranges = self.get_ranges(original_date.date());

        let cal_time = date.time();
        let mut start_time: Option<NaiveTime> = None;

        for index in 0..ranges.size() {
            let range = ranges.get(index);
            let range_start = range.get_start();
            let range_end = range.get_end();

            if range_end == Some(NaiveTime::MIN) || range_end.is_some_and(|end| cal_time < end) {
                start_time = if range_start.is_some_and(|start| cal_time > start) {
                    Some(cal_time)
                } else {
                    range_start
                };
                break;
            }
        }

        // No start time available on the current day: find the next working day and take its start time
        if start_time.is_none() {
            let mut non_working_day_count = 0;
            loop {
                match date.checked_add_signed(TimeDelta::days(1)) {
                    Some(next_day) => date = next_day,
                    None => {
                        date = original_date;
                        break;
                    }
                }

                non_working_day_count += 1;
                if non_working_day_count > MAX_NONWORKING_DAYS {
                    date = original_date;
                    break;
                }

                if self.is_working_date(date.date()) {
                    break;
                }
            }

            start_time = self.get_start_time(date.date());
        }

        LocalTimeHelper::set_time(Some(date), start_time).unwrap_or(date)
    }

    /// Get the previous work end time before the given date-time.
    ///
    /// Corresponds to Java: `public LocalDateTime getPreviousWorkFinish(LocalDateTime date)`
    pub fn get_previous_work_finish(&self, date: NaiveDateTime) -> NaiveDateTime {
        let original_date = date;
        let mut date = date;

        let ranges = self.get_ranges(original_date.date());

        let cal_time = date.time();
        let mut finish_time: Option<NaiveTime> = None;

        let mut index = ranges.size();
        while index > 0 {
            index -= 1;
            let range = ranges.get(index);
            let range_end = range.get_end();

            if (range_end == Some(NaiveTime::MIN) && cal_time == NaiveTime::MIN)
                || range_end.is_some_and(|end| cal_time >= end)
            {
                finish_time = range_end;
                break;
            }
        }

        // No end time available on the current day: find the previous working day and take its end time
        if finish_time.is_none() {
            let mut non_working_day_count = 0;
            loop {
                match date.checked_sub_signed(TimeDelta::days(1)) {
                    Some(previous_day) => date = previous_day,
                    None => {
                        date = original_date;
                        break;
                    }
                }

                non_working_day_count += 1;
                if non_working_day_count > MAX_NONWORKING_DAYS {
                    date = original_date;
                    break;
                }

                if self.is_working_date(date.date()) {
                    break;
                }
            }

            finish_time = self.get_finish_time(date.date());
        }

        LocalTimeHelper::set_end_time(Some(date), finish_time).unwrap_or(date)
    }

    /// Get the day type for the specified day of the week. For a derived calendar whose type is `DEFAULT`, walk up the calendar hierarchy to find the
    /// effective day type; if a misconfigured hierarchy makes the base calendar `DEFAULT` too, assume Saturday and Sunday are non-working days.
    ///
    /// Corresponds to Java: `public DayType getDayType(DayOfWeek day)`
    ///
    /// Java returns `null` directly when the primitive type is `null`; the Rust side expresses this as `None`.
    pub fn get_day_type(&self, day: Weekday) -> Option<DayType> {
        let result = self.days.get_calendar_day_type(day);
        if result == Some(DayType::Default) {
            if let Some(parent) = self.parent.upgrade() {
                if let Ok(parent_ref) = parent.try_borrow() {
                    return parent_ref.get_day_type(day);
                }
                return None;
            }

            return Some(if day == Weekday::Sat || day == Weekday::Sun {
                DayType::NonWorking
            } else {
                DayType::Working
            });
        }
        result
    }

    /// Determine whether the specified day of the week is a working day.
    ///
    /// Corresponds to Java: `public boolean isWorkingDay(DayOfWeek day)`
    pub fn is_working_day(&self, day: Weekday) -> bool {
        self.get_day_type(day) == Some(DayType::Working)
    }

    /// Determine whether the specified date is a working day (taking exceptions into account).
    ///
    /// Corresponds to Java: `public boolean isWorkingDate(LocalDate date)`
    pub fn is_working_date(&self, date: NaiveDate) -> bool {
        self.get_ranges(date).size() != 0
    }

    /// Get the working hours for the specified day of the week (falls back to the base calendar when undefined).
    ///
    /// Corresponds to Java: `public ProjectCalendarHours getHours(DayOfWeek day)`
    pub fn get_hours(&self, day: Weekday) -> Option<ProjectCalendarHours> {
        if let Some(hours) = self.days.get_calendar_hours_for_day(day) {
            return Some(hours.clone());
        }

        let parent = self.parent.upgrade()?;
        let result = match parent.try_borrow() {
            Ok(parent_ref) => parent_ref.get_hours(day),
            Err(_) => None,
        };
        result
    }

    /// Get the working hours for the specified date (falls back to the base calendar when undefined).
    ///
    /// Corresponds to Java: `public ProjectCalendarHours getHours(LocalDate date)`
    ///
    /// Rust has no overloading, so the `_for_date` suffix is added.
    pub fn get_hours_for_date(&self, date: NaiveDate) -> ProjectCalendarHours {
        self.get_ranges(date)
    }

    /// Get the working hours for the specified date-time.
    ///
    /// Corresponds to Java: `public ProjectCalendarHours getHours(LocalDateTime date)`
    ///
    /// Rust has no overloading, so the `_for_date_time` suffix is added.
    pub fn get_hours_for_date_time(&self, date: NaiveDateTime) -> ProjectCalendarHours {
        self.get_ranges(date.date())
    }

    /// Set the calendar unique ID.
    ///
    /// Corresponds to Java: `@Override public void setUniqueID(Integer uniqueID)`
    ///
    /// A temporary calendar does not modify the context's calendar unique ID mapping (corresponds to Java's
    /// `if (!m_temporaryCalendar)` guard). Java also passes `this` to the container's
    /// `updateUniqueID(element, old, new)`; the Rust side obtains the handle from `self_ref` and calls back into the container,
    /// and when the context is already borrowed it is filled in by the index rebuild before clash fixing.
    pub fn set_unique_id(&mut self, unique_id: Option<i32>) {
        if !self.temporary_calendar {
            if let Some(context) = self.context.upgrade() {
                if let Ok(mut context_ref) = context.try_borrow_mut() {
                    context_ref
                        .get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>())
                        .sync(unique_id);
                    context_ref
                        .get_calendars_mut()
                        .update_unique_id_for_calendar(self.self_ref.upgrade(), self.unique_id, unique_id);
                }
            }
        }
        self.unique_id = unique_id;
    }

    /// Get the calendar unique ID.
    ///
    /// Corresponds to Java: `@Override public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.unique_id
    }

    /// Get the calendar GUID.
    ///
    /// Corresponds to Java: `public UUID getGUID()`
    pub fn get_guid(&self) -> Option<Uuid> {
        self.guid
    }

    /// Set the calendar GUID.
    ///
    /// Corresponds to Java: `public void setGUID(UUID value)`
    pub fn set_guid(&mut self, value: Option<Uuid>) {
        self.guid = value;
    }

    /// Get the list of tasks using this calendar.
    ///
    /// Corresponds to Java: `public List<Task> getTasks()`
    ///
    /// Java aggregates the tasks of all projects in the context and filters by `getCalendarUniqueID()`. The Rust side iterates similarly;
    /// returns an empty list when the context is not wired up or is already borrowed (Java's `m_context` is a strong reference, so this cannot happen).
    pub fn get_tasks(&self) -> Vec<Rc<RefCell<Task>>> {
        let mut result = Vec::new();

        // Java: m_uniqueID.equals(t.getCalendarUniqueID()) -- always false when m_uniqueID is null
        let Some(unique_id) = self.unique_id else {
            return result;
        };
        let Some(context) = self.context.upgrade() else {
            return result;
        };
        let Ok(context_ref) = context.try_borrow() else {
            return result;
        };

        for project in context_ref.get_projects() {
            let Ok(project_ref) = project.try_borrow() else {
                continue;
            };

            for task in project_ref.get_tasks().as_slice() {
                if let Ok(task_ref) = task.try_borrow() {
                    if task_ref.get_calendar_unique_id() == Some(unique_id) {
                        result.push(Rc::clone(task));
                    }
                }
            }
        }

        result
    }

    /// Get the list of resources using this calendar.
    ///
    /// Corresponds to Java: `public List<Resource> getResources()`
    ///
    /// The degraded case is the same as [`ProjectCalendar::get_tasks`].
    pub fn get_resources(&self) -> Vec<Rc<RefCell<Resource>>> {
        let mut result = Vec::new();

        // Java: m_uniqueID.equals(r.getCalendarUniqueID())
        let Some(unique_id) = self.unique_id else {
            return result;
        };
        let Some(context) = self.context.upgrade() else {
            return result;
        };
        let Ok(context_ref) = context.try_borrow() else {
            return result;
        };

        for resource in context_ref.get_resources().as_slice() {
            if let Ok(resource_ref) = resource.try_borrow() {
                if resource_ref.get_calendar_unique_id() == Some(unique_id) {
                    result.push(Rc::clone(resource));
                }
            }
        }

        result
    }

    /// Get the number of resources using this calendar.
    ///
    /// Corresponds to Java: `public int getResourceCount()`
    pub fn get_resource_count(&self) -> i32 {
        self.get_resources().len() as i32
    }

    /// Remove this calendar from the project.
    ///
    /// Corresponds to Java: `public void remove()`
    pub fn remove(&mut self) {
        let Some(context) = self.context.upgrade() else {
            return;
        };
        let Ok(mut context_ref) = context.try_borrow_mut() else {
            return;
        };

        context_ref.get_calendars_mut().remove(self);
    }

    /// Get the exception applicable to the specified date (returns `None` when none matches).
    ///
    /// Corresponds to Java: `public ProjectCalendarException getException(LocalDate date)`
    ///
    /// Java returns the exception object itself; the Rust side returns a clone.
    pub fn get_exception(&self, date: NaiveDate) -> Option<ProjectCalendarException> {
        let mut exception = None;

        // Use the expanded exception list (including the single entries expanded from recurring exceptions)
        self.populate_expanded_exceptions();

        if let Ok(expanded) = self.expanded_exceptions.try_borrow() {
            if !expanded.is_empty() {
                let mut low: i64 = 0;
                let mut high: i64 = expanded.len() as i64 - 1;

                while low <= high {
                    let mid = (low + high) >> 1;
                    let mid_val = &expanded[mid as usize];
                    let comparison = match (mid_val.get_from_date(), mid_val.get_to_date()) {
                        (Some(from_date), Some(to_date)) => {
                            LocalDateHelper::compare_with_range(from_date, to_date, date)
                        }
                        // Java throws an NPE when the start/end dates are null; the Rust side degrades to "target is later"
                        _ => -1,
                    };

                    if comparison > 0 {
                        low = mid + 1;
                    } else if comparison < 0 {
                        high = mid - 1;
                    } else {
                        exception = Some(mid_val.clone());
                        break;
                    }
                }
            }
        }

        if exception.is_none() {
            if let Some(parent) = self.parent.upgrade() {
                if let Ok(parent_ref) = parent.try_borrow() {
                    exception = parent_ref.get_exception(date);
                }
            }
        }

        exception
    }

    /// Get the work week applicable to the specified date (returns `None` when none matches).
    ///
    /// Corresponds to Java: `public ProjectCalendarWeek getWorkWeek(LocalDate date)`
    ///
    /// Java sorts first and then binary searches; the Rust side scans linearly (equivalent result, see the file header).
    pub fn get_work_week(&self, date: NaiveDate) -> Option<ProjectCalendarWeek> {
        let mut week = None;

        for candidate in &self.work_weeks {
            let comparison = match candidate.get_date_range() {
                Some(range) => range.compare_to_date(date),
                None => 0,
            };

            if comparison == 0 {
                week = Some(candidate.clone());
                break;
            }
        }

        if week.is_none() {
            if let Some(parent) = self.parent.upgrade() {
                if let Ok(parent_ref) = parent.try_borrow() {
                    week = parent_ref.get_work_week(date);
                }
            }
        }

        week
    }

    /// Get the default work hours for the specified day of the week (ignoring exceptions and work weeks).
    ///
    /// Corresponds to Java: `public Duration getWork(DayOfWeek day, TimeUnit format)`
    ///
    /// Rust has no overloading, and same-named methods cannot be distinguished by argument count alone, so the `_for_day` suffix is added
    /// (consistent with `get_work_for_date`).
    pub fn get_work_for_day(&self, day: Weekday, format: TimeUnit) -> Duration {
        self.convert_format(self.get_total_time_all(&self.get_ranges_for_day(day)), format)
    }

    /// Get the work hours for the specified date.
    ///
    /// Corresponds to Java: `public Duration getWork(LocalDate date, TimeUnit format)`
    ///
    /// Rust has no overloading, so the `_for_date` suffix is added.
    pub fn get_work_for_date(&self, date: NaiveDate, format: TimeUnit) -> Duration {
        self.convert_format(self.get_total_time_all(&self.get_ranges(date)), format)
    }

    /// Get the work hours between two date-times under this calendar.
    ///
    /// Corresponds to Java: `public Duration getWork(LocalDateTime startDate, LocalDateTime endDate, TimeUnit format)`
    ///
    /// Java returns `null` when an argument is `null`; Rust arguments are non-null, so that branch is unreachable.
    pub fn get_work(
        &self,
        start_date: NaiveDateTime,
        end_date: NaiveDateTime,
        format: TimeUnit,
    ) -> Duration {
        if format.is_elapsed() {
            let elapsed_minutes = Duration::get_instance(
                end_date.signed_duration_since(start_date).num_minutes() as f64,
                TimeUnit::ElapsedMinutes,
            );
            return elapsed_minutes.convert_units(format, self);
        }

        let mut total_time: i64 = 0;

        let mut start_date = start_date;
        let mut end_date = end_date;

        // Ensure start is earlier than end and record whether they were swapped
        let invert = start_date > end_date;
        if invert {
            std::mem::swap(&mut start_date, &mut end_date);
        }

        let start_day = start_date.date();
        let end_day = end_date.date();

        if start_day == end_day {
            let ranges = self.get_ranges(start_day);
            if ranges.size() != 0 {
                total_time = self.get_total_time_in_range(
                    &ranges,
                    start_date.time(),
                    end_date.time(),
                );
            }
        } else {
            // Find the first working day in the interval
            let mut current_day = start_day;
            while !self.is_working_date(current_day) && current_day < end_day {
                match current_day.succ_opt() {
                    Some(next_day) => current_day = next_day,
                    None => break,
                }
            }

            if current_day < end_day {
                // If the first working day is the start date, keep the start time; otherwise zero the start time for that day
                // so that all of the day's work hours are used.
                let target_time = if current_day == start_day {
                    start_date.time()
                } else {
                    NaiveTime::MIN
                };

                total_time += self.get_total_time_from(&self.get_ranges(current_day), target_time);

                // Process the intermediate working days one by one
                while let Some(next_day) = current_day.succ_opt() {
                    current_day = next_day;

                    if current_day >= end_day {
                        break;
                    }

                    let ranges = self.get_ranges(current_day);
                    if ranges.size() == 0 {
                        continue;
                    }

                    total_time += self.get_total_time_all(&ranges);
                }
            }

            // Handle the last day
            let ranges = self.get_ranges(end_day);
            if ranges.size() != 0 {
                total_time +=
                    self.get_total_time_in_range(&ranges, NaiveTime::MIN, end_date.time());
            }
        }

        if invert {
            total_time = -total_time;
        }

        self.convert_format(total_time, format)
    }

    /// Convert a number of milliseconds into a `Duration` in the specified unit.
    ///
    /// Corresponds to Java: `private Duration convertFormat(long totalTime, TimeUnit format)`
    ///
    /// Java throws `IllegalArgumentException` for unsupported units; the Rust side does not panic and falls back to 0.
    fn convert_format(&self, total_time: i64, format: TimeUnit) -> Duration {
        let mut duration = total_time as f64;

        match format {
            TimeUnit::Minutes | TimeUnit::ElapsedMinutes => {
                duration /= (60 * 1000) as f64;
            }
            TimeUnit::Hours | TimeUnit::ElapsedHours => {
                duration /= (60 * 60 * 1000) as f64;
            }
            TimeUnit::Days => {
                let minutes_per_day = NumberHelper::get_double(self.get_minutes_per_day().map(f64::from));
                if minutes_per_day != 0.0 {
                    duration /= minutes_per_day * 60.0 * 1000.0;
                } else {
                    duration = 0.0;
                }
            }
            TimeUnit::Weeks => {
                let minutes_per_week =
                    NumberHelper::get_double(self.get_minutes_per_week().map(f64::from));
                if minutes_per_week != 0.0 {
                    duration /= minutes_per_week * 60.0 * 1000.0;
                } else {
                    duration = 0.0;
                }
            }
            TimeUnit::Months => {
                let days_per_month =
                    NumberHelper::get_double(self.get_days_per_month().map(f64::from));
                let minutes_per_day =
                    NumberHelper::get_double(self.get_minutes_per_day().map(f64::from));
                if days_per_month != 0.0 && minutes_per_day != 0.0 {
                    duration /= days_per_month * minutes_per_day * 60.0 * 1000.0;
                } else {
                    duration = 0.0;
                }
            }
            TimeUnit::ElapsedDays => {
                duration /= (24 * 60 * 60 * 1000) as f64;
            }
            TimeUnit::ElapsedWeeks => {
                duration /= (7 * 24 * 60 * 60 * 1000) as f64;
            }
            TimeUnit::ElapsedMonths => {
                duration /= 30.0 * 24.0 * 60.0 * 60.0 * 1000.0;
            }
            // Java: default -> throw new IllegalArgumentException(...)
            // Rust does not panic and falls back to 0 (PERCENT / YEARS / ELAPSED_YEARS / ELAPSED_PERCENT)
            _ => {
                duration = 0.0;
            }
        }

        Duration::get_instance(duration, format)
    }

    /// Get the work hours (milliseconds) "after / before the intersection" within a set of working hours.
    ///
    /// Corresponds to Java: `private long getTotalTime(ProjectCalendarHours hours, LocalTime targetTime)`
    fn get_total_time_from(&self, hours: &ProjectCalendarHours, target_time: NaiveTime) -> i64 {
        let mut total: i64 = 0;

        for range in hours.iter() {
            let range_end = range.get_end();
            if range_end == Some(NaiveTime::MIN)
                || range_end.map_or(true, |end| target_time <= end)
            {
                total += get_range_overlap_milliseconds(
                    range.get_start(),
                    range_end,
                    Some(target_time),
                    range_end,
                );
            }
        }

        total
    }

    /// Get the work hours (milliseconds) represented by a set of working hours.
    ///
    /// Corresponds to Java: `private long getTotalTime(ProjectCalendarHours hours)`
    fn get_total_time_all(&self, hours: &ProjectCalendarHours) -> i64 {
        let mut total: i64 = 0;

        for range in hours.iter() {
            total += range.get_duration_as_milliseconds();
        }

        total
    }

    /// Get the overlapping work hours (milliseconds) between a single day's working hours and a given time interval.
    ///
    /// Corresponds to Java: `private long getTotalTime(ProjectCalendarHours hours, LocalTime start, LocalTime end)`
    fn get_total_time_in_range(
        &self,
        hours: &ProjectCalendarHours,
        start: NaiveTime,
        end: NaiveTime,
    ) -> i64 {
        if start == end {
            return 0;
        }

        let mut total: i64 = 0;

        for range in hours.iter() {
            total += get_range_overlap_milliseconds(
                Some(start),
                Some(end),
                range.get_start(),
                range.get_end(),
            );
        }

        total
    }

    /// Get the list of calendars derived from this calendar.
    ///
    /// Corresponds to Java: `public List<ProjectCalendar> getDerivedCalendars()`
    /// (the filtering logic of the private `getDerivedCalendarStream()` has been merged into this method)
    pub fn get_derived_calendars(&self) -> Vec<Rc<RefCell<ProjectCalendar>>> {
        let mut result = Vec::new();

        let Some(context) = self.context.upgrade() else {
            return result;
        };
        let Ok(context_ref) = context.try_borrow() else {
            return result;
        };

        for calendar in context_ref.get_calendars().as_slice() {
            let parent_unique_id = match calendar.try_borrow() {
                Ok(calendar_ref) => match calendar_ref.parent.upgrade() {
                    Some(parent) => match parent.try_borrow() {
                        Ok(parent_ref) => parent_ref.unique_id,
                        Err(_) => None,
                    },
                    None => None,
                },
                Err(_) => None,
            };

            if self.unique_id.is_some() && parent_unique_id == self.unique_id {
                result.push(Rc::clone(calendar));
            }
        }

        result
    }

    /// Get the list of calendars derived from this calendar under the specified project.
    ///
    /// Corresponds to Java: `public List<ProjectCalendar> getDerivedCalendarsForProject(ProjectFile project)`
    ///
    /// Java filters out derived calendars "whose type is `PROJECT` and whose `getProjectUniqueID()` is not equal to the project's unique ID".
    /// On the Rust side `getProjectUniqueID()` is `Option<i32>`: when both it and the project unique ID are
    /// `None` they are treated as equal (Java would throw an NPE here via `null.equals`); when the project is borrowed its
    /// unique ID cannot be obtained and is treated as `None`.
    pub fn get_derived_calendars_for_project(
        &self,
        project: &Rc<RefCell<ProjectFile>>,
    ) -> Vec<Rc<RefCell<ProjectCalendar>>> {
        let project_unique_id = project
            .try_borrow()
            .ok()
            .and_then(|file| file.get_project_properties().get_unique_id());

        self.get_derived_calendars()
            .into_iter()
            .filter(|calendar| match calendar.try_borrow() {
                Ok(calendar_ref) => {
                    calendar_ref.get_type() != CalendarType::Project
                        || calendar_ref.get_project_unique_id() == project_unique_id
                }
                Err(_) => false,
            })
            .collect()
    }

    /// Determine whether this calendar is the base calendar of another calendar.
    ///
    /// Corresponds to Java: `public boolean isParent()`
    pub fn is_parent(&self) -> bool {
        !self.get_derived_calendars().is_empty()
    }

    /// Determine whether this calendar is derived from another calendar.
    ///
    /// Corresponds to Java: `public boolean isDerived()`
    pub fn is_derived(&self) -> bool {
        self.parent.upgrade().is_some()
    }

    /// Get the calendar type; defaults to `GLOBAL`.
    ///
    /// Corresponds to Java: `public CalendarType getType()`
    pub fn get_type(&self) -> CalendarType {
        self.r#type
    }

    /// Set the calendar type (ignored when `None`, consistent with Java ignoring `null`).
    ///
    /// Corresponds to Java: `public void setType(CalendarType type)`
    pub fn set_type(&mut self, r#type: Option<CalendarType>) {
        if let Some(r#type) = r#type {
            self.r#type = r#type;
        }
    }

    /// Get the unique ID of the project this calendar belongs to (`None` when not project-specific).
    ///
    /// Corresponds to Java: `public Integer getProjectUniqueID()`
    pub fn get_project_unique_id(&self) -> Option<i32> {
        self.project_unique_id
    }

    /// Set the unique ID of the project this calendar belongs to.
    ///
    /// Corresponds to Java: `public void setProjectUniqueID(Integer projectUniqueID)`
    pub fn set_project_unique_id(&mut self, project_unique_id: Option<i32>) {
        self.project_unique_id = project_unique_id;
    }

    /// Determine whether this calendar is a personal calendar; defaults to `false`.
    ///
    /// Corresponds to Java: `public boolean getPersonal()`
    pub fn get_personal(&self) -> bool {
        self.personal
    }

    /// Set whether this calendar is a personal calendar.
    ///
    /// Corresponds to Java: `public void setPersonal(boolean personal)`
    pub fn set_personal(&mut self, personal: bool) {
        self.personal = personal;
    }

    /// Clear the cached calendar data (and recursively clear the caches of derived calendars).
    ///
    /// Corresponds to Java: `private void clearWorkingDateCache()`
    fn clear_working_date_cache(&mut self) {
        if let Ok(mut cache) = self.start_time_cache.try_borrow_mut() {
            cache.clear();
        }
        self.get_date_last_result = None;

        if !self.temporary_calendar {
            for derived in self.get_derived_calendars() {
                if let Ok(mut derived_ref) = derived.try_borrow_mut() {
                    derived_ref.clear_working_date_cache();
                }
            }
        }
    }

    /// Get the working hours for the specified date (taking exceptions, work weeks, and the base calendar into account).
    ///
    /// Corresponds to Java: `protected ProjectCalendarHours getRanges(LocalDate date)`
    ///
    /// Java may return `null` (when the base calendar has no matching hours); the Rust side always returns an empty collection.
    pub fn get_ranges(&self, date: NaiveDate) -> ProjectCalendarHours {
        if let Some(schedule) = &self.manual_schedule {
            return schedule.get_ranges(date);
        }
        // First check the exceptions on this calendar and any base calendar for that date
        if let Some(exception) = self.get_exception(date) {
            return exception.get_hours().clone();
        }

        // Determine which work week definition to use
        let week = self.get_work_week(date);
        let day = date.weekday();

        let (day_type, hours) = match &week {
            Some(week) => (
                week.get_calendar_day_type(day),
                week.get_calendar_hours_for_day(day).cloned(),
            ),
            None => (
                self.days.get_calendar_day_type(day),
                self.days.get_calendar_hours_for_day(day).cloned(),
            ),
        };

        match day_type {
            Some(DayType::NonWorking) => empty_date_ranges(),
            Some(DayType::Working) => hours.unwrap_or_default(),
            // DEFAULT (and the unset case where Java would throw an NPE) falls back to the base calendar
            _ => self.parent_hours(day),
        }
    }

    /// Get the working hours for the specified day of the week (taking the base calendar into account).
    ///
    /// Corresponds to Java: `protected ProjectCalendarHours getRanges(DayOfWeek day)`
    pub fn get_ranges_for_day(&self, day: Weekday) -> ProjectCalendarHours {
        match self.days.get_calendar_day_type(day) {
            Some(DayType::NonWorking) => empty_date_ranges(),
            Some(DayType::Working) => self
                .days
                .get_calendar_hours_for_day(day)
                .cloned()
                .unwrap_or_default(),
            _ => self.parent_hours(day),
        }
    }

    /// Get the working hours for the specified day of the week from the base calendar.
    ///
    /// Corresponds to Java: `m_parent == null ? EMPTY_DATE_RANGES : m_parent.getHours(day)`
    fn parent_hours(&self, day: Weekday) -> ProjectCalendarHours {
        let Some(parent) = self.parent.upgrade() else {
            return empty_date_ranges();
        };

        let result = match parent.try_borrow() {
            Ok(parent_ref) => parent_ref.get_hours(day).unwrap_or_default(),
            Err(_) => empty_date_ranges(),
        };
        result
    }

    /// Ensure the exception list is sorted.
    ///
    /// Corresponds to Java: `private void sortExceptions()`
    fn sort_exceptions(&mut self) {
        if !self.exceptions_sorted {
            self.exceptions.sort();
            self.exceptions_sorted = true;
        }
    }

    /// Build the expanded exception list from the main exception list: a recurring exception generates a separate exception for each occurrence.
    ///
    /// Corresponds to Java: `private void populateExpandedExceptions()`
    fn populate_expanded_exceptions(&self) {
        if self.exceptions.is_empty() {
            return;
        }
        if let Ok(expanded) = self.expanded_exceptions.try_borrow() {
            if !expanded.is_empty() {
                return;
            }
        }

        // Split the exceptions into "recurring" and "non-recurring" groups; non-recurring exceptions are grouped by their recurrence type
        let mut non_recurring: Vec<ProjectCalendarException> = Vec::new();
        let mut recurring: HashMap<Option<RecurrenceType>, Vec<ProjectCalendarException>> =
            HashMap::new();

        for exception in &self.exceptions {
            let expanded = exception.get_expanded_exceptions();
            if expanded.len() == 1 {
                if let Some(first) = expanded.into_iter().next() {
                    non_recurring.push(first);
                }
            } else {
                let recurrence_type = exception
                    .get_recurring()
                    .and_then(|recurring| recurring.get_recurrence_type());
                recurring
                    .entry(recurrence_type)
                    .or_default()
                    .push(exception.clone());
            }
        }

        // Process recurring exceptions in reverse priority order so that the resulting map keeps the exception in effect on each date
        let mut map: BTreeMap<NaiveDate, ProjectCalendarException> = BTreeMap::new();
        for r#type in ORDERED_RECURRENCE_TYPES {
            if let Some(exceptions) = recurring.get(&Some(r#type)) {
                for exception in exceptions {
                    for expanded in exception.get_expanded_exceptions() {
                        if let Some(from_date) = expanded.get_from_date() {
                            map.insert(from_date, expanded);
                        }
                    }
                }
            }
        }

        // Override the map with non-recurring exceptions: non-recurring exceptions have the highest priority
        for exception in non_recurring {
            if let Some(from_date) = exception.get_from_date() {
                map.insert(from_date, exception);
            }
        }

        // BTreeMap keeps the expanded exceptions sorted by date
        if let Ok(mut expanded) = self.expanded_exceptions.try_borrow_mut() {
            expanded.extend(map.into_values());
        }
    }
}

/// `ProjectCalendar` exposes the unique ID through trait implementations.
///
/// Corresponds to Java: `public final class ProjectCalendar implements ProjectEntityWithID, TimePeriodEntity`
/// -- Java's interfaces are expressed as traits on the Rust side (see `crate::project_entity_with_unique_id`).
/// After implementing these two traits, `ProjectCalendarContainer` can reuse the capabilities of the container base class under the `ProjectEntityWithUniqueID`
/// constraint.
impl ProjectEntityWithUniqueID for ProjectCalendar {
    /// Corresponds to Java: `@Override public Integer getUniqueID()`
    fn get_unique_id(&self) -> Option<i32> {
        self.unique_id
    }
}

/// Corresponds to Java: `ProjectEntityWithMutableUniqueID.setUniqueID(Integer)`
///
/// Delegates to this type's same-named declaring method (which contains the temporary calendar guard and container sequence synchronization; see
/// [`ProjectCalendar::set_unique_id`]).
impl ProjectEntityWithMutableUniqueID for ProjectCalendar {
    /// Corresponds to Java: `@Override public void setUniqueID(Integer uniqueID)`
    fn set_unique_id(&mut self, unique_id: Option<i32>) {
        ProjectCalendar::set_unique_id(self, unique_id);
    }
}

/// The default base calendar name (used when no base calendar is provided).
///
/// Corresponds to Java: `public static final String DEFAULT_BASE_CALENDAR_NAME = "Standard";`
pub const DEFAULT_BASE_CALENDAR_NAME: &str = "Standard";

/// The maximum number of non-working days to skip when searching for the next working day.
///
/// Corresponds to Java: `private static final int MAX_NONWORKING_DAYS = 1000;`
///
/// Java comment: a calendar may be configured with no working days at all, in which case searching forward loops forever, hence this limit.
const MAX_NONWORKING_DAYS: i32 = 1000;

/// The processing order when expanding recurring exceptions (by priority from low to high).
///
/// Corresponds to Java:
/// ```text
/// private static final RecurrenceType[] ORDERED_RECURRENCE_TYPES = {
///    RecurrenceType.WEEKLY, RecurrenceType.MONTHLY, RecurrenceType.YEARLY, RecurrenceType.DAILY };
/// ```
const ORDERED_RECURRENCE_TYPES: [RecurrenceType; 4] = [
    RecurrenceType::Weekly,
    RecurrenceType::Monthly,
    RecurrenceType::Yearly,
    RecurrenceType::Daily,
];

/// The empty date range collection.
///
/// Corresponds to Java: `private static final ProjectCalendarHours EMPTY_DATE_RANGES` (anonymous empty implementation).
/// Rust's `ProjectCalendarHours::new()` is not a `const fn`, so an equivalent object is returned by a function.
fn empty_date_ranges() -> ProjectCalendarHours {
    ProjectCalendarHours::new()
}

/// Get the number of days in the specified year.
///
/// Corresponds to Java: `Year.of(year).length()`
fn year_length(year: i32) -> i32 {
    match NaiveDate::from_ymd_opt(year, 12, 31) {
        Some(date) => date.ordinal() as i32,
        None => 365,
    }
}

/// Compute the number of days between two date-times (returns 1 rather than 0 for the same day).
///
/// Corresponds to Java: `private int getDaysInRange(LocalDateTime startDate, LocalDateTime endDate)`
///
/// Java assumes `startDate < endDate`.
fn get_days_in_range(start_date: NaiveDateTime, end_date: NaiveDateTime) -> i32 {
    let end_date_year = end_date.year();
    let end_date_day_of_year = end_date.ordinal() as i32;

    let mut cal = start_date;

    if end_date_year == cal.year() {
        (end_date_day_of_year - cal.ordinal() as i32) + 1
    } else {
        let mut result = 0;
        loop {
            result += (year_length(cal.year()) - cal.ordinal() as i32) + 1;

            cal = match NaiveDate::from_ymd_opt(cal.year() + 1, 1, 1) {
                Some(date) => NaiveDateTime::new(date, NaiveTime::MIN),
                None => break,
            };

            if cal.year() >= end_date_year {
                break;
            }
        }
        result + end_date_day_of_year
    }
}

/// Compute the number of overlapping milliseconds between two time intervals.
///
/// Corresponds to Java: `private long getTime(LocalTime start1, LocalTime end1, LocalTime start2, LocalTime end2)`
///
/// Returns 0 when any argument is `None` (corresponding to Java's `null`).
fn get_range_overlap_milliseconds(
    start1: Option<NaiveTime>,
    end1: Option<NaiveTime>,
    start2: Option<NaiveTime>,
    end2: Option<NaiveTime>,
) -> i64 {
    let (start1, end1, start2, end2) = match (start1, end1, start2, end2) {
        (Some(start1), Some(end1), Some(start2), Some(end2)) => (start1, end1, start2, end2),
        _ => return 0,
    };

    let max_start = if start1 > start2 { start1 } else { start2 };

    let min_end = if end1 == NaiveTime::MIN && end2 != NaiveTime::MIN {
        end2
    } else if end1 != NaiveTime::MIN && end2 == NaiveTime::MIN {
        end1
    } else if end1 < end2 {
        end1
    } else {
        end2
    };

    if min_end == NaiveTime::MIN || max_start < min_end {
        return LocalTimeHelper::get_milliseconds_in_range(Some(max_start), Some(min_end));
    }

    0
}

/// Strip milliseconds from a date-time (keep to seconds).
///
/// Corresponds to Java: `if (currentDayEnd.getNano() != 0) { currentDayEnd = LocalDateTime.of(...); }`
fn truncate_to_seconds(date_time: NaiveDateTime) -> NaiveDateTime {
    if date_time.nanosecond() == 0 {
        return date_time;
    }

    match NaiveTime::from_hms_opt(date_time.hour(), date_time.minute(), date_time.second()) {
        Some(time) => NaiveDateTime::new(date_time.date(), time),
        None => date_time,
    }
}

/// Corresponds to Java: `Math.round(double)` -- returns the nearest integer, with .5 rounding toward positive infinity.
fn java_math_round(value: f64) -> i64 {
    (value + 0.5).floor() as i64
}

/// Corresponds to Java: `ProjectCalendarHelper.mergeExceptions(ProjectCalendar target, ProjectCalendar source)`
/// and the merge algorithm of
/// (`ProjectCalendarHelper` is replicated in place in this repository, not as a separate module; see the file header).
/// Java's "calendar version" overload simply forwards to the "list version" `source.getCalendarExceptions()`,
/// so the Rust side keeps only this one function.
fn merge_exceptions(target: &mut ProjectCalendar, source_exceptions: &[ProjectCalendarException]) {
    // Java: this snapshot is not updated inside the loop
    let expanded_target_exceptions = target.get_expanded_calendar_exceptions();

    for source_exception in source_exceptions {
        let expanded_source_exceptions = source_exception.get_expanded_exceptions();

        let collision = expanded_source_exceptions
            .iter()
            .any(|expanded_source| {
                expanded_target_exceptions
                    .iter()
                    .any(|expanded_target| expanded_target.contains_exception(expanded_source))
            });

        if collision {
            // Conflict: expand the source exceptions and add only the parts that do not conflict with the target calendar
            for expanded_source in &expanded_source_exceptions {
                let blocked = expanded_target_exceptions
                    .iter()
                    .any(|expanded_target| expanded_target.contains_exception(expanded_source));
                if !blocked {
                    let new_exception = target.add_calendar_exception(
                        expanded_source.get_from_date(),
                        expanded_source.get_to_date(),
                    );
                    new_exception.add_all(expanded_source.get_hours());
                }
            }
        } else {
            // No conflict: copy as-is
            let new_exception = match source_exception.get_recurring() {
                Some(recurring) => target.add_calendar_exception_recurring(recurring.clone()),
                None => target.add_calendar_exception(
                    source_exception.get_from_date(),
                    source_exception.get_to_date(),
                ),
            };
            new_exception.add_all(source_exception.get_hours());
        }
    }
}

impl TimeUnitDefaultsContainer for ProjectCalendar {
    /// Corresponds to Java: `@Override public Integer getMinutesPerDay()`
    fn get_minutes_per_day(&self) -> Option<i32> {
        ProjectCalendar::get_minutes_per_day(self)
    }

    /// Corresponds to Java: `@Override public Integer getMinutesPerWeek()`
    fn get_minutes_per_week(&self) -> Option<i32> {
        ProjectCalendar::get_minutes_per_week(self)
    }

    /// Corresponds to Java: `@Override public Integer getMinutesPerMonth()`
    fn get_minutes_per_month(&self) -> Option<i32> {
        ProjectCalendar::get_minutes_per_month(self)
    }

    /// Corresponds to Java: `@Override public Integer getMinutesPerYear()`
    fn get_minutes_per_year(&self) -> Option<i32> {
        ProjectCalendar::get_minutes_per_year(self)
    }

    /// Corresponds to Java: `@Override public Integer getDaysPerMonth()`
    fn get_days_per_month(&self) -> Option<i32> {
        ProjectCalendar::get_days_per_month(self)
    }
}

impl Display for ProjectCalendar {
    /// Corresponds to Java: `@Override public String toString()`
    ///
    /// Java outputs with `PrintWriter.println` (platform-dependent newline with a trailing newline);
    /// the Rust side always uses `\n` with no trailing newline.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "[ProjectCalendar")?;
        writeln!(f, "   ID={}", optional_text(self.unique_id))?;
        writeln!(f, "   name={}", self.get_name().unwrap_or("null"))?;
        writeln!(
            f,
            "   baseCalendarName={}",
            match self.parent.upgrade() {
                Some(parent) => match parent.try_borrow() {
                    Ok(parent_ref) => parent_ref.get_name().unwrap_or("").to_string(),
                    Err(_) => String::new(),
                },
                None => String::new(),
            }
        )?;

        for day in JAVA_DAY_OF_WEEK_ORDER {
            writeln!(f, "   [Day {}", java_day_of_week_name(day))?;
            writeln!(
                f,
                "      type={}",
                day_type_name(self.get_calendar_day_type(day))
            )?;
            writeln!(
                f,
                "      hours={}",
                self.get_hours(day)
                    .map_or_else(|| "null".to_string(), |hours| hours.to_string())
            )?;
            writeln!(f, "   ]")?;
        }

        if !self.exceptions.is_empty() {
            writeln!(f, "   [Exceptions=")?;
            for exception in &self.exceptions {
                writeln!(f, "      {}", exception)?;
            }
            writeln!(f, "   ]")?;
        }

        if !self.work_weeks.is_empty() {
            writeln!(f, "   [WorkWeeks=")?;
            for week in &self.work_weeks {
                writeln!(f, "      {}", week)?;
            }
            writeln!(f, "   ]")?;
        }

        write!(f, "]")
    }
}

//
// The following methods correspond to methods inherited from ProjectCalendarDays in Java (forwarding)
//

impl ProjectCalendar {
    /// Corresponds to Java: `setName(String name)` (inherited from `ProjectCalendarDays`)
    pub fn set_name(&mut self, name: Option<String>) {
        self.days.set_name(name);
    }

    /// Corresponds to Java: `getName()` (inherited from `ProjectCalendarDays`)
    pub fn get_name(&self) -> Option<&str> {
        self.days.get_name()
    }

    /// Corresponds to Java: `getCalendarHours()` (inherited from `ProjectCalendarDays`)
    pub fn get_calendar_hours(&self) -> &[Option<ProjectCalendarHours>; 7] {
        self.days.get_calendar_hours()
    }

    /// Corresponds to Java: `getCalendarHours(DayOfWeek day)` (inherited from `ProjectCalendarDays`)
    pub fn get_calendar_hours_for_day(&self, day: Weekday) -> Option<&ProjectCalendarHours> {
        self.days.get_calendar_hours_for_day(day)
    }

    /// Corresponds to Java: the mutable version of `getCalendarHours(DayOfWeek day)` (added on the Rust side)
    pub fn get_calendar_hours_for_day_mut(
        &mut self,
        day: Weekday,
    ) -> Option<&mut ProjectCalendarHours> {
        self.days.get_calendar_hours_for_day_mut(day)
    }

    /// Corresponds to Java: `addDefaultCalendarHours()` (inherited from `ProjectCalendarDays`)
    pub fn add_default_calendar_hours(&mut self) {
        self.days.add_default_calendar_hours();
    }

    /// Corresponds to Java: `addDefaultCalendarDays()` (inherited from `ProjectCalendarDays`)
    pub fn add_default_calendar_days(&mut self) {
        self.days.add_default_calendar_days();
    }

    /// Corresponds to Java: `addDefaultCalendarHours(DayOfWeek day)` (inherited from `ProjectCalendarDays`)
    pub fn add_default_calendar_hours_for_day(&mut self, day: Weekday) {
        self.days.add_default_calendar_hours_for_day(day);
    }

    /// Corresponds to Java: `getCalendarDayTypes()` (inherited from `ProjectCalendarDays`)
    pub fn get_calendar_day_types(&self) -> &[Option<DayType>; 7] {
        self.days.get_calendar_day_types()
    }

    /// Corresponds to Java: `getCalendarDayType(DayOfWeek day)` (inherited from `ProjectCalendarDays`)
    pub fn get_calendar_day_type(&self, day: Weekday) -> Option<DayType> {
        self.days.get_calendar_day_type(day)
    }

    /// Corresponds to Java: `setWorkingDay(DayOfWeek day, boolean working)` (inherited from `ProjectCalendarDays`)
    pub fn set_working_day(&mut self, day: Weekday, working: bool) {
        self.days.set_working_day(day, working);
    }

    /// Corresponds to Java: `setCalendarDayType(DayOfWeek day, DayType type)` (inherited from `ProjectCalendarDays`)
    pub fn set_calendar_day_type(&mut self, day: Weekday, r#type: DayType) {
        self.days.set_calendar_day_type(day, r#type);
    }

    /// Corresponds to Java: `DEFAULT_WORKING_MORNING` (inherited from `ProjectCalendarDays`)
    pub fn default_working_morning() -> LocalTimeRange {
        ProjectCalendarDays::default_working_morning()
    }

    /// Corresponds to Java: `DEFAULT_WORKING_AFTERNOON` (inherited from `ProjectCalendarDays`)
    pub fn default_working_afternoon() -> LocalTimeRange {
        ProjectCalendarDays::default_working_afternoon()
    }
}

/// Get Java-style text for a nullable value (`None` -> `"null"`).
fn optional_text<T: Display>(value: Option<T>) -> String {
    value.map_or_else(|| "null".to_string(), |value| value.to_string())
}

/// Corresponds to `name()` of the Java enum `DayOfWeek` (`MONDAY` ... `SUNDAY`).
fn java_day_of_week_name(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "MONDAY",
        Weekday::Tue => "TUESDAY",
        Weekday::Wed => "WEDNESDAY",
        Weekday::Thu => "THURSDAY",
        Weekday::Fri => "FRIDAY",
        Weekday::Sat => "SATURDAY",
        Weekday::Sun => "SUNDAY",
    }
}

/// Corresponds to `name()` of the Java enum `DayType` (`DayType` does not override `toString`).
fn day_type_name(day_type: Option<DayType>) -> &'static str {
    match day_type {
        None => "null",
        Some(DayType::NonWorking) => "NON_WORKING",
        Some(DayType::Working) => "WORKING",
        Some(DayType::Default) => "DEFAULT",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_date_range::LocalDateRange;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
    }

    fn time(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).expect("valid time")
    }

    fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDateTime::new(date(year, month, day), time(hour, minute))
    }

    fn hours(value: f64) -> Duration {
        Duration::get_instance(value, TimeUnit::Hours)
    }

    /// A temporary calendar without a context (to avoid tests depending on a concrete ProjectContext implementation).
    fn temp_calendar() -> ProjectCalendar {
        ProjectCalendar::new(Weak::new(), true)
    }

    /// Standard calendar: Monday to Friday working, default working hours 08:00-12:00 / 13:00-17:00.
    fn standard_calendar() -> ProjectCalendar {
        let mut calendar = temp_calendar();
        calendar.set_name(Some("Standard".to_string()));
        calendar.add_default_calendar_days();
        calendar.add_default_calendar_hours();
        calendar
    }

    #[test]
    fn standard_working_week() {
        let calendar = standard_calendar();

        assert_eq!(calendar.get_name(), Some("Standard"));
        for day in [
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
        ] {
            assert!(calendar.is_working_day(day));
        }
        assert!(!calendar.is_working_day(Weekday::Sat));
        assert!(!calendar.is_working_day(Weekday::Sun));

        assert_eq!(calendar.get_day_type(Weekday::Mon), Some(DayType::Working));
        assert_eq!(
            calendar.get_day_type(Weekday::Sun),
            Some(DayType::NonWorking)
        );

        // 2024-06-03 is a Monday, 2024-06-01 is a Saturday
        assert!(calendar.is_working_date(date(2024, 6, 3)));
        assert!(!calendar.is_working_date(date(2024, 6, 1)));

        assert_eq!(calendar.get_work_for_day(Weekday::Mon, TimeUnit::Hours), hours(8.0));
        assert_eq!(calendar.get_work_for_day(Weekday::Sat, TimeUnit::Hours), hours(0.0));
        assert_eq!(calendar.get_start_time(date(2024, 6, 3)), Some(time(8, 0)));
        assert_eq!(calendar.get_finish_time(date(2024, 6, 3)), Some(time(17, 0)));
        assert_eq!(calendar.get_start_time(date(2024, 6, 1)), None);
        assert_eq!(calendar.get_hours(Weekday::Mon).map(|value| value.size()), Some(2));
        assert_eq!(calendar.get_hours(Weekday::Sun).map(|value| value.size()), Some(0));
    }

    #[test]
    fn unset_day_types_are_not_working() {
        let calendar = temp_calendar();

        // When no DayType is set, Java getCalendarDayType returns null and the Rust side returns None
        assert_eq!(calendar.get_day_type(Weekday::Mon), None);
        assert!(!calendar.is_working_day(Weekday::Mon));
        assert!(!calendar.is_working_date(date(2024, 6, 3)));
        assert_eq!(calendar.get_ranges(date(2024, 6, 3)).size(), 0);
    }

    #[test]
    fn derived_calendar_falls_back_to_parent() {
        let parent = Rc::new(RefCell::new(standard_calendar()));
        let mut child = temp_calendar();

        assert!(!child.is_derived());
        child.set_parent(Some(Rc::clone(&parent)));
        assert!(child.is_derived());

        // setParent sets unset day types to DEFAULT
        assert_eq!(
            child.get_calendar_day_type(Weekday::Mon),
            Some(DayType::Default)
        );
        assert_eq!(child.get_day_type(Weekday::Mon), Some(DayType::Working));
        assert!(child.is_working_day(Weekday::Mon));
        assert!(!child.is_working_day(Weekday::Sat));

        // hours / work hours / start time all fall back to the base calendar
        assert_eq!(child.get_hours(Weekday::Mon).map(|value| value.size()), Some(2));
        assert_eq!(child.get_start_time(date(2024, 6, 3)), Some(time(8, 0)));
        assert_eq!(child.get_work_for_day(Weekday::Mon, TimeUnit::Hours), hours(8.0));
        assert!(child.is_working_date(date(2024, 6, 3)));
        assert!(!child.is_working_date(date(2024, 6, 1)));

        assert_eq!(child.get_parent_unique_id(), None);
        assert!(child.get_parent().is_some());

        // Exceptions on the base calendar also apply to derived calendars
        {
            let mut parent_ref = parent.borrow_mut();
            parent_ref.add_calendar_exception_for_date(Some(date(2024, 6, 3)));
        }
        assert!(!child.is_working_date(date(2024, 6, 3)));

        // Remove the parent calendar relationship
        child.set_parent(None);
        assert!(!child.is_derived());
        // The day type is still DEFAULT; with no parent calendar, Monday to Friday are treated as WORKING per Java semantics
        assert_eq!(child.get_calendar_day_type(Weekday::Mon), Some(DayType::Default));
        assert_eq!(child.get_day_type(Weekday::Mon), Some(DayType::Working));
        assert!(child.is_working_day(Weekday::Mon));
        assert_eq!(child.get_day_type(Weekday::Sat), Some(DayType::NonWorking));
        // but the DEFAULT "ranges" must fall back to the base calendar, and are empty without one -- consistent with Java behavior
        assert!(!child.is_working_date(date(2024, 6, 3)));
    }

    #[test]
    fn exceptions_override_day_type_and_hours() {
        let mut calendar = standard_calendar();
        let saturday = date(2024, 6, 1);
        let monday = date(2024, 6, 3);

        assert!(!calendar.is_working_date(saturday));
        {
            let exception = calendar.add_calendar_exception_for_date(Some(saturday));
            exception.add(LocalTimeRange::new(Some(time(9, 0)), Some(time(12, 0))));
        }

        assert!(calendar.is_working_date(saturday));
        assert_eq!(calendar.get_start_time(saturday), Some(time(9, 0)));
        assert_eq!(calendar.get_finish_time(saturday), Some(time(12, 0)));
        assert_eq!(
            calendar.get_work_for_date(saturday, TimeUnit::Hours),
            hours(3.0)
        );
        assert!(calendar.get_exception(saturday).is_some());
        assert!(calendar.get_exception(date(2024, 6, 8)).is_none());

        // Change a working day to a non-working day
        calendar.add_calendar_exception_for_date(Some(monday));
        assert!(!calendar.is_working_date(monday));
        assert_eq!(calendar.get_work_for_date(monday, TimeUnit::Hours), hours(0.0));
    }

    #[test]
    fn range_exception_and_removal() {
        let mut calendar = standard_calendar();

        {
            let exception = calendar.add_calendar_exception(
                Some(date(2024, 6, 3)),
                Some(date(2024, 6, 5)),
            );
            exception.add(LocalTimeRange::new(Some(time(10, 0)), Some(time(14, 0))));
        }

        assert!(calendar.is_working_date(date(2024, 6, 3)));
        assert!(calendar.is_working_date(date(2024, 6, 4)));
        assert!(calendar.is_working_date(date(2024, 6, 5)));
        assert_eq!(
            calendar.get_work_for_date(date(2024, 6, 4), TimeUnit::Hours),
            hours(4.0)
        );
        assert!(calendar.get_exception(date(2024, 6, 4)).is_some());

        assert_eq!(calendar.get_calendar_exceptions().len(), 1);
        assert_eq!(calendar.get_expanded_calendar_exceptions().len(), 1);

        // Remove (Java's equals compares only the working hours content)
        let expanded = calendar.get_expanded_calendar_exceptions();
        calendar.remove_calendar_exception(&expanded[0]);
        assert!(calendar.get_calendar_exceptions().is_empty());
        assert!(calendar.is_working_date(date(2024, 6, 3)));
        assert_eq!(
            calendar.get_work_for_date(date(2024, 6, 3), TimeUnit::Hours),
            hours(8.0)
        );

        // Clear
        calendar.add_calendar_exception_for_date(Some(date(2024, 6, 3)));
        assert_eq!(calendar.get_calendar_exceptions().len(), 1);
        calendar.clear_calendar_exceptions();
        assert!(calendar.get_calendar_exceptions().is_empty());
        assert!(calendar.get_expanded_calendar_exceptions().is_empty());
    }

    #[test]
    fn get_duration_counts_working_days() {
        let calendar = standard_calendar();

        // Monday 08:00 -> Friday 17:00: 5 working days
        assert_eq!(
            calendar.get_duration(
                Some(date_time(2024, 6, 3, 8, 0)),
                Some(date_time(2024, 6, 7, 17, 0))
            ),
            Some(Duration::get_instance_int(5, TimeUnit::Days))
        );

        // The same day returns 1 rather than 0 (Java semantics)
        assert_eq!(
            calendar.get_duration(
                Some(date_time(2024, 6, 3, 8, 0)),
                Some(date_time(2024, 6, 3, 9, 0))
            ),
            Some(Duration::get_instance_int(1, TimeUnit::Days))
        );

        // Spanning a weekend: 6/3 (Mon) -> 6/10 (Mon), 6 working days in total
        assert_eq!(
            calendar.get_duration(
                Some(date_time(2024, 6, 3, 8, 0)),
                Some(date_time(2024, 6, 10, 17, 0))
            ),
            Some(Duration::get_instance_int(6, TimeUnit::Days))
        );

        // Cross-year interval (covers the cross-year branch of getDaysInRange)
        assert_eq!(
            calendar.get_duration(
                Some(date_time(2024, 12, 30, 8, 0)),
                Some(date_time(2025, 1, 3, 17, 0))
            ),
            Some(Duration::get_instance_int(5, TimeUnit::Days))
        );

        assert!(calendar
            .get_duration(None, Some(date_time(2024, 6, 3, 8, 0)))
            .is_none());
    }

    #[test]
    fn get_work_across_dates() {
        let calendar = standard_calendar();

        // Same day
        assert_eq!(
            calendar.get_work(
                date_time(2024, 6, 3, 8, 0),
                date_time(2024, 6, 3, 17, 0),
                TimeUnit::Hours
            ),
            hours(8.0)
        );
        assert_eq!(
            calendar.get_work(
                date_time(2024, 6, 3, 9, 0),
                date_time(2024, 6, 3, 10, 30),
                TimeUnit::Hours
            ),
            hours(1.5)
        );

        // Cross-day
        assert_eq!(
            calendar.get_work(
                date_time(2024, 6, 3, 8, 0),
                date_time(2024, 6, 4, 17, 0),
                TimeUnit::Hours
            ),
            hours(16.0)
        );

        // Spanning a weekend
        assert_eq!(
            calendar.get_work(
                date_time(2024, 6, 7, 8, 0),
                date_time(2024, 6, 10, 17, 0),
                TimeUnit::Hours
            ),
            hours(16.0)
        );

        // Reversed arguments return a negative value
        assert_eq!(
            calendar.get_work(
                date_time(2024, 6, 4, 17, 0),
                date_time(2024, 6, 3, 8, 0),
                TimeUnit::Hours
            ),
            hours(-16.0)
        );

        // The interval covers only a weekend
        assert_eq!(
            calendar.get_work(
                date_time(2024, 6, 7, 17, 0),
                date_time(2024, 6, 10, 8, 0),
                TimeUnit::Hours
            ),
            hours(0.0)
        );

        // elapsed units are unaffected by the calendar
        assert_eq!(
            calendar.get_work(
                date_time(2024, 6, 1, 8, 0),
                date_time(2024, 6, 1, 10, 0),
                TimeUnit::ElapsedHours
            ),
            Duration::get_instance(2.0, TimeUnit::ElapsedHours)
        );
    }

    #[test]
    fn get_date_positive_and_negative() {
        let mut calendar = standard_calendar();
        let monday = date_time(2024, 6, 3, 8, 0);

        // Exactly one day of work hours -> 17:00 on the same day
        assert_eq!(calendar.get_date(monday, hours(8.0)), date_time(2024, 6, 3, 17, 0));

        // One day and one hour -> 09:00 the next day
        assert_eq!(calendar.get_date(monday, hours(9.0)), date_time(2024, 6, 4, 9, 0));

        // Spanning a weekend: Friday 08:00 + 9h -> 09:00 the following Monday
        assert_eq!(
            calendar.get_date(date_time(2024, 6, 7, 8, 0), hours(9.0)),
            date_time(2024, 6, 10, 9, 0)
        );

        // Reverse: Tuesday 17:00 - 9h -> Monday 16:00
        assert_eq!(
            calendar.get_date(date_time(2024, 6, 4, 17, 0), hours(-9.0)),
            date_time(2024, 6, 3, 16, 0)
        );

        // A zero duration returns the original value
        assert_eq!(calendar.get_date(monday, hours(0.0)), monday);
    }

    #[test]
    fn get_date_with_elapsed_duration() {
        let mut calendar = standard_calendar();

        // elapsed durations advance by natural time and are not limited by working days
        assert_eq!(
            calendar.get_date(
                date_time(2024, 6, 1, 10, 0),
                Duration::get_instance(90.0, TimeUnit::ElapsedMinutes)
            ),
            date_time(2024, 6, 1, 11, 30)
        );
    }

    #[test]
    fn next_and_previous_work_boundaries() {
        let calendar = standard_calendar();

        // Saturday -> 08:00 the following Monday
        assert_eq!(
            calendar.get_next_work_start(date_time(2024, 6, 1, 10, 0)),
            date_time(2024, 6, 3, 8, 0)
        );

        // During Monday's lunch break -> 13:00
        assert_eq!(
            calendar.get_next_work_start(date_time(2024, 6, 3, 12, 30)),
            date_time(2024, 6, 3, 13, 0)
        );

        // Before work starts on a working day -> 08:00 the same day
        assert_eq!(
            calendar.get_next_work_start(date_time(2024, 6, 3, 6, 0)),
            date_time(2024, 6, 3, 8, 0)
        );

        // Sunday -> 17:00 the previous Friday
        assert_eq!(
            calendar.get_previous_work_finish(date_time(2024, 6, 2, 10, 0)),
            date_time(2024, 5, 31, 17, 0)
        );

        // During Monday's lunch break -> 12:00
        assert_eq!(
            calendar.get_previous_work_finish(date_time(2024, 6, 3, 12, 30)),
            date_time(2024, 6, 3, 12, 0)
        );
    }

    #[test]
    fn no_working_days_terminates() {
        let mut calendar = temp_calendar();
        for day in JAVA_DAY_OF_WEEK_ORDER {
            calendar.set_calendar_day_type(day, DayType::NonWorking);
        }

        // Java uses MAX_NONWORKING_DAYS as a fallback to avoid an infinite loop; Rust likewise returns the original value
        let start = date_time(2024, 6, 3, 10, 0);
        assert_eq!(calendar.get_next_work_start(start), start);
        assert_eq!(calendar.get_previous_work_finish(start), start);
    }

    #[test]
    fn work_week_overrides_day_type_and_hours() {
        let mut calendar = standard_calendar();
        let saturday = date(2024, 6, 1);

        assert!(!calendar.is_working_date(saturday));

        {
            let week = calendar.add_work_week();
            week.set_date_range(Some(LocalDateRange::new(
                Some(date(2024, 5, 27)),
                Some(date(2024, 6, 2)),
            )));
            week.set_calendar_day_type(Weekday::Sat, DayType::Working);
            week.add_calendar_hours(Weekday::Sat)
                .add(LocalTimeRange::new(Some(time(9, 0)), Some(time(13, 0))));
        }

        assert_eq!(calendar.get_work_weeks().len(), 1);
        assert!(calendar.get_work_week(saturday).is_some());
        assert!(calendar.is_working_date(saturday));
        assert_eq!(calendar.get_start_time(saturday), Some(time(9, 0)));
        assert_eq!(calendar.get_work_for_date(saturday, TimeUnit::Hours), hours(4.0));

        // Unaffected outside the work week interval
        assert!(!calendar.is_working_date(date(2024, 6, 8)));

        // Exceptions take precedence over work weeks
        calendar.add_calendar_exception_for_date(Some(saturday));
        assert!(!calendar.is_working_date(saturday));
    }

    #[test]
    fn work_weeks_management() {
        let mut calendar = standard_calendar();
        assert!(calendar.get_work_weeks().is_empty());

        {
            let week = calendar.add_work_week();
            week.set_name(Some("First Week".to_string()));
            week.set_date_range(Some(LocalDateRange::new(
                Some(date(2024, 1, 1)),
                Some(date(2024, 1, 7)),
            )));
        }

        assert_eq!(calendar.get_work_weeks().len(), 1);
        assert_eq!(calendar.get_work_weeks()[0].get_name(), Some("First Week"));
        assert!(calendar.get_work_week(date(2024, 1, 3)).is_some());
        assert!(calendar.get_work_week(date(2024, 2, 1)).is_none());

        let week = calendar.get_work_weeks()[0].clone();
        calendar.remove_work_week(&week);
        assert!(calendar.get_work_weeks().is_empty());

        calendar.add_work_week();
        assert_eq!(calendar.get_work_weeks().len(), 1);
        calendar.clear_work_weeks();
        assert!(calendar.get_work_weeks().is_empty());
    }

    #[test]
    fn expanded_exceptions_with_work_weeks_are_generated() {
        let mut calendar = standard_calendar();

        {
            let week = calendar.add_work_week();
            week.set_date_range(Some(LocalDateRange::new(
                Some(date(2024, 6, 1)),
                Some(date(2024, 6, 2)),
            )));
            week.set_calendar_day_type(Weekday::Sat, DayType::Working);
            week.add_calendar_hours(Weekday::Sat)
                .add(LocalTimeRange::new(Some(time(9, 0)), Some(time(13, 0))));
        }

        // With no work weeks it is equivalent to getExpandedCalendarExceptions
        let expanded = calendar.get_expanded_calendar_exceptions_with_work_weeks();
        assert!(!expanded.is_empty());

        // It is likewise merged in after adding a normal exception
        calendar.add_calendar_exception_for_date(Some(date(2024, 6, 3)));
        let expanded = calendar.get_expanded_calendar_exceptions_with_work_weeks();
        assert!(expanded
            .iter()
            .any(|exception| exception.get_from_date() == Some(date(2024, 6, 3))));
    }

    #[test]
    fn minutes_defaults_and_unit_conversion() {
        let calendar = standard_calendar();

        // No context, no parent calendar, and no explicit setting -> None (corresponding to Java's null)
        assert_eq!(calendar.get_minutes_per_day(), None);
        assert_eq!(calendar.get_days_per_month(), None);

        // When minutes per day is 0, the per-day conversion result is 0
        assert_eq!(
            calendar.get_work_for_day(Weekday::Mon, TimeUnit::Days),
            Duration::get_instance(0.0, TimeUnit::Days)
        );
        assert_eq!(
            calendar.get_work_for_day(Weekday::Mon, TimeUnit::Minutes),
            Duration::get_instance(480.0, TimeUnit::Minutes)
        );

        let mut calendar = standard_calendar();
        calendar.set_calendar_minutes_per_day(Some(480));
        calendar.set_calendar_minutes_per_week(Some(2400));
        calendar.set_calendar_minutes_per_month(Some(9600));
        calendar.set_calendar_minutes_per_year(Some(115_200));

        assert_eq!(calendar.get_minutes_per_day(), Some(480));
        assert_eq!(calendar.get_minutes_per_week(), Some(2400));
        assert_eq!(calendar.get_minutes_per_month(), Some(9600));
        assert_eq!(calendar.get_minutes_per_year(), Some(115_200));
        assert_eq!(calendar.get_calendar_minutes_per_day(), Some(480));
        assert_eq!(calendar.get_calendar_minutes_per_week(), Some(2400));
        assert_eq!(calendar.get_calendar_minutes_per_month(), Some(9600));
        assert_eq!(calendar.get_calendar_minutes_per_year(), Some(115_200));

        assert_eq!(
            calendar.get_work_for_day(Weekday::Mon, TimeUnit::Days),
            Duration::get_instance(1.0, TimeUnit::Days)
        );
        assert_eq!(
            calendar.get_work_for_day(Weekday::Mon, TimeUnit::Weeks),
            Duration::get_instance(0.2, TimeUnit::Weeks)
        );
        assert_eq!(
            calendar.get_work_for_day(Weekday::Mon, TimeUnit::Hours),
            hours(8.0)
        );

        // Derived calendars inherit the parent calendar's minute values
        let parent = Rc::new(RefCell::new(calendar));
        let mut child = temp_calendar();
        child.set_parent(Some(Rc::clone(&parent)));
        assert_eq!(child.get_minutes_per_day(), Some(480));
        assert_eq!(child.get_minutes_per_week(), Some(2400));
        assert_eq!(child.get_minutes_per_month(), Some(9600));
        assert_eq!(child.get_minutes_per_year(), Some(115_200));

        // Used as a trait object to verify that the TimeUnitDefaultsContainer implementation is usable
        let container: &dyn TimeUnitDefaultsContainer = &child;
        assert_eq!(container.get_minutes_per_day(), Some(480));
    }

    #[test]
    fn ranges_for_day_and_date_follow_day_type() {
        let calendar = standard_calendar();

        assert_eq!(calendar.get_ranges_for_day(Weekday::Mon).size(), 2);
        assert_eq!(calendar.get_ranges_for_day(Weekday::Sun).size(), 0);
        assert_eq!(calendar.get_hours_for_date(date(2024, 6, 3)).size(), 2);
        assert_eq!(calendar.get_hours_for_date(date(2024, 6, 1)).size(), 0);
        assert_eq!(calendar.get_hours_for_date_time(date_time(2024, 6, 3, 10, 0)).size(), 2);
        assert_eq!(calendar.get_calendar_hours()[6].is_some(), true);
    }

    #[test]
    fn metadata_round_trips() {
        let mut calendar = temp_calendar();

        assert_eq!(calendar.get_type(), CalendarType::Global);
        calendar.set_type(None);
        assert_eq!(calendar.get_type(), CalendarType::Global);
        calendar.set_type(Some(CalendarType::Resource));
        assert_eq!(calendar.get_type(), CalendarType::Resource);

        assert!(!calendar.get_personal());
        calendar.set_personal(true);
        assert!(calendar.get_personal());

        assert_eq!(calendar.get_project_unique_id(), None);
        calendar.set_project_unique_id(Some(42));
        assert_eq!(calendar.get_project_unique_id(), Some(42));

        assert_eq!(calendar.get_guid(), None);
        let guid = Uuid::from_u128(0x1234_5678_9abc_def0_1234_5678_9abc_def0);
        calendar.set_guid(Some(guid));
        assert_eq!(calendar.get_guid(), Some(guid));

        // A temporary calendar does not access the context; after removing the context's unique ID allocation it assigns directly
        assert_eq!(calendar.get_unique_id(), None);
        calendar.set_unique_id(Some(7));
        assert_eq!(calendar.get_unique_id(), Some(7));

        // A temporary calendar is never the default calendar
        assert!(!calendar.get_default());
        calendar.set_default();
        assert!(!calendar.get_default());
    }

    #[test]
    fn derived_calendars_require_context() {
        let calendar = temp_calendar();

        assert!(calendar.get_project_context().is_none());
        assert!(calendar.get_derived_calendars().is_empty());
        assert!(!calendar.is_parent());

        // Without a context, remove() is a no-op and does not panic
        let mut calendar = temp_calendar();
        calendar.remove();
    }

    #[test]
    fn display_matches_java_format() {
        let mut calendar = standard_calendar();
        calendar
            .add_calendar_exception_for_date(Some(date(2024, 6, 1)))
            .add(LocalTimeRange::new(Some(time(9, 0)), Some(time(12, 0))));

        let text = calendar.to_string();
        assert!(text.starts_with("[ProjectCalendar\n"));
        assert!(text.contains("   ID=null\n"));
        assert!(text.contains("   name=Standard\n"));
        assert!(text.contains("   baseCalendarName=\n"));
        assert!(text.contains("   [Day MONDAY\n      type=WORKING\n"));
        assert!(text.contains("   [Day SUNDAY\n      type=NON_WORKING\n"));
        assert!(text.contains("   [Exceptions=\n"));
        assert!(text.ends_with("]\n]"));
    }

    /// Java: the `calendar != this` guard in `setParent` -- ignores malformed data that sets the calendar as its own parent.
    #[test]
    fn set_parent_ignores_self_reference() {
        let context = ProjectContext::shared();
        let calendar = context.borrow_mut().get_calendars_mut().add();

        calendar.borrow_mut().set_parent(Some(Rc::clone(&calendar)));

        assert!(calendar.borrow().get_parent().is_none());
    }

    /// `ProjectEntityWithUniqueID` / `ProjectEntityWithMutableUniqueID` delegate to this type's same-named methods.
    #[test]
    fn unique_id_trait_impls_delegate_to_the_declared_methods() {
        let mut calendar = temp_calendar();

        ProjectEntityWithMutableUniqueID::set_unique_id(&mut calendar, Some(11));
        assert_eq!(
            ProjectEntityWithUniqueID::get_unique_id(&calendar),
            Some(11)
        );
    }

    /// Look up the tasks / resources using this calendar by its unique ID (Java's `getTasks` / `getResources`).
    #[test]
    fn tasks_and_resources_filter_by_calendar_unique_id() {
        let file = ProjectFile::new();
        let context = file.borrow().get_project_context();

        let calendar = {
            let mut context_ref = context.borrow_mut();
            context_ref.get_calendars_mut().add()
        };
        calendar.borrow_mut().set_unique_id(Some(5));

        // A calendar without a unique ID (Java's `m_uniqueID.equals(...)` is always false) matches no entities
        let without_id = {
            let mut context_ref = context.borrow_mut();
            context_ref.get_calendars_mut().add()
        };
        without_id.borrow_mut().set_unique_id(None);
        assert!(without_id.borrow().get_tasks().is_empty());
        assert!(without_id.borrow().get_resources().is_empty());
        assert_eq!(without_id.borrow().get_resource_count(), 0);

        let task = ProjectFile::add_task_shared(&file);
        task.borrow_mut().set_calendar_unique_id(Some(5));

        let resource = file.borrow_mut().add_resource();
        resource.borrow_mut().set_calendar_unique_id(Some(5));

        let tasks = calendar.borrow().get_tasks();
        assert_eq!(tasks.len(), 1);
        assert!(Rc::ptr_eq(&tasks[0], &task));

        let resources = calendar.borrow().get_resources();
        assert_eq!(resources.len(), 1);
        assert!(Rc::ptr_eq(&resources[0], &resource));

        assert_eq!(calendar.borrow().get_resource_count(), 1);
    }

    /// Without a context (Java's `m_context` is a strong reference, so this cannot happen) it degrades to an empty collection.
    #[test]
    fn tasks_and_resources_are_empty_without_context() {
        let calendar = temp_calendar();

        assert!(calendar.get_tasks().is_empty());
        assert!(calendar.get_resources().is_empty());
        assert_eq!(calendar.get_resource_count(), 0);
    }

    /// `getDerivedCalendarsForProject` filters out calendars "whose type is `PROJECT` and whose project unique ID does not match".
    #[test]
    fn derived_calendars_for_project_filters_by_project_unique_id() {
        let file = ProjectFile::new();
        let context = file.borrow().get_project_context();

        let (base, derived) = {
            let mut context_ref = context.borrow_mut();
            let base = context_ref.get_calendars_mut().add();
            let derived = context_ref.get_calendars_mut().add();
            (base, derived)
        };
        base.borrow_mut().set_unique_id(Some(1));
        derived.borrow_mut().set_parent(Some(Rc::clone(&base)));

        // Defaults to GLOBAL type and is not filtered by project unique ID
        assert_eq!(base.borrow().get_derived_calendars_for_project(&file).len(), 1);

        // Filtered out when the type is PROJECT and the project unique ID does not match
        derived.borrow_mut().set_type(Some(CalendarType::Project));
        derived.borrow_mut().set_project_unique_id(Some(99));
        assert!(base
            .borrow()
            .get_derived_calendars_for_project(&file)
            .is_empty());

        // Kept when it matches the project unique ID (the project has no unique ID set in this test, so both sides are None)
        derived.borrow_mut().set_project_unique_id(None);
        assert_eq!(base.borrow().get_derived_calendars_for_project(&file).len(), 1);
    }
}
