//! 对应 Java: `org/mpxj/mpp/PropsBlock.java`

use std::collections::BTreeMap;
use std::fmt;

use chrono::{NaiveDateTime, NaiveTime};
use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::props::{
    Props, value_double, value_int, value_short, value_timestamp, value_unicode_string,
};
use crate::mpp::props_key::PropsKey;

/// 一个属性数据块。
///
/// 对应 Java: `final class PropsBlock extends Props`
///
/// # 继承的还原（组合 + 转发）
///
/// Java 侧 `PropsBlock` 继承 [`Props`]，因此同时拥有两套取值方法：
/// `Props` 的按键（`PropsKey`）版本，以及本类新增的按类型（`Integer`）版本。
/// Rust 无继承，故本类**持有一个 [`Props`]**：
///
/// - 按 `PropsKey` 取值的 11 个方法（`get_byte_array` / `get_byte` / `get_short` / …）
///   直接转发给内部 `Props`，语义等价于 Java 的继承；
/// - 按 `Integer` 取值的 6 个方法在 Java 里与前者构成**重载**，Rust 无重载，
///   统一加 `_by_type` 后缀（`get_byte_array_by_type` / `get_short_by_type` / …）。
///
/// # 块布局（Java 构造方法）
///
/// ```text
/// 偏移 0   int  dataSize    数据总大小（用作最后一个属性的结束偏移）
/// 偏移 4   int  itemCount   索引表条目数
/// 偏移 8   itemCount × (int itemKey, int itemOffset)   索引表
/// 其后                      各属性的原始数据
/// ```
///
/// `itemOffset` 是**块内绝对偏移**（Java 的 `populateMap` 直接从 `data` 起算）。
/// Java 用 `TreeMap` 按 `itemOffset` 升序排列后，依次用「上一个偏移 → 当前偏移」切出每个
/// 属性的字节区间，最后一个属性延伸到 `dataSize`；第一个属性由第二次迭代写出，
/// 故单条目块也能正确取出数据。
///
/// # 与 Java 的差异
///
/// - Java 在 `System.arraycopy` 越界或长度为负时抛 `AIOOBE` / `NegativeArraySizeException`；
///   Rust 侧对应的数据不做处理（该属性不写入），绝不 panic。
/// - Java 的 `toString()` 继承自 `Props`，Rust 侧同样转发 [`Props`] 的 [`Display`] 实现。
///
/// 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
pub struct PropsBlock {
    /// 对应 Java: `extends Props`（Rust 用组合代替继承）
    props: Props,
}

impl PropsBlock {
    /// 按块布局解析属性数据。
    ///
    /// 对应 Java: `PropsBlock(byte[] data)`
    pub fn new(data: &[u8]) -> Self {
        let mut result = Self { props: Props::new() };

        // Java: int dataSize = ByteArrayHelper.getInt(data, 0)
        // 数据不足时 Java 抛 AIOOBE，Rust 返回 0
        let data_size = if data.len() >= 4 {
            ByteArrayHelper::get_int(data, 0)
        } else {
            0
        };

        // Java: int itemCount = ByteArrayHelper.getInt(data, 4)
        let item_count = if data.len() >= 8 {
            ByteArrayHelper::get_int(data, 4)
        } else {
            0
        };

        // Java: 偏移表（TreeMap 保证按 itemOffset 升序迭代）
        let mut offset_map: BTreeMap<i32, i32> = BTreeMap::new();
        let mut offset = 8usize;
        for _ in 0..item_count {
            // Java: 越界抛 AIOOBE；Rust 侧直接停止读取索引表
            if offset + 8 > data.len() {
                break;
            }

            let item_key = ByteArrayHelper::get_int(data, offset);
            offset += 4;

            let item_offset = ByteArrayHelper::get_int(data, offset);
            offset += 4;

            offset_map.insert(item_offset, item_key);
        }

        let mut previous_item_offset: Option<i32> = None;
        let mut previous_item_key: Option<i32> = None;

        for item_offset in offset_map.keys() {
            if let (Some(previous_offset), Some(previous_key)) =
                (previous_item_offset, previous_item_key)
            {
                result.populate_map(data, previous_offset, previous_key, *item_offset);
            }

            previous_item_offset = Some(*item_offset);
            previous_item_key = offset_map.get(item_offset).copied();
        }

        if let (Some(previous_offset), Some(previous_key)) =
            (previous_item_offset, previous_item_key)
        {
            // Java: Integer itemOffset = Integer.valueOf(dataSize); populateMap(...)
            result.populate_map(data, previous_offset, previous_key, data_size);
        }

        result
    }

    /// 取属性数据（原始字节）。
    ///
    /// 对应 Java: `public byte[] getByteArray(Integer type)`
    pub fn get_byte_array_by_type(&self, type_: i32) -> Option<&[u8]> {
        self.props.get_byte_array_by_key(type_)
    }

    /// 取短整型值。
    ///
    /// 对应 Java: `public int getShort(Integer type)`
    pub fn get_short_by_type(&self, type_: i32) -> i32 {
        value_short(self.get_byte_array_by_type(type_))
    }

