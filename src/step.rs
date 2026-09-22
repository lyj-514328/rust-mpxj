//! Corresponds to Java: `org/mpxj/Step.java`
//!
//! Skipped / unported Java methods and reasons:
//!
//! | Java | Rust |
//! |---|---|
//! | `private Step(Builder builder)` | Private function `Step::from_builder` (Java's constructor is private and called only by `Builder.build()`) |
//! | `public Task getTask()` | [`Step::get_task`] (returns `Option<Rc<RefCell<Task>>>`, see below) |
//! | `public Integer getUniqueID()` | [`Step::get_unique_id`] |
//! | `public String getName()` | [`Step::get_name`] |
//! | `public Double getPercentComplete()` | [`Step::get_percent_complete`] |
//! | `public Integer getSequenceNumber()` | [`Step::get_sequence_number`] |
//! | `public Double getWeight()` | [`Step::get_weight`] |
//! | `public String getDescription()` | [`Step::get_description`] |
//! | `public Notes getDescriptionObject()` | [`Step::get_description_object`] (returns `Option<&dyn Notes>`) |
//! | `public boolean getComplete()` | [`Step::get_complete`] |
//! | `public static class Builder` | [`Builder`] |
//! | `public Builder(Task task)` | [`Builder::new`] (takes `&Rc<RefCell<Task>>`, see below) |
//! | `public Builder from(Step value)` | [`Builder::from_step`] (renamed to avoid confusion with the `From` trait's `from`) |
//! | `public Builder uniqueID(Integer value)` | [`Builder::unique_id`] |
//! | `public Builder name(String value)` | [`Builder::name`] |
//! | `public Builder percentComplete(Double value)` | [`Builder::percent_complete`] |
//! | `public Builder sequenceNumber(Integer value)` | [`Builder::sequence_number`] |
//! | `public Builder weight(Double value)` | [`Builder::weight`] |
//! | `public Builder description(Notes value)` | [`Builder::description`] |
//! | `public Builder description(String value)` | [`Builder::description_from_string`] (Rust has no overloading) |
//! | `public Step build()` | [`Builder::build`] |
//! | `@Override public String toString()` | Unported: Java does not override it (`Object.toString()` carries no useful information); the Rust side provides `Debug` instead |
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: not used by this class, nothing skipped.
//!
//! # Differences from Java
//!
//! 1. **Parent task reference**: Java's `m_task` is a strong reference; the Rust side stores
//!    `Weak<RefCell<Task>>` (Java's `Task` holds its step list, so a strong reference would form
//!    an `Rc` cycle). [`Step::get_task`] returns `None` when the parent task has been released.
//!    The step list is stored in `crate::task::Task`'s `TaskField.STEPS` (value type
//!    [`crate::field_value::FieldValue::Steps`]), see `Task::get_steps` / `Task::set_steps`.
//! 2. **`Notes` field**: Java's `m_description` is a `Notes` object; Rust's `Notes` is a trait,
//!    so it is carried as `Rc<dyn Notes>` (shareable, reference-copyable), and
//!    [`Step::get_description_object`] returns `Option<&dyn Notes>`.
//! 3. **unique ID allocation**: Java calls
//!    `m_task.getParentFile().getUniqueIdObjectSequence(Step.class).syncOrGetNext(m_uniqueID)`
//!    in the constructor. The Rust side likewise obtains the sequence generator via the parent
//!    task's `get_parent_file()`; when the parent task is not wired up / has been released / is
//!    borrowed, it falls back to the unique ID given explicitly by the Builder (Java would throw
//!    `NullPointerException`).
//! 4. **Builder chaining**: Java's setters modify `this` and return `this`; the Rust side is
//!    **consuming** (`mut self -> Self`), and `build()` takes `&self`, semantically equivalent.
//! 5. **`NumberHelper.getDouble`**: in `getComplete()`, Java uses
//!    `NumberHelper.getDouble(m_percentComplete) == 100.0` (`null` treated as 0); the Rust side
//!    expresses this with [`Option::unwrap_or`], same behavior.

use std::any::TypeId;
use std::cell::RefCell;
use std::fmt::{self, Debug, Formatter};
use std::rc::{Rc, Weak};

use crate::notes::{Notes, PlainNotes};
use crate::project_file::ProjectFile;
use crate::task::Task;

