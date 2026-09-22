//! Corresponds to Java: `org/mpxj/mpp/FixedMeta.java`
//!
//! Represents the "FixedMeta" stream in an MPP file: the block header describes the number of records in the associated "FixedData" block,
//! followed by several fixed-length records, each describing the position (offset, etc.) of the corresponding record in the `FixedData` block.
//! The exact structure of the records within the block is not yet fully understood, so this only holds the block as a whole and slices it by offset.
//!
//! Porting trade-offs:
//! - Java's constructor receives an `InputStream`; by convention the Rust side takes `&[u8]`; Java's
//! `is.available()` corresponds to `data.len()` (POI guarantees this value is accurate).
//! - Java inherits `MPPComponent` to reuse `readInt` / `readByteArray`; the Rust side uses this file's private
//! functions [`read_int`] / [`read_byte_array`] (insufficient data corresponds to Java's `EOFException`,
//! returning `Err`).
//! - Java's `Object[]` only ever holds `byte[]`, so the Rust side uses `Vec<Option<Vec<u8>>>`.
//! - Java's `toString()` corresponds to [`Display`].
//! - The places where Java throws (bad magic, division by zero caused by `itemSize` being 0, negative array length, array out of bounds)
//! are changed on the Rust side to return `Err` or to yield 0 / `None` respectively, never panicking.
//! - Does not involve reflection, JDBC, XML/JAXB, GUI, `javax.*`, `Serializable`.

use std::fmt::{Display, Formatter};
use std::io;

use crate::common::byte_array_helper::ByteArrayHelper;

use super::fixed_data::FixedData;
use super::fixed_meta_item_size_provider::FixedMetaItemSizeProvider;

/// `FixedMeta` block: block header + several fixed-length records.
///
/// Corresponds to Java: `final class FixedMeta extends MPPComponent`
pub struct FixedMeta {
/// The record count reported by the block header.
    ///
    /// Corresponds to Java: `private final int m_itemCount`
    item_count: i32,
/// The record count derived from the block size and the record size.
    ///
    /// Corresponds to Java: `private final int m_adjustedItemCount`
    adjusted_item_count: i32,
/// The data item for each record (on the Java side an `Object[]`, where each element is either `byte[]` or `null`).
    ///
    /// Corresponds to Java: `private final Object[] m_array`
    array: Vec<Option<Vec<u8>>>,
}

/// Block header magic number.
///
/// Corresponds to Java: `private static final int MAGIC = 0xFADFADBA` (Java's `int` is signed,
/// so the literal itself is negative; the same bit pattern is used here).
const MAGIC: i32 = 0xFADFADBA_u32 as i32;

/// Block header length.
///
/// Corresponds to Java: `private static final int HEADER_SIZE = 16`
const HEADER_SIZE: i32 = 16;

impl FixedMeta {
/// Constructor. The block size is known, so the record count in the block header is ignored and the count is derived directly from the given record size.
    ///
    /// Corresponds to Java: `FixedMeta(InputStream is, final int itemSize)`
    pub fn with_item_size(data: &[u8], item_size: i32) -> io::Result<Self> {
        let provider = move |_file_size: i32, _item_count: i32| item_size;
        Self::new(data, &provider)
    }

/// Constructor. The record size is determined by the given item size provider.
    ///
    /// Corresponds to Java: `FixedMeta(InputStream is, FixedMetaItemSizeProvider itemSizeProvider)`
    pub fn new(data: &[u8], item_size_provider: &dyn FixedMetaItemSizeProvider) -> io::Result<Self> {
// Java: int fileSize = is.available(); POI guarantees this value is accurate
        let file_size = data.len() as i32;

        //
// Block header: 4 ints (the 1st is magic, the 3rd is the record count, the rest are unknown)
        //
        let mut offset = 0usize;
        let magic = read_int(data, &mut offset)?;
        if magic != MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Bad magic number: {magic}"),
            ));
        }

        read_int(data, &mut offset)?;
        let item_count = read_int(data, &mut offset)?;
        read_int(data, &mut offset)?;

        let item_size = item_size_provider.get_item_size(file_size, item_count);

