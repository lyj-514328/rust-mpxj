//! Corresponds to Java: `org/mpxj/mpp/FieldMap.java`
//!
//! This class represents the "field -> data block location" mapping in an MPP file: it parses the field layout table in the `Props` stream
//! (28 bytes per record) into [`FieldItem`], and is responsible for reading from fixed data blocks / variable data blocks
//! individual field values, finally writing them into `FieldContainer` (Task / Resource / ResourceAssignment).
//!
//! # Differences from Java (trade-offs are centralized in the file header; the body only marks key points)
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `abstract class FieldMap` | the concrete `pub struct FieldMap` (no inheritance) | Java's 5 abstract methods [`FieldMap::use_type_as_var_data_key`] / `getDefaultTaskData` / `getDefaultResourceData` / `getDefaultAssignmentData` / `getDefaultRelationData` become **configurable state**: reading methods of the same name plus `set_use_type_as_var_data_key` / `set_default_*_data` writer methods. `FieldMap9` / `FieldMap12` / `FieldMap14` are no longer subclasses in Rust; instead they "construct `FieldMap` and configure these items" |
//! | inner class `FieldItem` | top-level `pub struct FieldItem` in the same file | changed to a standalone type per the task convention. Java's `FieldItem` is a **non-static inner class** and can implicitly access the outer `FieldMap`'s `getFieldData` / `getProjectProperties`; the Rust side turns these calls into an explicit `&FieldMap` parameter (equivalent to Java's `FieldMap.this`) |
//! | inner enum `FieldLocation` | top-level `pub enum FieldLocation` in the same file | variants match the declaration order (`FIXED_DATA < VAR_DATA < META_DATA < UNKNOWN`), so `Ord` is equivalent to Java's `compareTo` |
//! | `Map<FieldType, FieldItem> m_map` | `HashMap<MppFieldKey, FieldItem>` | follows the convention that "Java maps keyed by `FieldType` use `MppFieldKey` instead" |
//! | `Set<FieldType> getFieldTypes()` | `Vec<MppFieldType>` | Java returns `m_map.keySet()` (a set of `FieldType` references); Rust's key [`MppFieldKey`] cannot reproduce the custom field handle, so it returns a copy of each item's `MppFieldType` instead (keys and values correspond one-to-one, the set contents are equivalent, and the order is not guaranteed just as in Java) |
//! | `Integer` parameters / return values | `Option<i32>` | the "`null` -> `Option<T>`" convention |
//! | `byte[][] fixedData` | `&[Option<&[u8]>]` | in Java both the elements **and the array itself** can be `null`; Rust uses `None` for an empty element and an empty slice for an empty array. Java throws an NPE when indexing a `null` array; the Rust side returns `None` |
//! | out-of-bounds reads (`ByteArrayHelper.getShort/getInt`, `m_maxFixedDataSize[i]`) | return `None` / skip the record | Java throws `ArrayIndexOutOfBoundsException` |
//! | `System.out.println` (`m_debug`) | `println!` | output only when [`FieldMap::set_debug`] is enabled, with the same format as Java (`DataType` uses Rust's `Debug` name, whereas Java prints the enum constant name, differing only in case) |
//! | `toString()` | `Display` | sorting uses [`FieldItem::compare_to`] (Java's `Collections.sort` relies on `Comparable`) |
//! | `compareTo` / `equals` / `hashCode` | `Ord` + `PartialOrd` + `compare_to`; `PartialEq` / `Eq` / `Hash` | Java's `compareTo` compares only "location" and is inconsistent with `equals`; the Rust side provides both Java's original `compare_to` (returning `int`) and an `Ord` that delegates to it (used for `Display` sorting), and just like Java does **not guarantee** that `Ord` and `PartialEq` agree |
//! | `getFieldType(int)` delegates to `FieldTypeHelper.getInstance(project, fieldID)` | [`FieldTypeHelper::get_instance`] | the same delegation as Java; `crate::common::field_type_helper` was ported in the same change as this file |
//!
//! ## Representation of field values (consistent with the existing conventions for `Task` / `Resource` / `ResourceAssignment`)
//!
//! Java's `FieldItem.read` returns `Object`, whereas the Rust side returns [`FieldValue`]. `FieldValue` currently has only
//! scalar variants, so it is encoded per the conventions already established in this crate (see item 7 of the differences in the `crate::task` file header):
//!
//! | Java return value | Rust |
//! |---|---|
//! | `Integer`(`INTEGER` / `SHORT`) | `FieldValue::Integer` |
//! | `Double`(`CURRENCY` / `UNITS` / `NUMERIC` / `PERCENTAGE`) | `FieldValue::Double` |
//! | `Boolean` | `FieldValue::Boolean` |
//! | `LocalDateTime`(`DATE`) | `FieldValue::DateTime` |
//! | `String`(`STRING`) | `FieldValue::String` |
//! | `Duration`(`WORK` / `DURATION` / `DELAY`) | `FieldValue::Duration` |
//! | `Rate`(`RATE`) | `FieldValue::Rate` |
//! | various enums (`TIME_UNITS` / `CONSTRAINT` / `PRIORITY` / `TASK_TYPE` / `ACCRUE` / `WORKGROUP` / `RATE_UNITS` / `EARNED_VALUE_METHOD` / `RESOURCE_REQUEST_TYPE` / `BOOKING_TYPE`) | `FieldValue::Integer` (the enum's int value; `TaskType` has no int value, so it is encoded by the index into `TaskType::ALL`, consistent with `Task`) |
//! | `UUID` (`GUID`) | `FieldValue::String` (`UUID.toString()`; the caller can restore it with `Uuid::parse_str`) |
//! | `RtfNotes` (`NOTES`) | `FieldValue::String` (plain text with RTF markup removed, consistent with `Task::get_notes`; the original RTF is lost) |
//! | `byte[]` (`BINARY`) | `FieldValue::Bytes` (variable-length data) | the raw bytes of variable-length fields are preserved as in Java; fixed-length data is still not read, as in Java |
//!
//! ## Known functional gaps
//!
//! * **A custom field's name cannot be recovered from the key**: [`FieldKey::Custom`] is a `Copy` equivalence key
//!   that carries only the unique ID and field type/data type, not the dynamic name; to get the real name, read
//!   `UserDefinedField`.
//! * **Unknown fields are skipped**: Java's `FieldTypeHelper.getInstance` generates a
//!   placeholder `FieldType` (`FieldTypeClass.UNKNOWN`, with `getDataType()` being `null`) for an unrecognized field number and writes it into `m_map`;
//!   the Rust side's [`FieldMap::get_field_type`] can only return `None`, and `createFieldMap` then skips the record
//!   (this affects neither the other records nor the advancement of the data block index).
//! * **`ProjectFile` borrow conflict**: `ProjectProperties` is held by `ProjectFile`, and the Rust side can only access it with
//!   `RefCell::try_borrow` (see [`FieldMap::try_project_properties`]). When calling
//!   `populateContainer` / `read`, **do not hold a mutable borrow of `ProjectFile`**, otherwise duration-type fields
//!   cannot be converted (Java holds a stable reference, so this situation does not arise).
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class uses none of these mechanisms, so nothing is skipped.
//!
//! Skipped Java methods: none (all of `FieldMap`'s methods have been translated, including the inner class `FieldItem` and enum
//! `FieldLocation`).

use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use crate::accrue_type::AccrueType;
use crate::booking_type::BookingType;
use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::mpp_field_type::{MppFieldKey, MppFieldType};
use crate::common::number_helper::NumberHelper;
use crate::constraint_type::ConstraintType;
use crate::custom_field_container::CustomFieldContainer;
use crate::data_type::DataType;
use crate::duration::Duration;
use crate::earned_value_method::EarnedValueMethod;
use crate::field_container::FieldContainer;
use crate::field_key::FieldKey;
use crate::field_type::FieldType;
use crate::field_type_class::FieldTypeClass;
use crate::field_value::FieldValue;
use crate::mpp::double_var_data_field_reader::DoubleVarDataFieldReader;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::props::Props;
use crate::mpp::props_key::PropsKey;
use crate::mpp::string_var_data_field_reader::StringVarDataFieldReader;
use crate::mpp::timestamp_var_data_field_reader::TimestampVarDataFieldReader;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_data_field_reader::VarDataFieldReader;
use crate::mpp::var_meta::VarDataKeyResolver;
use crate::mpxj_enum::MpxjEnum;
use crate::priority::Priority;
use crate::project_file::ProjectFile;
use crate::project_properties::ProjectProperties;
use crate::rate::Rate;
use crate::resource_request_type::ResourceRequestType;
use crate::rtf_notes::RtfNotes;
use crate::task_type::TaskType;
use crate::time_unit::TimeUnit;
use crate::work_group::WorkGroup;

/// Number of bytes in a single record of the field layout table.
///
/// Corresponds to Java: `index += 28` in `createFieldMap`
const FIELD_MAP_RECORD_SIZE: usize = 28;

/// Maximum number of fixed data blocks.
///
/// Corresponds to Java: `private static final int MAX_FIXED_DATA_BLOCKS = 2;`
const MAX_FIXED_DATA_BLOCKS: usize = 2;

/// Fixed data offset meaning "no data / see variable data block".
///
/// Corresponds to Java: `dataBlockOffset != 65535` in `createFieldMap`
const NO_FIXED_DATA_OFFSET: i32 = 65535;

/// `Props` keys used by the task field layout table (the first one present is taken in order).
///
/// Corresponds to Java: `private static final PropsKey[] TASK_KEYS`
const TASK_KEYS: [PropsKey; 2] = [PropsKey::TaskFieldMap, PropsKey::TaskFieldMap2];

/// `Props` key used by the enterprise custom field layout table.
///
/// Corresponds to Java: `private static final PropsKey[] ENTERPRISE_CUSTOM_KEYS`
const ENTERPRISE_CUSTOM_KEYS: [PropsKey; 1] = [PropsKey::EnterpriseCustomFieldMap];

/// `Props` keys used by the resource field layout table (the first one present is taken in order).
///
/// Corresponds to Java: `private static final PropsKey[] RESOURCE_KEYS`
const RESOURCE_KEYS: [PropsKey; 2] = [PropsKey::ResourceFieldMap, PropsKey::ResourceFieldMap2];

/// `Props` keys used by the assignment field layout table (the first one present is taken in order).
///
/// Corresponds to Java: `private static final PropsKey[] ASSIGNMENT_KEYS`
const ASSIGNMENT_KEYS: [PropsKey; 2] = [
    PropsKey::AssignmentFieldMap,
    PropsKey::AssignmentFieldMap2,
];

/// `Props` key used by the relation field layout table.
///
/// Corresponds to Java: `private static final PropsKey[] RELATION_KEYS`
const RELATION_KEYS: [PropsKey; 1] = [PropsKey::RelationFieldMap];

