//! Corresponds to Java: `org/mpxj/HtmlNotes.java`

use crate::common::html_helper::HtmlHelper;
use crate::notes::Notes;
use std::fmt::{self, Display, Formatter};

/// Notes stored in HTML format.
///
/// Corresponds to Java: `public class HtmlNotes extends Notes`
///
/// # How the inheritance relationship is reproduced
///
/// In Java, `HtmlNotes` directly extends `Notes`; Rust has no inheritance, so it is changed to a standalone struct implementing
/// the [`crate::notes::Notes`] trait. Java stores the result of `super(HtmlHelper.strip(html))` into the base class's
/// `m_text`, which becomes this struct's field directly; the same applies to `m_html`.
///
/// Note: the task description once mentioned that `HtmlNotes` extends `StructuredNotes`, but in the Java source
/// `HtmlNotes extends Notes` (no inheritance relationship with `StructuredNotes`); this port follows the source.
///
/// # Skipped Java Methods
///
/// None.
pub struct HtmlNotes {
/// Plain text with HTML markup stripped, used by `Display`.
    ///
/// Corresponds to Java: `Notes.m_text` (assigned by `super(HtmlHelper.strip(html))`)
    m_text: Option<String>,

/// The original HTML document.
    ///
    /// Corresponds to Java: `private final String m_html;`
    m_html: Option<String>,
}

impl HtmlNotes {
/// Constructor.
    ///
    /// Corresponds to Java: `public HtmlNotes(String html)`
    ///
/// Java's `String` reference may be `null`, so the parameter uses `Option`.
    pub fn new(html: Option<String>) -> HtmlNotes {
        let m_text = HtmlHelper::strip(html.as_deref());

        HtmlNotes { m_text, m_html: html }
    }

/// Return the original HTML document.
    ///
    /// Corresponds to Java: `public String getHtml()`
    pub fn get_html(&self) -> Option<&str> {
        self.m_html.as_deref()
    }
}

impl Display for HtmlNotes {
/// Return the plain text with HTML markup stripped.
    ///
/// Corresponds to Java: inherited from `Notes.toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.m_text {
            Some(text) => f.write_str(text),
            None => Ok(()),
        }
    }
}

impl Notes for HtmlNotes {
/// Corresponds to Java: none (Rust-specific subtype-preserving copy, see [`Notes::to_box`])
    fn to_box(&self) -> Box<dyn Notes> {
        Box::new(HtmlNotes {
            m_text: self.m_text.clone(),
            m_html: self.m_html.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_html_is_preserved() {
        let notes = HtmlNotes::new(Some(String::from("plain text")));
        assert_eq!(notes.get_html(), Some("plain text"));
    }

    #[test]
    fn display_uses_stripped_text() {
        let html = String::from("plain text");
        let notes = HtmlNotes::new(Some(html.clone()));
        assert_eq!(
            notes.to_string(),
            HtmlHelper::strip(Some(html.as_str())).unwrap_or_default()
        );
        assert!(!notes.is_empty());
    }

    #[test]
    fn null_html_is_empty() {
        let notes = HtmlNotes::new(None);
        assert_eq!(notes.get_html(), None);
        assert!(notes.is_empty());
    }
}
