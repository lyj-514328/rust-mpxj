//! 对应 Java: `org/mpxj/ProjectCalendarException.java`
//!
//! 跳过的 Java 方法：无。
//!
//! # 依赖说明（重要）
//!
//! 本文件依赖 `org.mpxj.RecurringData`（对应 Rust 模块 `crate::recurring_data`）。
//! 该类型当前尚未移植，`src/lib.rs` 中也还未声明 `pub mod recurring_data;`，
//! 因此本文件目前会产生「未声明的模块」编译错误，需要在本模块落地后补上模块声明。
//! 本文件按下列（由 Java getter/setter 一一对应的）签名使用 `RecurringData`：
//!
//! ```text
//! RecurringData::new() -> RecurringData
//! set_recurrence_type(Option<RecurrenceType>)
//! get_recurrence_type() -> Option<RecurrenceType>
//! get_frequency() -> Option<i32>          // Java: Integer getFrequency()
//! get_calculated_first_date() -> NaiveDate // Java: LocalDate getCalculatedFirstDate()
//! get_calculated_last_date() -> NaiveDate  // Java: LocalDate getCalculatedLastDate()
//! get_dates() -> Vec<NaiveDate>            // Java: LocalDate[] getDates()
//! impl Display                             // Java: toString()
//! ```
//!
//! # 与 Java 的差异
//!
//! - Java 的 `ProjectCalendarException extends ProjectCalendarHours`（进而 `AbstractList`），
//!   Rust 无继承：[`ProjectCalendarException`] 以字段组合持有 [`ProjectCalendarHours`]，
//!   并把基类的集合方法逐个转发出来（`add` / `set` / `get` / `iter` / `size` / `clear` / `add_all`）。
//! - `equals` / `hashCode` 在 Java 侧来自 `AbstractList`（只比较列表内容），
//!   而 `compareTo` 比较的是起始日期。Java 本身即存在这种 `equals` 与 `compareTo`
//!   不一致：Rust 侧保持同样语义 —— `PartialEq` / `Hash` 只看工作时段内容，
//!   `Ord` 只看起始日期。
//! - Java 的 `contains(LocalDateTime)` / `contains(LocalDate)` 在入参为 `null` 时返回
//!   `false`；Rust 侧参数不可为 `null`，该分支不可达。若起止日期为 `null`（Java 会抛
//!   NPE），Rust 侧返回 `false`。

use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::slice::Iter;

use chrono::{NaiveDate, NaiveDateTime};

use crate::common::local_date_helper::LocalDateHelper;
use crate::local_time_range::LocalTimeRange;
use crate::project_calendar_hours::ProjectCalendarHours;
use crate::recurrence_type::RecurrenceType;
use crate::recurring_data::RecurringData;

/// 日历例外日期（可含重复规则），用于描述基日历/资源日历中的例外。
///
/// 对应 Java: `public final class ProjectCalendarException extends ProjectCalendarHours implements Comparable<ProjectCalendarException>`
#[derive(Debug, Clone)]
pub struct ProjectCalendarException {
    /// 对应 Java 的继承关系 `extends ProjectCalendarHours`
    hours: ProjectCalendarHours,
    /// 对应 Java 字段 `private final LocalDate m_fromDate`
    from_date: Option<NaiveDate>,
    /// 对应 Java 字段 `private final LocalDate m_toDate`
    to_date: Option<NaiveDate>,
    /// 对应 Java 字段 `private final RecurringData m_recurring`
    recurring: Option<RecurringData>,
    /// 对应 Java 字段 `private String m_name`
    name: Option<String>,
}

impl ProjectCalendarException {
    /// 构造方法（仅日期区间）。
    ///
    /// 对应 Java: `ProjectCalendarException(LocalDate fromDate, LocalDate toDate)`
    pub fn new(from_date: Option<NaiveDate>, to_date: Option<NaiveDate>) -> Self {
        Self::new_with_recurring(from_date, to_date, None)
    }

    /// 构造方法（仅重复规则）。
    ///
    /// 对应 Java: `ProjectCalendarException(RecurringData recurringData)`
    ///
    /// Java 通过构造方法重载表达，Rust 无法重载，故加 `_from_recurring` 后缀区分。
    pub fn new_from_recurring(recurring: RecurringData) -> Self {
        Self::new_with_recurring(None, None, Some(recurring))
    }

