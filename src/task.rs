//! Corresponds to Java: `org/mpxj/Task.java`

use std::cell::RefCell;
use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::rc::{Rc, Weak};

use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::abstract_field_container::AbstractFieldContainer;
use crate::accrue_type::AccrueType;
use crate::activity_status::ActivityStatus;
use crate::activity_type::ActivityType;
use crate::common::boolean_helper::BooleanHelper;
use crate::common::local_date_time_helper::LocalDateTimeHelper;
use crate::common::number_helper::NumberHelper;
use crate::constraint_type::ConstraintType;
use crate::duration::Duration;
use crate::earned_value_method::EarnedValueMethod;
use crate::field_container::FieldContainer;
use crate::field_key::FieldKey;
use crate::field_value::FieldValue;
use crate::listener::field_listener::FieldListener;
use crate::local_date_time_range::LocalDateTimeRange;
use crate::mpxj_enum::MpxjEnum;
use crate::notes::{Notes, PlainNotes};
use crate::percent_complete_type::PercentCompleteType;
use crate::priority::Priority;
use crate::project_calendar::ProjectCalendar;
use crate::project_entity_with_id::ProjectEntityWithID;
use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
use crate::project_file::ProjectFile;
use crate::recurring_task::RecurringTask;
use crate::relation::{Builder as RelationBuilder, Relation};
use crate::relation_type::RelationType;
use crate::resource::Resource;
use crate::resource_assignment::ResourceAssignment;
use crate::step::Step;
use crate::task_field::TaskField;
use crate::task_mode::TaskMode;
use crate::task_type::TaskType;
use crate::time_unit::TimeUnit;

// ---------------------------------------------------------------------------
// File header description
// ---------------------------------------------------------------------------
//
// # Java form
//
// ```java
// public final class Task extends AbstractFieldContainer<Task>
//    implements Comparable<Task>, ProjectEntityWithID, ChildTaskContainer
// ```
//
// Fields (the Java side has only 5 member variables; all the rest live in the field table of `AbstractFieldContainer`):
//
// | Java field | Rust field |
// |---|---|
// | `private final ProjectFile m_parentFile` | `parent_file: Weak<RefCell<ProjectFile>>` |
// | `private Task m_parent` | `parent: Weak<RefCell<Task>>` |
// | `private final List<Task> m_children` | `children: Vec<Rc<RefCell<Task>>>` |
// | `private RecurringTask m_recurringTask` | `recurring_task: Option<RecurringTask>` (lazily created, see [`Task::add_recurring_task`]) |
// | `AbstractFieldContainer`'s `m_fields` | `fields: AbstractFieldContainer` |
//
// # References and lifetime (Java → Rust)
//
// * `m_parentFile`: in Java it is managed by the GC, and `Task` directly holds `ProjectFile`. On the Rust side
// `ProjectFile` is stored in `Rc<RefCell<ProjectFile>>`, and the task holds a back-reference to its parent file,
// if a strong reference were used it would form an `Rc` cycle with `ProjectFile → TaskContainer → Task`,
// so it is changed to [`Weak`] (see [`Task::get_parent_file`]).
// * `m_parent` / `m_children`: in Java parent and child strongly reference each other (within the same object graph); on the Rust side
// child tasks are held by a strong `Rc<RefCell<Task>>` reference (sharing the same instance with `TaskContainer`),
// and the back-reference to the parent task uses [`Weak`] to avoid an `Rc` cycle.
// * Java's `Task` has no direct reference to `ProjectContext`; the context is obtained via
// `m_parentFile`. The Rust side keeps the same shape and additionally provides
// [`Task::get_project_context`] as a convenience entry point (Rust-specific).
// * Because parent/child wiring needs the entity's own shared handle, the implicit `this` in Java cannot be omitted on the Rust side,
// therefore [`Task::add_task`] / [`Task::add_child_task`] / [`Task::add_child_task_before`]
//   / [`Task::remove_child_task`] / [`Task::clear_child_tasks`] / [`Task::remove`]
// are changed to associated functions taking `this: &Rc<RefCell<Task>>` as the first argument.
//
// # Field storage (most important)
//
// Java's `set` / `get` / `getCachedValue` are provided by the base class `AbstractFieldContainer`,
// and the three abstract methods `handleFieldChange` / `getAlwaysCalculatedField` / `getCalculationMethod`
// are implemented by `Task`. The Rust side uses **composition** instead: `Task` holds
// an [`AbstractFieldContainer`] and delegates `set` / `get` / `get_cached_value` to it,
// and the responsibility of the three abstract methods above falls to `Task`'s own implementation (see
// [`Task::handle_field_change`],[`Task::get_always_calculated_field`],
// [`Task::get_calculation_method`]).
//
// Java's static `CALCULATED_FIELD_MAP` (`Map<FieldType, Function<Task, Object>>`) and
// `DEPENDENCY_MAP` (`Map<FieldType, List<FieldType>>`) are rewritten on the Rust side as `match`
// dispatch ([`Task::get_calculation_method`] / [`dependency_fields`]), without using function-pointer maps.
//
// # Differences from Java (besides lifetime)
//
// 1. **Calculated values are not written back to the cache**: after computing the value of a field that is not "always calculated", Java's `get(field)`
// calls `set(field, result)` to write it back to the field table as a cache (`AbstractFieldContainer.get`).
// Rust's `&self` getter cannot write to the field table, so it recomputes on demand each time. The semantics are equivalent (an existing
// value in the field table is returned first); the only difference is that no caching is done and listeners do not receive change events for computed results.
// 2. **Field change events and processing order**: in `AbstractFieldContainer.set`, Java first calls
// `handleFieldChange` and then `fireFieldChangeEvent`; on the Rust side the listener broadcast is done
// inside [`AbstractFieldContainer::set`], so `Task::set` can only call
// [`Task::handle_field_change`] afterwards. This does not affect the result of field-dependency cleanup.
// 3. **Index maintenance for `UNIQUE_ID`**: Java's `handleFieldChange` calls
// `m_parentFile.getTasks().updateUniqueID(this, old, new)` when `UNIQUE_ID` changes, and `setID` also calls
// `mapID` / `unmapID`. These all require the entity's own shared reference, so on the Rust side they are handled instead by `TaskContainer`
// building the index at `add` time (see `crate::task_container`). `set_id` / `set_unique_id`
// therefore only write the field value.
// 4. **Type casts**: Java's casts such as `(Duration) get(...)` throw
// `ClassCastException` on a type mismatch; the Rust side uniformly returns `None` (for the plain-text / date / duration read helpers).
// 5. **`Number` → `f64`**: Java's `Number` parameters/return values are uniformly `f64` on the Rust side
// (consistent with the convention of [`crate::common::number_helper::NumberHelper`]).
// 6. **`equals` / `hashCode` / `compareTo`**: Java does not override `equals` / `hashCode`,
// i.e. it compares by object identity; Rust's `Task` is an ordinary struct and cannot express identity, so
// it **does not implement** `PartialEq` / `Eq` / `Hash` (callers use `Rc::ptr_eq` to test for the same entity).
// Java's `compareTo` sorts by ID, which is inconsistent with "identity equality" and does not satisfy Rust's
// `Ord` contract (`a == b` must agree with `a.cmp(&b) == Equal`), so it is changed to
// the [`Task::compare_to`] method rather than an `Ord` implementation.
// 7. **Field value encoding for enums / UUID / Notes**: for enums, `crate::field_value::FieldValue`
// still uses a scalar variant (its documentation explains which values use which variant). To translate the accessors completely, the Rust side
// adopts a lossless encoding convention and restores it on read:
// * enum → `FieldValue::Integer` (`MpxjEnum::value()`; `TaskType` has no int value,
// so its declaration ordinal, i.e. the index into `TaskType::ALL`, is used; `Priority` uses `get_value()`);
//    * `UUID` → `FieldValue::String`(`Uuid::to_string()` / `Uuid::parse_str`);
// * `Notes` → `FieldValue::Notes(Rc<dyn Notes>)` (subtype-preserving).
// 8. **Relation lists**: the values of Java's `TaskField.PREDECESSORS` / `TaskField.SUCCESSORS` are
// `List<Relation>`; the field value is carried by [`FieldValue::Relations`], so `get`, consistently with Java,
// returns the relation list (these two fields belong to the "always calculated" set; they do not read the cache and are computed on the fly).
// [`Task::get_predecessors`] / [`Task::get_successors`] still query `RelationContainer` directly
// (an equivalent implementation; see the respective notes).
//
// # Interfaces of dependent modules (written against the real signatures landed in each module)
//
// The following modules had already landed when this file was written, and the call shapes match them:
//
// ```text
// TaskContainer::add_task(&mut self, task: Rc<RefCell<Task>>)
// TaskContainer::get_next_id(&self) -> i32
// TaskContainer::get_field_type_by_alias(&self, alias: &str) -> Option<FieldKey>
//
// ResourceAssignmentContainer::add(&mut self, assignment: Rc<RefCell<ResourceAssignment>>)
// ResourceAssignmentContainer::get_by_task_unique_id(&self, id: Option<i32>) -> Vec<Rc<RefCell<ResourceAssignment>>>
// ResourceAssignment::with_parent_file(&Rc<RefCell<ProjectFile>>)
// ResourceAssignment::set_task_unique_id / set_resource_unique_id / set_work / set_units
// ResourceAssignment::get_resource(&self) -> Option<Rc<RefCell<Resource>>>
// ResourceAssignment::get_resource_unique_id(&self) -> Option<i32>
// (Java's constant `ResourceAssignment.DEFAULT_UNITS` is not yet provided; its value 1.0 is used directly at `addResourceAssignment`)
//
// RelationContainer::add_predecessor(&mut self, builder: Builder) -> Rc<RefCell<Relation>>
// RelationContainer::remove_predecessor(&mut self, successor: &Task, predecessor: &Task, RelationType, Option<Duration>) -> bool
// RelationContainer::get_predecessors(&self, task: &Task) -> Vec<Rc<RefCell<Relation>>>
// RelationContainer::get_successors(&self, task: &Task) -> Vec<Rc<RefCell<Relation>>>
// Relation::Builder::successor_task(&Rc<RefCell<Task>>) -> Builder
// Relation::get_predecessor_task(&self) / get_successor_task(&self) -> Option<Rc<RefCell<Task>>>
//
// Resource::get_unique_id(&self) -> Option<i32>            // via ProjectEntityWithUniqueID
// ProjectProperties::get_default_duration_units / get_critical_slack_limit / get_status_date / get_project_id
// ProjectProperties: impl TimeUnitDefaultsContainer
// ProjectCalendar::get_unique_id / get_date / get_next_work_start / get_work
// ProjectCalendar: impl TimeUnitDefaultsContainer
// ProjectCalendarContainer::get_by_unique_id(&self, Option<i32>) -> Option<Rc<RefCell<ProjectCalendar>>>
// Duration::convert_units(&self, TimeUnit, &dyn TimeUnitDefaultsContainer)
// ```
//
// Below are the signatures of `crate::project_file::ProjectFile` (also already landed):
//
// ```text
// ProjectFile::get_project_config(&self) -> &ProjectConfig
// ProjectFile::get_project_properties(&self) -> &ProjectProperties
// ProjectFile::get_tasks(&self) -> &TaskContainer
// ProjectFile::get_tasks_mut(&mut self) -> &mut TaskContainer
// ProjectFile::get_resource_assignments(&self) -> &ResourceAssignmentContainer
// ProjectFile::get_resource_assignments_mut(&mut self) -> &mut ResourceAssignmentContainer
// ProjectFile::get_relations(&self) -> &RelationContainer
// ProjectFile::get_relations_mut(&mut self) -> &mut RelationContainer
// ProjectFile::get_calendars(&self) -> &ProjectCalendarContainer
// ProjectFile::get_default_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>>
// ProjectFile::get_child_tasks(&self) -> Vec<Rc<RefCell<Task>>>
// ProjectFile::get_task_by_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Task>>>
// ProjectFile::get_resource_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Resource>>>
// ProjectFile::get_project_context(&self) -> Rc<RefCell<ProjectContext>>
// ProjectFile::get_unique_id_object_sequence(&mut self, TypeId) -> Rc<ObjectSequence>
// ProjectFile::remove_task(&mut self, task: &Rc<RefCell<Task>>)
// ProjectFile::read_external_project(&mut self, file: Option<&str>) -> Option<Rc<RefCell<ProjectFile>>>
// ProjectFile::add_external_project(&mut self, file: Option<&str>, project: Option<Rc<RefCell<ProjectFile>>>)
// ```
//
// Note also: `crate::task_container::TaskContainer::add` calls
// `Task::new_in_file(&project_file, None)`; this file already provides that constructor name.
//
// # Known cross-module interface discrepancies (other files are not modified, only recorded)
//
// * `crate::cpm::microsoft_slack_calculator` is written against
//   `Task::get_parent_file(&self) -> Rc<RefCell<ProjectFile>>`,
// `Task::get_constraint_type(&self) -> ConstraintType`. This file keeps
// the Java semantics: the parent file is a weak reference (tasks without a parent file exist), and `get_constraint_type`
// returns `None` when the field is not set (Java returns `null`). That file needs to handle `Option` accordingly at integration time.
// * `crate::common::slack_helper` is written against `Task::set_start_slack(Duration)` /
// `set_finish_slack(Duration)` / `set_total_slack(Duration)`. This file follows
// Java's nullable semantics and unifies them as `Option<Duration>`; that spot needs a `Some(...)` wrapper at integration time.
// * the tests of `crate::slack_calculator` use `Task::new()`; this file already provides that no-argument constructor.
//
// # Skipped Java Methods (reasons)
//
// | Java method | Reason for skipping |
// |---|---|
// | `getTimephasedDurationValues` / `getTimephasedNumericValues` / `getTimephased*` (43 in total) | The timephased value family (including the fixed-cost proration / accrual implementations such as `getTimephasedFixedCostProRata`) has not been ported yet. The two entry points of [`crate::field_container::FieldContainer`] already exist and are provided by the trait default implementation -- that is, the fallback in Java for "a field not in the `TIMEPHASED_WORK_FUNCTIONS` / `TIMEPHASED_NUMERIC_FUNCTIONS` tables" (an all-`None` list), so it will not produce values that disagree with Java |
// | `getActivityCodeValues` / `addActivityCodeValue` / `defaultActivityCodeValues` | `ActivityCode` / `ActivityCodeValue` are P6/Primavera-specific data; this project is only an MPP/MPX reader, so they are not ported |
// | `getExpenseItems` / `setExpenseItems` / `defaultExpenseItems` | `ExpenseItem` is P6/Primavera-specific data (same as above) |
// | `getLocation` / `setLocation` | `Location` is P6/Primavera-specific data (same as above) |
// | `getBaselineTask` / `getBaselineTask(int)` | Java uses the `Task` object (identity) as the `Map` key, which Rust cannot express (both `ProjectFile`'s baseline and `Task` have landed; what is missing is the Java semantics of "looking up by object identity", see the trade-off in [`crate::default_baseline_strategy`] of using the unique ID as the key instead) |
// | `set(FieldType, boolean)` (private overload) | Rust has no overloading; the `put_bool` helper method takes on this role |
// | static `TIMEPHASED_WORK_FUNCTIONS` / `TIMEPHASED_NUMERIC_FUNCTIONS` | Serve the timephased methods skipped above |
//
// Reflection, JDBC, XML/JAXB, GUI, `javax.*`, `Serializable`: none appear in `Task`, so nothing is skipped.

/// The "parent file context snapshot" needed to create a task (Rust-specific).
///
/// Corresponds to Java: those values **read directly** inside the `Task(ProjectFile file, Task parent)` constructor --
/// the five auto-assignment switches of `ProjectConfig`,
/// `file.getUniqueIdObjectSequence(Task.class).getNext()`,
/// `file.getTasks().getNextID()`, and the
/// `file.getChildTasks().size()` needed by `generateWBS(null)` / `generateOutlineNumber(null)`.
///
/// On the Rust side, borrow rules require hoisting these reads out: the caller ([`crate::TaskContainer`]) reads the snapshot
/// without holding a `ProjectFile` borrow, then passes it to [`Task::new_in_file_with_context`],
/// so that the "create task" path never calls back into the parent file.
#[derive(Debug, Clone, Copy, Default)]
pub struct TaskCreationContext {
    /// Corresponds to Java: `ProjectConfig.autoTaskUniqueID`
    pub auto_task_unique_id: bool,
    /// Corresponds to Java: `ProjectConfig.autoTaskID`
    pub auto_task_id: bool,
    /// Corresponds to Java: `ProjectConfig.autoWBS`
    pub auto_wbs: bool,
    /// Corresponds to Java: `ProjectConfig.autoOutlineNumber`
    pub auto_outline_number: bool,
    /// Corresponds to Java: `ProjectConfig.autoOutlineLevel`
    pub auto_outline_level: bool,
    /// Corresponds to Java: `file.getUniqueIdObjectSequence(Task.class).getNext()`
/// (consumed only when `auto_task_unique_id` is true)
    pub next_unique_id: i32,
    /// Corresponds to Java: `file.getTasks().getNextID()`
/// (consumed only when `auto_task_id` is true)
    pub next_id: i32,
    /// Corresponds to Java: `file.getChildTasks().size()`
/// (consumed only when `auto_wbs` / `auto_outline_number` is set and there is no parent task)
    pub top_level_task_count: usize,
}

/// A task record.
///
/// Corresponds to Java: `public final class Task extends AbstractFieldContainer<Task>
/// implements Comparable<Task>, ProjectEntityWithID, ChildTaskContainer`
///
/// See the file header for how inheritance and interfaces are reproduced.
pub struct Task {
    /// Corresponds to Java: `private final ProjectFile m_parentFile;`
    ///
/// The Rust side stores a weak reference to avoid `ProjectFile → TaskContainer → Task → ProjectFile`
/// forming an `Rc` cycle.
    parent_file: Weak<RefCell<ProjectFile>>,

    /// Corresponds to Java: `private Task m_parent;`
    ///
/// The Rust side stores a weak reference to avoid parent and child tasks strongly referencing each other and forming an `Rc` cycle.
    parent: Weak<RefCell<Task>>,

    /// Corresponds to Java: `private final List<Task> m_children = new ArrayList<>();`
    children: Vec<Rc<RefCell<Task>>>,

/// Corresponds to Java: `m_fields` inherited from `AbstractFieldContainer`.
    ///
/// Java's inheritance is changed to composition on the Rust side; `set` / `get` / `get_cached_value` are delegated to it.
    fields: AbstractFieldContainer,

    /// Corresponds to Java: `AbstractFieldContainer.m_clearDependentFieldsEnabled`.
    ///
/// Java's dependency cleanup recurses through `set` (thereby triggering `handleFieldChange`), so this flag
/// needs to be maintained on the `Task` side as well (the same-named flag in `AbstractFieldContainer` is kept in sync at the same time).
    clear_dependent_fields_enabled: bool,

/// The task's own shared handle (Rust-specific).
    ///
/// Corresponds to Java: none (Java can use `this` as a `Task` reference anywhere).
    ///
/// Java's `Task.setID` calls back into `m_parentFile.getTasks().mapID(val, this)`,
/// which requires passing `this` as a `Task` reference to the container; Rust's `&mut self` cannot produce
/// an `Rc<RefCell<Task>>`, so a weak self-held handle is used to reproduce that callback. It is wired up by
/// [`crate::TaskContainer::add_task`] when the task is added to the container.
    self_ref: Weak<RefCell<Task>>,

    /// Corresponds to Java: `private RecurringTask m_recurringTask;`
    ///
/// Lazily created (see [`Task::add_recurring_task`]); `None` when the task is not recurring.
    recurring_task: Option<RecurringTask>,
}

impl Default for Task {
/// Corresponds to Java: none (Rust-specific, equivalent to [`Task::new`]).
    fn default() -> Self {
        Self::new()
    }
}

impl Task {
/// Construct a standalone task that belongs to no project file (Rust-specific).
    ///
/// Java's constructor is package-private `Task(ProjectFile file, Task parent)` and requires
/// a parent file; the Rust side additionally provides a form without a parent file, making it easy for tests and tooling code to construct temporary tasks
/// (in that case, accessors that depend on the parent file return empty values instead of panicking). To attach it to a project file, use
    /// [`Task::new_in_file`].
    pub fn new() -> Self {
        Self {
            parent_file: Weak::new(),
            parent: Weak::new(),
            children: Vec::new(),
            fields: AbstractFieldContainer::new(),
            clear_dependent_fields_enabled: true,
            self_ref: Weak::new(),
            recurring_task: None,
        }
    }

/// the constructor.
    ///
    /// Corresponds to Java: `Task(ProjectFile file, Task parent)`
    ///
/// In the constructor, Java automatically generates the unique ID, ID, WBS,
/// outline number, and outline level according to the switches in `ProjectConfig`; the Rust side keeps this consistent.
    ///
/// This entry point reads the parent file's `ProjectConfig`, unique ID sequence, and next task ID,
/// so it **requires the caller not to hold a `ProjectFile` borrow** (reads use `try_borrow`; when borrowed,
/// it degrades gracefully to "no auto-assignment"). When holding a borrow, use
/// [`Task::new_in_file_with_context`] instead and pass in a pre-read snapshot.
    ///
/// The method name `new_in_file` is a Rust-side naming choice (Java's constructor name is the same as the class name),
/// for callers such as `crate::task_container::TaskContainer::add`.
    pub fn new_in_file(
        file: &Rc<RefCell<ProjectFile>>,
        parent: Option<&Rc<RefCell<Task>>>,
    ) -> Self {
        let creation = Self::read_creation_context(file);
        Self::new_in_file_with_context(file, parent, creation)
    }

/// Construct a task from a context snapshot read by the caller -- **without calling back into the parent file at all**.
    ///
/// Corresponds to Java: `Task(ProjectFile file, Task parent)`, where the
/// `ProjectConfig` switches, unique ID sequence, next task ID, and top-level task count read inside the constructor are instead passed in via
/// [`TaskCreationContext`].
    ///
/// Because it does not call back into the parent file, this entry point can be used while the caller holds a `ProjectFile` borrow
/// ([`crate::TaskContainer::add`] does exactly this).
    pub fn new_in_file_with_context(
        file: &Rc<RefCell<ProjectFile>>,
        parent: Option<&Rc<RefCell<Task>>>,
        creation: TaskCreationContext,
    ) -> Self {
        let mut task = Self::new();
        task.parent_file = Rc::downgrade(file);
        task.parent = parent.map_or_else(Weak::new, Rc::downgrade);

        if creation.auto_task_unique_id {
            // Java: setUniqueID(file.getUniqueIdObjectSequence(Task.class).getNext());
            task.set_unique_id(Some(creation.next_unique_id));
        }

        if creation.auto_task_id {
            // Java: setID(file.getTasks().getNextID());
            task.set_id(Some(creation.next_id));
        }

        if creation.auto_wbs {
// Java: generateWBS(parent) -- the top-level task count is provided by the caller
            task.generate_wbs_with_count(parent, creation.top_level_task_count);
        }

        if creation.auto_outline_number {
            // Java: generateOutlineNumber(parent)
            task.generate_outline_number_with_count(parent, creation.top_level_task_count);
        }

        if creation.auto_outline_level {
            let outline_level = match parent {
                None => 1,
                Some(parent) => {
                    NumberHelper::get_int(parent.borrow().get_outline_level().map(f64::from)) + 1
                }
            };
            task.set_outline_level(Some(outline_level));
        }

        task
    }

/// Read the context snapshot needed to create a task from the parent file (Rust-specific).
    ///
/// Corresponds to Java: these values are read directly inside the `Task` constructor.
    ///
/// Reads uniformly use `try_borrow`: when the parent file is currently borrowed, it degrades gracefully to a default snapshot (i.e. no auto-
/// assignment is done) without panicking. For exact behavior, use [`Task::new_in_file_with_context`].
    fn read_creation_context(file: &Rc<RefCell<ProjectFile>>) -> TaskCreationContext {
        let mut creation = TaskCreationContext::default();

        if let Ok(file_ref) = file.try_borrow() {
            let config = file_ref.get_project_config();
            creation.auto_task_unique_id = config.get_auto_task_unique_id();
            creation.auto_task_id = config.get_auto_task_id();
            creation.auto_wbs = config.get_auto_wbs();
            creation.auto_outline_number = config.get_auto_outline_number();
            creation.auto_outline_level = config.get_auto_outline_level();

// Java: generateWBS(null) reads file.getChildTasks().size()
            creation.top_level_task_count = file_ref.get_child_tasks().len();
        }

        if creation.auto_task_unique_id {
            if let Ok(mut file_ref) = file.try_borrow_mut() {
                creation.next_unique_id = file_ref
                    .get_unique_id_object_sequence(std::any::TypeId::of::<Task>())
                    .get_next();
            }
        }

        if creation.auto_task_id {
            if let Ok(file_ref) = file.try_borrow() {
                creation.next_id = file_ref.get_tasks().get_next_id();
            }
        }

        creation
    }

/// Get the project file this task belongs to (upgrades the weak reference; returns `None` when the file has been released).
    ///
    /// Corresponds to Java: `public ProjectFile getParentFile()`
    pub fn get_parent_file(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.parent_file.upgrade()
    }

/// Wire up the task's own shared handle (Rust-specific).
    ///
/// Corresponds to Java: none (Java can use `this` as a `Task` reference anywhere).
    ///
/// Registered by [`crate::TaskContainer::add_task`] when the task is added to the container, so that
/// [`Task::set_id`] can reproduce the `mapID(val, this)` callback in Java's `setID`.
    pub fn set_self_reference(&mut self, task: &Rc<RefCell<Task>>) {
        self.self_ref = Rc::downgrade(task);
    }

/// Get the `ProjectContext` this task belongs to (Rust-specific convenience entry point).
    ///
/// On the Java side, `Task` does not directly hold `ProjectContext`; the context is obtained via `m_parentFile`;
/// the same source is kept here.
    pub fn get_project_context(&self) -> Option<Rc<RefCell<crate::project_context::ProjectContext>>> {
        self.parent_file
            .upgrade()
            .map(|file| file.borrow().get_project_context())
    }

    // -----------------------------------------------------------------------
// Field container: placement of set / get / get_cached_value and the three "abstract methods"
    // -----------------------------------------------------------------------

/// Set a field value (including write-back of calculated fields).
    ///
    /// Corresponds to Java: `@Override public void set(FieldType field, Object value)`
    pub fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
        // Corresponds to Java: `Task.set(FieldType, Object)`
// TaskField gets its own handling; the remaining keys (including the user-defined field `UserDefinedField`)
// fall into `super.set(field, value)`, i.e. are written to the field table.
// Previously non-TaskField keys were simply discarded here, so user-defined fields could not actually be written.
        match field {
            FieldKey::Task(task_field) => self.set_task_field(task_field, value),
            _ => self.fields.set(field, value),
        }
    }

/// Read a field value (including calculated fields).
    ///
    /// Corresponds to Java: `@Override public Object get(FieldType field)`
    pub fn get(&self, field: FieldKey) -> Option<FieldValue> {
        if let FieldKey::Task(task_field) = field {
            return self.get_task_field(task_field);
        }

// Corresponds to Java: for non-TaskField keys, `Task.get(FieldType)` falls into `super.get(field)`,
// i.e. it only reads from the field table -- user-defined fields (`UserDefinedField`) take exactly this path.
// Previously this returned `None` directly, so user-defined fields written to the container could not be read back.
        self.get_cached_value(field).cloned()
    }

/// Read only the stored field value without triggering calculation.
    ///
    /// Corresponds to Java: `@Override public Object getCachedValue(FieldType field)`
    pub fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
        self.fields.get_cached_value(field)
    }

/// Register a field change listener.
    ///
    /// Corresponds to Java: `@Override public void addFieldListener(FieldListener listener)`
    pub fn add_field_listener(&mut self, listener: Box<dyn FieldListener>) {
        self.fields.add_field_listener(listener);
    }

/// Clear the field change listeners.
    ///
    /// Corresponds to Java: `@Override public void removeFieldListener(FieldListener listener)`
    pub fn clear_field_listeners(&mut self) {
        self.fields.clear_field_listeners();
    }

/// Get (creating if necessary) the recurring task record.
    ///
    /// Corresponds to Java: `public RecurringTask addRecurringTask()`
    ///
/// On the Java side, a new one is created if `m_recurringTask` is null; on the Rust side this is equivalent to
    /// `Option::get_or_insert_with`.
    pub fn add_recurring_task(&mut self) -> &mut RecurringTask {
        self.recurring_task.get_or_insert_with(RecurringTask::new)
    }

/// Get the recurring task record; returns `None` when the current task is not a recurring task.
    ///
    /// Corresponds to Java: `public RecurringTask getRecurringTask()`
    pub fn get_recurring_task(&self) -> Option<&RecurringTask> {
        self.recurring_task.as_ref()
    }

/// Disable dependency-field cleanup and event firing on field updates.
    ///
    /// Corresponds to Java: `public void disableEvents()`
    pub fn disable_events(&mut self) {
        self.clear_dependent_fields_enabled = false;
        self.fields.disable_events();
    }

/// Enable dependency-field cleanup and event firing on field updates (the default state).
    ///
    /// Corresponds to Java: `public void enableEvents()`
    pub fn enable_events(&mut self) {
        self.clear_dependent_fields_enabled = true;
        self.fields.enable_events();
    }

    /// Corresponds to Java: `@Override boolean getAlwaysCalculatedField(FieldType field)`
    ///
/// Java's `ALWAYS_CALCULATED_FIELDS` set is rewritten as `match` dispatch.
    pub fn get_always_calculated_field(field: TaskField) -> bool {
        matches!(
            field,
            TaskField::ParentTaskUniqueId
                | TaskField::Predecessors
                | TaskField::Successors
                | TaskField::SchedulePercentComplete
                | TaskField::WorkSplits
        )
    }

    /// Corresponds to Java: `@Override Function<Task, Object> getCalculationMethod(FieldType field)`
    ///
