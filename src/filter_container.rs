//! Corresponds to Java: `org/mpxj/FilterContainer.java`
//!
//! Skipped/unported Java methods and the reasons:
//!
//! | Java | Rust |
//! |---|---|
//! | implicit default constructor | [`FilterContainer::new`] |
//! | `public void addFilter(Filter filter)` | [`FilterContainer::add_filter`] |
//! | `public void removeFilter(String filterName)` | [`FilterContainer::remove_filter`] |
//! | `public List<Filter> getResourceFilters()` | [`FilterContainer::get_resource_filters`] |
//! | `public List<Filter> getTaskFilters()` | [`FilterContainer::get_task_filters`] |
//! | `public Filter getFilterByName(String name)` | [`FilterContainer::get_filter_by_name`] |
//! | `public Filter getFilterByID(Integer id)` | [`FilterContainer::get_filter_by_id`] |
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: not used by this class, nothing skipped.
//!
//! # Differences from Java
//!
//! - **Element form**: Java's four collections store `Filter` references; the Rust side uniformly stores
//! `Rc<RefCell<Filter>>` (consistent with the other containers in this crate), so the lists and the two indexes share the same instances.
//! - **Removal by reference equality**: Java's `List.remove(Object)` locates the element using `equals`, but `Filter`
//! does not override `equals` (object identity); the Rust side replicates the identity semantics with `Rc::ptr_eq`.
//! - **`null` keys**: Java's `HashMap` allows `null` keys, so unnamed filters also enter
//! `m_filtersByName`; the Rust key types are `String` / `i32`, so filters with a `None` name or ID
//! are **not indexed** (they still enter the corresponding list according to their task/resource flag).
//! - **Parameters of `getFilterByName` / `getFilterByID`**: Java accepts parameters that may be `null`,
//! and the Rust side uses `Option<&str>` / `Option<i32>`.
//! - **List access**: Java returns a mutable list; the Rust side returns a read-only slice, and to modify use
//!   [`FilterContainer::add_filter`] / [`FilterContainer::remove_filter`].

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use crate::filter::Filter;

/// The collection of filter definitions under a project.
///
/// Corresponds to Java: `public class FilterContainer`
pub struct FilterContainer {
/// All task filters.
    ///
    /// Corresponds to Java: `private final List<Filter> m_taskFilters = new ArrayList<>();`
    m_task_filters: Vec<Rc<RefCell<Filter>>>,

/// All resource filters.
    ///
    /// Corresponds to Java: `private final List<Filter> m_resourceFilters = new ArrayList<>();`
    m_resource_filters: Vec<Rc<RefCell<Filter>>>,

/// Filters indexed by name.
    ///
    /// Corresponds to Java: `private final Map<String, Filter> m_filtersByName = new HashMap<>();`
    m_filters_by_name: HashMap<String, Rc<RefCell<Filter>>>,

/// Filters indexed by ID.
    ///
    /// Corresponds to Java: `private final Map<Integer, Filter> m_filtersByID = new HashMap<>();`
    m_filters_by_id: HashMap<i32, Rc<RefCell<Filter>>>,
}

