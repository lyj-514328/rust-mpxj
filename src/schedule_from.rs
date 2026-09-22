//! Corresponds to Java: `org/mpxj/ScheduleFrom.java`

use crate::mpxj_enum::MpxjEnum;
use std::fmt::{self, Display, Formatter};

/// Enum for the scheduling start direction.
///
/// Corresponds to Java: `public enum ScheduleFrom implements MpxjEnum`
///
/// In Java, `EnumHelper.createTypeArray` builds the int → enum array via reflection;
/// the Rust side does not use reflection and instead uses [`ScheduleFrom::ALL`] plus
/// the `match` in [`ScheduleFrom::get_instance`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ScheduleFrom {
    /// Schedule from the start date.
    ///
    /// Corresponds to Java: `START(0)`
    Start = 0,
    /// Schedule from the finish date.
    ///
    /// Corresponds to Java: `FINISH(1)`
    Finish = 1,
}

impl ScheduleFrom {
    /// All values, sorted ascending by int value.
    ///
    /// Corresponds to Java: `private static final ScheduleFrom[] TYPE_VALUES` (excluding the reflection part).
    pub const ALL: [ScheduleFrom; 2] = [ScheduleFrom::Start, ScheduleFrom::Finish];

    /// Returns the enum instance for the given int value, falling back to `START` when out of bounds.
    ///
    /// Corresponds to Java: `public static ScheduleFrom getInstance(int type)`
    pub fn get_instance(value: i32) -> ScheduleFrom {
        match value {
            0 => ScheduleFrom::Start,
            1 => ScheduleFrom::Finish,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = START.getValue(); }`
            _ => ScheduleFrom::Start,
        }
    }

    /// Corresponds to Java: `public static ScheduleFrom getInstance(Number type)`
    ///
    /// In Java, `null` goes through `getInstance(-1)` and ultimately falls back to `START`.
    pub fn get_instance_optional(value: Option<i32>) -> ScheduleFrom {
        ScheduleFrom::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for ScheduleFrom {
    /// Returns the int value associated with this enum.
    ///
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for ScheduleFrom {
    /// Corresponds to Java: `@Override public String toString()`
    ///
    /// Java returns `Integer.toString(m_value)`, i.e. the decimal string of the int value.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as i32)
    }
}

impl From<ScheduleFrom> for i32 {
    fn from(value: ScheduleFrom) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_from_round_trips() {
        for item in ScheduleFrom::ALL {
            assert_eq!(ScheduleFrom::get_instance(item.value()), item);
        }
    }

    #[test]
    fn schedule_from_falls_back_to_start() {
        assert_eq!(ScheduleFrom::get_instance(-1), ScheduleFrom::Start);
        assert_eq!(ScheduleFrom::get_instance(2), ScheduleFrom::Start);
        assert_eq!(
            ScheduleFrom::get_instance_optional(None),
            ScheduleFrom::Start
        );
    }

    #[test]
    fn schedule_from_to_string_is_decimal_value() {
        assert_eq!(ScheduleFrom::Start.to_string(), "0");
        assert_eq!(ScheduleFrom::Finish.to_string(), "1");
    }
}
