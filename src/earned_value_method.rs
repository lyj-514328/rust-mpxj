//! Corresponds to Java: `org/mpxj/EarnedValueMethod.java`

use crate::mpxj_enum::MpxjEnum;

/// Earned Value calculation method.
///
/// Corresponds to Java: `public enum EarnedValueMethod implements MpxjEnum`
///
/// In Java, `EnumHelper.createTypeArray(EarnedValueMethod.class)` builds the lookup table: the array length is
/// 2, and the values 0/1 fill it exactly, so `getInstance` never returns `null`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EarnedValueMethod {
    /// Corresponds to Java: `PERCENT_COMPLETE(0)`
    PercentComplete = 0,
    /// Corresponds to Java: `PHYSICAL_PERCENT_COMPLETE(1)`
    PhysicalPercentComplete = 1,
}

impl EarnedValueMethod {
    /// All values, in the same order as the Java declaration order.
    ///
    /// Corresponds to Java: `private static final EarnedValueMethod[] TYPE_VALUES` (after removing the reflection table construction).
    pub const ALL: [EarnedValueMethod; 2] = [
        EarnedValueMethod::PercentComplete,
        EarnedValueMethod::PhysicalPercentComplete,
    ];

    /// Returns the enum instance for the given int value, falling back to `PHYSICAL_PERCENT_COMPLETE` when out of bounds.
    ///
    /// Corresponds to Java: `public static EarnedValueMethod getInstance(int type)`
    pub fn get_instance(value: i32) -> EarnedValueMethod {
        match value {
            0 => EarnedValueMethod::PercentComplete,
            1 => EarnedValueMethod::PhysicalPercentComplete,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = PHYSICAL_PERCENT_COMPLETE.getValue(); }`
            _ => EarnedValueMethod::PhysicalPercentComplete,
        }
    }

    /// Corresponds to Java: `public static EarnedValueMethod getInstance(Number type)`
    ///
    /// In Java, `null` goes through `getInstance(-1)` and ultimately falls back to `PHYSICAL_PERCENT_COMPLETE`.
    pub fn get_instance_optional(value: Option<i32>) -> EarnedValueMethod {
        EarnedValueMethod::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for EarnedValueMethod {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<EarnedValueMethod> for i32 {
    fn from(value: EarnedValueMethod) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earned_value_method_round_trips() {
        for item in EarnedValueMethod::ALL {
            assert_eq!(EarnedValueMethod::get_instance(item.value()), item);
        }
    }

    #[test]
    fn earned_value_method_falls_back_to_physical_percent_complete() {
        assert_eq!(
            EarnedValueMethod::get_instance(-1),
            EarnedValueMethod::PhysicalPercentComplete
        );
        assert_eq!(
            EarnedValueMethod::get_instance(2),
            EarnedValueMethod::PhysicalPercentComplete
        );
        assert_eq!(
            EarnedValueMethod::get_instance_optional(None),
            EarnedValueMethod::PhysicalPercentComplete
        );
    }
}
