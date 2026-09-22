//! Corresponds to Java: `org/mpxj/mpx/LocaleUtility.java`
//!
//! # Porting trade-offs (`java.util.Locale` / `ResourceBundle` / `DateFormatSymbols`)
//!
//! Java's `LocaleUtility` depends on `java.util.Locale` and `ResourceBundle`: all values are read
//! from resource bundles through `LocaleData.getString/getChar/getObject/getInteger`, loading
//! `LocaleData_<lang>` by language (`LocaleData_en` is empty, so English uses all the defaults in `LocaleData`).
//! The Rust standard library has no equivalent, so this module handles it as follows:
//!
//! - [`MpxLocale`], defined in this module, replaces `java.util.Locale` and carries only the language codes MPX cares about
//!   (EN/DE/FR/IT/PT/SV/ZH/ES/RU, that is, Java's `SUPPORTED_LOCALES`);
//! - The resource bundle's "variant override + parent bundle fallback" semantics are carried by [`crate::mpx::locale_data::LocaleData`]
//!   (each language variant's `OVERRIDES` override the defaults in [`crate::mpx::locale_data::RESOURCE_DATA`]);
//!   [`LocaleUtility::set_locale`] calls its `get_string` / `get_char` / `get_integer` /
//!   `get_code_page` etc. key by key, corresponding one-to-one with Java's `ResourceBundle` lookups, so non-English
//!   locales such as German and French get their own currency symbol, separators, date / time formats, and other properties;
//! - Java's `DateFormatSymbols` is not used in this class (MPX date/number formats are handled by `MPXJFormats`)
//!   and is not involved here.
//!
//! Skipped Java methods/members: none (`setLocale` and `getSupportedLocales` are both ported, with `MpxLocale` added).

use chrono::NaiveTime;

use crate::mpx::locale_data::LocaleData;
use crate::project_properties::ProjectProperties;

/// Locale identifier supported by MPX, replacing Java's `java.util.Locale`.
///
/// Corresponds to Java: constants such as `new Locale("EN")` in `LocaleUtility.SUPPORTED_LOCALES`.
///
/// Java's `Locale` also includes country, variant, and language display name; MPX needs only the language code, so only
/// the language dimension is kept here. Java's `Locale` implements `equals` / `hashCode`; the Rust side corresponds with derived implementations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MpxLocale {
    /// English.
    En,
    /// German.
    De,
    /// French.
    Fr,
    /// Italian.
    It,
    /// Portuguese.
    Pt,
    /// Swedish.
    Sv,
    /// Chinese.
    Zh,
    /// Spanish.
    Es,
    /// Russian.
    Ru,
}

impl MpxLocale {
    /// All supported locales, in the same order as Java's `SUPPORTED_LOCALES`.
    ///
    /// Corresponds to Java: `private static final Locale[] SUPPORTED_LOCALES`
    pub const ALL: [MpxLocale; 9] = [
        MpxLocale::En,
        MpxLocale::De,
        MpxLocale::Fr,
        MpxLocale::It,
        MpxLocale::Pt,
        MpxLocale::Sv,
        MpxLocale::Zh,
        MpxLocale::Es,
        MpxLocale::Ru,
    ];

    /// Return the locale for a language code, case-insensitively (Java's `Locale(String)` lowercases the language code).
    ///
    /// Corresponds to Java: constructions such as `new Locale("EN")`. Returns `None` when unrecognized
    /// (Java would obtain a `Locale` not in `SUPPORTED_LOCALES`, whose resource resolution falls back to the defaults,
    /// equivalent to the English fallback here).
    pub fn get_instance(language: &str) -> Option<MpxLocale> {
        MpxLocale::ALL
            .iter()
            .find(|locale| language.eq_ignore_ascii_case(locale.get_language()))
            .copied()
    }

