//! Corresponds to Java: `org/mpxj/TotalSlackCalculationType.java`

/// Enum for total slack calculation methods.
///
/// Corresponds to Java: `public enum TotalSlackCalculationType`
///
/// The Java enum has no int value, does not implement `MpxjEnum`, and has no `getInstance`;
/// the Rust side keeps the same shape (it does not implement [`crate::mpxj_enum::MpxjEnum`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TotalSlackCalculationType {
    /// Use the smallest slack.
    ///
    /// Corresponds to Java: `SMALLEST_SLACK`
    SmallestSlack,
    /// Use the start slack.
    ///
    /// Corresponds to Java: `START_SLACK`
    StartSlack,
    /// Use the finish slack.
    ///
    /// Corresponds to Java: `FINISH_SLACK`
    FinishSlack,
}

impl TotalSlackCalculationType {
    /// All values, in Java declaration order.
    ///
    /// Corresponds to Java: `TotalSlackCalculationType.values()`
    pub const ALL: [TotalSlackCalculationType; 3] = [
        TotalSlackCalculationType::SmallestSlack,
        TotalSlackCalculationType::StartSlack,
        TotalSlackCalculationType::FinishSlack,
    ];

    /// Returns the enum instance by declaration order (ordinal).
    ///
    /// No corresponding method in Java; added on the Rust side to pair with
    /// [`TotalSlackCalculationType::ALL`] for lookups, corresponding to Java's
    /// `values()[value]` pattern. Java throws `ArrayIndexOutOfBoundsException` out of bounds,
    /// while the Rust side returns `None` per this repo's convention.
    pub fn get_instance(value: i32) -> Option<TotalSlackCalculationType> {
        usize::try_from(value)
            .ok()
            .and_then(|index| TotalSlackCalculationType::ALL.get(index))
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_slack_calculation_type_round_trips() {
        for (index, item) in TotalSlackCalculationType::ALL.iter().enumerate() {
            assert_eq!(
                TotalSlackCalculationType::get_instance(index as i32),
                Some(*item)
            );
        }
    }

    #[test]
    fn total_slack_calculation_type_all_matches_java_declaration_order() {
        assert_eq!(
            TotalSlackCalculationType::ALL,
            [
                TotalSlackCalculationType::SmallestSlack,
                TotalSlackCalculationType::StartSlack,
                TotalSlackCalculationType::FinishSlack
            ]
        );
    }

    #[test]
    fn total_slack_calculation_type_get_instance_is_out_of_range_safe() {
        assert_eq!(TotalSlackCalculationType::get_instance(-1), None);
        assert_eq!(TotalSlackCalculationType::get_instance(3), None);
    }
}
