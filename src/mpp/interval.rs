//! Corresponds to Java: `org/mpxj/mpp/Interval.java`
//!
//! Daily / weekly / monthly time intervals (e.g. the repeat interval of progress lines).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `public enum Interval implements MpxjEnum` | same-named `enum` + `impl crate::MpxjEnum` | Constants are ordered as declared in Java, and the discriminant value is Java's int value |
//! | `private static final Interval[] TYPE_VALUES = EnumHelper.createTypeArray(Interval.class)` | the `match` in [`Interval::get_instance`] + [`Interval::ALL`] | The table length is 3 (offset 0); values 0..2 are all filled, so Java's `getInstance` never returns `null` |
//! | `getInstance(Number type)` | [`Interval::get_instance_optional`] | Rust has no overloading, so an `_optional` suffix is added |
//! | `getName()` / `toString()` | [`Interval::get_name`] / `Display` | The name is not localized, matching Java |
//!
//! Skipped Java members: the reflective table construction in `EnumHelper.createTypeArray` (replaced on the Rust side by `match` + `ALL`).
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// Time interval.
///
/// Corresponds to Java: `public enum Interval`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Interval {
    /// Corresponds to Java: `DAILY(0, "Daily")`
    Daily = 0,
    /// Corresponds to Java: `WEEKLY(1, "Weekly")`
    Weekly = 1,
    /// Corresponds to Java: `MONTHLY(2, "Monthly")`
    Monthly = 2,
}

impl Interval {
/// All values, in the same order as declared in Java.
    ///
/// Corresponds to Java: `private static final Interval[] TYPE_VALUES` (after removing the reflective table construction).
    pub const ALL: [Interval; 3] = [Interval::Daily, Interval::Weekly, Interval::Monthly];

/// Get the enum instance by int value.
    ///
    /// Corresponds to Java: `public static Interval getInstance(int type)`
    ///
/// On out of bounds, Java first falls back to `DAILY.getValue()` (0) before indexing the table, so it never returns `null`.
    pub fn get_instance(value: i32) -> Interval {
        match value {
            0 => Interval::Daily,
            1 => Interval::Weekly,
            2 => Interval::Monthly,
            // Java: type < 0 || type >= TYPE_VALUES.length → type = DAILY.getValue()
            _ => Interval::Daily,
        }
    }

    /// Corresponds to Java: `public static Interval getInstance(Number type)`
    ///
/// On the Java side `null` goes through `getInstance(-1)`, which ultimately falls back to `DAILY`.
    pub fn get_instance_optional(value: Option<i32>) -> Interval {
        Interval::get_instance(value.unwrap_or(-1))
    }

/// Get the interval name (not localized).
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            Interval::Daily => "Daily",
            Interval::Weekly => "Weekly",
            Interval::Monthly => "Monthly",
        }
    }

    /// Corresponds to Java: `@Override public int getValue()`
    pub fn get_value(&self) -> i32 {
        *self as i32
    }
}

impl Display for Interval {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java returns `getName()`; the Rust side matches.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl MpxjEnum for Interval {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.get_value()
    }
}

impl From<Interval> for i32 {
/// A Rust-specific convenience conversion, equivalent to [`Interval::get_value`].
    fn from(value: Interval) -> i32 {
        value.get_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// The discriminant value is Java's int value, and `getInstance` round-trips consistently.
    #[test]
    fn round_trips_through_int_values() {
        for (index, item) in Interval::ALL.iter().enumerate() {
            assert_eq!(item.get_value(), index as i32);
            assert_eq!(Interval::get_instance(index as i32), *item);
            assert_eq!(MpxjEnum::value(item), index as i32);
        }
    }

/// Out of bounds falls back to DAILY (Java never returns null).
    #[test]
    fn out_of_range_falls_back_to_daily() {
        for value in [-1, -100, 3, 4, 1000, i32::MAX, i32::MIN] {
            assert_eq!(Interval::get_instance(value), Interval::Daily);
        }
        assert_eq!(Interval::get_instance_optional(None), Interval::Daily);
        assert_eq!(
            Interval::get_instance_optional(Some(2)),
            Interval::Monthly
        );
    }

/// The name and `toString()` match Java.
    #[test]
    fn names_and_to_string_match_java() {
        assert_eq!(Interval::Daily.get_name(), "Daily");
        assert_eq!(Interval::Weekly.to_string(), "Weekly");
        assert_eq!(Interval::Monthly.to_string(), "Monthly");
    }
}
