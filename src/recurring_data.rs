//! 对应 Java: `org/mpxj/RecurringData.java`
//!
//! 重复事件（重复任务 / 重复日历例外）的描述与日期展开算法：按日 / 周 / 月 / 年频率
//! 展开出一串发生日期。
//!
//! # 跳过的 Java 机制
//!
//! 本类不涉及反射、JDBC、XML/JAXB、GUI、`javax.*` 或 `Serializable`，**没有跳过任何 Java 方法**。
//! `toString()` 中用到的 Java 特有设施按下列方式替换：
//!
//! - `ByteArrayOutputStream` + `PrintWriter` → [`std::fmt::Write`]（即 `write!`）。
//! - `DateFormatSymbols`（跟随默认 Locale 输出星期 / 月份名称）→ 固定英文名称常量
//!   [`WEEKDAY_NAMES`] / [`MONTH_NAMES`]，取值与英文 Locale 下的 `DateFormatSymbols` 一致
//!   （`getWeekdays()` 是「下标 0 为空串、1 = Sunday … 7 = Saturday」的 8 元素数组，
//!   `getMonths()` 是「下标 0 = January … 11 = December、12 为空串」的 13 元素数组）。
//!
//! # 与 Java 的差异
//!
//! - Java 字段可为 `null`，Rust 侧统一为 `Option<T>`。Java 在缺少必需字段时会抛
//!   `NullPointerException` / `DateTimeException` / `ArrayIndexOutOfBoundsException`，
//!   Rust 侧不 panic：日期展开过程中一旦无法继续推进就结束展开（保留已生成的日期），
//!   文本化方法则以 `"null"` 占位；具体见各方法注释。
//! - Java 的 `getDates()` 返回内部数组引用，Rust 侧的 [`RecurringData::get_dates`] 返回克隆的
//!   `Vec<NaiveDate>`（Java 侧数组可变，Rust 侧只读）。
//! - Java 的 `getCalculatedFirstDate()` / `getCalculatedLastDate()` 在未生成任何日期时抛
//!   `ArrayIndexOutOfBoundsException`，Rust 侧分别返回哨兵值 `NaiveDate::MIN` / `NaiveDate::MAX`
//!   （与 `isValid()` 配合使用可避免依赖哨兵值）。
//! - Java 的 `setWeeklyDaysFromBitmap(Integer, int[])` 在 `masks` 数组越界时抛
//!   `ArrayIndexOutOfBoundsException`，Rust 侧按掩码 0 处理（即该位视为未设置）。
//! - Java 的 `LocalDate` 运算（`plusDays` / `plusMonths` / `plusYears`）越界时抛
//!   `DateTimeException`，Rust 侧用 `checked_*` 系列并结束展开。

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};

use chrono::{Datelike, Months, NaiveDate, TimeDelta, Weekday};

use crate::common::day_of_week_helper::DayOfWeekHelper;
use crate::recurrence_type::RecurrenceType;

/// `java.time.DayOfWeek.values()` 的顺序（周一 → 周日）。
///
/// 对应 Java: `RecurringData` 中出现的 `for (DayOfWeek day : DayOfWeek.values())`。
/// 注意这与 [`DayOfWeekHelper::ORDERED_DAYS`]（周日在前，用于按 int 值取下标）不同。
const JAVA_DAY_OF_WEEK_ORDER: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];

/// 对应 Java: `private static final String[] ORDINAL` —— 生成调试输出用的序数词。
const ORDINAL: [Option<&str>; 4] = [None, Some("every"), Some("every other"), Some("every 3rd")];

/// 对应 Java: `private static final String[] DAY_ORDINAL` —— 生成调试输出用的「第几个」。
const DAY_ORDINAL: [Option<&str>; 6] = [
    None,
    Some("First"),
    Some("Second"),
    Some("Third"),
    Some("Fourth"),
    Some("Last"),
];

/// 对应 Java: `DateFormatSymbols.getWeekdays()`（英文 Locale）：下标 0 为空串，1 = 周日 … 7 = 周六。
const WEEKDAY_NAMES: [&str; 8] = [
    "",
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// 对应 Java: `DateFormatSymbols.getMonths()`（英文 Locale）：下标 0 = January … 11 = December，12 为空串。
const MONTH_NAMES: [&str; 13] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
    "",
];

/// 重复事件的描述数据。
///
/// 对应 Java: `public class RecurringData`
#[derive(Debug, Clone)]
pub struct RecurringData {
    /// 对应 Java 字段 `private LocalDate m_startDate`
    start_date: Option<NaiveDate>,
    /// 对应 Java 字段 `private LocalDate m_finishDate`
    finish_date: Option<NaiveDate>,
    /// 对应 Java 字段 `private Integer m_occurrences`
    occurrences: Option<i32>,
    /// 对应 Java 字段 `private RecurrenceType m_recurrenceType`
    recurrence_type: Option<RecurrenceType>,
    /// 对应 Java 字段 `private boolean m_relative`
    relative: bool,
    /// 对应 Java 字段 `private boolean m_workingDaysOnly`
    working_days_only: bool,
    /// 对应 Java 字段 `private boolean m_useEndDate`
    use_end_date: bool,
    /// 对应 Java 字段 `private Integer m_frequency`
    frequency: Option<i32>,
    /// 对应 Java 字段 `private Integer m_dayNumber`
    day_number: Option<i32>,
    /// 对应 Java 字段 `private Integer m_monthNumber`
    month_number: Option<i32>,
    /// 对应 Java 字段 `private LocalDate[] m_dates`（`None` 表示缓存失效，对应 Java 的 `null`）
    ///
    /// Java 的 `populateDates()` 通过惰性写入 `m_dates` 实现缓存；Rust 侧的
    /// [`RecurringData::get_dates`] 等只读方法也要能触发填充，故用 `RefCell` 提供内部可变性。
    dates: RefCell<Option<Vec<NaiveDate>>>,
    /// 对应 Java 字段 `private final EnumSet<DayOfWeek> m_days`
    ///
    /// Java 的 `EnumSet` 按枚举自然序（周一 → 周日）迭代，`getDayOfWeek()` 取首个元素。
    /// `chrono::Weekday` 未实现 `Ord`，无法直接放进 `BTreeSet`；Rust 侧改用定长布尔数组
    /// 表达集合（下标换算见 [`day_index`]），迭代与「取首个元素」都显式按
    /// [`JAVA_DAY_OF_WEEK_ORDER`]（周一 → 周日）进行，与 Java 语义一致。
    days: [bool; 7],
}

impl RecurringData {
    /// 构造方法。
    ///
    /// 对应 Java: 隐式无参构造方法 `RecurringData()`
    pub fn new() -> Self {
        Self {
            start_date: None,
            finish_date: None,
            occurrences: None,
            recurrence_type: None,
            relative: false,
            working_days_only: false,
            use_end_date: false,
            frequency: None,
            day_number: None,
            month_number: None,
            dates: RefCell::new(None),
            days: [false; 7],
        }
    }

    /// 取重复的起始日期。
    ///
    /// 对应 Java: `public LocalDate getStartDate()`
    pub fn get_start_date(&self) -> Option<NaiveDate> {
        self.start_date
    }

