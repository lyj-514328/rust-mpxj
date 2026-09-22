//! Corresponds to Java: `org/mpxj/Duration.java`

use crate::time_unit::TimeUnit;
use crate::time_unit_defaults_container::TimeUnitDefaultsContainer;
use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};

/// A duration (amount + time unit) as represented in MPX files.
///
/// Corresponds to Java: `public final class Duration implements Comparable<Duration>`
///
/// The Java side uses a shared `ZERO_DURATIONS` object to represent the zero value and save memory; the Rust `Duration`
/// is a small, `Copy` value type (two fields, immutable), so it does not need this cache and behaves equivalently.
///
/// Also note: Java's two private constructors (`double` / `int` overloads) are merged in Rust into
/// [`Duration::new`].
#[derive(Debug, Clone, Copy)]
pub struct Duration {
/// Duration amount.
    ///
    /// Corresponds to Java: `private final double m_duration;`
    m_duration: f64,
/// Duration unit.
    ///
    /// Corresponds to Java: `private final TimeUnit m_units;`
    m_units: TimeUnit,
}

impl Duration {
/// Private constructor.
    ///
    /// Corresponds to Java: `private Duration(double duration, TimeUnit type)`
/// and `private Duration(int duration, TimeUnit type)` (Rust cannot overload, so this method handles both).
    fn new(duration: f64, units: TimeUnit) -> Duration {
        Duration {
            m_duration: duration,
            m_units: units,
        }
    }

/// Return the duration amount.
    ///
    /// Corresponds to Java: `public double getDuration()`
    pub fn get_duration(&self) -> f64 {
        self.m_duration
    }

/// Return the duration unit.
    ///
    /// Corresponds to Java: `public TimeUnit getUnits()`
    pub fn get_units(&self) -> TimeUnit {
        self.m_units
    }

/// Convert this duration into the target unit (approximate conversion using project defaults, ignoring calendar details).
    ///
    /// Corresponds to Java: `public Duration convertUnits(TimeUnit type, TimeUnitDefaultsContainer defaults)`
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

/// Convert a duration value from `from_units` to `to_units`, with defaults provided by a container.
    ///
    /// Corresponds to Java: `public static Duration convertUnits(double duration, TimeUnit fromUnits, TimeUnit toUnits, TimeUnitDefaultsContainer defaults)`
    ///
/// The Java side throws `NullPointerException` for a `null` default value; the Rust side does not panic and
/// treats a missing value as `0.0`.
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

/// Convert a duration value from `from_units` to `to_units`, with the default given directly as a number.
    ///
    /// Corresponds to Java: `public static Duration convertUnits(double duration, TimeUnit fromUnits, TimeUnit toUnits, double minutesPerDay, double minutesPerWeek, double daysPerMonth)`
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

// Step one: convert the source unit to minutes.
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
            // Java: default -> break(MINUTES / ELAPSED_MINUTES / PERCENT / ELAPSED_PERCENT)
            _ => {}
        }

// Step two: convert minutes to the target unit.
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

/// Create a `Duration` instance (`double` version).
    ///
    /// Corresponds to Java: `public static Duration getInstance(double duration, TimeUnit type)`
    pub fn get_instance(duration: f64, units: TimeUnit) -> Duration {
        Duration::new(duration, units)
    }

/// Create a `Duration` instance (`int` version).
    ///
    /// Corresponds to Java: `public static Duration getInstance(int duration, TimeUnit type)`
    ///
/// Java's `int` overload conflicts with `get_instance` in Rust, so the `_int` suffix is added.
    pub fn get_instance_int(duration: i32, units: TimeUnit) -> Duration {
        Duration::new(f64::from(duration), units)
    }

/// Compare only the duration amounts (ignoring the unit).
    ///
    /// Corresponds to Java: `public boolean durationComponentEquals(Duration rhs)`
    pub fn duration_component_equals(&self, rhs: &Duration) -> bool {
        Duration::duration_value_equals(self.m_duration, rhs.m_duration)
    }

/// Compare two duration values within the allowed tolerance (`0.00001`).
    ///
    /// Corresponds to Java: `public static boolean durationValueEquals(double lhs, double rhs)`
    pub fn duration_value_equals(lhs: f64, rhs: f64) -> bool {
        // Java: NumberHelper.equals(lhs, rhs, 0.00001) -> Math.abs(lhs - rhs) < delta
        (lhs - rhs).abs() < 0.00001
    }

/// Add two durations; when either is `None`, return the other following Java's `null` semantics.
    ///
    /// Corresponds to Java: `public static Duration add(Duration a, Duration b, TimeUnitDefaultsContainer defaults)`
    ///
/// Converts `b` to the unit of `a` when needed.
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

/// Negate.
    ///
    /// Corresponds to Java: `public Duration negate()`
    pub fn negate(&self) -> Duration {
        if self.m_duration == 0.0 {
            *self
        } else {
            Duration::get_instance(-self.m_duration, self.m_units)
        }
    }
}

impl PartialEq for Duration {
/// Returns `true` when both the amount (within tolerance) and the unit are equal.
    ///
    /// Corresponds to Java: `@Override public boolean equals(Object o)`
    fn eq(&self, other: &Self) -> bool {
        self.duration_component_equals(other) && self.m_units == other.m_units
    }
}

/// Note: `Duration` contains an `f64` field and cannot automatically derive `Eq`; but the Java side implements
/// `Comparable`, and Rust's `Ord` requires `Eq` as a supertrait, so `Eq` is implemented manually here
/// (equality is based on the tolerance comparison above, and `Duration` itself is immutable).
impl Eq for Duration {}

impl Ord for Duration {
    /// Corresponds to Java: `@Override public int compareTo(Duration rhs)`
    ///
/// When the units differ, first use fixed defaults (480 minutes per day, 2400 minutes per week, 20 days per month)
/// to convert `rhs` into this instance's unit, then compare.
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
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Note: Java's `double` string conversion keeps `.0` (e.g. `8.0`), while Rust's `f64`
/// `Display` outputs `8`; the two differ slightly in this method.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.m_duration, self.m_units)
    }
}
