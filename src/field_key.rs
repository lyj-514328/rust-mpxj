//! Corresponds to Java: the runtime type-erased form of `org/mpxj/FieldType`.
//!
//! On the Java side `AbstractFieldContainer` stores fields in a `Map<FieldType, Object>`,
//! whose keys are the `FieldType` interface and whose actual values come from the five enums `TaskField` / `ResourceField` /
//! `AssignmentField` / `ProjectField` / `ConstraintField`.
//! Rust's `HashMap` needs a concrete key type, so `FieldKey` is introduced here
//! as the unified carrier for these five enums — semantically equivalent to Java's `FieldType` interface.

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

/// Corresponds to Java: `UserDefinedField` — its equivalent form as a key in the field table.
///
/// Java's `AbstractFieldContainer` uses `FieldType` objects as keys; `UserDefinedField`
/// is one of them, and its `equals` / `hashCode` rely **only on `m_uniqueID`** (see the Java source
/// `UserDefinedField.equals` / `hashCode`). This type therefore likewise uses `unique_id` as
/// the sole basis for equivalence — both `PartialEq` / `Hash` are hand-written to compare only that field, matching Java semantics.
///
/// Besides `unique_id` it also carries the field type and data type (both small `Copy` enums),
/// so that the [`FieldType`] implementation can answer `get_field_type_class()` / `get_data_type()`.
///
/// # Why not directly hold `Rc<RefCell<UserDefinedField>>`
///
/// [`FieldKey`] needs `Copy + Eq + Hash` (it is used as a `HashMap` key and passed by value between
/// entities and containers). Holding an `Rc` would lose `Copy` and ripple through every call site in the crate.
#[derive(Debug, Clone, Copy)]
pub struct CustomFieldKey {
    unique_id: i32,
    field_type_class: FieldTypeClass,
    data_type: DataType,
}

impl CustomFieldKey {
/// Construct from a unique ID, field type, and data type.
    ///
    /// Corresponds to Java: `UserDefinedField.getUniqueID()` / `getFieldTypeClass()` / `getDataType()`
    pub fn new(unique_id: i32, field_type_class: FieldTypeClass, data_type: DataType) -> Self {
        Self {
            unique_id,
            field_type_class,
            data_type,
        }
    }

/// Construct from a [`UserDefinedField`].
    ///
/// Corresponds to Java: using a `UserDefinedField` instance directly as a `FieldType` key
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

/// Get the unique ID (the sole basis for equivalence of this type).
    ///
    /// Corresponds to Java: `UserDefinedField.getUniqueID()`
    pub fn get_unique_id(&self) -> i32 {
        self.unique_id
    }

/// Get the field type.
    ///
    /// Corresponds to Java: `UserDefinedField.getFieldTypeClass()`
    pub fn get_field_type_class(&self) -> FieldTypeClass {
        self.field_type_class
    }

/// Get the data type.
    ///
    /// Corresponds to Java: `UserDefinedField.getDataType()`
    pub fn get_data_type(&self) -> DataType {
        self.data_type
    }

/// Placeholder name for a custom field in the field mechanism.
    ///
/// Java's `UserDefinedField.getName()` returns a dynamic external name; this type is a `Copy`
/// equivalent key and cannot carry a `String`, so it returns this constant. To get the real name, read
    /// [`UserDefinedField`].
    pub const NAME: &'static str = "CUSTOM_FIELD";
}

impl PartialEq for CustomFieldKey {
/// Corresponds to Java: `UserDefinedField.equals` — compares only `m_uniqueID`
    fn eq(&self, other: &Self) -> bool {
        self.unique_id == other.unique_id
    }
}

impl Eq for CustomFieldKey {}

impl std::hash::Hash for CustomFieldKey {
/// Corresponds to Java: `UserDefinedField.hashCode` — hashes only `m_uniqueID`
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.unique_id.hash(state);
    }
}

/// Unified carrier for a field identity, corresponding to a Java `FieldType` interface reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldKey {
    /// Corresponds to Java: `TaskField`
    Task(TaskField),
    /// Corresponds to Java: `ResourceField`
    Resource(ResourceField),
    /// Corresponds to Java: `AssignmentField`
    Assignment(AssignmentField),
    /// Corresponds to Java: `ProjectField`
    Project(ProjectField),
    /// Corresponds to Java: `ConstraintField`
    Constraint(ConstraintField),
/// Corresponds to Java: `UserDefinedField` (custom field)
    Custom(CustomFieldKey),
}

impl FieldKey {
/// Return the concrete enum name of this field, for logging and error messages.
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
/// Get the equivalent key from a custom field instance.
    ///
/// Corresponds to Java: using a `UserDefinedField` instance directly as a `FieldType` key
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
// The custom field name is a dynamic string; a `Copy` key cannot carry it, so return the placeholder constant
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
// Same as above
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
// Corresponds to Java: `UserDefinedField.getUnitsType()` always returns null
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
// Corresponds to Java: `UserDefinedField.equals` compares only `m_uniqueID`
        let a = CustomFieldKey::new(123, FieldTypeClass::Task, DataType::Numeric);
        let b = CustomFieldKey::new(123, FieldTypeClass::Resource, DataType::Custom);

        assert_eq!(a, b, "different metadata but same unique ID should be treated as the same field (consistent with Java)");
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

// Different metadata but the same unique ID => still hits the same entry
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
// Corresponds to Java: `UserDefinedField.getUnitsType()` always returns null
        assert!(key.get_units_type().is_none());
    }

    #[test]
    fn custom_field_round_trips_through_a_field_container() {
// Regression: before this, custom fields could not be written to the field container (FieldKey had no "custom" variant)
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
// Regression: `Task::get` previously **returned `None` directly** for non-TaskField keys,
// whereas Java's `Task.get(FieldType)` falls back to `super.get(field)` (the field table).
// Otherwise custom fields written to the container cannot be read back.
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
