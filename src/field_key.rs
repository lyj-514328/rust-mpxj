//! 对应 Java: `org/mpxj/FieldType` 的运行时类型擦除形式。
//!
//! Java 侧 `AbstractFieldContainer` 用 `Map<FieldType, Object>` 存字段，
//! 键是 `FieldType` 接口，实际取值来自 `TaskField` / `ResourceField` /
//! `AssignmentField` / `ProjectField` / `ConstraintField` 五个枚举。
//! Rust 的 `HashMap` 需要一个具体键类型，因此这里引入 `FieldKey`
//! 作为这五个枚举的统一载体 —— 语义与 Java 的 `FieldType` 接口等价。

use std::cell::RefCell;
use std::rc::Rc;

use crate::assignment_field::AssignmentField;
use crate::constraint_field::ConstraintField;
use crate::data_type::DataType;
use crate::field_type::FieldType;
use crate::field_type_class::FieldTypeClass;
use crate::mpxj_enum::MpxjEnum;
use crate::project_field::ProjectField;
use crate::resource_field::ResourceField;
use crate::task_field::TaskField;
use crate::user_defined_field::UserDefinedField;

/// 对应 Java: `UserDefinedField` —— 它在字段表中作为键的等价形式。
///
/// Java 的 `AbstractFieldContainer` 用 `FieldType` 对象作键，`UserDefinedField`
/// 是其中之一；而它的 `equals` / `hashCode` **只依据 `m_uniqueID`**（见 Java 源码
/// `UserDefinedField.equals` / `hashCode`）。因此本类型同样以 `unique_id` 作为
/// 唯一的等价依据 —— `PartialEq` / `Hash` 均手写为只比较该字段，语义与 Java 一致。
///
/// 除 `unique_id` 外还携带字段类型与数据类型（都是 `Copy` 的小枚举），
/// 供 [`FieldType`] 实现回答 `get_field_type_class()` / `get_data_type()`。
///
/// # 为何不直接持有 `Rc<RefCell<UserDefinedField>>`
///
/// [`FieldKey`] 需要 `Copy + Eq + Hash`（它被当作 `HashMap` 键、并按值在
/// 实体与容器之间传递）。持有 `Rc` 会丢掉 `Copy`，进而波及全 crate 的调用点。
#[derive(Debug, Clone, Copy)]
pub struct CustomFieldKey {
    unique_id: i32,
    field_type_class: FieldTypeClass,
    data_type: DataType,
}

impl CustomFieldKey {
    /// 由 unique ID、字段类型、数据类型构造。
    ///
    /// 对应 Java: `UserDefinedField.getUniqueID()` / `getFieldTypeClass()` / `getDataType()`
    pub fn new(unique_id: i32, field_type_class: FieldTypeClass, data_type: DataType) -> Self {
        Self {
            unique_id,
            field_type_class,
            data_type,
        }
    }

    /// 由 [`UserDefinedField`] 构造。
    ///
    /// 对应 Java: 直接把 `UserDefinedField` 实例当作 `FieldType` 键使用
    pub fn from_field(field: &Rc<RefCell<UserDefinedField>>) -> Self {
        let field = field.borrow();
        Self {
            unique_id: field.get_unique_id(),
            field_type_class: field
                .get_field_type_class()
                .unwrap_or(FieldTypeClass::Unknown),
            data_type: field.get_data_type().unwrap_or(DataType::Custom),
        }
    }

    /// 取 unique ID（本类型的唯一等价依据）。
    ///
    /// 对应 Java: `UserDefinedField.getUniqueID()`
    pub fn get_unique_id(&self) -> i32 {
        self.unique_id
    }

    /// 取字段类型。
    ///
    /// 对应 Java: `UserDefinedField.getFieldTypeClass()`
    pub fn get_field_type_class(&self) -> FieldTypeClass {
        self.field_type_class
    }

    /// 取数据类型。
    ///
    /// 对应 Java: `UserDefinedField.getDataType()`
    pub fn get_data_type(&self) -> DataType {
        self.data_type
    }

    /// 自定义字段在字段机制里的名称占位。
    ///
    /// Java 的 `UserDefinedField.getName()` 返回动态的外部名；本类型是 `Copy` 的
    /// 等价键，无法承载 `String`，故返回该常量。需要真实名称时请直接读
    /// [`UserDefinedField`]。
    pub const NAME: &'static str = "CUSTOM_FIELD";
}

impl PartialEq for CustomFieldKey {
    /// 对应 Java: `UserDefinedField.equals` —— 只比较 `m_uniqueID`
    fn eq(&self, other: &Self) -> bool {
        self.unique_id == other.unique_id
    }
}

impl Eq for CustomFieldKey {}

