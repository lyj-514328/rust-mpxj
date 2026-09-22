//! Corresponds to Java: `org/mpxj/ProjectContext.java`
//!
//! `ProjectContext` is the hub of the domain model: it is held by `ProjectFile` and provides a single entry point for resources,
//! calendars, custom fields, shifts, units of measure, and other containers shared across multiple projects.
//!
//! # Java fields and getters not ported (their modules are not declared in `src/lib.rs`)
//!
//! The following containers hold data used by formats other than the MPP/MPX readers (Primavera / Asta, etc.),
//! and the corresponding modules are not declared in `src/lib.rs`; the Rust side provides neither fields nor getters:
//!
//! | Java field / getter | Notes |
//! |---|---|
//! | `m_locations` / `getLocations()` | `LocationContainer` (module not declared) |
//! | `m_expenseCategories` / `getExpenseCategories()` | `ExpenseCategoryContainer` (module not declared) |
//! | `m_costAccounts` / `getCostAccounts()` | `CostAccountContainer` (module not declared) |
//! | `m_activityCodes` / `getActivityCodes()` | `ActivityCodeContainer` (module not declared) |
//! | `m_projectCodes` / `getProjectCodes()` | `ProjectCodeContainer` (module not declared) |
//! | `m_resourceCodes` / `getResourceCodes()` | `ResourceCodeContainer` (module not declared) |
//! | `m_roleCodes` / `getRoleCodes()` | `RoleCodeContainer` (module not declared) |
//! | `m_resourceAssignmentCodes` / `getResourceAssignmentCodes()` | `ResourceAssignmentCodeContainer` (module not declared) |
//! | `m_currencies` / `getCurrencies()` | `CurrencyContainer` (module not declared) |
//!
//! # Differences from Java
//!
//! - **Object lifecycle**: Java's `ProjectContext` is managed by the GC, and entities hold a direct strong reference to it;
//! on the Rust side `ProjectContext` lives in `Rc<RefCell<ProjectContext>>`, and entities refer back to it with
//! a `Weak<RefCell<ProjectContext>>` (see [`ProjectContext::shared`]).
//! - **How child containers are held**: `ProjectContext` holds each child container directly as a plain field (value semantics);
//! the child containers' back-reference to the context (such as [`crate::project_calendar_container::ProjectCalendarContainer`])
//! uses `Weak`, and [`ProjectContext::shared`] completes the wiring.
//! - **Mutable accessors**: Java's `getCalendars()` and similar return mutable collection objects, so callers can directly
//! call `m_context.getCalendars().add(...)`. Rust's `&self` getters cannot provide that semantics,
//! so for containers held with value semantics (`ProjectCalendarContainer` and the like), besides the getter of the same name there is also
//! a `*_mut` accessor (Rust-specific, not a Java method); [`ProjectContext::get_custom_fields`]
//! returns an `Rc<RefCell<CustomFieldContainer>>`, and mutation goes through its `RefCell`, so no separate `_mut` is provided.
//! - **Project list**: Java's `m_projects` holds strong references to `ProjectFile` (in the `ProjectFile`
//! constructor, `m_context.getProjects().add(this)`). A strong reference on the Rust side would form an
//! `Rc` cycle with `ProjectFile → ProjectContext`, so it is changed to `Weak<RefCell<ProjectFile>>`
//! and registered via [`ProjectContext::add_project`], upgraded and returned by [`ProjectContext::get_projects`].
//! - **Exception list**: Java's `List<Exception>` corresponds to the Rust side's unified error type
//!   [`crate::mpxj_exception::MpxjError`].
//! - **Wiring of the sequence provider**: Java's `new WorkContourContainer(this)` /
//! `new NotesTopicContainer(this)` hands the context itself directly to the container. If the Rust side let the container strongly reference
//! the context, it would form an `Rc` cycle of `ProjectContext → container → ProjectContext`, causing a memory leak,
//! so the Rust side inserts an adapter that **holds only a weak reference to the context**,
//! `ContextSequenceProvider` (see the end of the file), wired up by [`ProjectContext::shared`];
//! it forwards requests to the context, thereby sharing the same set of sequence providers as Java.
//! - **Wiring of `ProjectCalendarContainer`**: this container likewise holds only a weak reference to the context,
//! and [`ProjectContext::shared`] calls `set_context` to complete the wiring.
//! - **Borrow splitting for calendar numbering**: Java can read the context configuration and sequences back inside the container; on the Rust side,
//! obtaining the container from `get_calendars_mut()` already borrows the context, so the same `ObjectSequence` ID is
//! obtained in advance through this type's creation entry point and then handed to the container to construct calendars.
//! - **The `Class<?>` parameter**: Java's `getUniqueIdObjectSequence(Class<?> c)` /
//! `resetUniqueIdObjectSequence(Class<?> c)` use `c.getName()` as the map key. On the Rust side, following the ported
//! [`crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider`], the key is changed to
//! [`std::any::TypeId`], and it returns a shared [`ObjectSequence`] (`Rc<ObjectSequence>`,
//! consistent with the semantics of Java's `computeIfAbsent` returning the same instance).
//! - **Cross-module interfaces are aligned**: `crate::structured_notes` has been changed to obtain
//! sequences via `get_unique_id_object_sequence::<StructuredNotes>(context)` ([`std::any::TypeId`] key)
//! and to use the `i32` return value of `sync_or_get_next` directly, matching
//!   [`crate::unique_id_object_sequence_provider`] / [`crate::common::object_sequence`]
//! this exactly (`crate::project_calendar` is likewise aligned to `TypeId`).
//! - `Debug` is not derived: the `EventManager` (and `ProjectFile`) in the field types do not implement `Debug`,
//! so this type can only be implemented by hand too (the relevant modules are all in place; what is missing is their own `Debug` implementations).
//!
//! # Constructor signatures of the containers used (verified as consistent)
//!
//! The following construction expressions are written according to the signatures already in place in each module:
//! `UnitOfMeasureContainer::new()`,`WorkContourContainer::new(sequence_provider)`,
//! `NotesTopicContainer::new(sequence_provider)`, `ProjectCalendarContainer::new_with_provider(sequence_provider)`, `CustomFieldContainer::new()` (returns
//! `Rc<RefCell<Self>>`),`UserDefinedFieldContainer::new()`,`ShiftContainer::new()`,
//! `ShiftPeriodContainer::new()`,`ProjectCalendarContainer::new()`,`ResourceContainer::new()`,
//! `TimeUnitDefaults::new()`,`ProjectConfig::new()`,`EventManager::new()`.
//! The unit tests also call each container's `is_empty()` (corresponding to Java `ArrayList.isEmpty()`).

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use crate::common::object_sequence::ObjectSequence;
use crate::custom_field_container::CustomFieldContainer;
use crate::event_manager::EventManager;
use crate::mpxj_exception::MpxjError;
use crate::notes_topic_container::NotesTopicContainer;
use crate::project_calendar::ProjectCalendar;
use crate::project_calendar_container::ProjectCalendarContainer;
use crate::project_config::ProjectConfig;
use crate::project_file::ProjectFile;
use crate::resource_container::ResourceContainer;
use crate::shift_container::ShiftContainer;
use crate::shift_period_container::ShiftPeriodContainer;
use crate::time_unit_defaults::TimeUnitDefaults;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;
use crate::unit_of_measure_container::UnitOfMeasureContainer;
use crate::user_defined_field_container::UserDefinedFieldContainer;
use crate::work_contour_container::WorkContourContainer;

