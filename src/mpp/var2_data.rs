//! 对应 Java: `org/mpxj/mpp/Var2Data.java`
//!
//! 变长数据块：每个数据项在块中的形式是「4 字节长度 + 数据本身」，而各项的位置由
//! [`VarMeta`] 给出的偏移索引（`唯一 ID → 数据类型 → 偏移`）决定。本类在构造时按索引把
//! 全部数据项取出缓存在 `偏移 → 字节数组` 映射里，随后按 `(唯一 ID, 数据类型)` 提供取值访问器。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `Var2Data(ProjectFile file, VarMeta meta, InputStream is)` | `Var2Data::new(&Rc<RefCell<ProjectFile>>, Rc<dyn VarMeta>, &[u8])` | 按约定 `InputStream` → `&[u8]`；`file` 只用于记录「忽略错误」（见下） |
//! | `private final VarMeta m_meta` | `Rc<dyn VarMeta>` | Java 保存引用（同一实例可能被 reader 另存一份），Rust 用共享句柄还原该语义 |
//! | `TreeMap<Integer, byte[]> m_map` | `BTreeMap<i32, Vec<u8>>` | `TreeMap` → `BTreeMap`：`toString` 的遍历顺序与 Java 一致 |
//! | `Integer` 参数（可空） | `Option<i32>` | 遵循「`null` → `Option<T>`」约定 |
//! | 重载 `getByteArray(Integer)` / `getByteArray(Integer, Integer)` | `get_byte_array_at_offset` / `get_byte_array` | Rust 无重载：以 `(id, type)` 取值的版本保留原名（Java 侧调用点最多），只按偏移取值的版本加 `_at_offset` 后缀。`getString` / `getUnicodeString` 同理 |
//! | 重载 `getInt(Integer, Integer)` / `getInt(Integer, int, Integer)` | `get_int` / `get_int_at` | 同上；`get_int_at` 的参数顺序与 Java 一致（`id`, `offset`, `type`） |
//! | `String toString()` / `String toString(Integer id)` | `impl Display` / `to_string_for_id` | Rust 无重载 |
//! | POI 抛 `IndexOutOfBoundsException` 时 `file.addIgnoredError(ex)` 并 `break` | 数据块被截断（长度字段本身读不出来）时记录 [`MpxjError`] 并停止解析 | Rust 侧切片越界可预判，不存在异常；其余「尽力容忍损坏文件」的逻辑与 Java 一致 |
//! | Java 的 `is.reset()` / `InputStreamHelper.skip(is, n)` | 内部游标定位（`position = 0` / `position + n`，负数不移动） | Rust 侧没有流，见 [`Var2Data::new`] 中的逐行注释 |
//!
//! # 本文件对并行模块接口的假设（集成时如不一致需对齐）
//!
//! Java 用 `MPPUtility` 解码字符串 / 时间戳 / 单字节，Rust 侧沿用同样的调用：
//!
//! ```text
//! MPPUtility::get_byte(data: &[u8], offset: usize) -> i32
//! MPPUtility::get_string(data: &[u8], offset: usize) -> String
//! MPPUtility::get_unicode_string(data: &[u8], offset: usize) -> String
//! MPPUtility::get_timestamp(data: &[u8], offset: usize) -> Option<NaiveDateTime>
//! ```
//!
//! 跳过的 Java 方法：无（`Var2Data` 没有 `getDate` / `getDuration` 访问器；日期与工期由调用方
//! 先经 `(id, type)` 取到字节数组、再用 `MPPUtility.getDate` / `getDuration` 解码，故本类的
//! 测试也用同样方式验证这两类数据）。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use chrono::NaiveDateTime;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpxj_exception::MpxjError;
use crate::mpp::abstract_var_meta::read_int;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::var_meta::VarMeta;
use crate::project_file::ProjectFile;

/// 一块变长数据。
///
/// 对应 Java: `final class Var2Data extends MPPComponent`
pub struct Var2Data {
    /// 数据项，按其在块中的偏移索引。
    ///
    /// 对应 Java 字段: `private final TreeMap<Integer, byte[]> m_map = new TreeMap<>();`
    map: BTreeMap<i32, Vec<u8>>,

    /// 与本块关联的元数据。
    ///
    /// 对应 Java 字段: `private final VarMeta m_meta;`
    meta: Rc<dyn VarMeta>,
}

