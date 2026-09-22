//! Corresponds to Java: `org/mpxj/TableContainer.java`
//!
//! Skipped/unported Java methods and reasons:
//!
//! | Java | Rust |
//! |---|---|
//! | `@Override protected void added(Table table)` | Private callback (registered in [`TableContainer::new`] via [`crate::list_with_callbacks::ListWithCallbacks::set_added_callback`]) |
//! | `@Override protected void removed(Table table)` | Private callback (same as above, registered via `set_removed_callback`) |
//! | `public Table getTaskTableByName(String name)` | [`TableContainer::get_task_table_by_name`] |
//! | `public Table getResourceTableByName(String name)` | [`TableContainer::get_resource_table_by_name`] |
//! | `private Map<String, Table> getIndex(Table table)` | Private function `index_for` |
//! | Inherited from `ListWithCallbacks<Table>`: `add` / `add(int, T)` / `set` / `get` / `remove(int)` / `clear` / `size` / `isEmpty` / `iterator` | [`TableContainer::add`] / [`TableContainer::add_at`] / [`TableContainer::set`] / [`TableContainer::get`] / [`TableContainer::remove`] / [`TableContainer::clear`] / [`TableContainer::len`] / [`TableContainer::is_empty`] / [`TableContainer::iter`] (also provides [`TableContainer::as_slice`]) |
//! | Element-lookup methods inherited from `ListWithCallbacks<Table>`, such as `remove(Object)` / `indexOf` / `contains` | Not ported: Java looks up using `equals` (object identity), whereas Rust stores `Rc<RefCell<Table>>` elements, so callers can use [`TableContainer::as_slice`] with `Rc::ptr_eq` to decide for themselves |
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: not used by this class, nothing skipped.
//!
//! # How the inheritance relationship is restored: composition + forwarding
//!
//! Java's `TableContainer extends ListWithCallbacks<Table>`. Rust has no inheritance, so this class instead
//! **composes** a [`crate::list_with_callbacks::ListWithCallbacks`] and forwards its public behavior;
//! the **name index** that Java maintains through the two override hooks `added` / `removed` is instead
//! implemented with `ListWithCallbacks`'s callback fields (the callback closure captures the index's `Rc` handle, see
//! [`TableContainer::new`]), with semantics identical to Java.
//!
//! # Differences from Java
//!
//! - **Element form**: Java's list stores `Table` references; the Rust side stores `Rc<RefCell<Table>>`
//! (consistent with this crate's other containers), so the name index can share the same instances as the list.
//! - **Tables whose name is `null`**: Java's `HashMap` allows `null` keys, so unnamed tables can also enter the index;
//! Rust's `HashMap<String, _>` cannot express a `null` key, so the callback registered in [`TableContainer::new`]
//! does **not index** unnamed tables (`get_task_table_by_name(None)` is therefore always `None`).
//! - **The `replaced` callback**: Java's `ListWithCallbacks.replaced` is not overridden by `TableContainer`,
//! so the name index is **not** updated after [`TableContainer::set`] replaces an element (consistent with Java; kept as is here).
//! - **Degrade gracefully when the index borrow fails**: the callback reads the table name / resource flag via `try_borrow`; if the caller currently holds
//! a mutable borrow of that table (impossible in Java), that index update is skipped, without panicking.
//! - **The argument of `getTaskTableByName`**: Java accepts a `String` that may be `null`; the Rust side uses `Option<&str>`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use crate::list_with_callbacks::ListWithCallbacks;
use crate::table::Table;

/// A table list indexed by name (the list and the index share the same `Rc` instances).
type TableIndex = Rc<RefCell<HashMap<String, Rc<RefCell<Table>>>>>;

/// The collection of table definitions in a project.
///
/// Corresponds to Java: `public class TableContainer extends ListWithCallbacks<Table>`
pub struct TableContainer {
/// The list itself (corresponds to Java's `extends ListWithCallbacks<Table>`).
    items: ListWithCallbacks<Rc<RefCell<Table>>>,

/// Task tables indexed by name.
    ///
    /// Corresponds to Java: `private final Map<String, Table> m_taskTablesByName = new HashMap<>();`
    task_tables_by_name: TableIndex,

/// Resource tables indexed by name.
    ///
    /// Corresponds to Java: `private final Map<String, Table> m_resourceTablesByName = new HashMap<>();`
    resource_tables_by_name: TableIndex,
}

