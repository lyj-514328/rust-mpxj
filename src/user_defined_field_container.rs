//! Corresponds to Java: `org/mpxj/UserDefinedFieldContainer.java`

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use crate::field_type_class::FieldTypeClass;
use crate::user_defined_field::UserDefinedField;

/// Manages all user-defined fields under a project.
///
/// Corresponds to Java: `public class UserDefinedFieldContainer implements Collection<UserDefinedField>`
///
/// # Structural trade-offs
///
/// - The Java constructor takes a `CustomFieldContainer` and stores it as `m_customFields`, but it is only used in
/// `remove`: `m_customFields.remove((UserDefinedField) o)`. On the Rust side,
/// [`crate::custom_field_container::CustomFieldContainer`] uses `FieldKey` as its key, and
/// `FieldKey` has no "user-defined field" variant, so this method call cannot be expressed; therefore the Rust side **does not store**
/// that reference, and the constructor takes no parameters.
/// - Java implements `Collection<UserDefinedField>`; the Rust side provides equivalent `iter` / `size` /
/// `is_empty` / `add` / `remove` / `clear` / `contains` and other methods, and does not implement `std::iter::Extend`
/// and other Rust collection traits (Java's `addAll` / `removeAll` / `retainAll` themselves just throw
///   `UnsupportedOperationException`).
///
/// # Behavior consistent with Java (including parts that appear to be defects in Java)
///
/// This class deliberately replicates Java's behavior, including the following two points:
///
/// - `remove` only removes from `m_uniqueIDMap`: Java's implementation is
/// `m_taskFields.remove(o)`, passing an **object** rather than a key, while the keys of these three maps are `Integer`,
/// so it never matches and has no actual effect.
/// - `clear` clears only the four category maps and does **not** clear `m_uniqueIDMap`, so after `clear()`
/// `size()` may still be non-zero.
///
/// # Differences from Java
///
/// - Map values are `Rc<RefCell<UserDefinedField>>` (shared + later mutable); Java holds object references directly.
/// - The `Function<Integer, UserDefinedField>` parameter of `getOrCreate*` → `impl FnOnce(i32) -> UserDefinedField`.
/// - `toArray()` → [`to_vec`](UserDefinedFieldContainer::to_vec);`iterator()` → [`iter`](UserDefinedFieldContainer::iter).
///
/// Skipped Java methods: `addAll` / `removeAll` / `retainAll` (Java throws
/// `UnsupportedOperationException` directly), `forEach` / `spliterator` (Java collection protocol; on the Rust side these are
/// implemented compositionally with `iter()`), `toArray(T[] a)` (Java's generic array mechanism),
/// and the `m_customFields.remove(...)` call (see above).
///
/// Unported Java methods: none.
pub struct UserDefinedFieldContainer {
/// Task user-defined fields.
    ///
    /// Corresponds to Java: `private final Map<Integer, UserDefinedField> m_taskFields = new HashMap<>();`
    m_task_fields: HashMap<i32, Rc<RefCell<UserDefinedField>>>,
/// Resource user-defined fields.
    ///
    /// Corresponds to Java: `private final Map<Integer, UserDefinedField> m_resourceFields = new HashMap<>();`
    m_resource_fields: HashMap<i32, Rc<RefCell<UserDefinedField>>>,
/// Resource assignment user-defined fields.
    ///
    /// Corresponds to Java: `private final Map<Integer, UserDefinedField> m_assignmentFields = new HashMap<>();`
    m_assignment_fields: HashMap<i32, Rc<RefCell<UserDefinedField>>>,
/// Project user-defined fields.
    ///
    /// Corresponds to Java: `private final Map<Integer, UserDefinedField> m_projectFields = new HashMap<>();`
    m_project_fields: HashMap<i32, Rc<RefCell<UserDefinedField>>>,
/// All user-defined fields (by unique ID).
    ///
    /// Corresponds to Java: `private final Map<Integer, UserDefinedField> m_uniqueIDMap = new HashMap<>();`
    m_unique_id_map: HashMap<i32, Rc<RefCell<UserDefinedField>>>,
}