    /// 取整型值。
    ///
    /// 对应 Java: `public int getInt(Integer type)`
    pub fn get_int_by_type(&self, type_: i32) -> i32 {
        value_int(self.get_byte_array_by_type(type_))
    }

    /// 取双精度值。
    ///
    /// 对应 Java: `public double getDouble(Integer type)`
    pub fn get_double_by_type(&self, type_: i32) -> f64 {
        value_double(self.get_byte_array_by_type(type_))
    }

    /// 取时间戳。
    ///
    /// 对应 Java: `public LocalDateTime getTimestamp(Integer type)`
    pub fn get_timestamp_by_type(&self, type_: i32) -> Option<NaiveDateTime> {
        value_timestamp(self.get_byte_array_by_type(type_))
    }

    /// 取 UTF-16LE 字符串。
    ///
    /// 对应 Java: `public String getUnicodeString(Integer type)`
    pub fn get_unicode_string_by_type(&self, type_: i32) -> Option<String> {
        value_unicode_string(self.get_byte_array_by_type(type_))
    }

    /// 从块中截取一段数据并写入属性表。
    ///
    /// 对应 Java: `private void populateMap(byte[] data, Integer previousItemOffset, Integer previousItemKey, Integer itemOffset)`
    fn populate_map(
        &mut self,
        data: &[u8],
        previous_item_offset: i32,
        previous_item_key: i32,
        item_offset: i32,
    ) {
        // Java: int itemSize = itemOffset - previousItemOffset，为负时抛
        // NegativeArraySizeException，越界时抛 AIOOBE；Rust 侧跳过该属性
        if previous_item_offset > item_offset {
            return;
        }

        let start = previous_item_offset.max(0) as usize;
        let end = item_offset.max(0) as usize;
        if end > data.len() {
            return;
        }

        self.props.put(previous_item_key, data[start..end].to_vec());
    }

    // -----------------------------------------------------------------
    // 以下方法在 Java 侧继承自 Props，Rust 侧转发给内部持有的 Props
    // -----------------------------------------------------------------

    /// 取属性数据（原始字节）。
    ///
    /// 对应 Java: `Props.getByteArray(PropsKey key)`
    pub fn get_byte_array(&self, key: PropsKey) -> Option<&[u8]> {
        self.props.get_byte_array(key)
    }

    /// 取字节值。
    ///
    /// 对应 Java: `Props.getByte(PropsKey key)`
    pub fn get_byte(&self, key: PropsKey) -> i8 {
        self.props.get_byte(key)
    }

    /// 取短整型值。
    ///
    /// 对应 Java: `Props.getShort(PropsKey key)`
    pub fn get_short(&self, key: PropsKey) -> i32 {
        self.props.get_short(key)
    }

    /// 取整型值。
    ///
    /// 对应 Java: `Props.getInt(PropsKey key)`
    pub fn get_int(&self, key: PropsKey) -> i32 {
        self.props.get_int(key)
    }

    /// 取双精度值。
    ///
    /// 对应 Java: `Props.getDouble(PropsKey key)`
    pub fn get_double(&self, key: PropsKey) -> f64 {
        self.props.get_double(key)
    }

    /// 取时间值。
    ///
    /// 对应 Java: `Props.getTime(PropsKey key)`
    pub fn get_time(&self, key: PropsKey) -> Option<NaiveTime> {
        self.props.get_time(key)
    }

    /// 取时间戳。
    ///
    /// 对应 Java: `Props.getTimestamp(PropsKey key)`
    pub fn get_timestamp(&self, key: PropsKey) -> Option<NaiveDateTime> {
        self.props.get_timestamp(key)
    }

    /// 取布尔值。
    ///
    /// 对应 Java: `Props.getBoolean(PropsKey key)`
    pub fn get_boolean(&self, key: PropsKey) -> bool {
        self.props.get_boolean(key)
    }

    /// 取 UTF-16LE 字符串。
    ///
    /// 对应 Java: `Props.getUnicodeString(PropsKey key)`
    pub fn get_unicode_string(&self, key: PropsKey) -> Option<String> {
        self.props.get_unicode_string(key)
    }

    /// 取 GUID。
    ///
    /// 对应 Java: `Props.getUUID(PropsKey key)`
    pub fn get_uuid(&self, key: PropsKey) -> Option<Uuid> {
        self.props.get_uuid(key)
    }

    /// 取全部键（升序）。
    ///
    /// 对应 Java: `Props.keySet()`
    pub fn key_set(&self) -> Vec<i32> {
        self.props.key_set()
    }
}

