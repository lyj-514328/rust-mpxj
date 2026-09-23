//! Corresponds to Java: `MSPDIReader.readCalendars` and its helpers
//! (`org/mpxj/mspdi/MSPDIReader.java` lines 375-834), plus the three lookup tables declared at
//! lines 2316-2345 of the same file (`RECURRENCE_TYPES`, `RELATIVE_MAP`, `DAY_MASKS`).
//!
//! This module holds the `impl ReaderContext` block for the calendar section; it is split out of
//! [`crate::mspdi::mspdi_reader`] so that each section can be compared against its Java line range
//! (see the file header of that module).
//!
//! # Java name -> Rust name
//!
//! | Java | Rust |
//! |---|---|
//! | `readCalendars(Project, HashMap<BigInteger, ProjectCalendar>)` (375) | [`ReaderContext::read_calendars`] |
//! | `updateBaseCalendarNames(List<Pair<ProjectCalendar, BigInteger>>, HashMap<BigInteger, ProjectCalendar>)` (411) | [`ReaderContext::update_base_calendar_names`] |
//! | `readCalendar(Project.Calendars.Calendar, HashMap<BigInteger, ProjectCalendar>, List<Pair<...>>)` (433) | [`ReaderContext::read_calendar`] |
//! | `readDay(ProjectCalendar, Project.Calendars.Calendar.WeekDays.WeekDay, boolean)` (481) | [`ReaderContext::read_day`] |
//! | `readNormalDay(ProjectCalendar, Project.Calendars.Calendar.WeekDays.WeekDay)` (506) | [`ReaderContext::read_normal_day`] |
//! | `readExceptionDay(ProjectCalendar, Project.Calendars.Calendar.WeekDays.WeekDay)` (535) | [`ReaderContext::read_exception_day`] |
//! | `readExceptions(Project.Calendars.Calendar, ProjectCalendar)` (566) | [`ReaderContext::read_exceptions`] |
//! | `readException(ProjectCalendar, Project.Calendars.Calendar.Exceptions.Exception)` (584) | [`ReaderContext::read_exception`] |
//! | `readRecurringData(Project.Calendars.Calendar.Exceptions.Exception, LocalDate, LocalDate)` (631) | [`ReaderContext::read_recurring_data`] |
//! | `getRecurrenceType(int)` (707) | [`ReaderContext::get_recurrence_type`] |
//! | `getRelative(int)` (728) | [`ReaderContext::get_relative`] |
//! | `getFrequency(Project.Calendars.Calendar.Exceptions.Exception)` (749) | [`ReaderContext::get_frequency`] |
//! | `readWorkWeeks(Project.Calendars.Calendar, ProjectCalendar)` (765) | [`ReaderContext::read_work_weeks`] |
//! | `readWorkWeek(ProjectCalendar, Project.Calendars.Calendar.WorkWeeks.WorkWeek)` (783) | [`ReaderContext::read_work_week`] |
//! | `readWorkWeekDay(ProjectCalendarWeek, DayOfWeek, Project.Calendars.Calendar.WorkWeeks.WorkWeek.WeekDays.WeekDay)` (815) | [`ReaderContext::read_work_week_day`] |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `List<Pair<ProjectCalendar, BigInteger>> baseCalendars` (`org.mpxj.common.Pair`) | `Vec<(Rc<RefCell<ProjectCalendar>>, i64)>` | Rust has no `Pair`; a tuple carries the same two values in the same order (calendar first, base calendar ID second) |
//! | `HashMap<BigInteger, ProjectCalendar> map`, keyed by the calendar Unique ID | `HashMap<i64, Rc<RefCell<ProjectCalendar>>>` | Same key type for every method Java hands the map to. `BigInteger` is represented by `i64` crate-wide (see the schema layer's differences table) |
//! | `map.put(calendar.getUID(), bc)` — Java's `HashMap` accepts a `null` key, so a `Calendar` without a `UID` element is registered under `null`; `map.get(project.getCalendarUID())` can then find it again | the `UID` element is a mandatory field of the Rust map key type, so a `Calendar` whose `UID` element is absent is not registered at all | A document that omits both `Calendar/UID` and `Project/CalendarUID` would resolve the default calendar through the `null` entry in Java; the Rust side falls back to `findOrCreateDefaultCalendar` instead. `UID` is `required = true` in the MSPDI schema, so real documents are unaffected |
//! | `m_projectFile.getCalendars().findOrCreateDefaultCalendar()` | the mutable calendar container is taken from the project context (`context.get_calendars_mut()`), because [`crate::project_file::ProjectFile::get_calendars`] hands out an immutable `Ref` | Java's `getCalendars()` returns the live container; the split only exists because of the Rust borrow checker |
//! | the JAXB wrapper `Project.Calendars` (`calendars != null`) is flattened into `Project::calendars: Vec<Calendar>` by the schema layer, and so are `Calendar.WeekDays`, `Calendar.Exceptions` and `Calendar.WorkWeeks` | the Java `!= null` guards become "the loop body runs zero times" (`readCalendars`, `readExceptions`, `readWorkWeeks`) or `!calendar.week_days.is_empty()` (`readCalendar`) | The flattened `Vec` cannot tell "wrapper element absent" from "wrapper element present but empty". For `readCalendar` this matters: Java takes the `else` branch only when `<WeekDays>` is absent, so a `<WeekDays/>` element with no `WeekDay` child would leave the day types untouched in Java, while the Rust side fills all seven days with `DayType::DEFAULT` (the `else` branch). The repeated element is `required = true` in the MSPDI schema, so only a hand-written document can tell the two apart |
//! | Calendars are plain Java objects reached through strong references (`ProjectCalendar bc`, `ProjectCalendarWeek week`, `ProjectCalendarException bce`) | every entity is `Rc<RefCell<_>>`, so each mutation goes through `try_borrow_mut` / `try_borrow` and a borrow conflict degrades by skipping that step | The conflict cannot happen on the Java side. Within one call Java mutates the same object through a local variable; the Rust side takes one `RefMut` for the whole sequence so the order of writes is preserved |
//! | `DayOfWeekHelper.getInstance(int)` returns `null` for a value outside 1-7; the value is then passed to `ProjectCalendar.setWorkingDay(null, ...)` (NPE in `DayOfWeekHelper.getValue`) or to `RecurringData.setDayOfWeek(null)` (NPE in `EnumSet.add(null)`) | `get_instance` returns `None` and the call is skipped | Hard convention: library code never panics. The reachable inputs (`DayType` 1-7 apart from 0, `MonthItem` 2-8) are unaffected |
//! | `readExceptionDay` / `readException` / `readWorkWeek` dereference `day.getTimePeriod()` / `xmlWeek.getTimePeriod()` without a null check (NPE when the element is missing) | the optional `time_period` degrades to `None` (no dates) | Rust-specific branch; it keeps a malformed document readable instead of panicking |
//! | `xmlWeekDays.getWeekDay().stream().collect(Collectors.toMap(d -> DayOfWeekHelper.getInstance(d.getDayType().intValue()), d -> d))` — `toMap` throws `IllegalStateException` on a duplicate day and NPE on a null day | a `HashMap<Weekday, &WeekDay>` filled with `insert` (the last `WeekDay` for a day wins) and entries whose `DayType` does not resolve to a day are skipped | Degradation of the two Java exceptions; both can only come from a malformed document |
//! | `NumberHelper.getInteger(Number)` applied to the `BigInteger` fields (`UID`, `Occurrences`, `Period`, `DaysOfWeek`, `MonthDay`) goes through `Math.round(doubleValue())`, `NumberHelper.getInt(Number)` through `Number.intValue()` | the schema holds `Option<i64>`, which is widened by the Rust-specific helpers `number` / `get_integer` / `get_int` in this module before [`NumberHelper`] is called | Keeps the Java rounding/truncation path (`f64` has no `From<i64>`). `BigInteger.intValue()` truncates to the low 32 bits where Rust's `as i32` (used for `DayType`) saturates — the crate-wide difference documented in [`crate::common::number_helper`] |
//! | the `RECURRENCE_TYPES` / `RELATIVE_MAP` / `DAY_MASKS` arrays are `private static final` fields of `MSPDIReader` | module-level `const`s in this file ([`RECURRENCE_TYPES`], [`RELATIVE_MAP`], [`DAY_MASKS`]) | They are only used by the recurring-exception code, so they sit next to it. Index 0 of `RECURRENCE_TYPES` is `null`, which makes the Rust element type `Option<RecurrenceType>` |
//! | `for (DayOfWeek day : DayOfWeek.values())` in `readWorkWeek`, and the explicit SUNDAY-first list in the `readCalendar` `else` branch | a single `const ALL_DAYS` (`java.time.DayOfWeek.values()` order: Monday → Sunday) is used for both | Each day is written independently, so the visit order cannot be observed in the result (the Rust calendar stores the seven days by `DayOfWeekHelper::get_value` index) |
//!
//! # Skipped Java members + reason
//!
//! None.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use chrono::{NaiveDate, Weekday};

