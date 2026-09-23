//! Corresponds to Java: `MSPDIReader.readExtendedAttributeDefinitions` /
//! `readOutlineCodeDefinitions` and their helpers (the custom field and outline code sections of
//! `org/mpxj/mspdi/MSPDIReader.java`, lines 836-875, 1595-1645 and 1815-1955).
//!
//! This module holds the `impl ReaderContext` block for the custom field section; it is split out of
//! [`crate::mspdi::mspdi_reader`] so that each section can be compared against its Java line range
//! (see the file header of that module).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `FieldType` (an interface reference; `field instanceof UserDefinedField` tests the custom field subclass) | [`FieldKey`] is the runtime erasure of a `FieldType` reference, and [`MppFieldType`] is kept where the Java code asks whether the field is a custom field (`MppFieldType::is_custom()`, which is Java's `instanceof UserDefinedField`) | `dyn FieldType` is not object safe; see [`crate::field_key`] and [`crate::common::mpp_field_type`]. `CustomFieldContainer` is keyed by `FieldKey`, so both `getOrCreate` call sites use `MppFieldType::to_field_key()` |
//! | `Map<FieldType, Map<BigInteger, CustomFieldValueItem>> m_customFieldValueItems` | `HashMap<MppFieldKey, HashMap<i64, CustomFieldValueItem>>` (`ReaderContext::custom_field_value_items`) | [`MppFieldKey`] is the hashable identity of a `FieldType` (a custom field uses its `UserDefinedField` unique ID); `BigInteger` is the schema layer's `i64`, and the stored value is the crate's *handle* for `org.mpxj.mpp.CustomFieldValueItem` (`crate::mpp::custom_field_value_item::CustomFieldValueItem`), which is also the type `reader_tasks` caches |
//! | `m_lookupTableMap.put(attribute.getLtuid(), field)` stores `null` keys and `null` values as well | the entry is inserted only when the `Ltuid` element and the field are both present | Rust `HashMap` keys cannot be `null` and [`MppFieldKey`] has no null state. Java's `Map.get` returns `null` for "key absent" and for "value is null" alike, so both lookups behave identically |
//! | `Integer.parseInt(attribute.getFieldID())` / `NumberHelper.getInt(outlineCode.getFieldID())` throw `NumberFormatException` for a malformed value, aborting the read | the definition is skipped (`str::parse().ok()` / [`NumberHelper::get_int_from_string`]) | Library code never panics; the rest of the document is still read (same choice as the sibling `reader_resources` section) |
//! | `m_projectFile.getCustomFields().getOrCreate(field).setAlias(alias)` with a `null` field (an `ExtendedAttribute` whose `FieldID` does not map to a field) | the alias is dropped | Java would build a `CustomField` with a null field type and then throw a `NullPointerException` in `CustomFieldContainer.registerAlias` (`type.getFieldTypeClass()`); Rust-specific branch, library code never panics |
//! | `NumberHelper.getInteger(BigInteger)` / `getInt(BigInteger)` | [`custom_fields_get_integer`] / [`custom_fields_get_int`] widen the schema's `i64` to `f64` before calling the same Rust helper | Java's methods take a `Number`; widening reproduces `BigInteger.doubleValue()`. Same convention as the `reader_resources` section (the `reader_tasks` section narrows with `as i32` instead, which only differs for values outside `i32`) |
//! | `new CustomFieldValueItem(uniqueID)` where `uniqueID` is `null` (a lookup table `<Value>` without a `<ValueID>`) | the value is skipped | `CustomFieldValueItem::new` takes an `i32`; Java's item would later make `getCustomFieldValueItemMap` throw a `NullPointerException` (`i.getUniqueID().intValue()`), so the item is unusable |
//! | `getValueItem(FieldType, BigInteger)` returns the live lookup table item, so the cache holds references to the items in the table | [`ReaderContext::get_value_item`] returns a handle whose storage is filled with a copy of the entry (unique ID, GUID, value, description, unknown block, parent unique ID, type, collapsed) | The Rust lookup table stores its elements by value, so a handle cannot share the stored item's storage; Java only reads the item through this path (`getOutlineCodeValue`), so a copy is enough |
//! | `Collectors.toMap` throws `IllegalStateException` for duplicate unique IDs | the later entry wins (`Iterator::collect`) | Duplicate value IDs are exactly the case `readOutlineCodeValues` renumbers below, so the difference only shows up for a document that repeats a unique ID within one lookup table |
//! | Java's `// TODO: use the code mask` in `getOutlineCodeValue` | reproduced as-is: the parent value is always joined with `"."` | Deliberate: the Rust port reproduces Java rather than improving it (see [`ReaderContext::get_outline_code_value`]) |
//! | `item.getValue().toString()` | [`custom_fields_field_value_to_string`] renders the value types [`DatatypeConverter::parse_outline_code_value`] produces | Java's `toString` of the remaining value types is not reproduced (yields an empty string). [`ReaderContext::tasks_get_outline_code_value`] in the sibling `reader_tasks` section renders the same types with the same rules |
//! | `ObjectSequence uniqueIdSequence = m_projectFile.getUniqueIdObjectSequence(CustomFieldValueItem.class)` (P6 compatibility for duplicate value IDs) | [`crate::common::object_sequence::ObjectSequence`] obtained with `TypeId::of::<CustomFieldValueItem>()` | Same sequence semantics (`sync` then `getNext`); the type id is taken from the `mpp` handle, which is this crate's carrier for the Java class named here |
//! | `Project.ExtendedAttributes` / `Project.OutlineCodes` wrappers and their `!= null` guards | the schema flattens both wrappers into `Vec`s, so the loops run unconditionally | An absent wrapper element yields an empty list, which is what Java's guard produces (see the schema headers) |
//!
//! # Skipped Java members + reason
//!
//! None. All nine methods of this section are ported.

