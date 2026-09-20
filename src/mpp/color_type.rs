//! 对应 Java: `org/mpxj/mpp/ColorType.java`
//!
//! Microsoft Project 使用的颜色枚举（注意：这是 MPXJ 自己的颜色枚举，
//! 不是 `java.awt.Color`）。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `public enum ColorType implements MpxjEnum` | 同名 `enum` + `impl crate::MpxjEnum` | 常量按 Java 声明顺序排列，判别值即 Java 的 int 值 |
//! | 构造参数 `java.awt.Color color` / `Color getColor()` | [`crate::mpp::mpp_utility::Color`] + `Option<Color>` | `java.awt.*` 属跳过项；Rust 侧复用 `crate::mpp::mpp_utility::Color`（`MPPUtility.get_color` 的返回值类型），并只保留 R/G/B 分量。Java 中 `AUTOMATIC` 的颜色为 `null`，故 getter 返回 `Option<Color>` |
//! | `private static final ColorType[] TYPE_VALUES = EnumHelper.createTypeArray(ColorType.class)` | [`ColorType::get_instance`] 的 `match` + [`ColorType::ALL`] | 表长 17（偏移 0），取值 0..16 全部被填充，故 Java 的 `getInstance` 不会返回 `null` |
//! | `getInstance(Number type)` | [`ColorType::get_instance_optional`] | Rust 无重载，加 `_optional` 后缀 |
//! | 未覆写 `toString()`（继承 `java.lang.Enum.toString()`，输出**常量名**） | `Display`，输出 `BLACK` / `RED` / … / `AUTOMATIC` | Java 枚举即使未覆写也有 `toString()`，其值就是常量名；注意这与 `getName()`（`"Black"` 等）不同 |
//!
//! 跳过的 Java 成员：`EnumHelper.createTypeArray` 的反射建表（Rust 侧以 `match` + `ALL` 代替）。
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*`（含 `java.awt.*`）/ `Serializable`：
//! 除颜色常量本身外，本类未使用这些机制。

use std::fmt::{self, Display, Formatter};

use crate::mpp::mpp_utility::Color;
use crate::mpxj_enum::MpxjEnum;

/// Microsoft Project 使用的颜色。
///
/// 对应 Java: `public enum ColorType`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorType {
    /// 对应 Java: `BLACK(0, "Black", Color.BLACK)`
    Black = 0,
    /// 对应 Java: `RED(1, "Red", Color.RED)`
    Red = 1,
    /// 对应 Java: `YELLOW(2, "Yellow", Color.YELLOW)`
    Yellow = 2,
    /// 对应 Java: `LIME(3, "Lime", new Color(148, 255, 148))`
    Lime = 3,
    /// 对应 Java: `AQUA(4, "Aqua", new Color(194, 220, 255))`
    Aqua = 4,
    /// 对应 Java: `BLUE(5, "Blue", Color.BLUE)`
    Blue = 5,
    /// 对应 Java: `FUSCHIA(6, "Fuschia", Color.MAGENTA)`
    Fuschia = 6,
    /// 对应 Java: `WHITE(7, "White", Color.WHITE)`
    White = 7,
    /// 对应 Java: `MAROON(8, "Maroon", new Color(128, 0, 0))`
    Maroon = 8,
    /// 对应 Java: `GREEN(9, "Green", new Color(0, 128, 0))`
    Green = 9,
    /// 对应 Java: `OLIVE(10, "Olive", new Color(128, 128, 0))`
    Olive = 10,
    /// 对应 Java: `NAVY(11, "Navy", new Color(0, 0, 128))`
    Navy = 11,
    /// 对应 Java: `PURPLE(12, "Purple", new Color(128, 0, 128))`
    Purple = 12,
    /// 对应 Java: `TEAL(13, "Teal", new Color(0, 128, 128))`
    Teal = 13,
    /// 对应 Java: `GRAY(14, "Gray", new Color(128, 128, 128))`
    Gray = 14,
    /// 对应 Java: `SILVER(15, "Silver", new Color(192, 192, 192))`
    Silver = 15,
    /// 对应 Java: `AUTOMATIC(16, "Automatic", null)`
    Automatic = 16,
}

impl ColorType {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `private static final ColorType[] TYPE_VALUES`（去掉反射建表后）。
    pub const ALL: [ColorType; 17] = [
        ColorType::Black,
        ColorType::Red,
        ColorType::Yellow,
        ColorType::Lime,
        ColorType::Aqua,
        ColorType::Blue,
        ColorType::Fuschia,
        ColorType::White,
        ColorType::Maroon,
        ColorType::Green,
        ColorType::Olive,
        ColorType::Navy,
        ColorType::Purple,
        ColorType::Teal,
        ColorType::Gray,
        ColorType::Silver,
        ColorType::Automatic,
    ];

    /// 按 int 值取枚举实例。
    ///
    /// 对应 Java: `public static ColorType getInstance(int type)`
    ///
    /// Java 的查找表长度为 17（`createTypeArray` 偏移 0），0..16 全部被填充，
    /// 越界时先回落到 `AUTOMATIC.getValue()`（16）再取表，故方法不会返回 `null`。
    pub fn get_instance(value: i32) -> ColorType {
        match value {
            0 => ColorType::Black,
            1 => ColorType::Red,
            2 => ColorType::Yellow,
            3 => ColorType::Lime,
            4 => ColorType::Aqua,
            5 => ColorType::Blue,
            6 => ColorType::Fuschia,
            7 => ColorType::White,
            8 => ColorType::Maroon,
            9 => ColorType::Green,
            10 => ColorType::Olive,
            11 => ColorType::Navy,
            12 => ColorType::Purple,
            13 => ColorType::Teal,
            14 => ColorType::Gray,
            15 => ColorType::Silver,
            // Java: type < 0 || type >= TYPE_VALUES.length → type = AUTOMATIC.getValue()
            16 => ColorType::Automatic,
            _ => ColorType::Automatic,
        }
    }

    /// 对应 Java: `public static ColorType getInstance(Number type)`
    ///
    /// Java 侧 `null` 会走 `getInstance(-1)`，最终回落到 `AUTOMATIC`。
    pub fn get_instance_optional(value: Option<i32>) -> ColorType {
        ColorType::get_instance(value.unwrap_or(-1))
    }

    /// 取颜色名称（未做本地化）。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            ColorType::Black => "Black",
            ColorType::Red => "Red",
            ColorType::Yellow => "Yellow",
            ColorType::Lime => "Lime",
            ColorType::Aqua => "Aqua",
            ColorType::Blue => "Blue",
            ColorType::Fuschia => "Fuschia",
            ColorType::White => "White",
            ColorType::Maroon => "Maroon",
            ColorType::Green => "Green",
            ColorType::Olive => "Olive",
            ColorType::Navy => "Navy",
            ColorType::Purple => "Purple",
            ColorType::Teal => "Teal",
            ColorType::Gray => "Gray",
            ColorType::Silver => "Silver",
            ColorType::Automatic => "Automatic",
        }
    }

    /// 取与 MS Project 中该颜色对应的颜色值；`AUTOMATIC` 返回 `None`。
    ///
    /// 对应 Java: `public Color getColor()`（Java 注释：`AUTOMATIC` 时返回 `null`）
    pub fn get_color(&self) -> Option<Color> {
        match self {
            ColorType::Black => Some(Color::new(0, 0, 0)),
            ColorType::Red => Some(Color::new(255, 0, 0)),
            ColorType::Yellow => Some(Color::new(255, 255, 0)),
            ColorType::Lime => Some(Color::new(148, 255, 148)),
            ColorType::Aqua => Some(Color::new(194, 220, 255)),
            ColorType::Blue => Some(Color::new(0, 0, 255)),
            // Java: Color.MAGENTA
            ColorType::Fuschia => Some(Color::new(255, 0, 255)),
            ColorType::White => Some(Color::new(255, 255, 255)),
            ColorType::Maroon => Some(Color::new(128, 0, 0)),
            ColorType::Green => Some(Color::new(0, 128, 0)),
            ColorType::Olive => Some(Color::new(128, 128, 0)),
            ColorType::Navy => Some(Color::new(0, 0, 128)),
            ColorType::Purple => Some(Color::new(128, 0, 128)),
            ColorType::Teal => Some(Color::new(0, 128, 128)),
            ColorType::Gray => Some(Color::new(128, 128, 128)),
            ColorType::Silver => Some(Color::new(192, 192, 192)),
            // Java: m_color == null
            ColorType::Automatic => None,
        }
    }

    /// 对应 Java: `@Override public int getValue()`
    pub fn get_value(&self) -> i32 {
        *self as i32
    }
}

impl Display for ColorType {
    /// 对应 Java: 继承自 `java.lang.Enum` 的 `toString()`
    ///
    /// Java 的 `Enum.toString()` 返回常量名（如 `AUTOMATIC`），本类未覆写，故 Rust 侧
    /// 输出同样的常量名（与 [`ColorType::get_name`] 的 `"Automatic"` 不同）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ColorType::Black => "BLACK",
            ColorType::Red => "RED",
            ColorType::Yellow => "YELLOW",
            ColorType::Lime => "LIME",
            ColorType::Aqua => "AQUA",
            ColorType::Blue => "BLUE",
            ColorType::Fuschia => "FUSCHIA",
            ColorType::White => "WHITE",
            ColorType::Maroon => "MAROON",
            ColorType::Green => "GREEN",
            ColorType::Olive => "OLIVE",
            ColorType::Navy => "NAVY",
            ColorType::Purple => "PURPLE",
            ColorType::Teal => "TEAL",
            ColorType::Gray => "GRAY",
            ColorType::Silver => "SILVER",
            ColorType::Automatic => "AUTOMATIC",
        })
    }
}

impl MpxjEnum for ColorType {
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.get_value()
    }
}

impl From<ColorType> for i32 {
    /// Rust 特有的便捷转换，等价于 [`ColorType::get_value`]。
    fn from(value: ColorType) -> i32 {
        value.get_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 判别值即 Java 的 int 值，且 `getInstance` 往返一致。
    #[test]
    fn round_trips_through_int_values() {
        for (index, item) in ColorType::ALL.iter().enumerate() {
            assert_eq!(item.get_value(), index as i32);
            assert_eq!(ColorType::get_instance(index as i32), *item);
            assert_eq!(MpxjEnum::value(item), index as i32);
        }
    }

    /// 越界回落 AUTOMATIC（Java 不会返回 null）。
    #[test]
    fn out_of_range_falls_back_to_automatic() {
        for value in [-1, -100, 17, 18, 1000, i32::MAX, i32::MIN] {
            assert_eq!(ColorType::get_instance(value), ColorType::Automatic);
        }
        assert_eq!(
            ColorType::get_instance_optional(None),
            ColorType::Automatic
        );
        assert_eq!(
            ColorType::get_instance_optional(Some(3)),
            ColorType::Lime
        );
    }

    /// 名称与 Java 一致。
    #[test]
    fn names_match_java() {
        assert_eq!(ColorType::Black.get_name(), "Black");
        assert_eq!(ColorType::Fuschia.get_name(), "Fuschia");
        assert_eq!(ColorType::Automatic.get_name(), "Automatic");
        for item in ColorType::ALL {
            assert!(!item.get_name().is_empty());
        }
    }

    /// `toString()`（Java 的 `Enum.toString()`）输出常量名，与 `getName()` 不同。
    #[test]
    fn to_string_outputs_the_java_constant_name() {
        assert_eq!(ColorType::Black.to_string(), "BLACK");
        assert_eq!(ColorType::Automatic.to_string(), "AUTOMATIC");
        assert_eq!(ColorType::Fuschia.to_string(), "FUSCHIA");
        assert_ne!(ColorType::Automatic.to_string(), ColorType::Automatic.get_name());
    }

    /// 颜色分量与 Java 的 `java.awt.Color` 取值一致；AUTOMATIC 为 null。
    #[test]
    fn colors_match_java_awt_values() {
        assert_eq!(ColorType::Black.get_color(), Some(Color::new(0, 0, 0)));
        assert_eq!(ColorType::Red.get_color(), Some(Color::new(255, 0, 0)));
        assert_eq!(ColorType::Yellow.get_color(), Some(Color::new(255, 255, 0)));
        // Java: new Color(148, 255, 148)
        assert_eq!(ColorType::Lime.get_color(), Some(Color::new(148, 255, 148)));
        // Java: new Color(194, 220, 255)
        assert_eq!(ColorType::Aqua.get_color(), Some(Color::new(194, 220, 255)));
        assert_eq!(ColorType::Blue.get_color(), Some(Color::new(0, 0, 255)));
        // Java: Color.MAGENTA
        assert_eq!(
            ColorType::Fuschia.get_color(),
            Some(Color::new(255, 0, 255))
        );
        assert_eq!(ColorType::White.get_color(), Some(Color::new(255, 255, 255)));
        assert_eq!(ColorType::Maroon.get_color(), Some(Color::new(128, 0, 0)));
        assert_eq!(ColorType::Green.get_color(), Some(Color::new(0, 128, 0)));
        assert_eq!(ColorType::Olive.get_color(), Some(Color::new(128, 128, 0)));
        assert_eq!(ColorType::Navy.get_color(), Some(Color::new(0, 0, 128)));
        assert_eq!(ColorType::Purple.get_color(), Some(Color::new(128, 0, 128)));
        assert_eq!(ColorType::Teal.get_color(), Some(Color::new(0, 128, 128)));
        assert_eq!(ColorType::Gray.get_color(), Some(Color::new(128, 128, 128)));
        assert_eq!(
            ColorType::Silver.get_color(),
            Some(Color::new(192, 192, 192))
        );

        // Java: AUTOMATIC 的颜色为 null
        assert_eq!(ColorType::Automatic.get_color(), None);

        // 除 AUTOMATIC 外全部有颜色
        assert_eq!(
            ColorType::ALL
                .iter()
                .filter(|item| item.get_color().is_none())
                .count(),
            1
        );
    }
}
