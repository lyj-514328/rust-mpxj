//! Corresponds to Java: `org/mpxj/mpp/GanttBarDateFormat.java`
//!
//! Date formats that can be displayed on the Gantt chart timescale.
//!
//! # Porting trade-offs
//!
//! - Java uses reflection via `EnumHelper.createTypeArray(GanttBarDateFormat.class)` to generate
//! the int → enum lookup table (array length = constant count 22; values 0..21 exactly fill it, **but the array index is
//! `m_value` rather than the declaration ordinal**, so `DDMMYYYY(21)` lands at index 21); Rust does not use reflection,
//! and instead uses [`GanttBarDateFormat::ALL`] + the `match` in [`GanttBarDateFormat::get_instance`].
//! - **`Ord` is not derived**: the natural order of a Java enum is the declaration order (`DDMMYYYY` has ordinal 3),
//! while its discriminant value is 21; the two are inconsistent, and deriving `Ord` would introduce ordering semantics that Java does not have.
//! - Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use them.
//!
//! Unported Java members: none.

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// Date format for the Gantt chart timescale.
///
/// Corresponds to Java: `public enum GanttBarDateFormat implements MpxjEnum`
///
/// The declaration order matches Java, and the discriminant value is Java's `m_value` (note that `Ddmmyyyy` has discriminant value 21).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GanttBarDateFormat {
    /// Corresponds to Java: `DEFAULT(0, "Default")`
    Default = 0,
    /// Corresponds to Java: `DDMMYY_MMSS(1, "28/01/02 12:33")`
    DdmmyyMmss = 1,
    /// Corresponds to Java: `DDMMYY(2, "28/01/02")`
    Ddmmyy = 2,
    /// Corresponds to Java: `DDMMYYYY(21, "28/01/2002")`
    Ddmmyyyy = 21,
    /// Corresponds to Java: `DD_MMMM_YYYY_HHMM(3, "28 January 2002 12:33")`
    DdMmmmYyyyHhmm = 3,
    /// Corresponds to Java: `DD_MMMM_YYYY(4, "28 January 2002")`
    DdMmmmYyyy = 4,
    /// Corresponds to Java: `DD_MMM_HHMM(5, "28 Jan 12:33")`
    DdMmmHhmm = 5,
    /// Corresponds to Java: `DD_MMM_YY(6, "28 Jan '02")`
    DdMmmYy = 6,
    /// Corresponds to Java: `DD_MMMM(7, "28 January")`
    DdMmmm = 7,
    /// Corresponds to Java: `DD_MMM(8, "28 Jan")`
    DdMmm = 8,
    /// Corresponds to Java: `DDD_DDMMYY_HHMM(9, "Mon 28/01/02 12:33")`
    DddDdmmyyHhmm = 9,
    /// Corresponds to Java: `DDD_DDMMYY(10, "Mon 28/01/02")`
    DddDdmmyy = 10,
    /// Corresponds to Java: `DDD_DD_MMM_YY(11, "Mon 28 Jan '02")`
    DddDdMmmYy = 11,
    /// Corresponds to Java: `DDD_HHMM(12, "Mon 12:33")`
    DddHhmm = 12,
    /// Corresponds to Java: `DDD_DD_MMM(16, "Mon 28 Jan")`
    DddDdMmm = 16,
    /// Corresponds to Java: `DDD_DDMM(17, "Mon 28/01")`
    DddDdmm = 17,
    /// Corresponds to Java: `DDD_DD(18, "Mon 28")`
    DddDd = 18,
    /// Corresponds to Java: `DDMM(13, "28/01")`
    Ddmm = 13,
    /// Corresponds to Java: `DD(14, "28")`
    Dd = 14,
    /// Corresponds to Java: `HHMM(15, "12:33")`
    Hhmm = 15,
    /// Corresponds to Java: `MWW(19, "1/W05")`
    Mww = 19,
    /// Corresponds to Java: `MWWYY_HHMM(20, "1/W05/02 12:33")`
    MwwyyHhmm = 20,
}

