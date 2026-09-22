//! Corresponds to Java: `org/mpxj/RelationContainer.java`
//!
//! Container for all [`Relation`] instances in the current project (including the "task → predecessor / successor relation" index).
//!
//! # Java → Rust method mapping
//!
//! | Java | Rust |
//! |---|---|
//! | `public RelationContainer(ProjectFile projectFile)` | [`RelationContainer::new`] + [`RelationContainer::set_context`] (Java receives the parent file in the constructor, while Rust splits this into two steps; see below) |
//! | `public List<Relation> getPredecessors(Task task)` | [`RelationContainer::get_predecessors`] (returns a snapshot `Vec`) |
//! | `public List<Relation> getSuccessors(Task task)` | [`RelationContainer::get_successors`] (returns a snapshot `Vec`) |
//! | `@Override protected void added(Relation element)` | private method `added` (called by [`RelationContainer::add`] / [`RelationContainer::set`]) |
//! | `@Override protected void removed(Relation element)` | private method `removed` (called by [`RelationContainer::remove`] / [`RelationContainer::remove_at`] / [`RelationContainer::set`]) |
//! | `@Override protected void replaced(Relation oldElement, Relation newElement)` | private method `replaced` (called by [`RelationContainer::set`]) |
//! | `public Relation addPredecessor(Relation.Builder builder)` | [`RelationContainer::add_predecessor`] |
//! | `public boolean removePredecessor(Task, Task, RelationType, Duration)` | [`RelationContainer::remove_predecessor`] |
//! | `getByUniqueID` inherited from `ProjectEntityContainer<Relation>` | [`RelationContainer::get_by_unique_id`] |
//! | `updateUniqueID(T, Integer, Integer)` inherited from `ProjectEntityContainer` | [`RelationContainer::update_unique_id`] |
//! | `fixUniqueIdClashes()` inherited from `ProjectEntityContainer` | [`RelationContainer::fix_unique_id_clashes`] |
//! | `firstUniqueID()` inherited from `ProjectEntityContainer` | [`RelationContainer::first_unique_id`] |
//! | `add(T)` / `remove(int)` / `set(int, T)` / `clear()` / `get(int)` / `size()` / `isEmpty()` / `iterator()` inherited from `ListWithCallbacks` | [`RelationContainer::add`] / [`RelationContainer::remove_at`] / [`RelationContainer::set`] / [`RelationContainer::clear`] / [`RelationContainer::get`] / [`RelationContainer::len`] / [`RelationContainer::is_empty`] / [`RelationContainer::iter`] |
//! | `ListWithCallbacks.remove(Object)` | [`RelationContainer::remove`] (Java locates by reference equality, Rust by pointer identity; see below) |
//! | `ProjectEntityContainer.getSequenceProvider()` | not provided (on the Rust side the container's private [`RelationContainer::sequence`] method obtains the sequence provider via the parent file) |
//! | `m_uniqueIDMap` / `m_uniqueIDClashList` inside `ProjectEntityContainer` | no index (replaced by linear search) + `unique_id_clash_list` (same meaning as the Java clash list) |
//!
//! # Skipped Java code and rationale
//!
//! | Java | Reason |
//! |---|---|
//! | `getSequenceProvider()` of `ProjectEntityContainer` | on the Rust side the container directly holds weak references to the parent `ProjectFile` and `ProjectContext`, and obtains the sequence provider through them (see [`RelationContainer::sequence`]); the provider is no longer exposed separately. |
//! | internal mechanisms related to the unique ID index (`m_uniqueIDMap`) | see "Base class not composed" below. |
//! | reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable` | this class does not use these mechanisms |
//!
//! # Base class not composed (`ProjectEntityContainer`)
//!
//! Java's `RelationContainer extends ProjectEntityContainer<Relation>`; the base class provides list storage,
//! the `added` / `removed` hooks, the unique ID index, and sequence synchronization. This crate has ported
//! [`crate::project_entity_container::ProjectEntityContainer`], but it requires elements to implement
//! `ProjectEntityWithUniqueID`, stores them as `Rc<RefCell<T>>`, and its `sort` requires elements to be `Ord`;
//! more importantly, it leaves the "element → container" weak back-reference for the caller to wire up, whereas this container's
//! `added` / `removed` / `replaced` and `removePredecessor` all directly manipulate the **index** (the predecessor / successor maps),
//! and combining them with the base class callbacks would introduce additional borrow re-entry problems. Therefore, following
//! the existing approach of [`crate::project_calendar_container::ProjectCalendarContainer`],
//! this container **holds** `Vec<Rc<RefCell<Relation>>>` **itself** and implements the base class's public behavior one by one (see the table above),
//! and replaces the index with a **linear search by unique ID**: a linear search inherently does not go stale when unique IDs change,
//! so `m_uniqueIDMap` and its companion methods are no longer needed.
//!
//! # Differences from Java
//!
//! - **Back-reference to the parent file**: Java's constructor receives `ProjectFile` and keeps a strong reference in the base class
//! (`super(projectFile)`, which is also the base class's sequence provider). A strong reference on the Rust side would form an
//! `Rc` cycle of "`ProjectFile` → container → `ProjectFile`", so it is changed to
//! `Weak<RefCell<ProjectFile>>`, constructed first with an empty weak reference, then wired up by
//! [`RelationContainer::set_context`] (Java does this in the constructor).
//! - **Automatic unique ID (Java does this in the `Relation` constructor)**: Java's
//! `Relation.Builder.build()` first checks that `uniqueID == null` and
//! `ProjectConfig.getAutoRelationUniqueID()` is true, then via the successor task's parent file requests
//! an ID from the `Relation` sequence provider, and calls the container's `updateUniqueID` to register it.
//! On the Rust side [`RelationBuilder::build`](crate::relation::Builder::build) returns a bare value and
//! cannot access the container, so these steps are performed in [`RelationContainer::add_predecessor`]
//! (the semantics and order correspond exactly to the Java constructor; see the method comment).
//! - **Weak reference to `ProjectContext` (Rust-specific)**: Java's container obtains the sequence provider
//! via `m_sequenceProvider` (i.e. the parent `ProjectFile`); if the Rust container did the same and obtained it only via `ProjectFile`,
//! it would inevitably fail to obtain it while **the caller holds a `ProjectFile` borrow** — and
//! `Task::add_predecessor` is exactly the form `file.borrow_mut().get_relations_mut().add_predecessor(..)`
//! (which is also the entry point for all readers). So, like [`crate::task_container::TaskContainer`],
//! in the same way, an additional weak reference to `ProjectContext` is kept (`ProjectContext` is a separate `RefCell`,
//! unaffected by the `ProjectFile` borrow), wired up by [`RelationContainer::set_project_context`].
//! When neither source is available it degrades gracefully to "do not allocate / skip" (Java is guaranteed to be available here), without panicking.
//! - **The `Relation::set_unique_id` callback is not wired up**: Java's `setUniqueID` notifies
//! the container via `m_successorTask.getParentFile().getRelations().updateUniqueID(this, ..)`.
//! Rust's [`Relation::set_unique_id`](crate::relation::Relation::set_unique_id) has only
//! `&mut self` and cannot access its own `Rc` handle (`Relation` has no `self_ref` weak reference), so it cannot construct
//! the `element` argument of `updateUniqueID`; therefore this callback is not wired up: sequence synchronization is handled by the container's own
//! [`RelationContainer::update_unique_id`] (`add_predecessor` and
//! [`RelationContainer::fix_unique_id_clashes`] call it).
//! - **Keys of the predecessor / successor index**: Java uses `Map<Task, List<Relation>>`, keyed by the object identity of `Task`
//! (`Task` does not override `hashCode` / `equals`). On the Rust side `Task` cannot be used as a `Hash` key, so instead
//! **the memory address of the `Task` value itself** (`*const Task as usize`) is used as the identity key, semantically equivalent to Java's reference equality;
//! the keys are maintained by [`RelationContainer::added`] / [`RelationContainer::removed`],
//! so [`RelationContainer::get_predecessors`] / [`RelationContainer::get_successors`]
//! need only a `&Task` (no `Rc` required, making it convenient for callers like `Task::get_successors(&self)`).
//! - **Identity checks in `add` / `remove`**: Java's `ListWithCallbacks.remove(Object)` uses the default
//! `equals` (reference equality) to locate the element; on the Rust side [`RelationContainer::remove`] receives
//! `&Relation` and uses `RefCell::as_ptr` to compare **pointer identity** (equivalent to Java's default `equals`).
//! - **Collection return values**: Java's `getPredecessors` / `getSuccessors` return a view of the internal list
//! (the shared `EMPTY_LIST` when there is no match); the Rust side returns a **snapshot** `Vec` (the elements are `Rc` handles,
//! so the elements themselves are still shared).
//! - **Null semantics**: Java throws
//! `NullPointerException` when `element.getSuccessorTask()` is `null`; the Rust side does not panic and degrades gracefully to "do not index / not found".
//! - **Element form**: elements are `Rc<RefCell<Relation>>` (in line with the convention that "shared entities use `Rc<RefCell<T>>`"),
//! so `removePredecessor` locates and removes a hit using `Rc::ptr_eq`.
//! - **`new` is `null` in `update_unique_id`**: Java writes `null` as a map key; on the Rust side
//! no index is created (the map key is `i32`, equivalent to "not found"); all other behavior is the same.
//! - **`clear()`**: Java's `ListWithCallbacks.clear()` only empties the list, leaving removed relations
//! behind in the predecessor / successor maps (a stale index). The Rust side's [`RelationContainer::clear`] also clears both indexes,
//! so `getPredecessors` / `getSuccessors` do not return relations that are no longer in the container (safer; results on the normal read
//! path are unchanged).
//! - **`fix_unique_id_clashes` depends on the parent file**: Java can guarantee that `m_sequenceProvider` is non-null;
//! on the Rust side, when neither source (`ProjectContext` / `ProjectFile`) can obtain the sequence provider it returns immediately,
//! keeping the clash list for a retry after wiring (Java has no such case).

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::{Rc, Weak};

