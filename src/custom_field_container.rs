//! Corresponds to Java: `org/mpxj/CustomFieldContainer.java`

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;
use uuid::Uuid;

use crate::custom_field::CustomField;
use crate::custom_field_lookup_table::CustomFieldValueItem;
use crate::field_key::FieldKey;
use crate::field_type::FieldType;
use crate::field_type_class::FieldTypeClass;

/// Container holding all custom field configurations.
///
/// Corresponds to Java: `public class CustomFieldContainer implements Iterable<CustomField>`
///
/// # Design trade-off: `CustomField` refers back to the parent container with `Weak`
///
/// Java's `CustomField` holds a parent container reference (`m_parent`) and calls
/// `m_parent.registerAlias(...)` in `setAlias`. On the Rust side a strong `Rc` reference would form
/// a "container → field → container" cycle causing a memory leak, so `CustomField` uses
/// `Weak<RefCell<CustomFieldContainer>>` to refer back to the parent container (following the "back-references use `Weak`" convention).
///
/// This introduces an API difference: the `Rc` for `self` cannot be obtained from `&mut self`, so
/// - [`new`](CustomFieldContainer::new) returns `Rc<RefCell<Self>>` directly;
/// - the "create field" methods [`add`](CustomFieldContainer::add) /
/// [`get_or_create`](CustomFieldContainer::get_or_create) are associated functions taking
/// `&Rc<RefCell<Self>>` (in Java they are instance methods).
///
/// Also: when calling these two methods you must **not** simultaneously hold a `borrow()` / `borrow_mut()` on the container,
/// otherwise it panics at runtime (a `RefCell` borrow conflict).
///
/// # Differences from Java
///
/// - `Map<FieldType, CustomField>` → `HashMap<FieldKey, Rc<RefCell<CustomField>>>`
/// (`FieldKey` is this crate's runtime erasure of the `FieldType` interface, but it **does not include**
/// `UserDefinedField`, so this container can currently only store configurations for built-in fields).
/// - `Map<Integer, CustomFieldValueItem>` / `Map<UUID, CustomFieldValueItem>` →
/// The value is `Rc<RefCell<CustomFieldValueItem>>`; the element type comes from
/// [`crate::custom_field_lookup_table::CustomFieldValueItem`], which is
/// the Rust-side field carrier for `org.mpxj.mpp.CustomFieldValueItem`
/// (the `mpp`-side [`crate::mpp::custom_field_value_item::CustomFieldValueItem`] is its
/// shared handle; both read and write the same field).
/// - `Map<Pair<FieldTypeClass, String>, FieldType>` → `HashMap<(FieldTypeClass, String), FieldKey>`.
/// - Java's `stream()` (`Stream<CustomField>`) is not ported: on the Rust side
/// [`iter`](CustomFieldContainer::iter) provides the equivalent capability.
///
/// Skipped Java mechanisms: `Stream` / `StreamSupport` (see above).
///
/// Unported Java methods: `stream()` (see above).
pub struct CustomFieldContainer {
/// Field type → configuration.
    ///
    /// Corresponds to Java: `private final Map<FieldType, CustomField> m_configMap = new HashMap<>();`
    config_map: HashMap<FieldKey, Rc<RefCell<CustomField>>>,
/// Value item unique ID → value item.
    ///
    /// Corresponds to Java: `private final Map<Integer, CustomFieldValueItem> m_valueMap = new HashMap<>();`
    value_map: HashMap<i32, Rc<RefCell<CustomFieldValueItem>>>,
/// Value item GUID → value item.
    ///
    /// Corresponds to Java: `private final Map<UUID, CustomFieldValueItem> m_guidMap = new HashMap<>();`
    guid_map: HashMap<Uuid, Rc<RefCell<CustomFieldValueItem>>>,
/// Alias index: `(entity type, alias)` → field type.
    ///
    /// Corresponds to Java: `private final Map<Pair<FieldTypeClass, String>, FieldType> m_aliasMap = new HashMap<>();`
    alias_map: HashMap<(FieldTypeClass, String), FieldKey>,
}

