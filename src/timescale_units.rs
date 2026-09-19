//! 对应 Java: `org/mpxj/TimescaleUnits.java`

use std::fmt::{self, Display, Formatter};

/// 甘特图时间刻度所用单位的枚举。
///
/// 对应 Java: `public enum TimescaleUnits`
///
/// Java 侧该枚举不带 int 值、不实现 `MpxjEnum`，也没有 `getInstance`
/// （int ↔ `TimescaleUnits` 的映射在 Java 中由 `org.mpxj.mpp.TimescaleUnitsHelper`
/// 完成）；Rust 侧保持同样的形态（不实现 [`crate::mpxj_enum::MpxjEnum`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TimescaleUnits {
    /// 对应 Java: `NONE("None")`
    None,
    /// 对应 Java: `MINUTES("Minutes")`
    Minutes,
    /// 对应 Java: `HOURS("Hours")`
    Hours,
    /// 对应 Java: `DAYS("Days")`
    Days,
    /// 对应 Java: `WEEKS("Weeks")`
    Weeks,
    /// 对应 Java: `THIRDS_OF_MONTHS("Thirds of Months")`
    ThirdsOfMonths,
    /// 对应 Java: `MONTHS("Months")`
    Months,
    /// 对应 Java: `QUARTERS("Quarters")`
    Quarters,
    /// 对应 Java: `HALF_YEARS("Half Years")`
    HalfYears,
    /// 对应 Java: `YEARS("Years")`
    Years,
}

impl TimescaleUnits {
    /// 全部取值，按 Java 声明顺序排列。
    ///
    /// 对应 Java: `TimescaleUnits.values()`
    pub const ALL: [TimescaleUnits; 10] = [
        TimescaleUnits::None,
        TimescaleUnits::Minutes,
        TimescaleUnits::Hours,
        TimescaleUnits::Days,
        TimescaleUnits::Weeks,
        TimescaleUnits::ThirdsOfMonths,
        TimescaleUnits::Months,
        TimescaleUnits::Quarters,
        TimescaleUnits::HalfYears,
        TimescaleUnits::Years,
    ];

    /// 按声明顺序（ordinal）取枚举实例。
    ///
    /// Java 侧无此方法；Rust 侧补充以便与 [`TimescaleUnits::ALL`] 配套查表，
    /// 对应 Java 中 `values()[value]` 的写法。Java 越界会抛
    /// `ArrayIndexOutOfBoundsException`，Rust 侧按本仓约定返回 `None`。
    pub fn get_instance(value: i32) -> Option<TimescaleUnits> {
        usize::try_from(value)
            .ok()
            .and_then(|index| TimescaleUnits::ALL.get(index))
            .copied()
    }

    /// 返回该时间刻度单位的名称（未做本地化）。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            TimescaleUnits::None => "None",
            TimescaleUnits::Minutes => "Minutes",
            TimescaleUnits::Hours => "Hours",
            TimescaleUnits::Days => "Days",
            TimescaleUnits::Weeks => "Weeks",
            TimescaleUnits::ThirdsOfMonths => "Thirds of Months",
            TimescaleUnits::Months => "Months",
            TimescaleUnits::Quarters => "Quarters",
            TimescaleUnits::HalfYears => "Half Years",
            TimescaleUnits::Years => "Years",
        }
    }
}

impl Display for TimescaleUnits {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 返回字段 `m_name`，与 `getName()` 相同。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timescale_units_round_trips() {
        for (index, item) in TimescaleUnits::ALL.iter().enumerate() {
            assert_eq!(TimescaleUnits::get_instance(index as i32), Some(*item));
        }
    }

    #[test]
    fn timescale_units_all_matches_java_declaration_order() {
        assert_eq!(
            TimescaleUnits::ALL,
            [
                TimescaleUnits::None,
                TimescaleUnits::Minutes,
                TimescaleUnits::Hours,
                TimescaleUnits::Days,
                TimescaleUnits::Weeks,
                TimescaleUnits::ThirdsOfMonths,
                TimescaleUnits::Months,
                TimescaleUnits::Quarters,
                TimescaleUnits::HalfYears,
                TimescaleUnits::Years
            ]
        );
    }

    #[test]
    fn timescale_units_get_instance_is_out_of_range_safe() {
        assert_eq!(TimescaleUnits::get_instance(-1), None);
        assert_eq!(TimescaleUnits::get_instance(10), None);
    }

    #[test]
    fn timescale_units_names_and_to_string_match_java() {
        for item in TimescaleUnits::ALL {
            assert_eq!(item.to_string(), item.get_name());
        }
        assert_eq!(TimescaleUnits::None.get_name(), "None");
        assert_eq!(TimescaleUnits::ThirdsOfMonths.get_name(), "Thirds of Months");
        assert_eq!(TimescaleUnits::HalfYears.get_name(), "Half Years");
        assert_eq!(TimescaleUnits::Years.to_string(), "Years");
    }
}
