//! Corresponds to Java: `org/mpxj/CustomField.java`

use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::rc::Weak;

use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::mpp_assignment_field::MPPAssignmentField;
use crate::common::mpp_field_type::MppFieldType;
use crate::common::mpp_project_field::MPPProjectField;
use crate::common::mpp_resource_field::MPPResourceField;
use crate::common::mpp_task_field::MPPTaskField;
use crate::custom_field_container::CustomFieldContainer;
use crate::custom_field_lookup_table::CustomFieldLookupTable;
use crate::custom_field_value_mask::CustomFieldValueMask;
use crate::field_key::FieldKey;
use crate::field_type::FieldType;
use crate::field_type_class::FieldTypeClass;
use crate::graphical_indicator::GraphicalIndicator;

/// Configuration information for a single field.
///
/// Corresponds to Java: `public class CustomField implements Comparable<CustomField>`
///
/// # Field Mapping and Structural Trade-offs
///
/// | Java | Rust |
/// |---|---|
/// | `private final FieldType m_field` | `FieldKey` (this crate's runtime erasure of the `FieldType` interface, see [`crate::field_key`]) |
/// | `private final CustomFieldContainer m_parent` | `Weak<RefCell<CustomFieldContainer>>` (points back to the parent container to register aliases in `setAlias`; `Weak` avoids an `Rc` cycle) |
/// | `private final CustomFieldLookupTable m_table` | [`CustomFieldLookupTable`] |
/// | `private final GraphicalIndicator m_indicator` | [`GraphicalIndicator`] (`src/graphical_indicator.rs`) |
/// | `private final List<CustomFieldValueMask> m_masks` | `Vec<CustomFieldValueMask>` |
/// | `private Integer m_uniqueID` | `Cell<Option<i32>>` (Java's `getUniqueID()` lazily back-fills this field; `Cell` provides equivalent mutability, see below) |
/// | `private String m_alias` | `Option<String>` |
///
/// # Differences from Java
///
/// - The lazy fallback of `getUniqueID()` **is implemented** (see [`CustomField::get_unique_id`]):
///   Java writes `FieldTypeHelper.getFieldID(m_field)` and returns it when the value is `null`;
///   the Rust side stores `m_unique_id` in a [`std::cell::Cell`], so "read fallback + back-fill
///   cache" can also be performed under `&self`. When `m_field` is [`FieldKey::Custom`], it is
///   computed via Java's `UserDefinedField.getValue()` branch (see module-level
///   `fallback_unique_id`).
/// - `getGraphicalIndicator()` returns a borrow of `m_indicator` and additionally provides
///   [`CustomField::get_graphical_indicator_mut`]: Java's `GraphicalIndicator` has reference
///   semantics, and callers modify it directly after obtaining it; the Rust instance is
///   usually shared via `Rc<RefCell<CustomField>>`, so following this crate's existing
///   practice (see `get_lookup_table_mut` / `get_masks_mut`) a separate mutable accessor is
///   provided.
///
/// Unported Java methods: none.
pub struct CustomField {
    /// Field type corresponding to this configuration.
    ///
    /// Corresponds to Java: `private final FieldType m_field;`
    m_field: FieldKey,
    /// Parent container.
    ///
    /// Corresponds to Java: `private final CustomFieldContainer m_parent;`
    m_parent: Weak<RefCell<CustomFieldContainer>>,
    /// Value lookup table.
    ///
    /// Corresponds to Java: `private final CustomFieldLookupTable m_table;`
    m_table: CustomFieldLookupTable,
    /// Graphical indicator configuration.
    ///
    /// Corresponds to Java: `private final GraphicalIndicator m_indicator;`
    m_indicator: GraphicalIndicator,
    /// Mask definitions.
    ///
    /// Corresponds to Java: `private final List<CustomFieldValueMask> m_masks;`
    m_masks: Vec<CustomFieldValueMask>,
    /// Unique ID.
    ///
    /// Corresponds to Java: `private Integer m_uniqueID;`
    ///
    /// A [`Cell`] is used instead of `Option<i32>`: Java's `getUniqueID()` writes the fallback
    /// result back to this field when it has not been set explicitly (lazy caching), and
    /// reproducing that behavior under `&self` requires interior mutability.
    m_unique_id: Cell<Option<i32>>,
    /// Field alias.
    ///
    /// Corresponds to Java: `private String m_alias;`
    m_alias: Option<String>,
}

