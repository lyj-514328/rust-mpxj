//! 对应 Java: `org/mpxj/mpp/FixDeferFix.java`
//!
//! 表示 Microsoft Project 98 文件格式里的一块变长数据项。数据以「块链」的形式存放：
//! 每个块头部是 4 字节的「下一个块偏移」，`-1` 表示链尾；`getByteArray` 会顺着链把
//! 一条数据项的所有分片拼回一个连续数组。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `MPPUtility.getByteArray` 越界抛异常 | `None` / 结束转储 | 先验证边界，再调用同一工具类 |
//!
//! # 跳过的 Java 成员 + 原因
//!
//! 无。
//!
//! 移植取舍：
//! - Java 的构造方法接收 `InputStream`，Rust 侧按约定改为 `&[u8]`，并保存为 `Vec<u8>`
//!   （Java 的 `InputStreamHelper.readAvailable` 本身就会复制一份数据，语义一致）。
//! - `MPPUtility.getByteArray` 在 Rust 侧需先校验范围，以便将 Java 的越界异常
//!   退化为 `None`；通过范围校验后复用 [`crate::mpp::mpp_utility::MPPUtility`]。
//! - Java 抛数组越界异常的地方（`ByteArrayHelper.getInt` / `MPPUtility.getByteArray`）在
//!   Rust 侧返回 `None`，绝不 panic。
//! - Java 的 `new String(byte[])` 使用平台默认字符集，Rust 侧按 UTF-8 容错解码；
//!   `MPPUtility.getUnicodeString` 按 UTF-16LE 解码（经 `encoding_rs`）。
//! - Java 的 `TreeSet<Integer>` 对应 `BTreeSet<i32>`；`toString()` 对应 [`Display`]，
//!   其中 Java 遇到越界会抛异常，Rust 侧改为立即结束转储。
//! - 未涉及反射、JDBC、XML/JAXB、GUI、`javax.*`、`Serializable`。

use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::mpp_utility::MPPUtility;

/// `FixDeferFix` 块：MPP98 格式中的变长数据块。
///
/// 对应 Java: `final class FixDeferFix extends MPPComponent`
pub struct FixDeferFix {
    /// 整个数据块。
    ///
    /// 对应 Java: `private final byte[] m_data`
    data: Vec<u8>,
}

/// [`FixDeferFix`] 的 `to_string` 遍历中单次迭代的结果。
enum DumpOutcome {
    /// 对应 Java 的 `continue`：跳过当前偏移。
    Skip,
    /// 成功解析出的一块数据。
    Emit(Vec<u8>),
    /// 对应 Java 在越界等处抛出的异常：停止转储。
    Abort,
}

impl FixDeferFix {
    /// 构造方法。数据块整体读入。
    ///
    /// 对应 Java: `FixDeferFix(InputStream is)`
    pub fn new(data: &[u8]) -> Self {
        // Java: m_data = InputStreamHelper.readAvailable(is)
        Self {
            data: data.to_vec(),
        }
    }

