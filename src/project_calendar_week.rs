//! 对应 Java: `org/mpxj/ProjectCalendarWeek.java`
//!
//! 跳过的 Java 方法：无。
//!
//! # 依赖说明（重要）
//!
//! [`ProjectCalendarWeek::convert_to_recurring_exceptions`] 依赖 `org.mpxj.RecurringData`
//! （对应 Rust 模块 `crate::recurring_data`）。该类型当前尚未移植，`src/lib.rs` 中也还
//! 未声明 `pub mod recurring_data;`，因此本文件目前会产生「未声明的模块」编译错误，
//! 需要在该模块落地后补上模块声明。本文件按下列签名使用 `RecurringData`：
//!
//! ```text
//! RecurringData::new() -> RecurringData
//! set_recurrence_type(Option<RecurrenceType>)
//! set_start_date(Option<NaiveDate>)
//! set_finish_date(Option<NaiveDate>)
//! set_use_end_date(bool)
//! set_weekly_day(Weekday, bool)
//! ```
//!
//! # 与 Java 的差异
//!
//! - Java 的 `ProjectCalendarWeek extends ProjectCalendarDays`；Rust 无继承，
//!   [`ProjectCalendarWeek`] 以字段组合持有 [`ProjectCalendarDays`]，并把基类方法
//!   逐个转发（见文件末尾「转发 ProjectCalendarDays」一节）。
//! - Java 在 `m_dateRange == null` 时抛 `UnsupportedOperationException`，Rust 侧
//!   [`ProjectCalendarWeek::convert_to_recurring_exceptions`] 返回 `None`；
//!   日期区间起止为 `null` 时（Java 抛 NPE）同样返回 `None`。
//! - Java 未覆写 `equals` / `hashCode`（沿用 `Object` 的引用语义），但实现了
//!   `compareTo`。Rust 的 `Ord` 需要 `Eq` 作为父 trait，因此 `PartialEq` / `Hash`
//!   与 `Ord` 保持一致：都只依据日期区间的起始日期。
//! - `toString()` 使用 `PrintWriter.println`，Java 会输出平台相关的换行符，且末尾会附加
//!   一个换行符；Rust 侧固定使用 `\n` 且末尾不带换行符。`DayType` 在 Java 侧未覆写
//!   `toString`，输出为枚举常量名（`WORKING` / `NON_WORKING` / `DEFAULT`），
//!   Rust 侧按同样文本输出。

use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};

use chrono::{NaiveDate, Weekday};

use crate::common::local_date_helper::LocalDateHelper;
use crate::day_type::DayType;
use crate::local_date_range::LocalDateRange;
use crate::local_time_range::LocalTimeRange;
use crate::project_calendar_days::{ProjectCalendarDays, JAVA_DAY_OF_WEEK_ORDER};
use crate::project_calendar_exception::ProjectCalendarException;
use crate::project_calendar_hours::ProjectCalendarHours;
use crate::recurrence_type::RecurrenceType;
use crate::recurring_data::RecurringData;

/// 一段有效期为指定日期区间的工作周（不含例外日期）。
///
/// 对应 Java: `public class ProjectCalendarWeek extends ProjectCalendarDays implements Comparable<ProjectCalendarWeek>`
#[derive(Debug, Clone)]
pub struct ProjectCalendarWeek {
    /// 对应 Java 的继承关系 `extends ProjectCalendarDays`
    days: ProjectCalendarDays,
    /// 对应 Java 字段 `private LocalDateRange m_dateRange`（`null` 表示默认工作周）
    date_range: Option<LocalDateRange>,
}

impl ProjectCalendarWeek {
    /// 构造方法。
    ///
    /// 对应 Java: 隐式无参构造方法 `ProjectCalendarWeek()`
    pub fn new() -> Self {
        Self {
            days: ProjectCalendarDays::new(),
            date_range: None,
        }
    }

    /// 取本工作周的有效日期区间（默认工作周返回 `None`）。
    ///
    /// 对应 Java: `public LocalDateRange getDateRange()`
    pub fn get_date_range(&self) -> Option<LocalDateRange> {
        self.date_range
    }

    /// 设置本工作周的有效日期区间（设为 `None` 表示默认工作周）。
    ///
    /// 对应 Java: `public void setDateRange(LocalDateRange range)`
    pub fn set_date_range(&mut self, range: Option<LocalDateRange>) {
        self.date_range = range;
    }