impl CustomField {
    /// Constructor.
    ///
    /// Corresponds to Java: `public CustomField(FieldType field, CustomFieldContainer parent)`
    ///
    /// Java's `parent` cannot be `null`; the Rust side allows passing `Weak::new()` to mean
    /// "no parent container" (in which case `set_alias` does not register an alias outward),
    /// which makes it convenient to construct fields standalone for tests.
    pub fn new(field: FieldKey, parent: Weak<RefCell<CustomFieldContainer>>) -> Self {
        Self {
            m_field: field,
            m_parent: parent,
            m_table: CustomFieldLookupTable::new(),
            m_indicator: GraphicalIndicator::new(field),
            m_masks: Vec::new(),
            m_unique_id: Cell::new(None),
            m_alias: None,
        }
    }

    /// Return the field type corresponding to this configuration.
    ///
    /// Corresponds to Java: `public FieldType getFieldType()`
    pub fn get_field_type(&self) -> FieldKey {
        self.m_field
    }

    /// Return the value lookup table.
    ///
    /// Corresponds to Java: `public CustomFieldLookupTable getLookupTable()`
    pub fn get_lookup_table(&self) -> &CustomFieldLookupTable {
        &self.m_table
    }

    /// Return the value lookup table mutably (the object Java returns is itself mutable).
    ///
    /// Corresponds to Java: `public CustomFieldLookupTable getLookupTable()`
    pub fn get_lookup_table_mut(&mut self) -> &mut CustomFieldLookupTable {
        &mut self.m_table
    }

    /// Return the graphical indicator configuration for this field.
    ///
    /// Corresponds to Java: `public GraphicalIndicator getGraphicalIndicator()`
    ///
    /// Java creates the returned instance in the constructor
    /// (`m_indicator = new GraphicalIndicator(field)`), so the indicator configuration for the
    /// same field is always the same instance; `GraphicalIndicatorReader` retrieves it and then
    /// populates it.
    pub fn get_graphical_indicator(&self) -> &GraphicalIndicator {
        &self.m_indicator
    }

    /// Return the graphical indicator configuration mutably (the object Java returns is itself mutable).
    ///
    /// Corresponds to Java: `public GraphicalIndicator getGraphicalIndicator()`
    pub fn get_graphical_indicator_mut(&mut self) -> &mut GraphicalIndicator {
        &mut self.m_indicator
    }

    /// Return the field alias.
    ///
    /// Corresponds to Java: `public String getAlias()`
    pub fn get_alias(&self) -> Option<&str> {
        self.m_alias.as_deref()
    }

    /// Set the field alias and register the alias index in the parent container.
    ///
    /// Corresponds to Java: `public CustomField setAlias(String alias)`
    ///
    /// The Java side returns `this` to support method chaining; the Rust instance is usually
    /// shared via `Rc<RefCell<CustomField>>`, so this takes `&mut self` and returns nothing.
    ///
    /// Java also calls `registerAlias` for a `null` alias; the Rust alias index key type is
    /// `(FieldTypeClass, String)`, which cannot express `null`, so registration only happens
    /// when the alias is not `None`.
    pub fn set_alias(&mut self, alias: Option<String>) {
        if let Some(alias) = alias.as_deref() {
            if let Some(parent) = self.m_parent.upgrade() {
                parent.borrow_mut().register_alias(self.m_field, alias);
            }
        }

        self.m_alias = alias;
    }

