//! 对应 Java: `org/mpxj/CustomFieldLookupTable.java`

use std::fmt::{self, Display, Formatter};
use uuid::Uuid;

use crate::custom_field_value_data_type::CustomFieldValueDataType;
use crate::field_value::FieldValue;
use crate::list_with_callbacks::ListWithCallbacks;

/// 自定义字段的取值查找表。
///
/// 对应 Java: `public class CustomFieldLookupTable extends ListWithCallbacks<CustomFieldValueItem>`
///
/// # 继承体系的还原方式
///
/// Java 侧继承 `ListWithCallbacks<CustomFieldValueItem>`；Rust 没有继承，改为**组合**：
/// 内部持有 [`ListWithCallbacks<CustomFieldValueItem>`]，并把列表操作转发出去
/// （[`add`](CustomFieldLookupTable::add) / [`get`](CustomFieldLookupTable::get) /
/// [`remove`](CustomFieldLookupTable::remove) 等）。
///
/// Java 的 `CustomFieldLookupTable` 没有再覆写 `added` / `removed` / `replaced` 钩子，
/// 因此三个回调保持未注册状态（等价于 Java 的空实现）；持有方可用
/// [`ListWithCallbacks::set_added_callback`] 等自行注册。
///
/// # 与 Java 的差异：元素类型用占位实现
///
/// Java 的元素类型是 `org.mpxj.mpp.CustomFieldValueItem`。该类型位于 `org.mpxj.mpp` 包，
/// 其 Rust 模块（`src/mpp/`）尚未落地且未在 [`crate`] 的模块清单中声明，本文件无法引用。
/// 为避免引用不存在的模块，本文件内提供了**同名占位实现** [`CustomFieldValueItem`]
/// （字段与 Java 一一对应）；待 mpp 模块落地后，应把占位实现移出并改为 `use` 正式类型。
///
/// 未移植的 Java 方法：无。
#[derive(Debug)]
pub struct CustomFieldLookupTable {
    /// 查找表内容。
    ///
    /// 对应 Java: `ListWithCallbacks<CustomFieldValueItem>` 继承而来的列表存储。
    entries: ListWithCallbacks<CustomFieldValueItem>,
    /// 查找表 GUID。
    ///
    /// 对应 Java: `private UUID m_guid;`
    m_guid: Option<Uuid>,
    /// 企业标志。
    ///
    /// 对应 Java: `private boolean m_enterprise;`
    m_enterprise: bool,
    /// 显示缩进标志（Java 初始值为 `true`）。
    ///
    /// 对应 Java: `private boolean m_showIndent = true;`
    m_show_indent: bool,
    /// 资源替代启用标志。
    ///
    /// 对应 Java: `private boolean m_resourceSubstitutionEnabled;`
    m_resource_substitution_enabled: bool,
    /// 仅叶子节点标志。
    ///
    /// 对应 Java: `private boolean m_leafOnly;`
    m_leaf_only: bool,
    /// 所有层级必填标志。
    ///
    /// 对应 Java: `private boolean m_allLevelsRequired;`
    m_all_levels_required: bool,
    /// 仅允许表内取值标志（Java 初始值为 `true`）。
    ///
    /// 对应 Java: `private boolean m_onlyTableValuesAllowed = true;`
    m_only_table_values_allowed: bool,
}

impl CustomFieldLookupTable {
    /// 构造方法。
    ///
    /// 对应 Java: `CustomFieldLookupTable()`（隐式默认构造方法，`m_showIndent` 与
    /// `m_onlyTableValuesAllowed` 初始为 `true`）。
    pub fn new() -> Self {
        Self {
            entries: ListWithCallbacks::new(),
            m_guid: None,
            m_enterprise: false,
            m_show_indent: true,
            m_resource_substitution_enabled: false,
            m_leaf_only: false,
            m_all_levels_required: false,
            m_only_table_values_allowed: true,
        }
    }

    /// 取查找表 GUID。
    ///
    /// 对应 Java: `public UUID getGUID()`
    pub fn get_guid(&self) -> Option<Uuid> {
        self.m_guid
    }

    /// 设置查找表 GUID。
    ///
    /// 对应 Java: `public void setGUID(UUID guid)`
    pub fn set_guid(&mut self, guid: Option<Uuid>) {
        self.m_guid = guid;
    }

