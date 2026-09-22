//! Mapping result of MPP field ID ↔ unified field type (Rust-specific).
//!
//! MPXJ's `org.mpxj.common.MPPTaskField` / `MPPResourceField` / `MPPAssignmentField` /
//! `MPPProjectField` / `MPPConstraintField` all provide
//! `FieldType getInstance(ProjectFile project, int value)`: map the field ID in the MPP file
//! to a unified `FieldType`.
//!
//! Java's `FieldType` may be either a **built-in field** (`TaskField` / `ResourceField` / …)
//! or a **custom field** `UserDefinedField` (when the high bit of the field ID is 1, i.e., an
//! enterprise custom field). The Rust [`FieldKey`] is this crate's runtime-erased carrier for
//! `FieldType`, but it requires `Copy + Eq + Hash` and cannot carry a `UserDefinedField` that
//! internally contains a `String`.
//!
//! Therefore this module introduces two types:
//! - [`MppFieldType`] — the return value of the mapping, expressing both the "built-in" and
//!   "custom" cases;
//! - [`MppFieldKey`] — an equivalent identity usable as a `HashMap` key (custom fields use
//!   their unique ID).

use std::cell::RefCell;
use std::rc::Rc;

use crate::data_type::DataType;
use crate::field_key::FieldKey;
use crate::field_type::FieldType;
use crate::field_type_class::FieldTypeClass;
use crate::user_defined_field::UserDefinedField;

/// Corresponds to Java: the return type `FieldType` of methods such as `MPPTaskField.getInstance(...)`.
#[derive(Clone)]
pub enum MppFieldType {
    /// Built-in field (`TaskField` / `ResourceField` / `AssignmentField` / `ProjectField` / `ConstraintField`).
    Builtin(FieldKey),
    /// Project custom field (`UserDefinedField`).
    ///
    /// Corresponds to Java: the return value of `project.getUserDefinedFields().getOrCreateTaskField(...)`.
    Custom(Rc<RefCell<UserDefinedField>>),
}

impl MppFieldType {
    /// Get an equivalent identity usable as a `HashMap` key.
    pub fn to_key(&self) -> MppFieldKey {
        match self {
            MppFieldType::Builtin(key) => MppFieldKey::Builtin(*key),
            MppFieldType::Custom(field) => MppFieldKey::Custom(field.borrow().get_unique_id()),
        }
    }

    /// Convert to the key used by the field container.
    ///
    /// Corresponds to Java: `container.set(item.getType(), value)` — Java uses `FieldType`
    /// directly as the key; the Rust side uses its [`FieldKey`] for built-in fields and
    /// converts custom fields to [`FieldKey::Custom`] (equivalence based on unique ID,
    /// consistent with Java's `UserDefinedField.equals`, see [`CustomFieldKey`]).
    pub fn to_field_key(&self) -> FieldKey {
        match self {
            MppFieldType::Builtin(key) => *key,
            MppFieldType::Custom(field) => {
                FieldKey::Custom(crate::field_key::CustomFieldKey::from_field(field))
            }
        }
    }

    /// Whether this is a built-in field.
    pub fn is_builtin(&self) -> bool {
        matches!(self, MppFieldType::Builtin(_))
    }

    /// Whether this is a custom field.
    pub fn is_custom(&self) -> bool {
        matches!(self, MppFieldType::Custom(_))
    }

    /// Get the [`FieldKey`] of a built-in field; returns `None` for a custom field.
    pub fn as_builtin(&self) -> Option<FieldKey> {
        match self {
            MppFieldType::Builtin(key) => Some(*key),
            MppFieldType::Custom(_) => None,
        }
    }

    /// Get the shared handle of a custom field; returns `None` for a built-in field.
    pub fn as_custom(&self) -> Option<&Rc<RefCell<UserDefinedField>>> {
        match self {
            MppFieldType::Builtin(_) => None,
            MppFieldType::Custom(field) => Some(field),
        }
    }

    /// Get the entity type this field belongs to.
    ///
    /// Corresponds to Java: `FieldType.getFieldTypeClass()`
    ///
    /// The corresponding method of `UserDefinedField` returns `Option` (the field may be unset
    /// on the Java side); here it falls back to `UNKNOWN` (consistent with how each caller
    /// handles Java returning `null`).
    pub fn get_field_type_class(&self) -> FieldTypeClass {
        match self {
            MppFieldType::Builtin(key) => key.get_field_type_class(),
            MppFieldType::Custom(field) => field
                .borrow()
                .get_field_type_class()
                .unwrap_or(FieldTypeClass::Unknown),
        }
    }

    /// Get the data type of this field.
    ///
    /// Corresponds to Java: `FieldType.getDataType()`
    ///
    /// When `UserDefinedField` has no data type set it falls back to `CUSTOM`
    /// (corresponding to the default passed to Java's `MPPTaskField.getInstance(project, value)`).
    pub fn get_data_type(&self) -> DataType {
        match self {
            MppFieldType::Builtin(key) => key.get_data_type(),
            MppFieldType::Custom(field) => field.borrow().get_data_type().unwrap_or(DataType::Custom),
        }
    }

    /// Get the display name of this field.
    ///
    /// Corresponds to Java: `FieldType.getName()`
    ///
    /// A custom field prefers the external name and falls back to the internal name when unset.
    pub fn get_name(&self) -> String {
        match self {
            MppFieldType::Builtin(key) => key.get_name().to_owned(),
            MppFieldType::Custom(field) => {
                let field = field.borrow();
                field
                    .get_name()
                    .map(str::to_owned)
                    .unwrap_or_else(|| field.name().to_owned())
            }
        }
    }
}

impl From<FieldKey> for MppFieldType {
    fn from(value: FieldKey) -> Self {
        MppFieldType::Builtin(value)
    }
}

impl std::fmt::Display for MppFieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.get_name())
    }
}

/// A hashable equivalent identity of [`MppFieldType`], used as a `HashMap` key.
///
/// Built-in fields use their [`FieldKey`]; custom fields use their unique ID (corresponding to
/// Java's `UserDefinedField.getValue()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MppFieldKey {
    /// Built-in field.
    Builtin(FieldKey),
    /// The unique ID of a custom field.
    Custom(i32),
}

impl std::fmt::Display for MppFieldKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MppFieldKey::Builtin(key) => write!(f, "{key}"),
            MppFieldKey::Custom(unique_id) => write!(f, "CustomField#{unique_id}"),
        }
    }
}
