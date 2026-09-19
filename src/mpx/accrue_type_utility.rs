//! 对应 Java: `org/mpxj/mpx/AccrueTypeUtility.java`
//!
//! # 移植取舍（本地化名表 / `ResourceBundle`）
//!
//! Java 用 `LocaleData.getStringArray(locale, LocaleData.ACCRUE_TYPES)` 从 `ResourceBundle`
//! 取本地化的成本计提方式名表；Rust 标准库没有 `ResourceBundle`，各语言的资源表由同包的
//! `locale_data_XX` 模块（并行移植）承载。本模块不依赖它们，一律使用英文默认名表
//! （`LocaleData.ACCRUE_TYPES_DATA`），`locale` 参数保留以对齐 Java 签名
//! （详见 `locale_utility` 文件头的取舍说明）。
//!
//! 未移植的成员：无。

use crate::accrue_type::AccrueType;
use crate::mpx::locale_utility::{equals_ignore_case, MpxLocale};

/// 英文（默认）成本计提方式名表。
///
/// 对应 Java: `LocaleData.ACCRUE_TYPES_DATA`（下标 + 1 即 `AccrueType` 的 int 值）。
const ACCRUE_TYPES: [&str; 3] = ["Start", "End", "Prorated"];

/// 管理 MPX 文件中 `AccrueType` 实例的工具方法。
///
/// 对应 Java: `final class AccrueTypeUtility`（私有构造方法禁止实例化，Rust 侧用无字段类型表达）。
pub struct AccrueTypeUtility;

impl AccrueTypeUtility {
    /// 把成本计提方式的文本形式转换成对应的枚举实例，无法识别的值按「按比例分摊」处理。
    ///
    /// 对应 Java: `public static AccrueType getInstance(String type, Locale locale)`
    ///
    /// Java 的 `type` 为 `String`（可能为 `null`，`equalsIgnoreCase(null)` 恒为 `false`），
    /// Rust 侧用 `Option<&str>` 承载，`None` 与「无法识别」等效。
    pub fn get_instance(r#type: Option<&str>, locale: MpxLocale) -> AccrueType {
        let mut result = None;

        for (loop_index, name) in accrue_types(locale).iter().enumerate() {
            if equals_ignore_case(name, r#type) {
                // Java: AccrueType.getInstance(loop + 1)
                result = AccrueType::get_instance((loop_index + 1) as i32);
                break;
            }
        }

        // Java: result == null -> AccrueType.PRORATED
        result.unwrap_or(AccrueType::Prorated)
    }
}

/// 取对应区域设置的成本计提方式名表。
///
/// 对应 Java: `LocaleData.getStringArray(locale, LocaleData.ACCRUE_TYPES)`
///
/// 取舍见文件头：目前一律返回英文默认名表。
fn accrue_types(_locale: MpxLocale) -> &'static [&'static str] {
    &ACCRUE_TYPES
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpxj_enum::MpxjEnum;

    #[test]
    fn accrue_type_round_trips_through_mpx_name() {
        for (index, name) in ACCRUE_TYPES.iter().enumerate() {
            let value = AccrueTypeUtility::get_instance(Some(name), MpxLocale::En);
            assert_eq!(value.value(), (index + 1) as i32);
            assert_eq!(ACCRUE_TYPES[(value.value() - 1) as usize], *name);
        }
    }

    #[test]
    fn accrue_type_ignores_case() {
        assert_eq!(
            AccrueTypeUtility::get_instance(Some("start"), MpxLocale::En),
            AccrueType::Start
        );
        assert_eq!(
            AccrueTypeUtility::get_instance(Some("PRORATED"), MpxLocale::En),
            AccrueType::Prorated
        );
    }

    #[test]
    fn accrue_type_falls_back_to_prorated() {
        assert_eq!(
            AccrueTypeUtility::get_instance(Some("Unknown"), MpxLocale::En),
            AccrueType::Prorated
        );
        assert_eq!(
            AccrueTypeUtility::get_instance(Some(" Start "), MpxLocale::En),
            AccrueType::Prorated
        );
        assert_eq!(
            AccrueTypeUtility::get_instance(None, MpxLocale::En),
            AccrueType::Prorated
        );
    }

    #[test]
    fn accrue_type_uses_english_table_for_non_english_locales() {
        // 取舍：本地化名表未移植，非英文区域设置同样使用英文名表。
        assert_eq!(
            AccrueTypeUtility::get_instance(Some("End"), MpxLocale::De),
            AccrueType::End
        );
    }
}