    /// 把本工作周转换为等价的重复例外列表。
    ///
    /// 对应 Java: `public List<ProjectCalendarException> convertToRecurringExceptions(LocalDate earliestStartDate, LocalDate latestFinishDate)`
    ///
    /// 默认工作周（`date_range` 为 `None`）无法展开，Java 抛
    /// `UnsupportedOperationException`，Rust 侧返回 `None`。
    /// 每个「非 `DEFAULT`」的星期都会生成一个按周重复的例外，重复区间被限制在
    /// 传入的边界与本工作周区间之内。
    pub fn convert_to_recurring_exceptions(
        &self,
        earliest_start_date: Option<NaiveDate>,
        latest_finish_date: Option<NaiveDate>,
    ) -> Option<Vec<ProjectCalendarException>> {
        // Java: We can't expand the default week
        let date_range = self.date_range?;

        // 避免生成超出传入边界的例外
        let mut from_date = date_range.get_start()?;
        if let Some(earliest_start_date) = earliest_start_date {
            if LocalDateHelper::compare(Some(earliest_start_date), Some(from_date)) > 0 {
                from_date = earliest_start_date;
            }
        }

        let mut to_date = date_range.get_end()?;
        if let Some(latest_finish_date) = latest_finish_date {
            if LocalDateHelper::compare(Some(to_date), Some(latest_finish_date)) > 0 {
                to_date = latest_finish_date;
            }
        }

        let mut result = Vec::new();

        // Java: for (DayOfWeek day : DayOfWeek.values())  —— 周一在前
        for day in JAVA_DAY_OF_WEEK_ORDER {
            if self.get_calendar_day_type(day) == Some(DayType::Default) {
                continue;
            }

            let mut recurrence = RecurringData::new();
            recurrence.set_recurrence_type(Some(RecurrenceType::Weekly));
            recurrence.set_start_date(Some(from_date));
            recurrence.set_finish_date(Some(to_date));
            recurrence.set_use_end_date(true);
            recurrence.set_weekly_day(day, true);

            let mut exception = ProjectCalendarException::new_from_recurring(recurrence);
            // Java: TODO: for consistency this should never be null, just empty?
            if let Some(hours) = self.get_calendar_hours_for_day(day) {
                exception.add_all(hours);
            }

            result.push(exception);
        }

        Some(result)
    }

    //
    // 以下方法对应 Java 从 ProjectCalendarDays 继承的方法（转发）
    //

    /// 对应 Java: `setName(String name)`（继承自 `ProjectCalendarDays`）
    pub fn set_name(&mut self, name: Option<String>) {
        self.days.set_name(name);
    }

    /// 对应 Java: `getName()`（继承自 `ProjectCalendarDays`）
    pub fn get_name(&self) -> Option<&str> {
        self.days.get_name()
    }

    /// 对应 Java: `getCalendarHours()`（继承自 `ProjectCalendarDays`）
    pub fn get_calendar_hours(&self) -> &[Option<ProjectCalendarHours>; 7] {
        self.days.get_calendar_hours()
    }

    /// 对应 Java: `getCalendarHours(DayOfWeek day)`（继承自 `ProjectCalendarDays`）
    pub fn get_calendar_hours_for_day(&self, day: Weekday) -> Option<&ProjectCalendarHours> {
        self.days.get_calendar_hours_for_day(day)
    }

    /// 对应 Java: `getCalendarHours(DayOfWeek day)` 的可变版本（Rust 侧补充）
    pub fn get_calendar_hours_for_day_mut(
        &mut self,
        day: Weekday,
    ) -> Option<&mut ProjectCalendarHours> {
        self.days.get_calendar_hours_for_day_mut(day)
    }

    /// 对应 Java: `addDefaultCalendarHours()`（继承自 `ProjectCalendarDays`）
    pub fn add_default_calendar_hours(&mut self) {
        self.days.add_default_calendar_hours();
    }

    /// 对应 Java: `addDefaultCalendarDays()`（继承自 `ProjectCalendarDays`）
    pub fn add_default_calendar_days(&mut self) {
        self.days.add_default_calendar_days();
    }

    /// 对应 Java: `addDefaultCalendarHours(DayOfWeek day)`（继承自 `ProjectCalendarDays`）
    pub fn add_default_calendar_hours_for_day(&mut self, day: Weekday) {
        self.days.add_default_calendar_hours_for_day(day);
    }

    /// 对应 Java: `addCalendarHours(DayOfWeek day)`（继承自 `ProjectCalendarDays`）
    pub fn add_calendar_hours(&mut self, day: Weekday) -> &mut ProjectCalendarHours {
        self.days.add_calendar_hours(day)
    }

    /// 对应 Java: `removeCalendarHours(DayOfWeek day)`（继承自 `ProjectCalendarDays`）
    pub fn remove_calendar_hours(&mut self, day: Weekday) {
        self.days.remove_calendar_hours(day);
    }

