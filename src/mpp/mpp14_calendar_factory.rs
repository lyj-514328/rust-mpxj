//! Corresponds to Java: `org/mpxj/mpp/MPP14CalendarFactory.java`
//!
//! Calendar reading specific to MPP14 (Microsoft Project 2010 and later): the only difference from MPP12 is
//! that in the fixed data block the arrangement order of the three fields "calendar ID / base calendar ID / resource ID" varies with
//! `ProjectProperties.applicationVersion`.
//!
//! # How the inheritance relationship is restored
//!
//! Java: `class MPP14CalendarFactory extends AbstractCalendarAndExceptionFactory`. Rust has no inheritance,
//! so it is restored with "composition + forwarding" (the same approach as `VarMeta12` / `AbstractVarMeta`):
//!
//! | Java | Rust |
//! |---|---|
//! | `extends AbstractCalendarAndExceptionFactory` | Field `base: AbstractCalendarAndExceptionFactory` (composition) |
//! | The three offsets determined in the constructor by the application version (Java's `m_calendarIDOffset` and similar fields) | This struct's `calendar_id_offset` / `base_id_offset` / `resource_id_offset` fields |
//! | The inherited `processCalendarData` | [`MPP14CalendarFactory::process_calendar_data`] forwards to `base` |
//! | The inherited `processCalendarExceptions` | [`MPP14CalendarFactory::process_calendar_exceptions`] forwards to `base.process_calendar_exceptions` |
//! | The 6 overridden accessors + `getCalendarVarMeta` | The corresponding implementations in [`AbstractCalendarFactoryMethods`] |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `ProjectFile file` | `Rc<RefCell<ProjectFile>>` | Shared per the package-wide convention; `try_borrow` is used to read the application version |
//! | `DocumentInputStream` | `&[u8]` (`&document.get_data()`) | Constructors uniformly take a byte slice instead |
//! | `NumberHelper.getInt(file.getProjectProperties().getApplicationVersion())` | `NumberHelper::get_int(version.map(f64::from))` | The Rust version of `NumberHelper` takes an `Option<f64>` value |
//! | `ApplicationVersion.PROJECT_2010` | [`crate::mpp::application_version::ApplicationVersion::PROJECT_2010`] | Same-named constant |
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: not used by this class.
//!
//! Skipped Java members: none.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::common::number_helper::NumberHelper;
use crate::mpp::abstract_calendar_and_exception_factory::AbstractCalendarAndExceptionFactory;
use crate::mpp::abstract_calendar_factory::AbstractCalendarFactoryMethods;
use crate::mpp::application_version::ApplicationVersion;
use crate::mpp::calendar_factory::CalendarFactory;
use crate::mpp::document_input_stream_factory::DocumentInputStreamFactory;
use crate::mpp::ole::DirectoryEntry;
use crate::mpp::props::Props;
use crate::mpp::var_meta::VarMeta;
use crate::mpp::var_meta12::VarMeta12;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_calendar::ProjectCalendar;
use crate::project_file::ProjectFile;

/// VarData type holding calendar names.
///
/// Corresponds to Java: `private static final Integer CALENDAR_NAME = Integer.valueOf(1);`
const CALENDAR_NAME: i32 = 1;

/// VarData type holding calendar data.
///
/// Corresponds to Java: `private static final Integer CALENDAR_DATA = Integer.valueOf(8);`
const CALENDAR_DATA: i32 = 8;

/// The version number of Microsoft Project 2010.
///
/// Corresponds to Java: `ApplicationVersion.PROJECT_2010`
const PROJECT_2010: i32 = ApplicationVersion::PROJECT_2010;

/// Calendar factory specific to MPP14.
///
/// Corresponds to Java: `class MPP14CalendarFactory extends AbstractCalendarAndExceptionFactory`
pub struct MPP14CalendarFactory {
/// The base class part (in Java this class itself is `AbstractCalendarAndExceptionFactory`).
    base: AbstractCalendarAndExceptionFactory,

    /// Corresponds to Java field: `private final int m_calendarIDOffset;`
    calendar_id_offset: usize,
    /// Corresponds to Java field: `private final int m_baseIDOffset;`
    base_id_offset: usize,
    /// Corresponds to Java field: `private final int m_resourceIDOffset;`
    resource_id_offset: usize,
}