use crate::common::boolean_helper::BooleanHelper;
use crate::common::day_of_week_helper::DayOfWeekHelper;
use crate::common::local_date_helper::LocalDateHelper;
use crate::common::number_helper::NumberHelper;
use crate::day_type::DayType;
use crate::local_date_range::LocalDateRange;
use crate::local_time_range::LocalTimeRange;
use crate::mspdi::mspdi_reader::ReaderContext;
use crate::mspdi::schema::calendars::{Calendar, exceptions, week_days, work_weeks};
use crate::mspdi::schema::project::Project;
use crate::project_calendar::ProjectCalendar;
use crate::project_calendar_week::ProjectCalendarWeek;
use crate::recurrence_type::RecurrenceType;
use crate::recurring_data::RecurringData;

/// Lookup table turning the MSPDI `Exception/Type` value into a [`RecurrenceType`].
///
/// Corresponds to Java: `private static final RecurrenceType[] RECURRENCE_TYPES`
///
/// Index 0 is `null` in Java (a `Type` of 0 or a missing `Type` means "not a recurring exception"),
/// which is the `None` element here.
const RECURRENCE_TYPES: [Option<RecurrenceType>; 8] = [
    None,
    Some(RecurrenceType::Daily),
    Some(RecurrenceType::Yearly),  // Absolute
    Some(RecurrenceType::Yearly),  // Relative
    Some(RecurrenceType::Monthly), // Absolute
    Some(RecurrenceType::Monthly), // Relative
    Some(RecurrenceType::Weekly),
    Some(RecurrenceType::Daily),
];

/// Which of the recurrence types are relative.
///
/// Corresponds to Java: `private static final boolean[] RELATIVE_MAP`
const RELATIVE_MAP: [bool; 6] = [false, false, false, true, false, true];

/// Bit value of each day in the MSPDI `Exception/DaysOfWeek` bitmap.
///
/// Corresponds to Java: `private static final int[] DAY_MASKS`
///
/// The index is the `DayOfWeekHelper::get_value` value (Sunday = 1 … Saturday = 7); index 0 is
/// unused, exactly as in Java.
const DAY_MASKS: [i32; 8] = [
    0x00, 0x01, // Sunday
    0x02, // Monday
    0x04, // Tuesday
    0x08, // Wednesday
    0x10, // Thursday
    0x20, // Friday
    0x40, // Saturday
];

/// The order in which the seven days of a week are visited: `java.time.DayOfWeek.values()`.
///
/// Corresponds to Java: `for (DayOfWeek day : DayOfWeek.values())` in `readWorkWeek`; the
/// `readCalendar` `else` branch lists the same days Sunday → Saturday, which writes exactly the
/// same values (see the differences table).
const ALL_DAYS: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];

/// Rust-specific: widen a nullable schema integer (`BigInteger` -> `i64`) to the `f64` that
/// [`NumberHelper`]'s `Number` parameters take.
///
/// Corresponds to Java: the `BigInteger` -> `double` conversion that `NumberHelper` performs
/// internally through `Number.doubleValue()` / `Number.intValue()`.
fn number(value: Option<i64>) -> Option<f64> {
    value.map(|value| value as f64)
}

/// Rust-specific: `NumberHelper.getInteger(Number)` applied to a schema `BigInteger` field.
///
/// Corresponds to Java: `NumberHelper.getInteger(calendar.getUID())` and the like, where the
/// argument is a `BigInteger`.
fn get_integer(value: Option<i64>) -> Option<i32> {
    NumberHelper::get_integer(number(value))
}

/// Rust-specific: `NumberHelper.getInt(Number)` applied to a schema `BigInteger` field.
///
/// Corresponds to Java: `NumberHelper.getInt(exception.getType())` and the like, where the argument
/// is a `BigInteger`.
fn get_int(value: Option<i64>) -> i32 {
    NumberHelper::get_int(number(value))
}