/// A container for common data that can be shared among multiple `ProjectFile` instances.
///
/// Corresponds to Java: `public class ProjectContext implements UniqueIdObjectSequenceProvider`
pub struct ProjectContext {
    /// Corresponds to Java field `private final UnitOfMeasureContainer m_unitsOfMeasure = new UnitOfMeasureContainer(this);`
    units_of_measure: UnitOfMeasureContainer,
    /// Corresponds to Java field `private final WorkContourContainer m_workContours = new WorkContourContainer(this);`
    work_contours: WorkContourContainer,
    /// Corresponds to Java field `private final NotesTopicContainer m_notesTopics = new NotesTopicContainer(this);`
    notes_topics: NotesTopicContainer,
    /// Corresponds to Java field `private final CustomFieldContainer m_customFields = new CustomFieldContainer();`
    ///
/// [`CustomFieldContainer::new`] returns `Rc<RefCell<Self>>` (`CustomField` needs a weak reference back to
/// its parent container), so the Rust side holds it as `Rc<RefCell<..>>`.
    custom_fields: Rc<RefCell<CustomFieldContainer>>,
    /// Corresponds to Java field `private final UserDefinedFieldContainer m_userDefinedFields = new UserDefinedFieldContainer(m_customFields);`
    user_defined_fields: UserDefinedFieldContainer,
    /// Corresponds to Java field `private final ShiftContainer m_shifts = new ShiftContainer(this);`
    shifts: ShiftContainer,
    /// Corresponds to Java field `private final ShiftPeriodContainer m_shiftPeriods = new ShiftPeriodContainer(this);`
    shift_periods: ShiftPeriodContainer,
    /// Corresponds to Java field `private final ProjectCalendarContainer m_calendars = new ProjectCalendarContainer(this);`
    calendars: ProjectCalendarContainer,
    /// Corresponds to Java field `private final ResourceContainer m_resources = new ResourceContainer(this);`
    resources: ResourceContainer,
    /// Corresponds to Java field `private final List<ProjectFile> m_projects = new ArrayList<>();`
    ///
/// The Rust side stores a weak reference to avoid an `Rc` cycle with `ProjectFile → ProjectContext`.
    projects: Vec<Weak<RefCell<ProjectFile>>>,
    /// Corresponds to Java field `private final TimeUnitDefaults m_timeUnitDefaults = new TimeUnitDefaults();`
    time_unit_defaults: TimeUnitDefaults,
    /// Corresponds to Java field `private final ProjectConfig m_projectConfig = new ProjectConfig();`
    project_config: ProjectConfig,
    /// Corresponds to Java field `private final Map<String, ObjectSequence> m_uniqueIdObjectSequences = new HashMap<>();`
    ///
/// Java's key is `Class.getName()`; on the Rust side, following the ported trait, it is changed to
/// [`std::any::TypeId`], with a shared `Rc<ObjectSequence>` as the value.
    unique_id_object_sequences: HashMap<TypeId, Rc<ObjectSequence>>,
    /// Corresponds to Java field `private final List<Exception> m_ignoredErrors = new ArrayList<>();`
    ignored_errors: Vec<MpxjError>,
    /// Corresponds to Java field `private final EventManager m_eventManager = new EventManager();`
    event_manager: EventManager,
/// The sequence provider used by child containers (Rust-specific wiring; see the file header).
    ///
/// Corresponds to Java: the `this` passed in each container's constructor (`ProjectContext` itself is
    /// `UniqueIdObjectSequenceProvider`).
    sequence_provider: Rc<RefCell<ContextSequenceProvider>>,
}

