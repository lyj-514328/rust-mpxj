//! Corresponds to Java: `org/mpxj/mpp/MPP9CalendarFactory.java`
//!
//! Calendar reading specific to MPP9 (Microsoft Project 2000): the working calendar block offset differs from MPP12/14,
//! and the exception date block has no recursion rule either (a fixed 64 bytes per entry).
//!
//! # Restoring the inheritance relationships
//!
//! Java: `class MPP9CalendarFactory extends AbstractCalendarFactory`. Rust has no inheritance, so it is
//! restored by "composition + forwarding" (consistent with the approach used for `VarMeta9` / `AbstractVarMeta`):
//!
//! | Java | Rust |
//! |---|---|
//! | `extends AbstractCalendarFactory` | field `base: AbstractCalendarFactory` (composition) |
//! | the inherited `processCalendarData` | [`MPP9CalendarFactory::process_calendar_data`] forwards to `base`, passing `self` in as `factory` |
//! | the 7 overridden accessors + `getCalendarVarMeta` | the corresponding implementations in [`AbstractCalendarFactoryMethods`] |
//! | the overridden `processCalendarExceptions` | the [`AbstractCalendarFactoryMethods::process_calendar_exceptions`] implementation |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `ProjectFile file` | `Rc<RefCell<ProjectFile>>` | shared by the crate-wide convention |
//! | `DocumentInputStream` | `&[u8]` (`&document.get_data()`) | constructors uniformly accept a byte slice instead |
//! | `Integer getCalendarNameVarDataType()` | `i32 get_calendar_name_var_data_type()` | Java's `Integer` is never null |
//! | `ByteArrayHelper.getShort` | `get_short_safe` | returns 0 when out of bounds (does not panic) |
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: not used by this class.
//!
//! Skipped Java methods/members: none.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::local_time_range::LocalTimeRange;
use crate::mpp::abstract_calendar_factory::{
    get_short_safe, plus_milliseconds, AbstractCalendarFactory, AbstractCalendarFactoryMethods,
};
use crate::mpp::calendar_factory::CalendarFactory;
use crate::mpp::document_input_stream_factory::DocumentInputStreamFactory;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::ole::DirectoryEntry;
use crate::mpp::props::Props;
use crate::mpp::var_meta::VarMeta;
use crate::mpp::var_meta9::VarMeta9;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_calendar::ProjectCalendar;
use crate::project_file::ProjectFile;

/// VarData type storing the calendar name.
///
/// Corresponds to Java: `private static final Integer CALENDAR_NAME = Integer.valueOf(1);`
const CALENDAR_NAME: i32 = 1;

/// VarData type storing the calendar data.
///
/// Corresponds to Java: `private static final Integer CALENDAR_DATA = Integer.valueOf(3);`
const CALENDAR_DATA: i32 = 3;

/// Fixed length of the exception date block.
///
/// Corresponds to Java: `offset = 4 + (60 * 7) + (index * 64);`
const EXCEPTION_BLOCK_SIZE: usize = 64;

/// Calendar factory specific to MPP9.
///
/// Corresponds to Java: `class MPP9CalendarFactory extends AbstractCalendarFactory`
pub struct MPP9CalendarFactory {
/// The base class part (in Java this class itself is `AbstractCalendarFactory`).
    base: AbstractCalendarFactory,
}

impl MPP9CalendarFactory {
/// Constructor.
    ///
    /// Corresponds to Java: `public MPP9CalendarFactory(ProjectFile file)`
    pub fn new(file: Rc<RefCell<ProjectFile>>) -> Self {
        Self {
            base: AbstractCalendarFactory::new(file),
        }
    }

/// Get the composed base class part.
    ///
/// Rust-specific: Java uses the base class members directly through inheritance, while the Rust side uses composition and therefore needs an explicit accessor.
    pub fn get_base(&self) -> &AbstractCalendarFactory {
        &self.base
    }

/// Calendar data block layout: 4-byte block header + 7 × 60 bytes (seven days of the week) + the exception date block.
    ///
/// Corresponds to Java: `AbstractCalendarFactory#processCalendarData(...)` (not overridden by this class, inherited directly)
    pub fn process_calendar_data(
        &self,
        project_dir: &DirectoryEntry,
        project_props: &Props,
        input_stream_factory: &DocumentInputStreamFactory,
        resource_map: &mut HashMap<i32, Rc<RefCell<ProjectCalendar>>>,
    ) -> MpxjResult<()> {
        self.base.process_calendar_data(
            self,
            project_dir,
            project_props,
            input_stream_factory,
            resource_map,
        )
    }
}

