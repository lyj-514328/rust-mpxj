//! 对应 Java: `org/mpxj/mpp/GanttBarDateFormat.java`
//!
//! 甘特图时间标尺（timescale）上可显示的日期格式。
//!
//! # 移植取舍
//!
//! - Java 用 `EnumHelper.createTypeArray(GanttBarDateFormat.class)` 反射生成
//!   int → 枚举的查找表（数组长度 = 常量个数 22，取值 0..21 恰好填满，**但数组下标是
//!   `m_value` 而非声明序号**，所以 `DDMMYYYY(21)` 落在下标 21）；Rust 不用反射，
//!   改为 [`GanttBarDateFormat::ALL`] + [`GanttBarDateFormat::get_instance`] 的 `match`。
//! - **不派生 `Ord`**：Java 枚举的自然序是声明序（`DDMMYYYY` 的 ordinal 为 3），
//!   而判别值是 21，两者不一致，派生 `Ord` 会引入 Java 没有的顺序语义。
//! - 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
//!
//! 未移植的 Java 成员：无。

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// 甘特图时间标尺的日期格式。
///
/// 对应 Java: `public enum GanttBarDateFormat implements MpxjEnum`
///
/// 声明顺序与 Java 一致，判别值即 Java 的 `m_value`（注意 `Ddmmyyyy` 的判别值为 21）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GanttBarDateFormat {
    /// 对应 Java: `DEFAULT(0, "Default")`
    Default = 0,
    /// 对应 Java: `DDMMYY_MMSS(1, "28/01/02 12:33")`
    DdmmyyMmss = 1,
    /// 对应 Java: `DDMMYY(2, "28/01/02")`
    Ddmmyy = 2,
    /// 对应 Java: `DDMMYYYY(21, "28/01/2002")`
    Ddmmyyyy = 21,
    /// 对应 Java: `DD_MMMM_YYYY_HHMM(3, "28 January 2002 12:33")`
    DdMmmmYyyyHhmm = 3,
    /// 对应 Java: `DD_MMMM_YYYY(4, "28 January 2002")`
    DdMmmmYyyy = 4,
    /// 对应 Java: `DD_MMM_HHMM(5, "28 Jan 12:33")`
    DdMmmHhmm = 5,
    /// 对应 Java: `DD_MMM_YY(6, "28 Jan '02")`
    DdMmmYy = 6,
    /// 对应 Java: `DD_MMMM(7, "28 January")`
    DdMmmm = 7,
    /// 对应 Java: `DD_MMM(8, "28 Jan")`
    DdMmm = 8,
    /// 对应 Java: `DDD_DDMMYY_HHMM(9, "Mon 28/01/02 12:33")`
    DddDdmmyyHhmm = 9,
    /// 对应 Java: `DDD_DDMMYY(10, "Mon 28/01/02")`
    DddDdmmyy = 10,
    /// 对应 Java: `DDD_DD_MMM_YY(11, "Mon 28 Jan '02")`
    DddDdMmmYy = 11,
    /// 对应 Java: `DDD_HHMM(12, "Mon 12:33")`
    DddHhmm = 12,
    /// 对应 Java: `DDD_DD_MMM(16, "Mon 28 Jan")`
    DddDdMmm = 16,
    /// 对应 Java: `DDD_DDMM(17, "Mon 28/01")`
    DddDdmm = 17,
    /// 对应 Java: `DDD_DD(18, "Mon 28")`
    DddDd = 18,
    /// 对应 Java: `DDMM(13, "28/01")`
    Ddmm = 13,
    /// 对应 Java: `DD(14, "28")`
    Dd = 14,
    /// 对应 Java: `HHMM(15, "12:33")`
    Hhmm = 15,
    /// 对应 Java: `MWW(19, "1/W05")`
    Mww = 19,
    /// 对应 Java: `MWWYY_HHMM(20, "1/W05/02 12:33")`
    MwwyyHhmm = 20,
}

impl GanttBarDateFormat {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `private static final GanttBarDateFormat[] TYPE_VALUES`（去掉反射建表后）。
    pub const ALL: [GanttBarDateFormat; 22] = [
        GanttBarDateFormat::Default,
        GanttBarDateFormat::DdmmyyMmss,
        GanttBarDateFormat::Ddmmyy,
        GanttBarDateFormat::Ddmmyyyy,
        GanttBarDateFormat::DdMmmmYyyyHhmm,
        GanttBarDateFormat::DdMmmmYyyy,
        GanttBarDateFormat::DdMmmHhmm,
        GanttBarDateFormat::DdMmmYy,
        GanttBarDateFormat::DdMmmm,
        GanttBarDateFormat::DdMmm,
        GanttBarDateFormat::DddDdmmyyHhmm,
        GanttBarDateFormat::DddDdmmyy,
        GanttBarDateFormat::DddDdMmmYy,
        GanttBarDateFormat::DddHhmm,
        GanttBarDateFormat::DddDdMmm,
        GanttBarDateFormat::DddDdmm,
        GanttBarDateFormat::DddDd,
        GanttBarDateFormat::Ddmm,
        GanttBarDateFormat::Dd,
        GanttBarDateFormat::Hhmm,
        GanttBarDateFormat::Mww,
        GanttBarDateFormat::MwwyyHhmm,
    ];

