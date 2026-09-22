//! Corresponds to Java: `org/mpxj/mpp/Props9.java`

use std::fmt;

use chrono::{NaiveDateTime, NaiveTime};
use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::props::{InputStreamCursor, Props, read_error};
use crate::mpp::props_key::PropsKey;
use crate::mpxj_exception::MpxjResult;

/// The `Props` stream of an MPP9 file.
///
/// Corresponds to Java: `final class Props9 extends Props`
///
/// # Restoring inheritance (composition + `Deref` + forwarding)
///
/// Java's `Props9` extends [`Props`] (Java does not override `toString()`). Rust has no inheritance, so this class uses
/// **composition + `Deref`/`DerefMut`** to restore it: it holds a [`Props`] (field `props`), so `&Props9`
/// can auto-coerce (deref coercion) to `&Props` -- "methods whose parameter is `&Props`" can also directly accept
/// `&Props9`, corresponding to Java's "a subclass instance can be passed directly to a base class parameter" (this crate already uses
/// the same approach in [`crate::recurring_task`] to restore inheritance).
///
/// At the end of the class, explicit forwarding of all of `Props`'s public methods is kept for call sites that do not need the coercion to call directly; these same-named methods
/// shadow the same-named methods on the `Deref` target [`Props`], but they are themselves method-by-method forwards with identical behavior,
/// so there is no semantic conflict.
///
/// # Constructor and `InputStream`
///
/// Java: `Props9(InputStream is) throws IOException`. Per project convention the Rust side changes
/// `InputStream` to `&[u8]`, and sequential reading is done by [`InputStreamCursor`]; Java's
/// `IOException` (insufficient data) corresponds to Rust's `Err(MpxjError)`.
///
/// # Block layout (Java constructor)
///
/// ```text
/// byte[16] header      headerCount = the short in this header (offset 12) -- the number of entries
/// headerCount × {
/// int  itemSize    Property data length
/// int  itemKey     Property key
///     int  unknown
///     byte data[itemSize]
/// byte pad         Pads to a two-byte boundary only when itemSize is odd
/// }
/// ```
///
/// Java returns an empty table directly when `is.available() < 16`; if
/// `availableBytes < itemSize || itemSize < 1` occurs in the loop it breaks (without throwing).
///
/// # Differences from Java
///
/// - Skip `getValue` / reflection-unrelated content; `toString()` reuses the [`Props`] implementation (Java does not override it).
/// - `availableBytes` is decremented as `i64` (Java uses `int`, which overflows on extreme input).
///
/// Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: not used by this class.
pub struct Props9 {
/// Corresponds to Java: `extends Props` (Rust uses composition instead of inheritance)
    props: Props,
}

impl Props9 {
/// Constructor.
    ///
    /// Corresponds to Java: `Props9(InputStream is) throws IOException`
    pub fn new(data: &[u8]) -> MpxjResult<Self> {
        let mut result = Self { props: Props::new() };
        let mut is = InputStreamCursor::new(data);

        // Java: if (is.available() < 16) { return; }
        if is.available() < 16 {
            return Ok(result);
        }

        // Java: byte[] header = InputStreamHelper.read(is, 16)
        let header = is.read(16).ok_or_else(read_error)?;
        // Java: int headerCount = ByteArrayHelper.getShort(header, 12)
        let header_count = ByteArrayHelper::get_short(header, 12);
        let mut found_count = 0;
        // Java: int availableBytes = is.available()
        let mut available_bytes = is.available() as i64;

        while found_count < header_count {
            // Java: int itemSize = readInt(is)
            let item_size = is.read_int().ok_or_else(read_error)?;
            // Java: int itemKey = readInt(is)
            let item_key = is.read_int().ok_or_else(read_error)?;
            // Java: /*int attrib3 = */ readInt(is)
            is.read_int().ok_or_else(read_error)?;
            available_bytes -= 12;

            if available_bytes < i64::from(item_size) || item_size < 1 {
                break;
            }

            // Java: byte[] data = InputStreamHelper.read(is, itemSize)
            let item = is.read(item_size as usize).ok_or_else(read_error)?;
            available_bytes -= i64::from(item_size);

            result.props.put(item_key, item.to_vec());
            found_count += 1;

// Java: align to a two-byte boundary
            if item.len() % 2 != 0 {
                is.skip(1);
            }
        }

        Ok(result)
    }

