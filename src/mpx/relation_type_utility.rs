//! Corresponds to Java: `org/mpxj/mpx/RelationTypeUtility.java`
//!
//! # Locale name tables
//!
//! Java uses `LocaleData.getStringArray(locale, LocaleData.RELATION_TYPES)` to obtain from
//! `ResourceBundle` the localised relation type name table (non-English languages are overridden by
//! resource bundles such as `LocaleData_de`); on the Rust side the same data is held by
//! [`LocaleData`] (localised overrides come from the `locale_data_XX` modules in the same package),
//! so this module obtains the name table **per locale** the same way Java does -- the `EA` in the
//! German file can only be recognised through the German name table.
//!
//! Skipped Java methods/members: none.

use crate::mpx::locale_data::LocaleData;
use crate::mpx::locale_utility::{equals_ignore_case, MpxLocale};
use crate::relation_type::RelationType;

/// English (default) relation type name table.
///
/// Corresponds to Java: `LocaleData.RELATION_TYPES_DATA` (the index is the `RelationType` int value).
const RELATION_TYPES: [Option<&str>; 4] = [Some("FF"), Some("FS"), Some("SF"), Some("SS")];

/// Utility methods for managing `RelationType` instances in MPX files.
///
/// Corresponds to Java: `final class RelationTypeUtility` (the private constructor prevents instantiation; the Rust side expresses this as a fieldless type).
pub struct RelationTypeUtility;

impl RelationTypeUtility {
    /// Convert the text form of a relation type into the corresponding enum instance; unrecognised values return `None`.
    ///
    /// Corresponds to Java: `public static RelationType getInstance(Locale locale, String type)`
    ///
    /// Java returns `null` for unrecognised values (including `null`, since `equalsIgnoreCase(null)`
    /// is always `false`); the Rust side uses `Option<RelationType>`.
    pub fn get_instance(locale: MpxLocale, r#type: Option<&str>) -> Option<RelationType> {
        let mut index = -1;

        for (loop_index, name) in relation_types(locale).iter().enumerate() {
            // Java's name table elements are never null; the Rust side skips `None` elements directly.
            if let Some(name) = name {
                if equals_ignore_case(name, r#type) {
                    index = loop_index as i32;
                    break;
                }
            }
        }

        if index != -1 {
            // Java: RelationType.getInstance(index)
            Some(RelationType::get_instance(index))
        } else {
            None
        }
    }
}

/// Obtain the relation type name table for the given locale.
///
/// Corresponds to Java: `LocaleData.getStringArray(locale, LocaleData.RELATION_TYPES)`
///
/// When the key is missing it falls back to the English default name table (Java's `ResourceBundle`
/// likewise falls back to the base class resource).
fn relation_types(locale: MpxLocale) -> &'static [Option<&'static str>] {
    LocaleData::get_string_array(locale, LocaleData::RELATION_TYPES).unwrap_or(&RELATION_TYPES)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpxj_enum::MpxjEnum;

    #[test]
    fn relation_type_round_trips_through_mpx_name() {
        for (index, name) in RELATION_TYPES.iter().enumerate() {
            let name = name.expect("English name table has no empty elements");
            let value = RelationTypeUtility::get_instance(MpxLocale::En, Some(name))
                .expect("values in the English name table should be recognizable");
            assert_eq!(value.value(), index as i32);
            assert_eq!(RELATION_TYPES[value.value() as usize], Some(name));
            assert_eq!(value.get_name(), name);
        }
    }

    #[test]
    fn relation_type_ignores_case() {
        assert_eq!(
            RelationTypeUtility::get_instance(MpxLocale::En, Some("fs")),
            Some(RelationType::FinishStart)
        );
        assert_eq!(
            RelationTypeUtility::get_instance(MpxLocale::En, Some("ss")),
            Some(RelationType::StartStart)
        );
    }

    #[test]
    fn relation_type_returns_none_for_unrecognised_values() {
        assert_eq!(RelationTypeUtility::get_instance(MpxLocale::En, Some("XX")), None);
        assert_eq!(RelationTypeUtility::get_instance(MpxLocale::En, Some("")), None);
        assert_eq!(RelationTypeUtility::get_instance(MpxLocale::En, Some(" FS")), None);
        assert_eq!(RelationTypeUtility::get_instance(MpxLocale::En, None), None);
    }

    #[test]
    fn relation_type_uses_the_locale_table() {
        // Java: LocaleData_de.RELATION_TYPES_DATA = { "EE", "EA", "AE", "AA" }
        assert_eq!(
            RelationTypeUtility::get_instance(MpxLocale::De, Some("EA")),
            Some(RelationType::FinishStart)
        );
        assert_eq!(
            RelationTypeUtility::get_instance(MpxLocale::De, Some("AA")),
            Some(RelationType::StartStart)
        );
        // The values in the English name table do not exist in the German name table
        assert_eq!(
            RelationTypeUtility::get_instance(MpxLocale::De, Some("FS")),
            None
        );
    }
}