/// Java returns a `Function<Task, Object>` (from the static `CALCULATED_FIELD_MAP`);
/// the Rust side instead dispatches directly with `match` and returns the computed result -- semantically
/// equivalent to `CALCULATED_FIELD_MAP.get(field).apply(this)`; unmatched fields return `None`.
    fn get_calculation_method(&self, field: TaskField) -> Option<FieldValue> {
        match field {
            TaskField::ParentTaskUniqueId => {
                self.calculate_parent_task_unique_id().map(FieldValue::Integer)
            }
            TaskField::StartVariance => self.calculate_start_variance().map(FieldValue::Duration),
            TaskField::FinishVariance => self.calculate_finish_variance().map(FieldValue::Duration),
            TaskField::StartSlack => self.calculate_start_slack().map(FieldValue::Duration),
            TaskField::FinishSlack => self.calculate_finish_slack().map(FieldValue::Duration),
            TaskField::CostVariance => self.calculate_cost_variance().map(FieldValue::Double),
            TaskField::DurationVariance => {
                self.calculate_duration_variance().map(FieldValue::Duration)
            }
            TaskField::WorkVariance => self.calculate_work_variance().map(FieldValue::Duration),
            TaskField::Cv => self.calculate_cv().map(FieldValue::Double),
            TaskField::Sv => self.calculate_sv().map(FieldValue::Double),
            TaskField::FreeSlack => self.calculate_free_slack().map(FieldValue::Duration),
            TaskField::TotalSlack => self.calculate_total_slack().map(FieldValue::Duration),
            TaskField::Critical => self.calculate_critical().map(FieldValue::Boolean),
            TaskField::CompleteThrough => {
                self.calculate_complete_through().map(FieldValue::DateTime)
            }
            TaskField::ExternalProject => {
                self.calculate_external_project().map(FieldValue::Boolean)
            }
// Java: the PREDECESSORS / SUCCESSORS / WORK_SPLITS / STEPS entries in CALCULATED_FIELD_MAP
            TaskField::Predecessors => Some(FieldValue::Relations(self.calculate_predecessors())),
            TaskField::Successors => Some(FieldValue::Relations(self.calculate_successors())),
            TaskField::WorkSplits => Some(FieldValue::TimeRanges(self.calculate_work_splits())),
            TaskField::Steps => Some(FieldValue::Steps(Self::default_steps())),
            TaskField::ActivityPercentComplete => self
                .calculate_activity_percent_complete()
                .map(FieldValue::Double),
            TaskField::SchedulePercentComplete => self
                .calculate_schedule_percent_complete()
                .map(FieldValue::Double),
            TaskField::ActualRegularCost => self
                .calculate_regular_cost(
                    self.get_actual_cost(),
                    self.get_actual_overtime_cost(),
                )
                .map(FieldValue::Double),
            TaskField::RemainingRegularCost => self
                .calculate_regular_cost(
                    self.get_remaining_cost(),
                    self.get_remaining_overtime_cost(),
                )
                .map(FieldValue::Double),
            TaskField::ActualRegularWork => self
                .calculate_regular_work(self.get_actual_work(), self.get_actual_overtime_work())
                .map(FieldValue::Duration),
            TaskField::RemainingRegularWork => self
                .calculate_regular_work(
                    self.get_remaining_work(),
                    self.get_remaining_overtime_work(),
                )
                .map(FieldValue::Duration),
            TaskField::Active => Some(FieldValue::Boolean(Self::default_active())),
            TaskField::Type => Some(FieldValue::Integer(task_type_to_value(Self::default_type()))),
            TaskField::TaskMode => Some(FieldValue::Integer(Self::default_task_mode().value())),
// Java: the default values of ACTIVITY_CODE_VALUES / EXPENSE_ITEMS / STEPS are respectively
// an empty Map / an empty List / an empty List, depending on unported types; see "Skipped Java Methods" in the file header.
            TaskField::Expanded => Some(FieldValue::Boolean(Self::default_expanded())),
            TaskField::AutoComputeActuals => {
                Some(FieldValue::Boolean(Self::default_auto_compute_actuals()))
            }
            _ => None,
        }
    }

/// Clear the calculated-field caches affected by this field change.
    ///
    /// Corresponds to Java: `@Override void handleFieldChange(FieldType field, Object oldValue, Object newValue)`
    ///
/// In Java, a change to `UNIQUE_ID` synchronizes the `TaskContainer` index; that action requires the entity's own
/// shared reference, so on the Rust side `TaskContainer` is responsible for it instead (see the file header).
    fn handle_field_change(&mut self, field: TaskField) {
        if field == TaskField::UniqueId {
            return;
        }

        // Java: clearDependentFields(DEPENDENCY_MAP, field);
        self.clear_dependent_fields(field);
    }

/// Recursively clear the fields that need recalculation based on the dependency map.
    ///
    /// Corresponds to Java: `AbstractFieldContainer.clearDependentFields(Map, FieldType)`
/// (the dependency map is `Task.DEPENDENCY_MAP`)
    ///
/// Java's dependency cleanup recurses through `set`, thereby clearing the dependencies of dependencies as well; the Rust side likewise goes through
/// [`Task::set_task_field`] to preserve that recursive semantics.
    fn clear_dependent_fields(&mut self, field: TaskField) {
        if !self.clear_dependent_fields_enabled {
            return;
        }

        let dependencies = dependency_fields(field);
        if dependencies.is_empty() {
            return;
        }

        for dependency in dependencies {
            self.set_task_field(*dependency, None);
        }
    }

/// Write a field value and perform field-change handling when the value actually changes.
    ///
/// Corresponds to Java: `AbstractFieldContainer.set` (which contains `handleFieldChange` and event broadcasting)
    fn set_task_field(&mut self, field: TaskField, value: Option<FieldValue>) {
        let changed = self.fields.get_cached_value(FieldKey::Task(field)).cloned() != value;
        self.fields.set(FieldKey::Task(field), value);

        if changed {
            self.handle_field_change(field);
        }
    }

/// Read a field value: return the value in the field table first, otherwise dispatch and evaluate as a calculated field.
    ///
    /// Corresponds to Java: `AbstractFieldContainer.get`
    fn get_task_field(&self, field: TaskField) -> Option<FieldValue> {
        let always_calculated = Self::get_always_calculated_field(field);

        if !always_calculated {
            if let Some(value) = self.fields.get_cached_value(FieldKey::Task(field)) {
                return Some(value.clone());
            }
        }

// Java: when the computed result is non-null and not always-calculated, it is written back to the field table as a cache;
// Rust's &self getter cannot write the cache, so it recomputes each time (see item 1 in the file header).
        self.get_calculation_method(field)
    }

    // -----------------------------------------------------------------------
// Typed read/write helper methods (corresponding to Java's casts and `set` overloads)
    // -----------------------------------------------------------------------

/// Corresponds to Java: `(String) get(field)` (Java throws `ClassCastException` on a type mismatch; Rust returns `None`)
    fn string_field(&self, field: TaskField) -> Option<String> {
        match self.get_task_field(field) {
            Some(FieldValue::String(value)) => Some(value),
            _ => None,
        }
    }

    /// Corresponds to Java: `(Integer) get(field)`
    fn integer_field(&self, field: TaskField) -> Option<i32> {
        self.get_task_field(field).and_then(|value| value.as_integer())
    }

/// Corresponds to Java: `(Number) get(field)`; the Rust side uniformly uses `f64`
    fn number_field(&self, field: TaskField) -> Option<f64> {
        self.get_task_field(field).and_then(|value| value.as_double())
    }

    /// Corresponds to Java: `BooleanHelper.getBoolean((Boolean) get(field))`
    fn boolean_field(&self, field: TaskField) -> bool {
        BooleanHelper::get_boolean(
            self.get_task_field(field)
                .and_then(|value| value.as_boolean()),
        )
    }

    /// Corresponds to Java: `(LocalDateTime) get(field)`
    fn date_time_field(&self, field: TaskField) -> Option<NaiveDateTime> {
        self.get_task_field(field)
            .and_then(|value| value.as_date_time())
    }

    /// Corresponds to Java: `(Duration) get(field)`
    fn duration_field(&self, field: TaskField) -> Option<Duration> {
        self.get_task_field(field)
            .and_then(|value| value.as_duration())
    }

/// Corresponds to Java: `(UUID) get(field)` (carried as a string on the Rust side; see item 7 in the file header differences)
    fn uuid_field(&self, field: TaskField) -> Option<Uuid> {
        self.string_field(field)
            .and_then(|value| Uuid::parse_str(&value).ok())
    }

/// Corresponds to Java: `set(field, value)` (`Integer` value)
    fn put_integer(&mut self, field: TaskField, value: Option<i32>) {
        self.set_task_field(field, value.map(FieldValue::Integer));
    }

/// Corresponds to Java: `set(field, value)` (`Number` value)
    fn put_number(&mut self, field: TaskField, value: Option<f64>) {
        self.set_task_field(field, value.map(FieldValue::Double));
    }

/// Corresponds to Java: `set(field, value)` (`String` value)
    fn put_string(&mut self, field: TaskField, value: Option<String>) {
        self.set_task_field(field, value.map(FieldValue::String));
    }

    /// Corresponds to Java: `private void set(FieldType field, boolean value)`
    ///
/// Java uses a private overload; Rust has no overloading, so a named helper method is used instead.
    fn put_bool(&mut self, field: TaskField, value: bool) {
        self.set_task_field(field, Some(FieldValue::Boolean(value)));
    }

/// Corresponds to Java: `set(field, value)` (`LocalDateTime` value)
    fn put_date_time(&mut self, field: TaskField, value: Option<NaiveDateTime>) {
        self.set_task_field(field, value.map(FieldValue::DateTime));
    }

/// Corresponds to Java: `set(field, value)` (`Duration` value)
    fn put_duration(&mut self, field: TaskField, value: Option<Duration>) {
        self.set_task_field(field, value.map(FieldValue::Duration));
    }

/// Corresponds to Java: `set(field, value)` (enum value; encoded as an int on the Rust side, see item 7 in the file header differences)
    fn put_enum(&mut self, field: TaskField, value: Option<i32>) {
        self.set_task_field(field, value.map(FieldValue::Integer));
    }

/// Corresponds to Java: `set(field, value)` (`UUID` value; encoded as a string on the Rust side)
    fn put_uuid(&mut self, field: TaskField, value: Option<Uuid>) {
        self.put_string(field, value.map(|value| value.to_string()));
    }
}

/// Get the dependency list for a task field.
///
/// Corresponds to Java: `private static final Map<FieldType, List<FieldType>> DEPENDENCY_MAP`
/// (the `FieldContainerDependencies` declarations in the static initializer block, rewritten as `match` dispatch)
fn dependency_fields(field: TaskField) -> &'static [TaskField] {
    match field {
        TaskField::StartVariance => &[TaskField::Start, TaskField::BaselineStart],
        TaskField::FinishVariance => &[TaskField::Finish, TaskField::BaselineFinish],
        TaskField::StartSlack => &[TaskField::EarlyStart, TaskField::LateStart],
        TaskField::FinishSlack => &[TaskField::EarlyFinish, TaskField::LateFinish],
        TaskField::TotalSlack => &[
            TaskField::StartSlack,
            TaskField::FinishSlack,
            TaskField::ActualStart,
        ],
        TaskField::FreeSlack => &[TaskField::EarlyStart, TaskField::EarlyFinish],
        TaskField::CostVariance => &[TaskField::Cost, TaskField::BaselineCost],
        TaskField::DurationVariance => &[TaskField::Duration, TaskField::BaselineDuration],
        TaskField::WorkVariance => &[TaskField::Work, TaskField::BaselineWork],
        TaskField::Cv => &[TaskField::Bcwp, TaskField::Acwp],
        TaskField::Sv => &[TaskField::Bcwp, TaskField::Bcws],
        TaskField::Critical => &[TaskField::TotalSlack, TaskField::ActualFinish],
        TaskField::CompleteThrough => &[
            TaskField::Duration,
            TaskField::ActualStart,
            TaskField::PercentComplete,
        ],
        TaskField::ExternalProject => &[TaskField::SubprojectFile, TaskField::ExternalTask],
        TaskField::ActivityPercentComplete => &[
            TaskField::PercentCompleteType,
            TaskField::PercentComplete,
            TaskField::PercentWorkComplete,
            TaskField::PhysicalPercentComplete,
        ],
        TaskField::RemainingRegularCost => &[
            TaskField::RemainingCost,
            TaskField::RemainingOvertimeCost,
        ],
        TaskField::ActualRegularCost => &[TaskField::ActualCost, TaskField::ActualOvertimeCost],
        TaskField::RemainingRegularWork => &[
            TaskField::RemainingWork,
            TaskField::RemainingOvertimeWork,
        ],
        TaskField::ActualRegularWork => &[TaskField::ActualWork, TaskField::ActualOvertimeWork],
        _ => &[],
    }
}