impl Var2Data {
    /// 构造：按元数据给出的偏移索引，从数据块中取出全部变长数据项。
    ///
    /// 对应 Java: `Var2Data(ProjectFile file, VarMeta meta, InputStream is) throws IOException`
    ///
    /// Rust 侧把 `InputStream` 换成字节切片，因此 Java 里“流的当前位置 / `reset` / `skip`”
    /// 用本函数内的 `position` 游标还原：
    /// - Java 的 `is.available()` → 剩余字节数；
    /// - Java 的 `is.reset()`（回到流的起始位置）→ `position = 0`；
    /// - Java 的 `InputStreamHelper.skip(is, n)` → `position += n`，`n <= 0` 时不移动
    ///   （与 `InputStream.skip` 对负数不跳过的约定一致）。
    ///
    /// `file` 仅用于 Java 的 `file.addIgnoredError(ex)` 分支：数据块在读完长度字段之前就结束
    /// （Java 会抛 `EOFException` / `IndexOutOfBoundsException`，构造失败）时，Rust 侧记录一条
    /// 忽略错误并停止解析，绝不 panic。
    pub fn new(file: &Rc<RefCell<ProjectFile>>, meta: Rc<dyn VarMeta>, data: &[u8]) -> Self {
        let mut map: BTreeMap<i32, Vec<u8>> = BTreeMap::new();

        // Java: int currentOffset = 0; int available = is.available();
        let available = data.len();
        let mut current_offset: i32 = 0;
        let mut position = 0usize;

        // Java: for (int itemOffset : meta.getOffsets())
        for &item_offset in meta.get_offsets() {
            // Java: if (itemOffset >= available) { continue; }
            if item_offset >= available as i32 {
                continue;
            }

            // Java 注释：偏移可以重复（数据项被去重共享）。映射以偏移为键，重复项会产生
            // 完全相同的条目，直接跳过，避免把流位置重置并重新跳到流起始处。
            // Java: if (m_map.containsKey(Integer.valueOf(itemOffset))) { continue; }
            if map.contains_key(&item_offset) {
                continue;
            }

            // Java: if (currentOffset > itemOffset) { is.reset(); InputStreamHelper.skip(is, itemOffset); }
            //       else { if (currentOffset < itemOffset) { InputStreamHelper.skip(is, itemOffset - currentOffset); } }
            // 两条分支的净效果都是「把读取位置移到 itemOffset」。
            if current_offset > item_offset {
                position = skip(0, item_offset);
            } else if current_offset < item_offset {
                // Java 的差额用 int 运算（可能回绕），Rust 侧用 wrapping_sub 保持同样的语义
                position = skip(position, item_offset.wrapping_sub(current_offset));
            }

            // Java: int size = readInt(is);
            let Some(size) = read_int(data, &mut position) else {
                // Java: POI 在越界时抛 EOFException / IndexOutOfBoundsException，构造方法随之失败。
                // Rust 侧不 panic：记录忽略错误后停止解析。
                if let Ok(mut file) = file.try_borrow_mut() {
                    file.add_ignored_error(MpxjError::new("Unexpected end of Var2Data block"));
                }
                break;
            };

            // Java 注释：尽力容忍损坏文件
            // Java: if (size < 0 || size > is.available()) { continue; }
            if size < 0 || size as usize > data.len().saturating_sub(position) {
                continue;
            }

            // Java: data = readByteArray(is, size);
            //（Java 的 try/catch IndexOutOfBoundsException 分支在此不可达：上面的长度检查
            // 已保证切片落在范围内。）
            let item = data[position..position + size as usize].to_vec();
            position += size as usize;

            // Java: m_map.put(Integer.valueOf(itemOffset), data);
            //       currentOffset = itemOffset + 4 + size;
            map.insert(item_offset, item);
            current_offset = item_offset.wrapping_add(4).wrapping_add(size);
        }

        Self { map, meta }
    }

    /// 按偏移取数据项的字节；不存在时返回 `None`。
    ///
    /// 对应 Java: `public byte[] getByteArray(Integer offset)`
    pub fn get_byte_array_at_offset(&self, offset: Option<i32>) -> Option<Vec<u8>> {
        // Java: byte[] result = null; if (offset != null) { result = m_map.get(offset); }
        let offset = offset?;
        self.map.get(&offset).cloned()
    }

    /// 按「唯一 ID + 数据类型」取数据项的字节；不存在时返回 `None`。
    ///
    /// 对应 Java: `public byte[] getByteArray(Integer id, Integer type)`
    pub fn get_byte_array(&self, id: Option<i32>, type_: Option<i32>) -> Option<Vec<u8>> {
        // Java: return getByteArray(m_meta.getOffset(id, type));
        self.get_byte_array_at_offset(self.meta.get_offset(id, type_))
    }

