//! 对应 Java: `org/mpxj/mpp/TimescaleTier.java`
//!
//! 甘特图时间刻度（timescale）中单个层（tier，顶层 / 中层 / 底层）的属性集合。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `public final class TimescaleTier` | 结构体 [`TimescaleTier`] | 1:1 移植；字段声明顺序与 Java 一致 |
//! | Java 的隐式无参构造方法（字段默认 `false` / `0` / `null`） | [`TimescaleTier::new`] | 与 Java 的 `new TimescaleTier()` 一致：两个标志位为 `false`、`count` 为 0，三个枚举字段为 `None`（Java 为 `null`） |
//! | `TimescaleUnits m_units` / `TimescaleFormat m_format` / `TimescaleAlignment m_alignment`（可为 `null`） | 字段与 getter 均为 `Option<...>` | 「`null` → `Option<T>`」约定：Java 的 `TimescaleTier` 允许「只设部分属性」，`getUnits()` 等在赋值前返回 `null` |
//! | `setUnits(TimescaleUnits)` | [`TimescaleTier::set_units`]（参数非 `Option`） | Java 的全部调用点都传入 `TimescaleUnitsHelper.getInstance(...)`，该方法不会返回 `null`；参数保持与 Java 相同的形态，便于 1:1 对应 |
//! | `setAlignment(TimescaleAlignment)` | [`TimescaleTier::set_alignment`]（参数非 `Option`） | 同上：所有调用点传入 `TimescaleAlignment.getInstance(...)`，不会返回 `null` |
//! | `setFormat(TimescaleFormat)` | [`TimescaleTier::set_format`]（参数为 `Option<TimescaleFormat>`） | `TimescaleFormat.getInstance(...)` 对查找表中未占用的下标（75 / 84 / 122）会返回 `null`（见 `crate::mpp::timescale_format::TimescaleFormat`），故参数用 `Option` |
//! | `toString()` | `Display` | 输出格式与 Java 一致（`Units` / `Alignment` 为 `null` 时输出 `null`，`Format` 为 `null` 时输出 `[null]`） |
//! | 未覆写 `equals` / `hashCode` | 派生 `PartialEq` / `Eq` / `Hash` | Rust 侧为便于比较与测试而派生（逐字段比较） |
//!
//! 未移植 / 跳过：无（JDBC / XML-JAXB / GUI / `javax.*` / `Serializable` 等机制本类未使用）。

use std::fmt::{self, Display, Formatter};

use crate::mpp::mpp_utility::format_optional_display;
use crate::mpp::timescale_alignment::TimescaleAlignment;
use crate::mpp::timescale_format::TimescaleFormat;
use crate::timescale_units::TimescaleUnits;

/// 时间刻度中的一层。
///
/// 对应 Java: `public final class TimescaleTier`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TimescaleTier {
    /// 是否使用财年。
    ///
    /// 对应 Java: `private boolean m_usesFiscalYear`
    uses_fiscal_year: bool,
    /// 是否显示刻度线。
    ///
    /// 对应 Java: `private boolean m_tickLines`
    tick_lines: bool,
    /// 时间刻度单位。
    ///
    /// 对应 Java: `private TimescaleUnits m_units`
    units: Option<TimescaleUnits>,
    /// 层计数（每格包含的单位数）。
    ///
    /// 对应 Java: `private int m_count`
    count: i32,
    /// 标签格式。
    ///
    /// 对应 Java: `private TimescaleFormat m_format`
    format: Option<TimescaleFormat>,
    /// 标签对齐方式。
    ///
    /// 对应 Java: `private TimescaleAlignment m_alignment`
    alignment: Option<TimescaleAlignment>,
}

impl TimescaleTier {
    /// 创建一个所有属性都处于 Java 默认值的实例。
    ///
    /// 对应 Java: `new TimescaleTier()`（隐式无参构造方法）
    pub fn new() -> TimescaleTier {
        TimescaleTier {
            uses_fiscal_year: false,
            tick_lines: false,
            units: None,
            count: 0,
            format: None,
            alignment: None,
        }
    }

    /// 取层计数。
    ///
    /// 对应 Java: `public int getCount()`
    pub fn get_count(&self) -> i32 {
        self.count
    }

    /// 设置层计数。
    ///
    /// 对应 Java: `public void setCount(int count)`
    pub fn set_count(&mut self, count: i32) {
        self.count = count;
    }

    /// 取标签格式。
    ///
    /// 对应 Java: `public TimescaleFormat getFormat()`
    pub fn get_format(&self) -> Option<TimescaleFormat> {
        self.format
    }

    /// 设置标签格式。
    ///
    /// 对应 Java: `public void setFormat(TimescaleFormat format)`
    pub fn set_format(&mut self, format: Option<TimescaleFormat>) {
        self.format = format;
    }

    /// 取是否显示刻度线。
    ///
    /// 对应 Java: `public boolean getTickLines()`
    pub fn get_tick_lines(&self) -> bool {
        self.tick_lines
    }

    /// 设置是否显示刻度线。
    ///
    /// 对应 Java: `public void setTickLines(boolean tickLines)`
    pub fn set_tick_lines(&mut self, tick_lines: bool) {
        self.tick_lines = tick_lines;
    }

