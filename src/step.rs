//! 对应 Java: `org/mpxj/Step.java`
//!
//! 跳过/未移植的 Java 方法与原因：
//!
//! | Java | Rust |
//! |---|---|
//! | `private Step(Builder builder)` | 私有函数 `Step::from_builder`（Java 的构造方法为私有，仅由 `Builder.build()` 调用） |
//! | `public Task getTask()` | [`Step::get_task`]（返回 `Option<Rc<RefCell<Task>>>`，见下） |
//! | `public Integer getUniqueID()` | [`Step::get_unique_id`] |
//! | `public String getName()` | [`Step::get_name`] |
//! | `public Double getPercentComplete()` | [`Step::get_percent_complete`] |
//! | `public Integer getSequenceNumber()` | [`Step::get_sequence_number`] |
//! | `public Double getWeight()` | [`Step::get_weight`] |
//! | `public String getDescription()` | [`Step::get_description`] |
//! | `public Notes getDescriptionObject()` | [`Step::get_description_object`]（返回 `Option<&dyn Notes>`） |
//! | `public boolean getComplete()` | [`Step::get_complete`] |
//! | `public static class Builder` | [`Builder`] |
//! | `public Builder(Task task)` | [`Builder::new`]（接收 `&Rc<RefCell<Task>>`，见下） |
//! | `public Builder from(Step value)` | [`Builder::from_step`]（改名以避免与 `From` trait 的 `from` 混淆） |
//! | `public Builder uniqueID(Integer value)` | [`Builder::unique_id`] |
//! | `public Builder name(String value)` | [`Builder::name`] |
//! | `public Builder percentComplete(Double value)` | [`Builder::percent_complete`] |
//! | `public Builder sequenceNumber(Integer value)` | [`Builder::sequence_number`] |
//! | `public Builder weight(Double value)` | [`Builder::weight`] |
//! | `public Builder description(Notes value)` | [`Builder::description`] |
//! | `public Builder description(String value)` | [`Builder::description_from_string`]（Rust 无重载） |
//! | `public Step build()` | [`Builder::build`] |
//! | `@Override public String toString()` | 未移植：Java 未覆写（`Object.toString()` 无实际信息），Rust 侧改为提供 `Debug` |
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//!
//! # 与 Java 的差异
//!
//! 1. **父任务引用**：Java 的 `m_task` 是强引用；Rust 侧存 `Weak<RefCell<Task>>`
//!    （`Task` 的步骤列表在 Java 中由 `Task` 持有，用强引用会形成 `Rc` 循环）。
//!    [`Step::get_task`] 在父任务已释放时返回 `None`。
//!    步骤列表的容器是 `crate::task::Task` 的 `TaskField.STEPS`（值类型
//!    [`crate::field_value::FieldValue::Steps`]），见 `Task::get_steps` / `Task::set_steps`。
//! 2. **`Notes` 字段**：Java 的 `m_description` 是 `Notes` 对象；Rust 的 `Notes` 是 trait，
//!    故用 `Rc<dyn Notes>` 承载（可共享、可复制引用），[`Step::get_description_object`]
//!    返回 `Option<&dyn Notes>`。
//! 3. **unique ID 分配**：Java 在构造方法中调用
//!    `m_task.getParentFile().getUniqueIdObjectSequence(Step.class).syncOrGetNext(m_uniqueID)`。
//!    Rust 侧同样经父任务的 `get_parent_file()` 取序号生成器；父任务未接线 / 已释放 /
//!    被借用时回落到 Builder 显式给出的 unique ID（Java 会抛 `NullPointerException`）。
//! 4. **Builder 的链式调用**：Java 的 setter 修改 `this` 并返回 `this`；Rust 侧改为
//!    **消费式**（`mut self -> Self`），`build()` 接收 `&self`，语义等价。
//! 5. **`NumberHelper.getDouble`**：`getComplete()` 中 Java 用
//!    `NumberHelper.getDouble(m_percentComplete) == 100.0`（`null` 视为 0）；Rust 侧用
//!    [`Option::unwrap_or`] 表达，行为一致。

use std::any::TypeId;
use std::cell::RefCell;
use std::fmt::{self, Debug, Formatter};
use std::rc::{Rc, Weak};

use crate::notes::{Notes, PlainNotes};
use crate::project_file::ProjectFile;
use crate::task::Task;

