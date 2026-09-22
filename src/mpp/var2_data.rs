//! Corresponds to Java: `org/mpxj/mpp/Var2Data.java`
//!
//! Variable-length data block: each data item in the block has the form "4-byte length + the data itself", and the position of each item is
//! determined by the offset index provided by [`VarMeta`] (`unique ID -> data type -> offset`). On construction this class reads all data items
//! by index into a `offset -> byte array` map, and then provides accessors keyed by `(unique ID, data type)`.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `Var2Data(ProjectFile file, VarMeta meta, InputStream is)` | `Var2Data::new(&Rc<RefCell<ProjectFile>>, Rc<dyn VarMeta>, &[u8])` | by convention `InputStream` -> `&[u8]`; `file` is only used to record "ignored errors" (see below) |
//! | `private final VarMeta m_meta` | `Rc<dyn VarMeta>` | Java stores a reference (the same instance may be stored separately by the reader); Rust reproduces that semantics with a shared handle |
//! | `TreeMap<Integer, byte[]> m_map` | `BTreeMap<i32, Vec<u8>>` | `TreeMap` -> `BTreeMap`: the traversal order of `toString` matches Java |
//! | `Integer` parameter (nullable) | `Option<i32>` | follows the "`null` -> `Option<T>`" convention |
//! | overloaded `getByteArray(Integer)` / `getByteArray(Integer, Integer)` | `get_byte_array_at_offset` / `get_byte_array` | Rust has no overloading: the version keyed by `(id, type)` keeps the original name (it has the most call sites in Java), while the offset-only version gets the `_at_offset` suffix. The same applies to `getString` / `getUnicodeString` |
//! | overloaded `getInt(Integer, Integer)` / `getInt(Integer, int, Integer)` | `get_int` / `get_int_at` | same as above; `get_int_at` keeps the same parameter order as Java (`id`, `offset`, `type`) |
//! | `String toString()` / `String toString(Integer id)` | `impl Display` / `to_string_for_id` | Rust has no overloading |
//! | when POI throws `IndexOutOfBoundsException`, `file.addIgnoredError(ex)` and `break` | when the data block is truncated (the length field itself cannot be read), record an [`MpxjError`] and stop parsing | slice out-of-bounds is predictable on the Rust side and no exception exists; the rest of the "best-effort tolerance of corrupt files" logic matches Java |
//! | Java's `is.reset()` / `InputStreamHelper.skip(is, n)` | internal cursor positioning (`position = 0` / `position + n`, no movement for negative values) | there is no stream on the Rust side; see the line-by-line comments in [`Var2Data::new`] |
//!
//! # This file's assumptions about related module interfaces (verified consistent)
//!
//! Java uses `MPPUtility` to decode strings / timestamps / single bytes, and the Rust side uses the same calls:
//!
//! ```text
//! MPPUtility::get_byte(data: &[u8], offset: usize) -> i32
//! MPPUtility::get_string(data: &[u8], offset: usize) -> String
//! MPPUtility::get_unicode_string(data: &[u8], offset: usize) -> String
//! MPPUtility::get_timestamp(data: &[u8], offset: usize) -> Option<NaiveDateTime>
//! ```
//!
//! Skipped Java methods: none (`Var2Data` has no `getDate` / `getDuration` accessors; dates and durations are decoded by the caller
//! first obtaining the byte array via `(id, type)` and then using `MPPUtility.getDate` / `getDuration`, so this class's
//! tests verify these two kinds of data in the same way).
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use chrono::NaiveDateTime;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpxj_exception::MpxjError;
use crate::mpp::abstract_var_meta::read_int;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::var_meta::VarMeta;
use crate::project_file::ProjectFile;

/// A block of variable-length data.
///
/// Corresponds to Java: `final class Var2Data extends MPPComponent`
pub struct Var2Data {
/// Data items, indexed by their offset in the block.
    ///
    /// Corresponds to Java field: `private final TreeMap<Integer, byte[]> m_map = new TreeMap<>();`
    map: BTreeMap<i32, Vec<u8>>,

/// Metadata associated with this block.
    ///
    /// Corresponds to Java field: `private final VarMeta m_meta;`
    meta: Rc<dyn VarMeta>,
}

