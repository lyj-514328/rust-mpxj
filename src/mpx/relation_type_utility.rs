//! 对应 Java: `org/mpxj/mpx/RelationTypeUtility.java`
//!
//! # 移植取舍（本地化名表 / `ResourceBundle`）
//!
//! Java 用 `LocaleData.getStringArray(locale, LocaleData.RELATION_TYPES)` 从 `ResourceBundle`
//! 取本地化的关系类型名表；Rust 标准库没有 `ResourceBundle`，各语言的资源表由同包的
//! `locale_data_XX` 模块（并行移植）承载。本模块不依赖它们，一律使用英文默认名表
//! （`LocaleData.RELATION_TYPES_DATA`），`locale` 参数保留以对齐 Java 签名
//! （详见 `locale_utility` 文件头的取舍说明）。
//!
//! 未移植的成员：无。

use crate::mpx::locale_utility::{equals_ignore_case, MpxLocale};
use crate::relation_type::RelationType;

/// 英文（默认）关系类型名表。
///
/// 对应 Java: `LocaleData.RELATION_TYPES_DATA`（下标即 `RelationType` 的 int 值）。
const RELATION_TYPES: [&str; 4] = ["FF", "FS", "SF", "SS"];

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
            if equals_ignore_case(name, r#type) {
                index = loop_index as i32;
                break;
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
/// 取舍见文件头：目前一律返回英文默认名表。
fn relation_types(_locale: MpxLocale) -> &'static [&'static str] {
    &RELATION_TYPES
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpxj_enum::MpxjEnum;

    #[test]
    fn relation_type_round_trips_through_mpx_name() {
        for (index, name) in RELATION_TYPES.iter().enumerate() {
            let value = RelationTypeUtility::get_instance(MpxLocale::En, Some(name))
                .expect("英文名表中的值应当可以识别");
            assert_eq!(value.value(), index as i32);
            assert_eq!(RELATION_TYPES[value.value() as usize], *name);
            assert_eq!(value.get_name(), *name);
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
    fn relation_type_uses_english_table_for_non_english_locales() {
        // 取舍：本地化名表未移植，非英文区域设置同样使用英文名表。
        assert_eq!(
            RelationTypeUtility::get_instance(MpxLocale::Sv, Some("FF")),
            Some(RelationType::FinishFinish)
        );
    }
}
