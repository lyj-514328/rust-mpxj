//! Corresponds to Java: `org/mpxj/mpx/TimeUnitUtility.java`
//!
//! # Locale name tables
//!
//! Java uses `LocaleData.getMap(locale, LocaleData.TIME_UNITS_MAP)` to obtain from `ResourceBundle`
//! the "lower-case name -> `TimeUnit` int value" mapping, which is expanded from each language's
//! `TIME_UNITS_ARRAY_DATA` (each unit may have multiple aliases, such as `"d"` / `"days"`). On the
//! Rust side the same data is held by [`LocaleData`] (localised overrides come from the
//! `locale_data_XX` modules in the same package), so this module obtains the name table **per
//! locale** the same way Java does -- the `1t` (Tage) in the German file can only be recognised
//! through the German name table.
//!
//! Skipped Java methods/members: none.

use crate::mpx::locale_data::LocaleData;
use crate::mpx::locale_utility::MpxLocale;
use crate::time_unit::TimeUnit;

/// English (default) time unit name table.
///
/// Corresponds to Java: `LocaleData.TIME_UNITS_ARRAY_DATA` (the index is the `TimeUnit` int value,
/// and the inner array holds all aliases of that unit).
const TIME_UNITS_ARRAY: [&[&str]; 14] = [
    &["m", "mins"],         // TimeUnit.MINUTES
    &["h", "hours"],        // TimeUnit.HOURS
    &["d", "days"],         // TimeUnit.DAYS
    &["w", "wk", "weeks"],  // TimeUnit.WEEKS
    &["mon", "months"],     // TimeUnit.MONTHS
    &["%"],                 // TimeUnit.PERCENT
    &["y", "years"],        // TimeUnit.YEARS
    &["em"],                // TimeUnit.ELAPSED_MINUTES
    &["eh"],                // TimeUnit.ELAPSED_HOURS
    &["ed"],                // TimeUnit.ELAPSED_DAYS
    &["ew"],                // TimeUnit.ELAPSED_WEEKS
    &["emon"],              // TimeUnit.ELAPSED_MONTHS
    &["ey"],                // TimeUnit.ELAPSED_YEARS
    &["e%"],                // TimeUnit.ELAPSED_PERCENT
];

/// Utility methods for managing `TimeUnit` instances in MPX files.
///
/// Corresponds to Java: `final class TimeUnitUtility` (the private constructor prevents instantiation; the Rust side expresses this as a fieldless type).
pub struct TimeUnitUtility;

impl TimeUnitUtility {
    /// Convert the string form of a time unit into the corresponding enum instance.
    ///
    /// Corresponds to Java: `public static TimeUnit getInstance(String units, Locale locale) throws MPXJException`
    ///
    /// Java calls `toLowerCase()` on the argument before looking it up; when it cannot be
    /// recognised it throws `MPXJException(INVALID_TIME_UNIT + " " + units)`, whereas the Rust side
    /// returns `None`. Java's `units` is a `String` (`null` triggers a `NullPointerException`), while
    /// the Rust side uses `Option<&str>`, and `None` likewise returns `None`.
    pub fn get_instance(units: Option<&str>, locale: MpxLocale) -> Option<TimeUnit> {
        let units = units?.to_lowercase();

        for (value, names) in time_units_array(locale).iter().enumerate() {
            if names.contains(&units.as_str()) {
                return Some(TimeUnit::get_instance(value as i32));
            }
        }

        None
    }
}

/// Obtain the time unit name table for the given locale.
///
/// Corresponds to Java: `LocaleData.getMap(locale, LocaleData.TIME_UNITS_MAP)` (whose data source is
/// `TIME_UNITS_ARRAY_DATA`)
///
/// When the key is missing it falls back to the English default name table (Java's `ResourceBundle`
/// likewise falls back to the base class resource).
fn time_units_array(locale: MpxLocale) -> &'static [&'static [&'static str]] {
    LocaleData::get_string_arrays(locale, LocaleData::TIME_UNITS_ARRAY).unwrap_or(&TIME_UNITS_ARRAY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_unit_round_trips_through_mpx_names() {
        for (value, names) in TIME_UNITS_ARRAY.iter().enumerate() {
            let unit = TimeUnit::get_instance(value as i32);
            for name in names.iter() {
                // MPX name -> model value
                assert_eq!(
                    TimeUnitUtility::get_instance(Some(name), MpxLocale::En),
                    Some(unit)
                );
                // Model value -> MPX name (the preferred name in the name table) -> parsed back to the model value
                assert_eq!(
                    TimeUnitUtility::get_instance(Some(names[0]), MpxLocale::En),
                    Some(unit)
                );
            }
        }
    }

    #[test]
    fn time_unit_name_is_not_always_an_mpx_name() {
        // Java's `TimeUnit.getName()` returns "mo" / "emo" for MONTHS / ELAPSED_MONTHS,
        // whereas the MPX English name table uses "mon" / "emon", so the two do not agree.
        assert_eq!(TimeUnit::Months.get_name(), "mo");
        assert_eq!(TimeUnitUtility::get_instance(Some("mon"), MpxLocale::En), Some(TimeUnit::Months));
        assert_eq!(TimeUnitUtility::get_instance(Some("mo"), MpxLocale::En), None);
        assert_eq!(TimeUnitUtility::get_instance(Some("emon"), MpxLocale::En), Some(TimeUnit::ElapsedMonths));
        assert_eq!(TimeUnitUtility::get_instance(Some("emo"), MpxLocale::En), None);
    }

    #[test]
    fn time_unit_is_case_insensitive() {
        // Java: map.get(units.toLowerCase())
        assert_eq!(
            TimeUnitUtility::get_instance(Some("H"), MpxLocale::En),
            Some(TimeUnit::Hours)
        );
        assert_eq!(
            TimeUnitUtility::get_instance(Some("DAYS"), MpxLocale::En),
            Some(TimeUnit::Days)
        );
        assert_eq!(
            TimeUnitUtility::get_instance(Some("Mon"), MpxLocale::En),
            Some(TimeUnit::Months)
        );
    }

    #[test]
    fn time_unit_rejects_unrecognised_values() {
        // Java: throws MPXJException(INVALID_TIME_UNIT)
        assert_eq!(TimeUnitUtility::get_instance(Some("x"), MpxLocale::En), None);
        assert_eq!(TimeUnitUtility::get_instance(Some("d "), MpxLocale::En), None);
        assert_eq!(TimeUnitUtility::get_instance(Some(""), MpxLocale::En), None);
        assert_eq!(TimeUnitUtility::get_instance(None, MpxLocale::En), None);
    }

    #[test]
    fn time_unit_uses_the_locale_table() {
        // Java: LocaleData_de.TIME_UNITS_ARRAY_DATA uses "t" for days (Tage)
        assert_eq!(
            TimeUnitUtility::get_instance(Some("t"), MpxLocale::De),
            Some(TimeUnit::Days)
        );
        assert_eq!(
            TimeUnitUtility::get_instance(Some("h"), MpxLocale::De),
            Some(TimeUnit::Hours)
        );
        // The "d"/"days" entries in the English name table do not exist in the German name table
        assert_eq!(TimeUnitUtility::get_instance(Some("d"), MpxLocale::De), None);
        // The "t" in the English name table is not any unit either
        assert_eq!(TimeUnitUtility::get_instance(Some("t"), MpxLocale::En), None);
    }
}