// Java: (fileSize - HEADER_SIZE) / itemSize; throws when itemSize is 0
// ArithmeticException; a negative result makes the array construction throw NegativeArraySizeException;
// the Rust side uniformly takes 0 (Java's int division truncates toward zero, so a negative numerator also yields 0).
        let adjusted_item_count = if item_size > 0 {
            (file_size - HEADER_SIZE) / item_size
        } else {
            0
        };

        let mut array = Vec::with_capacity(adjusted_item_count.max(0) as usize);
        for _ in 0..adjusted_item_count {
            array.push(Some(read_byte_array(data, &mut offset, item_size)?));
        }

        Ok(Self {
            item_count,
            adjusted_item_count,
            array,
        })
    }

/// Constructor. Given several candidate record sizes, makes a heuristic choice together with the record count of another fixed-length block.
    ///
    /// Corresponds to Java: `FixedMeta(InputStream is, final FixedData otherFixedBlock, final int... itemSizes)`
    pub fn with_item_sizes(
        data: &[u8],
        other_fixed_block: &FixedData,
        item_sizes: &[i32],
    ) -> io::Result<Self> {
        let provider = move |file_size: i32, item_count: i32| {
// Java: itemSizes[0], which throws ArrayIndexOutOfBoundsException when the array is empty; the Rust side takes 0
            let mut item_size = item_sizes.first().copied().unwrap_or(0);
            let available = file_size - HEADER_SIZE;
            // Java: Integer.MIN_VALUE
            let mut distance = i32::MIN;
            let other_fixed_block_count = other_fixed_block.get_item_count();

            for &test_item_size in item_sizes {
// Java throws ArithmeticException when taking a modulo by 0; the Rust side skips that candidate
                if test_item_size == 0 || available % test_item_size != 0 {
                    continue;
                }

                //
// If a candidate size divides the block size exactly and matches the record count of the other block, it is essentially certain
// that this is the correct record size, so the selection ends immediately.
                //
                if available / test_item_size == other_fixed_block_count {
                    item_size = test_item_size;
                    break;
                }

                //
// Otherwise use an empirical rule to choose the closest candidate (Java's int overflow wraps,
// while the Rust side uses wrapping to preserve the same semantics without panicking in debug).
                //
                let test_distance = item_count.wrapping_mul(test_item_size).wrapping_sub(available);
                if test_distance <= 0 && test_distance > distance {
                    item_size = test_item_size;
                    distance = test_distance;
                }
            }

            item_size
        };

        Self::new(data, &provider)
    }

/// Get the record count reported by the block header.
    ///
    /// Corresponds to Java: `public int getItemCount()`
    pub fn get_item_count(&self) -> i32 {
        self.item_count
    }

/// Get the record count derived from the block size and the record size.
    ///
    /// Corresponds to Java: `public int getAdjustedItemCount()`
    pub fn get_adjusted_item_count(&self) -> i32 {
        self.adjusted_item_count
    }

/// Get the data item at the given index; returns `None` when the index is out of bounds or there is no data at that position.
    ///
    /// Corresponds to Java: `public byte[] getByteArrayValue(int index)`
    pub fn get_byte_array_value(&self, index: i32) -> Option<&[u8]> {
        // Java: index >= 0 && index < m_array.length && m_array[index] != null
        if index < 0 {
            return None;
        }

        self.array.get(index as usize)?.as_deref()
    }
}

/// Debug view that dumps the block (corresponds to Java's `toString()`, for debugging only).
impl Display for FixedMeta {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "BEGIN: FixedMeta")?;
        writeln!(f, "   Adjusted Item count: {}", self.adjusted_item_count)?;

        for loop_index in 0..self.adjusted_item_count {
            writeln!(f, "   Data at index: {loop_index}")?;
// Java: ByteArrayHelper.hexdump((byte[]) m_array[loop], true); outputs an empty string when the element is null
            let item = self
                .array
                .get(loop_index as usize)
                .and_then(|value| value.as_deref())
                .unwrap_or(&[]);
            writeln!(f, "  {}", ByteArrayHelper::hexdump_all(item, true))?;
        }

        writeln!(f, "END: FixedMeta")?;
        writeln!(f)
    }
}

