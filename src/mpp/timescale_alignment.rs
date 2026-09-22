//! Corresponds to Java: `org/mpxj/mpp/TimescaleAlignment.java`
//!
//! The alignment of Gantt chart timescale labels.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `public enum TimescaleAlignment implements MpxjEnum` | same-named `enum` + `impl crate::MpxjEnum` | Constants are ordered as declared in Java, and the discriminant value is Java's int value |
//! | `private static final TimescaleAlignment[] TYPE_VALUES = EnumHelper.createTypeArray(TimescaleAlignment.class)` | the `match` in [`TimescaleAlignment::get_instance`] + [`TimescaleAlignment::ALL`] | The table length is 3 (offset 0); values 0..2 are all filled, so Java's `getInstance` never returns `null` |
//! | `getInstance(Number type)` | [`TimescaleAlignment::get_instance_optional`] | Rust has no overloading, so an `_optional` suffix is added |
//! | `getName()` / `toString()` | [`TimescaleAlignment::get_name`] / `Display` | The name is not localized, matching Java |
//!
//! Skipped Java members: the reflective table construction in `EnumHelper.createTypeArray` (replaced on the Rust side by `match` + `ALL`).
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// The alignment of timescale labels.
///
/// Corresponds to Java: `public enum TimescaleAlignment`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimescaleAlignment {
    /// Corresponds to Java: `LEFT(0, "Left")`
    Left = 0,
    /// Corresponds to Java: `CENTER(1, "Center")`
    Center = 1,
    /// Corresponds to Java: `RIGHT(2, "Right")`
    Right = 2,
}

impl TimescaleAlignment {
/// All values, in the same order as declared in Java.
    ///
/// Corresponds to Java: `private static final TimescaleAlignment[] TYPE_VALUES` (after removing the reflective table construction).
    pub const ALL: [TimescaleAlignment; 3] = [
        TimescaleAlignment::Left,
        TimescaleAlignment::Center,
        TimescaleAlignment::Right,
    ];

/// Get the enum instance by int value.
    ///
    /// Corresponds to Java: `public static TimescaleAlignment getInstance(int type)`
    ///
/// On out of bounds, Java first falls back to `CENTER.getValue()` (1) before indexing the table, so it never returns `null`.
    pub fn get_instance(value: i32) -> TimescaleAlignment {
        match value {
            0 => TimescaleAlignment::Left,
            1 => TimescaleAlignment::Center,
            2 => TimescaleAlignment::Right,
            // Java: type < 0 || type >= TYPE_VALUES.length → type = CENTER.getValue()
            _ => TimescaleAlignment::Center,
        }
    }

    /// Corresponds to Java: `public static TimescaleAlignment getInstance(Number type)`
    ///
/// On the Java side `null` goes through `getInstance(-1)`, which ultimately falls back to `CENTER`.
    pub fn get_instance_optional(value: Option<i32>) -> TimescaleAlignment {
        TimescaleAlignment::get_instance(value.unwrap_or(-1))
    }

/// Get the name (not localized).
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            TimescaleAlignment::Left => "Left",
            TimescaleAlignment::Center => "Center",
            TimescaleAlignment::Right => "Right",
        }
    }

    /// Corresponds to Java: `@Override public int getValue()`
    pub fn get_value(&self) -> i32 {
        *self as i32
    }
}

impl Display for TimescaleAlignment {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java returns `getName()`; the Rust side matches.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl MpxjEnum for TimescaleAlignment {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.get_value()
    }
}

impl From<TimescaleAlignment> for i32 {
/// A Rust-specific convenience conversion, equivalent to [`TimescaleAlignment::get_value`].
    fn from(value: TimescaleAlignment) -> i32 {
        value.get_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// The discriminant value is Java's int value, and `getInstance` round-trips consistently.
    #[test]
    fn round_trips_through_int_values() {
        for (index, item) in TimescaleAlignment::ALL.iter().enumerate() {
            assert_eq!(item.get_value(), index as i32);
            assert_eq!(TimescaleAlignment::get_instance(index as i32), *item);
            assert_eq!(MpxjEnum::value(item), index as i32);
        }
    }

/// Out of bounds falls back to CENTER (Java never returns null).
    #[test]
    fn out_of_range_falls_back_to_center() {
        for value in [-1, -100, 3, 4, 1000, i32::MAX, i32::MIN] {
            assert_eq!(
                TimescaleAlignment::get_instance(value),
                TimescaleAlignment::Center
            );
        }
        assert_eq!(
            TimescaleAlignment::get_instance_optional(None),
            TimescaleAlignment::Center
        );
        assert_eq!(
            TimescaleAlignment::get_instance_optional(Some(0)),
            TimescaleAlignment::Left
        );
    }

/// The name and `toString()` match Java.
    #[test]
    fn names_and_to_string_match_java() {
        assert_eq!(TimescaleAlignment::Left.get_name(), "Left");
        assert_eq!(TimescaleAlignment::Center.to_string(), "Center");
        assert_eq!(TimescaleAlignment::Right.to_string(), "Right");
    }
}
