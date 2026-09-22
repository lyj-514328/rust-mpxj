//! Corresponds to Java: `org/mpxj/ProjectEntityWithIDContainer.java`

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use crate::common::object_sequence::ObjectSequence;
use crate::project_entity_container::ProjectEntityContainer;
use crate::project_entity_with_id::ProjectEntityWithID;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;

/// A container that adds an `ID → entity` index on top of [`ProjectEntityContainer`].
///
/// Corresponds to Java:
/// `public abstract class ProjectEntityWithIDContainer<T extends ProjectEntityWithID & Comparable<T>> extends ProjectEntityContainer<T>`
///
/// # How the inheritance hierarchy is emulated: composition + forwarding
///
/// Rust has no inheritance, so this class cannot "be" a [`ProjectEntityContainer`]. Here **composition + forwarding** is used:
/// it internally holds a [`ProjectEntityContainer<T>`] and forwards the base class's public behavior method by method (see each method's description),
/// then layers on the ID index and renumbering logic added by this class.
///
/// | Java | Rust |
/// |---|---|
/// | `extends ProjectEntityContainer<T>` | Field `container: ProjectEntityContainer<T>` |
/// | `private final ObjectSequence m_idSequence = new ObjectSequence(1)` | Field `id_sequence: ObjectSequence` |
/// | `protected final Map<Integer, T> m_idMap` | Field `id_map: HashMap<i32, Rc<RefCell<T>>>` |
///
/// Composition was chosen over "providing the base class behavior through a trait" because in Java the base class's
/// `added` / `removed` hooks and the unique ID index are **state**, and trait default methods cannot hold this state
/// (traits cannot have fields); composition can fully preserve both behavior and state.
///
/// # Differences from Java
///
/// - **Element reference model**: consistent with the base class, elements are `Rc<RefCell<T>>`.
/// - **`Comparable<T>`**: Java uses `Collections.sort(this)` to sort by the elements' own natural order; the Rust side
/// constrains it to `T: Ord`, corresponding to the elements' `compareTo` implementation.
/// - **`null` keys**: `m_idMap` allows `null` keys in Java; the Rust key is `i32` and no `null` key is provided
/// (equivalent to "not found").
/// - **`NumberHelper.getInt(null)`**: Java takes 0 for a `null` ID; the Rust side expresses this equivalently with `unwrap_or(0)`.
///
/// Skipped Java mechanisms: none.
///
/// Unported Java methods: none.
pub struct ProjectEntityWithIDContainer<T> {
/// The base class container (Java's `extends ProjectEntityContainer<T>`).
    container: ProjectEntityContainer<T>,
/// ID sequence generator, starting from 1.
    ///
    /// Corresponds to Java: `private final ObjectSequence m_idSequence = new ObjectSequence(1);`
    id_sequence: ObjectSequence,
/// The ID → entity index, maintained by the caller through
/// [`map_id`](ProjectEntityWithIDContainer::map_id).
    ///
    /// Corresponds to Java: `protected final Map<Integer, T> m_idMap = new HashMap<>();`
    id_map: HashMap<i32, Rc<RefCell<T>>>,
}

impl<T: ProjectEntityWithID + Ord + 'static> ProjectEntityWithIDContainer<T> {
/// Constructor.
    ///
    /// Corresponds to Java: `public ProjectEntityWithIDContainer(UniqueIdObjectSequenceProvider projectFile)`
    pub fn new(sequence_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>) -> Self {
        Self {
            container: ProjectEntityContainer::new(sequence_provider),
            id_sequence: ObjectSequence::new(1),
            id_map: HashMap::new(),
        }
    }

/// Make the IDs of all entities contiguous.
    ///
    /// Corresponds to Java: `public void renumberIDs()`
    ///
/// Sort by natural order first, then assign IDs in turn: if the first entity after sorting has a non-zero ID, start from 1,
/// otherwise start from 0 (consistent with Java).
    pub fn renumber_ids(&mut self) {
        if self.container.is_empty() {
            return;
        }

        // Java: `Collections.sort(this)`
        self.container.sort();

        // Java: `int id = NumberHelper.getInt(firstEntity.getID()); if (id != 0) { id = 1; }`
        let first_id = self
            .container
            .get(0)
            .and_then(|entity| entity.borrow().get_id())
            .unwrap_or(0);
        let mut id = if first_id != 0 { 1 } else { 0 };

        for entity in self.container.iter() {
            entity.borrow_mut().set_id(Some(id));
            id += 1;
        }
    }

