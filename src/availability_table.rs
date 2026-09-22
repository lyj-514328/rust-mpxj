//! Corresponds to Java: `org/mpxj/AvailabilityTable.java`

use chrono::{Duration as ChronoDuration, NaiveDate, NaiveDateTime, NaiveTime};

use crate::availability::Availability;
use crate::common::local_date_time_helper::LocalDateTimeHelper;
use crate::local_date_time_range::LocalDateTimeRange;

/// Availability table for a resource (gives the number of available units per date range).
///
/// Corresponds to Java: `public final class AvailabilityTable extends ArrayList<Availability>`
///
/// Porting notes:
/// - The Java side extends `ArrayList<Availability>`; the Rust side instead holds a standalone
/// container of `Vec<Availability>`, providing only the collection operations needed for the port (length, get by index, append, iterate).
/// - Java's `START_DATE_NA` is taken entirely from `LocalDateTimeHelper` (enabled in this crate, called directly
/// as [`LocalDateTimeHelper::start_date_na`]); `END_DATE_NA`, however, **mixes two sources**,
/// so each occurrence must be checked against Java (`Java: AvailabilityTable.java`):
///
/// | Java location | Constant | Rust |
///   |---|---|---|
/// | `range.getEnd().isAfter(END_DATE_NA)` of `hasDefaultDateRange` | this class's constant 23:58 | [`AvailabilityTable::end_date_na`] |
/// | `rangeEnd.isAfter(END_DATE_NA)` of `availableFrom` | this class's constant 23:58 | [`AvailabilityTable::end_date_na`] |
/// | `rangeEnd.isAfter(END_DATE_NA)` of `availableTo` | this class's constant 23:58 | [`AvailabilityTable::end_date_na`] |
/// | `rangeEnd = range.getEnd() == null ? ...` of `availableTo` | helper 23:59 | [`LocalDateTimeHelper::end_date_na`] |
///
/// The two are not interchangeable (the Java comment explains that this class's constant exists to tolerate MS Project's inconsistent values,
/// see [`AvailabilityTable::end_date_na`]).
///
/// Java methods not ported: none (methods inherited from `ArrayList` are out of scope for this class's port).
#[derive(Debug, Clone, Default)]
pub struct AvailabilityTable {
/// The availability rows in the table.
    ///
/// Corresponds to Java: internal storage of `ArrayList<Availability>`.
    entries: Vec<Availability>,
}

impl AvailabilityTable {
/// Constructor.
    ///
/// Corresponds to Java: default constructor of `ArrayList<Availability>`.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

/// Construct a table from existing rows.
    ///
    /// Corresponds to Java: `ArrayList<Availability>(Collection<? extends Availability> c)`.
    pub fn from_entries(entries: Vec<Availability>) -> Self {
        Self { entries }
    }

/// Append a row.
    ///
    /// Corresponds to Java: `ArrayList.add(E element)`.
    pub fn add(&mut self, entry: Availability) {
        self.entries.push(entry);
    }

/// Advance one row.
    ///
/// Corresponds to Java: `ArrayList.get(int index)`; returns `None` when out of bounds (Java throws `IndexOutOfBoundsException`).
    pub fn get(&self, index: usize) -> Option<&Availability> {
        self.entries.get(index)
    }

/// Number of rows.
    ///
    /// Corresponds to Java: `ArrayList.size()`.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

/// Whether the table is empty.
    ///
    /// Corresponds to Java: `ArrayList.isEmpty()`.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

/// Iterate over the rows in the table.
    ///
    /// Corresponds to Java: `ArrayList.iterator()`.
    pub fn iter(&self) -> std::slice::Iter<'_, Availability> {
        self.entries.iter()
    }

/// Access the rows in the table as a slice.
    ///
/// Corresponds to Java: the list view of `ArrayList`.
    pub fn as_slice(&self) -> &[Availability] {
        &self.entries
    }

/// Get the table row effective at the given date.
    ///
    /// Corresponds to Java: `public Availability getEntryByDate(LocalDateTime date)`
    ///
/// Compare each range with the target date one by one: a matching range returns that row, and once a range lies entirely after the target date
/// the scan ends early (Java behaves the same, implicitly relying on the table already being sorted by range).
    pub fn get_entry_by_date(&self, date: NaiveDateTime) -> Option<&Availability> {
        let mut result = None;

        for entry in &self.entries {
            let range = entry.get_range();
            let comparison_result = range.compare_to_date(date);
            if comparison_result >= 0 {
                if comparison_result == 0 {
                    result = Some(entry);
                    break;
                }
            } else {
                break;
            }
        }

        result
    }

