//! Corresponds to Java: `org/mpxj/mpp/Props.java`
//!
//! `Props` describes the `Props*` streams in an MPP file: it is an "integer key -> byte array" property table,
//! populated by the version-specific subclasses according to their own byte layout, then read by key from upper layers (`ProjectPropertiesReader`,
//! `DocumentInputStreamFactory`, etc.).
//!
//! # Restoring inheritance (composition + forwarding)
//!
//! Java: `class Props extends MPPComponent`, and `Props8/Props9/Props12/Props14/PropsBlock
//! extends Props`. Rust has no inheritance, so:
//!
//! - `MPPComponent` (which only provides the `InputStream`-based `readByte` / `readShort` / `readInt` /
//!   `readByteArray`) replaces `InputStream` with `&[u8]` per this project's convention, and has been
//!   implemented by [`crate::mpp::mpp_component::MPPComponent`]; the [`InputStreamCursor`] in this file
//!   provides "cursor" convenience while its `read_int` / `read_short` / `read` forward directly to it,
//!   without duplicating byte-parsing logic. [`Props`] itself needs no base class (Java's
//!   `extends MPPComponent` merely reuses 4 read helpers).
//! - `Props8/9/12/14/PropsBlock` each **hold a [`Props`]** (private field `props`) and forward
//!   all of its public methods unchanged; `Props8/9/12/14` additionally implement `Deref`/`DerefMut` (targeting [`Props`]),
//!   so that references like `&Props9` auto-coerce to `&Props` -- equivalent to Java's inheritance (where a subclass instance can be passed directly to a
//!   base-class parameter), using the same technique as [`crate::recurring_task`].
//!
//! # Differences from Java in this class
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `byte[] getByteArray(PropsKey)` | `Option<&[u8]>` | Returns a borrow of the internal buffer without copying; `null` -> `None` |
//! | `byte getByte(PropsKey)` | `i8` | Java's `byte` is signed; when the property is an empty array Java throws `AIOOBE`, Rust returns 0 |
//! | `int getShort/getInt` | `i32` | When the property is too short Java throws `AIOOBE`, Rust returns 0 |
//! | `double getDouble` | `f64` | Same as above |
//! | `LocalTime getTime` | `Option<NaiveTime>` | Same as above, returns `None` |
//! | `LocalDateTime getTimestamp` | `Option<NaiveDateTime>` | Same as above, returns `None` |
//! | `UUID getUUID` | `Option<Uuid>` | When parsing the MPP9 string form fails Java throws `IllegalArgumentException`, Rust returns `None` |
//! | `Set<Integer> keySet()` | `Vec<i32>` | Returns a copy of the keys in `TreeMap` iteration order (ascending) |
//! | `toString()` | `Display` | The platform line separator of `PrintWriter.println` is normalized to `\n` |
//! | `protected final TreeMap<Integer, byte[]> m_map` | Private field `map` + [`Props::put`] / `get_byte_array_by_key` / `entries` | Java subclasses read and write this field directly; on the Rust side this class provides three `pub(crate)` read/write entry points (the only visibility change) |
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: not used by this class.
//!
//! # Interface assumptions about the related module `crate::mpp::mpp_utility::MPPUtility` (verified consistent)
//!
//! ```text
//! MPPUtility::get_double(&[u8], usize) -> f64                                // Java getDouble
//! MPPUtility::get_time(&[u8], usize) -> NaiveTime                            // Java getTime
//! MPPUtility::get_timestamp(&[u8], usize) -> Option<NaiveDateTime>           // Java getTimestamp
//! MPPUtility::get_unicode_string(&[u8], usize) -> String                     // Java getUnicodeString
//! MPPUtility::get_unicode_string_with_max_length(&[u8], usize, usize) -> String
//!                                                                            // Java getUnicodeString(data, offset, maxLength)
//! MPPUtility::get_guid(&[u8], usize) -> Option<Uuid>                         // Java getGUID
//! ```

use std::collections::BTreeMap;
use std::fmt;

use chrono::{NaiveDateTime, NaiveTime};
use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::mpp_component::MPPComponent;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::props_key::PropsKey;

/// Property table: integer key -> byte array.
///
/// Corresponds to Java: `class Props extends MPPComponent`
///
/// Rust-specific: Java has no explicit constructor (default constructor + subclasses populate `m_map`), so this provides
/// [`Props::new`].
#[derive(Debug, Default)]
pub struct Props {
    /// Corresponds to Java: `protected final TreeMap<Integer, byte[]> m_map`
    ///
    /// Uses `BTreeMap` to correspond to Java's `TreeMap` (keys ascending).
    map: BTreeMap<i32, Vec<u8>>,
}

