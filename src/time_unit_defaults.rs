//! Corresponds to Java: `org/mpxj/TimeUnitDefaults.java`

use crate::time_unit_defaults_container::TimeUnitDefaultsContainer;

/// Default minutes per day.
///
/// Corresponds to Java: `private static final Integer DEFAULT_MINUTES_PER_DAY = Integer.valueOf(480);`
const DEFAULT_MINUTES_PER_DAY: i32 = 480;

/// Default days per week.
///
/// Corresponds to Java: `private static final int DEFAULT_DAYS_PER_WEEK = 5;`
const DEFAULT_DAYS_PER_WEEK: i32 = 5;

/// Default days per month.
///
/// Corresponds to Java: `private static final Integer DEFAULT_DAYS_PER_MONTH = Integer.valueOf(20);`
const DEFAULT_DAYS_PER_MONTH: i32 = 20;

/// Duration-unit related defaults in the project context.
///
/// Corresponds to Java: `public class TimeUnitDefaults implements TimeUnitDefaultsContainer`
///
/// Porting note: Java caches the derived result back into the field in the getter (`m_minutesPerDay` etc.);
/// the Rust methods of [`TimeUnitDefaultsContainer`] take `&self`, so the Rust side instead derives from the fields each time
/// (falling back to the default value when `None`) without writing back to the fields; the return values are fully consistent with Java.
///
/// Unported Java methods: none.
#[derive(Debug, Clone)]
pub struct TimeUnitDefaults {
/// Minutes per day.
    ///
    /// Corresponds to Java: `private Integer m_minutesPerDay;`
    m_minutes_per_day: Option<i32>,
/// Minutes per week.
    ///
    /// Corresponds to Java: `private Integer m_minutesPerWeek;`
    m_minutes_per_week: Option<i32>,
/// Minutes per month.
    ///
    /// Corresponds to Java: `private Integer m_minutesPerMonth;`
    m_minutes_per_month: Option<i32>,
/// Minutes per year.
    ///
    /// Corresponds to Java: `private Integer m_minutesPerYear;`
    m_minutes_per_year: Option<i32>,
/// Days per month.
    ///
    /// Corresponds to Java: `private Integer m_daysPerMonth = DEFAULT_DAYS_PER_MONTH;`
    m_days_per_month: Option<i32>,
}

impl TimeUnitDefaults {
/// Constructor.
    ///
/// Corresponds to Java: default constructor (`m_daysPerMonth` initialized to `DEFAULT_DAYS_PER_MONTH`).
    pub fn new() -> Self {
        Self {
            m_minutes_per_day: None,
            m_minutes_per_week: None,
            m_minutes_per_month: None,
            m_minutes_per_year: None,
            m_days_per_month: Some(DEFAULT_DAYS_PER_MONTH),
        }
    }

/// Set the minutes per day.
    ///
    /// Corresponds to Java: `public void setMinutesPerDay(Integer minutesPerDay)`
    pub fn set_minutes_per_day(&mut self, minutes_per_day: Option<i32>) {
        self.m_minutes_per_day = minutes_per_day;
    }

/// Set the minutes per week.
    ///
    /// Corresponds to Java: `public void setMinutesPerWeek(Integer minutesPerWeek)`
    pub fn set_minutes_per_week(&mut self, minutes_per_week: Option<i32>) {
        self.m_minutes_per_week = minutes_per_week;
    }

/// Set the minutes per month.
    ///
    /// Corresponds to Java: `public void setMinutesPerMonth(Integer minutesPerMonth)`
    pub fn set_minutes_per_month(&mut self, minutes_per_month: Option<i32>) {
        self.m_minutes_per_month = minutes_per_month;
    }

/// Set the minutes per year.
    ///
    /// Corresponds to Java: `public void setMinutesPerYear(Integer minutesPerYear)`
    pub fn set_minutes_per_year(&mut self, minutes_per_year: Option<i32>) {
        self.m_minutes_per_year = minutes_per_year;
    }

/// Set the days per month.
    ///
    /// Corresponds to Java: `public void setDaysPerMonth(Integer daysPerMonth)`
    pub fn set_days_per_month(&mut self, days_per_month: Option<i32>) {
        self.m_days_per_month = days_per_month;
    }
}