use std::any::TypeId;
use std::collections::{HashMap, HashSet};

use chrono::Timelike;

use crate::common::boolean_helper::BooleanHelper;
use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::mpp_field_type::{MppFieldKey, MppFieldType};
use crate::common::number_helper::NumberHelper;
use crate::custom_field_container::CustomFieldContainer;
use crate::custom_field_value_data_type::CustomFieldValueDataType;
use crate::custom_field_value_mask::CustomFieldValueMask;
use crate::field_key::{CustomFieldKey, FieldKey};
use crate::field_type::FieldType;
use crate::field_value::FieldValue;
use crate::mpp::custom_field_value_item::CustomFieldValueItem;
use crate::mspdi::datatype_converter::DatatypeConverter;
use crate::mspdi::mspdi_reader::ReaderContext;
use crate::mspdi::schema::extended_attributes::ExtendedAttribute;
use crate::mspdi::schema::outline_codes::OutlineCode;
use crate::mspdi::schema::project::Project;

impl ReaderContext {
    /// This method extracts project extended attribute data from an MSPDI file.
    ///
    /// Corresponds to Java: `private void readExtendedAttributeDefinitions(Project project)`
    ///
    /// Java guards the body with `attributes != null`; the schema flattens the wrapper into a `Vec`,
    /// so an absent `<ExtendedAttributes>` element is seen as an empty list (see the file header).
    pub(crate) fn read_extended_attribute_definitions(&mut self, project: &Project) {
        for attribute in &project.extended_attributes {
            self.read_extended_attribute_definition(attribute);
        }
    }

    /// Read a single field alias from an extended attribute.
    ///
    /// Corresponds to Java: `private void readExtendedAttributeDefinition(Project.ExtendedAttributes.ExtendedAttribute attribute)`
    fn read_extended_attribute_definition(&mut self, attribute: &ExtendedAttribute) {
        // Java: FieldType field = FieldTypeHelper.getInstance(m_projectFile, Integer.parseInt(attribute.getFieldID()));
        // (an absent or unparsable FieldID yields None instead of throwing, see the file header)
        let field: Option<MppFieldType> = attribute
            .field_id
            .as_deref()
            .and_then(|value| value.parse::<i32>().ok())
            .and_then(|value| FieldTypeHelper::get_instance(&self.file, value));

        // Java: m_lookupTableMap.put(attribute.getLtuid(), field);
        // (Java also stores a null key / null value; the Rust map stores what can be represented,
        //  see the file header)
        if let (Some(ltuid), Some(field)) = (attribute.ltuid, field.as_ref()) {
            self.lookup_table_map.insert(ltuid, field.to_key());
        }

        // Java: String alias = attribute.getAlias();
        let mut alias = custom_fields_non_empty(attribute.alias.as_deref());

        // Java:
        // If we are dealing with an enterprise custom field (which will appear here as a
        // UserDefinedField instance) and we haven't been given an explicit alias, then fallback on
        // the field name.
        // if ((alias == null || alias.isEmpty()) && field instanceof UserDefinedField)
        // {
        //    alias = attribute.getFieldName();
        // }
        if alias.is_none() && field.as_ref().is_some_and(|field| field.is_custom()) {
            alias = custom_fields_non_empty(attribute.field_name.as_deref());
        }

        // Java: if (alias != null && !alias.isEmpty())
        //       {
        //          m_projectFile.getCustomFields().getOrCreate(field).setAlias(alias);
        //       }
        if let (Some(alias), Some(field)) = (alias, field) {
            let container = self.file.borrow().get_custom_fields();
            CustomFieldContainer::get_or_create(&container, field.to_field_key())
                .borrow_mut()
                .set_alias(Some(alias.to_owned()));
        }
    }

    /// This method extracts outline code/custom field data from an MSPDI file.
    ///
    /// Corresponds to Java: `private void readOutlineCodeDefinitions(Project project)`
    ///
    /// Java guards the body with `outlineCodes != null`; the schema flattens the wrapper into a
    /// `Vec`, so an absent `<OutlineCodes>` element is seen as an empty list (see the file header).
    pub(crate) fn read_outline_code_definitions(&mut self, project: &Project) {
        for outline_code in &project.outline_codes {
            self.read_outline_code_definition(outline_code);
        }
    }

