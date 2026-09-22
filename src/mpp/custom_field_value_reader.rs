//! Corresponds to Java: `org/mpxj/mpp/CustomFieldValueReader.java`
//!
//! Common implementation for reading custom field **value lists**: reads out from the fixed-length / variable-length data blocks of `TBkndOutlCode`
//! value items (unique ID, GUID, description, unknown data block, parent ID, value type, value), registers them with the custom field container by unique ID and
//! GUID, and attaches them to the corresponding field's lookup table by lookup table GUID.
//!
//! # Restoring inheritance (composition + forwarding)
//!
//! Java: `abstract class CustomFieldValueReader`, extended by `CustomFieldValueReader12` /
//! `CustomFieldValueReader14` (note: `CustomFieldValueReader9` does **not** extend this class; it is an
//! independent implementation with its own value list parsing logic). Rust has no inheritance, therefore:
//!
//! - This class is a **concrete struct** in Rust: Java's `abstract` is only used to forbid direct instantiation, and this class has no
//! abstract methods, so the Rust side introduces no superfluous type;
//! - [`CustomFieldValueReader12`](crate::mpp::custom_field_value_reader12::CustomFieldValueReader12)
//! and `CustomFieldValueReader14` each **hold one instance of this struct** (a private `base` field) and forward all public
//! methods one by one; at construction time, via [`CustomFieldValueReader::set_parent_offset`] /
//!   [`CustomFieldValueReader::set_type_offset`] /
//! [`CustomFieldValueReader::set_field_offset`], write the `protected` fields that Java subclasses assign directly.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `CustomFieldValueReader(ProjectFile, Map<UUID, FieldType>, VarMeta, Var2Data, FixedData, FixedData)` | Constructed in the same order, but the first parameter is changed to `&Rc<RefCell<ProjectFile>>`, and `Map<UUID, FieldType>` → `HashMap<Uuid, MppFieldType>` | `ProjectFile` is shared as `Rc<RefCell<..>>` in this crate; the `FieldType` interface is expressed by [`MppFieldType`] |
//! | `private final ProjectProperties m_properties` (taken from file at construction) | Do not store a reference; borrow on demand with `ProjectFile::try_borrow` | `ProjectProperties` is held by `ProjectFile`, and Rust cannot obtain a stable reference to it (consistent with the existing approach in `FieldMap`); on a borrow conflict, treat it as "properties unavailable" and never panic |
//! | `private final CustomFieldContainer m_container` | `Rc<RefCell<CustomFieldContainer>>` | As in Java, take `file.getCustomFields()` at construction |
//! | `protected int m_parentOffset / m_typeOffset / m_fieldOffset` | Private fields + `set_*` / `get_*` accessors (`usize`) | Java subclasses assign to the `protected` fields directly; Rust subclasses live in other files, hence accessors. The offset semantics is a non-negative byte index, expressed with `usize` |
//! | `protected Object getTypedValue(CustomFieldValueDataType, byte[])` | `pub fn get_typed_value(&self, Option<CustomFieldValueDataType>, &[u8]) -> Option<FieldValue>` | `Object` → [`FieldValue`]; `null` → `None`. When the value type is `null` (Java's `getInstance` did not match), guess as a string as in Java |
//! | `private String valueAsString(byte[])` | Private method `value_as_string` (returns [`FieldValue::String`]) | Java falls back to hexdump via `try/catch`; Rust's `MPPUtility::get_unicode_string` is a total function (it returns a string even for malformed data), so that fallback branch is unreachable, see the method comment |
//! | `ByteArrayHelper.getShort/getInt` throws `ArrayIndexOutOfBoundsException` when out of bounds | [`checked_short`] / [`checked_int`] return `None` and `continue` (skip that record) | Follows the "no panic" convention |
//! | `m_container.registerValue(item)` / `table.add(item)` / `table.set(index, item)` | `register_value(item.stored_item())` / `table.add(item.to_stored_item())` / `table.set(index, item.to_stored_item())` | Java puts the **same instance** in the container and the lookup table; Rust's lookup table stores by value (the element type is not `Rc`), so the lookup table side is a copy with identical content, while the container side shares the same storage as the instance constructed here (see [`CustomFieldValueItem`]) |
//! | `table.stream().noneMatch(...)` | `table.as_slice().iter().all(...)` | Java's `Stream` is not ported (this crate substitutes slice iteration) |
//!
//! Skipped Java methods: none.
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.
//!
//! # Known functional gaps
//!
//! The keys of `m_lookupTableMap` are lookup table GUIDs, and the values may be `null` in Java (`populateLookupTableMap`
//! puts in `null` when `FieldTypeHelper` cannot recognize the field number). Rust's `HashMap` value type is non-nullable,
//! so the caller only inserts fields that can be recognized; when not found, this class skips the lookup table handling of that value item just as Java does.

use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::mpp_field_type::MppFieldType;
use crate::custom_field_container::CustomFieldContainer;
use crate::custom_field_value_data_type::CustomFieldValueDataType;
use crate::field_value::FieldValue;
use crate::mpp::custom_field_value_item::CustomFieldValueItem;
use crate::mpp::fixed_data::FixedData;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_meta::VarMeta;
use crate::project_file::ProjectFile;
use crate::project_properties::ProjectProperties;

/// The offset of the value item unique ID within the fixed-length data block.
///
/// Corresponds to Java: `public static final int UNJQUE_ID_OFFSET = 4;`
/// (the constant name preserves the typo `UNJQUE` from the Java source).
pub const UNJQUE_ID_OFFSET: usize = 4;

