//! Corresponds to Java: `org/mpxj/ProjectFile.java`
//!
//! Represents a project schedule. It is the aggregation point of the model layer: it exposes all containers for tasks, resources, assignments,
//! relations, calendars, custom fields and so on, and delegates to [`ProjectContext`] to provide data shared across projects.
//!
//! # Restoring inheritance and interfaces (composition + forwarding)
//!
//! Java: `public final class ProjectFile implements ChildTaskContainer, ChildResourceContainer,
//! UniqueIdObjectSequenceProvider`. Rust has no inheritance, so the interfaces are restored as follows:
//!
//! | Java interface | Rust |
//! |---|---|
//! | `ChildTaskContainer` (`getChildTasks` / `addTask`) | inherent methods [`ProjectFile::get_child_tasks`] / [`ProjectFile::add_task`] (the `child_task_container` module is not declared in `src/lib.rs`, so no new trait is added, see below) |
//! | `ChildResourceContainer` (`getChildResources` / `addResource`) | inherent methods [`ProjectFile::get_child_resources`] / [`ProjectFile::add_resource`] (the `child_resource_container` module is not declared) |
//! | `UniqueIdObjectSequenceProvider` (`getUniqueIdObjectSequence`) | `impl UniqueIdObjectSequenceProvider for ProjectFile`, while keeping an inherent method of the same name (`crate::task` does not import the trait, see below) |
//!
//! # Cyclic references and wiring (core design)
//!
//! In Java, `ProjectFile`'s constructor calls `new ProjectContext(this)` (`ProjectContext`'s fields each
//! hand their own `this` to the child containers; `ProjectFile` then registers itself in `m_context.getProjects()`).
//! This kind of bidirectional strong reference must be broken apart in Rust, otherwise the `Rc` cycle would leak memory:
//!
//! ```text
//! ProjectFile --(strong Rc)--> ProjectContext
//! ProjectContext --(Weak)--> ProjectFile           // m_projects stores a weak reference
//! ProjectFile --(Weak)--> entities (Task/Resource/...)   // each entity points back to the file via a weak reference
//! child containers (TaskContainer/RelationContainer/...) --(Weak)--> ProjectFile
//! child containers (ResourceContainer/...) --(Weak)--> ProjectContext
//! ProjectFile --(Weak)--> itself (self_ref, used only by find_project and the like to retrieve the Rc handle)
//! ```
//!
//! Therefore [`ProjectFile::new`] **returns `Rc<RefCell<ProjectFile>>`** (rather than a value),
//! so that the `Rc` is constructed first and all back-reference wiring is completed afterwards. Wiring order ([`ProjectFile::new_with_context`]):
//!
//! 1. Obtain the `Rc<RefCell<ProjectContext>>` ([`ProjectFile::new`] uses
//!    [`ProjectContext::shared`], which wires the calendar container and the sequence generator adapter);
//! 2. Construct all fields with "empty weak references": `ProjectProperties` (weak reference to the parent file), `TaskContainer`,
//!    `ResourceAssignmentContainer`, `RelationContainer`, presentation-layer containers, `DataLinkContainer`,
//!    `ExternalProjectContainer`, and holds `Rc<RefCell<ProjectContext>>` directly;
//! 3. Wrap in `Rc::new(RefCell::new(..))` to obtain the file handle `file`;
//! 4. Backfill and wire (**must** happen after the `Rc` already exists):
//!    `self_ref` (weak self-reference), `properties` (`ProjectProperties::new` needs a weak reference to the parent file),
//!    `tasks.set_context(&file)`,`tasks.set_project_context(&context)`,`relations.set_context(&file)`,
//!    `external_projects.set_context(&file)`,`assignments.set_context(&context)`;
//! 5. Register this project with the context: `context.borrow_mut().add_project(&file)`;
//! 6. **Complete the wiring of resources in the context**: `context.get_resources_mut().set_context(&context)`
//!    (`ProjectContext::shared` currently wires only the calendar container and the sequence provider; `ResourceContainer`'s
//!    owner explicitly requires this step to be done by `ProjectFile::new()` in its file header). Also complete
//!    `calendars.set_context(&context)` so that callers passing a context that has not gone through `shared()`
//!    ([`ProjectContext::new`]) still work correctly (this call is idempotent).
//!
//! Note: the sequence provider adapter inside `ProjectContext` is a private field, so only
//! [`ProjectContext::shared`] can wire it; therefore [`ProjectFile::new`] always constructs the context with `shared()`,
//! while [`ProjectFile::new_with_context`] accepts a caller-provided context (if that context
//! has not gone through `shared()`, the sequence provider degrades to its own built-in counter, without panicking).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `m_baselines: Map<Integer, ProjectFile>` | `HashMap<i32, Rc<RefCell<ProjectFile>>>` | elements are shared handles; Java uses `TreeMap`, Rust uses `HashMap` (same key set, only iteration order differs) |
//! | `m_baselineTaskMap: Map<Integer, Map<Task, Task>>` | `HashMap<i32, HashMap<i32, Rc<RefCell<Task>>>>` | `Rc<RefCell<Task>>` cannot be used as a `HashMap` key, so the **task's unique ID** is used as the inner key (unique IDs are unique within a schedule, so the semantics are equivalent, and `DefaultBaselineStrategy` already fills it according to this convention) |
//! | `m_ignoredErrors: List<Exception>` | `Vec<MpxjError>` | consistent with the existing trade-off in `ProjectContext` (a unified error type) |
//! | `getViews/getTables/getFilters/getGroups` return mutable containers | return `&T` (with separate `*_mut`) | Java's getters return mutable collection objects, which Rust's `&self` getters cannot provide |
//! | `getCalendars/getResources/getShifts/...` (delegating to the context) | return `Ref<'_, T>` | the context is stored in a `RefCell`, so `&T` cannot be returned; `Ref` provides the same field access through `Deref`. **If the caller is currently holding a mutable borrow of the context** (which cannot happen in Java), `RefCell::borrow` will panic |
//! | `getProjectConfig()` returns `ProjectConfig` | returns `Ref<'_, ProjectConfig>` | same as above (`ProjectConfig` is held by the context) |
//! | `getDefaultCalendar()/setDefaultCalendar()` | same name + `Option` | delegates to [`ProjectProperties`] (in Java it also delegates to `m_properties`) |
//! | `setBaseline(ProjectFile)` / `getBaseline()` / `clearBaseline()` | [`ProjectFile::set_default_baseline`] / [`ProjectFile::get_default_baseline`] / [`ProjectFile::clear_default_baseline`] | Rust has no overloading, so the versions taking an `int` argument are named `set_baseline` / `get_baseline` / `clear_baseline` respectively |
//! | `getStartDate(Task)` / `getFinishDate(Task)` (private) | private methods `start_date` / `finish_date` | Rust naming style |
//! | `getPopulatedFields` uses the no-argument methods of the four container types | each container accepts the defaults and custom fields passed in by the parent file | avoids borrowing the parent file again through the containers while `ProjectFile` holds a borrow of the parent file |
//! | `toString()` | not provided | Java does not override `toString()` (`Debug` is not derived either: the dependent `ProjectContext` and others do not implement `Debug`) |
//!
//! # Skipped Java Methods and reasons
//!
//! | Java | Reason |
//! |---|---|
//! | `getActivityCodes()` / `getProjectCodes()` / `getResourceCodes()` / `getRoleCodes()` / `getResourceAssignmentCodes()` | the corresponding container modules are not declared in `src/lib.rs` (used by Primavera and other formats), and `ProjectContext` does not provide them either |
//! | `getExpenseCategories()` / `getCostAccounts()` / `getLocations()` / `getCurrencies()` | same as above (modules not declared) |
//! | the call to the **resource assignment** container in `fixUniqueIdClashes()` | `ResourceAssignmentContainer` does not yet provide `fix_unique_id_clashes`; the calendar container has been wired into the base-class index and clash repair |
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.
//!
//! # Known re-entrant borrow restrictions (must read)
//!
//! Entities such as `Task` / `ResourceAssignment` point back to the parent file via `Weak<RefCell<ProjectFile>>`,
//! and call `file.borrow()` / `file.borrow_mut()` when needed (see `crate::task`).
//! Therefore, for any "ProjectFile method that directly reaches these entities from `&mut self` (or `&self`)",
//! as soon as the caller enters through `file.borrow_mut()`, the entity's second borrow will panic:
//!
//! - [`ProjectFile::add_task`]: `TaskContainer::add` internally calls `Task::new_in_file(&file, ..)`,
//!   but the latter needs `file.borrow()`. The Rust side provides an equivalent **borrow-free** entry point
//!   [`ProjectFile::add_task_shared`] (an associated function that takes the file handle); prefer using it.
//! - [`ProjectFile::update_structure`]:`Task::generate_wbs` / `generate_outline_number`
//!   reads `file.get_child_tasks()` (that is, borrows the parent file again), which necessarily conflicts when entered from `&mut self`.
//!   this problem requires the `Task` side to switch to `try_borrow` degradation (or the caller to drive the container while not holding a file borrow)
//!   to be truly resolved; this is recorded here for reference only.
//! - [`ProjectFile::expand_subprojects`] / `replace_external_tasks` / `replace_relations`:
//!   `Task::expand_subproject` / `Task::add_predecessor` borrow the parent file again.
//!   the external project read callback ([`ExternalProjectContainer::set_project_reader`]) currently has no caller:
//!   Java's `readFile` uses `UniversalProjectReader`, but this crate does not have that class -- it relies on
//!   `Class.forName` reflection to iterate over candidate readers and dispatch by trial-reading with `getFileFormat()`; this is a categorical skip.
//!   the implemented [`crate::mpp::mpp_reader::MPPReader`] /
//!   [`crate::mpx::mpx_reader::MPXReader`] can be dispatched by extension and wired into that callback by the caller;
//!   when not wired, `read_file` always returns `None` (see its file header), so the external project branch is never executed,
//!   and after wiring, the borrow strategy needs to be aligned consistently.
//! - [`ProjectFile::fix_unique_id_clashes`]: containers need to obtain the sequence generator via `ProjectContext` (the relation container is already wired)
//!   or via the parent file; when neither is available, it degrades to "skip" (containers use `try_borrow*` internally and will not panic).
//!
//! These restrictions match the file-header notes of `crate::task_container` / `crate::relation_container`
//! regarding "re-entrant borrow", and must be handled uniformly during integration.
//!
//! # Model-layer interfaces used (all implemented; listed here only as an index)
//!
//! ```text
//! TableContainer::new() / FilterContainer::new() / GroupContainer::new() / ViewContainer::new()
//! ```
//! (Corresponds to Java's no-argument constructors; the presentation-layer containers are all implemented.)
//!
//! Also: [`ProjectFile::get_tasks`] returns `&TaskContainer`, while
//! `crate::default_baseline_strategy` iterates with `for task in project.get_tasks()`,
//! so this file adds an `IntoIterator` implementation for `&TaskContainer` (see the end of the file),
//! semantically equivalent to the Java-side `TaskContainer` being iterable itself.

use std::any::TypeId;
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use chrono::NaiveDateTime;