    /// 构造方法（日期区间 + 重复规则）。
    ///
    /// 对应 Java: `ProjectCalendarException(LocalDate fromDate, LocalDate toDate, RecurringData recurringData)`
    pub fn new_with_recurring(
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
        recurring: Option<RecurringData>,
    ) -> Self {
        Self {
            hours: ProjectCalendarHours::new(),
            from_date,
            to_date,
            recurring,
            name: None,
        }
    }

    /// 取例外名称。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// 设置例外名称。
    ///
    /// 对应 Java: `public void setName(String name)`
    pub fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }

    /// 取起始日期，重复例外时取计算出的首次发生日期。
    ///
    /// 对应 Java: `public LocalDate getFromDate()`
    pub fn get_from_date(&self) -> Option<NaiveDate> {
        match &self.recurring {
            None => self.from_date,
            Some(recurring) => Some(recurring.get_calculated_first_date()),
        }
    }

    /// 取结束日期，重复例外时取计算出的最后一次发生日期。
    ///
    /// 对应 Java: `public LocalDate getToDate()`
    pub fn get_to_date(&self) -> Option<NaiveDate> {
        match &self.recurring {
            None => self.to_date,
            Some(recurring) => Some(recurring.get_calculated_last_date()),
        }
    }

    /// 取重复规则数据（非重复例外时为 `None`）。
    ///
    /// 对应 Java: `public RecurringData getRecurring()`
    pub fn get_recurring(&self) -> Option<&RecurringData> {
        self.recurring.as_ref()
    }

    /// 是否有工作时段（即 `size() != 0`）。
    ///
    /// 对应 Java: `public boolean getWorking()`
    pub fn get_working(&self) -> bool {
        self.hours.size() != 0
    }

    /// 把本例外展开为例外列表。
    ///
    /// 对应 Java: `public List<ProjectCalendarException> getExpandedExceptions()`
    ///
    /// 非重复例外、或「每日重复且频率为 1」的例外只返回自身；其余重复例外按
    /// 计算出的每个日期各生成一个单日例外，并复制全部工作时段。
    pub fn get_expanded_exceptions(&self) -> Vec<ProjectCalendarException> {
        let mut result = Vec::new();

        match &self.recurring {
            None => result.push(self.clone()),
            Some(recurring) => {
                // Java: m_recurring.getRecurrenceType() == DAILY && NumberHelper.getInt(m_recurring.getFrequency()) == 1
                let single_day = recurring.get_recurrence_type() == Some(RecurrenceType::Daily)
                    && recurring
                        .get_frequency()
                        .map_or(0, |frequency| frequency as i32)
                        == 1;

                if single_day {
                    result.push(self.clone());
                } else {
                    let range_count = self.hours.size();
                    for date in recurring.get_dates().iter().copied() {
                        let mut new_exception =
                            ProjectCalendarException::new(Some(date), Some(date));
                        for range_index in 0..range_count {
                            new_exception.add(self.hours.get(range_index));
                        }
                        result.push(new_exception);
                    }
                }
            }
        }

        result
    }

    /// 判断给定日期时间是否落在本例外覆盖的日期区间内。
    ///
    /// 对应 Java: `public boolean contains(LocalDateTime date)`
    ///
    /// Java 侧与 `contains(LocalDate)` 实现相同（先取 `LocalDate` 再比较），此处直接复用。
    pub fn contains_date_time(&self, date: NaiveDateTime) -> bool {
        self.contains_date(date.date())
    }

    /// 判断给定日期是否落在本例外覆盖的日期区间内。
    ///
    /// 对应 Java: `public boolean contains(LocalDate date)`
    pub fn contains_date(&self, date: NaiveDate) -> bool {
        match (self.get_from_date(), self.get_to_date()) {
            (Some(from_date), Some(to_date)) => {
                LocalDateHelper::compare_with_range(from_date, to_date, date) == 0
            }
            // Java: 起止日期为 null 时会抛 NPE，Rust 侧退化为 false
            _ => false,
        }
    }

    /// 判断给定例外是否与本例外有任何日期重叠。
    ///
    /// 对应 Java: `public boolean contains(ProjectCalendarException exception)`
    ///
    /// Java 通过重载提供三种 `contains`，Rust 无重载，故分别命名为
    /// `contains_date_time` / `contains_date` / `contains_exception`。
    /// 空值语义与 Java `LocalDateHelper.compare` 一致（`null` 视为更大）。
    pub fn contains_exception(&self, exception: &ProjectCalendarException) -> bool {
        !(LocalDateHelper::compare(self.get_to_date(), exception.get_from_date()) < 0
            || LocalDateHelper::compare(exception.get_to_date(), self.get_from_date()) < 0)
    }

    /// 取本例外内部的工作时段集合（对应 Java 的 `extends ProjectCalendarHours` 关系）。
    ///
    /// 对应 Java: `this`（Rust 侧以访问器表达被拍平的继承关系）
    pub fn get_hours(&self) -> &ProjectCalendarHours {
        &self.hours
    }

    //
    // 以下方法对应 Java 从 ProjectCalendarHours 继承而来的集合操作
    //

    /// 对应 Java: `add(LocalTimeRange range)`（继承自 `ProjectCalendarHours`）
    pub fn add(&mut self, range: LocalTimeRange) -> bool {
        self.hours.add(range)
    }

    /// 对应 Java: `set(int index, LocalTimeRange value)`（继承自 `ProjectCalendarHours`）
    pub fn set(&mut self, index: usize, value: LocalTimeRange) -> Option<LocalTimeRange> {
        self.hours.set(index, value)
    }

    /// 对应 Java: `get(int index)`（继承自 `ProjectCalendarHours`）
    pub fn get(&self, index: usize) -> LocalTimeRange {
        self.hours.get(index)
    }

    /// 对应 Java: `iterator()`（继承自 `ProjectCalendarHours`）
    pub fn iter(&self) -> Iter<'_, LocalTimeRange> {
        self.hours.iter()
    }

    /// 对应 Java: `size()`（继承自 `ProjectCalendarHours`）
    pub fn size(&self) -> usize {
        self.hours.size()
    }

    /// 对应 Java: `clear()`（继承自 `ProjectCalendarHours`）
    pub fn clear(&mut self) {
        self.hours.clear();
    }

    /// 对应 Java: `addAll(Collection)`（继承自 `ProjectCalendarHours`）
    pub fn add_all(&mut self, other: &ProjectCalendarHours) -> bool {
        self.hours.add_all(other)
    }
}

