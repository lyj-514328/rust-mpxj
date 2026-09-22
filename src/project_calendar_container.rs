//! Corresponds to Java: `org/mpxj/ProjectCalendarContainer.java`
//!
//! Skipped / unported Java methods and reasons:
//!
//! | Java | Rust |
//! |---|---|
//! | `ProjectCalendarContainer(ProjectContext context)` | [`ProjectCalendarContainer::new`] + [`ProjectCalendarContainer::set_context`] |
//! | `@Override protected void removed(ProjectCalendar calendar)` | private method `removed` (called by [`ProjectCalendarContainer::remove`]) |
//! | `public ProjectCalendar add()` | [`ProjectCalendarContainer::add`] |
//! | `public ProjectCalendar addDefaultBaseCalendar()` | [`ProjectCalendarContainer::add_default_base_calendar`] |
//! | `public ProjectCalendar addDefaultDerivedCalendar()` | [`ProjectCalendarContainer::add_default_derived_calendar`] |
//! | `public ProjectCalendar findOrCreateDefaultCalendar()` | [`ProjectCalendarContainer::find_or_create_default_calendar`] |
//! | `public ProjectCalendar getByName(String calendarName)` | [`ProjectCalendarContainer::get_by_name`] |
//! | `public Integer getDefaultCalendarUniqueID()` | [`ProjectCalendarContainer::get_default_calendar_unique_id`] |
//! | `public void setDefaultCalendarUniqueID(Integer)` | [`ProjectCalendarContainer::set_default_calendar_unique_id`] |
//! | `add(T)` inherited from `ProjectEntityContainer<ProjectCalendar>` | [`ProjectCalendarContainer::add_calendar`] (Rust has no overloading, so a suffix distinguishes it from the same-named no-argument `add()`) |
//! | `getByUniqueID` / `size` / `isEmpty` / `iterator` / `get` inherited from `ProjectEntityContainer` | [`ProjectCalendarContainer::get_by_unique_id`] / [`ProjectCalendarContainer::len`] / [`ProjectCalendarContainer::is_empty`] / [`ProjectCalendarContainer::iter`] / [`ProjectCalendarContainer::get`] |
//! | `updateUniqueID(T element, Integer old, Integer new)` inherited from `ProjectEntityContainer` | [`ProjectCalendarContainer::update_unique_id_for_calendar`] (`self_ref` provides the element handle) |
//! | `fixUniqueIdClashes`, `getSequenceProvider`, `firstUniqueID`, and `ListWithCallbacks` callback registration inherited from `ProjectEntityContainer` | compose [`ProjectEntityContainer`]; the context first builds a weak-reference sequence adapter, and passes in the real sequence when fixing clashes |
//!
//! # Differences from Java
//!
//! - **Context back-reference**: the Java field `private final ProjectContext m_context` is a strong reference; the Rust side
//! changes it to `Weak<RefCell<ProjectContext>>` to avoid an `Rc` cycle between `ProjectContext` and the container.
//! Java initializes it with `this` in the constructor; Rust cannot obtain its own `Rc` inside `Self::new()`,
//! so it is first constructed with an empty `Weak`, then wired up by [`ProjectCalendarContainer::set_context`]
//! ([`crate::project_context::ProjectContext::shared`] performs the wiring automatically).
//! - **Element form**: Java stores a `ProjectCalendar` reference; a calendar is a mutable shared entity (tasks and resources reference it,
//! and derived calendars point back to the parent calendar); the Rust side stores `Rc<RefCell<ProjectCalendar>>`,
//! and [`crate::project_calendar::ProjectCalendar::new`] returns a value, so [`ProjectCalendarContainer::add`]
//! is responsible for wrapping it in an `Rc<RefCell<..>>`.
//! - **Reentrancy limits of the base class index**: if the context is already borrowed when `ProjectCalendar::set_unique_id` calls back,
//! the container cannot be borrowed; lookup by ID falls back to a linear search, and the base class index is rebuilt from the current elements before fixing clashes.
//! `fix_unique_id_clashes` receives the real context sequence from the caller, avoiding the adapter using
//! a separate degraded sequence on a borrow conflict.
//! - **Context reentrant borrow**: the container is a field of `ProjectContext`, so the caller must first `borrow_mut()` the context
//! to obtain the container's `&mut`; borrowing the same context again inside the container then fails. When creating a calendar through the context,
//! first obtain an ID from the real sequence and pass it into the container; when the container is used standalone, `try_borrow_mut` is still used for graceful degradation.
//! - **The element parameter of `remove`**: in Java it is `ListWithCallbacks.remove(Object)` (lookup by reference equality);
//! [`crate::project_calendar::ProjectCalendar::remove`] calls this container as `remove(self)`,
//! so the Rust side receives `&ProjectCalendar` and uses `RefCell::as_ptr` for a **pointer identity** comparison
//! (equivalent to Java's default `equals`). Because the parameter is an immutable reference, in `removed`
//! "clearing the parent of the removed calendar" can only be executed when the calendar's mutable reference can be borrowed again (usually the caller
//! currently holds a mutable borrow of that calendar, in which case it degrades to skipping); Java has no such restriction.
//! - In `removed`, "clearing the calendar of resources / tasks that reference this calendar" **cannot be restored**: Java gets the resources / tasks
//! directly from the strong `m_context` reference held by the container; the Rust container can only be obtained via
//! `ProjectContext::get_calendars_mut()`, and when `removed` runs the context is necessarily mutably borrowed,
//! so the `try_borrow` inside the resource / task lookup necessarily fails. In contrast, "derived calendars no longer point back to the removed parent
//! calendar" is changed to **directly scanning this container's elements** (semantically equivalent to Java's query through the context), so it can
//! run normally -- unlike the older version of this file header, which claimed it "depends on the not-yet-ported `Resource` / `Task`".

