//! Corresponds to Java: `org/mpxj/mpp/TimescaleTier.java`
//!
//! The set of properties for a single tier (top / middle / bottom) in a Gantt chart timescale.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `public final class TimescaleTier` | Struct [`TimescaleTier`] | 1:1 port; field declaration order matches Java |
//! | Java's implicit no-arg constructor (field defaults `false` / `0` / `null`) | [`TimescaleTier::new`] | Consistent with Java's `new TimescaleTier()`: the two flags are `false`, `count` is 0, and the three enum fields are `None` (Java uses `null`) |
//! | `TimescaleUnits m_units` / `TimescaleFormat m_format` / `TimescaleAlignment m_alignment` (may be `null`) | Fields and getters are all `Option<...>` | The "`null` → `Option<T>`" convention: Java's `TimescaleTier` allows "setting only some properties", and `getUnits()` etc. return `null` before assignment |
//! | `setUnits(TimescaleUnits)` | [`TimescaleTier::set_units`] (parameter is not `Option`) | All Java call sites pass `TimescaleUnitsHelper.getInstance(...)`, which never returns `null`; the parameter keeps the same shape as Java for 1:1 correspondence |
//! | `setAlignment(TimescaleAlignment)` | [`TimescaleTier::set_alignment`] (parameter is not `Option`) | Same as above: all call sites pass `TimescaleAlignment.getInstance(...)`, which never returns `null` |
//! | `setFormat(TimescaleFormat)` | [`TimescaleTier::set_format`] (parameter is `Option<TimescaleFormat>`) | `TimescaleFormat.getInstance(...)` returns `null` for unused indexes (75 / 84 / 122) in the lookup table (see `crate::mpp::timescale_format::TimescaleFormat`), so the parameter uses `Option` |
//! | `toString()` | `Display` | Output format matches Java (when `Units` / `Alignment` are `null` it outputs `null`, and when `Format` is `null` it outputs `[null]`) |
//! | Does not override `equals` / `hashCode` | Derives `PartialEq` / `Eq` / `Hash` | Derived on the Rust side for easier comparison and testing (field-by-field comparison) |
//!
//! Not ported / skipped: none (mechanisms such as JDBC / XML-JAXB / GUI / `javax.*` / `Serializable` are not used by this class).

use std::fmt::{self, Display, Formatter};

use crate::mpp::mpp_utility::format_optional_display;
use crate::mpp::timescale_alignment::TimescaleAlignment;
use crate::mpp::timescale_format::TimescaleFormat;
use crate::timescale_units::TimescaleUnits;

/// A single tier in the timescale.
///
/// Corresponds to Java: `public final class TimescaleTier`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TimescaleTier {
/// Whether to use fiscal years.
    ///
    /// Corresponds to Java: `private boolean m_usesFiscalYear`
    uses_fiscal_year: bool,
/// Whether to show tick lines.
    ///
    /// Corresponds to Java: `private boolean m_tickLines`
    tick_lines: bool,
/// Timescale units.
    ///
    /// Corresponds to Java: `private TimescaleUnits m_units`
    units: Option<TimescaleUnits>,
/// Tier count (number of units contained in each cell).
    ///
    /// Corresponds to Java: `private int m_count`
    count: i32,
/// Label format.
    ///
    /// Corresponds to Java: `private TimescaleFormat m_format`
    format: Option<TimescaleFormat>,
/// Label alignment.
    ///
    /// Corresponds to Java: `private TimescaleAlignment m_alignment`
    alignment: Option<TimescaleAlignment>,
}

impl TimescaleTier {
/// Create an instance with all properties at their Java default values.
    ///
/// Corresponds to Java: `new TimescaleTier()` (implicit no-arg constructor)
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

/// Get the tier count.
    ///
    /// Corresponds to Java: `public int getCount()`
    pub fn get_count(&self) -> i32 {
        self.count
    }

/// Set the tier count.
    ///
    /// Corresponds to Java: `public void setCount(int count)`
    pub fn set_count(&mut self, count: i32) {
        self.count = count;
    }

/// Get the label format.
    ///
    /// Corresponds to Java: `public TimescaleFormat getFormat()`
    pub fn get_format(&self) -> Option<TimescaleFormat> {
        self.format
    }

/// Set the label format.
    ///
    /// Corresponds to Java: `public void setFormat(TimescaleFormat format)`
    pub fn set_format(&mut self, format: Option<TimescaleFormat>) {
        self.format = format;
    }

/// Get whether tick lines are shown.
    ///
    /// Corresponds to Java: `public boolean getTickLines()`
    pub fn get_tick_lines(&self) -> bool {
        self.tick_lines
    }

/// Set whether to show tick lines.
    ///
    /// Corresponds to Java: `public void setTickLines(boolean tickLines)`
    pub fn set_tick_lines(&mut self, tick_lines: bool) {
        self.tick_lines = tick_lines;
    }

/// Get the timescale units.
    ///
    /// Corresponds to Java: `public TimescaleUnits getUnits()`
    pub fn get_units(&self) -> Option<TimescaleUnits> {
        self.units
    }

/// Set the timescale units.
    ///
    /// Corresponds to Java: `public void setUnits(TimescaleUnits units)`
    pub fn set_units(&mut self, units: TimescaleUnits) {
        self.units = Some(units);
    }

/// Get whether fiscal years are used.
    ///
    /// Corresponds to Java: `public boolean getUsesFiscalYear()`
    pub fn get_uses_fiscal_year(&self) -> bool {
        self.uses_fiscal_year
    }

/// Set whether to use fiscal years.
    ///
    /// Corresponds to Java: `public void setUsesFiscalYear(boolean usesFiscalYear)`
    pub fn set_uses_fiscal_year(&mut self, uses_fiscal_year: bool) {
        self.uses_fiscal_year = uses_fiscal_year;
    }

/// Get the label alignment.
    ///
    /// Corresponds to Java: `public TimescaleAlignment getAlignment()`
    pub fn get_alignment(&self) -> Option<TimescaleAlignment> {
        self.alignment
    }

/// Set the label alignment.
    ///
    /// Corresponds to Java: `public void setAlignment(TimescaleAlignment alignment)`
    pub fn set_alignment(&mut self, alignment: TimescaleAlignment) {
        self.alignment = Some(alignment);
    }
}

impl Default for TimescaleTier {
/// Equivalent to Java's implicit no-arg constructor.
    fn default() -> TimescaleTier {
        TimescaleTier::new()
    }
}

impl Display for TimescaleTier {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Concatenates field by field, in exactly the same order as Java.
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

/// The default state of Java's `new TimescaleTier()`.
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

/// Getters / setters match Java (usage in `GanttChartView9/12/14`).
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

/// `TimescaleFormat.getInstance` returns `None` for unused indexes, and the setter stores it as-is.
    #[test]
    fn format_may_be_missing() {
        let mut tier = TimescaleTier::new();
        tier.set_format(TimescaleFormat::get_instance(75));
        assert_eq!(tier.get_format(), None);
    }

/// The toString format matches Java.
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

/// The return value of `TimescaleUnitsHelper` can be passed directly to the setter (consistent with Java call sites).
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
