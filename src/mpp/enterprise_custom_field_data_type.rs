//! Corresponds to Java: `org/mpxj/mpp/EnterpriseCustomFieldDataType.java`
//!
//! Lookup table for "integer type ID ↔ [`DataType`]" of enterprise custom fields.
//! The meaning of the IDs is taken from Microsoft Project's XML data exchange format
//! (`https://docs.microsoft.com/en-us/office-project/xml-data-interchange/cftype-element`,
//! (see the comments in the Java source).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `public final class EnterpriseCustomFieldDataType` + two static `HashMap`s | Unit struct [`EnterpriseCustomFieldDataType`] + two `match` expressions | Java's `DATA_TYPE_MAP` / `ID_MAP` are just static lookup tables; the Rust side expresses the same mappings directly with `match` (this crate's customary approach, see `crate::data_type::DataType::get_instance`), with no need to introduce extra mechanisms such as `OnceLock` |
//! | `Integer getIDFromDataType(DataType type)` (may return `null`) | `Option<i32>` | The "`null` → `Option<T>`" convention |
//! | `DATA_TYPE_MAP.getOrDefault(value, DataType.CUSTOM)` | The `_` branch of the `match` falls back to `DataType::Custom` | Out-of-range IDs always fall back to CUSTOM |
//! | `ID_MAP.get(type)` (returns `null` when `type` is `null`) | The parameter is `DataType` (a value type), with no `null` case | `DataType` is a `Copy` enum on the Rust side |
//!
//! Skipped Java methods: none (this class only has the two static methods above).
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use crate::data_type::DataType;

/// Lookup table for enterprise custom field data types.
///
/// Corresponds to Java: `public final class EnterpriseCustomFieldDataType`
pub struct EnterpriseCustomFieldDataType;

impl EnterpriseCustomFieldDataType {
/// Get the data type of an enterprise custom field by integer type ID.
    ///
    /// Corresponds to Java: `public static DataType getDataTypeFromID(int value)`
    ///
/// Consistent with Java: an unrecognized ID falls back to [`DataType::Custom`].
    pub fn get_data_type_from_id(value: i32) -> DataType {
        match value {
            0 => DataType::Currency,
            1 => DataType::Date,
            2 => DataType::Duration,
// Java comment: 3 = Finish
            3 => DataType::Date,
            4 => DataType::Boolean,
            5 => DataType::Numeric,
// Java comment: 6 = Start
            6 => DataType::Date,
            7 => DataType::String,
            // Java: DATA_TYPE_MAP.getOrDefault(Integer.valueOf(value), DataType.CUSTOM)
            _ => DataType::Custom,
        }
    }

/// Get the integer type ID of an enterprise custom field by data type.
    ///
    /// Corresponds to Java: `public static Integer getIDFromDataType(DataType type)`
    ///
/// Consistent with Java: a type not in the table returns `None` (Java returns `null`). Note that this mapping is not
/// bidirectional — `DATE` has ID 1, while `3` / `6` (Finish / Start) have no corresponding entry in the reverse table.
    pub fn get_id_from_data_type(type_: DataType) -> Option<i32> {
        match type_ {
            DataType::Currency => Some(0),
            DataType::Date => Some(1),
            DataType::Duration => Some(2),
            DataType::Boolean => Some(4),
            DataType::Numeric => Some(5),
            DataType::String => Some(7),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// Forward table: the mapping of the 8 IDs is consistent with Java's `DATA_TYPE_MAP`.
    #[test]
    fn known_ids_map_to_the_java_data_types() {
        let cases = [
            (0, DataType::Currency),
            (1, DataType::Date),
            // Finish
            (2, DataType::Duration),
            (3, DataType::Date),
            (4, DataType::Boolean),
            (5, DataType::Numeric),
            // Start
            (6, DataType::Date),
            (7, DataType::String),
        ];

        for (id, data_type) in cases {
            assert_eq!(EnterpriseCustomFieldDataType::get_data_type_from_id(id), data_type);
        }
    }

/// Out-of-range / unrecognized IDs fall back to `CUSTOM`.
    #[test]
    fn unknown_ids_fall_back_to_custom() {
        for id in [-1, 8, 100, i32::MIN, i32::MAX] {
            assert_eq!(
                EnterpriseCustomFieldDataType::get_data_type_from_id(id),
                DataType::Custom
            );
        }
    }

/// Reverse table: the 6 reversible types are consistent with Java's `ID_MAP`.
    #[test]
    fn reverse_lookup_matches_the_java_id_map() {
        let cases = [
            (DataType::Currency, 0),
            (DataType::Date, 1),
            (DataType::Duration, 2),
            (DataType::Boolean, 4),
            (DataType::Numeric, 5),
            (DataType::String, 7),
        ];

        for (data_type, id) in cases {
            assert_eq!(
                EnterpriseCustomFieldDataType::get_id_from_data_type(data_type),
                Some(id)
            );
        }
    }

/// A type not in the table returns `None` (Java returns `null`).
    #[test]
    fn reverse_lookup_returns_none_for_unmapped_types() {
        for data_type in [
            DataType::Custom,
            DataType::Guid,
            DataType::Integer,
            DataType::Short,
            DataType::Work,
            DataType::Percentage,
            DataType::Units,
        ] {
            assert_eq!(
                EnterpriseCustomFieldDataType::get_id_from_data_type(data_type),
                None
            );
        }
    }

/// The two tables are not strictly inverse: Finish (3) / Start (6) both normalize to `DATE`, and the reverse yields only ID 1.
    #[test]
    fn forward_and_reverse_are_not_exact_inverses() {
        for id in [1, 3, 6] {
            assert_eq!(
                EnterpriseCustomFieldDataType::get_id_from_data_type(
                    EnterpriseCustomFieldDataType::get_data_type_from_id(id)
                ),
                Some(1)
            );
        }
    }
}
