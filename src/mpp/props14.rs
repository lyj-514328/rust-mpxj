//! 对应 Java: `org/mpxj/mpp/Props14.java`

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use chrono::{NaiveDateTime, NaiveTime};
use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::props::{InputStreamCursor, Props, read_error};
use crate::mpp::props_key::PropsKey;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_file::ProjectFile;

/// MPP14 文件的 `Props` 流。
///
/// 对应 Java: `final class Props14 extends Props`
///
/// # 继承的还原（组合 + `Deref` + 转发）
///
/// Java 的 `Props14` 继承 [`Props`]（Java 侧未覆写 `toString()`）。Rust 无继承，本类用
/// **组合 + `Deref`/`DerefMut`** 还原：持有一个 [`Props`]（字段 `props`），于是 `&Props14`
/// 能自动强转（deref coercion）成 `&Props` —— 「形参是 `&Props` 的方法」也可以直接接收
/// `&Props14`，对应 Java 里「子类实例可直接传给基类形参」（本 crate 已在
/// [`crate::recurring_task`] 用同一手法还原继承）。
///
/// 类末尾仍保留 `Props` 全部公开方法的显式转发，供不需要强转的调用点直接调用；这些同名方法
/// 会遮蔽 `Deref` 目标 [`Props`] 上的同名方法，而它们本身就是逐方法转发、行为完全一致，
/// 因此不存在语义冲突。
///
/// # 构造方法与 `InputStream`
///
/// Java: `Props14(ProjectFile file, InputStream is) throws IOException`。Rust 侧按本工程
/// 约定把 `InputStream` 换成 `&[u8]`，顺序读取由 [`InputStreamCursor`] 完成。
/// `ProjectFile` 仅用于出错时登记被忽略的错误（`addIgnoredError`），**不持有**，
/// 故用 `&Rc<RefCell<ProjectFile>>` 借用传参，不会造成 `Rc` 循环。
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
/// 与 `Props12` 的两处差异（Java 原文保留）：
///
/// 1. 循环开头检查「剩余不足 12 字节就退出」—— 这是 Java 侧针对 64 位 JRE 下
///    `DocumentInputStream` 行为的修复；
/// 2. 读取数据时捕获 `IndexOutOfBoundsException`（POI bug 61677），
///    登记被忽略的错误后中断解析，而不是向上抛异常。
///
/// # 与 Java 的差异
///
/// - `availableBytes` 用 `i64` 累减（Java 是 `int`，极端输入下会溢出）。
/// - `availableBytes` 与 Java 一样**不扣减对齐填充字节**，因此可能大于真实剩余字节数；
///   这正是 Java 需要捕获 `IndexOutOfBoundsException` 的原因，Rust 侧同样保留该分支。
/// - POI 的 `IndexOutOfBoundsException` 对应 Rust 的「游标读不到足够字节」，
///   此时同样登记 `file.addIgnoredError(..)` 并中断循环。
/// - `toString()` 沿用 [`Props`] 的实现（Java 未覆写）。
///
/// 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
pub struct Props14 {
    /// 对应 Java: `extends Props`（Rust 用组合代替继承）
    props: Props,
}

impl Props14 {
    /// 构造方法。
    ///
    /// 对应 Java: `Props14(ProjectFile file, InputStream is) throws IOException`
    pub fn new(file: &Rc<RefCell<ProjectFile>>, data: &[u8]) -> MpxjResult<Self> {
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
            // Java: 剩余不足 12 字节就退出（64 位 JRE 下的修复）
            if available_bytes < 12 {
                break;
            }

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

            // Java: try { data = InputStreamHelper.read(is, attrib1); }
            //       catch (IndexOutOfBoundsException ex) { file.addIgnoredError(ex); break; }
            let item = match is.read(attrib1 as usize) {
                Some(item) => item,
                None => {
                    file.borrow_mut()
                        .add_ignored_error(index_out_of_bounds_error());
                    break;
                }
            };
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

impl fmt::Display for Props14 {
    /// 对应 Java: `Props.toString()`（`Props14` 未覆写）
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.props, f)
    }
}

impl std::ops::Deref for Props14 {
    type Target = Props;

    /// 对应 Java: 继承关系带来的「子类实例可直接当作基类使用」
    fn deref(&self) -> &Self::Target {
        &self.props
    }
}

impl std::ops::DerefMut for Props14 {
    /// 对应 Java: 继承关系带来的「子类实例可直接当作基类使用」
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.props
    }
}

