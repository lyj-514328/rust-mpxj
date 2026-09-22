//! Corresponds to Java: `org/mpxj/RtfNotes.java`

use crate::common::rtf_helper::RtfHelper;
use crate::notes::Notes;
use std::fmt::{self, Display, Formatter};

/// Notes stored in RTF format.
///
/// Corresponds to Java: `public class RtfNotes extends Notes`
///
/// # How the inheritance relationship is reproduced
///
/// In Java, `RtfNotes` directly extends `Notes`; Rust has no inheritance, so it is changed to a standalone struct implementing
/// the [`crate::notes::Notes`] trait. Java stores the result of `super(RtfHelper.strip(rtf))` into the base class's
/// `m_text`, which becomes this struct's field directly; the same applies to `m_rtf`.
///
/// Note: the task description once mentioned that `RtfNotes` extends `StructuredNotes`, but in the Java source
/// `RtfNotes extends Notes` (no inheritance relationship with `StructuredNotes`); this port follows the source.
///
/// # Skipped Java Methods
///
/// None.
pub struct RtfNotes {
/// Plain text with RTF markup stripped, used by `Display`.
    ///
/// Corresponds to Java: `Notes.m_text` (assigned by `super(RtfHelper.strip(rtf))`)
    m_text: Option<String>,

/// The original RTF document.
    ///
    /// Corresponds to Java: `private final String m_rtf;`
    m_rtf: Option<String>,
}

impl RtfNotes {
/// Constructor.
    ///
    /// Corresponds to Java: `public RtfNotes(String rtf)`
    ///
/// Java's `String` reference may be `null`, so the parameter uses `Option`.
    pub fn new(rtf: Option<String>) -> RtfNotes {
        let m_text = RtfHelper::strip(rtf.as_deref());

        RtfNotes { m_text, m_rtf: rtf }
    }

/// Return the original RTF document.
    ///
    /// Corresponds to Java: `public String getRtf()`
    pub fn get_rtf(&self) -> Option<&str> {
        self.m_rtf.as_deref()
    }
}

impl Display for RtfNotes {
/// Return the plain text with RTF markup stripped.
    ///
/// Corresponds to Java: inherited from `Notes.toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.m_text {
            Some(text) => f.write_str(text),
            None => Ok(()),
        }
    }
}

impl Notes for RtfNotes {
/// Corresponds to Java: none (Rust-specific subtype-preserving copy, see [`Notes::to_box`])
    fn to_box(&self) -> Box<dyn Notes> {
        Box::new(RtfNotes {
            m_text: self.m_text.clone(),
            m_rtf: self.m_rtf.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_rtf_is_preserved() {
        let notes = RtfNotes::new(Some(String::from("plain text")));
        assert_eq!(notes.get_rtf(), Some("plain text"));
    }

    #[test]
    fn display_uses_stripped_text() {
        let rtf = String::from("plain text");
        let notes = RtfNotes::new(Some(rtf.clone()));
        assert_eq!(notes.to_string(), RtfHelper::strip(Some(rtf.as_str())).unwrap_or_default());
        assert!(!notes.is_empty());
    }

    #[test]
    fn null_rtf_is_empty() {
        let notes = RtfNotes::new(None);
        assert_eq!(notes.get_rtf(), None);
        assert!(notes.is_empty());
    }
}
