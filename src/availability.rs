//! 对应 Java: `org/mpxj/Availability.java`

use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};

use chrono::NaiveDateTime;

use crate::local_date_time_range::LocalDateTimeRange;

/// 资源可用性表（availability table）中的一行。
///
/// 对应 Java: `public final class Availability implements Comparable<Availability>`
///
/// 未移植的 Java 方法：无。
#[derive(Debug, Clone, Copy)]
pub struct Availability {
    /// 可用性区间。
    ///
    /// 对应 Java: `private final LocalDateTimeRange m_range;`
    m_range: LocalDateTimeRange,
    /// 该区间的可用单位数（Java 侧为 `Number`，允许为 `null`）。
    ///
    /// 对应 Java: `private final Number m_units;`
    m_units: Option<f64>,
}

impl Availability {
    /// 构造方法。
    ///
    /// 对应 Java: `public Availability(LocalDateTime startDate, LocalDateTime endDate, Number units)`
    pub fn new(
        start_date: Option<NaiveDateTime>,
        end_date: Option<NaiveDateTime>,
        units: Option<f64>,
    ) -> Self {
        Self {
            m_range: LocalDateTimeRange::new(start_date, end_date),
            m_units: units,
        }
    }

    /// 取可用性区间的日期范围。
    ///
    /// 对应 Java: `public LocalDateTimeRange getRange()`
    pub fn get_range(&self) -> LocalDateTimeRange {
        self.m_range
    }

    /// 取该区间的可用单位数。
    ///
    /// 对应 Java: `public Number getUnits()`
    pub fn get_units(&self) -> Option<f64> {
        self.m_units
    }
}

impl Ord for Availability {
    /// 对应 Java: `@Override public int compareTo(Availability o)`
    ///
    /// 直接比较两个可用性区间的日期范围。
    fn cmp(&self, other: &Self) -> Ordering {
        self.m_range.cmp(&other.m_range)
    }
}

impl PartialOrd for Availability {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Availability {
    /// Java 侧未覆写 `equals`/`hashCode`（沿用对象标识比较）；Rust 侧因 [`Ord`]
    /// 需要 `Eq` 父 trait，这里按「区间与可用单位数都相等」做值比较。
    ///
    /// 说明：该实现与 `compareTo`（只比较区间）在「区间相同、单位数不同」时存在
    /// 语义差异，与 Java 的 `Comparable`/`equals` 契约（本类未实现 `equals`）一致。
    fn eq(&self, other: &Self) -> bool {
        self.m_range == other.m_range && self.m_units == other.m_units
    }
}

/// 说明：`Availability` 含 `f64` 字段，无法自动派生 `Eq`；但 Java 侧实现了
/// `Comparable`，Rust 的 `Ord` 要求 `Eq` 作为父 trait，故这里手工实现 `Eq`。
impl Eq for Availability {}

impl Hash for Availability {
    /// Java 侧未覆写 `hashCode`；Rust 侧为便于把 `Availability` 用作哈希键，
    /// 按区间与可用单位数的位模式计算。
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.m_range.hash(state);
        self.m_units.map(f64::to_bits).hash(state);
    }
}

impl Display for Availability {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// 注意：Java 的 `Number.toString()` 会保留 `.0`（如 `1.0`），Rust 的 `f64`
    /// `Display` 会输出 `1`，二者在本方法上存在细微差异。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[Availability range={} units={}]",
            self.m_range,
            self.m_units
                .map_or_else(|| "null".to_string(), |units| units.to_string())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveTime};

    /// 构造测试用日期时间。
    fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(year, month, day).expect("合法日期"),
            NaiveTime::from_hms_opt(hour, minute, 0).expect("合法时间"),
        )
    }

    #[test]
    fn accessors_return_constructor_values() {
        let start = date_time(2024, 1, 1, 8, 0);
        let end = date_time(2024, 1, 31, 17, 0);
        let value = Availability::new(Some(start), Some(end), Some(0.5));

        assert_eq!(
            value.get_range(),
            LocalDateTimeRange::new(Some(start), Some(end))
        );
        assert_eq!(value.get_units(), Some(0.5));
    }

    #[test]
    fn null_arguments_are_preserved() {
        let value = Availability::new(None, None, None);

        assert_eq!(value.get_range().get_start(), None);
        assert_eq!(value.get_range().get_end(), None);
        assert_eq!(value.get_units(), None);
    }

    #[test]
    fn ordering_follows_range() {
        let earlier = Availability::new(
            Some(date_time(2024, 1, 1, 8, 0)),
            Some(date_time(2024, 1, 31, 17, 0)),
            Some(0.5),
        );
        let later = Availability::new(
            Some(date_time(2024, 2, 1, 8, 0)),
            Some(date_time(2024, 2, 28, 17, 0)),
            Some(0.5),
        );

        assert!(earlier < later);
        assert_eq!(earlier.cmp(&earlier.clone()), Ordering::Equal);
    }

    #[test]
    fn equality_compares_range_and_units() {
        let start = Some(date_time(2024, 1, 1, 8, 0));
        let end = Some(date_time(2024, 1, 31, 17, 0));
        let lhs = Availability::new(start, end, Some(0.5));
        let same = Availability::new(start, end, Some(0.5));
        let other_units = Availability::new(start, end, Some(0.25));

        assert_eq!(lhs, same);
        assert_ne!(lhs, other_units);
    }

    #[test]
    fn display_matches_java_format() {
        let value = Availability::new(
            Some(date_time(2024, 1, 1, 8, 0)),
            Some(date_time(2024, 1, 31, 17, 0)),
            Some(0.5),
        );

        assert_eq!(
            value.to_string(),
            "[Availability range=[LocalDateTimeRange start=2024-01-01 08:00:00 end=2024-01-31 17:00:00] units=0.5]"
        );
    }
}