impl ReaderContext {
    /// This method extracts calendar data from an MSPDI file.
    ///
    /// Corresponds to Java: `private void readCalendars(Project project, HashMap<BigInteger, ProjectCalendar> map)`
    ///
    /// The Java `BigInteger` key is the calendar Unique ID, represented as `i64` on the Rust side
    /// (see the schema layer's type mapping).
    pub(crate) fn read_calendars(
        &mut self,
        project: &Project,
        calendar_map: &mut HashMap<i64, Rc<RefCell<ProjectCalendar>>>,
    ) {
        //
        // Java: Project.Calendars calendars = project.getCalendars();
        //       if (calendars != null)
        //       {
        //          List<Pair<ProjectCalendar, BigInteger>> baseCalendars = new ArrayList<>();
        //          for (Project.Calendars.Calendar cal : calendars.getCalendar()) { readCalendar(...); }
        //          updateBaseCalendarNames(baseCalendars, map);
        //       }
        //
        // `Project::calendars` is flattened out of the `Calendars` wrapper, so an absent wrapper is
        // an empty `Vec` and the two loops below simply run zero times — the same set of calls Java
        // makes when the wrapper is `null` (see the differences table).
        //
        let mut base_calendars: Vec<(Rc<RefCell<ProjectCalendar>>, i64)> = Vec::new();

        for cal in &project.calendars {
            self.read_calendar(cal, calendar_map, &mut base_calendars);
        }

        Self::update_base_calendar_names(&base_calendars, calendar_map);

        //
        // Ensure that the default calendar is set in the project properties
        //
        // Java: ProjectCalendar defaultCalendar = map.get(project.getCalendarUID());
        let mut default_calendar = project
            .calendar_uid
            .and_then(|calendar_uid| calendar_map.get(&calendar_uid))
            .cloned();

        if default_calendar.is_none() {
            // Java: defaultCalendar = m_projectFile.getCalendars().findOrCreateDefaultCalendar();
            let context = self.file.borrow().get_project_context();
            if let Ok(mut context) = context.try_borrow_mut() {
                default_calendar = context.get_calendars_mut().find_or_create_default_calendar();
            }
        }

        // Java: m_projectFile.setDefaultCalendar(defaultCalendar);
        if let Ok(mut file) = self.file.try_borrow_mut() {
            file.set_default_calendar(default_calendar);
        }
    }

    /// The way calendars are stored in an MSPDI file means that there can be forward references
    /// between the base calendar unique ID for a derived calendar, and the base calendar itself. To
    /// get around this, we initially populate the base calendar name attribute with the base
    /// calendar unique ID, and now in this method we can convert those ID values into the correct
    /// names.
    ///
    /// Corresponds to Java: `private static void updateBaseCalendarNames(List<Pair<ProjectCalendar, BigInteger>> baseCalendars, HashMap<BigInteger, ProjectCalendar> map)`
    fn update_base_calendar_names(
        base_calendars: &[(Rc<RefCell<ProjectCalendar>>, i64)],
        calendar_map: &HashMap<i64, Rc<RefCell<ProjectCalendar>>>,
    ) {
        for (calendar, base_calendar_id) in base_calendars {
            // Java: ProjectCalendar baseCal = map.get(baseCalendarID);
            let base_calendar = calendar_map.get(base_calendar_id).cloned();

            // Java: if (baseCal != null) { cal.setParent(baseCal); }
            if let Some(base_calendar) = base_calendar {
                if let Ok(mut calendar_ref) = calendar.try_borrow_mut() {
                    calendar_ref.set_parent(Some(base_calendar));
                }
            }
        }
    }

    /// This method extracts data for a single calendar from an MSPDI file.
    ///
    /// Corresponds to Java: `private void readCalendar(Project.Calendars.Calendar calendar, HashMap<BigInteger, ProjectCalendar> map, List<Pair<ProjectCalendar, BigInteger>> baseCalendars)`
    fn read_calendar(
        &mut self,
        calendar: &Calendar,
        calendar_map: &mut HashMap<i64, Rc<RefCell<ProjectCalendar>>>,
        base_calendars: &mut Vec<(Rc<RefCell<ProjectCalendar>>, i64)>,
    ) {
        // Java: ProjectCalendar bc = m_projectFile.addCalendar();
        let bc = match self.file.try_borrow_mut() {
            Ok(mut file) => file.add_calendar(),
            // Rust-specific: a borrow conflict cannot happen in Java; skip this calendar rather than panic.
            Err(_) => return,
        };

        //
        // Java: bc.setUniqueID(NumberHelper.getInteger(calendar.getUID()));
        //       bc.setGUID(calendar.getGUID());
        //       bc.setName(calendar.getName());
        //
        if let Ok(mut bc_ref) = bc.try_borrow_mut() {
            bc_ref.set_unique_id(get_integer(calendar.uid));
            bc_ref.set_guid(calendar.guid);
            bc_ref.set_name(calendar.name.clone());
        }

        // Java: BigInteger baseCalendarID = calendar.getBaseCalendarUID();
        //       if (baseCalendarID != null) { baseCalendars.add(new Pair<>(bc, baseCalendarID)); }
        if let Some(base_calendar_id) = calendar.base_calendar_uid {
            base_calendars.push((Rc::clone(&bc), base_calendar_id));
        }

        // Java: readExceptions(calendar, bc);
        self.read_exceptions(calendar, &bc);

        // Java: boolean readExceptionsFromDays = bc.getCalendarExceptions().isEmpty();
        let read_exceptions_from_days = match bc.try_borrow_mut() {
            Ok(mut bc_ref) => bc_ref.get_calendar_exceptions().is_empty(),
            Err(_) => false,
        };

        //
        // Java: Project.Calendars.Calendar.WeekDays days = calendar.getWeekDays();
        //       if (days != null) { for (...) readDay(...); }
        //       else { bc.setCalendarDayType(<all seven days>, DayType.DEFAULT); }
        //
        if calendar.week_days.is_empty() {
            if let Ok(mut bc_ref) = bc.try_borrow_mut() {
                for day in ALL_DAYS {
                    bc_ref.set_calendar_day_type(day, DayType::Default);
                }
            }
        } else {
            for week_day in &calendar.week_days {
                self.read_day(&bc, week_day, read_exceptions_from_days);
            }
        }

        // Java: readWorkWeeks(calendar, bc);
        self.read_work_weeks(calendar, &bc);

        // Java: map.put(calendar.getUID(), bc);
        if let Some(uid) = calendar.uid {
            calendar_map.insert(uid, Rc::clone(&bc));
        }

        // Java: m_eventManager.fireCalendarReadEvent(bc);
        self.with_event_manager(|manager| manager.fire_calendar_read_event(&bc));
    }

    /// This method extracts data for a single day from an MSPDI file.
    ///
    /// Corresponds to Java: `private void readDay(ProjectCalendar calendar, Project.Calendars.Calendar.WeekDays.WeekDay day, boolean readExceptionsFromDays)`
    fn read_day(
        &mut self,
        calendar: &Rc<RefCell<ProjectCalendar>>,
        day: &week_days::WeekDay,
        read_exceptions_from_days: bool,
    ) {
        // Java: BigInteger dayType = day.getDayType();
        //       if (dayType != null)
        if let Some(day_type) = day.day_type {
            // Java: if (dayType.intValue() == 0)
            if day_type == 0 {
                if read_exceptions_from_days {
                    self.read_exception_day(calendar, day);
                }
            } else {
                self.read_normal_day(calendar, day);
            }
        }
    }

