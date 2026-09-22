//! Corresponds to Java: `org/mpxj/GraphicalIndicator.java`
//!
//! Skipped / unported Java methods and the reasons:
//!
//! | Java | Rust |
//! |---|---|
//! | `GraphicalIndicator(FieldType fieldType)` | [`GraphicalIndicator::new`] (package-visible on the Java side; must be public on the Rust side to be constructible, see below) |
//! | `public int evaluate(FieldContainer container)` | [`GraphicalIndicator::evaluate`] (changed to generics + `Any` downcasting, see below) |
//! | `public FieldType getFieldType()` | [`GraphicalIndicator::get_field_type`](`FieldType` → [`FieldKey`]) |
//! | `public boolean getDisplayGraphicalIndicators()` / `public void setDisplayGraphicalIndicators(boolean)` | [`GraphicalIndicator::get_display_graphical_indicators`] / [`GraphicalIndicator::set_display_graphical_indicators`] |
//! | `public List<GraphicalIndicatorCriteria> getNonSummaryRowCriteria()` | [`GraphicalIndicator::get_non_summary_row_criteria`] |
//! | `public List<GraphicalIndicatorCriteria> getProjectSummaryCriteria()` | [`GraphicalIndicator::get_project_summary_criteria`] |
//! | `public List<GraphicalIndicatorCriteria> getSummaryRowCriteria()` | [`GraphicalIndicator::get_summary_row_criteria`] |
//! | `public boolean getProjectSummaryInheritsFromSummaryRows()` / setter | [`GraphicalIndicator::get_project_summary_inherits_from_summary_rows`] / [`GraphicalIndicator::set_project_summary_inherits_from_summary_rows`] |
//! | `public boolean getSummaryRowsInheritFromNonSummaryRows()` / setter | [`GraphicalIndicator::get_summary_rows_inherit_from_non_summary_rows`] / [`GraphicalIndicator::set_summary_rows_inherit_from_non_summary_rows`] |
//! | `public boolean getShowDataValuesInToolTips()` / setter | [`GraphicalIndicator::get_show_data_values_in_tool_tips`] / [`GraphicalIndicator::set_show_data_values_in_tool_tips`] |
//! | `public void addNonSummaryRowCriteria(GraphicalIndicatorCriteria)` | [`GraphicalIndicator::add_non_summary_row_criteria`] |
//! | `public void addSummaryRowCriteria(GraphicalIndicatorCriteria)` | [`GraphicalIndicator::add_summary_row_criteria`] |
//! | `public void addProjectSummaryCriteria(GraphicalIndicatorCriteria)` | [`GraphicalIndicator::add_project_summary_criteria`] |
//! | `@Override public String toString()` | [`Display`] |
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: not used by this class, nothing skipped.
//! Also, the `NumberHelper.getInt` used on the Java side is expressed directly on the Rust side as
//! [`Option::unwrap_or`] (`null` → 0), with identical behavior.
//!
//! # Differences from Java
//!
//! 1. **`instanceof Task` → `Any` downcasting**: Java's `evaluate` uses
//! `container instanceof Task` to decide which set of criteria to use (project summary row / summary row / non-summary row),
//! and uses `Task.getUniqueID()` / `Task.getSummary()` to determine the row type. Rust trait objects cannot
//! be downcast directly, so [`GraphicalIndicator::evaluate`] is changed to a **generic** that receives
//! `&C` (`C: FieldContainer + 'static`), internally converting to `&dyn Any` first and then
//! `downcast_ref::<Task>()`, with semantics identical to Java; non-`Task` containers always use the non-summary row criteria
//! (matching Java's `else` branch).
//! 2. **Constructor visibility**: Java's constructor is package-visible (in practice only called by `CustomField`); this crate's
//! [`crate::custom_field::CustomField`] is in the `crate::custom_field` module, while this type is in
//! `crate::graphical_indicator`; `pub(crate)` / private cannot express the "constructible only within the package" visibility,
//! so the Rust side makes the constructor `pub` ([`crate::custom_field::CustomField::new`] also uses
//! it to create `m_indicator`).
//! 3. **How the criteria are held**: Java's three `List`s are separate fields; the Rust side likewise uses `Vec`s, and following this crate's
//! convention splits them into read-only slice accessors ([`GraphicalIndicator::get_non_summary_row_criteria`] etc.)
//! and `add_*` append methods. Java's `getNonSummaryRowCriteria()` returns the mutable list directly;
//! the Rust side does not provide an extra `_mut` (MPXJ only appends with `add_*`).

