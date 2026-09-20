//! 对应 Java: `org/mpxj/Relation.java`
//!
//! 两个任务之间的关系（前驱 → 后继）。
//!
//! # Java → Rust 方法对照
//!
//! | Java | Rust |
//! |---|---|
//! | `private Relation(Builder builder)` | [`Builder::build`] |
//! | `public RelationType getType()` | [`Relation::get_type`] |
//! | `public Duration getLag()` | [`Relation::get_lag`]（返回 `Option`，见下） |
//! | `public Task getPredecessorTask()` | [`Relation::get_predecessor_task`] |
//! | `public Task getSuccessorTask()` | [`Relation::get_successor_task`] |
//! | `@Override public Integer getUniqueID()` | [`Relation::get_unique_id`] |
//! | `@Override public void setUniqueID(Integer)` | [`Relation::set_unique_id`] |
//! | `public String getNotes()` | [`Relation::get_notes`] |
//! | `public boolean getDriving()` | [`Relation::get_driving`] |
//! | `public void setDriving(boolean)` | [`Relation::set_driving`] |
//! | `@Override public String toString()` | `Display`（另提供 `Debug`） |
//! | `implements ProjectEntityWithMutableUniqueID` | 实现 [`ProjectEntityWithUniqueID`] + [`ProjectEntityWithMutableUniqueID`] |
//! | `Builder.from(Relation)` | [`Builder::from`]（Rust 侧为关联函数） |
//! | `Builder.uniqueID / predecessorTask / successorTask / type / lag / notes / driving / build` | 同名 snake_case（`type` → `r#type`，与 crate 内既有 `r#type` 写法一致） |
//!
//! 说明：Java 的 `Relation.Builder` 是静态内部类，Rust 无法在类型内部声明类型，故与
//! [`crate::shift::Builder`] 的做法一致，落为模块级类型 [`Builder`]（`crate::relation::Builder`）。
//!
//! # 跳过的 Java 代码及原因
//!
//! | Java | 原因 |
//! |---|---|
//! | 构造方法中 `uniqueID == null` 时经 `m_successorTask.getParentFile()` → `ProjectConfig.getAutoRelationUniqueID()` / `getUniqueIdObjectSequence(Relation.class)` 的自动 unique ID 分配 | 未接线：Rust 侧完全由调用方经 [`Builder::unique_id`] 提供 | 依赖均已落地（`Task::get_parent_file`、`ProjectFile`、`ProjectConfig::get_auto_relation_unique_id`），缺的是把该逻辑接回 [`crate::relation_container::RelationContainer::add_predecessor`]（容器持有父文件与序号生成器）。 |
//! | `setUniqueID` 中的 `m_successorTask.getParentFile().getRelations().updateUniqueID(this, m_uniqueID, uniqueID)` | 未接线：Rust 侧容器按 unique ID 线性查找（见 [`crate::relation_container::RelationContainer`] 文件头），不需要回调维护索引；序号生成器同步由调用方经 `RelationContainer::update_unique_id` 完成 | 同上（依赖 `ProjectFile` / `Task` 均已落地），缺的是接线。 |
//! | 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable` | 本类未使用这些机制 |
//!
//! # 与 Java 的差异
//!
//! - **源 / 目标任务引用用 `Weak`**：Java 的 `m_predecessorTask` / `m_successorTask` 是强引用；
//!   Rust 侧 `Task` 与持有 `Rc<RefCell<Relation>>` 的 `RelationContainer` 之间若用强引用，
//!   会形成「任务 → 关系 → 任务」的 `Rc` 循环导致内存泄漏，因此改用
//!   `Weak<RefCell<Task>>`，由 [`Relation::get_predecessor_task`] /
//!   [`Relation::get_successor_task`] 在使用时升级，任务已释放时返回 `None`（Java 无此情形）。
//!   构造入口相应改为 [`Builder`]：任务以 `&Rc<RefCell<Task>>` 传入后降级为
//!   `Weak`，与 [`crate::shift::Shift`] / [`crate::shift_period::ShiftPeriod`] 的既有做法一致。
//!   `build()` 返回 `Relation` 值类型（`Relation` 自身不需要反向弱引用），由
//!   `RelationContainer` 负责包一层 `Rc<RefCell<..>>`。
//! - **`getLag()` 返回 `Option<Duration>`**：Java 的 `m_lag` 非空（构造方法把 `null` 替换为
//!   0 天）。Rust 侧用 `None` 表示「未显式设置」，在 lag 判定上与 Java 的 0 天等价；
//!   crate 内既有调用方 [`crate::cpm::microsoft_slack_calculator`] 也按 `Option<Duration>` 使用。
//! - **值相等语义**：Java 未覆写 `equals` / `hashCode`（即引用相等），容器依赖该语义，
//!   故 Rust 侧**不**派生 `PartialEq` / `Eq` / `Hash`。
//! - **`toString()` 中的任务**：Java 拼接 `Task.toString()`（输出任务字段内容）；Rust 侧改为
//!   输出任务的 unique ID（不依赖并行模块 `Task` 的 `Display` 实现）。
//! - **`Builder.from`**：Java 是实例方法（`new Builder().from(relation)`），Rust 侧为关联函数；
//!   Java 的 `from` 不复制 `m_driving`，Rust 侧保持一致。
//! - **`Builder.type(null)`**：Java 的 `type(RelationType)` 允许传 `null`（构造时会回落到
//!   `FINISH_START`，参与比较时不等）；Rust 的参数类型 `RelationType` 不可空，故没有对应写法。
//! - **`Builder.lag(null)`**：Java 的 `lag(Duration)` 允许传 `null`（构造时替换为 0 天，
//!   直接参与比较时会抛 `NullPointerException`）；Rust 侧 `lag(None)` 即「未设置」，等价于 0 天。

