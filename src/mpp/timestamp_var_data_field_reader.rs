//! Corresponds to Java: `org/mpxj/mpp/TimestampVarDataFieldReader.java`
//!
//! Implementation of [`VarDataFieldReader`]: reads a timestamp field from a variable-length data block.
//!
//! # Restoring inheritance
//!
//! Java: `class TimestampVarDataFieldReader extends VarDataFieldReader`.
//! By convention the Rust side maps: interface (abstract class) -> trait, implementation class -> standalone struct + `impl`.
//!
//! | Java | Rust |
//! |---|---|
//! | `TimestampVarDataFieldReader extends VarDataFieldReader` | [`TimestampVarDataFieldReader`] implements [`VarDataFieldReader`] |
//! | `private final CustomFieldContainer m_customFields` (inherited from the base class) | the struct holds a `custom_fields` field itself, exposed via [`VarDataFieldReader::get_custom_fields`] |
//! | `public TimestampVarDataFieldReader(CustomFieldContainer)` | [`TimestampVarDataFieldReader::new`] |
//! | `protected Object readValue(...)` | [`VarDataFieldReader::read_value`] |
//! | `protected Object coerceValue(Object)` | [`VarDataFieldReader::coerce_value`] |
//! | `private Object getRawTimestampValue(Var2Data, Integer, Integer)` | private method `TimestampVarDataFieldReader::get_raw_timestamp_value` |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `Object` return value | `Option<FieldValue>` | Java's `getRawTimestampValue` may return a `String` (512-byte branch) or a `LocalDateTime`; the Rust side maps these to `FieldValue::String` / `FieldValue::DateTime` respectively, null -> `None` |
//! | `MPPUtility.getTimestamp(data, 0)` | [`MPPUtility::get_timestamp`] | returns `Option<NaiveDateTime>` (Java returns null when the day count is NA, etc.) |
//! | `MPPUtility.getUnicodeString(data, 0)` | [`MPPUtility::get_unicode_string`] | returns `String` (Java returns `""` for empty data) |
//! | `value instanceof LocalDateTime` | matches `FieldValue::DateTime` | strictness matches Java: `java.time.LocalDate` (Rust's `FieldValue::Date`) does not pass |
//!
//! Skipped Java methods: none.
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::cell::RefCell;
use std::rc::Rc;

use crate::custom_field_container::CustomFieldContainer;
use crate::field_value::FieldValue;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_data_field_reader::VarDataFieldReader;

/// Read a timestamp field from a variable-length data block.
///
/// Corresponds to Java: `class TimestampVarDataFieldReader extends VarDataFieldReader`
pub struct TimestampVarDataFieldReader {
/// Custom field container.
    ///
/// Corresponds to Java field (inherited from the base class): `private final CustomFieldContainer m_customFields`
    custom_fields: Rc<RefCell<CustomFieldContainer>>,
}

impl TimestampVarDataFieldReader {
/// Constructor.
    ///
    /// Corresponds to Java: `public TimestampVarDataFieldReader(CustomFieldContainer customFields)`
    pub fn new(custom_fields: Rc<RefCell<CustomFieldContainer>>) -> Self {
        Self { custom_fields }
    }

/// Read the raw timestamp value from a variable-length data block.
    ///
    /// Corresponds to Java: `private Object getRawTimestampValue(Var2Data varData, Integer id, Integer type)`
    ///
/// A 512-byte data item is decoded as a UTF-16LE string (Java keeps this branch as-is),
/// while other data items of length at least 4 are decoded as a combined "time + day count" timestamp.
    fn get_raw_timestamp_value(
        &self,
        var_data: &Var2Data,
        id: Option<i32>,
        type_: Option<i32>,
    ) -> Option<FieldValue> {
        // Java: byte[] data = varData.getByteArray(id, type);
        //       if (data != null) { ... }
        let data = var_data.get_byte_array(id, type_)?;

        if data.len() == 512 {
            // Java: result = MPPUtility.getUnicodeString(data, 0);
            Some(FieldValue::String(MPPUtility::get_unicode_string(&data, 0)))
        } else if data.len() >= 4 {
            // Java: result = MPPUtility.getTimestamp(data, 0);
            MPPUtility::get_timestamp(&data, 0).map(FieldValue::DateTime)
        } else {
// Java: result stays null when the length is less than 4
            None
        }
    }
}

