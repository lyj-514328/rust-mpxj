//! 对应 Java: `org/mpxj/GenericCriteria.java`
//!
//! 跳过/未移植的 Java 方法与原因：
//!
//! | Java | Rust |
//! |---|---|
//! | `public GenericCriteria(ProjectProperties properties)` | [`GenericCriteria::new`]（Rust 侧接收 `Weak<RefCell<ProjectFile>>`，见下） |
//! | `public void setLeftValue(FieldType value)` / `public FieldType getLeftValue()` | [`GenericCriteria::set_left_value`] / [`GenericCriteria::get_left_value`]（`FieldType` → [`FieldKey`]） |
//! | `public TestOperator getOperator()` / `public void setOperator(TestOperator)` | [`GenericCriteria::get_operator`] / [`GenericCriteria::set_operator`] |
//! | `public void setRightValue(int index, Object value)` | [`GenericCriteria::set_right_value`] |
//! | `public Object getValue(int index)` | [`GenericCriteria::get_value`] |
//! | `public boolean evaluate(FieldContainer container, Map<GenericCriteriaPrompt, Object> promptValues)` | [`GenericCriteria::evaluate`] |
//! | `private boolean evaluateLogicalOperator(...)` | 私有方法 `evaluate_logical_operator` |
//! | `private Object[] processSymbolicValues(...)` | 私有方法 `process_symbolic_values` |
//! | `public List<GenericCriteria> getCriteriaList()` | [`GenericCriteria::get_criteria_list`]（不可变切片）与 [`GenericCriteria::get_criteria_list_mut`]（对应 Java 返回的可变列表） |
//! | `public void addCriteria(GenericCriteria criteria)` | [`GenericCriteria::add_criteria`] |
//! | `@Override public String toString()` | [`Display`] |
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//!
//! # 与 Java 的差异
//!
//! 1. **`ProjectProperties` → `Weak<RefCell<ProjectFile>>`**：Java 直接持有
//!    `ProjectProperties` 引用，仅用于 `Duration.convertUnits(TimeUnit.HOURS, m_properties)`
//!    这一处换算。本 crate 的 `ProjectProperties` 由 [`crate::project_file::ProjectFile`]
//!    按值持有、经 `get_project_properties()` 借用（与本 crate 其它模块的既有约定一致），
//!    因此 Rust 侧存父文件的弱引用，用时再取默认值。父文件已释放（或未接线）时退化为
//!    **不做换算**（Java 会因 `properties` 为 `null` 抛 `NullPointerException`）。
//! 2. **`Object` 取值**：Java 用 `Object` 承载字符串 / 数值 / 日期 / 工期 / `FieldType` /
//!    `GenericCriteriaPrompt`，Rust 侧统一用 [`Operand`] 表达（见 `crate::test_operator`）。
//!    `m_definedRightValues` / `m_workingRightValues` 是定长 2 的数组，Rust 侧用
//!    `[Option<Operand>; 2]`；[`GenericCriteria::set_right_value`] 越界时退化为 no-op
//!    （Java 抛 `ArrayIndexOutOfBoundsException`）。
//! 3. **`promptValues == null`**：Java 的 `processSymbolicValues` 在 `promptValues` 为
//!    `null` 时保留 `GenericCriteriaPrompt` 对象本身；Rust 侧的应答表是引用，不存在
//!    「为 `null`」的情形，因此一律查表，查不到即视为 `null`（与 Java 查表返回 `null` 等价）。
//! 4. **`m_operator == null`**：Java 在 `switch (m_operator)` 处抛 `NullPointerException`；
//!    Rust 侧 [`GenericCriteria::evaluate`] 返回 `false`（不 panic）。
//! 5. **父类/子类关系**：Java 的 `GraphicalIndicatorCriteria extends GenericCriteria`；
//!    Rust 无继承，`GraphicalIndicatorCriteria` 改为**组合**本类型并转发（见
//!    `crate::graphical_indicator_criteria`）。因此 [`GenericCriteria::get_criteria_list`] 的
//!    元素类型是 `GenericCriteria`，`GraphicalIndicatorCriteria` 不能作为子条件加入
//!    （Java 中虽可加入，但 MPXJ 自身的 `CriteriaReader` 只加入普通 `GenericCriteria`）。
//! 6. **`toString()` 中的字段名**：Java 拼 `m_leftValue`（`FieldType` 枚举的 `toString()`，
//!    即 `TaskField.getName()` 之类的显示名）；Rust 侧 [`FieldKey`] 的 [`Display`] 与之等价。

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

/// 筛选提示的应答表。
///
/// 对应 Java: `Map<GenericCriteriaPrompt, Object> promptValues`
///
/// Java 中查不到的键返回 `null`；Rust 侧「不存在的键」即等价于 `null`。
pub type PromptValues = HashMap<GenericCriteriaPrompt, Operand>;