/// The data block in which the field data resides.
///
/// Corresponds to Java: `enum FieldLocation`
///
/// The variant order matches Java's declaration order: Java's `FieldLocation.compareTo` compares by ordinal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FieldLocation {
    /// Fixed data block.
    FixedData,
    /// Variable data block.
    VarData,
    /// Metadata block (Java has not yet worked out where the boolean flags are in this block, so `read` ignores it).
    MetaData,
    /// Location unknown.
    Unknown,
}

impl FieldLocation {
    /// All values, in the same order as Java's declaration order.
    pub const ALL: [FieldLocation; 4] = [
        FieldLocation::FixedData,
        FieldLocation::VarData,
        FieldLocation::MetaData,
        FieldLocation::Unknown,
    ];

    /// Returns the Java enum constant name (for example `"FIXED_DATA"`).
    pub fn name(&self) -> &'static str {
        match self {
            FieldLocation::FixedData => "FIXED_DATA",
            FieldLocation::VarData => "VAR_DATA",
            FieldLocation::MetaData => "META_DATA",
            FieldLocation::Unknown => "UNKNOWN",
        }
    }
}

impl Display for FieldLocation {
    /// Corresponds to Java: the enum's built-in `toString()` (outputs the constant name)
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Describes the location of a single field in an MPP file and provides value accessors.
///
/// Corresponds to Java: `class FieldItem implements Comparable<FieldItem>`
///
/// `Clone` is Rust-specific: `Display` (Java's `toString`) copies the table contents before sorting.
#[derive(Clone)]
pub struct FieldItem {
    /// Field type.
    ///
    /// Corresponds to Java field: `private final FieldType m_type;`
    type_: MppFieldType,

    /// The data block the field is in.
    ///
    /// Corresponds to Java field: `private final FieldLocation m_location;`
    location: FieldLocation,

    /// Fixed data block index.
    ///
    /// Corresponds to Java field: `private final int m_fixedDataBlockIndex;`
    fixed_data_block_index: i32,

    /// Offset within the fixed data block (`VAR_DATA` items keep the 65535 used in Java).
    ///
    /// Corresponds to Java field: `private final int m_fixedDataOffset;`
    fixed_data_offset: i32,

    /// Key of the variable data block.
    ///
    /// Corresponds to Java field: `private final Integer m_varDataKey;`
    /// (Java assigns it in the constructor with `Integer.valueOf`, so it is never `null`; hence Rust uses `i32`;
    /// the public [`FieldItem::get_var_data_key`] still returns `Option<i32>`)
    var_data_key: i32,

    /// Metadata bit mask.
    ///
    /// Corresponds to Java field: `private final long m_mask;`
    mask: i64,

    /// Metadata block index.
    ///
    /// Corresponds to Java field: `private final int m_metaBlock;`
    meta_block: i32,
}

impl FieldItem {
    /// Constructor.
    ///
    /// Corresponds to Java: `FieldItem(FieldType type, FieldLocation location, int fixedDataBlockIndex, int fixedDataOffset, int varDataKey, long mask, int metaBlock)`
    pub fn new(
        type_: MppFieldType,
        location: FieldLocation,
        fixed_data_block_index: i32,
        fixed_data_offset: i32,
        var_data_key: i32,
        mask: i64,
        meta_block: i32,
    ) -> Self {
        FieldItem {
            type_,
            location,
            fixed_data_block_index,
            fixed_data_offset,
            var_data_key,
            mask,
            meta_block,
        }
    }

    /// Reads the value of a single field.
    ///
    /// Corresponds to Java: `public Object read(Integer id, byte[][] fixedData, Var2Data varData)`
    ///
    /// `field_map` corresponds to the outer `FieldMap` instance implicitly held by the Java inner class (`FieldMap.this`).
    pub fn read(
        &self,
        field_map: &FieldMap,
        id: Option<i32>,
        fixed_data: &[Option<&[u8]>],
        var_data: &Var2Data,
    ) -> Option<FieldValue> {
        match self.location {
            FieldLocation::FixedData => self.read_fixed_data(field_map, id, fixed_data, var_data),
            FieldLocation::VarData => self.read_var_data(
                field_map,
                self.type_.get_data_type(),
                id,
                fixed_data,
                var_data,
            ),
            // Java comment: the boolean flags are stored in the "metadata" block, but it is not yet known how to map the 4-byte
            // bit mask in the field layout table header to a data location, so the META_DATA location is ignored (no value is read).
            FieldLocation::MetaData => None,
            // Java: location unknown -- ignore.
            FieldLocation::Unknown => None,
        }
    }

    /// Reads a field value from a fixed data block.
    ///
    /// Corresponds to Java: `private Object readFixedData(Integer id, byte[][] fixedData, Var2Data varData)`
    fn read_fixed_data(
        &self,
        field_map: &FieldMap,
        id: Option<i32>,
        fixed_data: &[Option<&[u8]>],
        var_data: &Var2Data,
    ) -> Option<FieldValue> {
        // Java: if (m_fixedDataBlockIndex < fixedData.length) { byte[] data = fixedData[m_fixedDataBlockIndex]; ... }
        let block_index = usize::try_from(self.fixed_data_block_index).ok()?;
        let data = fixed_data.get(block_index).copied().flatten()?;

        // Java: if (data != null && m_fixedDataOffset < data.length)
        let offset = usize::try_from(self.fixed_data_offset).ok()?;
        if offset >= data.len() {
            return None;
        }

        match self.type_.get_data_type() {
            DataType::Date => {
                // Java: result = MPPUtility.getTimestamp(data, m_fixedDataOffset);
                MPPUtility::get_timestamp(data, offset).map(FieldValue::DateTime)
            }

            DataType::Integer => {
                // Java: Integer.valueOf(ByteArrayHelper.getInt(data, m_fixedDataOffset))
                Some(FieldValue::Integer(checked_int(data, offset)?))
            }

            DataType::Duration => {
                // Java: TimeUnit units = (TimeUnit) getFieldData(id, unitsType, fixedData, varData);
                //       if (units == null) { units = getProjectProperties().getDefaultDurationUnits(); }
                let units = self
                    .units_type()
                    .and_then(|units_type| {
                        field_map.get_field_data(
                            id,
                            &MppFieldType::Builtin(units_type),
                            fixed_data,
                            var_data,
                        )
                    })
                    .and_then(|value| value.as_integer())
                    .map(TimeUnit::get_instance)
                    .unwrap_or_else(|| field_map.default_duration_units());

                // Java: MPPUtility.getAdjustedDuration(getProjectProperties(), getInt(data, offset), units)
                field_map
                    .adjusted_duration(checked_int(data, offset)?, units)
                    .map(FieldValue::Duration)
            }

            DataType::TimeUnits => {
                // Java: MPPUtility.getDurationTimeUnits(getShort(data, offset), getProjectProperties().getDefaultDurationUnits())
                let default_units = field_map.try_project_properties();
                let default_units = default_units
                    .as_deref()
                    .map(ProjectProperties::get_default_duration_units);
                let units = MPPUtility::get_duration_time_units_with_default(
                    checked_short(data, offset)?,
                    default_units,
                );
                Some(FieldValue::Integer(units.value()))
            }

            DataType::Constraint => {
                // Java: ConstraintType.getInstance(ByteArrayHelper.getShort(data, m_fixedDataOffset))
                let value = checked_short(data, offset)?;
                Some(FieldValue::Integer(ConstraintType::get_instance(value).value()))
            }

            DataType::Priority => {
                // Java: Priority.getInstance(ByteArrayHelper.getShort(data, m_fixedDataOffset))
                let value = checked_short(data, offset)?;
                Some(FieldValue::Integer(
                    Priority::get_instance(value).get_value(),
                ))
            }

            DataType::Percentage => {
                // Java: MPPUtility.getPercentage(data, m_fixedDataOffset) (Java returns null when out of bounds/invalid)
                MPPUtility::get_percentage(data, offset).map(FieldValue::Double)
            }

            DataType::TaskType => {
                // Java: TaskTypeHelper.getInstance(ByteArrayHelper.getShort(data, m_fixedDataOffset))
                let value = checked_short(data, offset)?;
                Some(FieldValue::Integer(task_type_to_value(mpp_value_to_task_type(
                    value,
                ))))
            }

            DataType::Accrue => {
                // Java: AccrueType.getInstance(ByteArrayHelper.getShort(data, m_fixedDataOffset)) (may be null)
                let value = checked_short(data, offset)?;
                AccrueType::get_instance(value)
                    .map(|accrue_type| FieldValue::Integer(accrue_type.value()))
            }

            DataType::Currency | DataType::Units => {
                // Java comment: ignore the amount when the result is less than 0.1 cent / 0.1%
                // Java: double amount = MPPUtility.getDouble(data, m_fixedDataOffset);
                //       amount = Math.abs(amount) < 0.1 ? 0 : amount;
                //       result = NumberHelper.getDouble(amount / 100);
                let amount = MPPUtility::get_double(data, offset);
                let amount = if amount.abs() < 0.1 { 0.0 } else { amount };
                Some(FieldValue::Double(NumberHelper::get_double_value(
                    amount / 100.0,
                )))
            }

            DataType::Rate => {
                // Java: new Rate(MPPUtility.getDouble(data, m_fixedDataOffset), TimeUnit.HOURS)
                Some(FieldValue::Rate(Rate::new(
                    MPPUtility::get_double(data, offset),
                    TimeUnit::Hours,
                )))
            }

            DataType::Work => {
                // Java comment: ignore the duration when the result is less than 1 minute
                let duration = MPPUtility::get_double(data, offset);
                let duration = if duration.abs() < 1000.0 {
                    0.0
                } else {
                    duration
                };
                Some(FieldValue::Duration(Duration::get_instance(
                    duration / 60000.0,
                    TimeUnit::Hours,
                )))
            }

            DataType::Short => {
                // Java: Integer.valueOf(ByteArrayHelper.getShort(data, m_fixedDataOffset))
                Some(FieldValue::Integer(checked_short(data, offset)?))
            }

            DataType::Boolean => {
                // Java: Boolean.valueOf(ByteArrayHelper.getShort(data, m_fixedDataOffset) != 0)
                Some(FieldValue::Boolean(checked_short(data, offset)? != 0))
            }

            DataType::Delay => {
                // Java: MPPUtility.getDuration(ByteArrayHelper.getShort(data, m_fixedDataOffset), TimeUnit.HOURS)
                Some(FieldValue::Duration(MPPUtility::get_duration_int(
                    checked_short(data, offset)?,
                    TimeUnit::Hours,
                )))
            }

            DataType::WorkUnits => {
                // Java: int variableRateUnitsValue = MPPUtility.getByte(data, m_fixedDataOffset);
                //       result = variableRateUnitsValue == 0 ? null : MPPUtility.getWorkTimeUnits(variableRateUnitsValue);
                let value = MPPUtility::get_byte(data, offset);
                let units = if value == 0 {
                    None
                } else {
                    Some(MPPUtility::get_work_time_units(value))
                };
                units.map(|units| FieldValue::Integer(units.value()))
            }

            DataType::Workgroup => {
                // Java: WorkGroup.getInstance(ByteArrayHelper.getShort(data, m_fixedDataOffset))
                let value = checked_short(data, offset)?;
                Some(FieldValue::Integer(WorkGroup::get_instance(value).value()))
            }

            DataType::RateUnits => {
                // Java: TimeUnit.getInstance(ByteArrayHelper.getShort(data, m_fixedDataOffset) - 1)
                let value = checked_short(data, offset)?;
                Some(FieldValue::Integer(
                    TimeUnit::get_instance(value - 1).value(),
                ))
            }

            DataType::EarnedValueMethod => {
                // Java: EarnedValueMethod.getInstance(ByteArrayHelper.getShort(data, m_fixedDataOffset))
                let value = checked_short(data, offset)?;
                Some(FieldValue::Integer(
                    EarnedValueMethod::get_instance(value).value(),
                ))
            }

            DataType::ResourceRequestType => {
                // Java: ResourceRequestType.getInstance(ByteArrayHelper.getShort(data, m_fixedDataOffset))
                let value = checked_short(data, offset)?;
                Some(FieldValue::Integer(
                    ResourceRequestType::get_instance(value).value(),
                ))
            }

            DataType::Guid => {
                // Java: MPPUtility.getGUID(data, m_fixedDataOffset) (Java returns null when all zeros/insufficient data)
                // The Rust side carries UUIDs as FieldValue::String (see "Representation of field values" in the file header).
                MPPUtility::get_guid(data, offset).map(|guid| FieldValue::String(guid.to_string()))
            }

            // Java: BINARY -- binary data in a fixed data block is not processed.
            DataType::Binary => None,

            // Java: default -- unsupported fixed data type.
            _ => None,
        }
    }