impl Props {
    /// Construct an empty property table.
    ///
    /// No corresponding method on the Java side (implicit default constructor).
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    /// Write one property.
    ///
    /// Corresponds to Java: calls to `m_map.put(Integer, byte[])` made by subclasses (e.g. `Props8`, `PropsBlock`)
    pub fn put(&mut self, key: i32, value: Vec<u8>) {
        self.map.insert(key, value);
    }

    /// Rebuild an equivalent [`Props`] from the contents of another property table.
    ///
    /// Rust-specific: Java's `Props9/12/14 extends Props` and `ViewStateReader.getProps` returns
    /// a subclass instance directly, which is then used through a base-class reference; subclasses in this crate implement "inheritance" via composition (see the file header), and `Deref`
    /// can only yield a borrow and cannot take the inner [`Props`] out by value, so scenarios that "need an owned [`Props`]"
    /// rebuild one here by key value (contents identical to the source, including keys not covered by `PropsKey`).
    pub(crate) fn copied_from(source: &Props) -> Props {
        Props {
            map: source.map.clone(),
        }
    }

    /// Get the property data (raw bytes).
    ///
    /// Corresponds to Java: `public byte[] getByteArray(PropsKey key)`
    pub fn get_byte_array(&self, key: PropsKey) -> Option<&[u8]> {
        self.get_byte_array_by_key(key.get_value())
    }

    /// Get the property data by integer key.
    ///
    /// Rust-specific: Java subclasses read the protected field `m_map` directly
    /// (e.g. `PropsBlock.getByteArray(Integer)`); on the Rust side this method exposes the read entry point instead.
    pub(crate) fn get_byte_array_by_key(&self, key: i32) -> Option<&[u8]> {
        self.map.get(&key).map(Vec::as_slice)
    }

    /// Iterate over all properties in ascending key order.
    ///
    /// Rust-specific: Java subclasses iterate the protected field `m_map` directly (e.g. `Props8.toString()`).
    pub(crate) fn entries(&self) -> impl Iterator<Item = (&i32, &Vec<u8>)> {
        self.map.iter()
    }

    /// Get a byte value.
    ///
    /// Corresponds to Java: `public byte getByte(PropsKey key)`
    ///
    /// Java returns a signed `byte`; Rust uses `i8` to match. Returns 0 when the property does not exist,
    /// and returns 0 in Rust when the property is an empty array (Java throws `ArrayIndexOutOfBoundsException`).
    pub fn get_byte(&self, key: PropsKey) -> i8 {
        value_byte(self.get_byte_array(key))
    }

    /// Get a short value.
    ///
    /// Corresponds to Java: `public int getShort(PropsKey key)`
    pub fn get_short(&self, key: PropsKey) -> i32 {
        value_short(self.get_byte_array(key))
    }

    /// Get an int value.
    ///
    /// Corresponds to Java: `public int getInt(PropsKey key)`
    pub fn get_int(&self, key: PropsKey) -> i32 {
        value_int(self.get_byte_array(key))
    }

    /// Get a double value.
    ///
    /// Corresponds to Java: `public double getDouble(PropsKey key)`
    pub fn get_double(&self, key: PropsKey) -> f64 {
        value_double(self.get_byte_array(key))
    }

    /// Get a time value (time of day).
    ///
    /// Corresponds to Java: `public LocalTime getTime(PropsKey key)`
    pub fn get_time(&self, key: PropsKey) -> Option<NaiveTime> {
        value_time(self.get_byte_array(key))
    }

    /// Get a timestamp.
    ///
    /// Corresponds to Java: `public LocalDateTime getTimestamp(PropsKey key)`
    pub fn get_timestamp(&self, key: PropsKey) -> Option<NaiveDateTime> {
        value_timestamp(self.get_byte_array(key))
    }

    /// Get a boolean value (whether the short is non-zero).
    ///
    /// Corresponds to Java: `public boolean getBoolean(PropsKey key)`
    pub fn get_boolean(&self, key: PropsKey) -> bool {
        value_boolean(self.get_byte_array(key))
    }

    /// Get a UTF-16LE string.
    ///
    /// Corresponds to Java: `public String getUnicodeString(PropsKey key)`
    pub fn get_unicode_string(&self, key: PropsKey) -> Option<String> {
        value_unicode_string(self.get_byte_array(key))
    }

