//! Corresponds to Java: `org/mpxj/mpp/AbstractCalendarFactory.java`
//!
//! Parsing logic shared by the calendar reading of the three versions MPP9 / MPP12 / MPP14: based on the
//! `VarMeta` / `Var2Data` / `FixedMeta` / `FixedData` stored in `TBkndCal` it assembles the working hours of each calendar.
//!
//! # Restoring the inheritance relationships
//!
//! Java: `abstract class AbstractCalendarFactory implements CalendarFactory`,
//! `MPP9CalendarFactory extends AbstractCalendarFactory`,
//! `MPP12/MPP14CalendarFactory extends AbstractCalendarAndExceptionFactory extends AbstractCalendarFactory`.
//! Rust has no inheritance, so it is restored as follows (composition + forwarding, consistent with the approach used for `VarMeta`/`AbstractVarMeta`):
//!
//! | Java | Rust |
//! |---|---|
//! | `interface CalendarFactory` | [`crate::mpp::calendar_factory::CalendarFactory`] trait |
//! | `abstract class AbstractCalendarFactory` | the [`AbstractCalendarFactory`] struct in this file (holding `file`) |
//! | the base class's 8 `protected abstract` methods | the [`AbstractCalendarFactoryMethods`] trait (implemented by subclasses) |
//! | the base class implements `processCalendarData` (virtually calling the abstract methods) | [`AbstractCalendarFactory::process_calendar_data`] gains a `factory: &dyn AbstractCalendarFactoryMethods` parameter, and the base class calls back into the subclass implementation |
//! | `MPP9CalendarFactory extends AbstractCalendarFactory` | [`crate::mpp::mpp9_calendar_factory::MPP9CalendarFactory`] **composes** an `AbstractCalendarFactory` and forwards `process_calendar_data` to it (also passing itself as `factory`) |
//!
//! [`AbstractCalendarFactoryMethods`] is needed because Java's `processCalendarData` calls
//! abstract methods such as `getCalendarIDOffset()` that are overridden by subclasses; Rust has no virtual methods, so the subclass can only be passed explicitly
//! as a trait object (equivalent to Java's dynamic dispatch). The abstract method names are kept 1:1 with Java.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `ProjectFile m_file` | `Rc<RefCell<ProjectFile>>` | shared by convention; access uses `try_borrow` / `try_borrow_mut` to avoid a borrow conflict panic |
//! | `ProjectCalendar` | `Rc<RefCell<ProjectCalendar>>` | the calendar must be shared between the file container and local variables |
//! | `byte[]` | `&[u8]` / `Option<&[u8]>` | Java's `null` → `None` |
//! | `Pair<ProjectCalendar, Integer>` | `(Rc<RefCell<ProjectCalendar>>, i32)` | the `Pair` from [`crate::common::pair`] is not used |
//! | `ByteArrayHelper.getShort/getInt` | `get_short_safe` / `get_int_safe` | Java throws on out of bounds, while Rust must not panic (see the function docs) |
//! | `LocalTime.plus(duration, MILLIS)` | [`plus_milliseconds`] | chrono has no equivalent "wrap by milliseconds" method |
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: not used by this class.
//!
//! Skipped Java methods/members: none (`processCalendarHours` is `private`, and the Rust side likewise does not expose it outside the module).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use chrono::{NaiveTime, Timelike, Weekday};

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::day_of_week_helper::DayOfWeekHelper;
use crate::day_type::DayType;
use crate::local_time_range::LocalTimeRange;
use crate::mpp::document_input_stream_factory::DocumentInputStreamFactory;
use crate::mpp::fixed_data::FixedData;
use crate::mpp::fixed_meta::FixedMeta;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::ole::DirectoryEntry;
use crate::mpp::props::Props;
use crate::mpp::props_key::PropsKey;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_meta::VarMeta;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_calendar::ProjectCalendar;
use crate::project_calendar_days::ProjectCalendarDays;
use crate::project_file::ProjectFile;

/// Default working week (the index corresponds to `DayOfWeekHelper::get_instance(index + 1)`: 0 = Sunday).
///
/// Corresponds to Java: `private static final boolean[] DEFAULT_WORKING_WEEK`
const DEFAULT_WORKING_WEEK: [bool; 7] = [false, true, true, true, true, true, false];

/// The order of Java's `DayOfWeek.values()` (Monday → Sunday).
///
/// Note that this differs from [`DayOfWeekHelper::ORDERED_DAYS`] (Sunday → Saturday); Java's
/// `Stream.of(DayOfWeek.values())` uses the enum declaration order.
const JAVA_DAY_OF_WEEK_VALUES: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];

