//! 对应 Java: `org/mpxj/mpx/PriorityUtility.java`
//!
//! # 移植取舍（本地化名表 / `ResourceBundle`）
//!
//! Java 用 `LocaleData.getStringArray(locale, LocaleData.PRIORITY_TYPES)` 从 `ResourceBundle`
//! 取本地化的优先级名表；Rust 标准库没有 `ResourceBundle`，各语言的资源表由同包的
//! `locale_data_XX` 模块（并行移植）承载。本模块不依赖它们，一律使用英文默认名表
//! （`LocaleData.PRIORITY_TYPES_DATA`），`locale` 参数保留以对齐 Java 签名
//! （详见 `locale_utility` 文件头的取舍说明）。
//!
//! 未移植的成员：无。

use crate::mpx::locale_utility::{equals_ignore_case, MpxLocale};
use crate::priority::Priority;

/// 英文（默认）优先级名表。
///
/// 对应 Java: `LocaleData.PRIORITY_TYPES_DATA`（下标 + 1 再乘 100 即 `Priority` 的值）。
const PRIORITY_TYPES: [&str; 10] = [
    "Lowest",
    "Very Low",
    "Lower",
    "Low",
    "Medium",
    "High",
    "Higher",
    "Very High",
    "Highest",
    "Do Not Level",
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
                if equals_ignore_case(name, priority) {
                    index = loop_index;
                    break;
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
/// 取舍见文件头：目前一律返回英文默认名表。
fn priority_types(_locale: MpxLocale) -> &'static [&'static str] {
    &PRIORITY_TYPES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_round_trips_through_mpx_name() {
        for (index, name) in PRIORITY_TYPES.iter().enumerate() {
            let value = PriorityUtility::get_instance(MpxLocale::En, Some(name));
            assert_eq!(value.get_value(), (index as i32 + 1) * 100);
            let name_index = (value.get_value() / 100 - 1) as usize;
            assert_eq!(PRIORITY_TYPES[name_index], *name);
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
    fn priority_uses_english_table_for_non_english_locales() {
        // 取舍：本地化名表未移植，非英文区域设置同样使用英文名表。
        assert_eq!(
            PriorityUtility::get_instance(MpxLocale::Fr, Some("Highest")).get_value(),
            Priority::HIGHEST
        );
    }
}