/// Get the task field at the given index; returns `None` when out of bounds.
///
/// Corresponds to Java: `private TaskField selectField(TaskField[] fields, int index)`
/// (Java throws `IllegalArgumentException` for an out-of-bounds index; the Rust side returns `None`)
fn select_field(fields: &'static [TaskField], index: i32) -> Option<TaskField> {
    let index = usize::try_from(index).ok()?;
    if index < 1 {
        return None;
    }
    fields.get(index - 1).copied()
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.CUSTOM_COST`
fn custom_cost_fields() -> &'static [TaskField] {
    &[
        TaskField::Cost1,
        TaskField::Cost2,
        TaskField::Cost3,
        TaskField::Cost4,
        TaskField::Cost5,
        TaskField::Cost6,
        TaskField::Cost7,
        TaskField::Cost8,
        TaskField::Cost9,
        TaskField::Cost10,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.CUSTOM_DATE`
fn custom_date_fields() -> &'static [TaskField] {
    &[
        TaskField::Date1,
        TaskField::Date2,
        TaskField::Date3,
        TaskField::Date4,
        TaskField::Date5,
        TaskField::Date6,
        TaskField::Date7,
        TaskField::Date8,
        TaskField::Date9,
        TaskField::Date10,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.CUSTOM_DURATION`
fn custom_duration_fields() -> &'static [TaskField] {
    &[
        TaskField::Duration1,
        TaskField::Duration2,
        TaskField::Duration3,
        TaskField::Duration4,
        TaskField::Duration5,
        TaskField::Duration6,
        TaskField::Duration7,
        TaskField::Duration8,
        TaskField::Duration9,
        TaskField::Duration10,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.CUSTOM_FLAG`
fn custom_flag_fields() -> &'static [TaskField] {
    &[
        TaskField::Flag1,
        TaskField::Flag2,
        TaskField::Flag3,
        TaskField::Flag4,
        TaskField::Flag5,
        TaskField::Flag6,
        TaskField::Flag7,
        TaskField::Flag8,
        TaskField::Flag9,
        TaskField::Flag10,
        TaskField::Flag11,
        TaskField::Flag12,
        TaskField::Flag13,
        TaskField::Flag14,
        TaskField::Flag15,
        TaskField::Flag16,
        TaskField::Flag17,
        TaskField::Flag18,
        TaskField::Flag19,
        TaskField::Flag20,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.CUSTOM_FINISH`
fn custom_finish_fields() -> &'static [TaskField] {
    &[
        TaskField::Finish1,
        TaskField::Finish2,
        TaskField::Finish3,
        TaskField::Finish4,
        TaskField::Finish5,
        TaskField::Finish6,
        TaskField::Finish7,
        TaskField::Finish8,
        TaskField::Finish9,
        TaskField::Finish10,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.CUSTOM_NUMBER`
fn custom_number_fields() -> &'static [TaskField] {
    &[
        TaskField::Number1,
        TaskField::Number2,
        TaskField::Number3,
        TaskField::Number4,
        TaskField::Number5,
        TaskField::Number6,
        TaskField::Number7,
        TaskField::Number8,
        TaskField::Number9,
        TaskField::Number10,
        TaskField::Number11,
        TaskField::Number12,
        TaskField::Number13,
        TaskField::Number14,
        TaskField::Number15,
        TaskField::Number16,
        TaskField::Number17,
        TaskField::Number18,
        TaskField::Number19,
        TaskField::Number20,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.CUSTOM_START`
fn custom_start_fields() -> &'static [TaskField] {
    &[
        TaskField::Start1,
        TaskField::Start2,
        TaskField::Start3,
        TaskField::Start4,
        TaskField::Start5,
        TaskField::Start6,
        TaskField::Start7,
        TaskField::Start8,
        TaskField::Start9,
        TaskField::Start10,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.CUSTOM_TEXT`
fn custom_text_fields() -> &'static [TaskField] {
    &[
        TaskField::Text1,
        TaskField::Text2,
        TaskField::Text3,
        TaskField::Text4,
        TaskField::Text5,
        TaskField::Text6,
        TaskField::Text7,
        TaskField::Text8,
        TaskField::Text9,
        TaskField::Text10,
        TaskField::Text11,
        TaskField::Text12,
        TaskField::Text13,
        TaskField::Text14,
        TaskField::Text15,
        TaskField::Text16,
        TaskField::Text17,
        TaskField::Text18,
        TaskField::Text19,
        TaskField::Text20,
        TaskField::Text21,
        TaskField::Text22,
        TaskField::Text23,
        TaskField::Text24,
        TaskField::Text25,
        TaskField::Text26,
        TaskField::Text27,
        TaskField::Text28,
        TaskField::Text29,
        TaskField::Text30,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.CUSTOM_OUTLINE_CODE`
fn custom_outline_code_fields() -> &'static [TaskField] {
    &[
        TaskField::OutlineCode1,
        TaskField::OutlineCode2,
        TaskField::OutlineCode3,
        TaskField::OutlineCode4,
        TaskField::OutlineCode5,
        TaskField::OutlineCode6,
        TaskField::OutlineCode7,
        TaskField::OutlineCode8,
        TaskField::OutlineCode9,
        TaskField::OutlineCode10,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.ENTERPRISE_CUSTOM_COST`
fn enterprise_cost_fields() -> &'static [TaskField] {
    &[
        TaskField::EnterpriseCost1,
        TaskField::EnterpriseCost2,
        TaskField::EnterpriseCost3,
        TaskField::EnterpriseCost4,
        TaskField::EnterpriseCost5,
        TaskField::EnterpriseCost6,
        TaskField::EnterpriseCost7,
        TaskField::EnterpriseCost8,
        TaskField::EnterpriseCost9,
        TaskField::EnterpriseCost10,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.ENTERPRISE_CUSTOM_DATE`
fn enterprise_date_fields() -> &'static [TaskField] {
    &[
        TaskField::EnterpriseDate1,
        TaskField::EnterpriseDate2,
        TaskField::EnterpriseDate3,
        TaskField::EnterpriseDate4,
        TaskField::EnterpriseDate5,
        TaskField::EnterpriseDate6,
        TaskField::EnterpriseDate7,
        TaskField::EnterpriseDate8,
        TaskField::EnterpriseDate9,
        TaskField::EnterpriseDate10,
        TaskField::EnterpriseDate11,
        TaskField::EnterpriseDate12,
        TaskField::EnterpriseDate13,
        TaskField::EnterpriseDate14,
        TaskField::EnterpriseDate15,
        TaskField::EnterpriseDate16,
        TaskField::EnterpriseDate17,
        TaskField::EnterpriseDate18,
        TaskField::EnterpriseDate19,
        TaskField::EnterpriseDate20,
        TaskField::EnterpriseDate21,
        TaskField::EnterpriseDate22,
        TaskField::EnterpriseDate23,
        TaskField::EnterpriseDate24,
        TaskField::EnterpriseDate25,
        TaskField::EnterpriseDate26,
        TaskField::EnterpriseDate27,
        TaskField::EnterpriseDate28,
        TaskField::EnterpriseDate29,
        TaskField::EnterpriseDate30,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.ENTERPRISE_CUSTOM_DURATION`
fn enterprise_duration_fields() -> &'static [TaskField] {
    &[
        TaskField::EnterpriseDuration1,
        TaskField::EnterpriseDuration2,
        TaskField::EnterpriseDuration3,
        TaskField::EnterpriseDuration4,
        TaskField::EnterpriseDuration5,
        TaskField::EnterpriseDuration6,
        TaskField::EnterpriseDuration7,
        TaskField::EnterpriseDuration8,
        TaskField::EnterpriseDuration9,
        TaskField::EnterpriseDuration10,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.ENTERPRISE_CUSTOM_FLAG`
fn enterprise_flag_fields() -> &'static [TaskField] {
    &[
        TaskField::EnterpriseFlag1,
        TaskField::EnterpriseFlag2,
        TaskField::EnterpriseFlag3,
        TaskField::EnterpriseFlag4,
        TaskField::EnterpriseFlag5,
        TaskField::EnterpriseFlag6,
        TaskField::EnterpriseFlag7,
        TaskField::EnterpriseFlag8,
        TaskField::EnterpriseFlag9,
        TaskField::EnterpriseFlag10,
        TaskField::EnterpriseFlag11,
        TaskField::EnterpriseFlag12,
        TaskField::EnterpriseFlag13,
        TaskField::EnterpriseFlag14,
        TaskField::EnterpriseFlag15,
        TaskField::EnterpriseFlag16,
        TaskField::EnterpriseFlag17,
        TaskField::EnterpriseFlag18,
        TaskField::EnterpriseFlag19,
        TaskField::EnterpriseFlag20,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.ENTERPRISE_CUSTOM_NUMBER`
fn enterprise_number_fields() -> &'static [TaskField] {
    &[
        TaskField::EnterpriseNumber1,
        TaskField::EnterpriseNumber2,
        TaskField::EnterpriseNumber3,
        TaskField::EnterpriseNumber4,
        TaskField::EnterpriseNumber5,
        TaskField::EnterpriseNumber6,
        TaskField::EnterpriseNumber7,
        TaskField::EnterpriseNumber8,
        TaskField::EnterpriseNumber9,
        TaskField::EnterpriseNumber10,
        TaskField::EnterpriseNumber11,
        TaskField::EnterpriseNumber12,
        TaskField::EnterpriseNumber13,
        TaskField::EnterpriseNumber14,
        TaskField::EnterpriseNumber15,
        TaskField::EnterpriseNumber16,
        TaskField::EnterpriseNumber17,
        TaskField::EnterpriseNumber18,
        TaskField::EnterpriseNumber19,
        TaskField::EnterpriseNumber20,
        TaskField::EnterpriseNumber21,
        TaskField::EnterpriseNumber22,
        TaskField::EnterpriseNumber23,
        TaskField::EnterpriseNumber24,
        TaskField::EnterpriseNumber25,
        TaskField::EnterpriseNumber26,
        TaskField::EnterpriseNumber27,
        TaskField::EnterpriseNumber28,
        TaskField::EnterpriseNumber29,
        TaskField::EnterpriseNumber30,
        TaskField::EnterpriseNumber31,
        TaskField::EnterpriseNumber32,
        TaskField::EnterpriseNumber33,
        TaskField::EnterpriseNumber34,
        TaskField::EnterpriseNumber35,
        TaskField::EnterpriseNumber36,
        TaskField::EnterpriseNumber37,
        TaskField::EnterpriseNumber38,
        TaskField::EnterpriseNumber39,
        TaskField::EnterpriseNumber40,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.ENTERPRISE_CUSTOM_TEXT`
fn enterprise_text_fields() -> &'static [TaskField] {
    &[
        TaskField::EnterpriseText1,
        TaskField::EnterpriseText2,
        TaskField::EnterpriseText3,
        TaskField::EnterpriseText4,
        TaskField::EnterpriseText5,
        TaskField::EnterpriseText6,
        TaskField::EnterpriseText7,
        TaskField::EnterpriseText8,
        TaskField::EnterpriseText9,
        TaskField::EnterpriseText10,
        TaskField::EnterpriseText11,
        TaskField::EnterpriseText12,
        TaskField::EnterpriseText13,
        TaskField::EnterpriseText14,
        TaskField::EnterpriseText15,
        TaskField::EnterpriseText16,
        TaskField::EnterpriseText17,
        TaskField::EnterpriseText18,
        TaskField::EnterpriseText19,
        TaskField::EnterpriseText20,
        TaskField::EnterpriseText21,
        TaskField::EnterpriseText22,
        TaskField::EnterpriseText23,
        TaskField::EnterpriseText24,
        TaskField::EnterpriseText25,
        TaskField::EnterpriseText26,
        TaskField::EnterpriseText27,
        TaskField::EnterpriseText28,
        TaskField::EnterpriseText29,
        TaskField::EnterpriseText30,
        TaskField::EnterpriseText31,
        TaskField::EnterpriseText32,
        TaskField::EnterpriseText33,
        TaskField::EnterpriseText34,
        TaskField::EnterpriseText35,
        TaskField::EnterpriseText36,
        TaskField::EnterpriseText37,
        TaskField::EnterpriseText38,
        TaskField::EnterpriseText39,
        TaskField::EnterpriseText40,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.BASELINE_COSTS`
fn baseline_cost_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1Cost,
        TaskField::Baseline2Cost,
        TaskField::Baseline3Cost,
        TaskField::Baseline4Cost,
        TaskField::Baseline5Cost,
        TaskField::Baseline6Cost,
        TaskField::Baseline7Cost,
        TaskField::Baseline8Cost,
        TaskField::Baseline9Cost,
        TaskField::Baseline10Cost,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.BASELINE_DURATIONS`
fn baseline_duration_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1Duration,
        TaskField::Baseline2Duration,
        TaskField::Baseline3Duration,
        TaskField::Baseline4Duration,
        TaskField::Baseline5Duration,
        TaskField::Baseline6Duration,
        TaskField::Baseline7Duration,
        TaskField::Baseline8Duration,
        TaskField::Baseline9Duration,
        TaskField::Baseline10Duration,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.BASELINE_ESTIMATED_DURATIONS`
fn baseline_estimated_duration_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1EstimatedDuration,
        TaskField::Baseline2EstimatedDuration,
        TaskField::Baseline3EstimatedDuration,
        TaskField::Baseline4EstimatedDuration,
        TaskField::Baseline5EstimatedDuration,
        TaskField::Baseline6EstimatedDuration,
        TaskField::Baseline7EstimatedDuration,
        TaskField::Baseline8EstimatedDuration,
        TaskField::Baseline9EstimatedDuration,
        TaskField::Baseline10EstimatedDuration,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.BASELINE_STARTS`
fn baseline_start_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1Start,
        TaskField::Baseline2Start,
        TaskField::Baseline3Start,
        TaskField::Baseline4Start,
        TaskField::Baseline5Start,
        TaskField::Baseline6Start,
        TaskField::Baseline7Start,
        TaskField::Baseline8Start,
        TaskField::Baseline9Start,
        TaskField::Baseline10Start,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.BASELINE_ESTIMATED_STARTS`
fn baseline_estimated_start_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1EstimatedStart,
        TaskField::Baseline2EstimatedStart,
        TaskField::Baseline3EstimatedStart,
        TaskField::Baseline4EstimatedStart,
        TaskField::Baseline5EstimatedStart,
        TaskField::Baseline6EstimatedStart,
        TaskField::Baseline7EstimatedStart,
        TaskField::Baseline8EstimatedStart,
        TaskField::Baseline9EstimatedStart,
        TaskField::Baseline10EstimatedStart,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.BASELINE_FINISHES`
fn baseline_finish_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1Finish,
        TaskField::Baseline2Finish,
        TaskField::Baseline3Finish,
        TaskField::Baseline4Finish,
        TaskField::Baseline5Finish,
        TaskField::Baseline6Finish,
        TaskField::Baseline7Finish,
        TaskField::Baseline8Finish,
        TaskField::Baseline9Finish,
        TaskField::Baseline10Finish,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.BASELINE_ESTIMATED_FINISHES`
fn baseline_estimated_finish_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1EstimatedFinish,
        TaskField::Baseline2EstimatedFinish,
        TaskField::Baseline3EstimatedFinish,
        TaskField::Baseline4EstimatedFinish,
        TaskField::Baseline5EstimatedFinish,
        TaskField::Baseline6EstimatedFinish,
        TaskField::Baseline7EstimatedFinish,
        TaskField::Baseline8EstimatedFinish,
        TaskField::Baseline9EstimatedFinish,
        TaskField::Baseline10EstimatedFinish,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.BASELINE_WORKS`
fn baseline_work_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1Work,
        TaskField::Baseline2Work,
        TaskField::Baseline3Work,
        TaskField::Baseline4Work,
        TaskField::Baseline5Work,
        TaskField::Baseline6Work,
        TaskField::Baseline7Work,
        TaskField::Baseline8Work,
        TaskField::Baseline9Work,
        TaskField::Baseline10Work,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.BASELINE_FIXED_COSTS`
fn baseline_fixed_cost_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1FixedCost,
        TaskField::Baseline2FixedCost,
        TaskField::Baseline3FixedCost,
        TaskField::Baseline4FixedCost,
        TaskField::Baseline5FixedCost,
        TaskField::Baseline6FixedCost,
        TaskField::Baseline7FixedCost,
        TaskField::Baseline8FixedCost,
        TaskField::Baseline9FixedCost,
        TaskField::Baseline10FixedCost,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.BASELINE_FIXED_COST_ACCRUALS`
fn baseline_fixed_cost_accrual_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1FixedCostAccrual,
        TaskField::Baseline2FixedCostAccrual,
        TaskField::Baseline3FixedCostAccrual,
        TaskField::Baseline4FixedCostAccrual,
        TaskField::Baseline5FixedCostAccrual,
        TaskField::Baseline6FixedCostAccrual,
        TaskField::Baseline7FixedCostAccrual,
        TaskField::Baseline8FixedCostAccrual,
        TaskField::Baseline9FixedCostAccrual,
        TaskField::Baseline10FixedCostAccrual,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.BASELINE_BUDGET_COSTS`
fn baseline_budget_cost_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1BudgetCost,
        TaskField::Baseline2BudgetCost,
        TaskField::Baseline3BudgetCost,
        TaskField::Baseline4BudgetCost,
        TaskField::Baseline5BudgetCost,
        TaskField::Baseline6BudgetCost,
        TaskField::Baseline7BudgetCost,
        TaskField::Baseline8BudgetCost,
        TaskField::Baseline9BudgetCost,
        TaskField::Baseline10BudgetCost,
    ]
}

/// Corresponds to Java: `org.mpxj.common.TaskFieldLists.BASELINE_BUDGET_WORKS`
fn baseline_budget_work_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1BudgetWork,
        TaskField::Baseline2BudgetWork,
        TaskField::Baseline3BudgetWork,
        TaskField::Baseline4BudgetWork,
        TaskField::Baseline5BudgetWork,
        TaskField::Baseline6BudgetWork,
        TaskField::Baseline7BudgetWork,
        TaskField::Baseline8BudgetWork,
        TaskField::Baseline9BudgetWork,
        TaskField::Baseline10BudgetWork,
    ]
}

// ---------------------------------------------------------------------------
// Enum / type encoding helpers (see item 7 in the file header differences)
// ---------------------------------------------------------------------------

/// Encode `TaskType` as its declaration ordinal (on the Java side this enum has no int value).
fn task_type_to_value(value: TaskType) -> i32 {
    TaskType::ALL
        .iter()
        .position(|item| *item == value)
        .map_or(0, |index| index as i32)
}

/// Restore the int in the field to a `TaskType`.
fn task_type_from_value(value: i32) -> Option<TaskType> {
    TaskType::get_instance(value)
}

/// Restore the int in the field to a `ConstraintType` (falls back to `AS_SOON_AS_POSSIBLE` when out of bounds, matching Java).
fn constraint_type_from_value(value: i32) -> ConstraintType {
    ConstraintType::get_instance(value)
}

// ---------------------------------------------------------------------------
// Automatic WBS / outline number generation
// ---------------------------------------------------------------------------

impl Task {
/// Automatically generate the WBS field value.
    ///
    /// Corresponds to Java: `public void generateWBS(Task parent)`
    ///
/// Java throws `NullPointerException` when `parent.getWBS()` is `null`;
/// the Rust side treats it as an empty string (no panic).
    pub fn generate_wbs(&mut self, parent: Option<&Rc<RefCell<Task>>>) {
        let top_level_count = self.top_level_task_count(parent);
        self.generate_wbs_with_count(parent, top_level_count);
    }

/// Generate the WBS and explicitly pass in "the currently registered top-level task count".
    ///
/// Corresponds to Java: `public void generateWBS(Task parent)`, where the `parent == null`
/// branch reads `m_parentFile.getChildTasks().size()`.
    ///
/// [`crate::TaskContainer::update_structure`] calls this method while building the task hierarchy:
/// it cannot safely call back into `ProjectFile` (reentrant borrow), and it is itself the authoritative source of that count
/// (`TaskContainer.child_tasks` is `ProjectFile.getChildTasks()`), so it is passed in explicitly.
    pub fn generate_wbs_with_count(
        &mut self,
        parent: Option<&Rc<RefCell<Task>>>,
        top_level_count: usize,
    ) {
        let wbs = match parent {
            None => {
                if NumberHelper::get_int(self.get_unique_id().map(f64::from)) == 0 {
                    "0".to_owned()
                } else {
                    (top_level_count + 1).to_string()
                }
            }
            Some(parent) => {
                let mut wbs = parent.borrow().get_wbs().unwrap_or_default();
                let child_task_count = parent.borrow().get_child_tasks().len() + 1;
                if wbs == "0" {
                    wbs = child_task_count.to_string();
                } else {
                    wbs.push('.');
                    wbs.push_str(&child_task_count.to_string());
                }
                wbs
            }
        };

        self.set_wbs(Some(wbs));
    }

/// Automatically generate the outline number field value.
    ///
    /// Corresponds to Java: `public void generateOutlineNumber(Task parent)`
    pub fn generate_outline_number(&mut self, parent: Option<&Rc<RefCell<Task>>>) {
        let top_level_count = self.top_level_task_count(parent);
        self.generate_outline_number_with_count(parent, top_level_count);
    }

/// Generate the outline number and explicitly pass in "the currently registered top-level task count".
    ///
/// Corresponds to Java: `public void generateOutlineNumber(Task parent)`; the semantics and value source
/// are the same as [`Task::generate_wbs_with_count`].
    pub fn generate_outline_number_with_count(
        &mut self,
        parent: Option<&Rc<RefCell<Task>>>,
        top_level_count: usize,
    ) {
        let outline = match parent {
            None => {
                if NumberHelper::get_int(self.get_unique_id().map(f64::from)) == 0 {
                    "0".to_owned()
                } else {
                    (top_level_count + 1).to_string()
                }
            }
            Some(parent) => {
                let mut outline = parent.borrow().get_outline_number().unwrap_or_default();
                if let Some(index) = outline.rfind(".0") {
                    outline.truncate(index);
                }

                let child_task_count = parent.borrow().get_child_tasks().len() + 1;
                if outline == "0" {
                    outline = child_task_count.to_string();
                } else {
                    outline.push('.');
                    outline.push_str(&child_task_count.to_string());
                }
                outline
            }
        };

        self.set_outline_number(Some(outline));
    }

/// Read "the currently registered top-level task count", equivalent to Java's
    /// `m_parentFile.getChildTasks().size()`.
    ///
/// It is only meaningful when `parent == None`; returns `0` when `parent` is non-empty (that branch does not use it).
/// Degrades gracefully to `0` when the parent file is missing or currently mutably borrowed (no panic).
    fn top_level_task_count(&self, parent: Option<&Rc<RefCell<Task>>>) -> usize {
        if parent.is_some() {
            return 0;
        }

        self.get_parent_file()
            .and_then(|file| file.try_borrow().ok().map(|file| file.get_child_tasks().len()))
            .unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// Notes
// ---------------------------------------------------------------------------

impl Task {
/// Add plain-text notes to the current task.
    ///
    /// Corresponds to Java: `public void setNotes(String notes)`
    ///
    /// Java: `set(TaskField.NOTES, notes == null ? null : new Notes(notes));`
/// The Rust side uses [`PlainNotes`] to carry this concrete "plain-text notes" form.
    pub fn set_notes(&mut self, notes: Option<String>) {
        let value = notes.map(|text| {
            let notes: Rc<dyn Notes> = Rc::new(PlainNotes::new(Some(text)));
            FieldValue::Notes(notes)
        });
        self.set_task_field(TaskField::Notes, value);
    }

/// Set the `Notes` instance representing the task notes (the subtype information of `RtfNotes` / `HtmlNotes` / `StructuredNotes` /
/// `ParentNotes` is preserved losslessly via [`FieldValue::Notes`]).
    ///
    /// Corresponds to Java: `public void setNotesObject(Notes notes)`
    ///
/// On the Rust side `Notes` is a trait object, so the caller can only hand over a borrow; therefore a
/// subtype-preserving shallow copy is made via [`Notes::to_rc`] (Java stores the same instance directly).
    pub fn set_notes_object(&mut self, notes: Option<&dyn Notes>) {
        let value = notes.map(|notes| FieldValue::Notes(notes.to_rc()));
        self.set_task_field(TaskField::Notes, value);
    }
}

// ---------------------------------------------------------------------------
// Hierarchy structure (parent/child tasks)
// ---------------------------------------------------------------------------

impl Task {
/// Add a child task (with automatic WBS completion).
    ///
    /// Corresponds to Java: `@Override public Task addTask()`
    ///
/// Java's instance method accesses the implicit `this`; the Rust side needs the entity's own shared handle,
/// so it is changed to an associated function taking `this` as the first argument (see the file header).
    pub fn add_task(this: &Rc<RefCell<Task>>) -> Option<Rc<RefCell<Task>>> {
        let file = this.borrow().parent_file.upgrade()?;

        let task = Rc::new(RefCell::new(Task::new_in_file(&file, Some(this))));

        this.borrow_mut().children.push(Rc::clone(&task));
        file.borrow_mut().get_tasks_mut().add_task(Rc::clone(&task));
        this.borrow_mut().set_summary(true);

        Some(task)
    }

/// Attach a child task to the current task by outline level (used to build the hierarchy when reading MPX files).
    ///
    /// Corresponds to Java: `public void addChildTask(Task child, int childOutlineLevel)`
    pub fn add_child_task_at_level(
        this: &Rc<RefCell<Task>>,
        child: &Rc<RefCell<Task>>,
        child_outline_level: i32,
    ) {
        let outline_level = NumberHelper::get_int(this.borrow().get_outline_level().map(f64::from));

        let last_child = this.borrow().children.last().cloned();

        if (outline_level + 1) == child_outline_level {
            this.borrow_mut().children.push(Rc::clone(child));
            this.borrow_mut().set_summary(true);
        } else if let Some(last_child) = last_child {
            Task::add_child_task_at_level(&last_child, child, child_outline_level);
        }
    }

/// Associate a child task and automatically set the child task's outline level as needed.
    ///
    /// Corresponds to Java: `public void addChildTask(Task child)`
    pub fn add_child_task(this: &Rc<RefCell<Task>>, child: &Rc<RefCell<Task>>) {
        // Java: boolean autoOutlineLevel = m_parentFile.getProjectConfig().getAutoOutlineLevel();
        //
// Reads use `try_borrow`: the caller may currently hold a `ProjectFile` borrow, in which case the default
// `true` is used without panicking (Java has no such restriction).
        let auto_outline_level = match this.borrow().parent_file.upgrade() {
            Some(file) => match file.try_borrow() {
                Ok(file) => file.get_project_config().get_auto_outline_level(),
                Err(_) => true,
            },
            None => false,
        };

        Task::add_child_task_with_auto_outline_level(this, child, auto_outline_level);
    }

/// Associate a child task; the `autoOutlineLevel` switch is passed in explicitly by the caller.
    ///
/// Corresponds to Java: `public void addChildTask(Task child)`, where the
/// `autoOutlineLevel` read from `ProjectConfig` is instead passed in as a parameter -- this way the method does not call back into the parent file and can be used while `ProjectFile`
/// is already borrowed ([`crate::TaskContainer::update_structure`] does exactly this).
    pub fn add_child_task_with_auto_outline_level(
        this: &Rc<RefCell<Task>>,
        child: &Rc<RefCell<Task>>,
        auto_outline_level: bool,
    ) {
        child.borrow_mut().parent = Rc::downgrade(this);
        this.borrow_mut().children.push(Rc::clone(child));
        this.borrow_mut().set_summary(true);

        if auto_outline_level {
            let outline_level =
                NumberHelper::get_int(this.borrow().get_outline_level().map(f64::from)) + 1;
            child.borrow_mut().set_outline_level(Some(outline_level));
        }
    }

/// Insert a child task before the specified sibling task.
    ///
    /// Corresponds to Java: `public void addChildTaskBefore(Task child, Task previousSibling)`
    pub fn add_child_task_before(
        this: &Rc<RefCell<Task>>,
        child: &Rc<RefCell<Task>>,
        previous_sibling: &Rc<RefCell<Task>>,
    ) {
        let index = this
            .borrow()
            .children
            .iter()
            .position(|item| Rc::ptr_eq(item, previous_sibling));

        match index {
            None => this.borrow_mut().children.push(Rc::clone(child)),
            Some(index) => this.borrow_mut().children.insert(index, Rc::clone(child)),
        }

        child.borrow_mut().parent = Rc::downgrade(this);
        this.borrow_mut().set_summary(true);

        let auto_outline_level = this
            .borrow()
            .parent_file
            .upgrade()
            .map_or(false, |file| {
                file.borrow().get_project_config().get_auto_outline_level()
            });

        if auto_outline_level {
            let outline_level =
                NumberHelper::get_int(this.borrow().get_outline_level().map(f64::from)) + 1;
            child.borrow_mut().set_outline_level(Some(outline_level));
        }
    }

/// Remove a child task.
    ///
    /// Corresponds to Java: `public void removeChildTask(Task child)`
    pub fn remove_child_task(this: &Rc<RefCell<Task>>, child: &Rc<RefCell<Task>>) {
        let removed = {
            let mut task = this.borrow_mut();
            match task
                .children
                .iter()
                .position(|item| Rc::ptr_eq(item, child))
            {
                Some(index) => {
                    task.children.remove(index);
                    true
                }
                None => false,
            }
        };

        if removed {
            child.borrow_mut().parent = Weak::new();
        }

        let is_empty = this.borrow().children.is_empty();
        this.borrow_mut().set_summary(!is_empty);
    }

/// Clear the child task list (in preparation for building the hierarchy).
    ///
    /// Corresponds to Java: `public void clearChildTasks()`
    ///
/// Acts only when the list is non-empty, to ensure the `summary` flag is not overwritten in the "WBS entry with no child activities" case.
    pub fn clear_child_tasks(this: &Rc<RefCell<Task>>) {
        if this.borrow().children.is_empty() {
            return;
        }

        this.borrow_mut().children.clear();
        this.borrow_mut().set_summary(false);
    }

/// Get the parent task (upgrades the weak reference; returns `None` when there is no parent task or the parent task has been released).
    ///
    /// Corresponds to Java: `public Task getParentTask()`
    pub fn get_parent_task(&self) -> Option<Rc<RefCell<Task>>> {
        self.parent.upgrade()
    }

/// Get the child task list (returns a cloned snapshot of the shared handles).
    ///
    /// Corresponds to Java: `@Override public List<Task> getChildTasks()`
    pub fn get_child_tasks(&self) -> Vec<Rc<RefCell<Task>>> {
        self.children.clone()
    }

/// Determine whether the task has child tasks.
    ///
    /// Corresponds to Java: `public boolean hasChildTasks()`
    pub fn has_child_tasks(&self) -> bool {
        !self.children.is_empty()
    }

/// Remove this task from the project file.
    ///
    /// Corresponds to Java: `public void remove()`
    pub fn remove(this: &Rc<RefCell<Task>>) {
        if let Some(file) = this.borrow().parent_file.upgrade() {
            file.borrow_mut().remove_task(this);
        }
    }
}

// ---------------------------------------------------------------------------
// Resource assignments and relations (Relation)
// ---------------------------------------------------------------------------

impl Task {
/// Add a resource assignment to the current task.
    ///
    /// Corresponds to Java: `public ResourceAssignment addResourceAssignment(Resource resource)`
    pub fn add_resource_assignment(
        &mut self,
        resource: Option<&Rc<RefCell<Resource>>>,
    ) -> Option<Rc<RefCell<ResourceAssignment>>> {
        let file = self.parent_file.upgrade()?;

        let assignment = Rc::new(RefCell::new(ResourceAssignment::with_parent_file(&file)));
        {
            let mut assignment_ref = assignment.borrow_mut();
            assignment_ref.set_task_unique_id(self.get_unique_id());
            assignment_ref.set_resource_unique_id(
                resource.and_then(|resource| resource.borrow().get_unique_id()),
            );
            assignment_ref.set_work(self.get_duration());
            // Java: assignment.setUnits(ResourceAssignment.DEFAULT_UNITS)
// (`crate::resource_assignment` does not yet provide that constant, so its value 1.0 is used directly here)
            assignment_ref.set_units(Some(1.0));
        }

        file.borrow_mut()
            .get_resource_assignments_mut()
            .add(Rc::clone(&assignment));

        Some(assignment)
    }

/// Get an existing resource assignment to avoid adding a duplicate.
    ///
    /// Corresponds to Java: `public ResourceAssignment getExistingResourceAssignment(Resource resource)`
    pub fn get_existing_resource_assignment(
        &self,
        resource: Option<&Rc<RefCell<Resource>>>,
    ) -> Option<Rc<RefCell<ResourceAssignment>>> {
        let resource_unique_id = resource.and_then(|resource| resource.borrow().get_unique_id());

        self.get_resource_assignments().into_iter().find(|assignment| {
            let assignment = assignment.borrow();
            match (resource, assignment.get_resource()) {
                (None, None) => true,
                (Some(_), Some(_)) => {
                    resource_unique_id == assignment.get_resource_unique_id()
                }
                _ => false,
            }
        })
    }

/// Get all resource assignments of the current task.
    ///
    /// Corresponds to Java: `public List<ResourceAssignment> getResourceAssignments()`
    pub fn get_resource_assignments(&self) -> Vec<Rc<RefCell<ResourceAssignment>>> {
        match self.parent_file.upgrade() {
            Some(file) => file
                .borrow()
                .get_resource_assignments()
                .get_by_task_unique_id(self.get_unique_id()),
            None => Vec::new(),
        }
    }

/// Add a predecessor relation to the current task.
    ///
    /// Corresponds to Java: `public Relation addPredecessor(Relation.Builder builder)`
    ///
/// Java uses the implicit `this` inside the method body (`builder.successorTask(this)`); the Rust side
/// needs the entity's own shared handle, so it is changed to an associated function taking `this` as the first argument.
    pub fn add_predecessor(
        this: &Rc<RefCell<Task>>,
        builder: RelationBuilder,
    ) -> Option<Rc<RefCell<Relation>>> {
        let file = this.borrow().parent_file.upgrade()?;
        let builder = builder.successor_task(this);
        Some(file.borrow_mut().get_relations_mut().add_predecessor(builder))
    }

/// Delete the predecessor relation that exactly matches the given target task, relation type, and lag time.
    ///
    /// Corresponds to Java: `public boolean removePredecessor(Task targetTask, RelationType type, Duration lag)`
    pub fn remove_predecessor(
        &self,
        target_task: Option<&Rc<RefCell<Task>>>,
        relation_type: RelationType,
        lag: Option<Duration>,
    ) -> bool {
        let Some(file) = self.parent_file.upgrade() else {
            return false;
        };

        let Some(target_task) = target_task else {
            return false;
        };
        let target_task = target_task.borrow();

        file.borrow_mut().get_relations_mut().remove_predecessor(
            self,
            &target_task,
            relation_type,
            lag,
        )
    }

/// Get the predecessor relation list of the current task.
    ///
    /// Corresponds to Java: `@SuppressWarnings("unchecked") public List<Relation> getPredecessors()`
    ///
/// Java evaluates it via the field table `TaskField.PREDECESSORS` by calling `calculatePredecessors()`
/// (this field belongs to the "always calculated" set and is not written back to the cache); the Rust side queries the relation container directly,
/// to work around the field table lacking a `List<Relation>` variant (see item 7 in the file header differences).
    pub fn get_predecessors(&self) -> Vec<Rc<RefCell<Relation>>> {
        self.calculate_predecessors()
    }

/// Get the successor relation list of the current task.
    ///
    /// Corresponds to Java: `@SuppressWarnings("unchecked") public List<Relation> getSuccessors()`
    pub fn get_successors(&self) -> Vec<Rc<RefCell<Relation>>> {
        self.calculate_successors()
    }

/// Compute the predecessor relation list.
    ///
    /// Corresponds to Java: `private List<Relation> calculatePredecessors()`
    fn calculate_predecessors(&self) -> Vec<Rc<RefCell<Relation>>> {
        match self.parent_file.upgrade() {
            Some(file) => file.borrow().get_relations().get_predecessors(self),
            None => Vec::new(),
        }
    }

/// Compute the successor relation list.
    ///
    /// Corresponds to Java: `private List<Relation> calculateSuccessors()`
    fn calculate_successors(&self) -> Vec<Rc<RefCell<Relation>>> {
        match self.parent_file.upgrade() {
            Some(file) => file.borrow().get_relations().get_successors(self),
            None => Vec::new(),
        }
    }

/// Determine whether the given task is a predecessor of the current task.
    ///
    /// Corresponds to Java: `public boolean isPredecessor(Task task)`
    pub fn is_predecessor(&self, task: Option<&Rc<RefCell<Task>>>) -> bool {
        let Some(task) = task else {
            return false;
        };

        let unique_id = task.borrow().get_unique_id();
        self.get_predecessors().iter().any(|relation| {
            relation
                .borrow()
                .get_predecessor_task()
                .map(|task| task.borrow().get_unique_id())
                == Some(unique_id)
        })
    }

/// Determine whether the given task is a successor of the current task.
    ///
    /// Corresponds to Java: `public boolean isSuccessor(Task task)`
    ///
/// Java's implementation is `task.isPredecessor(this)`; the Rust side instead looks it up
/// in reverse in this task's successor list (semantically equivalent, and it does not need the entity's own shared handle).
    pub fn is_successor(&self, task: Option<&Rc<RefCell<Task>>>) -> bool {
        let Some(task) = task else {
            return false;
        };

        let unique_id = task.borrow().get_unique_id();
        self.get_successors().iter().any(|relation| {
            relation
                .borrow()
                .get_successor_task()
                .map(|task| task.borrow().get_unique_id())
                == Some(unique_id)
        })
    }
}

// ---------------------------------------------------------------------------
// Basic properties: percentage, cost, duration, dates
// ---------------------------------------------------------------------------

impl Task {
/// Set the percentage complete.
    ///
    /// Corresponds to Java: `public void setPercentageComplete(Number val)`
    pub fn set_percentage_complete(&mut self, val: Option<f64>) {
        self.put_number(TaskField::PercentComplete, val);
    }

/// Set the percentage work complete.
    ///
    /// Corresponds to Java: `public void setPercentageWorkComplete(Number val)`
    pub fn set_percentage_work_complete(&mut self, val: Option<f64>) {
        self.put_number(TaskField::PercentWorkComplete, val);
    }

/// Set the actual cost.
    ///
    /// Corresponds to Java: `public void setActualCost(Number val)`
    pub fn set_actual_cost(&mut self, val: Option<f64>) {
        self.put_number(TaskField::ActualCost, val);
    }

/// Set the actual duration.
    ///
    /// Corresponds to Java: `public void setActualDuration(Duration val)`
    pub fn set_actual_duration(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::ActualDuration, val);
    }

/// Set the actual finish date.
    ///
    /// Corresponds to Java: `public void setActualFinish(LocalDateTime val)`
    pub fn set_actual_finish(&mut self, val: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::ActualFinish, val);
    }

/// Set the actual start date.
    ///
    /// Corresponds to Java: `public void setActualStart(LocalDateTime val)`
    pub fn set_actual_start(&mut self, val: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::ActualStart, val);
    }

/// Set the actual work.
    ///
    /// Corresponds to Java: `public void setActualWork(Duration val)`
    pub fn set_actual_work(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::ActualWork, val);
    }

/// Set the baseline cost.
    ///
    /// Corresponds to Java: `public void setBaselineCost(Number val)`
    pub fn set_baseline_cost(&mut self, val: Option<f64>) {
        self.put_number(TaskField::BaselineCost, val);
    }

/// Set the baseline duration.
    ///
    /// Corresponds to Java: `public void setBaselineDuration(Duration val)`
    pub fn set_baseline_duration(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::BaselineDuration, val);
    }

/// Set the baseline finish date.
    ///
    /// Corresponds to Java: `public void setBaselineFinish(LocalDateTime val)`
    pub fn set_baseline_finish(&mut self, val: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::BaselineFinish, val);
    }

/// Set the baseline start date.
    ///
    /// Corresponds to Java: `public void setBaselineStart(LocalDateTime val)`
    pub fn set_baseline_start(&mut self, val: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::BaselineStart, val);
    }

/// Set the baseline work.
    ///
    /// Corresponds to Java: `public void setBaselineWork(Duration val)`
    pub fn set_baseline_work(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::BaselineWork, val);
    }

/// Set BCWP (budgeted cost of work performed).
    ///
    /// Corresponds to Java: `public void setBCWP(Number val)`
    pub fn set_bcwp(&mut self, val: Option<f64>) {
        self.put_number(TaskField::Bcwp, val);
    }

/// Set BCWS (budgeted cost of work scheduled).
    ///
    /// Corresponds to Java: `public void setBCWS(Number val)`
    pub fn set_bcws(&mut self, val: Option<f64>) {
        self.put_number(TaskField::Bcws, val);
    }

/// Set the "confirmed" flag.
    ///
    /// Corresponds to Java: `public void setConfirmed(boolean val)`
    pub fn set_confirmed(&mut self, val: bool) {
        self.put_bool(TaskField::Confirmed, val);
    }

/// Set the constraint date.
    ///
    /// Corresponds to Java: `public void setConstraintDate(LocalDateTime val)`
    pub fn set_constraint_date(&mut self, val: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::ConstraintDate, val);
    }

/// Set the secondary constraint date.
    ///
    /// Corresponds to Java: `public void setSecondaryConstraintDate(LocalDateTime date)`
    pub fn set_secondary_constraint_date(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::SecondaryConstraintDate, date);
    }

/// Set the constraint type.
    ///
    /// Corresponds to Java: `public void setConstraintType(ConstraintType type)`
    pub fn set_constraint_type(&mut self, r#type: Option<ConstraintType>) {
        self.put_enum(TaskField::ConstraintType, r#type.map(|value| value.value()));
    }

/// Set the secondary constraint type.
    ///
    /// Corresponds to Java: `public void setSecondaryConstraintType(ConstraintType type)`
    pub fn set_secondary_constraint_type(&mut self, r#type: Option<ConstraintType>) {
        self.put_enum(TaskField::SecondaryConstraintType, r#type.map(|value| value.value()));
    }

/// Set the contact.
    ///
    /// Corresponds to Java: `public void setContact(String val)`
    pub fn set_contact(&mut self, val: Option<String>) {
        self.put_string(TaskField::Contact, val);
    }

/// Set the total cost.
    ///
    /// Corresponds to Java: `public void setCost(Number val)`
    pub fn set_cost(&mut self, val: Option<f64>) {
        self.put_number(TaskField::Cost, val);
    }

/// Set a custom cost field.
    ///
    /// Corresponds to Java: `public void setCost(int index, Number value)`
    pub fn set_cost_at(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = select_field(custom_cost_fields(), index) {
            self.put_number(field, value);
        }
    }

/// Read a custom cost field.
    ///
    /// Corresponds to Java: `public Number getCost(int index)`
    pub fn get_cost_at(&self, index: i32) -> Option<f64> {
        select_field(custom_cost_fields(), index).and_then(|field| self.number_field(field))
    }

/// Set the cost variance.
    ///
    /// Corresponds to Java: `public void setCostVariance(Number val)`
    pub fn set_cost_variance(&mut self, val: Option<f64>) {
        self.put_number(TaskField::CostVariance, val);
    }

/// Set the created date.
    ///
    /// Corresponds to Java: `public void setCreateDate(LocalDateTime val)`
    pub fn set_create_date(&mut self, val: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::Created, val);
    }

/// Set the critical path flag.
    ///
    /// Corresponds to Java: `public void setCritical(boolean val)`
    pub fn set_critical(&mut self, val: bool) {
        self.put_bool(TaskField::Critical, val);
    }

/// Set the CV (earned value cost variance).
    ///
    /// Corresponds to Java: `public void setCV(Number val)`
    pub fn set_cv(&mut self, val: Option<f64>) {
        self.put_number(TaskField::Cv, val);
    }

/// Set the leveling delay.
    ///
    /// Corresponds to Java: `public void setLevelingDelay(Duration val)`
    pub fn set_leveling_delay(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::LevelingDelay, val);
    }

/// Set the duration.
    ///
    /// Corresponds to Java: `public void setDuration(Duration val)`
    pub fn set_duration(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::Duration, val);
    }

/// Set the duration text for a manually scheduled task.
    ///
    /// Corresponds to Java: `public void setDurationText(String val)`
    pub fn set_duration_text(&mut self, val: Option<String>) {
        self.put_string(TaskField::DurationText, val);
    }

/// Set the manual duration attribute.
    ///
    /// Corresponds to Java: `public void setManualDuration(Duration dur)`
    pub fn set_manual_duration(&mut self, dur: Option<Duration>) {
        self.put_duration(TaskField::ManualDuration, dur);
    }

/// Read the manual duration attribute.
    ///
    /// Corresponds to Java: `public Duration getManualDuration()`
    pub fn get_manual_duration(&self) -> Option<Duration> {
        self.duration_field(TaskField::ManualDuration)
    }

/// Set the duration variance.
    ///
    /// Corresponds to Java: `public void setDurationVariance(Duration duration)`
    pub fn set_duration_variance(&mut self, duration: Option<Duration>) {
        self.put_duration(TaskField::DurationVariance, duration);
    }

/// Set the early finish date.
    ///
    /// Corresponds to Java: `public void setEarlyFinish(LocalDateTime date)`
    pub fn set_early_finish(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::EarlyFinish, date);
    }

/// Set the remaining early finish date.
    ///
    /// Corresponds to Java: `public void setRemainingEarlyFinish(LocalDateTime date)`
    pub fn set_remaining_early_finish(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::RemainingEarlyFinish, date);
    }

/// Set the early start date.
    ///
    /// Corresponds to Java: `public void setEarlyStart(LocalDateTime date)`
    pub fn set_early_start(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::EarlyStart, date);
    }

/// Set the remaining early start date.
    ///
    /// Corresponds to Java: `public void setRemainingEarlyStart(LocalDateTime date)`
    pub fn set_remaining_early_start(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::RemainingEarlyStart, date);
    }

/// Set the finish date.
    ///
    /// Corresponds to Java: `public void setFinish(LocalDateTime date)`
    pub fn set_finish(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::Finish, date);
    }

/// Set the finish text for a manually scheduled task.
    ///
    /// Corresponds to Java: `public void setFinishText(String val)`
    pub fn set_finish_text(&mut self, val: Option<String>) {
        self.put_string(TaskField::FinishText, val);
    }

/// Set the finish variance.
    ///
    /// Corresponds to Java: `public void setFinishVariance(Duration duration)`
    pub fn set_finish_variance(&mut self, duration: Option<Duration>) {
        self.put_duration(TaskField::FinishVariance, duration);
    }

/// Set the fixed cost.
    ///
    /// Corresponds to Java: `public void setFixedCost(Number val)`
    pub fn set_fixed_cost(&mut self, val: Option<f64>) {
        self.put_number(TaskField::FixedCost, val);
    }

/// Set the free slack.
    ///
    /// Corresponds to Java: `public void setFreeSlack(Duration duration)`
    pub fn set_free_slack(&mut self, duration: Option<Duration>) {
        self.put_duration(TaskField::FreeSlack, duration);
    }

/// Set the "hide Gantt bar" flag.
    ///
    /// Corresponds to Java: `public void setHideBar(boolean flag)`
    pub fn set_hide_bar(&mut self, flag: bool) {
        self.put_bool(TaskField::HideBar, flag);
    }

/// Set the task ID.
    ///
    /// Corresponds to Java: `@Override public void setID(Integer val)`
    ///
/// Maintain the ID index of [`crate::TaskContainer`] consistently with Java: first `unmap_id` the old ID,
/// then `map_id` the new ID. This callback needs the task's own shared reference, which is wired up by [`Task::set_self_reference`]
/// when the task is added to its container; if the task is not yet wired up, or the parent file is currently borrowed, fall back to "not maintaining the index" instead of panicking.
    pub fn set_id(&mut self, val: Option<i32>) {
        // Java:
        //   Integer previous = getID();
        //   if (previous != null) { m_parentFile.getTasks().unmapID(previous); }
        //   m_parentFile.getTasks().mapID(val, this);
        //   set(TaskField.ID, val);
        let previous = self.get_id();
        if let Some(task) = self.self_ref.upgrade() {
            if let Some(file) = self.get_parent_file() {
// The caller may currently hold a borrow of `ProjectFile` (for example, the container changing IDs in bulk inside one of its own methods),
// in which case fall back to "not maintaining the ID index" instead of panicking.
                if let Ok(mut file_ref) = file.try_borrow_mut() {
                    let tasks = file_ref.get_tasks_mut();
                    if let Some(previous) = previous {
                        tasks.unmap_id(Some(previous));
                    }
                    if let Some(val) = val {
                        tasks.map_id(val, Rc::clone(&task));
                    }
                }
            }
        }

        self.put_integer(TaskField::Id, val);
    }

/// Set the late finish date.
    ///
    /// Corresponds to Java: `public void setLateFinish(LocalDateTime date)`
    pub fn set_late_finish(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::LateFinish, date);
    }

/// Set the remaining late finish date.
    ///
    /// Corresponds to Java: `public void setRemainingLateFinish(LocalDateTime date)`
    pub fn set_remaining_late_finish(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::RemainingLateFinish, date);
    }

/// Set the late start date.
    ///
    /// Corresponds to Java: `public void setLateStart(LocalDateTime date)`
    pub fn set_late_start(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::LateStart, date);
    }

/// Set the remaining late start date.
    ///
    /// Corresponds to Java: `public void setRemainingLateStart(LocalDateTime date)`
    pub fn set_remaining_late_start(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::RemainingLateStart, date);
    }

/// Set the "OLE link present" flag.
    ///
    /// Corresponds to Java: `public void setLinkedFields(boolean flag)`
    pub fn set_linked_fields(&mut self, flag: bool) {
        self.put_bool(TaskField::LinkedFields, flag);
    }

/// Set the "marked" flag.
    ///
    /// Corresponds to Java: `public void setMarked(boolean flag)`
    pub fn set_marked(&mut self, flag: bool) {
        self.put_bool(TaskField::Marked, flag);
    }

/// Set the milestone flag.
    ///
    /// Corresponds to Java: `public void setMilestone(boolean flag)`
    pub fn set_milestone(&mut self, flag: bool) {
        self.put_bool(TaskField::Milestone, flag);
    }

/// Set the task name.
    ///
    /// Corresponds to Java: `public void setName(String name)`
    pub fn set_name(&mut self, name: Option<String>) {
        self.put_string(TaskField::Name, name);
    }

/// Set the number of attached objects.
    ///
    /// Corresponds to Java: `public void setObjects(Integer val)`
    pub fn set_objects(&mut self, val: Option<i32>) {
        self.put_integer(TaskField::Objects, val);
    }

