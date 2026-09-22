//! Corresponds to Java: `org/mpxj/mpp/CustomFieldValueReader9.java`
//!
//! Value list reader for MPP9. Unlike 12 / 14, this class does **not inherit**
//! [`CustomFieldValueReader`](crate::mpp::custom_field_value_reader::CustomFieldValueReader),
//! it parses two data locations itself:
//!
//! 1. The `TASK_FIELD_ATTRIBUTES` block in `Props` — "field number → value list (value array + description array)";
//! 2. The `TBkndOutlCode` store — values/descriptions for enterprise outline codes (the fixed block gives field numbers, the variable block gives text).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `CustomFieldValueReader9(DirectoryEntry projectDir, ProjectFile file, Props projectProps)` | `new(&DirectoryEntry, &Rc<RefCell<ProjectFile>>, &Props)` | By convention `ProjectFile` is shared as `Rc<RefCell<..>>`; `Props` is borrowed |
//! | `private final Props m_projectProps` (stores the reference; takes `TASK_FIELD_ATTRIBUTES` in `processCustomFieldValues`) | Takes a copy of `TASK_FIELD_ATTRIBUTES` at construction time and stores it in `task_field_attributes` | `Props` does not implement `Clone`, and the established practice in this crate (see `DocumentInputStreamFactory`) is to "take the needed values at construction time and not hold `Props`"; the data content does not change between construction and parsing, so the semantics are equivalent |
//! | `private final ProjectProperties m_properties` | Does not store the reference; borrows on demand via `ProjectFile::try_borrow` | Same trade-off as `CustomFieldValueReader` |
//! | `public void process() throws IOException` | `pub fn process(&mut self) -> MpxjResult<()>` | The only mutable state in Java is the unique ID counter, which the Rust side places on `&mut self`; Java's `IOException` corresponds to [`MpxjResult`] (`FixedMeta` / `VarMeta9` parse failure) |
//! | `Map<FieldType, List<Pair<String, String>>>` | `HashMap<Option<MppFieldKey>, (Option<MppFieldType>, Vec<(Option<String>, Option<String>)>)>` | Java's `HashMap` allows a `null` key (the case where `map.get(id)` misses), which the Rust side represents with `Option<MppFieldKey>`; it also stores "the field corresponding to that key" in the value, in order to reproduce `field == null ? "Unknown" : field.getName()`. `Pair` is represented by a tuple (the `Pair` type itself is usable, but no semantics beyond `getFirst`/`getSecond` are needed here) |
//! | `List<Object> convertType(...)` | `Vec<Option<FieldValue>>` | Java's list elements may be `null` (e.g. `getTimestamp` / `getAdjustedDuration` returning null), which the Rust side represents with `Option` to keep indices aligned |
//! | `ByteArrayHelper.getInt/getShort` throws on out-of-bounds | [`checked_int`] / [`checked_short`] return `None` (skip / treat as 0), and `MPPUtility.getByteArray` does not copy when out of bounds | Follows the "never panic" convention |
//! | `UUID.nameUUIDFromBytes(byte[])` | This file's [`name_uuid_from_bytes`] (self-implemented MD5 + version/variant bits) | Java's method generates an MD5-based "version 3" UUID; the `uuid` crate's `v3` feature is not enabled and this crate does not allow new dependencies, so RFC 1321 MD5 is implemented in place |
//! | `m_container.getOrCreate(null)` (when `field` is null) | Skip that field | Java's `FieldTypeHelper` returns a placeholder object / null when it does not recognize a field number; Rust's `FieldKey` has no "unknown" variant (see the notes on `field_type_helper`) and cannot serve as a container key |
//! | `IOException` / `NullPointerException` (missing OLE entry, data out of bounds) | Return `Ok(())` / skip the item | Follows the "never panic" convention, annotated point by point |
//!
//! Skipped Java methods: none (`process` / `processCustomFieldValues` / `processOutlineCodeValues` /
//! both `populateContainer` overloads / `convertType` / `getDataType` are all translated; because Rust does not support overloading,
//! the second `populateContainer` is renamed to `populate_container_from_outline_code`).
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::mpp_field_type::{MppFieldKey, MppFieldType};
use crate::custom_field_container::CustomFieldContainer;
use crate::custom_field_value_data_type::CustomFieldValueDataType;
use crate::data_type::DataType;
use crate::field_type::FieldType;
use crate::field_type_class::FieldTypeClass;
use crate::field_value::FieldValue;
use crate::mpp::custom_field_value_item::CustomFieldValueItem;
use crate::mpp::custom_field_value_reader::{checked_int, checked_short};
use crate::mpp::fixed_data::FixedData;
use crate::mpp::fixed_meta::FixedMeta;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::ole::DirectoryEntry;
use crate::mpp::props::Props;
use crate::mpp::props_key::PropsKey;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_meta::VarMeta;
use crate::mpp::var_meta9::VarMeta9;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;
use crate::project_properties::ProjectProperties;

/// Data type key of the outline code "value" in the variable data block.
///
/// Corresponds to Java: `private static final Integer VALUE = Integer.valueOf(1);`
const VALUE: i32 = 1;

/// Data type key of the outline code "description" in the variable data block.
///
/// Corresponds to Java: `private static final Integer DESCRIPTION = Integer.valueOf(2);`
const DESCRIPTION: i32 = 2;

/// MPP9 value list reader.
///
/// Corresponds to Java: `class CustomFieldValueReader9`
pub struct CustomFieldValueReader9 {
    /// Project store (the parent store of `TBkndOutlCode`).
    ///
    /// Corresponds to Java field: `private final DirectoryEntry m_projectDir;`
    project_dir: DirectoryEntry,

    /// Project file.
    ///
    /// Corresponds to Java field: `private final ProjectFile m_file;`
    file: Rc<RefCell<ProjectFile>>,

    /// The `TASK_FIELD_ATTRIBUTES` data block in `Props`.
    ///
    /// Corresponds to Java: `m_projectProps.getByteArray(PropsKey.TASK_FIELD_ATTRIBUTES)`
    task_field_attributes: Option<Vec<u8>>,

    /// Custom field container.
    ///
    /// Corresponds to Java field: `private final CustomFieldContainer m_container;`
    container: Rc<RefCell<CustomFieldContainer>>,

