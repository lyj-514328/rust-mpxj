//! 对应 Java: `org/mpxj/mpp/Props.java`
//!
//! `Props` 描述 MPP 文件中的 `Props*` 流：它是一张「整数键 → 字节数组」的属性表，
//! 由各版本的子类按自己的字节布局填充，再由上层（`ProjectPropertiesReader`、
//! `DocumentInputStreamFactory` 等）按键取值。
//!
//! # 继承的还原（组合 + 转发）
//!
//! Java: `class Props extends MPPComponent`，且 `Props8/Props9/Props12/Props14/PropsBlock
//! extends Props`。Rust 无继承，故：
//!
//! - `MPPComponent`（只提供基于 `InputStream` 的 `readByte` / `readShort` / `readInt` /
//!   `readByteArray`）按本工程约定把 `InputStream` 换成 `&[u8]`，已由
//!   [`crate::mpp::mpp_component::MPPComponent`] 落地；本文件的 [`InputStreamCursor`]
//!   在提供「游标」便利的同时，其 `read_int` / `read_short` / `read` 直接转发给它，
//!   不再复刻一份字节解析逻辑。[`Props`] 自身不需要父类（Java 的
//!   `extends MPPComponent` 只是复用 4 个读取助手）。
//! - `Props8/9/12/14/PropsBlock` 各自**持有一个 [`Props`]**（私有字段 `props`），并把它
//!   的全部公开方法原样转发；`Props8/9/12/14` 另实现 `Deref`/`DerefMut`（目标为 [`Props`]），
//!   使 `&Props9` 之类的引用能自动强转成 `&Props` —— 与 Java 的继承（子类实例可直接传给
//!   基类形参）等价，手法同 [`crate::recurring_task`]。
//!
//! # 本类与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `byte[] getByteArray(PropsKey)` | `Option<&[u8]>` | 返回内部缓冲的借用，不复制；`null` → `None` |
//! | `byte getByte(PropsKey)` | `i8` | Java 的 `byte` 有符号；属性为空数组时 Java 抛 `AIOOBE`，Rust 返回 0 |
//! | `int getShort/getInt` | `i32` | 属性长度不足时 Java 抛 `AIOOBE`，Rust 返回 0 |
//! | `double getDouble` | `f64` | 同上 |
//! | `LocalTime getTime` | `Option<NaiveTime>` | 同上，返回 `None` |
//! | `LocalDateTime getTimestamp` | `Option<NaiveDateTime>` | 同上，返回 `None` |
//! | `UUID getUUID` | `Option<Uuid>` | MPP9 的字符串形式解析失败时 Java 抛 `IllegalArgumentException`，Rust 返回 `None` |
//! | `Set<Integer> keySet()` | `Vec<i32>` | 按 `TreeMap` 迭代顺序（升序）返回键的副本 |
//! | `toString()` | `Display` | `PrintWriter.println` 的平台换行统一为 `\n` |
//! | `protected final TreeMap<Integer, byte[]> m_map` | 私有字段 `map` + [`Props::put`] / `get_byte_array_by_key` / `entries` | Java 的子类直接读写该字段；Rust 侧由本类提供三个 `pub(crate)` 读写入口（唯一被改动的可见性） |
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
//!
//! # 对相关模块 `crate::mpp::mpp_utility::MPPUtility` 的接口假设（已核对一致）
//!
//! ```text
//! MPPUtility::get_double(&[u8], usize) -> f64                                // Java getDouble
//! MPPUtility::get_time(&[u8], usize) -> NaiveTime                            // Java getTime
//! MPPUtility::get_timestamp(&[u8], usize) -> Option<NaiveDateTime>           // Java getTimestamp
//! MPPUtility::get_unicode_string(&[u8], usize) -> String                     // Java getUnicodeString
//! MPPUtility::get_unicode_string_with_max_length(&[u8], usize, usize) -> String
//!                                                                            // Java getUnicodeString(data, offset, maxLength)
//! MPPUtility::get_guid(&[u8], usize) -> Option<Uuid>                         // Java getGUID
//! ```

use std::collections::BTreeMap;
use std::fmt;

use chrono::{NaiveDateTime, NaiveTime};
use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::mpp_component::MPPComponent;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::props_key::PropsKey;

