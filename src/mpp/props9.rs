//! 对应 Java: `org/mpxj/mpp/Props9.java`

use std::fmt;

use chrono::{NaiveDateTime, NaiveTime};
use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::props::{InputStreamCursor, Props, read_error};
use crate::mpp::props_key::PropsKey;
use crate::mpxj_exception::MpxjResult;

/// MPP9 文件的 `Props` 流。
///
/// 对应 Java: `final class Props9 extends Props`
///
/// # 继承的还原（组合 + 转发）
///
/// Java 的 `Props9` 继承 [`Props`]（Java 侧未覆写 `toString()`），Rust 无继承，故本类
/// 持有一个 [`Props`] 并把 `Props` 的全部公开方法原样转发。
///
/// # 构造方法与 `InputStream`
///
/// Java: `Props9(InputStream is) throws IOException`。Rust 侧按本工程约定把
/// `InputStream` 换成 `&[u8]`，顺序读取由 [`InputStreamCursor`] 完成；Java 的
/// `IOException`（读取不足）对应 Rust 的 `Err(MpxjError)`。
///
/// # 块布局（Java 构造方法）
///
/// ```text
/// byte[16] header      headerCount = 该头部的 short（偏移 12）—— 条目数
/// headerCount × {
///     int  itemSize    属性数据长度
///     int  itemKey     属性键
///     int  unknown
///     byte data[itemSize]
///     byte pad         仅当 itemSize 为奇数时补齐到两字节边界
/// }
/// ```
///
/// Java 在 `is.available() < 16` 时直接返回空表；循环中出现
/// `availableBytes < itemSize || itemSize < 1` 时中断（不抛异常）。
///
/// # 与 Java 的差异
///
/// - 跳过 `getValue`/反射无关内容；`toString()` 沿用 [`Props`] 的实现（Java 未覆写）。
/// - `availableBytes` 用 `i64` 累减（Java 是 `int`，极端输入下会溢出）。
///
/// 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
pub struct Props9 {
    /// 对应 Java: `extends Props`（Rust 用组合代替继承）
    props: Props,
}