    /// This method extracts the definition of a single outline code/custom lookup table from an
    /// MSPDI file.
    ///
    /// Corresponds to Java: `private void readOutlineCodeDefinition(Project.OutlineCodes.OutlineCode outlineCode)`
    fn read_outline_code_definition(&mut self, outline_code: &OutlineCode) {
        // Java:
        // FieldType fieldType;
        // String fieldID = outlineCode.getFieldID();
        // if (fieldID == null)
        // {
        //    fieldType = m_lookupTableMap.get(outlineCode.getGuid());
        // }
        // else
        // {
        //    fieldType = FieldTypeHelper.getInstance(m_projectFile, NumberHelper.getInt(outlineCode.getFieldID()));
        // }
        let field = match outline_code.field_id.as_deref() {
            None => outline_code
                .guid
                .and_then(|guid| self.lookup_table_map.get(&guid).copied())
                .and_then(|key| self.custom_fields_field_key_from_mpp_key(key)),
            Some(field_id) => FieldTypeHelper::get_instance(
                &self.file,
                NumberHelper::get_int_from_string(Some(field_id)),
            )
            .map(|field| field.to_field_key()),
        };

        // Java: if (fieldType != null) { ... }
        let Some(field) = field else {
            return;
        };

        // Java:
        // String newAlias = outlineCode.getAlias();
        // if (newAlias != null && !newAlias.isEmpty())
        // {
        //    CustomField field = m_projectFile.getCustomFields().get(fieldType);
        //    String currentAlias = field == null ? null : field.getAlias();
        //
        //    // Don't overwrite an alias we've read from extended attributes
        //    if (currentAlias == null || currentAlias.isEmpty())
        //    {
        //       if (field == null)
        //       {
        //          field = m_projectFile.getCustomFields().getOrCreate(fieldType);
        //       }
        //       field.setAlias(newAlias);
        //    }
        // }
        let new_alias = custom_fields_non_empty(outline_code.alias.as_deref());
        if let Some(new_alias) = new_alias {
            let container = self.file.borrow().get_custom_fields();
            let existing = container.borrow().get(field);
            let current_alias = existing
                .as_ref()
                .and_then(|custom_field| custom_field.borrow().get_alias().map(str::to_owned));

            if current_alias.is_none_or(|alias| alias.is_empty()) {
                let custom_field = match existing {
                    Some(custom_field) => custom_field,
                    None => CustomFieldContainer::get_or_create(&container, field),
                };
                custom_field
                    .borrow_mut()
                    .set_alias(Some(new_alias.to_owned()));
            }
        }

        // Java: readOutlineCodeValues(outlineCode, fieldType);
        self.read_outline_code_values(outline_code, field);

        // Java: readOutlineCodeMasks(outlineCode, fieldType);
        self.read_outline_code_masks(outline_code, field);
    }

    /// This method extracts the lookup table values for an outline code/custom field from an MSPDI
    /// file.
    ///
    /// Corresponds to Java: `private void readOutlineCodeValues(Project.OutlineCodes.OutlineCode outlineCode, FieldType fieldType)`
    ///
    /// Unlike the MPP readers this method does **not** register the value items in the
    /// `CustomFieldContainer` value index: Java only calls `table.add(item)` here.
    fn read_outline_code_values(&mut self, outline_code: &OutlineCode, field: FieldKey) {
        // Java: Project.OutlineCodes.OutlineCode.Values values = outlineCode.getValues(); if (values == null) { return; }
        let Some(values) = outline_code.values.as_ref() else {
            return;
        };

        // Java: ObjectSequence uniqueIdSequence = m_projectFile.getUniqueIdObjectSequence(CustomFieldValueItem.class);
        let unique_id_sequence = self
            .file
            .borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<CustomFieldValueItem>());

        // Java:
        // I have come across one example MSPDI file with duplicate unique IDs for custom field
        // values, possibly written by P6.
        // Ensure our ObjectSequence is in sync with the values from the file.
        // values.getValue().forEach(v -> uniqueIdSequence.sync(NumberHelper.getInteger(v.getValueID())));
        for value in &values.value {
            unique_id_sequence.sync(custom_fields_get_integer(value.value_id));
        }

        // Java: CustomField field = m_projectFile.getCustomFields().getOrCreate(fieldType);
        //       CustomFieldLookupTable table = field.getLookupTable();
        let container = self.file.borrow().get_custom_fields();
        let custom_field = CustomFieldContainer::get_or_create(&container, field);

        // Java: item.setValue(DatatypeConverter.parseOutlineCodeValue(value.getValue(), field.getFieldType().getDataType()));
        // (`CustomField.getFieldType()` returns the key the configuration was created with)
        let data_type = field.get_data_type();

        {
            let mut custom_field = custom_field.borrow_mut();
            let table = custom_field.get_lookup_table_mut();

            // Java: table.setEnterprise(BooleanHelper.getBoolean(outlineCode.isEnterprise()));
            table.set_enterprise(BooleanHelper::get_boolean(outline_code.enterprise));
            // Java: table.setAllLevelsRequired(BooleanHelper.getBoolean(outlineCode.isAllLevelsRequired()));
            table.set_all_levels_required(BooleanHelper::get_boolean(outline_code.all_levels_required));
            // Java: table.setGUID(outlineCode.getGuid());
            table.set_guid(outline_code.guid);
            // Java: table.setLeafOnly(BooleanHelper.getBoolean(outlineCode.isLeafOnly()));
            table.set_leaf_only(BooleanHelper::get_boolean(outline_code.leaf_only));
            // Java: table.setOnlyTableValuesAllowed(BooleanHelper.getBoolean(outlineCode.isOnlyTableValuesAllowed()));
            table.set_only_table_values_allowed(BooleanHelper::get_boolean(
                outline_code.only_table_values_allowed,
            ));
            // Java: table.setResourceSubstitutionEnabled(BooleanHelper.getBoolean(outlineCode.isResourceSubstitutionEnabled()));
            table.set_resource_substitution_enabled(BooleanHelper::get_boolean(
                outline_code.resource_substitution_enabled,
            ));
            // Java: table.setShowIndent(BooleanHelper.getBoolean(outlineCode.isShowIndent()));
            table.set_show_indent(BooleanHelper::get_boolean(outline_code.show_indent));
        }

        // Java: Set<Integer> uniqueIdValues = new HashSet<>();
        let mut unique_id_values: HashSet<i32> = HashSet::new();

