//! Corresponds to Java: `org/mpxj/mpp/LineStyle.java`
//!
//! Grid line styles used by Microsoft Project.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `public enum LineStyle implements MpxjEnum` | same-named `enum` + `impl crate::MpxjEnum` | Constants are ordered as declared in Java, and the discriminant value is Java's int value |
//! | `private static final LineStyle[] TYPE_VALUES = EnumHelper.createTypeArray(LineStyle.class, 1)` | the `match` in [`LineStyle::get_instance`] + [`LineStyle::ALL`] | The table length is the constant count 5 + offset 1 = **6**; values 0..4 are filled and **index 5 remains `null`**: `getInstance(5)` returns `null` in Java and `None` on the Rust side |
//! | `getInstance(int)` may return `null` | `Option<LineStyle>` | The "`null` → `Option<T>`" convention (for similar handling see `crate::accrue_type::AccrueType`) |
//! | `getInstance(Number type)` | [`LineStyle::get_instance_optional`] | Rust has no overloading, so an `_optional` suffix is added; note that the outermost `Option` is the "index 5 → null" case above |
//! | `getName()` / `toString()` | [`LineStyle::get_name`] / `Display` | The name is not localized, matching Java |
//!
//! Skipped Java members: the reflective table construction in `EnumHelper.createTypeArray` (replaced on the Rust side by `match` + `ALL`).
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// Grid line style.
///
/// Corresponds to Java: `public enum LineStyle`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineStyle {
    /// Corresponds to Java: `NONE(0, "None")`
    None = 0,
    /// Corresponds to Java: `SOLID(1, "Solid")`
    Solid = 1,
    /// Corresponds to Java: `DOTTED1(2, "Dotted1")`
    Dotted1 = 2,
    /// Corresponds to Java: `DOTTED2(3, "Dotted2")`
    Dotted2 = 3,
    /// Corresponds to Java: `DASHED(4, "Dashed")`
    Dashed = 4,
}

impl LineStyle {
/// All values, in the same order as declared in Java.
    ///
/// Corresponds to Java: `private static final LineStyle[] TYPE_VALUES` (after removing the reflective table construction).
    pub const ALL: [LineStyle; 5] = [
        LineStyle::None,
        LineStyle::Solid,
        LineStyle::Dotted1,
        LineStyle::Dotted2,
        LineStyle::Dashed,
    ];

/// Get the enum instance by int value; out of bounds falls back to `NONE`, and array index 5 (`null` in Java) returns `None`.
    ///
    /// Corresponds to Java: `public static LineStyle getInstance(int type)`
    pub fn get_instance(value: i32) -> Option<LineStyle> {
        match value {
            0 => Some(LineStyle::None),
            1 => Some(LineStyle::Solid),
            2 => Some(LineStyle::Dotted1),
            3 => Some(LineStyle::Dotted2),
            4 => Some(LineStyle::Dashed),
// Java: TYPE_VALUES has length 6, index 5 is not filled and its value is null
            5 => None,
            // Java: type < 0 || type >= TYPE_VALUES.length → type = NONE.getValue()(0)
            _ => Some(LineStyle::None),
        }
    }

    /// Corresponds to Java: `public static LineStyle getInstance(Number type)`
    ///
/// On the Java side `null` goes through `getInstance(-1)`, which ultimately falls back to `NONE`.
    pub fn get_instance_optional(value: Option<i32>) -> Option<LineStyle> {
        LineStyle::get_instance(value.unwrap_or(-1))
    }

/// Get the line style name (not localized).
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            LineStyle::None => "None",
            LineStyle::Solid => "Solid",
            LineStyle::Dotted1 => "Dotted1",
            LineStyle::Dotted2 => "Dotted2",
            LineStyle::Dashed => "Dashed",
        }
    }

    /// Corresponds to Java: `@Override public int getValue()`
    pub fn get_value(&self) -> i32 {
        *self as i32
    }
}

impl Display for LineStyle {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java returns `getName()`; the Rust side matches.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl MpxjEnum for LineStyle {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.get_value()
    }
}

impl From<LineStyle> for i32 {
/// A Rust-specific convenience conversion, equivalent to [`LineStyle::get_value`].
    fn from(value: LineStyle) -> i32 {
        value.get_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// The discriminant value is Java's int value, and `getInstance` round-trips consistently.
    #[test]
    fn round_trips_through_int_values() {
        for (index, item) in LineStyle::ALL.iter().enumerate() {
            assert_eq!(item.get_value(), index as i32);
            assert_eq!(LineStyle::get_instance(index as i32), Some(*item));
            assert_eq!(MpxjEnum::value(item), index as i32);
        }
    }

/// Index 5 is null in Java.
    #[test]
    fn index_five_is_none() {
        assert_eq!(LineStyle::get_instance(5), None);
    }

/// Out of bounds falls back to NONE; null in the `Number` overload also falls back to NONE.
    #[test]
    fn out_of_range_falls_back_to_none() {
        for value in [-1, -100, 6, 7, 1000, i32::MAX, i32::MIN] {
            assert_eq!(LineStyle::get_instance(value), Some(LineStyle::None));
        }
        assert_eq!(
            LineStyle::get_instance_optional(None),
            Some(LineStyle::None)
        );
        assert_eq!(
            LineStyle::get_instance_optional(Some(4)),
            Some(LineStyle::Dashed)
        );
    }

/// The name and `toString()` match Java.
    #[test]
    fn names_and_to_string_match_java() {
        assert_eq!(LineStyle::None.get_name(), "None");
        assert_eq!(LineStyle::Dotted1.to_string(), "Dotted1");
        assert_eq!(LineStyle::Dashed.to_string(), "Dashed");
    }
}