    /// Value item unique ID counter.
    ///
    /// Corresponds to Java field: `private int m_valueUniqueIDCounter;`
    value_unique_id_counter: i32,
}

impl CustomFieldValueReader9 {
    /// Constructor.
    ///
    /// Corresponds to Java: `public CustomFieldValueReader9(DirectoryEntry projectDir, ProjectFile file, Props projectProps)`
    pub fn new(
        project_dir: &DirectoryEntry,
        file: &Rc<RefCell<ProjectFile>>,
        project_props: &Props,
    ) -> Self {
        // Java: m_properties = file.getProjectProperties(); m_container = file.getCustomFields();
        // When the file is already mutably borrowed (which does not happen on the normal call path), degrade to a standalone container to avoid a panic.
        let container = match file.try_borrow() {
            Ok(file) => file.get_custom_fields(),
            Err(_) => CustomFieldContainer::new(),
        };

        Self {
            project_dir: project_dir.clone(),
            file: Rc::clone(file),
            // Java takes this block only in `processCustomFieldValues`; the Rust side takes it at construction time (see the file header)
            task_field_attributes: project_props
                .get_byte_array(PropsKey::TaskFieldAttributes)
                .map(<[u8]>::to_vec),
            container,
            value_unique_id_counter: 0,
        }
    }

    /// Reads all value lists.
    ///
    /// Corresponds to Java: `public void process() throws IOException`
    pub fn process(&mut self) -> MpxjResult<()> {
        // Java: processCustomFieldValues(); processOutlineCodeValues();
        self.process_custom_field_values();
        self.process_outline_code_values()
    }

    /// Reads value lists for custom fields that are not outline codes.
    ///
    /// Corresponds to Java: `private void processCustomFieldValues()`
    fn process_custom_field_values(&mut self) {
        // Java: byte[] data = m_projectProps.getByteArray(PropsKey.TASK_FIELD_ATTRIBUTES);
        //       if (data != null) { ... }
        // The Rust side copies the data locally: `populate_container` needs `&mut self` (the unique ID counter),
        // so borrowing `self.task_field_attributes` directly would conflict with the mutable borrow.
        let Some(data) = self.task_field_attributes.clone() else {
            return;
        };

        // Java: int index = 0; int offset = 0;
        let mut index = 0i32;
        let mut offset = 0usize;

        // Java: int length = ByteArrayHelper.getInt(data, offset); offset += 4;
        // Java throws when out of bounds; the Rust side returns directly (no value list to read)
        let Some(length) = checked_int(&data, offset) else {
            return;
        };
        offset += 4;

        // Java: int numberOfValueLists = ByteArrayHelper.getInt(data, offset); offset += 4;
        let Some(number_of_value_lists) = checked_int(&data, offset) else {
            return;
        };
        offset += 4;

        // Java: while (index < numberOfValueLists && offset < length)
        while index < number_of_value_lists && (offset as i64) < i64::from(length) {
            // Java: field = FieldTypeHelper.getInstance(m_file, ByteArrayHelper.getInt(data, offset));
            let Some(field_id) = checked_int(&data, offset) else {
                break;
            };
            offset += 4;
            let field = FieldTypeHelper::get_instance(&self.file, field_id);

            // Java: valueListOffset = ByteArrayHelper.getInt(data, offset); offset += 4;
            let Some(value_list_offset) = checked_int(&data, offset) else {
                break;
            };
            offset += 4;

            // Java: if (valueListOffset < data.length) { ... }
            // (Java throws for a negative valueListOffset due to the negative index; the Rust side skips that entry)
            if let Ok(value_list_offset) = usize::try_from(value_list_offset) {
                if value_list_offset < data.len() {
                    // Java: int tempOffset = valueListOffset; tempOffset += 8;
                    let base = value_list_offset as i64;
                    let mut temp_offset = value_list_offset.saturating_add(8);

                    // Java: int dataOffset = ByteArrayHelper.getInt(data, tempOffset) + valueListOffset; tempOffset += 4;
                    let data_offset = checked_int(&data, temp_offset).map(|value| i64::from(value) + base);
                    temp_offset = temp_offset.saturating_add(4);

                    // Java: int endDataOffset = ByteArrayHelper.getInt(data, tempOffset) + valueListOffset; tempOffset += 4;
                    let end_data_offset = checked_int(&data, temp_offset).map(|value| i64::from(value) + base);
                    temp_offset = temp_offset.saturating_add(4);

                    // Java: int endDescriptionOffset = ByteArrayHelper.getInt(data, tempOffset) + valueListOffset;
                    let end_description_offset =
                        checked_int(&data, temp_offset).map(|value| i64::from(value) + base);

                    // Java throws when any of the three offsets cannot be read; the Rust side skips that entry
                    if let (Some(data_offset), Some(end_data_offset), Some(end_description_offset)) =
                        (data_offset, end_data_offset, end_description_offset)
                    {
                        // Java: int valuesLength = endDataOffset - dataOffset;
                        //       byte[] values = new byte[valuesLength];
                        //       MPPUtility.getByteArray(data, dataOffset, valuesLength, values, 0);
                        let values = read_block(&data, data_offset, end_data_offset - data_offset);

                        // Java: int descriptionsLength = endDescriptionOffset - endDataOffset;
                        //       byte[] descriptions = new byte[descriptionsLength];
                        //       MPPUtility.getByteArray(data, endDataOffset, descriptionsLength, descriptions, 0);
                        let descriptions = read_block(
                            &data,
                            end_data_offset,
                            end_description_offset - end_data_offset,
                        );

                        // Java: populateContainer(field, values, descriptions);
                        self.populate_container(field.as_ref(), &values, &descriptions);
                    }
                }
            }

            // Java: index++;
            index += 1;
        }
    }

