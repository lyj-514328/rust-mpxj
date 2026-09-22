//! Corresponds to Java: `org/mpxj/mpp/RecurringTaskReader.java`
//!
//! Reads the recurring task definition from the recurring task data block of an MPP file (start / end date, duration, recurrence frequency,
//! which days of the week, and the relative or absolute rules for monthly / yearly recurrence, etc.).
//!
//! Data block layout (offset unit: bytes):
//!
//! ```text
//!  6 : start date (2 bytes, number of days since 1983-12-31)
//! 10 : end date (2 bytes)
//! 12 : duration (4 bytes, tenths of a minute)
//! 16 : duration units (2 bytes)
//! 18 : occurrence count (2 bytes)
//! 20 : recurrence type (2 bytes; value 1 = daily, 4 = weekly, 8 = monthly, 16 = yearly)
//! 24 : whether to use the end date (2 bytes, 1 means yes)
//! 26 : whether weekdays only (2 bytes, 1 means yes)
//! 28..40 : Sunday / Monday / ... / Saturday (2 bytes each, 1 means yes)
//! 42 : whether monthly recurrence uses a "relative" rule (2 bytes)
//! 44 : whether yearly recurrence uses an "absolute" rule (2 bytes; note the check is the opposite of offset 42)
//! 46/48/54/58 : frequency (daily / weekly / monthly absolute / monthly relative)
//! 50/52/56 : the "occurrence number" and day of week for the monthly relative / absolute rules
//! 60/62/64 : the "occurrence number", "day of week", and "month" for the yearly relative rule
//! 70 : the date for the yearly absolute rule (2 bytes)
//! ```
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `final class RecurringTaskReader` | `pub struct RecurringTaskReader` | Rust has no package-level visibility |
//! | `RecurringTaskReader(ProjectProperties properties)` | [`RecurringTaskReader::new`] takes `&ProjectProperties` and keeps only a **weak reference to its parent file** | `ProjectProperties` is held by `ProjectFile` and is not `Clone`, so the Rust side cannot keep a copy of the properties; reading the duration needs the "minutes per day / per week" from the properties, so it goes back to the parent file via the weak reference and borrows on demand (Java holds a stable reference) |
//! | `processRecurringTask(Task task, byte[] data)` (Java calls `task.addRecurringTask()` inside the method to get the instance and then writes it field by field) | [`RecurringTaskReader::process_recurring_task`] takes `&[u8]` and **returns** [`crate::recurring_task::RecurringTask`] | "reading" and "attaching to the task" are split into two steps: writing into `Task` on the Rust side requires `&mut Task`, so the caller takes the read result and attaches it to the task via `Task::add_recurring_task()` (already ported, see `crate::task::Task::add_recurring_task`). The returned value is exactly the type held by `Task` in Java, so the caller need not move any fields |
//! | `RecurringTask extends RecurringData` (additionally holds `m_duration`) | directly reuses the model layer's [`crate::recurring_task::RecurringTask`] (that type likewise reproduces inheritance with composition + `Deref`) | this file previously had its own type of the same name, resulting in two implementations alongside the model layer; they have now been merged into one |
//! | `ByteArrayHelper.getShort/getInt` out of bounds | check the length first and return 0 when insufficient | Java throws `ArrayIndexOutOfBoundsException` |
//! | `switch (rt.getRecurrenceType())` when the type is `null` | the `None` branch of the `match` (entering no case) | Java throws a `NullPointerException` when `switch`ing on a `null` enum |
//! | `setDayOfWeek(null)` when `DayOfWeekHelper.getInstance(int)` returns `null` | skip that setting | Java throws a `NullPointerException` at `EnumSet.add(null)` |
//!
//! Skipped Java members: none (`processRecurringTask` has been translated method by method, and all of its internal setter calls have been preserved).
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class uses none of these mechanisms.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use chrono::Weekday;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::day_of_week_helper::DayOfWeekHelper;
use crate::duration::Duration;
use crate::mpp::mpp_utility::MPPUtility;
use crate::project_file::ProjectFile;
use crate::project_properties::ProjectProperties;
use crate::recurrence_type::RecurrenceType;
use crate::recurring_task::RecurringTask;
use crate::time_unit::TimeUnit;

