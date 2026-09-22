//! Corresponds to Java: `org/mpxj/mpp/LinkStyle.java`
//!
//! The way relation (link) lines are drawn on the Gantt chart.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `public enum LinkStyle implements MpxjEnum` | same-named `enum` + `impl crate::MpxjEnum` | Constants are ordered as declared in Java, and the discriminant value is Java's int value |
//! | `private static final LinkStyle[] TYPE_VALUES = EnumHelper.createTypeArray(LinkStyle.class)` | the `match` in [`LinkStyle::get_instance`] + [`LinkStyle::ALL`] | The table length is 3 (offset 0); values 0..2 are all filled, so Java's `getInstance` never returns `null` |
//! | `getInstance(Number type)` | [`LinkStyle::get_instance_optional`] | Rust has no overloading, so an `_optional` suffix is added |
//! | `getName()` / `toString()` | [`LinkStyle::get_name`] / `Display` | The name is not localized, matching Java |
//!
//! Skipped Java members: the reflective table construction in `EnumHelper.createTypeArray` (replaced on the Rust side by `match` + `ALL`).
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// How relation lines are drawn.
///
/// Corresponds to Java: `public enum LinkStyle`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkStyle {
    /// Corresponds to Java: `NONE(0, "None")`
    None = 0,
    /// Corresponds to Java: `END_TOP(1, "End Top")`
    EndTop = 1,
    /// Corresponds to Java: `END_START(2, "End Start")`
    EndStart = 2,
}

impl LinkStyle {
/// All values, in the same order as declared in Java.
    ///
/// Corresponds to Java: `private static final LinkStyle[] TYPE_VALUES` (after removing the reflective table construction).
    pub const ALL: [LinkStyle; 3] = [LinkStyle::None, LinkStyle::EndTop, LinkStyle::EndStart];

/// Get the enum instance by int value.
    ///
    /// Corresponds to Java: `public static LinkStyle getInstance(int type)`
    ///
/// On out of bounds, Java first falls back to `END_TOP.getValue()` (1) before indexing the table, so it never returns `null`.
    pub fn get_instance(value: i32) -> LinkStyle {
        match value {
            0 => LinkStyle::None,
            1 => LinkStyle::EndTop,
            2 => LinkStyle::EndStart,
            // Java: type < 0 || type >= TYPE_VALUES.length → type = END_TOP.getValue()
            _ => LinkStyle::EndTop,
        }
    }

    /// Corresponds to Java: `public static LinkStyle getInstance(Number type)`
    ///
/// On the Java side `null` goes through `getInstance(-1)`, which ultimately falls back to `END_TOP`.
    pub fn get_instance_optional(value: Option<i32>) -> LinkStyle {
        LinkStyle::get_instance(value.unwrap_or(-1))
    }

/// Get the name (not localized).
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            LinkStyle::None => "None",
            LinkStyle::EndTop => "End Top",
            LinkStyle::EndStart => "End Start",
        }
    }

    /// Corresponds to Java: `@Override public int getValue()`
    pub fn get_value(&self) -> i32 {
        *self as i32
    }
}

impl Display for LinkStyle {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java returns `getName()`; the Rust side matches.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl MpxjEnum for LinkStyle {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.get_value()
    }
}

impl From<LinkStyle> for i32 {
/// A Rust-specific convenience conversion, equivalent to [`LinkStyle::get_value`].
    fn from(value: LinkStyle) -> i32 {
        value.get_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// The discriminant value is Java's int value, and `getInstance` round-trips consistently.
    #[test]
    fn round_trips_through_int_values() {
        for (index, item) in LinkStyle::ALL.iter().enumerate() {
            assert_eq!(item.get_value(), index as i32);
            assert_eq!(LinkStyle::get_instance(index as i32), *item);
            assert_eq!(MpxjEnum::value(item), index as i32);
        }
    }

/// Out of bounds falls back to END_TOP (Java never returns null).
    #[test]
    fn out_of_range_falls_back_to_end_top() {
        for value in [-1, -100, 3, 4, 1000, i32::MAX, i32::MIN] {
            assert_eq!(LinkStyle::get_instance(value), LinkStyle::EndTop);
        }
        assert_eq!(
            LinkStyle::get_instance_optional(None),
            LinkStyle::EndTop
        );
        assert_eq!(LinkStyle::get_instance_optional(Some(0)), LinkStyle::None);
    }

/// The name and `toString()` match Java.
    #[test]
    fn names_and_to_string_match_java() {
        assert_eq!(LinkStyle::None.get_name(), "None");
        assert_eq!(LinkStyle::EndTop.to_string(), "End Top");
        assert_eq!(LinkStyle::EndStart.to_string(), "End Start");
    }
}