/// The data type key of the value item "value" within the variable-length data block.
///
/// Corresponds to Java: `public static final Integer VALUE_LIST_VALUE = Integer.valueOf(22);`
pub const VALUE_LIST_VALUE: i32 = 22;

/// The data type key of the value item "description" within the variable-length data block.
///
/// Corresponds to Java: `public static final Integer VALUE_LIST_DESCRIPTION = Integer.valueOf(8);`
pub const VALUE_LIST_DESCRIPTION: i32 = 8;

/// The data type key of the value item "unknown data block" within the variable-length data block.
///
/// Corresponds to Java: `public static final Integer VALUE_LIST_UNKNOWN = Integer.valueOf(23);`
pub const VALUE_LIST_UNKNOWN: i32 = 23;

/// Common reading logic for custom field value lists.
///
/// Corresponds to Java: `abstract class CustomFieldValueReader`
pub struct CustomFieldValueReader {
/// The project file.
    ///
/// Corresponds to `m_properties = file.getProjectProperties()` in the Java constructor
/// (the Rust side changes this to borrowing on demand, see the type description).
    file: Rc<RefCell<ProjectFile>>,

/// The custom field container.
    ///
    /// Corresponds to Java field: `protected final CustomFieldContainer m_container;`
/// (in the constructor, `m_container = file.getCustomFields();`)
    container: Rc<RefCell<CustomFieldContainer>>,

/// Lookup table GUID → field.
    ///
    /// Corresponds to Java field: `protected final Map<UUID, FieldType> m_lookupTableMap;`
    lookup_table_map: HashMap<Uuid, MppFieldType>,

/// The variable-length data block metadata of the value list.
    ///
    /// Corresponds to Java field: `protected final VarMeta m_outlineCodeVarMeta;`
    outline_code_var_meta: Rc<dyn VarMeta>,

/// The variable-length data block of the value list.
    ///
    /// Corresponds to Java field: `protected final Var2Data m_outlineCodeVarData;`
    outline_code_var_data: Var2Data,

/// The fixed-length data block of the value list.
    ///
    /// Corresponds to Java field: `protected final FixedData m_outlineCodeFixedData;`
    outline_code_fixed_data: FixedData,

/// The second fixed-length data block of the value list (GUID and value type).
    ///
    /// Corresponds to Java field: `protected final FixedData m_outlineCodeFixedData2;`
    outline_code_fixed_data2: FixedData,

/// The offset of the parent unique ID within the fixed-length data block.
    ///
/// Corresponds to Java field: `protected int m_parentOffset;` (assigned by the subclass constructor)
    parent_offset: usize,

/// The offset of the value type within the second fixed-length data block.
    ///
/// Corresponds to Java field: `protected int m_typeOffset;` (assigned by the subclass constructor)
    type_offset: usize,

/// The offset of the lookup table GUID within the second fixed-length data block.
    ///
/// Corresponds to Java field: `protected int m_fieldOffset;` (assigned by the subclass constructor)
    field_offset: usize,
}