/// Set the outline level.
    ///
    /// Corresponds to Java: `public void setOutlineLevel(Integer val)`
    pub fn set_outline_level(&mut self, val: Option<i32>) {
        self.put_integer(TaskField::OutlineLevel, val);
    }

/// Set the outline number.
    ///
    /// Corresponds to Java: `public void setOutlineNumber(String val)`
    pub fn set_outline_number(&mut self, val: Option<String>) {
        self.put_string(TaskField::OutlineNumber, val);
    }

/// Set the priority.
    ///
    /// Corresponds to Java: `public void setPriority(Priority priority)`
    pub fn set_priority(&mut self, priority: Option<Priority>) {
        self.put_enum(
            TaskField::Priority,
            priority.map(|priority| priority.get_value()),
        );
    }

/// Set the source project name.
    ///
    /// Corresponds to Java: `public void setProject(String val)`
    pub fn set_project(&mut self, val: Option<String>) {
        self.put_string(TaskField::Project, val);
    }

/// Set the remaining cost.
    ///
    /// Corresponds to Java: `public void setRemainingCost(Number val)`
    pub fn set_remaining_cost(&mut self, val: Option<f64>) {
        self.put_number(TaskField::RemainingCost, val);
    }

/// Set the remaining duration.
    ///
    /// Corresponds to Java: `public void setRemainingDuration(Duration val)`
    pub fn set_remaining_duration(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::RemainingDuration, val);
    }

/// Set the remaining work.
    ///
    /// Corresponds to Java: `public void setRemainingWork(Duration val)`
    pub fn set_remaining_work(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::RemainingWork, val);
    }

/// Set the resource group.
    ///
    /// Corresponds to Java: `public void setResourceGroup(String val)`
    pub fn set_resource_group(&mut self, val: Option<String>) {
        self.put_string(TaskField::ResourceGroup, val);
    }

/// Set the resource initials.
    ///
    /// Corresponds to Java: `public void setResourceInitials(String val)`
    pub fn set_resource_initials(&mut self, val: Option<String>) {
        self.put_string(TaskField::ResourceInitials, val);
    }

/// Set the resource names.
    ///
    /// Corresponds to Java: `public void setResourceNames(String val)`
    pub fn set_resource_names(&mut self, val: Option<String>) {
        self.put_string(TaskField::ResourceNames, val);
    }

/// Set the resume date.
    ///
    /// Corresponds to Java: `public void setResume(LocalDateTime val)`
    pub fn set_resume(&mut self, val: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::Resume, val);
    }

/// Set the rollup flag.
    ///
    /// Corresponds to Java: `public void setRollup(boolean val)`
    pub fn set_rollup(&mut self, val: bool) {
        self.put_bool(TaskField::Rollup, val);
    }

/// Set the start date.
    ///
    /// Corresponds to Java: `public void setStart(LocalDateTime val)`
    pub fn set_start(&mut self, val: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::Start, val);
    }

/// Set the start text for a manually scheduled task.
    ///
    /// Corresponds to Java: `public void setStartText(String val)`
    pub fn set_start_text(&mut self, val: Option<String>) {
        self.put_string(TaskField::StartText, val);
    }

/// Set the start variance.
    ///
    /// Corresponds to Java: `public void setStartVariance(Duration val)`
    pub fn set_start_variance(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::StartVariance, val);
    }

/// Set the stop date.
    ///
    /// Corresponds to Java: `public void setStop(LocalDateTime val)`
    pub fn set_stop(&mut self, val: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::Stop, val);
    }

/// Set the subproject file path.
    ///
    /// Corresponds to Java: `public void setSubprojectFile(String val)`
    pub fn set_subproject_file(&mut self, val: Option<String>) {
        self.put_string(TaskField::SubprojectFile, val);
    }

/// Set the GUID of the linked subproject file.
    ///
    /// Corresponds to Java: `public void setSubprojectGUID(UUID guid)`
    pub fn set_subproject_guid(&mut self, guid: Option<Uuid>) {
        self.put_uuid(TaskField::SubprojectGuid, guid);
    }

/// Get the GUID of the linked subproject file.
    ///
    /// Corresponds to Java: `public UUID getSubprojectGUID()`
    pub fn get_subproject_guid(&self) -> Option<Uuid> {
        self.uuid_field(TaskField::SubprojectGuid)
    }

/// Set the summary task flag.
    ///
    /// Corresponds to Java: `public void setSummary(boolean val)`
    pub fn set_summary(&mut self, val: bool) {
        self.put_bool(TaskField::Summary, val);
    }

/// Set the SV (earned value schedule variance).
    ///
    /// Corresponds to Java: `public void setSV(Number val)`
    pub fn set_sv(&mut self, val: Option<f64>) {
        self.put_number(TaskField::Sv, val);
    }

/// Set the total slack.
    ///
    /// Corresponds to Java: `public void setTotalSlack(Duration val)`
    pub fn set_total_slack(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::TotalSlack, val);
    }

/// Set the unique ID.
    ///
    /// Corresponds to Java: `@Override public void setUniqueID(Integer val)`
    ///
/// Java synchronizes the `TaskContainer` unique ID index during field change handling,
/// while the Rust side leaves this to the container (see item 3 in the file header notes).
    pub fn set_unique_id(&mut self, val: Option<i32>) {
        self.put_integer(TaskField::UniqueId, val);
    }

/// Set the "update needed" flag.
    ///
    /// Corresponds to Java: `public void setUpdateNeeded(boolean val)`
    pub fn set_update_needed(&mut self, val: bool) {
        self.put_bool(TaskField::UpdateNeeded, val);
    }

/// Set the WBS code.
    ///
    /// Corresponds to Java: `public void setWBS(String val)`
    pub fn set_wbs(&mut self, val: Option<String>) {
        self.put_string(TaskField::Wbs, val);
    }

/// Set the work.
    ///
    /// Corresponds to Java: `public void setWork(Duration val)`
    pub fn set_work(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::Work, val);
    }

/// Set the work variance.
    ///
    /// Corresponds to Java: `public void setWorkVariance(Duration val)`
    pub fn set_work_variance(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::WorkVariance, val);
    }
}

// ---------------------------------------------------------------------------
// Basic property getters
// ---------------------------------------------------------------------------

impl Task {
/// Get the percentage complete.
    ///
    /// Corresponds to Java: `public Number getPercentageComplete()`
    pub fn get_percentage_complete(&self) -> Option<f64> {
        self.number_field(TaskField::PercentComplete)
    }

/// Get the percentage work complete.
    ///
    /// Corresponds to Java: `public Number getPercentageWorkComplete()`
    pub fn get_percentage_work_complete(&self) -> Option<f64> {
        self.number_field(TaskField::PercentWorkComplete)
    }

/// Get the actual cost.
    ///
    /// Corresponds to Java: `public Number getActualCost()`
    pub fn get_actual_cost(&self) -> Option<f64> {
        self.number_field(TaskField::ActualCost)
    }

/// Get the actual duration.
    ///
    /// Corresponds to Java: `public Duration getActualDuration()`
    pub fn get_actual_duration(&self) -> Option<Duration> {
        self.duration_field(TaskField::ActualDuration)
    }

/// Get the actual finish date.
    ///
    /// Corresponds to Java: `public LocalDateTime getActualFinish()`
    pub fn get_actual_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::ActualFinish)
    }

/// Get the actual start date.
    ///
    /// Corresponds to Java: `public LocalDateTime getActualStart()`
    pub fn get_actual_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::ActualStart)
    }

/// Get the actual work.
    ///
    /// Corresponds to Java: `public Duration getActualWork()`
    pub fn get_actual_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::ActualWork)
    }

/// Get the baseline cost.
    ///
    /// Corresponds to Java: `public Number getBaselineCost()`
    pub fn get_baseline_cost(&self) -> Option<f64> {
        self.number_field(TaskField::BaselineCost)
    }

/// Get the baseline duration.
    ///
    /// Corresponds to Java: `public Duration getBaselineDuration()`
    pub fn get_baseline_duration(&self) -> Option<Duration> {
        self.duration_field(TaskField::BaselineDuration)
    }

/// Get the baseline duration text.
    ///
    /// Corresponds to Java: `public String getBaselineDurationText()`
    pub fn get_baseline_duration_text(&self) -> Option<String> {
        self.string_field(TaskField::BaselineDuration)
    }

/// Set the baseline duration text.
    ///
    /// Corresponds to Java: `public void setBaselineDurationText(String value)`
    pub fn set_baseline_duration_text(&mut self, value: Option<String>) {
        self.put_string(TaskField::BaselineDuration, value);
    }

/// Get the baseline finish date.
    ///
    /// Corresponds to Java: `public LocalDateTime getBaselineFinish()`
    pub fn get_baseline_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::BaselineFinish)
    }

/// Get the baseline finish text.
    ///
    /// Corresponds to Java: `public String getBaselineFinishText()`
    pub fn get_baseline_finish_text(&self) -> Option<String> {
        self.string_field(TaskField::BaselineFinish)
    }

/// Set the baseline finish text.
    ///
    /// Corresponds to Java: `public void setBaselineFinishText(String value)`
    pub fn set_baseline_finish_text(&mut self, value: Option<String>) {
        self.put_string(TaskField::BaselineFinish, value);
    }

/// Get the baseline start date.
    ///
    /// Corresponds to Java: `public LocalDateTime getBaselineStart()`
    pub fn get_baseline_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::BaselineStart)
    }

/// Get the baseline start text.
    ///
    /// Corresponds to Java: `public String getBaselineStartText()`
    pub fn get_baseline_start_text(&self) -> Option<String> {
        self.string_field(TaskField::BaselineStart)
    }

/// Set the baseline start text.
    ///
    /// Corresponds to Java: `public void setBaselineStartText(String value)`
    pub fn set_baseline_start_text(&mut self, value: Option<String>) {
        self.put_string(TaskField::BaselineStart, value);
    }

/// Get the baseline work.
    ///
    /// Corresponds to Java: `public Duration getBaselineWork()`
    pub fn get_baseline_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::BaselineWork)
    }

/// Get the BCWP.
    ///
    /// Corresponds to Java: `public Number getBCWP()`
    pub fn get_bcwp(&self) -> Option<f64> {
        self.number_field(TaskField::Bcwp)
    }

/// Get the BCWS.
    ///
    /// Corresponds to Java: `public Number getBCWS()`
    pub fn get_bcws(&self) -> Option<f64> {
        self.number_field(TaskField::Bcws)
    }

/// Get the "confirmed" flag.
    ///
    /// Corresponds to Java: `public boolean getConfirmed()`
    pub fn get_confirmed(&self) -> bool {
        self.boolean_field(TaskField::Confirmed)
    }

/// Get the constraint date.
    ///
    /// Corresponds to Java: `public LocalDateTime getConstraintDate()`
    pub fn get_constraint_date(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::ConstraintDate)
    }

/// Get the secondary constraint date.
    ///
    /// Corresponds to Java: `public LocalDateTime getSecondaryConstraintDate()`
    pub fn get_secondary_constraint_date(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::SecondaryConstraintDate)
    }

/// Get the constraint type.
    ///
    /// Corresponds to Java: `public ConstraintType getConstraintType()`
    pub fn get_constraint_type(&self) -> Option<ConstraintType> {
        self.integer_field(TaskField::ConstraintType)
            .map(constraint_type_from_value)
    }

/// Get the secondary constraint type.
    ///
    /// Corresponds to Java: `public ConstraintType getSecondaryConstraintType()`
    pub fn get_secondary_constraint_type(&self) -> Option<ConstraintType> {
        self.integer_field(TaskField::SecondaryConstraintType)
            .map(constraint_type_from_value)
    }

/// Get the contact.
    ///
    /// Corresponds to Java: `public String getContact()`
    pub fn get_contact(&self) -> Option<String> {
        self.string_field(TaskField::Contact)
    }

/// Get the total cost.
    ///
    /// Corresponds to Java: `public Number getCost()`
    pub fn get_cost(&self) -> Option<f64> {
        self.number_field(TaskField::Cost)
    }

/// Get the cost variance.
    ///
    /// Corresponds to Java: `public Number getCostVariance()`
    pub fn get_cost_variance(&self) -> Option<f64> {
        self.number_field(TaskField::CostVariance)
    }

/// Get the created date.
    ///
    /// Corresponds to Java: `public LocalDateTime getCreateDate()`
    pub fn get_create_date(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::Created)
    }

/// Get the critical path flag.
    ///
    /// Corresponds to Java: `public boolean getCritical()`
    pub fn get_critical(&self) -> bool {
        self.boolean_field(TaskField::Critical)
    }

/// Get the CV.
    ///
    /// Corresponds to Java: `public Number getCV()`
    pub fn get_cv(&self) -> Option<f64> {
        self.number_field(TaskField::Cv)
    }

/// Get the leveling delay.
    ///
    /// Corresponds to Java: `public Duration getLevelingDelay()`
    pub fn get_leveling_delay(&self) -> Option<Duration> {
        self.duration_field(TaskField::LevelingDelay)
    }

/// Get the duration.
    ///
    /// Corresponds to Java: `public Duration getDuration()`
    pub fn get_duration(&self) -> Option<Duration> {
        self.duration_field(TaskField::Duration)
    }

/// Get the duration text for a manually scheduled task.
    ///
    /// Corresponds to Java: `public String getDurationText()`
    pub fn get_duration_text(&self) -> Option<String> {
        self.string_field(TaskField::DurationText)
    }

/// Set a custom duration field.
    ///
    /// Corresponds to Java: `public void setDuration(int index, Duration value)`
    pub fn set_duration_at(&mut self, index: i32, value: Option<Duration>) {
        if let Some(field) = select_field(custom_duration_fields(), index) {
            self.put_duration(field, value);
        }
    }

/// Read a custom duration field.
    ///
    /// Corresponds to Java: `public Duration getDuration(int index)`
    pub fn get_duration_at(&self, index: i32) -> Option<Duration> {
        select_field(custom_duration_fields(), index).and_then(|field| self.duration_field(field))
    }

/// Get the duration variance.
    ///
    /// Corresponds to Java: `public Duration getDurationVariance()`
    pub fn get_duration_variance(&self) -> Option<Duration> {
        self.duration_field(TaskField::DurationVariance)
    }

/// Get the early finish date.
    ///
    /// Corresponds to Java: `public LocalDateTime getEarlyFinish()`
    pub fn get_early_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::EarlyFinish)
    }

/// Get the remaining early finish date.
    ///
    /// Corresponds to Java: `public LocalDateTime getRemainingEarlyFinish()`
    pub fn get_remaining_early_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::RemainingEarlyFinish)
    }

/// Get the early start date.
    ///
    /// Corresponds to Java: `public LocalDateTime getEarlyStart()`
    pub fn get_early_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::EarlyStart)
    }

/// Get the remaining early start date.
    ///
    /// Corresponds to Java: `public LocalDateTime getRemainingEarlyStart()`
    pub fn get_remaining_early_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::RemainingEarlyStart)
    }

/// Get the finish date.
    ///
    /// Corresponds to Java: `public LocalDateTime getFinish()`
    pub fn get_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::Finish)
    }

/// Get the finish text for a manually scheduled task.
    ///
    /// Corresponds to Java: `public String getFinishText()`
    pub fn get_finish_text(&self) -> Option<String> {
        self.string_field(TaskField::FinishText)
    }

/// Set a custom finish date field.
    ///
    /// Corresponds to Java: `public void setFinish(int index, LocalDateTime value)`
    pub fn set_finish_at(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(custom_finish_fields(), index) {
            self.put_date_time(field, value);
        }
    }

/// Read a custom finish date field.
    ///
    /// Corresponds to Java: `public LocalDateTime getFinish(int index)`
    pub fn get_finish_at(&self, index: i32) -> Option<NaiveDateTime> {
        select_field(custom_finish_fields(), index).and_then(|field| self.date_time_field(field))
    }

/// Get the finish variance.
    ///
    /// Corresponds to Java: `public Duration getFinishVariance()`
    pub fn get_finish_variance(&self) -> Option<Duration> {
        self.duration_field(TaskField::FinishVariance)
    }

/// Get the fixed cost.
    ///
    /// Corresponds to Java: `public Number getFixedCost()`
    pub fn get_fixed_cost(&self) -> Option<f64> {
        self.number_field(TaskField::FixedCost)
    }

/// Set a custom flag field.
    ///
    /// Corresponds to Java: `public void setFlag(int index, boolean value)`
    pub fn set_flag(&mut self, index: i32, value: bool) {
        if let Some(field) = select_field(custom_flag_fields(), index) {
            self.put_bool(field, value);
        }
    }

/// Read a custom flag field.
    ///
    /// Corresponds to Java: `public boolean getFlag(int index)`
    pub fn get_flag(&self, index: i32) -> bool {
        select_field(custom_flag_fields(), index).map_or(false, |field| self.boolean_field(field))
    }

/// Get the free slack.
    ///
    /// Corresponds to Java: `public Duration getFreeSlack()`
    pub fn get_free_slack(&self) -> Option<Duration> {
        self.duration_field(TaskField::FreeSlack)
    }

/// Get the "hide Gantt bar" flag.
    ///
    /// Corresponds to Java: `public boolean getHideBar()`
    pub fn get_hide_bar(&self) -> bool {
        self.boolean_field(TaskField::HideBar)
    }

/// Get the task ID.
    ///
    /// Corresponds to Java: `@Override public Integer getID()`
    pub fn get_id(&self) -> Option<i32> {
        self.integer_field(TaskField::Id)
    }

/// Get the late finish date.
    ///
    /// Corresponds to Java: `public LocalDateTime getLateFinish()`
    pub fn get_late_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::LateFinish)
    }

/// Get the remaining late finish date.
    ///
    /// Corresponds to Java: `public LocalDateTime getRemainingLateFinish()`
    pub fn get_remaining_late_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::RemainingLateFinish)
    }

/// Get the late start date.
    ///
    /// Corresponds to Java: `public LocalDateTime getLateStart()`
    pub fn get_late_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::LateStart)
    }

/// Get the remaining late start date.
    ///
    /// Corresponds to Java: `public LocalDateTime getRemainingLateStart()`
    pub fn get_remaining_late_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::RemainingLateStart)
    }

/// Get the "OLE link present" flag.
    ///
    /// Corresponds to Java: `public boolean getLinkedFields()`
    pub fn get_linked_fields(&self) -> bool {
        self.boolean_field(TaskField::LinkedFields)
    }

/// Get the "marked" flag.
    ///
    /// Corresponds to Java: `public boolean getMarked()`
    pub fn get_marked(&self) -> bool {
        self.boolean_field(TaskField::Marked)
    }

/// Get the milestone flag.
    ///
    /// Corresponds to Java: `public boolean getMilestone()`
    pub fn get_milestone(&self) -> bool {
        self.boolean_field(TaskField::Milestone)
    }

/// Get the task name.
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> Option<String> {
        self.string_field(TaskField::Name)
    }

/// Get the plain text representation of the task notes (returns an empty string when there are no notes).
    ///
    /// Corresponds to Java: `public String getNotes()`
    ///
    /// Java: `Object notes = get(TaskField.NOTES); return notes == null ? "" : notes.toString();`
    pub fn get_notes(&self) -> String {
        self.notes_object()
            .map(|notes| notes.to_string())
            .unwrap_or_default()
    }

/// Get the `Notes` instance representing the task notes.
    ///
    /// Corresponds to Java: `public Notes getNotesObject()`
    ///
/// The subtypes (`RtfNotes` / `HtmlNotes` / `StructuredNotes` / `ParentNotes`) are
/// preserved faithfully via [`FieldValue::Notes`], so the original concrete type is returned.
    pub fn get_notes_object(&self) -> Option<Rc<dyn Notes>> {
        self.notes_object()
    }

/// Get the notes object from the field table (internal convenience method).
    ///
    /// Corresponds to Java: `(Notes) get(TaskField.NOTES)`
    fn notes_object(&self) -> Option<Rc<dyn Notes>> {
        match self.get_task_field(TaskField::Notes) {
            Some(FieldValue::Notes(notes)) => Some(notes),
            _ => None,
        }
    }

    // -----------------------------------------------------------------------
// Steps (TaskField.STEPS)
    // -----------------------------------------------------------------------

/// Get the step list for this task.
    ///
    /// Corresponds to Java: `@SuppressWarnings("unchecked") public List<Step> getSteps()`
    ///
/// Java returns the **live list** in the field table (when unset, it returns a new empty list created by `defaultSteps()` each time);
/// the Rust-side list is stored in the `Rc<RefCell<...>>` of [`FieldValue::Steps`], and this returns a snapshot of it,
/// so modifications to the returned value are not written back to the task.
    pub fn get_steps(&self) -> Vec<Step> {
        self.steps()
            .and_then(|steps| steps.try_borrow().ok().map(|steps| steps.clone()))
            .unwrap_or_default()
    }

/// Set the step list for this task.
    ///
    /// Corresponds to Java: `public void setSteps(List<Step> steps)`
    pub fn set_steps(&mut self, steps: Vec<Step>) {
        let value = FieldValue::Steps(Rc::new(RefCell::new(steps)));
        self.set_task_field(TaskField::Steps, Some(value));
    }

/// Default value for the step list.
    ///
/// Corresponds to Java: `private List<Step> defaultSteps()` (returns `new ArrayList<>()`)
    fn default_steps() -> Rc<RefCell<Vec<Step>>> {
        Rc::new(RefCell::new(Vec::new()))
    }

/// Get the step list from the field table (shared list, internal convenience method).
    ///
    /// Corresponds to Java: `(List<Step>) get(TaskField.STEPS)`
    fn steps(&self) -> Option<Rc<RefCell<Vec<Step>>>> {
        match self.get_task_field(TaskField::Steps) {
            Some(FieldValue::Steps(steps)) => Some(steps),
            _ => None,
        }
    }

    // -----------------------------------------------------------------------
// Work splits (TaskField.WORK_SPLITS)
    // -----------------------------------------------------------------------

/// Get the work splits within this task's range.
    ///
    /// Corresponds to Java: `@SuppressWarnings("unchecked") public List<LocalDateTimeRange> getWorkSplits()`
    ///
/// This field belongs to Java's `ALWAYS_CALCULATED_FIELDS`, so getting it is equivalent to
/// [`Task::calculate_work_splits`].
    pub fn get_work_splits(&self) -> Vec<LocalDateTimeRange> {
        match self.get_task_field(TaskField::WorkSplits) {
            Some(FieldValue::TimeRanges(ranges)) => ranges,
            _ => Vec::new(),
        }
    }

    /// Merge the work time ranges of each resource assignment.
    ///
    /// Corresponds to Java: `private List<LocalDateTimeRange> calculateWorkSplits()`
    fn calculate_work_splits(&self) -> Vec<LocalDateTimeRange> {
        // In MS Project, summary tasks do not show splits.
        if self.get_summary() {
            return Vec::new();
        }

        let mut result: Option<Vec<LocalDateTimeRange>> = None;
        for assignment in self.get_resource_assignments() {
// Borrow element by element: no borrow of the whole assignment is held during the call (Java has no borrow restrictions).
            let Ok(assignment) = assignment.try_borrow() else {
                continue;
            };

            let splits = assignment.get_work_splits();
            result = Some(match result {
                None => splits,
                Some(previous) => self.reduce_work_splits(previous, splits),
            });
        }

        result.unwrap_or_default()
    }

/// Merge two sets of work splits.
    ///
    /// Corresponds to Java: `private List<LocalDateTimeRange> reduceWorkSplits(List<LocalDateTimeRange> list1, List<LocalDateTimeRange> list2)`
    fn reduce_work_splits(
        &self,
        list1: Vec<LocalDateTimeRange>,
        list2: Vec<LocalDateTimeRange>,
    ) -> Vec<LocalDateTimeRange> {
        if list1 == list2 {
            return list1;
        }

        let mut index1 = 0;
        let mut index2 = 0;
        let mut result: Vec<LocalDateTimeRange> = Vec::new();
        let calendar = self.get_effective_calendar();

        while index1 < list1.len() && index2 < list2.len() {
            let range1 = list1[index1];
            let range2 = list2[index2];

            if range1.is_before(&range2) {
                Self::add_work_split(calendar.as_ref(), &mut result, range1);
                index1 += 1;
                continue;
            }

            if range2.is_before(&range1) {
                Self::add_work_split(calendar.as_ref(), &mut result, range2);
                index2 += 1;
                continue;
            }

            if range1 == range2 {
                Self::add_work_split(calendar.as_ref(), &mut result, range1);
                index1 += 1;
                index2 += 1;
                continue;
            }

            Self::add_work_split(
                calendar.as_ref(),
                &mut result,
                LocalDateTimeRange::new(
                    LocalDateTimeHelper::min(range1.get_start(), range2.get_start()),
                    LocalDateTimeHelper::max(range1.get_end(), range2.get_end()),
                ),
            );
            index1 += 1;
            index2 += 1;
        }

        if index1 != list1.len() {
            for range in &list1[index1..] {
                Self::add_work_split(calendar.as_ref(), &mut result, *range);
            }
        } else if index2 != list2.len() {
            for range in &list2[index2..] {
                Self::add_work_split(calendar.as_ref(), &mut result, *range);
            }
        }

        result
    }

/// Add a work split to the result list (merging with the previous one if necessary).
    ///
    /// Corresponds to Java: `private void addWorkSplit(ProjectCalendar calendar, List<LocalDateTimeRange> ranges, LocalDateTimeRange range)`
    ///
/// Java throws `NullPointerException` when `calendar` or a range endpoint is `null`; the Rust side
/// treats a missing calendar or empty date as "no working time" (`0.0`), and still does not panic.
    fn add_work_split(
        calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
        ranges: &mut Vec<LocalDateTimeRange>,
        range: LocalDateTimeRange,
    ) {
        if let Some(last_range) = ranges.last().copied() {
            let adjacent = last_range.get_end() == range.get_start();
            let no_working_time = work_hours(calendar, last_range.get_end(), range.get_start()) == 0.0;

            if adjacent || last_range.intersects_with(&range) || no_working_time {
                // Ignore range if it is entirely within lastRange
                let extends_last = match (range.get_end(), last_range.get_end()) {
                    (Some(range_end), Some(last_end)) => range_end > last_end,
                    _ => false,
                };
                if extends_last {
                    ranges.pop();
                    ranges.push(LocalDateTimeRange::new(
                        last_range.get_start(),
                        range.get_end(),
                    ));
                }
                return;
            }
        }

        ranges.push(range);
    }

/// Set a custom number field.
    ///
    /// Corresponds to Java: `public void setNumber(int index, Number value)`
    pub fn set_number(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = select_field(custom_number_fields(), index) {
            self.put_number(field, value);
        }
    }

/// Read a custom number field.
    ///
    /// Corresponds to Java: `public Number getNumber(int index)`
    pub fn get_number(&self, index: i32) -> Option<f64> {
        select_field(custom_number_fields(), index).and_then(|field| self.number_field(field))
    }

/// Get the number of attached objects.
    ///
    /// Corresponds to Java: `public Integer getObjects()`
    pub fn get_objects(&self) -> Option<i32> {
        self.integer_field(TaskField::Objects)
    }

/// Get the outline level.
    ///
    /// Corresponds to Java: `public Integer getOutlineLevel()`
    pub fn get_outline_level(&self) -> Option<i32> {
        self.integer_field(TaskField::OutlineLevel)
    }

/// Get the outline number.
    ///
    /// Corresponds to Java: `public String getOutlineNumber()`
    pub fn get_outline_number(&self) -> Option<String> {
        self.string_field(TaskField::OutlineNumber)
    }

/// Get the priority.
    ///
    /// Corresponds to Java: `public Priority getPriority()`
    pub fn get_priority(&self) -> Option<Priority> {
        self.integer_field(TaskField::Priority)
            .map(Priority::get_instance)
    }

/// Get the source project name.
    ///
    /// Corresponds to Java: `public String getProject()`
    pub fn get_project(&self) -> Option<String> {
        self.string_field(TaskField::Project)
    }

/// Get the remaining cost.
    ///
    /// Corresponds to Java: `public Number getRemainingCost()`
    pub fn get_remaining_cost(&self) -> Option<f64> {
        self.number_field(TaskField::RemainingCost)
    }

/// Get the remaining duration.
    ///
    /// Corresponds to Java: `public Duration getRemainingDuration()`
    pub fn get_remaining_duration(&self) -> Option<Duration> {
        self.duration_field(TaskField::RemainingDuration)
    }

/// Get the remaining work.
    ///
    /// Corresponds to Java: `public Duration getRemainingWork()`
    pub fn get_remaining_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::RemainingWork)
    }

/// Get the resource group.
    ///
    /// Corresponds to Java: `public String getResourceGroup()`
    pub fn get_resource_group(&self) -> Option<String> {
        self.string_field(TaskField::ResourceGroup)
    }

/// Get the resource initials.
    ///
    /// Corresponds to Java: `public String getResourceInitials()`
    pub fn get_resource_initials(&self) -> Option<String> {
        self.string_field(TaskField::ResourceInitials)
    }

/// Get the resource names.
    ///
    /// Corresponds to Java: `public String getResourceNames()`
    pub fn get_resource_names(&self) -> Option<String> {
        self.string_field(TaskField::ResourceNames)
    }

/// Get the resume date.
    ///
    /// Corresponds to Java: `public LocalDateTime getResume()`
    pub fn get_resume(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::Resume)
    }

/// Get the rollup flag.
    ///
    /// Corresponds to Java: `public boolean getRollup()`
    pub fn get_rollup(&self) -> bool {
        self.boolean_field(TaskField::Rollup)
    }

/// Get the start date.
    ///
    /// Corresponds to Java: `public LocalDateTime getStart()`
    pub fn get_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::Start)
    }

/// Get the start text for a manually scheduled task.
    ///
    /// Corresponds to Java: `public String getStartText()`
    pub fn get_start_text(&self) -> Option<String> {
        self.string_field(TaskField::StartText)
    }

/// Set a custom start date field.
    ///
    /// Corresponds to Java: `public void setStart(int index, LocalDateTime value)`
    pub fn set_start_at(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(custom_start_fields(), index) {
            self.put_date_time(field, value);
        }
    }

/// Read a custom start date field.
    ///
    /// Corresponds to Java: `public LocalDateTime getStart(int index)`
    pub fn get_start_at(&self, index: i32) -> Option<NaiveDateTime> {
        select_field(custom_start_fields(), index).and_then(|field| self.date_time_field(field))
    }

/// Get the start variance.
    ///
    /// Corresponds to Java: `public Duration getStartVariance()`
    pub fn get_start_variance(&self) -> Option<Duration> {
        self.duration_field(TaskField::StartVariance)
    }

/// Get the stop date.
    ///
    /// Corresponds to Java: `public LocalDateTime getStop()`
    pub fn get_stop(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::Stop)
    }

/// Get the subproject file path.
    ///
    /// Corresponds to Java: `public String getSubprojectFile()`
    pub fn get_subproject_file(&self) -> Option<String> {
        self.string_field(TaskField::SubprojectFile)
    }

