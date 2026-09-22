//! Corresponds to Java: `org/mpxj/ProjectDateFormat.java`

use crate::mpxj_enum::MpxjEnum;
use std::fmt::{self, Display, Formatter};

/// Project date format enum.
///
/// Corresponds to Java: `public enum ProjectDateFormat implements MpxjEnum`
///
/// The Java side uses `EnumHelper.createTypeArray` reflection to generate the int → enum array;
/// Rust does not use reflection; instead it uses [`ProjectDateFormat::ALL`] plus
/// the `match` in [`ProjectDateFormat::get_instance`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProjectDateFormat {
/// e.g. 25/12/98 12:56.
    ///
    /// Corresponds to Java: `DD_MM_YY_HH_MM(0)`
    DdMmYyHhMm = 0,
/// e.g. 25/05/98.
    ///
    /// Corresponds to Java: `DD_MM_YY(1)`
    DdMmYy = 1,
/// e.g. 13 December 2002 12:56.
    ///
    /// Corresponds to Java: `DD_MMMMM_YYYY_HH_MM(2)`
    DdMmmmmYyyyHhMm = 2,
/// e.g. 13 December 2002.
    ///
    /// Corresponds to Java: `DD_MMMMM_YYYY(3)`
    DdMmmmmYyyy = 3,
/// e.g. 24 Nov 12:56.
    ///
    /// Corresponds to Java: `DD_MMM_HH_MM(4)`
    DdMmmHhMm = 4,
/// e.g. 25 Aug '98.
    ///
    /// Corresponds to Java: `DD_MMM_YY(5)`
    DdMmmYy = 5,
/// e.g. 25 September.
    ///
    /// Corresponds to Java: `DD_MMMMM(6)`
    DdMmmmm = 6,
/// e.g. 25 Aug.
    ///
    /// Corresponds to Java: `DD_MMM(7)`
    DdMmm = 7,
/// e.g. Thu 25/05/98 12:56.
    ///
    /// Corresponds to Java: `EEE_DD_MM_YY_HH_MM(8)`
    EeeDdMmYyHhMm = 8,
/// e.g. Wed 25/05/98.
    ///
    /// Corresponds to Java: `EEE_DD_MM_YY(9)`
    EeeDdMmYy = 9,
/// e.g. Wed 25 Mar '98.
    ///
    /// Corresponds to Java: `EEE_DD_MMM_YY(10)`
    EeeDdMmmYy = 10,
/// e.g. Wed 12:56.
    ///
    /// Corresponds to Java: `EEE_HH_MM(11)`
    EeeHhMm = 11,
/// e.g. 25/5.
    ///
    /// Corresponds to Java: `DD_MM(12)`
    DdMm = 12,
/// e.g. 23.
    ///
    /// Corresponds to Java: `DD(13)`
    Dd = 13,
/// e.g. 12:56.
    ///
    /// Corresponds to Java: `HH_MM(14)`
    HhMm = 14,
/// e.g. Wed 23 Mar.
    ///
    /// Corresponds to Java: `EEE_DD_MMM(15)`
    EeeDdMmm = 15,
/// e.g. Wed 25/5.
    ///
    /// Corresponds to Java: `EEE_DD_MM(16)`
    EeeDdMm = 16,
/// e.g. Wed 05.
    ///
    /// Corresponds to Java: `EEE_DD(17)`
    EeeDd = 17,
/// e.g. 5/W25.
    ///
    /// Corresponds to Java: `DD_WWW(18)`
    DdWww = 18,
/// e.g. 5/W25/98 12:56.
    ///
    /// Corresponds to Java: `DD_WWW_YY_HH_MM(19)`
    DdWwwYyHhMm = 19,
/// e.g. 25/05/1998.
    ///
    /// Corresponds to Java: `DD_MM_YYYY(20)`
    DdMmYyyy = 20,
}

