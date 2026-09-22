//! Corresponds to Java: `org/mpxj/CustomFieldValueDataType.java`

use std::fmt::{self, Display, Formatter};

use crate::data_type::DataType;
use crate::mpxj_enum::MpxjEnum;

/// Data type of a custom field value item.
///
/// Corresponds to Java: `public enum CustomFieldValueDataType implements MpxjEnum`
///
/// On the Java side each constant carries three fields: `(int value, int maskValue, DataType type)`:
///
/// | Constant | `getValue()` | `getMaskValue()` | `getDataType()` |
/// |---|---|---|---|
/// | `DATE` | 4 | 4 | `DataType.DATE` |
/// | `DURATION` | 6 | 6 | `DataType.DURATION` |
/// | `COST` | 9 | 5 | `DataType.CURRENCY` |
/// | `NUMBER` | 15 | 7 | `DataType.NUMERIC` |
/// | `FLAG` | 17 | 17 | `DataType.BOOLEAN` |
/// | `TEXT` | 21 | 3 | `DataType.STRING` |
/// | `FINISH_DATE` | 27 | 9 | `DataType.DATE` |
///
/// On the Rust side `value` is carried by the explicit discriminant (returned directly by [`MpxjEnum::value`]),
/// while `maskValue` / `type` are returned by [`CustomFieldValueDataType::get_mask_value`] and
/// [`CustomFieldValueDataType::get_data_type`] via `match`.
///
/// # Differences from Java: reflection-based table creation replaced by `match`
///
/// Java uses `EnumHelper.createTypeArray` reflection to generate two tables:
/// * `TYPE_VALUES`: length `7 + 21 = 28`, indexed by `getValue()`;
/// * `MASK_VALUES`: length 28, indexed by `getMaskValue()`.
///
/// Rust does not use reflection; instead it uses [`CustomFieldValueDataType::ALL`] plus `match`;
/// [`get_instance`](CustomFieldValueDataType::get_instance) keeps Java's "return on index out of bounds
/// `null`" semantics (returns `None` when out of bounds or when the index has no corresponding constant).
///
/// Skipped Java mechanism: reflection (`EnumHelper.createTypeArray`, see above).
///
/// Unported Java methods: none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CustomFieldValueDataType {
    /// Corresponds to Java: `DATE(4, 4, DataType.DATE)`
    Date = 4,
    /// Corresponds to Java: `DURATION(6, 6, DataType.DURATION)`
    Duration = 6,
    /// Corresponds to Java: `COST(9, 5, DataType.CURRENCY)`
    Cost = 9,
    /// Corresponds to Java: `NUMBER(15, 7, DataType.NUMERIC)`
    Number = 15,
    /// Corresponds to Java: `FLAG(17, 17, DataType.BOOLEAN)`
    Flag = 17,
    /// Corresponds to Java: `TEXT(21, 3, DataType.STRING)`
    Text = 21,
    /// Corresponds to Java: `FINISH_DATE(27, 9, DataType.DATE)`
    FinishDate = 27,
}

impl CustomFieldValueDataType {
/// Java-side `TYPE_VALUES` array length: `EnumHelper.createTypeArray(class, 21)` generates
/// `number of constants (7) + 21 = 28` slots.
    const TYPE_VALUES_LEN: i32 = 28;

/// Java-side `MASK_VALUES` array length.
    const MASK_VALUES_LEN: i32 = 28;

/// All values, in the same order as the Java declarations.
    ///
/// Corresponds to Java: `values()` (both `EnumHelper.createTypeArray` and the static block depend on it).
    pub const ALL: [CustomFieldValueDataType; 7] = [
        CustomFieldValueDataType::Date,
        CustomFieldValueDataType::Duration,
        CustomFieldValueDataType::Cost,
        CustomFieldValueDataType::Number,
        CustomFieldValueDataType::Flag,
        CustomFieldValueDataType::Text,
        CustomFieldValueDataType::FinishDate,
    ];

/// Gets the enum instance by its `getValue()` value; returns `None` when none is found.
    ///
    /// Corresponds to Java: `public static CustomFieldValueDataType getInstance(int type)`
    ///
/// Java indexes into the `TYPE_VALUES` array and returns `null` when out of bounds; Rust uses `match`
/// as an equivalent expression (indices in `0..28` with no corresponding constant also return `None`).
    pub fn get_instance(value: i32) -> Option<CustomFieldValueDataType> {
        if !(0..Self::TYPE_VALUES_LEN).contains(&value) {
            return None;
        }

        match value {
            4 => Some(CustomFieldValueDataType::Date),
            6 => Some(CustomFieldValueDataType::Duration),
            9 => Some(CustomFieldValueDataType::Cost),
            15 => Some(CustomFieldValueDataType::Number),
            17 => Some(CustomFieldValueDataType::Flag),
            21 => Some(CustomFieldValueDataType::Text),
            27 => Some(CustomFieldValueDataType::FinishDate),
            _ => None,
        }
    }

/// Gets the enum instance by its mask value; returns `None` when none is found.
    ///
    /// Corresponds to Java: `public static CustomFieldValueDataType getInstanceByMaskValue(int type)`
    ///
/// Java indexes into the `MASK_VALUES` array and returns `null` when out of bounds.
    pub fn get_instance_by_mask_value(value: i32) -> Option<CustomFieldValueDataType> {
        if !(0..Self::MASK_VALUES_LEN).contains(&value) {
            return None;
        }

        match value {
            3 => Some(CustomFieldValueDataType::Text),
            4 => Some(CustomFieldValueDataType::Date),
            5 => Some(CustomFieldValueDataType::Cost),
            6 => Some(CustomFieldValueDataType::Duration),
            7 => Some(CustomFieldValueDataType::Number),
            9 => Some(CustomFieldValueDataType::FinishDate),
            17 => Some(CustomFieldValueDataType::Flag),
            _ => None,
        }
    }

/// Gets the MPXJ data type.
    ///
    /// Corresponds to Java: `public DataType getDataType()`
    pub fn get_data_type(&self) -> DataType {
        match self {
            CustomFieldValueDataType::Date => DataType::Date,
            CustomFieldValueDataType::Duration => DataType::Duration,
            CustomFieldValueDataType::Cost => DataType::Currency,
            CustomFieldValueDataType::Number => DataType::Numeric,
            CustomFieldValueDataType::Flag => DataType::Boolean,
            CustomFieldValueDataType::Text => DataType::String,
            CustomFieldValueDataType::FinishDate => DataType::Date,
        }
    }

/// Gets the mask value.
    ///
    /// Corresponds to Java: `public int getMaskValue()`
    pub fn get_mask_value(&self) -> i32 {
        match self {
            CustomFieldValueDataType::Date => 4,
            CustomFieldValueDataType::Duration => 6,
            CustomFieldValueDataType::Cost => 5,
            CustomFieldValueDataType::Number => 7,
            CustomFieldValueDataType::Flag => 17,
            CustomFieldValueDataType::Text => 3,
            CustomFieldValueDataType::FinishDate => 9,
        }
    }
}

