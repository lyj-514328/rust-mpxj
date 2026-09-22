//! Corresponds to Java: `org/mpxj/reader/AbstractProjectReader.java`
//!
//! Abstract implementation of [`ProjectReader`]: it carries the common logic for
//! "listener registration" and "dispatching listeners to a project file / project context /
//! another reader".
//!
//! # Java inheritance chain → Rust trait default methods
//!
//! Java: `public abstract class AbstractProjectReader implements ProjectReader`.
//! Rust has no inheritance; the mapping is:
//!
//! - [`AbstractProjectReader`] uses [`ProjectReader`] as a supertrait and writes the common
//!   implementation of the Java abstract base class as trait **default methods**, so a concrete
//!   reader only needs to `impl` it and provide listener storage;
//! - Java's private field `private List<ProjectListener> m_projectListeners` has no place in a
//!   trait, so implementors are instead required to provide two accessors (Rust-specific, see
//!   [`AbstractProjectReader::project_listeners`] / [`AbstractProjectReader::project_listeners_mut`]).
//!
//! # Listener storage and forwarding (a Rust-specific trade-off)
//!
//! Java's `m_projectListeners` is a `List<ProjectListener>`: the same listener **object** is
//! registered simultaneously on the project file's `EventManager`, the project context's
//! `EventManager`, and another reader
//! (`addListenersToProject` / `addListenersToContext` / `addListenersToReader`), sharing the
//! same instance in all three places. In contrast, Rust's
//! [`crate::event_manager::EventManager::add_project_listeners`] **takes ownership** as
//! `Box<dyn ProjectListener>` (`addProjectListeners` consumes the passed list).
//!
//! This module therefore introduces the shared handle [`SharedProjectListener`]
//! (`Rc<RefCell<Box<dyn ProjectListener>>>`):
//!
//! - **Registration**: the default method
//!   [`AbstractProjectReader::add_project_listener_default`] wraps the received
//!   `Box<dyn ProjectListener>` in a shared handle and stores it in a `Vec`;
//! - **Dispatch**: when it must be handed to an `EventManager`, the shared handle is cloned and
//!   boxed again (`Box::new(handle.clone())`). That `Box` itself implements [`ProjectListener`],
//!   and each callback is forwarded to the **same** listener instance inside the handle, which
//!   is equivalent to Java's "the same listener object registered in multiple places";
//! - **Observable behavior matches Java**: a listener registered in N places receives N
//!   notifications; [`AbstractProjectReader::add_listeners_to_reader`] hands the listener (via
//!   the shared handle) to another reader, and when that reader dispatches, it still acts on the
//!   same listener.
//! - **No panic**: when there is a borrow conflict inside the handle (a Rust-specific situation
//!   that Java does not have), forwarding is skipped; see
//!   [`SharedProjectListener::try_borrow_mut`].
//!
//! # Naming convention: implementations of `ProjectReader` methods in the abstract layer get a `_default` suffix
//!
//! In Java a subclass can override interface methods and reuse them along the inheritance chain.
//! A Rust trait default method **cannot** override a same-named method of a supertrait: if a
//! subtrait declares a same-named method, any call site (such as `reader.read_file(..)`) reports
//! E0034 (method resolution ambiguity) because two candidates exist. This crate therefore
//! adopts the following convention:
//!
//! | Layer | Java method | Rust default method |
//! |---|---|---|
//! | `AbstractProjectReader` | `addProjectListener(ProjectListener)` | [`AbstractProjectReader::add_project_listener_default`] |
//! | `AbstractProjectFileReader` | `readAll(File)` | `read_all_file_default` |
//! | `AbstractProjectFileReader` | `read(InputStream)` | `read_reader_default` |
//! | `AbstractProjectFileReader` | `readAll(InputStream)` | `read_all_reader_default` |
//! | `AbstractProjectStreamReader` | `read(File)` | `read_file_default` |
//! | `AbstractProjectStreamReader` | `readAll(File)` | `read_all_file_default` |
//! | `AbstractProjectStreamReader` | `readAll(InputStream)` | `read_all_reader_default` |
//!
//! A concrete reader, in its own `impl ProjectReader`, forwards the interface methods to the
//! corresponding `_default` methods of the abstract layer, and implements on the abstract-layer
//! trait those methods that are still "abstract" in that layer:
//!
//! ```ignore
//! // Example: stream reader (corresponds to Java `class MyReader extends AbstractProjectStreamReader`)
//! impl ProjectReader for MyStreamReader {
//!     fn add_project_listener(&mut self, listener: Box<dyn ProjectListener>) {
//!         AbstractProjectReader::add_project_listener_default(self, listener);
//!     }
//!     fn read_file(&mut self, path: &Path) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
//!         AbstractProjectStreamReader::read_file_default(self, path)
//!     }
//!     fn read_all_file(&mut self, path: &Path) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
//!         AbstractProjectStreamReader::read_all_file_default(self, path)
//!     }
//!     fn read_reader(&mut self, reader: &mut dyn Read) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
//!         // Java: read(InputStream) implemented by the concrete reader
//!         // ...
//!     }
//!     fn read_all_reader(
//!         &mut self,
//!         reader: &mut dyn Read,
//!     ) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
//!         AbstractProjectStreamReader::read_all_reader_default(self, reader)
//!     }
//! }
//! ```
//!
//! The same applies to a file-based reader (corresponds to Java `extends AbstractProjectFileReader`),
//! except that `read_file` is left for it to implement and the rest are forwarded to
//! `AbstractProjectFileReader::*_default`.
//!
//! # Differences from Java
//!
//! - **`addListenersToProject(ProjectFile)`**: Java writes
//!   `project.getEventManager().addProjectListeners(..)`; on the Rust side
//!   [`ProjectFile::get_event_manager`] only provides immutable access, so the context is
//!   obtained via `project.getProjectContext()` and the listeners are registered on its
//!   `EventManager` (`ProjectFile`'s event manager delegates to the context anyway, see the file
//!   header of [`crate::project_file`]; the two are equivalent);
//! - **Degradation on borrow conflict**: the process above needs a temporary borrow of
//!   `ProjectFile` and `ProjectContext` (`RefCell`). If the caller already holds a mutable borrow
//!   of them (a situation Java does not have), the Rust side degrades to "do not register"
//!   instead of panicking;
//! - **`m_projectListeners == null`**: Java lazily creates the list on first registration and
//!   null-checks in several places; on the Rust side the `Vec` always exists, and an empty list
//!   naturally means a no-op.
//!
//! # Skipped Java code and rationale
//!
//! This class does not use reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`, nor
//! does it contain code referencing `UniversalProjectReader` / `ProjectReaderUtility`, so there
//! are no skipped items.