impl CustomFieldValueReader {
/// Constructor.
    ///
    /// Corresponds to Java: `public CustomFieldValueReader(ProjectFile file, Map<UUID, FieldType> lookupTableMap, VarMeta outlineCodeVarMeta, Var2Data outlineCodeVarData, FixedData outlineCodeFixedData, FixedData outlineCodeFixedData2)`
    ///
/// The three offsets (`m_parentOffset` / `m_typeOffset` / `m_fieldOffset`) are assigned directly by the subclass constructor in Java;
/// on the Rust side they are 0 after construction and are set by the subclass (or caller) with `set_*_offset`.
    pub fn new(
        file: &Rc<RefCell<ProjectFile>>,
        lookup_table_map: HashMap<Uuid, MppFieldType>,
        outline_code_var_meta: Rc<dyn VarMeta>,
        outline_code_var_data: Var2Data,
        outline_code_fixed_data: FixedData,
        outline_code_fixed_data2: FixedData,
    ) -> Self {
        // Java: m_container = file.getCustomFields();
// When the file is already mutably borrowed (which does not happen on the normal call path), degrade to a standalone container to avoid a panic.
        let container = match file.try_borrow() {
            Ok(file) => file.get_custom_fields(),
            Err(_) => CustomFieldContainer::new(),
        };

        Self {
            file: Rc::clone(file),
            container,
            lookup_table_map,
            outline_code_var_meta,
            outline_code_var_data,
            outline_code_fixed_data,
            outline_code_fixed_data2,
            parent_offset: 0,
            type_offset: 0,
            field_offset: 0,
        }
    }

/// Reads all value items.
    ///
    /// Corresponds to Java: `public void process()`
    pub fn process(&self) {
        // Java: for (int loop = 0; loop < m_outlineCodeFixedData.getItemCount(); loop++)
        for loop_index in 0..self.outline_code_fixed_data.get_item_count() {
            // Java: byte[] fixedData = m_outlineCodeFixedData.getByteArrayValue(loop);
            //       if (fixedData == null) { continue; }
            let Some(fixed_data) = self
                .outline_code_fixed_data
                .get_byte_array_value(loop_index)
            else {
                continue;
            };

            // Java: Integer id = Integer.valueOf(ByteArrayHelper.getShort(fixedData, UNJQUE_ID_OFFSET));
// Java throws ArrayIndexOutOfBoundsException when there is insufficient data; the Rust side skips that record.
            let Some(id) = checked_short(fixed_data, UNJQUE_ID_OFFSET) else {
                continue;
            };

            // Java: if (!m_outlineCodeVarMeta.containsKey(id)) { continue; }
            if !self.outline_code_var_meta.contains_key(Some(id)) {
                continue;
            }

            // Java: byte[] value = m_outlineCodeVarData.getByteArray(id, VALUE_LIST_VALUE);
            //       if (value == null) { continue; }
            let Some(value) = self
                .outline_code_var_data
                .get_byte_array(Some(id), Some(VALUE_LIST_VALUE))
            else {
                continue;
            };

            // Java: CustomFieldValueItem item = new CustomFieldValueItem(id);
            let item = CustomFieldValueItem::new(id);

            // Java: item.setDescription(m_outlineCodeVarData.getUnicodeString(id, VALUE_LIST_DESCRIPTION));
            item.set_description(
                self.outline_code_var_data
                    .get_unicode_string(Some(id), Some(VALUE_LIST_DESCRIPTION)),
            );

            // Java: item.setUnknown(m_outlineCodeVarData.getByteArray(id, VALUE_LIST_UNKNOWN));
            item.set_unknown(
                self.outline_code_var_data
                    .get_byte_array(Some(id), Some(VALUE_LIST_UNKNOWN)),
            );

            // Java: item.setParentUniqueID(Integer.valueOf(ByteArrayHelper.getShort(fixedData, m_parentOffset)));
// Java throws when out of bounds; the Rust side keeps null (`None`) when it cannot be read.
            item.set_parent_unique_id(checked_short(fixed_data, self.parent_offset));

            // Java: byte[] fixedData2 = m_outlineCodeFixedData2.getByteArrayValue(loop);
            //       item.setGUID(MPPUtility.getGUID(fixedData2, 0));
// Java's `getGUID` itself returns null for null / too-short data, so an empty slice is used here to express absence.
            let fixed_data2 = self
                .outline_code_fixed_data2
                .get_byte_array_value(loop_index)
                .unwrap_or(&[]);
            item.set_guid(MPPUtility::get_guid(fixed_data2, 0));

            // Java: UUID lookupTableGuid = MPPUtility.getGUID(fixedData2, m_fieldOffset);
            let lookup_table_guid = MPPUtility::get_guid(fixed_data2, self.field_offset);

            // Java: item.setType(CustomFieldValueDataType.getInstance(ByteArrayHelper.getShort(fixedData2, m_typeOffset)));
// `getInstance` returns null for an undefined value index, which is equivalent to Rust's `and_then`.
            item.set_type(
                checked_short(fixed_data2, self.type_offset)
                    .and_then(CustomFieldValueDataType::get_instance),
            );

            // Java: item.setValue(getTypedValue(item.getType(), value));
            item.set_value(self.get_typed_value(item.get_type(), &value));

            // Java: m_container.registerValue(item);
            self.container
                .borrow_mut()
                .register_value(item.stored_item());

            // Java: FieldType field = m_lookupTableMap.get(lookupTableGuid);
            //       if (field != null) { ... }
// Java's `getGUID` may return null, and the map has no null keys, so the Rust side likewise skips.
            let field = lookup_table_guid
                .and_then(|guid| self.lookup_table_map.get(&guid))
                .cloned();

            if let Some(field) = field {
                // Java: CustomFieldLookupTable table = m_container.getOrCreate(field).getLookupTable();
                let custom_field =
                    CustomFieldContainer::get_or_create(&self.container, field.to_field_key());
                let unique_id = item.get_unique_id();

// Java comment: check for duplicates; the unique ID suffices to determine a duplicate.
                // Java: if (table.stream().noneMatch(i -> i.getUniqueID().equals(item.getUniqueID())))
                let is_new = custom_field
                    .borrow()
                    .get_lookup_table()
                    .as_slice()
                    .iter()
                    .all(|existing| existing.get_unique_id() != unique_id);

                if is_new {
                    // Java: table.add(item); table.setGUID(lookupTableGuid);
                    let mut custom_field = custom_field.borrow_mut();
                    let table = custom_field.get_lookup_table_mut();
                    table.add(item.to_stored_item());
                    table.set_guid(lookup_table_guid);
                } else {
// Java comment: replace the old instance in the lookup table with the new one, ensuring the container and lookup table point to the same item.
// The Rust side stores the lookup table by value, so it writes a copy with identical content.
                    let index = custom_field
                        .borrow()
                        .get_lookup_table()
                        .as_slice()
                        .iter()
                        .position(|existing| existing.get_unique_id() == unique_id);

                    if let Some(index) = index {
                        custom_field
                            .borrow_mut()
                            .get_lookup_table_mut()
                            .set(index, item.to_stored_item());
                    }
                }
            }
        }
    }

/// Converts raw bytes read from the MPP file into a Java-typed value.
    ///
    /// Corresponds to Java: `protected Object getTypedValue(CustomFieldValueDataType type, byte[] value)`
    ///
/// Java's `default` branch covers both "an unlisted value type" and "`type == null`"; on the Rust side
/// `FLAG` and `None` take the same branch. Java's `getTimestamp` / `getAdjustedDuration` may return
/// `null`, so `Option<FieldValue>` is returned.
    pub fn get_typed_value(
        &self,
        type_: Option<CustomFieldValueDataType>,
        value: &[u8],
    ) -> Option<FieldValue> {
        // Java: if (type == null) { result = valueAsString(value); }
        let Some(type_) = type_ else {
            return Some(self.value_as_string(value));
        };

        match type_ {
            // Java: case DATE: case FINISH_DATE: result = MPPUtility.getTimestamp(value, 0);
            CustomFieldValueDataType::Date | CustomFieldValueDataType::FinishDate => {
                MPPUtility::get_timestamp(value, 0).map(FieldValue::DateTime)
            }

            // Java:
            // case DURATION:
            //    TimeUnit units = MPPUtility.getDurationTimeUnits(ByteArrayHelper.getShort(value, 4), m_properties.getDefaultDurationUnits());
            //    result = MPPUtility.getAdjustedDuration(m_properties, ByteArrayHelper.getInt(value, 0), units);
            CustomFieldValueDataType::Duration => {
// Java reads `m_properties` directly; the Rust side borrows on demand (on a borrow conflict it degrades to no default unit)
                let properties = self.try_project_properties();
                let default_duration_units = properties
                    .as_deref()
                    .map(ProjectProperties::get_default_duration_units);
// Java throws when out of bounds; the Rust side treats an unreadable value as 0 (`getDurationTimeUnits(0)` = DAYS)
                let units = MPPUtility::get_duration_time_units_with_default(
                    checked_short(value, 4).unwrap_or(0),
                    default_duration_units,
                );
                let duration = checked_int(value, 0).unwrap_or(0);

// Java's `getAdjustedDuration` returns null when duration == -1;
// when the project properties cannot be borrowed it likewise returns None (Java has no such case).
                properties
                    .as_deref()
                    .and_then(|properties| {
                        MPPUtility::get_adjusted_duration(properties, duration, units)
                    })
                    .map(FieldValue::Duration)
            }

            // Java: case COST: result = Double.valueOf(MPPUtility.getDouble(value, 0) / 100);
            CustomFieldValueDataType::Cost => Some(FieldValue::Double(
                MPPUtility::get_double(value, 0) / 100.0,
            )),

            // Java: case NUMBER: result = Double.valueOf(MPPUtility.getDouble(value, 0));
            CustomFieldValueDataType::Number => {
                Some(FieldValue::Double(MPPUtility::get_double(value, 0)))
            }

            // Java: case TEXT: result = MPPUtility.getUnicodeString(value, 0);
            CustomFieldValueDataType::Text => {
                Some(FieldValue::String(MPPUtility::get_unicode_string(value, 0)))
            }

            // Java: default: result = valueAsString(value);
// (Java's switch does not list FLAG and the constants are all enumerated, so only default is reachable)
            CustomFieldValueDataType::Flag => Some(self.value_as_string(value)),
        }
    }

/// Best-effort interpretation of a run of bytes as a string.
    ///
    /// Corresponds to Java: `private String valueAsString(byte[] value)`
    ///
/// When `MPPUtility.getUnicodeString` throws, Java falls back to
/// `ByteArrayHelper.hexdump(value, false)`; Rust's
/// [`MPPUtility::get_unicode_string`] is a "total function" (out-of-bounds and malformed data only ever return a string),
/// so that `catch` branch is unreachable, and only the decode result is kept here.
    fn value_as_string(&self, value: &[u8]) -> FieldValue {
        // Java: try { result = MPPUtility.getUnicodeString(value, 0); }
        //       catch (Exception ex) { result = ByteArrayHelper.hexdump(value, false); }
        FieldValue::String(MPPUtility::get_unicode_string(value, 0))
    }

/// Gets the offset of the "parent unique ID" within the fixed-length data block.
    ///
/// Corresponds to Java: `protected int m_parentOffset` (an accessor on the Rust side, see the type description)
    pub fn get_parent_offset(&self) -> usize {
        self.parent_offset
    }

/// Sets the offset of the "parent unique ID".
    ///
/// Corresponds to Java: `m_parentOffset = ...` in the subclass constructor
    pub fn set_parent_offset(&mut self, offset: usize) {
        self.parent_offset = offset;
    }

/// Gets the offset of the "value type" within the second fixed-length data block.
    ///
/// Corresponds to Java: `protected int m_typeOffset` (an accessor on the Rust side, see the type description)
    pub fn get_type_offset(&self) -> usize {
        self.type_offset
    }

/// Sets the offset of the "value type".
    ///
/// Corresponds to Java: `m_typeOffset = ...` in the subclass constructor
    pub fn set_type_offset(&mut self, offset: usize) {
        self.type_offset = offset;
    }

/// Gets the offset of the "lookup table GUID" within the second fixed-length data block.
    ///
/// Corresponds to Java: `protected int m_fieldOffset` (an accessor on the Rust side, see the type description)
    pub fn get_field_offset(&self) -> usize {
        self.field_offset
    }

/// Sets the offset of the "lookup table GUID".
    ///
/// Corresponds to Java: `m_fieldOffset = ...` in the subclass constructor
    pub fn set_field_offset(&mut self, offset: usize) {
        self.field_offset = offset;
    }

/// Borrows the project properties (Rust-specific: Java stores a reference to the properties object in the constructor).
    fn try_project_properties(&self) -> Option<Ref<'_, ProjectProperties>> {
        let file = self.file.try_borrow().ok()?;
        Some(Ref::map(file, |file| file.get_project_properties()))
    }
}

