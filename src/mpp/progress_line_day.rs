//! Corresponds to Java: `org/mpxj/mpp/ProgressLineDay.java`
//!
//! Enum for the "day" used as the drawing point of a progress line.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `public enum ProgressLineDay implements MpxjEnum` | `enum` of the same name + `impl crate::MpxjEnum` | constants are ordered as declared in Java, and the discriminant is the Java int value |
//! | constructor parameter `java.time.DayOfWeek day` / `DayOfWeek getDay()` | `Option<chrono::Weekday>` | `java.time.DayOfWeek` is uniformly represented by [`chrono::Weekday`] on the Rust side (consistent with `crate::common::day_of_week_helper::DayOfWeekHelper`); in Java the value for `DAY` / `WORKINGDAY` / `NONWORKINGDAY` is `null`, hence `Option` |
//! | `private static final ProgressLineDay[] TYPE_VALUES = EnumHelper.createTypeArray(ProgressLineDay.class, 1)` | the `match` in [`ProgressLineDay::get_instance`] + [`ProgressLineDay::ALL`] | the table length is 10 constants + offset 1 = 11: values 1..10 are populated, **index 0 stays `null`**; and Java returns `null` directly for out-of-range values (it does not fall back to a default) |
//! | `getInstance(int)` may return `null` | `Option<ProgressLineDay>` | the "`null` -> `Option<T>`" convention |
//! | `getNextDay()` (`value + 1`, wrapping back to 1 when it exceeds 7, then table lookup) | [`ProgressLineDay::get_next_day`], returns a non-nullable `ProgressLineDay` | `value + 1` always wraps back to 1 when it exceeds 7, so the result always falls in 1..=7 (`SATURDAY` as well as `DAY` / `WORKINGDAY` / `NONWORKINGDAY` all yield `SUNDAY`), the table lookup always hits, and the Java side never returns `null` |
//! | does not override `toString()` (inherits `java.lang.Enum.toString()`, which outputs the **constant name**) | `Display`, outputs `SUNDAY` / `MONDAY` / ... / `NONWORKINGDAY` | A Java enum has a `toString()` even when not overridden, and its value is the constant name; the Rust side implements `Display` accordingly (used for the string representation of `GanttChartView`, etc.) |
//!
//! Note: this class on the Java side has **no** `getInstance(Number)` overload, so the Rust side does not provide one either
//! `get_instance_optional`.
//!
//! Skipped Java members: the reflection-based table building of `EnumHelper.createTypeArray` (replaced on the Rust side by `match` + `ALL`).
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::fmt::{self, Display, Formatter};

use chrono::Weekday;

use crate::mpxj_enum::MpxjEnum;

/// The "day" used as the drawing point of a progress line.
///
/// Corresponds to Java: `public enum ProgressLineDay`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProgressLineDay {
    /// Corresponds to Java: `SUNDAY(1, DayOfWeek.SUNDAY)`
    Sunday = 1,
    /// Corresponds to Java: `MONDAY(2, DayOfWeek.MONDAY)`
    Monday = 2,
    /// Corresponds to Java: `TUESDAY(3, DayOfWeek.TUESDAY)`
    Tuesday = 3,
    /// Corresponds to Java: `WEDNESDAY(4, DayOfWeek.WEDNESDAY)`
    Wednesday = 4,
    /// Corresponds to Java: `THURSDAY(5, DayOfWeek.THURSDAY)`
    Thursday = 5,
    /// Corresponds to Java: `FRIDAY(6, DayOfWeek.FRIDAY)`
    Friday = 6,
    /// Corresponds to Java: `SATURDAY(7, DayOfWeek.SATURDAY)`
    Saturday = 7,
    /// Corresponds to Java: `DAY(8, null)`
    Day = 8,
    /// Corresponds to Java: `WORKINGDAY(9, null)`
    Workingday = 9,
    /// Corresponds to Java: `NONWORKINGDAY(10, null)`
    Nonworkingday = 10,
}

