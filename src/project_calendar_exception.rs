//! Corresponds to Java: `org/mpxj/ProjectCalendarException.java`
//!
//! Skipped Java methods: none.
//!
//! # The `RecurringData` interface used
//!
//! This file depends on `org.mpxj.RecurringData` (corresponding to the already-ported [`crate::recurring_data::RecurringData`];
//! `src/lib.rs` already declares `pub mod recurring_data;`).
//! This file uses `RecurringData` through the following signatures (each mapping one-to-one to Java getters/setters):
//!
//! ```text
//! RecurringData::new() -> RecurringData
//! set_recurrence_type(Option<RecurrenceType>)
//! get_recurrence_type() -> Option<RecurrenceType>
//! get_frequency() -> Option<i32>          // Java: Integer getFrequency()
//! get_calculated_first_date() -> NaiveDate // Java: LocalDate getCalculatedFirstDate()
//! get_calculated_last_date() -> NaiveDate  // Java: LocalDate getCalculatedLastDate()
//! get_dates() -> Vec<NaiveDate>            // Java: LocalDate[] getDates()
//! impl Display                             // Java: toString()
//! ```
//!
//! # Differences from Java
//!
//! - Java's `ProjectCalendarException extends ProjectCalendarHours` (and thus `AbstractList`),
//! Rust has no inheritance: [`ProjectCalendarException`] holds a [`ProjectCalendarHours`] by composition,
//! and forwards the base class's collection methods one by one (`add` / `set` / `get` / `iter` / `size` / `clear` / `add_all`).
//! - On the Java side `equals` / `hashCode` come from `AbstractList` (comparing only list contents),
//! whereas `compareTo` compares the start date. Java itself has this `equals` and `compareTo`
//! inconsistency: the Rust side keeps the same semantics — `PartialEq` / `Hash` look only at working time contents,
//! while `Ord` looks only at the start date.
//! - Java's `contains(LocalDateTime)` / `contains(LocalDate)` return
//! `false` when the argument is `null`; Rust arguments cannot be `null`, so that branch is unreachable. If the start/end dates are `null` (Java throws
//! an NPE), the Rust side returns `false`.

use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::slice::Iter;

use chrono::{NaiveDate, NaiveDateTime};

use crate::common::local_date_helper::LocalDateHelper;
use crate::local_time_range::LocalTimeRange;
use crate::project_calendar_hours::ProjectCalendarHours;
use crate::recurrence_type::RecurrenceType;
use crate::recurring_data::RecurringData;

/// A calendar exception date (which may include recurrence), describing exceptions in a base calendar/resource calendar.
///
/// Corresponds to Java: `public final class ProjectCalendarException extends ProjectCalendarHours implements Comparable<ProjectCalendarException>`
#[derive(Debug, Clone)]
pub struct ProjectCalendarException {
/// Corresponds to Java's inheritance relationship `extends ProjectCalendarHours`
    hours: ProjectCalendarHours,
    /// Corresponds to Java field `private final LocalDate m_fromDate`
    from_date: Option<NaiveDate>,
    /// Corresponds to Java field `private final LocalDate m_toDate`
    to_date: Option<NaiveDate>,
    /// Corresponds to Java field `private final RecurringData m_recurring`
    recurring: Option<RecurringData>,
    /// Corresponds to Java field `private String m_name`
    name: Option<String>,
}

impl ProjectCalendarException {
/// Constructor (date range only).
    ///
    /// Corresponds to Java: `ProjectCalendarException(LocalDate fromDate, LocalDate toDate)`
    pub fn new(from_date: Option<NaiveDate>, to_date: Option<NaiveDate>) -> Self {
        Self::new_with_recurring(from_date, to_date, None)
    }

/// Constructor (recurrence only).
    ///
    /// Corresponds to Java: `ProjectCalendarException(RecurringData recurringData)`
    ///
/// Java expresses this through constructor overloads; Rust cannot overload, so a `_from_recurring` suffix distinguishes it.
    pub fn new_from_recurring(recurring: RecurringData) -> Self {
        Self::new_with_recurring(None, None, Some(recurring))
    }

/// Constructor (date range + recurrence).
    ///
    /// Corresponds to Java: `ProjectCalendarException(LocalDate fromDate, LocalDate toDate, RecurringData recurringData)`
    pub fn new_with_recurring(
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
        recurring: Option<RecurringData>,
    ) -> Self {
        Self {
            hours: ProjectCalendarHours::new(),
            from_date,
            to_date,
            recurring,
            name: None,
        }
    }

/// Get the exception name.
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

/// Set the exception name.
    ///
    /// Corresponds to Java: `public void setName(String name)`
    pub fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }

