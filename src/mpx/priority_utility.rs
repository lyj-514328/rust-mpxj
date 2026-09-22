//! Corresponds to Java: `org/mpxj/mpx/PriorityUtility.java`
//!
//! # Locale name tables
//!
//! Java uses `LocaleData.getStringArray(locale, LocaleData.PRIORITY_TYPES)` to obtain from
//! `ResourceBundle` the localised priority name table (non-English languages are overridden by
//! resource bundles such as `LocaleData_de`); on the Rust side the same data is held by
//! [`LocaleData`] (localised overrides come from the `locale_data_XX` modules in the same package),
//! so this module obtains the name table **per locale** the same way Java does.
//!
//! Skipped Java methods/members: none.

use crate::mpx::locale_data::LocaleData;
use crate::mpx::locale_utility::{equals_ignore_case, MpxLocale};
use crate::priority::Priority;

/// English (default) priority name table.
///
/// Corresponds to Java: `LocaleData.PRIORITY_TYPES_DATA` ((index + 1) * 100 is the `Priority` value).
const PRIORITY_TYPES: [Option<&str>; 10] = [
    Some("Lowest"),
    Some("Very Low"),
    Some("Lower"),
    Some("Low"),
    Some("Medium"),
    Some("High"),
    Some("Higher"),
    Some("Very High"),
    Some("Highest"),
    Some("Do Not Level"),
];

/// Utility methods for managing `Priority` instances in MPX files.
///
/// Corresponds to Java: `final class PriorityUtility` (the private constructor prevents instantiation; the Rust side expresses this as a fieldless type).
pub struct PriorityUtility;

impl PriorityUtility {
    /// Convert the text form of a priority into the corresponding instance; unrecognised values are treated as the "medium" priority.
    ///
    /// Corresponds to Java: `public static Priority getInstance(Locale locale, String priority)`
    ///
    /// Java explicitly skips the name table lookup for a `null` argument and keeps the default index;
    /// the Rust side uses `Option<&str>`.
    pub fn get_instance(locale: MpxLocale, priority: Option<&str>) -> Priority {
        let mut index = DEFAULT_PRIORITY_INDEX;

        if priority.is_some() {
            for (loop_index, name) in priority_types(locale).iter().enumerate() {
                // Java's name table elements are never null; the Rust side skips `None` elements directly.
                if let Some(name) = name {
                    if equals_ignore_case(name, priority) {
                        index = loop_index;
                        break;
                    }
                }
            }
        }

        // Java: Priority.getInstance((index + 1) * 100)
        Priority::get_instance((index as i32 + 1) * 100)
    }
}

/// Index of the default priority in the name table.
///
/// Corresponds to Java: `private static final int DEFAULT_PRIORITY_INDEX = 4`
const DEFAULT_PRIORITY_INDEX: usize = 4;

/// Obtain the priority name table for the given locale.
///
/// Corresponds to Java: `LocaleData.getStringArray(locale, LocaleData.PRIORITY_TYPES)`
///
/// When the key is missing it falls back to the English default name table (Java's `ResourceBundle`
/// likewise falls back to the base class resource).
fn priority_types(locale: MpxLocale) -> &'static [Option<&'static str>] {
    LocaleData::get_string_array(locale, LocaleData::PRIORITY_TYPES).unwrap_or(&PRIORITY_TYPES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_round_trips_through_mpx_name() {
        for (index, name) in PRIORITY_TYPES.iter().enumerate() {
            let name = name.expect("English name table has no null elements");
            let value = PriorityUtility::get_instance(MpxLocale::En, Some(name));
            assert_eq!(value.get_value(), (index as i32 + 1) * 100);
            let name_index = (value.get_value() / 100 - 1) as usize;
            assert_eq!(PRIORITY_TYPES[name_index], Some(name));
        }
    }

    #[test]
    fn priority_ignores_case() {
        assert_eq!(
            PriorityUtility::get_instance(MpxLocale::En, Some("very HIGH")).get_value(),
            Priority::VERY_HIGH
        );
        assert_eq!(
            PriorityUtility::get_instance(MpxLocale::En, Some("do not level")).get_value(),
            Priority::DO_NOT_LEVEL
        );
    }

    #[test]
    fn priority_falls_back_to_medium() {
        assert_eq!(
            PriorityUtility::get_instance(MpxLocale::En, Some("Unknown")).get_value(),
            Priority::MEDIUM
        );
        assert_eq!(
            PriorityUtility::get_instance(MpxLocale::En, Some(" Medium")).get_value(),
            Priority::MEDIUM
        );
        assert_eq!(
            PriorityUtility::get_instance(MpxLocale::En, None).get_value(),
            Priority::MEDIUM
        );
    }

    #[test]
    fn priority_uses_the_locale_table() {
        // Java: LocaleData_de.PRIORITY_TYPES_DATA[8] = "Am höchsten"
        assert_eq!(
            PriorityUtility::get_instance(MpxLocale::De, Some("Am höchsten")).get_value(),
            Priority::HIGHEST
        );
        // The values in the English name table do not exist in the German name table, so it falls back to "medium"
        assert_eq!(
            PriorityUtility::get_instance(MpxLocale::De, Some("Highest")).get_value(),
            Priority::MEDIUM
        );
    }
}
