//! Corresponds to Java: `org/mpxj/mpp/Props8.java`

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use chrono::{NaiveDateTime, NaiveTime};
use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::props::{InputStreamCursor, Props, read_error};
use crate::mpp::props_key::PropsKey;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;

/// The `Props` stream of an MPP8 file.
///
/// Corresponds to Java: `final class Props8 extends Props`
///
/// MPP8's `Props` is a set of properties keyed by integers; its format is not fully
/// understood, so the Java side wraps the entire parsing process in a `try`: on a read
/// error it records `addIgnoredError` and sets `m_complete` to `false`, while properties already read are retained.
///
/// # Reconstructing inheritance (composition + `Deref` + forwarding)
///
/// Java's `Props8` extends [`Props`]; Rust has no inheritance, so this class reconstructs it with **composition + `Deref`/`DerefMut`**:
/// it holds a [`Props`] (field `props`), so `&Props8` can be automatically coerced (deref coercion) into
/// `&Props` -- "a method whose parameter is `&Props`" can also directly accept `&Props8`, corresponding to "a subclass instance
/// can be passed directly to a base class parameter" in Java (this crate already reconstructs inheritance the same way in [`crate::recurring_task`]).
///
/// The end of the class still keeps explicit forwarding for all public methods of `Props`, for call sites that do not need the coercion to call directly; these same-named methods
/// shadow the same-named methods on the `Deref` target [`Props`], but they are themselves per-method forwards with identical behavior,
/// so there is no semantic conflict.
///
/// # Constructor and `InputStream`
///
/// Java: `Props8(ProjectFile file, InputStream is)`. On the Rust side, per this project's convention,
/// `InputStream` is replaced with `&[u8]`, and sequential reading is done by [`InputStreamCursor`].
/// `ProjectFile` is used only to register ignored errors on failure (`addIgnoredError`) and is **not held**,
/// so it is passed by borrow as `&Rc<RefCell<ProjectFile>>`, which does not cause an `Rc` cycle.
///
/// # Block layout (Java constructor)
///
/// ```text
/// int   file size
/// int   file size (repeated)
/// int   unknown
/// short count              entry count
/// short unknown
/// count × {
/// int   attrib1        length candidate (used as the length when attrib3 == 64)
/// byte  attrib[0..4]   attrib2 = the little-endian integer of these 4 bytes; attrib3 = attrib[2]
/// int   attrib5        length candidate (used as the length when attrib3 != 64; when equal to 65536 the length is fixed at 4)
/// byte  data[size]     attribute data
/// byte  pad            pad to a two-byte boundary only when the data length is odd
/// }
/// ```
///
/// # Differences from Java
///
/// - Java's `IOException` branch (`file.addIgnoredError(ex); m_complete = false;`) is triggered by
/// Rust's "cursor out of bounds returns `None`", so the constructor does not return a `Result` (consistent with Java,
/// the exception is swallowed internally).
/// - In `toString()`, the platform newline of `PrintWriter.println` is unified to `\n`.
///
/// Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: not used by this class.
pub struct Props8 {
/// Corresponds to Java: `extends Props` (Rust uses composition instead of inheritance)
    props: Props,
    /// Corresponds to Java: `private boolean m_complete = true`
    complete: bool,
}