    /// 设置重复的起始日期。
    ///
    /// 对应 Java: `public void setStartDate(LocalDate val)`
    pub fn set_start_date(&mut self, val: Option<NaiveDate>) {
        self.start_date = val;
        self.clear_dates_cache();
    }

    /// 取重复的结束日期。
    ///
    /// 对应 Java: `public LocalDate getFinishDate()`
    pub fn get_finish_date(&self) -> Option<NaiveDate> {
        self.finish_date
    }

    /// 设置重复的结束日期。
    ///
    /// 对应 Java: `public void setFinishDate(LocalDate val)`
    pub fn set_finish_date(&mut self, val: Option<NaiveDate>) {
        self.finish_date = val;
        self.clear_dates_cache();
    }

    /// 取发生次数。
    ///
    /// 对应 Java: `public Integer getOccurrences()`
    pub fn get_occurrences(&self) -> Option<i32> {
        self.occurrences
    }

    /// 设置发生次数。
    ///
    /// 对应 Java: `public void setOccurrences(Integer occurrences)`
    pub fn set_occurrences(&mut self, occurrences: Option<i32>) {
        self.occurrences = occurrences;
        self.clear_dates_cache();
    }

    /// 取重复类型。
    ///
    /// 对应 Java: `public RecurrenceType getRecurrenceType()`
    pub fn get_recurrence_type(&self) -> Option<RecurrenceType> {
        self.recurrence_type
    }

    /// 设置重复类型。
    ///
    /// 对应 Java: `public void setRecurrenceType(RecurrenceType type)`
    pub fn set_recurrence_type(&mut self, r#type: Option<RecurrenceType>) {
        self.recurrence_type = r#type;
        self.clear_dates_cache();
    }

    /// 取「使用结束日期」标志。
    ///
    /// 对应 Java: `public boolean getUseEndDate()`
    pub fn get_use_end_date(&self) -> bool {
        self.use_end_date
    }

    /// 设置「使用结束日期」标志。
    ///
    /// 对应 Java: `public void setUseEndDate(boolean useEndDate)`
    pub fn set_use_end_date(&mut self, use_end_date: bool) {
        self.use_end_date = use_end_date;
        self.clear_dates_cache();
    }

    /// 取「仅工作日」标志（仅对每日重复有意义）。
    ///
    /// 对应 Java: `public boolean getWorkingDaysOnly()`
    pub fn get_working_days_only(&self) -> bool {
        self.working_days_only
    }

    /// 设置「仅工作日」标志。
    ///
    /// 对应 Java: `public void setWorkingDaysOnly(boolean workingDaysOnly)`
    pub fn set_working_days_only(&mut self, working_days_only: bool) {
        self.working_days_only = working_days_only;
        self.clear_dates_cache();
    }

    /// 判断某星期几是否属于按周重复的一部分。
    ///
    /// 对应 Java: `public boolean getWeeklyDay(DayOfWeek day)`
    pub fn get_weekly_day(&self, day: Weekday) -> bool {
        self.days[day_index(day)]
    }

    /// 设置按周重复中单个星期几的开关状态。
    ///
    /// 对应 Java: `public void setWeeklyDay(DayOfWeek day, boolean value)`
    pub fn set_weekly_day(&mut self, day: Weekday, value: bool) {
        self.days[day_index(day)] = value;
        self.clear_dates_cache();
    }

    /// 用位图加掩码数组设置按周重复的每一天。
    ///
    /// 对应 Java: `public void setWeeklyDaysFromBitmap(Integer days, int[] masks)`
    ///
    /// 掩码下标用 [`DayOfWeekHelper::get_value`]（周日 = 1 … 周六 = 7）求值；
    /// Java 在 `masks` 越界时抛 `ArrayIndexOutOfBoundsException`，Rust 侧按掩码 0 处理。
    pub fn set_weekly_days_from_bitmap(&mut self, days: Option<i32>, masks: &[i32]) {
        if let Some(days) = days {
            for day in JAVA_DAY_OF_WEEK_ORDER {
                let mask = masks
                    .get(DayOfWeekHelper::get_value(day) as usize)
                    .copied()
                    .unwrap_or(0);
                self.set_weekly_day(day, (days & mask) != 0);
            }
            self.clear_dates_cache();
        }
    }

    /// 取「相对」标志，仅对按月 / 按年重复有意义。
    ///
    /// 对应 Java: `public boolean getRelative()`
    pub fn get_relative(&self) -> bool {
        self.relative
    }

    /// 设置「相对」标志，仅对按月 / 按年重复有意义。
    ///
    /// 对应 Java: `public void setRelative(boolean relative)`
    pub fn set_relative(&mut self, relative: bool) {
        self.relative = relative;
        self.clear_dates_cache();
    }

    /// 取重复频率。
    ///
    /// 对应 Java: `public Integer getFrequency()`
    pub fn get_frequency(&self) -> Option<i32> {
        self.frequency
    }

    /// 设置重复频率。
    ///
    /// 对应 Java: `public void setFrequency(Integer frequency)`
    pub fn set_frequency(&mut self, frequency: Option<i32>) {
        self.frequency = frequency;
        self.clear_dates_cache();
    }

    /// 取按月 / 按年「相对」重复所依据的星期几。
    ///
    /// 对应 Java: `public DayOfWeek getDayOfWeek()`
    ///
    /// Java 取 `EnumSet` 的首个元素（按周一 → 周日排序），为空时返回 `null`。
    pub fn get_day_of_week(&self) -> Option<Weekday> {
        JAVA_DAY_OF_WEEK_ORDER
            .into_iter()
            .find(|day| self.get_weekly_day(*day))
    }

    /// 设置按月 / 按年「相对」重复所依据的星期几（会覆盖原有取值）。
    ///
    /// 对应 Java: `public void setDayOfWeek(DayOfWeek day)`
    ///
    /// Java 传入 `null` 时 `EnumSet.add(null)` 抛 `NullPointerException`；
    /// Rust 侧参数非空，故该分支不可达。
    pub fn set_day_of_week(&mut self, day: Weekday) {
        self.days = [false; 7];
        self.days[day_index(day)] = true;
        self.clear_dates_cache();
    }

    /// 取按月 / 按年「绝对」重复所依据的日期号。
    ///
    /// 对应 Java: `public Integer getDayNumber()`
    pub fn get_day_number(&self) -> Option<i32> {
        self.day_number
    }

    /// 设置按月 / 按年「绝对」重复所依据的日期号。
    ///
    /// 对应 Java: `public void setDayNumber(Integer day)`
    pub fn set_day_number(&mut self, day: Option<i32>) {
        self.day_number = day;
        self.clear_dates_cache();
    }

    /// 取按年重复的月份号（1 = 一月 … 12 = 十二月）。
    ///
    /// 对应 Java: `public Integer getMonthNumber()`
    pub fn get_month_number(&self) -> Option<i32> {
        self.month_number
    }

    /// 设置按年重复的月份号。
    ///
    /// 对应 Java: `public void setMonthNumber(Integer month)`
    pub fn set_month_number(&mut self, month: Option<i32>) {
        self.month_number = month;
        self.clear_dates_cache();
    }

