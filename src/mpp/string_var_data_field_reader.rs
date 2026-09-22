//! Corresponds to Java: `org/mpxj/mpp/StringVarDataFieldReader.java`
//!
//! Implementation of [`VarDataFieldReader`]: reads a string field from a variable data block (UTF-16LE, NUL-terminated).
//!
//! # Restoring inheritance
//!
//! Java: `class StringVarDataFieldReader extends VarDataFieldReader`.
//! Per convention on the Rust side: interface (abstract class) → trait, implementation class → standalone struct + `impl`.
//!
//! | Java | Rust |
//! |---|---|
//! | `StringVarDataFieldReader extends VarDataFieldReader` | [`StringVarDataFieldReader`] implements [`VarDataFieldReader`] |
//! | `private final CustomFieldContainer m_customFields` (inherited from the base class) | the struct holds a `custom_fields` field itself, exposed via [`VarDataFieldReader::get_custom_fields`] |
//! | `public StringVarDataFieldReader(CustomFieldContainer)` | [`StringVarDataFieldReader::new`] |
//! | `protected Object readValue(...)` | [`VarDataFieldReader::read_value`] |
//! | `protected Object coerceValue(Object)` | [`VarDataFieldReader::coerce_value`] |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `varData.getUnicodeString(id, type)` (may be null) | `Option<String>` | `Var2Data::get_unicode_string` returns `None` when the data item does not exist, corresponding to Java's null |
//! | `Object` return value | `Option<FieldValue>` | the value read is always `FieldValue::String`; null → `None` |
//! | `value instanceof String` | matches `FieldValue::String` | the remaining variants (including `Duration` / `Rate` / `DateTime`) return null → `None`, just like Java |
//!
//! Skipped Java methods: none.
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class uses none of these mechanisms.

use std::cell::RefCell;
use std::rc::Rc;

use crate::custom_field_container::CustomFieldContainer;
use crate::field_value::FieldValue;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_data_field_reader::VarDataFieldReader;

/// Read a string field from a variable data block.
///
/// Corresponds to Java: `class StringVarDataFieldReader extends VarDataFieldReader`
pub struct StringVarDataFieldReader {
/// Custom field container.
    ///
/// Corresponds to Java field (inherited from the base class): `private final CustomFieldContainer m_customFields`
    custom_fields: Rc<RefCell<CustomFieldContainer>>,
}

impl StringVarDataFieldReader {
/// Constructor.
    ///
    /// Corresponds to Java: `public StringVarDataFieldReader(CustomFieldContainer customFields)`
    pub fn new(custom_fields: Rc<RefCell<CustomFieldContainer>>) -> Self {
        Self { custom_fields }
    }
}

impl VarDataFieldReader for StringVarDataFieldReader {
/// Corresponds to Java: `public CustomFieldContainer getCustomFields()` (a Rust-specific accessor; see the trait notes)
    fn get_custom_fields(&self) -> &Rc<RefCell<CustomFieldContainer>> {
        &self.custom_fields
    }

/// Read the data item as a UTF-16LE string; returns `None` when the data item does not exist (Java returns null).
    ///
    /// Corresponds to Java: `@Override protected Object readValue(Var2Data varData, Integer id, Integer type)`
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

/// Keeps a string value as-is; other types return `None` (Java returns null).
    ///
    /// Corresponds to Java: `@Override protected Object coerceValue(Object value)`
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

/// Data item type (used uniformly in the tests).
    const TYPE: i32 = 0x12;

/// Read a UTF-16LE (NUL-terminated) string data item.
    #[test]
    fn reads_unicode_string() {
// the UTF-16LE representation of "AB" + trailing NUL
        let var_data = var_data(&[(1, TYPE, vec![0x41, 0x00, 0x42, 0x00, 0x00, 0x00])]);
        let reader = StringVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(
            reader.read_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::String("AB".to_owned()))
        );
    }

/// Returns `None` when the data item is missing (Java's `getUnicodeString` returns null).
    #[test]
    fn missing_entry_returns_none() {
        let var_data = var_data(&[(1, TYPE, vec![0x41, 0x00, 0x00, 0x00])]);
        let reader = StringVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(reader.read_value(&var_data, Some(999), Some(TYPE)), None);
        assert_eq!(reader.read_value(&var_data, None, None), None);
    }

/// An empty data item (0 bytes) decodes to an empty string (Java's `getUnicodeString` returns `""`).
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

/// An out-of-bounds data item (unregistered id / type) returns `None` without panicking.
    #[test]
    fn out_of_range_entry_returns_none() {
        let var_data = var_data(&[(1, TYPE, vec![0x41, 0x00, 0x00, 0x00])]);
        let reader = StringVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

// data type mismatch
        assert_eq!(reader.read_value(&var_data, Some(1), Some(0x99)), None);
    }

/// `coerce_value`: only string values pass.
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
