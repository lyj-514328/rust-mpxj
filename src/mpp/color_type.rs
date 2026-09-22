//! Corresponds to Java: `org/mpxj/mpp/ColorType.java`
//!
//! Color enum used by Microsoft Project (note: this is MPXJ's own color enum,
//! not `java.awt.Color`).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `public enum ColorType implements MpxjEnum` | same-named `enum` + `impl crate::MpxjEnum` | constants are ordered by the Java declaration order and the discriminant is the Java int value |
//! | constructor parameter `java.awt.Color color` / `Color getColor()` | [`crate::mpp::mpp_utility::Color`] + `Option<Color>` | `java.awt.*` is a skipped item; Rust reuses `crate::mpp::mpp_utility::Color` (the return type of `MPPUtility.get_color`) and keeps only the R/G/B components. In Java the color of `AUTOMATIC` is `null`, so the getter returns `Option<Color>` |
//! | `private static final ColorType[] TYPE_VALUES = EnumHelper.createTypeArray(ColorType.class)` | `match` in [`ColorType::get_instance`] + [`ColorType::ALL`] | table length 17 (offset 0); values 0..16 are all populated, so Java's `getInstance` never returns `null` |
//! | `getInstance(Number type)` | [`ColorType::get_instance_optional`] | Rust has no overloading, so an `_optional` suffix is added |
//! | does not override `toString()` (inherits `java.lang.Enum.toString()`, which outputs the **constant name**) | `Display`, outputting `BLACK` / `RED` / … / `AUTOMATIC` | A Java enum has `toString()` even when it is not overridden, and its value is the constant name; note this differs from `getName()` (`"Black"` etc.) |
//!
//! Skipped Java members: reflection-based table creation of `EnumHelper.createTypeArray` (replaced on the Rust side by `match` + `ALL`).
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` (including `java.awt.*`) / `Serializable`:
//! apart from the color constants themselves, this class does not use these mechanisms.

use std::fmt::{self, Display, Formatter};

use crate::mpp::mpp_utility::Color;
use crate::mpxj_enum::MpxjEnum;

/// Color used by Microsoft Project.
///
/// Corresponds to Java: `public enum ColorType`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorType {
    /// Corresponds to Java: `BLACK(0, "Black", Color.BLACK)`
    Black = 0,
    /// Corresponds to Java: `RED(1, "Red", Color.RED)`
    Red = 1,
    /// Corresponds to Java: `YELLOW(2, "Yellow", Color.YELLOW)`
    Yellow = 2,
    /// Corresponds to Java: `LIME(3, "Lime", new Color(148, 255, 148))`
    Lime = 3,
    /// Corresponds to Java: `AQUA(4, "Aqua", new Color(194, 220, 255))`
    Aqua = 4,
    /// Corresponds to Java: `BLUE(5, "Blue", Color.BLUE)`
    Blue = 5,
    /// Corresponds to Java: `FUSCHIA(6, "Fuschia", Color.MAGENTA)`
    Fuschia = 6,
    /// Corresponds to Java: `WHITE(7, "White", Color.WHITE)`
    White = 7,
    /// Corresponds to Java: `MAROON(8, "Maroon", new Color(128, 0, 0))`
    Maroon = 8,
    /// Corresponds to Java: `GREEN(9, "Green", new Color(0, 128, 0))`
    Green = 9,
    /// Corresponds to Java: `OLIVE(10, "Olive", new Color(128, 128, 0))`
    Olive = 10,
    /// Corresponds to Java: `NAVY(11, "Navy", new Color(0, 0, 128))`
    Navy = 11,
    /// Corresponds to Java: `PURPLE(12, "Purple", new Color(128, 0, 128))`
    Purple = 12,
    /// Corresponds to Java: `TEAL(13, "Teal", new Color(0, 128, 128))`
    Teal = 13,
    /// Corresponds to Java: `GRAY(14, "Gray", new Color(128, 128, 128))`
    Gray = 14,
    /// Corresponds to Java: `SILVER(15, "Silver", new Color(192, 192, 192))`
    Silver = 15,
    /// Corresponds to Java: `AUTOMATIC(16, "Automatic", null)`
    Automatic = 16,
}

impl ColorType {
/// All values, in the same order as the Java declarations.
    ///
/// Corresponds to Java: `private static final ColorType[] TYPE_VALUES` (after removing the reflection-based table creation).
    pub const ALL: [ColorType; 17] = [
        ColorType::Black,
        ColorType::Red,
        ColorType::Yellow,
        ColorType::Lime,
        ColorType::Aqua,
        ColorType::Blue,
        ColorType::Fuschia,
        ColorType::White,
        ColorType::Maroon,
        ColorType::Green,
        ColorType::Olive,
        ColorType::Navy,
        ColorType::Purple,
        ColorType::Teal,
        ColorType::Gray,
        ColorType::Silver,
        ColorType::Automatic,
    ];

/// Gets the enum instance by int value.
    ///
    /// Corresponds to Java: `public static ColorType getInstance(int type)`
    ///
/// Java's lookup table length is 17 (`createTypeArray` offset 0) and 0..16 are all populated,
/// so on out of bounds it first falls back to `AUTOMATIC.getValue()` (16) and then indexes the table; the method therefore never returns `null`.
    pub fn get_instance(value: i32) -> ColorType {
        match value {
            0 => ColorType::Black,
            1 => ColorType::Red,
            2 => ColorType::Yellow,
            3 => ColorType::Lime,
            4 => ColorType::Aqua,
            5 => ColorType::Blue,
            6 => ColorType::Fuschia,
            7 => ColorType::White,
            8 => ColorType::Maroon,
            9 => ColorType::Green,
            10 => ColorType::Olive,
            11 => ColorType::Navy,
            12 => ColorType::Purple,
            13 => ColorType::Teal,
            14 => ColorType::Gray,
            15 => ColorType::Silver,
            // Java: type < 0 || type >= TYPE_VALUES.length → type = AUTOMATIC.getValue()
            16 => ColorType::Automatic,
            _ => ColorType::Automatic,
        }
    }

    /// Corresponds to Java: `public static ColorType getInstance(Number type)`
    ///
/// On the Java side `null` goes through `getInstance(-1)` and finally falls back to `AUTOMATIC`.
    pub fn get_instance_optional(value: Option<i32>) -> ColorType {
        ColorType::get_instance(value.unwrap_or(-1))
    }

/// Gets the color name (not localized).
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            ColorType::Black => "Black",
            ColorType::Red => "Red",
            ColorType::Yellow => "Yellow",
            ColorType::Lime => "Lime",
            ColorType::Aqua => "Aqua",
            ColorType::Blue => "Blue",
            ColorType::Fuschia => "Fuschia",
            ColorType::White => "White",
            ColorType::Maroon => "Maroon",
            ColorType::Green => "Green",
            ColorType::Olive => "Olive",
            ColorType::Navy => "Navy",
            ColorType::Purple => "Purple",
            ColorType::Teal => "Teal",
            ColorType::Gray => "Gray",
            ColorType::Silver => "Silver",
            ColorType::Automatic => "Automatic",
        }
    }

/// Gets the color value corresponding to this color in MS Project; `AUTOMATIC` returns `None`.
    ///
/// Corresponds to Java: `public Color getColor()` (Java comment: returns `null` for `AUTOMATIC`)
    pub fn get_color(&self) -> Option<Color> {
        match self {
            ColorType::Black => Some(Color::new(0, 0, 0)),
            ColorType::Red => Some(Color::new(255, 0, 0)),
            ColorType::Yellow => Some(Color::new(255, 255, 0)),
            ColorType::Lime => Some(Color::new(148, 255, 148)),
            ColorType::Aqua => Some(Color::new(194, 220, 255)),
            ColorType::Blue => Some(Color::new(0, 0, 255)),
            // Java: Color.MAGENTA
            ColorType::Fuschia => Some(Color::new(255, 0, 255)),
            ColorType::White => Some(Color::new(255, 255, 255)),
            ColorType::Maroon => Some(Color::new(128, 0, 0)),
            ColorType::Green => Some(Color::new(0, 128, 0)),
            ColorType::Olive => Some(Color::new(128, 128, 0)),
            ColorType::Navy => Some(Color::new(0, 0, 128)),
            ColorType::Purple => Some(Color::new(128, 0, 128)),
            ColorType::Teal => Some(Color::new(0, 128, 128)),
            ColorType::Gray => Some(Color::new(128, 128, 128)),
            ColorType::Silver => Some(Color::new(192, 192, 192)),
            // Java: m_color == null
            ColorType::Automatic => None,
        }
    }

    /// Corresponds to Java: `@Override public int getValue()`
    pub fn get_value(&self) -> i32 {
        *self as i32
    }
}

impl Display for ColorType {
/// Corresponds to Java: `toString()` inherited from `java.lang.Enum`
    ///
/// Java's `Enum.toString()` returns the constant name (e.g. `AUTOMATIC`); this class does not override it, so the Rust side
/// outputs the same constant name (different from `"Automatic"` of [`ColorType::get_name`]).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ColorType::Black => "BLACK",
            ColorType::Red => "RED",
            ColorType::Yellow => "YELLOW",
            ColorType::Lime => "LIME",
            ColorType::Aqua => "AQUA",
            ColorType::Blue => "BLUE",
            ColorType::Fuschia => "FUSCHIA",
            ColorType::White => "WHITE",
            ColorType::Maroon => "MAROON",
            ColorType::Green => "GREEN",
            ColorType::Olive => "OLIVE",
            ColorType::Navy => "NAVY",
            ColorType::Purple => "PURPLE",
            ColorType::Teal => "TEAL",
            ColorType::Gray => "GRAY",
            ColorType::Silver => "SILVER",
            ColorType::Automatic => "AUTOMATIC",
        })
    }
}

impl MpxjEnum for ColorType {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.get_value()
    }
}

impl From<ColorType> for i32 {
/// Rust-specific convenience conversion, equivalent to [`ColorType::get_value`].
    fn from(value: ColorType) -> i32 {
        value.get_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// The discriminant is the Java int value, and `getInstance` round-trips consistently.
    #[test]
    fn round_trips_through_int_values() {
        for (index, item) in ColorType::ALL.iter().enumerate() {
            assert_eq!(item.get_value(), index as i32);
            assert_eq!(ColorType::get_instance(index as i32), *item);
            assert_eq!(MpxjEnum::value(item), index as i32);
        }
    }

/// Out of bounds falls back to AUTOMATIC (Java never returns null).
    #[test]
    fn out_of_range_falls_back_to_automatic() {
        for value in [-1, -100, 17, 18, 1000, i32::MAX, i32::MIN] {
            assert_eq!(ColorType::get_instance(value), ColorType::Automatic);
        }
        assert_eq!(
            ColorType::get_instance_optional(None),
            ColorType::Automatic
        );
        assert_eq!(
            ColorType::get_instance_optional(Some(3)),
            ColorType::Lime
        );
    }

/// Name matches Java.
    #[test]
    fn names_match_java() {
        assert_eq!(ColorType::Black.get_name(), "Black");
        assert_eq!(ColorType::Fuschia.get_name(), "Fuschia");
        assert_eq!(ColorType::Automatic.get_name(), "Automatic");
        for item in ColorType::ALL {
            assert!(!item.get_name().is_empty());
        }
    }

/// `toString()` (Java's `Enum.toString()`) outputs the constant name, unlike `getName()`.
    #[test]
    fn to_string_outputs_the_java_constant_name() {
        assert_eq!(ColorType::Black.to_string(), "BLACK");
        assert_eq!(ColorType::Automatic.to_string(), "AUTOMATIC");
        assert_eq!(ColorType::Fuschia.to_string(), "FUSCHIA");
        assert_ne!(ColorType::Automatic.to_string(), ColorType::Automatic.get_name());
    }

/// Color components match the values of Java's `java.awt.Color`; AUTOMATIC is null.
    #[test]
    fn colors_match_java_awt_values() {
        assert_eq!(ColorType::Black.get_color(), Some(Color::new(0, 0, 0)));
        assert_eq!(ColorType::Red.get_color(), Some(Color::new(255, 0, 0)));
        assert_eq!(ColorType::Yellow.get_color(), Some(Color::new(255, 255, 0)));
        // Java: new Color(148, 255, 148)
        assert_eq!(ColorType::Lime.get_color(), Some(Color::new(148, 255, 148)));
        // Java: new Color(194, 220, 255)
        assert_eq!(ColorType::Aqua.get_color(), Some(Color::new(194, 220, 255)));
        assert_eq!(ColorType::Blue.get_color(), Some(Color::new(0, 0, 255)));
        // Java: Color.MAGENTA
        assert_eq!(
            ColorType::Fuschia.get_color(),
            Some(Color::new(255, 0, 255))
        );
        assert_eq!(ColorType::White.get_color(), Some(Color::new(255, 255, 255)));
        assert_eq!(ColorType::Maroon.get_color(), Some(Color::new(128, 0, 0)));
        assert_eq!(ColorType::Green.get_color(), Some(Color::new(0, 128, 0)));
        assert_eq!(ColorType::Olive.get_color(), Some(Color::new(128, 128, 0)));
        assert_eq!(ColorType::Navy.get_color(), Some(Color::new(0, 0, 128)));
        assert_eq!(ColorType::Purple.get_color(), Some(Color::new(128, 0, 128)));
        assert_eq!(ColorType::Teal.get_color(), Some(Color::new(0, 128, 128)));
        assert_eq!(ColorType::Gray.get_color(), Some(Color::new(128, 128, 128)));
        assert_eq!(
            ColorType::Silver.get_color(),
            Some(Color::new(192, 192, 192))
        );

// Java: the color of AUTOMATIC is null
        assert_eq!(ColorType::Automatic.get_color(), None);

// all except AUTOMATIC have a color
        assert_eq!(
            ColorType::ALL
                .iter()
                .filter(|item| item.get_color().is_none())
                .count(),
            1
        );
    }
}
