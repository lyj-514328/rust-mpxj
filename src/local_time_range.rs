//! Corresponds to Java: `org/mpxj/LocalTimeRange.java`

use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};

use chrono::{NaiveTime, Timelike};

/// Milliseconds per day, corresponding to Java `LocalTimeHelper.MS_PER_DAY`.
const MS_PER_DAY: i64 = 24 * 60 * 60 * 1000;

/// Represents a time range.
///
/// Corresponds to Java: `public final class LocalTimeRange implements Comparable<LocalTimeRange>`
///
/// On the Java side `m_start`/`m_end` may be `null` (see `EMPTY_RANGE`); the Rust side
/// uniformly represents them with [`Option`].
#[derive(Debug, Clone, Copy)]
pub struct LocalTimeRange {
    /// Corresponds to Java field `m_start`
    start: Option<NaiveTime>,
    /// Corresponds to Java field `m_end`
    end: Option<NaiveTime>,
    /// Corresponds to Java field `m_durationAsMilliseconds`; computed at construction time by
    /// `LocalTimeHelper.getMillisecondsInRange`
    duration_as_milliseconds: i64,
}

impl LocalTimeRange {
    /// Empty range.
    ///
    /// Corresponds to Java: `public static final LocalTimeRange EMPTY_RANGE = new LocalTimeRange(null, null)`
    pub const EMPTY_RANGE: LocalTimeRange = LocalTimeRange {
        start: None,
        end: None,
        duration_as_milliseconds: 0,
    };

    /// Constructor that also computes the range duration.
    ///
    /// Corresponds to Java: `LocalTimeRange(LocalTime start, LocalTime end)`
    pub fn new(start: Option<NaiveTime>, end: Option<NaiveTime>) -> Self {
        Self {
            start,
            end,
            duration_as_milliseconds: get_milliseconds_in_range(start, end),
        }
    }

    /// Return the range start time.
    ///
    /// Corresponds to Java: `getStart()`
    pub fn get_start(&self) -> Option<NaiveTime> {
        self.start
    }

    /// Return the range end time.
    ///
    /// Corresponds to Java: `getEnd()`
    pub fn get_end(&self) -> Option<NaiveTime> {
        self.end
    }

    /// Return the range duration (in milliseconds).
    ///
    /// Corresponds to Java: `getDurationAsMilliseconds()`
    pub fn get_duration_as_milliseconds(&self) -> i64 {
        self.duration_as_milliseconds
    }
}

/// Compute the number of milliseconds between two times, preserving Java's null handling.
///
/// Corresponds to Java: `LocalTimeHelper.getMillisecondsInRange(LocalTime rangeStart, LocalTime rangeEnd)`
///
/// Java uses `rangeEnd == LocalTime.MIDNIGHT` to detect crossing midnight; the Rust side handles it by value equivalence.
fn get_milliseconds_in_range(range_start: Option<NaiveTime>, range_end: Option<NaiveTime>) -> i64 {
    match (range_start, range_end) {
        (Some(start), Some(end)) => {
            if end == NaiveTime::MIN {
                MS_PER_DAY - start.num_seconds_from_midnight() as i64 * 1000
            } else {
                (end.num_seconds_from_midnight() as i64
                    - start.num_seconds_from_midnight() as i64)
                    * 1000
            }
        }
        _ => 0,
    }
}

/// Corresponds to Java: `LocalTimeHelper.compare(LocalTime, LocalTime)`.
///
/// Preserves Java's null semantics: `null` is treated as "greater", i.e. sorted after non-null times.
fn compare_optional_time(lhs: Option<NaiveTime>, rhs: Option<NaiveTime>) -> Ordering {
    match (lhs, rhs) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(lhs), Some(rhs)) => lhs.cmp(&rhs),
    }
}

/// Corresponds to Java: `LocalTime.toNanoOfDay()`: `null` is treated as 0.
fn to_nano_of_day(time: Option<NaiveTime>) -> i64 {
    time.map_or(0, |time| {
        time.num_seconds_from_midnight() as i64 * 1_000_000_000 + time.nanosecond() as i64
    })
}

impl Ord for LocalTimeRange {
    /// Corresponds to Java: `compareTo(LocalTimeRange o)`: first compares the start time, and when
    /// the start times are equal compares the duration with `Long.compare`.
    fn cmp(&self, other: &Self) -> Ordering {
        let result = compare_optional_time(self.start, other.start);
        if result == Ordering::Equal {
            self.duration_as_milliseconds
                .cmp(&other.duration_as_milliseconds)
        } else {
            result
        }
    }
}

impl PartialOrd for LocalTimeRange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for LocalTimeRange {
    /// Corresponds to Java: `equals(Object o)`: determined by whether the result of `compareTo` is 0.
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for LocalTimeRange {}

impl Hash for LocalTimeRange {
    /// Corresponds to Java: `hashCode()`: XORs the nanoOfDay of the start/end times.
    fn hash<H: Hasher>(&self, state: &mut H) {
        let start = to_nano_of_day(self.start);
        let end = to_nano_of_day(self.end);
        let hash = ((start as i32) ^ ((start >> 32) as i32)) ^ ((end as i32) ^ ((end >> 32) as i32));
        hash.hash(state);
    }
}

impl Display for LocalTimeRange {
    /// Corresponds to Java: `toString()`: `[LocalTimeRange start=... end=...]`, with `null` fields
    /// output as `null` following Java's convention.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[LocalTimeRange start={} end={}]",
            self.start
                .map_or_else(|| "null".to_string(), |time| time.to_string()),
            self.end
                .map_or_else(|| "null".to_string(), |time| time.to_string())
        )
    }
}
