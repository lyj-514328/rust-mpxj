//! Corresponds to Java: `org/mpxj/mpx/LocaleData_es.java`
//!
//! # Porting trade-offs (`ResourceBundle` / `ListResourceBundle`)
//!
//! Java's `LocaleData_es` is a Spanish resource bundle covering only some keys; keys it
//! does not cover fall back to the base class in `ResourceBundle`. On the Rust side the
//! keys this locale actually covers are written as module-level `pub const` values (scalar
//! constants are named after the Java `RESOURCE_DATA` keys, array constants keep the Java
//! field names) and aggregated by [`OVERRIDES`] (corresponding to the return value of
//! `getContents()`); uncovered keys are `None` and fall back to the English default
//! resources in the `locale_data` module. See the header of the `locale_data` module for
//! the lookup logic; the `ListResourceBundle` protocol methods are replaced by constant
//! tables and are not ported.
//!
//! This locale covers every key in `RESOURCE_DATA` (`DEFAULT_START_TIME` has the same value
//! as the English default).
//!
//! Some entries in the Java source still use their English original names (marked
//! `// TODO Translate`); they are copied here verbatim from the source literals.
//! Note also that several Spanish words in the Java source contain a soft hyphen U+00AD
//! (e.g. `"M\u00ED\u00ADnima"`), preserved here as `\u{ad}`.
//!
//! Skipped Java members: none.

use crate::code_page::CodePage;
use crate::currency_symbol_position::CurrencySymbolPosition;
use crate::date_order::DateOrder;
use crate::mpx::locale_data::LocaleOverrides;
use crate::project_date_format::ProjectDateFormat;
use crate::project_time_format::ProjectTimeFormat;

/// Spanish time unit alias table.
///
/// Corresponds to Java: `LocaleData_es.TIME_UNITS_ARRAY_DATA` (Java literal order: minutes,
/// hours, days, weeks, months, years, percent, followed by the elapsed units; index 5 is
/// years and index 6 is percent, differing from the `TimeUnit` value order, copied from Java).
pub const TIME_UNITS_ARRAY_DATA: [&'static [&'static str]; 14] = [
    &["m"],    // 0 TimeUnit.MINUTES
    &["h"],    // 1 TimeUnit.HOURS
    &["d"],    // 2 TimeUnit.DAYS
    &["s"],    // 3 TimeUnit.WEEKS (Semanas)
    &["ms"],   // 4 TimeUnit.MONTHS (Meses)
    &["a"],    // 5 TimeUnit.PERCENT (Java literal position, Años)
    &["%"],    // 6 TimeUnit.YEARS (Java literal position)
    &["em"],   // 7 TimeUnit.ELAPSED_MINUTES
    &["eh"],   // 8 TimeUnit.ELAPSED_HOURS
    &["ed"],   // 9 TimeUnit.ELAPSED_DAYS
    &["es"],   // 10 TimeUnit.ELAPSED_WEEKS
    &["ems"],  // 11 TimeUnit.ELAPSED_MONTHS
    &["ea"],   // 12 TimeUnit.ELAPSED_YEARS
    &["e%"],   // 13 TimeUnit.ELAPSED_PERCENT
];

