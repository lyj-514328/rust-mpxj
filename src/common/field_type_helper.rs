//! 对应 Java: `org/mpxj/common/FieldTypeHelper.java`
//!
//! `FieldType` 相关的工具方法：字段号 ↔ `FieldType` 的互换、文本型字段的回退映射、
//! 以及「字段值是否为默认值」的判断。
//!
//! # 与 Java 的差异（Rust 特有的接线）
//!
//! - **`FieldType` → [`MppFieldType`]**：Java 的 `FieldType` 是接口，可同时容纳
//!   `TaskField` / `ResourceField` / … 与 `UserDefinedField`；Rust 侧用
//!   [`crate::common::mpp_field_type::MppFieldType`] 表达这两种情形。
//! - **`null` → `Option`**：`getFieldID(null)` 在 Java 中返回 `-1`，Rust 侧以
//!   `Option<&MppFieldType>` 表达该可空入参。
//! - **`getPlaceholder`（跳过的 Java 方法）**：Java 的 `getInstance` 在无法识别字段号时
//!   返回一个匿名 `FieldType` 实现 —— `getFieldTypeClass()` 恒为 `FieldTypeClass.UNKNOWN`、
//!   `name()` 恒为 `"UNKNOWN"`、`getValue()` 为字段号 `fieldID`、
//!   `getName()` 为 `"Unknown"`（`type` 为 `null` 时）或
//!   `"Unknown " + 类型简单名 + "(" + fieldID + ")"`、`getDataType()` 与 `getUnitsType()`
//!   恒为 `null`。Rust 的 `MppFieldType` 只有内置字段与自定义字段两种变体，
//!   `FieldKey` 也没有 `UNKNOWN` 变体，**无法表达该占位对象**，因此所有原本返回占位对象的
//!   分支统一返回 `None`（调用方若要区分「未知类型」与「无字段」，需另行处理）。
//! - **`Object value` → `Option<&FieldValue>`**：Java 的 `valueIsNotDefault` 接收任意对象，
//!   按 `DataType` 强转（类型不符时抛 `ClassCastException` / `dataType` 为 `null` 时抛
//!   `NullPointerException`）；Rust 侧用 [`FieldValue`] 表达，遇到不符合预期的类型一律返回
//!   「是默认值」（不 `panic`），`dataType` 缺失时保留 Java 初始化值 `true`。
//!
//! # 依赖
//!
//! `getFieldID` / `getInstance` 需要兄弟模块 `MPPResourceField` / `MPPAssignmentField` /
//! `MPPProjectField` / `MPPConstraintField`（与 `MPPTaskField` 同批移植）的公开成员：
//! `RESOURCE_FIELD_BASE` / `ASSIGNMENT_FIELD_BASE` / `PROJECT_FIELD_BASE` /
//! `CONSTRAINT_FIELD_BASE` 常量以及 `get_id`、`get_instance_with_data_type`（约束字段为
//! `get_instance`）方法。
//!
//! 未移植的 Java 方法：`getPlaceholder`（见上）。

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::mpp_assignment_field::MPPAssignmentField;
use crate::common::mpp_constraint_field::MPPConstraintField;
use crate::common::mpp_field_type::MppFieldType;
use crate::common::mpp_project_field::MPPProjectField;
use crate::common::mpp_resource_field::MPPResourceField;
use crate::common::mpp_task_field::MPPTaskField;
use crate::data_type::DataType;
use crate::field_key::FieldKey;
use crate::field_type::FieldType;
use crate::field_type_class::FieldTypeClass;
use crate::field_value::FieldValue;
use crate::priority::Priority;
use crate::project_file::ProjectFile;
use crate::task_field::TaskField;

/// `FieldType` 相关工具。
///
/// 对应 Java: `public final class FieldTypeHelper`
pub struct FieldTypeHelper;