/// Find an entity by ID; return `None` when not found.
    ///
    /// Corresponds to Java: `public T getByID(Integer id)`
    pub fn get_by_id(&self, id: Option<i32>) -> Option<Rc<RefCell<T>>> {
        let id = id?;
        self.id_map.get(&id).cloned()
    }

/// Remove an ID mapping.
    ///
    /// Corresponds to Java: `public void unmapID(Integer id)`
    pub fn unmap_id(&mut self, id: Option<i32>) {
        if let Some(id) = id {
            self.id_map.remove(&id);
        }
    }

/// Add an ID mapping.
    ///
    /// Corresponds to Java: `public void mapID(Integer id, T entity)`
    pub fn map_id(&mut self, id: i32, entity: Rc<RefCell<T>>) {
        self.id_map.insert(id, entity);
    }

/// Get the next available ID.
    ///
    /// Corresponds to Java: `public Integer getNextID()`
    pub fn get_next_id(&self) -> i32 {
        self.id_sequence.get_next()
    }

/// Access the inner base class container.
    ///
/// Corresponds to Java's `extends ProjectEntityContainer<T>` relationship; the Rust side expresses it with composition,
/// so the base class instance must be exposed explicitly.
    pub fn container(&self) -> &ProjectEntityContainer<T> {
        &self.container
    }

/// Get the sequence generator provider.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.getSequenceProvider()`
    pub fn get_sequence_provider(&self) -> Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> {
        self.container.get_sequence_provider()
    }

/// The first value used when renumbering unique IDs.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.firstUniqueID()`
    pub fn first_unique_id(&self) -> i32 {
        self.container.first_unique_id()
    }

/// Find an entity by unique ID.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<T>>> {
        self.container.get_by_unique_id(id)
    }

/// Append an entity.
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(T element)`
    pub fn add(&mut self, element: Rc<RefCell<T>>) {
        self.container.add(element);
    }

/// Insert an entity at the given index (returns `false` when out of bounds).
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(int index, T element)`
    pub fn add_at(&mut self, index: usize, element: Rc<RefCell<T>>) -> bool {
        self.container.add_at(index, element)
    }

/// Replace the entity at the given index (returns `None` when out of bounds).
    ///
    /// Corresponds to Java: `ListWithCallbacks.set(int index, T element)`
    pub fn set(&mut self, index: usize, element: Rc<RefCell<T>>) -> Option<Rc<RefCell<T>>> {
        self.container.set(index, element)
    }

/// Remove the entity at the given index (returns `None` when out of bounds).
    ///
    /// Corresponds to Java: `ListWithCallbacks.remove(int index)`
    pub fn remove(&mut self, index: usize) -> Option<Rc<RefCell<T>>> {
        self.container.remove(index)
    }

/// Clear the entity list (does not fire the `removed` callback, consistent with Java).
    ///
    /// Corresponds to Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.container.clear();
    }

/// Get the entity by index.
    ///
    /// Corresponds to Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<Rc<RefCell<T>>> {
        self.container.get(index)
    }

/// Number of entities.
    ///
    /// Corresponds to Java: `ListWithCallbacks.size()`
    pub fn len(&self) -> usize {
        self.container.len()
    }

/// Whether the entity list is empty.
    ///
    /// Corresponds to Java: `ListWithCallbacks.isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.container.is_empty()
    }

/// Iterate over the entity list.
    ///
    /// Corresponds to Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<T>>> {
        self.container.iter()
    }

/// Access the entity list as a slice.
    ///
/// Corresponds to Java: the list view of `ListWithCallbacks`.
    pub fn as_slice(&self) -> &[Rc<RefCell<T>>] {
        self.container.as_slice()
    }

/// Sort by the elements' natural order.
    ///
    /// Corresponds to Java: `Collections.sort(this)`
    pub fn sort(&mut self) {
        self.container.sort();
    }

/// Update the unique ID index.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.updateUniqueID(T element, Integer oldUniqueID, Integer newUniqueID)`
    pub fn update_unique_id(
        &mut self,
        element: &Rc<RefCell<T>>,
        old_unique_id: Option<i32>,
        new_unique_id: Option<i32>,
    ) {
        self.container
            .update_unique_id(element, old_unique_id, new_unique_id);
    }

/// Reassign unique IDs for all entities with conflicting unique IDs.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.fixUniqueIdClashes()`
    pub fn fix_unique_id_clashes(&mut self) {
        self.container.fix_unique_id_clashes();
    }
}

