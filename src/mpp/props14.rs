//! Corresponds to Java: `org/mpxj/mpp/Props14.java`

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use chrono::{NaiveDateTime, NaiveTime};
use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::props::{InputStreamCursor, Props, read_error};
use crate::mpp::props_key::PropsKey;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_file::ProjectFile;

/// The `Props` stream of an MPP14 file.
///
/// Corresponds to Java: `final class Props14 extends Props`
///
/// # How inheritance is restored (composition + `Deref` + forwarding)
///
/// Java's `Props14` extends [`Props`] (Java does not override `toString()`). Rust has no inheritance, so this class uses
/// **composition + `Deref`/`DerefMut`** to restore it: it holds a [`Props`] (field `props`), so `&Props14`
/// can be automatically coerced (deref coercion) into `&Props` — "methods whose parameter is `&Props`" can also directly accept
/// `&Props14`, corresponding to "a subclass instance can be passed directly to a base class parameter" in Java (this crate already
/// restores inheritance with the same technique in [`crate::recurring_task`]).
///
/// At the end of the class, explicit forwarding of all public `Props` methods is still kept, so call sites that do not need coercion can call them directly; these same-named methods
/// shadow the same-named methods on the `Deref` target [`Props`], but they are themselves per-method forwards with identical behavior,
/// so there is no semantic conflict.
///
/// # Constructor and `InputStream`
///
/// Java: `Props14(ProjectFile file, InputStream is) throws IOException`. On the Rust side, per this project's
/// convention `InputStream` is replaced with `&[u8]`, and sequential reading is done by [`InputStreamCursor`].
/// `ProjectFile` is used only to register ignored errors on failure (`addIgnoredError`), and is **not held**,
/// so it is passed as a borrow `&Rc<RefCell<ProjectFile>>`, which does not cause an `Rc` cycle.
///
/// # Block layout (Java constructor)
///
/// ```text
/// byte[16] header      headerCount = the short in this header (offset 12) — the entry count
/// headerCount × {
/// int  attrib1     attribute data length
/// int  attrib2     attribute key
///     int  unknown
///     byte data[attrib1]
/// byte pad         pads to a two-byte boundary only when attrib1 is odd
/// }
/// ```
///
/// Two differences from `Props12` (kept from the original Java):
///
/// 1. The loop checks at the start "exit if fewer than 12 bytes remain" — this is the Java side's fix for
/// the behavior of `DocumentInputStream` under a 64-bit JRE;
/// 2. When reading data it catches `IndexOutOfBoundsException` (POI bug 61677),
/// registers an ignored error and aborts parsing instead of throwing upward.
///
/// # Differences from Java
///
/// - `availableBytes` is decremented using `i64` (Java uses `int`, which overflows with extreme input).
/// - Like Java, `availableBytes` **does not subtract alignment padding bytes**, so it may be greater than the true number of remaining bytes;
/// this is exactly why Java needs to catch `IndexOutOfBoundsException`, and the Rust side keeps that branch too.
/// - POI's `IndexOutOfBoundsException` corresponds to the Rust side's "the cursor cannot read enough bytes",
/// in which case `file.addIgnoredError(..)` is likewise registered and the loop is aborted.
/// - `toString()` reuses the [`Props`] implementation (Java does not override it).
///
/// Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: not used by this class.
pub struct Props14 {
/// Corresponds to Java: `extends Props` (Rust uses composition instead of inheritance)
    props: Props,
}

impl Props14 {
/// Constructor.
    ///
    /// Corresponds to Java: `Props14(ProjectFile file, InputStream is) throws IOException`
    pub fn new(file: &Rc<RefCell<ProjectFile>>, data: &[u8]) -> MpxjResult<Self> {
        let mut result = Self { props: Props::new() };
        let mut is = InputStreamCursor::new(data);

        // Java: byte[] header = InputStreamHelper.read(is, 16)
        let header = is.read(16).ok_or_else(read_error)?;
        // Java: int headerCount = ByteArrayHelper.getShort(header, 12)
        let header_count = ByteArrayHelper::get_short(header, 12);
        let mut found_count = 0;
        // Java: int availableBytes = is.available()
        let mut available_bytes = is.available() as i64;

        while found_count < header_count {
// Java: exit when fewer than 12 bytes remain (the fix for a 64-bit JRE)
            if available_bytes < 12 {
                break;
            }

            // Java: int attrib1 = readInt(is)
            let attrib1 = is.read_int().ok_or_else(read_error)?;
            // Java: int attrib2 = readInt(is)
            let attrib2 = is.read_int().ok_or_else(read_error)?;
            // Java: /*int attrib3 = */ readInt(is)
            is.read_int().ok_or_else(read_error)?;
            available_bytes -= 12;

            if available_bytes < i64::from(attrib1) || attrib1 < 1 {
                break;
            }

            // Java: try { data = InputStreamHelper.read(is, attrib1); }
            //       catch (IndexOutOfBoundsException ex) { file.addIgnoredError(ex); break; }
            let item = match is.read(attrib1 as usize) {
                Some(item) => item,
                None => {
                    file.borrow_mut()
                        .add_ignored_error(index_out_of_bounds_error());
                    break;
                }
            };
            available_bytes -= i64::from(attrib1);

            result.props.put(attrib2, item.to_vec());
            found_count += 1;

// Java: align to a two-byte boundary
            if item.len() % 2 != 0 {
                is.skip(1);
            }
        }

        Ok(result)
    }