    /// 根据 int 值取枚举实例，越界（含负值）时回落到 `DEFAULT`。
    ///
    /// 对应 Java: `public static GanttBarDateFormat getInstance(int type)`
    pub fn get_instance(value: i32) -> GanttBarDateFormat {
        match value {
            0 => GanttBarDateFormat::Default,
            1 => GanttBarDateFormat::DdmmyyMmss,
            2 => GanttBarDateFormat::Ddmmyy,
            3 => GanttBarDateFormat::DdMmmmYyyyHhmm,
            4 => GanttBarDateFormat::DdMmmmYyyy,
            5 => GanttBarDateFormat::DdMmmHhmm,
            6 => GanttBarDateFormat::DdMmmYy,
            7 => GanttBarDateFormat::DdMmmm,
            8 => GanttBarDateFormat::DdMmm,
            9 => GanttBarDateFormat::DddDdmmyyHhmm,
            10 => GanttBarDateFormat::DddDdmmyy,
            11 => GanttBarDateFormat::DddDdMmmYy,
            12 => GanttBarDateFormat::DddHhmm,
            13 => GanttBarDateFormat::Ddmm,
            14 => GanttBarDateFormat::Dd,
            15 => GanttBarDateFormat::Hhmm,
            16 => GanttBarDateFormat::DddDdMmm,
            17 => GanttBarDateFormat::DddDdmm,
            18 => GanttBarDateFormat::DddDd,
            19 => GanttBarDateFormat::Mww,
            20 => GanttBarDateFormat::MwwyyHhmm,
            21 => GanttBarDateFormat::Ddmmyyyy,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = DEFAULT.getValue(); }`
            _ => GanttBarDateFormat::Default,
        }
    }

    /// 对应 Java: `public static GanttBarDateFormat getInstance(Number type)`
    ///
    /// Java 侧 `null` 走 `getInstance(-1)`，最终回落到 `DEFAULT`。
    pub fn get_instance_optional(value: Option<i32>) -> GanttBarDateFormat {
        GanttBarDateFormat::get_instance(value.unwrap_or(-1))
    }

    /// 取格式名称（即格式样例，如 `"28 January 2002"`），与 Java 一致，未做本地化。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            GanttBarDateFormat::Default => "Default",
            GanttBarDateFormat::DdmmyyMmss => "28/01/02 12:33",
            GanttBarDateFormat::Ddmmyy => "28/01/02",
            GanttBarDateFormat::Ddmmyyyy => "28/01/2002",
            GanttBarDateFormat::DdMmmmYyyyHhmm => "28 January 2002 12:33",
            GanttBarDateFormat::DdMmmmYyyy => "28 January 2002",
            GanttBarDateFormat::DdMmmHhmm => "28 Jan 12:33",
            GanttBarDateFormat::DdMmmYy => "28 Jan '02",
            GanttBarDateFormat::DdMmmm => "28 January",
            GanttBarDateFormat::DdMmm => "28 Jan",
            GanttBarDateFormat::DddDdmmyyHhmm => "Mon 28/01/02 12:33",
            GanttBarDateFormat::DddDdmmyy => "Mon 28/01/02",
            GanttBarDateFormat::DddDdMmmYy => "Mon 28 Jan '02",
            GanttBarDateFormat::DddHhmm => "Mon 12:33",
            GanttBarDateFormat::DddDdMmm => "Mon 28 Jan",
            GanttBarDateFormat::DddDdmm => "Mon 28/01",
            GanttBarDateFormat::DddDd => "Mon 28",
            GanttBarDateFormat::Ddmm => "28/01",
            GanttBarDateFormat::Dd => "28",
            GanttBarDateFormat::Hhmm => "12:33",
            GanttBarDateFormat::Mww => "1/W05",
            GanttBarDateFormat::MwwyyHhmm => "1/W05/02 12:33",
        }
    }
}

impl MpxjEnum for GanttBarDateFormat {
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for GanttBarDateFormat {
    /// 对应 Java: `@Override public String toString()`，即 `getName()`。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl From<GanttBarDateFormat> for i32 {
    fn from(value: GanttBarDateFormat) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_get_instance() {
        for item in GanttBarDateFormat::ALL {
            assert_eq!(GanttBarDateFormat::get_instance(item.value()), item);
        }
    }

    #[test]
    fn out_of_range_falls_back_to_default() {
        assert_eq!(GanttBarDateFormat::get_instance(-1), GanttBarDateFormat::Default);
        assert_eq!(GanttBarDateFormat::get_instance(22), GanttBarDateFormat::Default);
        assert_eq!(
            GanttBarDateFormat::get_instance_optional(None),
            GanttBarDateFormat::Default
        );
    }

    #[test]
    fn lookup_uses_value_not_declaration_index() {
        // DDMMYYYY 的 m_value 为 21（声明位置却在第 4 个）
        assert_eq!(GanttBarDateFormat::get_instance(21), GanttBarDateFormat::Ddmmyyyy);
        assert_eq!(GanttBarDateFormat::get_instance(3), GanttBarDateFormat::DdMmmmYyyyHhmm);
    }

    #[test]
    fn names_match_java() {
        assert_eq!(GanttBarDateFormat::Default.to_string(), "Default");
        assert_eq!(GanttBarDateFormat::DdMmmmYyyy.to_string(), "28 January 2002");
        assert_eq!(GanttBarDateFormat::DdMmmYy.to_string(), "28 Jan '02");
    }
}
