//! Corresponds to Java: `org/mpxj/ResourceAssignmentContainer.java`
//!
//! Manages the collection of resource assignments in a project.
//!
//! # Differences from Java
//!
//! | Java | Rust |
//! |---|---|
//! | `extends ProjectEntityContainer<ResourceAssignment>` | Does not compose [`crate::project_entity_container::ProjectEntityContainer`]; instead uses an internal `Vec<Rc<RefCell<ResourceAssignment>>>` + linear lookup (see below) |
//! | `ResourceAssignmentContainer(ProjectFile projectFile)` | [`ResourceAssignmentContainer::new`] + [`ResourceAssignmentContainer::set_context`] (the parent file is held as a weak reference by [`crate::resource_assignment::ResourceAssignment`] itself) |
//! | `@Override protected void added(ResourceAssignment assignment)` | Private method `added` (called by [`ResourceAssignmentContainer::add`]) |
//! | `@Override public void removed(ResourceAssignment assignment)` | Private method `removed` (called by `remove` / `remove_by_unique_id`) |
//! | `public Set<FieldType> getPopulatedFields()` | [`ResourceAssignmentContainer::get_populated_fields`] (the parent file passes in the defaults and custom fields) |
//! | `public List<CustomField> getCustomFields()` | [`ResourceAssignmentContainer::get_custom_fields`] (via the context's custom field container) |
//! | `public FieldType getFieldTypeByAlias(String alias)` | [`ResourceAssignmentContainer::get_field_type_by_alias`] (returns `Option<FieldKey>`) |
//! | `public List<ResourceAssignment> getByTaskUniqueID(Integer)` | [`ResourceAssignmentContainer::get_by_task_unique_id`] |
//! | `public List<ResourceAssignment> getByResourceUniqueID(Integer)` | [`ResourceAssignmentContainer::get_by_resource_unique_id`] |
//! | Inherited from `ProjectEntityContainer`: `getByUniqueID` / `add` / `size` / `isEmpty` / `iterator` / `get` | [`ResourceAssignmentContainer::get_by_unique_id`] / [`ResourceAssignmentContainer::add`] / [`ResourceAssignmentContainer::len`] / [`ResourceAssignmentContainer::is_empty`] / [`ResourceAssignmentContainer::iter`] / [`ResourceAssignmentContainer::get`] |
//! | Inherited from `ProjectEntityContainer`: `updateUniqueID(T element, Integer old, Integer new)` | [`ResourceAssignmentContainer::update_unique_id`] (no element argument in Rust, see below) |
//! | Inherited from `ProjectEntityContainer`: `fixUniqueIdClashes` / `firstUniqueID` / `getSequenceProvider` | Not ported: see "Base class not composed" below |
//!
//! ## `ProjectEntityContainer` not composed
//!
//! Java's unique ID index (`m_uniqueIDMap`) and clash list come from the base class. On the Rust side this container instead uses
//! **linear lookup by current unique ID**: `ResourceAssignment::set_unique_id` updates its own field and then
//! calls back into [`ResourceAssignmentContainer::update_unique_id`] to synchronize the sequence generator, and linear lookup
//! by nature does not become stale when the unique ID changes, so there is no need to maintain an index and clash list, and the
//! `fixUniqueIdClashes` family of methods is skipped as well (consistent with
//! the handling in [`crate::project_calendar_container::ProjectCalendarContainer`]).
//!
//! ## Other differences
//!
//! * **Element shape**: Java stores a `ResourceAssignment` reference; the Rust side stores
//! `Rc<RefCell<ResourceAssignment>>` (consistent with the "shared entities use `Rc<RefCell<T>>`" convention).
//! * **The element argument of `remove`**: Java's is `ListWithCallbacks.remove(Object)` (reference equality).
//! The Rust side accepts `&Rc<RefCell<ResourceAssignment>>` and uses [`Rc::ptr_eq`] for pointer identity comparison;
//! it also provides [`ResourceAssignmentContainer::remove_by_unique_id`] (Rust-specific) for
//! [`crate::resource_assignment::ResourceAssignment::remove`] to use when it cannot obtain its own `Rc`.
//! * **Context back-reference**: the Java field `m_projectFile` is a strong reference; the Rust side changes it to
//! `Weak<RefCell<ProjectContext>>` to avoid `ProjectContext ↔ container` forming an `Rc` cycle.
//! * **Re-entrant borrowing**: internally the container always uses `try_borrow_mut` on the context; on failure it degrades to "not synchronizing the sequence
//! generator" and does not panic (Java has no such case).
//! * In Java, `getCustomFields` / `getFieldTypeByAlias` are accessed via `m_projectFile`; the Rust side
//! accesses them via the context (in Java `ProjectFile.getCustomFields()` itself delegates to the context).

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::{Rc, Weak};