impl Default for UserDefinedFieldContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl UserDefinedFieldContainer {
/// Constructor.
    ///
    /// Corresponds to Java: `public UserDefinedFieldContainer(CustomFieldContainer customFields)`
    ///
/// See the type description: the Rust side does not store the `customFields` reference.
    pub fn new() -> Self {
        Self {
            m_task_fields: HashMap::new(),
            m_resource_fields: HashMap::new(),
            m_assignment_fields: HashMap::new(),
            m_project_fields: HashMap::new(),
            m_unique_id_map: HashMap::new(),
        }
    }

/// Iterates over all user-defined fields.
    ///
    /// Corresponds to Java: `@Override public Iterator<UserDefinedField> iterator()`
    pub fn iter(&self) -> impl Iterator<Item = &Rc<RefCell<UserDefinedField>>> {
        self.m_unique_id_map.values()
    }

/// Returns all user-defined fields as a `Vec`.
    ///
    /// Corresponds to Java: `@Override public Object[] toArray()`
    pub fn to_vec(&self) -> Vec<Rc<RefCell<UserDefinedField>>> {
        self.m_unique_id_map.values().cloned().collect()
    }

/// Adds a user-defined field.
    ///
    /// Corresponds to Java: `@Override public boolean add(UserDefinedField field)`
    ///
/// Returns `false` when the field's entity type is not TASK / RESOURCE / ASSIGNMENT / PROJECT
/// (Java returns `false` for types such as `Constraint`, because `getMapFromFieldTypeClass` returns `null`).
    pub fn add(&mut self, field: Rc<RefCell<UserDefinedField>>) -> bool {
        let unique_id = field.borrow().get_unique_id();

        // Java: `Map<Integer, UserDefinedField> map = getMapFromFieldTypeClass(field); if (map == null) { return false; }`
        let target = match self.map_from_field_type_class(&field.borrow()) {
            Some(target) => target,
            None => return false,
        };

        target.insert(unique_id, Rc::clone(&field));
        self.add_field(field);
        true
    }

/// Gets the corresponding category map by entity type (Java's `getMapFromFieldTypeClass`).
    fn map_from_field_type_class(
        &mut self,
        field: &UserDefinedField,
    ) -> Option<&mut HashMap<i32, Rc<RefCell<UserDefinedField>>>> {
        match field.get_field_type_class() {
            Some(FieldTypeClass::Task) => Some(&mut self.m_task_fields),
            Some(FieldTypeClass::Resource) => Some(&mut self.m_resource_fields),
            Some(FieldTypeClass::Assignment) => Some(&mut self.m_assignment_fields),
            Some(FieldTypeClass::Project) => Some(&mut self.m_project_fields),
            _ => None,
        }
    }

/// Removes a user-defined field.
    ///
    /// Corresponds to Java: `@Override public boolean remove(Object o)`
    ///
/// Consistent with Java: only removes from `m_uniqueIDMap` (the `remove(o)` on the other three lines in Java passes an object
/// rather than a key, so it never matches); the return value indicates whether that mapping exists in `m_uniqueIDMap`.
/// In addition, Java also calls `m_customFields.remove(...)`; that call cannot be expressed with `FieldKey`,
/// see the type description.
    pub fn remove(&mut self, field: &UserDefinedField) -> bool {
        self.m_unique_id_map
            .remove(&field.get_unique_id())
            .is_some()
    }

/// Determines whether the specified user-defined field is contained.
    ///
    /// Corresponds to Java: `@Override public boolean contains(Object o)`
    ///
/// Java uses `m_uniqueIDMap.containsValue(o)`; `UserDefinedField` equality is determined by unique ID,
/// so it is equivalent to a key-based check.
    pub fn contains(&self, field: &UserDefinedField) -> bool {
        self.m_unique_id_map.contains_key(&field.get_unique_id())
    }

/// Determines whether all the given user-defined fields are contained.
    ///
    /// Corresponds to Java: `@Override public boolean containsAll(Collection<?> c)`
    pub fn contains_all(&self, fields: &[UserDefinedField]) -> bool {
        fields.iter().all(|field| self.contains(field))
    }

/// Clears the four category maps.
    ///
    /// Corresponds to Java: `@Override public void clear()`
    ///
/// Consistent with Java: does **not** clear `m_uniqueIDMap`, see the type description.
    pub fn clear(&mut self) {
        self.m_task_fields.clear();
        self.m_resource_fields.clear();
        self.m_assignment_fields.clear();
        self.m_project_fields.clear();
    }

/// The number of user-defined fields.
    ///
    /// Corresponds to Java: `@Override public int size()`
    pub fn size(&self) -> usize {
        self.m_unique_id_map.len()
    }

/// Whether there are no user-defined fields.
    ///
    /// Corresponds to Java: `@Override public boolean isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.m_unique_id_map.is_empty()
    }

