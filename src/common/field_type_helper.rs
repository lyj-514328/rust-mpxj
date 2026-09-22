//! Corresponds to Java: `org/mpxj/common/FieldTypeHelper.java`
//!
//! `FieldType`-related utility methods: field ID ↔ `FieldType` conversion, fallback mapping
//! for text fields, and the "is the field value the default value" check.
//!
//! # Differences from Java (Rust-specific wiring)
//!
//! - **`FieldType` → [`MppFieldType`]**: Java's `FieldType` is an interface that can hold both
//!   `TaskField` / `ResourceField` / … and `UserDefinedField`; the Rust side uses
//!   [`crate::common::mpp_field_type::MppFieldType`] to express these two cases.
//! - **`null` → `Option`**: `getFieldID(null)` returns `-1` in Java; the Rust side expresses
//!   this nullable input as `Option<&MppFieldType>`.
//! - **`getPlaceholder` (skipped Java method)**: Java's `getInstance` returns an anonymous
//!   `FieldType` implementation when it cannot recognize a field ID — `getFieldTypeClass()` is
//!   always `FieldTypeClass.UNKNOWN`, `name()` is always `"UNKNOWN"`, `getValue()` is the field
//!   ID `fieldID`, `getName()` is `"Unknown"` (when `type` is `null`) or
//!   `"Unknown " + type simple name + "(" + fieldID + ")"`, and `getDataType()` and
//!   `getUnitsType()` are always `null`. Rust's `MppFieldType` has only the built-in and custom
//!   variants, and `FieldKey` has no `UNKNOWN` variant, so **it cannot express that placeholder
//!   object**; therefore all branches that would return the placeholder uniformly return `None`
//!   (callers that need to distinguish "unknown type" from "no field" must handle this
//!   separately).
//! - **`Object value` → `Option<&FieldValue>`**: Java's `valueIsNotDefault` accepts any object
//!   and casts it by `DataType` (throws `ClassCastException` on a type mismatch /
//!   `NullPointerException` when `dataType` is `null`); the Rust side expresses it with
//!   [`FieldValue`] and returns "is the default value" for any unexpected type (without
//!   `panic`), keeping Java's initialized value `true` when `dataType` is missing.
//!
//! # Dependencies
//!
//! `getFieldID` / `getInstance` require the public members of the sibling modules
//! `MPPResourceField` / `MPPAssignmentField` / `MPPProjectField` / `MPPConstraintField` (ported
//! together with `MPPTaskField`): the `RESOURCE_FIELD_BASE` / `ASSIGNMENT_FIELD_BASE` /
//! `PROJECT_FIELD_BASE` / `CONSTRAINT_FIELD_BASE` constants and the `get_id`,
//! `get_instance_with_data_type` (constraint fields use `get_instance`) methods.
//!
//! Unported Java methods: `getPlaceholder` (see above).

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

/// `FieldType`-related utilities.
///
/// Corresponds to Java: `public final class FieldTypeHelper`
pub struct FieldTypeHelper;

