//! Corresponds to Java: `org/mpxj/mpp/GanttBarStartEndShape.java`
//!
//! Shapes drawn at the start / end of Gantt chart bars.
//!
//! # Porting trade-offs
//!
//! - Java uses reflection via `EnumHelper.createTypeArray(GanttBarStartEndShape.class)` to generate
//! the int → enum lookup table (array length = constant count 25; values 0..24 exactly fill it,
//! so `getInstance` never returns `null`); Rust does not use reflection and instead uses
//! [`GanttBarStartEndShape::ALL`] + the `match` in [`GanttBarStartEndShape::get_instance`].
//! - Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use them.
//!
//! Unported Java members: none.

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// Shape of the start/end of Gantt chart bars.
///
/// Corresponds to Java: `public enum GanttBarStartEndShape implements MpxjEnum`
///
/// The declaration order matches Java, and the discriminant value is Java's `m_value`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GanttBarStartEndShape {
    /// Corresponds to Java: `NONE(0, "None")`
    None = 0,
    /// Corresponds to Java: `NORTHHOMEPLATE(1, "North Home Plate")`
    NorthHomePlate = 1,
    /// Corresponds to Java: `SOUTHHOMEPLATE(2, "South Home Plate")`
    SouthHomePlate = 2,
    /// Corresponds to Java: `DIAMOND(3, "Diamond")`
    Diamond = 3,
    /// Corresponds to Java: `UPARROW(4, "Up Arrow")`
    UpArrow = 4,
    /// Corresponds to Java: `DOWNARROW(5, "Down Arrow")`
    DownArrow = 5,
    /// Corresponds to Java: `RIGHTARROW(6, "Right Arrow")`
    RightArrow = 6,
    /// Corresponds to Java: `LEFTARROW(7, "Left Arrow")`
    LeftArrow = 7,
    /// Corresponds to Java: `UPPOINTER(8, "Up Pointer")`
    UpPointer = 8,
    /// Corresponds to Java: `SOUTHMINIHOMEPLATE(9, "South Mini Home Plate")`
    SouthMiniHomePlate = 9,
    /// Corresponds to Java: `NORTHMINIHOMEPLATE(10, "North Mini Home Plate")`
    NorthMiniHomePlate = 10,
    /// Corresponds to Java: `VERTICALBAR(11, "Vertical Bar")`
    VerticalBar = 11,
    /// Corresponds to Java: `SQUARE(12, "Square")`
    Square = 12,
    /// Corresponds to Java: `DIAMONDCIRCLED(13, "Diamond Circled")`
    DiamondCircled = 13,
    /// Corresponds to Java: `DOWNPOINTER(14, "Down Pointer")`
    DownPointer = 14,
    /// Corresponds to Java: `UPARROWCIRCLED(15, "Up Arrow Circled")`
    UpArrowCircled = 15,
    /// Corresponds to Java: `DOWNARROWCIRCLED(16, "Down Arrow Circled")`
    DownArrowCircled = 16,
    /// Corresponds to Java: `UPPOINTERCIRCLED(17, "Up Pointer Circled")`
    UpPointerCircled = 17,
    /// Corresponds to Java: `DOWNPOINTERCIRCLED(18, "Down Pointer Circled")`
    DownPointerCircled = 18,
    /// Corresponds to Java: `CIRCLE(19, "Circle")`
    Circle = 19,
    /// Corresponds to Java: `STAR(20, "Star")`
    Star = 20,
    /// Corresponds to Java: `LEFTBRACKET(21, "Left Bracket")`
    LeftBracket = 21,
    /// Corresponds to Java: `RIGHTBRACKET(22, "Right Bracket")`
    RightBracket = 22,
    /// Corresponds to Java: `LEFTGRADIENT(23, "Left Gradient")`
    LeftGradient = 23,
    /// Corresponds to Java: `RIGHTGRADIENT(24, "Right Gradient")`
    RightGradient = 24,
}

