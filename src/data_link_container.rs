//! Corresponds to Java: `org/mpxj/DataLinkContainer.java`
//!
//! Manage the data link ([`DataLink`]) definitions belonging to a project.
//!
//! # Java → Rust Method Correspondence
//!
//! Java's `DataLinkContainer` is an **empty-bodied class** (`extends
//! ListWithCallbacks<DataLink>`, with no members of its own), so this file only needs to
//! forward the inherited public behavior one by one to [`ListWithCallbacks`]:
//!
//! | Java (inherited from `ListWithCallbacks` / `AbstractList`) | Rust |
//! |---|---|
//! | `public boolean add(T e)` | [`DataLinkContainer::add`] |
//! | `public void add(int index, T element)` | [`DataLinkContainer::add_at`] |
//! | `public T set(int index, T element)` | [`DataLinkContainer::set`] |
//! | `public T remove(int index)` | [`DataLinkContainer::remove_at`] |
//! | `AbstractCollection.remove(Object)` | [`DataLinkContainer::remove_element`] (Java uses reference equality; Rust uses pointer identity, see below) |
//! | `public void clear()` | [`DataLinkContainer::clear`] |
//! | `public T get(int index)` | [`DataLinkContainer::get`] |
//! | `public int size()` | [`DataLinkContainer::len`] |
//! | `public boolean isEmpty()` | [`DataLinkContainer::is_empty`] |
//! | `AbstractList.iterator()` | [`DataLinkContainer::iter`] |
//!
//! # Reproducing the Inheritance Hierarchy: Composition + Forwarding
//!
//! Java uses inheritance to obtain the list implementation; Rust has no inheritance, so this
//! type **composes** a [`ListWithCallbacks`], consistent with the "wrap + forward" approach of
//! [`crate::project_calendar_container::ProjectCalendarContainer`]. The three callbacks of
//! `ListWithCallbacks` (`added` / `removed` / `replaced`) remain unregistered in this class —
//! Java's `DataLinkContainer` does not override them either, so the behavior is identical (all
//! are empty implementations).
//!
//! # Differences from Java
//!
//! - **Element form**: Java stores `DataLink` references (mutable objects); the Rust side
//!   follows this crate's convention of "shared entities use `Rc<RefCell<T>>`" and stores
//!   `Rc<RefCell<DataLink>>`, so callers can still hold and modify the same link object as in
//!   Java.
//! - **`remove(Object)`**: Java's `AbstractCollection.remove(Object)` locates the element
//!   with `equals` (`DataLink` does not override it, i.e. object identity). The Rust side
//!   cannot express the comparison semantics of "object identity" (`&DataLink`'s `==` is a
//!   value comparison), so it uses [`RefCell::as_ptr`]'s **pointer identity** comparison,
//!   equivalent to Java's default `equals`.
//! - **Out-of-bounds does not panic**: Java's `get` / `set` / `remove(int)` / `add(int, T)`
//!   throw `IndexOutOfBoundsException` when out of bounds; the Rust side follows the
//!   convention of [`ListWithCallbacks`] (returns `None` / `false` when out of bounds) and
//!   does not panic.
//! - **Return value of `add(T)`**: Java always returns `true`; the Rust side returns nothing
//!   (consistent with [`ListWithCallbacks::add`]).
//! - **`sort` not provided**: Java's `ListWithCallbacks` itself has no sort method (sorting is
//!   provided by subclasses such as `ProjectEntityWithIDContainer`), and `DataLinkContainer`
//!   does not involve it.
//!
//! # Skipped Java Mechanisms
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::cell::RefCell;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use crate::data_link::DataLink;
use crate::list_with_callbacks::ListWithCallbacks;

/// A project's collection of data links.
///
/// Corresponds to Java: `public class DataLinkContainer extends ListWithCallbacks<DataLink>`
pub struct DataLinkContainer {
    /// List implementation (Java's `extends ListWithCallbacks<DataLink>`).
    list: ListWithCallbacks<Rc<RefCell<DataLink>>>,
}

impl DataLinkContainer {
    /// Constructor.
    ///
    /// Corresponds to Java: `DataLinkContainer`'s implicit no-arg constructor.
    pub fn new() -> Self {
        Self {
            list: ListWithCallbacks::new(),
        }
    }

    /// Append a link to the end of the list.
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(T e)`
    pub fn add(&mut self, link: Rc<RefCell<DataLink>>) {
        self.list.add(link);
    }

    /// Insert a link at the given index; returns `false` when out of bounds.
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(int index, T element)`
    pub fn add_at(&mut self, index: usize, link: Rc<RefCell<DataLink>>) -> bool {
        self.list.add_at(index, link)
    }

    /// Replace the link at the given index and return the old link that was replaced; returns `None` when out of bounds.
    ///
    /// Corresponds to Java: `ListWithCallbacks.set(int index, T element)`
    pub fn set(
        &mut self,
        index: usize,
        link: Rc<RefCell<DataLink>>,
    ) -> Option<Rc<RefCell<DataLink>>> {
        self.list.set(index, link)
    }

    /// Remove the link at the given index and return it; returns `None` when out of bounds.
    ///
    /// Corresponds to Java: `ListWithCallbacks.remove(int index)`
    pub fn remove_at(&mut self, index: usize) -> Option<Rc<RefCell<DataLink>>> {
        self.list.remove(index)
    }

