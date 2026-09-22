//! Corresponds to Java: `org/mpxj/RecurringData.java`
//!
//! Description and date expansion algorithm for recurring events (recurring tasks / recurring calendar exceptions): by day / week / month / year frequency
//! expands a series of occurrence dates.
//!
//! # Skipped Java Mechanisms
//!
//! This class does not involve reflection, JDBC, XML/JAXB, GUI, `javax.*`, or `Serializable`; **no Java methods are skipped**.
//! The Java-specific facilities used in `toString()` are replaced as follows:
//!
//! - `ByteArrayOutputStream` + `PrintWriter` → [`std::fmt::Write`] (i.e. `write!`).
//! - `DateFormatSymbols` (outputs weekday / month names following the default locale) → fixed English name constants
//! [`WEEKDAY_NAMES`] / [`MONTH_NAMES`], with values matching `DateFormatSymbols` under the English locale
//! (`getWeekdays()` is an 8-element array with "index 0 an empty string, 1 = Sunday … 7 = Saturday",
//! and `getMonths()` is a 13-element array with "index 0 = January … 11 = December, 12 an empty string").
//!
//! # Differences from Java
//!
//! - Java fields may be `null`; the Rust side uniformly uses `Option<T>`. Java throws when a required field is missing;
//!   `NullPointerException` / `DateTimeException` / `ArrayIndexOutOfBoundsException`,
//! the Rust side does not panic: once date expansion cannot proceed further it ends the expansion (keeping the dates already generated),
//! and the textual methods use `"null"` as a placeholder; see each method's documentation for details.
//! - Java's `getDates()` returns a reference to the internal array; the Rust-side [`RecurringData::get_dates`] returns a cloned
//! `Vec<NaiveDate>` (the Java-side array is mutable, the Rust side is read-only).
//! - Java's `getCalculatedFirstDate()` / `getCalculatedLastDate()` throw when no date has been generated;
//! `ArrayIndexOutOfBoundsException`, whereas the Rust side returns the sentinel values `NaiveDate::MIN` / `NaiveDate::MAX` respectively
//! (used together with `isValid()` this avoids relying on the sentinel values).
//! - Java's `setWeeklyDaysFromBitmap(Integer, int[])` throws when the `masks` array is out of bounds
//! `ArrayIndexOutOfBoundsException`; the Rust side treats it as mask 0 (i.e. the bit is considered unset).
//! - Java's `LocalDate` operations (`plusDays` / `plusMonths` / `plusYears`) throw `DateTimeException` on out of bounds;
//! the Rust side uses the `checked_*` family and ends the expansion.

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};

use chrono::{Datelike, Months, NaiveDate, TimeDelta, Weekday};

use crate::common::day_of_week_helper::DayOfWeekHelper;
use crate::recurrence_type::RecurrenceType;

/// The order of `java.time.DayOfWeek.values()` (Monday → Sunday).
///
/// Corresponds to Java: the `for (DayOfWeek day : DayOfWeek.values())` appearing in `RecurringData`.
/// Note this differs from [`DayOfWeekHelper::ORDERED_DAYS`] (Sunday first, used to index by the int value).
const JAVA_DAY_OF_WEEK_ORDER: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];

/// Corresponds to Java: `private static final String[] ORDINAL` — ordinals used to generate debug output.
const ORDINAL: [Option<&str>; 4] = [None, Some("every"), Some("every other"), Some("every 3rd")];

/// Corresponds to Java: `private static final String[] DAY_ORDINAL` — the "which occurrence" names used to generate debug output.
const DAY_ORDINAL: [Option<&str>; 6] = [
    None,
    Some("First"),
    Some("Second"),
    Some("Third"),
    Some("Fourth"),
    Some("Last"),
];

/// Corresponds to Java: `DateFormatSymbols.getWeekdays()` (English locale): index 0 an empty string, 1 = Sunday … 7 = Saturday.
const WEEKDAY_NAMES: [&str; 8] = [
    "",
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// Corresponds to Java: `DateFormatSymbols.getMonths()` (English locale): index 0 = January … 11 = December, 12 an empty string.
const MONTH_NAMES: [&str; 13] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
    "",
];

/// Description data for a recurring event.
///
/// Corresponds to Java: `public class RecurringData`
#[derive(Debug, Clone)]
pub struct RecurringData {
    /// Corresponds to Java field `private LocalDate m_startDate`
    start_date: Option<NaiveDate>,
    /// Corresponds to Java field `private LocalDate m_finishDate`
    finish_date: Option<NaiveDate>,
    /// Corresponds to Java field `private Integer m_occurrences`
    occurrences: Option<i32>,
    /// Corresponds to Java field `private RecurrenceType m_recurrenceType`
    recurrence_type: Option<RecurrenceType>,
    /// Corresponds to Java field `private boolean m_relative`
    relative: bool,
    /// Corresponds to Java field `private boolean m_workingDaysOnly`
    working_days_only: bool,
    /// Corresponds to Java field `private boolean m_useEndDate`
    use_end_date: bool,
    /// Corresponds to Java field `private Integer m_frequency`
    frequency: Option<i32>,
    /// Corresponds to Java field `private Integer m_dayNumber`
    day_number: Option<i32>,
    /// Corresponds to Java field `private Integer m_monthNumber`
    month_number: Option<i32>,
/// Corresponds to Java field `private LocalDate[] m_dates` (`None` means the cache is invalid, corresponding to Java's `null`)
    ///
/// Java's `populateDates()` implements caching by writing lazily to `m_dates`; the Rust-side
/// read-only methods such as [`RecurringData::get_dates`] must also be able to trigger population, so `RefCell` provides interior mutability.
    dates: RefCell<Option<Vec<NaiveDate>>>,
    /// Corresponds to Java field `private final EnumSet<DayOfWeek> m_days`
    ///
/// Java's `EnumSet` iterates in the enum's natural order (Monday → Sunday), and `getDayOfWeek()` takes the first element.
/// `chrono::Weekday` does not implement `Ord` and cannot be placed directly in a `BTreeSet`; the Rust side instead uses a fixed-length boolean array
/// to represent the set (see [`day_index`] for the index conversion), and both iteration and "take the first element" explicitly follow
/// [`JAVA_DAY_OF_WEEK_ORDER`] (Monday → Sunday), matching Java's semantics.
    days: [bool; 7],
}

