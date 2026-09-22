//! Corresponds to Java: `org/mpxj/common/RateHelper.java`
//!
//! Unit conversion utility for rates ([`Rate`]). Inside MPP files rates are always stored as
//! "amount per hour", and on reading they must be converted back to the target unit given by
//! the units field ([`RateHelper::convert_from_hours`]).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `TimeUnitDefaultsContainer defaults` | `&dyn TimeUnitDefaultsContainer` | The same-named trait has landed (`src/time_unit_defaults_container.rs`); `ProjectProperties` / `ProjectCalendar` / `TimeUnitDefaults` all implement it |
//! | `defaults.getMinutesPerDay().doubleValue()` throws an NPE when the value is `null` | `unwrap_or(0)` | Per this project's "library code does not panic" convention |
//!
//! Unported Java methods:
//!
//! | Java | Reason |
//! |---|---|
//! | `convertFromHours(TimeUnitDefaultsContainer, BigDecimal, TimeUnit)` | Only serves the MSPDI write path (`BigDecimal` precision and truncation in the `YEARS` branch); this crate is reader-only, so there is no call site |
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: not used by this class.

use crate::common::number_helper::NumberHelper;
use crate::rate::Rate;
use crate::time_unit::TimeUnit;
use crate::time_unit_defaults_container::TimeUnitDefaultsContainer;

/// Rate conversion utility.
///
/// Corresponds to Java: `public final class RateHelper` (a pure static utility class whose
/// private constructor prevents instantiation; the Rust side expresses it as a fieldless unit
/// struct + associated functions).
pub struct RateHelper;

impl RateHelper {
    /// Convert an "amount per hour" into a rate in the target unit.
    ///
    /// Corresponds to Java: `public static Rate convertFromHours(TimeUnitDefaultsContainer defaults, double value, TimeUnit targetUnits)`
    ///
    /// Only handles the five branches `MINUTES` / `DAYS` / `WEEKS` / `MONTHS` / `YEARS`; other
    /// units (`HOURS` and the `ELAPSED_*` variants) keep their original value. The result is
    /// rounded to two decimals via `NumberHelper.round(value, 2)`.
    pub fn convert_from_hours(
        defaults: &dyn TimeUnitDefaultsContainer,
        value: f64,
        target_units: TimeUnit,
    ) -> Rate {
        // Java: switch (targetUnits) { ... }
        let (minutes_per_day, minutes_per_week, minutes_per_month) = (
            f64::from(defaults.get_minutes_per_day().unwrap_or(0)),
            f64::from(defaults.get_minutes_per_week().unwrap_or(0)),
            f64::from(defaults.get_minutes_per_month().unwrap_or(0)),
        );

        let value = match target_units {
            TimeUnit::Minutes => value / 60.0,
            TimeUnit::Days => (value * minutes_per_day) / 60.0,
            TimeUnit::Weeks => (value * minutes_per_week) / 60.0,
            TimeUnit::Months => (value * minutes_per_month) / 60.0,
            TimeUnit::Years => (value * minutes_per_week * 52.0) / 60.0,
            _ => value,
        };

        // Java: return new Rate(NumberHelper.round(value, 2), targetUnits);
        Rate::new(NumberHelper::round(value, 2.0), target_units)
    }