    /// 取企业标志。
    ///
    /// 对应 Java: `public boolean getEnterprise()`
    pub fn get_enterprise(&self) -> bool {
        self.m_enterprise
    }

    /// 设置企业标志。
    ///
    /// 对应 Java: `public void setEnterprise(boolean enterprise)`
    pub fn set_enterprise(&mut self, enterprise: bool) {
        self.m_enterprise = enterprise;
    }

    /// 取显示缩进标志。
    ///
    /// 对应 Java: `public boolean getShowIndent()`
    pub fn get_show_indent(&self) -> bool {
        self.m_show_indent
    }

    /// 设置显示缩进标志。
    ///
    /// 对应 Java: `public void setShowIndent(boolean showIndent)`
    pub fn set_show_indent(&mut self, show_indent: bool) {
        self.m_show_indent = show_indent;
    }

    /// 取资源替代启用标志。
    ///
    /// 对应 Java: `public boolean getResourceSubstitutionEnabled()`
    pub fn get_resource_substitution_enabled(&self) -> bool {
        self.m_resource_substitution_enabled
    }

    /// 设置资源替代启用标志。
    ///
    /// 对应 Java: `public void setResourceSubstitutionEnabled(boolean resourceSubstitutionEnabled)`
    pub fn set_resource_substitution_enabled(&mut self, resource_substitution_enabled: bool) {
        self.m_resource_substitution_enabled = resource_substitution_enabled;
    }

    /// 取仅叶子节点标志。
    ///
    /// 对应 Java: `public boolean getLeafOnly()`
    pub fn get_leaf_only(&self) -> bool {
        self.m_leaf_only
    }

    /// 设置仅叶子节点标志。
    ///
    /// 对应 Java: `public void setLeafOnly(boolean leafOnly)`
    pub fn set_leaf_only(&mut self, leaf_only: bool) {
        self.m_leaf_only = leaf_only;
    }

    /// 取所有层级必填标志。
    ///
    /// 对应 Java: `public boolean getAllLevelsRequired()`
    pub fn get_all_levels_required(&self) -> bool {
        self.m_all_levels_required
    }

    /// 设置所有层级必填标志。
    ///
    /// 对应 Java: `public void setAllLevelsRequired(boolean allLevelsRequired)`
    pub fn set_all_levels_required(&mut self, all_levels_required: bool) {
        self.m_all_levels_required = all_levels_required;
    }

    /// 取仅允许表内取值标志。
    ///
    /// 对应 Java: `public boolean getOnlyTableValuesAllowed()`
    pub fn get_only_table_values_allowed(&self) -> bool {
        self.m_only_table_values_allowed
    }

    /// 设置仅允许表内取值标志。
    ///
    /// 对应 Java: `public void setOnlyTableValuesAllowed(boolean onlyTableValuesAllowed)`
    pub fn set_only_table_values_allowed(&mut self, only_table_values_allowed: bool) {
        self.m_only_table_values_allowed = only_table_values_allowed;
    }

    /// 向查找表追加一个取值项。
    ///
    /// 对应 Java: `ListWithCallbacks.add(T e)`
    pub fn add(&mut self, item: CustomFieldValueItem) {
        self.entries.add(item);
    }

    /// 在指定下标插入一个取值项（越界返回 `false`）。
    ///
    /// 对应 Java: `ListWithCallbacks.add(int index, T element)`
    pub fn add_at(&mut self, index: usize, item: CustomFieldValueItem) -> bool {
        self.entries.add_at(index, item)
    }