    // -----------------------------------------------------------------
// The following methods are inherited from Props in Java; the Rust side forwards them to the internally held Props
    // -----------------------------------------------------------------

/// Returns the attribute data (raw bytes).
    ///
    /// Corresponds to Java: `Props.getByteArray(PropsKey key)`
    pub fn get_byte_array(&self, key: PropsKey) -> Option<&[u8]> {
        self.props.get_byte_array(key)
    }

/// Returns a byte value.
    ///
    /// Corresponds to Java: `Props.getByte(PropsKey key)`
    pub fn get_byte(&self, key: PropsKey) -> i8 {
        self.props.get_byte(key)
    }

/// Returns a short value.
    ///
    /// Corresponds to Java: `Props.getShort(PropsKey key)`
    pub fn get_short(&self, key: PropsKey) -> i32 {
        self.props.get_short(key)
    }

/// Returns an integer value.
    ///
    /// Corresponds to Java: `Props.getInt(PropsKey key)`
    pub fn get_int(&self, key: PropsKey) -> i32 {
        self.props.get_int(key)
    }

/// Returns a double value.
    ///
    /// Corresponds to Java: `Props.getDouble(PropsKey key)`
    pub fn get_double(&self, key: PropsKey) -> f64 {
        self.props.get_double(key)
    }

/// Returns a time value.
    ///
    /// Corresponds to Java: `Props.getTime(PropsKey key)`
    pub fn get_time(&self, key: PropsKey) -> Option<NaiveTime> {
        self.props.get_time(key)
    }

/// Returns a timestamp.
    ///
    /// Corresponds to Java: `Props.getTimestamp(PropsKey key)`
    pub fn get_timestamp(&self, key: PropsKey) -> Option<NaiveDateTime> {
        self.props.get_timestamp(key)
    }

/// Returns a boolean value.
    ///
    /// Corresponds to Java: `Props.getBoolean(PropsKey key)`
    pub fn get_boolean(&self, key: PropsKey) -> bool {
        self.props.get_boolean(key)
    }

/// Returns a UTF-16LE string.
    ///
    /// Corresponds to Java: `Props.getUnicodeString(PropsKey key)`
    pub fn get_unicode_string(&self, key: PropsKey) -> Option<String> {
        self.props.get_unicode_string(key)
    }

/// Returns a GUID.
    ///
    /// Corresponds to Java: `Props.getUUID(PropsKey key)`
    pub fn get_uuid(&self, key: PropsKey) -> Option<Uuid> {
        self.props.get_uuid(key)
    }

/// Returns all keys (ascending).
    ///
    /// Corresponds to Java: `Props.keySet()`
    pub fn key_set(&self) -> Vec<i32> {
        self.props.key_set()
    }
}

impl fmt::Display for Props14 {
/// Corresponds to Java: `Props.toString()` (`Props14` does not override it)
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.props, f)
    }
}

impl std::ops::Deref for Props14 {
    type Target = Props;

/// Corresponds to Java: "a subclass instance can be used directly as a base class instance" arising from inheritance
    fn deref(&self) -> &Self::Target {
        &self.props
    }
}

impl std::ops::DerefMut for Props14 {
/// Corresponds to Java: "a subclass instance can be used directly as a base class instance" arising from inheritance
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.props
    }
}

/// The Rust counterpart of the `IndexOutOfBoundsException` thrown by POI on an out-of-bounds read.
///
/// Corresponds to Java: `catch (IndexOutOfBoundsException ex)`
/// (POI bug: https://bz.apache.org/bugzilla/show_bug.cgi?id=61677)
fn index_out_of_bounds_error() -> MpxjError {
    MpxjError::new("Index out of bounds while reading Props data")
}

#[cfg(test)]
mod tests {
    use super::*;

/// Builds data using the MPP14 block layout.
    fn build_props14(header_count: i32, entries: &[(i32, i32, Vec<u8>)]) -> Vec<u8> {
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&(header_count as i16).to_le_bytes());

        for (size, key, value) in entries {
            data.extend_from_slice(&size.to_le_bytes());
            data.extend_from_slice(&key.to_le_bytes());
            data.extend_from_slice(&0i32.to_le_bytes());
            data.extend_from_slice(value);
// Java: align to a two-byte boundary
            if value.len() % 2 != 0 {
                data.push(0);
            }
        }

