//! Corresponds to Java: `org/mpxj/EventManager.java`
//!
//! Subscription hub for events generated during project file reading and writing.
//!
//! # Java -> Rust method mapping
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
//! | `public void removeProjectListener(ProjectListener listener)` | [`EventManager::clear_project_listeners`] (different signature, see below) |
//! | `private List<ProjectListener> m_projectListeners;` | field `project_listeners: Vec<Box<dyn ProjectListener>>` |
//!
//! # Differences from Java
//!
//! - **Storage drops the "uninitialized" state**: Java's `m_projectListeners` starts as `null`, and every `fire*`
//! plus `add` / `remove` must null-check; the Rust side uses an always-present `Vec<Box<dyn ProjectListener>>`
//! (Java's `List` -> `Vec`), so all null-check branches disappear and an empty list is naturally a no-op.
//! - **`removeProjectListener` -> [`EventManager::clear_project_listeners`]**: Java's
//! `List.remove(Object)` removes the specified single listener by `equals` (here, by reference identity).
//! **The real blocking reason** (not "dependencies not in place"):
//! 1. [`EventManager::add_project_listener`] accepts a listener by `Box<dyn ProjectListener>` **by value**,
//! so the registrant cannot get the handle back; `dyn ProjectListener` also does not implement equality, so "remove by identity" cannot be expressed.
//! 2. That signature and storage shape are fixed by the existing reader interface: `crate::reader::project_reader::ProjectReader`
//! 's `add_project_listener(Box<dyn ProjectListener>)` (`reader/abstract_project_reader.rs`,
//! the implementations in `mpp/mpp_reader.rs`, `mpx/mpx_reader.rs` and the call site in `mpp/constraint_factory.rs`)
//! all pass by `Box`.
//! 3. This repo already has `SharedProjectListener` (`reader/abstract_project_reader.rs`,
//! `Rc<RefCell<Box<dyn ProjectListener>>>` + `Rc::ptr_eq`) which can express identity, but its internal `Rc`
//! is a private field with no exposed equality accessor -- this file cannot
//! implement a `ptr_eq` comparison from it; creating another same-shaped handle inside `EventManager` for this would
//! form two parallel implementations with `SharedProjectListener` (a practice this project explicitly avoids).
//! Therefore this keeps the semantically clear operation of **clearing all listeners**; when a single listener needs to be removed, the caller should instead
//! rebuild the `EventManager` or narrow the registration scope.
//! - **`fire_*` / registration methods take `&mut self`**: listener methods take `&mut self` (see
//! [`crate::listener::project_listener::ProjectListener`]), iterating over the listener list requires a mutable borrow,
//! so these methods are all `&mut self`. For [`crate::project_context::ProjectContext`], a mutable reference must be obtained via
//! `get_event_manager_mut()`.
//! - **Entity parameters use shared handles**: Java passes entity references; the Rust side uniformly uses `&Rc<RefCell<..>>`,
//! for the reason see the file header of [`crate::listener::project_listener`].
//! - **No `mem::take` to swap out the listener list**: this type only iterates listeners under `&mut self`, and a listener callback
//! cannot obtain this `EventManager`, so there is no possibility of a Rust aliasing conflict from "calling `fire_*` again inside a callback"; hence
//! simply iterating with `for listener in &mut self.project_listeners` is enough; if in the future `EventManager`
//! is placed in an `Rc<RefCell<..>>` and re-entered from a callback, the conflict will occur in the outer `RefCell` borrow (a runtime panic),
//! and the caller must `mem::take` itself before re-entering; this type cannot avoid it internally.
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

/// Provides subscription capability for project file read/write events.
///
/// Corresponds to Java: `public class EventManager`
pub struct EventManager {
    /// Corresponds to Java field `private List<ProjectListener> m_projectListeners;`
    ///
/// Java uses `List`, Rust uses `Vec<Box<dyn ProjectListener>>` (Java's `List` -> `Vec`).
    project_listeners: Vec<Box<dyn ProjectListener>>,
}

impl Default for EventManager {
/// Corresponds to Java: `EventManager`'s implicit no-argument constructor.
    fn default() -> Self {
        Self::new()
    }
}

