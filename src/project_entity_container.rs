//! Corresponds to Java: `org/mpxj/ProjectEntityContainer.java`
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `removed(T)` removes the index entry via the current unique ID | Removes the index entry by `Rc` identity | A calendar may still be mutably borrowed when `remove()` is called, so its unique ID cannot be read back; when the index was overwritten by another element with the same ID, that element is retained |
//! | `fixUniqueIdClashes()` obtains the sequence from the provider | A separate overload takes the real sequence | While the container holds a mutable borrow of `ProjectContext`, it cannot borrow the context again through the adapter |
//!
//! # Skipped Java members + reasons
//!
//! None.

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use crate::common::object_sequence::ObjectSequence;
use crate::list_with_callbacks::ListWithCallbacks;
use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;

/// Common implementation of a project entity container: storage, iteration, and lookup by unique ID.
///
/// Corresponds to Java:
/// `public abstract class ProjectEntityContainer<T extends ProjectEntityWithUniqueID> extends ListWithCallbacks<T>`
///
/// # Reproducing the Inheritance Hierarchy
///
/// Java implements this class with "inherit `ListWithCallbacks` + override `added` / `removed`".
/// Rust has no inheritance, so this class uses **composition + callback fields** instead:
///
/// | Java | Rust |
/// |---|---|
/// | `extends ListWithCallbacks<T>` | Internally holds a [`ListWithCallbacks<Rc<RefCell<T>>>`] field |
/// | `protected void added(T element)` override | Closure registered via [`ListWithCallbacks::set_added_callback`] |
/// | `protected void removed(T element)` override | Closure registered via [`ListWithCallbacks::set_removed_callback`] |
/// | `private final Map<Integer, T> m_uniqueIDMap` | `Rc<RefCell<HashMap<i32, Rc<RefCell<T>>>>>`, shared with the callback closures |
/// | `private final List<ProjectEntityWithMutableUniqueID> m_uniqueIDClashList` | `Rc<RefCell<Vec<Rc<RefCell<T>>>>>` |
///
/// Subsequent subclasses such as `ProjectEntityWithIDContainer` **cannot** inherit this class in
/// Rust, so they compose an instance of it and forward methods (see
/// [`crate::project_entity_with_id_container::ProjectEntityWithIDContainer`]).
///
/// # Differences from Java
///
/// - **Element reference model**: Java's list and map share the same object references, and
///   methods such as `fixUniqueIdClashes` modify elements in place. The Rust side expresses this
///   sharing and interior mutability with `Rc<RefCell<T>>` (following the convention "shared
///   entities use `Rc<RefCell<T>>`"), so `get` / `iter` return `Rc` handles.
/// - **Element type constraint**: Java declares `T extends ProjectEntityWithUniqueID`, but
///   `fixUniqueIdClashes` internally uses `instanceof` to determine whether an element is
///   mutable. The Rust struct itself carries no constraint;
///   [`fix_unique_id_clashes`](ProjectEntityContainer::fix_unique_id_clashes) expresses this
///   check equivalently with a **method-level** `where T: ProjectEntityWithMutableUniqueID`:
///   for entities with an immutable unique ID (such as `WorkContour`), the clash list is always
///   empty and the method is never called.
/// - **`null` key**: Java's `Map<Integer, T>` allows a `null` key (`added` / `removed` use it
///   for elements without a unique ID). The Rust map key is `i32`, so such elements are skipped
///   (equivalent to "not found").
/// - **`element.getClass()`**: Java uses the element's runtime type to obtain the sequence
///   generator; the Rust side uses `TypeId::of::<T>()` (the container element type is the
///   element type).
/// - **`firstUniqueID()`**: In Java it is `protected` and can be overridden by `TaskContainer`;
///   the Rust side provides a public method
///   [`first_unique_id`](ProjectEntityContainer::first_unique_id) returning the base-class
///   default of 1.
///
/// Skipped Java mechanisms: none (`Serializable` etc. are not used in this class).
///
/// Unported Java methods: none.
pub struct ProjectEntityContainer<T> {
    /// List contents and add/remove callbacks.
    ///
    /// Corresponds to Java: the list storage and `added` / `removed` hooks inherited from the `ListWithCallbacks<T>` base class.
    list: ListWithCallbacks<Rc<RefCell<T>>>,
    /// Sequence generator provider.
    ///
    /// Corresponds to Java: `protected final UniqueIdObjectSequenceProvider m_sequenceProvider;`
    sequence_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>,
    /// unique ID → entity index.
    ///
    /// Corresponds to Java: `private final Map<Integer, T> m_uniqueIDMap = new HashMap<>();`
    unique_id_map: Rc<RefCell<HashMap<i32, Rc<RefCell<T>>>>>,
    /// List of entities with unique ID clashes.
    ///
    /// Corresponds to Java: `private final List<ProjectEntityWithMutableUniqueID> m_uniqueIDClashList = new ArrayList<>();`
    unique_id_clash_list: Rc<RefCell<Vec<Rc<RefCell<T>>>>>,
}

