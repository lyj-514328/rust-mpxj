//! Corresponds to Java: `org/mpxj/NotesTopicContainer.java`

use std::cell::RefCell;
use std::rc::Rc;

use crate::notes_topic::NotesTopic;
use crate::project_entity_container::ProjectEntityContainer;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;

/// Collection of note topics available to the current project.
///
/// Corresponds to Java: `public class NotesTopicContainer extends ProjectEntityContainer<NotesTopic>`
///
/// # How the inheritance hierarchy is reproduced: composition + forwarding
///
/// Rust has no inheritance, so this class cannot "be" a [`ProjectEntityContainer`]; instead it composes one
/// `ProjectEntityContainer<NotesTopic>` and forwards its public behavior (`add` / `get` /
/// `get_by_unique_id` / `iter`, etc.). In Java this class only declares the constructor and `getDefaultTopic()`,
/// and all other methods are inherited from `ProjectEntityContainer`, so these forwarded methods are the "inherited methods".
///
/// # Dependencies on related modules
///
/// Java's `NotesTopic implements ProjectEntityWithUniqueID`; `src/notes_topic.rs` already provides the corresponding
/// `impl`, so this file can directly use `ProjectEntityContainer<NotesTopic>`.
///
/// # Differences from Java
///
/// - `getDefaultTopic()` internally uses [`NotesTopic::default_topic`] (corresponding to Java's static constant
/// `NotesTopic.DEFAULT`) to construct the default topic; Java returns the same static instance, whereas
/// `NotesTopic::default_topic()` returns a **newly created value** each time (consistent with the module's existing convention),
/// so repeated calls return equal values that are not the same `Rc`; the container only adds it once.
/// - **Does not** forward `fixUniqueIdClashes()`: in Java only entities whose unique ID can change enter the clash list,
/// and `NotesTopic`'s unique ID is `final`, so the clash list is always empty.
///
/// Skipped Java code: `fixUniqueIdClashes()` (see above).
///
/// Unported Java methods: none.
pub struct NotesTopicContainer {
/// Base class container.
    ///
/// Corresponds to Java's `extends ProjectEntityContainer<NotesTopic>`.
    container: ProjectEntityContainer<NotesTopic>,
}

impl NotesTopicContainer {
/// Constructor.
    ///
    /// Corresponds to Java: `public NotesTopicContainer(UniqueIdObjectSequenceProvider sequenceProvider)`
    pub fn new(sequence_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>) -> Self {
        Self {
            container: ProjectEntityContainer::new(sequence_provider),
        }
    }

/// Get the default topic; if the container does not yet have that unique ID, add it first and then return it.
    ///
    /// Corresponds to Java: `public NotesTopic getDefaultTopic()`
    ///
/// Java's implementation:
    /// ```text
    /// if (getByUniqueID(NotesTopic.DEFAULT.getUniqueID()) == null) { add(NotesTopic.DEFAULT); }
    /// return NotesTopic.DEFAULT;
    /// ```
    pub fn get_default_topic(&mut self) -> Rc<RefCell<NotesTopic>> {
        let default_topic = Rc::new(RefCell::new(NotesTopic::default_topic()));
        let unique_id = default_topic.borrow().get_unique_id();

        if self.container.get_by_unique_id(unique_id).is_none() {
            self.container.add(Rc::clone(&default_topic));
        }

        default_topic
    }

/// Get the sequence generator provider.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.getSequenceProvider()`
    pub fn get_sequence_provider(&self) -> Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> {
        self.container.get_sequence_provider()
    }

/// First value used when renumbering unique IDs.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.firstUniqueID()`
    pub fn first_unique_id(&self) -> i32 {
        self.container.first_unique_id()
    }

/// Append a note topic.
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(T element)`
    pub fn add(&mut self, element: Rc<RefCell<NotesTopic>>) {
        self.container.add(element);
    }

/// Insert a note topic at the given index (returns `false` if out of bounds).
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(int index, T element)`
    pub fn add_at(&mut self, index: usize, element: Rc<RefCell<NotesTopic>>) -> bool {
        self.container.add_at(index, element)
    }

/// Get a note topic by index.
    ///
    /// Corresponds to Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<Rc<RefCell<NotesTopic>>> {
        self.container.get(index)
    }