impl ProgressLineDay {
/// All values, in the same order as the Java declaration order.
    ///
/// Corresponds to Java: `private static final ProgressLineDay[] TYPE_VALUES` (with reflection-based construction removed).
    pub const ALL: [ProgressLineDay; 10] = [
        ProgressLineDay::Sunday,
        ProgressLineDay::Monday,
        ProgressLineDay::Tuesday,
        ProgressLineDay::Wednesday,
        ProgressLineDay::Thursday,
        ProgressLineDay::Friday,
        ProgressLineDay::Saturday,
        ProgressLineDay::Day,
        ProgressLineDay::Workingday,
        ProgressLineDay::Nonworkingday,
    ];

/// Get the enum instance by int value; out of range (including 0, where the Java table has `null` at that index) returns `None`.
    ///
    /// Corresponds to Java: `public static ProgressLineDay getInstance(int type)`
    ///
/// Unlike other enums, the Java side here does **not** fall back to a default value: out of range returns `null` directly.
    pub fn get_instance(value: i32) -> Option<ProgressLineDay> {
        match value {
            1 => Some(ProgressLineDay::Sunday),
            2 => Some(ProgressLineDay::Monday),
            3 => Some(ProgressLineDay::Tuesday),
            4 => Some(ProgressLineDay::Wednesday),
            5 => Some(ProgressLineDay::Thursday),
            6 => Some(ProgressLineDay::Friday),
            7 => Some(ProgressLineDay::Saturday),
            8 => Some(ProgressLineDay::Day),
            9 => Some(ProgressLineDay::Workingday),
            10 => Some(ProgressLineDay::Nonworkingday),
            // Java: type < 0 || type >= TYPE_VALUES.length(11)→ null;
// index 0 is not populated and is likewise null
            _ => None,
        }
    }

    /// Corresponds to Java: `@Override public int getValue()`
    pub fn get_value(&self) -> i32 {
        *self as i32
    }

/// Get the weekday equivalent to this value; `DAY` / `WORKINGDAY` / `NONWORKINGDAY` return `None`.
    ///
    /// Corresponds to Java: `public DayOfWeek getDay()`
    pub fn get_day(&self) -> Option<Weekday> {
        match self {
            ProgressLineDay::Sunday => Some(Weekday::Sun),
            ProgressLineDay::Monday => Some(Weekday::Mon),
            ProgressLineDay::Tuesday => Some(Weekday::Tue),
            ProgressLineDay::Wednesday => Some(Weekday::Wed),
            ProgressLineDay::Thursday => Some(Weekday::Thu),
            ProgressLineDay::Friday => Some(Weekday::Fri),
            ProgressLineDay::Saturday => Some(Weekday::Sat),
            // Java: m_day == null
            ProgressLineDay::Day | ProgressLineDay::Workingday | ProgressLineDay::Nonworkingday => {
                None
            }
        }
    }

/// Get the next day in the sequence (Sunday -> Monday wraps around automatically).
    ///
    /// Corresponds to Java: `public ProgressLineDay getNextDay()`
    ///
/// The Java implementation is `value + 1`, wrapping back to 1 when it exceeds 7, then passed to `getInstance`: thus
/// in-week values increment normally (`SUNDAY` -> `MONDAY` ...), while `SATURDAY`(7), `DAY`(8),
/// `WORKINGDAY`(9), `NONWORKINGDAY`(10) all wrap around to `SUNDAY`(1). The result always hits
/// the lookup table and, like Java, never returns an empty value (the `unwrap_or` branch exists only to avoid `unwrap`,
/// and is never actually hit).
    pub fn get_next_day(&self) -> ProgressLineDay {
        let mut value = self.get_value() + 1;
        if value > 7 {
            value = 1;
        }
        ProgressLineDay::get_instance(value).unwrap_or(ProgressLineDay::Sunday)
    }
}

impl Display for ProgressLineDay {
/// Corresponds to Java: `toString()` inherited from `java.lang.Enum`
    ///
/// Java's `Enum.toString()` returns the constant name (e.g. `MONDAY`), which this class does not override, so the Rust side
/// outputs the same constant name.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ProgressLineDay::Sunday => "SUNDAY",
            ProgressLineDay::Monday => "MONDAY",
            ProgressLineDay::Tuesday => "TUESDAY",
            ProgressLineDay::Wednesday => "WEDNESDAY",
            ProgressLineDay::Thursday => "THURSDAY",
            ProgressLineDay::Friday => "FRIDAY",
            ProgressLineDay::Saturday => "SATURDAY",
            ProgressLineDay::Day => "DAY",
            ProgressLineDay::Workingday => "WORKINGDAY",
            ProgressLineDay::Nonworkingday => "NONWORKINGDAY",
        })
    }
}

