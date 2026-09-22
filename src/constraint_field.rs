//! Corresponds to Java: `org/mpxj/ConstraintField.java`

use crate::data_type::DataType;
use crate::field_type::FieldType;
use crate::field_type_class::FieldTypeClass;
use crate::mpxj_enum::MpxjEnum;

/// A field on the constraint (Constraint) entity.
///
/// Corresponds to Java: `public enum ConstraintField implements FieldType`
///
/// The variant order matches the Java declaration order, so `*self as i32` equals Java's
/// `getValue()` (Java assigns values in declaration order in the static initializer block).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstraintField {
    UniqueId,
    Task1,
    Task2,
}

/// The English column name table for Java `LocaleData.CONSTRAINT_COLUMNS_ARRAY`, arranged by `getValue()` index.
///
/// Indices not assigned on the Java side are represented by the empty string `""`; this table has been verified to have no gaps.
const CONSTRAINT_COLUMN_NAMES: [&str; ConstraintField::MAX_VALUE] = ["Unique ID", "Task 1", "Task 2"];

impl ConstraintField {
    /// Total number of variants.
    ///
    /// Corresponds to Java: `public static final int MAX_VALUE`
    pub const MAX_VALUE: usize = 3;

    /// All values, in declaration order (i.e. ascending `getValue()`).
    pub const ALL: [ConstraintField; ConstraintField::MAX_VALUE] =
        [ConstraintField::UniqueId, ConstraintField::Task1, ConstraintField::Task2];

    /// Returns the enum instance for the given int value, returning `None` when out of bounds.
    ///
    /// Corresponds to Java: `public static ConstraintField getInstance(int type)`
    /// (Java returns `null` out of bounds).
    pub fn get_instance(value: i32) -> Option<ConstraintField> {
        if value >= 0 && (value as usize) < Self::MAX_VALUE {
            Some(Self::ALL[value as usize])
        } else {
            None
        }
    }
}

impl FieldType for ConstraintField {
    /// Corresponds to Java: `public FieldTypeClass getFieldTypeClass()`
    fn get_field_type_class(&self) -> FieldTypeClass {
        FieldTypeClass::Constraint
    }

    /// Corresponds to the `name()` provided by the Java enum
    fn name(&self) -> &'static str {
        match self {
            ConstraintField::UniqueId => "UNIQUE_ID",
            ConstraintField::Task1 => "TASK1",
            ConstraintField::Task2 => "TASK2",
        }
    }

    /// Corresponds to Java: `public String getName()`
    fn get_name(&self) -> &'static str {
        CONSTRAINT_COLUMN_NAMES[*self as usize]
    }

    /// Corresponds to Java: `public DataType getDataType()`
    fn get_data_type(&self) -> DataType {
        match self {
            ConstraintField::UniqueId | ConstraintField::Task1 | ConstraintField::Task2 => {
                DataType::Integer
            }
        }
    }

    /// Corresponds to Java: `public FieldType getUnitsType()` (Java returns `null`)
    fn get_units_type(&self) -> Option<Self> {
        None
    }
}

impl MpxjEnum for ConstraintField {
    /// Corresponds to Java: `public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl std::fmt::Display for ConstraintField {
    /// Corresponds to Java: `public String toString()`
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constraint_field_round_trips() {
        for item in ConstraintField::ALL {
            assert_eq!(ConstraintField::get_instance(item.value()), Some(item));
        }
    }

    #[test]
    fn constraint_field_out_of_range_returns_none() {
        assert_eq!(ConstraintField::get_instance(-1), None);
        assert_eq!(ConstraintField::get_instance(ConstraintField::MAX_VALUE as i32), None);
    }

    #[test]
    fn constraint_field_max_value_matches_variant_count() {
        assert_eq!(ConstraintField::ALL.len(), ConstraintField::MAX_VALUE);
    }
}