use std::any::TypeId;
use std::cell::RefCell;
use std::fmt::{self, Debug, Formatter};
use std::rc::{Rc, Weak};

use chrono::Weekday;

use crate::day_type::DayType;
use crate::mpx::locale_utility::equals_ignore_case;
use crate::project_calendar::{ProjectCalendar, DEFAULT_BASE_CALENDAR_NAME};
use crate::project_context::ProjectContext;
use crate::project_entity_container::ProjectEntityContainer;
use crate::common::object_sequence::ObjectSequence;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;

/// Collection of calendars participating in the current context (shared by multiple projects).
///
/// Corresponds to Java: `public class ProjectCalendarContainer extends ProjectEntityContainer<ProjectCalendar>`
pub struct ProjectCalendarContainer {
/// Calendars in the container.
    ///
/// Corresponds to Java: the internal list storage of `ProjectEntityContainer`.
    items: ProjectEntityContainer<ProjectCalendar>,

/// Unique ID of the context's default calendar.
    ///
    /// Corresponds to Java field `private Integer m_defaultCalendarUniqueID;`
    default_calendar_unique_id: Option<i32>,

/// Parent context (a weak reference, to avoid an `Rc` cycle).
    ///
    /// Corresponds to Java field `private final ProjectContext m_context;`
    context: Weak<RefCell<ProjectContext>>,
}