        // Java: for (Project.OutlineCodes.OutlineCode.Values.Value value : values.getValue())
        for value in &values.value {
            // Java: Integer uniqueID = NumberHelper.getInteger(value.getValueID());
            let Some(mut unique_id) = custom_fields_get_integer(value.value_id) else {
                // Rust-specific: Java creates an item whose unique ID is null and stores it in the
                // lookup table; that item is not addressable and would make
                // `getCustomFieldValueItemMap` throw a NullPointerException (see the file header).
                continue;
            };

            // Java:
            // if (uniqueIdValues.contains(uniqueID))
            // {
            //    // Assign a new unique ID using the object sequence
            //    uniqueID = uniqueIdSequence.getNext();
            // }
            if unique_id_values.contains(&unique_id) {
                unique_id = unique_id_sequence.get_next();
            }
            unique_id_values.insert(unique_id);

            // Java: CustomFieldValueItem item = new CustomFieldValueItem(uniqueID);
            let item = CustomFieldValueItem::new(unique_id);
            // Java: item.setDescription(value.getDescription());
            item.set_description(value.description.clone());
            // Java: item.setGUID(value.getFieldGUID());
            item.set_guid(value.field_guid);
            // Java: item.setCollapsed(BooleanHelper.getBoolean(value.isIsCollapsed()));
            item.set_collapsed(BooleanHelper::get_boolean(value.is_collapsed));
            // Java: item.setParentUniqueID(NumberHelper.getInteger(value.getParentValueID()));
            item.set_parent_unique_id(custom_fields_get_integer(value.parent_value_id));
            // Java: item.setType(CustomFieldValueDataType.getInstance(NumberHelper.getInt(value.getType())));
            item.set_type(CustomFieldValueDataType::get_instance(custom_fields_get_int(
                value.r#type,
            )));
            // Java: item.setValue(DatatypeConverter.parseOutlineCodeValue(value.getValue(), field.getFieldType().getDataType()));
            item.set_value(
                value
                    .value
                    .as_deref()
                    .and_then(|text| DatatypeConverter::parse_outline_code_value(text, data_type)),
            );

            // Java: table.add(item);
            custom_field
                .borrow_mut()
                .get_lookup_table_mut()
                .add(item.to_stored_item());
        }
    }

    /// This method extracts the lookup table masks for an outline code/custom field from an MSPDI
    /// file.
    ///
    /// Corresponds to Java: `private void readOutlineCodeMasks(Project.OutlineCodes.OutlineCode outlineCode, FieldType fieldType)`
    fn read_outline_code_masks(&mut self, outline_code: &OutlineCode, field: FieldKey) {
        // Java:
        // Project.OutlineCodes.OutlineCode.Masks masks = outlineCode.getMasks();
        // if (masks != null && !masks.getMask().isEmpty())
        let Some(masks) = outline_code
            .masks
            .as_ref()
            .filter(|masks| !masks.mask.is_empty())
        else {
            return;
        };

        // Java: CustomField field = m_projectFile.getCustomFields().getOrCreate(fieldType);
        //       List<CustomFieldValueMask> maskList = field.getMasks();
        let container = self.file.borrow().get_custom_fields();
        let custom_field = CustomFieldContainer::get_or_create(&container, field);
        let mut custom_field = custom_field.borrow_mut();
        let mask_list = custom_field.get_masks_mut();

        // Java: for (Project.OutlineCodes.OutlineCode.Masks.Mask mask : masks.getMask())
        for mask in &masks.mask {
            // Java: int length = NumberHelper.getInt(mask.getLength());
            let length = custom_fields_get_int(mask.length);
            // Java: int level = NumberHelper.getInt(mask.getLevel());
            let level = custom_fields_get_int(mask.level);
            // Java: String separator = mask.getSeparator();
            let separator = mask.separator.clone();
            // Java:
            // CustomFieldValueDataType type = CustomFieldValueDataType.getInstanceByMaskValue(NumberHelper.getInt(mask.getType()));
            // if (type == null) { type = CustomFieldValueDataType.TEXT; }
            let value_type =
                CustomFieldValueDataType::get_instance_by_mask_value(custom_fields_get_int(
                    mask.r#type,
                ))
                .or(Some(CustomFieldValueDataType::Text));

            // Java: CustomFieldValueMask item = new CustomFieldValueMask(length, level, separator, type);
            //       maskList.add(item);
            mask_list.push(CustomFieldValueMask::new(
                length, level, separator, value_type,
            ));
        }
    }

    /// Given a value ID, retrieve the equivalent outline code value.
    ///
    /// Corresponds to Java: `private String getOutlineCodeValue(FieldType mpxFieldID, BigInteger valueID)`
    ///
    /// `field` is `None` where Java would pass a `null` field type and `value_id` is `None` where the
    /// `ValueID` element is absent (Java's `HashMap.get(null)` returns `null`); both yield `None`,
    /// matching Java's `getValueItem` returning `null`.
    ///
    /// The parent value is joined with `"."` unconditionally, exactly like Java, which carries a
    /// `// TODO: use the code mask` comment at that line.
    pub(crate) fn get_outline_code_value(
        &mut self,
        field: Option<FieldKey>,
        value_id: Option<i64>,
    ) -> Option<String> {
        // Java: CustomFieldValueItem item = getValueItem(mpxFieldID, valueID);
        //       if (item != null && item.getValue() != null) { ... }
        let item = self.get_value_item(field?, value_id?)?;
        let value = item.get_value()?;

        // Java: result = item.getValue().toString();
        let mut result = custom_fields_field_value_to_string(&value);

        // Java:
        // Integer parentID = item.getParentUniqueID();
        // if (parentID != null)
        // {
        //    String parentResult = getOutlineCodeValue(mpxFieldID, BigInteger.valueOf(parentID.longValue()));
        //    if (parentResult != null)
        //    {
        //       // TODO: use the code mask
        //       result = parentResult + "." + result;
        //    }
        // }
        let parent_id = item.get_parent_unique_id();
        if let Some(parent_id) = parent_id {
            let parent_result = self.get_outline_code_value(field, Some(i64::from(parent_id)));
            if let Some(parent_result) = parent_result {
                result = format!("{parent_result}.{result}");
            }
        }

        Some(result)
    }

    /// Given a value ID, retrieve the equivalent lookup table entry.
    ///
    /// Corresponds to Java: `private CustomFieldValueItem getValueItem(FieldType fieldType, BigInteger valueID)`
    ///
    /// Java's `computeIfAbsent` leaves an existing cache entry (even an empty map) alone, which is
    /// what the `contains_key` test reproduces. The returned handle is a copy of the lookup table
    /// entry (see the file header).
    pub(crate) fn get_value_item(
        &mut self,
        field: FieldKey,
        value_id: i64,
    ) -> Option<CustomFieldValueItem> {
        let key = custom_fields_to_mpp_field_key(field);

        if !self.custom_field_value_items.contains_key(&key) {
            let map = self.get_custom_field_value_item_map(field);
            self.custom_field_value_items.insert(key, map);
        }

        self.custom_field_value_items
            .get(&key)
            .and_then(|map| map.get(&value_id))
            .cloned()
    }

    /// Populate a cache of lookup table entries.
    ///
    /// Corresponds to Java: `private Map<BigInteger, CustomFieldValueItem> getCustomFieldValueItemMap(FieldType fieldType)`
    ///
    /// Java's map holds the items of the lookup table themselves; the Rust lookup table stores its
    /// elements by value, so each entry is turned into a handle carrying a copy of its fields (see
    /// the file header).
    pub(crate) fn get_custom_field_value_item_map(
        &mut self,
        field: FieldKey,
    ) -> HashMap<i64, CustomFieldValueItem> {
        // Java: CustomField field = m_projectFile.getCustomFields().get(fieldType);
        let container = self.file.borrow().get_custom_fields();
        // Java: if (field == null) { return Collections.emptyMap(); }
        let Some(custom_field) = container.borrow().get(field) else {
            return HashMap::new();
        };

        // Java: return field.getLookupTable().stream().collect(
        //          Collectors.toMap(i -> BigInteger.valueOf(i.getUniqueID().intValue()), i -> i));
        let entries = custom_field.borrow().get_lookup_table().as_slice().to_vec();

        entries
            .into_iter()
            .map(|entry| {
                let item = CustomFieldValueItem::new(entry.get_unique_id());
                item.set_guid(entry.get_guid());
                item.set_value(entry.get_value().cloned());
                item.set_description(entry.get_description().map(str::to_owned));
                item.set_unknown(entry.get_unknown().map(<[u8]>::to_vec));
                item.set_parent_unique_id(entry.get_parent_unique_id());
                item.set_type(entry.get_type());
                item.set_collapsed(entry.get_collapsed());

                (i64::from(entry.get_unique_id()), item)
            })
            .collect()
    }

    /// Resolve a cached lookup table field identity back into a field key.
    ///
    /// Rust-specific: Java stores the `FieldType` itself in `m_lookupTableMap`, but the Rust field is
    /// typed `HashMap<Uuid, MppFieldKey>` (see [`crate::mspdi::mspdi_reader`]), so a custom field has
    /// to be looked up again in the project's user defined field container to rebuild the
    /// [`CustomFieldKey`] that was used as its container key.
    fn custom_fields_field_key_from_mpp_key(&self, key: MppFieldKey) -> Option<FieldKey> {
        match key {
            MppFieldKey::Builtin(field) => Some(field),
            MppFieldKey::Custom(unique_id) => {
                let file = self.file.try_borrow().ok()?;
                let fields = file.get_user_defined_fields();
                fields
                    .get_by_unique_id(Some(unique_id))
                    .map(|field| FieldKey::Custom(CustomFieldKey::from_field(&field)))
            }
        }
    }
}

