//! 对应 Java: `org/mpxj/mpp/Props8.java`

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use chrono::{NaiveDateTime, NaiveTime};
use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::props::{InputStreamCursor, Props, read_error};
use crate::mpp::props_key::PropsKey;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;

/// MPP8 文件的 `Props` 流。
///
/// 对应 Java: `final class Props8 extends Props`
///
/// MPP8 的 `Props` 是一组以整数为键的属性；其格式并未完全解析清楚，因此 Java 侧把整个
/// 解析过程包在 `try` 中：一旦读取出错就记录 `addIgnoredError` 并把 `m_complete`
/// 置为 `false`，已经读到的属性仍然保留。
///
/// # 继承的还原（组合 + `Deref` + 转发）
///
/// Java 的 `Props8` 继承 [`Props`]，Rust 无继承，本类用**组合 + `Deref`/`DerefMut`** 还原：
/// 持有一个 [`Props`]（字段 `props`），于是 `&Props8` 能自动强转（deref coercion）成
/// `&Props` —— 「形参是 `&Props` 的方法」也可以直接接收 `&Props8`，对应 Java 里「子类实例
/// 可直接传给基类形参」（本 crate 已在 [`crate::recurring_task`] 用同一手法还原继承）。
///
/// 类末尾仍保留 `Props` 全部公开方法的显式转发，供不需要强转的调用点直接调用；这些同名方法
/// 会遮蔽 `Deref` 目标 [`Props`] 上的同名方法，而它们本身就是逐方法转发、行为完全一致，
/// 因此不存在语义冲突。
///
/// # 构造方法与 `InputStream`
///
/// Java: `Props8(ProjectFile file, InputStream is)`。Rust 侧按本工程约定把
/// `InputStream` 换成 `&[u8]`，顺序读取由 [`InputStreamCursor`] 完成。
/// `ProjectFile` 仅用于出错时登记被忽略的错误（`addIgnoredError`），**不持有**，
/// 故用 `&Rc<RefCell<ProjectFile>>` 借用传参，不会造成 `Rc` 循环。
///
/// # 块布局（Java 构造方法）
///
/// ```text
/// int   file size
/// int   file size（重复）
/// int   unknown
/// short count              条目数
/// short unknown
/// count × {
///     int   attrib1        长度候选（attrib3 == 64 时作为长度）
///     byte  attrib[0..4]   attrib2 = 该 4 字节的小端整数；attrib3 = attrib[2]
///     int   attrib5        长度候选（attrib3 != 64 时作为长度；等于 65536 时长度固定为 4）
///     byte  data[size]     属性数据
///     byte  pad            仅当 data 长度为奇数时补齐到两字节边界
/// }
/// ```
///
/// # 与 Java 的差异
///
/// - Java 的 `IOException` 分支（`file.addIgnoredError(ex); m_complete = false;`）由
///   Rust 的「游标越界返回 `None`」触发，因此构造方法不返回 `Result`（与 Java 一致，
///   异常在内部被吞掉）。
/// - `toString()` 里 `PrintWriter.println` 的平台换行统一为 `\n`。
///
/// 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
pub struct Props8 {
    /// 对应 Java: `extends Props`（Rust 用组合代替继承）
    props: Props,
    /// 对应 Java: `private boolean m_complete = true`
    complete: bool,
}

impl Props8 {
    /// 构造方法。
    ///
    /// 对应 Java: `Props8(ProjectFile file, InputStream is)`
    pub fn new(file: &Rc<RefCell<ProjectFile>>, data: &[u8]) -> Self {
        let mut result = Self {
            props: Props::new(),
            complete: true,
        };

        if let Err(error) = result.parse(data) {
            // Java: catch (IOException ex) { file.addIgnoredError(ex); m_complete = false; }
            file.borrow_mut().add_ignored_error(error);
            result.complete = false;
        }

        result
    }

    /// 属性数据是否已完整读取。
    ///
    /// Java 侧 `m_complete` 是私有字段，只在 `toString()` 中体现；Rust 侧同样不对外暴露。
    #[cfg(test)]
    fn is_complete(&self) -> bool {
        self.complete
    }

