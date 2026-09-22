//! Corresponds to Java: `org/mpxj/mpp/FontBase.java`
//!
//! MS Project's standard font definition is split into two parts: the base font (name + size) and the styles applied to it
//! (color / bold / italic ...). This class represents the **base font** information.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `public class FontBase` | struct [`FontBase`] | 1:1 port |
//! | `FontBase(Integer index, String name, int size)` | [`FontBase::new`] (`index: Option<i32>`, `name: String`, `size: i32`) | Java's `Integer` may be `null` → `Option<i32>`; `name` is passed a non-null string at all three construction sites in this repo (`MPP9Reader` / `MPP12Reader` / `MPP14Reader` font table parsing), so the Rust side treats it as a non-nullable `String` (the Java side could theoretically pass `null`, but no call site does) |
//! | `String getName()` | `&str` | Borrowed return, semantics identical to Java |
//! | `Integer getIndex()` | `Option<i32>` | The "`null` → `Option<T>`" convention; on the Java side it is often used as a `Map` key (`m_fontBases.put(fontBase.getIndex(), fontBase)`) |
//! | `toString()` | `Display` | Output format is exactly the same as Java: `[FontBase name=... size=...]` |
//!
//! Not ported / skipped: none (this class does not use mechanisms such as JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`).

use std::fmt::{self, Display, Formatter};

/// Base font (name + size).
///
/// Corresponds to Java: `public class FontBase`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontBase {
/// The index of this font in the file's font table.
    ///
    /// Corresponds to Java: `private final Integer m_index`
    index: Option<i32>,
/// Font name.
    ///
    /// Corresponds to Java: `private final String m_name`
    name: String,
/// Font size.
    ///
    /// Corresponds to Java: `private final int m_size`
    size: i32,
}

impl FontBase {
/// Constructor.
    ///
    /// Corresponds to Java: `public FontBase(Integer index, String name, int size)`
    pub fn new(index: Option<i32>, name: String, size: i32) -> FontBase {
        FontBase { index, name, size }
    }

/// Get the font name.
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> &str {
        &self.name
    }

/// Get the font size.
    ///
    /// Corresponds to Java: `public int getSize()`
    pub fn get_size(&self) -> i32 {
        self.size
    }

/// Get the index of this base font in the file.
    ///
    /// Corresponds to Java: `public Integer getIndex()`
    pub fn get_index(&self) -> Option<i32> {
        self.index
    }
}

impl Display for FontBase {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java outputs only the name and size (not the index).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[FontBase name={} size={}]", self.name, self.size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// The getter matches Java.
    #[test]
    fn accessors_match_java() {
        let font = FontBase::new(Some(3), "Arial".to_owned(), 12);
        assert_eq!(font.get_name(), "Arial");
        assert_eq!(font.get_size(), 12);
        assert_eq!(font.get_index(), Some(3));
    }

/// `index` may be `None` (Java's `Integer` may be null).
    #[test]
    fn index_may_be_missing() {
        let font = FontBase::new(None, "Arial".to_owned(), 12);
        assert_eq!(font.get_index(), None);
    }

/// The toString format matches Java.
    #[test]
    fn to_string_matches_java() {
        let font = FontBase::new(Some(0), "Arial".to_owned(), 10);
        assert_eq!(font.to_string(), "[FontBase name=Arial size=10]");
    }

/// An empty name does not panic (Java's `getUnicodeString` never returns null, but may return an empty string).
    #[test]
    fn empty_name_is_allowed() {
        let font = FontBase::new(None, String::new(), 0);
        assert_eq!(font.get_name(), "");
        assert_eq!(font.to_string(), "[FontBase name= size=0]");
    }
}