use std::any::Any;
use std::fmt::{self, Display, Formatter};

use crate::field_container::FieldContainer;
use crate::field_key::FieldKey;
use crate::graphical_indicator_criteria::GraphicalIndicatorCriteria;
use crate::task::Task;

/// Definition of the graphical indicator for a single table column.
///
/// Corresponds to Java: `public class GraphicalIndicator`
pub struct GraphicalIndicator {
/// The field this indicator applies to.
    ///
    /// Corresponds to Java: `private final FieldType m_fieldType;`
    m_field_type: FieldKey,

/// Whether to display the graphical indicator instead of the actual value.
    ///
    /// Corresponds to Java: `private boolean m_displayGraphicalIndicators;`
    m_display_graphical_indicators: bool,

/// Whether summary rows inherit the non-summary row criteria.
    ///
    /// Corresponds to Java: `private boolean m_summaryRowsInheritFromNonSummaryRows;`
    m_summary_rows_inherit_from_non_summary_rows: bool,

/// Whether project summary rows inherit the summary row criteria.
    ///
    /// Corresponds to Java: `private boolean m_projectSummaryInheritsFromSummaryRows;`
    m_project_summary_inherits_from_summary_rows: bool,

/// Whether to display the data value as a tooltip.
    ///
    /// Corresponds to Java: `private boolean m_showDataValuesInToolTips;`
    m_show_data_values_in_tool_tips: bool,

/// Non-summary row criteria.
    ///
    /// Corresponds to Java: `private final List<GraphicalIndicatorCriteria> m_nonSummaryRowCriteria = new ArrayList<>();`
    m_non_summary_row_criteria: Vec<GraphicalIndicatorCriteria>,

/// Summary row criteria.
    ///
    /// Corresponds to Java: `private final List<GraphicalIndicatorCriteria> m_summaryRowCriteria = new ArrayList<>();`
    m_summary_row_criteria: Vec<GraphicalIndicatorCriteria>,

/// Project summary row criteria.
    ///
    /// Corresponds to Java: `private final List<GraphicalIndicatorCriteria> m_projectSummaryCriteria = new ArrayList<>();`
    m_project_summary_criteria: Vec<GraphicalIndicatorCriteria>,
}

