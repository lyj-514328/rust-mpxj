//! 对应 Java: `org/mpxj/GraphicalIndicator.java`
//!
//! 跳过/未移植的 Java 方法与原因：
//!
//! | Java | Rust |
//! |---|---|
//! | `GraphicalIndicator(FieldType fieldType)` | [`GraphicalIndicator::new`]（Java 侧为包内可见，Rust 侧必须公开才可构造，见下） |
//! | `public int evaluate(FieldContainer container)` | [`GraphicalIndicator::evaluate`]（改为泛型 + `Any` 向下转型，见下） |
//! | `public FieldType getFieldType()` | [`GraphicalIndicator::get_field_type`]（`FieldType` → [`FieldKey`]） |
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
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//! 另外 Java 侧使用的 `NumberHelper.getInt` 在 Rust 侧直接以
//! [`Option::unwrap_or`] 表达（`null` → 0），行为一致。
//!
//! # 与 Java 的差异
//!
//! 1. **`instanceof Task` → `Any` 向下转型**：Java 的 `evaluate` 用
//!    `container instanceof Task` 决定取哪一组条件（项目摘要行 / 摘要行 / 非摘要行），
//!    并用 `Task.getUniqueID()` / `Task.getSummary()` 判定行类型。Rust 的 trait 对象无法
//!    直接向下转型，因此 [`GraphicalIndicator::evaluate`] 改为**泛型** receive
//!    `&C`（`C: FieldContainer + 'static`），内部先转成 `&dyn Any` 再
//!    `downcast_ref::<Task>()`，语义与 Java 完全一致；非 `Task` 容器一律使用非摘要行条件
//!    （与 Java 的 `else` 分支一致）。
//! 2. **构造方法可见性**：Java 的构造方法是包内可见（由 `CustomField` 调用）；本 crate 的
//!    [`crate::custom_field::CustomField`] 在移植时跳过了 `GraphicalIndicator` 字段，
//!    因此 Rust 侧把构造方法设为 `pub`，由调用方自行创建。
//! 3. **条件的持有方式**：Java 的三个 `List` 分别是字段；Rust 侧同样是 `Vec`，并按本 crate
//!    的约定拆成只读切片访问器（[`GraphicalIndicator::get_non_summary_row_criteria`] 等）
//!    与 `add_*` 追加方法。Java 的 `getNonSummaryRowCriteria()` 直接返回可变列表，
//!    Rust 侧不额外提供 `_mut`（MPXJ 只用 `add_*` 追加）。

use std::any::Any;
use std::fmt::{self, Display, Formatter};

use crate::field_container::FieldContainer;
use crate::field_key::FieldKey;
use crate::graphical_indicator_criteria::GraphicalIndicatorCriteria;
use crate::task::Task;

/// 单个表格列的图形化指标（graphical indicator）定义。
///
/// 对应 Java: `public class GraphicalIndicator`
pub struct GraphicalIndicator {
    /// 本指标适用的字段。
    ///
    /// 对应 Java: `private final FieldType m_fieldType;`
    m_field_type: FieldKey,

    /// 是否以图形化指标替代实际取值显示。
    ///
    /// 对应 Java: `private boolean m_displayGraphicalIndicators;`
    m_display_graphical_indicators: bool,

    /// 摘要行是否继承非摘要行的条件。
    ///
    /// 对应 Java: `private boolean m_summaryRowsInheritFromNonSummaryRows;`
    m_summary_rows_inherit_from_non_summary_rows: bool,

    /// 项目摘要行是否继承摘要行的条件。
    ///
    /// 对应 Java: `private boolean m_projectSummaryInheritsFromSummaryRows;`
    m_project_summary_inherits_from_summary_rows: bool,

    /// 是否把数据值显示为工具提示。
    ///
    /// 对应 Java: `private boolean m_showDataValuesInToolTips;`
    m_show_data_values_in_tool_tips: bool,

    /// 非摘要行的条件。
    ///
    /// 对应 Java: `private final List<GraphicalIndicatorCriteria> m_nonSummaryRowCriteria = new ArrayList<>();`
    m_non_summary_row_criteria: Vec<GraphicalIndicatorCriteria>,

