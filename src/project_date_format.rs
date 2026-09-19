//! 对应 Java: `org/mpxj/ProjectDateFormat.java`

use crate::mpxj_enum::MpxjEnum;
use std::fmt::{self, Display, Formatter};

/// 项目日期格式的枚举。
///
/// 对应 Java: `public enum ProjectDateFormat implements MpxjEnum`
///
/// Java 侧用 `EnumHelper.createTypeArray` 反射生成 int → 枚举的数组；
/// Rust 侧不使用反射，改为 [`ProjectDateFormat::ALL`] 加
/// [`ProjectDateFormat::get_instance`] 的 `match`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProjectDateFormat {
    /// 形如 25/12/98 12:56。
    ///
    /// 对应 Java: `DD_MM_YY_HH_MM(0)`
    DdMmYyHhMm = 0,
    /// 形如 25/05/98。
    ///
    /// 对应 Java: `DD_MM_YY(1)`
    DdMmYy = 1,
    /// 形如 13 December 2002 12:56。
    ///
    /// 对应 Java: `DD_MMMMM_YYYY_HH_MM(2)`
    DdMmmmmYyyyHhMm = 2,
    /// 形如 13 December 2002。
    ///
    /// 对应 Java: `DD_MMMMM_YYYY(3)`
    DdMmmmmYyyy = 3,
    /// 形如 24 Nov 12:56。
    ///
    /// 对应 Java: `DD_MMM_HH_MM(4)`
    DdMmmHhMm = 4,
    /// 形如 25 Aug '98。
    ///
    /// 对应 Java: `DD_MMM_YY(5)`
    DdMmmYy = 5,
    /// 形如 25 September。
    ///
    /// 对应 Java: `DD_MMMMM(6)`
    DdMmmmm = 6,
    /// 形如 25 Aug。
    ///
    /// 对应 Java: `DD_MMM(7)`
    DdMmm = 7,
    /// 形如 Thu 25/05/98 12:56。
    ///
    /// 对应 Java: `EEE_DD_MM_YY_HH_MM(8)`
    EeeDdMmYyHhMm = 8,
    /// 形如 Wed 25/05/98。
    ///
    /// 对应 Java: `EEE_DD_MM_YY(9)`
    EeeDdMmYy = 9,
    /// 形如 Wed 25 Mar '98。
    ///
    /// 对应 Java: `EEE_DD_MMM_YY(10)`
    EeeDdMmmYy = 10,
    /// 形如 Wed 12:56。
    ///
    /// 对应 Java: `EEE_HH_MM(11)`
    EeeHhMm = 11,
    /// 形如 25/5。
    ///
    /// 对应 Java: `DD_MM(12)`
    DdMm = 12,
    /// 形如 23。
    ///
    /// 对应 Java: `DD(13)`
    Dd = 13,
    /// 形如 12:56。
    ///
    /// 对应 Java: `HH_MM(14)`
    HhMm = 14,
    /// 形如 Wed 23 Mar。
    ///
    /// 对应 Java: `EEE_DD_MMM(15)`
    EeeDdMmm = 15,
    /// 形如 Wed 25/5。
    ///
    /// 对应 Java: `EEE_DD_MM(16)`
    EeeDdMm = 16,
    /// 形如 Wed 05。
    ///
    /// 对应 Java: `EEE_DD(17)`
    EeeDd = 17,
    /// 形如 5/W25。
    ///
    /// 对应 Java: `DD_WWW(18)`
    DdWww = 18,
    /// 形如 5/W25/98 12:56。
    ///
    /// 对应 Java: `DD_WWW_YY_HH_MM(19)`
    DdWwwYyHhMm = 19,
    /// 形如 25/05/1998。
    ///
    /// 对应 Java: `DD_MM_YYYY(20)`
    DdMmYyyy = 20,
}

