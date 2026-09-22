//! Corresponds to Java: `org/mpxj/Notes.java`

use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

/// Common trait for plain-text notes; calling `Display` (equivalent to Java's
/// `toString()`) yields the plain-text representation of the note.
///
/// Corresponds to Java: `public class Notes`
///
/// # How the Inheritance Relationship Is Reproduced
///
/// In Java `Notes` is an instantiable base class, and `ParentNotes` / `StructuredNotes` /
/// `RtfNotes` / `HtmlNotes` all `extends Notes`. Rust has no inheritance, so this port
/// reproduces the base class `Notes` as `pub trait Notes`: each subclass is a separate
/// struct that implements this trait; state carried by the base-class field in Java
/// (`m_text`) is placed directly into each struct.
///
/// In Java `new Notes(text)` instantiates the base class directly to represent plain-text
/// notes; once the base class is reproduced as a trait, that form is carried by
/// [`PlainNotes`], with semantics identical to Java's `Notes`.
///
/// # Skipped Java Methods
///
/// None.
///
/// # Differences from Java
///
/// Java's field table shares `Notes` objects by reference (`Task.setNotesObject(Notes)`
/// stores the same instance directly), with no "copy" step. On the Rust side
/// [`crate::field_value::FieldValue::Notes`] needs to hold `Rc<dyn Notes>`, while existing
/// call sites hand out `&dyn Notes` (e.g. `task.set_notes_object(Some(&rtf_notes))` in
/// `mpp8_reader`), so each subtype implements [`Notes::to_box`] to provide a
/// **subtype-preserving** shallow copy ([`Notes::to_rc`] derives a shared handle from it).
pub trait Notes: Display {
    /// Determine whether the note text is empty.
    ///
    /// Corresponds to Java: `public boolean isEmpty()`
    ///
    /// Java's implementation is `String text = toString(); return text == null || text.isEmpty();`.
    /// Rust's `Display` can never return `null`, so this only checks whether the output is an
    /// empty string.
    fn is_empty(&self) -> bool {
        self.to_string().is_empty()
    }

    /// Copy into a new `Box<dyn Notes>` (Rust-specific method).
    ///
    /// Corresponds to Java: none (see "Differences from Java" in the trait docs).
    ///
    /// Each subtype must ensure the returned object has the **same concrete subtype** as
    /// itself rather than a degraded plain-text form; otherwise the subtype information of
    /// `RtfNotes` / `HtmlNotes` / `StructuredNotes` would be lost.
    fn to_box(&self) -> Box<dyn Notes>;

    /// Copy into a shared handle `Rc<dyn Notes>` (Rust-specific method).
    ///
    /// Corresponds to Java: none (see "Differences from Java" in the trait docs).
    ///
    /// The default implementation is based on [`Notes::to_box`], so subclasses need not
    /// reimplement it.
    fn to_rc(&self) -> Rc<dyn Notes> {
        Rc::from(self.to_box())
    }
}

/// Plain-text note.
///
/// Corresponds to Java: `public class Notes` (the concrete form when the base class is
/// instantiated directly)
///
/// Note: Java's base class `Notes` can both serve as a base class and be used directly via
/// `new Notes(text)`. A Rust trait cannot be instantiated, so this type carries the
/// concrete "plain-text note" form.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlainNotes {
    /// Note text.
    ///
    /// Corresponds to Java: `private final String m_text;`
    m_text: Option<String>,
}

impl PlainNotes {
    /// Constructor.
    ///
    /// Corresponds to Java: `public Notes(String text)`
    ///
    /// Java allows passing `null` (`ParentNotes` / `StructuredNotes` do so via `super(null)`),
    /// hence `Option` is used here.
    pub fn new(text: Option<String>) -> PlainNotes {
        PlainNotes { m_text: text }
    }
}

impl Display for PlainNotes {
    /// Return the plain-text content.
    ///
    /// Corresponds to Java: `@Override public String toString()`
    ///
    /// Java returns `null` when `m_text` is `null`; Rust's `Display` cannot express `null`, so
    /// here an empty string is written when it is `None`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.m_text {
            Some(text) => f.write_str(text),
            None => Ok(()),
        }
    }
}

impl Notes for PlainNotes {
    /// Corresponds to Java: none (Rust-specific subtype-preserving copy, see [`Notes::to_box`])
    fn to_box(&self) -> Box<dyn Notes> {
        Box::new(PlainNotes::new(self.m_text.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_notes_returns_text() {
        let notes = PlainNotes::new(Some(String::from("hello")));
        assert_eq!(notes.to_string(), "hello");
        assert!(!notes.is_empty());
    }

    #[test]
    fn empty_and_null_text_are_empty() {
        assert!(PlainNotes::new(Some(String::new())).is_empty());
        assert!(PlainNotes::new(None).is_empty());
        assert_eq!(PlainNotes::new(None).to_string(), "");
    }

    #[test]
    fn notes_trait_object_supports_empty_check() {
        let notes: Box<dyn Notes> = Box::new(PlainNotes::new(Some(String::from("x"))));
        assert!(!notes.is_empty());
        assert_eq!(notes.to_string(), "x");
    }
}