use crate::custom_field::CustomField;
use crate::field_key::FieldKey;
use crate::field_type_class::FieldTypeClass;
use crate::project_context::ProjectContext;
use crate::resource_assignment::ResourceAssignment;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;

/// The collection of resource assignments in a project.
///
/// Corresponds to Java: `public class ResourceAssignmentContainer extends ProjectEntityContainer<ResourceAssignment>`
pub struct ResourceAssignmentContainer {
/// The resource assignments in the container.
    ///
/// Corresponds to Java: the internal list storage of `ProjectEntityContainer` / `ListWithCallbacks`.
    items: Vec<Rc<RefCell<ResourceAssignment>>>,

/// Index built by resource unique ID.
    ///
    /// Corresponds to Java field `private final HashMap<Integer, List<ResourceAssignment>> m_resourceIndex = new HashMap<>();`
    resource_index: HashMap<i32, Vec<Rc<RefCell<ResourceAssignment>>>>,

/// Index built by task unique ID.
    ///
    /// Corresponds to Java field `private final HashMap<Integer, List<ResourceAssignment>> m_taskIndex = new HashMap<>();`
    task_index: HashMap<i32, Vec<Rc<RefCell<ResourceAssignment>>>>,

/// Owning context (weak reference to avoid an `Rc` cycle).
    ///
/// Corresponds to Java field `private final ProjectFile m_projectFile;` (the Rust side holds the context instead,
/// see the file header).
    context: Weak<RefCell<ProjectContext>>,
}

impl ResourceAssignmentContainer {
/// Returns the fields for which at least one resource assignment has a non-default cached value.
    ///
/// Corresponds to Java: `public Set<FieldType> getPopulatedFields()`; project defaults and custom
/// fields are passed in by the parent file caller, to avoid borrowing the parent file again while it is already borrowed.
    pub fn get_populated_fields(
        &self,
        custom: impl IntoIterator<Item = FieldKey>,
        defaults: crate::common::populated_fields::Defaults,
    ) -> std::collections::HashSet<FieldKey> {
        crate::common::populated_fields::get_populated_fields(
            crate::common::populated_fields::merge_fields(
                crate::assignment_field::AssignmentField::ALL.map(FieldKey::Assignment), custom,
            ),
            self.iter().cloned(),
            defaults,
        )
    }

/// Constructor.
    ///
    /// Corresponds to Java: `public ResourceAssignmentContainer(ProjectFile projectFile)`
    ///
/// The Rust side first constructs with an empty context weak reference, then [`ResourceAssignmentContainer::set_context`] wires it up.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            resource_index: HashMap::new(),
            task_index: HashMap::new(),
            context: Weak::new(),
        }
    }

/// Wires up the owning context (Rust-specific: Java receives the parent file directly in the constructor).
    ///
/// Corresponds to Java: `m_projectFile = projectFile;` in the constructor
    pub fn set_context(&mut self, context: &Rc<RefCell<ProjectContext>>) {
        self.context = Rc::downgrade(context);
    }

/// Returns the owning context.
    ///
/// Rust-specific accessor (Java has no corresponding method; the field is the parent file).
    pub fn get_context(&self) -> Option<Rc<RefCell<ProjectContext>>> {
        self.context.upgrade()
    }

/// Adds a resource assignment to the container.
    ///