use crate::calendar_type::CalendarType;
use crate::common::number_helper::NumberHelper;
use crate::common::object_sequence::ObjectSequence;
use crate::custom_field_container::CustomFieldContainer;
use crate::data_link_container::DataLinkContainer;
use crate::event_manager::EventManager;
use crate::common::populated_fields::Defaults;
use crate::field_key::FieldKey;
use crate::external_project_container::ExternalProjectContainer;
use crate::filter_container::FilterContainer;
use crate::group_container::GroupContainer;
use crate::mpxj_exception::MpxjError;
use crate::notes_topic_container::NotesTopicContainer;
use crate::project_calendar::ProjectCalendar;
use crate::project_calendar_container::ProjectCalendarContainer;
use crate::project_config::ProjectConfig;
use crate::project_context::ProjectContext;
use crate::project_properties::ProjectProperties;
use crate::relation::Builder as RelationBuilder;
use crate::relation_container::RelationContainer;
use crate::resource::Resource;
use crate::resource_assignment_container::ResourceAssignmentContainer;
use crate::resource_container::ResourceContainer;
use crate::shift_container::ShiftContainer;
use crate::shift_period_container::ShiftPeriodContainer;
use crate::table_container::TableContainer;
use crate::task::Task;
use crate::task_container::TaskContainer;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;
use crate::unit_of_measure_container::UnitOfMeasureContainer;
use crate::user_defined_field_container::UserDefinedFieldContainer;
use crate::view_container::ViewContainer;
use crate::work_contour_container::WorkContourContainer;

/// A project schedule.
///
/// Corresponds to Java: `public final class ProjectFile implements ChildTaskContainer,
/// ChildResourceContainer, UniqueIdObjectSequenceProvider`
pub struct ProjectFile {
    /// Project properties.
    ///
    /// Corresponds to Java field: `private final ProjectProperties m_properties = new ProjectProperties(this);`
    properties: ProjectProperties,

    /// Task container.
    ///
    /// Corresponds to Java field: `private final TaskContainer m_tasks = new TaskContainer(this);`
    tasks: TaskContainer,

    /// Resource assignment container.
    ///
    /// Corresponds to Java field: `private final ResourceAssignmentContainer m_assignments = new ResourceAssignmentContainer(this);`
    assignments: ResourceAssignmentContainer,

    /// Relation container.
    ///
    /// Corresponds to Java field: `private final RelationContainer m_relations = new RelationContainer(this);`
    relations: RelationContainer,

    /// Table container.
    ///
    /// Corresponds to Java field: `private final TableContainer m_tables = new TableContainer();`
    tables: TableContainer,

    /// Filter container.
    ///
    /// Corresponds to Java field: `private final FilterContainer m_filters = new FilterContainer();`
    filters: FilterContainer,

    /// Group container.
    ///
    /// Corresponds to Java field: `private final GroupContainer m_groups = new GroupContainer();`
    groups: GroupContainer,

    /// View container.
    ///
    /// Corresponds to Java field: `private final ViewContainer m_views = new ViewContainer();`
    views: ViewContainer,

    /// Data link container.
    ///
    /// Corresponds to Java field: `private final DataLinkContainer m_dataLinks = new DataLinkContainer();`
    data_links: DataLinkContainer,

    /// External (sub)project container.
    ///
    /// Corresponds to Java field: `private final ExternalProjectContainer m_externalProjects = new ExternalProjectContainer(this);`
    external_projects: ExternalProjectContainer,

    /// Baseline projects, indexed by sequence number (0 is the primary baseline, 1..=10 are Baseline1..Baseline10).
    ///
    /// Corresponds to Java field: `private final Map<Integer, ProjectFile> m_baselines = new TreeMap<>();`
    baselines: HashMap<i32, Rc<RefCell<ProjectFile>>>,

    /// Current project's task unique ID -> baseline project task (used by `Task.getBaselineTask()`).
    ///
    /// Corresponds to Java field: `private final Map<Integer, Map<Task, Task>> m_baselineTaskMap = new HashMap<>();`
    /// (the inner key uses the task unique ID instead, see the file header)
    baseline_task_map: HashMap<i32, HashMap<i32, Rc<RefCell<Task>>>>,

    /// Errors ignored while reading this project.
    ///
    /// Corresponds to Java field: `private final List<Exception> m_ignoredErrors = new ArrayList<>();`
    ignored_errors: Vec<MpxjError>,

    /// Project context (strong reference; the context stores only a weak reference to this file, so there is no cycle).
    ///
    /// Corresponds to Java field: `private final ProjectContext m_context;`
    context: Rc<RefCell<ProjectContext>>,

    /// Self handle (weak reference, Rust-specific): used by [`ProjectFile::get_self`] /
    /// [`ProjectFile::find_project`] to retrieve the `Rc` handle from `&self`.
    self_ref: Weak<RefCell<ProjectFile>>,
}

impl ProjectFile {
    /// Constructor.
    ///
    /// Corresponds to Java: `public ProjectFile()`
    ///
    /// The Rust side returns `Rc<RefCell<ProjectFile>>` (see the file header for cyclic references and wiring).
    pub fn new() -> Rc<RefCell<ProjectFile>> {
        // Java: this(new ProjectContext());
        ProjectFile::new_with_context(ProjectContext::shared())
    }

    /// Constructor (accepts a project context).
    ///
    /// Corresponds to Java: `public ProjectFile(ProjectContext context)`
    ///
    /// See the file header for the full wiring order.
    pub fn new_with_context(context: Rc<RefCell<ProjectContext>>) -> Rc<RefCell<ProjectFile>> {
        // Step 2: construct all fields with empty weak references (the `Rc` does not exist yet).
        let project = ProjectFile {
            properties: ProjectProperties::new(Weak::new()),
            tasks: TaskContainer::new(),
            assignments: ResourceAssignmentContainer::new(),
            relations: RelationContainer::new(),
            tables: TableContainer::new(),
            filters: FilterContainer::new(),
            groups: GroupContainer::new(),
            views: ViewContainer::new(),
            data_links: DataLinkContainer::new(),
            external_projects: ExternalProjectContainer::new(),
            baselines: HashMap::new(),
            baseline_task_map: HashMap::new(),
            ignored_errors: Vec::new(),
            context: Rc::clone(&context),
            self_ref: Weak::new(),
        };

        // Step 3: obtain the file handle.
        let file = Rc::new(RefCell::new(project));

        // Step 4: backfill and wire (must be after the `Rc` exists).
        {
            let mut file_ref = file.borrow_mut();
            file_ref.self_ref = Rc::downgrade(&file);
            file_ref.properties = ProjectProperties::new(Rc::downgrade(&file));
            file_ref.tasks.set_context(&file);
            file_ref.tasks.set_project_context(&context);
            file_ref.relations.set_context(&file);
            // Rust-specific: `RelationContainer::set_context` reads the context internally via `try_borrow`,
            // but at this moment `file` is mutably borrowed (the `file_ref` above), so the read necessarily fails; the relation container's
            // automatic unique ID needs `ProjectContext`, so the wiring is completed explicitly here.
            file_ref.relations.set_project_context(&context);
            file_ref.external_projects.set_context(&file);
            file_ref.assignments.set_context(&context);
        }

        // Step 5/6: register with the context and complete the back-reference wiring of the context's containers (idempotent).
        {
            let mut context_ref = context.borrow_mut();
            context_ref.add_project(&file);
            context_ref.get_calendars_mut().set_context(&context);
            context_ref.get_resources_mut().set_context(&context);
        }

        file
    }

    /// Get the self handle (Rust-specific: Java uses the implicit `this`).
    ///
    /// Returns `None` when the file has been dropped (which does not happen on normal call paths).
    pub fn get_self(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.self_ref.upgrade()
    }

    /// Get the parent project context.
    ///
    /// Corresponds to Java: `public ProjectContext getProjectContext()`
    pub fn get_project_context(&self) -> Rc<RefCell<ProjectContext>> {
        Rc::clone(&self.context)
    }