    /// 解析属性数据（对应 Java 构造方法 `try` 块内的全部逻辑）。
    fn parse(&mut self, data: &[u8]) -> MpxjResult<()> {
        let mut is = InputStreamCursor::new(data);

        // Java: readInt(is) —— File size
        is.read_int().ok_or_else(read_error)?;
        // Java: readInt(is) —— Repeat of file size
        is.read_int().ok_or_else(read_error)?;
        // Java: readInt(is) —— unknown
        is.read_int().ok_or_else(read_error)?;
        // Java: int count = readShort(is) —— Number of entries
        let count = is.read_short().ok_or_else(read_error)?;
        // Java: readShort(is) —— unknown
        is.read_short().ok_or_else(read_error)?;

        for _ in 0..count {
            // Java: int attrib1 = readInt(is)
            let attrib1 = is.read_int().ok_or_else(read_error)?;

            // Java: InputStreamHelper.read(is, attrib)
            let attrib = is.read(4).ok_or_else(read_error)?;
            // Java: int attrib2 = ByteArrayHelper.getInt(attrib, 0)
            let attrib2 = ByteArrayHelper::get_int(attrib, 0);
            // Java: int attrib3 = MPPUtility.getByte(attrib, 2) —— 无符号取值
            let attrib3 = i32::from(attrib[2]);
            // Java: int attrib5 = readInt(is)
            let attrib5 = is.read_int().ok_or_else(read_error)?;

            let mut size = if attrib3 == 64 {
                attrib1
            } else {
                attrib5
            };

            if attrib5 == 65536 {
                size = 4;
            }

            if size <= 0 {
                // Java: // bail out here as we don't understand the structure
                //       m_complete = false; break;
                self.complete = false;
                break;
            }

            // Java: data = InputStreamHelper.read(is, size)
            let item = is.read(size as usize).ok_or_else(read_error)?;
            self.props.put(attrib2, item.to_vec());

            // Java: 对齐到两字节边界
            if item.len() % 2 != 0 {
                is.skip(1);
            }
        }

        Ok(())
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

impl fmt::Display for Props8 {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 用 `PrintWriter.println`，换行符为平台相关（Windows 下是 `\r\n`）；
    /// Rust 侧统一用 `\n`。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "BEGIN Props")?;
        if self.complete {
            writeln!(f, "   COMPLETE")?;
        } else {
            writeln!(f, "   INCOMPLETE")?;
        }

        for (key, value) in self.props.entries() {
            writeln!(
                f,
                "   Key: {key} Value: {}",
                ByteArrayHelper::hexdump_all(value, true)
            )?;
        }

        writeln!(f, "END Props")?;
        // Java: pw.println();
        writeln!(f)?;

        Ok(())
    }
}

impl std::ops::Deref for Props8 {
    type Target = Props;

    /// 对应 Java: 继承关系带来的「子类实例可直接当作基类使用」
    fn deref(&self) -> &Self::Target {
        &self.props
    }
}

impl std::ops::DerefMut for Props8 {
    /// 对应 Java: 继承关系带来的「子类实例可直接当作基类使用」
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.props
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个属性条目：Java 从 `attrib` 的 4 个字节里同时取出
    /// `attrib2`（小端整数）与 `attrib3`（第 3 个字节）。
    struct Entry {
        attrib1: i32,
        attrib2: i32,
        attrib3: u8,
        attrib5: i32,
        data: Vec<u8>,
    }

    fn write_entries(entries: &[Entry]) -> Vec<u8> {
        let mut data = Vec::new();

        // Java: readInt ×3 + readShort count + readShort
        data.extend_from_slice(&1024i32.to_le_bytes());
        data.extend_from_slice(&1024i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(entries.len() as i16).to_le_bytes());
        data.extend_from_slice(&0i16.to_le_bytes());

        for entry in entries {
            data.extend_from_slice(&entry.attrib1.to_le_bytes());
            data.push((entry.attrib2 & 0xFF) as u8);
            data.push(((entry.attrib2 >> 8) & 0xFF) as u8);
            data.push(entry.attrib3);
            data.push(((entry.attrib2 >> 24) & 0xFF) as u8);
            data.extend_from_slice(&entry.attrib5.to_le_bytes());
            data.extend_from_slice(&entry.data);
            // Java: 对齐到两字节边界
            if entry.data.len() % 2 != 0 {
                data.push(0);
            }
        }

        data
    }

    #[test]
    fn reads_entries_with_attrib3_length() {
        // attrib3 == 64 → 长度取 attrib1
        let entries = [
            Entry {
                attrib1: 2,
                // PropsKey.PASSWORD_FLAG == 893386752 == 0x35400000
                attrib2: 893386752,
                attrib3: 64,
                attrib5: 0,
                data: vec![0x02, 0x01],
            },
            Entry {
                attrib1: 4,
                // PropsKey.ENCRYPTION_CODE == 893386759 == 0x35400007
                attrib2: 893386759,
                attrib3: 64,
                attrib5: 0,
                data: vec![0xD2, 0x04, 0x00, 0x00],
            },
        ];

        let file = ProjectFile::new();
        let props = Props8::new(&file, &write_entries(&entries));

        assert!(props.is_complete());
        assert_eq!(props.key_set(), vec![893386752, 893386759]);
        assert_eq!(props.get_short(PropsKey::PasswordFlag), 258);
        assert_eq!(props.get_int(PropsKey::EncryptionCode), 1234);
        assert!(props.get_boolean(PropsKey::PasswordFlag));
        assert!(file.borrow().get_ignored_errors().is_empty());
    }

