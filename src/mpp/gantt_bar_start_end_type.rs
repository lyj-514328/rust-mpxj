//! 对应 Java: `org/mpxj/mpp/GanttBarStartEndType.java`
//!
//! 甘特图条形「起点 / 终点」区域的样式（实心 / 框架 / 虚线）。
//!
//! # 移植取舍
//!
//! - Java 用 `EnumHelper.createTypeArray(GanttBarStartEndType.class)` 反射生成
//!   int → 枚举的查找表（数组长度 = 常量个数 3，取值 0..2 恰好填满，
//!   `getInstance` 不会返回 `null`）；Rust 不用反射，改为
//!   [`GanttBarStartEndType::ALL`] + [`GanttBarStartEndType::get_instance`] 的 `match`。
//! - 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
//!
//! 未移植的 Java 成员：无。

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// 甘特图条形起止区域的样式。
///
/// 对应 Java: `public enum GanttBarStartEndType implements MpxjEnum`
///
/// 声明顺序与 Java 一致，判别值即 Java 的 `m_value`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GanttBarStartEndType {
    /// 对应 Java: `SOLID(0, "Solid")`
    Solid = 0,
    /// 对应 Java: `FRAMED(1, "Framed")`
    Framed = 1,
    /// 对应 Java: `DASHED(2, "Dashed")`
    Dashed = 2,
}

impl GanttBarStartEndType {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `private static final GanttBarStartEndType[] TYPE_VALUES`（去掉反射建表后）。
    pub const ALL: [GanttBarStartEndType; 3] = [
        GanttBarStartEndType::Solid,
        GanttBarStartEndType::Framed,
        GanttBarStartEndType::Dashed,
    ];

    /// 根据 int 值取枚举实例，越界（含负值）时回落到 `SOLID`。
    ///
    /// 对应 Java: `public static GanttBarStartEndType getInstance(int type)`
    pub fn get_instance(value: i32) -> GanttBarStartEndType {
        match value {
            0 => GanttBarStartEndType::Solid,
            1 => GanttBarStartEndType::Framed,
            2 => GanttBarStartEndType::Dashed,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = SOLID.getValue(); }`
            _ => GanttBarStartEndType::Solid,
        }
    }

    /// 对应 Java: `public static GanttBarStartEndType getInstance(Number type)`
    ///
    /// Java 侧 `null` 走 `getInstance(-1)`，最终回落到 `SOLID`。
    pub fn get_instance_optional(value: Option<i32>) -> GanttBarStartEndType {
        GanttBarStartEndType::get_instance(value.unwrap_or(-1))
    }

    /// 取样式名称（如 `"Framed"`），与 Java 一致，未做本地化。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            GanttBarStartEndType::Solid => "Solid",
            GanttBarStartEndType::Framed => "Framed",
            GanttBarStartEndType::Dashed => "Dashed",
        }
    }
}

impl MpxjEnum for GanttBarStartEndType {
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for GanttBarStartEndType {
    /// 对应 Java: `@Override public String toString()`，即 `getName()`。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl From<GanttBarStartEndType> for i32 {
    fn from(value: GanttBarStartEndType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_get_instance() {
        for item in GanttBarStartEndType::ALL {
            assert_eq!(GanttBarStartEndType::get_instance(item.value()), item);
        }
    }

    #[test]
    fn out_of_range_falls_back_to_solid() {
        assert_eq!(GanttBarStartEndType::get_instance(-1), GanttBarStartEndType::Solid);
        assert_eq!(GanttBarStartEndType::get_instance(3), GanttBarStartEndType::Solid);
        assert_eq!(
            GanttBarStartEndType::get_instance_optional(None),
            GanttBarStartEndType::Solid
        );
        assert_eq!(
            GanttBarStartEndType::get_instance_optional(Some(2)),
            GanttBarStartEndType::Dashed
        );
    }

    #[test]
    fn names_match_java() {
        assert_eq!(GanttBarStartEndType::Solid.to_string(), "Solid");
        assert_eq!(GanttBarStartEndType::Dashed.get_name(), "Dashed");
    }
}
