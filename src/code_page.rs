//! Corresponds to Java: `org/mpxj/CodePage.java`

use std::fmt::{self, Display, Formatter};

/// Code pages used by MPX files.
///
/// Corresponds to Java: `public enum CodePage`
///
/// In Java each constant carries two fields `(String value, Charset charset)`, and a static block writes
/// `value` → enum into `NAME_MAP`. On the Rust side:
/// * `value` (the code page name in the MPX file) is returned by [`CodePage::get_value`] / [`Display`];
/// * `charset` is returned by [`CodePage::get_charset`] (`encoding_rs::Encoding`);
/// * The mapping is now done by the `match` in [`CodePage::get_instance`], so `NAME_MAP` is no longer needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodePage {
    /// Corresponds to Java: `ANSI("ANSI", CharsetHelper.CP1252)`
    Ansi,
    /// Corresponds to Java: `MAC("MAC", CharsetHelper.MAC_ROMAN)`
    Mac,
    /// Corresponds to Java: `LATIN("850", CharsetHelper.CP850)`
    Latin,
    /// Corresponds to Java: `US("437", CharsetHelper.CP437)`
    Us,
    /// Corresponds to Java: `ZH("ZH", CharsetHelper.GB2312)`
    Zh,
    /// Corresponds to Java: `RU("RU", CharsetHelper.CP1251)`
    Ru,
}

impl CodePage {
/// All values, in the same order as the Java declarations.
    ///
/// Corresponds to Java: all constants iterated by `EnumSet.allOf(CodePage.class)`.
    pub const ALL: [CodePage; 6] = [
        CodePage::Ansi,
        CodePage::Mac,
        CodePage::Latin,
        CodePage::Us,
        CodePage::Zh,
        CodePage::Ru,
    ];

/// Get an enum instance by MPX code page name, falling back to `ANSI` when unrecognized.
    ///
/// Corresponds to Java: `public static CodePage getInstance(String value)`, i.e.
    /// `NAME_MAP.getOrDefault(value, ANSI)`.
    pub fn get_instance(value: &str) -> CodePage {
        match value {
            "ANSI" => CodePage::Ansi,
            "MAC" => CodePage::Mac,
            "850" => CodePage::Latin,
            "437" => CodePage::Us,
            "ZH" => CodePage::Zh,
            "RU" => CodePage::Ru,
            _ => CodePage::Ansi,
        }
    }

/// Get an enum instance by MPX code page name; `None` falls back to `ANSI`.
    ///
/// Corresponds to Java: the behavior of `NAME_MAP.getOrDefault(null, ANSI)` (`HashMap` allows null keys,
/// returning `ANSI` on a miss).
    pub fn get_instance_optional(value: Option<&str>) -> CodePage {
        match value {
            Some(value) => CodePage::get_instance(value),
            None => CodePage::Ansi,
        }
    }

/// Return the Java charset name corresponding to this code page (the argument to `Charset.forName` in `CharsetHelper`).
    ///
/// Corresponds to Java: the charset identifier corresponding to `private final Charset m_charset`.
    pub fn get_charset_name(&self) -> &'static str {
        match self {
            CodePage::Ansi => "Cp1252",
            CodePage::Mac => "MacRoman",
            CodePage::Latin => "Cp850",
            CodePage::Us => "Cp437",
            CodePage::Zh => "GB2312",
            CodePage::Ru => "Cp1251",
        }
    }

/// Return the charset corresponding to this code page.
    ///
    /// Corresponds to Java: `public Charset getCharset()`
    ///
/// `encoding_rs` implements only the encodings in the WHATWG Encoding standard, which does not include CP437 and CP850,
/// so these two code pages return `None` (their charset names can still be obtained via [`CodePage::get_charset_name`]
/// ). `GB2312` is covered by `GBK` in the WHATWG standard, so it maps to [`encoding_rs::GBK`].
    pub fn get_charset(&self) -> Option<&'static encoding_rs::Encoding> {
        match self {
            CodePage::Ansi => Some(encoding_rs::WINDOWS_1252),
            CodePage::Mac => Some(encoding_rs::MACINTOSH),
// encoding_rs has no CP850 implementation
            CodePage::Latin => None,
// encoding_rs has no CP437 implementation
            CodePage::Us => None,
            CodePage::Zh => Some(encoding_rs::GBK),
            CodePage::Ru => Some(encoding_rs::WINDOWS_1251),
        }
    }

/// Return the code page name in the MPX file.
    ///
    /// Corresponds to Java: `private final String m_value`.
    pub fn get_value(&self) -> &'static str {
        match self {
            CodePage::Ansi => "ANSI",
            CodePage::Mac => "MAC",
            CodePage::Latin => "850",
            CodePage::Us => "437",
            CodePage::Zh => "ZH",
            CodePage::Ru => "RU",
        }
    }
}

impl Display for CodePage {
/// Corresponds to Java: `@Override public String toString()`, returning `m_value`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_page_round_trips() {
        for item in CodePage::ALL {
            assert_eq!(CodePage::get_instance(item.get_value()), item);
            assert_eq!(CodePage::get_instance(&item.to_string()), item);
        }
    }

    #[test]
    fn code_page_falls_back_to_ansi() {
        assert_eq!(CodePage::get_instance("unknown"), CodePage::Ansi);
        assert_eq!(CodePage::get_instance(""), CodePage::Ansi);
        assert_eq!(CodePage::get_instance_optional(None), CodePage::Ansi);
    }

    #[test]
    fn code_page_charsets() {
        assert_eq!(CodePage::Ansi.get_charset(), Some(encoding_rs::WINDOWS_1252));
        assert_eq!(CodePage::Mac.get_charset(), Some(encoding_rs::MACINTOSH));
        assert_eq!(CodePage::Zh.get_charset(), Some(encoding_rs::GBK));
        assert_eq!(CodePage::Ru.get_charset(), Some(encoding_rs::WINDOWS_1251));
// encoding_rs does not implement CP437 / CP850
        assert_eq!(CodePage::Us.get_charset(), None);
        assert_eq!(CodePage::Latin.get_charset(), None);
        assert_eq!(CodePage::Latin.get_charset_name(), "Cp850");
    }
}