impl MPP14CalendarFactory {
/// Constructor. Determines the arrangement order of the three ID fields by the application version.
    ///
    /// Corresponds to Java: `public MPP14CalendarFactory(ProjectFile file)`
    pub fn new(file: Rc<RefCell<ProjectFile>>) -> Self {
        // Java: if (NumberHelper.getInt(file.getProjectProperties().getApplicationVersion()) > ApplicationVersion.PROJECT_2010)
        let application_version = match file.try_borrow() {
            Ok(file) => NumberHelper::get_int(
                file.get_project_properties()
                    .get_application_version()
                    .map(f64::from),
            ),
// This branch never occurs in Java (the file is necessarily readable); the Rust side avoids a borrow conflict
            Err(_) => 0,
        };

        let base = AbstractCalendarAndExceptionFactory::new(file);

        if application_version > PROJECT_2010 {
            Self {
                base,
                calendar_id_offset: 8,
                base_id_offset: 0,
                resource_id_offset: 4,
            }
        } else {
            Self {
                base,
                calendar_id_offset: 0,
                base_id_offset: 4,
                resource_id_offset: 8,
            }
        }
    }

/// Returns the composed base class part.
    ///
/// Rust-specific: Java uses base class members directly through inheritance, whereas the Rust side uses composition and so must access them explicitly.
    pub fn get_base(&self) -> &AbstractCalendarAndExceptionFactory {
        &self.base
    }

/// Structure of the calendar data block: 4-byte block header + 7 × 60 bytes (the seven days of the week) + exception date block.
    ///
/// Corresponds to Java: `AbstractCalendarFactory#processCalendarData(...)` (not overridden by this class, inherited directly)
    pub fn process_calendar_data(
        &self,
        project_dir: &DirectoryEntry,
        project_props: &Props,
        input_stream_factory: &DocumentInputStreamFactory,
        resource_map: &mut HashMap<i32, Rc<RefCell<ProjectCalendar>>>,
    ) -> MpxjResult<()> {
        self.base
            .process_calendar_data(self, project_dir, project_props, input_stream_factory, resource_map)
    }

/// Parses the exception dates in the calendar data block.
    ///
    /// Corresponds to Java: `AbstractCalendarAndExceptionFactory#processCalendarExceptions(...)`
/// (Not overridden by this class, inherited directly; the Rust side forwards explicitly because it has no inheritance)
    pub fn process_calendar_exceptions(&self, data: &[u8], cal: &Rc<RefCell<ProjectCalendar>>) {
        self.base.process_calendar_exceptions(data, cal);
    }
}

impl CalendarFactory for MPP14CalendarFactory {
/// Corresponds to Java: `CalendarFactory#processCalendarData(...)` (this class implements that interface via inheritance)
    ///
/// Forwards to the method with the same name as in Java, avoiding duplicated logic.
    fn process_calendar_data(
        &self,
        project_dir: &DirectoryEntry,
        project_props: &Props,
        input_stream_factory: &DocumentInputStreamFactory,
        resource_map: &mut HashMap<i32, Rc<RefCell<ProjectCalendar>>>,
    ) -> MpxjResult<()> {
        MPP14CalendarFactory::process_calendar_data(
            self,
            project_dir,
            project_props,
            input_stream_factory,
            resource_map,
        )
    }
}

impl AbstractCalendarFactoryMethods for MPP14CalendarFactory {
    /// Corresponds to Java: `@Override protected int getCalendarIDOffset()`
    fn get_calendar_id_offset(&self) -> usize {
        self.calendar_id_offset
    }

    /// Corresponds to Java: `@Override protected int getBaseIDOffset()`
    fn get_base_id_offset(&self) -> usize {
        self.base_id_offset
    }

    /// Corresponds to Java: `@Override protected int getResourceIDOffset()`
    fn get_resource_id_offset(&self) -> usize {
        self.resource_id_offset
    }

    /// Corresponds to Java: `@Override protected int getCalendarHoursOffset()`
    fn get_calendar_hours_offset(&self) -> usize {
        0
    }