/// 属性表：整数键 → 字节数组。
///
/// 对应 Java: `class Props extends MPPComponent`
///
/// Rust 特有：Java 没有显式构造方法（默认构造 + 子类填充 `m_map`），此处提供
/// [`Props::new`]。
#[derive(Debug, Default)]
pub struct Props {
    /// 对应 Java: `protected final TreeMap<Integer, byte[]> m_map`
    ///
    /// 用 `BTreeMap` 与 Java 的 `TreeMap` 对应（键升序）。
    map: BTreeMap<i32, Vec<u8>>,
}

impl Props {
    /// 构造空的属性表。
    ///
    /// Java 侧无对应方法（隐式默认构造）。
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    /// 写入一条属性。
    ///
    /// 对应 Java: 子类对 `m_map.put(Integer, byte[])` 的调用（例如 `Props8`、`PropsBlock`）
    pub fn put(&mut self, key: i32, value: Vec<u8>) {
        self.map.insert(key, value);
    }

    /// 按另一份属性表的内容重建一份等价的 [`Props`]。
    ///
    /// Rust 特有：Java 的 `Props9/12/14 extends Props`，`ViewStateReader.getProps` 直接返回
    /// 子类实例、随后按基类引用使用；本 crate 的子类用组合实现「继承」（见文件头），`Deref`
    /// 只能给出借用、无法按值取出内部的 [`Props`]，故「需要拥有所有权的 [`Props`]」的场景
    /// 在此按键值重建一份（内容与来源完全一致，含 `PropsKey` 未覆盖的键）。
    pub(crate) fn copied_from(source: &Props) -> Props {
        Props {
            map: source.map.clone(),
        }
    }

    /// 取属性数据（原始字节）。
    ///
    /// 对应 Java: `public byte[] getByteArray(PropsKey key)`
    pub fn get_byte_array(&self, key: PropsKey) -> Option<&[u8]> {
        self.get_byte_array_by_key(key.get_value())
    }

    /// 按整数键取属性数据。
    ///
    /// Rust 特有：Java 的子类直接读 protected 字段 `m_map`
    /// （例如 `PropsBlock.getByteArray(Integer)`），Rust 侧改用本方法暴露读取入口。
    pub(crate) fn get_byte_array_by_key(&self, key: i32) -> Option<&[u8]> {
        self.map.get(&key).map(Vec::as_slice)
    }

    /// 按键升序遍历全部属性。
    ///
    /// Rust 特有：Java 的子类直接遍历 protected 字段 `m_map`（例如 `Props8.toString()`）。
    pub(crate) fn entries(&self) -> impl Iterator<Item = (&i32, &Vec<u8>)> {
        self.map.iter()
    }

    /// 取字节值。
    ///
    /// 对应 Java: `public byte getByte(PropsKey key)`
    ///
    /// Java 返回有符号 `byte`，Rust 用 `i8` 对应；属性不存在时返回 0，
    /// 属性为空数组时 Java 抛 `ArrayIndexOutOfBoundsException`，Rust 同样返回 0。
    pub fn get_byte(&self, key: PropsKey) -> i8 {
        value_byte(self.get_byte_array(key))
    }

    /// 取短整型值。
    ///
    /// 对应 Java: `public int getShort(PropsKey key)`
    pub fn get_short(&self, key: PropsKey) -> i32 {
        value_short(self.get_byte_array(key))
    }

    /// 取整型值。
    ///
    /// 对应 Java: `public int getInt(PropsKey key)`
    pub fn get_int(&self, key: PropsKey) -> i32 {
        value_int(self.get_byte_array(key))
    }

    /// 取双精度值。
    ///
    /// 对应 Java: `public double getDouble(PropsKey key)`
    pub fn get_double(&self, key: PropsKey) -> f64 {
        value_double(self.get_byte_array(key))
    }

    /// 取时间值（当日时刻）。
    ///
    /// 对应 Java: `public LocalTime getTime(PropsKey key)`
    pub fn get_time(&self, key: PropsKey) -> Option<NaiveTime> {
        value_time(self.get_byte_array(key))
    }

    /// 取时间戳。
    ///
    /// 对应 Java: `public LocalDateTime getTimestamp(PropsKey key)`
    pub fn get_timestamp(&self, key: PropsKey) -> Option<NaiveDateTime> {
        value_timestamp(self.get_byte_array(key))
    }

    /// 取布尔值（短整型是否为非零）。
    ///
    /// 对应 Java: `public boolean getBoolean(PropsKey key)`
    pub fn get_boolean(&self, key: PropsKey) -> bool {
        value_boolean(self.get_byte_array(key))
    }

