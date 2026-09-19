//! 对应 Java: `org/mpxj/LocalTimeRange.java`

use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};

use chrono::{NaiveTime, Timelike};

/// 每天的毫秒数，对应 Java `LocalTimeHelper.MS_PER_DAY`。
const MS_PER_DAY: i64 = 24 * 60 * 60 * 1000;

/// 表示一段时间区间。
///
/// 对应 Java: `public final class LocalTimeRange implements Comparable<LocalTimeRange>`
///
/// Java 侧的 `m_start`/`m_end` 允许为 `null`（见 `EMPTY_RANGE`），Rust 侧统一用
/// [`Option`] 表示。
#[derive(Debug, Clone, Copy)]
pub struct LocalTimeRange {
    /// 对应 Java 字段 `m_start`
    start: Option<NaiveTime>,
    /// 对应 Java 字段 `m_end`
    end: Option<NaiveTime>,
    /// 对应 Java 字段 `m_durationAsMilliseconds`，构造时由
    /// `LocalTimeHelper.getMillisecondsInRange` 计算得到
    duration_as_milliseconds: i64,
}

impl LocalTimeRange {
    /// 空区间。
    ///
    /// 对应 Java: `public static final LocalTimeRange EMPTY_RANGE = new LocalTimeRange(null, null)`
    pub const EMPTY_RANGE: LocalTimeRange = LocalTimeRange {
        start: None,
        end: None,
        duration_as_milliseconds: 0,
    };

    /// 构造方法，同时计算区间时长。
    ///
    /// 对应 Java: `LocalTimeRange(LocalTime start, LocalTime end)`
    pub fn new(start: Option<NaiveTime>, end: Option<NaiveTime>) -> Self {
        Self {
            start,
            end,
            duration_as_milliseconds: get_milliseconds_in_range(start, end),
        }
    }

    /// 取区间起始时间。
    ///
    /// 对应 Java: `getStart()`
    pub fn get_start(&self) -> Option<NaiveTime> {
        self.start
    }

    /// 取区间结束时间。
    ///
    /// 对应 Java: `getEnd()`
    pub fn get_end(&self) -> Option<NaiveTime> {
        self.end
    }

    /// 取区间时长（毫秒）。
    ///
    /// 对应 Java: `getDurationAsMilliseconds()`
    pub fn get_duration_as_milliseconds(&self) -> i64 {
        self.duration_as_milliseconds
    }
}

/// 计算两个时间之间的毫秒数，保留 Java 侧的 null 处理。
///
/// 对应 Java: `LocalTimeHelper.getMillisecondsInRange(LocalTime rangeStart, LocalTime rangeEnd)`
///
/// Java 用 `rangeEnd == LocalTime.MIDNIGHT` 判断跨零点，Rust 侧按值等价处理。
fn get_milliseconds_in_range(range_start: Option<NaiveTime>, range_end: Option<NaiveTime>) -> i64 {
    match (range_start, range_end) {
        (Some(start), Some(end)) => {
            if end == NaiveTime::MIN {
                MS_PER_DAY - start.num_seconds_from_midnight() as i64 * 1000
            } else {
                (end.num_seconds_from_midnight() as i64
                    - start.num_seconds_from_midnight() as i64)
                    * 1000
            }
        }
        _ => 0,
    }
}

/// 对应 Java: `LocalTimeHelper.compare(LocalTime, LocalTime)`。
///
/// 保留 Java 的 null 语义：`null` 视为「更大」，即排在非 null 时间之后。
fn compare_optional_time(lhs: Option<NaiveTime>, rhs: Option<NaiveTime>) -> Ordering {
    match (lhs, rhs) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(lhs), Some(rhs)) => lhs.cmp(&rhs),
    }
}

/// 对应 Java: `LocalTime.toNanoOfDay()`：`null` 视为 0。
fn to_nano_of_day(time: Option<NaiveTime>) -> i64 {
    time.map_or(0, |time| {
        time.num_seconds_from_midnight() as i64 * 1_000_000_000 + time.nanosecond() as i64
    })
}

impl Ord for LocalTimeRange {
    /// 对应 Java: `compareTo(LocalTimeRange o)`：先比较起始时间，
    /// 起始时间相同时用 `Long.compare` 比较时长。
    fn cmp(&self, other: &Self) -> Ordering {
        let result = compare_optional_time(self.start, other.start);
        if result == Ordering::Equal {
            self.duration_as_milliseconds
                .cmp(&other.duration_as_milliseconds)
        } else {
            result
        }
    }
}

impl PartialOrd for LocalTimeRange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for LocalTimeRange {
    /// 对应 Java: `equals(Object o)`：以 `compareTo` 的结果是否为 0 判定。
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for LocalTimeRange {}

impl Hash for LocalTimeRange {
    /// 对应 Java: `hashCode()`：使用起始/结束时间的 nanoOfDay 异或。
    fn hash<H: Hasher>(&self, state: &mut H) {
        let start = to_nano_of_day(self.start);
        let end = to_nano_of_day(self.end);
        let hash = ((start as i32) ^ ((start >> 32) as i32)) ^ ((end as i32) ^ ((end >> 32) as i32));
        hash.hash(state);
    }
}

impl Display for LocalTimeRange {
    /// 对应 Java: `toString()`：`[LocalTimeRange start=... end=...]`，
    /// `null` 字段按 Java 的习惯输出为 `null`。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[LocalTimeRange start={} end={}]",
            self.start
                .map_or_else(|| "null".to_string(), |time| time.to_string()),
            self.end
                .map_or_else(|| "null".to_string(), |time| time.to_string())
        )
    }
}
