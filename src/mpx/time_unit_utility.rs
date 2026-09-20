//! 对应 Java: `org/mpxj/mpx/TimeUnitUtility.java`
//!
//! # 区域设置名表
//!
//! Java 用 `LocaleData.getMap(locale, LocaleData.TIME_UNITS_MAP)` 从 `ResourceBundle` 取
//! 「小写名 → `TimeUnit` 的 int 值」映射，该映射由各语言的 `TIME_UNITS_ARRAY_DATA`
//! （每个单位可有多个别名，如 `"d"` / `"days"`）展开而来。Rust 侧同一份数据由 [`LocaleData`]
//! 承载（本地化覆盖来自同包的 `locale_data_XX` 模块），故本模块按 Java 的方式**按区域设置取
//! 名表** —— 德语文件里的 `1t`（Tage）只有走德语名表才能识别。
//!
//! 未移植的成员：无。

use crate::mpx::locale_data::LocaleData;
use crate::mpx::locale_utility::MpxLocale;
use crate::time_unit::TimeUnit;

/// 英文（默认）时间单位名表。
///
/// 对应 Java: `LocaleData.TIME_UNITS_ARRAY_DATA`（下标即 `TimeUnit` 的 int 值，
/// 内层数组为该单位的所有别名）。
const TIME_UNITS_ARRAY: [&[&str]; 14] = [
    &["m", "mins"],         // TimeUnit.MINUTES
    &["h", "hours"],        // TimeUnit.HOURS
    &["d", "days"],         // TimeUnit.DAYS
    &["w", "wk", "weeks"],  // TimeUnit.WEEKS
    &["mon", "months"],     // TimeUnit.MONTHS
    &["%"],                 // TimeUnit.PERCENT
    &["y", "years"],        // TimeUnit.YEARS
    &["em"],                // TimeUnit.ELAPSED_MINUTES
    &["eh"],                // TimeUnit.ELAPSED_HOURS
    &["ed"],                // TimeUnit.ELAPSED_DAYS
    &["ew"],                // TimeUnit.ELAPSED_WEEKS
    &["emon"],              // TimeUnit.ELAPSED_MONTHS
    &["ey"],                // TimeUnit.ELAPSED_YEARS
    &["e%"],                // TimeUnit.ELAPSED_PERCENT
];

/// 管理 MPX 文件中 `TimeUnit` 实例的工具方法。
///
/// 对应 Java: `final class TimeUnitUtility`（私有构造方法禁止实例化，Rust 侧用无字段类型表达）。
pub struct TimeUnitUtility;

impl TimeUnitUtility {
    /// 把时间单位的字符串形式转换成对应的枚举实例。
    ///
    /// 对应 Java: `public static TimeUnit getInstance(String units, Locale locale) throws MPXJException`
    ///
    /// Java 查找时先对入参调用 `toLowerCase()`；无法识别时抛
    /// `MPXJException(INVALID_TIME_UNIT + " " + units)`，Rust 侧改为返回 `None`。
    /// Java 的 `units` 为 `String`（`null` 会触发 `NullPointerException`），
    /// Rust 侧用 `Option<&str>` 承载，`None` 同样返回 `None`。
    pub fn get_instance(units: Option<&str>, locale: MpxLocale) -> Option<TimeUnit> {
        let units = units?.to_lowercase();

        for (value, names) in time_units_array(locale).iter().enumerate() {
            if names.contains(&units.as_str()) {
                return Some(TimeUnit::get_instance(value as i32));
            }
        }

        None
    }
}

/// 取对应区域设置的时间单位名表。
///
/// 对应 Java: `LocaleData.getMap(locale, LocaleData.TIME_UNITS_MAP)`（其数据源为
/// `TIME_UNITS_ARRAY_DATA`）
///
/// 键缺失时回落英文默认名表（Java 的 `ResourceBundle` 同样会回落到基类资源）。
fn time_units_array(locale: MpxLocale) -> &'static [&'static [&'static str]] {
    LocaleData::get_string_arrays(locale, LocaleData::TIME_UNITS_ARRAY).unwrap_or(&TIME_UNITS_ARRAY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_unit_round_trips_through_mpx_names() {
        for (value, names) in TIME_UNITS_ARRAY.iter().enumerate() {
            let unit = TimeUnit::get_instance(value as i32);
            for name in names.iter() {
                // MPX 名 → 模型值
                assert_eq!(
                    TimeUnitUtility::get_instance(Some(name), MpxLocale::En),
                    Some(unit)
                );
                // 模型值 → MPX 名（名表中的首选名）→ 再解析回模型值
                assert_eq!(
                    TimeUnitUtility::get_instance(Some(names[0]), MpxLocale::En),
                    Some(unit)
                );
            }
        }
    }

    #[test]
    fn time_unit_name_is_not_always_an_mpx_name() {
        // Java 的 `TimeUnit.getName()` 对 MONTHS / ELAPSED_MONTHS 返回 "mo" / "emo"，
        // 而 MPX 英文名表用的是 "mon" / "emon"，二者并不一致。
        assert_eq!(TimeUnit::Months.get_name(), "mo");
        assert_eq!(TimeUnitUtility::get_instance(Some("mon"), MpxLocale::En), Some(TimeUnit::Months));
        assert_eq!(TimeUnitUtility::get_instance(Some("mo"), MpxLocale::En), None);
        assert_eq!(TimeUnitUtility::get_instance(Some("emon"), MpxLocale::En), Some(TimeUnit::ElapsedMonths));
        assert_eq!(TimeUnitUtility::get_instance(Some("emo"), MpxLocale::En), None);
    }

    #[test]
    fn time_unit_is_case_insensitive() {
        // Java: map.get(units.toLowerCase())
        assert_eq!(
            TimeUnitUtility::get_instance(Some("H"), MpxLocale::En),
            Some(TimeUnit::Hours)
        );
        assert_eq!(
            TimeUnitUtility::get_instance(Some("DAYS"), MpxLocale::En),
            Some(TimeUnit::Days)
        );
        assert_eq!(
            TimeUnitUtility::get_instance(Some("Mon"), MpxLocale::En),
            Some(TimeUnit::Months)
        );
    }

    #[test]
    fn time_unit_rejects_unrecognised_values() {
        // Java: 抛 MPXJException(INVALID_TIME_UNIT)
        assert_eq!(TimeUnitUtility::get_instance(Some("x"), MpxLocale::En), None);
        assert_eq!(TimeUnitUtility::get_instance(Some("d "), MpxLocale::En), None);
        assert_eq!(TimeUnitUtility::get_instance(Some(""), MpxLocale::En), None);
        assert_eq!(TimeUnitUtility::get_instance(None, MpxLocale::En), None);
    }

    #[test]
    fn time_unit_uses_the_locale_table() {
        // Java: LocaleData_de.TIME_UNITS_ARRAY_DATA 用 "t" 表示天（Tage）
        assert_eq!(
            TimeUnitUtility::get_instance(Some("t"), MpxLocale::De),
            Some(TimeUnit::Days)
        );
        assert_eq!(
            TimeUnitUtility::get_instance(Some("h"), MpxLocale::De),
            Some(TimeUnit::Hours)
        );
        // 英文名表里的 "d"/"days" 在德语名表中不存在
        assert_eq!(TimeUnitUtility::get_instance(Some("d"), MpxLocale::De), None);
        // 英文名表里的 "t" 也不是任何单位
        assert_eq!(TimeUnitUtility::get_instance(Some("t"), MpxLocale::En), None);
    }
}
