//! 对应 Java: `org/mpxj/mpp/AbstractCalendarAndExceptionFactory.java`
//!
//! 在 [`crate::mpp::abstract_calendar_factory::AbstractCalendarFactory`] 的基础上，
//! 补齐 MPP12 / MPP14 共用的「例外日期 + 工作周」解析：数据块前 420 字节是标准工作时段，
//! 其后是例外日期块与工作周块。
//!
//! # 继承关系的还原
//!
//! Java: `abstract class AbstractCalendarAndExceptionFactory extends AbstractCalendarFactory`。
//! Rust 侧按「组合 + 转发」还原（与 `VarMeta` / `AbstractVarMeta` 的做法一致）：
//!
//! | Java | Rust |
//! |---|---|
//! | `extends AbstractCalendarFactory` | 字段 `base: AbstractCalendarFactory`（组合） |
//! | 继承而来的 `processCalendarData` | [`AbstractCalendarAndExceptionFactory::process_calendar_data`] 转发给 `base`，并把子类作为 `factory` 传入 |
//! | 继承而来的 8 个抽象方法 | 仍由最终子类实现 [`AbstractCalendarFactoryMethods`]（本类型不参与，避免重复实现） |
//! | 本类覆写的 `processCalendarExceptions` | [`AbstractCalendarAndExceptionFactory::process_calendar_exceptions`]（MPP12 / MPP14 的 `process_calendar_exceptions` 直接转发到它） |
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `byte[] data` | `&[u8]` | 同全包约定 |
//! | `ByteArrayHelper.getShort/getInt` | `get_short_safe` / `get_int_safe` | Java 越界抛 `ArrayIndexOutOfBoundsException`，Rust 返回 0（不 panic） |
//! | `RecurrenceType` 为 `null` 时的 `switch` | `match` 的 `None` 分支 | Java 的 `switch(null)` 抛 `NullPointerException`；Rust 侧把「递归类型值非法」视作「按普通例外处理」并返回 `None` |
//! | `DayOfWeekHelper.getInstance(...)` 返回 `null` | `Option<Weekday>` | 为 `None` 时跳过 `setDayOfWeek`（Java 会把 `null` 存进 `RecurringData`） |
//! | `LocalTime.plus(duration, MILLIS)` | [`plus_milliseconds`] | 复刻 Java 的按天回绕语义 |
//! | 例外名称长度为负时 `offset += nameLength` | 负值按 0 处理 | Java 会把偏移量往回转（可能死循环），Rust 侧只允许向前推进 |
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
//!
//! 跳过的 Java 成员：无。

use std::cell::RefCell;
use std::rc::Rc;

use chrono::{NaiveDate, Weekday};

use crate::common::day_of_week_helper::DayOfWeekHelper;
use crate::common::number_helper::NumberHelper;
use crate::day_type::DayType;
use crate::local_date_range::LocalDateRange;
use crate::local_time_range::LocalTimeRange;
use crate::mpp::abstract_calendar_factory::{
    get_int_safe, get_short_safe, plus_milliseconds, AbstractCalendarFactory,
    AbstractCalendarFactoryMethods,
};
use crate::mpp::document_input_stream_factory::DocumentInputStreamFactory;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::ole::DirectoryEntry;
use crate::mpp::props::Props;
use crate::mpxj_exception::MpxjResult;
use crate::project_calendar::ProjectCalendar;
use crate::project_calendar_week::ProjectCalendarWeek;
use crate::project_file::ProjectFile;
use crate::recurrence_type::RecurrenceType;
use crate::recurring_data::RecurringData;

/// 例外日期数据的起始偏移（前 420 字节为工作时段数据）。
///
/// 对应 Java: `int offset = 420; // The first 420 is for the working hours data`
const EXCEPTION_DATA_OFFSET: usize = 420;

/// 单条例外日期的固定长度。
///
/// 对应 Java 注释：`Each exception is a 92 byte block, followed by a variable length text block`
const EXCEPTION_BLOCK_SIZE: usize = 92;

/// 一个工作周块的固定长度：7 × 60 + 2 + 2 + 8 + 4。
///
/// 对应 Java: `data.length >= offset + ((7 * 60) + 2 + 2 + 8 + 4)`
const WORK_WEEK_BLOCK_SIZE: usize = (7 * 60) + 2 + 2 + 8 + 4;

