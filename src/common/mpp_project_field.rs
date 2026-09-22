//! Corresponds to Java: `org/mpxj/common/MPPProjectField.java`
//!
//! A utility class that maps the integers representing "project fields" in Microsoft Project
//! binary files to MPXJ's field enum.
//!
//! # Differences from Java
//!
//! - Java's `getInstance` has two overloads, `(ProjectFile, int)` and `(ProjectFile, int, DataType)`;
//!   the Rust side names the latter [`MPPProjectField::get_instance_with_data_type`] (Rust has
//!   no overloading).
//! - Java's `null` return value → `Option::None`. Unlike `MPPResourceField`, Java returns
//!   `null` for **non**-enterprise custom field values (project fields have no built-in field
//!   table), so this module only has the enterprise custom field branch; `getID` always returns
//!   `-1` for built-in fields.
//! - Java returns `FieldType`; the Rust side carries it with [`MppFieldType`], see the notes in
//!   [`crate::common::mpp_field_type`].
//! - The enterprise custom field branch calls
//!   `project.getUserDefinedFields().getOrCreateProjectField(...)` directly in Java; the Rust
//!   container interface requires `&mut`, so it is rewritten as the equivalent two steps (look up
//!   first, and build and register on a miss), see [`get_or_create_enterprise_field`].
//!
//! Skipped Java code: none. Unported Java methods: none.

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::mpp_field_type::MppFieldType;
use crate::data_type::DataType;
use crate::field_type_class::FieldTypeClass;
use crate::project_file::ProjectFile;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;
use crate::user_defined_field::{Builder, UserDefinedField};

/// Corresponds to Java: `org.mpxj.common.MPPProjectField`
pub struct MPPProjectField;

impl MPPProjectField {
    /// The field ID base for enterprise custom fields.
    ///
    /// Corresponds to Java: `public static final int PROJECT_FIELD_BASE = 0x0B600000;`
    pub const PROJECT_FIELD_BASE: i32 = 0x0B60_0000;

    /// Get the corresponding field type from a field ID in a Microsoft Project file.
    ///
    /// Corresponds to Java: `public static FieldType getInstance(ProjectFile project, int value)`
    ///
    /// See Java lines 47-50: `return getInstance(project, value, DataType.CUSTOM);`
    pub fn get_instance(project: &Rc<RefCell<ProjectFile>>, value: i32) -> Option<MppFieldType> {
        Self::get_instance_with_data_type(project, value, DataType::Custom)
    }

    /// Get the corresponding field type from a field ID in a Microsoft Project file, specifying
    /// the data type of custom fields.
    ///
    /// Corresponds to Java: `public static FieldType getInstance(ProjectFile project, int value, DataType customFieldDataType)`
    ///
    /// See Java lines 61-78.
    pub fn get_instance_with_data_type(
        project: &Rc<RefCell<ProjectFile>>,
        value: i32,
        custom_field_data_type: DataType,
    ) -> Option<MppFieldType> {
        // Java: `if ((value & 0x8000) != 0) { return project.getUserDefinedFields().getOrCreateProjectField(...); }`
        if (value & 0x8000) != 0 {
            return get_or_create_enterprise_field(project, value, custom_field_data_type)
                .map(MppFieldType::Custom);
        }

        // Java: `return null;` (project fields have no built-in field mapping table)
        None
    }

    /// Get the field ID used by the field in Microsoft Project.
    ///
    /// Corresponds to Java: `public static int getID(FieldType value)`
    ///
    /// See Java lines 86-98: only `UserDefinedField` returns `getValue()`, everything else returns
    /// `-1`.
    pub fn get_id(value: &MppFieldType) -> i32 {
        match value {
            // Java: `if (value instanceof UserDefinedField) { result = value.getValue(); } else { result = -1; }`
            MppFieldType::Custom(field) => match field.try_borrow() {
                Ok(field) => field.get_value(),
                // Rust-specific: cannot read while the field is mutably borrowed (no such case in Java).
                Err(_) => -1,
            },
            MppFieldType::Builtin(_) => -1,
        }
    }
}

