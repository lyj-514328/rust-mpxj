//! 对应 Java: `org/mpxj/EventManager.java`
//!
//! 项目文件读写时产生的事件的订阅中心。
//!
//! # Java → Rust 方法对照
//!
//! | Java | Rust |
//! |---|---|
//! | `public void fireTaskReadEvent(Task task)` | [`EventManager::fire_task_read_event`] |
//! | `public void fireTaskWrittenEvent(Task task)` | [`EventManager::fire_task_written_event`] |
//! | `public void fireResourceReadEvent(Resource resource)` | [`EventManager::fire_resource_read_event`] |
//! | `public void fireResourceWrittenEvent(Resource resource)` | [`EventManager::fire_resource_written_event`] |
//! | `public void fireCalendarReadEvent(ProjectCalendar calendar)` | [`EventManager::fire_calendar_read_event`] |
//! | `public void fireAssignmentReadEvent(ResourceAssignment resourceAssignment)` | [`EventManager::fire_assignment_read_event`] |
//! | `public void fireAssignmentWrittenEvent(ResourceAssignment resourceAssignment)` | [`EventManager::fire_assignment_written_event`] |
//! | `public void fireRelationReadEvent(Relation relation)` | [`EventManager::fire_relation_read_event`] |
//! | `public void fireRelationWrittenEvent(Relation relation)` | [`EventManager::fire_relation_written_event`] |
//! | `public void fireCalendarWrittenEvent(ProjectCalendar calendar)` | [`EventManager::fire_calendar_written_event`] |
//! | `public void addProjectListener(ProjectListener listener)` | [`EventManager::add_project_listener`] |
//! | `public void addProjectListeners(List<ProjectListener> listeners)` | [`EventManager::add_project_listeners`] |
//! | `public void removeProjectListener(ProjectListener listener)` | [`EventManager::clear_project_listeners`]（签名不同，见下） |
//! | `private List<ProjectListener> m_projectListeners;` | 字段 `project_listeners: Vec<Box<dyn ProjectListener>>` |
//!
//! # 与 Java 的差异
//!
//! - **存储去掉「未初始化」状态**：Java 的 `m_projectListeners` 初始为 `null`，各 `fire*`
//!   与 `add` / `remove` 都要判空；Rust 侧用始终存在的 `Vec<Box<dyn ProjectListener>>`
//!   （Java 的 `List` → `Vec`），因此所有判空分支消失，空列表自然就是空操作。
//! - **`removeProjectListener` → [`EventManager::clear_project_listeners`]**：Java 的
//!   `List.remove(Object)` 按 `equals`（本处即引用身份）移除指定的那一个监听器。
//!   **真实阻塞原因**（非「依赖未落地」）：
//!   1. [`EventManager::add_project_listener`] 以 `Box<dyn ProjectListener>` **按值**接收监听器，
//!      注册者拿不回句柄；`dyn ProjectListener` 也不实现相等性，故无法表达「按身份移除」。
//!   2. 该签名与存储形态由既有的读取器接口固定：`crate::reader::project_reader::ProjectReader`
//!      的 `add_project_listener(Box<dyn ProjectListener>)`（`reader/abstract_project_reader.rs`、
//!      `mpp/mpp_reader.rs`、`mpx/mpx_reader.rs` 的实现与 `mpp/constraint_factory.rs` 的调用点）
//!      都按 `Box` 传递。
//!   3. 本仓已有 `SharedProjectListener`（`reader/abstract_project_reader.rs`，
//!      `Rc<RefCell<Box<dyn ProjectListener>>>` + `Rc::ptr_eq`）可表达身份，但它的内部 `Rc`
//!      是私有字段、没有暴露相等性访问器 —— 本文件无法据此
//!      实现 `ptr_eq` 比较；若为此在 `EventManager` 内另造一份同形态句柄，则与
//!      `SharedProjectListener` 形成两份并行实现（本工程明令避免的做法）。
//!   因此这里保留**清空全部监听器**这一语义明确的操作；需要移除单个监听器时，由调用方改为
//!   重建 `EventManager` 或收紧注册范围。
//! - **`fire_*` / 注册方法取 `&mut self`**：监听器方法取 `&mut self`（见
//!   [`crate::listener::project_listener::ProjectListener`]），遍历监听器列表需要可变借用，
//!   故这些方法都是 `&mut self`。对 [`crate::project_context::ProjectContext`] 而言要经
//!   `get_event_manager_mut()` 取得可变引用。
//! - **实体参数用共享句柄**：Java 传实体引用，Rust 侧统一为 `&Rc<RefCell<..>>`，
//!   理由见 [`crate::listener::project_listener`] 文件头。
//! - **未做 `mem::take` 换出监听器列表**：本类型只在 `&mut self` 下遍历监听器，监听器回调
//!   拿不到本 `EventManager`，不存在「回调中再次 `fire_*`」造成 Rust 别名冲突的可能，因此
//!   直接 `for listener in &mut self.project_listeners` 遍历即可；若将来把 `EventManager`
//!   放进 `Rc<RefCell<..>>` 并在回调中重入，冲突会发生在 `RefCell` 外层借用（运行时 panic），
//!   需要调用方自行 `mem::take` 后再重入，本类型无法在内部规避。
//!
//! # 跳过的 Java 代码及原因
//!
//! | Java | 原因 |
//! |---|---|
//! | 本类未使用反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable` | 无对应代码 |