    /// 对应 Java: `getCalendarDayTypes()`（继承自 `ProjectCalendarDays`）
    pub fn get_calendar_day_types(&self) -> &[Option<DayType>; 7] {
        self.days.get_calendar_day_types()
    }

    /// 对应 Java: `getCalendarDayType(DayOfWeek day)`（继承自 `ProjectCalendarDays`）
    pub fn get_calendar_day_type(&self, day: Weekday) -> Option<DayType> {
        self.days.get_calendar_day_type(day)
    }

    /// 对应 Java: `setWorkingDay(DayOfWeek day, boolean working)`（继承自 `ProjectCalendarDays`）
    pub fn set_working_day(&mut self, day: Weekday, working: bool) {
        self.days.set_working_day(day, working);
    }

    /// 对应 Java: `setCalendarDayType(DayOfWeek day, DayType type)`（继承自 `ProjectCalendarDays`）
    pub fn set_calendar_day_type(&mut self, day: Weekday, r#type: DayType) {
        self.days.set_calendar_day_type(day, r#type);
    }

    /// 对应 Java: `DEFAULT_WORKING_MORNING`（继承自 `ProjectCalendarDays`）
    pub fn default_working_morning() -> LocalTimeRange {
        ProjectCalendarDays::default_working_morning()
    }

    /// 对应 Java: `DEFAULT_WORKING_AFTERNOON`（继承自 `ProjectCalendarDays`）
    pub fn default_working_afternoon() -> LocalTimeRange {
        ProjectCalendarDays::default_working_afternoon()
    }
}

impl Default for ProjectCalendarWeek {
    fn default() -> Self {
        Self::new()
    }
}

impl Ord for ProjectCalendarWeek {
    /// 对应 Java: `@Override public int compareTo(ProjectCalendarWeek o)`：
    /// 比较日期区间的起始日期（`null` 视为更大）。
    fn cmp(&self, other: &Self) -> Ordering {
        let lhs = self.date_range.and_then(|range| range.get_start());
        let rhs = other.date_range.and_then(|range| range.get_start());

        match LocalDateHelper::compare(lhs, rhs) {
            value if value < 0 => Ordering::Less,
            0 => Ordering::Equal,
            _ => Ordering::Greater,
        }
    }
}

impl PartialOrd for ProjectCalendarWeek {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ProjectCalendarWeek {
    /// 说明：Java 未覆写 `equals`（沿用引用相等），Rust 侧为与 `Ord` 保持一致，
    /// 按日期区间起始日期比较。
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for ProjectCalendarWeek {}

impl Hash for ProjectCalendarWeek {
    /// 与 [`ProjectCalendarWeek::eq`] 保持一致：只使用日期区间起始日期。
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.date_range
            .and_then(|range| range.get_start())
            .hash(state);
    }
}

impl Display for ProjectCalendarWeek {
    /// 对应 Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "[ProjectCalendarWeek")?;
        writeln!(f, "   name={}", self.get_name().unwrap_or("null"))?;
        writeln!(f, "   date_range={}", optional_range(self.date_range))?;

        for day in JAVA_DAY_OF_WEEK_ORDER {
            writeln!(f, "   [Day {}", java_day_of_week_name(day))?;
            writeln!(f, "      type={}", day_type_name(self.get_calendar_day_type(day)))?;
            writeln!(
                f,
                "      hours={}",
                self.get_calendar_hours_for_day(day)
                    .map_or_else(|| "null".to_string(), |hours| hours.to_string())
            )?;
            writeln!(f, "   ]")?;
        }

        write!(f, "]")
    }
}

/// 按 Java 习惯输出可空的日期区间（`null` → `null`）。
fn optional_range(range: Option<LocalDateRange>) -> String {
    range.map_or_else(|| "null".to_string(), |range| range.to_string())
}

/// 对应 Java 枚举 `DayOfWeek` 的 `name()`（`MONDAY` … `SUNDAY`）。
fn java_day_of_week_name(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "MONDAY",
        Weekday::Tue => "TUESDAY",
        Weekday::Wed => "WEDNESDAY",
        Weekday::Thu => "THURSDAY",
        Weekday::Fri => "FRIDAY",
        Weekday::Sat => "SATURDAY",
        Weekday::Sun => "SUNDAY",
    }
}

