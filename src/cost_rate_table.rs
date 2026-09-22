//! Corresponds to Java: `org/mpxj/CostRateTable.java`

use chrono::NaiveDateTime;

use crate::cost_rate_table_entry::CostRateTableEntry;
use crate::local_date_time_range::LocalDateTimeRange;

/// Cost rate table for a resource.
///
/// Corresponds to Java: `public class CostRateTable extends ArrayList<CostRateTableEntry>`
///
/// Porting notes: the Java side extends `ArrayList<CostRateTableEntry>`; the Rust side instead holds
/// `Vec<CostRateTableEntry>` in a standalone container, and provides only the collection operations needed for translation (length,
/// indexing, append, iterate).
///
/// Skipped Java methods: none (methods inherited from `ArrayList` are not counted as part of this class's translation scope).
#[derive(Debug, Default)]
pub struct CostRateTable {
/// The cost rate rows in the table.
    ///
/// Corresponds to Java: internal storage of `ArrayList<CostRateTableEntry>`.
    entries: Vec<CostRateTableEntry>,
}

impl CostRateTable {
/// Constructor.
    ///
/// Corresponds to Java: the default constructor of `ArrayList<CostRateTableEntry>`.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

/// Build a table from existing rows.
    ///
    /// Corresponds to Java: `ArrayList<CostRateTableEntry>(Collection<? extends CostRateTableEntry> c)`.
    pub fn from_entries(entries: Vec<CostRateTableEntry>) -> Self {
        Self { entries }
    }

/// Append a row.
    ///
    /// Corresponds to Java: `ArrayList.add(E element)`.
    pub fn add(&mut self, entry: CostRateTableEntry) {
        self.entries.push(entry);
    }

/// Get a row by index; returns `None` when out of bounds (Java throws `IndexOutOfBoundsException`).
    ///
    /// Corresponds to Java: `ArrayList.get(int index)`.
    pub fn get(&self, index: usize) -> Option<&CostRateTableEntry> {
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

/// Iterate the rows in the table.
    ///
    /// Corresponds to Java: `ArrayList.iterator()`.
    pub fn iter(&self) -> std::slice::Iter<'_, CostRateTableEntry> {
        self.entries.iter()
    }

/// Access the rows in the table as a slice.
    ///
/// Corresponds to Java: the list view of `ArrayList`.
    pub fn as_slice(&self) -> &[CostRateTableEntry] {
        &self.entries
    }

/// Get the table rows that intersect the given date range.
    ///
    /// Corresponds to Java: `public List<CostRateTableEntry> getEntriesByRange(LocalDateTimeRange range)`
    pub fn get_entries_by_range(&self, range: &LocalDateTimeRange) -> Vec<&CostRateTableEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.get_range().intersects_with(range))
            .collect()
    }

/// Get the table row in effect on the given date.
    ///
    /// Corresponds to Java: `public CostRateTableEntry getEntryByDate(LocalDateTime date)`
    ///
/// The Java side throws an NPE for rows with a `null` end date; the Rust side skips rows with a missing end date.
    pub fn get_entry_by_date(&self, date: NaiveDateTime) -> Option<&CostRateTableEntry> {
        let mut result = None;

        for entry in &self.entries {
            if entry.get_end_date().is_some_and(|end_date| date < end_date) {
                result = Some(entry);
                break;
            }
        }

        result
    }

/// Get the index of the table row in effect on the given date, returning `-1` when not found.
    ///
    /// Corresponds to Java: `public int getIndexByDate(LocalDateTime date)`
    pub fn get_index_by_date(&self, date: NaiveDateTime) -> i32 {
        let mut result = -1;
        let mut index = 0;

        for entry in &self.entries {
            if entry.get_end_date().is_some_and(|end_date| date < end_date) {
                result = index;
                break;
            }
            index += 1;
        }

        result
    }

/// Determine whether the table contains any row with a non-zero rate.
    ///
    /// Corresponds to Java: `public boolean tableIsPopulated()`
    pub fn table_is_populated(&self) -> bool {
        !(self.entries.is_empty() || (self.entries.len() == 1 && self.all_entries_are_default()))
    }

/// Determine whether all rates are 0.
    ///
    /// Corresponds to Java: `private boolean allEntriesAreDefault()`
    fn all_entries_are_default(&self) -> bool {
        for entry in &self.entries {
            for rate_index in 0..CostRateTableEntry::MAX_RATES {
                let rate = entry.get_rate(rate_index);
                if let Some(rate) = rate {
                    if rate.get_amount() != 0.0 {
                        return false;
                    }
                }
            }
        }

        true
    }

