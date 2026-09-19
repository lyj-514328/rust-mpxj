//! 对应 Java: `org/mpxj/listener/DefaultProjectListener.java`
//!
//! [`ProjectListener`] 的空实现。
//!
//! # Java → Rust 方法对照
//!
//! | Java | Rust |
//! |---|---|
//! | `@Override public void taskRead(Task task)` | [`ProjectListener::task_read`] |
//! | `@Override public void taskWritten(Task task)` | [`ProjectListener::task_written`] |
//! | `@Override public void resourceRead(Resource resource)` | [`ProjectListener::resource_read`] |
//! | `@Override public void resourceWritten(Resource resource)` | [`ProjectListener::resource_written`] |
//! | `@Override public void calendarRead(ProjectCalendar calendar)` | [`ProjectListener::calendar_read`] |
//! | `@Override public void calendarWritten(ProjectCalendar calendar)` | [`ProjectListener::calendar_written`] |
//! | `@Override public void assignmentRead(ResourceAssignment assignment)` | [`ProjectListener::assignment_read`] |
//! | `@Override public void assignmentWritten(ResourceAssignment assignment)` | [`ProjectListener::assignment_written`] |
//! | `@Override public void relationRead(Relation relation)` | [`ProjectListener::relation_read`] |
//! | `@Override public void relationWritten(Relation relation)` | [`ProjectListener::relation_written`] |
//!
//! # 与 Java 的差异
//!
//! - Java 的用法是**继承**本类后覆写需要的事件方法；Rust 没有继承，改为**实现**
//!   [`ProjectListener`] trait，只覆写关心的方法（trait 方法没有默认体，需要写全，
//!   但每个空实现只有一行）。本类型本身仍是「全部为空实现」的默认实现。
//! - Java 的隐式无参构造方法落为 [`DefaultProjectListener::new`] 与 `Default`。
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

/// [`ProjectListener`] 的默认（空）实现。
///
/// 对应 Java: `public class DefaultProjectListener implements ProjectListener`
pub struct DefaultProjectListener;

impl DefaultProjectListener {
    /// 构造方法。
    ///
    /// 对应 Java: `DefaultProjectListener` 的隐式无参构造方法。
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultProjectListener {
    /// 对应 Java: `DefaultProjectListener` 的隐式无参构造方法。
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectListener for DefaultProjectListener {
    /// 对应 Java: `@Override public void taskRead(Task task)`
    fn task_read(&mut self, _task: &Rc<RefCell<Task>>) {
        // 默认实现
    }

    /// 对应 Java: `@Override public void taskWritten(Task task)`
    fn task_written(&mut self, _task: &Rc<RefCell<Task>>) {
        // 默认实现
    }

    /// 对应 Java: `@Override public void resourceRead(Resource resource)`
    fn resource_read(&mut self, _resource: &Rc<RefCell<Resource>>) {
        // 默认实现
    }

    /// 对应 Java: `@Override public void resourceWritten(Resource resource)`
    fn resource_written(&mut self, _resource: &Rc<RefCell<Resource>>) {
        // 默认实现
    }

    /// 对应 Java: `@Override public void calendarRead(ProjectCalendar calendar)`
    fn calendar_read(&mut self, _calendar: &Rc<RefCell<ProjectCalendar>>) {
        // 默认实现
    }

    /// 对应 Java: `@Override public void calendarWritten(ProjectCalendar calendar)`
    fn calendar_written(&mut self, _calendar: &Rc<RefCell<ProjectCalendar>>) {
        // 默认实现
    }

    /// 对应 Java: `@Override public void assignmentRead(ResourceAssignment assignment)`
    fn assignment_read(&mut self, _assignment: &Rc<RefCell<ResourceAssignment>>) {
        // 默认实现
    }

    /// 对应 Java: `@Override public void assignmentWritten(ResourceAssignment assignment)`
    fn assignment_written(&mut self, _assignment: &Rc<RefCell<ResourceAssignment>>) {
        // 默认实现
    }

    /// 对应 Java: `@Override public void relationRead(Relation relation)`
    fn relation_read(&mut self, _relation: &Rc<RefCell<Relation>>) {
        // 默认实现
    }

    /// 对应 Java: `@Override public void relationWritten(Relation relation)`
    fn relation_written(&mut self, _relation: &Rc<RefCell<Relation>>) {
        // 默认实现
    }
}