    /// 取本重复规则展开出的全部起始日期。
    ///
    /// 对应 Java: `public LocalDate[] getDates()`
    ///
    /// Java 返回内部数组引用，Rust 侧返回克隆的 `Vec<NaiveDate>`。
    pub fn get_dates(&self) -> Vec<NaiveDate> {
        self.populate_dates();
        let dates = self.dates.borrow();
        dates.as_ref().cloned().unwrap_or_default()
    }

    /// 配置是否有效（即能否展开出一个或多个起始日期）。
    ///
    /// 对应 Java: `public boolean isValid()`
    pub fn is_valid(&self) -> bool {
        self.populate_dates();
        let populated = self.dates.borrow().as_ref().is_some_and(|dates| !dates.is_empty());
        populated
    }

    /// 取实际会发生例外的首个计算日期。
    ///
    /// 对应 Java: `public LocalDate getCalculatedFirstDate()`
    ///
    /// 用户给出的起始日期未必与算法生成的日期对齐，故需要本方法。
    /// Java 在未生成任何日期时抛 `ArrayIndexOutOfBoundsException`，Rust 侧返回
    /// `NaiveDate::MIN` 作为哨兵值（可用 [`RecurringData::is_valid`] 先行判断）。
    pub fn get_calculated_first_date(&self) -> NaiveDate {
        let dates = self.get_dates();
        dates.first().copied().unwrap_or(NaiveDate::MIN)
    }

    /// 取实际会发生例外的最后一个计算日期。
    ///
    /// 对应 Java: `public LocalDate getCalculatedLastDate()`
    ///
    /// Java 在未生成任何日期时抛 `ArrayIndexOutOfBoundsException`，Rust 侧返回
    /// `NaiveDate::MAX` 作为哨兵值（可用 [`RecurringData::is_valid`] 先行判断）。
    pub fn get_calculated_last_date(&self) -> NaiveDate {
        let dates = self.get_dates();
        dates.last().copied().unwrap_or(NaiveDate::MAX)
    }

    /// 按年「绝对」重复的设置：直接从一个日期取出日期号与月份号。
    ///
    /// 对应 Java: `public void setYearlyAbsoluteFromDate(LocalDate date)`
    pub fn set_yearly_absolute_from_date(&mut self, date: Option<NaiveDate>) {
        if let Some(date) = date {
            self.day_number = Some(date.day() as i32);
            self.month_number = Some(date.month() as i32);
            self.clear_dates_cache();
        }
    }

    /// 惰性展开日期并写入缓存。
    ///
    /// 对应 Java: `private void populateDates()`
    fn populate_dates(&self) {
        let populated = self.dates.borrow().is_some();
        if populated {
            return;
        }

        // Java: int frequency = NumberHelper.getInt(m_frequency); if (frequency < 1) frequency = 1;
        let mut frequency = self.frequency.unwrap_or(0);
        if frequency < 1 {
            frequency = 1;
        }

        let mut dates = Vec::new();

        // Java: 起始日期为 null 时各 getXxxDates 会抛 NullPointerException；
        // Rust 侧不 panic，保持空的日期列表（isValid() 返回 false）。
        if let Some(start_date) = self.start_date {
            // Java 的 switch(m_recurrenceType) 在类型为 null 时没有任何分支被选中
            match self.recurrence_type {
                Some(RecurrenceType::Daily) => self.get_daily_dates(frequency, start_date, &mut dates),
                Some(RecurrenceType::Weekly) => self.get_weekly_dates(frequency, start_date, &mut dates),
                Some(RecurrenceType::Monthly) => self.get_monthly_dates(frequency, start_date, &mut dates),
                Some(RecurrenceType::Yearly) => self.get_yearly_dates(start_date, &mut dates),
                None => {}
            }
        }

        *self.dates.borrow_mut() = Some(dates);
    }

    /// 判断是否还需要计算下一个日期。
    ///
    /// 对应 Java: `private boolean moreDates(LocalDate date, List<LocalDate> dates)`
    ///
    /// 没有结束日期时退化为使用发生次数（假定重复数据至少给出其中之一）。
    fn more_dates(&self, date: NaiveDate, dates: &[NaiveDate]) -> bool {
        match self.finish_date {
            None => {
                // Java: int occurrences = NumberHelper.getInt(m_occurrences); if (occurrences < 1) occurrences = 1;
                let occurrences = match self.occurrences {
                    Some(occurrences) if occurrences >= 1 => occurrences,
                    _ => 1,
                };
                dates.len() < occurrences as usize
            }
            // Java: !date.isAfter(m_finishDate)
            Some(finish_date) => date <= finish_date,
        }
    }

    /// 展开每日重复的日期。
    ///
    /// 对应 Java: `private void getDailyDates(int frequency, List<LocalDate> dates)`
    fn get_daily_dates(&self, frequency: i32, start_date: NaiveDate, dates: &mut Vec<NaiveDate>) {
        let mut date = start_date;
        while self.more_dates(date, dates) {
            dates.push(date);
            match plus_days(date, frequency as i64) {
                Some(next) => date = next,
                // Java: date.plusDays() 越界抛 DateTimeException，Rust 侧结束展开
                None => break,
            }
        }
    }

    /// 展开每周重复的日期。
    ///
    /// 对应 Java: `private void getWeeklyDates(int frequency, List<LocalDate> dates)`
    ///
    /// 算法要点：从包含起始日期的那一周的周日开始扫描（早于起始日期的匹配被忽略），
    /// 每次最多扫描 7 天，扫完一整周后按 `7 * (frequency - 1)` 跳过若干整周。
    fn get_weekly_dates(&self, frequency: i32, start_date: NaiveDate, dates: &mut Vec<NaiveDate>) {
        let mut date = start_date;

        //
        // 从包含起始日期的那一周的周日开始，略过早于起始日期的匹配。
        //
        let mut current_day = start_date.weekday();
        if current_day != Weekday::Sun {
            // Java: date.minusDays(currentDay.getValue())，其中 getValue() 为周一 = 1 … 周日 = 7
            match plus_days(date, -java_day_of_week_value(current_day)) {
                Some(adjusted) => date = adjusted,
                None => return,
            }
            current_day = Weekday::Sun;
        }

        while self.more_dates(date, dates) {
            let mut offset: i64 = 0;

            for _day_index in 0..7 {
                if self.get_weekly_day(current_day) {
                    if offset != 0 {
                        // 命中本周内的目标星期时，把日期推进到该天
                        match plus_days(date, offset) {
                            Some(advanced) => date = advanced,
                            None => return,
                        }
                        offset = 0;
                    }
                    if !self.more_dates(date, dates) {
                        break;
                    }

                    if date >= start_date {
                        dates.push(date);
                    }
                }

                offset += 1;
                // Java: currentDay.plus(1) —— 周日之后回绕到周一
                current_day = current_day.succ();
            }

            if frequency > 1 {
                offset += 7 * (frequency as i64 - 1);
            }
            match plus_days(date, offset) {
                Some(next) => date = next,
                None => return,
            }
        }
    }

