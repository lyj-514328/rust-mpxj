//! 对应 Java: `org/mpxj/LocalDateTimeRange.java`

use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};

use chrono::NaiveDateTime;

/// 表示一段时间区间。
///
/// 对应 Java: `public final class LocalDateTimeRange implements Comparable<LocalDateTimeRange>`
///
/// Java 侧的 `m_start`/`m_end` 允许为 `null`，Rust 侧统一用 [`Option`] 表示。
#[derive(Debug, Clone, Copy)]
pub struct LocalDateTimeRange {
    /// 对应 Java 字段 `m_start`
    start: Option<NaiveDateTime>,
    /// 对应 Java 字段 `m_end`
    end: Option<NaiveDateTime>,
}

impl LocalDateTimeRange {
    /// 构造方法。
    ///
    /// 对应 Java: `LocalDateTimeRange(LocalDateTime startDate, LocalDateTime endDate)`
    pub fn new(start_date: Option<NaiveDateTime>, end_date: Option<NaiveDateTime>) -> Self {
        Self {
            start: start_date,
            end: end_date,
        }
    }

    /// 取区间起始日期时间。
    ///
    /// 对应 Java: `getStart()`
    pub fn get_start(&self) -> Option<NaiveDateTime> {
        self.start
    }

    /// 取区间结束日期时间。
    ///
    /// 对应 Java: `getEnd()`
    pub fn get_end(&self) -> Option<NaiveDateTime> {
        self.end
    }

    /// 判断本区间是否整体早于给定区间。
    ///
    /// 对应 Java: `isBefore(LocalDateTimeRange range)`
    ///
    /// Java 中字段为 `null` 时会抛 NPE，Rust 侧退化为返回 `false`。
    pub fn is_before(&self, range: &LocalDateTimeRange) -> bool {
        matches!(self.compare_ranges(range), Some(result) if result < 0)
    }

    /// 判断本区间是否整体晚于给定区间。
    ///
    /// 对应 Java: `isAfter(LocalDateTimeRange range)`
    ///
    /// Java 中字段为 `null` 时会抛 NPE，Rust 侧退化为返回 `false`。
    pub fn is_after(&self, range: &LocalDateTimeRange) -> bool {
        matches!(self.compare_ranges(range), Some(result) if result > 0)
    }

    /// 判断本区间是否与给定区间相交。
    ///
    /// 对应 Java: `intersectsWith(LocalDateTimeRange range)`
    ///
    /// Java 中字段为 `null` 时会抛 NPE，Rust 侧退化为返回 `false`。
    pub fn intersects_with(&self, range: &LocalDateTimeRange) -> bool {
        matches!(self.compare_ranges(range), Some(0))
    }

    /// 返回本区间与给定区间的交集，没有交集时返回 `null`（Rust 侧为 [`None`]）。
    ///
    /// 对应 Java: `intersection(LocalDateTimeRange range)`
    pub fn intersection(&self, range: &LocalDateTimeRange) -> Option<LocalDateTimeRange> {
        // 没有交集，返回 null
        if !self.intersects_with(range) {
            return None;
        }

        match (self.start, self.end, range.start, range.end) {
            (Some(this_start), Some(this_end), Some(range_start), Some(range_end)) => {
                // 给定区间完全落在本区间内，直接返回给定区间
                if range_start >= this_start && range_end <= this_end {
                    return Some(*range);
                }

                // 计算交集
                let start = if range_start < this_start {
                    this_start
                } else {
                    range_start
                };
                let end = if range_end > this_end {
                    this_end
                } else {
                    range_end
                };

                // 本区间完全落在给定区间内则返回自身，否则新建交集区间
                if start == this_start && end == this_end {
                    Some(*self)
                } else {
                    Some(LocalDateTimeRange::new(Some(start), Some(end)))
                }
            }
            // intersects_with 为 false 时已经返回，此处不可达
            _ => None,
        }
    }