    /// Reads the outline code value lists.
    ///
    /// Corresponds to Java: `private void processOutlineCodeValues() throws IOException`
    fn process_outline_code_values(&mut self) -> MpxjResult<()> {
        // Java: DirectoryEntry outlineCodeDir = (DirectoryEntry) m_projectDir.getEntry("TBkndOutlCode");
        // POI throws when the entry is not found; the Rust side returns Ok(()) (no outline codes to read)
        let Some(outline_code_dir) = self.project_dir.get_directory("TBkndOutlCode") else {
            return Ok(());
        };

        // Java: FixedMeta fm = new FixedMeta(new DocumentInputStream(((DocumentEntry) outlineCodeDir.getEntry("FixedMeta"))), 10);
        let Some(fixed_meta_document) = outline_code_dir.get_document("FixedMeta") else {
            return Ok(());
        };
        let fixed_meta = FixedMeta::with_item_size(&fixed_meta_document.get_data(), 10)?;

        // Java: FixedData fd = new FixedData(fm, new DocumentInputStream(((DocumentEntry) outlineCodeDir.getEntry("FixedData"))));
        let Some(fixed_data_document) = outline_code_dir.get_document("FixedData") else {
            return Ok(());
        };
        let fixed_data = FixedData::new(&fixed_meta, &fixed_data_document.get_data());

        // Java: Map<Integer, FieldType> map = new HashMap<>();
        let mut map: HashMap<i32, MppFieldType> = HashMap::new();

        // Java: int items = fm.getItemCount(); for (int loop = 0; loop < items; loop++) { ... }
        for loop_index in 0..fixed_meta.get_item_count() {
            // Java: byte[] data = fd.getByteArrayValue(loop); if (data.length < 18) { continue; }
            // (Java throws an NPE for null; the Rust side skips it)
            let Some(data) = fixed_data.get_byte_array_value(loop_index) else {
                continue;
            };
            if data.len() < 18 {
                continue;
            }

            // Java: int fieldID = ByteArrayHelper.getInt(data, 12);
            let field_id = ByteArrayHelper::get_int(data, 12);

            // Java: FieldType fieldType = FieldTypeHelper.getInstance(m_file, fieldID);
            //       if (fieldType != null && fieldType.getFieldTypeClass() != FieldTypeClass.UNKNOWN)
            if let Some(field_type) = FieldTypeHelper::get_instance(&self.file, field_id) {
                if field_type.get_field_type_class() != FieldTypeClass::Unknown {
                    // Java: int index = ByteArrayHelper.getShort(data, 0);
                    //       map.put(Integer.valueOf(index), fieldType);
                    map.insert(ByteArrayHelper::get_short(data, 0), field_type);
                }
            }
        }

        // Java: VarMeta outlineCodeVarMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) outlineCodeDir.getEntry("VarMeta"))));
        let Some(var_meta_document) = outline_code_dir.get_document("VarMeta") else {
            return Ok(());
        };
        let outline_code_var_meta: Rc<dyn VarMeta> =
            Rc::new(VarMeta9::new(&var_meta_document.get_data())?);

        // Java: Var2Data outlineCodeVarData = new Var2Data(m_file, outlineCodeVarMeta, new DocumentInputStream(((DocumentEntry) outlineCodeDir.getEntry("Var2Data"))));
        let Some(var2_data_document) = outline_code_dir.get_document("Var2Data") else {
            return Ok(());
        };
        let outline_code_var_data = Var2Data::new(
            &self.file,
            Rc::clone(&outline_code_var_meta),
            &var2_data_document.get_data(),
        );

        // Java: Map<FieldType, List<Pair<String, String>>> valueMap = new HashMap<>();
        // Rust side: Java allows a null key (the case where `map.get(id)` misses), so the key uses `Option<MppFieldKey>`;
        // the field itself is also stored in the value for `populate_container_from_outline_code` to take the name.
        let mut value_map: HashMap<
            Option<MppFieldKey>,
            (Option<MppFieldType>, Vec<(Option<String>, Option<String>)>),
        > = HashMap::new();

        // Java: for (Integer id : outlineCodeVarMeta.getUniqueIdentifierArray())
        for id in outline_code_var_meta.get_unique_identifier_array() {
            // Java: FieldType fieldType = map.get(id);
            let field_type = map.get(&id).cloned();

            // Java: String value = outlineCodeVarData.getUnicodeString(id, VALUE);
            //       String description = outlineCodeVarData.getUnicodeString(id, DESCRIPTION);
            let value = outline_code_var_data.get_unicode_string(Some(id), Some(VALUE));
            let description = outline_code_var_data.get_unicode_string(Some(id), Some(DESCRIPTION));

            // Java: List<Pair<String, String>> list = valueMap.computeIfAbsent(fieldType, k -> new ArrayList<>());
            //       list.add(new Pair<>(value, description));
            let key = field_type.as_ref().map(MppFieldType::to_key);
            let entry = value_map
                .entry(key)
                .or_insert_with(|| (field_type, Vec::new()));
            entry.1.push((value, description));
        }

        // Java: for (Entry<FieldType, List<Pair<String, String>>> entry : valueMap.entrySet()) { populateContainer(entry.getKey(), entry.getValue()); }
        for (_, (field_type, items)) in value_map {
            self.populate_container_from_outline_code(field_type.as_ref(), &items);
        }

        Ok(())
    }

