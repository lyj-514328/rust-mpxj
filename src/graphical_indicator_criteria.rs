//! 对应 Java: `org/mpxj/GraphicalIndicatorCriteria.java`
//!
//! 跳过/未移植的 Java 方法与原因：
//!
//! | Java | Rust |
//! |---|---|
//! | `public GraphicalIndicatorCriteria(ProjectProperties properties)` | [`GraphicalIndicatorCriteria::new`]（Rust 侧接收 `Weak<RefCell<ProjectFile>>`，见下） |
//! | `public int getIndicator()` / `public void setIndicator(int indicator)` | [`GraphicalIndicatorCriteria::get_indicator`] / [`GraphicalIndicatorCriteria::set_indicator`] |
//! | `public int evaluate(FieldContainer container)` | [`GraphicalIndicatorCriteria::evaluate_indicator`]（Rust 无重载，加后缀以区别于继承来的 2 参数 `evaluate`） |
//! | `@Override public String toString()` | [`Display`] |
//! | 继承自 `GenericCriteria` 的 `setLeftValue` / `getLeftValue` / `getOperator` / `setOperator` / `setRightValue` / `getValue` / `evaluate(FieldContainer, Map)` / `getCriteriaList` / `addCriteria` | [`GraphicalIndicatorCriteria::set_left_value`] / [`GraphicalIndicatorCriteria::get_left_value`] / [`GraphicalIndicatorCriteria::get_operator`] / [`GraphicalIndicatorCriteria::set_operator`] / [`GraphicalIndicatorCriteria::set_right_value`] / [`GraphicalIndicatorCriteria::get_value`] / [`GraphicalIndicatorCriteria::evaluate`] / [`GraphicalIndicatorCriteria::get_criteria_list`] / [`GraphicalIndicatorCriteria::get_criteria_list_mut`] / [`GraphicalIndicatorCriteria::add_criteria`] |
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//!
//! # 继承关系的还原方式：组合 + 转发
//!
//! Java 的 `GraphicalIndicatorCriteria extends GenericCriteria`。Rust 没有继承，本类改为
//! **组合**一个 [`GenericCriteria`]（字段 `criteria`）并把基类的公开方法逐一转发，
//! 额外的 `m_indicator` 作为本类自己的字段。因此本类**不是** `GenericCriteria`，
//! 不能作为子条件加入其它 `GenericCriteria` 的 `criteriaList`（MPXJ 自身的
//! `CriteriaReader` 只加入普通 `GenericCriteria`，不受影响）。
//!
//! # 与 Java 的差异
//!
//! - **构造方法参数**：Java 取 `ProjectProperties`，Rust 侧取父项目弱引用（与
//!   [`GenericCriteria::new`] 一致，见该类型的文件头说明）。
//! - **`evaluate(FieldContainer)` 传 `null` 提示应答**：Java 显式传 `null`；Rust 侧
//!   传空表（等价于「查不到任何提示应答」，见 [`GenericCriteria::evaluate`] 的说明）。

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::Weak;

use crate::field_container::FieldContainer;
use crate::field_key::FieldKey;
use crate::generic_criteria::{GenericCriteria, PromptValues};
use crate::project_file::ProjectFile;
use crate::test_operator::{Operand, TestOperator};

/// 决定是否以图形化指标替代字段值显示的判定条件。
///
/// 对应 Java: `public final class GraphicalIndicatorCriteria extends GenericCriteria`
pub struct GraphicalIndicatorCriteria {
    /// 组合的基类实例（对应 Java 的 `extends GenericCriteria`）。
    criteria: GenericCriteria,

    /// 满足条件时显示的指标编号。
    ///
    /// 对应 Java: `private int m_indicator;`
    m_indicator: i32,
}

impl GraphicalIndicatorCriteria {
    /// 构造方法。
    ///
    /// 对应 Java: `public GraphicalIndicatorCriteria(ProjectProperties properties)`
    pub fn new(project: Weak<RefCell<ProjectFile>>) -> Self {
        Self {
            criteria: GenericCriteria::new(project),
            m_indicator: 0,
        }
    }

    /// 取指标编号。
    ///
    /// 对应 Java: `public int getIndicator()`
    pub fn get_indicator(&self) -> i32 {
        self.m_indicator
    }

