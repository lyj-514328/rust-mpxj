//! Corresponds to Java: `org/mpxj/TaskType.java`

/// Enum for task types.
///
/// Corresponds to Java: `public enum TaskType`
///
/// The Java enum has no int value, does not implement `MpxjEnum`, and has no `getInstance`
/// (the int ↔ `TaskType` mapping is done in Java by helper classes such as `org.mpxj.mpp.TaskTypeHelper`);
/// the Rust side keeps the same shape (it does not implement [`crate::mpxj_enum::MpxjEnum`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TaskType {
    /// Fixed units.
    ///
    /// Corresponds to Java: `FIXED_UNITS`
    FixedUnits,
    /// Fixed duration.
    ///
    /// Corresponds to Java: `FIXED_DURATION`
    FixedDuration,
    /// Fixed work.
    ///
    /// Corresponds to Java: `FIXED_WORK`
    FixedWork,
    /// Fixed duration and units.
    ///
    /// Corresponds to Java: `FIXED_DURATION_AND_UNITS`
    FixedDurationAndUnits,
}

impl TaskType {
    /// All values, in Java declaration order.
    ///
    /// Corresponds to Java: `TaskType.values()`
    pub const ALL: [TaskType; 4] = [
        TaskType::FixedUnits,
        TaskType::FixedDuration,
        TaskType::FixedWork,
        TaskType::FixedDurationAndUnits,
    ];

    /// Returns the enum instance by declaration order (ordinal).
    ///
    /// No corresponding method in Java; added on the Rust side to pair with [`TaskType::ALL`]
    /// for lookups, corresponding to Java's `values()[value]` pattern. Java throws
    /// `ArrayIndexOutOfBoundsException` out of bounds, while the Rust side returns `None` per this repo's convention.
    pub fn get_instance(value: i32) -> Option<TaskType> {
        usize::try_from(value)
            .ok()
            .and_then(|index| TaskType::ALL.get(index))
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_type_round_trips() {
        for (index, item) in TaskType::ALL.iter().enumerate() {
            assert_eq!(TaskType::get_instance(index as i32), Some(*item));
        }
    }

    #[test]
    fn task_type_all_matches_java_declaration_order() {
        assert_eq!(
            TaskType::ALL,
            [
                TaskType::FixedUnits,
                TaskType::FixedDuration,
                TaskType::FixedWork,
                TaskType::FixedDurationAndUnits
            ]
        );
    }

    #[test]
    fn task_type_get_instance_is_out_of_range_safe() {
        assert_eq!(TaskType::get_instance(-1), None);
        assert_eq!(TaskType::get_instance(4), None);
    }
}
