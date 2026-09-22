//! Corresponds to Java: `org/mpxj/mpx/AccrueTypeUtility.java`
//!
//! # Locale name tables
//!
//! Java uses `LocaleData.getStringArray(locale, LocaleData.ACCRUE_TYPES)` to obtain from
//! `ResourceBundle` the localised cost accrue type name table (non-English languages are overridden
//! by resource bundles such as `LocaleData_de`); on the Rust side the same data is held by
//! [`LocaleData`] (localised overrides come from the `locale_data_XX` modules in the same package),
//! so this module obtains the name table **per locale** the same way Java does.
//!
//! Skipped Java methods/members: none.

use crate::accrue_type::AccrueType;
use crate::mpx::locale_data::LocaleData;
use crate::mpx::locale_utility::{equals_ignore_case, MpxLocale};

/// English (default) cost accrue type name table.
///
/// Corresponds to Java: `LocaleData.ACCRUE_TYPES_DATA` (index + 1 is the `AccrueType` int value).
const ACCRUE_TYPES: [Option<&str>; 3] = [Some("Start"), Some("End"), Some("Prorated")];

/// Utility methods for managing `AccrueType` instances in MPX files.
///
/// Corresponds to Java: `final class AccrueTypeUtility` (the private constructor prevents instantiation; the Rust side expresses this as a fieldless type).
pub struct AccrueTypeUtility;

impl AccrueTypeUtility {
    /// Convert the text form of a cost accrue type into the corresponding enum instance; unrecognised values are treated as "prorated".
    ///
    /// Corresponds to Java: `public static AccrueType getInstance(String type, Locale locale)`
    ///
    /// Java's `type` is a `String` (it may be `null`, and `equalsIgnoreCase(null)` is always `false`);
    /// the Rust side uses `Option<&str>`, where `None` is equivalent to "unrecognised".
    pub fn get_instance(r#type: Option<&str>, locale: MpxLocale) -> AccrueType {
        let mut result = None;

        for (loop_index, name) in accrue_types(locale).iter().enumerate() {
            // Java's name table elements are never null; the Rust side skips `None` elements directly.
            if let Some(name) = name {
                if equals_ignore_case(name, r#type) {
                    // Java: AccrueType.getInstance(loop + 1)
                    result = AccrueType::get_instance((loop_index + 1) as i32);
                    break;
                }
            }
        }

        // Java: result == null -> AccrueType.PRORATED
        result.unwrap_or(AccrueType::Prorated)
    }
}

/// Obtain the cost accrue type name table for the given locale.
///
/// Corresponds to Java: `LocaleData.getStringArray(locale, LocaleData.ACCRUE_TYPES)`
///
/// When the key is missing it falls back to the English default name table (Java's `ResourceBundle`
/// likewise falls back to the base class resource).
fn accrue_types(locale: MpxLocale) -> &'static [Option<&'static str>] {
    LocaleData::get_string_array(locale, LocaleData::ACCRUE_TYPES).unwrap_or(&ACCRUE_TYPES)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpxj_enum::MpxjEnum;

    #[test]
    fn accrue_type_round_trips_through_mpx_name() {
        for (index, name) in ACCRUE_TYPES.iter().enumerate() {
            let name = name.expect("English name table has no empty elements");
            let value = AccrueTypeUtility::get_instance(Some(name), MpxLocale::En);
            assert_eq!(value.value(), (index + 1) as i32);
            assert_eq!(ACCRUE_TYPES[(value.value() - 1) as usize], Some(name));
        }
    }

    #[test]
    fn accrue_type_ignores_case() {
        assert_eq!(
            AccrueTypeUtility::get_instance(Some("start"), MpxLocale::En),
            AccrueType::Start
        );
        assert_eq!(
            AccrueTypeUtility::get_instance(Some("PRORATED"), MpxLocale::En),
            AccrueType::Prorated
        );
    }

    #[test]
    fn accrue_type_falls_back_to_prorated() {
        assert_eq!(
            AccrueTypeUtility::get_instance(Some("Unknown"), MpxLocale::En),
            AccrueType::Prorated
        );
        assert_eq!(
            AccrueTypeUtility::get_instance(Some(" Start "), MpxLocale::En),
            AccrueType::Prorated
        );
        assert_eq!(
            AccrueTypeUtility::get_instance(None, MpxLocale::En),
            AccrueType::Prorated
        );
    }

    #[test]
    fn accrue_type_uses_the_locale_table() {
        // Java: LocaleData_de.ACCRUE_TYPES_DATA = { "Anfang", "Ende", "Anteilig" }
        assert_eq!(
            AccrueTypeUtility::get_instance(Some("Ende"), MpxLocale::De),
            AccrueType::End
        );
        assert_eq!(
            AccrueTypeUtility::get_instance(Some("Anteilig"), MpxLocale::De),
            AccrueType::Prorated
        );
        // The values in the English name table do not exist in the German name table, so it falls back to "prorated"
        assert_eq!(
            AccrueTypeUtility::get_instance(Some("Start"), MpxLocale::De),
            AccrueType::Prorated
        );
    }
}
