//! 对应 Java: `org/mpxj/UserDefinedField.java`

use std::any::TypeId;
use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::data_type::DataType;
use crate::field_type_class::FieldTypeClass;
use crate::mpx::locale_utility::MpxLocale;
use crate::mpxj_enum::MpxjEnum;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;

/// 用户自定义字段。
///
/// 对应 Java: `public final class UserDefinedField implements FieldType`
///
/// # 结构取舍：不实现 [`crate::field_type::FieldType`]
///
/// 本 crate 的 `FieldType` trait 带有 `Copy` 约束（见其定义），而 `UserDefinedField`
/// 持有 `String`，无法满足 `Copy`；因此 Rust 侧**不实现该 trait**，改为提供与 Java
/// 完全同名的方法（[`get_field_type_class`](UserDefinedField::get_field_type_class) /
/// [`name`](UserDefinedField::name) / [`get_name`](UserDefinedField::get_name) /
/// [`get_data_type`](UserDefinedField::get_data_type) /
/// [`get_units_type`](UserDefinedField::get_units_type)），语义与 Java 一致。
/// [`MpxjEnum`] 没有 `Copy` 约束，故正常实现。
///
/// 另需注意：[`crate::field_key::FieldKey`] 用 [`crate::field_key::CustomFieldKey`]
/// （以 unique ID 为等价依据的 `Copy` 键）表达本类型，因此
/// [`crate::custom_field::CustomField`] 与
/// [`crate::custom_field_container::CustomFieldContainer`] 可以表示本类型对应的字段；
/// 代价是拿不到 `Rc<RefCell<UserDefinedField>>` 本身（见 `crate::custom_field` 文件头）。
///
/// # 与 Java 的差异
///
/// - Java 的 `Integer m_uniqueID` 在构造时必定由序号生成器赋值（`syncOrGetNext`），
///   Rust 侧因此用 `i32`；Java 的 `String m_externalName`、`FieldTypeClass m_fieldTypeClass`、
///   `DataType m_dataType` 可以为 `null`，Rust 侧用 `Option<_>`。
/// - Java 的 `getName(Locale)` 重载在 Rust 侧命名为 [`UserDefinedField::get_name_for_locale`]
///   （Java 的实现就是 `return getName();`，与 locale 无关）。
/// - `equals` / `hashCode` 只比较 `m_uniqueID`，Rust 侧按此实现 `PartialEq` / `Eq` / `Hash`。
///
/// 被跳过的 Java 机制：无。
///
/// 未移植的 Java 方法：无。
#[derive(Debug, Clone)]
pub struct UserDefinedField {
    /// unique ID。
    ///
    /// 对应 Java: `private final Integer m_uniqueID;`
    m_unique_id: i32,
    /// 字段所属实体类型。
    ///
    /// 对应 Java: `private final FieldTypeClass m_fieldTypeClass;`
    m_field_type_class: Option<FieldTypeClass>,
    /// 是否只用于 WBS（summary task）。
    ///
    /// 对应 Java: `private final boolean m_summaryTaskOnly;`
    m_summary_task_only: bool,
    /// 外部名称。
    ///
    /// 对应 Java: `private final String m_externalName;`
    m_external_name: Option<String>,
    /// 内部名称。
    ///
    /// 对应 Java: `private final String m_internalName;`
    m_internal_name: String,
    /// 数据类型。
    ///
    /// 对应 Java: `private final DataType m_dataType;`
    m_data_type: Option<DataType>,
}

impl UserDefinedField {
    /// 返回该枚举实例关联的 int 值（即 unique ID）。
    ///
    /// 对应 Java: `@Override public int getValue()`
    pub fn get_value(&self) -> i32 {
        self.m_unique_id
    }