/// Corresponds to Java: `ListWithCallbacks.add(E element)` (which in turn triggers
/// `ProjectEntityContainer.added` and `ResourceAssignmentContainer.added`)
    pub fn add(&mut self, assignment: Rc<RefCell<ResourceAssignment>>) {
        self.added(&assignment);
        self.items.push(assignment);
    }

/// Hook called after an assignment is added: updates the task/resource indexes.
    ///
    /// Corresponds to Java: `@Override protected void added(ResourceAssignment assignment)`
    fn added(&mut self, assignment: &Rc<RefCell<ResourceAssignment>>) {
// Java: super.added(assignment); -- the base class maintains the unique ID index,
// the Rust side uses linear lookup instead, so there is no index to maintain.

        let (task_unique_id, resource_unique_id) = match assignment.try_borrow() {
            Ok(assignment) => (
                assignment.get_task_unique_id(),
                assignment.get_resource_unique_id(),
            ),
            Err(_) => return,
        };

        if let Some(resource_unique_id) = resource_unique_id {
            self.resource_index
                .entry(resource_unique_id)
                .or_default()
                .push(Rc::clone(assignment));
        }

        if let Some(task_unique_id) = task_unique_id {
            self.task_index
                .entry(task_unique_id)
                .or_default()
                .push(Rc::clone(assignment));
        }
    }

/// Removes a resource assignment by `Rc` pointer identity.
    ///
/// Corresponds to Java: `ListWithCallbacks.remove(Object o)` (internally calls `removed(T element)`)
    ///
/// Java locates the element by reference equality; the Rust side expresses the same semantics with [`Rc::ptr_eq`].
    pub fn remove(&mut self, assignment: &Rc<RefCell<ResourceAssignment>>) -> bool {
        let index = self
            .items
            .iter()
            .position(|candidate| Rc::ptr_eq(candidate, assignment));

        let Some(index) = index else {
            return false;
        };

        let removed = self.items.remove(index);
        self.removed(&removed);
        true
    }

/// Removes a resource assignment by unique ID (Rust-specific, see the file header).
    ///
/// Corresponds to Java: a degraded implementation of `ListWithCallbacks.remove(Object o)`
/// (used by [`crate::resource_assignment::ResourceAssignment::remove`];
/// when the unique ID is `None` it does nothing and returns `false`).
    pub fn remove_by_unique_id(&mut self, unique_id: Option<i32>) -> bool {
        let unique_id = match unique_id {
            Some(unique_id) => unique_id,
            None => return false,
        };

        let index = self
            .items
            .iter()
            .position(|candidate| read_unique_id(candidate) == Some(unique_id));

        let Some(index) = index else {
            return false;
        };

        let removed = self.items.remove(index);
        self.removed(&removed);
        true
    }

/// Deletes all resource assignments under a given resource (Rust-specific: used by
/// [`crate::resource_container::ResourceContainer`]'s `removeAssignments` call).
    ///
    /// Corresponds to Java: `ResourceContainer.removeAssignments(ProjectFile project, Resource resource)`
/// the iterative deletion of resource assignments. Returns the number of deleted entries.
    pub fn remove_by_resource_unique_id(&mut self, resource_unique_id: Option<i32>) -> usize {
        let Some(resource_unique_id) = resource_unique_id else {
            return 0;
        };

        let targets: Vec<Rc<RefCell<ResourceAssignment>>> = self
            .items
            .iter()
            .filter(|assignment| {
                assignment
                    .try_borrow()
                    .ok()
                    .and_then(|assignment| assignment.get_resource_unique_id())
                    == Some(resource_unique_id)
            })
            .cloned()
            .collect();

        let mut removed = 0;
        for target in targets {
            if self.remove(&target) {
                removed += 1;
            }
        }

        removed
    }