impl VarDataFieldReader for TimestampVarDataFieldReader {
/// Corresponds to Java: `public CustomFieldContainer getCustomFields()` (Rust-specific accessor, see the trait documentation)
    fn get_custom_fields(&self) -> &Rc<RefCell<CustomFieldContainer>> {
        &self.custom_fields
    }

/// Read the timestamp data item.
    ///
    /// Corresponds to Java: `@Override protected Object readValue(Var2Data varData, Integer id, Integer type)`
    fn read_value(
        &self,
        var_data: &Var2Data,
        id: Option<i32>,
        type_: Option<i32>,
    ) -> Option<FieldValue> {
        // Java: return getRawTimestampValue(varData, id, type);
        self.get_raw_timestamp_value(var_data, id, type_)
    }

/// Accepts only `LocalDateTime` values; other types return `None` (Java returns null).
    ///
    /// Corresponds to Java: `@Override protected Object coerceValue(Object value)`
    fn coerce_value(&self, value: Option<&FieldValue>) -> Option<FieldValue> {
        // Java: if (value instanceof LocalDateTime) { return value; } return null;
        match value? {
            FieldValue::DateTime(value) => Some(FieldValue::DateTime(*value)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::var_data_field_reader::test_support::{
        container_with_item, guid_bytes, value_list_data_with_id, var_data,
    };
    use super::*;
    use crate::custom_field_lookup_table::CustomFieldValueItem;
    use chrono::{Days, NaiveDate, NaiveTime};

/// Data item type (used consistently in tests).
    const TYPE: i32 = 0x12;

/// Expected value of the timestamp data item: 1983-12-31 + 1000 days.
    fn expected_timestamp() -> chrono::NaiveDateTime {
        let date = NaiveDate::from_ymd_opt(1983, 12, 31)
            .and_then(|epoch| epoch.checked_add_days(Days::new(1000)))
            .expect("date can be computed");
        chrono::NaiveDateTime::new(date, NaiveTime::from_hms_opt(0, 0, 0).expect("valid time"))
    }

/// A timestamp data item in the form "time (tenths of a minute, 2 bytes) + day count (2 bytes)".
    fn timestamp_data() -> Vec<u8> {
        vec![0x00, 0x00, 0xE8, 0x03]
    }

/// Read a timestamp data item in the "time + day count" form.
    #[test]
    fn reads_timestamp_value() {
        let var_data = var_data(&[(1, TYPE, timestamp_data())]);
        let reader = TimestampVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(
            reader.read_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::DateTime(expected_timestamp()))
        );
    }

/// A 512-byte data item is decoded as a UTF-16LE string (Java's special branch).
    #[test]
    fn reads_512_byte_value_as_string() {
        let mut data = vec![0u8; 512];
// UTF-16LE representation of "A", with the rest zero (the first double 0 byte pair marks the end of the string)
        data[0] = 0x41;

        let var_data = var_data(&[(1, TYPE, data)]);
        let reader = TimestampVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(
            reader.read_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::String("A".to_owned()))
        );
    }

/// When the day count is NA (65535) returns `None` (null) per Java rules.
    #[test]
    fn na_day_count_returns_none() {
// time = 0, day count = 65535
        let var_data = var_data(&[(1, TYPE, vec![0x00, 0x00, 0xFF, 0xFF])]);
        let reader = TimestampVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(reader.read_value(&var_data, Some(1), Some(TYPE)), None);
    }

/// Returns `None` when the data item is missing or shorter than 4 bytes (Java keeps null).
    #[test]
    fn missing_or_short_data_returns_none() {
        let var_data = var_data(&[
            (1, TYPE, timestamp_data()),
            (2, TYPE, vec![0x00, 0x00, 0x00]),
            (3, TYPE, Vec::new()),
        ]);
        let reader = TimestampVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(reader.read_value(&var_data, Some(999), Some(TYPE)), None);
        assert_eq!(reader.read_value(&var_data, None, None), None);
        assert_eq!(reader.read_value(&var_data, Some(2), Some(TYPE)), None);
        assert_eq!(reader.read_value(&var_data, Some(3), Some(TYPE)), None);
    }

/// Value list branch: a `LocalDateTime` in a value item passes coercion and is returned as-is.
    #[test]
    fn value_list_item_is_coerced_to_date_time() {
        let mut item = CustomFieldValueItem::new(11);
        item.set_value(Some(FieldValue::DateTime(expected_timestamp())));
        let container = container_with_item(item);

        let var_data = var_data(&[(
            1,
            TYPE,
            value_list_data_with_id(11, guid_bytes(3)),
        )]);

        let reader = TimestampVarDataFieldReader::new(container);
        assert_eq!(
            reader.get_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::DateTime(expected_timestamp()))
        );
    }

/// `coerce_value`: only `LocalDateTime` values pass (`LocalDate` does not, consistent with Java's `instanceof`).
    #[test]
    fn coerce_value_accepts_date_time_only() {
        let reader = TimestampVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(
            reader.coerce_value(Some(&FieldValue::DateTime(expected_timestamp()))),
            Some(FieldValue::DateTime(expected_timestamp()))
        );
        assert_eq!(
            reader.coerce_value(Some(&FieldValue::Date(
                NaiveDate::from_ymd_opt(2024, 5, 1).expect("valid date")
            ))),
            None
        );
        assert_eq!(
            reader.coerce_value(Some(&FieldValue::String("2024-05-01".to_owned()))),
            None
        );
        assert_eq!(reader.coerce_value(None), None);
    }
}
