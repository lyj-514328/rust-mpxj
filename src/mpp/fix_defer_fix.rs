//! Corresponds to Java: `org/mpxj/mpp/FixDeferFix.java`
//!
//! Represents a variable-length data item in the Microsoft Project 98 file format. The data is stored as a "block chain":
//! each block header is a 4-byte "next block offset", with `-1` marking the end of the chain; `getByteArray` follows the chain to
//! reassemble all fragments of a data item into one contiguous array.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `MPPUtility.getByteArray` throws on out-of-bounds | `None` / abort dump | validate bounds first, then call the same utility class |
//!
//! # Skipped Java members + reason
//!
//! None.
//!
//! Porting trade-offs:
//! - The Java constructor takes an `InputStream`; by convention the Rust side changes this to `&[u8]` and stores it as `Vec<u8>`
//! (Java's `InputStreamHelper.readAvailable` already copies the data, so the semantics match).
//! - `MPPUtility.getByteArray` must validate the range first on the Rust side, so that Java's out-of-bounds exception
//! degrades gracefully to `None`; once the range check passes, reuse [`crate::mpp::mpp_utility::MPPUtility`].
//! - Where Java throws an array out-of-bounds exception (`ByteArrayHelper.getInt` / `MPPUtility.getByteArray`), the
//! Rust side returns `None` and never panics.
//! - Java's `new String(byte[])` uses the platform default charset; the Rust side decodes leniently as UTF-8;
//! `MPPUtility.getUnicodeString` decodes as UTF-16LE (via `encoding_rs`).
//! - Java's `TreeSet<Integer>` corresponds to `BTreeSet<i32>`; `toString()` corresponds to [`Display`],
//! where Java throws on out-of-bounds, while the Rust side aborts the dump immediately.
//! - Does not involve reflection, JDBC, XML/JAXB, GUI, `javax.*`, or `Serializable`.

use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::mpp_utility::MPPUtility;

/// `FixDeferFix` block: a variable-length data block in the MPP98 format.
///
/// Corresponds to Java: `final class FixDeferFix extends MPPComponent`
pub struct FixDeferFix {
/// The entire data block.
    ///
    /// Corresponds to Java: `private final byte[] m_data`
    data: Vec<u8>,
}

/// The result of a single iteration in [`FixDeferFix`]'s `to_string` traversal.
enum DumpOutcome {
/// Corresponds to Java's `continue`: skip the current offset.
    Skip,
/// A data block successfully parsed.
    Emit(Vec<u8>),
/// Corresponds to the exception Java throws on out-of-bounds etc.: abort the dump.
    Abort,
}

