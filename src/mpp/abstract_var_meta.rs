//! 对应 Java: `org/mpxj/mpp/AbstractVarMeta.java`
//!
//! 变长块体系的基类：解析 VarMeta 流后，持有「条目数、数据区大小、排序后的偏移数组」以及
//! 核心索引 `唯一 ID → 数据类型 → 偏移`，供 [`crate::mpp::var2_data::Var2Data`] 定位变长字段。
//!
//! # 继承与接口的还原（组合 + 转发）
//!
//! Java: `abstract class AbstractVarMeta extends MPPComponent implements VarMeta`。
//!
//! - `implements VarMeta` → `impl VarMeta for AbstractVarMeta`（本文件），同时保留同名固有方法，
//!   使 `AbstractVarMeta` 值本身与 `dyn VarMeta` 都能直接调用；
//! - `extends MPPComponent` → Java 的 `MPPComponent` 只提供 4 个读取助手
//!   （`readByte` / `readShort` / `readInt` / `readByteArray`，均接收 `InputStream`），
//!   Rust 侧已把它落地为 [`crate::mpp::mpp_component::MPPComponent`]（输入源统一为
//!   `&[u8]`）；本文件的 [`read_byte`] / [`read_short`] / [`read_int`] 三个 crate 内可见的
//!   自由函数**直接转发**到它，供 `VarMeta9` / `VarMeta12` / `Var2Data` 沿用既有引用路径；
//! - `VarMeta9 / VarMeta12 extends AbstractVarMeta` → 组合 + 转发（见各自文件）。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `protected int m_itemCount` / `protected int m_dataSize` / `protected Map<...> m_table` | 同名的 `pub(crate)` 字段 | Java 的子类直接读写这些受保护字段，Rust 侧以同 crate 可见字段等价还原 |
//! | `TreeMap<Integer, Map<Integer, Integer>>` | `BTreeMap<i32, BTreeMap<i32, i32>>` | `getUniqueIdentifierArray` / `getUniqueIdentifierSet` / `toString` 都依赖 `TreeMap` 的升序遍历顺序，故用有序的 `BTreeMap`（而非约定的 `HashMap`） |
//! | `Integer` 参数（可空） | `Option<i32>` | 遵循「`null` → `Option<T>`」约定 |
//! | `Set<Integer>` | `BTreeSet<i32>` | Rust 侧返回升序集合副本，而非 `TreeMap.keySet()` 的实时视图 |
//! | `FieldMap` 参数 | `Option<&dyn VarDataKeyResolver>` | `FieldMap` 已落地（[`crate::mpp::field_map::FieldMap`] 并已 `impl VarDataKeyResolver`），但本文件仍以 [`VarDataKeyResolver`] 取参：Java 的 `getFieldTypeFromVarDataKey` 返回 `FieldType` 接口，而本 crate 的 `FieldType` trait 带 `Option<Self>` 返回值、不是对象安全类型，无法以 trait 对象形式返回，故只声明 `toString` 所需的最小能力（返回字段显示名） |
//! | `toString()` | `impl Display` | Java 的 `toString()` 内部调用 `toString(null)` |
//! | 读取越界抛 `EOFException` / `IndexOutOfBoundsException` | 返回 `None`、提前结束解析 | 遵循「库代码不 panic」约定，见 [`read_int`] |
//!
//! 跳过的 Java 方法：无（`MPPComponent.readByteArray` 由调用方直接切片实现，
//! 因为 Rust 侧的 `&[u8]` 本身就是数组）。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use crate::mpp::mpp_component::MPPComponent;
use crate::mpp::var_meta::{VarDataKeyResolver, VarMeta};

