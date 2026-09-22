//! Corresponds to Java: `org/mpxj/mpp/VarDataFieldReader.java`
//!
//! Strategy class that reads field values from a variable-length data block ([`Var2Data`]) by `(unique ID, data type)`:
//! it first checks whether the first two bytes of the data item are a "value list" flag -- if so, it looks up the actual value in the custom
//! field container by the list item's unique ID / GUID; otherwise it delegates to the subclass's `readValue` to decode by type. The subclass's `coerceValue` is responsible for
//! coercing the value to the target type (returning `null` when the type does not match).
//!
//! # Restoring inheritance / interfaces
//!
//! Java: `abstract class VarDataFieldReader` (an abstract class with one concrete method `getValue` and two abstract methods).
//! On the Rust side it is restored as a trait by convention:
//!
//! | Java | Rust |
//! |---|---|
//! | `abstract class VarDataFieldReader` | [`VarDataFieldReader`] trait |
//! | `public Object getValue(Var2Data, Integer, Integer)` (a concrete method) | the trait's **default method** [`VarDataFieldReader::get_value`], with a body matching Java line by line |
//! | `protected abstract Object readValue(...)` | the trait's required method [`VarDataFieldReader::read_value`], provided by three implementation structs |
//! | `protected abstract Object coerceValue(Object)` | the trait's required method [`VarDataFieldReader::coerce_value`]; Java's `Object` (which may be null) -> `Option<&FieldValue>` |
//! | `private final CustomFieldContainer m_customFields` (a private field) | the trait's required accessor [`VarDataFieldReader::get_custom_fields`] (Rust-specific) |
//! | subclasses access `m_customFields` through inheritance | default methods of a trait cannot access the implementation struct's fields, so an accessor is declared; each of the three implementation structs holds its own `Rc<RefCell<CustomFieldContainer>>` and forwards |
//! | `private Object getValueByID(byte[])` (a private method) | the private free function `get_value_by_id` in this file (it needs `&dyn VarDataFieldReader` to obtain the container and the implementation class's `coerce_value`) |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `Object` return value | `Option<FieldValue>` | this crate uses [`FieldValue`] to carry Java's `Object` field values (able to represent `Double` / `String` / `LocalDateTime` and so on); `null` -> `None` |
//! | `Integer id` / `Integer type` | `Option<i32>` | the "`null` -> `Option<T>`" convention |
//! | `byte[] data` | `Vec<u8>` (used as `&[u8]` once retrieved) | Java's `byte[]` argument becomes `&[u8]` on the Rust side |
//! | `data.length` (throws an NPE when `data` is null) | returns `None` when the data item cannot be retrieved | follows the "do not panic" convention; this branch is actually unreachable (a flag hit means the data item exists and has length >= 2) |
//! | `ByteArrayHelper.getInt(data, 2)` throws `ArrayIndexOutOfBoundsException` when out of bounds | returns `None` when fewer than 6 bytes are available | same as above |
//! | `UUID guid` (which may be null) | `Option<Uuid>` | [`MPPUtility::get_guid`] returning `None` represents Java's null; no GUID lookup is performed in that case (Java's `HashMap.get(null)` also finds nothing) |
//! | `LocalDateHelper.getLocalDateTime(...)` | [`LocalDateHelper::get_local_date_time`] | returns `Option<NaiveDateTime>` (`null` -> `None`) |
//! | `m_customFields.getCustomFieldValueItemByGuid(...)` | calls `try_borrow()` first before accessing the container | returns `None` instead of panicking on a `RefCell` borrow conflict (see the [`get_value_by_id`] comment) |
//!
//! Skipped Java methods: none.
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::local_date_helper::LocalDateHelper;
use crate::custom_field_container::CustomFieldContainer;
use crate::field_value::FieldValue;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::var2_data::Var2Data;

/// The "value list with ID" flag.
///
/// Corresponds to Java: `private static final int VALUE_LIST_WITH_ID_MASK = 0x0701;`
const VALUE_LIST_WITH_ID_MASK: i32 = 0x0701;

/// The "value list without ID" flag.
///
/// Corresponds to Java: `private static final int VALUE_LIST_WITHOUT_ID_MASK = 0x0401;`
const VALUE_LIST_WITHOUT_ID_MASK: i32 = 0x0401;