impl ProjectContext {
/// Constructor (each field is initialized in place).
    ///
/// Corresponds to Java: the implicit no-argument constructor `ProjectContext()`.
    ///
/// Users who need to share it across entities should use [`ProjectContext::shared`] instead, which additionally completes the back-reference wiring
/// between the child containers and the context.
    pub fn new() -> Self {
// Java: passes `this` when constructing each container; the Rust side passes an adapter that holds only a weak reference to the context
// (wired up by ProjectContext::shared; see the file header).
        let sequence_provider = Rc::new(RefCell::new(ContextSequenceProvider::new()));
        let work_contour_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            sequence_provider.clone();
        let notes_topic_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            sequence_provider.clone();
        let calendar_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            sequence_provider.clone();

        Self {
            units_of_measure: UnitOfMeasureContainer::new(),
            work_contours: WorkContourContainer::new(work_contour_provider),
            notes_topics: NotesTopicContainer::new(notes_topic_provider),
            custom_fields: CustomFieldContainer::new(),
            user_defined_fields: UserDefinedFieldContainer::new(),
            shifts: ShiftContainer::new(),
            shift_periods: ShiftPeriodContainer::new(),
            calendars: ProjectCalendarContainer::new_with_provider(calendar_provider),
            resources: ResourceContainer::new(),
            projects: Vec::new(),
            time_unit_defaults: TimeUnitDefaults::new(),
            project_config: ProjectConfig::new(),
            unique_id_object_sequences: HashMap::new(),
            ignored_errors: Vec::new(),
            event_manager: EventManager::new(),
            sequence_provider,
        }
    }

/// Construct a shareable context (Rust-specific).
    ///
/// Java uses `new ProjectContext()` to obtain a directly shareable reference and passes `this`
/// to each child container at construction. Rust cannot obtain its own `Rc` inside `Self::new()`, so here an
/// `Rc<RefCell<ProjectContext>>` is constructed first, then the wiring is completed for the child containers that need to refer back to the context
/// (currently [`ProjectCalendarContainer`]).
    pub fn shared() -> Rc<RefCell<ProjectContext>> {
        let context = Rc::new(RefCell::new(ProjectContext::new()));

        {
            let mut context_ref = context.borrow_mut();
// Java: `new ProjectCalendarContainer(this)` —— this method adds the back-reference wiring.
            context_ref.calendars.set_context(&context);

// Java: `new ResourceContainer(this)` —— same as above (`ResourceContainer`'s sequence provider
// also needs the context to share the same set of `ObjectSequence` instances as Java).
            context_ref.resources.set_context(&context);

            // Java: `new WorkContourContainer(this)` / `new NotesTopicContainer(this)`
// —— attach the sequence provider adapter to this context (it only writes a weak reference and does not borrow the context again).
            let sequence_provider = Rc::clone(&context_ref.sequence_provider);
            sequence_provider.borrow_mut().set_context(&context);
        }

        context
    }

/// Return the units of measure container available in the current context.
    ///
    /// Corresponds to Java: `public UnitOfMeasureContainer getUnitsOfMeasure()`
    pub fn get_units_of_measure(&self) -> &UnitOfMeasureContainer {
        &self.units_of_measure
    }

/// Return a mutable reference to the units of measure container (Rust-specific: Java's getter returns a mutable collection object).
    pub fn get_units_of_measure_mut(&mut self) -> &mut UnitOfMeasureContainer {
        &mut self.units_of_measure
    }

/// Return the work contour container available in the current context.
    ///
    /// Corresponds to Java: `public WorkContourContainer getWorkContours()`
    pub fn get_work_contours(&self) -> &WorkContourContainer {
        &self.work_contours
    }

/// Return a mutable reference to the work contour container (Rust-specific).
    pub fn get_work_contours_mut(&mut self) -> &mut WorkContourContainer {
        &mut self.work_contours
    }

/// Return the notes topic container available in the current context.
    ///
    /// Corresponds to Java: `public NotesTopicContainer getNotesTopics()`
    pub fn get_notes_topics(&self) -> &NotesTopicContainer {
        &self.notes_topics
    }

/// Return a mutable reference to the notes topic container (Rust-specific).
    pub fn get_notes_topics_mut(&mut self) -> &mut NotesTopicContainer {
        &mut self.notes_topics
    }

/// Return the custom field container available in the current context.
    ///
    /// Corresponds to Java: `public CustomFieldContainer getCustomFields()`
    ///
/// [`crate::custom_field_container::CustomFieldContainer::new`] returns `Rc<RefCell<Self>>`,
/// so the Rust side returns a clone of the shared handle; adding and removing fields is done through the returned `RefCell`,
/// so no additional `_mut` accessor is provided.
    pub fn get_custom_fields(&self) -> Rc<RefCell<CustomFieldContainer>> {
        Rc::clone(&self.custom_fields)
    }

/// Return the user-defined field container available in the current context.
    ///
    /// Corresponds to Java: `public UserDefinedFieldContainer getUserDefinedFields()`
    pub fn get_user_defined_fields(&self) -> &UserDefinedFieldContainer {
        &self.user_defined_fields
    }

/// Return a mutable reference to the user-defined field container (Rust-specific).
    pub fn get_user_defined_fields_mut(&mut self) -> &mut UserDefinedFieldContainer {
        &mut self.user_defined_fields
    }

/// Return the shift container available in the current context.
    ///
    /// Corresponds to Java: `public ShiftContainer getShifts()`
    pub fn get_shifts(&self) -> &ShiftContainer {
        &self.shifts
    }

/// Return a mutable reference to the shift container (Rust-specific).
    pub fn get_shifts_mut(&mut self) -> &mut ShiftContainer {
        &mut self.shifts
    }

/// Return the shift period container available in the current context.
    ///
    /// Corresponds to Java: `public ShiftPeriodContainer getShiftPeriods()`
    pub fn get_shift_periods(&self) -> &ShiftPeriodContainer {
        &self.shift_periods
    }

/// Return a mutable reference to the shift period container (Rust-specific).
    pub fn get_shift_periods_mut(&mut self) -> &mut ShiftPeriodContainer {
        &mut self.shift_periods
    }

/// Return the calendar container available in the current context.
    ///
    /// Corresponds to Java: `public ProjectCalendarContainer getCalendars()`
    pub fn get_calendars(&self) -> &ProjectCalendarContainer {
        &self.calendars
    }

/// Return a mutable reference to the calendar container (Rust-specific).
    pub fn get_calendars_mut(&mut self) -> &mut ProjectCalendarContainer {
        &mut self.calendars
    }

/// Create a calendar and obtain a unique ID according to the configuration while the context is still accessible.
    ///
/// Corresponds to Java: `getCalendars().add()`; a Rust-specific entry point for borrow splitting.
    pub(crate) fn add_calendar(&mut self) -> Rc<RefCell<ProjectCalendar>> {
        let unique_id = self.next_calendar_unique_id(false);
        self.calendars.add_with_unique_id(unique_id)
    }

/// Create the default base calendar.
    ///
/// Corresponds to Java: `getCalendars().addDefaultBaseCalendar()`; a Rust-specific entry point for borrow splitting.
    pub(crate) fn add_default_base_calendar(&mut self) -> Rc<RefCell<ProjectCalendar>> {
        let unique_id = self.next_calendar_unique_id(false);
        self.calendars.add_default_base_calendar_with_unique_id(unique_id)
    }

/// Create the default derived calendar.
    ///
/// Corresponds to Java: `getCalendars().addDefaultDerivedCalendar()`; a Rust-specific entry point for borrow splitting.
    pub(crate) fn add_default_derived_calendar(&mut self) -> Rc<RefCell<ProjectCalendar>> {
        let unique_id = self.next_calendar_unique_id(false);
        self.calendars.add_default_derived_calendar_with_unique_id(unique_id)
    }

/// Find the default calendar; when the container is empty it allocates an ID following Java behavior even if automatic numbering is off.
    ///
/// Corresponds to Java: `getCalendars().findOrCreateDefaultCalendar()`; a Rust-specific entry point for borrow splitting.
    pub(crate) fn find_or_create_default_calendar(
        &mut self,
    ) -> Option<Rc<RefCell<ProjectCalendar>>> {
        let unique_id = if self.calendars.is_empty() {
            self.next_calendar_unique_id(true)
        } else {
            None
        };
        self.calendars.find_or_create_default_calendar_with_unique_id(unique_id)
    }

/// Obtain the next ID from the shared sequence used by the Java calendar constructor.
    ///
/// Corresponds to Java: the automatic numbering in `ProjectCalendar(ProjectContext, boolean)`;
/// `findOrCreateDefaultCalendar()` can force one allocation when the container is empty.
    fn next_calendar_unique_id(&mut self, force: bool) -> Option<i32> {
        if force || self.project_config.get_auto_calendar_unique_id() {
            Some(
                self.get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>())
                    .get_next(),
            )
        } else {
            None
        }
    }