/// Determine whether this table contains only a single default row whose start and end dates are both empty/NA.
    ///
    /// Corresponds to Java: `public boolean hasDefaultDateRange()`
    pub fn has_default_date_range(&self) -> bool {
// No ranges at all
        if self.entries.is_empty() {
            return true;
        }

// More than one range
        if self.entries.len() != 1 {
            return false;
        }

// Only one range whose start or end date is empty or NA
        match self.entries.first() {
            Some(entry) => {
                let range = entry.get_range();
                let start_is_na = match range.get_start() {
                    None => true,
                    Some(start) => start == LocalDateTimeHelper::start_date_na(),
                };
// Java: range.getEnd().isAfter(END_DATE_NA) (this class's own 23:58, not the helper's 23:59)
                let end_is_na = match range.get_end() {
                    None => true,
                    Some(end) => end > Self::end_date_na(),
                };
                start_is_na && end_is_na
            }
            None => true,
        }
    }

/// Compute the Available From property for the given date.
    ///
    /// Corresponds to Java: `public LocalDateTime availableFrom(LocalDateTime date)`
    ///
/// On the Java side, after fetching `previousRange.getEnd()` it calls `plusMinutes(1)`, and when the end date is
/// `null` it throws an NPE; the Rust side degrades gracefully by returning `None`.
    pub fn available_from(&mut self, date: NaiveDateTime) -> Option<NaiveDateTime> {
        if self.has_default_date_range() {
            return None;
        }

// Ensure the table is sorted (Java: sort(Comparator.naturalOrder()))
        self.entries.sort();

        let mut previous_range: Option<LocalDateTimeRange> = None;
        for availability in &self.entries {
            let current_range = availability.get_range();

            let range_start = current_range
                .get_start()
                .unwrap_or_else(LocalDateTimeHelper::start_date_na);

// Is the target date earlier than this range?
            if date < range_start {
                return previous_range
                    .and_then(|range| range.get_end())
                    .and_then(|end| end.checked_add_signed(ChronoDuration::minutes(1)));
            }

// Does the target date fall within this range?
            let range_end = current_range.get_end();
            let within_range = match range_end {
                None => true,
                Some(range_end) => range_end > Self::end_date_na() || date <= range_end,
            };

            if within_range {
                return if range_start == LocalDateTimeHelper::start_date_na() {
                    None
                } else {
                    Some(range_start)
                };
            }

// The target date is later than this range
            previous_range = Some(current_range);
        }

        previous_range
            .and_then(|range| range.get_end())
            .and_then(|end| end.checked_add_signed(ChronoDuration::minutes(1)))
    }

/// Compute the Available To property for the given date.
    ///
    /// Corresponds to Java: `public LocalDateTime availableTo(LocalDateTime date)`
    ///
/// On the Java side, after obtaining the range start date it calls `minusMinutes(1)`; the Rust side uses `checked_sub_signed`
/// to avoid panicking on overflow.
    pub fn available_to(&mut self, date: NaiveDateTime) -> Option<NaiveDateTime> {
        if self.has_default_date_range() {
            return None;
        }

// Ensure the table is sorted (Java: sort(Comparator.naturalOrder()))
        self.entries.sort();

        for availability in &self.entries {
            let current_range = availability.get_range();

            // Java: rangeEnd = currentRange.getEnd() == null
            //       ? LocalDateTimeHelper.END_DATE_NA : currentRange.getEnd();
// Note that the **helper** END_DATE_NA (23:59) is used here, not this class's 23:58:
// this class's constant is used only in `hasDefaultDateRange` / `availableFrom` / the
// `rangeEnd.isAfter(END_DATE_NA)` check at the end of the method (see the method docs and the file header).
            let range_end = current_range
                .get_end()
                .unwrap_or_else(LocalDateTimeHelper::end_date_na);

// Is the target date later than this range?
            if date > range_end {
                continue;
            }

            let range_start = current_range
                .get_start()
                .unwrap_or_else(LocalDateTimeHelper::start_date_na);

// Is the target date earlier than this range?
            if date < range_start {
                return range_start.checked_sub_signed(ChronoDuration::minutes(1));
            }

// The target date falls within this range
            return if range_end > Self::end_date_na() {
                None
            } else {
                Some(range_end)
            };
        }

        None
    }

    /// Corresponds to Java: `public static final LocalDateTime END_DATE_NA = LocalDateTime.of(2049, 12, 31, 23, 58)`
    ///