use std::cell::RefCell;
use std::cmp::Ordering;
use std::fmt::{self, Debug, Display, Formatter};
use std::rc::{Rc, Weak};

use crate::duration::Duration;
use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
use crate::relation_type::RelationType;
use crate::task::Task;
use crate::time_unit::TimeUnit;

/// 两个任务之间的关系。
///
/// 对应 Java: `public final class Relation implements ProjectEntityWithMutableUniqueID`
pub struct Relation {
    /// 对应 Java 字段 `private Integer m_uniqueID;`
    unique_id: Option<i32>,
    /// 对应 Java 字段 `private boolean m_driving;`
    driving: bool,
    /// 对应 Java 字段 `private final Task m_predecessorTask;`（Rust 侧为弱引用，见文件头）
    predecessor_task: Weak<RefCell<Task>>,
    /// 对应 Java 字段 `private final Task m_successorTask;`（Rust 侧为弱引用，见文件头）
    successor_task: Weak<RefCell<Task>>,
    /// 对应 Java 字段 `private final RelationType m_type;`
    relation_type: RelationType,
    /// 对应 Java 字段 `private final Duration m_lag;`（Rust 侧 `None` 等价于 Java 的 0 天）
    lag: Option<Duration>,
    /// 对应 Java 字段 `private final String m_notes;`
    notes: Option<String>,
}

impl Relation {
    /// 取关系类型。
    ///
    /// 对应 Java: `public RelationType getType()`
    pub fn get_type(&self) -> RelationType {
        self.relation_type
    }

    /// 取关系的滞后（lag）时长；`None` 表示未设置，等价于 Java 的 0 天。
    ///
    /// 对应 Java: `public Duration getLag()`
    ///
    /// 返回 `Option` 的原因见文件头（Java 侧为「非空，未设置时替换为 0 天」）。
    pub fn get_lag(&self) -> Option<Duration> {
        self.lag
    }

    /// 取关系的前驱任务；前驱任务已释放时返回 `None`。
    ///
    /// 对应 Java: `public Task getPredecessorTask()`
    pub fn get_predecessor_task(&self) -> Option<Rc<RefCell<Task>>> {
        self.predecessor_task.upgrade()
    }

    /// 取关系的后继任务；后继任务已释放时返回 `None`。
    ///
    /// 对应 Java: `public Task getSuccessorTask()`
    pub fn get_successor_task(&self) -> Option<Rc<RefCell<Task>>> {
        self.successor_task.upgrade()
    }

    /// 取关系的 unique ID。
    ///
    /// 对应 Java: `@Override public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.unique_id
    }

    /// 设置关系的 unique ID。
    ///
    /// 对应 Java: `@Override public void setUniqueID(Integer uniqueID)`
    ///
    /// Java 还会调用 `m_successorTask.getParentFile().getRelations().updateUniqueID(...)` 同步
    /// 容器的 unique ID 索引与序号生成器；该接线**尚未补回**（`ProjectFile` / `Task` 均已落地），
    /// Rust 侧容器改为按 unique ID 线性查找（无需维护索引），见文件头。
    pub fn set_unique_id(&mut self, unique_id: Option<i32>) {
        self.unique_id = unique_id;
    }

    /// 取关系上的备注。
    ///
    /// 对应 Java: `public String getNotes()`
    pub fn get_notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }

    /// 是否为驱动关系（仅在使用 `PrimaveraScheduler` 排程时由 MPXJ 内部填充）。
    ///
    /// 对应 Java: `public boolean getDriving()`
    pub fn get_driving(&self) -> bool {
        self.driving
    }

    /// 设置是否为驱动关系（MPXJ 内部使用）。
    ///
    /// 对应 Java: `public void setDriving(boolean driving)`
    pub fn set_driving(&mut self, driving: bool) {
        self.driving = driving;
    }
}