impl EventManager {
/// Constructor.
    ///
/// Corresponds to Java: `EventManager`'s implicit no-argument constructor (`m_projectListeners` is `null`;
/// on the Rust side an empty `Vec`, semantically equivalent, see the file header).
    pub fn new() -> Self {
        Self {
            project_listeners: Vec::new(),
        }
    }

/// Notify listeners: a task has been read from the project file.
    ///
    /// Corresponds to Java: `public void fireTaskReadEvent(Task task)`
    pub fn fire_task_read_event(&mut self, task: &Rc<RefCell<Task>>) {
        for listener in &mut self.project_listeners {
            listener.task_read(task);
        }
    }

/// Notify listeners: a task has been written to the project file.
    ///
    /// Corresponds to Java: `public void fireTaskWrittenEvent(Task task)`
    pub fn fire_task_written_event(&mut self, task: &Rc<RefCell<Task>>) {
        for listener in &mut self.project_listeners {
            listener.task_written(task);
        }
    }

/// Notify listeners: a resource has been read from the project file.
    ///
    /// Corresponds to Java: `public void fireResourceReadEvent(Resource resource)`
    pub fn fire_resource_read_event(&mut self, resource: &Rc<RefCell<Resource>>) {
        for listener in &mut self.project_listeners {
            listener.resource_read(resource);
        }
    }

/// Notify listeners: a resource has been written to the project file.
    ///
    /// Corresponds to Java: `public void fireResourceWrittenEvent(Resource resource)`
    pub fn fire_resource_written_event(&mut self, resource: &Rc<RefCell<Resource>>) {
        for listener in &mut self.project_listeners {
            listener.resource_written(resource);
        }
    }

/// Notify listeners: a calendar has been read from the project file.
    ///
    /// Corresponds to Java: `public void fireCalendarReadEvent(ProjectCalendar calendar)`
    pub fn fire_calendar_read_event(&mut self, calendar: &Rc<RefCell<ProjectCalendar>>) {
        for listener in &mut self.project_listeners {
            listener.calendar_read(calendar);
        }
    }

/// Notify listeners: a resource assignment has been read from the project file.
    ///
    /// Corresponds to Java: `public void fireAssignmentReadEvent(ResourceAssignment resourceAssignment)`
    pub fn fire_assignment_read_event(&mut self, assignment: &Rc<RefCell<ResourceAssignment>>) {
        for listener in &mut self.project_listeners {
            listener.assignment_read(assignment);
        }
    }

/// Notify listeners: a resource assignment has been written to the project file.
    ///
    /// Corresponds to Java: `public void fireAssignmentWrittenEvent(ResourceAssignment resourceAssignment)`
    pub fn fire_assignment_written_event(&mut self, assignment: &Rc<RefCell<ResourceAssignment>>) {
        for listener in &mut self.project_listeners {
            listener.assignment_written(assignment);
        }
    }

/// Notify listeners: a relation has been read from the project file.
    ///
    /// Corresponds to Java: `public void fireRelationReadEvent(Relation relation)`
    pub fn fire_relation_read_event(&mut self, relation: &Rc<RefCell<Relation>>) {
        for listener in &mut self.project_listeners {
            listener.relation_read(relation);
        }
    }

/// Notify listeners: a relation has been written to the project file.
    ///
    /// Corresponds to Java: `public void fireRelationWrittenEvent(Relation relation)`
    pub fn fire_relation_written_event(&mut self, relation: &Rc<RefCell<Relation>>) {
        for listener in &mut self.project_listeners {
            listener.relation_written(relation);
        }
    }

/// Notify listeners: a calendar has been written to the project file.
    ///
    /// Corresponds to Java: `public void fireCalendarWrittenEvent(ProjectCalendar calendar)`
    pub fn fire_calendar_written_event(&mut self, calendar: &Rc<RefCell<ProjectCalendar>>) {
        for listener in &mut self.project_listeners {
            listener.calendar_written(calendar);
        }
    }

/// Register a listener.
    ///
    /// Corresponds to Java: `public void addProjectListener(ProjectListener listener)`
    pub fn add_project_listener(&mut self, listener: Box<dyn ProjectListener>) {
        self.project_listeners.push(listener);
    }

/// Register a batch of listeners (appended one by one in the given order).
    ///
    /// Corresponds to Java: `public void addProjectListeners(List<ProjectListener> listeners)`
    ///
/// Java checks `listeners != null`; the Rust parameter is a `Vec`, there is no null reference, and an empty `Vec` is a no-op.
    pub fn add_project_listeners(&mut self, listeners: Vec<Box<dyn ProjectListener>>) {
        for listener in listeners {
            self.add_project_listener(listener);
        }
    }

/// Clear all listeners.
    ///
    /// Corresponds to Java: `public void removeProjectListener(ProjectListener listener)`
    ///
/// Java removes the specified listener by identity; Rust cannot express that semantic (reason in the file header), so it clears all instead.
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

/// Event recorder: writes received event names in order into a shared log.
    type EventLog = Rc<RefCell<Vec<&'static str>>>;

    struct Recorder {
        log: EventLog,
    }

    impl Recorder {
/// Returns the listener and its event log (the log shares the same `Rc` as the listener).
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

/// A listener that rewrites entity fields in its callback, to verify that the shared handle is writable.
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

/// The order in which `fire*` are written in Java.
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

/// Fire all 10 events defined in Java in sequence.
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
// An empty collection is equivalent to Java's null (both are no-ops)
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

// Passing means not panicking: the default implementation does nothing
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
