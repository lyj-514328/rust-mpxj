//! Corresponds to Java: `org/mpxj/BookingType.java`

use crate::mpxj_enum::MpxjEnum;

/// Resource booking type.
///
/// Corresponds to Java: `public enum BookingType implements MpxjEnum`
///
/// In Java, `EnumHelper.createTypeArray(BookingType.class)` builds the lookup table: the array length is
/// the constant count 2, and indices 0/1 are filled by `COMMITTED`/`PROPOSED` respectively, so `getInstance`
/// never returns `null`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BookingType {
    /// Corresponds to Java: `COMMITTED(0)`
    Committed = 0,
    /// Corresponds to Java: `PROPOSED(1)`
    Proposed = 1,
}

impl BookingType {
    /// All values, in the same order as the Java declaration order.
    ///
    /// Corresponds to Java: `private static final BookingType[] TYPE_VALUES` (after removing the reflection table construction).
    pub const ALL: [BookingType; 2] = [BookingType::Committed, BookingType::Proposed];

    /// Returns the enum instance for the given int value, falling back to `PROPOSED` when out of bounds.
    ///
    /// Corresponds to Java: `public static BookingType getInstance(int type)`
    ///
    /// Java has no `getInstance(Number)` overload, so `get_instance_optional` is not provided.
    pub fn get_instance(value: i32) -> BookingType {
        match value {
            0 => BookingType::Committed,
            1 => BookingType::Proposed,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = PROPOSED.getValue(); }`
            _ => BookingType::Proposed,
        }
    }
}

impl MpxjEnum for BookingType {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<BookingType> for i32 {
    fn from(value: BookingType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn booking_type_round_trips() {
        for item in BookingType::ALL {
            assert_eq!(BookingType::get_instance(item.value()), item);
        }
    }

    #[test]
    fn booking_type_falls_back_to_proposed() {
        assert_eq!(BookingType::get_instance(-1), BookingType::Proposed);
        assert_eq!(BookingType::get_instance(2), BookingType::Proposed);
    }
}
