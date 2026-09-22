//! Corresponds to Java: `org/mpxj/listener/ProjectListener.java`
//!
//! Interface for receiving project file read / write events.
//!
//! # Java to Rust method mapping
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
//! # Differences from Java
//!
//! - **Entities are passed as shared handles**: Java passes the entity reference directly
//!   (`Task task`); the Rust side uniformly uses shared handles such as `&Rc<RefCell<Task>>`
//!   rather than `&Task` or `&mut Task`. The reason matches the previously landed
//!   [`crate::listener::field_listener::FieldListener`]: entities are always held shared as
//!   `Rc<RefCell<..>>` within the crate, and the call sites of `fire_*` are often in the middle
//!   of a mutable operation sequence on the entity (for example, the write path first takes
//!   `&mut` to modify a field and then broadcasts the event), at which point another
//!   `&mut Task` alias cannot be constructed and handed out; after switching to shared handles,
//!   a listener can `borrow()` to read, `borrow_mut()` to modify, or `clone()` to keep the
//!   handle, which is closest to Java semantics and does not conflict with the call site's borrow.
//! - **Listener methods take `&mut self`**: Java listeners are ordinary objects; the Rust side
//!   allows a listener to accumulate state (counters, logs, etc.) in the callback, so consistent
//!   with the existing convention of [`crate::listener::field_listener::FieldListener`], all
//!   methods take `&mut self`.
//! - **`null` parameters**: Java's `fire*` allows passing `null` entities; Rust parameter types
//!   are references, so there is no corresponding form.
//!
//! # Skipped Java code and rationale
//!
//! | Java | Reason |
//! |---|---|
//! | This interface does not use reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable` | No corresponding code |

use std::cell::RefCell;
use std::rc::Rc;

use crate::project_calendar::ProjectCalendar;
use crate::relation::Relation;
use crate::resource::Resource;
use crate::resource_assignment::ResourceAssignment;
use crate::task::Task;

/// Types implementing this trait can receive event notifications from a project file.
///
/// Corresponds to Java: `public interface ProjectListener`
///
/// The trade-off of using shared handles (`&Rc<RefCell<..>>`) rather than `&mut` for entities
/// in the signatures is described in the file header.
pub trait ProjectListener {
    /// Called when a task is read from a file.
    ///
    /// Corresponds to Java: `void taskRead(Task task)`
    fn task_read(&mut self, task: &Rc<RefCell<Task>>);

    /// Called when a task is written to a file.
    ///
    /// Corresponds to Java: `void taskWritten(Task task)`
    fn task_written(&mut self, task: &Rc<RefCell<Task>>);

    /// Called when a resource is read from a file.
    ///
    /// Corresponds to Java: `void resourceRead(Resource resource)`
    fn resource_read(&mut self, resource: &Rc<RefCell<Resource>>);

    /// Called when a resource is written to a file.
    ///
    /// Corresponds to Java: `void resourceWritten(Resource resource)`
    fn resource_written(&mut self, resource: &Rc<RefCell<Resource>>);

    /// Called when a calendar is read from a file.
    ///
    /// Corresponds to Java: `void calendarRead(ProjectCalendar calendar)`
    fn calendar_read(&mut self, calendar: &Rc<RefCell<ProjectCalendar>>);

    /// Called when a calendar is written to a file.
    ///
    /// Corresponds to Java: `void calendarWritten(ProjectCalendar calendar)`
    fn calendar_written(&mut self, calendar: &Rc<RefCell<ProjectCalendar>>);

    /// Called when a resource assignment is read from a file.
    ///
    /// Corresponds to Java: `void assignmentRead(ResourceAssignment assignment)`
    fn assignment_read(&mut self, assignment: &Rc<RefCell<ResourceAssignment>>);

    /// Called when a resource assignment is written to a file.
    ///
    /// Corresponds to Java: `void assignmentWritten(ResourceAssignment assignment)`
    fn assignment_written(&mut self, assignment: &Rc<RefCell<ResourceAssignment>>);

    /// Called when a relation is read from a file.
    ///
    /// Corresponds to Java: `void relationRead(Relation relation)`
    fn relation_read(&mut self, relation: &Rc<RefCell<Relation>>);

    /// Called when a relation is written to a file.
    ///
    /// Corresponds to Java: `void relationWritten(Relation relation)`
    fn relation_written(&mut self, relation: &Rc<RefCell<Relation>>);
}