    /// Get the project configuration.
    ///
    /// Corresponds to Java: `public ProjectConfig getProjectConfig()`
    ///
    /// The Rust side returns `Ref` (the configuration is held by [`ProjectContext`], see the differences table in the file header).
    pub fn get_project_config(&self) -> Ref<'_, ProjectConfig> {
        // Java: return m_context.getProjectConfig();
        Ref::map(self.context.borrow(), |context| context.get_project_config())
    }

    /// Create a task and add it to the file (also registering it as a top-level project task).
    ///
    /// Corresponds to Java: `@Override public Task addTask()`
    ///
    /// Consistent with Java, it can be called while holding a `ProjectFile` borrow -- the configuration and sequence number needed by the container to create the task
    /// come from `ProjectContext`, so the parent file is not called back (see [`crate::TaskContainer::add`]).
    pub fn add_task(&mut self) -> Rc<RefCell<Task>> {
        // Java: return m_tasks.add();
        self.tasks.add()
    }

    /// Create a task and add it to the file (Rust-specific: the entry point when only the file handle is held).
    ///
    /// Corresponds to Java: `@Override public Task addTask()`
    ///
    /// Behaves the same as [`ProjectFile::add_task`]; the only difference is that it takes `Rc<RefCell<ProjectFile>>`
    /// instead of `&mut ProjectFile`, making it convenient in scenarios where only a shared handle is available.
    pub fn add_task_shared(file: &Rc<RefCell<ProjectFile>>) -> Rc<RefCell<Task>> {
        // Java: Task task = new Task(m_projectFile, null);
        let task = Rc::new(RefCell::new(Task::new_in_file(file, None)));

        // Java: the two steps of TaskContainer.add(): add to the container + register as a top-level project task
        {
            let mut file_ref = file.borrow_mut();
            file_ref.tasks.add_task(Rc::clone(&task));
            file_ref
                .tasks
                .get_child_tasks_mut()
                .push(Rc::clone(&task));
        }

        task
    }

    /// Remove a task from the project.
    ///
    /// Corresponds to Java: `public void removeTask(Task task)`
    pub fn remove_task(&mut self, task: &Rc<RefCell<Task>>) {
        // Java: m_tasks.remove(task);
        self.tasks.remove_task(task);
    }

    /// Get all top-level tasks (a snapshot).
    ///
    /// Corresponds to Java: `@Override public List<Task> getChildTasks()`
    pub fn get_child_tasks(&self) -> Vec<Rc<RefCell<Task>>> {
        // Java: return m_tasks.getChildTasks();
        self.tasks.get_child_tasks().to_vec()
    }

    /// Get all top-level resources (a snapshot).
    ///
    /// Corresponds to Java: `@Override public List<Resource> getChildResources()`
    pub fn get_child_resources(&self) -> Vec<Rc<RefCell<Resource>>> {
        // Java: return m_context.getResources().getChildResources();
        self.context.borrow().get_resources().get_child_resources()
    }

    /// Get the task container.
    ///
    /// Corresponds to Java: `public TaskContainer getTasks()`
    pub fn get_tasks(&self) -> &TaskContainer {
        &self.tasks
    }

    /// Get a mutable reference to the task container (Rust-specific: Java's getter returns a mutable container).
    pub fn get_tasks_mut(&mut self) -> &mut TaskContainer {
        &mut self.tasks
    }

    /// Create a calendar.
    ///
    /// Corresponds to Java: `public ProjectCalendar addCalendar()`
    pub fn add_calendar(&mut self) -> Rc<RefCell<ProjectCalendar>> {
        // Java: return m_context.getCalendars().add();
        self.context.borrow_mut().add_calendar()
    }

    /// Remove a calendar (returns whether it was found; Java returns `void`).
    ///
    /// Corresponds to Java: `public void removeCalendar(ProjectCalendar calendar)`
    pub fn remove_calendar(&mut self, calendar: &Rc<RefCell<ProjectCalendar>>) -> bool {
        // Java: m_context.getCalendars().remove(calendar);
        let context = Rc::clone(&self.context);
        let mut context_ref = context.borrow_mut();
        let Ok(calendar_ref) = calendar.try_borrow() else {
            // Java has no such branch; on the Rust side, pointer identity comparison is impossible while the calendar is mutably borrowed
            return false;
        };
        context_ref.get_calendars_mut().remove(&calendar_ref)
    }

    /// Create a base calendar named "Standard" and fill in the default working week and default working hours.
    ///
    /// Corresponds to Java: `public ProjectCalendar addDefaultBaseCalendar()`
    pub fn add_default_base_calendar(&mut self) -> Rc<RefCell<ProjectCalendar>> {
        // Java: return m_context.getCalendars().addDefaultBaseCalendar();
        self.context.borrow_mut().add_default_base_calendar()
    }

    /// Create a default derived calendar.
    ///
    /// Corresponds to Java: `public ProjectCalendar addDefaultDerivedCalendar()`
    pub fn add_default_derived_calendar(&mut self) -> Rc<RefCell<ProjectCalendar>> {
        // Java: return m_context.getCalendars().addDefaultDerivedCalendar();
        self.context.borrow_mut().add_default_derived_calendar()
    }

    /// Get the calendar container.
    ///
    /// Corresponds to Java: `public ProjectCalendarContainer getCalendars()`
    pub fn get_calendars(&self) -> Ref<'_, ProjectCalendarContainer> {
        // Java: return m_context.getCalendars();
        Ref::map(self.context.borrow(), |context| context.get_calendars())
    }

    /// Get all global calendars, together with the project-level calendars related to this project.
    ///
    /// Corresponds to Java: `public List<ProjectCalendar> getCalendarsForProject()`
    pub fn get_calendars_for_project(&self) -> Vec<Rc<RefCell<ProjectCalendar>>> {
        // Java:
        // m_context.getCalendars().stream()
        //    .filter(c -> c.getType() != CalendarType.PROJECT
        //                 || c.getProjectUniqueID().equals(m_properties.getUniqueID()))
        //    .collect(Collectors.toList());
        let context = self.context.borrow();
        let unique_id = self.properties.get_unique_id();
        context
            .get_calendars()
            .iter()
            .filter(|calendar| match calendar.try_borrow() {
                Ok(calendar) => {
                    calendar.get_type() != CalendarType::Project
                        || calendar.get_project_unique_id() == unique_id
                }
                // Java has no such branch (the element is necessarily readable); the Rust side avoids a borrow conflict
                Err(_) => false,
            })
            .cloned()
            .collect()
    }

    /// Get the project properties.
    ///
    /// Corresponds to Java: `public ProjectProperties getProjectProperties()`
    pub fn get_project_properties(&self) -> &ProjectProperties {
        &self.properties
    }

    /// Get a mutable reference to the project properties (Rust-specific: Java's getter returns a mutable object).
    pub fn get_project_properties_mut(&mut self) -> &mut ProjectProperties {
        &mut self.properties
    }

    /// Create a resource.
    ///
    /// Corresponds to Java: `@Override public Resource addResource()`
    pub fn add_resource(&mut self) -> Rc<RefCell<Resource>> {
        // Java: return m_context.getResources().add();
        self.context.borrow_mut().get_resources_mut().add()
    }

    /// Remove a resource (returns whether it was found; Java returns `void`).
    ///
    /// Corresponds to Java: `public void removeResource(Resource resource)`
    pub fn remove_resource(&mut self, resource: &Rc<RefCell<Resource>>) -> bool {
        // Java: m_context.getResources().remove(resource);
        self.context
            .borrow_mut()
            .get_resources_mut()
            .remove_resource(resource)
    }

    /// Get the resource container.
    ///
    /// Corresponds to Java: `public ResourceContainer getResources()`
    pub fn get_resources(&self) -> Ref<'_, ResourceContainer> {
        // Java: return m_context.getResources();
        Ref::map(self.context.borrow(), |context| context.get_resources())
    }

    /// Get the resource assignment container.
    ///
    /// Corresponds to Java: `public ResourceAssignmentContainer getResourceAssignments()`
    pub fn get_resource_assignments(&self) -> &ResourceAssignmentContainer {
        &self.assignments
    }

    /// Get a mutable reference to the resource assignment container (Rust-specific).
    pub fn get_resource_assignments_mut(&mut self) -> &mut ResourceAssignmentContainer {
        &mut self.assignments
    }

    /// Get the relation container.
    ///
    /// Corresponds to Java: `public RelationContainer getRelations()`
    pub fn get_relations(&self) -> &RelationContainer {
        &self.relations
    }

    /// Get a mutable reference to the relation container (Rust-specific).
    pub fn get_relations_mut(&mut self) -> &mut RelationContainer {
        &mut self.relations
    }

    /// Get a calendar by name; returns `None` when not found.
    ///
    /// Corresponds to Java: `public ProjectCalendar getCalendarByName(String calendarName)`
    pub fn get_calendar_by_name(&self, calendar_name: Option<&str>) -> Option<Rc<RefCell<ProjectCalendar>>> {
        // Java: return m_context.getCalendars().getByName(calendarName);
        self.context
            .borrow()
            .get_calendars()
            .get_by_name(calendar_name)
    }

    /// Get a calendar by unique ID; returns `None` when not found.
    ///
    /// Corresponds to Java: `public ProjectCalendar getCalendarByUniqueID(Integer calendarID)`
    pub fn get_calendar_by_unique_id(
        &self,
        calendar_id: Option<i32>,
    ) -> Option<Rc<RefCell<ProjectCalendar>>> {
        // Java: return m_context.getCalendars().getByUniqueID(calendarID);
        self.context
            .borrow()
            .get_calendars()
            .get_by_unique_id(calendar_id)
    }

    /// Get a task by ID; returns `None` when not found.
    ///
    /// Corresponds to Java: `public Task getTaskByID(Integer id)`
    pub fn get_task_by_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Task>>> {
        // Java: return m_tasks.getByID(id);
        self.tasks.get_by_id(id)
    }

    /// Get a task by unique ID; returns `None` when not found.
    ///
    /// Corresponds to Java: `public Task getTaskByUniqueID(Integer id)`
    pub fn get_task_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Task>>> {
        // Java: return m_tasks.getByUniqueID(id);
        self.tasks.get_by_unique_id(id)
    }

    /// Get a resource by ID; returns `None` when not found.
    ///
    /// Corresponds to Java: `public Resource getResourceByID(Integer id)`
    pub fn get_resource_by_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Resource>>> {
        // Java: return m_context.getResources().getByID(id);
        self.context.borrow().get_resources().get_by_id(id)
    }

    /// Get a resource by unique ID; returns `None` when not found.
    ///
    /// Corresponds to Java: `public Resource getResourceByUniqueID(Integer id)`
    pub fn get_resource_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Resource>>> {
        // Java: return m_context.getResources().getByUniqueID(id);
        self.context.borrow().get_resources().get_by_unique_id(id)
    }

    /// Rebuild the task / resource hierarchy of the project file from scratch.
    ///
    /// Corresponds to Java: `public void updateStructure()`
    pub fn update_structure(&mut self) {
        // Java: m_tasks.updateStructure(); m_context.getResources().updateStructure();
        //
        // Rust-side re-entrant borrow restriction: Java's `TaskContainer.updateStructure()` internally calls back into
        // `m_projectFile` to read `ProjectConfig`'s automatic numbering flags. `ProjectConfig` is stored in
        // `ProjectContext`'s `RefCell` (which does not conflict with the `ProjectFile` borrow held by the caller),
        // so the three flags are read out first and passed to the container as arguments; the container thus no longer calls back into the parent file,
        // and there is no need to temporarily move the task container out of `ProjectFile`.
        let (auto_wbs, auto_outline_number, auto_outline_level) = {
            let config = self.get_project_config();
            (
                config.get_auto_wbs(),
                config.get_auto_outline_number(),
                config.get_auto_outline_level(),
            )
        };

        self.tasks
            .update_structure(auto_wbs, auto_outline_number, auto_outline_level);

        self.context
            .borrow_mut()
            .get_resources_mut()
            .update_structure();
    }

    /// Get the earliest task start date.
    ///
    /// Corresponds to Java: `public LocalDateTime getEarliestStartDate()`
    pub fn get_earliest_start_date(&self) -> Option<NaiveDateTime> {
        // Java: m_tasks.stream().map(this::getStartDate).filter(Objects::nonNull)
        //       .min(Comparator.naturalOrder()).orElse(null);
        self.tasks
            .iter()
            .filter_map(|task| self.start_date(task))
            .min()
    }

    /// Calculate the project's actual start date.
    ///
    /// Corresponds to Java: `LocalDateTime getActualStart()`
    pub(crate) fn get_actual_start(&self) -> Option<NaiveDateTime> {
        // Java: m_tasks.stream().map(Task::getActualStart).filter(Objects::nonNull)
        //       .min(Comparator.naturalOrder()).orElse(null);
        self.tasks
            .iter()
            .filter_map(|task| read_task(task, |task| task.get_actual_start()).flatten())
            .min()
    }

    /// Calculate the project's actual finish date (returns `None` when any task is missing an actual finish time).
    ///
    /// Corresponds to Java: `LocalDateTime getActualFinish()`
    pub(crate) fn get_actual_finish(&self) -> Option<NaiveDateTime> {
        // Java:
        // if (m_tasks.stream().map(Task::getActualFinish).anyMatch(Objects::isNull)) { return null; }
        // return m_tasks.stream().map(Task::getActualFinish).filter(Objects::nonNull)
        //       .max(Comparator.naturalOrder()).orElse(null);
        let mut result: Option<NaiveDateTime> = None;
        for task in self.tasks.iter() {
            match read_task(task, |task| task.get_actual_finish()).flatten() {
                Some(value) => result = Some(match result {
                    Some(current) => current.max(value),
                    None => value,
                }),
                None => return None,
            }
        }
        result
    }

    /// Get the latest task finish date.
    ///
    /// Corresponds to Java: `public LocalDateTime getLatestFinishDate()`
    pub fn get_latest_finish_date(&self) -> Option<NaiveDateTime> {
        // Java: m_tasks.stream().map(this::getFinishDate).filter(Objects::nonNull)
        //       .max(Comparator.naturalOrder()).orElse(null);
        self.tasks
            .iter()
            .filter_map(|task| self.finish_date(task))
            .max()
    }

    /// Get the date a task uses for the "project start date".
    ///
    /// Corresponds to Java: `private LocalDateTime getStartDate(Task task)`
    fn start_date(&self, task: &Rc<RefCell<Task>>) -> Option<NaiveDateTime> {
        // Java: if (NumberHelper.getInt(task.getUniqueID()) == 0) { return null; }
        if NumberHelper::get_int(
            read_task(task, |task| task.get_unique_id()).flatten().map(f64::from),
        ) == 0
        {
            return None;
        }

        // Java: for a milestone, use the actual finish time, falling back to the planned finish time; for a non-milestone, use
        //       the actual start time, falling back to the planned start time.
        let milestone = read_task(task, |task| task.get_milestone()).unwrap_or(false);
        if milestone {
            read_task(task, |task| task.get_actual_finish())
                .flatten()
                .or_else(|| read_task(task, |task| task.get_finish()).flatten())
        } else {
            read_task(task, |task| task.get_actual_start())
                .flatten()
                .or_else(|| read_task(task, |task| task.get_start()).flatten())
        }
    }

    /// Get the date a task uses for the "project finish date".
    ///
    /// Corresponds to Java: `private LocalDateTime getFinishDate(Task task)`
    fn finish_date(&self, task: &Rc<RefCell<Task>>) -> Option<NaiveDateTime> {
        // Java: if (NumberHelper.getInt(task.getUniqueID()) == 0) { return null; }
        if NumberHelper::get_int(
            read_task(task, |task| task.get_unique_id()).flatten().map(f64::from),
        ) == 0
        {
            return None;
        }

        // Java: taskFinishDate = task.getActualFinish();
        //       if (taskFinishDate == null) { taskFinishDate = task.getFinish(); }
        read_task(task, |task| task.get_actual_finish())
            .flatten()
            .or_else(|| read_task(task, |task| task.get_finish()).flatten())
    }

    /// Get the view container.
    ///
    /// Corresponds to Java: `public ViewContainer getViews()`
    pub fn get_views(&self) -> &ViewContainer {
        &self.views
    }

    /// Get a mutable reference to the view container (Rust-specific).
    pub fn get_views_mut(&mut self) -> &mut ViewContainer {
        &mut self.views
    }

    /// Get the table container.
    ///
    /// Corresponds to Java: `public TableContainer getTables()`
    pub fn get_tables(&self) -> &TableContainer {
        &self.tables
    }

    /// Get a mutable reference to the table container (Rust-specific).
    pub fn get_tables_mut(&mut self) -> &mut TableContainer {
        &mut self.tables
    }

    /// Get the filter container.
    ///
    /// Corresponds to Java: `public FilterContainer getFilters()`
    pub fn get_filters(&self) -> &FilterContainer {
        &self.filters
    }

    /// Get a mutable reference to the filter container (Rust-specific).
    pub fn get_filters_mut(&mut self) -> &mut FilterContainer {
        &mut self.filters
    }

    /// Get the group container.
    ///
    /// Corresponds to Java: `public GroupContainer getGroups()`
    pub fn get_groups(&self) -> &GroupContainer {
        &self.groups
    }

    /// Get a mutable reference to the group container (Rust-specific).
    pub fn get_groups_mut(&mut self) -> &mut GroupContainer {
        &mut self.groups
    }

    /// Get the event manager.
    ///
    /// Corresponds to Java: `public EventManager getEventManager()`
    pub fn get_event_manager(&self) -> Ref<'_, EventManager> {
        // Java: return m_context.getEventManager();
        Ref::map(self.context.borrow(), |context| context.get_event_manager())
    }

    /// Get the custom field container.
    ///
    /// Corresponds to Java: `public CustomFieldContainer getCustomFields()`
    ///
    /// [`CustomFieldContainer`] is held by the context as `Rc<RefCell<..>>`, so the shared handle is returned directly.
    pub fn get_custom_fields(&self) -> Rc<RefCell<CustomFieldContainer>> {
        // Java: return m_context.getCustomFields();
        self.context.borrow().get_custom_fields()
    }

    /// Get the shift container.
    ///
    /// Corresponds to Java: `public ShiftContainer getShifts()`
    pub fn get_shifts(&self) -> Ref<'_, ShiftContainer> {
        // Java: return m_context.getShifts();
        Ref::map(self.context.borrow(), |context| context.get_shifts())
    }

    /// Get the shift period container.
    ///
    /// Corresponds to Java: `public ShiftPeriodContainer getShiftPeriods()`
    pub fn get_shift_periods(&self) -> Ref<'_, ShiftPeriodContainer> {
        // Java: return m_context.getShiftPeriods();
        Ref::map(self.context.borrow(), |context| context.get_shift_periods())
    }

    /// Get the data link container.
    ///
    /// Corresponds to Java: `public DataLinkContainer getDataLinks()`
    pub fn get_data_links(&self) -> &DataLinkContainer {
        &self.data_links
    }

    /// Get a mutable reference to the data link container (Rust-specific).
    pub fn get_data_links_mut(&mut self) -> &mut DataLinkContainer {
        &mut self.data_links
    }

    /// Get the user-defined field container.
    ///
    /// Corresponds to Java: `public UserDefinedFieldContainer getUserDefinedFields()`
    pub fn get_user_defined_fields(&self) -> Ref<'_, UserDefinedFieldContainer> {
        // Java: return m_context.getUserDefinedFields();
        Ref::map(self.context.borrow(), |context| {
            context.get_user_defined_fields()
        })
    }

    /// Return the fields that have at least one non-default cached value among tasks, resources and assignments.
    ///
    /// Corresponds to Java: the `UserDefinedFieldMap` constructor merges the
    /// `getPopulatedFields()` of the three containers in turn; this method collects the handles while holding the parent file borrow and scans them uniformly.
    pub fn get_populated_entity_fields(&self) -> std::collections::HashSet<FieldKey> {
        let defaults = Defaults::from_project_properties(&self.properties);
        let (custom_tasks, custom_resources, custom_assignments) = {
            let Ok(context) = self.context.try_borrow() else {
                return Default::default();
            };
            let custom = context.get_user_defined_fields();
            (
                custom.get_task_fields().iter().map(FieldKey::from).collect::<Vec<_>>(),
                custom.get_resource_fields().iter().map(FieldKey::from).collect::<Vec<_>>(),
                custom.get_assignment_fields().iter().map(FieldKey::from).collect::<Vec<_>>(),
            )
        };
        let mut result = self.tasks.get_populated_fields(custom_tasks, defaults);
        if let Ok(context) = self.context.try_borrow() {
            result.extend(context.get_resources().get_populated_fields(custom_resources));
        }
        result.extend(self.assignments.get_populated_fields(custom_assignments, defaults));
        result
    }

    /// Return the fields that have at least one non-default cached value among the project's four entity types.
    ///
    /// Corresponds to Java: `public Set<FieldType> getPopulatedFields()`.
    pub fn get_populated_fields(&self) -> std::collections::HashSet<FieldKey> {
        let mut result = self.get_populated_entity_fields();
        let custom = self.context.try_borrow().map(|context| {
            context.get_user_defined_fields().get_project_fields().iter()
                .map(FieldKey::from).collect::<Vec<_>>()
        }).unwrap_or_default();
        result.extend(self.properties.get_populated_fields(custom));
        result
    }

    /// Get the work contour container.
    ///
    /// Corresponds to Java: `public WorkContourContainer getWorkContours()`
    pub fn get_work_contours(&self) -> Ref<'_, WorkContourContainer> {
        // Java: return m_context.getWorkContours();
        Ref::map(self.context.borrow(), |context| context.get_work_contours())
    }

    /// Get the notes topic container.
    ///
    /// Corresponds to Java: `public NotesTopicContainer getNotesTopics()`
    pub fn get_notes_topics(&self) -> Ref<'_, NotesTopicContainer> {
        // Java: return m_context.getNotesTopics();
        Ref::map(self.context.borrow(), |context| context.get_notes_topics())
    }

    /// Get the unit of measure container.
    ///
    /// Corresponds to Java: `public UnitOfMeasureContainer getUnitsOfMeasure()`
    pub fn get_units_of_measure(&self) -> Ref<'_, UnitOfMeasureContainer> {
        // Java: return m_context.getUnitsOfMeasure();
        Ref::map(self.context.borrow(), |context| context.get_units_of_measure())
    }

    /// Get the default calendar: look it up by the calendar name in the project properties; if not found, take the project's first calendar,
    /// and if the project has no calendars, create a new default base calendar.
    ///
    /// Corresponds to Java: `public ProjectCalendar getDefaultCalendar()`
    pub fn get_default_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        // Java: return getProjectProperties().getDefaultCalendar();
        self.properties.get_default_calendar()
    }

    /// Set the default calendar (passing `None` does not modify it, equivalent to Java's `null` check).
    ///
    /// Corresponds to Java: `public void setDefaultCalendar(ProjectCalendar calendar)`
    pub fn set_default_calendar(&mut self, calendar: Option<Rc<RefCell<ProjectCalendar>>>) {
        // Java: if (calendar != null) { m_properties.setDefaultCalendar(calendar); }
        if let Some(calendar) = calendar {
            let id = calendar.try_borrow().ok().and_then(|value| value.get_unique_id());
            self.properties.set_default_calendar(Some(calendar));
            if let Ok(mut context) = self.context.try_borrow_mut() {
                context.get_calendars_mut().set_default_calendar_unique_id(id);
            }
        }
    }

    /// Get the internal calendar used for timephased baseline calculation: first find the calendar matching the baseline calendar name, falling back to the default calendar when not found.
    ///
    /// Corresponds to Java: `public ProjectCalendar getBaselineCalendar()`
    pub fn get_baseline_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        // Java:
        // ProjectCalendar result = getCalendarByName(m_properties.getBaselineCalendarName());
        // if (result == null) { result = getDefaultCalendar(); }
        // return result;
        let name = self.properties.get_baseline_calendar_name();
        self.get_calendar_by_name(name.as_deref())
            .or_else(|| self.get_default_calendar())
    }

    /// Get this project's baseline projects (keyed by baseline sequence number).
    ///
    /// Corresponds to Java: `public Map<Integer, ProjectFile> getBaselines()`
    pub fn get_baselines(&self) -> &HashMap<i32, Rc<RefCell<ProjectFile>>> {
        &self.baselines
    }

    /// Store the given project as the **default** baseline and use it to populate the baseline properties of this project's tasks.
    ///
    /// Corresponds to Java: `public void setBaseline(ProjectFile baseline)`
    pub fn set_default_baseline(&mut self, baseline: &Rc<RefCell<ProjectFile>>) {
        // Java: setBaseline(baseline, 0);
        self.set_baseline(baseline, 0);
    }

    /// Get the default baseline project.
    ///
    /// Corresponds to Java: `public ProjectFile getBaseline()`
    pub fn get_default_baseline(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        // Java: return getBaseline(0);
        self.get_baseline(0)
    }

    /// Store the given project as baselineN and use it to populate the baseline properties of this project's tasks.
    ///
    /// Corresponds to Java: `public void setBaseline(ProjectFile baseline, int index)`
    ///
    /// When `index` is `0`, populate the default baseline; `1..=10` populate Baseline1..Baseline10;
    /// other indices are only stored and not interpreted (consistent with Java).
    pub fn set_baseline(&mut self, baseline: &Rc<RefCell<ProjectFile>>, index: i32) {
        // Java: m_baselines.put(Integer.valueOf(index), baseline);
        self.baselines.insert(index, Rc::clone(baseline));

        if (0..=10).contains(&index) {
            // Java:
            // if (index == 0) { m_properties.setBaselineDate(baseline.getProjectProperties().getCreationDate()); }
            // else { m_properties.setBaselineDate(index, baseline.getProjectProperties().getCreationDate()); }
            let creation_date = baseline
                .borrow()
                .get_project_properties()
                .get_creation_date();
            if index == 0 {
                self.properties.set_baseline_date(creation_date);
            } else {
                self.properties
                    .set_baseline_date_for_index(index, creation_date);
            }

            // Java: getProjectConfig().getBaselineStrategy().populateBaseline(this, baseline, index);
            //
            // The Rust side first clones the context handle so that the strategy reference does not borrow `self`, allowing `self` to be passed below
            // as `&mut ProjectFile` to the strategy (see the "re-entrant borrow" notes in the file header).
            let context = Rc::clone(&self.context);
            let context_ref = context.borrow();
            let config = context_ref.get_project_config();
            let Some(strategy) = config.get_baseline_strategy() else {
                return;
            };
            let baseline_ref = baseline.borrow();
            strategy.populate_baseline(self, &baseline_ref, index);
        }
    }

    /// Get baselineN (`0` means the primary baseline, `1..=10` mean Baseline1..Baseline10).
    ///
    /// Corresponds to Java: `public ProjectFile getBaseline(int index)`
    pub fn get_baseline(&self, index: i32) -> Option<Rc<RefCell<ProjectFile>>> {
        // Java: return m_baselines.get(Integer.valueOf(index));
        self.baselines.get(&index).cloned()
    }

    /// Clear the default baseline.
    ///
    /// Corresponds to Java: `public void clearBaseline()`
    pub fn clear_default_baseline(&mut self) {
        // Java: clearBaseline(0);
        self.clear_baseline(0);
    }

    /// Clear the baselineN properties (does not remove it from `m_baselines`, consistent with Java).
    ///
    /// Corresponds to Java: `public void clearBaseline(int index)`
    pub fn clear_baseline(&mut self, index: i32) {
        // Java: getProjectConfig().getBaselineStrategy().clearBaseline(this, index);
        let context = Rc::clone(&self.context);
        let context_ref = context.borrow();
        let config = context_ref.get_project_config();
        let Some(strategy) = config.get_baseline_strategy() else {
            return;
        };
        strategy.clear_baseline(self, index);
    }

    /// Set the mapping from "this project's tasks -> baseline tasks" (populated by the strategy when a baseline is added).
    ///
    /// Corresponds to Java: `void setBaselineTaskMap(int index, Map<Task, Task> map)`
    ///
    /// The inner key is the current project task's unique ID (see the file header).
    pub fn set_baseline_task_map(
        &mut self,
        index: i32,
        map: HashMap<i32, Rc<RefCell<Task>>>,
    ) {
        // Java: m_baselineTaskMap.put(Integer.valueOf(index), map);
        self.baseline_task_map.insert(index, map);
    }

    /// Get the mapping "this project's task unique ID -> baseline task"; returns an empty map when not set.
    ///
    /// Corresponds to Java: `Map<Task, Task> getBaselineTaskMap(int index)`
    pub fn get_baseline_task_map(&self, index: i32) -> HashMap<i32, Rc<RefCell<Task>>> {
        // Java: return m_baselineTaskMap.getOrDefault(Integer.valueOf(index), Collections.emptyMap());
        self.baseline_task_map.get(&index).cloned().unwrap_or_default()
    }

    /// Recursively expand all subprojects in this file and its subprojects (attaching the subproject tasks under the original "subproject task").
    ///
    /// Corresponds to Java: `public void expandSubprojects(boolean replaceExternalTasks)`
    ///
    /// When `replace_external_tasks` is `true`, it also rewrites the
    /// predecessor / successor relations pointing to external tasks so that they point to the original tasks, and removes external task placeholders that are no longer needed.
    ///
    /// Note the re-entrant borrow restriction (see the file header).
    pub fn expand_subprojects(&mut self, replace_external_tasks: bool) {
        // Java: getTasks().stream().map(Task::expandSubproject).filter(Objects::nonNull)
        //       .forEach(p -> p.expandSubprojects(replaceExternalTasks));
        let tasks = self.tasks.as_slice().to_vec();
        let mut expanded_projects: Vec<Rc<RefCell<ProjectFile>>> = Vec::new();

        for task in tasks {
            // Java's Task.expandSubproject() returns null for a non-external task; here it is equivalently skipped.
            let subproject_file = match task.try_borrow() {
                Ok(task_ref) => {
                    if !task_ref.get_external_task() && !task_ref.get_external_project() {
                        continue;
                    }
                    task_ref.get_subproject_file()
                }
                // Java has no such branch; the Rust side avoids a borrow conflict
                Err(_) => continue,
            };

            // Java: Task.expandSubproject() reads the external project internally via getSubprojectObject()
            // (the result is cached by ExternalProjectContainer). When it cannot be read, it is equivalent to Java returning null.
            let Some(project) = self.external_projects.read(subproject_file.as_deref()) else {
                continue;
            };

            // Java: m_children.addAll(children); return subproject;
            if let Ok(mut task_ref) = task.try_borrow_mut() {
                if task_ref.expand_subproject().is_some() {
                    expanded_projects.push(project);
                }
            }
        }

        for project in expanded_projects {
            // Java: p.expandSubprojects(replaceExternalTasks);
            if let Ok(mut project_ref) = project.try_borrow_mut() {
                project_ref.expand_subprojects(replace_external_tasks);
            }
        }

        if replace_external_tasks {
            // Java: replaceExternalTasks();
            self.replace_external_tasks();
        }
    }

    /// Rewrite the (predecessor / successor) relations pointing to external tasks so that they point to the correct cross-project tasks, and delete the
    /// external task placeholders that are no longer needed.
    ///
    /// Corresponds to Java: `private void replaceExternalTasks()`
    fn replace_external_tasks(&mut self) {
        // Java:
        // List<Task> externalTasks = new ArrayList<>();
        // findExternalTasks(getChildTasks(), externalTasks);
        let mut external_tasks: Vec<Rc<RefCell<Task>>> = Vec::new();
        self.find_external_tasks(&self.get_child_tasks(), &mut external_tasks);

        // Java:
        // Set<Task> replacedTasks = externalTasks.stream().map(this::replaceRelations)
        //    .filter(Objects::nonNull).collect(Collectors.toSet());
        let mut replaced_tasks: Vec<Rc<RefCell<Task>>> = Vec::new();
        for external_task in external_tasks {
            if let Some(replaced) = self.replace_external_task_relations(&external_task) {
                if !contains_task(&replaced_tasks, &replaced) {
                    replaced_tasks.push(replaced);
                }
            }
        }

        // Java: removeExternalTasks(getChildTasks(), replacedTasks);
        self.remove_external_tasks(self.get_child_tasks(), &replaced_tasks);
    }

    /// Rewrite the predecessor / successor relations of an external task; returns that external task on success, otherwise `None`.
    ///
    /// Corresponds to Java: `private Task replaceRelations(Task externalTask)`
    fn replace_external_task_relations(
        &mut self,
        external_task: &Rc<RefCell<Task>>,
    ) -> Option<Rc<RefCell<Task>>> {
        // Java: ProjectFile originalProjectFile = findProject(externalTask.getSubprojectFile());
        let subproject_file = read_task(external_task, |task| task.get_subproject_file()).flatten();
        let original_project_file = self.find_project(subproject_file.as_deref())?;

        // Java: Task originalTask = findTask(originalProjectFile, externalTask);
        let original_task = self.find_task(&original_project_file, external_task)?;

        // Java: replaceRelations(externalTask, originalTask);
        self.replace_relations(external_task, &original_task);

        // Java: return externalTask;
        Some(Rc::clone(external_task))
    }

    /// Find the related [`ProjectFile`] by project file name; returns `None` when not found.
    ///
    /// Corresponds to Java: `private ProjectFile findProject(String name)`
    fn find_project(&mut self, name: Option<&str>) -> Option<Rc<RefCell<ProjectFile>>> {
        // Java:
        // if (name.equals(m_properties.getProjectFilePath())) { return this; }
        // return m_externalProjects.read(name);
        if let Some(name) = name {
            if Some(name) == self.properties.get_project_file_path().as_deref() {
                return self.get_self();
            }
        }

        self.external_projects.read(name)
    }

    /// Find the original task represented by an external task in the specified project.
    ///
    /// Corresponds to Java: `private Task findTask(ProjectFile file, Task externalTask)`
    fn find_task(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        external_task: &Rc<RefCell<Task>>,
    ) -> Option<Rc<RefCell<Task>>> {
        // Java:
        // Integer id = externalTask.getSubprojectTaskUniqueID();
        // if (id != null) { Task result = file.getTaskByUniqueID(id); if (result != null) { return result; } }
        // id = externalTask.getSubprojectTaskID();
        // if (id != null) { return file.getTaskByID(id); }
        // return null;
        let (subproject_task_unique_id, subproject_task_id) = read_task(external_task, |task| {
            (task.get_subproject_task_unique_id(), task.get_subproject_task_id())
        })
        .unwrap_or((None, None));

        if let Some(unique_id) = subproject_task_unique_id {
            if let Some(result) = file.borrow().get_task_by_unique_id(Some(unique_id)) {
                return Some(result);
            }
        }

        if let Some(id) = subproject_task_id {
            return file.borrow().get_task_by_id(Some(id));
        }

        None
    }

    /// Rewrite the predecessor / successor relations pointing to external tasks so that they point to the original tasks.
    ///
    /// Corresponds to Java: `private void replaceRelations(Task externalTask, Task originalTask)`
    ///
    /// Java uses `externalTask.getParentFile().getRelations()`; the Rust side uses this file's
    /// relation container directly (an external task is necessarily a child task of this file, see the file header); the result is equivalent.
    fn replace_relations(
        &mut self,
        external_task: &Rc<RefCell<Task>>,
        original_task: &Rc<RefCell<Task>>,
    ) {
        // Java: to avoid concurrent modification, copy the relation list first
        let successors = match external_task.try_borrow() {
            Ok(task_ref) => self.relations.get_successors(&task_ref),
            Err(_) => return,
        };
        let predecessors = match external_task.try_borrow() {
            Ok(task_ref) => self.relations.get_predecessors(&task_ref),
            Err(_) => return,
        };

        for original_relation in successors {
            // Java: relations.remove(originalRelation);
            let removed = match original_relation.try_borrow() {
                Ok(relation) => self.relations.remove(&relation),
                Err(_) => false,
            };
            if !removed {
                continue;
            }

            // Java:
            // originalRelation.getSuccessorTask().addPredecessor(
            //    new Relation.Builder().from(originalRelation).predecessorTask(originalTask));
            let successor = original_relation
                .try_borrow()
                .ok()
                .and_then(|relation| relation.get_successor_task());
            let Some(successor) = successor else {
                continue;
            };

            let builder = match original_relation.try_borrow() {
                Ok(relation) => {
                    RelationBuilder::from(&relation).predecessor_task(original_task)
                }
                Err(_) => continue,
            };
            Task::add_predecessor(&successor, builder);
        }

        for original_relation in predecessors {
            // Java: relations.remove(originalRelation);
            let removed = match original_relation.try_borrow() {
                Ok(relation) => self.relations.remove(&relation),
                Err(_) => false,
            };
            if !removed {
                continue;
            }

            // Java: originalTask.addPredecessor(new Relation.Builder().from(originalRelation));
            let builder = match original_relation.try_borrow() {
                Ok(relation) => RelationBuilder::from(&relation),
                Err(_) => continue,
            };
            Task::add_predecessor(original_task, builder);
        }
    }

    /// Recursively collect all external tasks in the task hierarchy.
    ///
    /// Corresponds to Java: `private void findExternalTasks(List<Task> tasks, List<Task> externalTasks)`
    fn find_external_tasks(
        &self,
        tasks: &[Rc<RefCell<Task>>],
        external_tasks: &mut Vec<Rc<RefCell<Task>>>,
    ) {
        // Java: externalTasks.addAll(tasks.stream().filter(Task::getExternalTask).collect(...));
        for task in tasks {
            if read_task(task, |task| task.get_external_task()).unwrap_or(false) {
                external_tasks.push(Rc::clone(task));
            }
        }

        // Java: tasks.forEach(t -> findExternalTasks(t.getChildTasks(), externalTasks));
        for task in tasks {
            let children = read_task(task, |task| task.get_child_tasks()).unwrap_or_default();
            self.find_external_tasks(&children, external_tasks);
        }
    }

    /// Recursively delete external tasks that are no longer needed.
    ///
    /// Corresponds to Java: `private void removeExternalTasks(List<Task> tasks, Set<Task> replacedTasks)`
    ///
    /// Java uses `tasks.removeIf(replacedTasks::contains)` to modify the **live list** directly;
    /// the Rust side has no "live list" handle, so removal is done according to the task's parent list: top-level tasks are removed from
    /// the `TaskContainer`'s top-level task list, and child tasks are removed with [`Task::remove_child_task`].
    fn remove_external_tasks(
        &mut self,
        tasks: Vec<Rc<RefCell<Task>>>,
        replaced_tasks: &[Rc<RefCell<Task>>],
    ) {
        for task in tasks {
            if contains_task(replaced_tasks, &task) {
                let parent_task =
                    read_task(&task, |task| task.get_parent_task()).flatten();
                match parent_task {
                    Some(parent_task) => Task::remove_child_task(&parent_task, &task),
                    None => self
                        .tasks
                        .get_child_tasks_mut()
                        .retain(|child| !Rc::ptr_eq(child, &task)),
                }
                continue;
            }

            let children = read_task(&task, |task| task.get_child_tasks()).unwrap_or_default();
            self.remove_external_tasks(children, replaced_tasks);
        }
    }

    /// Called by the reader after finishing reading a schedule.
    ///
    /// Corresponds to Java: `public void readComplete()`
    pub fn read_complete(&mut self) {
        // Java: fixUniqueIdClashes();
        self.fix_unique_id_clashes();
    }

    /// Renumber entities that have duplicate unique IDs.
    ///
    /// Corresponds to Java: `public void fixUniqueIdClashes()`
    ///
    /// Java also processes the resource assignment container; that Rust container does not yet provide clash repair.
    pub fn fix_unique_id_clashes(&mut self) {
        // Java: getTasks().fixUniqueIdClashes();
        self.tasks.fix_unique_id_clashes();

        // Java: getResources().fixUniqueIdClashes();
        self.context
            .borrow_mut()
            .get_resources_mut()
            .fix_unique_id_clashes();

        // Java: getCalendars().fixUniqueIdClashes();
        let calendar_sequence = self.context.borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>());
        self.context.borrow_mut().get_calendars_mut()
            .fix_unique_id_clashes(calendar_sequence);

        // Java: getResourceAssignments().fixUniqueIdClashes();
        // ResourceAssignmentContainer does not provide this method (same as above).

        // Java: getRelations().fixUniqueIdClashes();
        self.relations.fix_unique_id_clashes();
    }

    /// Get the sequence generator used to generate unique IDs for the specified type.
    ///
    /// Corresponds to Java: `@Override public ObjectSequence getUniqueIdObjectSequence(Class<?> c)`
    ///
    /// (a trait implementation of `UniqueIdObjectSequenceProvider` is also provided, see
    /// `impl UniqueIdObjectSequenceProvider for ProjectFile`.)
    pub fn get_unique_id_object_sequence(&mut self, type_id: TypeId) -> Rc<ObjectSequence> {
        // Java: return m_context.getUniqueIdObjectSequence(c);
        self.context
            .borrow_mut()
            .get_unique_id_object_sequence(type_id)
    }

    /// Record an error ignored while reading this project.
    ///
    /// Corresponds to Java: `public void addIgnoredError(Exception ex)`
    pub fn add_ignored_error(&mut self, error: MpxjError) {
        // Java: m_ignoredErrors.add(ex);
        self.ignored_errors.push(error);
    }

    /// Get the list of errors ignored while reading this project.
    ///
    /// Corresponds to Java: `public List<Exception> getIgnoredErrors()`
    pub fn get_ignored_errors(&self) -> &[MpxjError] {
        // Java: return m_ignoredErrors;
        &self.ignored_errors
    }

    /// Register an existing external (sub)project file.
    ///
    /// Corresponds to Java: `void addExternalProject(String fileName, ProjectFile projectFile)`
    ///
    /// On the Rust side both parameters may be `None` (a `None` file name or a `None` project is not registered),
    /// consistent with the nullable semantics of the caller (`crate::task`).
    pub(crate) fn add_external_project(
        &mut self,
        file_name: Option<&str>,
        project_file: Option<Rc<RefCell<ProjectFile>>>,
    ) {
        // Java: m_externalProjects.add(fileName, projectFile);
        let Some(file_name) = file_name else {
            return;
        };
        self.external_projects.add(file_name, project_file);
    }

    /// Read the specified external (sub)project file (the result is cached).
    ///
    /// Corresponds to Java: `ProjectFile readExternalProject(String fileName)`
    pub(crate) fn read_external_project(
        &mut self,
        file_name: Option<&str>,
    ) -> Option<Rc<RefCell<ProjectFile>>> {
        // Java: return m_externalProjects.read(fileName);
        self.external_projects.read(file_name)
    }
}