impl RecurringData {
/// Constructor.
    ///
/// Corresponds to Java: the implicit no-argument constructor `RecurringData()`
    pub fn new() -> Self {
        Self {
            start_date: None,
            finish_date: None,
            occurrences: None,
            recurrence_type: None,
            relative: false,
            working_days_only: false,
            use_end_date: false,
            frequency: None,
            day_number: None,
            month_number: None,
            dates: RefCell::new(None),
            days: [false; 7],
        }
    }

/// Get the recurrence start date.
    ///
    /// Corresponds to Java: `public LocalDate getStartDate()`
    pub fn get_start_date(&self) -> Option<NaiveDate> {
        self.start_date
    }

/// Set the recurrence start date.
    ///
    /// Corresponds to Java: `public void setStartDate(LocalDate val)`
    pub fn set_start_date(&mut self, val: Option<NaiveDate>) {
        self.start_date = val;
        self.clear_dates_cache();
    }

/// Get the recurrence end date.
    ///
    /// Corresponds to Java: `public LocalDate getFinishDate()`
    pub fn get_finish_date(&self) -> Option<NaiveDate> {
        self.finish_date
    }

/// Set the recurrence end date.
    ///
    /// Corresponds to Java: `public void setFinishDate(LocalDate val)`
    pub fn set_finish_date(&mut self, val: Option<NaiveDate>) {
        self.finish_date = val;
        self.clear_dates_cache();
    }

/// Get the number of occurrences.
    ///
    /// Corresponds to Java: `public Integer getOccurrences()`
    pub fn get_occurrences(&self) -> Option<i32> {
        self.occurrences
    }

/// Set the number of occurrences.
    ///
    /// Corresponds to Java: `public void setOccurrences(Integer occurrences)`
    pub fn set_occurrences(&mut self, occurrences: Option<i32>) {
        self.occurrences = occurrences;
        self.clear_dates_cache();
    }

/// Get the recurrence type.
    ///
    /// Corresponds to Java: `public RecurrenceType getRecurrenceType()`
    pub fn get_recurrence_type(&self) -> Option<RecurrenceType> {
        self.recurrence_type
    }

/// Set the recurrence type.
    ///
    /// Corresponds to Java: `public void setRecurrenceType(RecurrenceType type)`
    pub fn set_recurrence_type(&mut self, r#type: Option<RecurrenceType>) {
        self.recurrence_type = r#type;
        self.clear_dates_cache();
    }

/// Get the "use end date" flag.
    ///
    /// Corresponds to Java: `public boolean getUseEndDate()`
    pub fn get_use_end_date(&self) -> bool {
        self.use_end_date
    }

/// Set the "use end date" flag.
    ///
    /// Corresponds to Java: `public void setUseEndDate(boolean useEndDate)`
    pub fn set_use_end_date(&mut self, use_end_date: bool) {
        self.use_end_date = use_end_date;
        self.clear_dates_cache();
    }

/// Get the "working days only" flag (meaningful only for daily recurrence).
    ///
    /// Corresponds to Java: `public boolean getWorkingDaysOnly()`
    pub fn get_working_days_only(&self) -> bool {
        self.working_days_only
    }

/// Set the "working days only" flag.
    ///
    /// Corresponds to Java: `public void setWorkingDaysOnly(boolean workingDaysOnly)`
    pub fn set_working_days_only(&mut self, working_days_only: bool) {
        self.working_days_only = working_days_only;
        self.clear_dates_cache();
    }

/// Determine whether a given weekday is part of the weekly recurrence.
    ///
    /// Corresponds to Java: `public boolean getWeeklyDay(DayOfWeek day)`
    pub fn get_weekly_day(&self, day: Weekday) -> bool {
        self.days[day_index(day)]
    }

/// Set the on/off state of a single weekday in the weekly recurrence.
    ///
    /// Corresponds to Java: `public void setWeeklyDay(DayOfWeek day, boolean value)`
    pub fn set_weekly_day(&mut self, day: Weekday, value: bool) {
        self.days[day_index(day)] = value;
        self.clear_dates_cache();
    }

/// Set each day of the weekly recurrence using a bitmap plus a masks array.
    ///
    /// Corresponds to Java: `public void setWeeklyDaysFromBitmap(Integer days, int[] masks)`
    ///
/// The mask index is evaluated with [`DayOfWeekHelper::get_value`] (Sunday = 1 … Saturday = 7);
/// Java throws `ArrayIndexOutOfBoundsException` when `masks` is out of bounds; the Rust side treats it as mask 0.
    pub fn set_weekly_days_from_bitmap(&mut self, days: Option<i32>, masks: &[i32]) {
        if let Some(days) = days {
            for day in JAVA_DAY_OF_WEEK_ORDER {
                let mask = masks
                    .get(DayOfWeekHelper::get_value(day) as usize)
                    .copied()
                    .unwrap_or(0);
                self.set_weekly_day(day, (days & mask) != 0);
            }
            self.clear_dates_cache();
        }
    }

/// Get the "relative" flag, meaningful only for monthly / yearly recurrence.
    ///
    /// Corresponds to Java: `public boolean getRelative()`
    pub fn get_relative(&self) -> bool {
        self.relative
    }

/// Set the "relative" flag, meaningful only for monthly / yearly recurrence.
    ///
    /// Corresponds to Java: `public void setRelative(boolean relative)`
    pub fn set_relative(&mut self, relative: bool) {
        self.relative = relative;
        self.clear_dates_cache();
    }

/// Get the recurrence frequency.
    ///
    /// Corresponds to Java: `public Integer getFrequency()`
    pub fn get_frequency(&self) -> Option<i32> {
        self.frequency
    }

/// Set the recurrence frequency.
    ///
    /// Corresponds to Java: `public void setFrequency(Integer frequency)`
    pub fn set_frequency(&mut self, frequency: Option<i32>) {
        self.frequency = frequency;
        self.clear_dates_cache();
    }

/// Get the weekday on which the monthly / yearly "relative" recurrence is based.
    ///
    /// Corresponds to Java: `public DayOfWeek getDayOfWeek()`
    ///
/// Java takes the first element of the `EnumSet` (ordered Monday → Sunday), returning `null` when empty.
    pub fn get_day_of_week(&self) -> Option<Weekday> {
        JAVA_DAY_OF_WEEK_ORDER
            .into_iter()
            .find(|day| self.get_weekly_day(*day))
    }

/// Set the weekday on which the monthly / yearly "relative" recurrence is based (overwrites the previous value).
    ///
    /// Corresponds to Java: `public void setDayOfWeek(DayOfWeek day)`
    ///
/// When `null` is passed, Java's `EnumSet.add(null)` throws `NullPointerException`;
/// the Rust-side parameter is non-null, so that branch is unreachable.
    pub fn set_day_of_week(&mut self, day: Weekday) {
        self.days = [false; 7];
        self.days[day_index(day)] = true;
        self.clear_dates_cache();
    }

/// Get the day-of-month on which the monthly / yearly "absolute" recurrence is based.
    ///
    /// Corresponds to Java: `public Integer getDayNumber()`
    pub fn get_day_number(&self) -> Option<i32> {
        self.day_number
    }

/// Set the day-of-month on which the monthly / yearly "absolute" recurrence is based.
    ///
    /// Corresponds to Java: `public void setDayNumber(Integer day)`
    pub fn set_day_number(&mut self, day: Option<i32>) {
        self.day_number = day;
        self.clear_dates_cache();
    }

/// Get the month number of the yearly recurrence (1 = January … 12 = December).
    ///
    /// Corresponds to Java: `public Integer getMonthNumber()`
    pub fn get_month_number(&self) -> Option<i32> {
        self.month_number
    }

/// Set the month number of the yearly recurrence.
    ///
    /// Corresponds to Java: `public void setMonthNumber(Integer month)`
    pub fn set_month_number(&mut self, month: Option<i32>) {
        self.month_number = month;
        self.clear_dates_cache();
    }

/// Get all start dates expanded from this recurrence rule.
    ///
    /// Corresponds to Java: `public LocalDate[] getDates()`
    ///
/// Java returns a reference to the internal array; the Rust side returns a cloned `Vec<NaiveDate>`.
    pub fn get_dates(&self) -> Vec<NaiveDate> {
        self.populate_dates();
        let dates = self.dates.borrow();
        dates.as_ref().cloned().unwrap_or_default()
    }

/// Whether the configuration is valid (i.e. whether one or more start dates can be expanded).
    ///
    /// Corresponds to Java: `public boolean isValid()`
    pub fn is_valid(&self) -> bool {
        self.populate_dates();
        let populated = self.dates.borrow().as_ref().is_some_and(|dates| !dates.is_empty());
        populated
    }

/// Get the first calculated date on which an exception actually occurs.
    ///
    /// Corresponds to Java: `public LocalDate getCalculatedFirstDate()`
    ///
/// The start date given by the user is not necessarily aligned with the dates the algorithm generates, hence this method.
/// Java throws `ArrayIndexOutOfBoundsException` when no date has been generated; the Rust side returns
/// `NaiveDate::MIN` as a sentinel value (check with [`RecurringData::is_valid`] first).
    pub fn get_calculated_first_date(&self) -> NaiveDate {
        let dates = self.get_dates();
        dates.first().copied().unwrap_or(NaiveDate::MIN)
    }

/// Get the last calculated date on which an exception actually occurs.
    ///
    /// Corresponds to Java: `public LocalDate getCalculatedLastDate()`
    ///
/// Java throws `ArrayIndexOutOfBoundsException` when no date has been generated; the Rust side returns
/// `NaiveDate::MAX` as a sentinel value (check with [`RecurringData::is_valid`] first).
    pub fn get_calculated_last_date(&self) -> NaiveDate {
        let dates = self.get_dates();
        dates.last().copied().unwrap_or(NaiveDate::MAX)
    }

/// Setting for the yearly "absolute" recurrence: take the day-of-month and month number directly from a date.
    ///
    /// Corresponds to Java: `public void setYearlyAbsoluteFromDate(LocalDate date)`
    pub fn set_yearly_absolute_from_date(&mut self, date: Option<NaiveDate>) {
        if let Some(date) = date {
            self.day_number = Some(date.day() as i32);
            self.month_number = Some(date.month() as i32);
            self.clear_dates_cache();
        }
    }

/// Lazily expand the dates and write them to the cache.
    ///
    /// Corresponds to Java: `private void populateDates()`
    fn populate_dates(&self) {
        let populated = self.dates.borrow().is_some();
        if populated {
            return;
        }

        // Java: int frequency = NumberHelper.getInt(m_frequency); if (frequency < 1) frequency = 1;
        let mut frequency = self.frequency.unwrap_or(0);
        if frequency < 1 {
            frequency = 1;
        }

        let mut dates = Vec::new();

// Java: when the start date is null, each getXxxDates throws NullPointerException;
// the Rust side does not panic and keeps an empty date list (isValid() returns false).
        if let Some(start_date) = self.start_date {
// Java's switch(m_recurrenceType) selects no branch when the type is null
            match self.recurrence_type {
                Some(RecurrenceType::Daily) => self.get_daily_dates(frequency, start_date, &mut dates),
                Some(RecurrenceType::Weekly) => self.get_weekly_dates(frequency, start_date, &mut dates),
                Some(RecurrenceType::Monthly) => self.get_monthly_dates(frequency, start_date, &mut dates),
                Some(RecurrenceType::Yearly) => self.get_yearly_dates(start_date, &mut dates),
                None => {}
            }
        }

        *self.dates.borrow_mut() = Some(dates);
    }

/// Determine whether the next date still needs to be calculated.
    ///
    /// Corresponds to Java: `private boolean moreDates(LocalDate date, List<LocalDate> dates)`
    ///
/// Without an end date it degrades gracefully to using the occurrence count (assuming the recurrence data provides at least one of them).
    fn more_dates(&self, date: NaiveDate, dates: &[NaiveDate]) -> bool {
        match self.finish_date {
            None => {
                // Java: int occurrences = NumberHelper.getInt(m_occurrences); if (occurrences < 1) occurrences = 1;
                let occurrences = match self.occurrences {
                    Some(occurrences) if occurrences >= 1 => occurrences,
                    _ => 1,
                };
                dates.len() < occurrences as usize
            }
            // Java: !date.isAfter(m_finishDate)
            Some(finish_date) => date <= finish_date,
        }
    }

/// Expand the dates of a daily recurrence.
    ///
    /// Corresponds to Java: `private void getDailyDates(int frequency, List<LocalDate> dates)`
    fn get_daily_dates(&self, frequency: i32, start_date: NaiveDate, dates: &mut Vec<NaiveDate>) {
        let mut date = start_date;
        while self.more_dates(date, dates) {
            dates.push(date);
            match plus_days(date, frequency as i64) {
                Some(next) => date = next,
// Java: date.plusDays() throws DateTimeException on out of bounds; the Rust side ends the expansion
                None => break,
            }
        }
    }

/// Expand the dates of a weekly recurrence.
    ///
    /// Corresponds to Java: `private void getWeeklyDates(int frequency, List<LocalDate> dates)`
    ///
/// Algorithm outline: start scanning from the Sunday of the week containing the start date (matches earlier than the start date are ignored),
/// scan at most 7 days at a time, and after scanning a full week skip a number of whole weeks with `7 * (frequency - 1)`.
    fn get_weekly_dates(&self, frequency: i32, start_date: NaiveDate, dates: &mut Vec<NaiveDate>) {
        let mut date = start_date;

        //
// start from the Sunday of the week containing the start date, skipping matches earlier than the start date.
        //
        let mut current_day = start_date.weekday();
        if current_day != Weekday::Sun {
// Java: date.minusDays(currentDay.getValue()), where getValue() is Monday = 1 … Sunday = 7
            match plus_days(date, -java_day_of_week_value(current_day)) {
                Some(adjusted) => date = adjusted,
                None => return,
            }
            current_day = Weekday::Sun;
        }

        while self.more_dates(date, dates) {
            let mut offset: i64 = 0;

            for _day_index in 0..7 {
                if self.get_weekly_day(current_day) {
                    if offset != 0 {
// when a target weekday within the current week is hit, advance the date to that day
                        match plus_days(date, offset) {
                            Some(advanced) => date = advanced,
                            None => return,
                        }
                        offset = 0;
                    }
                    if !self.more_dates(date, dates) {
                        break;
                    }

                    if date >= start_date {
                        dates.push(date);
                    }
                }

                offset += 1;
// Java: currentDay.plus(1) — after Sunday it wraps around to Monday
                current_day = current_day.succ();
            }

            if frequency > 1 {
                offset += 7 * (frequency as i64 - 1);
            }
            match plus_days(date, offset) {
                Some(next) => date = next,
                None => return,
            }
        }
    }

/// Expand the dates of a monthly recurrence (distinguishing the "relative" and "absolute" modes).
    ///
    /// Corresponds to Java: `private void getMonthlyDates(int frequency, List<LocalDate> dates)`
    fn get_monthly_dates(&self, frequency: i32, start_date: NaiveDate, dates: &mut Vec<NaiveDate>) {
        if self.relative {
            self.get_monthly_relative_dates(frequency, start_date, dates);
        } else {
            self.get_monthly_absolute_dates(frequency, start_date, dates);
        }
    }

/// Expand dates of the form "the nth weekday of each month".
    ///
    /// Corresponds to Java: `private void getMonthlyRelativeDates(int frequency, List<LocalDate> dates)`
    ///
/// Each time, compute the relative day starting from the 1st of the current month; if the computed date is earlier than the start date, discard that occurrence,
/// but still advance to later months by the frequency. `dayNumber > 4` means take the last such weekday of the month.
    fn get_monthly_relative_dates(
        &self,
        frequency: i32,
        start_date: NaiveDate,
        dates: &mut Vec<NaiveDate>,
    ) {
        // Java: LocalDate.of(m_startDate.getYear(), m_startDate.getMonth(), 1)
        let Some(first_of_month) = date_of(start_date.year(), start_date.month() as i32, 1) else {
            return;
        };
        let mut date = first_of_month;
        let day_number = self.day_number.unwrap_or(0);

        while self.more_dates(date, dates) {
            let relative = if day_number > 4 {
                self.get_last_relative_day(date)
            } else {
                self.get_ordinal_relative_day(date, day_number)
            };
// Java: throws NullPointerException when getDayOfWeek() is null (no weekday set)
            let Some(relative) = relative else { return };

            if relative >= start_date {
                dates.push(relative);
                if !self.more_dates(relative, dates) {
                    break;
                }
            }

            // Java: date = LocalDate.of(date.getYear(), date.getMonth(), 1); date = date.plusMonths(frequency)
            let Some(month_start) = date_of(relative.year(), relative.month() as i32, 1) else {
                break;
            };
            match month_start.checked_add_months(Months::new(frequency as u32)) {
                Some(next) => date = next,
                None => break,
            }
        }
    }

/// Expand dates of the form "the nth day of each month" (when the month is too short, take the last day of the month).
    ///
    /// Corresponds to Java: `private void getMonthlyAbsoluteDates(int frequency, List<LocalDate> dates)`
    fn get_monthly_absolute_dates(
        &self,
        frequency: i32,
        start_date: NaiveDate,
        dates: &mut Vec<NaiveDate>,
    ) {
        let current_day_number = start_date.day() as i32;
        // Java: date = LocalDate.of(date.getYear(), date.getMonth(), 1)
        let Some(month_start) = date_of(start_date.year(), start_date.month() as i32, 1) else {
            return;
        };
        let mut date = month_start;
        let required_day_number = self.day_number.unwrap_or(0);

// when the required day-of-month is already earlier than the start date's day-of-month, start from the next month
        if required_day_number < current_day_number {
            match date.checked_add_months(Months::new(1)) {
                Some(next) => date = next,
                None => return,
            }
        }

        while self.more_dates(date, dates) {
            let max_day_number = length_of_month(date.year(), date.month())
                .map_or(0, |days| days as i32);
// Java: when the target day-of-month exceeds the number of days in the month, take the last day of the month (YearMonth.lengthOfMonth())
            let use_day_number = if required_day_number > max_day_number {
                max_day_number
            } else {
                required_day_number
            };

            let Some(current) = date_of(date.year(), date.month() as i32, use_day_number) else {
                break;
            };
            dates.push(current);

            // Java: date = LocalDate.of(date.getYear(), date.getMonth(), 1); date = date.plusMonths(frequency)
            let Some(next_month_start) = date_of(current.year(), current.month() as i32, 1) else {
                break;
            };
            match next_month_start.checked_add_months(Months::new(frequency as u32)) {
                Some(next) => date = next,
                None => break,
            }
        }
    }

/// Expand the dates of a yearly recurrence (distinguishing the "relative" and "absolute" modes).
    ///
    /// Corresponds to Java: `private void getYearlyDates(List<LocalDate> dates)`
    ///
/// Note that Java does not pass frequency to the yearly recurrence; yearly recurrence is always "every year".
    fn get_yearly_dates(&self, start_date: NaiveDate, dates: &mut Vec<NaiveDate>) {
        if self.relative {
            self.get_yearly_relative_dates(start_date, dates);
        } else {
            self.get_yearly_absolute_dates(start_date, dates);
        }
    }

/// Expand dates of the form "the nth weekday of a month in each year".
    ///
    /// Corresponds to Java: `private void getYearlyRelativeDates(List<LocalDate> dates)`
    fn get_yearly_relative_dates(&self, start_date: NaiveDate, dates: &mut Vec<NaiveDate>) {
        let month_number = self.month_number.unwrap_or(0);
        // Java: LocalDate.of(m_startDate.getYear(), NumberHelper.getInt(m_monthNumber), 1)
        let Some(first_of_month) = date_of(start_date.year(), month_number, 1) else {
            return;
        };
        let mut date = first_of_month;
        let day_number = self.day_number.unwrap_or(0);

        while self.more_dates(date, dates) {
            let relative = if day_number > 4 {
                self.get_last_relative_day(date)
            } else {
                self.get_ordinal_relative_day(date, day_number)
            };
// Java: throws NullPointerException when getDayOfWeek() is null (no weekday set)
            let Some(relative) = relative else { return };

            if relative >= start_date {
                dates.push(relative);
                if !self.more_dates(relative, dates) {
                    break;
                }
            }

            // Java: date = LocalDate.of(date.getYear() + 1, date.getMonth(), 1)
            let Some(month_start) = date_of(relative.year(), relative.month() as i32, 1) else {
                break;
            };
            match month_start.checked_add_months(Months::new(12)) {
                Some(next) => date = next,
                None => break,
            }
        }
    }

/// Expand dates of the form "a month and day in each year" (when the month is too short, take the last day of the month).
    ///
    /// Corresponds to Java: `private void getYearlyAbsoluteDates(List<LocalDate> dates)`
    fn get_yearly_absolute_dates(&self, start_date: NaiveDate, dates: &mut Vec<NaiveDate>) {
        let month_number = self.month_number.unwrap_or(0);
        // Java: LocalDate.of(m_startDate.getYear(), NumberHelper.getInt(m_monthNumber), 1)
        let Some(first_of_month) = date_of(start_date.year(), month_number, 1) else {
            return;
        };
        let mut date = first_of_month;
        let required_day_number = self.day_number.unwrap_or(0);

        while self.more_dates(date, dates) {
            let max_day_number = length_of_month(date.year(), date.month())
                .map_or(0, |days| days as i32);
// Java: when the target day-of-month exceeds the number of days in the month, take the last day of the month (YearMonth.lengthOfMonth())
            let use_day_number = if required_day_number > max_day_number {
                max_day_number
            } else {
                required_day_number
            };

            let Some(mut current) = date_of(date.year(), date.month() as i32, use_day_number) else {
                break;
            };
// when the computed date is earlier than the start date, shift it forward by one year (Java: date.plusYears(1);
// under Java's rules, February 29 of a leap year is adjusted to February 28 of the following year)
            if current < start_date {
                match current.checked_add_months(Months::new(12)) {
                    Some(shifted) => current = shifted,
                    None => break,
                }
            }

            dates.push(current);

            // Java: date = LocalDate.of(date.getYear(), date.getMonth(), 1); date = date.plusYears(1)
            let Some(next_month_start) = date_of(current.year(), current.month() as i32, 1) else {
                break;
            };
            match next_month_start.checked_add_months(Months::new(12)) {
                Some(next) => date = next,
                None => break,
            }
        }
    }

/// Move the date to the nth occurrence of the specified weekday in the month.
    ///
    /// Corresponds to Java: `private LocalDate getOrdinalRelativeDay(LocalDate date, int dayNumber)`
    ///
/// The passed-in `date` is usually the 1st of the month; first use the value from [`DayOfWeekHelper`] (Sunday = 1 … Saturday = 7)
/// to compute the offset "advancing from the 1st to the first target weekday", then jump to the nth with `7 * (dayNumber - 1)`.
/// When no weekday is set, Java throws `NullPointerException`; the Rust side returns `None`.
    fn get_ordinal_relative_day(&self, date: NaiveDate, day_number: i32) -> Option<NaiveDate> {
        let required_day_of_week = DayOfWeekHelper::get_value(self.get_day_of_week()?);
        let current_day_of_week = DayOfWeekHelper::get_value(date.weekday());

        let day_of_week_offset = if required_day_of_week > current_day_of_week {
            required_day_of_week - current_day_of_week
        } else if required_day_of_week < current_day_of_week {
            7 - (current_day_of_week - required_day_of_week)
        } else {
            0
        };

        let mut date = plus_days(date, day_of_week_offset as i64)?;

        if day_number > 1 {
            date = plus_days(date, 7 * (day_number as i64 - 1))?;
        }

        Some(date)
    }

/// Move the date to the last occurrence of the specified weekday in the month.
    ///
    /// Corresponds to Java: `private LocalDate getLastRelativeDay(LocalDate date)`
    ///
/// First move the date to the last day of the month, then step back to the target weekday.
/// When no weekday is set, Java throws `NullPointerException`; the Rust side returns `None`.
    fn get_last_relative_day(&self, date: NaiveDate) -> Option<NaiveDate> {
        // Java: YearMonth.of(date.getYear(), date.getMonth()).lengthOfMonth()
        let last_day_of_month = length_of_month(date.year(), date.month())?;
        let mut date = date_of(
            date.year(),
            date.month() as i32,
            last_day_of_month as i32,
        )?;

        let required_day_of_week = DayOfWeekHelper::get_value(self.get_day_of_week()?);
        let current_day_of_week = DayOfWeekHelper::get_value(date.weekday());

        let day_of_week_offset = if current_day_of_week > required_day_of_week {
            required_day_of_week - current_day_of_week
        } else if current_day_of_week < required_day_of_week {
            -7 + (required_day_of_week - current_day_of_week)
        } else {
            0
        };

        if day_of_week_offset != 0 {
            date = plus_days(date, day_of_week_offset as i64)?;
        }

        Some(date)
    }

/// Get the ordinal text for a given integer (used to generate debug output).
    ///
    /// Corresponds to Java: `private String getOrdinal(Integer value)`
    ///
/// Java throws `NullPointerException` when `value` is `null` and `ArrayIndexOutOfBoundsException` when the index is out of bounds;
/// the Rust side uniformly uses `"null"` as a placeholder.
    fn get_ordinal(&self, value: Option<i32>) -> String {
        match value {
            Some(index) if index >= ORDINAL.len() as i32 => format!("every {}th", index),
            Some(index) => ORDINAL
                .get(usize::try_from(index).unwrap_or(usize::MAX))
                .copied()
                .flatten()
                .map_or_else(|| "null".to_string(), str::to_string),
            None => "null".to_string(),
        }
    }

/// Invalidate the date cache.
    ///
    /// Corresponds to Java: `private void clearDatesCache()`
    fn clear_dates_cache(&self) {
        *self.dates.borrow_mut() = None;
    }
}

