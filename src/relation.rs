//! Corresponds to Java: `org/mpxj/Relation.java`
//!
//! A relationship between two tasks (predecessor → successor).
//!
//! # Java → Rust method mapping
//!
//! | Java | Rust |
//! |---|---|
//! | `private Relation(Builder builder)` | [`Builder::build`] |
//! | `public RelationType getType()` | [`Relation::get_type`] |
//! | `public Duration getLag()` | [`Relation::get_lag`] (returns `Option`, see below) |
//! | `public Task getPredecessorTask()` | [`Relation::get_predecessor_task`] |
//! | `public Task getSuccessorTask()` | [`Relation::get_successor_task`] |
//! | `@Override public Integer getUniqueID()` | [`Relation::get_unique_id`] |
//! | `@Override public void setUniqueID(Integer)` | [`Relation::set_unique_id`] |
//! | `public String getNotes()` | [`Relation::get_notes`] |
//! | `public boolean getDriving()` | [`Relation::get_driving`] |
//! | `public void setDriving(boolean)` | [`Relation::set_driving`] |
//! | `@Override public String toString()` | `Display` (a `Debug` is also provided) |
//! | `implements ProjectEntityWithMutableUniqueID` | implements [`ProjectEntityWithUniqueID`] + [`ProjectEntityWithMutableUniqueID`] |
//! | `Builder.from(Relation)` | [`Builder::from`] (an associated function on the Rust side) |
//! | `Builder.uniqueID / predecessorTask / successorTask / type / lag / notes / driving / build` | same names in snake_case (`type` → `r#type`, consistent with the existing `r#type` usage in this crate) |
//!
//! Note: Java's `Relation.Builder` is a static nested class; Rust cannot declare a type inside a type, so, as with
//! [`crate::shift::Builder`], it becomes a module-level type [`Builder`] (`crate::relation::Builder`).
//!
//! # Skipped Java code and rationale
//!
//! | Java | Reason |
//! |---|---|
//! | Automatic unique ID assignment in the constructor when `uniqueID == null`, via `m_successorTask.getParentFile()` → `ProjectConfig.getAutoRelationUniqueID()` / `getUniqueIdObjectSequence(Relation.class)` | **Wired up**, but the landing point is [`crate::relation_container::RelationContainer::add_predecessor`] (Java does it in `builder.build()`): Rust's [`Builder::build`] returns a bare value and cannot access the container, so it cannot perform "assign + register with the container" at the construction point; the container side implements Java's three steps one by one, see that method's comments. |
//! | `m_successorTask.getParentFile().getRelations().updateUniqueID(this, m_uniqueID, uniqueID)` in `setUniqueID` | Not wired up: `Relation` has no weak reference to its own `Rc` (no `self_ref`; [`crate::relation::Relation::set_unique_id`] only has `&mut self`), so the `element` argument of `updateUniqueID` cannot be provided. The Rust container looks up linearly by unique ID, so the index needs no callback maintenance; synchronising the sequence generator is done by the container's own `update_unique_id` (called by `add_predecessor` / `fix_unique_id_clashes`). |
//! | Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable` | This class does not use these mechanisms |
//!
//! # Differences from Java
//!
//! - **Source / target task references use `Weak`**: Java's `m_predecessorTask` / `m_successorTask` are strong references;
//! on the Rust side, if a strong reference were used between `Task` and the `RelationContainer` that holds the `Rc<RefCell<Relation>>`,
//! it would form a "task → relation → task" `Rc` cycle causing a memory leak, so instead
//! `Weak<RefCell<Task>>` is used, upgraded on use by [`Relation::get_predecessor_task`] /
//! [`Relation::get_successor_task`], returning `None` when the task has been dropped (no such case in Java).
//! The construction entry point is changed to [`Builder`] accordingly: the task is passed as `&Rc<RefCell<Task>>` and then downgraded to
//! `Weak`, consistent with the existing approach in [`crate::shift::Shift`] / [`crate::shift_period::ShiftPeriod`].
//! `build()` returns a `Relation` value type (`Relation` itself does not need a back-reference), and
//! `RelationContainer` is responsible for wrapping it in `Rc<RefCell<..>>`.
//! - **`getLag()` returns `Option<Duration>`**: Java's `m_lag` is non-null (the constructor replaces `null` with
//! 0 days). The Rust side uses `None` to mean "not explicitly set", equivalent to Java's 0 days in lag comparisons;
//! the existing caller in this crate, [`crate::cpm::microsoft_slack_calculator`], also uses it as `Option<Duration>`.
//! - **Value equality semantics**: Java does not override `equals` / `hashCode` (i.e. reference equality), and the container relies on that semantics,
//! so the Rust side does **not** derive `PartialEq` / `Eq` / `Hash`.
//! - **Tasks in `toString()`**: Java concatenates `Task.toString()` (outputting the task's field contents); the Rust side instead
//! outputs the task's unique ID (without relying on `Task`'s `Display` implementation).
//! - **`Builder.from`**: Java uses an instance method (`new Builder().from(relation)`); the Rust side uses an associated function;
//! Java's `from` does not copy `m_driving`; the Rust side stays consistent.
//! - **`Builder.type(null)`**: Java's `type(RelationType)` allows passing `null` (during construction it falls back to
//! `FINISH_START`, and is unequal when compared); Rust's parameter type `RelationType` is non-nullable, so there is no corresponding form.
//! - **`Builder.lag(null)`**: Java's `lag(Duration)` allows passing `null` (replaced with 0 days during construction;
//! comparing it directly throws `NullPointerException`); the Rust side's `lag(None)` means "not set", equivalent to 0 days.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::fmt::{self, Debug, Display, Formatter};
use std::rc::{Rc, Weak};

