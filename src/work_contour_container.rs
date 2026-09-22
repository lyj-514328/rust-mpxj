//! Corresponds to Java: `org/mpxj/WorkContourContainer.java`

use std::cell::RefCell;
use std::rc::Rc;

use crate::project_entity_container::ProjectEntityContainer;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;
use crate::work_contour::WorkContour;

/// The collection of work contours (resource curves) available to the current project.
///
/// Corresponds to Java: `public class WorkContourContainer extends ProjectEntityContainer<WorkContour>`
///
/// # How the inheritance hierarchy is reproduced: composition + forwarding
///
/// Rust has no inheritance, so this class cannot "be" a [`ProjectEntityContainer`]; instead it composes one
/// `ProjectEntityContainer<WorkContour>` and forwards its public behavior (`add` / `get` /
/// `get_by_unique_id` / `iter`, etc.). In Java this class declares only a constructor, and all other methods are inherited from
/// `ProjectEntityContainer`, so these forwarding methods are the "inherited methods".
///
/// # Differences from Java
///
/// - `fixUniqueIdClashes()` is **not** forwarded: in Java only entities with a mutable unique ID enter the clash list,
/// whereas the unique ID of `WorkContour` is `final` (its `ProjectEntityWithUniqueID` implementation only has read
/// methods), so the clash list is always empty and forwarding that method is pointless.
/// - Java's `WorkContour implements ProjectEntityWithUniqueID`: the Rust implementation has been moved to
/// [`crate::work_contour`]; this file only keeps the container logic.
///
/// Skipped Java code: `fixUniqueIdClashes()` (see above).
///
/// Skipped Java methods: none.
pub struct WorkContourContainer {
/// Base class container.
    ///
/// Corresponds to Java's `extends ProjectEntityContainer<WorkContour>`.
    container: ProjectEntityContainer<WorkContour>,
}

impl WorkContourContainer {
/// Constructor.
    ///
    /// Corresponds to Java: `public WorkContourContainer(UniqueIdObjectSequenceProvider sequenceProvider)`
    pub fn new(sequence_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>) -> Self {
        Self {
            container: ProjectEntityContainer::new(sequence_provider),
        }
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

/// Append a work contour.
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(T element)`
    pub fn add(&mut self, element: Rc<RefCell<WorkContour>>) {
        self.container.add(element);
    }

/// Get the work contour at the given index.
    ///
    /// Corresponds to Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<Rc<RefCell<WorkContour>>> {
        self.container.get(index)
    }

/// Insert a work contour at the specified index (returns `false` when out of bounds).
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(int index, T element)`
    pub fn add_at(&mut self, index: usize, element: Rc<RefCell<WorkContour>>) -> bool {
        self.container.add_at(index, element)
    }

/// Replace the work contour at the specified index (returns `None` when out of bounds).
    ///
    /// Corresponds to Java: `ListWithCallbacks.set(int index, T element)`
    pub fn set(
        &mut self,
        index: usize,
        element: Rc<RefCell<WorkContour>>,
    ) -> Option<Rc<RefCell<WorkContour>>> {
        self.container.set(index, element)
    }

/// Remove the work contour at the specified index (returns `None` when out of bounds).
    ///
    /// Corresponds to Java: `ListWithCallbacks.remove(int index)`
    pub fn remove(&mut self, index: usize) -> Option<Rc<RefCell<WorkContour>>> {
        self.container.remove(index)
    }

/// Get the work contour by unique ID.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<WorkContour>>> {
        self.container.get_by_unique_id(id)
    }

/// Update the unique ID index.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.updateUniqueID(T element, Integer oldUniqueID, Integer newUniqueID)`
    pub fn update_unique_id(
        &mut self,
        element: &Rc<RefCell<WorkContour>>,
        old_unique_id: Option<i32>,
        new_unique_id: Option<i32>,
    ) {
        self.container
            .update_unique_id(element, old_unique_id, new_unique_id);
    }

/// Number of work contours.
    ///
    /// Corresponds to Java: `ListWithCallbacks.size()`
    pub fn len(&self) -> usize {
        self.container.len()
    }

/// Whether the container is empty.
    ///
    /// Corresponds to Java: `ListWithCallbacks.isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.container.is_empty()
    }