impl ProjectCalendarContainer {
/// Constructor.
    ///
    /// Corresponds to Java: `public ProjectCalendarContainer(ProjectContext context)`
    ///
/// The Rust side first constructs it with an empty context weak reference, and [`ProjectCalendarContainer::set_context`] completes the wiring.
    pub fn new() -> Self {
        let provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            Rc::new(RefCell::new(crate::project_context::ContextSequenceProvider::new()));
        Self::new_with_provider(provider)
    }

/// Wire in the already-created sequence provider when the context is constructed.
    ///
/// Corresponds to Java: `super(context)`; the provider only stores a context weak reference, so construction does not require
/// a completed `ProjectContext`.
    pub(crate) fn new_with_provider(
        provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>,
    ) -> Self {
        Self {
            items: ProjectEntityContainer::new(provider),
            default_calendar_unique_id: None,
            context: Weak::new(),
        }
    }

/// Wire up the parent context (Rust-specific: Java receives `this` directly in the constructor).
    ///
/// Corresponds to Java: `m_context = context;` in the constructor
    pub(crate) fn set_context(&mut self, context: &Rc<RefCell<ProjectContext>>) {
        self.context = Rc::downgrade(context);
    }

/// Add an existing calendar to the container (the `add(T)` behavior inherited from the base class `ProjectEntityContainer`).
    ///
/// Corresponds to Java: `ListWithCallbacks.add(E element)` (which in turn triggers `ProjectEntityContainer.added`)
    ///
/// Rust has no overloading, so it is distinguished from the same-named no-argument [`ProjectCalendarContainer::add`] by the `_calendar`
/// suffix.
    pub fn add_calendar(&mut self, calendar: Rc<RefCell<ProjectCalendar>>) {
        if let Ok(mut calendar_ref) = calendar.try_borrow_mut() {
            calendar_ref.set_self_ref(Rc::downgrade(&calendar));
        }
        self.items.add(calendar);
    }

/// Create a new calendar and add it to the container.
    ///
    /// Corresponds to Java: `public ProjectCalendar add()`
    pub fn add(&mut self) -> Rc<RefCell<ProjectCalendar>> {
        // Java: ProjectCalendar calendar = new ProjectCalendar(m_context, false);
// ProjectCalendar::new on the Rust side returns a value, so it is wrapped in an Rc<RefCell<..>> here for sharing.
        let calendar = Rc::new(RefCell::new(ProjectCalendar::new(self.context.clone(), false)));
        self.add_calendar(Rc::clone(&calendar));
        calendar
    }

/// Create a calendar using an ID preallocated by a caller that holds a borrow of the context.
    ///
/// Corresponds to Java: `public ProjectCalendar add()`; a Rust-specific borrow-splitting entry point.
    pub(crate) fn add_with_unique_id(
        &mut self,
        unique_id: Option<i32>,
    ) -> Rc<RefCell<ProjectCalendar>> {
        let calendar = Rc::new(RefCell::new(ProjectCalendar::new_with_unique_id(
            self.context.clone(), false, unique_id,
        )));
        self.add_calendar(Rc::clone(&calendar));
        calendar
    }

/// Create a base calendar named "Standard" and populate it with the default working week and default working hours.
    ///
    /// Corresponds to Java: `public ProjectCalendar addDefaultBaseCalendar()`
    pub fn add_default_base_calendar(&mut self) -> Rc<RefCell<ProjectCalendar>> {
        let calendar = self.add();
        Self::initialize_default_base_calendar(&calendar);
        calendar
    }

/// Create the default base calendar with a preallocated ID when the context is already borrowed.
    ///
/// Corresponds to Java: `public ProjectCalendar addDefaultBaseCalendar()`; a Rust-specific borrow-splitting entry point.
    pub(crate) fn add_default_base_calendar_with_unique_id(
        &mut self, unique_id: Option<i32>,
    ) -> Rc<RefCell<ProjectCalendar>> {
        let calendar = self.add_with_unique_id(unique_id);
        Self::initialize_default_base_calendar(&calendar);
        calendar
    }

/// Populate the default base calendar's name, working week, and hours.
    ///
/// Corresponds to Java: the three calls after `add()` in `addDefaultBaseCalendar()`.
    fn initialize_default_base_calendar(calendar: &Rc<RefCell<ProjectCalendar>>) {
        if let Some(mut calendar_ref) = borrow_mut_or_skip(calendar) {
            calendar_ref.set_name(Some(DEFAULT_BASE_CALENDAR_NAME.to_owned()));
            calendar_ref.add_default_calendar_days();
            calendar_ref.add_default_calendar_hours();
        }
    }

/// Create a default derived calendar: all 7 days' working-day flags are set to `DEFAULT`.
    ///
    /// Corresponds to Java: `public ProjectCalendar addDefaultDerivedCalendar()`
    pub fn add_default_derived_calendar(&mut self) -> Rc<RefCell<ProjectCalendar>> {
        let calendar = self.add();
        Self::initialize_default_derived_calendar(&calendar);
        calendar
    }

/// Create the default derived calendar with a preallocated ID when the context is already borrowed.
    ///
/// Corresponds to Java: `public ProjectCalendar addDefaultDerivedCalendar()`; a Rust-specific borrow-splitting entry point.
    pub(crate) fn add_default_derived_calendar_with_unique_id(
        &mut self, unique_id: Option<i32>,
    ) -> Rc<RefCell<ProjectCalendar>> {
        let calendar = self.add_with_unique_id(unique_id);
        Self::initialize_default_derived_calendar(&calendar);
        calendar
    }

/// Initialize the weekday types for the default derived calendar's week.
    ///
/// Corresponds to Java: the seven `setCalendarDayType` calls in `addDefaultDerivedCalendar()`.
    fn initialize_default_derived_calendar(calendar: &Rc<RefCell<ProjectCalendar>>) {
        if let Some(mut calendar_ref) = borrow_mut_or_skip(calendar) {
// Java sets them in DayOfWeek declaration order (Sunday -> Saturday)
            for day in [
                Weekday::Sun,
                Weekday::Mon,
                Weekday::Tue,
                Weekday::Wed,
                Weekday::Thu,
                Weekday::Fri,
                Weekday::Sat,
            ] {
                calendar_ref.set_calendar_day_type(day, DayType::Default);
            }
        }
    }

/// Find the calendar named "Standard"; if not found, degrade to "the first calendar" or a newly created default base calendar.
    ///
    /// Corresponds to Java: `public ProjectCalendar findOrCreateDefaultCalendar()`
    pub fn find_or_create_default_calendar(
        &mut self,
    ) -> Option<Rc<RefCell<ProjectCalendar>>> {
        self.find_or_create_default_calendar_with_unique_id(None)
    }

/// Use the caller's preallocated ID only when a default calendar needs to be created.
    ///
/// Corresponds to Java: `public ProjectCalendar findOrCreateDefaultCalendar()`; a Rust-specific borrow-splitting entry point.
    pub(crate) fn find_or_create_default_calendar_with_unique_id(
        &mut self, unique_id: Option<i32>,
    ) -> Option<Rc<RefCell<ProjectCalendar>>> {
        if let Some(existing) = self.get_by_name(Some(DEFAULT_BASE_CALENDAR_NAME)) {
            return Some(existing);
        }

        if !self.is_empty() {
            // Java: result = get(0);
            return self.get(0);
        }

        let calendar = if unique_id.is_some() {
            self.add_default_base_calendar_with_unique_id(unique_id)
        } else {
            self.add_default_base_calendar()
        };

        // Java: if (NumberHelper.getInt(result.getUniqueID()) == 0)
        //       result.setUniqueID(m_context.getUniqueIdObjectSequence(ProjectCalendar.class).getNext());
        if read_unique_id(&calendar).unwrap_or(0) == 0 {
            let next_unique_id = self.context.upgrade().and_then(|context| {
                match context.try_borrow_mut() {
                    Ok(mut context_ref) => Some(
                        context_ref
                            .get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>())
                            .get_next(),
                    ),
// This branch never occurs in Java (the context is always borrowable); the Rust side avoids a reentrant borrow panic.
                    Err(_) => None,
                }
            });

            if let Some(next_unique_id) = next_unique_id {
                if let Some(mut calendar_ref) = borrow_mut_or_skip(&calendar) {
                    calendar_ref.set_unique_id(Some(next_unique_id));
                }
            }
        }