impl<T: ProjectEntityWithUniqueID + 'static> ProjectEntityContainer<T> {
    /// Constructor.
    ///
    /// Corresponds to Java: `public ProjectEntityContainer(UniqueIdObjectSequenceProvider sequenceProvider)`
    ///
    /// The constructor registers the `added` / `removed` callbacks into the internal list,
    /// matching the effect of a subclass overriding these two hooks in Java.
    pub fn new(sequence_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>) -> Self {
        let unique_id_map: Rc<RefCell<HashMap<i32, Rc<RefCell<T>>>>> =
            Rc::new(RefCell::new(HashMap::new()));
        let unique_id_clash_list: Rc<RefCell<Vec<Rc<RefCell<T>>>>> =
            Rc::new(RefCell::new(Vec::new()));
        let mut list: ListWithCallbacks<Rc<RefCell<T>>> = ListWithCallbacks::new();

        // Java: `@Override protected void added(T element)`
        {
            let map = Rc::clone(&unique_id_map);
            let clash_list = Rc::clone(&unique_id_clash_list);
            list.set_added_callback(Some(Box::new(move |element: &Rc<RefCell<T>>| {
                let unique_id = match element.try_borrow().ok().and_then(|element| element.get_unique_id()) {
                    Some(unique_id) => unique_id,
                    None => return,
                };

                let current_element = map.try_borrow().ok()
                    .and_then(|map| map.get(&unique_id).cloned());
                if let Some(current_element) = current_element {
                    // Java: re-adding the same instance is not treated as a clash
                    if Rc::ptr_eq(&current_element, element) {
                        return;
                    }

                    // Java: `if (currentElement instanceof ProjectEntityWithMutableUniqueID)`
                    // Record the clash candidate; for element types with an immutable unique ID, the clash list is
                    // never consumed (see the method-level constraint of `fix_unique_id_clashes`).
                    if let Ok(mut clashes) = clash_list.try_borrow_mut() {
                        clashes.push(Rc::clone(element));
                    }
                }

                if let Ok(mut map) = map.try_borrow_mut() {
                    map.insert(unique_id, Rc::clone(element));
                }
            })));
        }

        // Java: `@Override protected void removed(T element)`
        {
            let map = Rc::clone(&unique_id_map);
            list.set_removed_callback(Some(Box::new(move |element: &Rc<RefCell<T>>| {
                // A calendar may trigger this callback while holding a mutable borrow in its `remove()`.
                // Remove by element identity directly, with no need to re-enter and borrow the entity.
                if let Ok(mut map) = map.try_borrow_mut() {
                    map.retain(|_, candidate| !Rc::ptr_eq(candidate, element));
                }
            })));
        }

        Self {
            list,
            sequence_provider,
            unique_id_map,
            unique_id_clash_list,
        }
    }

    /// Return the sequence generator provider.
    ///
    /// Corresponds to Java: `public UniqueIdObjectSequenceProvider getSequenceProvider()`
    pub fn get_sequence_provider(&self) -> Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> {
        Rc::clone(&self.sequence_provider)
    }

    /// The first value used when renumbering unique IDs.
    ///
    /// Corresponds to Java: `protected int firstUniqueID()`
    ///
    /// On the Java side it is overridden by `TaskContainer` (starting from 0 when task 0 exists);
    /// on the Rust side each container decides for itself, and this method provides the base-class
    /// default.
    pub fn first_unique_id(&self) -> i32 {
        1
    }

    /// Find an entity by unique ID; returns `None` when not found.
    ///
    /// Corresponds to Java: `public T getByUniqueID(Integer id)`
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<T>>> {
        let id = id?;
        self.unique_id_map.borrow().get(&id).cloned()
    }

    /// Append an entity.
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(T element)` (updates the unique ID index via the `added` hook).
    pub fn add(&mut self, element: Rc<RefCell<T>>) {
        self.list.add(element);
    }

    /// Insert an entity at the given index.
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(int index, T element)`
    ///
    /// Returns `false` when out of bounds.
    pub fn add_at(&mut self, index: usize, element: Rc<RefCell<T>>) -> bool {
        self.list.add_at(index, element)
    }

    /// Replace the entity at the given index and return the old entity that was replaced.
    ///
    /// Corresponds to Java: `ListWithCallbacks.set(int index, T element)`
    ///
    /// Returns `None` when out of bounds.
    ///
    /// Consistent with Java: replacement does **not** update the unique ID index (Java's
    /// `ProjectEntityContainer` overrides only the two hooks `added` / `removed`, not `replaced`);
    /// when the index needs to be synchronized, the caller must explicitly use
    /// [`update_unique_id`](ProjectEntityContainer::update_unique_id).
    pub fn set(&mut self, index: usize, element: Rc<RefCell<T>>) -> Option<Rc<RefCell<T>>> {
        self.list.set(index, element)
    }

    /// Remove the entity at the given index and return it.
    ///
    /// Corresponds to Java: `ListWithCallbacks.remove(int index)`
    ///
    /// Returns `None` when out of bounds.
    pub fn remove(&mut self, index: usize) -> Option<Rc<RefCell<T>>> {
        self.list.remove(index)
    }

    /// Clear the container (does not trigger the `removed` callback, consistent with Java).
    ///
    /// Corresponds to Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.list.clear();
    }

    /// Return the entity at the given index.
    ///
    /// Corresponds to Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<Rc<RefCell<T>>> {
        self.list.get(index).cloned()
    }

    /// Number of entities.
    ///
    /// Corresponds to Java: `ListWithCallbacks.size()`
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Whether the container is empty.
    ///
    /// Corresponds to Java: `ListWithCallbacks.isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Iterate over the entities in the container.
    ///
    /// Corresponds to Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<T>>> {
        self.list.iter()
    }

    /// Access the entities in the container as a slice.
    ///
    /// Corresponds to Java: `ListWithCallbacks`'s list view.
    pub fn as_slice(&self) -> &[Rc<RefCell<T>>] {
        self.list.as_slice()
    }

    /// Sort in place by the entities' own ordering (does not trigger callbacks; see the docs of [`ListWithCallbacks`]).
    ///
    /// Corresponds to Java: `Collections.sort(this)` in `ProjectEntityWithIDContainer.renumberIDs()`
    pub fn sort(&mut self)
    where
        T: Ord,
    {
        self.list.sort();
    }

    /// Update the unique ID index: called when an element switches to a new unique ID.
    ///
    /// Corresponds to Java: `public void updateUniqueID(T element, Integer oldUniqueID, Integer newUniqueID)`
    ///
    /// On the Java side a `null` `newUniqueID` writes a `null` key; the Rust `i32` key cannot
    /// express it, so only the old key is removed and no new key is created (equivalent to "not
    /// found").
    pub fn update_unique_id(
        &mut self,
        element: &Rc<RefCell<T>>,
        old_unique_id: Option<i32>,
        new_unique_id: Option<i32>,
    ) {
        if let Some(old_unique_id) = old_unique_id {
            self.unique_id_map.borrow_mut().remove(&old_unique_id);
        }

        let new_unique_id = match new_unique_id {
            Some(new_unique_id) => new_unique_id,
            None => return,
        };

        let current_element = self.unique_id_map.borrow().get(&new_unique_id).cloned();
        if let Some(current_element) = current_element {
            if Rc::ptr_eq(&current_element, element) {
                return;
            }

            self.unique_id_clash_list
                .borrow_mut()
                .push(Rc::clone(element));
        }

        self.unique_id_map
            .borrow_mut()
            .insert(new_unique_id, Rc::clone(element));

        self.sequence_provider
            .borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<T>())
            .sync(Some(new_unique_id));
    }

    /// Reassign unique IDs for all entities with unique ID clashes.
    ///
    /// Corresponds to Java: `public void fixUniqueIdClashes()`
    ///
    /// Only entities with a mutable unique ID can produce clashes, so this method carries the
    /// `where T: ProjectEntityWithMutableUniqueID` constraint — equivalent to the runtime check
    /// done in Java via `instanceof ProjectEntityWithMutableUniqueID`.
    pub fn fix_unique_id_clashes(&mut self)
    where
        T: ProjectEntityWithMutableUniqueID,
    {
        let sequence = self
            .sequence_provider
            .borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<T>());
        self.fix_unique_id_clashes_with_sequence(sequence);
    }

    /// Fix clashes using the sequence already obtained from the parent context.
    ///
    /// Corresponds to Java: `fixUniqueIdClashes()`; the Rust calendar container cannot have the
    /// sequence provider borrow the same context again while it holds a mutable borrow of the
    /// context, so the caller passes in the real sequence.
    pub fn fix_unique_id_clashes_with_sequence(&mut self, sequence: Rc<ObjectSequence>)
    where
        T: ProjectEntityWithMutableUniqueID,
    {
        let clash_list: Vec<Rc<RefCell<T>>> = self.unique_id_clash_list.borrow().clone();
        if clash_list.is_empty() {
            return;
        }

        for element in &clash_list {
            let next = sequence.get_next();
            element.borrow_mut().set_unique_id(Some(next));
        }

        self.unique_id_clash_list.borrow_mut().clear();

        let mut map = self.unique_id_map.borrow_mut();
        map.clear();
        for element in self.list.iter() {
            if let Some(unique_id) = element.borrow().get_unique_id() {
                map.insert(unique_id, Rc::clone(element));
            }
        }
    }

    /// Rebuild the unique ID index and clash list from the current elements.
    ///
    /// Corresponds to Java: the per-element indexing behavior of `added(T)`; when a Rust calendar
    /// setter hits a re-entrant borrow of the context it cannot call back into the container, so
    /// that indexing change must be filled in before fixing clashes.
    pub fn rebuild_unique_id_index(&mut self) {
        let (Ok(mut map), Ok(mut clashes)) = (
            self.unique_id_map.try_borrow_mut(),
            self.unique_id_clash_list.try_borrow_mut(),
        ) else {
            return;
        };
        map.clear();
        clashes.clear();
        for item in self.list.iter() {
            let Ok(item_ref) = item.try_borrow() else {
                continue;
            };
            let Some(id) = item_ref.get_unique_id() else {
                continue;
            };
            if map.contains_key(&id) {
                clashes.push(Rc::clone(item));
            }
            map.insert(id, Rc::clone(item));
        }
    }
}

