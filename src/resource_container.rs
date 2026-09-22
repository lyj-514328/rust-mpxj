//! Corresponds to Java: `org/mpxj/ResourceContainer.java`
//!
//! # How the inheritance hierarchy is reused (composition + forwarding)
//!
//! Java's `ResourceContainer extends ProjectEntityWithIDContainer<Resource> implements ChildResourceContainer`.
//! Rust has no inheritance; by convention this class **composes** a
//! [`ProjectEntityWithIDContainer<Resource>`] and forwards the base class's public behavior one method at a time (see the notes on each method),
//! Only behavior new to this class (resource creation, child resource list, hierarchy rebuild, removal cleanup, custom field lookup) is implemented separately.
//! This differs from [`crate::project_calendar_container`], which "does not compose the base class and looks up linearly by unique ID":
//! That file cannot compose because `ProjectCalendar` does not implement `ProjectEntityWithUniqueID`, whereas
//! [`crate::resource::Resource`] already implements `ProjectEntityWithID` and `Ord`, so the base class can be reused in full,
//! so "composition + forwarding" is used here.
//!
//! # Differences from Java
//!
//! - **Constructor**: Java's is `ResourceContainer(ProjectContext context)`. On the Rust side
//! [`ResourceContainer::new`] takes no arguments (when [`crate::project_context::ProjectContext::new`] calls it
//! the context has not been built yet), and is then wired up by [`ResourceContainer::set_context`];
//! [`crate::project_context::ProjectContext::shared`] performs that wiring
//! (along with the calendar container and the sequence provider adapter).
//! - **Sequence generator provider**: Java passes `ProjectContext` itself (which implements
//! `UniqueIdObjectSequenceProvider`) to the base class. On the Rust side a strong reference to the context would form an `Rc` cycle,
//! so this file defines `ResourceSequenceProvider`, which holds only a weak reference (isomorphic to
//! `ContextSequenceProvider` in `project_context.rs`): when the context can be borrowed it forwards to it, otherwise it degrades gracefully to its own sequence,
//! ensuring no panic.
//! - **Context back-reference**: `private final ProjectContext m_context` → `Weak<RefCell<ProjectContext>>`.
//! - **Element shape**: `Resource` is `Rc<RefCell<Resource>>` (shared entity convention).
//! - **Overloads (Rust has no overloading)**:
//!
//!   | Java | Rust |
//!   |---|---|
//!   | `public Resource add()` | [`ResourceContainer::add`] |
//!   | `@Override public Resource addResource()` | [`ResourceContainer::add_resource`] |
//! | `ListWithCallbacks.add(T element)` (inherited) | [`ResourceContainer::add_resource_entry`] (suffixed to distinguish it from the argument-less `add()` of the same name) |
//! | `ListWithCallbacks.remove(Object)` + `removed(T)` | [`ResourceContainer::remove_resource`] (accepts an element handle) |
//!   | `ListWithCallbacks.remove(int index)` | [`ResourceContainer::remove`] |
//! - **Automatic numbering in `add`**: Java's `new Resource(m_context)` assigns the unique ID / ID inside the constructor;
//! on the Rust side `&mut self` cannot obtain its own `Rc` (Java uses `mapID(id, this)`), so instead
//! [`ResourceContainer::add`] assigns it after wrapping the `Rc` and wiring up its own weak reference (see
//! the file header of [`crate::resource`]). The automatic numbering switches come from `ProjectConfig`; when the context is unavailable
//! the Java defaults are used (both `true`).
//! - **`removeAssignments`**: Java deletes matching resource assignments one by one with `Iterator.remove()`.
//! The Rust side calls `ResourceAssignmentContainer`'s remove-by-resource-unique-ID method
//! (see "Interface assumptions" below).
//!
//! # Skipped Java Methods
//!
//! | Java | Reason |
//! |---|---|
//! | None | `getPopulatedFields` has been ported; the parent context passes in custom fields to avoid re-entrant borrowing |
//!
//! Skipped Java mechanisms: `Serializable` (unused by this class).
//!
//! # Interface assumptions about related modules
//!
//! - `crate::project_file::ProjectFile`:`get_resource_assignments()` →
//!   `&ResourceAssignmentContainer`,`get_resource_assignments_mut()` →
//!   `&mut ResourceAssignmentContainer`.
//! - `crate::resource_assignment_container::ResourceAssignmentContainer`:
//! `remove_by_resource_unique_id(Option<i32>)` (corresponds to the iterative deletion in Java's `removeAssignments`).

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::{Rc, Weak};

