//! 对应 Java: `org/mpxj/FieldType.java`

use crate::data_type::DataType;
use crate::field_type_class::FieldTypeClass;
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
/// - Java 的 `getName(Locale)` 重载未移植：reader 链路不使用本地化字段名，
///   如需使用可后续补上。目前只保留英文名 [`FieldType::get_name`]。
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

    /// 返回该字段的数据类型。
    ///
    /// 对应 Java: `DataType getDataType()`
    fn get_data_type(&self) -> DataType;

    /// 返回关联的单位字段（如果有）。
    ///
    /// 对应 Java: `FieldType getUnitsType()`
    fn get_units_type(&self) -> Option<Self>;
}
