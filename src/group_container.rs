//! Corresponds to Java: `org/mpxj/GroupContainer.java`
//!
//! Skipped / unported Java methods and reasons:
//!
//! | Java | Rust |
//! |---|---|
//! | `@Override protected void added(Group group)` | Private callback (registered in [`GroupContainer::new`] via [`crate::list_with_callbacks::ListWithCallbacks::set_added_callback`]) |
//! | `@Override protected void removed(Group group)` | Private callback (same as above, registered via `set_removed_callback`) |
//! | `public Group getByName(String name)` | [`GroupContainer::get_by_name`] |
//! | `add` / `add(int, T)` / `set` / `get` / `remove(int)` / `clear` / `size` / `isEmpty` / `iterator` inherited from `ListWithCallbacks<Group>` | [`GroupContainer::add`] / [`GroupContainer::add_at`] / [`GroupContainer::set`] / [`GroupContainer::get`] / [`GroupContainer::remove`] / [`GroupContainer::clear`] / [`GroupContainer::len`] / [`GroupContainer::is_empty`] / [`GroupContainer::iter`] (also provides [`GroupContainer::as_slice`]) |
//! | Element-lookup methods such as `remove(Object)` / `indexOf` / `contains` inherited from `ListWithCallbacks<Group>` | Unported: Java looks up with `equals` (object identity); the Rust elements are `Rc<RefCell<Group>>`, so callers can use [`GroupContainer::as_slice`] with `Rc::ptr_eq` to decide themselves |
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: not used by this class, nothing skipped.
//!
//! # Reproducing the Inheritance Relationship: Composition + Forwarding
//!
//! Java's `GroupContainer extends ListWithCallbacks<Group>`. Rust has no inheritance, so this
//! class instead **composes** a [`crate::list_with_callbacks::ListWithCallbacks`] and forwards
//! its public behavior; the **name index** maintained by the two override hooks `added` /
//! `removed` in Java is instead implemented with `ListWithCallbacks`'s callback fields (the
//! callback closures capture the index's `Rc` handle, see [`GroupContainer::new`]), semantically
//! consistent with Java.
//!
//! # Differences from Java
//!
//! - **Element form**: Java stores `Group` references; the Rust side stores
//!   `Rc<RefCell<Group>>` (consistent with this crate's other containers), so the name index
//!   and the list share the same instances.
//! - **Groups with a `null` name**: Java's `HashMap` allows a `null` key; Rust's
//!   `HashMap<String, _>` cannot express a `null` key, so unnamed groups are **not indexed**
//!   ([`GroupContainer::get_by_name`] therefore always returns `None`).
//! - **`replaced` callback**: Java's `replaced` is not overridden, so the name index is not
//!   updated after [`GroupContainer::set`] replaces an element (consistent with Java).
//! - **Degrade gracefully on index borrow failure**: the callback reads the group name via
//!   `try_borrow`; if the caller currently holds a mutable borrow of that group (a case that
//!   does not exist in Java), that index update is skipped rather than panicking.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use crate::group::Group;
use crate::list_with_callbacks::ListWithCallbacks;

/// Name-indexed group table (the list and the index share the same `Rc` instances).
type GroupIndex = Rc<RefCell<HashMap<String, Rc<RefCell<Group>>>>>;

/// A project's collection of group definitions.
///
/// Corresponds to Java: `public class GroupContainer extends ListWithCallbacks<Group>`
pub struct GroupContainer {
    /// The list itself (corresponds to Java's `extends ListWithCallbacks<Group>`).
    items: ListWithCallbacks<Rc<RefCell<Group>>>,

    /// Groups indexed by name.
    ///
    /// Corresponds to Java: `private final Map<String, Group> m_groupsByName = new HashMap<>();`
    groups_by_name: GroupIndex,
}

impl GroupContainer {
    /// Constructor (also registers the `added` / `removed` callbacks that maintain the name index).
    ///
    /// Corresponds to Java: implicit default constructor + the two override hooks `added` / `removed`.
    pub fn new() -> Self {
        let groups_by_name: GroupIndex = Rc::new(RefCell::new(HashMap::new()));
        let mut items = ListWithCallbacks::new();

        // Java: @Override protected void added(Group group) { m_groupsByName.put(group.getName(), group); }
        let added_index = Rc::clone(&groups_by_name);
        items.set_added_callback(Some(Box::new(move |group: &Rc<RefCell<Group>>| {
            let Some(name) = read_name(group) else {
                return;
            };

            added_index.borrow_mut().insert(name, Rc::clone(group));
        })));

        // Java: @Override protected void removed(Group group) { m_groupsByName.remove(group.getName()); }
        let removed_index = Rc::clone(&groups_by_name);
        items.set_removed_callback(Some(Box::new(move |group: &Rc<RefCell<Group>>| {
            let Some(name) = read_name(group) else {
                return;
            };

            removed_index.borrow_mut().remove(&name);
        })));

        Self {
            items,
            groups_by_name,
        }
    }

    /// Return a group definition by name.
    ///
    /// Corresponds to Java: `public Group getByName(String name)`
    pub fn get_by_name(&self, name: Option<&str>) -> Option<Rc<RefCell<Group>>> {
        let name = name?;
        self.groups_by_name.borrow().get(name).map(Rc::clone)
    }

