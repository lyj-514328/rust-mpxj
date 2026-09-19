//! 对应 Java: `org/mpxj/mpp/VarMeta12.java`
//!
//! MPP 12 及更新版本的 VarMeta 块解析：块头之后是 `条目数` 个「4 字节唯一 ID + 4 字节偏移 +
//! 2 字节类型 + 2 字节未知」的定长记录（每条 12 字节）。
//!
//! # 继承的还原（组合 + 转发）
//!
//! Java: `final class VarMeta12 extends AbstractVarMeta`。Rust 没有继承，因此：
//! - [`VarMeta12`] **组合**一个 [`AbstractVarMeta`]（基类状态与实现）；
//! - 基类的全部公开方法在本文件**逐一转发**，同时 `impl VarMeta`，使其可放进
//!   `Rc<dyn VarMeta>` 交给 [`crate::mpp::var2_data::Var2Data`]。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `VarMeta12(InputStream is) throws IOException` | `VarMeta12::new(&[u8]) -> MpxjResult<VarMeta12>` | 按约定 `InputStream` → `&[u8]`；`IOException`（坏 magic）→ `MpxjError` |
//! | `is.available() < 12` 时 `break` | `data.len() - position < 12` 时 `break` | 等价（Rust 侧没有流，用剩余字节数代替） |
//! | `readInt` / `readShort`（继承自 `MPPComponent`，越界抛 `EOFException`） | [`read_int`] / [`read_short`] 返回 `None`，循环 `break` 提前结束解析 | 遵循「库代码不 panic」约定 |
//! | `m_itemCount` 为负时 `new int[m_itemCount]` 抛 `NegativeArraySizeException` | 循环不执行，偏移数组为空 | 同上 |
//! | `m_table.computeIfAbsent(..)` | `BTreeMap::entry` + `or_default()` | 等价 |
//!
//! 注意：Java 注释指出，MS Project 本身能正常读取某些 magic 为 0 的文件，因此 **0 也是合法的
//! magic number**（与 [`crate::mpp::var_meta9::VarMeta9`] 不同）。
//!
//! 跳过的 Java 方法：无。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::mpp::abstract_var_meta::{read_int, read_short, AbstractVarMeta};
use crate::mpp::var_meta::{VarDataKeyResolver, VarMeta};

/// magic number（Java 中 `int MAGIC = 0xFADFADBA` 是负数常量，此处保留同样的语义）。
const MAGIC: i32 = 0xFADF_ADBA_u32 as i32;

/// 每条记录的字节数（4 + 4 + 2 + 2）。
///
/// 对应 Java: `if (is.available() < 12) { break; }` 中的字面量 12。
const ITEM_SIZE: usize = 12;

/// MPP 12 及更新版本的变长块元数据。
///
/// 对应 Java: `final class VarMeta12 extends AbstractVarMeta`
pub struct VarMeta12 {
    /// 基类部分（Java 里 `VarMeta12` 本身就是 `AbstractVarMeta`）。
    meta: AbstractVarMeta,
}

impl VarMeta12 {
    /// 构造：从 VarMeta 块的字节流中解析元数据。
    ///
    /// 对应 Java: `VarMeta12(InputStream is) throws IOException`
    ///
    /// 块头固定 24 字节：magic(4) + 未知(4) + 条目数(4) + 未知(4) + 未知(4) + 数据区大小(4)；
    /// 其后每条记录 12 字节：唯一 ID(4) + 偏移(4) + 类型(2) + 未知(2)。
    pub fn new(data: &[u8]) -> MpxjResult<Self> {
        let mut position = 0usize;

        // Java 注释：有一个样本里本该合法的 VarMeta 块 magic 为 0，MS Project 仍能读取，故把 0 视作合法。
        // Java: int magic = readInt(is); if (magic != 0 && magic != MAGIC) { throw new IOException("Bad magic number: " + magic); }
        let magic = read_int(data, &mut position).unwrap_or(0);
        if magic != 0 && magic != MAGIC {
            return Err(MpxjError::new(format!("Bad magic number: {magic}")));
        }

        // Java: /*m_unknown1 =*/ readInt(is);
        let _unknown1 = read_int(data, &mut position);
        // Java: m_itemCount = readInt(is);
        let item_count = read_int(data, &mut position).unwrap_or(0);
        // Java: /*m_unknown2 =*/ readInt(is); /*m_unknown3 =*/ readInt(is);
        let _unknown2 = read_int(data, &mut position);
        let _unknown3 = read_int(data, &mut position);
        // Java: m_dataSize = readInt(is);
        let data_size = read_int(data, &mut position).unwrap_or(0);

        let mut table: BTreeMap<i32, BTreeMap<i32, i32>> = BTreeMap::new();
        let mut offsets: Vec<i32> = Vec::new();

        for _ in 0..item_count {
            // Java: if (is.available() < 12) { break; }
            if data.len().saturating_sub(position) < ITEM_SIZE {
                break;
            }

            // Java: Integer uniqueID = Integer.valueOf(readInt(is));
            let Some(unique_id) = read_int(data, &mut position) else {
                break;
            };
            // Java: Integer offset = Integer.valueOf(readInt(is));
            let Some(offset) = read_int(data, &mut position) else {
                break;
            };
            // Java: Integer type = Integer.valueOf(readShort(is));
            let Some(type_) = read_short(data, &mut position) else {
                break;
            };
            // Java: readShort(is); // unknown 2 bytes
            let _unknown = read_short(data, &mut position);

            // Java: map = m_table.computeIfAbsent(uniqueID, k -> new TreeMap<>()); map.put(type, offset);
            table.entry(unique_id).or_default().insert(type_, offset);

            // Java: offsets[loop] = offset.intValue();
            offsets.push(offset);
        }

        // Java: Arrays.sort(offsets); setOffsets(offsets);
        offsets.sort();

        let mut meta = AbstractVarMeta::new();
        meta.item_count = item_count;
        meta.data_size = data_size;
        meta.table = table;
        meta.set_offsets(offsets);

        Ok(Self { meta })
    }

