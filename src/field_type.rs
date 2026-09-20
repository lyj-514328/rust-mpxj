//! 对应 Java: `org/mpxj/FieldType.java`

use crate::data_type::DataType;
use crate::field_type_class::FieldTypeClass;
use crate::mpx::locale_utility::MpxLocale;
use crate::mpxj_enum::MpxjEnum;

/// 表示 Task / Resource / Assignment 等实体中某个字段的类型。
///
/// 对应 Java: `public interface FieldType extends MpxjEnum`
///
/// 由 `TaskField`、`ResourceField`、`AssignmentField`、`ProjectField`、
/// `ConstraintField` 五个枚举实现。
///
/// # 与 Java 的差异
///
/// - Java 的 `getUnitsType()` 返回 `FieldType`（接口），这里返回 `Option<Self>`。
///   原因是 Java 中「单位字段」始终与主字段属于同一个枚举
///   （例如 `TaskField.DURATION` 的单位字段是 `TaskField.DURATION_UNITS`），
///   用关联类型可以避免在 trait 里引入类型擦除用的 `FieldKey`。
/// - Java 的 `getName(Locale)` 重载在 Rust 侧命名为 [`FieldType::get_name_for_locale`]
///   （无重载 + 本 crate 惯例：`getVariance` 之类的重载加后缀区分）。Java 的五个枚举
///   各覆写为 `LocaleData.getStringArray(locale, LocaleData.XXX_COLUMNS)[getValue()]`，
///   而 `org.mpxj.LocaleData` 只有英文资源（基类的 `XXX_COLUMNS_ARRAY` +
///   内容为空的 `LocaleData_en`，仓库中**不存在** `LocaleData_<lang>` 变体），
///   `ResourceBundle` 对任何语言都回落到基类，故 Java 对该重载的实现同样是对
///   任何 locale 都返回同一份英文列名；Rust 侧据此给出返回英文名的默认实现。
///   （注意：不要改用 [`crate::mpx::locale_data`] 的 `TASK_NAMES_DATA` /
///   `RESOURCE_NAMES_DATA` —— 那是 `org.mpxj.mpx.LocaleData.TASK_NAMES`（按 MPX 字段号
///   索引，如 `1 => "Name"`），与 `org.mpxj.LocaleData.TASK_COLUMNS`（按 `TaskField.value()`
///   索引，如 `NAME => "Task Name"`）既不同表也不同下标，混用会给出错误的名字。）
pub trait FieldType: MpxjEnum + Copy + Eq + std::hash::Hash + 'static {
    /// 返回该字段所属的实体类型。
    ///
    /// 对应 Java: `FieldTypeClass getFieldTypeClass()`
    fn get_field_type_class(&self) -> FieldTypeClass;

    /// 返回该字段的 Java 枚举常量名（例如 `"DURATION_UNITS"`）。
    ///
    /// 对应 Java 枚举自带的 `name()`
    fn name(&self) -> &'static str;

    /// 返回该字段的英文显示名（例如 `"Duration"`）。
    ///
    /// 对应 Java: `String getName()`
    ///
    /// Java 侧该值来自 `LocaleData` 的英文列名表，表中未定义的项为 `null`；
    /// 这里用空字符串 `""` 表示该情况。
    fn get_name(&self) -> &'static str;

    /// 返回该字段在给定区域设置下的显示名。
    ///
    /// 对应 Java: `String getName(Locale locale)`
    ///
    /// Java 的五个枚举都把该重写为
    /// `LocaleData.getStringArray(locale, LocaleData.TASK_COLUMNS / RESOURCE_COLUMNS / ...)`
    /// 后按 `getValue()` 取下标（越界返回 `null`，Rust 侧为 `""`）。
    /// 由于 `org.mpxj.LocaleData` 只带英文列名表（详见 type 文档），该查找对任何
    /// locale 都返回英文名，因此默认实现忽略 locale、直接返回 [`FieldType::get_name`]。
    fn get_name_for_locale(&self, locale: MpxLocale) -> &'static str {
        let _ = locale;
        self.get_name()
    }

    /// 返回该字段的数据类型。
    ///
    /// 对应 Java: `DataType getDataType()`
    fn get_data_type(&self) -> DataType;

    /// 返回关联的单位字段（如果有）。
    ///
    /// 对应 Java: `FieldType getUnitsType()`
    fn get_units_type(&self) -> Option<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_key::FieldKey;
    use crate::resource_field::ResourceField;
    use crate::task_field::TaskField;

    /// Java: `LocaleData.getStringArray(locale, LocaleData.XXX_COLUMNS)` 对任何 locale
    /// 都会回落到英文基类（仓库内没有 `LocaleData_<lang>` 变体），因此
    /// `getName(locale)` 的结果与 `getName()` 相同。
    #[test]
    fn get_name_for_locale_matches_english_for_every_locale() {
        let name = FieldKey::Task(TaskField::Name);
        for locale in MpxLocale::ALL {
            assert_eq!(
                FieldKey::get_name_for_locale(&name, locale),
                name.get_name(),
                "{locale}"
            );
        }
        assert_eq!(name.get_name_for_locale(MpxLocale::En), "Task Name");

        let duration = FieldKey::Task(TaskField::Duration);
        assert_eq!(duration.get_name_for_locale(MpxLocale::Zh), "Duration");

        // 资源字段同样回落到英文列名表
        let resource = FieldKey::Resource(ResourceField::MaxUnits);
        assert_eq!(resource.get_name_for_locale(MpxLocale::De), "Max Units");
    }
}