        Some(calendar)
    }

/// Find a calendar by name (case-insensitive); returns `None` when the name is `None`/an empty string.
    ///
    /// Corresponds to Java: `public ProjectCalendar getByName(String calendarName)`
    ///
/// Java uses `calendarName.equalsIgnoreCase(calendar.getName())` (Unicode case folding;
/// returns `false` when the argument is `null`); the Rust side delegates to
/// [`crate::mpx::locale_utility::equals_ignore_case`] (same semantics).
    pub fn get_by_name(
        &self,
        calendar_name: Option<&str>,
    ) -> Option<Rc<RefCell<ProjectCalendar>>> {
        let calendar_name = match calendar_name {
            Some(name) if !name.is_empty() => name,
            _ => return None,
        };

        self.items
            .iter()
            .find(|calendar| {
                equals_ignore_case(calendar_name, read_name(calendar).as_deref())
            })
            .map(Rc::clone)
    }

/// Get the unique ID of the context's default calendar.
    ///
    /// Corresponds to Java: `public Integer getDefaultCalendarUniqueID()`
    pub fn get_default_calendar_unique_id(&self) -> Option<i32> {
        self.default_calendar_unique_id
    }

/// Set the unique ID of the context's default calendar.
    ///
    /// Corresponds to Java: `public void setDefaultCalendarUniqueID(Integer defaultCalendarUniqueID)`
    pub fn set_default_calendar_unique_id(&mut self, default_calendar_unique_id: Option<i32>) {
        self.default_calendar_unique_id = default_calendar_unique_id;
    }

/// Update state after a unique ID change.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.updateUniqueID(T element, Integer oldUniqueID, Integer newUniqueID)`
/// the final step `m_sequenceProvider.getUniqueIdObjectSequence(element.getClass()).sync(newUniqueID)`
    ///
