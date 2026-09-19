//! 对应 Java: `org/mpxj/mpp/ExtendedData.java`

use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};

use chrono::NaiveDateTime;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::fix_defer_fix::FixDeferFix;
use crate::mpp::mpp_utility::MppUtility;

/// 扩展数据结构：保存与任务、资源相关联的附加非核心数据项。
///
/// 对应 Java: `final class ExtendedData`（包级可见）
///
/// 构造时把 var data 块中指定偏移处的数据块按 `[长度:4][类型:4][数据]` 逐项解析出来，
/// 并以「类型 → 数据」的形式放进 map。
///
/// # 移植说明（取舍）
///
/// - Java 的 `private final FixDeferFix m_data` 是「持有引用」；Rust 侧用
///   `&'a FixDeferFix`（不可变借用）表达，调用方只需保证 var data 活得比本对象久
///   （Java 里 `ExtendedData` 也只在同一方法内使用局部 var data）。
/// - Java 的 `byte[] getByteArray(Integer)` 返回内部数组本身；Rust 侧返回
///   `Option<&[u8]>`（借用内部存储，不复制）。
/// - Java 中若干处会抛出异常：解析项头时越界（`ArrayIndexOutOfBoundsException`）、
///   `size` 为负（`NegativeArraySizeException`）、子数组越过数据末尾
///   （`IndexOutOfBoundsException`）。Rust 侧一律改为「结束解析 / 跳过该项 / 返回默认值」
///   并注释，绝不 panic。
pub struct ExtendedData<'a> {
    /// 对应 Java 字段 `m_data`
    m_data: &'a FixDeferFix,
    /// 对应 Java 字段 `m_map`（类型 → 数据项）
    m_map: HashMap<i32, Vec<u8>>,
}

impl<'a> ExtendedData<'a> {
    /// 构造方法。以 var data 块中 `offset` 处的扩展数据块为输入。
    ///
    /// 对应 Java: `ExtendedData(FixDeferFix varData, int offset)`
    pub fn new(var_data: &'a FixDeferFix, offset: i32) -> Self {
        let mut map = HashMap::new();

        if let Some(data) = var_data.get_byte_array(offset) {
            let mut index = 0usize;

            while index < data.len() {
                // Java: `ByteArrayHelper.getInt(data, index)`，越界时抛异常；
                // Rust 侧放弃剩余数据（保留已解析的项）。
                if index + 4 > data.len() {
                    break;
                }
                let size = ByteArrayHelper::get_int(&data, index);
                index += 4;

                if index + 4 > data.len() {
                    break;
                }
                let item_type = ByteArrayHelper::get_int(&data, index);
                index += 4;

                // Java: size 为负时 `MPPUtility.cloneSubArray` 抛 NegativeArraySizeException，
                // 数据不够长时抛 IndexOutOfBoundsException；Rust 侧终止解析。
                let Ok(size) = usize::try_from(size) else {
                    break;
                };
                if index + size > data.len() {
                    break;
                }

                map.insert(item_type, MppUtility::clone_sub_array(&data, index, size));
                index += size;
            }
        }

        Self {
            m_data: var_data,
            m_map: map,
        }
    }

    /// 从扩展数据中取字符串。
    ///
    /// 对应 Java: `public String getString(Integer type)`
    pub fn get_string(&self, item_type: i32) -> Option<String> {
        let item = self.m_map.get(&item_type)?;
        let offset = Self::get_offset(item)?;
        self.m_data.get_string(offset)
    }

    /// 从扩展数据中取 Unicode 字符串。
    ///
    /// 对应 Java: `public String getUnicodeString(Integer type)`
    pub fn get_unicode_string(&self, item_type: i32) -> Option<String> {
        let item = self.m_map.get(&item_type)?;
        let offset = Self::get_offset(item)?;
        self.m_data.get_unicode_string(offset)
    }

    /// 从扩展数据中取短整型值。
    ///
    /// 对应 Java: `public int getShort(Integer type)`
    pub fn get_short(&self, item_type: i32) -> i32 {
        match self.m_map.get(&item_type) {
            // Java 在 item 不足 2 字节时抛异常，Rust 侧回落为 0
            Some(item) if item.len() >= 2 => ByteArrayHelper::get_short(item, 0),
            _ => 0,
        }
    }

    /// 从扩展数据中取整型值。
    ///
    /// 对应 Java: `public int getInt(Integer type)`
    pub fn get_int(&self, item_type: i32) -> i32 {
        match self.m_map.get(&item_type) {
            // Java 在 item 不足 4 字节时抛异常，Rust 侧回落为 0
            Some(item) if item.len() >= 4 => ByteArrayHelper::get_int(item, 0),
            _ => 0,
        }
    }

    /// 从扩展数据中取长整型值（六字节）。
    ///
    /// 对应 Java: `public long getLong(Integer type)`
    pub fn get_long(&self, item_type: i32) -> i64 {
        match self.m_map.get(&item_type) {
            // Java 在 item 不足 6 字节时抛异常，Rust 侧回落为 0
            Some(item) if item.len() >= 6 => MppUtility::get_long6(item, 0),
            _ => 0,
        }
    }