/// The offset of the "relative" flag when the recurrence type is "monthly".
///
/// Corresponds to Java: `rt.setRelative(ByteArrayHelper.getShort(data, 42) == 1)`
const MONTHLY_RELATIVE_OFFSET: usize = 42;

/// The offset of the "absolute" flag when the recurrence type is "yearly".
///
/// Corresponds to Java: `rt.setRelative(ByteArrayHelper.getShort(data, 44) != 1)`
const YEARLY_ABSOLUTE_OFFSET: usize = 44;

/// Offset of the start date.
///
/// Corresponds to Java: `MPPUtility.getDate(data, 6)`
const START_DATE_OFFSET: usize = 6;

/// Offset of the end date.
///
/// Corresponds to Java: `MPPUtility.getDate(data, 10)`
const FINISH_DATE_OFFSET: usize = 10;

/// Offset of the duration.
///
/// Corresponds to Java: `ByteArrayHelper.getInt(data, 12)`
const DURATION_OFFSET: usize = 12;

/// Offset of the duration units.
///
/// Corresponds to Java: `ByteArrayHelper.getShort(data, 16)`
const DURATION_UNITS_OFFSET: usize = 16;

/// Offset of the occurrence count.
///
/// Corresponds to Java: `ByteArrayHelper.getShort(data, 18)`
const OCCURRENCES_OFFSET: usize = 18;

/// Offset of the recurrence type.
///
/// Corresponds to Java: `ByteArrayHelper.getShort(data, 20)`
const RECURRENCE_TYPE_OFFSET: usize = 20;

/// Offset of the "use end date" flag.
///
/// Corresponds to Java: `ByteArrayHelper.getShort(data, 24)`
const USE_END_DATE_OFFSET: usize = 24;

/// Offset of the "weekdays only" flag.
///
/// Corresponds to Java: `ByteArrayHelper.getShort(data, 26)`
const WORKING_DAYS_ONLY_OFFSET: usize = 26;

/// Start offset of the 7 weekday flags (Sunday first).
///
/// Corresponds to Java: `ByteArrayHelper.getShort(data, 28)` … `ByteArrayHelper.getShort(data, 40)`
const WEEKLY_DAYS_OFFSET: usize = 28;

/// Reader for recurring task definitions.
///
/// Corresponds to Java: `final class RecurringTaskReader`
pub struct RecurringTaskReader {
    /// The parent file that owns the project properties (weak reference).
    ///
    /// Corresponds to Java field: `private final ProjectProperties m_properties`
    ///(Java keeps a reference to the properties object; the Rust side goes back to the parent file via a weak reference and borrows on demand, see the differences table in the file header)
    parent_file: Weak<RefCell<ProjectFile>>,
}

impl RecurringTaskReader {
    /// Constructor.
    ///
    /// Corresponds to Java: `public RecurringTaskReader(ProjectProperties properties)`
    pub fn new(properties: &ProjectProperties) -> Self {
        Self {
            parent_file: properties
                .get_parent_file()
                .map_or_else(Weak::new, |file| Rc::downgrade(&file)),
        }
    }