    /// 取基类部分（Rust 特有：Java 里 `VarMeta12` 本身就是 [`AbstractVarMeta`]）。
    pub fn get_meta(&self) -> &AbstractVarMeta {
        &self.meta
    }

    /// 取条目数（转发基类）。
    ///
    /// 对应 Java: `AbstractVarMeta#getItemCount()`
    pub fn get_item_count(&self) -> i32 {
        self.meta.get_item_count()
    }

    /// 取数据区大小（转发基类）。
    ///
    /// 对应 Java: `AbstractVarMeta#getDataSize()`
    pub fn get_data_size(&self) -> i32 {
        self.meta.get_data_size()
    }

    /// 取全部唯一 ID（升序，转发基类）。
    ///
    /// 对应 Java: `AbstractVarMeta#getUniqueIdentifierArray()`
    pub fn get_unique_identifier_array(&self) -> Vec<i32> {
        self.meta.get_unique_identifier_array()
    }

    /// 取全部唯一 ID 集合（升序，转发基类）。
    ///
    /// 对应 Java: `AbstractVarMeta#getUniqueIdentifierSet()`
    pub fn get_unique_identifier_set(&self) -> BTreeSet<i32> {
        self.meta.get_unique_identifier_set()
    }

    /// 按「唯一 ID + 类型」取偏移（转发基类）。
    ///
    /// 对应 Java: `AbstractVarMeta#getOffset(Integer id, Integer type)`
    pub fn get_offset(&self, id: Option<i32>, type_: Option<i32>) -> Option<i32> {
        self.meta.get_offset(id, type_)
    }

    /// 取排序后的偏移数组（转发基类）。
    ///
    /// 对应 Java: `AbstractVarMeta#getOffsets()`
    pub fn get_offsets(&self) -> &[i32] {
        self.meta.get_offsets()
    }

    /// 取指定唯一 ID 下定义过的类型（升序，转发基类）。
    ///
    /// 对应 Java: `AbstractVarMeta#getTypes(Integer id)`
    pub fn get_types(&self, id: Option<i32>) -> BTreeSet<i32> {
        self.meta.get_types(id)
    }

    /// 判断索引中是否存在给定唯一 ID（转发基类）。
    ///
    /// 对应 Java: `AbstractVarMeta#containsKey(Integer key)`
    pub fn contains_key(&self, key: Option<i32>) -> bool {
        self.meta.contains_key(key)
    }

    /// 以调试用文本导出（转发基类）。
    ///
    /// 对应 Java: `AbstractVarMeta#toString(FieldMap fieldMap)`
    pub fn to_string_with_field_map(&self, field_map: Option<&dyn VarDataKeyResolver>) -> String {
        self.meta.to_string_with_field_map(field_map)
    }
}

impl VarMeta for VarMeta12 {
    /// 转发基类实现。
    ///
    /// 对应 Java: `AbstractVarMeta#getItemCount()`
    fn get_item_count(&self) -> i32 {
        VarMeta12::get_item_count(self)
    }

    /// 转发基类实现。
    ///
    /// 对应 Java: `AbstractVarMeta#getDataSize()`
    fn get_data_size(&self) -> i32 {
        VarMeta12::get_data_size(self)
    }

    /// 转发基类实现。
    ///
    /// 对应 Java: `AbstractVarMeta#getUniqueIdentifierArray()`
    fn get_unique_identifier_array(&self) -> Vec<i32> {
        VarMeta12::get_unique_identifier_array(self)
    }

    /// 转发基类实现。
    ///
    /// 对应 Java: `AbstractVarMeta#getUniqueIdentifierSet()`
    fn get_unique_identifier_set(&self) -> BTreeSet<i32> {
        VarMeta12::get_unique_identifier_set(self)
    }