impl Props8 {
/// Constructor.
    ///
    /// Corresponds to Java: `Props8(ProjectFile file, InputStream is)`
    pub fn new(file: &Rc<RefCell<ProjectFile>>, data: &[u8]) -> Self {
        let mut result = Self {
            props: Props::new(),
            complete: true,
        };

        if let Err(error) = result.parse(data) {
            // Java: catch (IOException ex) { file.addIgnoredError(ex); m_complete = false; }
            file.borrow_mut().add_ignored_error(error);
            result.complete = false;
        }

        result
    }

/// Whether the attribute data has been read completely.
    ///
/// On the Java side `m_complete` is a private field exposed only in `toString()`; the Rust side likewise does not expose it.
    #[cfg(test)]
    fn is_complete(&self) -> bool {
        self.complete
    }

/// Parse the attribute data (corresponds to all logic inside the `try` block of the Java constructor).
    fn parse(&mut self, data: &[u8]) -> MpxjResult<()> {
        let mut is = InputStreamCursor::new(data);

        // Java: readInt(is) —— File size
        is.read_int().ok_or_else(read_error)?;
        // Java: readInt(is) —— Repeat of file size
        is.read_int().ok_or_else(read_error)?;
        // Java: readInt(is) —— unknown
        is.read_int().ok_or_else(read_error)?;
        // Java: int count = readShort(is) —— Number of entries
        let count = is.read_short().ok_or_else(read_error)?;
        // Java: readShort(is) —— unknown
        is.read_short().ok_or_else(read_error)?;

        for _ in 0..count {
            // Java: int attrib1 = readInt(is)
            let attrib1 = is.read_int().ok_or_else(read_error)?;

            // Java: InputStreamHelper.read(is, attrib)
            let attrib = is.read(4).ok_or_else(read_error)?;
            // Java: int attrib2 = ByteArrayHelper.getInt(attrib, 0)
            let attrib2 = ByteArrayHelper::get_int(attrib, 0);
// Java: int attrib3 = MPPUtility.getByte(attrib, 2) -- unsigned value
            let attrib3 = i32::from(attrib[2]);
            // Java: int attrib5 = readInt(is)
            let attrib5 = is.read_int().ok_or_else(read_error)?;

            let mut size = if attrib3 == 64 {
                attrib1
            } else {
                attrib5
            };

            if attrib5 == 65536 {
                size = 4;
            }

            if size <= 0 {
                // Java: // bail out here as we don't understand the structure
                //       m_complete = false; break;
                self.complete = false;
                break;
            }

            // Java: data = InputStreamHelper.read(is, size)
            let item = is.read(size as usize).ok_or_else(read_error)?;
            self.props.put(attrib2, item.to_vec());

// Java: align to a two-byte boundary
            if item.len() % 2 != 0 {
                is.skip(1);
            }
        }

        Ok(())
    }

    // -----------------------------------------------------------------
// The following methods are inherited from Props on the Java side; the Rust side forwards to the internally held Props
    // -----------------------------------------------------------------

/// Get the attribute data (raw bytes).
    ///
    /// Corresponds to Java: `Props.getByteArray(PropsKey key)`
    pub fn get_byte_array(&self, key: PropsKey) -> Option<&[u8]> {
        self.props.get_byte_array(key)
    }

/// Get a byte value.
    ///
    /// Corresponds to Java: `Props.getByte(PropsKey key)`
    pub fn get_byte(&self, key: PropsKey) -> i8 {
        self.props.get_byte(key)
    }

/// Get a short value.
    ///
    /// Corresponds to Java: `Props.getShort(PropsKey key)`
    pub fn get_short(&self, key: PropsKey) -> i32 {
        self.props.get_short(key)
    }

/// Get an int value.
    ///
    /// Corresponds to Java: `Props.getInt(PropsKey key)`
    pub fn get_int(&self, key: PropsKey) -> i32 {
        self.props.get_int(key)
    }

/// Get a double value.
    ///
    /// Corresponds to Java: `Props.getDouble(PropsKey key)`
    pub fn get_double(&self, key: PropsKey) -> f64 {
        self.props.get_double(key)
    }

/// Get a time value.
    ///
    /// Corresponds to Java: `Props.getTime(PropsKey key)`
    pub fn get_time(&self, key: PropsKey) -> Option<NaiveTime> {
        self.props.get_time(key)
    }

/// Get a timestamp.
    ///
    /// Corresponds to Java: `Props.getTimestamp(PropsKey key)`
    pub fn get_timestamp(&self, key: PropsKey) -> Option<NaiveDateTime> {
        self.props.get_timestamp(key)
    }

/// Get a boolean value.
    ///
    /// Corresponds to Java: `Props.getBoolean(PropsKey key)`
    pub fn get_boolean(&self, key: PropsKey) -> bool {
        self.props.get_boolean(key)
    }

/// Get a UTF-16LE string.
    ///
    /// Corresponds to Java: `Props.getUnicodeString(PropsKey key)`
    pub fn get_unicode_string(&self, key: PropsKey) -> Option<String> {
        self.props.get_unicode_string(key)
    }

/// Get a GUID.
    ///
    /// Corresponds to Java: `Props.getUUID(PropsKey key)`
    pub fn get_uuid(&self, key: PropsKey) -> Option<Uuid> {
        self.props.get_uuid(key)
    }

/// Get all keys (ascending).
    ///
    /// Corresponds to Java: `Props.keySet()`
    pub fn key_set(&self) -> Vec<i32> {
        self.props.key_set()
    }
}