impl fmt::Display for PropsBlock {
    /// 对应 Java: `Props.toString()`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.props, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 按 Java 的块布局构造数据：dataSize + itemCount + 索引表 + 属性数据。
    ///
    /// 索引表里的偏移是**块内绝对偏移**（Java 的 `populateMap` 直接从 `data` 起算），
    /// 并且最后一个属性以 `dataSize` 作为结束偏移 —— 所以这里让 `dataSize` 等于块的总长度。
    fn build_block(items: &[(i32, Vec<u8>)]) -> Vec<u8> {
        let base = 8 + 8 * items.len();
        let mut data = vec![0u8; base];
        let mut offset = base;

        for (index, (key, value)) in items.iter().enumerate() {
            data[8 + (index * 8)..8 + (index * 8) + 4].copy_from_slice(&key.to_le_bytes());
            data[8 + (index * 8) + 4..8 + (index * 8) + 8]
                .copy_from_slice(&(offset as i32).to_le_bytes());
            offset += value.len();
        }

        for (_, value) in items {
            data.extend_from_slice(value);
        }

        let total = data.len() as i32;
        data[0..4].copy_from_slice(&total.to_le_bytes());
        data[4..8].copy_from_slice(&(items.len() as i32).to_le_bytes());

        data
    }

    #[test]
    fn reads_block_items() {
        // 索引表按 itemOffset 排序（Java 用 TreeMap），即声明顺序
        let items = [
            (101, vec![0x01, 0x02]),                         // 短整数 513
            (102, vec![0xD2, 0x04, 0x00, 0x00]),             // 整数 1234
            (103, vec![0x41, 0x00, 0x42, 0x00, 0x00, 0x00]), // 字符串 "AB"
        ];
        let data = build_block(&items);

        let block = PropsBlock::new(&data);

        assert_eq!(block.key_set(), vec![101, 102, 103]);
        assert_eq!(block.get_byte_array_by_type(101), Some(&[0x01, 0x02][..]));
        assert_eq!(block.get_short_by_type(101), 513);
        assert_eq!(block.get_int_by_type(102), 1234);
        assert_eq!(
            block.get_unicode_string_by_type(103),
            Some("AB".to_owned())
        );
    }

    #[test]
    fn missing_type_returns_defaults() {
        let data = build_block(&[(101, vec![0x01, 0x02])]);
        let block = PropsBlock::new(&data);

        assert_eq!(block.get_byte_array_by_type(999), None);
        assert_eq!(block.get_short_by_type(999), 0);
        assert_eq!(block.get_int_by_type(999), 0);
        assert_eq!(block.get_double_by_type(999), 0.0);
        assert_eq!(block.get_timestamp_by_type(999), None);
        assert_eq!(block.get_unicode_string_by_type(999), None);

        // 长度不足的属性（2 字节）取 int/double 时 Java 抛 AIOOBE，Rust 返回默认值
        assert_eq!(block.get_int_by_type(101), 0);
        assert_eq!(block.get_double_by_type(101), 0.0);
        assert_eq!(block.get_timestamp_by_type(101), None);
    }

    #[test]
    fn last_item_ends_at_data_size() {
        let items = [
            (1, vec![0xAA]),
            (2, vec![0xBB, 0xCC, 0xDD]),
        ];
        let data = build_block(&items);

        let block = PropsBlock::new(&data);

        assert_eq!(block.get_byte_array_by_type(1), Some(&[0xAA][..]));
        assert_eq!(
            block.get_byte_array_by_type(2),
            Some(&[0xBB, 0xCC, 0xDD][..])
        );
    }

    #[test]
    fn malformed_data_does_not_panic() {
        // 空数据
        let block = PropsBlock::new(&[]);
        assert!(block.key_set().is_empty());

        // itemCount 声称有 3 项，但索引表被截断
        let mut data = Vec::new();
        data.extend_from_slice(&16i32.to_le_bytes());
        data.extend_from_slice(&3i32.to_le_bytes());
        data.extend_from_slice(&1i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        let block = PropsBlock::new(&data);
        assert_eq!(block.key_set(), vec![1]);

        // dataSize 超出实际数据长度：Java 抛 AIOOBE，Rust 跳过该属性
        let mut data = Vec::new();
        data.extend_from_slice(&1000i32.to_le_bytes());
        data.extend_from_slice(&1i32.to_le_bytes());
        data.extend_from_slice(&1i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&[0x01, 0x02]);
        let block = PropsBlock::new(&data);
        assert!(block.key_set().is_empty());

        // 最后一项的偏移大于 dataSize：Java 抛 NegativeArraySizeException，Rust 跳过
        let mut data = Vec::new();
        data.extend_from_slice(&2i32.to_le_bytes());
        data.extend_from_slice(&1i32.to_le_bytes());
        data.extend_from_slice(&7i32.to_le_bytes());
        data.extend_from_slice(&5i32.to_le_bytes());
        data.resize(21, 0);
        data.extend_from_slice(&[0x01, 0x02]);
        let block = PropsBlock::new(&data);
        assert!(block.key_set().is_empty());
    }

    #[test]
    fn forwards_props_key_methods() {
        let data = build_block(&[(502, vec![0x07, 0x00, 0x00, 0x00])]);
        let block = PropsBlock::new(&data);

        // 键值 502 不在 PropsKey 中，故按键取值返回默认值（证明转发链路可用）
        assert_eq!(block.get_int(PropsKey::Company), 0);
        assert_eq!(block.get_unicode_string(PropsKey::Company), None);
        assert!(block.to_string().starts_with("BEGIN Props\n"));
    }
}
