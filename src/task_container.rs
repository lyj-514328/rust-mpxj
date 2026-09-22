//! Corresponds to Java: `org/mpxj/TaskContainer.java`
//!
//! Container that manages the project's task collection and also carries the task **hierarchy** (parent/child tasks, outline level, WBS).
//!
//! # Java → Rust method mapping
//!
//! | Java | Rust |
//! |---|---|
//! | `public TaskContainer(ProjectFile projectFile)` | [`TaskContainer::new`] + [`TaskContainer::set_context`] (Java receives the parent file at construction time; the Rust side splits this into two steps, see below) |
//! | `@Override public List<Task> getChildTasks()` (`ChildTaskContainer`) | [`TaskContainer::get_child_tasks`] / [`TaskContainer::get_child_tasks_mut`] (Java returns a mutable list; the Rust side splits this into separate read and write methods) |
//! | `@Override public Task addTask()` (`ChildTaskContainer`) | [`TaskContainer::add_task`] (the method body is identical to `add()`; the Rust side keeps two method names because `add` is already taken by the element variant) |
//! | `public Task add()` | [`TaskContainer::add`] |
//! | `@Override public void removed(Task task)` | [`TaskContainer::removed`] (called by [`TaskContainer::remove_at`] / [`TaskContainer::remove_task`]) |
//! | `public void synchronizeTaskIDToHierarchy()` | [`TaskContainer::synchronize_task_id_to_hierarchy`] |
//! | `private int synchroizeTaskIDToHierarchy(ChildTaskContainer, int, int)` | private method [`TaskContainer::synchronize_task_id_to_hierarchy_for`] (the parameter is changed to a "child task list", see below) |
//! | `void updateStructure()` | [`TaskContainer::update_structure`] (`pub(crate)`, corresponding to Java's package-level visibility) |
//! | `public Set<FieldType> getPopulatedFields()` | [`TaskContainer::get_populated_fields`] (the default values and custom fields are passed in by the parent file) |
//! | `public List<CustomField> getCustomFields()` | [`TaskContainer::get_custom_fields`] |
//! | `public FieldType getFieldTypeByAlias(String alias)` | [`TaskContainer::get_field_type_by_alias`] (returns `Option<FieldKey>`, see below) |
//! | `@Override protected int firstUniqueID()` | [`TaskContainer::first_unique_id`] |
//! | The `getByID` / `mapID` / `unmapID` / `getNextID` / `renumberIDs` inherited from `ProjectEntityWithIDContainer` | [`TaskContainer::get_by_id`] / [`TaskContainer::map_id`] / [`TaskContainer::unmap_id`] / [`TaskContainer::get_next_id`] / [`TaskContainer::renumber_ids`] |
//! | The `getByUniqueID` / `updateUniqueID` / `fixUniqueIdClashes` inherited from `ProjectEntityContainer` | [`TaskContainer::get_by_unique_id`] / [`TaskContainer::update_unique_id`] / [`TaskContainer::fix_unique_id_clashes`] |
//! | The `add(T)` / `add(int, T)` / `set(int, T)` / `remove(int)` / `clear()` / `get(int)` / `size()` / `isEmpty()` / `iterator()` inherited from `ListWithCallbacks` | [`TaskContainer::add_task`] (element variant) / [`TaskContainer::add_at`] / [`TaskContainer::set`] / [`TaskContainer::remove_at`] / [`TaskContainer::clear`] / [`TaskContainer::get`] / [`TaskContainer::len`] / [`TaskContainer::is_empty`] / [`TaskContainer::iter`] |
//! | `ListWithCallbacks.remove(Object)` | [`TaskContainer::remove_task`] (Java locates by reference equality; Rust uses `Rc::ptr_eq`; semantics are identical) |
//! | `Collections.sort(this)` | [`TaskContainer::sort`] (by `Task::get_id()`, equivalent to Java's `Task.compareTo`) |
//! | `implements ChildTaskContainer` | see "Restoring `ChildTaskContainer`" below |
//!
//! # Skipped Java code and rationale
//!
//! | Java | Reason |
//! |---|---|
//! | In `removed(Task)`, "delete the resource assignments whose taskUniqueID matches in `getResourceAssignments()`" | **Wired up**: Java iterates and deletes via `m_projectFile.getResourceAssignments()`; on the Rust side [`TaskContainer::removed`] obtains the container through the weak parent `ProjectFile` reference and calls `remove` one by one (when the caller holds a `ProjectFile` borrow it degrades gracefully to skipping the cleanup, without panicking). |
//! | Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable` | this class does not use these mechanisms |
//!
//! # Restoring the inheritance hierarchy: composition + forwarding
//!
//! Java's `TaskContainer extends ProjectEntityWithIDContainer<Task>` (which in turn extends
//! `ProjectEntityContainer<Task>` → `ListWithCallbacks<Task>`) and implements `ChildTaskContainer`.
//! Rust has no inheritance, so it is restored using "composition + forwarding":
//!
//! - Internally uses `Vec<Rc<RefCell<Task>>>` to hold all tasks (corresponding to the `ListWithCallbacks` list),
//!   implementing the base class's public behavior one by one (see the table above), consistent with
//!   the approach of [`crate::project_calendar_container::ProjectCalendarContainer`].
//! - The two `ChildTaskContainer` methods (`getChildTasks` / `addTask`) are provided on this type as **inherent methods**
//!   [`TaskContainer::get_child_tasks`] / [`TaskContainer::add_task`]: `src/lib.rs`
//!   has not yet declared a `child_task_container` module, so this type does not implement that interface (the `Task` side likewise uses on-demand
//!   inherent methods, see `crate::task`); if this interface needs to be shared, a trait can be extracted and these two methods turned into
//!   trait implementations (behavior unchanged).
//! - Java's private recursive `synchroizeTaskIDToHierarchy(ChildTaskContainer parentTask, ...)` is called by both
//!   the "project file" (`m_projectFile`, whose `getChildTasks()` is this container's `m_childTasks`) and
//!   the "task" (`Task implements ChildTaskContainer`). The Rust side does not introduce the `ChildTaskContainer`
//!   trait, and instead uses the private method [`TaskContainer::synchronize_task_id_to_hierarchy_for`],
//!   which receives a "child task list": the root level passes this container's `m_childTasks`, the task level passes `task.get_child_tasks()`,
//!   equivalent to the two call sites in Java.
//!
//! # Base classes not composed (`ProjectEntityContainer` / `ProjectEntityWithIDContainer`)
//!
//! This crate has ported these two base classes, but they require elements to implement `ProjectEntityWithID` and satisfy `Ord`
//! (used for `Collections.sort`) and work by composition with a weak back-reference and callbacks; however this container's
//! `m_idMap`, `m_childTasks`, recursive deletion in `removed`, etc. are all interwoven with the task's own setters (`setID` calls back into
//! `getTasks().mapID(...)`), and composing them would introduce additional borrow-reentrancy problems. It is therefore implemented directly instead:
//!
//! - `m_uniqueIDMap` (the unique ID index) → **linear search by unique ID** ([`TaskContainer::get_by_unique_id`]);
//!   linear search does not become stale when a unique ID changes, so index maintenance is no longer needed; `update_unique_id` only maintains
//!   the conflict list and the sequence generator.
//! - `m_idMap` (the task ID index) → keeps `HashMap<i32, Rc<RefCell<Task>>>` (in Java it is populated by the MPX/MPP
//!   reader via `mapID` and maintained by `Task.setID`); provides [`TaskContainer::map_id`] /
//!   [`TaskContainer::unmap_id`] / [`TaskContainer::get_by_id`].
//! - `m_idSequence` → [`crate::common::object_sequence::ObjectSequence`], used by
//!   [`TaskContainer::get_next_id`].
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `getPopulatedFields()` takes default values and custom fields from the parent file | the caller passes them in explicitly | when `ProjectFile` is already borrowed the container cannot borrow the parent file back |
//!
//! - **Back-reference to the parent file**: Java's constructor receives `ProjectFile` and stores it as a strong reference
//!   (`m_projectFile`, which the base class also uses as the sequence provider). A strong reference on the Rust side would create
//!   an `Rc` cycle of "`ProjectFile` → container → `ProjectFile`", so it is changed to
//!   `Weak<RefCell<ProjectFile>>`, constructed first with an empty weak reference and then wired up by [`TaskContainer::set_context`]
//!   (Java does this in the constructor).
//! - **Direct use of `ProjectFile.getChildTasks()`**: Java's `add()` / `removed()` /
//!   `updateStructure()` access the **same** `m_childTasks` list through `m_projectFile.getChildTasks()`
//!   (`ProjectFile.getChildTasks()` is `m_tasks.getChildTasks()`). The Rust side
//!   accesses this container's `child_tasks` field directly, which is fully equivalent (and also avoids having to
//!   borrow the parent file again while holding `&mut self`).
//! - **Collection return value**: Java's `getChildTasks()` returns a mutable list view; the Rust side returns a slice
//!   [`TaskContainer::get_child_tasks`] and a mutable reference [`TaskContainer::get_child_tasks_mut`].
//! - **`getFieldTypeByAlias`**: Java returns `FieldType`; the Rust side, following the already-ported
//!   [`crate::custom_field_container::CustomFieldContainer`], returns `Option<FieldKey>`
//!   (`FieldKey` is this crate's runtime-erased form of the `FieldType` interface).
//! - **Null semantics**: `NumberHelper.getInt(null)` is 0 in Java; the Rust side expresses this equivalently with `unwrap_or(0)`
//!   ; Java uses the non-null collections returned by `task.getChildTasks()` etc. directly, while the Rust side, when the borrow
//!   fails (`try_borrow` errors), degrades gracefully to an empty collection or skips, without panicking.
//! - **Reentrant borrow**: in Java `Task.setID` calls back into `m_projectFile.getTasks().mapID(...)`,
//!   and `Task`'s constructor (`new_in_file`) together with `Task.generateWBS` / `generateOutlineNumber` /
//!   `addChildTask` all read the parent file's task count, configuration, or sequence generator; while `updateStructure` /
//!   `synchronizeTaskIDToHierarchy` / `renumberIDs` are in turn initiated by `ProjectFile`.
//!   In Rust, if the caller is mutably borrowing `ProjectFile`, the above task methods borrowing the parent file internally will fail.
//!   Handling:
//!   - The three auto-numbering switches needed by [`TaskContainer::update_structure`] are instead passed by the caller
//!     [`crate::ProjectFile::update_structure`] (`ProjectConfig` lives in `ProjectContext`'s
//!     `RefCell`, and reading it does not conflict with holding a `ProjectFile` borrow), while the task count needed for WBS / outline numbering
//!     is provided by the container itself; likewise for `Task::add_child_task_with_auto_outline_level`.
//!     The entire `update_structure` path therefore **never calls back into the parent file** and can be called in place while a borrow is held.
//!   - [`TaskContainer::add_task`] registers the task's own weak reference, and `Task::set_id` uses it to restore
//!     the `mapID` / `unmapID` callbacks; when the parent file is borrowed, that callback degrades gracefully to "do not maintain the index", without panicking.
//!   - This file's **own** parent file accesses ([`TaskContainer::custom_field_container`] /
//!     [`TaskContainer::sequence`]) all use `try_borrow` to degrade gracefully, without panicking.
//!   - The configuration, sequence, and top-level task count needed by [`TaskContainer::add`] to create a task are taken from
//!     `ProjectContext` and the container itself (see [`TaskContainer::creation_context`]),
//!     so it can likewise be called while the caller holds a `ProjectFile` borrow.
//!   - `Task::new_in_file` (the convenience entry point that reads the parent file) now uses `try_borrow`: when the parent file is borrowed
//!     it degrades gracefully to "do no automatic assignment" rather than panicking; when a borrow is held, use
//!     `Task::new_in_file_with_context` and pass in [`TaskCreationContext`] instead.
//! - **Sorting in `updateStructure`**: Java uses `Collections.sort(this)` (`Task.compareTo`
//!   compares by task ID); the Rust side uses [`TaskContainer::sort`] (by `Task::get_id()`), with identical results.
//! - **Recursive deletion in `removed`**: Java uses `while (true) { ... remove(childTaskList.get(0)); }`,
//!   whose termination relies on "`removed` removing the child task from the parent task's child task list" (that is, `child.m_parent`).
//!   The Rust side instead iterates over a snapshot of the child tasks and removes them one by one: the removal set matches the recursive order and does not rely on the child task's back-pointer.
//! - **Parent task back-fill in `updateStructure`**: Java's `parent.addChildTask(task)` also sets
//!   `child.m_parent = this`, which the later `lastTask.getParentTask()` relies on. The Rust side likewise
//!   relies on `Task::add_child_task` maintaining `child.get_parent_task()` (see "Model-layer interfaces used" below).
//!
//! # Model-layer interfaces used (all already in place; listed here only as an index)
//!
//! `Task` (`src/task.rs`) is used with the following signatures:
//!
//! ```text
//! Task::new() -> Task
//! Task::new_in_file(&Rc<RefCell<ProjectFile>>, Option<&Rc<RefCell<Task>>>) -> Task
//! Task::get_id(&self) -> Option<i32> / set_id(&mut self, Option<i32>)
//! Task::get_unique_id(&self) -> Option<i32> / set_unique_id(&mut self, Option<i32>)
//! Task::get_outline_level(&self) -> Option<i32> / set_outline_level(&mut self, Option<i32>)
//! Task::get_parent_task(&self) -> Option<Rc<RefCell<Task>>>
//! Task::get_child_tasks(&self) -> Vec<Rc<RefCell<Task>>>
//! Task::get_null(&self) -> bool
//! Task::get_wbs(&self) -> Option<String>
//! Task::generate_wbs(&mut self, Option<&Rc<RefCell<Task>>>)
//! Task::generate_outline_number(&mut self, Option<&Rc<RefCell<Task>>>)
//! Task::add_child_task(&Rc<RefCell<Task>>, &Rc<RefCell<Task>>)     // associated function with `this` as the first parameter
//! Task::remove_child_task(&Rc<RefCell<Task>>, &Rc<RefCell<Task>>)  // same as above
//! Task::clear_child_tasks(&Rc<RefCell<Task>>)                      // same as above
//! ```
//!
//! `ProjectFile` (`src/project_file.rs`) is used with the following signatures:
//!
//! ```text
//! ProjectFile::new() -> Rc<RefCell<ProjectFile>>
//! ProjectFile::get_project_config(&self) -> Ref<'_, ProjectConfig>
//! ProjectFile::get_custom_fields(&self) -> Rc<RefCell<CustomFieldContainer>>
//! ProjectFile::get_resource_assignments_mut(&mut self) -> &mut ResourceAssignmentContainer
//! ProjectFile::get_unique_id_object_sequence(&mut self, TypeId) -> Rc<ObjectSequence>
//!         // or `ProjectFile: UniqueIdObjectSequenceProvider` (as in Java),
//!         // the method call forms in this file hold for both versions
//! ```

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::{Rc, Weak};

