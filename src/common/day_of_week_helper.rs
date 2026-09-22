//! Corresponds to Java: `org/mpxj/common/DayOfWeekHelper.java`

use chrono::Weekday;

/// Utility methods for handling `DayOfWeek` (represented by [`Weekday`] on the Rust side).
///
/// Corresponds to Java: `public final class DayOfWeekHelper`
///
/// Porting notes:
/// - Java's `getValue` throws `NullPointerException` for null; the Rust parameter is a
///   non-nullable [`Weekday`], and the trailing `return 0` of its `switch` is unreachable under
///   an exhaustive enum, so the Rust exhaustive `match` needs no such branch.
/// - In `getInstance(int)` the Java `--type` wraps around on overflow; the Rust side uses
///   `wrapping_sub` to stay consistent.
///
/// Skipped Java methods: none.
pub struct DayOfWeekHelper;

impl DayOfWeekHelper {
    /// Corresponds to Java: `public static DayOfWeek getInstance(int type)`
    ///
    /// 1 means Sunday and 7 means Saturday; returns `None` when out of bounds.
    pub fn get_instance(r#type: i32) -> Option<Weekday> {
        let index = r#type.wrapping_sub(1);

        if index < 0 || index as usize >= Self::ORDERED_DAYS.len() {
            return None;
        }

        Some(Self::ORDERED_DAYS[index as usize])
    }

    /// Corresponds to Java: `public static int getValue(DayOfWeek day)`
    ///
    /// Sunday returns 1 and Saturday returns 7 (different from the ordinal built into `java.time.DayOfWeek`).
    pub fn get_value(day: Weekday) -> i32 {
        match day {
            Weekday::Sun => 1,
            Weekday::Mon => 2,
            Weekday::Tue => 3,
            Weekday::Wed => 4,
            Weekday::Thu => 5,
            Weekday::Fri => 6,
            Weekday::Sat => 7,
        }
    }

    /// Corresponds to Java: `public static final DayOfWeek[] ORDERED_DAYS`
    ///
    /// Mapping table from int value to enum; index 0 corresponds to int value 1 (Sunday).
    pub const ORDERED_DAYS: [Weekday; 7] = [
        Weekday::Sun,
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
    ];
}