impl Default for RecurringData {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for RecurringData {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java assembles it with `PrintWriter`, the Rust side uses `write!`; the weekday / month names are fixed to English
/// (see the file header).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[RecurringData {}",
            recurrence_type_name(self.recurrence_type)
        )?;

        match self.recurrence_type {
            Some(RecurrenceType::Daily) => {
                write!(f, " {}", self.get_ordinal(self.frequency))?;
                write!(
                    f,
                    "{}",
                    if self.working_days_only {
                        " Working day"
                    } else {
                        " Day"
                    }
                )?;
            }
            Some(RecurrenceType::Weekly) => {
                write!(f, " {}", self.get_ordinal(self.frequency))?;
                write!(f, " week on ")?;

                let mut separator = "";
                for day in JAVA_DAY_OF_WEEK_ORDER {
                    if self.get_weekly_day(day) {
                        write!(f, "{}{}", separator, weekday_name(Some(day)))?;
                        separator = ", ";
                    }
                }
            }
            Some(RecurrenceType::Monthly) => {
                if self.relative {
                    write!(f, " on The ")?;
                    write!(f, "{}", day_ordinal(self.day_number))?;
                    write!(f, " {}", weekday_name(self.get_day_of_week()))?;
                    write!(f, " of ")?;
                    write!(f, "{}", self.get_ordinal(self.frequency))?;
                } else {
                    write!(f, " on Day ")?;
                    write!(f, "{}", optional_integer(self.day_number))?;
                    write!(f, " of ")?;
                    write!(f, "{}", self.get_ordinal(self.frequency))?;
                }
                write!(f, " month")?;
            }
            Some(RecurrenceType::Yearly) => {
                write!(f, " on the ")?;
                if self.relative {
                    write!(f, "{}", day_ordinal(self.day_number))?;
                    write!(f, " {}", weekday_name(self.get_day_of_week()))?;
                    write!(f, " of {}", month_name(self.month_number))?;
                } else {
                    write!(
                        f,
                        "{} {}",
                        optional_integer(self.day_number),
                        month_name(self.month_number)
                    )?;
                }
            }
            None => {}
        }