impl FieldTypeHelper {
    /// 根据 MPXJ 的字段类型取 MPP9/MPP12 使用的字段号。
    ///
    /// 对应 Java: `public static int getFieldID(FieldType type)`
    ///
    /// `None` 对应 Java 的 `null`（返回 `-1`）。
    pub fn get_field_id(value: Option<&MppFieldType>) -> i32 {
        // Java: if (type == null) { result = -1; }
        let Some(value) = value else {
            return -1;
        };

        // Java: switch (type.getFieldTypeClass()) { ... default: result = -1; }
        match field_type_class_of(value) {
            Some(FieldTypeClass::Task) => {
                MPPTaskField::TASK_FIELD_BASE | MPPTaskField::get_id(value)
            }
            Some(FieldTypeClass::Resource) => {
                MPPResourceField::RESOURCE_FIELD_BASE | MPPResourceField::get_id(value)
            }
            Some(FieldTypeClass::Assignment) => {
                MPPAssignmentField::ASSIGNMENT_FIELD_BASE | MPPAssignmentField::get_id(value)
            }
            Some(FieldTypeClass::Project) => {
                MPPProjectField::PROJECT_FIELD_BASE | MPPProjectField::get_id(value)
            }
            // Java: default: result = -1;（Constraint / Unknown，以及自定义字段的 type class 为 null）
            _ => -1,
        }
    }

    /// 根据 MPP9/MPP12/MPP14 文件中的字段号取字段类型。
    ///
    /// 对应 Java: `public static final FieldType getInstance(ProjectFile project, int fieldID)`
    pub fn get_instance(project: &Rc<RefCell<ProjectFile>>, field_id: i32) -> Option<MppFieldType> {
        // Java: return getInstance(project, fieldID, DataType.CUSTOM);
        Self::get_instance_with_data_type(project, field_id, DataType::Custom)
    }

    /// 根据 MPP9/MPP12/MPP14 文件中的字段号取字段类型（自定义字段可指定数据类型）。
    ///
    /// 对应 Java: `public static final FieldType getInstance(ProjectFile project, int fieldID, DataType customFieldDataType)`
    pub fn get_instance_with_data_type(
        project: &Rc<RefCell<ProjectFile>>,
        field_id: i32,
        custom_field_data_type: DataType,
    ) -> Option<MppFieldType> {
        // Java: if (fieldID == -1) { return null; }
        if field_id == -1 {
            return None;
        }

        // Java: int prefix = fieldID & 0xFFFF0000; int index = fieldID & 0x0000FFFF;
        // （Java 的 `int` 为 32 位补码，`0xFFFF0000` 无法直接写成 Rust 的 `i32` 字面量）
        let prefix = field_id & (0xFFFF_0000u32 as i32);
        let index = field_id & 0x0000FFFF;

        match prefix {
            // Java: result = MPPTaskField.getInstance(project, index, customFieldDataType);
            //       if (result == null) { result = getPlaceholder(TaskField.class, index); }
            MPPTaskField::TASK_FIELD_BASE => {
                MPPTaskField::get_instance_with_data_type(project, index, custom_field_data_type)
            }
            MPPResourceField::RESOURCE_FIELD_BASE => {
                MPPResourceField::get_instance_with_data_type(project, index, custom_field_data_type)
            }
            MPPAssignmentField::ASSIGNMENT_FIELD_BASE => MPPAssignmentField::get_instance_with_data_type(
                project,
                index,
                custom_field_data_type,
            ),
            // Java: result = MPPConstraintField.getInstance(index);（Java 无 project 参数，
            // Rust 侧为统一签名保留了该参数，约束字段没有自定义字段分支）
            MPPConstraintField::CONSTRAINT_FIELD_BASE => {
                MPPConstraintField::get_instance(project, index)
            }
            MPPProjectField::PROJECT_FIELD_BASE => {
                MPPProjectField::get_instance_with_data_type(project, index, custom_field_data_type)
            }
            // Java: default: result = getPlaceholder(null, index);（占位对象无法表达，见模块说明）
            _ => None,
        }
    }