impl fmt::Display for Props8 {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java uses `PrintWriter.println`, whose newline is platform-dependent (`\r\n` on Windows);
/// the Rust side uniformly uses `\n`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "BEGIN Props")?;
        if self.complete {
            writeln!(f, "   COMPLETE")?;
        } else {
            writeln!(f, "   INCOMPLETE")?;
        }

        for (key, value) in self.props.entries() {
            writeln!(
                f,
                "   Key: {key} Value: {}",
                ByteArrayHelper::hexdump_all(value, true)
            )?;
        }

        writeln!(f, "END Props")?;
        // Java: pw.println();
        writeln!(f)?;

        Ok(())
    }
}

impl std::ops::Deref for Props8 {
    type Target = Props;

/// Corresponds to Java: "a subclass instance can be used directly as a base class" from the inheritance relationship
    fn deref(&self) -> &Self::Target {
        &self.props
    }
}

impl std::ops::DerefMut for Props8 {
/// Corresponds to Java: "a subclass instance can be used directly as a base class" from the inheritance relationship
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.props
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// Construct an attribute entry: Java extracts from the 4 bytes of `attrib` both
/// `attrib2` (a little-endian integer) and `attrib3` (the 3rd byte).
    struct Entry {
        attrib1: i32,
        attrib2: i32,
        attrib3: u8,
        attrib5: i32,
        data: Vec<u8>,
    }

    fn write_entries(entries: &[Entry]) -> Vec<u8> {
        let mut data = Vec::new();

        // Java: readInt ×3 + readShort count + readShort
        data.extend_from_slice(&1024i32.to_le_bytes());
        data.extend_from_slice(&1024i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(entries.len() as i16).to_le_bytes());
        data.extend_from_slice(&0i16.to_le_bytes());

        for entry in entries {
            data.extend_from_slice(&entry.attrib1.to_le_bytes());
            data.push((entry.attrib2 & 0xFF) as u8);
            data.push(((entry.attrib2 >> 8) & 0xFF) as u8);
            data.push(entry.attrib3);
            data.push(((entry.attrib2 >> 24) & 0xFF) as u8);
            data.extend_from_slice(&entry.attrib5.to_le_bytes());
            data.extend_from_slice(&entry.data);
// Java: align to a two-byte boundary
            if entry.data.len() % 2 != 0 {
                data.push(0);
            }
        }

        data
    }

    #[test]
    fn reads_entries_with_attrib3_length() {
// attrib3 == 64 -> the length is taken from attrib1
        let entries = [
            Entry {
                attrib1: 2,
                // PropsKey.PASSWORD_FLAG == 893386752 == 0x35400000
                attrib2: 893386752,
                attrib3: 64,
                attrib5: 0,
                data: vec![0x02, 0x01],
            },
            Entry {
                attrib1: 4,
                // PropsKey.ENCRYPTION_CODE == 893386759 == 0x35400007
                attrib2: 893386759,
                attrib3: 64,
                attrib5: 0,
                data: vec![0xD2, 0x04, 0x00, 0x00],
            },
        ];

        let file = ProjectFile::new();
        let props = Props8::new(&file, &write_entries(&entries));

        assert!(props.is_complete());
        assert_eq!(props.key_set(), vec![893386752, 893386759]);
        assert_eq!(props.get_short(PropsKey::PasswordFlag), 258);
        assert_eq!(props.get_int(PropsKey::EncryptionCode), 1234);
        assert!(props.get_boolean(PropsKey::PasswordFlag));
        assert!(file.borrow().get_ignored_errors().is_empty());
    }

    #[test]
    fn reads_entries_with_attrib5_length_and_65536_override() {
// Note: attrib3 is exactly the 3rd byte of attrib2 (the little-endian integer); the two must be consistent
        let entries = [
// attrib3 = 2 != 64 -> the length is taken from attrib5
            Entry {
                attrib1: 999,
                // PropsKey.TASK_FIELD_MAP == 131092 == 0x00020014(byte2 == 2)
                attrib2: 131092,
                attrib3: 2,
                attrib5: 2,
                data: vec![0x00, 0x00],
            },
// attrib5 == 65536 -> the length is fixed at 4 (overriding attrib1 = 1)
            Entry {
                attrib1: 1,
                // PropsKey.TASK_FIELD_NAME_ALIASES == 1048577 == 0x00100001(byte2 == 16)
                attrib2: 1048577,
                attrib3: 16,
                attrib5: 65536,
                data: vec![0x01, 0x00, 0x00, 0x00],
            },
        ];

        let file = ProjectFile::new();
        let props = Props8::new(&file, &write_entries(&entries));

        assert!(props.is_complete());
        assert_eq!(props.get_short(PropsKey::TaskFieldMap), 0);
        assert!(!props.get_boolean(PropsKey::TaskFieldMap));
        assert_eq!(props.get_int(PropsKey::TaskFieldNameAliases), 1);
    }

