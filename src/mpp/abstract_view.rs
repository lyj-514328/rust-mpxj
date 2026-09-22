//! Corresponds to Java: `org/mpxj/mpp/AbstractView.java`
//!
//! Common base class for all views: stores the view ID, name, type, and the name of the "table part", and looks up
//! the corresponding table instance by name in the current project's table container.
//!
//! Skipped Java mechanisms and reasons:
//!
//! | Java | Reason |
//! |---|---|
//! | Reflection / JDBC / XML(JAXB) / GUI / `javax.*` | this class does not use them |
//! | `Serializable` | Rust has no such mechanism (and this class does not implement it on the Java side either) |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `protected final ProjectFile m_file` | private field `m_file: Weak<RefCell<ProjectFile>>` | `ProjectFile` holds the view via `ViewContainer`, and a strong reference back from the view to the parent file would form an `Rc` cycle, so a weak reference is used as required; when the parent file has been released / is being mutably borrowed, [`AbstractView::get_table`] returns `None` (Java would throw `NullPointerException`) |
//! | `protected final TableContainer m_tables = parent.getTables()` | not cached; fetched on demand via the parent file | on the Rust side `TableContainer` is held inline by `ProjectFile` (not an `Rc`), so it cannot be borrowed long-term in the view; therefore it is resolved via `m_file` each time (same semantics as Java) |
//! | `protected Integer m_id` and the other 4 writable fields | private fields + [`AbstractView::set_id`] / [`AbstractView::set_name`] / [`AbstractView::set_type`] / [`AbstractView::set_table_name`] | Rust has no inheritance, so subclasses (such as `GenericView`, `View8`) cannot write base class fields directly; setters are used instead to express Java's protected field assignments |
//! | `implements View` | `impl View for AbstractView` | corresponds one-to-one with the [`crate::view::View`] trait |
//! | `toString()` | `impl Display` | Java uses string concatenation and renders `null` as `"null"`; the Rust side stays consistent |
//!
//! # Inheritance reconstruction approach: composition + forwarding
//!
//! Java's `AbstractView implements View` is a **base class that is inherited**: `AbstractMppView extends
//! AbstractView`. Rust has no inheritance, so `AbstractMppView` (see `abstract_mpp_view.rs`) instead **holds**
//! an instance of this type and forwards all 5 `View` methods and the 4 setters, with equivalent semantics.
//! This class itself has no abstract methods, so it can be instantiated directly.

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::{Rc, Weak};

use crate::project_file::ProjectFile;
use crate::table::Table;
use crate::view::View;
use crate::view_type::ViewType;

/// Common base class for all views.
///
/// Corresponds to Java: `public abstract class AbstractView implements View`
pub struct AbstractView {
/// Parent project file.
    ///
    /// Corresponds to Java field: `protected final ProjectFile m_file;`
    m_file: Weak<RefCell<ProjectFile>>,

/// View ID.
    ///
    /// Corresponds to Java field: `protected Integer m_id;`
    m_id: Option<i32>,

/// View name.
    ///
    /// Corresponds to Java field: `protected String m_name;`
    m_name: Option<String>,

/// View type.
    ///
    /// Corresponds to Java field: `protected ViewType m_type;`
    m_type: Option<ViewType>,

/// Name of the table part in the view.
    ///
    /// Corresponds to Java field: `protected String m_tableName;`
    m_table_name: Option<String>,
}

impl AbstractView {
/// Constructor.
    ///
    /// Corresponds to Java: `public AbstractView(ProjectFile parent)`
    ///
/// The Java side also caches `parent.getTables()` in `m_tables`; the Rust side does not cache it, see the file header.
    pub fn new(parent: &Rc<RefCell<ProjectFile>>) -> Self {
        Self {
            m_file: Rc::downgrade(parent),
            m_id: None,
            m_name: None,
            m_type: None,
            m_table_name: None,
        }
    }

/// Get the parent project file.
    ///
/// Rust-specific: Java's `m_file` is a protected field read directly by subclasses; returns
    /// `None`.
    pub fn get_file(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.m_file.upgrade()
    }

/// Set the view ID.
    ///
/// Rust-specific: corresponds to a Java subclass assigning to `protected Integer m_id`.
    pub fn set_id(&mut self, id: Option<i32>) {
        self.m_id = id;
    }

/// Set the view name.
    ///
/// Rust-specific: corresponds to a Java subclass assigning to `protected String m_name`.
    pub fn set_name(&mut self, name: Option<String>) {
        self.m_name = name;
    }

/// Set the view type.
    ///
/// Rust-specific: corresponds to a Java subclass assigning to `protected ViewType m_type`.
    pub fn set_type(&mut self, type_: Option<ViewType>) {
        self.m_type = type_;
    }

/// Set the name of the table part in the view.
    ///
/// Rust-specific: corresponds to a Java subclass assigning to `protected String m_tableName`.
    pub fn set_table_name(&mut self, table_name: Option<String>) {
        self.m_table_name = table_name;
    }
}

impl View for AbstractView {
    /// Corresponds to Java: `@Override public Integer getID()`
    fn get_id(&self) -> Option<i32> {
        self.m_id
    }

    /// Corresponds to Java: `@Override public String getName()`
    fn get_name(&self) -> Option<&str> {
        self.m_name.as_deref()
    }