impl ProjectDateFormat {
    /// 全部取值，按 int 值升序排列。
    ///
    /// 对应 Java: `private static final ProjectDateFormat[] TYPE_VALUES`（不含反射部分）。
    pub const ALL: [ProjectDateFormat; 21] = [
        ProjectDateFormat::DdMmYyHhMm,
        ProjectDateFormat::DdMmYy,
        ProjectDateFormat::DdMmmmmYyyyHhMm,
        ProjectDateFormat::DdMmmmmYyyy,
        ProjectDateFormat::DdMmmHhMm,
        ProjectDateFormat::DdMmmYy,
        ProjectDateFormat::DdMmmmm,
        ProjectDateFormat::DdMmm,
        ProjectDateFormat::EeeDdMmYyHhMm,
        ProjectDateFormat::EeeDdMmYy,
        ProjectDateFormat::EeeDdMmmYy,
        ProjectDateFormat::EeeHhMm,
        ProjectDateFormat::DdMm,
        ProjectDateFormat::Dd,
        ProjectDateFormat::HhMm,
        ProjectDateFormat::EeeDdMmm,
        ProjectDateFormat::EeeDdMm,
        ProjectDateFormat::EeeDd,
        ProjectDateFormat::DdWww,
        ProjectDateFormat::DdWwwYyHhMm,
        ProjectDateFormat::DdMmYyyy,
    ];

    /// 根据 int 值取枚举实例，越界时回落到 `DD_MM_YYYY`。
    ///
    /// 对应 Java: `public static ProjectDateFormat getInstance(int type)`
    pub fn get_instance(value: i32) -> ProjectDateFormat {
        match value {
            0 => ProjectDateFormat::DdMmYyHhMm,
            1 => ProjectDateFormat::DdMmYy,
            2 => ProjectDateFormat::DdMmmmmYyyyHhMm,
            3 => ProjectDateFormat::DdMmmmmYyyy,
            4 => ProjectDateFormat::DdMmmHhMm,
            5 => ProjectDateFormat::DdMmmYy,
            6 => ProjectDateFormat::DdMmmmm,
            7 => ProjectDateFormat::DdMmm,
            8 => ProjectDateFormat::EeeDdMmYyHhMm,
            9 => ProjectDateFormat::EeeDdMmYy,
            10 => ProjectDateFormat::EeeDdMmmYy,
            11 => ProjectDateFormat::EeeHhMm,
            12 => ProjectDateFormat::DdMm,
            13 => ProjectDateFormat::Dd,
            14 => ProjectDateFormat::HhMm,
            15 => ProjectDateFormat::EeeDdMmm,
            16 => ProjectDateFormat::EeeDdMm,
            17 => ProjectDateFormat::EeeDd,
            18 => ProjectDateFormat::DdWww,
            19 => ProjectDateFormat::DdWwwYyHhMm,
            20 => ProjectDateFormat::DdMmYyyy,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = DD_MM_YYYY.getValue(); }`
            _ => ProjectDateFormat::DdMmYyyy,
        }
    }

    /// 对应 Java: `public static ProjectDateFormat getInstance(Number type)`
    ///
    /// Java 侧 `null` 走 `getInstance(-1)`，最终回落到 `DD_MM_YYYY`。
    pub fn get_instance_optional(value: Option<i32>) -> ProjectDateFormat {
        ProjectDateFormat::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for ProjectDateFormat {
    /// 返回该枚举关联的 int 值。
    ///
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for ProjectDateFormat {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 返回 `Integer.toString(m_value)`，即 int 值的十进制字符串。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as i32)
    }
}

impl From<ProjectDateFormat> for i32 {
    fn from(value: ProjectDateFormat) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_date_format_round_trips() {
        for item in ProjectDateFormat::ALL {
            assert_eq!(ProjectDateFormat::get_instance(item.value()), item);
        }
    }

    #[test]
    fn project_date_format_falls_back_to_dd_mm_yyyy() {
        assert_eq!(
            ProjectDateFormat::get_instance(-1),
            ProjectDateFormat::DdMmYyyy
        );
        assert_eq!(
            ProjectDateFormat::get_instance(21),
            ProjectDateFormat::DdMmYyyy
        );
        assert_eq!(
            ProjectDateFormat::get_instance_optional(None),
            ProjectDateFormat::DdMmYyyy
        );
    }

    #[test]
    fn project_date_format_to_string_is_decimal_value() {
        assert_eq!(ProjectDateFormat::DdMmYyyy.to_string(), "20");
        assert_eq!(ProjectDateFormat::DdMmYyHhMm.to_string(), "0");
    }
}
