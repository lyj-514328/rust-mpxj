//! Corresponds to Java: `org/mpxj/TimescaleUnits.java`

use std::fmt::{self, Display, Formatter};

/// Enum for the units used by the Gantt chart timescale.
///
/// Corresponds to Java: `public enum TimescaleUnits`
///
/// On the Java side this enum has no int value, does not implement `MpxjEnum`, and has no `getInstance`
/// (the int <-> `TimescaleUnits` mapping is done in Java by `org.mpxj.mpp.TimescaleUnitsHelper`
/// ); the Rust side keeps the same shape (it does not implement [`crate::mpxj_enum::MpxjEnum`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TimescaleUnits {
    /// Corresponds to Java: `NONE("None")`
    None,
    /// Corresponds to Java: `MINUTES("Minutes")`
    Minutes,
    /// Corresponds to Java: `HOURS("Hours")`
    Hours,
    /// Corresponds to Java: `DAYS("Days")`
    Days,
    /// Corresponds to Java: `WEEKS("Weeks")`
    Weeks,
    /// Corresponds to Java: `THIRDS_OF_MONTHS("Thirds of Months")`
    ThirdsOfMonths,
    /// Corresponds to Java: `MONTHS("Months")`
    Months,
    /// Corresponds to Java: `QUARTERS("Quarters")`
    Quarters,
    /// Corresponds to Java: `HALF_YEARS("Half Years")`
    HalfYears,
    /// Corresponds to Java: `YEARS("Years")`
    Years,
}

impl TimescaleUnits {
/// All values, arranged in Java declaration order.
    ///
    /// Corresponds to Java: `TimescaleUnits.values()`
    pub const ALL: [TimescaleUnits; 10] = [
        TimescaleUnits::None,
        TimescaleUnits::Minutes,
        TimescaleUnits::Hours,
        TimescaleUnits::Days,
        TimescaleUnits::Weeks,
        TimescaleUnits::ThirdsOfMonths,
        TimescaleUnits::Months,
        TimescaleUnits::Quarters,
        TimescaleUnits::HalfYears,
        TimescaleUnits::Years,
    ];

/// Get the enum instance by declaration order (ordinal).
    ///
/// The Java side has no such method; the Rust side adds it to pair with [`TimescaleUnits::ALL`] for table lookup,
/// corresponding to the `values()[value]` form in Java. Java throws
/// `ArrayIndexOutOfBoundsException` when out of bounds; the Rust side returns `None` per this repository's convention.
    pub fn get_instance(value: i32) -> Option<TimescaleUnits> {
        usize::try_from(value)
            .ok()
            .and_then(|index| TimescaleUnits::ALL.get(index))
            .copied()
    }

/// Return the name of this timescale unit (not localized).
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            TimescaleUnits::None => "None",
            TimescaleUnits::Minutes => "Minutes",
            TimescaleUnits::Hours => "Hours",
            TimescaleUnits::Days => "Days",
            TimescaleUnits::Weeks => "Weeks",
            TimescaleUnits::ThirdsOfMonths => "Thirds of Months",
            TimescaleUnits::Months => "Months",
            TimescaleUnits::Quarters => "Quarters",
            TimescaleUnits::HalfYears => "Half Years",
            TimescaleUnits::Years => "Years",
        }
    }
}

impl Display for TimescaleUnits {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java returns the field `m_name`, the same as `getName()`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timescale_units_round_trips() {
        for (index, item) in TimescaleUnits::ALL.iter().enumerate() {
            assert_eq!(TimescaleUnits::get_instance(index as i32), Some(*item));
        }
    }

    #[test]
    fn timescale_units_all_matches_java_declaration_order() {
        assert_eq!(
            TimescaleUnits::ALL,
            [
                TimescaleUnits::None,
                TimescaleUnits::Minutes,
                TimescaleUnits::Hours,
                TimescaleUnits::Days,
                TimescaleUnits::Weeks,
                TimescaleUnits::ThirdsOfMonths,
                TimescaleUnits::Months,
                TimescaleUnits::Quarters,
                TimescaleUnits::HalfYears,
                TimescaleUnits::Years
            ]
        );
    }

    #[test]
    fn timescale_units_get_instance_is_out_of_range_safe() {
        assert_eq!(TimescaleUnits::get_instance(-1), None);
        assert_eq!(TimescaleUnits::get_instance(10), None);
    }

    #[test]
    fn timescale_units_names_and_to_string_match_java() {
        for item in TimescaleUnits::ALL {
            assert_eq!(item.to_string(), item.get_name());
        }
        assert_eq!(TimescaleUnits::None.get_name(), "None");
        assert_eq!(TimescaleUnits::ThirdsOfMonths.get_name(), "Thirds of Months");
        assert_eq!(TimescaleUnits::HalfYears.get_name(), "Half Years");
        assert_eq!(TimescaleUnits::Years.to_string(), "Years");
    }
}
