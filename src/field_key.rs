//! 对应 Java: `org/mpxj/FieldType` 的运行时类型擦除形式。
//!
//! Java 侧 `AbstractFieldContainer` 用 `Map<FieldType, Object>` 存字段，
//! 键是 `FieldType` 接口，实际取值来自 `TaskField` / `ResourceField` /
//! `AssignmentField` / `ProjectField` / `ConstraintField` 五个枚举。
//! Rust 的 `HashMap` 需要一个具体键类型，因此这里引入 `FieldKey`
//! 作为这五个枚举的统一载体 —— 语义与 Java 的 `FieldType` 接口等价。

use crate::assignment_field::AssignmentField;
use crate::constraint_field::ConstraintField;
use crate::data_type::DataType;
use crate::field_type::FieldType;
use crate::field_type_class::FieldTypeClass;
use crate::mpxj_enum::MpxjEnum;
use crate::project_field::ProjectField;
use crate::resource_field::ResourceField;
use crate::task_field::TaskField;

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

impl MpxjEnum for FieldKey {
    fn value(&self) -> i32 {
        match self {
            FieldKey::Task(field) => field.value(),
            FieldKey::Resource(field) => field.value(),
            FieldKey::Assignment(field) => field.value(),
            FieldKey::Project(field) => field.value(),
            FieldKey::Constraint(field) => field.value(),
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
        }
    }

    fn name(&self) -> &'static str {
        match self {
            FieldKey::Task(field) => field.name(),
            FieldKey::Resource(field) => field.name(),
            FieldKey::Assignment(field) => field.name(),
            FieldKey::Project(field) => field.name(),
            FieldKey::Constraint(field) => field.name(),
        }
    }

    fn get_name(&self) -> &'static str {
        match self {
            FieldKey::Task(field) => field.get_name(),
            FieldKey::Resource(field) => field.get_name(),
            FieldKey::Assignment(field) => field.get_name(),
            FieldKey::Project(field) => field.get_name(),
            FieldKey::Constraint(field) => field.get_name(),
        }
    }

    fn get_data_type(&self) -> DataType {
        match self {
            FieldKey::Task(field) => field.get_data_type(),
            FieldKey::Resource(field) => field.get_data_type(),
            FieldKey::Assignment(field) => field.get_data_type(),
            FieldKey::Project(field) => field.get_data_type(),
            FieldKey::Constraint(field) => field.get_data_type(),
        }
    }

    fn get_units_type(&self) -> Option<Self> {
        match self {
            FieldKey::Task(field) => field.get_units_type().map(FieldKey::Task),
            FieldKey::Resource(field) => field.get_units_type().map(FieldKey::Resource),
            FieldKey::Assignment(field) => field.get_units_type().map(FieldKey::Assignment),
            FieldKey::Project(field) => field.get_units_type().map(FieldKey::Project),
            FieldKey::Constraint(field) => field.get_units_type().map(FieldKey::Constraint),
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
}