    /// 把「文本版字段」（如 Start Text）映射回需要处理的非文本版字段。
    ///
    /// 对应 Java: `public static FieldType mapTextFields(FieldType field)`
    ///
    /// Java 仅处理 `TaskField`，其余类型原样返回。
    pub fn map_text_fields(field: MppFieldType) -> MppFieldType {
        match field {
            MppFieldType::Builtin(FieldKey::Task(task_field)) => match task_field {
                // Java: case START_TEXT: field = TaskField.START; break;
                TaskField::StartText => MppFieldType::Builtin(FieldKey::Task(TaskField::Start)),
                // Java: case FINISH_TEXT: field = TaskField.FINISH; break;
                TaskField::FinishText => MppFieldType::Builtin(FieldKey::Task(TaskField::Finish)),
                // Java: case DURATION_TEXT: field = TaskField.DURATION; break;
                TaskField::DurationText => MppFieldType::Builtin(FieldKey::Task(TaskField::Duration)),
                // Java: default: break;（原样返回）
                other => MppFieldType::Builtin(FieldKey::Task(other)),
            },
            // Java: if (!(field instanceof TaskField)) { return field; }
            other => other,
        }
    }

    /// 判断给定值对该字段类型而言是否为「非默认值」。
    ///
    /// 对应 Java: `public static final boolean valueIsNotDefault(FieldType type, Object value)`
    ///
    /// `None` 对应 Java 的 `null`（返回 `false`，即「是默认值」）。
    pub fn value_is_not_default(field_type: &MppFieldType, value: Option<&FieldValue>) -> bool {
        // Java: boolean result = true; if (value == null) { result = false; }
        let Some(value) = value else {
            return false;
        };

        // Java: switch (type.getDataType()) { ... }
        // Java 在 data type 为 null（自定义字段未设置时）会抛 NullPointerException；
        // Rust 侧保留初始化值 true（即「非默认值」，不 panic）。
        match data_type_of(field_type) {
            // Java: result = !(value.toString()).isEmpty();
            // 非字符串值（数字、布尔、日期等）的 toString 结果恒非空。
            Some(DataType::String) | Some(DataType::Notes) => match value {
                FieldValue::String(text) => !text.is_empty(),
                _ => true,
            },

            // Java: result = ((Number) value).doubleValue() != 0.0;
            // 非数值类型在 Java 中抛 ClassCastException，Rust 侧按「是默认值」处理。
            Some(DataType::Numeric)
            | Some(DataType::Currency)
            | Some(DataType::Percentage)
            | Some(DataType::Units)
            | Some(DataType::Integer)
            | Some(DataType::Short) => value.as_double().map_or(false, |value| value != 0.0),

            // Java: baseline durations can have string values
            //       if (value instanceof String) { result = !((String) value).isEmpty(); }
            //       else { result = ((Duration) value).getDuration() != 0.0; }
            Some(DataType::Work) | Some(DataType::Duration) => match value {
                FieldValue::String(text) => !text.is_empty(),
                _ => value
                    .as_duration()
                    .map_or(false, |duration| duration.get_duration() != 0.0),
            },

            // Java: result = ((Rate) value).getAmount() != 0.0;
            Some(DataType::Rate) => value
                .as_rate()
                .map_or(false, |rate| rate.get_amount() != 0.0),

            // Java: result = ((Boolean) value).booleanValue();
            Some(DataType::Boolean) => value.as_boolean().unwrap_or(false),

            // Java: result = ((Priority) value).getValue() != Priority.MEDIUM;
            // Rust 的 `FieldValue` 没有 `Priority` 变体，优先级以整数存储。
            Some(DataType::Priority) => value
                .as_integer()
                .map_or(false, |priority| priority != Priority::MEDIUM),

            // Java: default: break;（保留初始化值 true）
            _ => true,
        }
    }
}

/// 取字段所属的实体类型；自定义字段无类型时返回 `None`。
///
/// Java: `FieldType.getFieldTypeClass()`（`UserDefinedField` 可能返回 `null`）
fn field_type_class_of(field: &MppFieldType) -> Option<FieldTypeClass> {
    match field {
        MppFieldType::Builtin(key) => Some(key.get_field_type_class()),
        MppFieldType::Custom(field) => field
            .try_borrow()
            .ok()
            .and_then(|field| field.get_field_type_class()),
    }
}

