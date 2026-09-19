//! 对应 Java: `org/mpxj/mpp/FixedData.java`
//!
//! 表示 MPP 文件中的 "FixedData" 流。所谓 "Fixed Data" 指的是块内每条记录都有已知的
//! 最大长度，而不是所有记录长度完全一致：每条记录在数据体中的位置由关联的
//! [`FixedMeta`] 给出（记录的第 4..8 字节是偏移），长度则由相邻记录的偏移差决定。
//!
//! 移植取舍：
//! - Java 的构造方法接收 `InputStream`，Rust 侧按约定改为 `&[u8]`；Java 用
//!   `InputStreamHelper.readAvailable(is)` 一次性读完整个块，对应 Rust 侧直接使用整个切片。
//! - Java 的 `Object[]` 只会存放 `byte[]`，没有被赋值的条目为 `null`，因此 Rust 侧为
//!   `Vec<Option<Vec<u8>>>`（长度固定为 `FixedMeta` 给出的条数，与 Java 数组一致）。
//! - Java 的 `toString()` 对应 [`Display`]。
//! - Java 会抛异常的几处（`meta` 数据项为 `null` 导致的 NPE、`ByteArrayHelper.getInt` /
//!   `System.arraycopy` 的数组越界、负数组长度）在 Rust 侧改为跳过该条记录或返回 `None`，
//!   绝不 panic。
//! - Java 从 `InputStream` 读取失败时抛 `IOException`；Rust 侧输入是切片，读取越界只可能
//!   出现在本不该发生的情况下，此时该条记录取 `None`。
//! - 未涉及反射、JDBC、XML/JAXB、GUI、`javax.*`、`Serializable`。

use std::fmt::{Display, Formatter};

use crate::common::byte_array_helper::ByteArrayHelper;

use super::fixed_meta::FixedMeta;

/// `FixedData` 块：按 `FixedMeta` 描述切分出来的定长数据体。
///
/// 对应 Java: `final class FixedData extends MPPComponent`
pub struct FixedData {
    /// 块内所有数据项（Java 侧为 `Object[]`，未取到数据的条目为 `null`）。
    ///
    /// 对应 Java: `private final Object[] m_array`
    array: Vec<Option<Vec<u8>>>,
    /// 每条数据项在数据体中的起始偏移（未取到数据的条目保持 0，与 Java 一致）。
    ///
    /// 对应 Java: `private final int[] m_offset`
    offset: Vec<i32>,
}

impl FixedData {
    /// 构造方法。使用已经读入的元数据切分数据体。
    ///
    /// 对应 Java: `FixedData(FixedMeta meta, InputStream is)`
    pub fn new(meta: &FixedMeta, data: &[u8]) -> Self {
        Self::with_max_expected_size(meta, data, 0)
    }

    /// 构造方法。限制单条记录的最大长度，避免复制过多无用数据。
    ///
    /// 对应 Java: `FixedData(FixedMeta meta, InputStream is, int maxExpectedSize)`
    pub fn with_max_expected_size(meta: &FixedMeta, data: &[u8], max_expected_size: i32) -> Self {
        Self::with_limits(meta, data, max_expected_size, 0)
    }

    /// 构造方法。限制单条记录的最大长度，并在元数据给出的长度为 0 时改用最小长度。
    ///
    /// 对应 Java: `FixedData(FixedMeta meta, InputStream is, int maxExpectedSize, int minSize)`
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
        // Java 用 itemCount 直接构造数组，负数会抛 NegativeArraySizeException；Rust 侧取 0
        let item_count = item_count.max(0) as usize;
        let mut array = vec![None; item_count];
        let mut offset = vec![0i32; item_count];

