//! Corresponds to Java: `org/mpxj/CurrencySymbolPosition.java`

use crate::mpxj_enum::MpxjEnum;
use std::fmt::{self, Display, Formatter};

/// Position of the currency symbol relative to the amount.
///
/// Corresponds to Java: `public enum CurrencySymbolPosition implements MpxjEnum`
///
/// In Java, `EnumHelper.createTypeArray(CurrencySymbolPosition.class)` builds the lookup table:
/// the array length is 4, and the values 0..3 fill it exactly, so `getInstance` never returns `null`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CurrencySymbolPosition {
    /// Corresponds to Java: `AFTER(0)`
    After = 0,
    /// Corresponds to Java: `BEFORE(1)`
    Before = 1,
    /// Corresponds to Java: `AFTER_WITH_SPACE(2)`
    AfterWithSpace = 2,
    /// Corresponds to Java: `BEFORE_WITH_SPACE(3)`
    BeforeWithSpace = 3,
}

impl CurrencySymbolPosition {
    /// All values, in the same order as the Java declaration order.
    ///
    /// Corresponds to Java: `private static final CurrencySymbolPosition[] TYPE_VALUES` (after removing the reflection table construction).
    pub const ALL: [CurrencySymbolPosition; 4] = [
        CurrencySymbolPosition::After,
        CurrencySymbolPosition::Before,
        CurrencySymbolPosition::AfterWithSpace,
        CurrencySymbolPosition::BeforeWithSpace,
    ];

    /// Returns the enum instance for the given int value, falling back to `BEFORE` when out of bounds.
    ///
    /// Corresponds to Java: `public static CurrencySymbolPosition getInstance(int value)`
    ///
    /// Java has no `getInstance(Number)` overload, so `get_instance_optional` is not provided.
    pub fn get_instance(value: i32) -> CurrencySymbolPosition {
        match value {
            0 => CurrencySymbolPosition::After,
            1 => CurrencySymbolPosition::Before,
            2 => CurrencySymbolPosition::AfterWithSpace,
            3 => CurrencySymbolPosition::BeforeWithSpace,
            // Java: `if (value < 0 || value >= TYPE_VALUES.length) { value = BEFORE.getValue(); }`
            _ => CurrencySymbolPosition::Before,
        }
    }
}

impl MpxjEnum for CurrencySymbolPosition {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for CurrencySymbolPosition {
    /// Corresponds to Java: `@Override public String toString()`, i.e. `Integer.toString(m_value)`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

impl From<CurrencySymbolPosition> for i32 {
    fn from(value: CurrencySymbolPosition) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn currency_symbol_position_round_trips() {
        for item in CurrencySymbolPosition::ALL {
            assert_eq!(CurrencySymbolPosition::get_instance(item.value()), item);
        }
    }

    #[test]
    fn currency_symbol_position_falls_back_to_before() {
        assert_eq!(
            CurrencySymbolPosition::get_instance(-1),
            CurrencySymbolPosition::Before
        );
        assert_eq!(
            CurrencySymbolPosition::get_instance(4),
            CurrencySymbolPosition::Before
        );
    }

    #[test]
    fn currency_symbol_position_to_string() {
        assert_eq!(CurrencySymbolPosition::After.to_string(), "0");
        assert_eq!(CurrencySymbolPosition::BeforeWithSpace.to_string(), "3");
    }
}