    /// 摘要行的条件。
    ///
    /// 对应 Java: `private final List<GraphicalIndicatorCriteria> m_summaryRowCriteria = new ArrayList<>();`
    m_summary_row_criteria: Vec<GraphicalIndicatorCriteria>,

    /// 项目摘要行的条件。
    ///
    /// 对应 Java: `private final List<GraphicalIndicatorCriteria> m_projectSummaryCriteria = new ArrayList<>();`
    m_project_summary_criteria: Vec<GraphicalIndicatorCriteria>,
}

impl GraphicalIndicator {
    /// 构造方法。
    ///
    /// 对应 Java: `GraphicalIndicator(FieldType fieldType)`
    ///
    /// Java 侧为包内可见；Rust 侧见文件头说明（改为 `pub`）。
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

    /// 判断是否应显示图形化指标，返回 `-1` 表示不显示任何指标。
    ///
    /// 对应 Java: `public int evaluate(FieldContainer container)`
    ///
    /// 未命中任何条件时返回默认值 `0`。
    pub fn evaluate<C>(&self, container: &C) -> i32
    where
        C: FieldContainer + 'static,
    {
        //
        // 第一步：确定本次应使用的条件集合
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
            // 资源摘要行也可能存在，但目前无法判定，与 Java 一致地使用非摘要行条件
            None => &self.m_non_summary_row_criteria,
        };

        //
        // 第二步：逐个求值，取第一个命中的结果
        //
        let mut result = -1;
        for item in criteria {
            result = item.evaluate_indicator(container);
            if result != -1 {
                break;
            }
        }

        //
        // 第三步：仍未命中时返回默认值 0
        //
        if result == -1 {
            result = 0;
        }