/// Strategy for reading field values by type from a variable-length data block.
///
/// Corresponds to Java: `abstract class VarDataFieldReader`
///
/// [`get_value`](VarDataFieldReader::get_value) is a "template method", whose flow is provided by this trait;
/// the decoding and coercion for concrete types is determined by the implementation struct's
/// [`read_value`](VarDataFieldReader::read_value) /
/// [`coerce_value`](VarDataFieldReader::coerce_value).
pub trait VarDataFieldReader {
    /// Get the custom field container.
    ///
    /// Corresponds to Java field: `private final CustomFieldContainer m_customFields`
    ///
    /// Rust-specific: in Java subclasses access the base class's private field through inheritance, whereas a trait's default method cannot access the implementation
    /// struct's fields, so this accessor restores that access path. The three implementation structs simply return the
    /// `Rc<RefCell<CustomFieldContainer>>` they hold.
    fn get_custom_fields(&self) -> &Rc<RefCell<CustomFieldContainer>>;

    /// Get a value from a variable-length data block.
    ///
    /// Corresponds to Java: `public Object getValue(Var2Data varData, Integer id, Integer type)`
    ///
    /// When the first two bytes of the data item are a value list flag, it takes the branch that looks up the custom field container by the list item's unique ID / GUID
    /// (see the private function `get_value_by_id`); otherwise it delegates to the implementation class's `read_value`.
    fn get_value(
        &self,
        var_data: &Var2Data,
        id: Option<i32>,
        type_: Option<i32>,
    ) -> Option<FieldValue> {
        // Java: int flag = varData.getShort(id, type);
        let flag = var_data.get_short(id, type_);

        if flag == VALUE_LIST_WITH_ID_MASK || flag == VALUE_LIST_WITHOUT_ID_MASK {
            // Java: byte[] data = varData.getByteArray(id, type);
            //       if (data.length == 4) ...
            // Java throws an NPE for data == null; the Rust side returns None (this branch is actually unreachable).
            let data = var_data.get_byte_array(id, type_)?;

            if data.len() == 4 {
                // Java comment: not sure whether this is correct; no sample returning a non-null date is available.
                // Java: result = LocalDateHelper.getLocalDateTime(MPPUtility.getDate(data, 2));
                LocalDateHelper::get_local_date_time(MPPUtility::get_date(&data, 2))
                    .map(FieldValue::DateTime)
            } else {
                // Java: result = getValueByID(data);
                get_value_by_id(self, &data)
            }
        } else {
            // Java: result = readValue(varData, id, type);
            self.read_value(var_data, id, type_)
        }
    }

    /// Overridden by the implementation class: read the data item according to its own target type.
    ///
    /// Corresponds to Java: `protected abstract Object readValue(Var2Data varData, Integer id, Integer type)`
    fn read_value(
        &self,
        var_data: &Var2Data,
        id: Option<i32>,
        type_: Option<i32>,
    ) -> Option<FieldValue>;

    /// Overridden by the implementation class: coerce the value obtained from the value list item to the target type.
    ///
    /// Corresponds to Java: `protected abstract Object coerceValue(Object value)`
    ///
    /// Java's `value` may be null (when the value item has no value set) -> `None` on the Rust side.
    fn coerce_value(&self, value: Option<&FieldValue>) -> Option<FieldValue>;
}