impl MpxjEnum for CustomFieldValueDataType {
/// Returns the int value associated with this enum.
    ///
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for CustomFieldValueDataType {
/// The Java enum does not override `toString()`, so it returns the constant name by default (e.g. `"DATE"`); Rust uses
/// `Display` to provide the same output, used by [`crate::custom_field_value_mask::CustomFieldValueMask`]
/// `toString()`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CustomFieldValueDataType::Date => "DATE",
            CustomFieldValueDataType::Duration => "DURATION",
            CustomFieldValueDataType::Cost => "COST",
            CustomFieldValueDataType::Number => "NUMBER",
            CustomFieldValueDataType::Flag => "FLAG",
            CustomFieldValueDataType::Text => "TEXT",
            CustomFieldValueDataType::FinishDate => "FINISH_DATE",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_values_round_trip_through_get_instance() {
        for item in CustomFieldValueDataType::ALL {
            assert_eq!(
                CustomFieldValueDataType::get_instance(item.value()),
                Some(item)
            );
        }
    }

    #[test]
    fn get_instance_returns_none_outside_type_values() {
// index with no corresponding constant
        assert_eq!(CustomFieldValueDataType::get_instance(0), None);
        assert_eq!(CustomFieldValueDataType::get_instance(5), None);
        assert_eq!(CustomFieldValueDataType::get_instance(26), None);

// array index out of bounds
        assert_eq!(CustomFieldValueDataType::get_instance(-1), None);
        assert_eq!(CustomFieldValueDataType::get_instance(28), None);
    }

    #[test]
    fn get_instance_by_mask_value_matches_java_table() {
        assert_eq!(
            CustomFieldValueDataType::get_instance_by_mask_value(3),
            Some(CustomFieldValueDataType::Text)
        );
        assert_eq!(
            CustomFieldValueDataType::get_instance_by_mask_value(4),
            Some(CustomFieldValueDataType::Date)
        );
        assert_eq!(
            CustomFieldValueDataType::get_instance_by_mask_value(5),
            Some(CustomFieldValueDataType::Cost)
        );
        assert_eq!(
            CustomFieldValueDataType::get_instance_by_mask_value(6),
            Some(CustomFieldValueDataType::Duration)
        );
        assert_eq!(
            CustomFieldValueDataType::get_instance_by_mask_value(7),
            Some(CustomFieldValueDataType::Number)
        );
        assert_eq!(
            CustomFieldValueDataType::get_instance_by_mask_value(9),
            Some(CustomFieldValueDataType::FinishDate)
        );
        assert_eq!(
            CustomFieldValueDataType::get_instance_by_mask_value(17),
            Some(CustomFieldValueDataType::Flag)
        );

        assert_eq!(CustomFieldValueDataType::get_instance_by_mask_value(0), None);
        assert_eq!(CustomFieldValueDataType::get_instance_by_mask_value(28), None);
        assert_eq!(CustomFieldValueDataType::get_instance_by_mask_value(-1), None);
    }

    #[test]
    fn data_types_and_mask_values_match_java() {
        let expected = [
            (CustomFieldValueDataType::Date, DataType::Date, 4),
            (CustomFieldValueDataType::Duration, DataType::Duration, 6),
            (CustomFieldValueDataType::Cost, DataType::Currency, 5),
            (CustomFieldValueDataType::Number, DataType::Numeric, 7),
            (CustomFieldValueDataType::Flag, DataType::Boolean, 17),
            (CustomFieldValueDataType::Text, DataType::String, 3),
            (CustomFieldValueDataType::FinishDate, DataType::Date, 9),
        ];

        for (item, data_type, mask_value) in expected {
            assert_eq!(item.get_data_type(), data_type);
            assert_eq!(item.get_mask_value(), mask_value);
        }
    }

    #[test]
    fn display_matches_java_enum_names() {
        assert_eq!(CustomFieldValueDataType::Date.to_string(), "DATE");
        assert_eq!(CustomFieldValueDataType::Text.to_string(), "TEXT");
        assert_eq!(
            CustomFieldValueDataType::FinishDate.to_string(),
            "FINISH_DATE"
        );
    }
}