    /// 按偏移取出数据并按「双字节字符（UTF-16LE，NUL 结尾）」解码；不存在时返回 `None`。
    ///
    /// 对应 Java: `public String getUnicodeString(Integer offset)`
    pub fn get_unicode_string_at_offset(&self, offset: Option<i32>) -> Option<String> {
        let offset = offset?;
        let value = self.map.get(&offset)?;

        // Java: MPPUtility.getUnicodeString(value, 0);
        Some(MPPUtility::get_unicode_string(value, 0))
    }

    /// 按「唯一 ID + 数据类型」取出数据并按 UTF-16LE 解码；不存在时返回 `None`。
    ///
    /// 对应 Java: `public String getUnicodeString(Integer id, Integer type)`
    pub fn get_unicode_string(&self, id: Option<i32>, type_: Option<i32>) -> Option<String> {
        // Java: return getUnicodeString(m_meta.getOffset(id, type));
        self.get_unicode_string_at_offset(self.meta.get_offset(id, type_))
    }

    /// 按「唯一 ID + 数据类型」取时间戳；不存在或数据不足 4 字节时返回 `None`。
    ///
    /// 对应 Java: `public LocalDateTime getTimestamp(Integer id, Integer type)`
    pub fn get_timestamp(&self, id: Option<i32>, type_: Option<i32>) -> Option<NaiveDateTime> {
        let offset = self.meta.get_offset(id, type_)?;
        let value = self.map.get(&offset)?;

        // Java: if (value != null && value.length >= 4) { result = MPPUtility.getTimestamp(value, 0); }
        if value.len() < 4 {
            return None;
        }

        MPPUtility::get_timestamp(value, 0)
    }

    /// 按偏移取出数据并按「单字节字符（NUL 结尾）」解码；不存在时返回 `None`。
    ///
    /// 对应 Java: `public String getString(Integer offset)`
    pub fn get_string_at_offset(&self, offset: Option<i32>) -> Option<String> {
        let offset = offset?;
        let value = self.map.get(&offset)?;

        // Java: MPPUtility.getString(value, 0);
        Some(MPPUtility::get_string(value, 0))
    }

    /// 按「唯一 ID + 数据类型」取出数据并按单字节字符解码；不存在时返回 `None`。
    ///
    /// 对应 Java: `public String getString(Integer id, Integer type)`
    pub fn get_string(&self, id: Option<i32>, type_: Option<i32>) -> Option<String> {
        // Java: return getString(m_meta.getOffset(id, type));
        self.get_string_at_offset(self.meta.get_offset(id, type_))
    }

    /// 按「唯一 ID + 数据类型」取 2 字节整数（`0..=65535`）；不存在或数据不足 2 字节时返回 0。
    ///
    /// 对应 Java: `public int getShort(Integer id, Integer type)`
    pub fn get_short(&self, id: Option<i32>, type_: Option<i32>) -> i32 {
        let mut result = 0;

        if let Some(offset) = self.meta.get_offset(id, type_) {
            if let Some(value) = self.map.get(&offset) {
                // Java: if (value != null && value.length >= 2) { result = ByteArrayHelper.getShort(value, 0); }
                if value.len() >= 2 {
                    result = ByteArrayHelper::get_short(value, 0);
                }
            }
        }

        result
    }

    /// 按「唯一 ID + 数据类型」取 1 字节无符号整数（`0..=255`）；不存在时返回 0。
    ///
    /// 对应 Java: `public int getByte(Integer id, Integer type)`
    ///
    /// Java 对空数组调用 `MPPUtility.getByte` 会抛 `ArrayIndexOutOfBoundsException`，
    /// Rust 侧返回 0（见文件头「不 panic」约定）。
    pub fn get_byte(&self, id: Option<i32>, type_: Option<i32>) -> i32 {
        let mut result = 0;

        if let Some(offset) = self.meta.get_offset(id, type_) {
            if let Some(value) = self.map.get(&offset) {
                if !value.is_empty() {
                    // Java: result = MPPUtility.getByte(value, 0);
                    result = MPPUtility::get_byte(value, 0);
                }
            }
        }

        result
    }

    /// 按「唯一 ID + 数据类型」取 4 字节整数；不存在或数据不足 4 字节时返回 0。
    ///
    /// 对应 Java: `public int getInt(Integer id, Integer type)`
    pub fn get_int(&self, id: Option<i32>, type_: Option<i32>) -> i32 {
        let mut result = 0;

        if let Some(offset) = self.meta.get_offset(id, type_) {
            if let Some(value) = self.map.get(&offset) {
                // Java: if (value != null && value.length >= 4) { result = ByteArrayHelper.getInt(value, 0); }
                if value.len() >= 4 {
                    result = ByteArrayHelper::get_int(value, 0);
                }
            }
        }

        result
    }