    /// 设置指标编号。
    ///
    /// 对应 Java: `public void setIndicator(int indicator)`
    pub fn set_indicator(&mut self, indicator: i32) {
        self.m_indicator = indicator;
    }

    /// 求值：满足条件时返回指标编号，否则返回 `-1`。
    ///
    /// 对应 Java: `public int evaluate(FieldContainer container)`
    ///
    /// Rust 无重载，故加 `_indicator` 后缀以区别于继承来的
    /// `evaluate(FieldContainer, promptValues)`。
    pub fn evaluate_indicator(&self, container: &dyn FieldContainer) -> i32 {
        // Java: return (evaluate(container, null) ? m_indicator : -1);
        if self.criteria.evaluate(container, &PromptValues::new()) {
            self.m_indicator
        } else {
            -1
        }
    }

    /// 设置表达式左侧（字段）。
    ///
    /// 对应 Java: `GenericCriteria.setLeftValue(FieldType value)`
    pub fn set_left_value(&mut self, value: Option<FieldKey>) {
        self.criteria.set_left_value(value);
    }

    /// 取表达式左侧（字段）。
    ///
    /// 对应 Java: `GenericCriteria.getLeftValue()`
    pub fn get_left_value(&self) -> Option<FieldKey> {
        self.criteria.get_left_value()
    }

    /// 取测试运算符。
    ///
    /// 对应 Java: `GenericCriteria.getOperator()`
    pub fn get_operator(&self) -> Option<TestOperator> {
        self.criteria.get_operator()
    }

    /// 设置测试运算符。
    ///
    /// 对应 Java: `GenericCriteria.setOperator(TestOperator operator)`
    pub fn set_operator(&mut self, operator: Option<TestOperator>) {
        self.criteria.set_operator(operator);
    }

    /// 设置指定下标的右值。
    ///
    /// 对应 Java: `GenericCriteria.setRightValue(int index, Object value)`
    pub fn set_right_value(&mut self, index: usize, value: Option<Operand>) {
        self.criteria.set_right_value(index, value);
    }

    /// 取指定下标的右值原值。
    ///
    /// 对应 Java: `GenericCriteria.getValue(int index)`
    pub fn get_value(&self, index: usize) -> Option<&Operand> {
        self.criteria.get_value(index)
    }

    /// 求值并返回布尔结果（继承自 `GenericCriteria`）。
    ///
    /// 对应 Java: `GenericCriteria.evaluate(FieldContainer container, Map<GenericCriteriaPrompt, Object> promptValues)`
    pub fn evaluate(
        &self,
        container: &dyn FieldContainer,
        prompt_values: &PromptValues,
    ) -> bool {
        self.criteria.evaluate(container, prompt_values)
    }

    /// 取子条件列表。
    ///
    /// 对应 Java: `GenericCriteria.getCriteriaList()`
    pub fn get_criteria_list(&self) -> &[GenericCriteria] {
        self.criteria.get_criteria_list()
    }

    /// 以可变方式取子条件列表。
    ///
    /// 对应 Java: `GenericCriteria.getCriteriaList()`
    pub fn get_criteria_list_mut(&mut self) -> &mut Vec<GenericCriteria> {
        self.criteria.get_criteria_list_mut()
    }

    /// 追加一个子条件。
    ///
    /// 对应 Java: `GenericCriteria.addCriteria(GenericCriteria criteria)`
    pub fn add_criteria(&mut self, criteria: GenericCriteria) {
        self.criteria.add_criteria(criteria);
    }
}

impl Display for GraphicalIndicatorCriteria {
    /// 对应 Java: `@Override public String toString()`
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
    /// Java 未覆写调试输出；Rust 侧提供 `Debug`（复用 [`Display`]）。
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

    /// 测试用的字段容器：只保存 `getCachedValue` 可见的字段。
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
        // evaluate(FieldContainer) 传 null 提示应答 ⇒ 只含提示的 RHS 解析为 null
        let prompt = GenericCriteriaPrompt::new(crate::data_type::DataType::String, None);
        let mut criteria = GraphicalIndicatorCriteria::new(Weak::new());
        criteria.set_indicator(5);
        criteria.set_left_value(Some(FieldKey::Task(TaskField::Name)));
        criteria.set_operator(Some(TestOperator::Equals));
        criteria.set_right_value(0, Some(Operand::Prompt(prompt)));

        assert_eq!(criteria.evaluate_indicator(&container_with("A")), -1);
    }
}
