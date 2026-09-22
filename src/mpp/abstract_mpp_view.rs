//! Corresponds to Java: `org/mpxj/mpp/AbstractMppView.java`
//!
//! Common base class for all MPP views. On the Java side this class only provides a constructor (which passes `parent` to `AbstractView`),
//! with no state or behavior of its own.
//!
//! Skipped Java mechanisms and reasons:
//!
//! | Java | Reason |
//! |---|---|
//! | Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable` | Not used by this class |
//!
//! # How inheritance is reproduced: composition + forwarding
//!
//! | Java | Rust |
//! |---|---|
//! | `AbstractMppView extends AbstractView` | [`AbstractMppView`] **holds** an [`AbstractView`] and forwards the 5 `View` methods and 4 setters one by one (see each `Corresponds to Java:` comment) |
//! | `public AbstractMppView(ProjectFile parent)` | [`AbstractMppView::new`] |
//! | `parent` is held by `AbstractView` | Same as [`AbstractView`] (`Weak<RefCell<ProjectFile>>`, avoiding an `Rc` cycle) |
//!
//! Differences from Java: Java's `m_file` / `m_id` / `m_name` / `m_type` / `m_tableName` are
//! protected fields of `AbstractView`, read and written directly by subclasses (`View8`, `GenericView`); the Rust side instead expresses them via
//! setters such as [`AbstractMppView::set_id`] and [`AbstractMppView::as_abstract_view`] (when the underlying
//! instance is needed).

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use crate::mpp::abstract_view::AbstractView;
use crate::project_file::ProjectFile;
use crate::table::Table;
use crate::view::View;
use crate::view_type::ViewType;

/// Common base class for all MPP views.
///
/// Corresponds to Java: `public abstract class AbstractMppView extends AbstractView`
pub struct AbstractMppView {
/// Corresponds to Java: `extends AbstractView` (Rust uses composition instead of inheritance)
    view: AbstractView,
}

impl AbstractMppView {
/// Constructor.
    ///
    /// Corresponds to Java: `public AbstractMppView(ProjectFile parent)`
    ///
    /// Java: `super(parent);`
    pub fn new(parent: &Rc<RefCell<ProjectFile>>) -> Self {
        Self {
            view: AbstractView::new(parent),
        }
    }

/// Get the parent project file.
    ///
/// Corresponds to Java: `AbstractView.m_file` (protected field)
    pub fn get_file(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.view.get_file()
    }

/// Set the view ID.
    ///
/// Corresponds to Java: `AbstractView.m_id` (assignment to a protected field)
    pub fn set_id(&mut self, id: Option<i32>) {
        self.view.set_id(id);
    }

/// Set the view name.
    ///
/// Corresponds to Java: `AbstractView.m_name` (assignment to a protected field)
    pub fn set_name(&mut self, name: Option<String>) {
        self.view.set_name(name);
    }

/// Set the view type.
    ///
/// Corresponds to Java: `AbstractView.m_type` (assignment to a protected field)
    pub fn set_type(&mut self, type_: Option<ViewType>) {
        self.view.set_type(type_);
    }

/// Set the name of the table part of the view.
    ///
/// Corresponds to Java: `AbstractView.m_tableName` (assignment to a protected field)
    pub fn set_table_name(&mut self, table_name: Option<String>) {
        self.view.set_table_name(table_name);
    }

/// Get the internally held base class instance.
    ///
/// Rust-specific: Java inheritance is expressed by composition on the Rust side; use this method when the base class instance is needed.
    pub fn as_abstract_view(&self) -> &AbstractView {
        &self.view
    }

/// Get the internally held base class instance (mutable).
    ///
/// Rust-specific: see [`AbstractMppView::as_abstract_view`].
    pub fn as_abstract_view_mut(&mut self) -> &mut AbstractView {
        &mut self.view
    }
}

impl View for AbstractMppView {
/// Corresponds to Java: `AbstractView.getID()` (inherited)
    fn get_id(&self) -> Option<i32> {
        self.view.get_id()
    }

/// Corresponds to Java: `AbstractView.getName()` (inherited)
    fn get_name(&self) -> Option<&str> {
        self.view.get_name()
    }

/// Corresponds to Java: `AbstractView.getType()` (inherited)
    fn get_type(&self) -> Option<ViewType> {
        self.view.get_type()
    }

/// Corresponds to Java: `AbstractView.getTableName()` (inherited)
    fn get_table_name(&self) -> Option<&str> {
        self.view.get_table_name()
    }

/// Corresponds to Java: `AbstractView.getTable()` (inherited)
    fn get_table(&self) -> Option<Rc<RefCell<Table>>> {
        self.view.get_table()
    }
}

impl Display for AbstractMppView {
/// Corresponds to Java: `AbstractView.toString()` (not overridden by this class)
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.view, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// Construct a project file with an "Entry" table.
    fn file_with_table() -> Rc<RefCell<ProjectFile>> {
        let file = ProjectFile::new();
        let mut table = Table::new();
        table.set_name(Some("Entry".to_owned()));
        file.borrow_mut()
            .get_tables_mut()
            .add(Rc::new(RefCell::new(table)));
        file
    }

/// Setters / getters and `toString()` are all forwarded to the internal base class instance.
    #[test]
    fn forwards_to_abstract_view() {
        let file = file_with_table();
        let mut view = AbstractMppView::new(&file);
        view.set_id(Some(1));
        view.set_name(Some("Resource Sheet".to_owned()));
        view.set_type(Some(ViewType::ResourceSheet));
        view.set_table_name(Some("Entry".to_owned()));

        assert_eq!(view.get_id(), Some(1));
        assert_eq!(view.get_name(), Some("Resource Sheet"));
        assert_eq!(view.get_type(), Some(ViewType::ResourceSheet));
        assert_eq!(view.get_table_name(), Some("Entry"));
        assert!(view.get_table().is_some());
        assert_eq!(
            view.to_string(),
            "[View id=1 type=RESOURCE_SHEET name=Resource Sheet table=Entry]"
        );
    }

/// `as_abstract_view_mut` and the forwarding methods operate on the same instance.
    #[test]
    fn exposes_inner_abstract_view() {
        let file = file_with_table();
        let mut view = AbstractMppView::new(&file);

        view.as_abstract_view_mut().set_id(Some(9));
        assert_eq!(view.as_abstract_view().get_id(), Some(9));
        assert_eq!(view.get_id(), Some(9));
    }

/// When no name/type is set, `toString()` renders `null` and does not panic.
    #[test]
    fn empty_view_is_supported() {
        let file = file_with_table();
        let mut view = AbstractMppView::new(&file);

        assert_eq!(view.to_string(), "[View id=null type=null name=null]");
        assert!(view.get_file().is_some());
        view.set_table_name(Some("Missing".to_owned()));
        assert!(view.get_table().is_none());
    }
}