    /// Append a group and update the name index.
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(T element)`
    pub fn add(&mut self, group: Rc<RefCell<Group>>) {
        self.items.add(group);
    }

    /// Insert a group at the given index and update the name index.
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(int index, T element)`
    ///
    /// Returns `false` when out of bounds (Java throws `IndexOutOfBoundsException`).
    pub fn add_at(&mut self, index: usize, group: Rc<RefCell<Group>>) -> bool {
        self.items.add_at(index, group)
    }

    /// Clear the list (consistent with Java: does not trigger the `removed` callback, so the name index is not cleared).
    ///
    /// Corresponds to Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Return the group at the given index; returns `None` when out of bounds.
    ///
    /// Corresponds to Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<&Rc<RefCell<Group>>> {
        self.items.get(index)
    }

    /// Replace the group at the given index and return the old element (does not update the name index, consistent with Java).
    ///
    /// Corresponds to Java: `ListWithCallbacks.set(int index, T element)`
    pub fn set(&mut self, index: usize, group: Rc<RefCell<Group>>) -> Option<Rc<RefCell<Group>>> {
        self.items.set(index, group)
    }

    /// Remove the group at the given index, return it, and update the name index.
    ///
    /// Corresponds to Java: `ListWithCallbacks.remove(int index)`
    pub fn remove(&mut self, index: usize) -> Option<Rc<RefCell<Group>>> {
        self.items.remove(index)
    }

    /// Number of groups.
    ///
    /// Corresponds to Java: `ListWithCallbacks.size()`
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the list is empty.
    ///
    /// Corresponds to Java: `ListWithCallbacks.isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Iterate over the groups in the list.
    ///
    /// Corresponds to Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<Group>>> {
        self.items.iter()
    }

    /// Access the groups in the list as a slice.
    ///
    /// Corresponds to Java: `ListWithCallbacks`'s list view.
    pub fn as_slice(&self) -> &[Rc<RefCell<Group>>] {
        self.items.as_slice()
    }
}

impl Default for GroupContainer {
    /// Corresponds to Java: implicit default constructor, equivalent to [`GroupContainer::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for GroupContainer {
    /// Java does not override `toString()`; the Rust side outputs the list size and name index keys for easier diagnosis.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut keys: Vec<String> = self.groups_by_name.borrow().keys().cloned().collect();
        keys.sort();

        f.debug_struct("GroupContainer")
            .field("size", &self.items.len())
            .field("groups_by_name", &keys)
            .finish()
    }
}

/// Read the group name; returns `None` when already mutably borrowed.
fn read_name(group: &Rc<RefCell<Group>>) -> Option<String> {
    group
        .try_borrow()
        .ok()
        .and_then(|group| group.get_name().map(str::to_owned))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a group with a name.
    fn group(name: &str) -> Rc<RefCell<Group>> {
        Rc::new(RefCell::new(Group::new(
            Some(1),
            Some(name.to_owned()),
            false,
        )))
    }

    #[test]
    fn container_basics() {
        let mut container = GroupContainer::new();
        assert!(container.is_empty());

        container.add(group("A"));
        container.add(group("B"));

        assert_eq!(container.len(), 2);
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert!(container.get(0).is_some());
        assert!(container.get(2).is_none());
    }

    #[test]
    fn get_by_name_finds_group() {
        let mut container = GroupContainer::new();
        let entry = group("Entry");
        container.add(Rc::clone(&entry));

        assert!(container
            .get_by_name(Some("Entry"))
            .map(|found| Rc::ptr_eq(&found, &entry))
            .unwrap_or(false));
        assert!(container.get_by_name(Some("Missing")).is_none());
        assert!(container.get_by_name(None).is_none());
    }

    #[test]
    fn last_group_with_same_name_wins() {
        let mut container = GroupContainer::new();
        container.add(group("Entry"));
        let second = group("Entry");
        container.add(Rc::clone(&second));

        assert!(container
            .get_by_name(Some("Entry"))
            .map(|found| Rc::ptr_eq(&found, &second))
            .unwrap_or(false));
    }

    #[test]
    fn remove_updates_name_index() {
        let mut container = GroupContainer::new();
        container.add(group("Entry"));

        assert!(container.remove(0).is_some());
        assert!(container.is_empty());
        assert!(container.get_by_name(Some("Entry")).is_none());
    }

    #[test]
    fn unnamed_group_is_not_indexed() {
        let mut container = GroupContainer::new();
        container.add(Rc::new(RefCell::new(Group::new(None, None, false))));

        assert_eq!(container.len(), 1);
        assert!(container.get_by_name(None).is_none());
    }

    #[test]
    fn add_at_set_and_clear_forward_to_list() {
        let mut container = GroupContainer::new();
        container.add(group("A"));

        assert!(container.add_at(0, group("B")));
        assert_eq!(
            container
                .get(0)
                .and_then(|group| group.borrow().get_name().map(str::to_owned)),
            Some("B".to_owned())
        );

        let replaced = container.set(0, group("C")).expect("replaced successfully");
        assert_eq!(replaced.borrow().get_name(), Some("B"));

        // Out of bounds does not modify the list
        assert!(!container.add_at(9, group("D")));
        assert_eq!(container.len(), 2);

        container.clear();
        assert!(container.is_empty());
    }

    #[test]
    fn default_matches_new() {
        let container = GroupContainer::default();
        assert!(container.is_empty());
    }

    #[test]
    fn debug_reports_index() {
        let mut container = GroupContainer::new();
        container.add(group("Entry"));

        let text = format!("{container:?}");
        assert!(text.contains("size: 1"));
        assert!(text.contains("Entry"));
    }
}