/// Get the start date; for recurring exceptions, the computed first occurrence date.
    ///
    /// Corresponds to Java: `public LocalDate getFromDate()`
    pub fn get_from_date(&self) -> Option<NaiveDate> {
        match &self.recurring {
            None => self.from_date,
            Some(recurring) => Some(recurring.get_calculated_first_date()),
        }
    }

/// Get the end date; for recurring exceptions, the computed last occurrence date.
    ///
    /// Corresponds to Java: `public LocalDate getToDate()`
    pub fn get_to_date(&self) -> Option<NaiveDate> {
        match &self.recurring {
            None => self.to_date,
            Some(recurring) => Some(recurring.get_calculated_last_date()),
        }
    }

/// Get the recurrence data (`None` for non-recurring exceptions).
    ///
    /// Corresponds to Java: `public RecurringData getRecurring()`
    pub fn get_recurring(&self) -> Option<&RecurringData> {
        self.recurring.as_ref()
    }

/// Whether there are working times (i.e. `size() != 0`).
    ///
    /// Corresponds to Java: `public boolean getWorking()`
    pub fn get_working(&self) -> bool {
        self.hours.size() != 0
    }

/// Expand this exception into a list of exceptions.
    ///
    /// Corresponds to Java: `public List<ProjectCalendarException> getExpandedExceptions()`
    ///
/// A non-recurring exception, or one that recurs daily with a frequency of 1, returns only itself; other recurring exceptions are expanded
/// into one single-day exception per computed date, copying all working times.
    pub fn get_expanded_exceptions(&self) -> Vec<ProjectCalendarException> {
        let mut result = Vec::new();

        match &self.recurring {
            None => result.push(self.clone()),
            Some(recurring) => {
                // Java: m_recurring.getRecurrenceType() == DAILY && NumberHelper.getInt(m_recurring.getFrequency()) == 1
                let single_day = recurring.get_recurrence_type() == Some(RecurrenceType::Daily)
                    && recurring
                        .get_frequency()
                        .map_or(0, |frequency| frequency as i32)
                        == 1;

                if single_day {
                    result.push(self.clone());
                } else {
                    let range_count = self.hours.size();
                    for date in recurring.get_dates().iter().copied() {
                        let mut new_exception =
                            ProjectCalendarException::new(Some(date), Some(date));
                        for range_index in 0..range_count {
                            new_exception.add(self.hours.get(range_index));
                        }
                        result.push(new_exception);
                    }
                }
            }
        }

        result
    }

/// Determine whether the given date-time falls within the date range covered by this exception.
    ///
    /// Corresponds to Java: `public boolean contains(LocalDateTime date)`
    ///
/// Java implements this identically to `contains(LocalDate)` (take the `LocalDate` first, then compare); it is reused directly here.
    pub fn contains_date_time(&self, date: NaiveDateTime) -> bool {
        self.contains_date(date.date())
    }

/// Determine whether the given date falls within the date range covered by this exception.
    ///
    /// Corresponds to Java: `public boolean contains(LocalDate date)`
    pub fn contains_date(&self, date: NaiveDate) -> bool {
        match (self.get_from_date(), self.get_to_date()) {
            (Some(from_date), Some(to_date)) => {
                LocalDateHelper::compare_with_range(from_date, to_date, date) == 0
            }
// Java: throws an NPE when the start/end dates are null; the Rust side degrades gracefully to false
            _ => false,
        }
    }

/// Determine whether the given exception overlaps this exception on any date.
    ///
    /// Corresponds to Java: `public boolean contains(ProjectCalendarException exception)`
    ///
/// Java provides three `contains` overloads; Rust has no overloading, so they are named separately
    /// `contains_date_time` / `contains_date` / `contains_exception`.
/// The null semantics match Java's `LocalDateHelper.compare` (`null` is treated as greater).
    pub fn contains_exception(&self, exception: &ProjectCalendarException) -> bool {
        !(LocalDateHelper::compare(self.get_to_date(), exception.get_from_date()) < 0
            || LocalDateHelper::compare(exception.get_to_date(), self.get_from_date()) < 0)
    }

/// Get the collection of working times inside this exception (corresponds to Java's `extends ProjectCalendarHours` relationship).
    ///
