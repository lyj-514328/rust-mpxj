//! 对应 Java: `org/mpxj/mpp/LineStyle.java`
//!
//! Microsoft Project 使用的网格线线型。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `public enum LineStyle implements MpxjEnum` | 同名 `enum` + `impl crate::MpxjEnum` | 常量按 Java 声明顺序排列，判别值即 Java 的 int 值 |
//! | `private static final LineStyle[] TYPE_VALUES = EnumHelper.createTypeArray(LineStyle.class, 1)` | [`LineStyle::get_instance`] 的 `match` + [`LineStyle::ALL`] | 表长为常量数 5 + 偏移 1 = **6**，取值 0..4 被填充，**下标 5 保持 `null`**：`getInstance(5)` 在 Java 中返回 `null`，Rust 侧返回 `None` |
//! | `getInstance(int)` 可能返回 `null` | `Option<LineStyle>` | 「`null` → `Option<T>`」约定（同类处理见 `crate::accrue_type::AccrueType`） |
//! | `getInstance(Number type)` | [`LineStyle::get_instance_optional`] | Rust 无重载，加 `_optional` 后缀；注意其中最外层的 `Option` 是上述「下标 5 → null」 |
//! | `getName()` / `toString()` | [`LineStyle::get_name`] / `Display` | 名称未本地化，与 Java 一致 |
//!
//! 跳过的 Java 成员：`EnumHelper.createTypeArray` 的反射建表（Rust 侧以 `match` + `ALL` 代替）。
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// 网格线线型。
///
/// 对应 Java: `public enum LineStyle`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineStyle {
    /// 对应 Java: `NONE(0, "None")`
    None = 0,
    /// 对应 Java: `SOLID(1, "Solid")`
    Solid = 1,
    /// 对应 Java: `DOTTED1(2, "Dotted1")`
    Dotted1 = 2,
    /// 对应 Java: `DOTTED2(3, "Dotted2")`
    Dotted2 = 3,
    /// 对应 Java: `DASHED(4, "Dashed")`
    Dashed = 4,
}

impl LineStyle {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `private static final LineStyle[] TYPE_VALUES`（去掉反射建表后）。
    pub const ALL: [LineStyle; 5] = [
        LineStyle::None,
        LineStyle::Solid,
        LineStyle::Dotted1,
        LineStyle::Dotted2,
        LineStyle::Dashed,
    ];

    /// 按 int 值取枚举实例；越界回落 `NONE`，数组下标 5（Java 中为 `null`）返回 `None`。
    ///
    /// 对应 Java: `public static LineStyle getInstance(int type)`
    pub fn get_instance(value: i32) -> Option<LineStyle> {
        match value {
            0 => Some(LineStyle::None),
            1 => Some(LineStyle::Solid),
            2 => Some(LineStyle::Dotted1),
            3 => Some(LineStyle::Dotted2),
            4 => Some(LineStyle::Dashed),
            // Java: TYPE_VALUES 长度为 6，索引 5 未被填充，取值为 null
            5 => None,
            // Java: type < 0 || type >= TYPE_VALUES.length → type = NONE.getValue()（0）
            _ => Some(LineStyle::None),
        }
    }

    /// 对应 Java: `public static LineStyle getInstance(Number type)`
    ///
    /// Java 侧 `null` 会走 `getInstance(-1)`，最终回落到 `NONE`。
    pub fn get_instance_optional(value: Option<i32>) -> Option<LineStyle> {
        LineStyle::get_instance(value.unwrap_or(-1))
    }

    /// 取线型名称（未做本地化）。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            LineStyle::None => "None",
            LineStyle::Solid => "Solid",
            LineStyle::Dotted1 => "Dotted1",
            LineStyle::Dotted2 => "Dotted2",
            LineStyle::Dashed => "Dashed",
        }
    }

    /// 对应 Java: `@Override public int getValue()`
    pub fn get_value(&self) -> i32 {
        *self as i32
    }
}

impl Display for LineStyle {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 返回 `getName()`，Rust 侧一致。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl MpxjEnum for LineStyle {
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.get_value()
    }
}

impl From<LineStyle> for i32 {
    /// Rust 特有的便捷转换，等价于 [`LineStyle::get_value`]。
    fn from(value: LineStyle) -> i32 {
        value.get_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 判别值即 Java 的 int 值，且 `getInstance` 往返一致。
    #[test]
    fn round_trips_through_int_values() {
        for (index, item) in LineStyle::ALL.iter().enumerate() {
            assert_eq!(item.get_value(), index as i32);
            assert_eq!(LineStyle::get_instance(index as i32), Some(*item));
            assert_eq!(MpxjEnum::value(item), index as i32);
        }
    }

    /// 下标 5 在 Java 中为 null。
    #[test]
    fn index_five_is_none() {
        assert_eq!(LineStyle::get_instance(5), None);
    }

    /// 越界回落 NONE；`Number` 重载的 null 回落 NONE。
    #[test]
    fn out_of_range_falls_back_to_none() {
        for value in [-1, -100, 6, 7, 1000, i32::MAX, i32::MIN] {
            assert_eq!(LineStyle::get_instance(value), Some(LineStyle::None));
        }
        assert_eq!(
            LineStyle::get_instance_optional(None),
            Some(LineStyle::None)
        );
        assert_eq!(
            LineStyle::get_instance_optional(Some(4)),
            Some(LineStyle::Dashed)
        );
    }

    /// 名称与 `toString()` 与 Java 一致。
    #[test]
    fn names_and_to_string_match_java() {
        assert_eq!(LineStyle::None.get_name(), "None");
        assert_eq!(LineStyle::Dotted1.to_string(), "Dotted1");
        assert_eq!(LineStyle::Dashed.to_string(), "Dashed");
    }
}