/// When there is no element handle, only the sequence generator is synced; the calendar setter uses
/// [`ProjectCalendarContainer::update_unique_id_for_calendar`] to update the base class index.
    pub fn update_unique_id(&mut self, old_unique_id: Option<i32>, new_unique_id: Option<i32>) {
        let _ = old_unique_id;
        let Some(new_unique_id) = new_unique_id else {
            return;
        };

        // Java: m_sequenceProvider.getUniqueIdObjectSequence(element.getClass()).sync(newUniqueID);
        if let Some(context) = self.context.upgrade() {
            if let Ok(mut context_ref) = context.try_borrow_mut() {
                context_ref
                    .get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>())
                    .sync(Some(new_unique_id));
            }
        }
    }

/// Get the sequence provider held by the base class.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.getSequenceProvider()`.
    pub fn get_sequence_provider(&self) -> Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> {
        self.items.get_sequence_provider()
    }

/// Get the initial value of the unique ID sequence.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.firstUniqueID()`.
    pub fn first_unique_id(&self) -> i32 {
        self.items.first_unique_id()
    }

/// Update the calendar unique ID index and handle duplicate-ID candidates.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.updateUniqueID(this, oldUniqueID, newUniqueID)`;
/// A calendar not registered in the container only syncs the sequence generator.
    pub(crate) fn update_unique_id_for_calendar(
        &mut self,
        calendar: Option<Rc<RefCell<ProjectCalendar>>>,
        old_unique_id: Option<i32>,
        new_unique_id: Option<i32>,
    ) {
        if let Some(calendar) = calendar {
            self.items.update_unique_id(&calendar, old_unique_id, new_unique_id);
        } else {
            self.update_unique_id(old_unique_id, new_unique_id);
        }
    }

/// Reassign duplicate calendar unique IDs following the Java base class behavior.
    ///
/// Corresponds to Java: `ProjectEntityContainer.fixUniqueIdClashes()`; the caller obtains the sequence handle
/// before borrowing the context, to avoid a reverse borrow conflict with the sequence provider.
    pub fn fix_unique_id_clashes(&mut self, sequence: Rc<ObjectSequence>) {
        self.items.rebuild_unique_id_index();
        for calendar in self.items.iter() {
            sequence.sync(read_unique_id(calendar));
        }
        self.items.fix_unique_id_clashes_with_sequence(sequence);
    }

/// Remove a calendar from the container and perform post-removal cleanup.
    ///
/// Corresponds to Java: `ListWithCallbacks.remove(Object o)` (which internally calls `removed(T element)`)
    ///
/// Java locates the element by reference equality (default `equals`); the Rust side uses `RefCell::as_ptr` for a pointer identity comparison.
    pub fn remove(&mut self, calendar: &ProjectCalendar) -> bool {
        let target = calendar as *const ProjectCalendar;
        let index = self
            .items
            .iter()
            .position(|candidate| candidate.as_ptr() as *const ProjectCalendar == target);

        let Some(index) = index else {
            return false;
        };

        let Some(removed) = self.items.remove(index) else {
            return false;
        };
        self.removed(&removed);
        true
    }

/// Cleanup after a calendar is removed: clear related entities' back-references to this calendar.
    ///
    /// Corresponds to Java: `@Override public void removed(ProjectCalendar calendar)`
    fn removed(&mut self, calendar: &Rc<RefCell<ProjectCalendar>>) {
// Java: super.removed(calendar); -- the base class already maintains the index in items.remove.

        // Java: calendar.getDerivedCalendars().forEach(c -> c.setParent(null));
// Java finds this calendar's derived calendars through `m_context`; the Rust side instead directly scans this container's elements
// (semantically equivalent -- the container's element set is exactly the calendar set in the context), so the context need not be borrowed again:
// every call site of this method holds a mutable borrow of the context, so borrowing the context would fail.
        for candidate in self.items.iter() {
            let is_derived = match candidate.try_borrow() {
                Ok(candidate_ref) => candidate_ref
                    .get_parent()
                    .is_some_and(|parent| Rc::ptr_eq(&parent, calendar)),
                Err(_) => false,
            };

            if is_derived {
                if let Ok(mut candidate_ref) = candidate.try_borrow_mut() {
                    candidate_ref.set_parent(None);
                }
            }
        }

        // Java:
        // calendar.getResources().forEach(r -> r.setCalendar(null));
        // calendar.getTasks().forEach(t -> t.setCalendar(null));
// These two steps need to query resources / tasks through the context, but the container can only be obtained via `ProjectContext::get_calendars_mut()`,
// and when this method runs the context is necessarily mutably borrowed, so the `try_borrow` inside the query necessarily fails;
// therefore the Rust side cannot restore it (Java's `m_context` is a strong reference with no such restriction).

        // Java: calendar.setParent(null);
// The caller (`ProjectCalendar::remove` and the removal path in `ProjectFile`) usually holds a borrow of that calendar,
// so it cannot be mutably borrowed again, and it degrades to skipping, consistent with the file header.
        if let Ok(mut calendar_ref) = calendar.try_borrow_mut() {
            calendar_ref.set_parent(None);
        }
    }