use crate::duration::Duration;
use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
use crate::relation_type::RelationType;
use crate::task::Task;
use crate::time_unit::TimeUnit;

/// A relationship between two tasks.
///
/// Corresponds to Java: `public final class Relation implements ProjectEntityWithMutableUniqueID`
pub struct Relation {
    /// Corresponds to Java field `private Integer m_uniqueID;`
    unique_id: Option<i32>,
    /// Corresponds to Java field `private boolean m_driving;`
    driving: bool,
/// Corresponds to Java field `private final Task m_predecessorTask;` (a weak reference on the Rust side, see the file header)
    predecessor_task: Weak<RefCell<Task>>,
/// Corresponds to Java field `private final Task m_successorTask;` (a weak reference on the Rust side, see the file header)
    successor_task: Weak<RefCell<Task>>,
    /// Corresponds to Java field `private final RelationType m_type;`
    relation_type: RelationType,
/// Corresponds to Java field `private final Duration m_lag;` (on the Rust side `None` is equivalent to Java's 0 days)
    lag: Option<Duration>,
    /// Corresponds to Java field `private final String m_notes;`
    notes: Option<String>,
}

impl Relation {
/// Get the relation type.
    ///
    /// Corresponds to Java: `public RelationType getType()`
    pub fn get_type(&self) -> RelationType {
        self.relation_type
    }

/// Get the relation's lag duration; `None` means not set, equivalent to Java's 0 days.
    ///
    /// Corresponds to Java: `public Duration getLag()`
    ///
/// See the file header for why `Option` is returned (on the Java side it is "non-null, replaced with 0 days when unset").
    pub fn get_lag(&self) -> Option<Duration> {
        self.lag
    }

/// Get the relation's predecessor task; returns `None` when the predecessor task has been dropped.
    ///
    /// Corresponds to Java: `public Task getPredecessorTask()`
    pub fn get_predecessor_task(&self) -> Option<Rc<RefCell<Task>>> {
        self.predecessor_task.upgrade()
    }

/// Get the relation's successor task; returns `None` when the successor task has been dropped.
    ///
    /// Corresponds to Java: `public Task getSuccessorTask()`
    pub fn get_successor_task(&self) -> Option<Rc<RefCell<Task>>> {
        self.successor_task.upgrade()
    }

/// Get the relation's unique ID.
    ///
    /// Corresponds to Java: `@Override public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.unique_id
    }

