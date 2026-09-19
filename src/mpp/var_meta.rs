//! 对应 Java: `org/mpxj/mpp/VarMeta.java`
//!
//! VarMeta 系列（`VarMeta9` / `VarMeta12`）的公共接口：描述 `Var2Data` 变长数据块的布局
//! ——条目数、数据区大小、以及「唯一 ID → 数据类型 → 偏移」索引。
//!
//! # 接口的还原
//!
//! Java: `interface VarMeta`，由 `abstract class AbstractVarMeta implements VarMeta` 实现，
//! `VarMeta9` / `VarMeta12` 再继承 `AbstractVarMeta`。Rust 没有继承，按下列方式还原：
//!
//! | Java | Rust |
//! |---|---|
//! | `interface VarMeta` | 本文件的 [`VarMeta`] trait |
//! | `abstract class AbstractVarMeta implements VarMeta` | [`crate::mpp::abstract_var_meta::AbstractVarMeta`] 提供 `impl VarMeta` |
//! | `VarMeta9 / VarMeta12 extends AbstractVarMeta` | 各自**组合**一个 `AbstractVarMeta`，并把全部方法**转发**给它 |
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `Integer` 参数（可空） | `Option<i32>` | 遵循「`null` → `Option<T>`」约定；Java 对 `null` 的越界行为（如 `TreeMap.get(null)` 抛 NPE）在 Rust 侧退化为 `None` / `false` |
//! | `Integer[] getUniqueIdentifierArray()` | `Vec<i32>` | 元素顺序与 Java 的 `TreeMap.keySet()` 一致（升序） |
//! | `Set<Integer> getUniqueIdentifierSet()` / `Set<Integer> getTypes(Integer)` | `BTreeSet<i32>` | Java 返回 `TreeMap.keySet()` 的升序**实时视图**，Rust 返回升序集合副本 |
//! | `int[] getOffsets()` | `&[i32]` | Java 返回内部数组，Rust 返回内部切片的借用（避免复制；调用方无法改写） |
//! | `String toString(FieldMap fieldMap)` | `to_string_with_field_map(Option<&dyn VarDataKeyResolver>)` | `FieldMap` 尚未移植，改用只声明所需能力的 [`VarDataKeyResolver`]；`null` → `None` |
//!
//! 跳过的 Java 方法：无。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本接口未使用这些机制。

use std::collections::BTreeSet;

/// 「变长数据键 → 字段类型名」的查询接口（Rust 特有）。
///
/// 对应 Java: `org.mpxj.mpp.FieldMap#getFieldTypeFromVarDataKey(Integer key)`
///
/// Java 的 `AbstractVarMeta#toString(FieldMap)` 仅用它把 VarData 键翻译成字段名（调试输出）。
/// Rust 侧 `FieldMap` 尚未移植（也不在 `mpp/mod.rs` 的模块清单内），故这里只声明该方法所需的
/// 最小能力，避免 `AbstractVarMeta` 依赖一个不存在的模块：
///
/// - Java 返回 `FieldType`（`org.mpxj.FieldType` 接口），但 Rust 的 `FieldType` trait
///   带 `Option<Self>` 返回值、不是对象安全类型，无法以 trait 对象形式返回，故改返回**字段显示名**；
/// - 返回 `None` 表示 Java 的 `null`（该键不是 VarData 字段）。
pub trait VarDataKeyResolver {
    /// 把变长数据键翻译成字段显示名。
    ///
    /// 对应 Java: `FieldType getFieldTypeFromVarDataKey(Integer key)`
    fn get_field_type_from_var_data_key(&self, key: i32) -> Option<String>;
}

/// 变长数据块的元数据。
///
/// 对应 Java: `interface VarMeta`
pub trait VarMeta {
    /// 取 `Var2Data` 块中的条目数。
    ///
    /// 对应 Java: `int getItemCount()`
    fn get_item_count(&self) -> i32;

    /// 取 `Var2Data` 块的数据区大小。
    ///
    /// 对应 Java: `int getDataSize()`
    fn get_data_size(&self) -> i32;

    /// 取全部出现过数据的唯一 ID（升序）。
    ///
    /// 对应 Java: `Integer[] getUniqueIdentifierArray()`
    fn get_unique_identifier_array(&self) -> Vec<i32>;

    /// 取全部出现过数据的唯一 ID 集合（升序）。
    ///
    /// 对应 Java: `Set<Integer> getUniqueIdentifierSet()`
    fn get_unique_identifier_set(&self) -> BTreeSet<i32>;

    /// 按「唯一 ID + 数据类型」取该条目在 `Var2Data` 块中的偏移。
    ///
    /// 对应 Java: `Integer getOffset(Integer id, Integer type)`
    ///
    /// `id` / `type` 为 `None`（Java 的 `null`）或索引中不存在时返回 `None`。
    fn get_offset(&self, id: Option<i32>, type_: Option<i32>) -> Option<i32>;

    /// 取排序后的偏移数组。
    ///
    /// 对应 Java: `int[] getOffsets()`
    fn get_offsets(&self) -> &[i32];

    /// 取指定唯一 ID 下定义过的全部数据类型（升序）。
    ///
    /// 对应 Java: `Set<Integer> getTypes(Integer id)`
    ///
    /// 该 ID 不存在时返回空集合（与 Java 一致）。
    fn get_types(&self, id: Option<i32>) -> BTreeSet<i32>;

    /// 判断索引中是否存在给定唯一 ID。
    ///
    /// 对应 Java: `boolean containsKey(Integer key)`
    ///
    /// Java 对 `null` 会抛 NPE（`TreeMap.containsKey(null)`），Rust 侧返回 `false`。
    fn contains_key(&self, key: Option<i32>) -> bool;

    /// 以调试用文本导出本块内容。
    ///
    /// 对应 Java: `String toString(FieldMap fieldMap)`
    ///
    /// `field_map` 为 `None`（Java 的 `null`）时只输出数值型类型标识。
    fn to_string_with_field_map(&self, field_map: Option<&dyn VarDataKeyResolver>) -> String;
}