use crate::common::object_sequence::ObjectSequence;
use crate::duration::Duration;
use crate::project_file::ProjectFile;
use crate::relation::{lag_matches, Builder as RelationBuilder, Relation};
use crate::relation_type::RelationType;
use crate::task::Task;
// `ProjectFile` may be provided either as "implementing this trait" or as "offering an inherent method with the same signature" (see the file header
// "interface assumptions"); the former needs this import, the latter does not use it, so it is explicitly allowed to be unused.
#[allow(unused_imports)]
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;

/// Container for all relations in the current project.
///
/// Corresponds to Java: `public class RelationContainer extends ProjectEntityContainer<Relation>`
pub struct RelationContainer {
/// The relations in the container.
    ///
/// Corresponds to Java: the list storage inside the base class `ProjectEntityContainer`.
    items: Vec<Rc<RefCell<Relation>>>,

/// The "successor task → relations pointing to it (i.e. that task's predecessor relations)" index.
    ///
    /// Corresponds to Java field: `private final Map<Task, List<Relation>> m_predecessors = new HashMap<>();`
/// (the key is the identity of the Task object; on the Rust side it is expressed as the address of the Task value; see the file header)
    predecessors: HashMap<usize, Vec<Rc<RefCell<Relation>>>>,

/// The "predecessor task → relations starting from it (i.e. that task's successor relations)" index.
    ///
    /// Corresponds to Java field: `private final Map<Task, List<Relation>> m_successors = new HashMap<>();`
    successors: HashMap<usize, Vec<Rc<RefCell<Relation>>>>,

/// The unique ID clash list.
    ///
    /// Corresponds to Java field: `ProjectEntityContainer.m_uniqueIDClashList`
    unique_id_clash_list: Vec<Rc<RefCell<Relation>>>,

/// The parent project file (a weak reference, to avoid an `Rc` cycle).
    ///
/// Corresponds to Java: the `ProjectFile` passed to `RelationContainer(ProjectFile projectFile)` and kept by the base class
/// (which is also the base class's sequence provider `m_sequenceProvider`).
    context: Weak<RefCell<ProjectFile>>,

/// The project context (a weak reference).
    ///
/// Rust-specific: Java holds only the parent `ProjectFile` and accesses the sequence provider and project configuration through it indirectly.
/// The Rust side keeps a separate copy so that the `ProjectConfig` and the `Relation`
/// sequence provider needed for "automatic relation unique ID allocation" can be read **without borrowing `ProjectFile`** —
/// `ProjectContext` is a separate `RefCell`, so while the caller holds a `ProjectFile` borrow
/// (as `Task::add_predecessor` does) it can still be accessed safely. See the file header.
    project_context: Weak<RefCell<crate::project_context::ProjectContext>>,
}