/// 活动（activity）步骤。
///
/// 对应 Java: `public final class Step`
///
/// 实现 `Clone`（Rust 特有）：Java 的 `Task.getSteps()` 返回字段表里的**活列表**，
/// 而 Rust 侧的列表装在 `Rc<RefCell<Vec<Step>>>` 里，[`crate::task::Task::get_steps`]
/// 返回其快照，因此需要浅拷贝单个步骤（`m_description` 与 `m_task` 均按引用共享，
/// 与 Java 的对象共享语义一致）。
#[derive(Clone)]
pub struct Step {
    /// 父任务（弱引用）。
    ///
    /// 对应 Java: `private final Task m_task;`
    m_task: Weak<RefCell<Task>>,

    /// unique ID。
    ///
    /// 对应 Java: `private final Integer m_uniqueID;`
    m_unique_id: Option<i32>,

    /// 步骤名称。
    ///
    /// 对应 Java: `private final String m_name;`
    m_name: Option<String>,

    /// 完成百分比。
    ///
    /// 对应 Java: `private final Double m_percentComplete;`
    m_percent_complete: Option<f64>,

    /// 序号。
    ///
    /// 对应 Java: `private final Integer m_sequenceNumber;`
    m_sequence_number: Option<i32>,

    /// 权重。
    ///
    /// 对应 Java: `private final Double m_weight;`
    m_weight: Option<f64>,

    /// 步骤说明。
    ///
    /// 对应 Java: `private final Notes m_description;`
    m_description: Option<Rc<dyn Notes>>,
}

impl Step {
    /// 由 Builder 构造（对应 Java 的私有构造方法 `private Step(Builder builder)`）。
    fn from_builder(builder: &Builder) -> Self {
        // Java: m_uniqueID = m_task.getParentFile().getUniqueIdObjectSequence(Step.class).syncOrGetNext(builder.m_uniqueID);
        let unique_id = builder
            .resolve_unique_id()
            .or(builder.m_unique_id);

        Self {
            m_task: builder.m_task.clone(),
            m_unique_id: unique_id,
            m_name: builder.m_name.clone(),
            m_percent_complete: builder.m_percent_complete,
            m_sequence_number: builder.m_sequence_number,
            m_weight: builder.m_weight,
            m_description: builder.m_description.clone(),
        }
    }

    /// 取父任务（父任务已释放时返回 `None`）。
    ///
    /// 对应 Java: `public Task getTask()`
    pub fn get_task(&self) -> Option<Rc<RefCell<Task>>> {
        self.m_task.upgrade()
    }

    /// 取 unique ID。
    ///
    /// 对应 Java: `public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.m_unique_id
    }

    /// 取步骤名称。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.m_name.as_deref()
    }

    /// 取完成百分比。
    ///
    /// 对应 Java: `public Double getPercentComplete()`
    pub fn get_percent_complete(&self) -> Option<f64> {
        self.m_percent_complete
    }

    /// 取序号。
    ///
    /// 对应 Java: `public Integer getSequenceNumber()`
    pub fn get_sequence_number(&self) -> Option<i32> {
        self.m_sequence_number
    }

    /// 取权重。
    ///
    /// 对应 Java: `public Double getWeight()`
    pub fn get_weight(&self) -> Option<f64> {
        self.m_weight
    }

    /// 取步骤说明的纯文本。
    ///
    /// 对应 Java: `public String getDescription()`
    ///
    /// Java 在说明为 `null` 时返回空字符串。
    pub fn get_description(&self) -> String {
        match &self.m_description {
            Some(description) => description.to_string(),
            None => String::new(),
        }
    }

    /// 取步骤说明对象。
    ///
    /// 对应 Java: `public Notes getDescriptionObject()`
    pub fn get_description_object(&self) -> Option<&dyn Notes> {
        self.m_description.as_deref()
    }

    /// 步骤是否已完成（完成百分比为 100）。
    ///
    /// 对应 Java: `public boolean getComplete()`
    pub fn get_complete(&self) -> bool {
        self.m_percent_complete.unwrap_or(0.0) == 100.0
    }
}

