//! 对应 Java: `org/mpxj/Filter.java`
//!
//! 跳过/未移植的 Java 方法与原因：
//!
//! | Java | Rust |
//! |---|---|
//! | 隐式默认构造方法 | [`Filter::new`] |
//! | `public void setID(Integer id)` / `public Integer getID()` | [`Filter::set_id`] / [`Filter::get_id`] |
//! | `public void setName(String name)` / `public String getName()` | [`Filter::set_name`] / [`Filter::get_name`] |
//! | `public void setShowRelatedSummaryRows(boolean)` / `public boolean getShowRelatedSummaryRows()` | [`Filter::set_show_related_summary_rows`] / [`Filter::get_show_related_summary_rows`] |
//! | `public void setCriteria(GenericCriteria criteria)` / `public GenericCriteria getCriteria()` | [`Filter::set_criteria`] / [`Filter::get_criteria`] |
//! | `public boolean isTaskFilter()` / `public void setIsTaskFilter(boolean flag)` | [`Filter::is_task_filter`] / [`Filter::set_is_task_filter`] |
//! | `public boolean isResourceFilter()` / `public void setIsResourceFilter(boolean flag)` | [`Filter::is_resource_filter`] / [`Filter::set_is_resource_filter`] |
//! | `public boolean evaluate(FieldContainer container, Map<GenericCriteriaPrompt, Object> promptValues)` | [`Filter::evaluate`]（改为泛型 + `Any` 向下转型，见下） |
//! | `public void setPrompts(List<GenericCriteriaPrompt> prompts)` / `public List<GenericCriteriaPrompt> getPrompts()` | [`Filter::set_prompts`] / [`Filter::get_prompts`] |
//! | `@Override public String toString()` | [`Display`] |
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//!
//! # 与 Java 的差异
//!
//! 1. **`container instanceof Task` → `Any` 向下转型**：Java 在求值失败且开启了
//!    「显示相关摘要行」时，会判断容器是否为 `Task` 并递归测试其子任务。Rust 的 trait
//!    对象无法直接向下转型，因此 [`Filter::evaluate`] 改为**泛型**接收 `&C`
//!    （`C: FieldContainer + 'static`），内部转成 `&dyn Any` 后
//!    `downcast_ref::<Task>()`，与 Java 的判定等价。调用方传 `&Task`、
//!    `&Resource`、`&ResourceAssignment` 均可（后两者走「非 Task」分支）。
//! 2. **`prompts`**：Java 的 `m_prompts` 默认为 `null`，`getPrompts()` 可能返回 `null`；
//!    Rust 侧用 `Vec<GenericCriteriaPrompt>`（默认空表），[`Filter::get_prompts`] 返回空切片。
//! 3. **`promptValues`**：Java 允许传 `null`；Rust 侧传空表即可（等价于「查不到应答」）。
//! 4. **`equals` / `hashCode`**：Java 未覆写（对象同一性）；Rust 侧不实现
//!    `PartialEq` / `Eq` / `Hash`。
//! 5. **字段遍历时的借用失败**：递归测试子任务时以 `try_borrow` 取得子任务；
//!    若子任务已被可变借用（Java 无此情形），该子任务按「不匹配」处理，不 panic。

use std::any::Any;
use std::fmt::{self, Display, Formatter};

use crate::field_container::FieldContainer;
use crate::generic_criteria::{GenericCriteria, PromptValues};
use crate::generic_criteria_prompt::GenericCriteriaPrompt;
use crate::task::Task;

/// 应用于任务视图或资源视图的筛选器。
///
/// 对应 Java: `public final class Filter`
pub struct Filter {
    /// 筛选器的唯一 ID。
    ///
    /// 对应 Java: `private Integer m_id;`
    m_id: Option<i32>,
    /// 筛选器名称。
    ///
    /// 对应 Java: `private String m_name;`
    m_name: Option<String>,
    /// 是否为任务筛选器。
    ///
    /// 对应 Java: `private boolean m_isTaskFilter;`
    m_is_task_filter: bool,
    /// 是否为资源筛选器。
    ///
    /// 对应 Java: `private boolean m_isResourceFilter;`
    m_is_resource_filter: bool,
    /// 是否显示相关摘要行。
    ///
    /// 对应 Java: `private boolean m_showRelatedSummaryRows;`
    m_show_related_summary_rows: bool,
    /// 筛选条件。
    ///
    /// 对应 Java: `private GenericCriteria m_criteria;`
    m_criteria: Option<GenericCriteria>,
    /// 本筛选器需要的提示。
    ///
    /// 对应 Java: `private List<GenericCriteriaPrompt> m_prompts;`
    m_prompts: Vec<GenericCriteriaPrompt>,
}

impl Filter {
    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法（Java 侧未显式声明构造方法）。
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

    /// 设置筛选器的唯一 ID。
    ///
    /// 对应 Java: `public void setID(Integer id)`
    pub fn set_id(&mut self, id: Option<i32>) {
        self.m_id = id;
    }

    /// 取筛选器的唯一 ID。
    ///
    /// 对应 Java: `public Integer getID()`
    pub fn get_id(&self) -> Option<i32> {
        self.m_id
    }

