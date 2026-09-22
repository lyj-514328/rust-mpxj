//! Corresponds to Java: `org/mpxj/mpx/RecurrenceUtility.java`
//!
//! # Porting trade-offs (`RecurringTask` interface)
//!
//! Java's `getDurationUnits(RecurringTask)` and `getDays(RecurringTask)` take
//! `org.mpxj.RecurringTask` (which provides `getDuration()` and `getWeeklyDay()`).
//! This crate's [`crate::recurring_task::RecurringTask`] lives in the model layer, and the read
//! path does not use these two methods (the Java-side consumer is `MPXWriter`, which is not
//! ported), so the parameters are split by Java's **values** rather than by object:
//!
//! - `getDurationUnits` instead takes the duration directly (`Option<Duration>`), corresponding
//!   to the value of `recurrence.getDuration()` in Java;
//! - `getDays` instead takes [`RecurringData`] (that is, the base-class part of `RecurringTask`),
//!   whose `get_weekly_day` provides the same information as Java's `RecurringTask.getWeeklyDay()`.
//!
//! All other methods correspond one-to-one with Java; the branches where Java throws exceptions
//! (`NumberFormatException`, `DateTimeException`, `ArrayIndexOutOfBoundsException`) return `None`
//! on the Rust side.
//!
//! Skipped Java methods/members: none (including the `RECURRING_TASK_DAY_MASKS` constant and all
//! private static maps).

use chrono::{Datelike, NaiveDate, Weekday};

use crate::common::day_of_week_helper::DayOfWeekHelper;
use crate::duration::Duration;
use crate::project_properties::ProjectProperties;
use crate::recurrence_type::RecurrenceType;
use crate::recurring_data::RecurringData;
use crate::time_unit::TimeUnit;

/// Utility methods for managing recurring (periodic) task data in MPX files.
///
/// Corresponds to Java: `final class RecurrenceUtility` (a private constructor prevents instantiation; the Rust side uses a fieldless type).
pub struct RecurrenceUtility;

impl RecurrenceUtility {
    /// Mask table for date bitmaps in MPX records, corresponding to `DayOfWeekHelper.getValue` (Sunday = 1 … Saturday = 7).
    ///
    /// Corresponds to Java: `public static final int[] RECURRING_TASK_DAY_MASKS`
    pub const RECURRING_TASK_DAY_MASKS: [i32; 8] = [
        0x00,
        0x40, // Sunday
        0x20, // Monday
        0x10, // Tuesday
        0x08, // Wednesday
        0x04, // Thursday
        0x02, // Friday
        0x01, // Saturday
    ];

    /// Convert the "duration value + units integer" from an MPX record into a `Duration` instance.
    ///
    /// Corresponds to Java: `public static Duration getDuration(ProjectProperties properties, Integer durationValue, Integer unitsValue)`
    ///
    /// In an MPX record the duration is expressed in minutes and the units are given by `unitsValue`; a `null` duration returns `None`.
    pub fn get_duration(
        properties: &ProjectProperties,
        duration_value: Option<i32>,
        units_value: Option<i32>,
    ) -> Option<Duration> {
        // Java: durationValue == null -> result = null
        let duration_value = duration_value?;

        let result = Duration::get_instance_int(duration_value, TimeUnit::Minutes);
        let units = Self::get_duration_units_from_value(units_value);

        if result.get_units() != units {
            Some(result.convert_units(units, properties))
        } else {
            Some(result)
        }
    }

    /// Convert a `Duration` instance into an integer minute value that can be written directly to an MPX file.
    ///
    /// Corresponds to Java: `public static Integer getDurationValue(ProjectProperties properties, Duration duration)`
    pub fn get_duration_value(
        properties: &ProjectProperties,
        duration: Option<Duration>,
    ) -> Option<i32> {
        // Java: duration == null -> result = null
        let duration = duration?;

        let duration = if duration.get_units() != TimeUnit::Minutes {
            duration.convert_units(TimeUnit::Minutes, properties)
        } else {
            duration
        };

        // Java: Integer.valueOf((int) duration.getDuration())
        Some(duration.get_duration() as i32)
    }

