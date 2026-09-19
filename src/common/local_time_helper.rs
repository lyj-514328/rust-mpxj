//! 对应 Java: `org/mpxj/common/LocalTimeHelper.java`

use std::cmp::Ordering;

use chrono::{NaiveDateTime, NaiveTime, TimeDelta, Timelike};

/// 一天的毫秒数。
///
/// 对应 Java: `private static final long MS_PER_DAY`
const MS_PER_DAY: i64 = 24 * 60 * 60 * 1000;

/// 处理 `LocalTime`（Rust 侧为 [`NaiveTime`]）的工具方法。
///
/// 对应 Java: `public final class LocalTimeHelper`
///
/// 移植说明：
/// - Java 在 `date` 为 null 而 `time` 非 null 时会抛 `NullPointerException`（见 `setTime` /
///   `setEndTime`），Rust 侧统一返回 `None`。
/// - `Date.plusDays(1)` 在 Rust 侧用 `checked_add_signed`，不会 panic。
///
/// 未移植的方法：无。
pub struct LocalTimeHelper;

impl LocalTimeHelper {
    /// 对应 Java: `public static LocalTime getLocalTime(LocalDateTime date)`
    ///
    /// 对应 `date.toLocalTime()`，null 返回 `None`。
    pub fn get_local_time(date: Option<NaiveDateTime>) -> Option<NaiveTime> {
        date.map(|date| date.time())
    }

    /// 对应 Java: `public static int compare(LocalTime d1, LocalTime d2)`
    ///
    /// 处理 null：两边都为 null 返回 0，仅左边为 null 返回 1，仅右边为 null 返回 -1。
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

    /// 对应 Java: `public static LocalDateTime setTime(LocalDateTime date, LocalTime time)`
    ///
    /// 用 `time` 替换 `date` 的时间部分；`time` 为 null 时原样返回 `date`。
    pub fn set_time(date: Option<NaiveDateTime>, time: Option<NaiveTime>) -> Option<NaiveDateTime> {
        match time {
            None => date,
            Some(time) => date.map(|date| NaiveDateTime::new(date.date(), time)),
        }
    }

    /// 对应 Java: `public static LocalDateTime setEndTime(LocalDateTime date, LocalTime time)`
    ///
    /// 同 [`LocalTimeHelper::set_time`]，但把午夜视为当天结束（即次日 00:00）。
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

    /// 对应 Java: `public static long getMillisecondsInRange(LocalTime rangeStart, LocalTime rangeEnd)`
    ///
    /// 计算两个时间点之间的毫秒数，处理 null（返回 0）；`rangeEnd` 为午夜时按跨天处理。
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