    /// 转发基类实现。
    ///
    /// 对应 Java: `AbstractVarMeta#getOffset(Integer id, Integer type)`
    fn get_offset(&self, id: Option<i32>, type_: Option<i32>) -> Option<i32> {
        VarMeta12::get_offset(self, id, type_)
    }

    /// 转发基类实现。
    ///
    /// 对应 Java: `AbstractVarMeta#getOffsets()`
    fn get_offsets(&self) -> &[i32] {
        VarMeta12::get_offsets(self)
    }

    /// 转发基类实现。
    ///
    /// 对应 Java: `AbstractVarMeta#getTypes(Integer id)`
    fn get_types(&self, id: Option<i32>) -> BTreeSet<i32> {
        VarMeta12::get_types(self, id)
    }

    /// 转发基类实现。
    ///
    /// 对应 Java: `AbstractVarMeta#containsKey(Integer key)`
    fn contains_key(&self, key: Option<i32>) -> bool {
        VarMeta12::contains_key(self, key)
    }

    /// 转发基类实现。
    ///
    /// 对应 Java: `AbstractVarMeta#toString(FieldMap fieldMap)`
    fn to_string_with_field_map(&self, field_map: Option<&dyn VarDataKeyResolver>) -> String {
        VarMeta12::to_string_with_field_map(self, field_map)
    }
}

impl Display for VarMeta12 {
    /// 输出与 Java 的 `toString()` 相同的文本。
    ///
    /// 对应 Java: `AbstractVarMeta#toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.meta.to_string_with_field_map(None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 拼装一个 VarMeta12 块：块头 + 每条记录「唯一 ID(4) + 偏移(4) + 类型(2) + 未知(2)」。
    fn var_meta12_bytes(magic: i32, items: &[(i32, i32, i32)], data_size: i32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&magic.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(items.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&data_size.to_le_bytes());

        for (unique_id, offset, type_) in items {
            data.extend_from_slice(&unique_id.to_le_bytes());
            data.extend_from_slice(&offset.to_le_bytes());
            data.extend_from_slice(&(*type_ as i16).to_le_bytes());
            data.extend_from_slice(&0i16.to_le_bytes());
        }

        data
    }

    /// 解析条目数、索引与偏移数组。
    #[test]
    fn parses_items_offsets_and_types() {
        let bytes = var_meta12_bytes(MAGIC, &[(2, 40, 0x12), (1, 4, 0x34), (1, 8, 0x12)], 64);
        let meta = VarMeta12::new(&bytes).expect("块头合法");

        assert_eq!(meta.get_item_count(), 3);
        assert_eq!(meta.get_data_size(), 64);
        assert_eq!(meta.get_offsets(), [4, 8, 40]);
        assert_eq!(meta.get_unique_identifier_array(), vec![1, 2]);
        assert_eq!(meta.get_types(Some(1)), BTreeSet::from([0x12, 0x34]));
        assert_eq!(meta.get_offset(Some(1), Some(0x34)), Some(4));
        assert_eq!(meta.get_offset(Some(2), Some(0x12)), Some(40));
        assert!(meta.contains_key(Some(2)));

        // 转发与 Display
        assert_eq!(meta.get_meta().get_item_count(), 3);
        assert!(meta.to_string().ends_with("END: VarMeta\n\n"));
    }

    /// magic 为 0 也被接受（Java 注释：某些合法文件如此）。
    #[test]
    fn accepts_zero_magic_number() {
        let bytes = var_meta12_bytes(0, &[(1, 4, 0x12)], 8);
        let meta = VarMeta12::new(&bytes).expect("magic 为 0 合法");
        assert_eq!(meta.get_unique_identifier_array(), vec![1]);
    }

    /// 其它 magic 值 → 错误。
    #[test]
    fn rejects_bad_magic_number() {
        let bytes = var_meta12_bytes(0x01020304, &[], 0);
        let error = VarMeta12::new(&bytes).err().expect("magic 不匹配应报错");
        assert_eq!(error.message(), "Bad magic number: 16909060");
    }

    /// 剩余字节不足一条记录（12 字节）时提前结束，且不 panic。
    #[test]
    fn stops_when_fewer_than_one_item_remains() {
        // itemCount 声称 3 条，实际只给 1 条 + 6 个残字节
        let mut bytes = var_meta12_bytes(MAGIC, &[(1, 4, 0x12), (2, 8, 0x12), (3, 12, 0x12)], 16);
        bytes.truncate(24 + 12 + 6);

        let meta = VarMeta12::new(&bytes).expect("块头合法");
        assert_eq!(meta.get_item_count(), 3);
        assert_eq!(meta.get_unique_identifier_array(), vec![1]);
        assert_eq!(meta.get_offsets(), [4]);
    }

    /// 空输入不 panic。
    #[test]
    fn empty_input_does_not_panic() {
        // 块头读不到 → magic 视作 0（合法），条目数为 0
        let meta = VarMeta12::new(&[]).expect("magic 0 合法");
        assert_eq!(meta.get_item_count(), 0);
        assert!(meta.get_unique_identifier_array().is_empty());
    }
}