/// Gets all task user-defined fields.
    ///
    /// Corresponds to Java: `public Collection<UserDefinedField> getTaskFields()`
    pub fn get_task_fields(&self) -> Vec<Rc<RefCell<UserDefinedField>>> {
        self.m_task_fields.values().cloned().collect()
    }

/// Gets all resource user-defined fields.
    ///
    /// Corresponds to Java: `public Collection<UserDefinedField> getResourceFields()`
    pub fn get_resource_fields(&self) -> Vec<Rc<RefCell<UserDefinedField>>> {
        self.m_resource_fields.values().cloned().collect()
    }

/// Gets all resource assignment user-defined fields.
    ///
    /// Corresponds to Java: `public Collection<UserDefinedField> getAssignmentFields()`
    pub fn get_assignment_fields(&self) -> Vec<Rc<RefCell<UserDefinedField>>> {
        self.m_assignment_fields.values().cloned().collect()
    }

/// Gets all project user-defined fields.
    ///
    /// Corresponds to Java: `public Collection<UserDefinedField> getProjectFields()`
    pub fn get_project_fields(&self) -> Vec<Rc<RefCell<UserDefinedField>>> {
        self.m_project_fields.values().cloned().collect()
    }

/// Gets a task user-defined field by ID, creating it with `create` when absent.
    ///
    /// Corresponds to Java: `public UserDefinedField getOrCreateTaskField(Integer id, Function<Integer, UserDefinedField> createFunction)`
    pub fn get_or_create_task_field<F>(&mut self, id: i32, create: F) -> Rc<RefCell<UserDefinedField>>
    where
        F: FnOnce(i32) -> UserDefinedField,
    {
        if let Some(existing) = self.m_task_fields.get(&id) {
            return Rc::clone(existing);
        }

        let field = Rc::new(RefCell::new(create(id)));
        self.m_task_fields.insert(id, Rc::clone(&field));
        self.add_field(field)
    }

/// Gets a resource user-defined field by ID, creating it with `create` when absent.
    ///
    /// Corresponds to Java: `public UserDefinedField getOrCreateResourceField(Integer id, Function<Integer, UserDefinedField> createFunction)`
    pub fn get_or_create_resource_field<F>(
        &mut self,
        id: i32,
        create: F,
    ) -> Rc<RefCell<UserDefinedField>>
    where
        F: FnOnce(i32) -> UserDefinedField,
    {
        if let Some(existing) = self.m_resource_fields.get(&id) {
            return Rc::clone(existing);
        }

        let field = Rc::new(RefCell::new(create(id)));
        self.m_resource_fields.insert(id, Rc::clone(&field));
        self.add_field(field)
    }

