//! 对应 Java: `org/mpxj/LocalDateRange.java`

use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};

use chrono::{Datelike, NaiveDate};

/// 1970-01-01 在 `NaiveDate::num_days_from_ce()` 下的取值（0001-01-01 记为第 1 天），
/// 用于把 chrono 的 CE 日序换算为 Java `LocalDate.toEpochDay()` 的日序。
const UNIX_EPOCH_DAY_FROM_CE: i64 = 719_163;

/// 表示一段时间区间。
///
/// 对应 Java: `public final class LocalDateRange implements Comparable<LocalDateRange>`
///
/// Java 侧的 `m_start`/`m_end` 允许为 `null`，Rust 侧统一用 [`Option`] 表示。
#[derive(Debug, Clone, Copy)]
pub struct LocalDateRange {
    /// 对应 Java 字段 `m_start`
    start: Option<NaiveDate>,
    /// 对应 Java 字段 `m_end`
    end: Option<NaiveDate>,
}

impl LocalDateRange {
    /// 构造方法。
    ///
    /// 对应 Java: `LocalDateRange(LocalDate startDate, LocalDate endDate)`
    pub fn new(start_date: Option<NaiveDate>, end_date: Option<NaiveDate>) -> Self {
        Self {
            start: start_date,
            end: end_date,
        }
    }

    /// 取区间起始日期。
    ///
    /// 对应 Java: `getStart()`
    pub fn get_start(&self) -> Option<NaiveDate> {
        self.start
    }

    /// 取区间结束日期。
    ///
    /// 对应 Java: `getEnd()`
    pub fn get_end(&self) -> Option<NaiveDate> {
        self.end
    }

    /// 比较目标日期与区间：目标日期位于区间内返回 0，早于区间返回负数，晚于区间返回正数。
    ///
    /// 对应 Java: `compareTo(LocalDate date)`（内部调用
    /// `LocalDateHelper.compare(LocalDate startDate, LocalDate endDate, LocalDate targetDate)`）。
    ///
    /// Java 中起始/结束日期为 `null` 时会抛 NPE，Rust 侧退化为返回 0。
    pub fn compare_to_date(&self, date: NaiveDate) -> i32 {
        match (self.start, self.end) {
            (Some(start), Some(end)) => {
                if date < start {
                    -1
                } else if date > end {
                    1
                } else {
                    0
                }
            }
            _ => 0,
        }
    }
}

/// 对应 Java: `LocalDateHelper.compare(LocalDate, LocalDate)`。
///
/// 保留 Java 的 null 语义：`null` 视为「更大」，即排在非 null 日期之后。
fn compare_optional_date(lhs: Option<NaiveDate>, rhs: Option<NaiveDate>) -> Ordering {
    match (lhs, rhs) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(lhs), Some(rhs)) => lhs.cmp(&rhs),
    }
}

impl Ord for LocalDateRange {
    /// 对应 Java: `compareTo(LocalDateRange o)`：先比较起始日期，再比较结束日期。
    fn cmp(&self, other: &Self) -> Ordering {
        let result = compare_optional_date(self.start, other.start);
        if result == Ordering::Equal {
            compare_optional_date(self.end, other.end)
        } else {
            result
        }
    }
}

impl PartialOrd for LocalDateRange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for LocalDateRange {
    /// 对应 Java: `equals(Object o)`：以 `compareTo` 的结果是否为 0 判定。
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for LocalDateRange {}

impl Hash for LocalDateRange {
    /// 对应 Java: `hashCode()`：使用起始/结束日期的 epoch day 异或。
    fn hash<H: Hasher>(&self, state: &mut H) {
        let start = to_epoch_day(self.start);
        let end = to_epoch_day(self.end);
        let hash = ((start as i32) ^ ((start >> 32) as i32)) ^ ((end as i32) ^ ((end >> 32) as i32));
        hash.hash(state);
    }
}

impl Display for LocalDateRange {
    /// 对应 Java: `toString()`：`[LocalDateRange start=... end=...]`，
    /// `null` 字段按 Java 的习惯输出为 `null`。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[LocalDateRange start={} end={}]",
            self.start
                .map_or_else(|| "null".to_string(), |date| date.to_string()),
            self.end
                .map_or_else(|| "null".to_string(), |date| date.to_string())
        )
    }
}

/// 对应 Java: `LocalDate.toEpochDay()`：`null` 视为 0。
fn to_epoch_day(date: Option<NaiveDate>) -> i64 {
    date.map_or(0, |date| {
        date.num_days_from_ce() as i64 - UNIX_EPOCH_DAY_FROM_CE
    })
}
