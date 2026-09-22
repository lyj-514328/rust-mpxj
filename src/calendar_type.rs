//! Corresponds to Java: `org/mpxj/CalendarType.java`

use crate::mpxj_enum::MpxjEnum;

/// P6 calendar type.
///
/// Corresponds to Java: `public enum CalendarType`
///
/// The Java enum does not implement `MpxjEnum` and has no `getInstance`; the Rust side provides
/// [`CalendarType::ALL`] in declaration order and implements [`MpxjEnum::value`] using the ordinal (`ordinal()`),
/// so int and enum can be converted both ways (`get_instance` has no corresponding method in Java).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CalendarType {
    /// Corresponds to Java: `GLOBAL`
    Global,
    /// Corresponds to Java: `PROJECT`
    Project,
    /// Corresponds to Java: `RESOURCE`
    Resource,
}

impl CalendarType {
    /// All values, in the same order as the Java declaration order.
    pub const ALL: [CalendarType; 3] = [
        CalendarType::Global,
        CalendarType::Project,
        CalendarType::Resource,
    ];

    /// Returns the enum instance by ordinal (Java's `ordinal()`), returning `None` when out of bounds.
    ///
    /// No corresponding method in Java (Java uses enum constants or `values()[n]` directly).
    pub fn get_instance(value: i32) -> Option<CalendarType> {
        match value {
            0 => Some(CalendarType::Global),
            1 => Some(CalendarType::Project),
            2 => Some(CalendarType::Resource),
            _ => None,
        }
    }
}

impl MpxjEnum for CalendarType {
    /// Returns the ordinal corresponding to the declaration order.
    ///
    /// Java has no `getValue()`; this corresponds to `Enum.ordinal()`.
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<CalendarType> for i32 {
    fn from(value: CalendarType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_type_round_trips() {
        for item in CalendarType::ALL {
            assert_eq!(CalendarType::get_instance(item.value()), Some(item));
        }
    }

    #[test]
    fn calendar_type_out_of_range() {
        assert_eq!(CalendarType::get_instance(-1), None);
        assert_eq!(CalendarType::get_instance(3), None);
    }
}