/// Get (creating if necessary) the `UserDefinedField` corresponding to an enterprise custom field.
///
/// Corresponds to Java: `MPPProjectField.java` lines 65-74, i.e. the code executed when the
/// `0x8000` bit is set
///
/// ```java
/// return project.getUserDefinedFields().getOrCreateProjectField(Integer.valueOf(value), (k) -> {
///    int id = (k.intValue() & 0xFFF) + 1;
///    return new UserDefinedField.Builder(project)
///       .uniqueID(Integer.valueOf(PROJECT_FIELD_BASE + k.intValue()))
///       .internalName("ENTERPRISE_CUSTOM_FIELD" + id)
///       .externalName("Enterprise Custom Field " + id)
///       .fieldTypeClass(FieldTypeClass.PROJECT)
///       .dataType(customFieldDataType)
///       .build();
/// });
/// ```
///
/// # Differences from Java
///
/// - Java's `getOrCreateProjectField` keys on the MPP field ID `value`, returning the existing
///   field on a hit and calling the create function only on a miss; Rust's
///   [`crate::user_defined_field_container::UserDefinedFieldContainer::get_or_create_project_field`]
///   requires a `&mut` container, while `Builder::build()` also needs to borrow the same context
///   to get the sequence generator, and the two cannot be done within a single borrow. So this
///   is rewritten as two steps: first deduplicate in the unique ID index by the **equivalent,
///   one-to-one** unique ID (`PROJECT_FIELD_BASE + value`), and only build and register on a
///   miss. When it already exists, nothing is built, so just like Java no extra sequence number
///   is requested.
/// - Java uses `project` (`ProjectFile`) as the sequence generator provider; the Rust side passes
///   in `ProjectContext` directly, because `impl UniqueIdObjectSequenceProvider for ProjectFile`
///   already forwards to the context (sharing the same batch of sequence generators, see
///   `crate::project_file`).
/// - Returns `None` in cases such as the project being mutably borrowed so the context cannot be
///   obtained (no such case in Java).
fn get_or_create_enterprise_field(
    project: &Rc<RefCell<ProjectFile>>,
    value: i32,
    custom_field_data_type: DataType,
) -> Option<Rc<RefCell<UserDefinedField>>> {
    let context = project.try_borrow().ok()?.get_project_context();

    // Java: `PROJECT_FIELD_BASE + k.intValue()`
    let unique_id = MPPProjectField::PROJECT_FIELD_BASE + value;
    let existing = context
        .try_borrow()
        .ok()
        .and_then(|context| context.get_user_defined_fields().get_by_unique_id(Some(unique_id)));

    if let Some(existing) = existing {
        return Some(existing);
    }

    // Java: `int id = (k.intValue() & 0xFFF) + 1;`
    let id = (value & 0xFFF) + 1;
    let provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> = context.clone();
    let field = Builder::new(provider)
        .unique_id(Some(unique_id))
        .internal_name(Some(format!("ENTERPRISE_CUSTOM_FIELD{id}")))
        .external_name(Some(format!("Enterprise Custom Field {id}")))
        .field_type_class(FieldTypeClass::Project)
        .data_type(custom_field_data_type)
        .build();

    // Java: `getOrCreateProjectField(...)` (we already confirmed above that the field does not exist, so this creates it).
    // The field is fully built before acquiring the mutable borrow, avoiding a borrow conflict with the container.
    let created = context
        .try_borrow_mut()
        .ok()?
        .get_user_defined_fields_mut()
        .get_or_create_project_field(value, move |_| field);

    Some(created)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_key::FieldKey;
    use crate::project_field::ProjectField;
    use crate::resource_field::ResourceField;

    /// Create a new project file (the custom field branch needs a project context).
    fn project() -> Rc<RefCell<ProjectFile>> {
        ProjectFile::new()
    }

    #[test]
    fn project_field_base_matches_java() {
        assert_eq!(MPPProjectField::PROJECT_FIELD_BASE, 0x0B60_0000);
    }

    #[test]
    fn builtin_project_fields_have_no_mpp_mapping() {
        let project = project();

        // Java: always `return null;` for non-enterprise custom fields
        for value in [0, 1, 2, 99, 181, 182, 1000] {
            assert!(
                MPPProjectField::get_instance(&project, value).is_none(),
                "field number {value} returns null in Java"
            );
        }

        // Negative field ID: the `0x8000` bit of `i32::MIN` is 0, so it takes the built-in branch → None;
        // whereas `-1` (all bits set) has the `0x8000` bit set, so both Java and Rust take the enterprise custom field branch.
        assert!(MPPProjectField::get_instance(&project, i32::MIN).is_none());
        assert!(MPPProjectField::get_instance(&project, -1).is_some_and(|field| field.is_custom()));
        // The largest value before the `0x8000` bit is set still takes the built-in branch
        assert!(MPPProjectField::get_instance(&project, 0x7FFF).is_none());

        // Java: getID always returns -1 for non-UserDefinedField (regardless of field type)
        assert_eq!(
            MPPProjectField::get_id(&MppFieldType::Builtin(FieldKey::Project(
                ProjectField::Name
            ))),
            -1
        );
        assert_eq!(
            MPPProjectField::get_id(&MppFieldType::Builtin(FieldKey::Resource(
                ResourceField::Name
            ))),
            -1
        );
    }

    #[test]
    fn enterprise_custom_fields_are_created_once_and_reused() {
        let project = project();
        let value = 0x8000 | 3;

        let field = MPPProjectField::get_instance(&project, value).expect("should be enterprise custom field");
        let custom = field.as_custom().expect("should be custom field");
        assert!(field.is_custom());
        assert_eq!(
            custom.borrow().get_unique_id(),
            MPPProjectField::PROJECT_FIELD_BASE + value
        );
        assert_eq!(custom.borrow().name(), "ENTERPRISE_CUSTOM_FIELD4");
        assert_eq!(custom.borrow().get_name(), Some("Enterprise Custom Field 4"));
        assert_eq!(custom.borrow().get_field_type_class(), Some(FieldTypeClass::Project));
        assert_eq!(custom.borrow().get_data_type(), Some(DataType::Custom));
        assert_eq!(
            MPPProjectField::get_id(&field),
            MPPProjectField::PROJECT_FIELD_BASE + value
        );

        // Java's getOrCreateProjectField reuses the existing field (does not rebuild)
        let again = MPPProjectField::get_instance(&project, value).expect("should be enterprise custom field");
        let again = again.as_custom().expect("should be custom field");
        assert!(Rc::ptr_eq(custom, again));
        assert_eq!(
            project
                .borrow()
                .get_user_defined_fields()
                .get_project_fields()
                .len(),
            1
        );
    }

    #[test]
    fn custom_field_uses_requested_data_type() {
        let project = project();
        let value = 0x8000 | 0x2A;

        let field =
            MPPProjectField::get_instance_with_data_type(&project, value, DataType::Numeric)
                .expect("should be enterprise custom field");
        let custom = field.as_custom().expect("should be custom field");
        assert_eq!(custom.borrow().get_data_type(), Some(DataType::Numeric));
        // Java: id = (value & 0xFFF) + 1 = 0x2B
        assert_eq!(
            custom.borrow().get_name(),
            Some("Enterprise Custom Field 43")
        );
    }
}