        if let Some(start_date) = self.start_date {
            write!(f, " From {}", start_date)?;
        }

        if let Some(occurrences) = self.occurrences {
            write!(f, " For {} occurrences", occurrences)?;
        }

        if let Some(finish_date) = self.finish_date {
            write!(f, " To {}", finish_date)?;
        }

        write!(f, "]")
    }
}

/// Corresponds to Java's `java.time.DayOfWeek.getValue()`: Monday = 1 … Sunday = 7.
fn java_day_of_week_value(day: Weekday) -> i64 {
    day.num_days_from_monday() as i64 + 1
}

/// Convert a weekday to a [`RecurringData::days`] array index: corresponds to Java's `DayOfWeekHelper.getValue(day) - 1`.
fn day_index(day: Weekday) -> usize {
    (DayOfWeekHelper::get_value(day) - 1) as usize
}

/// Corresponds to Java's `LocalDate.plusDays(days)`; returns `None` on out of bounds (Java throws `DateTimeException`).
fn plus_days(date: NaiveDate, days: i64) -> Option<NaiveDate> {
    TimeDelta::try_days(days).and_then(|duration| date.checked_add_signed(duration))
}

/// Corresponds to Java's `LocalDate.of(year, month, day)`: returns `None` when the values are invalid (Java throws `DateTimeException`).
fn date_of(year: i32, month: i32, day: i32) -> Option<NaiveDate> {
    let month = u32::try_from(month).ok()?;
    let day = u32::try_from(day).ok()?;
    NaiveDate::from_ymd_opt(year, month, day)
}