/// Rust-specific helper for Java's `value != null && !value.isEmpty()` test, expressed as an
/// `Option`: an empty string is treated as absent, which is how Java reaches the fallback branch.
fn custom_fields_non_empty(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.is_empty())
}

/// Map a field key to the hashable identity used as the value item cache key.
///
/// Corresponds to Java: using a `FieldType` as the key of `m_customFieldValueItems`. Java's
/// `UserDefinedField.hashCode` / `equals` use the field's unique ID, which is what
/// [`MppFieldKey::Custom`] carries.
fn custom_fields_to_mpp_field_key(field: FieldKey) -> MppFieldKey {
    match field {
        FieldKey::Custom(key) => MppFieldKey::Custom(key.get_unique_id()),
        other => MppFieldKey::Builtin(other),
    }
}

/// Rust-specific helper for Java's `NumberHelper.getInteger(BigInteger)`: `Integer.valueOf(value.intValue())`.
///
/// The schema layer narrows every `BigInteger` field to `Option<i64>`; Java's method takes a
/// `Number` and both call sites pass a `BigInteger`, so the widened value is passed to the same
/// Rust helper the resource section uses.
fn custom_fields_get_integer(value: Option<i64>) -> Option<i32> {
    NumberHelper::get_integer(value.map(|value| value as f64))
}

/// Rust-specific helper for Java's `NumberHelper.getInt(BigInteger)`: `value.intValue()`, zero for
/// `null`.
fn custom_fields_get_int(value: Option<i64>) -> i32 {
    NumberHelper::get_int(value.map(|value| value as f64))
}

