//! 对应 Java: `org/mpxj/mpx/RelationTypeUtility.java`
//!
//! # 区域设置名表
//!
//! Java 用 `LocaleData.getStringArray(locale, LocaleData.RELATION_TYPES)` 从 `ResourceBundle`
//! 取本地化的关系类型名表（非英语语言由 `LocaleData_de` 等资源包覆盖）；Rust 侧同一份数据
//! 由 [`LocaleData`] 承载（本地化覆盖来自同包的 `locale_data_XX` 模块），故本模块按 Java 的
//! 方式**按区域设置取名表** —— 德语文件里的 `EA` 只有走德语名表才能识别。
//!
//! 未移植的成员：无。

use crate::mpx::locale_data::LocaleData;
use crate::mpx::locale_utility::{equals_ignore_case, MpxLocale};
use crate::relation_type::RelationType;

/// 英文（默认）关系类型名表。
///
/// 对应 Java: `LocaleData.RELATION_TYPES_DATA`（下标即 `RelationType` 的 int 值）。
const RELATION_TYPES: [Option<&str>; 4] = [Some("FF"), Some("FS"), Some("SF"), Some("SS")];

/// 管理 MPX 文件中 `RelationType` 实例的工具方法。
///
/// 对应 Java: `final class RelationTypeUtility`（私有构造方法禁止实例化，Rust 侧用无字段类型表达）。
pub struct RelationTypeUtility;

impl RelationTypeUtility {
    /// 把关系类型的文本形式转换成对应的枚举实例，无法识别的值返回 `None`。
    ///
    /// 对应 Java: `public static RelationType getInstance(Locale locale, String type)`
    ///
    /// Java 对无法识别的值（含 `null`，`equalsIgnoreCase(null)` 恒为 `false`）返回 `null`，
    /// Rust 侧用 `Option<RelationType>` 承载。
    pub fn get_instance(locale: MpxLocale, r#type: Option<&str>) -> Option<RelationType> {
        let mut index = -1;

        for (loop_index, name) in relation_types(locale).iter().enumerate() {
            // Java 的名表元素不会是 null；Rust 侧对 `None` 元素直接跳过。
            if let Some(name) = name {
                if equals_ignore_case(name, r#type) {
                    index = loop_index as i32;
                    break;
                }
            }
        }

        if index != -1 {
            // Java: RelationType.getInstance(index)
            Some(RelationType::get_instance(index))
        } else {
            None
        }
    }
}

/// 取对应区域设置的关系类型名表。
///
/// 对应 Java: `LocaleData.getStringArray(locale, LocaleData.RELATION_TYPES)`
///
/// 键缺失时回落英文默认名表（Java 的 `ResourceBundle` 同样会回落到基类资源）。
fn relation_types(locale: MpxLocale) -> &'static [Option<&'static str>] {
    LocaleData::get_string_array(locale, LocaleData::RELATION_TYPES).unwrap_or(&RELATION_TYPES)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpxj_enum::MpxjEnum;

    #[test]
    fn relation_type_round_trips_through_mpx_name() {
        for (index, name) in RELATION_TYPES.iter().enumerate() {
            let name = name.expect("英文名表没有空元素");
            let value = RelationTypeUtility::get_instance(MpxLocale::En, Some(name))
                .expect("英文名表中的值应当可以识别");
            assert_eq!(value.value(), index as i32);
            assert_eq!(RELATION_TYPES[value.value() as usize], Some(name));
            assert_eq!(value.get_name(), name);
        }
    }

    #[test]
    fn relation_type_ignores_case() {
        assert_eq!(
            RelationTypeUtility::get_instance(MpxLocale::En, Some("fs")),
            Some(RelationType::FinishStart)
        );
        assert_eq!(
            RelationTypeUtility::get_instance(MpxLocale::En, Some("ss")),
            Some(RelationType::StartStart)
        );
    }

    #[test]
    fn relation_type_returns_none_for_unrecognised_values() {
        assert_eq!(RelationTypeUtility::get_instance(MpxLocale::En, Some("XX")), None);
        assert_eq!(RelationTypeUtility::get_instance(MpxLocale::En, Some("")), None);
        assert_eq!(RelationTypeUtility::get_instance(MpxLocale::En, Some(" FS")), None);
        assert_eq!(RelationTypeUtility::get_instance(MpxLocale::En, None), None);
    }

    #[test]
    fn relation_type_uses_the_locale_table() {
        // Java: LocaleData_de.RELATION_TYPES_DATA = { "EE", "EA", "AE", "AA" }
        assert_eq!(
            RelationTypeUtility::get_instance(MpxLocale::De, Some("EA")),
            Some(RelationType::FinishStart)
        );
        assert_eq!(
            RelationTypeUtility::get_instance(MpxLocale::De, Some("AA")),
            Some(RelationType::StartStart)
        );
        // 英文名表里的值在德语名表中不存在
        assert_eq!(
            RelationTypeUtility::get_instance(MpxLocale::De, Some("FS")),
            None
        );
    }
}