impl UniqueIdObjectSequenceProvider for ProjectFile {
    /// Get (creating if necessary) the sequence generator for the specified type (forwarded to [`ProjectContext`]).
    ///
    /// Corresponds to Java: `@Override public ObjectSequence getUniqueIdObjectSequence(Class<?> c)`
    fn get_unique_id_object_sequence(&mut self, type_id: TypeId) -> Rc<ObjectSequence> {
        ProjectFile::get_unique_id_object_sequence(self, type_id)
    }
}

/// Read a task field with `try_borrow`; returns `None` when already borrowed (without panicking).
fn read_task<T>(task: &Rc<RefCell<Task>>, read: impl FnOnce(&Task) -> T) -> Option<T> {
    task.try_borrow().ok().map(|task| read(&task))
}

/// Determine whether the task list contains the specified task (`Rc` pointer identity, equivalent to Java's reference equality).
fn contains_task(tasks: &[Rc<RefCell<Task>>], task: &Rc<RefCell<Task>>) -> bool {
    tasks.iter().any(|candidate| Rc::ptr_eq(candidate, task))
}

impl<'a> IntoIterator for &'a TaskContainer {
    /// Iterate over the tasks in the task container.
    ///
    /// Rust-specific (see the file header): `crate::default_baseline_strategy` is written as
    /// `for task in project.get_tasks()`, while [`ProjectFile::get_tasks`] returns
    /// `&TaskContainer`, so an `IntoIterator` is added here for the reference (on the Java side `TaskContainer`
    /// is itself iterable).
    type Item = &'a Rc<RefCell<Task>>;
    type IntoIter = std::slice::Iter<'a, Rc<RefCell<Task>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// Note: Java's `m_context.getXxx()` returns mutable container objects; on the Rust side the containers in the context live behind
// a `RefCell`, so `&mut T` cannot be returned. To access a **mutable** container in the context, go through
// `ProjectFile::get_project_context` to obtain the context handle, for example:
// `file.borrow().get_project_context().borrow_mut().get_calendars_mut()`.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assignment_field::AssignmentField;
    use crate::data_link::DataLink;
    use crate::field_container::FieldContainer;
    use crate::field_key::FieldKey;
    use crate::field_value::FieldValue;
    use crate::project_entity_with_id::ProjectEntityWithID;
    use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
    use crate::resource_assignment::ResourceAssignment;
    use crate::resource_field::ResourceField;
    use crate::task_field::TaskField;

