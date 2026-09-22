//! Corresponds to Java: `org/mpxj/common/LocalTimeHelper.java`

use std::cmp::Ordering;

use chrono::{NaiveDateTime, NaiveTime, TimeDelta, Timelike};

/// Number of milliseconds in a day.
///
/// Corresponds to Java: `private static final long MS_PER_DAY`
const MS_PER_DAY: i64 = 24 * 60 * 60 * 1000;

/// Utility methods for handling `LocalTime` (represented by [`NaiveTime`] on the Rust side).
///
/// Corresponds to Java: `public final class LocalTimeHelper`
///
/// Porting notes:
/// - Java throws `NullPointerException` when `date` is null and `time` is non-null (see
///   `setTime` / `setEndTime`); the Rust side uniformly returns `None`.
/// - `Date.plusDays(1)` uses `checked_add_signed` on the Rust side and never panics.
///
/// Skipped Java methods: none.
pub struct LocalTimeHelper;

impl LocalTimeHelper {
    /// Corresponds to Java: `public static LocalTime getLocalTime(LocalDateTime date)`
    ///
    /// Corresponds to `date.toLocalTime()`; null returns `None`.
    pub fn get_local_time(date: Option<NaiveDateTime>) -> Option<NaiveTime> {
        date.map(|date| date.time())
    }

    /// Corresponds to Java: `public static int compare(LocalTime d1, LocalTime d2)`
    ///
    /// Handles null: both null returns 0, only the left null returns 1, only the right null returns -1.
    pub fn compare(d1: Option<NaiveTime>, d2: Option<NaiveTime>) -> i32 {
        match (d1, d2) {
            (None, None) => 0,
            (None, Some(_)) => 1,
            (Some(_), None) => -1,
            (Some(d1), Some(d2)) => match d1.cmp(&d2) {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            },
        }
    }

    /// Corresponds to Java: `public static LocalDateTime setTime(LocalDateTime date, LocalTime time)`
    ///
    /// Replaces the time part of `date` with `time`; when `time` is null, returns `date` unchanged.
    pub fn set_time(date: Option<NaiveDateTime>, time: Option<NaiveTime>) -> Option<NaiveDateTime> {
        match time {
            None => date,
            Some(time) => date.map(|date| NaiveDateTime::new(date.date(), time)),
        }
    }

    /// Corresponds to Java: `public static LocalDateTime setEndTime(LocalDateTime date, LocalTime time)`
    ///
    /// Same as [`LocalTimeHelper::set_time`], but treats midnight as the end of the day (that is, 00:00 of the next day).
    pub fn set_end_time(
        date: Option<NaiveDateTime>,
        time: Option<NaiveTime>,
    ) -> Option<NaiveDateTime> {
        let time = match time {
            None => return date,
            Some(time) => time,
        };

        let date = match date {
            None => return None,
            Some(date) => date,
        };

        let result = NaiveDateTime::new(date.date(), time);
        if time == NaiveTime::MIN {
            result.checked_add_signed(TimeDelta::days(1))
        } else {
            Some(result)
        }
    }

    /// Corresponds to Java: `public static long getMillisecondsInRange(LocalTime rangeStart, LocalTime rangeEnd)`
    ///
    /// Computes the number of milliseconds between two time points, handling null (returns 0); when `rangeEnd` is midnight it is treated as crossing into the next day.
    pub fn get_milliseconds_in_range(
        range_start: Option<NaiveTime>,
        range_end: Option<NaiveTime>,
    ) -> i64 {
        match (range_start, range_end) {
            (Some(range_start), Some(range_end)) => {
                let start = i64::from(range_start.num_seconds_from_midnight());
                let end = i64::from(range_end.num_seconds_from_midnight());
                if range_end == NaiveTime::MIN {
                    MS_PER_DAY - (start * 1000)
                } else {
                    (end - start) * 1000
                }
            }
            _ => 0,
        }
    }
}