    /// Return the unique ID.
    ///
    /// Corresponds to Java: `public Integer getUniqueID()`
    ///
    /// Java: when not set explicitly, falls back to [`FieldTypeHelper::get_field_id`] (i.e. the
    /// field number used by MS Project) and caches the result; reading again after
    /// `setUniqueID(null)` falls back anew. See module-level `fallback_unique_id` for the
    /// fallback computation (`FieldKey::Custom` must be handled via the
    /// `UserDefinedField.getValue()` branch and cannot use the built-in field lookup branch).
    pub fn get_unique_id(&self) -> Option<i32> {
        // Java: if (m_uniqueID == null) { m_uniqueID = Integer.valueOf(...); }
        if let Some(unique_id) = self.m_unique_id.get() {
            return Some(unique_id);
        }

        let unique_id = fallback_unique_id(self.m_field);
        self.m_unique_id.set(Some(unique_id));
        // Java's `getUniqueID()` always returns a non-null `Integer`; the Rust side keeps `Option`
        // only to express the "not yet computed" cache state, and the return value is always `Some`.
        Some(unique_id)
    }

    /// Set the unique ID.
    ///
    /// Corresponds to Java: `public CustomField setUniqueID(Integer uniqueID)`
    pub fn set_unique_id(&mut self, unique_id: Option<i32>) {
        self.m_unique_id.set(unique_id);
    }

    /// Return the mask definitions.
    ///
    /// Corresponds to Java: `public List<CustomFieldValueMask> getMasks()`
    pub fn get_masks(&self) -> &[CustomFieldValueMask] {
        &self.m_masks
    }

    /// Return the mask definitions mutably (the list Java returns is itself mutable, so callers can `add` directly).
    ///
    /// Corresponds to Java: `public List<CustomFieldValueMask> getMasks()`
    pub fn get_masks_mut(&mut self) -> &mut Vec<CustomFieldValueMask> {
        &mut self.m_masks
    }

    /// Sort key corresponding to the string concatenated in Java's `compareTo`:
    /// `fieldTypeClass.name() + "." + uniqueID + "." + alias`.
    ///
    /// Java uses `getUniqueID()` (which falls back lazily), so this likewise goes through
    /// [`CustomField::get_unique_id`] rather than reading the cached field directly.
    fn sort_key(&self) -> String {
        format!(
            "{}.{}.{}",
            field_type_class_name(self.m_field.get_field_type_class()),
            self.get_unique_id().unwrap_or(-1),
            // Java: "+ getAlias()"; outputs "null" when the alias is not set
            self.m_alias.as_deref().unwrap_or("null")
        )
    }
}

/// Reproduce Java `FieldTypeHelper.getFieldID(m_field)` (the fallback value of `CustomField.getUniqueID`).
///
/// Built-in fields forward directly to [`FieldTypeHelper::get_field_id`].
///
/// `FieldKey::Custom` must be handled separately: Java's `FieldTypeHelper.getFieldID`
/// dispatches on `FieldTypeClass` to `MPPTaskField.getID(...)` etc., and internally `getID`
/// takes the `value.getValue()` branch for `UserDefinedField` (returning the field's unique
/// ID) rather than the `ID_ARRAY` lookup. Rust's [`FieldKey::Custom`] is a `Copy` equivalent
/// key and cannot obtain a `UserDefinedField` handle; passing it as a built-in field
/// (`MppFieldType::Builtin`) would fall into the lookup branch (a custom field's unique ID
/// far exceeds the length of `ID_ARRAY`, giving -1), so this computes manually with the
/// semantics of `value.getValue()`, consistent with Java.
fn fallback_unique_id(field: FieldKey) -> i32 {
    match field {
        FieldKey::Custom(key) => match key.get_field_type_class() {
            // Java: result = MPPTaskField.TASK_FIELD_BASE | MPPTaskField.getID(type)
            FieldTypeClass::Task => MPPTaskField::TASK_FIELD_BASE | key.get_unique_id(),
            // Java: result = MPPResourceField.RESOURCE_FIELD_BASE | MPPResourceField.getID(type)
            FieldTypeClass::Resource => MPPResourceField::RESOURCE_FIELD_BASE | key.get_unique_id(),
            // Java: result = MPPAssignmentField.ASSIGNMENT_FIELD_BASE | MPPAssignmentField.getID(type)
            FieldTypeClass::Assignment => {
                MPPAssignmentField::ASSIGNMENT_FIELD_BASE | key.get_unique_id()
            }
            // Java: result = MPPProjectField.PROJECT_FIELD_BASE | MPPProjectField.getID(type)
            FieldTypeClass::Project => MPPProjectField::PROJECT_FIELD_BASE | key.get_unique_id(),
            // Java: default: result = -1; (Constraint / Unknown)
            _ => -1,
        },
        // Java: switch (type.getFieldTypeClass()) { case TASK/RESOURCE/ASSIGNMENT/PROJECT: ... }
        other => FieldTypeHelper::get_field_id(Some(&MppFieldType::from(other))),
    }
}