impl<'a> IntoIterator for &'a ProjectCalendarException {
    type Item = &'a LocalTimeRange;
    type IntoIter = Iter<'a, LocalTimeRange>;

    fn into_iter(self) -> Self::IntoIter {
        self.hours.iter()
    }
}

impl PartialEq for ProjectCalendarException {
    /// 对应 Java: `AbstractList.equals(Object)`：只比较工作时段内容。
    fn eq(&self, other: &Self) -> bool {
        self.hours == other.hours
    }
}

impl Eq for ProjectCalendarException {}

impl Hash for ProjectCalendarException {
    /// 对应 Java: `AbstractList.hashCode()`：只按工作时段内容计算哈希。
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hours.hash(state);
    }
}

impl Ord for ProjectCalendarException {
    /// 对应 Java: `@Override public int compareTo(ProjectCalendarException o)`：
    /// 按起始日期比较（`null` 视为更大）。
    fn cmp(&self, other: &Self) -> Ordering {
        match LocalDateHelper::compare(self.get_from_date(), other.get_from_date()) {
            value if value < 0 => Ordering::Less,
            0 => Ordering::Equal,
            _ => Ordering::Greater,
        }
    }
}

impl PartialOrd for ProjectCalendarException {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for ProjectCalendarException {
    /// 对应 Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[ProjectCalendarException")?;

        if let Some(name) = &self.name {
            if !name.is_empty() {
                write!(f, " name={}", name)?;
            }
        }

        write!(f, " working={}", self.get_working())?;

        match &self.recurring {
            None => {
                write!(f, " fromDate={}", optional_date(self.from_date))?;
                write!(f, " toDate={}", optional_date(self.to_date))?;
            }
            Some(recurring) => write!(f, " recurring={}", recurring)?,
        }

        for range in &self.hours {
            write!(f, "{}", range)?;
        }

        write!(f, "]")
    }
}

/// 按 Java 习惯输出可空日期（`null` → `null`）。
fn optional_date(date: Option<NaiveDate>) -> String {
    date.map_or_else(|| "null".to_string(), |date| date.to_string())
}

