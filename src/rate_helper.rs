//! 对应 Java: `org/mpxj/common/RateHelper.java`
//!
//! 费率（[`Rate`]）的单位换算工具。MPP 文件内部把费率一律存成「每小时金额」，
//! 读取时需要按单位字段给出的目标单位换算回来（[`RateHelper::convert_from_hours`]）。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `TimeUnitDefaultsContainer defaults` | `&dyn TimeUnitDefaultsContainer` | 同名 trait 已落地（`src/time_unit_defaults_container.rs`），`ProjectProperties` / `ProjectCalendar` / `TimeUnitDefaults` 均已实现 |
//! | `defaults.getMinutesPerDay().doubleValue()` 在取值为 `null` 时抛 NPE | `unwrap_or(0)` | 按本工程「库代码不 panic」约定 |
//!
//! 未移植的 Java 方法：
//!
//! | Java | 原因 |
//! |---|---|
//! | `convertFromHours(TimeUnitDefaultsContainer, BigDecimal, TimeUnit)` | 只服务于 MSPDI 的写入路径（`BigDecimal` 精度与 `YEARS` 分支的截断），本 crate 只做 reader，无调用点 |
//! | `convertToHours(TimeUnitDefaultsContainer, Rate)` | 其唯一调用点（`ResourceAssignment::getRatePerHour`）在 `resource_assignment.rs` 内已有等价的私有实现 `rate_to_hours`，接线不在本批文件范围内 |
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。

use crate::common::number_helper::NumberHelper;
use crate::rate::Rate;
use crate::time_unit::TimeUnit;
use crate::time_unit_defaults_container::TimeUnitDefaultsContainer;

/// 费率换算工具。
///
/// 对应 Java: `public final class RateHelper`（私有构造方法禁止实例化的纯静态工具类，
/// Rust 侧用无字段的单元结构体 + 关联函数表达）。
pub struct RateHelper;

impl RateHelper {
    /// 把「每小时金额」换算成目标单位的费率。
    ///
    /// 对应 Java: `public static Rate convertFromHours(TimeUnitDefaultsContainer defaults, double value, TimeUnit targetUnits)`
    ///
    /// 只处理 `MINUTES` / `DAYS` / `WEEKS` / `MONTHS` / `YEARS` 五个分支，其余单位
    /// （`HOURS` 与各 `ELAPSED_*`）保持原值；结果按 `NumberHelper.round(value, 2)` 取两位小数。
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_unit_defaults::TimeUnitDefaults;

    /// `TimeUnitDefaults` 的默认换算基数：480 分钟/天、2400 分钟/周、9600 分钟/月。
    fn defaults() -> TimeUnitDefaults {
        TimeUnitDefaults::new()
    }

    /// 与 Java `RateHelperTest.testConvertFromHours` 的同一组换算（金额放大 100 倍）一致。
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
        // 每年按 52 周计算
        assert_eq!(
            RateHelper::convert_from_hours(&defaults, 100.0, TimeUnit::Years),
            Rate::new(208000.0, TimeUnit::Years)
        );
    }

    /// `HOURS` 与 `ELAPSED_*` 等未列出的单位保持原值（只做两位小数取整）。
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