    /// Corresponds to Java: `@Override protected Integer getCalendarNameVarDataType()`
    fn get_calendar_name_var_data_type(&self) -> i32 {
        CALENDAR_NAME
    }

    /// Corresponds to Java: `@Override protected Integer getCalendarDataVarDataType()`
    fn get_calendar_data_var_data_type(&self) -> i32 {
        CALENDAR_DATA
    }

/// Reads `VarMeta12`.
    ///
    /// Corresponds to Java: `@Override protected VarMeta getCalendarVarMeta(DirectoryEntry directory) throws IOException`
    /// → `return new VarMeta12(new DocumentInputStream(((DocumentEntry) directory.getEntry("VarMeta"))));`
    fn get_calendar_var_meta(&self, directory: &DirectoryEntry) -> MpxjResult<Rc<dyn VarMeta>> {
// Java throws an exception (POI) or NullPointerException when an entry is missing; the Rust side returns Err.
        let Some(document) = directory.get_document("VarMeta") else {
            return Err(MpxjError::new("Missing VarMeta"));
        };

// Per convention: Java constructors that accept an InputStream are changed to accept &[u8]
        Ok(Rc::new(VarMeta12::new(&document.get_data())?))
    }

/// Forwards to the base class implementation (Java obtains that implementation through inheritance).
    ///
    /// Corresponds to Java: `AbstractCalendarAndExceptionFactory#processCalendarExceptions(byte[] data, ProjectCalendar cal)`
    fn process_calendar_exceptions(&self, data: &[u8], cal: &Rc<RefCell<ProjectCalendar>>) {
        self.base.process_calendar_exceptions(data, cal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    use chrono::{NaiveDate, NaiveTime, Weekday};
    use cfb::CompoundFile;

    use crate::local_date_range::LocalDateRange;
    use crate::local_time_range::LocalTimeRange;
    use crate::mpp::ole::PoifsFileSystem;

/// Number of days since 1983-12-31 (the MPP date epoch).
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

/// UTF-16LE encoding (including the terminating NUL, length aligned to 4 bytes).
    fn utf16le(value: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in value.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&0u16.to_le_bytes());
        while data.len() % 4 != 0 {
            data.push(0);
        }
        data
    }

/// A 60-byte working day block.
    fn day_block(default_flag: i32, periods: &[(u16, u32)]) -> [u8; 60] {
        let mut block = [0u8; 60];
        block[0..2].copy_from_slice(&(default_flag as u16).to_le_bytes());
        block[2..4].copy_from_slice(&(periods.len() as u16).to_le_bytes());

        for (index, (start, duration)) in periods.iter().enumerate() {
            let start_offset = 8 + index * 2;
            block[start_offset..start_offset + 2].copy_from_slice(&start.to_le_bytes());
            let duration_offset = 20 + index * 4;
            block[duration_offset..duration_offset + 4].copy_from_slice(&duration.to_le_bytes());
        }

        block
    }

/// MPP14 calendar data block: 7 × 60-byte working day blocks (from offset 0) + exception block + working week block.
    fn calendar_data_block() -> Vec<u8> {
        let mut data = Vec::new();

// Order: Sunday, Monday ... Saturday (DayOfWeekHelper.getInstance(index + 1))
        data.extend_from_slice(&day_block(0, &[])); // Sunday: non-working day
        data.extend_from_slice(&day_block(
            0,
            &[
                (tenths_of_minute(8, 0), 2400),
                (tenths_of_minute(13, 0), 2400),
            ],
        )); // Monday: 08:00-12:00 / 13:00-17:00
        data.extend_from_slice(&day_block(1, &[])); // Tuesday: default values
        data.extend_from_slice(&day_block(1, &[])); // Wednesday: default values
        data.extend_from_slice(&day_block(1, &[])); // Thursday: default values
        data.extend_from_slice(&day_block(0, &[])); // Friday: non-working day
        data.extend_from_slice(&day_block(0, &[])); // Saturday: non-working day

        assert_eq!(data.len(), 420);

// Exception block: 1 exception, 2024-03-01, 09:00-11:00
// Java: exceptionCount is at offset 420, and exception records start at offset 424
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&[0u8; 92]);
        {
            let exception = &mut data[424..424 + 92];
            exception[0..2].copy_from_slice(&date_days(2024, 3, 1).to_le_bytes());
            exception[2..4].copy_from_slice(&date_days(2024, 3, 1).to_le_bytes());
            exception[4..6].copy_from_slice(&1u16.to_le_bytes());
            exception[14..16].copy_from_slice(&1u16.to_le_bytes());
            exception[20..22].copy_from_slice(&tenths_of_minute(9, 0).to_le_bytes());
            exception[32..36].copy_from_slice(&1200u32.to_le_bytes());
// Recurrence type = 1 (DAILY) + frequency 1 → flattened into a plain exception
            exception[72..74].copy_from_slice(&1u16.to_le_bytes());
// Name length = 0
            exception[88..92].copy_from_slice(&0u32.to_le_bytes());
        }
        assert_eq!(data.len(), 516);

// The 4-byte block header of the working week
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 520);

// Working week block: Monday 08:00-12:00, the rest are default values
        let week_start = data.len();
        for day_index in 0..7 {
            let offset = week_start + day_index * 60;
            if day_index == 1 {
                data.extend_from_slice(&day_block(0, &[(tenths_of_minute(8, 0), 2400)]));
            } else {
                data.extend_from_slice(&day_block(1, &[]));
            }
            assert_eq!(data.len(), offset + 60);
        }
        data.extend_from_slice(&date_days(2024, 4, 1).to_le_bytes());
        data.extend_from_slice(&date_days(2024, 4, 7).to_le_bytes());
        data.extend_from_slice(&[0u8; 8]); // unknown 8 bytes
        data.extend_from_slice(&0u32.to_le_bytes()); // name length