/// 从字节切片中按小端读取一个 4 字节整数并前移游标。
///
/// 对应 Java: `MPPComponent.readInt(InputStream)`（Rust 特有：输入源由 `InputStream` 改为 `&[u8]`）
///
/// 实现转发到已落地的 [`MPPComponent::read_int`]，本文件不再复刻一份字节解析逻辑；
/// 保留本函数是为了让 `VarMeta9` / `VarMeta12` / `Var2Data` 沿用「按模块取读取助手」的
/// 既有引用路径（`crate::mpp::abstract_var_meta::read_int`）。
///
/// 剩余字节不足 4 个时 Java 抛 `EOFException`，Rust 侧返回 `None`（游标不动），由调用方决定
/// 提前结束解析的方式 —— 绝不 panic。
pub(crate) fn read_int(data: &[u8], position: &mut usize) -> Option<i32> {
    MPPComponent::read_int(data, position)
}

/// 从字节切片中按小端读取一个 2 字节整数并前移游标。
///
/// 对应 Java: `MPPComponent.readShort(InputStream)`（Rust 特有：输入源由 `InputStream` 改为 `&[u8]`）
///
/// 实现转发到 [`MPPComponent::read_short`]；返回值为 `0..=65535`，
/// 剩余字节不足 2 个时返回 `None`（Java 抛 `EOFException`）。
pub(crate) fn read_short(data: &[u8], position: &mut usize) -> Option<i32> {
    MPPComponent::read_short(data, position)
}

/// 从字节切片中读取一个无符号字节（`0..=255`）并前移游标。
///
/// 对应 Java: `MPPComponent.readByte(InputStream)`（Rust 特有：输入源由 `InputStream` 改为 `&[u8]`）
///
/// 实现转发到 [`MPPComponent::read_byte`]；到达末尾时返回 `None`（Java 抛 `EOFException`）。
pub(crate) fn read_byte(data: &[u8], position: &mut usize) -> Option<i32> {
    MPPComponent::read_byte(data, position)
}

/// 变长数据块的元数据（`VarMeta` 的基类实现）。
///
/// 对应 Java: `abstract class AbstractVarMeta extends MPPComponent implements VarMeta`
pub struct AbstractVarMeta {
    /// 条目数。
    ///
    /// 对应 Java 字段: `protected int m_itemCount;`
    ///
    /// `pub(crate)`：Java 的子类直接写该受保护字段（见文件头）。
    pub(crate) item_count: i32,

    /// `Var2Data` 块的数据区大小。
    ///
    /// 对应 Java 字段: `protected int m_dataSize;`
    pub(crate) data_size: i32,

    /// 排序后的条目偏移数组。
    ///
    /// 对应 Java 字段: `private int[] m_offsets;`
    offsets: Vec<i32>,

    /// 「唯一 ID → 数据类型 → 偏移」索引。
    ///
    /// 对应 Java 字段: `protected final Map<Integer, Map<Integer, Integer>> m_table = new TreeMap<>();`
    pub(crate) table: BTreeMap<i32, BTreeMap<i32, i32>>,
}

impl AbstractVarMeta {
    /// 构造一个空元数据（Rust 特有：对应 Java 的隐式 `protected AbstractVarMeta()`，
    /// 子类随后直接写受保护字段，见文件头）。
    pub fn new() -> Self {
        Self {
            item_count: 0,
            data_size: 0,
            offsets: Vec::new(),
            table: BTreeMap::new(),
        }
    }

    /// 取条目数。
    ///
    /// 对应 Java: `@Override public int getItemCount()`
    pub fn get_item_count(&self) -> i32 {
        self.item_count
    }

    /// 取数据区大小。
    ///
    /// 对应 Java: `@Override public int getDataSize()`
    pub fn get_data_size(&self) -> i32 {
        self.data_size
    }

    /// 取全部出现过数据的唯一 ID（升序，与 Java 的 `TreeMap.keySet()` 顺序一致）。
    ///
    /// 对应 Java: `@Override public Integer[] getUniqueIdentifierArray()`
    pub fn get_unique_identifier_array(&self) -> Vec<i32> {
        self.table.keys().copied().collect()
    }

    /// 取全部出现过数据的唯一 ID 集合（升序）。
    ///
    /// 对应 Java: `@Override public Set<Integer> getUniqueIdentifierSet()`
    pub fn get_unique_identifier_set(&self) -> BTreeSet<i32> {
        self.table.keys().copied().collect()
    }

