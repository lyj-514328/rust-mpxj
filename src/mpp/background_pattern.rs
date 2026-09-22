//! Corresponds to Java: `org/mpxj/mpp/BackgroundPattern.java`
//!
//! Group fill patterns.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `public enum BackgroundPattern implements MpxjEnum` | same-named `enum` + `impl crate::MpxjEnum` | Constants are ordered as declared in Java, and the discriminant value is Java's int value |
//! | `private static final BackgroundPattern[] TYPE_VALUES = EnumHelper.createTypeArray(BackgroundPattern.class)` | the `match` in [`BackgroundPattern::get_instance`] + [`BackgroundPattern::ALL`] | The table length is 14 (offset 0); values 0..13 are all filled, so Java's `getInstance` never returns `null` |
//! | `getName()` / `toString()` | [`BackgroundPattern::get_name`] / `Display` | The name is not localized, matching Java |
//!
//! Note: on the Java side this class does **not** have a `getInstance(Number)` overload, so the Rust side does not provide
//! `get_instance_optional`.
//!
//! Skipped Java members: the reflective table construction in `EnumHelper.createTypeArray` (replaced on the Rust side by `match` + `ALL`).
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// Background fill pattern.
///
/// Corresponds to Java: `public enum BackgroundPattern`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackgroundPattern {
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
    /// Corresponds to Java: `INVERSEBACKSLASH(7, "Inverse Back Slash")`
    Inversebackslash = 7,
    /// Corresponds to Java: `INVERSEFORWARDSLASH(8, "Inverse Forward Slash")`
    Inverseforwardslash = 8,
    /// Corresponds to Java: `LIGHTVERTICALSTRIPE(9, "Light Vertical Stripe")`
    Lightverticalstripe = 9,
    /// Corresponds to Java: `HEAVYVERTICALSTRIPE(10, "Heavy Vertical Stripe")`
    Heavyverticalstripe = 10,
    /// Corresponds to Java: `CHECKERED(11, "Checkered")`
    Checkered = 11,
    /// Corresponds to Java: `DENSEFORWARDSLASH(12, "Dense Forward Slash")`
    Denseforwardslash = 12,
    /// Corresponds to Java: `INVERSECHECKERED(13, "Inverse Checkered")`
    Inversecheckered = 13,
}

impl BackgroundPattern {
/// All values, in the same order as declared in Java.
    ///
/// Corresponds to Java: `private static final BackgroundPattern[] TYPE_VALUES` (after removing the reflective table construction).
    pub const ALL: [BackgroundPattern; 14] = [
        BackgroundPattern::Transparent,
        BackgroundPattern::Solid,
        BackgroundPattern::Lightdotted,
        BackgroundPattern::Dotted,
        BackgroundPattern::Heavydotted,
        BackgroundPattern::Backslash,
        BackgroundPattern::Forwardslash,
        BackgroundPattern::Inversebackslash,
        BackgroundPattern::Inverseforwardslash,
        BackgroundPattern::Lightverticalstripe,
        BackgroundPattern::Heavyverticalstripe,
        BackgroundPattern::Checkered,
        BackgroundPattern::Denseforwardslash,
        BackgroundPattern::Inversecheckered,
    ];

/// Get the enum instance by int value.
    ///
    /// Corresponds to Java: `public static BackgroundPattern getInstance(int type)`
    ///
/// On out of bounds, Java first falls back to `TRANSPARENT.getValue()` (0) before indexing the table, so it never returns `null`.
    pub fn get_instance(value: i32) -> BackgroundPattern {
        match value {
            0 => BackgroundPattern::Transparent,
            1 => BackgroundPattern::Solid,
            2 => BackgroundPattern::Lightdotted,
            3 => BackgroundPattern::Dotted,
            4 => BackgroundPattern::Heavydotted,
            5 => BackgroundPattern::Backslash,
            6 => BackgroundPattern::Forwardslash,
            7 => BackgroundPattern::Inversebackslash,
            8 => BackgroundPattern::Inverseforwardslash,
            9 => BackgroundPattern::Lightverticalstripe,
            10 => BackgroundPattern::Heavyverticalstripe,
            11 => BackgroundPattern::Checkered,
            12 => BackgroundPattern::Denseforwardslash,
            13 => BackgroundPattern::Inversecheckered,
            // Java: type < 0 || type >= TYPE_VALUES.length → type = TRANSPARENT.getValue()
            _ => BackgroundPattern::Transparent,
        }
    }

/// Get the pattern name (not localized).
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            BackgroundPattern::Transparent => "Transparent",
            BackgroundPattern::Solid => "Solid",
            BackgroundPattern::Lightdotted => "Light Dotted",
            BackgroundPattern::Dotted => "Dotted",
            BackgroundPattern::Heavydotted => "Heavy Dotted",
            BackgroundPattern::Backslash => "Back Slash",
            BackgroundPattern::Forwardslash => "Forward Slash",
            BackgroundPattern::Inversebackslash => "Inverse Back Slash",
            BackgroundPattern::Inverseforwardslash => "Inverse Forward Slash",
            BackgroundPattern::Lightverticalstripe => "Light Vertical Stripe",
            BackgroundPattern::Heavyverticalstripe => "Heavy Vertical Stripe",
            BackgroundPattern::Checkered => "Checkered",
            BackgroundPattern::Denseforwardslash => "Dense Forward Slash",
            BackgroundPattern::Inversecheckered => "Inverse Checkered",
        }
    }

    /// Corresponds to Java: `@Override public int getValue()`
    pub fn get_value(&self) -> i32 {
        *self as i32
    }
}

impl Display for BackgroundPattern {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java returns `getName()`; the Rust side matches.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl MpxjEnum for BackgroundPattern {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.get_value()
    }
}

impl From<BackgroundPattern> for i32 {
/// A Rust-specific convenience conversion, equivalent to [`BackgroundPattern::get_value`].
    fn from(value: BackgroundPattern) -> i32 {
        value.get_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// The discriminant value is Java's int value, and `getInstance` round-trips consistently.
    #[test]
    fn round_trips_through_int_values() {
        for (index, item) in BackgroundPattern::ALL.iter().enumerate() {
            assert_eq!(item.get_value(), index as i32);
            assert_eq!(BackgroundPattern::get_instance(index as i32), *item);
            assert_eq!(MpxjEnum::value(item), index as i32);
        }
    }

/// Out of bounds falls back to TRANSPARENT (Java never returns null).
    #[test]
    fn out_of_range_falls_back_to_transparent() {
        for value in [-1, -100, 14, 15, 1000, i32::MAX, i32::MIN] {
            assert_eq!(
                BackgroundPattern::get_instance(value),
                BackgroundPattern::Transparent
            );
        }
    }

/// The name and `toString()` match Java.
    #[test]
    fn names_and_to_string_match_java() {
        assert_eq!(BackgroundPattern::Transparent.get_name(), "Transparent");
        assert_eq!(BackgroundPattern::Solid.get_name(), "Solid");
        assert_eq!(
            BackgroundPattern::Inversebackslash.to_string(),
            "Inverse Back Slash"
        );
        assert_eq!(
            BackgroundPattern::Denseforwardslash.to_string(),
            "Dense Forward Slash"
        );
        assert_eq!(
            BackgroundPattern::Inversecheckered.to_string(),
            "Inverse Checkered"
        );
    }
}