/// Set the relation's unique ID.
    ///
    /// Corresponds to Java: `@Override public void setUniqueID(Integer uniqueID)`
    ///
/// Java also calls `m_successorTask.getParentFile().getRelations().updateUniqueID(this, ...)`
/// to synchronise the container's sequence generator; this callback **cannot be wired up** on the Rust side — under `&mut self` one cannot obtain one's own
/// `Rc` handle (`Relation` has no `self_ref`), so the `element` argument of `updateUniqueID` cannot be constructed.
/// Sequence synchronisation is instead handled by the container's own
    /// [`RelationContainer::update_unique_id`](crate::relation_container::RelationContainer::update_unique_id)
/// (`add_predecessor` and `fix_unique_id_clashes` call it).
    pub fn set_unique_id(&mut self, unique_id: Option<i32>) {
        self.unique_id = unique_id;
    }

/// Get the notes on the relation.
    ///
    /// Corresponds to Java: `public String getNotes()`
    pub fn get_notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }

/// Whether this is a driving relation (populated internally by MPXJ only when scheduling with `PrimaveraScheduler`).
    ///
    /// Corresponds to Java: `public boolean getDriving()`
    pub fn get_driving(&self) -> bool {
        self.driving
    }

/// Set whether this is a driving relation (used internally by MPXJ).
    ///
    /// Corresponds to Java: `public void setDriving(boolean driving)`
    pub fn set_driving(&mut self, driving: bool) {
        self.driving = driving;
    }
}

impl ProjectEntityWithUniqueID for Relation {
    /// Corresponds to Java: `@Override public Integer getUniqueID()`
    fn get_unique_id(&self) -> Option<i32> {
        Relation::get_unique_id(self)
    }
}

impl ProjectEntityWithMutableUniqueID for Relation {
    /// Corresponds to Java: `@Override public void setUniqueID(Integer uniqueID)`
    fn set_unique_id(&mut self, id: Option<i32>) {
        Relation::set_unique_id(self, id);
    }
}

impl Display for Relation {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java concatenates the two `Task`s' `toString()` directly; the Rust side substitutes the tasks' unique IDs (see the file header).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[Relation lag: {} type: {} {} -> {}]",
            self.lag.unwrap_or_else(zero_days),
            self.relation_type,
            describe_task(&self.predecessor_task),
            describe_task(&self.successor_task)
        )
    }
}

impl Debug for Relation {
/// The Java side uses only `toString()`; the Rust side also provides a structured `Debug`, where task references output their unique ID
/// (avoiding reliance on `Task`'s `Debug` implementation).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Relation")
            .field("unique_id", &self.unique_id)
            .field("relation_type", &self.relation_type)
            .field("lag", &self.lag)
            .field("driving", &self.driving)
            .field("notes", &self.notes)
            .field("predecessor_task_unique_id", &task_unique_id(&self.predecessor_task))
            .field("successor_task_unique_id", &task_unique_id(&self.successor_task))
            .finish()
    }
}