impl Props9 {
    /// 构造方法。
    ///
    /// 对应 Java: `Props9(InputStream is) throws IOException`
    pub fn new(data: &[u8]) -> MpxjResult<Self> {
        let mut result = Self { props: Props::new() };
        let mut is = InputStreamCursor::new(data);

        // Java: if (is.available() < 16) { return; }
        if is.available() < 16 {
            return Ok(result);
        }

        // Java: byte[] header = InputStreamHelper.read(is, 16)
        let header = is.read(16).ok_or_else(read_error)?;
        // Java: int headerCount = ByteArrayHelper.getShort(header, 12)
        let header_count = ByteArrayHelper::get_short(header, 12);
        let mut found_count = 0;
        // Java: int availableBytes = is.available()
        let mut available_bytes = is.available() as i64;

        while found_count < header_count {
            // Java: int itemSize = readInt(is)
            let item_size = is.read_int().ok_or_else(read_error)?;
            // Java: int itemKey = readInt(is)
            let item_key = is.read_int().ok_or_else(read_error)?;
            // Java: /*int attrib3 = */ readInt(is)
            is.read_int().ok_or_else(read_error)?;
            available_bytes -= 12;

            if available_bytes < i64::from(item_size) || item_size < 1 {
                break;
            }

            // Java: byte[] data = InputStreamHelper.read(is, itemSize)
            let item = is.read(item_size as usize).ok_or_else(read_error)?;
            available_bytes -= i64::from(item_size);

            result.props.put(item_key, item.to_vec());
            found_count += 1;

            // Java: 对齐到两字节边界
            if item.len() % 2 != 0 {
                is.skip(1);
            }
        }

        Ok(result)
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

impl fmt::Display for Props9 {
    /// 对应 Java: `Props.toString()`（`Props9` 未覆写）
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.props, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 按 MPP9 的块布局构造数据。
    fn build_props9(header_count: i32, entries: &[(i32, i32, Vec<u8>)]) -> Vec<u8> {
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&(header_count as i16).to_le_bytes());

        for (size, key, value) in entries {
            data.extend_from_slice(&size.to_le_bytes());
            data.extend_from_slice(&key.to_le_bytes());
            data.extend_from_slice(&0i32.to_le_bytes());
            data.extend_from_slice(value);
            // Java: 对齐到两字节边界
            if value.len() % 2 != 0 {
                data.push(0);
            }
        }

        data
    }

    #[test]
    fn reads_entries() {
        let data = build_props9(
            2,
            &[
                // PropsKey.TITLE == 37748744
                (4, 37748744, vec![0xD2, 0x04, 0x00, 0x00]),
                // PropsKey.SUBJECT == 37748745
                (2, 37748745, vec![0x02, 0x01]),
            ],
        );

        let props = Props9::new(&data).expect("应能解析");
        assert_eq!(props.key_set(), vec![37748744, 37748745]);
        assert_eq!(props.get_int(PropsKey::Title), 1234);
        assert_eq!(props.get_short(PropsKey::Subject), 258);
    }

    #[test]
    fn odd_length_data_is_aligned() {
        let data = build_props9(
            2,
            &[
                (1, 37748744, vec![0x07]),
                (2, 37748745, vec![0x02, 0x01]),
            ],
        );

        let props = Props9::new(&data).expect("应能解析");
        assert_eq!(props.get_byte(PropsKey::Title), 7);
        assert_eq!(props.get_short(PropsKey::Subject), 258);
    }

    #[test]
    fn short_header_returns_empty_props() {
        // Java: if (is.available() < 16) { return; } —— 不抛异常
        let props = Props9::new(&[0u8; 15]).expect("应能解析");
        assert!(props.key_set().is_empty());
        assert_eq!(props.get_int(PropsKey::Title), 0);
    }

    #[test]
    fn declared_count_larger_than_data_returns_error() {
        // 声明 3 条，实际只有 1 条：读第 2 条的头部时已到流末尾
        // （Java 侧 POI 的 DocumentInputStream 在此抛 IOException，故 Rust 返回 Err）
        let data = build_props9(3, &[(2, 37748744, vec![0x02, 0x01])]);

        assert!(Props9::new(&data).is_err());
    }

    #[test]
    fn item_size_beyond_available_bytes_breaks_loop_without_error() {
        // 第 2 条的头部完整但 itemSize 超过剩余字节 → Java: availableBytes < itemSize → break
        let mut data = build_props9(2, &[(2, 37748744, vec![0x02, 0x01])]);
        data.extend_from_slice(&100i32.to_le_bytes());
        data.extend_from_slice(&37748745i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());

        let props = Props9::new(&data).expect("应能解析");
        assert_eq!(props.key_set(), vec![37748744]);
    }

    #[test]
    fn invalid_item_size_breaks_loop() {
        // itemSize == 0 → Java: itemSize < 1 → break
        let data = build_props9(
            2,
            &[
                (2, 37748744, vec![0x02, 0x01]),
                (0, 37748745, Vec::new()),
            ],
        );

        let props = Props9::new(&data).expect("应能解析");
        assert_eq!(props.key_set(), vec![37748744]);
    }

    #[test]
    fn item_size_beyond_available_bytes_breaks_loop() {
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&2i16.to_le_bytes());
        // 第一条：itemSize 远大于剩余字节
        data.extend_from_slice(&100i32.to_le_bytes());
        data.extend_from_slice(&37748744i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());

        let props = Props9::new(&data).expect("应能解析");
        assert!(props.key_set().is_empty());
    }

    #[test]
    fn truncated_entry_returns_error() {
        // 声明 1 条，但条目头只有 8 字节
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&1i16.to_le_bytes());
        data.extend_from_slice(&4i32.to_le_bytes());
        data.extend_from_slice(&37748744i32.to_le_bytes());

        assert!(Props9::new(&data).is_err());
    }

    #[test]
    fn missing_keys_return_defaults_and_display_works() {
        let props = Props9::new(&build_props9(0, &[])).expect("应能解析");

        assert_eq!(props.get_byte(PropsKey::Company), 0);
        assert_eq!(props.get_short(PropsKey::Company), 0);
        assert_eq!(props.get_int(PropsKey::Company), 0);
        assert_eq!(props.get_double(PropsKey::Company), 0.0);
        assert_eq!(props.get_time(PropsKey::Company), None);
        assert_eq!(props.get_timestamp(PropsKey::Company), None);
        assert!(!props.get_boolean(PropsKey::Company));
        assert_eq!(props.get_unicode_string(PropsKey::Company), None);
        assert_eq!(props.get_uuid(PropsKey::Company), None);
        assert_eq!(props.get_byte_array(PropsKey::Company), None);
        assert!(props.key_set().is_empty());
        assert!(props.to_string().starts_with("BEGIN Props\n"));
    }
}