    /// 按「唯一 ID + 数据类型」取偏移。
    ///
    /// 对应 Java: `@Override public Integer getOffset(Integer id, Integer type)`
    ///
    /// Java 对 `type == null` 返回 `null`；对 `id == null` 会因 `TreeMap.get(null)` 抛 NPE，
    /// Rust 侧两种情形都返回 `None`。
    pub fn get_offset(&self, id: Option<i32>, type_: Option<i32>) -> Option<i32> {
        let id = id?;
        let type_ = type_?;

        // Java: Map<Integer, Integer> map = m_table.get(id);
        //       if (map != null && type != null) { result = map.get(type); }
        self.table.get(&id).and_then(|map| map.get(&type_).copied())
    }

    /// 设置排序后的偏移数组（供子类解析完成后调用）。
    ///
    /// 对应 Java: `protected void setOffsets(int[] offsets)`
    pub fn set_offsets(&mut self, offsets: Vec<i32>) {
        self.offsets = offsets;
    }

    /// 取排序后的偏移数组。
    ///
    /// 对应 Java: `@Override public int[] getOffsets()`
    pub fn get_offsets(&self) -> &[i32] {
        &self.offsets
    }

    /// 取指定唯一 ID 下定义过的全部数据类型（升序）。
    ///
    /// 对应 Java: `@Override public Set<Integer> getTypes(Integer id)`
    ///
    /// 该 ID 不存在（或为 `null`）时返回空集合。
    pub fn get_types(&self, id: Option<i32>) -> BTreeSet<i32> {
        // Java: Map<Integer, Integer> map = m_table.get(id);
        //       result = map != null ? map.keySet() : new HashSet<>();
        id.and_then(|id| self.table.get(&id))
            .map(|map| map.keys().copied().collect())
            .unwrap_or_default()
    }

    /// 判断索引中是否存在给定唯一 ID。
    ///
    /// 对应 Java: `@Override public boolean containsKey(Integer key)`
    ///
    /// Java 对 `null` 会因 `TreeMap.containsKey(null)` 抛 NPE，Rust 侧返回 `false`。
    pub fn contains_key(&self, key: Option<i32>) -> bool {
        key.is_some_and(|key| self.table.contains_key(&key))
    }

    /// 以调试用文本导出本块内容（`field_map` 为 `None` 时不翻译字段名）。
    ///
    /// 对应 Java: `@Override public String toString(FieldMap fieldMap)`
    pub fn to_string_with_field_map(&self, field_map: Option<&dyn VarDataKeyResolver>) -> String {
        let mut result = String::new();

        // Java: pw.println("BEGIN: VarMeta"); pw.println("   Item count: " + m_itemCount); ...
        result.push_str("BEGIN: VarMeta\n");
        result.push_str(&format!("   Item count: {}\n", self.item_count));
        result.push_str(&format!("   Data size: {}\n", self.data_size));

        for (unique_id, map) in &self.table {
            result.push_str(&format!("   Entries for Unique ID: {unique_id}\n"));

            for (type_, offset) in map {
                // Java: FieldType fieldType = fieldMap == null ? null : fieldMap.getFieldTypeFromVarDataKey(entry.getKey());
                let field_type = field_map
                    .and_then(|field_map| field_map.get_field_type_from_var_data_key(*type_));

                // Java: pw.println("      Type=" + (fieldType == null ? entry.getKey() : fieldType) + " Offset=" + entry.getValue());
                let label = match field_type {
                    Some(name) => name,
                    None => type_.to_string(),
                };
                result.push_str(&format!("      Type={label} Offset={offset}\n"));
            }
        }

        // Java: pw.println("END: VarMeta"); pw.println();
        result.push_str("END: VarMeta\n\n");
        result
    }
}

impl VarMeta for AbstractVarMeta {
    /// 转发到固有方法。
    ///
    /// 对应 Java: `@Override public int getItemCount()`
    fn get_item_count(&self) -> i32 {
        AbstractVarMeta::get_item_count(self)
    }

