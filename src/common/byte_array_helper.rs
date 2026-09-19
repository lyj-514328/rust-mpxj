//! 对应 Java: `org/mpxj/common/ByteArrayHelper.java`

/// 处理字节数组的工具方法。
///
/// 对应 Java: `public final class ByteArrayHelper`
///
/// 移植说明：
/// - Java 的 `byte[]` 参数在 Rust 侧为 `&[u8]`；Java 针对 `buffer == null` 返回空串的分支，
///   在 Rust 侧由空切片天然覆盖（循环不执行，结果同样为空串）。
/// - 越界访问与 Java 抛 `ArrayIndexOutOfBoundsException` 的行为一致（Rust 侧 panic）。
/// - Java 的 `DecimalFormat("00000")` 用 `format!("{:05}", ...)` 等价实现。
/// - `hexdump(byte[], int, int, int...)` 的 Java 实现中 `blockIndex` 从不自增，只要 `blockStarts`
///   非空且首元素大于 0 就会死循环（该方法在 Java 源码中也没有调用方）。Rust 侧按意图补上
///   `block_index` 自增，使 `blockStarts` 依次生效，其余逻辑保持一致。
///
/// 未移植的方法：无。
pub struct ByteArrayHelper;

/// 字节到十六进制字符的转换表。
///
/// 对应 Java: `private static final char[] HEX_DIGITS`
const HEX_DIGITS: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
];

impl ByteArrayHelper {
    /// 对应 Java: `public static final int getShort(byte[] data, int offset)`
    ///
    /// 从 `offset` 开始按小端读取两个字节，结果为 0..65535 的正整数。
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

    /// 对应 Java: `public static final int getInt(byte[] data, int offset)`
    ///
    /// 从 `offset` 开始按小端读取四个字节，最高字节会构成符号位（与 Java 一致可为负）。
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

    /// 对应 Java: `public static final long getLong(byte[] data, int offset)`
    ///
    /// 从 `offset` 开始按小端读取八个字节。
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

    /// 对应 Java: `public static final String hexdump(byte[] buffer, int offset, int length, boolean ascii)`
    ///
    /// 输出 `offset` 起 `length` 个字节的十六进制表示；`ascii` 为真时追加 ASCII 字符视图。
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
                // Java: char c = (char) buffer[loop]，byte 为负时符号扩展成 0xFFxx（必然 > 200）
                let raw = i32::from(buffer[index] as i8);
                let value = if raw < 0 { raw + 0x10000 } else { raw };
                result.push(if value > 200 || value < 27 {
                    ' '
                } else {
                    // 取值 27..=200 一定落在 u8 范围内，转换无损
                    value as u8 as char
                });
            }
        }

        result
    }

    /// 对应 Java: `public static final String hexdump(byte[] buffer, boolean ascii)`
    ///
    /// 输出整个数组。
    pub fn hexdump_all(buffer: &[u8], ascii: bool) -> String {
        Self::hexdump(buffer, 0, buffer.len(), ascii)
    }

    /// 对应 Java: `public static final String hexdump(byte[] buffer, boolean ascii, int columns, String prefix)`
    ///
    /// 按固定宽度分列输出，每行前缀为 `prefix` 加五位十进制偏移量。
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

    /// 对应 Java: `public static final String hexdump(byte[] buffer, int offset, int length, boolean ascii, int columns, String prefix)`
    ///
    /// 按固定宽度分列输出指定区间，每行的五位偏移量相对于 `offset`。
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

    /// 对应 Java: `public static final String hexdump(byte[] buffer, int offset, int length, int... blockStarts)`
    ///
    /// 把数据按 `blockStarts` 指定的块结束位置切块，逐块用 `[...]` 包裹后拼接。
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