/// Return the Java enum name of `FieldTypeClass` (`Enum.name()`), used to reproduce the sort key of `compareTo`.
fn field_type_class_name(type_class: FieldTypeClass) -> &'static str {
    match type_class {
        FieldTypeClass::Task => "TASK",
        FieldTypeClass::Resource => "RESOURCE",
        FieldTypeClass::Assignment => "ASSIGNMENT",
        FieldTypeClass::Constraint => "CONSTRAINT",
        FieldTypeClass::Project => "PROJECT",
        FieldTypeClass::Unknown => "UNKNOWN",
    }
}

impl PartialEq for CustomField {
    /// Java does not override `equals` (it uses reference equality) but implements `Comparable`.
    /// Rust's `Ord` requires a matching `Eq`, so this is implemented via the sort key of
    /// `compareTo` — consistent with Java's `TreeSet` / `TreeMap` using `compareTo` for equality.
    fn eq(&self, other: &Self) -> bool {
        self.sort_key() == other.sort_key()
    }
}

impl Eq for CustomField {}

impl PartialOrd for CustomField {
    /// Corresponds to Java: `public int compareTo(CustomField f)`
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CustomField {
    /// Corresponds to Java: `public int compareTo(CustomField f)`
    fn cmp(&self, other: &Self) -> Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl Display for CustomField {
    /// Corresponds to Java: `@Override public String toString()`
    ///
    /// Java: `"[CustomField field=" + m_field + " alias=" + m_alias + "]"`; here `m_field` is an
    /// enum and outputs the constant name (e.g. `"TEXT1"`), and `m_alias` outputs `null` when it
    /// is `null`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[CustomField field={} alias={}]",
            self.m_field.name(),
            self.m_alias.as_deref().unwrap_or("null")
        )
    }
}

impl std::fmt::Debug for CustomField {
    /// Java does not override debug output beyond `toString()`; the Rust side provides `Debug` for easier diagnosis.
    ///
    /// Deliberately **does not** output `m_parent`: child objects point back to the parent
    /// container via `Weak`, and the parent container holds all child objects, so printing the
    /// parent container would cause infinite recursion.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomField")
            .field("field", &self.m_field.name())
            // Read the cached value directly (without triggering the fallback); `Cell`'s own `Debug` output adds an extra layer of wrapping
            .field("unique_id", &self.m_unique_id.get())
            .field("alias", &self.m_alias)
            .field("table", &self.m_table)
            .field("indicator", &self.m_indicator)
            .field("masks", &self.m_masks)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::custom_field_value_data_type::CustomFieldValueDataType;
    use crate::resource_field::ResourceField;
    use crate::task_field::TaskField;

    /// Construct a field with "no parent container" (Java's `m_parent` must be non-null; the Rust side uses a dangling `Weak` instead).
    fn standalone(field: FieldKey) -> CustomField {
        CustomField::new(field, Weak::new())
    }

    #[test]
    fn accessors_round_trip() {
        let mut field = standalone(FieldKey::Task(TaskField::Text1));

        assert_eq!(field.get_field_type(), FieldKey::Task(TaskField::Text1));
        assert_eq!(field.get_alias(), None);
        assert!(field.get_masks().is_empty());
        assert!(field.get_lookup_table().is_empty());

        field.set_alias(Some("Alias".to_owned()));
        field.set_unique_id(Some(42));

        assert_eq!(field.get_alias(), Some("Alias"));
        assert_eq!(field.get_unique_id(), Some(42));
    }

