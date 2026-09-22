//! Corresponds to Java: `org/mpxj/Filter.java`
//!
//! Skipped/unported Java methods and reasons:
//!
//! | Java | Rust |
//! |---|---|
//! | Implicit default constructor | [`Filter::new`] |
//! | `public void setID(Integer id)` / `public Integer getID()` | [`Filter::set_id`] / [`Filter::get_id`] |
//! | `public void setName(String name)` / `public String getName()` | [`Filter::set_name`] / [`Filter::get_name`] |
//! | `public void setShowRelatedSummaryRows(boolean)` / `public boolean getShowRelatedSummaryRows()` | [`Filter::set_show_related_summary_rows`] / [`Filter::get_show_related_summary_rows`] |
//! | `public void setCriteria(GenericCriteria criteria)` / `public GenericCriteria getCriteria()` | [`Filter::set_criteria`] / [`Filter::get_criteria`] |
//! | `public boolean isTaskFilter()` / `public void setIsTaskFilter(boolean flag)` | [`Filter::is_task_filter`] / [`Filter::set_is_task_filter`] |
//! | `public boolean isResourceFilter()` / `public void setIsResourceFilter(boolean flag)` | [`Filter::is_resource_filter`] / [`Filter::set_is_resource_filter`] |
//! | `public boolean evaluate(FieldContainer container, Map<GenericCriteriaPrompt, Object> promptValues)` | [`Filter::evaluate`] (changed to a generic + `Any` downcast, see below) |
//! | `public void setPrompts(List<GenericCriteriaPrompt> prompts)` / `public List<GenericCriteriaPrompt> getPrompts()` | [`Filter::set_prompts`] / [`Filter::get_prompts`] |
//! | `@Override public String toString()` | [`Display`] |
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: not used by this class, nothing skipped.
//!
//! # Differences from Java
//!
//! 1. **`container instanceof Task` → `Any` downcast**: when evaluation fails and "show related summary rows" is enabled, Java checks
//! whether the container is a `Task` and recursively tests its child tasks. Rust trait
//! objects cannot be downcast directly, so [`Filter::evaluate`] is changed to accept `&C` **generically**
//! (`C: FieldContainer + 'static`), converting internally to `&dyn Any` and then
//! `downcast_ref::<Task>()`, which is equivalent to Java's check. Callers may pass `&Task`,
//! `&Resource`, or `&ResourceAssignment` (the latter two take the "not a Task" branch).
//! 2. **`prompts`**: Java's `m_prompts` defaults to `null`, so `getPrompts()` may return `null`;
//! the Rust side uses `Vec<GenericCriteriaPrompt>` (empty by default), and [`Filter::get_prompts`] returns an empty slice.
//! 3. **`promptValues`**: Java allows `null`; the Rust side can pass an empty map (equivalent to "no answer found").
//! 4. **`equals` / `hashCode`**: Java does not override them (object identity); the Rust side does not implement
//!    `PartialEq` / `Eq` / `Hash`.
//! 5. **Borrow failure during field traversal**: child tasks are obtained via `try_borrow` when testing recursively;
//! if a child task is already mutably borrowed (impossible in Java), that child task is treated as "not matching", without panicking.

use std::any::Any;
use std::fmt::{self, Display, Formatter};

use crate::field_container::FieldContainer;
use crate::generic_criteria::{GenericCriteria, PromptValues};
use crate::generic_criteria_prompt::GenericCriteriaPrompt;
use crate::task::Task;