    /// Corresponds to Java: `@Override public ViewType getType()`
    fn get_type(&self) -> Option<ViewType> {
        self.m_type
    }

    /// Corresponds to Java: `@Override public String getTableName()`
    fn get_table_name(&self) -> Option<&str> {
        self.m_table_name.as_deref()
    }

/// Get the table instance corresponding to the table part in the view.
    ///
    /// Corresponds to Java: `@Override public Table getTable()`
    ///
    /// Java: `return m_tables.getTaskTableByName(m_tableName);`
/// Returns `None` when the parent file has been released or is being mutably borrowed (Java would throw `NullPointerException`).
    fn get_table(&self) -> Option<Rc<RefCell<Table>>> {
        let file = self.m_file.upgrade()?;
        let file_ref = file.try_borrow().ok()?;

        file_ref
            .get_tables()
            .get_task_table_by_name(self.m_table_name.as_deref())
    }
}

impl Display for AbstractView {
    /// Corresponds to Java: `@Override public String toString()`
    ///
    /// Java: `"[View id=" + m_id + " type=" + m_type + " name=" + m_name
    /// + (m_tableName == null ? "" : " table=" + m_tableName) + "]"`
    ///
/// When concatenating, Java renders `null` as `"null"` (`ViewType` goes through its `toString()`, i.e. the constant name).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[View id={} type={} name={}", java_int(self.m_id), java_view_type(self.m_type), java_str(self.m_name.as_deref()))?;

        if let Some(table_name) = &self.m_table_name {
            write!(f, " table={table_name}")?;
        }

        f.write_str("]")
    }
}

/// Render a nullable integer using Java's string concatenation rules (`null` -> `"null"`).
fn java_int(value: Option<i32>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

/// Render a nullable `ViewType` using Java's string concatenation rules (`null` -> `"null"`, otherwise use `toString()`).
fn java_view_type(value: Option<ViewType>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

/// Render a nullable string using Java's string concatenation rules (`null` -> `"null"`).
fn java_str(value: Option<&str>) -> &str {
    value.unwrap_or("null")
}

#[cfg(test)]
mod tests {
    use super::*;

/// Construct a project file with an "Entry" table and a "Resource" table.
    fn file_with_tables() -> Rc<RefCell<ProjectFile>> {
        let file = ProjectFile::new();

        let mut entry = Table::new();
        entry.set_name(Some("Entry".to_owned()));
        let mut resource = Table::new();
        resource.set_name(Some("Resource".to_owned()));

        {
            let mut file_ref = file.borrow_mut();
            let tables = file_ref.get_tables_mut();
            tables.add(Rc::new(RefCell::new(entry)));
            tables.add(Rc::new(RefCell::new(resource)));
        }

        file
    }

/// The constructor, accessors, and `toString()` output are consistent with Java.
    #[test]
    fn accessors_and_display_match_java() {
        let file = file_with_tables();
        let mut view = AbstractView::new(&file);
        view.set_id(Some(3));
        view.set_name(Some("Gantt Chart".to_owned()));
        view.set_type(Some(ViewType::GanttChart));
        view.set_table_name(Some("Entry".to_owned()));

        assert_eq!(view.get_id(), Some(3));
        assert_eq!(view.get_name(), Some("Gantt Chart"));
        assert_eq!(view.get_type(), Some(ViewType::GanttChart));
        assert_eq!(view.get_table_name(), Some("Entry"));
        assert_eq!(
            view.to_string(),
            "[View id=3 type=GANTT_CHART name=Gantt Chart table=Entry]"
        );
    }

/// Unassigned fields are rendered as `"null"` in `toString()`, and `table=` is not output.
    #[test]
    fn empty_view_display_matches_java() {
        let file = file_with_tables();
        let view = AbstractView::new(&file);

        assert_eq!(view.get_id(), None);
        assert_eq!(view.get_name(), None);
        assert_eq!(view.get_type(), None);
        assert_eq!(view.get_table_name(), None);
        assert_eq!(view.to_string(), "[View id=null type=null name=null]");
    }

/// `getTable()` hits the task table in the table container by name.
    #[test]
    fn get_table_resolves_by_name() {
        let file = file_with_tables();
        let mut view = AbstractView::new(&file);
        view.set_table_name(Some("Entry".to_owned()));

        let expected = file
            .borrow()
            .get_tables()
            .get_task_table_by_name(Some("Entry"))
            .expect("Entry exists in the table");
        let found = view.get_table().expect("should find table");
        assert!(Rc::ptr_eq(&found, &expected));

// name does not exist / name not set -> None
        view.set_table_name(Some("Missing".to_owned()));
        assert!(view.get_table().is_none());
        view.set_table_name(None);
        assert!(view.get_table().is_none());
    }

/// After the parent file is released, getting the table returns `None` (Java would throw `NullPointerException`) without panicking.
    #[test]
    fn dropped_parent_yields_none() {
        let file = file_with_tables();
        let mut view = AbstractView::new(&file);
        view.set_table_name(Some("Entry".to_owned()));

        assert!(view.get_file().is_some());
        drop(file);

        assert!(view.get_file().is_none());
        assert!(view.get_table().is_none());
    }
}
