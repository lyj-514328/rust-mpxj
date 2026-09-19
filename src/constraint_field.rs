//! 对应 Java: `org/mpxj/ConstraintField.java`

use crate::data_type::DataType;
use crate::field_type::FieldType;
use crate::field_type_class::FieldTypeClass;
use crate::mpxj_enum::MpxjEnum;

/// 表示约束（Constraint）实体中的某个字段。
///
/// 对应 Java: `public enum ConstraintField implements FieldType`
///
/// 变体顺序与 Java 声明顺序一致，因此 `*self as i32` 等于 Java 的
/// `getValue()`（Java 用静态初始化块按声明顺序赋值）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstraintField {
    UniqueId,
    Task1,
    Task2,
}

/// Java `LocaleData.CONSTRAINT_COLUMNS_ARRAY` 的英文列名表，按 `getValue()` 下标排列。
///
/// Java 侧未赋值的下标用空字符串 `""` 表示；经核对本表无空缺。
const CONSTRAINT_COLUMN_NAMES: [&str; ConstraintField::MAX_VALUE] = ["Unique ID", "Task 1", "Task 2"];

impl ConstraintField {
    /// 变体总数。
    ///
    /// 对应 Java: `public static final int MAX_VALUE`
    pub const MAX_VALUE: usize = 3;

    /// 全部取值，按声明顺序（即 `getValue()` 升序）排列。
    pub const ALL: [ConstraintField; ConstraintField::MAX_VALUE] =
        [ConstraintField::UniqueId, ConstraintField::Task1, ConstraintField::Task2];

    /// 根据 int 值取枚举实例，越界时返回 `None`。
    ///
    /// 对应 Java: `public static ConstraintField getInstance(int type)`
    /// （Java 越界返回 `null`）。
    pub fn get_instance(value: i32) -> Option<ConstraintField> {
        if value >= 0 && (value as usize) < Self::MAX_VALUE {
            Some(Self::ALL[value as usize])
        } else {
            None
        }
    }
}

impl FieldType for ConstraintField {
    /// 对应 Java: `public FieldTypeClass getFieldTypeClass()`
    fn get_field_type_class(&self) -> FieldTypeClass {
        FieldTypeClass::Constraint
    }

    /// 对应 Java 枚举自带的 `name()`
    fn name(&self) -> &'static str {
        match self {
            ConstraintField::UniqueId => "UNIQUE_ID",
            ConstraintField::Task1 => "TASK1",
            ConstraintField::Task2 => "TASK2",
        }
    }

    /// 对应 Java: `public String getName()`
    fn get_name(&self) -> &'static str {
        CONSTRAINT_COLUMN_NAMES[*self as usize]
    }

    /// 对应 Java: `public DataType getDataType()`
    fn get_data_type(&self) -> DataType {
        match self {
            ConstraintField::UniqueId | ConstraintField::Task1 | ConstraintField::Task2 => {
                DataType::Integer
            }
        }
    }

    /// 对应 Java: `public FieldType getUnitsType()`（Java 返回 `null`）
    fn get_units_type(&self) -> Option<Self> {
        None
    }
}

impl MpxjEnum for ConstraintField {
    /// 对应 Java: `public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl std::fmt::Display for ConstraintField {
    /// 对应 Java: `public String toString()`
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constraint_field_round_trips() {
        for item in ConstraintField::ALL {
            assert_eq!(ConstraintField::get_instance(item.value()), Some(item));
        }
    }

    #[test]
    fn constraint_field_out_of_range_returns_none() {
        assert_eq!(ConstraintField::get_instance(-1), None);
        assert_eq!(ConstraintField::get_instance(ConstraintField::MAX_VALUE as i32), None);
    }

    #[test]
    fn constraint_field_max_value_matches_variant_count() {
        assert_eq!(ConstraintField::ALL.len(), ConstraintField::MAX_VALUE);
    }
}