/// This value differs from `LocalDateTimeHelper.END_DATE_NA` and exists to tolerate the inconsistent
/// values used by Microsoft Project. chrono's date-time construction is not a `const fn`, so this is a function instead.
    pub fn end_date_na() -> NaiveDateTime {
        match (
            NaiveDate::from_ymd_opt(2049, 12, 31),
            NaiveTime::from_hms_opt(23, 58, 0),
        ) {
            (Some(date), Some(time)) => NaiveDateTime::new(date, time),
            _ => NaiveDateTime::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// Construct a test date-time.
    fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(year, month, day).expect("valid date"),
            NaiveTime::from_hms_opt(hour, minute, 0).expect("valid time"),
        )
    }

/// Construct a test date-time with seconds (used to distinguish `END_DATE_NA` 23:58 from 23:59).
    fn date_time_with_seconds(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> NaiveDateTime {
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(year, month, day).expect("valid date"),
            NaiveTime::from_hms_opt(hour, minute, second).expect("valid time"),
        )
    }

/// Construct an availability row.
    fn availability(start: Option<NaiveDateTime>, end: Option<NaiveDateTime>) -> Availability {
        Availability::new(start, end, Some(1.0))
    }

/// Construct a table spanning January 2024 -> March 2024.
    fn sample_table() -> AvailabilityTable {
        AvailabilityTable::from_entries(vec![
            availability(
                Some(date_time(2024, 1, 1, 8, 0)),
                Some(date_time(2024, 1, 31, 17, 0)),
            ),
            availability(
                Some(date_time(2024, 2, 1, 8, 0)),
                Some(date_time(2024, 2, 29, 17, 0)),
            ),
            availability(
                Some(date_time(2024, 3, 1, 8, 0)),
                Some(date_time(2024, 3, 31, 17, 0)),
            ),
        ])
    }

    #[test]
    fn container_basics() {
        let mut table = AvailabilityTable::new();
        assert!(table.is_empty());

        table.add(availability(None, None));
        assert_eq!(table.len(), 1);
        assert_eq!(table.get(0).map(|entry| entry.get_units()), Some(Some(1.0)));
        assert_eq!(table.get(1), None);
        assert_eq!(table.iter().count(), 1);
        assert_eq!(table.as_slice().len(), 1);
    }

    #[test]
    fn get_entry_by_date_finds_matching_range() {
        let table = sample_table();

        let hit = table.get_entry_by_date(date_time(2024, 2, 15, 9, 0));
        assert_eq!(
            hit.map(|entry| entry.get_range().get_start()),
            Some(Some(date_time(2024, 2, 1, 8, 0)))
        );

// Falls before all ranges
        assert!(table
            .get_entry_by_date(date_time(2023, 12, 31, 9, 0))
            .is_none());

// Falls after all ranges
        assert!(table
            .get_entry_by_date(date_time(2024, 12, 31, 9, 0))
            .is_none());
    }

    #[test]
    fn has_default_date_range_detects_defaults() {
// An empty table is treated as default
        assert!(AvailabilityTable::new().has_default_date_range());

// A single empty range is treated as default
        let mut table = AvailabilityTable::new();
        table.add(availability(None, None));
        assert!(table.has_default_date_range());

// A single NA range (1984-01-01 -> later than 2049-12-31 23:58) is treated as default
        let mut table = AvailabilityTable::new();
        table.add(availability(
            Some(LocalDateTimeHelper::start_date_na()),
            Some(date_time(2050, 1, 1, 0, 0)),
        ));
        assert!(table.has_default_date_range());

// A single concrete range is not default
        let mut table = AvailabilityTable::new();
        table.add(availability(
            Some(date_time(2024, 1, 1, 8, 0)),
            Some(date_time(2024, 1, 31, 17, 0)),
        ));
        assert!(!table.has_default_date_range());

// Multiple ranges are not default
        assert!(!sample_table().has_default_date_range());
    }

    #[test]
    fn available_from_and_to_cover_range_boundaries() {
        let mut table = sample_table();

// Falls within the first range -> returns that range's start
        assert_eq!(
            table.available_from(date_time(2024, 1, 15, 9, 0)),
            Some(date_time(2024, 1, 1, 8, 0))
        );

// Falls between two ranges -> returns the previous range's end + 1 minute
        assert_eq!(
            table.available_from(date_time(2024, 1, 31, 18, 0)),
            Some(date_time(2024, 1, 31, 17, 1))
        );

// Falls within the third range -> returns that range's start
        assert_eq!(
            table.available_from(date_time(2024, 3, 31, 17, 0)),
            Some(date_time(2024, 3, 1, 8, 0))
        );

        let mut table = sample_table();
// Falls within the first range -> returns that range's end
        assert_eq!(
            table.available_to(date_time(2024, 1, 15, 9, 0)),
            Some(date_time(2024, 1, 31, 17, 0))
        );

// Falls between two ranges -> returns the next range's start - 1 minute
        assert_eq!(
            table.available_to(date_time(2024, 1, 31, 18, 0)),
            Some(date_time(2024, 2, 1, 7, 59))
        );
    }

    #[test]
    fn available_from_and_to_return_none_for_default_table() {
        let mut table = AvailabilityTable::new();
        table.add(availability(None, None));

        assert_eq!(table.available_from(date_time(2024, 1, 15, 9, 0)), None);

        let mut table = AvailabilityTable::new();
        table.add(availability(None, None));
        assert_eq!(table.available_to(date_time(2024, 1, 15, 9, 0)), None);
    }

    #[test]
    fn end_date_na_matches_java_constant() {
// Java: `AvailabilityTable.END_DATE_NA` (this class's own constant)
        assert_eq!(
            AvailabilityTable::end_date_na(),
            date_time(2049, 12, 31, 23, 58)
        );
// Differs from the helper's value (23:59); the two are not interchangeable
        assert_eq!(
            LocalDateTimeHelper::end_date_na(),
            date_time(2049, 12, 31, 23, 59)
        );
        assert_ne!(
            AvailabilityTable::end_date_na(),
            LocalDateTimeHelper::end_date_na()
        );
    }