    /// 从扩展数据中取双精度浮点值。
    ///
    /// 对应 Java: `public double getDouble(Integer type)`
    pub fn get_double(&self, item_type: i32) -> f64 {
        match self.m_map.get(&item_type) {
            // Java 在 item 不足 8 字节时抛异常，Rust 侧回落为 0
            Some(item) if item.len() >= 8 => MppUtility::get_double(item, 0),
            _ => 0.0,
        }
    }

    /// 从扩展数据中取时间戳。
    ///
    /// 对应 Java: `public LocalDateTime getTimestamp(Integer type)`
    pub fn get_timestamp(&self, item_type: i32) -> Option<NaiveDateTime> {
        match self.m_map.get(&item_type) {
            // Java 在 item 不足 4 字节时抛异常，Rust 侧返回 None
            Some(item) if item.len() >= 4 => MppUtility::get_timestamp(item, 0),
            _ => None,
        }
    }

    /// 取原始字节数组。
    ///
    /// 对应 Java: `public byte[] getByteArray(Integer type)`
    pub fn get_byte_array(&self, item_type: i32) -> Option<&[u8]> {
        self.m_map.get(&item_type).map(Vec::as_slice)
    }

    /// 把整型值转换成偏移量。
    ///
    /// 对应 Java: `private int getOffset(byte[] data)`
    ///
    /// Java 侧 item 不足 4 字节时 `ByteArrayHelper.getInt` 会抛异常，
    /// Rust 侧返回 `None`（调用方随即回落为「没有该项」）。
    fn get_offset(data: &[u8]) -> Option<i32> {
        if data.len() < 4 {
            return None;
        }

        Some(-1 - ByteArrayHelper::get_int(data, 0))
    }
}

