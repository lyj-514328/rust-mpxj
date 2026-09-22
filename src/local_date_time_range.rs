//! Corresponds to Java: `org/mpxj/LocalDateTimeRange.java`

use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};

use chrono::NaiveDateTime;

/// Represents a time range.
///
/// Corresponds to Java: `public final class LocalDateTimeRange implements Comparable<LocalDateTimeRange>`
///
/// Java's `m_start`/`m_end` may be `null`; Rust uniformly represents them with [`Option`].
#[derive(Debug, Clone, Copy)]
pub struct LocalDateTimeRange {
    /// Corresponds to Java field `m_start`
    start: Option<NaiveDateTime>,
    /// Corresponds to Java field `m_end`
    end: Option<NaiveDateTime>,
}

impl LocalDateTimeRange {
/// Constructor.
    ///
    /// Corresponds to Java: `LocalDateTimeRange(LocalDateTime startDate, LocalDateTime endDate)`
    pub fn new(start_date: Option<NaiveDateTime>, end_date: Option<NaiveDateTime>) -> Self {
        Self {
            start: start_date,
            end: end_date,
        }
    }

/// Gets the range start date-time.
    ///
    /// Corresponds to Java: `getStart()`
    pub fn get_start(&self) -> Option<NaiveDateTime> {
        self.start
    }

/// Gets the range end date-time.
    ///
    /// Corresponds to Java: `getEnd()`
    pub fn get_end(&self) -> Option<NaiveDateTime> {
        self.end
    }

/// Determines whether this range is entirely before the given range.
    ///
    /// Corresponds to Java: `isBefore(LocalDateTimeRange range)`
    ///
/// Java throws an NPE when a field is `null`; Rust degrades gracefully to returning `false`.
    pub fn is_before(&self, range: &LocalDateTimeRange) -> bool {
        matches!(self.compare_ranges(range), Some(result) if result < 0)
    }

/// Determines whether this range is entirely after the given range.
    ///
    /// Corresponds to Java: `isAfter(LocalDateTimeRange range)`
    ///
/// Java throws an NPE when a field is `null`; Rust degrades gracefully to returning `false`.
    pub fn is_after(&self, range: &LocalDateTimeRange) -> bool {
        matches!(self.compare_ranges(range), Some(result) if result > 0)
    }

/// Determines whether this range intersects the given range.
    ///
    /// Corresponds to Java: `intersectsWith(LocalDateTimeRange range)`
    ///
/// Java throws an NPE when a field is `null`; Rust degrades gracefully to returning `false`.
    pub fn intersects_with(&self, range: &LocalDateTimeRange) -> bool {
        matches!(self.compare_ranges(range), Some(0))
    }

/// Returns the intersection of this range and the given range, or `null` when there is no intersection ([`None`] on the Rust side).
    ///
    /// Corresponds to Java: `intersection(LocalDateTimeRange range)`
    pub fn intersection(&self, range: &LocalDateTimeRange) -> Option<LocalDateTimeRange> {
// no intersection, return null
        if !self.intersects_with(range) {
            return None;
        }

        match (self.start, self.end, range.start, range.end) {
            (Some(this_start), Some(this_end), Some(range_start), Some(range_end)) => {
// the given range falls entirely within this range; return the given range directly
                if range_start >= this_start && range_end <= this_end {
                    return Some(*range);
                }

// compute the intersection
                let start = if range_start < this_start {
                    this_start
                } else {
                    range_start
                };
                let end = if range_end > this_end {
                    this_end
                } else {
                    range_end
                };

// if this range falls entirely within the given range return itself, otherwise build a new intersection range
                if start == this_start && end == this_end {
                    Some(*self)
                } else {
                    Some(LocalDateTimeRange::new(Some(start), Some(end)))
                }
            }
// when intersects_with is false we already returned, so this is unreachable
            _ => None,
        }
    }

/// Compares the target date-time with the range: returns 0 if within the range, negative if before it, positive if after it.
    ///
/// Corresponds to Java: `compareTo(LocalDateTime date)` (internally calls
    /// `LocalDateTimeHelper.compare(LocalDateTime startDate, LocalDateTime endDate, LocalDateTime targetDate)`).
    ///
/// Java throws an NPE when the start/end date-time is `null`; Rust degrades gracefully to returning 0.
    pub fn compare_to_date(&self, date: NaiveDateTime) -> i32 {
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

    /// Corresponds to Java: `LocalDateTimeHelper.compare(LocalDateTime, LocalDateTime,
/// LocalDateTime, LocalDateTime)`, i.e. the comparison result of two ranges:
/// `-1` means this range ends before the given range starts, `1` means this range starts after the given range ends,
/// `0` means the two ranges intersect. Returns [`None`] when any field is `null` (Java throws an NPE).
    fn compare_ranges(&self, range: &LocalDateTimeRange) -> Option<i32> {
        match (self.start, self.end, range.start, range.end) {
            (Some(start1), Some(end1), Some(start2), Some(end2)) => {
                Some(compare_range_values(start1, end1, start2, end2))
            }
            _ => None,
        }
    }
}

/// Corresponds to Java: `LocalDateTimeHelper.compare(LocalDateTime startRange1, LocalDateTime endRange1,
/// LocalDateTime startRange2, LocalDateTime endRange2)`.
fn compare_range_values(
    start_range1: NaiveDateTime,
    end_range1: NaiveDateTime,
    start_range2: NaiveDateTime,
    end_range2: NaiveDateTime,
) -> i32 {
// range 1 ends before range 2 starts
    if end_range1 <= start_range2 {
        return -1;
    }

// range 1 starts after range 2 ends
    if start_range1 >= end_range2 {
        return 1;
    }

    0
}

/// Corresponds to Java: `LocalDateTimeHelper.compare(LocalDateTime, LocalDateTime)`.
///
/// Keeps Java's null semantics: `null` is treated as "greater", i.e. sorted after non-null values.
fn compare_optional_date_time(lhs: Option<NaiveDateTime>, rhs: Option<NaiveDateTime>) -> Ordering {
    match (lhs, rhs) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(lhs), Some(rhs)) => lhs.cmp(&rhs),
    }
}