use crate::common::object_sequence::ObjectSequence;
use crate::custom_field::CustomField;
use crate::custom_field_container::CustomFieldContainer;
use crate::field_key::FieldKey;
use crate::field_type_class::FieldTypeClass;
use crate::project_file::ProjectFile;
use crate::resource_assignment::ResourceAssignment;
use crate::task::{Task, TaskCreationContext};
// `ProjectFile` may be provided either as "implementing this trait" or as "providing an inherent method with the same signature" (see the file header
// "Interface assumptions"); the former requires this import, while the latter does not use it, so it is explicitly allowed to be unused.
#[allow(unused_imports)]
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;

/// The project's task collection.
///
/// Corresponds to Java:
/// `public class TaskContainer extends ProjectEntityWithIDContainer<Task> implements ChildTaskContainer`
pub struct TaskContainer {
    /// All tasks in the container.
    ///
    /// Corresponds to Java: `ProjectEntityWithIDContainer` / `ProjectEntityContainer` /
    /// The list storage inherited from `ListWithCallbacks`.
    items: Vec<Rc<RefCell<Task>>>,

    /// The project's top-level task list (excluding child tasks).
    ///
    /// Corresponds to Java field: `private final List<Task> m_childTasks = new ArrayList<>();`
    /// (also the list returned by `ProjectFile.getChildTasks()`)
    child_tasks: Vec<Rc<RefCell<Task>>>,

    /// Task ID → task index.
    ///
    /// Corresponds to Java field: `ProjectEntityWithIDContainer.m_idMap`
    id_map: HashMap<i32, Rc<RefCell<Task>>>,

    /// Task ID sequence generator.
    ///
    /// Corresponds to Java field: `ProjectEntityWithIDContainer.m_idSequence`
    id_sequence: ObjectSequence,

    /// Unique ID conflict list.
    ///
    /// Corresponds to Java field: `ProjectEntityContainer.m_uniqueIDClashList`
    unique_id_clash_list: Vec<Rc<RefCell<Task>>>,

    /// Parent project file (weak reference, to avoid an `Rc` cycle).
    ///
    /// Corresponds to Java field: `private final ProjectFile m_projectFile;`
    /// (also the base class's sequence provider `m_sequenceProvider`)
    context: Weak<RefCell<ProjectFile>>,