/// Activity step.
///
/// Corresponds to Java: `public final class Step`
///
/// Implements `Clone` (Rust-specific): Java's `Task.getSteps()` returns the **live list** in
/// the field table, whereas the Rust list is held in `Rc<RefCell<Vec<Step>>>` and
/// [`crate::task::Task::get_steps`] returns a snapshot of it, so individual steps need a
/// shallow copy (`m_description` and `m_task` are both shared by reference, consistent with
/// Java's object-sharing semantics).
#[derive(Clone)]
pub struct Step {
    /// Parent task (weak reference).
    ///
    /// Corresponds to Java: `private final Task m_task;`
    m_task: Weak<RefCell<Task>>,

    /// Unique ID.
    ///
    /// Corresponds to Java: `private final Integer m_uniqueID;`
    m_unique_id: Option<i32>,

    /// Step name.
    ///
    /// Corresponds to Java: `private final String m_name;`
    m_name: Option<String>,

    /// Percent complete.
    ///
    /// Corresponds to Java: `private final Double m_percentComplete;`
    m_percent_complete: Option<f64>,

    /// Sequence number.
    ///
    /// Corresponds to Java: `private final Integer m_sequenceNumber;`
    m_sequence_number: Option<i32>,

    /// Weight.
    ///
    /// Corresponds to Java: `private final Double m_weight;`
    m_weight: Option<f64>,

    /// Step description.
    ///
    /// Corresponds to Java: `private final Notes m_description;`
    m_description: Option<Rc<dyn Notes>>,
}

impl Step {
    /// Construct from a Builder (corresponds to Java's private constructor `private Step(Builder builder)`).
    fn from_builder(builder: &Builder) -> Self {
        // Java: m_uniqueID = m_task.getParentFile().getUniqueIdObjectSequence(Step.class).syncOrGetNext(builder.m_uniqueID);
        let unique_id = builder
            .resolve_unique_id()
            .or(builder.m_unique_id);

        Self {
            m_task: builder.m_task.clone(),
            m_unique_id: unique_id,
            m_name: builder.m_name.clone(),
            m_percent_complete: builder.m_percent_complete,
            m_sequence_number: builder.m_sequence_number,
            m_weight: builder.m_weight,
            m_description: builder.m_description.clone(),
        }
    }

    /// Return the parent task (returns `None` when the parent task has been released).
    ///
    /// Corresponds to Java: `public Task getTask()`
    pub fn get_task(&self) -> Option<Rc<RefCell<Task>>> {
        self.m_task.upgrade()
    }

    /// Return the unique ID.
    ///
    /// Corresponds to Java: `public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.m_unique_id
    }

    /// Return the step name.
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.m_name.as_deref()
    }

    /// Return the percent complete.
    ///
    /// Corresponds to Java: `public Double getPercentComplete()`
    pub fn get_percent_complete(&self) -> Option<f64> {
        self.m_percent_complete
    }

    /// Return the sequence number.
    ///
    /// Corresponds to Java: `public Integer getSequenceNumber()`
    pub fn get_sequence_number(&self) -> Option<i32> {
        self.m_sequence_number
    }

    /// Return the weight.
    ///
    /// Corresponds to Java: `public Double getWeight()`
    pub fn get_weight(&self) -> Option<f64> {
        self.m_weight
    }

    /// Return the plain text of the step description.
    ///
    /// Corresponds to Java: `public String getDescription()`
    ///
    /// Java returns an empty string when the description is `null`.
    pub fn get_description(&self) -> String {
        match &self.m_description {
            Some(description) => description.to_string(),
            None => String::new(),
        }
    }

    /// Return the step description object.
    ///
    /// Corresponds to Java: `public Notes getDescriptionObject()`
    pub fn get_description_object(&self) -> Option<&dyn Notes> {
        self.m_description.as_deref()
    }

    /// Whether the step is complete (percent complete is 100).
    ///
    /// Corresponds to Java: `public boolean getComplete()`
    pub fn get_complete(&self) -> bool {
        self.m_percent_complete.unwrap_or(0.0) == 100.0
    }
}

impl Debug for Step {
    /// Java does not override `toString()`; the Rust side provides `Debug` for easier diagnosis
    /// (`dyn Notes` is not `Debug`, so the description entry outputs its plain text).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Step")
            .field("unique_id", &self.m_unique_id)
            .field("name", &self.m_name)
            .field("percent_complete", &self.m_percent_complete)
            .field("sequence_number", &self.m_sequence_number)
            .field("weight", &self.m_weight)
            .field("description", &self.get_description())
            .field("task_is_set", &(self.m_task.strong_count() > 0))
            .finish()
    }
}