/// Read a little-endian int from `data[pos..]` and advance `pos`.
///
/// Corresponds to Java: `MPPComponent#readInt(InputStream)` (Java throws `EOFException` when there is insufficient data,
/// the Rust side returns `ErrorKind::UnexpectedEof`).
fn read_int(data: &[u8], pos: &mut usize) -> io::Result<i32> {
    let end = pos
        .checked_add(4)
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "FixedMeta stream is truncated"))?;
    let bytes = data.get(*pos..end).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "FixedMeta stream is truncated",
        )
    })?;
    *pos = end;
    Ok(ByteArrayHelper::get_int(bytes, 0))
}

/// Read `size` bytes from `data[pos..]` and advance `pos`.
///
/// Corresponds to Java: `MPPComponent#readByteArray(InputStream, int)` (Java throws
/// `RuntimeException` when there is insufficient data and `NegativeArraySizeException` when `size` is negative; the Rust side returns `Err`).
fn read_byte_array(data: &[u8], pos: &mut usize, size: i32) -> io::Result<Vec<u8>> {
    if size < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("negative item size: {size}"),
        ));
    }

    let end = pos
        .checked_add(size as usize)
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "FixedMeta stream is truncated"))?;
    let bytes = data.get(*pos..end).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "FixedMeta stream is truncated",
        )
    })?;
    *pos = end;
    Ok(bytes.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

/// Hand-builds a "FixedMeta" byte stream: a 16-byte block header + `count` `item_size`-byte records.
    ///
/// Each record: the first 4 bytes are the marker `index`, bytes 4..8 are the offset into the `FixedData`
/// data body that the record points to, and the remaining bytes are filled with `0xEE`.
    fn build_meta_stream(
        item_size: usize,
        count: usize,
        reported_count: i32,
        item_offsets: &[i32],
    ) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFADFADBAu32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&reported_count.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());

        for index in 0..count {
            let mut item = vec![0xEE_u8; item_size];
            if item_size >= 4 {
                item[0..4].copy_from_slice(&(index as i32).to_le_bytes());
            }
            if item_size >= 8 {
                let item_offset = item_offsets.get(index).copied().unwrap_or(0);
                item[4..8].copy_from_slice(&item_offset.to_le_bytes());
            }
            data.extend_from_slice(&item);
        }

        data
    }

/// The count reported by the block header and the count derived from the block size are stored in two separate getters.
    #[test]
    fn reads_header_and_adjusted_item_count() {
// 3 records of 8 bytes => 16 + 24 = 40 bytes; the block header falsely reports 99 records
        let data = build_meta_stream(8, 3, 99, &[0, 8, 16]);
        assert_eq!(data.len(), 40);

        let meta = FixedMeta::with_item_size(&data, 8).expect("meta parsed");

        assert_eq!(meta.get_item_count(), 99);
        assert_eq!(meta.get_adjusted_item_count(), 3);
    }

/// Get the record contents by index (including the offset field within the record).
    #[test]
    fn reads_item_values() {
        let data = build_meta_stream(8, 3, 3, &[0, 8, 16]);
        let meta = FixedMeta::with_item_size(&data, 8).expect("meta parsed");

        let item = meta.get_byte_array_value(1).expect("item 1");
        assert_eq!(item.len(), 8);
        assert_eq!(ByteArrayHelper::get_int(item, 0), 1);
        assert_eq!(ByteArrayHelper::get_int(item, 4), 8);

        let last = meta.get_byte_array_value(2).expect("item 2");
        assert_eq!(ByteArrayHelper::get_int(last, 4), 16);
    }

/// An out-of-bounds index returns None and never panics.
    #[test]
    fn out_of_range_index_returns_none() {
        let data = build_meta_stream(8, 2, 2, &[0, 8]);
        let meta = FixedMeta::with_item_size(&data, 8).expect("meta parsed");

        assert!(meta.get_byte_array_value(2).is_none());
        assert!(meta.get_byte_array_value(1000).is_none());
        assert!(meta.get_byte_array_value(-1).is_none());
        assert!(meta.get_byte_array_value(i32::MIN).is_none());
    }

/// Bad magic returns Err (Java throws IOException).
    #[test]
    fn bad_magic_returns_error() {
        let mut data = build_meta_stream(8, 1, 1, &[0]);
        data[0] = 0x00;

        assert!(FixedMeta::with_item_size(&data, 8).is_err());
    }