/// Hook called after an assignment is removed: removes it from the task/resource indexes.
    ///
    /// Corresponds to Java: `@Override public void removed(ResourceAssignment assignment)`
    fn removed(&mut self, assignment: &Rc<RefCell<ResourceAssignment>>) {
// Java: super.removed(assignment); -- the base class maintains the unique ID index,
// the Rust side uses linear lookup instead, so there is no index to maintain.

        let (task_unique_id, resource_unique_id) = match assignment.try_borrow() {
            Ok(assignment) => (
                assignment.get_task_unique_id(),
                assignment.get_resource_unique_id(),
            ),
            Err(_) => return,
        };

        if let Some(resource_unique_id) = resource_unique_id {
            remove_from_index(&mut self.resource_index, resource_unique_id, assignment);
        }

        if let Some(task_unique_id) = task_unique_id {
            remove_from_index(&mut self.task_index, task_unique_id, assignment);
        }
    }

/// Updates the state after a unique ID change.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.updateUniqueID(T element, Integer oldUniqueID, Integer newUniqueID)`
/// the final step `m_sequenceProvider.getUniqueIdObjectSequence(element.getClass()).sync(newUniqueID)`
    ///
/// The Rust container looks up linearly by current unique ID (no index or clash list, see the file header), so it only needs to
/// synchronize the sequence generator to the new unique ID. See
    /// [`crate::resource_assignment::ResourceAssignment::set_unique_id`].
    pub fn update_unique_id(&mut self, old_unique_id: Option<i32>, new_unique_id: Option<i32>) {
        let _ = old_unique_id;
        let Some(new_unique_id) = new_unique_id else {
            return;
        };

        // Java: m_sequenceProvider.getUniqueIdObjectSequence(element.getClass()).sync(newUniqueID);
        if let Some(context) = self.context.upgrade() {
            if let Ok(mut context) = context.try_borrow_mut() {
                context
                    .get_unique_id_object_sequence(TypeId::of::<ResourceAssignment>())
                    .sync(Some(new_unique_id));
            }
        }
    }

/// Returns the custom fields of resource assignments.
    ///
    /// Corresponds to Java: `public List<CustomField> getCustomFields()`
    pub fn get_custom_fields(&self) -> Vec<Rc<RefCell<CustomField>>> {
        let Some(context) = self.context.upgrade() else {
            return Vec::new();
        };

        let custom_fields = match context.try_borrow() {
            Ok(context) => context.get_custom_fields(),
            Err(_) => return Vec::new(),
        };

        match custom_fields.try_borrow() {
            Ok(custom_fields) => {
                custom_fields.get_custom_fields_by_field_type_class(FieldTypeClass::Assignment)
            }
            Err(_) => Vec::new(),
        }
    }

/// Returns the field key by alias.
    ///
    /// Corresponds to Java: `public FieldType getFieldTypeByAlias(String alias)`
    ///
/// The Rust side returns `Option<FieldKey>` (`None` if not found).
    pub fn get_field_type_by_alias(&self, alias: &str) -> Option<FieldKey> {
        let context = self.context.upgrade()?;
        let custom_fields = match context.try_borrow() {
            Ok(context) => context.get_custom_fields(),
            Err(_) => return None,
        };

        match custom_fields.try_borrow() {
            Ok(custom_fields) => {
                custom_fields.get_field_type_by_alias(FieldTypeClass::Assignment, alias)
            }
            Err(_) => None,
        }
    }

/// Returns the resource assignments for a task unique ID; returns an empty list when there is no match.
    ///
    /// Corresponds to Java: `public List<ResourceAssignment> getByTaskUniqueID(Integer taskUniqueID)`
    pub fn get_by_task_unique_id(&self, task_unique_id: Option<i32>) -> Vec<Rc<RefCell<ResourceAssignment>>> {
        match task_unique_id.and_then(|id| self.task_index.get(&id)) {
            Some(assignments) => assignments.clone(),
            None => Vec::new(),
        }
    }

/// Returns the resource assignments for a resource unique ID; returns an empty list when there is no match.
    ///
    /// Corresponds to Java: `public List<ResourceAssignment> getByResourceUniqueID(Integer resourceUniqueID)`
    pub fn get_by_resource_unique_id(
        &self,
        resource_unique_id: Option<i32>,
    ) -> Vec<Rc<RefCell<ResourceAssignment>>> {
        match resource_unique_id.and_then(|id| self.resource_index.get(&id)) {
            Some(assignments) => assignments.clone(),
            None => Vec::new(),
        }
    }

