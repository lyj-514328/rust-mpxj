//! 对应 Java: `org/mpxj/mpp/AbstractCalendarFactory.java`
//!
//! MPP9 / MPP12 / MPP14 三个版本的日历读取所共用的解析逻辑：按 `TBkndCal` 存储里的
//! `VarMeta` / `Var2Data` / `FixedMeta` / `FixedData` 组合出每个日历的工作时段。
//!
//! # 继承关系的还原
//!
//! Java: `abstract class AbstractCalendarFactory implements CalendarFactory`，
//! `MPP9CalendarFactory extends AbstractCalendarFactory`，
//! `MPP12/MPP14CalendarFactory extends AbstractCalendarAndExceptionFactory extends AbstractCalendarFactory`。
//! Rust 没有继承，按下列方式还原（组合 + 转发，与 `VarMeta`/`AbstractVarMeta` 的做法一致）：
//!
//! | Java | Rust |
//! |---|---|
//! | `interface CalendarFactory` | [`crate::mpp::calendar_factory::CalendarFactory`] trait |
//! | `abstract class AbstractCalendarFactory` | 本文件的 [`AbstractCalendarFactory`] 结构体（持有 `file`） |
//! | 基类的 8 个 `protected abstract` 方法 | [`AbstractCalendarFactoryMethods`] trait（由子类实现） |
//! | 基类实现 `processCalendarData`（虚调用抽象方法） | [`AbstractCalendarFactory::process_calendar_data`] 增加 `factory: &dyn AbstractCalendarFactoryMethods` 参数，由基类回调子类实现 |
//! | `MPP9CalendarFactory extends AbstractCalendarFactory` | [`crate::mpp::mpp9_calendar_factory::MPP9CalendarFactory`] **组合**一个 `AbstractCalendarFactory`，并把 `process_calendar_data` 转发给它（同时把自己作为 `factory` 传入） |
//!
//! 之所以需要 [`AbstractCalendarFactoryMethods`]：Java 的 `processCalendarData` 会调用
//! `getCalendarIDOffset()` 等由子类覆写的抽象方法，Rust 没有虚方法，只能显式把子类
//! 作为 trait 对象传入（等价于 Java 的动态分派）。抽象方法名与 Java 保持 1:1。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `ProjectFile m_file` | `Rc<RefCell<ProjectFile>>` | 按约定共享；访问用 `try_borrow` / `try_borrow_mut`，避免借用冲突 panic |
//! | `ProjectCalendar` | `Rc<RefCell<ProjectCalendar>>` | 日历需要在文件容器与局部变量间共享 |
//! | `byte[]` | `&[u8]` / `Option<&[u8]>` | Java 的 `null` → `None` |
//! | `Pair<ProjectCalendar, Integer>` | `(Rc<RefCell<ProjectCalendar>>, i32)` | [`crate::common::pair`] 的 `Pair` 未被使用 |
//! | `ByteArrayHelper.getShort/getInt` | `get_short_safe` / `get_int_safe` | Java 越界抛异常，Rust 必须不 panic（见函数注释） |
//! | `LocalTime.plus(duration, MILLIS)` | [`plus_milliseconds`] | chrono 没有等价的「按毫秒回绕」方法 |
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
//!
//! 跳过的 Java 成员：无（`processCalendarHours` 为 `private`，Rust 侧同样不对模块外暴露）。

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

/// 默认工作周（下标与 `DayOfWeekHelper::get_instance(index + 1)` 对应：0 = 周日）。
///
/// 对应 Java: `private static final boolean[] DEFAULT_WORKING_WEEK`
const DEFAULT_WORKING_WEEK: [bool; 7] = [false, true, true, true, true, true, false];

/// Java 的 `DayOfWeek.values()` 的顺序（周一 → 周日）。
///
/// 注意与 [`DayOfWeekHelper::ORDERED_DAYS`]（周日 → 周六）不同，Java 的
/// `Stream.of(DayOfWeek.values())` 用的是枚举声明顺序。
const JAVA_DAY_OF_WEEK_VALUES: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];

/// `AbstractCalendarFactory` 中由子类覆写的抽象方法的 Rust 表达。
///
/// 对应 Java: `AbstractCalendarFactory` 的 8 个 `protected abstract` 方法。
/// Java 的基类在 `processCalendarData` 中虚调用它们；Rust 侧改为由调用方把具体工厂
/// 作为 `&dyn` 传入（见文件头「继承关系的还原」）。
pub trait AbstractCalendarFactoryMethods {
    /// 日历唯一 ID 在定长数据块中的偏移。
    ///
    /// 对应 Java: `protected abstract int getCalendarIDOffset()`
    fn get_calendar_id_offset(&self) -> usize;