    /// Reads a field value from a variable data block.
    ///
    /// Corresponds to Java: `private Object readVarData(DataType dataType, Integer id, byte[][] fixedData, Var2Data varData)`
    fn read_var_data(
        &self,
        field_map: &FieldMap,
        data_type: DataType,
        id: Option<i32>,
        fixed_data: &[Option<&[u8]>],
        var_data: &Var2Data,
    ) -> Option<FieldValue> {
        let key = self.get_var_data_key();

        match data_type {
            DataType::Duration => {
                // Java: TimeUnit units = (TimeUnit) getFieldData(id, unitsType, fixedData, varData);
                //       if (units == null) { units = TimeUnit.HOURS; }
                let units = self
                    .units_type()
                    .and_then(|units_type| {
                        field_map.get_field_data(
                            id,
                            &MppFieldType::Builtin(units_type),
                            fixed_data,
                            var_data,
                        )
                    })
                    .and_then(|value| value.as_integer())
                    .map(TimeUnit::get_instance)
                    .unwrap_or(TimeUnit::Hours);

                self.get_custom_field_duration_value(field_map, var_data, id, key, units)
            }

            DataType::TimeUnits => {
                // Java: MPPUtility.getDurationTimeUnits(varData.getShort(id, key), getProjectProperties().getDefaultDurationUnits())
                let default_units = field_map.try_project_properties();
                let default_units = default_units
                    .as_deref()
                    .map(ProjectProperties::get_default_duration_units);
                let units = MPPUtility::get_duration_time_units_with_default(
                    var_data.get_short(id, key),
                    default_units,
                );
                Some(FieldValue::Integer(units.value()))
            }

            DataType::Currency => {
                // Java: double amount = varData.getDouble(id, key);
                //       amount = Math.abs(amount) < 0.1 ? 0 : amount;
                //       result = NumberHelper.getDouble(amount / 100);
                let amount = var_data.get_double(id, key);
                let amount = if amount.abs() < 0.1 { 0.0 } else { amount };
                Some(FieldValue::Double(NumberHelper::get_double_value(
                    amount / 100.0,
                )))
            }

            DataType::String => {
                // Java: m_stringVarDataReader.getValue(varData, id, m_varDataKey)
                field_map
                    .string_var_data_reader
                    .get_value(var_data, id, key)
            }

            DataType::Date => {
                // Java: m_timestampVarDataReader.getValue(varData, id, m_varDataKey)
                field_map
                    .timestamp_var_data_reader
                    .get_value(var_data, id, key)
            }

            DataType::Numeric => {
                // Java: m_doubleVarDataReader.getValue(varData, id, m_varDataKey)
                field_map.double_var_data_reader.get_value(var_data, id, key)
            }

            DataType::Integer => {
                // Java: Integer.valueOf(varData.getInt(id, m_varDataKey))
                Some(FieldValue::Integer(var_data.get_int(id, key)))
            }

            DataType::Work => {
                // Java comment: ignore the duration when the result is less than 1 minute
                let duration = var_data.get_double(id, key);
                let duration = if duration.abs() < 1000.0 {
                    0.0
                } else {
                    duration
                };
                Some(FieldValue::Duration(Duration::get_instance(
                    duration / 60000.0,
                    TimeUnit::Hours,
                )))
            }

            DataType::Notes => {
                // Java: String notes = varData.getString(id, m_varDataKey);
                //       result = notes == null ? null : new RtfNotes(notes);
                // The Rust FieldValue has no RtfNotes variant, so plain text is stored per this crate's convention
                // (`RtfNotes::to_string()` is the text with RTF markup removed).
                var_data.get_string(id, key).map(|notes| {
                    // The Java implementation of FixDeferFix directly does new String(byte[]), but the RTF
                    // converter ignores the trailing NUL; non-RTF notes in the samples also carry the newline of the same record
                    // which is removed as well, matching Java's output.
                    let had_nul = notes.ends_with('\0');
                    let mut notes = notes.trim_end_matches('\0').to_owned();
                    if had_nul && notes.ends_with('\n') {
                        notes.pop();
                    }
                    FieldValue::String(RtfNotes::new(Some(notes)).to_string())
                })
            }

            DataType::Delay => {
                // Java: MPPUtility.getDuration(varData.getShort(id, m_varDataKey), TimeUnit.HOURS)
                Some(FieldValue::Duration(MPPUtility::get_duration_int(
                    var_data.get_short(id, key),
                    TimeUnit::Hours,
                )))
            }

            DataType::WorkUnits => {
                // Java: int value = varData.getByte(id, m_varDataKey);
                //       result = value == 0 ? null : MPPUtility.getWorkTimeUnits(value);
                let value = var_data.get_byte(id, key);
                let units = if value == 0 {
                    None
                } else {
                    Some(MPPUtility::get_work_time_units(value))
                };
                units.map(|units| FieldValue::Integer(units.value()))
            }

            DataType::RateUnits => {
                // Java: TimeUnit.getInstance(varData.getShort(id, m_varDataKey) - 1)
                Some(FieldValue::Integer(
                    TimeUnit::get_instance(var_data.get_short(id, key) - 1).value(),
                ))
            }

            DataType::EarnedValueMethod => {
                // Java: EarnedValueMethod.getInstance(varData.getShort(id, m_varDataKey))
                Some(FieldValue::Integer(
                    EarnedValueMethod::get_instance(var_data.get_short(id, key)).value(),
                ))
            }

            DataType::ResourceRequestType => {
                // Java: ResourceRequestType.getInstance(varData.getShort(id, m_varDataKey))
                Some(FieldValue::Integer(
                    ResourceRequestType::get_instance(var_data.get_short(id, key)).value(),
                ))
            }

            DataType::Accrue => {
                // Java: AccrueType.getInstance(varData.getShort(id, m_varDataKey)) (may be null)
                AccrueType::get_instance(var_data.get_short(id, key))
                    .map(|accrue_type| FieldValue::Integer(accrue_type.value()))
            }

            DataType::Percentage | DataType::Short => {
                // Java: Integer.valueOf(varData.getShort(id, m_varDataKey))
                Some(FieldValue::Integer(var_data.get_short(id, key)))
            }

            DataType::Boolean => {
                // Java: Boolean.valueOf(varData.getShort(id, m_varDataKey) != 0)
                Some(FieldValue::Boolean(var_data.get_short(id, key) != 0))
            }

            DataType::Workgroup => {
                // Java: WorkGroup.getInstance(varData.getShort(id, m_varDataKey))
                Some(FieldValue::Integer(
                    WorkGroup::get_instance(var_data.get_short(id, key)).value(),
                ))
            }

            DataType::Guid => {
                // Java: MPPUtility.getGUID(varData.getByteArray(id, m_varDataKey), 0)
                // The Rust side carries UUIDs as FieldValue::String (see "Representation of field values" in the file header).
                let data = var_data.get_byte_array(id, key)?;
                MPPUtility::get_guid(&data, 0).map(|guid| FieldValue::String(guid.to_string()))
            }

            DataType::BookingType => {
                // Java: BookingType.getInstance(varData.getShort(id, m_varDataKey))
                Some(FieldValue::Integer(
                    BookingType::get_instance(var_data.get_short(id, key)).value(),
                ))
            }

            // Java: BINARY —— result = varData.getByteArray(id, m_varDataKey);
            DataType::Binary => var_data.get_byte_array(id, key).map(FieldValue::Bytes),

            // Java: default -- unsupported variable data type.
            _ => None,
        }
    }

    /// Gets the duration value of a custom field.
    ///
    /// Corresponds to Java: `private Object getCustomFieldDurationValue(Var2Data varData, Integer id, Integer type, TimeUnit units)`
    fn get_custom_field_duration_value(
        &self,
        field_map: &FieldMap,
        var_data: &Var2Data,
        id: Option<i32>,
        type_: Option<i32>,
        units: TimeUnit,
    ) -> Option<FieldValue> {
        let data = var_data.get_byte_array(id, type_)?;

        if data.len() == 512 {
            // Java: result = MPPUtility.getUnicodeString(data, 0);
            Some(FieldValue::String(MPPUtility::get_unicode_string(
                &data, 0,
            )))
        } else if data.len() >= 4 {
            // Java: int duration = ByteArrayHelper.getInt(data, 0);
            //       result = MPPUtility.getAdjustedDuration(getProjectProperties(), duration, units);
            field_map
                .adjusted_duration(checked_int(&data, 0)?, units)
                .map(FieldValue::Duration)
        } else {
            None
        }
    }

    /// Gets the field type.
    ///
    /// Corresponds to Java: `public FieldType getType()`
    pub fn get_type(&self) -> &MppFieldType {
        &self.type_
    }

    /// Gets the fixed data block index of this field.
    ///
    /// Corresponds to Java: `public int getFixedDataBlockIndex()`
    pub fn get_fixed_data_block_index(&self) -> i32 {
        self.fixed_data_block_index
    }

    /// Gets the offset of this field within the fixed data block.
    ///
    /// Corresponds to Java: `public int getFixedDataOffset()`
    pub fn get_fixed_data_offset(&self) -> i32 {
        self.fixed_data_offset
    }