    /// 展开每月重复的日期（区分「相对」与「绝对」两种模式）。
    ///
    /// 对应 Java: `private void getMonthlyDates(int frequency, List<LocalDate> dates)`
    fn get_monthly_dates(&self, frequency: i32, start_date: NaiveDate, dates: &mut Vec<NaiveDate>) {
        if self.relative {
            self.get_monthly_relative_dates(frequency, start_date, dates);
        } else {
            self.get_monthly_absolute_dates(frequency, start_date, dates);
        }
    }

    /// 展开「每月的第 n 个星期几」形式的日期。
    ///
    /// 对应 Java: `private void getMonthlyRelativeDates(int frequency, List<LocalDate> dates)`
    ///
    /// 每次都从当月的 1 号出发计算相对日；若算出的日期早于起始日期则放弃该次，
    /// 但仍要按频率推进到后面的月份。`dayNumber > 4` 表示取当月最后一个星期几。
    fn get_monthly_relative_dates(
        &self,
        frequency: i32,
        start_date: NaiveDate,
        dates: &mut Vec<NaiveDate>,
    ) {
        // Java: LocalDate.of(m_startDate.getYear(), m_startDate.getMonth(), 1)
        let Some(first_of_month) = date_of(start_date.year(), start_date.month() as i32, 1) else {
            return;
        };
        let mut date = first_of_month;
        let day_number = self.day_number.unwrap_or(0);

        while self.more_dates(date, dates) {
            let relative = if day_number > 4 {
                self.get_last_relative_day(date)
            } else {
                self.get_ordinal_relative_day(date, day_number)
            };
            // Java: getDayOfWeek() 为 null（未设置星期几）时抛 NullPointerException
            let Some(relative) = relative else { return };

            if relative >= start_date {
                dates.push(relative);
                if !self.more_dates(relative, dates) {
                    break;
                }
            }

            // Java: date = LocalDate.of(date.getYear(), date.getMonth(), 1); date = date.plusMonths(frequency)
            let Some(month_start) = date_of(relative.year(), relative.month() as i32, 1) else {
                break;
            };
            match month_start.checked_add_months(Months::new(frequency as u32)) {
                Some(next) => date = next,
                None => break,
            }
        }
    }

    /// 展开「每月的第 n 天」形式的日期（月末不足时取当月最后一天）。
    ///
    /// 对应 Java: `private void getMonthlyAbsoluteDates(int frequency, List<LocalDate> dates)`
    fn get_monthly_absolute_dates(
        &self,
        frequency: i32,
        start_date: NaiveDate,
        dates: &mut Vec<NaiveDate>,
    ) {
        let current_day_number = start_date.day() as i32;
        // Java: date = LocalDate.of(date.getYear(), date.getMonth(), 1)
        let Some(month_start) = date_of(start_date.year(), start_date.month() as i32, 1) else {
            return;
        };
        let mut date = month_start;
        let required_day_number = self.day_number.unwrap_or(0);

        // 要求的日期号已经早于起始日期的日期号时，从下一个月开始
        if required_day_number < current_day_number {
            match date.checked_add_months(Months::new(1)) {
                Some(next) => date = next,
                None => return,
            }
        }

        while self.more_dates(date, dates) {
            let max_day_number = length_of_month(date.year(), date.month())
                .map_or(0, |days| days as i32);
            // Java: 目标日期号超过当月天数时取当月最后一天（YearMonth.lengthOfMonth()）
            let use_day_number = if required_day_number > max_day_number {
                max_day_number
            } else {
                required_day_number
            };

            let Some(current) = date_of(date.year(), date.month() as i32, use_day_number) else {
                break;
            };
            dates.push(current);

            // Java: date = LocalDate.of(date.getYear(), date.getMonth(), 1); date = date.plusMonths(frequency)
            let Some(next_month_start) = date_of(current.year(), current.month() as i32, 1) else {
                break;
            };
            match next_month_start.checked_add_months(Months::new(frequency as u32)) {
                Some(next) => date = next,
                None => break,
            }
        }
    }

    /// 展开每年重复的日期（区分「相对」与「绝对」两种模式）。
    ///
    /// 对应 Java: `private void getYearlyDates(List<LocalDate> dates)`
    ///
    /// 注意 Java 未把 frequency 传给按年重复，按年重复恒为「每年」。
    fn get_yearly_dates(&self, start_date: NaiveDate, dates: &mut Vec<NaiveDate>) {
        if self.relative {
            self.get_yearly_relative_dates(start_date, dates);
        } else {
            self.get_yearly_absolute_dates(start_date, dates);
        }
    }

    /// 展开「每年某月的第 n 个星期几」形式的日期。
    ///
    /// 对应 Java: `private void getYearlyRelativeDates(List<LocalDate> dates)`
    fn get_yearly_relative_dates(&self, start_date: NaiveDate, dates: &mut Vec<NaiveDate>) {
        let month_number = self.month_number.unwrap_or(0);
        // Java: LocalDate.of(m_startDate.getYear(), NumberHelper.getInt(m_monthNumber), 1)
        let Some(first_of_month) = date_of(start_date.year(), month_number, 1) else {
            return;
        };
        let mut date = first_of_month;
        let day_number = self.day_number.unwrap_or(0);

        while self.more_dates(date, dates) {
            let relative = if day_number > 4 {
                self.get_last_relative_day(date)
            } else {
                self.get_ordinal_relative_day(date, day_number)
            };
            // Java: getDayOfWeek() 为 null（未设置星期几）时抛 NullPointerException
            let Some(relative) = relative else { return };

            if relative >= start_date {
                dates.push(relative);
                if !self.more_dates(relative, dates) {
                    break;
                }
            }

            // Java: date = LocalDate.of(date.getYear() + 1, date.getMonth(), 1)
            let Some(month_start) = date_of(relative.year(), relative.month() as i32, 1) else {
                break;
            };
            match month_start.checked_add_months(Months::new(12)) {
                Some(next) => date = next,
                None => break,
            }
        }
    }

    /// 展开「每年某月某日」形式的日期（当月不足时取当月最后一天）。
    ///
    /// 对应 Java: `private void getYearlyAbsoluteDates(List<LocalDate> dates)`
    fn get_yearly_absolute_dates(&self, start_date: NaiveDate, dates: &mut Vec<NaiveDate>) {
        let month_number = self.month_number.unwrap_or(0);
        // Java: LocalDate.of(m_startDate.getYear(), NumberHelper.getInt(m_monthNumber), 1)
        let Some(first_of_month) = date_of(start_date.year(), month_number, 1) else {
            return;
        };
        let mut date = first_of_month;
        let required_day_number = self.day_number.unwrap_or(0);

        while self.more_dates(date, dates) {
            let max_day_number = length_of_month(date.year(), date.month())
                .map_or(0, |days| days as i32);
            // Java: 目标日期号超过当月天数时取当月最后一天（YearMonth.lengthOfMonth()）
            let use_day_number = if required_day_number > max_day_number {
                max_day_number
            } else {
                required_day_number
            };

            let Some(mut current) = date_of(date.year(), date.month() as i32, use_day_number) else {
                break;
            };
            // 算出的日期早于起始日期时，整体后移一年（Java: date.plusYears(1)，
            // 闰年的 2 月 29 日会按 Java 规则调整为次年的 2 月 28 日）
            if current < start_date {
                match current.checked_add_months(Months::new(12)) {
                    Some(shifted) => current = shifted,
                    None => break,
                }
            }

            dates.push(current);

            // Java: date = LocalDate.of(date.getYear(), date.getMonth(), 1); date = date.plusYears(1)
            let Some(next_month_start) = date_of(current.year(), current.month() as i32, 1) else {
                break;
            };
            match next_month_start.checked_add_months(Months::new(12)) {
                Some(next) => date = next,
                None => break,
            }
        }
    }

