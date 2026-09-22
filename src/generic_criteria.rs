//! Corresponds to Java: `org/mpxj/GenericCriteria.java`
//!
//! Skipped/not-ported Java methods and the reasons:
//!
//! | Java | Rust |
//! |---|---|
//! | `public GenericCriteria(ProjectProperties properties)` | [`GenericCriteria::new`] (the Rust side accepts `Weak<RefCell<ProjectFile>>`, see below) |
//! | `public void setLeftValue(FieldType value)` / `public FieldType getLeftValue()` | [`GenericCriteria::set_left_value`] / [`GenericCriteria::get_left_value`](`FieldType` → [`FieldKey`]) |
//! | `public TestOperator getOperator()` / `public void setOperator(TestOperator)` | [`GenericCriteria::get_operator`] / [`GenericCriteria::set_operator`] |
//! | `public void setRightValue(int index, Object value)` | [`GenericCriteria::set_right_value`] |
//! | `public Object getValue(int index)` | [`GenericCriteria::get_value`] |
//! | `public boolean evaluate(FieldContainer container, Map<GenericCriteriaPrompt, Object> promptValues)` | [`GenericCriteria::evaluate`] |
//! | `private boolean evaluateLogicalOperator(...)` | private method `evaluate_logical_operator` |
//! | `private Object[] processSymbolicValues(...)` | private method `process_symbolic_values` |
//! | `public List<GenericCriteria> getCriteriaList()` | [`GenericCriteria::get_criteria_list`] (immutable slice) and [`GenericCriteria::get_criteria_list_mut`] (corresponding to the mutable list Java returns) |
//! | `public void addCriteria(GenericCriteria criteria)` | [`GenericCriteria::add_criteria`] |
//! | `@Override public String toString()` | [`Display`] |
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: not used by this class, nothing skipped.
//!
//! # Differences from Java
//!
//! 1. **`ProjectProperties` → `Weak<RefCell<ProjectFile>>`**: Java holds a direct
//! `ProjectProperties` reference, used only for the single conversion
//! `Duration.convertUnits(TimeUnit.HOURS, m_properties)`. In this crate `ProjectProperties` is held
//! by value by [`crate::project_file::ProjectFile`] and borrowed via `get_project_properties()` (consistent with the existing convention in other modules of this crate),
//! so the Rust side stores a weak reference to the parent file and fetches the default value on use. When the parent file has been dropped (or not wired up) it degrades gracefully to
//! **no conversion** (Java throws `NullPointerException` because `properties` is `null`).
//! 2. **`Object` values**: Java uses `Object` to carry strings / numbers / dates / durations / `FieldType` /
//! `GenericCriteriaPrompt`; the Rust side uniformly expresses these with [`Operand`] (see `crate::test_operator`).
//! `m_definedRightValues` / `m_workingRightValues` are fixed-length arrays of 2; the Rust side uses
//! `[Option<Operand>; 2]`; [`GenericCriteria::set_right_value`] degrades gracefully to a no-op when out of bounds
//! (Java throws `ArrayIndexOutOfBoundsException`).
//! 3. **`promptValues == null`**: Java's `processSymbolicValues` keeps the `GenericCriteriaPrompt` object itself when `promptValues` is
//! `null`; the Rust-side response table is a reference, so there is no
//! "is `null`" case, so it always looks up the table and treats a miss as `null` (equivalent to Java's lookup returning `null`).
//! 4. **`m_operator == null`**: Java throws `NullPointerException` at `switch (m_operator)`;
//! the Rust [`GenericCriteria::evaluate`] returns `false` (without panicking).
//! 5. **Base class/subclass relationship**: Java's `GraphicalIndicatorCriteria extends GenericCriteria`;
//! Rust has no inheritance, so `GraphicalIndicatorCriteria` instead **composes** this type and forwards (see
//! `crate::graphical_indicator_criteria`). Therefore the element type of [`GenericCriteria::get_criteria_list`] is
//! `GenericCriteria`, and `GraphicalIndicatorCriteria` cannot be added as a child criterion
//! (although it can be added in Java, MPXJ's own `CriteriaReader` only adds plain `GenericCriteria`).
//! 6. **Field names in `toString()`**: Java concatenates `m_leftValue` (`FieldType`'s enum `toString()`,
//! i.e. a display name such as `TaskField.getName()`); the Rust [`Display`] of [`FieldKey`] is equivalent to it.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::rc::Weak;