    /// Remove a link by pointer identity (equivalent to Java's `AbstractCollection.remove(Object)`).
    ///
    /// Corresponds to Java: `AbstractCollection.remove(Object o)`
    pub fn remove_element(&mut self, link: &DataLink) -> bool {
        let target = link as *const DataLink;
        let index = self
            .list
            .iter()
            .position(|candidate| candidate.as_ptr() as *const DataLink == target);

        match index {
            Some(index) => {
                self.list.remove(index);
                true
            }
            None => false,
        }
    }

    /// Clear the list (consistent with Java: does **not** trigger the `removed` callback).
    ///
    /// Corresponds to Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.list.clear();
    }

    /// Return the link at the given index; returns `None` when out of bounds.
    ///
    /// Corresponds to Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<Rc<RefCell<DataLink>>> {
        self.list.get(index).cloned()
    }

    /// Number of links.
    ///
    /// Corresponds to Java: `ListWithCallbacks.size()`
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Whether the list is empty.
    ///
    /// Corresponds to Java: `ListWithCallbacks.isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Iterate over the links in the list.
    ///
    /// Corresponds to Java: `AbstractList.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<DataLink>>> {
        self.list.iter()
    }

    /// Access the list contents as a slice.
    ///
    /// Corresponds to Java: `AbstractList`'s list view.
    pub fn as_slice(&self) -> &[Rc<RefCell<DataLink>>] {
        self.list.as_slice()
    }
}

impl Default for DataLinkContainer {
    /// Corresponds to Java: `DataLinkContainer`'s implicit no-arg constructor.
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for DataLinkContainer {
    /// Java does not override `toString()`; the Rust side outputs the list contents (elements are
    /// `Rc<RefCell<DataLink>>`; elements currently borrowed render as
    /// `RefCell { value: <borrowed> }` in their `Debug` output and do not panic).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataLinkContainer")
            .field("size", &self.list.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_key::FieldKey;
    use crate::task_field::TaskField;

    /// Construct a link handle.
    fn link(id: &str) -> Rc<RefCell<DataLink>> {
        Rc::new(RefCell::new(DataLink::new(id)))
    }

    #[test]
    fn container_basics() {
        let mut container = DataLinkContainer::new();
        assert!(container.is_empty());
        assert_eq!(container.len(), 0);
        assert!(container.get(0).is_none());

        container.add(link("1"));
        container.add(link("2"));

        assert_eq!(container.len(), 2);
        assert!(!container.is_empty());
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert_eq!(container.get(0).map(|l| l.borrow().get_id().to_owned()), Some("1".to_owned()));
        assert!(container.get(2).is_none());
    }

    #[test]
    fn add_at_and_set_round_trip() {
        let mut container = DataLinkContainer::new();
        let first = link("1");
        let second = link("2");
        let third = link("3");

        container.add(Rc::clone(&first));
        container.add(Rc::clone(&third));

        assert!(container.add_at(1, Rc::clone(&second)));
        // Out-of-bounds insertion does not modify the list (Java throws IndexOutOfBoundsException)
        assert!(!container.add_at(9, Rc::clone(&first)));
        assert_eq!(
            container.iter().map(|l| l.borrow().get_id().to_owned()).collect::<Vec<_>>(),
            vec!["1".to_owned(), "2".to_owned(), "3".to_owned()]
        );

        let replacement = link("9");
        let old = container.set(0, Rc::clone(&replacement)).expect("index 0 exists");
        assert!(Rc::ptr_eq(&old, &first));
        assert!(container.set(9, Rc::clone(&first)).is_none());
    }

    #[test]
    fn remove_at_and_remove_element() {
        let mut container = DataLinkContainer::new();
        let first = link("1");
        let second = link("2");
        container.add(Rc::clone(&first));
        container.add(Rc::clone(&second));

        // Java: remove(int index)
        let removed = container.remove_at(0).expect("index 0 exists");
        assert!(Rc::ptr_eq(&removed, &first));
        assert!(container.remove_at(9).is_none());

        // Java: remove(Object) — pointer identity
        let removed = {
            let second_ref = second.borrow();
            container.remove_element(&second_ref)
        };
        assert!(removed);
        assert!(container.is_empty());

        // No longer in the container: removing again returns false
        let removed = {
            let second_ref = second.borrow();
            container.remove_element(&second_ref)
        };
        assert!(!removed);
    }

    #[test]
    fn clear_drops_all_links() {
        let mut container = DataLinkContainer::new();
        container.add(link("1"));
        container.add(link("2"));

        container.clear();

        assert!(container.is_empty());
        assert_eq!(container.iter().count(), 0);
    }

    #[test]
    fn links_are_shared_with_callers_like_java() {
        let mut container = DataLinkContainer::new();
        let shared = link("1");
        container.add(Rc::clone(&shared));

        // The caller can still modify the same object in the container (Java semantics)
        shared
            .borrow_mut()
            .set_source_field(Some(FieldKey::Task(TaskField::Name)));

        let from_container = container.get(0).expect("added to container");
        assert_eq!(
            from_container.borrow().get_source_field(),
            Some(FieldKey::Task(TaskField::Name))
        );
        assert!(Rc::ptr_eq(&from_container, &shared));
    }

    #[test]
    fn default_matches_new() {
        let container = DataLinkContainer::default();
        assert!(container.is_empty());
        assert!(format!("{container:?}").contains("size: 0"));
    }
}