/// Reads a two-byte short integer; returns `None` when there is insufficient data.
///
/// Corresponds to Java: `ByteArrayHelper.getShort(byte[], int)` (Java throws when out of bounds;
/// the Rust side returns `None`).
pub(crate) fn checked_short(data: &[u8], offset: usize) -> Option<i32> {
    if offset.checked_add(2)? > data.len() {
        return None;
    }

    Some(ByteArrayHelper::get_short(data, offset))
}

/// Reads a four-byte integer; returns `None` when there is insufficient data.
///
/// Corresponds to Java: `ByteArrayHelper.getInt(byte[], int)` (Java throws when out of bounds).
pub(crate) fn checked_int(data: &[u8], offset: usize) -> Option<i32> {
    if offset.checked_add(4)? > data.len() {
        return None;
    }

    Some(ByteArrayHelper::get_int(data, offset))
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::collections::HashMap;
    use std::rc::Rc;

    use uuid::Uuid;

    use crate::common::mpp_field_type::MppFieldType;
    use crate::data_type::DataType;
    use crate::field_key::{CustomFieldKey, FieldKey};
    use crate::field_type_class::FieldTypeClass;
    use crate::mpp::fixed_data::FixedData;
    use crate::mpp::fixed_meta::FixedMeta;
    use crate::mpp::var2_data::Var2Data;
    use crate::mpp::var_meta::VarMeta;
    use crate::mpp::var_meta9::VarMeta9;
    use crate::project_file::ProjectFile;

/// Builds a `FixedMeta` block: a 16-byte block header + several 8-byte records (bytes 4..8 are the data offset).
    pub(crate) fn fixed_meta_bytes(offsets: &[i32]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFADFADBAu32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(offsets.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());

        for offset in offsets {
            let mut item = vec![0xEE_u8; 8];
            item[4..8].copy_from_slice(&offset.to_le_bytes());
            data.extend_from_slice(&item);
        }

        data
    }

/// Builds a `FixedData` from several records (each record's length is determined by its actual byte count in the data body).
    pub(crate) fn fixed_data(items: &[Vec<u8>]) -> FixedData {
        let mut offsets = Vec::new();
        let mut body = Vec::new();

        for item in items {
            offsets.push(body.len() as i32);
            body.extend_from_slice(item);
        }

// Metadata records are a fixed 8 bytes; the offset is taken from each record's real position in the data body
        let meta =
            FixedMeta::with_item_size(&fixed_meta_bytes(&offsets), 8).expect("FixedMeta is valid");
        FixedData::new(&meta, &body)
    }

/// Builds an empty `FixedData` (0 records).
    pub(crate) fn empty_fixed_data() -> FixedData {
        fixed_data(&[])
    }

/// Manually assembles a Var2Data block and returns "data block + offset per item".
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

/// Assembles the matching VarMeta9 block from "unique ID + type + data item index".
    pub(crate) fn var_meta9_bytes(
        entries: &[(i32, i32, usize)],
        offsets: &[i32],
        data_size: i32,
    ) -> Vec<u8> {
        let mut data = Vec::new();
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

/// Builds a `Var2Data` and returns its metadata handle at the same time (the reader constructor needs both).
    pub(crate) fn var_data_with_meta(
        entries: &[(i32, i32, Vec<u8>)],
    ) -> (Rc<dyn VarMeta>, Var2Data) {
        let items: Vec<Vec<u8>> = entries.iter().map(|(_, _, data)| data.clone()).collect();
        let (data, offsets) = data_block(&items);
        let table: Vec<(i32, i32, usize)> = entries
            .iter()
            .enumerate()
            .map(|(index, (id, type_, _))| (*id, *type_, index))
            .collect();

        let meta: Rc<dyn VarMeta> = Rc::new(
            VarMeta9::new(&var_meta9_bytes(&table, &offsets, data.len() as i32))
                .expect("valid metadata"),
        );
        let file = ProjectFile::new();
        let var_data = Var2Data::new(&file, Rc::clone(&meta), &data);
        (meta, var_data)
    }

/// Builds a 16-byte GUID using the byte order of [`crate::mpp::mpp_utility::MPPUtility::get_guid`].
    pub(crate) fn guid_bytes(value: u128) -> [u8; 16] {
        let uuid = Uuid::from_u128(value);
        let bytes = uuid.as_bytes();
        [
            bytes[3], bytes[2], bytes[1], bytes[0], bytes[5], bytes[4], bytes[7], bytes[6],
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]
    }

/// Builds the "lookup table GUID → field" map.
    pub(crate) fn lookup_table_map(entries: &[(u128, MppFieldType)]) -> HashMap<Uuid, MppFieldType> {
        entries
            .iter()
            .map(|(guid, field)| (Uuid::from_u128(*guid), field.clone()))
            .collect()
    }

/// Builds a custom field key (for tests).
    pub(crate) fn custom_field_key(unique_id: i32) -> FieldKey {
        FieldKey::Custom(CustomFieldKey::new(
            unique_id,
            FieldTypeClass::Task,
            DataType::String,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{
        custom_field_key, empty_fixed_data, fixed_data, guid_bytes, lookup_table_map,
        var_data_with_meta,
    };
    use super::*;
    use crate::duration::Duration;
    use crate::field_key::FieldKey;
    use crate::mpxj_enum::MpxjEnum;
    use crate::task_field::TaskField;
    use crate::time_unit::TimeUnit;
    use chrono::NaiveDate;

/// The value item unique ID (used uniformly by the tests).
    const ID: i32 = 1;

/// Fixed-length data block record length (reader12's offsets are 8 / 32 / 48, so the second block needs 50 bytes or more).
    const FIXED_ITEM_SIZE: usize = 20;
    const FIXED_DATA2_ITEM_SIZE: usize = 64;

/// The built-in task field NAME.
    fn name_field() -> MppFieldType {
        MppFieldType::Builtin(FieldKey::Task(TaskField::Name))
    }

/// Builds a fixed-length data block record.
    fn fixed_record(unique_id: i32, parent_unique_id: Option<i32>) -> Vec<u8> {
        let mut record = vec![0u8; FIXED_ITEM_SIZE];
        record[UNJQUE_ID_OFFSET..UNJQUE_ID_OFFSET + 2]
            .copy_from_slice(&(unique_id as u16).to_le_bytes());
        if let Some(parent_unique_id) = parent_unique_id {
            record[8..10].copy_from_slice(&(parent_unique_id as u16).to_le_bytes());
        }
        record
    }

/// Builds a record of the second fixed-length data block (value item GUID / lookup table GUID / value type).
    fn fixed_record2(
        item_guid: Option<u128>,
        lookup_table_guid: Option<u128>,
        type_: Option<CustomFieldValueDataType>,
    ) -> Vec<u8> {
        let mut record = vec![0u8; FIXED_DATA2_ITEM_SIZE];
        if let Some(guid) = item_guid {
            record[0..16].copy_from_slice(&guid_bytes(guid));
        }
        if let Some(guid) = lookup_table_guid {
            record[32..48].copy_from_slice(&guid_bytes(guid));
        }
        if let Some(type_) = type_ {
            record[48..50].copy_from_slice(&(type_.value() as u16).to_le_bytes());
        }
        record
    }

/// Builds a reader sample: 1 record in the fixed-length block + 1 record in the second fixed-length block + 3 items in the variable-length block.
    ///
/// Fixed-length block (20 bytes): [4..6) = ID, [8..10) = parent ID = 7
/// Second fixed-length block (64 bytes): [0..16) = value item GUID, [32..48) = lookup table GUID,
/// [48..50) = value type = NUMBER(15)
/// Variable-length block: 22 (value) = 8-byte double 1.5; 8 (description) = "desc"; 23 (unknown) = 3 bytes
    fn sample_reader() -> (CustomFieldValueReader, Rc<RefCell<ProjectFile>>) {
        let mut utf16 = Vec::new();
        for unit in "desc".encode_utf16() {
            utf16.extend_from_slice(&unit.to_le_bytes());
        }
        utf16.extend_from_slice(&[0x00, 0x00]);

        let (meta, var_data) = var_data_with_meta(&[
            (ID, VALUE_LIST_VALUE, 1.5f64.to_le_bytes().to_vec()),
            (ID, VALUE_LIST_DESCRIPTION, utf16),
            (ID, VALUE_LIST_UNKNOWN, vec![1, 2, 3]),
        ]);

        let file = ProjectFile::new();
        let mut reader = CustomFieldValueReader::new(
            &file,
            lookup_table_map(&[(9, name_field())]),
            meta,
            var_data,
            fixed_data(&[fixed_record(ID, Some(7))]),
            fixed_data(&[fixed_record2(
                Some(5),
                Some(9),
                Some(CustomFieldValueDataType::Number),
            )]),
        );
// Same offsets as CustomFieldValueReader12
        reader.set_parent_offset(8);
        reader.set_type_offset(48);
        reader.set_field_offset(32);

        (reader, file)
    }

/// The value item is registered with the container (by unique ID and GUID) and attached to the corresponding field's lookup table.
    #[test]
    fn process_registers_items_and_populates_lookup_table() {
        let (reader, file) = sample_reader();
        reader.process();

        let container = file.borrow().get_custom_fields();

// Can be found by unique ID / GUID
        let by_id = container
            .borrow()
            .get_custom_field_value_item_by_unique_id(ID)
            .expect("findable by unique ID");
        let by_guid = container
            .borrow()
            .get_custom_field_value_item_by_guid(Uuid::from_u128(5))
            .expect("findable by GUID");
        assert!(Rc::ptr_eq(&by_id, &by_guid));

        let item = by_id.borrow();
        assert_eq!(item.get_unique_id(), ID);
        assert_eq!(item.get_parent_unique_id(), Some(7));
        assert_eq!(item.get_description(), Some("desc"));
        assert_eq!(item.get_unknown(), Some(&[1u8, 2, 3][..]));
        assert_eq!(item.get_type(), Some(CustomFieldValueDataType::Number));
        assert_eq!(item.get_value(), Some(&FieldValue::Double(1.5)));

// Lookup table: field NAME is found by lookup table GUID; the table has one item and the GUID is set
        let custom_field = container
            .borrow()
            .get(FieldKey::Task(TaskField::Name))
            .expect("field configuration should have been created");
        let custom_field = custom_field.borrow();
        let table = custom_field.get_lookup_table();
        assert_eq!(table.len(), 1);
        assert_eq!(table.get_guid(), Some(Uuid::from_u128(9)));
        assert_eq!(table.get(0).map(|item| item.get_unique_id()), Some(ID));
        assert_eq!(
            table.get(0).and_then(|item| item.get_value()),
            Some(&FieldValue::Double(1.5))
        );
    }

/// Duplicate unique ID: the second occurrence replaces the element in the lookup table instead of appending.
    #[test]
    fn process_replaces_duplicate_entries_in_lookup_table() {
        let (meta, var_data) =
            var_data_with_meta(&[(ID, VALUE_LIST_VALUE, 2.5f64.to_le_bytes().to_vec())]);

        let file = ProjectFile::new();
        let mut reader = CustomFieldValueReader::new(
            &file,
            lookup_table_map(&[(9, name_field())]),
            meta,
            var_data,
            fixed_data(&[fixed_record(ID, Some(7)), fixed_record(ID, Some(7))]),
            fixed_data(&[
                fixed_record2(None, Some(9), Some(CustomFieldValueDataType::Number)),
                fixed_record2(None, Some(9), Some(CustomFieldValueDataType::Number)),
            ]),
        );
        reader.set_parent_offset(8);
        reader.set_type_offset(48);
        reader.set_field_offset(32);
        reader.process();

        let container = file.borrow().get_custom_fields();
        let custom_field = container
            .borrow()
            .get(FieldKey::Task(TaskField::Name))
            .expect("field configuration should have been created");
        let custom_field = custom_field.borrow();
// Still only one item (Java's "replace" branch)
        assert_eq!(custom_field.get_lookup_table().len(), 1);
        assert_eq!(
            custom_field
                .get_lookup_table()
                .get(0)
                .and_then(|item| item.get_value()),
            Some(&FieldValue::Double(2.5))
        );
    }

/// When the lookup table GUID cannot find a field in the map, register only with the container (Java's `field == null` branch).
    #[test]
    fn process_skips_unknown_lookup_table_guid() {
        let (meta, var_data) =
            var_data_with_meta(&[(ID, VALUE_LIST_VALUE, 1.5f64.to_le_bytes().to_vec())]);

        let file = ProjectFile::new();
        let mut reader = CustomFieldValueReader::new(
            &file,
// Empty map: no field found
            HashMap::new(),
            meta,
            var_data,
            fixed_data(&[fixed_record(ID, None)]),
            fixed_data(&[fixed_record2(None, Some(9), None)]),
        );
        reader.set_parent_offset(8);
        reader.set_type_offset(48);
        reader.set_field_offset(32);
        reader.process();

        let container = file.borrow().get_custom_fields();
        assert!(container
            .borrow()
            .get_custom_field_value_item_by_unique_id(ID)
            .is_some());
        assert_eq!(container.borrow().size(), 0);
    }

/// Skips when the unique ID is not in the variable-length data metadata (Java's `continue`).
    #[test]
    fn process_skips_items_without_var_data() {
        let (meta, var_data) =
            var_data_with_meta(&[(99, VALUE_LIST_VALUE, 1.5f64.to_le_bytes().to_vec())]);

        let file = ProjectFile::new();
        let mut reader = CustomFieldValueReader::new(
            &file,
            lookup_table_map(&[(9, name_field())]),
            meta,
            var_data,
            fixed_data(&[fixed_record(ID, None)]),
            fixed_data(&[fixed_record2(None, Some(9), None)]),
        );
        reader.set_parent_offset(8);
        reader.set_type_offset(48);
        reader.set_field_offset(32);
        reader.process();

        let container = file.borrow().get_custom_fields();
        assert!(container
            .borrow()
            .get_custom_field_value_item_by_unique_id(ID)
            .is_none());
    }

/// Does not panic when the fixed-length block is empty / the record is too short.
    #[test]
    fn process_handles_empty_and_truncated_data() {
// Fixed-length block is empty
        let (meta, var_data) =
            var_data_with_meta(&[(ID, VALUE_LIST_VALUE, 1.5f64.to_le_bytes().to_vec())]);
        let file = ProjectFile::new();
        let mut reader = CustomFieldValueReader::new(
            &file,
            lookup_table_map(&[(9, name_field())]),
            meta,
            var_data,
            empty_fixed_data(),
            empty_fixed_data(),
        );
        reader.set_parent_offset(8);
        reader.set_type_offset(48);
        reader.set_field_offset(32);
        reader.process();
        assert!(file
            .borrow()
            .get_custom_fields()
            .borrow()
            .get_custom_field_value_item_by_unique_id(ID)
            .is_none());

// Record is only 2 bytes: the unique ID cannot be read (Java throws an out-of-bounds exception)
        let (meta, var_data) =
            var_data_with_meta(&[(ID, VALUE_LIST_VALUE, 1.5f64.to_le_bytes().to_vec())]);
        let file = ProjectFile::new();
        let mut reader = CustomFieldValueReader::new(
            &file,
            lookup_table_map(&[(9, name_field())]),
            meta,
            var_data,
            fixed_data(&[vec![0x01, 0x00]]),
            fixed_data(&[vec![0x00, 0x00]]),
        );
        reader.set_parent_offset(8);
        reader.set_type_offset(48);
        reader.set_field_offset(32);
        reader.process();

        assert!(file
            .borrow()
            .get_custom_fields()
            .borrow()
            .get_custom_field_value_item_by_unique_id(ID)
            .is_none());
    }

/// Value type mapping (the branches of `getTypedValue`).
    #[test]
    fn typed_value_maps_each_type() {
        let (reader, file) = sample_reader();

// Project properties affect duration conversion
        {
            let mut file = file.borrow_mut();
            let properties = file.get_project_properties_mut();
            properties.set_minutes_per_day(Some(480));
            properties.set_days_per_month(Some(20));
        }

// DATE: 4-byte timestamp (the low 2 bytes are seconds within the day, the high 2 bytes are days)
        let date_bytes = [0x00, 0x00, 0x02, 0x00];
        assert_eq!(
            reader.get_typed_value(Some(CustomFieldValueDataType::Date), &date_bytes),
            Some(FieldValue::DateTime(
                NaiveDate::from_ymd_opt(1984, 1, 2)
                    .expect("valid date")
                    .and_hms_opt(0, 0, 0)
                    .expect("valid time")
            ))
        );
// FINISH_DATE and DATE take the same branch
        assert_eq!(
            reader.get_typed_value(Some(CustomFieldValueDataType::FinishDate), &date_bytes),
            reader.get_typed_value(Some(CustomFieldValueDataType::Date), &date_bytes)
        );

// DURATION: 4-byte duration + 2-byte unit (7 = days)
        let mut duration_bytes = 4800i32.to_le_bytes().to_vec();
        duration_bytes.extend_from_slice(&7u16.to_le_bytes());
        assert_eq!(
            reader.get_typed_value(Some(CustomFieldValueDataType::Duration), &duration_bytes),
            Some(FieldValue::Duration(Duration::get_instance(
                1.0,
                TimeUnit::Days
            )))
        );

// COST: a double in "cents" units
        assert_eq!(
            reader.get_typed_value(
                Some(CustomFieldValueDataType::Cost),
                &250.0f64.to_le_bytes()
            ),
            Some(FieldValue::Double(2.5))
        );

// NUMBER: double
        assert_eq!(
            reader.get_typed_value(
                Some(CustomFieldValueDataType::Number),
                &1.5f64.to_le_bytes()
            ),
            Some(FieldValue::Double(1.5))
        );

        // TEXT:UTF-16LE
        assert_eq!(
            reader.get_typed_value(
                Some(CustomFieldValueDataType::Text),
                &[0x41, 0x00, 0x42, 0x00, 0x00, 0x00]
            ),
            Some(FieldValue::String("AB".to_owned()))
        );

// Both FLAG and "unknown type" are guessed as a string (Java's default branch)
        assert_eq!(
            reader.get_typed_value(
                Some(CustomFieldValueDataType::Flag),
                &[0x41, 0x00, 0x00, 0x00]
            ),
            Some(FieldValue::String("A".to_owned()))
        );
        assert_eq!(
            reader.get_typed_value(None, &[0x41, 0x00, 0x00, 0x00]),
            Some(FieldValue::String("A".to_owned()))
        );
    }

/// Missing / malformed data does not panic and returns Java's default values.
    #[test]
    fn typed_value_handles_missing_data() {
        let (reader, _file) = sample_reader();

// Empty data: the date timestamp cannot be read → None
        assert_eq!(
            reader.get_typed_value(Some(CustomFieldValueDataType::Date), &[]),
            None
        );
// Duration: an out-of-bounds read is treated as 0 → 0 days (Java's `getInt` throws when out of bounds)
        assert_eq!(
            reader.get_typed_value(Some(CustomFieldValueDataType::Duration), &[]),
            Some(FieldValue::Duration(Duration::get_instance(
                0.0,
                TimeUnit::Days
            )))
        );
// Double out of bounds → 0 (consistent with Java's `getDouble`)
        assert_eq!(
            reader.get_typed_value(Some(CustomFieldValueDataType::Cost), &[0x01]),
            Some(FieldValue::Double(0.0))
        );
        assert_eq!(
            reader.get_typed_value(Some(CustomFieldValueDataType::Number), &[]),
            Some(FieldValue::Double(0.0))
        );
// Empty string
        assert_eq!(
            reader.get_typed_value(Some(CustomFieldValueDataType::Text), &[]),
            Some(FieldValue::String(String::new()))
        );

// duration == -1 → Java's `getAdjustedDuration` returns null
        let mut duration_bytes = (-1i32).to_le_bytes().to_vec();
        duration_bytes.extend_from_slice(&7u16.to_le_bytes());
        assert_eq!(
            reader.get_typed_value(Some(CustomFieldValueDataType::Duration), &duration_bytes),
            None
        );
    }

/// Round-trip of the offset accessors (Java's `protected` fields).
    #[test]
    fn offsets_round_trip() {
        let (mut reader, _file) = sample_reader();

        assert_eq!(reader.get_parent_offset(), 8);
        assert_eq!(reader.get_type_offset(), 48);
        assert_eq!(reader.get_field_offset(), 32);

        reader.set_parent_offset(10);
        reader.set_type_offset(16);
        reader.set_field_offset(18);
        assert_eq!(reader.get_parent_offset(), 10);
        assert_eq!(reader.get_type_offset(), 16);
        assert_eq!(reader.get_field_offset(), 18);
    }

/// A custom field (`FieldKey::Custom`) can also be attached to a lookup table.
    #[test]
    fn process_handles_custom_field_keys() {
        let (meta, var_data) =
            var_data_with_meta(&[(ID, VALUE_LIST_VALUE, vec![0x41, 0x00, 0x00, 0x00])]);

        let file = ProjectFile::new();
        let mut reader = CustomFieldValueReader::new(
            &file,
            lookup_table_map(&[(9, MppFieldType::Builtin(custom_field_key(3)))]),
            meta,
            var_data,
            fixed_data(&[fixed_record(ID, None)]),
            fixed_data(&[fixed_record2(
                None,
                Some(9),
                Some(CustomFieldValueDataType::Text),
            )]),
        );
        reader.set_parent_offset(8);
        reader.set_type_offset(48);
        reader.set_field_offset(32);
        reader.process();

        let container = file.borrow().get_custom_fields();
        let custom_field = container
            .borrow()
            .get(custom_field_key(3))
            .expect("custom field should have been created");
        assert_eq!(custom_field.borrow().get_lookup_table().len(), 1);
    }
}
