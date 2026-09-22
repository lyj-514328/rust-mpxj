//! Corresponds to Java: `org/mpxj/mpp/CostRateTableFactory.java`
//!
//! Reads a resource's cost rate table: in an MPP file each table consists of several 44-byte records, and each record gives
//! the standard rate / overtime rate / cost per use / end date; the start date is derived as "the previous record's end date + 1 minute".
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `ProjectFile m_file` | `Rc<RefCell<ProjectFile>>` | Shared per the crate-wide convention; project properties are read only in `RateHelper.convertFromHours` |
//! | `public void process(Resource resource, int index, byte[] data)` | `process(&self, resource: &mut Resource, index: i32, data: Option<&[u8]>` | `null` → `None`; `setCostRateTable` requires `&mut Resource` |
//! | `RateHelper.convertFromHours(m_file.getProjectProperties(), value, units)` | [`RateHelper::convert_from_hours`] (`crate::rate_helper`, i.e. Java's `org.mpxj.common.RateHelper`) | A single implementation shared with `MPPUtility.convertRateFromHours` (previously this file and `mpp14_reader` each had a copy; they have been merged) |
//! | `MPPUtility.getTimestampFromTenths` returns `null` | `Option<NaiveDateTime>`, and the record is skipped when no value is available | Java would NPE here; the Rust side treats unparseable records as invalid data |
//! | `endDate.minusMinutes(1)` / `plusMinutes(1)` | `checked_sub_signed` / `checked_add_signed` | Keeps the original value on overflow (does not panic) |
//! | `ByteArrayHelper.getShort` | `get_short_safe` | Returns 0 when out of bounds |
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: not used by this class.
//!
//! Skipped Java members: none.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::Timelike;

use crate::common::local_date_time_helper::LocalDateTimeHelper;
use crate::common::number_helper::NumberHelper;
use crate::cost_rate_table::CostRateTable;
use crate::cost_rate_table_entry::CostRateTableEntry;
use crate::mpp::abstract_calendar_factory::get_short_safe;
use crate::mpp::mpp_utility::MPPUtility;
use crate::project_file::ProjectFile;
use crate::rate::Rate;
use crate::rate_helper::RateHelper;
use crate::resource::Resource;
use crate::resource_field::ResourceField;
use crate::time_unit::TimeUnit;

/// Length of a single rate record.
///
/// Corresponds to Java: `for (int i = 16; i + 44 <= data.length; i += 44)`
const ITEM_SIZE: usize = 44;

/// Start offset of the record area.
///
/// Corresponds to Java: `int i = 16`
const DATA_OFFSET: usize = 16;

/// Common code for reading a resource cost rate table.
///
/// Corresponds to Java: `final class CostRateTableFactory`
pub struct CostRateTableFactory {
    /// Corresponds to Java field: `private final ProjectFile m_file;`
    file: Rc<RefCell<ProjectFile>>,
}