/// 递归类型映射表（下标为 MPP 文件中的整数值）。
///
/// 对应 Java: `private static final RecurrenceType[] RECURRENCE_TYPES`
const RECURRENCE_TYPES: [Option<RecurrenceType>; 8] = [
    None,
    Some(RecurrenceType::Daily),
    Some(RecurrenceType::Yearly), // Absolute
    Some(RecurrenceType::Yearly), // Relative
    Some(RecurrenceType::Monthly), // Absolute
    Some(RecurrenceType::Monthly), // Relative
    Some(RecurrenceType::Weekly),
    Some(RecurrenceType::Daily),
];

/// 「是否为相对递归」映射表（下标为 MPP 文件中的整数值）。
///
/// 对应 Java: `private static final boolean[] RELATIVE_MAP`
const RELATIVE_MAP: [bool; 6] = [false, false, false, true, false, true];

/// 星期的位掩码（下标为 `DayOfWeekHelper::get_value` 的取值）。
///
/// 对应 Java: `private static final int[] DAY_MASKS`
const DAY_MASKS: [i32; 8] = [
    0x00, 0x01, // Sunday
    0x02, // Monday
    0x04, // Tuesday
    0x08, // Wednesday
    0x10, // Thursday
    0x20, // Friday
    0x40, // Saturday
];

/// 从 MPP 文件读取日历数据（含例外日期与工作周）。
///
/// 对应 Java: `abstract class AbstractCalendarAndExceptionFactory extends AbstractCalendarFactory`
pub struct AbstractCalendarAndExceptionFactory {
    /// 基类部分（Java 里本类就是 `AbstractCalendarFactory`）。
    base: AbstractCalendarFactory,
}

impl AbstractCalendarAndExceptionFactory {
    /// 构造方法。
    ///
    /// 对应 Java: `public AbstractCalendarAndExceptionFactory(ProjectFile file)`
    pub fn new(file: Rc<RefCell<ProjectFile>>) -> Self {
        Self {
            base: AbstractCalendarFactory::new(file),
        }
    }

    /// 取组合的基类部分。
    ///
    /// Rust 特有：Java 通过继承直接使用基类成员，Rust 侧用组合故需显式取用。
    pub fn get_base(&self) -> &AbstractCalendarFactory {
        &self.base
    }

    /// 日历数据块的结构：4 字节块头 + 7 × 60 字节（一周七天）+（MPP12 起）例外日期块。
    ///
    /// 对应 Java: `AbstractCalendarFactory#processCalendarData(...)`（本类未覆写，直接继承）
    ///
    /// 转发给组合的基类实现，并把 `factory`（最终子类）传入以完成抽象方法的动态分派。
    pub fn process_calendar_data(
        &self,
        factory: &dyn AbstractCalendarFactoryMethods,
        project_dir: &DirectoryEntry,
        project_props: &Props,
        input_stream_factory: &DocumentInputStreamFactory,
        resource_map: &mut std::collections::HashMap<i32, Rc<RefCell<ProjectCalendar>>>,
    ) -> MpxjResult<()> {
        self.base.process_calendar_data(
            factory,
            project_dir,
            project_props,
            input_stream_factory,
            resource_map,
        )
    }

