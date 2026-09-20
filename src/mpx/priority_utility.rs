//! 对应 Java: `org/mpxj/mpx/PriorityUtility.java`
//!
//! # 区域设置名表
//!
//! Java 用 `LocaleData.getStringArray(locale, LocaleData.PRIORITY_TYPES)` 从 `ResourceBundle`
//! 取本地化的优先级名表（非英语语言由 `LocaleData_de` 等资源包覆盖）；Rust 侧同一份数据
//! 由 [`LocaleData`] 承载（本地化覆盖来自同包的 `locale_data_XX` 模块），故本模块按 Java 的
//! 方式**按区域设置取名表**。
//!
//! 未移植的成员：无。

use crate::mpx::locale_data::LocaleData;
use crate::mpx::locale_utility::{equals_ignore_case, MpxLocale};
use crate::priority::Priority;

/// 英文（默认）优先级名表。
///
/// 对应 Java: `LocaleData.PRIORITY_TYPES_DATA`（下标 + 1 再乘 100 即 `Priority` 的值）。
const PRIORITY_TYPES: [Option<&str>; 10] = [
    Some("Lowest"),
    Some("Very Low"),
    Some("Lower"),
    Some("Low"),
    Some("Medium"),
    Some("High"),
    Some("Higher"),
    Some("Very High"),
    Some("Highest"),
    Some("Do Not Level"),
];

/// 管理 MPX 文件中 `Priority` 实例的工具方法。
///
/// 对应 Java: `final class PriorityUtility`（私有构造方法禁止实例化，Rust 侧用无字段类型表达）。
pub struct PriorityUtility;

impl PriorityUtility {
    /// 把优先级的文本形式转换成对应的实例，无法识别的值按「中」优先级处理。
    ///
    /// 对应 Java: `public static Priority getInstance(Locale locale, String priority)`
    ///
    /// Java 对 `null` 入参显式跳过名表查找并保留默认下标，Rust 侧用 `Option<&str>` 承载。
    pub fn get_instance(locale: MpxLocale, priority: Option<&str>) -> Priority {
        let mut index = DEFAULT_PRIORITY_INDEX;

        if priority.is_some() {
            for (loop_index, name) in priority_types(locale).iter().enumerate() {
                // Java 的名表元素不会是 null；Rust 侧对 `None` 元素直接跳过。
                if let Some(name) = name {
                    if equals_ignore_case(name, priority) {
                        index = loop_index;
                        break;
                    }
                }
            }
        }

        // Java: Priority.getInstance((index + 1) * 100)
        Priority::get_instance((index as i32 + 1) * 100)
    }
}

/// 默认优先级在名表中的下标。
///
/// 对应 Java: `private static final int DEFAULT_PRIORITY_INDEX = 4`
const DEFAULT_PRIORITY_INDEX: usize = 4;

/// 取对应区域设置的优先级名表。
///
/// 对应 Java: `LocaleData.getStringArray(locale, LocaleData.PRIORITY_TYPES)`
///
/// 键缺失时回落英文默认名表（Java 的 `ResourceBundle` 同样会回落到基类资源）。
fn priority_types(locale: MpxLocale) -> &'static [Option<&'static str>] {
    LocaleData::get_string_array(locale, LocaleData::PRIORITY_TYPES).unwrap_or(&PRIORITY_TYPES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_round_trips_through_mpx_name() {
        for (index, name) in PRIORITY_TYPES.iter().enumerate() {
            let name = name.expect("英文名表没有空元素");
            let value = PriorityUtility::get_instance(MpxLocale::En, Some(name));
            assert_eq!(value.get_value(), (index as i32 + 1) * 100);
            let name_index = (value.get_value() / 100 - 1) as usize;
            assert_eq!(PRIORITY_TYPES[name_index], Some(name));
        }
    }

    #[test]
    fn priority_ignores_case() {
        assert_eq!(
            PriorityUtility::get_instance(MpxLocale::En, Some("very HIGH")).get_value(),
            Priority::VERY_HIGH
        );
        assert_eq!(
            PriorityUtility::get_instance(MpxLocale::En, Some("do not level")).get_value(),
            Priority::DO_NOT_LEVEL
        );
    }

    #[test]
    fn priority_falls_back_to_medium() {
        assert_eq!(
            PriorityUtility::get_instance(MpxLocale::En, Some("Unknown")).get_value(),
            Priority::MEDIUM
        );
        assert_eq!(
            PriorityUtility::get_instance(MpxLocale::En, Some(" Medium")).get_value(),
            Priority::MEDIUM
        );
        assert_eq!(
            PriorityUtility::get_instance(MpxLocale::En, None).get_value(),
            Priority::MEDIUM
        );
    }

    #[test]
    fn priority_uses_the_locale_table() {
        // Java: LocaleData_de.PRIORITY_TYPES_DATA[8] = "Am höchsten"
        assert_eq!(
            PriorityUtility::get_instance(MpxLocale::De, Some("Am höchsten")).get_value(),
            Priority::HIGHEST
        );
        // 英文名表里的值在德语名表中不存在，回落「中」
        assert_eq!(
            PriorityUtility::get_instance(MpxLocale::De, Some("Highest")).get_value(),
            Priority::MEDIUM
        );
    }
}