    /// Gets the variable data key of this field.
    ///
    /// Corresponds to Java: `public Integer getVarDataKey()`
    pub fn get_var_data_key(&self) -> Option<i32> {
        Some(self.var_data_key)
    }

    /// Gets the data block this field is in.
    ///
    /// Corresponds to Java: `public FieldLocation getFieldLocation()`
    pub fn get_field_location(&self) -> FieldLocation {
        self.location
    }

    /// Compares two field items.
    ///
    /// Corresponds to Java: `@Override public int compareTo(FieldItem item)`
    ///
    /// Consistent with Java: first compare the location, then compare the block index / offset / variable data key by location,
    /// the return value is Java's "subtraction" result (may be negative, zero, or positive).
    pub fn compare_to(&self, item: &FieldItem) -> i32 {
        let result = compare_ord(self.location, item.location);
        if result != 0 {
            return result;
        }

        match self.location {
            FieldLocation::FixedData => {
                // Java: m_fixedDataBlockIndex - item.m_fixedDataBlockIndex
                let result = self.fixed_data_block_index.wrapping_sub(item.fixed_data_block_index);
                if result != 0 {
                    return result;
                }
                // Java: m_fixedDataOffset - item.m_fixedDataOffset
                self.fixed_data_offset.wrapping_sub(item.fixed_data_offset)
            }
            FieldLocation::VarData => {
                // Java: m_varDataKey.intValue() - item.m_varDataKey.intValue()
                self.var_data_key.wrapping_sub(item.var_data_key)
            }
            // Java: default -- no further comparison for the remaining locations.
            FieldLocation::MetaData | FieldLocation::Unknown => 0,
        }
    }

    /// Gets the "companion units field of a duration field".
    ///
    /// Corresponds to Java: `m_type.getUnitsType()`
    ///
    /// For a custom field Java always returns `null` (`UserDefinedField.getUnitsType()`); the Rust side returns `None`.
    fn units_type(&self) -> Option<FieldKey> {
        match &self.type_ {
            MppFieldType::Builtin(key) => key.get_units_type(),
            MppFieldType::Custom(_) => None,
        }
    }
}

impl Display for FieldItem {
    /// Corresponds to Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[FieldItem type={}.{} location={}",
            field_type_class_name(self.type_.get_field_type_class()),
            self.type_,
            self.location
        )?;

        match self.location {
            FieldLocation::FixedData => {
                write!(
                    f,
                    " fixedDataBlockIndex={} fixedDataBlockOffset={}",
                    self.fixed_data_block_index, self.fixed_data_offset
                )?;
            }
            FieldLocation::VarData => {
                write!(f, " varDataKey={}", self.var_data_key)?;
            }
            FieldLocation::MetaData => {
                // Java: Long.toHexString(m_mask)
                write!(f, " mask={:x} block={}", self.mask, self.meta_block)?;
            }
            FieldLocation::Unknown => {}
        }

        f.write_str("]")
    }
}

impl PartialEq for FieldItem {
    /// Rust-specific: Java's `FieldItem` does not override `equals` (it compares by identity),
    /// so this implements structural equality by "field type + location information" (see the differences table in the file header).
    fn eq(&self, other: &Self) -> bool {
        self.type_.to_key() == other.type_.to_key()
            && self.location == other.location
            && self.fixed_data_block_index == other.fixed_data_block_index
            && self.fixed_data_offset == other.fixed_data_offset
            && self.var_data_key == other.var_data_key
            && self.mask == other.mask
            && self.meta_block == other.meta_block
    }
}

impl Eq for FieldItem {}

impl std::hash::Hash for FieldItem {
    /// Rust-specific: consistent with [`FieldItem::eq`] (Java's `toString` / `compareTo` do not require hashing).
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.type_.to_key().hash(state);
        self.location.hash(state);
        self.fixed_data_block_index.hash(state);
        self.fixed_data_offset.hash(state);
        self.var_data_key.hash(state);
        self.mask.hash(state);
        self.meta_block.hash(state);
    }
}

impl PartialOrd for FieldItem {
    /// Corresponds to Java: the result of `Comparable<FieldItem>` comparison.
    ///
    /// Note: as in Java, this ordering is **not** consistent with [`FieldItem::eq`] (two different fields can have
    /// the same location), so `Ord` is used only for the `Display` sorting of [`FieldMap`].
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FieldItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.compare_to(other).cmp(&0)
    }
}