    /// 按「唯一 ID + 数据类型」从数据项的任意偏移处取 4 字节整数；取不到时返回 0。
    ///
    /// 对应 Java: `public int getInt(Integer id, int offset, Integer type)`
    ///
    /// Java 对负的 `offset` 会抛 `ArrayIndexOutOfBoundsException`，Rust 侧返回 0。
    pub fn get_int_at(&self, id: Option<i32>, offset: i32, type_: Option<i32>) -> i32 {
        let mut result = 0;

        if let Some(meta_offset) = self.meta.get_offset(id, type_) {
            if let Some(value) = self.map.get(&meta_offset) {
                // Java: if (value != null && value.length >= offset + 4) { result = ByteArrayHelper.getInt(value, offset); }
                if offset >= 0 && (offset as usize) + 4 <= value.len() {
                    result = ByteArrayHelper::get_int(value, offset as usize);
                }
            }
        }

        result
    }

    /// 按「唯一 ID + 数据类型」取 8 字节整数；不存在或数据不足 8 字节时返回 0。
    ///
    /// 对应 Java: `public long getLong(Integer id, Integer type)`
    pub fn get_long(&self, id: Option<i32>, type_: Option<i32>) -> i64 {
        let mut result = 0;

        if let Some(offset) = self.meta.get_offset(id, type_) {
            if let Some(value) = self.map.get(&offset) {
                // Java: if (value != null && value.length >= 8) { result = ByteArrayHelper.getLong(value, 0); }
                if value.len() >= 8 {
                    result = ByteArrayHelper::get_long(value, 0);
                }
            }
        }

        result
    }

    /// 按「唯一 ID + 数据类型」取 8 字节浮点数；不存在或数据不足 8 字节时返回 0。
    ///
    /// 对应 Java: `public double getDouble(Integer id, Integer type)`
    ///
    /// 与 Java 一致：把 `getLong` 的位模式解释为 `double`，`NaN` 归一为 0。
    pub fn get_double(&self, id: Option<i32>, type_: Option<i32>) -> f64 {
        // Java: double result = Double.longBitsToDouble(getLong(id, type)); if (Double.isNaN(result)) { result = 0; }
        let mut result = f64::from_bits(self.get_long(id, type_) as u64);
        if result.is_nan() {
            result = 0.0;
        }
        result
    }

    /// 取底层元数据（主要供调试使用）。
    ///
    /// 对应 Java: `public VarMeta getVarMeta()`
    pub fn get_var_meta(&self) -> &dyn VarMeta {
        &*self.meta
    }

    /// 以调试用文本导出本块全部数据项。
    ///
    /// 对应 Java: `public String toString(Integer id)`
    ///
    /// Rust 侧无法重载 `to_string`，故该方法改名为 `to_string_for_id`（整块导出见 `Display`）。
    /// `id` 为 `None` 时返回空文本（Java 对 `null` 会因 `getTypes(null)` 抛 NPE）。
    pub fn to_string_for_id(&self, id: Option<i32>) -> String {
        let mut result = String::new();
        result.push_str(&format!("BEGIN Var2Data for {}\n", format_id(id)));

        for type_ in self.meta.get_types(id) {
            let offset = self.meta.get_offset(id, Some(type_));
            let data = offset.and_then(|offset| self.map.get(&offset));

            let data: &[u8] = match &data {
                Some(data) => data,
                // Java: data.length 会抛 NPE；Rust 侧按空数据输出（不 panic）
                None => &[],
            };
            let size = data.len();
            result.push_str(&format!(
                "   Data at offset: {} size: {size}\n",
                format_id(offset)
            ));
            result.push_str(&ByteArrayHelper::hexdump_columns(data, true, 16, "   "));
        }

        result.push_str(&format!("END Var2Data for {}\n\n", format_id(id)));
        result
    }
}

impl Display for Var2Data {
    /// 输出与 Java 的 `toString()` 相同的文本。
    ///
    /// 对应 Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut result = String::new();
        result.push_str("BEGIN Var2Data\n");

        for (offset, data) in &self.map {
            let size = data.len();
            result.push_str(&format!("   Data at offset: {offset} size: {size}\n"));
            result.push_str(&ByteArrayHelper::hexdump_columns(data, true, 16, "   "));
        }

        result.push_str("END Var2Data\n\n");
        f.write_str(&result)
    }
}