/// Corresponds to Java: `LocalDateTime.toEpochSecond(ZoneOffset.UTC)`: `null` is treated as 0.
fn to_epoch_second(date_time: Option<NaiveDateTime>) -> i64 {
    date_time.map_or(0, |date_time| date_time.and_utc().timestamp())
}

impl Ord for LocalDateTimeRange {
/// Corresponds to Java: `compareTo(LocalDateTimeRange o)`: compare the start date-time first, then the end date-time.
    fn cmp(&self, other: &Self) -> Ordering {
        let result = compare_optional_date_time(self.start, other.start);
        if result == Ordering::Equal {
            compare_optional_date_time(self.end, other.end)
        } else {
            result
        }
    }
}

impl PartialOrd for LocalDateTimeRange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for LocalDateTimeRange {
/// Corresponds to Java: `equals(Object o)`: decided by whether the result of `compareTo` is 0.
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for LocalDateTimeRange {}

impl Hash for LocalDateTimeRange {
/// Corresponds to Java: `hashCode()`: XOR of the epoch seconds of the start/end date-times.
    fn hash<H: Hasher>(&self, state: &mut H) {
        let start = to_epoch_second(self.start);
        let end = to_epoch_second(self.end);
        let hash = ((start as i32) ^ ((start >> 32) as i32)) ^ ((end as i32) ^ ((end >> 32) as i32));
        hash.hash(state);
    }
}

impl Display for LocalDateTimeRange {
    /// Corresponds to Java: `toString()`:`[LocalDateTimeRange start=... end=...]`,
/// A `null` field is output as `null`. Java's `LocalDateTime.toString()` separates
/// the date and time with `T`; here we keep chrono's default format.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[LocalDateTimeRange start={} end={}]",
            self.start
                .map_or_else(|| "null".to_string(), |date_time| date_time.to_string()),
            self.end
                .map_or_else(|| "null".to_string(), |date_time| date_time.to_string())
        )
    }
}