    /// Reads the recurring task data.
    ///
    /// Corresponds to Java: `public void processRecurringTask(Task task, byte[] data)`
    ///
    /// Java creates a `RecurringTask` inside the method and writes it into the passed-in `task`; the Rust side returns that instance
    /// (see the differences table in the file header), and the caller is responsible for attaching it.
    pub fn process_recurring_task(&self, data: &[u8]) -> RecurringTask {
        // Java: rt.setDuration(MPPUtility.getAdjustedDuration(m_properties, ByteArrayHelper.getInt(data, 12),
        //      MPPUtility.getDurationTimeUnits(ByteArrayHelper.getShort(data, 16))));
        // (compute the duration first, then take the mutable borrow of `RecurringData`; the two borrow scopes must not overlap.)
        let duration = self.adjusted_duration(
            read_int(data, DURATION_OFFSET),
            MPPUtility::get_duration_time_units(read_short(data, DURATION_UNITS_OFFSET)),
        );

        let mut result = RecurringTask::new();
        result.set_duration(duration);
        // Java: `RecurringTask rt = task.addRecurringTask();` -- this crate returns that instance,
        // and the base class `RecurringData`'s methods are called directly on `rt` via `DerefMut` (the same shape as Java).
        let rt = &mut result;

        // Java: rt.setStartDate(MPPUtility.getDate(data, 6));
        rt.set_start_date(MPPUtility::get_date(data, START_DATE_OFFSET));
        // Java: rt.setFinishDate(MPPUtility.getDate(data, 10));
        rt.set_finish_date(MPPUtility::get_date(data, FINISH_DATE_OFFSET));
        // Java: rt.setOccurrences(Integer.valueOf(ByteArrayHelper.getShort(data, 18)));
        rt.set_occurrences(Some(read_short(data, OCCURRENCES_OFFSET)));
        // Java: rt.setRecurrenceType(RecurrenceType.getInstance(ByteArrayHelper.getShort(data, 20)));
        rt.set_recurrence_type(RecurrenceType::get_instance(read_short(
            data,
            RECURRENCE_TYPE_OFFSET,
        )));
        // Java: rt.setUseEndDate(ByteArrayHelper.getShort(data, 24) == 1);
        rt.set_use_end_date(read_short(data, USE_END_DATE_OFFSET) == 1);
        // Java: rt.setWorkingDaysOnly(ByteArrayHelper.getShort(data, 26) == 1);
        rt.set_working_days_only(read_short(data, WORKING_DAYS_ONLY_OFFSET) == 1);

        // Java:
        // rt.setWeeklyDay(DayOfWeek.SUNDAY, ByteArrayHelper.getShort(data, 28) == 1);
        // rt.setWeeklyDay(DayOfWeek.MONDAY, ByteArrayHelper.getShort(data, 30) == 1);
        // ... through to SATURDAY (offset 40)
        for (index, day) in DAYS.into_iter().enumerate() {
            rt.set_weekly_day(day, read_short(data, WEEKLY_DAYS_OFFSET + index * 2) == 1);
        }

        // Java: int frequencyOffset = 0; int dayOfWeekOffset = 0; int dayNumberOffset = 0;
        //       int monthNumberOffset = 0; int dateOffset = 0;
        let mut frequency_offset = 0usize;
        let mut day_of_week_offset = 0usize;
        let mut day_number_offset = 0usize;
        let mut month_number_offset = 0usize;
        let mut date_offset = 0usize;

        // Java: switch (rt.getRecurrenceType()) -- Java throws an NPE for null; the Rust side enters no branch
        match rt.get_recurrence_type() {
            Some(RecurrenceType::Daily) => {
                // Java: frequencyOffset = 46;
                frequency_offset = 46;
            }
            Some(RecurrenceType::Weekly) => {
                // Java: frequencyOffset = 48;
                frequency_offset = 48;
            }
            Some(RecurrenceType::Monthly) => {
                // Java: rt.setRelative(ByteArrayHelper.getShort(data, 42) == 1);
                rt.set_relative(read_short(data, MONTHLY_RELATIVE_OFFSET) == 1);
                if rt.get_relative() {
                    // Java: frequencyOffset = 58; dayNumberOffset = 50; dayOfWeekOffset = 52;
                    frequency_offset = 58;
                    day_number_offset = 50;
                    day_of_week_offset = 52;
                } else {
                    // Java: frequencyOffset = 54; dayNumberOffset = 56;
                    frequency_offset = 54;
                    day_number_offset = 56;
                }
            }
            Some(RecurrenceType::Yearly) => {
                // Java: rt.setRelative(ByteArrayHelper.getShort(data, 44) != 1); (the opposite of the monthly case)
                rt.set_relative(read_short(data, YEARLY_ABSOLUTE_OFFSET) != 1);
                if rt.get_relative() {
                    // Java: dayNumberOffset = 60; dayOfWeekOffset = 62; monthNumberOffset = 64;
                    day_number_offset = 60;
                    day_of_week_offset = 62;
                    month_number_offset = 64;
                } else {
                    // Java: dateOffset = 70;
                    date_offset = 70;
                }
            }
            None => {}
        }

        if frequency_offset != 0 {
            // Java: rt.setFrequency(Integer.valueOf(ByteArrayHelper.getShort(data, frequencyOffset)));
            rt.set_frequency(Some(read_short(data, frequency_offset)));
        }

        if day_of_week_offset != 0 {
            // Java: rt.setDayOfWeek(DayOfWeekHelper.getInstance(ByteArrayHelper.getShort(data, dayOfWeekOffset) + 1));
            // Java throws an NPE inside setDayOfWeek when it gets null; the Rust side skips that setting
            if let Some(day) = DayOfWeekHelper::get_instance(read_short(data, day_of_week_offset) + 1) {
                rt.set_day_of_week(day);
            }
        }

        if day_number_offset != 0 {
            // Java: rt.setDayNumber(Integer.valueOf(ByteArrayHelper.getShort(data, dayNumberOffset)));
            rt.set_day_number(Some(read_short(data, day_number_offset)));
        }

        if month_number_offset != 0 {
            // Java: rt.setMonthNumber(Integer.valueOf(ByteArrayHelper.getShort(data, monthNumberOffset)));
            rt.set_month_number(Some(read_short(data, month_number_offset)));
        }

        if date_offset != 0 {
            // Java: rt.setYearlyAbsoluteFromDate(MPPUtility.getDate(data, dateOffset));
            rt.set_yearly_absolute_from_date(MPPUtility::get_date(data, date_offset));
        }

        result
    }