impl CostRateTableFactory {
/// Constructor.
    ///
    /// Corresponds to Java: `public CostRateTableFactory(ProjectFile file)`
    pub fn new(file: Rc<RefCell<ProjectFile>>) -> Self {
        Self { file }
    }

/// Build (and write) one of a resource's cost rate tables from a block of data.
    ///
    /// Corresponds to Java: `public void process(Resource resource, int index, byte[] data)`
    pub fn process(&self, resource: &mut Resource, index: i32, data: Option<&[u8]>) {
        // Java: List<CostRateTableEntry> entries = new ArrayList<>();
        let mut entries: Vec<CostRateTableEntry> = Vec::new();

        //
        // Extract core data
        //
        match data {
            None => {
                //
                // MS Project economises by not actually storing the first cost rate
                // table if it doesn't need to, so we take this into account here.
                //
                if index == 0 {
                    // Java: Rate standardRate = resource.getStandardRate() == null ? Rate.ZERO
                    //                         : (Rate) resource.getCachedValue(ResourceField.STANDARD_RATE);
                    let standard_rate = if resource.get_standard_rate().is_none() {
                        Rate::ZERO
                    } else {
// The cached value always exists on the Java side (otherwise it is null); the Rust side falls back to a 0 rate
                        cached_rate(resource, ResourceField::StandardRate).unwrap_or(Rate::ZERO)
                    };

                    // Java: Rate overtimeRate = resource.getOvertimeRate() == null ? Rate.ZERO
                    //                         : (Rate) resource.getCachedValue(ResourceField.OVERTIME_RATE);
                    let overtime_rate = if resource.get_overtime_rate().is_none() {
                        Rate::ZERO
                    } else {
                        cached_rate(resource, ResourceField::OvertimeRate).unwrap_or(Rate::ZERO)
                    };

                    // Java: Number costPerUse = resource.getCostPerUse() == null ? NumberHelper.DOUBLE_ZERO
                    //                        : (Number) resource.getCachedValue(ResourceField.COST_PER_USE);
                    let cost_per_use = if resource.get_cost_per_use().is_none() {
                        Some(NumberHelper::DOUBLE_ZERO)
                    } else {
                        cached_double(resource, ResourceField::CostPerUse)
                    };

                    // Java: LocalDateTime endDate = CostRateTableEntry.DEFAULT_ENTRY.getEndDate();
                    let end_date = CostRateTableEntry::default_entry().get_end_date();

                    // Java: entries.add(new CostRateTableEntry(null, endDate, costPerUse, standardRate, overtimeRate));
                    entries.push(CostRateTableEntry::new(
                        None,
                        end_date,
                        cost_per_use,
                        &[standard_rate, overtime_rate],
                    ));
                } else {
                    // Java: entries.add(CostRateTableEntry.DEFAULT_ENTRY);
                    entries.push(CostRateTableEntry::default_entry());
                }
            }

            Some(data) => {
// Java: m_file.getProjectProperties() (used by RateHelper.convertFromHours)
                let file_ref = self.file.try_borrow().ok();
                let defaults = file_ref.as_ref().map(|file| file.get_project_properties());

                // Java: RateHelper.convertFromHours(m_file.getProjectProperties(), value, units)
// (keeps the original value when the project file is mutably borrowed -- the file is always available on the Java side, so this case does not occur)
                let convert_from_hours = |value: f64, target_units: TimeUnit| match defaults {
                    Some(defaults) => RateHelper::convert_from_hours(defaults, value, target_units),
                    None => Rate::new(value, target_units),
                };

                let mut i = DATA_OFFSET;
                while i + ITEM_SIZE <= data.len() {
                    // Java: TimeUnit standardRateFormat = getFormat(ByteArrayHelper.getShort(data, i + 8));
                    let standard_rate_format = self.get_format(get_short_safe(data, i + 8));
                    // Java: Rate standardRate = RateHelper.convertFromHours(m_file.getProjectProperties(), MPPUtility.getDouble(data, i), standardRateFormat);
                    let standard_rate =
                        convert_from_hours(MPPUtility::get_double(data, i), standard_rate_format);

                    // Java: TimeUnit overtimeRateFormat = getFormat(ByteArrayHelper.getShort(data, i + 24));
                    let overtime_rate_format = self.get_format(get_short_safe(data, i + 24));
                    // Java: Rate overtimeRate = RateHelper.convertFromHours(..., MPPUtility.getDouble(data, i + 16), overtimeRateFormat);
                    let overtime_rate = convert_from_hours(
                        MPPUtility::get_double(data, i + 16),
                        overtime_rate_format,
                    );

                    // Java: Double costPerUse = NumberHelper.getDouble(MPPUtility.getDouble(data, i + 32) / 100.0);
                    let cost_per_use = NumberHelper::get_double(Some(
                        MPPUtility::get_double(data, i + 32) / 100.0,
                    ));

                    // Java: LocalDateTime endDate = MPPUtility.getTimestampFromTenths(data, i + 40);
// (Java would NPE when no value is available; the Rust side treats that record as invalid and skips it)
                    let Some(mut end_date) =
                        MPPUtility::get_timestamp_from_tenths(data, i + 40)
                    else {
                        i += ITEM_SIZE;
                        continue;
                    };

                    if end_date > LocalDateTimeHelper::end_date_na() {
                        // Java: endDate = LocalDateTimeHelper.END_DATE_NA;
                        end_date = LocalDateTimeHelper::end_date_na();
                    } else {
                        //
                        // MPP files only store the end date of the range, and typically this
                        // will be represented as the last minute of the range, e,g, 07:59,
                        // so the next range starts at 08:00. Occasionally we see the start time of the
                        // next range stored here, so this heuristic is used to identify what looks
                        // like a start time (minutes divisible by 10) and subtracts one minute so that
                        // the next range starts at the correct time.
                        //
                        // Java: int minutes = endDate.getMinute(); if ((minutes % 5) == 0) { endDate = endDate.minusMinutes(1); }
                        let minutes = end_date.minute();
                        if minutes % 5 == 0 {
                            end_date = chrono::Duration::try_minutes(1)
                                .and_then(|delta| end_date.checked_sub_signed(delta))
                                .unwrap_or(end_date);
                        }
                    }

                    // Heuristic to weed out invalid entries: if the timestamp has seconds
                    // the whole entry is likely to be invalid and can be ignored.
                    // Java: if (endDate.getSecond() != 0) { continue; }
                    if end_date.second() != 0 {
                        i += ITEM_SIZE;
                        continue;
                    }

                    // Java: entries.add(new CostRateTableEntry(null, endDate, costPerUse, standardRate, overtimeRate));
                    entries.push(CostRateTableEntry::new(
                        None,
                        Some(end_date),
                        Some(cost_per_use),
                        &[standard_rate, overtime_rate],
                    ));

                    i += ITEM_SIZE;
                }
            }
        }

        //
        // Populate start dates
        //
        // Java: Collections.sort(entries);
        entries.sort();

        // Java: CostRateTable result = new CostRateTable();
        let mut result = CostRateTable::new();

        for entry_index in 0..entries.len() {
            // Java: if (i == 0) { startDate = LocalDateTimeHelper.START_DATE_NA; }
            //       else { startDate = entries.get(i - 1).getEndDate().plusMinutes(1); }
            let start_date = if entry_index == 0 {
                Some(LocalDateTimeHelper::start_date_na())
            } else {
                entries[entry_index - 1].get_end_date().and_then(|end_date| {
                    end_date.checked_add_signed(chrono::Duration::try_minutes(1)?)
                })
            };

            let entry = &entries[entry_index];
            // Java: result.add(new CostRateTableEntry(startDate, entry.getEndDate(), entry.getCostPerUse(), entry.getStandardRate(), entry.getOvertimeRate()));
            result.add(CostRateTableEntry::new(
                start_date,
                entry.get_end_date(),
                entry.get_cost_per_use(),
                &[entry.get_standard_rate(), entry.get_overtime_rate()],
            ));
        }

        // Java: resource.setCostRateTable(index, result);
        resource.set_cost_rate_table(index, result);
    }

/// Convert an integer into a time unit.
    ///
    /// Corresponds to Java: `private TimeUnit getFormat(int format)`
    fn get_format(&self, format: i32) -> TimeUnit {
        if format == 0xFFFF {
            return TimeUnit::Hours;
        }

        let result = MPPUtility::get_work_time_units(format);

        // For "flat" rates (for example, for cost or material resources) where there is
        // no time component, the MPP file stores a time unit which we recognise
        // as elapsed minutes. If we encounter this, reset the time units to hours
        // so we don't try to change the value.
        // TODO: improve handling of  cost and material rates
        if result == TimeUnit::ElapsedMinutes {
            TimeUnit::Hours
        } else {
            result
        }
    }
}