/// Get the summary task flag.
    ///
    /// Corresponds to Java: `public boolean getSummary()`
    pub fn get_summary(&self) -> bool {
        self.boolean_field(TaskField::Summary)
    }

/// Get the SV.
    ///
    /// Corresponds to Java: `public Number getSV()`
    pub fn get_sv(&self) -> Option<f64> {
        self.number_field(TaskField::Sv)
    }

/// Set a custom text field.
    ///
    /// Corresponds to Java: `public void setText(int index, String value)`
    pub fn set_text(&mut self, index: i32, value: Option<String>) {
        if let Some(field) = select_field(custom_text_fields(), index) {
            self.put_string(field, value);
        }
    }

/// Read a custom text field.
    ///
    /// Corresponds to Java: `public String getText(int index)`
    pub fn get_text(&self, index: i32) -> Option<String> {
        select_field(custom_text_fields(), index).and_then(|field| self.string_field(field))
    }

/// Set a custom outline code field.
    ///
    /// Corresponds to Java: `public void setOutlineCode(int index, String value)`
    pub fn set_outline_code(&mut self, index: i32, value: Option<String>) {
        if let Some(field) = select_field(custom_outline_code_fields(), index) {
            self.put_string(field, value);
        }
    }

/// Read a custom outline code field.
    ///
    /// Corresponds to Java: `public String getOutlineCode(int index)`
    pub fn get_outline_code(&self, index: i32) -> Option<String> {
        select_field(custom_outline_code_fields(), index).and_then(|field| self.string_field(field))
    }

/// Get the total slack.
    ///
    /// Corresponds to Java: `public Duration getTotalSlack()`
    pub fn get_total_slack(&self) -> Option<Duration> {
        self.duration_field(TaskField::TotalSlack)
    }

/// Get the unique ID.
    ///
    /// Corresponds to Java: `@Override public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.integer_field(TaskField::UniqueId)
    }

/// Get the "update needed" flag.
    ///
    /// Corresponds to Java: `public boolean getUpdateNeeded()`
    pub fn get_update_needed(&self) -> bool {
        self.boolean_field(TaskField::UpdateNeeded)
    }

/// Get the WBS code.
    ///
    /// Corresponds to Java: `public String getWBS()`
    pub fn get_wbs(&self) -> Option<String> {
        self.string_field(TaskField::Wbs)
    }

/// Get the work.
    ///
    /// Corresponds to Java: `public Duration getWork()`
    pub fn get_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::Work)
    }

/// Get the work variance.
    ///
    /// Corresponds to Java: `public Duration getWorkVariance()`
    pub fn get_work_variance(&self) -> Option<Duration> {
        self.duration_field(TaskField::WorkVariance)
    }

/// Get the parent task unique ID.
    ///
    /// Corresponds to Java: `public Integer getParentTaskUniqueID()`
    pub fn get_parent_task_unique_id(&self) -> Option<i32> {
        self.integer_field(TaskField::ParentTaskUniqueId)
    }

/// Compare two tasks by ID.
    ///
    /// Corresponds to Java: `@Override public int compareTo(Task o)`
    ///
/// Java's `compareTo` is inconsistent with identity equality and cannot be mapped to Rust's `Ord`
/// (see difference 6 in the file header), so it is kept as a named method.
    pub fn compare_to(&self, other: &Task) -> Ordering {
        let id1 = NumberHelper::get_int(self.get_id().map(f64::from));
        let id2 = NumberHelper::get_int(other.get_id().map(f64::from));
        id1.cmp(&id2)
    }
}

// ---------------------------------------------------------------------------
// Boolean / date / enum / UUID properties
// ---------------------------------------------------------------------------

impl Task {
/// Get the "duration is estimated" flag.
    ///
    /// Corresponds to Java: `public boolean getEstimated()`
    pub fn get_estimated(&self) -> bool {
        self.boolean_field(TaskField::Estimated)
    }

/// Set the "duration is estimated" flag.
    ///
    /// Corresponds to Java: `public void setEstimated(boolean estimated)`
    pub fn set_estimated(&mut self, estimated: bool) {
        self.put_bool(TaskField::Estimated, estimated);
    }

/// Get the deadline date.
    ///
    /// Corresponds to Java: `public LocalDateTime getDeadline()`
    pub fn get_deadline(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::Deadline)
    }

/// Set the deadline date.
    ///
    /// Corresponds to Java: `public void setDeadline(LocalDateTime deadline)`
    pub fn set_deadline(&mut self, deadline: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::Deadline, deadline);
    }

/// Get the task type.
    ///
    /// Corresponds to Java: `public TaskType getType()`
    pub fn get_type(&self) -> Option<TaskType> {
        self.integer_field(TaskField::Type)
            .and_then(task_type_from_value)
    }

/// Set the task type.
    ///
    /// Corresponds to Java: `public void setType(TaskType type)`
    pub fn set_type(&mut self, r#type: Option<TaskType>) {
        self.put_enum(TaskField::Type, r#type.map(task_type_to_value));
    }

/// Get the "null task" flag.
    ///
    /// Corresponds to Java: `public boolean getNull()`
    pub fn get_null(&self) -> bool {
        self.boolean_field(TaskField::Null)
    }

/// Set the "null task" flag.
    ///
    /// Corresponds to Java: `public void setNull(boolean isNull)`
    pub fn set_null(&mut self, is_null: bool) {
        self.put_bool(TaskField::Null, is_null);
    }

/// Get the "resume valid" flag.
    ///
    /// Corresponds to Java: `public boolean getResumeValid()`
    pub fn get_resume_valid(&self) -> bool {
        self.boolean_field(TaskField::ResumeValid)
    }

/// Set the "resume valid" flag.
    ///
    /// Corresponds to Java: `public void setResumeValid(boolean resumeValid)`
    pub fn set_resume_valid(&mut self, resume_valid: bool) {
        self.put_bool(TaskField::ResumeValid, resume_valid);
    }

/// Get the "recurring task" flag.
    ///
    /// Corresponds to Java: `public boolean getRecurring()`
    pub fn get_recurring(&self) -> bool {
        self.boolean_field(TaskField::Recurring)
    }

/// Set the "recurring task" flag.
    ///
    /// Corresponds to Java: `public void setRecurring(boolean recurring)`
    pub fn set_recurring(&mut self, recurring: bool) {
        self.put_bool(TaskField::Recurring, recurring);
    }

/// Get the "resource overallocated" flag.
    ///
    /// Corresponds to Java: `public boolean getOverAllocated()`
    pub fn get_over_allocated(&self) -> bool {
        self.boolean_field(TaskField::Overallocated)
    }

/// Set the "resource overallocated" flag.
    ///
    /// Corresponds to Java: `public void setOverAllocated(boolean overAllocated)`
    pub fn set_over_allocated(&mut self, over_allocated: bool) {
        self.put_bool(TaskField::Overallocated, over_allocated);
    }

/// Get the unique ID of the task in the subproject.
    ///
    /// Corresponds to Java: `public Integer getSubprojectTaskUniqueID()`
    pub fn get_subproject_task_unique_id(&self) -> Option<i32> {
        self.integer_field(TaskField::SubprojectTaskUniqueId)
    }

/// Set the unique ID of the task in the subproject.
    ///
    /// Corresponds to Java: `public void setSubprojectTaskUniqueID(Integer subprojectUniqueTaskID)`
    pub fn set_subproject_task_unique_id(&mut self, subproject_unique_task_id: Option<i32>) {
        self.put_integer(TaskField::SubprojectTaskUniqueId, subproject_unique_task_id);
    }

/// Get the ID of the task in the subproject.
    ///
    /// Corresponds to Java: `public Integer getSubprojectTaskID()`
    pub fn get_subproject_task_id(&self) -> Option<i32> {
        self.integer_field(TaskField::SubprojectTaskId)
    }

/// Set the ID of the task in the subproject.
    ///
    /// Corresponds to Java: `public void setSubprojectTaskID(Integer subprojectTaskID)`
    pub fn set_subproject_task_id(&mut self, subproject_task_id: Option<i32>) {
        self.put_integer(TaskField::SubprojectTaskId, subproject_task_id);
    }

/// Set the subproject tasks unique ID offset.
    ///
    /// Corresponds to Java: `public void setSubprojectTasksUniqueIDOffset(Integer offset)`
    pub fn set_subproject_tasks_unique_id_offset(&mut self, offset: Option<i32>) {
        self.put_integer(TaskField::SubprojectTasksUniqueidOffset, offset);
    }

/// Get the subproject tasks unique ID offset.
    ///
    /// Corresponds to Java: `public Integer getSubprojectTasksUniqueIDOffset()`
    pub fn get_subproject_tasks_unique_id_offset(&self) -> Option<i32> {
        self.integer_field(TaskField::SubprojectTasksUniqueidOffset)
    }

/// Get the subproject read-only flag.
    ///
    /// Corresponds to Java: `public boolean getSubprojectReadOnly()`
    pub fn get_subproject_read_only(&self) -> bool {
        self.boolean_field(TaskField::SubprojectReadOnly)
    }

/// Set the subproject read-only flag.
    ///
    /// Corresponds to Java: `public void setSubprojectReadOnly(boolean subprojectReadOnly)`
    pub fn set_subproject_read_only(&mut self, subproject_read_only: bool) {
        self.put_bool(TaskField::SubprojectReadOnly, subproject_read_only);
    }

/// Get the external task flag.
    ///
    /// Corresponds to Java: `public boolean getExternalTask()`
    pub fn get_external_task(&self) -> bool {
        self.boolean_field(TaskField::ExternalTask)
    }

/// Set the external task flag.
    ///
    /// Corresponds to Java: `public void setExternalTask(boolean externalTask)`
    pub fn set_external_task(&mut self, external_task: bool) {
        self.put_bool(TaskField::ExternalTask, external_task);
    }

/// Get the external project flag.
    ///
    /// Corresponds to Java: `public boolean getExternalProject()`
    pub fn get_external_project(&self) -> bool {
        self.boolean_field(TaskField::ExternalProject)
    }

/// Get ACWP (actual cost of work performed).
    ///
    /// Corresponds to Java: `public Number getACWP()`
    pub fn get_acwp(&self) -> Option<f64> {
        self.number_field(TaskField::Acwp)
    }

/// Set ACWP.
    ///
    /// Corresponds to Java: `public void setACWP(Number acwp)`
    pub fn set_acwp(&mut self, acwp: Option<f64>) {
        self.put_number(TaskField::Acwp, acwp);
    }

/// Get the leveling delay units.
    ///
    /// Corresponds to Java: `public TimeUnit getLevelingDelayFormat()`
    pub fn get_leveling_delay_format(&self) -> Option<TimeUnit> {
        self.integer_field(TaskField::LevelingDelayUnits)
            .map(TimeUnit::get_instance)
    }

/// Set the leveling delay units.
    ///
    /// Corresponds to Java: `public void setLevelingDelayFormat(TimeUnit levelingDelayFormat)`
    pub fn set_leveling_delay_format(&mut self, leveling_delay_format: Option<TimeUnit>) {
        self.put_enum(
            TaskField::LevelingDelayUnits,
            leveling_delay_format.map(|value| value.value()),
        );
    }

/// Get the "ignore resource calendar" flag.
    ///
    /// Corresponds to Java: `public boolean getIgnoreResourceCalendar()`
    pub fn get_ignore_resource_calendar(&self) -> bool {
        self.boolean_field(TaskField::IgnoreResourceCalendar)
    }

/// Set the "ignore resource calendar" flag.
    ///
    /// Corresponds to Java: `public void setIgnoreResourceCalendar(boolean ignoreResourceCalendar)`
    pub fn set_ignore_resource_calendar(&mut self, ignore_resource_calendar: bool) {
        self.put_bool(TaskField::IgnoreResourceCalendar, ignore_resource_calendar);
    }

/// Get the physical percent complete.
    ///
    /// Corresponds to Java: `public Number getPhysicalPercentComplete()`
    pub fn get_physical_percent_complete(&self) -> Option<f64> {
        self.number_field(TaskField::PhysicalPercentComplete)
    }

/// Set the physical percent complete.
    ///
    /// Corresponds to Java: `public void setPhysicalPercentComplete(Number physicalPercentComplete)`
    pub fn set_physical_percent_complete(&mut self, physical_percent_complete: Option<f64>) {
        self.put_number(TaskField::PhysicalPercentComplete, physical_percent_complete);
    }

/// Get the estimated weight.
    ///
    /// Corresponds to Java: `public Number getEstimatedWeight()`
    pub fn get_estimated_weight(&self) -> Option<f64> {
        self.number_field(TaskField::EstimatedWeight)
    }

/// Set the estimated weight.
    ///
    /// Corresponds to Java: `public void setEstimatedWeight(Number estimatedWeight)`
    pub fn set_estimated_weight(&mut self, estimated_weight: Option<f64>) {
        self.put_number(TaskField::EstimatedWeight, estimated_weight);
    }

/// Get the "auto compute actuals" flag.
    ///
    /// Corresponds to Java: `public boolean getAutoComputeActuals()`
    pub fn get_auto_compute_actuals(&self) -> bool {
        self.boolean_field(TaskField::AutoComputeActuals)
    }

/// Set the "auto compute actuals" flag.
    ///
    /// Corresponds to Java: `public void setAutoComputeActuals(boolean autoComputeActuals)`
    pub fn set_auto_compute_actuals(&mut self, auto_compute_actuals: bool) {
        self.put_bool(TaskField::AutoComputeActuals, auto_compute_actuals);
    }

/// Get the earned value method.
    ///
    /// Corresponds to Java: `public EarnedValueMethod getEarnedValueMethod()`
    pub fn get_earned_value_method(&self) -> Option<EarnedValueMethod> {
        self.integer_field(TaskField::EarnedValueMethod)
            .map(EarnedValueMethod::get_instance)
    }

/// Set the earned value method.
    ///
    /// Corresponds to Java: `public void setEarnedValueMethod(EarnedValueMethod earnedValueMethod)`
    pub fn set_earned_value_method(&mut self, earned_value_method: Option<EarnedValueMethod>) {
        self.put_enum(
            TaskField::EarnedValueMethod,
            earned_value_method.map(|value| value.value()),
        );
    }

/// Get the protected actual work.
    ///
    /// Corresponds to Java: `public Duration getActualWorkProtected()`
    pub fn get_actual_work_protected(&self) -> Option<Duration> {
        self.duration_field(TaskField::ActualWorkProtected)
    }

/// Set the protected actual work.
    ///
    /// Corresponds to Java: `public void setActualWorkProtected(Duration actualWorkProtected)`
    pub fn set_actual_work_protected(&mut self, actual_work_protected: Option<Duration>) {
        self.put_duration(TaskField::ActualWorkProtected, actual_work_protected);
    }

/// Get the protected actual overtime work.
    ///
    /// Corresponds to Java: `public Duration getActualOvertimeWorkProtected()`
    pub fn get_actual_overtime_work_protected(&self) -> Option<Duration> {
        self.duration_field(TaskField::ActualOvertimeWorkProtected)
    }

/// Set the protected actual overtime work.
    ///
    /// Corresponds to Java: `public void setActualOvertimeWorkProtected(Duration actualOvertimeWorkProtected)`
    pub fn set_actual_overtime_work_protected(
        &mut self,
        actual_overtime_work_protected: Option<Duration>,
    ) {
        self.put_duration(
            TaskField::ActualOvertimeWorkProtected,
            actual_overtime_work_protected,
        );
    }

/// Get the regular work.
    ///
    /// Corresponds to Java: `public Duration getRegularWork()`
    pub fn get_regular_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::RegularWork)
    }

/// Set the regular work.
    ///
    /// Corresponds to Java: `public void setRegularWork(Duration regularWork)`
    pub fn set_regular_work(&mut self, regular_work: Option<Duration>) {
        self.put_duration(TaskField::RegularWork, regular_work);
    }

/// Set the "effort driven" flag.
    ///
    /// Corresponds to Java: `public void setEffortDriven(boolean flag)`
    pub fn set_effort_driven(&mut self, flag: bool) {
        self.put_bool(TaskField::EffortDriven, flag);
    }

/// Get the "effort driven" flag.
    ///
    /// Corresponds to Java: `public boolean getEffortDriven()`
    pub fn get_effort_driven(&self) -> bool {
        self.boolean_field(TaskField::EffortDriven)
    }

/// Set a custom date field.
    ///
    /// Corresponds to Java: `public void setDate(int index, LocalDateTime value)`
    pub fn set_date(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(custom_date_fields(), index) {
            self.put_date_time(field, value);
        }
    }

/// Read a custom date field.
    ///
    /// Corresponds to Java: `public LocalDateTime getDate(int index)`
    pub fn get_date(&self, index: i32) -> Option<NaiveDateTime> {
        select_field(custom_date_fields(), index).and_then(|field| self.date_time_field(field))
    }

/// Get the overtime cost.
    ///
    /// Corresponds to Java: `public Number getOvertimeCost()`
    pub fn get_overtime_cost(&self) -> Option<f64> {
        self.number_field(TaskField::OvertimeCost)
    }

/// Set the overtime cost.
    ///
    /// Corresponds to Java: `public void setOvertimeCost(Number number)`
    pub fn set_overtime_cost(&mut self, number: Option<f64>) {
        self.put_number(TaskField::OvertimeCost, number);
    }

/// Get the actual overtime cost.
    ///
    /// Corresponds to Java: `public Number getActualOvertimeCost()`
    pub fn get_actual_overtime_cost(&self) -> Option<f64> {
        self.number_field(TaskField::ActualOvertimeCost)
    }

/// Set the actual overtime cost.
    ///
    /// Corresponds to Java: `public void setActualOvertimeCost(Number cost)`
    pub fn set_actual_overtime_cost(&mut self, cost: Option<f64>) {
        self.put_number(TaskField::ActualOvertimeCost, cost);
    }

/// Get the actual overtime work.
    ///
    /// Corresponds to Java: `public Duration getActualOvertimeWork()`
    pub fn get_actual_overtime_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::ActualOvertimeWork)
    }

/// Set the actual overtime work.
    ///
    /// Corresponds to Java: `public void setActualOvertimeWork(Duration work)`
    pub fn set_actual_overtime_work(&mut self, work: Option<Duration>) {
        self.put_duration(TaskField::ActualOvertimeWork, work);
    }

/// Get the fixed cost accrual.
    ///
    /// Corresponds to Java: `public AccrueType getFixedCostAccrual()`
    pub fn get_fixed_cost_accrual(&self) -> Option<AccrueType> {
        self.integer_field(TaskField::FixedCostAccrual)
            .and_then(AccrueType::get_instance)
    }

/// Set the fixed cost accrual.
    ///
    /// Corresponds to Java: `public void setFixedCostAccrual(AccrueType type)`
    pub fn set_fixed_cost_accrual(&mut self, r#type: Option<AccrueType>) {
        self.put_enum(
            TaskField::FixedCostAccrual,
            r#type.map(|value| value.value()),
        );
    }

/// Get the hyperlink text.
    ///
    /// Corresponds to Java: `public String getHyperlink()`
    pub fn get_hyperlink(&self) -> Option<String> {
        self.string_field(TaskField::Hyperlink)
    }

/// Get the hyperlink address.
    ///
    /// Corresponds to Java: `public String getHyperlinkAddress()`
    pub fn get_hyperlink_address(&self) -> Option<String> {
        self.string_field(TaskField::HyperlinkAddress)
    }

/// Get the hyperlink sub address.
    ///
    /// Corresponds to Java: `public String getHyperlinkSubAddress()`
    pub fn get_hyperlink_sub_address(&self) -> Option<String> {
        self.string_field(TaskField::HyperlinkSubaddress)
    }

/// Get the hyperlink screen tip.
    ///
    /// Corresponds to Java: `public String getHyperlinkScreenTip()`
    pub fn get_hyperlink_screen_tip(&self) -> Option<String> {
        self.string_field(TaskField::HyperlinkScreenTip)
    }

/// Set the hyperlink text.
    ///
    /// Corresponds to Java: `public void setHyperlink(String text)`
    pub fn set_hyperlink(&mut self, text: Option<String>) {
        self.put_string(TaskField::Hyperlink, text);
    }

/// Set the hyperlink address.
    ///
    /// Corresponds to Java: `public void setHyperlinkAddress(String text)`
    pub fn set_hyperlink_address(&mut self, text: Option<String>) {
        self.put_string(TaskField::HyperlinkAddress, text);
    }

/// Set the hyperlink sub address.
    ///
    /// Corresponds to Java: `public void setHyperlinkSubAddress(String text)`
    pub fn set_hyperlink_sub_address(&mut self, text: Option<String>) {
        self.put_string(TaskField::HyperlinkSubaddress, text);
    }

/// Set the hyperlink screen tip.
    ///
    /// Corresponds to Java: `public void setHyperlinkScreenTip(String text)`
    pub fn set_hyperlink_screen_tip(&mut self, text: Option<String>) {
        self.put_string(TaskField::HyperlinkScreenTip, text);
    }

/// Get the "level assignments" flag.
    ///
    /// Corresponds to Java: `public boolean getLevelAssignments()`
    pub fn get_level_assignments(&self) -> bool {
        self.boolean_field(TaskField::LevelAssignments)
    }

/// Set the "level assignments" flag.
    ///
    /// Corresponds to Java: `public void setLevelAssignments(boolean flag)`
    pub fn set_level_assignments(&mut self, flag: bool) {
        self.put_bool(TaskField::LevelAssignments, flag);
    }

/// Get the "leveling can split" flag.
    ///
    /// Corresponds to Java: `public boolean getLevelingCanSplit()`
    pub fn get_leveling_can_split(&self) -> bool {
        self.boolean_field(TaskField::LevelingCanSplit)
    }

/// Set the "leveling can split" flag.
    ///
    /// Corresponds to Java: `public void setLevelingCanSplit(boolean flag)`
    pub fn set_leveling_can_split(&mut self, flag: bool) {
        self.put_bool(TaskField::LevelingCanSplit, flag);
    }

/// Get the overtime work.
    ///
    /// Corresponds to Java: `public Duration getOvertimeWork()`
    pub fn get_overtime_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::OvertimeWork)
    }

/// Set the overtime work.
    ///
    /// Corresponds to Java: `public void setOvertimeWork(Duration work)`
    pub fn set_overtime_work(&mut self, work: Option<Duration>) {
        self.put_duration(TaskField::OvertimeWork, work);
    }

/// Get the preleveled start date.
    ///
    /// Corresponds to Java: `public LocalDateTime getPreleveledStart()`
    pub fn get_preleveled_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::PreleveledStart)
    }

/// Get the preleveled finish date.
    ///
    /// Corresponds to Java: `public LocalDateTime getPreleveledFinish()`
    pub fn get_preleveled_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::PreleveledFinish)
    }

/// Set the preleveled start date.
    ///
    /// Corresponds to Java: `public void setPreleveledStart(LocalDateTime date)`
    pub fn set_preleveled_start(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::PreleveledStart, date);
    }

/// Set the preleveled finish date.
    ///
    /// Corresponds to Java: `public void setPreleveledFinish(LocalDateTime date)`
    pub fn set_preleveled_finish(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::PreleveledFinish, date);
    }

/// Get the remaining overtime work.
    ///
    /// Corresponds to Java: `public Duration getRemainingOvertimeWork()`
    pub fn get_remaining_overtime_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::RemainingOvertimeWork)
    }

/// Set the remaining overtime work.
    ///
    /// Corresponds to Java: `public void setRemainingOvertimeWork(Duration work)`
    pub fn set_remaining_overtime_work(&mut self, work: Option<Duration>) {
        self.put_duration(TaskField::RemainingOvertimeWork, work);
    }

/// Get the remaining overtime cost.
    ///
    /// Corresponds to Java: `public Number getRemainingOvertimeCost()`
    pub fn get_remaining_overtime_cost(&self) -> Option<f64> {
        self.number_field(TaskField::RemainingOvertimeCost)
    }

/// Set the remaining overtime cost.
    ///
    /// Corresponds to Java: `public void setRemainingOvertimeCost(Number cost)`
    pub fn set_remaining_overtime_cost(&mut self, cost: Option<f64>) {
        self.put_number(TaskField::RemainingOvertimeCost, cost);
    }

/// Get the calendar associated with the task.
    ///
    /// Corresponds to Java: `public ProjectCalendar getCalendar()`
    pub fn get_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        let file = self.parent_file.upgrade()?;
        let calendar_unique_id = self.get_calendar_unique_id();
        file.borrow().get_calendars().get_by_unique_id(calendar_unique_id)
    }

/// Set the calendar unique ID.
    ///
    /// Corresponds to Java: `public void setCalendarUniqueID(Integer id)`
    pub fn set_calendar_unique_id(&mut self, id: Option<i32>) {
        self.put_integer(TaskField::CalendarUniqueId, id);
    }

/// Get the calendar unique ID.
    ///
    /// Corresponds to Java: `public Integer getCalendarUniqueID()`
    pub fn get_calendar_unique_id(&self) -> Option<i32> {
        self.integer_field(TaskField::CalendarUniqueId)
    }

/// Set the calendar associated with the task.
    ///
    /// Corresponds to Java: `public void setCalendar(ProjectCalendar calendar)`
    pub fn set_calendar(&mut self, calendar: Option<&Rc<RefCell<ProjectCalendar>>>) {
        let unique_id = calendar.and_then(|calendar| calendar.borrow().get_unique_id());
        self.set_calendar_unique_id(unique_id);
    }

/// Get the "expanded" flag.
    ///
    /// Corresponds to Java: `public boolean getExpanded()`
    pub fn get_expanded(&self) -> bool {
        self.boolean_field(TaskField::Expanded)
    }

/// Set the "expanded" flag.
    ///
    /// Corresponds to Java: `public void setExpanded(boolean expanded)`
    pub fn set_expanded(&mut self, expanded: bool) {
        self.put_bool(TaskField::Expanded, expanded);
    }

/// Set the start slack.
    ///
    /// Corresponds to Java: `public void setStartSlack(Duration duration)`
    pub fn set_start_slack(&mut self, duration: Option<Duration>) {
        self.put_duration(TaskField::StartSlack, duration);
    }

/// Set the finish slack.
    ///
    /// Corresponds to Java: `public void setFinishSlack(Duration duration)`
    pub fn set_finish_slack(&mut self, duration: Option<Duration>) {
        self.put_duration(TaskField::FinishSlack, duration);
    }

/// Get the start slack.
    ///
    /// Corresponds to Java: `public Duration getStartSlack()`
    pub fn get_start_slack(&self) -> Option<Duration> {
        self.duration_field(TaskField::StartSlack)
    }

/// Get the finish slack.
    ///
    /// Corresponds to Java: `public Duration getFinishSlack()`
    pub fn get_finish_slack(&self) -> Option<Duration> {
        self.duration_field(TaskField::FinishSlack)
    }

/// Read a field value by field alias.
    ///
    /// Corresponds to Java: `public Object getFieldByAlias(String alias)`
    pub fn get_field_by_alias(&self, alias: &str) -> Option<FieldValue> {
        let file = self.parent_file.upgrade()?;
        let field = file.borrow().get_tasks().get_field_type_by_alias(alias)?;
        self.get(field)
    }

/// Set a field value by field alias.
    ///
    /// Corresponds to Java: `public void setFieldByAlias(String alias, Object value)`
    pub fn set_field_by_alias(&mut self, alias: &str, value: Option<FieldValue>) {
        let Some(file) = self.parent_file.upgrade() else {
            return;
        };
        let field = file.borrow().get_tasks().get_field_type_by_alias(alias);
        if let Some(field) = field {
            self.set(field, value);
        }
    }
}

// ---------------------------------------------------------------------------
// Multiple baselines (Baseline 1-10) and Enterprise custom fields
// ---------------------------------------------------------------------------