/// Get a calendar by index; returns `None` when out of bounds.
    ///
    /// Corresponds to Java: `ArrayList.get(int index)`.
    pub fn get(&self, index: usize) -> Option<Rc<RefCell<ProjectCalendar>>> {
        self.items.get(index)
    }

/// Number of calendars.
    ///
/// Corresponds to Java: `size()` inherited from `ArrayList` by `ProjectEntityContainer`.
    pub fn len(&self) -> usize {
        self.items.len()
    }

/// Whether the container is empty.
    ///
/// Corresponds to Java: `isEmpty()` inherited from `ArrayList` by `ProjectEntityContainer`.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

/// Iterate over the calendars in the container.
    ///
/// Corresponds to Java: `iterator()` inherited from `ArrayList` by `ProjectEntityContainer`.
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<ProjectCalendar>>> {
        self.items.iter()
    }

/// Access the calendars in the container as a slice.
    ///
/// Corresponds to Java: the list view of `ProjectEntityContainer`
/// ([`crate::project_calendar::ProjectCalendar::get_derived_calendars`] uses this method).
    pub fn as_slice(&self) -> &[Rc<RefCell<ProjectCalendar>>] {
        self.items.as_slice()
    }

/// Find a calendar by unique ID; returns `None` when not found.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<ProjectCalendar>>> {
        let id = id?;
        if let Some(indexed) = self.items.get_by_unique_id(Some(id)) {
            if read_unique_id(&indexed) == Some(id) {
                return Some(indexed);
            }
        }
// When the caller holds a mutable borrow of the context, the calendar setter cannot sync the base class index.
        self.items
            .iter()
            .find(|calendar| read_unique_id(calendar) == Some(id))
            .map(Rc::clone)
    }
}

impl Default for ProjectCalendarContainer {
/// Corresponds to Java: the explicitly declared `ProjectCalendarContainer(ProjectContext)`,
/// which is equivalent on the Rust side to [`ProjectCalendarContainer::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for ProjectCalendarContainer {
/// Java does not override `toString()`; the Rust side outputs the container size, the default calendar unique ID, and the context wiring status
/// ([`crate::project_calendar::ProjectCalendar`] does not implement `Debug`, so element contents are not output).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProjectCalendarContainer")
            .field("size", &self.items.len())
            .field("default_calendar_unique_id", &self.default_calendar_unique_id)
            .field("context_is_set", &(self.context.strong_count() > 0))
            .finish()
    }
}

/// Obtain a mutable reference to a calendar via `try_borrow_mut`; returns `None` when already borrowed (does not panic).
fn borrow_mut_or_skip(
    calendar: &Rc<RefCell<ProjectCalendar>>,
) -> Option<std::cell::RefMut<'_, ProjectCalendar>> {
    calendar.try_borrow_mut().ok()
}

/// Read a calendar name; returns `None` when already borrowed.
fn read_name(calendar: &Rc<RefCell<ProjectCalendar>>) -> Option<String> {
    calendar
        .try_borrow()
        .ok()
        .and_then(|calendar| calendar.get_name().map(str::to_owned))
}

