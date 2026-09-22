//! Corresponds to Java: `org/mpxj/ViewState.java`
//!
//! Skipped / unported Java methods and reasons:
//!
//! | Java | Rust |
//! |---|---|
//! | `public ViewState(ProjectFile file, String viewName, List<Integer> uniqueIdList, int filterID)` | [`ViewState::new`] (the Rust side takes `Weak<RefCell<ProjectFile>>`, see below) |
//! | `public String getViewName()` | [`ViewState::get_view_name`] |
//! | `public List<Integer> getUniqueIdList()` | [`ViewState::get_unique_id_list`] |
//! | `public Filter getFilter()` | [`ViewState::get_filter`] |
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: not used by this class, nothing skipped.
//!
//! # Differences from Java
//!
//! - **Parent project reference**: Java's `m_file` is a strong reference; the Rust side stores
//! `Weak<RefCell<ProjectFile>>` (`ProjectFile` holds `ViewContainer`, and
//! `ViewContainer` in turn holds `ViewState`; using a strong reference would form an `Rc` cycle).
//! [`ViewState::get_filter`] returns `None` when the parent file has been dropped / is already borrowed
//! (Java throws `NullPointerException`).
//! - **Interface assumptions about related modules**: `getFilter()` relies on
//! `ProjectFile::get_filters(&self) -> &FilterContainer` (Java's
//! `ProjectFile.getFilters()` returns `FilterContainer`), consistent with how other modules in this crate
//! use `ProjectFile`.
//! - **`equals` / `hashCode`**: Java does not override them (object identity); the Rust side does not implement
//!   `PartialEq` / `Eq` / `Hash`.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::filter::Filter;
use crate::project_file::ProjectFile;

/// A view state saved in the project file.
///
/// Corresponds to Java: `public final class ViewState`
pub struct ViewState {
/// Parent project (weak reference).
    ///
    /// Corresponds to Java: `private final ProjectFile m_file;`
    m_file: Weak<RefCell<ProjectFile>>,

/// View name.
    ///
    /// Corresponds to Java: `private final String m_viewName;`
    m_view_name: Option<String>,

/// List of unique IDs of the view contents.
    ///
    /// Corresponds to Java: `private final List<Integer> m_uniqueIdList;`
    m_unique_id_list: Vec<i32>,

/// ID of the currently applied filter.
    ///
    /// Corresponds to Java: `private final Integer m_filterID;`
    m_filter_id: i32,
}

impl ViewState {
/// Constructor.
    ///
    /// Corresponds to Java: `public ViewState(ProjectFile file, String viewName, List<Integer> uniqueIdList, int filterID)`
    ///
/// The Rust side takes the parent project as a weak reference (see the file header).
    pub fn new(
        file: Weak<RefCell<ProjectFile>>,
        view_name: Option<String>,
        unique_id_list: Vec<i32>,
        filter_id: i32,
    ) -> Self {
        Self {
            m_file: file,
            m_view_name: view_name,
            m_unique_id_list: unique_id_list,
            m_filter_id: filter_id,
        }
    }

/// Get the view name.
    ///
    /// Corresponds to Java: `public String getViewName()`
    pub fn get_view_name(&self) -> Option<&str> {
        self.m_view_name.as_deref()
    }

/// Get the list of unique IDs of the view contents.
    ///
    /// Corresponds to Java: `public List<Integer> getUniqueIdList()`
    pub fn get_unique_id_list(&self) -> &[i32] {
        &self.m_unique_id_list
    }

/// Get the currently applied filter.
    ///
    /// Corresponds to Java: `public Filter getFilter()`
    ///
/// Return `None` when the parent project is not wired up / has been dropped / is already borrowed (Java throws
    /// `NullPointerException`).
    pub fn get_filter(&self) -> Option<Rc<RefCell<Filter>>> {
        let file = self.m_file.upgrade()?;
        let file_ref = file.try_borrow().ok()?;

        file_ref.get_filters().get_filter_by_id(Some(self.m_filter_id))
    }
}

impl std::fmt::Debug for ViewState {
/// Java does not override `toString()`; the Rust side outputs the fields (not the parent project, to avoid recursive `Rc` printing).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ViewState")
            .field("view_name", &self.m_view_name)
            .field("unique_id_list", &self.m_unique_id_list)
            .field("filter_id", &self.m_filter_id)
            .field("file_is_set", &(self.m_file.strong_count() > 0))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessors_round_trip() {
        let state = ViewState::new(Weak::new(), Some("Gantt Chart".to_owned()), vec![1, 2, 3], 7);

        assert_eq!(state.get_view_name(), Some("Gantt Chart"));
        assert_eq!(state.get_unique_id_list(), &[1, 2, 3]);
    }

    #[test]
    fn empty_unique_id_list_is_supported() {
        let state = ViewState::new(Weak::new(), None, Vec::new(), 0);

        assert_eq!(state.get_view_name(), None);
        assert!(state.get_unique_id_list().is_empty());
    }

    #[test]
    fn get_filter_returns_none_without_parent_file() {
        let state = ViewState::new(Weak::new(), Some("View".to_owned()), Vec::new(), 1);
        assert!(state.get_filter().is_none());
    }

    #[test]
    fn debug_reports_file_state() {
        let state = ViewState::new(Weak::new(), None, vec![1], 2);
        let text = format!("{state:?}");
        assert!(text.contains("filter_id: 2"));
        assert!(text.contains("file_is_set: false"));
    }
}