    #[test]
    fn odd_length_data_is_aligned() {
// The first data item has length 1 (odd), so skip 1 padding byte before reading the second item
        let entries = [
            Entry {
                attrib1: 1,
                // PropsKey.TITLE == 37748744 == 0x02400008
                attrib2: 37748744,
                attrib3: 64,
                attrib5: 0,
                data: vec![0x03],
            },
            Entry {
                attrib1: 2,
                // PropsKey.SUBJECT == 37748745 == 0x02400009
                attrib2: 37748745,
                attrib3: 64,
                attrib5: 0,
                data: vec![0x02, 0x01],
            },
        ];

        let file = ProjectFile::new();
        let props = Props8::new(&file, &write_entries(&entries));

        assert!(props.is_complete());
        assert_eq!(props.get_byte(PropsKey::Title), 3);
        assert_eq!(props.get_short(PropsKey::Subject), 258);
    }

    #[test]
    fn zero_length_entry_stops_parsing() {
        let entries = [
            Entry {
                attrib1: 2,
                attrib2: 37748744,
                attrib3: 64,
                attrib5: 0,
                data: vec![0x02, 0x01],
            },
// size == 0 -> the structure is unintelligible; stop parsing
            Entry {
                attrib1: 0,
                attrib2: 37748745,
                attrib3: 64,
                attrib5: 0,
                data: Vec::new(),
            },
        ];

        let file = ProjectFile::new();
        let props = Props8::new(&file, &write_entries(&entries));

        assert!(!props.is_complete());
        assert_eq!(props.key_set(), vec![37748744]);
        assert_eq!(props.get_short(PropsKey::Title), 258);
        assert_eq!(props.get_short(PropsKey::Subject), 0);
        assert!(props.to_string().contains("INCOMPLETE"));
    }

    #[test]
    fn truncated_data_records_ignored_error() {
// Declares 2 entries, but the second entry's header is truncated
        let mut data = write_entries(&[Entry {
            attrib1: 2,
            attrib2: 37748744,
            attrib3: 64,
            attrib5: 0,
            data: vec![0x02, 0x01],
        }]);
        data[12] = 2; // count = 2 (little-endian short at offset 12, 4th byte is the low byte)

        let file = ProjectFile::new();
        let props = Props8::new(&file, &data);

        assert!(!props.is_complete());
// Properties already read are retained
        assert_eq!(props.get_short(PropsKey::Title), 258);
        // Java: file.addIgnoredError(ex)
        assert_eq!(file.borrow().get_ignored_errors().len(), 1);
        assert!(props.to_string().contains("INCOMPLETE"));
    }

    #[test]
    fn empty_data_is_incomplete() {
        let file = ProjectFile::new();
        let props = Props8::new(&file, &[]);

        assert!(!props.is_complete());
        assert!(props.key_set().is_empty());
        assert_eq!(file.borrow().get_ignored_errors().len(), 1);
    }

    #[test]
    fn display_lists_values() {
        let entries = [Entry {
            attrib1: 2,
            attrib2: 37748744,
            attrib3: 64,
            attrib5: 0,
            data: vec![0x02, 0x01],
        }];

        let file = ProjectFile::new();
        let props = Props8::new(&file, &write_entries(&entries));

        let text = props.to_string();
        assert!(text.starts_with("BEGIN Props\n   COMPLETE\n"));
// Java: ByteArrayHelper.hexdump(value, true) -- includes an ASCII view (non-printable characters are spaces)
        assert!(text.contains("   Key: 37748744 Value: 02 01"));
        assert!(text.ends_with("END Props\n\n"));
    }

    #[test]
    fn missing_keys_return_defaults() {
        let file = ProjectFile::new();
        let props = Props8::new(&file, &write_entries(&[]));

        assert!(props.is_complete());
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
    }
}