use std::cell::{RefCell, RefMut};
use std::rc::Rc;

use crate::listener::project_listener::ProjectListener;
use crate::project_calendar::ProjectCalendar;
use crate::project_context::ProjectContext;
use crate::project_file::ProjectFile;
use crate::reader::project_reader::ProjectReader;
use crate::relation::Relation;
use crate::resource::Resource;
use crate::resource_assignment::ResourceAssignment;
use crate::task::Task;

/// A shareable, cloneable listener handle (Rust-specific, see the file header).
///
/// On the Java side `AbstractProjectReader.m_projectListeners` directly stores references to
/// listener objects, and the same object can be registered on multiple `EventManager`s; on the
/// Rust side this handle expresses "the same listener instance held in multiple places": cloning
/// the handle only increments the reference count, while the handle itself implements
/// [`ProjectListener`] and forwards callbacks to the inner listener.
pub struct SharedProjectListener(Rc<RefCell<Box<dyn ProjectListener>>>);

impl SharedProjectListener {
    /// Box the listener and store it in the shared handle.
    ///
    /// Corresponds to Java: `m_projectListeners.add(listener)` (keeps a reference)
    pub fn new(listener: Box<dyn ProjectListener>) -> Self {
        Self(Rc::new(RefCell::new(listener)))
    }

    /// Mutably borrow the inner listener; returns `None` while the handle is already borrowed.
    ///
    /// Rust-specific (Java has no borrow checking): the caller uses this to skip forwarding on a
    /// borrow conflict instead of panicking.
    pub fn try_borrow_mut(&self) -> Option<RefMut<'_, Box<dyn ProjectListener>>> {
        self.0.try_borrow_mut().ok()
    }
}

