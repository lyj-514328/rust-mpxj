//! 对应 Java: `org/mpxj/mpp/RecurringTaskReader.java`
//!
//! 从 MPP 文件的重复任务数据块中读出重复任务定义（起始 / 结束日期、工期、重复频率、
//! 每周的哪几天、每月 / 每年的相对或绝对规则等）。
//!
//! 数据块布局（偏移单位：字节）：
//!
//! ```text
//!  6 : 起始日期（2 字节，自 1983-12-31 起的天数）
//! 10 : 结束日期（2 字节）
//! 12 : 工期（4 字节，十分之一分钟）
//! 16 : 工期单位（2 字节）
//! 18 : 发生次数（2 字节）
//! 20 : 重复类型（2 字节；取值 1 = 按日、4 = 按周、8 = 按月、16 = 按年）
//! 24 : 是否使用结束日期（2 字节，1 表示是）
//! 26 : 是否仅工作日（2 字节，1 表示是）
//! 28..40 : 周日 / 周一 / … / 周六（各 2 字节，1 表示是）
//! 42 : 按月重复是否为「相对」规则（2 字节）
//! 44 : 按年重复是否为「绝对」规则（2 字节，注意与 42 的判定方向相反）
//! 46/48/54/58 : 频率（按日 / 按周 / 按月绝对 / 按月相对）
//! 50/52/56 : 按月相对 / 绝对规则的「第几个」与星期几
//! 60/62/64 : 按年相对规则的「第几个」「星期几」「月份」
//! 70 : 按年绝对规则的日期（2 字节）
//! ```
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `final class RecurringTaskReader` | `pub struct RecurringTaskReader` | Rust 无包级可见性 |
//! | `RecurringTaskReader(ProjectProperties properties)` | [`RecurringTaskReader::new`] 接收 `&ProjectProperties`，只保留其**父文件弱引用** | `ProjectProperties` 由 `ProjectFile` 持有且不是 `Clone`，Rust 侧无法保存一份属性副本；读取工期时需要属性里的「每天 / 每周分钟数」，故按弱引用回到父文件按需借用（Java 持有稳定引用） |
//! | `processRecurringTask(Task task, byte[] data)`（`task.addRecurringTask()` 返回 `RecurringTask`） | [`RecurringTaskReader::process_recurring_task`] 接收 `&[u8]` 并**返回**本文件的 [`RecurringTask`] | 「读取」与「挂到任务上」拆成两步：Java 在方法内部 `task.addRecurringTask()` 取到实例后逐字段写入；Rust 侧写入任务需要 `&mut Task`，故由调用方拿到读取结果后自行经 `Task::add_recurring_task()`（已落地，见 `crate::task::Task::add_recurring_task`）挂到任务上 |
//! | `RecurringTask extends RecurringData`（额外持有 `m_duration`） | 本文件的 [`RecurringTask`] 用**组合 + 转发**表达：持有一个 [`RecurringData`] 与一个 `Option<Duration>` | 遵循「Java 继承 → 组合」约定（`RecurringData` 无 `m_duration`，故 duration 只能放在本类型上）。**已知重复**：`crate::recurring_task::RecurringTask` 是同一 Java 类的另一份实现（用 `Deref` 还原继承，访问器名为 `get_recurring_data` / `get_recurring_data_mut`）；合并两者会牵动调用方（`mpx_reader.rs`），故本文件暂留同名类型 |
//! | `ByteArrayHelper.getShort/getInt` 越界 | 先判长度，不足返回 0 | Java 抛 `ArrayIndexOutOfBoundsException` |
//! | `switch (rt.getRecurrenceType())` 在类型为 `null` 时 | `match` 的 `None` 分支（不进入任何 case） | Java 对 `null` 枚举做 `switch` 会抛 `NullPointerException` |
//! | `DayOfWeekHelper.getInstance(int)` 返回 `null` 时 `setDayOfWeek(null)` | 跳过该次设置 | Java 会在 `EnumSet.add(null)` 抛 `NullPointerException` |
//!
//! 跳过的 Java 成员：无（`processRecurringTask` 已逐一翻译，其内部所有 setter 调用均已保留）。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use chrono::Weekday;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::day_of_week_helper::DayOfWeekHelper;
use crate::duration::Duration;
use crate::mpp::mpp_utility::MPPUtility;
use crate::project_file::ProjectFile;
use crate::project_properties::ProjectProperties;
use crate::recurrence_type::RecurrenceType;
use crate::recurring_data::RecurringData;
use crate::time_unit::TimeUnit;

