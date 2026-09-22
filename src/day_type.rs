//! Corresponds to Java: `org/mpxj/DayType.java`

use crate::mpxj_enum::MpxjEnum;

/// Type of a day in a project calendar.
///
/// Corresponds to Java: `public enum DayType implements MpxjEnum`
///
/// In Java, `EnumHelper.createTypeArray(DayType.class)` builds the lookup table: the array length is 3,
/// and the values 0..2 fill it exactly, so `getInstance` never returns `null`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DayType {
    /// Corresponds to Java: `NON_WORKING(0)`
    NonWorking = 0,
    /// Corresponds to Java: `WORKING(1)`
    Working = 1,
    /// Corresponds to Java: `DEFAULT(2)`
    Default = 2,
}

impl DayType {
    /// All values, in the same order as the Java declaration order.
    ///
    /// Corresponds to Java: `private static final DayType[] TYPE_VALUES` (after removing the reflection table construction).
    pub const ALL: [DayType; 3] = [DayType::NonWorking, DayType::Working, DayType::Default];

    /// Returns the enum instance for the given int value, falling back to `NON_WORKING` when out of bounds.
    ///
    /// Corresponds to Java: `public static DayType getInstance(int type)`
    pub fn get_instance(value: i32) -> DayType {
        match value {
            0 => DayType::NonWorking,
            1 => DayType::Working,
            2 => DayType::Default,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = NON_WORKING.getValue(); }`
            _ => DayType::NonWorking,
        }
    }

    /// Corresponds to Java: `public static DayType getInstance(Number type)`
    ///
    /// In Java, `null` goes through `getInstance(-1)` and ultimately falls back to `NON_WORKING`.
    pub fn get_instance_optional(value: Option<i32>) -> DayType {
        DayType::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for DayType {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<DayType> for i32 {
    fn from(value: DayType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_type_round_trips() {
        for item in DayType::ALL {
            assert_eq!(DayType::get_instance(item.value()), item);
        }
    }

    #[test]
    fn day_type_falls_back_to_non_working() {
        assert_eq!(DayType::get_instance(-1), DayType::NonWorking);
        assert_eq!(DayType::get_instance(3), DayType::NonWorking);
        assert_eq!(DayType::get_instance_optional(None), DayType::NonWorking);
    }
}
