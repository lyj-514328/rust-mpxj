//! Corresponds to Java: `org/mpxj/View.java`
//!
//! Unported Java methods: none (all 5 methods of this interface have been translated).
//!
//! Skipped / unported Java mechanisms: reflection, JDBC, XML/JAXB, GUI, `javax.*`, `Serializable`
//! are all unused.
//!
//! # How the interface is restored
//!
//! Java's `public interface View` is restored as the [`View`] trait on the Rust side, with the 5 methods corresponding one by one.
//!
//! # Differences from Java
//!
//! - **Implementing classes**: Java's implementing classes are in `org.mpxj.mpp` (`MPPView` etc.); this crate has already placed all
//! implementing classes under `src/mpp/` (`AbstractView` / `AbstractMppView` / `GanttChartView` / `GenericView` /
//! `View8` / `SplitView9` etc., all `impl View`); this trait only provides the contract, and other consumers
//! (such as the test double for `ViewContainer`) implement it themselves as well.
//! - **Nullable returns**: Java's implementations allow returning `null`, expressed with `Option` on the Rust side;
//! name-like return values use `Option<&str>` (avoiding string copies).
//! - **`getTable()`**: Java returns a `Table` reference; the Rust side returns
//! `Option<Rc<RefCell<Table>>>`, sharing the same data as the table instance held by this crate's container
//! (see [`crate::table_container::TableContainer`]).

use std::cell::RefCell;
use std::rc::Rc;

use crate::table::Table;
use crate::view_type::ViewType;

/// A view in an MS Project file (Gantt chart, resource sheet, etc.).
///
/// Corresponds to Java: `public interface View`
///
/// View data is written to the file only when the user first views that view; views that have never been viewed have no information in the MPP file.
pub trait View {
/// Get the view ID (does not represent the view type).
    ///
    /// Corresponds to Java: `Integer getID()`
    fn get_id(&self) -> Option<i32>;

/// Get the view name (MPXJ has stripped the internal `&` shortcut marker).
    ///
    /// Corresponds to Java: `String getName()`
    fn get_name(&self) -> Option<&str>;

/// Get the view type.
    ///
    /// Corresponds to Java: `ViewType getType()`
    fn get_type(&self) -> Option<ViewType>;

/// Get the name of the table part of the view.
    ///
    /// Corresponds to Java: `String getTableName()`
    fn get_table_name(&self) -> Option<&str>;

/// Get the [`Table`] instance corresponding to the table part of the view.
    ///
    /// Corresponds to Java: `Table getTable()`
    fn get_table(&self) -> Option<Rc<RefCell<Table>>>;
}

#[cfg(test)]
mod tests {
    use super::*;

/// Test view: verifies that the trait contract can be implemented (corresponding to the place of implementing classes such as `MPPView` in Java).
    struct MockView {
        id: Option<i32>,
        name: Option<String>,
        view_type: Option<ViewType>,
        table_name: Option<String>,
        table: Option<Rc<RefCell<Table>>>,
    }

    impl View for MockView {
        fn get_id(&self) -> Option<i32> {
            self.id
        }

        fn get_name(&self) -> Option<&str> {
            self.name.as_deref()
        }

        fn get_type(&self) -> Option<ViewType> {
            self.view_type
        }

        fn get_table_name(&self) -> Option<&str> {
            self.table_name.as_deref()
        }

        fn get_table(&self) -> Option<Rc<RefCell<Table>>> {
            self.table.clone()
        }
    }

    #[test]
    fn trait_is_implementable_as_object() {
        let mut table = Table::new();
        table.set_name(Some("Entry".to_owned()));
        let table = Rc::new(RefCell::new(table));

        let view: Box<dyn View> = Box::new(MockView {
            id: Some(1),
            name: Some("Gantt Chart".to_owned()),
            view_type: Some(ViewType::GanttChart),
            table_name: Some("Entry".to_owned()),
            table: Some(Rc::clone(&table)),
        });

        assert_eq!(view.get_id(), Some(1));
        assert_eq!(view.get_name(), Some("Gantt Chart"));
        assert_eq!(view.get_type(), Some(ViewType::GanttChart));
        assert_eq!(view.get_table_name(), Some("Entry"));
        assert!(view
            .get_table()
            .map(|found| Rc::ptr_eq(&found, &table))
            .unwrap_or(false));
    }
}
