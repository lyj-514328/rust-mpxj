//! Corresponds to Java: `org/mpxj/mpp/FixedData.java`
//!
//! Represents the "FixedData" stream in an MPP file. "Fixed Data" means that each record in the block has a known
//! maximum length, rather than all records having exactly the same length: each record's position in the data body is given
//! by the associated [`FixedMeta`] (bytes 4..8 of the record are the offset), and the length is determined by the offset difference between adjacent records.
//!
//! Porting trade-offs:
//! - Java's constructor takes an `InputStream`; per convention the Rust side changes it to `&[u8]`; Java uses
//! `InputStreamHelper.readAvailable(is)` to read the entire block at once, corresponding to the Rust side using the whole slice directly.
//! - Java's `Object[]` only ever stores `byte[]`, and entries that are never assigned are `null`, so the Rust side uses
//! `Vec<Option<Vec<u8>>>` (a fixed length equal to the count given by `FixedMeta`, consistent with the Java array).
//! - Java's `toString()` corresponds to [`Display`].
//! - The few places where Java throws (`meta` item being `null` causing an NPE, array out of bounds in `ByteArrayHelper.getInt` /
//! `System.arraycopy`, negative array length) are changed on the Rust side to skip that record or return `None`,
//! never panicking.
//! - Java throws `IOException` when reading from the `InputStream` fails; on the Rust side the input is a slice, so a read out of bounds can only
//! occur in a situation that should not happen, in which case that record takes `None`.
//! - Does not involve reflection, JDBC, XML/JAXB, GUI, `javax.*`, or `Serializable`.

use std::fmt::{Display, Formatter};

use crate::common::byte_array_helper::ByteArrayHelper;

use super::fixed_meta::FixedMeta;

/// The `FixedData` block: a fixed-length data body split according to the description in `FixedMeta`.
///
/// Corresponds to Java: `final class FixedData extends MPPComponent`
pub struct FixedData {
/// All data items in the block (on the Java side an `Object[]`, with entries without data being `null`).
    ///
    /// Corresponds to Java: `private final Object[] m_array`
    array: Vec<Option<Vec<u8>>>,
/// The starting offset of each data item in the data body (entries without data stay 0, consistent with Java).
    ///
    /// Corresponds to Java: `private final int[] m_offset`
    offset: Vec<i32>,
}