    /// 解析日历数据块中的例外日期（含重复例外与工作周）。
    ///
    /// 对应 Java: `@Override protected void processCalendarExceptions(byte[] data, ProjectCalendar cal)`
    pub fn process_calendar_exceptions(&self, data: &[u8], cal: &Rc<RefCell<ProjectCalendar>>) {
        // Java: if (data.length > 420)
        if data.len() <= EXCEPTION_DATA_OFFSET {
            return;
        }

        // Java: int offset = 420;
        let mut offset = EXCEPTION_DATA_OFFSET;
        // Java: int exceptionCount = ByteArrayHelper.getShort(data, offset);
        // （Java 在 data.length == 421 时越界抛异常；Rust 侧返回 0）
        let exception_count = get_short_safe(data, offset);

        {
            let Ok(mut cal_ref) = cal.try_borrow_mut() else {
                return;
            };

            if exception_count == 0 {
                // Java: align with 8 byte boundary ready to read work weeks
                offset += 4;
            } else {
                //
                // Move to the start of the first exception
                //
                // Java: offset += 4;
                offset += 4;

                //
                // Each exception is a 92 byte block, followed by a
                // variable length text block
                //
                for _ in 0..exception_count {
                    // Java: if (offset + 92 > data.length) { break; }
                    if offset + EXCEPTION_BLOCK_SIZE > data.len() {
                        // Bail out if we don't have at least 92 bytes available
                        break;
                    }

                    // Java: LocalDate fromDate = MPPUtility.getDate(data, offset);
                    let from_date = MPPUtility::get_date(data, offset);
                    // Java: LocalDate toDate = MPPUtility.getDate(data, offset + 2);
                    let to_date = MPPUtility::get_date(data, offset + 2);
                    // Java: RecurringData rd = readRecurringData(data, offset, fromDate, toDate);
                    let recurring = self.read_recurring_data(data, offset, from_date, to_date);

                    let exception = match recurring {
                        // Java: exception = cal.addCalendarException(fromDate, toDate);
                        None => cal_ref.add_calendar_exception(from_date, to_date),
                        // Java: exception = cal.addCalendarException(rd);
                        Some(rd) => cal_ref.add_calendar_exception_recurring(rd),
                    };

                    // Java: int periodCount = ByteArrayHelper.getShort(data, offset + 14);
                    let period_count = get_short_safe(data, offset + 14);
                    if period_count != 0 {
                        let mut exception_period_index: i32 = 0;
                        while exception_period_index < period_count {
                            // Java: LocalTime start = MPPUtility.getTime(data, offset + 20 + (exceptionPeriodIndex * 2));
                            let start = MPPUtility::get_time(
                                data,
                                offset + 20 + (exception_period_index as usize * 2),
                            );
                            // Java: long duration = MPPUtility.getDuration(data, offset + 32 + (exceptionPeriodIndex * 4));
                            let duration = MPPUtility::get_duration(
                                data,
                                offset + 32 + (exception_period_index as usize * 4),
                            );
                            // Java: LocalTime end = start.plus(duration, ChronoUnit.MILLIS);
                            let end = plus_milliseconds(start, duration);
                            // Java: exception.add(new LocalTimeRange(start, end));
                            exception.add(LocalTimeRange::new(Some(start), Some(end)));
                            exception_period_index += 1;
                        }
                    }

                    //
                    // Extract the name length - ensure that it is aligned to a 4 byte boundary
                    //
                    // Java: int exceptionNameLength = ByteArrayHelper.getInt(data, offset + 88);
                    let mut exception_name_length = get_int_safe(data, offset + 88);
                    if exception_name_length % 4 != 0 {
                        exception_name_length = ((exception_name_length / 4) + 1) * 4;
                    }

                    if exception_name_length != 0 {
                        // Java: exception.setName(MPPUtility.getUnicodeString(data, offset + 92));
                        exception
                            .set_name(Some(MPPUtility::get_unicode_string(data, offset + 92)));
                    }

                    // Java: offset += (92 + exceptionNameLength);
                    // （Java 的长度为负时会回退偏移量，Rust 侧只允许向前推进，避免死循环）
                    offset += EXCEPTION_BLOCK_SIZE + exception_name_length.max(0) as usize;
                }
            }
        }

        // Java: processWorkWeeks(data, offset, cal);
        self.process_work_weeks(data, offset, cal);
    }