    /// Converts the duration in the file into a user-visible duration according to the project properties.
    ///
    /// Corresponds to Java: `MPPUtility.getAdjustedDuration(m_properties, duration, timeUnit)`
    ///
    /// The Rust side needs to borrow the properties object in the parent file: when the parent file is not wired up or is mutably borrowed, it degrades gracefully to
    /// [`MPPUtility::get_duration_int`] (no unit adjustment, no panic).
    fn adjusted_duration(&self, duration: i32, time_unit: TimeUnit) -> Option<Duration> {
        let Some(file) = self.parent_file.upgrade() else {
            return Some(MPPUtility::get_duration_int(duration, time_unit));
        };

        match file.try_borrow() {
            Ok(file) => MPPUtility::get_adjusted_duration(
                file.get_project_properties(),
                duration,
                time_unit,
            ),
            Err(_) => Some(MPPUtility::get_duration_int(duration, time_unit)),
        }
    }
}

/// The order of the 7 weekdays (consistent with the call order from `setWeeklyDay(SUNDAY, ...)` to `setWeeklyDay(SATURDAY, ...)` in Java:
/// starting at offset 28, 2 bytes per day).
const DAYS: [Weekday; 7] = [
    Weekday::Sun,
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
];

/// Reads a two-byte short integer; returns 0 when there is insufficient data (Java throws `ArrayIndexOutOfBoundsException`).
fn read_short(data: &[u8], offset: usize) -> i32 {
    if offset.checked_add(2).is_none_or(|end| end > data.len()) {
        return 0;
    }

    ByteArrayHelper::get_short(data, offset)
}