    /// 转发到固有方法。
    ///
    /// 对应 Java: `@Override public int getDataSize()`
    fn get_data_size(&self) -> i32 {
        AbstractVarMeta::get_data_size(self)
    }

    /// 转发到固有方法。
    ///
    /// 对应 Java: `@Override public Integer[] getUniqueIdentifierArray()`
    fn get_unique_identifier_array(&self) -> Vec<i32> {
        AbstractVarMeta::get_unique_identifier_array(self)
    }

    /// 转发到固有方法。
    ///
    /// 对应 Java: `@Override public Set<Integer> getUniqueIdentifierSet()`
    fn get_unique_identifier_set(&self) -> BTreeSet<i32> {
        AbstractVarMeta::get_unique_identifier_set(self)
    }

    /// 转发到固有方法。
    ///
    /// 对应 Java: `@Override public Integer getOffset(Integer id, Integer type)`
    fn get_offset(&self, id: Option<i32>, type_: Option<i32>) -> Option<i32> {
        AbstractVarMeta::get_offset(self, id, type_)
    }

    /// 转发到固有方法。
    ///
    /// 对应 Java: `@Override public int[] getOffsets()`
    fn get_offsets(&self) -> &[i32] {
        AbstractVarMeta::get_offsets(self)
    }

    /// 转发到固有方法。
    ///
    /// 对应 Java: `@Override public Set<Integer> getTypes(Integer id)`
    fn get_types(&self, id: Option<i32>) -> BTreeSet<i32> {
        AbstractVarMeta::get_types(self, id)
    }

    /// 转发到固有方法。
    ///
    /// 对应 Java: `@Override public boolean containsKey(Integer key)`
    fn contains_key(&self, key: Option<i32>) -> bool {
        AbstractVarMeta::contains_key(self, key)
    }

    /// 转发到固有方法。
    ///
    /// 对应 Java: `@Override public String toString(FieldMap fieldMap)`
    fn to_string_with_field_map(&self, field_map: Option<&dyn VarDataKeyResolver>) -> String {
        AbstractVarMeta::to_string_with_field_map(self, field_map)
    }
}

impl Display for AbstractVarMeta {
    /// 输出与 Java 的 `toString()` 相同的文本（内部调用 `toString(null)`）。
    ///
    /// 对应 Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_with_field_map(None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 空元数据的默认值。
    #[test]
    fn empty_meta_has_zeroed_defaults() {
        let meta = AbstractVarMeta::new();

        assert_eq!(meta.get_item_count(), 0);
        assert_eq!(meta.get_data_size(), 0);
        assert!(meta.get_offsets().is_empty());
        assert!(meta.get_unique_identifier_array().is_empty());
        assert!(meta.get_unique_identifier_set().is_empty());
        assert!(!meta.contains_key(Some(1)));
        assert_eq!(meta.get_offset(Some(1), Some(2)), None);
        assert!(meta.get_types(Some(1)).is_empty());
    }

    /// 索引的排序与查询语义（与 Java 的 `TreeMap` 一致）。
    #[test]
    fn table_lookup_and_ordering_match_java_treemap() {
        let mut meta = AbstractVarMeta::new();
        meta.item_count = 3;
        meta.data_size = 64;
        // 故意乱序插入，验证输出顺序为升序
        meta.table.insert(9, BTreeMap::from([(0x12, 40)]));
        meta.table.insert(1, BTreeMap::from([(0x12, 4), (0x34, 8)]));
        meta.set_offsets(vec![4, 8, 40]);

        assert_eq!(meta.get_item_count(), 3);
        assert_eq!(meta.get_data_size(), 64);
        assert_eq!(meta.get_unique_identifier_array(), vec![1, 9]);
        assert_eq!(
            meta.get_unique_identifier_set(),
            BTreeSet::from([1, 9])
        );
        assert_eq!(meta.get_offsets(), [4, 8, 40]);
        assert_eq!(meta.get_offset(Some(1), Some(0x34)), Some(8));
        assert_eq!(meta.get_offset(Some(1), Some(0x99)), None);
        assert_eq!(meta.get_offset(Some(7), Some(0x12)), None);
        // Java: type == null → null；id == null → NPE（Rust 侧退化为 None）
        assert_eq!(meta.get_offset(Some(1), None), None);
        assert_eq!(meta.get_offset(None, Some(0x12)), None);
        assert_eq!(meta.get_types(Some(1)), BTreeSet::from([0x12, 0x34]));
        assert!(meta.contains_key(Some(9)));
        assert!(!meta.contains_key(Some(10)));
        assert!(!meta.contains_key(None));
    }