    /// 取 unique ID。
    ///
    /// 对应 Java: `public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> i32 {
        self.m_unique_id
    }

    /// 是否为 WBS（在 MPXJ 中表示为 summary task）专用的用户自定义字段。
    ///
    /// 对应 Java: `public boolean getSummaryTaskOnly()`
    pub fn get_summary_task_only(&self) -> bool {
        self.m_summary_task_only
    }

    /// 取字段所属实体类型。
    ///
    /// 对应 Java: `@Override public FieldTypeClass getFieldTypeClass()`
    pub fn get_field_type_class(&self) -> Option<FieldTypeClass> {
        self.m_field_type_class
    }

    /// 取外部名称（显示名）。
    ///
    /// 对应 Java: `@Override public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.m_external_name.as_deref()
    }

    /// 取给定区域设置下的显示名。
    ///
    /// 对应 Java: `@Override public String getName(Locale locale)`
    ///
    /// Java 的实现是 `return getName();` —— 自定义字段的名称来自用户 / 文件，
    /// 与区域设置无关。
    pub fn get_name_for_locale(&self, locale: MpxLocale) -> Option<&str> {
        let _ = locale;
        self.get_name()
    }

    /// 取内部名称（Java 枚举常量名的等价物）。
    ///
    /// 对应 Java: `@Override public String name()`
    pub fn name(&self) -> &str {
        &self.m_internal_name
    }

    /// 取数据类型。
    ///
    /// 对应 Java: `@Override public DataType getDataType()`
    pub fn get_data_type(&self) -> Option<DataType> {
        self.m_data_type
    }

    /// 取关联的单位字段。
    ///
    /// 对应 Java: `@Override public FieldType getUnitsType()`（恒返回 `null`）
    ///
    /// Rust 侧返回 `Option<UserDefinedField>` 并恒为 `None`；之所以返回自身类型而不是
    /// `Option<FieldKey>`，是为了与 [`crate::field_type::FieldType::get_units_type`] 的
    /// `Option<Self>` 形状保持一致。
    pub fn get_units_type(&self) -> Option<UserDefinedField> {
        None
    }
}

impl MpxjEnum for UserDefinedField {
    /// 返回该枚举实例关联的 int 值。
    ///
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.m_unique_id
    }
}

impl PartialEq for UserDefinedField {
    /// 对应 Java: `@Override public boolean equals(Object o)`
    ///
    /// Java 侧先比较类型（本类是 `final`），再比较 `m_uniqueID`，因此等价于只比较 unique ID。
    fn eq(&self, other: &Self) -> bool {
        self.m_unique_id == other.m_unique_id
    }
}

impl Eq for UserDefinedField {}

impl Hash for UserDefinedField {
    /// 对应 Java: `@Override public int hashCode()`
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.m_unique_id.hash(state);
    }
}

impl Display for UserDefinedField {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 返回 `getName()`（可能为 `null`）；Rust 侧在外部名缺失时输出 `null`，
    /// 与 Java 中 `String.valueOf` 面对 `null` 的行为一致。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.m_external_name.as_deref().unwrap_or("null"))
    }
}

/// 用户自定义字段构建器。
///
/// 对应 Java: `public static class Builder`
///
/// Rust 侧沿用 Java 的链式调用风格，按 Rust 习惯以「消耗自身并返回 `Self`」的方式实现。
#[derive(Clone)]
pub struct Builder {
    /// 序号生成器提供者。
    ///
    /// 对应 Java: `private final UniqueIdObjectSequenceProvider m_sequenceProvider;`
    m_sequence_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>,
    /// 对应 Java: `private Integer m_uniqueID;`
    m_unique_id: Option<i32>,
    /// 对应 Java: `private FieldTypeClass m_fieldTypeClass;`
    m_field_type_class: Option<FieldTypeClass>,
    /// 对应 Java: `private boolean m_summaryTaskOnly;`
    m_summary_task_only: bool,
    /// 对应 Java: `private String m_externalName;`
    m_external_name: Option<String>,
    /// 对应 Java: `private String m_internalName;`
    m_internal_name: Option<String>,
    /// 对应 Java: `private DataType m_dataType;`
    m_data_type: Option<DataType>,
}

impl Builder {
    /// 构造方法。
    ///
    /// 对应 Java: `public Builder(UniqueIdObjectSequenceProvider sequenceProvider)`
    pub fn new(sequence_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>) -> Self {
        Self {
            m_sequence_provider: sequence_provider,
            m_unique_id: None,
            m_field_type_class: None,
            m_summary_task_only: false,
            m_external_name: None,
            m_internal_name: None,
            m_data_type: None,
        }
    }

