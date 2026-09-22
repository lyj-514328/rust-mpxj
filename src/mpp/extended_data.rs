//! Corresponds to Java: `org/mpxj/mpp/ExtendedData.java`

use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};

use chrono::NaiveDateTime;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::fix_defer_fix::FixDeferFix;
use crate::mpp::mpp_utility::MppUtility;

/// Extended data structure: holds additional non-core data items associated with tasks and resources.
///
/// Corresponds to Java: `final class ExtendedData` (package-visible)
///
/// On construction, parse the data block at the specified offset in the var data block item by item as `[length:4][type:4][data]`
/// and put it into a map as "type -> data".
///
/// # Porting notes (trade-offs)
///
/// - Java's `private final FixDeferFix m_data` "holds a reference"; the Rust side expresses this as
/// `&'a FixDeferFix` (an immutable borrow); the caller only needs to ensure var data outlives this object
/// (in Java, `ExtendedData` also only uses a local var data within the same method).
/// - Java's `byte[] getByteArray(Integer)` returns the internal array itself; the Rust side returns
/// `Option<&[u8]>` (borrows the internal storage, no copy).
/// - Java throws exceptions in several places: out-of-bounds while parsing the item header (`ArrayIndexOutOfBoundsException`),
/// a negative `size` (`NegativeArraySizeException`), a sub-array past the end of the data
/// (`IndexOutOfBoundsException`). The Rust side always changes these to "stop parsing / skip the item / return the default value"
/// with a comment, and never panics.
pub struct ExtendedData<'a> {
    /// Corresponds to Java field `m_data`
    m_data: &'a FixDeferFix,
/// Corresponds to Java field `m_map` (type -> data item)
    m_map: HashMap<i32, Vec<u8>>,
}

impl<'a> ExtendedData<'a> {
/// Constructor. Takes the extended data block at `offset` in the var data block as input.
    ///
    /// Corresponds to Java: `ExtendedData(FixDeferFix varData, int offset)`
    pub fn new(var_data: &'a FixDeferFix, offset: i32) -> Self {
        let mut map = HashMap::new();

        if let Some(data) = var_data.get_byte_array(offset) {
            let mut index = 0usize;

            while index < data.len() {
// Java: `ByteArrayHelper.getInt(data, index)`, throws on out-of-bounds;
// the Rust side discards the remaining data (keeping the items already parsed).
                if index + 4 > data.len() {
                    break;
                }
                let size = ByteArrayHelper::get_int(&data, index);
                index += 4;

                if index + 4 > data.len() {
                    break;
                }
                let item_type = ByteArrayHelper::get_int(&data, index);
                index += 4;

// Java: when size is negative `MPPUtility.cloneSubArray` throws NegativeArraySizeException,
// and throws IndexOutOfBoundsException when the data is too short; the Rust side stops parsing.
                let Ok(size) = usize::try_from(size) else {
                    break;
                };
                if index + size > data.len() {
                    break;
                }

                map.insert(item_type, MppUtility::clone_sub_array(&data, index, size));
                index += size;
            }
        }

        Self {
            m_data: var_data,
            m_map: map,
        }
    }

/// Get a string from the extended data.
    ///
    /// Corresponds to Java: `public String getString(Integer type)`
    pub fn get_string(&self, item_type: i32) -> Option<String> {
        let item = self.m_map.get(&item_type)?;
        let offset = Self::get_offset(item)?;
        self.m_data.get_string(offset)
    }

/// Get a Unicode string from the extended data.
    ///
    /// Corresponds to Java: `public String getUnicodeString(Integer type)`
    pub fn get_unicode_string(&self, item_type: i32) -> Option<String> {
        let item = self.m_map.get(&item_type)?;
        let offset = Self::get_offset(item)?;
        self.m_data.get_unicode_string(offset)
    }

/// Get a short integer value from the extended data.
    ///
    /// Corresponds to Java: `public int getShort(Integer type)`
    pub fn get_short(&self, item_type: i32) -> i32 {
        match self.m_map.get(&item_type) {
// Java throws when the item is shorter than 2 bytes; the Rust side falls back to 0
            Some(item) if item.len() >= 2 => ByteArrayHelper::get_short(item, 0),
            _ => 0,
        }
    }

/// Get an integer value from the extended data.
    ///
    /// Corresponds to Java: `public int getInt(Integer type)`
    pub fn get_int(&self, item_type: i32) -> i32 {
        match self.m_map.get(&item_type) {
// Java throws when the item is shorter than 4 bytes; the Rust side falls back to 0
            Some(item) if item.len() >= 4 => ByteArrayHelper::get_int(item, 0),
            _ => 0,
        }
    }

/// Get a long integer value from the extended data (six bytes).
    ///
    /// Corresponds to Java: `public long getLong(Integer type)`
    pub fn get_long(&self, item_type: i32) -> i64 {
        match self.m_map.get(&item_type) {
// Java throws when the item is shorter than 6 bytes; the Rust side falls back to 0
            Some(item) if item.len() >= 6 => MppUtility::get_long6(item, 0),
            _ => 0,
        }
    }

/// Get a double-precision floating-point value from the extended data.
    ///
    /// Corresponds to Java: `public double getDouble(Integer type)`
    pub fn get_double(&self, item_type: i32) -> f64 {
        match self.m_map.get(&item_type) {
// Java throws when the item is shorter than 8 bytes; the Rust side falls back to 0
            Some(item) if item.len() >= 8 => MppUtility::get_double(item, 0),
            _ => 0.0,
        }
    }

/// Get a timestamp from the extended data.
    ///
    /// Corresponds to Java: `public LocalDateTime getTimestamp(Integer type)`
    pub fn get_timestamp(&self, item_type: i32) -> Option<NaiveDateTime> {
        match self.m_map.get(&item_type) {
// Java throws when the item is shorter than 4 bytes; the Rust side returns None
            Some(item) if item.len() >= 4 => MppUtility::get_timestamp(item, 0),
            _ => None,
        }
    }

/// Get the raw byte array.
    ///
    /// Corresponds to Java: `public byte[] getByteArray(Integer type)`
    pub fn get_byte_array(&self, item_type: i32) -> Option<&[u8]> {
        self.m_map.get(&item_type).map(Vec::as_slice)
    }

/// Convert an integer value into an offset.
    ///
    /// Corresponds to Java: `private int getOffset(byte[] data)`
    ///
/// On the Java side, when the item is shorter than 4 bytes `ByteArrayHelper.getInt` throws,
/// and the Rust side returns `None` (the caller then falls back to "no such item").
    fn get_offset(data: &[u8]) -> Option<i32> {
        if data.len() < 4 {
            return None;
        }

        Some(-1 - ByteArrayHelper::get_int(data, 0))
    }
}