    /// 把日期移动到当月的第 n 个指定星期几。
    ///
    /// 对应 Java: `private LocalDate getOrdinalRelativeDay(LocalDate date, int dayNumber)`
    ///
    /// 传入的 `date` 通常是当月 1 号；先用 [`DayOfWeekHelper`] 的取值（周日 = 1 … 周六 = 7）
    /// 计算「从 1 号向后推进到第一个目标星期几」的偏移，再按 `7 * (dayNumber - 1)` 跳到第 n 个。
    /// 未设置星期几时 Java 抛 `NullPointerException`，Rust 侧返回 `None`。
    fn get_ordinal_relative_day(&self, date: NaiveDate, day_number: i32) -> Option<NaiveDate> {
        let required_day_of_week = DayOfWeekHelper::get_value(self.get_day_of_week()?);
        let current_day_of_week = DayOfWeekHelper::get_value(date.weekday());

        let day_of_week_offset = if required_day_of_week > current_day_of_week {
            required_day_of_week - current_day_of_week
        } else if required_day_of_week < current_day_of_week {
            7 - (current_day_of_week - required_day_of_week)
        } else {
            0
        };

        let mut date = plus_days(date, day_of_week_offset as i64)?;

        if day_number > 1 {
            date = plus_days(date, 7 * (day_number as i64 - 1))?;
        }

        Some(date)
    }

    /// 把日期移动到当月的最后一个指定星期几。
    ///
    /// 对应 Java: `private LocalDate getLastRelativeDay(LocalDate date)`
    ///
    /// 先把日期移到当月最后一天，再向前回退到目标星期几。
    /// 未设置星期几时 Java 抛 `NullPointerException`，Rust 侧返回 `None`。
    fn get_last_relative_day(&self, date: NaiveDate) -> Option<NaiveDate> {
        // Java: YearMonth.of(date.getYear(), date.getMonth()).lengthOfMonth()
        let last_day_of_month = length_of_month(date.year(), date.month())?;
        let mut date = date_of(
            date.year(),
            date.month() as i32,
            last_day_of_month as i32,
        )?;

        let required_day_of_week = DayOfWeekHelper::get_value(self.get_day_of_week()?);
        let current_day_of_week = DayOfWeekHelper::get_value(date.weekday());

        let day_of_week_offset = if current_day_of_week > required_day_of_week {
            required_day_of_week - current_day_of_week
        } else if current_day_of_week < required_day_of_week {
            -7 + (required_day_of_week - current_day_of_week)
        } else {
            0
        };

        if day_of_week_offset != 0 {
            date = plus_days(date, day_of_week_offset as i64)?;
        }

        Some(date)
    }

    /// 取给定整数的序数词文本（用于生成调试输出）。
    ///
    /// 对应 Java: `private String getOrdinal(Integer value)`
    ///
    /// Java 在 `value` 为 `null` 时抛 `NullPointerException`、下标越界时抛
    /// `ArrayIndexOutOfBoundsException`，Rust 侧统一以 `"null"` 占位。
    fn get_ordinal(&self, value: Option<i32>) -> String {
        match value {
            Some(index) if index >= ORDINAL.len() as i32 => format!("every {}th", index),
            Some(index) => ORDINAL
                .get(usize::try_from(index).unwrap_or(usize::MAX))
                .copied()
                .flatten()
                .map_or_else(|| "null".to_string(), str::to_string),
            None => "null".to_string(),
        }
    }

    /// 使日期缓存失效。
    ///
    /// 对应 Java: `private void clearDatesCache()`
    fn clear_dates_cache(&self) {
        *self.dates.borrow_mut() = None;
    }
}

impl Default for RecurringData {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for RecurringData {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 用 `PrintWriter` 拼装，Rust 侧用 `write!`；星期 / 月份名称固定为英文
    /// （见文件头说明）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[RecurringData {}",
            recurrence_type_name(self.recurrence_type)
        )?;

        match self.recurrence_type {
            Some(RecurrenceType::Daily) => {
                write!(f, " {}", self.get_ordinal(self.frequency))?;
                write!(
                    f,
                    "{}",
                    if self.working_days_only {
                        " Working day"
                    } else {
                        " Day"
                    }
                )?;
            }
            Some(RecurrenceType::Weekly) => {
                write!(f, " {}", self.get_ordinal(self.frequency))?;
                write!(f, " week on ")?;

                let mut separator = "";
                for day in JAVA_DAY_OF_WEEK_ORDER {
                    if self.get_weekly_day(day) {
                        write!(f, "{}{}", separator, weekday_name(Some(day)))?;
                        separator = ", ";
                    }
                }
            }
            Some(RecurrenceType::Monthly) => {
                if self.relative {
                    write!(f, " on The ")?;
                    write!(f, "{}", day_ordinal(self.day_number))?;
                    write!(f, " {}", weekday_name(self.get_day_of_week()))?;
                    write!(f, " of ")?;
                    write!(f, "{}", self.get_ordinal(self.frequency))?;
                } else {
                    write!(f, " on Day ")?;
                    write!(f, "{}", optional_integer(self.day_number))?;
                    write!(f, " of ")?;
                    write!(f, "{}", self.get_ordinal(self.frequency))?;
                }
                write!(f, " month")?;
            }
            Some(RecurrenceType::Yearly) => {
                write!(f, " on the ")?;
                if self.relative {
                    write!(f, "{}", day_ordinal(self.day_number))?;
                    write!(f, " {}", weekday_name(self.get_day_of_week()))?;
                    write!(f, " of {}", month_name(self.month_number))?;
                } else {
                    write!(
                        f,
                        "{} {}",
                        optional_integer(self.day_number),
                        month_name(self.month_number)
                    )?;
                }
            }
            None => {}
        }

        if let Some(start_date) = self.start_date {
            write!(f, " From {}", start_date)?;
        }

        if let Some(occurrences) = self.occurrences {
            write!(f, " For {} occurrences", occurrences)?;
        }

        if let Some(finish_date) = self.finish_date {
            write!(f, " To {}", finish_date)?;
        }

        write!(f, "]")
    }
}

/// 对应 Java 的 `java.time.DayOfWeek.getValue()`：周一 = 1 … 周日 = 7。
fn java_day_of_week_value(day: Weekday) -> i64 {
    day.num_days_from_monday() as i64 + 1
}

/// 把星期换算为 [`RecurringData::days`] 数组下标：对应 Java 的 `DayOfWeekHelper.getValue(day) - 1`。
fn day_index(day: Weekday) -> usize {
    (DayOfWeekHelper::get_value(day) - 1) as usize
}

/// 对应 Java 的 `LocalDate.plusDays(days)`，越界时返回 `None`（Java 抛 `DateTimeException`）。
fn plus_days(date: NaiveDate, days: i64) -> Option<NaiveDate> {
    TimeDelta::try_days(days).and_then(|duration| date.checked_add_signed(duration))
}