    /// Convert duration units into an integer value suitable for writing to an MPX file.
    ///
    /// Corresponds to Java: `public static Integer getDurationUnits(RecurringTask recurrence)`
    ///
    /// See the file header for the trade-off: Java obtains the duration from `RecurringTask.getDuration()`, while the Rust side takes the duration directly.
    /// Returns `None` when the units are not in the private `DURATION_UNITS` map (days / weeks / hours / minutes).
    pub fn get_duration_units(duration: Option<Duration>) -> Option<i32> {
        let duration = duration?;
        duration_units_value(duration.get_units())
    }

    /// Convert the units integer of a recurring task in an MPX record into a `TimeUnit`, falling back to days when unrecognized.
    ///
    /// Corresponds to Java: `private static TimeUnit getDurationUnits(Integer value)`
    ///
    /// On the Java side this overloads [`RecurrenceUtility::get_duration_units`]; Rust does not support overloading, so a suffix distinguishes them.
    fn get_duration_units_from_value(value: Option<i32>) -> TimeUnit {
        let result = value
            .and_then(|index| usize::try_from(index).ok())
            .and_then(|index| DURATION_UNITS.get(index).copied());

        // Java: result == null -> TimeUnit.DAYS
        result.unwrap_or(TimeUnit::Days)
    }

    /// Convert a recurrence type integer from an MPX file into a `RecurrenceType`, returning `None` when unrecognized.
    ///
    /// Corresponds to Java: `public static RecurrenceType getRecurrenceType(Integer value)`
    pub fn get_recurrence_type(value: Option<i32>) -> Option<RecurrenceType> {
        match value? {
            1 => Some(RecurrenceType::Daily),
            4 => Some(RecurrenceType::Weekly),
            8 => Some(RecurrenceType::Monthly),
            16 => Some(RecurrenceType::Yearly),
            _ => None,
        }
    }

    /// Convert a `RecurrenceType` into the recurrence type integer used in MPX files, with `None` returning `None`.
    ///
    /// Corresponds to Java: `public static Integer getRecurrenceValue(RecurrenceType value)`
    pub fn get_recurrence_value(value: Option<RecurrenceType>) -> Option<i32> {
        match value? {
            RecurrenceType::Daily => Some(1),
            RecurrenceType::Weekly => Some(4),
            RecurrenceType::Monthly => Some(8),
            RecurrenceType::Yearly => Some(16),
        }
    }

    /// Convert the string form of a day-of-week bitmap into an integer bitmap.
    ///
    /// Corresponds to Java: `public static Integer getDays(String days)`
    ///
    /// The first character is the most significant bit (the semantics of `Integer.parseInt(days, 2)`). Java throws
    /// `NumberFormatException` for invalid input, whereas the Rust side returns `None`.
    pub fn get_days_from_string(days: Option<&str>) -> Option<i32> {
        days.and_then(|days| i32::from_str_radix(days, 2).ok())
    }

    /// Convert the days of a weekly recurrence into a bitmap string.
    ///
    /// Corresponds to Java: `public static String getDays(RecurringTask task)`
    ///
    /// The order comes from [`DayOfWeekHelper::ORDERED_DAYS`] (Sunday → Saturday), so the first character is Sunday.
    /// See the file header for the trade-off: Java takes the value from `RecurringTask.getWeeklyDay()`, while the Rust side takes a [`RecurringData`].
    pub fn get_days(data: &RecurringData) -> String {
        let mut result = String::new();
        for day in DayOfWeekHelper::ORDERED_DAYS {
            result.push(if data.get_weekly_day(day) { '1' } else { '0' });
        }
        result
    }

    /// Convert an MPX day-of-week index into a [`Weekday`]; index 0 (`null` on the Java side) returns `None`.
    ///
    /// Corresponds to Java: `public static DayOfWeek getDay(Integer day)`
    ///
    /// Java throws `ArrayIndexOutOfBoundsException` for an out-of-bounds index, whereas the Rust side returns `None`.
    pub fn get_day(day: Option<i32>) -> Option<Weekday> {
        let index = usize::try_from(day?).ok()?;
        DAY_ARRAY.get(index).copied().flatten()
    }