    /// 设置筛选器名称。
    ///
    /// 对应 Java: `public void setName(String name)`
    pub fn set_name(&mut self, name: Option<String>) {
        self.m_name = name;
    }

    /// 取筛选器名称。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.m_name.as_deref()
    }

    /// 设置「显示相关摘要行」标志。
    ///
    /// 对应 Java: `public void setShowRelatedSummaryRows(boolean showRelatedSummaryRows)`
    pub fn set_show_related_summary_rows(&mut self, show_related_summary_rows: bool) {
        self.m_show_related_summary_rows = show_related_summary_rows;
    }

    /// 取「显示相关摘要行」标志。
    ///
    /// 对应 Java: `public boolean getShowRelatedSummaryRows()`
    pub fn get_show_related_summary_rows(&self) -> bool {
        self.m_show_related_summary_rows
    }

    /// 设置筛选条件。
    ///
    /// 对应 Java: `public void setCriteria(GenericCriteria criteria)`
    pub fn set_criteria(&mut self, criteria: Option<GenericCriteria>) {
        self.m_criteria = criteria;
    }

    /// 取筛选条件。
    ///
    /// 对应 Java: `public GenericCriteria getCriteria()`
    pub fn get_criteria(&self) -> Option<&GenericCriteria> {
        self.m_criteria.as_ref()
    }

    /// 以可变方式取筛选条件（Rust 特有，便于就地修改条件）。
    ///
    /// 对应 Java: `public GenericCriteria getCriteria()`
    pub fn get_criteria_mut(&mut self) -> Option<&mut GenericCriteria> {
        self.m_criteria.as_mut()
    }

    /// 是否为任务筛选器。
    ///
    /// 对应 Java: `public boolean isTaskFilter()`
    pub fn is_task_filter(&self) -> bool {
        self.m_is_task_filter
    }

    /// 设置任务筛选器标志。
    ///
    /// 对应 Java: `public void setIsTaskFilter(boolean flag)`
    pub fn set_is_task_filter(&mut self, flag: bool) {
        self.m_is_task_filter = flag;
    }

    /// 是否为资源筛选器。
    ///
    /// 对应 Java: `public boolean isResourceFilter()`
    pub fn is_resource_filter(&self) -> bool {
        self.m_is_resource_filter
    }

    /// 设置资源筛选器标志。
    ///
    /// 对应 Java: `public void setIsResourceFilter(boolean flag)`
    pub fn set_is_resource_filter(&mut self, flag: bool) {
        self.m_is_resource_filter = flag;
    }

    /// 求值：容器的取值满足筛选条件时返回 `true`。
    ///
    /// 对应 Java: `public boolean evaluate(FieldContainer container, Map<GenericCriteriaPrompt, Object> promptValues)`
    ///
    /// 无筛选条件时恒为 `true`。条件不满足、但开启了「显示相关摘要行」且容器是 `Task` 时，
    /// 会递归测试其子任务，任一子任务匹配即返回 `true`。
    pub fn evaluate<C>(&self, container: &C, prompt_values: &PromptValues) -> bool
    where
        C: FieldContainer + 'static,
    {
        let mut result = true;

        if let Some(criteria) = &self.m_criteria {
            result = criteria.evaluate(container, prompt_values);

            //
            // 本行未通过，但它是摘要行且需要包含相关摘要行时，递归测试其子任务
            //
            if !result && self.m_show_related_summary_rows {
                let any: &dyn Any = container;
                if let Some(task) = any.downcast_ref::<Task>() {
                    for child in task.get_child_tasks() {
                        let matched = match child.try_borrow() {
                            Ok(child) => self.evaluate(&*child, prompt_values),
                            // Java 无此情形；避免重入借用 panic
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

    /// 设置本筛选器需要的提示。
    ///
    /// 对应 Java: `public void setPrompts(List<GenericCriteriaPrompt> prompts)`
    pub fn set_prompts(&mut self, prompts: Vec<GenericCriteriaPrompt>) {
        self.m_prompts = prompts;
    }

    /// 取本筛选器需要的提示。
    ///
    /// 对应 Java: `public List<GenericCriteriaPrompt> getPrompts()`
    ///
    /// Java 默认为 `null`；Rust 侧返回空切片。
    pub fn get_prompts(&self) -> &[GenericCriteriaPrompt] {
        &self.m_prompts
    }
}

impl Default for Filter {
    /// 对应 Java: 隐式默认构造方法，等价于 [`Filter::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl Display for Filter {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// 仅在存在筛选条件时输出 `criteria=...` 段。
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

    /// 构造一条「Name 等于 value」的条件。
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

        // 未开启「显示相关摘要行」⇒ 摘要行本身不匹配则失败
        assert!(!filter.evaluate(&*parent.borrow(), &PromptValues::new()));

        // 开启后递归测试子任务
        filter.set_show_related_summary_rows(true);
        assert!(filter.evaluate(&*parent.borrow(), &PromptValues::new()));

        // 子任务也不匹配时仍然失败
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