    /// 解析重复例外的递归规则；不构成重复例外时返回 `None`（Java 的 `null`）。
    ///
    /// 对应 Java: `private RecurringData readRecurringData(byte[] data, int offset, LocalDate fromDate, LocalDate toDate)`
    fn read_recurring_data(
        &self,
        data: &[u8],
        offset: usize,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
    ) -> Option<RecurringData> {
        let mut rd = RecurringData::new();
        // Java: int recurrenceTypeValue = ByteArrayHelper.getShort(data, offset + 72);
        let recurrence_type_value = get_short_safe(data, offset + 72);

        // Java: rd.setStartDate(fromDate); rd.setFinishDate(toDate);
        rd.set_start_date(from_date);
        rd.set_finish_date(to_date);
        // Java: rd.setRecurrenceType(getRecurrenceType(recurrenceTypeValue));
        rd.set_recurrence_type(self.get_recurrence_type(recurrence_type_value));
        // Java: rd.setRelative(getRelative(recurrenceTypeValue));
        rd.set_relative(self.get_relative(recurrence_type_value));
        // Java: rd.setOccurrences(Integer.valueOf(ByteArrayHelper.getShort(data, offset + 4)));
        rd.set_occurrences(Some(get_short_safe(data, offset + 4)));

        // Java: switch (rd.getRecurrenceType())
        match rd.get_recurrence_type() {
            Some(RecurrenceType::Daily) => {
                // Java: if (recurrenceTypeValue == 1) { frequency = 1; } else { frequency = ByteArrayHelper.getShort(data, offset + 76); }
                let frequency = if recurrence_type_value == 1 {
                    1
                } else {
                    get_short_safe(data, offset + 76)
                };
                rd.set_frequency(Some(frequency));
            }

            Some(RecurrenceType::Weekly) => {
                // Java: rd.setWeeklyDaysFromBitmap(Integer.valueOf(MPPUtility.getByte(data, offset + 76)), DAY_MASKS);
                rd.set_weekly_days_from_bitmap(
                    Some(MPPUtility::get_byte(data, offset + 76)),
                    &DAY_MASKS,
                );
                // Java: rd.setFrequency(Integer.valueOf(ByteArrayHelper.getShort(data, offset + 78)));
                rd.set_frequency(Some(get_short_safe(data, offset + 78)));
            }

            Some(RecurrenceType::Monthly) => {
                if rd.get_relative() {
                    // Java: rd.setDayOfWeek(DayOfWeekHelper.getInstance(MPPUtility.getByte(data, offset + 77) - 2));
                    if let Some(day) =
                        DayOfWeekHelper::get_instance(MPPUtility::get_byte(data, offset + 77) - 2)
                    {
                        rd.set_day_of_week(day);
                    }
                    // Java: rd.setDayNumber(Integer.valueOf(MPPUtility.getByte(data, offset + 76) + 1));
                    rd.set_day_number(Some(MPPUtility::get_byte(data, offset + 76) + 1));
                    // Java: rd.setFrequency(Integer.valueOf(ByteArrayHelper.getShort(data, offset + 78)));
                    rd.set_frequency(Some(get_short_safe(data, offset + 78)));
                } else {
                    // Java: rd.setDayNumber(Integer.valueOf(MPPUtility.getByte(data, offset + 76)));
                    rd.set_day_number(Some(MPPUtility::get_byte(data, offset + 76)));
                    // Java: rd.setFrequency(Integer.valueOf(MPPUtility.getByte(data, offset + 78)));
                    rd.set_frequency(Some(MPPUtility::get_byte(data, offset + 78)));
                }
            }

            Some(RecurrenceType::Yearly) => {
                if rd.get_relative() {
                    // Java: rd.setDayOfWeek(DayOfWeekHelper.getInstance(MPPUtility.getByte(data, offset + 78) - 2));
                    if let Some(day) =
                        DayOfWeekHelper::get_instance(MPPUtility::get_byte(data, offset + 78) - 2)
                    {
                        rd.set_day_of_week(day);
                    }
                    // Java: rd.setDayNumber(Integer.valueOf(MPPUtility.getByte(data, offset + 77) + 1));
                    rd.set_day_number(Some(MPPUtility::get_byte(data, offset + 77) + 1));
                } else {
                    // Java: rd.setDayNumber(Integer.valueOf(MPPUtility.getByte(data, offset + 77)));
                    rd.set_day_number(Some(MPPUtility::get_byte(data, offset + 77)));
                }
                // Java: rd.setMonthNumber(Integer.valueOf(MPPUtility.getByte(data, offset + 76) + 1));
                rd.set_month_number(Some(MPPUtility::get_byte(data, offset + 76) + 1));
            }

            None => {
                // Java 的 switch(null) 抛 NullPointerException；Rust 侧把非法递归类型
                // 视作「不构成重复例外」，交由调用方按普通例外处理。
                return None;
            }
        }

        //
        // Flatten daily recurring exceptions if they only result in one date range.
        // Flatten exception if it doesn't generate any dates.
        //
        // Java: if (rd.getRecurrenceType() == RecurrenceType.DAILY && NumberHelper.getInt(rd.getFrequency()) == 1) { rd = null; }
        if rd.get_recurrence_type() == Some(RecurrenceType::Daily)
            && NumberHelper::get_int(rd.get_frequency().map(f64::from)) == 1
        {
            return None;
        }

        Some(rd)
    }

