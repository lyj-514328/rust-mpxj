//! 对应 Java: `org/mpxj/mpp/GanttBarStartEndShape.java`
//!
//! 甘特图条形「起点 / 终点」处绘制的形状。
//!
//! # 移植取舍
//!
//! - Java 用 `EnumHelper.createTypeArray(GanttBarStartEndShape.class)` 反射生成
//!   int → 枚举的查找表（数组长度 = 常量个数 25，取值 0..24 恰好填满，
//!   `getInstance` 不会返回 `null`）；Rust 不用反射，改为
//!   [`GanttBarStartEndShape::ALL`] + [`GanttBarStartEndShape::get_instance`] 的 `match`。
//! - 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
//!
//! 未移植的 Java 成员：无。

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// 甘特图条形起止端的形状。
///
/// 对应 Java: `public enum GanttBarStartEndShape implements MpxjEnum`
///
/// 声明顺序与 Java 一致，判别值即 Java 的 `m_value`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GanttBarStartEndShape {
    /// 对应 Java: `NONE(0, "None")`
    None = 0,
    /// 对应 Java: `NORTHHOMEPLATE(1, "North Home Plate")`
    NorthHomePlate = 1,
    /// 对应 Java: `SOUTHHOMEPLATE(2, "South Home Plate")`
    SouthHomePlate = 2,
    /// 对应 Java: `DIAMOND(3, "Diamond")`
    Diamond = 3,
    /// 对应 Java: `UPARROW(4, "Up Arrow")`
    UpArrow = 4,
    /// 对应 Java: `DOWNARROW(5, "Down Arrow")`
    DownArrow = 5,
    /// 对应 Java: `RIGHTARROW(6, "Right Arrow")`
    RightArrow = 6,
    /// 对应 Java: `LEFTARROW(7, "Left Arrow")`
    LeftArrow = 7,
    /// 对应 Java: `UPPOINTER(8, "Up Pointer")`
    UpPointer = 8,
    /// 对应 Java: `SOUTHMINIHOMEPLATE(9, "South Mini Home Plate")`
    SouthMiniHomePlate = 9,
    /// 对应 Java: `NORTHMINIHOMEPLATE(10, "North Mini Home Plate")`
    NorthMiniHomePlate = 10,
    /// 对应 Java: `VERTICALBAR(11, "Vertical Bar")`
    VerticalBar = 11,
    /// 对应 Java: `SQUARE(12, "Square")`
    Square = 12,
    /// 对应 Java: `DIAMONDCIRCLED(13, "Diamond Circled")`
    DiamondCircled = 13,
    /// 对应 Java: `DOWNPOINTER(14, "Down Pointer")`
    DownPointer = 14,
    /// 对应 Java: `UPARROWCIRCLED(15, "Up Arrow Circled")`
    UpArrowCircled = 15,
    /// 对应 Java: `DOWNARROWCIRCLED(16, "Down Arrow Circled")`
    DownArrowCircled = 16,
    /// 对应 Java: `UPPOINTERCIRCLED(17, "Up Pointer Circled")`
    UpPointerCircled = 17,
    /// 对应 Java: `DOWNPOINTERCIRCLED(18, "Down Pointer Circled")`
    DownPointerCircled = 18,
    /// 对应 Java: `CIRCLE(19, "Circle")`
    Circle = 19,
    /// 对应 Java: `STAR(20, "Star")`
    Star = 20,
    /// 对应 Java: `LEFTBRACKET(21, "Left Bracket")`
    LeftBracket = 21,
    /// 对应 Java: `RIGHTBRACKET(22, "Right Bracket")`
    RightBracket = 22,
    /// 对应 Java: `LEFTGRADIENT(23, "Left Gradient")`
    LeftGradient = 23,
    /// 对应 Java: `RIGHTGRADIENT(24, "Right Gradient")`
    RightGradient = 24,
}

impl GanttBarStartEndShape {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `private static final GanttBarStartEndShape[] TYPE_VALUES`
    /// （去掉反射建表后剩下的常量清单）。
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

    /// 根据 int 值取枚举实例，越界（含负值）时回落到 `NONE`。
    ///
    /// 对应 Java: `public static GanttBarStartEndShape getInstance(int type)`
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

    /// 对应 Java: `public static GanttBarStartEndShape getInstance(Number type)`
    ///
    /// Java 侧 `null` 走 `getInstance(-1)`，最终回落到 `NONE`。
    pub fn get_instance_optional(value: Option<i32>) -> GanttBarStartEndShape {
        GanttBarStartEndShape::get_instance(value.unwrap_or(-1))
    }

    /// 取形状名称（如 `"North Home Plate"`），与 Java 一致，未做本地化。
    ///
    /// 对应 Java: `public String getName()`
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
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for GanttBarStartEndShape {
    /// 对应 Java: `@Override public String toString()`，即 `getName()`。
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
        // Java 的 `getInstance(Number)` 对 null 走 -1
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