    /// Java: when `getUniqueID()` is not set explicitly, falls back to
    /// `FieldTypeHelper.getFieldID(m_field)` and caches the result; falls back anew after
    /// `setUniqueID(null)`.
    #[test]
    fn unique_id_falls_back_to_field_id_and_is_cached() {
        let mut field = standalone(FieldKey::Task(TaskField::Text1));

        let expected = FieldTypeHelper::get_field_id(Some(&MppFieldType::from(FieldKey::Task(
            TaskField::Text1,
        ))));
        assert_eq!(field.get_unique_id(), Some(expected));
        // The fallback result is back-filled into the cache (visible by reading the cached field directly via Debug)
        assert_eq!(field.m_unique_id.get(), Some(expected));

        // Reading again after setUniqueID(null) → falls back anew
        field.set_unique_id(None);
        assert_eq!(field.get_unique_id(), Some(expected));

        // An explicitly set value takes precedence
        field.set_unique_id(Some(7));
        assert_eq!(field.get_unique_id(), Some(7));
    }

    /// Fallback value for custom field keys (`FieldKey::Custom`).
    ///
    /// Java's `MPPTaskField.getID(UserDefinedField)` returns `value.getValue()` (i.e. the
    /// field's unique ID) and then bitwise-ORs it with `TASK_FIELD_BASE`; it cannot use the
    /// built-in field `ID_ARRAY` lookup branch.
    #[test]
    fn unique_id_fallback_for_custom_field_keys() {
        let key = crate::field_key::CustomFieldKey::new(
            0x0B40_8000,
            FieldTypeClass::Task,
            crate::data_type::DataType::Custom,
        );
        let field = standalone(FieldKey::Custom(key));

        assert_eq!(
            field.get_unique_id(),
            Some(MPPTaskField::TASK_FIELD_BASE | 0x0B40_8000)
        );

        // Resource / assignment / project custom fields each use their own BASE
        let resource_key = crate::field_key::CustomFieldKey::new(
            5,
            FieldTypeClass::Resource,
            crate::data_type::DataType::Custom,
        );
        assert_eq!(
            standalone(FieldKey::Custom(resource_key)).get_unique_id(),
            Some(MPPResourceField::RESOURCE_FIELD_BASE | 5)
        );

        let assignment_key = crate::field_key::CustomFieldKey::new(
            5,
            FieldTypeClass::Assignment,
            crate::data_type::DataType::Custom,
        );
        assert_eq!(
            standalone(FieldKey::Custom(assignment_key)).get_unique_id(),
            Some(MPPAssignmentField::ASSIGNMENT_FIELD_BASE | 5)
        );

        let project_key = crate::field_key::CustomFieldKey::new(
            5,
            FieldTypeClass::Project,
            crate::data_type::DataType::Custom,
        );
        assert_eq!(
            standalone(FieldKey::Custom(project_key)).get_unique_id(),
            Some(MPPProjectField::PROJECT_FIELD_BASE | 5)
        );

        // Java: default: result = -1; (constraint fields do not participate in getFieldID's switch)
        let constraint_key = crate::field_key::CustomFieldKey::new(
            5,
            FieldTypeClass::Constraint,
            crate::data_type::DataType::Custom,
        );
        assert_eq!(standalone(FieldKey::Custom(constraint_key)).get_unique_id(), Some(-1));
    }

    #[test]
    fn graphical_indicator_is_created_in_constructor() {
        let mut field = standalone(FieldKey::Task(TaskField::Text1));

        // Java: m_indicator = new GraphicalIndicator(field)
        assert_eq!(
            field.get_graphical_indicator().get_field_type(),
            FieldKey::Task(TaskField::Text1)
        );
        assert!(!field.get_graphical_indicator().get_display_graphical_indicators());

        // Java's indicator object is itself mutable (the same instance is read and written)
        field
            .get_graphical_indicator_mut()
            .set_display_graphical_indicators(true);
        assert!(field.get_graphical_indicator().get_display_graphical_indicators());
    }