impl Clone for SharedProjectListener {
    /// Clone the shared handle (only increments the reference count; still points to the same
    /// listener instance).
    ///
    /// Corresponds to Java: passing the same object reference in `m_projectListeners.forEach(reader::addProjectListener)`
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl ProjectListener for SharedProjectListener {
    /// Forward to the listener inside the handle.
    ///
    /// Corresponds to Java: `listener.taskRead(task)`
    fn task_read(&mut self, task: &Rc<RefCell<Task>>) {
        forward(self, |listener| listener.task_read(task));
    }

    /// Forward to the listener inside the handle.
    ///
    /// Corresponds to Java: `listener.taskWritten(task)`
    fn task_written(&mut self, task: &Rc<RefCell<Task>>) {
        forward(self, |listener| listener.task_written(task));
    }

    /// Forward to the listener inside the handle.
    ///
    /// Corresponds to Java: `listener.resourceRead(resource)`
    fn resource_read(&mut self, resource: &Rc<RefCell<Resource>>) {
        forward(self, |listener| listener.resource_read(resource));
    }

    /// Forward to the listener inside the handle.
    ///
    /// Corresponds to Java: `listener.resourceWritten(resource)`
    fn resource_written(&mut self, resource: &Rc<RefCell<Resource>>) {
        forward(self, |listener| listener.resource_written(resource));
    }

    /// Forward to the listener inside the handle.
    ///
    /// Corresponds to Java: `listener.calendarRead(calendar)`
    fn calendar_read(&mut self, calendar: &Rc<RefCell<ProjectCalendar>>) {
        forward(self, |listener| listener.calendar_read(calendar));
    }

    /// Forward to the listener inside the handle.
    ///
    /// Corresponds to Java: `listener.calendarWritten(calendar)`
    fn calendar_written(&mut self, calendar: &Rc<RefCell<ProjectCalendar>>) {
        forward(self, |listener| listener.calendar_written(calendar));
    }

    /// Forward to the listener inside the handle.
    ///
    /// Corresponds to Java: `listener.assignmentRead(assignment)`
    fn assignment_read(&mut self, assignment: &Rc<RefCell<ResourceAssignment>>) {
        forward(self, |listener| listener.assignment_read(assignment));
    }

    /// Forward to the listener inside the handle.
    ///
    /// Corresponds to Java: `listener.assignmentWritten(assignment)`
    fn assignment_written(&mut self, assignment: &Rc<RefCell<ResourceAssignment>>) {
        forward(self, |listener| listener.assignment_written(assignment));
    }

    /// Forward to the listener inside the handle.
    ///
    /// Corresponds to Java: `listener.relationRead(relation)`
    fn relation_read(&mut self, relation: &Rc<RefCell<Relation>>) {
        forward(self, |listener| listener.relation_read(relation));
    }

    /// Forward to the listener inside the handle.
    ///
    /// Corresponds to Java: `listener.relationWritten(relation)`
    fn relation_written(&mut self, relation: &Rc<RefCell<Relation>>) {
        forward(self, |listener| listener.relation_written(relation));
    }
}

/// Abstract implementation of [`ProjectReader`].
///
/// Corresponds to Java: `public abstract class AbstractProjectReader implements ProjectReader`
pub trait AbstractProjectReader: ProjectReader {
    /// Return the list of listeners registered on this reader.
    ///
    /// Rust-specific (on the Java side `m_projectListeners` is a private field with no
    /// corresponding method): a trait cannot carry fields, so the implementor exposes the
    /// storage. A concrete reader typically writes `&self.listeners`.
    fn project_listeners(&self) -> &[SharedProjectListener];

    /// Return the list of listeners registered on this reader (mutable).
    ///
    /// Rust-specific (same as above): a concrete reader typically writes `&mut self.listeners`.
    fn project_listeners_mut(&mut self) -> &mut Vec<SharedProjectListener>;

    /// Register a listener.
    ///
    /// Corresponds to Java: `@Override public void addProjectListener(ProjectListener listener)`
    ///
    /// Java lazily creates the list on first registration; on the Rust side the storage always
    /// exists, so this only does one thing: box the listener into a shared handle. A concrete
    /// reader should forward to this method in `impl ProjectReader::add_project_listener`
    /// (for the reason see "Naming convention" in the file header).
    fn add_project_listener_default(&mut self, listener: Box<dyn ProjectListener>) {
        // Java: if (m_projectListeners == null) { m_projectListeners = new ArrayList<>(); }
        //       m_projectListeners.add(listener);
        self.project_listeners_mut()
            .push(SharedProjectListener::new(listener));
    }

