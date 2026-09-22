//! Corresponds to Java: `org/mpxj/RelationshipLagCalendar.java`

/// Enum for the calendar used to calculate relationship lag.
///
/// Corresponds to Java: `public enum RelationshipLagCalendar`
///
/// The Java enum has no int value, does not implement `MpxjEnum`, and has no `getInstance`;
/// the Rust side keeps the same shape (it does not implement [`crate::mpxj_enum::MpxjEnum`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RelationshipLagCalendar {
    /// Corresponds to Java: `PREDECESSOR`
    Predecessor,
    /// Corresponds to Java: `SUCCESSOR`
    Successor,
    /// Corresponds to Java: `PROJECT_DEFAULT`
    ProjectDefault,
    /// Corresponds to Java: `TWENTY_FOUR_HOUR`
    TwentyFourHour,
}

impl RelationshipLagCalendar {
    /// All values, in Java declaration order.
    ///
    /// Corresponds to Java: `RelationshipLagCalendar.values()`
    pub const ALL: [RelationshipLagCalendar; 4] = [
        RelationshipLagCalendar::Predecessor,
        RelationshipLagCalendar::Successor,
        RelationshipLagCalendar::ProjectDefault,
        RelationshipLagCalendar::TwentyFourHour,
    ];

    /// Returns the enum instance by declaration order (ordinal).
    ///
    /// No corresponding method in Java; added on the Rust side to pair with [`RelationshipLagCalendar::ALL`] for lookups,
    /// corresponding to Java's `values()[value]` pattern. Java throws
    /// `ArrayIndexOutOfBoundsException` out of bounds, while the Rust side returns `None` per this repo's convention.
    pub fn get_instance(value: i32) -> Option<RelationshipLagCalendar> {
        usize::try_from(value)
            .ok()
            .and_then(|index| RelationshipLagCalendar::ALL.get(index))
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relationship_lag_calendar_round_trips() {
        for (index, item) in RelationshipLagCalendar::ALL.iter().enumerate() {
            assert_eq!(
                RelationshipLagCalendar::get_instance(index as i32),
                Some(*item)
            );
        }
    }

    #[test]
    fn relationship_lag_calendar_all_matches_java_declaration_order() {
        assert_eq!(
            RelationshipLagCalendar::ALL,
            [
                RelationshipLagCalendar::Predecessor,
                RelationshipLagCalendar::Successor,
                RelationshipLagCalendar::ProjectDefault,
                RelationshipLagCalendar::TwentyFourHour
            ]
        );
    }

    #[test]
    fn relationship_lag_calendar_get_instance_is_out_of_range_safe() {
        assert_eq!(RelationshipLagCalendar::get_instance(-1), None);
        assert_eq!(RelationshipLagCalendar::get_instance(4), None);
    }
}