impl RelationContainer {
/// Constructor.
    ///
    /// Corresponds to Java: `public RelationContainer(ProjectFile projectFile)`
    ///
/// On the Rust side it is constructed first with an empty parent-file weak reference, and [`RelationContainer::set_context`] completes the wiring.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            predecessors: HashMap::new(),
            successors: HashMap::new(),
            unique_id_clash_list: Vec::new(),
            context: Weak::new(),
            project_context: Weak::new(),
        }
    }

/// Wire up the parent project file (Rust-specific: Java receives `ProjectFile` directly in the constructor).
    ///
/// Corresponds to Java: `super(projectFile)` and the base class field `m_sequenceProvider`
    pub(crate) fn set_context(&mut self, project_file: &Rc<RefCell<ProjectFile>>) {
        self.context = Rc::downgrade(project_file);

// Rust-specific: also record `ProjectContext` (see the field comment). Here `project_file`
// may already be mutably borrowed by the caller (as happens during the wiring phase of `ProjectFile::new_with_context`),
// in which case it cannot be read, and `set_project_context` wires it up later.
        self.project_context = match project_file.try_borrow() {
            Ok(file_ref) => Rc::downgrade(&file_ref.get_project_context()),
            Err(_) => Weak::new(),
        };
    }

/// Wire up the project context (Rust-specific: Java accesses it indirectly via the parent `ProjectFile`).
    ///
/// Corresponds to Java: `m_sequenceProvider` of the base class `ProjectEntityContainer`
/// (injected at construction; the container uses it to obtain the `ProjectConfig` and the `Relation` sequence provider).
    pub(crate) fn set_project_context(
        &mut self,
        context: &Rc<RefCell<crate::project_context::ProjectContext>>,
    ) {
        self.project_context = Rc::downgrade(context);
    }

/// Return the parent project file (returns `None` when not wired up).
    ///
/// Corresponds to Java: the `ProjectFile` held by the base class (the inverse relation of `ProjectFile.getRelations()`)
    pub fn get_context(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.context.upgrade()
    }

/// Append a relation to the container and update the predecessor / successor index.
    ///
/// Corresponds to Java: `ListWithCallbacks.add(T element)` (which in turn triggers the `added` callback)
    pub fn add(&mut self, relation: Rc<RefCell<Relation>>) {
        self.items.push(Rc::clone(&relation));
        self.added(&relation);
    }

/// Return the predecessor relations of the given task; returns an empty collection when there are none.
    ///
    /// Corresponds to Java: `public List<Relation> getPredecessors(Task task)`
    pub fn get_predecessors(&self, task: &Task) -> Vec<Rc<RefCell<Relation>>> {
        self.predecessors
            .get(&task_key(task))
            .cloned()
            .unwrap_or_default()
    }

/// Return the successor relations of the given task; returns an empty collection when there are none.
    ///
    /// Corresponds to Java: `public List<Relation> getSuccessors(Task task)`
    pub fn get_successors(&self, task: &Task) -> Vec<Rc<RefCell<Relation>>> {
        self.successors
            .get(&task_key(task))
            .cloned()
            .unwrap_or_default()
    }

/// Create a predecessor relation from the provided values and add it to the container; if a relation with "same predecessor + same type + same lag" already exists between the two tasks,
/// the existing relation is returned directly (no new one is created).
    ///
    /// Corresponds to Java: `public Relation addPredecessor(Relation.Builder builder)`
    pub fn add_predecessor(&mut self, builder: RelationBuilder) -> Rc<RefCell<Relation>> {
        //
// Java: first search m_predecessors for a relation with the same predecessor and reuse it if it matches exactly
        //
        if let Some(existing) = self.find_matching_predecessor(&builder) {
            return existing;
        }

        //
// Java: create a new predecessor relation if necessary and add it to the container
        //
// Java's `Relation` constructor (i.e. `builder.build()`) completes the
// unique ID handling here in three steps, mapped one by one below:
        //
        //   Integer uniqueID = builder.m_uniqueID;
        //   if (uniqueID == null) {
        //      ProjectFile project = m_successorTask.getParentFile();
        //      if (project.getProjectConfig().getAutoRelationUniqueID()) {
        //         uniqueID = project.getUniqueIdObjectSequence(Relation.class).getNext();
        //      }
        //   }
        //   if (uniqueID != null) {
        //      m_successorTask.getParentFile().getRelations().updateUniqueID(this, m_uniqueID, uniqueID);
        //      m_uniqueID = uniqueID;
        //   }
        //
// (On the Rust side `Builder::build()` returns a bare value and cannot access the container, so these steps live in the container.)
        let mut relation = builder.build();

        // Java: if (uniqueID == null) { ... getUniqueIdObjectSequence(Relation.class).getNext(); }
        if relation.get_unique_id().is_none() && self.auto_relation_unique_id() {
            if let Some(sequence) = self.sequence() {
                relation.set_unique_id(Some(sequence.get_next()));
            }
        }

        let relation = Rc::new(RefCell::new(relation));

        // Java: if (uniqueID != null) { ...updateUniqueID(this, m_uniqueID, uniqueID); m_uniqueID = uniqueID; }
// At this point the relation has not yet been registered in the container (same in Java), and `oldUniqueID` is the pre-construction field value `null`.
        if let Some(unique_id) = read_unique_id(&relation) {
            self.update_unique_id(&relation, None, Some(unique_id));
        }

        self.add(Rc::clone(&relation));

        relation
    }

/// Remove a matching predecessor relation from the given successor task.
    ///
    /// Corresponds to Java: `public boolean removePredecessor(Task successorTask, Task predecessorTask, RelationType type, Duration lag)`
    pub fn remove_predecessor(
        &mut self,
        successor_task: &Task,
        predecessor_task: &Task,
        relation_type: RelationType,
        lag: Option<Duration>,
    ) -> bool {
        //
// Java: fetch the successor task's predecessor relation list
        //
        let candidates = match self.predecessors.get(&task_key(successor_task)) {
            Some(candidates) => candidates.clone(),
            None => return false,
        };

        if candidates.is_empty() {
            return false;
        }

        //
// Java: a null lag is treated as 0 days (on the Rust side None means 0 days)
        //
        for relation in candidates {
            let matched = match relation.try_borrow() {
                Ok(relation) => {
                    let same_predecessor = relation
                        .get_predecessor_task()
                        .map(|task| same_task(&task, predecessor_task))
                        .unwrap_or(false);

                    same_predecessor
                        && relation.get_type() == relation_type
                        && lag_matches(relation.get_lag(), lag)
                }
// Java has no such branch (the element is always readable); the Rust side avoids a borrow-conflict panic
                Err(_) => false,
            };

            if matched {
                self.remove_element(&relation);
                return true;
            }
        }

        false
    }