impl CustomFieldContainer {
/// Constructor.
    ///
    /// Corresponds to Java: `public CustomFieldContainer()`
    ///
/// The Rust side returns `Rc<RefCell<Self>>`, because creating a field requires handing the parent container's `Weak` to
/// [`CustomField`] (see the type documentation).
    pub fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            config_map: HashMap::new(),
            value_map: HashMap::new(),
            guid_map: HashMap::new(),
            alias_map: HashMap::new(),
        }))
    }

/// Get the configuration by field type; returns `None` when not configured.
    ///
    /// Corresponds to Java: `public CustomField get(FieldType field)`
    pub fn get(&self, field: FieldKey) -> Option<Rc<RefCell<CustomField>>> {
        self.config_map.get(&field).cloned()
    }

/// Get the configuration by field type, creating it when absent.
    ///
    /// Corresponds to Java: `public CustomField getOrCreate(FieldType field)`
    pub fn get_or_create(
        container: &Rc<RefCell<Self>>,
        field: FieldKey,
    ) -> Rc<RefCell<CustomField>> {
        if let Some(existing) = container.borrow().get(field) {
            return existing;
        }

        Self::add(container, field)
    }

/// Create and register a field configuration (overwriting an existing configuration).
    ///
    /// Corresponds to Java: `public CustomField add(FieldType field)`
    ///
/// See the type documentation: on the Rust side this is an associated function and requires passing the parent container itself.
    pub fn add(
        container: &Rc<RefCell<Self>>,
        field: FieldKey,
    ) -> Rc<RefCell<CustomField>> {
        let custom_field = Rc::new(RefCell::new(CustomField::new(field, Rc::downgrade(container))));
        container
            .borrow_mut()
            .config_map
            .insert(field, Rc::clone(&custom_field));
        custom_field
    }

/// Remove a field configuration.
    ///
    /// Corresponds to Java: `public void remove(FieldType field)`
    pub fn remove(&mut self, field: FieldKey) {
        self.config_map.remove(&field);
    }

/// Look up a field type by alias; returns `None` when not found.
    ///
    /// Corresponds to Java: `public FieldType getFieldTypeByAlias(FieldTypeClass typeClass, String alias)`
    pub fn get_field_type_by_alias(
        &self,
        type_class: FieldTypeClass,
        alias: &str,
    ) -> Option<FieldKey> {
        self.alias_map
            .get(&(type_class, alias.to_owned()))
            .copied()
    }

/// Get all field configurations for a given entity type.
    ///
    /// Corresponds to Java: `public List<CustomField> getCustomFieldsByFieldTypeClass(FieldTypeClass typeClass)`
    ///
/// Java collects them via `stream()`; the Rust side filters `iter()` directly. The return order inherits
/// the iteration order of `HashMap` (Java's `HashMap` likewise does not guarantee order).
    pub fn get_custom_fields_by_field_type_class(
        &self,
        type_class: FieldTypeClass,
    ) -> Vec<Rc<RefCell<CustomField>>> {
        self.iter()
            .filter(|custom_field| {
                custom_field.borrow().get_field_type().get_field_type_class() == type_class
            })
            .cloned()
            .collect()
    }

/// The number of field configurations.
    ///
    /// Corresponds to Java: `public int size()`
    pub fn size(&self) -> usize {
        self.config_map.len()
    }

/// Whether there are no field configurations.
    ///
/// A convenience method added on the Rust side (Java leaves this to the caller with `size() == 0`).
    pub fn is_empty(&self) -> bool {
        self.config_map.is_empty()
    }

/// Iterate over all field configurations.
    ///
    /// Corresponds to Java: `@Override public Iterator<CustomField> iterator()`
    pub fn iter(&self) -> impl Iterator<Item = &Rc<RefCell<CustomField>>> {
        self.config_map.values()
    }

/// Get a custom field value item by unique ID.
    ///
    /// Corresponds to Java: `public CustomFieldValueItem getCustomFieldValueItemByUniqueID(int uniqueID)`
    ///
