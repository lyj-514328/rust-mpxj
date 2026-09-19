//! 对应 Java: `org/mpxj/mpp/VarMeta9.java`
//!
//! MPP 9 及更早版本的 VarMeta 块解析：块头之后是 `条目数` 个「3 字节唯一 ID + 1 字节类型 +
//! 4 字节偏移」的定长记录。
//!
//! # 继承的还原（组合 + 转发）
//!
//! Java: `final class VarMeta9 extends AbstractVarMeta`。Rust 没有继承，因此：
//! - [`VarMeta9`] **组合**一个 [`AbstractVarMeta`]（基类状态与实现）；
//! - 基类的全部公开方法在本文件**逐一转发**，同时 `impl VarMeta`，使 `VarMeta9` 与
//!   `AbstractVarMeta` 在使用上等价（可放进 `Rc<dyn VarMeta>` 交给 `Var2Data`）。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `VarMeta9(InputStream is) throws IOException` | `VarMeta9::new(&[u8]) -> MpxjResult<VarMeta9>` | 按约定 `InputStream` → `&[u8]`；`IOException`（坏 magic）→ `MpxjError` |
//! | `readInt` / `readByte`（继承自 `MPPComponent`，越界抛 `EOFException`） | [`read_int`] / [`read_byte`] 返回 `None`，循环 `break` 提前结束解析 | 遵循「库代码不 panic」约定（Java 此时构造失败） |
//! | `m_itemCount` 为负时 `new int[m_itemCount]` 抛 `NegativeArraySizeException` | 循环不执行，偏移数组为空 | 同上 |
//! | `m_table.computeIfAbsent(..)` | `BTreeMap::entry` + `or_default()` | 等价 |
//! | `3 字节唯一 ID` 借 4 字节数组（末字节恒 0）+ `ByteArrayHelper.getInt` 解析 | 同 Java 做法，保留其符号语义 | MPP 的细节：唯一 ID 以 3 字节小端存放 |
//!
//! 跳过的 Java 方法：无。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::mpp::abstract_var_meta::{read_byte, read_int, AbstractVarMeta};
use crate::mpp::var_meta::{VarDataKeyResolver, VarMeta};

/// magic number（Java 中 `int MAGIC = 0xFADFADBA` 是负数常量，此处保留同样的语义）。
const MAGIC: i32 = 0xFADF_ADBA_u32 as i32;

/// MPP 9 及更早版本的变长块元数据。
///
/// 对应 Java: `final class VarMeta9 extends AbstractVarMeta`
pub struct VarMeta9 {
    /// 基类部分（Java 里 `VarMeta9` 本身就是 `AbstractVarMeta`）。
    meta: AbstractVarMeta,
}

