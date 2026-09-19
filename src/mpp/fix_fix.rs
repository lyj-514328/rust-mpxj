//! 对应 Java: `org/mpxj/mpp/FixFix.java`
//!
//! 表示 Microsoft Project 98 文件格式里的一块定长数据项。正常情况下块大小是 `itemSize`
//! 的整数倍，但也见过偏大的实例，因此这里记录块大小除以 `itemSize` 的余数
//! （[`FixFix::get_diff`]），供调用方判断是否需要用别的记录大小重新解释数据。
//!
//! 移植取舍：
//! - Java 的构造方法接收 `InputStream`，Rust 侧按约定改为 `&[u8]`；Java 的
//!   `is.available()` 对应切片长度。
//! - Java 继承 `MPPComponent` 复用 `readByteArray`；Rust 侧直接按记录大小切分切片。
//! - Java 的 `Object[]` 只会存放 `byte[]`，因此 Rust 侧为 `Vec<Vec<u8>>`
//!   （构造时每条记录都会被填充，Java 里的 `null` 检查在本类中恒不成立）。
//! - Java 的 `toString()` 对应 [`Display`]。
//! - Java 会抛异常的两处（`itemSize` 为 0 导致的除零、`getByteArrayValue` 的数组越界）
//!   在 Rust 侧分别取 0 / 返回 `None`，绝不 panic。
//! - 未涉及反射、JDBC、XML/JAXB、GUI、`javax.*`、`Serializable`。

use std::fmt::{Display, Formatter};

use crate::common::byte_array_helper::ByteArrayHelper;

/// `FixFix` 块：MPP98 格式中的定长数据块。
///
/// 对应 Java: `final class FixFix extends MPPComponent`
pub struct FixFix {
    /// 块内所有数据项。
    ///
    /// 对应 Java: `private final Object[] m_array`
    array: Vec<Vec<u8>>,
    /// 块的整体大小。
    ///
    /// 对应 Java: `private final int m_size`
    size: i32,
    /// 块大小除以记录大小后的余数。
    ///
    /// 对应 Java: `private final int m_diff`
    diff: i32,
}

impl FixFix {
    /// 构造方法。从数据中按 `item_size` 切分定长记录，并记录除不尽的余数。
    ///
    /// 对应 Java: `FixFix(int itemSize, InputStream is)`
    pub fn new(item_size: i32, data: &[u8]) -> Self {
        // Java: m_size = is.available()
        let size = data.len() as i32;

        // Java: m_size % itemSize 与 m_size / itemSize，itemSize 为 0 时抛
        // ArithmeticException；Rust 侧统一取 0
        let (diff, item_count) = if item_size > 0 {
            (size % item_size, size / item_size)
        } else {
            (0, 0)
        };

        let mut array = Vec::with_capacity(item_count.max(0) as usize);
        let mut position = 0usize;

        for _ in 0..item_count {
            // Java: readByteArray(is, itemSize)，读不足一条时抛异常；
            // Rust 侧取到切片末尾为止（正常情况下不会发生）
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

    /// 取块大小除以记录大小后的余数；非 0 说明块内记录的大小与预期不符。
    ///
    /// 对应 Java: `public int getDiff()`
    pub fn get_diff(&self) -> i32 {
        self.diff
    }

    /// 取块的整体大小。
    ///
    /// 对应 Java: `public int getSize()`
    pub fn get_size(&self) -> i32 {
        self.size
    }

    /// 取指定下标的数据项；下标越界时返回 `None`。
    ///
    /// 对应 Java: `public byte[] getByteArrayValue(int index)`
    ///
    /// Java 侧没有做下标检查（越界抛 `ArrayIndexOutOfBoundsException`），Rust 侧返回 `None`。
    pub fn get_byte_array_value(&self, index: i32) -> Option<&[u8]> {
        if index < 0 {
            return None;
        }

        self.array.get(index as usize).map(Vec::as_slice)
    }

    /// 取块内记录条数。
    ///
    /// 对应 Java: `public int getItemCount()`
    pub fn get_item_count(&self) -> i32 {
        self.array.len() as i32
    }
}

/// 转储块的调试视图（对应 Java 的 `toString()`，仅供调试使用）。
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

    /// 用递增字节的数据便于断言切片内容。
    fn build_data(length: usize) -> Vec<u8> {
        (0..length).map(|index| index as u8).collect()
    }

    /// 正常情况：块大小是记录大小的整数倍。
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

    /// 块大小偏大时余数非 0，余下的字节不会成为一条记录。
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

    /// 越界下标返回 None（Java 会抛 ArrayIndexOutOfBoundsException）。
    #[test]
    fn out_of_range_index_returns_none() {
        let fix = FixFix::new(4, &build_data(8));

        assert!(fix.get_byte_array_value(2).is_none());
        assert!(fix.get_byte_array_value(-1).is_none());
    }

    /// 空数据块与记录大小为 0（Java 会抛 ArithmeticException）。
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

    /// Display 输出每条记录的十六进制转储。
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