/// Converts Java's "int subtraction comparison" result into 0 / -1 / 1.
fn compare_ord<T: Ord>(left: T, right: T) -> i32 {
    match left.cmp(&right) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// The "field <-> data block location" mapping in an MPP file.
///
/// Corresponds to Java: `abstract class FieldMap`
pub struct FieldMap {
    /// Project file (used to resolve custom fields and project properties).
    ///
    /// Corresponds to Java field: `protected final ProjectFile m_file;`
    file: Rc<RefCell<ProjectFile>>,

    /// Variable data reader for string fields.
    ///
    /// Corresponds to Java field: `final VarDataFieldReader m_stringVarDataReader;`
    string_var_data_reader: StringVarDataFieldReader,

    /// Variable data reader for double fields.
    ///
    /// Corresponds to Java field: `final VarDataFieldReader m_doubleVarDataReader;`
    double_var_data_reader: DoubleVarDataFieldReader,

    /// Variable data reader for timestamp fields.
    ///
    /// Corresponds to Java field: `final VarDataFieldReader m_timestampVarDataReader;`
    timestamp_var_data_reader: TimestampVarDataFieldReader,

    /// Field mapping table.
    ///
    /// Corresponds to Java field: `private final Map<FieldType, FieldItem> m_map = new HashMap<>();`
    map: HashMap<MppFieldKey, FieldItem>,

    /// Maximum offset of each fixed data block.
    ///
    /// Corresponds to Java field: `private final int[] m_maxFixedDataSize = new int[MAX_FIXED_DATA_BLOCKS];`
    max_fixed_data_size: [i32; MAX_FIXED_DATA_BLOCKS],

    /// Whether to output debug information.
    ///
    /// Corresponds to Java field: `private boolean m_debug;`
    debug: bool,

    /// Whether to use the low 16 bits of the field type value as the variable data key.
    ///
    /// Corresponds to Java: the override result of `protected abstract boolean useTypeAsVarDataKey()`
    /// (`FieldMap9` / `FieldMap12` are `false`, `FieldMap14` is `true`).
    use_type_as_var_data_key: bool,

    /// Default layout data for task fields (used when the field layout table is missing from `Props`).
    ///
    /// Corresponds to Java: `protected abstract FieldItem[] getDefaultTaskData()`
    default_task_data: Vec<FieldItem>,

    /// Default layout data for resource fields.
    ///
    /// Corresponds to Java: `protected abstract FieldItem[] getDefaultResourceData()`
    default_resource_data: Vec<FieldItem>,

    /// Default layout data for assignment fields.
    ///
    /// Corresponds to Java: `protected abstract FieldItem[] getDefaultAssignmentData()`
    default_assignment_data: Vec<FieldItem>,

    /// Default layout data for relation fields.
    ///
    /// Corresponds to Java: `protected abstract FieldItem[] getDefaultRelationData()`
    default_relation_data: Vec<FieldItem>,
}

impl FieldMap {
    /// Constructor.
    ///
    /// Corresponds to Java: `public FieldMap(ProjectFile file)`
    ///
    /// Java stores a reference to `file.getProjectProperties()` here; on the Rust side `ProjectProperties`
    /// is held internally by `ProjectFile`, so no stable reference can be taken; therefore each read borrows it via
    /// [`FieldMap::try_project_properties`].
    pub fn new(file: &Rc<RefCell<ProjectFile>>) -> Self {
        // Java: m_customFields = file.getCustomFields();
        // When the file is mutably borrowed (which does not happen on the normal call path), degrade gracefully to a standalone container to avoid a panic.
        let custom_fields = match file.try_borrow() {
            Ok(file) => file.get_custom_fields(),
            Err(_) => CustomFieldContainer::new(),
        };

        // Java: the three lines such as m_stringVarDataReader = new StringVarDataFieldReader(m_customFields);
        let string_var_data_reader = StringVarDataFieldReader::new(Rc::clone(&custom_fields));
        let double_var_data_reader = DoubleVarDataFieldReader::new(Rc::clone(&custom_fields));
        let timestamp_var_data_reader = TimestampVarDataFieldReader::new(custom_fields);

        FieldMap {
            file: Rc::clone(file),
            string_var_data_reader,
            double_var_data_reader,
            timestamp_var_data_reader,
            map: HashMap::new(),
            max_fixed_data_size: [0; MAX_FIXED_DATA_BLOCKS],
            debug: false,
            use_type_as_var_data_key: false,
            default_task_data: Vec::new(),
            default_resource_data: Vec::new(),
            default_assignment_data: Vec::new(),
            default_relation_data: Vec::new(),
        }
    }

    /// Enables / disables debug output.
    ///
    /// Corresponds to Java: `public void setDebug(boolean value)`
    pub fn set_debug(&mut self, value: bool) {
        self.debug = value;
    }

    /// Gets all field types in this mapping table.
    ///
    /// Corresponds to Java: `public Set<FieldType> getFieldTypes()`
    ///
    /// Java returns `m_map.keySet()`; Rust's key cannot reproduce the custom field handle, so it returns each item's
    /// `MppFieldType` copy (equivalent in content to the key set).
    pub fn get_field_types(&self) -> Vec<MppFieldType> {
        self.map
            .values()
            .map(|item| item.get_type().clone())
            .collect()
    }

    /// Builds the field mapping from a block of raw data.
    ///
    /// Corresponds to Java: `private void createFieldMap(byte[] data)`
    fn create_field_map(&mut self, data: &[u8]) {
        let mut index = 0usize;
        let mut last_data_block_offset = 0i32;
        let mut data_block_index = 0i32;

        // Java: while (index < data.length)
        // The Rust side adds a length check: when there is less than one record, Java throws an out-of-bounds exception, whereas Rust ends parsing directly.
        while index + FIELD_MAP_RECORD_SIZE <= data.len() {
            // Java: long mask = ByteArrayHelper.getInt(data, index);
            let mask = i64::from(checked_int(data, index).unwrap_or(0));

            // Java: int dataBlockOffset = ByteArrayHelper.getShort(data, index + 4);
            let data_block_offset = checked_short(data, index + 4).unwrap_or(0);

            // Java: int metaFlags = MPPUtility.getByte(data, index + 8); (unused)

            // Java: int typeValue = ByteArrayHelper.getInt(data, index + 12);
            let type_value = checked_int(data, index + 12).unwrap_or(0);

            // Java: FieldType type = getFieldType(typeValue);
            let type_ = self.get_field_type(type_value);

            // Java: int category = ByteArrayHelper.getShort(data, index + 20);
            let category = checked_short(data, index + 20).unwrap_or(0);

            // Java: int sizeInBytes = MPPUtility.getShort(data, index + 22); (unused)
            // Java: int metaIndex = MPPUtility.getInt(data, index + 24); (unused)

            //
            // Categories
            //
            // 02 - Short values [RATE_UNITS, WORKGROUP, ACCRUE, TIME_UNITS, PRIORITY, TASK_TYPE, CONSTRAINT, ACCRUE, PERCENTAGE, SHORT, WORK_UNITS]  - BOOKING_TYPE, EARNED_VALUE_METHOD, DELIVERABLE_TYPE, RESOURCE_REQUEST_TYPE - we have as string in MPXJ????
            // 03 - Int values [DURATION, INTEGER] - Recalc outline codes as Boolean?
            // 05 - Rate, Number [RATE, NUMERIC]
            // 08 - String (and some durations!!!) [STRING, DURATION]
            // 0B - Boolean (meta block 0?) - [BOOLEAN]
            // 13 - Date - [DATE]
            // 48 - GUID - [GUID]
            // 64 - Boolean (meta block 1?)- [BOOLEAN]
            // 65 - Work, Currency [WORK, CURRENCY]
            // 66 - Units [UNITS]
            // 1D - Raw bytes [BINARY, ASCII_STRING] - Exception: outline code indexes, they are integers, but stored as part of a binary block

            // Java: if (useTypeAsVarDataKey()) { varDataKey = typeValue & 0x0000FFFF; }
            //       else { varDataKey = MPPUtility.getByte(data, index + 6); }
            let var_data_key = if self.use_type_as_var_data_key {
                type_value & 0x0000_FFFF
            } else {
                MPPUtility::get_byte(data, index + 6)
            };

            let (location, meta_block) = match category {
                0x0B => (FieldLocation::MetaData, 0),
                0x64 => (FieldLocation::MetaData, 1),
                _ => {
                    if data_block_offset != NO_FIXED_DATA_OFFSET {
                        if data_block_offset < last_data_block_offset {
                            // Java: ++dataBlockIndex
                            data_block_index = data_block_index.wrapping_add(1);
                        }
                        last_data_block_offset = data_block_offset;

                        // Java: int typeSize = getFixedDataFieldSize(type);
                        //       if (dataBlockOffset + typeSize > m_maxFixedDataSize[dataBlockIndex]) { ... }
                        if let Some(type_) = &type_ {
                            let type_size = self.get_fixed_data_field_size(type_);
                            if let Some(max_size) = usize::try_from(data_block_index)
                                .ok()
                                .and_then(|index| self.max_fixed_data_size.get_mut(index))
                            {
                                if data_block_offset + type_size > *max_size {
                                    *max_size = data_block_offset + type_size;
                                }
                            }
                        }
                        (FieldLocation::FixedData, 0)
                    } else if var_data_key != 0 {
                        (FieldLocation::VarData, 0)
                    } else {
                        (FieldLocation::Unknown, 0)
                    }
                }
            };

            // On the Java side an unknown field number gets a placeholder FieldType and is written into m_map; Rust has no placeholder carrier,
            // so the record is skipped (see the differences table in the file header).
            if let Some(type_) = type_ {
                let item = FieldItem::new(
                    type_,
                    location,
                    data_block_index,
                    data_block_offset,
                    var_data_key,
                    mask,
                    meta_block,
                );

                if self.debug {
                    // Java: System.out.println(ByteArrayHelper.hexdump(data, index, 28, false) + " " + item
                    //       + " mpxjDataType=" + item.getType().getDataType() + " index=" + index);
                    println!(
                        "{} {} mpxjDataType={:?} index={}",
                        ByteArrayHelper::hexdump(data, index, FIELD_MAP_RECORD_SIZE, false),
                        item,
                        item.get_type().get_data_type(),
                        index
                    );
                }

                // Java: m_map.put(type, item);
                let key = item.get_type().to_key();
                self.map.insert(key, item);
            }

            index += FIELD_MAP_RECORD_SIZE;
        }
    }

    /// Whether to use the low 16 bits of the field type value as the variable data key.
    ///
    /// Corresponds to Java: `protected abstract boolean useTypeAsVarDataKey()`
    ///
    /// On the Rust side this is configured by [`FieldMap::set_use_type_as_var_data_key`] (default `false`, i.e.
    /// the behavior of `FieldMap9` / `FieldMap12`).
    pub fn use_type_as_var_data_key(&self) -> bool {
        self.use_type_as_var_data_key
    }

    /// Sets "whether to use the field type value as the variable data key".
    ///
    /// Rust-specific: replaces the Java subclass override of `useTypeAsVarDataKey()`.
    pub fn set_use_type_as_var_data_key(&mut self, value: bool) {
        self.use_type_as_var_data_key = value;
    }

    /// Gets the default layout data for task fields.
    ///
    /// Corresponds to Java: `protected abstract FieldItem[] getDefaultTaskData()`
    pub fn get_default_task_data(&self) -> &[FieldItem] {
        &self.default_task_data
    }

    /// Gets the default layout data for resource fields.
    ///
    /// Corresponds to Java: `protected abstract FieldItem[] getDefaultResourceData()`
    pub fn get_default_resource_data(&self) -> &[FieldItem] {
        &self.default_resource_data
    }

    /// Gets the default layout data for assignment fields.
    ///
    /// Corresponds to Java: `protected abstract FieldItem[] getDefaultAssignmentData()`
    pub fn get_default_assignment_data(&self) -> &[FieldItem] {
        &self.default_assignment_data
    }

    /// Gets the default layout data for relation fields.
    ///
    /// Corresponds to Java: `protected abstract FieldItem[] getDefaultRelationData()`
    pub fn get_default_relation_data(&self) -> &[FieldItem] {
        &self.default_relation_data
    }

    /// Sets the default layout data for task fields.
    ///
    /// Rust-specific: replaces the Java subclass override of `getDefaultTaskData()`
    /// (corresponds to the table of item-by-item `new FieldItem(...)` in `FieldMap9` / `FieldMap12` / `FieldMap14`).
    pub fn set_default_task_data(&mut self, value: Vec<FieldItem>) {
        self.default_task_data = value;
    }

    /// Sets the default layout data for resource fields.
    ///
    /// Rust-specific: replaces the Java subclass override of `getDefaultResourceData()`.
    pub fn set_default_resource_data(&mut self, value: Vec<FieldItem>) {
        self.default_resource_data = value;
    }

    /// Sets the default layout data for assignment fields.
    ///
    /// Rust-specific: replaces the Java subclass override of `getDefaultAssignmentData()`.
    pub fn set_default_assignment_data(&mut self, value: Vec<FieldItem>) {
        self.default_assignment_data = value;
    }

    /// Sets the default layout data for relation fields.
    ///
    /// Rust-specific: replaces the Java subclass override of `getDefaultRelationData()`.
    pub fn set_default_relation_data(&mut self, value: Vec<FieldItem>) {
        self.default_relation_data = value;
    }

    /// Derives the field type from the field number.
    ///
    /// Corresponds to Java: `protected FieldType getFieldType(int fieldID)`
    ///
    /// Java returns a placeholder `FieldType` (`FieldTypeClass.UNKNOWN`) when it cannot recognize the field number; the Rust side's
    /// corresponding `FieldTypeHelper` returns `None` (see the differences table in the file header).
    pub fn get_field_type(&self, field_id: i32) -> Option<MppFieldType> {
        // Java: return FieldTypeHelper.getInstance(m_file, fieldID);
        FieldTypeHelper::get_instance(&self.file, field_id)
    }

    /// Builds the task field mapping.
    ///
    /// Corresponds to Java: `public void createTaskFieldMap(Props props)`
    pub fn create_task_field_map(&mut self, props: &Props) {
        // Java: byte[] fieldMapData = null; for (PropsKey key : TASK_KEYS) { ... if (fieldMapData != null) break; }
        match find_props_data(props, &TASK_KEYS) {
            None => {
                // Java: populateDefaultData(getDefaultTaskData());
                let default_data = std::mem::take(&mut self.default_task_data);
                self.populate_default_data(&default_data);
                self.default_task_data = default_data;
            }
            Some(field_map_data) => self.create_field_map(field_map_data),
        }
    }

    /// Builds the relation field mapping.
    ///
    /// Corresponds to Java: `public void createRelationFieldMap(Props props)`
    pub fn create_relation_field_map(&mut self, props: &Props) {
        match find_props_data(props, &RELATION_KEYS) {
            None => {
                let default_data = std::mem::take(&mut self.default_relation_data);
                self.populate_default_data(&default_data);
                self.default_relation_data = default_data;
            }
            Some(field_map_data) => self.create_field_map(field_map_data),
        }
    }

    /// Builds the enterprise custom field mapping.
    ///
    /// Corresponds to Java: `public void createEnterpriseCustomFieldMap(Props props, FieldTypeClass fieldTypeClass)`
    pub fn create_enterprise_custom_field_map(
        &mut self,
        props: &Props,
        field_type_class: FieldTypeClass,
    ) {
        let Some(field_map_data) = find_props_data(props, &ENTERPRISE_CUSTOM_KEYS) else {
            // Java: do nothing when fieldMapData == null.
            return;
        };

        // Java: int index = 4; while (index < fieldMapData.length) { ... index += 4; }
        let mut index = 4usize;
        while index + 4 <= field_map_data.len() {
            // Java: int typeValue = ByteArrayHelper.getInt(fieldMapData, index);
            let type_value = checked_int(field_map_data, index).unwrap_or(0);

            // Java: FieldType type = getFieldType(typeValue);
            //       if (type != null && type.getFieldTypeClass() == fieldTypeClass && type instanceof UserDefinedField)
            if let Some(type_) = self.get_field_type(type_value) {
                if type_.get_field_type_class() == field_type_class && type_.is_custom() {
                    // Java: int varDataKey = (typeValue & 0xFFFF);
                    let var_data_key = type_value & 0xFFFF;
                    let item = FieldItem::new(
                        type_,
                        FieldLocation::VarData,
                        0,
                        0,
                        var_data_key,
                        0,
                        0,
                    );
                    // Java: m_map.put(type, item);
                    let key = item.get_type().to_key();
                    self.map.insert(key, item);
                }
            }

            index += 4;
        }
    }

    /// Builds the resource field mapping.
    ///
    /// Corresponds to Java: `public void createResourceFieldMap(Props props)`
    pub fn create_resource_field_map(&mut self, props: &Props) {
        match find_props_data(props, &RESOURCE_KEYS) {
            None => {
                let default_data = std::mem::take(&mut self.default_resource_data);
                self.populate_default_data(&default_data);
                self.default_resource_data = default_data;
            }
            Some(field_map_data) => self.create_field_map(field_map_data),
        }
    }

    /// Builds the assignment field mapping.
    ///
    /// Corresponds to Java: `public void createAssignmentFieldMap(Props props)`
    pub fn create_assignment_field_map(&mut self, props: &Props) {
        match find_props_data(props, &ASSIGNMENT_KEYS) {
            None => {
                let default_data = std::mem::take(&mut self.default_assignment_data);
                self.populate_default_data(&default_data);
                self.default_assignment_data = default_data;
            }
            Some(field_map_data) => self.create_field_map(field_map_data),
        }
    }

    /// Builds the variable data field mapping from an explicit "field type -> variable data key" list.
    ///
    /// Corresponds to Java: `public void createVarDataFieldMap(Map<FieldType, Integer> map)`
    ///
    /// Java uses a `Map` for its `put` semantics (a duplicate key is overwritten by the later entry); `MppFieldType` cannot be used as
    /// a `HashMap` key, so the Rust side accepts a slice of key-value pairs and writes them in order, with equivalent semantics.
    /// Currently used for the special case in MPP14 where "resource assignment custom fields are represented with different key values".
    pub fn create_var_data_field_map(&mut self, map: &[(MppFieldType, i32)]) {
        for (type_, var_data_key) in map {
            let item = FieldItem::new(
                type_.clone(),
                FieldLocation::VarData,
                0,
                0,
                *var_data_key,
                0,
                0,
            );
            // Java: m_map.put(entry.getKey(), item);
            let key = item.get_type().to_key();
            self.map.insert(key, item);
        }
    }

    /// Populates the mapping table with the default layout data.
    ///
    /// Corresponds to Java: `private void populateDefaultData(FieldItem[] defaultData)`
    fn populate_default_data(&mut self, default_data: &[FieldItem]) {
        for item in default_data {
            // Java: m_map.put(item.getType(), item);
            self.map.insert(item.get_type().to_key(), item.clone());
        }
    }

    /// Writes a block of raw data into the field container according to the field layout.
    ///
    /// Corresponds to Java: `public void populateContainer(FieldTypeClass fieldTypeClass, FieldContainer container, Integer id, byte[][] fixedData, Var2Data varData)`
    ///
    /// In Java both the `fixedData` array and its elements can be `null`: a `null` array is represented by an empty slice,
    /// and a `null` element is represented by `None`.
    pub fn populate_container(
        &self,
        field_type_class: FieldTypeClass,
        container: &mut dyn FieldContainer,
        id: Option<i32>,
        fixed_data: &[Option<&[u8]>],
        var_data: &Var2Data,
    ) {
        // Java: m_map.values().forEach(v -> populateItem(v, fieldTypeClass, container, id, fixedData, varData));
        for item in self.map.values() {
            self.populate_item(item, field_type_class, container, id, fixed_data, var_data);
        }
    }

    /// Writes a single field into the field container according to the field layout.
    ///
    /// Corresponds to Java: `public void populateField(FieldType type, FieldTypeClass fieldTypeClass, FieldContainer container, Integer id, byte[][] fixedData, Var2Data varData)`
    pub fn populate_field(
        &self,
        type_: &MppFieldType,
        field_type_class: FieldTypeClass,
        container: &mut dyn FieldContainer,
        id: Option<i32>,
        fixed_data: &[Option<&[u8]>],
        var_data: &Var2Data,
    ) {
        // Java: populateItem(m_map.get(type), fieldTypeClass, container, id, fixedData, varData);
        //       (when type is not in the table, Java passes null to populateItem and throws an NPE; the Rust side simply does not handle it)
        if let Some(item) = self.map.get(&type_.to_key()) {
            self.populate_item(item, field_type_class, container, id, fixed_data, var_data);
        }
    }

    /// Writes a single field item into the field container.
    ///
    /// Corresponds to Java: `private void populateItem(FieldItem item, FieldTypeClass fieldTypeClass, FieldContainer container, Integer id, byte[][] fixedData, Var2Data varData)`
    fn populate_item(
        &self,
        item: &FieldItem,
        field_type_class: FieldTypeClass,
        container: &mut dyn FieldContainer,
        id: Option<i32>,
        fixed_data: &[Option<&[u8]>],
        var_data: &Var2Data,
    ) {
        if item.get_type().get_field_type_class() != field_type_class {
            return;
        }

        let value = item.read(self, id, fixed_data, var_data);

        // Java: container.set(item.getType(), value);
        // Built-in fields use their `FieldKey`; custom fields are converted to `FieldKey::Custom`
        // (using the unique ID as the equivalence basis, consistent with Java's `UserDefinedField.equals`).
        container.set(item.get_type().to_field_key(), value);
    }

    /// Gets the maximum offset of the fixed data block.
    ///
    /// Corresponds to Java: `public int getMaxFixedDataSize(int blockIndex)`
    ///
    /// Java throws `ArrayIndexOutOfBoundsException` when out of bounds; the Rust side returns `None`.
    pub fn get_max_fixed_data_size(&self, block_index: i32) -> Option<i32> {
        usize::try_from(block_index)
            .ok()
            .and_then(|index| self.max_fixed_data_size.get(index))
            .copied()
    }

    /// Gets the fixed data offset of a field (returns `-1` when not in the table, consistent with Java).
    ///
    /// Corresponds to Java: `public int getFixedDataOffset(FieldType type)`
    pub fn get_fixed_data_offset(&self, type_: &MppFieldType) -> i32 {
        match self.map.get(&type_.to_key()) {
            Some(item) => item.get_fixed_data_offset(),
            None => -1,
        }
    }

    /// Gets the variable data key of a field (returns `None` when not in the table).
    ///
    /// Corresponds to Java: `public Integer getVarDataKey(FieldType type)`
    pub fn get_var_data_key(&self, type_: &MppFieldType) -> Option<i32> {
        self.map
            .get(&type_.to_key())
            .and_then(FieldItem::get_var_data_key)
    }

    /// Looks up the field type from the variable data key.
    ///
    /// Corresponds to Java: `public FieldType getFieldTypeFromVarDataKey(Integer key)`
    ///
    /// A Java comment states that this method is for diagnostic use only (it traverses the whole table and is not efficient); the same applies on the Rust side,
    /// and the traversal order is as nondeterministic as Java's `HashMap`.
    pub fn get_field_type_from_var_data_key(&self, key: Option<i32>) -> Option<MppFieldType> {
        for item in self.map.values() {
            if item.get_field_location() == FieldLocation::VarData
                && item.get_var_data_key() == key
            {
                return Some(item.get_type().clone());
            }
        }

        None
    }

    /// Gets the data block location of a field (returns `None` when not in the table).
    ///
    /// Corresponds to Java: `public FieldLocation getFieldLocation(FieldType type)`
    pub fn get_field_location(&self, type_: &MppFieldType) -> Option<FieldLocation> {
        self.map
            .get(&type_.to_key())
            .map(FieldItem::get_field_location)
    }

    /// Reads the value of a single field.
    ///
    /// Corresponds to Java: `protected Object getFieldData(Integer id, FieldType type, byte[][] fixedData, Var2Data varData)`
    pub fn get_field_data(
        &self,
        id: Option<i32>,
        type_: &MppFieldType,
        fixed_data: &[Option<&[u8]>],
        var_data: &Var2Data,
    ) -> Option<FieldValue> {
        self.map
            .get(&type_.to_key())
            .and_then(|item| item.read(self, id, fixed_data, var_data))
    }

    /// Gets the project properties.
    ///
    /// Corresponds to Java: `protected ProjectProperties getProjectProperties()`
    ///
    /// Java stores a reference to the properties object in the constructor; on the Rust side the properties object is held by `ProjectFile`,
    /// so it is borrowed via `RefCell::try_borrow` each time; when the file is mutably borrowed it returns `None`
    /// (this situation does not arise in Java; see "Known functional gaps" in the file header).
    pub fn get_project_properties(&self) -> Option<Ref<'_, ProjectProperties>> {
        // Java: return m_properties;
        self.try_project_properties()
    }

    /// Clears the field mapping.
    ///
    /// Corresponds to Java: `public void clear()`
    pub fn clear(&mut self) {
        // Java: m_map.clear(); Arrays.fill(m_maxFixedDataSize, 0);
        self.map.clear();
        self.max_fixed_data_size = [0; MAX_FIXED_DATA_BLOCKS];
    }

    /// Gets the number of bytes a field occupies in the fixed data block.
    ///
    /// Corresponds to Java: `private int getFixedDataFieldSize(FieldType type)`
    ///
    /// Java's `type.getDataType()` may be `null` (placeholder type); in that case it returns 0.
    fn get_fixed_data_field_size(&self, type_: &MppFieldType) -> i32 {
        match type_.get_data_type() {
            DataType::Date | DataType::Integer | DataType::Duration => 4,
            DataType::TimeUnits
            | DataType::Constraint
            | DataType::Priority
            | DataType::Percentage
            | DataType::TaskType
            | DataType::Accrue
            | DataType::Short
            | DataType::Boolean
            | DataType::Delay
            | DataType::Workgroup
            | DataType::RateUnits
            | DataType::EarnedValueMethod
            | DataType::ResourceRequestType => 2,
            DataType::Currency | DataType::Units | DataType::Rate | DataType::Work => 8,
            DataType::WorkUnits => 1,
            DataType::Guid => 16,
            _ => 0,
        }
    }

    /// Gets the project's default duration units.
    ///
    /// Corresponds to Java: `getProjectProperties().getDefaultDurationUnits()`
    ///
    /// When the project properties cannot be borrowed, falls back to the `ProjectProperties` default value `DAYS`.
    fn default_duration_units(&self) -> TimeUnit {
        self.try_project_properties()
            .as_deref()
            .map_or(TimeUnit::Days, ProjectProperties::get_default_duration_units)
    }

    /// Converts the duration in the file into a user-visible duration according to the project properties.
    ///
    /// Corresponds to Java: `MPPUtility.getAdjustedDuration(getProjectProperties(), duration, units)`
    ///
    /// Returns `None` when the project properties cannot be borrowed (the file is mutably borrowed).
    fn adjusted_duration(&self, duration: i32, units: TimeUnit) -> Option<Duration> {
        let properties = self.try_project_properties();
        MPPUtility::get_adjusted_duration(properties.as_deref()?, duration, units)
    }

    /// Borrows the project properties (Rust-specific: Java holds the reference directly).
    fn try_project_properties(&self) -> Option<Ref<'_, ProjectProperties>> {
        let file = self.file.try_borrow().ok()?;
        Some(Ref::map(file, |file| file.get_project_properties()))
    }
}