/// 重复类型为「按月」时，「相对」标志所在的偏移。
///
/// 对应 Java: `rt.setRelative(ByteArrayHelper.getShort(data, 42) == 1)`
const MONTHLY_RELATIVE_OFFSET: usize = 42;

/// 重复类型为「按年」时，「绝对」标志所在的偏移。
///
/// 对应 Java: `rt.setRelative(ByteArrayHelper.getShort(data, 44) != 1)`
const YEARLY_ABSOLUTE_OFFSET: usize = 44;

/// 起始日期所在偏移。
///
/// 对应 Java: `MPPUtility.getDate(data, 6)`
const START_DATE_OFFSET: usize = 6;

/// 结束日期所在偏移。
///
/// 对应 Java: `MPPUtility.getDate(data, 10)`
const FINISH_DATE_OFFSET: usize = 10;

/// 工期所在偏移。
///
/// 对应 Java: `ByteArrayHelper.getInt(data, 12)`
const DURATION_OFFSET: usize = 12;

/// 工期单位所在偏移。
///
/// 对应 Java: `ByteArrayHelper.getShort(data, 16)`
const DURATION_UNITS_OFFSET: usize = 16;

/// 发生次数所在偏移。
///
/// 对应 Java: `ByteArrayHelper.getShort(data, 18)`
const OCCURRENCES_OFFSET: usize = 18;

/// 重复类型所在偏移。
///
/// 对应 Java: `ByteArrayHelper.getShort(data, 20)`
const RECURRENCE_TYPE_OFFSET: usize = 20;

/// 「使用结束日期」标志所在偏移。
///
/// 对应 Java: `ByteArrayHelper.getShort(data, 24)`
const USE_END_DATE_OFFSET: usize = 24;

/// 「仅工作日」标志所在偏移。
///
/// 对应 Java: `ByteArrayHelper.getShort(data, 26)`
const WORKING_DAYS_ONLY_OFFSET: usize = 26;

/// 每周 7 天标志的起始偏移（周日在前）。
///
/// 对应 Java: `ByteArrayHelper.getShort(data, 28)` … `ByteArrayHelper.getShort(data, 40)`
const WEEKLY_DAYS_OFFSET: usize = 28;

/// 重复任务的读取结果。
///
/// 对应 Java: `public final class RecurringTask extends RecurringData`
///（Rust 侧为组合 + 转发的还原，见文件头差异表）。
pub struct RecurringTask {
    /// 对应 Java 字段: `private Duration m_duration`
    duration: Option<Duration>,
    /// 对应 Java 的父类 `RecurringData`
    data: RecurringData,
}

impl RecurringTask {
    /// 构造方法。
    ///
    /// 对应 Java: `RecurringTask` 的隐式默认构造方法（父类 `RecurringData` 同时初始化）
    pub fn new() -> Self {
        Self {
            duration: None,
            data: RecurringData::new(),
        }
    }

    /// 取重复任务的工期。
    ///
    /// 对应 Java: `public Duration getDuration()`
    pub fn get_duration(&self) -> Option<Duration> {
        self.duration
    }

    /// 设置重复任务的工期。
    ///
    /// 对应 Java: `public void setDuration(Duration duration)`
    pub fn set_duration(&mut self, duration: Option<Duration>) {
        self.duration = duration;
    }

    /// 取重复数据部分（对应 Java 中继承自 `RecurringData` 的字段）。
    ///
    /// Rust 特有：Java 侧无该方法（字段由继承直接获得）。
    pub fn get_data(&self) -> &RecurringData {
        &self.data
    }

