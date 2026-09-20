//! 对应 Java: `org/mpxj/mpp/ChartPattern.java`
//!
//! 甘特图条形中段（middle）的填充图案。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `public enum ChartPattern implements MpxjEnum` | 同名 `enum` + `impl crate::MpxjEnum` | 常量按 Java 声明顺序排列，判别值即 Java 的 int 值 |
//! | `private static final ChartPattern[] TYPE_VALUES = EnumHelper.createTypeArray(ChartPattern.class)` | [`ChartPattern::get_instance`] 的 `match` + [`ChartPattern::ALL`] | 表长 12（偏移 0），取值 0..11 全部被填充，故 Java 的 `getInstance` 不会返回 `null` |
//! | `getInstance(Number type)` | [`ChartPattern::get_instance_optional`] | Rust 无重载，加 `_optional` 后缀 |
//! | `getName()` / `toString()` | [`ChartPattern::get_name`] / `Display` | 名称未本地化，与 Java 一致 |
//!
//! 跳过的 Java 成员：`EnumHelper.createTypeArray` 的反射建表（Rust 侧以 `match` + `ALL` 代替）。
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// 甘特图条形填充图案。
///
/// 对应 Java: `public enum ChartPattern`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChartPattern {
    /// 对应 Java: `TRANSPARENT(0, "Transparent")`
    Transparent = 0,
    /// 对应 Java: `SOLID(1, "Solid")`
    Solid = 1,
    /// 对应 Java: `LIGHTDOTTED(2, "Light Dotted")`
    Lightdotted = 2,
    /// 对应 Java: `DOTTED(3, "Dotted")`
    Dotted = 3,
    /// 对应 Java: `HEAVYDOTTED(4, "Heavy Dotted")`
    Heavydotted = 4,
    /// 对应 Java: `BACKSLASH(5, "Back Slash")`
    Backslash = 5,
    /// 对应 Java: `FORWARDSLASH(6, "Forward Slash")`
    Forwardslash = 6,
    /// 对应 Java: `CHECKERED(7, "Checkered")`
    Checkered = 7,
    /// 对应 Java: `VERTICALSTRIPE(8, "Vertical Stripe")`
    Verticalstripe = 8,
    /// 对应 Java: `HORIZONTALSTRIPE(9, "Horizontal Stripe")`
    Horizontalstripe = 9,
    /// 对应 Java: `GRID(10, "Grid")`
    Grid = 10,
    /// 对应 Java: `SOLIDHAIRY(11, "Solid Hairy")`
    Solidhairy = 11,
}

impl ChartPattern {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `private static final ChartPattern[] TYPE_VALUES`（去掉反射建表后）。
    pub const ALL: [ChartPattern; 12] = [
        ChartPattern::Transparent,
        ChartPattern::Solid,
        ChartPattern::Lightdotted,
        ChartPattern::Dotted,
        ChartPattern::Heavydotted,
        ChartPattern::Backslash,
        ChartPattern::Forwardslash,
        ChartPattern::Checkered,
        ChartPattern::Verticalstripe,
        ChartPattern::Horizontalstripe,
        ChartPattern::Grid,
        ChartPattern::Solidhairy,
    ];

    /// 按 int 值取枚举实例。
    ///
    /// 对应 Java: `public static ChartPattern getInstance(int type)`
    ///
    /// 越界时 Java 先回落到 `TRANSPARENT.getValue()`（0）再取表，故不会返回 `null`。
    pub fn get_instance(value: i32) -> ChartPattern {
        match value {
            0 => ChartPattern::Transparent,
            1 => ChartPattern::Solid,
            2 => ChartPattern::Lightdotted,
            3 => ChartPattern::Dotted,
            4 => ChartPattern::Heavydotted,
            5 => ChartPattern::Backslash,
            6 => ChartPattern::Forwardslash,
            7 => ChartPattern::Checkered,
            8 => ChartPattern::Verticalstripe,
            9 => ChartPattern::Horizontalstripe,
            10 => ChartPattern::Grid,
            11 => ChartPattern::Solidhairy,
            // Java: type < 0 || type >= TYPE_VALUES.length → type = TRANSPARENT.getValue()
            _ => ChartPattern::Transparent,
        }
    }

    /// 对应 Java: `public static ChartPattern getInstance(Number type)`
    ///
    /// Java 侧 `null` 会走 `getInstance(-1)`，最终回落到 `TRANSPARENT`。
    pub fn get_instance_optional(value: Option<i32>) -> ChartPattern {
        ChartPattern::get_instance(value.unwrap_or(-1))
    }

    /// 取图案名称（未做本地化）。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            ChartPattern::Transparent => "Transparent",
            ChartPattern::Solid => "Solid",
            ChartPattern::Lightdotted => "Light Dotted",
            ChartPattern::Dotted => "Dotted",
            ChartPattern::Heavydotted => "Heavy Dotted",
            ChartPattern::Backslash => "Back Slash",
            ChartPattern::Forwardslash => "Forward Slash",
            ChartPattern::Checkered => "Checkered",
            ChartPattern::Verticalstripe => "Vertical Stripe",
            ChartPattern::Horizontalstripe => "Horizontal Stripe",
            ChartPattern::Grid => "Grid",
            ChartPattern::Solidhairy => "Solid Hairy",
        }
    }

    /// 对应 Java: `@Override public int getValue()`
    pub fn get_value(&self) -> i32 {
        *self as i32
    }
}

impl Display for ChartPattern {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 返回 `getName()`，Rust 侧一致。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl MpxjEnum for ChartPattern {
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.get_value()
    }
}

impl From<ChartPattern> for i32 {
    /// Rust 特有的便捷转换，等价于 [`ChartPattern::get_value`]。
    fn from(value: ChartPattern) -> i32 {
        value.get_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 判别值即 Java 的 int 值，且 `getInstance` 往返一致。
    #[test]
    fn round_trips_through_int_values() {
        for (index, item) in ChartPattern::ALL.iter().enumerate() {
            assert_eq!(item.get_value(), index as i32);
            assert_eq!(ChartPattern::get_instance(index as i32), *item);
            assert_eq!(MpxjEnum::value(item), index as i32);
        }
    }

    /// 越界回落 TRANSPARENT（Java 不会返回 null）。
    #[test]
    fn out_of_range_falls_back_to_transparent() {
        for value in [-1, -100, 12, 13, 1000, i32::MAX, i32::MIN] {
            assert_eq!(ChartPattern::get_instance(value), ChartPattern::Transparent);
        }
        assert_eq!(
            ChartPattern::get_instance_optional(None),
            ChartPattern::Transparent
        );
        assert_eq!(
            ChartPattern::get_instance_optional(Some(11)),
            ChartPattern::Solidhairy
        );
    }

    /// 名称与 `toString()` 与 Java 一致。
    #[test]
    fn names_and_to_string_match_java() {
        assert_eq!(ChartPattern::Transparent.get_name(), "Transparent");
        assert_eq!(ChartPattern::Lightdotted.get_name(), "Light Dotted");
        assert_eq!(ChartPattern::Heavydotted.to_string(), "Heavy Dotted");
        assert_eq!(ChartPattern::Backslash.to_string(), "Back Slash");
        assert_eq!(ChartPattern::Solidhairy.to_string(), "Solid Hairy");
    }
}