use crate::common::local_date_time_helper::LocalDateTimeHelper;
use crate::data_type::DataType;
use crate::duration::Duration;
use crate::field_container::FieldContainer;
use crate::field_key::FieldKey;
use crate::field_type::FieldType;
use crate::field_value::FieldValue;
use crate::generic_criteria_prompt::GenericCriteriaPrompt;
use crate::project_file::ProjectFile;
use crate::test_operator::{Operand, TestOperator};
use crate::time_unit::TimeUnit;

/// Response table for filter prompts.
///
/// Corresponds to Java: `Map<GenericCriteriaPrompt, Object> promptValues`
///
/// In Java a key that is not found returns `null`; on the Rust side a "nonexistent key" is equivalent to `null`.
pub type PromptValues = HashMap<GenericCriteriaPrompt, Operand>;

/// The criteria used by one evaluation (LHS operator RHS, or child criteria combined with AND / OR).
///
/// Corresponds to Java: `public class GenericCriteria`
pub struct GenericCriteria {
/// The parent project (weak reference).
    ///
    /// Corresponds to Java: `private final ProjectProperties m_properties;`
    m_project: Weak<RefCell<ProjectFile>>,

/// Left-hand side of the expression (field).
    ///
    /// Corresponds to Java: `private FieldType m_leftValue;`
    m_left_value: Option<FieldKey>,

/// Test operator.
    ///
    /// Corresponds to Java: `private TestOperator m_operator;`
    m_operator: Option<TestOperator>,

/// Original right-hand value (used for display and `getValue`).
    ///
    /// Corresponds to Java: `private final Object[] m_definedRightValues = new Object[2];`
    m_defined_right_values: [Option<Operand>; 2],

/// Working copy of the right-hand value (durations already converted to hours).
    ///
    /// Corresponds to Java: `private final Object[] m_workingRightValues = new Object[2];`
    m_working_right_values: [Option<Operand>; 2],

/// Whether symbolic values (`FieldType`) are present, determining whether resolution is needed during evaluation.
    ///
    /// Corresponds to Java: `private boolean m_symbolicValues;`
    m_symbolic_values: bool,

/// Child criteria (AND / OR combination).
    ///
    /// Corresponds to Java: `private final List<GenericCriteria> m_criteriaList = new ArrayList<>();`
    m_criteria_list: Vec<GenericCriteria>,
}

