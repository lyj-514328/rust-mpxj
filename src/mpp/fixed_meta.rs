//! 对应 Java: `org/mpxj/mpp/FixedMeta.java`
//!
//! 表示 MPP 文件中的 "FixedMeta" 流：块头描述了关联 "FixedData" 块中记录的条数，
//! 其后是若干条定长记录，每条记录描述 `FixedData` 块中对应记录的位置（偏移等）。
//! 块内记录的具体结构目前并未完全弄清，因此这里只做整体持有与按偏移切片。
//!
//! 移植取舍：
//! - Java 的构造方法接收 `InputStream`，Rust 侧按约定改为 `&[u8]`；Java 的
//!   `is.available()` 对应 `data.len()`（POI 保证该值准确）。
//! - Java 继承 `MPPComponent` 复用 `readInt` / `readByteArray`；Rust 侧改为本文件的私有
//!   函数 [`read_int`] / [`read_byte_array`]（数据不足时对应 Java 的 `EOFException`，
//!   返回 `Err`）。
//! - Java 的 `Object[]` 只会存放 `byte[]`，因此 Rust 侧为 `Vec<Option<Vec<u8>>>`。
//! - Java 的 `toString()` 对应 [`Display`]。
//! - Java 会抛异常的几处（坏 magic、`itemSize` 为 0 导致的除零、负数组长度、数组越界）
//!   在 Rust 侧分别改为返回 `Err` 或取 0 / `None`，绝不 panic。
//! - 未涉及反射、JDBC、XML/JAXB、GUI、`javax.*`、`Serializable`。

use std::fmt::{Display, Formatter};
use std::io;

use crate::common::byte_array_helper::ByteArrayHelper;

use super::fixed_data::FixedData;
use super::fixed_meta_item_size_provider::FixedMetaItemSizeProvider;

/// `FixedMeta` 块：块头 + 若干定长记录。
///
/// 对应 Java: `final class FixedMeta extends MPPComponent`
pub struct FixedMeta {
    /// 块头报告的记录条数。
    ///
    /// 对应 Java: `private final int m_itemCount`
    item_count: i32,
    /// 依据块大小与记录大小推算出的记录条数。
    ///
    /// 对应 Java: `private final int m_adjustedItemCount`
    adjusted_item_count: i32,
    /// 每条记录对应的数据项（Java 侧为 `Object[]`，每个元素或者是 `byte[]` 或者是 `null`）。
    ///
    /// 对应 Java: `private final Object[] m_array`
    array: Vec<Option<Vec<u8>>>,
}

/// 块头魔数。
///
/// 对应 Java: `private static final int MAGIC = 0xFADFADBA`（Java 的 `int` 为有符号，
/// 字面量本身就是负数，这里用同样的位模式）。
const MAGIC: i32 = 0xFADFADBA_u32 as i32;

/// 块头长度。
///
/// 对应 Java: `private static final int HEADER_SIZE = 16`
const HEADER_SIZE: i32 = 16;

impl FixedMeta {
    /// 构造方法。已知块大小，因此忽略块头里的记录条数，直接按给定记录大小推算。
    ///
    /// 对应 Java: `FixedMeta(InputStream is, final int itemSize)`
    pub fn with_item_size(data: &[u8], item_size: i32) -> io::Result<Self> {
        let provider = move |_file_size: i32, _item_count: i32| item_size;
        Self::new(data, &provider)
    }

