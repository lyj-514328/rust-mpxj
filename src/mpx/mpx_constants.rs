//! Corresponds to Java: `org/mpxj/mpx/MPXConstants.java`
//!
//! Constants used when reading and writing MPX (Microsoft Project Exchange) text files:
//! the end-of-line sequence, record numbers for the various record types, and the placeholder character representing a newline in notes text.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `public static final String EOL` | [`MPXConstants::EOL`] | Java's `String` -> `&str` |
//! | `public static final int XXX_RECORD_NUMBER` | [`MPXConstants::XXX_RECORD_NUMBER`] | Java's `int` -> `i32` |
//! | `static final char EOL_PLACEHOLDER` | [`MPXConstants::EOL_PLACEHOLDER`] | Java's `char` -> `char`, with the value `'\u{7F}'` |
//! | `static final String EOL_PLACEHOLDER_STRING` | [`MPXConstants::EOL_PLACEHOLDER_STRING`] | Java constructs it from a single-byte array; the Rust side is directly the `&str` of the same character |
//! | `private MPXConstants()` | none | Java uses a private constructor to prevent instantiation; the Rust side keeps an empty struct as a namespace (same style as `NumberHelper`); instantiating it is meaningless but does not affect semantics |
//!
//! Several trade-offs:
//!
//! - Java's `MPXConstants` is package-visible (`final class MPXConstants`), and among the constants
//!   `EOL_PLACEHOLDER` / `EOL_PLACEHOLDER_STRING` are package-private; the Rust module path
//!   `crate::mpx::mpx_constants` already limits visibility (`mpx` is not `pub use`d in `src/lib.rs`),
//!   so the constants are all declared `pub`, without distinguishing with `pub(crate)`.
//! - Java expresses the same 0x7F character in two forms: `char` and a "String constructed from a single byte";
//!   on the Rust side both `char` and `&str` are UTF-8 representations, and `EOL_PLACEHOLDER_STRING` is
//!   the string form of `EOL_PLACEHOLDER`, so their contents match.
//!
//! Reflection / JDBC / XML (JAXB) / GUI / `javax.*` / `Serializable`: not used by this class, nothing skipped.
//! Skipped Java methods: none (the only skipped private constructor is shown in the table above).

/// Collection of constants used for reading and writing MPX files.
///
/// Corresponds to Java: `final class MPXConstants`
pub struct MPXConstants;

impl MPXConstants {
    /// End-of-line sequence used in MPX files.
    ///
    /// Corresponds to Java: `public static final String EOL`
    pub const EOL: &'static str = "\r\n";

    /// Record number for comments records.
    ///
    /// Corresponds to Java: `public static final int COMMENTS_RECORD_NUMBER`
    pub const COMMENTS_RECORD_NUMBER: i32 = 0;

    /// Record number for currency settings records.
    ///
    /// Corresponds to Java: `public static final int CURRENCY_SETTINGS_RECORD_NUMBER`
    pub const CURRENCY_SETTINGS_RECORD_NUMBER: i32 = 10;

    /// Record number for default settings records.
    ///
    /// Corresponds to Java: `public static final int DEFAULT_SETTINGS_RECORD_NUMBER`
    pub const DEFAULT_SETTINGS_RECORD_NUMBER: i32 = 11;

    /// Record number for date/time settings records.
    ///
    /// Corresponds to Java: `public static final int DATE_TIME_SETTINGS_RECORD_NUMBER`
    pub const DATE_TIME_SETTINGS_RECORD_NUMBER: i32 = 12;

    /// Record number for base calendar records.
    ///
    /// Corresponds to Java: `public static final int BASE_CALENDAR_RECORD_NUMBER`
    pub const BASE_CALENDAR_RECORD_NUMBER: i32 = 20;

    /// Record number for base calendar hours records.
    ///
    /// Corresponds to Java: `public static final int BASE_CALENDAR_HOURS_RECORD_NUMBER`
    pub const BASE_CALENDAR_HOURS_RECORD_NUMBER: i32 = 25;

    /// Record number for base calendar exception (non-working day) records.
    ///
    /// Corresponds to Java: `public static final int BASE_CALENDAR_EXCEPTION_RECORD_NUMBER`
    pub const BASE_CALENDAR_EXCEPTION_RECORD_NUMBER: i32 = 26;

