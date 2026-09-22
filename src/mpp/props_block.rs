//! Corresponds to Java: `org/mpxj/mpp/PropsBlock.java`

use std::collections::BTreeMap;
use std::fmt;

use chrono::{NaiveDateTime, NaiveTime};
use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::props::{
    Props, value_double, value_int, value_short, value_timestamp, value_unicode_string,
};
use crate::mpp::props_key::PropsKey;

/// A block of property data.
///
/// Corresponds to Java: `final class PropsBlock extends Props`
///
/// # Restoring inheritance (composition + forwarding)
///
/// Java's `PropsBlock` extends [`Props`], so it has two sets of accessor methods:
/// the key-based (`PropsKey`) versions from `Props`, plus this class's new type-based (`Integer`) versions.
/// Rust has no inheritance, so this class **holds a [`Props`]**:
///
/// - The 11 methods that access by `PropsKey` (`get_byte_array` / `get_byte` / `get_short` / …)
/// forward directly to the inner `Props`, semantically equivalent to Java inheritance;
/// - The 6 methods that access by `Integer` form an **overload** with the former in Java; Rust has no overloading,
/// so they uniformly get a `_by_type` suffix (`get_byte_array_by_type` / `get_short_by_type` / …).
///
/// # Block layout (Java constructor)
///
/// ```text
/// Offset 0   int  dataSize    total data size (used as the end offset of the last property)
/// Offset 4   int  itemCount   number of index table entries
/// Offset 8   itemCount × (int itemKey, int itemOffset)   index table
/// After that                raw data for each property
/// ```
///
/// `itemOffset` is an **absolute offset within the block** (Java's `populateMap` computes from `data` directly).
/// Java sorts by `itemOffset` ascending with a `TreeMap`, then slices each property using "previous offset → current offset"
/// byte range; the last property extends to `dataSize`; the first property is written on the second iteration,
/// so even a single-entry block retrieves its data correctly.
///
/// # Differences from Java
///
/// - Java throws `AIOOBE` / `NegativeArraySizeException` when `System.arraycopy` is out of bounds or the length is negative;
/// the Rust side leaves the corresponding data unprocessed (that property is not written) and never panics.
/// - Java's `toString()` is inherited from `Props`; the Rust side likewise forwards to [`Props`]'s [`Display`] implementation.
///
/// Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: not used by this class.
pub struct PropsBlock {
/// Corresponds to Java: `extends Props` (Rust uses composition instead of inheritance)
    props: Props,
}