/// Rust representation of the abstract methods in `AbstractCalendarFactory` overridden by subclasses.
///
/// Corresponds to Java: the 8 `protected abstract` methods of `AbstractCalendarFactory`.
/// Java's base class virtually calls them in `processCalendarData`; the Rust side instead has the caller pass the concrete factory
/// in as `&dyn` (see "Restoring the inheritance relationships" in the file header).
pub trait AbstractCalendarFactoryMethods {
/// Offset of the calendar unique ID within the fixed-length data block.
    ///
    /// Corresponds to Java: `protected abstract int getCalendarIDOffset()`
    fn get_calendar_id_offset(&self) -> usize;

/// Offset of the base calendar unique ID within the fixed-length data block.
    ///
    /// Corresponds to Java: `protected abstract int getBaseIDOffset()`
    fn get_base_id_offset(&self) -> usize;

/// Offset of the resource ID within the fixed-length data block.
    ///
    /// Corresponds to Java: `protected abstract int getResourceIDOffset()`
    fn get_resource_id_offset(&self) -> usize;

/// Read the `VarMeta` stream under `TBkndCal`.
    ///
    /// Corresponds to Java: `protected abstract VarMeta getCalendarVarMeta(DirectoryEntry directory) throws IOException`
    fn get_calendar_var_meta(&self, directory: &DirectoryEntry) -> MpxjResult<Rc<dyn VarMeta>>;

/// Start offset of each weekday's working-hours block within the calendar data block.
    ///
    /// Corresponds to Java: `protected abstract int getCalendarHoursOffset()`
    fn get_calendar_hours_offset(&self) -> usize;

/// VarData type storing the calendar name.
    ///
    /// Corresponds to Java: `protected abstract Integer getCalendarNameVarDataType()`
    fn get_calendar_name_var_data_type(&self) -> i32;

/// VarData type storing the calendar data.
    ///
    /// Corresponds to Java: `protected abstract Integer getCalendarDataVarDataType()`
    fn get_calendar_data_var_data_type(&self) -> i32;

/// Parse the exception dates in the calendar data block.
    ///
    /// Corresponds to Java: `protected abstract void processCalendarExceptions(byte[] data, ProjectCalendar cal)`
    fn process_calendar_exceptions(&self, data: &[u8], cal: &Rc<RefCell<ProjectCalendar>>);
}

/// Common implementation for calendar reading.
///
/// Corresponds to Java: `abstract class AbstractCalendarFactory implements CalendarFactory`
pub struct AbstractCalendarFactory {
    /// Corresponds to Java field: `private final ProjectFile m_file;`
    file: Rc<RefCell<ProjectFile>>,
}