/// Handles a data block in the form "value = integer unique ID + GUID".
///
/// Corresponds to Java: `private Object getValueByID(byte[] data)`
///
/// In Java this is a private instance method of the base class; on the Rust side it is changed to a free function taking the reader,
/// because it needs the container and the implementation class's `coerce_value`. The parameter uses a `?Sized` generic (rather than `&dyn`) because
/// the `Self` in a trait default method may itself be `dyn VarDataFieldReader`, and an unsized `&Self`
/// cannot be coerced directly into a trait object.
///
/// The data block is 26 bytes in total: a 2-byte mask + a 4-byte unique ID + a 16-byte GUID + 4 unknown bytes.
fn get_value_by_id<T: VarDataFieldReader + ?Sized>(
    reader: &T,
    data: &[u8],
) -> Option<FieldValue> {
    // Java: int uniqueId = ByteArrayHelper.getInt(data, 2);
    // Java throws ArrayIndexOutOfBoundsException when fewer than 6 bytes are available; the Rust side returns None.
    if data.len() < 6 {
        return None;
    }
    let unique_id = ByteArrayHelper::get_int(data, 2);

    // Java: UUID guid = MPPUtility.getGUID(data, 6);
    // When fewer than 16 bytes are available, the Rust side yields None (Java returns null).
    let guid = MPPUtility::get_guid(data, 6);

    // Java's container method is read-only; the Rust side uses try_borrow to avoid panicking when "the container is mutably borrowed";
    // on a borrow conflict it is treated as "not found" (this case does not occur in Java).
    let container = reader.get_custom_fields().try_borrow().ok()?;

    // Java: if (uniqueId == -1) { item = m_customFields.getCustomFieldValueItemByGuid(guid); }
    //       else { item = ...getCustomFieldValueItemByUniqueID(uniqueId);
    //              if (item == null) { item = ...getCustomFieldValueItemByGuid(guid); } }
    let item = if unique_id == -1 {
        guid.and_then(|guid| container.get_custom_field_value_item_by_guid(guid))
    } else {
        match container.get_custom_field_value_item_by_unique_id(unique_id) {
            Some(item) => Some(item),
            // Java comment: fall back to the GUID when the unique ID is not found
            None => guid.and_then(|guid| container.get_custom_field_value_item_by_guid(guid)),
        }
    };

    // Java: return item == null ? null : coerceValue(item.getValue());
    let item = item?;
    // The value item is itself a RefCell: try_borrow is likewise used to avoid panicking
    let item = item.try_borrow().ok()?;
    reader.coerce_value(item.get_value())
}

/// Unit test support for this file: manually assemble `Var2Data` and its metadata.
///
/// The tests in the three implementation class files also reuse the constructors here (the byte layout of the `VarMeta9` block matches
/// that of the tests in [`crate::mpp::var2_data`]).
#[cfg(test)]
pub(crate) mod test_support {
    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::custom_field_container::CustomFieldContainer;
    use crate::custom_field_lookup_table::CustomFieldValueItem;
    use crate::mpp::var2_data::Var2Data;
    use crate::mpp::var_meta9::VarMeta9;
    use crate::project_file::ProjectFile;

    /// Manually assemble a data block: each item has the same form as in MPP (a 4-byte length + the data itself).
    ///
    /// Returns "the data block + the offset of each item".
    pub(crate) fn data_block(items: &[Vec<u8>]) -> (Vec<u8>, Vec<i32>) {
        let mut data = Vec::new();
        let mut offsets = Vec::new();

        for item in items {
            offsets.push(data.len() as i32);
            data.extend_from_slice(&(item.len() as i32).to_le_bytes());
            data.extend_from_slice(item);
        }

        (data, offsets)
    }

    /// Assemble the matching VarMeta9 block from "unique ID + type + data item index".
    pub(crate) fn var_meta9_bytes(
        entries: &[(i32, i32, usize)],
        offsets: &[i32],
        data_size: i32,
    ) -> Vec<u8> {
        let mut data = Vec::new();
        // magic = 0xFADFADBA (little-endian)
        data.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(entries.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&data_size.to_le_bytes());

        for (unique_id, type_, index) in entries {
            data.extend_from_slice(&unique_id.to_le_bytes()[..3]);
            data.push(*type_ as u8);
            data.extend_from_slice(&offsets[*index].to_le_bytes());
        }

        data
    }

    /// Construct a `Var2Data` from a list of `(unique ID, data type, data)`.
    pub(crate) fn var_data(entries: &[(i32, i32, Vec<u8>)]) -> Var2Data {
        let items: Vec<Vec<u8>> = entries.iter().map(|(_, _, data)| data.clone()).collect();
        let (data, offsets) = data_block(&items);
        let table: Vec<(i32, i32, usize)> = entries
            .iter()
            .enumerate()
            .map(|(index, (id, type_, _))| (*id, *type_, index))
            .collect();

        let meta = VarMeta9::new(&var_meta9_bytes(&table, &offsets, data.len() as i32))
            .expect("valid metadata");
        let file = ProjectFile::new();
        Var2Data::new(&file, Rc::new(meta), &data)
    }

    /// Construct a custom field container with the given value item already registered.
    pub(crate) fn container_with_item(
        item: CustomFieldValueItem,
    ) -> Rc<RefCell<CustomFieldContainer>> {
        let container = CustomFieldContainer::new();
        container
            .borrow_mut()
            .register_value(Rc::new(RefCell::new(item)));
        container
    }