/// Relation builder.
///
/// Corresponds to Java: `public static class Relation.Builder`
///
/// # Differences from Java
///
/// - Java's `Builder` fields are **package-visible** and read directly by `RelationContainer` (`builder.m_successorTask`
/// etc.); Rust has no package visibility, so here `pub(crate)` fields express the equivalent, and the reading approach is exactly the same as Java's.
/// - Java's methods modify in place and return `this`; the Rust side follows the approach of [`crate::shift::Builder`]:
/// consuming itself and returning `Self`.
/// - Java allows `type(null)` / `lag(null)`; on the Rust side the `type` parameter is non-nullable, and `lag(None)` means 0 days
/// (see the file header).
#[derive(Debug)]
pub struct Builder {
    /// Corresponds to Java field `Integer m_uniqueID;`
    pub(crate) unique_id: Option<i32>,
    /// Corresponds to Java field `Task m_predecessorTask;`
    pub(crate) predecessor_task: Weak<RefCell<Task>>,
    /// Corresponds to Java field `Task m_successorTask;`
    pub(crate) successor_task: Weak<RefCell<Task>>,
    /// Corresponds to Java field `RelationType m_type = RelationType.FINISH_START;`
    pub(crate) relation_type: RelationType,
    /// Corresponds to Java field `Duration m_lag = Duration.getInstance(0, TimeUnit.DAYS);`
    ///
/// The Rust side uses `None` to express the same default value (0 days).
    pub(crate) lag: Option<Duration>,
    /// Corresponds to Java field `String m_notes;`
    pub(crate) notes: Option<String>,
    /// Corresponds to Java field `boolean m_driving;`
    pub(crate) driving: bool,
}

impl Builder {
/// Constructor (each field uses the same default value as Java).
    ///
/// Corresponds to Java: `Relation.Builder`'s implicit default constructor.
    pub fn new() -> Self {
        Self {
            unique_id: None,
            predecessor_task: Weak::new(),
            successor_task: Weak::new(),
            relation_type: RelationType::FinishStart,
            lag: None,
            notes: None,
            driving: false,
        }
    }

/// Initialise the builder from an existing relation's fields (does not copy `driving`, consistent with Java).
    ///
    /// Corresponds to Java: `public Builder from(Relation value)`
    pub fn from(value: &Relation) -> Self {
        Self {
            unique_id: value.unique_id,
            predecessor_task: value.predecessor_task.clone(),
            successor_task: value.successor_task.clone(),
            relation_type: value.relation_type,
            lag: value.lag,
            notes: value.notes.clone(),
            driving: false,
        }
    }

/// Set the unique ID.
    ///
    /// Corresponds to Java: `public Builder uniqueID(Integer value)`
    pub fn unique_id(mut self, value: Option<i32>) -> Self {
        self.unique_id = value;
        self
    }

/// Set the predecessor task (the Rust side accepts an `Rc` and downgrades it to `Weak` internally).
    ///
    /// Corresponds to Java: `public Builder predecessorTask(Task value)`
    pub fn predecessor_task(mut self, value: &Rc<RefCell<Task>>) -> Self {
        self.predecessor_task = Rc::downgrade(value);
        self
    }

/// Set the successor task (the Rust side accepts an `Rc` and downgrades it to `Weak` internally).
    ///
    /// Corresponds to Java: `public Builder successorTask(Task value)`
    pub fn successor_task(mut self, value: &Rc<RefCell<Task>>) -> Self {
        self.successor_task = Rc::downgrade(value);
        self
    }

/// Set the relation type.
    ///
    /// Corresponds to Java: `public Builder type(RelationType value)`
    ///
/// The Java method is named `type`, but `type` is a keyword in Rust, so, following the existing convention in this crate
/// (see `r#type` in `crate::project_calendar`), it is written as `r#type`.
    pub fn r#type(mut self, value: RelationType) -> Self {
        self.relation_type = value;
        self
    }

/// Set the lag duration; `None` means 0 days (Java's default value).
    ///
    /// Corresponds to Java: `public Builder lag(Duration value)`
    pub fn lag(mut self, value: Option<Duration>) -> Self {
        self.lag = value;
        self
    }

/// Set the notes.
    ///
    /// Corresponds to Java: `public Builder notes(String value)`
    pub fn notes(mut self, value: Option<String>) -> Self {
        self.notes = value;
        self
    }

