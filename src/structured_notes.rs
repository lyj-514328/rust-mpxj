//! Corresponds to Java: `org/mpxj/StructuredNotes.java`

use crate::notes::Notes;
use crate::notes_topic::NotesTopic;
use crate::project_context::ProjectContext;
use std::fmt::{self, Display, Formatter};

/// Structured notes belonging to a topic.
///
/// Corresponds to Java: `public class StructuredNotes extends Notes`
///
/// # How the inheritance relationship is restored
///
/// In Java `StructuredNotes` directly extends `Notes`; Rust has no inheritance, so it is changed to a standalone struct implementing
/// the [`crate::notes::Notes`] trait. The Java constructor passes `super(null)` to the base class's
/// `m_text`, which is not used in this class's logic (this class overrides `toString()`), so it is no longer kept; this class's own
/// `m_uniqueID` / `m_topic` / `m_notes` are placed into the struct as-is.
///
/// # Differences from Java
///
/// - Java uses `context.getUniqueIdObjectSequence(StructuredNotes.class)` to get the unique ID
/// sequence; Rust has no class literal, so a class name string is passed instead (consistent with Java `Class.getName()`
/// being used as the map key).
/// - Java's `toString()` throws when `m_topic` is `null` and the notes are non-empty
/// a `NullPointerException`; Rust does not panic and treats the topic name as an empty string.
///
/// # Skipped Java Methods
///
/// None.
pub struct StructuredNotes {
/// Unique ID.
    ///
    /// Corresponds to Java: `private final Integer m_uniqueID;`
    m_unique_id: Option<i32>,

/// The topic the notes belong to.
    ///
    /// Corresponds to Java: `private final NotesTopic m_topic;`
    m_topic: Option<NotesTopic>,

/// The actual note content.
    ///
    /// Corresponds to Java: `private final Notes m_notes;`
    m_notes: Option<Box<dyn Notes>>,
}

impl StructuredNotes {
/// Constructor.
    ///
    /// Corresponds to Java: `public StructuredNotes(ProjectContext context, Integer uniqueID, NotesTopic topic, Notes notes)`
    ///
/// When `unique_id` is `None`, take the next value from the context's sequence; otherwise sync the sequence and return that value
/// (semantically consistent with `ObjectSequence.syncOrGetNext`).
    pub fn new(
        context: &mut ProjectContext,
        unique_id: Option<i32>,
        topic: Option<NotesTopic>,
        notes: Option<Box<dyn Notes>>,
    ) -> StructuredNotes {
        let m_unique_id = crate::unique_id_object_sequence_provider::get_unique_id_object_sequence::<
            StructuredNotes,
        >(context)
        .sync_or_get_next(unique_id);

        StructuredNotes {
            m_unique_id: Some(m_unique_id),
            m_topic: topic,
            m_notes: notes,
        }
    }

/// Return the unique ID of these notes.
    ///
    /// Corresponds to Java: `public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.m_unique_id
    }

/// Return the topic ID of these notes.
    ///
    /// Corresponds to Java: `public Integer getTopicID()`
    pub fn get_topic_id(&self) -> Option<i32> {
        self.m_topic
            .as_ref()
            .and_then(|topic| topic.get_unique_id())
    }

/// Return the topic of these notes.
    ///
    /// Corresponds to Java: `public NotesTopic getNotesTopic()`
    pub fn get_notes_topic(&self) -> Option<&NotesTopic> {
        self.m_topic.as_ref()
    }

/// Return the note content.
    ///
    /// Corresponds to Java: `public Notes getNotes()`
    pub fn get_notes(&self) -> Option<&dyn Notes> {
        self.m_notes.as_deref()
    }
}

impl Display for StructuredNotes {
/// Return "topic name + newline + note body + newline".
    ///
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java returns `null` when the body is `null` or an empty string; Rust's `Display` cannot express `null`,
/// so it outputs an empty string here (the `is_empty` result is consistent with Java).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Some(text) = self.m_notes.as_ref().map(|notes| notes.to_string()) else {
            return Ok(());
        };

        if text.is_empty() {
            return Ok(());
        }

        let name = self
            .m_topic
            .as_ref()
            .and_then(|topic| topic.get_name())
            .unwrap_or("");

        write!(f, "{}\n{}\n", name, text)
    }
}

impl Notes for StructuredNotes {
/// Corresponds to Java: none (Rust-specific subtype-preserving copy, see [`Notes::to_box`])
    fn to_box(&self) -> Box<dyn Notes> {
        Box::new(StructuredNotes {
            m_unique_id: self.m_unique_id,
            m_topic: self.m_topic.clone(),
            m_notes: self.m_notes.as_ref().map(|notes| notes.to_box()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::PlainNotes;

    fn body(text: &str) -> Option<Box<dyn Notes>> {
        Some(Box::new(PlainNotes::new(Some(String::from(text)))))
    }

    #[test]
    fn unique_id_comes_from_the_context_sequence() {
        let mut context = ProjectContext::default();
        let notes = StructuredNotes::new(&mut context, None, None, body("body"));
        assert_eq!(notes.get_unique_id(), Some(1));

        let notes = StructuredNotes::new(&mut context, None, None, body("body"));
        assert_eq!(notes.get_unique_id(), Some(2));
    }

    #[test]
    fn supplied_unique_id_is_preserved_and_syncs_sequence() {
        let mut context = ProjectContext::default();
        let notes = StructuredNotes::new(&mut context, Some(42), None, body("body"));
        assert_eq!(notes.get_unique_id(), Some(42));

// After the sequence is synced to 42, the next automatic ID should be 43.
        let notes = StructuredNotes::new(&mut context, None, None, body("body"));
        assert_eq!(notes.get_unique_id(), Some(43));
    }

    #[test]
    fn accessors_reflect_constructor_arguments() {
        let mut context = ProjectContext::default();
        let notes = StructuredNotes::new(&mut context, None, None, body("body"));
        assert!(notes.get_notes_topic().is_none());
        assert_eq!(notes.get_topic_id(), None);
        assert_eq!(
            notes.get_notes().map(|notes| notes.to_string()),
            Some(String::from("body"))
        );
    }

    #[test]
    fn empty_body_renders_nothing() {
        let mut context = ProjectContext::default();
        let notes = StructuredNotes::new(&mut context, None, None, None);
        assert_eq!(notes.to_string(), "");
        assert!(notes.is_empty());

        let notes = StructuredNotes::new(&mut context, None, None, body(""));
        assert_eq!(notes.to_string(), "");
        assert!(notes.is_empty());
    }

    #[test]
    fn non_empty_body_without_topic_renders_body_only() {
        let mut context = ProjectContext::default();
        let notes = StructuredNotes::new(&mut context, None, None, body("body"));
// Java throws an NPE when topic is null; Rust does not panic and treats the topic name as an empty string.
        assert_eq!(notes.to_string(), "\nbody\n");
        assert!(!notes.is_empty());
    }
}