/// 对应 Java 的 `LocalDate.of(year, month, day)`：取值非法时返回 `None`（Java 抛 `DateTimeException`）。
fn date_of(year: i32, month: i32, day: i32) -> Option<NaiveDate> {
    let month = u32::try_from(month).ok()?;
    let day = u32::try_from(day).ok()?;
    NaiveDate::from_ymd_opt(year, month, day)
}

/// 对应 Java 的 `YearMonth.lengthOfMonth()`：当月天数。
fn length_of_month(year: i32, month: u32) -> Option<u32> {
    let start = NaiveDate::from_ymd_opt(year, month, 1)?;
    let end = start.checked_add_months(Months::new(1))?;
    Some((end - start).num_days() as u32)
}

/// 对应 Java 中 `pw.print(m_recurrenceType)` 的输出（枚举 `toString()`，`null` 输出 `"null"`）。
fn recurrence_type_name(recurrence_type: Option<RecurrenceType>) -> &'static str {
    recurrence_type.map_or("null", |recurrence_type| recurrence_type.get_name())
}

/// 对应 Java 中 `Integer` 直接参与字符串拼接的输出（`null` 输出 `"null"`）。
fn optional_integer(value: Option<i32>) -> String {
    value.map_or_else(|| "null".to_string(), |value| value.to_string())
}

/// 对应 Java 中 `Integer`（`m_dayNumber`）经 `DAY_ORDINAL` 取「第几个」名称的输出。
///
/// Java 在 `null` 时抛 `NullPointerException`、下标越界时抛 `ArrayIndexOutOfBoundsException`，
/// Rust 侧统一以 `"null"` 占位。
fn day_ordinal(value: Option<i32>) -> String {
    value
        .and_then(|index| usize::try_from(index).ok())
        .and_then(|index| DAY_ORDINAL.get(index))
        .copied()
        .flatten()
        .map_or_else(|| "null".to_string(), str::to_string)
}

/// 对应 Java 中 `DateFormatSymbols.getWeekdays()[DayOfWeekHelper.getValue(day)]` 的输出。
///
/// `getValue` 的取值是周日 = 1 … 周六 = 7，正好是 `WEEKDAY_NAMES` 的下标；
/// Java 在 `day` 为 `null` 时抛 `NullPointerException`，Rust 侧以 `"null"` 占位。
fn weekday_name(day: Option<Weekday>) -> &'static str {
    match day {
        Some(day) => WEEKDAY_NAMES
            .get(DayOfWeekHelper::get_value(day) as usize)
            .copied()
            .unwrap_or("null"),
        None => "null",
    }
}