impl GraphicalIndicator {
/// Constructor.
    ///
    /// Corresponds to Java: `GraphicalIndicator(FieldType fieldType)`
    ///
/// Package-visible on the Java side; see the file header note for the Rust side (changed to `pub`).
    pub fn new(field_type: FieldKey) -> Self {
        Self {
            m_field_type: field_type,
            m_display_graphical_indicators: false,
            m_summary_rows_inherit_from_non_summary_rows: false,
            m_project_summary_inherits_from_summary_rows: false,
            m_show_data_values_in_tool_tips: false,
            m_non_summary_row_criteria: Vec::new(),
            m_summary_row_criteria: Vec::new(),
            m_project_summary_criteria: Vec::new(),
        }
    }

/// Determine whether a graphical indicator should be displayed; returning `-1` means no indicator is displayed.
    ///
    /// Corresponds to Java: `public int evaluate(FieldContainer container)`
    ///
/// Return the default value `0` when no criteria match.
    pub fn evaluate<C>(&self, container: &C) -> i32
    where
        C: FieldContainer + 'static,
    {
        //
// Step one: determine which criteria set to use this time
        //
        let any: &dyn Any = container;
        let criteria: &[GraphicalIndicatorCriteria] = match any.downcast_ref::<Task>() {
            Some(task) => {
                if task.get_unique_id().unwrap_or(0) == 0 {
                    if !self.m_project_summary_inherits_from_summary_rows {
                        &self.m_project_summary_criteria
                    } else if !self.m_summary_rows_inherit_from_non_summary_rows {
                        &self.m_summary_row_criteria
                    } else {
                        &self.m_non_summary_row_criteria
                    }
                } else if task.get_summary() {
                    if !self.m_summary_rows_inherit_from_non_summary_rows {
                        &self.m_summary_row_criteria
                    } else {
                        &self.m_non_summary_row_criteria
                    }
                } else {
                    &self.m_non_summary_row_criteria
                }
            }
// Resource summary rows may also exist, but cannot be determined here, so use the non-summary row criteria as Java does
            None => &self.m_non_summary_row_criteria,
        };

        //
// Step two: evaluate one by one, taking the result of the first match
        //
        let mut result = -1;
        for item in criteria {
            result = item.evaluate_indicator(container);
            if result != -1 {
                break;
            }
        }

        //
// Step three: if still no match, return the default value 0
        //
        if result == -1 {
            result = 0;
        }

        result
    }

/// Get the field this indicator applies to.
    ///
    /// Corresponds to Java: `public FieldType getFieldType()`
    pub fn get_field_type(&self) -> FieldKey {
        self.m_field_type
    }

/// Get the "display graphical indicator" flag.
    ///
    /// Corresponds to Java: `public boolean getDisplayGraphicalIndicators()`
    pub fn get_display_graphical_indicators(&self) -> bool {
        self.m_display_graphical_indicators
    }

/// Set the "display graphical indicator" flag.
    ///
    /// Corresponds to Java: `public void setDisplayGraphicalIndicators(boolean displayGraphicalIndicators)`
    pub fn set_display_graphical_indicators(&mut self, display_graphical_indicators: bool) {
        self.m_display_graphical_indicators = display_graphical_indicators;
    }

/// Get the non-summary row criteria.
    ///
    /// Corresponds to Java: `public List<GraphicalIndicatorCriteria> getNonSummaryRowCriteria()`
    pub fn get_non_summary_row_criteria(&self) -> &[GraphicalIndicatorCriteria] {
        &self.m_non_summary_row_criteria
    }

/// Get the project summary row criteria.
    ///
    /// Corresponds to Java: `public List<GraphicalIndicatorCriteria> getProjectSummaryCriteria()`
    pub fn get_project_summary_criteria(&self) -> &[GraphicalIndicatorCriteria] {
        &self.m_project_summary_criteria
    }

/// Get the summary row criteria.
    ///
    /// Corresponds to Java: `public List<GraphicalIndicatorCriteria> getSummaryRowCriteria()`
    pub fn get_summary_row_criteria(&self) -> &[GraphicalIndicatorCriteria] {
        &self.m_summary_row_criteria
    }

/// Get the "project summary rows inherit summary row criteria" flag.
    ///
    /// Corresponds to Java: `public boolean getProjectSummaryInheritsFromSummaryRows()`
    pub fn get_project_summary_inherits_from_summary_rows(&self) -> bool {
        self.m_project_summary_inherits_from_summary_rows
    }

/// Set the "project summary rows inherit summary row criteria" flag.
    ///
    /// Corresponds to Java: `public void setProjectSummaryInheritsFromSummaryRows(boolean projectSummaryInheritsFromSummaryRows)`
    pub fn set_project_summary_inherits_from_summary_rows(
        &mut self,
        project_summary_inherits_from_summary_rows: bool,
    ) {
        self.m_project_summary_inherits_from_summary_rows =
            project_summary_inherits_from_summary_rows;
    }

/// Get the "summary rows inherit non-summary row criteria" flag.
    ///
    /// Corresponds to Java: `public boolean getSummaryRowsInheritFromNonSummaryRows()`
    pub fn get_summary_rows_inherit_from_non_summary_rows(&self) -> bool {
        self.m_summary_rows_inherit_from_non_summary_rows
    }

/// Set the "summary rows inherit non-summary row criteria" flag.
    ///
    /// Corresponds to Java: `public void setSummaryRowsInheritFromNonSummaryRows(boolean summaryRowsInheritFromNonSummaryRows)`
    pub fn set_summary_rows_inherit_from_non_summary_rows(
        &mut self,
        summary_rows_inherit_from_non_summary_rows: bool,
    ) {
        self.m_summary_rows_inherit_from_non_summary_rows =
            summary_rows_inherit_from_non_summary_rows;
    }

/// Get the "display data value as tooltip" flag.
    ///
    /// Corresponds to Java: `public boolean getShowDataValuesInToolTips()`
    pub fn get_show_data_values_in_tool_tips(&self) -> bool {
        self.m_show_data_values_in_tool_tips
    }

/// Set the "display data value as tooltip" flag.
    ///
    /// Corresponds to Java: `public void setShowDataValuesInToolTips(boolean showDataValuesInToolTips)`
    pub fn set_show_data_values_in_tool_tips(&mut self, show_data_values_in_tool_tips: bool) {
        self.m_show_data_values_in_tool_tips = show_data_values_in_tool_tips;
    }

/// Append non-summary row criteria.
    ///
    /// Corresponds to Java: `public void addNonSummaryRowCriteria(GraphicalIndicatorCriteria criteria)`
    pub fn add_non_summary_row_criteria(&mut self, criteria: GraphicalIndicatorCriteria) {
        self.m_non_summary_row_criteria.push(criteria);
    }

/// Append summary row criteria.
    ///
    /// Corresponds to Java: `public void addSummaryRowCriteria(GraphicalIndicatorCriteria criteria)`
    pub fn add_summary_row_criteria(&mut self, criteria: GraphicalIndicatorCriteria) {
        self.m_summary_row_criteria.push(criteria);
    }

/// Append project summary row criteria.
    ///
    /// Corresponds to Java: `public void addProjectSummaryCriteria(GraphicalIndicatorCriteria criteria)`
    pub fn add_project_summary_criteria(&mut self, criteria: GraphicalIndicatorCriteria) {
        self.m_project_summary_criteria.push(criteria);
    }
}

