//! 对应 Java: `org/mpxj/mpx/ModelUtility.java`
//!
//! # 移植取舍（`Object` 字段值）
//!
//! Java 的 `isFieldPopulated(FieldType field, Object value)` 接收任意 `Object`，按字段的
//! `DataType` 强制转换后判断是否为「非默认值」。Rust 侧字段值统一由
//! [`crate::field_value::FieldValue`] 承载，故入参为 `Option<&FieldValue>`。
//!
//! 类型不匹配时 Java 会抛 `ClassCastException`；Rust 侧不 panic，按「有值即视为已填充」
//! 处理（与 Java 中 `!(value instanceof X)` 分支的语义一致）。
//!
//! `DataType::RelationList`：Java 判断 `!((List<Relation>) value).isEmpty()`，Rust 侧对应
//! [`FieldValue::Relations`]（见 `field_value` 文件头）。
//!
//! 未移植的成员：无。

use crate::data_type::DataType;
use crate::field_type::FieldType;
use crate::field_value::FieldValue;

/// 创建 MPX 任务 / 资源模型的公共代码。
///
/// 对应 Java: `final class ModelUtility`（私有构造方法禁止实例化，Rust 侧用无字段类型表达）。
pub struct ModelUtility;

impl ModelUtility {
    /// 判断字段是否被填充了非默认值。
    ///
    /// 对应 Java: `public static boolean isFieldPopulated(FieldType field, Object value)`
    pub fn is_field_populated<F: FieldType>(field: F, value: Option<&FieldValue>) -> bool {
        let Some(value) = value else {
            return false;
        };

        match field.get_data_type() {
            DataType::String => {
                // Java: !(value instanceof String) || !((String) value).isEmpty()
                match value {
                    FieldValue::String(text) => !text.is_empty(),
                    _ => true,
                }
            }
            DataType::Numeric | DataType::Currency => {
                // Java: ((Number) value).doubleValue() != 0.0
                value.as_double().is_none_or(|number| number != 0.0)
            }
            DataType::Duration | DataType::Work => {
                // Java: !(value instanceof Duration) || ((Duration) value).getDuration() != 0.0
                match value {
                    FieldValue::Duration(duration) => duration.get_duration() != 0.0,
                    _ => true,
                }
            }
            DataType::Rate => {
                // Java: ((Rate) value).getAmount() != 0.0
                value
                    .as_rate()
                    .is_none_or(|rate| rate.get_amount() != 0.0)
            }
            DataType::Boolean => {
                // Java: ((Boolean) value).booleanValue()
                value.as_boolean().is_none_or(|flag| flag)
            }
            DataType::RelationList => {
                // Java: populated = !((List<Relation>) value).isEmpty();
                match value {
                    FieldValue::Relations(relations) => !relations.is_empty(),
                    _ => true,
                }
            }
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::duration::Duration;
    use crate::rate::Rate;
    use crate::resource_field::ResourceField;
    use crate::task_field::TaskField;
    use crate::time_unit::TimeUnit;

    #[test]
    fn null_value_is_never_populated() {
        assert!(!ModelUtility::is_field_populated(TaskField::Text1, None));
        assert!(!ModelUtility::is_field_populated(TaskField::Number1, None));
        assert!(!ModelUtility::is_field_populated(TaskField::Milestone, None));
    }

    #[test]
    fn string_field_uses_emptiness() {
        assert!(!ModelUtility::is_field_populated(
            TaskField::Text1,
            Some(&FieldValue::String(String::new()))
        ));
        assert!(ModelUtility::is_field_populated(
            TaskField::Text1,
            Some(&FieldValue::String("text".to_owned()))
        ));
    }

    #[test]
    fn numeric_and_currency_fields_compare_against_zero() {
        assert!(!ModelUtility::is_field_populated(
            TaskField::Number1,
            Some(&FieldValue::Double(0.0))
        ));
        assert!(!ModelUtility::is_field_populated(
            TaskField::Number1,
            Some(&FieldValue::Integer(0))
        ));
        assert!(ModelUtility::is_field_populated(
            TaskField::Number1,
            Some(&FieldValue::Double(1.5))
        ));
        assert!(!ModelUtility::is_field_populated(
            TaskField::Cost,
            Some(&FieldValue::Double(0.0))
        ));
        assert!(ModelUtility::is_field_populated(
            TaskField::Cost,
            Some(&FieldValue::Double(100.0))
        ));
    }

    #[test]
    fn duration_and_work_fields_compare_against_zero() {
        assert!(!ModelUtility::is_field_populated(
            TaskField::Duration,
            Some(&FieldValue::Duration(Duration::get_instance(
                0.0,
                TimeUnit::Days
            )))
        ));
        assert!(ModelUtility::is_field_populated(
            TaskField::Duration,
            Some(&FieldValue::Duration(Duration::get_instance(
                2.0,
                TimeUnit::Days
            )))
        ));
        assert!(!ModelUtility::is_field_populated(
            TaskField::Work,
            Some(&FieldValue::Duration(Duration::get_instance(
                0.0,
                TimeUnit::Hours
            )))
        ));
    }

    #[test]
    fn rate_field_compares_amount_against_zero() {
        assert!(!ModelUtility::is_field_populated(
            ResourceField::StandardRate,
            Some(&FieldValue::Rate(Rate::new(0.0, TimeUnit::Hours)))
        ));
        assert!(ModelUtility::is_field_populated(
            ResourceField::StandardRate,
            Some(&FieldValue::Rate(Rate::new(10.0, TimeUnit::Hours)))
        ));
    }

    #[test]
    fn boolean_field_uses_its_value() {
        assert!(!ModelUtility::is_field_populated(
            TaskField::Milestone,
            Some(&FieldValue::Boolean(false))
        ));
        assert!(ModelUtility::is_field_populated(
            TaskField::Milestone,
            Some(&FieldValue::Boolean(true))
        ));
    }

    #[test]
    fn relation_list_field_treats_any_value_as_populated() {
        // 见文件头：`FieldValue` 无法表达关系列表，有值即视为已填充。
        assert!(ModelUtility::is_field_populated(
            TaskField::Predecessors,
            Some(&FieldValue::Integer(1))
        ));
    }

    #[test]
    fn other_data_types_are_always_populated() {
        let date = NaiveDate::from_ymd_opt(2024, 5, 1).expect("合法日期");
        assert!(ModelUtility::is_field_populated(
            TaskField::Start,
            Some(&FieldValue::Date(date))
        ));
    }

    #[test]
    fn mismatched_value_types_are_treated_as_populated() {
        // Java 会抛 ClassCastException；Rust 侧按「有值即已填充」处理。
        assert!(ModelUtility::is_field_populated(
            TaskField::Text1,
            Some(&FieldValue::Integer(1))
        ));
        assert!(ModelUtility::is_field_populated(
            TaskField::Duration,
            Some(&FieldValue::Integer(1))
        ));
    }
}
