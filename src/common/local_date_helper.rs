//! 对应 Java: `org/mpxj/common/LocalDateHelper.java`

use std::cmp::Ordering;

use chrono::{Local, Months, NaiveDate, NaiveDateTime, NaiveTime};

/// 处理 `LocalDate`（Rust 侧为 [`NaiveDate`]）的工具方法。
///
/// 对应 Java: `public final class LocalDateHelper`
///
/// 移植说明：
/// - `TWO_DIGIT_YEAR_BASE_DATE` 在 Java 中是 `static final` 常量（`LocalDate.now().minusYears(80)`），
///   chrono 的日期构造不是 `const fn`，因此 Rust 侧改为函数
///   [`LocalDateHelper::two_digit_year_base_date`]，取值仍按调用时的当前日期计算。
/// - Java 的 `compareTo` 只保证正负号，Rust 侧的统一返回 -1 / 0 / 1。
///
/// 未移植的方法：无。
pub struct LocalDateHelper;

impl LocalDateHelper {
    /// 对应 Java: `public static LocalDateTime getLocalDateTime(LocalDate date)`
    ///
    /// 对应 `date.atStartOfDay()`，null 返回 `None`。
    pub fn get_local_date_time(date: Option<NaiveDate>) -> Option<NaiveDateTime> {
        date.map(|date| NaiveDateTime::new(date, NaiveTime::MIN))
    }

    /// 对应 Java: `public static LocalDate getLocalDate(LocalDateTime date)`
    ///
    /// 对应 `date.toLocalDate()`，null 返回 `None`。
    pub fn get_local_date(date: Option<NaiveDateTime>) -> Option<NaiveDate> {
        date.map(|date| date.date())
    }

    /// 对应 Java: `public static int compare(LocalDate d1, LocalDate d2)`
    ///
    /// 处理 null：两边都为 null 返回 0，仅左边为 null 返回 1，仅右边为 null 返回 -1。
    pub fn compare(d1: Option<NaiveDate>, d2: Option<NaiveDate>) -> i32 {
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

    /// 对应 Java: `public static int compare(LocalDate startDate, LocalDate endDate, LocalDate targetDate)`
    ///
    /// 判断目标日期是否落在给定区间内：早于区间返回 -1，晚于区间返回 1，区间内返回 0。
    pub fn compare_with_range(
        start_date: NaiveDate,
        end_date: NaiveDate,
        target_date: NaiveDate,
    ) -> i32 {
        if target_date < start_date {
            -1
        } else if target_date > end_date {
            1
        } else {
            0
        }
    }

    /// 对应 Java: `public static LocalDateTime getDayEndDate(LocalDate date)`
    ///
    /// Java 的注释写的是 23:59:59.999，实际实现用的是 `LocalTime.of(23, 59, 59)`，此处按实现翻译。
    pub fn get_day_end_date(date: Option<NaiveDate>) -> Option<NaiveDateTime> {
        let date = date?;
        NaiveTime::from_hms_opt(23, 59, 59).map(|time| NaiveDateTime::new(date, time))
    }

    /// 对应 Java: `public static final LocalDate TWO_DIGIT_YEAR_BASE_DATE`
    ///
    /// 等价于 `LocalDate.now().minusYears(80)`（Java 使用系统默认时区，Rust 侧用 [`Local`]）。
    pub fn two_digit_year_base_date() -> NaiveDate {
        match Local::now()
            .date_naive()
            .checked_sub_months(Months::new(80 * 12))
        {
            Some(date) => date,
            None => NaiveDate::MIN,
        }
    }
}