use crate::common::object_sequence::ObjectSequence;
use crate::custom_field::CustomField;
use crate::field_key::FieldKey;
use crate::field_type_class::FieldTypeClass;
use crate::project_context::ProjectContext;
use crate::project_entity_with_id::ProjectEntityWithID;
use crate::project_entity_with_id_container::ProjectEntityWithIDContainer;
use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
use crate::project_file::ProjectFile;
use crate::resource::Resource;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;

/// Manages the collection of resources in a project.
///
/// Corresponds to Java: `public class ResourceContainer extends ProjectEntityWithIDContainer<Resource> implements ChildResourceContainer`
pub struct ResourceContainer {
/// Base class container (Java's `extends ProjectEntityWithIDContainer<Resource>`).
    container: ProjectEntityWithIDContainer<Resource>,

/// Parent context (weak reference to avoid an `Rc` cycle).
    ///
    /// Corresponds to Java: `private final ProjectContext m_context;`
    context: Weak<RefCell<ProjectContext>>,

/// Sequence generator provider handed to the base class (see the file header; Java passes `this` directly).
    sequence_provider: Rc<RefCell<ResourceSequenceProvider>>,

/// Top-level child resource list.
    ///
    /// Corresponds to Java: `private final List<Resource> m_childResources = new ArrayList<>();`
    child_resources: Vec<Rc<RefCell<Resource>>>,
}