impl Debug for Step {
    /// Java 未覆写 `toString()`；Rust 侧提供 `Debug`，便于诊断
    /// （`dyn Notes` 不是 `Debug`，故说明一栏输出其纯文本）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Step")
            .field("unique_id", &self.m_unique_id)
            .field("name", &self.m_name)
            .field("percent_complete", &self.m_percent_complete)
            .field("sequence_number", &self.m_sequence_number)
            .field("weight", &self.m_weight)
            .field("description", &self.get_description())
            .field("task_is_set", &(self.m_task.strong_count() > 0))
            .finish()
    }
}

/// 步骤构造器。
///
/// 对应 Java: `public static class Builder`
pub struct Builder {
    /// 父任务（弱引用）。
    ///
    /// 对应 Java: `private final Task m_task;`
    m_task: Weak<RefCell<Task>>,

    /// 对应 Java: `private Integer m_uniqueID;`
    m_unique_id: Option<i32>,

    /// 对应 Java: `private String m_name;`
    m_name: Option<String>,

    /// 对应 Java: `private Double m_percentComplete;`
    m_percent_complete: Option<f64>,

    /// 对应 Java: `private Integer m_sequenceNumber;`
    m_sequence_number: Option<i32>,

    /// 对应 Java: `private Double m_weight;`
    m_weight: Option<f64>,

    /// 对应 Java: `private Notes m_description;`
    m_description: Option<Rc<dyn Notes>>,
}

impl Builder {
    /// 构造方法。
    ///
    /// 对应 Java: `public Builder(Task task)`
    pub fn new(task: &Rc<RefCell<Task>>) -> Self {
        Self {
            m_task: Rc::downgrade(task),
            m_unique_id: None,
            m_name: None,
            m_percent_complete: None,
            m_sequence_number: None,
            m_weight: None,
            m_description: None,
        }
    }

    /// 由已有步骤初始化构造器。
    ///
    /// 对应 Java: `public Builder from(Step value)`
    ///
    /// 改名以避开 `From` trait 的 `from`（Rust 侧命名约定）。
    pub fn from_step(mut self, value: &Step) -> Self {
        self.m_unique_id = value.m_unique_id;
        self.m_name = value.m_name.clone();
        self.m_percent_complete = value.m_percent_complete;
        self.m_sequence_number = value.m_sequence_number;
        self.m_weight = value.m_weight;
        self.m_description = value.m_description.clone();
        self
    }

    /// 设置 unique ID。
    ///
    /// 对应 Java: `public Builder uniqueID(Integer value)`
    pub fn unique_id(mut self, value: Option<i32>) -> Self {
        self.m_unique_id = value;
        self
    }

    /// 设置名称。
    ///
    /// 对应 Java: `public Builder name(String value)`
    pub fn name(mut self, value: Option<String>) -> Self {
        self.m_name = value;
        self
    }

    /// 设置完成百分比。
    ///
    /// 对应 Java: `public Builder percentComplete(Double value)`
    pub fn percent_complete(mut self, value: Option<f64>) -> Self {
        self.m_percent_complete = value;
        self
    }

    /// 设置序号。
    ///
    /// 对应 Java: `public Builder sequenceNumber(Integer value)`
    pub fn sequence_number(mut self, value: Option<i32>) -> Self {
        self.m_sequence_number = value;
        self
    }

    /// 设置权重。
    ///
    /// 对应 Java: `public Builder weight(Double value)`
    pub fn weight(mut self, value: Option<f64>) -> Self {
        self.m_weight = value;
        self
    }

    /// 设置说明对象。
    ///
    /// 对应 Java: `public Builder description(Notes value)`
    pub fn description(mut self, value: Option<Rc<dyn Notes>>) -> Self {
        self.m_description = value;
        self
    }

    /// 以纯文本设置说明（Java 中由 `description(String)` 重载承担，Rust 侧无重载故另起方法名）。
    ///
    /// 对应 Java: `public Builder description(String value)`
    ///
    /// Java: `m_description = value == null ? null : new Notes(value);`
    pub fn description_from_string(mut self, value: Option<String>) -> Self {
        self.m_description = value.map(|value| {
            let notes: Rc<dyn Notes> = Rc::new(PlainNotes::new(Some(value)));
            notes
        });
        self
    }

    /// 构造 [`Step`] 实例。
    ///
    /// 对应 Java: `public Step build()`
    pub fn build(&self) -> Step {
        Step::from_builder(self)
    }