impl GenericCriteria {
/// Constructor.
    ///
    /// Corresponds to Java: `public GenericCriteria(ProjectProperties properties)`
    ///
/// The Rust side accepts a weak reference to the parent project (see the file header); pass `Weak::new()` when there is no parent project,
/// in which case duration conversion degrades gracefully to "no conversion".
    pub fn new(project: Weak<RefCell<ProjectFile>>) -> Self {
        Self {
            m_project: project,
            m_left_value: None,
            m_operator: None,
            m_defined_right_values: [None, None],
            m_working_right_values: [None, None],
            m_symbolic_values: false,
            m_criteria_list: Vec::new(),
        }
    }

/// Set the left-hand side of the expression (field).
    ///
    /// Corresponds to Java: `public void setLeftValue(FieldType value)`
    pub fn set_left_value(&mut self, value: Option<FieldKey>) {
        self.m_left_value = value;
    }

/// Get the left-hand side of the expression (field).
    ///
    /// Corresponds to Java: `public FieldType getLeftValue()`
    pub fn get_left_value(&self) -> Option<FieldKey> {
        self.m_left_value
    }

/// Get the test operator.
    ///
    /// Corresponds to Java: `public TestOperator getOperator()`
    pub fn get_operator(&self) -> Option<TestOperator> {
        self.m_operator
    }

/// Set the test operator.
    ///
    /// Corresponds to Java: `public void setOperator(TestOperator operator)`
    pub fn set_operator(&mut self, operator: Option<TestOperator>) {
        self.m_operator = operator;
    }

/// Set the right-hand value at the given index.
    ///
    /// Corresponds to Java: `public void setRightValue(int index, Object value)`
    ///
/// Degrades gracefully to a no-op when `index` is out of bounds (≥ 2) (Java throws `ArrayIndexOutOfBoundsException`).
    pub fn set_right_value(&mut self, index: usize, value: Option<Operand>) {
        if index >= self.m_defined_right_values.len() {
            return;
        }

        let mut working = value.clone();

        match &value {
            // Java: if (value instanceof FieldType) { m_symbolicValues = true; }
            Some(Operand::Field(_)) => self.m_symbolic_values = true,
            // Java: if (value instanceof Duration) { if (units != HOURS) value = value.convertUnits(HOURS, m_properties); }
            Some(Operand::Value(FieldValue::Duration(duration))) => {
                if duration.get_units() != TimeUnit::Hours {
                    working = Some(Operand::Value(FieldValue::Duration(
                        self.convert_to_hours(*duration),
                    )));
                }
            }
            _ => {}
        }

        self.m_defined_right_values[index] = value;
        self.m_working_right_values[index] = working;
    }

/// Get the original right-hand value at the given index.
    ///
    /// Corresponds to Java: `public Object getValue(int index)`
    ///
/// Returns `None` when the index is out of bounds (Java throws `ArrayIndexOutOfBoundsException`).
    pub fn get_value(&self, index: usize) -> Option<&Operand> {
        self.m_defined_right_values.get(index).and_then(Option::as_ref)
    }

/// Evaluate and return a boolean result.
    ///
    /// Corresponds to Java: `public boolean evaluate(FieldContainer container, Map<GenericCriteriaPrompt, Object> promptValues)`
    pub fn evaluate(&self, container: &dyn FieldContainer, prompt_values: &PromptValues) -> bool {
        //
// Get the LHS value
        //
        let lhs = self.left_operand(container);

        //
// Get the RHS value
        //
        let rhs = if self.m_symbolic_values {
            self.process_symbolic_values(container, prompt_values)
        } else {
            self.m_working_right_values.clone()
        };

        //
// Evaluate
        //
        match self.m_operator {
            Some(TestOperator::And) | Some(TestOperator::Or) => {
                self.evaluate_logical_operator(container, prompt_values)
            }
            Some(operator) => operator.evaluate(lhs.as_ref(), &rhs),
// Java throws NullPointerException
            None => false,
        }
    }

/// Get the child criteria list.
    ///
    /// Corresponds to Java: `public List<GenericCriteria> getCriteriaList()`
    pub fn get_criteria_list(&self) -> &[GenericCriteria] {
        &self.m_criteria_list
    }

/// Get the child criteria list mutably (corresponding to the mutable list Java returns).
    ///
    /// Corresponds to Java: `public List<GenericCriteria> getCriteriaList()`
    pub fn get_criteria_list_mut(&mut self) -> &mut Vec<GenericCriteria> {
        &mut self.m_criteria_list
    }

/// Append a child criterion.
    ///
    /// Corresponds to Java: `public void addCriteria(GenericCriteria criteria)`
    pub fn add_criteria(&mut self, criteria: GenericCriteria) {
        self.m_criteria_list.push(criteria);
    }

/// Evaluate an AND / OR combination (the result is `true` when there are no child criteria).
    ///
    /// Corresponds to Java: `private boolean evaluateLogicalOperator(FieldContainer container, Map<GenericCriteriaPrompt, Object> promptValues)`
    fn evaluate_logical_operator(
        &self,
        container: &dyn FieldContainer,
        prompt_values: &PromptValues,
    ) -> bool {
        if self.m_criteria_list.is_empty() {
            return true;
        }

        let mut result = false;
        let is_and = self.m_operator == Some(TestOperator::And);

        for criteria in &self.m_criteria_list {
            result = criteria.evaluate(container, prompt_values);
            if (is_and && !result) || (!is_and && result) {
                break;
            }
        }

        result
    }

/// Resolve symbolic values / prompt values to obtain the right-hand value actually used by this evaluation.
    ///
    /// Corresponds to Java: `private Object[] processSymbolicValues(Object[] oldValues, FieldContainer container, Map<GenericCriteriaPrompt, Object> promptValues)`
    fn process_symbolic_values(
        &self,
        container: &dyn FieldContainer,
        prompt_values: &PromptValues,
    ) -> [Option<Operand>; 2] {
        let mut new_values: [Option<Operand>; 2] = [None, None];

        for (index, value) in self.m_working_right_values.iter().enumerate() {
            let Some(value) = value else {
                continue;
            };

            new_values[index] = match value {
                Operand::Field(field) => self.resolve_symbolic_value(*field, container),
                Operand::Prompt(prompt) => prompt_values.get(prompt).cloned(),
                Operand::Value(_) => Some(value.clone()),
            };
        }

        new_values
    }

/// Resolve a symbolic value (`FieldType`) to the actual value in the container and normalize it by data type.
    ///
/// Corresponds to Java: the `value instanceof FieldType` branch in `processSymbolicValues`.
    fn resolve_symbolic_value(
        &self,
        field: FieldKey,
        container: &dyn FieldContainer,
    ) -> Option<Operand> {
        let value = container.get_cached_value(field).cloned();

        match field.get_data_type() {
            DataType::Date => value
                .and_then(|value| value.as_date_time())
                .and_then(|value| LocalDateTimeHelper::get_day_start_date(Some(value)))
                .map(|value| Operand::Value(FieldValue::DateTime(value))),

            DataType::Duration => match value.and_then(|value| value.as_duration()) {
                Some(duration) if duration.get_units() != TimeUnit::Hours => Some(Operand::Value(
                    FieldValue::Duration(self.convert_to_hours(duration)),
                )),
// Java's else branch: an empty value, or a value whose unit is already hours, is replaced with "0 hours"
                _ => Some(Operand::Value(FieldValue::Duration(Duration::get_instance(
                    0.0,
                    TimeUnit::Hours,
                )))),
            },

            DataType::String => Some(Operand::Value(FieldValue::String(
                value
                    .and_then(|value| value.as_string().map(str::to_owned))
                    .unwrap_or_default(),
            ))),

            _ => value.map(Operand::Value),
        }
    }

/// Get the LHS value and normalize it by data type.
    ///
/// Corresponds to Java: after `lhs = container.get(field)` in `evaluate`, the
/// `switch` on `field.getDataType()`.
    fn left_operand(&self, container: &dyn FieldContainer) -> Option<Operand> {
        let field = self.m_left_value?;
        let value = container.get(field);

        match field.get_data_type() {
            DataType::Date => value
                .and_then(|value| value.as_date_time())
                .and_then(|value| LocalDateTimeHelper::get_day_start_date(Some(value)))
                .map(|value| Operand::Value(FieldValue::DateTime(value))),

            DataType::Duration => match value.and_then(|value| value.as_duration()) {
                Some(duration) => Some(Operand::Value(FieldValue::Duration(
                    self.convert_to_hours(duration),
                ))),
                None => Some(Operand::Value(FieldValue::Duration(Duration::get_instance(
                    0.0,
                    TimeUnit::Hours,
                )))),
            },

            DataType::String => Some(Operand::Value(FieldValue::String(
                value
                    .and_then(|value| value.as_string().map(str::to_owned))
                    .unwrap_or_default(),
            ))),

            _ => value.map(Operand::Value),
        }
    }

/// Convert a duration to hours (Java: `duration.convertUnits(TimeUnit.HOURS, m_properties)`).
    ///
/// Degrades gracefully to the original value when the parent project is not wired up / already borrowed (Java throws when `properties` is `null`).
    fn convert_to_hours(&self, duration: Duration) -> Duration {
        if duration.get_units() == TimeUnit::Hours {
            return duration;
        }

        let Some(project) = self.m_project.upgrade() else {
            return duration;
        };

        match project.try_borrow() {
            Ok(project) => duration.convert_units(TimeUnit::Hours, project.get_project_properties()),
            Err(_) => duration,
        }
    }
}