impl Task {
/// Set the baseline cost (multiple baselines).
    ///
    /// Corresponds to Java: `public void setBaselineCost(int baselineNumber, Number value)`
    pub fn set_baseline_cost_at(&mut self, baseline_number: i32, value: Option<f64>) {
        if let Some(field) = select_field(baseline_cost_fields(), baseline_number) {
            self.put_number(field, value);
        }
    }

/// Set the baseline duration (multiple baselines).
    ///
    /// Corresponds to Java: `public void setBaselineDuration(int baselineNumber, Duration value)`
    pub fn set_baseline_duration_at(&mut self, baseline_number: i32, value: Option<Duration>) {
        if let Some(field) = select_field(baseline_duration_fields(), baseline_number) {
            self.put_duration(field, value);
        }
    }

/// Set the baseline finish date (multiple baselines).
    ///
    /// Corresponds to Java: `public void setBaselineFinish(int baselineNumber, LocalDateTime value)`
    pub fn set_baseline_finish_at(&mut self, baseline_number: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(baseline_finish_fields(), baseline_number) {
            self.put_date_time(field, value);
        }
    }

/// Set baseline start date (multiple baselines).
    ///
    /// Corresponds to Java: `public void setBaselineStart(int baselineNumber, LocalDateTime value)`
    pub fn set_baseline_start_at(&mut self, baseline_number: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(baseline_start_fields(), baseline_number) {
            self.put_date_time(field, value);
        }
    }

/// Set baseline work (multiple baselines).
    ///
    /// Corresponds to Java: `public void setBaselineWork(int baselineNumber, Duration value)`
    pub fn set_baseline_work_at(&mut self, baseline_number: i32, value: Option<Duration>) {
        if let Some(field) = select_field(baseline_work_fields(), baseline_number) {
            self.put_duration(field, value);
        }
    }

/// Get baseline cost (multiple baselines).
    ///
    /// Corresponds to Java: `public Number getBaselineCost(int baselineNumber)`
    pub fn get_baseline_cost_at(&self, baseline_number: i32) -> Option<f64> {
        select_field(baseline_cost_fields(), baseline_number)
            .and_then(|field| self.number_field(field))
    }

/// Get baseline duration (multiple baselines).
    ///
    /// Corresponds to Java: `public Duration getBaselineDuration(int baselineNumber)`
    pub fn get_baseline_duration_at(&self, baseline_number: i32) -> Option<Duration> {
        select_field(baseline_duration_fields(), baseline_number)
            .and_then(|field| self.duration_field(field))
    }

/// Get baseline duration text (multiple baselines).
    ///
    /// Corresponds to Java: `public String getBaselineDurationText(int baselineNumber)`
    pub fn get_baseline_duration_text_at(&self, baseline_number: i32) -> Option<String> {
        select_field(baseline_duration_fields(), baseline_number)
            .and_then(|field| self.string_field(field))
    }

/// Set baseline duration text (multiple baselines).
    ///
    /// Corresponds to Java: `public void setBaselineDurationText(int baselineNumber, String value)`
    pub fn set_baseline_duration_text_at(&mut self, baseline_number: i32, value: Option<String>) {
        if let Some(field) = select_field(baseline_duration_fields(), baseline_number) {
            self.put_string(field, value);
        }
    }

/// Get baseline finish date (multiple baselines).
    ///
    /// Corresponds to Java: `public LocalDateTime getBaselineFinish(int baselineNumber)`
    pub fn get_baseline_finish_at(&self, baseline_number: i32) -> Option<NaiveDateTime> {
        select_field(baseline_finish_fields(), baseline_number)
            .and_then(|field| self.date_time_field(field))
    }

/// Get baseline finish text (multiple baselines).
    ///
    /// Corresponds to Java: `public String getBaselineFinishText(int baselineNumber)`
    pub fn get_baseline_finish_text_at(&self, baseline_number: i32) -> Option<String> {
        select_field(baseline_finish_fields(), baseline_number)
            .and_then(|field| self.string_field(field))
    }

/// Set baseline finish text (multiple baselines).
    ///
    /// Corresponds to Java: `public void setBaselineFinishText(int baselineNumber, String value)`
    pub fn set_baseline_finish_text_at(&mut self, baseline_number: i32, value: Option<String>) {
        if let Some(field) = select_field(baseline_finish_fields(), baseline_number) {
            self.put_string(field, value);
        }
    }

/// Get baseline start date (multiple baselines).
    ///
    /// Corresponds to Java: `public LocalDateTime getBaselineStart(int baselineNumber)`
    pub fn get_baseline_start_at(&self, baseline_number: i32) -> Option<NaiveDateTime> {
        select_field(baseline_start_fields(), baseline_number)
            .and_then(|field| self.date_time_field(field))
    }

/// Get baseline start text (multiple baselines).
    ///
    /// Corresponds to Java: `public String getBaselineStartText(int baselineNumber)`
    pub fn get_baseline_start_text_at(&self, baseline_number: i32) -> Option<String> {
        select_field(baseline_start_fields(), baseline_number)
            .and_then(|field| self.string_field(field))
    }

/// Set baseline start text (multiple baselines).
    ///
    /// Corresponds to Java: `public void setBaselineStartText(int baselineNumber, String value)`
    pub fn set_baseline_start_text_at(&mut self, baseline_number: i32, value: Option<String>) {
        if let Some(field) = select_field(baseline_start_fields(), baseline_number) {
            self.put_string(field, value);
        }
    }

/// Get baseline work (multiple baselines).
    ///
    /// Corresponds to Java: `public Duration getBaselineWork(int baselineNumber)`
    pub fn get_baseline_work_at(&self, baseline_number: i32) -> Option<Duration> {
        select_field(baseline_work_fields(), baseline_number)
            .and_then(|field| self.duration_field(field))
    }

/// Get enterprise custom cost field.
    ///
    /// Corresponds to Java: `public Number getEnterpriseCost(int index)`
    pub fn get_enterprise_cost(&self, index: i32) -> Option<f64> {
        select_field(enterprise_cost_fields(), index).and_then(|field| self.number_field(field))
    }

/// Set enterprise custom cost field.
    ///
    /// Corresponds to Java: `public void setEnterpriseCost(int index, Number value)`
    pub fn set_enterprise_cost(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = select_field(enterprise_cost_fields(), index) {
            self.put_number(field, value);
        }
    }

/// Get enterprise custom date field.
    ///
    /// Corresponds to Java: `public LocalDateTime getEnterpriseDate(int index)`
    pub fn get_enterprise_date(&self, index: i32) -> Option<NaiveDateTime> {
        select_field(enterprise_date_fields(), index).and_then(|field| self.date_time_field(field))
    }

/// Set enterprise custom date field.
    ///
    /// Corresponds to Java: `public void setEnterpriseDate(int index, LocalDateTime value)`
    pub fn set_enterprise_date(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(enterprise_date_fields(), index) {
            self.put_date_time(field, value);
        }
    }

/// Get enterprise custom duration field.
    ///
    /// Corresponds to Java: `public Duration getEnterpriseDuration(int index)`
    pub fn get_enterprise_duration(&self, index: i32) -> Option<Duration> {
        select_field(enterprise_duration_fields(), index)
            .and_then(|field| self.duration_field(field))
    }

/// Set enterprise custom duration field.
    ///
    /// Corresponds to Java: `public void setEnterpriseDuration(int index, Duration value)`
    pub fn set_enterprise_duration(&mut self, index: i32, value: Option<Duration>) {
        if let Some(field) = select_field(enterprise_duration_fields(), index) {
            self.put_duration(field, value);
        }
    }

/// Get enterprise custom flag field.
    ///
    /// Corresponds to Java: `public boolean getEnterpriseFlag(int index)`
    pub fn get_enterprise_flag(&self, index: i32) -> bool {
        select_field(enterprise_flag_fields(), index).map_or(false, |field| self.boolean_field(field))
    }

/// Set enterprise custom flag field.
    ///
    /// Corresponds to Java: `public void setEnterpriseFlag(int index, boolean value)`
    pub fn set_enterprise_flag(&mut self, index: i32, value: bool) {
        if let Some(field) = select_field(enterprise_flag_fields(), index) {
            self.put_bool(field, value);
        }
    }

/// Get enterprise custom number field.
    ///
    /// Corresponds to Java: `public Number getEnterpriseNumber(int index)`
    pub fn get_enterprise_number(&self, index: i32) -> Option<f64> {
        select_field(enterprise_number_fields(), index).and_then(|field| self.number_field(field))
    }

/// Set enterprise custom number field.
    ///
    /// Corresponds to Java: `public void setEnterpriseNumber(int index, Number value)`
    pub fn set_enterprise_number(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = select_field(enterprise_number_fields(), index) {
            self.put_number(field, value);
        }
    }

/// Get enterprise custom text field.
    ///
    /// Corresponds to Java: `public String getEnterpriseText(int index)`
    pub fn get_enterprise_text(&self, index: i32) -> Option<String> {
        select_field(enterprise_text_fields(), index).and_then(|field| self.string_field(field))
    }

/// Set enterprise custom text field.
    ///
    /// Corresponds to Java: `public void setEnterpriseText(int index, String value)`
    pub fn set_enterprise_text(&mut self, index: i32, value: Option<String>) {
        if let Some(field) = select_field(enterprise_text_fields(), index) {
            self.put_string(field, value);
        }
    }

/// Set baseline fixed cost.
    ///
    /// Corresponds to Java: `public void setBaselineFixedCost(Number val)`
    pub fn set_baseline_fixed_cost(&mut self, val: Option<f64>) {
        self.put_number(TaskField::BaselineFixedCost, val);
    }

/// Get baseline fixed cost.
    ///
    /// Corresponds to Java: `public Number getBaselineFixedCost()`
    pub fn get_baseline_fixed_cost(&self) -> Option<f64> {
        self.number_field(TaskField::BaselineFixedCost)
    }

/// Set baseline fixed cost (multiple baselines).
    ///
    /// Corresponds to Java: `public void setBaselineFixedCost(int baselineNumber, Number value)`
    pub fn set_baseline_fixed_cost_at(&mut self, baseline_number: i32, value: Option<f64>) {
        if let Some(field) = select_field(baseline_fixed_cost_fields(), baseline_number) {
            self.put_number(field, value);
        }
    }

/// Get baseline fixed cost (multiple baselines).
    ///
    /// Corresponds to Java: `public Number getBaselineFixedCost(int baselineNumber)`
    pub fn get_baseline_fixed_cost_at(&self, baseline_number: i32) -> Option<f64> {
        select_field(baseline_fixed_cost_fields(), baseline_number)
            .and_then(|field| self.number_field(field))
    }

/// Get baseline fixed cost accrual method.
    ///
    /// Corresponds to Java: `public AccrueType getBaselineFixedCostAccrual()`
    pub fn get_baseline_fixed_cost_accrual(&self) -> Option<AccrueType> {
        self.integer_field(TaskField::BaselineFixedCostAccrual)
            .and_then(AccrueType::get_instance)
    }

/// Set baseline fixed cost accrual method.
    ///
    /// Corresponds to Java: `public void setBaselineFixedCostAccrual(AccrueType type)`
    pub fn set_baseline_fixed_cost_accrual(&mut self, r#type: Option<AccrueType>) {
        self.put_enum(
            TaskField::BaselineFixedCostAccrual,
            r#type.map(|value| value.value()),
        );
    }

/// Set baseline fixed cost accrual method (multiple baselines).
    ///
    /// Corresponds to Java: `public void setBaselineFixedCostAccrual(int baselineNumber, AccrueType value)`
    pub fn set_baseline_fixed_cost_accrual_at(
        &mut self,
        baseline_number: i32,
        value: Option<AccrueType>,
    ) {
        if let Some(field) = select_field(baseline_fixed_cost_accrual_fields(), baseline_number) {
            self.put_enum(field, value.map(|value| value.value()));
        }
    }

/// Get baseline fixed cost accrual method (multiple baselines).
    ///
    /// Corresponds to Java: `public AccrueType getBaselineFixedCostAccrual(int baselineNumber)`
    pub fn get_baseline_fixed_cost_accrual_at(&self, baseline_number: i32) -> Option<AccrueType> {
        select_field(baseline_fixed_cost_accrual_fields(), baseline_number)
            .and_then(|field| self.integer_field(field))
            .and_then(AccrueType::get_instance)
    }

/// Get baseline budget cost.
    ///
    /// Corresponds to Java: `public Number getBaselineBudgetCost()`
    pub fn get_baseline_budget_cost(&self) -> Option<f64> {
        self.number_field(TaskField::BaselineBudgetCost)
    }

/// Set baseline budget cost.
    ///
    /// Corresponds to Java: `public void setBaselineBudgetCost(Number value)`
    pub fn set_baseline_budget_cost(&mut self, value: Option<f64>) {
        self.put_number(TaskField::BaselineBudgetCost, value);
    }

/// Get baseline budget work.
    ///
    /// Corresponds to Java: `public Duration getBaselineBudgetWork()`
    pub fn get_baseline_budget_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::BaselineBudgetWork)
    }

/// Set baseline budget work.
    ///
    /// Corresponds to Java: `public void setBaselineBudgetWork(Duration value)`
    pub fn set_baseline_budget_work(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::BaselineBudgetWork, value);
    }

/// Get baseline budget cost (multiple baselines).
    ///
    /// Corresponds to Java: `public Number getBaselineBudgetCost(int baselineNumber)`
    pub fn get_baseline_budget_cost_at(&self, baseline_number: i32) -> Option<f64> {
        select_field(baseline_budget_cost_fields(), baseline_number)
            .and_then(|field| self.number_field(field))
    }

/// Set baseline budget cost (multiple baselines).
    ///
    /// Corresponds to Java: `public void setBaselineBudgetCost(int baselineNumber, Number value)`
    pub fn set_baseline_budget_cost_at(&mut self, baseline_number: i32, value: Option<f64>) {
        if let Some(field) = select_field(baseline_budget_cost_fields(), baseline_number) {
            self.put_number(field, value);
        }
    }

/// Get baseline budget work (multiple baselines).
    ///
    /// Corresponds to Java: `public Duration getBaselineBudgetWork(int baselineNumber)`
    pub fn get_baseline_budget_work_at(&self, baseline_number: i32) -> Option<Duration> {
        select_field(baseline_budget_work_fields(), baseline_number)
            .and_then(|field| self.duration_field(field))
    }

/// Set baseline budget work (multiple baselines).
    ///
    /// Corresponds to Java: `public void setBaselineBudgetWork(int baselineNumber, Duration value)`
    pub fn set_baseline_budget_work_at(&mut self, baseline_number: i32, value: Option<Duration>) {
        if let Some(field) = select_field(baseline_budget_work_fields(), baseline_number) {
            self.put_duration(field, value);
        }
    }
}

// ---------------------------------------------------------------------------
// Baseline estimate values / percent complete / GUID / task mode
// ---------------------------------------------------------------------------

impl Task {
/// Get baseline estimated duration.
    ///
    /// Corresponds to Java: `public Duration getBaselineEstimatedDuration()`
    pub fn get_baseline_estimated_duration(&self) -> Option<Duration> {
        self.duration_field(TaskField::BaselineEstimatedDuration)
    }

/// Set baseline estimated duration.
    ///
    /// Corresponds to Java: `public void setBaselineEstimatedDuration(Duration duration)`
    pub fn set_baseline_estimated_duration(&mut self, duration: Option<Duration>) {
        self.put_duration(TaskField::BaselineEstimatedDuration, duration);
    }

/// Set baseline estimated duration (multiple baselines).
    ///
    /// Corresponds to Java: `public void setBaselineEstimatedDuration(int baselineNumber, Duration value)`
    pub fn set_baseline_estimated_duration_at(&mut self, baseline_number: i32, value: Option<Duration>) {
        if let Some(field) = select_field(baseline_estimated_duration_fields(), baseline_number) {
            self.put_duration(field, value);
        }
    }

/// Get baseline estimated duration (multiple baselines).
    ///
    /// Corresponds to Java: `public Duration getBaselineEstimatedDuration(int baselineNumber)`
    pub fn get_baseline_estimated_duration_at(&self, baseline_number: i32) -> Option<Duration> {
        select_field(baseline_estimated_duration_fields(), baseline_number)
            .and_then(|field| self.duration_field(field))
    }

/// Get baseline estimated start date.
    ///
    /// Corresponds to Java: `public LocalDateTime getBaselineEstimatedStart()`
    pub fn get_baseline_estimated_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::BaselineEstimatedStart)
    }

/// Set baseline estimated start date.
    ///
    /// Corresponds to Java: `public void setBaselineEstimatedStart(LocalDateTime date)`
    pub fn set_baseline_estimated_start(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::BaselineEstimatedStart, date);
    }

/// Get baseline estimated start date (multiple baselines).
    ///
    /// Corresponds to Java: `public LocalDateTime getBaselineEstimatedStart(int baselineNumber)`
    pub fn get_baseline_estimated_start_at(&self, baseline_number: i32) -> Option<NaiveDateTime> {
        select_field(baseline_estimated_start_fields(), baseline_number)
            .and_then(|field| self.date_time_field(field))
    }

/// Set baseline estimated start date (multiple baselines).
    ///
    /// Corresponds to Java: `public void setBaselineEstimatedStart(int baselineNumber, LocalDateTime value)`
    pub fn set_baseline_estimated_start_at(&mut self, baseline_number: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(baseline_estimated_start_fields(), baseline_number) {
            self.put_date_time(field, value);
        }
    }

/// Get baseline estimated finish date.
    ///
    /// Corresponds to Java: `public LocalDateTime getBaselineEstimatedFinish()`
    pub fn get_baseline_estimated_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::BaselineEstimatedFinish)
    }

/// Set baseline estimated finish date.
    ///
    /// Corresponds to Java: `public void setBaselineEstimatedFinish(LocalDateTime date)`
    pub fn set_baseline_estimated_finish(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::BaselineEstimatedFinish, date);
    }

/// Get baseline estimated finish date (multiple baselines).
    ///
    /// Corresponds to Java: `public LocalDateTime getBaselineEstimatedFinish(int baselineNumber)`
    pub fn get_baseline_estimated_finish_at(&self, baseline_number: i32) -> Option<NaiveDateTime> {
        select_field(baseline_estimated_finish_fields(), baseline_number)
            .and_then(|field| self.date_time_field(field))
    }

/// Set baseline estimated finish date (multiple baselines).
    ///
    /// Corresponds to Java: `public void setBaselineEstimatedFinish(int baselineNumber, LocalDateTime value)`
    pub fn set_baseline_estimated_finish_at(&mut self, baseline_number: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(baseline_estimated_finish_fields(), baseline_number) {
            self.put_date_time(field, value);
        }
    }

/// Get the "complete through" date.
    ///
    /// Corresponds to Java: `public LocalDateTime getCompleteThrough()`
    pub fn get_complete_through(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::CompleteThrough)
    }

/// Set the "complete through" date.
    ///
    /// Corresponds to Java: `public void setCompleteThrough(LocalDateTime value)`
    pub fn set_complete_through(&mut self, value: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::CompleteThrough, value);
    }

/// Get the summary progress date.
    ///
    /// Corresponds to Java: `public LocalDateTime getSummaryProgress()`
    pub fn get_summary_progress(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::SummaryProgress)
    }

/// Set the summary progress date.
    ///
    /// Corresponds to Java: `public void setSummaryProgress(LocalDateTime value)`
    pub fn set_summary_progress(&mut self, value: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::SummaryProgress, value);
    }

/// Get the task GUID.
    ///
    /// Corresponds to Java: `public UUID getGUID()`
    pub fn get_guid(&self) -> Option<Uuid> {
        self.uuid_field(TaskField::Guid)
    }

/// Set the task GUID.
    ///
    /// Corresponds to Java: `public void setGUID(UUID value)`
    pub fn set_guid(&mut self, value: Option<Uuid>) {
        self.put_uuid(TaskField::Guid, value);
    }

/// Get the task mode.
    ///
    /// Corresponds to Java: `public TaskMode getTaskMode()`
    pub fn get_task_mode(&self) -> Option<TaskMode> {
        self.integer_field(TaskField::TaskMode)
            .map(TaskMode::get_instance)
    }

/// Set the task mode.
    ///
    /// Corresponds to Java: `public void setTaskMode(TaskMode mode)`
    pub fn set_task_mode(&mut self, mode: Option<TaskMode>) {
        self.put_enum(TaskField::TaskMode, mode.map(|value| value.value()));
    }

/// Get the active flag.
    ///
    /// Corresponds to Java: `public boolean getActive()`
    pub fn get_active(&self) -> bool {
        self.boolean_field(TaskField::Active)
    }

/// Set the active flag.
    ///
    /// Corresponds to Java: `public void setActive(boolean active)`
    pub fn set_active(&mut self, active: bool) {
        self.put_bool(TaskField::Active, active);
    }

/// Get the "canonical" version of the activity ID (summary tasks replace the project ID prefix with `PROJECT`).
    ///
    /// Corresponds to Java: `public String getCanonicalActivityID()`
    pub fn get_canonical_activity_id(&self) -> Option<String> {
        let activity_id = self.get_activity_id();
        if !self.get_summary() {
            return activity_id;
        }

        let Some(activity_id) = activity_id else {
            return None;
        };

        let project_id = self
            .parent_file
            .upgrade()
            .and_then(|file| file.borrow().get_project_properties().get_project_id());

        match project_id {
            Some(project_id) if !project_id.is_empty() && activity_id.starts_with(&project_id) => {
                Some(format!("PROJECT{}", &activity_id[project_id.len()..]))
            }
            _ => Some(activity_id),
        }
    }

/// Get the activity ID.
    ///
    /// Corresponds to Java: `public String getActivityID()`
    pub fn get_activity_id(&self) -> Option<String> {
        self.string_field(TaskField::ActivityId)
    }

/// Set the activity ID.
    ///
    /// Corresponds to Java: `public void setActivityID(String value)`
    pub fn set_activity_id(&mut self, value: Option<String>) {
        self.put_string(TaskField::ActivityId, value);
    }

/// Set the percent complete type.
    ///
    /// Corresponds to Java: `public void setPercentCompleteType(PercentCompleteType value)`
    pub fn set_percent_complete_type(&mut self, value: Option<PercentCompleteType>) {
        self.put_enum(
            TaskField::PercentCompleteType,
            value.map(|value| value.value()),
        );
    }

/// Get the percent complete type.
    ///
    /// Corresponds to Java: `public PercentCompleteType getPercentCompleteType()`
    pub fn get_percent_complete_type(&self) -> Option<PercentCompleteType> {
        self.integer_field(TaskField::PercentCompleteType)
            .and_then(PercentCompleteType::get_instance)
    }

/// Get the activity status.
    ///
    /// Corresponds to Java: `public ActivityStatus getActivityStatus()`
    pub fn get_activity_status(&self) -> Option<ActivityStatus> {
        self.integer_field(TaskField::ActivityStatus)
            .and_then(ActivityStatus::get_instance)
    }

/// Set the activity status.
    ///
    /// Corresponds to Java: `public void setActivityStatus(ActivityStatus value)`
    pub fn set_activity_status(&mut self, value: Option<ActivityStatus>) {
        self.put_enum(TaskField::ActivityStatus, value.map(|value| value.value()));
    }

/// Get the activity type.
    ///
    /// Corresponds to Java: `public ActivityType getActivityType()`
    pub fn get_activity_type(&self) -> Option<ActivityType> {
        self.integer_field(TaskField::ActivityType)
            .and_then(ActivityType::get_instance)
    }

/// Set the activity type.
    ///
    /// Corresponds to Java: `public void setActivityType(ActivityType value)`
    pub fn set_activity_type(&mut self, value: Option<ActivityType>) {
        self.put_enum(TaskField::ActivityType, value.map(|value| value.value()));
    }

/// Get the "longest path" flag.
    ///
    /// Corresponds to Java: `public boolean getLongestPath()`
    pub fn get_longest_path(&self) -> bool {
        self.boolean_field(TaskField::LongestPath)
    }

/// Set the "longest path" flag.
    ///
    /// Corresponds to Java: `public void setLongestPath(boolean value)`
    pub fn set_longest_path(&mut self, value: bool) {
        self.put_bool(TaskField::LongestPath, value);
    }

/// Get the external early start date.
    ///
    /// Corresponds to Java: `public LocalDateTime getExternalEarlyStart()`
    pub fn get_external_early_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::ExternalEarlyStart)
    }

/// Set the external early start date.
    ///
    /// Corresponds to Java: `public void setExternalEarlyStart(LocalDateTime value)`
    pub fn set_external_early_start(&mut self, value: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::ExternalEarlyStart, value);
    }

/// Get the external late finish date.
    ///
    /// Corresponds to Java: `public LocalDateTime getExternalLateFinish()`
    pub fn get_external_late_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::ExternalLateFinish)
    }

/// Set the external late finish date.
    ///
    /// Corresponds to Java: `public void setExternalLateFinish(LocalDateTime value)`
    pub fn set_external_late_finish(&mut self, value: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::ExternalLateFinish, value);
    }

/// Get the Sprint ID.
    ///
    /// Corresponds to Java: `public Integer getSprintID()`
    pub fn get_sprint_id(&self) -> Option<i32> {
        self.integer_field(TaskField::SprintId)
    }

/// Set the Sprint ID.
    ///
    /// Corresponds to Java: `public void setSprintID(Integer value)`
    pub fn set_sprint_id(&mut self, value: Option<i32>) {
        self.put_integer(TaskField::SprintId, value);
    }

/// Get the board status ID.
    ///
    /// Corresponds to Java: `public Integer getBoardStatusID()`
    pub fn get_board_status_id(&self) -> Option<i32> {
        self.integer_field(TaskField::BoardStatusId)
    }

/// Set the board status ID.
    ///
    /// Corresponds to Java: `public void setBoardStatusID(Integer value)`
    pub fn set_board_status_id(&mut self, value: Option<i32>) {
        self.put_integer(TaskField::BoardStatusId, value);
    }

/// Get the "response pending" flag.
    ///
    /// Corresponds to Java: `public boolean getResponsePending()`
    pub fn get_response_pending(&self) -> bool {
        self.boolean_field(TaskField::ResponsePending)
    }

/// Set the "response pending" flag.
    ///
    /// Corresponds to Java: `public void setResponsePending(boolean value)`
    pub fn set_response_pending(&mut self, value: bool) {
        self.put_bool(TaskField::ResponsePending, value);
    }

/// Get the planned start date.
    ///
    /// Corresponds to Java: `public LocalDateTime getScheduledStart()`
    pub fn get_scheduled_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::ScheduledStart)
    }

/// Set the planned start date.
    ///
    /// Corresponds to Java: `public void setScheduledStart(LocalDateTime value)`
    pub fn set_scheduled_start(&mut self, value: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::ScheduledStart, value);
    }

/// Get the planned finish date.
    ///
    /// Corresponds to Java: `public LocalDateTime getScheduledFinish()`
    pub fn get_scheduled_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::ScheduledFinish)
    }

/// Set the planned finish date.
    ///
    /// Corresponds to Java: `public void setScheduledFinish(LocalDateTime value)`
    pub fn set_scheduled_finish(&mut self, value: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::ScheduledFinish, value);
    }

/// Get the planned duration.
    ///
    /// Corresponds to Java: `public Duration getScheduledDuration()`
    pub fn get_scheduled_duration(&self) -> Option<Duration> {
        self.duration_field(TaskField::ScheduledDuration)
    }

/// Set the planned duration.
    ///
    /// Corresponds to Java: `public void setScheduledDuration(Duration value)`
    pub fn set_scheduled_duration(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::ScheduledDuration, value);
    }

/// Get the budget cost.
    ///
    /// Corresponds to Java: `public Number getBudgetCost()`
    pub fn get_budget_cost(&self) -> Option<f64> {
        self.number_field(TaskField::BudgetCost)
    }

/// Set the budget cost.
    ///
    /// Corresponds to Java: `public void setBudgetCost(Number value)`
    pub fn set_budget_cost(&mut self, value: Option<f64>) {
        self.put_number(TaskField::BudgetCost, value);
    }

/// Get the budget work.
    ///
    /// Corresponds to Java: `public Duration getBudgetWork()`
    pub fn get_budget_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::BudgetWork)
    }

/// Set the budget work.
    ///
    /// Corresponds to Java: `public void setBudgetWork(Duration value)`
    pub fn set_budget_work(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::BudgetWork, value);
    }

/// Set the task sequence number.
    ///
    /// Corresponds to Java: `public void setSequenceNumber(Integer sequenceNumber)`
    pub fn set_sequence_number(&mut self, sequence_number: Option<i32>) {
        self.put_integer(TaskField::SequenceNumber, sequence_number);
    }

/// Get the task sequence number.
    ///
    /// Corresponds to Java: `public Integer getSequenceNumber()`
    pub fn get_sequence_number(&self) -> Option<i32> {
        self.integer_field(TaskField::SequenceNumber)
    }

/// Get the location unique ID.
    ///
    /// Corresponds to Java: `public Integer getLocationUniqueID()`
    pub fn get_location_unique_id(&self) -> Option<i32> {
        self.integer_field(TaskField::LocationUniqueId)
    }

/// Set the location unique ID.
    ///
    /// Corresponds to Java: `public void setLocationUniqueID(Integer uniqueID)`
    pub fn set_location_unique_id(&mut self, unique_id: Option<i32>) {
        self.put_integer(TaskField::LocationUniqueId, unique_id);
    }

/// Get the Asta Powerproject bar name.
    ///
    /// Corresponds to Java: `public String getBarName()`
    pub fn get_bar_name(&self) -> Option<String> {
        self.string_field(TaskField::BarName)
    }

/// Set the Asta Powerproject bar name.
    ///
    /// Corresponds to Java: `public void setBarName(String value)`
    pub fn set_bar_name(&mut self, value: Option<String>) {
        self.put_string(TaskField::BarName, value);
    }

/// Get the expected finish date.
    ///
    /// Corresponds to Java: `public LocalDateTime getExpectedFinish()`
    pub fn get_expected_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::ExpectedFinish)
    }

/// Set the expected finish date.
    ///
    /// Corresponds to Java: `public void setExpectedFinish(LocalDateTime value)`
    pub fn set_expected_finish(&mut self, value: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::ExpectedFinish, value);
    }
}

// ---------------------------------------------------------------------------
// Work / cost breakdown (labor, non-labor, material, expense)
// ---------------------------------------------------------------------------

impl Task {
/// Set the labor portion of actual work.
    ///
    /// Corresponds to Java: `public void setActualWorkLabor(Duration value)`
    pub fn set_actual_work_labor(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::ActualWorkLabor, value);
    }

/// Get the labor portion of actual work.
    ///
    /// Corresponds to Java: `public Duration getActualWorkLabor()`
    pub fn get_actual_work_labor(&self) -> Option<Duration> {
        self.duration_field(TaskField::ActualWorkLabor)
    }

/// Set the non-labor portion of actual work (deprecated legacy Java method name).
    ///
    /// Corresponds to Java: `@Deprecated public void setActualWorkNonlabor(Duration value)`
    pub fn set_actual_work_nonlabor(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::ActualWorkNonlabor, value);
    }

/// Set the non-labor portion of actual work.
    ///
    /// Corresponds to Java: `public void setActualWorkNonLabor(Duration value)`
    pub fn set_actual_work_non_labor(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::ActualWorkNonlabor, value);
    }

/// Get the non-labor portion of actual work (deprecated legacy Java method name).
    ///
    /// Corresponds to Java: `@Deprecated public Duration getActualWorkNonlabor()`
    pub fn get_actual_work_nonlabor(&self) -> Option<Duration> {
        self.duration_field(TaskField::ActualWorkNonlabor)
    }

/// Get the non-labor portion of actual work.
    ///
    /// Corresponds to Java: `public Duration getActualWorkNonLabor()`
    pub fn get_actual_work_non_labor(&self) -> Option<Duration> {
        self.duration_field(TaskField::ActualWorkNonlabor)
    }

/// Set the labor portion of planned work.
    ///
    /// Corresponds to Java: `public void setPlannedWorkLabor(Duration value)`
    pub fn set_planned_work_labor(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::PlannedWorkLabor, value);
    }

/// Get the labor portion of planned work.
    ///
    /// Corresponds to Java: `public Duration getPlannedWorkLabor()`
    pub fn get_planned_work_labor(&self) -> Option<Duration> {
        self.duration_field(TaskField::PlannedWorkLabor)
    }

/// Set the non-labor portion of planned work (deprecated legacy Java method name).
    ///
    /// Corresponds to Java: `@Deprecated public void setPlannedWorkNonlabor(Duration value)`
    pub fn set_planned_work_nonlabor(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::PlannedWorkNonlabor, value);
    }

/// Set the non-labor portion of planned work.
    ///
    /// Corresponds to Java: `public void setPlannedWorkNonLabor(Duration value)`
    pub fn set_planned_work_non_labor(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::PlannedWorkNonlabor, value);
    }

/// Get the non-labor portion of planned work (deprecated legacy Java method name).
    ///
    /// Corresponds to Java: `@Deprecated public Duration getPlannedWorkNonlabor()`
    pub fn get_planned_work_nonlabor(&self) -> Option<Duration> {
        self.duration_field(TaskField::PlannedWorkNonlabor)
    }

/// Get the non-labor portion of planned work.
    ///
    /// Corresponds to Java: `public Duration getPlannedWorkNonLabor()`
    pub fn get_planned_work_non_labor(&self) -> Option<Duration> {
        self.duration_field(TaskField::PlannedWorkNonlabor)
    }

/// Set the labor portion of remaining work.
    ///
    /// Corresponds to Java: `public void setRemainingWorkLabor(Duration value)`
    pub fn set_remaining_work_labor(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::RemainingWorkLabor, value);
    }

/// Get the labor portion of remaining work.
    ///
    /// Corresponds to Java: `public Duration getRemainingWorkLabor()`
    pub fn get_remaining_work_labor(&self) -> Option<Duration> {
        self.duration_field(TaskField::RemainingWorkLabor)
    }

/// Set the non-labor portion of remaining work (deprecated legacy Java method name).
    ///
    /// Corresponds to Java: `@Deprecated public void setRemainingWorkNonlabor(Duration value)`
    pub fn set_remaining_work_nonlabor(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::RemainingWorkNonlabor, value);
    }

/// Set the non-labor portion of remaining work.
    ///
    /// Corresponds to Java: `public void setRemainingWorkNonLabor(Duration value)`
    pub fn set_remaining_work_non_labor(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::RemainingWorkNonlabor, value);
    }

/// Get the non-labor portion of remaining work (deprecated legacy Java method name).
    ///
    /// Corresponds to Java: `@Deprecated public Duration getRemainingWorkNonlabor()`
    pub fn get_remaining_work_nonlabor(&self) -> Option<Duration> {
        self.duration_field(TaskField::RemainingWorkNonlabor)
    }

