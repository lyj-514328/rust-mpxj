//! 对应 Java: `org/mpxj/mpx/RecurrenceUtility.java`
//!
//! # 移植取舍（`RecurringTask` 接口）
//!
//! Java 的 `getDurationUnits(RecurringTask)` 与 `getDays(RecurringTask)` 接收
//! `org.mpxj.RecurringTask`（由 `Task` 实现，提供 `getDuration()` 与 `getWeeklyDay()`）。
//! 本 crate 未移植 `RecurringTask`（见 `task`、`recurring_data` 文件头的说明），因此：
//!
//! - `getDurationUnits` 改为直接接收工期（`Option<Duration>`），对应 Java 中
//!   `recurrence.getDuration()` 的取值结果；
//! - `getDays` 改为接收 [`RecurringData`]，其 `get_weekly_day` 提供与 Java
//!   `RecurringTask.getWeeklyDay()` 相同的信息。
//!
//! 其余方法均与 Java 逐一对应；Java 抛异常的分支（`NumberFormatException`、
//! `DateTimeException`、`ArrayIndexOutOfBoundsException`）在 Rust 侧改为返回 `None`。
//!
//! 未移植的成员：无（含常量 `RECURRING_TASK_DAY_MASKS` 与全部私有静态映射表）。

use chrono::{Datelike, NaiveDate, Weekday};

use crate::common::day_of_week_helper::DayOfWeekHelper;
use crate::duration::Duration;
use crate::project_properties::ProjectProperties;
use crate::recurrence_type::RecurrenceType;
use crate::recurring_data::RecurringData;
use crate::time_unit::TimeUnit;

/// 管理 MPX 文件中重复（周期）任务相关数据的工具方法。
///
/// 对应 Java: `final class RecurrenceUtility`（私有构造方法禁止实例化，Rust 侧用无字段类型表达）。
pub struct RecurrenceUtility;

impl RecurrenceUtility {
    /// MPX 记录中日期位图与 `DayOfWeekHelper.getValue`（周日 = 1 … 周六 = 7）对应的掩码表。
    ///
    /// 对应 Java: `public static final int[] RECURRING_TASK_DAY_MASKS`
    pub const RECURRING_TASK_DAY_MASKS: [i32; 8] = [
        0x00,
        0x40, // Sunday
        0x20, // Monday
        0x10, // Tuesday
        0x08, // Wednesday
        0x04, // Thursday
        0x02, // Friday
        0x01, // Saturday
    ];

    /// 把 MPX 记录里的「工期值 + 单位整数」转换为 `Duration` 实例。
    ///
    /// 对应 Java: `public static Duration getDuration(ProjectProperties properties, Integer durationValue, Integer unitsValue)`
    ///
    /// 工期在 MPX 记录中以分钟表示，单位由 `unitsValue` 指定；`null` 工期返回 `None`。
    pub fn get_duration(
        properties: &ProjectProperties,
        duration_value: Option<i32>,
        units_value: Option<i32>,
    ) -> Option<Duration> {
        // Java: durationValue == null -> result = null
        let duration_value = duration_value?;

        let result = Duration::get_instance_int(duration_value, TimeUnit::Minutes);
        let units = Self::get_duration_units_from_value(units_value);

        if result.get_units() != units {
            Some(result.convert_units(units, properties))
        } else {
            Some(result)
        }
    }

    /// 把 `Duration` 实例转换为可以直接写入 MPX 文件的整数分钟值。
    ///
    /// 对应 Java: `public static Integer getDurationValue(ProjectProperties properties, Duration duration)`
    pub fn get_duration_value(
        properties: &ProjectProperties,
        duration: Option<Duration>,
    ) -> Option<i32> {
        // Java: duration == null -> result = null
        let duration = duration?;

        let duration = if duration.get_units() != TimeUnit::Minutes {
            duration.convert_units(TimeUnit::Minutes, properties)
        } else {
            duration
        };

        // Java: Integer.valueOf((int) duration.getDuration())
        Some(duration.get_duration() as i32)
    }