    /// Get a GUID.
    ///
    /// Corresponds to Java: `public UUID getUUID(PropsKey key)`
    ///
    /// When the property is longer than 16 bytes Java first tries to parse it as an MPP9 "string-form GUID"
    /// (`{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}`, at most 76 bytes).
    pub fn get_uuid(&self, key: PropsKey) -> Option<Uuid> {
        value_uuid(self.get_byte_array(key))
    }

    /// Get all keys (ascending, matching Java `TreeMap` iteration order).
    ///
    /// Corresponds to Java: `public Set<Integer> keySet()`
    pub fn key_set(&self) -> Vec<i32> {
        self.map.keys().copied().collect()
    }
}

impl fmt::Display for Props {
    /// Corresponds to Java: `@Override public String toString()`
    ///
    /// Java uses `PrintWriter.println`, whose line separator is platform-dependent (on Windows it is `\r\n`);
    /// the Rust side always uses `\n`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Java: build a "key value -> PropsKey list" mapping (one key value may correspond to multiple enum constants)
        let mut key_labels: BTreeMap<i32, Vec<PropsKey>> = BTreeMap::new();
        for key in PropsKey::ALL {
            key_labels.entry(key.get_value()).or_default().push(key);
        }

        writeln!(f, "BEGIN Props")?;

        for (key, value) in &self.map {
            let label = match key_labels.get(key) {
                None => key.to_string(),
                Some(keys) => keys
                    .iter()
                    .map(|item| format!("{}({})", item, item.get_value()))
                    .collect::<Vec<String>>()
                    .join(","),
            };

            writeln!(f, "   Key: {label} Value: ")?;
            // Java: pw.println(ByteArrayHelper.hexdump(value, true, 16, "      "))
            write!(
                f,
                "{}",
                ByteArrayHelper::hexdump_columns(value, true, 16, "      ")
            )?;
        }

        writeln!(f, "END Props")?;
        // Java: pw.println();
        writeln!(f)?;

        Ok(())
    }
}

/// A cursor for sequentially reading `&[u8]`.
///
/// Rust-specific: Java's `Props8/9/12/14` constructors accept an `InputStream` and use
/// `MPPComponent.readInt/readShort`, `InputStreamHelper.read/skip` and
/// `InputStream.available()` to read data. This project's convention replaces `InputStream` with `&[u8]`,
/// so this struct provides one-to-one corresponding operations.
///
/// Here `read_int` / `read_short` / `read` forward directly to the already-implemented
/// [`crate::mpp::mpp_component::MPPComponent`] (corresponding to `MPPComponent.readInt/readShort` and
/// `InputStreamHelper.read`), and this file does not duplicate an equivalent implementation;
/// `available` / `skip` correspond to `InputStream.available()` and `InputStreamHelper.skip`,
/// for which `MPPComponent` has no equivalent, so they remain implemented in this file.
///
/// Out of bounds (where Java throws `EOFException` / `RuntimeException`) always returns `None` / leaves the position unmoved,
/// leaving it to the caller to decide whether to return `Err` or abort parsing; never panics.
#[derive(Debug, Clone, Copy)]
pub(crate) struct InputStreamCursor<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> InputStreamCursor<'a> {
    /// Create a cursor.
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    /// Number of remaining bytes.
    ///
    /// Corresponds to Java: `InputStream.available()`
    ///
    /// `MPPComponent` has no equivalent member, so this remains in this file.
    pub(crate) fn available(&self) -> usize {
        self.data.len().saturating_sub(self.position)
    }

    /// Read a four-byte little-endian integer.
    ///
    /// Corresponds to Java: `MPPComponent.readInt(InputStream)` (forwards to
    /// [`MPPComponent::read_int`])
    pub(crate) fn read_int(&mut self) -> Option<i32> {
        MPPComponent::read_int(self.data, &mut self.position)
    }

    /// Read a two-byte little-endian short (0..65535).
    ///
    /// Corresponds to Java: `MPPComponent.readShort(InputStream)` (forwards to
    /// [`MPPComponent::read_short`])
    pub(crate) fn read_short(&mut self) -> Option<i32> {
        MPPComponent::read_short(self.data, &mut self.position)
    }

    /// Read a byte slice of the specified length.
    ///
    /// Corresponds to Java: `InputStreamHelper.read(InputStream, int)`
    /// (forwards to [`MPPComponent::read_byte_array`]; when the length is insufficient Java throws an exception, Rust returns `None`)
    pub(crate) fn read(&mut self, size: usize) -> Option<&'a [u8]> {
        MPPComponent::read_byte_array(self.data, &mut self.position, size)
    }

    /// Skip the specified number of bytes.
    ///
    /// Corresponds to Java: `InputStreamHelper.skip(InputStream, long)`
    /// (Java throws `IOException` when it cannot skip; the Rust side positions directly at the end)
    ///
    /// `MPPComponent` has no equivalent member, so this remains in this file.
    pub(crate) fn skip(&mut self, size: usize) {
        self.position = (self.position + size).min(self.data.len());
    }
}