    /// 取 UTF-16LE 字符串。
    ///
    /// 对应 Java: `public String getUnicodeString(PropsKey key)`
    pub fn get_unicode_string(&self, key: PropsKey) -> Option<String> {
        value_unicode_string(self.get_byte_array(key))
    }

    /// 取 GUID。
    ///
    /// 对应 Java: `public UUID getUUID(PropsKey key)`
    ///
    /// Java 在属性长度大于 16 字节时先尝试按 MPP9 的「字符串形式 GUID」解析
    /// （`{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}`，最多 76 字节）。
    pub fn get_uuid(&self, key: PropsKey) -> Option<Uuid> {
        value_uuid(self.get_byte_array(key))
    }

    /// 取全部键（升序，对应 Java `TreeMap` 的迭代顺序）。
    ///
    /// 对应 Java: `public Set<Integer> keySet()`
    pub fn key_set(&self) -> Vec<i32> {
        self.map.keys().copied().collect()
    }
}

impl fmt::Display for Props {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 用 `PrintWriter.println`，换行符为平台相关（Windows 下是 `\r\n`）；
    /// Rust 侧统一用 `\n`。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Java: 建立「键值 → PropsKey 列表」的映射（一个键值可能对应多个枚举常量）
        let mut key_labels: BTreeMap<i32, Vec<PropsKey>> = BTreeMap::new();
        for key in PropsKey::ALL {
            key_labels.entry(key.get_value()).or_default().push(key);
        }

        writeln!(f, "BEGIN Props")?;

        for (key, value) in &self.map {
            let label = match key_labels.get(key) {
                None => key.to_string(),
                Some(keys) => keys
                    .iter()
                    .map(|item| format!("{}({})", item, item.get_value()))
                    .collect::<Vec<String>>()
                    .join(","),
            };

            writeln!(f, "   Key: {label} Value: ")?;
            // Java: pw.println(ByteArrayHelper.hexdump(value, true, 16, "      "))
            write!(
                f,
                "{}",
                ByteArrayHelper::hexdump_columns(value, true, 16, "      ")
            )?;
        }

        writeln!(f, "END Props")?;
        // Java: pw.println();
        writeln!(f)?;

        Ok(())
    }
}

/// 顺序读取 `&[u8]` 的游标。
///
/// Rust 特有：Java 的 `Props8/9/12/14` 构造方法接收 `InputStream`，并用
/// `MPPComponent.readInt/readShort`、`InputStreamHelper.read/skip` 与
/// `InputStream.available()` 读取数据。本工程约定把 `InputStream` 换成 `&[u8]`，
/// 故用本结构提供一一对应的操作。
///
/// 其中 `read_int` / `read_short` / `read` 直接转发到已落地的
/// [`crate::mpp::mpp_component::MPPComponent`]（对应 `MPPComponent.readInt/readShort` 与
/// `InputStreamHelper.read`），本文件不再复刻一份等价实现；
/// `available` / `skip` 对应的是 `InputStream.available()` 与 `InputStreamHelper.skip`，
/// `MPPComponent` 中没有等价方法，故仍在本文件实现。
///
/// 越界（Java 会抛 `EOFException` / `RuntimeException`）一律返回 `None` / 不移动位置，
/// 由调用方决定是「返回 `Err`」还是「中断解析」，绝不 panic。
#[derive(Debug, Clone, Copy)]
pub(crate) struct InputStreamCursor<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> InputStreamCursor<'a> {
    /// 创建游标。
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    /// 剩余字节数。
    ///
    /// 对应 Java: `InputStream.available()`
    ///
    /// `MPPComponent` 没有等价成员，故保留在本文件。
    pub(crate) fn available(&self) -> usize {
        self.data.len().saturating_sub(self.position)
    }

    /// 读一个四字节小端整数。
    ///
    /// 对应 Java: `MPPComponent.readInt(InputStream)`（转发到
    /// [`MPPComponent::read_int`]）
    pub(crate) fn read_int(&mut self) -> Option<i32> {
        MPPComponent::read_int(self.data, &mut self.position)
    }

    /// 读一个两字节小端短整数（0..65535）。
    ///
    /// 对应 Java: `MPPComponent.readShort(InputStream)`（转发到
    /// [`MPPComponent::read_short`]）
    pub(crate) fn read_short(&mut self) -> Option<i32> {
        MPPComponent::read_short(self.data, &mut self.position)
    }

    /// 读指定长度的字节切片。
    ///
    /// 对应 Java: `InputStreamHelper.read(InputStream, int)`
    /// （转发到 [`MPPComponent::read_byte_array`]；长度不足时 Java 抛异常，Rust 返回 `None`）
    pub(crate) fn read(&mut self, size: usize) -> Option<&'a [u8]> {
        MPPComponent::read_byte_array(self.data, &mut self.position, size)
    }

    /// 跳过指定长度的字节。
    ///
    /// 对应 Java: `InputStreamHelper.skip(InputStream, long)`
    /// （Java 在无法跳过时抛 `IOException`，Rust 侧直接定位到末尾）
    ///
    /// `MPPComponent` 没有等价成员，故保留在本文件。
    pub(crate) fn skip(&mut self, size: usize) {
        self.position = (self.position + size).min(self.data.len());
    }
}