    /// 把工期单位转换为适合写入 MPX 文件的整数值。
    ///
    /// 对应 Java: `public static Integer getDurationUnits(RecurringTask recurrence)`
    ///
    /// 取舍见文件头：Java 从 `RecurringTask.getDuration()` 取工期，Rust 侧直接接收工期。
    /// 单位不在私有映射表 `DURATION_UNITS`（天 / 周 / 小时 / 分钟）中时返回 `None`。
    pub fn get_duration_units(duration: Option<Duration>) -> Option<i32> {
        let duration = duration?;
        duration_units_value(duration.get_units())
    }

    /// 把 MPX 记录中重复任务的单位整数转换为 `TimeUnit`，无法识别时回落为「天」。
    ///
    /// 对应 Java: `private static TimeUnit getDurationUnits(Integer value)`
    ///
    /// Java 侧与 [`RecurrenceUtility::get_duration_units`] 构成重载，Rust 无法重载，故加后缀区分。
    fn get_duration_units_from_value(value: Option<i32>) -> TimeUnit {
        let result = value
            .and_then(|index| usize::try_from(index).ok())
            .and_then(|index| DURATION_UNITS.get(index).copied());

        // Java: result == null -> TimeUnit.DAYS
        result.unwrap_or(TimeUnit::Days)
    }

    /// 把 MPX 文件中的重复类型整数转换为 `RecurrenceType`，无法识别时返回 `None`。
    ///
    /// 对应 Java: `public static RecurrenceType getRecurrenceType(Integer value)`
    pub fn get_recurrence_type(value: Option<i32>) -> Option<RecurrenceType> {
        match value? {
            1 => Some(RecurrenceType::Daily),
            4 => Some(RecurrenceType::Weekly),
            8 => Some(RecurrenceType::Monthly),
            16 => Some(RecurrenceType::Yearly),
            _ => None,
        }
    }

    /// 把 `RecurrenceType` 转换为 MPX 文件中的重复类型整数，`None` 返回 `None`。
    ///
    /// 对应 Java: `public static Integer getRecurrenceValue(RecurrenceType value)`
    pub fn get_recurrence_value(value: Option<RecurrenceType>) -> Option<i32> {
        match value? {
            RecurrenceType::Daily => Some(1),
            RecurrenceType::Weekly => Some(4),
            RecurrenceType::Monthly => Some(8),
            RecurrenceType::Yearly => Some(16),
        }
    }

    /// 把「星期几」位图的字符串形式转换为整数位图。
    ///
    /// 对应 Java: `public static Integer getDays(String days)`
    ///
    /// 首字符为最高位（`Integer.parseInt(days, 2)` 的语义）。Java 对非法输入抛
    /// `NumberFormatException`，Rust 侧返回 `None`。
    pub fn get_days_from_string(days: Option<&str>) -> Option<i32> {
        days.and_then(|days| i32::from_str_radix(days, 2).ok())
    }

    /// 把按周重复的星期几转换为位图字符串。
    ///
    /// 对应 Java: `public static String getDays(RecurringTask task)`
    ///
    /// 顺序取自 [`DayOfWeekHelper::ORDERED_DAYS`]（周日 → 周六），即首字符为周日。
    /// 取舍见文件头：Java 从 `RecurringTask.getWeeklyDay()` 取值，Rust 侧接收 [`RecurringData`]。
    pub fn get_days(data: &RecurringData) -> String {
        let mut result = String::new();
        for day in DayOfWeekHelper::ORDERED_DAYS {
            result.push(if data.get_weekly_day(day) { '1' } else { '0' });
        }
        result
    }

    /// 把 MPX 的「星期几」序号转换为 [`Weekday`]，序号 0（Java 侧为 `null`）返回 `None`。
    ///
    /// 对应 Java: `public static DayOfWeek getDay(Integer day)`
    ///
    /// Java 对越界序号抛 `ArrayIndexOutOfBoundsException`，Rust 侧返回 `None`。
    pub fn get_day(day: Option<i32>) -> Option<Weekday> {
        let index = usize::try_from(day?).ok()?;
        DAY_ARRAY.get(index).copied().flatten()
    }