        result
    }

    /// 取本指标适用的字段。
    ///
    /// 对应 Java: `public FieldType getFieldType()`
    pub fn get_field_type(&self) -> FieldKey {
        self.m_field_type
    }

    /// 取「是否显示图形化指标」标志。
    ///
    /// 对应 Java: `public boolean getDisplayGraphicalIndicators()`
    pub fn get_display_graphical_indicators(&self) -> bool {
        self.m_display_graphical_indicators
    }

    /// 设置「是否显示图形化指标」标志。
    ///
    /// 对应 Java: `public void setDisplayGraphicalIndicators(boolean displayGraphicalIndicators)`
    pub fn set_display_graphical_indicators(&mut self, display_graphical_indicators: bool) {
        self.m_display_graphical_indicators = display_graphical_indicators;
    }

    /// 取非摘要行的条件。
    ///
    /// 对应 Java: `public List<GraphicalIndicatorCriteria> getNonSummaryRowCriteria()`
    pub fn get_non_summary_row_criteria(&self) -> &[GraphicalIndicatorCriteria] {
        &self.m_non_summary_row_criteria
    }

    /// 取项目摘要行的条件。
    ///
    /// 对应 Java: `public List<GraphicalIndicatorCriteria> getProjectSummaryCriteria()`
    pub fn get_project_summary_criteria(&self) -> &[GraphicalIndicatorCriteria] {
        &self.m_project_summary_criteria
    }

    /// 取摘要行的条件。
    ///
    /// 对应 Java: `public List<GraphicalIndicatorCriteria> getSummaryRowCriteria()`
    pub fn get_summary_row_criteria(&self) -> &[GraphicalIndicatorCriteria] {
        &self.m_summary_row_criteria
    }

    /// 取「项目摘要行是否继承摘要行条件」标志。
    ///
    /// 对应 Java: `public boolean getProjectSummaryInheritsFromSummaryRows()`
    pub fn get_project_summary_inherits_from_summary_rows(&self) -> bool {
        self.m_project_summary_inherits_from_summary_rows
    }

    /// 设置「项目摘要行是否继承摘要行条件」标志。
    ///
    /// 对应 Java: `public void setProjectSummaryInheritsFromSummaryRows(boolean projectSummaryInheritsFromSummaryRows)`
    pub fn set_project_summary_inherits_from_summary_rows(
        &mut self,
        project_summary_inherits_from_summary_rows: bool,
    ) {
        self.m_project_summary_inherits_from_summary_rows =
            project_summary_inherits_from_summary_rows;
    }

    /// 取「摘要行是否继承非摘要行条件」标志。
    ///
    /// 对应 Java: `public boolean getSummaryRowsInheritFromNonSummaryRows()`
    pub fn get_summary_rows_inherit_from_non_summary_rows(&self) -> bool {
        self.m_summary_rows_inherit_from_non_summary_rows
    }

    /// 设置「摘要行是否继承非摘要行条件」标志。
    ///
    /// 对应 Java: `public void setSummaryRowsInheritFromNonSummaryRows(boolean summaryRowsInheritFromNonSummaryRows)`
    pub fn set_summary_rows_inherit_from_non_summary_rows(
        &mut self,
        summary_rows_inherit_from_non_summary_rows: bool,
    ) {
        self.m_summary_rows_inherit_from_non_summary_rows =
            summary_rows_inherit_from_non_summary_rows;
    }

    /// 取「是否把数据值显示为工具提示」标志。
    ///
    /// 对应 Java: `public boolean getShowDataValuesInToolTips()`
    pub fn get_show_data_values_in_tool_tips(&self) -> bool {
        self.m_show_data_values_in_tool_tips
    }

    /// 设置「是否把数据值显示为工具提示」标志。
    ///
    /// 对应 Java: `public void setShowDataValuesInToolTips(boolean showDataValuesInToolTips)`
    pub fn set_show_data_values_in_tool_tips(&mut self, show_data_values_in_tool_tips: bool) {
        self.m_show_data_values_in_tool_tips = show_data_values_in_tool_tips;
    }

    /// 追加非摘要行的条件。
    ///
    /// 对应 Java: `public void addNonSummaryRowCriteria(GraphicalIndicatorCriteria criteria)`
    pub fn add_non_summary_row_criteria(&mut self, criteria: GraphicalIndicatorCriteria) {
        self.m_non_summary_row_criteria.push(criteria);
    }

    /// 追加摘要行的条件。
    ///
    /// 对应 Java: `public void addSummaryRowCriteria(GraphicalIndicatorCriteria criteria)`
    pub fn add_summary_row_criteria(&mut self, criteria: GraphicalIndicatorCriteria) {
        self.m_summary_row_criteria.push(criteria);
    }

    /// 追加项目摘要行的条件。
    ///
    /// 对应 Java: `public void addProjectSummaryCriteria(GraphicalIndicatorCriteria criteria)`
    pub fn add_project_summary_criteria(&mut self, criteria: GraphicalIndicatorCriteria) {
        self.m_project_summary_criteria.push(criteria);
    }
}

impl Display for GraphicalIndicator {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// 与 Java 一致地逐行输出（Java 用 `pw.println`，故输出含尾随换行）。
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
    /// Java 未覆写调试输出；Rust 侧提供 `Debug`（复用 [`Display`]）。
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

    /// 测试用的字段容器（非 `Task`，对应 Java 中 `instanceof Task` 为假的路径）。
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

    /// 构造一条「Name 等于 value 时显示 indicator 号指标」的条件。
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

        // Java: 全部未命中时 result = 0
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

        // unique ID 为 0（未设置）⇒ 项目摘要行
        assert_eq!(indicator.evaluate(&*task.borrow()), 3);

        // 普通行 ⇒ 非摘要行条件
        task.borrow_mut().set_unique_id(Some(1));
        assert_eq!(indicator.evaluate(&*task.borrow()), 1);

        // 摘要行 ⇒ 摘要行条件（默认不继承非摘要行条件）
        task.borrow_mut().set_summary(true);
        assert_eq!(indicator.evaluate(&*task.borrow()), 2);

        // 开启继承后摘要行改用非摘要行条件
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

        // 项目摘要行继承摘要行（而摘要行不继承非摘要行）⇒ 摘要行条件
        indicator.set_project_summary_inherits_from_summary_rows(true);
        assert_eq!(indicator.evaluate(&*task.borrow()), 2);

        // 两级继承都开启 ⇒ 非摘要行条件
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
