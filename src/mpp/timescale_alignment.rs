//! 对应 Java: `org/mpxj/mpp/TimescaleAlignment.java`
//!
//! 甘特图时间刻度标签的对齐方式。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `public enum TimescaleAlignment implements MpxjEnum` | 同名 `enum` + `impl crate::MpxjEnum` | 常量按 Java 声明顺序排列，判别值即 Java 的 int 值 |
//! | `private static final TimescaleAlignment[] TYPE_VALUES = EnumHelper.createTypeArray(TimescaleAlignment.class)` | [`TimescaleAlignment::get_instance`] 的 `match` + [`TimescaleAlignment::ALL`] | 表长 3（偏移 0），取值 0..2 全部被填充，故 Java 的 `getInstance` 不会返回 `null` |
//! | `getInstance(Number type)` | [`TimescaleAlignment::get_instance_optional`] | Rust 无重载，加 `_optional` 后缀 |
//! | `getName()` / `toString()` | [`TimescaleAlignment::get_name`] / `Display` | 名称未本地化，与 Java 一致 |
//!
//! 跳过的 Java 成员：`EnumHelper.createTypeArray` 的反射建表（Rust 侧以 `match` + `ALL` 代替）。
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// 时间刻度标签的对齐方式。
///
/// 对应 Java: `public enum TimescaleAlignment`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimescaleAlignment {
    /// 对应 Java: `LEFT(0, "Left")`
    Left = 0,
    /// 对应 Java: `CENTER(1, "Center")`
    Center = 1,
    /// 对应 Java: `RIGHT(2, "Right")`
    Right = 2,
}

impl TimescaleAlignment {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `private static final TimescaleAlignment[] TYPE_VALUES`（去掉反射建表后）。
    pub const ALL: [TimescaleAlignment; 3] = [
        TimescaleAlignment::Left,
        TimescaleAlignment::Center,
        TimescaleAlignment::Right,
    ];

    /// 按 int 值取枚举实例。
    ///
    /// 对应 Java: `public static TimescaleAlignment getInstance(int type)`
    ///
    /// 越界时 Java 先回落到 `CENTER.getValue()`（1）再取表，故不会返回 `null`。
    pub fn get_instance(value: i32) -> TimescaleAlignment {
        match value {
            0 => TimescaleAlignment::Left,
            1 => TimescaleAlignment::Center,
            2 => TimescaleAlignment::Right,
            // Java: type < 0 || type >= TYPE_VALUES.length → type = CENTER.getValue()
            _ => TimescaleAlignment::Center,
        }
    }

    /// 对应 Java: `public static TimescaleAlignment getInstance(Number type)`
    ///
    /// Java 侧 `null` 会走 `getInstance(-1)`，最终回落到 `CENTER`。
    pub fn get_instance_optional(value: Option<i32>) -> TimescaleAlignment {
        TimescaleAlignment::get_instance(value.unwrap_or(-1))
    }

    /// 取名称（未做本地化）。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            TimescaleAlignment::Left => "Left",
            TimescaleAlignment::Center => "Center",
            TimescaleAlignment::Right => "Right",
        }
    }

    /// 对应 Java: `@Override public int getValue()`
    pub fn get_value(&self) -> i32 {
        *self as i32
    }
}

impl Display for TimescaleAlignment {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 返回 `getName()`，Rust 侧一致。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl MpxjEnum for TimescaleAlignment {
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.get_value()
    }
}

impl From<TimescaleAlignment> for i32 {
    /// Rust 特有的便捷转换，等价于 [`TimescaleAlignment::get_value`]。
    fn from(value: TimescaleAlignment) -> i32 {
        value.get_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 判别值即 Java 的 int 值，且 `getInstance` 往返一致。
    #[test]
    fn round_trips_through_int_values() {
        for (index, item) in TimescaleAlignment::ALL.iter().enumerate() {
            assert_eq!(item.get_value(), index as i32);
            assert_eq!(TimescaleAlignment::get_instance(index as i32), *item);
            assert_eq!(MpxjEnum::value(item), index as i32);
        }
    }

    /// 越界回落 CENTER（Java 不会返回 null）。
    #[test]
    fn out_of_range_falls_back_to_center() {
        for value in [-1, -100, 3, 4, 1000, i32::MAX, i32::MIN] {
            assert_eq!(
                TimescaleAlignment::get_instance(value),
                TimescaleAlignment::Center
            );
        }
        assert_eq!(
            TimescaleAlignment::get_instance_optional(None),
            TimescaleAlignment::Center
        );
        assert_eq!(
            TimescaleAlignment::get_instance_optional(Some(0)),
            TimescaleAlignment::Left
        );
    }

    /// 名称与 `toString()` 与 Java 一致。
    #[test]
    fn names_and_to_string_match_java() {
        assert_eq!(TimescaleAlignment::Left.get_name(), "Left");
        assert_eq!(TimescaleAlignment::Center.to_string(), "Center");
        assert_eq!(TimescaleAlignment::Right.to_string(), "Right");
    }
}