    /// Return the language code (lowercase).
    ///
    /// Corresponds to Java: `Locale.getLanguage()`
    pub fn get_language(&self) -> &'static str {
        match self {
            MpxLocale::En => "en",
            MpxLocale::De => "de",
            MpxLocale::Fr => "fr",
            MpxLocale::It => "it",
            MpxLocale::Pt => "pt",
            MpxLocale::Sv => "sv",
            MpxLocale::Zh => "zh",
            MpxLocale::Es => "es",
            MpxLocale::Ru => "ru",
        }
    }
}

impl std::fmt::Display for MpxLocale {
    /// Corresponds to Java: `Locale.toString()` (the lowercase language code when only a language is present).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_language())
    }
}

/// Locale utility methods for MPX files.
///
/// Corresponds to Java: `final class LocaleUtility` (a private constructor prevents instantiation; the Rust side uses a fieldless type).
pub struct LocaleUtility;

impl LocaleUtility {
    /// When the parent file's locale is updated, reset locale-related properties to the new locale's defaults.
    ///
    /// Corresponds to Java: `public static void setLocale(ProjectProperties properties, Locale locale)`
    ///
    /// Trade-off: Java obtains localized resources by `locale` through `ResourceBundle` (variant override + English default fallback);
    /// the Rust side delegates this semantics to [`LocaleData`]'s same-named key accessors, corresponding item by item to Java's
    /// `LocaleData.getString/getChar/getObject/getInteger`.
    ///
    /// Note: Java's `getChar` / `getObject` throw `MissingResourceException` for missing keys (these keys always exist in the
    /// resource bundle); the Rust side's `get_char` returns `None` when no value is found, so here we only set the value when one
    /// is found, to avoid panicking.
    pub fn set_locale(properties: &mut ProjectProperties, locale: MpxLocale) {
        // ---- MPX-specific properties ----
        // Java: properties.setMpxDelimiter(LocaleData.getChar(locale, LocaleData.FILE_DELIMITER));
        if let Some(delimiter) = LocaleData::get_char(locale, LocaleData::FILE_DELIMITER) {
            properties.set_mpx_delimiter(delimiter);
        }

        // Java: properties.setMpxProgramName(LocaleData.getString(locale, LocaleData.PROGRAM_NAME));
        properties.set_mpx_program_name(
            LocaleData::get_string(locale, LocaleData::PROGRAM_NAME).map(str::to_owned),
        );

        // Java: properties.setMpxCodePage((CodePage) LocaleData.getObject(locale, LocaleData.CODE_PAGE));
        properties.set_mpx_code_page(Some(LocaleData::get_code_page(locale)));

        // Java: properties.setDateSeparator(LocaleData.getChar(locale, LocaleData.DATE_SEPARATOR));
        if let Some(separator) = LocaleData::get_char(locale, LocaleData::DATE_SEPARATOR) {
            properties.set_date_separator(separator);
        }

        // Java: properties.setTimeSeparator(LocaleData.getChar(locale, LocaleData.TIME_SEPARATOR));
        if let Some(separator) = LocaleData::get_char(locale, LocaleData::TIME_SEPARATOR) {
            properties.set_time_separator(separator);
        }

        // Java: properties.setTimeFormat((ProjectTimeFormat) LocaleData.getObject(locale, LocaleData.TIME_FORMAT));
        properties.set_time_format(Some(LocaleData::get_time_format(locale)));

        // Java: properties.setDateOrder((DateOrder) LocaleData.getObject(locale, LocaleData.DATE_ORDER));
        properties.set_date_order(Some(LocaleData::get_date_order(locale)));

        // Java: properties.setAMText(LocaleData.getString(locale, LocaleData.AM_TEXT));
        properties
            .set_am_text(LocaleData::get_string(locale, LocaleData::AM_TEXT).map(str::to_owned));

        // Java: properties.setPMText(LocaleData.getString(locale, LocaleData.PM_TEXT));
        properties
            .set_pm_text(LocaleData::get_string(locale, LocaleData::PM_TEXT).map(str::to_owned));

        // Java: properties.setDateFormat((ProjectDateFormat) LocaleData.getObject(locale, LocaleData.DATE_FORMAT));
        //       properties.setBarTextDateFormat((ProjectDateFormat) LocaleData.getObject(locale, LocaleData.DATE_FORMAT));
        let date_format = LocaleData::get_date_format(locale);
        properties.set_date_format(Some(date_format));
        properties.set_bar_text_date_format(Some(date_format));

        // ---- General properties ----
        // Java: properties.setCurrencySymbol(LocaleData.getString(locale, LocaleData.CURRENCY_SYMBOL));
        properties.set_currency_symbol(
            LocaleData::get_string(locale, LocaleData::CURRENCY_SYMBOL).map(str::to_owned),
        );

        // Java: properties.setSymbolPosition((CurrencySymbolPosition) LocaleData.getObject(locale, LocaleData.CURRENCY_SYMBOL_POSITION));
        properties.set_symbol_position(Some(LocaleData::get_currency_symbol_position(locale)));

        // Java: properties.setCurrencyDigits(LocaleData.getInteger(locale, LocaleData.CURRENCY_DIGITS));
        properties
            .set_currency_digits(LocaleData::get_integer(locale, LocaleData::CURRENCY_DIGITS));

        // Java: properties.setThousandsSeparator(LocaleData.getChar(locale, LocaleData.CURRENCY_THOUSANDS_SEPARATOR));
        if let Some(separator) =
            LocaleData::get_char(locale, LocaleData::CURRENCY_THOUSANDS_SEPARATOR)
        {
            properties.set_thousands_separator(separator);
        }

        // Java: properties.setDecimalSeparator(LocaleData.getChar(locale, LocaleData.CURRENCY_DECIMAL_SEPARATOR));
        if let Some(separator) =
            LocaleData::get_char(locale, LocaleData::CURRENCY_DECIMAL_SEPARATOR)
        {
            properties.set_decimal_separator(separator);
        }

        // Java: LocalTime.ofSecondOfDay(LocaleData.getInteger(locale, DEFAULT_START_TIME) * 60L)
        // For an out-of-bounds value Java throws DateTimeException; the Rust side ignores the property (returns None).
        if let Some(time) = LocaleData::get_integer(locale, LocaleData::DEFAULT_START_TIME)
            .and_then(time_of_day_from_minutes)
        {
            properties.set_default_start_time(Some(time));
        }
    }