    /// Create a project file.
    fn project() -> Rc<RefCell<ProjectFile>> {
        ProjectFile::new()
    }

    /// Create a task in the specified file (using the borrow-free entry point).
    fn add_task(file: &Rc<RefCell<ProjectFile>>) -> Rc<RefCell<Task>> {
        ProjectFile::add_task_shared(file)
    }

    // -----------------------------------------------------------------------
    // Root container assembly
    // -----------------------------------------------------------------------

    #[test]
    fn new_registers_project_in_context_and_wires_resources() {
        let file = project();
        let context = file.borrow().get_project_context();

        // Java: m_context.getProjects().add(this) in the constructor
        let projects = context.borrow().get_projects();
        assert_eq!(projects.len(), 1);
        assert!(Rc::ptr_eq(&projects[0], &file));

        // ProjectContext::shared wires only the calendar container; the resource container is completed by ProjectFile::new
        assert!(context.borrow().get_resources().get_context().is_some());

        // The resource assignment container holds a weak reference to the context (wired during construction)
        assert!(file
            .borrow()
            .get_resource_assignments()
            .get_context()
            .is_some());
    }

    /// When creating a task, read the real project configuration and use the context's shared sequence numbers.
    #[test]
    fn new_tasks_share_the_context_unique_id_sequence() {
        let file = project();
        let first = add_task(&file);
        let second = add_task(&file);
        assert_eq!(first.borrow().get_unique_id(), Some(1));
        assert_eq!(second.borrow().get_unique_id(), Some(2));
        let context = file.borrow().get_project_context();
        let sequence = context.borrow_mut().get_unique_id_object_sequence(TypeId::of::<Task>());
        assert_eq!(sequence.get_next(), 3);
    }