impl ProjectEntityWithUniqueID for Relation {
    /// 对应 Java: `@Override public Integer getUniqueID()`
    fn get_unique_id(&self) -> Option<i32> {
        Relation::get_unique_id(self)
    }
}

impl ProjectEntityWithMutableUniqueID for Relation {
    /// 对应 Java: `@Override public void setUniqueID(Integer uniqueID)`
    fn set_unique_id(&mut self, id: Option<i32>) {
        Relation::set_unique_id(self, id);
    }
}

impl Display for Relation {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 直接拼接两个 `Task` 的 `toString()`；Rust 侧以任务的 unique ID 代替（见文件头）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[Relation lag: {} type: {} {} -> {}]",
            self.lag.unwrap_or_else(zero_days),
            self.relation_type,
            describe_task(&self.predecessor_task),
            describe_task(&self.successor_task)
        )
    }
}

impl Debug for Relation {
    /// Java 侧只用 `toString()`；Rust 侧另提供结构化 `Debug`，任务引用输出其 unique ID
    /// （避免依赖并行模块 `Task` 的 `Debug` 实现）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Relation")
            .field("unique_id", &self.unique_id)
            .field("relation_type", &self.relation_type)
            .field("lag", &self.lag)
            .field("driving", &self.driving)
            .field("notes", &self.notes)
            .field("predecessor_task_unique_id", &task_unique_id(&self.predecessor_task))
            .field("successor_task_unique_id", &task_unique_id(&self.successor_task))
            .finish()
    }
}

/// 关系构造器。
///
/// 对应 Java: `public static class Relation.Builder`
///
/// # 与 Java 的差异
///
/// - Java 的 `Builder` 字段是**包级可见**，由 `RelationContainer` 直接读取（`builder.m_successorTask`
///   等）；Rust 没有包级可见性，这里用 `pub(crate)` 字段等价表达，读取方式与 Java 完全一致。
/// - Java 的方法原地修改并返回 `this`；Rust 侧沿用 [`crate::shift::Builder`] 的做法：
///   消耗自身并返回 `Self`。
/// - Java 允许 `type(null)` / `lag(null)`；Rust 侧 `type` 参数不可空，`lag(None)` 表示 0 天
///   （见文件头）。
#[derive(Debug)]
pub struct Builder {
    /// 对应 Java 字段 `Integer m_uniqueID;`
    pub(crate) unique_id: Option<i32>,
    /// 对应 Java 字段 `Task m_predecessorTask;`
    pub(crate) predecessor_task: Weak<RefCell<Task>>,
    /// 对应 Java 字段 `Task m_successorTask;`
    pub(crate) successor_task: Weak<RefCell<Task>>,
    /// 对应 Java 字段 `RelationType m_type = RelationType.FINISH_START;`
    pub(crate) relation_type: RelationType,
    /// 对应 Java 字段 `Duration m_lag = Duration.getInstance(0, TimeUnit.DAYS);`
    ///
    /// Rust 侧用 `None` 表达同一个默认值（0 天）。
    pub(crate) lag: Option<Duration>,
    /// 对应 Java 字段 `String m_notes;`
    pub(crate) notes: Option<String>,
    /// 对应 Java 字段 `boolean m_driving;`
    pub(crate) driving: bool,
}

impl Builder {
    /// 构造方法（各字段使用与 Java 相同的默认值）。
    ///
    /// 对应 Java: `Relation.Builder` 的隐式默认构造方法。
    pub fn new() -> Self {
        Self {
            unique_id: None,
            predecessor_task: Weak::new(),
            successor_task: Weak::new(),
            relation_type: RelationType::FinishStart,
            lag: None,
            notes: None,
            driving: false,
        }
    }

    /// 以已有关系的字段初始化构造器（不复制 `driving`，与 Java 一致）。
    ///
    /// 对应 Java: `public Builder from(Relation value)`
    pub fn from(value: &Relation) -> Self {
        Self {
            unique_id: value.unique_id,
            predecessor_task: value.predecessor_task.clone(),
            successor_task: value.successor_task.clone(),
            relation_type: value.relation_type,
            lag: value.lag,
            notes: value.notes.clone(),
            driving: false,
        }
    }

