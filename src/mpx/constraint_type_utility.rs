//! 对应 Java: `org/mpxj/mpx/ConstraintTypeUtility.java`
//!
//! # 区域设置名表
//!
//! Java 用 `LocaleData.getStringArray(locale, LocaleData.CONSTRAINT_TYPES)` 从 `ResourceBundle`
//! 取本地化的约束类型名表（非英语语言由 `LocaleData_de` 等资源包覆盖）；Rust 侧同一份数据
//! 由 [`LocaleData`] 承载（本地化覆盖来自同包的 `locale_data_XX` 模块），故本模块按 Java 的
//! 方式**按区域设置取名表**。
//!
//! 未移植的成员：无。

use crate::constraint_type::ConstraintType;
use crate::mpx::locale_data::LocaleData;
use crate::mpx::locale_utility::{equals_ignore_case, MpxLocale};

/// 英文（默认）约束类型名表。
///
/// 对应 Java: `LocaleData.CONSTRAINT_TYPES_DATA`（顺序即 `ConstraintType` 的 int 值 0..7）。
const CONSTRAINT_TYPES: [Option<&str>; 8] = [
    Some("As Soon As Possible"),
    Some("As Late As Possible"),
    Some("Must Start On"),
    Some("Must Finish On"),
    Some("Start No Earlier Than"),
    Some("Start No Later Than"),
    Some("Finish No Earlier Than"),
    Some("Finish No Later Than"),
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
            // Java 的名表元素不会是 null；Rust 侧对 `None` 元素直接跳过。
            if let Some(name) = name {
                if equals_ignore_case(name, r#type) {
                    index = loop_index;
                    break;
                }
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
/// 键缺失时回落英文默认名表（Java 的 `ResourceBundle` 同样会回落到基类资源）。
fn constraint_types(locale: MpxLocale) -> &'static [Option<&'static str>] {
    LocaleData::get_string_array(locale, LocaleData::CONSTRAINT_TYPES).unwrap_or(&CONSTRAINT_TYPES)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpxj_enum::MpxjEnum;

    #[test]
    fn constraint_type_round_trips_through_mpx_name() {
        for name in CONSTRAINT_TYPES {
            let name = name.expect("英文名表没有空元素");
            let value = ConstraintTypeUtility::get_instance(MpxLocale::En, Some(name));
            // 名表下标即 ConstraintType 的 int 值，转换回来应与名表一一对应。
            assert_eq!(CONSTRAINT_TYPES[value.value() as usize], Some(name));
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
    fn constraint_type_uses_the_locale_table() {
        // Java: LocaleData_de.CONSTRAINT_TYPES_DATA[4] = "Anfang nicht früher als"
        assert_eq!(
            ConstraintTypeUtility::get_instance(MpxLocale::De, Some("Anfang nicht früher als")),
            ConstraintType::StartNoEarlierThan
        );
        assert_eq!(
            ConstraintTypeUtility::get_instance(MpxLocale::De, Some("So früh wie möglich")),
            ConstraintType::AsSoonAsPossible
        );
        // 英文名表里的值在德语名表中不存在，回落「越早越好」
        assert_eq!(
            ConstraintTypeUtility::get_instance(MpxLocale::De, Some("Must Start On")),
            ConstraintType::AsSoonAsPossible
        );
    }
}
