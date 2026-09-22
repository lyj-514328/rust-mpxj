//! Corresponds to Java: `org/mpxj/common/CharsetHelper.java`
//!
//! Common charset constants. In Java each constant is a `java.nio.charset.Charset`; the Rust
//! side uniformly uses `encoding_rs::Encoding` (consistent with [`crate::CodePage::get_charset`]).
//!
//! Porting trade-offs:
//! - Java's constants can never be null (`Charset.forName` throws for unknown names); Rust's
//!   `encoding_rs` only implements the encodings in the WHATWG Encoding standard, lacks CP437
//!   and CP850, and has no BOM-aware UTF-16, so the constant type is
//!   `Option<&'static Encoding>`: constants that cannot be mapped take `None` (their Java names
//!   can still be recognized via [`CharsetHelper::get_charset`]).
//! - Java's `StandardCharsets.UTF_16` determines byte order from the BOM (BE when there is no
//!   BOM); `encoding_rs` has no equivalent encoding, so `UTF16` returns `None`, and callers
//!   probe the BOM and then use `UTF16LE` or `encoding_rs::UTF_16BE`.
//! - `GB2312` is covered by `GBK` in the WHATWG standard, so it maps to [`encoding_rs::GBK`]
//!   (consistent with `CodePage::get_charset`).
//! - Java's `Charset.forName(String)` is handled on the Rust side by
//!   [`CharsetHelper::get_charset`], returning `None` (never throwing) when it cannot recognize
//!   or map a name.
//!
//! Unported methods: none.

use encoding_rs::Encoding;

/// Common charset constants.
///
/// Corresponds to Java: `public class CharsetHelper`
pub struct CharsetHelper;

impl CharsetHelper {
    /// Corresponds to Java: `public static final Charset UTF8 = StandardCharsets.UTF_8`
    pub const UTF8: Option<&'static Encoding> = Some(encoding_rs::UTF_8);

    /// Corresponds to Java: `public static final Charset UTF16 = StandardCharsets.UTF_16`
    ///
    /// Java determines byte order from the BOM; `encoding_rs` has no equivalent encoding, so
    /// this is `None`.
    pub const UTF16: Option<&'static Encoding> = None;

    /// Corresponds to Java: `public static final Charset UTF16LE = StandardCharsets.UTF_16LE`
    pub const UTF16LE: Option<&'static Encoding> = Some(encoding_rs::UTF_16LE);

    /// Corresponds to Java: `public static final Charset CP1252 = Charset.forName("Cp1252")`
    pub const CP1252: Option<&'static Encoding> = Some(encoding_rs::WINDOWS_1252);

    /// Corresponds to Java: `public static final Charset MAC_ROMAN = Charset.forName("MacRoman")`
    pub const MAC_ROMAN: Option<&'static Encoding> = Some(encoding_rs::MACINTOSH);

    /// Corresponds to Java: `public static final Charset CP850 = Charset.forName("Cp850")`
    ///
    /// `encoding_rs` does not implement CP850, so this is `None`.
    pub const CP850: Option<&'static Encoding> = None;

    /// Corresponds to Java: `public static final Charset CP437 = Charset.forName("Cp437")`
    ///
    /// `encoding_rs` does not implement CP437, so this is `None`.
    pub const CP437: Option<&'static Encoding> = None;

    /// Corresponds to Java: `public static final Charset GB2312 = Charset.forName("GB2312")`
    pub const GB2312: Option<&'static Encoding> = Some(encoding_rs::GBK);

    /// Corresponds to Java: `public static final Charset CP1251 = Charset.forName("Cp1251")`
    pub const CP1251: Option<&'static Encoding> = Some(encoding_rs::WINDOWS_1251);