/// An empty data stream (not even a block header) returns Err (Java throws EOFException).
    #[test]
    fn empty_stream_returns_error() {
        assert!(FixedMeta::with_item_size(&[], 8).is_err());
        assert!(FixedMeta::with_item_size(&[0u8; 8], 8).is_err());
    }

/// The record size comes from the provider, and the provider receives the block size and the count reported by the block header.
    #[test]
    fn uses_item_size_provider() {
        let data = build_meta_stream(8, 3, 7, &[0, 8, 16]);
        let expected_file_size = data.len() as i32;

        let provider = move |file_size: i32, item_count: i32| {
            assert_eq!(file_size, expected_file_size);
            assert_eq!(item_count, 7);
            8
        };

        let meta = FixedMeta::new(&data, &provider).expect("meta parsed");
        assert_eq!(meta.get_adjusted_item_count(), 3);
    }

/// The provider returning 0 / a negative number does not panic and yields an empty block (Java would throw).
    #[test]
    fn zero_item_size_yields_empty_block() {
        let data = build_meta_stream(8, 3, 3, &[0, 8, 16]);
        let meta = FixedMeta::with_item_size(&data, 0).expect("meta parsed");

        assert_eq!(meta.get_adjusted_item_count(), 0);
        assert!(meta.get_byte_array_value(0).is_none());
    }

/// Trailing leftover bytes are ignored when the block size is not an integer multiple of the record size.
    #[test]
    fn ignores_partial_trailing_item() {
// 16 + 3*8 + 4 = 44 bytes, 44-16 = 28 => 28 / 8 = 3 records
        let mut data = build_meta_stream(8, 3, 3, &[0, 8, 16]);
        data.extend_from_slice(&[0xEE; 4]);

        let meta = FixedMeta::with_item_size(&data, 8).expect("meta parsed");
        assert_eq!(meta.get_adjusted_item_count(), 3);
    }

/// Selects a candidate record size directly when it matches the other block's count (Java's first break branch).
    #[test]
    fn chooses_item_size_matching_other_block_count() {
// FixedData body: 3 records of 8 bytes
        let fixed_data = FixedData::with_fixed_size(8, &[0xAA; 24]);
        assert_eq!(fixed_data.get_item_count(), 3);

// available = 24 => both candidates 6 and 8 divide it evenly, but only 8 yields 3 records
        let data = build_meta_stream(8, 3, 3, &[0, 8, 16]);
        let meta = FixedMeta::with_item_sizes(&data, &fixed_data, &[6, 8]).expect("meta parsed");

        assert_eq!(meta.get_adjusted_item_count(), 3);
    }

/// Falls back to the first candidate when no candidate satisfies the conditions (the empirical rule branch is not satisfied).
    #[test]
    fn falls_back_to_first_candidate_when_none_fit() {
        let fixed_data = FixedData::with_fixed_size(8, &[0xAA; 24]);

// available = 24: candidate 10 does not divide evenly; candidate 12 divides evenly but its count does not match,
// and (itemCount * 12 - available) > 0 makes the empirical rule fail as well => keep itemSizes[0] = 10
        let data = build_meta_stream(12, 2, 3, &[0, 12]);
        let meta = FixedMeta::with_item_sizes(&data, &fixed_data, &[10, 12]).expect("meta parsed");

        assert_eq!(meta.get_item_count(), 3);
        // (40 - 16) / 10 = 2
        assert_eq!(meta.get_adjusted_item_count(), 2);
        assert_eq!(meta.get_byte_array_value(0).map(<[u8]>::len), Some(10));
    }

/// Display outputs the block header information and a hex dump of each record.
    #[test]
    fn display_dumps_block() {
        let data = build_meta_stream(4, 2, 2, &[0, 4]);
        let meta = FixedMeta::with_item_size(&data, 4).expect("meta parsed");

        let text = meta.to_string();
        assert!(text.starts_with("BEGIN: FixedMeta\n"));
        assert!(text.contains("   Adjusted Item count: 2\n"));
        assert!(text.contains("   Data at index: 0\n"));
        assert!(text.contains("   Data at index: 1\n"));
        assert!(text.ends_with("END: FixedMeta\n\n"));
    }
}