/// Get the non-labor portion of remaining work.
    ///
    /// Corresponds to Java: `public Duration getRemainingWorkNonLabor()`
    pub fn get_remaining_work_non_labor(&self) -> Option<Duration> {
        self.duration_field(TaskField::RemainingWorkNonlabor)
    }

/// Determine whether a manually scheduled task should display start text.
    ///
    /// Corresponds to Java: `public boolean getShowStartText()`
    pub fn get_show_start_text(&self) -> bool {
        self.boolean_field(TaskField::ShowStartText)
    }

/// Determine whether a manually scheduled task should display finish text.
    ///
    /// Corresponds to Java: `public boolean getShowFinishText()`
    pub fn get_show_finish_text(&self) -> bool {
        self.boolean_field(TaskField::ShowFinishText)
    }

/// Determine whether a manually scheduled task should display duration text.
    ///
    /// Corresponds to Java: `public boolean getShowDurationText()`
    pub fn get_show_duration_text(&self) -> bool {
        self.boolean_field(TaskField::ShowDurationText)
    }

/// Get the percent complete determined by the percent complete type.
    ///
    /// Corresponds to Java: `public Number getActivityPercentComplete()`
    pub fn get_activity_percent_complete(&self) -> Option<f64> {
        self.number_field(TaskField::ActivityPercentComplete)
    }

/// Get the methodology GUID.
    ///
    /// Corresponds to Java: `public UUID getMethodologyGUID()`
    pub fn get_methodology_guid(&self) -> Option<Uuid> {
        self.uuid_field(TaskField::MethodologyGuid)
    }

/// Set the methodology GUID.
    ///
    /// Corresponds to Java: `public void setMethodologyGUID(UUID value)`
    pub fn set_methodology_guid(&mut self, value: Option<Uuid>) {
        self.put_uuid(TaskField::MethodologyGuid, value);
    }

/// Get the float path number.
    ///
    /// Corresponds to Java: `public Integer getFloatPath()`
    pub fn get_float_path(&self) -> Option<i32> {
        self.integer_field(TaskField::FloatPath)
    }

/// Set the float path number.
    ///
    /// Corresponds to Java: `public void setFloatPath(Integer value)`
    pub fn set_float_path(&mut self, value: Option<i32>) {
        self.put_integer(TaskField::FloatPath, value);
    }

/// Get the float path order.
    ///
    /// Corresponds to Java: `public Integer getFloatPathOrder()`
    pub fn get_float_path_order(&self) -> Option<i32> {
        self.integer_field(TaskField::FloatPathOrder)
    }

/// Set the float path order.
    ///
    /// Corresponds to Java: `public void setFloatPathOrder(Integer value)`
    pub fn set_float_path_order(&mut self, value: Option<i32>) {
        self.put_integer(TaskField::FloatPathOrder, value);
    }

/// Get the original budget.
    ///
    /// Corresponds to Java: `public Number getOriginalBudget()`
    pub fn get_original_budget(&self) -> Option<f64> {
        self.number_field(TaskField::OriginalBudget)
    }

/// Set the original budget.
    ///
    /// Corresponds to Java: `public void setOriginalBudget(Number originalBudget)`
    pub fn set_original_budget(&mut self, original_budget: Option<f64>) {
        self.put_number(TaskField::OriginalBudget, original_budget);
    }

/// Get the schedule percent complete.
    ///
    /// Corresponds to Java: `public Number getSchedulePercentComplete()`
    pub fn get_schedule_percent_complete(&self) -> Option<f64> {
        self.number_field(TaskField::SchedulePercentComplete)
    }

/// Get the effective calendar: fall back to the project default calendar when the task's own calendar is empty.
    ///
    /// Corresponds to Java: `public ProjectCalendar getEffectiveCalendar()`
    pub fn get_effective_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        if let Some(calendar) = self.get_calendar() {
            return Some(calendar);
        }

        self.parent_file
            .upgrade()
            .and_then(|file| file.borrow().get_default_calendar())
    }

/// Get actual regular work.
    ///
    /// Corresponds to Java: `public Duration getActualRegularWork()`
    pub fn get_actual_regular_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::ActualRegularWork)
    }

/// Get remaining regular work.
    ///
    /// Corresponds to Java: `public Duration getRemainingRegularWork()`
    pub fn get_remaining_regular_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::RemainingRegularWork)
    }

/// Get actual regular cost.
    ///
    /// Corresponds to Java: `public Number getActualRegularCost()`
    pub fn get_actual_regular_cost(&self) -> Option<f64> {
        self.number_field(TaskField::ActualRegularCost)
    }

/// Get remaining regular cost.
    ///
    /// Corresponds to Java: `public Number getRemainingRegularCost()`
    pub fn get_remaining_regular_cost(&self) -> Option<f64> {
        self.number_field(TaskField::RemainingRegularCost)
    }

/// Set the labor portion of planned cost.
    ///
    /// Corresponds to Java: `public void setPlannedCostLabor(Number value)`
    pub fn set_planned_cost_labor(&mut self, value: Option<f64>) {
        self.put_number(TaskField::PlannedCostLabor, value);
    }

/// Get the labor portion of planned cost.
    ///
    /// Corresponds to Java: `public Number getPlannedCostLabor()`
    pub fn get_planned_cost_labor(&self) -> Option<f64> {
        self.number_field(TaskField::PlannedCostLabor)
    }

/// Set the non-labor portion of planned cost.
    ///
    /// Corresponds to Java: `public void setPlannedCostNonLabor(Number value)`
    pub fn set_planned_cost_non_labor(&mut self, value: Option<f64>) {
        self.put_number(TaskField::PlannedCostNonLabor, value);
    }

/// Get the non-labor portion of planned cost.
    ///
    /// Corresponds to Java: `public Number getPlannedCostNonLabor()`
    pub fn get_planned_cost_non_labor(&self) -> Option<f64> {
        self.number_field(TaskField::PlannedCostNonLabor)
    }

/// Set the material portion of planned cost.
    ///
    /// Corresponds to Java: `public void setPlannedCostMaterial(Number value)`
    pub fn set_planned_cost_material(&mut self, value: Option<f64>) {
        self.put_number(TaskField::PlannedCostMaterial, value);
    }

/// Get the material portion of planned cost.
    ///
    /// Corresponds to Java: `public Number getPlannedCostMaterial()`
    pub fn get_planned_cost_material(&self) -> Option<f64> {
        self.number_field(TaskField::PlannedCostMaterial)
    }

/// Set the expense portion of planned cost.
    ///
    /// Corresponds to Java: `public void setPlannedCostExpense(Number value)`
    pub fn set_planned_cost_expense(&mut self, value: Option<f64>) {
        self.put_number(TaskField::PlannedCostExpense, value);
    }

/// Get the expense portion of planned cost.
    ///
    /// Corresponds to Java: `public Number getPlannedCostExpense()`
    pub fn get_planned_cost_expense(&self) -> Option<f64> {
        self.number_field(TaskField::PlannedCostExpense)
    }

/// Set the labor portion of actual cost.
    ///
    /// Corresponds to Java: `public void setActualCostLabor(Number value)`
    pub fn set_actual_cost_labor(&mut self, value: Option<f64>) {
        self.put_number(TaskField::ActualCostLabor, value);
    }

/// Get the labor portion of actual cost.
    ///
    /// Corresponds to Java: `public Number getActualCostLabor()`
    pub fn get_actual_cost_labor(&self) -> Option<f64> {
        self.number_field(TaskField::ActualCostLabor)
    }

/// Set the non-labor portion of actual cost.
    ///
    /// Corresponds to Java: `public void setActualCostNonLabor(Number value)`
    pub fn set_actual_cost_non_labor(&mut self, value: Option<f64>) {
        self.put_number(TaskField::ActualCostNonLabor, value);
    }

/// Get the non-labor portion of actual cost.
    ///
    /// Corresponds to Java: `public Number getActualCostNonLabor()`
    pub fn get_actual_cost_non_labor(&self) -> Option<f64> {
        self.number_field(TaskField::ActualCostNonLabor)
    }

/// Set the material portion of actual cost.
    ///
    /// Corresponds to Java: `public void setActualCostMaterial(Number value)`
    pub fn set_actual_cost_material(&mut self, value: Option<f64>) {
        self.put_number(TaskField::ActualCostMaterial, value);
    }

/// Get the material portion of actual cost.
    ///
    /// Corresponds to Java: `public Number getActualCostMaterial()`
    pub fn get_actual_cost_material(&self) -> Option<f64> {
        self.number_field(TaskField::ActualCostMaterial)
    }

/// Set the expense portion of actual cost.
    ///
    /// Corresponds to Java: `public void setActualCostExpense(Number value)`
    pub fn set_actual_cost_expense(&mut self, value: Option<f64>) {
        self.put_number(TaskField::ActualCostExpense, value);
    }

/// Get the expense portion of actual cost.
    ///
    /// Corresponds to Java: `public Number getActualCostExpense()`
    pub fn get_actual_cost_expense(&self) -> Option<f64> {
        self.number_field(TaskField::ActualCostExpense)
    }

/// Set the labor portion of remaining cost.
    ///
    /// Corresponds to Java: `public void setRemainingCostLabor(Number value)`
    pub fn set_remaining_cost_labor(&mut self, value: Option<f64>) {
        self.put_number(TaskField::RemainingCostLabor, value);
    }

/// Get the labor portion of remaining cost.
    ///
    /// Corresponds to Java: `public Number getRemainingCostLabor()`
    pub fn get_remaining_cost_labor(&self) -> Option<f64> {
        self.number_field(TaskField::RemainingCostLabor)
    }

/// Set the non-labor portion of remaining cost.
    ///
    /// Corresponds to Java: `public void setRemainingCostNonLabor(Number value)`
    pub fn set_remaining_cost_non_labor(&mut self, value: Option<f64>) {
        self.put_number(TaskField::RemainingCostNonLabor, value);
    }

/// Get the non-labor portion of remaining cost.
    ///
    /// Corresponds to Java: `public Number getRemainingCostNonLabor()`
    pub fn get_remaining_cost_non_labor(&self) -> Option<f64> {
        self.number_field(TaskField::RemainingCostNonLabor)
    }

/// Set the material portion of remaining cost.
    ///
    /// Corresponds to Java: `public void setRemainingCostMaterial(Number value)`
    pub fn set_remaining_cost_material(&mut self, value: Option<f64>) {
        self.put_number(TaskField::RemainingCostMaterial, value);
    }

/// Get the material portion of remaining cost.
    ///
    /// Corresponds to Java: `public Number getRemainingCostMaterial()`
    pub fn get_remaining_cost_material(&self) -> Option<f64> {
        self.number_field(TaskField::RemainingCostMaterial)
    }

/// Set the expense portion of remaining cost.
    ///
    /// Corresponds to Java: `public void setRemainingCostExpense(Number value)`
    pub fn set_remaining_cost_expense(&mut self, value: Option<f64>) {
        self.put_number(TaskField::RemainingCostExpense, value);
    }

/// Get the expense portion of remaining cost.
    ///
    /// Corresponds to Java: `public Number getRemainingCostExpense()`
    pub fn get_remaining_cost_expense(&self) -> Option<f64> {
        self.number_field(TaskField::RemainingCostExpense)
    }
}

// ---------------------------------------------------------------------------
// Remaining quantities and Asta / Primavera extension fields
// ---------------------------------------------------------------------------

impl Task {
/// Set the stored material quantity.
    ///
    /// Corresponds to Java: `public void setStoredMaterial(Number value)`
    pub fn set_stored_material(&mut self, value: Option<f64>) {
        self.put_number(TaskField::StoredMaterial, value);
    }

/// Get the stored material quantity.
    ///
    /// Corresponds to Java: `public Number getStoredMaterial()`
    pub fn get_stored_material(&self) -> Option<f64> {
        self.number_field(TaskField::StoredMaterial)
    }

/// Set the feature of work.
    ///
    /// Corresponds to Java: `public void setFeatureOfWork(String value)`
    pub fn set_feature_of_work(&mut self, value: Option<String>) {
        self.put_string(TaskField::FeatureOfWork, value);
    }

/// Get the feature of work.
    ///
    /// Corresponds to Java: `public String getFeatureOfWork()`
    pub fn get_feature_of_work(&self) -> Option<String> {
        self.string_field(TaskField::FeatureOfWork)
    }

/// Set the category of work.
    ///
    /// Corresponds to Java: `public void setCategoryOfWork(String value)`
    pub fn set_category_of_work(&mut self, value: Option<String>) {
        self.put_string(TaskField::CategoryOfWork, value);
    }

/// Get the category of work.
    ///
    /// Corresponds to Java: `public String getCategoryOfWork()`
    pub fn get_category_of_work(&self) -> Option<String> {
        self.string_field(TaskField::CategoryOfWork)
    }

/// Set the phase of work.
    ///
    /// Corresponds to Java: `public void setPhaseOfWork(String value)`
    pub fn set_phase_of_work(&mut self, value: Option<String>) {
        self.put_string(TaskField::PhaseOfWork, value);
    }

/// Get the phase of work.
    ///
    /// Corresponds to Java: `public String getPhaseOfWork()`
    pub fn get_phase_of_work(&self) -> Option<String> {
        self.string_field(TaskField::PhaseOfWork)
    }

/// Set the bid item.
    ///
    /// Corresponds to Java: `public void setBidItem(String value)`
    pub fn set_bid_item(&mut self, value: Option<String>) {
        self.put_string(TaskField::BidItem, value);
    }

/// Get the bid item.
    ///
    /// Corresponds to Java: `public String getBidItem()`
    pub fn get_bid_item(&self) -> Option<String> {
        self.string_field(TaskField::BidItem)
    }

/// Set the mod or claim number.
    ///
    /// Corresponds to Java: `public void setModOrClaimNumber(String value)`
    pub fn set_mod_or_claim_number(&mut self, value: Option<String>) {
        self.put_string(TaskField::ModOrClaimNumber, value);
    }

/// Get the mod or claim number.
    ///
    /// Corresponds to Java: `public String getModOrClaimNumber()`
    pub fn get_mod_or_claim_number(&self) -> Option<String> {
        self.string_field(TaskField::ModOrClaimNumber)
    }

/// Set the work area code.
    ///
    /// Corresponds to Java: `public void setWorkAreaCode(String value)`
    pub fn set_work_area_code(&mut self, value: Option<String>) {
        self.put_string(TaskField::WorkAreaCode, value);
    }

/// Get the work area code.
    ///
    /// Corresponds to Java: `public String getWorkAreaCode()`
    pub fn get_work_area_code(&self) -> Option<String> {
        self.string_field(TaskField::WorkAreaCode)
    }

/// Set the responsibility code.
    ///
    /// Corresponds to Java: `public void setResponsibilityCode(String value)`
    pub fn set_responsibility_code(&mut self, value: Option<String>) {
        self.put_string(TaskField::ResponsibilityCode, value);
    }

/// Get the responsibility code.
    ///
    /// Corresponds to Java: `public String getResponsibilityCode()`
    pub fn get_responsibility_code(&self) -> Option<String> {
        self.string_field(TaskField::ResponsibilityCode)
    }

/// Set the workers per day.
    ///
    /// Corresponds to Java: `public void setWorkersPerDay(Integer value)`
    pub fn set_workers_per_day(&mut self, value: Option<i32>) {
        self.put_integer(TaskField::WorkersPerDay, value);
    }

/// Get the workers per day.
    ///
    /// Corresponds to Java: `public Integer getWorkersPerDay()`
    pub fn get_workers_per_day(&self) -> Option<i32> {
        self.integer_field(TaskField::WorkersPerDay)
    }

/// Set the hammock code flag.
    ///
    /// Corresponds to Java: `public void setHammockCode(boolean value)`
    pub fn set_hammock_code(&mut self, value: bool) {
        self.put_bool(TaskField::HammockCode, value);
    }

/// Get the hammock code flag.
    ///
    /// Corresponds to Java: `public boolean getHammockCode()`
    pub fn get_hammock_code(&self) -> bool {
        self.boolean_field(TaskField::HammockCode)
    }

/// Set the mail field.
    ///
    /// Corresponds to Java: `public void setMail(String value)`
    pub fn set_mail(&mut self, value: Option<String>) {
        self.put_string(TaskField::Mail, value);
    }

/// Get the mail field.
    ///
    /// Corresponds to Java: `public String getMail()`
    pub fn get_mail(&self) -> Option<String> {
        self.string_field(TaskField::Mail)
    }

/// Set the section field.
    ///
    /// Corresponds to Java: `public void setSection(String value)`
    pub fn set_section(&mut self, value: Option<String>) {
        self.put_string(TaskField::Section, value);
    }

/// Get the section field.
    ///
    /// Corresponds to Java: `public String getSection()`
    pub fn get_section(&self) -> Option<String> {
        self.string_field(TaskField::Section)
    }

/// Set the manager field.
    ///
    /// Corresponds to Java: `public void setManager(String value)`
    pub fn set_manager(&mut self, value: Option<String>) {
        self.put_string(TaskField::Manager, value);
    }

/// Get the manager field.
    ///
    /// Corresponds to Java: `public String getManager()`
    pub fn get_manager(&self) -> Option<String> {
        self.string_field(TaskField::Manager)
    }

/// Set the department field.
    ///
    /// Corresponds to Java: `public void setDepartment(String value)`
    pub fn set_department(&mut self, value: Option<String>) {
        self.put_string(TaskField::Department, value);
    }

/// Get the department field.
    ///
    /// Corresponds to Java: `public String getDepartment()`
    pub fn get_department(&self) -> Option<String> {
        self.string_field(TaskField::Department)
    }

/// Set the overall percent complete.
    ///
    /// Corresponds to Java: `public void setOverallPercentComplete(Number value)`
    pub fn set_overall_percent_complete(&mut self, value: Option<f64>) {
        self.put_number(TaskField::OverallPercentComplete, value);
    }

/// Get the overall percent complete.
    ///
    /// Corresponds to Java: `public Number getOverallPercentComplete()`
    pub fn get_overall_percent_complete(&self) -> Option<f64> {
        self.number_field(TaskField::OverallPercentComplete)
    }

/// Set the planned finish date.
    ///
    /// Corresponds to Java: `public void setPlannedFinish(LocalDateTime value)`
    pub fn set_planned_finish(&mut self, value: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::PlannedFinish, value);
    }

/// Get the planned finish date.
    ///
    /// Corresponds to Java: `public LocalDateTime getPlannedFinish()`
    pub fn get_planned_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::PlannedFinish)
    }

/// Set the planned start date.
    ///
    /// Corresponds to Java: `public void setPlannedStart(LocalDateTime value)`
    pub fn set_planned_start(&mut self, value: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::PlannedStart, value);
    }

/// Get the planned start date.
    ///
    /// Corresponds to Java: `public LocalDateTime getPlannedStart()`
    pub fn get_planned_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::PlannedStart)
    }

/// Set the planned duration.
    ///
    /// Corresponds to Java: `public void setPlannedDuration(Duration value)`
    pub fn set_planned_duration(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::PlannedDuration, value);
    }

/// Get the planned duration.
    ///
    /// Corresponds to Java: `public Duration getPlannedDuration()`
    pub fn get_planned_duration(&self) -> Option<Duration> {
        self.duration_field(TaskField::PlannedDuration)
    }

/// Set the planned work.
    ///
    /// Corresponds to Java: `public void setPlannedWork(Duration value)`
    pub fn set_planned_work(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::PlannedWork, value);
    }

/// Get the planned work.
    ///
    /// Corresponds to Java: `public Duration getPlannedWork()`
    pub fn get_planned_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::PlannedWork)
    }

/// Set the planned cost.
    ///
    /// Corresponds to Java: `public void setPlannedCost(Number value)`
    pub fn set_planned_cost(&mut self, value: Option<f64>) {
        self.put_number(TaskField::PlannedCost, value);
    }

/// Get the planned cost.
    ///
    /// Corresponds to Java: `public Number getPlannedCost()`
    pub fn get_planned_cost(&self) -> Option<f64> {
        self.number_field(TaskField::PlannedCost)
    }

/// Set the suspend date.
    ///
    /// Corresponds to Java: `public void setSuspendDate(LocalDateTime value)`
    pub fn set_suspend_date(&mut self, value: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::SuspendDate, value);
    }

/// Get the suspend date.
    ///
    /// Corresponds to Java: `public LocalDateTime getSuspendDate()`
    pub fn get_suspend_date(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::SuspendDate)
    }

/// Set the primary resource unique ID.
    ///
    /// Corresponds to Java: `public void setPrimaryResourceUniqueID(Integer value)`
    pub fn set_primary_resource_unique_id(&mut self, value: Option<i32>) {
        self.put_integer(TaskField::PrimaryResourceUniqueId, value);
    }

/// Get the primary resource unique ID.
    ///
    /// Corresponds to Java: `public Integer getPrimaryResourceUniqueID()`
    pub fn get_primary_resource_unique_id(&self) -> Option<i32> {
        self.integer_field(TaskField::PrimaryResourceUniqueId)
    }

/// Get the primary resource.
    ///
    /// Corresponds to Java: `public Resource getPrimaryResource()`
    pub fn get_primary_resource(&self) -> Option<Rc<RefCell<Resource>>> {
        let file = self.parent_file.upgrade()?;
        let unique_id = self.get_primary_resource_unique_id();
        file.borrow().get_resource_by_unique_id(unique_id)
    }

/// Set the primary resource.
    ///
    /// Corresponds to Java: `public void setPrimaryResource(Resource resource)`
    pub fn set_primary_resource(&mut self, resource: Option<&Rc<RefCell<Resource>>>) {
        let unique_id = resource.and_then(|resource| resource.borrow().get_unique_id());
        self.set_primary_resource_unique_id(unique_id);
    }
}

// ---------------------------------------------------------------------------
// Subprojects (external projects)
// ---------------------------------------------------------------------------

impl Task {
/// If this task represents an external project (subproject), expand it and attach the subproject tasks as child tasks.
    ///
    /// Corresponds to Java: `public ProjectFile expandSubproject()`
    pub fn expand_subproject(&mut self) -> Option<Rc<RefCell<ProjectFile>>> {
        if !self.get_external_project() || self.get_subproject_object().is_none() {
            return None;
        }

        let subproject = self.get_subproject_object()?;

        let children = {
            let summary_task = subproject.borrow().get_task_by_id(Some(0));
            match summary_task {
                None => subproject.borrow().get_child_tasks(),
                Some(summary_task) => summary_task.borrow().get_child_tasks(),
            }
        };

        self.children.extend(children);
        Some(subproject)
    }

/// Read the external project file referenced by this task (the result is cached by the project file).
    ///
    /// Corresponds to Java: `public ProjectFile getSubprojectObject()`
    pub fn get_subproject_object(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        if !self.get_external_task() && !self.get_external_project() {
            return None;
        }

        let file = self.parent_file.upgrade()?;
        let subproject_file = self.get_subproject_file();
        file.borrow_mut().read_external_project(subproject_file.as_deref())
    }

/// Attach the already-read project file to the external project referenced by this task.
    ///
    /// Corresponds to Java: `public void setSubprojectObject(ProjectFile projectFile)`
    pub fn set_subproject_object(&mut self, project_file: Option<Rc<RefCell<ProjectFile>>>) {
        let Some(file) = self.parent_file.upgrade() else {
            return;
        };

        let subproject_file = self.get_subproject_file();
        file.borrow_mut()
            .add_external_project(subproject_file.as_deref(), project_file);
    }
}

// ---------------------------------------------------------------------------
// Calculated fields (correspond to Java's calculateXxx / defaultXxx methods)
// ---------------------------------------------------------------------------

impl Task {
/// Get the parent task unique ID.
    ///
    /// Corresponds to Java: `private Integer calculateParentTaskUniqueID()`
    fn calculate_parent_task_unique_id(&self) -> Option<i32> {
        self.parent.upgrade()?.borrow().get_unique_id()
    }

/// Calculate the start variance.
    ///
    /// Corresponds to Java: `private Duration calculateStartVariance()`
    ///
/// Returns `None` when the project file is missing, since the default duration units cannot be obtained (Java throws `NullPointerException`).
    fn calculate_start_variance(&self) -> Option<Duration> {
        let format = {
            let file = self.parent_file.upgrade()?;
            let file_ref = file.borrow();
            file_ref.get_project_properties().get_default_duration_units()
        };

        let calendar = self.get_effective_calendar();
        let variance = match calendar.as_ref() {
            Some(calendar) => {
                let calendar_ref = calendar.borrow();
                LocalDateTimeHelper::get_variance(
                    Some(&calendar_ref),
                    self.get_baseline_start(),
                    self.get_start(),
                    format,
                )
            }
            None => LocalDateTimeHelper::get_variance(
                None,
                self.get_baseline_start(),
                self.get_start(),
                format,
            ),
        };

        Some(variance)
    }

/// Calculate the finish variance.
    ///
    /// Corresponds to Java: `private Duration calculateFinishVariance()`
    fn calculate_finish_variance(&self) -> Option<Duration> {
        let format = {
            let file = self.parent_file.upgrade()?;
            let file_ref = file.borrow();
            file_ref.get_project_properties().get_default_duration_units()
        };

        let calendar = self.get_effective_calendar();
        let variance = match calendar.as_ref() {
            Some(calendar) => {
                let calendar_ref = calendar.borrow();
                LocalDateTimeHelper::get_variance(
                    Some(&calendar_ref),
                    self.get_baseline_finish(),
                    self.get_finish(),
                    format,
                )
            }
            None => LocalDateTimeHelper::get_variance(
                None,
                self.get_baseline_finish(),
                self.get_finish(),
                format,
            ),
        };

        Some(variance)
    }

/// Calculate start slack (delegated to the `ProjectConfig` slack calculator).
    ///
    /// Corresponds to Java: `private Duration calculateStartSlack()`
    fn calculate_start_slack(&self) -> Option<Duration> {
        self.parent_file
            .upgrade()?
            .borrow()
            .get_project_config()
            .get_slack_calculator()
            .and_then(|calculator| calculator.calculate_start_slack(self))
    }

/// Calculate finish slack (delegated to the `ProjectConfig` slack calculator).
    ///
    /// Corresponds to Java: `private Duration calculateFinishSlack()`
    fn calculate_finish_slack(&self) -> Option<Duration> {
        self.parent_file
            .upgrade()?
            .borrow()
            .get_project_config()
            .get_slack_calculator()
            .and_then(|calculator| calculator.calculate_finish_slack(self))
    }

/// Calculate the cost variance.
    ///
    /// Corresponds to Java: `private Double calculateCostVariance()`
    fn calculate_cost_variance(&self) -> Option<f64> {
        let cost = self.get_cost()?;
        let baseline_cost = self.get_baseline_cost()?;
        Some(cost - baseline_cost)
    }

/// Calculate the duration variance.
    ///
    /// Corresponds to Java: `private Duration calculateDurationVariance()`
    ///
/// Returns `None` when the project file is missing, since the baseline duration units cannot be converted.
    fn calculate_duration_variance(&self) -> Option<Duration> {
        let duration = self.get_duration()?;
        let baseline_duration = self.get_baseline_duration()?;

        let file = self.parent_file.upgrade()?;
        let file_ref = file.borrow();
        let baseline =
            baseline_duration.convert_units(duration.get_units(), file_ref.get_project_properties());

        Some(Duration::get_instance(
            duration.get_duration() - baseline.get_duration(),
            duration.get_units(),
        ))
    }

/// Calculate the work variance.
    ///
    /// Corresponds to Java: `private Duration calculateWorkVariance()`
    fn calculate_work_variance(&self) -> Option<Duration> {
        let work = self.get_work()?;
        let baseline_work = self.get_baseline_work()?;

        let file = self.parent_file.upgrade()?;
        let file_ref = file.borrow();
        let baseline =
            baseline_work.convert_units(work.get_units(), file_ref.get_project_properties());

        Some(Duration::get_instance(
            work.get_duration() - baseline.get_duration(),
            work.get_units(),
        ))
    }

/// Calculate CV (BCWP - ACWP).
    ///
    /// Corresponds to Java: `private Double calculateCV()`
    fn calculate_cv(&self) -> Option<f64> {
        Some(NumberHelper::get_double(self.get_bcwp()) - NumberHelper::get_double(self.get_acwp()))
    }

/// Calculate SV (BCWP - BCWS).
    ///
    /// Corresponds to Java: `private Double calculateSV()`
    fn calculate_sv(&self) -> Option<f64> {
        let bcwp = self.get_bcwp()?;
        let bcws = self.get_bcws()?;
        Some(bcwp - bcws)
    }

/// Calculate free slack (delegated to the `ProjectConfig` slack calculator).
    ///
    /// Corresponds to Java: `private Duration calculateFreeSlack()`
    fn calculate_free_slack(&self) -> Option<Duration> {
        self.parent_file
            .upgrade()?
            .borrow()
            .get_project_config()
            .get_slack_calculator()
            .and_then(|calculator| calculator.calculate_free_slack(self))
    }

/// Calculate total slack (delegated to the `ProjectConfig` slack calculator).
    ///
    /// Corresponds to Java: `private Duration calculateTotalSlack()`
    fn calculate_total_slack(&self) -> Option<Duration> {
        self.parent_file
            .upgrade()?
            .borrow()
            .get_project_config()
            .get_slack_calculator()
            .and_then(|calculator| calculator.calculate_total_slack(self))
    }

/// Calculate the critical path flag.
    ///
    /// Corresponds to Java: `private Boolean calculateCritical()`
    ///
/// Returns `None` when the project file is missing, since the critical slack limit cannot be obtained (`getCritical()` is therefore `false`).
    fn calculate_critical(&self) -> Option<bool> {
        if self.get_actual_finish().is_some() {
            return Some(false);
        }

// Java returns Boolean.FALSE when total slack is missing, not null.
        let mut total_slack = match self.get_total_slack() {
            Some(value) => value,
            None => return Some(false),
        };

        let file = self.parent_file.upgrade()?;
        let file_ref = file.borrow();
        let critical_slack_limit = file_ref.get_project_properties().get_critical_slack_limit();

        if critical_slack_limit.get_duration() != 0.0
            && total_slack.get_duration() != 0.0
            && total_slack.get_units() != critical_slack_limit.get_units()
        {
            let calendar = self.get_effective_calendar();
            if let Some(calendar) = calendar {
                total_slack =
                    total_slack.convert_units(critical_slack_limit.get_units(), &*calendar.borrow());
            }
        }

        Some(
            total_slack.get_duration() <= critical_slack_limit.get_duration()
                && NumberHelper::get_int(self.get_percentage_complete()) != 100
                && (self.get_task_mode() == Some(TaskMode::AutoScheduled)
                    || (self.get_duration_text().is_none()
                        && self.get_start_text().is_none()
                        && self.get_finish_text().is_none())),
        )
    }

/// Calculate the "complete through" date.
    ///
    /// Corresponds to Java: `private LocalDateTime calculateCompleteThrough()`
    fn calculate_complete_through(&self) -> Option<NaiveDateTime> {
        let percent_complete = NumberHelper::get_int(self.get_percentage_complete());

        if percent_complete == 0 {
            return None;
        }

        if percent_complete == 100 {
            return self.get_actual_finish();
        }

        let actual_start = self.get_actual_start()?;
        let duration = self.get_duration()?;

        let duration = Duration::get_instance(
            (duration.get_duration() * f64::from(percent_complete)) / 100.0,
            duration.get_units(),
        );
        let calendar = self.get_effective_calendar()?;
        let value = calendar.borrow_mut().get_date(actual_start, duration);

        let complete_through_is_next_work_start = self
            .parent_file
            .upgrade()
            .map_or(false, |file| {
                file.borrow()
                    .get_project_config()
                    .get_complete_through_is_next_work_start()
            });

        if complete_through_is_next_work_start {
            // Java: calendar.getNextWorkStart(value)
            let calendar = self.get_effective_calendar()?;
            return Some(calendar.borrow().get_next_work_start(value));
        }

        Some(value)
    }

/// Calculate the external project flag.
    ///
    /// Corresponds to Java: `private Boolean calculateExternalProject()`
    fn calculate_external_project(&self) -> Option<bool> {
        Some(self.get_subproject_file().is_some() && !self.get_external_task())
    }

/// Calculate percent complete (source selected by `PercentCompleteType`).
    ///
    /// Corresponds to Java: `private Number calculateActivityPercentComplete()`
    fn calculate_activity_percent_complete(&self) -> Option<f64> {
        match self.get_percent_complete_type() {
            None => self.get_percentage_complete(),
            Some(PercentCompleteType::Units) => self.get_percentage_work_complete(),
            Some(PercentCompleteType::Physical) => self.get_physical_percent_complete(),
            Some(_) => self.get_percentage_complete(),
        }
    }

/// Calculate the schedule percent complete.
    ///
    /// Corresponds to Java: `private Number calculateSchedulePercentComplete()`
    ///
/// Java notes this as `TODO: review and implement Schedule Percent Complete for WBS`.
    fn calculate_schedule_percent_complete(&self) -> Option<f64> {
        if self.get_summary() {
            return None;
        }

        let file = self.parent_file.upgrade()?;
        let file_ref = file.borrow();
        let data_date = file_ref.get_project_properties().get_status_date()?;
        let baseline_start = self.get_baseline_start()?;
        let baseline_finish = self.get_baseline_finish()?;

        if data_date < baseline_start {
            return Some(NumberHelper::DOUBLE_ZERO);
        }

        if data_date > baseline_finish {
            return Some(NumberHelper::DOUBLE_ONEHUNDRED);
        }

        let calendar = self.get_effective_calendar()?;
        let total_work = calendar
            .borrow()
            .get_work(baseline_start, baseline_finish, TimeUnit::Hours)
            .get_duration();
        if total_work == 0.0 {
            return None;
        }

        let current_work = calendar
            .borrow()
            .get_work(baseline_start, data_date, TimeUnit::Hours)
            .get_duration();

        Some((current_work * 100.0) / total_work)
    }

/// Calculate regular cost (total cost - overtime cost).
    ///
    /// Corresponds to Java: `private Number calculateRegularCost(Supplier<Number> totalCostSupplier, Supplier<Number> overtimeCostSupplier)`
    ///
/// Java uses a `Supplier` for lazy evaluation; on the Rust side the caller passes in the resolved values directly.
    fn calculate_regular_cost(
        &self,
        total_cost: Option<f64>,
        overtime_cost: Option<f64>,
    ) -> Option<f64> {
        if total_cost.is_none() && overtime_cost.is_none() {
            return None;
        }

        Some(
            NumberHelper::get_double(total_cost) - NumberHelper::get_double(overtime_cost),
        )
    }

/// Calculate regular work (total work - overtime work).
    ///
    /// Corresponds to Java: `private Duration calculateRegularWork(Supplier<Duration> totalWorkSupplier, Supplier<Duration> overtimeWorkSupplier)`
    ///
/// Java uses `getCalendar()` as the default source for unit conversion; the Rust side skips conversion when the calendar is empty
/// (Java throws `NullPointerException`).
    fn calculate_regular_work(
        &self,
        total_work: Option<Duration>,
        overtime_work: Option<Duration>,
    ) -> Option<Duration> {
        if total_work.is_none() && overtime_work.is_none() {
            return None;
        }

        let units = match (total_work, overtime_work) {
            (Some(total_work), _) => total_work.get_units(),
            (None, Some(overtime_work)) => overtime_work.get_units(),
// The case where both are None was already excluded above
            (None, None) => return None,
        };

        let calendar = self.get_calendar();
        let convert = |work: Option<Duration>| -> f64 {
            match (work, calendar.as_ref()) {
                (Some(work), Some(calendar)) => {
                    work.convert_units(units, &*calendar.borrow()).get_duration()
                }
                (Some(work), None) => work.get_duration(),
                (None, _) => 0.0,
            }
        };

        let total = convert(total_work);
        let overtime = convert(overtime_work);

        Some(Duration::get_instance(total - overtime, units))
    }

/// Get the default value of the `active` field.
    ///
    /// Corresponds to Java: `private Boolean defaultActive()`
    fn default_active() -> bool {
        true
    }

/// Get the default value of the `type` field.
    ///
    /// Corresponds to Java: `private TaskType defaultType()`
    fn default_type() -> TaskType {
        TaskType::FixedUnits
    }

/// Get the default value of the `taskMode` field.
    ///
    /// Corresponds to Java: `private TaskMode defaultTaskMode()`
    fn default_task_mode() -> TaskMode {
        TaskMode::AutoScheduled
    }

/// Get the default value of the `expanded` field.
    ///
    /// Corresponds to Java: `private Boolean defaultExpanded()`
    fn default_expanded() -> bool {
        true
    }

/// Get the default value of the `autoComputeActuals` field.
    ///
    /// Corresponds to Java: `private Boolean defaultAutoComputeActuals()`
    fn default_auto_compute_actuals() -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// Display / trait implementations
// ---------------------------------------------------------------------------

impl Display for Task {
    /// Corresponds to Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[Task id={}", optional_display(self.get_id()))?;
        write!(f, " uniqueID={}", optional_display(self.get_unique_id()))?;

