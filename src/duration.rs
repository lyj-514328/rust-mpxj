//! 对应 Java: `org/mpxj/Duration.java`

use crate::time_unit::TimeUnit;
use crate::time_unit_defaults_container::TimeUnitDefaultsContainer;
use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};

/// MPX 文件中表示的一段工期（数量 + 时间单位）。
///
/// 对应 Java: `public final class Duration implements Comparable<Duration>`
///
/// Java 侧用 `ZERO_DURATIONS` 共享对象表示 0 值以节省内存；Rust 侧 `Duration`
/// 是小而可 `Copy` 的值类型（两个字段、不可变），不需要这层缓存，行为等价。
///
/// 另注：Java 的两个私有构造方法（`double` / `int` 重载）在 Rust 中合并为
/// [`Duration::new`]。
#[derive(Debug, Clone, Copy)]
pub struct Duration {
    /// 工期数量。
    ///
    /// 对应 Java: `private final double m_duration;`
    m_duration: f64,
    /// 工期单位。
    ///
    /// 对应 Java: `private final TimeUnit m_units;`
    m_units: TimeUnit,
}

impl Duration {
    /// 私有构造方法。
    ///
    /// 对应 Java: `private Duration(double duration, TimeUnit type)`
    /// 及 `private Duration(int duration, TimeUnit type)`（Rust 无法重载，由本方法统一承担）。
    fn new(duration: f64, units: TimeUnit) -> Duration {
        Duration {
            m_duration: duration,
            m_units: units,
        }
    }

    /// 返回工期数量。
    ///
    /// 对应 Java: `public double getDuration()`
    pub fn get_duration(&self) -> f64 {
        self.m_duration
    }

    /// 返回工期单位。
    ///
    /// 对应 Java: `public TimeUnit getUnits()`
    pub fn get_units(&self) -> TimeUnit {
        self.m_units
    }

    /// 把本工期换算成目标单位（近似换算，参考项目默认值，不考虑日历细节）。
    ///
    /// 对应 Java: `public Duration convertUnits(TimeUnit type, TimeUnitDefaultsContainer defaults)`
    pub fn convert_units(
        &self,
        units: TimeUnit,
        defaults: &dyn TimeUnitDefaultsContainer,
    ) -> Duration {
        if units == self.m_units {
            return *self;
        }

        Duration::convert_units_with_defaults(self.m_duration, self.m_units, units, defaults)
    }

    /// 把工期值从 `from_units` 换算到 `to_units`，默认值由一个容器提供。
    ///
    /// 对应 Java: `public static Duration convertUnits(double duration, TimeUnit fromUnits, TimeUnit toUnits, TimeUnitDefaultsContainer defaults)`
    ///
    /// Java 侧对 `null` 的默认值会抛 `NullPointerException`；Rust 侧不 panic，
    /// 缺省值按 `0.0` 处理。
    pub fn convert_units_with_defaults(
        duration: f64,
        from_units: TimeUnit,
        to_units: TimeUnit,
        defaults: &dyn TimeUnitDefaultsContainer,
    ) -> Duration {
        Duration::convert_units_with_values(
            duration,
            from_units,
            to_units,
            defaults.get_minutes_per_day().map(f64::from).unwrap_or(0.0),
            defaults
                .get_minutes_per_week()
                .map(f64::from)
                .unwrap_or(0.0),
            defaults.get_days_per_month().map(f64::from).unwrap_or(0.0),
        )
    }

    /// 把工期值从 `from_units` 换算到 `to_units`，默认值直接以数值给出。
    ///
    /// 对应 Java: `public static Duration convertUnits(double duration, TimeUnit fromUnits, TimeUnit toUnits, double minutesPerDay, double minutesPerWeek, double daysPerMonth)`
    pub fn convert_units_with_values(
        duration: f64,
        from_units: TimeUnit,
        to_units: TimeUnit,
        minutes_per_day: f64,
        minutes_per_week: f64,
        days_per_month: f64,
    ) -> Duration {
        let mut duration = duration;

        if from_units == to_units {
            return Duration::get_instance(duration, from_units);
        }

        // 第一步：把源单位换算为分钟。
        match from_units {
            TimeUnit::Years => duration *= minutes_per_week * 52.0,
            TimeUnit::ElapsedYears => duration *= 60.0 * 24.0 * 7.0 * 52.0,
            TimeUnit::Months => duration *= minutes_per_day * days_per_month,
            TimeUnit::ElapsedMonths => duration *= 60.0 * 24.0 * 30.0,
            TimeUnit::Weeks => duration *= minutes_per_week,
            TimeUnit::ElapsedWeeks => duration *= 60.0 * 24.0 * 7.0,
            TimeUnit::Days => duration *= minutes_per_day,
            TimeUnit::ElapsedDays => duration *= 60.0 * 24.0,
            TimeUnit::Hours | TimeUnit::ElapsedHours => duration *= 60.0,
            // Java: default -> break（MINUTES / ELAPSED_MINUTES / PERCENT / ELAPSED_PERCENT）
            _ => {}
        }

        // 第二步：把分钟换算为目标单位。
        if to_units != TimeUnit::Minutes && to_units != TimeUnit::ElapsedMinutes {
            match to_units {
                TimeUnit::Hours | TimeUnit::ElapsedHours => duration /= 60.0,
                TimeUnit::Days => {
                    if minutes_per_day != 0.0 {
                        duration /= minutes_per_day;
                    } else {
                        duration = 0.0;
                    }
                }
                TimeUnit::ElapsedDays => duration /= 60.0 * 24.0,
                TimeUnit::Weeks => {
                    if minutes_per_week != 0.0 {
                        duration /= minutes_per_week;
                    } else {
                        duration = 0.0;
                    }
                }
                TimeUnit::ElapsedWeeks => duration /= 60.0 * 24.0 * 7.0,
                TimeUnit::Months => {
                    if minutes_per_day != 0.0 && days_per_month != 0.0 {
                        duration /= minutes_per_day * days_per_month;
                    } else {
                        duration = 0.0;
                    }
                }
                TimeUnit::ElapsedMonths => duration /= 60.0 * 24.0 * 30.0,
                TimeUnit::Years => {
                    if minutes_per_week != 0.0 {
                        duration /= minutes_per_week * 52.0;
                    } else {
                        duration = 0.0;
                    }
                }
                TimeUnit::ElapsedYears => duration /= 60.0 * 24.0 * 7.0 * 52.0,
                // Java: default -> break
                _ => {}
            }
        }

        Duration::get_instance(duration, to_units)
    }

