//! Corresponds to Java: `org/mpxj/ActivityStatus.java`

use crate::mpxj_enum::MpxjEnum;

/// Execution status of an Activity.
///
/// Corresponds to Java: `public enum ActivityStatus`
///
/// The Java enum does not implement `MpxjEnum` and has no `getInstance`; the Rust side provides
/// [`ActivityStatus::ALL`] in declaration order and implements [`MpxjEnum::value`] using the ordinal (`ordinal()`),
/// so int and enum can be converted both ways (`get_instance` has no corresponding method in Java).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActivityStatus {
    /// Corresponds to Java: `NOT_STARTED`
    NotStarted,
    /// Corresponds to Java: `IN_PROGRESS`
    InProgress,
    /// Corresponds to Java: `COMPLETED`
    Completed,
}

impl ActivityStatus {
    /// All values, in the same order as the Java declaration order.
    pub const ALL: [ActivityStatus; 3] = [
        ActivityStatus::NotStarted,
        ActivityStatus::InProgress,
        ActivityStatus::Completed,
    ];

    /// Returns the enum instance by ordinal (Java's `ordinal()`), returning `None` when out of bounds.
    ///
    /// No corresponding method in Java (Java uses enum constants or `values()[n]` directly).
    pub fn get_instance(value: i32) -> Option<ActivityStatus> {
        match value {
            0 => Some(ActivityStatus::NotStarted),
            1 => Some(ActivityStatus::InProgress),
            2 => Some(ActivityStatus::Completed),
            _ => None,
        }
    }
}

impl MpxjEnum for ActivityStatus {
    /// Returns the ordinal corresponding to the declaration order.
    ///
    /// Java has no `getValue()`; this corresponds to `Enum.ordinal()`.
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<ActivityStatus> for i32 {
    fn from(value: ActivityStatus) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_status_round_trips() {
        for item in ActivityStatus::ALL {
            assert_eq!(ActivityStatus::get_instance(item.value()), Some(item));
        }
    }

    #[test]
    fn activity_status_out_of_range() {
        assert_eq!(ActivityStatus::get_instance(-1), None);
        assert_eq!(ActivityStatus::get_instance(3), None);
    }
}