impl PropsBlock {
/// Parse property data according to the block layout.
    ///
    /// Corresponds to Java: `PropsBlock(byte[] data)`
    pub fn new(data: &[u8]) -> Self {
        let mut result = Self { props: Props::new() };

        // Java: int dataSize = ByteArrayHelper.getInt(data, 0)
// Java throws AIOOBE when there is insufficient data; Rust returns 0
        let data_size = if data.len() >= 4 {
            ByteArrayHelper::get_int(data, 0)
        } else {
            0
        };

        // Java: int itemCount = ByteArrayHelper.getInt(data, 4)
        let item_count = if data.len() >= 8 {
            ByteArrayHelper::get_int(data, 4)
        } else {
            0
        };

// Java: offset table (TreeMap guarantees iteration in ascending itemOffset order)
        let mut offset_map: BTreeMap<i32, i32> = BTreeMap::new();
        let mut offset = 8usize;
        for _ in 0..item_count {
// Java: throws AIOOBE when out of bounds; Rust stops reading the index table immediately
            if offset + 8 > data.len() {
                break;
            }

            let item_key = ByteArrayHelper::get_int(data, offset);
            offset += 4;

            let item_offset = ByteArrayHelper::get_int(data, offset);
            offset += 4;

            offset_map.insert(item_offset, item_key);
        }

        let mut previous_item_offset: Option<i32> = None;
        let mut previous_item_key: Option<i32> = None;

        for item_offset in offset_map.keys() {
            if let (Some(previous_offset), Some(previous_key)) =
                (previous_item_offset, previous_item_key)
            {
                result.populate_map(data, previous_offset, previous_key, *item_offset);
            }

            previous_item_offset = Some(*item_offset);
            previous_item_key = offset_map.get(item_offset).copied();
        }

        if let (Some(previous_offset), Some(previous_key)) =
            (previous_item_offset, previous_item_key)
        {
            // Java: Integer itemOffset = Integer.valueOf(dataSize); populateMap(...)
            result.populate_map(data, previous_offset, previous_key, data_size);
        }

        result
    }

/// Get property data (raw bytes).
    ///
    /// Corresponds to Java: `public byte[] getByteArray(Integer type)`
    pub fn get_byte_array_by_type(&self, type_: i32) -> Option<&[u8]> {
        self.props.get_byte_array_by_key(type_)
    }

/// Get a short integer value.
    ///
    /// Corresponds to Java: `public int getShort(Integer type)`
    pub fn get_short_by_type(&self, type_: i32) -> i32 {
        value_short(self.get_byte_array_by_type(type_))
    }

/// Get an integer value.
    ///
    /// Corresponds to Java: `public int getInt(Integer type)`
    pub fn get_int_by_type(&self, type_: i32) -> i32 {
        value_int(self.get_byte_array_by_type(type_))
    }

/// Get a double value.
    ///
    /// Corresponds to Java: `public double getDouble(Integer type)`
    pub fn get_double_by_type(&self, type_: i32) -> f64 {
        value_double(self.get_byte_array_by_type(type_))
    }

/// Get a timestamp.
    ///
    /// Corresponds to Java: `public LocalDateTime getTimestamp(Integer type)`
    pub fn get_timestamp_by_type(&self, type_: i32) -> Option<NaiveDateTime> {
        value_timestamp(self.get_byte_array_by_type(type_))
    }

/// Get a UTF-16LE string.
    ///
    /// Corresponds to Java: `public String getUnicodeString(Integer type)`
    pub fn get_unicode_string_by_type(&self, type_: i32) -> Option<String> {
        value_unicode_string(self.get_byte_array_by_type(type_))
    }

/// Slice a section of data from the block and write it to the property table.
    ///
    /// Corresponds to Java: `private void populateMap(byte[] data, Integer previousItemOffset, Integer previousItemKey, Integer itemOffset)`
    fn populate_map(
        &mut self,
        data: &[u8],
        previous_item_offset: i32,
        previous_item_key: i32,
        item_offset: i32,
    ) {
// Java: int itemSize = itemOffset - previousItemOffset; when negative it throws
// NegativeArraySizeException, and when out of bounds it throws AIOOBE; Rust skips that property
        if previous_item_offset > item_offset {
            return;
        }

        let start = previous_item_offset.max(0) as usize;
        let end = item_offset.max(0) as usize;
        if end > data.len() {
            return;
        }

        self.props.put(previous_item_key, data[start..end].to_vec());
    }

    // -----------------------------------------------------------------
// The following methods are inherited from Props in Java; the Rust side forwards to the internally held Props
    // -----------------------------------------------------------------

/// Get property data (raw bytes).
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

/// Get a short integer value.
    ///
    /// Corresponds to Java: `Props.getShort(PropsKey key)`
    pub fn get_short(&self, key: PropsKey) -> i32 {
        self.props.get_short(key)
    }

/// Get an integer value.
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

impl fmt::Display for PropsBlock {
    /// Corresponds to Java: `Props.toString()`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.props, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// Build data according to Java's block layout: dataSize + itemCount + index table + property data.
    ///
/// Offsets in the index table are **absolute offsets within the block** (Java's `populateMap` computes from `data` directly),
/// and the last property uses `dataSize` as its end offset — so `dataSize` here equals the block's total length.
    fn build_block(items: &[(i32, Vec<u8>)]) -> Vec<u8> {
        let base = 8 + 8 * items.len();
        let mut data = vec![0u8; base];
        let mut offset = base;

        for (index, (key, value)) in items.iter().enumerate() {
            data[8 + (index * 8)..8 + (index * 8) + 4].copy_from_slice(&key.to_le_bytes());
            data[8 + (index * 8) + 4..8 + (index * 8) + 8]
                .copy_from_slice(&(offset as i32).to_le_bytes());
            offset += value.len();
        }

        for (_, value) in items {
            data.extend_from_slice(value);
        }

        let total = data.len() as i32;
        data[0..4].copy_from_slice(&total.to_le_bytes());
        data[4..8].copy_from_slice(&(items.len() as i32).to_le_bytes());

        data
    }

