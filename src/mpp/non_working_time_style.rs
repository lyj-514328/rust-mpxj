//! Corresponds to Java: `org/mpxj/mpp/NonWorkingTimeStyle.java`
//!
//! How non-working time is drawn on the Gantt chart.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `public enum NonWorkingTimeStyle implements MpxjEnum` | same-named `enum` + `impl crate::MpxjEnum` | Constants are ordered as declared in Java, and the discriminant value is Java's int value |
//! | `private static final NonWorkingTimeStyle[] TYPE_VALUES = EnumHelper.createTypeArray(NonWorkingTimeStyle.class)` | the `match` in [`NonWorkingTimeStyle::get_instance`] + [`NonWorkingTimeStyle::ALL`] | The table length is 3 (offset 0); values 0..2 are all filled, so Java's `getInstance` never returns `null` |
//! | `getInstance(Number type)` | [`NonWorkingTimeStyle::get_instance_optional`] | Rust has no overloading, so an `_optional` suffix is added |
//! | `getName()` / `toString()` | [`NonWorkingTimeStyle::get_name`] / `Display` | The name is not localized, matching Java |
//!
//! Skipped Java members: the reflective table construction in `EnumHelper.createTypeArray` (replaced on the Rust side by `match` + `ALL`).
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// How non-working time is drawn.
///
/// Corresponds to Java: `public enum NonWorkingTimeStyle`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NonWorkingTimeStyle {
    /// Corresponds to Java: `BEHIND(0, "Behind")`
    Behind = 0,
    /// Corresponds to Java: `IN_FRONT(1, "In Front")`
    InFront = 1,
    /// Corresponds to Java: `DO_NOT_DRAW(2, "Do Not Draw")`
    DoNotDraw = 2,
}

impl NonWorkingTimeStyle {
/// All values, in the same order as declared in Java.
    ///
/// Corresponds to Java: `private static final NonWorkingTimeStyle[] TYPE_VALUES` (after removing the reflective table construction).
    pub const ALL: [NonWorkingTimeStyle; 3] = [
        NonWorkingTimeStyle::Behind,
        NonWorkingTimeStyle::InFront,
        NonWorkingTimeStyle::DoNotDraw,
    ];

/// Get the enum instance by int value.
    ///
    /// Corresponds to Java: `public static NonWorkingTimeStyle getInstance(int type)`
    ///
/// On out of bounds, Java first falls back to `IN_FRONT.getValue()` (1) before indexing the table, so it never returns `null`.
    pub fn get_instance(value: i32) -> NonWorkingTimeStyle {
        match value {
            0 => NonWorkingTimeStyle::Behind,
            1 => NonWorkingTimeStyle::InFront,
            2 => NonWorkingTimeStyle::DoNotDraw,
            // Java: type < 0 || type >= TYPE_VALUES.length → type = IN_FRONT.getValue()
            _ => NonWorkingTimeStyle::InFront,
        }
    }

    /// Corresponds to Java: `public static NonWorkingTimeStyle getInstance(Number type)`
    ///
/// On the Java side `null` goes through `getInstance(-1)`, which ultimately falls back to `IN_FRONT`.
    pub fn get_instance_optional(value: Option<i32>) -> NonWorkingTimeStyle {
        NonWorkingTimeStyle::get_instance(value.unwrap_or(-1))
    }

/// Get the name (not localized).
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            NonWorkingTimeStyle::Behind => "Behind",
            NonWorkingTimeStyle::InFront => "In Front",
            NonWorkingTimeStyle::DoNotDraw => "Do Not Draw",
        }
    }

    /// Corresponds to Java: `@Override public int getValue()`
    pub fn get_value(&self) -> i32 {
        *self as i32
    }
}

impl Display for NonWorkingTimeStyle {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java returns `getName()`; the Rust side matches.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl MpxjEnum for NonWorkingTimeStyle {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.get_value()
    }
}

impl From<NonWorkingTimeStyle> for i32 {
/// A Rust-specific convenience conversion, equivalent to [`NonWorkingTimeStyle::get_value`].
    fn from(value: NonWorkingTimeStyle) -> i32 {
        value.get_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// The discriminant value is Java's int value, and `getInstance` round-trips consistently.
    #[test]
    fn round_trips_through_int_values() {
        for (index, item) in NonWorkingTimeStyle::ALL.iter().enumerate() {
            assert_eq!(item.get_value(), index as i32);
            assert_eq!(NonWorkingTimeStyle::get_instance(index as i32), *item);
            assert_eq!(MpxjEnum::value(item), index as i32);
        }
    }

/// Out of bounds falls back to IN_FRONT (Java never returns null).
    #[test]
    fn out_of_range_falls_back_to_in_front() {
        for value in [-1, -100, 3, 4, 1000, i32::MAX, i32::MIN] {
            assert_eq!(
                NonWorkingTimeStyle::get_instance(value),
                NonWorkingTimeStyle::InFront
            );
        }
        assert_eq!(
            NonWorkingTimeStyle::get_instance_optional(None),
            NonWorkingTimeStyle::InFront
        );
        assert_eq!(
            NonWorkingTimeStyle::get_instance_optional(Some(2)),
            NonWorkingTimeStyle::DoNotDraw
        );
    }

/// The name and `toString()` match Java.
    #[test]
    fn names_and_to_string_match_java() {
        assert_eq!(NonWorkingTimeStyle::Behind.get_name(), "Behind");
        assert_eq!(NonWorkingTimeStyle::InFront.to_string(), "In Front");
        assert_eq!(NonWorkingTimeStyle::DoNotDraw.to_string(), "Do Not Draw");
    }
}