    /// 按下标取取值项。
    ///
    /// 对应 Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<&CustomFieldValueItem> {
        self.entries.get(index)
    }

    /// 替换指定下标的取值项，返回被替换的旧元素（越界返回 `None`）。
    ///
    /// 对应 Java: `ListWithCallbacks.set(int index, T element)`
    pub fn set(&mut self, index: usize, item: CustomFieldValueItem) -> Option<CustomFieldValueItem> {
        self.entries.set(index, item)
    }

    /// 移除指定下标的取值项并返回它（越界返回 `None`）。
    ///
    /// 对应 Java: `ListWithCallbacks.remove(int index)`
    pub fn remove(&mut self, index: usize) -> Option<CustomFieldValueItem> {
        self.entries.remove(index)
    }

    /// 清空查找表（不触发 `removed` 回调，与 Java 一致）。
    ///
    /// 对应 Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 取值项个数。
    ///
    /// 对应 Java: `ListWithCallbacks.size()`
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 查找表是否为空。
    ///
    /// 对应 Java: `ListWithCallbacks.isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 迭代查找表内的取值项。
    ///
    /// 对应 Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, CustomFieldValueItem> {
        self.entries.iter()
    }

    /// 以切片形式访问查找表内容。
    ///
    /// 对应 Java: `ListWithCallbacks` 的列表视图。
    pub fn as_slice(&self) -> &[CustomFieldValueItem] {
        self.entries.as_slice()
    }

    /// 按 unique ID 查找取值项，找不到时返回 `None`。
    ///
    /// Rust 侧新增的便捷查询：Java 的 `CustomFieldLookupTable` 只提供列表语义，没有按
    /// unique ID 查找的方法；MPP 读取时需要按 unique ID 定位取值项，故在此提供线性查找。
    pub fn get_by_unique_id(&self, unique_id: i32) -> Option<&CustomFieldValueItem> {
        self.entries
            .iter()
            .find(|item| item.get_unique_id() == unique_id)
    }
}

impl Default for CustomFieldLookupTable {
    /// 与 Java 的隐式默认构造方法一致（注意 `m_showIndent` 与
    /// `m_onlyTableValuesAllowed` 为 `true`，因此不能 `derive(Default)`）。
    fn default() -> Self {
        Self::new()
    }
}

/// 自定义字段的一个候选取值（占位实现）。
///
/// 对应 Java: `org.mpxj.mpp.CustomFieldValueItem`
///
/// **本类型是占位实现**：`org.mpxj.mpp` 的 Rust 模块尚未落地且未在 [`crate`] 的模块清单中
/// 声明，[`CustomFieldLookupTable`] 无法引用正式类型，故在此按 Java 字段一一对应地先行落地。
/// mpp 模块移植后，应删除本类型并改为 `use` 正式实现。
///
/// # 与 Java 的差异
///
/// - Java 的 `Object m_value` → `Option<FieldValue>`（对应 crate 对 Java `Object` 字段值的统一表达）；
/// - Java 的可空引用字段（`UUID` / `String` / `byte[]` / `Integer` / 枚举）→ `Option<_>`；
/// - Java 未覆写 `equals` / `hashCode`（引用相等）；Rust 侧只派生 `Debug` / `Clone`。
#[derive(Debug, Clone)]
pub struct CustomFieldValueItem {
    /// 取值项 unique ID。
    ///
    /// 对应 Java: `private final Integer m_uniqueID;`
    m_unique_id: i32,
    /// 取值项 GUID。
    ///
    /// 对应 Java: `private UUID m_guid;`
    m_guid: Option<Uuid>,
    /// 取值项的值。
    ///
    /// 对应 Java: `private Object m_value;`
    m_value: Option<FieldValue>,
    /// 取值项描述。
    ///
    /// 对应 Java: `private String m_description;`
    m_description: Option<String>,
    /// 未知数据块。
    ///
    /// 对应 Java: `private byte[] m_unknown;`
    m_unknown: Option<Vec<u8>>,
    /// 父级 unique ID。
    ///
    /// 对应 Java: `private Integer m_parentUniqueID;`
    m_parent_unique_id: Option<i32>,
    /// 取值类型。
    ///
    /// 对应 Java: `private CustomFieldValueDataType m_type;`
    m_type: Option<CustomFieldValueDataType>,
    /// 折叠标志。
    ///
    /// 对应 Java: `private boolean m_collapsed;`
    m_collapsed: bool,
}

impl CustomFieldValueItem {
    /// 构造方法。
    ///
    /// 对应 Java: `public CustomFieldValueItem(Integer uniqueID)`
    pub fn new(unique_id: i32) -> Self {
        Self {
            m_unique_id: unique_id,
            m_guid: None,
            m_value: None,
            m_description: None,
            m_unknown: None,
            m_parent_unique_id: None,
            m_type: None,
            m_collapsed: false,
        }
    }