    /// Record number for project header (project properties) records.
    ///
    /// Corresponds to Java: `public static final int PROJECT_HEADER_RECORD_NUMBER`
    pub const PROJECT_HEADER_RECORD_NUMBER: i32 = 30;

    /// Record number for resource calendar records.
    ///
    /// Corresponds to Java: `public static final int RESOURCE_CALENDAR_RECORD_NUMBER`
    pub const RESOURCE_CALENDAR_RECORD_NUMBER: i32 = 55;

    /// Record number for resource calendar hours records.
    ///
    /// Corresponds to Java: `public static final int RESOURCE_CALENDAR_HOURS_RECORD_NUMBER`
    pub const RESOURCE_CALENDAR_HOURS_RECORD_NUMBER: i32 = 56;

    /// Record number for resource calendar exception (non-working day) records.
    ///
    /// Corresponds to Java: `public static final int RESOURCE_CALENDAR_EXCEPTION_RECORD_NUMBER`
    pub const RESOURCE_CALENDAR_EXCEPTION_RECORD_NUMBER: i32 = 57;

    /// Record number for text resource field model records.
    ///
    /// Corresponds to Java: `public static final int RESOURCE_MODEL_TEXT_RECORD_NUMBER`
    pub const RESOURCE_MODEL_TEXT_RECORD_NUMBER: i32 = 40;

    /// Record number for numeric resource field model records.
    ///
    /// Corresponds to Java: `public static final int RESOURCE_MODEL_NUMERIC_RECORD_NUMBER`
    pub const RESOURCE_MODEL_NUMERIC_RECORD_NUMBER: i32 = 41;

    /// Record number for resource records.
    ///
    /// Corresponds to Java: `public static final int RESOURCE_RECORD_NUMBER`
    pub const RESOURCE_RECORD_NUMBER: i32 = 50;

    /// Record number for resource notes records.
    ///
    /// Corresponds to Java: `public static final int RESOURCE_NOTES_RECORD_NUMBER`
    pub const RESOURCE_NOTES_RECORD_NUMBER: i32 = 51;

    /// Record number for text task field model records.
    ///
    /// Corresponds to Java: `public static final int TASK_MODEL_TEXT_RECORD_NUMBER`
    pub const TASK_MODEL_TEXT_RECORD_NUMBER: i32 = 60;

    /// Record number for numeric task field model records.
    ///
    /// Corresponds to Java: `public static final int TASK_MODEL_NUMERIC_RECORD_NUMBER`
    pub const TASK_MODEL_NUMERIC_RECORD_NUMBER: i32 = 61;

    /// Record number for task records.
    ///
    /// Corresponds to Java: `public static final int TASK_RECORD_NUMBER`
    pub const TASK_RECORD_NUMBER: i32 = 70;

    /// Record number for task notes records.
    ///
    /// Corresponds to Java: `public static final int TASK_NOTES_RECORD_NUMBER`
    pub const TASK_NOTES_RECORD_NUMBER: i32 = 71;

    /// Record number for recurring task records.
    ///
    /// Corresponds to Java: `public static final int RECURRING_TASK_RECORD_NUMBER`
    pub const RECURRING_TASK_RECORD_NUMBER: i32 = 72;

    /// Record number for resource assignment records.
    ///
    /// Corresponds to Java: `public static final int RESOURCE_ASSIGNMENT_RECORD_NUMBER`
    pub const RESOURCE_ASSIGNMENT_RECORD_NUMBER: i32 = 75;

    /// Record number for resource assignment workgroup fields records.
    ///
    /// Corresponds to Java: `public static final int RESOURCE_ASSIGNMENT_WORKGROUP_FIELDS_RECORD_NUMBER`
    pub const RESOURCE_ASSIGNMENT_WORKGROUP_FIELDS_RECORD_NUMBER: i32 = 76;

    /// Record number for project names records.
    ///
    /// Corresponds to Java: `public static final int PROJECT_NAMES_RECORD_NUMBER`
    pub const PROJECT_NAMES_RECORD_NUMBER: i32 = 80;

    /// Record number for DDE/OLE client links records.
    ///
    /// Corresponds to Java: `public static final int DDE_OLE_CLIENT_LINKS_RECORD_NUMBER`
    pub const DDE_OLE_CLIENT_LINKS_RECORD_NUMBER: i32 = 81;