impl Display for GraphicalIndicator {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Output line by line as Java does (Java uses `pw.println`, so the output includes a trailing newline).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut result = String::new();

        result.push_str("[GraphicalIndicator\n");
        result.push_str(&format!(" FieldType={}\n", self.m_field_type));
        result.push_str(&format!(
            " DisplayGraphicalIndicators={}\n",
            self.m_display_graphical_indicators
        ));
        result.push_str(&format!(
            " SummaryRowsInheritFromNonSummaryRows={}\n",
            self.m_summary_rows_inherit_from_non_summary_rows
        ));
        result.push_str(&format!(
            " ProjectSummaryInheritsFromSummaryRows={}\n",
            self.m_project_summary_inherits_from_summary_rows
        ));
        result.push_str(&format!(
            " ShowDataValuesInToolTips={}\n",
            self.m_show_data_values_in_tool_tips
        ));

        result.push_str(" NonSummaryRowCriteria=\n");
        for criteria in &self.m_non_summary_row_criteria {
            result.push_str(&format!("  {criteria}\n"));
        }

        result.push_str(" SummaryRowCriteria=\n");
        for criteria in &self.m_summary_row_criteria {
            result.push_str(&format!("  {criteria}\n"));
        }

        result.push_str(" ProjectSummaryCriteria=\n");
        for criteria in &self.m_project_summary_criteria {
            result.push_str(&format!("  {criteria}\n"));
        }

        result.push_str("]\n");
        f.write_str(&result)
    }
}

impl fmt::Debug for GraphicalIndicator {
/// Java does not override debug output; the Rust side provides `Debug` (reusing [`Display`]).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_value::FieldValue;
    use crate::listener::field_listener::FieldListener;
    use crate::task_field::TaskField;
    use crate::test_operator::{Operand, TestOperator};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

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

/// Construct a criterion "display indicator number N when Name equals value".
    fn criteria(indicator: i32, value: &str) -> GraphicalIndicatorCriteria {
        let mut criteria = GraphicalIndicatorCriteria::new(std::rc::Weak::new());
        criteria.set_indicator(indicator);
        criteria.set_left_value(Some(FieldKey::Task(TaskField::Name)));
        criteria.set_operator(Some(TestOperator::Equals));
        criteria.set_right_value(0, Some(Operand::Value(FieldValue::String(value.to_owned()))));
        criteria
    }

    #[test]
    fn accessors_round_trip() {
        let mut indicator = GraphicalIndicator::new(FieldKey::Task(TaskField::Name));

        assert_eq!(indicator.get_field_type(), FieldKey::Task(TaskField::Name));
        assert!(!indicator.get_display_graphical_indicators());
        assert!(!indicator.get_summary_rows_inherit_from_non_summary_rows());
        assert!(!indicator.get_project_summary_inherits_from_summary_rows());
        assert!(!indicator.get_show_data_values_in_tool_tips());
        assert!(indicator.get_non_summary_row_criteria().is_empty());
        assert!(indicator.get_summary_row_criteria().is_empty());
        assert!(indicator.get_project_summary_criteria().is_empty());

        indicator.set_display_graphical_indicators(true);
        indicator.set_summary_rows_inherit_from_non_summary_rows(true);
        indicator.set_project_summary_inherits_from_summary_rows(true);
        indicator.set_show_data_values_in_tool_tips(true);
        indicator.add_non_summary_row_criteria(criteria(1, "A"));
        indicator.add_summary_row_criteria(criteria(2, "A"));
        indicator.add_project_summary_criteria(criteria(3, "A"));

        assert!(indicator.get_display_graphical_indicators());
        assert!(indicator.get_summary_rows_inherit_from_non_summary_rows());
        assert!(indicator.get_project_summary_inherits_from_summary_rows());
        assert!(indicator.get_show_data_values_in_tool_tips());
        assert_eq!(indicator.get_non_summary_row_criteria().len(), 1);
        assert_eq!(indicator.get_summary_row_criteria().len(), 1);
        assert_eq!(indicator.get_project_summary_criteria().len(), 1);
    }