    /// 设置 unique ID。
    ///
    /// 对应 Java: `public Builder uniqueID(Integer value)`
    pub fn unique_id(mut self, value: Option<i32>) -> Self {
        self.unique_id = value;
        self
    }

    /// 设置前驱任务（Rust 侧接收 `Rc`，内部降级为 `Weak`）。
    ///
    /// 对应 Java: `public Builder predecessorTask(Task value)`
    pub fn predecessor_task(mut self, value: &Rc<RefCell<Task>>) -> Self {
        self.predecessor_task = Rc::downgrade(value);
        self
    }

    /// 设置后继任务（Rust 侧接收 `Rc`，内部降级为 `Weak`）。
    ///
    /// 对应 Java: `public Builder successorTask(Task value)`
    pub fn successor_task(mut self, value: &Rc<RefCell<Task>>) -> Self {
        self.successor_task = Rc::downgrade(value);
        self
    }

    /// 设置关系类型。
    ///
    /// 对应 Java: `public Builder type(RelationType value)`
    ///
    /// Java 方法名为 `type`，Rust 中 `type` 是关键字，故按本 crate 既有写法
    /// （见 `crate::project_calendar` 的 `r#type`）写作 `r#type`。
    pub fn r#type(mut self, value: RelationType) -> Self {
        self.relation_type = value;
        self
    }

    /// 设置滞后时长；`None` 表示 0 天（Java 的默认值）。
    ///
    /// 对应 Java: `public Builder lag(Duration value)`
    pub fn lag(mut self, value: Option<Duration>) -> Self {
        self.lag = value;
        self
    }

    /// 设置备注。
    ///
    /// 对应 Java: `public Builder notes(String value)`
    pub fn notes(mut self, value: Option<String>) -> Self {
        self.notes = value;
        self
    }

    /// 设置驱动关系标志。
    ///
    /// 对应 Java: `public Builder driving(boolean value)`
    pub fn driving(mut self, value: bool) -> Self {
        self.driving = value;
        self
    }

    /// 构造 [`Relation`] 实例。
    ///
    /// 对应 Java: `public Relation build()`
    ///
    /// Java 的构造方法会在这里做 unique ID 的自动分配与容器同步，Rust 侧的取舍见文件头。
    pub fn build(self) -> Relation {
        Relation {
            unique_id: self.unique_id,
            driving: self.driving,
            predecessor_task: self.predecessor_task,
            successor_task: self.successor_task,
            relation_type: self.relation_type,
            lag: self.lag,
            notes: self.notes,
        }
    }
}

impl Default for Builder {
    /// 对应 Java: `Relation.Builder` 的隐式默认构造方法。
    fn default() -> Self {
        Self::new()
    }
}

/// 0 天（Java 中 `Duration.getInstance(0, TimeUnit.DAYS)` 对应的默认 lag）。
fn zero_days() -> Duration {
    Duration::get_instance_int(0, TimeUnit::Days)
}

/// 读取弱引用任务的 unique ID（任务已释放或正被可变借用时返回 `None`）。
fn task_unique_id(task: &Weak<RefCell<Task>>) -> Option<i32> {
    task.upgrade()
        .and_then(|task| task.try_borrow().ok().and_then(|task| task.get_unique_id()))
}

/// 生成 Java `Relation.toString()` 中任务引用的文字形式（Java 用 `Task.toString()`）。
fn describe_task(task: &Weak<RefCell<Task>>) -> String {
    match task_unique_id(task) {
        Some(unique_id) => format!("Task {unique_id}"),
        None => "null".to_owned(),
    }
}