    /// 取从 `offset` 开始的一条数据项（可能跨多个块）。
    ///
    /// 对应 Java: `public byte[] getByteArray(int offset)`
    pub fn get_byte_array(&self, offset: i32) -> Option<Vec<u8>> {
        let data = &self.data;

        // Java: offset > 0 && offset < m_data.length
        if offset <= 0 || offset >= data.len() as i32 {
            return None;
        }

        let mut offset = offset;

        // Java: ByteArrayHelper.getInt(m_data, offset)，越界抛数组越界异常；Rust 侧返回 None
        let mut next_block_offset = read_int(data, offset)?;
        offset += 4;

        let item_size = read_int(data, offset)?;
        offset += 4;

        // Java: itemSize > 0 && itemSize < m_data.length
        if item_size <= 0 || item_size >= data.len() as i32 {
            return None;
        }

        // 除首个块外的每个块可承载的数据长度
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

    /// 取指定偏移处的字符串。
    ///
    /// 对应 Java: `public String getString(int offset)`
    pub fn get_string(&self, offset: i32) -> Option<String> {
        let data = self.get_byte_array(offset)?;
        // Java: new String(data) 使用平台默认字符集；Rust 侧按 UTF-8 容错解码
        Some(String::from_utf8_lossy(&data).into_owned())
    }

    /// 取指定偏移处的 Unicode（UTF-16LE）字符串。
    ///
    /// 对应 Java: `public String getUnicodeString(int offset)`
    pub fn get_unicode_string(&self, offset: i32) -> Option<String> {
        let data = self.get_byte_array(offset)?;
        Some(MPPUtility::get_unicode_string(&data, 0))
    }

    /// 从 `file_offset` 开始走一遍块链，返回该偏移处数据块的内容。
    ///
    /// 对应 Java: `FixDeferFix#toString()` 里 while 循环的循环体；Java 抛异常的地方
    /// 返回 [`DumpOutcome::Abort`]（Rust 侧不 panic），Java 的 `continue` 对应
    /// [`DumpOutcome::Skip`]，成功解析出数据对应 [`DumpOutcome::Emit`]。
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

        // Java: buffer = new byte[itemSize]，itemSize 为负时抛 NegativeArraySizeException
        if item_size < 0 {
            *file_offset = file_offset_value;
            return DumpOutcome::Abort;
        }

        let mut item_remaining_size = item_size;
        let mut buffer = vec![0u8; item_size as usize];
        let mut buffer_offset = 0i32;

        while next_block_offset != -1 {
            // Java: MPPUtility.getByteArray(...)，越界抛异常
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
                // Java: fileOffset = available 后继续循环，接着就会在 getByteArray 中越界抛异常
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

/// 转储块的调试视图（对应 Java 的 `toString()`，仅供调试使用）。
impl Display for FixDeferFix {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "BEGIN FixDeferFix")?;

        // Java: int available = m_data.length
        let available = self.data.len() as i32;

        // Java: 跳过 4 字节头部
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

/// 从 `data[offset..offset+4]` 读取一个小端 int。
///
/// 对应 Java: `ByteArrayHelper.getInt(byte[], int)`（Java 越界抛
/// `ArrayIndexOutOfBoundsException`，Rust 侧返回 `None`）。
fn read_int(data: &[u8], offset: i32) -> Option<i32> {
    if offset < 0 {
        return None;
    }

    let start = offset as usize;
    let end = start.checked_add(4)?;
    data.get(start..end).map(|bytes| ByteArrayHelper::get_int(bytes, 0))
}

/// 把 `data[offset..offset+size]` 复制到 `dest[dest_offset..]`。
///
/// 对应 Java: `MPPUtility.getByteArray(byte[] data, int offset, int size, byte[] buffer, int bufferOffset)`
/// （Java 由 `System.arraycopy` 检查边界并抛异常，Rust 侧返回 `None`）。
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

    /// 写入小端 int。
    fn write_int(data: &mut [u8], offset: usize, value: i32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// 单块数据：nextBlockOffset = -1，itemSize = 28，其后 28 字节为数据。
    fn build_single_block_stream() -> Vec<u8> {
        let mut data = vec![0u8; 40];
        write_int(&mut data, 4, -1);
        write_int(&mut data, 8, 28);

        for index in 0..28 {
            data[12 + index] = index as u8;
        }

        data
    }

    /// 单块：整条数据都在第一个块里。
    #[test]
    fn reads_single_block() {
        let data = build_single_block_stream();
        let fix_defer = FixDeferFix::new(&data);

        let result = fix_defer.get_byte_array(4).expect("data found");
        assert_eq!(result.len(), 28);
        assert_eq!(result.as_slice(), &data[12..40]);
    }

    /// 跨块：第一块 28 字节，链到第二个块再取 12 字节，拼成 40 字节。
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

    /// 偏移非法或数据块不完整时返回 None。
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

        // itemSize 不小于数据块长度 => None
        let mut clipped = vec![0u8; 40];
        write_int(&mut clipped, 4, -1);
        write_int(&mut clipped, 8, 40);
        assert!(FixDeferFix::new(&clipped).get_byte_array(4).is_none());
    }

    /// 空数据块返回 None。
    #[test]
    fn empty_block_returns_none() {
        let fix_defer = FixDeferFix::new(&[]);

        assert!(fix_defer.get_byte_array(4).is_none());
        assert!(fix_defer.get_string(4).is_none());
        assert!(fix_defer.get_unicode_string(4).is_none());
    }

    /// 单字节字符串。
    #[test]
    fn reads_ascii_string() {
        let mut data = vec![0u8; 17];
        write_int(&mut data, 4, -1);
        write_int(&mut data, 8, 5);
        data[12..17].copy_from_slice(b"hello");

        let fix_defer = FixDeferFix::new(&data);
        assert_eq!(fix_defer.get_string(4), Some("hello".to_string()));
    }

    /// UTF-16LE 字符串：遇到双字节 0 结束。
    #[test]
    fn reads_unicode_string() {
        let mut data = vec![0u8; 40];
        write_int(&mut data, 4, -1);
        write_int(&mut data, 8, 28);
        data[12..16].copy_from_slice(&[0x48, 0x00, 0x69, 0x00]); // "Hi"

        let fix_defer = FixDeferFix::new(&data);
        assert_eq!(fix_defer.get_unicode_string(4), Some("Hi".to_string()));
    }

    /// Display 输出块头与每块数据的十六进制转储。
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

    /// Display 中 nextBlockOffset = -1 且 itemSize > 28 的块会被整体跳过。
    #[test]
    fn display_skips_oversized_tail_block() {
        let mut data = vec![0u8; 60];
        // 第一个块：next = -1 且 itemSize = 100 > 28 => 跳过
        write_int(&mut data, 4, -1);
        write_int(&mut data, 8, 100);
        // 第二个块：next = -1，itemSize = 4，数据 "ABCD"
        write_int(&mut data, 40, -1);
        write_int(&mut data, 44, 4);
        data[48..52].copy_from_slice(b"ABCD");

        let text = FixDeferFix::new(&data).to_string();
        assert!(text.contains("   Data: offset: 40 size: 4\n"));
        assert!(!text.contains("   Data: offset: 4 size:"));
    }
}
