//! 对应 Java: `org/mpxj/common/LocalDateTimeHelper.java`

use std::cmp::Ordering;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

use crate::duration::Duration;
use crate::project_calendar::ProjectCalendar;
use crate::time_unit::TimeUnit;

/// 处理 `LocalDateTime`（Rust 侧为 [`NaiveDateTime`]）的工具方法。
///
/// 对应 Java: `public final class LocalDateTimeHelper`
///
/// 移植说明：
/// - `START_DATE_NA` / `END_DATE_NA` 在 Java 中是 `static final` 常量；chrono 的日期时间构造不是
///   `const fn`，因此 Rust 侧改为 [`LocalDateTimeHelper::start_date_na`] /
///   [`LocalDateTimeHelper::end_date_na`] 两个函数。
/// - `parseBest(DateTimeFormatter, String)` 的 `DateTimeFormatter` 在 chrono 中没有等价对象，
///   改用 chrono 的格式字符串 `&str`；解析失败时 Java 抛 `DateTimeParseException`，Rust 返回 `None`。
/// - `getVariance` 使用的 `org.mpxj.Duration`、`ProjectCalendar`、`TimeUnit` 在 Rust 侧对应
///   [`crate::duration::Duration`]、[`crate::project_calendar::ProjectCalendar`] 与
///   [`crate::time_unit::TimeUnit`]。
///
/// 未移植的方法：无。
pub struct LocalDateTimeHelper;

impl LocalDateTimeHelper {
    /// 对应 Java: `public static LocalDateTime getDayStartDate(LocalDateTime date)`
    ///
    /// 返回当天 00:00:00，null 返回 `None`。
    pub fn get_day_start_date(date: Option<NaiveDateTime>) -> Option<NaiveDateTime> {
        date.map(|date| NaiveDateTime::new(date.date(), NaiveTime::MIN))
    }

    /// 对应 Java: `public static int compare(LocalDateTime startDate, LocalDateTime endDate, LocalDateTime targetDate)`
    ///
    /// 目标日期早于区间返回 -1，晚于区间返回 1，落在区间内返回 0。
    pub fn compare_with_range(
        start_date: NaiveDateTime,
        end_date: NaiveDateTime,
        target_date: NaiveDateTime,
    ) -> i32 {
        if target_date < start_date {
            -1
        } else if target_date > end_date {
            1
        } else {
            0
        }
    }

    /// 对应 Java: `public static int compare(LocalDateTime startRange1, LocalDateTime endRange1, LocalDateTime startRange2, LocalDateTime endRange2)`
    ///
    /// 区间 1 结束于区间 2 开始之前返回 -1，区间 1 开始于区间 2 结束之后返回 1，相交返回 0。
    pub fn compare_ranges(
        start_range1: NaiveDateTime,
        end_range1: NaiveDateTime,
        start_range2: NaiveDateTime,
        end_range2: NaiveDateTime,
    ) -> i32 {
        // 区间 1 结束于区间 2 开始之前
        if end_range1 <= start_range2 {
            return -1;
        }

        // 区间 1 开始于区间 2 结束之后
        if start_range1 >= end_range2 {
            return 1;
        }

        0
    }

    /// 对应 Java: `public static int compare(LocalDateTime d1, LocalDateTime d2)`
    ///
    /// 处理 null：两边都为 null 返回 0，仅左边为 null 返回 1，仅右边为 null 返回 -1。
    pub fn compare(d1: Option<NaiveDateTime>, d2: Option<NaiveDateTime>) -> i32 {
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

    /// 对应 Java: `public static LocalDateTime min(LocalDateTime d1, LocalDateTime d2)`
    ///
    /// 非 null 值始终早于 null 值。
    pub fn min(d1: Option<NaiveDateTime>, d2: Option<NaiveDateTime>) -> Option<NaiveDateTime> {
        match (d1, d2) {
            (None, d2) => d2,
            (d1, None) => d1,
            (Some(d1), Some(d2)) => Some(if d1 < d2 { d1 } else { d2 }),
        }
    }

    /// 对应 Java: `public static LocalDateTime max(LocalDateTime d1, LocalDateTime d2)`
    ///
    /// 非 null 值始终晚于 null 值。
    pub fn max(d1: Option<NaiveDateTime>, d2: Option<NaiveDateTime>) -> Option<NaiveDateTime> {
        match (d1, d2) {
            (None, d2) => d2,
            (d1, None) => d1,
            (Some(d1), Some(d2)) => Some(if d1 > d2 { d1 } else { d2 }),
        }
    }

    /// 对应 Java: `public static LocalDateTime parseBest(DateTimeFormatter format, String value)`
    ///
    /// Java 用 `format.parseBest` 先尝试 `LocalDateTime`，失败再尝试 `LocalDate` 并转为当天 00:00；
    /// Rust 侧用 `format` 作为 chrono 格式字符串复现同样的两级回退。
    pub fn parse_best(format: &str, value: &str) -> Option<NaiveDateTime> {
        if let Ok(date_time) = NaiveDateTime::parse_from_str(value, format) {
            return Some(date_time);
        }

        match NaiveDate::parse_from_str(value, format) {
            Ok(date) => Some(NaiveDateTime::new(date, NaiveTime::MIN)),
            Err(_) => None,
        }
    }

    /// 对应 Java: `public static Duration getVariance(ProjectCalendar calendar, LocalDateTime date1, LocalDateTime date2, TimeUnit format)`
    ///
    /// 在给定日历下计算两个日期之间的工作时间差；任一参数为 null 时返回 0 工期。
    pub fn get_variance(
        calendar: Option<&ProjectCalendar>,
        date1: Option<NaiveDateTime>,
        date2: Option<NaiveDateTime>,
        format: TimeUnit,
    ) -> Duration {
        match (calendar, date1, date2) {
            (Some(calendar), Some(date1), Some(date2)) => calendar.get_work(date1, date2, format),
            _ => Duration::get_instance(0.0, format),
        }
    }

    /// 对应 Java: `public static final LocalDateTime START_DATE_NA`
    ///
    /// 表示 NA 的区间起始日期：1984-01-01 00:00:00。
    pub fn start_date_na() -> NaiveDateTime {
        match (
            NaiveDate::from_ymd_opt(1984, 1, 1),
            NaiveTime::from_hms_opt(0, 0, 0),
        ) {
            (Some(date), Some(time)) => NaiveDateTime::new(date, time),
            _ => NaiveDateTime::default(),
        }
    }

    /// 对应 Java: `public static final LocalDateTime END_DATE_NA`
    ///
    /// 表示 NA 的区间结束日期：2049-12-31 23:59:00（沿用旧版 MS Project 识别的取值，详见 Java 注释）。
    pub fn end_date_na() -> NaiveDateTime {
        match (
            NaiveDate::from_ymd_opt(2049, 12, 31),
            NaiveTime::from_hms_opt(23, 59, 0),
        ) {
            (Some(date), Some(time)) => NaiveDateTime::new(date, time),
            _ => NaiveDateTime::default(),
        }
    }
}