/// 比较两个 lag：与 Java 的 `Duration.compareTo(...) == 0` 等价
/// （单位不同时先换算，见 [`Duration`] 的 `Ord`）。
pub(crate) fn lag_matches(lhs: Option<Duration>, rhs: Option<Duration>) -> bool {
    lhs.unwrap_or_else(zero_days).cmp(&rhs.unwrap_or_else(zero_days)) == Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个独立的任务实例（不绑定父项目文件）。
    fn task() -> Rc<RefCell<Task>> {
        Rc::new(RefCell::new(Task::new()))
    }

    #[test]
    fn builder_defaults_match_java() {
        let relation = Builder::new().build();

        assert_eq!(relation.get_type(), RelationType::FinishStart);
        // Java: Builder 的 lag 默认为 0 天；Rust 侧等价表达为 None
        assert_eq!(relation.get_lag(), None);
        assert_eq!(relation.get_notes(), None);
        assert!(!relation.get_driving());
        assert_eq!(relation.get_unique_id(), None);
        assert!(relation.get_predecessor_task().is_none());
        assert!(relation.get_successor_task().is_none());
    }

    #[test]
    fn builder_sets_all_fields() {
        let predecessor = task();
        let successor = task();
        let lag = Duration::get_instance(2.0, TimeUnit::Days);

        let mut relation = Builder::new()
            .unique_id(Some(7))
            .predecessor_task(&predecessor)
            .successor_task(&successor)
            .r#type(RelationType::StartStart)
            .lag(Some(lag))
            .notes(Some("备注".to_owned()))
            .driving(true)
            .build();

        assert_eq!(relation.get_unique_id(), Some(7));
        assert_eq!(relation.get_type(), RelationType::StartStart);
        assert_eq!(relation.get_lag(), Some(lag));
        assert_eq!(relation.get_notes(), Some("备注"));
        assert!(relation.get_driving());
        assert!(Rc::ptr_eq(
            &relation.get_predecessor_task().expect("前驱任务仍在"),
            &predecessor
        ));
        assert!(Rc::ptr_eq(
            &relation.get_successor_task().expect("后继任务仍在"),
            &successor
        ));

        // setDriving / setUniqueID 可写
        relation.set_driving(false);
        relation.set_unique_id(Some(9));
        assert!(!relation.get_driving());
        assert_eq!(relation.get_unique_id(), Some(9));
    }

    #[test]
    fn from_copies_fields_but_not_driving() {
        let predecessor = task();
        let successor = task();
        let original = Builder::new()
            .unique_id(Some(3))
            .predecessor_task(&predecessor)
            .successor_task(&successor)
            .r#type(RelationType::FinishFinish)
            .lag(Some(Duration::get_instance(1.0, TimeUnit::Hours)))
            .notes(Some("notes".to_owned()))
            .driving(true)
            .build();

        let copy = Builder::from(&original).build();

        assert_eq!(copy.get_unique_id(), Some(3));
        assert_eq!(copy.get_type(), RelationType::FinishFinish);
        assert_eq!(copy.get_lag(), Some(Duration::get_instance(1.0, TimeUnit::Hours)));
        assert_eq!(copy.get_notes(), Some("notes"));
        // Java 的 from 不复制 m_driving
        assert!(!copy.get_driving());
        assert!(copy.get_predecessor_task().is_some());
        assert!(copy.get_successor_task().is_some());
    }

    #[test]
    fn dropped_task_yields_none() {
        let predecessor = task();
        let successor = task();
        let relation = Builder::new()
            .predecessor_task(&predecessor)
            .successor_task(&successor)
            .build();

        drop(predecessor);
        assert!(relation.get_predecessor_task().is_none());
        assert!(relation.get_successor_task().is_some());
    }

    #[test]
    fn entity_traits_are_implemented() {
        let mut relation = Builder::new().build();

        ProjectEntityWithMutableUniqueID::set_unique_id(&mut relation, Some(5));
        assert_eq!(ProjectEntityWithUniqueID::get_unique_id(&relation), Some(5));
        assert_eq!(relation.get_unique_id(), Some(5));
    }

    #[test]
    fn display_and_debug_describe_relation() {
        let predecessor = task();
        predecessor.borrow_mut().set_unique_id(Some(1));
        let successor = task();
        successor.borrow_mut().set_unique_id(Some(2));

        let relation = Builder::new()
            .predecessor_task(&predecessor)
            .successor_task(&successor)
            .r#type(RelationType::StartFinish)
            .lag(Some(Duration::get_instance(3.0, TimeUnit::Days)))
            .build();

        // Java: "[Relation lag: 3.0d type: SF Task 1 -> Task 2]"
        assert_eq!(
            relation.to_string(),
            "[Relation lag: 3d type: SF Task 1 -> Task 2]"
        );
        assert!(format!("{relation:?}").contains("relation_type: StartFinish"));
        assert!(format!("{relation:?}").contains("predecessor_task_unique_id: Some(1)"));
    }

    #[test]
    fn lag_matches_follows_java_compare_to() {
        // Java: Duration.compareTo == 0，单位不同会先换算
        assert!(lag_matches(None, None));
        assert!(lag_matches(
            Some(Duration::get_instance_int(0, TimeUnit::Days)),
            None
        ));
        assert!(lag_matches(
            Some(Duration::get_instance(1.0, TimeUnit::Days)),
            Some(Duration::get_instance(8.0, TimeUnit::Hours))
        ));
        assert!(!lag_matches(
            Some(Duration::get_instance(2.0, TimeUnit::Days)),
            Some(Duration::get_instance(8.0, TimeUnit::Hours))
        ));
    }
}