impl<T: Debug> Debug for ProjectEntityContainer<T> {
    /// Java does not override `toString()`; the Rust side outputs the list and unique ID index,
    /// ignoring the sequence generator provider that is not `Debug`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProjectEntityContainer")
            .field("list", &self.list)
            .field("unique_id_map", &self.unique_id_map.borrow())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::object_sequence::ObjectSequence;
    use std::collections::HashMap as StdHashMap;

    /// Test entity: unique ID is readable and writable.
    #[derive(Debug, PartialEq, Eq)]
    struct Entity {
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

    /// Test sequence generator provider whose behavior matches Java's `ProjectContext`.
    #[derive(Default)]
    struct TestProvider {
        sequences: StdHashMap<TypeId, Rc<ObjectSequence>>,
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

    /// Construct a container with a test provider.
    fn container() -> (
        ProjectEntityContainer<Entity>,
        Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>,
    ) {
        let provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            Rc::new(RefCell::new(TestProvider::default()));
        (ProjectEntityContainer::new(Rc::clone(&provider)), provider)
    }

    /// Construct an entity.
    fn entity(unique_id: Option<i32>) -> Rc<RefCell<Entity>> {
        Rc::new(RefCell::new(Entity { unique_id }))
    }

    #[test]
    fn add_indexes_by_unique_id() {
        let (mut container, _) = container();
        assert!(container.is_empty());

        let first = entity(Some(5));
        let second = entity(Some(6));
        container.add(Rc::clone(&first));
        container.add(Rc::clone(&second));

        assert_eq!(container.len(), 2);
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(5)).expect("is indexed"),
            &first
        ));
        assert!(container.get_by_unique_id(Some(7)).is_none());
        assert!(container.get_by_unique_id(None).is_none());
    }

    #[test]
    fn elements_without_unique_id_are_not_indexed() {
        let (mut container, _) = container();
        container.add(entity(None));

        assert_eq!(container.len(), 1);
        assert!(container.get_by_unique_id(Some(0)).is_none());
    }

    #[test]
    fn removing_an_element_drops_its_index_entry() {
        let (mut container, _) = container();
        container.add(entity(Some(5)));

        let removed = container.remove(0).expect("index 0 exists");
        assert_eq!(removed.borrow().get_unique_id(), Some(5));
        assert!(container.get_by_unique_id(Some(5)).is_none());
    }

    #[test]
    fn re_adding_the_same_instance_is_not_a_clash() {
        let (mut container, _) = container();
        let first = entity(Some(5));

        container.add(Rc::clone(&first));
        container.add(Rc::clone(&first));

        assert_eq!(container.len(), 2);
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(5)).expect("is indexed"),
            &first
        ));

        // Re-adding the same instance does not enter the clash list
        container.fix_unique_id_clashes();
        assert_eq!(first.borrow().get_unique_id(), Some(5));
    }

    #[test]
    fn clashing_unique_ids_are_renumbered() {
        let (mut container, provider) = container();

        let first = entity(Some(5));
        let second = entity(Some(5));
        container.add(Rc::clone(&first));
        container.add(Rc::clone(&second));

        // On a clash the latter overwrites the index
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(5)).expect("is indexed"),
            &second
        ));

        // Fix the clash: the clashing entity takes a new value from the sequence (which starts at 1)
        container.fix_unique_id_clashes();
        assert_eq!(second.borrow().get_unique_id(), Some(1));
        assert_eq!(first.borrow().get_unique_id(), Some(5));

        // The index is rebuilt
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(1)).expect("is indexed"),
            &second
        ));
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(5)).expect("is indexed"),
            &first
        ));

        // The sequence has advanced
        assert_eq!(
            provider
                .borrow_mut()
                .get_unique_id_object_sequence(TypeId::of::<Entity>())
                .get_next(),
            2
        );
    }

    #[test]
    fn fixing_clashes_on_empty_list_is_a_no_op() {
        let (mut container, provider) = container();
        container.add(entity(Some(1)));

        container.fix_unique_id_clashes();

        // The sequence was not consumed
        assert_eq!(
            provider
                .borrow_mut()
                .get_unique_id_object_sequence(TypeId::of::<Entity>())
                .get_next(),
            1
        );
    }

    #[test]
    fn update_unique_id_moves_the_index_entry_and_syncs_sequence() {
        let (mut container, provider) = container();
        let first = entity(Some(5));
        container.add(Rc::clone(&first));

        container.update_unique_id(&first, Some(5), Some(9));

        assert!(container.get_by_unique_id(Some(5)).is_none());
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(9)).expect("is indexed"),
            &first
        ));

        // Java: the sequence generator is synchronized with the new unique ID
        assert_eq!(
            provider
                .borrow_mut()
                .get_unique_id_object_sequence(TypeId::of::<Entity>())
                .get_next(),
            10
        );
    }

    #[test]
    fn update_unique_id_detects_clash_with_other_element() {
        let (mut container, _) = container();
        let first = entity(Some(1));
        let second = entity(Some(2));
        container.add(Rc::clone(&first));
        container.add(Rc::clone(&second));

        // second switches to the same unique ID as first
        container.update_unique_id(&second, Some(2), Some(1));
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(1)).expect("is indexed"),
            &second
        ));

        // After fixing, second takes the next value in the sequence (synchronized to 2 after 1 was taken)
        container.fix_unique_id_clashes();
        assert_eq!(second.borrow().get_unique_id(), Some(2));
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(1)).expect("is indexed"),
            &first
        ));
    }

    #[test]
    fn update_unique_id_with_null_new_value_only_removes_old_entry() {
        let (mut container, _) = container();
        let first = entity(Some(5));
        container.add(Rc::clone(&first));

        container.update_unique_id(&first, Some(5), None);

        assert!(container.get_by_unique_id(Some(5)).is_none());
        assert_eq!(container.len(), 1);
    }

    #[test]
    fn list_operations_keep_the_index_in_sync() {
        let (mut container, _) = container();

        // add_at inserts and creates the index entry
        assert!(container.add_at(0, entity(Some(1))));
        assert!(!container.add_at(2, entity(Some(3))));
        assert_eq!(container.len(), 1);
        assert!(container.get_by_unique_id(Some(1)).is_some());

        // set replaces an element: consistent with Java, the unique ID index is **not** updated
        // (`ProjectEntityContainer` in Java overrides only `added` / `removed`)
        let replacement = entity(Some(2));
        let old = container.set(0, Rc::clone(&replacement)).expect("index 0 exists");
        assert_eq!(old.borrow().get_unique_id(), Some(1));
        assert!(container.get_by_unique_id(Some(2)).is_none());
        assert!(container.get_by_unique_id(Some(1)).is_some());
        assert!(Rc::ptr_eq(
            &container.get(0).expect("index 0 exists"),
            &replacement
        ));

        // clear does not trigger the removed callback, so the index keeps its old contents (consistent with Java's clear implementation)
        container.clear();
        assert!(container.is_empty());
        assert!(container.get_by_unique_id(Some(1)).is_some());
    }
}