    /// Convert a [`Weekday`] into an MPX day-of-week index (Monday = 1 … Sunday = 7).
    ///
    /// Corresponds to Java: `public static Integer getDay(DayOfWeek day)`
    ///
    /// On the Java side this overloads [`RecurrenceUtility::get_day`]; Rust does not support overloading, so a suffix distinguishes them.
    pub fn get_day_index(day: Option<Weekday>) -> Option<i32> {
        match day? {
            Weekday::Mon => Some(1),
            Weekday::Tue => Some(2),
            Weekday::Wed => Some(3),
            Weekday::Thu => Some(4),
            Weekday::Fri => Some(5),
            Weekday::Sat => Some(6),
            Weekday::Sun => Some(7),
        }
    }

    /// Return the date for a "yearly, absolute date" recurrence (the year comes from the recurrence's start date).
    ///
    /// Corresponds to Java: `public static LocalDate getYearlyAbsoluteAsDate(RecurringData data)`
    ///
    /// Returns `None` when any of month/day/start date is missing; Java's `LocalDate.of` throws
    /// `DateTimeException` for an invalid date, whereas the Rust side returns `None`.
    pub fn get_yearly_absolute_as_date(data: &RecurringData) -> Option<NaiveDate> {
        let yearly_absolute_day = data.get_day_number()?;
        let yearly_absolute_month = data.get_month_number()?;
        let start_date = data.get_start_date()?;

        NaiveDate::from_ymd_opt(
            start_date.year(),
            u32::try_from(yearly_absolute_month).ok()?,
            u32::try_from(yearly_absolute_day).ok()?,
        )
    }
}

/// Map from the duration units integer in a recurring task record to a `TimeUnit`.
///
/// Corresponds to Java: `private static final TimeUnit[] DURATION_UNITS`
const DURATION_UNITS: [TimeUnit; 4] = [
    TimeUnit::Days,
    TimeUnit::Weeks,
    TimeUnit::Hours,
    TimeUnit::Minutes,
];

/// Reverse map from a `TimeUnit` to the duration units integer in a recurring task record.
///
/// Corresponds to Java: `private static final Map<TimeUnit, Integer> UNITS_MAP` (built by a `DURATION_UNITS` static block)
fn duration_units_value(units: TimeUnit) -> Option<i32> {
    DURATION_UNITS
        .iter()
        .position(|candidate| *candidate == units)
        .map(|index| index as i32)
}