    #[test]
    fn reads_entries_with_attrib5_length_and_65536_override() {
        // 注意：attrib3 就是 attrib2（小端整数）的第 3 个字节，两者必须自洽
        let entries = [
            // attrib3 = 2 != 64 → 长度取 attrib5
            Entry {
                attrib1: 999,
                // PropsKey.TASK_FIELD_MAP == 131092 == 0x00020014（byte2 == 2）
                attrib2: 131092,
                attrib3: 2,
                attrib5: 2,
                data: vec![0x00, 0x00],
            },
            // attrib5 == 65536 → 长度固定为 4（覆盖 attrib1 = 1）
            Entry {
                attrib1: 1,
                // PropsKey.TASK_FIELD_NAME_ALIASES == 1048577 == 0x00100001（byte2 == 16）
                attrib2: 1048577,
                attrib3: 16,
                attrib5: 65536,
                data: vec![0x01, 0x00, 0x00, 0x00],
            },
        ];

        let file = ProjectFile::new();
        let props = Props8::new(&file, &write_entries(&entries));

        assert!(props.is_complete());
        assert_eq!(props.get_short(PropsKey::TaskFieldMap), 0);
        assert!(!props.get_boolean(PropsKey::TaskFieldMap));
        assert_eq!(props.get_int(PropsKey::TaskFieldNameAliases), 1);
    }

    #[test]
    fn odd_length_data_is_aligned() {
        // 第一条数据长度为 1（奇数），需要跳过 1 个对齐字节后再读第二条
        let entries = [
            Entry {
                attrib1: 1,
                // PropsKey.TITLE == 37748744 == 0x02400008
                attrib2: 37748744,
                attrib3: 64,
                attrib5: 0,
                data: vec![0x03],
            },
            Entry {
                attrib1: 2,
                // PropsKey.SUBJECT == 37748745 == 0x02400009
                attrib2: 37748745,
                attrib3: 64,
                attrib5: 0,
                data: vec![0x02, 0x01],
            },
        ];

        let file = ProjectFile::new();
        let props = Props8::new(&file, &write_entries(&entries));

        assert!(props.is_complete());
        assert_eq!(props.get_byte(PropsKey::Title), 3);
        assert_eq!(props.get_short(PropsKey::Subject), 258);
    }

    #[test]
    fn zero_length_entry_stops_parsing() {
        let entries = [
            Entry {
                attrib1: 2,
                attrib2: 37748744,
                attrib3: 64,
                attrib5: 0,
                data: vec![0x02, 0x01],
            },
            // size == 0 → 结构不可理解，停止解析
            Entry {
                attrib1: 0,
                attrib2: 37748745,
                attrib3: 64,
                attrib5: 0,
                data: Vec::new(),
            },
        ];

        let file = ProjectFile::new();
        let props = Props8::new(&file, &write_entries(&entries));

        assert!(!props.is_complete());
        assert_eq!(props.key_set(), vec![37748744]);
        assert_eq!(props.get_short(PropsKey::Title), 258);
        assert_eq!(props.get_short(PropsKey::Subject), 0);
        assert!(props.to_string().contains("INCOMPLETE"));
    }

    #[test]
    fn truncated_data_records_ignored_error() {
        // 声明 2 条，但第二条的头部被截断
        let mut data = write_entries(&[Entry {
            attrib1: 2,
            attrib2: 37748744,
            attrib3: 64,
            attrib5: 0,
            data: vec![0x02, 0x01],
        }]);
        data[12] = 2; // count = 2（小端 short 位于偏移 12，第 4 字节为低字节）

        let file = ProjectFile::new();
        let props = Props8::new(&file, &data);

        assert!(!props.is_complete());
        // 已读到的属性保留
        assert_eq!(props.get_short(PropsKey::Title), 258);
        // Java: file.addIgnoredError(ex)
        assert_eq!(file.borrow().get_ignored_errors().len(), 1);
        assert!(props.to_string().contains("INCOMPLETE"));
    }

    #[test]
    fn empty_data_is_incomplete() {
        let file = ProjectFile::new();
        let props = Props8::new(&file, &[]);

        assert!(!props.is_complete());
        assert!(props.key_set().is_empty());
        assert_eq!(file.borrow().get_ignored_errors().len(), 1);
    }

    #[test]
    fn display_lists_values() {
        let entries = [Entry {
            attrib1: 2,
            attrib2: 37748744,
            attrib3: 64,
            attrib5: 0,
            data: vec![0x02, 0x01],
        }];

        let file = ProjectFile::new();
        let props = Props8::new(&file, &write_entries(&entries));

        let text = props.to_string();
        assert!(text.starts_with("BEGIN Props\n   COMPLETE\n"));
        // Java: ByteArrayHelper.hexdump(value, true) —— 含 ASCII 视图（不可打印字符为空格）
        assert!(text.contains("   Key: 37748744 Value: 02 01"));
        assert!(text.ends_with("END Props\n\n"));
    }

    #[test]
    fn missing_keys_return_defaults() {
        let file = ProjectFile::new();
        let props = Props8::new(&file, &write_entries(&[]));

        assert!(props.is_complete());
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
    }
}