        data
    }

/// Var2Data data block: a 4-byte length prefix per entry; returns (data block, offsets of the entries).
    fn var2_data_block(items: &[Vec<u8>]) -> (Vec<u8>, Vec<i32>) {
        let mut data = Vec::new();
        let mut offsets = Vec::new();

        for item in items {
            offsets.push(data.len() as i32);
            data.extend_from_slice(&(item.len() as i32).to_le_bytes());
            data.extend_from_slice(item);
        }

        (data, offsets)
    }

/// VarMeta12 data block: 24-byte block header + 12 bytes per entry.
    fn var_meta12_bytes(entries: &[(i32, i32, i32)], data_size: i32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes()); // unknown 1
        data.extend_from_slice(&(entries.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes()); // unknown 2
        data.extend_from_slice(&0i32.to_le_bytes()); // unknown 3
        data.extend_from_slice(&data_size.to_le_bytes());

        for (unique_id, type_, offset) in entries {
            data.extend_from_slice(&unique_id.to_le_bytes());
            data.extend_from_slice(&offset.to_le_bytes());
            data.extend_from_slice(&(*type_ as u16).to_le_bytes());
            data.extend_from_slice(&0u16.to_le_bytes());
        }

        data
    }

/// FixedMeta data block: 16-byte block header + `item_size` bytes per entry (bytes 5..8 are the data offset).
    fn fixed_meta_bytes(offsets: &[i32], item_size: usize) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(offsets.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());

        for offset in offsets {
            let mut record = vec![0u8; item_size];
            record[4..8].copy_from_slice(&offset.to_le_bytes());
            data.extend_from_slice(&record);
        }

        data
    }

/// A fixed-length data block: 12-byte records (calendar ID / base calendar ID / resource ID).
    fn fixed_data_record(calendar_id: i32, base_id: i32, resource_id: i32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&calendar_id.to_le_bytes());
        data.extend_from_slice(&base_id.to_le_bytes());
        data.extend_from_slice(&resource_id.to_le_bytes());
        data
    }