/// Read a calendar unique ID; returns `None` when already borrowed.
fn read_unique_id(calendar: &Rc<RefCell<ProjectCalendar>>) -> Option<i32> {
    calendar
        .try_borrow()
        .ok()
        .and_then(|calendar| calendar.get_unique_id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_context::ProjectContext;

/// Get a mutable reference to an element in the container, for reading calendar state in assertions.
    fn read(calendar: &Rc<RefCell<ProjectCalendar>>) -> std::cell::Ref<'_, ProjectCalendar> {
        calendar.borrow()
    }

    #[test]
    fn container_basics() {
        let mut container = ProjectCalendarContainer::new();
        assert!(container.is_empty());

        container.add_calendar(Rc::new(RefCell::new(ProjectCalendar::new(Weak::new(), false))));
        container.add_calendar(Rc::new(RefCell::new(ProjectCalendar::new(Weak::new(), false))));

        assert_eq!(container.len(), 2);
        assert!(!container.is_empty());
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert!(container.get(0).is_some());
        assert!(container.get(2).is_none());
    }

    #[test]
    fn add_creates_and_registers_calendar() {
        let mut container = ProjectCalendarContainer::new();
        let calendar = container.add();

        assert_eq!(container.len(), 1);
        assert!(Rc::ptr_eq(&container.get(0).expect("added to container"), &calendar));
    }

    #[test]
    fn add_default_base_calendar_sets_name_and_default_week() {
        let mut container = ProjectCalendarContainer::new();
        let calendar = container.add_default_base_calendar();

        {
            let calendar = read(&calendar);
            assert_eq!(calendar.get_name(), Some(DEFAULT_BASE_CALENDAR_NAME));
// Java: addDefaultCalendarDays() -> Monday to Friday working, Saturday and Sunday non-working
            assert_eq!(
                calendar.get_calendar_day_type(Weekday::Mon),
                Some(DayType::Working)
            );
            assert_eq!(
                calendar.get_calendar_day_type(Weekday::Sun),
                Some(DayType::NonWorking)
            );
        }

// Java: addDefaultCalendarHours() -> Monday has working hours
        assert!(read(&calendar)
            .get_calendar_hours_for_day(Weekday::Mon)
            .is_some());
    }

    #[test]
    fn add_default_derived_calendar_sets_all_days_to_default() {
        let mut container = ProjectCalendarContainer::new();
        let calendar = container.add_default_derived_calendar();
        let calendar = read(&calendar);

        for day in [
            Weekday::Sun,
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
            Weekday::Sat,
        ] {
            assert_eq!(calendar.get_calendar_day_type(day), Some(DayType::Default));
        }
    }

    #[test]
    fn get_by_name_ignores_ascii_case_and_empty_names() {
        let mut container = ProjectCalendarContainer::new();
        let calendar = container.add();
        calendar.borrow_mut().set_name(Some("Standard".to_owned()));

        assert!(container
            .get_by_name(Some("standard"))
            .map(|found| Rc::ptr_eq(&found, &calendar))
            .unwrap_or(false));
        assert!(container.get_by_name(Some("Other")).is_none());
        assert!(container.get_by_name(Some("")).is_none());
        assert!(container.get_by_name(None).is_none());
    }

    #[test]
    fn default_calendar_unique_id_round_trips() {
        let mut container = ProjectCalendarContainer::new();
        assert_eq!(container.get_default_calendar_unique_id(), None);

        container.set_default_calendar_unique_id(Some(7));
        assert_eq!(container.get_default_calendar_unique_id(), Some(7));

        container.set_default_calendar_unique_id(None);
        assert_eq!(container.get_default_calendar_unique_id(), None);
    }

    #[test]
    fn remove_drops_calendar_from_container() {
        let mut container = ProjectCalendarContainer::new();
        let calendar = container.add();
        assert_eq!(container.len(), 1);

// Java: remove(Object) locates by reference equality; the Rust side passes in a &ProjectCalendar pointing to that calendar
        let removed = {
            let calendar_ref = read(&calendar);
            container.remove(&calendar_ref)
        };
        assert!(removed);
        assert!(container.is_empty());

// removing a calendar that is no longer in the container returns false
        let removed_again = {
            let calendar_ref = read(&calendar);
            container.remove(&calendar_ref)
        };
        assert!(!removed_again);
    }

/// After a calendar is removed, its derived calendars no longer point back to the removed parent calendar (Java's
    /// `calendar.getDerivedCalendars().forEach(c -> c.setParent(null))`).
    #[test]
    fn remove_clears_parent_of_derived_calendars() {
        let mut container = ProjectCalendarContainer::new();
        let parent = container.add();
        let child = container.add();

        child.borrow_mut().set_parent(Some(Rc::clone(&parent)));
        assert!(child.borrow().get_parent().is_some());

        let removed = {
            let calendar_ref = read(&parent);
            container.remove(&calendar_ref)
        };

        assert!(removed);
        assert!(child.borrow().get_parent().is_none());
    }

    #[test]
    fn remove_unknown_calendar_returns_false() {
        let mut container = ProjectCalendarContainer::new();
        let standalone = ProjectCalendar::new(Weak::new(), false);

        assert!(!container.remove(&standalone));
    }

    #[test]
    fn find_or_create_default_calendar_reuses_or_creates() {
// empty container: create a new "Standard" default base calendar
        let mut container = ProjectCalendarContainer::new();
        let created = container
            .find_or_create_default_calendar()
            .expect("empty container creates default calendar");
        assert_eq!(read(&created).get_name(), Some(DEFAULT_BASE_CALENDAR_NAME));
        assert_eq!(container.len(), 1);

// "Standard" already exists: reuse it directly
        let reused = container
            .find_or_create_default_calendar()
            .expect("reuse existing Standard");
        assert!(Rc::ptr_eq(&created, &reused));
        assert_eq!(container.len(), 1);

// no "Standard" but other calendars exist: take the first one
        let mut container = ProjectCalendarContainer::new();
        let first = container.add();
        first.borrow_mut().set_name(Some("Custom".to_owned()));
        let found = container
            .find_or_create_default_calendar()
            .expect("get first calendar");
        assert!(Rc::ptr_eq(&first, &found));
        assert_eq!(container.len(), 1);
    }

    #[test]
    fn find_or_create_default_calendar_allocates_unique_id_from_context() {
        let context = ProjectContext::shared();
        let mut container = ProjectCalendarContainer::new();
        container.set_context(&context);

        let calendar = container
            .find_or_create_default_calendar()
            .expect("create default calendar");

// the sequence in the context should already follow the allocated unique ID
        let next = context
            .borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>())
            .get_next();
        assert!(
            next > read(&calendar).get_unique_id().unwrap_or(0),
            "sequence should have followed the assigned unique ID"
        );
    }

    #[test]
    fn update_unique_id_syncs_context_sequence() {
        let context = ProjectContext::shared();
        let mut container = ProjectCalendarContainer::new();
        container.set_context(&context);

        container.update_unique_id(Some(1), Some(20));

        let next = context
            .borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>())
            .get_next();
        assert_eq!(next, 21);
    }

    #[test]
    fn get_by_unique_id_finds_calendar() {
        let mut container = ProjectCalendarContainer::new();
        let calendar = Rc::new(RefCell::new(ProjectCalendar::new(Weak::new(), false)));
        calendar.borrow_mut().set_unique_id(Some(3));
        container.add_calendar(Rc::clone(&calendar));

        assert!(container
            .get_by_unique_id(Some(3))
            .map(|found| Rc::ptr_eq(&found, &calendar))
            .unwrap_or(false));
        assert!(container.get_by_unique_id(Some(4)).is_none());
        assert!(container.get_by_unique_id(None).is_none());
    }

