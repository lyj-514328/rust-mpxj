//! 对应 Java: `org/mpxj/mpp/BackgroundPattern.java`
//!
//! 分组（group）填充图案。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `public enum BackgroundPattern implements MpxjEnum` | 同名 `enum` + `impl crate::MpxjEnum` | 常量按 Java 声明顺序排列，判别值即 Java 的 int 值 |
//! | `private static final BackgroundPattern[] TYPE_VALUES = EnumHelper.createTypeArray(BackgroundPattern.class)` | [`BackgroundPattern::get_instance`] 的 `match` + [`BackgroundPattern::ALL`] | 表长 14（偏移 0），取值 0..13 全部被填充，故 Java 的 `getInstance` 不会返回 `null` |
//! | `getName()` / `toString()` | [`BackgroundPattern::get_name`] / `Display` | 名称未本地化，与 Java 一致 |
//!
//! 注意：Java 侧本类**没有** `getInstance(Number)` 重载，因此 Rust 侧也不提供
//! `get_instance_optional`。
//!
//! 跳过的 Java 成员：`EnumHelper.createTypeArray` 的反射建表（Rust 侧以 `match` + `ALL` 代替）。
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// 背景填充图案。
///
/// 对应 Java: `public enum BackgroundPattern`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackgroundPattern {
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
    /// 对应 Java: `INVERSEBACKSLASH(7, "Inverse Back Slash")`
    Inversebackslash = 7,
    /// 对应 Java: `INVERSEFORWARDSLASH(8, "Inverse Forward Slash")`
    Inverseforwardslash = 8,
    /// 对应 Java: `LIGHTVERTICALSTRIPE(9, "Light Vertical Stripe")`
    Lightverticalstripe = 9,
    /// 对应 Java: `HEAVYVERTICALSTRIPE(10, "Heavy Vertical Stripe")`
    Heavyverticalstripe = 10,
    /// 对应 Java: `CHECKERED(11, "Checkered")`
    Checkered = 11,
    /// 对应 Java: `DENSEFORWARDSLASH(12, "Dense Forward Slash")`
    Denseforwardslash = 12,
    /// 对应 Java: `INVERSECHECKERED(13, "Inverse Checkered")`
    Inversecheckered = 13,
}

impl BackgroundPattern {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `private static final BackgroundPattern[] TYPE_VALUES`（去掉反射建表后）。
    pub const ALL: [BackgroundPattern; 14] = [
        BackgroundPattern::Transparent,
        BackgroundPattern::Solid,
        BackgroundPattern::Lightdotted,
        BackgroundPattern::Dotted,
        BackgroundPattern::Heavydotted,
        BackgroundPattern::Backslash,
        BackgroundPattern::Forwardslash,
        BackgroundPattern::Inversebackslash,
        BackgroundPattern::Inverseforwardslash,
        BackgroundPattern::Lightverticalstripe,
        BackgroundPattern::Heavyverticalstripe,
        BackgroundPattern::Checkered,
        BackgroundPattern::Denseforwardslash,
        BackgroundPattern::Inversecheckered,
    ];

    /// 按 int 值取枚举实例。
    ///
    /// 对应 Java: `public static BackgroundPattern getInstance(int type)`
    ///
    /// 越界时 Java 先回落到 `TRANSPARENT.getValue()`（0）再取表，故不会返回 `null`。
    pub fn get_instance(value: i32) -> BackgroundPattern {
        match value {
            0 => BackgroundPattern::Transparent,
            1 => BackgroundPattern::Solid,
            2 => BackgroundPattern::Lightdotted,
            3 => BackgroundPattern::Dotted,
            4 => BackgroundPattern::Heavydotted,
            5 => BackgroundPattern::Backslash,
            6 => BackgroundPattern::Forwardslash,
            7 => BackgroundPattern::Inversebackslash,
            8 => BackgroundPattern::Inverseforwardslash,
            9 => BackgroundPattern::Lightverticalstripe,
            10 => BackgroundPattern::Heavyverticalstripe,
            11 => BackgroundPattern::Checkered,
            12 => BackgroundPattern::Denseforwardslash,
            13 => BackgroundPattern::Inversecheckered,
            // Java: type < 0 || type >= TYPE_VALUES.length → type = TRANSPARENT.getValue()
            _ => BackgroundPattern::Transparent,
        }
    }

    /// 取图案名称（未做本地化）。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            BackgroundPattern::Transparent => "Transparent",
            BackgroundPattern::Solid => "Solid",
            BackgroundPattern::Lightdotted => "Light Dotted",
            BackgroundPattern::Dotted => "Dotted",
            BackgroundPattern::Heavydotted => "Heavy Dotted",
            BackgroundPattern::Backslash => "Back Slash",
            BackgroundPattern::Forwardslash => "Forward Slash",
            BackgroundPattern::Inversebackslash => "Inverse Back Slash",
            BackgroundPattern::Inverseforwardslash => "Inverse Forward Slash",
            BackgroundPattern::Lightverticalstripe => "Light Vertical Stripe",
            BackgroundPattern::Heavyverticalstripe => "Heavy Vertical Stripe",
            BackgroundPattern::Checkered => "Checkered",
            BackgroundPattern::Denseforwardslash => "Dense Forward Slash",
            BackgroundPattern::Inversecheckered => "Inverse Checkered",
        }
    }

    /// 对应 Java: `@Override public int getValue()`
    pub fn get_value(&self) -> i32 {
        *self as i32
    }
}

impl Display for BackgroundPattern {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 返回 `getName()`，Rust 侧一致。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl MpxjEnum for BackgroundPattern {
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.get_value()
    }
}

impl From<BackgroundPattern> for i32 {
    /// Rust 特有的便捷转换，等价于 [`BackgroundPattern::get_value`]。
    fn from(value: BackgroundPattern) -> i32 {
        value.get_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 判别值即 Java 的 int 值，且 `getInstance` 往返一致。
    #[test]
    fn round_trips_through_int_values() {
        for (index, item) in BackgroundPattern::ALL.iter().enumerate() {
            assert_eq!(item.get_value(), index as i32);
            assert_eq!(BackgroundPattern::get_instance(index as i32), *item);
            assert_eq!(MpxjEnum::value(item), index as i32);
        }
    }

    /// 越界回落 TRANSPARENT（Java 不会返回 null）。
    #[test]
    fn out_of_range_falls_back_to_transparent() {
        for value in [-1, -100, 14, 15, 1000, i32::MAX, i32::MIN] {
            assert_eq!(
                BackgroundPattern::get_instance(value),
                BackgroundPattern::Transparent
            );
        }
    }

    /// 名称与 `toString()` 与 Java 一致。
    #[test]
    fn names_and_to_string_match_java() {
        assert_eq!(BackgroundPattern::Transparent.get_name(), "Transparent");
        assert_eq!(BackgroundPattern::Solid.get_name(), "Solid");
        assert_eq!(
            BackgroundPattern::Inversebackslash.to_string(),
            "Inverse Back Slash"
        );
        assert_eq!(
            BackgroundPattern::Denseforwardslash.to_string(),
            "Dense Forward Slash"
        );
        assert_eq!(
            BackgroundPattern::Inversecheckered.to_string(),
            "Inverse Checkered"
        );
    }
}
