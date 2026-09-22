//! Corresponds to Java: `org/mpxj/ConstraintType.java`

use crate::mpxj_enum::MpxjEnum;

/// Task constraint type.
///
/// Corresponds to Java: `public enum ConstraintType implements MpxjEnum`
///
/// In Java, `EnumHelper.createTypeArray(ConstraintType.class)` builds the lookup table: the array length is
/// 10, and the values 0..9 fill it exactly, so `getInstance` never returns `null`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstraintType {
    /// Corresponds to Java: `AS_SOON_AS_POSSIBLE(0)`
    AsSoonAsPossible = 0,
    /// Corresponds to Java: `AS_LATE_AS_POSSIBLE(1)`
    AsLateAsPossible = 1,
    /// Corresponds to Java: `MUST_START_ON(2)`
    MustStartOn = 2,
    /// Corresponds to Java: `MUST_FINISH_ON(3)`
    MustFinishOn = 3,
    /// Corresponds to Java: `START_NO_EARLIER_THAN(4)`
    StartNoEarlierThan = 4,
    /// Corresponds to Java: `START_NO_LATER_THAN(5)`
    StartNoLaterThan = 5,
    /// Corresponds to Java: `FINISH_NO_EARLIER_THAN(6)`
    FinishNoEarlierThan = 6,
    /// Corresponds to Java: `FINISH_NO_LATER_THAN(7)`
    FinishNoLaterThan = 7,
    /// Corresponds to Java: `START_ON(8)`
    StartOn = 8,
    /// Corresponds to Java: `FINISH_ON(9)`
    FinishOn = 9,
}

impl ConstraintType {
    /// All values, in the same order as the Java declaration order.
    ///
    /// Corresponds to Java: `private static final ConstraintType[] TYPE_VALUES` (after removing the reflection table construction).
    pub const ALL: [ConstraintType; 10] = [
        ConstraintType::AsSoonAsPossible,
        ConstraintType::AsLateAsPossible,
        ConstraintType::MustStartOn,
        ConstraintType::MustFinishOn,
        ConstraintType::StartNoEarlierThan,
        ConstraintType::StartNoLaterThan,
        ConstraintType::FinishNoEarlierThan,
        ConstraintType::FinishNoLaterThan,
        ConstraintType::StartOn,
        ConstraintType::FinishOn,
    ];

    /// Returns the enum instance for the given int value, falling back to `AS_SOON_AS_POSSIBLE` when out of bounds.
    ///
    /// Corresponds to Java: `public static ConstraintType getInstance(int type)`
    pub fn get_instance(value: i32) -> ConstraintType {
        match value {
            0 => ConstraintType::AsSoonAsPossible,
            1 => ConstraintType::AsLateAsPossible,
            2 => ConstraintType::MustStartOn,
            3 => ConstraintType::MustFinishOn,
            4 => ConstraintType::StartNoEarlierThan,
            5 => ConstraintType::StartNoLaterThan,
            6 => ConstraintType::FinishNoEarlierThan,
            7 => ConstraintType::FinishNoLaterThan,
            8 => ConstraintType::StartOn,
            9 => ConstraintType::FinishOn,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = AS_SOON_AS_POSSIBLE.getValue(); }`
            _ => ConstraintType::AsSoonAsPossible,
        }
    }

    /// Corresponds to Java: `public static ConstraintType getInstance(Number type)`
    ///
    /// In Java, `null` goes through `getInstance(-1)` and ultimately falls back to `AS_SOON_AS_POSSIBLE`.
    pub fn get_instance_optional(value: Option<i32>) -> ConstraintType {
        ConstraintType::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for ConstraintType {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<ConstraintType> for i32 {
    fn from(value: ConstraintType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constraint_type_round_trips() {
        for item in ConstraintType::ALL {
            assert_eq!(ConstraintType::get_instance(item.value()), item);
        }
    }

    #[test]
    fn constraint_type_falls_back_to_as_soon_as_possible() {
        assert_eq!(
            ConstraintType::get_instance(-1),
            ConstraintType::AsSoonAsPossible
        );
        assert_eq!(
            ConstraintType::get_instance(10),
            ConstraintType::AsSoonAsPossible
        );
        assert_eq!(
            ConstraintType::get_instance_optional(None),
            ConstraintType::AsSoonAsPossible
        );
    }
}