/// Iterate over the work contours in the container.
    ///
    /// Corresponds to Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<WorkContour>>> {
        self.container.iter()
    }

/// Access the container contents as a slice.
    ///
/// Corresponds to Java: the `ListWithCallbacks` list view.
    pub fn as_slice(&self) -> &[Rc<RefCell<WorkContour>>] {
        self.container.as_slice()
    }

/// Clear the container (does not trigger the `removed` callback, matching Java).
    ///
    /// Corresponds to Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.container.clear();
    }
}

impl std::fmt::Debug for WorkContourContainer {
/// Java does not override `toString()`; the Rust side outputs the inner container.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkContourContainer")
            .field("container", &self.container)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
    use crate::common::object_sequence::ObjectSequence;
    use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;
    use std::any::TypeId;
    use std::collections::HashMap;

/// Sequence generator provider for tests.
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
    fn container() -> WorkContourContainer {
        let provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            Rc::new(RefCell::new(TestProvider::default()));
        WorkContourContainer::new(provider)
    }

    #[test]
    fn work_contour_exposes_unique_id_through_the_trait() {
        let flat = WorkContour::flat();

        assert_eq!(ProjectEntityWithUniqueID::get_unique_id(&flat), Some(1));
        assert_eq!(flat.get_unique_id(), Some(1));
    }

    #[test]
    fn container_stores_and_indexes_work_contours() {
        let mut container = container();
        assert!(container.is_empty());

        let flat = Rc::new(RefCell::new(WorkContour::flat()));
        let bell = Rc::new(RefCell::new(WorkContour::bell()));

        container.add(Rc::clone(&flat));
        container.add(Rc::clone(&bell));

        assert_eq!(container.len(), 2);
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert_eq!(container.first_unique_id(), 1);
        assert_eq!(
            container.get(0).map(|item| item.borrow().get_name().map(str::to_owned)),
            Some(Some("FLAT".to_owned()))
        );

        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(7)).expect("BELL indexed"),
            &bell
        ));
        assert!(container.get_by_unique_id(Some(1)).is_some());
        assert!(container.get_by_unique_id(Some(99)).is_none());
        assert!(container.get_by_unique_id(None).is_none());
    }

    #[test]
    fn removing_and_clearing_updates_the_container() {
        let mut container = container();
        let flat = Rc::new(RefCell::new(WorkContour::flat()));
        container.add(Rc::clone(&flat));

// add_at out of bounds
        assert!(!container.add_at(5, Rc::new(RefCell::new(WorkContour::bell()))));
        assert_eq!(container.len(), 1);

// the index is cleaned up in sync after remove
        let removed = container.remove(0).expect("index exists");
        assert_eq!(removed.borrow().get_name(), Some("FLAT"));
        assert!(container.is_empty());
        assert!(container.get_by_unique_id(Some(1)).is_none());
        assert!(container.remove(0).is_none());

// clear does not trigger the removed callback
        let bell = Rc::new(RefCell::new(WorkContour::bell()));
        container.add(Rc::clone(&bell));
        container.clear();
        assert!(container.is_empty());
        assert!(container.set(0, Rc::clone(&bell)).is_none());
    }

    #[test]
    fn update_unique_id_moves_the_index_entry() {
        let mut container = container();
        let flat = Rc::new(RefCell::new(WorkContour::flat()));
        container.add(Rc::clone(&flat));

        container.update_unique_id(&flat, Some(1), Some(100));

        assert!(container.get_by_unique_id(Some(1)).is_none());
        assert!(container.get_by_unique_id(Some(100)).is_some());
    }
}