/// 读取属性对应的异常（对应 Java `InputStreamHelper.read` 抛出的
/// `RuntimeException("Unable to read the required number of bytes")`）。
pub(crate) fn read_error() -> crate::mpxj_exception::MpxjError {
    crate::mpxj_exception::MpxjError::new("Unable to read the required number of bytes")
}

/// 取字节值。[`Props::get_byte`] 与 `PropsBlock::get_byte_array_by_type` 路径共用。
pub(crate) fn value_byte(item: Option<&[u8]>) -> i8 {
    match item {
        None => 0,
        // Java 对空数组取 item[0] 会抛 AIOOBE，Rust 返回 0
        Some(item) => item.first().map_or(0, |value| *value as i8),
    }
}

/// 取短整型值。
pub(crate) fn value_short(item: Option<&[u8]>) -> i32 {
    match item {
        // Java 在数据不足时抛 AIOOBE，Rust 返回 0
        None => 0,
        Some(item) if item.len() < 2 => 0,
        Some(item) => ByteArrayHelper::get_short(item, 0),
    }
}

/// 取整型值。
pub(crate) fn value_int(item: Option<&[u8]>) -> i32 {
    match item {
        // Java 在数据不足时抛 AIOOBE，Rust 返回 0
        None => 0,
        Some(item) if item.len() < 4 => 0,
        Some(item) => ByteArrayHelper::get_int(item, 0),
    }
}

/// 取双精度值。
pub(crate) fn value_double(item: Option<&[u8]>) -> f64 {
    match item {
        // Java 在数据不足时抛 AIOOBE，Rust 返回 0
        None => 0.0,
        Some(item) if item.len() < 8 => 0.0,
        Some(item) => MPPUtility::get_double(item, 0),
    }
}

/// 取时间值（当日时刻）。
pub(crate) fn value_time(item: Option<&[u8]>) -> Option<NaiveTime> {
    let item = item?;
    // Java 在数据不足时抛 AIOOBE，Rust 返回 None
    if item.len() < 2 {
        return None;
    }

    Some(MPPUtility::get_time(item, 0))
}

/// 取时间戳。
pub(crate) fn value_timestamp(item: Option<&[u8]>) -> Option<NaiveDateTime> {
    let item = item?;
    // Java 在数据不足时抛 AIOOBE，Rust 返回 None
    if item.len() < 4 {
        return None;
    }

    MPPUtility::get_timestamp(item, 0)
}

/// 取布尔值。
pub(crate) fn value_boolean(item: Option<&[u8]>) -> bool {
    // Java: !(ByteArrayHelper.getShort(item, 0) == 0)
    value_short(item) != 0
}

/// 取 UTF-16LE 字符串。
pub(crate) fn value_unicode_string(item: Option<&[u8]>) -> Option<String> {
    let item = item?;
    Some(MPPUtility::get_unicode_string(item, 0))
}

