//! Corresponds to Java: `org/mpxj/mpp/FixFix.java`
//!
//! Represents a block of fixed-length data items in the Microsoft Project 98 file format. Normally the block size is an integer
//! multiple of `itemSize`, but oversized instances have also been seen, so the remainder of the block size divided by
//! `itemSize` is recorded here ([`FixFix::get_diff`]), letting callers decide whether the data must be reinterpreted with a different record size.
//!
//! Porting trade-offs:
//! - The Java constructor accepts an `InputStream`; by convention the Rust side takes `&[u8]`; Java's
//! `is.available()` corresponds to the slice length.
//! - Java extends `MPPComponent` to reuse `readByteArray`; the Rust side splits the slice directly by record size.
//! - Java's `Object[]` only ever holds `byte[]`, so the Rust side uses `Vec<Vec<u8>>`
//! (every record is populated at construction time, so the `null` check in Java never holds in this class).
//! - Java's `toString()` corresponds to [`Display`].
//! - The two places where Java throws (division by zero when `itemSize` is 0, and the array out-of-bounds in `getByteArrayValue`)
//! respectively yield 0 / return `None` on the Rust side and never panic.
//! - Does not involve reflection, JDBC, XML/JAXB, GUI, `javax.*`, or `Serializable`.

use std::fmt::{Display, Formatter};

use crate::common::byte_array_helper::ByteArrayHelper;

/// `FixFix` block: a fixed-length data block in the MPP98 format.
///
/// Corresponds to Java: `final class FixFix extends MPPComponent`
pub struct FixFix {
/// All data items in the block.
    ///
    /// Corresponds to Java: `private final Object[] m_array`
    array: Vec<Vec<u8>>,
/// The overall size of the block.
    ///
    /// Corresponds to Java: `private final int m_size`
    size: i32,
/// The remainder of the block size divided by the record size.
    ///
    /// Corresponds to Java: `private final int m_diff`
    diff: i32,
}

impl FixFix {
/// Constructor. Splits fixed-length records from the data by `item_size` and records the remainder of the division.
    ///
    /// Corresponds to Java: `FixFix(int itemSize, InputStream is)`
    pub fn new(item_size: i32, data: &[u8]) -> Self {
        // Java: m_size = is.available()
        let size = data.len() as i32;

// Java: m_size % itemSize and m_size / itemSize, throws when itemSize is 0
// ArithmeticException; the Rust side uniformly takes 0
        let (diff, item_count) = if item_size > 0 {
            (size % item_size, size / item_size)
        } else {
            (0, 0)
        };

        let mut array = Vec::with_capacity(item_count.max(0) as usize);
        let mut position = 0usize;

        for _ in 0..item_count {
// Java: readByteArray(is, itemSize), throws when fewer than one record can be read;
// the Rust side reads up to the end of the slice (should not happen under normal conditions)
            let read_size = item_size as usize;
            if let Some(item) = data
                .get(position..)
                .and_then(|remaining| remaining.get(..read_size))
            {
                array.push(item.to_vec());
            }

            position = position.saturating_add(read_size);
        }

        Self { array, size, diff }
    }

/// Get the remainder of the block size divided by the record size; a non-zero value means the records in the block do not match the expected size.
    ///
    /// Corresponds to Java: `public int getDiff()`
    pub fn get_diff(&self) -> i32 {
        self.diff
    }

/// Get the overall size of the block.
    ///
    /// Corresponds to Java: `public int getSize()`
    pub fn get_size(&self) -> i32 {
        self.size
    }

/// Get the data item at the given index; returns `None` when the index is out of bounds.
    ///
    /// Corresponds to Java: `public byte[] getByteArrayValue(int index)`
    ///
/// The Java side performs no index check (out of bounds throws `ArrayIndexOutOfBoundsException`); the Rust side returns `None`.
    pub fn get_byte_array_value(&self, index: i32) -> Option<&[u8]> {
        if index < 0 {
            return None;
        }

        self.array.get(index as usize).map(Vec::as_slice)
    }

/// Get the number of records in the block.
    ///
    /// Corresponds to Java: `public int getItemCount()`
    pub fn get_item_count(&self) -> i32 {
        self.array.len() as i32
    }
}

/// Debug view that dumps the block (corresponds to Java's `toString()`, for debugging only).
impl Display for FixFix {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "BEGIN FixFix")?;

        for loop_index in 0..self.array.len() {
            writeln!(f, "   Data at index: {loop_index}")?;
            let item = self.array.get(loop_index).map(Vec::as_slice).unwrap_or(&[]);
            writeln!(f, "  {}", ByteArrayHelper::hexdump_all(item, true))?;
        }

        writeln!(f, "END FixFix")?;
        writeln!(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// Use data with incrementing bytes to make slice content easy to assert.
    fn build_data(length: usize) -> Vec<u8> {
        (0..length).map(|index| index as u8).collect()
    }

/// Normal case: the block size is an integer multiple of the record size.
    #[test]
    fn splits_full_blocks() {
        let data = build_data(12);
        let fix = FixFix::new(4, &data);

        assert_eq!(fix.get_size(), 12);
        assert_eq!(fix.get_diff(), 0);
        assert_eq!(fix.get_item_count(), 3);
        assert_eq!(fix.get_byte_array_value(0), Some(&data[0..4]));
        assert_eq!(fix.get_byte_array_value(1), Some(&data[4..8]));
        assert_eq!(fix.get_byte_array_value(2), Some(&data[8..12]));
    }

/// When the block size is oversized the remainder is non-zero and the leftover bytes do not become a record.
    #[test]
    fn reports_remainder() {
        let data = build_data(10);
        let fix = FixFix::new(4, &data);

        assert_eq!(fix.get_size(), 10);
        // 10 % 4 == 2
        assert_eq!(fix.get_diff(), 2);
        // 10 / 4 == 2
        assert_eq!(fix.get_item_count(), 2);
    }

/// An out-of-bounds index returns None (Java throws ArrayIndexOutOfBoundsException).
    #[test]
    fn out_of_range_index_returns_none() {
        let fix = FixFix::new(4, &build_data(8));

        assert!(fix.get_byte_array_value(2).is_none());
        assert!(fix.get_byte_array_value(-1).is_none());
    }

/// Empty data block and record size 0 (Java throws ArithmeticException).
    #[test]
    fn empty_block_and_zero_item_size() {
        let fix = FixFix::new(4, &[]);
        assert_eq!(fix.get_size(), 0);
        assert_eq!(fix.get_diff(), 0);
        assert_eq!(fix.get_item_count(), 0);
        assert!(fix.get_byte_array_value(0).is_none());

        let fix = FixFix::new(0, &build_data(10));
        assert_eq!(fix.get_size(), 10);
        assert_eq!(fix.get_diff(), 0);
        assert_eq!(fix.get_item_count(), 0);
    }

/// Display prints a hex dump of each record.
    #[test]
    fn display_dumps_block() {
        let fix = FixFix::new(2, &build_data(4));

        let text = fix.to_string();
        assert!(text.starts_with("BEGIN FixFix\n"));
        assert!(text.contains("   Data at index: 0\n"));
        assert!(text.contains("   Data at index: 1\n"));
        assert!(text.ends_with("END FixFix\n\n"));
    }
}