impl FilterContainer {
/// Constructor.
    ///
/// Corresponds to Java: the implicit default constructor (the Java side does not declare a constructor explicitly).
    pub fn new() -> Self {
        Self {
            m_task_filters: Vec::new(),
            m_resource_filters: Vec::new(),
            m_filters_by_name: HashMap::new(),
            m_filters_by_id: HashMap::new(),
        }
    }

/// Add a filter definition to the project file (it enters the task/resource list according to its flag, and name and ID indexes are built).
    ///
    /// Corresponds to Java: `public void addFilter(Filter filter)`
    pub fn add_filter(&mut self, filter: Rc<RefCell<Filter>>) {
// Read the flag / name / ID; when already mutably borrowed, degrade gracefully to "not added to any collection".
        let Ok(filter_ref) = filter.try_borrow() else {
            return;
        };

        let is_task_filter = filter_ref.is_task_filter();
        let is_resource_filter = filter_ref.is_resource_filter();
        let name = filter_ref.get_name().map(str::to_owned);
        let id = filter_ref.get_id();
        drop(filter_ref);

        if is_task_filter {
            self.m_task_filters.push(Rc::clone(&filter));
        }

        if is_resource_filter {
            self.m_resource_filters.push(Rc::clone(&filter));
        }

// Java's HashMap allows null keys; the Rust side skips this (see the file header note)
        if let Some(name) = name {
            self.m_filters_by_name.insert(name, Rc::clone(&filter));
        }

        if let Some(id) = id {
            self.m_filters_by_id.insert(id, Rc::clone(&filter));
        }
    }

/// Remove a filter by name.
    ///
    /// Corresponds to Java: `public void removeFilter(String filterName)`
    pub fn remove_filter(&mut self, filter_name: Option<&str>) {
        let Some(filter) = self.get_filter_by_name(filter_name) else {
            return;
        };

        let (is_task_filter, is_resource_filter, id) = match filter.try_borrow() {
            Ok(filter_ref) => (
                filter_ref.is_task_filter(),
                filter_ref.is_resource_filter(),
                filter_ref.get_id(),
            ),
            Err(_) => (false, false, None),
        };

        if is_task_filter {
            remove_by_identity(&mut self.m_task_filters, &filter);
        }

        if is_resource_filter {
            remove_by_identity(&mut self.m_resource_filters, &filter);
        }

        if let Some(filter_name) = filter_name {
            self.m_filters_by_name.remove(filter_name);
        }

        if let Some(id) = id {
            self.m_filters_by_id.remove(&id);
        }
    }

/// Get all resource filters.
    ///
    /// Corresponds to Java: `public List<Filter> getResourceFilters()`
    pub fn get_resource_filters(&self) -> &[Rc<RefCell<Filter>>] {
        &self.m_resource_filters
    }

/// Get all task filters.
    ///
    /// Corresponds to Java: `public List<Filter> getTaskFilters()`
    pub fn get_task_filters(&self) -> &[Rc<RefCell<Filter>>] {
        &self.m_task_filters
    }

/// Get a filter by name.
    ///
    /// Corresponds to Java: `public Filter getFilterByName(String name)`
    pub fn get_filter_by_name(&self, name: Option<&str>) -> Option<Rc<RefCell<Filter>>> {
        let name = name?;
        self.m_filters_by_name.get(name).map(Rc::clone)
    }

/// Get a filter by ID.
    ///
    /// Corresponds to Java: `public Filter getFilterByID(Integer id)`
    pub fn get_filter_by_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Filter>>> {
        let id = id?;
        self.m_filters_by_id.get(&id).map(Rc::clone)
    }
}

impl Default for FilterContainer {
/// Corresponds to Java: the implicit default constructor, equivalent to [`FilterContainer::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for FilterContainer {
/// The Java side does not override `toString()`; the Rust side outputs the sizes of the collections to aid diagnosis.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilterContainer")
            .field("task_filters", &self.m_task_filters.len())
            .field("resource_filters", &self.m_resource_filters.len())
            .field("filters_by_name", &self.m_filters_by_name.len())
            .field("filters_by_id", &self.m_filters_by_id.len())
            .finish()
    }
}