    #[test]
    fn non_task_container_uses_non_summary_criteria() {
        let mut indicator = GraphicalIndicator::new(FieldKey::Task(TaskField::Name));
        indicator.add_non_summary_row_criteria(criteria(7, "A"));
        indicator.add_summary_row_criteria(criteria(9, "A"));

        let mut container = MockContainer::default();
        container.set(
            FieldKey::Task(TaskField::Name),
            Some(FieldValue::String("A".to_owned())),
        );

        assert_eq!(indicator.evaluate(&container), 7);
    }

    #[test]
    fn unmatched_criteria_fall_back_to_zero() {
        let mut indicator = GraphicalIndicator::new(FieldKey::Task(TaskField::Name));
        indicator.add_non_summary_row_criteria(criteria(7, "A"));

        let mut container = MockContainer::default();
        container.set(
            FieldKey::Task(TaskField::Name),
            Some(FieldValue::String("B".to_owned())),
        );

// Java: result = 0 when nothing matches
        assert_eq!(indicator.evaluate(&container), 0);
    }

    #[test]
    fn no_criteria_at_all_falls_back_to_zero() {
        let indicator = GraphicalIndicator::new(FieldKey::Task(TaskField::Name));
        let container = MockContainer::default();

        assert_eq!(indicator.evaluate(&container), 0);
    }

    #[test]
    fn task_rows_select_criteria_by_row_type() {
        let mut indicator = GraphicalIndicator::new(FieldKey::Task(TaskField::Name));
        indicator.add_non_summary_row_criteria(criteria(1, "A"));
        indicator.add_summary_row_criteria(criteria(2, "A"));
        indicator.add_project_summary_criteria(criteria(3, "A"));

        let task = Rc::new(RefCell::new(Task::new()));
        task.borrow_mut().set_name(Some("A".to_owned()));

// unique ID is 0 (unset) ⇒ project summary row
        assert_eq!(indicator.evaluate(&*task.borrow()), 3);

// ordinary row ⇒ non-summary row criteria
        task.borrow_mut().set_unique_id(Some(1));
        assert_eq!(indicator.evaluate(&*task.borrow()), 1);

// summary row ⇒ summary row criteria (does not inherit non-summary row criteria by default)
        task.borrow_mut().set_summary(true);
        assert_eq!(indicator.evaluate(&*task.borrow()), 2);

// after enabling inheritance, the summary row uses the non-summary row criteria
        indicator.set_summary_rows_inherit_from_non_summary_rows(true);
        assert_eq!(indicator.evaluate(&*task.borrow()), 1);
    }

    #[test]
    fn project_summary_inheritance_chain() {
        let mut indicator = GraphicalIndicator::new(FieldKey::Task(TaskField::Name));
        indicator.add_non_summary_row_criteria(criteria(1, "A"));
        indicator.add_summary_row_criteria(criteria(2, "A"));
        indicator.add_project_summary_criteria(criteria(3, "A"));

        let task = Rc::new(RefCell::new(Task::new()));
        task.borrow_mut().set_name(Some("A".to_owned()));

// project summary row inherits the summary row (and the summary row does not inherit the non-summary row) ⇒ summary row criteria
        indicator.set_project_summary_inherits_from_summary_rows(true);
        assert_eq!(indicator.evaluate(&*task.borrow()), 2);

// both levels of inheritance enabled ⇒ non-summary row criteria
        indicator.set_summary_rows_inherit_from_non_summary_rows(true);
        assert_eq!(indicator.evaluate(&*task.borrow()), 1);
    }

    #[test]
    fn display_matches_java_format() {
        let mut indicator = GraphicalIndicator::new(FieldKey::Task(TaskField::Name));
        indicator.add_non_summary_row_criteria(criteria(1, "A"));

        let expected = concat!(
            "[GraphicalIndicator\n",
            " FieldType=Task Name\n",
            " DisplayGraphicalIndicators=false\n",
            " SummaryRowsInheritFromNonSummaryRows=false\n",
            " ProjectSummaryInheritsFromSummaryRows=false\n",
            " ShowDataValuesInToolTips=false\n",
            " NonSummaryRowCriteria=\n",
            "  [GraphicalIndicatorCriteria indicator=1 criteria=(Task Name EQUALS A)]\n",
            " SummaryRowCriteria=\n",
            " ProjectSummaryCriteria=\n",
            "]\n",
        );

        assert_eq!(indicator.to_string(), expected);
    }
}