/// Looks up a resource assignment by unique ID, returning `None` if not found.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<ResourceAssignment>>> {
        let id = id?;
        self.items
            .iter()
            .find(|assignment| read_unique_id(assignment) == Some(id))
            .map(Rc::clone)
    }

/// Returns the resource assignment at the given index; returns `None` if out of bounds.
    ///
    /// Corresponds to Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<Rc<RefCell<ResourceAssignment>>> {
        self.items.get(index).map(Rc::clone)
    }

/// Number of resource assignments.
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

/// Iterates over the resource assignments in the container.
    ///
    /// Corresponds to Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<ResourceAssignment>>> {
        self.items.iter()
    }

/// Accesses the resource assignments in the container as a slice.
    ///
/// Corresponds to Java: the list view of `ListWithCallbacks`.
    pub fn as_slice(&self) -> &[Rc<RefCell<ResourceAssignment>>] {
        self.items.as_slice()
    }
}

impl Default for ResourceAssignmentContainer {
/// Corresponds to Java: the explicitly declared `ResourceAssignmentContainer(ProjectFile)`,
/// which on the Rust side is equivalent to [`ResourceAssignmentContainer::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for ResourceAssignmentContainer {
/// Java does not override `toString()`; the Rust side outputs the container size, index size and context wiring state
/// ([`crate::resource_assignment::ResourceAssignment`] does not implement `Debug`, so element contents are not output).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceAssignmentContainer")
            .field("size", &self.items.len())
            .field("task_index_size", &self.task_index.len())
            .field("resource_index_size", &self.resource_index.len())
            .field("context_is_set", &(self.context.strong_count() > 0))
            .finish()
    }
}

/// Removes an entry from the index by `Rc` pointer identity.
fn remove_from_index(
    index: &mut HashMap<i32, Vec<Rc<RefCell<ResourceAssignment>>>>,
    key: i32,
    assignment: &Rc<RefCell<ResourceAssignment>>,
) {
    if let Some(assignments) = index.get_mut(&key) {
        assignments.retain(|candidate| !Rc::ptr_eq(candidate, assignment));
    }
}

