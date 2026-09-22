//! Corresponds to Java: `org/mpxj/mpp/DoubleVarDataFieldReader.java`
//!
//! Implementation of [`VarDataFieldReader`]: reads a double-precision floating-point field from a variable-length data block.
//!
//! # How the inheritance is restored
//!
//! Java: `class DoubleVarDataFieldReader extends VarDataFieldReader`.
//! The Rust side follows the convention: interface (abstract class) → trait, implementation class → standalone struct + `impl`.
//!
//! | Java | Rust |
//! |---|---|
//! | `DoubleVarDataFieldReader extends VarDataFieldReader` | [`DoubleVarDataFieldReader`] implements [`VarDataFieldReader`] |
//! | `private final CustomFieldContainer m_customFields` (inherited from the base class) | The struct holds its own `custom_fields` field, exposed via [`VarDataFieldReader::get_custom_fields`] |
//! | `public DoubleVarDataFieldReader(CustomFieldContainer)` | [`DoubleVarDataFieldReader::new`] |
//! | `protected Object readValue(...)` | [`VarDataFieldReader::read_value`] |
//! | `protected Object coerceValue(Object)` | [`VarDataFieldReader::coerce_value`] |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `NumberHelper.getDouble(varData.getDouble(id, type))` | [`NumberHelper::get_double`] | Java boxes the primitive `double` into `Double`; Rust's `f64` is not boxed, so `Some(f64)` is passed (always non-`None`, consistent with Java's autoboxing, and the result is always that value itself) |
//! | `Object` return value | `Option<FieldValue>` | The value read is always `FieldValue::Double`; `null` → `None` |
//! | `value instanceof Number` | [`FieldValue::as_double`] | Java's `Number` covers `Integer` / `Short` / `Double` (as well as `Long` / `Float` etc., which this crate does not carry); Rust's `FieldValue` has only the three numeric variants `Double` / `Integer` / `Short`, and the other variants return `null` → `None` just like Java |
//! | `NumberHelper.getDouble(((Number) value).doubleValue())` | [`FieldValue::as_double`] + `FieldValue::Double` | Semantically equivalent: `Integer(3)` → `Double(3.0)` |
//!
//! Skipped Java methods: none.
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::number_helper::NumberHelper;
use crate::custom_field_container::CustomFieldContainer;
use crate::field_value::FieldValue;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_data_field_reader::VarDataFieldReader;

/// Read a double-precision floating-point field from a variable-length data block.
///
/// Corresponds to Java: `class DoubleVarDataFieldReader extends VarDataFieldReader`
pub struct DoubleVarDataFieldReader {
/// Custom field container.
    ///
/// Corresponds to Java field (inherited from the base class): `private final CustomFieldContainer m_customFields`
    custom_fields: Rc<RefCell<CustomFieldContainer>>,
}

impl DoubleVarDataFieldReader {
/// Constructor.
    ///
    /// Corresponds to Java: `public DoubleVarDataFieldReader(CustomFieldContainer customFields)`
    pub fn new(custom_fields: Rc<RefCell<CustomFieldContainer>>) -> Self {
        Self { custom_fields }
    }
}

impl VarDataFieldReader for DoubleVarDataFieldReader {
/// Corresponds to Java: `public CustomFieldContainer getCustomFields()` (Rust-specific accessor, see the trait documentation)
    fn get_custom_fields(&self) -> &Rc<RefCell<CustomFieldContainer>> {
        &self.custom_fields
    }

/// Read the data item as a double-precision float (0 when the data item is missing).
    ///
    /// Corresponds to Java: `@Override protected Object readValue(Var2Data varData, Integer id, Integer type)`
    fn read_value(
        &self,
        var_data: &Var2Data,
        id: Option<i32>,
        type_: Option<i32>,
    ) -> Option<FieldValue> {
        // Java: return NumberHelper.getDouble(varData.getDouble(id, type));
        Some(FieldValue::Double(NumberHelper::get_double(Some(
            var_data.get_double(id, type_),
        ))))
    }

/// Coerce the value of the value-list item to a double-precision float; return `None` when it is not numeric (Java returns null).
    ///
    /// Corresponds to Java: `@Override protected Object coerceValue(Object value)`
    fn coerce_value(&self, value: Option<&FieldValue>) -> Option<FieldValue> {
        // Java: if (value instanceof Number) { return NumberHelper.getDouble(((Number) value).doubleValue()); }
        //       return null;
        value?.as_double().map(FieldValue::Double)
    }
}

#[cfg(test)]
mod tests {
    use super::super::var_data_field_reader::test_support::{container_with_item, var_data};
    use super::*;
    use crate::custom_field_lookup_table::CustomFieldValueItem;

/// Data item type (used uniformly in tests).
    const TYPE: i32 = 0x12;

/// Read an 8-byte floating-point data item.
    #[test]
    fn reads_double_value() {
        let var_data = var_data(&[(
            1,
            TYPE,
            vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF8, 0x3F],
        )]);
        let reader = DoubleVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(
            reader.read_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::Double(1.5))
        );
    }

/// When the data item is missing, `Var2Data.getDouble` returns 0 (Java also returns 0, not null).
    #[test]
    fn missing_entry_reads_zero() {
        let var_data = var_data(&[(1, TYPE, vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF8, 0x3F])]);
        let reader = DoubleVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(
            reader.read_value(&var_data, Some(999), Some(TYPE)),
            Some(FieldValue::Double(0.0))
        );
    }

/// When the data item is shorter than 8 bytes / the bit pattern is NaN, treat it as 0 (consistent with `Var2Data.getDouble`).
    #[test]
    fn short_or_nan_data_reads_zero() {
// 4 bytes: less than one double
        let four_byte_var_data = var_data(&[(1, TYPE, vec![0x00, 0x00, 0xF8, 0x3F])]);
        let reader = DoubleVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));
        assert_eq!(
            reader.read_value(&four_byte_var_data, Some(1), Some(TYPE)),
            Some(FieldValue::Double(0.0))
        );

// NaN bit pattern
        let nan_var_data = var_data(&[(1, TYPE, f64::NAN.to_bits().to_le_bytes().to_vec())]);
        let reader = DoubleVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));
        assert_eq!(
            reader.read_value(&nan_var_data, Some(1), Some(TYPE)),
            Some(FieldValue::Double(0.0))
        );
    }

/// `coerce_value`: widen numeric values (including `Integer` / `Short`) to `Double`.
    #[test]
    fn coerce_value_widens_numbers() {
        let reader = DoubleVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(
            reader.coerce_value(Some(&FieldValue::Double(2.5))),
            Some(FieldValue::Double(2.5))
        );
        assert_eq!(
            reader.coerce_value(Some(&FieldValue::Integer(3))),
            Some(FieldValue::Double(3.0))
        );
        assert_eq!(
            reader.coerce_value(Some(&FieldValue::Short(4))),
            Some(FieldValue::Double(4.0))
        );
    }

/// `coerce_value`: non-numeric values and null (`None`) return `None`.
    #[test]
    fn coerce_value_rejects_non_numbers() {
        let reader = DoubleVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(reader.coerce_value(None), None);
        assert_eq!(
            reader.coerce_value(Some(&FieldValue::String("x".to_owned()))),
            None
        );
        assert_eq!(reader.coerce_value(Some(&FieldValue::Boolean(true))), None);
    }
}