impl Display for FieldMap {
    /// Corresponds to Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Java: ArrayList<FieldItem> items = new ArrayList<>(m_map.values()); Collections.sort(items);
        let mut items: Vec<FieldItem> = self.map.values().cloned().collect();
        items.sort();

        f.write_str("[FieldMap\n")?;

        for (block, max_size) in self.max_fixed_data_size.iter().enumerate() {
            writeln!(f, " MaxFixedOffset (block {block})={max_size}")?;
        }

        for item in &items {
            writeln!(f, " {item}")?;
        }

        f.write_str("]\n")
    }
}

impl VarDataKeyResolver for FieldMap {
    /// Corresponds to Java: `FieldType getFieldTypeFromVarDataKey(Integer key)`
    /// (`AbstractVarMeta#toString(FieldMap)` uses it to translate the variable data key into a field name)
    ///
    /// The Rust side's [`VarDataKeyResolver`] is declared to return the field display name, so here it takes
    /// the display name of the `MppFieldType` from the result of [`FieldMap::get_field_type_from_var_data_key`].
    fn get_field_type_from_var_data_key(&self, key: i32) -> Option<String> {
        FieldMap::get_field_type_from_var_data_key(self, Some(key))
            .map(|type_| type_.get_name())
    }
}

/// Takes the first property data that exists, in order.
///
/// Corresponds to Java: in `create*FieldMap`, a loop of the form
/// `for (PropsKey key : KEYS) { fieldMapData = props.getByteArray(key); if (fieldMapData != null) break; }`
/// (a Rust-specific helper function shared by 5 call sites).
fn find_props_data<'a>(props: &'a Props, keys: &[PropsKey]) -> Option<&'a [u8]> {
    for key in keys {
        if let Some(data) = props.get_byte_array(*key) {
            return Some(data);
        }
    }

    None
}

