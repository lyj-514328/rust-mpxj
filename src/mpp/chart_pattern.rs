//! Corresponds to Java: `org/mpxj/mpp/ChartPattern.java`
//!
//! Fill patterns for the middle section of Gantt chart bars.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `public enum ChartPattern implements MpxjEnum` | same-named `enum` + `impl crate::MpxjEnum` | Constants are ordered as declared in Java, and the discriminant value is Java's int value |
//! | `private static final ChartPattern[] TYPE_VALUES = EnumHelper.createTypeArray(ChartPattern.class)` | the `match` in [`ChartPattern::get_instance`] + [`ChartPattern::ALL`] | The table length is 12 (offset 0); values 0..11 are all filled, so Java's `getInstance` never returns `null` |
//! | `getInstance(Number type)` | [`ChartPattern::get_instance_optional`] | Rust has no overloading, so an `_optional` suffix is added |
//! | `getName()` / `toString()` | [`ChartPattern::get_name`] / `Display` | The name is not localized, matching Java |
//!
//! Skipped Java members: the reflective table construction in `EnumHelper.createTypeArray` (replaced on the Rust side by `match` + `ALL`).
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// Gantt chart bar fill pattern.
///
/// Corresponds to Java: `public enum ChartPattern`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChartPattern {
    /// Corresponds to Java: `TRANSPARENT(0, "Transparent")`
    Transparent = 0,
    /// Corresponds to Java: `SOLID(1, "Solid")`
    Solid = 1,
    /// Corresponds to Java: `LIGHTDOTTED(2, "Light Dotted")`
    Lightdotted = 2,
    /// Corresponds to Java: `DOTTED(3, "Dotted")`
    Dotted = 3,
    /// Corresponds to Java: `HEAVYDOTTED(4, "Heavy Dotted")`
    Heavydotted = 4,
    /// Corresponds to Java: `BACKSLASH(5, "Back Slash")`
    Backslash = 5,
    /// Corresponds to Java: `FORWARDSLASH(6, "Forward Slash")`
    Forwardslash = 6,
    /// Corresponds to Java: `CHECKERED(7, "Checkered")`
    Checkered = 7,
    /// Corresponds to Java: `VERTICALSTRIPE(8, "Vertical Stripe")`
    Verticalstripe = 8,
    /// Corresponds to Java: `HORIZONTALSTRIPE(9, "Horizontal Stripe")`
    Horizontalstripe = 9,
    /// Corresponds to Java: `GRID(10, "Grid")`
    Grid = 10,
    /// Corresponds to Java: `SOLIDHAIRY(11, "Solid Hairy")`
    Solidhairy = 11,
}

impl ChartPattern {
/// All values, in the same order as declared in Java.
    ///
/// Corresponds to Java: `private static final ChartPattern[] TYPE_VALUES` (after removing the reflective table construction).
    pub const ALL: [ChartPattern; 12] = [
        ChartPattern::Transparent,
        ChartPattern::Solid,
        ChartPattern::Lightdotted,
        ChartPattern::Dotted,
        ChartPattern::Heavydotted,
        ChartPattern::Backslash,
        ChartPattern::Forwardslash,
        ChartPattern::Checkered,
        ChartPattern::Verticalstripe,
        ChartPattern::Horizontalstripe,
        ChartPattern::Grid,
        ChartPattern::Solidhairy,
    ];

/// Get the enum instance by int value.
    ///
    /// Corresponds to Java: `public static ChartPattern getInstance(int type)`
    ///
/// On out of bounds, Java first falls back to `TRANSPARENT.getValue()` (0) before indexing the table, so it never returns `null`.
    pub fn get_instance(value: i32) -> ChartPattern {
        match value {
            0 => ChartPattern::Transparent,
            1 => ChartPattern::Solid,
            2 => ChartPattern::Lightdotted,
            3 => ChartPattern::Dotted,
            4 => ChartPattern::Heavydotted,
            5 => ChartPattern::Backslash,
            6 => ChartPattern::Forwardslash,
            7 => ChartPattern::Checkered,
            8 => ChartPattern::Verticalstripe,
            9 => ChartPattern::Horizontalstripe,
            10 => ChartPattern::Grid,
            11 => ChartPattern::Solidhairy,
            // Java: type < 0 || type >= TYPE_VALUES.length → type = TRANSPARENT.getValue()
            _ => ChartPattern::Transparent,
        }
    }

    /// Corresponds to Java: `public static ChartPattern getInstance(Number type)`
    ///
/// On the Java side `null` goes through `getInstance(-1)`, which ultimately falls back to `TRANSPARENT`.
    pub fn get_instance_optional(value: Option<i32>) -> ChartPattern {
        ChartPattern::get_instance(value.unwrap_or(-1))
    }

/// Get the pattern name (not localized).
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            ChartPattern::Transparent => "Transparent",
            ChartPattern::Solid => "Solid",
            ChartPattern::Lightdotted => "Light Dotted",
            ChartPattern::Dotted => "Dotted",
            ChartPattern::Heavydotted => "Heavy Dotted",
            ChartPattern::Backslash => "Back Slash",
            ChartPattern::Forwardslash => "Forward Slash",
            ChartPattern::Checkered => "Checkered",
            ChartPattern::Verticalstripe => "Vertical Stripe",
            ChartPattern::Horizontalstripe => "Horizontal Stripe",
            ChartPattern::Grid => "Grid",
            ChartPattern::Solidhairy => "Solid Hairy",
        }
    }

    /// Corresponds to Java: `@Override public int getValue()`
    pub fn get_value(&self) -> i32 {
        *self as i32
    }
}

impl Display for ChartPattern {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java returns `getName()`; the Rust side matches.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl MpxjEnum for ChartPattern {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.get_value()
    }
}

impl From<ChartPattern> for i32 {
/// A Rust-specific convenience conversion, equivalent to [`ChartPattern::get_value`].
    fn from(value: ChartPattern) -> i32 {
        value.get_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// The discriminant value is Java's int value, and `getInstance` round-trips consistently.
    #[test]
    fn round_trips_through_int_values() {
        for (index, item) in ChartPattern::ALL.iter().enumerate() {
            assert_eq!(item.get_value(), index as i32);
            assert_eq!(ChartPattern::get_instance(index as i32), *item);
            assert_eq!(MpxjEnum::value(item), index as i32);
        }
    }

/// Out of bounds falls back to TRANSPARENT (Java never returns null).
    #[test]
    fn out_of_range_falls_back_to_transparent() {
        for value in [-1, -100, 12, 13, 1000, i32::MAX, i32::MIN] {
            assert_eq!(ChartPattern::get_instance(value), ChartPattern::Transparent);
        }
        assert_eq!(
            ChartPattern::get_instance_optional(None),
            ChartPattern::Transparent
        );
        assert_eq!(
            ChartPattern::get_instance_optional(Some(11)),
            ChartPattern::Solidhairy
        );
    }

/// The name and `toString()` match Java.
    #[test]
    fn names_and_to_string_match_java() {
        assert_eq!(ChartPattern::Transparent.get_name(), "Transparent");
        assert_eq!(ChartPattern::Lightdotted.get_name(), "Light Dotted");
        assert_eq!(ChartPattern::Heavydotted.to_string(), "Heavy Dotted");
        assert_eq!(ChartPattern::Backslash.to_string(), "Back Slash");
        assert_eq!(ChartPattern::Solidhairy.to_string(), "Solid Hairy");
    }
}