    /// This method extracts data for a normal working day from an MSPDI file.
    ///
    /// Corresponds to Java: `private void readNormalDay(ProjectCalendar calendar, Project.Calendars.Calendar.WeekDays.WeekDay weekDay)`
    fn read_normal_day(
        &mut self,
        calendar: &Rc<RefCell<ProjectCalendar>>,
        week_day: &week_days::WeekDay,
    ) {
        // Java: int dayNumber = weekDay.getDayType().intValue();
        //
        // `read_day` only calls this method once `day_type` is known to be present; the Rust side
        // still degrades instead of unwrapping (see the differences table).
        let Some(day_number) = week_day.day_type else {
            return;
        };
        let day = match DayOfWeekHelper::get_instance(day_number as i32) {
            Some(day) => day,
            // Java: getInstance returns null outside 1-7, which then NPEs inside setWorkingDay.
            None => return,
        };

        let Ok(mut calendar_ref) = calendar.try_borrow_mut() else {
            return;
        };

        // Java: calendar.setWorkingDay(day, BooleanHelper.getBoolean(weekDay.isDayWorking()));
        calendar_ref.set_working_day(day, BooleanHelper::get_boolean(week_day.day_working));

        // Java: ProjectCalendarHours hours = calendar.addCalendarHours(day);
        let hours = calendar_ref.add_calendar_hours(day);

        // Java: Project.Calendars.Calendar.WeekDays.WeekDay.WorkingTimes times = weekDay.getWorkingTimes();
        //       if (times != null) { for (WorkingTime period : times.getWorkingTime()) { ... hours.add(...); } }
        for period in &week_day.working_times {
            let start_time = period.from_time;
            let end_time = period.to_time;

            if let (Some(start_time), Some(end_time)) = (start_time, end_time) {
                hours.add(LocalTimeRange::new(Some(start_time), Some(end_time)));
            }
        }
    }

    /// This method extracts data for an exception day from an MSPDI file.
    ///
    /// Corresponds to Java: `private void readExceptionDay(ProjectCalendar calendar, Project.Calendars.Calendar.WeekDays.WeekDay day)`
    fn read_exception_day(
        &mut self,
        calendar: &Rc<RefCell<ProjectCalendar>>,
        day: &week_days::WeekDay,
    ) {
        // Java: Project.Calendars.Calendar.WeekDays.WeekDay.TimePeriod timePeriod = day.getTimePeriod();
        //       LocalDate fromDate = LocalDateHelper.getLocalDate(timePeriod.getFromDate());
        //       LocalDate toDate = LocalDateHelper.getLocalDate(timePeriod.getToDate());
        let (from_date, to_date) = match &day.time_period {
            Some(time_period) => (
                LocalDateHelper::get_local_date(time_period.from_date),
                LocalDateHelper::get_local_date(time_period.to_date),
            ),
            None => (None, None),
        };

        let Ok(mut calendar_ref) = calendar.try_borrow_mut() else {
            return;
        };

        // Java: ProjectCalendarException exception = calendar.addCalendarException(fromDate, toDate);
        let exception = calendar_ref.add_calendar_exception(from_date, to_date);

        // Java: Project.Calendars.Calendar.WeekDays.WeekDay.WorkingTimes times = day.getWorkingTimes();
        //       if (times != null) { for (WorkingTime period : time) { ... exception.add(...); } }
        for period in &day.working_times {
            let start_time = period.from_time;
            let end_time = period.to_time;

            if let (Some(start_time), Some(end_time)) = (start_time, end_time) {
                exception.add(LocalTimeRange::new(Some(start_time), Some(end_time)));
            }
        }
    }

    /// Reads any exceptions present in the file. This is only used in MSPDI
    /// file versions saved by Project 2007 and later.
    ///
    /// Corresponds to Java: `private void readExceptions(Project.Calendars.Calendar calendar, ProjectCalendar bc)`
    fn read_exceptions(&mut self, calendar: &Calendar, bc: &Rc<RefCell<ProjectCalendar>>) {
        // Java: Project.Calendars.Calendar.Exceptions exceptions = calendar.getExceptions();
        //       if (exceptions != null) { for (Exception exception : exceptions.getException()) readException(bc, exception); }
        //
        // The `Exceptions` wrapper is flattened into `Calendar::exceptions` (see the differences table).
        for exception in &calendar.exceptions {
            self.read_exception(bc, exception);
        }
    }

    /// Read a single calendar exception.
    ///
    /// Corresponds to Java: `private void readException(ProjectCalendar bc, Project.Calendars.Calendar.Exceptions.Exception exception)`
    fn read_exception(
        &mut self,
        bc: &Rc<RefCell<ProjectCalendar>>,
        exception: &exceptions::Exception,
    ) {
        // Java: LocalDate fromDate = LocalDateHelper.getLocalDate(exception.getTimePeriod().getFromDate());
        //       LocalDate toDate = LocalDateHelper.getLocalDate(exception.getTimePeriod().getToDate());
        let (from_date, to_date) = match &exception.time_period {
            Some(time_period) => (
                LocalDateHelper::get_local_date(time_period.from_date),
                LocalDateHelper::get_local_date(time_period.to_date),
            ),
            None => (None, None),
        };

        // Vico Schedule Planner seems to write start and end dates to FromTime and ToTime
        // rather than FromDate and ToDate. This is plain wrong, and appears to be ignored by MS Project,
        // so we will ignore it too!
        if from_date.is_none() && to_date.is_none() {
            return;
        }

        // Java: RecurringData recurringData = readRecurringData(exception, fromDate, toDate);
        //       if (recurringData != null && !recurringData.isValid()) return;
        let recurring_data = self.read_recurring_data(exception, from_date, to_date);

        if let Some(recurring_data) = &recurring_data {
            if !recurring_data.is_valid() {
                return;
            }
        }

        let Ok(mut bc_ref) = bc.try_borrow_mut() else {
            return;
        };

        // Java: ProjectCalendarException bce;
        //       if (recurringData == null) { bce = bc.addCalendarException(fromDate, toDate); }
        //       else { bce = bc.addCalendarException(recurringData); }
        let bce = match recurring_data {
            Some(recurring_data) => bc_ref.add_calendar_exception_recurring(recurring_data),
            None => bc_ref.add_calendar_exception(from_date, to_date),
        };

        // Java: bce.setName(exception.getName());
        bce.set_name(exception.name.clone());

        // Java: Project.Calendars.Calendar.Exceptions.Exception.WorkingTimes times = exception.getWorkingTimes();
        //       if (times != null) { for (WorkingTime period : time) { ... bce.add(...); } }
        for period in &exception.working_times {
            let start_time = period.from_time;
            let end_time = period.to_time;

            if let (Some(start_time), Some(end_time)) = (start_time, end_time) {
                bce.add(LocalTimeRange::new(Some(start_time), Some(end_time)));
            }
        }
    }