/// Remove an element from a list by `Rc` identity (Java's `List.remove(Object)`).
fn remove_by_identity(filters: &mut Vec<Rc<RefCell<Filter>>>, filter: &Rc<RefCell<Filter>>) {
    if let Some(index) = filters
        .iter()
        .position(|candidate| Rc::ptr_eq(candidate, filter))
    {
        filters.remove(index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// Create a filter.
    fn filter(id: i32, name: &str, task: bool, resource: bool) -> Rc<RefCell<Filter>> {
        let mut value = Filter::new();
        value.set_id(Some(id));
        value.set_name(Some(name.to_owned()));
        value.set_is_task_filter(task);
        value.set_is_resource_filter(resource);
        Rc::new(RefCell::new(value))
    }

    #[test]
    fn container_basics() {
        let mut container = FilterContainer::new();
        assert!(container.get_task_filters().is_empty());
        assert!(container.get_resource_filters().is_empty());

        container.add_filter(filter(1, "Tasks", true, false));
        container.add_filter(filter(2, "Resources", false, true));

        assert_eq!(container.get_task_filters().len(), 1);
        assert_eq!(container.get_resource_filters().len(), 1);
        assert!(container.get_filter_by_id(None).is_none());
        assert!(container.get_filter_by_name(None).is_none());
        assert!(container.get_filter_by_id(Some(9)).is_none());
        assert!(container.get_filter_by_name(Some("Missing")).is_none());
    }

    #[test]
    fn both_flags_put_filter_in_both_lists() {
        let mut container = FilterContainer::new();
        let both = filter(1, "Both", true, true);
        container.add_filter(Rc::clone(&both));

        assert_eq!(container.get_task_filters().len(), 1);
        assert_eq!(container.get_resource_filters().len(), 1);
    }

    #[test]
    fn name_and_id_indexes_are_populated() {
        let mut container = FilterContainer::new();
        let entry = filter(4, "Entry", true, false);
        container.add_filter(Rc::clone(&entry));

        assert!(container
            .get_filter_by_name(Some("Entry"))
            .map(|found| Rc::ptr_eq(&found, &entry))
            .unwrap_or(false));
        assert!(container
            .get_filter_by_id(Some(4))
            .map(|found| Rc::ptr_eq(&found, &entry))
            .unwrap_or(false));

// Filters with the same name keep the last one added (Java's HashMap.put)
        let replacement = filter(5, "Entry", true, false);
        container.add_filter(Rc::clone(&replacement));
        assert!(container
            .get_filter_by_name(Some("Entry"))
            .map(|found| Rc::ptr_eq(&found, &replacement))
            .unwrap_or(false));
    }

    #[test]
    fn remove_filter_updates_all_collections() {
        let mut container = FilterContainer::new();
        container.add_filter(filter(1, "Entry", true, true));
        container.add_filter(filter(2, "Other", true, false));

        container.remove_filter(Some("Entry"));

        assert_eq!(container.get_task_filters().len(), 1);
        assert!(container.get_resource_filters().is_empty());
        assert!(container.get_filter_by_name(Some("Entry")).is_none());
        assert!(container.get_filter_by_id(Some(1)).is_none());
        assert_eq!(
            container
                .get_task_filters()
                .first()
                .and_then(|filter| filter.borrow().get_name().map(str::to_owned)),
            Some("Other".to_owned())
        );
    }

    #[test]
    fn remove_unknown_filter_is_noop() {
        let mut container = FilterContainer::new();
        container.add_filter(filter(1, "Entry", true, false));

        container.remove_filter(Some("Missing"));
        container.remove_filter(None);

        assert_eq!(container.get_task_filters().len(), 1);
    }

    #[test]
    fn unnamed_filter_is_not_indexed_but_listed() {
        let mut container = FilterContainer::new();
        let mut value = Filter::new();
        value.set_is_task_filter(true);
        let unnamed = Rc::new(RefCell::new(value));
        container.add_filter(Rc::clone(&unnamed));

        assert_eq!(container.get_task_filters().len(), 1);
        assert!(container.get_filter_by_name(None).is_none());
        assert!(container.get_filter_by_id(None).is_none());
    }

    #[test]
    fn debug_reports_collection_sizes() {
        let mut container = FilterContainer::new();
        container.add_filter(filter(1, "Entry", true, false));

        let text = format!("{container:?}");
        assert!(text.contains("task_filters: 1"));
        assert!(text.contains("resource_filters: 0"));
    }

    #[test]
    fn default_matches_new() {
        let container = FilterContainer::default();
        assert!(container.get_task_filters().is_empty());
    }
}