/// Gets a resource assignment user-defined field by ID, creating it with `create` when absent.
    ///
    /// Corresponds to Java: `public UserDefinedField getOrCreateAssignmentField(Integer id, Function<Integer, UserDefinedField> createFunction)`
    pub fn get_or_create_assignment_field<F>(
        &mut self,
        id: i32,
        create: F,
    ) -> Rc<RefCell<UserDefinedField>>
    where
        F: FnOnce(i32) -> UserDefinedField,
    {
        if let Some(existing) = self.m_assignment_fields.get(&id) {
            return Rc::clone(existing);
        }

        let field = Rc::new(RefCell::new(create(id)));
        self.m_assignment_fields.insert(id, Rc::clone(&field));
        self.add_field(field)
    }

/// Gets a project user-defined field by ID, creating it with `create` when absent.
    ///
    /// Corresponds to Java: `public UserDefinedField getOrCreateProjectField(Integer id, Function<Integer, UserDefinedField> createFunction)`
    pub fn get_or_create_project_field<F>(
        &mut self,
        id: i32,
        create: F,
    ) -> Rc<RefCell<UserDefinedField>>
    where
        F: FnOnce(i32) -> UserDefinedField,
    {
        if let Some(existing) = self.m_project_fields.get(&id) {
            return Rc::clone(existing);
        }

        let field = Rc::new(RefCell::new(create(id)));
        self.m_project_fields.insert(id, Rc::clone(&field));
        self.add_field(field)
    }

/// Gets a user-defined field by unique ID.
    ///
    /// Corresponds to Java: `public UserDefinedField getByUniqueID(Integer id)`
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<UserDefinedField>>> {
        let id = id?;
        self.m_unique_id_map.get(&id).cloned()
    }

/// Registers a field in the unique ID index and returns it (Java's `addField`).
    fn add_field(&mut self, field: Rc<RefCell<UserDefinedField>>) -> Rc<RefCell<UserDefinedField>> {
        let unique_id = field.borrow().get_unique_id();
        self.m_unique_id_map.insert(unique_id, Rc::clone(&field));
        field
    }
}

impl Debug for UserDefinedFieldContainer {
/// Java does not override `toString()`; the Rust side outputs the unique ID index.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserDefinedFieldContainer")
            .field("m_unique_id_map", &self.m_unique_id_map)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::object_sequence::ObjectSequence;
    use crate::data_type::DataType;
    use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;
    use std::any::TypeId;

/// A sequence generator provider for tests.
    #[derive(Default)]
    struct TestProvider {
        sequences: HashMap<TypeId, Rc<ObjectSequence>>,
    }

    impl UniqueIdObjectSequenceProvider for TestProvider {
        fn get_unique_id_object_sequence(&mut self, type_id: TypeId) -> Rc<ObjectSequence> {
            Rc::clone(
                self.sequences
                    .entry(type_id)
                    .or_insert_with(|| Rc::new(ObjectSequence::new(1))),
            )
        }
    }

/// Builds a user-defined field.
    fn field(unique_id: i32, type_class: FieldTypeClass) -> UserDefinedField {
        let provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            Rc::new(RefCell::new(TestProvider::default()));

        crate::user_defined_field::Builder::new(provider)
            .unique_id(Some(unique_id))
            .field_type_class(type_class)
            .external_name(Some(format!("field{unique_id}")))
            .data_type(DataType::String)
            .build()
    }