impl ResourceContainer {
/// Returns the fields for which at least one resource has a non-default cached value.
    ///
/// Corresponds to Java: `public Set<FieldType> getPopulatedFields()`; custom fields are
/// passed in by the context caller; as in Java, resources do not use project properties as defaults.
    pub fn get_populated_fields(
        &self,
        custom: impl IntoIterator<Item = FieldKey>,
    ) -> std::collections::HashSet<FieldKey> {
        crate::common::populated_fields::get_populated_fields(
            crate::common::populated_fields::merge_fields(
                crate::resource_field::ResourceField::ALL.map(FieldKey::Resource), custom,
            ),
            self.iter().cloned(),
            crate::common::populated_fields::Defaults::without_project(),
        )
    }

/// Constructor.
    ///
    /// Corresponds to Java: `public ResourceContainer(ProjectContext context)`
    ///
/// Argument-less constructor on the Rust side; [`ResourceContainer::set_context`] wires up the context.
    pub fn new() -> Self {
        let sequence_provider = Rc::new(RefCell::new(ResourceSequenceProvider::new()));
        let base_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            sequence_provider.clone();

        Self {
            container: ProjectEntityWithIDContainer::new(base_provider),
            context: Weak::new(),
            sequence_provider,
            child_resources: Vec::new(),
        }
    }

/// Wires up the parent context (Rust-specific: Java receives `this` directly in the constructor).
    ///
/// Corresponds to Java: `m_context = context;` in the constructor, and the base class constructor's
/// `super(context)` (sequence generator provider).
    pub(crate) fn set_context(&mut self, context: &Rc<RefCell<ProjectContext>>) {
        self.context = Rc::downgrade(context);
        self.sequence_provider.borrow_mut().set_context(context);
    }

/// Returns the parent context (Rust-specific: Java has no such getter).
    pub fn get_context(&self) -> Option<Rc<RefCell<ProjectContext>>> {
        self.context.upgrade()
    }

/// Returns the top-level child resource list (Java returns a live list, Rust returns a snapshot).
    ///
    /// Corresponds to Java: `@Override public List<Resource> getChildResources()`
    pub fn get_child_resources(&self) -> Vec<Rc<RefCell<Resource>>> {
        self.child_resources.clone()
    }

/// Creates a new resource and adds it to the container.
    ///
    /// Corresponds to Java: `@Override public Resource addResource()`
    pub fn add_resource(&mut self) -> Rc<RefCell<Resource>> {
        self.add()
    }

/// Creates a new resource and adds it to the container.
    ///
    /// Corresponds to Java: `public Resource add()`
    pub fn add(&mut self) -> Rc<RefCell<Resource>> {
        let resource = Rc::new(RefCell::new(Resource::new(self.context.clone())));
        resource.borrow_mut().set_self_reference(&resource);

        let (auto_unique_id, auto_id) = self.auto_id_flags();

        if auto_unique_id {
            let next_unique_id = self
                .container
                .get_sequence_provider()
                .borrow_mut()
                .get_unique_id_object_sequence(TypeId::of::<Resource>())
                .get_next();
            resource.borrow_mut().set_unique_id(Some(next_unique_id));
        }

        if auto_id {
            let next_id = self.container.get_next_id();
            resource.borrow_mut().set_id(Some(next_id));
            self.map_id(next_id, Rc::clone(&resource));
        }

        self.container.add(Rc::clone(&resource));
        self.child_resources.push(Rc::clone(&resource));
        resource
    }

/// Adds an existing resource to the base class container (Java inherits `add(T element)` from `ListWithCallbacks`).
    ///
/// Rust has no overloading, so it is distinguished from the argument-less [`ResourceContainer::add`] of the same name by the `_entry` suffix.
    pub fn add_resource_entry(&mut self, resource: Rc<RefCell<Resource>>) {
        self.container.add(resource);
    }

/// Removes a resource from the container and performs post-removal cleanup.
    ///
/// Corresponds to Java: `ListWithCallbacks.remove(Object)` (internally calls `@Override protected void removed(Resource)`)
    ///
/// Java locates the element by reference equality; the Rust side uses `Rc::ptr_eq` for pointer identity comparison.
    pub fn remove_resource(&mut self, resource: &Rc<RefCell<Resource>>) -> bool {
        let index = self
            .container
            .iter()
            .position(|candidate| Rc::ptr_eq(candidate, resource));

        let Some(index) = index else {
            return false;
        };

// Java: super.removed(resource) -- the base class's removed callback fires in remove(index).
        self.container.remove(index);
        self.removed(resource);
        true
    }

/// Cleanup after a resource is removed.
    ///
    /// Corresponds to Java: `@Override protected void removed(Resource resource)`
    fn removed(&mut self, resource: &Rc<RefCell<Resource>>) {
        // Java: m_idMap.remove(resource.getID());
        self.unmap_id(resource.borrow().get_id());

        // Java:
        // Resource parentResource = resource.getParentResource();
        // if (parentResource != null) { parentResource.removeChildResource(resource); }
        // else { m_childResources.remove(resource); }
        //
// Java's getParentResource() goes through the context's resource container; the Rust side goes through this container's own
// unique ID index: in most call scenarios the container is currently mutably borrowed (the caller holds the context's
// `borrow_mut`), and borrowing the context again at that point would fail (see the file header).
        let parent_unique_id = resource.borrow().get_parent_resource_unique_id();
        let parent = self.get_by_unique_id(parent_unique_id);
        match parent {
            Some(parent) => {
                if let Ok(mut parent_ref) = parent.try_borrow_mut() {
                    parent_ref.remove_child_resource(resource);
                }
            }
            None => self.child_resources.retain(|item| !Rc::ptr_eq(item, resource)),
        }

        // Java: m_context.getProjects().forEach(p -> removeAssignments(p, resource));
        let projects = match self.context.upgrade() {
            Some(context) => match context.try_borrow() {
                Ok(context_ref) => context_ref.get_projects(),
                Err(_) => Vec::new(),
            },
            None => Vec::new(),
        };
        for project in projects {
            self.remove_assignments(&project, resource);
        }

        // Java:
        // ProjectCalendar calendar = resource.getCalendar();
        // if (calendar != null) { calendar.remove(); }
        let calendar = resource.borrow().get_calendar();
        if let Some(calendar) = calendar {
            if let Ok(mut calendar_ref) = calendar.try_borrow_mut() {
                calendar_ref.remove();
            }
        }
    }

/// Deletes all resource assignments belonging to the given resource in a project.
    ///
    /// Corresponds to Java: `private void removeAssignments(ProjectFile project, Resource resource)`
    fn remove_assignments(
        &mut self,
        project: &Rc<RefCell<ProjectFile>>,
        resource: &Rc<RefCell<Resource>>,
    ) {
        let resource_unique_id = resource.borrow().get_unique_id();
        if let Ok(mut project_ref) = project.try_borrow_mut() {
            project_ref
                .get_resource_assignments_mut()
                .remove_by_resource_unique_id(resource_unique_id);
        }
    }

/// Rebuilds the resource hierarchy based on the parent resource ID.
    ///
    /// Corresponds to Java: `void updateStructure()`
    pub fn update_structure(&mut self) {
        if self.len() <= 1 {
            return;
        }

        let resources: Vec<Rc<RefCell<Resource>>> = self.container.iter().cloned().collect();

        // Java: m_childResources.clear(); this.forEach(r -> r.getChildResources().clear());
        self.child_resources.clear();
        for resource in &resources {
            resource.borrow_mut().get_child_resources_mut().clear();
        }

        // Java: this.forEach(r -> { ... });
        for resource in &resources {
// Java uses r.getParentResource() (goes through the context's resource container); the Rust side goes through this container's own
// unique ID index, for the same reason as `removed` (avoid borrowing the context again while it is mutably borrowed).
            let parent_unique_id = resource.borrow().get_parent_resource_unique_id();
            let parent = self.get_by_unique_id(parent_unique_id);
            match parent {
                None => self.child_resources.push(Rc::clone(resource)),
                Some(parent) => {
                    if let Ok(mut parent_ref) = parent.try_borrow_mut() {
                        parent_ref.add_child_resource(resource);
                    }
                }
            }
        }
    }

/// Returns all custom field configurations for a given entity type.
    ///
    /// Corresponds to Java: `public List<CustomField> getCustomFields()`
    ///
/// Returns an empty list when the context is unavailable (this branch cannot occur in Java).
    pub fn get_custom_fields(&self) -> Vec<Rc<RefCell<CustomField>>> {
        match self.context.upgrade() {
            Some(context) => match context.try_borrow() {
                Ok(context_ref) => context_ref
                    .get_custom_fields()
                    .borrow()
                    .get_custom_fields_by_field_type_class(FieldTypeClass::Resource),
                Err(_) => Vec::new(),
            },
            None => Vec::new(),
        }
    }

/// Returns the field type by alias.
    ///
    /// Corresponds to Java: `public FieldType getFieldTypeByAlias(String alias)`
    pub fn get_field_type_by_alias(&self, alias: &str) -> Option<FieldKey> {
        let context = self.context.upgrade()?;
        let context_ref = context.try_borrow().ok()?;
        context_ref
            .get_custom_fields()
            .borrow()
            .get_field_type_by_alias(FieldTypeClass::Resource, alias)
    }

/// Reads the automatic numbering switches (`ProjectConfig.getAutoResourceUniqueID` / `getAutoResourceID`).
    ///
/// When the context is unavailable the Java defaults are used (both `true`).
    fn auto_id_flags(&self) -> (bool, bool) {
        match self.context.upgrade() {
            Some(context) => match context.try_borrow() {
                Ok(context_ref) => {
                    let config = context_ref.get_project_config();
                    (
                        config.get_auto_resource_unique_id(),
                        config.get_auto_resource_id(),
                    )
                }
                Err(_) => (true, true),
            },
            None => (true, true),
        }
    }

/// Returns the sequence generator provider.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.getSequenceProvider()`
    pub fn get_sequence_provider(&self) -> Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> {
        self.container.get_sequence_provider()
    }

/// The first value used when renumbering unique IDs.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.firstUniqueID()`
    pub fn first_unique_id(&self) -> i32 {
        self.container.first_unique_id()
    }

/// Looks up a resource by unique ID, returning `None` if not found.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Resource>>> {
        self.container.get_by_unique_id(id)
    }