use std::cell::RefCell;
use std::rc::Rc;

use crate::listener::project_listener::ProjectListener;
use crate::project_calendar::ProjectCalendar;
use crate::relation::Relation;
use crate::resource::Resource;
use crate::resource_assignment::ResourceAssignment;
use crate::task::Task;

/// 提供项目文件读写事件的订阅能力。
///
/// 对应 Java: `public class EventManager`
pub struct EventManager {
    /// 对应 Java 字段 `private List<ProjectListener> m_projectListeners;`
    ///
    /// Java 用 `List`，Rust 用 `Vec<Box<dyn ProjectListener>>`（Java 的 `List` → `Vec`）。
    project_listeners: Vec<Box<dyn ProjectListener>>,
}

impl Default for EventManager {
    /// 对应 Java: `EventManager` 的隐式无参构造方法。
    fn default() -> Self {
        Self::new()
    }
}

impl EventManager {
    /// 构造方法。
    ///
    /// 对应 Java: `EventManager` 的隐式无参构造方法（`m_projectListeners` 为 `null`；
    /// Rust 侧为空 `Vec`，语义等价，见文件头）。
    pub fn new() -> Self {
        Self {
            project_listeners: Vec::new(),
        }
    }

    /// 通知监听器：已从项目文件中读入一个任务。
    ///
    /// 对应 Java: `public void fireTaskReadEvent(Task task)`
    pub fn fire_task_read_event(&mut self, task: &Rc<RefCell<Task>>) {
        for listener in &mut self.project_listeners {
            listener.task_read(task);
        }
    }

    /// 通知监听器：已将任务写出到项目文件。
    ///
    /// 对应 Java: `public void fireTaskWrittenEvent(Task task)`
    pub fn fire_task_written_event(&mut self, task: &Rc<RefCell<Task>>) {
        for listener in &mut self.project_listeners {
            listener.task_written(task);
        }
    }

    /// 通知监听器：已从项目文件中读入一个资源。
    ///
    /// 对应 Java: `public void fireResourceReadEvent(Resource resource)`
    pub fn fire_resource_read_event(&mut self, resource: &Rc<RefCell<Resource>>) {
        for listener in &mut self.project_listeners {
            listener.resource_read(resource);
        }
    }

    /// 通知监听器：已将资源写出到项目文件。
    ///
    /// 对应 Java: `public void fireResourceWrittenEvent(Resource resource)`
    pub fn fire_resource_written_event(&mut self, resource: &Rc<RefCell<Resource>>) {
        for listener in &mut self.project_listeners {
            listener.resource_written(resource);
        }
    }

    /// 通知监听器：已从项目文件中读入一个日历。
    ///
    /// 对应 Java: `public void fireCalendarReadEvent(ProjectCalendar calendar)`
    pub fn fire_calendar_read_event(&mut self, calendar: &Rc<RefCell<ProjectCalendar>>) {
        for listener in &mut self.project_listeners {
            listener.calendar_read(calendar);
        }
    }

    /// 通知监听器：已从项目文件中读入一个资源分配。
    ///
    /// 对应 Java: `public void fireAssignmentReadEvent(ResourceAssignment resourceAssignment)`
    pub fn fire_assignment_read_event(&mut self, assignment: &Rc<RefCell<ResourceAssignment>>) {
        for listener in &mut self.project_listeners {
            listener.assignment_read(assignment);
        }
    }

    /// 通知监听器：已将资源分配写出到项目文件。
    ///
    /// 对应 Java: `public void fireAssignmentWrittenEvent(ResourceAssignment resourceAssignment)`
    pub fn fire_assignment_written_event(&mut self, assignment: &Rc<RefCell<ResourceAssignment>>) {
        for listener in &mut self.project_listeners {
            listener.assignment_written(assignment);
        }
    }