/// Builds a compound document containing `TBkndCal` and returns the project storage.
    fn sample_project_directory(calendar_data: &[u8]) -> (PoifsFileSystem, DirectoryEntry) {
        let name = utf16le("Test Calendar");
        let (var2_data, offsets) = var2_data_block(&[name, calendar_data.to_vec()]);
        let var_meta = var_meta12_bytes(
            &[
                (1, CALENDAR_NAME, offsets[0]),
                (1, CALENDAR_DATA, offsets[1]),
            ],
            var2_data.len() as i32,
        );
        let fixed_meta = fixed_meta_bytes(&[0], 10);
        let fixed_data = fixed_data_record(1, 0, 1);

        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).expect("create compound document");
        compound.create_storage("/   114").expect("create storage");
        compound
            .create_storage("/   114/TBkndCal")
            .expect("create calendar storage");
        compound
            .create_stream("/   114/TBkndCal/VarMeta")
            .expect("create VarMeta")
            .write_all(&var_meta)
            .expect("write VarMeta");
        compound
            .create_stream("/   114/TBkndCal/Var2Data")
            .expect("create Var2Data")
            .write_all(&var2_data)
            .expect("write Var2Data");
        compound
            .create_stream("/   114/TBkndCal/FixedMeta")
            .expect("create FixedMeta")
            .write_all(&fixed_meta)
            .expect("write FixedMeta");
        compound
            .create_stream("/   114/TBkndCal/FixedData")
            .expect("create FixedData")
            .write_all(&fixed_data)
            .expect("write FixedData");

        let bytes = compound.into_inner().into_inner();
        let file_system = PoifsFileSystem::new(Cursor::new(bytes)).expect("open compound document");
        let project_dir = file_system
            .get_root()
            .get_directory("   114")
            .expect("project storage");

        (file_system, project_dir)
    }

    #[test]
    fn calendar_data_is_parsed_end_to_end() {
        let (_file_system, project_dir) = sample_project_directory(&calendar_data_block());

        let file = ProjectFile::new();
        let factory = MPP14CalendarFactory::new(Rc::clone(&file));
        let props = Props::new();
        let input_stream_factory = DocumentInputStreamFactory::new(&props);
        let mut resource_map: HashMap<i32, Rc<RefCell<ProjectCalendar>>> = HashMap::new();

        factory
            .process_calendar_data(&project_dir, &props, &input_stream_factory, &mut resource_map)
            .expect("calendar parsing should succeed");

// Resource ID 1 has been registered with the calendar
        assert_eq!(resource_map.len(), 1);
        assert!(resource_map.contains_key(&1));

        let calendar = Rc::clone(&resource_map[&1]);
        let calendar_ref = calendar.borrow();

        assert_eq!(calendar_ref.get_name(), Some("Test Calendar"));
        assert_eq!(calendar_ref.get_unique_id(), Some(1));

// Monday: two explicitly defined time periods
        assert_eq!(
            calendar_ref.get_day_type(Weekday::Mon),
            Some(crate::day_type::DayType::Working)
        );
        let monday = calendar_ref.get_hours(Weekday::Mon).expect("Monday work hours");
        assert_eq!(monday.size(), 2);
        assert_eq!(
            monday.get(0),
            LocalTimeRange::new(Some(time(8, 0)), Some(time(12, 0)))
        );
        assert_eq!(
            monday.get(1),
            LocalTimeRange::new(Some(time(13, 0)), Some(time(17, 0)))
        );

// Sunday / Friday: explicit non-working days
        assert_eq!(
            calendar_ref.get_day_type(Weekday::Sun),
            Some(crate::day_type::DayType::NonWorking)
        );
        assert_eq!(
            calendar_ref.get_day_type(Weekday::Fri),
            Some(crate::day_type::DayType::NonWorking)
        );

// Wednesday: takes the default value (the default working day 08:00-12:00 / 13:00-17:00 "when DEFAULT_CALENDAR_HOURS is not provided")
        assert_eq!(
            calendar_ref.get_day_type(Weekday::Wed),
            Some(crate::day_type::DayType::Working)
        );
        let wednesday = calendar_ref.get_hours(Weekday::Wed).expect("Wednesday should inherit default hours");
        assert_eq!(wednesday.size(), 2);

        drop(calendar_ref);

// Exception dates
        let mut calendar_ref = calendar.borrow_mut();
        let exceptions = calendar_ref.get_calendar_exceptions();
        assert_eq!(exceptions.len(), 1);
        assert_eq!(
            exceptions[0].get_from_date(),
            NaiveDate::from_ymd_opt(2024, 3, 1)
        );
        let exception_hours = exceptions[0].get_hours();
        assert_eq!(exception_hours.size(), 1);
        assert_eq!(
            exception_hours.get(0),
            LocalTimeRange::new(Some(time(9, 0)), Some(time(11, 0)))
        );
        drop(calendar_ref);

// Working week
        let calendar_ref = calendar.borrow();
        let weeks = calendar_ref.get_work_weeks();
        assert_eq!(weeks.len(), 1);
        assert_eq!(
            weeks[0].get_date_range(),
            Some(LocalDateRange::new(
                NaiveDate::from_ymd_opt(2024, 4, 1),
                NaiveDate::from_ymd_opt(2024, 4, 7)
            ))
        );
        let week_monday = weeks[0]
            .get_calendar_hours_for_day(Weekday::Mon)
            .expect("Monday of the working week");
        assert_eq!(week_monday.size(), 1);
        assert_eq!(
            week_monday.get(0),
            LocalTimeRange::new(Some(time(8, 0)), Some(time(12, 0)))
        );
        drop(calendar_ref);

// The default calendar has been written to the project properties
        assert_eq!(file.borrow().get_calendars().len(), 1);
        assert!(file
            .borrow()
            .get_project_properties()
            .get_default_calendar()
            .is_some());
    }

    #[test]
    fn missing_calendar_directory_returns_an_error() {
        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).expect("create compound document");
        compound.create_storage("/   114").expect("create storage");
        let bytes = compound.into_inner().into_inner();
        let file_system = PoifsFileSystem::new(Cursor::new(bytes)).expect("open compound document");
        let project_dir = file_system
            .get_root()
            .get_directory("   114")
            .expect("project storage");

        let file = ProjectFile::new();
        let factory = MPP14CalendarFactory::new(Rc::clone(&file));
        let props = Props::new();
        let input_stream_factory = DocumentInputStreamFactory::new(&props);
        let mut resource_map: HashMap<i32, Rc<RefCell<ProjectCalendar>>> = HashMap::new();

