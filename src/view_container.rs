//! Corresponds to Java: `org/mpxj/ViewContainer.java`
//!
//! Skipped/unported Java methods and the reasons:
//!
//! | Java | Rust |
//! |---|---|
//! | `public void setViewState(ViewState viewState)` | [`ViewContainer::set_view_state`] |
//! | `public ViewState getViewState()` | [`ViewContainer::get_view_state`] |
//! | `add` / `add(int, T)` / `set` / `get` / `remove(int)` / `clear` / `size` / `isEmpty` / `iterator` inherited from `ListWithCallbacks<View>` | [`ViewContainer::add`] / [`ViewContainer::add_at`] / [`ViewContainer::set`] / [`ViewContainer::get`] / [`ViewContainer::remove`] / [`ViewContainer::clear`] / [`ViewContainer::len`] / [`ViewContainer::is_empty`] / [`ViewContainer::iter`] (plus [`ViewContainer::as_slice`]) |
//! | methods that look up elements such as `remove(Object)` / `indexOf` / `contains` inherited from `ListWithCallbacks<View>` | Not ported: Java uses `equals` (object identity) for lookup, whereas on the Rust side elements are `Rc<RefCell<dyn View>>`; the caller can use [`ViewContainer::as_slice`] together with `Rc::ptr_eq` to decide for itself |
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: not used by this class, nothing skipped.
//!
//! # How the inheritance relationship is reproduced: composition + forwarding
//!
//! Java's `ViewContainer extends ListWithCallbacks<View>`. Rust has no inheritance, so this class instead
//! **composes** a [`crate::list_with_callbacks::ListWithCallbacks`] and forwards its public behavior.
//! In Java this class does not override any `ListWithCallbacks` hook, so no callbacks need to be registered after composition.
//!
//! # Differences from Java
//!
//! - **Element form**: `View` is an interface (the [`crate::view::View`] trait on the Rust side), so the element type is
//! `Rc<RefCell<dyn View>>`; Java stores a `View` reference. `Rc<RefCell<..>>` is consistent with the other
//! containers in this crate, making it easy for the list and the view state to share the same instances.
//! - **`getViewState()`**: Java may return `null`; the Rust side returns `Option<&ViewState>`;
//! when ownership is needed, the caller can `Clone` it (this class provides no extra `_mut` accessor, and neither does Java).

use std::cell::RefCell;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use crate::list_with_callbacks::ListWithCallbacks;
use crate::view::View;
use crate::view_state::ViewState;

/// The collection of views in a project (and the view state saved in the file).
///
/// Corresponds to Java: `public class ViewContainer extends ListWithCallbacks<View>`
pub struct ViewContainer {
/// The list itself (corresponds to Java's `extends ListWithCallbacks<View>`).
    items: ListWithCallbacks<Rc<RefCell<dyn View>>>,

/// The view state saved in this file.
    ///
    /// Corresponds to Java: `private ViewState m_viewState;`
    m_view_state: Option<ViewState>,
}

impl ViewContainer {
/// Constructor.
    ///
/// Corresponds to Java: the implicit default constructor (Java declares no explicit constructor).
    pub fn new() -> Self {
        Self {
            items: ListWithCallbacks::new(),
            m_view_state: None,
        }
    }

/// Set the view state saved in this file.
    ///
    /// Corresponds to Java: `public void setViewState(ViewState viewState)`
    pub fn set_view_state(&mut self, view_state: Option<ViewState>) {
        self.m_view_state = view_state;
    }

/// Get the view state saved in this file.
    ///
    /// Corresponds to Java: `public ViewState getViewState()`
    pub fn get_view_state(&self) -> Option<&ViewState> {
        self.m_view_state.as_ref()
    }

/// Append a view.
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(T element)`
    pub fn add(&mut self, view: Rc<RefCell<dyn View>>) {
        self.items.add(view);
    }

/// Insert a view at the specified index.
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(int index, T element)`
    ///
/// Returns `false` when out of bounds (Java throws `IndexOutOfBoundsException`).
    pub fn add_at(&mut self, index: usize, view: Rc<RefCell<dyn View>>) -> bool {
        self.items.add_at(index, view)
    }

/// Clear the list (matching Java: does not trigger the `removed` callback).
    ///
    /// Corresponds to Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.items.clear();
    }

/// Get the view at the given index; returns `None` when out of bounds.
    ///
    /// Corresponds to Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<&Rc<RefCell<dyn View>>> {
        self.items.get(index)
    }

