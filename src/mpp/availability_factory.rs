//! Corresponds to Java: `org/mpxj/mpp/AvailabilityFactory.java`
//!
//! Reads a resource's "availability table": in an MPP file each resource has an availability block,
//! within which each 20-byte record holds "start time + available units + end time".
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `public void process(Resource resource, byte[] data)` | `process(&self, resource: &Resource, data: Option<&[u8]>) -> Vec<Availability>` | Java appends directly to `resource.getAvailability()` and sorts; Rust's [`Resource::get_availability`] only returns a read-only `Ref<'_, AvailabilityTable>`, so the factory cannot write in place and instead **returns the rows to append** (the `data` branch is already sorted per Java semantics), which the caller writes via [`Resource::add_availability`] (all 4 MPP readers are wired this way) |
//! | `byte[] data` | `Option<&[u8]>` | Java's `null` means "the file has no availability block" |
//! | `LocalDateTime` | `Option<NaiveDateTime>` | `MPPUtility.getTimestampFromTenths` may yield no value (Java would NPE here); Rust keeps `None` |
//! | `endDate.minusMinutes(1)` | Subtract 1 minute only when a value is obtained | Same as above |
//! | `NumberHelper.getDouble(unitsValue / 100)` | `NumberHelper::get_double(Some(units_value / 100.0))` | Same-package convention |
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: not used by this class.
//!
//! Skipped Java members: none (Java's implicit default constructor is represented by [`AvailabilityFactory::new`]).

use crate::availability::Availability;
use crate::common::local_date_time_helper::LocalDateTimeHelper;
use crate::common::number_helper::NumberHelper;
use crate::field_key::FieldKey;
use crate::mpp::abstract_calendar_factory::get_short_safe;
use crate::mpp::mpp_utility::MPPUtility;
use crate::resource::Resource;
use crate::resource_field::ResourceField;

/// Length of each availability record.
///
/// Corresponds to Java: `offset += 20;`
const ITEM_SIZE: usize = 20;

/// Starting offset of the record area.
///
/// Corresponds to Java: `int offset = 12;`
const DATA_OFFSET: usize = 12;

/// Common code for reading a resource availability table.
///
/// Corresponds to Java: `final class AvailabilityFactory`
pub struct AvailabilityFactory;

impl AvailabilityFactory {
/// Constructor.
    ///
/// Corresponds to Java's implicit default constructor `AvailabilityFactory()`
    pub fn new() -> Self {
        Self
    }

/// Populate the resource's availability table.
    ///
    /// Corresponds to Java: `public void process(Resource resource, byte[] data)`
    ///
/// The return value is the rows this method should append to the resource's availability table (see "Differences from Java" in the file header).
    pub fn process(&self, resource: &Resource, data: Option<&[u8]>) -> Vec<Availability> {
        let mut table: Vec<Availability> = Vec::new();

        match data {
            None => {
// If there is no availability table, we construct one.
// Note the use of getCachedValue to ensure the values read from the file are used rather than trying to compute them.
                let mut available_from = resource
                    .get_cached_value(FieldKey::from(ResourceField::AvailableFrom))
                    .and_then(|value| value.as_date_time());
                let mut available_to = resource
                    .get_cached_value(FieldKey::from(ResourceField::AvailableTo))
                    .and_then(|value| value.as_date_time());

                if available_from.is_none() {
                    available_from = Some(LocalDateTimeHelper::start_date_na());
                }
                if available_to.is_none() {
                    available_to = Some(LocalDateTimeHelper::end_date_na());
                }

                let max_units = resource
                    .get_cached_value(FieldKey::from(ResourceField::MaxUnits))
                    .and_then(|value| value.as_double());

                table.push(Availability::new(available_from, available_to, max_units));
            }

            Some(data) => {
                // Java: int items = ByteArrayHelper.getShort(data, 0);
                let items = get_short_safe(data, 0);
                let mut offset = DATA_OFFSET;

                for _ in 0..items {
                    // Java: double unitsValue = MPPUtility.getDouble(data, offset + 4);
                    let units_value = MPPUtility::get_double(data, offset + 4);
                    if units_value != 0.0 {
                        // Java: LocalDateTime startDate = MPPUtility.getTimestampFromTenths(data, offset);
                        let start_date = MPPUtility::get_timestamp_from_tenths(data, offset);
                        // Java: LocalDateTime endDate = MPPUtility.getTimestampFromTenths(data, offset + 20);
                        //       endDate = endDate.minusMinutes(1);
// Note: Java takes this record's end time from "the start position of the next record" (in MPP, adjacent intervals
// share a timestamp at the boundary); this is translated as is.
                        let end_date = MPPUtility::get_timestamp_from_tenths(data, offset + 20)
                            .and_then(|end_date| {
                                end_date.checked_sub_signed(chrono::Duration::try_minutes(1)?)
                            });

                        // Java: Double units = NumberHelper.getDouble(unitsValue / 100);
                        let units = NumberHelper::get_double(Some(units_value / 100.0));

                        // Java: if (startDate.isBefore(LocalDateTimeHelper.START_DATE_NA)) { startDate = LocalDateTimeHelper.START_DATE_NA; }
                        let start_date = clamp_start_date(start_date);
                        // Java: if (endDate.isAfter(LocalDateTimeHelper.END_DATE_NA)) { endDate = LocalDateTimeHelper.END_DATE_NA; }
                        let end_date = clamp_end_date(end_date);

                        // Java: Availability item = new Availability(startDate, endDate, units);
                        //       table.add(item);
                        table.push(Availability::new(start_date, end_date, Some(units)));
                    }
                    offset += ITEM_SIZE;
                }

                // Java: Collections.sort(table);
                table.sort();
            }
        }

        table
    }
}

impl Default for AvailabilityFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Raise a start time earlier than `START_DATE_NA` up to `START_DATE_NA`.
///
/// Corresponds to Java: `if (startDate.isBefore(LocalDateTimeHelper.START_DATE_NA)) { ... }`
/// (Java would NPE on `null`; Rust keeps `None`).
fn clamp_start_date(value: Option<chrono::NaiveDateTime>) -> Option<chrono::NaiveDateTime> {
    let start_date_na = LocalDateTimeHelper::start_date_na();

    value.map(|value| if value < start_date_na { start_date_na } else { value })
}

/// Clamp an end time later than `END_DATE_NA` down to `END_DATE_NA`.
///
/// Corresponds to Java: `if (endDate.isAfter(LocalDateTimeHelper.END_DATE_NA)) { ... }`
/// (Java would NPE on `null`; Rust keeps `None`).
fn clamp_end_date(value: Option<chrono::NaiveDateTime>) -> Option<chrono::NaiveDateTime> {
    let end_date_na = LocalDateTimeHelper::end_date_na();

    value.map(|value| if value > end_date_na { end_date_na } else { value })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    use chrono::{NaiveDate, NaiveDateTime};

    use crate::field_value::FieldValue;
    use crate::project_file::ProjectFile;

/// Build a resource (attached to a standalone `ProjectFile`; ID allocation is handled by the container).
    ///
/// The file handle in the return value must stay alive, otherwise the weak context reference held by the resource becomes invalid.
    fn resource() -> (Rc<RefCell<ProjectFile>>, Rc<RefCell<Resource>>) {
        let file = ProjectFile::new();
        let resource = file.borrow_mut().add_resource();
        (file, resource)
    }

/// Timestamp: a 4-byte integer in units of "seconds since 1983-12-31 / 6".
    fn timestamp_tenths(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> i32 {
        let epoch = NaiveDate::from_ymd_opt(1983, 12, 31)
            .expect("valid date")
            .and_hms_opt(0, 0, 0)
            .expect("valid time");
        let value = date_time(year, month, day, hour, minute);
        ((value - epoch).num_seconds() / 6) as i32
    }

/// A 20-byte availability record: start time (4 bytes) + available units (8-byte double) + 8 unknown bytes.
    ///
/// Note: Java reads the end time using `offset + 20`, i.e. the start time of the **next record**
/// (in MPP, adjacent intervals share a timestamp at the boundary), so this test's data is laid out the same way.
    fn slot(start_tenths: i32, units_value: f64) -> [u8; 20] {
        let mut data = [0u8; 20];
        data[0..4].copy_from_slice(&start_tenths.to_le_bytes());
        data[4..12].copy_from_slice(&units_value.to_le_bytes());
        data
    }

    fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .expect("valid date")
            .and_hms_opt(hour, minute, 0)
            .expect("valid time")
    }

/// 12-byte block header (the first 2 bytes are the entry count) + several 20-byte records.
    fn data_block(items: u16, slots: &[[u8; 20]]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&items.to_le_bytes());
        data.extend_from_slice(&[0u8; 10]);
        for slot in slots {
            data.extend_from_slice(slot);
        }
        data
    }

    #[test]
    fn missing_availability_block_builds_a_default_entry() {
        let (_file, resource) = resource();

        let table = AvailabilityFactory::new().process(&resource.borrow(), None);

        assert_eq!(table.len(), 1);
        let range = table[0].get_range();
        assert_eq!(range.get_start(), Some(LocalDateTimeHelper::start_date_na()));
        assert_eq!(range.get_end(), Some(LocalDateTimeHelper::end_date_na()));
// MAX_UNITS not cached -> units is None (Java's null)
        assert_eq!(table[0].get_units(), None);
    }

    #[test]
    fn cached_values_are_used_for_the_default_entry() {
        let (_file, resource) = resource();

        {
            let mut resource_ref = resource.borrow_mut();
            resource_ref.set(
                FieldKey::from(ResourceField::AvailableFrom),
                Some(FieldValue::DateTime(date_time(2024, 1, 1, 8, 0))),
            );
            resource_ref.set(
                FieldKey::from(ResourceField::AvailableTo),
                Some(FieldValue::DateTime(date_time(2024, 12, 31, 17, 0))),
            );
            resource_ref.set(
                FieldKey::from(ResourceField::MaxUnits),
                Some(FieldValue::Double(75.0)),
            );
        }

        let table = AvailabilityFactory::new().process(&resource.borrow(), None);

        assert_eq!(table.len(), 1);
        assert_eq!(
            table[0].get_range().get_start(),
            Some(date_time(2024, 1, 1, 8, 0))
        );
        assert_eq!(
            table[0].get_range().get_end(),
            Some(date_time(2024, 12, 31, 17, 0))
        );
        assert_eq!(table[0].get_units(), Some(75.0));
    }

    #[test]
    fn data_block_entries_are_parsed_and_sorted() {
// Three records: the first two are valid (units non-zero), the third has 0 units and is skipped
        let data = data_block(
            3,
            &[
// From 2023-01-01 08:00, 50%
                slot(timestamp_tenths(2023, 1, 1, 8, 0), 5000.0),
// From 2023-06-30 17:01 (i.e. the previous record's end time + 1 minute), 100%
                slot(timestamp_tenths(2023, 6, 30, 17, 1), 10000.0),
// From 2024-07-01 00:01 (i.e. the previous record's end time + 1 minute), units 0 -> skipped
                slot(timestamp_tenths(2024, 7, 1, 0, 1), 0.0),
            ],
        );

        let (_file, resource) = resource();
        let table = AvailabilityFactory::new().process(&resource.borrow(), Some(&data));

// The 0 value is skipped; the remaining two are in ascending interval order
        assert_eq!(table.len(), 2);
        assert_eq!(
            table[0].get_range().get_start(),
            Some(date_time(2023, 1, 1, 8, 0))
        );
// End time = the next record's start time (17:01) minus 1 minute
        assert_eq!(
            table[0].get_range().get_end(),
            Some(date_time(2023, 6, 30, 17, 0))
        );
        assert_eq!(table[0].get_units(), Some(50.0));

        assert_eq!(
            table[1].get_range().get_start(),
            Some(date_time(2023, 6, 30, 17, 1))
        );
        assert_eq!(
            table[1].get_range().get_end(),
            Some(date_time(2024, 7, 1, 0, 0))
        );
        assert_eq!(table[1].get_units(), Some(100.0));
    }

    #[test]
    fn dates_outside_the_supported_range_are_clamped() {
// Start time earlier than START_DATE_NA, end time later than END_DATE_NA
        let data = data_block(
            2,
            &[
                slot(timestamp_tenths(1900, 1, 1, 0, 0), 10000.0),
                slot(timestamp_tenths(2100, 1, 1, 0, 1), 0.0),
            ],
        );

        let (_file, resource) = resource();
        let table = AvailabilityFactory::new().process(&resource.borrow(), Some(&data));

        assert_eq!(table.len(), 1);
        assert_eq!(
            table[0].get_range().get_start(),
            Some(LocalDateTimeHelper::start_date_na())
        );
        assert_eq!(
            table[0].get_range().get_end(),
            Some(LocalDateTimeHelper::end_date_na())
        );
    }

    #[test]
    fn truncated_data_does_not_panic() {
        let (_file, resource) = resource();
        let factory = AvailabilityFactory::new();

// When the data is insufficient each field is treated as 0 (Java throws an array out-of-bounds); it must not panic
        for data in [vec![], vec![0u8; 4], vec![1u8, 0u8], vec![5u8, 0u8, 0u8, 0u8]] {
            let table = factory.process(&resource.borrow(), Some(&data));
            assert!(table.len() <= 5);
        }
    }
}