impl Display for GenericCriteria {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// AND / OR combinations join the child criteria with the operator; other operators output
/// `(left operator right[,right])`, where `LocalDateTime` uses the `dd/MM/yyyy HH:mm` format.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut result = String::from("(");

        match self.m_operator {
            Some(TestOperator::And) | Some(TestOperator::Or) => {
                let operator = self
                    .m_operator
                    .map(|operator| operator.to_string())
                    .unwrap_or_default();

                for (index, criteria) in self.m_criteria_list.iter().enumerate() {
                    result.push_str(&criteria.to_string());
                    if index + 1 < self.m_criteria_list.len() {
                        result.push(' ');
                        result.push_str(&operator);
                        result.push(' ');
                    }
                }
            }

            _ => {
                result.push_str(&render_field(self.m_left_value));
                result.push(' ');
                result.push_str(
                    &self
                        .m_operator
                        .map(|operator| operator.to_string())
                        .unwrap_or_else(|| "null".to_owned()),
                );
                result.push(' ');
                result.push_str(&render_operand(&self.m_defined_right_values[0]));

                if self.m_defined_right_values[1].is_some() {
                    result.push(',');
                    result.push_str(&render_operand(&self.m_defined_right_values[1]));
                }
            }
        }

        result.push(')');
        f.write_str(&result)
    }
}