impl FieldTypeHelper {
    /// Get the field ID used by MPP9/MPP12 from MPXJ's field type.
    ///
    /// Corresponds to Java: `public static int getFieldID(FieldType type)`
    ///
    /// `None` corresponds to Java's `null` (returns `-1`).
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
            // Java: default: result = -1; (Constraint / Unknown, and custom fields whose type class is null)
            _ => -1,
        }
    }

    /// Get the field type from a field ID in an MPP9/MPP12/MPP14 file.
    ///
    /// Corresponds to Java: `public static final FieldType getInstance(ProjectFile project, int fieldID)`
    pub fn get_instance(project: &Rc<RefCell<ProjectFile>>, field_id: i32) -> Option<MppFieldType> {
        // Java: return getInstance(project, fieldID, DataType.CUSTOM);
        Self::get_instance_with_data_type(project, field_id, DataType::Custom)
    }

    /// Get the field type from a field ID in an MPP9/MPP12/MPP14 file (custom fields may
    /// specify a data type).
    ///
    /// Corresponds to Java: `public static final FieldType getInstance(ProjectFile project, int fieldID, DataType customFieldDataType)`
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
        // (Java's `int` is 32-bit two's complement, and `0xFFFF0000` cannot be written directly as a Rust `i32` literal)
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
            // Java: result = MPPConstraintField.getInstance(index); (Java has no project parameter,
            // the Rust side keeps that parameter for a uniform signature; constraint fields have no custom-field branch)
            MPPConstraintField::CONSTRAINT_FIELD_BASE => {
                MPPConstraintField::get_instance(project, index)
            }
            MPPProjectField::PROJECT_FIELD_BASE => {
                MPPProjectField::get_instance_with_data_type(project, index, custom_field_data_type)
            }
            // Java: default: result = getPlaceholder(null, index); (the placeholder object cannot be expressed, see the module notes)
            _ => None,
        }
    }

    /// Map a "text version field" (e.g. Start Text) back to the non-text field that needs handling.
    ///
    /// Corresponds to Java: `public static FieldType mapTextFields(FieldType field)`
    ///
    /// Java only handles `TaskField` and returns other types unchanged.
    pub fn map_text_fields(field: MppFieldType) -> MppFieldType {
        match field {
            MppFieldType::Builtin(FieldKey::Task(task_field)) => match task_field {
                // Java: case START_TEXT: field = TaskField.START; break;
                TaskField::StartText => MppFieldType::Builtin(FieldKey::Task(TaskField::Start)),
                // Java: case FINISH_TEXT: field = TaskField.FINISH; break;
                TaskField::FinishText => MppFieldType::Builtin(FieldKey::Task(TaskField::Finish)),
                // Java: case DURATION_TEXT: field = TaskField.DURATION; break;
                TaskField::DurationText => MppFieldType::Builtin(FieldKey::Task(TaskField::Duration)),
                // Java: default: break; (return unchanged)
                other => MppFieldType::Builtin(FieldKey::Task(other)),
            },
            // Java: if (!(field instanceof TaskField)) { return field; }
            other => other,
        }
    }

    /// Determine whether the given value is a "non-default value" for the field type.
    ///
    /// Corresponds to Java: `public static final boolean valueIsNotDefault(FieldType type, Object value)`
    ///
    /// `None` corresponds to Java's `null` (returns `false`, i.e., "is the default value").
    pub fn value_is_not_default(field_type: &MppFieldType, value: Option<&FieldValue>) -> bool {
        // Java: boolean result = true; if (value == null) { result = false; }
        let Some(value) = value else {
            return false;
        };

        // Java: switch (type.getDataType()) { ... }
        // Java throws NullPointerException when the data type is null (a custom field that was never set);
        // the Rust side keeps the initialized value true (i.e., "non-default value", without panicking).
        match data_type_of(field_type) {
            // Java: result = !(value.toString()).isEmpty();
            // The toString result of a non-string value (number, boolean, date, etc.) is always non-empty.
            Some(DataType::String) | Some(DataType::Notes) => match value {
                FieldValue::String(text) => !text.is_empty(),
                _ => true,
            },

            // Java: result = ((Number) value).doubleValue() != 0.0;
            // Non-numeric types throw ClassCastException in Java; the Rust side treats them as "is the default value".
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
            // Rust's `FieldValue` has no `Priority` variant; priority is stored as an integer.
            Some(DataType::Priority) => value
                .as_integer()
                .map_or(false, |priority| priority != Priority::MEDIUM),

            // Java: default: break; (keeps the initialized value true)
            _ => true,
        }
    }
}

/// Get the entity type the field belongs to; returns `None` when a custom field has no type.
///
/// Java: `FieldType.getFieldTypeClass()` (`UserDefinedField` may return `null`)
fn field_type_class_of(field: &MppFieldType) -> Option<FieldTypeClass> {
    match field {
        MppFieldType::Builtin(key) => Some(key.get_field_type_class()),
        MppFieldType::Custom(field) => field
            .try_borrow()
            .ok()
            .and_then(|field| field.get_field_type_class()),
    }
}

