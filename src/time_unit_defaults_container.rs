//! 对应 Java: `org/mpxj/TimeUnitDefaultsContainer.java`

/// 提供时长单位换算所需默认值的接口。
///
/// 对应 Java: `public interface TimeUnitDefaultsContainer`
pub trait TimeUnitDefaultsContainer {
    /// 每天分钟数。
    ///
    /// 对应 Java: `Integer getMinutesPerDay()`
    fn get_minutes_per_day(&self) -> Option<i32>;

    /// 每周分钟数。
    ///
    /// 对应 Java: `Integer getMinutesPerWeek()`
    fn get_minutes_per_week(&self) -> Option<i32>;

    /// 每月分钟数。
    ///
    /// 对应 Java: `Integer getMinutesPerMonth()`
    fn get_minutes_per_month(&self) -> Option<i32>;

    /// 每年分钟数。
    ///
    /// 对应 Java: `Integer getMinutesPerYear()`
    fn get_minutes_per_year(&self) -> Option<i32>;

    /// 每月天数。
    ///
    /// 对应 Java: `Integer getDaysPerMonth()`
    fn get_days_per_month(&self) -> Option<i32>;
}
