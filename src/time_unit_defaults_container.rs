//! Corresponds to Java: `org/mpxj/TimeUnitDefaultsContainer.java`

/// Interface providing the default values needed for duration unit conversion.
///
/// Corresponds to Java: `public interface TimeUnitDefaultsContainer`
pub trait TimeUnitDefaultsContainer {
    /// Minutes per day.
    ///
    /// Corresponds to Java: `Integer getMinutesPerDay()`
    fn get_minutes_per_day(&self) -> Option<i32>;

    /// Minutes per week.
    ///
    /// Corresponds to Java: `Integer getMinutesPerWeek()`
    fn get_minutes_per_week(&self) -> Option<i32>;

    /// Minutes per month.
    ///
    /// Corresponds to Java: `Integer getMinutesPerMonth()`
    fn get_minutes_per_month(&self) -> Option<i32>;

    /// Minutes per year.
    ///
    /// Corresponds to Java: `Integer getMinutesPerYear()`
    fn get_minutes_per_year(&self) -> Option<i32>;

    /// Days per month.
    ///
    /// Corresponds to Java: `Integer getDaysPerMonth()`
    fn get_days_per_month(&self) -> Option<i32>;
}