    #[test]
    fn add_indexes_fields_by_type_and_unique_id() {
        let mut container = UserDefinedFieldContainer::new();
        assert!(container.is_empty());

        let task = Rc::new(RefCell::new(field(1, FieldTypeClass::Task)));
        let resource = Rc::new(RefCell::new(field(2, FieldTypeClass::Resource)));
        let assignment = Rc::new(RefCell::new(field(3, FieldTypeClass::Assignment)));
        let project = Rc::new(RefCell::new(field(4, FieldTypeClass::Project)));

        assert!(container.add(Rc::clone(&task)));
        assert!(container.add(Rc::clone(&resource)));
        assert!(container.add(Rc::clone(&assignment)));
        assert!(container.add(Rc::clone(&project)));

        assert_eq!(container.size(), 4);
        assert_eq!(container.iter().count(), 4);
        assert_eq!(container.to_vec().len(), 4);
        assert_eq!(container.get_task_fields().len(), 1);
        assert_eq!(container.get_resource_fields().len(), 1);
        assert_eq!(container.get_assignment_fields().len(), 1);
        assert_eq!(container.get_project_fields().len(), 1);

        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(1)).expect("registered"),
            &task
        ));
        assert!(container.get_by_unique_id(Some(9)).is_none());
        assert!(container.get_by_unique_id(None).is_none());
    }

    #[test]
    fn unsupported_field_type_class_is_rejected() {
        let mut container = UserDefinedFieldContainer::new();
        let unsupported = Rc::new(RefCell::new(field(1, FieldTypeClass::Constraint)));

        assert!(!container.add(unsupported));
        assert!(container.is_empty());
    }

    #[test]
    fn remove_only_affects_the_unique_id_index() {
        let mut container = UserDefinedFieldContainer::new();
        let task = Rc::new(RefCell::new(field(1, FieldTypeClass::Task)));
        container.add(Rc::clone(&task));

        assert!(container.remove(&task.borrow()));
        assert!(container.is_empty());
        assert!(container.get_by_unique_id(Some(1)).is_none());

// Consistent with Java: the field remains in the category map
        assert_eq!(container.get_task_fields().len(), 1);

// Removing again returns false
        assert!(!container.remove(&task.borrow()));
    }

    #[test]
    fn contains_and_contains_all_use_unique_id() {
        let mut container = UserDefinedFieldContainer::new();
        let task = field(1, FieldTypeClass::Task);
        container.add(Rc::new(RefCell::new(task.clone())));

        assert!(container.contains(&task));
        assert!(!container.contains(&field(2, FieldTypeClass::Task)));

        assert!(container.contains_all(&[task.clone()]));
        assert!(!container.contains_all(&[task.clone(), field(2, FieldTypeClass::Task)]));
        assert!(container.contains_all(&[]));
    }

    #[test]
    fn clear_only_empties_the_per_type_maps() {
        let mut container = UserDefinedFieldContainer::new();
        container.add(Rc::new(RefCell::new(field(1, FieldTypeClass::Task))));

        container.clear();

// Consistent with Java: size() is still non-zero after clear
        assert_eq!(container.size(), 1);
        assert_eq!(container.get_task_fields().len(), 0);
        assert!(container.get_by_unique_id(Some(1)).is_some());
    }

    #[test]
    fn get_or_create_reuses_and_creates_fields() {
        let mut container = UserDefinedFieldContainer::new();

        let created = container.get_or_create_task_field(5, |id| {
            field(id, FieldTypeClass::Task)
        });
        assert_eq!(created.borrow().get_unique_id(), 5);
        assert_eq!(container.size(), 1);

// The create function is not called again when it already exists
        let create_calls = std::cell::Cell::new(0);
        let existing = container.get_or_create_task_field(5, |id| {
            create_calls.set(create_calls.get() + 1);
            field(id, FieldTypeClass::Task)
        });
        assert_eq!(create_calls.get(), 0);
        assert!(Rc::ptr_eq(&created, &existing));
        assert_eq!(container.size(), 1);

// The other categories are independent of each other
        container.get_or_create_resource_field(6, |id| field(id, FieldTypeClass::Resource));
        container.get_or_create_assignment_field(7, |id| field(id, FieldTypeClass::Assignment));
        container.get_or_create_project_field(8, |id| field(id, FieldTypeClass::Project));

        assert_eq!(container.size(), 4);
        assert_eq!(container.get_resource_fields().len(), 1);
        assert_eq!(container.get_assignment_fields().len(), 1);
        assert_eq!(container.get_project_fields().len(), 1);
    }
}