/// Set the driving relation flag.
    ///
    /// Corresponds to Java: `public Builder driving(boolean value)`
    pub fn driving(mut self, value: bool) -> Self {
        self.driving = value;
        self
    }

/// Construct a [`Relation`] instance.
    ///
    /// Corresponds to Java: `public Relation build()`
    ///
/// Java's constructor also performs automatic unique ID assignment and registers with the container (see "Skipped Java code" in the file header,
/// first line): Rust's [`Builder::build`] returns a bare value and cannot access the container, so these steps are
    /// [`RelationContainer::add_predecessor`](crate::relation_container::RelationContainer::add_predecessor)
/// executed immediately after creating the instance (same semantics and order). Therefore an instance obtained by calling `build()` **directly**
/// still has the unique ID provided by the builder (or `None` if not provided).
    pub fn build(self) -> Relation {
        Relation {
            unique_id: self.unique_id,
            driving: self.driving,
            predecessor_task: self.predecessor_task,
            successor_task: self.successor_task,
            relation_type: self.relation_type,
            lag: self.lag,
            notes: self.notes,
        }
    }
}

impl Default for Builder {
/// Corresponds to Java: `Relation.Builder`'s implicit default constructor.
    fn default() -> Self {
        Self::new()
    }
}

/// 0 days (the default lag corresponding to `Duration.getInstance(0, TimeUnit.DAYS)` in Java).
fn zero_days() -> Duration {
    Duration::get_instance_int(0, TimeUnit::Days)
}

/// Read the unique ID of a weakly referenced task (returns `None` when the task has been dropped or is currently mutably borrowed).
fn task_unique_id(task: &Weak<RefCell<Task>>) -> Option<i32> {
    task.upgrade()
        .and_then(|task| task.try_borrow().ok().and_then(|task| task.get_unique_id()))
}

/// Produce the textual form of a task reference as in Java's `Relation.toString()` (Java uses `Task.toString()`).
fn describe_task(task: &Weak<RefCell<Task>>) -> String {
    match task_unique_id(task) {
        Some(unique_id) => format!("Task {unique_id}"),
        None => "null".to_owned(),
    }
}