/// 取字段的数据类型；自定义字段无类型时返回 `None`（Java 为 `null`）。
fn data_type_of(field: &MppFieldType) -> Option<DataType> {
    match field {
        MppFieldType::Builtin(key) => Some(key.get_data_type()),
        MppFieldType::Custom(field) => field
            .try_borrow()
            .ok()
            .and_then(|field| field.get_data_type()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duration::Duration;
    use crate::rate::Rate;
    use crate::time_unit::TimeUnit;

    /// 构造一个项目（自定义字段用）。
    fn project() -> Rc<RefCell<ProjectFile>> {
        ProjectFile::new()
    }

    /// 构造一个内置任务字段。
    fn task_field(field: TaskField) -> MppFieldType {
        MppFieldType::Builtin(FieldKey::Task(field))
    }

    /// 构造一个指定数据类型的自定义字段。
    fn custom_field(project: &Rc<RefCell<ProjectFile>>, data_type: DataType) -> MppFieldType {
        MPPTaskField::get_instance_with_data_type(project, 0x8000, data_type)
            .expect("应创建自定义字段")
    }

    #[test]
    fn get_field_id_matches_java() {
        assert_eq!(FieldTypeHelper::get_field_id(None), -1);

        // Java: MPPTaskField.TASK_FIELD_BASE | MPPTaskField.getID(type)
        // NAME 只出现在 FIELD_ARRAY[14]
        assert_eq!(
            FieldTypeHelper::get_field_id(Some(&task_field(TaskField::Name))),
            MPPTaskField::TASK_FIELD_BASE | 14
        );
        // START 出现在 35 与 1283，ID_ARRAY 取后者
        assert_eq!(
            FieldTypeHelper::get_field_id(Some(&task_field(TaskField::Start))),
            MPPTaskField::TASK_FIELD_BASE | 1283
        );

        // Java: default: result = -1;（ConstraintField 不在 switch 中）
        let constraint = MppFieldType::Builtin(FieldKey::Constraint(
            crate::constraint_field::ConstraintField::UniqueId,
        ));
        assert_eq!(FieldTypeHelper::get_field_id(Some(&constraint)), -1);

        // 自定义任务字段：Java 走 TASK 分支，getID 返回其 unique ID
        let custom = custom_field(&project(), DataType::Custom);
        assert_eq!(
            FieldTypeHelper::get_field_id(Some(&custom)),
            MPPTaskField::TASK_FIELD_BASE | (MPPTaskField::TASK_FIELD_BASE + 0x8000)
        );
    }

    #[test]
    fn get_instance_handles_task_prefix_and_unknown_ids() {
        let project = project();

        // Java: fieldID == -1 → null
        assert!(FieldTypeHelper::get_instance(&project, -1).is_none());

        // Java: prefix == MPPTaskField.TASK_FIELD_BASE → MPPTaskField.getInstance(...)
        let field = FieldTypeHelper::get_instance(&project, MPPTaskField::TASK_FIELD_BASE | 14)
            .expect("应映射到 TaskField.NAME");
        assert_eq!(field.as_builtin(), Some(FieldKey::Task(TaskField::Name)));

        // Java: 前缀未知 → getPlaceholder(null, index)（Rust 无法表达，返回 None）
        assert!(FieldTypeHelper::get_instance(&project, 0x12340000 | 5).is_none());
    }

    #[test]
    fn map_text_fields_matches_java() {
        assert_eq!(
            FieldTypeHelper::map_text_fields(task_field(TaskField::StartText)).as_builtin(),
            Some(FieldKey::Task(TaskField::Start))
        );
        assert_eq!(
            FieldTypeHelper::map_text_fields(task_field(TaskField::FinishText)).as_builtin(),
            Some(FieldKey::Task(TaskField::Finish))
        );
        assert_eq!(
            FieldTypeHelper::map_text_fields(task_field(TaskField::DurationText)).as_builtin(),
            Some(FieldKey::Task(TaskField::Duration))
        );

        // 其它任务字段原样返回
        assert_eq!(
            FieldTypeHelper::map_text_fields(task_field(TaskField::Name)).as_builtin(),
            Some(FieldKey::Task(TaskField::Name))
        );

        // 非 TaskField（含自定义字段）原样返回
        let custom = custom_field(&project(), DataType::Custom);
        assert_eq!(
            FieldTypeHelper::map_text_fields(custom.clone()).to_key(),
            custom.to_key()
        );
    }

    #[test]
    fn value_is_not_default_matches_java() {
        let project = project();

        // Java: value == null → false
        assert!(!FieldTypeHelper::value_is_not_default(
            &task_field(TaskField::Name),
            None
        ));

        // STRING / NOTES：非空即非默认
        let name = task_field(TaskField::Name);
        assert!(!FieldTypeHelper::value_is_not_default(
            &name,
            Some(&FieldValue::String(String::new()))
        ));
        assert!(FieldTypeHelper::value_is_not_default(
            &name,
            Some(&FieldValue::String("Task".to_owned()))
        ));
        let notes = task_field(TaskField::Notes);
        assert!(!FieldTypeHelper::value_is_not_default(
            &notes,
            Some(&FieldValue::String(String::new()))
        ));

        // NUMERIC / CURRENCY / PERCENTAGE / INTEGER：非 0 即非默认
        for field in [
            task_field(TaskField::Cost),
            task_field(TaskField::PercentComplete),
            task_field(TaskField::Id),
        ] {
            assert!(!FieldTypeHelper::value_is_not_default(
                &field,
                Some(&FieldValue::Integer(0))
            ));
            assert!(FieldTypeHelper::value_is_not_default(
                &field,
                Some(&FieldValue::Double(1.5))
            ));
        }

        // WORK / DURATION：字符串按非空判断，否则按工期是否为 0 判断
        let duration = task_field(TaskField::Duration);
        assert!(!FieldTypeHelper::value_is_not_default(
            &duration,
            Some(&FieldValue::String(String::new()))
        ));
        assert!(FieldTypeHelper::value_is_not_default(
            &duration,
            Some(&FieldValue::String("2 days".to_owned()))
        ));
        assert!(!FieldTypeHelper::value_is_not_default(
            &duration,
            Some(&FieldValue::Duration(Duration::get_instance(
                0.0,
                TimeUnit::Days
            )))
        ));
        assert!(FieldTypeHelper::value_is_not_default(
            &duration,
            Some(&FieldValue::Duration(Duration::get_instance(
                2.0,
                TimeUnit::Days
            )))
        ));

        // RATE：费率金额非 0 即非默认
        let rate_field = custom_field(&project, DataType::Rate);
        assert!(!FieldTypeHelper::value_is_not_default(
            &rate_field,
            Some(&FieldValue::Rate(Rate::new(0.0, TimeUnit::Hours)))
        ));
        assert!(FieldTypeHelper::value_is_not_default(
            &rate_field,
            Some(&FieldValue::Rate(Rate::new(10.0, TimeUnit::Hours)))
        ));

        // BOOLEAN：值本身
        let milestone = task_field(TaskField::Milestone);
        assert!(!FieldTypeHelper::value_is_not_default(
            &milestone,
            Some(&FieldValue::Boolean(false))
        ));
        assert!(FieldTypeHelper::value_is_not_default(
            &milestone,
            Some(&FieldValue::Boolean(true))
        ));

        // PRIORITY：非 MEDIUM 即非默认
        let priority = task_field(TaskField::Priority);
        assert!(!FieldTypeHelper::value_is_not_default(
            &priority,
            Some(&FieldValue::Integer(Priority::MEDIUM))
        ));
        assert!(FieldTypeHelper::value_is_not_default(
            &priority,
            Some(&FieldValue::Integer(Priority::HIGH))
        ));

        // 未列举的数据类型（如 DATE）：Java 保留初始化值 true
        assert!(FieldTypeHelper::value_is_not_default(
            &task_field(TaskField::Start),
            Some(&FieldValue::String(String::new()))
        ));
    }
}