/// Remove a relation from the container (located by reference equality).
    ///
/// Corresponds to Java: `ListWithCallbacks.remove(Object o)` (which internally calls `removed(T element)`)
    pub fn remove(&mut self, relation: &Relation) -> bool {
        let target = relation as *const Relation;
        let index = self
            .items
            .iter()
            .position(|candidate| candidate.as_ptr() as *const Relation == target);

        match index {
            Some(index) => {
                self.remove_at(index);
                true
            }
            None => false,
        }
    }

/// Remove and return the relation at the given index (returns `None` if out of bounds).
    ///
    /// Corresponds to Java: `ListWithCallbacks.remove(int index)`
    pub fn remove_at(&mut self, index: usize) -> Option<Rc<RefCell<Relation>>> {
        if index >= self.items.len() {
            return None;
        }

        let removed = self.items.remove(index);
        self.removed(&removed);
        Some(removed)
    }

/// Replace the relation at the given index and return the replaced old relation (returns `None` if out of bounds).
    ///
/// Corresponds to Java: `ListWithCallbacks.set(int index, T element)` (triggers `replaced`)
    pub fn set(&mut self, index: usize, relation: Rc<RefCell<Relation>>) -> Option<Rc<RefCell<Relation>>> {
        if index >= self.items.len() {
            return None;
        }

        let old_relation = std::mem::replace(&mut self.items[index], relation);
        let new_relation = Rc::clone(&self.items[index]);
        self.replaced(&old_relation, &new_relation);
        Some(old_relation)
    }

/// Empty the container (does not trigger `removed`, consistent with Java).
    ///
    /// Corresponds to Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.items.clear();
        self.predecessors.clear();
        self.successors.clear();
    }

/// Return the relation at the given index; returns `None` if out of bounds.
    ///
    /// Corresponds to Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<Rc<RefCell<Relation>>> {
        self.items.get(index).map(Rc::clone)
    }

/// Number of relations.
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

/// Iterate over the relations in the container.
    ///
    /// Corresponds to Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<Relation>>> {
        self.items.iter()
    }

/// Access the relations in the container as a slice.
    ///
/// Corresponds to Java: the list view of the base class (`ArrayList`).
    pub fn as_slice(&self) -> &[Rc<RefCell<Relation>>] {
        self.items.as_slice()
    }

/// Find a relation by unique ID; returns `None` when not found.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    ///
/// The Rust side uses a linear search by unique ID (no index; see the file header).
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Relation>>> {
        let id = id?;
        self.items
            .iter()
            .find(|relation| read_unique_id(relation) == Some(id))
            .map(Rc::clone)
    }

/// The first value used when renumbering unique IDs (the base class default).
    ///
/// Corresponds to Java: `ProjectEntityContainer.firstUniqueID()` (`RelationContainer` does not override it)
    pub fn first_unique_id(&self) -> i32 {
        1
    }

/// Update the container state when a relation's unique ID changes.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.updateUniqueID(T element, Integer oldUniqueID, Integer newUniqueID)`
    ///
/// The Rust side uses a linear search by unique ID, so it only needs to maintain the clash list and the sequence provider; in Java, calls
/// triggered by the caller (`Relation.setUniqueID`) are initiated by the user on the Rust side (see
/// the note on [`crate::relation::Relation::set_unique_id`]).
    pub fn update_unique_id(
        &mut self,
        element: &Rc<RefCell<Relation>>,
        old_unique_id: Option<i32>,
        new_unique_id: Option<i32>,
    ) {
        // Java: if (oldUniqueID != null) { m_uniqueIDMap.remove(oldUniqueID); }
// The Rust side uses a linear search, so there are no index entries to remove.
        let _ = old_unique_id;

        // Java: T currentElement = m_uniqueIDMap.get(newUniqueID);
        //       if (currentElement == element) return;
        let current_element = self.get_by_unique_id(new_unique_id);
        if let Some(current_element) = &current_element {
            if Rc::ptr_eq(current_element, element) {
                return;
            }
        }

        // Java: if (currentElement instanceof ProjectEntityWithMutableUniqueID)
        //       { m_uniqueIDClashList.add(element); }
// A Relation's unique ID is mutable, so a clash is recorded consistently with Java.
        if current_element.is_some() {
            self.unique_id_clash_list.push(Rc::clone(element));
        }

        // Java: m_uniqueIDMap.put(newUniqueID, element);
        //       m_sequenceProvider.getUniqueIdObjectSequence(element.getClass()).sync(newUniqueID);
        if let Some(sequence) = self.sequence() {
            sequence.sync(new_unique_id);
        }
    }

/// Reassign unique IDs to relations with unique ID clashes.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.fixUniqueIdClashes()`
    pub fn fix_unique_id_clashes(&mut self) {
        if self.unique_id_clash_list.is_empty() {
            return;
        }

        // Java: ObjectSequence sequence = m_sequenceProvider.getUniqueIdObjectSequence(
        //          m_uniqueIDClashList.get(0).getClass());
// When neither source (`ProjectContext` / `ProjectFile`) can obtain the sequence provider on the Rust side
// renumbering is impossible (Java is guaranteed to be available here); the clash list is kept for later processing after wiring.
        let Some(sequence) = self.sequence() else {
            return;
        };

        // Java: m_uniqueIDClashList.forEach(i -> i.setUniqueID(sequence.getNext()));
        for relation in std::mem::take(&mut self.unique_id_clash_list) {
            if let Ok(mut relation) = relation.try_borrow_mut() {
                relation.set_unique_id(Some(sequence.get_next()));
            }
        }

        // Java: m_uniqueIDMap.clear(); forEach(i -> m_uniqueIDMap.put(...));
// The Rust side uses a linear search by unique ID, so there is no index to rebuild.
    }

/// Index maintenance after a relation is added.
    ///
    /// Corresponds to Java: `@Override protected void added(Relation element)`
    fn added(&mut self, element: &Rc<RefCell<Relation>>) {
        // Java: m_predecessors.computeIfAbsent(element.getSuccessorTask(), t -> new ArrayList<>()).add(element);
        if let Some(key) = referenced_task_key(element, TaskRole::Successor) {
            self.predecessors
                .entry(key)
                .or_default()
                .push(Rc::clone(element));
        }

        // Java: m_successors.computeIfAbsent(element.getPredecessorTask(), t -> new ArrayList<>()).add(element);
        if let Some(key) = referenced_task_key(element, TaskRole::Predecessor) {
            self.successors
                .entry(key)
                .or_default()
                .push(Rc::clone(element));
        }
    }