    /// 取 unique ID。
    ///
    /// 对应 Java: `public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> i32 {
        self.m_unique_id
    }

    /// 设置取值。
    ///
    /// 对应 Java: `public void setValue(Object value)`
    pub fn set_value(&mut self, value: Option<FieldValue>) {
        self.m_value = value;
    }

    /// 取取值。
    ///
    /// 对应 Java: `public Object getValue()`
    pub fn get_value(&self) -> Option<&FieldValue> {
        self.m_value.as_ref()
    }

    /// 设置描述。
    ///
    /// 对应 Java: `public void setDescription(String description)`
    pub fn set_description(&mut self, description: Option<String>) {
        self.m_description = description;
    }

    /// 取描述。
    ///
    /// 对应 Java: `public String getDescription()`
    pub fn get_description(&self) -> Option<&str> {
        self.m_description.as_deref()
    }

    /// 设置未知数据块。
    ///
    /// 对应 Java: `public void setUnknown(byte[] unknown)`
    pub fn set_unknown(&mut self, unknown: Option<Vec<u8>>) {
        self.m_unknown = unknown;
    }

    /// 取未知数据块。
    ///
    /// 对应 Java: `public byte[] getUnknown()`
    pub fn get_unknown(&self) -> Option<&[u8]> {
        self.m_unknown.as_deref()
    }

    /// 设置父级 unique ID。
    ///
    /// 对应 Java: `public void setParentUniqueID(Integer id)`
    pub fn set_parent_unique_id(&mut self, id: Option<i32>) {
        self.m_parent_unique_id = id;
    }

    /// 取父级 unique ID。
    ///
    /// 对应 Java: `public Integer getParentUniqueID()`
    pub fn get_parent_unique_id(&self) -> Option<i32> {
        self.m_parent_unique_id
    }

    /// 取 GUID。
    ///
    /// 对应 Java: `public UUID getGUID()`
    pub fn get_guid(&self) -> Option<Uuid> {
        self.m_guid
    }

    /// 设置 GUID。
    ///
    /// 对应 Java: `public void setGUID(UUID guid)`
    pub fn set_guid(&mut self, guid: Option<Uuid>) {
        self.m_guid = guid;
    }

    /// 取取值类型。
    ///
    /// 对应 Java: `public CustomFieldValueDataType getType()`
    pub fn get_type(&self) -> Option<CustomFieldValueDataType> {
        self.m_type
    }

    /// 设置取值类型。
    ///
    /// 对应 Java: `public void setType(CustomFieldValueDataType type)`
    pub fn set_type(&mut self, value_type: Option<CustomFieldValueDataType>) {
        self.m_type = value_type;
    }

    /// 取折叠标志。
    ///
    /// 对应 Java: `public boolean getCollapsed()`
    pub fn get_collapsed(&self) -> bool {
        self.m_collapsed
    }

    /// 设置折叠标志。
    ///
    /// 对应 Java: `public void setCollapsed(boolean collapsed)`
    pub fn set_collapsed(&mut self, collapsed: bool) {
        self.m_collapsed = collapsed;
    }
}