        if let Some(activity_id) = self.get_activity_id() {
            write!(f, " activityID={activity_id}")?;
        }

        write!(f, " name={}", optional_display(self.get_name()))?;

        if self.get_external_task() {
            write!(
                f,
                " [EXTERNAL uid={} id={}]",
                optional_display(self.get_subproject_task_unique_id()),
                optional_display(self.get_subproject_task_id())
            )
        } else {
            write!(f, "]")
        }
    }
}

/// Corresponds to Java's output that concatenates nullable values directly into a string (`null` is output as `"null"`).
fn optional_display<T: Display>(value: Option<T>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| value.to_string())
}

/// Get the work (in hours) between two instants on the calendar.
///
/// Corresponds to Java: `calendar.getWork(start, finish, TimeUnit.HOURS).getDuration()`.
/// Returns `0.0` when the calendar is missing, a date is empty, or the calendar is currently borrowed (Java throws `NullPointerException`).
fn work_hours(
    calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
    start: Option<NaiveDateTime>,
    finish: Option<NaiveDateTime>,
) -> f64 {
    match (calendar, start, finish) {
        (Some(calendar), Some(start), Some(finish)) => match calendar.try_borrow() {
            Ok(calendar) => calendar
                .get_work(start, finish, TimeUnit::Hours)
                .get_duration(),
            Err(_) => 0.0,
        },
        _ => 0.0,
    }
}

impl FieldContainer for Task {
    /// Corresponds to Java: `@Override public void set(FieldType field, Object value)`
    fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
        Task::set(self, field, value);
    }

    /// Corresponds to Java: `@Override public Object get(FieldType field)`
    fn get(&self, field: FieldKey) -> Option<FieldValue> {
        Task::get(self, field)
    }

    /// Corresponds to Java: `@Override public Object getCachedValue(FieldType field)`
    fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
        Task::get_cached_value(self, field)
    }

    /// Corresponds to Java: `@Override public void addFieldListener(FieldListener listener)`
    fn add_field_listener(&mut self, listener: Box<dyn FieldListener>) {
        Task::add_field_listener(self, listener);
    }

    /// Corresponds to Java: `@Override public void removeFieldListener(FieldListener listener)`
    fn clear_field_listeners(&mut self) {
        Task::clear_field_listeners(self);
    }
}

impl ProjectEntityWithUniqueID for Task {
    /// Corresponds to Java: `@Override public Integer getUniqueID()`
    fn get_unique_id(&self) -> Option<i32> {
        self.integer_field(TaskField::UniqueId)
    }
}

impl ProjectEntityWithMutableUniqueID for Task {
    /// Corresponds to Java: `@Override public void setUniqueID(Integer val)`
    fn set_unique_id(&mut self, id: Option<i32>) {
        self.put_integer(TaskField::UniqueId, id);
    }
}

impl ProjectEntityWithID for Task {
    /// Corresponds to Java: `@Override public Integer getID()`
    fn get_id(&self) -> Option<i32> {
        self.integer_field(TaskField::Id)
    }

    /// Corresponds to Java: `@Override public void setID(Integer val)`
    fn set_id(&mut self, id: Option<i32>) {
        self.put_integer(TaskField::Id, id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html_notes::HtmlNotes;
    use crate::notes::PlainNotes;
    use crate::rtf_notes::RtfNotes;
    use crate::step::Builder as StepBuilder;

    fn datetime(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .expect("valid date")
            .and_hms_opt(hour, minute, 0)
            .expect("valid time")
    }

    fn days(value: f64) -> Duration {
        Duration::get_instance(value, TimeUnit::Days)
    }

/// Note: only paths that do not depend on the parent file are covered here — the parent `ProjectFile`-dependent
/// [`Task::new_in_file`] and accessors such as `add_task` / `get_calendar` / `get_resource_assignments`
/// are outside the scope of this test.
    #[test]
    fn field_values_round_trip() {
        let mut task = Task::new();

        task.set_name(Some("Design".to_owned()));
        task.set_wbs(Some("1.2".to_owned()));
        task.set_duration(Some(days(3.0)));
        task.set_start(Some(datetime(2024, 1, 1, 9, 0)));
        task.set_finish(Some(datetime(2024, 1, 5, 17, 0)));
        task.set_cost(Some(1200.0));
        task.set_percentage_complete(Some(50.0));
        task.set_milestone(true);
        task.set_unique_id(Some(7));
        task.set_id(Some(2));

        assert_eq!(task.get_name(), Some("Design".to_owned()));
        assert_eq!(task.get_wbs(), Some("1.2".to_owned()));
        assert_eq!(task.get_duration(), Some(days(3.0)));
        assert_eq!(task.get_start(), Some(datetime(2024, 1, 1, 9, 0)));
        assert_eq!(task.get_finish(), Some(datetime(2024, 1, 5, 17, 0)));
        assert_eq!(task.get_cost(), Some(1200.0));
        assert_eq!(task.get_percentage_complete(), Some(50.0));
        assert!(task.get_milestone());
        assert_eq!(task.get_unique_id(), Some(7));
        assert_eq!(task.get_id(), Some(2));

// Unset fields return None / false
        assert_eq!(task.get_contact(), None);
        assert!(!task.get_confirmed());

// Passing None is equivalent to Java's set(field, null): it removes the field from the field table
        task.set_name(None);
        assert_eq!(task.get_name(), None);
        assert_eq!(task.get_cached_value(FieldKey::Task(TaskField::Name)), None);
    }

    #[test]
    fn field_container_is_delegated_and_cached() {
        let mut task = Task::new();

        task.set(
            FieldKey::Task(TaskField::Notes),
            Some(FieldValue::String("note".to_owned())),
        );
        assert_eq!(
            task.get_cached_value(FieldKey::Task(TaskField::Notes)),
            Some(&FieldValue::String("note".to_owned()))
        );
        assert_eq!(
            Task::get(&task, FieldKey::Task(TaskField::Notes)),
            Some(FieldValue::String("note".to_owned()))
        );

// Calculated defaults are not written back to the field table (see difference 1 in the file header)
        assert_eq!(
            task.get(FieldKey::Task(TaskField::AutoComputeActuals)),
            Some(FieldValue::Boolean(true))
        );
        assert_eq!(
            task.get_cached_value(FieldKey::Task(TaskField::AutoComputeActuals)),
            None
        );
    }

    #[test]
    fn field_container_trait_dispatches_to_task() {
        let mut task = Task::new();

        {
            let container: &mut dyn FieldContainer = &mut task;
            container.set(
                FieldKey::Task(TaskField::Name),
                Some(FieldValue::String("Trait".to_owned())),
            );
        }

        assert_eq!(task.get_name(), Some("Trait".to_owned()));
    }

    #[test]
    fn calculated_fields_are_computed_on_demand() {
        let mut task = Task::new();

// Java: calculateCostVariance — requires both cost and baselineCost to be present
        assert_eq!(task.get_cost_variance(), None);
        task.set_cost(Some(100.0));
        task.set_baseline_cost(Some(80.0));
        assert_eq!(task.get_cost_variance(), Some(20.0));

        // Java: calculateSV / calculateCV
        task.set_bcwp(Some(30.0));
        task.set_bcws(Some(10.0));
        task.set_acwp(Some(5.0));
        assert_eq!(task.get_sv(), Some(20.0));
        assert_eq!(task.get_cv(), Some(25.0));

        // Java: calculateExternalProject
        assert!(!task.get_external_project());
        task.set_subproject_file(Some("sub.mpp".to_owned()));
        assert!(task.get_external_project());
        task.set_external_task(true);
        assert!(!task.get_external_project());

        // Java: calculateActivityPercentComplete
        task.set_percentage_complete(Some(10.0));
        task.set_percentage_work_complete(Some(20.0));
        task.set_physical_percent_complete(Some(30.0));
        assert_eq!(task.get_activity_percent_complete(), Some(10.0));
        task.set_percent_complete_type(Some(PercentCompleteType::Units));
        assert_eq!(task.get_activity_percent_complete(), Some(20.0));
        task.set_percent_complete_type(Some(PercentCompleteType::Physical));
        assert_eq!(task.get_activity_percent_complete(), Some(30.0));
    }

    #[test]
    fn default_calculated_values_match_java() {
        let mut task = Task::new();

// Java's CALCULATED_FIELD_MAP defaults: active / type / taskMode / expanded /
        // autoComputeActuals
        assert!(task.get_active());
        assert_eq!(task.get_type(), Some(TaskType::FixedUnits));
        assert_eq!(task.get_task_mode(), Some(TaskMode::AutoScheduled));
        assert!(task.get_expanded());
        assert!(task.get_auto_compute_actuals());

// Explicit settings override the defaults
        task.set_active(false);
        task.set_type(Some(TaskType::FixedWork));
        task.set_task_mode(Some(TaskMode::ManuallyScheduled));
        task.set_expanded(false);
        task.set_auto_compute_actuals(false);

        assert!(!task.get_active());
        assert_eq!(task.get_type(), Some(TaskType::FixedWork));
        assert_eq!(task.get_task_mode(), Some(TaskMode::ManuallyScheduled));
        assert!(!task.get_expanded());
        assert!(!task.get_auto_compute_actuals());
    }

    #[test]
    fn always_calculated_fields_are_not_cached() {
        assert!(Task::get_always_calculated_field(TaskField::WorkSplits));
        assert!(Task::get_always_calculated_field(TaskField::Predecessors));
        assert!(!Task::get_always_calculated_field(TaskField::Name));

        let task = Task::new();

// Without a parent file SCHEDULE_PERCENT_COMPLETE cannot be calculated; it returns None and does not enter the field table
        assert_eq!(
            task.get(FieldKey::Task(TaskField::SchedulePercentComplete)),
            None
        );
        assert_eq!(
            task.get_cached_value(FieldKey::Task(TaskField::SchedulePercentComplete)),
            None
        );
    }

    #[test]
    fn enum_and_uuid_fields_round_trip() {
        let mut task = Task::new();

        task.set_constraint_type(Some(ConstraintType::MustStartOn));
        assert_eq!(
            task.get_constraint_type(),
            Some(ConstraintType::MustStartOn)
        );
        task.set_secondary_constraint_type(Some(ConstraintType::FinishNoLaterThan));
        assert_eq!(
            task.get_secondary_constraint_type(),
            Some(ConstraintType::FinishNoLaterThan)
        );

        task.set_priority(Some(Priority::get_instance(Priority::HIGH)));
        assert_eq!(
            task.get_priority().map(|value| value.get_value()),
            Some(Priority::HIGH)
        );

        task.set_leveling_delay_format(Some(TimeUnit::ElapsedDays));
        assert_eq!(task.get_leveling_delay_format(), Some(TimeUnit::ElapsedDays));

        task.set_fixed_cost_accrual(Some(AccrueType::Start));
        assert_eq!(task.get_fixed_cost_accrual(), Some(AccrueType::Start));

        task.set_earned_value_method(Some(EarnedValueMethod::PhysicalPercentComplete));
        assert_eq!(
            task.get_earned_value_method(),
            Some(EarnedValueMethod::PhysicalPercentComplete)
        );

        task.set_activity_status(Some(ActivityStatus::Completed));
        assert_eq!(
            task.get_activity_status(),
            Some(ActivityStatus::Completed)
        );

// UUIDs are carried as strings (see difference 7 in the file header)
        let guid = Uuid::parse_str("2f9d1c2e-6d5b-4f0e-9a3c-1b2d3e4f5a6b").expect("valid UUID");
        task.set_guid(Some(guid));
        assert_eq!(task.get_guid(), Some(guid));
        assert_eq!(
            task.get_cached_value(FieldKey::Task(TaskField::Guid)),
            Some(&FieldValue::String(guid.to_string()))
        );

        task.set_guid(None);
        assert_eq!(task.get_guid(), None);
    }

    #[test]
    fn notes_objects_keep_their_subtype() {
        let mut task = Task::new();

        assert_eq!(task.get_notes(), "");
        assert!(task.get_notes_object().is_none());

        // Java: setNotes(String) → new Notes(notes)
        task.set_notes(Some("hello".to_owned()));
        assert_eq!(task.get_notes(), "hello");
        assert_eq!(
            task.get_notes_object().map(|notes| notes.to_string()),
            Some("hello".to_owned())
        );

        let plain = PlainNotes::new(Some("object".to_owned()));
        task.set_notes_object(Some(&plain));
        assert_eq!(task.get_notes(), "object");

// Java: setNotesObject(new RtfNotes(rtf)); RtfNotes plain text has the RTF markup removed,
// so printing "hello" proves the subtype has not degraded to a plain text note.
        let rtf = RtfNotes::new(Some("{\\rtf1 hello}".to_owned()));
        task.set_notes_object(Some(&rtf));
        assert_eq!(task.get_notes(), "hello");
        assert_eq!(
            task.get_cached_value(FieldKey::Task(TaskField::Notes))
                .map(|value| value.type_name()),
            Some("Notes")
        );

// Java: setNotesObject(new HtmlNotes(html)); likewise, "plain text with markup removed" confirms the subtype is preserved
        let html = HtmlNotes::new(Some("<html><body>world</body></html>".to_owned()));
        task.set_notes_object(Some(&html));
        assert_eq!(task.get_notes(), "world");

        task.set_notes_object(None);
        assert_eq!(task.get_notes(), "");
        assert!(task.get_notes_object().is_none());
    }

    #[test]
    fn steps_round_trip_through_the_field_table() {
        let mut task = Task::new();
// Java: getSteps() returns defaultSteps() (an empty list) when unset
        assert!(task.get_steps().is_empty());
        assert!(matches!(
            task.get(FieldKey::Task(TaskField::Steps)),
            Some(FieldValue::Steps(_))
        ));

        let step = StepBuilder::new(&Rc::new(RefCell::new(Task::new())))
            .unique_id(Some(7))
            .name(Some("Design".to_owned()))
            .percent_complete(Some(100.0))
            .build();
        task.set_steps(vec![step]);

        let steps = task.get_steps();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].get_unique_id(), Some(7));
        assert_eq!(steps[0].get_name(), Some("Design"));
        assert!(steps[0].get_complete());
    }

    #[test]
    fn work_splits_follow_assignment_splits() {
// No resource assignments and not a summary task ⇒ empty list (Java's calculateWorkSplits)
        let task = Task::new();
        assert!(!task.get_summary());
        assert!(task.get_work_splits().is_empty());
        assert!(matches!(
            task.get(FieldKey::Task(TaskField::WorkSplits)),
            Some(FieldValue::TimeRanges(_))
        ));

// Summary tasks do not show splits (Java comment: In MS Project, summary tasks do not show splits)
        let mut summary = Task::new();
        summary.set_summary(true);
        assert!(summary.get_work_splits().is_empty());
    }

    #[test]
    fn indexed_fields_map_to_the_java_field_lists() {
        let mut task = Task::new();

        task.set_text(1, Some("first".to_owned()));
        task.set_text(30, Some("last".to_owned()));
        assert_eq!(task.get_text(1), Some("first".to_owned()));
        assert_eq!(task.get_text(30), Some("last".to_owned()));

// Java throws IllegalArgumentException; the Rust side returns None / false
        assert_eq!(task.get_text(0), None);
        assert_eq!(task.get_text(31), None);
        task.set_text(31, Some("ignored".to_owned()));

        task.set_cost_at(2, Some(10.0));
        assert_eq!(task.get_cost_at(2), Some(10.0));
        assert_eq!(task.get_cost_at(11), None);

        task.set_flag(20, true);
        assert!(task.get_flag(20));
        assert!(!task.get_flag(21));

        task.set_duration_at(10, Some(days(1.0)));
        assert_eq!(task.get_duration_at(10), Some(days(1.0)));
        task.set_date(1, Some(datetime(2024, 3, 1, 8, 0)));
        assert_eq!(task.get_date(1), Some(datetime(2024, 3, 1, 8, 0)));
        task.set_start_at(1, Some(datetime(2024, 3, 1, 8, 0)));
        assert_eq!(task.get_start_at(1), Some(datetime(2024, 3, 1, 8, 0)));
        task.set_finish_at(1, Some(datetime(2024, 3, 2, 8, 0)));
        assert_eq!(task.get_finish_at(1), Some(datetime(2024, 3, 2, 8, 0)));
        task.set_number(1, Some(1.5));
        assert_eq!(task.get_number(1), Some(1.5));
        task.set_outline_code(1, Some("OC".to_owned()));
        assert_eq!(task.get_outline_code(1), Some("OC".to_owned()));
        task.set_enterprise_text(1, Some("ET".to_owned()));
        assert_eq!(task.get_enterprise_text(1), Some("ET".to_owned()));
        task.set_enterprise_flag(1, true);
        assert!(task.get_enterprise_flag(1));

// Multiple baseline fields
        task.set_baseline_cost_at(1, Some(5.0));
        assert_eq!(task.get_baseline_cost_at(1), Some(5.0));
        assert_eq!(task.get_baseline_cost_at(0), None);
        task.set_baseline_work_at(3, Some(days(2.0)));
        assert_eq!(task.get_baseline_work_at(3), Some(days(2.0)));
    }

    #[test]
    fn baseline_duration_and_text_share_one_field() {
        let mut task = Task::new();

// Java: the BASELINE_DURATION field can hold either a Duration or a String,
// determined by instanceof when reading (the Rust side returns based on the value type)
        task.set_baseline_duration(Some(days(4.0)));
        assert_eq!(task.get_baseline_duration(), Some(days(4.0)));
        assert_eq!(task.get_baseline_duration_text(), None);

        task.set_baseline_duration_text(Some("4d".to_owned()));
        assert_eq!(task.get_baseline_duration_text(), Some("4d".to_owned()));
        assert_eq!(task.get_baseline_duration(), None);
    }

    #[test]
    fn hierarchy_links_parent_and_children() {
        let parent = Rc::new(RefCell::new(Task::new()));
        parent.borrow_mut().set_outline_level(Some(1));
        parent.borrow_mut().set_unique_id(Some(7));

        let child = Rc::new(RefCell::new(Task::new()));
        Task::add_child_task(&parent, &child);

        assert!(parent.borrow().has_child_tasks());
        assert_eq!(parent.borrow().get_child_tasks().len(), 1);
        assert!(parent.borrow().get_summary());
        assert_eq!(child.borrow().get_parent_task_unique_id(), Some(7));

        let recovered = child.borrow().get_parent_task().expect("parent task still exists");
        assert!(Rc::ptr_eq(&recovered, &parent));

// After removing the child task the parent is no longer a summary task, and the back reference is cleared
        Task::remove_child_task(&parent, &child);
        assert!(!parent.borrow().has_child_tasks());
        assert!(!parent.borrow().get_summary());
        assert!(child.borrow().get_parent_task().is_none());
        assert_eq!(child.borrow().get_parent_task_unique_id(), None);
    }

    #[test]
    fn child_tasks_can_be_inserted_and_cleared() {
        let parent = Rc::new(RefCell::new(Task::new()));
        let first = Rc::new(RefCell::new(Task::new()));
        let second = Rc::new(RefCell::new(Task::new()));

        Task::add_child_task(&parent, &first);
        Task::add_child_task(&parent, &second);

        let third = Rc::new(RefCell::new(Task::new()));
        Task::add_child_task_before(&parent, &third, &second);

        let children = parent.borrow().get_child_tasks();
        assert_eq!(children.len(), 3);
        assert!(Rc::ptr_eq(&children[0], &first));
        assert!(Rc::ptr_eq(&children[1], &third));
        assert!(Rc::ptr_eq(&children[2], &second));

// When the sibling task to insert before is not in the list, append to the end
        let outsider = Rc::new(RefCell::new(Task::new()));
        let fourth = Rc::new(RefCell::new(Task::new()));
        Task::add_child_task_before(&parent, &fourth, &outsider);
        let children = parent.borrow().get_child_tasks();
        assert_eq!(children.len(), 4);
        assert!(Rc::ptr_eq(children.last().expect("not empty"), &fourth));

        Task::clear_child_tasks(&parent);
        assert!(!parent.borrow().has_child_tasks());
        assert!(!parent.borrow().get_summary());
        assert!(third.borrow().get_parent_task().is_some());

// clearChildTasks does not rewrite the summary flag when the list is empty
        parent.borrow_mut().set_summary(true);
        Task::clear_child_tasks(&parent);
        assert!(parent.borrow().get_summary());
    }

    #[test]
    fn child_tasks_can_be_attached_by_outline_level() {
        let root = Rc::new(RefCell::new(Task::new()));
        root.borrow_mut().set_outline_level(Some(1));

        let level2 = Rc::new(RefCell::new(Task::new()));
        level2.borrow_mut().set_outline_level(Some(2));
        Task::add_child_task_at_level(&root, &level2, 2);
        assert_eq!(root.borrow().get_child_tasks().len(), 1);

// A deeper level should be attached to the last child task
        let level3 = Rc::new(RefCell::new(Task::new()));
        Task::add_child_task_at_level(&root, &level3, 3);
        assert_eq!(root.borrow().get_child_tasks().len(), 1);
        assert_eq!(level2.borrow().get_child_tasks().len(), 1);

// Does nothing when the level does not match the current one and there are no child tasks
        let other = Rc::new(RefCell::new(Task::new()));
        other.borrow_mut().set_outline_level(Some(5));
        let orphan = Rc::new(RefCell::new(Task::new()));
        Task::add_child_task_at_level(&other, &orphan, 2);
        assert!(!other.borrow().has_child_tasks());
    }

    #[test]
    fn generated_wbs_uses_parent_numbering() {
        let mut root = Task::new();
        root.set_unique_id(Some(0));
        root.generate_wbs(None);
        assert_eq!(root.get_wbs(), Some("0".to_owned()));
        root.generate_outline_number(None);
        assert_eq!(root.get_outline_number(), Some("0".to_owned()));

// When the parent WBS is "0", use "number of child tasks + 1" directly
        let mut zero_parent = Task::new();
        zero_parent.set_wbs(Some("0".to_owned()));
        let zero_parent = Rc::new(RefCell::new(zero_parent));
        let mut child = Task::new();
        child.generate_wbs(Some(&zero_parent));
        assert_eq!(child.get_wbs(), Some("1".to_owned()));

// When the parent WBS is not "0", append ".n"; the outline number first strips a trailing ".0"
        let mut parent = Task::new();
        parent.set_wbs(Some("1".to_owned()));
        parent.set_outline_number(Some("1.0".to_owned()));
        let parent = Rc::new(RefCell::new(parent));
        child.generate_wbs(Some(&parent));
        assert_eq!(child.get_wbs(), Some("1.1".to_owned()));
        child.generate_outline_number(Some(&parent));
        assert_eq!(child.get_outline_number(), Some("1.1".to_owned()));

// Top-level tasks are numbered by "project file child task count + 1" (1 when there is no parent file)
        let mut top = Task::new();
        top.set_unique_id(Some(1));
        top.generate_wbs(None);
        assert_eq!(top.get_wbs(), Some("1".to_owned()));
    }

    #[test]
    fn compare_to_orders_by_id() {
        let mut first = Task::new();
        first.set_id(Some(1));
        let mut second = Task::new();
        second.set_id(Some(2));

        assert_eq!(first.compare_to(&second), Ordering::Less);
        assert_eq!(second.compare_to(&first), Ordering::Greater);
        assert_eq!(first.compare_to(&first), Ordering::Equal);

// Treats an unset ID as 0 (Java: NumberHelper.getInt(null) == 0)
        let third = Task::new();
        assert_eq!(third.compare_to(&first), Ordering::Less);
    }

    #[test]
    fn display_matches_java_to_string() {
        let mut task = Task::new();
        task.set_id(Some(1));
        task.set_unique_id(Some(2));
        task.set_name(Some("X".to_owned()));

        assert_eq!(task.to_string(), "[Task id=1 uniqueID=2 name=X]");

        task.set_activity_id(Some("A".to_owned()));
        assert_eq!(task.to_string(), "[Task id=1 uniqueID=2 activityID=A name=X]");

        task.set_external_task(true);
        task.set_subproject_task_unique_id(Some(9));
        task.set_subproject_task_id(Some(3));
// Java: the external task branch no longer appends an extra "]"
        assert_eq!(
            task.to_string(),
            "[Task id=1 uniqueID=2 activityID=A name=X [EXTERNAL uid=9 id=3]"
        );
    }

    #[test]
    fn events_can_be_disabled() {
        let mut task = Task::new();

        task.disable_events();
        task.set_cost(Some(10.0));
        task.set_baseline_cost(Some(1.0));
// Dependency cleanup is disabled, but values are still computed on demand (the Rust side does not cache calculation results)
        assert_eq!(task.get_cost_variance(), Some(9.0));
        task.enable_events();
    }

    #[test]
    fn project_entity_traits_are_implemented() {
        let mut task = Task::new();

        ProjectEntityWithMutableUniqueID::set_unique_id(&mut task, Some(11));
        assert_eq!(ProjectEntityWithUniqueID::get_unique_id(&task), Some(11));

        ProjectEntityWithID::set_id(&mut task, Some(3));
        assert_eq!(ProjectEntityWithID::get_id(&task), Some(3));

// The inherent method of the same name accesses the same field
        assert_eq!(task.get_unique_id(), Some(11));
        assert_eq!(task.get_id(), Some(3));
    }
}