    /// Convert a rate into an "amount per hour".
    ///
    /// Corresponds to Java: `public static double convertToHours(TimeUnitDefaultsContainer defaults, Rate rate)`
    ///
    /// Only handles the five branches `MINUTES` / `DAYS` / `WEEKS` / `MONTHS` / `YEARS`; other
    /// units (`HOURS` and the `ELAPSED_*` variants) keep their original value. Unlike
    /// [`RateHelper::convert_from_hours`], Java does **not** round here, so `NumberHelper::round`
    /// is not applied.
    pub fn convert_to_hours(defaults: &dyn TimeUnitDefaultsContainer, rate: &Rate) -> f64 {
        let amount = rate.get_amount();

        // Java: switch (rate.getUnits()) { … }
        match rate.get_units() {
            TimeUnit::Minutes => amount * 60.0,
            TimeUnit::Days => {
                (amount * 60.0) / f64::from(defaults.get_minutes_per_day().unwrap_or(0))
            }
            TimeUnit::Weeks => {
                (amount * 60.0) / f64::from(defaults.get_minutes_per_week().unwrap_or(0))
            }
            TimeUnit::Months => {
                (amount * 60.0) / f64::from(defaults.get_minutes_per_month().unwrap_or(0))
            }
            // Java: (amount * 60.0) / (defaults.getMinutesPerWeek().intValue() * 52)
            // Note that the 52 here is **integer** multiplication (same value as `* 52` in `convert_from_hours`).
            TimeUnit::Years => {
                (amount * 60.0) / f64::from(defaults.get_minutes_per_week().unwrap_or(0) * 52)
            }
            _ => amount,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_unit_defaults::TimeUnitDefaults;

    /// The default conversion bases of `TimeUnitDefaults`: 480 minutes/day, 2400 minutes/week, 9600 minutes/month.
    fn defaults() -> TimeUnitDefaults {
        TimeUnitDefaults::new()
    }

    /// Consistent with the same set of conversions in Java's `RateHelperTest.testConvertFromHours` (amounts scaled up 100x).
    #[test]
    fn converts_from_hours_to_the_target_unit() {
        let defaults = defaults();

        assert_eq!(
            RateHelper::convert_from_hours(&defaults, 60.0, TimeUnit::Minutes),
            Rate::new(1.0, TimeUnit::Minutes)
        );
        assert_eq!(
            RateHelper::convert_from_hours(&defaults, 100.0, TimeUnit::Days),
            Rate::new(800.0, TimeUnit::Days)
        );
        assert_eq!(
            RateHelper::convert_from_hours(&defaults, 100.0, TimeUnit::Weeks),
            Rate::new(4000.0, TimeUnit::Weeks)
        );
        assert_eq!(
            RateHelper::convert_from_hours(&defaults, 100.0, TimeUnit::Months),
            Rate::new(16000.0, TimeUnit::Months)
        );
        // Each year is calculated as 52 weeks
        assert_eq!(
            RateHelper::convert_from_hours(&defaults, 100.0, TimeUnit::Years),
            Rate::new(208000.0, TimeUnit::Years)
        );
    }

    /// `convert_to_hours` is the inverse conversion of `convert_from_hours` (the two do not affect
    /// each other on the Java side; here the same bases are used to verify round-trip consistency).
    #[test]
    fn converts_to_hours_is_the_inverse_of_from_hours() {
        let defaults = defaults();

        assert_eq!(
            RateHelper::convert_to_hours(&defaults, &Rate::new(1.0, TimeUnit::Minutes)),
            60.0
        );
        assert_eq!(
            RateHelper::convert_to_hours(&defaults, &Rate::new(800.0, TimeUnit::Days)),
            100.0
        );
        assert_eq!(
            RateHelper::convert_to_hours(&defaults, &Rate::new(4000.0, TimeUnit::Weeks)),
            100.0
        );
        assert_eq!(
            RateHelper::convert_to_hours(&defaults, &Rate::new(16000.0, TimeUnit::Months)),
            100.0
        );
        // Each year is calculated as 52 weeks
        assert_eq!(
            RateHelper::convert_to_hours(&defaults, &Rate::new(208000.0, TimeUnit::Years)),
            100.0
        );
    }

    /// An hourly rate is returned unchanged (Java's `default: break`).
    #[test]
    fn convert_to_hours_leaves_other_units_unchanged() {
        let defaults = defaults();

        assert_eq!(
            RateHelper::convert_to_hours(&defaults, &Rate::new(12.34, TimeUnit::Hours)),
            12.34
        );
    }

    /// Units not listed, such as `HOURS` and `ELAPSED_*`, keep their original value (only rounded to two decimals).
    #[test]
    fn leaves_other_units_unchanged() {
        let defaults = defaults();

        assert_eq!(
            RateHelper::convert_from_hours(&defaults, 12.345, TimeUnit::Hours),
            Rate::new(12.35, TimeUnit::Hours)
        );
        assert_eq!(
            RateHelper::convert_from_hours(&defaults, 50.0, TimeUnit::ElapsedDays),
            Rate::new(50.0, TimeUnit::ElapsedDays)
        );
    }
}