impl AbstractCalendarFactory {
/// Constructor.
    ///
    /// Corresponds to Java: `public AbstractCalendarFactory(ProjectFile file)`
    ///
/// The Java side holds a strong `ProjectFile` reference; the Rust side uses `Rc<RefCell<ProjectFile>>` sharing by convention.
    pub fn new(file: Rc<RefCell<ProjectFile>>) -> Self {
        Self { file }
    }

/// Get the owning project file.
    ///
/// Rust-specific: Java's subclasses access the protected field `m_file` directly, while the Rust side provides an accessor.
    pub fn get_file(&self) -> &Rc<RefCell<ProjectFile>> {
        &self.file
    }

/// Calendar data block layout: 4-byte block header + 7 × 60 bytes (seven days of the week) + (from MPP12) the exception date block.
    ///
    /// Corresponds to Java: `public void processCalendarData(DirectoryEntry projectDir, Props projectProps,
    /// DocumentInputStreamFactory inputStreamFactory, HashMap<Integer, ProjectCalendar> resourceMap)`
    ///
/// `factory` is the "subclass" — on the Java side the abstract method implementations come from inheritance and virtual methods, while the Rust side passes them in explicitly.
    pub fn process_calendar_data(
        &self,
        factory: &dyn AbstractCalendarFactoryMethods,
        project_dir: &DirectoryEntry,
        project_props: &Props,
        input_stream_factory: &DocumentInputStreamFactory,
        resource_map: &mut HashMap<i32, Rc<RefCell<ProjectCalendar>>>,
    ) -> MpxjResult<()> {
        // Java: DirectoryEntry calDir = (DirectoryEntry) projectDir.getEntry("TBkndCal");
// POI throws when it cannot find an entry and throws ClassCastException when the cast fails; the Rust side returns Err.
        let Some(cal_dir) = project_dir.get_directory("TBkndCal") else {
            return Err(MpxjError::new("Missing TBkndCal directory"));
        };

        // Java: VarMeta calVarMeta = getCalendarVarMeta(calDir);
        let cal_var_meta = factory.get_calendar_var_meta(&cal_dir)?;

        // Java: Var2Data calVarData = new Var2Data(m_file, calVarMeta, new DocumentInputStream((DocumentEntry) calDir.getEntry("Var2Data")));
        let Some(var2_data_bytes) = cal_dir.get_document("Var2Data").map(|entry| entry.get_data())
        else {
            return Err(MpxjError::new("Missing TBkndCal/Var2Data"));
        };
        let cal_var_data = Var2Data::new(&self.file, cal_var_meta, &var2_data_bytes);

        // Java: FixedMeta calFixedMeta = new FixedMeta(new DocumentInputStream(((DocumentEntry) calDir.getEntry("FixedMeta"))), 10);
        let Some(fixed_meta_bytes) = cal_dir.get_document("FixedMeta").map(|entry| entry.get_data())
        else {
            return Err(MpxjError::new("Missing TBkndCal/FixedMeta"));
        };
        let cal_fixed_meta = FixedMeta::with_item_size(&fixed_meta_bytes, 10)?;

        // Java: FixedData calFixedData = new FixedData(calFixedMeta, inputStreamFactory.getInstance(calDir, "FixedData"), 12);
        let cal_fixed_data = FixedData::with_max_expected_size(
            &cal_fixed_meta,
            &input_stream_factory
                .get_instance(&cal_dir, "FixedData")?
                .to_bytes(),
            12,
        );

        // Java: FixedData calFixed2Data = null;
        //       if (calDir.hasEntry("Fixed2Meta")) { ... }
        let cal_fixed2_data = match cal_dir.get_document("Fixed2Meta") {
            Some(document) => {
                // Java: FixedMeta calFixed2Meta = new FixedMeta(new DocumentInputStream(...), 9);
                let cal_fixed2_meta = FixedMeta::with_item_size(&document.get_data(), 9)?;
                // Java: calFixed2Data = new FixedData(calFixed2Meta, inputStreamFactory.getInstance(calDir, "Fixed2Data"), 48);
                Some(FixedData::with_max_expected_size(
                    &cal_fixed2_meta,
                    &input_stream_factory
                        .get_instance(&cal_dir, "Fixed2Data")?
                        .to_bytes(),
                    48,
                ))
            }
            None => None,
        };

        // Java: HashMap<Integer, ProjectCalendar> calendarMap = new HashMap<>();
        let mut calendar_map: HashMap<i32, Rc<RefCell<ProjectCalendar>>> = HashMap::new();
        // Java: int items = calFixedData.getItemCount();
        let items = cal_fixed_data.get_item_count();
        // Java: List<Pair<ProjectCalendar, Integer>> baseCalendars = new ArrayList<>();
        let mut base_calendars: Vec<(Rc<RefCell<ProjectCalendar>>, i32)> = Vec::new();

        // Java: byte[] defaultCalendarData = projectProps.getByteArray(PropsKey.DEFAULT_CALENDAR_HOURS);
        let default_calendar_data = project_props.get_byte_array(PropsKey::DefaultCalendarHours);

        // Java: ProjectCalendar defaultCalendar = new ProjectCalendar(m_file);
        //       processCalendarHours(defaultCalendarData, null, defaultCalendar, true);
        let mut default_calendar = ProjectCalendar::new(self.project_context_weak(), false);
        self.process_calendar_hours(
            factory,
            default_calendar_data,
            None,
            &mut default_calendar,
            true,
        );

        // Java: for (int loop = 0; loop < items; loop++)
        for loop_index in 0..items {
            // Java: byte[] fixedData = calFixedData.getByteArrayValue(loop);
            let Some(fixed_data) = cal_fixed_data.get_byte_array_value(loop_index) else {
                continue;
            };
            // Java: byte[] fixedData2 = calFixed2Data == null ? null : calFixed2Data.getByteArrayValue(loop);
            let fixed_data2 = cal_fixed2_data
                .as_ref()
                .and_then(|data| data.get_byte_array_value(loop_index));

            // Java: if (fixedData != null && fixedData.length >= 8)
            if fixed_data.len() < 8 {
                continue;
            }

            let mut offset: usize = 0;

            //
            // Bug 890909, here we ensure that we have a complete 12 byte
            // block before attempting to process the data.
            //
            // Java: while (offset + 12 <= fixedData.length)
            while offset + 12 <= fixed_data.len() {
                // Java: Integer calendarID = Integer.valueOf(ByteArrayHelper.getInt(fixedData, offset + getCalendarIDOffset()));
                let calendar_id =
                    get_int_safe(fixed_data, offset + factory.get_calendar_id_offset());
                // Java: int baseCalendarID = ByteArrayHelper.getInt(fixedData, offset + getBaseIDOffset());
                let base_calendar_id =
                    get_int_safe(fixed_data, offset + factory.get_base_id_offset());

                // Java: if (calendarID.intValue() > 0 && !calendarMap.containsKey(calendarID))
                if calendar_id > 0 && !calendar_map.contains_key(&calendar_id) {
                    // Java: byte[] varData = calVarData.getByteArray(calendarID, getCalendarDataVarDataType());
                    let mut var_data = cal_var_data.get_byte_array(
                        Some(calendar_id),
                        Some(factory.get_calendar_data_var_data_type()),
                    );

                    let cal: Rc<RefCell<ProjectCalendar>>;

                    // Java: if (baseCalendarID <= 0 || baseCalendarID == calendarID.intValue())
                    if base_calendar_id <= 0 || base_calendar_id == calendar_id {
                        // Java: if (varData != null || defaultCalendarData != null)
                        if var_data.is_some() || default_calendar_data.is_some() {
                            cal = self.add_calendar();
                            if var_data.is_none() {
                                var_data = default_calendar_data.map(<[u8]>::to_vec);
                            }
                        } else {
                            cal = self.add_default_base_calendar();
                        }

                        // Java: cal.setName(calVarData.getUnicodeString(calendarID, getCalendarNameVarDataType()));
                        let name = cal_var_data.get_unicode_string(
                            Some(calendar_id),
                            Some(factory.get_calendar_name_var_data_type()),
                        );
                        if let Ok(mut cal_ref) = cal.try_borrow_mut() {
                            cal_ref.set_name(name);
                        }

                        //
                        // In theory, base calendars should not have a resource ID attached to them.
                        // In practice, I've seen a few sample files where this is the case.
                        // As long as the resource ID isn't already linked to a calendar, we'll
                        // use the resource ID.
                        //
                        // Java: Integer resourceID = Integer.valueOf(ByteArrayHelper.getInt(fixedData, offset + getResourceIDOffset()));
                        let resource_id =
                            get_int_safe(fixed_data, offset + factory.get_resource_id_offset());
                        // Java: if (resourceID.intValue() > 0 && !resourceMap.containsKey(resourceID))
                        if resource_id > 0 && !resource_map.contains_key(&resource_id) {
                            resource_map.insert(resource_id, Rc::clone(&cal));
                        }
                    } else {
                        if var_data.is_some() {
                            cal = self.add_calendar();
                        } else {
                            cal = self.add_default_derived_calendar();
                        }

                        // Java: baseCalendars.add(new Pair<>(cal, Integer.valueOf(baseCalendarID)));
                        base_calendars.push((Rc::clone(&cal), base_calendar_id));

                        // Java: Integer resourceID = ...; resourceMap.put(resourceID, cal);
                        let resource_id =
                            get_int_safe(fixed_data, offset + factory.get_resource_id_offset());
                        resource_map.insert(resource_id, Rc::clone(&cal));
                    }

                    // Java: cal.setUniqueID(calendarID);
                    //       cal.setGUID(MPPUtility.getGUID(fixedData2, 0));
                    let guid = MPPUtility::get_guid(fixed_data2.unwrap_or_default(), 0);
                    if let Ok(mut cal_ref) = cal.try_borrow_mut() {
                        cal_ref.set_unique_id(Some(calendar_id));
                        cal_ref.set_guid(guid);
                    }

                    if var_data.is_none() {
                        if base_calendar_id <= 0 {
                            // Java: Stream.of(DayOfWeek.values()).forEach(cal::addCalendarHours);
                            for day in JAVA_DAY_OF_WEEK_VALUES {
                                if let Ok(mut cal_ref) = cal.try_borrow_mut() {
                                    let _ = cal_ref.add_calendar_hours(day);
                                }
                            }
                        }
                    } else if let Some(var_data) = &var_data {
                        // Java: processCalendarHours(varData, defaultCalendar, cal, baseCalendarID <= 0);
                        if let Ok(mut cal_ref) = cal.try_borrow_mut() {
                            self.process_calendar_hours(
                                factory,
                                Some(var_data.as_slice()),
                                Some(&default_calendar),
                                &mut cal_ref,
                                base_calendar_id <= 0,
                            );
                        }
                        // Java: processCalendarExceptions(varData, cal);
                        factory.process_calendar_exceptions(var_data, &cal);
                    }

                    // Java: calendarMap.put(calendarID, cal);
                    calendar_map.insert(calendar_id, Rc::clone(&cal));
                    // Java: eventManager.fireCalendarReadEvent(cal);
                    self.fire_calendar_read_event(&cal);
                }

                offset += 12;
            }
        }

        // Java: updateBaseCalendarNames(baseCalendars, calendarMap);
        self.update_base_calendar_names(&base_calendars, &calendar_map);

        // Java:
        // ProjectCalendar projectDefaultCalendar = m_file.getCalendars().getByName(projectProps.getUnicodeString(PropsKey.DEFAULT_CALENDAR_NAME));
        // if (projectDefaultCalendar == null) { projectDefaultCalendar = m_file.getCalendars().findOrCreateDefaultCalendar(); }
        // m_file.getProjectProperties().setDefaultCalendar(projectDefaultCalendar);
        let default_name = project_props.get_unicode_string(PropsKey::DefaultCalendarName);
        let found = match self.file.try_borrow() {
            Ok(file) => file.get_calendars().get_by_name(default_name.as_deref()),
            Err(_) => None,
        };

        let project_default_calendar = match found {
            Some(calendar) => Some(calendar),
            None => match self.file.try_borrow() {
                Ok(file) => {
                    let context = file.get_project_context();
                    drop(file);
                    match context.try_borrow_mut() {
                        Ok(mut context_ref) => {
                            context_ref.find_or_create_default_calendar()
                        }
                        Err(_) => None,
                    }
                }
                Err(_) => None,
            },
        };

        if let Some(project_default_calendar) = project_default_calendar {
            if let Ok(mut file) = self.file.try_borrow_mut() {
                file.set_default_calendar(Some(project_default_calendar));
            }
        }

        Ok(())
    }

/// Set the working-day flag for each day; also writes the working hours when the data block contains them.
    ///
    /// Corresponds to Java: `private void processCalendarHours(byte[] data, ProjectCalendar defaultCalendar,
    /// ProjectCalendar cal, boolean isBaseCalendar)`
    ///
/// Java's `data` / `defaultCalendar` may be `null`, corresponding to `None` on the Rust side.
/// In Java `cal` may be either a freshly constructed standalone instance (the default calendar) or a calendar in the file,
/// so the Rust side accepts `&mut ProjectCalendar` (the caller borrows it out of the `Rc<RefCell<..>>` itself).
    ///
    /// NOTE: MPP14 defines the concept of working weeks. MPXJ does not
    /// currently support this, and thus we only read the working hours
    /// for the default working week.
    fn process_calendar_hours(
        &self,
        factory: &dyn AbstractCalendarFactoryMethods,
        data: Option<&[u8]>,
        default_calendar: Option<&ProjectCalendar>,
        cal: &mut ProjectCalendar,
        is_base_calendar: bool,
    ) {
        for index in 0..7usize {
            // Java: offset = getCalendarHoursOffset() + (60 * index);
            let offset = factory.get_calendar_hours_offset() + (60 * index);
            // Java: defaultFlag = data == null ? 1 : ByteArrayHelper.getShort(data, offset);
            let default_flag = match data {
                None => 1,
                Some(data) => get_short_safe(data, offset),
            };
            // Java: day = DayOfWeekHelper.getInstance(index + 1);
            let Some(day) = DayOfWeekHelper::get_instance(index as i32 + 1) else {
                continue;
            };

            if default_flag == 1 {
                if is_base_calendar {
                    // Java: hours = cal.addCalendarHours(day);
                    let _ = cal.add_calendar_hours(day);

                    match default_calendar {
                        None => {
                            // Java: cal.setWorkingDay(day, DEFAULT_WORKING_WEEK[index]);
                            cal.set_working_day(day, DEFAULT_WORKING_WEEK[index]);
                            if cal.is_working_day(day) {
                                // Java: hours.add(ProjectCalendarDays.DEFAULT_WORKING_MORNING);
                                //       hours.add(ProjectCalendarDays.DEFAULT_WORKING_AFTERNOON);
                                if let Some(hours) = cal.get_calendar_hours_for_day_mut(day) {
                                    hours.add(ProjectCalendarDays::default_working_morning());
                                    hours.add(ProjectCalendarDays::default_working_afternoon());
                                }
                            }
                        }
                        Some(default_calendar) => {
                            // Java: boolean workingDay = defaultCalendar.isWorkingDay(day);
                            //       cal.setWorkingDay(day, workingDay);
                            //       if (workingDay) { hours.addAll(defaultCalendar.getHours(day)); }
                            let working_day = default_calendar.is_working_day(day);
                            cal.set_working_day(day, working_day);
                            if working_day {
// Java's getHours may return null (working hours undefined), hence Option is used to receive it
                                if let Some(default_hours) = default_calendar.get_hours(day) {
                                    if let Some(hours) = cal.get_calendar_hours_for_day_mut(day) {
                                        hours.add_all(&default_hours);
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // Java: cal.setCalendarDayType(day, DayType.DEFAULT);
                    cal.set_calendar_day_type(day, DayType::Default);
                }
            } else {
                //
// Working hours are explicitly defined
                //
                // Java: periodIndex = 0; periodCount = ByteArrayHelper.getShort(data, offset + 2);
// (defaultFlag != 1 can only come from Some(data), so this fallback never triggers)
                let Some(data) = data else {
                    continue;
                };
                let period_count = get_short_safe(data, offset + 2);
                let mut date_ranges: Vec<LocalTimeRange> = Vec::new();

                let mut period_index: i32 = 0;
                while period_index < period_count {
                    // Java: LocalTime start = MPPUtility.getTime(data, offset + 8 + (periodIndex * 2));
                    let start =
                        MPPUtility::get_time(data, offset + 8 + (period_index as usize * 2));
                    // Java: duration = MPPUtility.getDuration(data, offset + 20 + (periodIndex * 4));
                    let duration =
                        MPPUtility::get_duration(data, offset + 20 + (period_index as usize * 4));
                    // Java: LocalTime end = start.plus(duration, ChronoUnit.MILLIS);
                    let end = plus_milliseconds(start, duration);
                    date_ranges.push(LocalTimeRange::new(Some(start), Some(end)));
                    period_index += 1;
                }

                if date_ranges.is_empty() {
                    if is_base_calendar {
                        // Java: cal.addCalendarHours(DayOfWeekHelper.getInstance(index + 1));
                        let _ = cal.add_calendar_hours(day);
                    }
                    // Java: cal.setWorkingDay(day, false);
                    cal.set_working_day(day, false);
                } else {
                    // Java: cal.setWorkingDay(day, true);
                    //       hours = cal.addCalendarHours(DayOfWeekHelper.getInstance(index + 1));
                    //       hours.addAll(dateRanges);
                    cal.set_working_day(day, true);
                    let hours = cal.add_calendar_hours(day);
                    for range in date_ranges {
                        hours.add(range);
                    }
                }
            }
        }
    }

/// Convert the "base calendar ID" placeholder (an MPP14 derived calendar may forward-reference its base calendar) into a real parent-child relationship.
    ///
    /// Corresponds to Java: `private void updateBaseCalendarNames(List<Pair<ProjectCalendar, Integer>> baseCalendars,
    /// HashMap<Integer, ProjectCalendar> map)`
    fn update_base_calendar_names(
        &self,
        base_calendars: &[(Rc<RefCell<ProjectCalendar>>, i32)],
        map: &HashMap<i32, Rc<RefCell<ProjectCalendar>>>,
    ) {
        for (cal, base_calendar_id) in base_calendars {
            let base_cal = map.get(base_calendar_id);

            // Java: if (baseCal != null && baseCal.getName() != null)
            let valid = match base_cal {
                Some(base_cal) => match base_cal.try_borrow() {
                    Ok(base_cal_ref) => base_cal_ref.get_name().is_some(),
                    Err(_) => false,
                },
                None => false,
            };

            if valid {
                if let Ok(mut cal_ref) = cal.try_borrow_mut() {
                    cal_ref.set_parent(base_cal.map(Rc::clone));
                }
            } else {
// Java: m_file.removeCalendar(cal); (removes the invalid calendar to avoid serious problems later)
                if let Ok(mut file) = self.file.try_borrow_mut() {
                    file.remove_calendar(cal);
                }
            }
        }
    }

/// Get a weak reference to the context (used when creating a standalone calendar).
    ///
/// Java's `new ProjectCalendar(m_file)` internally obtains `file.getProjectContext()` (a strong reference);
/// the Rust [`ProjectCalendar::new`] needs a `Weak<RefCell<ProjectContext>>`.
    fn project_context_weak(
        &self,
    ) -> std::rc::Weak<RefCell<crate::project_context::ProjectContext>> {
        match self.file.try_borrow() {
            Ok(file) => Rc::downgrade(&file.get_project_context()),
            Err(_) => std::rc::Weak::new(),
        }
    }

/// Create a new calendar.
    ///
    /// Corresponds to Java: `m_file.addCalendar()`
    ///
/// When the file is already borrowed (which cannot happen in Java) returns a detached calendar to guarantee no panic.
    fn add_calendar(&self) -> Rc<RefCell<ProjectCalendar>> {
        match self.file.try_borrow_mut() {
            Ok(mut file) => file.add_calendar(),
            Err(_) => Rc::new(RefCell::new(ProjectCalendar::new(
                std::rc::Weak::new(),
                false,
            ))),
        }
    }

/// Create a base calendar named "Standard".
    ///
    /// Corresponds to Java: `m_file.addDefaultBaseCalendar()`
    fn add_default_base_calendar(&self) -> Rc<RefCell<ProjectCalendar>> {
        match self.file.try_borrow_mut() {
            Ok(mut file) => file.add_default_base_calendar(),
            Err(_) => self.add_calendar(),
        }
    }

/// Create a default derived calendar.
    ///
    /// Corresponds to Java: `m_file.addDefaultDerivedCalendar()`
    fn add_default_derived_calendar(&self) -> Rc<RefCell<ProjectCalendar>> {
        match self.file.try_borrow_mut() {
            Ok(mut file) => file.add_default_derived_calendar(),
            Err(_) => self.add_calendar(),
        }
    }

/// Fire the "calendar read" event.
    ///
    /// Corresponds to Java: `m_file.getEventManager().fireCalendarReadEvent(cal)`
    ///
/// The Rust-side event manager hangs off `ProjectContext` (`EventManager::fire_calendar_read_event`
/// needs `&mut`), so it is fired through a mutable borrow of the context.
    fn fire_calendar_read_event(&self, calendar: &Rc<RefCell<ProjectCalendar>>) {
        let Ok(file) = self.file.try_borrow() else {
            return;
        };
        let context = file.get_project_context();
        drop(file);

        if let Ok(mut context_ref) = context.try_borrow_mut() {
            context_ref
                .get_event_manager_mut()
                .fire_calendar_read_event(calendar);
        }
    }
}

/// Read a little-endian `short`, returning 0 when fewer than two bytes of data are available.
///
/// [`ByteArrayHelper::get_short`] panics when out of bounds (Java throws
/// `ArrayIndexOutOfBoundsException`), so this function is used throughout this module for bounds protection.
pub(crate) fn get_short_safe(data: &[u8], offset: usize) -> i32 {
    if offset.checked_add(2).map_or(true, |end| end > data.len()) {
// Java throws ArrayIndexOutOfBoundsException; the Rust side returns 0
        return 0;
    }

    ByteArrayHelper::get_short(data, offset)
}

/// Read a little-endian `int`, returning 0 when fewer than four bytes of data are available.
///
/// Same rationale as [`get_short_safe`].
pub(crate) fn get_int_safe(data: &[u8], offset: usize) -> i32 {
    if offset.checked_add(4).map_or(true, |end| end > data.len()) {
// Java throws ArrayIndexOutOfBoundsException; the Rust side returns 0
        return 0;
    }

    ByteArrayHelper::get_int(data, offset)
}

/// Add a number of milliseconds to a `LocalTime`, wrapping around a day (24 hours).
///
/// Corresponds to Java: `LocalTime.plus(long amountToAdd, ChronoUnit.MILLIS)` — Java's
/// `LocalTime` is a clock modulo 24 hours and `plus` wraps around; chrono's add/subtract reports an overflow error, so it is replicated here.
pub(crate) fn plus_milliseconds(time: NaiveTime, milliseconds: i64) -> NaiveTime {
    const MILLIS_PER_DAY: i64 = 86_400_000;

    let current = i64::from(time.num_seconds_from_midnight()) * 1_000
        + i64::from(time.nanosecond() / 1_000_000);
    let total = (current + milliseconds).rem_euclid(MILLIS_PER_DAY);
    let seconds = (total / 1_000) as u32;
    let nanos = ((total % 1_000) * 1_000_000) as u32;

    NaiveTime::from_num_seconds_from_midnight_opt(seconds, nanos).unwrap_or(NaiveTime::MIN)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

/// Test "subclass": implements the abstract methods, whose behavior matches `MPP9CalendarFactory`
/// (calendar ID offset 0, base calendar offset 4, resource offset 8, working hours offset 4).
    struct TestFactory;

    impl AbstractCalendarFactoryMethods for TestFactory {
        fn get_calendar_id_offset(&self) -> usize {
            0
        }

        fn get_base_id_offset(&self) -> usize {
            4
        }

        fn get_resource_id_offset(&self) -> usize {
            8
        }

        fn get_calendar_hours_offset(&self) -> usize {
            4
        }

        fn get_calendar_name_var_data_type(&self) -> i32 {
            1
        }

        fn get_calendar_data_var_data_type(&self) -> i32 {
            3
        }

        fn get_calendar_var_meta(
            &self,
            directory: &DirectoryEntry,
        ) -> MpxjResult<Rc<dyn VarMeta>> {
            let Some(document) = directory.get_document("VarMeta") else {
                return Err(MpxjError::new("Missing VarMeta"));
            };
            Ok(Rc::new(crate::mpp::var_meta9::VarMeta9::new(
                &document.get_data(),
            )?))
        }

        fn process_calendar_exceptions(&self, _data: &[u8], _cal: &Rc<RefCell<ProjectCalendar>>) {
// The tests in this module cover only working-hours parsing
        }
    }

/// Weekday block (60 bytes): default flag + number of periods + each period's start time / duration.
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

/// Time value: a value measured from midnight in units of one tenth of a minute.
    fn tenths_of_minute(hour: u32, minute: u32) -> u16 {
        ((hour * 60 + minute) * 10) as u16
    }

/// Duration value: in units of one tenth of a minute.
    fn duration_tenths(hours: u32) -> u32 {
        hours * 60 * 10
    }

    fn default_week_blocks() -> Vec<u8> {
// 4-byte block header (TestFactory's get_calendar_hours_offset() is 4)
        let mut data = vec![0u8; 4];
// The order is Sunday, Monday … Saturday (DayOfWeekHelper.getInstance(index + 1))
        data.extend_from_slice(&day_block(1, &[])); // Sunday: use default value
        data.extend_from_slice(&day_block(
            0,
            &[
                (tenths_of_minute(9, 0), duration_tenths(4)),
                (tenths_of_minute(14, 0), duration_tenths(3)),
            ],
        )); // Monday: 09:00-13:00 / 14:00-17:00
        data.extend_from_slice(&day_block(0, &[])); // Tuesday: rest day
        data.extend_from_slice(&day_block(1, &[])); // Wednesday: use default value
        data.extend_from_slice(&day_block(0, &[])); // Thursday: rest day
        data.extend_from_slice(&day_block(1, &[])); // Friday: use default value
        data.extend_from_slice(&day_block(0, &[])); // Saturday: rest day
        data
    }

    fn time(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).expect("valid time")
    }

/// Build a "calendar data" byte stream containing only the working-hours data block and drive parsing.
    ///
/// The data must include the 4-byte block header (this test's offset convention is `get_calendar_hours_offset() == 4`).
    fn process_week_blocks(data: &[u8]) -> ProjectCalendar {
        let file = ProjectFile::new();
        let factory = TestFactory;
        let base = AbstractCalendarFactory::new(Rc::clone(&file));

        let mut calendar = ProjectCalendar::new(base.project_context_weak(), false);
        base.process_calendar_hours(&factory, Some(data), None, &mut calendar, true);
        calendar
    }

    #[test]
    fn explicit_working_hours_are_parsed() {
        let calendar = process_week_blocks(&default_week_blocks());

        assert_eq!(calendar.get_day_type(Weekday::Mon), Some(DayType::Working));
        let monday = calendar.get_hours(Weekday::Mon).expect("Monday should have working hours");
        assert_eq!(monday.size(), 2);
        assert_eq!(
            monday.get(0),
            LocalTimeRange::new(Some(time(9, 0)), Some(time(13, 0)))
        );
        assert_eq!(
            monday.get(1),
            LocalTimeRange::new(Some(time(14, 0)), Some(time(17, 0)))
        );

        assert_eq!(
            calendar.get_day_type(Weekday::Tue),
            Some(DayType::NonWorking)
        );
        assert_eq!(calendar.get_hours(Weekday::Tue).map(|h| h.size()), Some(0));
    }

    #[test]
    fn default_flag_days_fall_back_to_the_default_working_week() {
// No defaultCalendar provided (Java's null): Monday to Friday at 08:00-12:00 / 13:00-17:00
        let calendar = process_week_blocks(&default_week_blocks());

        assert_eq!(
            calendar.get_day_type(Weekday::Wed),
            Some(DayType::Working)
        );
        let wednesday = calendar.get_hours(Weekday::Wed).expect("Wednesday should have default working hours");
        assert_eq!(wednesday.size(), 2);
        assert_eq!(
            wednesday.get(0),
            LocalTimeRange::new(Some(time(8, 0)), Some(time(12, 0)))
        );
        assert_eq!(
            wednesday.get(1),
            LocalTimeRange::new(Some(time(13, 0)), Some(time(17, 0)))
        );

// Sunday / Saturday are the default non-working days
        assert_eq!(
            calendar.get_day_type(Weekday::Sun),
            Some(DayType::NonWorking)
        );
        assert_eq!(
            calendar.get_day_type(Weekday::Sat),
            Some(DayType::NonWorking)
        );
    }

    #[test]
    fn date_ranges_are_filled_from_the_supplied_default_calendar() {
        let file = ProjectFile::new();
        let factory = TestFactory;
        let base = AbstractCalendarFactory::new(Rc::clone(&file));

// Default calendar: all default flags (Monday to Friday 08:00-12:00 / 13:00-17:00)
        let mut default_calendar = ProjectCalendar::new(std::rc::Weak::new(), false);
        base.process_calendar_hours(&factory, None, None, &mut default_calendar, true);

        let mut calendar = ProjectCalendar::new(std::rc::Weak::new(), false);
        let data = default_week_blocks();
        base.process_calendar_hours(
            &factory,
            Some(data.as_slice()),
            Some(&default_calendar),
            &mut calendar,
            true,
        );

        let friday = calendar.get_hours(Weekday::Fri).expect("Friday should inherit default hours");
        assert_eq!(friday.size(), 2);
        assert_eq!(
            friday.get(0),
            LocalTimeRange::new(Some(time(8, 0)), Some(time(12, 0)))
        );
    }

    #[test]
    fn derived_calendar_default_days_are_marked_default() {
        let file = ProjectFile::new();
        let factory = TestFactory;
        let base = AbstractCalendarFactory::new(Rc::clone(&file));

        let mut calendar = ProjectCalendar::new(std::rc::Weak::new(), false);
// isBaseCalendar = false: days with the default flag are written as DayType::DEFAULT
        let data = default_week_blocks();
        base.process_calendar_hours(
            &factory,
            Some(data.as_slice()),
            None,
            &mut calendar,
            false,
        );

        assert_eq!(
            calendar.get_calendar_day_type(Weekday::Sun),
            Some(DayType::Default)
        );
        assert_eq!(
            calendar.get_calendar_day_type(Weekday::Mon),
            Some(DayType::Working)
        );
        assert_eq!(
            calendar.get_calendar_day_type(Weekday::Tue),
            Some(DayType::NonWorking)
        );
    }

    #[test]
    fn truncated_data_does_not_panic() {
        let file = ProjectFile::new();
        let factory = TestFactory;
        let base = AbstractCalendarFactory::new(Rc::clone(&file));

// The data block is far shorter than 7 × 60 bytes: every read is out of bounds (Java throws, Rust returns 0 / empty)
        for data in [vec![], vec![0u8; 4], vec![0u8; 30]] {
            let mut calendar = ProjectCalendar::new(std::rc::Weak::new(), false);
            base.process_calendar_hours(
                &factory,
                Some(data.as_slice()),
                None,
                &mut calendar,
                true,
            );
        }
    }

    #[test]
    fn plus_milliseconds_wraps_around_midnight() {
        assert_eq!(plus_milliseconds(time(23, 30), 60 * 60 * 1000), time(0, 30));
        assert_eq!(plus_milliseconds(time(8, 0), 0), time(8, 0));
    }

    #[test]
    fn safe_readers_return_zero_when_out_of_range() {
        assert_eq!(get_short_safe(&[1, 2], 1), 0);
        assert_eq!(get_short_safe(&[1, 2], 0), 0x0201);
        assert_eq!(get_int_safe(&[1, 2, 3], 0), 0);
        assert_eq!(get_int_safe(&[1, 2, 3, 4], 0), 0x0403_0201);
    }
}