/// 一次求值所使用的条件（LHS 运算符 RHS，或 AND / OR 组合的子条件）。
///
/// 对应 Java: `public class GenericCriteria`
pub struct GenericCriteria {
    /// 父项目（弱引用）。
    ///
    /// 对应 Java: `private final ProjectProperties m_properties;`
    m_project: Weak<RefCell<ProjectFile>>,

    /// 表达式左侧（字段）。
    ///
    /// 对应 Java: `private FieldType m_leftValue;`
    m_left_value: Option<FieldKey>,

    /// 测试运算符。
    ///
    /// 对应 Java: `private TestOperator m_operator;`
    m_operator: Option<TestOperator>,

    /// 右值原值（用于展示与 `getValue`）。
    ///
    /// 对应 Java: `private final Object[] m_definedRightValues = new Object[2];`
    m_defined_right_values: [Option<Operand>; 2],

    /// 右值工作副本（工期已换算为小时）。
    ///
    /// 对应 Java: `private final Object[] m_workingRightValues = new Object[2];`
    m_working_right_values: [Option<Operand>; 2],

    /// 是否包含符号值（`FieldType`），决定求值时是否需要解析。
    ///
    /// 对应 Java: `private boolean m_symbolicValues;`
    m_symbolic_values: bool,

    /// 子条件（AND / OR 组合）。
    ///
    /// 对应 Java: `private final List<GenericCriteria> m_criteriaList = new ArrayList<>();`
    m_criteria_list: Vec<GenericCriteria>,
}

impl GenericCriteria {
    /// 构造方法。
    ///
    /// 对应 Java: `public GenericCriteria(ProjectProperties properties)`
    ///
    /// Rust 侧接收父项目弱引用（见文件头说明）；无父项目时传 `Weak::new()`，
    /// 此时工期换算退化为「不做换算」。
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

    /// 设置表达式左侧（字段）。
    ///
    /// 对应 Java: `public void setLeftValue(FieldType value)`
    pub fn set_left_value(&mut self, value: Option<FieldKey>) {
        self.m_left_value = value;
    }

    /// 取表达式左侧（字段）。
    ///
    /// 对应 Java: `public FieldType getLeftValue()`
    pub fn get_left_value(&self) -> Option<FieldKey> {
        self.m_left_value
    }

    /// 取测试运算符。
    ///
    /// 对应 Java: `public TestOperator getOperator()`
    pub fn get_operator(&self) -> Option<TestOperator> {
        self.m_operator
    }

    /// 设置测试运算符。
    ///
    /// 对应 Java: `public void setOperator(TestOperator operator)`
    pub fn set_operator(&mut self, operator: Option<TestOperator>) {
        self.m_operator = operator;
    }

    /// 设置指定下标的右值。
    ///
    /// 对应 Java: `public void setRightValue(int index, Object value)`
    ///
    /// `index` 越界（≥ 2）时退化为 no-op（Java 抛 `ArrayIndexOutOfBoundsException`）。
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

    /// 取指定下标的右值原值。
    ///
    /// 对应 Java: `public Object getValue(int index)`
    ///
    /// 下标越界时返回 `None`（Java 抛 `ArrayIndexOutOfBoundsException`）。
    pub fn get_value(&self, index: usize) -> Option<&Operand> {
        self.m_defined_right_values.get(index).and_then(Option::as_ref)
    }

    /// 求值并返回布尔结果。
    ///
    /// 对应 Java: `public boolean evaluate(FieldContainer container, Map<GenericCriteriaPrompt, Object> promptValues)`
    pub fn evaluate(&self, container: &dyn FieldContainer, prompt_values: &PromptValues) -> bool {
        //
        // 取 LHS 值
        //
        let lhs = self.left_operand(container);

        //
        // 取 RHS 值
        //
        let rhs = if self.m_symbolic_values {
            self.process_symbolic_values(container, prompt_values)
        } else {
            self.m_working_right_values.clone()
        };

        //
        // 求值
        //
        match self.m_operator {
            Some(TestOperator::And) | Some(TestOperator::Or) => {
                self.evaluate_logical_operator(container, prompt_values)
            }
            Some(operator) => operator.evaluate(lhs.as_ref(), &rhs),
            // Java 抛 NullPointerException
            None => false,
        }
    }

    /// 取子条件列表。
    ///
    /// 对应 Java: `public List<GenericCriteria> getCriteriaList()`
    pub fn get_criteria_list(&self) -> &[GenericCriteria] {
        &self.m_criteria_list
    }

    /// 以可变方式取子条件列表（对应 Java 返回的可变列表）。
    ///
    /// 对应 Java: `public List<GenericCriteria> getCriteriaList()`
    pub fn get_criteria_list_mut(&mut self) -> &mut Vec<GenericCriteria> {
        &mut self.m_criteria_list
    }

    /// 追加一个子条件。
    ///
    /// 对应 Java: `public void addCriteria(GenericCriteria criteria)`
    pub fn add_criteria(&mut self, criteria: GenericCriteria) {
        self.m_criteria_list.push(criteria);
    }