/// 对应 Java 中 `DateFormatSymbols.getMonths()[m_monthNumber - 1]` 的输出。
///
/// Java 在 `null` 时抛 `NullPointerException`、下标越界时抛 `ArrayIndexOutOfBoundsException`，
/// Rust 侧统一以 `"null"` 占位。
fn month_name(month_number: Option<i32>) -> &'static str {
    month_number
        .and_then(|month_number| usize::try_from(month_number - 1).ok())
        .and_then(|index| MONTH_NAMES.get(index))
        .copied()
        .unwrap_or("null")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("合法日期")
    }

    fn daily(frequency: Option<i32>) -> RecurringData {
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Daily));
        data.set_frequency(frequency);
        data
    }

    fn weekly(days: &[Weekday], frequency: Option<i32>) -> RecurringData {
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Weekly));
        data.set_frequency(frequency);
        for day in days {
            data.set_weekly_day(*day, true);
        }
        data
    }

    #[test]
    fn defaults_and_accessors() {
        let mut data = RecurringData::new();

        assert_eq!(data.get_start_date(), None);
        assert_eq!(data.get_finish_date(), None);
        assert_eq!(data.get_occurrences(), None);
        assert_eq!(data.get_recurrence_type(), None);
        assert_eq!(data.get_frequency(), None);
        assert_eq!(data.get_day_number(), None);
        assert_eq!(data.get_month_number(), None);
        assert_eq!(data.get_day_of_week(), None);
        assert!(!data.get_relative());
        assert!(!data.get_working_days_only());
        assert!(!data.get_use_end_date());

        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_finish_date(Some(date(2024, 12, 31)));
        data.set_occurrences(Some(5));
        data.set_recurrence_type(Some(RecurrenceType::Monthly));
        data.set_frequency(Some(2));
        data.set_day_number(Some(15));
        data.set_month_number(Some(6));
        data.set_relative(true);
        data.set_working_days_only(true);
        data.set_use_end_date(true);

        assert_eq!(data.get_start_date(), Some(date(2024, 1, 1)));
        assert_eq!(data.get_finish_date(), Some(date(2024, 12, 31)));
        assert_eq!(data.get_occurrences(), Some(5));
        assert_eq!(data.get_recurrence_type(), Some(RecurrenceType::Monthly));
        assert_eq!(data.get_frequency(), Some(2));
        assert_eq!(data.get_day_number(), Some(15));
        assert_eq!(data.get_month_number(), Some(6));
        assert!(data.get_relative());
        assert!(data.get_working_days_only());
        assert!(data.get_use_end_date());

        data.set_recurrence_type(None);
        assert_eq!(data.get_recurrence_type(), None);
    }

    #[test]
    fn daily_uses_occurrences_when_no_finish_date() {
        let mut data = daily(Some(3));
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_occurrences(Some(4));

        assert!(data.is_valid());
        assert_eq!(
            data.get_dates(),
            vec![
                date(2024, 1, 1),
                date(2024, 1, 4),
                date(2024, 1, 7),
                date(2024, 1, 10)
            ]
        );
        assert_eq!(data.get_calculated_first_date(), date(2024, 1, 1));
        assert_eq!(data.get_calculated_last_date(), date(2024, 1, 10));
    }

    #[test]
    fn daily_uses_finish_date_boundary_inclusive() {
        let mut data = daily(Some(2));
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_finish_date(Some(date(2024, 1, 5)));
        data.set_use_end_date(true);

        // 结束日期为包含边界
        assert_eq!(
            data.get_dates(),
            vec![date(2024, 1, 1), date(2024, 1, 3), date(2024, 1, 5)]
        );
    }

    #[test]
    fn daily_defaults_frequency_and_occurrences_to_one() {
        // 频率与发生次数都为 null 时，Java 的 NumberHelper.getInt 回落为 0，再被提升为 1
        let mut data = daily(None);
        data.set_start_date(Some(date(2024, 1, 1)));

        assert_eq!(data.get_dates(), vec![date(2024, 1, 1)]);
    }

    #[test]
    fn daily_changes_clear_cached_dates() {
        let mut data = daily(Some(1));
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_occurrences(Some(3));

        assert_eq!(data.get_dates().len(), 3);

        // 缓存被清空后按新的发生次数重新展开
        data.set_occurrences(Some(5));
        assert_eq!(data.get_dates().len(), 5);
    }

    #[test]
    fn missing_start_date_yields_empty_dates() {
        // Java 侧会抛 NullPointerException，Rust 侧退化为空结果（见文件头说明）
        let mut data = daily(Some(1));
        data.set_occurrences(Some(3));

        assert!(!data.is_valid());
        assert!(data.get_dates().is_empty());
        assert_eq!(data.get_calculated_first_date(), NaiveDate::MIN);
        assert_eq!(data.get_calculated_last_date(), NaiveDate::MAX);
    }

    #[test]
    fn missing_recurrence_type_yields_empty_dates() {
        let mut data = RecurringData::new();
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_occurrences(Some(3));

        assert!(!data.is_valid());
    }

    #[test]
    fn weekly_multiple_days() {
        // 2024-01-01 是周一
        let mut data = weekly(&[Weekday::Mon, Weekday::Wed, Weekday::Fri], Some(1));
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_occurrences(Some(6));

        assert_eq!(
            data.get_dates(),
            vec![
                date(2024, 1, 1),
                date(2024, 1, 3),
                date(2024, 1, 5),
                date(2024, 1, 8),
                date(2024, 1, 10),
                date(2024, 1, 12)
            ]
        );
    }

    #[test]
    fn weekly_ignores_days_before_start_date() {
        // 起始日期是周三，且只重复周三：周一 / 周二虽在同一周但早于起始日期，必须被忽略
        let mut data = weekly(&[Weekday::Wed], Some(1));
        data.set_start_date(Some(date(2024, 1, 3)));
        data.set_occurrences(Some(3));

        assert_eq!(
            data.get_dates(),
            vec![date(2024, 1, 3), date(2024, 1, 10), date(2024, 1, 17)]
        );
    }

    #[test]
    fn weekly_frequency_skips_whole_weeks() {
        let mut data = weekly(&[Weekday::Mon], Some(2));
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_occurrences(Some(3));

        assert_eq!(
            data.get_dates(),
            vec![date(2024, 1, 1), date(2024, 1, 15), date(2024, 1, 29)]
        );
    }

    #[test]
    fn weekly_finish_date_boundary() {
        let mut data = weekly(&[Weekday::Mon, Weekday::Fri], Some(1));
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_finish_date(Some(date(2024, 1, 12)));

        assert_eq!(
            data.get_dates(),
            vec![
                date(2024, 1, 1),
                date(2024, 1, 5),
                date(2024, 1, 8),
                date(2024, 1, 12)
            ]
        );
    }

    #[test]
    fn weekly_days_from_bitmap() {
        // 掩码下标：周日 = 1、周一 = 2 … 周六 = 7
        let masks = [0, 1, 2, 4, 8, 16, 32, 64];
        let mut data = weekly(&[], None);
        // 周一（2）与周五（32）
        data.set_weekly_days_from_bitmap(Some(2 | 32), &masks);

        assert!(data.get_weekly_day(Weekday::Mon));
        assert!(data.get_weekly_day(Weekday::Fri));
        assert!(!data.get_weekly_day(Weekday::Sun));
        assert!(!data.get_weekly_day(Weekday::Tue));
        assert!(!data.get_weekly_day(Weekday::Wed));
        assert!(!data.get_weekly_day(Weekday::Thu));
        assert!(!data.get_weekly_day(Weekday::Sat));
    }

    #[test]
    fn weekly_days_from_bitmap_ignores_null_bitmap() {
        let mut data = weekly(&[], None);
        data.set_weekly_day(Weekday::Tue, true);
        data.set_weekly_days_from_bitmap(None, &[0, 1, 2, 4, 8, 16, 32, 64]);

        assert!(data.get_weekly_day(Weekday::Tue));
    }

    #[test]
    fn day_of_week_returns_first_in_java_enum_order() {
        let mut data = weekly(&[], None);
        data.set_weekly_day(Weekday::Wed, true);
        data.set_weekly_day(Weekday::Mon, true);

        // Java 的 EnumSet 迭代顺序为周一 → 周日，首个元素是周一
        assert_eq!(data.get_day_of_week(), Some(Weekday::Mon));

        data.set_day_of_week(Weekday::Fri);
        assert_eq!(data.get_day_of_week(), Some(Weekday::Fri));
        assert!(!data.get_weekly_day(Weekday::Mon));
    }

    #[test]
    fn monthly_absolute_clamps_to_end_of_month_and_handles_leap_year() {
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Monthly));
        data.set_frequency(Some(1));
        data.set_start_date(Some(date(2024, 1, 15)));
        data.set_day_number(Some(31));
        data.set_occurrences(Some(4));

        // 2 月截断到 29 日（2024 是闰年），4 月截断到 30 日
        assert_eq!(
            data.get_dates(),
            vec![
                date(2024, 1, 31),
                date(2024, 2, 29),
                date(2024, 3, 31),
                date(2024, 4, 30)
            ]
        );
    }

    #[test]
    fn monthly_absolute_skips_to_next_month_when_day_already_passed() {
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Monthly));
        data.set_frequency(Some(1));
        data.set_start_date(Some(date(2024, 1, 20)));
        data.set_day_number(Some(15));
        data.set_occurrences(Some(2));

        assert_eq!(
            data.get_dates(),
            vec![date(2024, 2, 15), date(2024, 3, 15)]
        );
    }

    #[test]
    fn monthly_relative_ordinal_day() {
        // 每月的第一个周一：2024-01-01、2024-02-05、2024-03-04
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Monthly));
        data.set_frequency(Some(1));
        data.set_relative(true);
        data.set_day_number(Some(1));
        data.set_day_of_week(Weekday::Mon);
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_occurrences(Some(3));

        assert_eq!(
            data.get_dates(),
            vec![date(2024, 1, 1), date(2024, 2, 5), date(2024, 3, 4)]
        );
    }

    #[test]
    fn monthly_relative_last_day_of_week() {
        // dayNumber > 4 表示当月最后一个指定星期几
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Monthly));
        data.set_frequency(Some(1));
        data.set_relative(true);
        data.set_day_number(Some(5));
        data.set_day_of_week(Weekday::Mon);
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_occurrences(Some(3));

        assert_eq!(
            data.get_dates(),
            vec![date(2024, 1, 29), date(2024, 2, 26), date(2024, 3, 25)]
        );
    }

    #[test]
    fn monthly_relative_ignores_dates_before_start() {
        // 起始日期是 1 月 15 日，第一个周一（1 月 1 日）早于起始日期，被忽略
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Monthly));
        data.set_frequency(Some(1));
        data.set_relative(true);
        data.set_day_number(Some(1));
        data.set_day_of_week(Weekday::Mon);
        data.set_start_date(Some(date(2024, 1, 15)));
        data.set_occurrences(Some(2));

        assert_eq!(
            data.get_dates(),
            vec![date(2024, 2, 5), date(2024, 3, 4)]
        );
    }

    #[test]
    fn monthly_relative_uses_monthly_frequency() {
        // 每两个月的最后一个周五
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Monthly));
        data.set_frequency(Some(2));
        data.set_relative(true);
        data.set_day_number(Some(5));
        data.set_day_of_week(Weekday::Fri);
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_occurrences(Some(3));

        // 2024 年 1 月最后一个周五是 1 月 26 日，之后隔月：3 月 29 日、5 月 31 日
        assert_eq!(
            data.get_dates(),
            vec![date(2024, 1, 26), date(2024, 3, 29), date(2024, 5, 31)]
        );
    }

    #[test]
    fn yearly_absolute_clamps_to_end_of_february() {
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Yearly));
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_month_number(Some(2));
        data.set_day_number(Some(29));
        data.set_occurrences(Some(3));

        // 闰年 2 月 29 日，平年截断到 2 月 28 日
        assert_eq!(
            data.get_dates(),
            vec![date(2024, 2, 29), date(2025, 2, 28), date(2026, 2, 28)]
        );
    }

    #[test]
    fn yearly_absolute_shifts_year_when_before_start_date() {
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Yearly));
        data.set_start_date(Some(date(2024, 6, 15)));
        data.set_month_number(Some(3));
        data.set_day_number(Some(10));
        data.set_occurrences(Some(2));

        assert_eq!(
            data.get_dates(),
            vec![date(2025, 3, 10), date(2026, 3, 10)]
        );
    }

    #[test]
    fn yearly_relative_ordinal_day() {
        // 每年 11 月的第四个周四（感恩节）
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Yearly));
        data.set_relative(true);
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_month_number(Some(11));
        data.set_day_number(Some(4));
        data.set_day_of_week(Weekday::Thu);
        data.set_occurrences(Some(3));

        assert_eq!(
            data.get_dates(),
            vec![date(2024, 11, 28), date(2025, 11, 27), date(2026, 11, 26)]
        );
    }

    #[test]
    fn yearly_relative_last_day_of_week() {
        // 每年 2 月的最后一个周五
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Yearly));
        data.set_relative(true);
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_month_number(Some(2));
        data.set_day_number(Some(5));
        data.set_day_of_week(Weekday::Fri);
        data.set_occurrences(Some(3));

        // 2024-02-23、2025-02-28、2026-02-27
        assert_eq!(
            data.get_dates(),
            vec![date(2024, 2, 23), date(2025, 2, 28), date(2026, 2, 27)]
        );
    }

    #[test]
    fn yearly_crosses_year_boundary() {
        // 起始日期在 12 月，按年重复会跨年
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Yearly));
        data.set_start_date(Some(date(2024, 12, 20)));
        data.set_month_number(Some(12));
        data.set_day_number(Some(31));
        data.set_occurrences(Some(3));

        assert_eq!(
            data.get_dates(),
            vec![
                date(2024, 12, 31),
                date(2025, 12, 31),
                date(2026, 12, 31)
            ]
        );
    }

    #[test]
    fn yearly_ignores_frequency() {
        // Java 的 getYearlyDates 不接收 frequency，按年重复恒为每年
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Yearly));
        data.set_frequency(Some(3));
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_month_number(Some(1));
        data.set_day_number(Some(1));
        data.set_occurrences(Some(3));

        assert_eq!(
            data.get_dates(),
            vec![date(2024, 1, 1), date(2025, 1, 1), date(2026, 1, 1)]
        );
    }

    #[test]
    fn yearly_absolute_from_date() {
        let mut data = RecurringData::new();
        data.set_month_number(Some(1));
        data.set_day_number(Some(1));
        data.set_yearly_absolute_from_date(Some(date(2024, 6, 15)));

        assert_eq!(data.get_month_number(), Some(6));
        assert_eq!(data.get_day_number(), Some(15));

        // Java 侧 date 为 null 时不改变任何字段
        data.set_yearly_absolute_from_date(None);
        assert_eq!(data.get_month_number(), Some(6));
        assert_eq!(data.get_day_number(), Some(15));
    }

    #[test]
    fn dates_cache_is_shared_between_accessors() {
        let mut data = daily(Some(1));
        data.set_start_date(Some(date(2024, 3, 1)));
        data.set_finish_date(Some(date(2024, 3, 3)));

        assert_eq!(data.get_dates().len(), 3);
        assert_eq!(data.get_calculated_first_date(), date(2024, 3, 1));
        assert_eq!(data.get_calculated_last_date(), date(2024, 3, 3));

        // 改动任一影响展开的字段都会让缓存失效
        data.set_start_date(Some(date(2024, 3, 2)));
        assert_eq!(data.get_dates(), vec![date(2024, 3, 2), date(2024, 3, 3)]);
    }

    #[test]
    fn display_daily() {
        let mut data = daily(Some(1));
        data.set_start_date(Some(date(2024, 1, 1)));
        data.set_occurrences(Some(3));

        assert_eq!(
            data.to_string(),
            "[RecurringData Daily every Day From 2024-01-01 For 3 occurrences]"
        );

        data.set_frequency(Some(2));
        data.set_working_days_only(true);
        assert_eq!(
            data.to_string(),
            "[RecurringData Daily every other Working day From 2024-01-01 For 3 occurrences]"
        );
    }

    #[test]
    fn display_weekly() {
        let mut data = weekly(&[Weekday::Fri, Weekday::Mon], Some(1));
        data.set_start_date(Some(date(2024, 1, 1)));

        assert_eq!(
            data.to_string(),
            "[RecurringData Weekly every week on Monday, Friday From 2024-01-01]"
        );

        data.set_frequency(Some(3));
        data.set_finish_date(Some(date(2024, 12, 31)));
        assert_eq!(
            data.to_string(),
            "[RecurringData Weekly every 3rd week on Monday, Friday From 2024-01-01 To 2024-12-31]"
        );
    }

    #[test]
    fn display_monthly() {
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Monthly));
        data.set_frequency(Some(3));
        data.set_start_date(Some(date(2024, 1, 15)));
        data.set_day_number(Some(15));

        assert_eq!(
            data.to_string(),
            "[RecurringData Monthly on Day 15 of every 3rd month From 2024-01-15]"
        );

        data.set_relative(true);
        data.set_day_number(Some(2));
        data.set_day_of_week(Weekday::Mon);
        assert_eq!(
            data.to_string(),
            "[RecurringData Monthly on The Second Monday of every 3rd month From 2024-01-15]"
        );
    }

    #[test]
    fn display_yearly() {
        let mut data = RecurringData::new();
        data.set_recurrence_type(Some(RecurrenceType::Yearly));
        data.set_day_number(Some(15));
        data.set_month_number(Some(6));

        assert_eq!(data.to_string(), "[RecurringData Yearly on the 15 June]");

        data.set_relative(true);
        data.set_day_number(Some(4));
        data.set_day_of_week(Weekday::Thu);
        data.set_month_number(Some(11));
        assert_eq!(
            data.to_string(),
            "[RecurringData Yearly on the Fourth Thursday of November]"
        );

        // dayNumber > 4 对应 DAY_ORDINAL 的 "Last"
        data.set_day_number(Some(5));
        assert_eq!(
            data.to_string(),
            "[RecurringData Yearly on the Last Thursday of November]"
        );
    }

    #[test]
    fn display_empty_recurrence_data() {
        let data = RecurringData::new();
        assert_eq!(data.to_string(), "[RecurringData null]");
    }

    #[test]
    fn display_frequency_above_ordinal_table() {
        let mut data = daily(Some(5));
        data.set_start_date(Some(date(2024, 1, 1)));

        assert_eq!(
            data.to_string(),
            "[RecurringData Daily every 5th Day From 2024-01-01]"
        );
    }
}
