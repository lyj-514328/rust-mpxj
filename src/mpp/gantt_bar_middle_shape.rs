//! Corresponds to Java: `org/mpxj/mpp/GanttBarMiddleShape.java`
//!
//! Drawing shape of the "middle" segment of a Gantt bar.
//!
//! # Porting trade-offs
//!
//! - Java uses `EnumHelper.createTypeArray(GanttBarMiddleShape.class)` to generate by reflection
//! an int → enum lookup table (the array length equals the number of constants, 8, and the values 0..7 fill it exactly,
//! so `getInstance` never returns `null`); Rust does not use reflection and instead uses
//! a `match` over [`GanttBarMiddleShape::ALL`] + [`GanttBarMiddleShape::get_instance`].
//! - Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use them.
//!
//! Java members not ported: none.

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// Shape of the middle segment of a Gantt bar.
///
/// Corresponds to Java: `public enum GanttBarMiddleShape implements MpxjEnum`
///
/// The declaration order matches Java, and the discriminant is Java's `m_value`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GanttBarMiddleShape {
    /// Corresponds to Java: `NONE(0, "None")`
    None = 0,
    /// Corresponds to Java: `ROUNDED(1, "Rounded")`
    Rounded = 1,
    /// Corresponds to Java: `TOPTHINROUNDED(2, "Top Thin Rounded")`
    TopThinRounded = 2,
    /// Corresponds to Java: `MIDDLETHINROUNDED(3, "Middle Thin Rounded")`
    MiddleThinRounded = 3,
    /// Corresponds to Java: `BOTTOMTHINROUNDED(4, "Bottom Thin Rounded")`
    BottomThinRounded = 4,
    /// Corresponds to Java: `TOPLINE(5, "Top Line")`
    TopLine = 5,
    /// Corresponds to Java: `MIDDLELINE(6, "Middle Line")`
    MiddleLine = 6,
    /// Corresponds to Java: `BOTTOMLINE(7, "Bottom Line")`
    BottomLine = 7,
}

impl GanttBarMiddleShape {
/// All values, in the same order as Java's declaration order.
    ///
/// Corresponds to Java: `private static final GanttBarMiddleShape[] TYPE_VALUES` (after removing the reflection-built table).
    pub const ALL: [GanttBarMiddleShape; 8] = [
        GanttBarMiddleShape::None,
        GanttBarMiddleShape::Rounded,
        GanttBarMiddleShape::TopThinRounded,
        GanttBarMiddleShape::MiddleThinRounded,
        GanttBarMiddleShape::BottomThinRounded,
        GanttBarMiddleShape::TopLine,
        GanttBarMiddleShape::MiddleLine,
        GanttBarMiddleShape::BottomLine,
    ];

/// Return the enum instance for an int value; out-of-bounds values (including negatives) fall back to `NONE`.
    ///
    /// Corresponds to Java: `public static GanttBarMiddleShape getInstance(int type)`
    pub fn get_instance(value: i32) -> GanttBarMiddleShape {
        match value {
            0 => GanttBarMiddleShape::None,
            1 => GanttBarMiddleShape::Rounded,
            2 => GanttBarMiddleShape::TopThinRounded,
            3 => GanttBarMiddleShape::MiddleThinRounded,
            4 => GanttBarMiddleShape::BottomThinRounded,
            5 => GanttBarMiddleShape::TopLine,
            6 => GanttBarMiddleShape::MiddleLine,
            7 => GanttBarMiddleShape::BottomLine,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = NONE.getValue(); }`
            _ => GanttBarMiddleShape::None,
        }
    }

    /// Corresponds to Java: `public static GanttBarMiddleShape getInstance(Number type)`
    ///
/// In Java, `null` goes through `getInstance(-1)` and ultimately falls back to `NONE`.
    pub fn get_instance_optional(value: Option<i32>) -> GanttBarMiddleShape {
        GanttBarMiddleShape::get_instance(value.unwrap_or(-1))
    }

/// Return the shape name (for example `"Top Thin Rounded"`), matching Java; no localization is performed.
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            GanttBarMiddleShape::None => "None",
            GanttBarMiddleShape::Rounded => "Rounded",
            GanttBarMiddleShape::TopThinRounded => "Top Thin Rounded",
            GanttBarMiddleShape::MiddleThinRounded => "Middle Thin Rounded",
            GanttBarMiddleShape::BottomThinRounded => "Bottom Thin Rounded",
            GanttBarMiddleShape::TopLine => "Top Line",
            GanttBarMiddleShape::MiddleLine => "Middle Line",
            GanttBarMiddleShape::BottomLine => "Bottom Line",
        }
    }
}

impl MpxjEnum for GanttBarMiddleShape {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for GanttBarMiddleShape {
/// Corresponds to Java: `@Override public String toString()`, that is, `getName()`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl From<GanttBarMiddleShape> for i32 {
    fn from(value: GanttBarMiddleShape) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_get_instance() {
        for item in GanttBarMiddleShape::ALL {
            assert_eq!(GanttBarMiddleShape::get_instance(item.value()), item);
        }
    }

    #[test]
    fn out_of_range_falls_back_to_none() {
        assert_eq!(GanttBarMiddleShape::get_instance(-1), GanttBarMiddleShape::None);
        assert_eq!(GanttBarMiddleShape::get_instance(8), GanttBarMiddleShape::None);
        assert_eq!(
            GanttBarMiddleShape::get_instance_optional(None),
            GanttBarMiddleShape::None
        );
        assert_eq!(
            GanttBarMiddleShape::get_instance_optional(Some(7)),
            GanttBarMiddleShape::BottomLine
        );
    }

    #[test]
    fn names_match_java() {
        assert_eq!(GanttBarMiddleShape::Rounded.to_string(), "Rounded");
        assert_eq!(
            GanttBarMiddleShape::MiddleThinRounded.to_string(),
            "Middle Thin Rounded"
        );
    }
}