/// Corresponds to Java's `YearMonth.lengthOfMonth()`: the number of days in the month.
fn length_of_month(year: i32, month: u32) -> Option<u32> {
    let start = NaiveDate::from_ymd_opt(year, month, 1)?;
    let end = start.checked_add_months(Months::new(1))?;
    Some((end - start).num_days() as u32)
}

/// Corresponds to the output of `pw.print(m_recurrenceType)` in Java (the enum `toString()`; `null` outputs `"null"`).
fn recurrence_type_name(recurrence_type: Option<RecurrenceType>) -> &'static str {
    recurrence_type.map_or("null", |recurrence_type| recurrence_type.get_name())
}

/// Corresponds to the output of an `Integer` participating directly in string concatenation in Java (`null` outputs `"null"`).
fn optional_integer(value: Option<i32>) -> String {
    value.map_or_else(|| "null".to_string(), |value| value.to_string())
}

/// Corresponds to the output of an `Integer` (`m_dayNumber`) in Java looking up the "which occurrence" name via `DAY_ORDINAL`.
///
/// Java throws `NullPointerException` on `null` and `ArrayIndexOutOfBoundsException` when the index is out of bounds;
/// the Rust side uniformly uses `"null"` as a placeholder.
fn day_ordinal(value: Option<i32>) -> String {
    value
        .and_then(|index| usize::try_from(index).ok())
        .and_then(|index| DAY_ORDINAL.get(index))
        .copied()
        .flatten()
        .map_or_else(|| "null".to_string(), str::to_string)
}