/// Looks up a resource by ID, returning `None` if not found.
    ///
    /// Corresponds to Java: `ProjectEntityWithIDContainer.getByID(Integer id)`
    pub fn get_by_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Resource>>> {
        self.container.get_by_id(id)
    }

/// Removes an ID mapping.
    ///
    /// Corresponds to Java: `ProjectEntityWithIDContainer.unmapID(Integer id)`
    pub fn unmap_id(&mut self, id: Option<i32>) {
        self.container.unmap_id(id);
    }

/// Adds an ID mapping.
    ///
    /// Corresponds to Java: `ProjectEntityWithIDContainer.mapID(Integer id, T entity)`
    pub fn map_id(&mut self, id: i32, resource: Rc<RefCell<Resource>>) {
        self.container.map_id(id, resource);
    }

/// Returns the next available ID.
    ///
    /// Corresponds to Java: `ProjectEntityWithIDContainer.getNextID()`
    pub fn get_next_id(&self) -> i32 {
        self.container.get_next_id()
    }

/// Inserts a resource at the given index (returns `false` if out of bounds).
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(int index, T element)`
    pub fn add_at(&mut self, index: usize, resource: Rc<RefCell<Resource>>) -> bool {
        self.container.add_at(index, resource)
    }

/// Replaces the resource at the given index (returns `None` if out of bounds).
    ///
    /// Corresponds to Java: `ListWithCallbacks.set(int index, T element)`
    pub fn set(&mut self, index: usize, resource: Rc<RefCell<Resource>>) -> Option<Rc<RefCell<Resource>>> {
        self.container.set(index, resource)
    }

/// Removes the resource at the given index (returns `None` if out of bounds).
    ///
    /// Corresponds to Java: `ListWithCallbacks.remove(int index)`
    pub fn remove(&mut self, index: usize) -> Option<Rc<RefCell<Resource>>> {
        self.container.remove(index)
    }

/// Clears the resource list (does not fire the `removed` callback, consistent with Java).
    ///
    /// Corresponds to Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.container.clear();
    }

/// Returns the resource at the given index.
    ///
    /// Corresponds to Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<Rc<RefCell<Resource>>> {
        self.container.get(index)
    }

/// Number of resources.
    ///
    /// Corresponds to Java: `ListWithCallbacks.size()`
    pub fn len(&self) -> usize {
        self.container.len()
    }

/// Whether the resource list is empty.
    ///
    /// Corresponds to Java: `ListWithCallbacks.isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.container.is_empty()
    }