    /// Populates the container from a "value array + description array".
    ///
    /// Corresponds to Java: `private void populateContainer(FieldType field, byte[] values, byte[] descriptions)`
    fn populate_container(
        &mut self,
        field: Option<&MppFieldType>,
        values: &[u8],
        descriptions: &[u8],
    ) {
        // Java: CustomField config = m_container.getOrCreate(field);
        // Java's `field` may be null (field number not recognized): `HashMap` allows a null key, but the immediately following
        // `config.getFieldType().getName()` throws an NPE. Rust's `FieldKey` has no "unknown" variant, so it is skipped.
        let Some(field) = field else {
            return;
        };

        let custom_field = CustomFieldContainer::get_or_create(&self.container, field.to_field_key());

        // Java: String fieldTypeName = config.getFieldType().getName();
        let field_type_name = custom_field.borrow().get_field_type().get_name().to_owned();

        // Java: table.setGUID(UUID.nameUUIDFromBytes(fieldTypeName.getBytes()));
        // (Java takes bytes using the platform default encoding; field names are all ASCII, so the Rust side's UTF-8 gives the same result)
        custom_field
            .borrow_mut()
            .get_lookup_table_mut()
            .set_guid(Some(name_uuid_from_bytes(field_type_name.as_bytes())));

        // Java: List<Object> descriptionList = convertType(DataType.STRING, descriptions);
        let description_list = self.convert_type(DataType::String, descriptions);
        // Java: List<Object> valueList = convertType(field.getDataType(), values);
        let value_list = self.convert_type(field.get_data_type(), values);
        // Java: CustomFieldValueDataType itemType = getDataType(field);
        let item_type = Self::get_data_type(Some(field));

        // Java: for (int index = 0; index < descriptionList.size(); index++)
        for index in 0..description_list.len() {
            // Java: CustomFieldValueItem item = new CustomFieldValueItem(Integer.valueOf(++m_valueUniqueIDCounter));
            self.value_unique_id_counter += 1;
            let item = CustomFieldValueItem::new(self.value_unique_id_counter);

            // Java: item.setDescription((String) descriptionList.get(index));
            item.set_description(
                description_list[index]
                    .as_ref()
                    .and_then(|value| value.as_string().map(str::to_owned)),
            );
            // Java: item.setType(itemType);
            item.set_type(Some(item_type));
            // Java: item.setGUID(UUID.nameUUIDFromBytes((fieldTypeName + item.getUniqueID()).getBytes()));
            item.set_guid(Some(name_uuid_from_bytes(
                format!("{field_type_name}{}", item.get_unique_id()).as_bytes(),
            )));
            // Java: if (index < valueList.size()) { item.setValue(valueList.get(index)); }
            if index < value_list.len() {
                item.set_value(value_list[index].clone());
            }

            // Java: m_container.registerValue(item);
            self.container
                .borrow_mut()
                .register_value(item.stored_item());
            // Java: table.add(item);
            custom_field
                .borrow_mut()
                .get_lookup_table_mut()
                .add(item.to_stored_item());
        }
    }

    /// Populates the container from outline code data.
    ///
    /// Corresponds to Java: `private void populateContainer(FieldType field, List<Pair<String, String>> items)`
    ///
    /// Rust has no overloading, so this method adds a `_from_outline_code` suffix after `populate_container`.
    /// Unlike `populate_container`: Java here only adds the value items to the lookup table and does **not** register them in the container.
    fn populate_container_from_outline_code(
        &mut self,
        field: Option<&MppFieldType>,
        items: &[(Option<String>, Option<String>)],
    ) {
        // Java: CustomField config = m_container.getOrCreate(field);
        // When `field` is null, Java uses "Unknown" as the field name but still needs `getOrCreate(null)`;
        // Rust's `FieldKey` cannot carry null, so it is skipped (the same trade-off as `populate_container`).
        let Some(field) = field else {
            return;
        };

        let custom_field = CustomFieldContainer::get_or_create(&self.container, field.to_field_key());

        // Java: String fieldTypeName = field == null ? "Unknown" : field.getName();
        // (The branch where `field` is None already returned above, so the field name is always taken here)
        let field_type_name = field.get_name();

        // Java: table.setGUID(UUID.nameUUIDFromBytes(fieldTypeName.getBytes()));
        custom_field
            .borrow_mut()
            .get_lookup_table_mut()
            .set_guid(Some(name_uuid_from_bytes(field_type_name.as_bytes())));

        // Java: CustomFieldValueDataType itemType = getDataType(field);
        let item_type = Self::get_data_type(Some(field));

        // Java: for (Pair<String, String> pair : items)
        for (value, description) in items {
            // Java: CustomFieldValueItem item = new CustomFieldValueItem(Integer.valueOf(++m_valueUniqueIDCounter));
            self.value_unique_id_counter += 1;
            let item = CustomFieldValueItem::new(self.value_unique_id_counter);

            // Java: item.setValue(pair.getFirst());
            item.set_value(value.clone().map(FieldValue::String));
            // Java: item.setDescription(pair.getSecond());
            item.set_description(description.clone());
            // Java: item.setType(itemType);
            item.set_type(Some(item_type));
            // Java: item.setGUID(UUID.nameUUIDFromBytes((fieldTypeName + item.getUniqueID()).getBytes()));
            item.set_guid(Some(name_uuid_from_bytes(
                format!("{field_type_name}{}", item.get_unique_id()).as_bytes(),
            )));

            // Java: table.add(item);
            custom_field
                .borrow_mut()
                .get_lookup_table_mut()
                .add(item.to_stored_item());
        }
    }

    /// Splits a raw data segment into value lists by data type.
    ///
    /// Corresponds to Java: `private List<Object> convertType(DataType type, byte[] data)`
    ///
    /// `Vec<Option<FieldValue>>` corresponds to Java's `List<Object>`: elements may be `null`
    /// (when `getTimestamp` / `getAdjustedDuration` return null), using `Option` to keep indices aligned.
    fn convert_type(&self, type_: DataType, data: &[u8]) -> Vec<Option<FieldValue>> {
        // Java: List<Object> result = new ArrayList<>(); int index = 0;
        let mut result = Vec::new();
        let mut index = 0usize;

        // Java: while (index < data.length)
        while index < data.len() {
            match type_ {
                // Java: case STRING: String value = MPPUtility.getUnicodeString(data, index);
                //       result.add(value); index += ((value.length() + 1) * 2);
                DataType::String => {
                    let value = MPPUtility::get_unicode_string(data, index);
                    // Java's `String.length()` is the number of UTF-16 code units
                    let units = value.encode_utf16().count() + 1;
                    result.push(Some(FieldValue::String(value)));
                    index = index.saturating_add(units * 2);
                }

                // Java: case CURRENCY: Double value = Double.valueOf(MPPUtility.getDouble(data, index) / 100);
                DataType::Currency => {
                    result.push(Some(FieldValue::Double(
                        MPPUtility::get_double(data, index) / 100.0,
                    )));
                    index = index.saturating_add(8);
                }

                // Java: case NUMERIC: Double value = Double.valueOf(MPPUtility.getDouble(data, index));
                DataType::Numeric => {
                    result.push(Some(FieldValue::Double(MPPUtility::get_double(data, index))));
                    index = index.saturating_add(8);
                }

                // Java: case DATE: LocalDateTime value = MPPUtility.getTimestamp(data, index);
                //       result.add(value); index += 4;
                DataType::Date => {
                    result.push(MPPUtility::get_timestamp(data, index).map(FieldValue::DateTime));
                    index = index.saturating_add(4);
                }

                // Java:
                // case DURATION:
                //    TimeUnit units = MPPUtility.getDurationTimeUnits(ByteArrayHelper.getShort(data, index + 4), m_properties.getDefaultDurationUnits());
                //    Duration value = MPPUtility.getAdjustedDuration(m_properties, ByteArrayHelper.getInt(data, index), units);
                //    result.add(value); index += 6;
                DataType::Duration => {
                    let properties = self.try_project_properties();
                    let default_duration_units = properties
                        .as_deref()
                        .map(ProjectProperties::get_default_duration_units);
                    // Java throws when out of bounds; the Rust side treats it as 0 when unavailable (`getDurationTimeUnits(0)` = DAYS)
                    let units = MPPUtility::get_duration_time_units_with_default(
                        checked_short(data, index.saturating_add(4)).unwrap_or(0),
                        default_duration_units,
                    );
                    let duration = checked_int(data, index).unwrap_or(0);

                    result.push(
                        properties
                            .as_deref()
                            .and_then(|properties| {
                                MPPUtility::get_adjusted_duration(properties, duration, units)
                            })
                            .map(FieldValue::Duration),
                    );
                    index = index.saturating_add(6);
                }

                // Java: case BOOLEAN: Boolean value = Boolean.valueOf(ByteArrayHelper.getShort(data, index) == 1);
                //       result.add(value); index += 2;
                DataType::Boolean => {
                    result.push(Some(FieldValue::Boolean(
                        checked_short(data, index).unwrap_or(0) == 1,
                    )));
                    index = index.saturating_add(2);
                }

                // Java: default: index = data.length; (end parsing)
                _ => index = data.len(),
            }
        }

        result
    }

