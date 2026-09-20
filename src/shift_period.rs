//! 对应 Java: `org/mpxj/ShiftPeriod.java`
//!
//! 跳过的 Java 方法：无。
//!
//! # 与 Java 的差异
//!
//! - **唯一 ID 分配**：Java 的构造方法通过
//!   `UniqueIdObjectSequenceProvider#getUniqueIdObjectSequence(ShiftPeriod.class).syncOrGetNext(...)`
//!   分配/同步唯一 ID。`ObjectSequence`、`UniqueIdObjectSequenceProvider` 均已落地，但本仓
//!   的 [`Builder`] 尚未接线，仍直接使用调用方通过 [`Builder::unique_id`] 传入的值；
//!   `ShiftPeriod` 也未实现 `ProjectEntityWithUniqueID`（该 trait 已落地，属待接线项，
//!   见 `TECH_DEBT.md` 六），但仍提供 `get_unique_id()`。
//! - **父班次引用**：Java 用强引用 `Shift m_parentShift` 回指父班次（构造时还会把自身
//!   加入父班次的 `TreeSet`）。Rust 侧用 [`Weak`] 弱引用表达回指（避免与
//!   `Shift.periods` 形成循环引用），[`ShiftPeriod::get_parent_shift`] 在父班次已释放时
//!   返回 `None`。
//! - **构造方法**：Java 的构造方法是私有的，只能通过 [`Builder`] 创建；Rust 侧同样
//!   不提供公开构造方法，`Builder::build` 会按 Java 语义把新建时段注册进父班次。
//! - `equals` / `hashCode`：Java 未覆写（引用相等），但 `TreeSet` 使用 `compareTo`
//!   判断重复。Rust 侧 `PartialEq` / `Hash` 与 `Ord`（按开始时间）保持一致，
//!   以便 `BTreeSet` 与 Java 的 `TreeSet` 去重行为完全一致。
//! - `getDuration()` / `compareTo()` 在 `start` 为 `null` 时 Java 会抛 NPE，
//!   Rust 侧退化为：时长为 0 小时、`null` 视为排序中更大（与
//!   `LocalTimeHelper.compare` 的空值语义一致）。

use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::rc::{Rc, Weak};

use chrono::NaiveTime;

use crate::duration::Duration;
use crate::shift::Shift;
use crate::time_unit::TimeUnit;

/// 资源班次中的一个时段。
///
/// 对应 Java: `public final class ShiftPeriod implements ProjectEntityWithUniqueID, Comparable<ShiftPeriod>`
#[derive(Debug, Clone)]
pub struct ShiftPeriod {
    /// 对应 Java 字段 `private final Integer m_uniqueID`
    unique_id: Option<i32>,
    /// 对应 Java 字段 `private final Shift m_parentShift`（Rust 侧为弱引用）
    parent_shift: Weak<Shift>,
    /// 对应 Java 字段 `private final LocalTime m_start`
    start: Option<NaiveTime>,
}

impl ShiftPeriod {
    /// 取父班次（父班次已释放时返回 `None`）。
    ///
    /// 对应 Java: `public Shift getParentShift()`
    pub fn get_parent_shift(&self) -> Option<Rc<Shift>> {
        self.parent_shift.upgrade()
    }

    /// 取时段唯一 ID。
    ///
    /// 对应 Java: `@Override public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.unique_id
    }

    /// 取时段开始时间。
    ///
    /// 对应 Java: `public LocalTime getStart()`
    pub fn get_start(&self) -> Option<NaiveTime> {
        self.start
    }

    /// 计算本时段的时长。
    ///
    /// 对应 Java: `public Duration getDuration()`
    ///
    /// 时长 = 下一个时段的开始时间 - 本时段开始时间；没有更晚的时段时以零点为界
    /// （跨零点时为 `24 + 差值`），与 Java `LocalTime.until(..., HOURS)` 的结果一致。
    pub fn get_duration(&self) -> Duration {
        let start = match self.start {
            Some(start) => start,
            // Java: start 为 null 时抛 NPE，Rust 侧退化为 0 小时
            None => return Duration::get_instance(0.0, TimeUnit::Hours),
        };

        // Java: m_parentShift.getPeriods().stream().filter(p -> p.m_start.isAfter(m_start))
        //       .findFirst().map(p -> p.m_start).orElse(LocalTime.MIDNIGHT)
        // 集合已按开始时间有序，取“大于本时段开始时间”的最小值等价于 findFirst。
        let next_start = self
            .parent_shift
            .upgrade()
            .map(|shift| shift.get_periods())
            .unwrap_or_default()
            .iter()
            .filter_map(|period| period.start)
            .filter(|period_start| *period_start > start)
            .min()
            .unwrap_or(NaiveTime::MIN);

        let mut hours = next_start.signed_duration_since(start).num_hours();
        if hours < 0 {
            hours += 24;
        }

        Duration::get_instance(hours as f64, TimeUnit::Hours)
    }
}

