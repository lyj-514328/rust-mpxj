//! Corresponds to Java: `org/mpxj/SchedulingProgressedActivities.java`

/// Enum for how progressed activities are scheduled.
///
/// Corresponds to Java: `public enum SchedulingProgressedActivities`
///
/// The Java enum has no int value, does not implement `MpxjEnum`, and has no `getInstance`;
/// the Rust side keeps the same shape (it does not implement [`crate::mpxj_enum::MpxjEnum`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SchedulingProgressedActivities {
    /// Retain logic.
    ///
    /// Corresponds to Java: `RETAINED_LOGIC`
    RetainedLogic,
    /// Progress override.
    ///
    /// Corresponds to Java: `PROGRESS_OVERRIDE`
    ProgressOverride,
    /// Actual dates.
    ///
    /// Corresponds to Java: `ACTUAL_DATES`
    ActualDates,
}

impl SchedulingProgressedActivities {
    /// All values, in Java declaration order.
    ///
    /// Corresponds to Java: `SchedulingProgressedActivities.values()`
    pub const ALL: [SchedulingProgressedActivities; 3] = [
        SchedulingProgressedActivities::RetainedLogic,
        SchedulingProgressedActivities::ProgressOverride,
        SchedulingProgressedActivities::ActualDates,
    ];

    /// Returns the enum instance by declaration order (ordinal).
    ///
    /// No corresponding method in Java; added on the Rust side to pair with
    /// [`SchedulingProgressedActivities::ALL`] for lookups, corresponding to Java's
    /// `values()[value]` pattern. Java throws `ArrayIndexOutOfBoundsException` out of bounds,
    /// while the Rust side returns `None` per this repo's convention.
    pub fn get_instance(value: i32) -> Option<SchedulingProgressedActivities> {
        usize::try_from(value)
            .ok()
            .and_then(|index| SchedulingProgressedActivities::ALL.get(index))
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduling_progressed_activities_round_trips() {
        for (index, item) in SchedulingProgressedActivities::ALL.iter().enumerate() {
            assert_eq!(
                SchedulingProgressedActivities::get_instance(index as i32),
                Some(*item)
            );
        }
    }

    #[test]
    fn scheduling_progressed_activities_all_matches_java_declaration_order() {
        assert_eq!(
            SchedulingProgressedActivities::ALL,
            [
                SchedulingProgressedActivities::RetainedLogic,
                SchedulingProgressedActivities::ProgressOverride,
                SchedulingProgressedActivities::ActualDates
            ]
        );
    }

    #[test]
    fn scheduling_progressed_activities_get_instance_is_out_of_range_safe() {
        assert_eq!(SchedulingProgressedActivities::get_instance(-1), None);
        assert_eq!(SchedulingProgressedActivities::get_instance(3), None);
    }
}