impl CalendarFactory for MPP9CalendarFactory {
/// Corresponds to Java: `CalendarFactory#processCalendarData(...)` (this class implements that interface via inheritance)
    ///
/// Forwards to the method with the same name as in Java to avoid duplicating logic.
    fn process_calendar_data(
        &self,
        project_dir: &DirectoryEntry,
        project_props: &Props,
        input_stream_factory: &DocumentInputStreamFactory,
        resource_map: &mut HashMap<i32, Rc<RefCell<ProjectCalendar>>>,
    ) -> MpxjResult<()> {
        MPP9CalendarFactory::process_calendar_data(
            self,
            project_dir,
            project_props,
            input_stream_factory,
            resource_map,
        )
    }
}

impl AbstractCalendarFactoryMethods for MPP9CalendarFactory {
    /// Corresponds to Java: `@Override protected int getCalendarIDOffset()`
    fn get_calendar_id_offset(&self) -> usize {
        0
    }

    /// Corresponds to Java: `@Override protected int getBaseIDOffset()`
    fn get_base_id_offset(&self) -> usize {
        4
    }

    /// Corresponds to Java: `@Override protected int getResourceIDOffset()`
    fn get_resource_id_offset(&self) -> usize {
        8
    }

    /// Corresponds to Java: `@Override protected int getCalendarHoursOffset()`
    fn get_calendar_hours_offset(&self) -> usize {
        4
    }

    /// Corresponds to Java: `@Override protected Integer getCalendarNameVarDataType()`
    fn get_calendar_name_var_data_type(&self) -> i32 {
        CALENDAR_NAME
    }