/// Debug output: print the formatted contents of the extended data block.
///
/// Corresponds to Java: `@Override public String toString()`
///
/// Java iterates with `HashMap` (order unspecified); Rust's `HashMap` likewise does not guarantee order.
impl Display for ExtendedData<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "BEGIN ExtendedData")?;

        for (item_type, data) in &self.m_map {
            writeln!(
                formatter,
                "Type: {} Data:{}",
                item_type,
                ByteArrayHelper::hexdump_all(data, false)
            )?;
        }

        writeln!(formatter, "END ExtendedData")?;
        writeln!(formatter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

/// Write a FixDeferFix data block at `offset` in `data` (`nextBlockOffset = -1`,
/// i.e. the data follows immediately), and return the number of bytes written.
    fn write_block(data: &mut [u8], offset: usize, payload: &[u8]) {
        data[offset..offset + 4].copy_from_slice(&(-1i32).to_le_bytes());
        data[offset + 4..offset + 8].copy_from_slice(&(payload.len() as i32).to_le_bytes());
        data[offset + 8..offset + 8 + payload.len()].copy_from_slice(payload);
    }

/// Build the byte contents of the "extended data block" itself: a sequence of `[length:4][type:4][data]`.
    fn extended_block(items: &[(i32, Vec<u8>)]) -> Vec<u8> {
        let mut result = Vec::new();

        for (item_type, payload) in items {
            result.extend_from_slice(&(payload.len() as i32).to_le_bytes());
            result.extend_from_slice(&item_type.to_le_bytes());
            result.extend_from_slice(payload);
        }

        result
    }

/// Build var data: place the extended data block itself at offset 8 (**single-block** case; the payload must not exceed the first block's capacity of 28 bytes).
    fn var_data(items: &[(i32, Vec<u8>)]) -> Vec<u8> {
        let block = extended_block(items);
        assert!(
            block.len() <= 28,
            "payload exceeds first block capacity of 28 bytes，use var_data_chained instead"
        );

        let mut data = vec![0u8; 8 + 8 + block.len()];
        write_block(&mut data, 8, &block);
        data
    }

/// Build var data spanning **two blocks**.
    ///
/// `FixDeferFix` uses a chained block layout: at the offset comes the "next block offset" first (`-1` marks the end of the chain), immediately followed by
/// "the length of this data item", and only then the data itself; the first block carries 28 bytes, and each subsequent block carries 32 bytes.
    fn var_data_chained(items: &[(i32, Vec<u8>)]) -> Vec<u8> {
        let block = extended_block(items);
        assert!(
            block.len() > 28 && block.len() - 28 <= 32,
            "this helper only covers the case of「first block 28 bytes + second block no more than 32 bytes」"
        );

// Offset of the second block header: 8 (start of block 1's header) + 8 (header length) + 28 (data carried by block 1)
        let block2_offset: i32 = 44;

        let mut data = vec![0u8; 8];
// Block 1: next block offset + total length of this data item + first 28 bytes of data
        data.extend_from_slice(&block2_offset.to_le_bytes());
        data.extend_from_slice(&(block.len() as i32).to_le_bytes());
        data.extend_from_slice(&block[..28]);
// Block 2: end-of-chain marker + remaining data
        data.extend_from_slice(&(-1i32).to_le_bytes());
        data.extend_from_slice(&block[28..]);

        data
    }

    #[test]
    fn reads_numeric_items() {
        let data = var_data_chained(&[
            (100, vec![0x34, 0x12, 0x00, 0x00]),
            (200, vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F]),
            (300, vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06]),
        ]);
        let fix_defer_fix = FixDeferFix::new(&data);
        let extended = ExtendedData::new(&fix_defer_fix, 8);

        assert_eq!(extended.get_short(100), 0x1234);
        assert_eq!(extended.get_int(100), 0x1234);
        assert_eq!(
            extended.get_byte_array(100),
            Some([0x34u8, 0x12, 0x00, 0x00].as_slice())
        );
        assert_eq!(extended.get_double(200), 1.0);