// Java throws an exception at projectDir.getEntry("TBkndCal")
        let result =
            factory.process_calendar_data(&project_dir, &props, &input_stream_factory, &mut resource_map);
        assert!(result.is_err());
    }

    #[test]
    fn truncated_calendar_data_does_not_panic() {
// The calendar data block is truncated (no exception block or working week block)
        let (_file_system, project_dir) = sample_project_directory(&[0u8; 10]);

        let file = ProjectFile::new();
        let factory = MPP14CalendarFactory::new(Rc::clone(&file));
        let props = Props::new();
        let input_stream_factory = DocumentInputStreamFactory::new(&props);
        let mut resource_map: HashMap<i32, Rc<RefCell<ProjectCalendar>>> = HashMap::new();

        factory
            .process_calendar_data(&project_dir, &props, &input_stream_factory, &mut resource_map)
            .expect("data truncated should still return normally");

        let calendar = Rc::clone(&resource_map[&1]);
        let mut calendar_ref = calendar.borrow_mut();
        assert_eq!(calendar_ref.get_calendar_exceptions().len(), 0);
        assert_eq!(calendar_ref.get_work_weeks().len(), 0);
// All days are treated as explicit non-working days (time period count is 0)
        assert_eq!(
            calendar_ref.get_day_type(Weekday::Mon),
            Some(crate::day_type::DayType::NonWorking)
        );
    }

    #[test]
    fn offsets_switch_for_project_2013_and_later() {
        let file = ProjectFile::new();
        // Java: ApplicationVersion.PROJECT_2013 == 15 > PROJECT_2010
        file.borrow_mut()
            .get_project_properties_mut()
            .set_application_version(Some(15));

        let factory = MPP14CalendarFactory::new(Rc::clone(&file));
        assert_eq!(factory.get_calendar_id_offset(), 8);
        assert_eq!(factory.get_base_id_offset(), 0);
        assert_eq!(factory.get_resource_id_offset(), 4);
        assert_eq!(factory.get_calendar_hours_offset(), 0);

        let file = ProjectFile::new();
        file.borrow_mut()
            .get_project_properties_mut()
            .set_application_version(Some(14));
        let factory = MPP14CalendarFactory::new(Rc::clone(&file));
        assert_eq!(factory.get_calendar_id_offset(), 0);
        assert_eq!(factory.get_base_id_offset(), 4);
        assert_eq!(factory.get_resource_id_offset(), 8);
    }
}