/// Index maintenance after a relation is removed.
    ///
    /// Corresponds to Java: `@Override protected void removed(Relation element)`
    fn removed(&mut self, element: &Rc<RefCell<Relation>>) {
        // Java: m_predecessors.getOrDefault(element.getSuccessorTask(), EMPTY_LIST).remove(element);
        if let Some(key) = referenced_task_key(element, TaskRole::Successor) {
            if let Some(list) = self.predecessors.get_mut(&key) {
                list.retain(|candidate| !Rc::ptr_eq(candidate, element));
            }
        }

        // Java: m_successors.getOrDefault(element.getPredecessorTask(), EMPTY_LIST).remove(element);
        if let Some(key) = referenced_task_key(element, TaskRole::Predecessor) {
            if let Some(list) = self.successors.get_mut(&key) {
                list.retain(|candidate| !Rc::ptr_eq(candidate, element));
            }
        }
    }

/// Index maintenance when a relation is replaced.
    ///
    /// Corresponds to Java: `@Override protected void replaced(Relation oldElement, Relation newElement)`
    fn replaced(&mut self, old_element: &Rc<RefCell<Relation>>, new_element: &Rc<RefCell<Relation>>) {
        // Java: removed(oldElement); added(newElement);
        self.removed(old_element);
        self.added(new_element);
    }

/// Search the "successor task's predecessor relations" for an existing relation that exactly matches the provided values (the first part of
/// Java's `addPredecessor`).
    fn find_matching_predecessor(
        &self,
        builder: &RelationBuilder,
    ) -> Option<Rc<RefCell<Relation>>> {
// Hold the Rc until the search ends, so the task is not freed and its address invalidated.
        let successor = builder.successor_task.upgrade()?;
        let predecessor = builder.predecessor_task.upgrade()?;

        let key = task_key_of(&successor)?;
        let predecessor_key = task_key_of(&predecessor)?;

        for relation in self.predecessors.get(&key)? {
            // Java: if (relation.getPredecessorTask() == builder.m_predecessorTask)
            let same_predecessor = match relation.try_borrow() {
                Ok(value) => value
                    .get_predecessor_task()
                    .map(|task| task_key_of(&task) == Some(predecessor_key))
                    .unwrap_or(false),
                Err(_) => false,
            };

            if !same_predecessor {
                continue;
            }

            // Java: if (relation.getType() == builder.m_type
            //             && relation.getLag().compareTo(builder.m_lag) == 0) { return relation; }
// Java checks only the first relation with the same predecessor (and breaks if it does not match).
            let matched = match relation.try_borrow() {
                Ok(value) => {
                    value.get_type() == builder.relation_type
                        && lag_matches(value.get_lag(), builder.lag)
                }
                Err(_) => false,
            };

            return if matched {
                Some(Rc::clone(relation))
            } else {
                None
            };
        }

        None
    }

/// Remove an element by `Rc` identity and trigger `removed`.
    fn remove_element(&mut self, element: &Rc<RefCell<Relation>>) -> bool {
        let index = self
            .items
            .iter()
            .position(|candidate| Rc::ptr_eq(candidate, element));

        match index {
            Some(index) => {
                let removed = self.items.remove(index);
                self.removed(&removed);
                true
            }
            None => false,
        }
    }

/// Obtain the `Relation` sequence provider supplied by the parent project file (returns `None` when neither source is available).
    ///
    /// Corresponds to Java: `m_sequenceProvider.getUniqueIdObjectSequence(Relation.class)`
    ///
/// The Rust side prefers to obtain it via [`RelationContainer::project_context`]: the caller (`Task::add_predecessor`
/// and the like) usually holds a mutable `ProjectFile` borrow, in which case only the separate `ProjectContext` can be borrowed.
/// As a fallback it obtains it via the parent `ProjectFile` (the fallback for when `set_project_context` has not been wired up).
    fn sequence(&self) -> Option<Rc<ObjectSequence>> {
        if let Some(context) = self.project_context.upgrade() {
            if let Ok(mut context) = context.try_borrow_mut() {
                return Some(
                    crate::unique_id_object_sequence_provider::get_unique_id_object_sequence::<
                        Relation,
                    >(&mut *context),
                );
            }
        }

        let project_file = self.context.upgrade()?;
        let mut project_file = project_file.try_borrow_mut().ok()?;

// Supports both forms: ProjectFile as an implementation of `UniqueIdObjectSequenceProvider`
// (consistent with Java), or as a type offering an inherent method with the same signature (the assumption in `crate::task`).
        Some(project_file.get_unique_id_object_sequence(TypeId::of::<Relation>()))
    }

/// The "automatically allocate relation unique IDs" switch in the project configuration (defaults to `true` in Java).
    ///
    /// Corresponds to Java: `m_successorTask.getParentFile().getProjectConfig().getAutoRelationUniqueID()`
    ///
/// The lookup path is the same as [`RelationContainer::sequence`]; returns `false` when neither source is available
/// (in that case the sequence provider is likewise unavailable, and the result matches Java: no ID is allocated).
    fn auto_relation_unique_id(&self) -> bool {
        if let Some(context) = self.project_context.upgrade() {
            if let Ok(context_ref) = context.try_borrow() {
                return context_ref
                    .get_project_config()
                    .get_auto_relation_unique_id();
            }
        }

        match self.context.upgrade() {
            Some(project_file) => match project_file.try_borrow() {
                Ok(project_file) => project_file
                    .get_project_config()
                    .get_auto_relation_unique_id(),
                Err(_) => false,
            },
            None => false,
        }
    }
}

impl Default for RelationContainer {
/// Corresponds to Java: the explicitly declared `RelationContainer(ProjectFile)`,
/// which on the Rust side is equivalent to [`RelationContainer::new`] (the parent file is wired up by `set_context`).
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for RelationContainer {
/// Java does not override `toString()`; the Rust side outputs the container size and index size
/// (to avoid depending on the `Debug` details of `Task` / `Relation`).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("RelationContainer")
            .field("size", &self.items.len())
            .field("predecessor_index_size", &self.predecessors.len())
            .field("successor_index_size", &self.successors.len())
            .field("context_is_set", &(self.context.strong_count() > 0))
            .finish()
    }
}