    /// 经父任务的父文件分配 / 同步 unique ID。
    ///
    /// 对应 Java: `m_task.getParentFile().getUniqueIdObjectSequence(Step.class).syncOrGetNext(builder.m_uniqueID)`
    ///
    /// 任一环节不可用（任务未接线、父文件已释放或被借用）时返回 `None`。
    fn resolve_unique_id(&self) -> Option<i32> {
        let task = self.m_task.upgrade()?;
        let parent_file: Rc<RefCell<ProjectFile>> = task.try_borrow().ok()?.get_parent_file()?;
        let mut parent_file_ref = parent_file.try_borrow_mut().ok()?;

        Some(
            parent_file_ref
                .get_unique_id_object_sequence(TypeId::of::<Step>())
                .sync_or_get_next(self.m_unique_id),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建一个没有父文件的任务（任务图之外，用于测试步骤本身的行为）。
    fn standalone_task() -> Rc<RefCell<Task>> {
        Rc::new(RefCell::new(Task::new()))
    }

    #[test]
    fn builder_populates_fields() {
        let task = standalone_task();
        let step = Builder::new(&task)
            .unique_id(Some(5))
            .name(Some("Design".to_owned()))
            .percent_complete(Some(50.0))
            .sequence_number(Some(2))
            .weight(Some(1.5))
            .description_from_string(Some("Do it".to_owned()))
            .build();

        assert_eq!(step.get_unique_id(), Some(5));
        assert_eq!(step.get_name(), Some("Design"));
        assert_eq!(step.get_percent_complete(), Some(50.0));
        assert_eq!(step.get_sequence_number(), Some(2));
        assert_eq!(step.get_weight(), Some(1.5));
        assert_eq!(step.get_description(), "Do it");
        assert!(!step.get_complete());
    }

    #[test]
    fn get_task_returns_parent_task() {
        let task = standalone_task();
        let step = Builder::new(&task).build();

        assert!(step
            .get_task()
            .map(|found| Rc::ptr_eq(&found, &task))
            .unwrap_or(false));
    }

    #[test]
    fn complete_flag_requires_percent_complete_of_one_hundred() {
        let task = standalone_task();

        assert!(!Builder::new(&task).percent_complete(Some(99.0)).build().get_complete());
        assert!(Builder::new(&task).percent_complete(Some(100.0)).build().get_complete());
        // Java: NumberHelper.getDouble(null) == 0.0
        assert!(!Builder::new(&task).build().get_complete());
    }

    #[test]
    fn description_object_is_shared_when_provided() {
        let task = standalone_task();
        let description: Rc<dyn Notes> = Rc::new(PlainNotes::new(Some("Text".to_owned())));
        let step = Builder::new(&task)
            .description(Some(Rc::clone(&description)))
            .build();

        assert_eq!(step.get_description(), "Text");
        assert!(step
            .get_description_object()
            .map(|found| found.to_string() == "Text")
            .unwrap_or(false));
    }

    #[test]
    fn description_defaults_to_empty_string() {
        let task = standalone_task();
        let step = Builder::new(&task).build();

        assert_eq!(step.get_description(), "");
        assert!(step.get_description_object().is_none());
    }

    #[test]
    fn from_step_copies_fields() {
        let task = standalone_task();
        let source = Builder::new(&task)
            .unique_id(Some(1))
            .name(Some("A".to_owned()))
            .percent_complete(Some(100.0))
            .description_from_string(Some("d".to_owned()))
            .build();

        let copy = Builder::new(&task).from_step(&source).build();

        assert_eq!(copy.get_unique_id(), Some(1));
        assert_eq!(copy.get_name(), Some("A"));
        assert_eq!(copy.get_percent_complete(), Some(100.0));
        assert_eq!(copy.get_description(), "d");
        assert!(copy.get_complete());
    }

    #[test]
    fn unique_id_falls_back_without_parent_file() {
        // 父任务没有父文件 ⇒ 无法分配 unique ID，回落为显式给出的值
        let task = standalone_task();
        let step = Builder::new(&task).unique_id(Some(9)).build();
        assert_eq!(step.get_unique_id(), Some(9));

        // 未显式给出时为 None
        let step = Builder::new(&task).build();
        assert_eq!(step.get_unique_id(), None);
    }

    #[test]
    fn debug_reports_fields() {
        let task = standalone_task();
        let step = Builder::new(&task)
            .name(Some("Design".to_owned()))
            .build();

        let text = format!("{step:?}");
        assert!(text.contains("Design"));
        assert!(text.contains("task_is_set: true"));
    }
}