impl FixDeferFix {
/// Constructor. Reads the entire data block.
    ///
    /// Corresponds to Java: `FixDeferFix(InputStream is)`
    pub fn new(data: &[u8]) -> Self {
        // Java: m_data = InputStreamHelper.readAvailable(is)
        Self {
            data: data.to_vec(),
        }
    }

/// Get the data item starting at `offset` (may span multiple blocks).
    ///
    /// Corresponds to Java: `public byte[] getByteArray(int offset)`
    pub fn get_byte_array(&self, offset: i32) -> Option<Vec<u8>> {
        let data = &self.data;

        // Java: offset > 0 && offset < m_data.length
        if offset <= 0 || offset >= data.len() as i32 {
            return None;
        }

        let mut offset = offset;

// Java: ByteArrayHelper.getInt(m_data, offset), throws an array out-of-bounds exception; the Rust side returns None
        let mut next_block_offset = read_int(data, offset)?;
        offset += 4;

        let item_size = read_int(data, offset)?;
        offset += 4;

        // Java: itemSize > 0 && itemSize < m_data.length
        if item_size <= 0 || item_size >= data.len() as i32 {
            return None;
        }

// Data length each block other than the first can carry
        let mut block_remaining_size = 28i32;

        if next_block_offset != -1 || item_size <= block_remaining_size {
            let mut item_remaining_size = item_size;
            let mut result = vec![0u8; item_size as usize];
            let mut result_offset = 0i32;

            while next_block_offset != -1 {
                // Java: MPPUtility.getByteArray(m_data, offset, blockRemainingSize, result, resultOffset)
                copy_bytes(data, offset, block_remaining_size, &mut result, result_offset)?;
                result_offset += block_remaining_size;
                offset += block_remaining_size;
                item_remaining_size -= block_remaining_size;

                if offset != next_block_offset {
                    offset = next_block_offset;
                }

                next_block_offset = read_int(data, offset)?;
                offset += 4;
                block_remaining_size = 32;
            }

            // Java: MPPUtility.getByteArray(m_data, offset, itemRemainingSize, result, resultOffset)
            copy_bytes(data, offset, item_remaining_size, &mut result, result_offset)?;

            Some(result)
        } else {
            None
        }
    }

/// Get the string at the specified offset.
    ///
    /// Corresponds to Java: `public String getString(int offset)`
    pub fn get_string(&self, offset: i32) -> Option<String> {
        let data = self.get_byte_array(offset)?;
// Java: new String(data) uses the platform default charset; the Rust side decodes leniently as UTF-8
        Some(String::from_utf8_lossy(&data).into_owned())
    }

/// Get the Unicode (UTF-16LE) string at the specified offset.
    ///
    /// Corresponds to Java: `public String getUnicodeString(int offset)`
    pub fn get_unicode_string(&self, offset: i32) -> Option<String> {
        let data = self.get_byte_array(offset)?;
        Some(MPPUtility::get_unicode_string(&data, 0))
    }

/// Walk the block chain starting at `file_offset` and return the contents of the data block at that offset.
    ///
/// Corresponds to Java: the loop body of the while loop in `FixDeferFix#toString()`; where Java throws,
/// return [`DumpOutcome::Abort`] (the Rust side does not panic); Java's `continue` corresponds to
/// [`DumpOutcome::Skip`], and successfully parsed data corresponds to [`DumpOutcome::Emit`].
    fn dump_block(
        &self,
        file_offset: &mut i32,
        skipped: &mut BTreeSet<i32>,
        read: &mut BTreeSet<i32>,
    ) -> DumpOutcome {
        let data = &self.data;
        let mut file_offset_value = *file_offset;

        let mut next_block_offset = match read_int(data, file_offset_value) {
            Some(value) => value,
            None => return DumpOutcome::Abort,
        };
        file_offset_value += 4;

        let item_size = match read_int(data, file_offset_value) {
            Some(value) => value,
            None => return DumpOutcome::Abort,
        };
        file_offset_value += 4;

        let mut block_remaining_size = 28i32;

        if next_block_offset == -1 && item_size > block_remaining_size {
            file_offset_value += block_remaining_size;
            *file_offset = file_offset_value;
            return DumpOutcome::Skip;
        }

// Java: buffer = new byte[itemSize], throws NegativeArraySizeException when itemSize is negative
        if item_size < 0 {
            *file_offset = file_offset_value;
            return DumpOutcome::Abort;
        }

        let mut item_remaining_size = item_size;
        let mut buffer = vec![0u8; item_size as usize];
        let mut buffer_offset = 0i32;

        while next_block_offset != -1 {
// Java: MPPUtility.getByteArray(...), throws on out-of-bounds
            if copy_bytes(
                data,
                file_offset_value,
                block_remaining_size,
                &mut buffer,
                buffer_offset,
            )
            .is_none()
            {
                *file_offset = file_offset_value;
                return DumpOutcome::Abort;
            }

            buffer_offset += block_remaining_size;
            file_offset_value += block_remaining_size;
            item_remaining_size -= block_remaining_size;

            if file_offset_value != next_block_offset {
                skipped.insert(file_offset_value);
                file_offset_value = next_block_offset;
            }

            if !read.insert(file_offset_value) {
// Java: after fileOffset = available the loop continues, and then getByteArray throws out-of-bounds
                *file_offset = file_offset_value;
                return DumpOutcome::Abort;
            }

            next_block_offset = match read_int(data, file_offset_value) {
                Some(value) => value,
                None => {
                    *file_offset = file_offset_value;
                    return DumpOutcome::Abort;
                }
            };
            file_offset_value += 4;
            block_remaining_size = 32;
        }

        // Java: MPPUtility.getByteArray(m_data, fileOffset, itemRemainingSize, buffer, bufferOffset)
        if copy_bytes(
            data,
            file_offset_value,
            item_remaining_size,
            &mut buffer,
            buffer_offset,
        )
        .is_none()
        {
            *file_offset = file_offset_value;
            return DumpOutcome::Abort;
        }

        file_offset_value += item_remaining_size;

        if item_remaining_size < block_remaining_size {
            let skip = block_remaining_size - item_remaining_size;
            file_offset_value += skip;
        }

        *file_offset = file_offset_value;
        DumpOutcome::Emit(buffer)
    }
}