/// Return the resource container available in the current context.
    ///
    /// Corresponds to Java: `public ResourceContainer getResources()`
    pub fn get_resources(&self) -> &ResourceContainer {
        &self.resources
    }

/// Return a mutable reference to the resource container (Rust-specific).
    pub fn get_resources_mut(&mut self) -> &mut ResourceContainer {
        &mut self.resources
    }

/// Return the project list in this context (the Rust side returns a snapshot of upgraded strong references; released projects are ignored).
    ///
    /// Corresponds to Java: `public List<ProjectFile> getProjects()`
    pub fn get_projects(&self) -> Vec<Rc<RefCell<ProjectFile>>> {
        self.projects.iter().filter_map(Weak::upgrade).collect()
    }

/// Register a project in this context (a Rust-specific entry point).
    ///
/// Corresponds to Java: `m_context.getProjects().add(this)` in the `ProjectFile` constructor
    pub fn add_project(&mut self, project: &Rc<RefCell<ProjectFile>>) {
        self.projects.push(Rc::downgrade(project));
    }

/// Return the duration-unit-related defaults in the context.
    ///
    /// Corresponds to Java: `public TimeUnitDefaults getTimeUnitDefaults()`
    pub fn get_time_unit_defaults(&self) -> &TimeUnitDefaults {
        &self.time_unit_defaults
    }