impl std::hash::Hash for CustomFieldKey {
    /// 对应 Java: `UserDefinedField.hashCode` —— 只散列 `m_uniqueID`
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.unique_id.hash(state);
    }
}

/// 字段标识的统一载体，对应 Java 的 `FieldType` 接口引用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldKey {
    /// 对应 Java: `TaskField`
    Task(TaskField),
    /// 对应 Java: `ResourceField`
    Resource(ResourceField),
    /// 对应 Java: `AssignmentField`
    Assignment(AssignmentField),
    /// 对应 Java: `ProjectField`
    Project(ProjectField),
    /// 对应 Java: `ConstraintField`
    Constraint(ConstraintField),
    /// 对应 Java: `UserDefinedField`（自定义字段）
    Custom(CustomFieldKey),
}

impl FieldKey {
    /// 返回该字段的具体枚举名，便于日志与错误信息。
    pub fn type_name(&self) -> &'static str {
        match self {
            FieldKey::Task(_) => "TaskField",
            FieldKey::Resource(_) => "ResourceField",
            FieldKey::Assignment(_) => "AssignmentField",
            FieldKey::Project(_) => "ProjectField",
            FieldKey::Constraint(_) => "ConstraintField",
            FieldKey::Custom(_) => "UserDefinedField",
        }
    }
}

impl From<TaskField> for FieldKey {
    fn from(value: TaskField) -> Self {
        FieldKey::Task(value)
    }
}

impl From<ResourceField> for FieldKey {
    fn from(value: ResourceField) -> Self {
        FieldKey::Resource(value)
    }
}

impl From<AssignmentField> for FieldKey {
    fn from(value: AssignmentField) -> Self {
        FieldKey::Assignment(value)
    }
}

impl From<ProjectField> for FieldKey {
    fn from(value: ProjectField) -> Self {
        FieldKey::Project(value)
    }
}

impl From<ConstraintField> for FieldKey {
    fn from(value: ConstraintField) -> Self {
        FieldKey::Constraint(value)
    }
}

impl From<CustomFieldKey> for FieldKey {
    fn from(value: CustomFieldKey) -> Self {
        FieldKey::Custom(value)
    }
}

impl From<&Rc<RefCell<UserDefinedField>>> for FieldKey {
    /// 由自定义字段实例取等价键。
    ///
    /// 对应 Java: 把 `UserDefinedField` 实例直接当作 `FieldType` 键
    fn from(value: &Rc<RefCell<UserDefinedField>>) -> Self {
        FieldKey::Custom(CustomFieldKey::from_field(value))
    }
}

impl MpxjEnum for FieldKey {
    fn value(&self) -> i32 {
        match self {
            FieldKey::Task(field) => field.value(),
            FieldKey::Resource(field) => field.value(),
            FieldKey::Assignment(field) => field.value(),
            FieldKey::Project(field) => field.value(),
            FieldKey::Constraint(field) => field.value(),
            FieldKey::Custom(field) => field.get_unique_id(),
        }
    }
}

impl FieldType for FieldKey {
    fn get_field_type_class(&self) -> FieldTypeClass {
        match self {
            FieldKey::Task(field) => field.get_field_type_class(),
            FieldKey::Resource(field) => field.get_field_type_class(),
            FieldKey::Assignment(field) => field.get_field_type_class(),
            FieldKey::Project(field) => field.get_field_type_class(),
            FieldKey::Constraint(field) => field.get_field_type_class(),
            FieldKey::Custom(field) => field.get_field_type_class(),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            FieldKey::Task(field) => field.name(),
            FieldKey::Resource(field) => field.name(),
            FieldKey::Assignment(field) => field.name(),
            FieldKey::Project(field) => field.name(),
            FieldKey::Constraint(field) => field.name(),
            // 自定义字段的名称是动态字符串，`Copy` 键无法承载，返回占位常量
            FieldKey::Custom(_) => CustomFieldKey::NAME,
        }
    }

    fn get_name(&self) -> &'static str {
        match self {
            FieldKey::Task(field) => field.get_name(),
            FieldKey::Resource(field) => field.get_name(),
            FieldKey::Assignment(field) => field.get_name(),
            FieldKey::Project(field) => field.get_name(),
            FieldKey::Constraint(field) => field.get_name(),
            // 同上
            FieldKey::Custom(_) => CustomFieldKey::NAME,
        }
    }

    fn get_data_type(&self) -> DataType {
        match self {
            FieldKey::Task(field) => field.get_data_type(),
            FieldKey::Resource(field) => field.get_data_type(),
            FieldKey::Assignment(field) => field.get_data_type(),
            FieldKey::Project(field) => field.get_data_type(),
            FieldKey::Constraint(field) => field.get_data_type(),
            FieldKey::Custom(field) => field.get_data_type(),
        }
    }

    fn get_units_type(&self) -> Option<Self> {
        match self {
            FieldKey::Task(field) => field.get_units_type().map(FieldKey::Task),
            FieldKey::Resource(field) => field.get_units_type().map(FieldKey::Resource),
            FieldKey::Assignment(field) => field.get_units_type().map(FieldKey::Assignment),
            FieldKey::Project(field) => field.get_units_type().map(FieldKey::Project),
            FieldKey::Constraint(field) => field.get_units_type().map(FieldKey::Constraint),
            // 对应 Java: `UserDefinedField.getUnitsType()` 恒返回 null
            FieldKey::Custom(_) => None,
        }
    }
}

