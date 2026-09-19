//! 对应 Java: `org/mpxj/CustomField.java`

use std::cell::RefCell;
use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::rc::Weak;

use crate::custom_field_container::CustomFieldContainer;
use crate::custom_field_lookup_table::CustomFieldLookupTable;
use crate::custom_field_value_mask::CustomFieldValueMask;
use crate::field_key::FieldKey;
use crate::field_type::FieldType;
use crate::field_type_class::FieldTypeClass;

/// 单个字段的配置信息。
///
/// 对应 Java: `public class CustomField implements Comparable<CustomField>`
///
/// # 字段映射与结构取舍
///
/// | Java | Rust |
/// |---|---|
/// | `private final FieldType m_field` | `FieldKey`（本 crate 对 `FieldType` 接口的运行时擦除，见 [`crate::field_key`]） |
/// | `private final CustomFieldContainer m_parent` | `Weak<RefCell<CustomFieldContainer>>`（`setAlias` 时回指父容器注册别名，用 `Weak` 避免 `Rc` 循环） |
/// | `private final CustomFieldLookupTable m_table` | [`CustomFieldLookupTable`] |
/// | `private final GraphicalIndicator m_indicator` | **未移植**（见下） |
/// | `private final List<CustomFieldValueMask> m_masks` | `Vec<CustomFieldValueMask>` |
/// | `private Integer m_uniqueID` | `Option<i32>` |
/// | `private String m_alias` | `Option<String>` |
///
/// # 被跳过的 Java 代码
///
/// - `private final GraphicalIndicator m_indicator` 与 `getGraphicalIndicator()`：
///   `GraphicalIndicator`（`org.mpxj.GraphicalIndicator`）不在本次移植范围，且 `src/lib.rs`
///   未声明对应模块，Rust 侧无法引用，故整体跳过。
/// - `getUniqueID()` 中「未显式设置时回落到 `FieldTypeHelper.getFieldID(m_field)`」的惰性求值：
///   `FieldTypeHelper.getFieldID` 依赖 `org.mpxj.mpp.MPPTaskField` 等 MPP 字段 ID 表，
///   这些类型尚未移植。Rust 侧的 [`CustomField::get_unique_id`] 只返回显式设置过的值，
///   未设置时返回 `None`（对应 Java 中「尚未缓存过 MPP 字段 ID」的状态）。
///
/// 未移植的 Java 方法：`getGraphicalIndicator()`（见上）。
pub struct CustomField {
    /// 该配置对应的字段类型。
    ///
    /// 对应 Java: `private final FieldType m_field;`
    m_field: FieldKey,
    /// 父容器。
    ///
    /// 对应 Java: `private final CustomFieldContainer m_parent;`
    m_parent: Weak<RefCell<CustomFieldContainer>>,
    /// 取值查找表。
    ///
    /// 对应 Java: `private final CustomFieldLookupTable m_table;`
    m_table: CustomFieldLookupTable,
    /// mask 定义。
    ///
    /// 对应 Java: `private final List<CustomFieldValueMask> m_masks;`
    m_masks: Vec<CustomFieldValueMask>,
    /// unique ID。
    ///
    /// 对应 Java: `private Integer m_uniqueID;`
    m_unique_id: Option<i32>,
    /// 字段别名。
    ///
    /// 对应 Java: `private String m_alias;`
    m_alias: Option<String>,
}

impl CustomField {
    /// 构造方法。
    ///
    /// 对应 Java: `public CustomField(FieldType field, CustomFieldContainer parent)`
    ///
    /// Java 的 `parent` 不可为 `null`；Rust 侧允许传入 `Weak::new()` 表示「无父容器」
    /// （此时 `set_alias` 不会向外注册别名），便于单独构造字段做测试。
    pub fn new(field: FieldKey, parent: Weak<RefCell<CustomFieldContainer>>) -> Self {
        Self {
            m_field: field,
            m_parent: parent,
            m_table: CustomFieldLookupTable::new(),
            m_masks: Vec::new(),
            m_unique_id: None,
            m_alias: None,
        }
    }

    /// 取该配置对应的字段类型。
    ///
    /// 对应 Java: `public FieldType getFieldType()`
    pub fn get_field_type(&self) -> FieldKey {
        self.m_field
    }

    /// 取取值查找表。
    ///
    /// 对应 Java: `public CustomFieldLookupTable getLookupTable()`
    pub fn get_lookup_table(&self) -> &CustomFieldLookupTable {
        &self.m_table
    }

    /// 以可变方式取取值查找表（Java 返回的对象本身可变）。
    ///
    /// 对应 Java: `public CustomFieldLookupTable getLookupTable()`
    pub fn get_lookup_table_mut(&mut self) -> &mut CustomFieldLookupTable {
        &mut self.m_table
    }