    /// Add the listeners registered on this reader to the target project's `EventManager`.
    ///
    /// Corresponds to Java: `protected void addListenersToProject(ProjectFile project)`
    ///
    /// Java writes `project.getEventManager().addProjectListeners(m_projectListeners)`; on the
    /// Rust side `ProjectFile`'s event manager only provides immutable access, so registration
    /// goes through its project context (equivalent, see the file header).
    fn add_listeners_to_project(&self, project: &Rc<RefCell<ProjectFile>>) {
        // Java: the project file's event manager is its context's event manager
        //       (ProjectFile.getEventManager() → m_context.getEventManager()),
        //       so the context is obtained first and then registration happens.
        let context = match project.try_borrow() {
            Ok(project) => project.get_project_context(),
            // Java has no such branch; the Rust side avoids a borrow-conflict panic (see the file header)
            Err(_) => return,
        };

        self.add_listeners_to_context(&context);
    }

    /// Add the listeners registered on this reader to the target project context's `EventManager`.
    ///
    /// Corresponds to Java: `protected void addListenersToContext(ProjectContext context)`
    fn add_listeners_to_context(&self, context: &Rc<RefCell<ProjectContext>>) {
        // Java: context.getEventManager().addProjectListeners(m_projectListeners);
        let Ok(mut context) = context.try_borrow_mut() else {
            // Java has no such branch; the Rust side avoids a borrow-conflict panic (see the file header)
            return;
        };

        let listeners: Vec<Box<dyn ProjectListener>> = self
            .project_listeners()
            .iter()
            .map(|listener| Box::new(listener.clone()) as Box<dyn ProjectListener>)
            .collect();

        context
            .get_event_manager_mut()
            .add_project_listeners(listeners);
    }

    /// Hand the listeners registered on this reader over to another reader.
    ///
    /// Corresponds to Java: `protected void addListenersToReader(ProjectReader reader)`
    ///
    /// Java checks `m_projectListeners != null`; on the Rust side an empty `Vec` means a no-op.
    fn add_listeners_to_reader(&self, reader: &mut dyn ProjectReader) {
        // Java: m_projectListeners.forEach(reader::addProjectListener);
        for listener in self.project_listeners() {
            reader.add_project_listener(Box::new(listener.clone()));
        }
    }
}

/// Forward the callback to the listener inside the shared handle; skip it when the handle is
/// already borrowed (no panic).
///
/// Corresponds to Java: `listener.xxx(event)` (Java calls the listener object's method directly)
fn forward<T>(
    handle: &SharedProjectListener,
    call: impl FnOnce(&mut dyn ProjectListener) -> T,
) -> Option<T> {
    let mut listener = handle.try_borrow_mut()?;
    Some(call(&mut **listener))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpxj_exception::{MpxjError, MpxjResult};
    use std::io::Read;
    use std::path::Path;

    /// Event recorder: writes the received event names into a shared log in order.
    type EventLog = Rc<RefCell<Vec<&'static str>>>;

    struct Recorder {
        log: EventLog,
    }

    impl Recorder {
        /// Returns the listener and its event log (the log and the listener share the same `Rc`).
        fn shared() -> (Self, EventLog) {
            let log: EventLog = Rc::new(RefCell::new(Vec::new()));
            (Self { log: log.clone() }, log)
        }
    }

    impl ProjectListener for Recorder {
        fn task_read(&mut self, _task: &Rc<RefCell<Task>>) {
            self.log.borrow_mut().push("task_read");
        }

        fn task_written(&mut self, _task: &Rc<RefCell<Task>>) {}

        fn resource_read(&mut self, _resource: &Rc<RefCell<Resource>>) {}

        fn resource_written(&mut self, _resource: &Rc<RefCell<Resource>>) {}

        fn calendar_read(&mut self, _calendar: &Rc<RefCell<ProjectCalendar>>) {
            self.log.borrow_mut().push("calendar_read");
        }

        fn calendar_written(&mut self, _calendar: &Rc<RefCell<ProjectCalendar>>) {}

        fn assignment_read(&mut self, _assignment: &Rc<RefCell<ResourceAssignment>>) {}

        fn assignment_written(&mut self, _assignment: &Rc<RefCell<ResourceAssignment>>) {}

        fn relation_read(&mut self, _relation: &Rc<RefCell<Relation>>) {}

        fn relation_written(&mut self, _relation: &Rc<RefCell<Relation>>) {}
    }

    /// Minimal reader implementation: only provides listener storage; all read methods go through
    /// the abstract layer's default implementations.
    #[derive(Default)]
    struct StubReader {
        listeners: Vec<SharedProjectListener>,
    }

    impl ProjectReader for StubReader {
        fn add_project_listener(&mut self, listener: Box<dyn ProjectListener>) {
            self.add_project_listener_default(listener);
        }

        fn read_file(&mut self, _path: &Path) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
            Err(MpxjError::new(MpxjError::INVALID_FILE))
        }

        fn read_all_file(&mut self, path: &Path) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
            Ok(vec![self.read_file(path)?])
        }

