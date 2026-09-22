//! Corresponds to Java: `org/mpxj/common/LocalDateHelper.java`

use std::cmp::Ordering;

use chrono::{Local, Months, NaiveDate, NaiveDateTime, NaiveTime};

/// Utility methods for handling `LocalDate` (represented by [`NaiveDate`] on the Rust side).
///
/// Corresponds to Java: `public final class LocalDateHelper`
///
/// Porting notes:
/// - In Java `TWO_DIGIT_YEAR_BASE_DATE` is a `static final` constant
///   (`LocalDate.now().minusYears(80)`); chrono's date construction is not a `const fn`, so the
///   Rust side uses the function [`LocalDateHelper::two_digit_year_base_date`] instead, still
///   computed from the current date at call time.
/// - Java's `compareTo` only guarantees the sign, while the Rust side uniformly returns -1 / 0 / 1.
///
/// Skipped Java methods: none.
pub struct LocalDateHelper;

impl LocalDateHelper {
    /// Corresponds to Java: `public static LocalDateTime getLocalDateTime(LocalDate date)`
    ///
    /// Corresponds to `date.atStartOfDay()`; null returns `None`.
    pub fn get_local_date_time(date: Option<NaiveDate>) -> Option<NaiveDateTime> {
        date.map(|date| NaiveDateTime::new(date, NaiveTime::MIN))
    }

    /// Corresponds to Java: `public static LocalDate getLocalDate(LocalDateTime date)`
    ///
    /// Corresponds to `date.toLocalDate()`; null returns `None`.
    pub fn get_local_date(date: Option<NaiveDateTime>) -> Option<NaiveDate> {
        date.map(|date| date.date())
    }

    /// Corresponds to Java: `public static int compare(LocalDate d1, LocalDate d2)`
    ///
    /// Handles null: both null returns 0, only the left null returns 1, only the right null returns -1.
    pub fn compare(d1: Option<NaiveDate>, d2: Option<NaiveDate>) -> i32 {
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

    /// Corresponds to Java: `public static int compare(LocalDate startDate, LocalDate endDate, LocalDate targetDate)`
    ///
    /// Determines whether the target date falls within the given range: before the range returns -1, after the range returns 1, within the range returns 0.
    pub fn compare_with_range(
        start_date: NaiveDate,
        end_date: NaiveDate,
        target_date: NaiveDate,
    ) -> i32 {
        if target_date < start_date {
            -1
        } else if target_date > end_date {
            1
        } else {
            0
        }
    }

    /// Corresponds to Java: `public static LocalDateTime getDayEndDate(LocalDate date)`
    ///
    /// The Java comment says 23:59:59.999, but the actual implementation uses `LocalTime.of(23, 59, 59)`; translated here according to the implementation.
    pub fn get_day_end_date(date: Option<NaiveDate>) -> Option<NaiveDateTime> {
        let date = date?;
        NaiveTime::from_hms_opt(23, 59, 59).map(|time| NaiveDateTime::new(date, time))
    }

    /// Corresponds to Java: `public static final LocalDate TWO_DIGIT_YEAR_BASE_DATE`
    ///
    /// Equivalent to `LocalDate.now().minusYears(80)` (Java uses the system default time zone; the Rust side uses [`Local`]).
    pub fn two_digit_year_base_date() -> NaiveDate {
        match Local::now()
            .date_naive()
            .checked_sub_months(Months::new(80 * 12))
        {
            Some(date) => date,
            None => NaiveDate::MIN,
        }
    }
}