    #[test]
    fn lookup_table_and_masks_are_mutable() {
        let mut field = standalone(FieldKey::Task(TaskField::Number1));

        field
            .get_lookup_table_mut()
            .add(crate::custom_field_lookup_table::CustomFieldValueItem::new(1));
        assert_eq!(field.get_lookup_table().len(), 1);

        field.get_masks_mut().push(CustomFieldValueMask::new(
            10,
            1,
            Some("-".to_owned()),
            Some(CustomFieldValueDataType::Text),
        ));
        assert_eq!(field.get_masks().len(), 1);
        assert_eq!(field.get_masks()[0].get_length(), 10);
    }

    #[test]
    fn set_alias_registers_with_parent_container() {
        let container = CustomFieldContainer::new();
        let field = CustomFieldContainer::add(&container, FieldKey::Task(TaskField::Text1));

        field.borrow_mut().set_alias(Some("MyAlias".to_owned()));

        assert_eq!(
            container
                .borrow()
                .get_field_type_by_alias(FieldTypeClass::Task, "MyAlias"),
            Some(FieldKey::Task(TaskField::Text1))
        );
    }

    #[test]
    fn set_alias_with_none_does_not_register() {
        let container = CustomFieldContainer::new();
        let field = CustomFieldContainer::add(&container, FieldKey::Task(TaskField::Text1));

        field.borrow_mut().set_alias(None);

        assert_eq!(
            container
                .borrow()
                .get_field_type_by_alias(FieldTypeClass::Task, "MyAlias"),
            None
        );
    }

    #[test]
    fn ordering_follows_field_type_class_then_unique_id_then_alias() {
        let mut task_field = standalone(FieldKey::Task(TaskField::Text1));
        task_field.set_unique_id(Some(1));
        task_field.set_alias(Some("A".to_owned()));

        let mut resource_field = standalone(FieldKey::Resource(ResourceField::Text1));
        resource_field.set_unique_id(Some(1));
        resource_field.set_alias(Some("A".to_owned()));

        // RESOURCE < TASK (compared by enum name)
        assert!(resource_field < task_field);

        // Within the same type, the smaller unique ID comes first
        let mut task_field_two = standalone(FieldKey::Task(TaskField::Text2));
        task_field_two.set_unique_id(Some(2));
        task_field_two.set_alias(Some("A".to_owned()));
        assert!(task_field < task_field_two);

        // For the same type and ID, compare by alias
        let mut task_field_alias_b = standalone(FieldKey::Task(TaskField::Text1));
        task_field_alias_b.set_unique_id(Some(1));
        task_field_alias_b.set_alias(Some("B".to_owned()));
        assert!(task_field < task_field_alias_b);
    }

    /// Java's `compareTo` builds the sort key with `getUniqueID()` (lazy fallback), so a field
    /// whose ID is not set explicitly gets the fallback field number rather than the string
    /// `"null"`.
    #[test]
    fn sort_key_uses_the_fallback_unique_id() {
        let mut unset = standalone(FieldKey::Task(TaskField::Text1));
        unset.set_alias(Some("A".to_owned()));

        let fallback = FieldTypeHelper::get_field_id(Some(&MppFieldType::from(FieldKey::Task(
            TaskField::Text1,
        ))));
        assert_eq!(unset.sort_key(), format!("TASK.{fallback}.A"));

        // The explicitly set ID participates in string comparison: "1" < the fallback field number (multiple digits)
        let mut set = standalone(FieldKey::Task(TaskField::Text1));
        set.set_unique_id(Some(1));
        set.set_alias(Some("A".to_owned()));
        assert!(set.sort_key() < unset.sort_key());
    }

    #[test]
    fn display_matches_java_format() {
        let mut field = standalone(FieldKey::Task(TaskField::Text1));

        assert_eq!(field.to_string(), "[CustomField field=TEXT1 alias=null]");

        field.set_alias(Some("Alias".to_owned()));
        assert_eq!(field.to_string(), "[CustomField field=TEXT1 alias=Alias]");
    }
}