/// Step builder.
///
/// Corresponds to Java: `public static class Builder`
pub struct Builder {
    /// Parent task (weak reference).
    ///
    /// Corresponds to Java: `private final Task m_task;`
    m_task: Weak<RefCell<Task>>,

    /// Corresponds to Java: `private Integer m_uniqueID;`
    m_unique_id: Option<i32>,

    /// Corresponds to Java: `private String m_name;`
    m_name: Option<String>,

    /// Corresponds to Java: `private Double m_percentComplete;`
    m_percent_complete: Option<f64>,

    /// Corresponds to Java: `private Integer m_sequenceNumber;`
    m_sequence_number: Option<i32>,

    /// Corresponds to Java: `private Double m_weight;`
    m_weight: Option<f64>,

    /// Corresponds to Java: `private Notes m_description;`
    m_description: Option<Rc<dyn Notes>>,
}

impl Builder {
    /// Constructor.
    ///
    /// Corresponds to Java: `public Builder(Task task)`
    pub fn new(task: &Rc<RefCell<Task>>) -> Self {
        Self {
            m_task: Rc::downgrade(task),
            m_unique_id: None,
            m_name: None,
            m_percent_complete: None,
            m_sequence_number: None,
            m_weight: None,
            m_description: None,
        }
    }

    /// Initialize the builder from an existing step.
    ///
    /// Corresponds to Java: `public Builder from(Step value)`
    ///
    /// Renamed to avoid the `From` trait's `from` (Rust-side naming convention).
    pub fn from_step(mut self, value: &Step) -> Self {
        self.m_unique_id = value.m_unique_id;
        self.m_name = value.m_name.clone();
        self.m_percent_complete = value.m_percent_complete;
        self.m_sequence_number = value.m_sequence_number;
        self.m_weight = value.m_weight;
        self.m_description = value.m_description.clone();
        self
    }

    /// Set the unique ID.
    ///
    /// Corresponds to Java: `public Builder uniqueID(Integer value)`
    pub fn unique_id(mut self, value: Option<i32>) -> Self {
        self.m_unique_id = value;
        self
    }

    /// Set the name.
    ///
    /// Corresponds to Java: `public Builder name(String value)`
    pub fn name(mut self, value: Option<String>) -> Self {
        self.m_name = value;
        self
    }

    /// Set the percent complete.
    ///
    /// Corresponds to Java: `public Builder percentComplete(Double value)`
    pub fn percent_complete(mut self, value: Option<f64>) -> Self {
        self.m_percent_complete = value;
        self
    }

    /// Set the sequence number.
    ///
    /// Corresponds to Java: `public Builder sequenceNumber(Integer value)`
    pub fn sequence_number(mut self, value: Option<i32>) -> Self {
        self.m_sequence_number = value;
        self
    }

    /// Set the weight.
    ///
    /// Corresponds to Java: `public Builder weight(Double value)`
    pub fn weight(mut self, value: Option<f64>) -> Self {
        self.m_weight = value;
        self
    }

    /// Set the description object.
    ///
    /// Corresponds to Java: `public Builder description(Notes value)`
    pub fn description(mut self, value: Option<Rc<dyn Notes>>) -> Self {
        self.m_description = value;
        self
    }

    /// Set the description as plain text (in Java this is carried by the `description(String)` overload; Rust has no overloading, hence a separate method name).
    ///
    /// Corresponds to Java: `public Builder description(String value)`
    ///
    /// Java: `m_description = value == null ? null : new Notes(value);`
    pub fn description_from_string(mut self, value: Option<String>) -> Self {
        self.m_description = value.map(|value| {
            let notes: Rc<dyn Notes> = Rc::new(PlainNotes::new(Some(value)));
            notes
        });
        self
    }

    /// Build a [`Step`] instance.
    ///
    /// Corresponds to Java: `public Step build()`
    pub fn build(&self) -> Step {
        Step::from_builder(self)
    }

