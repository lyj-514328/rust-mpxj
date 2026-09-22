//! Corresponds to Java: `org/mpxj/TimeUnit.java`

use crate::mpxj_enum::MpxjEnum;
use std::fmt::{self, Display, Formatter};

/// Enum for time units.
///
/// Corresponds to Java: `public enum TimeUnit implements MpxjEnum`
///
/// Each constant in Java carries three fields `(int value, String name, boolean elapsed)`,
/// and `EnumHelper.createTypeArray` is used to build the int → enum array via reflection. On the Rust side:
/// * `value` is expressed with explicit discriminants;
/// * `name` / `elapsed` are returned by methods ([`TimeUnit::get_name`] / [`TimeUnit::is_elapsed`]);
/// * reflection is no longer used, and the mapping is done by the `match` in [`TimeUnit::get_instance`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TimeUnit {
    /// Corresponds to Java: `MINUTES(0, "m", false)`
    Minutes = 0,
    /// Corresponds to Java: `HOURS(1, "h", false)`
    Hours = 1,
    /// Corresponds to Java: `DAYS(2, "d", false)`
    Days = 2,
    /// Corresponds to Java: `WEEKS(3, "w", false)`
    Weeks = 3,
    /// Corresponds to Java: `MONTHS(4, "mo", false)`
    Months = 4,
    /// Corresponds to Java: `PERCENT(5, "%", false)`
    Percent = 5,
    /// Corresponds to Java: `YEARS(6, "y", false)`
    Years = 6,
    /// Corresponds to Java: `ELAPSED_MINUTES(7, "em", true)`
    ElapsedMinutes = 7,
    /// Corresponds to Java: `ELAPSED_HOURS(8, "eh", true)`
    ElapsedHours = 8,
    /// Corresponds to Java: `ELAPSED_DAYS(9, "ed", true)`
    ElapsedDays = 9,
    /// Corresponds to Java: `ELAPSED_WEEKS(10, "ew", true)`
    ElapsedWeeks = 10,
    /// Corresponds to Java: `ELAPSED_MONTHS(11, "emo", true)`
    ElapsedMonths = 11,
    /// Corresponds to Java: `ELAPSED_YEARS(12, "ey", true)`
    ElapsedYears = 12,
    /// Corresponds to Java: `ELAPSED_PERCENT(13, "e%", true)`
    ElapsedPercent = 13,
}

impl TimeUnit {
    /// All values, sorted ascending by int value.
    ///
    /// Corresponds to Java: `private static final TimeUnit[] TYPE_VALUES` (excluding the reflection part).
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

    /// Returns the enum instance for the given int value, falling back to `DAYS` when out of bounds.
    ///
    /// Corresponds to Java: `public static TimeUnit getInstance(int type)`
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

    /// Corresponds to Java: `public static TimeUnit getInstance(Number type)`
    ///
    /// In Java, `null` goes through `getInstance(-1)` and ultimately falls back to `DAYS`.
    pub fn get_instance_optional(value: Option<i32>) -> TimeUnit {
        TimeUnit::get_instance(value.unwrap_or(-1))
    }

    /// Returns the name corresponding to this enum (such as `"m"` / `"emo"`).
    ///
    /// Corresponds to Java: `public String getName()`
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

    /// Determines whether this time unit represents elapsed time.
    ///
    /// Corresponds to Java: `public boolean isElapsed()`
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
    /// Returns the int value associated with this enum.
    ///
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for TimeUnit {
    /// Corresponds to Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl From<TimeUnit> for i32 {
    fn from(value: TimeUnit) -> i32 {
        value as i32
    }
}
