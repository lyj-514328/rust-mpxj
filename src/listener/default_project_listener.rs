//! Corresponds to Java: `org/mpxj/listener/DefaultProjectListener.java`
//!
//! Empty implementation of [`ProjectListener`].
//!
//! # Java to Rust method mapping
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
//! # Differences from Java
//!
//! - In Java the idiom is to **extend** this class and override the event methods of interest;
//!   Rust has no inheritance, so it becomes **implementing** the [`ProjectListener`] trait and
//!   overriding only the methods of interest (trait methods have no default body, so they must
//!   all be written, but each empty implementation is a single line). This type itself remains
//!   the default implementation in which everything is an empty implementation.
//! - Java's implicit no-argument constructor maps to [`DefaultProjectListener::new`] and `Default`.
//!
//! # Skipped Java code and rationale
//!
//! | Java | Reason |
//! |---|---|
//! | This class does not use reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable` | No corresponding code |

use std::cell::RefCell;
use std::rc::Rc;

use crate::listener::project_listener::ProjectListener;
use crate::project_calendar::ProjectCalendar;
use crate::relation::Relation;
use crate::resource::Resource;
use crate::resource_assignment::ResourceAssignment;
use crate::task::Task;

/// Default (empty) implementation of [`ProjectListener`].
///
/// Corresponds to Java: `public class DefaultProjectListener implements ProjectListener`
pub struct DefaultProjectListener;

impl DefaultProjectListener {
    /// Constructor.
    ///
    /// Corresponds to Java: implicit no-argument constructor of `DefaultProjectListener`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultProjectListener {
    /// Corresponds to Java: implicit no-argument constructor of `DefaultProjectListener`.
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectListener for DefaultProjectListener {
    /// Corresponds to Java: `@Override public void taskRead(Task task)`
    fn task_read(&mut self, _task: &Rc<RefCell<Task>>) {
        // Default implementation
    }

    /// Corresponds to Java: `@Override public void taskWritten(Task task)`
    fn task_written(&mut self, _task: &Rc<RefCell<Task>>) {
        // Default implementation
    }

    /// Corresponds to Java: `@Override public void resourceRead(Resource resource)`
    fn resource_read(&mut self, _resource: &Rc<RefCell<Resource>>) {
        // Default implementation
    }

    /// Corresponds to Java: `@Override public void resourceWritten(Resource resource)`
    fn resource_written(&mut self, _resource: &Rc<RefCell<Resource>>) {
        // Default implementation
    }

    /// Corresponds to Java: `@Override public void calendarRead(ProjectCalendar calendar)`
    fn calendar_read(&mut self, _calendar: &Rc<RefCell<ProjectCalendar>>) {
        // Default implementation
    }

    /// Corresponds to Java: `@Override public void calendarWritten(ProjectCalendar calendar)`
    fn calendar_written(&mut self, _calendar: &Rc<RefCell<ProjectCalendar>>) {
        // Default implementation
    }

    /// Corresponds to Java: `@Override public void assignmentRead(ResourceAssignment assignment)`
    fn assignment_read(&mut self, _assignment: &Rc<RefCell<ResourceAssignment>>) {
        // Default implementation
    }

    /// Corresponds to Java: `@Override public void assignmentWritten(ResourceAssignment assignment)`
    fn assignment_written(&mut self, _assignment: &Rc<RefCell<ResourceAssignment>>) {
        // Default implementation
    }

    /// Corresponds to Java: `@Override public void relationRead(Relation relation)`
    fn relation_read(&mut self, _relation: &Rc<RefCell<Relation>>) {
        // Default implementation
    }

    /// Corresponds to Java: `@Override public void relationWritten(Relation relation)`
    fn relation_written(&mut self, _relation: &Rc<RefCell<Relation>>) {
        // Default implementation
    }
}
