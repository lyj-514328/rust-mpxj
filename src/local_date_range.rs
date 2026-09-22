//! Corresponds to Java: `org/mpxj/LocalDateRange.java`

use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};

use chrono::{Datelike, NaiveDate};

/// The value of 1970-01-01 under `NaiveDate::num_days_from_ce()` (with 0001-01-01 counted as
/// day 1), used to convert chrono's CE day number into the day number of Java
/// `LocalDate.toEpochDay()`.
const UNIX_EPOCH_DAY_FROM_CE: i64 = 719_163;

/// Represents a date range.
///
/// Corresponds to Java: `public final class LocalDateRange implements Comparable<LocalDateRange>`
///
/// On the Java side `m_start`/`m_end` may be `null`; the Rust side uniformly represents them with [`Option`].
#[derive(Debug, Clone, Copy)]
pub struct LocalDateRange {
    /// Corresponds to Java field `m_start`
    start: Option<NaiveDate>,
    /// Corresponds to Java field `m_end`
    end: Option<NaiveDate>,
}

impl LocalDateRange {
    /// Constructor.
    ///
    /// Corresponds to Java: `LocalDateRange(LocalDate startDate, LocalDate endDate)`
    pub fn new(start_date: Option<NaiveDate>, end_date: Option<NaiveDate>) -> Self {
        Self {
            start: start_date,
            end: end_date,
        }
    }

    /// Return the range start date.
    ///
    /// Corresponds to Java: `getStart()`
    pub fn get_start(&self) -> Option<NaiveDate> {
        self.start
    }

    /// Return the range end date.
    ///
    /// Corresponds to Java: `getEnd()`
    pub fn get_end(&self) -> Option<NaiveDate> {
        self.end
    }

    /// Compare a target date with the range: returns 0 when the target date is within the range, a negative number when earlier than the range, and a positive number when later.
    ///
    /// Corresponds to Java: `compareTo(LocalDate date)` (internally calls
    /// `LocalDateHelper.compare(LocalDate startDate, LocalDate endDate, LocalDate targetDate)`).
    ///
    /// Java throws an NPE when the start/end date is `null`; the Rust side degrades to returning 0.
    pub fn compare_to_date(&self, date: NaiveDate) -> i32 {
        match (self.start, self.end) {
            (Some(start), Some(end)) => {
                if date < start {
                    -1
                } else if date > end {
                    1
                } else {
                    0
                }
            }
            _ => 0,
        }
    }
}

/// Corresponds to Java: `LocalDateHelper.compare(LocalDate, LocalDate)`.
///
/// Preserves Java's null semantics: `null` is treated as "greater", i.e. sorted after non-null dates.
fn compare_optional_date(lhs: Option<NaiveDate>, rhs: Option<NaiveDate>) -> Ordering {
    match (lhs, rhs) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(lhs), Some(rhs)) => lhs.cmp(&rhs),
    }
}

impl Ord for LocalDateRange {
    /// Corresponds to Java: `compareTo(LocalDateRange o)`: first compares the start date, then the end date.
    fn cmp(&self, other: &Self) -> Ordering {
        let result = compare_optional_date(self.start, other.start);
        if result == Ordering::Equal {
            compare_optional_date(self.end, other.end)
        } else {
            result
        }
    }
}

impl PartialOrd for LocalDateRange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for LocalDateRange {
    /// Corresponds to Java: `equals(Object o)`: determined by whether the result of `compareTo` is 0.
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for LocalDateRange {}

impl Hash for LocalDateRange {
    /// Corresponds to Java: `hashCode()`: XORs the epoch day of the start/end dates.
    fn hash<H: Hasher>(&self, state: &mut H) {
        let start = to_epoch_day(self.start);
        let end = to_epoch_day(self.end);
        let hash = ((start as i32) ^ ((start >> 32) as i32)) ^ ((end as i32) ^ ((end >> 32) as i32));
        hash.hash(state);
    }
}

impl Display for LocalDateRange {
    /// Corresponds to Java: `toString()`: `[LocalDateRange start=... end=...]`, with `null` fields
    /// output as `null` following Java's convention.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[LocalDateRange start={} end={}]",
            self.start
                .map_or_else(|| "null".to_string(), |date| date.to_string()),
            self.end
                .map_or_else(|| "null".to_string(), |date| date.to_string())
        )
    }
}

/// Corresponds to Java: `LocalDate.toEpochDay()`: `null` is treated as 0.
fn to_epoch_day(date: Option<NaiveDate>) -> i64 {
    date.map_or(0, |date| {
        date.num_days_from_ce() as i64 - UNIX_EPOCH_DAY_FROM_CE
    })
}