/// Corresponds to the output of `DateFormatSymbols.getWeekdays()[DayOfWeekHelper.getValue(day)]` in Java.
///
/// The value of `getValue` is Sunday = 1 … Saturday = 7, which happens to be the index into `WEEKDAY_NAMES`;
/// Java throws `NullPointerException` when `day` is `null`; the Rust side uses `"null"` as a placeholder.
fn weekday_name(day: Option<Weekday>) -> &'static str {
    match day {
        Some(day) => WEEKDAY_NAMES
            .get(DayOfWeekHelper::get_value(day) as usize)
            .copied()
            .unwrap_or("null"),
        None => "null",
    }
}

/// Corresponds to the output of `DateFormatSymbols.getMonths()[m_monthNumber - 1]` in Java.
///
/// Java throws `NullPointerException` on `null` and `ArrayIndexOutOfBoundsException` when the index is out of bounds;
/// the Rust side uniformly uses `"null"` as a placeholder.
fn month_name(month_number: Option<i32>) -> &'static str {
    month_number
        .and_then(|month_number| usize::try_from(month_number - 1).ok())
        .and_then(|index| MONTH_NAMES.get(index))
        .copied()
        .unwrap_or("null")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
    }

    fn daily(frequency: Option<i32>) -> RecurringData {
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Daily));
        data.set_frequency(frequency);
        data
    }

    fn weekly(days: &[Weekday], frequency: Option<i32>) -> RecurringData {
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Weekly));
        data.set_frequency(frequency);
        for day in days {
            data.set_weekly_day(*day, true);
        }
        data
    }

    #[test]
    fn defaults_and_accessors() {
        let mut data = RecurringData::new();

        assert_eq!(data.get_start_date(), None);
        assert_eq!(data.get_finish_date(), None);
        assert_eq!(data.get_occurrences(), None);
        assert_eq!(data.get_recurrence_type(), None);
        assert_eq!(data.get_frequency(), None);
        assert_eq!(data.get_day_number(), None);
        assert_eq!(data.get_month_number(), None);
        assert_eq!(data.get_day_of_week(), None);
        assert!(!data.get_relative());
        assert!(!data.get_working_days_only());
        assert!(!data.get_use_end_date());

        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_finish_date(Some(date(2024, 12, 31)));
        data.set_occurrences(Some(5));
        data.set_recurrence_type(Some(RecurrenceType::Monthly));
        data.set_frequency(Some(2));
        data.set_day_number(Some(15));
        data.set_month_number(Some(6));
        data.set_relative(true);
        data.set_working_days_only(true);
        data.set_use_end_date(true);

        assert_eq!(data.get_start_date(), Some(date(2024, 1, 1)));
        assert_eq!(data.get_finish_date(), Some(date(2024, 12, 31)));
        assert_eq!(data.get_occurrences(), Some(5));
        assert_eq!(data.get_recurrence_type(), Some(RecurrenceType::Monthly));
        assert_eq!(data.get_frequency(), Some(2));
        assert_eq!(data.get_day_number(), Some(15));
        assert_eq!(data.get_month_number(), Some(6));
        assert!(data.get_relative());
        assert!(data.get_working_days_only());
        assert!(data.get_use_end_date());

        data.set_recurrence_type(None);
        assert_eq!(data.get_recurrence_type(), None);
    }

    #[test]
    fn daily_uses_occurrences_when_no_finish_date() {
        let mut data = daily(Some(3));
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_occurrences(Some(4));

        assert!(data.is_valid());
        assert_eq!(
            data.get_dates(),
            vec![
                date(2024, 1, 1),
                date(2024, 1, 4),
                date(2024, 1, 7),
                date(2024, 1, 10)
            ]
        );
        assert_eq!(data.get_calculated_first_date(), date(2024, 1, 1));
        assert_eq!(data.get_calculated_last_date(), date(2024, 1, 10));
    }

    #[test]
    fn daily_uses_finish_date_boundary_inclusive() {
        let mut data = daily(Some(2));
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_finish_date(Some(date(2024, 1, 5)));
        data.set_use_end_date(true);

// the end date is inclusive
        assert_eq!(
            data.get_dates(),
            vec![date(2024, 1, 1), date(2024, 1, 3), date(2024, 1, 5)]
        );
    }

    #[test]
    fn daily_defaults_frequency_and_occurrences_to_one() {
// when both the frequency and the occurrence count are null, Java's NumberHelper.getInt falls back to 0, which is then promoted to 1
        let mut data = daily(None);
        data.set_start_date(Some(date(2024, 1, 1)));

        assert_eq!(data.get_dates(), vec![date(2024, 1, 1)]);
    }

    #[test]
    fn daily_changes_clear_cached_dates() {
        let mut data = daily(Some(1));
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_occurrences(Some(3));

        assert_eq!(data.get_dates().len(), 3);

// after the cache is cleared, it expands again according to the new occurrence count
        data.set_occurrences(Some(5));
        assert_eq!(data.get_dates().len(), 5);
    }

    #[test]
    fn missing_start_date_yields_empty_dates() {
// the Java side throws NullPointerException; the Rust side degrades gracefully to an empty result (see the file header)
        let mut data = daily(Some(1));
        data.set_occurrences(Some(3));

        assert!(!data.is_valid());
        assert!(data.get_dates().is_empty());
        assert_eq!(data.get_calculated_first_date(), NaiveDate::MIN);
        assert_eq!(data.get_calculated_last_date(), NaiveDate::MAX);
    }

    #[test]
    fn missing_recurrence_type_yields_empty_dates() {
        let mut data = RecurringData::new();
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_occurrences(Some(3));

        assert!(!data.is_valid());
    }

    #[test]
    fn weekly_multiple_days() {
// 2024-01-01 is a Monday
        let mut data = weekly(&[Weekday::Mon, Weekday::Wed, Weekday::Fri], Some(1));
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_occurrences(Some(6));

        assert_eq!(
            data.get_dates(),
            vec![
                date(2024, 1, 1),
                date(2024, 1, 3),
                date(2024, 1, 5),
                date(2024, 1, 8),
                date(2024, 1, 10),
                date(2024, 1, 12)
            ]
        );
    }

    #[test]
    fn weekly_ignores_days_before_start_date() {
// the start date is a Wednesday and only Wednesday recurs: Monday / Tuesday are in the same week but earlier than the start date, so they must be ignored
        let mut data = weekly(&[Weekday::Wed], Some(1));
        data.set_start_date(Some(date(2024, 1, 3)));
        data.set_occurrences(Some(3));

        assert_eq!(
            data.get_dates(),
            vec![date(2024, 1, 3), date(2024, 1, 10), date(2024, 1, 17)]
        );
    }

    #[test]
    fn weekly_frequency_skips_whole_weeks() {
        let mut data = weekly(&[Weekday::Mon], Some(2));
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_occurrences(Some(3));

        assert_eq!(
            data.get_dates(),
            vec![date(2024, 1, 1), date(2024, 1, 15), date(2024, 1, 29)]
        );
    }

    #[test]
    fn weekly_finish_date_boundary() {
        let mut data = weekly(&[Weekday::Mon, Weekday::Fri], Some(1));
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_finish_date(Some(date(2024, 1, 12)));

        assert_eq!(
            data.get_dates(),
            vec![
                date(2024, 1, 1),
                date(2024, 1, 5),
                date(2024, 1, 8),
                date(2024, 1, 12)
            ]
        );
    }

    #[test]
    fn weekly_days_from_bitmap() {
// mask indices: Sunday = 1, Monday = 2 … Saturday = 7
        let masks = [0, 1, 2, 4, 8, 16, 32, 64];
        let mut data = weekly(&[], None);
// Monday (2) and Friday (32)
        data.set_weekly_days_from_bitmap(Some(2 | 32), &masks);

        assert!(data.get_weekly_day(Weekday::Mon));
        assert!(data.get_weekly_day(Weekday::Fri));
        assert!(!data.get_weekly_day(Weekday::Sun));
        assert!(!data.get_weekly_day(Weekday::Tue));
        assert!(!data.get_weekly_day(Weekday::Wed));
        assert!(!data.get_weekly_day(Weekday::Thu));
        assert!(!data.get_weekly_day(Weekday::Sat));
    }

    #[test]
    fn weekly_days_from_bitmap_ignores_null_bitmap() {
        let mut data = weekly(&[], None);
        data.set_weekly_day(Weekday::Tue, true);
        data.set_weekly_days_from_bitmap(None, &[0, 1, 2, 4, 8, 16, 32, 64]);

        assert!(data.get_weekly_day(Weekday::Tue));
    }

    #[test]
    fn day_of_week_returns_first_in_java_enum_order() {
        let mut data = weekly(&[], None);
        data.set_weekly_day(Weekday::Wed, true);
        data.set_weekly_day(Weekday::Mon, true);

// Java's EnumSet iterates in the order Monday → Sunday, so the first element is Monday
        assert_eq!(data.get_day_of_week(), Some(Weekday::Mon));

        data.set_day_of_week(Weekday::Fri);
        assert_eq!(data.get_day_of_week(), Some(Weekday::Fri));
        assert!(!data.get_weekly_day(Weekday::Mon));
    }

    #[test]
    fn monthly_absolute_clamps_to_end_of_month_and_handles_leap_year() {
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Monthly));
        data.set_frequency(Some(1));
        data.set_start_date(Some(date(2024, 1, 15)));
        data.set_day_number(Some(31));
        data.set_occurrences(Some(4));

