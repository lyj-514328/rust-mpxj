//! Corresponds to Java: `org/mpxj/mpp/CompObj.java`
//!
//! Reads the `\1CompObj` stream in an OLE compound document. This is OLE's "compound document information" block,
//! the part of which we can interpret gives the **application name, version and file format identifier of the application that created the document**,
//! and `MPPReader` selects the matching variant reader precisely by the string returned by [`CompObj::get_file_format`] (for example
//! `Microsoft.Project.9`).
//!
//! Stream layout (corresponds word-for-word to the Java constructor, MPP little-endian):
//!
//! ```text
//! byte[28]  skipped (InputStreamHelper.skip(is, 28))
//! int       applicationName length (including the trailing 0 byte)
//! byte[]    applicationName (UTF-8/ASCII, with the last byte being 0)
//! --- present only when applicationName != "Microsoft Project 4.0" ---
//! int       fileFormat length (including the trailing 0 byte; <= 0 means it ends here)
//! byte[]    fileFormat (last byte is 0)
//! int       applicationID length (including the trailing 0 byte; <= 0 means absent)
//! byte[]    applicationID (last byte is 0)
//! ```
//!
//! # Inheritance emulation
//!
//! Java: `final class CompObj extends MPPComponent`. Rust has no inheritance; `MPPComponent` has, per the project
//! convention, degenerated into a standalone utility type (see [`crate::mpp::mpp_component`]); this class directly calls its
//! [`MPPComponent::read_int`] / [`MPPComponent::read_byte_array`] to read in cursor order.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `CompObj(InputStream is) throws IOException` | `CompObj::new(&[u8]) -> MpxjResult<CompObj>` | Per convention `InputStream` → `&[u8]`; failing to read the required bytes (Java throws `EOFException`) → `MpxjError` (message reuses the original text from `InputStreamHelper.read`) |
//! | When `length <= 0`, `new String(bytes, 0, length - 1)` throws `StringIndexOutOfBoundsException` / `NegativeArraySizeException` | Likewise returns `Err` | Follows the "library code must not panic" convention |
//! | `new String(byte[])` uses the platform default charset | Forgiving UTF-8 decoding (the same handling as `fix_defer_fix.rs`) | The strings in CompObj are in fact all ASCII |
//! | `Pattern.compile("Microsoft.Project.(\\d+)\\.0")` + `Matcher.matches()` | The private function [`parse_application_version`] implements this pattern by hand | Convention: no new dependencies, no regex crate; when the number exceeds `int`, Java throws `NumberFormatException` while Rust returns `None` |
//! | `getApplicationVersion()` returns `Integer` (nullable) | `Option<i32>` | —— |
//! | `getApplicationID()` / `getFileFormat()` return `String` (nullable) | `Option<&str>` | —— |
//! | `toString()` | `Display` | In the Java source the string **is missing the closing `]`**, which is preserved 1:1 here; nullable fields are rendered as `null` following the semantics of Java string concatenation |
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: not used by this class.
//!
//! Skipped Java members: none.

use std::fmt::{self, Display, Formatter};

use crate::mpp::mpp_component::MPPComponent;
use crate::mpp::props::read_error;
use crate::mpxj_exception::MpxjResult;

/// The file format identifier for `Microsoft Project 4.0` is hard-coded (a special-case branch in the Java constructor).
const APPLICATION_NAME_MPP4: &str = "Microsoft Project 4.0";
/// Corresponds to Java: the constant assigned to `m_fileFormat` in the special-case branch.
const FILE_FORMAT_MPP4: &str = "MSProject.MPP4";
/// Corresponds to Java: the constant assigned to `m_applicationID` in the special-case branch.
const APPLICATION_ID_MPP4: &str = "MSProject.Project.4";

/// Corresponds to Java field `private static final Pattern PATTERN`: the pattern prefix.
const PATTERN_APPLICATION: &str = "Microsoft";
/// Corresponds to Java field `private static final Pattern PATTERN`: the pattern body.
const PATTERN_PROJECT: &str = "Project";

/// The `\1CompObj` stream of an OLE compound document.
///
/// Corresponds to Java: `final class CompObj extends MPPComponent`
///
/// The Java side has no `equals` / `hashCode` (i.e. it compares by reference), and the Rust side likewise does not implement
/// `PartialEq` / `Eq`; `Debug` exists only to aid debug output.
#[derive(Debug)]
pub struct CompObj {
    /// Corresponds to Java: `private final String m_applicationName`
    m_application_name: String,
    /// Corresponds to Java: `private Integer m_applicationVersion`
    m_application_version: Option<i32>,
    /// Corresponds to Java: `private String m_applicationID`
    m_application_id: Option<String>,
    /// Corresponds to Java: `private String m_fileFormat`
    m_file_format: Option<String>,
}