impl std::fmt::Display for FieldKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn field_key_forwards_to_inner_enum() {
        let key = FieldKey::Task(TaskField::Name);
        assert_eq!(key.get_field_type_class(), FieldTypeClass::Task);
        assert_eq!(key.get_name(), "Task Name");
        assert_eq!(key.name(), "NAME");

        let key = FieldKey::Resource(ResourceField::Name);
        assert_eq!(key.get_field_type_class(), FieldTypeClass::Resource);
    }

    #[test]
    fn field_key_maps_units_type_into_same_family() {
        let units = FieldKey::Task(TaskField::Duration).get_units_type();
        assert!(matches!(units, Some(FieldKey::Task(_))));
    }

    #[test]
    fn custom_field_key_equality_follows_java_unique_id_only() {
        // 对应 Java: `UserDefinedField.equals` 只比较 `m_uniqueID`
        let a = CustomFieldKey::new(123, FieldTypeClass::Task, DataType::Numeric);
        let b = CustomFieldKey::new(123, FieldTypeClass::Resource, DataType::Custom);

        assert_eq!(a, b, "元信息不同但 unique ID 相同应视为同一字段（与 Java 一致）");
        assert_ne!(a, CustomFieldKey::new(124, FieldTypeClass::Task, DataType::Numeric));
    }

    #[test]
    fn custom_field_key_is_usable_as_a_map_key() {
        let mut map = HashMap::new();
        map.insert(
            FieldKey::Custom(CustomFieldKey::new(
                123,
                FieldTypeClass::Task,
                DataType::Numeric,
            )),
            "value",
        );

        // 元信息不同但 unique ID 相同 → 仍命中同一条目
        let probe = FieldKey::Custom(CustomFieldKey::new(
            123,
            FieldTypeClass::Resource,
            DataType::Custom,
        ));
        assert_eq!(map.get(&probe), Some(&"value"));

        let missing = FieldKey::Custom(CustomFieldKey::new(
            999,
            FieldTypeClass::Task,
            DataType::Numeric,
        ));
        assert_eq!(map.get(&missing), None);
    }

    #[test]
    fn custom_field_key_forwards_metadata() {
        let key = FieldKey::Custom(CustomFieldKey::new(
            7,
            FieldTypeClass::Assignment,
            DataType::Numeric,
        ));

        assert_eq!(key.type_name(), "UserDefinedField");
        assert_eq!(key.get_field_type_class(), FieldTypeClass::Assignment);
        assert_eq!(key.get_data_type(), DataType::Numeric);
        assert_eq!(key.value(), 7);
        // 对应 Java: `UserDefinedField.getUnitsType()` 恒返回 null
        assert!(key.get_units_type().is_none());
    }

    #[test]
    fn custom_field_round_trips_through_a_field_container() {
        // 回归：在此之前自定义字段无法写入字段容器（FieldKey 无「自定义」变体）
        let mut container = crate::abstract_field_container::AbstractFieldContainer::new();
        let key = FieldKey::Custom(CustomFieldKey::new(
            500,
            FieldTypeClass::Task,
            DataType::Numeric,
        ));

        container.set(key, Some(crate::field_value::FieldValue::Integer(42)));

        assert_eq!(
            container.get_cached_value(key),
            Some(&crate::field_value::FieldValue::Integer(42))
        );
    }

    #[test]
    fn custom_field_value_is_readable_back_from_a_task() {
        // 回归：`Task::get` 此前对非 TaskField 的键**直接返回 `None`**，
        // 而 Java 的 `Task.get(FieldType)` 会回落到 `super.get(field)`（字段表）。
        // 否则写进容器的自定义字段读不出来。
        use crate::field_value::FieldValue;
        use crate::task::Task;

        let mut task = Task::new();
        let key = FieldKey::Custom(CustomFieldKey::new(
            1234,
            FieldTypeClass::Task,
            DataType::Numeric,
        ));

        task.set(key, Some(FieldValue::Integer(7)));

        assert_eq!(task.get(key), Some(FieldValue::Integer(7)));
    }
}