impl GanttBarStartEndShape {
/// All values, in the same order as declared in Java.
    ///
    /// Corresponds to Java: `private static final GanttBarStartEndShape[] TYPE_VALUES`
/// (the list of constants left after removing the reflective table construction).
    pub const ALL: [GanttBarStartEndShape; 25] = [
        GanttBarStartEndShape::None,
        GanttBarStartEndShape::NorthHomePlate,
        GanttBarStartEndShape::SouthHomePlate,
        GanttBarStartEndShape::Diamond,
        GanttBarStartEndShape::UpArrow,
        GanttBarStartEndShape::DownArrow,
        GanttBarStartEndShape::RightArrow,
        GanttBarStartEndShape::LeftArrow,
        GanttBarStartEndShape::UpPointer,
        GanttBarStartEndShape::SouthMiniHomePlate,
        GanttBarStartEndShape::NorthMiniHomePlate,
        GanttBarStartEndShape::VerticalBar,
        GanttBarStartEndShape::Square,
        GanttBarStartEndShape::DiamondCircled,
        GanttBarStartEndShape::DownPointer,
        GanttBarStartEndShape::UpArrowCircled,
        GanttBarStartEndShape::DownArrowCircled,
        GanttBarStartEndShape::UpPointerCircled,
        GanttBarStartEndShape::DownPointerCircled,
        GanttBarStartEndShape::Circle,
        GanttBarStartEndShape::Star,
        GanttBarStartEndShape::LeftBracket,
        GanttBarStartEndShape::RightBracket,
        GanttBarStartEndShape::LeftGradient,
        GanttBarStartEndShape::RightGradient,
    ];

/// Get the enum instance by int value; on out of bounds (including negative values) it falls back to `NONE`.
    ///
    /// Corresponds to Java: `public static GanttBarStartEndShape getInstance(int type)`
    pub fn get_instance(value: i32) -> GanttBarStartEndShape {
        match value {
            0 => GanttBarStartEndShape::None,
            1 => GanttBarStartEndShape::NorthHomePlate,
            2 => GanttBarStartEndShape::SouthHomePlate,
            3 => GanttBarStartEndShape::Diamond,
            4 => GanttBarStartEndShape::UpArrow,
            5 => GanttBarStartEndShape::DownArrow,
            6 => GanttBarStartEndShape::RightArrow,
            7 => GanttBarStartEndShape::LeftArrow,
            8 => GanttBarStartEndShape::UpPointer,
            9 => GanttBarStartEndShape::SouthMiniHomePlate,
            10 => GanttBarStartEndShape::NorthMiniHomePlate,
            11 => GanttBarStartEndShape::VerticalBar,
            12 => GanttBarStartEndShape::Square,
            13 => GanttBarStartEndShape::DiamondCircled,
            14 => GanttBarStartEndShape::DownPointer,
            15 => GanttBarStartEndShape::UpArrowCircled,
            16 => GanttBarStartEndShape::DownArrowCircled,
            17 => GanttBarStartEndShape::UpPointerCircled,
            18 => GanttBarStartEndShape::DownPointerCircled,
            19 => GanttBarStartEndShape::Circle,
            20 => GanttBarStartEndShape::Star,
            21 => GanttBarStartEndShape::LeftBracket,
            22 => GanttBarStartEndShape::RightBracket,
            23 => GanttBarStartEndShape::LeftGradient,
            24 => GanttBarStartEndShape::RightGradient,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = NONE.getValue(); }`
            _ => GanttBarStartEndShape::None,
        }
    }

    /// Corresponds to Java: `public static GanttBarStartEndShape getInstance(Number type)`
    ///
/// On the Java side `null` goes through `getInstance(-1)`, which ultimately falls back to `NONE`.
    pub fn get_instance_optional(value: Option<i32>) -> GanttBarStartEndShape {
        GanttBarStartEndShape::get_instance(value.unwrap_or(-1))
    }

/// Get the shape name (such as `"North Home Plate"`), matching Java, not localized.
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            GanttBarStartEndShape::None => "None",
            GanttBarStartEndShape::NorthHomePlate => "North Home Plate",
            GanttBarStartEndShape::SouthHomePlate => "South Home Plate",
            GanttBarStartEndShape::Diamond => "Diamond",
            GanttBarStartEndShape::UpArrow => "Up Arrow",
            GanttBarStartEndShape::DownArrow => "Down Arrow",
            GanttBarStartEndShape::RightArrow => "Right Arrow",
            GanttBarStartEndShape::LeftArrow => "Left Arrow",
            GanttBarStartEndShape::UpPointer => "Up Pointer",
            GanttBarStartEndShape::SouthMiniHomePlate => "South Mini Home Plate",
            GanttBarStartEndShape::NorthMiniHomePlate => "North Mini Home Plate",
            GanttBarStartEndShape::VerticalBar => "Vertical Bar",
            GanttBarStartEndShape::Square => "Square",
            GanttBarStartEndShape::DiamondCircled => "Diamond Circled",
            GanttBarStartEndShape::DownPointer => "Down Pointer",
            GanttBarStartEndShape::UpArrowCircled => "Up Arrow Circled",
            GanttBarStartEndShape::DownArrowCircled => "Down Arrow Circled",
            GanttBarStartEndShape::UpPointerCircled => "Up Pointer Circled",
            GanttBarStartEndShape::DownPointerCircled => "Down Pointer Circled",
            GanttBarStartEndShape::Circle => "Circle",
            GanttBarStartEndShape::Star => "Star",
            GanttBarStartEndShape::LeftBracket => "Left Bracket",
            GanttBarStartEndShape::RightBracket => "Right Bracket",
            GanttBarStartEndShape::LeftGradient => "Left Gradient",
            GanttBarStartEndShape::RightGradient => "Right Gradient",
        }
    }
}

impl MpxjEnum for GanttBarStartEndShape {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for GanttBarStartEndShape {
/// Corresponds to Java: `@Override public String toString()`, i.e. `getName()`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl From<GanttBarStartEndShape> for i32 {
    fn from(value: GanttBarStartEndShape) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_get_instance() {
        for item in GanttBarStartEndShape::ALL {
            assert_eq!(GanttBarStartEndShape::get_instance(item.value()), item);
        }
    }

    #[test]
    fn out_of_range_falls_back_to_none() {
        assert_eq!(GanttBarStartEndShape::get_instance(-1), GanttBarStartEndShape::None);
        assert_eq!(GanttBarStartEndShape::get_instance(25), GanttBarStartEndShape::None);
        assert_eq!(GanttBarStartEndShape::get_instance(i32::MIN), GanttBarStartEndShape::None);
        assert_eq!(
            GanttBarStartEndShape::get_instance_optional(None),
            GanttBarStartEndShape::None
        );
// Java's `getInstance(Number)` routes null through -1
        assert_eq!(
            GanttBarStartEndShape::get_instance_optional(Some(3)),
            GanttBarStartEndShape::Diamond
        );
    }

    #[test]
    fn names_match_java() {
        assert_eq!(GanttBarStartEndShape::None.to_string(), "None");
        assert_eq!(
            GanttBarStartEndShape::SouthMiniHomePlate.to_string(),
            "South Mini Home Plate"
        );
        assert_eq!(
            GanttBarStartEndShape::RightGradient.get_name(),
            "Right Gradient"
        );
    }
}