/// Reads the unique ID of a resource assignment, returning `None` if already borrowed.
fn read_unique_id(assignment: &Rc<RefCell<ResourceAssignment>>) -> Option<i32> {
    assignment
        .try_borrow()
        .ok()
        .and_then(|assignment| assignment.get_unique_id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duration::Duration;
    use crate::time_unit::TimeUnit;

/// Constructs a resource assignment with the context wired up and the unique ID already set.
    fn assignment(
        container: &mut ResourceAssignmentContainer,
        unique_id: i32,
        task_unique_id: i32,
        resource_unique_id: i32,
    ) -> Rc<RefCell<ResourceAssignment>> {
        let mut item = ResourceAssignment::new();
        item.set_unique_id(Some(unique_id));
        item.set_task_unique_id(Some(task_unique_id));
        item.set_resource_unique_id(Some(resource_unique_id));

        let item = Rc::new(RefCell::new(item));
        container.add(Rc::clone(&item));
        item
    }

    #[test]
    fn container_basics() {
        let mut container = ResourceAssignmentContainer::new();
        assert!(container.is_empty());

        assignment(&mut container, 1, 10, 20);
        assignment(&mut container, 2, 10, 21);

        assert_eq!(container.len(), 2);
        assert!(!container.is_empty());
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert!(container.get(0).is_some());
        assert!(container.get(2).is_none());
    }

    #[test]
    fn lookup_by_unique_id_and_indices() {
        let mut container = ResourceAssignmentContainer::new();
        let first = assignment(&mut container, 1, 10, 20);
        let second = assignment(&mut container, 2, 10, 21);

        assert!(container
            .get_by_unique_id(Some(1))
            .map(|found| Rc::ptr_eq(&found, &first))
            .unwrap_or(false));
        assert!(container.get_by_unique_id(Some(3)).is_none());
        assert!(container.get_by_unique_id(None).is_none());

// Two entries retrieved by task unique ID
        let by_task = container.get_by_task_unique_id(Some(10));
        assert_eq!(by_task.len(), 2);

// One entry retrieved by each resource unique ID
        let by_resource = container.get_by_resource_unique_id(Some(20));
        assert_eq!(by_resource.len(), 1);
        assert!(Rc::ptr_eq(&by_resource[0], &first));
        assert_eq!(container.get_by_resource_unique_id(Some(21)).len(), 1);
        assert!(container.get_by_resource_unique_id(Some(99)).is_empty());
        assert!(container.get_by_task_unique_id(None).is_empty());

// second is not hit by the resource index
        assert!(container
            .get_by_resource_unique_id(Some(21))
            .first()
            .map(|found| Rc::ptr_eq(found, &second))
            .unwrap_or(false));
    }

    #[test]
    fn remove_updates_indices() {
        let mut container = ResourceAssignmentContainer::new();
        let first = assignment(&mut container, 1, 10, 20);
        let second = assignment(&mut container, 2, 10, 20);

        assert_eq!(container.get_by_task_unique_id(Some(10)).len(), 2);
        assert_eq!(container.get_by_resource_unique_id(Some(20)).len(), 2);

        assert!(container.remove(&first));
        assert_eq!(container.len(), 1);
        assert_eq!(container.get_by_task_unique_id(Some(10)).len(), 1);
        assert_eq!(container.get_by_resource_unique_id(Some(20)).len(), 1);
        assert!(container.get_by_unique_id(Some(1)).is_none());

// Removing the same element again returns false
        assert!(!container.remove(&first));

// Located by Rc pointer identity: another Rc pointing to the same object should also hit
        assert!(container.remove(&Rc::clone(&second)));
        assert!(container.is_empty());
        assert!(container.get_by_resource_unique_id(Some(20)).is_empty());
    }

    #[test]
    fn remove_by_unique_id_drops_element() {
        let mut container = ResourceAssignmentContainer::new();
        assignment(&mut container, 1, 10, 20);

        assert!(!container.remove_by_unique_id(None));
        assert!(!container.remove_by_unique_id(Some(2)));
        assert!(container.remove_by_unique_id(Some(1)));
        assert!(container.is_empty());
    }

    #[test]
    fn resource_assignment_has_no_parent_file() {
// An unwired assignment cannot remove itself (Java's remove() depends on the parent file)
        let mut assignment = ResourceAssignment::new();
        assignment.set_unique_id(Some(1));
        assignment.set_work(Some(Duration::get_instance(1.0, TimeUnit::Hours)));

        assert!(!assignment.remove());
    }

    #[test]
    fn context_wiring_and_sequence_sync() {
        let context = ProjectContext::shared();
        let mut container = ResourceAssignmentContainer::new();
        container.set_context(&context);

        assert!(container.get_context().is_some());
        assert!(container.get_by_unique_id(Some(1)).is_none());
        assert!(container.get_custom_fields().is_empty());
        assert!(container.get_field_type_by_alias("alias").is_none());

        container.update_unique_id(Some(1), Some(20));

        let next = context
            .borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<ResourceAssignment>())
            .get_next();
        assert_eq!(next, 21);
    }

    #[test]
    fn update_unique_id_without_context_is_a_no_op() {
        let mut container = ResourceAssignmentContainer::new();
        container.update_unique_id(Some(1), Some(2));
        assert!(container.get_context().is_none());
    }

    #[test]
    fn default_matches_new() {
        let container = ResourceAssignmentContainer::default();
        assert!(container.is_empty());
    }

    #[test]
    fn debug_reports_size_and_context_state() {
        let mut container = ResourceAssignmentContainer::new();
        assert!(format!("{container:?}").contains("context_is_set: false"));

        assignment(&mut container, 1, 10, 20);
        let text = format!("{container:?}");
        assert!(text.contains("size: 1"));
        assert!(text.contains("task_index_size: 1"));
        assert!(text.contains("resource_index_size: 1"));
    }
}