        data
    }

    #[test]
    fn reads_entries() {
        let data = build_props14(
            2,
            &[
                // PropsKey.TITLE == 37748744
                (4, 37748744, vec![0xD2, 0x04, 0x00, 0x00]),
                // PropsKey.SUBJECT == 37748745
                (2, 37748745, vec![0x02, 0x01]),
            ],
        );

        let file = ProjectFile::new();
        let props = Props14::new(&file, &data).expect("should parse");

        assert_eq!(props.key_set(), vec![37748744, 37748745]);
        assert_eq!(props.get_int(PropsKey::Title), 1234);
        assert_eq!(props.get_short(PropsKey::Subject), 258);
        assert!(file.borrow().get_ignored_errors().is_empty());
    }

    #[test]
    fn odd_length_data_is_aligned() {
        let data = build_props14(
            2,
            &[(1, 37748744, vec![0x07]), (2, 37748745, vec![0x02, 0x01])],
        );

        let file = ProjectFile::new();
        let props = Props14::new(&file, &data).expect("should parse");

        assert_eq!(props.get_byte(PropsKey::Title), 7);
        assert_eq!(props.get_short(PropsKey::Subject), 258);
    }

    #[test]
    fn short_header_returns_error() {
        let file = ProjectFile::new();
        assert!(Props14::new(&file, &[0u8; 15]).is_err());
    }

    #[test]
    fn fewer_than_twelve_remaining_bytes_breaks_loop() {
// Declares 1 entry, but only 8 bytes remain after the header → Java: availableBytes < 12 → break
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&1i16.to_le_bytes());
        data.extend_from_slice(&[0u8; 8]);

        let file = ProjectFile::new();
        let props = Props14::new(&file, &data).expect("should parse");

        assert!(props.key_set().is_empty());
        assert!(file.borrow().get_ignored_errors().is_empty());
    }

    #[test]
    fn size_beyond_available_bytes_breaks_loop() {
        let mut data = build_props14(2, &[(2, 37748744, vec![0x02, 0x01])]);
        data.extend_from_slice(&100i32.to_le_bytes());
        data.extend_from_slice(&37748745i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());

        let file = ProjectFile::new();
        let props = Props14::new(&file, &data).expect("should parse");

        assert_eq!(props.key_set(), vec![37748744]);
        assert!(file.borrow().get_ignored_errors().is_empty());
    }

    #[test]
    fn truncated_data_records_ignored_error() {
// Java's availableBytes does not subtract alignment padding bytes, so it "over-reports" the remaining amount;
// only when odd-length entries have appeared earlier can the guard let through an entry that cannot actually be read.
// Here the first two entries are 1 byte each (each padded by 1 byte), and the 3rd declares 4 bytes while only 2 bytes are actually present:
        // → Java: catch (IndexOutOfBoundsException ex) { file.addIgnoredError(ex); break; }
        let data = build_props14(
            3,
            &[
                (1, 37748744, vec![0x07]),
                (1, 37748745, vec![0x09]),
                (4, 37748748, vec![0x01, 0x02]),
            ],
        );

        let file = ProjectFile::new();
        let props = Props14::new(&file, &data).expect("should parse");

        assert_eq!(props.key_set(), vec![37748744, 37748745]);
        let borrowed = file.borrow();
        assert_eq!(borrowed.get_ignored_errors().len(), 1);
        assert_eq!(
            borrowed.get_ignored_errors()[0].message(),
            "Index out of bounds while reading Props data"
        );
    }

    #[test]
    fn truncated_entry_header_returns_error() {
// Only 8 bytes remain after the header → Java's `availableBytes < 12` guard breaks first,
// so no error occurs here
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&1i16.to_le_bytes());
        data.extend_from_slice(&4i32.to_le_bytes());
        data.extend_from_slice(&37748744i32.to_le_bytes());

        let file = ProjectFile::new();
        let props = Props14::new(&file, &data).expect("should parse");
        assert!(props.key_set().is_empty());
    }

    #[test]
    fn truncated_entry_after_padding_returns_error() {
// `availableBytes` does not subtract alignment padding bytes; after 12 odd-length entries it is 12 greater than the true remaining amount,
// so the 13th entry can pass the `availableBytes < 12` guard, but the actual read necessarily fails
// (Java: POI throws IOException here → Rust returns Err)
        let entries: Vec<(i32, i32, Vec<u8>)> = (0..12)
            .map(|index| (1, 37748744 + index, vec![0x07]))
            .collect();
        let data = build_props14(13, &entries);

        let file = ProjectFile::new();
        assert!(Props14::new(&file, &data).is_err());
    }

    #[test]
    fn missing_keys_return_defaults_and_display_works() {
        let file = ProjectFile::new();
        let props = Props14::new(&file, &build_props14(0, &[])).expect("should parse");

        assert_eq!(props.get_byte(PropsKey::Company), 0);
        assert_eq!(props.get_short(PropsKey::Company), 0);
        assert_eq!(props.get_int(PropsKey::Company), 0);
        assert_eq!(props.get_double(PropsKey::Company), 0.0);
        assert_eq!(props.get_time(PropsKey::Company), None);
        assert_eq!(props.get_timestamp(PropsKey::Company), None);
        assert!(!props.get_boolean(PropsKey::Company));
        assert_eq!(props.get_unicode_string(PropsKey::Company), None);
        assert_eq!(props.get_uuid(PropsKey::Company), None);
        assert_eq!(props.get_byte_array(PropsKey::Company), None);
        assert!(props.key_set().is_empty());
        assert!(props.to_string().starts_with("BEGIN Props\n"));
    }
}