/// Iterates over the resource list.
    ///
    /// Corresponds to Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<Resource>>> {
        self.container.iter()
    }

/// Accesses the resource list as a slice.
    ///
/// Corresponds to Java: the list view of `ListWithCallbacks`.
    pub fn as_slice(&self) -> &[Rc<RefCell<Resource>>] {
        self.container.as_slice()
    }

/// Sorts by the resources' own order (`compareTo`, i.e. ID).
    ///
    /// Corresponds to Java: `Collections.sort(this)`
    pub fn sort(&mut self) {
        self.container.sort();
    }

/// Updates the unique ID index.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.updateUniqueID(T element, Integer oldUniqueID, Integer newUniqueID)`
    pub fn update_unique_id(
        &mut self,
        resource: &Rc<RefCell<Resource>>,
        old_unique_id: Option<i32>,
        new_unique_id: Option<i32>,
    ) {
        self.container
            .update_unique_id(resource, old_unique_id, new_unique_id);
    }

/// Reassigns unique IDs for all resources with unique ID clashes.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.fixUniqueIdClashes()`
    pub fn fix_unique_id_clashes(&mut self) {
        self.container.fix_unique_id_clashes();
    }

/// Makes all resource IDs contiguous.
    ///
    /// Corresponds to Java: `ProjectEntityWithIDContainer.renumberIDs()`
    pub fn renumber_ids(&mut self) {
        self.container.renumber_ids();
    }

/// Accesses the internal base class container.
    ///
/// Corresponds to Java's `extends ProjectEntityWithIDContainer<Resource>` relationship; the Rust side expresses it via composition,
/// so the base class instance must be exposed explicitly.
    pub fn container(&self) -> &ProjectEntityWithIDContainer<Resource> {
        &self.container
    }
}

impl Default for ResourceContainer {
/// Corresponds to Java: the explicitly declared `ResourceContainer(ProjectContext)`,
/// which on the Rust side is equivalent to [`ResourceContainer::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for ResourceContainer {
/// Java does not override `toString()`; the Rust side outputs the container size, child resource count and context wiring state.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceContainer")
            .field("container", &self.container)
            .field("child_resources", &self.child_resources.len())
            .field("context_is_set", &(self.context.strong_count() > 0))
            .finish()
    }
}

