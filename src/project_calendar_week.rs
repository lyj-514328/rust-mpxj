//! Corresponds to Java: `org/mpxj/ProjectCalendarWeek.java`
//!
//! Skipped Java methods: none.
//!
//! # The `RecurringData` interface used
//!
//! [`ProjectCalendarWeek::convert_to_recurring_exceptions`] relies on `org.mpxj.RecurringData`
//! (corresponding to the already-ported [`crate::recurring_data::RecurringData`]; `src/lib.rs` already declares
//! `pub mod recurring_data;`). This file uses `RecurringData` with the following signature:
//!
//! ```text
//! RecurringData::new() -> RecurringData
//! set_recurrence_type(Option<RecurrenceType>)
//! set_start_date(Option<NaiveDate>)
//! set_finish_date(Option<NaiveDate>)
//! set_use_end_date(bool)
//! set_weekly_day(Weekday, bool)
//! ```
//!
//! # Differences from Java
//!
//! - Java's `ProjectCalendarWeek extends ProjectCalendarDays`; Rust has no inheritance, so
//! [`ProjectCalendarWeek`] holds a [`ProjectCalendarDays`] by field composition and forwards each
//! base class method (see the "Forwarding ProjectCalendarDays" section at the end of the file).
//! - Java throws `UnsupportedOperationException` when `m_dateRange == null`; the Rust side
//! [`ProjectCalendarWeek::convert_to_recurring_exceptions`] returns `None`;
//! when the date range start or end is `null` (Java throws an NPE) it likewise returns `None`.
//! - Java does not override `equals` / `hashCode` (it keeps `Object`'s reference semantics), but implements
//! `compareTo`. Rust's `Ord` requires `Eq` as a supertrait, so `PartialEq` / `Hash`
//! are kept consistent with `Ord`: they all use only the start date of the date range.
//! - `toString()` uses `PrintWriter.println`, so Java outputs a platform-dependent line separator and appends
//! a line separator at the end; the Rust side always uses `\n` with no trailing line separator. `DayType` does not override
//! `toString` in Java, so it prints the enum constant name (`WORKING` / `NON_WORKING` / `DEFAULT`),
//! and the Rust side prints the same text.

use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};

use chrono::{NaiveDate, Weekday};

use crate::common::local_date_helper::LocalDateHelper;
use crate::day_type::DayType;
use crate::local_date_range::LocalDateRange;
use crate::local_time_range::LocalTimeRange;
use crate::project_calendar_days::{ProjectCalendarDays, JAVA_DAY_OF_WEEK_ORDER};
use crate::project_calendar_exception::ProjectCalendarException;
use crate::project_calendar_hours::ProjectCalendarHours;
use crate::recurrence_type::RecurrenceType;
use crate::recurring_data::RecurringData;

/// A work week valid for a specified date range (excluding exception dates).
///
/// Corresponds to Java: `public class ProjectCalendarWeek extends ProjectCalendarDays implements Comparable<ProjectCalendarWeek>`
#[derive(Debug, Clone)]
pub struct ProjectCalendarWeek {
/// Corresponds to Java's inheritance `extends ProjectCalendarDays`
    days: ProjectCalendarDays,
/// Corresponds to Java field `private LocalDateRange m_dateRange` (`null` means the default work week)
    date_range: Option<LocalDateRange>,
}