/// 对应 Java 枚举 `DayType` 的 `name()`（`DayType` 未覆写 `toString`）。
fn day_type_name(day_type: Option<DayType>) -> &'static str {
    match day_type {
        None => "null",
        Some(DayType::NonWorking) => "NON_WORKING",
        Some(DayType::Working) => "WORKING",
        Some(DayType::Default) => "DEFAULT",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("合法日期")
    }

    fn week() -> ProjectCalendarWeek {
        let mut week = ProjectCalendarWeek::new();
        week.set_date_range(Some(LocalDateRange::new(
            Some(date(2024, 1, 1)),
            Some(date(2024, 1, 7)),
        )));
        week
    }

    #[test]
    fn initial_state() {
        let week = ProjectCalendarWeek::new();
        assert_eq!(week.get_date_range(), None);
        assert_eq!(week.get_name(), None);
        for hours in week.get_calendar_hours() {
            assert!(hours.is_none());
        }
    }

    #[test]
    fn date_range_is_stored() {
        let mut week = ProjectCalendarWeek::new();
        let range = LocalDateRange::new(Some(date(2024, 1, 1)), Some(date(2024, 1, 7)));
        week.set_date_range(Some(range));
        assert_eq!(week.get_date_range(), Some(range));
        assert_eq!(week.get_date_range().and_then(|range| range.get_start()), Some(date(2024, 1, 1)));

        week.set_date_range(None);
        assert_eq!(week.get_date_range(), None);
    }

    #[test]
    fn forwarding_of_project_calendar_days() {
        let mut week = week();
        week.set_name(Some("第一周".to_string()));
        week.add_default_calendar_days();
        week.add_default_calendar_hours();

        assert_eq!(week.get_name(), Some("第一周"));
        assert_eq!(week.get_calendar_day_type(Weekday::Mon), Some(DayType::Working));
        assert_eq!(
            week.get_calendar_hours_for_day(Weekday::Mon)
                .map(ProjectCalendarHours::size),
            Some(2)
        );
        assert_eq!(
            week.get_calendar_hours_for_day(Weekday::Sun)
                .map(ProjectCalendarHours::size),
            Some(0)
        );
        assert_eq!(week.add_calendar_hours(Weekday::Sat).size(), 0);
        week.remove_calendar_hours(Weekday::Sat);
        assert_eq!(week.get_calendar_hours_for_day(Weekday::Sat), None);
    }

    #[test]
    fn default_week_cannot_be_expanded() {
        let week = ProjectCalendarWeek::new();
        assert!(week.convert_to_recurring_exceptions(None, None).is_none());
    }

    #[test]
    fn convert_to_recurring_exceptions_skips_default_days() {
        let mut week = week();
        // 不设置任何 DayType 时全部为 null（非 DEFAULT），7 天都会生成例外
        let exceptions = week
            .convert_to_recurring_exceptions(None, None)
            .expect("工作周可以展开");
        assert_eq!(exceptions.len(), 7);

        // DEFAULT 的星期会被跳过
        week.set_calendar_day_type(Weekday::Mon, DayType::Default);
        let exceptions = week
            .convert_to_recurring_exceptions(None, None)
            .expect("工作周可以展开");
        assert_eq!(exceptions.len(), 6);
        assert!(exceptions.iter().all(|exception| exception.get_recurring().is_some()));
    }

    #[test]
    fn convert_to_recurring_exceptions_copies_hours() {
        let mut week = week();
        week.add_default_calendar_days();
        week.add_default_calendar_hours();

        let exceptions = week
            .convert_to_recurring_exceptions(Some(date(2024, 1, 1)), Some(date(2024, 1, 7)))
            .expect("工作周可以展开");

        assert_eq!(exceptions.len(), 7);
        // 每个例外都带有对应星期的工作时段（周末为空）
        let sizes: Vec<usize> = exceptions.iter().map(ProjectCalendarException::size).collect();
        assert_eq!(sizes, vec![2, 2, 2, 2, 2, 0, 0]);
    }

    #[test]
    fn compare_uses_date_range_start() {
        let first = week();
        let mut second = week();
        second.set_date_range(Some(LocalDateRange::new(
            Some(date(2024, 1, 8)),
            Some(date(2024, 1, 14)),
        )));

        assert!(first < second);
        assert_eq!(first.cmp(&second), Ordering::Less);

        let mut first_copy = week();
        first_copy.set_name(Some("副本".to_string()));
        assert_eq!(first, first_copy);
    }

    #[test]
    fn display_matches_java_format() {
        let mut week = week();
        week.set_name(Some("第一周".to_string()));
        week.set_calendar_day_type(Weekday::Mon, DayType::Working);

        let text = week.to_string();
        assert!(text.starts_with("[ProjectCalendarWeek\n"));
        assert!(text.contains("   name=第一周\n"));
        assert!(text.contains("   date_range=[LocalDateRange start=2024-01-01 end=2024-01-07]\n"));
        assert!(text.contains("   [Day MONDAY\n      type=WORKING\n      hours=null\n   ]\n"));
        assert!(text.contains("   [Day SUNDAY\n      type=null\n      hours=null\n   ]\n"));
        assert!(text.ends_with("]\n]"));
    }
}
