//! Corresponds to Java: `org/mpxj/common/ByteArrayHelper.java`

/// Utility methods for handling byte arrays.
///
/// Corresponds to Java: `public final class ByteArrayHelper`
///
/// Porting notes:
/// - The Java `byte[]` parameter is `&[u8]` on the Rust side; the Java branch that returns an
///   empty string for `buffer == null` is naturally covered by an empty slice on the Rust side
///   (the loop does not run and the result is likewise an empty string).
/// - Out-of-bounds access matches Java throwing `ArrayIndexOutOfBoundsException` (Rust panics).
/// - Java's `DecimalFormat("00000")` is implemented equivalently with `format!("{:05}", ...)`.
/// - In the Java implementation of `hexdump(byte[], int, int, int...)` `blockIndex` is never
///   incremented, so as long as `blockStarts` is non-empty and its first element is greater than
///   0 it loops forever (that method has no callers in the Java source either). The Rust side
///   increments `block_index` as intended so that `blockStarts` take effect in turn, keeping the
///   rest of the logic identical.
///
/// Skipped Java methods: none.
pub struct ByteArrayHelper;

/// Lookup table converting a byte to a hexadecimal character.
///
/// Corresponds to Java: `private static final char[] HEX_DIGITS`
const HEX_DIGITS: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
];

impl ByteArrayHelper {
    /// Corresponds to Java: `public static final int getShort(byte[] data, int offset)`
    ///
    /// Reads two bytes little-endian starting at `offset`; the result is a positive integer in 0..65535.
    pub fn get_short(data: &[u8], offset: usize) -> i32 {
        let mut result = 0i32;
        let mut index = offset;

        let mut shift_by = 0;
        while shift_by < 16 {
            result |= i32::from(data[index]) << shift_by;
            index += 1;
            shift_by += 8;
        }

        result
    }

    /// Corresponds to Java: `public static final int getInt(byte[] data, int offset)`
    ///
    /// Reads four bytes little-endian starting at `offset`; the highest byte forms the sign bit (can be negative, as in Java).
    pub fn get_int(data: &[u8], offset: usize) -> i32 {
        let mut result = 0i32;
        let mut index = offset;

        let mut shift_by = 0;
        while shift_by < 32 {
            result |= i32::from(data[index]) << shift_by;
            index += 1;
            shift_by += 8;
        }

        result
    }

    /// Corresponds to Java: `public static final long getLong(byte[] data, int offset)`
    ///
    /// Reads eight bytes little-endian starting at `offset`.
    pub fn get_long(data: &[u8], offset: usize) -> i64 {
        let mut result = 0i64;
        let mut index = offset;

        let mut shift_by = 0;
        while shift_by < 64 {
            result |= i64::from(data[index]) << shift_by;
            index += 1;
            shift_by += 8;
        }

        result
    }

    /// Corresponds to Java: `public static final String hexdump(byte[] buffer, int offset, int length, boolean ascii)`
    ///
    /// Outputs the hexadecimal representation of `length` bytes starting at `offset`; when `ascii` is true, appends an ASCII character view.
    pub fn hexdump(buffer: &[u8], offset: usize, length: usize, ascii: bool) -> String {
        let mut result = String::new();
        let count = (offset + length).min(buffer.len());

        for index in offset..count {
            if !result.is_empty() {
                result.push(' ');
            }
            result.push(HEX_DIGITS[usize::from((buffer[index] & 0xF0) >> 4)]);
            result.push(HEX_DIGITS[usize::from(buffer[index] & 0x0F)]);
        }

        if ascii {
            result.push_str("   ");

            for index in offset..count {
                // Java: char c = (char) buffer[loop]; a negative byte sign-extends to 0xFFxx (always > 200)
                let raw = i32::from(buffer[index] as i8);
                let value = if raw < 0 { raw + 0x10000 } else { raw };
                result.push(if value > 200 || value < 27 {
                    ' '
                } else {
                    // Values 27..=200 always fall within u8 range, so the conversion is lossless
                    value as u8 as char
                });
            }
        }

        result
    }

    /// Corresponds to Java: `public static final String hexdump(byte[] buffer, boolean ascii)`
    ///
    /// Outputs the entire array.
    pub fn hexdump_all(buffer: &[u8], ascii: bool) -> String {
        Self::hexdump(buffer, 0, buffer.len(), ascii)
    }

    /// Corresponds to Java: `public static final String hexdump(byte[] buffer, boolean ascii, int columns, String prefix)`
    ///
    /// Outputs in fixed-width columns; each line is prefixed with `prefix` plus a five-digit decimal offset.
    pub fn hexdump_columns(buffer: &[u8], ascii: bool, columns: usize, prefix: &str) -> String {
        let mut result = String::new();
        let mut index = 0;
        let mut columns = columns;

        while index < buffer.len() {
            if index + columns > buffer.len() {
                columns = buffer.len() - index;
            }

            result.push_str(prefix);
            result.push_str(&format!("{index:05}"));
            result.push_str(": ");
            result.push_str(&Self::hexdump(buffer, index, columns, ascii));
            result.push('\n');

            index += columns;
        }

        result
    }

    /// Corresponds to Java: `public static final String hexdump(byte[] buffer, int offset, int length, boolean ascii, int columns, String prefix)`
    ///
    /// Outputs the given range in fixed-width columns; the five-digit offset on each line is relative to `offset`.
    pub fn hexdump_columns_at(
        buffer: &[u8],
        offset: usize,
        length: usize,
        ascii: bool,
        columns: usize,
        prefix: &str,
    ) -> String {
        let mut result = String::new();
        let mut index = offset;
        let mut columns = columns;

        while index < offset + length {
            if index + columns > offset + length {
                columns = (offset + length) - index;
            }

            result.push_str(prefix);
            result.push_str(&format!("{:05}", index - offset));
            result.push_str(": ");
            result.push_str(&Self::hexdump(buffer, index, columns, ascii));
            result.push('\n');

            index += columns;
        }

        result
    }

    /// Corresponds to Java: `public static final String hexdump(byte[] buffer, int offset, int length, int... blockStarts)`
    ///
    /// Splits the data into blocks at the end positions given by `blockStarts`, wraps each block in `[...]`, and concatenates them.
    pub fn hexdump_blocks(
        buffer: &[u8],
        offset: usize,
        length: usize,
        block_starts: &[usize],
    ) -> String {
        let mut result = String::new();

        let mut buffer_index = 0;
        let mut block_index = 0;

        while buffer_index < length {
            let block_end = if block_index == block_starts.len() {
                length
            } else {
                block_starts[block_index]
            };
            let block_length = block_end.saturating_sub(buffer_index);
            result.push('[');
            result.push_str(&Self::hexdump(buffer, offset + buffer_index, block_length, false));
            result.push(']');
            buffer_index += block_length;
            block_index += 1;
        }

        result
    }
}