/// 取 GUID（MPP9 会存字符串形式，最多 76 字节）。
pub(crate) fn value_uuid(item: Option<&[u8]>) -> Option<Uuid> {
    let item = item?;

    if item.len() > 16 {
        // Java: String value = MPPUtility.getUnicodeString(item, 0, 76)
        let value = MPPUtility::get_unicode_string_with_max_length(item, 0, 76);

        // Java: value.length() == 38 && value.charAt(0) == '{' && value.charAt(37) == '}'
        // 按字符（而非字节）计数，与 Java 的 char 语义一致。
        if value.chars().count() == 38 && value.starts_with('{') && value.ends_with('}') {
            // Java: UUID.fromString(value.substring(1, 37)) —— 解析失败时 Java 抛
            // IllegalArgumentException，Rust 返回 None
            let inner: String = value.chars().skip(1).take(36).collect();
            return Uuid::parse_str(&inner).ok();
        }
    }

    MPPUtility::get_guid(item, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一份包含各种类型属性的表。
    fn sample_props() -> Props {
        let mut props = Props::new();

        // 单字节
        props.put(PropsKey::Company.get_value(), vec![0x07]);
        // 两字节短整数：258
        props.put(PropsKey::DaysPerMonth.get_value(), vec![0x02, 0x01]);
        // 四字节整数：1234
        props.put(PropsKey::MinutesPerDay.get_value(), vec![0xD2, 0x04, 0x00, 0x00]);
        // 双精度：1.5
        props.put(
            PropsKey::StandardRate.get_value(),
            1.5f64.to_le_bytes().to_vec(),
        );
        // UTF-16LE 字符串 "AB"
        props.put(
            PropsKey::Manager.get_value(),
            vec![0x41, 0x00, 0x42, 0x00, 0x00, 0x00],
        );
        // 布尔：false（短整数 0）/ true（短整数 1）
        props.put(PropsKey::ScheduleFrom.get_value(), vec![0x00, 0x00]);
        props.put(PropsKey::HonorConstraints.get_value(), vec![0x01, 0x00]);
        // 时间：短整数 600 → 600/10*60 = 3600 秒 = 01:00:00
        props.put(PropsKey::StartTime.get_value(), vec![0x58, 0x02]);
        // 时间戳：首 2 字节为当日秒数/6（=10 → 60 秒），后 2 字节为天数（=2）
        // → 1983-12-31 + 2 天 = 1984-01-02 00:01:00
        props.put(
            PropsKey::StatusDate.get_value(),
            vec![0x0A, 0x00, 0x02, 0x00],
        );

        props
    }

    #[test]
    fn reads_scalar_values() {
        let props = sample_props();

        assert_eq!(props.get_byte(PropsKey::Company), 7);
        assert_eq!(props.get_short(PropsKey::DaysPerMonth), 258);
        assert_eq!(props.get_int(PropsKey::MinutesPerDay), 1234);
        assert_eq!(props.get_double(PropsKey::StandardRate), 1.5);
        assert_eq!(
            props.get_unicode_string(PropsKey::Manager),
            Some("AB".to_owned())
        );
        assert!(!props.get_boolean(PropsKey::ScheduleFrom));
        assert!(props.get_boolean(PropsKey::HonorConstraints));
        assert_eq!(
            props.get_time(PropsKey::StartTime),
            Some(NaiveTime::from_hms_opt(1, 0, 0).expect("合法时间"))
        );
        assert_eq!(
            props.get_timestamp(PropsKey::StatusDate),
            Some(
                chrono::NaiveDate::from_ymd_opt(1984, 1, 2)
                    .expect("合法日期")
                    .and_hms_opt(0, 1, 0)
                    .expect("合法时间")
            )
        );
    }

    #[test]
    fn reads_byte_array_and_key_set() {
        let props = sample_props();

        assert_eq!(props.get_byte_array(PropsKey::Company), Some(&[0x07][..]));
        assert!(props.get_byte_array(PropsKey::Category).is_none());

        let keys = props.key_set();
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(keys, sorted, "key_set 应按 key 升序返回");
        assert_eq!(keys.len(), 9);
    }

    #[test]
    fn missing_keys_return_defaults() {
        let props = Props::new();

        assert_eq!(props.get_byte(PropsKey::Company), 0);
        assert_eq!(props.get_short(PropsKey::Company), 0);
        assert_eq!(props.get_int(PropsKey::Company), 0);
        assert_eq!(props.get_double(PropsKey::Company), 0.0);
        assert_eq!(props.get_time(PropsKey::Company), None);
        assert_eq!(props.get_timestamp(PropsKey::Company), None);
        assert!(!props.get_boolean(PropsKey::Company));
        assert_eq!(props.get_unicode_string(PropsKey::Company), None);
        assert_eq!(props.get_uuid(PropsKey::Company), None);
        assert!(props.key_set().is_empty());
    }

    #[test]
    fn short_values_do_not_panic() {
        let mut props = Props::new();
        // 空数组与长度不足的数组：Java 会抛 AIOOBE，Rust 必须返回默认值
        props.put(PropsKey::Company.get_value(), Vec::new());
        props.put(PropsKey::Manager.get_value(), vec![0x01]);

        assert_eq!(props.get_byte(PropsKey::Company), 0);
        assert_eq!(props.get_short(PropsKey::Company), 0);
        assert_eq!(props.get_int(PropsKey::Company), 0);
        assert_eq!(props.get_double(PropsKey::Company), 0.0);
        assert_eq!(props.get_time(PropsKey::Company), None);
        assert_eq!(props.get_timestamp(PropsKey::Company), None);
        assert!(!props.get_boolean(PropsKey::Company));
        assert_eq!(props.get_short(PropsKey::Manager), 0);
        assert_eq!(props.get_int(PropsKey::Manager), 0);
        assert_eq!(props.get_timestamp(PropsKey::Manager), None);
    }

    #[test]
    fn byte_is_signed() {
        let mut props = Props::new();
        props.put(PropsKey::Company.get_value(), vec![0xFF]);

        // Java: byte 有符号，(byte) 0xFF == -1
        assert_eq!(props.get_byte(PropsKey::Company), -1);
    }

    #[test]
    fn uuid_from_binary_and_from_string() {
        let mut props = Props::new();

        // 16 字节二进制形式（Java 的 getGUID 要求低 8 字节非全零，否则返回 null）
        let mut binary = vec![0u8; 16];
        binary[15] = 1;
        props.put(PropsKey::Guid.get_value(), binary);
        let uuid = props.get_uuid(PropsKey::Guid).expect("应能解析二进制 GUID");
        assert_eq!(uuid.as_bytes()[15], 1);

        // 全零的二进制形式（Java: long1 == 0 && long2 == 0 → null）
        props.put(PropsKey::Comments.get_value(), vec![0u8; 16]);
        assert_eq!(props.get_uuid(PropsKey::Comments), None);

        // MPP9 的字符串形式（76 字节，UTF-16LE，"{" + 36 字符 + "}"）
        let text = "{01234567-89AB-CDEF-0123-456789ABCDEF}";
        assert_eq!(text.chars().count(), 38);
        let mut bytes = Vec::new();
        for ch in text.encode_utf16() {
            bytes.extend_from_slice(&ch.to_le_bytes());
        }
        bytes.extend_from_slice(&[0x00, 0x00]);
        props.put(PropsKey::Category.get_value(), bytes);
        assert_eq!(
            props.get_uuid(PropsKey::Category),
            Some(Uuid::parse_str("01234567-89AB-CDEF-0123-456789ABCDEF").expect("合法 GUID"))
        );

        // 长度 38 且带花括号、但内容不是合法 GUID：Java 抛 IllegalArgumentException，
        // Rust 返回 None（不 panic）
        let text = format!("{{{}}}", "x".repeat(36));
        assert_eq!(text.chars().count(), 38);
        let mut bytes = Vec::new();
        for ch in text.encode_utf16() {
            bytes.extend_from_slice(&ch.to_le_bytes());
        }
        props.put(PropsKey::Title.get_value(), bytes);
        assert_eq!(props.get_uuid(PropsKey::Title), None);
    }

    #[test]
    fn display_matches_java_layout() {
        let mut props = Props::new();
        props.put(PropsKey::Company.get_value(), vec![0xAB, 0xCD]);
        // 未知键（不在 PropsKey 中）应按十进制输出
        props.put(99, vec![0x00]);

        let text = props.to_string();
        assert!(text.starts_with("BEGIN Props\n"));
        assert!(text.contains("   Key: COMPANY(37748736) Value: \n"));
        assert!(text.contains("     00000: AB CD"));
        assert!(text.contains("   Key: 99 Value: \n"));
        assert!(text.ends_with("END Props\n\n"));
    }

    #[test]
    fn duplicate_key_values_are_all_listed() {
        let mut props = Props::new();
        // 574619658 同时对应 TABLE_NAME 与 UPPER_VIEW_NAME
        props.put(574619658, vec![0x01]);

        let text = props.to_string();
        assert!(text.contains("TABLE_NAME(574619658),UPPER_VIEW_NAME(574619658)"));
    }

    #[test]
    fn cursor_reads_and_stops_at_end() {
        let data = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        let mut cursor = InputStreamCursor::new(&data);

        assert_eq!(cursor.available(), 7);
        assert_eq!(cursor.read_int(), Some(0x04030201));
        assert_eq!(cursor.read_short(), Some(0x0605));
        assert_eq!(cursor.read(1), Some(&[0x07][..]));
        assert_eq!(cursor.available(), 0);

        // 越界返回 None 且不 panic
        assert_eq!(cursor.read(1), None);
        assert_eq!(cursor.read_int(), None);
        cursor.skip(10);
        assert_eq!(cursor.available(), 0);
    }
}