    /// Return the locales supported by MPX functionality.
    ///
    /// Corresponds to Java: `public static Locale[] getSupportedLocales()`
    pub fn get_supported_locales() -> &'static [MpxLocale] {
        &MpxLocale::ALL
    }
}

/// Convert "minutes since midnight" into a time, corresponding to Java's `LocalTime.ofSecondOfDay(minutes * 60L)`.
///
/// Java throws `DateTimeException` for out-of-bounds values (negative or >= 86400 seconds); the Rust side does not panic and returns `None`.
fn time_of_day_from_minutes(minutes: i32) -> Option<NaiveTime> {
    let seconds = i64::from(minutes) * 60;
    if !(0..86_400).contains(&seconds) {
        return None;
    }

    NaiveTime::from_num_seconds_from_midnight_opt(seconds as u32, 0)
}

/// Case-insensitive string comparison.
///
/// Corresponds to Java: `java.lang.String.equalsIgnoreCase` (a JDK method, not an MPXJ member).
/// Java compares character by character using `Character.toLowerCase/toUpperCase`, while the Rust side approximates
/// with `str::to_lowercase` (the two may differ in case folding for a few characters).
/// A `null` argument corresponds to `None`: Java's `a.equalsIgnoreCase(null)` returns `false`.
pub(crate) fn equals_ignore_case(name: &str, value: Option<&str>) -> bool {
    match value {
        None => false,
        Some(value) => name.to_lowercase() == value.to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Weak;

    use super::*;
    use crate::code_page::CodePage;

    /// Build a project properties instance for testing (no parent file).
    fn properties() -> ProjectProperties {
        ProjectProperties::new(Weak::new())
    }

    #[test]
    fn mpx_locale_round_trips_through_all() {
        assert_eq!(LocaleUtility::get_supported_locales().len(), 9);
        for locale in MpxLocale::ALL {
            assert_eq!(MpxLocale::get_instance(locale.get_language()), Some(locale));
            // Java: new Locale("EN") lowercases the language code; the constructor argument is case-insensitive.
            assert_eq!(
                MpxLocale::get_instance(&locale.get_language().to_uppercase()),
                Some(locale)
            );
            assert_eq!(locale.to_string(), locale.get_language());
        }
    }

    #[test]
    fn mpx_locale_rejects_unknown_languages() {
        assert_eq!(MpxLocale::get_instance("zz"), None);
        assert_eq!(MpxLocale::get_instance(""), None);
        assert_eq!(MpxLocale::get_instance("eng"), None);
    }

    #[test]
    fn set_locale_applies_english_defaults() {
        let mut properties = properties();
        LocaleUtility::set_locale(&mut properties, MpxLocale::En);

        assert_eq!(properties.get_mpx_delimiter(), ',');
        assert_eq!(properties.get_mpx_program_name(), "Microsoft Project for Windows");
        assert_eq!(properties.get_mpx_code_page(), CodePage::Ansi);
        assert_eq!(properties.get_date_separator(), '/');
        assert_eq!(properties.get_time_separator(), ':');
        assert_eq!(properties.get_currency_symbol(), "$");
        assert_eq!(
            properties.get_default_start_time(),
            NaiveTime::from_hms_opt(8, 0, 0).unwrap()
        );
    }

    /// Non-English locales take their values from their localized resources (before the fix they always fell back to the English default resources).
    ///
    /// Expected values match Java's `LocaleData_de` / `LocaleData_fr` (see the constants in `locale_data_de` /
    /// `locale_data_fr`).
    #[test]
    fn set_locale_applies_localised_values_for_non_english_locales() {
        // German: delimiter ';', date separator '.', 24-hour clock, currency symbol trailing with a space
        let mut german_properties = properties();
        LocaleUtility::set_locale(&mut german_properties, MpxLocale::De);

        assert_eq!(german_properties.get_mpx_delimiter(), ';');
        assert_eq!(german_properties.get_mpx_code_page(), CodePage::Ansi);
        assert_eq!(german_properties.get_date_separator(), '.');
        assert_eq!(german_properties.get_time_separator(), ':');
        assert_eq!(german_properties.get_thousands_separator(), '.');
        assert_eq!(german_properties.get_decimal_separator(), ',');
        assert_eq!(german_properties.get_currency_symbol(), "€");
        assert_eq!(
            german_properties.get_default_start_time(),
            NaiveTime::from_hms_opt(8, 0, 0).unwrap()
        );

        // French: delimiter ';', currency symbol is an empty string
        let mut french_properties = properties();
        LocaleUtility::set_locale(&mut french_properties, MpxLocale::Fr);

        assert_eq!(french_properties.get_mpx_delimiter(), ';');
        assert_eq!(french_properties.get_currency_symbol(), "");
    }

    #[test]
    fn time_of_day_conversion_matches_java() {
        assert_eq!(time_of_day_from_minutes(480), NaiveTime::from_hms_opt(8, 0, 0));
        assert_eq!(time_of_day_from_minutes(0), NaiveTime::from_hms_opt(0, 0, 0));
        // Java: LocalTime.ofSecondOfDay throws DateTimeException for out-of-bounds values.
        assert_eq!(time_of_day_from_minutes(-1), None);
        assert_eq!(time_of_day_from_minutes(24 * 60), None);
    }

    #[test]
    fn equals_ignore_case_matches_java_semantics() {
        assert!(equals_ignore_case("Start", Some("start")));
        assert!(equals_ignore_case("Start", Some("START")));
        assert!(!equals_ignore_case("Start", Some("End")));
        // Java: "Start".equalsIgnoreCase(null) == false
        assert!(!equals_ignore_case("Start", None));
    }
}
