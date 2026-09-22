//! Corresponds to Java: `org/mpxj/DateOrder.java`

use crate::mpxj_enum::MpxjEnum;
use std::fmt::{self, Display, Formatter};

/// Date writing order (month-day-year / day-month-year / year-month-day).
///
/// Corresponds to Java: `public enum DateOrder implements MpxjEnum`
///
/// In Java, `EnumHelper.createTypeArray(DateOrder.class)` builds the lookup table: the array length is 3,
/// and the values 0..2 fill it exactly, so `getInstance` never returns `null`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DateOrder {
    /// Corresponds to Java: `MDY(0)`
    Mdy = 0,
    /// Corresponds to Java: `DMY(1)`
    Dmy = 1,
    /// Corresponds to Java: `YMD(2)`
    Ymd = 2,
}

impl DateOrder {
    /// All values, in the same order as the Java declaration order.
    ///
    /// Corresponds to Java: `private static final DateOrder[] TYPE_VALUES` (after removing the reflection table construction).
    pub const ALL: [DateOrder; 3] = [DateOrder::Mdy, DateOrder::Dmy, DateOrder::Ymd];

    /// Returns the enum instance for the given int value, falling back to `MDY` when out of bounds.
    ///
    /// Corresponds to Java: `public static DateOrder getInstance(int type)`
    pub fn get_instance(value: i32) -> DateOrder {
        match value {
            0 => DateOrder::Mdy,
            1 => DateOrder::Dmy,
            2 => DateOrder::Ymd,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = MDY.getValue(); }`
            _ => DateOrder::Mdy,
        }
    }

    /// Corresponds to Java: `public static DateOrder getInstance(Number type)`
    ///
    /// In Java, `null` goes through `getInstance(-1)` and ultimately falls back to `MDY`.
    pub fn get_instance_optional(value: Option<i32>) -> DateOrder {
        DateOrder::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for DateOrder {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for DateOrder {
    /// Corresponds to Java: `@Override public String toString()`, i.e. `Integer.toString(m_value)`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

impl From<DateOrder> for i32 {
    fn from(value: DateOrder) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_order_round_trips() {
        for item in DateOrder::ALL {
            assert_eq!(DateOrder::get_instance(item.value()), item);
        }
    }

    #[test]
    fn date_order_falls_back_to_mdy() {
        assert_eq!(DateOrder::get_instance(-1), DateOrder::Mdy);
        assert_eq!(DateOrder::get_instance(3), DateOrder::Mdy);
        assert_eq!(DateOrder::get_instance_optional(None), DateOrder::Mdy);
    }

    #[test]
    fn date_order_to_string() {
        assert_eq!(DateOrder::Mdy.to_string(), "0");
        assert_eq!(DateOrder::Ymd.to_string(), "2");
    }
}
