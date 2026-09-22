//! Corresponds to Java: `org/mpxj/common/MPPConstraintField.java`
//!
//! A utility class that maps the integers representing "constraint fields" in Microsoft Project
//! binary files to MPXJ's field enum.
//!
//! # Differences from Java
//!
//! - Java's `getInstance` / `getID` use the `FieldType` interface; the Rust side carries it with
//!   [`MppFieldType`] (built-in / custom cases), see the notes in
//!   [`crate::common::mpp_field_type`]. Constraint fields have no "enterprise custom field"
//!   branch, so the return value here is always [`MppFieldType::Builtin`].
//! - Java's `null` return value → `Option::None`; reading the table out of bounds returns `None`,
//!   and reading `ID_ARRAY` out of bounds returns `-1` instead of throwing
//!   `ArrayIndexOutOfBoundsException`.
//! - Java's `getInstance(int value)` has no `project` parameter; the Rust side provides
//!   [`MPPConstraintField::get_instance`] (with an unused `project`) for a **uniform signature**
//!   with modules such as `MPPAssignmentField` / `MPPTaskField`, and keeps the equivalent
//!   original Java signature [`MPPConstraintField::get_instance_from_value`].
//! - Java's `ID_ARRAY` is derived from `FIELD_ARRAY` at compile time per Java's static
//!   initializer block (first fill with -1, then set `ID_ARRAY[field.getValue()] = loop`).
//!
//! Skipped Java code: none. Unported Java methods: none.

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::mpp_field_type::MppFieldType;
use crate::constraint_field::ConstraintField;
use crate::field_key::FieldKey;
use crate::mpxj_enum::MpxjEnum;
use crate::project_file::ProjectFile;

/// Corresponds to Java: `org.mpxj.common.MPPConstraintField`
pub struct MPPConstraintField;

impl MPPConstraintField {
    /// The field ID base for enterprise custom fields.
    ///
    /// Corresponds to Java: `public static final int CONSTRAINT_FIELD_BASE = 0xD400000;`
    pub const CONSTRAINT_FIELD_BASE: i32 = 0x0D40_0000;

    /// Get the corresponding field type from a field ID in a Microsoft Project file.
    ///
    /// Java: `public static FieldType getInstance(int value)` (no `project` parameter).
    ///
    /// The Rust side keeps the `project` parameter to maintain a uniform signature with
    /// `MPPAssignmentField` / `MPPTaskField` etc.; constraint fields have no custom field branch,
    /// so this parameter is unused.
    pub fn get_instance(
        project: &Rc<RefCell<ProjectFile>>,
        value: i32,
    ) -> Option<MppFieldType> {
        // Java's getInstance does not use project (constraint fields have no custom field).
        let _ = project;
        Self::get_instance_from_value(value)
    }

    /// Get the corresponding field type from a field ID in a Microsoft Project file.
    ///
    /// Corresponds to Java: `public static FieldType getInstance(int value)`
    ///
    /// See Java lines 45-55.
    pub fn get_instance_from_value(value: i32) -> Option<MppFieldType> {
        // Java: `FieldType result = null; if (value >= 0 && value < FIELD_ARRAY.length) { result = FIELD_ARRAY[value]; }`
        if value >= 0 && (value as usize) < MAX_VALUE {
            return FIELD_ARRAY[value as usize]
                .map(|field| MppFieldType::Builtin(FieldKey::Constraint(field)));
        }

        None
    }

    /// Get the field ID used by the field in Microsoft Project.
    ///
    /// Corresponds to Java: `public static int getID(FieldType value)`
    ///
    /// See Java lines 63-66: `ID_ARRAY[value.getValue()]`.
    pub fn get_id(value: &MppFieldType) -> i32 {
        match value {
            // Constraint fields have no custom field branch; this branch only aligns with Java's
            // `value instanceof UserDefinedField` semantics (Java also returns getValue());
            // returns -1 when the field is mutably borrowed (no such case in Java).
            MppFieldType::Custom(field) => field
                .try_borrow()
                .map(|field| field.get_value())
                .unwrap_or(-1),
            // Java: `ID_ARRAY[value.getValue()]`, throwing
            // ArrayIndexOutOfBoundsException when out of bounds (can happen when a non-constraint field is passed);
            // the Rust side does not panic and degrades gracefully to ID_ARRAY's default value -1.
            MppFieldType::Builtin(key) => usize::try_from(key.value())
                .ok()
                .and_then(|index| ID_ARRAY.get(index))
                .copied()
                .unwrap_or(-1),
        }
    }
}

/// Mapping table length (the number of elements in `FIELD_ARRAY`).
///
/// Corresponds to Java: `private static final int MAX_VALUE = 10;`
const MAX_VALUE: usize = 10;

