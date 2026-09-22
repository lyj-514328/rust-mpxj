//! Corresponds to Java: `org/mpxj/mpx/LocaleData_de.java`
//!
//! # Porting trade-offs (`ResourceBundle` / `ListResourceBundle`)
//!
//! Java's `LocaleData_de` is a German resource bundle that overrides only some keys;
//! keys it does not override fall back to the base class via `ResourceBundle`. On the Rust
//! side the keys this language actually overrides are written as module-level `pub const`
//! (scalar constants named after the Java `RESOURCE_DATA` keys, array constants keeping the
//! Java field names) and aggregated in [`OVERRIDES`] (corresponding to the return value of
//! `getContents()`); keys that are not overridden are `None` and fall back to the English
//! default resource in the `locale_data` module. See the `locale_data` module header for the
//! lookup logic; the `ListResourceBundle` protocol methods are replaced by the constant
//! table and are not ported.
//!
//! Keys not overridden by this language: `PROGRAM_NAME`, `CODE_PAGE`, `CURRENCY_DIGITS`, `DATE_ORDER`,
//! `DEFAULT_START_TIME`, `TIME_SEPARATOR`, `BAR_TEXT_DATE_FORMAT`.
//!
//! Skipped Java methods/members: none.

use crate::currency_symbol_position::CurrencySymbolPosition;
use crate::mpx::locale_data::LocaleOverrides;
use crate::project_date_format::ProjectDateFormat;
use crate::project_time_format::ProjectTimeFormat;

/// German time unit alias table.
///
/// Corresponds to Java: `LocaleData_de.TIME_UNITS_ARRAY_DATA` (Java literal order: minutes, hours,
/// days, weeks, months, years, percent, followed by the elapsed units; index 5 is years and
/// index 6 is percent, which differs from the `TimeUnit` value order; Java is copied verbatim here).
pub const TIME_UNITS_ARRAY_DATA: [&'static [&'static str]; 14] = [
    &["m"],    // 0 TimeUnit.MINUTES
    &["h"],    // 1 TimeUnit.HOURS
    &["t"],    // 2 TimeUnit.DAYS
    &["w"],    // 3 TimeUnit.WEEKS
    &["mon"],  // 4 TimeUnit.MONTHS
    &["y"],    // 5 TimeUnit.PERCENT (Java literal position)
    &["%"],    // 6 TimeUnit.YEARS (Java literal position)
    &["fm"],   // 7 TimeUnit.ELAPSED_MINUTES
    &["fh"],   // 8 TimeUnit.ELAPSED_HOURS
    &["ft"],   // 9 TimeUnit.ELAPSED_DAYS
    &["fw"],   // 10 TimeUnit.ELAPSED_WEEKS
    &["fmon"], // 11 TimeUnit.ELAPSED_MONTHS
    &["fy"],   // 12 TimeUnit.ELAPSED_YEARS
    &["f%"],   // 13 TimeUnit.ELAPSED_PERCENT
];