    /// Project context (weak reference).
    ///
    /// Rust-specific: Java's `TaskContainer` only holds `m_projectFile` and accesses the context indirectly through it.
    /// The Rust side keeps a separate copy so that the `ProjectConfig` and unique ID sequence needed to "create a task" can be
    /// read **without borrowing `ProjectFile`** — `ProjectContext` is a separate `RefCell`,
    /// so it can be safely accessed even while the caller holds a `ProjectFile` borrow (see
    /// [`TaskContainer::creation_context`]).
    project_context: Weak<RefCell<crate::project_context::ProjectContext>>,
}

impl TaskContainer {
    /// Returns the fields for which at least one task has a non-default cached value.
    ///
    /// Corresponds to Java: `public Set<FieldType> getPopulatedFields()`; while the parent file is borrowed, the
    /// caller passes in the default values and custom fields, avoiding a reverse borrow of the parent file through the weak reference.
    pub fn get_populated_fields(
        &self,
        custom: impl IntoIterator<Item = FieldKey>,
        defaults: crate::common::populated_fields::Defaults,
    ) -> std::collections::HashSet<FieldKey> {
        crate::common::populated_fields::get_populated_fields(
            crate::common::populated_fields::merge_fields(
                crate::task_field::TaskField::ALL.map(FieldKey::Task), custom,
            ),
            self.iter().cloned(),
            defaults,
        )
    }