    /// 设置 unique ID。
    ///
    /// 对应 Java: `public Builder uniqueID(Integer value)`
    pub fn unique_id(mut self, value: Option<i32>) -> Self {
        self.m_unique_id = value;
        self
    }

    /// 设置字段所属实体类型。
    ///
    /// 对应 Java: `public Builder fieldTypeClass(FieldTypeClass value)`
    pub fn field_type_class(mut self, value: FieldTypeClass) -> Self {
        self.m_field_type_class = Some(value);
        self
    }

    /// 设置「只用于 WBS」标志。
    ///
    /// 对应 Java: `public Builder summaryTaskOnly(boolean value)`
    pub fn summary_task_only(mut self, value: bool) -> Self {
        self.m_summary_task_only = value;
        self
    }

    /// 设置外部名称。
    ///
    /// 对应 Java: `public Builder externalName(String value)`
    pub fn external_name(mut self, value: Option<String>) -> Self {
        self.m_external_name = value;
        self
    }

    /// 设置内部名称。
    ///
    /// 对应 Java: `public Builder internalName(String value)`
    pub fn internal_name(mut self, value: Option<String>) -> Self {
        self.m_internal_name = value;
        self
    }

    /// 设置数据类型。
    ///
    /// 对应 Java: `public Builder dataType(DataType value)`
    pub fn data_type(mut self, value: DataType) -> Self {
        self.m_data_type = Some(value);
        self
    }

    /// 构建 [`UserDefinedField`]。
    ///
    /// 对应 Java: `public UserDefinedField build()`
    ///
    /// 私有构造方法中的两件事在这里完成：向序号生成器申请/同步 unique ID，以及在内部名
    /// 缺失时生成 `"user_field_<uniqueID>"`。
    pub fn build(self) -> UserDefinedField {
        // Java:
        // `m_uniqueID = builder.m_sequenceProvider.getUniqueIdObjectSequence(UserDefinedField.class).syncOrGetNext(builder.m_uniqueID);`
        let unique_id = self
            .m_sequence_provider
            .borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<UserDefinedField>())
            .sync_or_get_next(self.m_unique_id);

        // Java:
        // `m_internalName = builder.m_internalName == null || builder.m_internalName.isEmpty() ? "user_field_" + m_uniqueID : builder.m_internalName;`
        let internal_name = match self.m_internal_name {
            Some(internal_name) if !internal_name.is_empty() => internal_name,
            _ => format!("user_field_{unique_id}"),
        };

        UserDefinedField {
            m_unique_id: unique_id,
            m_field_type_class: self.m_field_type_class,
            m_summary_task_only: self.m_summary_task_only,
            m_external_name: self.m_external_name,
            m_internal_name: internal_name,
            m_data_type: self.m_data_type,
        }
    }
}

impl fmt::Debug for Builder {
    /// 无法 `Debug` 的是序号生成器提供者（trait object），这里略去它。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Builder")
            .field("m_unique_id", &self.m_unique_id)
            .field("m_field_type_class", &self.m_field_type_class)
            .field("m_summary_task_only", &self.m_summary_task_only)
            .field("m_external_name", &self.m_external_name)
            .field("m_internal_name", &self.m_internal_name)
            .field("m_data_type", &self.m_data_type)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::object_sequence::ObjectSequence;
    use std::collections::HashMap;

    /// 测试用序号生成器提供者。
    #[derive(Default)]
    struct TestProvider {
        sequences: HashMap<TypeId, Rc<ObjectSequence>>,
    }

    impl UniqueIdObjectSequenceProvider for TestProvider {
        fn get_unique_id_object_sequence(&mut self, type_id: TypeId) -> Rc<ObjectSequence> {
            Rc::clone(
                self.sequences
                    .entry(type_id)
                    .or_insert_with(|| Rc::new(ObjectSequence::new(1))),
            )
        }
    }

    /// 构造一个序号生成器提供者。
    fn provider() -> Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> {
        Rc::new(RefCell::new(TestProvider::default()))
    }

