//! Corresponds to Java: `org/mpxj/mpp/Props12.java`

use std::fmt;

use chrono::{NaiveDateTime, NaiveTime};
use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::props::{InputStreamCursor, Props, read_error};
use crate::mpp::props_key::PropsKey;
use crate::mpxj_exception::MpxjResult;

/// The `Props` stream of an MPP12 file.
///
/// Corresponds to Java: `final class Props12 extends Props`
///
/// # Restoring inheritance (composition + `Deref` + forwarding)
///
/// Java's `Props12` extends [`Props`] (Java does not override `toString()`). Rust has no inheritance, so this class uses
/// **composition + `Deref`/`DerefMut`** to restore it: it holds a [`Props`] (field `props`), so `&Props12`
/// automatically coerces (deref coercion) to `&Props` — "a method whose parameter is `&Props`" can directly accept
/// `&Props12`, corresponding to Java's "a subclass instance can be passed directly to a base-class parameter" (this crate already
/// restores inheritance with the same technique in [`crate::recurring_task`]).
///
/// At the end of the class, explicit forwarding of all of `Props`'s public methods is still kept so call sites that do not need the coercion can call them directly; these same-named methods
/// shadow the same-named methods on the `Deref` target [`Props`], but they are themselves method-by-method forwards with identical behavior,
/// so there is no semantic conflict.
///
/// # Constructor and `InputStream`
///
/// Java: `Props12(InputStream is) throws IOException`. On the Rust side, per this project's convention,
/// `InputStream` is replaced by `&[u8]`; Java's `IOException` (insufficient data) corresponds to Rust's
/// `Err(MpxjError)`.
///
/// # Block layout (Java constructor)
///
/// ```text
/// byte[16] header      headerCount = the short in this header (offset 12) — number of entries
/// headerCount × {
/// int  attrib1     attribute data length
/// int  attrib2     attribute key
///     int  unknown
///     byte data[attrib1]
/// byte pad         pads to a two-byte boundary only when attrib1 is odd
/// }
/// ```
///
/// Unlike `Props9`, `Props12` does not do an `available()` pre-check and reads the 16-byte header directly;
/// the loop breaks when `availableBytes < attrib1 || attrib1 < 1` occurs (without throwing).
///
/// # Differences from Java
///
/// - `availableBytes` is decremented as `i64` (Java uses `int`, which overflows on extreme input).
/// - `toString()` reuses [`Props`]'s implementation (Java does not override it).
///
/// Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class uses none.
pub struct Props12 {
/// Corresponds to Java: `extends Props` (Rust uses composition instead of inheritance)
    props: Props,
}

impl Props12 {
/// Constructor.
    ///
    /// Corresponds to Java: `Props12(InputStream is) throws IOException`
    pub fn new(data: &[u8]) -> MpxjResult<Self> {
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

            // Java: data = InputStreamHelper.read(is, attrib1)
            let item = is.read(attrib1 as usize).ok_or_else(read_error)?;
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
// The following methods are inherited from Props on the Java side; on the Rust side they forward to the internally held Props
    // -----------------------------------------------------------------

/// Get the attribute data (raw bytes).
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

impl fmt::Display for Props12 {
/// Corresponds to Java: `Props.toString()` (`Props12` does not override it)
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.props, f)
    }
}

impl std::ops::Deref for Props12 {
    type Target = Props;

/// Corresponds to Java: the "subclass instance can be used directly as a base-class instance" behavior from the inheritance relationship
    fn deref(&self) -> &Self::Target {
        &self.props
    }
}

impl std::ops::DerefMut for Props12 {
/// Corresponds to Java: the "subclass instance can be used directly as a base-class instance" behavior from the inheritance relationship
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.props
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// Build data according to the MPP12 block layout.
    fn build_props12(header_count: i32, entries: &[(i32, i32, Vec<u8>)]) -> Vec<u8> {
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
        let data = build_props12(
            2,
            &[
                // PropsKey.TITLE == 37748744
                (4, 37748744, vec![0xD2, 0x04, 0x00, 0x00]),
                // PropsKey.SUBJECT == 37748745
                (2, 37748745, vec![0x02, 0x01]),
            ],
        );

        let props = Props12::new(&data).expect("should parse");
        assert_eq!(props.key_set(), vec![37748744, 37748745]);
        assert_eq!(props.get_int(PropsKey::Title), 1234);
        assert_eq!(props.get_short(PropsKey::Subject), 258);
    }

    #[test]
    fn odd_length_data_is_aligned() {
        let data = build_props12(
            2,
            &[(1, 37748744, vec![0x07]), (2, 37748745, vec![0x02, 0x01])],
        );

        let props = Props12::new(&data).expect("should parse");
        assert_eq!(props.get_byte(PropsKey::Title), 7);
        assert_eq!(props.get_short(PropsKey::Subject), 258);
    }

    #[test]
    fn short_header_returns_error() {
// Java: InputStreamHelper.read(is, 16) throws when there is insufficient data
        assert!(Props12::new(&[0u8; 15]).is_err());
    }

    #[test]
    fn invalid_size_breaks_loop() {
        let data = build_props12(
            2,
            &[
                (2, 37748744, vec![0x02, 0x01]),
                (0, 37748745, Vec::new()),
            ],
        );

        let props = Props12::new(&data).expect("should parse");
        assert_eq!(props.key_set(), vec![37748744]);
    }

    #[test]
    fn size_beyond_available_bytes_breaks_loop_without_error() {
        let mut data = build_props12(2, &[(2, 37748744, vec![0x02, 0x01])]);
        data.extend_from_slice(&100i32.to_le_bytes());
        data.extend_from_slice(&37748745i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());

        let props = Props12::new(&data).expect("should parse");
        assert_eq!(props.key_set(), vec![37748744]);
    }

    #[test]
    fn truncated_entry_returns_error() {
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&1i16.to_le_bytes());
        data.extend_from_slice(&4i32.to_le_bytes());
        data.extend_from_slice(&37748744i32.to_le_bytes());

        assert!(Props12::new(&data).is_err());
    }

    #[test]
    fn missing_keys_return_defaults_and_display_works() {
        let props = Props12::new(&build_props12(0, &[])).expect("should parse");

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