    /// Assemble a "value list with ID" data block (the mask is [`VALUE_LIST_WITH_ID_MASK`](super::VALUE_LIST_WITH_ID_MASK)).
    pub(crate) fn value_list_data_with_id(unique_id: i32, guid: [u8; 16]) -> Vec<u8> {
        value_list_data(super::VALUE_LIST_WITH_ID_MASK, unique_id, guid)
    }

    /// Assemble a "value list" data block: a 2-byte mask + a 4-byte unique ID + a 16-byte GUID + 4 unknown bytes.
    pub(crate) fn value_list_data(mask: i32, unique_id: i32, guid: [u8; 16]) -> Vec<u8> {
        let mut data = Vec::new();
        // The mask is written with Java's getShort semantics (little-endian)
        data.extend_from_slice(&mask.to_le_bytes()[..2]);
        data.extend_from_slice(&unique_id.to_le_bytes());
        data.extend_from_slice(&guid);
        data.extend_from_slice(&[0u8; 4]);
        data
    }

    /// Assemble a 16-byte GUID according to the byte order of
    /// [`MPPUtility::get_guid`](crate::mpp::mpp_utility::MPPUtility::get_guid) (the first 4 / 2 / 2 bytes are reversed).
    pub(crate) fn guid_bytes(value: u128) -> [u8; 16] {
        let uuid = uuid::Uuid::from_u128(value);
        let bytes = uuid.as_bytes();
        [
            bytes[3], bytes[2], bytes[1], bytes[0], bytes[5], bytes[4], bytes[7], bytes[6],
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{container_with_item, guid_bytes, value_list_data, var_data};
    use super::*;
    use chrono::NaiveDate;
    use crate::custom_field_lookup_table::CustomFieldValueItem;
    use crate::mpp::double_var_data_field_reader::DoubleVarDataFieldReader;
    use crate::mpp::string_var_data_field_reader::StringVarDataFieldReader;

    /// The data item type (used uniformly in the tests).
    const TYPE: i32 = 0x12;

    /// A value list item is hit (unique ID is not -1): the return value is coerced to `Double` by the implementation class.
    #[test]
    fn value_list_item_is_looked_up_by_unique_id() {
        let mut item = CustomFieldValueItem::new(11);
        // Java's value item value can be any Object; an Integer is used here to verify the coercion of the Double reader
        item.set_value(Some(FieldValue::Integer(3)));
        let container = container_with_item(item);

        let var_data = var_data(&[(
            1,
            TYPE,
            value_list_data(VALUE_LIST_WITH_ID_MASK, 11, guid_bytes(7)),
        )]);

        let reader = DoubleVarDataFieldReader::new(container);
        assert_eq!(
            reader.get_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::Double(3.0))
        );
    }

    /// When the unique ID is -1, look up the value item by GUID.
    #[test]
    fn value_list_item_is_looked_up_by_guid_when_unique_id_is_minus_one() {
        let mut item = CustomFieldValueItem::new(11);
        item.set_guid(Some(uuid::Uuid::from_u128(7)));
        item.set_value(Some(FieldValue::Integer(5)));
        let container = container_with_item(item);

        let var_data = var_data(&[(
            1,
            TYPE,
            value_list_data(VALUE_LIST_WITH_ID_MASK, -1, guid_bytes(7)),
        )]);

        let reader = DoubleVarDataFieldReader::new(container);
        assert_eq!(
            reader.get_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::Double(5.0))
        );
    }