/// A filter applied to a task view or resource view.
///
/// Corresponds to Java: `public final class Filter`
pub struct Filter {
/// The filter's unique ID.
    ///
    /// Corresponds to Java: `private Integer m_id;`
    m_id: Option<i32>,
/// The filter name.
    ///
    /// Corresponds to Java: `private String m_name;`
    m_name: Option<String>,
/// Whether this is a task filter.
    ///
    /// Corresponds to Java: `private boolean m_isTaskFilter;`
    m_is_task_filter: bool,
/// Whether this is a resource filter.
    ///
    /// Corresponds to Java: `private boolean m_isResourceFilter;`
    m_is_resource_filter: bool,
/// Whether to show related summary rows.
    ///
    /// Corresponds to Java: `private boolean m_showRelatedSummaryRows;`
    m_show_related_summary_rows: bool,
/// The filter criteria.
    ///
    /// Corresponds to Java: `private GenericCriteria m_criteria;`
    m_criteria: Option<GenericCriteria>,
/// The prompts required by this filter.
    ///
    /// Corresponds to Java: `private List<GenericCriteriaPrompt> m_prompts;`
    m_prompts: Vec<GenericCriteriaPrompt>,
}

impl Filter {
/// Constructor.
    ///
/// Corresponds to Java: the implicit default constructor (Java declares no explicit constructor).
    pub fn new() -> Self {
        Self {
            m_id: None,
            m_name: None,
            m_is_task_filter: false,
            m_is_resource_filter: false,
            m_show_related_summary_rows: false,
            m_criteria: None,
            m_prompts: Vec::new(),
        }
    }

/// Set the filter's unique ID.
    ///
    /// Corresponds to Java: `public void setID(Integer id)`
    pub fn set_id(&mut self, id: Option<i32>) {
        self.m_id = id;
    }

/// Get the filter's unique ID.
    ///
    /// Corresponds to Java: `public Integer getID()`
    pub fn get_id(&self) -> Option<i32> {
        self.m_id
    }

/// Set the filter name.
    ///
    /// Corresponds to Java: `public void setName(String name)`
    pub fn set_name(&mut self, name: Option<String>) {
        self.m_name = name;
    }

/// Get the filter name.
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.m_name.as_deref()
    }

/// Set the "show related summary rows" flag.
    ///
    /// Corresponds to Java: `public void setShowRelatedSummaryRows(boolean showRelatedSummaryRows)`
    pub fn set_show_related_summary_rows(&mut self, show_related_summary_rows: bool) {
        self.m_show_related_summary_rows = show_related_summary_rows;
    }

/// Get the "show related summary rows" flag.
    ///
    /// Corresponds to Java: `public boolean getShowRelatedSummaryRows()`
    pub fn get_show_related_summary_rows(&self) -> bool {
        self.m_show_related_summary_rows
    }

/// Set the filter criteria.
    ///
    /// Corresponds to Java: `public void setCriteria(GenericCriteria criteria)`
    pub fn set_criteria(&mut self, criteria: Option<GenericCriteria>) {
        self.m_criteria = criteria;
    }

/// Get the filter criteria.
    ///
    /// Corresponds to Java: `public GenericCriteria getCriteria()`
    pub fn get_criteria(&self) -> Option<&GenericCriteria> {
        self.m_criteria.as_ref()
    }

/// Get the filter criteria mutably (Rust-specific, to allow in-place modification).
    ///
    /// Corresponds to Java: `public GenericCriteria getCriteria()`
    pub fn get_criteria_mut(&mut self) -> Option<&mut GenericCriteria> {
        self.m_criteria.as_mut()
    }

/// Whether this is a task filter.
    ///
    /// Corresponds to Java: `public boolean isTaskFilter()`
    pub fn is_task_filter(&self) -> bool {
        self.m_is_task_filter
    }

/// Set the task filter flag.
    ///
    /// Corresponds to Java: `public void setIsTaskFilter(boolean flag)`
    pub fn set_is_task_filter(&mut self, flag: bool) {
        self.m_is_task_filter = flag;
    }

/// Whether this is a resource filter.
    ///
    /// Corresponds to Java: `public boolean isResourceFilter()`
    pub fn is_resource_filter(&self) -> bool {
        self.m_is_resource_filter
    }

/// Set the resource filter flag.
    ///
    /// Corresponds to Java: `public void setIsResourceFilter(boolean flag)`
    pub fn set_is_resource_filter(&mut self, flag: bool) {
        self.m_is_resource_filter = flag;
    }

