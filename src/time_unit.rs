//! 对应 Java: `org/mpxj/TimeUnit.java`

use crate::mpxj_enum::MpxjEnum;
use std::fmt::{self, Display, Formatter};

/// 时间单位的枚举。
///
/// 对应 Java: `public enum TimeUnit implements MpxjEnum`
///
/// Java 侧每个常量携带 `(int value, String name, boolean elapsed)` 三个字段，
/// 并用 `EnumHelper.createTypeArray` 反射生成 int → 枚举的数组。Rust 侧：
/// * `value` 用显式判别值表达；
/// * `name` / `elapsed` 用方法（[`TimeUnit::get_name`] / [`TimeUnit::is_elapsed`]）返回；
/// * 不再使用反射，映射改由 [`TimeUnit::get_instance`] 的 `match` 完成。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TimeUnit {
    /// 对应 Java: `MINUTES(0, "m", false)`
    Minutes = 0,
    /// 对应 Java: `HOURS(1, "h", false)`
    Hours = 1,
    /// 对应 Java: `DAYS(2, "d", false)`
    Days = 2,
    /// 对应 Java: `WEEKS(3, "w", false)`
    Weeks = 3,
    /// 对应 Java: `MONTHS(4, "mo", false)`
    Months = 4,
    /// 对应 Java: `PERCENT(5, "%", false)`
    Percent = 5,
    /// 对应 Java: `YEARS(6, "y", false)`
    Years = 6,
    /// 对应 Java: `ELAPSED_MINUTES(7, "em", true)`
    ElapsedMinutes = 7,
    /// 对应 Java: `ELAPSED_HOURS(8, "eh", true)`
    ElapsedHours = 8,
    /// 对应 Java: `ELAPSED_DAYS(9, "ed", true)`
    ElapsedDays = 9,
    /// 对应 Java: `ELAPSED_WEEKS(10, "ew", true)`
    ElapsedWeeks = 10,
    /// 对应 Java: `ELAPSED_MONTHS(11, "emo", true)`
    ElapsedMonths = 11,
    /// 对应 Java: `ELAPSED_YEARS(12, "ey", true)`
    ElapsedYears = 12,
    /// 对应 Java: `ELAPSED_PERCENT(13, "e%", true)`
    ElapsedPercent = 13,
}

impl TimeUnit {
    /// 全部取值，按 int 值升序排列。
    ///
    /// 对应 Java: `private static final TimeUnit[] TYPE_VALUES`（不含反射部分）。
    pub const ALL: [TimeUnit; 14] = [
        TimeUnit::Minutes,
        TimeUnit::Hours,
        TimeUnit::Days,
        TimeUnit::Weeks,
        TimeUnit::Months,
        TimeUnit::Percent,
        TimeUnit::Years,
        TimeUnit::ElapsedMinutes,
        TimeUnit::ElapsedHours,
        TimeUnit::ElapsedDays,
        TimeUnit::ElapsedWeeks,
        TimeUnit::ElapsedMonths,
        TimeUnit::ElapsedYears,
        TimeUnit::ElapsedPercent,
    ];

    /// 根据 int 值取枚举实例，越界时回落到 `DAYS`。
    ///
    /// 对应 Java: `public static TimeUnit getInstance(int type)`
    pub fn get_instance(value: i32) -> TimeUnit {
        match value {
            0 => TimeUnit::Minutes,
            1 => TimeUnit::Hours,
            2 => TimeUnit::Days,
            3 => TimeUnit::Weeks,
            4 => TimeUnit::Months,
            5 => TimeUnit::Percent,
            6 => TimeUnit::Years,
            7 => TimeUnit::ElapsedMinutes,
            8 => TimeUnit::ElapsedHours,
            9 => TimeUnit::ElapsedDays,
            10 => TimeUnit::ElapsedWeeks,
            11 => TimeUnit::ElapsedMonths,
            12 => TimeUnit::ElapsedYears,
            13 => TimeUnit::ElapsedPercent,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = DAYS.getValue(); }`
            _ => TimeUnit::Days,
        }
    }

    /// 对应 Java: `public static TimeUnit getInstance(Number type)`
    ///
    /// Java 侧 `null` 走 `getInstance(-1)`，最终回落到 `DAYS`。
    pub fn get_instance_optional(value: Option<i32>) -> TimeUnit {
        TimeUnit::get_instance(value.unwrap_or(-1))
    }

    /// 返回该枚举对应的名字（如 `"m"` / `"emo"`）。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            TimeUnit::Minutes => "m",
            TimeUnit::Hours => "h",
            TimeUnit::Days => "d",
            TimeUnit::Weeks => "w",
            TimeUnit::Months => "mo",
            TimeUnit::Percent => "%",
            TimeUnit::Years => "y",
            TimeUnit::ElapsedMinutes => "em",
            TimeUnit::ElapsedHours => "eh",
            TimeUnit::ElapsedDays => "ed",
            TimeUnit::ElapsedWeeks => "ew",
            TimeUnit::ElapsedMonths => "emo",
            TimeUnit::ElapsedYears => "ey",
            TimeUnit::ElapsedPercent => "e%",
        }
    }

    /// 判断该时间单位是否表示「已用时间（elapsed）」。
    ///
    /// 对应 Java: `public boolean isElapsed()`
    pub fn is_elapsed(&self) -> bool {
        matches!(
            self,
            TimeUnit::ElapsedMinutes
                | TimeUnit::ElapsedHours
                | TimeUnit::ElapsedDays
                | TimeUnit::ElapsedWeeks
                | TimeUnit::ElapsedMonths
                | TimeUnit::ElapsedYears
                | TimeUnit::ElapsedPercent
        )
    }
}

impl MpxjEnum for TimeUnit {
    /// 返回该枚举关联的 int 值。
    ///
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for TimeUnit {
    /// 对应 Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl From<TimeUnit> for i32 {
    fn from(value: TimeUnit) -> i32 {
        value as i32
    }
}
