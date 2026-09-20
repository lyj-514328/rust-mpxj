//! 对应 Java: `org/mpxj/mpp/LinkStyle.java`
//!
//! 甘特图上关系（链接）线的画法。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `public enum LinkStyle implements MpxjEnum` | 同名 `enum` + `impl crate::MpxjEnum` | 常量按 Java 声明顺序排列，判别值即 Java 的 int 值 |
//! | `private static final LinkStyle[] TYPE_VALUES = EnumHelper.createTypeArray(LinkStyle.class)` | [`LinkStyle::get_instance`] 的 `match` + [`LinkStyle::ALL`] | 表长 3（偏移 0），取值 0..2 全部被填充，故 Java 的 `getInstance` 不会返回 `null` |
//! | `getInstance(Number type)` | [`LinkStyle::get_instance_optional`] | Rust 无重载，加 `_optional` 后缀 |
//! | `getName()` / `toString()` | [`LinkStyle::get_name`] / `Display` | 名称未本地化，与 Java 一致 |
//!
//! 跳过的 Java 成员：`EnumHelper.createTypeArray` 的反射建表（Rust 侧以 `match` + `ALL` 代替）。
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// 关系线的画法。
///
/// 对应 Java: `public enum LinkStyle`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkStyle {
    /// 对应 Java: `NONE(0, "None")`
    None = 0,
    /// 对应 Java: `END_TOP(1, "End Top")`
    EndTop = 1,
    /// 对应 Java: `END_START(2, "End Start")`
    EndStart = 2,
}

impl LinkStyle {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `private static final LinkStyle[] TYPE_VALUES`（去掉反射建表后）。
    pub const ALL: [LinkStyle; 3] = [LinkStyle::None, LinkStyle::EndTop, LinkStyle::EndStart];

    /// 按 int 值取枚举实例。
    ///
    /// 对应 Java: `public static LinkStyle getInstance(int type)`
    ///
    /// 越界时 Java 先回落到 `END_TOP.getValue()`（1）再取表，故不会返回 `null`。
    pub fn get_instance(value: i32) -> LinkStyle {
        match value {
            0 => LinkStyle::None,
            1 => LinkStyle::EndTop,
            2 => LinkStyle::EndStart,
            // Java: type < 0 || type >= TYPE_VALUES.length → type = END_TOP.getValue()
            _ => LinkStyle::EndTop,
        }
    }

    /// 对应 Java: `public static LinkStyle getInstance(Number type)`
    ///
    /// Java 侧 `null` 会走 `getInstance(-1)`，最终回落到 `END_TOP`。
    pub fn get_instance_optional(value: Option<i32>) -> LinkStyle {
        LinkStyle::get_instance(value.unwrap_or(-1))
    }

    /// 取名称（未做本地化）。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            LinkStyle::None => "None",
            LinkStyle::EndTop => "End Top",
            LinkStyle::EndStart => "End Start",
        }
    }

    /// 对应 Java: `@Override public int getValue()`
    pub fn get_value(&self) -> i32 {
        *self as i32
    }
}

impl Display for LinkStyle {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 返回 `getName()`，Rust 侧一致。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl MpxjEnum for LinkStyle {
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.get_value()
    }
}

impl From<LinkStyle> for i32 {
    /// Rust 特有的便捷转换，等价于 [`LinkStyle::get_value`]。
    fn from(value: LinkStyle) -> i32 {
        value.get_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 判别值即 Java 的 int 值，且 `getInstance` 往返一致。
    #[test]
    fn round_trips_through_int_values() {
        for (index, item) in LinkStyle::ALL.iter().enumerate() {
            assert_eq!(item.get_value(), index as i32);
            assert_eq!(LinkStyle::get_instance(index as i32), *item);
            assert_eq!(MpxjEnum::value(item), index as i32);
        }
    }

    /// 越界回落 END_TOP（Java 不会返回 null）。
    #[test]
    fn out_of_range_falls_back_to_end_top() {
        for value in [-1, -100, 3, 4, 1000, i32::MAX, i32::MIN] {
            assert_eq!(LinkStyle::get_instance(value), LinkStyle::EndTop);
        }
        assert_eq!(
            LinkStyle::get_instance_optional(None),
            LinkStyle::EndTop
        );
        assert_eq!(LinkStyle::get_instance_optional(Some(0)), LinkStyle::None);
    }

    /// 名称与 `toString()` 与 Java 一致。
    #[test]
    fn names_and_to_string_match_java() {
        assert_eq!(LinkStyle::None.get_name(), "None");
        assert_eq!(LinkStyle::EndTop.to_string(), "End Top");
        assert_eq!(LinkStyle::EndStart.to_string(), "End Start");
    }
}