/// Rust-specific helper for Java's `Object.toString()` applied to a lookup table value
/// (`item.getValue().toString()` in `getOutlineCodeValue`).
///
/// Only the value types `DatatypeConverter.parseOutlineCodeValue` can produce are rendered; the
/// remaining variants (notes, lists, entity references) have no Java-equivalent text form here and
/// yield an empty string. [`ReaderContext::tasks_get_outline_code_value`] uses the same rules.
fn custom_fields_field_value_to_string(value: &FieldValue) -> String {
    match value {
        FieldValue::String(text) => text.clone(),
        FieldValue::Integer(number) => number.to_string(),
        FieldValue::Short(number) => number.to_string(),
        // Java's `Double.toString` always keeps a decimal point ("2.0"), Rust's `Display` drops it.
        FieldValue::Double(number) => {
            if number.fract() == 0.0 && number.abs() < 1.0e7 {
                format!("{number:.1}")
            } else {
                number.to_string()
            }
        }
        FieldValue::Boolean(flag) => flag.to_string(),
        // Java's `LocalDateTime.toString` omits the seconds (and the fraction) when they are zero.
        FieldValue::DateTime(date_time) => {
            if date_time.second() == 0 && date_time.nanosecond() == 0 {
                date_time.format("%Y-%m-%dT%H:%M").to_string()
            } else if date_time.nanosecond() == 0 {
                date_time.format("%Y-%m-%dT%H:%M:%S").to_string()
            } else {
                date_time.format("%Y-%m-%dT%H:%M:%S%.f").to_string()
            }
        }
        // `Duration`'s `Display` renders `value + units` like Java, except that Rust drops a trailing
        // `.0` on integer amounts (`8h` where Java's `Double.toString` yields `8.0h`); this is the
        // "Known deviation" row in the `duration.rs` file header, not a deliberate rendering rule here.
        FieldValue::Duration(duration) => duration.to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::cell::RefCell;
    use std::io::Cursor;
    use std::rc::Rc;

    use uuid::Uuid;

    use crate::data_type::DataType;
    use crate::field_type_class::FieldTypeClass;
    use crate::mspdi::mspdi_reader::MSPDIReader;
    use crate::project_file::ProjectFile;
    use crate::resource::Resource;
    use crate::resource_field::ResourceField;
    use crate::task::Task;
    use crate::task_field::TaskField;

    /// Parse a literal MSPDI document with the reader under test.
    fn read(xml: &str) -> Rc<RefCell<ProjectFile>> {
        let mut reader = MSPDIReader::new();
        reader
            .read(&mut Cursor::new(xml.as_bytes()))
            .expect("valid MSPDI document")
    }

    /// The field key of the enterprise (custom) task field whose MPP field ID is 188776448:
    /// `TASK_FIELD_BASE | 0x8000`, i.e. the first field ID that `MPPTaskField` maps to a
    /// `UserDefinedField`.
    fn enterprise_task_field() -> FieldKey {
        FieldKey::Custom(CustomFieldKey::new(
            crate::common::mpp_task_field::MPPTaskField::TASK_FIELD_BASE | 0x8000,
            FieldTypeClass::Task,
            DataType::Custom,
        ))
    }

    /// The alias of a custom field configuration, if one exists.
    fn alias_of(file: &Rc<RefCell<ProjectFile>>, field: FieldKey) -> Option<String> {
        let container = file.borrow().get_custom_fields();
        let custom_field = container.borrow().get(field)?;
        let alias = custom_field.borrow().get_alias().map(str::to_owned);
        alias
    }

    /// An `ExtendedAttribute` with an `Alias` writes the alias onto the project's custom field
    /// container (and registers it in the alias index).
    #[test]
    fn extended_attribute_alias_lands_on_the_custom_field_container() {
        let file = read(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Project xmlns="http://schemas.microsoft.com/project">
  <ExtendedAttributes>
    <ExtendedAttribute>
      <FieldID>188743731</FieldID>
      <FieldName>Text1</FieldName>
      <Alias>Cost Code</Alias>
    </ExtendedAttribute>
  </ExtendedAttributes>
</Project>"#,
        );

        let text1 = FieldKey::Task(TaskField::Text1);
        assert_eq!(
            alias_of(&file, text1),
            Some("Cost Code".to_owned()),
            "the Alias element of the extended attribute definition"
        );

        let container = file.borrow().get_custom_fields();
        assert_eq!(
            container
                .borrow()
                .get_field_type_by_alias(FieldTypeClass::Task, "Cost Code"),
            Some(text1)
        );
    }

    /// An enterprise definition without an `Alias` falls back to `FieldName`; the fallback only
    /// applies to a `UserDefinedField` (the custom field of the document), so the `FieldName` of a
    /// built-in field is not turned into an alias.
    #[test]
    fn enterprise_definition_without_alias_uses_the_field_name() {
        let file = read(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Project xmlns="http://schemas.microsoft.com/project">
  <ExtendedAttributes>
    <ExtendedAttribute>
      <FieldID>188776448</FieldID>
      <FieldName>Enterprise Text 1</FieldName>
    </ExtendedAttribute>
    <ExtendedAttribute>
      <FieldID>188743731</FieldID>
      <FieldName>Text1</FieldName>
    </ExtendedAttribute>
  </ExtendedAttributes>
</Project>"#,
        );

        let container = file.borrow().get_custom_fields();

        let enterprise = enterprise_task_field();
        let custom_field = container
            .borrow()
            .get(enterprise)
            .expect("the enterprise field definition is registered");
        // The container is keyed by the `FieldKey` built from the `UserDefinedField` (unique ID
        // `TASK_FIELD_BASE | 0x8000`).
        assert_eq!(custom_field.borrow().get_field_type(), enterprise);
        assert_eq!(
            custom_field.borrow().get_alias(),
            Some("Enterprise Text 1")
        );

        // The built-in Text1 has a FieldName but no Alias: `field instanceof UserDefinedField` is
        // false, so no configuration (and no alias) is created for it.
        assert_eq!(alias_of(&file, FieldKey::Task(TaskField::Text1)), None);
        assert_eq!(container.borrow().size(), 1);
    }

    /// The lookup table values are stored on the custom field configuration of the outline code
    /// field, together with the table flags.
    #[test]
    fn outline_code_values_are_stored_in_the_lookup_table() {
        let file = read(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Project xmlns="http://schemas.microsoft.com/project">
  <OutlineCodes>
    <OutlineCode>
      <Guid>D8C7E507-A694-44B6-A2D2-64B892118ADC</Guid>
      <FieldID>205521174</FieldID>
      <FieldName>Outline Code1</FieldName>
      <Values>
        <Value>
          <ValueID>1</ValueID>
          <ParentValueID>0</ParentValueID>
          <Value>Alpha</Value>
        </Value>
        <Value>
          <ValueID>2</ValueID>
          <ParentValueID>1</ParentValueID>
          <Value>Beta</Value>
        </Value>
      </Values>
      <Enterprise>1</Enterprise>
      <ResourceSubstitutionEnabled>0</ResourceSubstitutionEnabled>
      <LeafOnly>0</LeafOnly>
      <AllLevelsRequired>1</AllLevelsRequired>
      <OnlyTableValuesAllowed>0</OnlyTableValuesAllowed>
      <ShowIndent>0</ShowIndent>
    </OutlineCode>
  </OutlineCodes>
</Project>"#,
        );

        let container = file.borrow().get_custom_fields();
        let custom_field = container
            .borrow()
            .get(FieldKey::Resource(ResourceField::OutlineCode1))
            .expect("the outline code definition is registered");
        let custom_field = custom_field.borrow();
        let table = custom_field.get_lookup_table();

        assert_eq!(table.len(), 2);
        assert_eq!(
            table.get_guid(),
            Some(Uuid::parse_str("D8C7E507-A694-44B6-A2D2-64B892118ADC").expect("valid UUID"))
        );
        assert!(table.get_enterprise());
        assert!(table.get_all_levels_required());
        assert!(!table.get_leaf_only());
        assert!(!table.get_resource_substitution_enabled());
        assert!(!table.get_only_table_values_allowed());
        // `<ShowIndent>0</ShowIndent>` overrides the `true` the table starts with.
        assert!(!table.get_show_indent());

        let first = table.get_by_unique_id(1).expect("value ID 1");
        assert_eq!(first.get_value(), Some(&FieldValue::String("Alpha".to_owned())));
        assert_eq!(first.get_parent_unique_id(), Some(0));
        let second = table.get_by_unique_id(2).expect("value ID 2");
        assert_eq!(second.get_value(), Some(&FieldValue::String("Beta".to_owned())));
        assert_eq!(second.get_parent_unique_id(), Some(1));
    }

    /// A resource `OutlineCode` element is resolved through the lookup table read by this section:
    /// the parent value is joined with `"."` (Java's `// TODO: use the code mask`) even though the
    /// mask names `"/"` as the separator.
    #[test]
    fn resolves_a_resource_outline_code_value_with_the_parent_join() {
        let file = read(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Project xmlns="http://schemas.microsoft.com/project">
  <OutlineCodes>
    <OutlineCode>
      <Guid>D8C7E507-A694-44B6-A2D2-64B892118ADC</Guid>
      <FieldID>205521174</FieldID>
      <Values>
        <Value>
          <ValueID>1</ValueID>
          <ParentValueID>0</ParentValueID>
          <Value>Alpha</Value>
        </Value>
        <Value>
          <ValueID>2</ValueID>
          <ParentValueID>1</ParentValueID>
          <Value>Beta</Value>
        </Value>
      </Values>
      <Masks>
        <Mask>
          <Level>1</Level>
          <Type>0</Type>
          <Length>0</Length>
          <Separator>/</Separator>
        </Mask>
      </Masks>
    </OutlineCode>
  </OutlineCodes>
  <Resources>
    <Resource>
      <UID>1</UID>
      <ID>1</ID>
      <Name>Resource One</Name>
      <OutlineCode>
        <FieldID>205521174</FieldID>
        <ValueID>2</ValueID>
      </OutlineCode>
    </Resource>
  </Resources>
</Project>"#,
        );

        let resources: Vec<Rc<RefCell<Resource>>> =
            file.borrow().get_resources().iter().cloned().collect();
        assert_eq!(resources.len(), 1);

        assert_eq!(
            resources[0]
                .borrow()
                .get(FieldKey::Resource(ResourceField::OutlineCode1)),
            Some(FieldValue::String("Alpha.Beta".to_owned()))
        );
    }

    /// The same resolution through the task `OutlineCode` element; the task section calls the same
    /// Java method (`getOutlineCodeValue`), which it currently reproduces locally (see the
    /// differences table of [`crate::mspdi::reader_tasks`]).
    #[test]
    fn resolves_a_task_outline_code_value_with_the_parent_join() {
        let file = read(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Project xmlns="http://schemas.microsoft.com/project">
  <OutlineCodes>
    <OutlineCode>
      <FieldID>188743731</FieldID>
      <Values>
        <Value>
          <ValueID>1</ValueID>
          <ParentValueID>0</ParentValueID>
          <Value>Alpha</Value>
        </Value>
        <Value>
          <ValueID>2</ValueID>
          <ParentValueID>1</ParentValueID>
          <Value>Beta</Value>
        </Value>
      </Values>
    </OutlineCode>
  </OutlineCodes>
  <Tasks>
    <Task>
      <UID>1</UID>
      <ID>1</ID>
      <Name>Task One</Name>
      <OutlineCode>
        <FieldID>188743731</FieldID>
        <ValueID>2</ValueID>
      </OutlineCode>
    </Task>
  </Tasks>
</Project>"#,
        );

        let tasks: Vec<Rc<RefCell<Task>>> = file.borrow().get_tasks().iter().cloned().collect();
        assert_eq!(tasks.len(), 1);

        assert_eq!(
            tasks[0].borrow().get(FieldKey::Task(TaskField::Text1)),
            Some(FieldValue::String("Alpha.Beta".to_owned()))
        );
    }

    /// A value ID that is not in the lookup table resolves to nothing, and so does a lookup table
    /// that does not exist.
    #[test]
    fn unknown_value_ids_yield_no_value() {
        let file = read(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Project xmlns="http://schemas.microsoft.com/project">
  <OutlineCodes>
    <OutlineCode>
      <FieldID>205521174</FieldID>
      <Values>
        <Value>
          <ValueID>1</ValueID>
          <Value>Alpha</Value>
        </Value>
      </Values>
    </OutlineCode>
  </OutlineCodes>
  <Resources>
    <Resource>
      <UID>1</UID>
      <ID>1</ID>
      <OutlineCode>
        <FieldID>205521174</FieldID>
        <ValueID>7</ValueID>
      </OutlineCode>
    </Resource>
  </Resources>
</Project>"#,
        );

        let resources: Vec<Rc<RefCell<Resource>>> =
            file.borrow().get_resources().iter().cloned().collect();
        // Java's `set(field, null)` clears the field, so nothing is readable back.
        assert_eq!(
            resources[0]
                .borrow()
                .get(FieldKey::Resource(ResourceField::OutlineCode1)),
            None
        );
    }

    /// The masks are read with the length, level, separator and the mask-value mapping of
    /// `CustomFieldValueDataType`; an unrecognised type falls back to `TEXT`.
    #[test]
    fn outline_code_masks_are_read() {
        let file = read(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Project xmlns="http://schemas.microsoft.com/project">
  <OutlineCodes>
    <OutlineCode>
      <FieldID>205521174</FieldID>
      <Masks>
        <Mask>
          <Level>1</Level>
          <Type>7</Type>
          <Length>3</Length>
          <Separator>.</Separator>
        </Mask>
        <Mask>
          <Level>2</Level>
          <Type>99</Type>
          <Length>0</Length>
        </Mask>
      </Masks>
    </OutlineCode>
  </OutlineCodes>
</Project>"#,
        );

        let container = file.borrow().get_custom_fields();
        let custom_field = container
            .borrow()
            .get(FieldKey::Resource(ResourceField::OutlineCode1))
            .expect("the outline code definition is registered");
        let custom_field = custom_field.borrow();
        let masks = custom_field.get_masks();

        assert_eq!(masks.len(), 2);
        // Mask value 7 is CustomFieldValueDataType.NUMBER.
        assert_eq!(masks[0].get_length(), 3);
        assert_eq!(masks[0].get_level(), 1);
        assert_eq!(masks[0].get_separator(), Some("."));
        assert_eq!(masks[0].get_type(), Some(CustomFieldValueDataType::Number));
        // An unknown mask value falls back to TEXT.
        assert_eq!(masks[1].get_length(), 0);
        assert_eq!(masks[1].get_level(), 2);
        assert_eq!(masks[1].get_separator(), None);
        assert_eq!(masks[1].get_type(), Some(CustomFieldValueDataType::Text));
    }

    /// A definition whose `FieldID` does not map to a field (and one without a `FieldID` at all) is
    /// skipped instead of aborting the read.
    #[test]
    fn definitions_without_a_usable_field_id_are_skipped() {
        let file = read(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Project xmlns="http://schemas.microsoft.com/project">
  <OutlineCodes>
    <OutlineCode>
      <FieldID>1</FieldID>
      <Values>
        <Value>
          <ValueID>1</ValueID>
          <Value>Alpha</Value>
        </Value>
      </Values>
    </OutlineCode>
    <OutlineCode>
      <Guid>D8C7E507-A694-44B6-A2D2-64B892118ADC</Guid>
      <Values>
        <Value>
          <ValueID>1</ValueID>
          <Value>Beta</Value>
        </Value>
      </Values>
    </OutlineCode>
  </OutlineCodes>
  <ExtendedAttributes>
    <ExtendedAttribute>
      <FieldID>0000</FieldID>
      <Alias>Not a field</Alias>
    </ExtendedAttribute>
  </ExtendedAttributes>
</Project>"#,
        );

        // Neither outline code produced a configuration (field ID 1 has no known prefix; the
        // unknown GUID is not in the lookup table map), and the unparsable extended attribute field
        // ID produced no alias.
        let container = file.borrow().get_custom_fields();
        assert!(container.borrow().is_empty());
    }

    /// `get_value_item` caches the lookup table entries per field, and `get_outline_code_value`
    /// renders the value types of the lookup table.
    #[test]
    fn caches_lookup_table_items_and_renders_their_values() {
        let mut reader = MSPDIReader::new();
        let file = reader
            .read(&mut Cursor::new(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Project xmlns="http://schemas.microsoft.com/project">
  <OutlineCodes>
    <OutlineCode>
      <FieldID>205521174</FieldID>
      <Values>
        <Value>
          <ValueID>1</ValueID>
          <Value>Alpha</Value>
          <Description>first</Description>
        </Value>
      </Values>
    </OutlineCode>
  </OutlineCodes>
</Project>"#
                    .as_slice(),
            ))
            .expect("valid MSPDI document");

        let mut context = ReaderContext::new(Rc::clone(&file), true);
        let field = FieldKey::Resource(ResourceField::OutlineCode1);

        let item = context
            .get_value_item(field, 1)
            .expect("value ID 1 is in the lookup table");
        assert_eq!(item.get_value(), Some(FieldValue::String("Alpha".to_owned())));
        assert_eq!(item.get_description(), Some("first".to_owned()));

        assert_eq!(
            context.get_outline_code_value(Some(field), Some(1)),
            Some("Alpha".to_owned())
        );
        // The cache holds the field's entries after the first lookup.
        assert!(context
            .custom_field_value_items
            .contains_key(&MppFieldKey::Builtin(field)));
    }
}