/// The role of a task reference within a relation (used to select the corresponding index key).
enum TaskRole {
/// The successor task (the key of `m_predecessors`).
    Successor,
/// The predecessor task (the key of `m_successors`).
    Predecessor,
}

/// Return the identity key of a task reference in a relation (returns `None` when the task has been freed or is currently mutably borrowed).
fn referenced_task_key(relation: &Rc<RefCell<Relation>>, role: TaskRole) -> Option<usize> {
    let task = match relation.try_borrow() {
        Ok(relation) => match role {
            TaskRole::Successor => relation.get_successor_task(),
            TaskRole::Predecessor => relation.get_predecessor_task(),
        },
        Err(_) => None,
    };

    task.and_then(|task| task_key_of(&task))
}

/// The identity key of a Task: the memory address of the Task value itself (equivalent to Java's reference equality).
fn task_key(task: &Task) -> usize {
    task as *const Task as usize
}

/// Return the identity key of a task wrapped in `Rc<RefCell<Task>>` (returns `None` when it is currently mutably borrowed).
fn task_key_of(task: &Rc<RefCell<Task>>) -> Option<usize> {
    task.try_borrow().ok().map(|task| task_key(&task))
}

/// Determine whether `Rc<RefCell<Task>>` and `&Task` point to the same task.
fn same_task(task: &Rc<RefCell<Task>>, other: &Task) -> bool {
    task.try_borrow()
        .map(|task| std::ptr::eq(&*task, other))
        .unwrap_or(false)
}