impl Display for CustomFieldValueItem {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java:
    /// `String.format("[CustomFieldValueItem uniqueID=%d guid=%s parentId=%d value=%s description=%s]", ...)`
    ///
    /// `null` 字段按 Java 的 `%s` 规则输出 `null`（Java 侧 `%d` 遇 `null` 会抛异常，
    /// Rust 侧统一输出 `null`）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[CustomFieldValueItem uniqueID={} guid={} parentId={} value={} description={}]",
            self.m_unique_id,
            self.m_guid
                .map(|guid| guid.to_string())
                .unwrap_or_else(|| "null".to_owned()),
            self.m_parent_unique_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "null".to_owned()),
            self.m_value
                .as_ref()
                .map(|value| format!("{value:?}"))
                .unwrap_or_else(|| "null".to_owned()),
            self.m_description.as_deref().unwrap_or("null")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_flags_match_java_initial_values() {
        let table = CustomFieldLookupTable::new();

        assert!(table.get_show_indent());
        assert!(table.get_only_table_values_allowed());
        assert!(!table.get_enterprise());
        assert!(!table.get_resource_substitution_enabled());
        assert!(!table.get_leaf_only());
        assert!(!table.get_all_levels_required());
        assert!(table.get_guid().is_none());
    }

    #[test]
    fn flags_are_settable() {
        let mut table = CustomFieldLookupTable::new();
        let guid = Uuid::from_u128(1);

        table.set_guid(Some(guid));
        table.set_enterprise(true);
        table.set_show_indent(false);
        table.set_resource_substitution_enabled(true);
        table.set_leaf_only(true);
        table.set_all_levels_required(true);
        table.set_only_table_values_allowed(false);

        assert_eq!(table.get_guid(), Some(guid));
        assert!(table.get_enterprise());
        assert!(!table.get_show_indent());
        assert!(table.get_resource_substitution_enabled());
        assert!(table.get_leaf_only());
        assert!(table.get_all_levels_required());
        assert!(!table.get_only_table_values_allowed());
    }

    #[test]
    fn list_operations_and_lookup_by_unique_id() {
        let mut table = CustomFieldLookupTable::new();
        assert!(table.is_empty());

        let mut first = CustomFieldValueItem::new(10);
        first.set_description(Some("A".to_owned()));
        first.set_value(Some(FieldValue::String("A".to_owned())));
        let second = CustomFieldValueItem::new(20);

        table.add(first);
        table.add_at(1, second);

        assert_eq!(table.len(), 2);
        assert_eq!(table.iter().count(), 2);
        assert_eq!(table.as_slice().len(), 2);
        assert_eq!(
            table.get_by_unique_id(10).and_then(|item| item.get_description()),
            Some("A")
        );
        assert!(table.get_by_unique_id(30).is_none());

        // add_at 越界
        assert!(!table.add_at(5, CustomFieldValueItem::new(30)));
        assert_eq!(table.len(), 2);

        // set / remove
        let replaced = table.set(0, CustomFieldValueItem::new(30)).expect("下标存在");
        assert_eq!(replaced.get_unique_id(), 10);
        assert!(table.get_by_unique_id(30).is_some());

        let removed = table.remove(0).expect("下标存在");
        assert_eq!(removed.get_unique_id(), 30);
        assert!(table.get_by_unique_id(30).is_none());

        // 越界
        assert!(table.set(5, CustomFieldValueItem::new(40)).is_none());
        assert!(table.remove(5).is_none());

        table.clear();
        assert!(table.is_empty());
    }

    #[test]
    fn lookup_table_callbacks_can_be_registered() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let log = Rc::new(RefCell::new(Vec::new()));
        let mut table = CustomFieldLookupTable::new();

        let added = Rc::clone(&log);
        table
            .entries
            .set_added_callback(Some(Box::new(move |item: &CustomFieldValueItem| {
                added.borrow_mut().push(format!("added:{}", item.get_unique_id()));
            })));

        table.add(CustomFieldValueItem::new(7));

        assert_eq!(*log.borrow(), vec!["added:7".to_owned()]);
    }

    #[test]
    fn value_item_accessors_round_trip() {
        let mut item = CustomFieldValueItem::new(4);
        let guid = Uuid::from_u128(42);

        item.set_guid(Some(guid));
        item.set_value(Some(FieldValue::Integer(1)));
        item.set_description(Some("desc".to_owned()));
        item.set_unknown(Some(vec![1, 2, 3]));
        item.set_parent_unique_id(Some(2));
        item.set_type(Some(CustomFieldValueDataType::Number));
        item.set_collapsed(true);

        assert_eq!(item.get_unique_id(), 4);
        assert_eq!(item.get_guid(), Some(guid));
        assert_eq!(item.get_value(), Some(&FieldValue::Integer(1)));
        assert_eq!(item.get_description(), Some("desc"));
        assert_eq!(item.get_unknown(), Some(&[1, 2, 3][..]));
        assert_eq!(item.get_parent_unique_id(), Some(2));
        assert_eq!(item.get_type(), Some(CustomFieldValueDataType::Number));
        assert!(item.get_collapsed());
    }

    #[test]
    fn value_item_display_matches_java_format() {
        let mut item = CustomFieldValueItem::new(4);
        item.set_description(Some("desc".to_owned()));
        item.set_parent_unique_id(Some(2));

        assert_eq!(
            item.to_string(),
            "[CustomFieldValueItem uniqueID=4 guid=null parentId=2 value=null description=desc]"
        );
    }
}