    /// Constructor.
    ///
    /// Corresponds to Java: `public TaskContainer(ProjectFile projectFile)`
    ///
    /// The Rust side constructs first with an empty parent file weak reference, and [`TaskContainer::set_context`] completes the wiring.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            child_tasks: Vec::new(),
            id_map: HashMap::new(),
            id_sequence: ObjectSequence::new(1),
            unique_id_clash_list: Vec::new(),
            context: Weak::new(),
            project_context: Weak::new(),
        }
    }

    /// Wires up the parent project file (Rust-specific: Java receives `ProjectFile` directly in the constructor).
    ///
    /// Corresponds to Java: `m_projectFile = projectFile;` in the constructor and the base class's `super(projectFile)`
    pub(crate) fn set_context(&mut self, project_file: &Rc<RefCell<ProjectFile>>) {
        self.context = Rc::downgrade(project_file);

        // Rust-specific: also record `ProjectContext` (see the field documentation).
        self.project_context = match project_file.try_borrow() {
            Ok(file_ref) => Rc::downgrade(&file_ref.get_project_context()),
            Err(_) => Weak::new(),
        };
    }

    /// Wires up the project context already held when the parent file was constructed.
    ///
    /// Corresponds to Java: the `ProjectContext` obtained through the parent file in `TaskContainer(ProjectFile)`;
    /// during Rust construction the parent file is being mutably borrowed, so it must be passed in explicitly by the caller that owns the context.
    pub(crate) fn set_project_context(
        &mut self,
        context: &Rc<RefCell<crate::project_context::ProjectContext>>,
    ) {
        self.project_context = Rc::downgrade(context);
    }

    /// Gets the parent project file (returns `None` when not wired up).
    ///
    /// Corresponds to Java: `private final ProjectFile m_projectFile;`
    pub fn get_context(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.context.upgrade()
    }

    /// Reads the context snapshot needed to "create a task" (Rust-specific).
    ///
    /// Corresponds to Java: the `ProjectConfig` switches read directly inside the `Task` constructor,
    /// `file.getUniqueIdObjectSequence(Task.class).getNext()`,
    /// `file.getTasks().getNextID()`, and the
    /// `file.getChildTasks().size()`.
    ///
    /// On the Rust side these values come from `ProjectContext` (configuration and unique ID sequence generator) and the container itself
    /// (ID sequence, top-level task count), and **all bypass `ProjectFile`** — so this method can be called while the caller holds
    /// a `ProjectFile` borrow.
    fn creation_context(&self) -> TaskCreationContext {
        let mut creation = TaskCreationContext::default();

        if let Some(project_context) = self.project_context.upgrade() {
            // Java: ProjectConfig config = file.getProjectConfig(); (these five auto switches)
            if let Ok(project_context_ref) = project_context.try_borrow() {
                let config = project_context_ref.get_project_config();
                creation.auto_task_unique_id = config.get_auto_task_unique_id();
                creation.auto_task_id = config.get_auto_task_id();
                creation.auto_wbs = config.get_auto_wbs();
                creation.auto_outline_number = config.get_auto_outline_number();
                creation.auto_outline_level = config.get_auto_outline_level();
            }

            if creation.auto_task_unique_id {
                // Java: file.getUniqueIdObjectSequence(Task.class).getNext();
                // (the sequence generator uses a Cell internally; once the Rc is obtained it can be advanced without holding a context borrow)
                if let Ok(mut project_context_ref) = project_context.try_borrow_mut() {
                    creation.next_unique_id =
                        crate::unique_id_object_sequence_provider::get_unique_id_object_sequence::<
                            Task,
                        >(&mut *project_context_ref)
                        .get_next();
                }
            }
        }

        if creation.auto_task_id {
            // Java: file.getTasks().getNextID(); — that is, this container's own ID sequence generator
            creation.next_id = self.get_next_id();
        }

        // Java: file.getChildTasks().size(); (at this point the current task has not yet been registered in child_tasks)
        creation.top_level_task_count = self.child_tasks.len();

        creation
    }

    /// Gets the project's top-level task list.
    ///
    /// Corresponds to Java: `@Override public List<Task> getChildTasks()`(`ChildTaskContainer`)
    pub fn get_child_tasks(&self) -> &[Rc<RefCell<Task>>] {
        &self.child_tasks
    }

    /// Gets a mutable reference to the project's top-level task list (Rust-specific: Java's `getChildTasks()` returns a mutable list).
    pub fn get_child_tasks_mut(&mut self) -> &mut Vec<Rc<RefCell<Task>>> {
        &mut self.child_tasks
    }

    /// Appends an existing task to the container.
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(T element)` (which in turn triggers the base class's `added`)
    ///
    /// Rust has no overloading, so the element variant and the no-argument variant [`TaskContainer::add`] are distinguished by name
    /// (consistent with the approach of [`crate::project_calendar_container::ProjectCalendarContainer::add_calendar`]).
    /// Note: this method does **not** add the task to the project's top-level task list (in Java the caller separately calls
    /// `m_projectFile.getChildTasks().add(task)`); when that behavior is needed, use
    /// [`TaskContainer::get_child_tasks_mut`].
    pub fn add_task(&mut self, task: Rc<RefCell<Task>>) {
        // Rust-specific: register the task's own shared handle, so that `Task::set_id` can restore Java's
        // `mapID(val, this)` callback in `setID` (see `Task::set_self_reference`).
        if let Ok(mut task_ref) = task.try_borrow_mut() {
            task_ref.set_self_reference(&task);
        }

        // Java: `setID(...)` in the `Task` constructor immediately calls `mapID(...)`; on the Rust side, at construction time
        // the `Rc<RefCell<Task>>` is not yet available (`set_self_reference` needs it), so that callback is a no-op,
        // so the registration is performed once more here — equivalent to Java's final state (entering the container registers the current ID).
        if let Some(id) = read_task(&task, |task| task.get_id()).flatten() {
            self.map_id(id, Rc::clone(&task));
        }

        self.items.push(Rc::clone(&task));
        self.added(&task);
    }

    /// Creates a new task and adds it to the container (also registering it as a project top-level task).
    ///
    /// Corresponds to Java: `public Task add()` (the implementation body of `ChildTaskContainer.addTask()`)
    pub fn add(&mut self) -> Rc<RefCell<Task>> {
        // Java: Task task = new Task(m_projectFile, null);
        let task = match self.context.upgrade() {
            Some(project_file) => {
                // Rust-specific: first read the context snapshot needed to create a task (taken from `ProjectContext` and
                // the container itself, **without touching `ProjectFile`**), then construct the task — so this method can be
                // called while the caller holds a `ProjectFile` borrow (Java has no such restriction).
                let creation = self.creation_context();
                Rc::new(RefCell::new(Task::new_in_file_with_context(
                    &project_file,
                    None,
                    creation,
                )))
            }
            // In Java m_projectFile always exists; on the Rust side, when not wired up it degrades gracefully to a task without a parent file (see the file header)
            None => Rc::new(RefCell::new(Task::new())),
        };

        self.add_task(Rc::clone(&task));

        // Java: m_projectFile.getChildTasks().add(task);
        // ProjectFile.getChildTasks() is this container's m_childTasks (see the file header)
        self.child_tasks.push(Rc::clone(&task));

        task
    }

    /// Cleanup after a task is removed: remove it from the ID index, the parent task's child task list, and the project's top-level task list,
    /// and recursively remove its child tasks.
    ///
    /// Corresponds to Java: `@Override public void removed(Task task)`
    pub fn removed(&mut self, task: &Rc<RefCell<Task>>) {
        //
        // Java: super.removed(task); — the base class removes the element from the unique ID index.
        // The Rust side uses linear search by unique ID (see the file header), so there is no index to maintain.
        //

        // Java: m_idMap.remove(task.getID());
        if let Some(id) = read_task(task, |task| task.get_id()).flatten() {
            self.id_map.remove(&id);
        }

        // Java: Task parentTask = task.getParentTask();
        //       if (parentTask != null) { parentTask.removeChildTask(task); }
        //       else { m_projectFile.getChildTasks().remove(task); }
        let parent_task = read_task(task, |task| task.get_parent_task()).flatten();
        match parent_task {
            // Java: parentTask.removeChildTask(task);
            // (`Task::remove_child_task` is an associated function with `this` as the first parameter, see the `crate::task` file header)
            Some(parent_task) => Task::remove_child_task(&parent_task, task),
            // Java: m_projectFile.getChildTasks().remove(task);
            None => {
                self.child_tasks.retain(|child| !Rc::ptr_eq(child, task));
            }
        }

        //
        // Java:
        // Iterator<ResourceAssignment> iter = m_projectFile.getResourceAssignments().iterator();
        // while (iter.hasNext()) { ResourceAssignment assignment = iter.next();
        //    if (assignment.getTaskUniqueID().equals(task.getUniqueID())) { iter.remove(); } }
        //
        // Java's `m_projectFile.getResourceAssignments()` is the parent file's resource assignment container,
        // and the Rust side obtains it through `context` (the weak parent `ProjectFile` reference). When the task unique ID is `null`
        // Java's `equals` never matches (`Integer.equals(null)` is false), so it is skipped here;
        // when the caller holds a `ProjectFile` borrow the cleanup cannot be performed (Java has no such case) and degrades gracefully to skipping it rather than panicking.
        if let Some(task_unique_id) = read_task(task, |task| task.get_unique_id()).flatten() {
            if let Some(project_file) = self.context.upgrade() {
                if let Ok(mut project_file) = project_file.try_borrow_mut() {
                    let assignments = project_file.get_resource_assignments_mut();

                    // Collect first, then remove: `remove` needs `&mut`, which cannot coexist with the borrow used for iteration.
                    let targets: Vec<Rc<RefCell<ResourceAssignment>>> = assignments
                        .iter()
                        .filter(|assignment| {
                            assignment
                                .try_borrow()
                                .ok()
                                .and_then(|assignment| assignment.get_task_unique_id())
                                == Some(task_unique_id)
                        })
                        .cloned()
                        .collect();

                    for assignment in targets {
                        assignments.remove(&assignment);
                    }
                }
            }
        }

        //
        // Java: recursively remove child tasks
        // while (true) { List<Task> childTaskList = task.getChildTasks();
        //    if (childTaskList.isEmpty()) break; remove(childTaskList.get(0)); }
        //
        // Java's loop relies on "removed removes the child task from the **parent task**'s child task list" (via child.m_parent),
        // while the Rust side iterates over a snapshot of the child tasks: the removal set matches the recursive order and does not rely on the child task's back-pointer.
        let child_tasks: Vec<Rc<RefCell<Task>>> =
            read_task(task, |task| task.get_child_tasks().iter().cloned().collect())
                .unwrap_or_default();

        for child_task in child_tasks {
            self.remove_task(&child_task);
        }
    }

    /// Renumbers task IDs in ID order (Microsoft Project determines the display order from the ID),
    /// and synchronizes the tasks' outline level.
    ///
    /// Corresponds to Java: `public void synchronizeTaskIDToHierarchy()`
    pub fn synchronize_task_id_to_hierarchy(&mut self) {
        // Java: int currentID = (getByID(Integer.valueOf(0)) == null ? 1 : 0);
        let current_id = self.first_unique_id();

        // Java: synchroizeTaskIDToHierarchy(m_projectFile, currentID, currentID);
        // ProjectFile.getChildTasks() is this container's m_childTasks (see the file header)
        let child_tasks = self.child_tasks.clone();
        self.synchronize_task_id_to_hierarchy_for(child_tasks, current_id, current_id);
    }

    /// Recursively renumbers child task IDs and updates the outline level.
    ///
    /// Corresponds to Java: `private int synchroizeTaskIDToHierarchy(ChildTaskContainer parentTask, int currentID, int currentLevel)`
    ///
    /// Java uses `ChildTaskContainer` to unify the two kinds of parent container, "project file" and "task"; the Rust side instead
    /// receives a child task list directly (see the file header).
    fn synchronize_task_id_to_hierarchy_for(
        &mut self,
        tasks: Vec<Rc<RefCell<Task>>>,
        current_id: i32,
        current_level: i32,
    ) -> i32 {
        let mut current_id = current_id;

        for task in tasks {
            // Java: task.setID(Integer.valueOf(currentID)); task.setOutlineLevel(Integer.valueOf(currentLevel));
            if let Ok(mut task_ref) = task.try_borrow_mut() {
                task_ref.set_id(Some(current_id));
                task_ref.set_outline_level(Some(current_level));
            }

            // Java: currentID = synchroizeTaskIDToHierarchy(task, currentID + 1, currentLevel + 1);
            let children: Vec<Rc<RefCell<Task>>> =
                read_task(&task, |task| task.get_child_tasks().iter().cloned().collect())
                    .unwrap_or_default();
            current_id =
                self.synchronize_task_id_to_hierarchy_for(children, current_id + 1, current_level + 1);
        }

        current_id
    }

    /// Rebuilds the project's hierarchy (parent / child tasks, WBS, outline numbering) from the tasks' `outlineLevel` field.
    ///
    /// Corresponds to Java: `void updateStructure()`
    ///
    /// Java calls back into `m_projectFile.getProjectConfig()` inside this method to read the three auto-numbering switches;
    /// the Rust side instead has the caller [`crate::ProjectFile::update_structure`] pass them in explicitly —
    /// `ProjectConfig` lives in `ProjectContext`'s `RefCell`, and the caller reading it does not conflict with
    /// "holding a `ProjectFile` borrow", while the container itself cannot safely call back into the parent file while it is mutably borrowed.
    ///
    /// This method therefore **never calls back into the parent file**: it can be called directly on the container while `ProjectFile`
    /// is already borrowed, without temporarily moving the container out of `ProjectFile`.
    pub(crate) fn update_structure(
        &mut self,
        auto_wbs: bool,
        auto_outline_number: bool,
        auto_outline_level: bool,
    ) {
        if self.items.len() <= 1 {
            return;
        }

        // Java: Collections.sort(this); (Task.compareTo compares by ID)
        self.sort();

        // Java: m_projectFile.getChildTasks().clear();
        // ProjectFile.getChildTasks() is this container's m_childTasks (see the file header)
        self.child_tasks.clear();

        let mut last_task: Option<Rc<RefCell<Task>>> = None;
        let mut last_level: i32 = -1;

        // Clone the list first: the loop body modifies child_tasks (Java iterates over the same list, but read-only)
        for task in self.items.clone() {
            // Java: task.clearChildTasks();
            // (`Task::clear_child_tasks` is an associated function with `this` as the first parameter)
            Task::clear_child_tasks(&task);

            let mut parent: Option<Rc<RefCell<Task>>> = None;

            // Java: if (!task.getNull()) { ... }
            let is_null = read_task(&task, |task| task.get_null()).unwrap_or(false);
            if !is_null {
                let mut level = read_task(&task, |task| task.get_outline_level())
                    .flatten()
                    .unwrap_or(0);

                if let Some(last_task_ref) = &last_task {
                    if level == last_level {
                        // Java: parent = lastTask.getParentTask(); level = lastLevel;
                        parent =
                            read_task(last_task_ref, |last| last.get_parent_task()).flatten();
                        level = last_level;
                    } else if level > last_level {
                        // Java: parent = lastTask;
                        parent = Some(Rc::clone(last_task_ref));
                    } else {
                        // Java: while (level <= lastLevel) { parent = lastTask.getParentTask();
                        //          if (parent == null) break;
                        //          lastLevel = NumberHelper.getInt(parent.getOutlineLevel());
                        //          lastTask = parent; }
                        let mut cursor = Rc::clone(last_task_ref);
                        while level <= last_level {
                            let next = read_task(&cursor, |cursor| cursor.get_parent_task()).flatten();
                            let Some(next) = next else {
                                parent = None;
                                break;
                            };

                            last_level = read_task(&next, |next| next.get_outline_level())
                                .flatten()
                                .unwrap_or(0);
                            cursor = Rc::clone(&next);
                            parent = Some(Rc::clone(&next));
                        }
                    }
                }

                last_task = Some(Rc::clone(&task));
                last_level = level;

                // Java: if (autoWbs || task.getWBS() == null) { task.generateWBS(parent); }
                //
                // Java's `generateWBS(null)` reads `m_projectFile.getChildTasks().size()`;
                // this method is exactly the authoritative source of that count (`TaskContainer.child_tasks` is
                // `ProjectFile.getChildTasks()`), and at this moment the caller may be holding a `ProjectFile`
                // borrow and cannot safely call back, so it is passed in explicitly (fully consistent with Java's value: the current task has not yet
                // been registered in child_tasks, so len() is "the number of top-level tasks already registered before it").
                let has_wbs = read_task(&task, |task| task.get_wbs().is_some()).unwrap_or(false);
                if auto_wbs || !has_wbs {
                    if let Ok(mut task_ref) = task.try_borrow_mut() {
                        task_ref.generate_wbs_with_count(parent.as_ref(), self.child_tasks.len());
                    }
                }

                // Java: if (autoOutlineNumber) { task.generateOutlineNumber(parent); }
                if auto_outline_number {
                    if let Ok(mut task_ref) = task.try_borrow_mut() {
                        task_ref.generate_outline_number_with_count(
                            parent.as_ref(),
                            self.child_tasks.len(),
                        );
                    }
                }
            }

            // Java: if (parent == null) { m_projectFile.getChildTasks().add(task); }
            //       else { parent.addChildTask(task); }
            match parent {
                // Java: parent.addChildTask(task);
                // (`Task::add_child_task` is an associated function with `this` as the first parameter;
                //   the already-read `autoOutlineLevel` is passed in here to avoid calling back into the parent file)
                Some(parent) => {
                    Task::add_child_task_with_auto_outline_level(&parent, &task, auto_outline_level)
                }
                // Java: m_projectFile.getChildTasks().add(task);
                None => self.child_tasks.push(Rc::clone(&task)),
            }
        }
    }

    /// Gets the task custom field list.
    ///
    /// Corresponds to Java: `public List<CustomField> getCustomFields()`
    ///
    /// Java: `m_projectFile.getCustomFields().getCustomFieldsByFieldTypeClass(FieldTypeClass.TASK)`;
    /// Returns an empty list when the parent file is not wired up (in Java it is always available).
    pub fn get_custom_fields(&self) -> Vec<Rc<RefCell<CustomField>>> {
        let Some(custom_fields) = self.custom_field_container() else {
            return Vec::new();
        };

        let result = custom_fields
            .borrow()
            .get_custom_fields_by_field_type_class(FieldTypeClass::Task);
        result
    }

    /// Gets the field type by alias.
    ///
    /// Corresponds to Java: `public FieldType getFieldTypeByAlias(String alias)`
    ///
    /// Java returns `FieldType`, the Rust side returns [`FieldKey`] (see the file header); returns `None` when the parent file is not wired up.
    pub fn get_field_type_by_alias(&self, alias: &str) -> Option<FieldKey> {
        let custom_fields = self.custom_field_container()?;
        let field_type = custom_fields
            .borrow()
            .get_field_type_by_alias(FieldTypeClass::Task, alias);
        field_type
    }

    /// The first value used when renumbering unique IDs: starts from 0 if a task with ID 0 exists, otherwise from 1.
    ///
    /// Corresponds to Java: `@Override protected int firstUniqueID()`
    pub fn first_unique_id(&self) -> i32 {
        // Java: Task firstEntity = getByID(Integer.valueOf(0));
        //       return firstEntity == null ? 1 : 0;
        if self.get_by_id(Some(0)).is_none() {
            1
        } else {
            0
        }
    }

    /// Finds a task by ID; returns `None` when not found.
    ///
    /// Corresponds to Java: `ProjectEntityWithIDContainer.getByID(Integer id)`
    pub fn get_by_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Task>>> {
        let id = id?;
        self.id_map.get(&id).cloned()
    }

    /// Registers an ID mapping.
    ///
    /// Corresponds to Java: `ProjectEntityWithIDContainer.mapID(Integer id, T entity)`
    pub fn map_id(&mut self, id: i32, task: Rc<RefCell<Task>>) {
        self.id_map.insert(id, task);
    }

    /// Removes an ID mapping.
    ///
    /// Corresponds to Java: `ProjectEntityWithIDContainer.unmapID(Integer id)`
    pub fn unmap_id(&mut self, id: Option<i32>) {
        if let Some(id) = id {
            self.id_map.remove(&id);
        }
    }

    /// Gets the next available task ID.
    ///
    /// Corresponds to Java: `ProjectEntityWithIDContainer.getNextID()`
    pub fn get_next_id(&self) -> i32 {
        self.id_sequence.get_next()
    }

    /// Makes all task IDs consecutive (sort by ID first, then assign in order).
    ///
    /// Corresponds to Java: `ProjectEntityWithIDContainer.renumberIDs()`
    ///
    /// In Java `Collections.sort` triggers the `replaced` callback through `set(int, T)`; `TaskContainer`
    /// does not override `replaced`, so there is no additional effect, and the Rust side sorts directly.
    pub fn renumber_ids(&mut self) {
        if self.items.is_empty() {
            return;
        }

        // Java: Collections.sort(this);
        self.sort();

        // Java: int id = NumberHelper.getInt(firstEntity.getID()); if (id != 0) { id = 1; }
        let first_id = self
            .items
            .first()
            .and_then(|task| read_task(task, |task| task.get_id()))
            .flatten()
            .unwrap_or(0);
        let mut id = if first_id != 0 { 1 } else { 0 };

        for task in self.items.clone() {
            if let Ok(mut task_ref) = task.try_borrow_mut() {
                task_ref.set_id(Some(id));
            }
            id += 1;
        }
    }

    /// Finds a task by unique ID; returns `None` when not found.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    ///
    /// The Rust side uses linear search by unique ID (no index, see the file header).
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Task>>> {
        let id = id?;
        self.items
            .iter()
            .find(|task| read_task(task, |task| task.get_unique_id()).flatten() == Some(id))
            .map(Rc::clone)
    }

    /// Updates the container state when a task's unique ID changes.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.updateUniqueID(T element, Integer oldUniqueID, Integer newUniqueID)`
    ///
    /// The Rust side uses linear search by unique ID, so it only needs to maintain the conflict list and the sequence generator.
    pub fn update_unique_id(
        &mut self,
        element: &Rc<RefCell<Task>>,
        old_unique_id: Option<i32>,
        new_unique_id: Option<i32>,
    ) {
        // Java: if (oldUniqueID != null) { m_uniqueIDMap.remove(oldUniqueID); }
        // The Rust side uses linear search, so there is no index entry to remove.
        let _ = old_unique_id;

        // Java: T currentElement = m_uniqueIDMap.get(newUniqueID);
        //       if (currentElement == element) return;
        //       if (currentElement instanceof ProjectEntityWithMutableUniqueID)
        //       { m_uniqueIDClashList.add(element); }
        // A Task's unique ID is mutable, so a conflict is recorded consistently with Java.
        if self
            .find_other_with_unique_id(new_unique_id, element)
            .is_some()
        {
            self.unique_id_clash_list.push(Rc::clone(element));
        }

        // Java: m_uniqueIDMap.put(newUniqueID, element);
        //       m_sequenceProvider.getUniqueIdObjectSequence(element.getClass()).sync(newUniqueID);
        if let Some(sequence) = self.sequence() {
            sequence.sync(new_unique_id);
        }
    }

    /// Reallocates unique IDs for tasks that have unique ID conflicts.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.fixUniqueIdClashes()`
    pub fn fix_unique_id_clashes(&mut self) {
        if self.unique_id_clash_list.is_empty() {
            return;
        }

        // Java: ObjectSequence sequence = m_sequenceProvider.getUniqueIdObjectSequence(
        //          m_uniqueIDClashList.get(0).getClass());
        // On the Rust side, when the parent file is not wired up the sequence generator cannot be obtained (in Java it is always available),
        // so the conflict list is kept and processed once wiring is complete.
        let Some(sequence) = self.sequence() else {
            return;
        };

        // Java: m_uniqueIDClashList.forEach(i -> i.setUniqueID(sequence.getNext()));
        for task in std::mem::take(&mut self.unique_id_clash_list) {
            if let Ok(mut task_ref) = task.try_borrow_mut() {
                task_ref.set_unique_id(Some(sequence.get_next()));
            }
        }

        // Java: m_uniqueIDMap.clear(); forEach(i -> m_uniqueIDMap.put(...));
        // The Rust side uses linear search by unique ID, so there is no index to rebuild.
    }

    /// Inserts a task at the given index (returns `false` if out of bounds).
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(int index, T element)`
    pub fn add_at(&mut self, index: usize, task: Rc<RefCell<Task>>) -> bool {
        if index > self.items.len() {
            return false;
        }

        self.items.insert(index, Rc::clone(&task));
        self.added(&task);
        true
    }

    /// Replaces the task at the given index, returning the old task that was replaced (returns `None` if out of bounds).
    ///
    /// Corresponds to Java: `ListWithCallbacks.set(int index, T element)`
    ///
    /// Consistent with Java: `TaskContainer` does not override `replaced`, so replacement does **not** update the ID / unique ID index.
    pub fn set(&mut self, index: usize, task: Rc<RefCell<Task>>) -> Option<Rc<RefCell<Task>>> {
        if index >= self.items.len() {
            return None;
        }

        Some(std::mem::replace(&mut self.items[index], task))
    }

    /// Removes and returns the task at the given index (returns `None` if out of bounds).
    ///
    /// Corresponds to Java: `ListWithCallbacks.remove(int index)`
    pub fn remove_at(&mut self, index: usize) -> Option<Rc<RefCell<Task>>> {
        if index >= self.items.len() {
            return None;
        }

        let removed = self.items.remove(index);
        self.removed(&removed);
        Some(removed)
    }

    /// Removes a task by its identity (equivalent to Java's `ListWithCallbacks.remove(Object)`).
    ///
    /// Corresponds to Java: `ListWithCallbacks.remove(Object o)`
    pub fn remove_task(&mut self, task: &Rc<RefCell<Task>>) -> bool {
        let index = self
            .items
            .iter()
            .position(|candidate| Rc::ptr_eq(candidate, task));

        match index {
            Some(index) => {
                let removed = self.items.remove(index);
                self.removed(&removed);
                true
            }
            None => false,
        }
    }

    /// Clears the task list (does not trigger `removed`, consistent with Java).
    ///
    /// Corresponds to Java: `ListWithCallbacks.clear()`
    ///
    /// Same as Java: `clear()` does not clear `m_idMap` (and therefore does not clear the Rust side's `id_map`).
    /// The top-level task list also remains unchanged as in Java (`updateStructure` clears it explicitly).
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Gets the task at the given index; returns `None` if out of bounds.
    ///
    /// Corresponds to Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<Rc<RefCell<Task>>> {
        self.items.get(index).map(Rc::clone)
    }

    /// The number of tasks.
    ///
    /// Corresponds to Java: `ListWithCallbacks.size()`
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the container is empty.
    ///
    /// Corresponds to Java: `ListWithCallbacks.isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Iterates over the tasks in the container.
    ///
    /// Corresponds to Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<Task>>> {
        self.items.iter()
    }

    /// Accesses the tasks in the container as a slice.
    ///
    /// Corresponds to Java: the base class's (`ArrayList`) list view.
    pub fn as_slice(&self) -> &[Rc<RefCell<Task>>] {
        self.items.as_slice()
    }

    /// Sorts in place by task ID (equivalent to Java's `Collections.sort(this)`).
    ///
    /// Corresponds to Java: `Collections.sort(this)` in `ProjectEntityWithIDContainer`
    /// (compared by ID via `Task.compareTo`)
    pub fn sort(&mut self) {
        self.items
            .sort_by_key(|task| read_task(task, |task| task.get_id()).flatten().unwrap_or(0));
    }

    /// Index maintenance after a task is added.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.added(T element)`
    fn added(&mut self, element: &Rc<RefCell<Task>>) {
        // Java: if (element.getUniqueID() == null) return;
        let Some(unique_id) = read_task(element, |task| task.get_unique_id()).flatten() else {
            return;
        };

        // Java: T currentElement = m_uniqueIDMap.get(uniqueID); if (currentElement == element) return;
        //       if (currentElement instanceof ProjectEntityWithMutableUniqueID)
        //       { m_uniqueIDClashList.add(element); }
        // A Task's unique ID is mutable, so a conflict is recorded consistently with Java.
        if self.find_other_with_unique_id(Some(unique_id), element).is_some() {
            self.unique_id_clash_list.push(Rc::clone(element));
        }

        // Java: m_uniqueIDMap.put(element.getUniqueID(), element);
        // The Rust side uses linear search by unique ID, so there is no index to write.
    }

    /// Gets the `Task` sequence generator provided by the parent project file (returns `None` when the parent file is not wired up or is already borrowed).
    ///
    /// Corresponds to Java: `m_sequenceProvider.getUniqueIdObjectSequence(Task.class)`
    fn sequence(&self) -> Option<Rc<ObjectSequence>> {
        let project_file = self.context.upgrade()?;
        let mut project_file = project_file.try_borrow_mut().ok()?;

        // Compatible with two forms: ProjectFile implementing `UniqueIdObjectSequenceProvider`
        // (as in Java), or providing an inherent method with the same signature (the assumption in `crate::task`).
        Some(project_file.get_unique_id_object_sequence(TypeId::of::<Task>()))
    }

    /// Gets the custom field container of the parent project file.
    ///
    /// Corresponds to Java: `m_projectFile.getCustomFields()`
    fn custom_field_container(&self) -> Option<Rc<RefCell<CustomFieldContainer>>> {
        let project_file = self.context.upgrade()?;
        let project_file = project_file.try_borrow().ok()?;
        Some(project_file.get_custom_fields())
    }

    /// Finds another task occupying the given unique ID (Java uses the unique ID index to determine "that ID is already occupied").
    ///
    /// Java's index stores the **last** element added when the same unique ID is registered multiple times; the Rust side takes, in list order,
    /// the first **other** element with the same ID, equivalently expressing the "already occupied" determination (this can only affect the
    /// order of elements in the conflict list, not the final allocation result).
    fn find_other_with_unique_id(
        &self,
        unique_id: Option<i32>,
        exclude: &Rc<RefCell<Task>>,
    ) -> Option<Rc<RefCell<Task>>> {
        let unique_id = unique_id?;
        self.items
            .iter()
            .find(|task| {
                !Rc::ptr_eq(task, exclude)
                    && read_task(task, |task| task.get_unique_id()).flatten() == Some(unique_id)
            })
            .map(Rc::clone)
    }
}