impl Var2Data {
/// Constructor: read all variable-length data items from the data block using the offset index provided by the metadata.
    ///
    /// Corresponds to Java: `Var2Data(ProjectFile file, VarMeta meta, InputStream is) throws IOException`
    ///
/// The Rust side replaces `InputStream` with a byte slice, so Java's "current stream position / `reset` / `skip`"
/// is reproduced with the `position` cursor in this function:
/// - Java's `is.available()` -> number of remaining bytes;
/// - Java's `is.reset()` (return to the stream start) -> `position = 0`;
/// - Java's `InputStreamHelper.skip(is, n)` -> `position += n`, with no movement when `n <= 0`
/// (consistent with the `InputStream.skip` convention of not skipping for negative values).
    ///
/// `file` is used only for Java's `file.addIgnoredError(ex)` branch: when the data block ends before the length field is fully read
/// (Java throws `EOFException` / `IndexOutOfBoundsException` and construction fails), the Rust side records an
/// ignored error and stops parsing, never panicking.
    pub fn new(file: &Rc<RefCell<ProjectFile>>, meta: Rc<dyn VarMeta>, data: &[u8]) -> Self {
        let mut map: BTreeMap<i32, Vec<u8>> = BTreeMap::new();

        // Java: int currentOffset = 0; int available = is.available();
        let available = data.len();
        let mut current_offset: i32 = 0;
        let mut position = 0usize;

        // Java: for (int itemOffset : meta.getOffsets())
        for &item_offset in meta.get_offsets() {
            // Java: if (itemOffset >= available) { continue; }
            if item_offset >= available as i32 {
                continue;
            }

// Java comment: offsets may repeat (data items are deduplicated and shared). The map is keyed by offset, so duplicates would produce
// exactly identical entries; skip them directly to avoid resetting the stream position and seeking back to the stream start.
            // Java: if (m_map.containsKey(Integer.valueOf(itemOffset))) { continue; }
            if map.contains_key(&item_offset) {
                continue;
            }

            // Java: if (currentOffset > itemOffset) { is.reset(); InputStreamHelper.skip(is, itemOffset); }
            //       else { if (currentOffset < itemOffset) { InputStreamHelper.skip(is, itemOffset - currentOffset); } }
// the net effect of both branches is "move the read position to itemOffset".
            if current_offset > item_offset {
                position = skip(0, item_offset);
            } else if current_offset < item_offset {
// Java computes the difference with int arithmetic (which may wrap); the Rust side uses wrapping_sub to preserve the same semantics
                position = skip(position, item_offset.wrapping_sub(current_offset));
            }

            // Java: int size = readInt(is);
            let Some(size) = read_int(data, &mut position) else {
// Java: POI throws EOFException / IndexOutOfBoundsException when out of bounds, and the constructor fails as a result.
// The Rust side does not panic: it records an ignored error and stops parsing.
                if let Ok(mut file) = file.try_borrow_mut() {
                    file.add_ignored_error(MpxjError::new("Unexpected end of Var2Data block"));
                }
                break;
            };

// Java comment: best-effort tolerance of corrupt files
            // Java: if (size < 0 || size > is.available()) { continue; }
            if size < 0 || size as usize > data.len().saturating_sub(position) {
                continue;
            }

            // Java: data = readByteArray(is, size);
// (Java's try/catch IndexOutOfBoundsException branch is unreachable here: the length check above
// already guarantees the slice falls within range.)
            let item = data[position..position + size as usize].to_vec();
            position += size as usize;

            // Java: m_map.put(Integer.valueOf(itemOffset), data);
            //       currentOffset = itemOffset + 4 + size;
            map.insert(item_offset, item);
            current_offset = item_offset.wrapping_add(4).wrapping_add(size);
        }

        Self { map, meta }
    }

/// Get the bytes of a data item by offset; returns `None` when it does not exist.
    ///
    /// Corresponds to Java: `public byte[] getByteArray(Integer offset)`
    pub fn get_byte_array_at_offset(&self, offset: Option<i32>) -> Option<Vec<u8>> {
        // Java: byte[] result = null; if (offset != null) { result = m_map.get(offset); }
        let offset = offset?;
        self.map.get(&offset).cloned()
    }

/// Get the bytes of a data item by "unique ID + data type"; returns `None` when it does not exist.
    ///
    /// Corresponds to Java: `public byte[] getByteArray(Integer id, Integer type)`
    pub fn get_byte_array(&self, id: Option<i32>, type_: Option<i32>) -> Option<Vec<u8>> {
        // Java: return getByteArray(m_meta.getOffset(id, type));
        self.get_byte_array_at_offset(self.meta.get_offset(id, type_))
    }

/// Get data by offset and decode it as "double-byte characters (UTF-16LE, NUL-terminated)"; returns `None` when it does not exist.
    ///
    /// Corresponds to Java: `public String getUnicodeString(Integer offset)`
    pub fn get_unicode_string_at_offset(&self, offset: Option<i32>) -> Option<String> {
        let offset = offset?;
        let value = self.map.get(&offset)?;

        // Java: MPPUtility.getUnicodeString(value, 0);
        Some(MPPUtility::get_unicode_string(value, 0))
    }

/// Get data by "unique ID + data type" and decode it as UTF-16LE; returns `None` when it does not exist.
    ///
    /// Corresponds to Java: `public String getUnicodeString(Integer id, Integer type)`
    pub fn get_unicode_string(&self, id: Option<i32>, type_: Option<i32>) -> Option<String> {
        // Java: return getUnicodeString(m_meta.getOffset(id, type));
        self.get_unicode_string_at_offset(self.meta.get_offset(id, type_))
    }

/// Get a timestamp by "unique ID + data type"; returns `None` when it does not exist or the data is fewer than 4 bytes.
    ///
    /// Corresponds to Java: `public LocalDateTime getTimestamp(Integer id, Integer type)`
    pub fn get_timestamp(&self, id: Option<i32>, type_: Option<i32>) -> Option<NaiveDateTime> {
        let offset = self.meta.get_offset(id, type_)?;
        let value = self.map.get(&offset)?;

        // Java: if (value != null && value.length >= 4) { result = MPPUtility.getTimestamp(value, 0); }
        if value.len() < 4 {
            return None;
        }

        MPPUtility::get_timestamp(value, 0)
    }

/// Get data by offset and decode it as "single-byte characters (NUL-terminated)"; returns `None` when it does not exist.
    ///
    /// Corresponds to Java: `public String getString(Integer offset)`
    pub fn get_string_at_offset(&self, offset: Option<i32>) -> Option<String> {
        let offset = offset?;
        let value = self.map.get(&offset)?;

        // Java: MPPUtility.getString(value, 0);
        Some(MPPUtility::get_string(value, 0))
    }

/// Get data by "unique ID + data type" and decode it as single-byte characters; returns `None` when it does not exist.
    ///
    /// Corresponds to Java: `public String getString(Integer id, Integer type)`
    pub fn get_string(&self, id: Option<i32>, type_: Option<i32>) -> Option<String> {
        // Java: return getString(m_meta.getOffset(id, type));
        self.get_string_at_offset(self.meta.get_offset(id, type_))
    }

/// Get a 2-byte integer by "unique ID + data type" (`0..=65535`); returns 0 when it does not exist or the data is fewer than 2 bytes.
    ///
    /// Corresponds to Java: `public int getShort(Integer id, Integer type)`
    pub fn get_short(&self, id: Option<i32>, type_: Option<i32>) -> i32 {
        let mut result = 0;

        if let Some(offset) = self.meta.get_offset(id, type_) {
            if let Some(value) = self.map.get(&offset) {
                // Java: if (value != null && value.length >= 2) { result = ByteArrayHelper.getShort(value, 0); }
                if value.len() >= 2 {
                    result = ByteArrayHelper::get_short(value, 0);
                }
            }
        }

        result
    }

/// Get a 1-byte unsigned integer by "unique ID + data type" (`0..=255`); returns 0 when it does not exist.
    ///
    /// Corresponds to Java: `public int getByte(Integer id, Integer type)`
    ///
/// Java throws `ArrayIndexOutOfBoundsException` when calling `MPPUtility.getByte` on an empty array;
/// the Rust side returns 0 (see the "never panic" convention in the file header).
    pub fn get_byte(&self, id: Option<i32>, type_: Option<i32>) -> i32 {
        let mut result = 0;

        if let Some(offset) = self.meta.get_offset(id, type_) {
            if let Some(value) = self.map.get(&offset) {
                if !value.is_empty() {
                    // Java: result = MPPUtility.getByte(value, 0);
                    result = MPPUtility::get_byte(value, 0);
                }
            }
        }

        result
    }

/// Get a 4-byte integer by "unique ID + data type"; returns 0 when it does not exist or the data is fewer than 4 bytes.
    ///
    /// Corresponds to Java: `public int getInt(Integer id, Integer type)`
    pub fn get_int(&self, id: Option<i32>, type_: Option<i32>) -> i32 {
        let mut result = 0;

        if let Some(offset) = self.meta.get_offset(id, type_) {
            if let Some(value) = self.map.get(&offset) {
                // Java: if (value != null && value.length >= 4) { result = ByteArrayHelper.getInt(value, 0); }
                if value.len() >= 4 {
                    result = ByteArrayHelper::get_int(value, 0);
                }
            }
        }

        result
    }

/// Get a 4-byte integer at an arbitrary offset within a data item by "unique ID + data type"; returns 0 when unavailable.
    ///
    /// Corresponds to Java: `public int getInt(Integer id, int offset, Integer type)`
    ///
/// Java throws `ArrayIndexOutOfBoundsException` for a negative `offset`; the Rust side returns 0.
    pub fn get_int_at(&self, id: Option<i32>, offset: i32, type_: Option<i32>) -> i32 {
        let mut result = 0;

        if let Some(meta_offset) = self.meta.get_offset(id, type_) {
            if let Some(value) = self.map.get(&meta_offset) {
                // Java: if (value != null && value.length >= offset + 4) { result = ByteArrayHelper.getInt(value, offset); }
                if offset >= 0 && (offset as usize) + 4 <= value.len() {
                    result = ByteArrayHelper::get_int(value, offset as usize);
                }
            }
        }

        result
    }

/// Get an 8-byte integer by "unique ID + data type"; returns 0 when it does not exist or the data is fewer than 8 bytes.
    ///
    /// Corresponds to Java: `public long getLong(Integer id, Integer type)`
    pub fn get_long(&self, id: Option<i32>, type_: Option<i32>) -> i64 {
        let mut result = 0;

        if let Some(offset) = self.meta.get_offset(id, type_) {
            if let Some(value) = self.map.get(&offset) {
                // Java: if (value != null && value.length >= 8) { result = ByteArrayHelper.getLong(value, 0); }
                if value.len() >= 8 {
                    result = ByteArrayHelper::get_long(value, 0);
                }
            }
        }

        result
    }

/// Get an 8-byte float by "unique ID + data type"; returns 0 when it does not exist or the data is fewer than 8 bytes.
    ///
    /// Corresponds to Java: `public double getDouble(Integer id, Integer type)`
    ///
/// Consistent with Java: interpret the bit pattern of `getLong` as a `double`, normalizing `NaN` to 0.
    pub fn get_double(&self, id: Option<i32>, type_: Option<i32>) -> f64 {
        // Java: double result = Double.longBitsToDouble(getLong(id, type)); if (Double.isNaN(result)) { result = 0; }
        let mut result = f64::from_bits(self.get_long(id, type_) as u64);
        if result.is_nan() {
            result = 0.0;
        }
        result
    }

/// Get the underlying metadata (mainly for debugging).
    ///
    /// Corresponds to Java: `public VarMeta getVarMeta()`
    pub fn get_var_meta(&self) -> &dyn VarMeta {
        &*self.meta
    }

/// Export all data items of this block as debug text.
    ///
    /// Corresponds to Java: `public String toString(Integer id)`
    ///
/// The Rust side cannot overload `to_string`, so this method is renamed `to_string_for_id` (for whole-block export see `Display`).
/// Returns empty text when `id` is `None` (Java throws an NPE for `null` because of `getTypes(null)`).
    pub fn to_string_for_id(&self, id: Option<i32>) -> String {
        let mut result = String::new();
        result.push_str(&format!("BEGIN Var2Data for {}\n", format_id(id)));

        for type_ in self.meta.get_types(id) {
            let offset = self.meta.get_offset(id, Some(type_));
            let data = offset.and_then(|offset| self.map.get(&offset));

            let data: &[u8] = match &data {
                Some(data) => data,
// Java: data.length throws an NPE; the Rust side outputs as empty data (does not panic)
                None => &[],
            };
            let size = data.len();
            result.push_str(&format!(
                "   Data at offset: {} size: {size}\n",
                format_id(offset)
            ));
            result.push_str(&ByteArrayHelper::hexdump_columns(data, true, 16, "   "));
        }

        result.push_str(&format!("END Var2Data for {}\n\n", format_id(id)));
        result
    }
}