/// Sequence generator provider used by [`ResourceContainer`] (Rust-specific, see the file header).
///
/// Isomorphic to `ContextSequenceProvider` inside [`crate::project_context`]: it forwards to the context first,
/// and degrades gracefully to its own sequence when the context is unavailable (not wired up or already borrowed), ensuring no panic.
struct ResourceSequenceProvider {
/// Target context (weak reference to avoid an `Rc` cycle).
    context: Weak<RefCell<ProjectContext>>,
/// Fallback sequence generator used when the context is unavailable.
    fallback: HashMap<TypeId, Rc<ObjectSequence>>,
}

impl ResourceSequenceProvider {
    fn new() -> Self {
        Self {
            context: Weak::new(),
            fallback: HashMap::new(),
        }
    }

/// Wires up the target context (writes only the weak reference).
    fn set_context(&mut self, context: &Rc<RefCell<ProjectContext>>) {
        self.context = Rc::downgrade(context);
    }
}

impl UniqueIdObjectSequenceProvider for ResourceSequenceProvider {
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

/// Constructs a container with the context already wired up.
    fn container() -> (Rc<RefCell<ProjectContext>>, ResourceContainer) {
        let context = ProjectContext::shared();
        let mut container = ResourceContainer::new();
        container.set_context(&context);
        (context, container)
    }

    #[test]
    fn container_basics() {
        let (_, mut container) = container();
        assert!(container.is_empty());
        assert_eq!(container.first_unique_id(), 1);

        container.add();
        container.add();

        assert_eq!(container.len(), 2);
        assert!(!container.is_empty());
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert!(container.get(0).is_some());
        assert!(container.get(2).is_none());
    }

    #[test]
    fn add_assigns_ids_and_registers_child_resource() {
        let (_, mut container) = container();

        let resource = container.add();

        let unique_id = resource.borrow().get_unique_id();
        let id = resource.borrow().get_id();
        assert!(unique_id.is_some());
        assert!(id.is_some());
        assert_eq!(container.len(), 1);
        assert_eq!(container.get_child_resources().len(), 1);

// Both the unique ID and ID indexes can be looked up
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(unique_id).expect("unique ID indexed"),
            &resource
        ));
        assert!(Rc::ptr_eq(
            &container.get_by_id(id).expect("ID mapped"),
            &resource
        ));

// addResource() is equivalent to add()
        let second = container.add_resource();
        assert_eq!(container.len(), 2);
        assert_ne!(
            second.borrow().get_unique_id(),
            unique_id,
            "auto numbering should not repeat"
        );
    }

    #[test]
    fn add_resource_entry_registers_existing_resource() {
        let (context, mut container) = container();

        let resource = Rc::new(RefCell::new(Resource::new(Rc::downgrade(&context))));
        resource.borrow_mut().set_unique_id(Some(9));

        container.add_resource_entry(Rc::clone(&resource));

        assert_eq!(container.len(), 1);
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(9)).expect("indexed"),
            &resource
        ));
        assert!(container.get_child_resources().is_empty());
    }

    #[test]
    fn remove_resource_drops_indexes_and_child_entry() {
        let (_, mut container) = container();
        let resource = container.add();
        let id = resource.borrow().get_id();

        assert!(container.remove_resource(&resource));
        assert!(container.is_empty());
        assert!(container.get_by_id(id).is_none());
        assert!(container.get_child_resources().is_empty());

// Removing again returns false
        assert!(!container.remove_resource(&resource));
    }

    #[test]
    fn remove_by_index_is_forwarded() {
        let (_, mut container) = container();
        container.add();
        container.add();

        let removed = container.remove(0).expect("index 0 exists");
        assert!(removed.borrow().get_id().is_some());
        assert_eq!(container.len(), 1);
        assert!(container.remove(5).is_none());
    }

    #[test]
    fn update_structure_rebuilds_hierarchy_from_parent_ids() {
        let context = ProjectContext::shared();
        let mut container = ResourceContainer::new();
        container.set_context(&context);

        let parent = Rc::new(RefCell::new(Resource::new(Rc::downgrade(&context))));
        let child = Rc::new(RefCell::new(Resource::new(Rc::downgrade(&context))));
        let orphan = Rc::new(RefCell::new(Resource::new(Rc::downgrade(&context))));

        parent.borrow_mut().set_unique_id(Some(1));
        child.borrow_mut().set_unique_id(Some(2));
        orphan.borrow_mut().set_unique_id(Some(3));
        child.borrow_mut().set_parent_resource(Some(&parent));

        container.add_resource_entry(Rc::clone(&parent));
        container.add_resource_entry(Rc::clone(&child));
        container.add_resource_entry(Rc::clone(&orphan));

        container.update_structure();

// Top level: parent and orphan (child is attached under parent)
        let top_level = container.get_child_resources();
        assert_eq!(top_level.len(), 2);
        assert_eq!(parent.borrow().get_child_resources().len(), 1);
        assert!(Rc::ptr_eq(
            &parent.borrow().get_child_resources()[0],
            &child
        ));
    }

    #[test]
    fn unique_id_clashes_are_detected_and_fixed() {
        let context = ProjectContext::shared();
        let mut container = ResourceContainer::new();
        container.set_context(&context);

        let first = Rc::new(RefCell::new(Resource::new(Rc::downgrade(&context))));
        let second = Rc::new(RefCell::new(Resource::new(Rc::downgrade(&context))));
        first.borrow_mut().set_unique_id(Some(1));
        second.borrow_mut().set_unique_id(Some(2));

        container.add_resource_entry(Rc::clone(&first));
        container.add_resource_entry(Rc::clone(&second));

// Create a clash: second switches to the same unique ID as first (advancing the sequence to 2 at the same time)
        container.update_unique_id(&second, Some(2), Some(1));
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(1)).expect("indexed"),
            &second
        ));

        container.fix_unique_id_clashes();

