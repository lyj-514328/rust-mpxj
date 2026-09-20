//! 对应 Java: `org/mpxj/mpp/GanttBarMiddleShape.java`
//!
//! 甘特图条形「中段」的绘制形状。
//!
//! # 移植取舍
//!
//! - Java 用 `EnumHelper.createTypeArray(GanttBarMiddleShape.class)` 反射生成
//!   int → 枚举的查找表（数组长度 = 常量个数 8，取值 0..7 恰好填满，
//!   `getInstance` 不会返回 `null`）；Rust 不用反射，改为
//!   [`GanttBarMiddleShape::ALL`] + [`GanttBarMiddleShape::get_instance`] 的 `match`。
//! - 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
//!
//! 未移植的 Java 成员：无。

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// 甘特图条形中段的形状。
///
/// 对应 Java: `public enum GanttBarMiddleShape implements MpxjEnum`
///
/// 声明顺序与 Java 一致，判别值即 Java 的 `m_value`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GanttBarMiddleShape {
    /// 对应 Java: `NONE(0, "None")`
    None = 0,
    /// 对应 Java: `ROUNDED(1, "Rounded")`
    Rounded = 1,
    /// 对应 Java: `TOPTHINROUNDED(2, "Top Thin Rounded")`
    TopThinRounded = 2,
    /// 对应 Java: `MIDDLETHINROUNDED(3, "Middle Thin Rounded")`
    MiddleThinRounded = 3,
    /// 对应 Java: `BOTTOMTHINROUNDED(4, "Bottom Thin Rounded")`
    BottomThinRounded = 4,
    /// 对应 Java: `TOPLINE(5, "Top Line")`
    TopLine = 5,
    /// 对应 Java: `MIDDLELINE(6, "Middle Line")`
    MiddleLine = 6,
    /// 对应 Java: `BOTTOMLINE(7, "Bottom Line")`
    BottomLine = 7,
}

impl GanttBarMiddleShape {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `private static final GanttBarMiddleShape[] TYPE_VALUES`（去掉反射建表后）。
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

    /// 根据 int 值取枚举实例，越界（含负值）时回落到 `NONE`。
    ///
    /// 对应 Java: `public static GanttBarMiddleShape getInstance(int type)`
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

    /// 对应 Java: `public static GanttBarMiddleShape getInstance(Number type)`
    ///
    /// Java 侧 `null` 走 `getInstance(-1)`，最终回落到 `NONE`。
    pub fn get_instance_optional(value: Option<i32>) -> GanttBarMiddleShape {
        GanttBarMiddleShape::get_instance(value.unwrap_or(-1))
    }

    /// 取形状名称（如 `"Top Thin Rounded"`），与 Java 一致，未做本地化。
    ///
    /// 对应 Java: `public String getName()`
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
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for GanttBarMiddleShape {
    /// 对应 Java: `@Override public String toString()`，即 `getName()`。
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
