//! Corresponds to Java: `org/mpxj/AccrueType.java`

use crate::mpxj_enum::MpxjEnum;

/// Cost accrual method (accrue type).
///
/// Corresponds to Java: `public enum AccrueType implements MpxjEnum`
///
/// In Java, `EnumHelper.createTypeArray(AccrueType.class, 1)` builds the int → enum
/// lookup table: the array length is `3 + 1 = 4`, and the values 1/2/3 are written into indices 1/2/3 respectively, while index 0 stays
/// `null`. Therefore Java's `getInstance(0)` returns `null`, and the Rust side correspondingly returns `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccrueType {
    /// Corresponds to Java: `START(1)`
    Start = 1,
    /// Corresponds to Java: `END(2)`
    End = 2,
    /// Corresponds to Java: `PRORATED(3)`
    Prorated = 3,
}

impl AccrueType {
    /// All values, in the same order as the Java declaration order.
    ///
    /// Corresponds to Java: `private static final AccrueType[] TYPE_VALUES` (after removing the reflection table construction).
    pub const ALL: [AccrueType; 3] = [AccrueType::Start, AccrueType::End, AccrueType::Prorated];

    /// Returns the enum instance for the given int value; falls back to `PRORATED` out of bounds, and index 0 (which is `null` in Java) returns `None`.
    ///
    /// Corresponds to Java: `public static AccrueType getInstance(int type)`
    pub fn get_instance(value: i32) -> Option<AccrueType> {
        match value {
            1 => Some(AccrueType::Start),
            2 => Some(AccrueType::End),
            3 => Some(AccrueType::Prorated),
            // Java: TYPE_VALUES has length 4, and index 0 is unfilled, so the lookup result is null.
            0 => None,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = PRORATED.getValue(); }`
            _ => Some(AccrueType::Prorated),
        }
    }

    /// Corresponds to Java: `public static AccrueType getInstance(Number type)`
    ///
    /// In Java, `null` goes through `getInstance(-1)` and ultimately falls back to `PRORATED`.
    pub fn get_instance_optional(value: Option<i32>) -> Option<AccrueType> {
        AccrueType::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for AccrueType {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<AccrueType> for i32 {
    fn from(value: AccrueType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accrue_type_round_trips() {
        for item in AccrueType::ALL {
            assert_eq!(AccrueType::get_instance(item.value()), Some(item));
        }
    }

    #[test]
    fn accrue_type_index_zero_is_none() {
        // Java: TYPE_VALUES[0] is null
        assert_eq!(AccrueType::get_instance(0), None);
    }

    #[test]
    fn accrue_type_falls_back_to_prorated() {
        assert_eq!(AccrueType::get_instance(-1), Some(AccrueType::Prorated));
        assert_eq!(AccrueType::get_instance(4), Some(AccrueType::Prorated));
        assert_eq!(
            AccrueType::get_instance_optional(None),
            Some(AccrueType::Prorated)
        );
    }
}