/// 模拟 Java 的 `InputStreamHelper.skip`：`count <= 0` 时不移动（`InputStream.skip` 亦然）。
fn skip(position: usize, count: i32) -> usize {
    if count <= 0 {
        position
    } else {
        position.saturating_add(count as usize)
    }
}

/// 把可空的整数按 Java 的字符串拼接方式渲染（`null` → `"null"`）。
fn format_id(value: Option<i32>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpp::var_meta9::VarMeta9;
    use chrono::{Days, NaiveDate, NaiveTime};

    /// 手工拼装 Var2Data 数据块，返回「数据块 + 每项的偏移」。
    ///
    /// 每项的形式与 MPP 一致：4 字节长度 + 数据本身。
    fn data_block(items: &[Vec<u8>]) -> (Vec<u8>, Vec<i32>) {
        let mut data = Vec::new();
        let mut offsets = Vec::new();

        for item in items {
            offsets.push(data.len() as i32);
            data.extend_from_slice(&(item.len() as i32).to_le_bytes());
            data.extend_from_slice(item);
        }

        (data, offsets)
    }

    /// 用「唯一 ID + 类型 + 数据项索引」拼装配套的 VarMeta9 块。
    fn var_meta9_bytes(entries: &[(i32, i32, usize)], offsets: &[i32], data_size: i32) -> Vec<u8> {
        let mut data = Vec::new();
        // magic = 0xFADFADBA（小端）
        data.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(entries.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&data_size.to_le_bytes());

        for (unique_id, type_, index) in entries {
            data.extend_from_slice(&unique_id.to_le_bytes()[..3]);
            data.push(*type_ as u8);
            data.extend_from_slice(&offsets[*index].to_le_bytes());
        }

        data
    }

    /// 测试用样本：数据块、配套元数据与各数据项的 `(id, type)`。
    ///
    /// 各数据项（按偏移顺序）：
    /// 0) `ID=1 TYPE=0x12` → 1 字节 42
    /// 1) `ID=2 TYPE=0x12` → 2 字节 0x1234（4660）
    /// 2) `ID=3 TYPE=0x12` → 4 字节 0x12345678
    /// 3) `ID=4 TYPE=0x12` → 8 字节 double 1.5
    /// 4) `ID=5 TYPE=0x12` → 8 字节 long 1
    /// 5) `ID=6 TYPE=0x12` → 单字节字符串 "AB"
    /// 6) `ID=7 TYPE=0x12` → UTF-16LE 字符串 "AB"
    /// 7) `ID=8 TYPE=0x12` → 时间戳（秒数 0、天数 1000）
    /// 8) `ID=9 TYPE=0x12` → 日期字节（天数 1 → 1984-01-01）
    /// 9) `ID=10 TYPE=0x12` → 工期字节（4800 个十分之一分钟 → 8 小时）
    /// 10) `ID=11 TYPE=0x12` → 日期 NA（65535）
    /// 11) `ID=12 TYPE=0x12` → 6 字节，用来验证从任意偏移读整数
    /// 12) `ID=13 TYPE=0x12` → 空数据（0 字节）
    fn sample() -> (Vec<u8>, Vec<u8>, Vec<(i32, i32)>) {
        let items = vec![
            vec![0x2A],
            vec![0x34, 0x12],
            vec![0x78, 0x56, 0x34, 0x12],
            vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF8, 0x3F],
            vec![0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            vec![0x41, 0x42, 0x00],
            vec![0x41, 0x00, 0x42, 0x00, 0x00, 0x00],
            vec![0x00, 0x00, 0xE8, 0x03],
            vec![0x01, 0x00],
            vec![0xC0, 0x12],
            vec![0xFF, 0xFF],
            vec![0x00, 0x00, 0x78, 0x56, 0x34, 0x12],
            Vec::new(),
        ];

        let (data, offsets) = data_block(&items);
        let entries: Vec<(i32, i32, usize)> = (0..items.len())
            .map(|index| (index as i32 + 1, 0x12, index))
            .collect();
        let meta = var_meta9_bytes(&entries, &offsets, data.len() as i32);
        let ids: Vec<(i32, i32)> = entries.iter().map(|entry| (entry.0, entry.1)).collect();

        (data, meta, ids)
    }

    /// 构造一个 Var2Data 样本。
    fn sample_var_data() -> (Var2Data, Rc<RefCell<ProjectFile>>) {
        let (data, meta_bytes, _) = sample();
        let meta = VarMeta9::new(&meta_bytes).expect("元数据合法");
        let file = ProjectFile::new();
        let var_data = Var2Data::new(&file, Rc::new(meta), &data);
        (var_data, file)
    }

    /// 按 MPXJ 规则解码日期（对应 `MPPUtility.getDate`，Java 侧由调用方调用）。
    ///
    /// 这里保留一个薄封装，是为了让测试的期望值写法与 Java 调用点一致。
    fn decode_date(data: &[u8]) -> Option<NaiveDate> {
        MPPUtility::get_date(data, 0)
    }

    /// 按 MPXJ 规则解码工期（对应 `MPPUtility.getDuration`）：返回毫秒数。
    fn decode_duration(data: &[u8]) -> i64 {
        MPPUtility::get_duration(data, 0)
    }

    /// 元数据给出条目数，且各项能按 `(id, type)` 取到。
    #[test]
    fn parses_all_items_indexed_by_id_and_type() {
        let (var_data, _file) = sample_var_data();
        let (_, meta_bytes, ids) = sample();

        // 条目数解析
        let meta = VarMeta9::new(&meta_bytes).expect("元数据合法");
        assert_eq!(meta.get_item_count(), 13);
        assert_eq!(var_data.get_var_meta().get_item_count(), 13);
        assert_eq!(var_data.get_var_meta().get_offsets().len(), 13);

        // 按 (id, type) 取各种类型
        let (id, type_) = ids[0];
        assert_eq!(var_data.get_byte(Some(id), Some(type_)), 42);
        assert_eq!(
            var_data.get_byte_array(Some(id), Some(type_)),
            Some(vec![0x2A])
        );

        let (id, type_) = ids[1];
        assert_eq!(var_data.get_short(Some(id), Some(type_)), 0x1234);

        let (id, type_) = ids[2];
        assert_eq!(var_data.get_int(Some(id), Some(type_)), 0x12345678);

        let (id, type_) = ids[3];
        assert_eq!(var_data.get_double(Some(id), Some(type_)), 1.5);

        let (id, type_) = ids[4];
        assert_eq!(var_data.get_long(Some(id), Some(type_)), 1);

        let (id, type_) = ids[5];
        assert_eq!(
            var_data.get_string(Some(id), Some(type_)),
            Some("AB".to_owned())
        );

        let (id, type_) = ids[6];
        assert_eq!(
            var_data.get_unicode_string(Some(id), Some(type_)),
            Some("AB".to_owned())
        );

        // 时间戳：1983-12-31 + 1000 天，秒数为 0
        let (id, type_) = ids[7];
        let expected_date = NaiveDate::from_ymd_opt(1983, 12, 31)
            .and_then(|epoch| epoch.checked_add_days(Days::new(1000)))
            .expect("日期可计算");
        assert_eq!(
            var_data.get_timestamp(Some(id), Some(type_)),
            Some(NaiveDateTime::new(
                expected_date,
                NaiveTime::from_hms_opt(0, 0, 0).expect("合法时间")
            ))
        );

        // 从数据项内部任意偏移读整数
        let (id, type_) = ids[11];
        assert_eq!(var_data.get_int_at(Some(id), 2, Some(type_)), 0x12345678);
        assert_eq!(var_data.get_int(Some(id), Some(type_)), 0x5678_0000);
        // 负偏移（Java 抛 ArrayIndexOutOfBoundsException）→ 0
        assert_eq!(var_data.get_int_at(Some(id), -1, Some(type_)), 0);
        // 超出数据项长度 → 0
        assert_eq!(var_data.get_int_at(Some(id), 3, Some(type_)), 0);

        // 空数据项：Java 的 getByte 会抛异常，Rust 侧返回 0
        let (id, type_) = ids[12];
        assert_eq!(var_data.get_byte_array(Some(id), Some(type_)), Some(Vec::new()));
        assert_eq!(var_data.get_byte(Some(id), Some(type_)), 0);
        assert_eq!(var_data.get_int(Some(id), Some(type_)), 0);
    }

    /// 日期与工期：Var2Data 按 `(id, type)` 给出字节，调用方按 MPXJ 规则解码。
    #[test]
    fn date_and_duration_items_decode_from_retrieved_bytes() {
        let (var_data, _file) = sample_var_data();
        let (_, _, ids) = sample();

        // 日期：2 字节「自 1983-12-31 起的天数」
        let (id, type_) = ids[8];
        let date_bytes = var_data
            .get_byte_array(Some(id), Some(type_))
            .expect("有数据");
        assert_eq!(date_bytes, vec![0x01, 0x00]);
        assert_eq!(decode_date(&date_bytes), NaiveDate::from_ymd_opt(1984, 1, 1));

        // 日期 NA（65535）
        let (id, type_) = ids[10];
        let na_bytes = var_data
            .get_byte_array(Some(id), Some(type_))
            .expect("有数据");
        assert_eq!(decode_date(&na_bytes), None);

        // 工期：2 字节「十分之一分钟」，4800 → 8 小时
        let (id, type_) = ids[9];
        let duration_bytes = var_data
            .get_byte_array(Some(id), Some(type_))
            .expect("有数据");
        assert_eq!(decode_duration(&duration_bytes), 8 * 60 * 60 * 1000);
    }

    /// 按偏移取值（Java 的 `getByteArray(Integer)` 重载）。
    #[test]
    fn values_can_be_read_by_offset() {
        let (data, meta_bytes, ids) = sample();
        let meta = VarMeta9::new(&meta_bytes).expect("元数据合法");
        let file = ProjectFile::new();
        let var_data = Var2Data::new(&file, Rc::new(meta), &data);

        let (id, type_) = ids[5];
        let offset = var_data
            .get_var_meta()
            .get_offset(Some(id), Some(type_))
            .expect("有偏移");

        assert_eq!(
            var_data.get_byte_array_at_offset(Some(offset)),
            Some(vec![0x41, 0x42, 0x00])
        );
        assert_eq!(
            var_data.get_string_at_offset(Some(offset)),
            Some("AB".to_owned())
        );
        assert_eq!(var_data.get_byte_array_at_offset(None), None);
        assert_eq!(var_data.get_byte_array_at_offset(Some(9999)), None);
        assert_eq!(var_data.get_string_at_offset(Some(9999)), None);
        assert_eq!(var_data.get_unicode_string_at_offset(Some(9999)), None);
    }

    /// 缺失条目返回默认值（Java 返回 0 / null）。
    #[test]
    fn missing_entries_return_java_defaults() {
        let (var_data, _file) = sample_var_data();
        let missing_id = Some(4242);
        let type_ = Some(0x12);

        assert_eq!(var_data.get_byte(missing_id, type_), 0);
        assert_eq!(var_data.get_short(missing_id, type_), 0);
        assert_eq!(var_data.get_int(missing_id, type_), 0);
        assert_eq!(var_data.get_int_at(missing_id, 0, type_), 0);
        assert_eq!(var_data.get_long(missing_id, type_), 0);
        assert_eq!(var_data.get_double(missing_id, type_), 0.0);
        assert_eq!(var_data.get_string(missing_id, type_), None);
        assert_eq!(var_data.get_unicode_string(missing_id, type_), None);
        assert_eq!(var_data.get_timestamp(missing_id, type_), None);
        assert_eq!(var_data.get_byte_array(missing_id, type_), None);

        // 类型不匹配 / 参数为 None（Java 的 null）
        assert_eq!(var_data.get_byte(Some(4242), Some(0x99)), 0);
        assert_eq!(var_data.get_int(None, None), 0);
        assert_eq!(var_data.get_byte_array(None, None), None);
    }

    /// 偏移越界 / 长度字段损坏时不 panic，也不写入条目。
    #[test]
    fn out_of_range_and_corrupt_items_are_ignored() {
        // 数据块只有 4 字节（长度字段 0，无数据）
        let data = 0i32.to_le_bytes().to_vec();

        // 偏移 1000 超出数据块（Java: itemOffset >= available → continue）
        let meta_bytes = var_meta9_bytes(&[(1, 0x12, 0)], &[1000], 4);
        let meta = VarMeta9::new(&meta_bytes).expect("元数据合法");
        let file = ProjectFile::new();
        let var_data = Var2Data::new(&file, Rc::new(meta), &data);
        assert_eq!(var_data.get_byte_array(Some(1), Some(0x12)), None);
        assert!(file.borrow().get_ignored_errors().is_empty());

        // 负偏移（Java 同样会去读流起始处）：不 panic
        let meta_bytes = var_meta9_bytes(&[(1, 0x12, 0)], &[-8], 4);
        let meta = VarMeta9::new(&meta_bytes).expect("元数据合法");
        let file = ProjectFile::new();
        let var_data = Var2Data::new(&file, Rc::new(meta), &data);
        assert_eq!(
            var_data.get_byte_array_at_offset(Some(-8)),
            Some(Vec::new())
        );

        // 长度字段为负 → continue
        let mut negative = (-1i32).to_le_bytes().to_vec();
        negative.extend_from_slice(&[0x2A, 0x2A, 0x2A]);
        let meta_bytes = var_meta9_bytes(&[(1, 0x12, 0)], &[0], 7);
        let meta = VarMeta9::new(&meta_bytes).expect("元数据合法");
        let file = ProjectFile::new();
        let var_data = Var2Data::new(&file, Rc::new(meta), &negative);
        assert_eq!(var_data.get_byte_array(Some(1), Some(0x12)), None);

        // 长度字段大于剩余字节 → continue
        let mut oversized = 100i32.to_le_bytes().to_vec();
        oversized.extend_from_slice(&[0x2A, 0x2A]);
        let meta_bytes = var_meta9_bytes(&[(1, 0x12, 0)], &[0], 6);
        let meta = VarMeta9::new(&meta_bytes).expect("元数据合法");
        let file = ProjectFile::new();
        let var_data = Var2Data::new(&file, Rc::new(meta), &oversized);
        assert_eq!(var_data.get_byte_array(Some(1), Some(0x12)), None);
    }

    /// 数据块被截断（长度字段本身读不出来）时记录忽略错误并停止解析。
    #[test]
    fn truncated_block_records_ignored_error() {
        // 数据块长 6 字节，偏移 4 处只剩 2 字节，长度字段读不出来
        let data = vec![0u8; 6];
        let meta_bytes = var_meta9_bytes(&[(1, 0x12, 0)], &[4], 6);
        let meta = VarMeta9::new(&meta_bytes).expect("元数据合法");
        let file = ProjectFile::new();

        let var_data = Var2Data::new(&file, Rc::new(meta), &data);

        assert_eq!(var_data.get_byte_array(Some(1), Some(0x12)), None);
        let errors = file.borrow();
        assert_eq!(errors.get_ignored_errors().len(), 1);
        assert_eq!(
            errors.get_ignored_errors()[0].message(),
            "Unexpected end of Var2Data block"
        );
    }

    /// 重复偏移只保留一个条目（Java 的去重逻辑）。
    #[test]
    fn repeated_offsets_are_deduplicated() {
        let (data, offsets) = data_block(&[vec![0x2A], vec![0x99]]);

        // 两个条目都指向第一项
        let meta_bytes = var_meta9_bytes(&[(1, 0x12, 0), (2, 0x12, 0)], &offsets, data.len() as i32);
        let meta = VarMeta9::new(&meta_bytes).expect("元数据合法");
        let file = ProjectFile::new();
        let var_data = Var2Data::new(&file, Rc::new(meta), &data);

        assert_eq!(var_data.get_byte_array(Some(1), Some(0x12)), Some(vec![0x2A]));
        assert_eq!(var_data.get_byte_array(Some(2), Some(0x12)), Some(vec![0x2A]));

        // 映射里只应有一条偏移 0 的记录
        let text = var_data.to_string();
        assert_eq!(text.matches("Data at offset: 0 ").count(), 1);
    }

    /// 调试输出与 Java 的格式一致。
    #[test]
    fn to_string_matches_java_format() {
        let (data, meta_bytes, _) = sample();
        let meta = VarMeta9::new(&meta_bytes).expect("元数据合法");
        let file = ProjectFile::new();
        let var_data = Var2Data::new(&file, Rc::new(meta), &data);

        let text = var_data.to_string();
        assert!(text.starts_with("BEGIN Var2Data\n"));
        assert!(text.ends_with("END Var2Data\n\n"));
        assert!(text.contains("   Data at offset: 0 size: 1\n"));
        // hexdump 的每行前缀是 5 位偏移量
        assert!(text.contains("   00000: 2A   *"));

        let by_id = var_data.to_string_for_id(Some(1));
        assert!(by_id.starts_with("BEGIN Var2Data for 1\n"));
        assert!(by_id.contains("   Data at offset: 0 size: 1\n"));
        assert!(by_id.ends_with("END Var2Data for 1\n\n"));

        // 未知 ID：无条目，仅有头尾
        assert_eq!(
            var_data.to_string_for_id(Some(999)),
            "BEGIN Var2Data for 999\nEND Var2Data for 999\n\n"
        );
        // Java 对 null 会抛 NPE；Rust 侧输出 "null"
        assert!(var_data.to_string_for_id(None).contains("for null"));
    }

    /// `getVarMeta` 返回构造时传入的元数据。
    #[test]
    fn exposes_its_metadata() {
        let (var_data, _file) = sample_var_data();

        assert_eq!(var_data.get_var_meta().get_item_count(), 13);
        assert_eq!(var_data.get_var_meta().get_data_size(), 100);
        assert!(var_data.get_var_meta().contains_key(Some(1)));
        assert_eq!(
            var_data.get_var_meta().get_types(Some(1)),
            std::collections::BTreeSet::from([0x12])
        );
    }
}