/// When fixing two calendars with the same ID via the base class index, only the later-added element is rearranged.
    #[test]
    fn fix_unique_id_clashes_rebuilds_index_and_renumbers_duplicates() {
        let mut container = ProjectCalendarContainer::new();
        let first = container.add();
        let second = container.add();
        first.borrow_mut().set_unique_id(Some(7));
        second.borrow_mut().set_unique_id(Some(7));
        let sequence = container.items.get_sequence_provider().borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>());
        container.fix_unique_id_clashes(sequence);

        assert_eq!(first.borrow().get_unique_id(), Some(7));
        assert_eq!(second.borrow().get_unique_id(), Some(8));
        assert!(container.get_by_unique_id(Some(7)).is_some_and(|found| Rc::ptr_eq(&found, &first)));
        assert!(container.get_by_unique_id(Some(8)).is_some_and(|found| Rc::ptr_eq(&found, &second)));
    }

/// Adding to the base class container does not trigger a RefCell panic when the element is already borrowed.
    #[test]
    fn add_calendar_with_borrowed_element_degrades_to_linear_lookup() {
        let mut container = ProjectCalendarContainer::new();
        let calendar = Rc::new(RefCell::new(ProjectCalendar::new(Weak::new(), false)));
        let mut borrowed = calendar.borrow_mut();
        borrowed.set_unique_id(Some(12));
        container.add_calendar(Rc::clone(&calendar));
        drop(borrowed);
        assert!(container.get_by_unique_id(Some(12))
            .is_some_and(|found| Rc::ptr_eq(&found, &calendar)));
    }

    #[test]
    fn default_matches_new() {
        let container = ProjectCalendarContainer::default();
        assert!(container.is_empty());
    }

    #[test]
    fn debug_reports_size_and_context_state() {
        let mut container = ProjectCalendarContainer::new();
        assert!(format!("{container:?}").contains("context_is_set: false"));

        container.add();
        let text = format!("{container:?}");
        assert!(text.contains("size: 1"));
    }
}