    /// 把整数值映射为递归类型；非法值返回 `None`（Java 的 `null`）。
    ///
    /// 对应 Java: `private RecurrenceType getRecurrenceType(int value)`
    fn get_recurrence_type(&self, value: i32) -> Option<RecurrenceType> {
        if value < 0 || value as usize >= RECURRENCE_TYPES.len() {
            return None;
        }

        RECURRENCE_TYPES[value as usize]
    }

    /// 判断递归是否为「相对」；非法值返回 `false`。
    ///
    /// 对应 Java: `private boolean getRelative(int value)`
    fn get_relative(&self, value: i32) -> bool {
        if value < 0 || value as usize >= RELATIVE_MAP.len() {
            return false;
        }

        RELATIVE_MAP[value as usize]
    }

    /// 读取工作周定义。
    ///
    /// 对应 Java: `private void processWorkWeeks(byte[] data, int offset, ProjectCalendar cal)`
    fn process_work_weeks(&self, data: &[u8], offset: usize, cal: &Rc<RefCell<ProjectCalendar>>) {
        // Java: skip 4 byte header
        let mut offset = offset + 4;

        let Ok(mut cal_ref) = cal.try_borrow_mut() else {
            return;
        };

        // Java: while (data.length >= offset + ((7 * 60) + 2 + 2 + 8 + 4))
        while offset.saturating_add(WORK_WEEK_BLOCK_SIZE) <= data.len() {
            // Java: ProjectCalendarWeek week = cal.addWorkWeek();
            let week = cal_ref.add_work_week();

            for day in DayOfWeekHelper::ORDERED_DAYS {
                // 60 byte block per day
                self.process_work_week_day(data, offset, week, day);
                offset += 60;
            }

            // Java: LocalDate startDate = MPPUtility.getDate(data, offset); offset += 2;
            let start_date = MPPUtility::get_date(data, offset);
            offset += 2;

            // Java: LocalDate finishDate = MPPUtility.getDate(data, offset); offset += 2;
            let finish_date = MPPUtility::get_date(data, offset);
            offset += 2;

            // skip unknown 8 bytes
            offset += 8;

            //
            // Extract the name length - ensure that it is aligned to a 4 byte boundary
            //
            // Java: int nameLength = ByteArrayHelper.getInt(data, offset);
            let mut name_length = get_int_safe(data, offset);
            if name_length % 4 != 0 {
                name_length = ((name_length / 4) + 1) * 4;
            }
            offset += 4;

            if name_length != 0 {
                // Java: String name = MPPUtility.getUnicodeString(data, offset, nameLength);
                //       offset += nameLength; week.setName(name);
                // （Java 的长度为负时会回退偏移量，Rust 侧只允许向前推进）
                if name_length > 0 {
                    let name = MPPUtility::get_unicode_string_with_max_length(
                        data,
                        offset,
                        name_length as usize,
                    );
                    offset += name_length as usize;
                    week.set_name(Some(name));
                }
            }

            // Java: week.setDateRange(new LocalDateRange(startDate, finishDate));
            week.set_date_range(Some(LocalDateRange::new(start_date, finish_date)));
        }
    }