    /// Build the recurring data for a calendar exception.
    ///
    /// Corresponds to Java: `private RecurringData readRecurringData(Project.Calendars.Calendar.Exceptions.Exception exception, LocalDate fromDate, LocalDate toDate)`
    ///
    /// Java returns `null` when the exception is not a recurring one (or when the recurrence
    /// collapses to a single date range); that is the `None` case here.
    fn read_recurring_data(
        &self,
        exception: &exceptions::Exception,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
    ) -> Option<RecurringData> {
        // Java: RecurrenceType rt = getRecurrenceType(NumberHelper.getInt(exception.getType()));
        //       if (rt != null) { ... }
        let recurrence_type = self.get_recurrence_type(get_int(exception.r#type))?;

        let mut rd = RecurringData::new();
        rd.set_start_date(from_date);
        rd.set_finish_date(to_date);
        rd.set_recurrence_type(Some(recurrence_type));
        // Java: rd.setRelative(getRelative(NumberHelper.getInt(exception.getType())));
        rd.set_relative(self.get_relative(get_int(exception.r#type)));
        // Java: rd.setOccurrences(NumberHelper.getInteger(exception.getOccurrences()));
        rd.set_occurrences(get_integer(exception.occurrences));

        // Java: switch (rd.getRecurrenceType()) — the value was just assigned from `recurrence_type`.
        match recurrence_type {
            RecurrenceType::Daily => {
                // Java: rd.setFrequency(getFrequency(exception));
                rd.set_frequency(self.get_frequency(exception));
            }

            RecurrenceType::Weekly => {
                // Java: rd.setWeeklyDaysFromBitmap(NumberHelper.getInteger(exception.getDaysOfWeek()), DAY_MASKS);
                rd.set_weekly_days_from_bitmap(get_integer(exception.days_of_week), &DAY_MASKS);
                rd.set_frequency(self.get_frequency(exception));
            }

            RecurrenceType::Monthly => {
                if rd.get_relative() {
                    // Java: rd.setDayOfWeek(DayOfWeekHelper.getInstance(NumberHelper.getInt(exception.getMonthItem()) - 2));
                    if let Some(day) =
                        DayOfWeekHelper::get_instance(get_int(exception.month_item) - 2)
                    {
                        rd.set_day_of_week(day);
                    }
                    // Java: rd.setDayNumber(Integer.valueOf(NumberHelper.getInt(exception.getMonthPosition()) + 1));
                    rd.set_day_number(Some(get_int(exception.month_position) + 1));
                } else {
                    // Java: rd.setDayNumber(NumberHelper.getInteger(exception.getMonthDay()));
                    rd.set_day_number(get_integer(exception.month_day));
                }
                rd.set_frequency(self.get_frequency(exception));
            }

            RecurrenceType::Yearly => {
                if rd.get_relative() {
                    if let Some(day) =
                        DayOfWeekHelper::get_instance(get_int(exception.month_item) - 2)
                    {
                        rd.set_day_of_week(day);
                    }
                    rd.set_day_number(Some(get_int(exception.month_position) + 1));
                } else {
                    rd.set_day_number(get_integer(exception.month_day));
                }
                // Java: rd.setMonthNumber(Integer.valueOf(NumberHelper.getInt(exception.getMonth()) + 1));
                rd.set_month_number(Some(get_int(exception.month) + 1));
            }
        }

        //
        // Flatten daily recurring exceptions if they only result in one date range.
        //
        // Java: if (rd.getRecurrenceType() == RecurrenceType.DAILY && NumberHelper.getInt(rd.getFrequency()) == 1) { rd = null; }
        if rd.get_recurrence_type() == Some(RecurrenceType::Daily)
            && NumberHelper::get_int(rd.get_frequency().map(|value| value as f64)) == 1
        {
            return None;
        }

        Some(rd)
    }

    /// Retrieve the recurrence type.
    ///
    /// Corresponds to Java: `private RecurrenceType getRecurrenceType(int value)`
    fn get_recurrence_type(&self, value: i32) -> Option<RecurrenceType> {
        if value < 0 || value as usize >= RECURRENCE_TYPES.len() {
            None
        } else {
            RECURRENCE_TYPES[value as usize]
        }
    }

    /// Determine if the exception is relative based on the recurrence type integer value.
    ///
    /// Corresponds to Java: `private boolean getRelative(int value)`
    fn get_relative(&self, value: i32) -> bool {
        if value < 0 || value as usize >= RELATIVE_MAP.len() {
            false
        } else {
            RELATIVE_MAP[value as usize]
        }
    }

    /// Retrieve the frequency of an exception.
    ///
    /// Corresponds to Java: `private Integer getFrequency(Project.Calendars.Calendar.Exceptions.Exception exception)`
    fn get_frequency(&self, exception: &exceptions::Exception) -> Option<i32> {
        // Java: Integer period = NumberHelper.getInteger(exception.getPeriod());
        //       if (period == null) { period = Integer.valueOf(1); }
        let period = get_integer(exception.period);
        Some(period.unwrap_or(1))
    }

    /// Read the work weeks associated with this calendar.
    ///
    /// Corresponds to Java: `private void readWorkWeeks(Project.Calendars.Calendar xmlCalendar, ProjectCalendar mpxjCalendar)`
    fn read_work_weeks(
        &mut self,
        xml_calendar: &Calendar,
        mpxj_calendar: &Rc<RefCell<ProjectCalendar>>,
    ) {
        // Java: WorkWeeks ww = xmlCalendar.getWorkWeeks();
        //       if (ww != null) { for (WorkWeek xmlWeek : ww.getWorkWeek()) readWorkWeek(mpxjCalendar, xmlWeek); }
        //
        // The `WorkWeeks` wrapper is flattened into `Calendar::work_weeks` (see the differences table).
        for xml_week in &xml_calendar.work_weeks {
            self.read_work_week(mpxj_calendar, xml_week);
        }
    }

    /// Read a single work week associated with a calendar.
    ///
    /// Corresponds to Java: `private void readWorkWeek(ProjectCalendar mpxjCalendar, Project.Calendars.Calendar.WorkWeeks.WorkWeek xmlWeek)`
    fn read_work_week(
        &mut self,
        mpxj_calendar: &Rc<RefCell<ProjectCalendar>>,
        xml_week: &work_weeks::WorkWeek,
    ) {
        let Ok(mut calendar_ref) = mpxj_calendar.try_borrow_mut() else {
            return;
        };

        // Java: ProjectCalendarWeek week = mpxjCalendar.addWorkWeek();
        let week = calendar_ref.add_work_week();

        // Java: week.setName(xmlWeek.getName());
        week.set_name(xml_week.name.clone());

        // Java: week.setDateRange(new LocalDateRange(
        //          LocalDateHelper.getLocalDate(xmlWeek.getTimePeriod().getFromDate()),
        //          LocalDateHelper.getLocalDate(xmlWeek.getTimePeriod().getToDate())));
        let (from_date, to_date) = match &xml_week.time_period {
            Some(time_period) => (
                LocalDateHelper::get_local_date(time_period.from_date),
                LocalDateHelper::get_local_date(time_period.to_date),
            ),
            None => (None, None),
        };
        week.set_date_range(Some(LocalDateRange::new(from_date, to_date)));

        //
        // Java: WeekDays xmlWeekDays = xmlWeek.getWeekDays();
        //       if (xmlWeekDays != null)
        //       {
        //          Map<DayOfWeek, WeekDay> map = xmlWeekDays.getWeekDay().stream()
        //             .collect(Collectors.toMap(d -> DayOfWeekHelper.getInstance(d.getDayType().intValue()), d -> d));
        //          for (DayOfWeek day : DayOfWeek.values())
        //          {
        //             WeekDay xmlWeekDay = map.get(day);
        //             if (xmlWeekDay == null) { week.setWorkingDay(day, false); }
        //             else { readWorkWeekDay(week, day, xmlWeekDay); }
        //          }
        //       }
        //
        let mut week_day_map: HashMap<Weekday, &work_weeks::week_days::WeekDay> = HashMap::new();

        for week_day in &xml_week.week_days {
            let day = week_day
                .day_type
                .and_then(|day_type| DayOfWeekHelper::get_instance(day_type as i32));

            if let Some(day) = day {
                // Java's Collectors.toMap throws on a duplicate (or unmappable) day; the Rust side
                // keeps the last entry (see the differences table).
                week_day_map.insert(day, week_day);
            }
        }

        for day in ALL_DAYS {
            match week_day_map.get(&day) {
                None => week.set_working_day(day, false),
                Some(xml_week_day) => self.read_work_week_day(week, day, xml_week_day),
            }
        }
    }

    /// Read a day from a work week associated with a calendar.
    ///
    /// Corresponds to Java: `private void readWorkWeekDay(ProjectCalendarWeek week, DayOfWeek day, WeekDay xmlWeekDay)`
    fn read_work_week_day(
        &mut self,
        week: &mut ProjectCalendarWeek,
        day: Weekday,
        xml_week_day: &work_weeks::week_days::WeekDay,
    ) {
        // Java: week.setWorkingDay(day, BooleanHelper.getBoolean(xmlWeekDay.isDayWorking()));
        week.set_working_day(day, BooleanHelper::get_boolean(xml_week_day.day_working));

        // Java: ProjectCalendarHours hours = week.addCalendarHours(day);
        let hours = week.add_calendar_hours(day);

        // Java: Project.Calendars.Calendar.WorkWeeks.WorkWeek.WeekDays.WeekDay.WorkingTimes times = xmlWeekDay.getWorkingTimes();
        //       if (times != null) { for (WorkingTime period : times.getWorkingTime()) { ... hours.add(...); } }
        for period in &xml_week_day.working_times {
            let start_time = period.from_time;
            let end_time = period.to_time;

            if let (Some(start_time), Some(end_time)) = (start_time, end_time) {
                hours.add(LocalTimeRange::new(Some(start_time), Some(end_time)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::io::Cursor;
    use std::rc::Rc;

    use chrono::{NaiveDate, NaiveTime, Weekday};

    use crate::day_type::DayType;
    use crate::mspdi::mspdi_reader::MSPDIReader;
    use crate::project_calendar::ProjectCalendar;
    use crate::project_calendar_hours::ProjectCalendarHours;
    use crate::project_file::ProjectFile;
    use crate::recurrence_type::RecurrenceType;

    /// A calendar with named week days and working times: Sunday is marked as non-working, Monday
    /// works 08:00-12:00 and 13:00-17:00. The project points at it with `CalendarUID`.
    const WEEK_DAYS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Project xmlns="http://schemas.microsoft.com/project">
    <SaveVersion>14</SaveVersion>
    <CalendarUID>1</CalendarUID>
    <Calendars>
        <Calendar>
            <UID>1</UID>
            <GUID>01234567-89AB-CDEF-0123-456789ABCDEF</GUID>
            <Name>Standard</Name>
            <IsBaseCalendar>1</IsBaseCalendar>
            <BaseCalendarUID>-1</BaseCalendarUID>
            <WeekDays>
                <WeekDay>
                    <DayType>1</DayType>
                    <DayWorking>0</DayWorking>
                </WeekDay>
                <WeekDay>
                    <DayType>2</DayType>
                    <DayWorking>1</DayWorking>
                    <WorkingTimes>
                        <WorkingTime><FromTime>08:00:00</FromTime><ToTime>12:00:00</ToTime></WorkingTime>
                        <WorkingTime><FromTime>13:00:00</FromTime><ToTime>17:00:00</ToTime></WorkingTime>
                    </WorkingTimes>
                </WeekDay>
            </WeekDays>
        </Calendar>
    </Calendars>
</Project>"#;

    /// The derived calendar (UID 2) is declared *before* its base calendar (UID 1), so the base
    /// calendar can only be resolved once every calendar has been read.
    const FORWARD_REFERENCE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Project xmlns="http://schemas.microsoft.com/project">
    <SaveVersion>14</SaveVersion>
    <CalendarUID>2</CalendarUID>
    <Calendars>
        <Calendar>
            <UID>2</UID>
            <Name>Night shift</Name>
            <BaseCalendarUID>1</BaseCalendarUID>
            <WeekDays>
                <WeekDay>
                    <DayType>2</DayType>
                    <DayWorking>1</DayWorking>
                    <WorkingTimes>
                        <WorkingTime><FromTime>22:00:00</FromTime><ToTime>23:00:00</ToTime></WorkingTime>
                    </WorkingTimes>
                </WeekDay>
            </WeekDays>
        </Calendar>
        <Calendar>
            <UID>1</UID>
            <Name>Standard</Name>
            <IsBaseCalendar>1</IsBaseCalendar>
            <BaseCalendarUID>-1</BaseCalendarUID>
            <WeekDays>
                <WeekDay>
                    <DayType>2</DayType>
                    <DayWorking>1</DayWorking>
                    <WorkingTimes>
                        <WorkingTime><FromTime>08:00:00</FromTime><ToTime>12:00:00</ToTime></WorkingTime>
                    </WorkingTimes>
                </WeekDay>
            </WeekDays>
        </Calendar>
    </Calendars>
</Project>"#;

    /// Two exceptions in one document:
    /// - calendar 1 (UID 1) has no `<Exceptions>` element, so the `DayType` 0 week day is read as a
    ///   legacy exception day (Project 2003 and earlier);
    /// - calendar 2 (UID 2) uses the `<Exceptions>` element (Project 2007 and later) with a monthly
    ///   relative recurring exception.
    const EXCEPTIONS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Project xmlns="http://schemas.microsoft.com/project">
    <SaveVersion>14</SaveVersion>
    <CalendarUID>1</CalendarUID>
    <Calendars>
        <Calendar>
            <UID>1</UID>
            <Name>Standard</Name>
            <IsBaseCalendar>1</IsBaseCalendar>
            <WeekDays>
                <WeekDay>
                    <DayType>0</DayType>
                    <DayWorking>0</DayWorking>
                    <TimePeriod>
                        <FromDate>2024-12-25T00:00:00</FromDate>
                        <ToDate>2024-12-26T00:00:00</ToDate>
                    </TimePeriod>
                    <WorkingTimes>
                        <WorkingTime><FromTime>09:00:00</FromTime><ToTime>10:00:00</ToTime></WorkingTime>
                    </WorkingTimes>
                </WeekDay>
                <WeekDay>
                    <DayType>2</DayType>
                    <DayWorking>1</DayWorking>
                </WeekDay>
            </WeekDays>
        </Calendar>
        <Calendar>
            <UID>2</UID>
            <Name>Monthly</Name>
            <IsBaseCalendar>1</IsBaseCalendar>
            <WeekDays>
                <WeekDay>
                    <DayType>2</DayType>
                    <DayWorking>1</DayWorking>
                </WeekDay>
            </WeekDays>
            <Exceptions>
                <Exception>
                    <EnteredByOccurrences>1</EnteredByOccurrences>
                    <TimePeriod>
                        <FromDate>2002-01-01T00:00:00</FromDate>
                        <ToDate>2002-05-06T23:59:00</ToDate>
                    </TimePeriod>
                    <Occurrences>3</Occurrences>
                    <Name>Monthly Relative 1</Name>
                    <Type>5</Type>
                    <Period>2</Period>
                    <MonthItem>4</MonthItem>
                    <MonthPosition>0</MonthPosition>
                    <DayWorking>0</DayWorking>
                    <WorkingTimes>
                        <WorkingTime><FromTime>10:00:00</FromTime><ToTime>11:00:00</ToTime></WorkingTime>
                    </WorkingTimes>
                </Exception>
            </Exceptions>
        </Calendar>
    </Calendars>
</Project>"#;

    /// A work week with a day-specific pattern: only Tuesday works, 08:30-12:30.
    const WORK_WEEK_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Project xmlns="http://schemas.microsoft.com/project">
    <SaveVersion>14</SaveVersion>
    <CalendarUID>1</CalendarUID>
    <Calendars>
        <Calendar>
            <UID>1</UID>
            <Name>Standard</Name>
            <IsBaseCalendar>1</IsBaseCalendar>
            <WeekDays>
                <WeekDay>
                    <DayType>2</DayType>
                    <DayWorking>1</DayWorking>
                    <WorkingTimes>
                        <WorkingTime><FromTime>08:00:00</FromTime><ToTime>12:00:00</ToTime></WorkingTime>
                    </WorkingTimes>
                </WeekDay>
            </WeekDays>
            <WorkWeeks>
                <WorkWeek>
                    <TimePeriod>
                        <FromDate>2024-06-03T00:00:00</FromDate>
                        <ToDate>2024-06-07T00:00:00</ToDate>
                    </TimePeriod>
                    <Name>Summer hours</Name>
                    <WeekDays>
                        <WeekDay>
                            <DayType>3</DayType>
                            <DayWorking>1</DayWorking>
                            <WorkingTimes>
                                <WorkingTime><FromTime>08:30:00</FromTime><ToTime>12:30:00</ToTime></WorkingTime>
                            </WorkingTimes>
                        </WeekDay>
                    </WeekDays>
                </WorkWeek>
            </WorkWeeks>
        </Calendar>
    </Calendars>
</Project>"#;

    /// Read a document through the real reader entry point.
    fn read(xml: &str) -> Rc<RefCell<ProjectFile>> {
        MSPDIReader::new()
            .read(&mut Cursor::new(xml.as_bytes()))
            .expect("the document should be read")
    }

    /// The calendar with the given unique ID (the reader stores it in the project's container).
    fn calendar(file: &Rc<RefCell<ProjectFile>>, uid: i32) -> Rc<RefCell<ProjectCalendar>> {
        file.borrow()
            .get_calendar_by_unique_id(Some(uid))
            .expect("the calendar should be present")
    }

    fn time(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).expect("valid time")
    }

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
    }

    /// The scalar fields and the working hours of a named base calendar are stored on the
    /// `ProjectCalendar` created for the `Calendar` element.
    #[test]
    fn reads_week_days_and_working_times() {
        let file = read(WEEK_DAYS_XML);
        let calendar = calendar(&file, 1);

        {
            let calendar_ref = calendar.borrow();

            assert_eq!(calendar_ref.get_unique_id(), Some(1));
            assert_eq!(calendar_ref.get_name(), Some("Standard"));
            assert_eq!(
                calendar_ref.get_guid(),
                Some(uuid::Uuid::parse_str("01234567-89AB-CDEF-0123-456789ABCDEF").expect("valid uuid"))
            );

            // `DayWorking=0` on DayType 1 (Sunday) => NON_WORKING, with no working hours.
            assert_eq!(
                calendar_ref.get_calendar_day_type(Weekday::Sun),
                Some(DayType::NonWorking)
            );
            assert!(!calendar_ref.is_working_day(Weekday::Sun));
            // `readNormalDay` calls `addCalendarHours` for every `WeekDay`, so a non-working day
            // ends up with an empty (not absent) working-hours collection.
            assert_eq!(
                calendar_ref
                    .get_calendar_hours_for_day(Weekday::Sun)
                    .map(ProjectCalendarHours::size),
                Some(0)
            );

            // `DayWorking=1` on DayType 2 (Monday) => WORKING, with both WorkingTime periods.
            assert_eq!(
                calendar_ref.get_calendar_day_type(Weekday::Mon),
                Some(DayType::Working)
            );
            assert!(calendar_ref.is_working_day(Weekday::Mon));

            let hours = calendar_ref
                .get_calendar_hours_for_day(Weekday::Mon)
                .expect("Monday has working hours");
            assert_eq!(hours.size(), 2);
            assert_eq!(hours.get(0).get_start(), Some(time(8, 0)));
            assert_eq!(hours.get(0).get_end(), Some(time(12, 0)));
            assert_eq!(hours.get(1).get_start(), Some(time(13, 0)));
            assert_eq!(hours.get(1).get_end(), Some(time(17, 0)));

            // The week days that carry no `WeekDay` element are left untouched by Java.
            assert_eq!(calendar_ref.get_calendar_day_type(Weekday::Tue), None);
        }

        // `Project/CalendarUID` selects the default calendar.
        let default_calendar = file.borrow().get_default_calendar();
        assert_eq!(
            default_calendar.map(|calendar| calendar.borrow().get_unique_id()),
            Some(Some(1))
        );
    }

    /// A `BaseCalendarUID` forward reference is resolved once every calendar has been read:
    /// calendar 2 is declared first and must still end up with calendar 1 as its parent.
    #[test]
    fn resolves_a_base_calendar_forward_reference() {
        let file = read(FORWARD_REFERENCE_XML);

        assert_eq!(file.borrow().get_calendars().len(), 2);

        let derived = calendar(&file, 2);
        let derived_ref = derived.borrow();
        assert_eq!(derived_ref.get_parent_unique_id(), Some(1));
        assert!(derived_ref.is_derived());

        let parent = derived_ref.get_parent().expect("the parent is resolved");
        assert_eq!(parent.borrow().get_unique_id(), Some(1));
        assert_eq!(parent.borrow().get_name(), Some("Standard"));
        drop(derived_ref);

        // `BaseCalendarUID` -1 does not resolve to a calendar.
        let base = calendar(&file, 1);
        assert_eq!(base.borrow().get_parent_unique_id(), None);
        assert!(!base.borrow().is_derived());
    }

    /// A `DayType` 0 week day is read as an exception day when the calendar carries no
    /// `<Exceptions>` element (the Project 2003 form).
    #[test]
    fn reads_an_exception_day_from_the_week_days() {
        let file = read(EXCEPTIONS_XML);
        let calendar = calendar(&file, 1);

        let mut calendar_ref = calendar.borrow_mut();
        let exceptions = calendar_ref.get_calendar_exceptions();

        assert_eq!(exceptions.len(), 1);
        let exception = &exceptions[0];
        assert_eq!(exception.get_from_date(), Some(date(2024, 12, 25)));
        assert_eq!(exception.get_to_date(), Some(date(2024, 12, 26)));
        assert!(exception.get_recurring().is_none());
        assert_eq!(exception.size(), 1);
        assert_eq!(exception.get(0).get_start(), Some(time(9, 0)));
        assert_eq!(exception.get(0).get_end(), Some(time(10, 0)));

        // The `DayType` 2 week day is read as a normal day at the same time.
        assert_eq!(
            calendar_ref.get_calendar_day_type(Weekday::Mon),
            Some(DayType::Working)
        );
    }

    /// A recurring exception is turned into `RecurringData` with the values from the
    /// `Type` / `Period` / `MonthItem` / `MonthPosition` elements.
    #[test]
    fn reads_a_recurring_exception() {
        let file = read(EXCEPTIONS_XML);
        let calendar = calendar(&file, 2);

        let mut calendar_ref = calendar.borrow_mut();
        let exceptions = calendar_ref.get_calendar_exceptions();

        assert_eq!(exceptions.len(), 1);
        let exception = &exceptions[0];
        assert_eq!(exception.get_name(), Some("Monthly Relative 1"));
        // `ProjectCalendarException.getFromDate` / `getToDate` report the first / last *calculated*
        // occurrence for a recurring exception: the first Monday of January 2002 and the last
        // occurrence inside the exception's date range.
        assert_eq!(exception.get_from_date(), Some(date(2002, 1, 7)));
        assert_eq!(exception.get_to_date(), Some(date(2002, 5, 6)));
        assert_eq!(exception.size(), 1);
        assert_eq!(exception.get(0).get_start(), Some(time(10, 0)));
        assert_eq!(exception.get(0).get_end(), Some(time(11, 0)));

        let recurring = exception.get_recurring().expect("the exception recurs");
        // `Type` 5 => RECURRENCE_TYPES[5] = MONTHLY and RELATIVE_MAP[5] = true.
        assert_eq!(
            recurring.get_recurrence_type(),
            Some(RecurrenceType::Monthly)
        );
        assert!(recurring.get_relative());
        // `MonthItem` 4 => DayOfWeekHelper.getInstance(4 - 2) = Monday.
        assert_eq!(recurring.get_day_of_week(), Some(Weekday::Mon));
        // `MonthPosition` 0 => the first Monday of the month.
        assert_eq!(recurring.get_day_number(), Some(1));
        assert_eq!(recurring.get_frequency(), Some(2));
        assert_eq!(recurring.get_occurrences(), Some(3));
        assert_eq!(recurring.get_start_date(), Some(date(2002, 1, 1)));
        assert_eq!(recurring.get_finish_date(), Some(date(2002, 5, 6)));
    }

    /// A work week keeps its date range and applies the day-specific pattern to the calendar week
    /// it belongs to: Tuesday works 08:30-12:30, the days without a `WeekDay` element do not work.
    #[test]
    fn reads_a_work_week() {
        let file = read(WORK_WEEK_XML);
        let calendar = calendar(&file, 1);

        let calendar_ref = calendar.borrow();
        let weeks = calendar_ref.get_work_weeks();

        assert_eq!(weeks.len(), 1);
        let week = &weeks[0];
        assert_eq!(week.get_name(), Some("Summer hours"));
        assert_eq!(
            week.get_date_range().and_then(|range| range.get_start()),
            Some(date(2024, 6, 3))
        );
        assert_eq!(
            week.get_date_range().and_then(|range| range.get_end()),
            Some(date(2024, 6, 7))
        );

        // `DayType` 3 => Tuesday, `DayWorking` 1.
        assert_eq!(
            week.get_calendar_day_type(Weekday::Tue),
            Some(DayType::Working)
        );
        let hours = week
            .get_calendar_hours_for_day(Weekday::Tue)
            .expect("Tuesday has working hours");
        assert_eq!(hours.size(), 1);
        assert_eq!(hours.get(0).get_start(), Some(time(8, 30)));
        assert_eq!(hours.get(0).get_end(), Some(time(12, 30)));

        // Days with no `WeekDay` element are marked as non-working; `setWorkingDay(day, false)`
        // leaves an empty working-hours collection behind, as Java's `setCalendarDayType` does.
        assert_eq!(
            week.get_calendar_day_type(Weekday::Mon),
            Some(DayType::NonWorking)
        );
        assert_eq!(
            week.get_calendar_hours_for_day(Weekday::Mon)
                .map(ProjectCalendarHours::size),
            Some(0)
        );

        // The calendar itself is unaffected by the work week.
        assert_eq!(
            calendar_ref.get_calendar_day_type(Weekday::Mon),
            Some(DayType::Working)
        );
    }
}