/// The Java comment notes "used only when reading MPP files".
    pub fn get_custom_field_value_item_by_unique_id(
        &self,
        unique_id: i32,
    ) -> Option<Rc<RefCell<CustomFieldValueItem>>> {
        self.value_map.get(&unique_id).cloned()
    }

/// Get a custom field value item by GUID.
    ///
    /// Corresponds to Java: `public CustomFieldValueItem getCustomFieldValueItemByGuid(UUID guid)`
    ///
/// The Java comment notes "used only when reading MPP files".
    pub fn get_custom_field_value_item_by_guid(
        &self,
        guid: Uuid,
    ) -> Option<Rc<RefCell<CustomFieldValueItem>>> {
        self.guid_map.get(&guid).cloned()
    }

/// Register a value item in the index.
    ///
    /// Corresponds to Java: `public void registerValue(CustomFieldValueItem item)`
    ///
/// The Java comment notes "used only when reading MPP files".
    pub fn register_value(&mut self, item: Rc<RefCell<CustomFieldValueItem>>) {
        let unique_id = item.borrow().get_unique_id();
        let guid = item.borrow().get_guid();

        self.value_map.insert(unique_id, Rc::clone(&item));

        if let Some(guid) = guid {
            self.guid_map.insert(guid, item);
        }
    }

/// Remove a value item from the index.
    ///
    /// Corresponds to Java: `public void deregisterValue(CustomFieldValueItem item)`
    ///
/// The Java comment notes "used only when reading MPP files".
    pub fn deregister_value(&mut self, item: &Rc<RefCell<CustomFieldValueItem>>) {
        self.value_map.remove(&item.borrow().get_unique_id());

        if let Some(guid) = item.borrow().get_guid() {
            self.guid_map.remove(&guid);
        }
    }

/// Register the field alias index.
    ///
/// Corresponds to Java: `void registerAlias(FieldType type, String alias)` (package-private)
    ///
/// Triggered by the [`CustomField::set_alias`] callback; the Rust visibility is `pub(crate)`, matching Java's
/// package-private visibility.
    pub(crate) fn register_alias(&mut self, field_type: FieldKey, alias: &str) {
        self.alias_map.insert(
            (field_type.get_field_type_class(), alias.to_owned()),
            field_type,
        );
    }
}

impl Debug for CustomFieldContainer {
/// Java does not override `toString()`; the Rust side prints the individual indexes (`CustomField`'s `Debug` does not recurse
/// into the parent container, see the documentation of [`CustomField`]).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomFieldContainer")
            .field("config_map", &self.config_map)
            .field("value_map", &self.value_map)
            .field("guid_map", &self.guid_map)
            .field("alias_map", &self.alias_map)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::custom_field_value_data_type::CustomFieldValueDataType;
    use crate::resource_field::ResourceField;
    use crate::task_field::TaskField;
    use crate::project_field::ProjectField;

    #[test]
    fn add_get_and_remove() {
        let container = CustomFieldContainer::new();

        assert!(container.borrow().is_empty());
        assert!(container.borrow().get(FieldKey::Task(TaskField::Text1)).is_none());

        let field = CustomFieldContainer::add(&container, FieldKey::Task(TaskField::Text1));
        assert_eq!(container.borrow().size(), 1);
        assert!(Rc::ptr_eq(
            &container
                .borrow()
                .get(FieldKey::Task(TaskField::Text1))
                .expect("registered"),
            &field
        ));

// add overwrites an existing configuration
        let replacement = CustomFieldContainer::add(&container, FieldKey::Task(TaskField::Text1));
        assert_eq!(container.borrow().size(), 1);
        assert!(!Rc::ptr_eq(&replacement, &field));

        container
            .borrow_mut()
            .remove(FieldKey::Task(TaskField::Text1));
        assert!(container.borrow().is_empty());
    }

    #[test]
    fn get_or_create_reuses_existing_configuration() {
        let container = CustomFieldContainer::new();

        let first =
            CustomFieldContainer::get_or_create(&container, FieldKey::Task(TaskField::Number1));
        let second =
            CustomFieldContainer::get_or_create(&container, FieldKey::Task(TaskField::Number1));

        assert!(Rc::ptr_eq(&first, &second));
        assert_eq!(container.borrow().size(), 1);
    }