    /// Returns the value item type corresponding to a custom field.
    ///
    /// Corresponds to Java: `private CustomFieldValueDataType getDataType(FieldType field)`
    ///
    /// Java uses a static `Map<DataType, CustomFieldValueDataType> TYPE_MAP` + "TEXT when not found";
    /// the Rust side expresses this equivalently with `match` (the mapping table is constant, so no runtime table construction is needed).
    /// Java's instance method only uses the static table, so the Rust side implements it as an associated function.
    fn get_data_type(field: Option<&MppFieldType>) -> CustomFieldValueDataType {
        // Java: if (field != null) { result = TYPE_MAP.get(field.getDataType()); }
        //       if (result == null) { result = CustomFieldValueDataType.TEXT; }
        match field.map(MppFieldType::get_data_type) {
            Some(DataType::String) => CustomFieldValueDataType::Text,
            Some(DataType::Currency) => CustomFieldValueDataType::Cost,
            Some(DataType::Numeric) => CustomFieldValueDataType::Number,
            Some(DataType::Date) => CustomFieldValueDataType::Date,
            Some(DataType::Duration) => CustomFieldValueDataType::Duration,
            Some(DataType::Boolean) => CustomFieldValueDataType::Flag,
            _ => CustomFieldValueDataType::Text,
        }
    }

    /// Borrows the project properties (Rust-specific: Java stores a reference to the properties object in the constructor).
    fn try_project_properties(&self) -> Option<Ref<'_, ProjectProperties>> {
        let file = self.file.try_borrow().ok()?;
        Some(Ref::map(file, |file| file.get_project_properties()))
    }
}

/// Copies a data segment with Java semantics.
///
/// Corresponds to Java: `new byte[size]` + `MPPUtility.getByteArray(data, offset, size, buffer, 0)`.
/// Java throws `NegativeArraySizeException` when `size` is negative and throws
/// `ArrayIndexOutOfBoundsException` when the offset is out of bounds; the Rust side uniformly degrades to "copy as much as possible, with 0 for the out-of-bounds part".
fn read_block(data: &[u8], offset: i64, size: i64) -> Vec<u8> {
    // Java: new byte[valuesLength] (negative length throws)
    let size = usize::try_from(size).unwrap_or(0);
    let mut result = vec![0u8; size];

    // Java: MPPUtility.getByteArray(data, dataOffset, valuesLength, values, 0);
    // (A negative offset throws an out-of-bounds exception in Java; the Rust side degrades to 0; `get_byte_array` still performs bounds checks internally)
    let offset = usize::try_from(offset.max(0)).unwrap_or(0);
    MPPUtility::get_byte_array(data, offset, size, &mut result, 0);

    result
}

/// Generates a "name-based" UUID from arbitrary bytes.
///
/// Corresponds to Java: `UUID.nameUUIDFromBytes(byte[] name)`
///
/// `java.util.UUID.nameUUIDFromBytes` takes the 16 bytes of `MD5(name)` and rewrites the version bits
/// (`0x30`) and variant bits (`0x80`) per RFC 4122 before constructing the UUID. The `uuid` crate's `v3` feature is not enabled (this crate
/// does not allow new dependencies), so MD5 is implemented in place here.
fn name_uuid_from_bytes(name: &[u8]) -> Uuid {
    let mut digest = md5(name);

    // Java: md5Bytes[6] &= 0x0f; md5Bytes[6] |= 0x30;
    //       md5Bytes[8] &= 0x3f; md5Bytes[8] |= 0x80;
    digest[6] = (digest[6] & 0x0F) | 0x30;
    digest[8] = (digest[8] & 0x3F) | 0x80;

    Uuid::from_bytes(digest)
}

/// Left-shift amounts for each MD5 round.
///
/// Corresponds to RFC 1321's `s` table.
const MD5_S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

/// Additive constants for each MD5 round (`floor(2^32 * abs(sin(i + 1)))`).
///
/// Corresponds to RFC 1321's `K` table.
const MD5_K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