/// The exception for reading a property (corresponding to what Java's `InputStreamHelper.read` throws)
/// `RuntimeException("Unable to read the required number of bytes")`).
pub(crate) fn read_error() -> crate::mpxj_exception::MpxjError {
    crate::mpxj_exception::MpxjError::new("Unable to read the required number of bytes")
}

/// Get a byte value. Shared by the [`Props::get_byte`] and `PropsBlock::get_byte_array_by_type` paths.
pub(crate) fn value_byte(item: Option<&[u8]>) -> i8 {
    match item {
        None => 0,
        // Java throws AIOOBE when taking item[0] from an empty array; Rust returns 0
        Some(item) => item.first().map_or(0, |value| *value as i8),
    }
}

/// Get a short value.
pub(crate) fn value_short(item: Option<&[u8]>) -> i32 {
    match item {
        // Java throws AIOOBE when there is insufficient data; Rust returns 0
        None => 0,
        Some(item) if item.len() < 2 => 0,
        Some(item) => ByteArrayHelper::get_short(item, 0),
    }
}

/// Get an int value.
pub(crate) fn value_int(item: Option<&[u8]>) -> i32 {
    match item {
        // Java throws AIOOBE when there is insufficient data; Rust returns 0
        None => 0,
        Some(item) if item.len() < 4 => 0,
        Some(item) => ByteArrayHelper::get_int(item, 0),
    }
}

/// Get a double value.
pub(crate) fn value_double(item: Option<&[u8]>) -> f64 {
    match item {
        // Java throws AIOOBE when there is insufficient data; Rust returns 0
        None => 0.0,
        Some(item) if item.len() < 8 => 0.0,
        Some(item) => MPPUtility::get_double(item, 0),
    }
}

/// Get a time value (time of day).
pub(crate) fn value_time(item: Option<&[u8]>) -> Option<NaiveTime> {
    let item = item?;
    // Java throws AIOOBE when there is insufficient data; Rust returns None
    if item.len() < 2 {
        return None;
    }

    Some(MPPUtility::get_time(item, 0))
}

/// Get a timestamp.
pub(crate) fn value_timestamp(item: Option<&[u8]>) -> Option<NaiveDateTime> {
    let item = item?;
    // Java throws AIOOBE when there is insufficient data; Rust returns None
    if item.len() < 4 {
        return None;
    }

    MPPUtility::get_timestamp(item, 0)
}

/// Get a boolean value.
pub(crate) fn value_boolean(item: Option<&[u8]>) -> bool {
    // Java: !(ByteArrayHelper.getShort(item, 0) == 0)
    value_short(item) != 0
}

/// Get a UTF-16LE string.
pub(crate) fn value_unicode_string(item: Option<&[u8]>) -> Option<String> {
    let item = item?;
    Some(MPPUtility::get_unicode_string(item, 0))
}