/// 调试输出：输出扩展数据块的格式化内容。
///
/// 对应 Java: `@Override public String toString()`
///
/// Java 用 `HashMap` 迭代（顺序不定），Rust 的 `HashMap` 同样不保证顺序。
impl Display for ExtendedData<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "BEGIN ExtendedData")?;

        for (item_type, data) in &self.m_map {
            writeln!(
                formatter,
                "Type: {} Data:{}",
                item_type,
                ByteArrayHelper::hexdump_all(data, false)
            )?;
        }

        writeln!(formatter, "END ExtendedData")?;
        writeln!(formatter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    /// 在 `data` 的 `offset` 处写入一个 FixDeferFix 数据块（`nextBlockOffset = -1`，
    /// 即数据紧随其后），返回写入的字节数。
    fn write_block(data: &mut [u8], offset: usize, payload: &[u8]) {
        data[offset..offset + 4].copy_from_slice(&(-1i32).to_le_bytes());
        data[offset + 4..offset + 8].copy_from_slice(&(payload.len() as i32).to_le_bytes());
        data[offset + 8..offset + 8 + payload.len()].copy_from_slice(payload);
    }

    /// 构造「扩展数据块」自身的字节内容：`[长度:4][类型:4][数据]` 序列。
    fn extended_block(items: &[(i32, Vec<u8>)]) -> Vec<u8> {
        let mut result = Vec::new();

        for (item_type, payload) in items {
            result.extend_from_slice(&(payload.len() as i32).to_le_bytes());
            result.extend_from_slice(&item_type.to_le_bytes());
            result.extend_from_slice(payload);
        }

        result
    }

    /// 构造 var data：偏移 8 处放扩展数据块本身（**单块**场景，负载不得超过首块容量 28 字节）。
    fn var_data(items: &[(i32, Vec<u8>)]) -> Vec<u8> {
        let block = extended_block(items);
        assert!(
            block.len() <= 28,
            "负载超过首块容量 28 字节，请改用 var_data_chained"
        );

        let mut data = vec![0u8; 8 + 8 + block.len()];
        write_block(&mut data, 8, &block);
        data
    }

    /// 构造**跨两个块**的 var data。
    ///
    /// `FixDeferFix` 是链式块布局：偏移处先是「下一块偏移」（`-1` 表示链尾），紧随其后是
    /// 「本条数据长度」，之后才是数据本体；首块承载 28 字节，后续块每块承载 32 字节。
    fn var_data_chained(items: &[(i32, Vec<u8>)]) -> Vec<u8> {
        let block = extended_block(items);
        assert!(
            block.len() > 28 && block.len() - 28 <= 32,
            "本辅助函数只覆盖「首块 28 字节 + 第二块不超过 32 字节」的情形"
        );

        // 第二块头部所在偏移：8（块 1 的头部起点）+ 8（头部长度）+ 28（块 1 承载的数据）
        let block2_offset: i32 = 44;

        let mut data = vec![0u8; 8];
        // 块 1：下一块偏移 + 本条数据总长度 + 前 28 字节数据
        data.extend_from_slice(&block2_offset.to_le_bytes());
        data.extend_from_slice(&(block.len() as i32).to_le_bytes());
        data.extend_from_slice(&block[..28]);
        // 块 2：链尾标记 + 剩余数据
        data.extend_from_slice(&(-1i32).to_le_bytes());
        data.extend_from_slice(&block[28..]);

        data
    }

    #[test]
    fn reads_numeric_items() {
        let data = var_data_chained(&[
            (100, vec![0x34, 0x12, 0x00, 0x00]),
            (200, vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F]),
            (300, vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06]),
        ]);
        let fix_defer_fix = FixDeferFix::new(&data);
        let extended = ExtendedData::new(&fix_defer_fix, 8);

        assert_eq!(extended.get_short(100), 0x1234);
        assert_eq!(extended.get_int(100), 0x1234);
        assert_eq!(
            extended.get_byte_array(100),
            Some([0x34u8, 0x12, 0x00, 0x00].as_slice())
        );
        assert_eq!(extended.get_double(200), 1.0);
        // 六字节小端长整型
        assert_eq!(extended.get_long(300), 0x0605_0403_0201);
    }

    #[test]
    fn reads_timestamp_items() {
        // 偏移 0 的 2 字节为时间（十分之一分钟），偏移 2 的 2 字节为「纪元 + 天数」
        let data = var_data(&[(100, vec![0x00, 0x00, 0x02, 0x00])]);
        let fix_defer_fix = FixDeferFix::new(&data);
        let extended = ExtendedData::new(&fix_defer_fix, 8);

        assert_eq!(
            extended.get_timestamp(100),
            Some(
                NaiveDate::from_ymd_opt(1984, 1, 2)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
            )
        );
    }

    #[test]
    fn missing_types_return_defaults() {
        let data = var_data(&[(100, vec![0x01, 0x00, 0x00, 0x00])]);
        let fix_defer_fix = FixDeferFix::new(&data);
        let extended = ExtendedData::new(&fix_defer_fix, 8);

        assert_eq!(extended.get_short(999), 0);
        assert_eq!(extended.get_int(999), 0);
        assert_eq!(extended.get_long(999), 0);
        assert_eq!(extended.get_double(999), 0.0);
        assert_eq!(extended.get_timestamp(999), None);
        assert_eq!(extended.get_string(999), None);
        assert_eq!(extended.get_unicode_string(999), None);
        assert_eq!(extended.get_byte_array(999), None);
    }

    #[test]
    fn short_items_do_not_panic() {
        // 只有 2 字节：short 可读，int / long / double / timestamp 取不到
        let data = var_data(&[(100, vec![0x34, 0x12])]);
        let fix_defer_fix = FixDeferFix::new(&data);
        let extended = ExtendedData::new(&fix_defer_fix, 8);

        assert_eq!(extended.get_short(100), 0x1234);
        assert_eq!(extended.get_int(100), 0);
        assert_eq!(extended.get_long(100), 0);
        assert_eq!(extended.get_double(100), 0.0);
        assert_eq!(extended.get_timestamp(100), None);
        // 不足 4 字节时无法解释偏移量
        assert_eq!(extended.get_string(100), None);
    }

    #[test]
    fn string_items_are_located_through_negative_offsets() {
        // 扩展数据块中，字符串项存放的是「-1 - 偏移量」；这里指向 var data 的偏移 100
        let items = vec![
            (10, (-1i32 - 100).to_le_bytes().to_vec()),
            (11, (-1i32 - 150).to_le_bytes().to_vec()),
        ];
        let block = extended_block(&items);

        let mut data = vec![0u8; 200];
        write_block(&mut data, 8, &block);
        // 偏移 100 处放单字节字符串 "AB"
        write_block(&mut data, 100, &[0x41, 0x42]);
        // 偏移 150 处放 UTF-16LE 字符串 "CD"
        write_block(&mut data, 150, &[0x43, 0x00, 0x44, 0x00]);

        let fix_defer_fix = FixDeferFix::new(&data);
        let extended = ExtendedData::new(&fix_defer_fix, 8);

        assert_eq!(extended.get_string(10).as_deref(), Some("AB"));
        assert_eq!(extended.get_unicode_string(11).as_deref(), Some("CD"));
    }

    #[test]
    fn absent_var_data_yields_an_empty_map() {
        // getByteArray(offset) 返回 None（offset 必须大于 0）
        let data = vec![0u8; 32];
        let fix_defer_fix = FixDeferFix::new(&data);
        let extended = ExtendedData::new(&fix_defer_fix, 0);

        assert_eq!(extended.get_byte_array(100), None);
        assert_eq!(extended.get_int(100), 0);
    }

    #[test]
    fn display_lists_every_item() {
        let data = var_data(&[(100, vec![0x34, 0x12, 0x00, 0x00])]);
        let fix_defer_fix = FixDeferFix::new(&data);
        let extended = ExtendedData::new(&fix_defer_fix, 8);

        let text = extended.to_string();

        assert!(text.starts_with("BEGIN ExtendedData\n"));
        assert!(text.contains("Type: 100 Data:34 12 00 00"));
        assert!(text.ends_with("END ExtendedData\n\n"));
    }
}
