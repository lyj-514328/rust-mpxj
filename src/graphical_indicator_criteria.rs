//! Corresponds to Java: `org/mpxj/GraphicalIndicatorCriteria.java`
//!
//! Skipped/unported Java methods and the reasons:
//!
//! | Java | Rust |
//! |---|---|
//! | `public GraphicalIndicatorCriteria(ProjectProperties properties)` | [`GraphicalIndicatorCriteria::new`] (the Rust side accepts `Weak<RefCell<ProjectFile>>`, see below) |
//! | `public int getIndicator()` / `public void setIndicator(int indicator)` | [`GraphicalIndicatorCriteria::get_indicator`] / [`GraphicalIndicatorCriteria::set_indicator`] |
//! | `public int evaluate(FieldContainer container)` | [`GraphicalIndicatorCriteria::evaluate_indicator`] (Rust has no overloading; a suffix is added to distinguish it from the inherited 2-parameter `evaluate`) |
//! | `@Override public String toString()` | [`Display`] |
//! | `setLeftValue` / `getLeftValue` / `getOperator` / `setOperator` / `setRightValue` / `getValue` / `evaluate(FieldContainer, Map)` / `getCriteriaList` / `addCriteria` inherited from `GenericCriteria` | [`GraphicalIndicatorCriteria::set_left_value`] / [`GraphicalIndicatorCriteria::get_left_value`] / [`GraphicalIndicatorCriteria::get_operator`] / [`GraphicalIndicatorCriteria::set_operator`] / [`GraphicalIndicatorCriteria::set_right_value`] / [`GraphicalIndicatorCriteria::get_value`] / [`GraphicalIndicatorCriteria::evaluate`] / [`GraphicalIndicatorCriteria::get_criteria_list`] / [`GraphicalIndicatorCriteria::get_criteria_list_mut`] / [`GraphicalIndicatorCriteria::add_criteria`] |
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: not used by this class, nothing skipped.
//!
//! # How the inheritance relationship is reproduced: composition + forwarding
//!
//! Java's `GraphicalIndicatorCriteria extends GenericCriteria`. Rust has no inheritance, so this class instead
//! **composes** a [`GenericCriteria`] (field `criteria`) and forwards the base class's public methods one by one,
//! while the additional `m_indicator` is a field of this class itself. Therefore this class is **not** a `GenericCriteria`
//! and cannot be added as a sub-criterion to another `GenericCriteria`'s `criteriaList` (MPXJ's own
//! `CriteriaReader` only adds ordinary `GenericCriteria`, so it is unaffected).
//!
//! # Differences from Java
//!
//! - **Constructor parameter**: Java takes `ProjectProperties`; the Rust side takes a weak reference to the parent project (consistent with
//! [`GenericCriteria::new`]; see that type's file header).
//! - **`evaluate(FieldContainer)` passing a `null` prompt response**: Java passes `null` explicitly; the Rust side
//! passes an empty map (equivalent to "no prompt response can be found"; see the note on [`GenericCriteria::evaluate`]).

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::Weak;

use crate::field_container::FieldContainer;
use crate::field_key::FieldKey;
use crate::generic_criteria::{GenericCriteria, PromptValues};
use crate::project_file::ProjectFile;
use crate::test_operator::{Operand, TestOperator};

/// A criterion that determines whether to display a graphical indicator instead of the field value.
///
/// Corresponds to Java: `public final class GraphicalIndicatorCriteria extends GenericCriteria`
pub struct GraphicalIndicatorCriteria {
/// The composed base class instance (corresponds to Java's `extends GenericCriteria`).
    criteria: GenericCriteria,

/// The indicator number to display when the criterion is met.
    ///
    /// Corresponds to Java: `private int m_indicator;`
    m_indicator: i32,
}