/// Spanish resource key `ACCRUE_TYPES` name table.
///
/// Corresponds to Java: `LocaleData_es.ACCRUE_TYPES_DATA`.
pub const ACCRUE_TYPES_DATA: [Option<&'static str>; 3] =
    [Some("Comienzo"), Some("Fin"), Some("Prorrateo")];

/// Spanish resource key `RELATION_TYPES` name table.
///
/// Corresponds to Java: `LocaleData_es.RELATION_TYPES_DATA`.
pub const RELATION_TYPES_DATA: [Option<&'static str>; 4] =
    [Some("FF"), Some("FC"), Some("CF"), Some("CC")];

/// Spanish resource key `PRIORITY_TYPES` name table.
///
/// Corresponds to Java: `LocaleData_es.PRIORITY_TYPES_DATA`.
pub const PRIORITY_TYPES_DATA: [Option<&'static str>; 10] = [
    Some("Mí\u{ad}nima"),
    Some("Muy Baja"),
    Some("Más Baja"),
    Some("Baja"),
    Some("Media"),
    Some("Alta"),
    Some("Más Alta"),
    Some("Muy Alta"),
    Some("Máxima"),
    Some("No Redistribuir"),
];

/// Spanish resource key `CONSTRAINT_TYPES` name table.
///
/// Corresponds to Java: `LocaleData_es.CONSTRAINT_TYPES_DATA`.
pub const CONSTRAINT_TYPES_DATA: [Option<&'static str>; 8] = [
    Some("Lo antes posible"),
    Some("Lo más tarde posible"),
    Some("Debe comenzar el"),
    Some("Debe finalizar el"),
    Some("No comenzar antes del"),
    Some("No comenzar después del"),
    Some("No finalizar antes del"),
    Some("No finalizar después del"),
];

/// Spanish resource key `TASK_NAMES` name table (indices correspond to MPX task field numbers; end-of-line comments give the English default name).
///
/// Corresponds to Java: `LocaleData_es.TASK_NAMES_DATA`.
pub const TASK_NAMES_DATA: [Option<&'static str>; 153] = [
    None,                              // -
    Some("Nombre"),                    // Name
    Some("WBS"),                       // WBS
    Some("Nivel de Esquema"),          // Outline Level
    Some("Texto1"),                    // Text1
    Some("Texto2"),                    // Text2
    Some("Texto3"),                    // Text3
    Some("Texto4"),                    // Text4
    Some("Texto5"),                    // Text5
    Some("Texto6"),                    // Text6
    Some("Texto7"),                    // Text7
    Some("Texto8"),                    // Text8
    Some("Texto9"),                    // Text9
    Some("Texto10"),                   // Text10
    Some("Notas"),                     // Notes
    Some("Contacto"),                  // Contact
    Some("Grupo de Recursos"),         // Resource Group
    None,                              // -
    None,                              // -
    None,                              // -
    Some("Trabajo"),                   // Work
    Some("Línea de Base de Trabajo"),  // Baseline Work
    Some("Trabajo Real"),              // Actual Work
    Some("Trabajo Restante"),          // Remaining Work
    Some("Variación de Trabajo"),      // Work Variance
    Some("% Trabajo Completado"),      // % Work Complete
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    Some("Costo"),                     // Cost
    Some("Lí\u{ad}nea de Base de Costo"), // Baseline Cost
    Some("Costo Real"),                // Actual Cost
    Some("Costo Restante"),            // Remaining Cost
    Some("Variación de Costo"),        // Cost Variance
    Some("Costo Fijo"),                // Fixed Cost
    Some("Costo1"),                    // Cost1
    Some("Costo2"),                    // Cost2
    Some("Costo3"),                    // Cost3
    None,                              // -
    Some("Duración"),                  // Duration
    Some("Línea Base de Duración"),    // Baseline Duration
    Some("Duración Real"),             // Actual Duration
    Some("Duración Restante"),         // Remaining Duration
    Some("% Completado"),              // % Complete
    Some("Variación de Duración"),     // Duration Variance
    Some("Duración1"),                 // Duration1
    Some("Duración2"),                 // Duration2
    Some("Duración3"),                 // Duration3
    None,                              // -
    Some("Comienzo"),                  // Start
    Some("Fin"),                       // Finish
    Some("Comienzo Temprano"),         // Early Start
    Some("Fin Temprano"),              // Early Finish
    Some("Comienzo Tardí\u{ad}o"),     // Late Start
    Some("Fin TardÃ\u{ad}o"),          // Late Finish (garbled text in the Java source, preserved as-is)
    Some("Lí\u{ad}nea de Base de Comienzo"), // Baseline Start
    Some("Lí\u{ad}nea de Base de Fin"), // Baseline Finish
    Some("Comienzo Real"),             // Actual Start
    Some("Fin Real"),                  // Actual Finish
    Some("Comienzo1"),                 // Start1
    Some("Fin1"),                      // Finish1
    Some("Comienzo2"),                 // Start2
    Some("Fin2"),                      // Finish2
    Some("Comienzo3"),                 // Start3
    Some("Fin3"),                      // Finish3
    Some("Variación de Comienzo"),     // Start Variance
    Some("Variación de Fin"),          // Finish Variance
    Some("Fecha de Restricción"),      // Constraint Date
    None,                              // -
    Some("Predecesoras"),              // Predecessors
    Some("Sucesoras"),                 // Successors
    Some("Nombres de Recursos"),       // Resource Names
    Some("Iniciales de Recursos"),     // Resource Initials
    Some("Predecesoras de ID Único"),  // Unique ID Predecessors
    Some("Sucesoras de ID Único"),     // Unique ID Successors
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    Some("Fijo"),                      // Fixed
    Some("Hito"),                      // Milestone
    Some("Crí\u{ad}tico"),             // Critical
    Some("Marked"),                    // Marked (not translated in Java source)
    Some("Rollup"),                    // Rollup (not translated in Java source)
    Some("BCWS"),                      // BCWS (not translated in Java source)
    Some("BCWP"),                      // BCWP (not translated in Java source)
    Some("SV"),                        // SV (not translated in Java source)
    Some("CV"),                        // CV (not translated in Java source)
    None,                              // -
    Some("ID"),                        // ID (not translated in Java source)
    Some("Tipo de Restricción"),       // Constraint Type
    Some("Demora"),                    // Delay
    Some("Free Slack"),                // Free Slack (not translated in Java source)
    Some("Total Slack"),               // Total Slack (not translated in Java source)
    Some("Prioridad"),                 // Priority
    Some("Subproject File"),           // Subproject File (not translated in Java source)
    Some("Proyecto"),                  // Project
    Some("ID Único"),                  // Unique ID
    Some("Número de Esquema"),         // Outline Number
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    Some("Indicador1"),                // Flag1
    Some("Indicador2"),                // Flag2
    Some("Indicador3"),                // Flag3
    Some("Indicador4"),                // Flag4
    Some("Indicador5"),                // Flag5
    Some("Indicador6"),                // Flag6
    Some("Indicador7"),                // Flag7
    Some("Indicador8"),                // Flag8
    Some("Indicador9"),                // Flag9
    Some("Indicador10"),               // Flag10
    Some("Summary"),                   // Summary (not translated in Java source)
    Some("Objetos"),                   // Objects
    Some("Campos Enlazados"),          // Linked Fields
    Some("Ocultar Barra"),             // Hide Bar
    None,                              // -
    Some("Creada"),                    // Created
    Some("Comienzo4"),                 // Start4
    Some("Fin4"),                      // Finish4
    Some("Comienzo5"),                 // Start5
    Some("Fin5"),                      // Finish5
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    Some("Confirmed"),                 // Confirmed (not translated in Java source)
    Some("Update Needed"),             // Update Needed (not translated in Java source)
    None,                              // -
    None,                              // -
    None,                              // -
    Some("Número1"),                   // Number1
    Some("Número2"),                   // Number2
    Some("Número3"),                   // Number3
    Some("Número4"),                   // Number4
    Some("Número5"),                   // Number5
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    Some("Stop"),                      // Stop (not translated in Java source)
    Some("Resume No Earlier Than"),    // Resume No Earlier Than (not translated in Java source)
    Some("Resume"),                    // Resume (not translated in Java source)
];

/// Spanish resource key `RESOURCE_NAMES` name table (indices correspond to MPX resource field numbers; end-of-line comments give the English default name).
///
/// Corresponds to Java: `LocaleData_es.RESOURCE_NAMES_DATA`.
pub const RESOURCE_NAMES_DATA: [Option<&'static str>; 52] = [
    None,                            // -
    Some("Nombre"),                  // Name
    Some("Iniciales"),               // Initials
    Some("Grupo"),                   // Group
    Some("Código"),                  // Code
    Some("Texto1"),                  // Text1
    Some("Texto2"),                  // Text2
    Some("Texto3"),                  // Text3
    Some("Texto4"),                  // Text4
    Some("Texto5"),                  // Text5
    Some("Notas"),                   // Notes
    Some("Correo electrónico"),      // Email Address
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    Some("Trabajo"),                 // Work
    Some("Línea de Base de Trabajo"), // Baseline Work
    Some("Trabajo Real"),            // Actual Work
    Some("Trabajo Restante"),        // Remaining Work
    Some("Overtime Work"),           // Overtime Work (not translated in Java source)
    Some("Variación de Trabajo"),    // Work Variance
    Some("% Trabajo Completado"),    // % Work Complete
    None,                            // -
    None,                            // -
    None,                            // -
    Some("Costo"),                   // Cost
    Some("Lí\u{ad}nea de Base de Costo"), // Baseline Cost
    Some("Costo Real"),              // Actual Cost
    Some("Costo Restante"),          // Remaining Cost
    Some("Variación de Costo"),      // Cost Variance
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    Some("ID"),                      // ID (not translated in Java source)
    Some("Max Units"),               // Max Units (not translated in Java source)
    Some("Standard Rate"),           // Standard Rate (not translated in Java source)
    Some("Overtime Rate"),           // Overtime Rate (not translated in Java source)
    Some("Costo Por Uso"),           // Cost Per Use
    Some("Acumulación de costos"),   // Accrue At
    Some("Sobreasignado"),           // Overallocated
    Some("Pico"),                    // Peak
    Some("Calendario Base"),         // Base Calendar
    Some("ID Único"),                // Unique ID
    Some("Objetos"),                 // Objects
    Some("Campos Enlazados"),        // Linked Fields
];

/// Corresponds to resource key `FILE_DELIMITER`
pub const FILE_DELIMITER: char = ',';
/// Corresponds to resource key `PROGRAM_NAME`
pub const PROGRAM_NAME: &'static str = "Microsoft Project for Windows";
/// Corresponds to resource key `FILE_VERSION`
pub const FILE_VERSION: &'static str = "4.0";
/// Corresponds to resource key `CODE_PAGE`
pub const CODE_PAGE: CodePage = CodePage::Ansi;
/// Corresponds to resource key `CURRENCY_SYMBOL`
pub const CURRENCY_SYMBOL: &'static str = "$";
/// Corresponds to resource key `CURRENCY_SYMBOL_POSITION`
pub const CURRENCY_SYMBOL_POSITION: CurrencySymbolPosition = CurrencySymbolPosition::Before;
/// Corresponds to resource key `CURRENCY_DIGITS`
pub const CURRENCY_DIGITS: i32 = 2;
/// Corresponds to resource key `CURRENCY_THOUSANDS_SEPARATOR`
pub const CURRENCY_THOUSANDS_SEPARATOR: char = '.';
/// Corresponds to resource key `CURRENCY_DECIMAL_SEPARATOR`
pub const CURRENCY_DECIMAL_SEPARATOR: char = ',';
/// Corresponds to resource key `DATE_ORDER`
pub const DATE_ORDER: DateOrder = DateOrder::Dmy;
/// Corresponds to resource key `TIME_FORMAT`
pub const TIME_FORMAT: ProjectTimeFormat = ProjectTimeFormat::TwentyFourHour;
/// Corresponds to resource key `DEFAULT_START_TIME` (minutes since midnight, same value as the English default)
pub const DEFAULT_START_TIME: i32 = 480;
/// Corresponds to resource key `DATE_SEPARATOR`
pub const DATE_SEPARATOR: char = '/';
/// Corresponds to resource key `TIME_SEPARATOR`
pub const TIME_SEPARATOR: char = ':';
/// Corresponds to resource key `AM_TEXT`
pub const AM_TEXT: &'static str = "am";
/// Corresponds to resource key `PM_TEXT`
pub const PM_TEXT: &'static str = "pm";
/// Corresponds to resource key `DATE_FORMAT`
pub const DATE_FORMAT: ProjectDateFormat = ProjectDateFormat::DdMmYyyy;
/// Corresponds to resource key `BAR_TEXT_DATE_FORMAT`
pub const BAR_TEXT_DATE_FORMAT: i32 = 0;
/// Corresponds to resource key `NA`
pub const NA: &'static str = "NA";
/// Corresponds to resource key `YES` (Java source is `"S\u00ED\u00AD"`, contains a soft hyphen)
pub const YES: &'static str = "Sí\u{ad}";
/// Corresponds to resource key `NO`
pub const NO: &'static str = "No";

/// Contents of the Spanish resource bundle, corresponding to Java `LocaleData_es.RESOURCE_DATA`
/// (i.e. the return value of `getContents()`).
pub static OVERRIDES: LocaleOverrides = LocaleOverrides {
    file_delimiter: Some(FILE_DELIMITER),
    program_name: Some(PROGRAM_NAME),
    file_version: Some(FILE_VERSION),
    code_page: Some(CODE_PAGE),
    currency_symbol: Some(CURRENCY_SYMBOL),
    currency_symbol_position: Some(CURRENCY_SYMBOL_POSITION),
    currency_digits: Some(CURRENCY_DIGITS),
    currency_thousands_separator: Some(CURRENCY_THOUSANDS_SEPARATOR),
    currency_decimal_separator: Some(CURRENCY_DECIMAL_SEPARATOR),
    date_order: Some(DATE_ORDER),
    time_format: Some(TIME_FORMAT),
    default_start_time: Some(DEFAULT_START_TIME),
    date_separator: Some(DATE_SEPARATOR),
    time_separator: Some(TIME_SEPARATOR),
    am_text: Some(AM_TEXT),
    pm_text: Some(PM_TEXT),
    date_format: Some(DATE_FORMAT),
    bar_text_date_format: Some(BAR_TEXT_DATE_FORMAT),
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
        assert_eq!(TASK_NAMES_DATA[1], Some("Nombre"));
        assert_eq!(TASK_NAMES_DATA[25], Some("% Trabajo Completado"));
        assert_eq!(TASK_NAMES_DATA[31], Some("Lí\u{ad}nea de Base de Costo"));
        assert_eq!(TASK_NAMES_DATA[55], Some("Fin TardÃ\u{ad}o"));
        assert_eq!(TASK_NAMES_DATA[82], Some("Crí\u{ad}tico"));
        assert_eq!(TASK_NAMES_DATA[152], Some("Resume"));

        assert_eq!(RESOURCE_NAMES_DATA[11], Some("Correo electrónico"));
        assert_eq!(RESOURCE_NAMES_DATA[24], Some("Overtime Work"));
        assert_eq!(RESOURCE_NAMES_DATA[41], Some("Max Units"));
        assert_eq!(RESOURCE_NAMES_DATA[51], Some("Campos Enlazados"));

        assert_eq!(TIME_UNITS_ARRAY_DATA[3], ["s"].as_slice());
        assert_eq!(TIME_UNITS_ARRAY_DATA[4], ["ms"].as_slice());
        assert_eq!(PRIORITY_TYPES_DATA[0], Some("Mí\u{ad}nima"));
        assert_eq!(RELATION_TYPES_DATA[1], Some("FC"));
        assert_eq!(CONSTRAINT_TYPES_DATA[6], Some("No finalizar antes del"));
    }

    #[test]
    fn locale_data_returns_spanish_tables() {
        assert_eq!(
            LocaleData::get_string_array(MpxLocale::Es, LocaleData::TASK_NAMES),
            Some(TASK_NAMES_DATA.as_slice())
        );
        assert_eq!(LocaleData::get_string(MpxLocale::Es, LocaleData::YES), Some("Sí\u{ad}"));
        assert_eq!(LocaleData::get_string(MpxLocale::Es, LocaleData::NO), Some("No"));
        assert_eq!(LocaleData::get_code_page(MpxLocale::Es), CodePage::Ansi);
        assert_eq!(LocaleData::get_date_order(MpxLocale::Es), DateOrder::Dmy);
        assert_eq!(LocaleData::get_char(MpxLocale::Es, LocaleData::CURRENCY_DECIMAL_SEPARATOR), Some(','));
        // The covered keys include DEFAULT_START_TIME (same value as the English default)
        assert_eq!(
            LocaleData::get_integer(MpxLocale::Es, LocaleData::DEFAULT_START_TIME),
            Some(480)
        );
        assert_eq!(DEFAULT_START_TIME, 480);
    }
}