    /// 基日历唯一 ID 在定长数据块中的偏移。
    ///
    /// 对应 Java: `protected abstract int getBaseIDOffset()`
    fn get_base_id_offset(&self) -> usize;

    /// 资源 ID 在定长数据块中的偏移。
    ///
    /// 对应 Java: `protected abstract int getResourceIDOffset()`
    fn get_resource_id_offset(&self) -> usize;

    /// 读取 `TBkndCal` 下的 `VarMeta` 流。
    ///
    /// 对应 Java: `protected abstract VarMeta getCalendarVarMeta(DirectoryEntry directory) throws IOException`
    fn get_calendar_var_meta(&self, directory: &DirectoryEntry) -> MpxjResult<Rc<dyn VarMeta>>;

    /// 每个星期的工作时段块在日历数据块中的起始偏移。
    ///
    /// 对应 Java: `protected abstract int getCalendarHoursOffset()`
    fn get_calendar_hours_offset(&self) -> usize;

    /// 存放日历名称的 VarData 类型。
    ///
    /// 对应 Java: `protected abstract Integer getCalendarNameVarDataType()`
    fn get_calendar_name_var_data_type(&self) -> i32;

    /// 存放日历数据的 VarData 类型。
    ///
    /// 对应 Java: `protected abstract Integer getCalendarDataVarDataType()`
    fn get_calendar_data_var_data_type(&self) -> i32;

    /// 解析日历数据块中的例外日期。
    ///
    /// 对应 Java: `protected abstract void processCalendarExceptions(byte[] data, ProjectCalendar cal)`
    fn process_calendar_exceptions(&self, data: &[u8], cal: &Rc<RefCell<ProjectCalendar>>);
}

/// 日历读取的公共实现。
///
/// 对应 Java: `abstract class AbstractCalendarFactory implements CalendarFactory`
pub struct AbstractCalendarFactory {
    /// 对应 Java 字段: `private final ProjectFile m_file;`
    file: Rc<RefCell<ProjectFile>>,
}

impl AbstractCalendarFactory {
    /// 构造方法。
    ///
    /// 对应 Java: `public AbstractCalendarFactory(ProjectFile file)`
    ///
    /// Java 侧持有 `ProjectFile` 强引用；Rust 侧按约定用 `Rc<RefCell<ProjectFile>>` 共享。
    pub fn new(file: Rc<RefCell<ProjectFile>>) -> Self {
        Self { file }
    }

    /// 取所属项目文件。
    ///
    /// Rust 特有：Java 的子类直接访问 protected 字段 `m_file`，Rust 侧提供读取入口。
    pub fn get_file(&self) -> &Rc<RefCell<ProjectFile>> {
        &self.file
    }