        fn read_reader(&mut self, _reader: &mut dyn Read) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
            Err(MpxjError::new(MpxjError::INVALID_FILE))
        }

        fn read_all_reader(
            &mut self,
            reader: &mut dyn Read,
        ) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
            Ok(vec![self.read_reader(reader)?])
        }
    }

    impl AbstractProjectReader for StubReader {
        fn project_listeners(&self) -> &[SharedProjectListener] {
            &self.listeners
        }

        fn project_listeners_mut(&mut self) -> &mut Vec<SharedProjectListener> {
            &mut self.listeners
        }
    }

    /// Fire one `taskRead` event (through the project context's event manager).
    fn fire_task_read(context: &Rc<RefCell<ProjectContext>>) {
        context
            .borrow_mut()
            .get_event_manager_mut()
            .fire_task_read_event(&Rc::new(RefCell::new(Task::new())));
    }

    #[test]
    fn add_project_listener_default_registers_listeners() {
        let mut reader = StubReader::default();
        assert!(reader.project_listeners().is_empty());

        let (first, first_log) = Recorder::shared();
        let (second, second_log) = Recorder::shared();
        reader.add_project_listener_default(Box::new(first));
        reader.add_project_listener_default(Box::new(second));

        assert_eq!(reader.project_listeners().len(), 2);

        let file = ProjectFile::new();
        reader.add_listeners_to_project(&file);

        fire_task_read(&file.borrow().get_project_context());

        assert_eq!(*first_log.borrow(), vec!["task_read"]);
        assert_eq!(*second_log.borrow(), vec!["task_read"]);
    }

    #[test]
    fn listeners_are_shared_so_each_registration_is_notified_once() {
        let mut reader = StubReader::default();
        let (recorder, log) = Recorder::shared();
        reader.add_project_listener_default(Box::new(recorder));

        let file = ProjectFile::new();
        let context = file.borrow().get_project_context();

        // Java: the same listener object can be registered on both the project and the context, and each notifies it once
        reader.add_listeners_to_project(&file);
        reader.add_listeners_to_context(&context);

        fire_task_read(&context);
        assert_eq!(*log.borrow(), vec!["task_read", "task_read"]);
    }

    #[test]
    fn add_listeners_to_reader_forwards_the_same_listener() {
        let mut source = StubReader::default();
        let mut target = StubReader::default();
        let (recorder, log) = Recorder::shared();
        source.add_project_listener_default(Box::new(recorder));

        // Java: m_projectListeners.forEach(reader::addProjectListener)
        // -- register **this** reader's listeners one by one on the target reader
        source.add_listeners_to_reader(&mut target);
        assert_eq!(target.project_listeners().len(), 1);

        // The handed-over listener is still the same instance
        let context = ProjectContext::shared();
        target.add_listeners_to_context(&context);
        fire_task_read(&context);

        assert_eq!(*log.borrow(), vec!["task_read"]);
    }

    #[test]
    fn add_listeners_to_reader_without_listeners_is_noop() {
        let source = StubReader::default();
        let mut target = StubReader::default();

        // The source reader has no listeners registered: the target reader is not modified (equivalent to Java's null check)
        source.add_listeners_to_reader(&mut target);

        assert!(target.project_listeners().is_empty());
    }

    #[test]
    fn add_listeners_to_project_works_for_an_empty_listener_list() {
        let reader = StubReader::default();
        let file = ProjectFile::new();

        reader.add_listeners_to_project(&file);

        // Events are still dispatched when no listener is registered (no receiver)
        fire_task_read(&file.borrow().get_project_context());
    }

    #[test]
    fn borrowed_context_degrades_instead_of_panicking() {
        let mut reader = StubReader::default();
        let (recorder, log) = Recorder::shared();
        reader.add_project_listener_default(Box::new(recorder));

        let context = ProjectContext::shared();

        // A mutable borrow of the context is already held (a situation Java does not have): the Rust side degrades to "do not register" and does not panic
        let guard = context.borrow_mut();
        reader.add_listeners_to_context(&context);
        drop(guard);

        fire_task_read(&context);
        assert!(log.borrow().is_empty());
    }
}
