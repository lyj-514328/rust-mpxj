//! 对应 Java: `org/mpxj/listener/ProjectListener.java`
//!
//! 接收项目文件读取 / 写出事件的接口。
//!
//! # Java → Rust 方法对照
//!
//! | Java | Rust |
//! |---|---|
//! | `void taskRead(Task task)` | [`ProjectListener::task_read`] |
//! | `void taskWritten(Task task)` | [`ProjectListener::task_written`] |
//! | `void resourceRead(Resource resource)` | [`ProjectListener::resource_read`] |
//! | `void resourceWritten(Resource resource)` | [`ProjectListener::resource_written`] |
//! | `void calendarRead(ProjectCalendar calendar)` | [`ProjectListener::calendar_read`] |
//! | `void calendarWritten(ProjectCalendar calendar)` | [`ProjectListener::calendar_written`] |
//! | `void assignmentRead(ResourceAssignment assignment)` | [`ProjectListener::assignment_read`] |
//! | `void assignmentWritten(ResourceAssignment assignment)` | [`ProjectListener::assignment_written`] |
//! | `void relationRead(Relation relation)` | [`ProjectListener::relation_read`] |
//! | `void relationWritten(Relation relation)` | [`ProjectListener::relation_written`] |
//!
//! # 与 Java 的差异
//!
//! - **实体以共享句柄传递**：Java 直接把实体引用传进来（`Task task`），Rust 侧统一写作
//!   `&Rc<RefCell<Task>>` 这类共享句柄，而不是 `&Task` 或 `&mut Task`。原因与前一批落地的
//!   [`crate::listener::field_listener::FieldListener`] 一致：实体在 crate 内一律以
//!   `Rc<RefCell<..>>` 共享持有，`fire_*` 的调用点往往正处在对实体的可变操作序列中
//!   （例如写出流程先取 `&mut` 改字段，再广播事件），此时无法再构造一个 `&mut Task` 别名
//!   交出去；改用共享句柄后，监听器想读就 `borrow()`、想改就 `borrow_mut()`，也可以自行
//!   `clone()` 保留句柄，语义与 Java 最接近，且不会与调用点的借用冲突。
//! - **监听器方法取 `&mut self`**：Java 的监听器是普通对象；Rust 侧允许监听器在回调中
//!   累积状态（计数、日志等），故与 [`crate::listener::field_listener::FieldListener`] 的
//!   既有约定一致，统一取 `&mut self`。
//! - **`null` 参数**：Java 的 `fire*` 允许传 `null` 实体；Rust 参数类型是引用，不存在对应写法。
//!
//! # 跳过的 Java 代码及原因
//!
//! | Java | 原因 |
//! |---|---|
//! | 本接口未使用反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable` | 无对应代码 |

use std::cell::RefCell;
use std::rc::Rc;

use crate::project_calendar::ProjectCalendar;
use crate::relation::Relation;
use crate::resource::Resource;
use crate::resource_assignment::ResourceAssignment;
use crate::task::Task;

/// 实现本 trait 的类型可以接收项目文件中的事件通知。
///
/// 对应 Java: `public interface ProjectListener`
///
/// 签名中实体用共享句柄（`&Rc<RefCell<..>>`）而非 `&mut` 的取舍见文件头。
pub trait ProjectListener {
    /// 从文件中读入一个任务时调用。
    ///
    /// 对应 Java: `void taskRead(Task task)`
    fn task_read(&mut self, task: &Rc<RefCell<Task>>);

    /// 将一个任务写出到文件时调用。
    ///
    /// 对应 Java: `void taskWritten(Task task)`
    fn task_written(&mut self, task: &Rc<RefCell<Task>>);

    /// 从文件中读入一个资源时调用。
    ///
    /// 对应 Java: `void resourceRead(Resource resource)`
    fn resource_read(&mut self, resource: &Rc<RefCell<Resource>>);

    /// 将一个资源写出到文件时调用。
    ///
    /// 对应 Java: `void resourceWritten(Resource resource)`
    fn resource_written(&mut self, resource: &Rc<RefCell<Resource>>);

    /// 从文件中读入一个日历时调用。
    ///
    /// 对应 Java: `void calendarRead(ProjectCalendar calendar)`
    fn calendar_read(&mut self, calendar: &Rc<RefCell<ProjectCalendar>>);

    /// 将一个日历写出到文件时调用。
    ///
    /// 对应 Java: `void calendarWritten(ProjectCalendar calendar)`
    fn calendar_written(&mut self, calendar: &Rc<RefCell<ProjectCalendar>>);

    /// 从文件中读入一个资源分配时调用。
    ///
    /// 对应 Java: `void assignmentRead(ResourceAssignment assignment)`
    fn assignment_read(&mut self, assignment: &Rc<RefCell<ResourceAssignment>>);

    /// 将一个资源分配写出到文件时调用。
    ///
    /// 对应 Java: `void assignmentWritten(ResourceAssignment assignment)`
    fn assignment_written(&mut self, assignment: &Rc<RefCell<ResourceAssignment>>);

    /// 从文件中读入一个关系时调用。
    ///
    /// 对应 Java: `void relationRead(Relation relation)`
    fn relation_read(&mut self, relation: &Rc<RefCell<Relation>>);

    /// 将一个关系写出到文件时调用。
    ///
    /// 对应 Java: `void relationWritten(Relation relation)`
    fn relation_written(&mut self, relation: &Rc<RefCell<Relation>>);
}