impl VarMeta9 {
    /// 构造：从 VarMeta 块的字节流中解析元数据。
    ///
    /// 对应 Java: `VarMeta9(InputStream is) throws IOException`
    ///
    /// 块头固定 24 字节：magic(4) + 未知(4) + 条目数(4) + 未知(4) + 未知(4) + 数据区大小(4)；
    /// 其后每条记录 8 字节：唯一 ID(3) + 类型(1) + 偏移(4)。
    pub fn new(data: &[u8]) -> MpxjResult<Self> {
        let mut position = 0usize;

        // Java: int magic = readInt(is); if (magic != MAGIC) { throw new IOException("Bad magic number: " + magic); }
        // （Java 在输入不足 4 字节时抛 EOFException，Rust 侧以 0 代入）
        let magic = read_int(data, &mut position).unwrap_or(0);
        if magic != MAGIC {
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

        //
        // Java 注释：Microsoft 在这里有点古怪 —— 唯一 ID 以 3 字节整数存放。
        // 沿用 Java 的做法：用 4 字节数组读取，末字节恒为 0，再按小端解析。
        //
        let mut unique_id_array = [0u8; 4];

        // Java: for (int loop = 0; loop < m_itemCount; loop++)
        // （item_count 为负时 Rust 的范围为空，与 Java「构造失败」的差异见文件头）
        for _ in 0..item_count {
            // Java: InputStreamHelper.read(is, uniqueIDArray, 3);
            if position + 3 > data.len() {
                // Java 抛 EOFException（构造失败）；Rust 侧提前结束解析，不 panic。
                break;
            }
            unique_id_array[..3].copy_from_slice(&data[position..position + 3]);
            position += 3;

            // Java: uniqueID = Integer.valueOf(ByteArrayHelper.getInt(uniqueIDArray, 0));
            let unique_id = ByteArrayHelper::get_int(&unique_id_array, 0);

            // Java: type = Integer.valueOf(readByte(is));
            let Some(type_) = read_byte(data, &mut position) else {
                break;
            };

            // Java: offset = Integer.valueOf(readInt(is));
            let Some(offset) = read_int(data, &mut position) else {
                break;
            };

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

    /// 取基类部分（Rust 特有：Java 里 `VarMeta9` 本身就是 [`AbstractVarMeta`]）。
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

impl VarMeta for VarMeta9 {
    /// 转发基类实现。
    ///
    /// 对应 Java: `AbstractVarMeta#getItemCount()`
    fn get_item_count(&self) -> i32 {
        VarMeta9::get_item_count(self)
    }

    /// 转发基类实现。
    ///
    /// 对应 Java: `AbstractVarMeta#getDataSize()`
    fn get_data_size(&self) -> i32 {
        VarMeta9::get_data_size(self)
    }

    /// 转发基类实现。
    ///
    /// 对应 Java: `AbstractVarMeta#getUniqueIdentifierArray()`
    fn get_unique_identifier_array(&self) -> Vec<i32> {
        VarMeta9::get_unique_identifier_array(self)
    }

    /// 转发基类实现。
    ///
    /// 对应 Java: `AbstractVarMeta#getUniqueIdentifierSet()`
    fn get_unique_identifier_set(&self) -> BTreeSet<i32> {
        VarMeta9::get_unique_identifier_set(self)
    }

    /// 转发基类实现。
    ///
    /// 对应 Java: `AbstractVarMeta#getOffset(Integer id, Integer type)`
    fn get_offset(&self, id: Option<i32>, type_: Option<i32>) -> Option<i32> {
        VarMeta9::get_offset(self, id, type_)
    }

    /// 转发基类实现。
    ///
    /// 对应 Java: `AbstractVarMeta#getOffsets()`
    fn get_offsets(&self) -> &[i32] {
        VarMeta9::get_offsets(self)
    }

    /// 转发基类实现。
    ///
    /// 对应 Java: `AbstractVarMeta#getTypes(Integer id)`
    fn get_types(&self, id: Option<i32>) -> BTreeSet<i32> {
        VarMeta9::get_types(self, id)
    }

    /// 转发基类实现。
    ///
    /// 对应 Java: `AbstractVarMeta#containsKey(Integer key)`
    fn contains_key(&self, key: Option<i32>) -> bool {
        VarMeta9::contains_key(self, key)
    }

    /// 转发基类实现。
    ///
    /// 对应 Java: `AbstractVarMeta#toString(FieldMap fieldMap)`
    fn to_string_with_field_map(&self, field_map: Option<&dyn VarDataKeyResolver>) -> String {
        VarMeta9::to_string_with_field_map(self, field_map)
    }
}

impl Display for VarMeta9 {
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

    /// 拼装一个 VarMeta9 块：块头 + 每条记录「唯一 ID(3) + 类型(1) + 偏移(4)」。
    fn var_meta9_bytes(items: &[(i32, i32, i32)], data_size: i32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&MAGIC.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(items.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&data_size.to_le_bytes());

        for (unique_id, type_, offset) in items {
            data.extend_from_slice(&unique_id.to_le_bytes()[..3]);
            data.push(*type_ as u8);
            data.extend_from_slice(&offset.to_le_bytes());
        }

        data
    }

    /// 解析条目数、索引与偏移数组。
    #[test]
    fn parses_items_offsets_and_types() {
        // 故意乱序给出，验证偏移数组被排序、唯一 ID 按升序输出
        let bytes = var_meta9_bytes(&[(2, 0x12, 40), (1, 0x34, 4), (1, 0x12, 8)], 64);
        let meta = VarMeta9::new(&bytes).expect("块头合法");

        assert_eq!(meta.get_item_count(), 3);
        assert_eq!(meta.get_data_size(), 64);
        assert_eq!(meta.get_offsets(), [4, 8, 40]);
        assert_eq!(meta.get_unique_identifier_array(), vec![1, 2]);
        assert_eq!(meta.get_unique_identifier_set(), BTreeSet::from([1, 2]));
        assert_eq!(meta.get_types(Some(1)), BTreeSet::from([0x12, 0x34]));
        assert_eq!(meta.get_types(Some(2)), BTreeSet::from([0x12]));
        assert!(meta.get_types(Some(3)).is_empty());
        assert_eq!(meta.get_offset(Some(1), Some(0x34)), Some(4));
        assert_eq!(meta.get_offset(Some(1), Some(0x99)), None);
        assert!(meta.contains_key(Some(2)));
        assert!(!meta.contains_key(Some(3)));

        // 转发与 Display
        assert_eq!(meta.get_meta().get_item_count(), 3);
        assert!(meta.to_string().starts_with("BEGIN: VarMeta\n   Item count: 3\n"));
        // 同一唯一 ID 下按数据类型升序输出（与 Java 的 TreeMap 一致）
        assert!(meta
            .to_string()
            .contains("Entries for Unique ID: 1\n      Type=18 Offset=8\n      Type=52 Offset=4\n"));
    }

    /// 唯一 ID 以 3 字节存放（第 4 字节恒为 0），验证符号语义。
    #[test]
    fn unique_id_is_read_from_three_bytes() {
        // 0x00FFFFFF 的最高字节为 0，因此是正数 16777215
        let bytes = var_meta9_bytes(&[(0xFFFFFF, 1, 0)], 1);
        let meta = VarMeta9::new(&bytes).expect("块头合法");

        assert_eq!(meta.get_unique_identifier_array(), vec![0xFFFFFF]);
    }

    /// 坏 magic number → 错误（Java 抛 IOException）。
    #[test]
    fn rejects_bad_magic_number() {
        let mut bytes = var_meta9_bytes(&[], 0);
        bytes[..4].copy_from_slice(&0x01020304_u32.to_le_bytes());

        let error = VarMeta9::new(&bytes).err().expect("magic 不匹配应报错");
        assert_eq!(error.message(), "Bad magic number: 16909060");
    }

    /// 截断 / 空输入不 panic（Java 抛 EOFException）。
    #[test]
    fn truncated_input_does_not_panic() {
        assert!(VarMeta9::new(&[]).is_err());

        // 块头完整、但记录只给了一条（itemCount 声称 3 条）
        let mut bytes = var_meta9_bytes(&[(1, 0x12, 4), (2, 0x12, 8), (3, 0x12, 12)], 16);
        bytes.truncate(24 + 8 + 3);

        let meta = VarMeta9::new(&bytes).expect("块头合法");
        assert_eq!(meta.get_item_count(), 3);
        assert_eq!(meta.get_unique_identifier_array(), vec![1]);
        assert_eq!(meta.get_offsets(), [4]);
    }
}