/// Return a mutable reference to the duration unit defaults (Rust-specific).
    pub fn get_time_unit_defaults_mut(&mut self) -> &mut TimeUnitDefaults {
        &mut self.time_unit_defaults
    }

/// Return the MPXJ configuration shared by the projects in this context.
    ///
    /// Corresponds to Java: `public ProjectConfig getProjectConfig()`
    pub fn get_project_config(&self) -> &ProjectConfig {
        &self.project_config
    }

/// Return a mutable reference to the MPXJ configuration (Rust-specific).
    pub fn get_project_config_mut(&mut self) -> &mut ProjectConfig {
        &mut self.project_config
    }

/// Record an error that was ignored while reading context data.
    ///
    /// Corresponds to Java: `public void addIgnoredError(Exception ex)`
    pub fn add_ignored_error(&mut self, error: MpxjError) {
        self.ignored_errors.push(error);
    }

/// Return the list of errors ignored while reading context data.
    ///
    /// Corresponds to Java: `public List<Exception> getIgnoredErrors()`
    pub fn get_ignored_errors(&self) -> &[MpxjError] {
        &self.ignored_errors
    }

/// Return the event manager.
    ///
    /// Corresponds to Java: `public EventManager getEventManager()`
    pub fn get_event_manager(&self) -> &EventManager {
        &self.event_manager
    }