impl Default for TimeUnitDefaults {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeUnitDefaultsContainer for TimeUnitDefaults {
/// Get the minutes per day, returning the default value 480 when unset.
    ///
    /// Corresponds to Java: `@Override public Integer getMinutesPerDay()`
    fn get_minutes_per_day(&self) -> Option<i32> {
        Some(self.m_minutes_per_day.unwrap_or(DEFAULT_MINUTES_PER_DAY))
    }

/// Get the minutes per week, deriving it as "5 days per week × minutes per day" when unset.
    ///
    /// Corresponds to Java: `@Override public Integer getMinutesPerWeek()`
    fn get_minutes_per_week(&self) -> Option<i32> {
        self.m_minutes_per_week
            .or_else(|| Some(DEFAULT_DAYS_PER_WEEK * self.get_minutes_per_day().unwrap_or(0)))
    }

/// Get the minutes per month, deriving it as "minutes per day × days per month" when unset.
    ///
    /// Corresponds to Java: `@Override public Integer getMinutesPerMonth()`
    fn get_minutes_per_month(&self) -> Option<i32> {
        self.m_minutes_per_month.or_else(|| {
            Some(self.get_minutes_per_day().unwrap_or(0) * self.get_days_per_month().unwrap_or(0))
        })
    }

/// Get the minutes per year, deriving it as "minutes per day × days per month × 12" when unset.
    ///
    /// Corresponds to Java: `@Override public Integer getMinutesPerYear()`
    fn get_minutes_per_year(&self) -> Option<i32> {
        self.m_minutes_per_year.or_else(|| {
            Some(
                self.get_minutes_per_day().unwrap_or(0) * self.get_days_per_month().unwrap_or(0) * 12,
            )
        })
    }

/// Get the days per month, returning `None` when unset (Java returns `null`).
    ///
    /// Corresponds to Java: `@Override public Integer getDaysPerMonth()`
    fn get_days_per_month(&self) -> Option<i32> {
        self.m_days_per_month
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_java_constants() {
        let defaults = TimeUnitDefaults::new();

        assert_eq!(defaults.get_minutes_per_day(), Some(480));
        assert_eq!(defaults.get_minutes_per_week(), Some(2400));
        assert_eq!(defaults.get_minutes_per_month(), Some(9600));
        assert_eq!(defaults.get_minutes_per_year(), Some(115_200));
        assert_eq!(defaults.get_days_per_month(), Some(20));
    }

    #[test]
    fn explicit_values_take_precedence() {
        let mut defaults = TimeUnitDefaults::new();
        defaults.set_minutes_per_day(Some(600));
        defaults.set_minutes_per_week(Some(3000));
        defaults.set_minutes_per_month(Some(12_000));
        defaults.set_minutes_per_year(Some(144_000));
        defaults.set_days_per_month(Some(30));

        assert_eq!(defaults.get_minutes_per_day(), Some(600));
        assert_eq!(defaults.get_minutes_per_week(), Some(3000));
        assert_eq!(defaults.get_minutes_per_month(), Some(12_000));
        assert_eq!(defaults.get_minutes_per_year(), Some(144_000));
        assert_eq!(defaults.get_days_per_month(), Some(30));
    }

    #[test]
    fn derived_values_follow_minutes_per_day_and_days_per_month() {
        let mut defaults = TimeUnitDefaults::new();
        defaults.set_minutes_per_day(Some(600));

        assert_eq!(defaults.get_minutes_per_week(), Some(3000));
        assert_eq!(defaults.get_minutes_per_month(), Some(12_000));
        assert_eq!(defaults.get_minutes_per_year(), Some(144_000));

// Java: when setDaysPerMonth(null), getMinutesPerMonth uses 0 in the calculation
        let mut defaults = TimeUnitDefaults::new();
        defaults.set_days_per_month(None);
        assert_eq!(defaults.get_days_per_month(), None);
        assert_eq!(defaults.get_minutes_per_month(), Some(0));
        assert_eq!(defaults.get_minutes_per_year(), Some(0));
    }

    #[test]
    fn default_trait_matches_new() {
        let defaults = TimeUnitDefaults::default();

        assert_eq!(defaults.get_minutes_per_day(), Some(480));
        assert_eq!(defaults.get_days_per_month(), Some(20));
    }

/// Use it as a trait object to verify that the `TimeUnitDefaultsContainer` implementation is usable.
    #[test]
    fn usable_as_trait_object() {
        let defaults = TimeUnitDefaults::new();
        let container: &dyn TimeUnitDefaultsContainer = &defaults;

        assert_eq!(container.get_minutes_per_day(), Some(480));
        assert_eq!(container.get_minutes_per_year(), Some(115_200));
    }
}