#[cfg(test)]
mod tests {
    use chrono::NaiveTime;

    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("合法日期")
    }

    fn range(from: u32, to: u32) -> LocalTimeRange {
        LocalTimeRange::new(
            NaiveTime::from_hms_opt(from, 0, 0),
            NaiveTime::from_hms_opt(to, 0, 0),
        )
    }

    #[test]
    fn single_day_exception() {
        let mut exception = ProjectCalendarException::new(Some(date(2024, 1, 1)), Some(date(2024, 1, 1)));

        assert_eq!(exception.get_from_date(), Some(date(2024, 1, 1)));
        assert_eq!(exception.get_to_date(), Some(date(2024, 1, 1)));
        assert!(exception.get_recurring().is_none());
        assert!(!exception.get_working());

        exception.add(range(9, 12));
        assert!(exception.get_working());
        assert_eq!(exception.size(), 1);
        assert_eq!(exception.get(0), range(9, 12));
    }

    #[test]
    fn contains_date_and_date_time() {
        let exception = ProjectCalendarException::new(Some(date(2024, 1, 8)), Some(date(2024, 1, 12)));

        assert!(exception.contains_date(date(2024, 1, 8)));
        assert!(exception.contains_date(date(2024, 1, 10)));
        assert!(exception.contains_date(date(2024, 1, 12)));
        assert!(!exception.contains_date(date(2024, 1, 7)));
        assert!(!exception.contains_date(date(2024, 1, 13)));

        let date_time = date(2024, 1, 10)
            .and_hms_opt(9, 30, 0)
            .expect("合法日期时间");
        assert!(exception.contains_date_time(date_time));
        assert!(!exception.contains_date_time(
            date(2024, 1, 20)
                .and_hms_opt(9, 30, 0)
                .expect("合法日期时间")
        ));

        // 起止日期缺失时（Java 抛 NPE）Rust 返回 false
        let empty = ProjectCalendarException::new(None, None);
        assert!(!empty.contains_date(date(2024, 1, 8)));
    }

    #[test]
    fn contains_exception_detects_overlap() {
        let first = ProjectCalendarException::new(Some(date(2024, 1, 1)), Some(date(2024, 1, 10)));
        let second = ProjectCalendarException::new(Some(date(2024, 1, 10)), Some(date(2024, 1, 20)));
        let third = ProjectCalendarException::new(Some(date(2024, 1, 11)), Some(date(2024, 1, 20)));

        assert!(first.contains_exception(&second));
        assert!(second.contains_exception(&first));
        assert!(!first.contains_exception(&third));
        assert!(!third.contains_exception(&first));
    }

    #[test]
    fn compare_by_from_date() {
        let first = ProjectCalendarException::new(Some(date(2024, 1, 1)), Some(date(2024, 1, 10)));
        let second = ProjectCalendarException::new(Some(date(2024, 1, 10)), Some(date(2024, 1, 20)));

        assert!(first < second);
        assert_eq!(first.cmp(&first), Ordering::Equal);
    }

    #[test]
    fn expansion_of_non_recurring_exception_returns_self() {
        let mut exception = ProjectCalendarException::new(Some(date(2024, 1, 1)), Some(date(2024, 1, 3)));
        exception.add(range(9, 12));

        let expanded = exception.get_expanded_exceptions();
        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded[0].get_from_date(), Some(date(2024, 1, 1)));
        assert_eq!(expanded[0].size(), 1);
    }

    #[test]
    fn expansion_of_daily_single_frequency_returns_self() {
        let mut recurring = RecurringData::new();
        recurring.set_recurrence_type(Some(RecurrenceType::Daily));
        recurring.set_frequency(Some(1));

        let mut exception = ProjectCalendarException::new_from_recurring(recurring);
        exception.add(range(9, 12));

        let expanded = exception.get_expanded_exceptions();
        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded[0].size(), 1);
    }

    #[test]
    fn display_matches_java_format() {
        let mut exception = ProjectCalendarException::new(Some(date(2024, 1, 1)), Some(date(2024, 1, 1)));
        assert_eq!(
            exception.to_string(),
            "[ProjectCalendarException working=false fromDate=2024-01-01 toDate=2024-01-01]"
        );

        exception.set_name(Some("春节".to_string()));
        exception.add(range(8, 12));
        assert_eq!(
            exception.to_string(),
            "[ProjectCalendarException name=春节 working=true fromDate=2024-01-01 toDate=2024-01-01[LocalTimeRange start=08:00:00 end=12:00:00]]"
        );
    }

    #[test]
    fn equality_and_hash_follow_abstract_list_semantics() {
        let mut lhs = ProjectCalendarException::new(Some(date(2024, 1, 1)), Some(date(2024, 1, 1)));
        lhs.add(range(9, 12));

        let mut rhs = ProjectCalendarException::new(Some(date(2024, 2, 1)), Some(date(2024, 2, 1)));
        rhs.add(range(9, 12));

        // Java: equals 来自 AbstractList，只比较列表内容
        assert_eq!(lhs, rhs);

        rhs.clear();
        assert_ne!(lhs, rhs);
    }
}