    #[test]
    fn alias_index_is_registered_and_queried() {
        let container = CustomFieldContainer::new();
        let field = CustomFieldContainer::add(&container, FieldKey::Task(TaskField::Text1));

        field.borrow_mut().set_alias(Some("MyAlias".to_owned()));

        assert_eq!(
            container
                .borrow()
                .get_field_type_by_alias(FieldTypeClass::Task, "MyAlias"),
            Some(FieldKey::Task(TaskField::Text1))
        );

// Not found when the entity type does not match
        assert_eq!(
            container
                .borrow()
                .get_field_type_by_alias(FieldTypeClass::Resource, "MyAlias"),
            None
        );
        assert_eq!(
            container
                .borrow()
                .get_field_type_by_alias(FieldTypeClass::Task, "Other"),
            None
        );
    }

    #[test]
    fn custom_fields_can_be_filtered_by_field_type_class() {
        let container = CustomFieldContainer::new();
        CustomFieldContainer::add(&container, FieldKey::Task(TaskField::Text1));
        CustomFieldContainer::add(&container, FieldKey::Task(TaskField::Number1));
        CustomFieldContainer::add(&container, FieldKey::Resource(ResourceField::Text1));
        CustomFieldContainer::add(&container, FieldKey::Project(ProjectField::Cost));

        let container = container.borrow();
        assert_eq!(container.get_custom_fields_by_field_type_class(FieldTypeClass::Task).len(), 2);
        assert_eq!(
            container
                .get_custom_fields_by_field_type_class(FieldTypeClass::Resource)
                .len(),
            1
        );
        assert_eq!(
            container
                .get_custom_fields_by_field_type_class(FieldTypeClass::Project)
                .len(),
            1
        );
        assert_eq!(
            container
                .get_custom_fields_by_field_type_class(FieldTypeClass::Assignment)
                .len(),
            0
        );
        assert_eq!(container.iter().count(), 4);
    }

    #[test]
    fn value_items_can_be_indexed_by_unique_id_and_guid() {
        let container = CustomFieldContainer::new();
        let guid = Uuid::from_u128(7);

        let mut item = CustomFieldValueItem::new(11);
        item.set_guid(Some(guid));
        item.set_type(Some(CustomFieldValueDataType::Text));
        let item = Rc::new(RefCell::new(item));

        container.borrow_mut().register_value(Rc::clone(&item));

        assert!(Rc::ptr_eq(
            &container
                .borrow()
                .get_custom_field_value_item_by_unique_id(11)
                .expect("registered"),
            &item
        ));
        assert!(Rc::ptr_eq(
            &container
                .borrow()
                .get_custom_field_value_item_by_guid(guid)
                .expect("registered"),
            &item
        ));
        assert!(container
            .borrow()
            .get_custom_field_value_item_by_unique_id(12)
            .is_none());
        assert!(container
            .borrow()
            .get_custom_field_value_item_by_guid(Uuid::from_u128(8))
            .is_none());

// After unregistering, neither index matches anymore
        container.borrow_mut().deregister_value(&item);
        assert!(container
            .borrow()
            .get_custom_field_value_item_by_unique_id(11)
            .is_none());
        assert!(container
            .borrow()
            .get_custom_field_value_item_by_guid(guid)
            .is_none());
    }

    #[test]
    fn value_item_without_guid_is_indexed_by_unique_id_only() {
        let container = CustomFieldContainer::new();
        let item = Rc::new(RefCell::new(CustomFieldValueItem::new(3)));

        container.borrow_mut().register_value(Rc::clone(&item));

        assert!(container
            .borrow()
            .get_custom_field_value_item_by_unique_id(3)
            .is_some());

        container.borrow_mut().deregister_value(&item);
        assert!(container
            .borrow()
            .get_custom_field_value_item_by_unique_id(3)
            .is_none());
    }
}