/// Replace the view at the specified index and return the old element.
    ///
    /// Corresponds to Java: `ListWithCallbacks.set(int index, T element)`
    pub fn set(
        &mut self,
        index: usize,
        view: Rc<RefCell<dyn View>>,
    ) -> Option<Rc<RefCell<dyn View>>> {
        self.items.set(index, view)
    }

/// Remove the view at the specified index and return it.
    ///
    /// Corresponds to Java: `ListWithCallbacks.remove(int index)`
    pub fn remove(&mut self, index: usize) -> Option<Rc<RefCell<dyn View>>> {
        self.items.remove(index)
    }

/// Number of views.
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

/// Iterate over the views in the list.
    ///
    /// Corresponds to Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<dyn View>>> {
        self.items.iter()
    }

/// Access the views in the list as a slice.
    ///
/// Corresponds to Java: the `ListWithCallbacks` list view.
    pub fn as_slice(&self) -> &[Rc<RefCell<dyn View>>] {
        self.items.as_slice()
    }
}

impl Default for ViewContainer {
/// Corresponds to Java: the implicit default constructor, equivalent to [`ViewContainer::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for ViewContainer {
/// Java does not override `toString()`; the Rust side outputs the size and whether view state is saved
/// (`dyn View` does not implement `Debug`, so element contents are not output).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewContainer")
            .field("size", &self.items.len())
            .field("view_state_is_set", &self.m_view_state.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::Table;
    use crate::view_type::ViewType;

/// View implementation for tests (the corresponding Java implementation class is in `org.mpxj.mpp` and is not ported here).
    struct MockView {
        id: Option<i32>,
        name: Option<String>,
    }

    impl View for MockView {
        fn get_id(&self) -> Option<i32> {
            self.id
        }

        fn get_name(&self) -> Option<&str> {
            self.name.as_deref()
        }

        fn get_type(&self) -> Option<ViewType> {
            None
        }

        fn get_table_name(&self) -> Option<&str> {
            None
        }

        fn get_table(&self) -> Option<Rc<RefCell<Table>>> {
            None
        }
    }

/// Create a test view.
    fn view(id: i32, name: &str) -> Rc<RefCell<dyn View>> {
        Rc::new(RefCell::new(MockView {
            id: Some(id),
            name: Some(name.to_owned()),
        }))
    }

    #[test]
    fn container_basics() {
        let mut container = ViewContainer::new();
        assert!(container.is_empty());
        assert!(container.get_view_state().is_none());

        container.add(view(1, "Gantt Chart"));
        container.add(view(2, "Resource Sheet"));

        assert_eq!(container.len(), 2);
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert!(container.get(0).is_some());
        assert!(container.get(2).is_none());
        assert_eq!(
            container
                .get(1)
                .and_then(|view| view.borrow().get_name().map(str::to_owned)),
            Some("Resource Sheet".to_owned())
        );
    }

    #[test]
    fn view_state_round_trips() {
        let mut container = ViewContainer::new();
        container.set_view_state(Some(ViewState::new(
            std::rc::Weak::new(),
            Some("Gantt Chart".to_owned()),
            vec![1, 2],
            3,
        )));

        let state = container.get_view_state().expect("view state set");
        assert_eq!(state.get_view_name(), Some("Gantt Chart"));
        assert_eq!(state.get_unique_id_list(), &[1, 2]);

        container.set_view_state(None);
        assert!(container.get_view_state().is_none());
    }

    #[test]
    fn list_operations_forward_to_base_list() {
        let mut container = ViewContainer::new();
        container.add(view(1, "A"));

        assert!(container.add_at(0, view(2, "B")));
        assert_eq!(
            container
                .get(0)
                .and_then(|view| view.borrow().get_name().map(str::to_owned)),
            Some("B".to_owned())
        );

        let replaced = container.set(0, view(3, "C")).expect("replacement succeeded");
        assert_eq!(
            replaced.borrow().get_name().map(str::to_owned),
            Some("B".to_owned())
        );

        assert!(container.remove(0).is_some());
        assert_eq!(container.len(), 1);

        container.clear();
        assert!(container.is_empty());

        assert!(!container.add_at(9, view(4, "D")));
    }

    #[test]
    fn default_matches_new() {
        let container = ViewContainer::default();
        assert!(container.is_empty());
    }

    #[test]
    fn debug_reports_size_and_state() {
        let mut container = ViewContainer::new();
        container.add(view(1, "A"));

        let text = format!("{container:?}");
        assert!(text.contains("size: 1"));
        assert!(text.contains("view_state_is_set: false"));
    }
}