    #[test]
    fn reads_block_items() {
// The index table is sorted by itemOffset (Java uses TreeMap), i.e. declaration order
        let items = [
            (101, vec![0x01, 0x02]),                         // short integer 513
            (102, vec![0xD2, 0x04, 0x00, 0x00]),             // integer 1234
            (103, vec![0x41, 0x00, 0x42, 0x00, 0x00, 0x00]), // string "AB"
        ];
        let data = build_block(&items);

        let block = PropsBlock::new(&data);

        assert_eq!(block.key_set(), vec![101, 102, 103]);
        assert_eq!(block.get_byte_array_by_type(101), Some(&[0x01, 0x02][..]));
        assert_eq!(block.get_short_by_type(101), 513);
        assert_eq!(block.get_int_by_type(102), 1234);
        assert_eq!(
            block.get_unicode_string_by_type(103),
            Some("AB".to_owned())
        );
    }

    #[test]
    fn missing_type_returns_defaults() {
        let data = build_block(&[(101, vec![0x01, 0x02])]);
        let block = PropsBlock::new(&data);

        assert_eq!(block.get_byte_array_by_type(999), None);
        assert_eq!(block.get_short_by_type(999), 0);
        assert_eq!(block.get_int_by_type(999), 0);
        assert_eq!(block.get_double_by_type(999), 0.0);
        assert_eq!(block.get_timestamp_by_type(999), None);
        assert_eq!(block.get_unicode_string_by_type(999), None);

// When a property that is too short (2 bytes) is read as int/double, Java throws AIOOBE; Rust returns the default value
        assert_eq!(block.get_int_by_type(101), 0);
        assert_eq!(block.get_double_by_type(101), 0.0);
        assert_eq!(block.get_timestamp_by_type(101), None);
    }

    #[test]
    fn last_item_ends_at_data_size() {
        let items = [
            (1, vec![0xAA]),
            (2, vec![0xBB, 0xCC, 0xDD]),
        ];
        let data = build_block(&items);

        let block = PropsBlock::new(&data);

        assert_eq!(block.get_byte_array_by_type(1), Some(&[0xAA][..]));
        assert_eq!(
            block.get_byte_array_by_type(2),
            Some(&[0xBB, 0xCC, 0xDD][..])
        );
    }

    #[test]
    fn malformed_data_does_not_panic() {
// Empty data
        let block = PropsBlock::new(&[]);
        assert!(block.key_set().is_empty());

// itemCount claims 3 entries, but the index table is truncated
        let mut data = Vec::new();
        data.extend_from_slice(&16i32.to_le_bytes());
        data.extend_from_slice(&3i32.to_le_bytes());
        data.extend_from_slice(&1i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        let block = PropsBlock::new(&data);
        assert_eq!(block.key_set(), vec![1]);

// dataSize exceeds the actual data length: Java throws AIOOBE, Rust skips that property
        let mut data = Vec::new();
        data.extend_from_slice(&1000i32.to_le_bytes());
        data.extend_from_slice(&1i32.to_le_bytes());
        data.extend_from_slice(&1i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&[0x01, 0x02]);
        let block = PropsBlock::new(&data);
        assert!(block.key_set().is_empty());

// The last entry's offset is greater than dataSize: Java throws NegativeArraySizeException, Rust skips it
        let mut data = Vec::new();
        data.extend_from_slice(&2i32.to_le_bytes());
        data.extend_from_slice(&1i32.to_le_bytes());
        data.extend_from_slice(&7i32.to_le_bytes());
        data.extend_from_slice(&5i32.to_le_bytes());
        data.resize(21, 0);
        data.extend_from_slice(&[0x01, 0x02]);
        let block = PropsBlock::new(&data);
        assert!(block.key_set().is_empty());
    }

    #[test]
    fn forwards_props_key_methods() {
        let data = build_block(&[(502, vec![0x07, 0x00, 0x00, 0x00])]);
        let block = PropsBlock::new(&data);

// Key value 502 is not in PropsKey, so key-based access returns the default value (proving the forwarding path works)
        assert_eq!(block.get_int(PropsKey::Company), 0);
        assert_eq!(block.get_unicode_string(PropsKey::Company), None);
        assert!(block.to_string().starts_with("BEGIN Props\n"));
    }
}