/// The **usage site** takes this class's constant (23:58): `end.isAfter(END_DATE_NA)` in `has_default_date_range`.
    ///
/// The end time is set to 2049-12-31 23:58:30, which falls exactly between the two constants: by this class's 23:58 it is judged NA
/// (default range); if the helper's 23:59 were used by mistake it would be judged a concrete range.
    #[test]
    fn has_default_date_range_uses_the_class_end_date_na() {
        let mut table = AvailabilityTable::new();
        table.add(availability(
            Some(LocalDateTimeHelper::start_date_na()),
            Some(date_time_with_seconds(2049, 12, 31, 23, 58, 30)),
        ));

        assert!(table.has_default_date_range());
    }

/// The **usage site** takes this class's constant (23:58): `rangeEnd.isAfter(END_DATE_NA)` in `available_from`.
    ///
/// The first range ends at 23:58:30 and the target date is 23:58:40: by this class's 23:58, that end time counts as "beyond NA"
/// open range, so the target date falls within the range -> returns the range start; if the helper's 23:59 were used by mistake,
/// it would instead return "the previous row's end time + 1 minute" (23:58:31).
    #[test]
    fn available_from_uses_the_class_end_date_na() {
        let mut table = AvailabilityTable::new();
        table.add(availability(
            Some(date_time(2023, 10, 10, 0, 0)),
            Some(date_time_with_seconds(2049, 12, 31, 23, 58, 30)),
        ));
        table.add(availability(Some(date_time(2050, 6, 1, 0, 0)), None));

        assert_eq!(
            table.available_from(date_time_with_seconds(2049, 12, 31, 23, 58, 40)),
            Some(date_time(2023, 10, 10, 0, 0))
        );
    }

/// The **usage site** takes the helper constant (23:59): in `availableTo`
    /// `rangeEnd = range.getEnd() == null ? LocalDateTimeHelper.END_DATE_NA : ...`.
    ///
/// for a single range "starting at a concrete date and ending empty (open range)": Java uses the helper's 23:59,
/// and the trailing `rangeEnd.isAfter(END_DATE_NA /* 23:58 */)` check is true -> returns `null`.
/// If this class's 23:58 were used by mistake, `23:58 > 23:58` would be false -> it would wrongly return that 23:58.
    #[test]
    fn available_to_uses_the_helper_end_date_na_for_open_ended_ranges() {
        let mut table = AvailabilityTable::new();
        table.add(availability(Some(date_time(2024, 1, 1, 8, 0)), None));

        assert_eq!(table.available_to(date_time(2024, 6, 1, 9, 0)), None);
    }

/// The **usage site** takes this class's constant (23:58): the trailing `rangeEnd.isAfter(END_DATE_NA)` in `available_to`.
    ///
/// The range ends at 23:58:30: by this class's 23:58 it is judged "beyond NA" -> returns `None`;
/// if the helper's 23:59 were used by mistake, it would return the range end time itself.
    #[test]
    fn available_to_uses_the_class_end_date_na() {
        let mut table = AvailabilityTable::new();
        table.add(availability(
            Some(date_time(2023, 10, 10, 0, 0)),
            Some(date_time_with_seconds(2049, 12, 31, 23, 58, 30)),
        ));

        assert_eq!(table.available_to(date_time(2024, 1, 1, 0, 0)), None);
    }
}
