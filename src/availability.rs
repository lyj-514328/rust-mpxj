//! Corresponds to Java: `org/mpxj/Availability.java`

use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};

use chrono::NaiveDateTime;

use crate::local_date_time_range::LocalDateTimeRange;

/// A row in a resource availability table.
///
/// Corresponds to Java: `public final class Availability implements Comparable<Availability>`
///
/// Unported Java methods: none.
#[derive(Debug, Clone, Copy)]
pub struct Availability {
/// The availability range.
    ///
    /// Corresponds to Java: `private final LocalDateTimeRange m_range;`
    m_range: LocalDateTimeRange,
/// The number of available units for this range (Java uses `Number` and allows `null`).
    ///
    /// Corresponds to Java: `private final Number m_units;`
    m_units: Option<f64>,
}

impl Availability {
/// Constructor.
    ///
    /// Corresponds to Java: `public Availability(LocalDateTime startDate, LocalDateTime endDate, Number units)`
    pub fn new(
        start_date: Option<NaiveDateTime>,
        end_date: Option<NaiveDateTime>,
        units: Option<f64>,
    ) -> Self {
        Self {
            m_range: LocalDateTimeRange::new(start_date, end_date),
            m_units: units,
        }
    }

/// Get the date range of the availability range.
    ///
    /// Corresponds to Java: `public LocalDateTimeRange getRange()`
    pub fn get_range(&self) -> LocalDateTimeRange {
        self.m_range
    }

/// Get the number of available units for this range.
    ///
    /// Corresponds to Java: `public Number getUnits()`
    pub fn get_units(&self) -> Option<f64> {
        self.m_units
    }
}

impl Ord for Availability {
    /// Corresponds to Java: `@Override public int compareTo(Availability o)`
    ///
/// Compare the date ranges of two availability ranges directly.
    fn cmp(&self, other: &Self) -> Ordering {
        self.m_range.cmp(&other.m_range)
    }
}

impl PartialOrd for Availability {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Availability {
/// The Java side does not override `equals`/`hashCode` (it keeps object identity comparison); the Rust side, because [`Ord`]
/// requires the `Eq` supertrait, performs value comparison here based on "both the range and the number of available units are equal".
    ///
/// Note: this implementation differs semantically from `compareTo` (which compares only the range) when "the ranges are the same but the unit counts differ",
/// which is consistent with Java's `Comparable`/`equals` contract (this class does not implement `equals`).
    fn eq(&self, other: &Self) -> bool {
        self.m_range == other.m_range && self.m_units == other.m_units
    }
}

/// Note: `Availability` contains an `f64` field and cannot automatically derive `Eq`; however, the Java side implements
/// `Comparable`, and Rust's `Ord` requires `Eq` as a supertrait, so `Eq` is implemented manually here.
impl Eq for Availability {}

impl Hash for Availability {
/// The Java side does not override `hashCode`; the Rust side, to make it convenient to use `Availability` as a hash key,
/// computes it from the bit patterns of the range and the number of available units.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.m_range.hash(state);
        self.m_units.map(f64::to_bits).hash(state);
    }
}

impl Display for Availability {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Note: Java's `Number.toString()` keeps `.0` (e.g. `1.0`), whereas Rust's `f64`
/// `Display` outputs `1`; the two differ slightly in this method.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[Availability range={} units={}]",
            self.m_range,
            self.m_units
                .map_or_else(|| "null".to_string(), |units| units.to_string())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveTime};

/// Construct a date-time for testing.
    fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(year, month, day).expect("valid date"),
            NaiveTime::from_hms_opt(hour, minute, 0).expect("valid time"),
        )
    }

    #[test]
    fn accessors_return_constructor_values() {
        let start = date_time(2024, 1, 1, 8, 0);
        let end = date_time(2024, 1, 31, 17, 0);
        let value = Availability::new(Some(start), Some(end), Some(0.5));

        assert_eq!(
            value.get_range(),
            LocalDateTimeRange::new(Some(start), Some(end))
        );
        assert_eq!(value.get_units(), Some(0.5));
    }

    #[test]
    fn null_arguments_are_preserved() {
        let value = Availability::new(None, None, None);

        assert_eq!(value.get_range().get_start(), None);
        assert_eq!(value.get_range().get_end(), None);
        assert_eq!(value.get_units(), None);
    }

    #[test]
    fn ordering_follows_range() {
        let earlier = Availability::new(
            Some(date_time(2024, 1, 1, 8, 0)),
            Some(date_time(2024, 1, 31, 17, 0)),
            Some(0.5),
        );
        let later = Availability::new(
            Some(date_time(2024, 2, 1, 8, 0)),
            Some(date_time(2024, 2, 28, 17, 0)),
            Some(0.5),
        );

        assert!(earlier < later);
        assert_eq!(earlier.cmp(&earlier.clone()), Ordering::Equal);
    }

    #[test]
    fn equality_compares_range_and_units() {
        let start = Some(date_time(2024, 1, 1, 8, 0));
        let end = Some(date_time(2024, 1, 31, 17, 0));
        let lhs = Availability::new(start, end, Some(0.5));
        let same = Availability::new(start, end, Some(0.5));
        let other_units = Availability::new(start, end, Some(0.25));

        assert_eq!(lhs, same);
        assert_ne!(lhs, other_units);
    }

    #[test]
    fn display_matches_java_format() {
        let value = Availability::new(
            Some(date_time(2024, 1, 1, 8, 0)),
            Some(date_time(2024, 1, 31, 17, 0)),
            Some(0.5),
        );

        assert_eq!(
            value.to_string(),
            "[Availability range=[LocalDateTimeRange start=2024-01-01 08:00:00 end=2024-01-31 17:00:00] units=0.5]"
        );
    }
}