// February is truncated to the 29th (2024 is a leap year), April is truncated to the 30th
        assert_eq!(
            data.get_dates(),
            vec![
                date(2024, 1, 31),
                date(2024, 2, 29),
                date(2024, 3, 31),
                date(2024, 4, 30)
            ]
        );
    }

    #[test]
    fn monthly_absolute_skips_to_next_month_when_day_already_passed() {
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Monthly));
        data.set_frequency(Some(1));
        data.set_start_date(Some(date(2024, 1, 20)));
        data.set_day_number(Some(15));
        data.set_occurrences(Some(2));

        assert_eq!(
            data.get_dates(),
            vec![date(2024, 2, 15), date(2024, 3, 15)]
        );
    }

    #[test]
    fn monthly_relative_ordinal_day() {
// the first Monday of each month: 2024-01-01, 2024-02-05, 2024-03-04
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Monthly));
        data.set_frequency(Some(1));
        data.set_relative(true);
        data.set_day_number(Some(1));
        data.set_day_of_week(Weekday::Mon);
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_occurrences(Some(3));

        assert_eq!(
            data.get_dates(),
            vec![date(2024, 1, 1), date(2024, 2, 5), date(2024, 3, 4)]
        );
    }

    #[test]
    fn monthly_relative_last_day_of_week() {
// dayNumber > 4 means the last occurrence of the specified weekday in the month
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Monthly));
        data.set_frequency(Some(1));
        data.set_relative(true);
        data.set_day_number(Some(5));
        data.set_day_of_week(Weekday::Mon);
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_occurrences(Some(3));

        assert_eq!(
            data.get_dates(),
            vec![date(2024, 1, 29), date(2024, 2, 26), date(2024, 3, 25)]
        );
    }

    #[test]
    fn monthly_relative_ignores_dates_before_start() {
// the start date is January 15; the first Monday (January 1) is earlier than the start date and is ignored
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Monthly));
        data.set_frequency(Some(1));
        data.set_relative(true);
        data.set_day_number(Some(1));
        data.set_day_of_week(Weekday::Mon);
        data.set_start_date(Some(date(2024, 1, 15)));
        data.set_occurrences(Some(2));

        assert_eq!(
            data.get_dates(),
            vec![date(2024, 2, 5), date(2024, 3, 4)]
        );
    }

    #[test]
    fn monthly_relative_uses_monthly_frequency() {
// the last Friday of every two months
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Monthly));
        data.set_frequency(Some(2));
        data.set_relative(true);
        data.set_day_number(Some(5));
        data.set_day_of_week(Weekday::Fri);
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_occurrences(Some(3));