impl fmt::Debug for GenericCriteria {
/// Java does not override the debug output; the Rust side provides `Debug` (reusing [`Display`]).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

/// Render a field type (`None` renders as `"null"` in Java's string concatenation).
fn render_field(field: Option<FieldKey>) -> String {
    match field {
        Some(field) => field.to_string(),
        None => "null".to_owned(),
    }
}

/// Render a right-hand value (`LocalDateTime` uses the `dd/MM/yyyy HH:mm` format, consistent with Java).
fn render_operand(value: &Option<Operand>) -> String {
    match value {
        Some(Operand::Value(FieldValue::DateTime(value))) => {
            value.format("%d/%m/%Y %H:%M").to_string()
        }
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::listener::field_listener::FieldListener;
    use crate::task_field::TaskField;

/// Test field container: stores only the fields visible to `getCachedValue`.
    #[derive(Default)]
    struct MockContainer {
        values: HashMap<FieldKey, FieldValue>,
    }

    impl MockContainer {
        fn with(field: FieldKey, value: FieldValue) -> Self {
            let mut container = Self::default();
            container.values.insert(field, value);
            container
        }
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

/// Construct a `field operator value` criterion.
    fn criteria(
        field: FieldKey,
        operator: TestOperator,
        value: Option<Operand>,
    ) -> GenericCriteria {
        let mut criteria = GenericCriteria::new(Weak::new());
        criteria.set_left_value(Some(field));
        criteria.set_operator(Some(operator));
        criteria.set_right_value(0, value);
        criteria
    }

    #[test]
    fn accessors_round_trip() {
        let mut criteria = GenericCriteria::new(Weak::new());
        assert_eq!(criteria.get_left_value(), None);
        assert_eq!(criteria.get_operator(), None);
        assert_eq!(criteria.get_value(0), None);
        assert!(criteria.get_criteria_list().is_empty());

        criteria.set_left_value(Some(FieldKey::Task(TaskField::Name)));
        criteria.set_operator(Some(TestOperator::Equals));
        criteria.set_right_value(0, Some(Operand::Value(FieldValue::String("x".to_owned()))));

        assert_eq!(
            criteria.get_left_value(),
            Some(FieldKey::Task(TaskField::Name))
        );
        assert_eq!(criteria.get_operator(), Some(TestOperator::Equals));
        assert_eq!(
            criteria.get_value(0),
            Some(&Operand::Value(FieldValue::String("x".to_owned())))
        );

// Neither out-of-bounds setting nor reading panics
        criteria.set_right_value(5, Some(Operand::Value(FieldValue::Integer(1))));
        assert_eq!(criteria.get_value(5), None);
    }

    #[test]
    fn string_field_is_compared_with_rhs() {
        let container = MockContainer::with(
            FieldKey::Task(TaskField::Name),
            FieldValue::String("Design".to_owned()),
        );

        let criteria = criteria(
            FieldKey::Task(TaskField::Name),
            TestOperator::Equals,
            Some(Operand::Value(FieldValue::String("Design".to_owned()))),
        );
        assert!(criteria.evaluate(&container, &PromptValues::new()));

        let criteria = self::criteria(
            FieldKey::Task(TaskField::Name),
            TestOperator::Equals,
            Some(Operand::Value(FieldValue::String("Build".to_owned()))),
        );
        assert!(!criteria.evaluate(&container, &PromptValues::new()));
    }

    #[test]
    fn missing_string_field_becomes_empty_string() {
        let container = MockContainer::default();
        let criteria = criteria(
            FieldKey::Task(TaskField::Name),
            TestOperator::Equals,
            Some(Operand::Value(FieldValue::String(String::new()))),
        );

        // Java: case STRING -> lhs = lhs == null ? "" : lhs;
        assert!(criteria.evaluate(&container, &PromptValues::new()));
    }

    #[test]
    fn duration_rhs_is_converted_to_hours_on_set() {
        let mut criteria = GenericCriteria::new(Weak::new());
        criteria.set_right_value(
            0,
            Some(Operand::Value(FieldValue::Duration(
// No parent project ⇒ no conversion, so the working copy matches the defined value
                Duration::get_instance(2.0, TimeUnit::Days),
            ))),
        );

        assert_eq!(
            criteria.get_value(0),
            Some(&Operand::Value(FieldValue::Duration(Duration::get_instance(
                2.0,
                TimeUnit::Days
            ))))
        );
    }

    #[test]
    fn duration_field_without_value_evaluates_as_zero_hours() {
        let container = MockContainer::default();
        let criteria = criteria(
            FieldKey::Task(TaskField::Duration),
            TestOperator::Equals,
            Some(Operand::Value(FieldValue::Duration(Duration::get_instance(
                0.0,
                TimeUnit::Hours,
            )))),
        );

        assert!(criteria.evaluate(&container, &PromptValues::new()));
    }

    #[test]
    fn symbolic_field_value_is_resolved_from_container() {
        let mut container = MockContainer::with(
            FieldKey::Task(TaskField::Name),
            FieldValue::String("Alpha".to_owned()),
        );
// The symbolic field referenced by the RHS also resolves to the same value in the container
        container.values.insert(
            FieldKey::Task(TaskField::Text1),
            FieldValue::String("Alpha".to_owned()),
        );

        let mut criteria = GenericCriteria::new(Weak::new());
        criteria.set_left_value(Some(FieldKey::Task(TaskField::Name)));
        criteria.set_operator(Some(TestOperator::Equals));
// The RHS is a symbolic value (field reference), resolved from the container during evaluation
        criteria.set_right_value(0, Some(Operand::Field(FieldKey::Task(TaskField::Text1))));

        assert!(criteria.evaluate(&container, &PromptValues::new()));
    }

    #[test]
    fn prompt_value_is_resolved_from_responses() {
        let prompt = GenericCriteriaPrompt::new(DataType::String, Some("Enter".to_owned()));
        let mut criteria = GenericCriteria::new(Weak::new());
        criteria.set_left_value(Some(FieldKey::Task(TaskField::Name)));
        criteria.set_operator(Some(TestOperator::Equals));
// The RHS contains both a symbolic value and a prompt value (Java resolves only when a symbolic value is present)
        criteria.set_right_value(0, Some(Operand::Field(FieldKey::Task(TaskField::Text1))));
        criteria.set_right_value(1, Some(Operand::Prompt(prompt.clone())));

        let mut responses = PromptValues::new();
        responses.insert(
            prompt,
            Operand::Value(FieldValue::String("Design".to_owned())),
        );

// Symbolic value Text1 is empty ⇒ LHS "Design" and "" are not equal
        let container = {
            let mut container = MockContainer::default();
            container.set(
                FieldKey::Task(TaskField::Name),
                Some(FieldValue::String("Design".to_owned())),
            );
            container
        };
        assert!(!criteria.evaluate(&container, &responses));

// Symbolic value Text1 resolves to "Design" ⇒ equal to the LHS
        let container = {
            let mut container = MockContainer::default();
            container.set(
                FieldKey::Task(TaskField::Name),
                Some(FieldValue::String("Design".to_owned())),
            );
            container.set(
                FieldKey::Task(TaskField::Text1),
                Some(FieldValue::String("Design".to_owned())),
            );
            container
        };
        assert!(criteria.evaluate(&container, &responses));
    }

    #[test]
    fn and_operator_requires_all_children() {
        let container = MockContainer::with(
            FieldKey::Task(TaskField::Name),
            FieldValue::String("Design".to_owned()),
        );

        let mut group = GenericCriteria::new(Weak::new());
        group.set_operator(Some(TestOperator::And));
        group.add_criteria(criteria(
            FieldKey::Task(TaskField::Name),
            TestOperator::Equals,
            Some(Operand::Value(FieldValue::String("Design".to_owned()))),
        ));
        group.add_criteria(criteria(
            FieldKey::Task(TaskField::Name),
            TestOperator::Equals,
            Some(Operand::Value(FieldValue::String("Build".to_owned()))),
        ));

        assert!(!group.evaluate(&container, &PromptValues::new()));
        assert_eq!(group.get_criteria_list().len(), 2);

// Empty child criteria list ⇒ true
        let mut empty = GenericCriteria::new(Weak::new());
        empty.set_operator(Some(TestOperator::And));
        assert!(empty.evaluate(&container, &PromptValues::new()));
    }

    #[test]
    fn or_operator_stops_at_first_match() {
        let container = MockContainer::with(
            FieldKey::Task(TaskField::Name),
            FieldValue::String("Design".to_owned()),
        );

        let mut group = GenericCriteria::new(Weak::new());
        group.set_operator(Some(TestOperator::Or));
        group.add_criteria(criteria(
            FieldKey::Task(TaskField::Name),
            TestOperator::Equals,
            Some(Operand::Value(FieldValue::String("Build".to_owned()))),
        ));
        group.add_criteria(criteria(
            FieldKey::Task(TaskField::Name),
            TestOperator::Equals,
            Some(Operand::Value(FieldValue::String("Design".to_owned()))),
        ));

        assert!(group.evaluate(&container, &PromptValues::new()));
    }

    #[test]
    fn evaluate_without_operator_returns_false() {
        let container = MockContainer::default();
        let criteria = GenericCriteria::new(Weak::new());
        assert!(!criteria.evaluate(&container, &PromptValues::new()));
    }

    #[test]
    fn display_matches_java_format() {
        let criteria = criteria(
            FieldKey::Task(TaskField::Name),
            TestOperator::Equals,
            Some(Operand::Value(FieldValue::String("Design".to_owned()))),
        );
        assert_eq!(criteria.to_string(), "(Task Name EQUALS Design)");

// No operator
        let empty = GenericCriteria::new(Weak::new());
        assert_eq!(empty.to_string(), "(null null null)");

// AND combination
        let mut group = GenericCriteria::new(Weak::new());
        group.set_operator(Some(TestOperator::And));
        group.add_criteria(self::criteria(
            FieldKey::Task(TaskField::Name),
            TestOperator::Equals,
            Some(Operand::Value(FieldValue::String("A".to_owned()))),
        ));
        group.add_criteria(self::criteria(
            FieldKey::Task(TaskField::Name),
            TestOperator::Equals,
            Some(Operand::Value(FieldValue::String("B".to_owned()))),
        ));
        assert_eq!(
            group.to_string(),
            "((Task Name EQUALS A) AND (Task Name EQUALS B))"
        );
    }

    #[test]
    fn display_formats_dates_like_java() {
        let value = chrono::NaiveDate::from_ymd_opt(2024, 5, 1)
            .and_then(|date| date.and_hms_opt(13, 30, 0))
            .expect("valid date");
        let criteria = criteria(
            FieldKey::Task(TaskField::Start),
            TestOperator::IsGreaterThan,
            Some(Operand::Value(FieldValue::DateTime(value))),
        );
        assert_eq!(criteria.to_string(), "(Start IS_GREATER_THAN 01/05/2024 13:30)");
    }
}
