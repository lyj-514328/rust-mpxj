//! 对应 Java: `org/mpxj/ProjectTimeFormat.java`

use crate::mpxj_enum::MpxjEnum;
use std::fmt::{self, Display, Formatter};

/// 项目时间格式的枚举。
///
/// 对应 Java: `public enum ProjectTimeFormat implements MpxjEnum`
///
/// Java 侧用 `EnumHelper.createTypeArray` 反射生成 int → 枚举的数组；
/// Rust 侧不使用反射，改为 [`ProjectTimeFormat::ALL`] 加
/// [`ProjectTimeFormat::get_instance`] 的 `match`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProjectTimeFormat {
    /// 12 小时制。
    ///
    /// 对应 Java: `TWELVE_HOUR(0)`
    TwelveHour = 0,
    /// 24 小时制。
    ///
    /// 对应 Java: `TWENTY_FOUR_HOUR(1)`
    TwentyFourHour = 1,
}

impl ProjectTimeFormat {
    /// 全部取值，按 int 值升序排列。
    ///
    /// 对应 Java: `private static final ProjectTimeFormat[] TYPE_VALUES`（不含反射部分）。
    pub const ALL: [ProjectTimeFormat; 2] = [
        ProjectTimeFormat::TwelveHour,
        ProjectTimeFormat::TwentyFourHour,
    ];

    /// 根据 int 值取枚举实例，越界时回落到 `TWELVE_HOUR`。
    ///
    /// 对应 Java: `public static ProjectTimeFormat getInstance(int type)`
    pub fn get_instance(value: i32) -> ProjectTimeFormat {
        match value {
            0 => ProjectTimeFormat::TwelveHour,
            1 => ProjectTimeFormat::TwentyFourHour,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = TWELVE_HOUR.getValue(); }`
            _ => ProjectTimeFormat::TwelveHour,
        }
    }

    /// 对应 Java: `public static ProjectTimeFormat getInstance(Number type)`
    ///
    /// Java 侧 `null` 走 `getInstance(-1)`，最终回落到 `TWELVE_HOUR`。
    pub fn get_instance_optional(value: Option<i32>) -> ProjectTimeFormat {
        ProjectTimeFormat::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for ProjectTimeFormat {
    /// 返回该枚举关联的 int 值。
    ///
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for ProjectTimeFormat {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 返回 `Integer.toString(m_value)`，即 int 值的十进制字符串。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as i32)
    }
}

impl From<ProjectTimeFormat> for i32 {
    fn from(value: ProjectTimeFormat) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_time_format_round_trips() {
        for item in ProjectTimeFormat::ALL {
            assert_eq!(ProjectTimeFormat::get_instance(item.value()), item);
        }
    }

    #[test]
    fn project_time_format_falls_back_to_twelve_hour() {
        assert_eq!(
            ProjectTimeFormat::get_instance(-1),
            ProjectTimeFormat::TwelveHour
        );
        assert_eq!(
            ProjectTimeFormat::get_instance(2),
            ProjectTimeFormat::TwelveHour
        );
        assert_eq!(
            ProjectTimeFormat::get_instance_optional(None),
            ProjectTimeFormat::TwelveHour
        );
    }

    #[test]
    fn project_time_format_to_string_is_decimal_value() {
        assert_eq!(ProjectTimeFormat::TwelveHour.to_string(), "0");
        assert_eq!(ProjectTimeFormat::TwentyFourHour.to_string(), "1");
    }
}