    /// `toString()` / `toString(FieldMap)` 的文本格式。
    #[test]
    fn to_string_matches_java_format() {
        let mut meta = AbstractVarMeta::new();
        meta.item_count = 1;
        meta.data_size = 8;
        meta.table.insert(7, BTreeMap::from([(0x12, 4)]));
        meta.set_offsets(vec![4]);

        assert_eq!(
            meta.to_string(),
            "BEGIN: VarMeta\n   Item count: 1\n   Data size: 8\n   \
             Entries for Unique ID: 7\n      Type=18 Offset=4\nEND: VarMeta\n\n"
        );

        // field_map 提供字段名时，类型标识被替换成字段显示名
        struct Resolver;
        impl VarDataKeyResolver for Resolver {
            fn get_field_type_from_var_data_key(&self, key: i32) -> Option<String> {
                if key == 0x12 {
                    Some("Notes".to_owned())
                } else {
                    None
                }
            }
        }

        assert!(meta
            .to_string_with_field_map(Some(&Resolver))
            .contains("      Type=Notes Offset=4\n"));
    }

    /// 读取助手在越界时返回 `None` 而不是 panic（Java 抛 `EOFException`）。
    #[test]
    fn read_helpers_return_none_past_the_end() {
        let data = [0x01u8, 0x02, 0x03];

        let mut position = 0;
        assert_eq!(read_byte(&data, &mut position), Some(1));
        assert_eq!(position, 1);

        // 只剩 2 字节，读 4 字节整数失败且游标不动
        assert_eq!(read_int(&data, &mut position), None);
        assert_eq!(position, 1);

        assert_eq!(read_short(&data, &mut position), Some(0x0302));
        assert_eq!(position, 3);
        assert_eq!(read_byte(&data, &mut position), None);
        assert_eq!(position, 3);
    }

    /// 去重回归：本文件的读取助手与 [`MPPComponent`] 的同名方法取值、游标推进完全一致。
    ///
    /// 这两个模块曾各存一份等价实现（另有 `props.rs` 的 `InputStreamCursor`），
    /// 如今前者**转发**到后者；本测试把「两边行为一致」锁死。
    #[test]
    fn read_helpers_match_mpp_component() {
        let data = [0xFFu8, 0x34, 0x12, 0x78, 0x56, 0x34, 0x12, 0x01];

        let mut helper_position = 0usize;
        let mut component_position = 0usize;

        assert_eq!(
            read_byte(&data, &mut helper_position),
            MPPComponent::read_byte(&data, &mut component_position)
        );
        assert_eq!(helper_position, component_position);

        assert_eq!(
            read_short(&data, &mut helper_position),
            MPPComponent::read_short(&data, &mut component_position)
        );
        assert_eq!(helper_position, component_position);

        assert_eq!(
            read_int(&data, &mut helper_position),
            MPPComponent::read_int(&data, &mut component_position)
        );
        assert_eq!(helper_position, component_position);
        // 1（byte）+ 2（short）+ 4（int）= 7
        assert_eq!(helper_position, 7);

        // 越界：两边同样返回 None，且都不移动游标
        let short = [0x01u8];
        let mut helper_position = 1usize;
        let mut component_position = 1usize;
        assert_eq!(
            read_int(&short, &mut helper_position),
            MPPComponent::read_int(&short, &mut component_position)
        );
        assert_eq!(helper_position, component_position);
        assert_eq!(helper_position, 1);
    }
}
