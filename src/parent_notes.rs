//! Corresponds to Java: `org/mpxj/ParentNotes.java`

use crate::notes::Notes;
use std::fmt::{self, Display, Formatter};

/// A note composed of one or more child notes.
///
/// Corresponds to Java: `public class ParentNotes extends Notes`
///
/// # How the inheritance relationship is restored
///
/// In Java, `ParentNotes` directly extends `Notes`; Rust has no inheritance, so it becomes a standalone struct implementing
/// the [`crate::notes::Notes`] trait. The `m_text` that the Java constructor passes to the base class via `super(null)`
/// does not participate in any logic in this class (this class overrides `toString()`), so the field is no longer kept.
///
/// # Skipped Java Methods
///
/// None.
pub struct ParentNotes {
/// The list of child notes.
    ///
    /// Corresponds to Java: `private final List<Notes> m_childNotes;`
    m_child_notes: Vec<Box<dyn Notes>>,
}

impl ParentNotes {
/// Constructor.
    ///
    /// Corresponds to Java: `public ParentNotes(List<Notes> childNotes)`
    pub fn new(child_notes: Vec<Box<dyn Notes>>) -> ParentNotes {
        ParentNotes {
            m_child_notes: child_notes,
        }
    }

/// Return the list of child notes.
    ///
    /// Corresponds to Java: `public List<Notes> getChildNotes()`
    pub fn get_child_notes(&self) -> &[Box<dyn Notes>] {
        &self.m_child_notes
    }
}

impl Display for ParentNotes {
/// Return the concatenated result of the plain-text representations of all child notes.
    ///
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java first filters out `null` elements, then calls `toString()` on each, joins them with `\n` and calls `trim()`.
/// Rust's `Vec<Box<dyn Notes>>` contains no `null` elements, so only joining and trimming are needed.
    ///
/// Note also: Java's `String.trim()` removes only leading/trailing characters with code points `<= U+0020`, whereas Rust's
/// `str::trim()` removes all Unicode whitespace characters, so the result may differ slightly for extreme inputs.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let text = self
            .m_child_notes
            .iter()
            .map(|notes| notes.to_string())
            .collect::<Vec<String>>()
            .join("\n");

        f.write_str(text.trim())
    }
}

impl Notes for ParentNotes {
/// Corresponds to Java: none (a Rust-specific subtype-preserving copy, see [`Notes::to_box`])
    fn to_box(&self) -> Box<dyn Notes> {
        Box::new(ParentNotes::new(
            self.m_child_notes
                .iter()
                .map(|notes| notes.to_box())
                .collect(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::PlainNotes;

    fn plain(text: &str) -> Box<dyn Notes> {
        Box::new(PlainNotes::new(Some(String::from(text))))
    }

    #[test]
    fn child_notes_are_joined_with_newlines() {
        let notes = ParentNotes::new(vec![plain("a"), plain("b")]);
        assert_eq!(notes.get_child_notes().len(), 2);
        assert_eq!(notes.to_string(), "a\nb");
        assert!(!notes.is_empty());
    }

    #[test]
    fn empty_parent_notes_are_empty() {
        let notes = ParentNotes::new(Vec::new());
        assert_eq!(notes.to_string(), "");
        assert!(notes.is_empty());
    }

    #[test]
    fn surrounding_whitespace_is_trimmed() {
        let notes = ParentNotes::new(vec![plain("  a  "), plain("b")]);
        assert_eq!(notes.to_string(), "a  \nb");
    }
}