    /// 取字段别名。
    ///
    /// 对应 Java: `public String getAlias()`
    pub fn get_alias(&self) -> Option<&str> {
        self.m_alias.as_deref()
    }

    /// 设置字段别名，并在父容器中注册别名索引。
    ///
    /// 对应 Java: `public CustomField setAlias(String alias)`
    ///
    /// Java 侧返回 `this` 以支持链式调用；Rust 侧的实例通常被
    /// `Rc<RefCell<CustomField>>` 共享，故改为 `&mut self` 且不返回。
    ///
    /// Java 对 `null` 别名同样会调用 `registerAlias`；Rust 侧的别名索引键类型为
    /// `(FieldTypeClass, String)`，无法表达 `null`，因此只在别名非 `None` 时注册。
    pub fn set_alias(&mut self, alias: Option<String>) {
        if let Some(alias) = alias.as_deref() {
            if let Some(parent) = self.m_parent.upgrade() {
                parent.borrow_mut().register_alias(self.m_field, alias);
            }
        }

        self.m_alias = alias;
    }

    /// 取 unique ID。
    ///
    /// 对应 Java: `public Integer getUniqueID()`
    ///
    /// 见类型说明：Java 未显式设置时会回落到 MPP 字段 ID，Rust 侧在该回落机制移植前
    /// 只返回显式设置的值。
    pub fn get_unique_id(&self) -> Option<i32> {
        self.m_unique_id
    }

    /// 设置 unique ID。
    ///
    /// 对应 Java: `public CustomField setUniqueID(Integer uniqueID)`
    pub fn set_unique_id(&mut self, unique_id: Option<i32>) {
        self.m_unique_id = unique_id;
    }

    /// 取 mask 定义。
    ///
    /// 对应 Java: `public List<CustomFieldValueMask> getMasks()`
    pub fn get_masks(&self) -> &[CustomFieldValueMask] {
        &self.m_masks
    }

    /// 以可变方式取 mask 定义（Java 返回的列表本身可变，调用方可直接 `add`）。
    ///
    /// 对应 Java: `public List<CustomFieldValueMask> getMasks()`
    pub fn get_masks_mut(&mut self) -> &mut Vec<CustomFieldValueMask> {
        &mut self.m_masks
    }

    /// 排序键，对应 Java `compareTo` 中拼接的字符串：
    /// `fieldTypeClass.name() + "." + uniqueID + "." + alias`。
    fn sort_key(&self) -> String {
        format!(
            "{}.{}.{}",
            field_type_class_name(self.m_field.get_field_type_class()),
            option_to_java_string(self.m_unique_id),
            self.m_alias
                .clone()
                .unwrap_or_else(|| "null".to_owned())
        )
    }
}

/// 取 `FieldTypeClass` 的 Java 枚举名（`Enum.name()`），用于复刻 `compareTo` 的排序键。
fn field_type_class_name(type_class: FieldTypeClass) -> &'static str {
    match type_class {
        FieldTypeClass::Task => "TASK",
        FieldTypeClass::Resource => "RESOURCE",
        FieldTypeClass::Assignment => "ASSIGNMENT",
        FieldTypeClass::Constraint => "CONSTRAINT",
        FieldTypeClass::Project => "PROJECT",
        FieldTypeClass::Unknown => "UNKNOWN",
    }
}