impl GraphicalIndicatorCriteria {
/// Constructor.
    ///
    /// Corresponds to Java: `public GraphicalIndicatorCriteria(ProjectProperties properties)`
    pub fn new(project: Weak<RefCell<ProjectFile>>) -> Self {
        Self {
            criteria: GenericCriteria::new(project),
            m_indicator: 0,
        }
    }

/// Get the indicator number.
    ///
    /// Corresponds to Java: `public int getIndicator()`
    pub fn get_indicator(&self) -> i32 {
        self.m_indicator
    }

/// Set the indicator number.
    ///
    /// Corresponds to Java: `public void setIndicator(int indicator)`
    pub fn set_indicator(&mut self, indicator: i32) {
        self.m_indicator = indicator;
    }

/// Evaluate: returns the indicator number when the criterion is met, otherwise returns `-1`.
    ///
    /// Corresponds to Java: `public int evaluate(FieldContainer container)`
    ///
/// Rust has no overloading, so the `_indicator` suffix is added to distinguish it from the inherited
    /// `evaluate(FieldContainer, promptValues)`.
    pub fn evaluate_indicator(&self, container: &dyn FieldContainer) -> i32 {
        // Java: return (evaluate(container, null) ? m_indicator : -1);
        if self.criteria.evaluate(container, &PromptValues::new()) {
            self.m_indicator
        } else {
            -1
        }
    }

/// Set the left-hand side of the expression (a field).
    ///
    /// Corresponds to Java: `GenericCriteria.setLeftValue(FieldType value)`
    pub fn set_left_value(&mut self, value: Option<FieldKey>) {
        self.criteria.set_left_value(value);
    }

/// Get the left-hand side of the expression (a field).
    ///
    /// Corresponds to Java: `GenericCriteria.getLeftValue()`
    pub fn get_left_value(&self) -> Option<FieldKey> {
        self.criteria.get_left_value()
    }

/// Get the test operator.
    ///
    /// Corresponds to Java: `GenericCriteria.getOperator()`
    pub fn get_operator(&self) -> Option<TestOperator> {
        self.criteria.get_operator()
    }

/// Set the test operator.
    ///
    /// Corresponds to Java: `GenericCriteria.setOperator(TestOperator operator)`
    pub fn set_operator(&mut self, operator: Option<TestOperator>) {
        self.criteria.set_operator(operator);
    }

/// Set the right-hand value at the specified index.
    ///
    /// Corresponds to Java: `GenericCriteria.setRightValue(int index, Object value)`
    pub fn set_right_value(&mut self, index: usize, value: Option<Operand>) {
        self.criteria.set_right_value(index, value);
    }

/// Get the raw right-hand value at the specified index.
    ///
    /// Corresponds to Java: `GenericCriteria.getValue(int index)`
    pub fn get_value(&self, index: usize) -> Option<&Operand> {
        self.criteria.get_value(index)
    }

/// Evaluate and return a boolean result (inherited from `GenericCriteria`).
    ///
    /// Corresponds to Java: `GenericCriteria.evaluate(FieldContainer container, Map<GenericCriteriaPrompt, Object> promptValues)`
    pub fn evaluate(
        &self,
        container: &dyn FieldContainer,
        prompt_values: &PromptValues,
    ) -> bool {
        self.criteria.evaluate(container, prompt_values)
    }

/// Get the list of sub-criteria.
    ///
    /// Corresponds to Java: `GenericCriteria.getCriteriaList()`
    pub fn get_criteria_list(&self) -> &[GenericCriteria] {
        self.criteria.get_criteria_list()
    }

/// Get the list of sub-criteria mutably.
    ///
    /// Corresponds to Java: `GenericCriteria.getCriteriaList()`
    pub fn get_criteria_list_mut(&mut self) -> &mut Vec<GenericCriteria> {
        self.criteria.get_criteria_list_mut()
    }

/// Append a sub-criterion.
    ///
    /// Corresponds to Java: `GenericCriteria.addCriteria(GenericCriteria criteria)`
    pub fn add_criteria(&mut self, criteria: GenericCriteria) {
        self.criteria.add_criteria(criteria);
    }
}

impl Display for GraphicalIndicatorCriteria {
    /// Corresponds to Java: `@Override public String toString()`
    ///
    /// Java: `"[GraphicalIndicatorCriteria indicator=" + m_indicator + " criteria=" + super.toString() + "]"`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[GraphicalIndicatorCriteria indicator={} criteria={}]",
            self.m_indicator, self.criteria
        )
    }
}

impl fmt::Debug for GraphicalIndicatorCriteria {
/// Java does not override the debug output; the Rust side provides `Debug` (reusing [`Display`]).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_value::FieldValue;
    use crate::generic_criteria_prompt::GenericCriteriaPrompt;
    use crate::listener::field_listener::FieldListener;
    use crate::task_field::TaskField;
    use std::collections::HashMap;

/// Field container for tests: stores only the fields visible to `getCachedValue`.
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