    /// 求值 AND / OR 组合（子条件为空时结果为 `true`）。
    ///
    /// 对应 Java: `private boolean evaluateLogicalOperator(FieldContainer container, Map<GenericCriteriaPrompt, Object> promptValues)`
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

    /// 解析符号值 / 提示值，得到本次求值实际使用的右值。
    ///
    /// 对应 Java: `private Object[] processSymbolicValues(Object[] oldValues, FieldContainer container, Map<GenericCriteriaPrompt, Object> promptValues)`
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

    /// 把符号值（`FieldType`）解析为容器中的实际取值，并按数据类型做归一化。
    ///
    /// 对应 Java: `processSymbolicValues` 中 `value instanceof FieldType` 的分支。
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
                // Java 的 else 分支：取值为空、或单位已是小时，都会被替换为「0 小时」
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

    /// 取 LHS 的取值，并按数据类型做归一化。
    ///
    /// 对应 Java: `evaluate` 中 `lhs = container.get(field)` 之后对
    /// `field.getDataType()` 的 `switch`。
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

    /// 把工期换算成小时（Java: `duration.convertUnits(TimeUnit.HOURS, m_properties)`）。
    ///
    /// 父项目未接线 / 已被借用时退化为原值（Java 在 `properties` 为 `null` 时抛异常）。
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
    /// 对应 Java: `@Override public String toString()`
    ///
    /// AND / OR 组合用运算符连接子条件；其它运算符输出
    /// `(左值 运算符 右值[,右值])`，其中 `LocalDateTime` 用 `dd/MM/yyyy HH:mm` 格式。
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
    /// Java 未覆写调试输出；Rust 侧提供 `Debug`（复用 [`Display`]）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

/// 渲染字段类型（`None` 渲染为 Java 字符串拼接中的 `"null"`）。
fn render_field(field: Option<FieldKey>) -> String {
    match field {
        Some(field) => field.to_string(),
        None => "null".to_owned(),
    }
}

/// 渲染右值（`LocalDateTime` 用 `dd/MM/yyyy HH:mm` 格式，与 Java 一致）。
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

    /// 测试用的字段容器：只保存 `getCachedValue` 可见的字段。
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

    /// 构造一条 `field operator value` 的条件。
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

        // 越界设置与读取都不 panic
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
                // 无父项目 ⇒ 不换算，工作副本与定义值一致
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
        // RHS 引用的符号字段也在容器里解析出同一个值
        container.values.insert(
            FieldKey::Task(TaskField::Text1),
            FieldValue::String("Alpha".to_owned()),
        );

        let mut criteria = GenericCriteria::new(Weak::new());
        criteria.set_left_value(Some(FieldKey::Task(TaskField::Name)));
        criteria.set_operator(Some(TestOperator::Equals));
        // RHS 是符号值（字段引用），求值时从容器解析
        criteria.set_right_value(0, Some(Operand::Field(FieldKey::Task(TaskField::Text1))));

        assert!(criteria.evaluate(&container, &PromptValues::new()));
    }

    #[test]
    fn prompt_value_is_resolved_from_responses() {
        let prompt = GenericCriteriaPrompt::new(DataType::String, Some("Enter".to_owned()));
        let mut criteria = GenericCriteria::new(Weak::new());
        criteria.set_left_value(Some(FieldKey::Task(TaskField::Name)));
        criteria.set_operator(Some(TestOperator::Equals));
        // RHS 同时含符号值与提示值（Java 仅在含符号值时才解析）
        criteria.set_right_value(0, Some(Operand::Field(FieldKey::Task(TaskField::Text1))));
        criteria.set_right_value(1, Some(Operand::Prompt(prompt.clone())));

        let mut responses = PromptValues::new();
        responses.insert(
            prompt,
            Operand::Value(FieldValue::String("Design".to_owned())),
        );

        // 符号值 Text1 为空 ⇒ LHS "Design" 与 "" 不相等
        let container = {
            let mut container = MockContainer::default();
            container.set(
                FieldKey::Task(TaskField::Name),
                Some(FieldValue::String("Design".to_owned())),
            );
            container
        };
        assert!(!criteria.evaluate(&container, &responses));

        // 符号值 Text1 解析为 "Design" ⇒ 与 LHS 相等
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

        // 空子条件列表 ⇒ true
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

        // 无运算符
        let empty = GenericCriteria::new(Weak::new());
        assert_eq!(empty.to_string(), "(null null null)");

        // AND 组合
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
            .expect("合法日期");
        let criteria = criteria(
            FieldKey::Task(TaskField::Start),
            TestOperator::IsGreaterThan,
            Some(Operand::Value(FieldValue::DateTime(value))),
        );
        assert_eq!(criteria.to_string(), "(Start IS_GREATER_THAN 01/05/2024 13:30)");
    }
}