impl TableContainer {
/// Constructor (also registers the `added` / `removed` callbacks that maintain the name index).
    ///
/// Corresponds to Java: the implicit default constructor + the two override hooks `added` / `removed`.
    pub fn new() -> Self {
        let task_tables_by_name: TableIndex = Rc::new(RefCell::new(HashMap::new()));
        let resource_tables_by_name: TableIndex = Rc::new(RefCell::new(HashMap::new()));

        let mut items = ListWithCallbacks::new();

        // Java: @Override protected void added(Table table) { getIndex(table).put(table.getName(), table); }
        let added_task_index = Rc::clone(&task_tables_by_name);
        let added_resource_index = Rc::clone(&resource_tables_by_name);
        items.set_added_callback(Some(Box::new(
            move |table: &Rc<RefCell<Table>>| {
                let Some(name) = read_name(table) else {
                    return;
                };

                let index = index_for(table, &added_task_index, &added_resource_index);
                index.borrow_mut().insert(name, Rc::clone(table));
            },
        )));

        // Java: @Override protected void removed(Table table) { getIndex(table).remove(table.getName()); }
        let removed_task_index = Rc::clone(&task_tables_by_name);
        let removed_resource_index = Rc::clone(&resource_tables_by_name);
        items.set_removed_callback(Some(Box::new(
            move |table: &Rc<RefCell<Table>>| {
                let Some(name) = read_name(table) else {
                    return;
                };

                let index = index_for(table, &removed_task_index, &removed_resource_index);
                index.borrow_mut().remove(&name);
            },
        )));

        Self {
            items,
            task_tables_by_name,
            resource_tables_by_name,
        }
    }

/// Get a task table definition by name.
    ///
    /// Corresponds to Java: `public Table getTaskTableByName(String name)`
    pub fn get_task_table_by_name(&self, name: Option<&str>) -> Option<Rc<RefCell<Table>>> {
        let name = name?;
        self.task_tables_by_name.borrow().get(name).map(Rc::clone)
    }

/// Get a resource table definition by name.
    ///
    /// Corresponds to Java: `public Table getResourceTableByName(String name)`
    pub fn get_resource_table_by_name(&self, name: Option<&str>) -> Option<Rc<RefCell<Table>>> {
        let name = name?;
        self.resource_tables_by_name
            .borrow()
            .get(name)
            .map(Rc::clone)
    }

/// Append a table and update the name index.
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(T element)`
    pub fn add(&mut self, table: Rc<RefCell<Table>>) {
        self.items.add(table);
    }

/// Insert a table at the specified index and update the name index.
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(int index, T element)`
    ///
/// Returns `false` when out of bounds (Java throws `IndexOutOfBoundsException`).
    pub fn add_at(&mut self, index: usize, table: Rc<RefCell<Table>>) -> bool {
        self.items.add_at(index, table)
    }

/// Clear the list (consistent with Java: the `removed` callback is not triggered, so the name index is not cleared).
    ///
    /// Corresponds to Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.items.clear();
    }

/// Get a table by index; returns `None` when out of bounds.
    ///
    /// Corresponds to Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<&Rc<RefCell<Table>>> {
        self.items.get(index)
    }

/// Replace the table at the specified index and return the old element (does not update the name index, consistent with Java).
    ///
    /// Corresponds to Java: `ListWithCallbacks.set(int index, T element)`
    pub fn set(
        &mut self,
        index: usize,
        table: Rc<RefCell<Table>>,
    ) -> Option<Rc<RefCell<Table>>> {
        self.items.set(index, table)
    }

/// Remove the table at the specified index and return it, updating the name index.
    ///
    /// Corresponds to Java: `ListWithCallbacks.remove(int index)`
    pub fn remove(&mut self, index: usize) -> Option<Rc<RefCell<Table>>> {
        self.items.remove(index)
    }

/// The number of tables.
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

/// Iterate over the tables in the list.
    ///
    /// Corresponds to Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<Table>>> {
        self.items.iter()
    }

/// Access the tables in the list as a slice.
    ///
/// Corresponds to Java: the list view of `ListWithCallbacks`.
    pub fn as_slice(&self) -> &[Rc<RefCell<Table>>] {
        self.items.as_slice()
    }
}

impl Default for TableContainer {
/// Corresponds to Java: the implicit default constructor, equivalent to [`TableContainer::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for TableContainer {
/// Java does not override `toString()`; the Rust side outputs the list size and the keys of the two name indexes,
/// to aid diagnosis (it does not output `Table` contents).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let read_keys = |index: &TableIndex| -> Vec<String> {
            let mut keys: Vec<String> = index.borrow().keys().cloned().collect();
            keys.sort();
            keys
        };

        f.debug_struct("TableContainer")
            .field("size", &self.items.len())
            .field("task_tables", &read_keys(&self.task_tables_by_name))
            .field("resource_tables", &read_keys(&self.resource_tables_by_name))
            .finish()
    }
}