/// Get a GUID (MPP9 stores it in string form, at most 76 bytes).
pub(crate) fn value_uuid(item: Option<&[u8]>) -> Option<Uuid> {
    let item = item?;

    if item.len() > 16 {
        // Java: String value = MPPUtility.getUnicodeString(item, 0, 76)
        let value = MPPUtility::get_unicode_string_with_max_length(item, 0, 76);

        // Java: value.length() == 38 && value.charAt(0) == '{' && value.charAt(37) == '}'
        // Counts by character (not byte), consistent with Java's char semantics.
        if value.chars().count() == 38 && value.starts_with('{') && value.ends_with('}') {
            // Java: UUID.fromString(value.substring(1, 37)) -- when parsing fails Java throws
            // IllegalArgumentException; Rust returns None
            let inner: String = value.chars().skip(1).take(36).collect();
            return Uuid::parse_str(&inner).ok();
        }
    }

    MPPUtility::get_guid(item, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Construct a table containing properties of various types.
    fn sample_props() -> Props {
        let mut props = Props::new();

        // Single byte
        props.put(PropsKey::Company.get_value(), vec![0x07]);
        // Two-byte short: 258
        props.put(PropsKey::DaysPerMonth.get_value(), vec![0x02, 0x01]);
        // Four-byte integer: 1234
        props.put(PropsKey::MinutesPerDay.get_value(), vec![0xD2, 0x04, 0x00, 0x00]);
        // Double: 1.5
        props.put(
            PropsKey::StandardRate.get_value(),
            1.5f64.to_le_bytes().to_vec(),
        );
        // UTF-16LE string "AB"
        props.put(
            PropsKey::Manager.get_value(),
            vec![0x41, 0x00, 0x42, 0x00, 0x00, 0x00],
        );
        // Boolean: false (short 0) / true (short 1)
        props.put(PropsKey::ScheduleFrom.get_value(), vec![0x00, 0x00]);
        props.put(PropsKey::HonorConstraints.get_value(), vec![0x01, 0x00]);
        // Time: short 600 -> 600/10*60 = 3600 seconds = 01:00:00
        props.put(PropsKey::StartTime.get_value(), vec![0x58, 0x02]);
        // Timestamp: the first 2 bytes are seconds-of-day/6 (=10 -> 60 seconds), the next 2 bytes are the day count (=2)
        // -> 1983-12-31 + 2 days = 1984-01-02 00:01:00
        props.put(
            PropsKey::StatusDate.get_value(),
            vec![0x0A, 0x00, 0x02, 0x00],
        );

        props
    }

    #[test]
    fn reads_scalar_values() {
        let props = sample_props();

        assert_eq!(props.get_byte(PropsKey::Company), 7);
        assert_eq!(props.get_short(PropsKey::DaysPerMonth), 258);
        assert_eq!(props.get_int(PropsKey::MinutesPerDay), 1234);
        assert_eq!(props.get_double(PropsKey::StandardRate), 1.5);
        assert_eq!(
            props.get_unicode_string(PropsKey::Manager),
            Some("AB".to_owned())
        );
        assert!(!props.get_boolean(PropsKey::ScheduleFrom));
        assert!(props.get_boolean(PropsKey::HonorConstraints));
        assert_eq!(
            props.get_time(PropsKey::StartTime),
            Some(NaiveTime::from_hms_opt(1, 0, 0).expect("valid time"))
        );
        assert_eq!(
            props.get_timestamp(PropsKey::StatusDate),
            Some(
                chrono::NaiveDate::from_ymd_opt(1984, 1, 2)
                    .expect("valid date")
                    .and_hms_opt(0, 1, 0)
                    .expect("valid time")
            )
        );
    }

    #[test]
    fn reads_byte_array_and_key_set() {
        let props = sample_props();

        assert_eq!(props.get_byte_array(PropsKey::Company), Some(&[0x07][..]));
        assert!(props.get_byte_array(PropsKey::Category).is_none());

        let keys = props.key_set();
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(keys, sorted, "key_set should return keys in ascending order");
        assert_eq!(keys.len(), 9);
    }

    #[test]
    fn missing_keys_return_defaults() {
        let props = Props::new();

        assert_eq!(props.get_byte(PropsKey::Company), 0);
        assert_eq!(props.get_short(PropsKey::Company), 0);
        assert_eq!(props.get_int(PropsKey::Company), 0);
        assert_eq!(props.get_double(PropsKey::Company), 0.0);
        assert_eq!(props.get_time(PropsKey::Company), None);
        assert_eq!(props.get_timestamp(PropsKey::Company), None);
        assert!(!props.get_boolean(PropsKey::Company));
        assert_eq!(props.get_unicode_string(PropsKey::Company), None);
        assert_eq!(props.get_uuid(PropsKey::Company), None);
        assert!(props.key_set().is_empty());
    }

    #[test]
    fn short_values_do_not_panic() {
        let mut props = Props::new();
        // Empty arrays and arrays that are too short: Java throws AIOOBE, Rust must return the default value
        props.put(PropsKey::Company.get_value(), Vec::new());
        props.put(PropsKey::Manager.get_value(), vec![0x01]);

        assert_eq!(props.get_byte(PropsKey::Company), 0);
        assert_eq!(props.get_short(PropsKey::Company), 0);
        assert_eq!(props.get_int(PropsKey::Company), 0);
        assert_eq!(props.get_double(PropsKey::Company), 0.0);
        assert_eq!(props.get_time(PropsKey::Company), None);
        assert_eq!(props.get_timestamp(PropsKey::Company), None);
        assert!(!props.get_boolean(PropsKey::Company));
        assert_eq!(props.get_short(PropsKey::Manager), 0);
        assert_eq!(props.get_int(PropsKey::Manager), 0);
        assert_eq!(props.get_timestamp(PropsKey::Manager), None);
    }

    #[test]
    fn byte_is_signed() {
        let mut props = Props::new();
        props.put(PropsKey::Company.get_value(), vec![0xFF]);

        // Java: byte is signed, (byte) 0xFF == -1
        assert_eq!(props.get_byte(PropsKey::Company), -1);
    }

    #[test]
    fn uuid_from_binary_and_from_string() {
        let mut props = Props::new();

        // 16-byte binary form (Java's getGUID requires the low 8 bytes to be non-zero, otherwise it returns null)
        let mut binary = vec![0u8; 16];
        binary[15] = 1;
        props.put(PropsKey::Guid.get_value(), binary);
        let uuid = props.get_uuid(PropsKey::Guid).expect("should parse binary GUID");
        assert_eq!(uuid.as_bytes()[15], 1);

        // All-zero binary form (Java: long1 == 0 && long2 == 0 -> null)
        props.put(PropsKey::Comments.get_value(), vec![0u8; 16]);
        assert_eq!(props.get_uuid(PropsKey::Comments), None);

        // MPP9 string form (76 bytes, UTF-16LE, "{" + 36 characters + "}")
        let text = "{01234567-89AB-CDEF-0123-456789ABCDEF}";
        assert_eq!(text.chars().count(), 38);
        let mut bytes = Vec::new();
        for ch in text.encode_utf16() {
            bytes.extend_from_slice(&ch.to_le_bytes());
        }
        bytes.extend_from_slice(&[0x00, 0x00]);
        props.put(PropsKey::Category.get_value(), bytes);
        assert_eq!(
            props.get_uuid(PropsKey::Category),
            Some(Uuid::parse_str("01234567-89AB-CDEF-0123-456789ABCDEF").expect("valid GUID"))
        );

        // Length 38 with braces but the content is not a valid GUID: Java throws IllegalArgumentException,
        // Rust returns None (without panicking)
        let text = format!("{{{}}}", "x".repeat(36));
        assert_eq!(text.chars().count(), 38);
        let mut bytes = Vec::new();
        for ch in text.encode_utf16() {
            bytes.extend_from_slice(&ch.to_le_bytes());
        }
        props.put(PropsKey::Title.get_value(), bytes);
        assert_eq!(props.get_uuid(PropsKey::Title), None);
    }

    #[test]
    fn display_matches_java_layout() {
        let mut props = Props::new();
        props.put(PropsKey::Company.get_value(), vec![0xAB, 0xCD]);
        // Unknown keys (not in PropsKey) should be printed in decimal
        props.put(99, vec![0x00]);

        let text = props.to_string();
        assert!(text.starts_with("BEGIN Props\n"));
        assert!(text.contains("   Key: COMPANY(37748736) Value: \n"));
        assert!(text.contains("     00000: AB CD"));
        assert!(text.contains("   Key: 99 Value: \n"));
        assert!(text.ends_with("END Props\n\n"));
    }

    #[test]
    fn duplicate_key_values_are_all_listed() {
        let mut props = Props::new();
        // 574619658 corresponds to both TABLE_NAME and UPPER_VIEW_NAME
        props.put(574619658, vec![0x01]);

        let text = props.to_string();
        assert!(text.contains("TABLE_NAME(574619658),UPPER_VIEW_NAME(574619658)"));
    }

    #[test]
    fn cursor_reads_and_stops_at_end() {
        let data = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        let mut cursor = InputStreamCursor::new(&data);

        assert_eq!(cursor.available(), 7);
        assert_eq!(cursor.read_int(), Some(0x04030201));
        assert_eq!(cursor.read_short(), Some(0x0605));
        assert_eq!(cursor.read(1), Some(&[0x07][..]));
        assert_eq!(cursor.available(), 0);

        // Out of bounds returns None without panicking
        assert_eq!(cursor.read(1), None);
        assert_eq!(cursor.read_int(), None);
        cursor.skip(10);
        assert_eq!(cursor.available(), 0);
    }
}