/// Reads a four-byte integer; returns 0 when there is insufficient data (Java throws `ArrayIndexOutOfBoundsException`).
fn read_int(data: &[u8], offset: usize) -> i32 {
    if offset.checked_add(4).is_none_or(|end| end > data.len()) {
        return 0;
    }

    ByteArrayHelper::get_int(data, offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::rc::Weak;

    /// Properties object without a parent file (duration conversion degrades gracefully to "no adjustment").
    fn properties() -> ProjectProperties {
        ProjectProperties::new(Weak::new())
    }

    /// Constructs a recurring task data block: all fields are written at their offsets, with enough space left at the end.
    ///
    /// The default content is a "monthly relative rule": start 1984-01-01, end 1984-01-31, duration 1 day,
    /// 5 occurrences, Monday and Friday, frequency 2, the 3rd Wednesday.
    fn recurring_task_data() -> Vec<u8> {
        let mut data = vec![0u8; 80];

        // Start date: 1984-01-01 (1 day since 1983-12-31)
        data[START_DATE_OFFSET..START_DATE_OFFSET + 2].copy_from_slice(&1u16.to_le_bytes());
        // End date: 1984-01-31 (31 days)
        data[FINISH_DATE_OFFSET..FINISH_DATE_OFFSET + 2].copy_from_slice(&31u16.to_le_bytes());
        // Duration: 4800 tenths of a minute = 1 day
        data[DURATION_OFFSET..DURATION_OFFSET + 4].copy_from_slice(&4800i32.to_le_bytes());
        // Duration units: 7 = Days
        data[DURATION_UNITS_OFFSET..DURATION_UNITS_OFFSET + 2]
            .copy_from_slice(&7u16.to_le_bytes());
        // Occurrence count
        data[OCCURRENCES_OFFSET..OCCURRENCES_OFFSET + 2].copy_from_slice(&5u16.to_le_bytes());
        // Recurrence type: 8 = Monthly
        data[RECURRENCE_TYPE_OFFSET..RECURRENCE_TYPE_OFFSET + 2]
            .copy_from_slice(&8u16.to_le_bytes());
        // Use end date / weekdays only
        data[USE_END_DATE_OFFSET..USE_END_DATE_OFFSET + 2].copy_from_slice(&1u16.to_le_bytes());
        data[WORKING_DAYS_ONLY_OFFSET..WORKING_DAYS_ONLY_OFFSET + 2]
            .copy_from_slice(&1u16.to_le_bytes());
        // Monday (offset 30) and Friday (offset 38)
        data[30..32].copy_from_slice(&1u16.to_le_bytes());
        data[38..40].copy_from_slice(&1u16.to_le_bytes());
        // Monthly "relative" rule
        data[MONTHLY_RELATIVE_OFFSET..MONTHLY_RELATIVE_OFFSET + 2]
            .copy_from_slice(&1u16.to_le_bytes());
        // Frequency 2, occurrence 3, day of week 3 (Java adds 1 to get 4 -> Wednesday)
        data[58..60].copy_from_slice(&2u16.to_le_bytes());
        data[50..52].copy_from_slice(&3u16.to_le_bytes());
        data[52..54].copy_from_slice(&3u16.to_le_bytes());

        data
    }

    /// Rewrites the recurrence type in the data block to the given value.
    fn with_recurrence_type(mut data: Vec<u8>, value: u16) -> Vec<u8> {
        data[RECURRENCE_TYPE_OFFSET..RECURRENCE_TYPE_OFFSET + 2]
            .copy_from_slice(&value.to_le_bytes());
        data
    }

    /// Monthly relative rule: almost all fields are read out.
    #[test]
    fn reads_monthly_relative_recurring_task() {
        let reader = RecurringTaskReader::new(&properties());
        let task = reader.process_recurring_task(&recurring_task_data());
        let rt = task.get_recurring_data();

        assert_eq!(rt.get_start_date(), NaiveDate::from_ymd_opt(1984, 1, 1));
        assert_eq!(rt.get_finish_date(), NaiveDate::from_ymd_opt(1984, 1, 31));
        // Duration units DAYS, value 4800 tenths of a minute (not adjusted by the project default when there is no parent file)
        assert_eq!(
            task.get_duration().map(|d| (d.get_duration(), d.get_units())),
            Some((4800.0 / 4800.0, TimeUnit::Days))
        );
        assert_eq!(rt.get_occurrences(), Some(5));
        assert_eq!(rt.get_recurrence_type(), Some(RecurrenceType::Monthly));
        assert!(rt.get_use_end_date());
        assert!(rt.get_working_days_only());
        assert!(rt.get_relative());
        assert_eq!(rt.get_frequency(), Some(2));
        assert_eq!(rt.get_day_number(), Some(3));
        // DayOfWeekHelper values: Sunday = 1 ... Saturday = 7, so 4 corresponds to Wednesday
        assert_eq!(rt.get_day_of_week(), Some(Weekday::Wed));
        // The monthly rule does not involve a month number
        assert_eq!(rt.get_month_number(), None);

        // Java's `setDayOfWeek` clears first and then sets a single day of the week, so only Wednesday remains in the end
        assert!(rt.get_weekly_day(Weekday::Wed));
        assert!(!rt.get_weekly_day(Weekday::Sun));
        assert!(!rt.get_weekly_day(Weekday::Mon));
        assert!(!rt.get_weekly_day(Weekday::Fri));
    }

    /// The 7 weekday flags (daily recurrence, not overwritten by `setDayOfWeek`).
    #[test]
    fn reads_weekly_day_flags() {
        let data = with_recurrence_type(recurring_task_data(), 1); // Daily
        let reader = RecurringTaskReader::new(&properties());
        let task = reader.process_recurring_task(&data);
        let rt = task.get_recurring_data();

        assert_eq!(rt.get_recurrence_type(), Some(RecurrenceType::Daily));
        assert!(!rt.get_weekly_day(Weekday::Sun));
        assert!(rt.get_weekly_day(Weekday::Mon));
        assert!(!rt.get_weekly_day(Weekday::Tue));
        assert!(!rt.get_weekly_day(Weekday::Wed));
        assert!(!rt.get_weekly_day(Weekday::Thu));
        assert!(rt.get_weekly_day(Weekday::Fri));
        assert!(!rt.get_weekly_day(Weekday::Sat));
        // The daily rule does not set "relative"
        assert!(!rt.get_relative());
        assert_eq!(rt.get_day_number(), None);
        // The daily rule's frequency comes from offset 46 (default 0)
        assert_eq!(rt.get_frequency(), Some(0));
    }

    /// Daily recurrence: the frequency comes from offset 46.
    #[test]
    fn reads_daily_frequency() {
        let mut data = with_recurrence_type(recurring_task_data(), 1);
        data[46..48].copy_from_slice(&3u16.to_le_bytes());

        let reader = RecurringTaskReader::new(&properties());
        let task = reader.process_recurring_task(&data);

        assert_eq!(task.get_recurring_data().get_frequency(), Some(3));
    }

    /// Weekly recurrence: the frequency comes from offset 48.
    #[test]
    fn reads_weekly_frequency() {
        let mut data = with_recurrence_type(recurring_task_data(), 4);
        data[48..50].copy_from_slice(&2u16.to_le_bytes());

        let reader = RecurringTaskReader::new(&properties());
        let task = reader.process_recurring_task(&data);

        assert_eq!(
            task.get_recurring_data().get_recurrence_type(),
            Some(RecurrenceType::Weekly)
        );
        assert_eq!(task.get_recurring_data().get_frequency(), Some(2));
    }

    /// Monthly absolute rule: the frequency and "day of month" come from another set of offsets.
    #[test]
    fn reads_monthly_absolute_recurring_task() {
        let mut data = with_recurrence_type(recurring_task_data(), 8);
        data[MONTHLY_RELATIVE_OFFSET..MONTHLY_RELATIVE_OFFSET + 2]
            .copy_from_slice(&0u16.to_le_bytes()); // not relative
        data[54..56].copy_from_slice(&1u16.to_le_bytes()); // frequency
        data[56..58].copy_from_slice(&15u16.to_le_bytes()); // day number

        let reader = RecurringTaskReader::new(&properties());
        let task = reader.process_recurring_task(&data);
        let rt = task.get_recurring_data();

        assert!(!rt.get_relative());
        assert_eq!(rt.get_frequency(), Some(1));
        assert_eq!(rt.get_day_number(), Some(15));
        // The absolute rule does not set the day of week; the weekday flags keep the values from the data block (Monday and Friday),
        // so `get_day_of_week` returns the first one in Java's enum order (Monday).
        assert_eq!(rt.get_day_of_week(), Some(Weekday::Mon));
        assert!(rt.get_weekly_day(Weekday::Mon));
        assert!(rt.get_weekly_day(Weekday::Fri));
        assert_eq!(rt.get_month_number(), None);
    }

    /// Yearly absolute rule: `data[44] == 1` means "absolute", and the month number and day of month are derived from the date field.
    #[test]
    fn reads_yearly_absolute_recurring_task() {
        let mut data = with_recurrence_type(recurring_task_data(), 16);
        data[YEARLY_ABSOLUTE_OFFSET..YEARLY_ABSOLUTE_OFFSET + 2]
            .copy_from_slice(&1u16.to_le_bytes());
        // 1983-12-31 + 100 days = 1984-04-09
        data[70..72].copy_from_slice(&100u16.to_le_bytes());

        let reader = RecurringTaskReader::new(&properties());
        let task = reader.process_recurring_task(&data);
        let rt = task.get_recurring_data();

        assert!(!rt.get_relative());
        assert_eq!(rt.get_month_number(), Some(4));
        assert_eq!(rt.get_day_number(), Some(9));
        assert_eq!(rt.get_frequency(), None);
    }

    /// Yearly relative rule: `data[44] != 1` means "relative", and it reads the occurrence number / day of week / month.
    #[test]
    fn reads_yearly_relative_recurring_task() {
        let mut data = with_recurrence_type(recurring_task_data(), 16);
        data[YEARLY_ABSOLUTE_OFFSET..YEARLY_ABSOLUTE_OFFSET + 2]
            .copy_from_slice(&0u16.to_le_bytes());
        data[60..62].copy_from_slice(&4u16.to_le_bytes()); // occurrence number
        data[62..64].copy_from_slice(&5u16.to_le_bytes()); // day of week（+1 = 6 → Friday）
        data[64..66].copy_from_slice(&11u16.to_le_bytes()); // month

        let reader = RecurringTaskReader::new(&properties());
        let task = reader.process_recurring_task(&data);
        let rt = task.get_recurring_data();

        assert!(rt.get_relative());
        assert_eq!(rt.get_day_number(), Some(4));
        assert_eq!(rt.get_day_of_week(), Some(Weekday::Fri));
        assert_eq!(rt.get_month_number(), Some(11));
    }

    /// When there is insufficient data, all reads degrade gracefully to 0 / `None` and do not panic.
    #[test]
    fn short_data_does_not_panic() {
        let reader = RecurringTaskReader::new(&properties());

        let task = reader.process_recurring_task(&[]);
        assert_eq!(task.get_recurring_data().get_start_date(), None);
        assert_eq!(task.get_recurring_data().get_finish_date(), None);
        // The duration reads as 0, and remains 0 after conversion by DAYS (Java's getAdjustedDuration likewise returns 0 rather than null)
        assert_eq!(
            task.get_duration().map(|value| (value.get_duration(), value.get_units())),
            Some((0.0, TimeUnit::Days))
        );
        // Java throws an array-out-of-bounds exception; the Rust side degrades gracefully to 0
        assert_eq!(task.get_recurring_data().get_occurrences(), Some(0));
        // RecurrenceType.getInstance(0) falls back to DAILY in Java
        assert_eq!(
            task.get_recurring_data().get_recurrence_type(),
            Some(RecurrenceType::Daily)
        );
        assert!(!task.get_recurring_data().get_use_end_date());
        assert_eq!(task.get_recurring_data().get_frequency(), Some(0));

        // Just enough to read the duration units
        let task = reader.process_recurring_task(&recurring_task_data()[..18]);
        assert_eq!(task.get_recurring_data().get_occurrences(), Some(0));
    }

    /// Values that fall into a hole in the `RecurrenceType` lookup table correspond to Java's `null`.
    #[test]
    fn recurrence_type_hole_yields_no_recurrence_type() {
        let data = with_recurrence_type(recurring_task_data(), 2);
        let reader = RecurringTaskReader::new(&properties());
        let task = reader.process_recurring_task(&data);
        let rt = task.get_recurring_data();

        assert_eq!(rt.get_recurrence_type(), None);
        assert_eq!(rt.get_frequency(), None);
        assert_eq!(rt.get_day_number(), None);
        assert_eq!(rt.get_month_number(), None);
    }

    /// When the parent file is available, the duration is converted according to the project properties (480 minutes/day -> 9600, i.e. 2 days).
    #[test]
    fn adjusted_duration_uses_project_properties() {
        let file = ProjectFile::new();
        let reader = {
            let file_ref = file.borrow();
            RecurringTaskReader::new(file_ref.get_project_properties())
        };

        let mut data = recurring_task_data();
        data[DURATION_OFFSET..DURATION_OFFSET + 4].copy_from_slice(&9600i32.to_le_bytes());

        let task = reader.process_recurring_task(&data);
        let duration = task.get_duration().expect("duration valid");
        assert_eq!(duration.get_duration(), 2.0);
        assert_eq!(duration.get_units(), TimeUnit::Days);
    }

    /// For non-"day" units, convert directly by unit (the project default is not involved).
    #[test]
    fn adjusted_duration_for_hour_units() {
        let file = ProjectFile::new();
        let reader = {
            let file_ref = file.borrow();
            RecurringTaskReader::new(file_ref.get_project_properties())
        };

        let mut data = recurring_task_data();
        data[DURATION_UNITS_OFFSET..DURATION_UNITS_OFFSET + 2]
            .copy_from_slice(&5u16.to_le_bytes()); // Hours
        data[DURATION_OFFSET..DURATION_OFFSET + 4].copy_from_slice(&600i32.to_le_bytes());

        let task = reader.process_recurring_task(&data);
        let duration = task.get_duration().expect("duration valid");
        assert_eq!(duration.get_duration(), 1.0);
        assert_eq!(duration.get_units(), TimeUnit::Hours);
    }
}