/// Get the cached rate value of a resource field.
///
/// Corresponds to Java: `(Rate) resource.getCachedValue(field)`
fn cached_rate(resource: &Resource, field: ResourceField) -> Option<Rate> {
    resource
        .get_cached_value(crate::field_key::FieldKey::from(field))
        .and_then(|value| value.as_rate())
}

/// Get the cached numeric value of a resource field.
///
/// Corresponds to Java: `(Number) resource.getCachedValue(field)`
fn cached_double(resource: &Resource, field: ResourceField) -> Option<f64> {
    resource
        .get_cached_value(crate::field_key::FieldKey::from(field))
        .and_then(|value| value.as_double())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    use crate::project_file::ProjectFile;
    use crate::time_unit_defaults::TimeUnitDefaults;

/// Timestamp: a 4-byte integer in units of "seconds since 1983-12-31 / 6".
    fn timestamp_tenths(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> i32 {
        let epoch = NaiveDate::from_ymd_opt(1983, 12, 31)
            .expect("valid date")
            .and_hms_opt(0, 0, 0)
            .expect("valid time");
        let value = date_time(year, month, day, hour, minute);
        ((value - epoch).num_seconds() / 6) as i32
    }

    fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .expect("valid date")
            .and_hms_opt(hour, minute, 0)
            .expect("valid time")
    }

/// A single 44-byte rate record.
    fn record(standard: f64, overtime: f64, cost_per_use: f64, end_tenths: i32) -> [u8; 44] {
        let mut data = [0u8; 44];
        data[0..8].copy_from_slice(&standard.to_le_bytes());
// Rate format: 0xFFFF → TimeUnit.HOURS (no conversion)
        data[8..10].copy_from_slice(&0xFFFFu16.to_le_bytes());
        data[16..24].copy_from_slice(&overtime.to_le_bytes());
        data[24..26].copy_from_slice(&0xFFFFu16.to_le_bytes());
        data[32..40].copy_from_slice(&cost_per_use.to_le_bytes());
        data[40..44].copy_from_slice(&end_tenths.to_le_bytes());
        data
    }

/// A 16-byte block header plus several 44-byte records.
    fn data_block(records: &[[u8; 44]]) -> Vec<u8> {
        let mut data = vec![0u8; 16];
        for record in records {
            data.extend_from_slice(record);
        }
        data
    }

    fn resource() -> (Rc<RefCell<ProjectFile>>, Rc<RefCell<Resource>>) {
        let file = ProjectFile::new();
        let resource = file.borrow_mut().add_resource();
        (file, resource)
    }

    #[test]
    fn missing_first_cost_rate_table_is_reconstructed_from_the_resource() {
        let (file, resource) = resource();
        let factory = CostRateTableFactory::new(Rc::clone(&file));

        factory.process(&mut resource.borrow_mut(), 0, None);

        let resource_ref = resource.borrow();
        let table = resource_ref.get_cost_rate_table(0).expect("table A");
        assert_eq!(table.len(), 1);

        let entry = table.get(0).expect("first entry");
        assert_eq!(
            entry.get_start_date(),
            Some(LocalDateTimeHelper::start_date_na())
        );
        assert_eq!(
            entry.get_end_date(),
            Some(LocalDateTimeHelper::end_date_na())
        );
// Java: `getCostPerUse()` takes its value from the current rate table row (here 0.0 for the default row, not null),
// so it takes the `(Number) resource.getCachedValue(ResourceField.COST_PER_USE)` branch;
// this field was never read from the file → the cache is null, and the Rust side likewise gets None.
        assert_eq!(entry.get_cost_per_use(), None);
        assert_eq!(entry.get_standard_rate(), Rate::ZERO);
        assert_eq!(entry.get_overtime_rate(), Rate::ZERO);
    }

    #[test]
    fn missing_other_cost_rate_tables_use_the_default_entry() {
        let (file, resource) = resource();
        let factory = CostRateTableFactory::new(Rc::clone(&file));

        factory.process(&mut resource.borrow_mut(), 3, None);

        let resource_ref = resource.borrow();
        let table = resource_ref.get_cost_rate_table(3).expect("table D");
        assert_eq!(table.len(), 1);
        assert_eq!(
            table.get(0).and_then(|entry| entry.get_end_date()),
            Some(LocalDateTimeHelper::end_date_na())
        );
        assert_eq!(table.get(0).and_then(|entry| entry.get_cost_per_use()), Some(0.0));
    }

    #[test]
    fn data_block_records_are_parsed_sorted_and_chained() {
// Record A: end time 17:01 (minute value 1 does not trigger the "minus one minute" heuristic)
// Record B: end time 18:00 (minute value 0 → subtract one minute to get 17:59)
// Record C: the seconds of the end time are non-zero → the whole record is ignored
        let data = data_block(&[
            record(
                100.0,
                150.0,
                1250.0,
                timestamp_tenths(2024, 12, 31, 17, 1),
            ),
            record(200.0, 250.0, 0.0, timestamp_tenths(2024, 6, 30, 18, 0)),
            record(
                300.0,
                350.0,
                0.0,
                timestamp_tenths(2024, 8, 1, 10, 0) + 1,
            ),
        ]);

        let (file, resource) = resource();
        let factory = CostRateTableFactory::new(Rc::clone(&file));
        factory.process(&mut resource.borrow_mut(), 0, Some(&data));

        let resource_ref = resource.borrow();
        let table = resource_ref.get_cost_rate_table(0).expect("table A");

// C is ignored, A / B are in ascending end-time order
        assert_eq!(table.len(), 2);

        let first = table.get(0).expect("first entry");
        assert_eq!(first.get_end_date(), Some(date_time(2024, 6, 30, 17, 59)));
        assert_eq!(first.get_start_date(), Some(LocalDateTimeHelper::start_date_na()));
        assert_eq!(first.get_standard_rate(), Rate::new(200.0, TimeUnit::Hours));
        assert_eq!(first.get_overtime_rate(), Rate::new(250.0, TimeUnit::Hours));

        let second = table.get(1).expect("second entry");
// Start time = previous record's end time + 1 minute
        assert_eq!(second.get_start_date(), Some(date_time(2024, 6, 30, 18, 0)));
        assert_eq!(second.get_end_date(), Some(date_time(2024, 12, 31, 17, 1)));
        assert_eq!(second.get_standard_rate(), Rate::new(100.0, TimeUnit::Hours));
        assert_eq!(second.get_overtime_rate(), Rate::new(150.0, TimeUnit::Hours));
        // 1250 / 100 = 12.5
        assert_eq!(second.get_cost_per_use(), Some(12.5));
    }

    #[test]
    fn truncated_data_does_not_panic() {
        let (file, resource) = resource();
        let factory = CostRateTableFactory::new(Rc::clone(&file));

// Less than one record / truncated record: Java throws an array index out of bounds exception
        for data in [vec![], vec![0u8; 16], vec![0u8; 20], vec![0u8; 59]] {
            factory.process(&mut resource.borrow_mut(), 0, Some(&data));
        }

        assert!(resource.borrow().get_cost_rate_table(0).is_some());
    }

    #[test]
    fn formats_are_converted_to_time_units() {
        let (file, _resource) = resource();
        let factory = CostRateTableFactory::new(file);

        assert_eq!(factory.get_format(0xFFFF), TimeUnit::Hours);
        // getWorkTimeUnits(3) → TimeUnit.DAYS
        assert_eq!(factory.get_format(3), TimeUnit::Days);
// getWorkTimeUnits(8) → ELAPSED_MINUTES → falls back to HOURS
        assert_eq!(factory.get_format(8), TimeUnit::Hours);
        assert_eq!(factory.get_format(1), TimeUnit::Minutes);
    }

    #[test]
    fn rate_conversion_matches_rate_helper() {
// The conversion is provided by `RateHelper` (this file no longer carries its own implementation)
        let defaults = TimeUnitDefaults::new();

        assert_eq!(
            RateHelper::convert_from_hours(&defaults, 60.0, TimeUnit::Minutes),
            Rate::new(1.0, TimeUnit::Minutes)
        );
        assert_eq!(
            RateHelper::convert_from_hours(&defaults, 12.345, TimeUnit::Hours),
            Rate::new(12.35, TimeUnit::Hours)
        );
// 480 minutes per day: 100/h -> 800/d
        assert_eq!(
            RateHelper::convert_from_hours(&defaults, 100.0, TimeUnit::Days),
            Rate::new(800.0, TimeUnit::Days)
        );
        assert_eq!(
            LocalDateTimeHelper::start_date_na().time(),
            NaiveTime::from_hms_opt(0, 0, 0).expect("valid time")
        );
    }
}