    /// 通知监听器：已从项目文件中读入一个关系。
    ///
    /// 对应 Java: `public void fireRelationReadEvent(Relation relation)`
    pub fn fire_relation_read_event(&mut self, relation: &Rc<RefCell<Relation>>) {
        for listener in &mut self.project_listeners {
            listener.relation_read(relation);
        }
    }

    /// 通知监听器：已将关系写出到项目文件。
    ///
    /// 对应 Java: `public void fireRelationWrittenEvent(Relation relation)`
    pub fn fire_relation_written_event(&mut self, relation: &Rc<RefCell<Relation>>) {
        for listener in &mut self.project_listeners {
            listener.relation_written(relation);
        }
    }

    /// 通知监听器：已将日历写出到项目文件。
    ///
    /// 对应 Java: `public void fireCalendarWrittenEvent(ProjectCalendar calendar)`
    pub fn fire_calendar_written_event(&mut self, calendar: &Rc<RefCell<ProjectCalendar>>) {
        for listener in &mut self.project_listeners {
            listener.calendar_written(calendar);
        }
    }

    /// 注册一个监听器。
    ///
    /// 对应 Java: `public void addProjectListener(ProjectListener listener)`
    pub fn add_project_listener(&mut self, listener: Box<dyn ProjectListener>) {
        self.project_listeners.push(listener);
    }

    /// 注册一批监听器（按给定顺序逐个追加）。
    ///
    /// 对应 Java: `public void addProjectListeners(List<ProjectListener> listeners)`
    ///
    /// Java 会判 `listeners != null`；Rust 的参数是 `Vec`，不存在空引用，空 `Vec` 即空操作。
    pub fn add_project_listeners(&mut self, listeners: Vec<Box<dyn ProjectListener>>) {
        for listener in listeners {
            self.add_project_listener(listener);
        }
    }