impl FixedData {
/// Constructor. Splits the data body using the already-read metadata.
    ///
    /// Corresponds to Java: `FixedData(FixedMeta meta, InputStream is)`
    pub fn new(meta: &FixedMeta, data: &[u8]) -> Self {
        Self::with_max_expected_size(meta, data, 0)
    }

/// Constructor. Limits the maximum length of a single record to avoid copying too much useless data.
    ///
    /// Corresponds to Java: `FixedData(FixedMeta meta, InputStream is, int maxExpectedSize)`
    pub fn with_max_expected_size(meta: &FixedMeta, data: &[u8], max_expected_size: i32) -> Self {
        Self::with_limits(meta, data, max_expected_size, 0)
    }

/// Constructor. Limits the maximum length of a single record and falls back to the minimum length when the length given by the metadata is 0.
    ///
    /// Corresponds to Java: `FixedData(FixedMeta meta, InputStream is, int maxExpectedSize, int minSize)`
    pub fn with_limits(
        meta: &FixedMeta,
        data: &[u8],
        max_expected_size: i32,
        min_size: i32,
    ) -> Self {
        // Java: byte[] buffer = InputStreamHelper.readAvailable(is);
        let buffer = data;
        let buffer_length = buffer.len() as i32;

        let item_count = meta.get_adjusted_item_count();
// Java constructs the array directly from itemCount; a negative value throws NegativeArraySizeException; the Rust side uses 0
        let item_count = item_count.max(0) as usize;
        let mut array = vec![None; item_count];
        let mut offset = vec![0i32; item_count];

        for loop_index in 0..item_count {
// Java: meta.getByteArrayValue(loop); when null the subsequent getInt throws an NPE; the Rust side skips that record
            let Some(meta_data) = meta.get_byte_array_value(loop_index as i32) else {
                continue;
            };

// Java: ByteArrayHelper.getInt(metaData, 4); when metaData is shorter than 8 bytes the array is out of bounds; the Rust side skips
            if meta_data.len() < 8 {
                continue;
            }
            let item_offset = ByteArrayHelper::get_int(meta_data, 4);

            if item_offset < 0 || item_offset > buffer_length {
                continue;
            }

            let mut item_size = if loop_index + 1 == item_count {
                buffer_length - item_offset
            } else {
                let Some(next_meta_data) = meta.get_byte_array_value(loop_index as i32 + 1) else {
                    continue;
                };
                if next_meta_data.len() < 8 {
                    continue;
                }
                let next_item_offset = ByteArrayHelper::get_int(next_meta_data, 4);
// Java's int overflow wraps around; the Rust side uses wrapping to preserve the semantics without panicking
                next_item_offset.wrapping_sub(item_offset)
            };

            if item_size == 0 {
                item_size = min_size;
            }

            let available = buffer_length - item_offset;

            if item_size < 0 || item_size > available {
                item_size = if max_expected_size == 0 {
                    available
                } else {
                    max_expected_size.min(available)
                };
            }

            if max_expected_size != 0 && item_size > max_expected_size {
                item_size = max_expected_size;
            }

            if item_size > 0 {
                array[loop_index] = clone_sub_array(buffer, item_offset, item_size);
                offset[loop_index] = item_offset;
            }
        }

        Self { array, offset }
    }

/// Constructor. Ignores the length given by the metadata and uses the record length provided by the caller instead.
    ///
    /// Corresponds to Java: `FixedData(FixedMeta meta, int itemSize, InputStream is)`
    pub fn with_item_size(meta: &FixedMeta, item_size: i32, data: &[u8]) -> Self {
        // Java: byte[] buffer = InputStreamHelper.readAvailable(is);
        let buffer = data;
        let buffer_length = buffer.len() as i32;

        let item_count = meta.get_adjusted_item_count();
// Java constructs the array directly from itemCount; a negative value throws NegativeArraySizeException; the Rust side uses 0
        let item_count = item_count.max(0) as usize;
        let mut array = vec![None; item_count];
        let mut offset = vec![0i32; item_count];

// Java reuses (and overwrites) the parameter itemSize in the loop; the Rust side stays consistent
        let mut item_size = item_size;

        for loop_index in 0..item_count {
            let Some(meta_data) = meta.get_byte_array_value(loop_index as i32) else {
                continue;
            };
            if meta_data.len() < 8 {
                continue;
            }
            let item_offset = ByteArrayHelper::get_int(meta_data, 4);

            if item_offset < 0 || item_offset > buffer_length {
                continue;
            }

            let available = buffer_length - item_offset;

            if item_size < 0 {
                item_size = available;
            } else if item_size > available {
                item_size = available;
            }

            array[loop_index] = clone_sub_array(buffer, item_offset, item_size);
            offset[loop_index] = item_offset;
        }

        Self { array, offset }
    }

/// Constructor. The record length is fixed and known, so no metadata is needed.
    ///
    /// Corresponds to Java: `FixedData(int itemSize, InputStream is)`
    pub fn with_fixed_size(item_size: i32, data: &[u8]) -> Self {
        Self::with_fixed_size_and_remainder(item_size, data, false)
    }

/// Constructor. The record length is fixed and known, with an option to read the trailing remainder that is shorter than one record.
    ///
    /// Corresponds to Java: `FixedData(int itemSize, InputStream is, boolean readRemainderBlock)`
    pub fn with_fixed_size_and_remainder(
        item_size: i32,
        data: &[u8],
        read_remainder_block: bool,
    ) -> Self {
// Java: is.available() / itemSize; when itemSize is 0 it throws ArithmeticException; the Rust side uses 0
        let mut item_count = if item_size > 0 {
            data.len() / item_size as usize
        } else {
            0
        };

// Java: when readRemainderBlock && is.available() % itemSize != 0, read one extra record
        if read_remainder_block && item_size > 0 && data.len() % item_size as usize != 0 {
            item_count += 1;
        }

        let mut array = Vec::with_capacity(item_count);
        let mut offset = Vec::with_capacity(item_count);

        let mut position = 0usize;
        let mut item_offset = 0i32;

        for _ in 0..item_count {
            // Java: int currentItemSize = is.available() < itemSize ? is.available() : itemSize
            let available = (data.len() - position) as i32;
            let current_item_size = if read_remainder_block && available < item_size {
                available
            } else {
                item_size
            };

            offset.push(item_offset);
// Java throws when it cannot read a full record; the Rust side reads up to the end of the slice (which does not happen normally)
            let read_size = current_item_size.max(0) as usize;
            array.push(
                data.get(position..)
                    .and_then(|remaining| remaining.get(..read_size))
                    .map(<[u8]>::to_vec),
            );

            position = position.saturating_add(read_size);
            item_offset = item_offset.wrapping_add(item_size);
        }

        Self { array, offset }
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

/// Get the number of data items in the block (without consulting the metadata).
    ///
    /// Corresponds to Java: `public int getItemCount()`
    pub fn get_item_count(&self) -> i32 {
        self.array.len() as i32
    }

/// Determine whether an offset value falls within the block.
    ///
    /// Corresponds to Java: `public boolean isValidOffset(Integer offset)`
    ///
/// Rust has no overloading; the `Integer` version corresponds to this method and the `int` version corresponds to [`FixedData::is_valid_offset`].
    pub fn is_valid_offset_optional(&self, offset: Option<i32>) -> bool {
        // Java: offset != null && isValidOffset(offset.intValue())
        match offset {
            Some(value) => self.is_valid_offset(value),
            None => false,
        }
    }

/// Determine whether an offset value falls within the block.
    ///
    /// Corresponds to Java: `public boolean isValidOffset(int offset)`
    pub fn is_valid_offset(&self, offset: i32) -> bool {
        offset >= 0 && offset < self.array.len() as i32
    }

/// Convert an offset value to a data item index; returns -1 when not found (consistent with Java).
    ///
    /// Corresponds to Java: `public int getIndexFromOffset(int offset)`
    pub fn get_index_from_offset(&self, offset: i32) -> i32 {
        for (index, value) in self.offset.iter().enumerate() {
            if *value == offset {
                return index as i32;
            }
        }

        -1
    }
}

/// Debug view dumping the block (corresponds to Java's `toString()`, for debugging only).
impl Display for FixedData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "BEGIN FixedData")?;

        for loop_index in 0..self.array.len() {
            writeln!(
                f,
                "   Data at index: {} offset: {}",
                loop_index,
                self.offset.get(loop_index).copied().unwrap_or(0)
            )?;
// Java: ByteArrayHelper.hexdump((byte[]) m_array[loop], true); when the element is null an empty string is output
            let item = self
                .array
                .get(loop_index)
                .and_then(|value| value.as_deref())
                .unwrap_or(&[]);
            writeln!(f, "  {}", ByteArrayHelper::hexdump_all(item, true))?;
        }

        writeln!(f, "END FixedData")?;
        writeln!(f)
    }
}