/// Read a relation's unique ID (returns `None` when it is currently mutably borrowed).
fn read_unique_id(relation: &Rc<RefCell<Relation>>) -> Option<i32> {
    relation
        .try_borrow()
        .ok()
        .and_then(|relation| relation.get_unique_id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_unit::TimeUnit;

/// Construct a standalone task.
    fn task() -> Rc<RefCell<Task>> {
        Rc::new(RefCell::new(Task::new()))
    }

/// Construct a relation.
    fn relation(
        predecessor: &Rc<RefCell<Task>>,
        successor: &Rc<RefCell<Task>>,
        relation_type: RelationType,
        lag: Option<Duration>,
    ) -> Rc<RefCell<Relation>> {
        Rc::new(RefCell::new(
            RelationBuilder::new()
                .predecessor_task(predecessor)
                .successor_task(successor)
                .r#type(relation_type)
                .lag(lag)
                .build(),
        ))
    }

/// Construct an unwired container.
    fn container() -> RelationContainer {
        RelationContainer::new()
    }

/// Construct a container wired to a newly created parent file.
    fn wired_container() -> (RelationContainer, Rc<RefCell<ProjectFile>>) {
        let project_file = ProjectFile::new();
        let mut container = RelationContainer::new();
        container.set_context(&project_file);
        (container, project_file)
    }

    #[test]
    fn add_indexes_predecessors_and_successors() {
        let first = task();
        let second = task();
        let third = task();

        let mut container = container();
        let ab = relation(&first, &second, RelationType::FinishStart, None);
        let cb = relation(&third, &second, RelationType::FinishStart, None);

        container.add(Rc::clone(&ab));
        container.add(Rc::clone(&cb));

        assert_eq!(container.len(), 2);
        assert!(!container.is_empty());
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert!(Rc::ptr_eq(&container.get(0).expect("index 0 exists"), &ab));
        assert!(container.get(2).is_none());

// The second task has two predecessor relations (A -> B, C -> B)
        let predecessors = container.get_predecessors(&second.borrow());
        assert_eq!(predecessors.len(), 2);
        assert!(predecessors.iter().any(|item| Rc::ptr_eq(item, &ab)));
        assert!(predecessors.iter().any(|item| Rc::ptr_eq(item, &cb)));

// The first task has one successor relation
        let successors = container.get_successors(&first.borrow());
        assert_eq!(successors.len(), 1);
        assert!(Rc::ptr_eq(&successors[0], &ab));

// A task with no relations returns an empty collection
        assert!(container.get_predecessors(&first.borrow()).is_empty());
        assert!(container.get_successors(&task().borrow()).is_empty());

// third's successor relation is C -> B, and first's predecessor relations are empty
        let third_successors = container.get_successors(&third.borrow());
        assert_eq!(third_successors.len(), 1);
        assert!(Rc::ptr_eq(&third_successors[0], &cb));
    }

    #[test]
    fn add_predecessor_deduplicates_identical_relations() {
        let predecessor = task();
        let successor = task();

        let mut container = container();

        let builder = || {
            RelationBuilder::new()
                .predecessor_task(&predecessor)
                .successor_task(&successor)
                .r#type(RelationType::FinishStart)
                .lag(Some(Duration::get_instance_int(2, TimeUnit::Days)))
        };

        let first = container.add_predecessor(builder());
        assert_eq!(container.len(), 1);

// Java: same predecessor + same type + same lag → reuse the existing relation
        let second = container.add_predecessor(builder());
        assert!(Rc::ptr_eq(&first, &second));
        assert_eq!(container.len(), 1);

// Different type → create new
        let different_type = container.add_predecessor(builder().r#type(RelationType::StartStart));
        assert!(!Rc::ptr_eq(&first, &different_type));
        assert_eq!(container.len(), 2);

// Different lag → create new (1 day ≠ 2 days)
        let different_lag = container.add_predecessor(builder().lag(Some(Duration::get_instance_int(1, TimeUnit::Days))));
        assert!(!Rc::ptr_eq(&first, &different_lag));
        assert_eq!(container.len(), 3);

// Java: checks only the first relation with the same predecessor and breaks out of the loop if it does not match fully (it does not look at later relations)
        let zero_lag_implicit = container.add_predecessor(
            RelationBuilder::new()
                .predecessor_task(&predecessor)
                .successor_task(&successor)
                .r#type(RelationType::FinishFinish),
        );
        assert_eq!(container.len(), 4);

        let zero_lag_explicit = container.add_predecessor(
            RelationBuilder::new()
                .predecessor_task(&predecessor)
                .successor_task(&successor)
                .r#type(RelationType::FinishFinish)
                .lag(Some(Duration::get_instance_int(0, TimeUnit::Days))),
        );
        assert!(!Rc::ptr_eq(&zero_lag_implicit, &zero_lag_explicit));
        assert_eq!(container.len(), 5);

// An unset lag (equivalent to 0 days) is treated the same as an explicit 0 days (the first relation with the same predecessor is the FF one)
// (container() is not called again here because a local variable of the same name already exists in the surrounding scope)
        let mut fresh_container = RelationContainer::new();
        let implicit_zero = fresh_container.add_predecessor(
            RelationBuilder::new()
                .predecessor_task(&predecessor)
                .successor_task(&successor)
                .r#type(RelationType::FinishFinish),
        );
        let explicit_zero = fresh_container.add_predecessor(
            RelationBuilder::new()
                .predecessor_task(&predecessor)
                .successor_task(&successor)
                .r#type(RelationType::FinishFinish)
                .lag(Some(Duration::get_instance_int(0, TimeUnit::Days))),
        );
        assert!(Rc::ptr_eq(&implicit_zero, &explicit_zero));
        assert_eq!(fresh_container.len(), 1);
    }

    #[test]
    fn remove_predecessor_matches_type_and_lag() {
        let predecessor = task();
        let other_predecessor = task();
        let successor = task();

        let mut container = container();
        container.add(relation(
            &predecessor,
            &successor,
            RelationType::FinishStart,
            Some(Duration::get_instance_int(1, TimeUnit::Days)),
        ));
        container.add(relation(
            &other_predecessor,
            &successor,
            RelationType::FinishStart,
            None,
        ));
        assert_eq!(container.len(), 2);

// Type does not match → do not remove
        assert!(!container.remove_predecessor(
            &successor.borrow(),
            &predecessor.borrow(),
            RelationType::StartStart,
            Some(Duration::get_instance_int(1, TimeUnit::Days)),
        ));
        assert_eq!(container.len(), 2);

// Lag does not match → do not remove
        assert!(!container.remove_predecessor(
            &successor.borrow(),
            &predecessor.borrow(),
            RelationType::FinishStart,
            Some(Duration::get_instance_int(2, TimeUnit::Days)),
        ));
        assert_eq!(container.len(), 2);

// Predecessor task does not match → do not remove
        assert!(!container.remove_predecessor(
            &successor.borrow(),
            &task().borrow(),
            RelationType::FinishStart,
            Some(Duration::get_instance_int(1, TimeUnit::Days)),
        ));
        assert_eq!(container.len(), 2);

// Exact match → remove
        assert!(container.remove_predecessor(
            &successor.borrow(),
            &predecessor.borrow(),
            RelationType::FinishStart,
            Some(Duration::get_instance_int(1, TimeUnit::Days)),
        ));
        assert_eq!(container.len(), 1);
        assert_eq!(container.get_predecessors(&successor.borrow()).len(), 1);

// A lag of None is equivalent to 0 days
        assert!(container.remove_predecessor(
            &successor.borrow(),
            &other_predecessor.borrow(),
            RelationType::FinishStart,
            None,
        ));
        assert!(container.is_empty());
        assert!(container.get_successors(&predecessor.borrow()).is_empty());
    }

    #[test]
    fn remove_predecessor_on_task_without_relations_returns_false() {
        let mut container = container();
        assert!(!container.remove_predecessor(
            &task().borrow(),
            &task().borrow(),
            RelationType::FinishStart,
            None,
        ));
    }

    #[test]
    fn remove_uses_reference_identity_and_updates_index() {
        let predecessor = task();
        let successor = task();

        let mut container = container();
        let stored = relation(&predecessor, &successor, RelationType::FinishStart, None);
        container.add(Rc::clone(&stored));

// A relation not in the container: returns false
        let standalone = relation(&predecessor, &successor, RelationType::FinishStart, None);
        let removed_unknown = {
            let value = standalone.borrow();
            container.remove(&value)
        };
        assert!(!removed_unknown);
        assert_eq!(container.len(), 1);

// A relation in the container: removed by pointer identity, cleaning up the index
        let removed = {
            let value = stored.borrow();
            container.remove(&value)
        };
        assert!(removed);
        assert!(container.is_empty());
        assert!(container.get_predecessors(&successor.borrow()).is_empty());
    }

    #[test]
    fn set_replaces_and_reindexes() {
        let first_predecessor = task();
        let second_predecessor = task();
        let successor = task();

        let mut container = container();
        let original = relation(&first_predecessor, &successor, RelationType::FinishStart, None);
        container.add(Rc::clone(&original));

        let replacement = relation(&second_predecessor, &successor, RelationType::FinishFinish, None);
        let replaced = container.set(0, Rc::clone(&replacement)).expect("index 0 exists");

        assert!(Rc::ptr_eq(&replaced, &original));
        assert!(Rc::ptr_eq(&container.get(0).expect("index 0 exists"), &replacement));

        // Java: replaced(old, new) = removed(old) + added(new)
        assert!(container.get_successors(&first_predecessor.borrow()).is_empty());
        assert_eq!(container.get_successors(&second_predecessor.borrow()).len(), 1);
        assert_eq!(container.get_predecessors(&successor.borrow()).len(), 1);

// Out of bounds does not modify the container
        assert!(container
            .set(5, relation(&first_predecessor, &successor, RelationType::FinishStart, None))
            .is_none());
        assert_eq!(container.len(), 1);
    }

    #[test]
    fn remove_at_cleans_up_index() {
        let predecessor = task();
        let successor = task();

        let mut container = container();
        container.add(relation(&predecessor, &successor, RelationType::FinishStart, None));

        assert!(container.remove_at(1).is_none());
        let removed = container.remove_at(0).expect("index 0 exists");
        assert_eq!(removed.borrow().get_type(), RelationType::FinishStart);
        assert!(container.get_predecessors(&successor.borrow()).is_empty());
    }

    #[test]
    fn clear_drops_relations_and_index() {
        let predecessor = task();
        let successor = task();

        let mut container = container();
        container.add(relation(&predecessor, &successor, RelationType::FinishStart, None));
        container.clear();

        assert!(container.is_empty());
        assert!(container.get_predecessors(&successor.borrow()).is_empty());
        assert!(container.get_successors(&predecessor.borrow()).is_empty());
    }

    #[test]
    fn unique_id_lookup_and_clash_resolution() {
        let (mut container, project_file) = wired_container();

        let predecessor = task();
        let successor = task();

        let first = relation(&predecessor, &successor, RelationType::FinishStart, None);
        let second = relation(&predecessor, &successor, RelationType::StartStart, None);

        first.borrow_mut().set_unique_id(Some(4));
        second.borrow_mut().set_unique_id(Some(5));
        container.add(Rc::clone(&first));
        container.add(Rc::clone(&second));

        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(4)).expect("registered"),
            &first
        ));
        assert!(container.get_by_unique_id(Some(6)).is_none());
        assert!(container.get_by_unique_id(None).is_none());
        assert_eq!(container.first_unique_id(), 1);

// Change second's unique ID to the same as first → it enters the clash list
        container.update_unique_id(&second, Some(5), Some(4));
        container.fix_unique_id_clashes();

// The clashing element takes the next value from the sequence provider (numbering starts at 1)
        let renumbered = read_unique_id(&second).expect("still holds unique ID");
        assert_ne!(renumbered, 4);
        assert_eq!(read_unique_id(&first), Some(4));

// The sequence provider lives in the parent file (ProjectFile implements UniqueIdObjectSequenceProvider)
        let next = UniqueIdObjectSequenceProvider::get_unique_id_object_sequence(
            &mut *project_file.borrow_mut(),
            TypeId::of::<Relation>(),
        )
        .get_next();
        assert!(next > renumbered);
    }