        for loop_index in 0..item_count {
            // Java: meta.getByteArrayValue(loop)，为 null 时后续 getInt 抛 NPE；Rust 侧跳过该条
            let Some(meta_data) = meta.get_byte_array_value(loop_index as i32) else {
                continue;
            };

            // Java: ByteArrayHelper.getInt(metaData, 4)，metaData 不足 8 字节时数组越界；Rust 侧跳过
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
                // Java 的 int 溢出会回绕，Rust 侧用 wrapping 保持语义且不 panic
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

    /// 构造方法。忽略元数据给出的长度，改用调用方提供的记录长度。
    ///
    /// 对应 Java: `FixedData(FixedMeta meta, int itemSize, InputStream is)`
    pub fn with_item_size(meta: &FixedMeta, item_size: i32, data: &[u8]) -> Self {
        // Java: byte[] buffer = InputStreamHelper.readAvailable(is);
        let buffer = data;
        let buffer_length = buffer.len() as i32;

        let item_count = meta.get_adjusted_item_count();
        // Java 用 itemCount 直接构造数组，负数会抛 NegativeArraySizeException；Rust 侧取 0
        let item_count = item_count.max(0) as usize;
        let mut array = vec![None; item_count];
        let mut offset = vec![0i32; item_count];

        // Java 在循环中复用（并覆盖）入参 itemSize，Rust 侧保持一致
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

    /// 构造方法。记录长度固定且已知，无需元数据。
    ///
    /// 对应 Java: `FixedData(int itemSize, InputStream is)`
    pub fn with_fixed_size(item_size: i32, data: &[u8]) -> Self {
        Self::with_fixed_size_and_remainder(item_size, data, false)
    }

    /// 构造方法。记录长度固定且已知，可选择是否读取末尾不足一条的残余数据。
    ///
    /// 对应 Java: `FixedData(int itemSize, InputStream is, boolean readRemainderBlock)`
    pub fn with_fixed_size_and_remainder(
        item_size: i32,
        data: &[u8],
        read_remainder_block: bool,
    ) -> Self {
        // Java: is.available() / itemSize，itemSize 为 0 时抛 ArithmeticException；Rust 侧取 0
        let mut item_count = if item_size > 0 {
            data.len() / item_size as usize
        } else {
            0
        };

        // Java: readRemainderBlock && is.available() % itemSize != 0 时多读一条
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
            // Java 读不足一条时抛异常；Rust 侧取到切片末尾为止（正常情况下不会发生）
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

    /// 取指定下标的数据项；下标越界或该位置没有数据时返回 `None`。
    ///
    /// 对应 Java: `public byte[] getByteArrayValue(int index)`
    pub fn get_byte_array_value(&self, index: i32) -> Option<&[u8]> {
        // Java: index >= 0 && index < m_array.length && m_array[index] != null
        if index < 0 {
            return None;
        }

        self.array.get(index as usize)?.as_deref()
    }

    /// 取块内数据项条数（不参考元数据）。
    ///
    /// 对应 Java: `public int getItemCount()`
    pub fn get_item_count(&self) -> i32 {
        self.array.len() as i32
    }

    /// 判断偏移值是否落在块内。
    ///
    /// 对应 Java: `public boolean isValidOffset(Integer offset)`
    ///
    /// Rust 无重载，`Integer` 版本对应本方法，`int` 版本对应 [`FixedData::is_valid_offset`]。
    pub fn is_valid_offset_optional(&self, offset: Option<i32>) -> bool {
        // Java: offset != null && isValidOffset(offset.intValue())
        match offset {
            Some(value) => self.is_valid_offset(value),
            None => false,
        }
    }

    /// 判断偏移值是否落在块内。
    ///
    /// 对应 Java: `public boolean isValidOffset(int offset)`
    pub fn is_valid_offset(&self, offset: i32) -> bool {
        offset >= 0 && offset < self.array.len() as i32
    }

    /// 把偏移值转换为数据项下标；找不到时返回 -1（与 Java 一致）。
    ///
    /// 对应 Java: `public int getIndexFromOffset(int offset)`
    pub fn get_index_from_offset(&self, offset: i32) -> i32 {
        for (index, value) in self.offset.iter().enumerate() {
            if *value == offset {
                return index as i32;
            }
        }

        -1
    }
}

/// 转储块的调试视图（对应 Java 的 `toString()`，仅供调试使用）。
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
            // Java: ByteArrayHelper.hexdump((byte[]) m_array[loop], true)，元素为 null 时输出空串
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

/// 复制 `data[offset..offset+size]` 到新数组。
///
/// 对应 Java: `MPPUtility.cloneSubArray(byte[] data, int offset, int size)`（Java 越界时
/// 由 `System.arraycopy` 抛异常，Rust 侧返回 `None`）。
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

    /// 手工构造 "FixedMeta" 字节流：16 字节块头 + `offsets.len()` 条 8 字节记录，
    /// 每条记录的第 4..8 字节存放该条数据在 "FixedData" 数据体中的偏移。
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

    /// 用递增字节的数据体，便于断言切片内容。
    fn build_data_stream(length: usize) -> Vec<u8> {
        (0..length).map(|index| index as u8).collect()
    }

    /// 按元数据给出的偏移与相邻偏移差切分数据体。
    #[test]
    fn splits_data_body_by_offset() {
        let meta = FixedMeta::with_item_size(&build_meta_stream(&[0, 8, 16]), 8).expect("meta");
        let data = build_data_stream(24);

        let fixed = FixedData::new(&meta, &data);

        assert_eq!(fixed.get_item_count(), 3);
        assert_eq!(fixed.get_byte_array_value(0), Some(&data[0..8]));
        assert_eq!(fixed.get_byte_array_value(1), Some(&data[8..16]));
        // 最后一条记录一直取到数据体末尾
        assert_eq!(fixed.get_byte_array_value(2), Some(&data[16..24]));
    }

    /// 偏移与下标互查。
    #[test]
    fn offset_lookup() {
        let meta = FixedMeta::with_item_size(&build_meta_stream(&[0, 8, 16]), 8).expect("meta");
        let fixed = FixedData::new(&meta, &build_data_stream(24));

        assert_eq!(fixed.get_index_from_offset(0), 0);
        assert_eq!(fixed.get_index_from_offset(8), 1);
        assert_eq!(fixed.get_index_from_offset(16), 2);
        // Java: 找不到时返回 -1
        assert_eq!(fixed.get_index_from_offset(17), -1);

        assert!(fixed.is_valid_offset(0));
        assert!(fixed.is_valid_offset(2));
        assert!(!fixed.is_valid_offset(3));
        assert!(!fixed.is_valid_offset(-1));

        assert!(fixed.is_valid_offset_optional(Some(1)));
        assert!(!fixed.is_valid_offset_optional(None));
    }

    /// 越界下标 / 空数据体不会 panic。
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

    /// 偏移超出数据体长度时跳过该条记录。
    #[test]
    fn skips_offset_beyond_data() {
        let meta =
            FixedMeta::with_item_size(&build_meta_stream(&[0, 8, 1000]), 8).expect("meta");
        let fixed = FixedData::new(&meta, &build_data_stream(16));

        assert_eq!(fixed.get_byte_array_value(0), Some(&build_data_stream(16)[0..8]));
        assert_eq!(fixed.get_byte_array_value(1), Some(&build_data_stream(16)[8..16]));
        // 第三条偏移越界，Java 侧 continue，Rust 侧保持 None
        assert!(fixed.get_byte_array_value(2).is_none());
        assert_eq!(fixed.get_index_from_offset(1000), -1);
    }

    /// 元数据给出的相邻偏移相同（长度为 0）时改用 minSize。
    #[test]
    fn zero_length_item_uses_min_size() {
        let meta = FixedMeta::with_item_size(&build_meta_stream(&[0, 0]), 8).expect("meta");
        let data = build_data_stream(10);

        let fixed = FixedData::with_limits(&meta, &data, 0, 4);

        assert_eq!(fixed.get_byte_array_value(0), Some(&data[0..4]));
        // 最后一条记录一直取到数据体末尾
        assert_eq!(fixed.get_byte_array_value(1), Some(&data[0..10]));
    }

    /// maxExpectedSize 限制单条记录长度。
    #[test]
    fn max_expected_size_limits_item_length() {
        let meta = FixedMeta::with_item_size(&build_meta_stream(&[0, 4]), 8).expect("meta");
        let data = build_data_stream(32);

        let fixed = FixedData::with_limits(&meta, &data, 6, 0);

        assert_eq!(fixed.get_byte_array_value(0), Some(&data[0..4]));
        // 最后一条从 4 到末尾共 28 字节，被 maxExpectedSize = 6 截断
        assert_eq!(fixed.get_byte_array_value(1), Some(&data[4..10]));
    }

    /// 使用调用方给定的记录长度切分数据体。
    #[test]
    fn uses_supplied_item_size() {
        let meta = FixedMeta::with_item_size(&build_meta_stream(&[0, 4, 8]), 8).expect("meta");
        let data = build_data_stream(18);

        let fixed = FixedData::with_item_size(&meta, 5, &data);

        assert_eq!(fixed.get_byte_array_value(0), Some(&data[0..5]));
        assert_eq!(fixed.get_byte_array_value(1), Some(&data[4..9]));
        // 第三条从偏移 8 起最多只有 10 字节可用，5 字节仍在范围内
        assert_eq!(fixed.get_byte_array_value(2), Some(&data[8..13]));
    }

    /// 固定长度数据体：itemSize 为负时取到数据体末尾。
    #[test]
    fn fixed_size_constructors() {
        let data = build_data_stream(10);

        let fixed = FixedData::with_fixed_size(4, &data);
        assert_eq!(fixed.get_item_count(), 2);
        assert_eq!(fixed.get_byte_array_value(0), Some(&data[0..4]));
        assert_eq!(fixed.get_byte_array_value(1), Some(&data[4..8]));
        assert_eq!(fixed.get_index_from_offset(4), 1);

        // 读取末尾残余数据
        let fixed = FixedData::with_fixed_size_and_remainder(4, &data, true);
        assert_eq!(fixed.get_item_count(), 3);
        assert_eq!(fixed.get_byte_array_value(2), Some(&data[8..10]));
        assert_eq!(fixed.get_index_from_offset(8), 2);

        // 长度为 0 的记录大小（Java 会抛 ArithmeticException）
        let fixed = FixedData::with_fixed_size(0, &data);
        assert_eq!(fixed.get_item_count(), 0);
        assert!(fixed.get_byte_array_value(0).is_none());
    }

    /// Display 输出每条数据项的下标、偏移与十六进制转储。
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