    /// 创建 `Duration` 实例（`double` 版本）。
    ///
    /// 对应 Java: `public static Duration getInstance(double duration, TimeUnit type)`
    pub fn get_instance(duration: f64, units: TimeUnit) -> Duration {
        Duration::new(duration, units)
    }

    /// 创建 `Duration` 实例（`int` 版本）。
    ///
    /// 对应 Java: `public static Duration getInstance(int duration, TimeUnit type)`
    ///
    /// Java 的 `int` 重载在 Rust 中与 `get_instance` 冲突，故加 `_int` 后缀。
    pub fn get_instance_int(duration: i32, units: TimeUnit) -> Duration {
        Duration::new(f64::from(duration), units)
    }

    /// 只比较工期数量（不考虑单位）。
    ///
    /// 对应 Java: `public boolean durationComponentEquals(Duration rhs)`
    pub fn duration_component_equals(&self, rhs: &Duration) -> bool {
        Duration::duration_value_equals(self.m_duration, rhs.m_duration)
    }

    /// 在允许误差（`0.00001`）内比较两个工期值。
    ///
    /// 对应 Java: `public static boolean durationValueEquals(double lhs, double rhs)`
    pub fn duration_value_equals(lhs: f64, rhs: f64) -> bool {
        // Java: NumberHelper.equals(lhs, rhs, 0.00001) -> Math.abs(lhs - rhs) < delta
        (lhs - rhs).abs() < 0.00001
    }

    /// 两个工期相加，任一为 `None` 时按 Java 的 `null` 语义返回另一方。
    ///
    /// 对应 Java: `public static Duration add(Duration a, Duration b, TimeUnitDefaultsContainer defaults)`
    ///
    /// 需要时会把 `b` 换算成 `a` 的单位。
    pub fn add(
        a: Option<Duration>,
        b: Option<Duration>,
        defaults: &dyn TimeUnitDefaultsContainer,
    ) -> Option<Duration> {
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(b),
            (Some(a), None) => Some(a),
            (Some(a), Some(mut b)) => {
                let units = a.get_units();
                if b.get_units() != units {
                    b = b.convert_units(units, defaults);
                }
                Some(Duration::get_instance(
                    a.get_duration() + b.get_duration(),
                    units,
                ))
            }
        }
    }

    /// 取反。
    ///
    /// 对应 Java: `public Duration negate()`
    pub fn negate(&self) -> Duration {
        if self.m_duration == 0.0 {
            *self
        } else {
            Duration::get_instance(-self.m_duration, self.m_units)
        }
    }
}

impl PartialEq for Duration {
    /// 数量（在误差范围内）与单位都相等时返回 `true`。
    ///
    /// 对应 Java: `@Override public boolean equals(Object o)`
    fn eq(&self, other: &Self) -> bool {
        self.duration_component_equals(other) && self.m_units == other.m_units
    }
}

/// 说明：`Duration` 含 `f64` 字段，无法自动派生 `Eq`；但 Java 侧实现了
/// `Comparable`，Rust 的 `Ord` 要求 `Eq` 作为父 trait，故这里手工实现 `Eq`
/// （相等性基于上面的误差比较，`Duration` 本身不可变）。
impl Eq for Duration {}

impl Ord for Duration {
    /// 对应 Java: `@Override public int compareTo(Duration rhs)`
    ///
    /// 单位不同时，先用固定的默认值（每天 480 分钟、每周 2400 分钟、每月 20 天）
    /// 把 `rhs` 换算成本实例的单位再比较。
    fn cmp(&self, other: &Self) -> Ordering {
        let mut rhs = *other;
        if self.m_units != rhs.m_units {
            rhs = Duration::convert_units_with_values(
                rhs.m_duration,
                rhs.m_units,
                self.m_units,
                8.0 * 60.0,
                5.0 * 8.0 * 60.0,
                20.0,
            );
        }

        if self.duration_component_equals(&rhs) {
            Ordering::Equal
        } else if self.m_duration < rhs.m_duration {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

impl PartialOrd for Duration {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for Duration {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// 注意：Java 的 `double` 字符串化会保留 `.0`（如 `8.0`），Rust 的 `f64`
    /// `Display` 会输出 `8`，二者在本方法上存在细微差异。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.m_duration, self.m_units)
    }
}