impl CompObj {
/// Constructor: read and parse the entire `\1CompObj` stream.
    ///
    /// Corresponds to Java: `CompObj(InputStream is) throws IOException`
    pub fn new(data: &[u8]) -> MpxjResult<Self> {
        let mut position = 0usize;

        // Java: InputStreamHelper.skip(is, 28);
// (Java throws IOException when the skip fails; the Rust side seeks directly, and subsequent reads return Err)
        position = (position + 28).min(data.len());

        // Java: length = readInt(is); m_applicationName = new String(readByteArray(is, length), 0, length - 1);
        let length = MPPComponent::read_int(data, &mut position).ok_or_else(read_error)?;
        let m_application_name = read_string(data, &mut position, length)?;

        // Java: Matcher matcher = PATTERN.matcher(m_applicationName); if (matcher.matches()) { m_applicationVersion = Integer.valueOf(matcher.group(1)); }
        let m_application_version = parse_application_version(&m_application_name);

        let mut m_file_format = None;
        let mut m_application_id = None;

        if m_application_name == APPLICATION_NAME_MPP4 {
            // Java: m_fileFormat = "MSProject.MPP4"; m_applicationID = "MSProject.Project.4";
            m_file_format = Some(FILE_FORMAT_MPP4.to_owned());
            m_application_id = Some(APPLICATION_ID_MPP4.to_owned());
        } else {
            // Java: length = readInt(is); if (length > 0) { ... }
            let length = MPPComponent::read_int(data, &mut position).ok_or_else(read_error)?;
            if length > 0 {
                m_file_format = Some(read_string(data, &mut position, length)?);

                // Java: length = readInt(is); if (length > 0) { m_applicationID = ... }
                let length = MPPComponent::read_int(data, &mut position).ok_or_else(read_error)?;
                if length > 0 {
                    m_application_id = Some(read_string(data, &mut position, length)?);
                }
            }
        }

        Ok(Self {
            m_application_name,
            m_application_version,
            m_application_id,
            m_file_format,
        })
    }

/// Get the application name that created the document.
    ///
    /// Corresponds to Java: `public String getApplicationName()`
    pub fn get_application_name(&self) -> &str {
        &self.m_application_name
    }

/// Get the application version (parsed from the application name using `Microsoft.Project.(\d+).0`).
    ///
    /// Corresponds to Java: `public Integer getApplicationVersion()`
    pub fn get_application_version(&self) -> Option<i32> {
        self.m_application_version
    }

/// Get the application ID.
    ///
    /// Corresponds to Java: `public String getApplicationID()`
    pub fn get_application_id(&self) -> Option<&str> {
        self.m_application_id.as_deref()
    }

/// Get the file format identifier (`MPPReader` uses it to select the variant reader).
    ///
    /// Corresponds to Java: `public String getFileFormat()`
    pub fn get_file_format(&self) -> Option<&str> {
        self.m_file_format.as_deref()
    }
}

impl Display for CompObj {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// The Java source is `"[CompObj applicationName=" + ... + " fileFormat=" + m_fileFormat`,
/// with no `]` at the end (this detail is preserved 1:1).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[CompObj applicationName={} applicationID={} fileFormat={}",
            self.m_application_name,
            java_nullable(self.m_application_id.as_deref()),
            java_nullable(self.m_file_format.as_deref())
        )
    }
}

/// Read a string whose "length includes the trailing 0 byte".
///
/// Corresponds to Java: `new String(readByteArray(is, length), 0, length - 1)`
///
/// Java throws `StringIndexOutOfBoundsException` (`length == 0`) or
/// `NegativeArraySizeException` (`length < 0`) when `length <= 0`; the Rust side returns `Err` (no panic).
fn read_string(data: &[u8], position: &mut usize, length: i32) -> MpxjResult<String> {
    if length < 1 {
        return Err(read_error());
    }

    let bytes = MPPComponent::read_byte_array(data, position, length as usize)
        .ok_or_else(read_error)?;

// Java: new String(bytes, 0, length - 1) -- drop the trailing 0 byte; the platform default charset is decoded as forgiving UTF-8
    Ok(String::from_utf8_lossy(&bytes[..bytes.len() - 1]).into_owned())
}