/// Map from an MPX day-of-week index to a [`Weekday`] (index 0 corresponds to Java's `null`).
///
/// Corresponds to Java: `private static final DayOfWeek[] DAY_ARRAY`
const DAY_ARRAY: [Option<Weekday>; 8] = [
    None,
    Some(Weekday::Mon),
    Some(Weekday::Tue),
    Some(Weekday::Wed),
    Some(Weekday::Thu),
    Some(Weekday::Fri),
    Some(Weekday::Sat),
    Some(Weekday::Sun),
];

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::project_file::ProjectFile;

    /// Build a project properties instance for testing: 480 minutes per day, 2400 minutes per week, 20 days per month.
    ///
    /// The time-unit defaults belong to Java's "always calculated fields" and are actually stored in the parent file's project context,
    /// so the parent file must be kept (a weak reference) and the file object returned alongside it for the caller to hold.
    fn properties() -> (Rc<RefCell<ProjectFile>>, ProjectProperties) {
        let file = ProjectFile::new();
        let mut properties = ProjectProperties::new(Rc::downgrade(&file));
        properties.set_minutes_per_day(Some(480));
        properties.set_minutes_per_week(Some(2400));
        properties.set_days_per_month(Some(20));
        (file, properties)
    }

    #[test]
    fn duration_round_trips_through_mpx_values() {
        let (_file, properties) = properties();
        // (MPX units integer, expected units)
        let cases = [
            (0, TimeUnit::Days),
            (1, TimeUnit::Weeks),
            (2, TimeUnit::Hours),
            (3, TimeUnit::Minutes),
        ];

        for (units_value, units) in cases {
            // MPX value -> model value
            let duration =
                RecurrenceUtility::get_duration(&properties, Some(480), Some(units_value))
                    .expect("duration value is not null");
            assert_eq!(duration.get_units(), units);

            // model value -> MPX value -> model value
            let value = RecurrenceUtility::get_duration_value(&properties, Some(duration))
                .expect("duration is not null");
            let units_index = RecurrenceUtility::get_duration_units(Some(duration))
                .expect("units should map to an MPX integer");
            assert_eq!(units_index, units_value);

            let round_tripped =
                RecurrenceUtility::get_duration(&properties, Some(value), Some(units_index))
                    .expect("duration value is not null");
            assert_eq!(round_tripped.get_units(), units);
            assert_eq!(round_tripped, duration);
        }
    }

    #[test]
    fn duration_value_is_expressed_in_minutes() {
        let (_file, properties) = properties();
        assert_eq!(
            RecurrenceUtility::get_duration_value(
                &properties,
                Some(Duration::get_instance(1.0, TimeUnit::Days))
            ),
            Some(480)
        );
        assert_eq!(
            RecurrenceUtility::get_duration_value(
                &properties,
                Some(Duration::get_instance(2.0, TimeUnit::Hours))
            ),
            Some(120)
        );
        assert_eq!(
            RecurrenceUtility::get_duration_value(
                &properties,
                Some(Duration::get_instance(30.0, TimeUnit::Minutes))
            ),
            Some(30)
        );
    }

    #[test]
    fn null_duration_values_are_preserved() {
        let (_file, properties) = properties();
        assert_eq!(RecurrenceUtility::get_duration(&properties, None, Some(0)), None);
        assert_eq!(
            RecurrenceUtility::get_duration_value(&properties, None),
            None
        );
        assert_eq!(RecurrenceUtility::get_duration_units(None), None);
    }

    #[test]
    fn duration_units_default_to_days() {
        let (_file, properties) = properties();
        // Unknown / out-of-bounds / missing units integers all fall back to days (the duration value itself is still converted from minutes to days)
        for units_value in [None, Some(-1), Some(4), Some(100)] {
            let duration = RecurrenceUtility::get_duration(&properties, Some(1), units_value)
                .expect("duration value is not null");
            assert_eq!(duration.get_units(), TimeUnit::Days);
            assert!(Duration::duration_value_equals(
                duration.get_duration(),
                1.0 / 480.0
            ));
        }
    }

    #[test]
    fn duration_units_without_mpx_representation_return_none() {
        // Java: UNITS_MAP contains only days / weeks / hours / minutes
        for units in [
            TimeUnit::Months,
            TimeUnit::Years,
            TimeUnit::Percent,
            TimeUnit::ElapsedDays,
        ] {
            assert_eq!(
                RecurrenceUtility::get_duration_units(Some(Duration::get_instance(1.0, units))),
                None
            );
        }
    }

    #[test]
    fn recurrence_type_round_trips() {
        for recurrence_type in RecurrenceType::ALL {
            let value = RecurrenceUtility::get_recurrence_value(Some(recurrence_type))
                .expect("recurrence type should map to an MPX integer");
            assert_eq!(
                RecurrenceUtility::get_recurrence_type(Some(value)),
                Some(recurrence_type)
            );
        }
    }

    #[test]
    fn recurrence_type_rejects_unknown_values() {
        for value in [None, Some(0), Some(2), Some(17), Some(-1)] {
            assert_eq!(RecurrenceUtility::get_recurrence_type(value), None);
        }
        assert_eq!(RecurrenceUtility::get_recurrence_value(None), None);
    }

    #[test]
    fn days_bitmap_round_trips() {
        let mut data = RecurringData::new();
        data.set_weekly_day(Weekday::Sun, true);
        data.set_weekly_day(Weekday::Sat, true);

        // model value -> MPX value: the first character is Sunday
        let days = RecurrenceUtility::get_days(&data);
        assert_eq!(days, "1000001");

        // MPX value -> model value
        let bitmap = RecurrenceUtility::get_days_from_string(Some(days.as_str()))
            .expect("valid binary bitmap");
        assert_eq!(bitmap, 0b1000001);

        let mut round_tripped = RecurringData::new();
        round_tripped.set_weekly_days_from_bitmap(
            Some(bitmap),
            &RecurrenceUtility::RECURRING_TASK_DAY_MASKS,
        );
        assert!(round_tripped.get_weekly_day(Weekday::Sun));
        assert!(round_tripped.get_weekly_day(Weekday::Sat));
        assert!(!round_tripped.get_weekly_day(Weekday::Mon));
        assert_eq!(RecurrenceUtility::get_days(&round_tripped), days);
    }

    #[test]
    fn days_bitmap_handles_empty_and_invalid_values() {
        let data = RecurringData::new();
        assert_eq!(RecurrenceUtility::get_days(&data), "0000000");
        assert_eq!(RecurrenceUtility::get_days_from_string(Some("0000000")), Some(0));
        // Java: Integer.parseInt handling of null and invalid input (NPE -> none / NumberFormatException)
        assert_eq!(RecurrenceUtility::get_days_from_string(None), None);
        assert_eq!(RecurrenceUtility::get_days_from_string(Some("")), None);
        assert_eq!(RecurrenceUtility::get_days_from_string(Some("1010120")), None);
    }

    #[test]
    fn day_values_round_trip() {
        let expected = [
            (1, Weekday::Mon),
            (2, Weekday::Tue),
            (3, Weekday::Wed),
            (4, Weekday::Thu),
            (5, Weekday::Fri),
            (6, Weekday::Sat),
            (7, Weekday::Sun),
        ];

        for (index, day) in expected {
            assert_eq!(RecurrenceUtility::get_day(Some(index)), Some(day));
            assert_eq!(RecurrenceUtility::get_day_index(Some(day)), Some(index));
        }
    }

    #[test]
    fn day_values_handle_out_of_range_and_missing_values() {
        // Java: DAY_ARRAY[0] is null
        assert_eq!(RecurrenceUtility::get_day(Some(0)), None);
        // Java: throws ArrayIndexOutOfBoundsException
        assert_eq!(RecurrenceUtility::get_day(Some(8)), None);
        assert_eq!(RecurrenceUtility::get_day(Some(-1)), None);
        assert_eq!(RecurrenceUtility::get_day(None), None);
        assert_eq!(RecurrenceUtility::get_day_index(None), None);
    }

    #[test]
    fn yearly_absolute_date_uses_start_date_year() {
        let mut data = RecurringData::new();
        data.set_start_date(Some(
            NaiveDate::from_ymd_opt(2024, 5, 1).expect("valid date"),
        ));
        data.set_day_number(Some(10));
        data.set_month_number(Some(6));

        assert_eq!(
            RecurrenceUtility::get_yearly_absolute_as_date(&data),
            NaiveDate::from_ymd_opt(2024, 6, 10)
        );
    }

    #[test]
    fn yearly_absolute_date_requires_all_components() {
        let start_date = NaiveDate::from_ymd_opt(2024, 5, 1).expect("valid date");

        let mut data = RecurringData::new();
        data.set_start_date(Some(start_date));
        assert_eq!(RecurrenceUtility::get_yearly_absolute_as_date(&data), None);

        let mut data = RecurringData::new();
        data.set_day_number(Some(10));
        data.set_month_number(Some(6));
        assert_eq!(RecurrenceUtility::get_yearly_absolute_as_date(&data), None);

        // Java: LocalDate.of throws DateTimeException for an invalid date
        let mut data = RecurringData::new();
        data.set_start_date(Some(start_date));
        data.set_day_number(Some(31));
        data.set_month_number(Some(2));
        assert_eq!(RecurrenceUtility::get_yearly_absolute_as_date(&data), None);

        let mut data = RecurringData::new();
        data.set_start_date(Some(start_date));
        data.set_day_number(Some(1));
        data.set_month_number(Some(13));
        assert_eq!(RecurrenceUtility::get_yearly_absolute_as_date(&data), None);
    }

    #[test]
    fn recurring_task_day_masks_match_java() {
        assert_eq!(
            RecurrenceUtility::RECURRING_TASK_DAY_MASKS,
            [0x00, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01]
        );
    }
}