    #[test]
    fn unique_ids_are_allocated_from_one() {
        let provider = provider();

        let first = Builder::new(Rc::clone(&provider)).build();
        let second = Builder::new(Rc::clone(&provider)).build();

        assert_eq!(first.get_unique_id(), 1);
        assert_eq!(second.get_unique_id(), 2);
    }

    #[test]
    fn explicit_unique_id_syncs_the_sequence() {
        let provider = provider();

        let field = Builder::new(Rc::clone(&provider))
            .unique_id(Some(7))
            .build();
        assert_eq!(field.get_unique_id(), 7);

        // 序列已同步到 8
        let next = Builder::new(Rc::clone(&provider)).build();
        assert_eq!(next.get_unique_id(), 8);
    }

    /// Java: `getName(Locale)` 的实现就是 `return getName();`，与区域设置无关。
    #[test]
    fn name_for_locale_equals_name() {
        let provider = provider();
        let field = Builder::new(Rc::clone(&provider))
            .internal_name(Some("user_field_1".to_owned()))
            .external_name(Some("My Field".to_owned()))
            .field_type_class(FieldTypeClass::Task)
            .data_type(DataType::Numeric)
            .build();

        for locale in MpxLocale::ALL {
            assert_eq!(field.get_name_for_locale(locale), field.get_name(), "{locale}");
        }
        assert_eq!(field.get_name_for_locale(MpxLocale::Zh), Some("My Field"));

        // 未设置外部名时恒为 None（Java 返回 null）
        let unnamed = Builder::new(Rc::clone(&provider)).build();
        assert_eq!(unnamed.get_name_for_locale(MpxLocale::En), None);
    }

    #[test]
    fn internal_name_defaults_to_user_field_prefix() {
        let provider = provider();

        let field = Builder::new(Rc::clone(&provider)).build();
        assert_eq!(field.name(), "user_field_1");

        let empty = Builder::new(Rc::clone(&provider))
            .internal_name(Some(String::new()))
            .build();
        assert_eq!(empty.name(), "user_field_2");

        let named = Builder::new(Rc::clone(&provider))
            .internal_name(Some("internal".to_owned()))
            .build();
        assert_eq!(named.name(), "internal");
    }

    #[test]
    fn accessors_return_builder_values() {
        let provider = provider();

        let field = Builder::new(provider)
            .field_type_class(FieldTypeClass::Task)
            .summary_task_only(true)
            .external_name(Some("External".to_owned()))
            .data_type(DataType::String)
            .build();

        assert_eq!(field.get_field_type_class(), Some(FieldTypeClass::Task));
        assert!(field.get_summary_task_only());
        assert_eq!(field.get_name(), Some("External"));
        assert_eq!(field.get_data_type(), Some(DataType::String));
        assert_eq!(field.get_value(), field.get_unique_id());
        assert!(field.get_units_type().is_none());
    }

    #[test]
    fn missing_optional_values_are_none() {
        let provider = provider();
        let field = Builder::new(provider).build();

        assert_eq!(field.get_field_type_class(), None);
        assert_eq!(field.get_name(), None);
        assert_eq!(field.get_data_type(), None);
        assert!(!field.get_summary_task_only());
    }

    #[test]
    fn equality_and_hash_follow_unique_id() {
        let provider = provider();

        // unique ID 显式指定为同一个值
        let first = Builder::new(Rc::clone(&provider))
            .unique_id(Some(5))
            .external_name(Some("A".to_owned()))
            .build();
        let second = Builder::new(Rc::clone(&provider))
            .unique_id(Some(5))
            .external_name(Some("B".to_owned()))
            .build();
        let third = Builder::new(Rc::clone(&provider))
            .unique_id(Some(6))
            .build();

        assert_eq!(first, second);
        assert_ne!(first, third);

        let mut map = HashMap::new();
        map.insert(first.clone(), "first");
        assert_eq!(map.get(&second), Some(&"first"));
        assert_eq!(map.get(&third), None);
    }

    #[test]
    fn display_returns_external_name() {
        let provider = provider();

        let named = Builder::new(Rc::clone(&provider))
            .external_name(Some("External".to_owned()))
            .build();
        assert_eq!(named.to_string(), "External");

        let unnamed = Builder::new(provider).build();
        assert_eq!(unnamed.to_string(), "null");
    }
}