    /// Get a charset by name, case-insensitively; returns `None` when it cannot be recognized
    /// or mapped.
    ///
    /// Corresponds to Java: `Charset.forName(String)` (Java throws
    /// `UnsupportedCharsetException` for unknown names; the Rust side returns `None` instead).
    ///
    /// Names follow the spelling used in Java / MPX files (e.g. `Cp1252`, `GB2312`, see
    /// [`crate::CodePage::get_charset_name`]). `Cp437`, `Cp850` and BOM-aware `UTF-16`
    /// have no implementation in `encoding_rs` and also return `None`.
    pub fn get_charset(name: &str) -> Option<&'static Encoding> {
        match name.to_ascii_lowercase().as_str() {
            "utf-8" | "utf8" => Some(encoding_rs::UTF_8),
            "utf-16le" | "utf16le" => Some(encoding_rs::UTF_16LE),
            "cp1252" | "windows-1252" => Some(encoding_rs::WINDOWS_1252),
            "macroman" | "macintosh" => Some(encoding_rs::MACINTOSH),
            "gb2312" | "gbk" => Some(encoding_rs::GBK),
            "cp1251" | "windows-1251" => Some(encoding_rs::WINDOWS_1251),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CodePage;

    #[test]
    fn constants_match_java_declarations() {
        assert_eq!(CharsetHelper::UTF8, Some(encoding_rs::UTF_8));
        assert_eq!(CharsetHelper::UTF16LE, Some(encoding_rs::UTF_16LE));
        assert_eq!(CharsetHelper::CP1252, Some(encoding_rs::WINDOWS_1252));
        assert_eq!(CharsetHelper::MAC_ROMAN, Some(encoding_rs::MACINTOSH));
        assert_eq!(CharsetHelper::GB2312, Some(encoding_rs::GBK));
        assert_eq!(CharsetHelper::CP1251, Some(encoding_rs::WINDOWS_1251));
    }

    #[test]
    fn encodings_missing_from_encoding_rs_are_none() {
        // CP437 / CP850 are not implemented in encoding_rs
        assert_eq!(CharsetHelper::CP437, None);
        assert_eq!(CharsetHelper::CP850, None);
        // Java's UTF_16 determines byte order from the BOM; encoding_rs has no equivalent encoding
        assert_eq!(CharsetHelper::UTF16, None);
    }

    #[test]
    fn get_charset_resolves_java_names() {
        assert_eq!(CharsetHelper::get_charset("Cp1252"), Some(encoding_rs::WINDOWS_1252));
        assert_eq!(CharsetHelper::get_charset("cp1252"), Some(encoding_rs::WINDOWS_1252));
        assert_eq!(CharsetHelper::get_charset("MacRoman"), Some(encoding_rs::MACINTOSH));
        assert_eq!(CharsetHelper::get_charset("GB2312"), Some(encoding_rs::GBK));
        assert_eq!(CharsetHelper::get_charset("Cp1251"), Some(encoding_rs::WINDOWS_1251));
        assert_eq!(CharsetHelper::get_charset("UTF-16LE"), Some(encoding_rs::UTF_16LE));
        assert_eq!(CharsetHelper::get_charset("UTF-8"), Some(encoding_rs::UTF_8));
    }

    #[test]
    fn get_charset_returns_none_for_unmappable_names() {
        assert_eq!(CharsetHelper::get_charset("Cp437"), None);
        assert_eq!(CharsetHelper::get_charset("Cp850"), None);
        assert_eq!(CharsetHelper::get_charset("UTF-16"), None);
        assert_eq!(CharsetHelper::get_charset("no-such-charset"), None);
    }

    #[test]
    fn get_charset_matches_code_page_charsets() {
        // Names produced by `CodePage::get_charset_name` should round-trip to the same charset via `CharsetHelper`
        for page in CodePage::ALL {
            assert_eq!(
                CharsetHelper::get_charset(page.get_charset_name()),
                page.get_charset(),
                "charset mapping for code page {:?} is inconsistent",
                page
            );
        }
    }

    #[test]
    fn constants_can_decode_bytes() {
        let encoding = CharsetHelper::CP1252.expect("CP1252 has implementation");
        let (decoded, _, _) = encoding.decode(&[0xE9]);
        assert_eq!(decoded, "é");

        let encoding = CharsetHelper::GB2312.expect("GB2312 has implementation");
        let (decoded, _, _) = encoding.decode(&[0xD6, 0xD0]);
        assert_eq!(decoded, "中");
    }
}