    /// 读取工作周中的某一天。
    ///
    /// 对应 Java: `private void processWorkWeekDay(byte[] data, int offset, ProjectCalendarWeek week, DayOfWeek day)`
    fn process_work_week_day(
        &self,
        data: &[u8],
        offset: usize,
        week: &mut ProjectCalendarWeek,
        day: Weekday,
    ) {
        // Java: int dayType = ByteArrayHelper.getShort(data, offset);
        let day_type = get_short_safe(data, offset);

        if day_type == 1 {
            // Java: week.setCalendarDayType(day, DayType.DEFAULT);
            week.set_calendar_day_type(day, DayType::Default);
        } else {
            // Java: ProjectCalendarHours hours = week.addCalendarHours(day);
            //       int rangeCount = ByteArrayHelper.getShort(data, offset + 2);
            let range_count = get_short_safe(data, offset + 2);

            if range_count == 0 {
                // Java: week.setCalendarDayType(day, DayType.NON_WORKING);
                //（Java 先 addCalendarHours 再设置类型；Rust 侧顺序相反但结果一致：
                //  `DayType::NonWorking` 保证该天存在一个空的工作时段集合）
                week.set_calendar_day_type(day, DayType::NonWorking);
                let _ = week.add_calendar_hours(day);
            } else {
                // Java: week.setCalendarDayType(day, DayType.WORKING);
                week.set_calendar_day_type(day, DayType::Working);

                // Java: for (int index = 0; index < rangeCount; index++) { ... hours.add(...); }
                let mut ranges: Vec<LocalTimeRange> = Vec::new();
                let mut index: i32 = 0;
                while index < range_count {
                    // Java: LocalTime startTime = MPPUtility.getTime(data, offset + 8 + (index * 2));
                    let start_time =
                        MPPUtility::get_time(data, offset + 8 + (index as usize * 2));
                    // Java: int durationInSeconds = ByteArrayHelper.getInt(data, offset + 20 + (index * 4)) * 6;
                    // （Java 的 int 乘法会回绕，Rust 侧用 wrapping_mul 保持语义）
                    let duration_in_seconds =
                        get_int_safe(data, offset + 20 + (index as usize * 4)).wrapping_mul(6);
                    // Java: LocalTime finishTime = startTime.plusSeconds(durationInSeconds);
                    let finish_time =
                        plus_milliseconds(start_time, i64::from(duration_in_seconds) * 1_000);
                    ranges.push(LocalTimeRange::new(Some(start_time), Some(finish_time)));
                    index += 1;
                }

                let hours = week.add_calendar_hours(day);
                for range in ranges {
                    hours.add(range);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    /// 1983-12-31（MPP 日期的纪元）。
    fn epoch() -> NaiveDate {
        NaiveDate::from_ymd_opt(1983, 12, 31).expect("合法日期")
    }

    /// 把日期转成 MPP 的「自纪元起的天数」。
    fn date_days(year: i32, month: u32, day: u32) -> u16 {
        let date = NaiveDate::from_ymd_opt(year, month, day).expect("合法日期");
        (date - epoch()).num_days() as u16
    }

    /// 时间值的「十分之一分钟」编码。
    fn tenths_of_minute(hour: u32, minute: u32) -> u16 {
        ((hour * 60 + minute) * 10) as u16
    }

    fn time(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).expect("合法时间")
    }

    fn calendar() -> Rc<RefCell<ProjectCalendar>> {
        Rc::new(RefCell::new(ProjectCalendar::new(
            std::rc::Weak::new(),
            false,
        )))
    }

    fn factory() -> AbstractCalendarAndExceptionFactory {
        AbstractCalendarAndExceptionFactory::new(ProjectFile::new())
    }

    #[test]
    fn data_without_exceptions_or_work_weeks_is_ignored() {
        let cal = calendar();

        // 长度 421：只够读到 exceptionCount 的第一个字节（Java 会越界抛异常）
        factory().process_calendar_exceptions(&vec![0u8; 421], &cal);
        assert_eq!(cal.borrow_mut().get_calendar_exceptions().len(), 0);
        assert_eq!(cal.borrow().get_work_weeks().len(), 0);

        // 长度 420：Java 的 `data.length > 420` 为假，直接跳过
        factory().process_calendar_exceptions(&vec![0u8; 420], &cal);
        assert_eq!(cal.borrow().get_work_weeks().len(), 0);
    }

    #[test]
    fn single_day_exception_is_parsed() {
        // 420 字节工作时段 + exceptionCount(2) + 4 字节对齐 + 92 字节例外块
        let mut data = vec![0u8; 424 + EXCEPTION_BLOCK_SIZE];
        data[EXCEPTION_DATA_OFFSET..EXCEPTION_DATA_OFFSET + 2]
            .copy_from_slice(&1u16.to_le_bytes());

        let exception = &mut data[424..424 + EXCEPTION_BLOCK_SIZE];
        // fromDate / toDate：2024-01-01
        exception[0..2].copy_from_slice(&date_days(2024, 1, 1).to_le_bytes());
        exception[2..4].copy_from_slice(&date_days(2024, 1, 1).to_le_bytes());
        // occurrences
        exception[4..6].copy_from_slice(&1u16.to_le_bytes());
        // periodCount = 1
        exception[14..16].copy_from_slice(&1u16.to_le_bytes());
        // 时段：09:00 起，时长 2 小时（以十分之一分钟计）
        exception[20..22].copy_from_slice(&tenths_of_minute(9, 0).to_le_bytes());
        exception[32..36].copy_from_slice(&1200u32.to_le_bytes());
        // 递归类型 = 1（DAILY，频率固定为 1 → 扁平化为普通例外）
        exception[72..74].copy_from_slice(&1u16.to_le_bytes());
        // 名称长度 = 0
        exception[88..92].copy_from_slice(&0u32.to_le_bytes());

        let cal = calendar();
        factory().process_calendar_exceptions(&data, &cal);

        let mut cal_ref = cal.borrow_mut();
        let exceptions = cal_ref.get_calendar_exceptions();
        assert_eq!(exceptions.len(), 1);

        let parsed = &exceptions[0];
        assert_eq!(
            parsed.get_from_date(),
            NaiveDate::from_ymd_opt(2024, 1, 1)
        );
        assert_eq!(parsed.get_to_date(), NaiveDate::from_ymd_opt(2024, 1, 1));
        assert!(parsed.get_recurring().is_none());

        let hours = parsed.get_hours();
        assert_eq!(hours.size(), 1);
        assert_eq!(
            hours.get(0),
            LocalTimeRange::new(Some(time(9, 0)), Some(time(11, 0)))
        );
    }

    #[test]
    fn recurring_exception_keeps_its_recurrence() {
        let mut data = vec![0u8; 424 + EXCEPTION_BLOCK_SIZE];
        data[EXCEPTION_DATA_OFFSET..EXCEPTION_DATA_OFFSET + 2]
            .copy_from_slice(&1u16.to_le_bytes());

        let exception = &mut data[424..424 + EXCEPTION_BLOCK_SIZE];
        exception[0..2].copy_from_slice(&date_days(2024, 1, 1).to_le_bytes());
        exception[2..4].copy_from_slice(&date_days(2024, 3, 31).to_le_bytes());
        // periodCount = 0
        // 递归类型 = 6（WEEKLY）
        exception[72..74].copy_from_slice(&6u16.to_le_bytes());
        // 按周重复的星期位图：周日 + 周一
        exception[76] = 0x03;
        // 频率 = 2
        exception[78..80].copy_from_slice(&2u16.to_le_bytes());
        exception[88..92].copy_from_slice(&0u32.to_le_bytes());

        let cal = calendar();
        factory().process_calendar_exceptions(&data, &cal);

        let mut cal_ref = cal.borrow_mut();
        let exceptions = cal_ref.get_calendar_exceptions();
        assert_eq!(exceptions.len(), 1);

        let recurring = exceptions[0].get_recurring().expect("应为重复例外");
        assert_eq!(recurring.get_recurrence_type(), Some(RecurrenceType::Weekly));
        assert_eq!(recurring.get_frequency(), Some(2));
        assert!(recurring.get_weekly_day(Weekday::Sun));
        assert!(recurring.get_weekly_day(Weekday::Mon));
        assert!(!recurring.get_weekly_day(Weekday::Tue));
    }

    #[test]
    fn exception_with_a_name_is_parsed() {
        // 名称 "AB"（UTF-16LE，含结尾 NUL）共 6 字节 → 对齐为 8 字节
        let name: Vec<u8> = vec![0x41, 0x00, 0x42, 0x00, 0x00, 0x00, 0x00, 0x00];
        let mut data = vec![0u8; 424 + EXCEPTION_BLOCK_SIZE + name.len()];
        data[EXCEPTION_DATA_OFFSET..EXCEPTION_DATA_OFFSET + 2]
            .copy_from_slice(&1u16.to_le_bytes());

        let exception = &mut data[424..424 + EXCEPTION_BLOCK_SIZE];
        exception[0..2].copy_from_slice(&date_days(2024, 5, 1).to_le_bytes());
        exception[2..4].copy_from_slice(&date_days(2024, 5, 1).to_le_bytes());
        exception[72..74].copy_from_slice(&1u16.to_le_bytes());
        // 名称长度（未对齐的原始值）
        exception[88..92].copy_from_slice(&6u32.to_le_bytes());
        data[424 + EXCEPTION_BLOCK_SIZE..].copy_from_slice(&name);

        let cal = calendar();
        factory().process_calendar_exceptions(&data, &cal);

        let mut cal_ref = cal.borrow_mut();
        let exceptions = cal_ref.get_calendar_exceptions();
        assert_eq!(exceptions.len(), 1);
        assert_eq!(exceptions[0].get_name(), Some("AB"));
    }

    #[test]
    fn work_week_is_parsed() {
        // 420 字节工作时段 + exceptionCount = 0 + 4 字节对齐 + 1 个工作周块
        let week_start = 428usize;
        let mut data = vec![0u8; week_start + WORK_WEEK_BLOCK_SIZE];

        //
        // 一周七天（顺序：周日 → 周六），每天 60 字节
        //
        for day_index in 0..7 {
            let day_offset = week_start + day_index * 60;
            if day_index == 1 {
                // 周一：1 个时段 08:00-12:00
                data[day_offset] = 0; // dayType != 1
                data[day_offset + 2..day_offset + 4].copy_from_slice(&1u16.to_le_bytes());
                data[day_offset + 8..day_offset + 10]
                    .copy_from_slice(&tenths_of_minute(8, 0).to_le_bytes());
                data[day_offset + 20..day_offset + 24]
                    .copy_from_slice(&2400u32.to_le_bytes());
            } else {
                // 其余天：dayType == 1 → DayType::DEFAULT
                data[day_offset] = 1;
            }
        }

        // 起始 / 结束日期
        let start = week_start + 420;
        data[start..start + 2].copy_from_slice(&date_days(2024, 1, 1).to_le_bytes());
        data[start + 2..start + 4].copy_from_slice(&date_days(2024, 1, 7).to_le_bytes());
        // 8 字节未知数据 + 名称长度 0
        data[start + 12..start + 16].copy_from_slice(&0u32.to_le_bytes());

        let cal = calendar();
        factory().process_calendar_exceptions(&data, &cal);

        let cal_ref = cal.borrow();
        let weeks = cal_ref.get_work_weeks();
        assert_eq!(weeks.len(), 1);

        let week = &weeks[0];
        assert_eq!(
            week.get_date_range(),
            Some(LocalDateRange::new(
                NaiveDate::from_ymd_opt(2024, 1, 1),
                NaiveDate::from_ymd_opt(2024, 1, 7)
            ))
        );
        assert_eq!(week.get_name(), None);
        assert_eq!(
            week.get_calendar_day_type(Weekday::Sun),
            Some(DayType::Default)
        );
        assert_eq!(
            week.get_calendar_day_type(Weekday::Mon),
            Some(DayType::Working)
        );

        let monday = week.get_calendar_hours_for_day(Weekday::Mon).expect("周一工作时段");
        assert_eq!(monday.size(), 1);
        assert_eq!(
            monday.get(0),
            LocalTimeRange::new(Some(time(8, 0)), Some(time(12, 0)))
        );
    }

    #[test]
    fn truncated_work_week_block_is_skipped() {
        // 差一个字节不够一个完整的工作周块
        let data = vec![0u8; 428 + WORK_WEEK_BLOCK_SIZE - 1];
        let cal = calendar();
        factory().process_calendar_exceptions(&data, &cal);

        assert_eq!(cal.borrow().get_work_weeks().len(), 0);
    }

    #[test]
    fn invalid_recurrence_type_values_map_to_none() {
        let factory = factory();
        assert_eq!(factory.get_recurrence_type(0), None);
        assert_eq!(factory.get_recurrence_type(1), Some(RecurrenceType::Daily));
        assert_eq!(factory.get_recurrence_type(8), None);
        assert_eq!(factory.get_recurrence_type(-1), None);

        assert!(!factory.get_relative(0));
        assert!(factory.get_relative(3));
        assert!(!factory.get_relative(6));
    }
}