impl Display for Var2Data {
/// Output the same text as Java's `toString()`.
    ///
    /// Corresponds to Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut result = String::new();
        result.push_str("BEGIN Var2Data\n");

        for (offset, data) in &self.map {
            let size = data.len();
            result.push_str(&format!("   Data at offset: {offset} size: {size}\n"));
            result.push_str(&ByteArrayHelper::hexdump_columns(data, true, 16, "   "));
        }

        result.push_str("END Var2Data\n\n");
        f.write_str(&result)
    }
}

/// Mimic Java's `InputStreamHelper.skip`: no movement when `count <= 0` (same as `InputStream.skip`).
fn skip(position: usize, count: i32) -> usize {
    if count <= 0 {
        position
    } else {
        position.saturating_add(count as usize)
    }
}

/// Render a nullable integer the way Java string concatenation does (`null` -> `"null"`).
fn format_id(value: Option<i32>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpp::var_meta9::VarMeta9;
    use chrono::{Days, NaiveDate, NaiveTime};

/// Manually assemble a Var2Data data block, returning "the data block + the offset of each item".
    ///
/// Each item has the same form as MPP: 4-byte length + the data itself.
    fn data_block(items: &[Vec<u8>]) -> (Vec<u8>, Vec<i32>) {
        let mut data = Vec::new();
        let mut offsets = Vec::new();

        for item in items {
            offsets.push(data.len() as i32);
            data.extend_from_slice(&(item.len() as i32).to_le_bytes());
            data.extend_from_slice(item);
        }

        (data, offsets)
    }

/// Assemble the matching VarMeta9 block from "unique ID + type + data item index".
    fn var_meta9_bytes(entries: &[(i32, i32, usize)], offsets: &[i32], data_size: i32) -> Vec<u8> {
        let mut data = Vec::new();
// magic = 0xFADFADBA (little-endian)
        data.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(entries.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&data_size.to_le_bytes());

        for (unique_id, type_, index) in entries {
            data.extend_from_slice(&unique_id.to_le_bytes()[..3]);
            data.push(*type_ as u8);
            data.extend_from_slice(&offsets[*index].to_le_bytes());
        }

        data
    }

/// Test fixture: the data block, matching metadata, and the `(id, type)` of each data item.
    ///
/// Each data item (in offset order):
/// 0) `ID=1 TYPE=0x12` -> 1 byte 42
/// 1) `ID=2 TYPE=0x12` -> 2 bytes 0x1234 (4660)
/// 2) `ID=3 TYPE=0x12` -> 4 bytes 0x12345678
/// 3) `ID=4 TYPE=0x12` -> 8-byte double 1.5
/// 4) `ID=5 TYPE=0x12` -> 8-byte long 1
/// 5) `ID=6 TYPE=0x12` -> single-byte string "AB"
/// 6) `ID=7 TYPE=0x12` -> UTF-16LE string "AB"
/// 7) `ID=8 TYPE=0x12` -> timestamp (seconds 0, days 1000)
/// 8) `ID=9 TYPE=0x12` -> date bytes (days 1 -> 1984-01-01)
/// 9) `ID=10 TYPE=0x12` -> duration bytes (4800 tenths of a minute -> 8 hours)
/// 10) `ID=11 TYPE=0x12` -> date NA (65535)
/// 11) `ID=12 TYPE=0x12` -> 6 bytes, used to verify reading an integer from an arbitrary offset
/// 12) `ID=13 TYPE=0x12` -> empty data (0 bytes)
    fn sample() -> (Vec<u8>, Vec<u8>, Vec<(i32, i32)>) {
        let items = vec![
            vec![0x2A],
            vec![0x34, 0x12],
            vec![0x78, 0x56, 0x34, 0x12],
            vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF8, 0x3F],
            vec![0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            vec![0x41, 0x42, 0x00],
            vec![0x41, 0x00, 0x42, 0x00, 0x00, 0x00],
            vec![0x00, 0x00, 0xE8, 0x03],
            vec![0x01, 0x00],
            vec![0xC0, 0x12],
            vec![0xFF, 0xFF],
            vec![0x00, 0x00, 0x78, 0x56, 0x34, 0x12],
            Vec::new(),
        ];

        let (data, offsets) = data_block(&items);
        let entries: Vec<(i32, i32, usize)> = (0..items.len())
            .map(|index| (index as i32 + 1, 0x12, index))
            .collect();
        let meta = var_meta9_bytes(&entries, &offsets, data.len() as i32);
        let ids: Vec<(i32, i32)> = entries.iter().map(|entry| (entry.0, entry.1)).collect();

        (data, meta, ids)
    }

/// Construct a Var2Data fixture.
    fn sample_var_data() -> (Var2Data, Rc<RefCell<ProjectFile>>) {
        let (data, meta_bytes, _) = sample();
        let meta = VarMeta9::new(&meta_bytes).expect("valid metadata");
        let file = ProjectFile::new();
        let var_data = Var2Data::new(&file, Rc::new(meta), &data);
        (var_data, file)
    }

/// Decode a date per MPXJ rules (corresponds to `MPPUtility.getDate`, called by the caller on the Java side).
    ///
/// A thin wrapper is kept here so that the test's expected-value expressions match the Java call sites.
    fn decode_date(data: &[u8]) -> Option<NaiveDate> {
        MPPUtility::get_date(data, 0)
    }

/// Decode a duration per MPXJ rules (corresponds to `MPPUtility.getDuration`): returns milliseconds.
    fn decode_duration(data: &[u8]) -> i64 {
        MPPUtility::get_duration(data, 0)
    }

/// The metadata provides the item count, and each item can be retrieved by `(id, type)`.
    #[test]
    fn parses_all_items_indexed_by_id_and_type() {
        let (var_data, _file) = sample_var_data();
        let (_, meta_bytes, ids) = sample();

// item count parsing
        let meta = VarMeta9::new(&meta_bytes).expect("valid metadata");
        assert_eq!(meta.get_item_count(), 13);
        assert_eq!(var_data.get_var_meta().get_item_count(), 13);
        assert_eq!(var_data.get_var_meta().get_offsets().len(), 13);

// retrieve various types by (id, type)
        let (id, type_) = ids[0];
        assert_eq!(var_data.get_byte(Some(id), Some(type_)), 42);
        assert_eq!(
            var_data.get_byte_array(Some(id), Some(type_)),
            Some(vec![0x2A])
        );

        let (id, type_) = ids[1];
        assert_eq!(var_data.get_short(Some(id), Some(type_)), 0x1234);

        let (id, type_) = ids[2];
        assert_eq!(var_data.get_int(Some(id), Some(type_)), 0x12345678);

        let (id, type_) = ids[3];
        assert_eq!(var_data.get_double(Some(id), Some(type_)), 1.5);

        let (id, type_) = ids[4];
        assert_eq!(var_data.get_long(Some(id), Some(type_)), 1);

        let (id, type_) = ids[5];
        assert_eq!(
            var_data.get_string(Some(id), Some(type_)),
            Some("AB".to_owned())
        );

        let (id, type_) = ids[6];
        assert_eq!(
            var_data.get_unicode_string(Some(id), Some(type_)),
            Some("AB".to_owned())
        );

// timestamp: 1983-12-31 + 1000 days, seconds 0
        let (id, type_) = ids[7];
        let expected_date = NaiveDate::from_ymd_opt(1983, 12, 31)
            .and_then(|epoch| epoch.checked_add_days(Days::new(1000)))
            .expect("date is computable");
        assert_eq!(
            var_data.get_timestamp(Some(id), Some(type_)),
            Some(NaiveDateTime::new(
                expected_date,
                NaiveTime::from_hms_opt(0, 0, 0).expect("valid time")
            ))
        );

// read an integer from an arbitrary offset within the data item
        let (id, type_) = ids[11];
        assert_eq!(var_data.get_int_at(Some(id), 2, Some(type_)), 0x12345678);
        assert_eq!(var_data.get_int(Some(id), Some(type_)), 0x5678_0000);
// negative offset (Java throws ArrayIndexOutOfBoundsException) -> 0
        assert_eq!(var_data.get_int_at(Some(id), -1, Some(type_)), 0);
// beyond the data item length -> 0
        assert_eq!(var_data.get_int_at(Some(id), 3, Some(type_)), 0);

// empty data item: Java's getByte throws, the Rust side returns 0
        let (id, type_) = ids[12];
        assert_eq!(var_data.get_byte_array(Some(id), Some(type_)), Some(Vec::new()));
        assert_eq!(var_data.get_byte(Some(id), Some(type_)), 0);
        assert_eq!(var_data.get_int(Some(id), Some(type_)), 0);
    }

/// Dates and durations: Var2Data provides bytes by `(id, type)`, and the caller decodes them per MPXJ rules.
    #[test]
    fn date_and_duration_items_decode_from_retrieved_bytes() {
        let (var_data, _file) = sample_var_data();
        let (_, _, ids) = sample();

// date: 2 bytes of "days since 1983-12-31"
        let (id, type_) = ids[8];
        let date_bytes = var_data
            .get_byte_array(Some(id), Some(type_))
            .expect("has data");
        assert_eq!(date_bytes, vec![0x01, 0x00]);
        assert_eq!(decode_date(&date_bytes), NaiveDate::from_ymd_opt(1984, 1, 1));

// date NA (65535)
        let (id, type_) = ids[10];
        let na_bytes = var_data
            .get_byte_array(Some(id), Some(type_))
            .expect("has data");
        assert_eq!(decode_date(&na_bytes), None);

// duration: 2 bytes of "tenths of a minute", 4800 -> 8 hours
        let (id, type_) = ids[9];
        let duration_bytes = var_data
            .get_byte_array(Some(id), Some(type_))
            .expect("has data");
        assert_eq!(decode_duration(&duration_bytes), 8 * 60 * 60 * 1000);
    }

/// Get a value by offset (Java's `getByteArray(Integer)` overload).
    #[test]
    fn values_can_be_read_by_offset() {
        let (data, meta_bytes, ids) = sample();
        let meta = VarMeta9::new(&meta_bytes).expect("valid metadata");
        let file = ProjectFile::new();
        let var_data = Var2Data::new(&file, Rc::new(meta), &data);

        let (id, type_) = ids[5];
        let offset = var_data
            .get_var_meta()
            .get_offset(Some(id), Some(type_))
            .expect("has offset");

        assert_eq!(
            var_data.get_byte_array_at_offset(Some(offset)),
            Some(vec![0x41, 0x42, 0x00])
        );
        assert_eq!(
            var_data.get_string_at_offset(Some(offset)),
            Some("AB".to_owned())
        );
        assert_eq!(var_data.get_byte_array_at_offset(None), None);
        assert_eq!(var_data.get_byte_array_at_offset(Some(9999)), None);
        assert_eq!(var_data.get_string_at_offset(Some(9999)), None);
        assert_eq!(var_data.get_unicode_string_at_offset(Some(9999)), None);
    }

/// A missing item returns a default value (Java returns 0 / null).
    #[test]
    fn missing_entries_return_java_defaults() {
        let (var_data, _file) = sample_var_data();
        let missing_id = Some(4242);
        let type_ = Some(0x12);

        assert_eq!(var_data.get_byte(missing_id, type_), 0);
        assert_eq!(var_data.get_short(missing_id, type_), 0);
        assert_eq!(var_data.get_int(missing_id, type_), 0);
        assert_eq!(var_data.get_int_at(missing_id, 0, type_), 0);
        assert_eq!(var_data.get_long(missing_id, type_), 0);
        assert_eq!(var_data.get_double(missing_id, type_), 0.0);
        assert_eq!(var_data.get_string(missing_id, type_), None);
        assert_eq!(var_data.get_unicode_string(missing_id, type_), None);
        assert_eq!(var_data.get_timestamp(missing_id, type_), None);
        assert_eq!(var_data.get_byte_array(missing_id, type_), None);

// type mismatch / parameter is None (Java's null)
        assert_eq!(var_data.get_byte(Some(4242), Some(0x99)), 0);
        assert_eq!(var_data.get_int(None, None), 0);
        assert_eq!(var_data.get_byte_array(None, None), None);
    }

/// Does not panic when the offset is out of bounds / the length field is corrupt, and does not write an item either.
    #[test]
    fn out_of_range_and_corrupt_items_are_ignored() {
// the data block is only 4 bytes (length field 0, no data)
        let data = 0i32.to_le_bytes().to_vec();

// offset 1000 exceeds the data block (Java: itemOffset >= available -> continue)
        let meta_bytes = var_meta9_bytes(&[(1, 0x12, 0)], &[1000], 4);
        let meta = VarMeta9::new(&meta_bytes).expect("valid metadata");
        let file = ProjectFile::new();
        let var_data = Var2Data::new(&file, Rc::new(meta), &data);
        assert_eq!(var_data.get_byte_array(Some(1), Some(0x12)), None);
        assert!(file.borrow().get_ignored_errors().is_empty());

// negative offset (Java likewise reads from the stream start): does not panic
        let meta_bytes = var_meta9_bytes(&[(1, 0x12, 0)], &[-8], 4);
        let meta = VarMeta9::new(&meta_bytes).expect("valid metadata");
        let file = ProjectFile::new();
        let var_data = Var2Data::new(&file, Rc::new(meta), &data);
        assert_eq!(
            var_data.get_byte_array_at_offset(Some(-8)),
            Some(Vec::new())
        );

// length field is negative -> continue
        let mut negative = (-1i32).to_le_bytes().to_vec();
        negative.extend_from_slice(&[0x2A, 0x2A, 0x2A]);
        let meta_bytes = var_meta9_bytes(&[(1, 0x12, 0)], &[0], 7);
        let meta = VarMeta9::new(&meta_bytes).expect("valid metadata");
        let file = ProjectFile::new();
        let var_data = Var2Data::new(&file, Rc::new(meta), &negative);
        assert_eq!(var_data.get_byte_array(Some(1), Some(0x12)), None);

// length field is greater than the remaining bytes -> continue
        let mut oversized = 100i32.to_le_bytes().to_vec();
        oversized.extend_from_slice(&[0x2A, 0x2A]);
        let meta_bytes = var_meta9_bytes(&[(1, 0x12, 0)], &[0], 6);
        let meta = VarMeta9::new(&meta_bytes).expect("valid metadata");
        let file = ProjectFile::new();
        let var_data = Var2Data::new(&file, Rc::new(meta), &oversized);
        assert_eq!(var_data.get_byte_array(Some(1), Some(0x12)), None);
    }

/// Records an ignored error and stops parsing when the data block is truncated (the length field itself cannot be read).
    #[test]
    fn truncated_block_records_ignored_error() {
// the data block is 6 bytes long; only 2 bytes remain at offset 4, so the length field cannot be read
        let data = vec![0u8; 6];
        let meta_bytes = var_meta9_bytes(&[(1, 0x12, 0)], &[4], 6);
        let meta = VarMeta9::new(&meta_bytes).expect("valid metadata");
        let file = ProjectFile::new();

        let var_data = Var2Data::new(&file, Rc::new(meta), &data);

        assert_eq!(var_data.get_byte_array(Some(1), Some(0x12)), None);
        let errors = file.borrow();
        assert_eq!(errors.get_ignored_errors().len(), 1);
        assert_eq!(
            errors.get_ignored_errors()[0].message(),
            "Unexpected end of Var2Data block"
        );
    }

/// Only one item is kept for a duplicate offset (Java's deduplication logic).
    #[test]
    fn repeated_offsets_are_deduplicated() {
        let (data, offsets) = data_block(&[vec![0x2A], vec![0x99]]);

// both items point to the first entry
        let meta_bytes = var_meta9_bytes(&[(1, 0x12, 0), (2, 0x12, 0)], &offsets, data.len() as i32);
        let meta = VarMeta9::new(&meta_bytes).expect("valid metadata");
        let file = ProjectFile::new();
        let var_data = Var2Data::new(&file, Rc::new(meta), &data);

        assert_eq!(var_data.get_byte_array(Some(1), Some(0x12)), Some(vec![0x2A]));
        assert_eq!(var_data.get_byte_array(Some(2), Some(0x12)), Some(vec![0x2A]));

// the map should contain only one record at offset 0
        let text = var_data.to_string();
        assert_eq!(text.matches("Data at offset: 0 ").count(), 1);
    }

/// The debug output matches Java's format.
    #[test]
    fn to_string_matches_java_format() {
        let (data, meta_bytes, _) = sample();
        let meta = VarMeta9::new(&meta_bytes).expect("valid metadata");
        let file = ProjectFile::new();
        let var_data = Var2Data::new(&file, Rc::new(meta), &data);

        let text = var_data.to_string();
        assert!(text.starts_with("BEGIN Var2Data\n"));
        assert!(text.ends_with("END Var2Data\n\n"));
        assert!(text.contains("   Data at offset: 0 size: 1\n"));
// each line of the hexdump is prefixed with a 5-digit offset
        assert!(text.contains("   00000: 2A   *"));

        let by_id = var_data.to_string_for_id(Some(1));
        assert!(by_id.starts_with("BEGIN Var2Data for 1\n"));
        assert!(by_id.contains("   Data at offset: 0 size: 1\n"));
        assert!(by_id.ends_with("END Var2Data for 1\n\n"));

// unknown ID: no entries, only the header and footer
        assert_eq!(
            var_data.to_string_for_id(Some(999)),
            "BEGIN Var2Data for 999\nEND Var2Data for 999\n\n"
        );
// Java throws an NPE for null; the Rust side outputs "null"
        assert!(var_data.to_string_for_id(None).contains("for null"));
    }

/// `getVarMeta` returns the metadata passed in at construction.
    #[test]
    fn exposes_its_metadata() {
        let (var_data, _file) = sample_var_data();

        assert_eq!(var_data.get_var_meta().get_item_count(), 13);
        assert_eq!(var_data.get_var_meta().get_data_size(), 100);
        assert!(var_data.get_var_meta().contains_key(Some(1)));
        assert_eq!(
            var_data.get_var_meta().get_types(Some(1)),
            std::collections::BTreeSet::from([0x12])
        );
    }
}