/// Get the field's data type; returns `None` when a custom field has no type (Java returns `null`).
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

    /// Construct a project (used for custom fields).
    fn project() -> Rc<RefCell<ProjectFile>> {
        ProjectFile::new()
    }

    /// Construct a built-in task field.
    fn task_field(field: TaskField) -> MppFieldType {
        MppFieldType::Builtin(FieldKey::Task(field))
    }

    /// Construct a custom field with the specified data type.
    fn custom_field(project: &Rc<RefCell<ProjectFile>>, data_type: DataType) -> MppFieldType {
        MPPTaskField::get_instance_with_data_type(project, 0x8000, data_type)
            .expect("should create custom field")
    }

    #[test]
    fn get_field_id_matches_java() {
        assert_eq!(FieldTypeHelper::get_field_id(None), -1);

        // Java: MPPTaskField.TASK_FIELD_BASE | MPPTaskField.getID(type)
        // NAME only appears in FIELD_ARRAY[14]
        assert_eq!(
            FieldTypeHelper::get_field_id(Some(&task_field(TaskField::Name))),
            MPPTaskField::TASK_FIELD_BASE | 14
        );
        // START appears at 35 and 1283; ID_ARRAY takes the latter
        assert_eq!(
            FieldTypeHelper::get_field_id(Some(&task_field(TaskField::Start))),
            MPPTaskField::TASK_FIELD_BASE | 1283
        );

        // Java: default: result = -1; (ConstraintField is not in the switch)
        let constraint = MppFieldType::Builtin(FieldKey::Constraint(
            crate::constraint_field::ConstraintField::UniqueId,
        ));
        assert_eq!(FieldTypeHelper::get_field_id(Some(&constraint)), -1);

        // Custom task field: Java takes the TASK branch, getID returns its unique ID
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
            .expect("should map to TaskField.NAME");
        assert_eq!(field.as_builtin(), Some(FieldKey::Task(TaskField::Name)));

        // Java: unknown prefix → getPlaceholder(null, index) (Rust cannot express it, returns None)
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

        // Other task fields are returned unchanged
        assert_eq!(
            FieldTypeHelper::map_text_fields(task_field(TaskField::Name)).as_builtin(),
            Some(FieldKey::Task(TaskField::Name))
        );

        // Non-TaskField (including custom fields) returned unchanged
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

        // STRING / NOTES: non-empty means non-default
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

        // NUMERIC / CURRENCY / PERCENTAGE / INTEGER: non-zero means non-default
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

        // WORK / DURATION: strings are judged by non-emptiness, otherwise by whether the duration is 0
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

        // RATE: a non-zero rate amount means non-default
        let rate_field = custom_field(&project, DataType::Rate);
        assert!(!FieldTypeHelper::value_is_not_default(
            &rate_field,
            Some(&FieldValue::Rate(Rate::new(0.0, TimeUnit::Hours)))
        ));
        assert!(FieldTypeHelper::value_is_not_default(
            &rate_field,
            Some(&FieldValue::Rate(Rate::new(10.0, TimeUnit::Hours)))
        ));

        // BOOLEAN: the value itself
        let milestone = task_field(TaskField::Milestone);
        assert!(!FieldTypeHelper::value_is_not_default(
            &milestone,
            Some(&FieldValue::Boolean(false))
        ));
        assert!(FieldTypeHelper::value_is_not_default(
            &milestone,
            Some(&FieldValue::Boolean(true))
        ));

        // PRIORITY: anything other than MEDIUM is non-default
        let priority = task_field(TaskField::Priority);
        assert!(!FieldTypeHelper::value_is_not_default(
            &priority,
            Some(&FieldValue::Integer(Priority::MEDIUM))
        ));
        assert!(FieldTypeHelper::value_is_not_default(
            &priority,
            Some(&FieldValue::Integer(Priority::HIGH))
        ));

        // Unlisted data types (e.g. DATE): Java keeps the initialized value true
        assert!(FieldTypeHelper::value_is_not_default(
            &task_field(TaskField::Start),
            Some(&FieldValue::String(String::new()))
        ));
    }
}