/// Debug view of the dump block (corresponds to Java's `toString()`, for debugging only).
impl Display for FixDeferFix {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "BEGIN FixDeferFix")?;

        // Java: int available = m_data.length
        let available = self.data.len() as i32;

// Java: skip the 4-byte header
        let mut file_offset = 4i32;
        let mut skipped: BTreeSet<i32> = BTreeSet::new();
        let mut read: BTreeSet<i32> = BTreeSet::new();

        while file_offset < available || !skipped.is_empty() {
            if file_offset >= available {
                // Java: skipped.first() + skipped.remove(temp)
                match skipped.pop_first() {
                    Some(value) => file_offset = value,
                    None => break,
                }
            }

            if !read.insert(file_offset) {
                file_offset = available;
                continue;
            }

            let start_offset = file_offset;

            match self.dump_block(&mut file_offset, &mut skipped, &mut read) {
                DumpOutcome::Skip => continue,
                DumpOutcome::Emit(buffer) => {
                    writeln!(
                        f,
                        "   Data: offset: {start_offset} size: {}",
                        buffer.len()
                    )?;
                    writeln!(f, "  {}", ByteArrayHelper::hexdump_all(&buffer, true))?;
                }
                DumpOutcome::Abort => break,
            }
        }

        writeln!(f, "END FixDeferFix")?;
        writeln!(f)
    }
}

/// Read a little-endian int from `data[offset..offset+4]`.
///
/// Corresponds to Java: `ByteArrayHelper.getInt(byte[], int)` (Java throws on out-of-bounds
/// `ArrayIndexOutOfBoundsException`; the Rust side returns `None`).
fn read_int(data: &[u8], offset: i32) -> Option<i32> {
    if offset < 0 {
        return None;
    }

    let start = offset as usize;
    let end = start.checked_add(4)?;
    data.get(start..end).map(|bytes| ByteArrayHelper::get_int(bytes, 0))
}