// The clashing resource is renumbered
        assert_eq!(second.borrow().get_unique_id(), Some(2));
        assert_eq!(first.borrow().get_unique_id(), Some(1));
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(2)).expect("indexed"),
            &second
        ));
    }

    #[test]
    fn map_and_unmap_id_are_forwarded() {
        let (context, mut container) = container();
        let resource = Rc::new(RefCell::new(Resource::new(Rc::downgrade(&context))));

        container.map_id(7, Rc::clone(&resource));
        assert!(Rc::ptr_eq(
            &container.get_by_id(Some(7)).expect("mapped"),
            &resource
        ));

        container.unmap_id(Some(7));
        assert!(container.get_by_id(Some(7)).is_none());
        assert!(container.get_by_id(None).is_none());
    }

    #[test]
    fn renumber_ids_is_forwarded() {
        let (context, mut container) = container();
        let first = Rc::new(RefCell::new(Resource::new(Rc::downgrade(&context))));
        let second = Rc::new(RefCell::new(Resource::new(Rc::downgrade(&context))));
        first.borrow_mut().set_id(Some(5));
        second.borrow_mut().set_id(Some(9));

        container.add_resource_entry(Rc::clone(&first));
        container.add_resource_entry(Rc::clone(&second));
        container.renumber_ids();

        let ids: Vec<Option<i32>> = container
            .iter()
            .map(|resource| resource.borrow().get_id())
            .collect();
        assert_eq!(ids, vec![Some(1), Some(2)]);
    }

    #[test]
    fn context_backed_queries_degrade_without_context() {
        let mut container = ResourceContainer::new();

        assert!(container.get_context().is_none());
        assert!(container.get_custom_fields().is_empty());
        assert!(container.get_field_type_by_alias("alias").is_none());

// Resources can still be created when the context is not wired up (using the fallback sequence generator and Java default automatic numbering)
        let resource = container.add();
        assert!(resource.borrow().get_unique_id().is_some());
        assert!(resource.borrow().get_id().is_some());
    }

    #[test]
    fn context_backed_queries_work_when_wired() {
        let (_context, container) = container();

        assert!(container.get_context().is_some());
// Empty custom field container
        assert!(container.get_custom_fields().is_empty());
        assert!(container.get_field_type_by_alias("missing").is_none());
    }

    #[test]
    fn default_matches_new() {
        let container = ResourceContainer::default();
        assert!(container.is_empty());
    }

    #[test]
    fn debug_reports_size_and_context_state() {
        let mut container = ResourceContainer::new();
        assert!(format!("{container:?}").contains("context_is_set: false"));

        container.add();
        let text = format!("{container:?}");
        assert!(text.contains("ResourceContainer"));
    }
}