impl ProjectCalendarWeek {
/// Constructor.
    ///
/// Corresponds to Java: the implicit no-argument constructor `ProjectCalendarWeek()`
    pub fn new() -> Self {
        Self {
            days: ProjectCalendarDays::new(),
            date_range: None,
        }
    }

/// Get the effective date range of this work week (returns `None` for the default work week).
    ///
    /// Corresponds to Java: `public LocalDateRange getDateRange()`
    pub fn get_date_range(&self) -> Option<LocalDateRange> {
        self.date_range
    }

/// Set the effective date range of this work week (`None` means the default work week).
    ///
    /// Corresponds to Java: `public void setDateRange(LocalDateRange range)`
    pub fn set_date_range(&mut self, range: Option<LocalDateRange>) {
        self.date_range = range;
    }

/// Convert this work week into the equivalent list of recurring exceptions.
    ///
    /// Corresponds to Java: `public List<ProjectCalendarException> convertToRecurringExceptions(LocalDate earliestStartDate, LocalDate latestFinishDate)`
    ///
/// The default work week (`date_range` is `None`) cannot be expanded; Java throws
/// `UnsupportedOperationException`, and the Rust side returns `None`.
/// Each day that is not `DEFAULT` generates a weekly recurring exception, with the recurrence range limited to
/// the passed-in bounds and this work week's range.
    pub fn convert_to_recurring_exceptions(
        &self,
        earliest_start_date: Option<NaiveDate>,
        latest_finish_date: Option<NaiveDate>,
    ) -> Option<Vec<ProjectCalendarException>> {
        // Java: We can't expand the default week
        let date_range = self.date_range?;

// Avoid generating exceptions beyond the passed-in bounds
        let mut from_date = date_range.get_start()?;
        if let Some(earliest_start_date) = earliest_start_date {
            if LocalDateHelper::compare(Some(earliest_start_date), Some(from_date)) > 0 {
                from_date = earliest_start_date;
            }
        }

        let mut to_date = date_range.get_end()?;
        if let Some(latest_finish_date) = latest_finish_date {
            if LocalDateHelper::compare(Some(to_date), Some(latest_finish_date)) > 0 {
                to_date = latest_finish_date;
            }
        }

        let mut result = Vec::new();

// Java: for (DayOfWeek day : DayOfWeek.values())  —— Monday first
        for day in JAVA_DAY_OF_WEEK_ORDER {
            if self.get_calendar_day_type(day) == Some(DayType::Default) {
                continue;
            }

            let mut recurrence = RecurringData::new();
            recurrence.set_recurrence_type(Some(RecurrenceType::Weekly));
            recurrence.set_start_date(Some(from_date));
            recurrence.set_finish_date(Some(to_date));
            recurrence.set_use_end_date(true);
            recurrence.set_weekly_day(day, true);

            let mut exception = ProjectCalendarException::new_from_recurring(recurrence);
            // Java: TODO: for consistency this should never be null, just empty?
            if let Some(hours) = self.get_calendar_hours_for_day(day) {
                exception.add_all(hours);
            }

            result.push(exception);
        }

        Some(result)
    }

    //
// The following methods correspond to the methods Java inherits from ProjectCalendarDays (forwarding)
    //

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

/// Corresponds to Java: `addCalendarHours(DayOfWeek day)` (inherited from `ProjectCalendarDays`)
    pub fn add_calendar_hours(&mut self, day: Weekday) -> &mut ProjectCalendarHours {
        self.days.add_calendar_hours(day)
    }

/// Corresponds to Java: `removeCalendarHours(DayOfWeek day)` (inherited from `ProjectCalendarDays`)
    pub fn remove_calendar_hours(&mut self, day: Weekday) {
        self.days.remove_calendar_hours(day);
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

impl Default for ProjectCalendarWeek {
    fn default() -> Self {
        Self::new()
    }
}

impl Ord for ProjectCalendarWeek {
    /// Corresponds to Java: `@Override public int compareTo(ProjectCalendarWeek o)`:
/// Compares the start date of the date range (`null` is treated as greater).
    fn cmp(&self, other: &Self) -> Ordering {
        let lhs = self.date_range.and_then(|range| range.get_start());
        let rhs = other.date_range.and_then(|range| range.get_start());

        match LocalDateHelper::compare(lhs, rhs) {
            value if value < 0 => Ordering::Less,
            0 => Ordering::Equal,
            _ => Ordering::Greater,
        }
    }
}

impl PartialOrd for ProjectCalendarWeek {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ProjectCalendarWeek {
/// Note: Java does not override `equals` (it keeps reference equality); to stay consistent with `Ord`, the Rust side
/// compares by the start date of the date range.
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for ProjectCalendarWeek {}

impl Hash for ProjectCalendarWeek {
/// Consistent with [`ProjectCalendarWeek::eq`]: only the start date of the date range is used.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.date_range
            .and_then(|range| range.get_start())
            .hash(state);
    }
}

impl Display for ProjectCalendarWeek {
    /// Corresponds to Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "[ProjectCalendarWeek")?;
        writeln!(f, "   name={}", self.get_name().unwrap_or("null"))?;
        writeln!(f, "   date_range={}", optional_range(self.date_range))?;

        for day in JAVA_DAY_OF_WEEK_ORDER {
            writeln!(f, "   [Day {}", java_day_of_week_name(day))?;
            writeln!(f, "      type={}", day_type_name(self.get_calendar_day_type(day)))?;
            writeln!(
                f,
                "      hours={}",
                self.get_calendar_hours_for_day(day)
                    .map_or_else(|| "null".to_string(), |hours| hours.to_string())
            )?;
            writeln!(f, "   ]")?;
        }

        write!(f, "]")
    }
}

/// Prints a nullable date range following Java conventions (`null` → `null`).
fn optional_range(range: Option<LocalDateRange>) -> String {
    range.map_or_else(|| "null".to_string(), |range| range.to_string())
}

/// Corresponds to Java enum `DayOfWeek`'s `name()` (`MONDAY` … `SUNDAY`).
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

/// Corresponds to Java enum `DayType`'s `name()` (`DayType` does not override `toString`).
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

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
    }