    fn container_with(name: &str) -> MockContainer {
        let mut container = MockContainer::default();
        container.set(
            FieldKey::Task(TaskField::Name),
            Some(FieldValue::String(name.to_owned())),
        );
        container
    }

    #[test]
    fn accessors_round_trip() {
        let mut criteria = GraphicalIndicatorCriteria::new(Weak::new());
        assert_eq!(criteria.get_indicator(), 0);
        assert_eq!(criteria.get_left_value(), None);
        assert_eq!(criteria.get_operator(), None);
        assert_eq!(criteria.get_value(0), None);
        assert!(criteria.get_criteria_list().is_empty());

        criteria.set_indicator(3);
        criteria.set_left_value(Some(FieldKey::Task(TaskField::Name)));
        criteria.set_operator(Some(TestOperator::Equals));
        criteria.set_right_value(0, Some(Operand::Value(FieldValue::String("A".to_owned()))));

        assert_eq!(criteria.get_indicator(), 3);
        assert_eq!(
            criteria.get_left_value(),
            Some(FieldKey::Task(TaskField::Name))
        );
        assert_eq!(criteria.get_operator(), Some(TestOperator::Equals));
        assert_eq!(
            criteria.get_value(0),
            Some(&Operand::Value(FieldValue::String("A".to_owned())))
        );
    }

    #[test]
    fn evaluate_indicator_returns_indicator_or_minus_one() {
        let mut criteria = GraphicalIndicatorCriteria::new(Weak::new());
        criteria.set_indicator(2);
        criteria.set_left_value(Some(FieldKey::Task(TaskField::Name)));
        criteria.set_operator(Some(TestOperator::Equals));
        criteria.set_right_value(0, Some(Operand::Value(FieldValue::String("A".to_owned()))));

        assert_eq!(criteria.evaluate_indicator(&container_with("A")), 2);
        assert_eq!(criteria.evaluate_indicator(&container_with("B")), -1);
    }

    #[test]
    fn inherited_evaluate_is_forwarded() {
        let mut criteria = GraphicalIndicatorCriteria::new(Weak::new());
        criteria.set_left_value(Some(FieldKey::Task(TaskField::Name)));
        criteria.set_operator(Some(TestOperator::Equals));
        criteria.set_right_value(0, Some(Operand::Value(FieldValue::String("A".to_owned()))));

        let empty = PromptValues::new();
        assert!(criteria.evaluate(&container_with("A"), &empty));
        assert!(!criteria.evaluate(&container_with("B"), &empty));
    }

    #[test]
    fn criteria_list_is_forwarded() {
        let mut criteria = GraphicalIndicatorCriteria::new(Weak::new());
        criteria.set_operator(Some(TestOperator::And));

        let mut child = GenericCriteria::new(Weak::new());
        child.set_operator(Some(TestOperator::Equals));
        criteria.add_criteria(child);

        assert_eq!(criteria.get_criteria_list().len(), 1);
        criteria.get_criteria_list_mut().clear();
        assert!(criteria.get_criteria_list().is_empty());
    }

    #[test]
    fn display_matches_java_format() {
        let mut criteria = GraphicalIndicatorCriteria::new(Weak::new());
        criteria.set_indicator(1);
        criteria.set_left_value(Some(FieldKey::Task(TaskField::Name)));
        criteria.set_operator(Some(TestOperator::Equals));
        criteria.set_right_value(0, Some(Operand::Value(FieldValue::String("A".to_owned()))));

        assert_eq!(
            criteria.to_string(),
            "[GraphicalIndicatorCriteria indicator=1 criteria=(Task Name EQUALS A)]"
        );
    }

    #[test]
    fn prompt_responses_are_absent_in_indicator_evaluation() {
// evaluate(FieldContainer) passes a null prompt response ⇒ an RHS containing only prompts resolves to null
        let prompt = GenericCriteriaPrompt::new(crate::data_type::DataType::String, None);
        let mut criteria = GraphicalIndicatorCriteria::new(Weak::new());
        criteria.set_indicator(5);
        criteria.set_left_value(Some(FieldKey::Task(TaskField::Name)));
        criteria.set_operator(Some(TestOperator::Equals));
        criteria.set_right_value(0, Some(Operand::Prompt(prompt)));

        assert_eq!(criteria.evaluate_indicator(&container_with("A")), -1);
    }
}