    /// 把 [`Weekday`] 转换为 MPX 的「星期几」序号（周一 = 1 … 周日 = 7）。
    ///
    /// 对应 Java: `public static Integer getDay(DayOfWeek day)`
    ///
    /// Java 侧与 [`RecurrenceUtility::get_day`] 构成重载，Rust 无法重载，故加后缀区分。
    pub fn get_day_index(day: Option<Weekday>) -> Option<i32> {
        match day? {
            Weekday::Mon => Some(1),
            Weekday::Tue => Some(2),
            Weekday::Wed => Some(3),
            Weekday::Thu => Some(4),
            Weekday::Fri => Some(5),
            Weekday::Sat => Some(6),
            Weekday::Sun => Some(7),
        }
    }

    /// 取「按年、绝对日期」重复的日期（年份取自重复的起始日期）。
    ///
    /// 对应 Java: `public static LocalDate getYearlyAbsoluteAsDate(RecurringData data)`
    ///
    /// 缺少月/日/起始日期任一值时返回 `None`；Java 的 `LocalDate.of` 对非法日期抛
    /// `DateTimeException`，Rust 侧返回 `None`。
    pub fn get_yearly_absolute_as_date(data: &RecurringData) -> Option<NaiveDate> {
        let yearly_absolute_day = data.get_day_number()?;
        let yearly_absolute_month = data.get_month_number()?;
        let start_date = data.get_start_date()?;

        NaiveDate::from_ymd_opt(
            start_date.year(),
            u32::try_from(yearly_absolute_month).ok()?,
            u32::try_from(yearly_absolute_day).ok()?,
        )
    }
}

/// 重复任务记录中工期单位整数到 `TimeUnit` 的映射表。
///
/// 对应 Java: `private static final TimeUnit[] DURATION_UNITS`
const DURATION_UNITS: [TimeUnit; 4] = [
    TimeUnit::Days,
    TimeUnit::Weeks,
    TimeUnit::Hours,
    TimeUnit::Minutes,
];

/// `TimeUnit` 到重复任务记录中工期单位整数的反向映射。
///
/// 对应 Java: `private static final Map<TimeUnit, Integer> UNITS_MAP`（由 `DURATION_UNITS` 静态块建立）
fn duration_units_value(units: TimeUnit) -> Option<i32> {
    DURATION_UNITS
        .iter()
        .position(|candidate| *candidate == units)
        .map(|index| index as i32)
}