/// Replace the note topic at the given index (returns `None` if out of bounds).
    ///
    /// Corresponds to Java: `ListWithCallbacks.set(int index, T element)`
    pub fn set(
        &mut self,
        index: usize,
        element: Rc<RefCell<NotesTopic>>,
    ) -> Option<Rc<RefCell<NotesTopic>>> {
        self.container.set(index, element)
    }

/// Remove the note topic at the given index (returns `None` if out of bounds).
    ///
    /// Corresponds to Java: `ListWithCallbacks.remove(int index)`
    pub fn remove(&mut self, index: usize) -> Option<Rc<RefCell<NotesTopic>>> {
        self.container.remove(index)
    }

/// Get a note topic by unique ID.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<NotesTopic>>> {
        self.container.get_by_unique_id(id)
    }

/// Update the unique ID index.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.updateUniqueID(T element, Integer oldUniqueID, Integer newUniqueID)`
    pub fn update_unique_id(
        &mut self,
        element: &Rc<RefCell<NotesTopic>>,
        old_unique_id: Option<i32>,
        new_unique_id: Option<i32>,
    ) {
        self.container
            .update_unique_id(element, old_unique_id, new_unique_id);
    }

/// Number of note topics.
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

/// Iterate over the note topics in the container.
    ///
    /// Corresponds to Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<NotesTopic>>> {
        self.container.iter()
    }

/// Access the container contents as a slice.
    ///
/// Corresponds to Java: the list view of `ListWithCallbacks`.
    pub fn as_slice(&self) -> &[Rc<RefCell<NotesTopic>>] {
        self.container.as_slice()
    }

/// Clear the container (does not trigger the `removed` callback, consistent with Java).
    ///
    /// Corresponds to Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.container.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn container() -> NotesTopicContainer {
        let provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            Rc::new(RefCell::new(TestProvider::default()));
        NotesTopicContainer::new(provider)
    }

    #[test]
    fn get_default_topic_adds_it_once() {
        let mut container = container();
        assert!(container.is_empty());

        let first = container.get_default_topic();
        assert_eq!(first.borrow().get_unique_id(), Some(1));
        assert_eq!(first.borrow().get_name(), Some("Notes"));
        assert_eq!(container.len(), 1);

// A second call does not add it again (Java's `getByUniqueID(DEFAULT.getUniqueID()) == null` check)
        let second = container.get_default_topic();
        assert_eq!(container.len(), 1);
        assert_eq!(*second.borrow(), *first.borrow());
    }

    #[test]
    fn container_stores_and_indexes_topics() {
        let mut container = container();
        let topic = Rc::new(RefCell::new(NotesTopic::default_topic()));
        container.add(Rc::clone(&topic));

        assert_eq!(container.len(), 1);
        assert_eq!(container.iter().count(), 1);
        assert_eq!(container.as_slice().len(), 1);
        assert_eq!(container.first_unique_id(), 1);
        assert!(Rc::ptr_eq(&container.get(0).expect("index 0 exists"), &topic));
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(1)).expect("indexed"),
            &topic
        ));
        assert!(container.get_by_unique_id(Some(2)).is_none());
        assert!(container.get_by_unique_id(None).is_none());

// add_at out of bounds
        assert!(!container.add_at(
            5,
            Rc::new(RefCell::new(NotesTopic::default_topic()))
        ));
        assert_eq!(container.len(), 1);

// The index is cleaned up in sync after remove
        let removed = container.remove(0).expect("index exists");
        assert_eq!(removed.borrow().get_unique_id(), Some(1));
        assert!(container.is_empty());
        assert!(container.get_by_unique_id(Some(1)).is_none());
    }
}