/// Maximum number of cost rate tables.
    ///
    /// Corresponds to Java: `public static final int MAX_TABLES = 5;`
    pub const MAX_TABLES: i32 = 5;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rate::Rate;
    use crate::time_unit::TimeUnit;
    use chrono::{NaiveDate, NaiveTime};

/// Build a date-time for tests.
    fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(year, month, day).expect("valid date"),
            NaiveTime::from_hms_opt(hour, minute, 0).expect("valid time"),
        )
    }

/// Build a cost rate row.
    fn entry(
        start: NaiveDateTime,
        end: Option<NaiveDateTime>,
        standard_rate: f64,
        overtime_rate: f64,
    ) -> CostRateTableEntry {
        CostRateTableEntry::new(
            Some(start),
            end,
            Some(0.0),
            &[
                Rate::new(standard_rate, TimeUnit::Hours),
                Rate::new(overtime_rate, TimeUnit::Hours),
            ],
        )
    }

/// Build a cost rate table with two rows for January / February.
    fn sample_table() -> CostRateTable {
        CostRateTable::from_entries(vec![
            entry(
                date_time(2024, 1, 1, 8, 0),
                Some(date_time(2024, 1, 31, 17, 0)),
                100.0,
                150.0,
            ),
            entry(
                date_time(2024, 2, 1, 8, 0),
                Some(date_time(2024, 2, 29, 17, 0)),
                110.0,
                160.0,
            ),
        ])
    }

    #[test]
    fn container_basics() {
        let mut table = CostRateTable::new();
        assert!(table.is_empty());

        table.add(entry(
            date_time(2024, 1, 1, 8, 0),
            Some(date_time(2024, 1, 31, 17, 0)),
            100.0,
            150.0,
        ));
        assert_eq!(table.len(), 1);
        assert!(table.get(0).is_some());
        assert!(table.get(1).is_none());
        assert_eq!(table.iter().count(), 1);
        assert_eq!(table.as_slice().len(), 1);
        assert_eq!(CostRateTable::MAX_TABLES, 5);
    }

    #[test]
    fn get_entries_by_range_filters_intersecting_entries() {
        let table = sample_table();

// Intersects only the January row
        let range = LocalDateTimeRange::new(
            Some(date_time(2024, 1, 15, 0, 0)),
            Some(date_time(2024, 1, 20, 0, 0)),
        );
        let matches = table.get_entries_by_range(&range);
        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].get_standard_rate(),
            Rate::new(100.0, TimeUnit::Hours)
        );

// Spans both rows
        let range = LocalDateTimeRange::new(
            Some(date_time(2024, 1, 15, 0, 0)),
            Some(date_time(2024, 2, 15, 0, 0)),
        );
        assert_eq!(table.get_entries_by_range(&range).len(), 2);

// Does not intersect any row
        let range = LocalDateTimeRange::new(
            Some(date_time(2025, 1, 15, 0, 0)),
            Some(date_time(2025, 2, 15, 0, 0)),
        );
        assert!(table.get_entries_by_range(&range).is_empty());
    }

    #[test]
    fn get_entry_and_index_by_date_return_first_matching_entry() {
        let table = sample_table();

        let hit = table.get_entry_by_date(date_time(2024, 1, 15, 9, 0));
        assert_eq!(
            hit.map(|entry| entry.get_standard_rate()),
            Some(Rate::new(100.0, TimeUnit::Hours))
        );
        assert_eq!(table.get_index_by_date(date_time(2024, 1, 15, 9, 0)), 0);

        let hit = table.get_entry_by_date(date_time(2024, 2, 15, 9, 0));
        assert_eq!(
            hit.map(|entry| entry.get_standard_rate()),
            Some(Rate::new(110.0, TimeUnit::Hours))
        );
        assert_eq!(table.get_index_by_date(date_time(2024, 2, 15, 9, 0)), 1);

// Later than all rows
        assert!(table.get_entry_by_date(date_time(2025, 1, 15, 9, 0)).is_none());
        assert_eq!(table.get_index_by_date(date_time(2025, 1, 15, 9, 0)), -1);
    }

    #[test]
    fn table_is_populated_detects_default_only_table() {
// An empty table is not considered populated
        assert!(!CostRateTable::new().table_is_populated());

// Only one row with all rates zero -> not populated
        let mut table = CostRateTable::new();
        table.add(entry(date_time(2024, 1, 1, 8, 0), None, 0.0, 0.0));
        assert!(!table.table_is_populated());

// Only one row but with a non-zero rate -> populated
        let mut table = CostRateTable::new();
        table.add(entry(date_time(2024, 1, 1, 8, 0), None, 100.0, 0.0));
        assert!(table.table_is_populated());

// Multiple rows -> populated
        assert!(sample_table().table_is_populated());
    }
}
