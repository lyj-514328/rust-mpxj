//! Corresponds to Java: `org/mpxj/mpp/ProjectPropertiesReader.java`
//!
//! Reads project properties from MPP8 / MPP9 / MPP12 files: some come from the `Props` stream, others from the OLE compound document's
//! two "document summary information" property set streams.
//!
//! # Property Set parsing (Rust-specific, replaces Apache POI's HPSF)
//!
//! Java uses POI's `PropertySet` / `SummaryInformation` / `DocumentSummaryInformation` to read
//! the `\u{5}SummaryInformation` and `\u{5}DocumentSummaryInformation` streams in the OLE compound document.
//! The Rust `cfb` crate does **not** support HPSF, so this file implements property set parsing itself, with the layout following
//! [MS-OLEPS](“Property Set Stream”):
//!
//! ```text
//! the stream header (28 bytes):
//!   0  : WORD byte order (0xFFFE = little endian)
//!   2  : WORD format version
//!   4  : DWORD OS version
//!   8  : 16-byte CLSID
//!   24 : DWORD number of property sets (sections)
//! Description of each section (20 bytes): 16-byte FMTID + DWORD offset relative to the start of this stream
//! section (starting at the above offset):
//!   DWORD section byte length
//!   DWORD property count
//!   property count × (DWORD property ID, DWORD offset relative to the start of the section)
//!   values of each property: DWORD type + value (string types additionally carry a DWORD length/character count)
//! ```
//!
//! Implemented property types: `VT_I2` / `VT_I4` / `VT_UI1` / `VT_UI2` / `VT_UI4` / `VT_BOOL` /
//! `VT_R8` / `VT_LPSTR` / `VT_LPWSTR` / `VT_BSTR` / `VT_FILETIME` / `VT_BLOB` / `VT_EMPTY`.
//! **Not implemented** (always returns [`PropertyValue::Unsupported`], never panics):
//! `VT_NULL`,`VT_I8`,`VT_UI8`,`VT_DATE`,`VT_CF`,`VT_CLSID`,`VT_STREAM`,
//! `VT_STORAGE`, the `VT_VECTOR` family (`0x1000` flag), and the `VT_ARRAY` family.
//!
//! Skipped Java / POI behavior and the reasons:
//!
//! | Java / POI | Rust | Notes |
//! |---|---|---|
//! | `PropertySet` supports big-endian byte order (`0xFEFF`) | Only little endian (`0xFFFE`) is supported; otherwise returns `None` | MPP files are always little endian; POI's big-endian support is not needed here (never panics) |
//! | POI throws `RuntimeException` on parse failure | Returns `None`, and the caller logs "ignore error" | Follows the "never panic" convention; see below |
//! | `SummaryInformation`'s `getThumbnail` / `getPageCount` / `getWordCount` / `getCharCount` / `getSecurity` | Not exposed | `ProjectPropertiesReader` does not use these properties ([`SummaryInformation`] only provides the getters used by the reader) |
//! | `DocumentSummaryInformation`'s `getByteCount` / `getLineCount` / `getParCount` / `getSlideCount` / `getNoteCount` / `getHiddenCount` / `getMultimediaClipCount` / `getScale` / `getHeadingPairs` / `getDocumentParts` / `getLinksDirty` | Not exposed | Same as above (not used by the reader) |
//! | `CustomProperty` | Key-value pairs of [`PropertyValue`] | The Rust side uses `BTreeMap<String, PropertyValue>` to represent the name dictionary + values (Java's `CustomProperties` is an ordered `TreeMap`) |
//! | `new String(bytes, codepage)` | [`decode_lpstr`] maps to `encoding_rs` via [`codepage_to_encoding`] | POI's `CodePageUtil.codepageToEncoding` uses Java charset names. Code pages without an `encoding_rs` implementation (EBCDIC `cp037`, Johab, MacArabic, etc.) fall back to [`DEFAULT_CODEPAGE`] (POI fails because the charset is unavailable) |
//! | When the code page property (ID 1) is missing, POI uses `Property.DEFAULT_CODEPAGE` (Windows-1252) | [`DEFAULT_CODEPAGE`] | Matches POI, **not** the Java platform default charset; dictionary names are also decoded using the section code page (corresponds to `Section.readDictionary`) |
//!
//! # Differences from Java (`ProjectPropertiesReader` itself)
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `final class ProjectPropertiesReader` | `pub struct ProjectPropertiesReader` + associated functions | Java's methods do not depend on instance state |
//! | `process(ProjectFile file, Props props, DirectoryEntry rootDir) throws MPXJException` | [`ProjectPropertiesReader::process`] returns `MpxjResult<()>` | Uses [`MpxjError`] uniformly |
//! | `getPopulatedValue(String, Supplier<String>)` | [`ProjectPropertiesReader::get_populated_value`] accepts `impl FnOnce` | Rust closures are equivalent to Java's `Supplier` |
//! | `getLocalDateTime(Date)` | [`ProjectPropertiesReader::get_local_date_time`] | FILETIME is first parsed as a UTC [`NaiveDateTime`], then converted using the system local time zone, matching Java's `LocalDateTime.ofInstant`; invalid times return `None` |
//! | `ph.setBaselineDate(1..10, …)` | `ph.set_baseline_date_for_index(1..10, …)` | Rust has no overloading |
//! | `ph.setCustomProperties(Map<String, Object>)` | `set_custom_properties(HashMap<String, FieldValue>)` | The values `FieldValue` can carry correspond to a subset of Java's `Object` (see [`custom_property_field_value`]) |
//! | `summaryInformation.getEditTime()` (`long`) | `get_edit_time()` returns `i64`, narrowed to `i32` when written | Matches Java's `(int)` cast |
//! | Property set parse failure | Logs "ignore error" and treats as empty properties | A corrupt Java `SummaryInformation` stream throws upward (the whole read fails); the Rust side chooses to continue reading (never panics) |
//! | Property setters immediately update `ProjectContext`'s default duration units | After `process` finishes assigning properties and releases the `ProjectFile` borrow, it syncs the minutes per day/week and days per month | While holding a long-lived mutable borrow of `ProjectFile`, the setters' reverse borrow would fail; the final values match Java before task reading begins |
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.
//!
//! [MS-OLEPS]: https://learn.microsoft.com/openspecs/windows_protocols/ms-oleps/

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use chrono::{Local, NaiveDate, NaiveDateTime, TimeZone};

use crate::common::day_of_week_helper::DayOfWeekHelper;
use crate::common::number_helper::NumberHelper;
use crate::duration::Duration;
use crate::field_value::FieldValue;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::ole::DirectoryEntry;
use crate::mpp::props::Props;
use crate::mpp::props_key::PropsKey;
use crate::mpp::task_type_helper::TaskTypeHelper;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_file::ProjectFile;
use crate::rate::Rate;
use crate::schedule_from::ScheduleFrom;
use crate::time_unit::TimeUnit;

/// Name of the summary information property set stream (first character is the control character `\x05`).
///
/// Corresponds to Java: `org.apache.poi.hpsf.SummaryInformation.DEFAULT_STREAM_NAME`
const SUMMARY_INFORMATION_STREAM: &str = "\u{5}SummaryInformation";

/// Name of the document summary information property set stream (first character is the control character `\x05`).
///
/// Corresponds to Java: `org.apache.poi.hpsf.DocumentSummaryInformation.DEFAULT_STREAM_NAME`
const DOCUMENT_SUMMARY_INFORMATION_STREAM: &str = "\u{5}DocumentSummaryInformation";

/// Number of bytes in the property set stream header.
const PROPERTY_SET_HEADER_SIZE: usize = 28;

/// Number of bytes in each section description (16-byte FMTID + 4-byte offset).
const SECTION_DESCRIPTOR_SIZE: usize = 20;

/// Default code page (Windows-1252), used for `VT_LPSTR` when the section does not declare a code page.
///
/// Corresponds to Java: POI's `Property.DEFAULT_CODEPAGE` (i.e. `CodePageUtil.CP_WINDOWS_1252`).
/// POI sets the code page to `-1` when the section's code page property (ID 1) is missing, falling back to this constant when decoding,
/// **not** the Java platform default charset.
const DEFAULT_CODEPAGE: u16 = 1252;

/// Code page: UTF-8.
///
/// Corresponds to Java: `CodePageUtil.CP_UTF8`
const CODEPAGE_UTF8: u16 = 65001;

/// Code page: UTF-16LE.
///
/// Corresponds to Java: `CodePageUtil.CP_UTF16`
const CODEPAGE_UTF16LE: u16 = 1200;

/// In the resource pool file name data block, the fixed header length before the file name.
///
/// Corresponds to Java: `int offset = 18;`
const RESOURCE_POOL_HEADER_SIZE: usize = 18;

/// In the resource pool file name data block, the fixed header length after the 8.3 file name.
///
/// Corresponds to Java: `offset += 34;`
const RESOURCE_POOL_TRAILER_SIZE: usize = 34;

// ---------------------------------------------------------------------------
// Property Set parsing
// ---------------------------------------------------------------------------

/// A property value within a property set.
///
/// Corresponds to Java: `org.apache.poi.hpsf.Property` (and the value types of its `getValue()`)
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    /// `VT_EMPTY`.
    Empty,
    /// `VT_LPSTR` / `VT_LPWSTR` / `VT_BSTR`.
    String(String),
    /// Binary OLE property value.
    Bytes(Vec<u8>),
    /// `VT_I2` / `VT_UI1` / `VT_UI2`.
    I2(i16),
    /// `VT_I4` / `VT_UI4`.
    I4(i32),
    /// `VT_BOOL`.
    Bool(bool),
    /// `VT_R8`.
    R8(f64),
    /// `VT_FILETIME`.
    FileTime(NaiveDateTime),
    /// Unimplemented type (see the file header).
    Unsupported,
}