    /// 构造方法。由给定的 item size provider 决定记录大小。
    ///
    /// 对应 Java: `FixedMeta(InputStream is, FixedMetaItemSizeProvider itemSizeProvider)`
    pub fn new(data: &[u8], item_size_provider: &dyn FixedMetaItemSizeProvider) -> io::Result<Self> {
        // Java: int fileSize = is.available(); POI 保证该值准确
        let file_size = data.len() as i32;

        //
        // 块头：4 个 int（第 1 个是 magic，第 3 个是记录条数，其余未知）
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

        // Java: (fileSize - HEADER_SIZE) / itemSize，itemSize 为 0 时抛
        // ArithmeticException，结果为负时构造数组抛 NegativeArraySizeException；
        // Rust 侧统一取 0（Java 的 int 除法向零截断，负数分子同样得到 0）。
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

    /// 构造方法。给出若干候选记录大小，结合另一个定长块的数据条数做启发式选择。
    ///
    /// 对应 Java: `FixedMeta(InputStream is, final FixedData otherFixedBlock, final int... itemSizes)`
    pub fn with_item_sizes(
        data: &[u8],
        other_fixed_block: &FixedData,
        item_sizes: &[i32],
    ) -> io::Result<Self> {
        let provider = move |file_size: i32, item_count: i32| {
            // Java: itemSizes[0]，数组为空时抛 ArrayIndexOutOfBoundsException；Rust 侧取 0
            let mut item_size = item_sizes.first().copied().unwrap_or(0);
            let available = file_size - HEADER_SIZE;
            // Java: Integer.MIN_VALUE
            let mut distance = i32::MIN;
            let other_fixed_block_count = other_fixed_block.get_item_count();

            for &test_item_size in item_sizes {
                // Java 对 0 取模会抛 ArithmeticException；Rust 侧跳过该候选
                if test_item_size == 0 || available % test_item_size != 0 {
                    continue;
                }

                //
                // 若候选大小正好整除块大小，且与另一块的记录条数一致，基本可以确定
                // 这就是正确的记录大小，直接结束选择。
                //
                if available / test_item_size == other_fixed_block_count {
                    item_size = test_item_size;
                    break;
                }

                //
                // 否则用经验规则选择最接近的候选（Java 的 int 溢出会回绕，
                // Rust 侧用 wrapping 保持同样语义，且不会在 debug 下 panic）。
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

    /// 取块头报告的记录条数。
    ///
    /// 对应 Java: `public int getItemCount()`
    pub fn get_item_count(&self) -> i32 {
        self.item_count
    }

    /// 取依据块大小与记录大小推算出的记录条数。
    ///
    /// 对应 Java: `public int getAdjustedItemCount()`
    pub fn get_adjusted_item_count(&self) -> i32 {
        self.adjusted_item_count
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
}

/// 转储块的调试视图（对应 Java 的 `toString()`，仅供调试使用）。
impl Display for FixedMeta {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "BEGIN: FixedMeta")?;
        writeln!(f, "   Adjusted Item count: {}", self.adjusted_item_count)?;

        for loop_index in 0..self.adjusted_item_count {
            writeln!(f, "   Data at index: {loop_index}")?;
            // Java: ByteArrayHelper.hexdump((byte[]) m_array[loop], true)，元素为 null 时输出空串
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

/// 从 `data[pos..]` 读取一个小端 int，并前移 `pos`。
///
/// 对应 Java: `MPPComponent#readInt(InputStream)`（数据不足时 Java 抛 `EOFException`，
/// Rust 侧返回 `ErrorKind::UnexpectedEof`）。
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

/// 从 `data[pos..]` 读取 `size` 个字节，并前移 `pos`。
///
/// 对应 Java: `MPPComponent#readByteArray(InputStream, int)`（数据不足时 Java 抛
/// `RuntimeException`，`size` 为负时抛 `NegativeArraySizeException`；Rust 侧返回 `Err`）。
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

    /// 手工构造 "FixedMeta" 字节流：16 字节块头 + `count` 条 `item_size` 字节记录。
    ///
    /// 每条记录：前 4 字节为标记 `index`，第 4..8 字节为该记录指向 `FixedData`
    /// 数据体中的偏移，其余字节填 `0xEE`。
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

    /// 块头报告的条数与按块大小推算的条数分别存放在两个 getter 中。
    #[test]
    fn reads_header_and_adjusted_item_count() {
        // 3 条 8 字节记录 => 16 + 24 = 40 字节，块头谎报 99 条
        let data = build_meta_stream(8, 3, 99, &[0, 8, 16]);
        assert_eq!(data.len(), 40);

        let meta = FixedMeta::with_item_size(&data, 8).expect("meta parsed");

        assert_eq!(meta.get_item_count(), 99);
        assert_eq!(meta.get_adjusted_item_count(), 3);
    }

    /// 按下标取记录内容（含记录内的偏移字段）。
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

    /// 越界下标返回 None，绝不 panic。
    #[test]
    fn out_of_range_index_returns_none() {
        let data = build_meta_stream(8, 2, 2, &[0, 8]);
        let meta = FixedMeta::with_item_size(&data, 8).expect("meta parsed");

        assert!(meta.get_byte_array_value(2).is_none());
        assert!(meta.get_byte_array_value(1000).is_none());
        assert!(meta.get_byte_array_value(-1).is_none());
        assert!(meta.get_byte_array_value(i32::MIN).is_none());
    }

    /// 坏 magic 返回 Err（Java 抛 IOException）。
    #[test]
    fn bad_magic_returns_error() {
        let mut data = build_meta_stream(8, 1, 1, &[0]);
        data[0] = 0x00;

        assert!(FixedMeta::with_item_size(&data, 8).is_err());
    }

    /// 空数据流（连块头都没有）返回 Err（Java 抛 EOFException）。
    #[test]
    fn empty_stream_returns_error() {
        assert!(FixedMeta::with_item_size(&[], 8).is_err());
        assert!(FixedMeta::with_item_size(&[0u8; 8], 8).is_err());
    }

    /// 记录大小取自 provider，且 provider 收到的是块大小与块头报告的条数。
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

    /// provider 返回 0 / 负数时不会 panic，得到空块（Java 会抛异常）。
    #[test]
    fn zero_item_size_yields_empty_block() {
        let data = build_meta_stream(8, 3, 3, &[0, 8, 16]);
        let meta = FixedMeta::with_item_size(&data, 0).expect("meta parsed");

        assert_eq!(meta.get_adjusted_item_count(), 0);
        assert!(meta.get_byte_array_value(0).is_none());
    }

    /// 块大小不是记录大小的整数倍时忽略尾部残余字节。
    #[test]
    fn ignores_partial_trailing_item() {
        // 16 + 3*8 + 4 = 44 字节，44-16 = 28 => 28 / 8 = 3 条
        let mut data = build_meta_stream(8, 3, 3, &[0, 8, 16]);
        data.extend_from_slice(&[0xEE; 4]);

        let meta = FixedMeta::with_item_size(&data, 8).expect("meta parsed");
        assert_eq!(meta.get_adjusted_item_count(), 3);
    }

    /// 候选记录大小与另一块的条数匹配时直接选中（Java 的第一个 break 分支）。
    #[test]
    fn chooses_item_size_matching_other_block_count() {
        // FixedData 数据体：3 条 8 字节记录
        let fixed_data = FixedData::with_fixed_size(8, &[0xAA; 24]);
        assert_eq!(fixed_data.get_item_count(), 3);

        // available = 24 => 候选 6 与 8 都能整除，只有 8 能得到 3 条
        let data = build_meta_stream(8, 3, 3, &[0, 8, 16]);
        let meta = FixedMeta::with_item_sizes(&data, &fixed_data, &[6, 8]).expect("meta parsed");

        assert_eq!(meta.get_adjusted_item_count(), 3);
    }

    /// 没有候选满足条件时退回到第一个候选（经验规则分支不满足）。
    #[test]
    fn falls_back_to_first_candidate_when_none_fit() {
        let fixed_data = FixedData::with_fixed_size(8, &[0xAA; 24]);

        // available = 24：候选 10 不能整除；候选 12 虽然整除但条数不匹配、
        // 且 (itemCount * 12 - available) > 0 使经验规则也不成立 => 保持 itemSizes[0] = 10
        let data = build_meta_stream(12, 2, 3, &[0, 12]);
        let meta = FixedMeta::with_item_sizes(&data, &fixed_data, &[10, 12]).expect("meta parsed");

        assert_eq!(meta.get_item_count(), 3);
        // (40 - 16) / 10 = 2
        assert_eq!(meta.get_adjusted_item_count(), 2);
        assert_eq!(meta.get_byte_array_value(0).map(<[u8]>::len), Some(10));
    }

    /// Display 输出块头信息与每条记录的十六进制转储。
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