/// Reads a two-byte short integer; returns `None` when there is insufficient data.
///
/// Java's `ByteArrayHelper.getShort` throws `ArrayIndexOutOfBoundsException` when out of bounds;
/// the Rust side uniformly returns `None` (all reads in this file where "Java throws an out-of-bounds exception" go through these two helper functions).
fn checked_short(data: &[u8], offset: usize) -> Option<i32> {
    if offset.checked_add(2)? > data.len() {
        return None;
    }

    Some(ByteArrayHelper::get_short(data, offset))
}

/// Reads a four-byte integer; returns `None` when there is insufficient data.
fn checked_int(data: &[u8], offset: usize) -> Option<i32> {
    if offset.checked_add(4)? > data.len() {
        return None;
    }

    Some(ByteArrayHelper::get_int(data, offset))
}

/// Converts the task type value in an MPP file into [`TaskType`].
///
/// Corresponds to Java: `org.mpxj.mpp.TaskTypeHelper.getInstance(int)`
///
/// (that utility class has not been ported to this crate, so it is inlined here; out-of-range values always fall back to `FIXED_WORK`.)
fn mpp_value_to_task_type(value: i32) -> TaskType {
    match value {
        0 => TaskType::FixedUnits,
        1 => TaskType::FixedDuration,
        _ => TaskType::FixedWork,
    }
}

/// Encodes [`TaskType`] as a field value.
///
/// Corresponds to Java: `org.mpxj.mpp.TaskTypeHelper.getValue(TaskType)`
///
/// `TaskType` has no int value, so this crate uniformly encodes it by the index into `TaskType::ALL`
/// (consistent with `crate::task::task_type_to_value`), so `Task::get_type()` can recover it.
fn task_type_to_value(value: TaskType) -> i32 {
    TaskType::ALL
        .iter()
        .position(|item| *item == value)
        .map_or(0, |index| index as i32)
}

