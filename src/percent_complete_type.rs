//! Corresponds to Java: `org/mpxj/PercentCompleteType.java`

use crate::mpxj_enum::MpxjEnum;

/// Meaning of percent complete (by duration / by physical amount / by resource units / by scope).
///
/// Corresponds to Java: `public enum PercentCompleteType`
///
/// The Java enum does not implement `MpxjEnum` and has no `getInstance`; the Rust side provides
/// [`PercentCompleteType::ALL`] in declaration order and implements [`MpxjEnum::value`] using the ordinal (`ordinal()`),
/// so int and enum can be converted both ways (`get_instance` has no corresponding method in Java).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PercentCompleteType {
    /// Corresponds to Java: `DURATION`
    Duration,
    /// Corresponds to Java: `PHYSICAL`
    Physical,
    /// Corresponds to Java: `UNITS`
    Units,
    /// Corresponds to Java: `SCOPE`
    Scope,
}

impl PercentCompleteType {
    /// All values, in the same order as the Java declaration order.
    pub const ALL: [PercentCompleteType; 4] = [
        PercentCompleteType::Duration,
        PercentCompleteType::Physical,
        PercentCompleteType::Units,
        PercentCompleteType::Scope,
    ];

    /// Returns the enum instance by ordinal (Java's `ordinal()`), returning `None` when out of bounds.
    ///
    /// No corresponding method in Java (Java uses enum constants or `values()[n]` directly).
    pub fn get_instance(value: i32) -> Option<PercentCompleteType> {
        match value {
            0 => Some(PercentCompleteType::Duration),
            1 => Some(PercentCompleteType::Physical),
            2 => Some(PercentCompleteType::Units),
            3 => Some(PercentCompleteType::Scope),
            _ => None,
        }
    }
}

impl MpxjEnum for PercentCompleteType {
    /// Returns the ordinal corresponding to the declaration order.
    ///
    /// Java has no `getValue()`; this corresponds to `Enum.ordinal()`.
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<PercentCompleteType> for i32 {
    fn from(value: PercentCompleteType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_complete_type_round_trips() {
        for item in PercentCompleteType::ALL {
            assert_eq!(PercentCompleteType::get_instance(item.value()), Some(item));
        }
    }

    #[test]
    fn percent_complete_type_out_of_range() {
        assert_eq!(PercentCompleteType::get_instance(-1), None);
        assert_eq!(PercentCompleteType::get_instance(4), None);
    }
}