    /// Field scanning collects only non-default cached values and covers the four field containers.
    #[test]
    fn populated_fields_distinguish_defaults_across_containers() {
        let file = project();
        let task = add_task(&file);
        task.borrow_mut().set(FieldKey::Task(TaskField::Text1), Some(FieldValue::String(String::new())));
        task.borrow_mut().set(FieldKey::Task(TaskField::Text2), Some(FieldValue::String("内容".into())));
        task.borrow_mut().set(FieldKey::Task(TaskField::Flag1), Some(FieldValue::Boolean(false)));
        let context = file.borrow().get_project_context();
        let resource = context.borrow_mut().get_resources_mut().add();
        resource.borrow_mut().set(FieldKey::Resource(ResourceField::Number1), Some(FieldValue::Double(0.0)));
        resource.borrow_mut().set(FieldKey::Resource(ResourceField::Text1), Some(FieldValue::String("资源".into())));
        let assignment = Rc::new(RefCell::new(ResourceAssignment::new()));
        assignment.borrow_mut().set(FieldKey::Assignment(AssignmentField::Flag1), Some(FieldValue::Boolean(true)));
        file.borrow_mut().get_resource_assignments_mut().add(assignment);

        let fields = file.borrow().get_populated_fields();
        assert!(fields.contains(&FieldKey::Task(TaskField::Text2)));
        assert!(fields.contains(&FieldKey::Resource(ResourceField::Text1)));
        assert!(fields.contains(&FieldKey::Assignment(AssignmentField::Flag1)));
        assert!(!fields.contains(&FieldKey::Task(TaskField::Text1)));
        assert!(!fields.contains(&FieldKey::Task(TaskField::Flag1)));
        assert!(!fields.contains(&FieldKey::Resource(ResourceField::Number1)));
    }