/// Evaluate: returns `true` when the container's values satisfy the filter criteria.
    ///
    /// Corresponds to Java: `public boolean evaluate(FieldContainer container, Map<GenericCriteriaPrompt, Object> promptValues)`
    ///
/// With no criteria it is always `true`. When the criteria are not satisfied but "show related summary rows" is enabled and the container is a `Task`,
/// its child tasks are tested recursively, and `true` is returned if any child matches.
    pub fn evaluate<C>(&self, container: &C, prompt_values: &PromptValues) -> bool
    where
        C: FieldContainer + 'static,
    {
        let mut result = true;

        if let Some(criteria) = &self.m_criteria {
            result = criteria.evaluate(container, prompt_values);

            //
// This row failed, but if it is a summary row and related summary rows are required, test its child tasks recursively
            //
            if !result && self.m_show_related_summary_rows {
                let any: &dyn Any = container;
                if let Some(task) = any.downcast_ref::<Task>() {
                    for child in task.get_child_tasks() {
                        let matched = match child.try_borrow() {
                            Ok(child) => self.evaluate(&*child, prompt_values),
// Impossible in Java; avoids a re-entrant borrow panic
                            Err(_) => false,
                        };

                        if matched {
                            result = true;
                            break;
                        }
                    }
                }
            }
        }

        result
    }

/// Set the prompts required by this filter.
    ///
    /// Corresponds to Java: `public void setPrompts(List<GenericCriteriaPrompt> prompts)`
    pub fn set_prompts(&mut self, prompts: Vec<GenericCriteriaPrompt>) {
        self.m_prompts = prompts;
    }

/// Get the prompts required by this filter.
    ///
    /// Corresponds to Java: `public List<GenericCriteriaPrompt> getPrompts()`
    ///
/// Java defaults to `null`; the Rust side returns an empty slice.
    pub fn get_prompts(&self) -> &[GenericCriteriaPrompt] {
        &self.m_prompts
    }
}

impl Default for Filter {
/// Corresponds to Java: the implicit default constructor, equivalent to [`Filter::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl Display for Filter {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Outputs the `criteria=...` section only when criteria are present.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut result = format!(
            "[Filter id={} name={} showRelatedSummaryRows={}",
            self.m_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "null".to_owned()),
            self.m_name.as_deref().unwrap_or("null"),
            self.m_show_related_summary_rows
        );

        if let Some(criteria) = &self.m_criteria {
            result.push_str(" criteria=");
            result.push_str(&criteria.to_string());
        }

        result.push(']');
        f.write_str(&result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_key::FieldKey;
    use crate::field_value::FieldValue;
    use crate::listener::field_listener::FieldListener;
    use crate::task::Task;
    use crate::task_field::TaskField;
    use crate::test_operator::{Operand, TestOperator};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::{Rc, Weak};

/// Field container for tests (not a `Task`, corresponding to the path where `instanceof Task` is false in Java).
    #[derive(Default)]
    struct MockContainer {
        values: HashMap<FieldKey, FieldValue>,
    }

    impl FieldContainer for MockContainer {
        fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
            match value {
                Some(value) => self.values.insert(field, value),
                None => self.values.remove(&field),
            };
        }

        fn get(&self, field: FieldKey) -> Option<FieldValue> {
            self.values.get(&field).cloned()
        }

        fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
            self.values.get(&field)
        }

        fn add_field_listener(&mut self, _listener: Box<dyn FieldListener>) {}

        fn clear_field_listeners(&mut self) {}
    }

