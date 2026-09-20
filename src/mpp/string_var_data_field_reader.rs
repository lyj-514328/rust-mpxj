//! 对应 Java: `org/mpxj/mpp/StringVarDataFieldReader.java`
//!
//! [`VarDataFieldReader`] 的实现：从变长数据块中读一个字符串字段（UTF-16LE，NUL 结尾）。
//!
//! # 继承的还原
//!
//! Java: `class StringVarDataFieldReader extends VarDataFieldReader`。
//! Rust 侧按约定：接口（抽象类）→ trait，实现类 → 独立结构体 + `impl`。
//!
//! | Java | Rust |
//! |---|---|
//! | `StringVarDataFieldReader extends VarDataFieldReader` | [`StringVarDataFieldReader`] 实现 [`VarDataFieldReader`] |
//! | `private final CustomFieldContainer m_customFields`（继承自基类） | 结构体自持 `custom_fields` 字段，经 [`VarDataFieldReader::get_custom_fields`] 暴露 |
//! | `public StringVarDataFieldReader(CustomFieldContainer)` | [`StringVarDataFieldReader::new`] |
//! | `protected Object readValue(...)` | [`VarDataFieldReader::read_value`] |
//! | `protected Object coerceValue(Object)` | [`VarDataFieldReader::coerce_value`] |
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `varData.getUnicodeString(id, type)`（可为 null） | `Option<String>` | `Var2Data::get_unicode_string` 在数据项不存在时返回 `None`，与 Java 的 null 对应 |
//! | `Object` 返回值 | `Option<FieldValue>` | 读到的值恒为 `FieldValue::String`；null → `None` |
//! | `value instanceof String` | 匹配 `FieldValue::String` | 其余变体（含 `Duration` / `Rate` / `DateTime`）与 Java 一样返回 null → `None` |
//!
//! 跳过的 Java 方法：无。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::cell::RefCell;
use std::rc::Rc;

use crate::custom_field_container::CustomFieldContainer;
use crate::field_value::FieldValue;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_data_field_reader::VarDataFieldReader;

/// 从变长数据块中读字符串字段。
///
/// 对应 Java: `class StringVarDataFieldReader extends VarDataFieldReader`
pub struct StringVarDataFieldReader {
    /// 自定义字段容器。
    ///
    /// 对应 Java 字段（继承自基类）: `private final CustomFieldContainer m_customFields`
    custom_fields: Rc<RefCell<CustomFieldContainer>>,
}

impl StringVarDataFieldReader {
    /// 构造方法。
    ///
    /// 对应 Java: `public StringVarDataFieldReader(CustomFieldContainer customFields)`
    pub fn new(custom_fields: Rc<RefCell<CustomFieldContainer>>) -> Self {
        Self { custom_fields }
    }
}

impl VarDataFieldReader for StringVarDataFieldReader {
    /// 对应 Java: `public CustomFieldContainer getCustomFields()`（Rust 特有访问器，见 trait 说明）
    fn get_custom_fields(&self) -> &Rc<RefCell<CustomFieldContainer>> {
        &self.custom_fields
    }

    /// 按 UTF-16LE 字符串读取数据项；数据项不存在时返回 `None`（Java 返回 null）。
    ///
    /// 对应 Java: `@Override protected Object readValue(Var2Data varData, Integer id, Integer type)`
    fn read_value(
        &self,
        var_data: &Var2Data,
        id: Option<i32>,
        type_: Option<i32>,
    ) -> Option<FieldValue> {
        // Java: return varData.getUnicodeString(id, type);
        var_data
            .get_unicode_string(id, type_)
            .map(FieldValue::String)
    }

    /// 原样保留字符串取值；其他类型返回 `None`（Java 返回 null）。
    ///
    /// 对应 Java: `@Override protected Object coerceValue(Object value)`
    fn coerce_value(&self, value: Option<&FieldValue>) -> Option<FieldValue> {
        // Java: if (value instanceof String) { return value; } return null;
        match value? {
            FieldValue::String(text) => Some(FieldValue::String(text.clone())),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::var_data_field_reader::test_support::{container_with_item, var_data};
    use super::*;
    use crate::custom_field_lookup_table::CustomFieldValueItem;

    /// 数据项类型（测试里统一使用）。
    const TYPE: i32 = 0x12;

    /// 读取 UTF-16LE（NUL 结尾）字符串数据项。
    #[test]
    fn reads_unicode_string() {
        // "AB" 的 UTF-16LE 表示 + 结尾 NUL
        let var_data = var_data(&[(1, TYPE, vec![0x41, 0x00, 0x42, 0x00, 0x00, 0x00])]);
        let reader = StringVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(
            reader.read_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::String("AB".to_owned()))
        );
    }

    /// 数据项缺失时返回 `None`（Java 的 `getUnicodeString` 返回 null）。
    #[test]
    fn missing_entry_returns_none() {
        let var_data = var_data(&[(1, TYPE, vec![0x41, 0x00, 0x00, 0x00])]);
        let reader = StringVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(reader.read_value(&var_data, Some(999), Some(TYPE)), None);
        assert_eq!(reader.read_value(&var_data, None, None), None);
    }

    /// 空数据项（0 字节）解码为空串（Java 的 `getUnicodeString` 返回 `""`）。
    #[test]
    fn empty_entry_reads_empty_string() {
        let var_data = var_data(&[(1, TYPE, Vec::new())]);
        let reader = StringVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(
            reader.read_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::String(String::new()))
        );
    }

    /// 数据项越界（未登记的 id / type）返回 `None` 而不 panic。
    #[test]
    fn out_of_range_entry_returns_none() {
        let var_data = var_data(&[(1, TYPE, vec![0x41, 0x00, 0x00, 0x00])]);
        let reader = StringVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        // 数据类型不匹配
        assert_eq!(reader.read_value(&var_data, Some(1), Some(0x99)), None);
    }

    /// `coerce_value`：仅字符串取值通过。
    #[test]
    fn coerce_value_accepts_strings_only() {
        let reader = StringVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(
            reader.coerce_value(Some(&FieldValue::String("x".to_owned()))),
            Some(FieldValue::String("x".to_owned()))
        );
        assert_eq!(reader.coerce_value(None), None);
        assert_eq!(reader.coerce_value(Some(&FieldValue::Integer(1))), None);
        assert_eq!(reader.coerce_value(Some(&FieldValue::Boolean(false))), None);
    }
}
