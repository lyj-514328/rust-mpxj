//! Corresponds to Java: `org/mpxj/common/LocalDateTimeHelper.java`

use std::cmp::Ordering;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

use crate::duration::Duration;
use crate::project_calendar::ProjectCalendar;
use crate::time_unit::TimeUnit;

/// Utility methods for handling `LocalDateTime` (on the Rust side [`NaiveDateTime`]).
///
/// Corresponds to Java: `public final class LocalDateTimeHelper`
///
/// Porting notes:
/// - `START_DATE_NA` / `END_DATE_NA` are `static final` constants in Java; chrono's date-time
///   construction is not `const fn`, so the Rust side changes them to the two functions
///   [`LocalDateTimeHelper::start_date_na`] / [`LocalDateTimeHelper::end_date_na`].
/// - The `DateTimeFormatter` of `parseBest(DateTimeFormatter, String)` has no equivalent in
///   chrono, so chrono's format string `&str` is used instead; Java throws
///   `DateTimeParseException` on parse failure, Rust returns `None`.
/// - The `org.mpxj.Duration`, `ProjectCalendar`, and `TimeUnit` used by `getVariance`
///   correspond on the Rust side to [`crate::duration::Duration`],
///   [`crate::project_calendar::ProjectCalendar`], and [`crate::time_unit::TimeUnit`].
///
/// Unported methods: none.
pub struct LocalDateTimeHelper;

impl LocalDateTimeHelper {
    /// Corresponds to Java: `public static LocalDateTime getDayStartDate(LocalDateTime date)`
    ///
    /// Returns 00:00:00 of the same day; returns `None` for null.
    pub fn get_day_start_date(date: Option<NaiveDateTime>) -> Option<NaiveDateTime> {
        date.map(|date| NaiveDateTime::new(date.date(), NaiveTime::MIN))
    }

    /// Corresponds to Java: `public static int compare(LocalDateTime startDate, LocalDateTime endDate, LocalDateTime targetDate)`
    ///
    /// Returns -1 if the target date is before the range, 1 if after, and 0 if within the range.
    pub fn compare_with_range(
        start_date: NaiveDateTime,
        end_date: NaiveDateTime,
        target_date: NaiveDateTime,
    ) -> i32 {
        if target_date < start_date {
            -1
        } else if target_date > end_date {
            1
        } else {
            0
        }
    }

    /// Corresponds to Java: `public static int compare(LocalDateTime startRange1, LocalDateTime endRange1, LocalDateTime startRange2, LocalDateTime endRange2)`
    ///
    /// Returns -1 if range 1 ends before range 2 starts, 1 if range 1 starts after range 2
    /// ends, and 0 if they intersect.
    pub fn compare_ranges(
        start_range1: NaiveDateTime,
        end_range1: NaiveDateTime,
        start_range2: NaiveDateTime,
        end_range2: NaiveDateTime,
    ) -> i32 {
        // Range 1 ends before range 2 starts
        if end_range1 <= start_range2 {
            return -1;
        }

        // Range 1 starts after range 2 ends
        if start_range1 >= end_range2 {
            return 1;
        }

        0
    }

    /// Corresponds to Java: `public static int compare(LocalDateTime d1, LocalDateTime d2)`
    ///
    /// Handles null: both null returns 0, only left null returns 1, only right null returns -1.
    pub fn compare(d1: Option<NaiveDateTime>, d2: Option<NaiveDateTime>) -> i32 {
        match (d1, d2) {
            (None, None) => 0,
            (None, Some(_)) => 1,
            (Some(_), None) => -1,
            (Some(d1), Some(d2)) => match d1.cmp(&d2) {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            },
        }
    }

    /// Corresponds to Java: `public static LocalDateTime min(LocalDateTime d1, LocalDateTime d2)`
    ///
    /// A non-null value is always earlier than a null value.
    pub fn min(d1: Option<NaiveDateTime>, d2: Option<NaiveDateTime>) -> Option<NaiveDateTime> {
        match (d1, d2) {
            (None, d2) => d2,
            (d1, None) => d1,
            (Some(d1), Some(d2)) => Some(if d1 < d2 { d1 } else { d2 }),
        }
    }

    /// Corresponds to Java: `public static LocalDateTime max(LocalDateTime d1, LocalDateTime d2)`
    ///
    /// A non-null value is always later than a null value.
    pub fn max(d1: Option<NaiveDateTime>, d2: Option<NaiveDateTime>) -> Option<NaiveDateTime> {
        match (d1, d2) {
            (None, d2) => d2,
            (d1, None) => d1,
            (Some(d1), Some(d2)) => Some(if d1 > d2 { d1 } else { d2 }),
        }
    }

    /// Corresponds to Java: `public static LocalDateTime parseBest(DateTimeFormatter format, String value)`
    ///
    /// Java uses `format.parseBest` to try `LocalDateTime` first, then `LocalDate` on failure
    /// and converts it to 00:00 of the same day; the Rust side uses `format` as a chrono format
    /// string to reproduce the same two-level fallback.
    pub fn parse_best(format: &str, value: &str) -> Option<NaiveDateTime> {
        if let Ok(date_time) = NaiveDateTime::parse_from_str(value, format) {
            return Some(date_time);
        }

        match NaiveDate::parse_from_str(value, format) {
            Ok(date) => Some(NaiveDateTime::new(date, NaiveTime::MIN)),
            Err(_) => None,
        }
    }

    /// Corresponds to Java: `public static Duration getVariance(ProjectCalendar calendar, LocalDateTime date1, LocalDateTime date2, TimeUnit format)`
    ///
    /// Computes the working-time difference between two dates under the given calendar; returns
    /// a zero duration when any argument is null.
    pub fn get_variance(
        calendar: Option<&ProjectCalendar>,
        date1: Option<NaiveDateTime>,
        date2: Option<NaiveDateTime>,
        format: TimeUnit,
    ) -> Duration {
        match (calendar, date1, date2) {
            (Some(calendar), Some(date1), Some(date2)) => calendar.get_work(date1, date2, format),
            _ => Duration::get_instance(0.0, format),
        }
    }

    /// Corresponds to Java: `public static final LocalDateTime START_DATE_NA`
    ///
    /// The range start date representing NA: 1984-01-01 00:00:00.
    pub fn start_date_na() -> NaiveDateTime {
        match (
            NaiveDate::from_ymd_opt(1984, 1, 1),
            NaiveTime::from_hms_opt(0, 0, 0),
        ) {
            (Some(date), Some(time)) => NaiveDateTime::new(date, time),
            _ => NaiveDateTime::default(),
        }
    }

    /// Corresponds to Java: `public static final LocalDateTime END_DATE_NA`
    ///
    /// The range end date representing NA: 2049-12-31 23:59:00 (the value recognized by older
    /// versions of MS Project; see the Java comments for details).
    pub fn end_date_na() -> NaiveDateTime {
        match (
            NaiveDate::from_ymd_opt(2049, 12, 31),
            NaiveTime::from_hms_opt(23, 59, 0),
        ) {
            (Some(date), Some(time)) => NaiveDateTime::new(date, time),
            _ => NaiveDateTime::default(),
        }
    }
}