/// Corresponds to Java: `this` (on the Rust side an accessor expresses the flattened inheritance relationship)
    pub fn get_hours(&self) -> &ProjectCalendarHours {
        &self.hours
    }

    //
// The following methods correspond to the collection operations Java inherits from ProjectCalendarHours
    //

/// Corresponds to Java: `add(LocalTimeRange range)` (inherited from `ProjectCalendarHours`)
    pub fn add(&mut self, range: LocalTimeRange) -> bool {
        self.hours.add(range)
    }

/// Corresponds to Java: `set(int index, LocalTimeRange value)` (inherited from `ProjectCalendarHours`)
    pub fn set(&mut self, index: usize, value: LocalTimeRange) -> Option<LocalTimeRange> {
        self.hours.set(index, value)
    }

/// Corresponds to Java: `get(int index)` (inherited from `ProjectCalendarHours`)
    pub fn get(&self, index: usize) -> LocalTimeRange {
        self.hours.get(index)
    }

/// Corresponds to Java: `iterator()` (inherited from `ProjectCalendarHours`)
    pub fn iter(&self) -> Iter<'_, LocalTimeRange> {
        self.hours.iter()
    }

/// Corresponds to Java: `size()` (inherited from `ProjectCalendarHours`)
    pub fn size(&self) -> usize {
        self.hours.size()
    }

/// Corresponds to Java: `clear()` (inherited from `ProjectCalendarHours`)
    pub fn clear(&mut self) {
        self.hours.clear();
    }

/// Corresponds to Java: `addAll(Collection)` (inherited from `ProjectCalendarHours`)
    pub fn add_all(&mut self, other: &ProjectCalendarHours) -> bool {
        self.hours.add_all(other)
    }
}

impl<'a> IntoIterator for &'a ProjectCalendarException {
    type Item = &'a LocalTimeRange;
    type IntoIter = Iter<'a, LocalTimeRange>;

    fn into_iter(self) -> Self::IntoIter {
        self.hours.iter()
    }
}

impl PartialEq for ProjectCalendarException {
/// Corresponds to Java: `AbstractList.equals(Object)`: compares only working time contents.
    fn eq(&self, other: &Self) -> bool {
        self.hours == other.hours
    }
}

impl Eq for ProjectCalendarException {}

impl Hash for ProjectCalendarException {
/// Corresponds to Java: `AbstractList.hashCode()`: computes the hash only from working time contents.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hours.hash(state);
    }
}

impl Ord for ProjectCalendarException {
    /// Corresponds to Java: `@Override public int compareTo(ProjectCalendarException o)`:
/// Compare by start date (`null` is treated as greater).
    fn cmp(&self, other: &Self) -> Ordering {
        match LocalDateHelper::compare(self.get_from_date(), other.get_from_date()) {
            value if value < 0 => Ordering::Less,
            0 => Ordering::Equal,
            _ => Ordering::Greater,
        }
    }
}

impl PartialOrd for ProjectCalendarException {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for ProjectCalendarException {
    /// Corresponds to Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[ProjectCalendarException")?;

        if let Some(name) = &self.name {
            if !name.is_empty() {
                write!(f, " name={}", name)?;
            }
        }

        write!(f, " working={}", self.get_working())?;

        match &self.recurring {
            None => {
                write!(f, " fromDate={}", optional_date(self.from_date))?;
                write!(f, " toDate={}", optional_date(self.to_date))?;
            }
            Some(recurring) => write!(f, " recurring={}", recurring)?,
        }

        for range in &self.hours {
            write!(f, "{}", range)?;
        }

        write!(f, "]")
    }
}

/// Output a nullable date following Java convention (`null` → `null`).
fn optional_date(date: Option<NaiveDate>) -> String {
    date.map_or_else(|| "null".to_string(), |date| date.to_string())
}