/// Build a "Name equals value" criterion.
    fn criteria(value: &str) -> GenericCriteria {
        let mut criteria = GenericCriteria::new(Weak::new());
        criteria.set_left_value(Some(FieldKey::Task(TaskField::Name)));
        criteria.set_operator(Some(TestOperator::Equals));
        criteria.set_right_value(0, Some(Operand::Value(FieldValue::String(value.to_owned()))));
        criteria
    }

    #[test]
    fn accessors_round_trip() {
        let mut filter = Filter::new();

        assert_eq!(filter.get_id(), None);
        assert_eq!(filter.get_name(), None);
        assert!(!filter.is_task_filter());
        assert!(!filter.is_resource_filter());
        assert!(!filter.get_show_related_summary_rows());
        assert!(filter.get_criteria().is_none());
        assert!(filter.get_prompts().is_empty());

        filter.set_id(Some(3));
        filter.set_name(Some("My Filter".to_owned()));
        filter.set_is_task_filter(true);
        filter.set_is_resource_filter(true);
        filter.set_show_related_summary_rows(true);
        filter.set_criteria(Some(criteria("A")));
        filter.set_prompts(vec![GenericCriteriaPrompt::new(
            crate::data_type::DataType::String,
            Some("Enter".to_owned()),
        )]);

        assert_eq!(filter.get_id(), Some(3));
        assert_eq!(filter.get_name(), Some("My Filter"));
        assert!(filter.is_task_filter());
        assert!(filter.is_resource_filter());
        assert!(filter.get_show_related_summary_rows());
        assert!(filter.get_criteria().is_some());
        assert_eq!(filter.get_prompts().len(), 1);
    }

    #[test]
    fn evaluate_without_criteria_is_true() {
        let filter = Filter::new();
        let container = MockContainer::default();

        assert!(filter.evaluate(&container, &PromptValues::new()));
    }

    #[test]
    fn evaluate_uses_criteria() {
        let mut filter = Filter::new();
        filter.set_criteria(Some(criteria("A")));

        let mut container = MockContainer::default();
        container.set(
            FieldKey::Task(TaskField::Name),
            Some(FieldValue::String("A".to_owned())),
        );
        assert!(filter.evaluate(&container, &PromptValues::new()));

        container.set(
            FieldKey::Task(TaskField::Name),
            Some(FieldValue::String("B".to_owned())),
        );
        assert!(!filter.evaluate(&container, &PromptValues::new()));
    }

    #[test]
    fn related_summary_rows_are_tested_recursively_for_tasks() {
        let parent = Rc::new(RefCell::new(Task::new()));
        parent.borrow_mut().set_name(Some("Summary".to_owned()));

        let child = Rc::new(RefCell::new(Task::new()));
        child.borrow_mut().set_name(Some("Design".to_owned()));
        Task::add_child_task(&parent, &child);

        let mut filter = Filter::new();
        filter.set_criteria(Some(criteria("Design")));

// "Show related summary rows" is off ⇒ the summary row itself not matching means failure
        assert!(!filter.evaluate(&*parent.borrow(), &PromptValues::new()));

// When enabled, test child tasks recursively
        filter.set_show_related_summary_rows(true);
        assert!(filter.evaluate(&*parent.borrow(), &PromptValues::new()));

// Still fails when no child task matches either
        filter.set_criteria(Some(criteria("Build")));
        assert!(!filter.evaluate(&*parent.borrow(), &PromptValues::new()));
    }

    #[test]
    fn related_summary_rows_do_not_recurse_for_non_tasks() {
        let mut filter = Filter::new();
        filter.set_criteria(Some(criteria("A")));
        filter.set_show_related_summary_rows(true);

        let container = MockContainer::default();
        assert!(!filter.evaluate(&container, &PromptValues::new()));
    }

    #[test]
    fn display_matches_java_format() {
        let mut filter = Filter::new();
        assert_eq!(
            filter.to_string(),
            "[Filter id=null name=null showRelatedSummaryRows=false]"
        );

        filter.set_id(Some(1));
        filter.set_name(Some("F".to_owned()));
        filter.set_show_related_summary_rows(true);
        filter.set_criteria(Some(criteria("A")));

        assert_eq!(
            filter.to_string(),
            "[Filter id=1 name=F showRelatedSummaryRows=true criteria=(Task Name EQUALS A)]"
        );
    }

    #[test]
    fn default_matches_new() {
        let filter = Filter::default();
        assert!(filter.get_criteria().is_none());
    }
}