    /// Allocate / synchronize the unique ID via the parent task's parent file.
    ///
    /// Corresponds to Java: `m_task.getParentFile().getUniqueIdObjectSequence(Step.class).syncOrGetNext(builder.m_uniqueID)`
    ///
    /// Returns `None` when any step is unavailable (task not wired up, parent file released, or borrowed).
    fn resolve_unique_id(&self) -> Option<i32> {
        let task = self.m_task.upgrade()?;
        let parent_file: Rc<RefCell<ProjectFile>> = task.try_borrow().ok()?.get_parent_file()?;
        let mut parent_file_ref = parent_file.try_borrow_mut().ok()?;

        Some(
            parent_file_ref
                .get_unique_id_object_sequence(TypeId::of::<Step>())
                .sync_or_get_next(self.m_unique_id),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a task with no parent file (outside the task graph, for testing step behavior itself).
    fn standalone_task() -> Rc<RefCell<Task>> {
        Rc::new(RefCell::new(Task::new()))
    }

    #[test]
    fn builder_populates_fields() {
        let task = standalone_task();
        let step = Builder::new(&task)
            .unique_id(Some(5))
            .name(Some("Design".to_owned()))
            .percent_complete(Some(50.0))
            .sequence_number(Some(2))
            .weight(Some(1.5))
            .description_from_string(Some("Do it".to_owned()))
            .build();

        assert_eq!(step.get_unique_id(), Some(5));
        assert_eq!(step.get_name(), Some("Design"));
        assert_eq!(step.get_percent_complete(), Some(50.0));
        assert_eq!(step.get_sequence_number(), Some(2));
        assert_eq!(step.get_weight(), Some(1.5));
        assert_eq!(step.get_description(), "Do it");
        assert!(!step.get_complete());
    }

    #[test]
    fn get_task_returns_parent_task() {
        let task = standalone_task();
        let step = Builder::new(&task).build();

        assert!(step
            .get_task()
            .map(|found| Rc::ptr_eq(&found, &task))
            .unwrap_or(false));
    }

    #[test]
    fn complete_flag_requires_percent_complete_of_one_hundred() {
        let task = standalone_task();

        assert!(!Builder::new(&task).percent_complete(Some(99.0)).build().get_complete());
        assert!(Builder::new(&task).percent_complete(Some(100.0)).build().get_complete());
        // Java: NumberHelper.getDouble(null) == 0.0
        assert!(!Builder::new(&task).build().get_complete());
    }

    #[test]
    fn description_object_is_shared_when_provided() {
        let task = standalone_task();
        let description: Rc<dyn Notes> = Rc::new(PlainNotes::new(Some("Text".to_owned())));
        let step = Builder::new(&task)
            .description(Some(Rc::clone(&description)))
            .build();

        assert_eq!(step.get_description(), "Text");
        assert!(step
            .get_description_object()
            .map(|found| found.to_string() == "Text")
            .unwrap_or(false));
    }

    #[test]
    fn description_defaults_to_empty_string() {
        let task = standalone_task();
        let step = Builder::new(&task).build();

        assert_eq!(step.get_description(), "");
        assert!(step.get_description_object().is_none());
    }

    #[test]
    fn from_step_copies_fields() {
        let task = standalone_task();
        let source = Builder::new(&task)
            .unique_id(Some(1))
            .name(Some("A".to_owned()))
            .percent_complete(Some(100.0))
            .description_from_string(Some("d".to_owned()))
            .build();

        let copy = Builder::new(&task).from_step(&source).build();

        assert_eq!(copy.get_unique_id(), Some(1));
        assert_eq!(copy.get_name(), Some("A"));
        assert_eq!(copy.get_percent_complete(), Some(100.0));
        assert_eq!(copy.get_description(), "d");
        assert!(copy.get_complete());
    }

    #[test]
    fn unique_id_falls_back_without_parent_file() {
        // The parent task has no parent file ⇒ cannot allocate a unique ID, falls back to the explicitly given value
        let task = standalone_task();
        let step = Builder::new(&task).unique_id(Some(9)).build();
        assert_eq!(step.get_unique_id(), Some(9));

        // None when not given explicitly
        let step = Builder::new(&task).build();
        assert_eq!(step.get_unique_id(), None);
    }

    #[test]
    fn debug_reports_fields() {
        let task = standalone_task();
        let step = Builder::new(&task)
            .name(Some("Design".to_owned()))
            .build();

        let text = format!("{step:?}");
        assert!(text.contains("Design"));
        assert!(text.contains("task_is_set: true"));
    }
}