/// Copy `data[offset..offset+size]` to `dest[dest_offset..]`.
///
/// Corresponds to Java: `MPPUtility.getByteArray(byte[] data, int offset, int size, byte[] buffer, int bufferOffset)`
/// (Java checks bounds via `System.arraycopy` and throws; the Rust side returns `None`).
fn copy_bytes(
    data: &[u8],
    offset: i32,
    size: i32,
    dest: &mut [u8],
    dest_offset: i32,
) -> Option<()> {
    if offset < 0 || size < 0 || dest_offset < 0 {
        return None;
    }

    let source_start = offset as usize;
    let source_end = source_start.checked_add(size as usize)?;
    let destination_start = dest_offset as usize;
    let destination_end = destination_start.checked_add(size as usize)?;

    data.get(source_start..source_end)?;
    dest.get(destination_start..destination_end)?;
    MPPUtility::get_byte_array(data, source_start, size as usize, dest, destination_start);

    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

/// Write a little-endian int.
    fn write_int(data: &mut [u8], offset: usize, value: i32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

/// Single-block data: nextBlockOffset = -1, itemSize = 28, followed by 28 bytes of data.
    fn build_single_block_stream() -> Vec<u8> {
        let mut data = vec![0u8; 40];
        write_int(&mut data, 4, -1);
        write_int(&mut data, 8, 28);

        for index in 0..28 {
            data[12 + index] = index as u8;
        }

        data
    }

/// Single block: the whole data item is in the first block.
    #[test]
    fn reads_single_block() {
        let data = build_single_block_stream();
        let fix_defer = FixDeferFix::new(&data);

        let result = fix_defer.get_byte_array(4).expect("data found");
        assert_eq!(result.len(), 28);
        assert_eq!(result.as_slice(), &data[12..40]);
    }

/// Cross-block: the first block has 28 bytes, chains to the second block to take 12 more bytes, forming 40 bytes.
    #[test]
    fn reads_linked_blocks() {
        let mut data = vec![0u8; 56];
        write_int(&mut data, 4, 40);
        write_int(&mut data, 8, 40);

        for index in 0..28 {
            data[12 + index] = 0xAA;
        }
        write_int(&mut data, 40, -1);
        for index in 0..12 {
            data[44 + index] = 0xBB;
        }

        let fix_defer = FixDeferFix::new(&data);
        let result = fix_defer.get_byte_array(4).expect("data found");

        assert_eq!(result.len(), 40);
        assert_eq!(&result[0..28], &[0xAA_u8; 28]);
        assert_eq!(&result[28..40], &[0xBB_u8; 12]);
    }

/// Return None when the offset is invalid or the data block is incomplete.
    #[test]
    fn invalid_offsets_return_none() {
        let data = build_single_block_stream();
        let fix_defer = FixDeferFix::new(&data);

        // Java: offset > 0
        assert!(fix_defer.get_byte_array(0).is_none());
        assert!(fix_defer.get_byte_array(-1).is_none());
        // Java: offset < m_data.length
        assert!(fix_defer.get_byte_array(40).is_none());
        assert!(fix_defer.get_byte_array(1000).is_none());
        assert!(fix_defer.get_string(0).is_none());

// itemSize is not less than the data block length => None
        let mut clipped = vec![0u8; 40];
        write_int(&mut clipped, 4, -1);
        write_int(&mut clipped, 8, 40);
        assert!(FixDeferFix::new(&clipped).get_byte_array(4).is_none());
    }

/// Return None for an empty data block.
    #[test]
    fn empty_block_returns_none() {
        let fix_defer = FixDeferFix::new(&[]);

        assert!(fix_defer.get_byte_array(4).is_none());
        assert!(fix_defer.get_string(4).is_none());
        assert!(fix_defer.get_unicode_string(4).is_none());
    }

/// Single-byte string.
    #[test]
    fn reads_ascii_string() {
        let mut data = vec![0u8; 17];
        write_int(&mut data, 4, -1);
        write_int(&mut data, 8, 5);
        data[12..17].copy_from_slice(b"hello");

        let fix_defer = FixDeferFix::new(&data);
        assert_eq!(fix_defer.get_string(4), Some("hello".to_string()));
    }

/// UTF-16LE string: terminated by a two-byte 0.
    #[test]
    fn reads_unicode_string() {
        let mut data = vec![0u8; 40];
        write_int(&mut data, 4, -1);
        write_int(&mut data, 8, 28);
        data[12..16].copy_from_slice(&[0x48, 0x00, 0x69, 0x00]); // "Hi"

        let fix_defer = FixDeferFix::new(&data);
        assert_eq!(fix_defer.get_unicode_string(4), Some("Hi".to_string()));
    }

/// Display outputs the block header and a hex dump of each block's data.
    #[test]
    fn display_dumps_block() {
        let mut data = vec![0u8; 17];
        write_int(&mut data, 4, -1);
        write_int(&mut data, 8, 5);
        data[12..17].copy_from_slice(b"hello");

        let text = FixDeferFix::new(&data).to_string();
        assert!(text.starts_with("BEGIN FixDeferFix\n"));
        assert!(text.contains("   Data: offset: 4 size: 5\n"));
        assert!(text.ends_with("END FixDeferFix\n\n"));
    }

/// In Display, a block with nextBlockOffset = -1 and itemSize > 28 is skipped entirely.
    #[test]
    fn display_skips_oversized_tail_block() {
        let mut data = vec![0u8; 60];
// First block: next = -1 and itemSize = 100 > 28 => skipped
        write_int(&mut data, 4, -1);
        write_int(&mut data, 8, 100);
// Second block: next = -1, itemSize = 4, data "ABCD"
        write_int(&mut data, 40, -1);
        write_int(&mut data, 44, 4);
        data[48..52].copy_from_slice(b"ABCD");

        let text = FixDeferFix::new(&data).to_string();
        assert!(text.contains("   Data: offset: 40 size: 4\n"));
        assert!(!text.contains("   Data: offset: 4 size:"));
    }
}