/// Computes the MD5 digest (RFC 1321).
///
/// Rust-specific: on the Java side this capability is provided by `java.security.MessageDigest`.
fn md5(input: &[u8]) -> [u8; 16] {
    // Padding: append 0x80, zero-pad to 56 mod 64, then append the 8-byte little-endian bit length
    let mut message = input.to_vec();
    let bit_length = (input.len() as u64).wrapping_mul(8);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_le_bytes());

    let mut a0: u32 = 0x6745_2301;
    let mut b0: u32 = 0xefcd_ab89;
    let mut c0: u32 = 0x98ba_dcfe;
    let mut d0: u32 = 0x1032_5476;

    for chunk in message.chunks_exact(64) {
        let mut words = [0u32; 16];
        for (index, word) in words.iter_mut().enumerate() {
            let start = index * 4;
            *word = u32::from_le_bytes([
                chunk[start],
                chunk[start + 1],
                chunk[start + 2],
                chunk[start + 3],
            ]);
        }

        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for index in 0..64 {
            let (f, g) = match index / 16 {
                0 => ((b & c) | (!b & d), index),
                1 => ((d & b) | (!d & c), (5 * index + 1) % 16),
                2 => (b ^ c ^ d, (3 * index + 5) % 16),
                _ => (c ^ (b | !d), (7 * index) % 16),
            };

            let temp = d;
            d = c;
            c = b;
            b = b.wrapping_add(
                a.wrapping_add(f)
                    .wrapping_add(MD5_K[index])
                    .wrapping_add(words[g])
                    .rotate_left(MD5_S[index]),
            );
            a = temp;
        }

        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }

    let mut digest = [0u8; 16];
    digest[0..4].copy_from_slice(&a0.to_le_bytes());
    digest[4..8].copy_from_slice(&b0.to_le_bytes());
    digest[8..12].copy_from_slice(&c0.to_le_bytes());
    digest[12..16].copy_from_slice(&d0.to_le_bytes());
    digest
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use cfb::CompoundFile;

    use super::*;
    use crate::common::mpp_task_field::MPPTaskField;
    use crate::field_key::FieldKey;
    use crate::mpp::custom_field_value_reader::test_support::{data_block, var_meta9_bytes};
    use crate::mpp::ole::PoifsFileSystem;
    use crate::task_field::TaskField;

    /// Field number of the built-in task field NAME.
    fn name_field_id() -> i32 {
        MPPTaskField::TASK_FIELD_BASE | 14
    }

    /// Built-in task field NAME.
    fn name_field() -> MppFieldType {
        MppFieldType::Builtin(FieldKey::Task(TaskField::Name))
    }

    /// UTF-16LE (NUL-terminated) bytes.
    fn utf16(text: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        bytes.extend_from_slice(&[0x00, 0x00]);
        bytes
    }

    /// Builds a compound document that writes the outline code streams into the `TBkndOutlCode` store.
    ///
    /// `outline_code_records` are the records of the fixed data block (20 bytes each: [0..2) = index,
    /// [12..16) = field number); `var_data` is the content of the variable data block.
    fn compound_file(outline_code_records: &[Vec<u8>], var_data: &[u8], var_meta: &[u8]) -> Vec<u8> {
        // FixedMeta: 10-byte records, with offsets taken from the position of each record in the data body
        let mut fixed_meta = Vec::new();
        fixed_meta.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        fixed_meta.extend_from_slice(&0i32.to_le_bytes());
        fixed_meta.extend_from_slice(&(outline_code_records.len() as i32).to_le_bytes());
        fixed_meta.extend_from_slice(&0i32.to_le_bytes());

        let mut body = Vec::new();
        for record in outline_code_records {
            let mut meta_record = vec![0u8; 10];
            meta_record[4..8].copy_from_slice(&(body.len() as i32).to_le_bytes());
            fixed_meta.extend_from_slice(&meta_record);
            body.extend_from_slice(record);
        }

        let mut file = CompoundFile::create(Cursor::new(Vec::new())).expect("should be able to create compound document");
        file.create_storage("/TBkndOutlCode").expect("create storage");
        file.create_stream("/TBkndOutlCode/FixedMeta")
            .expect("create stream")
            .write_all(&fixed_meta)
            .expect("write FixedMeta");
        file.create_stream("/TBkndOutlCode/FixedData")
            .expect("create stream")
            .write_all(&body)
            .expect("write FixedData");
        file.create_stream("/TBkndOutlCode/VarMeta")
            .expect("create stream")
            .write_all(var_meta)
            .expect("write VarMeta");
        file.create_stream("/TBkndOutlCode/Var2Data")
            .expect("create stream")
            .write_all(var_data)
            .expect("write Var2Data");

        file.into_inner().into_inner()
    }

    /// Opens the compound document and gets the root store.
    ///
    /// When an empty slice is passed, uses a compound document containing no streams instead (corresponding to "there is no `TBkndOutlCode` in the project store").
    fn root_directory(bytes: Vec<u8>) -> DirectoryEntry {
        let bytes = if bytes.is_empty() {
            empty_compound_file()
        } else {
            bytes
        };

        PoifsFileSystem::new(Cursor::new(bytes))
            .expect("compound document is parseable")
            .get_root()
    }

    /// Builds a compound document containing no streams (corresponding to "there is no `TBkndOutlCode` in the project store").
    fn empty_compound_file() -> Vec<u8> {
        CompoundFile::create(Cursor::new(Vec::new()))
            .expect("should be able to create compound document")
            .into_inner()
            .into_inner()
    }

    /// One record of the outline code fixed data block (20 bytes).
    fn outline_code_record(index: i32, field_id: i32) -> Vec<u8> {
        let mut record = vec![0u8; 20];
        record[0..2].copy_from_slice(&(index as u16).to_le_bytes());
        record[12..16].copy_from_slice(&field_id.to_le_bytes());
        record
    }

    /// MD5 matches the standard test vectors (RFC 1321 Appendix A.5).
    #[test]
    fn md5_matches_known_vectors() {
        assert_eq!(
            md5(b""),
            [
                0xd4, 0x1d, 0x8c, 0xd9, 0x8f, 0x00, 0xb2, 0x04, 0xe9, 0x80, 0x09, 0x98, 0xec, 0xf8,
                0x42, 0x7e
            ]
        );
        assert_eq!(
            md5(b"abc"),
            [
                0x90, 0x01, 0x50, 0x98, 0x3c, 0xd2, 0x4f, 0xb0, 0xd6, 0x96, 0x3f, 0x7d, 0x28, 0xe1,
                0x7f, 0x72
            ]
        );
    }

    /// `nameUUIDFromBytes` matches Java's "version 3" UUID.
    #[test]
    fn name_uuid_matches_java() {
        // Java: UUID.nameUUIDFromBytes("test".getBytes()) →
        //       098f6bcd-4621-3373-8ade-4e832627b4f6
        assert_eq!(
            name_uuid_from_bytes(b"test"),
            Uuid::parse_str("098f6bcd-4621-3373-8ade-4e832627b4f6").expect("valid GUID")
        );
        // Version bits are 3, variant bits are RFC 4122
        let bytes = name_uuid_from_bytes(b"test").into_bytes();
        assert_eq!(bytes[6] >> 4, 3);
        assert_eq!(bytes[8] >> 6, 0b10);
    }

    /// The value lists in `Props.TASK_FIELD_ATTRIBUTES` are read as value items (description / value / type / GUID).
    #[test]
    fn process_reads_task_field_attributes() {
        // Data block layout:
        //   [0..4)   total length
        //   [4..8)   number of value lists
        //   [8..12)  field number  [12..16) value list offset (absolute)
        //   [32..40) reserved  [40..44) data offset (relative)  [44..48) data end  [48..52) description end
        let mut block = vec![0u8; 66];
        block[0..4].copy_from_slice(&66i32.to_le_bytes());
        block[4..8].copy_from_slice(&1i32.to_le_bytes());
        block[8..12].copy_from_slice(&name_field_id().to_le_bytes());
        block[12..16].copy_from_slice(&32i32.to_le_bytes());
        // Value is "AB", description is "X"
        block[32 + 24..32 + 30].copy_from_slice(&utf16("AB"));
        block[32 + 30..32 + 34].copy_from_slice(&utf16("X"));
        block[32 + 8..32 + 12].copy_from_slice(&24i32.to_le_bytes());
        block[32 + 12..32 + 16].copy_from_slice(&30i32.to_le_bytes());
        block[32 + 16..32 + 20].copy_from_slice(&34i32.to_le_bytes());

        let mut props = Props::new();
        props.put(PropsKey::TaskFieldAttributes.get_value(), block);

        let file = ProjectFile::new();
        // There is no TBkndOutlCode under the root store: the outline code branch returns Ok(())
        let the_root = root_directory(Vec::new());
        let mut reader = CustomFieldValueReader9::new(&the_root, &file, &props);
        reader.process().expect("parsing should not fail");

        let container = file.borrow().get_custom_fields();
        let item = container
            .borrow()
            .get_custom_field_value_item_by_unique_id(1)
            .expect("value item should be registered");
        let item = item.borrow();
        assert_eq!(item.get_description(), Some("X"));
        assert_eq!(item.get_value(), Some(&FieldValue::String("AB".to_owned())));
        assert_eq!(item.get_type(), Some(CustomFieldValueDataType::Text));
        // GUID = nameUUIDFromBytes(field name + uniqueID)
        let field_name = name_field().get_name();
        assert_eq!(
            item.get_guid(),
            Some(name_uuid_from_bytes(
                format!("{field_name}1").as_bytes()
            ))
        );

        // Lookup table: GUID = nameUUIDFromBytes(field name)
        let custom_field = container
            .borrow()
            .get(FieldKey::Task(TaskField::Name))
            .expect("field configuration should be created");
        let custom_field = custom_field.borrow();
        assert_eq!(
            custom_field.get_lookup_table().get_guid(),
            Some(name_uuid_from_bytes(field_name.as_bytes()))
        );
        assert_eq!(custom_field.get_lookup_table().len(), 1);
    }

    /// Does not panic when `TASK_FIELD_ATTRIBUTES` is empty / truncated.
    #[test]
    fn process_handles_missing_task_field_attributes() {
        let file = ProjectFile::new();
        let the_root = root_directory(Vec::new());

        // The property is entirely absent
        let mut reader = CustomFieldValueReader9::new(&the_root, &file, &Props::new());
        reader.process().expect("parsing should not fail");
        assert!(file.borrow().get_custom_fields().borrow().is_empty());

        // Only 4 bytes (cannot read the number of value lists)
        let mut props = Props::new();
        props.put(PropsKey::TaskFieldAttributes.get_value(), vec![0u8; 4]);
        let mut reader = CustomFieldValueReader9::new(&the_root, &file, &props);
        reader.process().expect("parsing should not fail");

        // The field number points to an unrecognized value (FieldTypeHelper returns None)
        let mut block = vec![0u8; 20];
        block[0..4].copy_from_slice(&20i32.to_le_bytes());
        block[4..8].copy_from_slice(&1i32.to_le_bytes());
        block[8..12].copy_from_slice(&0x1234_0005i32.to_le_bytes());
        block[12..16].copy_from_slice(&16i32.to_le_bytes());
        let mut props = Props::new();
        props.put(PropsKey::TaskFieldAttributes.get_value(), block);
        let mut reader = CustomFieldValueReader9::new(&the_root, &file, &props);
        reader.process().expect("parsing should not fail");
    }

    /// Outline codes: the field number mapping + the values / descriptions in the variable data block are written into the lookup table.
    #[test]
    fn process_reads_outline_code_values() {
        // Variable data block: id=1 and id=2 each have a value (type 1) and a description (type 2)
        let items = vec![
            utf16("A"),
            utf16("desc A"),
            utf16("B"),
            utf16("desc B"),
        ];
        let (var_data, offsets) = data_block(&items);
        let var_meta = var_meta9_bytes(
            &[(1, VALUE, 0), (1, DESCRIPTION, 1), (2, VALUE, 2), (2, DESCRIPTION, 3)],
            &offsets,
            var_data.len() as i32,
        );

        // Three fixed records: the first two map to NAME, the third has an unrecognized field number
        let records = vec![
            outline_code_record(1, name_field_id()),
            outline_code_record(2, name_field_id()),
            outline_code_record(3, 0x1234_0005),
        ];
        let the_root = root_directory(compound_file(&records, &var_data, &var_meta));

        let file = ProjectFile::new();
        let mut reader = CustomFieldValueReader9::new(&the_root, &file, &Props::new());
        reader.process().expect("parsing should not fail");

        let container = file.borrow().get_custom_fields();
        let custom_field = container
            .borrow()
            .get(FieldKey::Task(TaskField::Name))
            .expect("field configuration should be created");
        let custom_field = custom_field.borrow();
        let table = custom_field.get_lookup_table();

        // The unrecognized field number (the third) produces no value item
        assert_eq!(table.len(), 2);
        assert_eq!(table.get(0).and_then(|item| item.get_value()), Some(&FieldValue::String("A".to_owned())));
        assert_eq!(table.get(0).and_then(|item| item.get_description()), Some("desc A"));
        assert_eq!(table.get(1).and_then(|item| item.get_value()), Some(&FieldValue::String("B".to_owned())));
        assert_eq!(table.get(1).and_then(|item| item.get_description()), Some("desc B"));
        // The unique ID is produced by incrementing the counter (Java's `++m_valueUniqueIDCounter`)
        assert_eq!(table.get(0).map(|item| item.get_unique_id()), Some(1));
        assert_eq!(table.get(1).map(|item| item.get_unique_id()), Some(2));
        // The value type comes from the field's DataType (STRING → TEXT)
        assert_eq!(table.get(0).and_then(|item| item.get_type()), Some(CustomFieldValueDataType::Text));
    }

    /// Does not panic when the outline code streams are missing / malformed.
    #[test]
    fn process_handles_broken_outline_code_streams() {
        let file = ProjectFile::new();

        // The store exists but none of the four streams exist
        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).expect("create compound document");
        compound.create_storage("/TBkndOutlCode").expect("create storage");
        let the_root = root_directory(compound.into_inner().into_inner());
        let mut reader = CustomFieldValueReader9::new(&the_root, &file, &Props::new());
        reader.process().expect("parsing should not fail");

        // FixedMeta content is invalid (wrong magic) → returns Err
        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).expect("create compound document");
        compound.create_storage("/TBkndOutlCode").expect("create storage");
        compound
            .create_stream("/TBkndOutlCode/FixedMeta")
            .expect("create stream")
            .write_all(&[0u8; 32])
            .expect("write");
        compound
            .create_stream("/TBkndOutlCode/FixedData")
            .expect("create stream")
            .write_all(&[0u8; 32])
            .expect("write");
        let the_root = root_directory(compound.into_inner().into_inner());
        let mut reader = CustomFieldValueReader9::new(&the_root, &file, &Props::new());
        assert!(reader.process().is_err());

        // FixedData is shorter than the metadata declares: skip when reading, do not panic
        let var_meta = var_meta9_bytes(&[], &[], 0);
        let the_root = root_directory(compound_file(&[outline_code_record(1, name_field_id())], &[], &var_meta));
        let mut reader = CustomFieldValueReader9::new(&the_root, &file, &Props::new());
        reader.process().expect("parsing should not fail");
    }

    /// Value type mapping (`getDataType`'s static mapping table + TEXT as the fallback).
    #[test]
    fn data_type_mapping_matches_java() {
        let cases = [
            (DataType::String, CustomFieldValueDataType::Text),
            (DataType::Currency, CustomFieldValueDataType::Cost),
            (DataType::Numeric, CustomFieldValueDataType::Number),
            (DataType::Date, CustomFieldValueDataType::Date),
            (DataType::Duration, CustomFieldValueDataType::Duration),
            (DataType::Boolean, CustomFieldValueDataType::Flag),
            // Type not in the table → TEXT
            (DataType::Custom, CustomFieldValueDataType::Text),
        ];

        for (data_type, expected) in cases {
            let key = FieldKey::Custom(crate::field_key::CustomFieldKey::new(
                1,
                FieldTypeClass::Task,
                data_type,
            ));
            let field = MppFieldType::Builtin(key);
            assert_eq!(
                CustomFieldValueReader9::get_data_type(Some(&field)),
                expected
            );
        }

        // Java: field == null → TEXT
        assert_eq!(
            CustomFieldValueReader9::get_data_type(None),
            CustomFieldValueDataType::Text
        );
    }

    /// The branches of `convertType` (`Vec<Option<FieldValue>>` keeps Java's index alignment).
    #[test]
    fn convert_type_matches_java() {
        let file = ProjectFile::new();
        let the_root = root_directory(Vec::new());
        let reader = CustomFieldValueReader9::new(&the_root, &file, &Props::new());

        // STRING: UTF-16LE + NUL, advance by (length + 1) * 2
        let values = reader.convert_type(DataType::String, &[0x41, 0x00, 0x42, 0x00, 0x00, 0x00, 0x43, 0x00, 0x00, 0x00]);
        assert_eq!(
            values,
            vec![
                Some(FieldValue::String("AB".to_owned())),
                Some(FieldValue::String("C".to_owned()))
            ]
        );

        // CURRENCY / NUMERIC: 8-byte double precision
        assert_eq!(
            reader.convert_type(DataType::Currency, &250.0f64.to_le_bytes()),
            vec![Some(FieldValue::Double(2.5))]
        );
        assert_eq!(
            reader.convert_type(DataType::Numeric, &1.5f64.to_le_bytes()),
            vec![Some(FieldValue::Double(1.5))]
        );

        // DATE: 4-byte timestamp (None when the day count is invalid, but the index still advances by one)
        assert_eq!(
            reader.convert_type(DataType::Date, &[0x00, 0x00, 0x02, 0x00]),
            vec![Some(FieldValue::DateTime(
                chrono::NaiveDate::from_ymd_opt(1984, 1, 2)
                    .expect("valid date")
                    .and_hms_opt(0, 0, 0)
                    .expect("valid time")
            ))]
        );

        // DURATION: 4-byte duration + 2-byte units
        let mut duration = 0i32.to_le_bytes().to_vec();
        duration.extend_from_slice(&7u16.to_le_bytes());
        let values = reader.convert_type(DataType::Duration, &duration);
        assert_eq!(values.len(), 1);
        assert!(matches!(values[0], Some(FieldValue::Duration(_))));

        // BOOLEAN: short integer == 1
        assert_eq!(
            reader.convert_type(DataType::Boolean, &[0x01, 0x00, 0x00, 0x00]),
            vec![
                Some(FieldValue::Boolean(true)),
                Some(FieldValue::Boolean(false))
            ]
        );

        // Unsupported type: end parsing immediately
        assert!(reader.convert_type(DataType::Guid, &[0u8; 16]).is_empty());

        // Empty data
        assert!(reader.convert_type(DataType::String, &[]).is_empty());
    }
}