/// 复刻 Java 字符串拼接中 `null` 的输出。
fn option_to_java_string(value: Option<i32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

impl PartialEq for CustomField {
    /// Java 未覆写 `equals`（使用引用相等），但实现了 `Comparable`。Rust 的 `Ord` 要求配套
    /// `Eq`，故按 `compareTo` 的排序键实现 —— 与 Java 中 `TreeSet` / `TreeMap` 用
    /// `compareTo` 判等的行为一致。
    fn eq(&self, other: &Self) -> bool {
        self.sort_key() == other.sort_key()
    }
}

impl Eq for CustomField {}

impl PartialOrd for CustomField {
    /// 对应 Java: `public int compareTo(CustomField f)`
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CustomField {
    /// 对应 Java: `public int compareTo(CustomField f)`
    fn cmp(&self, other: &Self) -> Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl Display for CustomField {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java: `"[CustomField field=" + m_field + " alias=" + m_alias + "]"`；其中 `m_field`
    /// 是枚举，输出常量名（如 `"TEXT1"`），`m_alias` 为 `null` 时输出 `null`。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[CustomField field={} alias={}]",
            self.m_field.name(),
            self.m_alias.as_deref().unwrap_or("null")
        )
    }
}

impl std::fmt::Debug for CustomField {
    /// Java 未覆写 `toString()` 以外的调试输出；Rust 侧为便于诊断提供 `Debug`。
    ///
    /// 刻意**不**输出 `m_parent`：子对象经 `Weak` 回指父容器，父容器又持有全部子对象，
    /// 打印父容器会造成无限递归。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomField")
            .field("field", &self.m_field.name())
            .field("unique_id", &self.m_unique_id)
            .field("alias", &self.m_alias)
            .field("table", &self.m_table)
            .field("masks", &self.m_masks)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::custom_field_value_data_type::CustomFieldValueDataType;
    use crate::resource_field::ResourceField;
    use crate::task_field::TaskField;

    /// 构造一个「无父容器」的字段（Java 的 `m_parent` 必非空，Rust 侧用悬空 `Weak` 代替）。
    fn standalone(field: FieldKey) -> CustomField {
        CustomField::new(field, Weak::new())
    }

    #[test]
    fn accessors_round_trip() {
        let mut field = standalone(FieldKey::Task(TaskField::Text1));

        assert_eq!(field.get_field_type(), FieldKey::Task(TaskField::Text1));
        assert_eq!(field.get_alias(), None);
        assert_eq!(field.get_unique_id(), None);
        assert!(field.get_masks().is_empty());
        assert!(field.get_lookup_table().is_empty());

        field.set_alias(Some("Alias".to_owned()));
        field.set_unique_id(Some(42));

        assert_eq!(field.get_alias(), Some("Alias"));
        assert_eq!(field.get_unique_id(), Some(42));
    }

    #[test]
    fn lookup_table_and_masks_are_mutable() {
        let mut field = standalone(FieldKey::Task(TaskField::Number1));

        field
            .get_lookup_table_mut()
            .add(crate::custom_field_lookup_table::CustomFieldValueItem::new(1));
        assert_eq!(field.get_lookup_table().len(), 1);

        field.get_masks_mut().push(CustomFieldValueMask::new(
            10,
            1,
            Some("-".to_owned()),
            Some(CustomFieldValueDataType::Text),
        ));
        assert_eq!(field.get_masks().len(), 1);
        assert_eq!(field.get_masks()[0].get_length(), 10);
    }

    #[test]
    fn set_alias_registers_with_parent_container() {
        let container = CustomFieldContainer::new();
        let field = CustomFieldContainer::add(&container, FieldKey::Task(TaskField::Text1));

        field.borrow_mut().set_alias(Some("MyAlias".to_owned()));

        assert_eq!(
            container
                .borrow()
                .get_field_type_by_alias(FieldTypeClass::Task, "MyAlias"),
            Some(FieldKey::Task(TaskField::Text1))
        );
    }

    #[test]
    fn set_alias_with_none_does_not_register() {
        let container = CustomFieldContainer::new();
        let field = CustomFieldContainer::add(&container, FieldKey::Task(TaskField::Text1));

        field.borrow_mut().set_alias(None);

        assert_eq!(
            container
                .borrow()
                .get_field_type_by_alias(FieldTypeClass::Task, "MyAlias"),
            None
        );
    }

    #[test]
    fn ordering_follows_field_type_class_then_unique_id_then_alias() {
        let mut task_field = standalone(FieldKey::Task(TaskField::Text1));
        task_field.set_unique_id(Some(1));
        task_field.set_alias(Some("A".to_owned()));

        let mut resource_field = standalone(FieldKey::Resource(ResourceField::Text1));
        resource_field.set_unique_id(Some(1));
        resource_field.set_alias(Some("A".to_owned()));

        // RESOURCE < TASK（按枚举名比较）
        assert!(resource_field < task_field);

        // 同类型下 unique ID 较小者在前
        let mut task_field_two = standalone(FieldKey::Task(TaskField::Text2));
        task_field_two.set_unique_id(Some(2));
        task_field_two.set_alias(Some("A".to_owned()));
        assert!(task_field < task_field_two);

        // 同类型同 ID 时按别名比较
        let mut task_field_alias_b = standalone(FieldKey::Task(TaskField::Text1));
        task_field_alias_b.set_unique_id(Some(1));
        task_field_alias_b.set_alias(Some("B".to_owned()));
        assert!(task_field < task_field_alias_b);
    }

    #[test]
    fn unset_unique_id_sorts_as_null_like_java() {
        let mut unset = standalone(FieldKey::Task(TaskField::Text1));
        unset.set_alias(Some("A".to_owned()));

        let mut set = standalone(FieldKey::Task(TaskField::Text1));
        set.set_unique_id(Some(1));
        set.set_alias(Some("A".to_owned()));

        // Java 中字符串拼接把 null 渲染为 "null"，"null" > "1"
        assert!(set < unset);
    }

    #[test]
    fn display_matches_java_format() {
        let mut field = standalone(FieldKey::Task(TaskField::Text1));

        assert_eq!(field.to_string(), "[CustomField field=TEXT1 alias=null]");

        field.set_alias(Some("Alias".to_owned()));
        assert_eq!(field.to_string(), "[CustomField field=TEXT1 alias=Alias]");
    }
}
