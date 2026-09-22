//! Corresponds to Java: `org/mpxj/mpx/ConstraintTypeUtility.java`
//!
//! # Locale name tables
//!
//! Java uses `LocaleData.getStringArray(locale, LocaleData.CONSTRAINT_TYPES)` to obtain from
//! `ResourceBundle` the localised constraint type name table (non-English languages are overridden
//! by resource bundles such as `LocaleData_de`); on the Rust side the same data is held by
//! [`LocaleData`] (localised overrides come from the `locale_data_XX` modules in the same package),
//! so this module obtains the name table **per locale** the same way Java does.
//!
//! Skipped Java methods/members: none.

use crate::constraint_type::ConstraintType;
use crate::mpx::locale_data::LocaleData;
use crate::mpx::locale_utility::{equals_ignore_case, MpxLocale};

/// English (default) constraint type name table.
///
/// Corresponds to Java: `LocaleData.CONSTRAINT_TYPES_DATA` (the order is the `ConstraintType` int value 0..7).
const CONSTRAINT_TYPES: [Option<&str>; 8] = [
    Some("As Soon As Possible"),
    Some("As Late As Possible"),
    Some("Must Start On"),
    Some("Must Finish On"),
    Some("Start No Earlier Than"),
    Some("Start No Later Than"),
    Some("Finish No Earlier Than"),
    Some("Finish No Later Than"),
];

/// Utility methods for managing `ConstraintType` instances in MPX files.
///
/// Corresponds to Java: `final class ConstraintTypeUtility` (the private constructor prevents instantiation; the Rust side expresses this as a fieldless type).
pub struct ConstraintTypeUtility;

impl ConstraintTypeUtility {
    /// Convert the text form of a constraint type into the corresponding enum instance; unrecognised values are treated as "as soon as possible".
    ///
    /// Corresponds to Java: `public static ConstraintType getInstance(Locale locale, String type)`
    ///
    /// Java's `type` is a `String` (it may be `null`, and `equalsIgnoreCase(null)` is always `false`);
    /// the Rust side uses `Option<&str>`, where `None` is equivalent to "unrecognised".
    pub fn get_instance(locale: MpxLocale, r#type: Option<&str>) -> ConstraintType {
        let mut index = 0;

        for (loop_index, name) in constraint_types(locale).iter().enumerate() {
            // Java's name table elements are never null; the Rust side skips `None` elements directly.
            if let Some(name) = name {
                if equals_ignore_case(name, r#type) {
                    index = loop_index;
                    break;
                }
            }
        }

        // Java: ConstraintType.getInstance(index); when index is out of bounds it falls back to AS_SOON_AS_POSSIBLE
        ConstraintType::get_instance(index as i32)
    }
}

/// Obtain the constraint type name table for the given locale.
///
/// Corresponds to Java: `LocaleData.getStringArray(locale, LocaleData.CONSTRAINT_TYPES)`
///
/// When the key is missing it falls back to the English default name table (Java's `ResourceBundle`
/// likewise falls back to the base class resource).
fn constraint_types(locale: MpxLocale) -> &'static [Option<&'static str>] {
    LocaleData::get_string_array(locale, LocaleData::CONSTRAINT_TYPES).unwrap_or(&CONSTRAINT_TYPES)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpxj_enum::MpxjEnum;

    #[test]
    fn constraint_type_round_trips_through_mpx_name() {
        for name in CONSTRAINT_TYPES {
            let name = name.expect("English name table has no null elements");
            let value = ConstraintTypeUtility::get_instance(MpxLocale::En, Some(name));
            // The name table index is the ConstraintType int value; converting back should correspond one-to-one with the name table.
            assert_eq!(CONSTRAINT_TYPES[value.value() as usize], Some(name));
            assert_eq!(
                ConstraintTypeUtility::get_instance(MpxLocale::En, Some(name)),
                value
            );
        }
    }

    #[test]
    fn constraint_type_ignores_case_and_unrecognised_values() {
        assert_eq!(
            ConstraintTypeUtility::get_instance(MpxLocale::En, Some("must start on")),
            ConstraintType::MustStartOn
        );
        assert_eq!(
            ConstraintTypeUtility::get_instance(MpxLocale::En, Some("MUST START ON")),
            ConstraintType::MustStartOn
        );
        // Java: unrecognised values and null both fall back to "as soon as possible"
        assert_eq!(
            ConstraintTypeUtility::get_instance(MpxLocale::En, Some("Unknown")),
            ConstraintType::AsSoonAsPossible
        );
        assert_eq!(
            ConstraintTypeUtility::get_instance(MpxLocale::En, Some("")),
            ConstraintType::AsSoonAsPossible
        );
        assert_eq!(
            ConstraintTypeUtility::get_instance(MpxLocale::En, None),
            ConstraintType::AsSoonAsPossible
        );
    }

    #[test]
    fn constraint_type_without_english_name_falls_back() {
        // StartOn(8) / FinishOn(9) have no corresponding entry in the English name table (the table has only 8 entries).
        assert_eq!(
            ConstraintTypeUtility::get_instance(MpxLocale::En, Some("Start On")),
            ConstraintType::AsSoonAsPossible
        );
    }

    #[test]
    fn constraint_type_uses_the_locale_table() {
        // Java: LocaleData_de.CONSTRAINT_TYPES_DATA[4] = "Anfang nicht früher als"
        assert_eq!(
            ConstraintTypeUtility::get_instance(MpxLocale::De, Some("Anfang nicht früher als")),
            ConstraintType::StartNoEarlierThan
        );
        assert_eq!(
            ConstraintTypeUtility::get_instance(MpxLocale::De, Some("So früh wie möglich")),
            ConstraintType::AsSoonAsPossible
        );
        // The values in the English name table do not exist in the German name table, so it falls back to "as soon as possible"
        assert_eq!(
            ConstraintTypeUtility::get_instance(MpxLocale::De, Some("Must Start On")),
            ConstraintType::AsSoonAsPossible
        );
    }
}