/// Java: the `Relation` constructor, when `uniqueID == null` and `AutoRelationUniqueID` is true,
/// (the `ProjectConfig` default), requests a unique ID from the `Relation` sequence provider.
    #[test]
    fn add_predecessor_allocates_unique_ids_when_configured() {
        let (mut container, project_file) = wired_container();

        let predecessor = task();
        let successor = task();

        let first = container.add_predecessor(
            RelationBuilder::new()
                .predecessor_task(&predecessor)
                .successor_task(&successor)
                .r#type(RelationType::FinishStart),
        );
        let second = container.add_predecessor(
            RelationBuilder::new()
                .predecessor_task(&predecessor)
                .successor_task(&successor)
                .r#type(RelationType::StartStart),
        );

// Numbering starts at 1
        assert_eq!(read_unique_id(&first), Some(1));
        assert_eq!(read_unique_id(&second), Some(2));
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(1)).expect("registered"),
            &first
        ));

// Java: the allocation is performed via `updateUniqueID`, and the sequence provider is synchronized to the allocated value
        let next = UniqueIdObjectSequenceProvider::get_unique_id_object_sequence(
            &mut *project_file.borrow_mut(),
            TypeId::of::<Relation>(),
        )
        .get_next();
        assert_eq!(next, 3);
    }

/// When the builder provides an explicit unique ID, that value is used and synchronized into the sequence provider
/// (Java: `m_sequenceProvider...sync(newUniqueID)` at the end of `updateUniqueID`).
    #[test]
    fn add_predecessor_keeps_explicit_unique_id() {
        let (mut container, project_file) = wired_container();

        let predecessor = task();
        let successor = task();

        let explicit = container.add_predecessor(
            RelationBuilder::new()
                .predecessor_task(&predecessor)
                .successor_task(&successor)
                .unique_id(Some(5)),
        );
        assert_eq!(read_unique_id(&explicit), Some(5));

        let next = UniqueIdObjectSequenceProvider::get_unique_id_object_sequence(
            &mut *project_file.borrow_mut(),
            TypeId::of::<Relation>(),
        )
        .get_next();
        assert_eq!(next, 6);
    }

/// When `AutoRelationUniqueID = false` no ID is allocated (Java: stays `null`).
    #[test]
    fn add_predecessor_without_auto_ids_leaves_unique_id_unset() {
        let (mut container, project_file) = wired_container();
        project_file
            .borrow()
            .get_project_context()
            .borrow_mut()
            .get_project_config_mut()
            .set_auto_relation_unique_id(false);

        let predecessor = task();
        let successor = task();

        let relation = container.add_predecessor(
            RelationBuilder::new()
                .predecessor_task(&predecessor)
                .successor_task(&successor),
        );

        assert_eq!(read_unique_id(&relation), None);
    }

/// Key regression test: `Task::add_predecessor` is
    /// `file.borrow_mut().get_relations_mut().add_predecessor(builder)` ——
/// During the call `ProjectFile` is in a **mutably borrowed** state, so the sequence provider can only be obtained via
/// `ProjectContext` (a separate `RefCell`); see the file header.
    #[test]
    fn add_predecessor_allocates_unique_ids_while_the_file_is_borrowed() {
        let project_file = ProjectFile::new();
        let predecessor = Rc::new(RefCell::new(Task::new_in_file(&project_file, None)));
        let successor = Rc::new(RefCell::new(Task::new_in_file(&project_file, None)));

        let relation = project_file
            .borrow_mut()
            .get_relations_mut()
            .add_predecessor(
                RelationBuilder::new()
                    .predecessor_task(&predecessor)
                    .successor_task(&successor),
            );

        assert_eq!(read_unique_id(&relation), Some(1));
        assert_eq!(project_file.borrow().get_relations().len(), 1);
    }

    #[test]
    fn unique_id_update_is_idempotent_for_same_element() {
        let (mut container, _) = wired_container();

        let predecessor = task();
        let successor = task();
        let relation = relation(&predecessor, &successor, RelationType::FinishStart, None);
        relation.borrow_mut().set_unique_id(Some(2));
        container.add(Rc::clone(&relation));

// Java: when currentElement == element it does not enter the clash list
        container.update_unique_id(&relation, Some(2), Some(2));
        container.fix_unique_id_clashes();

        assert_eq!(read_unique_id(&relation), Some(2));
    }

    #[test]
    fn fix_unique_id_clashes_without_context_keeps_clash_list() {
// Parent file not wired up: Java throws because the sequence provider is unavailable, while the Rust side degrades gracefully to doing nothing
        let mut container = container();

        let predecessor = task();
        let successor = task();
        let first = relation(&predecessor, &successor, RelationType::FinishStart, None);
        let second = relation(&predecessor, &successor, RelationType::StartStart, None);
        first.borrow_mut().set_unique_id(Some(1));
        second.borrow_mut().set_unique_id(Some(1));
        container.add(Rc::clone(&first));
        container.add(Rc::clone(&second));

        container.update_unique_id(&second, Some(1), Some(1));
        container.fix_unique_id_clashes();

        assert_eq!(read_unique_id(&second), Some(1));
    }

    #[test]
    fn default_matches_new_and_debug_reports_sizes() {
        let container = RelationContainer::default();
        assert!(container.is_empty());

        let text = format!("{container:?}");
        assert!(text.contains("size: 0"));
        assert!(text.contains("context_is_set: false"));
    }

    #[test]
    fn context_can_be_read_back() {
        let (container, project_file) = wired_container();
        assert!(Rc::ptr_eq(
            &container.get_context().expect("wired up"),
            &project_file
        ));
        assert!(RelationContainer::new().get_context().is_none());
    }
}