    /// 比较目标日期时间与区间：位于区间内返回 0，早于区间返回负数，晚于区间返回正数。
    ///
    /// 对应 Java: `compareTo(LocalDateTime date)`（内部调用
    /// `LocalDateTimeHelper.compare(LocalDateTime startDate, LocalDateTime endDate, LocalDateTime targetDate)`）。
    ///
    /// Java 中起始/结束日期时间为 `null` 时会抛 NPE，Rust 侧退化为返回 0。
    pub fn compare_to_date(&self, date: NaiveDateTime) -> i32 {
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

    /// 对应 Java: `LocalDateTimeHelper.compare(LocalDateTime, LocalDateTime,
    /// LocalDateTime, LocalDateTime)`，即两个区间的比较结果：
    /// `-1` 表示本区间在给定区间开始之前结束，`1` 表示本区间在给定区间结束之后开始，
    /// `0` 表示两个区间相交。任一字段为 `null` 时返回 [`None`]（Java 侧会抛 NPE）。
    fn compare_ranges(&self, range: &LocalDateTimeRange) -> Option<i32> {
        match (self.start, self.end, range.start, range.end) {
            (Some(start1), Some(end1), Some(start2), Some(end2)) => {
                Some(compare_range_values(start1, end1, start2, end2))
            }
            _ => None,
        }
    }
}

/// 对应 Java: `LocalDateTimeHelper.compare(LocalDateTime startRange1, LocalDateTime endRange1,
/// LocalDateTime startRange2, LocalDateTime endRange2)`。
fn compare_range_values(
    start_range1: NaiveDateTime,
    end_range1: NaiveDateTime,
    start_range2: NaiveDateTime,
    end_range2: NaiveDateTime,
) -> i32 {
    // 区间 1 在区间 2 开始之前结束
    if end_range1 <= start_range2 {
        return -1;
    }

    // 区间 1 在区间 2 结束之后开始
    if start_range1 >= end_range2 {
        return 1;
    }

    0
}

/// 对应 Java: `LocalDateTimeHelper.compare(LocalDateTime, LocalDateTime)`。
///
/// 保留 Java 的 null 语义：`null` 视为「更大」，即排在非 null 值之后。
fn compare_optional_date_time(lhs: Option<NaiveDateTime>, rhs: Option<NaiveDateTime>) -> Ordering {
    match (lhs, rhs) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(lhs), Some(rhs)) => lhs.cmp(&rhs),
    }
}

/// 对应 Java: `LocalDateTime.toEpochSecond(ZoneOffset.UTC)`：`null` 视为 0。
fn to_epoch_second(date_time: Option<NaiveDateTime>) -> i64 {
    date_time.map_or(0, |date_time| date_time.and_utc().timestamp())
}

impl Ord for LocalDateTimeRange {
    /// 对应 Java: `compareTo(LocalDateTimeRange o)`：先比较起始日期时间，再比较结束日期时间。
    fn cmp(&self, other: &Self) -> Ordering {
        let result = compare_optional_date_time(self.start, other.start);
        if result == Ordering::Equal {
            compare_optional_date_time(self.end, other.end)
        } else {
            result
        }
    }
}

impl PartialOrd for LocalDateTimeRange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for LocalDateTimeRange {
    /// 对应 Java: `equals(Object o)`：以 `compareTo` 的结果是否为 0 判定。
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for LocalDateTimeRange {}

impl Hash for LocalDateTimeRange {
    /// 对应 Java: `hashCode()`：使用起始/结束日期时间的 epoch second 异或。
    fn hash<H: Hasher>(&self, state: &mut H) {
        let start = to_epoch_second(self.start);
        let end = to_epoch_second(self.end);
        let hash = ((start as i32) ^ ((start >> 32) as i32)) ^ ((end as i32) ^ ((end >> 32) as i32));
        hash.hash(state);
    }
}

impl Display for LocalDateTimeRange {
    /// 对应 Java: `toString()`：`[LocalDateTimeRange start=... end=...]`，
    /// `null` 字段输出为 `null`。Java 的 `LocalDateTime.toString()` 用 `T` 分隔
    /// 日期与时间，这里沿用 chrono 的默认格式。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[LocalDateTimeRange start={} end={}]",
            self.start
                .map_or_else(|| "null".to_string(), |date_time| date_time.to_string()),
            self.end
                .map_or_else(|| "null".to_string(), |date_time| date_time.to_string())
        )
    }
}