/// Match the application name against `Microsoft.Project.(\d+)\.0` as a whole and extract the version number.
///
/// Corresponds to Java: `PATTERN` (`Pattern.compile("Microsoft.Project.(\\d+)\\.0")`) together with
/// `Matcher.matches()` (**whole-string** matching) and `Integer.valueOf(matcher.group(1))`.
///
/// The `.` in the pattern means any character in a Java regex; here too a match simply skips one character.
/// Return `Some` when the version number parses as an `int`, otherwise `None` (Java throws `NumberFormatException`).
fn parse_application_version(application_name: &str) -> Option<i32> {
// Process characters one by one with Java's char (UTF-16 code unit) semantics
    let chars: Vec<char> = application_name.chars().collect();
    let mut index = 0usize;

// "Microsoft" + any 1 character
    if !matches_at(&chars, index, PATTERN_APPLICATION) {
        return None;
    }
    index += PATTERN_APPLICATION.chars().count() + 1;

// "Project" + any 1 character
    if !matches_at(&chars, index, PATTERN_PROJECT) {
        return None;
    }
    index += PATTERN_PROJECT.chars().count() + 1;

// (\d+): at least 1 digit
    let digits_start = index;
    while index < chars.len() && chars[index].is_ascii_digit() {
        index += 1;
    }
    if index == digits_start {
        return None;
    }

// The ending must be exactly ".0"
    if index + 2 != chars.len() || chars[index] != '.' || chars[index + 1] != '0' {
        return None;
    }

    let digits: String = chars[digits_start..index].iter().collect();
    digits.parse::<i32>().ok()
}

/// Determine whether `chars[offset..]` starts with `expected`.
///
/// Java's `Matcher` treats insufficient characters as "no match" rather than throwing.
fn matches_at(chars: &[char], offset: usize, expected: &str) -> bool {
    let expected: Vec<char> = expected.chars().collect();
    match chars.get(offset..offset + expected.len()) {
        Some(slice) => slice == expected.as_slice(),
        None => false,
    }
}