    #[test]
    fn new_with_context_reuses_the_supplied_context() {
        let context = ProjectContext::shared();
        let first = ProjectFile::new_with_context(Rc::clone(&context));
        let second = ProjectFile::new_with_context(Rc::clone(&context));

        assert_eq!(context.borrow().get_projects().len(), 2);

        // The two projects share the same context (consistent with Java)
        assert!(Rc::ptr_eq(
            &first.borrow().get_project_context(),
            &second.borrow().get_project_context()
        ));

        // Shared sequence generator
        let next = first
            .borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<Task>())
            .get_next();
        let next_again = second
            .borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<Task>())
            .get_next();
        assert_eq!(next, 1);
        assert_eq!(next_again, 2);
    }

    #[test]
    fn containers_start_empty_and_are_reachable() {
        let file = project();
        let file_ref = file.borrow();

        assert!(file_ref.get_tasks().is_empty());
        assert!(file_ref.get_child_tasks().is_empty());
        assert!(file_ref.get_child_resources().is_empty());
        assert!(file_ref.get_resource_assignments().is_empty());
        assert!(file_ref.get_relations().is_empty());
        assert!(file_ref.get_calendars().is_empty());
        assert!(file_ref.get_resources().is_empty());
        assert!(file_ref.get_tables().is_empty());
        // FilterContainer does not provide `len` / `is_empty` (its author provides access by name / ID lookup)
        let _ = file_ref.get_filters();
        assert!(file_ref.get_groups().is_empty());
        assert!(file_ref.get_views().is_empty());
        assert!(file_ref.get_data_links().is_empty());
        assert!(file_ref.get_shifts().is_empty());
        assert!(file_ref.get_shift_periods().is_empty());
        assert!(file_ref.get_units_of_measure().is_empty());
        assert!(file_ref.get_work_contours().is_empty());
        assert!(file_ref.get_notes_topics().is_empty());
        assert!(file_ref.get_user_defined_fields().is_empty());
        assert!(file_ref.get_baselines().is_empty());
        assert!(file_ref.get_ignored_errors().is_empty());

        // The project configuration comes from the context (`Ref` provides the same access through `Deref`)
        assert!(file_ref.get_project_config().get_auto_task_id());

        // The event manager is available (`Ref<EventManager>`, from the context)
        let _ = file_ref.get_event_manager();
    }

    #[test]
    fn data_links_are_owned_by_the_file() {
        let file = project();

        {
            let mut file_ref = file.borrow_mut();
            file_ref
                .get_data_links_mut()
                .add(Rc::new(RefCell::new(DataLink::new("1"))));
        }

        let file_ref = file.borrow();
        assert_eq!(file_ref.get_data_links().len(), 1);
        assert_eq!(
            file_ref.get_data_links().get(0).map(|link| link.borrow().get_id().to_owned()),
            Some("1".to_owned())
        );
    }

    #[test]
    fn ignored_errors_are_collected() {
        let file = project();

        {
            let mut file_ref = file.borrow_mut();
            file_ref.add_ignored_error(MpxjError::new("first"));
            file_ref.add_ignored_error(MpxjError::new("second"));
        }

        let file_ref = file.borrow();
        assert_eq!(file_ref.get_ignored_errors().len(), 2);
        assert_eq!(
            file_ref.get_ignored_errors().first().map(MpxjError::message),
            Some("first")
        );
    }

    // -----------------------------------------------------------------------
    // Add / remove tasks / resources / assignments
    // -----------------------------------------------------------------------

    #[test]
    fn add_and_remove_task() {
        let file = project();

        let task = add_task(&file);
        assert_eq!(file.borrow().get_tasks().len(), 1);
        assert_eq!(file.borrow().get_child_tasks().len(), 1);

        // Java: automatically assign unique ID and ID (enabled by default in ProjectConfig)
        let unique_id = task.borrow().get_unique_id();
        let id = task.borrow().get_id();
        assert!(unique_id.is_some());
        assert!(id.is_some());

        // Lookup by ID / unique ID
        assert!(Rc::ptr_eq(
            &file.borrow().get_task_by_unique_id(unique_id).expect("registered"),
            &task
        ));

        // The ID index (Java's `m_idMap`) is populated by the reader via `mapID`; `addTask()` itself does not register the ID
        file.borrow_mut()
            .get_tasks_mut()
            .map_id(id.expect("ID assigned"), Rc::clone(&task));
        assert!(Rc::ptr_eq(
            &file.borrow().get_task_by_id(id).expect("registered"),
            &task
        ));

        file.borrow_mut().remove_task(&task);
        assert!(file.borrow().get_tasks().is_empty());
        assert!(file.borrow().get_child_tasks().is_empty());
    }

    #[test]
    fn add_and_remove_resource() {
        let file = project();

        let resource = file.borrow_mut().add_resource();
        assert_eq!(file.borrow().get_resources().len(), 1);
        assert_eq!(file.borrow().get_child_resources().len(), 1);

        let unique_id = resource.borrow().get_unique_id();
        let id = resource.borrow().get_id();
        assert!(unique_id.is_some());
        assert!(id.is_some());

        assert!(Rc::ptr_eq(
            &file
                .borrow()
                .get_resource_by_unique_id(unique_id)
                .expect("registered"),
            &resource
        ));
        assert!(Rc::ptr_eq(
            &file.borrow().get_resource_by_id(id).expect("registered"),
            &resource
        ));

        assert!(file.borrow_mut().remove_resource(&resource));
        assert!(file.borrow().get_resources().is_empty());
        assert!(file.borrow().get_child_resources().is_empty());

        // A repeated removal returns false
        assert!(!file.borrow_mut().remove_resource(&resource));
    }

    #[test]
    fn add_and_remove_assignment() {
        let file = project();

        let task = add_task(&file);
        let resource = file.borrow_mut().add_resource();

        let assignment = Rc::new(RefCell::new(ResourceAssignment::with_parent_file(&file)));
        {
            let mut assignment_ref = assignment.borrow_mut();
            assignment_ref.set_task_unique_id(task.borrow().get_unique_id());
            assignment_ref.set_resource_unique_id(resource.borrow().get_unique_id());
        }

        file.borrow_mut()
            .get_resource_assignments_mut()
            .add(Rc::clone(&assignment));
        assert_eq!(file.borrow().get_resource_assignments().len(), 1);

        // Indexed by task / resource unique ID
        assert_eq!(
            file.borrow()
                .get_resource_assignments()
                .get_by_task_unique_id(task.borrow().get_unique_id())
                .len(),
            1
        );

        let unique_id = assignment.borrow().get_unique_id();
        assert!(Rc::ptr_eq(
            &file
                .borrow()
                .get_resource_assignments()
                .get_by_unique_id(unique_id)
                .expect("registered"),
            &assignment
        ));

        assert!(file
            .borrow_mut()
            .get_resource_assignments_mut()
            .remove(&assignment));
        assert!(file.borrow().get_resource_assignments().is_empty());
    }

    #[test]
    fn project_dates_are_calculated_from_tasks() {
        let file = project();

        let first = add_task(&file);
        let second = add_task(&file);

        let start = NaiveDateTime::parse_from_str("2024-01-01 08:00", "%Y-%m-%d %H:%M").ok();
        let finish = NaiveDateTime::parse_from_str("2024-01-05 17:00", "%Y-%m-%d %H:%M").ok();
        first.borrow_mut().set_start(start);
        first.borrow_mut().set_finish(finish);
        second.borrow_mut().set_start(NaiveDateTime::parse_from_str(
            "2023-12-01 08:00",
            "%Y-%m-%d %H:%M",
        )
        .ok());

        let file_ref = file.borrow();
        assert_eq!(
            file_ref.get_earliest_start_date(),
            NaiveDateTime::parse_from_str("2023-12-01 08:00", "%Y-%m-%d %H:%M").ok()
        );
        assert_eq!(file_ref.get_latest_finish_date(), finish);

        // When the actual time is missing, get_actual_start / get_actual_finish behave the same as Java
        assert_eq!(file_ref.get_actual_start(), None);
        first
            .borrow_mut()
            .set_actual_start(NaiveDateTime::parse_from_str(
                "2024-01-01 08:00",
                "%Y-%m-%d %H:%M",
            )
            .ok());
        assert!(file_ref.get_actual_start().is_some());

        // When any task is missing actualFinish, the whole thing returns None
        assert_eq!(file_ref.get_actual_finish(), None);
        second
            .borrow_mut()
            .set_actual_finish(NaiveDateTime::parse_from_str(
                "2024-01-03 17:00",
                "%Y-%m-%d %H:%M",
            )
            .ok());
        first
            .borrow_mut()
            .set_actual_finish(NaiveDateTime::parse_from_str(
                "2024-01-05 17:00",
                "%Y-%m-%d %H:%M",
            )
            .ok());
        assert_eq!(
            file_ref.get_actual_finish(),
            NaiveDateTime::parse_from_str("2024-01-05 17:00", "%Y-%m-%d %H:%M").ok()
        );
    }

    // -----------------------------------------------------------------------
    // Calendar resolution
    // -----------------------------------------------------------------------

    #[test]
    fn default_base_calendar_is_registered_and_resolvable() {
        let file = project();

        let calendar = file.borrow_mut().add_default_base_calendar();
        assert_eq!(
            calendar.borrow().get_name(),
            Some(crate::project_calendar::DEFAULT_BASE_CALENDAR_NAME)
        );

        let file_ref = file.borrow();
        assert_eq!(file_ref.get_calendars().len(), 1);

        // Resolve by name / unique ID
        let by_name = file_ref
            .get_calendar_by_name(Some(crate::project_calendar::DEFAULT_BASE_CALENDAR_NAME))
            .expect("findable by name");
        assert!(Rc::ptr_eq(&by_name, &calendar));

        // `addDefaultBaseCalendar` itself does not assign a unique ID (in Java it is assigned by `findOrCreateDefaultCalendar`
        // or by the reader); here it is set explicitly to verify resolution by unique ID
        drop(file_ref);
        calendar.borrow_mut().set_unique_id(Some(7));
        let file_ref = file.borrow();

        let by_id = file_ref
            .get_calendar_by_unique_id(Some(7))
            .expect("findable by unique ID");
        assert!(Rc::ptr_eq(&by_id, &calendar));

        assert!(file_ref.get_calendar_by_name(Some("Missing")).is_none());
        assert!(file_ref.get_calendar_by_unique_id(Some(999)).is_none());
    }

    #[test]
    fn default_calendar_and_baseline_calendar_resolution() {
        let file = project();

        let calendar = file.borrow_mut().add_default_base_calendar();
        file.borrow_mut()
            .set_default_calendar(Some(Rc::clone(&calendar)));

        let file_ref = file.borrow();
        assert!(Rc::ptr_eq(
            &file_ref.get_default_calendar().expect("default calendar set"),
            &calendar
        ));

        // Baseline calendar: fall back to the default calendar when no baseline calendar name is recorded in ProjectProperties
        assert!(Rc::ptr_eq(
            &file_ref.get_baseline_calendar().expect("falls back to default calendar"),
            &calendar
        ));
    }

    #[test]
    fn calendars_for_project_filters_project_type_calendars() {
        let file = project();
        let project_unique_id = file.borrow().get_project_properties().get_unique_id();

        let global = file.borrow_mut().add_calendar();
        {
            let mut global_ref = global.borrow_mut();
            global_ref.set_type(Some(CalendarType::Global));
            global_ref.set_name(Some("Global".to_owned()));
        }

        let mine = file.borrow_mut().add_calendar();
        {
            let mut mine_ref = mine.borrow_mut();
            mine_ref.set_type(Some(CalendarType::Project));
            mine_ref.set_project_unique_id(project_unique_id);
            mine_ref.set_name(Some("Mine".to_owned()));
        }

        let other = file.borrow_mut().add_calendar();
        {
            let mut other_ref = other.borrow_mut();
            other_ref.set_type(Some(CalendarType::Project));
            other_ref.set_project_unique_id(Some(999));
            other_ref.set_name(Some("Other".to_owned()));
        }

        let calendars = file.borrow().get_calendars_for_project();
        // All global calendars are kept; project-level calendars keep only those belonging to this project
        assert_eq!(calendars.len(), 2);
        assert!(calendars.iter().any(|calendar| Rc::ptr_eq(calendar, &global)));
        assert!(calendars.iter().any(|calendar| Rc::ptr_eq(calendar, &mine)));
        assert!(!calendars.iter().any(|calendar| Rc::ptr_eq(calendar, &other)));
    }

    #[test]
    fn calendar_can_be_removed() {
        let file = project();
        let calendar = file.borrow_mut().add_default_base_calendar();

        assert!(file.borrow_mut().remove_calendar(&calendar));
        assert!(file.borrow().get_calendars().is_empty());
        assert!(!file.borrow_mut().remove_calendar(&calendar));
    }

    // -----------------------------------------------------------------------
    // Baseline
    // -----------------------------------------------------------------------

    /// Set up a pair of files whose "current project tasks <-> baseline project tasks" can be matched (same GUID).
    fn project_with_baseline() -> (
        Rc<RefCell<ProjectFile>>,
        Rc<RefCell<ProjectFile>>,
        Rc<RefCell<Task>>,
        Rc<RefCell<Task>>,
    ) {
        let file = project();
        let current_task = add_task(&file);

        let baseline = project();
        let baseline_task = add_task(&baseline);

        // DefaultBaselineStrategy uses the task's GUID as the match key; both sides must match
        let guid = uuid::Uuid::parse_str("11111111-1111-1111-1111-111111111111").ok();
        current_task.borrow_mut().set_guid(guid);
        baseline_task.borrow_mut().set_guid(guid);

        (file, baseline, current_task, baseline_task)
    }

    #[test]
    fn set_baseline_stores_project_and_populates_task_map() {
        let (file, baseline, current_task, baseline_task) = project_with_baseline();

        let task_guid = current_task.borrow().get_guid();
        assert!(task_guid.is_some(), "task should be assigned a GUID automatically");

        file.borrow_mut().set_default_baseline(&baseline);

        // Java: m_baselines.put(0, baseline)
        assert!(Rc::ptr_eq(
            &file.borrow().get_default_baseline().expect("default baseline set"),
            &baseline
        ));
        assert!(Rc::ptr_eq(
            &file.borrow().get_baseline(0).expect("baseline 0 set"),
            &baseline
        ));
        assert!(file.borrow().get_baseline(5).is_none());

        // Java: populateBaseline populates baselineTaskMap (the Rust-side inner key is the task unique ID)
        let map = file.borrow().get_baseline_task_map(0);
        let current_unique_id = current_task.borrow().get_unique_id().expect("has unique ID");
        let mapped = map.get(&current_unique_id).expect("baseline task mapped");
        assert!(Rc::ptr_eq(mapped, &baseline_task));

        // An unset sequence number returns an empty map
        assert!(file.borrow().get_baseline_task_map(3).is_empty());
    }

    #[test]
    fn baseline_attributes_are_copied_from_the_baseline_project() {
        let (file, baseline, current_task, baseline_task) = project_with_baseline();

        let cost = Some(123.0);
        baseline_task.borrow_mut().set_cost(cost);

        file.borrow_mut().set_default_baseline(&baseline);

        // Java: populateBaseline writes the baseline's Cost into the current task's BaselineCost
        assert_eq!(current_task.borrow().get_baseline_cost(), cost);
        assert_eq!(
            current_task
                .borrow()
                .get_cached_value(FieldKey::Task(TaskField::BaselineCost))
                .cloned(),
            Some(crate::field_value::FieldValue::Double(123.0))
        );

        // Java: the baseline date is the baseline project's creation date
        let creation_date = baseline
            .borrow()
            .get_project_properties()
            .get_creation_date();
        assert_eq!(
            file.borrow().get_project_properties().get_baseline_date(),
            creation_date
        );
    }

    #[test]
    fn numbered_baseline_is_stored_and_cleared() {
        let (file, baseline, current_task, baseline_task) = project_with_baseline();
        baseline_task.borrow_mut().set_cost(Some(50.0));

        file.borrow_mut().set_baseline(&baseline, 3);
        assert!(Rc::ptr_eq(
            &file.borrow().get_baseline(3).expect("baseline 3 set"),
            &baseline
        ));
        // Sequence number 3 populates Baseline3Cost (not the default baseline's BaselineCost)
        assert_eq!(
            current_task
                .borrow()
                .get_cached_value(FieldKey::Task(TaskField::Baseline3Cost))
                .cloned(),
            Some(crate::field_value::FieldValue::Double(50.0))
        );
        assert_eq!(current_task.borrow().get_baseline_cost(), None);

        // Java: clearBaseline(index) clears the corresponding fields through the strategy
        file.borrow_mut().clear_baseline(3);
        assert_eq!(
            current_task
                .borrow()
                .get_cached_value(FieldKey::Task(TaskField::Baseline3Cost))
                .cloned(),
            None
        );

        // An out-of-bounds sequence number is only stored and not interpreted
        let other = project();
        file.borrow_mut().set_baseline(&other, 11);
        assert!(Rc::ptr_eq(
            &file.borrow().get_baseline(11).expect("saved"),
            &other
        ));
        assert!(current_task.borrow().get_baseline_cost().is_none());
    }

    #[test]
    fn clear_default_baseline_clears_attributes_but_keeps_the_map() {
        let (file, baseline, current_task, baseline_task) = project_with_baseline();
        baseline_task.borrow_mut().set_cost(Some(75.0));
        file.borrow_mut().set_default_baseline(&baseline);
        assert_eq!(current_task.borrow().get_baseline_cost(), Some(75.0));

        file.borrow_mut().clear_default_baseline();
        assert_eq!(current_task.borrow().get_baseline_cost(), None);
        // Java: clearBaseline does not remove the baseline from m_baselines
        assert!(file.borrow().get_default_baseline().is_some());
    }

    // -----------------------------------------------------------------------
    // Structure rebuild / unique ID / external project
    // -----------------------------------------------------------------------

    #[test]
    fn read_complete_runs_unique_id_clash_repair() {
        let file = project();

        let root = add_task(&file);
        let child = add_task(&file);
        root.borrow_mut().set_id(Some(1));
        child.borrow_mut().set_id(Some(2));

        // Java: readComplete() → fixUniqueIdClashes()
        // (Rust-side containers use try_borrow and degrade to skipping on a borrow conflict, without panicking)
        file.borrow_mut().read_complete();

        assert_eq!(file.borrow().get_tasks().len(), 2);
        assert!(root.borrow().get_unique_id().is_some());
        assert!(child.borrow().get_unique_id().is_some());
    }

    #[test]
    fn update_structure_is_a_noop_for_a_single_task() {
        let file = project();
        let only = add_task(&file);
        only.borrow_mut().set_outline_level(Some(0));

        // Java: TaskContainer.updateStructure() returns immediately when the task count is <= 1
        file.borrow_mut().update_structure();

        assert_eq!(file.borrow().get_tasks().len(), 1);
        // `addTask()` has already registered the task as a top-level task (Java's `m_childTasks`), so the early return does not clear it
        assert_eq!(file.borrow().get_child_tasks().len(), 1);
    }

    #[test]
    fn external_projects_round_trip() {
        let file = project();
        let subproject = project();

        {
            let mut file_ref = file.borrow_mut();
            file_ref.add_external_project(Some("X:\\projects\\child.mpp"), Some(Rc::clone(&subproject)));
        }

        let found = file.borrow_mut().read_external_project(Some("X:\\projects\\child.mpp"));
        assert!(found
            .map(|found| Rc::ptr_eq(&found, &subproject))
            .unwrap_or(false));

        // An unregistered file name returns None (no read callback is wired)
        assert!(file
            .borrow_mut()
            .read_external_project(Some("X:\\projects\\other.mpp"))
            .is_none());
    }

    #[test]
    fn expand_subprojects_without_external_tasks_is_a_noop() {
        let file = project();
        add_task(&file);
        add_task(&file);

        file.borrow_mut().expand_subprojects(true);

        assert_eq!(file.borrow().get_tasks().len(), 2);
        assert_eq!(file.borrow().get_child_tasks().len(), 2);
    }

    #[test]
    fn relationships_can_be_created_and_removed() {
        let file = project();
        let first = add_task(&file);
        let second = add_task(&file);

        // Task::add_predecessor borrows the parent file, so the file borrow is released first here
        let relation = Task::add_predecessor(
            &second,
            RelationBuilder::new().predecessor_task(&first),
        )
        .expect("available when file is not emptied");

        assert_eq!(file.borrow().get_relations().len(), 1);
        assert_eq!(second.borrow().get_predecessors().len(), 1);
        assert_eq!(first.borrow().get_successors().len(), 1);

        {
            let relation_ref = relation.borrow();
            assert!(file.borrow_mut().get_relations_mut().remove(&relation_ref));
        }
        assert!(file.borrow().get_relations().is_empty());
    }

    #[test]
    fn get_self_returns_the_shared_handle() {
        let file = project();
        assert!(Rc::ptr_eq(&file.borrow().get_self().expect("has handle"), &file));
    }
}