    /// 取可变的重复数据部分。
    ///
    /// Rust 特有：Java 侧字段由继承直接获得。
    pub fn get_data_mut(&mut self) -> &mut RecurringData {
        &mut self.data
    }
}

impl Default for RecurringTask {
    /// 对应 Java: 隐式默认构造方法。
    fn default() -> Self {
        Self::new()
    }
}

/// 重复任务定义读取器。
///
/// 对应 Java: `final class RecurringTaskReader`
pub struct RecurringTaskReader {
    /// 项目属性所属的父文件（弱引用）。
    ///
    /// 对应 Java 字段: `private final ProjectProperties m_properties`
    ///（Java 保存属性对象的引用；Rust 侧按弱引用回到父文件按需借用，见文件头差异表）
    parent_file: Weak<RefCell<ProjectFile>>,
}

impl RecurringTaskReader {
    /// 构造方法。
    ///
    /// 对应 Java: `public RecurringTaskReader(ProjectProperties properties)`
    pub fn new(properties: &ProjectProperties) -> Self {
        Self {
            parent_file: properties
                .get_parent_file()
                .map_or_else(Weak::new, |file| Rc::downgrade(&file)),
        }
    }

    /// 读取重复任务数据。
    ///
    /// 对应 Java: `public void processRecurringTask(Task task, byte[] data)`
    ///
    /// Java 会在方法内部创建 `RecurringTask` 并写入传入的 `task`；Rust 侧返回该实例
    /// （见文件头差异表），由调用方负责挂载。
    pub fn process_recurring_task(&self, data: &[u8]) -> RecurringTask {
        // Java: rt.setDuration(MPPUtility.getAdjustedDuration(m_properties, ByteArrayHelper.getInt(data, 12),
        //      MPPUtility.getDurationTimeUnits(ByteArrayHelper.getShort(data, 16))));
        // （先算出工期，再取得 `RecurringData` 的可变借用；两者的借用期不能重叠。）
        let duration = self.adjusted_duration(
            read_int(data, DURATION_OFFSET),
            MPPUtility::get_duration_time_units(read_short(data, DURATION_UNITS_OFFSET)),
        );

        let mut result = RecurringTask::new();
        result.set_duration(duration);
        let rt = result.get_data_mut();

        // Java: rt.setStartDate(MPPUtility.getDate(data, 6));
        rt.set_start_date(MPPUtility::get_date(data, START_DATE_OFFSET));
        // Java: rt.setFinishDate(MPPUtility.getDate(data, 10));
        rt.set_finish_date(MPPUtility::get_date(data, FINISH_DATE_OFFSET));
        // Java: rt.setOccurrences(Integer.valueOf(ByteArrayHelper.getShort(data, 18)));
        rt.set_occurrences(Some(read_short(data, OCCURRENCES_OFFSET)));
        // Java: rt.setRecurrenceType(RecurrenceType.getInstance(ByteArrayHelper.getShort(data, 20)));
        rt.set_recurrence_type(RecurrenceType::get_instance(read_short(
            data,
            RECURRENCE_TYPE_OFFSET,
        )));
        // Java: rt.setUseEndDate(ByteArrayHelper.getShort(data, 24) == 1);
        rt.set_use_end_date(read_short(data, USE_END_DATE_OFFSET) == 1);
        // Java: rt.setWorkingDaysOnly(ByteArrayHelper.getShort(data, 26) == 1);
        rt.set_working_days_only(read_short(data, WORKING_DAYS_ONLY_OFFSET) == 1);

        // Java:
        // rt.setWeeklyDay(DayOfWeek.SUNDAY, ByteArrayHelper.getShort(data, 28) == 1);
        // rt.setWeeklyDay(DayOfWeek.MONDAY, ByteArrayHelper.getShort(data, 30) == 1);
        // … 依次到 SATURDAY（偏移 40）
        for (index, day) in DAYS.into_iter().enumerate() {
            rt.set_weekly_day(day, read_short(data, WEEKLY_DAYS_OFFSET + index * 2) == 1);
        }

        // Java: int frequencyOffset = 0; int dayOfWeekOffset = 0; int dayNumberOffset = 0;
        //       int monthNumberOffset = 0; int dateOffset = 0;
        let mut frequency_offset = 0usize;
        let mut day_of_week_offset = 0usize;
        let mut day_number_offset = 0usize;
        let mut month_number_offset = 0usize;
        let mut date_offset = 0usize;

        // Java: switch (rt.getRecurrenceType()) —— Java 对 null 会抛 NPE，Rust 侧不进入任何分支
        match rt.get_recurrence_type() {
            Some(RecurrenceType::Daily) => {
                // Java: frequencyOffset = 46;
                frequency_offset = 46;
            }
            Some(RecurrenceType::Weekly) => {
                // Java: frequencyOffset = 48;
                frequency_offset = 48;
            }
            Some(RecurrenceType::Monthly) => {
                // Java: rt.setRelative(ByteArrayHelper.getShort(data, 42) == 1);
                rt.set_relative(read_short(data, MONTHLY_RELATIVE_OFFSET) == 1);
                if rt.get_relative() {
                    // Java: frequencyOffset = 58; dayNumberOffset = 50; dayOfWeekOffset = 52;
                    frequency_offset = 58;
                    day_number_offset = 50;
                    day_of_week_offset = 52;
                } else {
                    // Java: frequencyOffset = 54; dayNumberOffset = 56;
                    frequency_offset = 54;
                    day_number_offset = 56;
                }
            }
            Some(RecurrenceType::Yearly) => {
                // Java: rt.setRelative(ByteArrayHelper.getShort(data, 44) != 1);（与按月相反）
                rt.set_relative(read_short(data, YEARLY_ABSOLUTE_OFFSET) != 1);
                if rt.get_relative() {
                    // Java: dayNumberOffset = 60; dayOfWeekOffset = 62; monthNumberOffset = 64;
                    day_number_offset = 60;
                    day_of_week_offset = 62;
                    month_number_offset = 64;
                } else {
                    // Java: dateOffset = 70;
                    date_offset = 70;
                }
            }
            None => {}
        }

        if frequency_offset != 0 {
            // Java: rt.setFrequency(Integer.valueOf(ByteArrayHelper.getShort(data, frequencyOffset)));
            rt.set_frequency(Some(read_short(data, frequency_offset)));
        }

        if day_of_week_offset != 0 {
            // Java: rt.setDayOfWeek(DayOfWeekHelper.getInstance(ByteArrayHelper.getShort(data, dayOfWeekOffset) + 1));
            // Java 在取到 null 时于 setDayOfWeek 内抛 NPE，Rust 侧跳过该次设置
            if let Some(day) = DayOfWeekHelper::get_instance(read_short(data, day_of_week_offset) + 1) {
                rt.set_day_of_week(day);
            }
        }

        if day_number_offset != 0 {
            // Java: rt.setDayNumber(Integer.valueOf(ByteArrayHelper.getShort(data, dayNumberOffset)));
            rt.set_day_number(Some(read_short(data, day_number_offset)));
        }

        if month_number_offset != 0 {
            // Java: rt.setMonthNumber(Integer.valueOf(ByteArrayHelper.getShort(data, monthNumberOffset)));
            rt.set_month_number(Some(read_short(data, month_number_offset)));
        }

        if date_offset != 0 {
            // Java: rt.setYearlyAbsoluteFromDate(MPPUtility.getDate(data, dateOffset));
            rt.set_yearly_absolute_from_date(MPPUtility::get_date(data, date_offset));
        }

        result
    }