/// Gets the Java enum constant name of [`FieldTypeClass`] (used for `toString` output).
///
/// Rust's [`FieldTypeClass`] has no `Display`; its `Debug` name differs from the Java constant name only in case.
fn field_type_class_name(class: FieldTypeClass) -> &'static str {
    match class {
        FieldTypeClass::Task => "TASK",
        FieldTypeClass::Resource => "RESOURCE",
        FieldTypeClass::Assignment => "ASSIGNMENT",
        FieldTypeClass::Constraint => "CONSTRAINT",
        FieldTypeClass::Project => "PROJECT",
        FieldTypeClass::Unknown => "UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::mpp_resource_field::MPPResourceField;
    use crate::common::mpp_task_field::MPPTaskField;
    use crate::listener::field_listener::FieldListener;
    use crate::mpp::var_data_field_reader::test_support::var_data;
    use crate::task_field::TaskField;

    /// Task / resource field number prefixes (consistent with Java's `MPPTaskField.TASK_FIELD_BASE` /
    /// `MPPResourceField.RESOURCE_FIELD_BASE`, taken from the already-ported `MPP*Field` constants).
    const TASK_FIELD_BASE: i32 = MPPTaskField::TASK_FIELD_BASE;
    const RESOURCE_FIELD_BASE: i32 = MPPResourceField::RESOURCE_FIELD_BASE;

    /// Field layout table records for tests (28 bytes, with field locations consistent with Java's `createFieldMap`).
    fn record(
        mask: i32,
        data_block_offset: i32,
        var_data_key: u8,
        type_value: i32,
        category: i32,
    ) -> Vec<u8> {
        let mut data = vec![0u8; FIELD_MAP_RECORD_SIZE];
        data[0..4].copy_from_slice(&mask.to_le_bytes());
        data[4..6].copy_from_slice(&(data_block_offset as u16).to_le_bytes());
        data[6] = var_data_key;
        data[12..16].copy_from_slice(&type_value.to_le_bytes());
        data[20..22].copy_from_slice(&(category as u16).to_le_bytes());
        data
    }

    /// Constructs a built-in task field.
    fn task_field(field: TaskField) -> MppFieldType {
        MppFieldType::Builtin(FieldKey::Task(field))
    }

    /// Constructs a props containing the task field layout table (`TaskFieldMap`).
    ///
    /// Record contents (field numbers taken from Java's `MPPTaskField.FIELD_ARRAY`):
    /// 86 = UNIQUE_ID,29 = DURATION,32 = PERCENT_COMPLETE,14 = NAME,
    /// 16 = WBS,19 = CRITICAL,23 = ID.
    fn props_with_task_field_map() -> Props {
        let mut bytes = Vec::new();
        // Fixed data block 0: offset 0 (4 bytes) -> offset 4 (4 bytes) -> offset 8 (2 bytes)
        bytes.extend(record(0x0000_0001, 0, 0, TASK_FIELD_BASE | 86, 0x03));
        bytes.extend(record(0x0000_0002, 4, 0, TASK_FIELD_BASE | 29, 0x03));
        bytes.extend(record(0x0000_0004, 8, 0, TASK_FIELD_BASE | 32, 0x02));
        // Variable data: key 11
        bytes.extend(record(0, 65535, 11, TASK_FIELD_BASE | 14, 0x08));
        // Variable data block but key 0 -> location unknown
        bytes.extend(record(0, 65535, 0, TASK_FIELD_BASE | 16, 0x08));
        // Metadata block 0
        bytes.extend(record(0x0000_0010, 0, 0, TASK_FIELD_BASE | 19, 0x0B));
        // Offset wrap-around -> fixed data block 1, offset 0 (4 bytes)
        bytes.extend(record(0, 0, 0, TASK_FIELD_BASE | 23, 0x03));

        let mut props = Props::new();
        props.put(PropsKey::TaskFieldMap.get_value(), bytes);
        props
    }

    /// Fixed data block (8 bytes): unique ID = 7 at offset 0, percentage = 50 at offset 4.
    fn fixed_data() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&7i32.to_le_bytes());
        data.extend_from_slice(&50u16.to_le_bytes());
        data
    }

    /// UTF-16LE representation of "Hello" (NUL-terminated).
    fn utf16(text: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        bytes.extend_from_slice(&[0x00, 0x00]);
        bytes
    }

    /// Field container for tests (consistent with the test container in `crate::mpp::mpp_bit_flag`).
    #[derive(Default)]
    struct MockContainer {
        values: HashMap<FieldKey, FieldValue>,
    }

    impl FieldContainer for MockContainer {
        fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
            match value {
                Some(value) => self.values.insert(field, value),
                None => self.values.remove(&field),
            };
        }

        fn get(&self, field: FieldKey) -> Option<FieldValue> {
            self.values.get(&field).cloned()
        }

        fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
            self.values.get(&field)
        }

        fn add_field_listener(&mut self, _listener: Box<dyn FieldListener>) {}

        fn clear_field_listeners(&mut self) {}
    }

    /// Parses the field layout table: number of fields, location, offset, variable data key, and maximum offset of each block.
    #[test]
    fn create_task_field_map_reads_the_layout_table() {
        let file = ProjectFile::new();
        let mut field_map = FieldMap::new(&file);
        field_map.create_task_field_map(&props_with_task_field_map());

        // All 7 records hit known fields
        assert_eq!(field_map.get_field_types().len(), 7);

        assert_eq!(
            field_map.get_field_location(&task_field(TaskField::UniqueId)),
            Some(FieldLocation::FixedData)
        );
        assert_eq!(
            field_map.get_fixed_data_offset(&task_field(TaskField::UniqueId)),
            0
        );
        assert_eq!(
            field_map.get_fixed_data_offset(&task_field(TaskField::PercentComplete)),
            8
        );
        // Variable data fields keep the 65535 offset from Java
        assert_eq!(
            field_map.get_fixed_data_offset(&task_field(TaskField::Name)),
            65535
        );
        assert_eq!(
            field_map.get_field_location(&task_field(TaskField::Name)),
            Some(FieldLocation::VarData)
        );
        assert_eq!(
            field_map.get_var_data_key(&task_field(TaskField::Name)),
            Some(11)
        );

        // Variable data key 0 -> location unknown
        assert_eq!(
            field_map.get_field_location(&task_field(TaskField::Wbs)),
            Some(FieldLocation::Unknown)
        );
        // Metadata block
        assert_eq!(
            field_map.get_field_location(&task_field(TaskField::Critical)),
            Some(FieldLocation::MetaData)
        );

        // Offset wrap-around -> the second fixed data block
        let block_index = field_map
            .get_field_types()
            .into_iter()
            .find_map(|type_| match &type_ {
                MppFieldType::Builtin(FieldKey::Task(TaskField::Id)) => Some(()),
                _ => None,
            });
        assert!(block_index.is_some(), "ID field should be registered");

        // Block 0: maximum offset = 8 + 2 = 10; block 1: maximum offset = 0 + 4 = 4
        assert_eq!(field_map.get_max_fixed_data_size(0), Some(10));
        assert_eq!(field_map.get_max_fixed_data_size(1), Some(4));
        // Java throws when out of bounds; Rust returns None
        assert_eq!(field_map.get_max_fixed_data_size(2), None);
        assert_eq!(field_map.get_max_fixed_data_size(-1), None);

        // Look up the field from the variable data key
        assert_eq!(
            field_map.get_field_type_from_var_data_key(Some(11)).map(|t| t.to_key()),
            Some(MppFieldKey::Builtin(FieldKey::Task(TaskField::Name)))
        );
        assert!(field_map.get_field_type_from_var_data_key(Some(999)).is_none());

        // A field not in the table
        assert_eq!(field_map.get_field_location(&task_field(TaskField::Cost)), None);
        assert_eq!(field_map.get_var_data_key(&task_field(TaskField::Cost)), None);
        assert_eq!(field_map.get_fixed_data_offset(&task_field(TaskField::Cost)), -1);
    }

    /// Uses the default layout data when the field layout table is missing from `Props`.
    #[test]
    fn create_task_field_map_uses_default_data_when_props_are_missing() {
        let file = ProjectFile::new();
        let mut field_map = FieldMap::new(&file);
        field_map.set_default_task_data(vec![
            FieldItem::new(
                task_field(TaskField::UniqueId),
                FieldLocation::FixedData,
                0,
                0,
                0,
                0,
                0,
            ),
            FieldItem::new(
                task_field(TaskField::Name),
                FieldLocation::VarData,
                0,
                65535,
                11,
                0,
                0,
            ),
        ]);
        field_map.create_task_field_map(&Props::new());

        assert_eq!(field_map.get_field_types().len(), 2);
        assert_eq!(
            field_map.get_fixed_data_offset(&task_field(TaskField::UniqueId)),
            0
        );
        assert_eq!(
            field_map.get_var_data_key(&task_field(TaskField::Name)),
            Some(11)
        );
        // Java's populateDefaultData does not update m_maxFixedDataSize
        assert_eq!(field_map.get_max_fixed_data_size(0), Some(0));
    }

    /// Enterprise custom field table: only keeps custom fields matching the target entity type.
    #[test]
    fn create_enterprise_custom_field_map_filters_by_field_type_class() {
        let mut bytes = vec![0u8; 4];
        // Task enterprise custom field (the 0x8000 bit indicates a custom field)
        bytes.extend_from_slice(&(TASK_FIELD_BASE | 0x8001).to_le_bytes());
        // Resource enterprise custom field: entity type does not match
        bytes.extend_from_slice(&(RESOURCE_FIELD_BASE | 0x8001).to_le_bytes());

        let mut props = Props::new();
        props.put(PropsKey::EnterpriseCustomFieldMap.get_value(), bytes);

        let file = ProjectFile::new();
        let mut field_map = FieldMap::new(&file);
        field_map.create_enterprise_custom_field_map(&props, FieldTypeClass::Task);

        let types = field_map.get_field_types();
        assert_eq!(types.len(), 1);
        assert!(types[0].is_custom());
        assert_eq!(
            field_map.get_field_location(&types[0]),
            Some(FieldLocation::VarData)
        );
        assert_eq!(field_map.get_var_data_key(&types[0]), Some(0x8001));
    }

    /// Explicit "field type -> variable data key" mapping.
    #[test]
    fn create_var_data_field_map_registers_explicit_keys() {
        let file = ProjectFile::new();
        let mut field_map = FieldMap::new(&file);
        field_map.create_var_data_field_map(&[
            (task_field(TaskField::Name), 11),
            (task_field(TaskField::Duration), 31),
        ]);

        assert_eq!(field_map.get_field_types().len(), 2);
        assert_eq!(
            field_map.get_field_location(&task_field(TaskField::Name)),
            Some(FieldLocation::VarData)
        );
        assert_eq!(
            field_map.get_var_data_key(&task_field(TaskField::Duration)),
            Some(31)
        );
    }

    /// `populateContainer`: only writes fields whose field type matches and whose data exists.
    #[test]
    fn populate_container_populates_present_fields_only() {
        let mut bytes = Vec::new();
        bytes.extend(record(0, 0, 0, TASK_FIELD_BASE | 86, 0x03)); // UNIQUE_ID：fixed offset 0（Integer）
        bytes.extend(record(0, 4, 0, TASK_FIELD_BASE | 32, 0x02)); // PERCENT_COMPLETE：fixed offset 4（Percentage）
        bytes.extend(record(0, 65535, 11, TASK_FIELD_BASE | 14, 0x08)); // NAME：variable data key 11
        bytes.extend(record(0, 8, 0, TASK_FIELD_BASE | 23, 0x03)); // ID：fixed offset 8（data is only 8 bytes）

        let mut props = Props::new();
        props.put(PropsKey::TaskFieldMap.get_value(), bytes);

        let file = ProjectFile::new();
        let mut field_map = FieldMap::new(&file);
        field_map.create_task_field_map(&props);

        let fixed = fixed_data();
        let var = var_data(&[(1, 11, utf16("Hello"))]);
        let mut container = MockContainer::default();

        field_map.populate_container(
            FieldTypeClass::Task,
            &mut container,
            Some(1),
            &[Some(fixed.as_slice())],
            &var,
        );

        // Integer / percentage in the fixed data block
        assert_eq!(
            container.get(FieldKey::Task(TaskField::UniqueId)),
            Some(FieldValue::Integer(7))
        );
        assert_eq!(
            container.get(FieldKey::Task(TaskField::PercentComplete)),
            Some(FieldValue::Double(50.0))
        );
        // String in the variable data block
        assert_eq!(
            container.get(FieldKey::Task(TaskField::Name)),
            Some(FieldValue::String("Hello".to_owned()))
        );
        // The field is in the table but its data is missing: Java passes null (equivalent to removal); the Rust side does not write it
        assert_eq!(container.get(FieldKey::Task(TaskField::Id)), None);
        // The field is not in the table: populateContainer does not touch it
        assert_eq!(container.get(FieldKey::Task(TaskField::Cost)), None);
    }

    /// `populateContainer`: writes nothing when the field type does not match.
    #[test]
    fn populate_container_ignores_other_field_type_classes() {
        let file = ProjectFile::new();
        let mut field_map = FieldMap::new(&file);
        field_map.create_task_field_map(&props_with_task_field_map());

        let fixed = fixed_data();
        let var = var_data(&[(1, 11, utf16("Hello"))]);
        let mut container = MockContainer::default();

        field_map.populate_container(
            FieldTypeClass::Resource,
            &mut container,
            Some(1),
            &[Some(fixed.as_slice())],
            &var,
        );

        assert!(container.values.is_empty());
    }

    /// `populateField`: writes a single field; does not panic when the field is not in the table.
    #[test]
    fn populate_field_writes_a_single_field() {
        let file = ProjectFile::new();
        let mut field_map = FieldMap::new(&file);
        field_map.create_task_field_map(&props_with_task_field_map());

        let fixed = fixed_data();
        let var = var_data(&[]);
        let mut container = MockContainer::default();

        field_map.populate_field(
            &task_field(TaskField::UniqueId),
            FieldTypeClass::Task,
            &mut container,
            Some(1),
            &[Some(fixed.as_slice())],
            &var,
        );
        assert_eq!(
            container.get(FieldKey::Task(TaskField::UniqueId)),
            Some(FieldValue::Integer(7))
        );

        // A field not in the table: Java passes null to populateItem and throws an NPE; the Rust side does not handle it
        field_map.populate_field(
            &task_field(TaskField::Cost),
            FieldTypeClass::Task,
            &mut container,
            Some(1),
            &[Some(fixed.as_slice())],
            &var,
        );
        assert_eq!(container.get(FieldKey::Task(TaskField::Cost)), None);
    }

    /// `getFieldData`: neither the metadata block nor unknown fields yield a value.
    #[test]
    fn get_field_data_ignores_meta_data_and_missing_fields() {
        let file = ProjectFile::new();
        let mut field_map = FieldMap::new(&file);
        field_map.create_task_field_map(&props_with_task_field_map());

        let fixed = fixed_data();
        let empty: Vec<Option<&[u8]>> = Vec::new();
        let var = var_data(&[(1, 11, utf16("Hello"))]);

        // A boolean field in the metadata block (Java's read ignores META_DATA)
        assert_eq!(
            field_map.get_field_data(
                Some(1),
                &task_field(TaskField::Critical),
                &[Some(fixed.as_slice())],
                &var
            ),
            None
        );
        // The fixed data block does not exist (Java passes a null array and throws an NPE; Rust returns None)
        assert_eq!(
            field_map.get_field_data(
                Some(1),
                &task_field(TaskField::UniqueId),
                &empty,
                &var
            ),
            None
        );
        // The field is not in the table
        assert_eq!(
            field_map.get_field_data(
                Some(1),
                &task_field(TaskField::Cost),
                &[Some(fixed.as_slice())],
                &var
            ),
            None
        );
    }

    /// `clear` clears the mapping and the maximum offsets.
    #[test]
    fn clear_resets_the_map_and_offsets() {
        let file = ProjectFile::new();
        let mut field_map = FieldMap::new(&file);
        field_map.create_task_field_map(&props_with_task_field_map());
        assert!(!field_map.get_field_types().is_empty());

        field_map.clear();

        assert!(field_map.get_field_types().is_empty());
        assert_eq!(field_map.get_max_fixed_data_size(0), Some(0));
        assert_eq!(field_map.get_max_fixed_data_size(1), Some(0));
    }

    /// Output format of `Display` (Java's `toString`).
    #[test]
    fn display_matches_java_layout() {
        let file = ProjectFile::new();
        let mut field_map = FieldMap::new(&file);
        field_map.set_default_task_data(vec![
            FieldItem::new(
                task_field(TaskField::UniqueId),
                FieldLocation::FixedData,
                0,
                0,
                0,
                0,
                0,
            ),
            FieldItem::new(
                task_field(TaskField::Name),
                FieldLocation::VarData,
                0,
                65535,
                11,
                0,
                0,
            ),
        ]);
        field_map.create_task_field_map(&Props::new());

        let text = field_map.to_string();
        assert!(text.starts_with("[FieldMap\n"));
        assert!(text.contains(" MaxFixedOffset (block 0)=0\n"));
        assert!(text.contains(" MaxFixedOffset (block 1)=0\n"));
        // Fixed data items come before variable data items (the declaration order of FieldLocation)
        assert!(text.contains("[FieldItem type=TASK."));
        assert!(text.contains("location=FIXED_DATA fixedDataBlockIndex=0 fixedDataBlockOffset=0]"));
        assert!(text.contains("location=VAR_DATA varDataKey=11]"));
        assert!(text.ends_with("]\n"));
    }

    /// Comparison order of `FieldItem` (Java's `compareTo`).
    #[test]
    fn field_item_compare_to_follows_java_order() {
        let fixed_before = FieldItem::new(
            task_field(TaskField::UniqueId),
            FieldLocation::FixedData,
            0,
            0,
            0,
            0,
            0,
        );
        let fixed_after = FieldItem::new(
            task_field(TaskField::Id),
            FieldLocation::FixedData,
            0,
            4,
            0,
            0,
            0,
        );
        let var_item = FieldItem::new(
            task_field(TaskField::Name),
            FieldLocation::VarData,
            0,
            65535,
            11,
            0,
            0,
        );
        let meta_item = FieldItem::new(
            task_field(TaskField::Critical),
            FieldLocation::MetaData,
            0,
            0,
            0,
            1,
            0,
        );

        assert!(fixed_before.compare_to(&fixed_after) < 0);
        assert!(fixed_after.compare_to(&fixed_before) > 0);
        // Location first: fixed < variable < metadata
        assert!(fixed_after.compare_to(&var_item) < 0);
        assert!(var_item.compare_to(&meta_item) < 0);
        assert_eq!(fixed_before.compare_to(&fixed_before), 0);
    }
}