/// German name table for resource key `ACCRUE_TYPES`.
///
/// Corresponds to Java: `LocaleData_de.ACCRUE_TYPES_DATA`.
pub const ACCRUE_TYPES_DATA: [Option<&'static str>; 3] =
    [Some("Anfang"), Some("Ende"), Some("Anteilig")];

/// German name table for resource key `RELATION_TYPES`.
///
/// Corresponds to Java: `LocaleData_de.RELATION_TYPES_DATA`.
pub const RELATION_TYPES_DATA: [Option<&'static str>; 4] =
    [Some("EE"), Some("EA"), Some("AE"), Some("AA")];

/// German name table for resource key `PRIORITY_TYPES`.
///
/// Corresponds to Java: `LocaleData_de.PRIORITY_TYPES_DATA`.
pub const PRIORITY_TYPES_DATA: [Option<&'static str>; 10] = [
    Some("Am niedrigsten"),
    Some("Sehr niedrig"),
    Some("Niedriger"),
    Some("Niedrig"),
    Some("Mittel"),
    Some("Hoch"),
    Some("Höher"),
    Some("Sehr hoch"),
    Some("Am höchsten"),
    Some("Nicht abgleichen"),
];

/// German name table for resource key `CONSTRAINT_TYPES`.
///
/// Corresponds to Java: `LocaleData_de.CONSTRAINT_TYPES_DATA`.
pub const CONSTRAINT_TYPES_DATA: [Option<&'static str>; 8] = [
    Some("So früh wie möglich"),
    Some("So spät wie möglich"),
    Some("Muss anfangen am"),
    Some("Muss enden am"),
    Some("Anfang nicht früher als"),
    Some("Anfang nicht später als"),
    Some("Ende nicht früher als"),
    Some("Ende nicht später als"),
];

/// German name table for resource key `TASK_NAMES` (indices correspond to MPX task field numbers; the end-of-line comments are the English default names).
///
/// Corresponds to Java: `LocaleData_de.TASK_NAMES_DATA`.
pub const TASK_NAMES_DATA: [Option<&'static str>; 153] = [
    None,                                 // -
    Some("Name"),                         // Name
    Some("PSP-Code"),                     // WBS
    Some("Gliederungsebene"),             // Outline Level
    Some("Text1"),                        // Text1
    Some("Text2"),                        // Text2
    Some("Text3"),                        // Text3
    Some("Text4"),                        // Text4
    Some("Text5"),                        // Text5
    Some("Text6"),                        // Text6
    Some("Text7"),                        // Text7
    Some("Text8"),                        // Text8
    Some("Text9"),                        // Text9
    Some("Text10"),                       // Text10
    Some("Notizen"),                      // Notes
    Some("Kontaktperson"),                // Contact
    Some("Ressourcengruppe"),             // Resource Group
    None,                                 // -
    None,                                 // -
    None,                                 // -
    Some("Arbeit"),                       // Work
    Some("Geplante Arbeit"),              // Baseline Work
    Some("Aktuelle Arbeit"),              // Actual Work
    Some("Verbleibende Arbeit"),          // Remaining Work
    Some("Abweichung Arbeit"),            // Work Variance
    Some("% Arbeit abgeschlossen"),       // % Work Complete
    None,                                 // -
    None,                                 // -
    None,                                 // -
    None,                                 // -
    Some("Kosten"),                       // Cost
    Some("Geplante Kosten"),              // Baseline Cost
    Some("Aktuelle Kosten"),              // Actual Cost
    Some("Verbleibende Kosten"),          // Remaining Cost
    Some("Abweichung Kosten"),            // Cost Variance
    Some("Feste Kosten"),                 // Fixed Cost
    Some("Kosten1"),                      // Cost1
    Some("Kosten2"),                      // Cost2
    Some("Kosten3"),                      // Cost3
    None,                                 // -
    Some("Dauer"),                        // Duration
    Some("Geplante Dauer"),               // Baseline Duration
    Some("Aktuelle Dauer"),               // Actual Duration
    Some("Verbleibende Dauer"),           // Remaining Duration
    Some("% Abgeschlossen"),              // % Complete
    Some("Abweichung Dauer"),             // Duration Variance
    Some("Dauer1"),                       // Duration1
    Some("Dauer2"),                       // Duration2
    Some("Dauer3"),                       // Duration3
    None,                                 // -
    Some("Anfang"),                       // Start
    Some("Ende"),                         // Finish
    Some("Frühester Anfang"),             // Early Start
    Some("Frühestes Ende"),               // Early Finish
    Some("Spätester Anfang"),             // Late Start
    Some("Spätestes Ende"),               // Late Finish
    Some("Geplanter Anfang"),             // Baseline Start
    Some("Geplantes Ende"),               // Baseline Finish
    Some("Aktueller Anfang"),             // Actual Start
    Some("Aktuelles Ende"),               // Actual Finish
    Some("Anfang1"),                      // Start1
    Some("Ende1"),                        // Finish1
    Some("Anfang2"),                      // Start2
    Some("Ende2"),                        // Finish2
    Some("Anfang3"),                      // Start3
    Some("Ende3"),                        // Finish3
    Some("Abweichung Anfang"),            // Start Variance
    Some("Abweichung Ende"),              // Finish Variance
    Some("Einschränkungstermin"),         // Constraint Date
    None,                                 // -
    Some("Vorgänger"),                    // Predecessors
    Some("Nachfolger"),                   // Successors
    Some("Ressourcenamen"),               // Resource Names
    Some("Ressourcenkürzel"),             // Resource Initials
    Some("Einmalige Nr. für Vorgänger"),  // Unique ID Predecessors
    Some("Einmalige Nr. für Nachfolger"), // Unique ID Successors
    None,                                 // -
    None,                                 // -
    None,                                 // -
    None,                                 // -
    Some("Fest"),                         // Fixed
    Some("Meilenstein"),                  // Milestone
    Some("Kritisch"),                     // Critical
    Some("Markiert"),                     // Marked
    Some("Rollup"),                       // Rollup
    Some("SKAA"),                         // BCWS
    Some("SKBA"),                         // BCWP
    Some("PA"),                           // SV
    Some("KA"),                           // CV
    None,                                 // -
    Some("Nr."),                          // ID
    Some("Einschränkungsart"),            // Constraint Type
    Some("Verzögerung"),                  // Delay
    Some("Freie Pufferzeit"),             // Free Slack
    Some("Gesamte Pufferzeit"),           // Total Slack
    Some("Priorität"),                    // Priority
    Some("Teilprojektdatei"),             // Subproject File
    Some("Projekt"),                      // Project
    Some("Einmalige Nr."),                // Unique ID
    Some("Gliederungsnummer"),            // Outline Number
    None,                                 // -
    None,                                 // -
    None,                                 // -
    None,                                 // -
    None,                                 // -
    None,                                 // -
    None,                                 // -
    None,                                 // -
    None,                                 // -
    None,                                 // -
    Some("Attribut1"),                    // Flag1
    Some("Attribut2"),                    // Flag2
    Some("Attribut3"),                    // Flag3
    Some("Attribut4"),                    // Flag4
    Some("Attribut5"),                    // Flag5
    Some("Attribut6"),                    // Flag6
    Some("Attribut7"),                    // Flag7
    Some("Attribut8"),                    // Flag8
    Some("Attribut9"),                    // Flag9
    Some("Attribut10"),                   // Flag10
    Some("Sammelvorgang"),                // Summary
    Some("Objekte"),                      // Objects
    Some("Verknüpfte Felder"),            // Linked Fields
    Some("Balken ausblenden"),            // Hide Bar
    None,                                 // -
    Some("Erzeugt"),                      // Created
    Some("Anfang4"),                      // Start4
    Some("Ende4"),                        // Finish4
    Some("Anfang5"),                      // Start5
    Some("Ende5"),                        // Finish5
    None,                                 // -
    None,                                 // -
    None,                                 // -
    None,                                 // -
    None,                                 // -
    Some("Bestätigt"),                    // Confirmed
    Some("Aktualisierung erforderlich"),  // Update Needed
    None,                                 // -
    None,                                 // -
    None,                                 // -
    Some("Zahl1"),                        // Number1
    Some("Zahl2"),                        // Number2
    Some("Zahl3"),                        // Number3
    Some("Zahl4"),                        // Number4
    Some("Zahl5"),                        // Number5
    None,                                 // -
    None,                                 // -
    None,                                 // -
    None,                                 // -
    None,                                 // -
    Some("Unterbrechungstermin"),         // Stop
    Some("Wiederaufnahme nicht früher als"), // Resume No Earlier Than
    Some("Wiederaufnahme"),               // Resume
];

/// German name table for resource key `RESOURCE_NAMES` (indices correspond to MPX resource field numbers; the end-of-line comments are the English default names).
///
/// Corresponds to Java: `LocaleData_de.RESOURCE_NAMES_DATA`.
pub const RESOURCE_NAMES_DATA: [Option<&'static str>; 52] = [
    None,                          // -
    Some("Name"),                  // Name
    Some("Kürzel"),                // Initials
    Some("Gruppe"),                // Group
    Some("Code"),                  // Code
    Some("Text1"),                 // Text1
    Some("Text2"),                 // Text2
    Some("Text3"),                 // Text3
    Some("Text4"),                 // Text4
    Some("Text5"),                 // Text5
    Some("Notizen"),               // Notes
    Some("E-Mail-Adresse"),        // Email Address
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    Some("Arbeit"),                // Work
    Some("Geplante Arbeit"),       // Baseline Work
    Some("Aktuelle Arbeit"),       // Actual Work
    Some("Verbleibende Arbeit"),   // Remaining Work
    Some("Überstundenarbeit"),     // Overtime Work
    Some("Abweichung Arbeit"),     // Work Variance
    Some("% Arbeit abgeschlossen"), // % Work Complete
    None,                          // -
    None,                          // -
    None,                          // -
    Some("Kosten"),                // Cost
    Some("Geplante Kosten"),       // Baseline Cost
    Some("Aktuelle Kosten"),       // Actual Cost
    Some("Verbleibende Kosten"),   // Remaining Cost
    Some("Abweichung Kosten"),     // Cost Variance
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    Some("Nr."),                   // ID
    Some("Max. Einheiten"),        // Max Units
    Some("Standardsatz"),          // Standard Rate
    Some("Überstundensatz"),       // Overtime Rate
    Some("Kosten pro Einsatz"),    // Cost Per Use
    Some("Fällig am"),             // Accrue At
    Some("Überlastet"),            // Overallocated
    Some("Spitze"),                // Peak
    Some("Basiskalender"),         // Base Calendar
    Some("Einmalige Nr."),         // Unique ID
    Some("Objekte"),               // Objects
    Some("Verknüpfte Felder"),     // Linked Fields
];

/// Corresponds to resource key `FILE_DELIMITER`
pub const FILE_DELIMITER: char = ';';
/// Corresponds to resource key `FILE_VERSION`
pub const FILE_VERSION: &'static str = "4,0";
/// Corresponds to resource key `YES`
pub const YES: &'static str = "Ja";
/// Corresponds to resource key `NO`
pub const NO: &'static str = "Nein";
/// Corresponds to resource key `CURRENCY_SYMBOL`
pub const CURRENCY_SYMBOL: &'static str = "€";
/// Corresponds to resource key `CURRENCY_SYMBOL_POSITION`
pub const CURRENCY_SYMBOL_POSITION: CurrencySymbolPosition = CurrencySymbolPosition::AfterWithSpace;
/// Corresponds to resource key `CURRENCY_THOUSANDS_SEPARATOR`
pub const CURRENCY_THOUSANDS_SEPARATOR: char = '.';
/// Corresponds to resource key `CURRENCY_DECIMAL_SEPARATOR`
pub const CURRENCY_DECIMAL_SEPARATOR: char = ',';
/// Corresponds to resource key `TIME_FORMAT`
pub const TIME_FORMAT: ProjectTimeFormat = ProjectTimeFormat::TwentyFourHour;
/// Corresponds to resource key `DATE_SEPARATOR`
pub const DATE_SEPARATOR: char = '.';
/// Corresponds to resource key `AM_TEXT`
pub const AM_TEXT: &'static str = "";
/// Corresponds to resource key `PM_TEXT`
pub const PM_TEXT: &'static str = "";
/// Corresponds to resource key `DATE_FORMAT`
pub const DATE_FORMAT: ProjectDateFormat = ProjectDateFormat::EeeDdMmYy;
/// Corresponds to resource key `NA`
pub const NA: &'static str = "NV";

/// Contents of the German resource bundle, corresponding to Java `LocaleData_de.RESOURCE_DATA`
/// (i.e. the return value of `getContents()`).
pub static OVERRIDES: LocaleOverrides = LocaleOverrides {
    file_delimiter: Some(FILE_DELIMITER),
    program_name: None,
    file_version: Some(FILE_VERSION),
    code_page: None,
    currency_symbol: Some(CURRENCY_SYMBOL),
    currency_symbol_position: Some(CURRENCY_SYMBOL_POSITION),
    currency_digits: None,
    currency_thousands_separator: Some(CURRENCY_THOUSANDS_SEPARATOR),
    currency_decimal_separator: Some(CURRENCY_DECIMAL_SEPARATOR),
    date_order: None,
    time_format: Some(TIME_FORMAT),
    default_start_time: None,
    date_separator: Some(DATE_SEPARATOR),
    time_separator: None,
    am_text: Some(AM_TEXT),
    pm_text: Some(PM_TEXT),
    date_format: Some(DATE_FORMAT),
    bar_text_date_format: None,
    na: Some(NA),
    yes: Some(YES),
    no: Some(NO),
    time_units_array: Some(&TIME_UNITS_ARRAY_DATA),
    accrue_types: Some(&ACCRUE_TYPES_DATA),
    relation_types: Some(&RELATION_TYPES_DATA),
    priority_types: Some(&PRIORITY_TYPES_DATA),
    constraint_types: Some(&CONSTRAINT_TYPES_DATA),
    task_names: Some(&TASK_NAMES_DATA),
    resource_names: Some(&RESOURCE_NAMES_DATA),
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpx::locale_data::LocaleData;
    use crate::mpx::locale_utility::MpxLocale;

    #[test]
    fn array_lengths_match_java() {
        assert_eq!(TIME_UNITS_ARRAY_DATA.len(), 14);
        assert_eq!(ACCRUE_TYPES_DATA.len(), 3);
        assert_eq!(RELATION_TYPES_DATA.len(), 4);
        assert_eq!(PRIORITY_TYPES_DATA.len(), 10);
        assert_eq!(CONSTRAINT_TYPES_DATA.len(), 8);
        assert_eq!(TASK_NAMES_DATA.len(), 153);
        assert_eq!(RESOURCE_NAMES_DATA.len(), 52);
    }

    #[test]
    fn sampled_elements_match_java() {
        assert_eq!(TASK_NAMES_DATA[1], Some("Name"));
        assert_eq!(TASK_NAMES_DATA[2], Some("PSP-Code"));
        assert_eq!(TASK_NAMES_DATA[3], Some("Gliederungsebene"));
        assert_eq!(TASK_NAMES_DATA[52], Some("Frühester Anfang"));
        assert_eq!(TASK_NAMES_DATA[74], Some("Einmalige Nr. für Vorgänger"));
        assert_eq!(TASK_NAMES_DATA[85], Some("SKAA"));
        assert_eq!(TASK_NAMES_DATA[152], Some("Wiederaufnahme"));

        assert_eq!(RESOURCE_NAMES_DATA[2], Some("Kürzel"));
        assert_eq!(RESOURCE_NAMES_DATA[24], Some("Überstundenarbeit"));
        assert_eq!(RESOURCE_NAMES_DATA[51], Some("Verknüpfte Felder"));

        assert_eq!(TIME_UNITS_ARRAY_DATA[2], ["t"].as_slice());
        assert_eq!(TIME_UNITS_ARRAY_DATA[11], ["fmon"].as_slice());
        assert_eq!(ACCRUE_TYPES_DATA[2], Some("Anteilig"));
        assert_eq!(PRIORITY_TYPES_DATA[6], Some("Höher"));
        assert_eq!(CONSTRAINT_TYPES_DATA[0], Some("So früh wie möglich"));
    }

    #[test]
    fn locale_data_returns_german_tables() {
        assert_eq!(
            LocaleData::get_string_array(MpxLocale::De, LocaleData::TASK_NAMES),
            Some(TASK_NAMES_DATA.as_slice())
        );
        assert_eq!(
            LocaleData::get_string_arrays(MpxLocale::De, LocaleData::TIME_UNITS_ARRAY),
            Some(TIME_UNITS_ARRAY_DATA.as_slice())
        );
        assert_eq!(LocaleData::get_string(MpxLocale::De, LocaleData::YES), Some("Ja"));
        assert_eq!(LocaleData::get_char(MpxLocale::De, LocaleData::FILE_DELIMITER), Some(';'));
        // Keys that are not overridden fall back to English
        assert_eq!(
            LocaleData::get_string(MpxLocale::De, LocaleData::PROGRAM_NAME),
            Some("Microsoft Project for Windows")
        );
        assert_eq!(
            LocaleData::get_char(MpxLocale::De, LocaleData::TIME_SEPARATOR),
            Some(':')
        );
    }
}