    /// 按项目属性把文件中的工期换算成用户可见的工期。
    ///
    /// 对应 Java: `MPPUtility.getAdjustedDuration(m_properties, duration, timeUnit)`
    ///
    /// Rust 侧需要借用父文件里的属性对象：父文件未接线或正被可变借用时退化为
    /// [`MPPUtility::get_duration_int`]（不做单位调整，不 panic）。
    fn adjusted_duration(&self, duration: i32, time_unit: TimeUnit) -> Option<Duration> {
        let Some(file) = self.parent_file.upgrade() else {
            return Some(MPPUtility::get_duration_int(duration, time_unit));
        };

        match file.try_borrow() {
            Ok(file) => MPPUtility::get_adjusted_duration(
                file.get_project_properties(),
                duration,
                time_unit,
            ),
            Err(_) => Some(MPPUtility::get_duration_int(duration, time_unit)),
        }
    }
}

/// 每周 7 天的顺序（与 Java 中 `setWeeklyDay(SUNDAY, …)` 到 `setWeeklyDay(SATURDAY, …)` 的
/// 调用顺序一致：偏移 28 起，每 2 字节一天）。
const DAYS: [Weekday; 7] = [
    Weekday::Sun,
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
];

/// 读两字节短整数；数据不足时返回 0（Java 抛 `ArrayIndexOutOfBoundsException`）。
fn read_short(data: &[u8], offset: usize) -> i32 {
    if offset.checked_add(2).is_none_or(|end| end > data.len()) {
        return 0;
    }

    ByteArrayHelper::get_short(data, offset)
}