impl PropertyValue {
    /// Returns the string value; returns `None` if it is not a string.
    ///
    /// Corresponds to Java: `Property.getStringValue()`
    pub fn as_string(&self) -> Option<&str> {
        match self {
            PropertyValue::String(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the two-byte integer value (`VT_I2` / `VT_UI1` / `VT_UI2`).
    ///
    /// Corresponds to Java: `Property.getShortValue()` / `getIntValue()`
    pub fn as_i2(&self) -> Option<i16> {
        match self {
            PropertyValue::I2(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the four-byte integer value (`VT_I4` / `VT_UI4`).
    ///
    /// Corresponds to Java: `Property.getIntValue()` / `getLongValue()`
    pub fn as_i4(&self) -> Option<i32> {
        match self {
            PropertyValue::I4(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the boolean value.
    ///
    /// Corresponds to Java: `Property.getBooleanValue()`
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PropertyValue::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the double-precision value.
    ///
    /// Corresponds to Java: `Property.getDoubleValue()`
    pub fn as_r8(&self) -> Option<f64> {
        match self {
            PropertyValue::R8(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the timestamp value.
    ///
    /// Corresponds to Java: `Property.getDateValue()`
    pub fn as_file_time(&self) -> Option<NaiveDateTime> {
        match self {
            PropertyValue::FileTime(value) => Some(*value),
            _ => None,
        }
    }
}

/// A section within a property set stream.
///
/// Corresponds to Java: `org.apache.poi.hpsf.Section`
#[derive(Debug, Clone)]
struct Section {
    /// Property ID → decoded value.
    properties: BTreeMap<i32, PropertyValue>,
    /// Property ID → raw bytes after the type field (used by composite properties such as dictionaries).
    raw: BTreeMap<i32, Vec<u8>>,
    /// Code page declared by the section, used for decoding `VT_LPSTR` and dictionary names; [`DEFAULT_CODEPAGE`] when missing.
    ///
    /// Corresponds to Java: POI `Section`'s `codepage` (`-1` when there is no ID 1, falling back when decoding
    /// `Property.DEFAULT_CODEPAGE`).
    codepage: u16,
}

impl Default for Section {
    fn default() -> Self {
        Self {
            properties: BTreeMap::new(),
            raw: BTreeMap::new(),
            codepage: DEFAULT_CODEPAGE,
        }
    }
}

impl Section {
    /// Returns the string property.
    fn string(&self, id: i32) -> Option<String> {
        self.properties
            .get(&id)
            .and_then(PropertyValue::as_string)
            .map(str::to_owned)
    }

    /// Returns the timestamp property.
    fn file_time(&self, id: i32) -> Option<NaiveDateTime> {
        self.properties.get(&id).and_then(PropertyValue::as_file_time)
    }
}

/// Summary information (the first section of the `\u{5}SummaryInformation` stream).
///
/// Corresponds to Java: `org.apache.poi.hpsf.SummaryInformation`
///
/// Only provides the getters used by `ProjectPropertiesReader` (see "Skipped Java / POI behavior" in the file header).
#[derive(Debug, Clone, Default)]
pub struct SummaryInformation {
    /// Corresponds to Java: the first section of the `PropertySet` held by `SummaryInformation`
    section: Section,
}

impl SummaryInformation {
    /// Parses a property set stream.
    ///
    /// Corresponds to Java: `new SummaryInformation(PropertySet ps)`
    ///
    /// Returns `None` when the data is not a valid property set stream (Java throws an exception).
    pub fn new(data: &[u8]) -> Option<Self> {
        let sections = parse_property_set(data)?;
        Some(Self {
            section: sections.into_iter().next().unwrap_or_default(),
        })
    }

    /// Returns the document title.
    ///
    /// Corresponds to Java: `public String getTitle()`
    pub fn get_title(&self) -> Option<String> {
        self.section.string(PIDSI_TITLE)
    }

    /// Returns the subject.
    ///
    /// Corresponds to Java: `public String getSubject()`
    pub fn get_subject(&self) -> Option<String> {
        self.section.string(PIDSI_SUBJECT)
    }

    /// Returns the author.
    ///
    /// Corresponds to Java: `public String getAuthor()`
    pub fn get_author(&self) -> Option<String> {
        self.section.string(PIDSI_AUTHOR)
    }

    /// Returns the keywords.
    ///
    /// Corresponds to Java: `public String getKeywords()`
    pub fn get_keywords(&self) -> Option<String> {
        self.section.string(PIDSI_KEYWORDS)
    }

    /// Returns the comments.
    ///
    /// Corresponds to Java: `public String getComments()`
    pub fn get_comments(&self) -> Option<String> {
        self.section.string(PIDSI_COMMENTS)
    }

    /// Returns the template name.
    ///
    /// Corresponds to Java: `public String getTemplate()`
    pub fn get_template(&self) -> Option<String> {
        self.section.string(PIDSI_TEMPLATE)
    }

    /// Returns the last saved by.
    ///
    /// Corresponds to Java: `public String getLastAuthor()`
    pub fn get_last_author(&self) -> Option<String> {
        self.section.string(PIDSI_LASTAUTHOR)
    }

    /// Returns the revision number (as a string).
    ///
    /// Corresponds to Java: `public String getRevNumber()`
    pub fn get_rev_number(&self) -> Option<String> {
        self.section.string(PIDSI_REVNUMBER)
    }

    /// Returns the creation time.
    ///
    /// Corresponds to Java: `public Date getCreateDateTime()`
    pub fn get_create_date_time(&self) -> Option<NaiveDateTime> {
        self.section.file_time(PIDSI_CREATE_DTM)
    }

    /// Returns the last saved time.
    ///
    /// Corresponds to Java: `public Date getLastSaveDateTime()`
    pub fn get_last_save_date_time(&self) -> Option<NaiveDateTime> {
        self.section.file_time(PIDSI_LASTSAVE_DTM)
    }

    /// Returns the last printed time.
    ///
    /// Corresponds to Java: `public Date getLastPrinted()`
    pub fn get_last_printed(&self) -> Option<NaiveDateTime> {
        self.section.file_time(PIDSI_LASTPRINTED)
    }

    /// Returns the application name.
    ///
    /// Corresponds to Java: `public String getApplicationName()`
    pub fn get_application_name(&self) -> Option<String> {
        self.section.string(PIDSI_APPNAME)
    }

    /// Returns the total editing time (FILETIME's 100-nanosecond count).
    ///
    /// Corresponds to Java: `public long getEditTime()`
    pub fn get_edit_time(&self) -> i64 {
        match self.section.file_time(PIDSI_EDITTIME) {
            None => 0,
            Some(value) => {
                // POI's SummaryInformation.getEditTime() preserves FILETIME's 100-nanosecond count.
                let epoch = NaiveDate::from_ymd_opt(1601, 1, 1).and_then(|date| date.and_hms_opt(0, 0, 0));
                match epoch {
                    None => 0,
                    Some(epoch) => (value - epoch).num_nanoseconds().map(|ns| ns / 100).unwrap_or(0),
                }
            }
        }
    }
}

/// Document summary information (the two sections of the `\u{5}DocumentSummaryInformation` stream).
///
/// Corresponds to Java: `org.apache.poi.hpsf.DocumentSummaryInformation`
#[derive(Debug, Clone, Default)]
pub struct DocumentSummaryInformation {
    /// Corresponds to Java: the first section (document summary properties)
    section: Section,
    /// Custom document properties (name → value).
    ///
    /// Corresponds to Java: `public CustomProperties getCustomProperties()`
    custom_properties: BTreeMap<String, PropertyValue>,
}

impl DocumentSummaryInformation {
    /// Parses a property set stream.
    ///
    /// Corresponds to Java: `new DocumentSummaryInformation(PropertySet ps)`
    pub fn new(data: &[u8]) -> Option<Self> {
        let sections = parse_property_set(data)?;
        let mut sections = sections.into_iter();
        let section = sections.next().unwrap_or_default();
        let custom_properties = sections
            .next()
            .map(|custom_section| parse_custom_properties(&custom_section))
            .unwrap_or_default();

        Some(Self {
            section,
            custom_properties,
        })
    }

    /// Returns the category.
    ///
    /// Corresponds to Java: `public String getCategory()`
    pub fn get_category(&self) -> Option<String> {
        self.section.string(PIDDSI_CATEGORY)
    }

    /// Returns the presentation format.
    ///
    /// Corresponds to Java: `public String getPresentationFormat()`
    pub fn get_presentation_format(&self) -> Option<String> {
        self.section.string(PIDDSI_PRESFORMAT)
    }

    /// Returns the manager.
    ///
    /// Corresponds to Java: `public String getManager()`
    pub fn get_manager(&self) -> Option<String> {
        self.section.string(PIDDSI_MANAGER)
    }

    /// Returns the company.
    ///
    /// Corresponds to Java: `public String getCompany()`
    pub fn get_company(&self) -> Option<String> {
        self.section.string(PIDDSI_COMPANY)
    }

    /// Returns the content type.
    ///
    /// Corresponds to Java: `public String getContentType()`
    pub fn get_content_type(&self) -> Option<String> {
        self.section.string(PIDDSI_CONTENTTYPE)
    }

    /// Returns the content status.
    ///
    /// Corresponds to Java: `public String getContentStatus()`
    pub fn get_content_status(&self) -> Option<String> {
        self.section.string(PIDDSI_CONTENTSTATUS)
    }

    /// Returns the language.
    ///
    /// Corresponds to Java: `public String getLanguage()`
    pub fn get_language(&self) -> Option<String> {
        self.section.string(PIDDSI_LANGUAGE)
    }

    /// Returns the document version.
    ///
    /// Corresponds to Java: `public String getDocumentVersion()`
    pub fn get_document_version(&self) -> Option<String> {
        self.section.string(PIDDSI_DOCVERSION)
    }

    /// Returns the custom document properties.
    ///
    /// Corresponds to Java: `public CustomProperties getCustomProperties()`
    pub fn get_custom_properties(&self) -> &BTreeMap<String, PropertyValue> {
        &self.custom_properties
    }
}

// ---- Standard property IDs (corresponding to POI's `PropertyIDMap` / `Section` constants) ----

/// Code page.
const PID_CODEPAGE: i32 = 1;

/// Summary information: title.
const PIDSI_TITLE: i32 = 2;
/// Summary information: subject.
const PIDSI_SUBJECT: i32 = 3;
/// Summary information: author.
const PIDSI_AUTHOR: i32 = 4;
/// Summary information: keywords.
const PIDSI_KEYWORDS: i32 = 5;
/// Summary information: comments.
const PIDSI_COMMENTS: i32 = 6;
/// Summary information: template.
const PIDSI_TEMPLATE: i32 = 7;
/// Summary information: last saved by.
const PIDSI_LASTAUTHOR: i32 = 8;
/// Summary information: revision number.
const PIDSI_REVNUMBER: i32 = 9;
/// Summary information: editing time (FILETIME).
const PIDSI_EDITTIME: i32 = 10;
/// Summary information: last printed time.
const PIDSI_LASTPRINTED: i32 = 11;
/// Summary information: creation time.
const PIDSI_CREATE_DTM: i32 = 12;
/// Summary information: last saved time.
const PIDSI_LASTSAVE_DTM: i32 = 13;
/// Summary information: application name.
const PIDSI_APPNAME: i32 = 18;

/// Document summary information: category.
const PIDDSI_CATEGORY: i32 = 2;
/// Document summary information: presentation format.
const PIDDSI_PRESFORMAT: i32 = 3;
/// Document summary information: manager.
const PIDDSI_MANAGER: i32 = 14;
/// Document summary information: company.
const PIDDSI_COMPANY: i32 = 15;
/// Document summary information: content type.
const PIDDSI_CONTENTTYPE: i32 = 26;
/// Document summary information: content status.
const PIDDSI_CONTENTSTATUS: i32 = 27;
/// Document summary information: language.
const PIDDSI_LANGUAGE: i32 = 28;
/// Document summary information: document version.
const PIDDSI_DOCVERSION: i32 = 29;

/// Name dictionary in the custom property section (without a Variant type header).
///
/// Corresponds to Java: `PropertyIDMap.PID_DICTIONARY`
const PID_DICTIONARY: i32 = 0;

// ---- Variant types (corresponding to `org.apache.poi.hpsf.Variant`) ----

/// `VT_EMPTY`.
const VT_EMPTY: u32 = 0;
/// `VT_NULL`.
const VT_NULL: u32 = 1;
/// `VT_I2`.
const VT_I2: u32 = 2;
/// `VT_I4`.
const VT_I4: u32 = 3;
/// `VT_R8`.
const VT_R8: u32 = 5;
/// `VT_BSTR`.
const VT_BSTR: u32 = 8;
/// `VT_BOOL`.
const VT_BOOL: u32 = 11;
/// `VT_UI1`.
const VT_UI1: u32 = 17;
/// `VT_UI2`.
const VT_UI2: u32 = 18;
/// `VT_UI4`.
const VT_UI4: u32 = 19;
/// `VT_LPSTR`.
const VT_LPSTR: u32 = 30;
/// `VT_LPWSTR`.
const VT_LPWSTR: u32 = 31;
/// `VT_FILETIME`.
const VT_FILETIME: u32 = 64;
/// `VT_BLOB`.
const VT_BLOB: u32 = 65;
/// Vector flag (`VT_VECTOR`).
const VT_VECTOR_FLAG: u32 = 0x1000;
/// Array flag (`VT_ARRAY`).
const VT_ARRAY_FLAG: u32 = 0x2000;

/// Parses a property set stream and returns the sections.
///
/// Corresponds to Java: `new PropertySet(InputStream)`
///
/// Returns `None` when the data is invalid (insufficient length, byte order is not little endian, section description out of bounds).
fn parse_property_set(data: &[u8]) -> Option<Vec<Section>> {
    if data.len() < PROPERTY_SET_HEADER_SIZE {
        return None;
    }

    // Java: m_byteOrder = LittleEndian.getShort(...)
    // Only little endian (0xFFFE) is supported; big endian (0xFEFF) does not appear in MPP files.
    let byte_order = read_u16(data, 0)?;
    if byte_order != 0xFFFE {
        return None;
    }

    // Java: m_format = LittleEndian.getShort(...) (unused)
    // Java: m_osVersion = LittleEndian.getInt(...) (unused)
    // Java: m_classID = new ClassID(...) (unused)

    // Java: m_sectionCount = LittleEndian.readInt(stream);
    let section_count = read_i32(data, 24)?;
    if section_count < 0 {
        return None;
    }

    let mut sections = Vec::new();
    for index in 0..section_count as usize {
        // Java: m_formatID = new ClassID(stream); long offset = LittleEndian.getUInt(stream); stream.seek(offset);
        let descriptor = PROPERTY_SET_HEADER_SIZE + index * SECTION_DESCRIPTOR_SIZE;
        let section_offset = read_u32(data, descriptor + 16)? as usize;

        if let Some(section) = parse_section(data, section_offset) {
            sections.push(section);
        }
    }

    Some(sections)
}

/// Parses a single section.
///
/// Corresponds to Java: `Section.read(InputStream)`
fn parse_section(data: &[u8], section_offset: usize) -> Option<Section> {
    // Java: m_size = LittleEndian.readInt(stream); m_propertyCount = LittleEndian.readInt(stream);
    let size = read_i32(data, section_offset)?;
    let property_count = read_i32(data, section_offset + 4)?;
    if size < 0 || property_count < 0 {
        return None;
    }

    let mut entries: Vec<(i32, usize)> = Vec::with_capacity(property_count as usize);
    let mut position = section_offset + 8;
    for _ in 0..property_count {
        // Java: m_properties.put(propertyID, new PropertyIdentifier(type, offset))
        let id = read_i32(data, position)?;
        let offset = read_u32(data, position + 4)? as usize;
        entries.push((id, offset));
        position += 8;
    }

    let section_end = section_offset.saturating_add(size as usize);
    // The length of each property is determined by "the next larger offset" (Java's POI also splits by offset order).
    let mut sorted_offsets: Vec<usize> = entries.iter().map(|(_, offset)| *offset).collect();
    sorted_offsets.sort_unstable();

    // Get the code page first: decoding `VT_LPSTR` depends on it.
    let codepage = entries
        .iter()
        .find(|(id, _)| *id == PID_CODEPAGE)
        .and_then(|(_, offset)| {
            let start = section_offset + offset;
            let variant = read_u32(data, start)?;
            if variant & VT_VECTOR_FLAG != 0 || variant & VT_ARRAY_FLAG != 0 {
                return None;
            }
            if variant != VT_I2 {
                return None;
            }
            read_u16(data, start + 4)
        })
        .unwrap_or(DEFAULT_CODEPAGE);

    let mut section = Section::default();
    section.codepage = codepage;
    for (id, offset) in entries {
        let start = section_offset + offset;
        let end = sorted_offsets
            .iter()
            .find(|candidate| **candidate > offset)
            .map(|candidate| section_offset + *candidate)
            .unwrap_or(section_end)
            .min(data.len());

        // OLEPS property 0 is the name dictionary without a Variant type header; property 1 is the code page.
        if id == PID_DICTIONARY {
            if let Some(raw) = data.get(start..end) {
                section.raw.insert(id, raw.to_vec());
            }
            continue;
        }
        if let Some((value, raw)) = parse_value(data, start, end, codepage) {
            section.properties.insert(id, value);
            section.raw.insert(id, raw);
        }
    }

    Some(section)
}

/// Parses a single property value.
///
/// Corresponds to Java: `Property.read(...)`
///
/// Returns the "decoded value" and the "raw bytes after the type field"; returns `None` on parse failure.
fn parse_value(
    data: &[u8],
    start: usize,
    end: usize,
    codepage: u16,
) -> Option<(PropertyValue, Vec<u8>)> {
    if start.checked_add(4)? > end {
        return None;
    }

    // Java: Variant's type field is 4 bytes; the low 16 bits are the type, the high bits are the vector / array flags.
    let variant = read_u32(data, start)?;
    let raw_start = start + 4;
    let raw = data.get(raw_start..end)?.to_vec();

    if variant & VT_VECTOR_FLAG != 0 || variant & VT_ARRAY_FLAG != 0 {
        return Some((PropertyValue::Unsupported, raw));
    }

    let value = match variant {
        VT_EMPTY => PropertyValue::Empty,
        // VT_NULL has no value, treated as "unsupported" (POI returns null)
        VT_NULL => PropertyValue::Unsupported,
        VT_I2 => PropertyValue::I2(read_i16(data, raw_start)?),
        VT_I4 => PropertyValue::I4(read_i32(data, raw_start)?),
        VT_R8 => PropertyValue::R8(read_f64(data, raw_start)?),
        VT_BOOL => PropertyValue::Bool(read_i16(data, raw_start)? != 0),
        VT_UI1 => PropertyValue::I2(i16::from(read_u8(data, raw_start)?)),
        VT_UI2 => PropertyValue::I2(read_u16(data, raw_start)? as i16),
        VT_UI4 => PropertyValue::I4(read_u32(data, raw_start)? as i32),
        VT_LPSTR => {
            // Java: int length = LittleEndian.getInt(...); String value = new String(bytes, codepage)
            let length = read_u32(data, raw_start)? as usize;
            let begin = raw_start + 4;
            let bytes = bounded(data, begin, length, end);
            PropertyValue::String(decode_lpstr(bytes, codepage))
        }
        VT_LPWSTR => {
            // Java: int length = LittleEndian.getInt(...); (character count, each character is 2 bytes)
            let characters = read_u32(data, raw_start)? as usize;
            let begin = raw_start + 4;
            let bytes = bounded(data, begin, characters.saturating_mul(2), end);
            PropertyValue::String(decode_utf16le(bytes))
        }
        VT_BSTR => {
            // VT_BSTR, like VT_LPWSTR, carries a 4-byte "byte count" prefix
            let bytes_count = read_u32(data, raw_start)? as usize;
            let begin = raw_start + 4;
            let bytes = bounded(data, begin, bytes_count, end);
            PropertyValue::String(decode_utf16le(bytes))
        }
        VT_FILETIME => {
            // Java: Filetime consists of two DWORDs (low 32 bits first)
            let ticks = read_i64(data, raw_start)?;
            match filetime_to_datetime(ticks) {
                Some(value) => PropertyValue::FileTime(value),
                None => PropertyValue::Unsupported,
            }
        }
        VT_BLOB => {
            // POI's byte[] includes the length prefix and excludes the 4-byte alignment padding at the end of the property value.
            let size = read_u32(data, raw_start)? as usize;
            let value_end = raw_start.checked_add(4)?.checked_add(size)?;
            PropertyValue::Bytes(data.get(raw_start..value_end.min(end))?.to_vec())
        }
        // VT_I8 / VT_UI8 / VT_DATE / VT_CF / VT_CLSID / VT_STREAM / VT_STORAGE etc. are not implemented
        _ => PropertyValue::Unsupported,
    };

    Some((value, raw))
}

/// Parses the custom property section: takes names from the ID 0 name dictionary, then pairs them with the values of the same ID.
///
/// Corresponds to Java: `DocumentSummaryInformation.getCustomProperties()` (internally in POI by
/// `Section.getProperties(int)` and `Property.getDictionary()`)
///
/// Dictionary layout (matching POI's `Property.getDictionary()`; no Variant type header):
///
/// ```text
/// DWORD entry count
/// entry count × (DWORD property ID, DWORD name byte count (including trailing NUL), name bytes); no padding between entries
/// ```
fn parse_custom_properties(section: &Section) -> BTreeMap<String, PropertyValue> {
    let mut result = BTreeMap::new();

    let Some(names) = section.raw.get(&PID_DICTIONARY) else {
        return result;
    };

    on_dictionary_entries(names, section.codepage, &mut |id, name| {
        if id == PID_DICTIONARY || id == PID_CODEPAGE {
            return;
        }
        if let Some(value) = section.properties.get(&id) {
            result.insert(name.to_owned(), value.clone());
        }
    });

    result
}

/// Iterates over the (property ID, name) entries in the name dictionary.
///
/// `codepage` is the code page declared by the section ([`DEFAULT_CODEPAGE`] when missing), matching POI's
/// `Section.readDictionary(leis, length, codepage)`. Terminates early on corrupt data (never panics).
fn on_dictionary_entries(data: &[u8], codepage: u16, callback: &mut impl FnMut(i32, &str)) {
    let Some(count) = read_i32(data, 0) else {
        return;
    };
    // Defensive upper bound: avoids long loops caused by corrupt data
    if count <= 0 || count > 10_000 {
        return;
    }

    let mut position = 4usize;
    for _ in 0..count {
        let Some(id) = read_i32(data, position) else {
            return;
        };
        let Some(length) = read_i32(data, position + 4) else {
            return;
        };
        if length <= 0 {
            return;
        }

        let begin = position + 8;
        let bytes = bounded(data, begin, length as usize, data.len());
        // Java: names end with NUL; POI strips the trailing NUL
        let name = decode_lpstr(bytes, codepage);
        let name = name.trim_end_matches('\0');
        callback(id, name);

        position += 8 + length as usize;
        if position >= data.len() {
            return;
        }
    }
}

/// Converts `FILETIME` (100-nanosecond count since 1601-01-01) to [`NaiveDateTime`].
///
/// Corresponds to Java: `org.apache.poi.hpsf.Filetime.getJavaDate()`
///
/// Returns `None` when the value is non-positive (invalid values such as `0xFFFFFFFF` in POI).
fn filetime_to_datetime(ticks: i64) -> Option<NaiveDateTime> {
    if ticks <= 0 {
        return None;
    }

    let ticks = ticks as u64;
    let seconds = (ticks / 10_000_000) as i64;
    let nanoseconds = ((ticks % 10_000_000) * 100) as i64;

    let epoch = NaiveDate::from_ymd_opt(1601, 1, 1)?.and_hms_opt(0, 0, 0)?;
    epoch
        .checked_add_signed(chrono::Duration::seconds(seconds))?
        .checked_add_signed(chrono::Duration::nanoseconds(nanoseconds))
}

/// Decodes `VT_LPSTR` bytes using the code page (strips the trailing NUL character).
///
/// Corresponds to Java: `new String(bytes, codepage)` (POI's `CodePageUtil.getStringFromCodePage`)
fn decode_lpstr(bytes: &[u8], codepage: u16) -> String {
    let value = match codepage_to_encoding(codepage) {
        Some(encoding) => encoding.decode(bytes).0.into_owned(),
        // Code page with no corresponding `encoding_rs` implementation: POI fails when resolving the Java charset name;
        // the Rust side falls back to the default code page (see the differences table in the file header).
        None => encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned(),
    };

    value.trim_end_matches('\0').to_owned()
}

/// Maps an OLE property set code page number to an `encoding_rs` charset.
///
/// Corresponds to Java: `org.apache.poi.util.CodePageUtil.codepageToEncoding(int)`.
///
/// POI returns a fixed Java charset name for explicitly listed code pages and `"cp" + code page number` otherwise; the Rust side only
/// maps values that `encoding_rs` (the WHATWG Encoding standard) implements and that POI produces. Unmappable
/// (EBCDIC `cp037`,Johab,MacArabic / MacHebrew / MacGreek / MacThai / MacCentralEurope /
/// ones (MacIceland / MacTurkish / MacCroatian, ISO-2022-KR, US-ASCII, etc.) return `None`,
/// and the caller falls back to [`DEFAULT_CODEPAGE`].
///
/// A few mappings are equivalent consolidations of Java names in the WHATWG standard: `ISO-8859-1` → `windows-1252`,
/// `ISO-8859-9` → `windows-1254` (WHATWG points these two labels to the corresponding windows code pages),
/// `GB2312` → `GBK`, Mac Chinese/Japanese/Korean code pages → the corresponding CJK encodings.
fn codepage_to_encoding(codepage: u16) -> Option<&'static encoding_rs::Encoding> {
    Some(match codepage {
        // CodePageUtil.CP_UTF16 / CP_UTF16_BE / CP_UTF8
        CODEPAGE_UTF16LE => encoding_rs::UTF_16LE,
        1201 => encoding_rs::UTF_16BE,
        CODEPAGE_UTF8 => encoding_rs::UTF_8,
        // CP_SJIS(932) / CP_MAC_JAPAN(10001) → "SJIS"
        932 | 10001 => encoding_rs::SHIFT_JIS,
        // CP_GBK(936) / CP_GB2312(52936) / CP_MAC_CHINESE_SIMPLE(10008) → "GBK"/"GB2312"/"EUC_CN"
        936 | 52936 | 10008 => encoding_rs::GBK,
        // CP_MS949(949) / CP_EUC_KR(51949) / CP_MAC_KOREAN(10003) → "ms949"/"EUC-KR"
        949 | 51949 | 10003 => encoding_rs::EUC_KR,
        // CP_MAC_CHINESE_TRADITIONAL(10002) → "Big5"; same for cp950 in POI's default branch
        950 | 10002 => encoding_rs::BIG5,
        // cp874 in POI's default branch
        874 => encoding_rs::WINDOWS_874,
        // CP_WINDOWS_1250..1258 / CP_WINDOWS_1252_BIFF23(32769)
        1250 => encoding_rs::WINDOWS_1250,
        1251 => encoding_rs::WINDOWS_1251,
        1252 | 32769 => encoding_rs::WINDOWS_1252,
        1253 => encoding_rs::WINDOWS_1253,
        1254 => encoding_rs::WINDOWS_1254,
        1255 => encoding_rs::WINDOWS_1255,
        1256 => encoding_rs::WINDOWS_1256,
        1257 => encoding_rs::WINDOWS_1257,
        1258 => encoding_rs::WINDOWS_1258,
        // CP_MAC_ROMAN(10000) / CP_MAC_ROMAN_BIFF23(32768) → "MacRoman"
        10000 | 32768 => encoding_rs::MACINTOSH,
        // CP_MAC_CYRILLIC(10007) → "MacCyrillic"
        10007 => encoding_rs::X_MAC_CYRILLIC,
        // CP_KOI8_R(20866)
        20866 => encoding_rs::KOI8_R,
        // CP_ISO_8859_1(28591): WHATWG consolidates iso-8859-1 into windows-1252
        28591 => encoding_rs::WINDOWS_1252,
        28592 => encoding_rs::ISO_8859_2,
        28593 => encoding_rs::ISO_8859_3,
        28594 => encoding_rs::ISO_8859_4,
        28595 => encoding_rs::ISO_8859_5,
        28596 => encoding_rs::ISO_8859_6,
        28597 => encoding_rs::ISO_8859_7,
        28598 => encoding_rs::ISO_8859_8,
        // CP_ISO_8859_9(28599): WHATWG consolidates iso-8859-9 into windows-1254
        28599 => encoding_rs::WINDOWS_1254,
        // CP_ISO_2022_JP1..3(50220/50221/50222)
        50220 | 50221 | 50222 => encoding_rs::ISO_2022_JP,
        // CP_EUC_JP(51932)
        51932 => encoding_rs::EUC_JP,
        // CP_GB18030(54936)
        54936 => encoding_rs::GB18030,
        _ => return None,
    })
}

/// Decodes as UTF-16LE (strips the trailing NUL character).
///
/// Corresponds to Java: `new String(bytes, StandardCharsets.UTF_16LE)`
fn decode_utf16le(bytes: &[u8]) -> String {
    let value = encoding_rs::UTF_16LE.decode(bytes).0.into_owned();
    value.trim_end_matches('\0').to_owned()
}

/// Takes `data[begin..begin + length]`, also limited by `end` and the slice length.
fn bounded(data: &[u8], begin: usize, length: usize, end: usize) -> &[u8] {
    let limit = end.min(data.len()).max(begin);
    let requested = begin.saturating_add(length).min(limit);
    data.get(begin..requested).unwrap_or(&[])
}

/// Reads one byte.
fn read_u8(data: &[u8], offset: usize) -> Option<u8> {
    data.get(offset).copied()
}

/// Reads two bytes (little endian).
fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    let bytes: [u8; 2] = data.get(offset..offset.checked_add(2)?)?.try_into().ok()?;
    Some(u16::from_le_bytes(bytes))
}

/// Reads two bytes (little endian, signed).
fn read_i16(data: &[u8], offset: usize) -> Option<i16> {
    let bytes: [u8; 2] = data.get(offset..offset.checked_add(2)?)?.try_into().ok()?;
    Some(i16::from_le_bytes(bytes))
}

/// Reads four bytes (little endian, unsigned).
fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    let bytes: [u8; 4] = data.get(offset..offset.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

/// Reads four bytes (little endian, signed).
fn read_i32(data: &[u8], offset: usize) -> Option<i32> {
    let bytes: [u8; 4] = data.get(offset..offset.checked_add(4)?)?.try_into().ok()?;
    Some(i32::from_le_bytes(bytes))
}

/// Reads eight bytes (little endian).
fn read_i64(data: &[u8], offset: usize) -> Option<i64> {
    let bytes: [u8; 8] = data.get(offset..offset.checked_add(8)?)?.try_into().ok()?;
    Some(i64::from_le_bytes(bytes))
}

/// Reads eight bytes (little endian, interpreted as `f64` per IEEE 754).
fn read_f64(data: &[u8], offset: usize) -> Option<f64> {
    Some(f64::from_bits(read_i64(data, offset)? as u64))
}

/// Converts a property value to [`FieldValue`] (for custom document properties).
///
/// Corresponds to Java: `Object value = property.getValue(); if (value instanceof Date) { value = getLocalDateTime((Date) value); }`
fn custom_property_field_value(value: &PropertyValue) -> Option<FieldValue> {
    match value {
        PropertyValue::Empty | PropertyValue::Unsupported => None,
        PropertyValue::String(value) => Some(FieldValue::String(value.clone())),
        PropertyValue::Bytes(value) => Some(FieldValue::Bytes(value.clone())),
        PropertyValue::I2(value) => Some(FieldValue::Integer(i32::from(*value))),
        PropertyValue::I4(value) => Some(FieldValue::Integer(*value)),
        PropertyValue::Bool(value) => Some(FieldValue::Boolean(*value)),
        PropertyValue::R8(value) => Some(FieldValue::Double(*value)),
        // Java converts the instant corresponding to FILETIME into a LocalDateTime using the system default time zone.
        PropertyValue::FileTime(value) => Some(FieldValue::DateTime(Local.from_utc_datetime(value).naive_local())),
    }
}

// ---------------------------------------------------------------------------
// Reader
// ---------------------------------------------------------------------------

/// Project properties reader.
///
/// Corresponds to Java: `final class ProjectPropertiesReader`
pub struct ProjectPropertiesReader;

impl ProjectPropertiesReader {
    /// Main entry point for processing project properties.
    ///
    /// Corresponds to Java: `public void process(ProjectFile file, Props props, DirectoryEntry rootDir) throws MPXJException`
    pub fn process(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &Props,
        root_dir: &DirectoryEntry,
    ) -> MpxjResult<()> {
        // Java:
        // PropertySet ps = null;
        // try { ps = new PropertySet(new DocumentInputStream((DocumentEntry) rootDir.getEntry(SummaryInformation.DEFAULT_STREAM_NAME))); }
        // catch (FileNotFoundException ex) { file.addIgnoredError(ex); }
        // SummaryInformation summaryInformation = ps == null ? new SummaryInformation() : new SummaryInformation(ps);
        let summary_information = match root_dir.get_document(SUMMARY_INFORMATION_STREAM) {
            None => {
                file.borrow_mut().add_ignored_error(MpxjError::new(format!(
                    "Entry not found: {SUMMARY_INFORMATION_STREAM}"
                )));
                SummaryInformation::default()
            }
            Some(document) => match SummaryInformation::new(&document.get_data()) {
                Some(value) => value,
                None => {
                    // Java's POI throws here (the whole read fails); the Rust side logs an ignore-error and continues.
                    file.borrow_mut()
                        .add_ignored_error(MpxjError::new("Corrupt summary information"));
                    SummaryInformation::default()
                }
            },
        };

        // Java:
        // try { ps = new PropertySet(...DocumentSummaryInformation...); }
        // catch (RuntimeException | FileNotFoundException ex) { file.addIgnoredError(ex); ps = null; }
        let document_summary_information = match root_dir.get_document(DOCUMENT_SUMMARY_INFORMATION_STREAM) {
            None => {
                file.borrow_mut().add_ignored_error(MpxjError::new(format!(
                    "Entry not found: {DOCUMENT_SUMMARY_INFORMATION_STREAM}"
                )));
                DocumentSummaryInformation::default()
            }
            Some(document) => match DocumentSummaryInformation::new(&document.get_data()) {
                Some(value) => value,
                None => {
                    file.borrow_mut()
                        .add_ignored_error(MpxjError::new("Corrupt document summary information"));
                    DocumentSummaryInformation::default()
                }
            },
        };

        // Java:
        // Map<String, Object> customPropertiesMap = new TreeMap<>();
        // CustomProperties customProperties = documentSummaryInformation.getCustomProperties();
        // if (customProperties != null) { for (CustomProperty property : customProperties.properties()) { … } }
        let mut custom_properties_map: HashMap<String, FieldValue> = HashMap::new();
        for (name, value) in document_summary_information.get_custom_properties() {
            if let Some(value) = custom_property_field_value(value) {
                custom_properties_map.insert(name.clone(), value);
            }
        }

        // Java: ph.setResourcePoolFile(getResourcePool(props.getByteArray(PropsKey.RESOURCE_POOL)));
        let resource_pool = Self::get_resource_pool(props.get_byte_array(PropsKey::ResourcePool));

        //
        // The following is all the assignment after `ProjectProperties ph = file.getProjectProperties();` in Java.
        // The Rust property object is held by `ProjectFile`, so the whole process borrows it only once (avoids a re-entrant borrow panic).
        //
        {
            let mut file_ref = file.borrow_mut();
            let ph = file_ref.get_project_properties_mut();

            // Java: ph.setGUID(props.getUUID(PropsKey.GUID));
            ph.set_guid(props.get_uuid(PropsKey::Guid));
            // Java: ph.setStartDate(props.getTimestamp(PropsKey.PROJECT_START_DATE));
            ph.set_start_date(props.get_timestamp(PropsKey::ProjectStartDate));
            // Java: ph.setFinishDate(props.getTimestamp(PropsKey.PROJECT_FINISH_DATE));
            ph.set_finish_date(props.get_timestamp(PropsKey::ProjectFinishDate));
            // Java: ph.setScheduleFrom(ScheduleFrom.getInstance(1 - props.getShort(PropsKey.SCHEDULE_FROM)));
            ph.set_schedule_from(Some(ScheduleFrom::get_instance(
                1 - props.get_short(PropsKey::ScheduleFrom),
            )));
            // Java: ph.setDefaultStartTime(props.getTime(PropsKey.START_TIME));
            ph.set_default_start_time(props.get_time(PropsKey::StartTime));
            // Java: ph.setDefaultEndTime(props.getTime(PropsKey.END_TIME));
            ph.set_default_end_time(props.get_time(PropsKey::EndTime));
            // Java: ph.setStatusDate(props.getTimestamp(PropsKey.STATUS_DATE));
            ph.set_status_date(props.get_timestamp(PropsKey::StatusDate));
            // Java: ph.setHyperlinkBase(props.getUnicodeString(PropsKey.HYPERLINK_BASE));
            ph.set_hyperlink_base(props.get_unicode_string(PropsKey::HyperlinkBase));

            // Java: ph.setDefaultDurationUnits(MPPUtility.getDurationTimeUnits(props.getShort(PropsKey.DURATION_UNITS)));
            ph.set_default_duration_units(Some(MPPUtility::get_duration_time_units(
                props.get_short(PropsKey::DurationUnits),
            )));
            // Java: ph.setMinutesPerDay(Integer.valueOf(props.getInt(PropsKey.MINUTES_PER_DAY)));
            ph.set_minutes_per_day(Some(props.get_int(PropsKey::MinutesPerDay)));
            // Java: ph.setMinutesPerWeek(Integer.valueOf(props.getInt(PropsKey.MINUTES_PER_WEEK)));
            ph.set_minutes_per_week(Some(props.get_int(PropsKey::MinutesPerWeek)));
            // Java: ph.setDefaultOvertimeRate(new Rate(props.getDouble(PropsKey.OVERTIME_RATE), TimeUnit.HOURS));
            ph.set_default_overtime_rate(Some(Rate::new(
                props.get_double(PropsKey::OvertimeRate),
                TimeUnit::Hours,
            )));
            // Java: ph.setDefaultStandardRate(new Rate(props.getDouble(PropsKey.STANDARD_RATE), TimeUnit.HOURS));
            ph.set_default_standard_rate(Some(Rate::new(
                props.get_double(PropsKey::StandardRate),
                TimeUnit::Hours,
            )));
            // Java: ph.setDefaultWorkUnits(MPPUtility.getWorkTimeUnits(props.getShort(PropsKey.WORK_UNITS)));
            ph.set_default_work_units(Some(MPPUtility::get_work_time_units(props.get_short(
                PropsKey::WorkUnits,
            ))));
            // Java: ph.setSplitInProgressTasks(props.getBoolean(PropsKey.SPLIT_TASKS));
            ph.set_split_in_progress_tasks(props.get_boolean(PropsKey::SplitTasks));
            // Java: ph.setUpdatingTaskStatusUpdatesResourceStatus(props.getBoolean(PropsKey.TASK_UPDATES_RESOURCE));
            ph.set_updating_task_status_updates_resource_status(props.get_boolean(
                PropsKey::TaskUpdatesResource,
            ));
            // Java: ph.setCriticalSlackLimit(Duration.getInstance(props.getInt(PropsKey.CRITICAL_SLACK_LIMIT), TimeUnit.DAYS));
            ph.set_critical_slack_limit(Some(Duration::get_instance(
                f64::from(props.get_int(PropsKey::CriticalSlackLimit)),
                TimeUnit::Days,
            )));

            // Java: ph.setCurrencyDigits(Integer.valueOf(props.getShort(PropsKey.CURRENCY_DIGITS)));
            ph.set_currency_digits(Some(props.get_short(PropsKey::CurrencyDigits)));
            // Java: ph.setCurrencySymbol(props.getUnicodeString(PropsKey.CURRENCY_SYMBOL));
            ph.set_currency_symbol(props.get_unicode_string(PropsKey::CurrencySymbol));
            // Java: ph.setCurrencyCode(props.getUnicodeString(PropsKey.CURRENCY_CODE));
            ph.set_currency_code(props.get_unicode_string(PropsKey::CurrencyCode));
            // Java: ph.setDefaultTaskType(TaskTypeHelper.getInstance(props.getShort(PropsKey.DEFAULT_TASK_TYPE)));
            ph.set_default_task_type(Some(TaskTypeHelper::get_instance(props.get_short(
                PropsKey::DefaultTaskType,
            ))));
            // Java: ph.setSymbolPosition(MPPUtility.getSymbolPosition(props.getShort(PropsKey.CURRENCY_PLACEMENT)));
            ph.set_symbol_position(Some(MPPUtility::get_symbol_position(props.get_short(
                PropsKey::CurrencyPlacement,
            ))));
            // Java: ph.setWeekStartDay(DayOfWeekHelper.getInstance(props.getShort(PropsKey.WEEK_START_DAY) + 1));
            ph.set_week_start_day(DayOfWeekHelper::get_instance(
                props.get_short(PropsKey::WeekStartDay) + 1,
            ));
            // Java: ph.setFiscalYearStartMonth(Integer.valueOf(props.getShort(PropsKey.FISCAL_YEAR_START_MONTH)));
            ph.set_fiscal_year_start_month(Some(props.get_short(PropsKey::FiscalYearStartMonth)));
            // Java: ph.setFiscalYearStart(props.getShort(PropsKey.FISCAL_YEAR_START) == 1);
            ph.set_fiscal_year_start(props.get_short(PropsKey::FiscalYearStart) == 1);
            // Java: ph.setDaysPerMonth(Integer.valueOf(props.getShort(PropsKey.DAYS_PER_MONTH)));
            ph.set_days_per_month(Some(props.get_short(PropsKey::DaysPerMonth)));
            // Java: ph.setEditableActualCosts(props.getBoolean(PropsKey.EDITABLE_ACTUAL_COSTS));
            ph.set_editable_actual_costs(props.get_boolean(PropsKey::EditableActualCosts));
            // Java: ph.setHonorConstraints(!props.getBoolean(PropsKey.HONOR_CONSTRAINTS));
            ph.set_honor_constraints(!props.get_boolean(PropsKey::HonorConstraints));
            // Java: ph.setBaselineCalendarName(props.getUnicodeString(PropsKey.BASELINE_CALENDAR_NAME));
            ph.set_baseline_calendar_name(props.get_unicode_string(PropsKey::BaselineCalendarName));

            // Java: ph.setProjectTitle(getPopulatedValue(summaryInformation.getTitle(), () -> props.getUnicodeString(PropsKey.TITLE)));
            ph.set_project_title(Self::get_populated_value(
                summary_information.get_title(),
                || props.get_unicode_string(PropsKey::Title),
            ));
            ph.set_subject(Self::get_populated_value(
                summary_information.get_subject(),
                || props.get_unicode_string(PropsKey::Subject),
            ));
            ph.set_author(Self::get_populated_value(
                summary_information.get_author(),
                || props.get_unicode_string(PropsKey::Author),
            ));
            ph.set_keywords(Self::get_populated_value(
                summary_information.get_keywords(),
                || props.get_unicode_string(PropsKey::Keywords),
            ));
            ph.set_comments(Self::get_populated_value(
                summary_information.get_comments(),
                || props.get_unicode_string(PropsKey::Comments),
            ));
            // Java: ph.setTemplate(summaryInformation.getTemplate());
            ph.set_template(summary_information.get_template());
            // Java: ph.setLastAuthor(summaryInformation.getLastAuthor());
            ph.set_last_author(summary_information.get_last_author());
            // Java: ph.setRevision(NumberHelper.parseInteger(summaryInformation.getRevNumber()));
            ph.set_revision(NumberHelper::parse_integer(
                summary_information.get_rev_number().as_deref(),
            ));
            // Java: ph.setCreationDate(getLocalDateTime(summaryInformation.getCreateDateTime()));
            ph.set_creation_date(Self::get_local_date_time(
                summary_information.get_create_date_time(),
            ));
            // Java: ph.setLastSaved(getLocalDateTime(summaryInformation.getLastSaveDateTime()));
            ph.set_last_saved(Self::get_local_date_time(
                summary_information.get_last_save_date_time(),
            ));
            // Java: ph.setShortApplicationName(summaryInformation.getApplicationName());
            ph.set_short_application_name(summary_information.get_application_name());
            // Java: ph.setEditingTime(Integer.valueOf((int) summaryInformation.getEditTime()));
            ph.set_editing_time(Some(summary_information.get_edit_time() as i32));
            // Java: ph.setLastPrinted(getLocalDateTime(summaryInformation.getLastPrinted()));
            ph.set_last_printed(Self::get_local_date_time(
                summary_information.get_last_printed(),
            ));

            // Java: ph.setCategory(getPopulatedValue(documentSummaryInformation.getCategory(), () -> props.getUnicodeString(PropsKey.CATEGORY)));
            ph.set_category(Self::get_populated_value(
                document_summary_information.get_category(),
                || props.get_unicode_string(PropsKey::Category),
            ));
            // Java: ph.setPresentationFormat(documentSummaryInformation.getPresentationFormat());
            ph.set_presentation_format(document_summary_information.get_presentation_format());
            // Java: ph.setManager(getPopulatedValue(documentSummaryInformation.getManager(), () -> props.getUnicodeString(PropsKey.MANAGER)));
            ph.set_manager(Self::get_populated_value(
                document_summary_information.get_manager(),
                || props.get_unicode_string(PropsKey::Manager),
            ));
            // Java: ph.setCompany(getPopulatedValue(documentSummaryInformation.getCompany(), () -> props.getUnicodeString(PropsKey.COMPANY)));
            ph.set_company(Self::get_populated_value(
                document_summary_information.get_company(),
                || props.get_unicode_string(PropsKey::Company),
            ));
            // Java: ph.setContentType(documentSummaryInformation.getContentType());
            ph.set_content_type(document_summary_information.get_content_type());
            // Java: ph.setContentStatus(documentSummaryInformation.getContentStatus());
            ph.set_content_status(document_summary_information.get_content_status());
            // Java: ph.setLanguage(documentSummaryInformation.getLanguage());
            ph.set_language(document_summary_information.get_language());
            // Java: ph.setDocumentVersion(documentSummaryInformation.getDocumentVersion());
            ph.set_document_version(document_summary_information.get_document_version());

            // Java: ph.setCustomProperties(customPropertiesMap);
            ph.set_custom_properties(custom_properties_map);

            // Java: ph.setMultipleCriticalPaths(props.getBoolean(PropsKey.MULTIPLE_CRITICAL_PATHS));
            ph.set_multiple_critical_paths(props.get_boolean(PropsKey::MultipleCriticalPaths));

            // Java: ph.setBaselineDate(props.getTimestamp(PropsKey.BASELINE_DATE));
            ph.set_baseline_date(props.get_timestamp(PropsKey::BaselineDate));
            // Java: ph.setBaselineDate(1..10, props.getTimestamp(PropsKey.BASELINE1_DATE..BASELINE10_DATE));
            ph.set_baseline_date_for_index(1, props.get_timestamp(PropsKey::Baseline1Date));
            ph.set_baseline_date_for_index(2, props.get_timestamp(PropsKey::Baseline2Date));
            ph.set_baseline_date_for_index(3, props.get_timestamp(PropsKey::Baseline3Date));
            ph.set_baseline_date_for_index(4, props.get_timestamp(PropsKey::Baseline4Date));
            ph.set_baseline_date_for_index(5, props.get_timestamp(PropsKey::Baseline5Date));
            ph.set_baseline_date_for_index(6, props.get_timestamp(PropsKey::Baseline6Date));
            ph.set_baseline_date_for_index(7, props.get_timestamp(PropsKey::Baseline7Date));
            ph.set_baseline_date_for_index(8, props.get_timestamp(PropsKey::Baseline8Date));
            ph.set_baseline_date_for_index(9, props.get_timestamp(PropsKey::Baseline9Date));
            ph.set_baseline_date_for_index(10, props.get_timestamp(PropsKey::Baseline10Date));

            // Java: ph.setNewTasksAreManual(props.getBoolean(PropsKey.NEW_TASKS_ARE_MANUAL));
            ph.set_new_tasks_are_manual(props.get_boolean(PropsKey::NewTasksAreManual));

            // Java: ph.setResourcePoolFile(getResourcePool(props.getByteArray(PropsKey.RESOURCE_POOL)));
            ph.set_resource_pool_file(resource_pool);
        }

        // Java's three setters immediately update the context via ProjectFile; at this point the file borrow has been released.
        let context = file.borrow().get_project_context();
        let mut context_ref = context.try_borrow_mut().map_err(|_| {
            MpxjError::new("project context borrow conflict, cannot update duration units default value")
        })?;
        let defaults = context_ref.get_time_unit_defaults_mut();
        defaults.set_minutes_per_day(Some(props.get_int(PropsKey::MinutesPerDay)));
        defaults.set_minutes_per_week(Some(props.get_int(PropsKey::MinutesPerWeek)));
        defaults.set_days_per_month(Some(props.get_short(PropsKey::DaysPerMonth)));

        Ok(())
    }

    /// Extracts the resource pool file name from the resource pool data block.
    ///
    /// Corresponds to Java: `private String getResourcePool(byte[] data)`
    fn get_resource_pool(data: Option<&[u8]>) -> Option<String> {
        // Java: if (data == null) { return null; }
        let data = data?;

        // Java: int offset = 18; (18-byte block header)
        let mut offset = RESOURCE_POOL_HEADER_SIZE;
        // Java: if (offset + 4 >= data.length) { return null; }
        if offset + 4 >= data.len() {
            return None;
        }

        // Java: int length = ByteArrayHelper.getInt(data, offset); (length of the 8.3 file name)
        let length = read_i32(data, offset).unwrap_or(0);
        offset += 4;

        // Java: offset += length;
        if length < 0 {
            return None;
        }
        offset = offset.saturating_add(length as usize);

        // Java: offset += 34; (34-byte block header)
        offset = offset.saturating_add(RESOURCE_POOL_TRAILER_SIZE);
        // Java: if (offset >= data.length) { return null; }
        if offset >= data.len() {
            return None;
        }

        // Java: return MPPUtility.getUnicodeString(data, offset);
        Some(MPPUtility::get_unicode_string(data, offset))
    }

    /// Converts a `java.util.Date` to a local date-time.
    ///
    /// Corresponds to Java: `private LocalDateTime getLocalDateTime(Date date)`
    ///
    /// Java: `LocalDateTime.ofInstant(date.toInstant(), TimeZone.getDefault().toZoneId())`.
    /// On the Rust side, FILETIME is temporarily stored as a UTC [`NaiveDateTime`], then converted using the system default time zone.
    fn get_local_date_time(date: Option<NaiveDateTime>) -> Option<NaiveDateTime> {
        date.map(|value| Local.from_utc_datetime(&value).naive_local())
    }

    /// Prefers the first value and falls back to the supplier's value when it is empty.
    ///
    /// Corresponds to Java: `private String getPopulatedValue(String value, Supplier<String> supplier)`
    fn get_populated_value(
        value: Option<String>,
        supplier: impl FnOnce() -> Option<String>,
    ) -> Option<String> {
        // Java: if (value == null || value.isEmpty()) { value = supplier.get(); if (value != null && value.isEmpty()) { value = null; } }
        let value = match value {
            Some(value) if !value.is_empty() => return Some(value),
            _ => supplier(),
        };

        match value {
            Some(value) if value.is_empty() => None,
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpp::ole::PoifsFileSystem;
    use std::io::{Cursor, Write};

    use cfb::CompoundFile;

    /// FMTID of the property set stream (its content does not matter to this implementation; it only needs to exist).
    const FMTID: [u8; 16] = [0xAA; 16];

    /// A property to be written into a property set section.
    enum TestProperty {
        /// `VT_LPSTR` (bytes encoded in Windows-1252 or UTF-8).
        LpStr(Vec<u8>),
        /// `VT_I2` (unsigned form, for values such as code pages in the range 0..65535).
        U16(u16),
        /// `VT_I4`.
        I4(i32),
        /// `VT_FILETIME` (100-nanosecond count since 1601-01-01).
        FileTime(i64),
        /// `VT_BOOL`.
        Bool(bool),
    }

    /// Writes a test property into a buffer (excluding the 4-byte "offset relative to the start of the section").
    fn write_property(buffer: &mut Vec<u8>, property: &TestProperty) {
        match property {
            TestProperty::LpStr(value) => {
                buffer.extend_from_slice(&VT_LPSTR.to_le_bytes());
                buffer.extend_from_slice(&(value.len() as u32).to_le_bytes());
                buffer.extend_from_slice(value);
                // Consistent with MS-OLEPS: pad to a 4-byte boundary
                while buffer.len() % 4 != 0 {
                    buffer.push(0);
                }
            }
            TestProperty::U16(value) => {
                buffer.extend_from_slice(&VT_I2.to_le_bytes());
                buffer.extend_from_slice(&value.to_le_bytes());
                buffer.extend_from_slice(&[0, 0]);
            }
            TestProperty::I4(value) => {
                buffer.extend_from_slice(&VT_I4.to_le_bytes());
                buffer.extend_from_slice(&value.to_le_bytes());
            }
            TestProperty::FileTime(value) => {
                buffer.extend_from_slice(&VT_FILETIME.to_le_bytes());
                buffer.extend_from_slice(&value.to_le_bytes());
            }
            TestProperty::Bool(value) => {
                buffer.extend_from_slice(&VT_BOOL.to_le_bytes());
                buffer.extend_from_slice(&(if *value { 1i16 } else { 0i16 }).to_le_bytes());
                buffer.extend_from_slice(&[0, 0]);
            }
        }
    }

    /// Builds a property set stream: each item of `sections` is an ordered map of "property ID → property".
    fn property_set_stream(sections: &[Vec<(i32, TestProperty)>]) -> Vec<u8> {
        let mut header = vec![0u8; PROPERTY_SET_HEADER_SIZE];
        header[0..2].copy_from_slice(&0xFFFEu16.to_le_bytes());
        header[24..28].copy_from_slice(&(sections.len() as i32).to_le_bytes());

        let mut descriptors = Vec::new();
        let mut body = Vec::new();

        for section in sections {
            let section_offset = PROPERTY_SET_HEADER_SIZE + sections.len() * SECTION_DESCRIPTOR_SIZE + body.len();

            let mut values = Vec::new();
            let mut offsets = Vec::new();
            for (id, property) in section {
                offsets.push((*id, values.len() + 8 + section.len() * 8));
                write_property(&mut values, property);
            }

            let section_size = 8 + section.len() * 8 + values.len();
            let mut section_bytes = Vec::new();
            section_bytes.extend_from_slice(&(section_size as i32).to_le_bytes());
            section_bytes.extend_from_slice(&(section.len() as i32).to_le_bytes());
            for (id, offset) in &offsets {
                section_bytes.extend_from_slice(&id.to_le_bytes());
                section_bytes.extend_from_slice(&(*offset as u32).to_le_bytes());
            }
            section_bytes.extend_from_slice(&values);

            descriptors.extend_from_slice(&FMTID);
            descriptors.extend_from_slice(&(section_offset as u32).to_le_bytes());
            body.extend_from_slice(&section_bytes);
        }

        let mut stream = header;
        stream.extend_from_slice(&descriptors);
        stream.extend_from_slice(&body);
        stream
    }

    /// Bytes of a `VT_LPSTR` (Windows-1252, NUL-terminated).
    fn lpstr(value: &str) -> Vec<u8> {
        let mut bytes = value.as_bytes().to_vec();
        bytes.push(0);
        bytes
    }

    /// Computes the `FILETIME` corresponding to a point in time (100-nanosecond count since 1601-01-01).
    fn filetime(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> i64 {
        let target = NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|date| date.and_hms_opt(hour, minute, second))
            .expect("valid time");
        let epoch = NaiveDate::from_ymd_opt(1601, 1, 1)
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .expect("valid epoch");

        (target - epoch).num_seconds() * 10_000_000
    }

    /// Summary information: reads the title / author / revision number / timestamps / application name.
    #[test]
    fn summary_information_reads_standard_properties() {
        let stream = property_set_stream(&[vec![
            (PID_CODEPAGE, TestProperty::U16(1252)),
            (PIDSI_TITLE, TestProperty::LpStr(lpstr("My Project"))),
            (PIDSI_AUTHOR, TestProperty::LpStr(lpstr("Jon"))),
            (PIDSI_REVNUMBER, TestProperty::LpStr(lpstr("7"))),
            // 2024-01-02 03:04:05
            (
                PIDSI_CREATE_DTM,
                TestProperty::FileTime(filetime(2024, 1, 2, 3, 4, 5)),
            ),
            (PIDSI_APPNAME, TestProperty::LpStr(lpstr("Microsoft Project"))),
        ]]);

        let information = SummaryInformation::new(&stream).expect("property set is parseable");

        assert_eq!(information.get_title().as_deref(), Some("My Project"));
        assert_eq!(information.get_author().as_deref(), Some("Jon"));
        assert_eq!(information.get_rev_number().as_deref(), Some("7"));
        assert_eq!(
            information.get_create_date_time(),
            NaiveDate::from_ymd_opt(2024, 1, 2).and_then(|date| date.and_hms_opt(3, 4, 5))
        );
        assert_eq!(
            information.get_application_name().as_deref(),
            Some("Microsoft Project")
        );
        // Unwritten properties return None
        assert_eq!(information.get_subject(), None);
        assert_eq!(information.get_last_save_date_time(), None);
    }

    /// Corresponds to Java: `getLocalDateTime(Date)`; FILETIME represents a UTC instant, so properties should be converted using the system time zone.
    #[test]
    fn filetime_project_properties_use_local_time_zone() {
        let utc = NaiveDate::from_ymd_opt(2011, 6, 8)
            .and_then(|day| day.and_hms_opt(16, 24, 0))
            .expect("valid time");
        let expected = Local.from_utc_datetime(&utc).naive_local();
        assert_eq!(ProjectPropertiesReader::get_local_date_time(Some(utc)), Some(expected));
        assert_eq!(ProjectPropertiesReader::get_local_date_time(None), None);
        assert_eq!(
            custom_property_field_value(&PropertyValue::FileTime(utc)),
            Some(FieldValue::DateTime(expected))
        );
    }

    /// A `VT_LPSTR` with code page 65001 is decoded as UTF-8.
    #[test]
    fn summary_information_honours_the_codepage() {
        let stream = property_set_stream(&[vec![
            (PID_CODEPAGE, TestProperty::U16(65001)),
            (PIDSI_TITLE, TestProperty::LpStr(lpstr("中文标题"))),
        ]]);

        let information = SummaryInformation::new(&stream).expect("property set is parseable");
        assert_eq!(information.get_title().as_deref(), Some("中文标题"));
    }

    /// Windows-1251 Cyrillic bytes should match Java's code page decoding.
    #[test]
    fn lpstr_honours_windows_1251() {
        assert_eq!(decode_lpstr(&[b'\x77', b'\xF0', 0], 1251), "wр");
    }

    /// The declared code page is mapped item by item per POI's `codepageToEncoding` (bytes are all taken from the corresponding encoding).
    #[test]
    fn lpstr_honours_poi_code_page_mapping() {
        // GBK(936) and GB18030(54936)
        assert_eq!(decode_lpstr(&[0xD6, 0xD0, 0], 936), "中");
        assert_eq!(decode_lpstr(&[0xD6, 0xD0, 0], 54936), "中");
        // Shift_JIS(932):0x82 0xA0 → "あ"
        assert_eq!(decode_lpstr(&[0x82, 0xA0, 0], 932), "あ");
        // EUC-KR(949):0xB0 0xA1 → "가"
        assert_eq!(decode_lpstr(&[0xB0, 0xA1, 0], 949), "가");
        // Big5(950): 0xA4 0xA4 → "中"
        assert_eq!(decode_lpstr(&[0xA4, 0xA4, 0], 950), "中");
        // UTF-16BE(1201): 0x4E 0x2D → "中"
        assert_eq!(decode_lpstr(&[0x4E, 0x2D, 0, 0], 1201), "中");
        // windows-874(874):0xA1 → "ก"
        assert_eq!(decode_lpstr(&[0xA1, 0], 874), "ก");
        // ISO-8859-5(28595): 0xB0 → "А" (the same byte is "°" in cp1251)
        assert_eq!(decode_lpstr(&[0xB0, 0], 28595), "А");
        // KOI8-R(20866): 0xD2 → "р" (the same byte is "Т" in cp1251)
        assert_eq!(decode_lpstr(&[0xD2, 0], 20866), "р");
    }

    /// A code page without an `encoding_rs` implementation falls back to the default code page (Windows-1252) without panicking.
    #[test]
    fn lpstr_falls_back_to_the_default_code_page() {
        // cp037 (EBCDIC) has no implementation in encoding_rs
        assert_eq!(decode_lpstr(&[0xF0, 0], 37), "ð");
        // Unknown code pages also fall back to the default
        assert_eq!(decode_lpstr(&[0xF0, 0], 9999), "ð");
    }

    /// The name dictionary is decoded using the code page declared by the section (corresponds to POI `Section.readDictionary`).
    #[test]
    fn dictionary_names_use_the_section_codepage() {
        // One entry: ID=2, name length=3 (including trailing NUL), cp1251 bytes 0x77 0xF0 0x00 for the name "wр"
        let mut data = 1i32.to_le_bytes().to_vec();
        data.extend_from_slice(&2i32.to_le_bytes());
        data.extend_from_slice(&3i32.to_le_bytes());
        data.extend_from_slice(&[0x77, 0xF0, 0x00]);

        let mut names = Vec::new();
        on_dictionary_entries(&data, 1251, &mut |id, name| names.push((id, name.to_owned())));
        assert_eq!(names, vec![(2, "wр".to_owned())]);
    }

    /// `parse_section` records the code page declared by the section; falls back to the default when not declared.
    #[test]
    fn section_records_the_declared_codepage() {
        let stream = property_set_stream(&[vec![
            (PID_CODEPAGE, TestProperty::U16(1251)),
            (PIDSI_TITLE, TestProperty::LpStr(lpstr("x"))),
        ]]);
        let sections = parse_property_set(&stream).expect("property set is parseable");
        assert_eq!(sections[0].codepage, 1251);

        let stream = property_set_stream(&[vec![(PIDSI_TITLE, TestProperty::LpStr(lpstr("x")))]]);
        let sections = parse_property_set(&stream).expect("property set is parseable");
        assert_eq!(sections[0].codepage, DEFAULT_CODEPAGE);
    }

    /// Document summary information: standard properties + custom properties (string / integer / boolean).
    #[test]
    fn document_summary_information_reads_custom_properties() {
        let first_section = vec![
            (PID_CODEPAGE, TestProperty::U16(1252)),
            (PIDDSI_CATEGORY, TestProperty::LpStr(lpstr("Planning"))),
            (PIDDSI_MANAGER, TestProperty::LpStr(lpstr("Alice"))),
            (PIDDSI_COMPANY, TestProperty::LpStr(lpstr("Acme"))),
            (PIDDSI_CONTENTTYPE, TestProperty::LpStr(lpstr("Project"))),
            (PIDDSI_CONTENTSTATUS, TestProperty::LpStr(lpstr("Final"))),
            (PIDDSI_LANGUAGE, TestProperty::LpStr(lpstr("en-GB"))),
            (PIDDSI_DOCVERSION, TestProperty::LpStr(lpstr("1.2"))),
        ];
        let custom_section = raw_custom_properties_section(&[
            ("Checked by", TestProperty::LpStr(lpstr("Bob"))),
            ("Revision", TestProperty::I4(3)),
            ("Reviewed", TestProperty::Bool(true)),
        ]);
        let stream = property_set_stream_with_raw_section(&first_section, &custom_section);

        let information = DocumentSummaryInformation::new(&stream).expect("property set is parseable");

        assert_eq!(information.get_category().as_deref(), Some("Planning"));
        assert_eq!(information.get_manager().as_deref(), Some("Alice"));
        assert_eq!(information.get_company().as_deref(), Some("Acme"));
        assert_eq!(information.get_content_type().as_deref(), Some("Project"));
        assert_eq!(information.get_content_status().as_deref(), Some("Final"));
        assert_eq!(information.get_language().as_deref(), Some("en-GB"));
        assert_eq!(information.get_document_version().as_deref(), Some("1.2"));

        let custom = information.get_custom_properties();
        assert_eq!(
            custom.get("Checked by"),
            Some(&PropertyValue::String("Bob".to_owned()))
        );
        assert_eq!(custom.get("Revision"), Some(&PropertyValue::I4(3)));
        assert_eq!(custom.get("Reviewed"), Some(&PropertyValue::Bool(true)));
    }

    /// The custom property dictionary of real MPP documents is at property ID 0, not the code page property ID 1.
    #[test]
    fn real_mpp_custom_properties_are_preserved() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("mpxj/junit/data/mpp14assignmentcustom.mpp");
        let fs = PoifsFileSystem::new(std::fs::File::open(path).expect("open sample"))
            .expect("read compound document");
        let data = fs.get_root().get_document(DOCUMENT_SUMMARY_INFORMATION_STREAM)
            .expect("document summary property set");
        let info = DocumentSummaryInformation::new(&data.get_data()).expect("parse property set");
        assert_eq!(
            info.get_custom_properties().get("% Complete"),
            Some(&PropertyValue::String("0%".to_owned()))
        );
        assert_eq!(
            info.get_custom_properties().get("Duration"),
            Some(&PropertyValue::String("43 days".to_owned())),
            "parsed properties: {:?}", info.get_custom_properties()
        );
    }

    /// Java exposes `_PID_LINKBASE` as byte[] in sf259, so its raw bytes must not be discarded.
    #[test]
    fn real_mpp_binary_custom_property_is_preserved() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("mpxj/junit/data/generated/task-baselines/sf259.mpp");
        let fs = PoifsFileSystem::new(std::fs::File::open(path).expect("open sample"))
            .expect("read compound document");
        let data = fs.get_root().get_document(DOCUMENT_SUMMARY_INFORMATION_STREAM)
            .expect("document summary property set");
        let sections = parse_property_set(&data.get_data()).expect("parse property set");
        let section = sections.get(1).expect("custom properties section");
        let mut link_id = None;
        on_dictionary_entries(section.raw.get(&PID_DICTIONARY).expect("name dictionary"), section.codepage, &mut |id, name| {
            if name == "_PID_LINKBASE" { link_id = Some(id); }
        });
        let id = link_id.expect("_PID_LINKBASE");
        assert_eq!(
            section.properties.get(&id),
            Some(&PropertyValue::Bytes(vec![2, 0, 0, 0, 0, 0]))
        );
    }

    /// Assembles "custom properties" (name + value) into the bytes of one section.
    ///
    /// Property IDs are assigned sequentially starting from 2; property 0 is the name dictionary without a type header, property 1 is the code page.
    fn raw_custom_properties_section(properties: &[(&str, TestProperty)]) -> Vec<u8> {
        // Name dictionary: entry count + (property ID, name byte count, name bytes), no padding between entries.
        let mut name_dictionary = Vec::new();
        name_dictionary.extend_from_slice(&(properties.len() as i32).to_le_bytes());
        for (index, (name, _)) in properties.iter().enumerate() {
            let mut name_bytes = name.as_bytes().to_vec();
            name_bytes.push(0);
            name_dictionary.extend_from_slice(&((index + 2) as i32).to_le_bytes());
            name_dictionary.extend_from_slice(&(name_bytes.len() as i32).to_le_bytes());
            name_dictionary.extend_from_slice(&name_bytes);
        }

        // Value area: property 0 has no type header; property 1 is the code page; the rest are custom properties.
        let mut values = Vec::new();
        let mut offsets = Vec::new();
        offsets.push(values.len());
        values.extend_from_slice(&name_dictionary);
        offsets.push(values.len());
        write_property(&mut values, &TestProperty::U16(1252));
        for (_, property) in properties {
            offsets.push(values.len());
            write_property(&mut values, property);
        }

        let mut ids = vec![PID_DICTIONARY, PID_CODEPAGE];
        ids.extend((0..properties.len()).map(|index| (index + 2) as i32));

        let mut section = Vec::new();
        let header = 8 + ids.len() * 8;
        let size = header + values.len();
        section.extend_from_slice(&(size as i32).to_le_bytes());
        section.extend_from_slice(&(ids.len() as i32).to_le_bytes());
        for (id, offset) in ids.iter().zip(offsets.iter()) {
            section.extend_from_slice(&id.to_le_bytes());
            section.extend_from_slice(&(*offset as u32 + header as u32).to_le_bytes());
        }
        section.extend_from_slice(&values);
        section
    }

    /// Assembles a property set stream from a "test property section" and "raw section bytes".
    fn property_set_stream_with_raw_section(
        first: &[(i32, TestProperty)],
        raw_section: &[u8],
    ) -> Vec<u8> {
        let mut header = vec![0u8; PROPERTY_SET_HEADER_SIZE];
        header[0..2].copy_from_slice(&0xFFFEu16.to_le_bytes());
        header[24..28].copy_from_slice(&2i32.to_le_bytes());

        let mut descriptors = Vec::new();
        let first_offset = PROPERTY_SET_HEADER_SIZE + 2 * SECTION_DESCRIPTOR_SIZE;

        let mut values = Vec::new();
        let mut offsets = Vec::new();
        for (id, property) in first {
            offsets.push((*id, values.len() + 8 + first.len() * 8));
            write_property(&mut values, property);
        }
        let mut first_section = Vec::new();
        first_section.extend_from_slice(&((8 + first.len() * 8 + values.len()) as i32).to_le_bytes());
        first_section.extend_from_slice(&(first.len() as i32).to_le_bytes());
        for (id, offset) in &offsets {
            first_section.extend_from_slice(&id.to_le_bytes());
            first_section.extend_from_slice(&(*offset as u32).to_le_bytes());
        }
        first_section.extend_from_slice(&values);

        let second_offset = first_offset + first_section.len();

        descriptors.extend_from_slice(&FMTID);
        descriptors.extend_from_slice(&(first_offset as u32).to_le_bytes());
        descriptors.extend_from_slice(&FMTID);
        descriptors.extend_from_slice(&(second_offset as u32).to_le_bytes());

        let mut stream = header;
        stream.extend_from_slice(&descriptors);
        stream.extend_from_slice(&first_section);
        stream.extend_from_slice(raw_section);
        stream
    }

    /// An invalid property set stream returns `None` without panicking.
    #[test]
    fn invalid_property_sets_return_none() {
        assert!(SummaryInformation::new(&[]).is_none());
        assert!(SummaryInformation::new(&[0u8; 4]).is_none());
        // Big-endian byte order is not supported
        let mut big_endian = vec![0u8; PROPERTY_SET_HEADER_SIZE];
        big_endian[0..2].copy_from_slice(&0xFEFFu16.to_le_bytes());
        assert!(SummaryInformation::new(&big_endian).is_none());
        // Section count is negative
        let mut negative = vec![0u8; PROPERTY_SET_HEADER_SIZE];
        negative[0..2].copy_from_slice(&0xFFFEu16.to_le_bytes());
        negative[24..28].copy_from_slice(&(-1i32).to_le_bytes());
        assert!(SummaryInformation::new(&negative).is_none());
        // Section offset out of bounds (returns an empty section instead of panicking)
        let mut out_of_range = vec![0u8; PROPERTY_SET_HEADER_SIZE + SECTION_DESCRIPTOR_SIZE];
        out_of_range[0..2].copy_from_slice(&0xFFFEu16.to_le_bytes());
        out_of_range[24..28].copy_from_slice(&1i32.to_le_bytes());
        out_of_range[PROPERTY_SET_HEADER_SIZE + 16..PROPERTY_SET_HEADER_SIZE + 20]
            .copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        let information = SummaryInformation::new(&out_of_range).expect("returns empty properties");
        assert_eq!(information.get_title(), None);
    }

    /// `getPopulatedValue`: prefers the first value and falls back to the supplier when it is empty.
    #[test]
    fn populated_value_falls_back_to_the_supplier() {
        assert_eq!(
            ProjectPropertiesReader::get_populated_value(Some("a".to_owned()), || Some("b".to_owned())),
            Some("a".to_owned())
        );
        assert_eq!(
            ProjectPropertiesReader::get_populated_value(Some(String::new()), || Some("b".to_owned())),
            Some("b".to_owned())
        );
        assert_eq!(
            ProjectPropertiesReader::get_populated_value(None, || Some("b".to_owned())),
            Some("b".to_owned())
        );
        // An empty string from the supplier is treated as "no value"
        assert_eq!(
            ProjectPropertiesReader::get_populated_value(None, || Some(String::new())),
            None
        );
        assert_eq!(
            ProjectPropertiesReader::get_populated_value(Some("a".to_owned()), || None),
            Some("a".to_owned())
        );
    }

    /// Resource pool file name parsing (including out-of-bounds and missing data).
    #[test]
    fn resource_pool_name_is_extracted() {
        // 18-byte header + 4-byte length + 8.3 file name + 34-byte header + UTF-16LE path
        let mut data = vec![0u8; RESOURCE_POOL_HEADER_SIZE];
        let short_name = b"POOL.MPP";
        data.extend_from_slice(&(short_name.len() as i32).to_le_bytes());
        data.extend_from_slice(short_name);
        data.extend_from_slice(&[0u8; RESOURCE_POOL_TRAILER_SIZE]);
        for unit in "\\\\server\\pool.mpp".encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00]);

        assert_eq!(
            ProjectPropertiesReader::get_resource_pool(Some(&data)).as_deref(),
            Some("\\\\server\\pool.mpp")
        );

        // Missing / too-short data
        assert_eq!(ProjectPropertiesReader::get_resource_pool(None), None);
        assert_eq!(ProjectPropertiesReader::get_resource_pool(Some(&[0u8; 10])), None);
    }

    /// Drives `process` with a real compound document: properties come from `Props` and the two property set streams.
    #[test]
    fn process_reads_props_and_property_sets() {
        let summary_stream = property_set_stream(&[vec![
            (PID_CODEPAGE, TestProperty::U16(1252)),
            (PIDSI_TITLE, TestProperty::LpStr(lpstr("From summary"))),
            (PIDSI_AUTHOR, TestProperty::LpStr(lpstr("Author"))),
        ]]);
        let raw_custom =
            raw_custom_properties_section(&[("Checked by", TestProperty::LpStr(lpstr("Bob")))]);
        let document_summary_stream = property_set_stream_with_raw_section(
            &[
                (PID_CODEPAGE, TestProperty::U16(1252)),
                (PIDDSI_COMPANY, TestProperty::LpStr(lpstr("Acme"))),
            ],
            &raw_custom,
        );

        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        compound
            .create_stream(SUMMARY_INFORMATION_STREAM)
            .unwrap()
            .write_all(&summary_stream)
            .unwrap();
        compound
            .create_stream(DOCUMENT_SUMMARY_INFORMATION_STREAM)
            .unwrap()
            .write_all(&document_summary_stream)
            .unwrap();
        let bytes = compound.into_inner().into_inner();

        let fs = PoifsFileSystem::new(Cursor::new(bytes)).unwrap();
        let root = fs.get_root();

        let mut props = Props::new();
        props.put(PropsKey::Title.get_value(), utf16("From props"));
        props.put(PropsKey::Subject.get_value(), utf16("Subject"));
        props.put(PropsKey::Manager.get_value(), utf16("Manager"));
        props.put(PropsKey::MinutesPerDay.get_value(), 450i32.to_le_bytes().to_vec());
        props.put(PropsKey::MinutesPerWeek.get_value(), 2250i32.to_le_bytes().to_vec());
        props.put(PropsKey::DaysPerMonth.get_value(), 22u16.to_le_bytes().to_vec());

        let file = ProjectFile::new();
        ProjectPropertiesReader
            .process(&file, &props, &root)
            .expect("project properties read successfully");

        let file_ref = file.borrow();
        let properties = file_ref.get_project_properties();
        // The summary information has a title → do not use the Props title (Java's getPopulatedValue)
        assert_eq!(properties.get_project_title().as_deref(), Some("From summary"));
        assert_eq!(properties.get_author().as_deref(), Some("Author"));
        // The summary information has no subject / manager → fall back to Props
        assert_eq!(properties.get_subject().as_deref(), Some("Subject"));
        assert_eq!(properties.get_manager().as_deref(), Some("Manager"));
        assert_eq!(properties.get_company().as_deref(), Some("Acme"));
        assert_eq!(properties.get_minutes_per_day(), Some(450));
        assert_eq!(properties.get_minutes_per_week(), Some(2250));
        assert_eq!(properties.get_minutes_per_month(), Some(9900));
        assert_eq!(properties.get_days_per_month(), Some(22));
        // Custom document properties
        assert_eq!(
            properties.get_custom_properties().get("Checked by"),
            Some(&FieldValue::String("Bob".to_owned()))
        );
    }

    /// When both property set streams are missing, logs "ignore error" and still reads `Props`.
    #[test]
    fn missing_property_sets_are_recorded_as_ignored_errors() {
        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        compound.create_storage("/   114").unwrap();
        let bytes = compound.into_inner().into_inner();
        let fs = PoifsFileSystem::new(Cursor::new(bytes)).unwrap();

        let mut props = Props::new();
        props.put(PropsKey::Title.get_value(), utf16("From props"));

        let file = ProjectFile::new();
        ProjectPropertiesReader
            .process(&file, &props, &fs.get_root())
            .expect("missing property set is not an error");

        let file_ref = file.borrow();
        assert_eq!(
            file_ref.get_project_properties().get_project_title().as_deref(),
            Some("From props")
        );
        assert_eq!(file_ref.get_ignored_errors().len(), 2);
    }

    /// UTF-16LE (NUL-terminated) encoding, for use by `Props`.
    fn utf16(value: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in value.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00]);
        data
    }
}
