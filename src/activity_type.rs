//! Corresponds to Java: `org/mpxj/ActivityType.java`

use crate::mpxj_enum::MpxjEnum;

/// P6/PPX activity type.
///
/// Corresponds to Java: `public enum ActivityType`
///
/// On the Java side this enum does not implement `MpxjEnum` and has no `getInstance`; the Rust
/// side provides [`ActivityType::ALL`] in declaration order and implements [`MpxjEnum::value`]
/// using the ordinal (`ordinal()`), so ints and the enum can convert between each other
/// (`get_instance` has no counterpart on the Java side).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActivityType {
    /// Corresponds to Java: `TASK_DEPENDENT`
    TaskDependent,
    /// Corresponds to Java: `RESOURCE_DEPENDENT`
    ResourceDependent,
    /// Corresponds to Java: `LEVEL_OF_EFFORT`
    LevelOfEffort,
    /// Corresponds to Java: `START_MILESTONE`
    StartMilestone,
    /// Corresponds to Java: `FINISH_MILESTONE`
    FinishMilestone,
    /// Corresponds to Java: `WBS_SUMMARY`
    WbsSummary,
    /// Corresponds to Java: `HAMMOCK`
    Hammock,
    /// Corresponds to Java: `START_FLAG`
    StartFlag,
    /// Corresponds to Java: `FINISH_FLAG`
    FinishFlag,
}

impl ActivityType {
    /// All values, in the same order as Java's declaration order.
    pub const ALL: [ActivityType; 9] = [
        ActivityType::TaskDependent,
        ActivityType::ResourceDependent,
        ActivityType::LevelOfEffort,
        ActivityType::StartMilestone,
        ActivityType::FinishMilestone,
        ActivityType::WbsSummary,
        ActivityType::Hammock,
        ActivityType::StartFlag,
        ActivityType::FinishFlag,
    ];

    /// Return an enum instance by ordinal (Java's `ordinal()`); returns `None` when out of bounds.
    ///
    /// No counterpart on the Java side (Java uses the enum constants directly or `values()[n]`).
    pub fn get_instance(value: i32) -> Option<ActivityType> {
        match value {
            0 => Some(ActivityType::TaskDependent),
            1 => Some(ActivityType::ResourceDependent),
            2 => Some(ActivityType::LevelOfEffort),
            3 => Some(ActivityType::StartMilestone),
            4 => Some(ActivityType::FinishMilestone),
            5 => Some(ActivityType::WbsSummary),
            6 => Some(ActivityType::Hammock),
            7 => Some(ActivityType::StartFlag),
            8 => Some(ActivityType::FinishFlag),
            _ => None,
        }
    }
}

impl MpxjEnum for ActivityType {
    /// Return the ordinal corresponding to declaration order.
    ///
    /// Java has no `getValue()`; this corresponds to `Enum.ordinal()`.
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<ActivityType> for i32 {
    fn from(value: ActivityType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_type_round_trips() {
        for item in ActivityType::ALL {
            assert_eq!(ActivityType::get_instance(item.value()), Some(item));
        }
    }

    #[test]
    fn activity_type_out_of_range() {
        assert_eq!(ActivityType::get_instance(-1), None);
        assert_eq!(ActivityType::get_instance(9), None);
    }
}