/// Render a nullable string following the semantics of Java string concatenation: `null` → `"null"`.
fn java_nullable(value: Option<&str>) -> &str {
    value.unwrap_or("null")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    use cfb::CompoundFile;

    use crate::mpp::ole::PoifsFileSystem;

/// Write a `\1CompObj` stream into a compound document to simulate the real MPP file layout.
    ///
/// Corresponds to Java `MPPReader`: after `root.getEntryNames().contains("\1CompObj")`, retrieve that stream.
    fn compound_file_with_comp_obj(payload: &[u8]) -> Vec<u8> {
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        file.create_stream("/\u{1}CompObj")
            .unwrap()
            .write_all(payload)
            .unwrap();
        file.into_inner().into_inner()
    }

/// Assemble a stream's data according to the CompObj layout.
    fn comp_obj_payload(
        application_name: &str,
        file_format: Option<&str>,
        application_id: Option<&str>,
    ) -> Vec<u8> {
        let mut data = vec![0u8; 28];
        append_string(&mut data, application_name);

        if let Some(file_format) = file_format {
            append_string(&mut data, file_format);
        }

        if let Some(application_id) = application_id {
            append_string(&mut data, application_id);
        }

        data
    }

/// Append "length (including the trailing 0 byte) + content + 0 byte".
    fn append_string(data: &mut Vec<u8>, value: &str) {
        data.extend_from_slice(&((value.len() + 1) as i32).to_le_bytes());
        data.extend_from_slice(value.as_bytes());
        data.push(0);
    }

/// Read `\1CompObj` from a hand-built compound document and parse it.
    fn parse_from_compound_file(payload: &[u8]) -> CompObj {
        let bytes = compound_file_with_comp_obj(payload);
        let fs = PoifsFileSystem::new(Cursor::new(bytes)).unwrap();
        let root = fs.get_root();
        assert!(root.get_entry_names().contains(&"\u{1}CompObj".to_owned()));

        let document = root
            .get_document("\u{1}CompObj")
            .expect("should find \\1CompObj stream");

        // Corresponds to Java: new CompObj(new DocumentInputStream((DocumentEntry) root.getEntry("\1CompObj")))
        CompObj::new(&document.get_data()).expect("should parse CompObj")
    }

    #[test]
    fn parses_mpp14_file_format() {
        let comp_obj = parse_from_compound_file(&comp_obj_payload(
            "Microsoft Project 14.0",
            Some("MSProject.MPP14"),
            Some("MSProject.Project.9"),
        ));

        assert_eq!(comp_obj.get_application_name(), "Microsoft Project 14.0");
        assert_eq!(comp_obj.get_application_version(), Some(14));
        assert_eq!(comp_obj.get_file_format(), Some("MSProject.MPP14"));
        assert_eq!(comp_obj.get_application_id(), Some("MSProject.Project.9"));
    }

    #[test]
    fn mpp4_format_is_hard_coded() {
// Java: when applicationName == "Microsoft Project 4.0", the subsequent fields are no longer read
        let comp_obj = parse_from_compound_file(&comp_obj_payload(
            "Microsoft Project 4.0",
            None,
            None,
        ));

        assert_eq!(comp_obj.get_application_version(), Some(4));
        assert_eq!(comp_obj.get_file_format(), Some("MSProject.MPP4"));
        assert_eq!(comp_obj.get_application_id(), Some("MSProject.Project.4"));
    }

    #[test]
    fn absent_optional_fields_stay_none() {
// fileFormat length is 0: Java's `if (length > 0)` is false and both fields are null
        let mut payload = vec![0u8; 28];
        append_string(&mut payload, "Microsoft Project 9.0");
        payload.extend_from_slice(&0i32.to_le_bytes());

        let comp_obj = parse_from_compound_file(&payload);
        assert_eq!(comp_obj.get_application_version(), Some(9));
        assert_eq!(comp_obj.get_file_format(), None);
        assert_eq!(comp_obj.get_application_id(), None);

// fileFormat is present but the applicationID length is 0
        let mut payload = vec![0u8; 28];
        append_string(&mut payload, "Microsoft Project 9.0");
        append_string(&mut payload, "MSProject.MPP9");
        payload.extend_from_slice(&0i32.to_le_bytes());

        let comp_obj = parse_from_compound_file(&payload);
        assert_eq!(comp_obj.get_file_format(), Some("MSProject.MPP9"));
        assert_eq!(comp_obj.get_application_id(), None);
    }

    #[test]
    fn unknown_application_name_has_no_version() {
        let mut payload = vec![0u8; 28];
        append_string(&mut payload, "Not Microsoft Project");
        append_string(&mut payload, "MSProject.MPP9");
// The applicationID length is 0
        payload.extend_from_slice(&0i32.to_le_bytes());

        let comp_obj = parse_from_compound_file(&payload);

        assert_eq!(comp_obj.get_application_version(), None);
        assert_eq!(comp_obj.get_file_format(), Some("MSProject.MPP9"));
        assert_eq!(comp_obj.get_application_id(), None);
    }

    #[test]
    fn version_pattern_requires_the_full_match() {
// Whole-string match: the version number is not taken when the ending is not ".0"
        assert_eq!(parse_application_version("Microsoft Project 14.1"), None);
        assert_eq!(parse_application_version("Microsoft Project 14.0x"), None);
        assert_eq!(parse_application_version("xMicrosoft Project 14.0"), None);
// The `.` in `Microsoft.Project.` can match any character (Java regex `.`)
        assert_eq!(parse_application_version("Microsoft-Project-14.0"), Some(14));
// No digits
        assert_eq!(parse_application_version("Microsoft Project .0"), None);
// Out of int range: Java throws NumberFormatException, Rust returns None
        assert_eq!(parse_application_version("Microsoft Project 99999999999.0"), None);
    }

    #[test]
    fn truncated_stream_returns_error() {
// Empty stream: Java throws EOFException during the skip/read phase
        assert!(CompObj::new(&[]).is_err());
// Only 28 bytes, so the applicationName length cannot be read
        assert!(CompObj::new(&[0u8; 28]).is_err());
// The applicationName length is declared as 10 but the content is insufficient
        let mut payload = vec![0u8; 28];
        payload.extend_from_slice(&10i32.to_le_bytes());
        payload.extend_from_slice(b"abc");
        assert!(CompObj::new(&payload).is_err());
// applicationName length is 0: Java throws StringIndexOutOfBoundsException
        let mut payload = vec![0u8; 28];
        payload.extend_from_slice(&0i32.to_le_bytes());
        assert!(CompObj::new(&payload).is_err());

// The stream ends right after fileFormat: Java throws EOFException when reading the applicationID length
        let mut payload = vec![0u8; 28];
        append_string(&mut payload, "Microsoft Project 9.0");
        append_string(&mut payload, "MSProject.MPP9");
        assert!(CompObj::new(&payload).is_err());
    }

    #[test]
    fn display_matches_java_layout() {
        let comp_obj = parse_from_compound_file(&comp_obj_payload(
            "Microsoft Project 14.0",
            Some("MSProject.MPP14"),
            Some("MSProject.Project.9"),
        ));

// The toString in the Java source has no "]" at the end
        assert_eq!(
            comp_obj.to_string(),
            "[CompObj applicationName=Microsoft Project 14.0 applicationID=MSProject.Project.9 fileFormat=MSProject.MPP14"
        );

// Nullable fields are rendered as "null" following the semantics of Java string concatenation
        let comp_obj = parse_from_compound_file(&comp_obj_payload(
            "Microsoft Project 4.0",
            None,
            None,
        ));
        assert_eq!(
            comp_obj.to_string(),
            "[CompObj applicationName=Microsoft Project 4.0 applicationID=MSProject.Project.4 fileFormat=MSProject.MPP4"
        );
    }
}