    // -----------------------------------------------------------------
// The following methods are inherited from Props on the Java side; the Rust side forwards them to the inner Props
    // -----------------------------------------------------------------

/// Get the property data (raw bytes).
    ///
    /// Corresponds to Java: `Props.getByteArray(PropsKey key)`
    pub fn get_byte_array(&self, key: PropsKey) -> Option<&[u8]> {
        self.props.get_byte_array(key)
    }

/// Get the byte value.
    ///
    /// Corresponds to Java: `Props.getByte(PropsKey key)`
    pub fn get_byte(&self, key: PropsKey) -> i8 {
        self.props.get_byte(key)
    }

/// Get the short value.
    ///
    /// Corresponds to Java: `Props.getShort(PropsKey key)`
    pub fn get_short(&self, key: PropsKey) -> i32 {
        self.props.get_short(key)
    }

/// Get the int value.
    ///
    /// Corresponds to Java: `Props.getInt(PropsKey key)`
    pub fn get_int(&self, key: PropsKey) -> i32 {
        self.props.get_int(key)
    }

/// Get the double value.
    ///
    /// Corresponds to Java: `Props.getDouble(PropsKey key)`
    pub fn get_double(&self, key: PropsKey) -> f64 {
        self.props.get_double(key)
    }

/// Get the time value.
    ///
    /// Corresponds to Java: `Props.getTime(PropsKey key)`
    pub fn get_time(&self, key: PropsKey) -> Option<NaiveTime> {
        self.props.get_time(key)
    }

/// Get the timestamp.
    ///
    /// Corresponds to Java: `Props.getTimestamp(PropsKey key)`
    pub fn get_timestamp(&self, key: PropsKey) -> Option<NaiveDateTime> {
        self.props.get_timestamp(key)
    }

/// Get the boolean value.
    ///
    /// Corresponds to Java: `Props.getBoolean(PropsKey key)`
    pub fn get_boolean(&self, key: PropsKey) -> bool {
        self.props.get_boolean(key)
    }

/// Get the UTF-16LE string.
    ///
    /// Corresponds to Java: `Props.getUnicodeString(PropsKey key)`
    pub fn get_unicode_string(&self, key: PropsKey) -> Option<String> {
        self.props.get_unicode_string(key)
    }

/// Get the GUID.
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

impl fmt::Display for Props9 {
/// Corresponds to Java: `Props.toString()` (`Props9` does not override it)
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.props, f)
    }
}

impl std::ops::Deref for Props9 {
    type Target = Props;

/// Corresponds to Java: "a subclass instance can be used directly as a base class instance" brought by the inheritance relationship
    fn deref(&self) -> &Self::Target {
        &self.props
    }
}

impl std::ops::DerefMut for Props9 {
/// Corresponds to Java: "a subclass instance can be used directly as a base class instance" brought by the inheritance relationship
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.props
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// Construct the data according to the MPP9 block layout.
    fn build_props9(header_count: i32, entries: &[(i32, i32, Vec<u8>)]) -> Vec<u8> {
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
        let data = build_props9(
            2,
            &[
                // PropsKey.TITLE == 37748744
                (4, 37748744, vec![0xD2, 0x04, 0x00, 0x00]),
                // PropsKey.SUBJECT == 37748745
                (2, 37748745, vec![0x02, 0x01]),
            ],
        );

        let props = Props9::new(&data).expect("should parse");
        assert_eq!(props.key_set(), vec![37748744, 37748745]);
        assert_eq!(props.get_int(PropsKey::Title), 1234);
        assert_eq!(props.get_short(PropsKey::Subject), 258);
    }

    #[test]
    fn odd_length_data_is_aligned() {
        let data = build_props9(
            2,
            &[
                (1, 37748744, vec![0x07]),
                (2, 37748745, vec![0x02, 0x01]),
            ],
        );

        let props = Props9::new(&data).expect("should parse");
        assert_eq!(props.get_byte(PropsKey::Title), 7);
        assert_eq!(props.get_short(PropsKey::Subject), 258);
    }

    #[test]
    fn short_header_returns_empty_props() {
// Java: if (is.available() < 16) { return; } -- does not throw
        let props = Props9::new(&[0u8; 15]).expect("should parse");
        assert!(props.key_set().is_empty());
        assert_eq!(props.get_int(PropsKey::Title), 0);
    }

    #[test]
    fn declared_count_larger_than_data_returns_error() {
// Declares 3 entries but only 1 exists: the stream end is reached when reading the header of the 2nd entry
// (Java's POI DocumentInputStream throws IOException here, so Rust returns Err)
        let data = build_props9(3, &[(2, 37748744, vec![0x02, 0x01])]);

        assert!(Props9::new(&data).is_err());
    }

    #[test]
    fn item_size_beyond_available_bytes_breaks_loop_without_error() {
// The 2nd entry's header is complete but itemSize exceeds the remaining bytes -> Java: availableBytes < itemSize -> break
        let mut data = build_props9(2, &[(2, 37748744, vec![0x02, 0x01])]);
        data.extend_from_slice(&100i32.to_le_bytes());
        data.extend_from_slice(&37748745i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());

        let props = Props9::new(&data).expect("should parse");
        assert_eq!(props.key_set(), vec![37748744]);
    }

    #[test]
    fn invalid_item_size_breaks_loop() {
        // itemSize == 0 → Java: itemSize < 1 → break
        let data = build_props9(
            2,
            &[
                (2, 37748744, vec![0x02, 0x01]),
                (0, 37748745, Vec::new()),
            ],
        );

        let props = Props9::new(&data).expect("should parse");
        assert_eq!(props.key_set(), vec![37748744]);
    }

    #[test]
    fn item_size_beyond_available_bytes_breaks_loop() {
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&2i16.to_le_bytes());
// First entry: itemSize is far larger than the remaining bytes
        data.extend_from_slice(&100i32.to_le_bytes());
        data.extend_from_slice(&37748744i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());

        let props = Props9::new(&data).expect("should parse");
        assert!(props.key_set().is_empty());
    }

    #[test]
    fn truncated_entry_returns_error() {
// Declares 1 entry, but the entry header is only 8 bytes
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&1i16.to_le_bytes());
        data.extend_from_slice(&4i32.to_le_bytes());
        data.extend_from_slice(&37748744i32.to_le_bytes());

        assert!(Props9::new(&data).is_err());
    }

    #[test]
    fn missing_keys_return_defaults_and_display_works() {
        let props = Props9::new(&build_props9(0, &[])).expect("should parse");

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