/// 读四字节整数；数据不足时返回 0（Java 抛 `ArrayIndexOutOfBoundsException`）。
fn read_int(data: &[u8], offset: usize) -> i32 {
    if offset.checked_add(4).is_none_or(|end| end > data.len()) {
        return 0;
    }

    ByteArrayHelper::get_int(data, offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::rc::Weak;

    /// 不含父文件的属性对象（工期换算退化为「不调整」）。
    fn properties() -> ProjectProperties {
        ProjectProperties::new(Weak::new())
    }

    /// 构造重复任务数据块：所有字段按偏移写入，末尾留出足够空间。
    ///
    /// 默认内容为「按月相对规则」：起始 1984-01-01、结束 1984-01-31、工期 1 天、
    /// 发生 5 次、周一与周五、频率 2、第 3 个周三。
    fn recurring_task_data() -> Vec<u8> {
        let mut data = vec![0u8; 80];

        // 起始日期：1984-01-01（自 1983-12-31 起 1 天）
        data[START_DATE_OFFSET..START_DATE_OFFSET + 2].copy_from_slice(&1u16.to_le_bytes());
        // 结束日期：1984-01-31（31 天）
        data[FINISH_DATE_OFFSET..FINISH_DATE_OFFSET + 2].copy_from_slice(&31u16.to_le_bytes());
        // 工期：4800 个十分之一分钟 = 1 天
        data[DURATION_OFFSET..DURATION_OFFSET + 4].copy_from_slice(&4800i32.to_le_bytes());
        // 工期单位：7 = Days
        data[DURATION_UNITS_OFFSET..DURATION_UNITS_OFFSET + 2]
            .copy_from_slice(&7u16.to_le_bytes());
        // 发生次数
        data[OCCURRENCES_OFFSET..OCCURRENCES_OFFSET + 2].copy_from_slice(&5u16.to_le_bytes());
        // 重复类型：8 = Monthly
        data[RECURRENCE_TYPE_OFFSET..RECURRENCE_TYPE_OFFSET + 2]
            .copy_from_slice(&8u16.to_le_bytes());
        // 使用结束日期 / 仅工作日
        data[USE_END_DATE_OFFSET..USE_END_DATE_OFFSET + 2].copy_from_slice(&1u16.to_le_bytes());
        data[WORKING_DAYS_ONLY_OFFSET..WORKING_DAYS_ONLY_OFFSET + 2]
            .copy_from_slice(&1u16.to_le_bytes());
        // 周一（偏移 30）与周五（偏移 38）
        data[30..32].copy_from_slice(&1u16.to_le_bytes());
        data[38..40].copy_from_slice(&1u16.to_le_bytes());
        // 按月「相对」规则
        data[MONTHLY_RELATIVE_OFFSET..MONTHLY_RELATIVE_OFFSET + 2]
            .copy_from_slice(&1u16.to_le_bytes());
        // 频率 2、第 3 个、星期几 3（Java 侧取值 + 1 = 4 → 周三）
        data[58..60].copy_from_slice(&2u16.to_le_bytes());
        data[50..52].copy_from_slice(&3u16.to_le_bytes());
        data[52..54].copy_from_slice(&3u16.to_le_bytes());

        data
    }

    /// 把数据块中的重复类型改写为指定值。
    fn with_recurrence_type(mut data: Vec<u8>, value: u16) -> Vec<u8> {
        data[RECURRENCE_TYPE_OFFSET..RECURRENCE_TYPE_OFFSET + 2]
            .copy_from_slice(&value.to_le_bytes());
        data
    }

    /// 按月相对规则：绝大部分字段都被读出。
    #[test]
    fn reads_monthly_relative_recurring_task() {
        let reader = RecurringTaskReader::new(&properties());
        let task = reader.process_recurring_task(&recurring_task_data());
        let rt = task.get_data();

        assert_eq!(rt.get_start_date(), NaiveDate::from_ymd_opt(1984, 1, 1));
        assert_eq!(rt.get_finish_date(), NaiveDate::from_ymd_opt(1984, 1, 31));
        // 工期单位 DAYS、值 4800 个十分之一分钟（无父文件时不按项目默认值调整）
        assert_eq!(
            task.get_duration().map(|d| (d.get_duration(), d.get_units())),
            Some((4800.0 / 4800.0, TimeUnit::Days))
        );
        assert_eq!(rt.get_occurrences(), Some(5));
        assert_eq!(rt.get_recurrence_type(), Some(RecurrenceType::Monthly));
        assert!(rt.get_use_end_date());
        assert!(rt.get_working_days_only());
        assert!(rt.get_relative());
        assert_eq!(rt.get_frequency(), Some(2));
        assert_eq!(rt.get_day_number(), Some(3));
        // DayOfWeekHelper 的取值：周日 = 1 … 周六 = 7，故 4 对应周三
        assert_eq!(rt.get_day_of_week(), Some(Weekday::Wed));
        // 按月规则不涉及月份号
        assert_eq!(rt.get_month_number(), None);

        // Java 的 `setDayOfWeek` 会先清空再设置单个星期几，因此最终只剩周三
        assert!(rt.get_weekly_day(Weekday::Wed));
        assert!(!rt.get_weekly_day(Weekday::Sun));
        assert!(!rt.get_weekly_day(Weekday::Mon));
        assert!(!rt.get_weekly_day(Weekday::Fri));
    }

    /// 每周 7 天标志（按日重复，不会被 `setDayOfWeek` 覆盖）。
    #[test]
    fn reads_weekly_day_flags() {
        let data = with_recurrence_type(recurring_task_data(), 1); // Daily
        let reader = RecurringTaskReader::new(&properties());
        let task = reader.process_recurring_task(&data);
        let rt = task.get_data();

        assert_eq!(rt.get_recurrence_type(), Some(RecurrenceType::Daily));
        assert!(!rt.get_weekly_day(Weekday::Sun));
        assert!(rt.get_weekly_day(Weekday::Mon));
        assert!(!rt.get_weekly_day(Weekday::Tue));
        assert!(!rt.get_weekly_day(Weekday::Wed));
        assert!(!rt.get_weekly_day(Weekday::Thu));
        assert!(rt.get_weekly_day(Weekday::Fri));
        assert!(!rt.get_weekly_day(Weekday::Sat));
        // 按日规则不设置「相对」
        assert!(!rt.get_relative());
        assert_eq!(rt.get_day_number(), None);
        // 按日规则的频率取自偏移 46（默认 0）
        assert_eq!(rt.get_frequency(), Some(0));
    }

    /// 按日重复：频率取自偏移 46。
    #[test]
    fn reads_daily_frequency() {
        let mut data = with_recurrence_type(recurring_task_data(), 1);
        data[46..48].copy_from_slice(&3u16.to_le_bytes());

        let reader = RecurringTaskReader::new(&properties());
        let task = reader.process_recurring_task(&data);

        assert_eq!(task.get_data().get_frequency(), Some(3));
    }

    /// 按周重复：频率取自偏移 48。
    #[test]
    fn reads_weekly_frequency() {
        let mut data = with_recurrence_type(recurring_task_data(), 4);
        data[48..50].copy_from_slice(&2u16.to_le_bytes());

        let reader = RecurringTaskReader::new(&properties());
        let task = reader.process_recurring_task(&data);

        assert_eq!(
            task.get_data().get_recurrence_type(),
            Some(RecurrenceType::Weekly)
        );
        assert_eq!(task.get_data().get_frequency(), Some(2));
    }

    /// 按月绝对规则：频率与「第几天」取自另一组偏移。
    #[test]
    fn reads_monthly_absolute_recurring_task() {
        let mut data = with_recurrence_type(recurring_task_data(), 8);
        data[MONTHLY_RELATIVE_OFFSET..MONTHLY_RELATIVE_OFFSET + 2]
            .copy_from_slice(&0u16.to_le_bytes()); // 非相对
        data[54..56].copy_from_slice(&1u16.to_le_bytes()); // 频率
        data[56..58].copy_from_slice(&15u16.to_le_bytes()); // 第几天

        let reader = RecurringTaskReader::new(&properties());
        let task = reader.process_recurring_task(&data);
        let rt = task.get_data();

        assert!(!rt.get_relative());
        assert_eq!(rt.get_frequency(), Some(1));
        assert_eq!(rt.get_day_number(), Some(15));
        // 绝对规则不会设置星期几；每周标志保留数据块中的取值（周一与周五），
        // `get_day_of_week` 因而返回 Java 枚举顺序中的第一个（周一）。
        assert_eq!(rt.get_day_of_week(), Some(Weekday::Mon));
        assert!(rt.get_weekly_day(Weekday::Mon));
        assert!(rt.get_weekly_day(Weekday::Fri));
        assert_eq!(rt.get_month_number(), None);
    }

    /// 按年绝对规则：`data[44] == 1` 表示「绝对」，从日期字段推导出月份号与日期号。
    #[test]
    fn reads_yearly_absolute_recurring_task() {
        let mut data = with_recurrence_type(recurring_task_data(), 16);
        data[YEARLY_ABSOLUTE_OFFSET..YEARLY_ABSOLUTE_OFFSET + 2]
            .copy_from_slice(&1u16.to_le_bytes());
        // 1983-12-31 + 100 天 = 1984-04-09
        data[70..72].copy_from_slice(&100u16.to_le_bytes());

        let reader = RecurringTaskReader::new(&properties());
        let task = reader.process_recurring_task(&data);
        let rt = task.get_data();

        assert!(!rt.get_relative());
        assert_eq!(rt.get_month_number(), Some(4));
        assert_eq!(rt.get_day_number(), Some(9));
        assert_eq!(rt.get_frequency(), None);
    }

    /// 按年相对规则：`data[44] != 1` 表示「相对」，读取第几个 / 星期几 / 月份。
    #[test]
    fn reads_yearly_relative_recurring_task() {
        let mut data = with_recurrence_type(recurring_task_data(), 16);
        data[YEARLY_ABSOLUTE_OFFSET..YEARLY_ABSOLUTE_OFFSET + 2]
            .copy_from_slice(&0u16.to_le_bytes());
        data[60..62].copy_from_slice(&4u16.to_le_bytes()); // 第几个
        data[62..64].copy_from_slice(&5u16.to_le_bytes()); // 星期几（+1 = 6 → 周五）
        data[64..66].copy_from_slice(&11u16.to_le_bytes()); // 月份

        let reader = RecurringTaskReader::new(&properties());
        let task = reader.process_recurring_task(&data);
        let rt = task.get_data();

        assert!(rt.get_relative());
        assert_eq!(rt.get_day_number(), Some(4));
        assert_eq!(rt.get_day_of_week(), Some(Weekday::Fri));
        assert_eq!(rt.get_month_number(), Some(11));
    }

    /// 数据不足时所有读取都退化为 0 / `None`，不 panic。
    #[test]
    fn short_data_does_not_panic() {
        let reader = RecurringTaskReader::new(&properties());

        let task = reader.process_recurring_task(&[]);
        assert_eq!(task.get_data().get_start_date(), None);
        assert_eq!(task.get_data().get_finish_date(), None);
        // 工期读到 0，按 DAYS 换算后仍为 0（Java 的 getAdjustedDuration 同样返回 0 而非 null）
        assert_eq!(
            task.get_duration().map(|value| (value.get_duration(), value.get_units())),
            Some((0.0, TimeUnit::Days))
        );
        // Java 会因数组越界抛异常；Rust 侧退化为 0
        assert_eq!(task.get_data().get_occurrences(), Some(0));
        // RecurrenceType.getInstance(0) 在 Java 中回落到 DAILY
        assert_eq!(
            task.get_data().get_recurrence_type(),
            Some(RecurrenceType::Daily)
        );
        assert!(!task.get_data().get_use_end_date());
        assert_eq!(task.get_data().get_frequency(), Some(0));

        // 只够读到工期单位
        let task = reader.process_recurring_task(&recurring_task_data()[..18]);
        assert_eq!(task.get_data().get_occurrences(), Some(0));
    }

    /// 落在 `RecurrenceType` 查找表空洞中的取值对应 Java 的 `null`。
    #[test]
    fn recurrence_type_hole_yields_no_recurrence_type() {
        let data = with_recurrence_type(recurring_task_data(), 2);
        let reader = RecurringTaskReader::new(&properties());
        let task = reader.process_recurring_task(&data);
        let rt = task.get_data();

        assert_eq!(rt.get_recurrence_type(), None);
        assert_eq!(rt.get_frequency(), None);
        assert_eq!(rt.get_day_number(), None);
        assert_eq!(rt.get_month_number(), None);
    }

    /// 父文件可用时按项目属性换算工期（480 分钟/天 → 9600 即 2 天）。
    #[test]
    fn adjusted_duration_uses_project_properties() {
        let file = ProjectFile::new();
        let reader = {
            let file_ref = file.borrow();
            RecurringTaskReader::new(file_ref.get_project_properties())
        };

        let mut data = recurring_task_data();
        data[DURATION_OFFSET..DURATION_OFFSET + 4].copy_from_slice(&9600i32.to_le_bytes());

        let task = reader.process_recurring_task(&data);
        let duration = task.get_duration().expect("工期有效");
        assert_eq!(duration.get_duration(), 2.0);
        assert_eq!(duration.get_units(), TimeUnit::Days);
    }

    /// 非「天」单位时直接按单位换算（不涉及项目默认值）。
    #[test]
    fn adjusted_duration_for_hour_units() {
        let file = ProjectFile::new();
        let reader = {
            let file_ref = file.borrow();
            RecurringTaskReader::new(file_ref.get_project_properties())
        };

        let mut data = recurring_task_data();
        data[DURATION_UNITS_OFFSET..DURATION_UNITS_OFFSET + 2]
            .copy_from_slice(&5u16.to_le_bytes()); // Hours
        data[DURATION_OFFSET..DURATION_OFFSET + 4].copy_from_slice(&600i32.to_le_bytes());

        let task = reader.process_recurring_task(&data);
        let duration = task.get_duration().expect("工期有效");
        assert_eq!(duration.get_duration(), 1.0);
        assert_eq!(duration.get_units(), TimeUnit::Hours);
    }

    /// `RecurringTask` 的工期既能读写，又能访问组合的 `RecurringData`。
    #[test]
    fn recurring_task_wraps_recurring_data() {
        let mut task = RecurringTask::new();
        assert_eq!(task.get_duration(), None);
        assert_eq!(task.get_data().get_recurrence_type(), None);

        task.set_duration(Some(Duration::get_instance(3.0, TimeUnit::Days)));
        task.get_data_mut()
            .set_recurrence_type(Some(RecurrenceType::Weekly));

        assert_eq!(
            task.get_duration().map(|d| d.get_duration()),
            Some(3.0)
        );
        assert_eq!(
            task.get_data().get_recurrence_type(),
            Some(RecurrenceType::Weekly)
        );

        // Default 等价于 new
        assert_eq!(RecurringTask::default().get_duration(), None);
    }
}