/// Compare two lags: equivalent to Java's `Duration.compareTo(...) == 0`
/// (units are converted first when different, see `Ord` for [`Duration`]).
pub(crate) fn lag_matches(lhs: Option<Duration>, rhs: Option<Duration>) -> bool {
    lhs.unwrap_or_else(zero_days).cmp(&rhs.unwrap_or_else(zero_days)) == Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;

/// Construct a standalone task instance (not bound to a parent project file).
    fn task() -> Rc<RefCell<Task>> {
        Rc::new(RefCell::new(Task::new()))
    }

    #[test]
    fn builder_defaults_match_java() {
        let relation = Builder::new().build();

        assert_eq!(relation.get_type(), RelationType::FinishStart);
// Java: the Builder's lag defaults to 0 days; the Rust side expresses the equivalent as None
        assert_eq!(relation.get_lag(), None);
        assert_eq!(relation.get_notes(), None);
        assert!(!relation.get_driving());
        assert_eq!(relation.get_unique_id(), None);
        assert!(relation.get_predecessor_task().is_none());
        assert!(relation.get_successor_task().is_none());
    }

    #[test]
    fn builder_sets_all_fields() {
        let predecessor = task();
        let successor = task();
        let lag = Duration::get_instance(2.0, TimeUnit::Days);

        let mut relation = Builder::new()
            .unique_id(Some(7))
            .predecessor_task(&predecessor)
            .successor_task(&successor)
            .r#type(RelationType::StartStart)
            .lag(Some(lag))
            .notes(Some("Notes".to_owned()))
            .driving(true)
            .build();

        assert_eq!(relation.get_unique_id(), Some(7));
        assert_eq!(relation.get_type(), RelationType::StartStart);
        assert_eq!(relation.get_lag(), Some(lag));
        assert_eq!(relation.get_notes(), Some("Notes"));
        assert!(relation.get_driving());
        assert!(Rc::ptr_eq(
            &relation.get_predecessor_task().expect("predecessor task still exists"),
            &predecessor
        ));
        assert!(Rc::ptr_eq(
            &relation.get_successor_task().expect("successor task still exists"),
            &successor
        ));

// setDriving / setUniqueID are writable
        relation.set_driving(false);
        relation.set_unique_id(Some(9));
        assert!(!relation.get_driving());
        assert_eq!(relation.get_unique_id(), Some(9));
    }

    #[test]
    fn from_copies_fields_but_not_driving() {
        let predecessor = task();
        let successor = task();
        let original = Builder::new()
            .unique_id(Some(3))
            .predecessor_task(&predecessor)
            .successor_task(&successor)
            .r#type(RelationType::FinishFinish)
            .lag(Some(Duration::get_instance(1.0, TimeUnit::Hours)))
            .notes(Some("notes".to_owned()))
            .driving(true)
            .build();

        let copy = Builder::from(&original).build();

        assert_eq!(copy.get_unique_id(), Some(3));
        assert_eq!(copy.get_type(), RelationType::FinishFinish);
        assert_eq!(copy.get_lag(), Some(Duration::get_instance(1.0, TimeUnit::Hours)));
        assert_eq!(copy.get_notes(), Some("notes"));
// Java's from does not copy m_driving
        assert!(!copy.get_driving());
        assert!(copy.get_predecessor_task().is_some());
        assert!(copy.get_successor_task().is_some());
    }

    #[test]
    fn dropped_task_yields_none() {
        let predecessor = task();
        let successor = task();
        let relation = Builder::new()
            .predecessor_task(&predecessor)
            .successor_task(&successor)
            .build();

        drop(predecessor);
        assert!(relation.get_predecessor_task().is_none());
        assert!(relation.get_successor_task().is_some());
    }

    #[test]
    fn entity_traits_are_implemented() {
        let mut relation = Builder::new().build();

        ProjectEntityWithMutableUniqueID::set_unique_id(&mut relation, Some(5));
        assert_eq!(ProjectEntityWithUniqueID::get_unique_id(&relation), Some(5));
        assert_eq!(relation.get_unique_id(), Some(5));
    }

    #[test]
    fn display_and_debug_describe_relation() {
        let predecessor = task();
        predecessor.borrow_mut().set_unique_id(Some(1));
        let successor = task();
        successor.borrow_mut().set_unique_id(Some(2));

        let relation = Builder::new()
            .predecessor_task(&predecessor)
            .successor_task(&successor)
            .r#type(RelationType::StartFinish)
            .lag(Some(Duration::get_instance(3.0, TimeUnit::Days)))
            .build();

        // Java: "[Relation lag: 3.0d type: SF Task 1 -> Task 2]"
        assert_eq!(
            relation.to_string(),
            "[Relation lag: 3d type: SF Task 1 -> Task 2]"
        );
        assert!(format!("{relation:?}").contains("relation_type: StartFinish"));
        assert!(format!("{relation:?}").contains("predecessor_task_unique_id: Some(1)"));
    }

    #[test]
    fn lag_matches_follows_java_compare_to() {
// Java: Duration.compareTo == 0; different units are converted first
        assert!(lag_matches(None, None));
        assert!(lag_matches(
            Some(Duration::get_instance_int(0, TimeUnit::Days)),
            None
        ));
        assert!(lag_matches(
            Some(Duration::get_instance(1.0, TimeUnit::Days)),
            Some(Duration::get_instance(8.0, TimeUnit::Hours))
        ));
        assert!(!lag_matches(
            Some(Duration::get_instance(2.0, TimeUnit::Days)),
            Some(Duration::get_instance(8.0, TimeUnit::Hours))
        ));
    }
}