/// MPX 星期几序号到 [`Weekday`] 的映射表（下标 0 对应 Java 的 `null`）。
///
/// 对应 Java: `private static final DayOfWeek[] DAY_ARRAY`
const DAY_ARRAY: [Option<Weekday>; 8] = [
    None,
    Some(Weekday::Mon),
    Some(Weekday::Tue),
    Some(Weekday::Wed),
    Some(Weekday::Thu),
    Some(Weekday::Fri),
    Some(Weekday::Sat),
    Some(Weekday::Sun),
];

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::project_file::ProjectFile;

    /// 构造一个用于测试的项目属性：每天 480 分钟、每周 2400 分钟、每月 20 天。
    ///
    /// 时间单位默认值属于 Java 的「始终计算字段」，实际存放在父文件的项目上下文里，
    /// 因此必须保留父文件（弱引用），并把文件对象一并返回给调用方持有。
    fn properties() -> (Rc<RefCell<ProjectFile>>, ProjectProperties) {
        let file = ProjectFile::new();
        let mut properties = ProjectProperties::new(Rc::downgrade(&file));
        properties.set_minutes_per_day(Some(480));
        properties.set_minutes_per_week(Some(2400));
        properties.set_days_per_month(Some(20));
        (file, properties)
    }

    #[test]
    fn duration_round_trips_through_mpx_values() {
        let (_file, properties) = properties();
        // (MPX 单位整数, 期望单位)
        let cases = [
            (0, TimeUnit::Days),
            (1, TimeUnit::Weeks),
            (2, TimeUnit::Hours),
            (3, TimeUnit::Minutes),
        ];

        for (units_value, units) in cases {
            // MPX 值 → 模型值
            let duration =
                RecurrenceUtility::get_duration(&properties, Some(480), Some(units_value))
                    .expect("工期值非空");
            assert_eq!(duration.get_units(), units);

            // 模型值 → MPX 值 → 模型值
            let value = RecurrenceUtility::get_duration_value(&properties, Some(duration))
                .expect("工期非空");
            let units_index = RecurrenceUtility::get_duration_units(Some(duration))
                .expect("单位应可映射为 MPX 整数");
            assert_eq!(units_index, units_value);

            let round_tripped =
                RecurrenceUtility::get_duration(&properties, Some(value), Some(units_index))
                    .expect("工期值非空");
            assert_eq!(round_tripped.get_units(), units);
            assert_eq!(round_tripped, duration);
        }
    }

    #[test]
    fn duration_value_is_expressed_in_minutes() {
        let (_file, properties) = properties();
        assert_eq!(
            RecurrenceUtility::get_duration_value(
                &properties,
                Some(Duration::get_instance(1.0, TimeUnit::Days))
            ),
            Some(480)
        );
        assert_eq!(
            RecurrenceUtility::get_duration_value(
                &properties,
                Some(Duration::get_instance(2.0, TimeUnit::Hours))
            ),
            Some(120)
        );
        assert_eq!(
            RecurrenceUtility::get_duration_value(
                &properties,
                Some(Duration::get_instance(30.0, TimeUnit::Minutes))
            ),
            Some(30)
        );
    }

    #[test]
    fn null_duration_values_are_preserved() {
        let (_file, properties) = properties();
        assert_eq!(RecurrenceUtility::get_duration(&properties, None, Some(0)), None);
        assert_eq!(
            RecurrenceUtility::get_duration_value(&properties, None),
            None
        );
        assert_eq!(RecurrenceUtility::get_duration_units(None), None);
    }

    #[test]
    fn duration_units_default_to_days() {
        let (_file, properties) = properties();
        // 未知 / 越界 / 缺失的单位整数都回落为「天」（工期值本身仍会按分钟 → 天换算）
        for units_value in [None, Some(-1), Some(4), Some(100)] {
            let duration = RecurrenceUtility::get_duration(&properties, Some(1), units_value)
                .expect("工期值非空");
            assert_eq!(duration.get_units(), TimeUnit::Days);
            assert!(Duration::duration_value_equals(
                duration.get_duration(),
                1.0 / 480.0
            ));
        }
    }

    #[test]
    fn duration_units_without_mpx_representation_return_none() {
        // Java: UNITS_MAP 中只有天 / 周 / 小时 / 分钟
        for units in [
            TimeUnit::Months,
            TimeUnit::Years,
            TimeUnit::Percent,
            TimeUnit::ElapsedDays,
        ] {
            assert_eq!(
                RecurrenceUtility::get_duration_units(Some(Duration::get_instance(1.0, units))),
                None
            );
        }
    }

    #[test]
    fn recurrence_type_round_trips() {
        for recurrence_type in RecurrenceType::ALL {
            let value = RecurrenceUtility::get_recurrence_value(Some(recurrence_type))
                .expect("重复类型应可映射为 MPX 整数");
            assert_eq!(
                RecurrenceUtility::get_recurrence_type(Some(value)),
                Some(recurrence_type)
            );
        }
    }

    #[test]
    fn recurrence_type_rejects_unknown_values() {
        for value in [None, Some(0), Some(2), Some(17), Some(-1)] {
            assert_eq!(RecurrenceUtility::get_recurrence_type(value), None);
        }
        assert_eq!(RecurrenceUtility::get_recurrence_value(None), None);
    }

    #[test]
    fn days_bitmap_round_trips() {
        let mut data = RecurringData::new();
        data.set_weekly_day(Weekday::Sun, true);
        data.set_weekly_day(Weekday::Sat, true);

        // 模型值 → MPX 值：首字符为周日
        let days = RecurrenceUtility::get_days(&data);
        assert_eq!(days, "1000001");

        // MPX 值 → 模型值
        let bitmap = RecurrenceUtility::get_days_from_string(Some(days.as_str()))
            .expect("合法二进制位图");
        assert_eq!(bitmap, 0b1000001);

        let mut round_tripped = RecurringData::new();
        round_tripped.set_weekly_days_from_bitmap(
            Some(bitmap),
            &RecurrenceUtility::RECURRING_TASK_DAY_MASKS,
        );
        assert!(round_tripped.get_weekly_day(Weekday::Sun));
        assert!(round_tripped.get_weekly_day(Weekday::Sat));
        assert!(!round_tripped.get_weekly_day(Weekday::Mon));
        assert_eq!(RecurrenceUtility::get_days(&round_tripped), days);
    }

    #[test]
    fn days_bitmap_handles_empty_and_invalid_values() {
        let data = RecurringData::new();
        assert_eq!(RecurrenceUtility::get_days(&data), "0000000");
        assert_eq!(RecurrenceUtility::get_days_from_string(Some("0000000")), Some(0));
        // Java: Integer.parseInt 对 null 与非法输入的处理（NPE -> none / NumberFormatException）
        assert_eq!(RecurrenceUtility::get_days_from_string(None), None);
        assert_eq!(RecurrenceUtility::get_days_from_string(Some("")), None);
        assert_eq!(RecurrenceUtility::get_days_from_string(Some("1010120")), None);
    }

    #[test]
    fn day_values_round_trip() {
        let expected = [
            (1, Weekday::Mon),
            (2, Weekday::Tue),
            (3, Weekday::Wed),
            (4, Weekday::Thu),
            (5, Weekday::Fri),
            (6, Weekday::Sat),
            (7, Weekday::Sun),
        ];

        for (index, day) in expected {
            assert_eq!(RecurrenceUtility::get_day(Some(index)), Some(day));
            assert_eq!(RecurrenceUtility::get_day_index(Some(day)), Some(index));
        }
    }

    #[test]
    fn day_values_handle_out_of_range_and_missing_values() {
        // Java: DAY_ARRAY[0] 为 null
        assert_eq!(RecurrenceUtility::get_day(Some(0)), None);
        // Java: 抛 ArrayIndexOutOfBoundsException
        assert_eq!(RecurrenceUtility::get_day(Some(8)), None);
        assert_eq!(RecurrenceUtility::get_day(Some(-1)), None);
        assert_eq!(RecurrenceUtility::get_day(None), None);
        assert_eq!(RecurrenceUtility::get_day_index(None), None);
    }

    #[test]
    fn yearly_absolute_date_uses_start_date_year() {
        let mut data = RecurringData::new();
        data.set_start_date(Some(
            NaiveDate::from_ymd_opt(2024, 5, 1).expect("合法日期"),
        ));
        data.set_day_number(Some(10));
        data.set_month_number(Some(6));

        assert_eq!(
            RecurrenceUtility::get_yearly_absolute_as_date(&data),
            NaiveDate::from_ymd_opt(2024, 6, 10)
        );
    }

    #[test]
    fn yearly_absolute_date_requires_all_components() {
        let start_date = NaiveDate::from_ymd_opt(2024, 5, 1).expect("合法日期");

        let mut data = RecurringData::new();
        data.set_start_date(Some(start_date));
        assert_eq!(RecurrenceUtility::get_yearly_absolute_as_date(&data), None);

        let mut data = RecurringData::new();
        data.set_day_number(Some(10));
        data.set_month_number(Some(6));
        assert_eq!(RecurrenceUtility::get_yearly_absolute_as_date(&data), None);

        // Java: LocalDate.of 对非法日期抛 DateTimeException
        let mut data = RecurringData::new();
        data.set_start_date(Some(start_date));
        data.set_day_number(Some(31));
        data.set_month_number(Some(2));
        assert_eq!(RecurrenceUtility::get_yearly_absolute_as_date(&data), None);

        let mut data = RecurringData::new();
        data.set_start_date(Some(start_date));
        data.set_day_number(Some(1));
        data.set_month_number(Some(13));
        assert_eq!(RecurrenceUtility::get_yearly_absolute_as_date(&data), None);
    }

    #[test]
    fn recurring_task_day_masks_match_java() {
        assert_eq!(
            RecurrenceUtility::RECURRING_TASK_DAY_MASKS,
            [0x00, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01]
        );
    }
}