/// POI 读取越界时抛出的 `IndexOutOfBoundsException` 的 Rust 对应物。
///
/// 对应 Java: `catch (IndexOutOfBoundsException ex)`
/// （POI bug: https://bz.apache.org/bugzilla/show_bug.cgi?id=61677）
fn index_out_of_bounds_error() -> MpxjError {
    MpxjError::new("Index out of bounds while reading Props data")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 按 MPP14 的块布局构造数据。
    fn build_props14(header_count: i32, entries: &[(i32, i32, Vec<u8>)]) -> Vec<u8> {
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
        let data = build_props14(
            2,
            &[
                // PropsKey.TITLE == 37748744
                (4, 37748744, vec![0xD2, 0x04, 0x00, 0x00]),
                // PropsKey.SUBJECT == 37748745
                (2, 37748745, vec![0x02, 0x01]),
            ],
        );

        let file = ProjectFile::new();
        let props = Props14::new(&file, &data).expect("应能解析");

        assert_eq!(props.key_set(), vec![37748744, 37748745]);
        assert_eq!(props.get_int(PropsKey::Title), 1234);
        assert_eq!(props.get_short(PropsKey::Subject), 258);
        assert!(file.borrow().get_ignored_errors().is_empty());
    }

    #[test]
    fn odd_length_data_is_aligned() {
        let data = build_props14(
            2,
            &[(1, 37748744, vec![0x07]), (2, 37748745, vec![0x02, 0x01])],
        );

        let file = ProjectFile::new();
        let props = Props14::new(&file, &data).expect("应能解析");

        assert_eq!(props.get_byte(PropsKey::Title), 7);
        assert_eq!(props.get_short(PropsKey::Subject), 258);
    }

    #[test]
    fn short_header_returns_error() {
        let file = ProjectFile::new();
        assert!(Props14::new(&file, &[0u8; 15]).is_err());
    }

    #[test]
    fn fewer_than_twelve_remaining_bytes_breaks_loop() {
        // 声明 1 条，但头部之后只剩 8 字节 → Java: availableBytes < 12 → break
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&1i16.to_le_bytes());
        data.extend_from_slice(&[0u8; 8]);

        let file = ProjectFile::new();
        let props = Props14::new(&file, &data).expect("应能解析");

        assert!(props.key_set().is_empty());
        assert!(file.borrow().get_ignored_errors().is_empty());
    }

    #[test]
    fn size_beyond_available_bytes_breaks_loop() {
        let mut data = build_props14(2, &[(2, 37748744, vec![0x02, 0x01])]);
        data.extend_from_slice(&100i32.to_le_bytes());
        data.extend_from_slice(&37748745i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());

        let file = ProjectFile::new();
        let props = Props14::new(&file, &data).expect("应能解析");

        assert_eq!(props.key_set(), vec![37748744]);
        assert!(file.borrow().get_ignored_errors().is_empty());
    }

    #[test]
    fn truncated_data_records_ignored_error() {
        // Java 的 availableBytes 不扣减对齐填充字节，因此会「多报」剩余量，
        // 只有在前面出现过奇数长度条目时，守卫才可能放过一条实际读不到的数据。
        // 这里前面两条各 1 字节（各填充 1 字节），第 3 条声明 4 字节而实际只有 2 字节：
        // → Java: catch (IndexOutOfBoundsException ex) { file.addIgnoredError(ex); break; }
        let data = build_props14(
            3,
            &[
                (1, 37748744, vec![0x07]),
                (1, 37748745, vec![0x09]),
                (4, 37748748, vec![0x01, 0x02]),
            ],
        );

        let file = ProjectFile::new();
        let props = Props14::new(&file, &data).expect("应能解析");

        assert_eq!(props.key_set(), vec![37748744, 37748745]);
        let borrowed = file.borrow();
        assert_eq!(borrowed.get_ignored_errors().len(), 1);
        assert_eq!(
            borrowed.get_ignored_errors()[0].message(),
            "Index out of bounds while reading Props data"
        );
    }

    #[test]
    fn truncated_entry_header_returns_error() {
        // 头部之后只剩 8 字节 → Java 的 `availableBytes < 12` 守卫会先 break，
        // 因此这里不会出错
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&1i16.to_le_bytes());
        data.extend_from_slice(&4i32.to_le_bytes());
        data.extend_from_slice(&37748744i32.to_le_bytes());

        let file = ProjectFile::new();
        let props = Props14::new(&file, &data).expect("应能解析");
        assert!(props.key_set().is_empty());
    }

    #[test]
    fn truncated_entry_after_padding_returns_error() {
        // `availableBytes` 不扣减对齐填充字节，12 个奇数长度条目之后它比真实剩余量多 12，
        // 于是第 13 条能越过 `availableBytes < 12` 的守卫，但实际读取必然失败
        // （Java: POI 在此抛 IOException → Rust 返回 Err）
        let entries: Vec<(i32, i32, Vec<u8>)> = (0..12)
            .map(|index| (1, 37748744 + index, vec![0x07]))
            .collect();
        let data = build_props14(13, &entries);

        let file = ProjectFile::new();
        assert!(Props14::new(&file, &data).is_err());
    }

    #[test]
    fn missing_keys_return_defaults_and_display_works() {
        let file = ProjectFile::new();
        let props = Props14::new(&file, &build_props14(0, &[])).expect("应能解析");

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