impl MpxjEnum for ProgressLineDay {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.get_value()
    }
}

impl From<ProgressLineDay> for i32 {
/// Rust-specific convenience conversion, equivalent to [`ProgressLineDay::get_value`].
    fn from(value: ProgressLineDay) -> i32 {
        value.get_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// The discriminant is the Java int value, and `getInstance` round-trips consistently.
    #[test]
    fn round_trips_through_int_values() {
        for (index, item) in ProgressLineDay::ALL.iter().enumerate() {
            assert_eq!(item.get_value(), index as i32 + 1);
            assert_eq!(
                ProgressLineDay::get_instance(item.get_value()),
                Some(*item)
            );
            assert_eq!(MpxjEnum::value(item), index as i32 + 1);
        }
    }

/// The Java lookup table has null at index 0, and out of range does not fall back to a default value either.
    #[test]
    fn missing_slots_return_none() {
        for value in [-1, 0, 11, 100, i32::MAX, i32::MIN] {
            assert_eq!(ProgressLineDay::get_instance(value), None, "index {value}");
        }
    }

/// The `getDay()` mapping matches `java.time.DayOfWeek`, and non-weekday values return None.
    #[test]
    fn get_day_matches_java_day_of_week() {
        assert_eq!(ProgressLineDay::Sunday.get_day(), Some(Weekday::Sun));
        assert_eq!(ProgressLineDay::Monday.get_day(), Some(Weekday::Mon));
        assert_eq!(ProgressLineDay::Tuesday.get_day(), Some(Weekday::Tue));
        assert_eq!(ProgressLineDay::Wednesday.get_day(), Some(Weekday::Wed));
        assert_eq!(ProgressLineDay::Thursday.get_day(), Some(Weekday::Thu));
        assert_eq!(ProgressLineDay::Friday.get_day(), Some(Weekday::Fri));
        assert_eq!(ProgressLineDay::Saturday.get_day(), Some(Weekday::Sat));

        assert_eq!(ProgressLineDay::Day.get_day(), None);
        assert_eq!(ProgressLineDay::Workingday.get_day(), None);
        assert_eq!(ProgressLineDay::Nonworkingday.get_day(), None);
    }

/// The sequence and wrap-around behavior of `getNextDay()` match Java.
    #[test]
    fn get_next_day_wraps_and_handles_special_values() {
        assert_eq!(
            ProgressLineDay::Sunday.get_next_day(),
            ProgressLineDay::Monday
        );
        assert_eq!(
            ProgressLineDay::Monday.get_next_day(),
            ProgressLineDay::Tuesday
        );
// Java: wraps back to 1 when it exceeds 7, so the next day after SATURDAY(7) is SUNDAY(1)
        assert_eq!(
            ProgressLineDay::Saturday.get_next_day(),
            ProgressLineDay::Sunday
        );
// values above 7 wrap back to 1: DAY / WORKINGDAY / NONWORKINGDAY all yield SUNDAY
        for day in [
            ProgressLineDay::Day,
            ProgressLineDay::Workingday,
            ProgressLineDay::Nonworkingday,
        ] {
            assert_eq!(day.get_next_day(), ProgressLineDay::Sunday, "{day:?}");
        }
    }

/// `toString()` (Java's `Enum.toString()`) outputs the constant name.
    #[test]
    fn to_string_outputs_the_java_constant_name() {
        assert_eq!(ProgressLineDay::Sunday.to_string(), "SUNDAY");
        assert_eq!(ProgressLineDay::Monday.to_string(), "MONDAY");
        assert_eq!(ProgressLineDay::Saturday.to_string(), "SATURDAY");
        assert_eq!(ProgressLineDay::Day.to_string(), "DAY");
        assert_eq!(ProgressLineDay::Workingday.to_string(), "WORKINGDAY");
        assert_eq!(ProgressLineDay::Nonworkingday.to_string(), "NONWORKINGDAY");
        assert_eq!(i32::from(ProgressLineDay::Nonworkingday), 10);
        assert_eq!(format!("{:?}", ProgressLineDay::Sunday), "Sunday");
    }
}