    /// 清空全部监听器。
    ///
    /// 对应 Java: `public void removeProjectListener(ProjectListener listener)`
    ///
    /// Java 按身份移除指定监听器，Rust 无法表达该语义（原因见文件头），改为清空全部。
    pub fn clear_project_listeners(&mut self) {
        self.project_listeners.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::listener::default_project_listener::DefaultProjectListener;
    use crate::relation::Builder as RelationBuilder;
    use std::rc::Weak;

    /// 事件记录器：把收到的事件名按顺序写入共享日志。
    type EventLog = Rc<RefCell<Vec<&'static str>>>;

    struct Recorder {
        log: EventLog,
    }

    impl Recorder {
        /// 返回监听器与其事件日志（日志与监听器共享同一 `Rc`）。
        fn shared() -> (Self, EventLog) {
            let log: EventLog = Rc::new(RefCell::new(Vec::new()));
            (Self { log: log.clone() }, log)
        }

        fn record(&mut self, event: &'static str) {
            self.log.borrow_mut().push(event);
        }
    }

    impl ProjectListener for Recorder {
        fn task_read(&mut self, _task: &Rc<RefCell<Task>>) {
            self.record("task_read");
        }

        fn task_written(&mut self, _task: &Rc<RefCell<Task>>) {
            self.record("task_written");
        }

        fn resource_read(&mut self, _resource: &Rc<RefCell<Resource>>) {
            self.record("resource_read");
        }

        fn resource_written(&mut self, _resource: &Rc<RefCell<Resource>>) {
            self.record("resource_written");
        }

        fn calendar_read(&mut self, _calendar: &Rc<RefCell<ProjectCalendar>>) {
            self.record("calendar_read");
        }

        fn calendar_written(&mut self, _calendar: &Rc<RefCell<ProjectCalendar>>) {
            self.record("calendar_written");
        }

        fn assignment_read(&mut self, _assignment: &Rc<RefCell<ResourceAssignment>>) {
            self.record("assignment_read");
        }

        fn assignment_written(&mut self, _assignment: &Rc<RefCell<ResourceAssignment>>) {
            self.record("assignment_written");
        }

        fn relation_read(&mut self, _relation: &Rc<RefCell<Relation>>) {
            self.record("relation_read");
        }

        fn relation_written(&mut self, _relation: &Rc<RefCell<Relation>>) {
            self.record("relation_written");
        }
    }

    /// 回调里改写实体字段的监听器，用于验证共享句柄可写。
    struct TaskIdSetter;

    impl ProjectListener for TaskIdSetter {
        fn task_read(&mut self, task: &Rc<RefCell<Task>>) {
            task.borrow_mut().set_unique_id(Some(99));
        }

        fn task_written(&mut self, _task: &Rc<RefCell<Task>>) {}

        fn resource_read(&mut self, _resource: &Rc<RefCell<Resource>>) {}

        fn resource_written(&mut self, _resource: &Rc<RefCell<Resource>>) {}

        fn calendar_read(&mut self, _calendar: &Rc<RefCell<ProjectCalendar>>) {}

        fn calendar_written(&mut self, _calendar: &Rc<RefCell<ProjectCalendar>>) {}

        fn assignment_read(&mut self, _assignment: &Rc<RefCell<ResourceAssignment>>) {}

        fn assignment_written(&mut self, _assignment: &Rc<RefCell<ResourceAssignment>>) {}

        fn relation_read(&mut self, _relation: &Rc<RefCell<Relation>>) {}

        fn relation_written(&mut self, _relation: &Rc<RefCell<Relation>>) {}
    }

    /// Java 中 `fire*` 的书写顺序。
    const ALL_EVENTS: [&str; 10] = [
        "task_read",
        "task_written",
        "resource_read",
        "resource_written",
        "calendar_read",
        "assignment_read",
        "assignment_written",
        "relation_read",
        "relation_written",
        "calendar_written",
    ];

    fn task() -> Rc<RefCell<Task>> {
        Rc::new(RefCell::new(Task::new()))
    }

    fn resource() -> Rc<RefCell<Resource>> {
        Rc::new(RefCell::new(Resource::new(Weak::new())))
    }

    fn assignment() -> Rc<RefCell<ResourceAssignment>> {
        Rc::new(RefCell::new(ResourceAssignment::new()))
    }

    fn relation() -> Rc<RefCell<Relation>> {
        Rc::new(RefCell::new(RelationBuilder::new().build()))
    }

    fn calendar() -> Rc<RefCell<ProjectCalendar>> {
        Rc::new(RefCell::new(ProjectCalendar::new(Weak::new(), false)))
    }

    /// 依次触发 Java 中定义的全部 10 个事件。
    fn fire_all(manager: &mut EventManager) {
        manager.fire_task_read_event(&task());
        manager.fire_task_written_event(&task());
        manager.fire_resource_read_event(&resource());
        manager.fire_resource_written_event(&resource());
        manager.fire_calendar_read_event(&calendar());
        manager.fire_assignment_read_event(&assignment());
        manager.fire_assignment_written_event(&assignment());
        manager.fire_relation_read_event(&relation());
        manager.fire_relation_written_event(&relation());
        manager.fire_calendar_written_event(&calendar());
    }

    #[test]
    fn fire_events_broadcast_to_every_listener() {
        let mut manager = EventManager::new();
        let (first, first_log) = Recorder::shared();
        let (second, second_log) = Recorder::shared();
        manager.add_project_listener(Box::new(first));
        manager.add_project_listener(Box::new(second));

        fire_all(&mut manager);

        assert_eq!(*first_log.borrow(), ALL_EVENTS.to_vec());
        assert_eq!(*second_log.borrow(), ALL_EVENTS.to_vec());
    }

    #[test]
    fn events_without_listeners_are_noop() {
        let mut manager = EventManager::new();

        fire_all(&mut manager);
    }

    #[test]
    fn add_project_listeners_registers_each_listener() {
        let mut manager = EventManager::new();
        let (first, first_log) = Recorder::shared();
        let (second, second_log) = Recorder::shared();

        manager.add_project_listeners(vec![Box::new(first), Box::new(second)]);
        // 空集合等价于 Java 的 null（都是空操作）
        manager.add_project_listeners(Vec::new());

        manager.fire_task_read_event(&task());

        assert_eq!(*first_log.borrow(), vec!["task_read"]);
        assert_eq!(*second_log.borrow(), vec!["task_read"]);
    }

    #[test]
    fn clear_project_listeners_stops_broadcast() {
        let mut manager = EventManager::new();
        let (recorder, log) = Recorder::shared();
        manager.add_project_listener(Box::new(recorder));

        manager.clear_project_listeners();
        fire_all(&mut manager);

        assert!(log.borrow().is_empty());
    }

    #[test]
    fn default_project_listener_is_a_noop_listener() {
        let mut manager = EventManager::new();
        manager.add_project_listener(Box::new(DefaultProjectListener::new()));

        // 不 panic 即通过：默认实现不做任何事
        fire_all(&mut manager);
    }

    #[test]
    fn listener_can_mutate_entity_through_shared_handle() {
        let mut manager = EventManager::new();
        manager.add_project_listener(Box::new(TaskIdSetter));

        let task = task();
        manager.fire_task_read_event(&task);

        assert_eq!(task.borrow().get_unique_id(), Some(99));
    }
}