/// MS Project field ID → constraint field.
///
/// Corresponds to Java: the `FIELD_ARRAY` at lines 70-77 of `MPPConstraintField.java`
/// (`private static final ConstraintField[] FIELD_ARRAY = new ConstraintField[MAX_VALUE]`
/// and its `static {}` initializer block). In Java, an unassigned index is `null`; on the Rust
/// side it is `None`; the comment after each entry is the index explicitly assigned in Java.
const FIELD_ARRAY: [Option<ConstraintField>; MAX_VALUE] = [
    None, // 0
    None, // 1
    None, // 2
    None, // 3
    None, // 4
    None, // 5
    Some(ConstraintField::Task1), // 6
    Some(ConstraintField::Task2), // 7
    None, // 8
    Some(ConstraintField::UniqueId), // 9
];

/// The reverse mapping table from field → MS Project field ID.
///
/// Corresponds to Java: the `ID_ARRAY` static initializer block at lines 79-92 of
/// `MPPConstraintField.java` (first calls `Arrays.fill(ID_ARRAY, -1)`, then iterates
/// `FIELD_ARRAY` setting `ID_ARRAY[field.getValue()] = loop`).
const ID_ARRAY: [i32; ConstraintField::MAX_VALUE] = build_id_array(&FIELD_ARRAY);

/// Derive `ID_ARRAY` at compile time following Java's static initializer block.
const fn build_id_array(
    field_array: &[Option<ConstraintField>; MAX_VALUE],
) -> [i32; ConstraintField::MAX_VALUE] {
    let mut result = [-1; ConstraintField::MAX_VALUE];
    let mut loop_index = 0;
    while loop_index < field_array.len() {
        if let Some(field) = field_array[loop_index] {
            result[field as usize] = loop_index as i32;
        }
        loop_index += 1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assignment_field::AssignmentField;
    use crate::field_type::FieldType;

    /// Create a new project file (Java's getInstance does not need it; it is passed only for a uniform signature).
    fn project() -> Rc<RefCell<ProjectFile>> {
        ProjectFile::new()
    }

    /// Get a built-in constraint field.
    fn builtin(value: i32) -> ConstraintField {
        match MPPConstraintField::get_instance(&project(), value) {
            Some(MppFieldType::Builtin(FieldKey::Constraint(field))) => field,
            _ => panic!("field number {value} should be a constraint field"),
        }
    }

    #[test]
    fn field_numbers_match_java() {
        // Aligned item by item with Java's `static {}` block
        let field = builtin(6);
        assert_eq!(field, ConstraintField::Task1);
        assert_eq!(field.name(), "TASK1");

        let field = builtin(7);
        assert_eq!(field, ConstraintField::Task2);
        assert_eq!(field.name(), "TASK2");

        let field = builtin(9);
        assert_eq!(field, ConstraintField::UniqueId);
        assert_eq!(field.name(), "UNIQUE_ID");
    }

    #[test]
    fn unassigned_and_out_of_range_field_numbers_are_none() {
        let project = project();

        // These indices are unassigned (i.e. null) in Java
        for value in [0, 1, 2, 3, 4, 5, 8] {
            assert!(
                MPPConstraintField::get_instance(&project, value).is_none(),
                "field number {value} is not assigned in Java"
            );
        }

        // Out of bounds (Java returns null)
        assert!(MPPConstraintField::get_instance(&project, -1).is_none());
        assert!(MPPConstraintField::get_instance(&project, MAX_VALUE as i32).is_none());
        assert!(MPPConstraintField::get_instance(&project, 100).is_none());
    }

    #[test]
    fn java_signature_overload_matches_contract_signature() {
        for value in -1..(MAX_VALUE as i32 + 1) {
            assert_eq!(
                MPPConstraintField::get_instance_from_value(value).map(|field| MPPConstraintField::get_id(&field)),
                MPPConstraintField::get_instance(&project(), value)
                    .map(|field| MPPConstraintField::get_id(&field)),
                "field number {value}"
            );
        }
    }

    #[test]
    fn id_round_trips_for_every_mapped_field_number() {
        let project = project();

        for value in 0..(MAX_VALUE as i32) {
            if let Some(field) = MPPConstraintField::get_instance(&project, value) {
                assert_eq!(MPPConstraintField::get_id(&field), value, "field number {value}");
            }
        }
    }

    #[test]
    fn id_array_matches_java_static_initialiser() {
        // Java: ID_ARRAY[UNIQUE_ID.getValue()] = 9 / TASK1 = 6 / TASK2 = 7
        assert_eq!(
            MPPConstraintField::get_id(&MppFieldType::Builtin(FieldKey::Constraint(
                ConstraintField::UniqueId
            ))),
            9
        );
        assert_eq!(
            MPPConstraintField::get_id(&MppFieldType::Builtin(FieldKey::Constraint(
                ConstraintField::Task1
            ))),
            6
        );
        assert_eq!(
            MPPConstraintField::get_id(&MppFieldType::Builtin(FieldKey::Constraint(
                ConstraintField::Task2
            ))),
            7
        );

        // A non-constraint field (getValue() beyond the length of ID_ARRAY) throws
        // ArrayIndexOutOfBoundsException in Java; the Rust side degrades gracefully to -1.
        assert_eq!(
            MPPConstraintField::get_id(&MppFieldType::Builtin(FieldKey::Assignment(
                AssignmentField::Finish
            ))),
            -1
        );
    }
}