    /// Record number for file creation records.
    ///
    /// Corresponds to Java: `public static final int FILE_CREATION_RECORD_NUMBER`
    ///
    /// Note: this value is a "placeholder value"; the text actually written to the file is `MPX`.
    /// A numeric placeholder is used so that all record types can be uniformly identified by a number.
    pub const FILE_CREATION_RECORD_NUMBER: i32 = 999;

    /// Placeholder character used in MPX files to represent an embedded carriage return/line feed within notes text.
    ///
    /// Corresponds to Java: `static final char EOL_PLACEHOLDER`
    pub const EOL_PLACEHOLDER: char = '\u{7F}';

    /// String form of [`MPXConstants::EOL_PLACEHOLDER`].
    ///
    /// Corresponds to Java: `static final String EOL_PLACEHOLDER_STRING`
    pub const EOL_PLACEHOLDER_STRING: &'static str = "\u{7F}";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eol_is_crlf() {
        assert_eq!(MPXConstants::EOL, "\r\n");
        assert_eq!(MPXConstants::EOL.chars().count(), 2);
    }

    #[test]
    fn eol_placeholder_matches_java_char() {
        assert_eq!(MPXConstants::EOL_PLACEHOLDER as u32, 0x7F);
        assert_eq!(MPXConstants::EOL_PLACEHOLDER_STRING, "\u{7F}");
        assert_eq!(MPXConstants::EOL_PLACEHOLDER_STRING.len(), 1);
    }

    /// Verify the record number values in Java's `MPXConstants` one by one.
    #[test]
    fn record_numbers_match_java() {
        assert_eq!(MPXConstants::COMMENTS_RECORD_NUMBER, 0);
        assert_eq!(MPXConstants::CURRENCY_SETTINGS_RECORD_NUMBER, 10);
        assert_eq!(MPXConstants::DEFAULT_SETTINGS_RECORD_NUMBER, 11);
        assert_eq!(MPXConstants::DATE_TIME_SETTINGS_RECORD_NUMBER, 12);
        assert_eq!(MPXConstants::BASE_CALENDAR_RECORD_NUMBER, 20);
        assert_eq!(MPXConstants::BASE_CALENDAR_HOURS_RECORD_NUMBER, 25);
        assert_eq!(MPXConstants::BASE_CALENDAR_EXCEPTION_RECORD_NUMBER, 26);
        assert_eq!(MPXConstants::PROJECT_HEADER_RECORD_NUMBER, 30);
        assert_eq!(MPXConstants::RESOURCE_MODEL_TEXT_RECORD_NUMBER, 40);
        assert_eq!(MPXConstants::RESOURCE_MODEL_NUMERIC_RECORD_NUMBER, 41);
        assert_eq!(MPXConstants::RESOURCE_RECORD_NUMBER, 50);
        assert_eq!(MPXConstants::RESOURCE_NOTES_RECORD_NUMBER, 51);
        assert_eq!(MPXConstants::RESOURCE_CALENDAR_RECORD_NUMBER, 55);
        assert_eq!(MPXConstants::RESOURCE_CALENDAR_HOURS_RECORD_NUMBER, 56);
        assert_eq!(MPXConstants::RESOURCE_CALENDAR_EXCEPTION_RECORD_NUMBER, 57);
        assert_eq!(MPXConstants::TASK_MODEL_TEXT_RECORD_NUMBER, 60);
        assert_eq!(MPXConstants::TASK_MODEL_NUMERIC_RECORD_NUMBER, 61);
        assert_eq!(MPXConstants::TASK_RECORD_NUMBER, 70);
        assert_eq!(MPXConstants::TASK_NOTES_RECORD_NUMBER, 71);
        assert_eq!(MPXConstants::RECURRING_TASK_RECORD_NUMBER, 72);
        assert_eq!(MPXConstants::RESOURCE_ASSIGNMENT_RECORD_NUMBER, 75);
        assert_eq!(
            MPXConstants::RESOURCE_ASSIGNMENT_WORKGROUP_FIELDS_RECORD_NUMBER,
            76
        );
        assert_eq!(MPXConstants::PROJECT_NAMES_RECORD_NUMBER, 80);
        assert_eq!(MPXConstants::DDE_OLE_CLIENT_LINKS_RECORD_NUMBER, 81);
        assert_eq!(MPXConstants::FILE_CREATION_RECORD_NUMBER, 999);
    }
}
