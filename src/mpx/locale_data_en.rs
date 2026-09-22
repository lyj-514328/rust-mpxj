//! Corresponds to Java: `org/mpxj/mpx/LocaleData_en.java`
//!
//! # Porting trade-offs (`ResourceBundle` / `ListResourceBundle`)
//!
//! Java's `LocaleData_en` is an empty resource bundle (`RESOURCE_DATA = {}`): English overrides no
//! keys, and all values come from the base class `LocaleData` default resource. On the Rust side
//! [`LocaleOverrides::EMPTY`] (corresponding to the return value of `getContents()`) expresses the
//! same fact; see the `locale_data` module header for the lookup logic. The `ListResourceBundle`
//! protocol methods (`getContents` / `handleGetObject` / `getKeys`) are replaced by that constant
//! and are not ported.
//!
//! Skipped Java methods/members: none.

use crate::mpx::locale_data::LocaleOverrides;

/// Contents of the English resource bundle, i.e. "overrides no keys".
///
/// Corresponds to Java: `LocaleData_en.RESOURCE_DATA` (an `Object[][]` of length 0).
pub static OVERRIDES: LocaleOverrides = LocaleOverrides::EMPTY;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpx::locale_data::LocaleData;
    use crate::mpx::locale_utility::MpxLocale;

    #[test]
    fn english_bundle_overrides_nothing() {
        // Java: LocaleData_en.RESOURCE_DATA = {}, all values fall back to the LocaleData default resource
        assert_eq!(OVERRIDES.task_names, None);
        assert_eq!(OVERRIDES.time_units_array, None);
        assert_eq!(OVERRIDES.accrue_types, None);

        assert_eq!(
            LocaleData::get_string(MpxLocale::En, LocaleData::PROGRAM_NAME),
            Some("Microsoft Project for Windows")
        );
        assert_eq!(
            LocaleData::get_string_array(MpxLocale::En, LocaleData::TASK_NAMES).map(<[Option<&str>]>::len),
            Some(153)
        );
        assert_eq!(
            LocaleData::get_map(MpxLocale::En, LocaleData::TIME_UNITS_MAP)
                .and_then(|map| map.get("w").copied()),
            Some(3)
        );
    }
}
