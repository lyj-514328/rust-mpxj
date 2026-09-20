//! 对应 Java: `org/mpxj/CustomField.java`

use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::rc::Weak;

use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::mpp_assignment_field::MPPAssignmentField;
use crate::common::mpp_field_type::MppFieldType;
use crate::common::mpp_project_field::MPPProjectField;
use crate::common::mpp_resource_field::MPPResourceField;
use crate::common::mpp_task_field::MPPTaskField;
use crate::custom_field_container::CustomFieldContainer;
use crate::custom_field_lookup_table::CustomFieldLookupTable;
use crate::custom_field_value_mask::CustomFieldValueMask;
use crate::field_key::FieldKey;
use crate::field_type::FieldType;
use crate::field_type_class::FieldTypeClass;
use crate::graphical_indicator::GraphicalIndicator;

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
/// | `private final GraphicalIndicator m_indicator` | [`GraphicalIndicator`]（`src/graphical_indicator.rs`） |
/// | `private final List<CustomFieldValueMask> m_masks` | `Vec<CustomFieldValueMask>` |
/// | `private Integer m_uniqueID` | `Cell<Option<i32>>`（Java 的 `getUniqueID()` 会惰性回填该字段，`Cell` 提供等价的可变性，见下） |
/// | `private String m_alias` | `Option<String>` |
///
/// # 与 Java 的差异
///
/// - `getUniqueID()` 的惰性回落**已实现**（见 [`CustomField::get_unique_id`]）：
///   Java 在该值为 `null` 时写入 `FieldTypeHelper.getFieldID(m_field)` 并返回，
///   Rust 侧用 [`std::cell::Cell`] 保存 `m_unique_id`，因此 `&self` 下也能完成
///   「读回落 + 回填缓存」。`m_field` 为 [`FieldKey::Custom`] 时按 Java 的
///   `UserDefinedField.getValue()` 分支计算（见模块级 `fallback_unique_id`）。
/// - `getGraphicalIndicator()` 返回的是 `m_indicator` 的借用，且额外提供
///   [`CustomField::get_graphical_indicator_mut`]：Java 的 `GraphicalIndicator` 是引用语义，
///   调用方拿到后直接修改；Rust 侧实例通常被 `Rc<RefCell<CustomField>>` 共享，故按本 crate
///   的既有做法（见 `get_lookup_table_mut` / `get_masks_mut`）另给一个可变访问器。
///
/// 未移植的 Java 方法：无。
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
    /// 图形化指标配置。
    ///
    /// 对应 Java: `private final GraphicalIndicator m_indicator;`
    m_indicator: GraphicalIndicator,
    /// mask 定义。
    ///
    /// 对应 Java: `private final List<CustomFieldValueMask> m_masks;`
    m_masks: Vec<CustomFieldValueMask>,
    /// unique ID。
    ///
    /// 对应 Java: `private Integer m_uniqueID;`
    ///
    /// 用 [`Cell`] 而非 `Option<i32>`：Java 的 `getUniqueID()` 会在未显式设置时把回落结果
    /// 写回该字段（惰性缓存），`&self` 下要复刻该行为需要内部可变性。
    m_unique_id: Cell<Option<i32>>,
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
            m_indicator: GraphicalIndicator::new(field),
            m_masks: Vec::new(),
            m_unique_id: Cell::new(None),
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

    /// 取该字段的图形化指标配置。
    ///
    /// 对应 Java: `public GraphicalIndicator getGraphicalIndicator()`
    ///
    /// Java 返回的实例在构造时创建（`m_indicator = new GraphicalIndicator(field)`），
    /// 因此同一字段的指标配置始终是同一个实例；`GraphicalIndicatorReader` 正是取出它再填充。
    pub fn get_graphical_indicator(&self) -> &GraphicalIndicator {
        &self.m_indicator
    }

    /// 以可变方式取图形化指标配置（Java 返回的对象本身可变）。
    ///
    /// 对应 Java: `public GraphicalIndicator getGraphicalIndicator()`
    pub fn get_graphical_indicator_mut(&mut self) -> &mut GraphicalIndicator {
        &mut self.m_indicator
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
    /// Java：未显式设置时回落到 [`FieldTypeHelper::get_field_id`]（即 MS Project 使用的
    /// 字段号）并把结果缓存下来；`setUniqueID(null)` 之后再取会重新回落。
    /// 回落的计算见模块级 `fallback_unique_id`（`FieldKey::Custom` 需按
    /// `UserDefinedField.getValue()` 分支处理，不能走内置字段的查表分支）。
    pub fn get_unique_id(&self) -> Option<i32> {
        // Java: if (m_uniqueID == null) { m_uniqueID = Integer.valueOf(...); }
        if let Some(unique_id) = self.m_unique_id.get() {
            return Some(unique_id);
        }

        let unique_id = fallback_unique_id(self.m_field);
        self.m_unique_id.set(Some(unique_id));
        // Java 的 `getUniqueID()` 恒返回非 null 的 `Integer`，Rust 侧保留 `Option`
        // 只是为了表达「尚未计算」的缓存状态，返回值始终是 `Some`。
        Some(unique_id)
    }

    /// 设置 unique ID。
    ///
    /// 对应 Java: `public CustomField setUniqueID(Integer uniqueID)`
    pub fn set_unique_id(&mut self, unique_id: Option<i32>) {
        self.m_unique_id.set(unique_id);
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
    ///
    /// Java 用的是 `getUniqueID()`（会惰性回落），故这里同样走
    /// [`CustomField::get_unique_id`]，而不是直接读缓存字段。
    fn sort_key(&self) -> String {
        format!(
            "{}.{}.{}",
            field_type_class_name(self.m_field.get_field_type_class()),
            self.get_unique_id().unwrap_or(-1),
            // Java: "+ getAlias()"，别名未设置时输出 "null"
            self.m_alias.as_deref().unwrap_or("null")
        )
    }
}

/// 复刻 Java `FieldTypeHelper.getFieldID(m_field)`（`CustomField.getUniqueID` 的回落值）。
///
/// 内置字段直接转发 [`FieldTypeHelper::get_field_id`]。
///
/// `FieldKey::Custom` 必须单独处理：Java 的 `FieldTypeHelper.getFieldID` 按
/// `FieldTypeClass` 分派到 `MPPTaskField.getID(...)` 等，而 `getID` 内部对
/// `UserDefinedField` 走的是 `value.getValue()` 分支（返回该字段的 unique ID），
/// 并非 `ID_ARRAY` 查表。Rust 的 [`FieldKey::Custom`] 是 `Copy` 的等价键、
/// 拿不到 `UserDefinedField` 句柄，若按内置字段传成 `MppFieldType::Builtin`
/// 会落入查表分支（自定义字段的 unique ID 远超 `ID_ARRAY` 长度，结果是 -1），
/// 故这里按 `value.getValue()` 的语义手工计算，与 Java 一致。
fn fallback_unique_id(field: FieldKey) -> i32 {
    match field {
        FieldKey::Custom(key) => match key.get_field_type_class() {
            // Java: result = MPPTaskField.TASK_FIELD_BASE | MPPTaskField.getID(type)
            FieldTypeClass::Task => MPPTaskField::TASK_FIELD_BASE | key.get_unique_id(),
            // Java: result = MPPResourceField.RESOURCE_FIELD_BASE | MPPResourceField.getID(type)
            FieldTypeClass::Resource => MPPResourceField::RESOURCE_FIELD_BASE | key.get_unique_id(),
            // Java: result = MPPAssignmentField.ASSIGNMENT_FIELD_BASE | MPPAssignmentField.getID(type)
            FieldTypeClass::Assignment => {
                MPPAssignmentField::ASSIGNMENT_FIELD_BASE | key.get_unique_id()
            }
            // Java: result = MPPProjectField.PROJECT_FIELD_BASE | MPPProjectField.getID(type)
            FieldTypeClass::Project => MPPProjectField::PROJECT_FIELD_BASE | key.get_unique_id(),
            // Java: default: result = -1;（Constraint / Unknown）
            _ => -1,
        },
        // Java: switch (type.getFieldTypeClass()) { case TASK/RESOURCE/ASSIGNMENT/PROJECT: ... }
        other => FieldTypeHelper::get_field_id(Some(&MppFieldType::from(other))),
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
            // 直接读缓存值（不触发回落），`Cell` 本身的 `Debug` 输出会多一层包装
            .field("unique_id", &self.m_unique_id.get())
            .field("alias", &self.m_alias)
            .field("table", &self.m_table)
            .field("indicator", &self.m_indicator)
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
        assert!(field.get_masks().is_empty());
        assert!(field.get_lookup_table().is_empty());

        field.set_alias(Some("Alias".to_owned()));
        field.set_unique_id(Some(42));

        assert_eq!(field.get_alias(), Some("Alias"));
        assert_eq!(field.get_unique_id(), Some(42));
    }

    /// Java: `getUniqueID()` 未显式设置时回落到 `FieldTypeHelper.getFieldID(m_field)`
    /// 并把结果缓存；`setUniqueID(null)` 之后重新回落。
    #[test]
    fn unique_id_falls_back_to_field_id_and_is_cached() {
        let mut field = standalone(FieldKey::Task(TaskField::Text1));

        let expected = FieldTypeHelper::get_field_id(Some(&MppFieldType::from(FieldKey::Task(
            TaskField::Text1,
        ))));
        assert_eq!(field.get_unique_id(), Some(expected));
        // 回落结果被回填缓存（Debug 直接读缓存字段即可看到）
        assert_eq!(field.m_unique_id.get(), Some(expected));

        // setUniqueID(null) 之后再次读取 → 重新回落
        field.set_unique_id(None);
        assert_eq!(field.get_unique_id(), Some(expected));

        // 显式设置的值优先
        field.set_unique_id(Some(7));
        assert_eq!(field.get_unique_id(), Some(7));
    }

    /// 自定义字段键（`FieldKey::Custom`）的回落值。
    ///
    /// Java 的 `MPPTaskField.getID(UserDefinedField)` 返回 `value.getValue()`（即该字段的
    /// unique ID），随后与 `TASK_FIELD_BASE` 按位或；不能走内置字段的 `ID_ARRAY` 查表分支。
    #[test]
    fn unique_id_fallback_for_custom_field_keys() {
        let key = crate::field_key::CustomFieldKey::new(
            0x0B40_8000,
            FieldTypeClass::Task,
            crate::data_type::DataType::Custom,
        );
        let field = standalone(FieldKey::Custom(key));

        assert_eq!(
            field.get_unique_id(),
            Some(MPPTaskField::TASK_FIELD_BASE | 0x0B40_8000)
        );

        // 资源 / 分配 / 项目类自定义字段各用自己的 BASE
        let resource_key = crate::field_key::CustomFieldKey::new(
            5,
            FieldTypeClass::Resource,
            crate::data_type::DataType::Custom,
        );
        assert_eq!(
            standalone(FieldKey::Custom(resource_key)).get_unique_id(),
            Some(MPPResourceField::RESOURCE_FIELD_BASE | 5)
        );

        let assignment_key = crate::field_key::CustomFieldKey::new(
            5,
            FieldTypeClass::Assignment,
            crate::data_type::DataType::Custom,
        );
        assert_eq!(
            standalone(FieldKey::Custom(assignment_key)).get_unique_id(),
            Some(MPPAssignmentField::ASSIGNMENT_FIELD_BASE | 5)
        );

        let project_key = crate::field_key::CustomFieldKey::new(
            5,
            FieldTypeClass::Project,
            crate::data_type::DataType::Custom,
        );
        assert_eq!(
            standalone(FieldKey::Custom(project_key)).get_unique_id(),
            Some(MPPProjectField::PROJECT_FIELD_BASE | 5)
        );

        // Java: default: result = -1;（约束字段不参与 getFieldID 的 switch）
        let constraint_key = crate::field_key::CustomFieldKey::new(
            5,
            FieldTypeClass::Constraint,
            crate::data_type::DataType::Custom,
        );
        assert_eq!(standalone(FieldKey::Custom(constraint_key)).get_unique_id(), Some(-1));
    }

    #[test]
    fn graphical_indicator_is_created_in_constructor() {
        let mut field = standalone(FieldKey::Task(TaskField::Text1));

        // Java: m_indicator = new GraphicalIndicator(field)
        assert_eq!(
            field.get_graphical_indicator().get_field_type(),
            FieldKey::Task(TaskField::Text1)
        );
        assert!(!field.get_graphical_indicator().get_display_graphical_indicators());

        // Java 的指标对象本身可变（同一实例被读写）
        field
            .get_graphical_indicator_mut()
            .set_display_graphical_indicators(true);
        assert!(field.get_graphical_indicator().get_display_graphical_indicators());
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

    /// Java 的 `compareTo` 用 `getUniqueID()`（惰性回落）拼排序键，因此未显式设置 ID 的字段
    /// 得到的是回落出的字段号，而不是字符串 `"null"`。
    #[test]
    fn sort_key_uses_the_fallback_unique_id() {
        let mut unset = standalone(FieldKey::Task(TaskField::Text1));
        unset.set_alias(Some("A".to_owned()));

        let fallback = FieldTypeHelper::get_field_id(Some(&MppFieldType::from(FieldKey::Task(
            TaskField::Text1,
        ))));
        assert_eq!(unset.sort_key(), format!("TASK.{fallback}.A"));

        // 显式设置的 ID 参与字符串比较："1" < 回落出的字段号（多位数）
        let mut set = standalone(FieldKey::Task(TaskField::Text1));
        set.set_unique_id(Some(1));
        set.set_alias(Some("A".to_owned()));
        assert!(set.sort_key() < unset.sort_key());
    }

    #[test]
    fn display_matches_java_format() {
        let mut field = standalone(FieldKey::Task(TaskField::Text1));

        assert_eq!(field.to_string(), "[CustomField field=TEXT1 alias=null]");

        field.set_alias(Some("Alias".to_owned()));
        assert_eq!(field.to_string(), "[CustomField field=TEXT1 alias=Alias]");
    }
}