/// 对应 Java: `LocalTime.compareTo`，`null` 视为「更大」。
fn compare_optional_time(lhs: Option<NaiveTime>, rhs: Option<NaiveTime>) -> Ordering {
    match (lhs, rhs) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(lhs), Some(rhs)) => lhs.cmp(&rhs),
    }
}

impl Ord for ShiftPeriod {
    /// 对应 Java: `@Override public int compareTo(ShiftPeriod o)`：按开始时间比较。
    fn cmp(&self, other: &Self) -> Ordering {
        compare_optional_time(self.start, other.start)
    }
}

impl PartialOrd for ShiftPeriod {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ShiftPeriod {
    /// 说明：Java 未覆写 `equals`，但 `TreeSet` 用 `compareTo` 去重；Rust 侧与 `Ord`
    /// 保持一致，保证 `BTreeSet` 的去重语义与 Java 相同。
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for ShiftPeriod {}

impl Hash for ShiftPeriod {
    /// 与 [`ShiftPeriod::eq`] 保持一致：只使用开始时间。
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.start.hash(state);
    }
}

/// 班次时段构造器。
///
/// 对应 Java: `public static class ShiftPeriod.Builder`
///
/// Java 的 `Builder` 方法原地修改并返回 `this`；Rust 侧采用与
/// `crate::unit_of_measure::Builder` 一致的「消耗自身并返回 `Self`」写法。
#[derive(Debug)]
pub struct Builder {
    /// 对应 Java 字段 `private final Shift m_shift`
    shift: Rc<Shift>,
    /// 对应 Java 字段 `private Integer m_uniqueID`
    unique_id: Option<i32>,
    /// 对应 Java 字段 `private LocalTime m_start`
    start: Option<NaiveTime>,
}

impl Builder {
    /// 构造方法。
    ///
    /// 对应 Java: `public Builder(UniqueIdObjectSequenceProvider sequenceProvider, Shift shift)`
    ///
    /// Java 的序列提供者用于经 `ObjectSequence` 分配 unique ID；本仓尚未接线
    /// （`UniqueIdObjectSequenceProvider` 已落地），故不接收该参数（见文件头说明）。
    pub fn new(shift: &Rc<Shift>) -> Self {
        Self {
            shift: Rc::clone(shift),
            unique_id: None,
            start: None,
        }
    }

    /// 设置唯一 ID。
    ///
    /// 对应 Java: `public Builder uniqueID(Integer value)`
    pub fn unique_id(mut self, value: Option<i32>) -> Self {
        self.unique_id = value;
        self
    }

    /// 设置开始时间。
    ///
    /// 对应 Java: `public Builder start(LocalTime value)`
    pub fn start(mut self, value: Option<NaiveTime>) -> Self {
        self.start = value;
        self
    }

    /// 以小时间隔设置开始时间（分钟、秒为 0）。
    ///
    /// 对应 Java: `public Builder start(Integer value)`（`LocalTime.of(value, 0)`）
    ///
    /// Rust 无法重载，故加 `_from_hour` 后缀。Java 对越界小时抛
    /// `DateTimeException`，Rust 侧退化为未设置开始时间（`None`）。
    pub fn start_from_hour(mut self, value: i32) -> Self {
        self.start = NaiveTime::from_hms_opt(value as u32, 0, 0);
        self
    }