    /// Corresponds to Java: `@Override protected Integer getCalendarDataVarDataType()`
    fn get_calendar_data_var_data_type(&self) -> i32 {
        CALENDAR_DATA
    }

/// Read `VarMeta9`.
    ///
    /// Corresponds to Java: `@Override protected VarMeta getCalendarVarMeta(DirectoryEntry directory) throws IOException`
    /// → `return new VarMeta9(new DocumentInputStream(((DocumentEntry) directory.getEntry("VarMeta"))));`
    fn get_calendar_var_meta(&self, directory: &DirectoryEntry) -> MpxjResult<Rc<dyn VarMeta>> {
// Java throws (POI) or NullPointerException when an entry is missing; the Rust side returns Err.
        let Some(document) = directory.get_document("VarMeta") else {
            return Err(MpxjError::new("Missing VarMeta"));
        };

// By convention: Java constructors that accept an InputStream are changed to accept &[u8]
        Ok(Rc::new(VarMeta9::new(&document.get_data())?))
    }

/// Parse exception dates (the MPP9 exception block is a fixed 64 bytes with no recursion rule).
    ///
    /// Corresponds to Java: `@Override protected void processCalendarExceptions(byte[] data, ProjectCalendar cal)`
    fn process_calendar_exceptions(&self, data: &[u8], cal: &Rc<RefCell<ProjectCalendar>>) {
        //
        // Handle any exceptions
        //
        // Java: int exceptionCount = ByteArrayHelper.getShort(data, 0);
        let exception_count = get_short_safe(data, 0);

        if exception_count == 0 {
            return;
        }

        let Ok(mut cal_ref) = cal.try_borrow_mut() else {
            return;
        };

        for index in 0..exception_count {
            // Java: offset = 4 + (60 * 7) + (index * 64);
            let offset = 4 + (60 * 7) + (index as usize * EXCEPTION_BLOCK_SIZE);

            // Java: LocalDate fromDate = MPPUtility.getDate(data, offset);
            let from_date = MPPUtility::get_date(data, offset);
            // Java: LocalDate toDate = MPPUtility.getDate(data, offset + 2);
            let to_date = MPPUtility::get_date(data, offset + 2);
            // Java: exception = cal.addCalendarException(fromDate, toDate);
            let exception = cal_ref.add_calendar_exception(from_date, to_date);

            // Java: periodCount = ByteArrayHelper.getShort(data, offset + 6);
            let period_count = get_short_safe(data, offset + 6);
            if period_count != 0 {
                let mut exception_period_index: i32 = 0;
                while exception_period_index < period_count {
                    // Java: LocalTime start = MPPUtility.getTime(data, offset + 12 + (exceptionPeriodIndex * 2));
                    let start = MPPUtility::get_time(
                        data,
                        offset + 12 + (exception_period_index as usize * 2),
                    );
                    // Java: duration = MPPUtility.getDuration(data, offset + 24 + (exceptionPeriodIndex * 4));
                    let duration = MPPUtility::get_duration(
                        data,
                        offset + 24 + (exception_period_index as usize * 4),
                    );
                    // Java: LocalTime end = start.plus(duration, ChronoUnit.MILLIS);
                    let end = plus_milliseconds(start, duration);
                    // Java: exception.add(new LocalTimeRange(start, end));
                    exception.add(LocalTimeRange::new(Some(start), Some(end)));
                    exception_period_index += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveTime};

/// 1983-12-31 (the epoch for MPP dates).
    fn date_days(year: i32, month: u32, day: u32) -> u16 {
        let epoch = NaiveDate::from_ymd_opt(1983, 12, 31).expect("valid date");
        let date = NaiveDate::from_ymd_opt(year, month, day).expect("valid date");
        (date - epoch).num_days() as u16
    }

    fn tenths_of_minute(hour: u32, minute: u32) -> u16 {
        ((hour * 60 + minute) * 10) as u16
    }

    fn time(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).expect("valid time")
    }

    #[test]
    fn exception_offsets_follow_the_mpp9_layout() {
// 4-byte block header + 7 × 60 bytes + one 64-byte exception
        let mut data = vec![0u8; 4 + (60 * 7) + EXCEPTION_BLOCK_SIZE];
        data[0..2].copy_from_slice(&1u16.to_le_bytes());

        let offset = 4 + (60 * 7);
        data[offset..offset + 2].copy_from_slice(&date_days(2024, 2, 5).to_le_bytes());
        data[offset + 2..offset + 4].copy_from_slice(&date_days(2024, 2, 6).to_le_bytes());
        data[offset + 6..offset + 8].copy_from_slice(&1u16.to_le_bytes());
        data[offset + 12..offset + 14].copy_from_slice(&tenths_of_minute(10, 30).to_le_bytes());
        data[offset + 24..offset + 28].copy_from_slice(&900u32.to_le_bytes());

        let cal = Rc::new(RefCell::new(ProjectCalendar::new(
            std::rc::Weak::new(),
            false,
        )));
        let factory = MPP9CalendarFactory::new(ProjectFile::new());
        factory.process_calendar_exceptions(&data, &cal);

        let mut cal_ref = cal.borrow_mut();
        let exceptions = cal_ref.get_calendar_exceptions();
        assert_eq!(exceptions.len(), 1);
        assert_eq!(
            exceptions[0].get_from_date(),
            NaiveDate::from_ymd_opt(2024, 2, 5)
        );
        assert_eq!(
            exceptions[0].get_to_date(),
            NaiveDate::from_ymd_opt(2024, 2, 6)
        );

        let hours = exceptions[0].get_hours();
// 900 tenths of a minute = 90 minutes → 10:30 + 1:30 = 12:00
        assert_eq!(hours.size(), 1);
        assert_eq!(
            hours.get(0),
            LocalTimeRange::new(Some(time(10, 30)), Some(time(12, 0)))
        );
    }

    #[test]
    fn missing_or_truncated_data_does_not_panic() {
        let factory = MPP9CalendarFactory::new(ProjectFile::new());

// Empty block / insufficient data: Java throws at getShort / getDate, while the Rust side returns 0 / None
        for data in [vec![], vec![0u8; 8], vec![1u8, 0u8]] {
            let cal = Rc::new(RefCell::new(ProjectCalendar::new(
                std::rc::Weak::new(),
                false,
            )));
            factory.process_calendar_exceptions(&data, &cal);

            if data.len() == 2 {
// exceptionCount = 1, but all the data after it is out of bounds → generates one exception with an NA date
                let mut cal_ref = cal.borrow_mut();
                let exceptions = cal_ref.get_calendar_exceptions();
                assert_eq!(exceptions.len(), 1);
                assert_eq!(exceptions[0].get_from_date(), None);
                assert_eq!(exceptions[0].get_to_date(), None);
                assert_eq!(exceptions[0].get_hours().size(), 0);
            } else {
                assert_eq!(cal.borrow_mut().get_calendar_exceptions().len(), 0);
            }
        }
    }

    #[test]
    fn offsets_match_the_mpp9_layout() {
        let factory = MPP9CalendarFactory::new(ProjectFile::new());
        assert_eq!(factory.get_calendar_id_offset(), 0);
        assert_eq!(factory.get_base_id_offset(), 4);
        assert_eq!(factory.get_resource_id_offset(), 8);
        assert_eq!(factory.get_calendar_hours_offset(), 4);
        assert_eq!(factory.get_calendar_name_var_data_type(), CALENDAR_NAME);
        assert_eq!(factory.get_calendar_data_var_data_type(), CALENDAR_DATA);
    }
}
