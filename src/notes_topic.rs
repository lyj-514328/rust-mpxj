//! Corresponds to Java: `org/mpxj/NotesTopic.java`
//!
//! # Differences from Java
//!
//! - **Interface**: Java's `NotesTopic implements ProjectEntityWithUniqueID`; on the Rust side it is
//!   `impl crate::project_entity_with_unique_id::ProjectEntityWithUniqueID for NotesTopic`
//! expressed by (that trait's `get_unique_id()` returns `Option<i32>`).
//! - **Unique ID assignment**: the Java constructor
//!   `UniqueIdObjectSequenceProvider#getUniqueIdObjectSequence(NotesTopic.class).syncOrGetNext(...)`
//! assigns/synchronizes the unique ID. The Rust [`Builder`] does not yet accept a sequence provider (consistent with
//! the handling of `crate::shift::Builder` and `crate::unit_of_measure::Builder`),
//! and directly uses the value passed by the caller via [`Builder::unique_id`].
//! - **Constant `DEFAULT`**: Java's `public static final NotesTopic DEFAULT` cannot, in Rust,
//! be constructed as a `const` (it contains a `String`), so it becomes the function [`NotesTopic::default_topic`],
//! and [`Default`] is also implemented for idiomatic use.

use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;

/// A topic used to organize P6 notes.
///
/// Corresponds to Java: `public final class NotesTopic implements ProjectEntityWithUniqueID`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotesTopic {
    /// Corresponds to Java field `private final Integer m_uniqueID`
    unique_id: Option<i32>,
    /// Corresponds to Java field `private final Integer m_sequenceNumber`
    sequence_number: Option<i32>,
    /// Corresponds to Java field `private final String m_name`
    name: Option<String>,
    /// Corresponds to Java field `private final boolean m_availableForEPS`
    available_for_eps: bool,
    /// Corresponds to Java field `private final boolean m_availableForProject`
    available_for_project: bool,
    /// Corresponds to Java field `private final boolean m_availableForWBS`
    available_for_wbs: bool,
    /// Corresponds to Java field `private final boolean m_availableForActivity`
    available_for_activity: bool,
}

impl NotesTopic {
/// Get the unique ID.
    ///
    /// Corresponds to Java: `@Override public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.unique_id
    }

/// Get the sequence number.
    ///
    /// Corresponds to Java: `public Integer getSequenceNumber()`
    pub fn get_sequence_number(&self) -> Option<i32> {
        self.sequence_number
    }

/// Get the name.
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

/// Get the "available for EPS" flag.
    ///
    /// Corresponds to Java: `public boolean getAvailableForEPS()`
    pub fn get_available_for_eps(&self) -> bool {
        self.available_for_eps
    }

/// Get the "available for project" flag.
    ///
    /// Corresponds to Java: `public boolean getAvailableForProject()`
    pub fn get_available_for_project(&self) -> bool {
        self.available_for_project
    }

/// Get the "available for WBS" flag.
    ///
    /// Corresponds to Java: `public boolean getAvailableForWBS()`
    pub fn get_available_for_wbs(&self) -> bool {
        self.available_for_wbs
    }

/// Get the "available for activity" flag.
    ///
    /// Corresponds to Java: `public boolean getAvailableForActivity()`
    pub fn get_available_for_activity(&self) -> bool {
        self.available_for_activity
    }

/// Construct the instance corresponding to the Java constant `NotesTopic.DEFAULT`.
    ///
    /// Corresponds to Java: `public static final NotesTopic DEFAULT`
    ///
/// Java shares the same instance; the Rust side returns a newly created value each time (`NotesTopic` is a value type).
    pub fn default_topic() -> NotesTopic {
        Builder::new()
            .unique_id(Some(1))
            .sequence_number(Some(1))
            .name(Some("Notes".to_owned()))
            .available_for_eps(true)
            .available_for_project(true)
            .available_for_wbs(true)
            .available_for_activity(true)
            .build()
    }
}

impl Default for NotesTopic {
    /// Corresponds to Java: `public static final NotesTopic DEFAULT`
    fn default() -> Self {
        NotesTopic::default_topic()
    }
}

impl ProjectEntityWithUniqueID for NotesTopic {
/// Get the unique ID.
    ///
    /// Corresponds to Java: `@Override public Integer getUniqueID()`
    fn get_unique_id(&self) -> Option<i32> {
        self.unique_id
    }
}

/// `NotesTopic` builder.
///
/// Corresponds to Java: `public static class NotesTopic.Builder`
///
/// Java's `Builder` methods modify in place and return `this`; the Rust side uses the same style as
/// `crate::shift::Builder` and `crate::unit_of_measure::Builder`:
/// "consume self and return `Self`".
#[derive(Debug, Clone, Default)]
pub struct Builder {
    /// Corresponds to Java field `private Integer m_uniqueID`
    unique_id: Option<i32>,
    /// Corresponds to Java field `private Integer m_sequenceNumber`
    sequence_number: Option<i32>,
    /// Corresponds to Java field `private String m_name`
    name: Option<String>,
    /// Corresponds to Java field `private boolean m_availableForEPS`
    available_for_eps: bool,
    /// Corresponds to Java field `private boolean m_availableForProject`
    available_for_project: bool,
    /// Corresponds to Java field `private boolean m_availableForWBS`
    available_for_wbs: bool,
    /// Corresponds to Java field `private boolean m_availableForActivity`
    available_for_activity: bool,
}

