//! Corresponds to Java: `org/mpxj/RecurrenceType.java`

use crate::mpxj_enum::MpxjEnum;
use std::fmt::{self, Display, Formatter};

/// Enum of recurrence (periodic) types.
///
/// Corresponds to Java: `public enum RecurrenceType implements MpxjEnum`
///
/// Java uses `EnumHelper.createTypeArray(RecurrenceType.class, 13)` to reflectively generate
/// the lookup array: the enum has only 4 values but the array length is `4 + 13 = 17`, so
/// indices 2, 3, 5, 6, 7, 9…15 are all holes. `getInstance(int)` falls back to `DAILY` when
/// `type < 1 || type >= 17`, and returns `null` when it hits a hole.
///
/// The Rust side does not use reflection and instead uses [`RecurrenceType::ALL`] plus a
/// `match` in [`RecurrenceType::get_instance`]; to faithfully express Java's behavior of
/// possibly returning `null`, this method returns `Option<RecurrenceType>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RecurrenceType {
    /// Daily.
    ///
    /// Corresponds to Java: `DAILY(1, "Daily")`
    Daily = 1,
    /// Weekly.
    ///
    /// Corresponds to Java: `WEEKLY(4, "Weekly")`
    Weekly = 4,
    /// Monthly.
    ///
    /// Corresponds to Java: `MONTHLY(8, "Monthly")`
    Monthly = 8,
    /// Yearly.
    ///
    /// Corresponds to Java: `YEARLY(16, "Yearly")`
    Yearly = 16,
}

impl RecurrenceType {
    /// All values, ordered by ascending int value.
    ///
    /// Corresponds to Java: `private static final RecurrenceType[] TYPE_VALUES` (excluding the reflection part and the holes).
    pub const ALL: [RecurrenceType; 4] = [
        RecurrenceType::Daily,
        RecurrenceType::Weekly,
        RecurrenceType::Monthly,
        RecurrenceType::Yearly,
    ];

    /// Length of Java's lookup array: `EnumHelper.createTypeArray(RecurrenceType.class, 13)`
    /// = enum count 4 + offset 13.
    const TYPE_VALUES_LENGTH: i32 = 17;

    /// Return an enum instance for an int value.
    ///
    /// Out-of-range values (`type < 1 || type >= 17`) fall back to `DAILY`; values that fall in
    /// holes of the lookup array correspond to Java's `null` and return `None` here.
    ///
    /// Corresponds to Java: `public static RecurrenceType getInstance(int type)`
    pub fn get_instance(value: i32) -> Option<RecurrenceType> {
        match value {
            1 => Some(RecurrenceType::Daily),
            4 => Some(RecurrenceType::Weekly),
            8 => Some(RecurrenceType::Monthly),
            16 => Some(RecurrenceType::Yearly),
            // Java: `if (type < 1 || type >= TYPE_VALUES.length) { type = DAILY.getValue(); }`
            _ if value < 1 || value >= RecurrenceType::TYPE_VALUES_LENGTH => {
                Some(RecurrenceType::Daily)
            }
            // Java: a hole in the lookup array, returns null.
            _ => None,
        }
    }

    /// Return the name corresponding to this recurrence type (e.g. `"Daily"`).
    ///
    /// Corresponds to Java: `private final String m_name;` exposed via `toString()`.
    pub fn get_name(&self) -> &'static str {
        match self {
            RecurrenceType::Daily => "Daily",
            RecurrenceType::Weekly => "Weekly",
            RecurrenceType::Monthly => "Monthly",
            RecurrenceType::Yearly => "Yearly",
        }
    }
}

impl MpxjEnum for RecurrenceType {
    /// Return the int value associated with this enum.
    ///
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for RecurrenceType {
    /// Corresponds to Java: `@Override public String toString()`
    ///
    /// Java returns the field `m_name`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl From<RecurrenceType> for i32 {
    fn from(value: RecurrenceType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recurrence_type_round_trips() {
        for item in RecurrenceType::ALL {
            assert_eq!(RecurrenceType::get_instance(item.value()), Some(item));
        }
    }

    #[test]
    fn recurrence_type_falls_back_to_daily() {
        assert_eq!(RecurrenceType::get_instance(0), Some(RecurrenceType::Daily));
        assert_eq!(RecurrenceType::get_instance(-1), Some(RecurrenceType::Daily));
        assert_eq!(RecurrenceType::get_instance(17), Some(RecurrenceType::Daily));
        assert_eq!(RecurrenceType::get_instance(100), Some(RecurrenceType::Daily));
    }

    #[test]
    fn recurrence_type_returns_none_for_lookup_array_holes() {
        // The offset 13 of createTypeArray on the Java side creates holes, and getInstance returns null.
        for value in [2, 3, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15] {
            assert_eq!(RecurrenceType::get_instance(value), None);
        }
    }

    #[test]
    fn recurrence_type_to_string_matches_java_names() {
        assert_eq!(RecurrenceType::Daily.to_string(), "Daily");
        assert_eq!(RecurrenceType::Weekly.to_string(), "Weekly");
        assert_eq!(RecurrenceType::Monthly.to_string(), "Monthly");
        assert_eq!(RecurrenceType::Yearly.to_string(), "Yearly");
    }
}