impl<T: Debug> Debug for ProjectEntityWithIDContainer<T> {
/// `toString()` is not overridden on the Java side; the Rust side outputs the inner container and the ID index.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProjectEntityWithIDContainer")
            .field("container", &self.container)
            .field("id_map", &self.id_map)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
    use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
    use std::any::TypeId;

/// Test entity: has both an ID and a unique ID.
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct Entity {
        id: Option<i32>,
        unique_id: Option<i32>,
    }

    impl ProjectEntityWithUniqueID for Entity {
        fn get_unique_id(&self) -> Option<i32> {
            self.unique_id
        }
    }

    impl ProjectEntityWithMutableUniqueID for Entity {
        fn set_unique_id(&mut self, id: Option<i32>) {
            self.unique_id = id;
        }
    }

    impl ProjectEntityWithID for Entity {
        fn get_id(&self) -> Option<i32> {
            self.id
        }

        fn set_id(&mut self, id: Option<i32>) {
            self.id = id;
        }
    }

/// Test sequence generator provider.
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

/// Construct a container.
    fn container() -> ProjectEntityWithIDContainer<Entity> {
        let provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            Rc::new(RefCell::new(TestProvider::default()));
        ProjectEntityWithIDContainer::new(provider)
    }

/// Construct an entity.
    fn entity(id: i32, unique_id: i32) -> Rc<RefCell<Entity>> {
        Rc::new(RefCell::new(Entity {
            id: Some(id),
            unique_id: Some(unique_id),
        }))
    }

    #[test]
    fn base_container_behaviour_is_forwarded() {
        let mut container = container();
        assert!(container.is_empty());

        container.add(entity(1, 10));
        assert_eq!(container.len(), 1);
        assert_eq!(container.iter().count(), 1);
        assert_eq!(container.as_slice().len(), 1);
        assert!(container.get(0).is_some());
        assert!(container.get(1).is_none());
        assert!(container.get_by_unique_id(Some(10)).is_some());
        assert!(container.get_by_id(Some(1)).is_none());
        assert_eq!(container.first_unique_id(), 1);
    }

    #[test]
    fn id_map_can_be_populated_and_removed() {
        let mut container = container();
        let first = entity(1, 10);
        container.add(Rc::clone(&first));

        container.map_id(1, Rc::clone(&first));
        assert!(Rc::ptr_eq(
            &container.get_by_id(Some(1)).expect("mapped"),
            &first
        ));

        container.unmap_id(Some(1));
        assert!(container.get_by_id(Some(1)).is_none());
        assert!(container.get_by_id(None).is_none());
    }

    #[test]
    fn get_next_id_starts_at_one() {
        let container = container();

        assert_eq!(container.get_next_id(), 1);
        assert_eq!(container.get_next_id(), 2);
        assert_eq!(container.get_next_id(), 3);
    }

    #[test]
    fn renumber_ids_fills_gaps_from_one() {
        let mut container = container();
        container.add(entity(5, 50));
        container.add(entity(1, 10));
        container.add(entity(3, 30));

        container.renumber_ids();

// After sorting the smallest ID is 1 (non-zero), so contiguous numbering starts from 1
        let ids: Vec<Option<i32>> = container
            .iter()
            .map(|entity| entity.borrow().get_id())
            .collect();
        assert_eq!(ids, vec![Some(1), Some(2), Some(3)]);
    }

    #[test]
    fn renumber_ids_starts_at_zero_when_first_id_is_zero() {
        let mut container = container();
        container.add(entity(0, 0));
        container.add(entity(4, 40));

        container.renumber_ids();

        let ids: Vec<Option<i32>> = container
            .iter()
            .map(|entity| entity.borrow().get_id())
            .collect();
        assert_eq!(ids, vec![Some(0), Some(1)]);
    }

    #[test]
    fn renumber_ids_on_empty_container_is_a_no_op() {
        let mut container = container();
        container.renumber_ids();
        assert!(container.is_empty());
    }

    #[test]
    fn unique_id_clash_handling_is_forwarded() {
        let mut container = container();
        let first = entity(1, 10);
        let second = entity(2, 10);
        container.add(Rc::clone(&first));
        container.add(Rc::clone(&second));

        container.fix_unique_id_clashes();

// Conflicting entries take the next value in the sequence
        assert_eq!(second.borrow().get_unique_id(), Some(1));
        assert_eq!(first.borrow().get_unique_id(), Some(10));
    }
}
