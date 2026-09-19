//! 对应 Java: `org/mpxj/mpx/ConstraintTypeUtility.java`
//!
//! # 移植取舍（本地化名表 / `ResourceBundle`）
//!
//! Java 用 `LocaleData.getStringArray(locale, LocaleData.CONSTRAINT_TYPES)` 从
//! `ResourceBundle` 取本地化的约束类型名表；Rust 标准库没有 `ResourceBundle`，
//! 各语言的资源表由同包的 `locale_data_XX` 模块（并行移植）承载。
//! 本模块不依赖它们，一律使用英文默认名表（`LocaleData.CONSTRAINT_TYPES_DATA`），
//! `locale` 参数保留以对齐 Java 签名（详见 `locale_utility` 文件头的取舍说明）。
//!
//! 未移植的成员：无。

use crate::constraint_type::ConstraintType;
use crate::mpx::locale_utility::{equals_ignore_case, MpxLocale};

/// 英文（默认）约束类型名表。
///
/// 对应 Java: `LocaleData.CONSTRAINT_TYPES_DATA`（顺序即 `ConstraintType` 的 int 值 0..7）。
const CONSTRAINT_TYPES: [&str; 8] = [
    "As Soon As Possible",
    "As Late As Possible",
    "Must Start On",
    "Must Finish On",
    "Start No Earlier Than",
    "Start No Later Than",
    "Finish No Earlier Than",
    "Finish No Later Than",
];

/// 管理 MPX 文件中 `ConstraintType` 实例的工具方法。
///
/// 对应 Java: `final class ConstraintTypeUtility`（私有构造方法禁止实例化，Rust 侧用无字段类型表达）。
pub struct ConstraintTypeUtility;

impl ConstraintTypeUtility {
    /// 把约束类型的文本形式转换成对应的枚举实例，无法识别的值按「越早越好」处理。
    ///
    /// 对应 Java: `public static ConstraintType getInstance(Locale locale, String type)`
    ///
    /// Java 的 `type` 为 `String`（可能为 `null`，`equalsIgnoreCase(null)` 恒为 `false`），
    /// Rust 侧用 `Option<&str>` 承载，`None` 与「无法识别」等效。
    pub fn get_instance(locale: MpxLocale, r#type: Option<&str>) -> ConstraintType {
        let mut index = 0;

        for (loop_index, name) in constraint_types(locale).iter().enumerate() {
            if equals_ignore_case(name, r#type) {
                index = loop_index;
                break;
            }
        }

        // Java: ConstraintType.getInstance(index)，index 越界时回落 AS_SOON_AS_POSSIBLE
        ConstraintType::get_instance(index as i32)
    }
}

/// 取对应区域设置的约束类型名表。
///
/// 对应 Java: `LocaleData.getStringArray(locale, LocaleData.CONSTRAINT_TYPES)`
///
/// 取舍见文件头：目前一律返回英文默认名表。
fn constraint_types(_locale: MpxLocale) -> &'static [&'static str] {
    &CONSTRAINT_TYPES
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpxj_enum::MpxjEnum;

    #[test]
    fn constraint_type_round_trips_through_mpx_name() {
        for name in CONSTRAINT_TYPES {
            let value = ConstraintTypeUtility::get_instance(MpxLocale::En, Some(name));
            // 名表下标即 ConstraintType 的 int 值，转换回来应与名表一一对应。
            assert_eq!(CONSTRAINT_TYPES[value.value() as usize], name);
            assert_eq!(
                ConstraintTypeUtility::get_instance(MpxLocale::En, Some(name)),
                value
            );
        }
    }

    #[test]
    fn constraint_type_ignores_case_and_unrecognised_values() {
        assert_eq!(
            ConstraintTypeUtility::get_instance(MpxLocale::En, Some("must start on")),
            ConstraintType::MustStartOn
        );
        assert_eq!(
            ConstraintTypeUtility::get_instance(MpxLocale::En, Some("MUST START ON")),
            ConstraintType::MustStartOn
        );
        // Java: 无法识别的值与 null 都回落到「越早越好」
        assert_eq!(
            ConstraintTypeUtility::get_instance(MpxLocale::En, Some("Unknown")),
            ConstraintType::AsSoonAsPossible
        );
        assert_eq!(
            ConstraintTypeUtility::get_instance(MpxLocale::En, Some("")),
            ConstraintType::AsSoonAsPossible
        );
        assert_eq!(
            ConstraintTypeUtility::get_instance(MpxLocale::En, None),
            ConstraintType::AsSoonAsPossible
        );
    }

    #[test]
    fn constraint_type_without_english_name_falls_back() {
        // StartOn(8) / FinishOn(9) 在英文名表中没有对应项（名表只有 8 项）。
        assert_eq!(
            ConstraintTypeUtility::get_instance(MpxLocale::En, Some("Start On")),
            ConstraintType::AsSoonAsPossible
        );
    }

    #[test]
    fn constraint_type_uses_english_table_for_non_english_locales() {
        // 取舍：本地化名表未移植，非英文区域设置同样使用英文名表。
        assert_eq!(
            ConstraintTypeUtility::get_instance(MpxLocale::Zh, Some("Must Start On")),
            ConstraintType::MustStartOn
        );
    }
}