impl ProjectDateFormat {
/// All values, sorted ascending by int value.
    ///
/// Corresponds to Java: `private static final ProjectDateFormat[] TYPE_VALUES` (excluding the reflection part).
    pub const ALL: [ProjectDateFormat; 21] = [
        ProjectDateFormat::DdMmYyHhMm,
        ProjectDateFormat::DdMmYy,
        ProjectDateFormat::DdMmmmmYyyyHhMm,
        ProjectDateFormat::DdMmmmmYyyy,
        ProjectDateFormat::DdMmmHhMm,
        ProjectDateFormat::DdMmmYy,
        ProjectDateFormat::DdMmmmm,
        ProjectDateFormat::DdMmm,
        ProjectDateFormat::EeeDdMmYyHhMm,
        ProjectDateFormat::EeeDdMmYy,
        ProjectDateFormat::EeeDdMmmYy,
        ProjectDateFormat::EeeHhMm,
        ProjectDateFormat::DdMm,
        ProjectDateFormat::Dd,
        ProjectDateFormat::HhMm,
        ProjectDateFormat::EeeDdMmm,
        ProjectDateFormat::EeeDdMm,
        ProjectDateFormat::EeeDd,
        ProjectDateFormat::DdWww,
        ProjectDateFormat::DdWwwYyHhMm,
        ProjectDateFormat::DdMmYyyy,
    ];

/// Gets the enum instance by int value; falls back to `DD_MM_YYYY` when out of bounds.
    ///
    /// Corresponds to Java: `public static ProjectDateFormat getInstance(int type)`
    pub fn get_instance(value: i32) -> ProjectDateFormat {
        match value {
            0 => ProjectDateFormat::DdMmYyHhMm,
            1 => ProjectDateFormat::DdMmYy,
            2 => ProjectDateFormat::DdMmmmmYyyyHhMm,
            3 => ProjectDateFormat::DdMmmmmYyyy,
            4 => ProjectDateFormat::DdMmmHhMm,
            5 => ProjectDateFormat::DdMmmYy,
            6 => ProjectDateFormat::DdMmmmm,
            7 => ProjectDateFormat::DdMmm,
            8 => ProjectDateFormat::EeeDdMmYyHhMm,
            9 => ProjectDateFormat::EeeDdMmYy,
            10 => ProjectDateFormat::EeeDdMmmYy,
            11 => ProjectDateFormat::EeeHhMm,
            12 => ProjectDateFormat::DdMm,
            13 => ProjectDateFormat::Dd,
            14 => ProjectDateFormat::HhMm,
            15 => ProjectDateFormat::EeeDdMmm,
            16 => ProjectDateFormat::EeeDdMm,
            17 => ProjectDateFormat::EeeDd,
            18 => ProjectDateFormat::DdWww,
            19 => ProjectDateFormat::DdWwwYyHhMm,
            20 => ProjectDateFormat::DdMmYyyy,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = DD_MM_YYYY.getValue(); }`
            _ => ProjectDateFormat::DdMmYyyy,
        }
    }

    /// Corresponds to Java: `public static ProjectDateFormat getInstance(Number type)`
    ///
/// On the Java side `null` goes through `getInstance(-1)` and finally falls back to `DD_MM_YYYY`.
    pub fn get_instance_optional(value: Option<i32>) -> ProjectDateFormat {
        ProjectDateFormat::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for ProjectDateFormat {
/// Returns the int value associated with this enum.
    ///
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for ProjectDateFormat {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java returns `Integer.toString(m_value)`, i.e. the decimal string of the int value.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as i32)
    }
}

impl From<ProjectDateFormat> for i32 {
    fn from(value: ProjectDateFormat) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_date_format_round_trips() {
        for item in ProjectDateFormat::ALL {
            assert_eq!(ProjectDateFormat::get_instance(item.value()), item);
        }
    }

    #[test]
    fn project_date_format_falls_back_to_dd_mm_yyyy() {
        assert_eq!(
            ProjectDateFormat::get_instance(-1),
            ProjectDateFormat::DdMmYyyy
        );
        assert_eq!(
            ProjectDateFormat::get_instance(21),
            ProjectDateFormat::DdMmYyyy
        );
        assert_eq!(
            ProjectDateFormat::get_instance_optional(None),
            ProjectDateFormat::DdMmYyyy
        );
    }

    #[test]
    fn project_date_format_to_string_is_decimal_value() {
        assert_eq!(ProjectDateFormat::DdMmYyyy.to_string(), "20");
        assert_eq!(ProjectDateFormat::DdMmYyHhMm.to_string(), "0");
    }
}