    /// 取时间刻度单位。
    ///
    /// 对应 Java: `public TimescaleUnits getUnits()`
    pub fn get_units(&self) -> Option<TimescaleUnits> {
        self.units
    }

    /// 设置时间刻度单位。
    ///
    /// 对应 Java: `public void setUnits(TimescaleUnits units)`
    pub fn set_units(&mut self, units: TimescaleUnits) {
        self.units = Some(units);
    }

    /// 取是否使用财年。
    ///
    /// 对应 Java: `public boolean getUsesFiscalYear()`
    pub fn get_uses_fiscal_year(&self) -> bool {
        self.uses_fiscal_year
    }

    /// 设置是否使用财年。
    ///
    /// 对应 Java: `public void setUsesFiscalYear(boolean usesFiscalYear)`
    pub fn set_uses_fiscal_year(&mut self, uses_fiscal_year: bool) {
        self.uses_fiscal_year = uses_fiscal_year;
    }

    /// 取标签对齐方式。
    ///
    /// 对应 Java: `public TimescaleAlignment getAlignment()`
    pub fn get_alignment(&self) -> Option<TimescaleAlignment> {
        self.alignment
    }

    /// 设置标签对齐方式。
    ///
    /// 对应 Java: `public void setAlignment(TimescaleAlignment alignment)`
    pub fn set_alignment(&mut self, alignment: TimescaleAlignment) {
        self.alignment = Some(alignment);
    }
}

impl Default for TimescaleTier {
    /// 等价于 Java 的隐式无参构造方法。
    fn default() -> TimescaleTier {
        TimescaleTier::new()
    }
}

impl Display for TimescaleTier {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// 逐字段拼接，顺序与 Java 完全一致。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[TimescaleTier UsesFiscalYear={} TickLines={} Units={} Count={} Format=[{}] Alignment={}]",
            self.uses_fiscal_year,
            self.tick_lines,
            format_optional_display(self.units.as_ref()),
            self.count,
            format_optional_display(self.format.as_ref()),
            format_optional_display(self.alignment.as_ref())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Java 的 `new TimescaleTier()` 默认状态。
    #[test]
    fn new_matches_java_defaults() {
        let tier = TimescaleTier::new();
        assert!(!tier.get_uses_fiscal_year());
        assert!(!tier.get_tick_lines());
        assert_eq!(tier.get_units(), None);
        assert_eq!(tier.get_count(), 0);
        assert_eq!(tier.get_format(), None);
        assert_eq!(tier.get_alignment(), None);
        assert_eq!(tier, TimescaleTier::default());
    }

    /// getter / setter 与 Java 一致（`GanttChartView9/12/14` 的用法）。
    #[test]
    fn accessors_match_java() {
        let mut tier = TimescaleTier::new();
        tier.set_tick_lines(true);
        tier.set_uses_fiscal_year(true);
        tier.set_units(TimescaleUnits::Months);
        tier.set_count(2);
        tier.set_format(TimescaleFormat::get_instance(9));
        tier.set_alignment(TimescaleAlignment::get_instance(1));

        assert!(tier.get_tick_lines());
        assert!(tier.get_uses_fiscal_year());
        assert_eq!(tier.get_units(), Some(TimescaleUnits::Months));
        assert_eq!(tier.get_count(), 2);
        assert_eq!(tier.get_format(), Some(TimescaleFormat::MonthsMmmm));
        assert_eq!(tier.get_alignment(), Some(TimescaleAlignment::Center));
    }

    /// `TimescaleFormat.getInstance` 对未占用下标返回 `None`，setter 照原样保存。
    #[test]
    fn format_may_be_missing() {
        let mut tier = TimescaleTier::new();
        tier.set_format(TimescaleFormat::get_instance(75));
        assert_eq!(tier.get_format(), None);
    }

    /// toString 格式与 Java 一致。
    #[test]
    fn to_string_matches_java() {
        let mut tier = TimescaleTier::new();
        assert_eq!(
            tier.to_string(),
            "[TimescaleTier UsesFiscalYear=false TickLines=false Units=null Count=0 Format=[null] Alignment=null]"
        );

        tier.set_uses_fiscal_year(true);
        tier.set_tick_lines(true);
        tier.set_units(TimescaleUnits::Years);
        tier.set_count(1);
        tier.set_format(Some(TimescaleFormat::None));
        tier.set_alignment(TimescaleAlignment::Left);
        assert_eq!(
            tier.to_string(),
            "[TimescaleTier UsesFiscalYear=true TickLines=true Units=Years Count=1 Format=[None] Alignment=Left]"
        );
    }

    /// `TimescaleUnitsHelper` 的返回值可以直接交给 setter（与 Java 调用点一致）。
    #[test]
    fn units_helper_output_can_be_assigned_directly() {
        use crate::mpp::timescale_units_helper::TimescaleUnitsHelper;

        let mut tier = TimescaleTier::new();
        tier.set_units(TimescaleUnitsHelper::get_instance(8));
        assert_eq!(tier.get_units(), Some(TimescaleUnits::Years));

        tier.set_units(TimescaleUnitsHelper::get_instance(100));
        assert_eq!(tier.get_units(), Some(TimescaleUnits::None));
    }
}