    /// 构造 [`ShiftPeriod`] 实例，并把该时段注册进父班次。
    ///
    /// 对应 Java: `public ShiftPeriod build()`
    ///
    /// Java 返回的对象与注册进父班次集合的对象是同一个（引用语义）；
    /// Rust 侧 `ShiftPeriod` 是值类型，返回克隆值，二者字段与环境等价。
    pub fn build(self) -> ShiftPeriod {
        let period = ShiftPeriod {
            unique_id: self.unique_id,
            parent_shift: Rc::downgrade(&self.shift),
            start: self.start,
        };

        // Java: ShiftPeriod 构造方法中的 m_parentShift.getPeriods().add(this)
        self.shift.add_period(period.clone());

        period
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shift::Builder as ShiftBuilder;

    fn time(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).expect("合法时间")
    }

    fn start(hour: u32) -> Option<NaiveTime> {
        NaiveTime::from_hms_opt(hour, 0, 0)
    }

    fn hours(value: f64) -> Duration {
        Duration::get_instance(value, TimeUnit::Hours)
    }

    #[test]
    fn period_is_registered_in_parent_shift() {
        let shift = ShiftBuilder::new().name(Some("早班".to_string())).build();
        let period = Builder::new(&shift).unique_id(Some(1)).start(start(8)).build();

        assert_eq!(period.get_unique_id(), Some(1));
        assert_eq!(period.get_start(), start(8));
        assert_eq!(shift.get_periods().len(), 1);
        assert_eq!(
            period.get_parent_shift().map(|shift| shift.get_name().map(str::to_string)),
            Some(Some("早班".to_string()))
        );
    }

    #[test]
    fn duration_is_time_to_next_period() {
        let shift = ShiftBuilder::new().build();
        let morning = Builder::new(&shift).start(start(8)).build();
        let evening = Builder::new(&shift).start(start(20)).build();

        assert_eq!(morning.get_duration(), hours(12.0));
        assert_eq!(evening.get_duration(), hours(4.0));
    }

    #[test]
    fn duration_of_single_period_wraps_to_midnight() {
        let shift = ShiftBuilder::new().build();
        let period = Builder::new(&shift).start(start(8)).build();
        assert_eq!(period.get_duration(), hours(16.0));
    }

    #[test]
    fn duration_of_midnight_period_without_later_period_is_zero() {
        let shift = ShiftBuilder::new().build();
        let midnight = Builder::new(&shift).start(start(0)).build();
        // Java: m_start.until(LocalTime.MIDNIGHT, HOURS) == 0
        assert_eq!(midnight.get_duration(), hours(0.0));

        let next = Builder::new(&shift).start(start(0)).build();
        // 与上一个时段开始时间相同，被 BTreeSet 去重（Java TreeSet 行为一致）
        assert_eq!(next.get_duration(), hours(0.0));
        assert_eq!(shift.get_periods().len(), 1);
    }

    #[test]
    fn duration_uses_start_based_ordering() {
        let shift = ShiftBuilder::new().build();
        Builder::new(&shift).start(start(6)).build();
        Builder::new(&shift).start(start(14)).build();
        let night = Builder::new(&shift).start(start(22)).build();

        // 22:00 之后没有更晚的时段，跨零点到次日 00:00 共 2 小时
        assert_eq!(night.get_duration(), hours(2.0));
    }

    #[test]
    fn null_start_degrades_gracefully() {
        let shift = ShiftBuilder::new().build();
        let period = Builder::new(&shift).build();

        assert_eq!(period.get_start(), None);
        // Java: 抛 NPE，Rust 侧退化为 0 小时
        assert_eq!(period.get_duration(), hours(0.0));
    }

    #[test]
    fn start_from_hour_rejects_invalid_hour() {
        let shift = ShiftBuilder::new().build();
        let valid = Builder::new(&shift).start_from_hour(9).build();
        assert_eq!(valid.get_start(), Some(time(9, 0)));

        let invalid = Builder::new(&shift).start_from_hour(25).build();
        assert_eq!(invalid.get_start(), None);
    }

    #[test]
    fn ordering_follows_start_time() {
        let shift = ShiftBuilder::new().build();
        let morning = Builder::new(&shift).start(start(8)).build();
        let afternoon = Builder::new(&shift).start(start(13)).build();
        let same_as_morning = Builder::new(&shift).start(start(8)).build();

        assert!(morning < afternoon);
        assert_eq!(morning.cmp(&same_as_morning), Ordering::Equal);
        assert_eq!(morning, same_as_morning);

        let periods = shift.get_periods();
        assert_eq!(periods.len(), 2);
        assert_eq!(periods[0].get_start(), start(8));
        assert_eq!(periods[1].get_start(), start(13));
    }
}