impl Builder {
/// Constructor.
    ///
    /// Corresponds to Java: `public Builder(UniqueIdObjectSequenceProvider sequenceProvider)`
    ///
/// Java's sequence provider is used to assign unique IDs via `ObjectSequence`; this repository has not yet wired it up
/// (`UniqueIdObjectSequenceProvider` is already in place), so this parameter is not accepted (see the file header).
    pub fn new() -> Self {
        Self::default()
    }

/// Initialize all fields from an existing `NotesTopic`.
    ///
    /// Corresponds to Java: `public Builder from(NotesTopic value)`
    pub fn from(mut self, value: &NotesTopic) -> Self {
        self.unique_id = value.unique_id;
        self.sequence_number = value.sequence_number;
        self.name = value.name.clone();
        self.available_for_eps = value.available_for_eps;
        self.available_for_project = value.available_for_project;
        self.available_for_wbs = value.available_for_wbs;
        self.available_for_activity = value.available_for_activity;
        self
    }

/// Set the unique ID.
    ///
    /// Corresponds to Java: `public Builder uniqueID(Integer value)`
    pub fn unique_id(mut self, value: Option<i32>) -> Self {
        self.unique_id = value;
        self
    }

/// Set the sequence number.
    ///
    /// Corresponds to Java: `public Builder sequenceNumber(Integer value)`
    pub fn sequence_number(mut self, value: Option<i32>) -> Self {
        self.sequence_number = value;
        self
    }

/// Set the name.
    ///
    /// Corresponds to Java: `public Builder name(String value)`
    pub fn name(mut self, value: Option<String>) -> Self {
        self.name = value;
        self
    }

/// Set the "available for EPS" flag.
    ///
    /// Corresponds to Java: `public Builder availableForEPS(boolean value)`
    pub fn available_for_eps(mut self, value: bool) -> Self {
        self.available_for_eps = value;
        self
    }

/// Set the "available for project" flag.
    ///
    /// Corresponds to Java: `public Builder availableForProject(boolean value)`
    pub fn available_for_project(mut self, value: bool) -> Self {
        self.available_for_project = value;
        self
    }

/// Set the "available for WBS" flag.
    ///
    /// Corresponds to Java: `public Builder availableForWBS(boolean value)`
    pub fn available_for_wbs(mut self, value: bool) -> Self {
        self.available_for_wbs = value;
        self
    }

/// Set the "available for activity" flag.
    ///
    /// Corresponds to Java: `public Builder availableForActivity(boolean value)`
    pub fn available_for_activity(mut self, value: bool) -> Self {
        self.available_for_activity = value;
        self
    }

/// Construct a [`NotesTopic`] instance.
    ///
    /// Corresponds to Java: `public NotesTopic build()`
    pub fn build(self) -> NotesTopic {
        NotesTopic {
            unique_id: self.unique_id,
            sequence_number: self.sequence_number,
            name: self.name,
            available_for_eps: self.available_for_eps,
            available_for_project: self.available_for_project,
            available_for_wbs: self.available_for_wbs,
            available_for_activity: self.available_for_activity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_sets_all_fields() {
        let topic = Builder::new()
            .unique_id(Some(7))
            .sequence_number(Some(3))
            .name(Some("Topic".to_owned()))
            .available_for_eps(true)
            .available_for_project(false)
            .available_for_wbs(true)
            .available_for_activity(false)
            .build();

        assert_eq!(topic.get_unique_id(), Some(7));
        assert_eq!(topic.get_sequence_number(), Some(3));
        assert_eq!(topic.get_name(), Some("Topic"));
        assert!(topic.get_available_for_eps());
        assert!(!topic.get_available_for_project());
        assert!(topic.get_available_for_wbs());
        assert!(!topic.get_available_for_activity());
    }

    #[test]
    fn builder_defaults_are_empty() {
        let topic = Builder::new().build();
        assert_eq!(topic.get_unique_id(), None);
        assert_eq!(topic.get_sequence_number(), None);
        assert_eq!(topic.get_name(), None);
        assert!(!topic.get_available_for_eps());
        assert!(!topic.get_available_for_project());
        assert!(!topic.get_available_for_wbs());
        assert!(!topic.get_available_for_activity());
    }

    #[test]
    fn builder_from_copies_existing_instance() {
        let source = Builder::new()
            .unique_id(Some(2))
            .sequence_number(Some(4))
            .name(Some("source".to_owned()))
            .available_for_activity(true)
            .build();

        let copy = Builder::new().from(&source).build();
        assert_eq!(copy, source);

// Java's `from` only copies fields and does not modify the original instance
        assert_eq!(source.get_name(), Some("source"));
    }

    #[test]
    fn default_topic_matches_java_default() {
        let topic = NotesTopic::default_topic();
        assert_eq!(topic.get_unique_id(), Some(1));
        assert_eq!(topic.get_sequence_number(), Some(1));
        assert_eq!(topic.get_name(), Some("Notes"));
        assert!(topic.get_available_for_eps());
        assert!(topic.get_available_for_project());
        assert!(topic.get_available_for_wbs());
        assert!(topic.get_available_for_activity());

        assert_eq!(NotesTopic::default(), topic);
    }

    #[test]
    fn implements_project_entity_with_unique_id() {
        fn unique_id_of<T: ProjectEntityWithUniqueID>(value: &T) -> Option<i32> {
            value.get_unique_id()
        }

        assert_eq!(unique_id_of(&NotesTopic::default_topic()), Some(1));
    }
}
