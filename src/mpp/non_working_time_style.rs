//! 对应 Java: `org/mpxj/mpp/NonWorkingTimeStyle.java`
//!
//! 甘特图上非工作时间（non-working time）的绘制方式。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `public enum NonWorkingTimeStyle implements MpxjEnum` | 同名 `enum` + `impl crate::MpxjEnum` | 常量按 Java 声明顺序排列，判别值即 Java 的 int 值 |
//! | `private static final NonWorkingTimeStyle[] TYPE_VALUES = EnumHelper.createTypeArray(NonWorkingTimeStyle.class)` | [`NonWorkingTimeStyle::get_instance`] 的 `match` + [`NonWorkingTimeStyle::ALL`] | 表长 3（偏移 0），取值 0..2 全部被填充，故 Java 的 `getInstance` 不会返回 `null` |
//! | `getInstance(Number type)` | [`NonWorkingTimeStyle::get_instance_optional`] | Rust 无重载，加 `_optional` 后缀 |
//! | `getName()` / `toString()` | [`NonWorkingTimeStyle::get_name`] / `Display` | 名称未本地化，与 Java 一致 |
//!
//! 跳过的 Java 成员：`EnumHelper.createTypeArray` 的反射建表（Rust 侧以 `match` + `ALL` 代替）。
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// 非工作时间的绘制方式。
///
/// 对应 Java: `public enum NonWorkingTimeStyle`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NonWorkingTimeStyle {
    /// 对应 Java: `BEHIND(0, "Behind")`
    Behind = 0,
    /// 对应 Java: `IN_FRONT(1, "In Front")`
    InFront = 1,
    /// 对应 Java: `DO_NOT_DRAW(2, "Do Not Draw")`
    DoNotDraw = 2,
}

impl NonWorkingTimeStyle {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `private static final NonWorkingTimeStyle[] TYPE_VALUES`（去掉反射建表后）。
    pub const ALL: [NonWorkingTimeStyle; 3] = [
        NonWorkingTimeStyle::Behind,
        NonWorkingTimeStyle::InFront,
        NonWorkingTimeStyle::DoNotDraw,
    ];

    /// 按 int 值取枚举实例。
    ///
    /// 对应 Java: `public static NonWorkingTimeStyle getInstance(int type)`
    ///
    /// 越界时 Java 先回落到 `IN_FRONT.getValue()`（1）再取表，故不会返回 `null`。
    pub fn get_instance(value: i32) -> NonWorkingTimeStyle {
        match value {
            0 => NonWorkingTimeStyle::Behind,
            1 => NonWorkingTimeStyle::InFront,
            2 => NonWorkingTimeStyle::DoNotDraw,
            // Java: type < 0 || type >= TYPE_VALUES.length → type = IN_FRONT.getValue()
            _ => NonWorkingTimeStyle::InFront,
        }
    }

    /// 对应 Java: `public static NonWorkingTimeStyle getInstance(Number type)`
    ///
    /// Java 侧 `null` 会走 `getInstance(-1)`，最终回落到 `IN_FRONT`。
    pub fn get_instance_optional(value: Option<i32>) -> NonWorkingTimeStyle {
        NonWorkingTimeStyle::get_instance(value.unwrap_or(-1))
    }

    /// 取名称（未做本地化）。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            NonWorkingTimeStyle::Behind => "Behind",
            NonWorkingTimeStyle::InFront => "In Front",
            NonWorkingTimeStyle::DoNotDraw => "Do Not Draw",
        }
    }

    /// 对应 Java: `@Override public int getValue()`
    pub fn get_value(&self) -> i32 {
        *self as i32
    }
}

impl Display for NonWorkingTimeStyle {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 返回 `getName()`，Rust 侧一致。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl MpxjEnum for NonWorkingTimeStyle {
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.get_value()
    }
}

impl From<NonWorkingTimeStyle> for i32 {
    /// Rust 特有的便捷转换，等价于 [`NonWorkingTimeStyle::get_value`]。
    fn from(value: NonWorkingTimeStyle) -> i32 {
        value.get_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 判别值即 Java 的 int 值，且 `getInstance` 往返一致。
    #[test]
    fn round_trips_through_int_values() {
        for (index, item) in NonWorkingTimeStyle::ALL.iter().enumerate() {
            assert_eq!(item.get_value(), index as i32);
            assert_eq!(NonWorkingTimeStyle::get_instance(index as i32), *item);
            assert_eq!(MpxjEnum::value(item), index as i32);
        }
    }

    /// 越界回落 IN_FRONT（Java 不会返回 null）。
    #[test]
    fn out_of_range_falls_back_to_in_front() {
        for value in [-1, -100, 3, 4, 1000, i32::MAX, i32::MIN] {
            assert_eq!(
                NonWorkingTimeStyle::get_instance(value),
                NonWorkingTimeStyle::InFront
            );
        }
        assert_eq!(
            NonWorkingTimeStyle::get_instance_optional(None),
            NonWorkingTimeStyle::InFront
        );
        assert_eq!(
            NonWorkingTimeStyle::get_instance_optional(Some(2)),
            NonWorkingTimeStyle::DoNotDraw
        );
    }

    /// 名称与 `toString()` 与 Java 一致。
    #[test]
    fn names_and_to_string_match_java() {
        assert_eq!(NonWorkingTimeStyle::Behind.get_name(), "Behind");
        assert_eq!(NonWorkingTimeStyle::InFront.to_string(), "In Front");
        assert_eq!(NonWorkingTimeStyle::DoNotDraw.to_string(), "Do Not Draw");
    }
}