/// Return a mutable reference to the event manager (Rust-specific: triggering events requires mutable access).
    pub fn get_event_manager_mut(&mut self) -> &mut EventManager {
        &mut self.event_manager
    }

/// Remove the unique ID sequence of the given type so that numbering for that type restarts from 1.
    ///
    /// Corresponds to Java: `public void resetUniqueIdObjectSequence(Class<?> c)`
    ///
/// Java uses `c.getName()` as the map key; on the Rust side, following the ported trait, a [`TypeId`] is passed instead.
    pub fn reset_unique_id_object_sequence(&mut self, type_id: TypeId) {
        self.unique_id_object_sequences.remove(&type_id);
    }
}

impl Default for ProjectContext {
/// Corresponds to Java: the implicit no-argument constructor `ProjectContext()`.
    fn default() -> Self {
        Self::new()
    }
}

impl UniqueIdObjectSequenceProvider for ProjectContext {
/// Return (creating if absent) the unique ID sequence for the given type.
    ///
    /// Corresponds to Java: `@Override public ObjectSequence getUniqueIdObjectSequence(Class<?> c)`
    ///
/// Java's map key is `Class.getName()`; on the Rust side, per the trait definition, it is changed to [`TypeId`];
/// Java's `computeIfAbsent` returns the same cached `ObjectSequence` instance,
/// and the Rust side uses `Rc<ObjectSequence>` to express the sharing of the same instance.
    fn get_unique_id_object_sequence(&mut self, type_id: TypeId) -> Rc<ObjectSequence> {
        sequence_for(&mut self.unique_id_object_sequences, type_id)
    }
}

/// Return (or create) the unique ID sequence corresponding to the given type.
///
/// Corresponds to Java: `m_uniqueIdObjectSequences.computeIfAbsent(c.getName(), x -> new ObjectSequence(1))`
fn sequence_for(
    sequences: &mut HashMap<TypeId, Rc<ObjectSequence>>,
    type_id: TypeId,
) -> Rc<ObjectSequence> {
    Rc::clone(
        sequences
            .entry(type_id)
            .or_insert_with(|| Rc::new(ObjectSequence::new(1))),
    )
}