/// Get the name index to use (Java's `getIndex(Table)`).
fn index_for(
    table: &Rc<RefCell<Table>>,
    task_tables: &TableIndex,
    resource_tables: &TableIndex,
) -> TableIndex {
    if read_resource_flag(table) {
        Rc::clone(resource_tables)
    } else {
        Rc::clone(task_tables)
    }
}

/// Read the table name; returns `None` when it is already mutably borrowed.
fn read_name(table: &Rc<RefCell<Table>>) -> Option<String> {
    table
        .try_borrow()
        .ok()
        .and_then(|table| table.get_name().map(str::to_owned))
}

/// Read the resource table flag; when already mutably borrowed it is treated as `false` (task table).
fn read_resource_flag(table: &Rc<RefCell<Table>>) -> bool {
    table
        .try_borrow()
        .map(|table| table.get_resource_flag())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

/// Create a table with a name and resource flag.
    fn table(name: &str, resource: bool) -> Rc<RefCell<Table>> {
        let mut value = Table::new();
        value.set_name(Some(name.to_owned()));
        value.set_resource_flag(resource);
        Rc::new(RefCell::new(value))
    }

    #[test]
    fn container_basics() {
        let mut container = TableContainer::new();
        assert!(container.is_empty());

        container.add(table("Entry", false));
        container.add(table("Resources", true));

        assert_eq!(container.len(), 2);
        assert!(!container.is_empty());
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert!(container.get(0).is_some());
        assert!(container.get(2).is_none());
    }

    #[test]
    fn name_indexes_are_separated_by_resource_flag() {
        let mut container = TableContainer::new();
        let task_table = table("Entry", false);
        let resource_table = table("Entry", true);
        container.add(Rc::clone(&task_table));
        container.add(Rc::clone(&resource_table));

        assert!(container
            .get_task_table_by_name(Some("Entry"))
            .map(|found| Rc::ptr_eq(&found, &task_table))
            .unwrap_or(false));
        assert!(container
            .get_resource_table_by_name(Some("Entry"))
            .map(|found| Rc::ptr_eq(&found, &resource_table))
            .unwrap_or(false));

        assert!(container.get_task_table_by_name(Some("Other")).is_none());
        assert!(container.get_task_table_by_name(None).is_none());
    }

    #[test]
    fn last_table_with_same_name_wins() {
        let mut container = TableContainer::new();
        container.add(table("Entry", false));
        let second = table("Entry", false);
        container.add(Rc::clone(&second));

// Java's HashMap.put lets the last insertion win
        assert!(container
            .get_task_table_by_name(Some("Entry"))
            .map(|found| Rc::ptr_eq(&found, &second))
            .unwrap_or(false));
    }

    #[test]
    fn remove_updates_name_index() {
        let mut container = TableContainer::new();
        container.add(table("Entry", false));

        let removed = container.remove(0).expect("removal succeeded");
        assert!(container.is_empty());
        assert!(!removed.borrow().get_resource_flag());
        assert!(container.get_task_table_by_name(Some("Entry")).is_none());
    }

    #[test]
    fn unnamed_table_is_not_indexed() {
        let mut container = TableContainer::new();
        container.add(Rc::new(RefCell::new(Table::new())));

        assert_eq!(container.len(), 1);
        assert!(container.get_task_table_by_name(None).is_none());
        assert!(container.get_resource_table_by_name(None).is_none());
    }

    #[test]
    fn add_at_and_set_forward_to_list() {
        let mut container = TableContainer::new();
        container.add(table("A", false));

        assert!(container.add_at(0, table("B", false)));
        assert_eq!(container.len(), 2);
        assert_eq!(
            container
                .get(0)
                .and_then(|table| table.borrow().get_name().map(str::to_owned)),
            Some("B".to_owned())
        );

        let replaced = container.set(0, table("C", false)).expect("replacement succeeded");
        assert_eq!(replaced.borrow().get_name(), Some("B"));

// Out of bounds does not modify the list
        assert!(!container.add_at(9, table("D", false)));
        assert_eq!(container.len(), 2);
    }

    #[test]
    fn default_matches_new() {
        let container = TableContainer::default();
        assert!(container.is_empty());
    }

    #[test]
    fn debug_reports_indexes() {
        let mut container = TableContainer::new();
        container.add(table("Entry", false));
        container.add(table("Resources", true));

        let text = format!("{container:?}");
        assert!(text.contains("size: 2"));
        assert!(text.contains("Entry"));
        assert!(text.contains("Resources"));
    }
}