    fn week() -> ProjectCalendarWeek {
        let mut week = ProjectCalendarWeek::new();
        week.set_date_range(Some(LocalDateRange::new(
            Some(date(2024, 1, 1)),
            Some(date(2024, 1, 7)),
        )));
        week
    }

    #[test]
    fn initial_state() {
        let week = ProjectCalendarWeek::new();
        assert_eq!(week.get_date_range(), None);
        assert_eq!(week.get_name(), None);
        for hours in week.get_calendar_hours() {
            assert!(hours.is_none());
        }
    }

    #[test]
    fn date_range_is_stored() {
        let mut week = ProjectCalendarWeek::new();
        let range = LocalDateRange::new(Some(date(2024, 1, 1)), Some(date(2024, 1, 7)));
        week.set_date_range(Some(range));
        assert_eq!(week.get_date_range(), Some(range));
        assert_eq!(week.get_date_range().and_then(|range| range.get_start()), Some(date(2024, 1, 1)));

        week.set_date_range(None);
        assert_eq!(week.get_date_range(), None);
    }

    #[test]
    fn forwarding_of_project_calendar_days() {
        let mut week = week();
        week.set_name(Some("第一周".to_string()));
        week.add_default_calendar_days();
        week.add_default_calendar_hours();

        assert_eq!(week.get_name(), Some("第一周"));
        assert_eq!(week.get_calendar_day_type(Weekday::Mon), Some(DayType::Working));
        assert_eq!(
            week.get_calendar_hours_for_day(Weekday::Mon)
                .map(ProjectCalendarHours::size),
            Some(2)
        );
        assert_eq!(
            week.get_calendar_hours_for_day(Weekday::Sun)
                .map(ProjectCalendarHours::size),
            Some(0)
        );
        assert_eq!(week.add_calendar_hours(Weekday::Sat).size(), 0);
        week.remove_calendar_hours(Weekday::Sat);
        assert_eq!(week.get_calendar_hours_for_day(Weekday::Sat), None);
    }

    #[test]
    fn default_week_cannot_be_expanded() {
        let week = ProjectCalendarWeek::new();
        assert!(week.convert_to_recurring_exceptions(None, None).is_none());
    }

    #[test]
    fn convert_to_recurring_exceptions_skips_default_days() {
        let mut week = week();
// When no DayType is set all are null (not DEFAULT), so all 7 days generate exceptions
        let exceptions = week
            .convert_to_recurring_exceptions(None, None)
            .expect("working week can be expanded");
        assert_eq!(exceptions.len(), 7);

// Days marked DEFAULT are skipped
        week.set_calendar_day_type(Weekday::Mon, DayType::Default);
        let exceptions = week
            .convert_to_recurring_exceptions(None, None)
            .expect("working week can be expanded");
        assert_eq!(exceptions.len(), 6);
        assert!(exceptions.iter().all(|exception| exception.get_recurring().is_some()));
    }

    #[test]
    fn convert_to_recurring_exceptions_copies_hours() {
        let mut week = week();
        week.add_default_calendar_days();
        week.add_default_calendar_hours();

        let exceptions = week
            .convert_to_recurring_exceptions(Some(date(2024, 1, 1)), Some(date(2024, 1, 7)))
            .expect("working week can be expanded");

        assert_eq!(exceptions.len(), 7);
// Each exception carries the working hours of the corresponding day (empty for weekends)
        let sizes: Vec<usize> = exceptions.iter().map(ProjectCalendarException::size).collect();
        assert_eq!(sizes, vec![2, 2, 2, 2, 2, 0, 0]);
    }

    #[test]
    fn compare_uses_date_range_start() {
        let first = week();
        let mut second = week();
        second.set_date_range(Some(LocalDateRange::new(
            Some(date(2024, 1, 8)),
            Some(date(2024, 1, 14)),
        )));

        assert!(first < second);
        assert_eq!(first.cmp(&second), Ordering::Less);

        let mut first_copy = week();
        first_copy.set_name(Some("副本".to_string()));
        assert_eq!(first, first_copy);
    }

    #[test]
    fn display_matches_java_format() {
        let mut week = week();
        week.set_name(Some("第一周".to_string()));
        week.set_calendar_day_type(Weekday::Mon, DayType::Working);

        let text = week.to_string();
        assert!(text.starts_with("[ProjectCalendarWeek\n"));
        assert!(text.contains("   name=第一周\n"));
        assert!(text.contains("   date_range=[LocalDateRange start=2024-01-01 end=2024-01-07]\n"));
        assert!(text.contains("   [Day MONDAY\n      type=WORKING\n      hours=null\n   ]\n"));
        assert!(text.contains("   [Day SUNDAY\n      type=null\n      hours=null\n   ]\n"));
        assert!(text.ends_with("]\n]"));
    }
}