impl GanttBarDateFormat {
/// All values, in the same order as declared in Java.
    ///
/// Corresponds to Java: `private static final GanttBarDateFormat[] TYPE_VALUES` (after removing the reflective table construction).
    pub const ALL: [GanttBarDateFormat; 22] = [
        GanttBarDateFormat::Default,
        GanttBarDateFormat::DdmmyyMmss,
        GanttBarDateFormat::Ddmmyy,
        GanttBarDateFormat::Ddmmyyyy,
        GanttBarDateFormat::DdMmmmYyyyHhmm,
        GanttBarDateFormat::DdMmmmYyyy,
        GanttBarDateFormat::DdMmmHhmm,
        GanttBarDateFormat::DdMmmYy,
        GanttBarDateFormat::DdMmmm,
        GanttBarDateFormat::DdMmm,
        GanttBarDateFormat::DddDdmmyyHhmm,
        GanttBarDateFormat::DddDdmmyy,
        GanttBarDateFormat::DddDdMmmYy,
        GanttBarDateFormat::DddHhmm,
        GanttBarDateFormat::DddDdMmm,
        GanttBarDateFormat::DddDdmm,
        GanttBarDateFormat::DddDd,
        GanttBarDateFormat::Ddmm,
        GanttBarDateFormat::Dd,
        GanttBarDateFormat::Hhmm,
        GanttBarDateFormat::Mww,
        GanttBarDateFormat::MwwyyHhmm,
    ];

/// Get the enum instance by int value; on out of bounds (including negative values) it falls back to `DEFAULT`.
    ///
    /// Corresponds to Java: `public static GanttBarDateFormat getInstance(int type)`
    pub fn get_instance(value: i32) -> GanttBarDateFormat {
        match value {
            0 => GanttBarDateFormat::Default,
            1 => GanttBarDateFormat::DdmmyyMmss,
            2 => GanttBarDateFormat::Ddmmyy,
            3 => GanttBarDateFormat::DdMmmmYyyyHhmm,
            4 => GanttBarDateFormat::DdMmmmYyyy,
            5 => GanttBarDateFormat::DdMmmHhmm,
            6 => GanttBarDateFormat::DdMmmYy,
            7 => GanttBarDateFormat::DdMmmm,
            8 => GanttBarDateFormat::DdMmm,
            9 => GanttBarDateFormat::DddDdmmyyHhmm,
            10 => GanttBarDateFormat::DddDdmmyy,
            11 => GanttBarDateFormat::DddDdMmmYy,
            12 => GanttBarDateFormat::DddHhmm,
            13 => GanttBarDateFormat::Ddmm,
            14 => GanttBarDateFormat::Dd,
            15 => GanttBarDateFormat::Hhmm,
            16 => GanttBarDateFormat::DddDdMmm,
            17 => GanttBarDateFormat::DddDdmm,
            18 => GanttBarDateFormat::DddDd,
            19 => GanttBarDateFormat::Mww,
            20 => GanttBarDateFormat::MwwyyHhmm,
            21 => GanttBarDateFormat::Ddmmyyyy,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = DEFAULT.getValue(); }`
            _ => GanttBarDateFormat::Default,
        }
    }

    /// Corresponds to Java: `public static GanttBarDateFormat getInstance(Number type)`
    ///
/// On the Java side `null` goes through `getInstance(-1)`, which ultimately falls back to `DEFAULT`.
    pub fn get_instance_optional(value: Option<i32>) -> GanttBarDateFormat {
        GanttBarDateFormat::get_instance(value.unwrap_or(-1))
    }

/// Get the format name (i.e. the format example, such as `"28 January 2002"`), matching Java, not localized.
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            GanttBarDateFormat::Default => "Default",
            GanttBarDateFormat::DdmmyyMmss => "28/01/02 12:33",
            GanttBarDateFormat::Ddmmyy => "28/01/02",
            GanttBarDateFormat::Ddmmyyyy => "28/01/2002",
            GanttBarDateFormat::DdMmmmYyyyHhmm => "28 January 2002 12:33",
            GanttBarDateFormat::DdMmmmYyyy => "28 January 2002",
            GanttBarDateFormat::DdMmmHhmm => "28 Jan 12:33",
            GanttBarDateFormat::DdMmmYy => "28 Jan '02",
            GanttBarDateFormat::DdMmmm => "28 January",
            GanttBarDateFormat::DdMmm => "28 Jan",
            GanttBarDateFormat::DddDdmmyyHhmm => "Mon 28/01/02 12:33",
            GanttBarDateFormat::DddDdmmyy => "Mon 28/01/02",
            GanttBarDateFormat::DddDdMmmYy => "Mon 28 Jan '02",
            GanttBarDateFormat::DddHhmm => "Mon 12:33",
            GanttBarDateFormat::DddDdMmm => "Mon 28 Jan",
            GanttBarDateFormat::DddDdmm => "Mon 28/01",
            GanttBarDateFormat::DddDd => "Mon 28",
            GanttBarDateFormat::Ddmm => "28/01",
            GanttBarDateFormat::Dd => "28",
            GanttBarDateFormat::Hhmm => "12:33",
            GanttBarDateFormat::Mww => "1/W05",
            GanttBarDateFormat::MwwyyHhmm => "1/W05/02 12:33",
        }
    }
}

impl MpxjEnum for GanttBarDateFormat {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for GanttBarDateFormat {
/// Corresponds to Java: `@Override public String toString()`, i.e. `getName()`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl From<GanttBarDateFormat> for i32 {
    fn from(value: GanttBarDateFormat) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_get_instance() {
        for item in GanttBarDateFormat::ALL {
            assert_eq!(GanttBarDateFormat::get_instance(item.value()), item);
        }
    }

    #[test]
    fn out_of_range_falls_back_to_default() {
        assert_eq!(GanttBarDateFormat::get_instance(-1), GanttBarDateFormat::Default);
        assert_eq!(GanttBarDateFormat::get_instance(22), GanttBarDateFormat::Default);
        assert_eq!(
            GanttBarDateFormat::get_instance_optional(None),
            GanttBarDateFormat::Default
        );
    }

    #[test]
    fn lookup_uses_value_not_declaration_index() {
// DDMMYYYY has m_value 21 (even though it is declared in the 4th position)
        assert_eq!(GanttBarDateFormat::get_instance(21), GanttBarDateFormat::Ddmmyyyy);
        assert_eq!(GanttBarDateFormat::get_instance(3), GanttBarDateFormat::DdMmmmYyyyHhmm);
    }

    #[test]
    fn names_match_java() {
        assert_eq!(GanttBarDateFormat::Default.to_string(), "Default");
        assert_eq!(GanttBarDateFormat::DdMmmmYyyy.to_string(), "28 January 2002");
        assert_eq!(GanttBarDateFormat::DdMmmYy.to_string(), "28 Jan '02");
    }
}