// Six-byte little-endian long integer
        assert_eq!(extended.get_long(300), 0x0605_0403_0201);
    }

    #[test]
    fn reads_timestamp_items() {
// The 2 bytes at offset 0 are the time (tenths of a minute); the 2 bytes at offset 2 are "epoch + days"
        let data = var_data(&[(100, vec![0x00, 0x00, 0x02, 0x00])]);
        let fix_defer_fix = FixDeferFix::new(&data);
        let extended = ExtendedData::new(&fix_defer_fix, 8);

        assert_eq!(
            extended.get_timestamp(100),
            Some(
                NaiveDate::from_ymd_opt(1984, 1, 2)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
            )
        );
    }

    #[test]
    fn missing_types_return_defaults() {
        let data = var_data(&[(100, vec![0x01, 0x00, 0x00, 0x00])]);
        let fix_defer_fix = FixDeferFix::new(&data);
        let extended = ExtendedData::new(&fix_defer_fix, 8);

        assert_eq!(extended.get_short(999), 0);
        assert_eq!(extended.get_int(999), 0);
        assert_eq!(extended.get_long(999), 0);
        assert_eq!(extended.get_double(999), 0.0);
        assert_eq!(extended.get_timestamp(999), None);
        assert_eq!(extended.get_string(999), None);
        assert_eq!(extended.get_unicode_string(999), None);
        assert_eq!(extended.get_byte_array(999), None);
    }

    #[test]
    fn short_items_do_not_panic() {
// Only 2 bytes: short is readable; int / long / double / timestamp cannot be read
        let data = var_data(&[(100, vec![0x34, 0x12])]);
        let fix_defer_fix = FixDeferFix::new(&data);
        let extended = ExtendedData::new(&fix_defer_fix, 8);

        assert_eq!(extended.get_short(100), 0x1234);
        assert_eq!(extended.get_int(100), 0);
        assert_eq!(extended.get_long(100), 0);
        assert_eq!(extended.get_double(100), 0.0);
        assert_eq!(extended.get_timestamp(100), None);
// Cannot interpret the offset when there are fewer than 4 bytes
        assert_eq!(extended.get_string(100), None);
    }

    #[test]
    fn string_items_are_located_through_negative_offsets() {
// In the extended data block, a string item stores "-1 - offset"; this points to offset 100 in var data
        let items = vec![
            (10, (-1i32 - 100).to_le_bytes().to_vec()),
            (11, (-1i32 - 150).to_le_bytes().to_vec()),
        ];
        let block = extended_block(&items);

        let mut data = vec![0u8; 200];
        write_block(&mut data, 8, &block);
// Place the single-byte string "AB" at offset 100
        write_block(&mut data, 100, &[0x41, 0x42]);
// Place the UTF-16LE string "CD" at offset 150
        write_block(&mut data, 150, &[0x43, 0x00, 0x44, 0x00]);

        let fix_defer_fix = FixDeferFix::new(&data);
        let extended = ExtendedData::new(&fix_defer_fix, 8);

        assert_eq!(extended.get_string(10).as_deref(), Some("AB"));
        assert_eq!(extended.get_unicode_string(11).as_deref(), Some("CD"));
    }

    #[test]
    fn absent_var_data_yields_an_empty_map() {
// getByteArray(offset) returns None (offset must be greater than 0)
        let data = vec![0u8; 32];
        let fix_defer_fix = FixDeferFix::new(&data);
        let extended = ExtendedData::new(&fix_defer_fix, 0);

        assert_eq!(extended.get_byte_array(100), None);
        assert_eq!(extended.get_int(100), 0);
    }

    #[test]
    fn display_lists_every_item() {
        let data = var_data(&[(100, vec![0x34, 0x12, 0x00, 0x00])]);
        let fix_defer_fix = FixDeferFix::new(&data);
        let extended = ExtendedData::new(&fix_defer_fix, 8);

        let text = extended.to_string();

        assert!(text.starts_with("BEGIN ExtendedData\n"));
        assert!(text.contains("Type: 100 Data:34 12 00 00"));
        assert!(text.ends_with("END ExtendedData\n\n"));
    }
}