#[cfg(test)]
mod tests {
    use chrono::NaiveTime;

    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
    }

    fn range(from: u32, to: u32) -> LocalTimeRange {
        LocalTimeRange::new(
            NaiveTime::from_hms_opt(from, 0, 0),
            NaiveTime::from_hms_opt(to, 0, 0),
        )
    }

    #[test]
    fn single_day_exception() {
        let mut exception = ProjectCalendarException::new(Some(date(2024, 1, 1)), Some(date(2024, 1, 1)));

        assert_eq!(exception.get_from_date(), Some(date(2024, 1, 1)));
        assert_eq!(exception.get_to_date(), Some(date(2024, 1, 1)));
        assert!(exception.get_recurring().is_none());
        assert!(!exception.get_working());

        exception.add(range(9, 12));
        assert!(exception.get_working());
        assert_eq!(exception.size(), 1);
        assert_eq!(exception.get(0), range(9, 12));
    }

    #[test]
    fn contains_date_and_date_time() {
        let exception = ProjectCalendarException::new(Some(date(2024, 1, 8)), Some(date(2024, 1, 12)));

        assert!(exception.contains_date(date(2024, 1, 8)));
        assert!(exception.contains_date(date(2024, 1, 10)));
        assert!(exception.contains_date(date(2024, 1, 12)));
        assert!(!exception.contains_date(date(2024, 1, 7)));
        assert!(!exception.contains_date(date(2024, 1, 13)));

        let date_time = date(2024, 1, 10)
            .and_hms_opt(9, 30, 0)
            .expect("valid date-time");
        assert!(exception.contains_date_time(date_time));
        assert!(!exception.contains_date_time(
            date(2024, 1, 20)
                .and_hms_opt(9, 30, 0)
                .expect("valid date-time")
        ));

// When the start/end dates are missing (Java throws an NPE), Rust returns false
        let empty = ProjectCalendarException::new(None, None);
        assert!(!empty.contains_date(date(2024, 1, 8)));
    }

    #[test]
    fn contains_exception_detects_overlap() {
        let first = ProjectCalendarException::new(Some(date(2024, 1, 1)), Some(date(2024, 1, 10)));
        let second = ProjectCalendarException::new(Some(date(2024, 1, 10)), Some(date(2024, 1, 20)));
        let third = ProjectCalendarException::new(Some(date(2024, 1, 11)), Some(date(2024, 1, 20)));

        assert!(first.contains_exception(&second));
        assert!(second.contains_exception(&first));
        assert!(!first.contains_exception(&third));
        assert!(!third.contains_exception(&first));
    }

    #[test]
    fn compare_by_from_date() {
        let first = ProjectCalendarException::new(Some(date(2024, 1, 1)), Some(date(2024, 1, 10)));
        let second = ProjectCalendarException::new(Some(date(2024, 1, 10)), Some(date(2024, 1, 20)));

        assert!(first < second);
        assert_eq!(first.cmp(&first), Ordering::Equal);
    }

    #[test]
    fn expansion_of_non_recurring_exception_returns_self() {
        let mut exception = ProjectCalendarException::new(Some(date(2024, 1, 1)), Some(date(2024, 1, 3)));
        exception.add(range(9, 12));

        let expanded = exception.get_expanded_exceptions();
        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded[0].get_from_date(), Some(date(2024, 1, 1)));
        assert_eq!(expanded[0].size(), 1);
    }

    #[test]
    fn expansion_of_daily_single_frequency_returns_self() {
        let mut recurring = RecurringData::new();
        recurring.set_recurrence_type(Some(RecurrenceType::Daily));
        recurring.set_frequency(Some(1));

        let mut exception = ProjectCalendarException::new_from_recurring(recurring);
        exception.add(range(9, 12));

        let expanded = exception.get_expanded_exceptions();
        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded[0].size(), 1);
    }

    #[test]
    fn display_matches_java_format() {
        let mut exception = ProjectCalendarException::new(Some(date(2024, 1, 1)), Some(date(2024, 1, 1)));
        assert_eq!(
            exception.to_string(),
            "[ProjectCalendarException working=false fromDate=2024-01-01 toDate=2024-01-01]"
        );

        exception.set_name(Some("Spring Festival".to_string()));
        exception.add(range(8, 12));
        assert_eq!(
            exception.to_string(),
            "[ProjectCalendarException name=Spring Festival working=true fromDate=2024-01-01 toDate=2024-01-01[LocalTimeRange start=08:00:00 end=12:00:00]]"
        );
    }

    #[test]
    fn equality_and_hash_follow_abstract_list_semantics() {
        let mut lhs = ProjectCalendarException::new(Some(date(2024, 1, 1)), Some(date(2024, 1, 1)));
        lhs.add(range(9, 12));

        let mut rhs = ProjectCalendarException::new(Some(date(2024, 2, 1)), Some(date(2024, 2, 1)));
        rhs.add(range(9, 12));

// Java: equals comes from AbstractList, comparing only list contents
        assert_eq!(lhs, rhs);

        rhs.clear();
        assert_ne!(lhs, rhs);
    }
}