/// The sequence provider used by child containers (Rust-specific wiring; see the file header).
///
/// In Java, `new WorkContourContainer(this)` / `new NotesTopicContainer(this)` hand
/// `ProjectContext` itself (which implements `UniqueIdObjectSequenceProvider`) to the container; if the Rust side let the container
/// strongly reference the context, it would form an `Rc` cycle of `ProjectContext → container → ProjectContext`, causing a memory leak.
/// So here an adapter that **holds only a weak reference to the context** is provided: it forwards requests to the context's
/// `getUniqueIdObjectSequence`, sharing the same set of sequence providers as Java.
///
/// When the context is not yet wired up (during [`ProjectContext::new`] construction) or is already borrowed, it degrades gracefully to this adapter's own
/// standalone sequence provider, guaranteeing no panic and no failure (Java has no such case).
pub(crate) struct ContextSequenceProvider {
/// The target context (a weak reference, to avoid an `Rc` cycle).
    context: Weak<RefCell<ProjectContext>>,
/// The fallback sequence provider used when the context is unavailable.
    fallback: HashMap<TypeId, Rc<ObjectSequence>>,
}

impl ContextSequenceProvider {
    pub(crate) fn new() -> Self {
        Self {
            context: Weak::new(),
            fallback: HashMap::new(),
        }
    }

/// Wire up the target context (only writes a weak reference).
    fn set_context(&mut self, context: &Rc<RefCell<ProjectContext>>) {
        self.context = Rc::downgrade(context);
    }
}