    /// 日历数据块的结构：4 字节块头 + 7 × 60 字节（一周七天）+（MPP12 起）例外日期块。
    ///
    /// 对应 Java: `public void processCalendarData(DirectoryEntry projectDir, Props projectProps,
    /// DocumentInputStreamFactory inputStreamFactory, HashMap<Integer, ProjectCalendar> resourceMap)`
    ///
    /// `factory` 是「子类」——Java 侧由继承与虚方法提供抽象方法实现，Rust 侧显式传入。
    pub fn process_calendar_data(
        &self,
        factory: &dyn AbstractCalendarFactoryMethods,
        project_dir: &DirectoryEntry,
        project_props: &Props,
        input_stream_factory: &DocumentInputStreamFactory,
        resource_map: &mut HashMap<i32, Rc<RefCell<ProjectCalendar>>>,
    ) -> MpxjResult<()> {
        // Java: DirectoryEntry calDir = (DirectoryEntry) projectDir.getEntry("TBkndCal");
        // POI 找不到条目时抛异常，强转失败时抛 ClassCastException；Rust 侧返回 Err。
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
                            context_ref.get_calendars_mut().find_or_create_default_calendar()
                        }
                        Err(_) => None,
                    }
                }
                Err(_) => None,
            },
        };

        if let Some(project_default_calendar) = project_default_calendar {
            if let Ok(mut file) = self.file.try_borrow_mut() {
                file.get_project_properties_mut()
                    .set_default_calendar(Some(project_default_calendar));
            }
        }

        Ok(())
    }

    /// 设置每一天的工作日标志；数据块里存在工作时段时一并写入。
    ///
    /// 对应 Java: `private void processCalendarHours(byte[] data, ProjectCalendar defaultCalendar,
    /// ProjectCalendar cal, boolean isBaseCalendar)`
    ///
    /// Java 的 `data` / `defaultCalendar` 允许为 `null`，Rust 侧对应 `None`。
    /// `cal` 在 Java 里既可能是刚构造的独立实例（默认日历），也可能是文件中的日历，
    /// 故 Rust 侧接收 `&mut ProjectCalendar`（调用方自行从 `Rc<RefCell<..>>` 借出）。
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
                                // Java 的 getHours 可能返回 null（未定义工作时段），故用 Option 承接
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
                // 显式定义了工作时段
                //
                // Java: periodIndex = 0; periodCount = ByteArrayHelper.getShort(data, offset + 2);
                // （defaultFlag != 1 只可能来自 Some(data)，此处的兜底不会触发）
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

    /// 把「基日历 ID」占位（MPP14 的派生日历可能前向引用基日历）转换为真正的父子关系。
    ///
    /// 对应 Java: `private void updateBaseCalendarNames(List<Pair<ProjectCalendar, Integer>> baseCalendars,
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
                // Java: m_file.removeCalendar(cal);（移除非法日历，避免后续出现严重问题）
                if let Ok(mut file) = self.file.try_borrow_mut() {
                    file.remove_calendar(cal);
                }
            }
        }
    }

    /// 取上下文的弱引用（供新建独立日历时使用）。
    ///
    /// Java 的 `new ProjectCalendar(m_file)` 内部取 `file.getProjectContext()`（强引用）；
    /// Rust 的 [`ProjectCalendar::new`] 需要 `Weak<RefCell<ProjectContext>>`。
    fn project_context_weak(
        &self,
    ) -> std::rc::Weak<RefCell<crate::project_context::ProjectContext>> {
        match self.file.try_borrow() {
            Ok(file) => Rc::downgrade(&file.get_project_context()),
            Err(_) => std::rc::Weak::new(),
        }
    }

    /// 新建日历。
    ///
    /// 对应 Java: `m_file.addCalendar()`
    ///
    /// 文件正被借用（Java 不会出现）时返回一个游离日历，保证不 panic。
    fn add_calendar(&self) -> Rc<RefCell<ProjectCalendar>> {
        match self.file.try_borrow_mut() {
            Ok(mut file) => file.add_calendar(),
            Err(_) => Rc::new(RefCell::new(ProjectCalendar::new(
                std::rc::Weak::new(),
                false,
            ))),
        }
    }

    /// 新建名为 “Standard” 的基准日历。
    ///
    /// 对应 Java: `m_file.addDefaultBaseCalendar()`
    fn add_default_base_calendar(&self) -> Rc<RefCell<ProjectCalendar>> {
        match self.file.try_borrow_mut() {
            Ok(mut file) => file.add_default_base_calendar(),
            Err(_) => self.add_calendar(),
        }
    }

    /// 新建一个默认派生日历。
    ///
    /// 对应 Java: `m_file.addDefaultDerivedCalendar()`
    fn add_default_derived_calendar(&self) -> Rc<RefCell<ProjectCalendar>> {
        match self.file.try_borrow_mut() {
            Ok(mut file) => file.add_default_derived_calendar(),
            Err(_) => self.add_calendar(),
        }
    }

    /// 触发「日历已读取」事件。
    ///
    /// 对应 Java: `m_file.getEventManager().fireCalendarReadEvent(cal)`
    ///
    /// Rust 侧的事件管理器挂在 `ProjectContext` 上（`EventManager::fire_calendar_read_event`
    /// 需要 `&mut`），故经由上下文的可变借用触发。
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

/// 读取小端 `short`，数据不足两个字节时返回 0。
///
/// [`ByteArrayHelper::get_short`] 越界时会 panic（Java 抛
/// `ArrayIndexOutOfBoundsException`），本模块内统一用本函数做边界保护。
pub(crate) fn get_short_safe(data: &[u8], offset: usize) -> i32 {
    if offset.checked_add(2).map_or(true, |end| end > data.len()) {
        // Java 会抛 ArrayIndexOutOfBoundsException；Rust 侧返回 0
        return 0;
    }

    ByteArrayHelper::get_short(data, offset)
}