impl Default for TaskContainer {
    /// Corresponds to Java: the explicitly declared `TaskContainer(ProjectFile)`,
    /// equivalent on the Rust side to [`TaskContainer::new`] (the parent file is wired up by `set_context`).
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for TaskContainer {
    /// Java does not override `toString()`; the Rust side outputs the container size, top-level task count, and parent file wiring state
    /// (to avoid depending on `Task`'s `Debug` implementation).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskContainer")
            .field("size", &self.items.len())
            .field("child_task_count", &self.child_tasks.len())
            .field("context_is_set", &(self.context.strong_count() > 0))
            .finish()
    }
}

/// Reads a task field with `try_borrow`, returning `None` when it is already mutably borrowed (without panicking).
fn read_task<T>(task: &Rc<RefCell<Task>>, read: impl FnOnce(&Task) -> T) -> Option<T> {
    task.try_borrow().ok().map(|task| read(&task))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wires up to the new parent file's task container.
    fn container() -> (TaskContainer, Rc<RefCell<ProjectFile>>) {
        let project_file = ProjectFile::new();
        let mut container = TaskContainer::new();
        container.set_context(&project_file);
        (container, project_file)
    }

    /// Creates a new task under the given parent file (not added to the container).
    fn task(project_file: &Rc<RefCell<ProjectFile>>) -> Rc<RefCell<Task>> {
        Rc::new(RefCell::new(Task::new_in_file(project_file, None)))
    }

    /// Creates a new task with an ID and outline level under the given parent file, and adds it to the container.
    fn add_task(
        container: &mut TaskContainer,
        project_file: &Rc<RefCell<ProjectFile>>,
        id: i32,
        outline_level: i32,
    ) -> Rc<RefCell<Task>> {
        let task = task(project_file);
        {
            let mut task_ref = task.borrow_mut();
            task_ref.set_id(Some(id));
            task_ref.set_outline_level(Some(outline_level));
        }
        container.add_task(Rc::clone(&task));
        task
    }

    #[test]
    fn add_creates_task_and_registers_project_child_task() {
        let (mut container, _) = container();
        assert!(container.is_empty());

        let task = container.add();

        assert_eq!(container.len(), 1);
        assert!(!container.is_empty());
        assert_eq!(container.get_child_tasks().len(), 1);
        assert!(Rc::ptr_eq(
            &container.get(0).expect("index 0 exists"),
            &task
        ));
        assert!(Rc::ptr_eq(
            &container.get_child_tasks()[0],
            &container.get(0).expect("index 0 exists")
        ));
        assert!(container.get(1).is_none());
    }

    #[test]
    fn add_without_context_still_creates_task() {
        // In Java m_projectFile always exists; on the Rust side, when not wired up it degrades gracefully to a task without a parent file (see the file header)
        let mut container = TaskContainer::new();
        let task = container.add();

        assert_eq!(container.len(), 1);
        assert_eq!(container.get_child_tasks().len(), 1);
        assert_eq!(read_task(&task, |task| task.get_id()).flatten(), None);
    }

    #[test]
    fn list_operations_round_trip() {
        let (mut container, project_file) = container();
        let first = task(&project_file);
        let second = task(&project_file);
        let third = task(&project_file);

        container.add_task(Rc::clone(&first));
        container.add_task(Rc::clone(&second));
        assert_eq!(container.len(), 2);
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);

        // add_at insertion
        assert!(container.add_at(1, Rc::clone(&third)));
        assert!(Rc::ptr_eq(&container.get(1).expect("index 1 exists"), &third));
        assert!(!container.add_at(9, Rc::clone(&first)));

        // set replacement (consistent with Java: does not update the index)
        let replacement = task(&project_file);
        let old = container.set(0, Rc::clone(&replacement)).expect("index 0 exists");
        assert!(Rc::ptr_eq(&old, &first));
        assert!(container.set(9, Rc::clone(&first)).is_none());

        // remove_at removal
        let removed = container.remove_at(0).expect("index 0 exists");
        assert!(Rc::ptr_eq(&removed, &replacement));
        assert!(container.remove_at(9).is_none());

        // remove_task removal by identity
        assert!(container.remove_task(&third));
        assert!(!container.remove_task(&third));

        // clear the list (consistent with Java: does not clear the ID index)
        container.clear();
        assert!(container.is_empty());
    }

    #[test]
    fn id_index_and_sequence() {
        let (mut container, project_file) = container();
        let task = task(&project_file);

        assert!(container.get_by_id(Some(7)).is_none());
        assert!(container.get_by_id(None).is_none());

        container.map_id(7, Rc::clone(&task));
        assert!(Rc::ptr_eq(
            &container.get_by_id(Some(7)).expect("mapped"),
            &task
        ));

        container.unmap_id(Some(7));
        assert!(container.get_by_id(Some(7)).is_none());
        container.unmap_id(None);

        // Java: ProjectEntityWithIDContainer.m_idSequence starts from 1
        assert_eq!(container.get_next_id(), 1);
        assert_eq!(container.get_next_id(), 2);
    }

    #[test]
    fn first_unique_id_depends_on_task_zero() {
        let (mut container, project_file) = container();
        assert_eq!(container.first_unique_id(), 1);

        let zero = task(&project_file);
        zero.borrow_mut().set_id(Some(0));
        container.map_id(0, Rc::clone(&zero));
        assert_eq!(container.first_unique_id(), 0);
    }

    #[test]
    fn unique_id_lookup_and_clash_resolution() {
        let (mut container, project_file) = container();

        let first = task(&project_file);
        let second = task(&project_file);
        first.borrow_mut().set_unique_id(Some(3));
        second.borrow_mut().set_unique_id(Some(4));

        container.add_task(Rc::clone(&first));
        container.add_task(Rc::clone(&second));

        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(3)).expect("registered"),
            &first
        ));
        assert!(container.get_by_unique_id(Some(9)).is_none());
        assert!(container.get_by_unique_id(None).is_none());

        // Change second's unique ID to be the same as first → it enters the conflict list
        container.update_unique_id(&second, Some(4), Some(3));
        assert!(container
            .get_by_unique_id(Some(3))
            .map(|task| Rc::ptr_eq(&task, &first))
            .unwrap_or(false));

        container.fix_unique_id_clashes();

        let renumbered =
            read_task(&second, |task| task.get_unique_id()).flatten().expect("still has a unique ID");
        assert_ne!(renumbered, 3);
        assert_eq!(
            read_task(&first, |task| task.get_unique_id()).flatten(),
            Some(3)
        );

        // The sequence generator comes from the parent file (ProjectFile implements UniqueIdObjectSequenceProvider)
        let next = UniqueIdObjectSequenceProvider::get_unique_id_object_sequence(
            &mut *project_file.borrow_mut(),
            TypeId::of::<Task>(),
        )
        .get_next();
        assert!(next > renumbered);
    }

    #[test]
    fn fix_unique_id_clashes_without_context_keeps_clash_list() {
        // Parent file not wired up: Java throws an exception because the sequence generator is unavailable; the Rust side degrades gracefully to doing nothing
        let mut container = TaskContainer::new();
        let project_file = ProjectFile::new();

        let first = task(&project_file);
        let second = task(&project_file);
        first.borrow_mut().set_unique_id(Some(1));
        second.borrow_mut().set_unique_id(Some(1));
        container.add_task(Rc::clone(&first));
        container.add_task(Rc::clone(&second));

        container.update_unique_id(&second, Some(1), Some(1));
        container.fix_unique_id_clashes();

        assert_eq!(
            read_task(&second, |task| task.get_unique_id()).flatten(),
            Some(1)
        );
    }

    #[test]
    fn removed_drops_task_and_recurses_into_children() {
        let (mut container, project_file) = container();

        let parent = add_task(&mut container, &project_file, 1, 0);
        let child = task(&project_file);
        let grandchild = task(&project_file);
        Task::add_child_task(&child, &grandchild);
        container.add_task(Rc::clone(&child));
        container.add_task(Rc::clone(&grandchild));
        container.get_child_tasks_mut().push(Rc::clone(&parent));

        // Attach child under parent (Java: Task.addChildTask)
        Task::add_child_task(&parent, &child);

        assert_eq!(container.len(), 3);

        // Java: removed recursively removes child tasks and removes from the parent / project top-level list
        container.remove_task(&parent);

        assert!(container.is_empty());
        assert!(container.get_child_tasks().is_empty());
        assert!(parent.borrow().get_child_tasks().is_empty());
    }

    #[test]
    fn update_structure_builds_hierarchy_from_outline_levels() {
        let (mut container, project_file) = container();

        // Hierarchy: 0 / 1 / 2 / 0 (the last task returns to the top level)
        let root = add_task(&mut container, &project_file, 1, 0);
        let child = add_task(&mut container, &project_file, 2, 1);
        let grandchild = add_task(&mut container, &project_file, 3, 2);
        let second_root = add_task(&mut container, &project_file, 4, 0);

        container.update_structure(true, true, true);

        // Top-level tasks: root and second_root
        let root_tasks = container.get_child_tasks();
        assert_eq!(root_tasks.len(), 2);
        assert!(Rc::ptr_eq(&root_tasks[0], &root));
        assert!(Rc::ptr_eq(&root_tasks[1], &second_root));

        // root → child → grandchild
        let root_children = root.borrow().get_child_tasks();
        assert_eq!(root_children.len(), 1);
        assert!(Rc::ptr_eq(&root_children[0], &child));

        let child_children = child.borrow().get_child_tasks();
        assert_eq!(child_children.len(), 1);
        assert!(Rc::ptr_eq(&child_children[0], &grandchild));

        // Java: autoWbs / autoOutlineNumber are enabled by default → generate WBS and outline numbering
        assert!(read_task(&root, |task| task.get_wbs().is_some()).unwrap_or(false));
        assert!(read_task(&child, |task| task.get_wbs().is_some()).unwrap_or(false));
        assert!(read_task(&grandchild, |task| task.get_wbs().is_some()).unwrap_or(false));
    }

    /// Java: `removed(Task)` deletes all resource assignments of that task
    /// (`assignment.getTaskUniqueID().equals(task.getUniqueID())`).
    #[test]
    fn removed_deletes_the_tasks_resource_assignments() {
        let (mut container, project_file) = container();

        let task = add_task(&mut container, &project_file, 1, 0);
        task.borrow_mut().set_unique_id(Some(1));

        let other = add_task(&mut container, &project_file, 2, 0);
        other.borrow_mut().set_unique_id(Some(2));

        let matching = Rc::new(RefCell::new(ResourceAssignment::new()));
        matching.borrow_mut().set_task_unique_id(Some(1));
        let other_task_assignment = Rc::new(RefCell::new(ResourceAssignment::new()));
        other_task_assignment.borrow_mut().set_task_unique_id(Some(2));
        let unassigned = Rc::new(RefCell::new(ResourceAssignment::new()));

        {
            let mut file_ref = project_file.borrow_mut();
            let assignments = file_ref.get_resource_assignments_mut();
            assignments.add(Rc::clone(&matching));
            assignments.add(Rc::clone(&other_task_assignment));
            assignments.add(Rc::clone(&unassigned));
        }

        container.remove_task(&task);

        // Delete only the one whose taskUniqueID matches
        let assignments = project_file.borrow();
        let remaining = assignments.get_resource_assignments().as_slice();
        assert_eq!(remaining.len(), 2);
        assert!(remaining
            .iter()
            .any(|assignment| Rc::ptr_eq(assignment, &other_task_assignment)));
        assert!(remaining
            .iter()
            .any(|assignment| Rc::ptr_eq(assignment, &unassigned)));
        assert!(!remaining
            .iter()
            .any(|assignment| Rc::ptr_eq(assignment, &matching)));
    }

    #[test]
    fn update_structure_is_noop_for_single_task() {
        let (mut container, project_file) = container();
        let only = add_task(&mut container, &project_file, 1, 0);

        let wbs_before = read_task(&only, |task| task.get_wbs()).flatten();

        container.update_structure(true, true, true);

        assert_eq!(container.len(), 1);
        assert_eq!(container.get_child_tasks().len(), 0);
        // Java: autoWbs is enabled by default, so WBS is already generated when the task is created; updateStructure does not change it for a single task
        assert_eq!(
            read_task(&only, |task| task.get_wbs()).flatten(),
            wbs_before
        );
    }

    #[test]
    fn update_structure_generates_sequential_wbs_for_top_level_tasks() {
        let (mut container, project_file) = container();
        let mut tasks = Vec::new();

        // Simulate "the file has no WBS / outline numbering column": clear the values generated automatically at construction time
        for id in 1..=3 {
            let task = add_task(&mut container, &project_file, id, 0);
            {
                let mut task_ref = task.borrow_mut();
                task_ref.set_wbs(None);
                task_ref.set_outline_number(None);
            }
            tasks.push(task);
        }

        container.update_structure(true, true, true);

        let wbs: Vec<Option<String>> = tasks
            .iter()
            .map(|task| read_task(task, |task| task.get_wbs()).flatten())
            .collect();
        let outline: Vec<Option<String>> = tasks
            .iter()
            .map(|task| read_task(task, |task| task.get_outline_number()).flatten())
            .collect();

        // Java: top-level tasks get 1, 2, 3 in turn, based on getChildTasks().size() + 1 at call time
        assert_eq!(
            wbs,
            vec![
                Some("1".to_owned()),
                Some("2".to_owned()),
                Some("3".to_owned())
            ]
        );
        assert_eq!(outline, wbs);
    }

    #[test]
    fn set_id_registers_the_task_in_the_parent_file_index() {
        let (mut container, project_file) = container();
        let task = add_task(&mut container, &project_file, 7, 0);

        // The ID change happens after the task enters the container — at this point `Task::set_self_reference` is wired up,
        // and `set_id` should call back into the container's map_id / unmap_id just like Java's `setID`
        task.borrow_mut().set_id(Some(9));

        let file = project_file.borrow();
        assert!(
            file.get_task_by_id(Some(9)).is_some(),
            "new ID should be registered in the parent file's task index"
        );
        assert!(
            file.get_task_by_id(Some(7)).is_none(),
            "old ID should be removed from the parent file's task index"
        );
    }

    #[test]
    fn update_structure_is_safe_while_the_file_is_borrowed() {
        // Regression: before the fix, `TaskContainer::update_structure` would re-enter and borrow the parent file through
        // `Task::generate_wbs` / `Task::add_child_task`; if the caller held a `ProjectFile`
        // mutable borrow it would panic, and the caller could only temporarily move the task container out of the file for processing
        // (at the cost of losing the top-level task count and always getting a WBS of 1). This test reproduces that call pattern directly.
        let (mut container, project_file) = container();

        for id in 1..=3 {
            let task = add_task(&mut container, &project_file, id, 0);
            let mut task_ref = task.borrow_mut();
            // Simulate "the file has no WBS / outline numbering column"
            task_ref.set_wbs(None);
            task_ref.set_outline_number(None);
        }

        // Put the container back into the parent file, then call in place **while holding its mutable borrow**
        *project_file.borrow_mut().get_tasks_mut() = container;
        project_file.borrow_mut().update_structure();

        let file = project_file.borrow();
        let wbs: Vec<Option<String>> = file
            .get_tasks()
            .as_slice()
            .iter()
            .map(|task| task.borrow().get_wbs())
            .collect();

        assert_eq!(
            wbs,
            vec![
                Some("1".to_owned()),
                Some("2".to_owned()),
                Some("3".to_owned())
            ]
        );
    }

    #[test]
    fn add_is_safe_while_the_file_is_borrowed() {
        // Regression: before the fix, `TaskContainer::add` would borrow the parent file through `Task::new_in_file`, while the caller
        // (`ProjectFile::add_task`) necessarily held a mutable `ProjectFile` borrow → it would necessarily panic.
        // This test reproduces that call pattern directly.
        let (container, project_file) = container();
        *project_file.borrow_mut().get_tasks_mut() = container;

        let first = project_file.borrow_mut().add_task();
        let second = project_file.borrow_mut().add_task();

        // Automatic assignment really happened (rather than being silently skipped due to a failed borrow)
        assert!(first.borrow().get_unique_id().is_some());
        assert!(second.borrow().get_unique_id().is_some());
        assert_ne!(first.borrow().get_id(), None);
        assert_ne!(second.borrow().get_id(), None);

        // The WBS of top-level tasks increases in turn
        assert_eq!(first.borrow().get_wbs(), Some("1".to_owned()));
        assert_eq!(second.borrow().get_wbs(), Some("2".to_owned()));

        // The automatically assigned task ID has been registered in the index
        let first_id = first.borrow().get_id();
        assert!(project_file.borrow().get_task_by_id(first_id).is_some());
    }

    #[test]
    fn synchronize_task_id_to_hierarchy_renumbers_depth_first() {
        let (mut container, project_file) = container();

        // ID and outline level agree: 5 (top level) → 6 → 7
        let root = add_task(&mut container, &project_file, 5, 0);
        let child = add_task(&mut container, &project_file, 6, 1);
        let grandchild = add_task(&mut container, &project_file, 7, 2);

        container.update_structure(true, true, true);
        container.synchronize_task_id_to_hierarchy();

        // Java: currentID starts from firstUniqueID() (1) and numbers depth-first in order
        assert_eq!(read_task(&root, |task| task.get_id()).flatten(), Some(1));
        assert_eq!(read_task(&child, |task| task.get_id()).flatten(), Some(2));
        assert_eq!(read_task(&grandchild, |task| task.get_id()).flatten(), Some(3));

        // Java: also updates the outline level (currentLevel's initial value equals currentID, i.e. 1)
        assert_eq!(
            read_task(&root, |task| task.get_outline_level()).flatten(),
            Some(1)
        );
        assert_eq!(
            read_task(&child, |task| task.get_outline_level()).flatten(),
            Some(2)
        );
        assert_eq!(
            read_task(&grandchild, |task| task.get_outline_level()).flatten(),
            Some(3)
        );
    }

    #[test]
    fn renumber_ids_starts_at_one_or_zero() {
        let (mut container, project_file) = container();
        let first = add_task(&mut container, &project_file, 5, 0);
        let second = add_task(&mut container, &project_file, 2, 0);

        container.renumber_ids();

        assert_eq!(read_task(&second, |task| task.get_id()).flatten(), Some(1));
        assert_eq!(read_task(&first, |task| task.get_id()).flatten(), Some(2));

        // Starts from 0 when a task with ID 0 exists
        // (container() is no longer called here, since the same-named local variable above would shadow that test helper function)
        let second_project_file = ProjectFile::new();
        let mut second_container = TaskContainer::new();
        second_container.set_context(&second_project_file);
        let zero = add_task(&mut second_container, &second_project_file, 0, 0);
        let other = add_task(&mut second_container, &second_project_file, 3, 0);

        second_container.renumber_ids();

        assert_eq!(read_task(&zero, |task| task.get_id()).flatten(), Some(0));
        assert_eq!(read_task(&other, |task| task.get_id()).flatten(), Some(1));
    }

    #[test]
    fn custom_fields_are_empty_without_configuration() {
        let (container, _) = container();

        assert!(container.get_custom_fields().is_empty());
        assert_eq!(container.get_field_type_by_alias("MyAlias"), None);

        // Also degrades gracefully to empty when the parent file is not wired up
        let container = TaskContainer::new();
        assert!(container.get_custom_fields().is_empty());
        assert_eq!(container.get_field_type_by_alias("MyAlias"), None);
    }

    #[test]
    fn default_matches_new_and_debug_reports_sizes() {
        let container = TaskContainer::default();
        assert!(container.is_empty());
        assert!(container.get_child_tasks().is_empty());

        let text = format!("{container:?}");
        assert!(text.contains("size: 0"));
        assert!(text.contains("child_task_count: 0"));
        assert!(text.contains("context_is_set: false"));
    }

    #[test]
    fn context_can_be_read_back() {
        let (container, project_file) = container();
        assert!(Rc::ptr_eq(
            &container.get_context().expect("wired up"),
            &project_file
        ));
        assert!(TaskContainer::new().get_context().is_none());
    }
}
