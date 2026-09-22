//! Corresponds to Java: `org/mpxj/ProjectTimeFormat.java`

use crate::mpxj_enum::MpxjEnum;
use std::fmt::{self, Display, Formatter};

/// Enum for project time formats.
///
/// Corresponds to Java: `public enum ProjectTimeFormat implements MpxjEnum`
///
/// In Java, `EnumHelper.createTypeArray` builds the int → enum array via reflection;
/// the Rust side does not use reflection and instead uses [`ProjectTimeFormat::ALL`] plus
/// the `match` in [`ProjectTimeFormat::get_instance`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProjectTimeFormat {
    /// 12-hour clock.
    ///
    /// Corresponds to Java: `TWELVE_HOUR(0)`
    TwelveHour = 0,
    /// 24-hour clock.
    ///
    /// Corresponds to Java: `TWENTY_FOUR_HOUR(1)`
    TwentyFourHour = 1,
}

impl ProjectTimeFormat {
    /// All values, sorted ascending by int value.
    ///
    /// Corresponds to Java: `private static final ProjectTimeFormat[] TYPE_VALUES` (excluding the reflection part).
    pub const ALL: [ProjectTimeFormat; 2] = [
        ProjectTimeFormat::TwelveHour,
        ProjectTimeFormat::TwentyFourHour,
    ];

    /// Returns the enum instance for the given int value, falling back to `TWELVE_HOUR` when out of bounds.
    ///
    /// Corresponds to Java: `public static ProjectTimeFormat getInstance(int type)`
    pub fn get_instance(value: i32) -> ProjectTimeFormat {
        match value {
            0 => ProjectTimeFormat::TwelveHour,
            1 => ProjectTimeFormat::TwentyFourHour,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = TWELVE_HOUR.getValue(); }`
            _ => ProjectTimeFormat::TwelveHour,
        }
    }

    /// Corresponds to Java: `public static ProjectTimeFormat getInstance(Number type)`
    ///
    /// In Java, `null` goes through `getInstance(-1)` and ultimately falls back to `TWELVE_HOUR`.
    pub fn get_instance_optional(value: Option<i32>) -> ProjectTimeFormat {
        ProjectTimeFormat::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for ProjectTimeFormat {
    /// Returns the int value associated with this enum.
    ///
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for ProjectTimeFormat {
    /// Corresponds to Java: `@Override public String toString()`
    ///
    /// Java returns `Integer.toString(m_value)`, i.e. the decimal string of the int value.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as i32)
    }
}

impl From<ProjectTimeFormat> for i32 {
    fn from(value: ProjectTimeFormat) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_time_format_round_trips() {
        for item in ProjectTimeFormat::ALL {
            assert_eq!(ProjectTimeFormat::get_instance(item.value()), item);
        }
    }

    #[test]
    fn project_time_format_falls_back_to_twelve_hour() {
        assert_eq!(
            ProjectTimeFormat::get_instance(-1),
            ProjectTimeFormat::TwelveHour
        );
        assert_eq!(
            ProjectTimeFormat::get_instance(2),
            ProjectTimeFormat::TwelveHour
        );
        assert_eq!(
            ProjectTimeFormat::get_instance_optional(None),
            ProjectTimeFormat::TwelveHour
        );
    }

    #[test]
    fn project_time_format_to_string_is_decimal_value() {
        assert_eq!(ProjectTimeFormat::TwelveHour.to_string(), "0");
        assert_eq!(ProjectTimeFormat::TwentyFourHour.to_string(), "1");
    }
}