/// 读取小端 `int`，数据不足四个字节时返回 0。
///
/// 理由同 [`get_short_safe`]。
pub(crate) fn get_int_safe(data: &[u8], offset: usize) -> i32 {
    if offset.checked_add(4).map_or(true, |end| end > data.len()) {
        // Java 会抛 ArrayIndexOutOfBoundsException；Rust 侧返回 0
        return 0;
    }

    ByteArrayHelper::get_int(data, offset)
}

/// 在 `LocalTime` 上加上若干毫秒，并按一天（24 小时）回绕。
///
/// 对应 Java: `LocalTime.plus(long amountToAdd, ChronoUnit.MILLIS)` —— Java 的
/// `LocalTime` 是模 24 小时的时钟，`plus` 会回绕；chrono 的加减法会溢出报错，故此处复刻。
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

    /// 测试用「子类」：实现抽象方法，抽象方法行为与 `MPP9CalendarFactory` 一致
    /// （日历 ID 偏移 0、基日历偏移 4、资源偏移 8、工作时段偏移 4）。
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
            // 本模块的测试只覆盖工作时段解析
        }
    }

    /// 工作日块（60 字节）：默认标志 + 时段数 + 各时段的开始时间 / 时长。
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

    /// 时间值：以「十分之一分钟」为单位的自午夜起算值。
    fn tenths_of_minute(hour: u32, minute: u32) -> u16 {
        ((hour * 60 + minute) * 10) as u16
    }

    /// 时长值：以「十分之一分钟」为单位。
    fn duration_tenths(hours: u32) -> u32 {
        hours * 60 * 10
    }

    fn default_week_blocks() -> Vec<u8> {
        // 4 字节块头（TestFactory 的 get_calendar_hours_offset() 为 4）
        let mut data = vec![0u8; 4];
        // 顺序为 周日、周一 … 周六（DayOfWeekHelper.getInstance(index + 1)）
        data.extend_from_slice(&day_block(1, &[])); // 周日：用默认值
        data.extend_from_slice(&day_block(
            0,
            &[
                (tenths_of_minute(9, 0), duration_tenths(4)),
                (tenths_of_minute(14, 0), duration_tenths(3)),
            ],
        )); // 周一：09:00-13:00 / 14:00-17:00
        data.extend_from_slice(&day_block(0, &[])); // 周二：休息
        data.extend_from_slice(&day_block(1, &[])); // 周三：用默认值
        data.extend_from_slice(&day_block(0, &[])); // 周四：休息
        data.extend_from_slice(&day_block(1, &[])); // 周五：用默认值
        data.extend_from_slice(&day_block(0, &[])); // 周六：休息
        data
    }

    fn time(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).expect("合法时间")
    }

    /// 构造仅含工作时段数据块的「日历数据」字节流并驱动解析。
    ///
    /// 数据需要包含 4 字节块头（本测试的偏移约定为 `get_calendar_hours_offset() == 4`）。
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
        let monday = calendar.get_hours(Weekday::Mon).expect("周一应有工作时段");
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
        // 未提供 defaultCalendar（Java 的 null）：周一至周五按 08:00-12:00 / 13:00-17:00
        let calendar = process_week_blocks(&default_week_blocks());

        assert_eq!(
            calendar.get_day_type(Weekday::Wed),
            Some(DayType::Working)
        );
        let wednesday = calendar.get_hours(Weekday::Wed).expect("周三应有默认工作时段");
        assert_eq!(wednesday.size(), 2);
        assert_eq!(
            wednesday.get(0),
            LocalTimeRange::new(Some(time(8, 0)), Some(time(12, 0)))
        );
        assert_eq!(
            wednesday.get(1),
            LocalTimeRange::new(Some(time(13, 0)), Some(time(17, 0)))
        );

        // 周日 / 周六为默认的非工作日
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

        // 默认日历：全部为默认标志（周一至周五 08:00-12:00 / 13:00-17:00）
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

        let friday = calendar.get_hours(Weekday::Fri).expect("周五应继承默认时段");
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
        // isBaseCalendar = false：默认标志的日子写入 DayType::DEFAULT
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

        // 数据块远短于 7 × 60 字节：所有取数都越界（Java 抛异常，Rust 返回 0 / 空）
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