    /// Fall back to the GUID when the unique ID is not found.
    #[test]
    fn value_list_item_falls_back_to_guid() {
        let mut item = CustomFieldValueItem::new(11);
        item.set_guid(Some(uuid::Uuid::from_u128(9)));
        item.set_value(Some(FieldValue::Integer(5)));
        let container = container_with_item(item);

        // The unique ID does not match the registered value item
        let var_data = var_data(&[(
            1,
            TYPE,
            value_list_data(VALUE_LIST_WITH_ID_MASK, 42, guid_bytes(9)),
        )]);

        let reader = DoubleVarDataFieldReader::new(container);
        assert_eq!(
            reader.get_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::Double(5.0))
        );
    }

    /// Returns `None` when neither the unique ID nor the GUID is found (Java returns null).
    #[test]
    fn unknown_value_list_item_returns_none() {
        let mut item = CustomFieldValueItem::new(11);
        item.set_value(Some(FieldValue::Integer(5)));
        let container = container_with_item(item);

        let unmatched_var_data = var_data(&[(
            1,
            TYPE,
            value_list_data(VALUE_LIST_WITHOUT_ID_MASK, 42, guid_bytes(1)),
        )]);

        let reader = DoubleVarDataFieldReader::new(container);
        assert_eq!(
            reader.get_value(&unmatched_var_data, Some(1), Some(TYPE)),
            None
        );

        // The value item is registered but has no value set (Java's getValue returns null)
        let empty_item = CustomFieldValueItem::new(7);
        let container = container_with_item(empty_item);
        let empty_item_var_data = var_data(&[(
            1,
            TYPE,
            value_list_data(VALUE_LIST_WITHOUT_ID_MASK, 7, guid_bytes(1)),
        )]);
        let reader = DoubleVarDataFieldReader::new(container);
        assert_eq!(
            reader.get_value(&empty_item_var_data, Some(1), Some(TYPE)),
            None
        );
    }

    /// A 4-byte value list data block is decoded as a date (Java's `data.length == 4` branch).
    #[test]
    fn four_byte_value_list_is_decoded_as_date() {
        // mask 0x0701 + 1 day -> 1984-01-01
        let data = vec![0x01, 0x07, 0x01, 0x00];
        let var_data = var_data(&[(1, TYPE, data)]);

        let reader = DoubleVarDataFieldReader::new(container_with_item(CustomFieldValueItem::new(
            1,
        )));

        assert_eq!(
            reader.get_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::DateTime(
                NaiveDate::from_ymd_opt(1984, 1, 1)
                    .expect("valid date")
                    .and_hms_opt(0, 0, 0)
                    .expect("valid time")
            ))
        );
    }

    /// When the data item is missing, the flag reads as 0 (no value list flag hit), so it is delegated to the implementation class's `read_value`.
    #[test]
    fn missing_entry_is_delegated_to_read_value() {
        let var_data = var_data(&[(1, TYPE, value_list_data(VALUE_LIST_WITH_ID_MASK, 1, [0; 16]))]);

        // String reader: the data item does not exist -> Java's getUnicodeString returns null -> None
        let reader = StringVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));
        assert_eq!(reader.get_value(&var_data, Some(999), Some(TYPE)), None);
        assert_eq!(reader.get_value(&var_data, None, None), None);

        // Double reader: Java's getDouble returns 0 for a missing data item, so this is not None
        let reader = DoubleVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));
        assert_eq!(
            reader.get_value(&var_data, Some(999), Some(TYPE)),
            Some(FieldValue::Double(0.0))
        );
    }

    /// Returns `None` rather than panicking when the value list data block is too short (fewer than 6 bytes).
    #[test]
    fn truncated_value_list_data_returns_none() {
        // Only a mask is present; Java's ByteArrayHelper.getInt(data, 2) would throw an out-of-bounds exception
        let mask_only_var_data = var_data(&[(1, TYPE, vec![0x01, 0x04])]);
        let reader = DoubleVarDataFieldReader::new(container_with_item(CustomFieldValueItem::new(
            1,
        )));
        assert_eq!(
            reader.get_value(&mask_only_var_data, Some(1), Some(TYPE)),
            None
        );

        // 5 bytes (mask + 3 bytes) are likewise insufficient to read a 4-byte unique ID
        let five_byte_var_data = var_data(&[(1, TYPE, vec![0x01, 0x04, 0x00, 0x00, 0x00])]);
        let reader = DoubleVarDataFieldReader::new(container_with_item(CustomFieldValueItem::new(
            1,
        )));
        assert_eq!(
            reader.get_value(&five_byte_var_data, Some(1), Some(TYPE)),
            None
        );
    }

    /// When it is not a value list flag, it takes `read_value` (determined by the implementation class).
    #[test]
    fn non_list_flag_uses_read_value() {
        let var_data = var_data(&[(1, TYPE, vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF8, 0x3F])]);
        let reader = DoubleVarDataFieldReader::new(container_with_item(CustomFieldValueItem::new(
            1,
        )));

        assert_eq!(
            reader.get_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::Double(1.5))
        );
    }
}
