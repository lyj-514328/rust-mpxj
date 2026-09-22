//! Corresponds to Java: `org/mpxj/CriticalActivityType.java`

use crate::mpxj_enum::MpxjEnum;

/// Determines how a task's critical path flag is evaluated.
///
/// Corresponds to Java: `public enum CriticalActivityType`
///
/// The Java enum does not implement `MpxjEnum` and has no `getInstance`; the Rust side provides
/// [`CriticalActivityType::ALL`] in declaration order and implements [`MpxjEnum::value`] using the ordinal (`ordinal()`),
/// so int and enum can be converted both ways (`get_instance` has no corresponding method in Java).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CriticalActivityType {
    /// Corresponds to Java: `TOTAL_FLOAT`
    TotalFloat,
    /// Corresponds to Java: `LONGEST_PATH`
    LongestPath,
}

impl CriticalActivityType {
    /// All values, in the same order as the Java declaration order.
    pub const ALL: [CriticalActivityType; 2] = [
        CriticalActivityType::TotalFloat,
        CriticalActivityType::LongestPath,
    ];

    /// Returns the enum instance by ordinal (Java's `ordinal()`), returning `None` when out of bounds.
    ///
    /// No corresponding method in Java (Java uses enum constants or `values()[n]` directly).
    pub fn get_instance(value: i32) -> Option<CriticalActivityType> {
        match value {
            0 => Some(CriticalActivityType::TotalFloat),
            1 => Some(CriticalActivityType::LongestPath),
            _ => None,
        }
    }
}

impl MpxjEnum for CriticalActivityType {
    /// Returns the ordinal corresponding to the declaration order.
    ///
    /// Java has no `getValue()`; this corresponds to `Enum.ordinal()`.
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<CriticalActivityType> for i32 {
    fn from(value: CriticalActivityType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn critical_activity_type_round_trips() {
        for item in CriticalActivityType::ALL {
            assert_eq!(CriticalActivityType::get_instance(item.value()), Some(item));
        }
    }

    #[test]
    fn critical_activity_type_out_of_range() {
        assert_eq!(CriticalActivityType::get_instance(-1), None);
        assert_eq!(CriticalActivityType::get_instance(2), None);
    }
}