impl UniqueIdObjectSequenceProvider for ContextSequenceProvider {
    fn get_unique_id_object_sequence(&mut self, type_id: TypeId) -> Rc<ObjectSequence> {
        if let Some(context) = self.context.upgrade() {
            if let Ok(mut context_ref) = context.try_borrow_mut() {
                return context_ref.get_unique_id_object_sequence(type_id);
            }
        }

        Rc::clone(
            self.fallback
                .entry(type_id)
                .or_insert_with(|| Rc::new(ObjectSequence::new(1))),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_calendar::{ProjectCalendar, DEFAULT_BASE_CALENDAR_NAME};
    use crate::time_unit_defaults_container::TimeUnitDefaultsContainer;

/// A test "entity" type, used only to distinguish `TypeId`s.
    struct Alpha;

/// A test "entity" type, used only to distinguish `TypeId`s.
    struct Beta;

    #[test]
    fn new_context_containers_are_empty() {
        let context = ProjectContext::new();

        assert!(context.get_units_of_measure().is_empty());
        assert!(context.get_work_contours().is_empty());
        assert!(context.get_notes_topics().is_empty());
        assert!(context.get_custom_fields().borrow().is_empty());
        assert!(context.get_user_defined_fields().is_empty());
        assert!(context.get_shifts().is_empty());
        assert!(context.get_shift_periods().is_empty());
        assert!(context.get_calendars().is_empty());
        assert!(context.get_resources().is_empty());
        assert!(context.get_projects().is_empty());
        assert!(context.get_ignored_errors().is_empty());
    }

    #[test]
    fn time_unit_defaults_and_project_config_match_java_defaults() {
        let context = ProjectContext::new();

        assert_eq!(
            context.get_time_unit_defaults().get_minutes_per_day(),
            Some(480)
        );
        assert!(context.get_project_config().get_auto_calendar_unique_id());
    }

    #[test]
    fn mutable_accessors_reach_the_same_containers() {
        let mut context = ProjectContext::new();

        context.get_calendars_mut().add_calendar(Rc::new(
            RefCell::new(ProjectCalendar::new(Weak::new(), false)),
        ));
        assert_eq!(context.get_calendars().len(), 1);

        context.get_project_config_mut().set_auto_wbs(false);
        assert!(!context.get_project_config().get_auto_wbs());

        context
            .get_time_unit_defaults_mut()
            .set_minutes_per_day(Some(600));
        assert_eq!(
            context.get_time_unit_defaults().get_minutes_per_day(),
            Some(600)
        );
    }

    #[test]
    fn unique_id_sequences_are_per_type_and_resettable() {
        let mut context = ProjectContext::new();

// The same type shares the same sequence
        let sequence = context.get_unique_id_object_sequence(TypeId::of::<Alpha>());
        assert_eq!(sequence.get_next(), 1);
        assert_eq!(sequence.get_next(), 2);
        assert!(Rc::ptr_eq(
            &sequence,
            &context.get_unique_id_object_sequence(TypeId::of::<Alpha>())
        ));

// Different types count independently
        assert_eq!(
            context
                .get_unique_id_object_sequence(TypeId::of::<Beta>())
                .get_next(),
            1
        );

// After synchronizing with a known in-use number, the sequence follows after it
        let sequence = context.get_unique_id_object_sequence(TypeId::of::<Alpha>());
        assert_eq!(sequence.sync_or_get_next(Some(10)), 10);
        assert_eq!(sequence.get_next(), 11);

// After reset, numbering restarts from 1
        context.reset_unique_id_object_sequence(TypeId::of::<Alpha>());
        assert_eq!(
            context
                .get_unique_id_object_sequence(TypeId::of::<Alpha>())
                .get_next(),
            1
        );
    }

    #[test]
    fn shared_context_implements_unique_id_object_sequence_provider() {
        let context = ProjectContext::shared();
        let mut guard = context.borrow_mut();
        let provider: &mut ProjectContext = &mut guard;

        assert_eq!(
            UniqueIdObjectSequenceProvider::get_unique_id_object_sequence(
                provider,
                TypeId::of::<Alpha>()
            )
            .get_next(),
            1
        );
    }

    #[test]
    fn ignored_errors_are_collected() {
        let mut context = ProjectContext::new();

        context.add_ignored_error(MpxjError::new("first"));
        context.add_ignored_error(MpxjError::new("second"));

        assert_eq!(context.get_ignored_errors().len(), 2);
        assert_eq!(
            context.get_ignored_errors().first().map(MpxjError::message),
            Some("first")
        );
    }

    #[test]
    fn shared_context_calendar_container_is_usable() {
        let context = ProjectContext::shared();

        let created = {
            let mut context_ref = context.borrow_mut();
            let calendar = context_ref.find_or_create_default_calendar();
            assert_eq!(context_ref.get_calendars().len(), 1);
            calendar
        }
        .expect("empty context creates default base calendar");

        assert_eq!(
            created.borrow().get_name(),
            Some(DEFAULT_BASE_CALENDAR_NAME)
        );
        assert_eq!(created.borrow().get_unique_id(), Some(1));

// The context itself is still available (the weak reference caused no leak or dangling)
        assert_eq!(context.borrow().get_calendars().len(), 1);
    }

/// Corresponds to Java: `ProjectCalendarContainer.add()` shares the context calendar sequence with the constructor.
    #[test]
    fn calendar_creation_uses_shared_sequence_while_context_is_borrowed() {
        let context = ProjectContext::shared();
        let (first, second, derived) = {
            let mut context_ref = context.borrow_mut();
            (
                context_ref.add_calendar(),
                context_ref.add_default_base_calendar(),
                context_ref.add_default_derived_calendar(),
            )
        };

        assert_eq!(first.borrow().get_unique_id(), Some(1));
        assert_eq!(second.borrow().get_unique_id(), Some(2));
        assert_eq!(derived.borrow().get_unique_id(), Some(3));
        assert!(context.borrow().get_calendars().get_by_unique_id(Some(2)).is_some());
        assert_eq!(
            context
                .borrow_mut()
                .get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>())
                .get_next(),
            4,
        );
    }

/// Corresponds to Java: when automatic numbering is off, only `findOrCreateDefaultCalendar` forcibly supplies an ID.
    #[test]
    fn default_calendar_gets_id_even_when_auto_ids_are_disabled() {
        let context = ProjectContext::shared();
        context
            .borrow_mut()
            .get_project_config_mut()
            .set_auto_calendar_unique_id(false);
        let default = context
            .borrow_mut()
            .find_or_create_default_calendar()
            .expect("default calendar");
        let ordinary = context.borrow_mut().add_calendar();

        assert_eq!(default.borrow().get_unique_id(), Some(1));
        assert_eq!(ordinary.borrow().get_unique_id(), None);
        assert_eq!(
            context
                .borrow_mut()
                .get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>())
                .get_next(),
            2
        );
    }

    #[test]
    fn default_matches_new() {
        let context = ProjectContext::default();
        assert!(context.get_calendars().is_empty());
        assert!(context.get_shifts().is_empty());
    }
}