// the last Friday of January 2024 is January 26; every other month thereafter: March 29, May 31
        assert_eq!(
            data.get_dates(),
            vec![date(2024, 1, 26), date(2024, 3, 29), date(2024, 5, 31)]
        );
    }

    #[test]
    fn yearly_absolute_clamps_to_end_of_february() {
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Yearly));
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_month_number(Some(2));
        data.set_day_number(Some(29));
        data.set_occurrences(Some(3));

// February 29 in a leap year; truncated to February 28 in a common year
        assert_eq!(
            data.get_dates(),
            vec![date(2024, 2, 29), date(2025, 2, 28), date(2026, 2, 28)]
        );
    }

    #[test]
    fn yearly_absolute_shifts_year_when_before_start_date() {
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Yearly));
        data.set_start_date(Some(date(2024, 6, 15)));
        data.set_month_number(Some(3));
        data.set_day_number(Some(10));
        data.set_occurrences(Some(2));

        assert_eq!(
            data.get_dates(),
            vec![date(2025, 3, 10), date(2026, 3, 10)]
        );
    }

    #[test]
    fn yearly_relative_ordinal_day() {
// the fourth Thursday of November each year (Thanksgiving)
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Yearly));
        data.set_relative(true);
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_month_number(Some(11));
        data.set_day_number(Some(4));
        data.set_day_of_week(Weekday::Thu);
        data.set_occurrences(Some(3));

        assert_eq!(
            data.get_dates(),
            vec![date(2024, 11, 28), date(2025, 11, 27), date(2026, 11, 26)]
        );
    }

    #[test]
    fn yearly_relative_last_day_of_week() {
// the last Friday of February each year
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Yearly));
        data.set_relative(true);
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_month_number(Some(2));
        data.set_day_number(Some(5));
        data.set_day_of_week(Weekday::Fri);
        data.set_occurrences(Some(3));

        // 2024-02-23,2025-02-28,2026-02-27
        assert_eq!(
            data.get_dates(),
            vec![date(2024, 2, 23), date(2025, 2, 28), date(2026, 2, 27)]
        );
    }

    #[test]
    fn yearly_crosses_year_boundary() {
// the start date is in December; yearly recurrence crosses into the next year
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Yearly));
        data.set_start_date(Some(date(2024, 12, 20)));
        data.set_month_number(Some(12));
        data.set_day_number(Some(31));
        data.set_occurrences(Some(3));

        assert_eq!(
            data.get_dates(),
            vec![
                date(2024, 12, 31),
                date(2025, 12, 31),
                date(2026, 12, 31)
            ]
        );
    }

    #[test]
    fn yearly_ignores_frequency() {
// Java's getYearlyDates does not accept frequency; yearly recurrence is always every year
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Yearly));
        data.set_frequency(Some(3));
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_month_number(Some(1));
        data.set_day_number(Some(1));
        data.set_occurrences(Some(3));

        assert_eq!(
            data.get_dates(),
            vec![date(2024, 1, 1), date(2025, 1, 1), date(2026, 1, 1)]
        );
    }

    #[test]
    fn yearly_absolute_from_date() {
        let mut data = RecurringData::new();
        data.set_month_number(Some(1));
        data.set_day_number(Some(1));
        data.set_yearly_absolute_from_date(Some(date(2024, 6, 15)));

        assert_eq!(data.get_month_number(), Some(6));
        assert_eq!(data.get_day_number(), Some(15));

// the Java side changes no field when date is null
        data.set_yearly_absolute_from_date(None);
        assert_eq!(data.get_month_number(), Some(6));
        assert_eq!(data.get_day_number(), Some(15));
    }

    #[test]
    fn dates_cache_is_shared_between_accessors() {
        let mut data = daily(Some(1));
        data.set_start_date(Some(date(2024, 3, 1)));
        data.set_finish_date(Some(date(2024, 3, 3)));

        assert_eq!(data.get_dates().len(), 3);
        assert_eq!(data.get_calculated_first_date(), date(2024, 3, 1));
        assert_eq!(data.get_calculated_last_date(), date(2024, 3, 3));

// changing any field that affects expansion invalidates the cache
        data.set_start_date(Some(date(2024, 3, 2)));
        assert_eq!(data.get_dates(), vec![date(2024, 3, 2), date(2024, 3, 3)]);
    }

    #[test]
    fn display_daily() {
        let mut data = daily(Some(1));
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_occurrences(Some(3));

        assert_eq!(
            data.to_string(),
            "[RecurringData Daily every Day From 2024-01-01 For 3 occurrences]"
        );

        data.set_frequency(Some(2));
        data.set_working_days_only(true);
        assert_eq!(
            data.to_string(),
            "[RecurringData Daily every other Working day From 2024-01-01 For 3 occurrences]"
        );
    }

    #[test]
    fn display_weekly() {
        let mut data = weekly(&[Weekday::Fri, Weekday::Mon], Some(1));
        data.set_start_date(Some(date(2024, 1, 1)));

        assert_eq!(
            data.to_string(),
            "[RecurringData Weekly every week on Monday, Friday From 2024-01-01]"
        );

        data.set_frequency(Some(3));
        data.set_finish_date(Some(date(2024, 12, 31)));
        assert_eq!(
            data.to_string(),
            "[RecurringData Weekly every 3rd week on Monday, Friday From 2024-01-01 To 2024-12-31]"
        );
    }

    #[test]
    fn display_monthly() {
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Monthly));
        data.set_frequency(Some(3));
        data.set_start_date(Some(date(2024, 1, 15)));
        data.set_day_number(Some(15));

        assert_eq!(
            data.to_string(),
            "[RecurringData Monthly on Day 15 of every 3rd month From 2024-01-15]"
        );

        data.set_relative(true);
        data.set_day_number(Some(2));
        data.set_day_of_week(Weekday::Mon);
        assert_eq!(
            data.to_string(),
            "[RecurringData Monthly on The Second Monday of every 3rd month From 2024-01-15]"
        );
    }

    #[test]
    fn display_yearly() {
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Yearly));
        data.set_day_number(Some(15));
        data.set_month_number(Some(6));

        assert_eq!(data.to_string(), "[RecurringData Yearly on the 15 June]");

        data.set_relative(true);
        data.set_day_number(Some(4));
        data.set_day_of_week(Weekday::Thu);
        data.set_month_number(Some(11));
        assert_eq!(
            data.to_string(),
            "[RecurringData Yearly on the Fourth Thursday of November]"
        );

// dayNumber > 4 corresponds to "Last" in DAY_ORDINAL
        data.set_day_number(Some(5));
        assert_eq!(
            data.to_string(),
            "[RecurringData Yearly on the Last Thursday of November]"
        );
    }

    #[test]
    fn display_empty_recurrence_data() {
        let data = RecurringData::new();
        assert_eq!(data.to_string(), "[RecurringData null]");
    }

    #[test]
    fn display_frequency_above_ordinal_table() {
        let mut data = daily(Some(5));
        data.set_start_date(Some(date(2024, 1, 1)));

        assert_eq!(
            data.to_string(),
            "[RecurringData Daily every 5th Day From 2024-01-01]"
        );
    }
}