/// Copy `data[offset..offset+size]` into a new array.
///
/// Corresponds to Java: `MPPUtility.cloneSubArray(byte[] data, int offset, int size)` (when out of bounds Java
/// throws from `System.arraycopy`; the Rust side returns `None`).
fn clone_sub_array(data: &[u8], offset: i32, size: i32) -> Option<Vec<u8>> {
    if offset < 0 || size < 0 {
        return None;
    }

    let start = offset as usize;
    let end = start.checked_add(size as usize)?;
    data.get(start..end).map(<[u8]>::to_vec)
}

#[cfg(test)]
mod tests {
    use super::*;

/// Manually build a "FixedMeta" byte stream: a 16-byte block header + `offsets.len()` 8-byte records,
/// where bytes 4..8 of each record hold the offset of that data item in the "FixedData" data body.
    fn build_meta_stream(offsets: &[i32]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFADFADBAu32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(offsets.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());

        for offset in offsets {
            let mut item = vec![0xEE_u8; 8];
            item[4..8].copy_from_slice(&offset.to_le_bytes());
            data.extend_from_slice(&item);
        }

        data
    }

/// A data body of incrementing bytes, to make asserting slice contents easy.
    fn build_data_stream(length: usize) -> Vec<u8> {
        (0..length).map(|index| index as u8).collect()
    }

/// Split the data body according to the offsets given by the metadata and the differences between adjacent offsets.
    #[test]
    fn splits_data_body_by_offset() {
        let meta = FixedMeta::with_item_size(&build_meta_stream(&[0, 8, 16]), 8).expect("meta");
        let data = build_data_stream(24);

        let fixed = FixedData::new(&meta, &data);

        assert_eq!(fixed.get_item_count(), 3);
        assert_eq!(fixed.get_byte_array_value(0), Some(&data[0..8]));
        assert_eq!(fixed.get_byte_array_value(1), Some(&data[8..16]));
// The last record extends all the way to the end of the data body
        assert_eq!(fixed.get_byte_array_value(2), Some(&data[16..24]));
    }

/// Mutual lookup of offsets and indices.
    #[test]
    fn offset_lookup() {
        let meta = FixedMeta::with_item_size(&build_meta_stream(&[0, 8, 16]), 8).expect("meta");
        let fixed = FixedData::new(&meta, &build_data_stream(24));

        assert_eq!(fixed.get_index_from_offset(0), 0);
        assert_eq!(fixed.get_index_from_offset(8), 1);
        assert_eq!(fixed.get_index_from_offset(16), 2);
// Java: returns -1 when not found
        assert_eq!(fixed.get_index_from_offset(17), -1);

        assert!(fixed.is_valid_offset(0));
        assert!(fixed.is_valid_offset(2));
        assert!(!fixed.is_valid_offset(3));
        assert!(!fixed.is_valid_offset(-1));

        assert!(fixed.is_valid_offset_optional(Some(1)));
        assert!(!fixed.is_valid_offset_optional(None));
    }

/// Out-of-bounds indices / an empty data body do not panic.
    #[test]
    fn out_of_range_and_empty_data() {
        let meta = FixedMeta::with_item_size(&build_meta_stream(&[0, 8]), 8).expect("meta");

        let fixed = FixedData::new(&meta, &[]);
        assert_eq!(fixed.get_item_count(), 2);
        assert!(fixed.get_byte_array_value(0).is_none());
        assert!(fixed.get_byte_array_value(2).is_none());
        assert!(fixed.get_byte_array_value(-1).is_none());

        let fixed = FixedData::new(&meta, &build_data_stream(16));
        assert!(fixed.get_byte_array_value(2).is_none());
        assert!(fixed.get_byte_array_value(1000).is_none());
    }

/// Skip the record when the offset exceeds the length of the data body.
    #[test]
    fn skips_offset_beyond_data() {
        let meta =
            FixedMeta::with_item_size(&build_meta_stream(&[0, 8, 1000]), 8).expect("meta");
        let fixed = FixedData::new(&meta, &build_data_stream(16));

        assert_eq!(fixed.get_byte_array_value(0), Some(&build_data_stream(16)[0..8]));
        assert_eq!(fixed.get_byte_array_value(1), Some(&build_data_stream(16)[8..16]));
// The third offset is out of bounds; the Java side continues, the Rust side keeps None
        assert!(fixed.get_byte_array_value(2).is_none());
        assert_eq!(fixed.get_index_from_offset(1000), -1);
    }

/// Fall back to minSize when the metadata gives equal adjacent offsets (length 0).
    #[test]
    fn zero_length_item_uses_min_size() {
        let meta = FixedMeta::with_item_size(&build_meta_stream(&[0, 0]), 8).expect("meta");
        let data = build_data_stream(10);

        let fixed = FixedData::with_limits(&meta, &data, 0, 4);

        assert_eq!(fixed.get_byte_array_value(0), Some(&data[0..4]));
// The last record extends all the way to the end of the data body
        assert_eq!(fixed.get_byte_array_value(1), Some(&data[0..10]));
    }

/// maxExpectedSize limits the length of a single record.
    #[test]
    fn max_expected_size_limits_item_length() {
        let meta = FixedMeta::with_item_size(&build_meta_stream(&[0, 4]), 8).expect("meta");
        let data = build_data_stream(32);

        let fixed = FixedData::with_limits(&meta, &data, 6, 0);

        assert_eq!(fixed.get_byte_array_value(0), Some(&data[0..4]));
// The last record runs from 4 to the end, 28 bytes total, truncated by maxExpectedSize = 6
        assert_eq!(fixed.get_byte_array_value(1), Some(&data[4..10]));
    }

/// Split the data body using the record length given by the caller.
    #[test]
    fn uses_supplied_item_size() {
        let meta = FixedMeta::with_item_size(&build_meta_stream(&[0, 4, 8]), 8).expect("meta");
        let data = build_data_stream(18);

        let fixed = FixedData::with_item_size(&meta, 5, &data);

        assert_eq!(fixed.get_byte_array_value(0), Some(&data[0..5]));
        assert_eq!(fixed.get_byte_array_value(1), Some(&data[4..9]));
// The third record has at most 10 bytes available from offset 8; 5 bytes are still within range
        assert_eq!(fixed.get_byte_array_value(2), Some(&data[8..13]));
    }

/// Fixed-length data body: when itemSize is negative, read up to the end of the data body.
    #[test]
    fn fixed_size_constructors() {
        let data = build_data_stream(10);

        let fixed = FixedData::with_fixed_size(4, &data);
        assert_eq!(fixed.get_item_count(), 2);
        assert_eq!(fixed.get_byte_array_value(0), Some(&data[0..4]));
        assert_eq!(fixed.get_byte_array_value(1), Some(&data[4..8]));
        assert_eq!(fixed.get_index_from_offset(4), 1);

// Read the trailing remainder data
        let fixed = FixedData::with_fixed_size_and_remainder(4, &data, true);
        assert_eq!(fixed.get_item_count(), 3);
        assert_eq!(fixed.get_byte_array_value(2), Some(&data[8..10]));
        assert_eq!(fixed.get_index_from_offset(8), 2);

// Record size with length 0 (Java throws ArithmeticException)
        let fixed = FixedData::with_fixed_size(0, &data);
        assert_eq!(fixed.get_item_count(), 0);
        assert!(fixed.get_byte_array_value(0).is_none());
    }

/// The Display output shows each data item's index, offset, and hex dump.
    #[test]
    fn display_dumps_block() {
        let meta = FixedMeta::with_item_size(&build_meta_stream(&[0, 8]), 8).expect("meta");
        let fixed = FixedData::new(&meta, &build_data_stream(16));

        let text = fixed.to_string();
        assert!(text.starts_with("BEGIN FixedData\n"));
        assert!(text.contains("   Data at index: 0 offset: 0\n"));
        assert!(text.contains("   Data at index: 1 offset: 8\n"));
        assert!(text.ends_with("END FixedData\n\n"));
    }
}
