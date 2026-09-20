//! 对应 Java: `org/mpxj/mpp/Props12.java`

use std::fmt;

use chrono::{NaiveDateTime, NaiveTime};
use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::props::{InputStreamCursor, Props, read_error};
use crate::mpp::props_key::PropsKey;
use crate::mpxj_exception::MpxjResult;

/// MPP12 文件的 `Props` 流。
///
/// 对应 Java: `final class Props12 extends Props`
///
/// # 继承的还原（组合 + `Deref` + 转发）
///
/// Java 的 `Props12` 继承 [`Props`]（Java 侧未覆写 `toString()`）。Rust 无继承，本类用
/// **组合 + `Deref`/`DerefMut`** 还原：持有一个 [`Props`]（字段 `props`），于是 `&Props12`
/// 能自动强转（deref coercion）成 `&Props` —— 「形参是 `&Props` 的方法」也可以直接接收
/// `&Props12`，对应 Java 里「子类实例可直接传给基类形参」（本 crate 已在
/// [`crate::recurring_task`] 用同一手法还原继承）。
///
/// 类末尾仍保留 `Props` 全部公开方法的显式转发，供不需要强转的调用点直接调用；这些同名方法
/// 会遮蔽 `Deref` 目标 [`Props`] 上的同名方法，而它们本身就是逐方法转发、行为完全一致，
/// 因此不存在语义冲突。
///
/// # 构造方法与 `InputStream`
///
/// Java: `Props12(InputStream is) throws IOException`。Rust 侧按本工程约定把
/// `InputStream` 换成 `&[u8]`；Java 的 `IOException`（读取不足）对应 Rust 的
/// `Err(MpxjError)`。
///
/// # 块布局（Java 构造方法）
///
/// ```text
/// byte[16] header      headerCount = 该头部的 short（偏移 12）—— 条目数
/// headerCount × {
///     int  attrib1     属性数据长度
///     int  attrib2     属性键
///     int  unknown
///     byte data[attrib1]
///     byte pad         仅当 attrib1 为奇数时补齐到两字节边界
/// }
/// ```
///
/// 与 `Props9` 不同，`Props12` 不做 `available()` 预检，直接读 16 字节头部；
/// 循环中出现 `availableBytes < attrib1 || attrib1 < 1` 时中断（不抛异常）。
///
/// # 与 Java 的差异
///
/// - `availableBytes` 用 `i64` 累减（Java 是 `int`，极端输入下会溢出）。
/// - `toString()` 沿用 [`Props`] 的实现（Java 未覆写）。
///
/// 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
pub struct Props12 {
    /// 对应 Java: `extends Props`（Rust 用组合代替继承）
    props: Props,
}

impl Props12 {
    /// 构造方法。
    ///
    /// 对应 Java: `Props12(InputStream is) throws IOException`
    pub fn new(data: &[u8]) -> MpxjResult<Self> {
        let mut result = Self { props: Props::new() };
        let mut is = InputStreamCursor::new(data);

        // Java: byte[] header = InputStreamHelper.read(is, 16)
        let header = is.read(16).ok_or_else(read_error)?;
        // Java: int headerCount = ByteArrayHelper.getShort(header, 12)
        let header_count = ByteArrayHelper::get_short(header, 12);
        let mut found_count = 0;
        // Java: int availableBytes = is.available()
        let mut available_bytes = is.available() as i64;

        while found_count < header_count {
            // Java: int attrib1 = readInt(is)
            let attrib1 = is.read_int().ok_or_else(read_error)?;
            // Java: int attrib2 = readInt(is)
            let attrib2 = is.read_int().ok_or_else(read_error)?;
            // Java: /*int attrib3 = */ readInt(is)
            is.read_int().ok_or_else(read_error)?;
            available_bytes -= 12;

            if available_bytes < i64::from(attrib1) || attrib1 < 1 {
                break;
            }

            // Java: data = InputStreamHelper.read(is, attrib1)
            let item = is.read(attrib1 as usize).ok_or_else(read_error)?;
            available_bytes -= i64::from(attrib1);

            result.props.put(attrib2, item.to_vec());
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

impl fmt::Display for Props12 {
    /// 对应 Java: `Props.toString()`（`Props12` 未覆写）
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.props, f)
    }
}

impl std::ops::Deref for Props12 {
    type Target = Props;

    /// 对应 Java: 继承关系带来的「子类实例可直接当作基类使用」
    fn deref(&self) -> &Self::Target {
        &self.props
    }
}

impl std::ops::DerefMut for Props12 {
    /// 对应 Java: 继承关系带来的「子类实例可直接当作基类使用」
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.props
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 按 MPP12 的块布局构造数据。
    fn build_props12(header_count: i32, entries: &[(i32, i32, Vec<u8>)]) -> Vec<u8> {
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
        let data = build_props12(
            2,
            &[
                // PropsKey.TITLE == 37748744
                (4, 37748744, vec![0xD2, 0x04, 0x00, 0x00]),
                // PropsKey.SUBJECT == 37748745
                (2, 37748745, vec![0x02, 0x01]),
            ],
        );

        let props = Props12::new(&data).expect("应能解析");
        assert_eq!(props.key_set(), vec![37748744, 37748745]);
        assert_eq!(props.get_int(PropsKey::Title), 1234);
        assert_eq!(props.get_short(PropsKey::Subject), 258);
    }

    #[test]
    fn odd_length_data_is_aligned() {
        let data = build_props12(
            2,
            &[(1, 37748744, vec![0x07]), (2, 37748745, vec![0x02, 0x01])],
        );

        let props = Props12::new(&data).expect("应能解析");
        assert_eq!(props.get_byte(PropsKey::Title), 7);
        assert_eq!(props.get_short(PropsKey::Subject), 258);
    }

    #[test]
    fn short_header_returns_error() {
        // Java: InputStreamHelper.read(is, 16) 在数据不足时抛异常
        assert!(Props12::new(&[0u8; 15]).is_err());
    }

    #[test]
    fn invalid_size_breaks_loop() {
        let data = build_props12(
            2,
            &[
                (2, 37748744, vec![0x02, 0x01]),
                (0, 37748745, Vec::new()),
            ],
        );

        let props = Props12::new(&data).expect("应能解析");
        assert_eq!(props.key_set(), vec![37748744]);
    }

    #[test]
    fn size_beyond_available_bytes_breaks_loop_without_error() {
        let mut data = build_props12(2, &[(2, 37748744, vec![0x02, 0x01])]);
        data.extend_from_slice(&100i32.to_le_bytes());
        data.extend_from_slice(&37748745i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());

        let props = Props12::new(&data).expect("应能解析");
        assert_eq!(props.key_set(), vec![37748744]);
    }

    #[test]
    fn truncated_entry_returns_error() {
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&1i16.to_le_bytes());
        data.extend_from_slice(&4i32.to_le_bytes());
        data.extend_from_slice(&37748744i32.to_le_bytes());

        assert!(Props12::new(&data).is_err());
    }

    #[test]
    fn missing_keys_return_defaults_and_display_works() {
        let props = Props12::new(&build_props12(0, &[])).expect("应能解析");

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
