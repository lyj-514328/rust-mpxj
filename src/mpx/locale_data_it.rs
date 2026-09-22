//! Corresponds to Java: `org/mpxj/mpx/LocaleData_it.java`
//!
//! # Porting trade-offs (`ResourceBundle` / `ListResourceBundle`)
//!
//! Java's `LocaleData_it` is the Italian resource bundle and overrides only some keys; keys it does not
//! override are resolved by `ResourceBundle` by falling back from the base bundle. The Rust side writes the
//! keys this language actually overrides as module-level `pub const`s (scalar constants are named after the
//! keys in Java's `RESOURCE_DATA`, and array constants keep Java's field names), and aggregates them in
//! [`OVERRIDES`] (corresponding to the return value of `getContents()`); keys that are not overridden are
//! `None` and fall back to the English default resources via the `locale_data` module. See the module header
//! of `locale_data` for the value-lookup logic; the protocol methods of `ListResourceBundle` are replaced by
//! constant tables and are not ported.
//!
//! Keys not overridden by this language: `DEFAULT_START_TIME`.
//!
//! Skipped Java methods/members: none.

use crate::code_page::CodePage;
use crate::currency_symbol_position::CurrencySymbolPosition;
use crate::date_order::DateOrder;
use crate::mpx::locale_data::LocaleOverrides;
use crate::project_date_format::ProjectDateFormat;
use crate::project_time_format::ProjectTimeFormat;

/// Italian time unit alias table.
///
/// Corresponds to Java: `LocaleData_it.TIME_UNITS_ARRAY_DATA` (Java literal order: minutes, hours, days,
/// weeks, months, years, percent, followed by the "elapsed" units; index 5 is years and index 6 is percent,
/// which differs from the `TimeUnit` value order; copied verbatim from Java here).
pub const TIME_UNITS_ARRAY_DATA: [&'static [&'static str]; 14] = [
    &["r"],   // 0 TimeUnit.MINUTES
    &["o"],   // 1 TimeUnit.HOURS
    &["g"],   // 2 TimeUnit.DAYS（giorni）
    &["s"],   // 3 TimeUnit.WEEKS（settimane）
    &["m"],   // 4 TimeUnit.MONTHS（mesi）
    &["a"],   // 5 TimeUnit.PERCENT (Java literal position, anni)
    &["%"],   // 6 TimeUnit.YEARS (Java literal position)
    &["tr"],  // 7 TimeUnit.ELAPSED_MINUTES
    &["to"],  // 8 TimeUnit.ELAPSED_HOURS
    &["tg"],  // 9 TimeUnit.ELAPSED_DAYS
    &["ts"],  // 10 TimeUnit.ELAPSED_WEEKS
    &["tm"],  // 11 TimeUnit.ELAPSED_MONTHS
    &["ta"],  // 12 TimeUnit.ELAPSED_YEARS
    &["t%"],  // 13 TimeUnit.ELAPSED_PERCENT
];

/// Name table for the Italian resource key `ACCRUE_TYPES`.
///
/// Corresponds to Java: `LocaleData_it.ACCRUE_TYPES_DATA`.
pub const ACCRUE_TYPES_DATA: [Option<&'static str>; 3] =
    [Some("Inizio"), Some("Fine"), Some("Proporzione")];

/// Name table for the Italian resource key `RELATION_TYPES`.
///
/// Corresponds to Java: `LocaleData_it.RELATION_TYPES_DATA` (the same as the English defaults, but explicitly overridden in the Java variant).
pub const RELATION_TYPES_DATA: [Option<&'static str>; 4] =
    [Some("FF"), Some("FS"), Some("SF"), Some("SS")];

/// Name table for the Italian resource key `PRIORITY_TYPES`.
///
/// Corresponds to Java: `LocaleData_it.PRIORITY_TYPES_DATA`.
pub const PRIORITY_TYPES_DATA: [Option<&'static str>; 10] = [
    Some("Il minimo"),
    Some("Molto Basso"),
    Some("Piu' Basso"),
    Some("Basso"),
    Some("Medio"),
    Some("Alto"),
    Some("Piu' Alto"),
    Some("Molto Alto"),
    Some("Il maggiore"),
    Some("Non livellare"),
];

/// Name table for the Italian resource key `CONSTRAINT_TYPES`.
///
/// Corresponds to Java: `LocaleData_it.CONSTRAINT_TYPES_DATA`.
pub const CONSTRAINT_TYPES_DATA: [Option<&'static str>; 8] = [
    Some("Appena Possibile"),
    Some("Piu' Tardi Possibile"),
    Some("Deve Cominciare il"),
    Some("Deve Finire il"),
    Some("Non iniziare prima di"),
    Some("Non iniziare piu' tardi di"),
    Some("Finire non prima"),
    Some("Finire entro"),
];

/// Name table for the Italian resource key `TASK_NAMES` (indices correspond to MPX task field numbers; end-of-line comments are the English default names).
///
/// Corresponds to Java: `LocaleData_it.TASK_NAMES_DATA`.
pub const TASK_NAMES_DATA: [Option<&'static str>; 153] = [
    None,                          // -
    Some("Nome"),                  // Name
    Some("WBS"),                   // WBS
    Some("Livello Esterno"),       // Outline Level
    Some("Testo1"),                // Text1
    Some("Testo2"),                // Text2
    Some("Testo3"),                // Text3
    Some("Testo4"),                // Text4
    Some("Testo5"),                // Text5
    Some("Testo6"),                // Text6
    Some("Testo7"),                // Text7
    Some("Testo8"),                // Text8
    Some("Testo9"),                // Text9
    Some("Testo10"),               // Text10
    Some("Note"),                  // Notes
    Some("Contatti"),              // Contact
    Some("Gruppo delle Risorse"),  // Resource Group
    None,                          // -
    None,                          // -
    None,                          // -
    Some("Lavoro"),                // Work
    Some("Lavoro Normale"),        // Baseline Work
    Some("Lavoro Reale"),          // Actual Work
    Some("Lavoro Restante"),       // Remaining Work
    Some("Modifiche di Lavoro"),   // Work Variance
    Some("% lavoro completo"),     // % Work Complete
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    Some("Costo"),                 // Cost
    Some("Costo Pianificato"),     // Baseline Cost
    Some("Costo Reale"),           // Actual Cost
    Some("Costo Restante"),        // Remaining Cost
    Some("Variazione di Costo"),   // Cost Variance
    Some("Costo Fisso"),           // Fixed Cost
    Some("Costo1"),                // Cost1
    Some("Costo2"),                // Cost2
    Some("Costo3"),                // Cost3
    None,                          // -
    Some("Durata"),                // Duration
    Some("Durata Pianificata"),    // Baseline Duration
    Some("Durata Reale"),          // Actual Duration
    Some("Durata Restante"),       // Remaining Duration
    Some("% Completa"),            // % Complete
    Some("Variazione di Durata"),  // Duration Variance
    Some("Durata1"),               // Duration1
    Some("Durata2"),               // Duration2
    Some("Durata3"),               // Duration3
    None,                          // -
    Some("Inizio"),                // Start
    Some("Fine"),                  // Finish
    Some("Minimo Inizio"),         // Early Start
    Some("Minima Fine"),           // Early Finish
    Some("Inizio Massimo"),        // Late Start
    Some("Fine Massimo"),          // Late Finish
    Some("Inizio Pianificato"),    // Baseline Start
    Some("Fine Pianificata"),      // Baseline Finish
    Some("Inizio Reale"),          // Actual Start
    Some("Fine Reale"),            // Actual Finish
    Some("Inizio1"),               // Start1
    Some("Fine1"),                 // Finish1
    Some("Inizio2"),               // Start2
    Some("Fine2"),                 // Finish2
    Some("Inizio3"),               // Start3
    Some("Fine3"),                 // Finish3
    Some("Margine di Inizio"),     // Start Variance
    Some("Margine di Fine"),       // Finish Variance
    Some("Data Vincolante"),       // Constraint Date
    None,                          // -
    Some("Predecessori"),          // Predecessors
    Some("Successori"),            // Successors
    Some("Nomi Risorse"),          // Resource Names
    Some("Iniziali Risourse"),     // Resource Initials
    Some("ID Unico dei Predecessori"), // Unique ID Predecessors
    Some("ID Unique dei Successori"), // Unique ID Successors
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    Some("Fissato"),               // Fixed
    Some("Jalon"),                 // Milestone
    Some("Critico"),               // Critical
    Some("Marcato"),               // Marked
    Some("Rollup"),                // Rollup
    Some("BCWS"),                  // BCWS
    Some("BCWP"),                  // BCWP
    Some("SV"),                    // SV
    Some("CV"),                    // CV
    None,                          // -
    Some("ID"),                    // ID
    Some("Tipo di Vincolo"),       // Constraint Type
    Some("Ritardo"),               // Delay
    Some("Margine Libero"),        // Free Slack
    Some("Margine Totale"),        // Total Slack
    Some("Priorita'"),             // Priority
    Some("File Subprogetto"),      // Subproject File
    Some("Progetto"),              // Project
    Some("ID Unico"),              // Unique ID
    Some("Numero Esterno"),        // Outline Number
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    Some("Indicatore1"),           // Flag1
    Some("Indicatore2"),           // Flag2
    Some("Indicatore3"),           // Flag3
    Some("Indicatore4"),           // Flag4
    Some("Indicatore5"),           // Flag5
    Some("Indicatore6"),           // Flag6
    Some("Indicatore7"),           // Flag7
    Some("Indicatore8"),           // Flag8
    Some("Indicatore9"),           // Flag9
    Some("Indicatore10"),          // Flag10
    Some("Indice"),                // Summary
    Some("Oggetto"),               // Objects
    Some("Campi Collegati"),       // Linked Fields
    Some("Nascondi Barra"),        // Hide Bar
    None,                          // -
    Some("Creato"),                // Created
    Some("Inizio4"),               // Start4
    Some("Fine4"),                 // Finish4
    Some("Inizio5"),               // Start5
    Some("Fine5"),                 // Finish5
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    Some("Confermato"),            // Confirmed
    Some("Necessario Aggiornamento"), // Update Needed
    None,                          // -
    None,                          // -
    None,                          // -
    Some("Numero1"),               // Number1
    Some("Numero2"),               // Number2
    Some("Numero3"),               // Number3
    Some("Numero4"),               // Number4
    Some("Numero5"),               // Number5
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    Some("Stop"),                  // Stop
    Some("Continuare non prima del"), // Resume No Earlier Than
    Some("Continuare"),            // Resume
];

/// Name table for the Italian resource key `RESOURCE_NAMES` (indices correspond to MPX resource field numbers; end-of-line comments are the English default names).
///
/// Corresponds to Java: `LocaleData_it.RESOURCE_NAMES_DATA`.
pub const RESOURCE_NAMES_DATA: [Option<&'static str>; 52] = [
    None,                          // -
    Some("Nome"),                  // Name
    Some("Iniziali"),              // Initials
    Some("Gruppo"),                // Group
    Some("Codice"),                // Code
    Some("Testo1"),                // Text1
    Some("Testo2"),                // Text2
    Some("Testo3"),                // Text3
    Some("Testo4"),                // Text4
    Some("Testo5"),                // Text5
    Some("Note"),                  // Notes
    Some("Indirizzo e-mail"),      // Email Address
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    Some("Lavoro"),                // Work
    Some("Lavoro Pianificato"),    // Baseline Work
    Some("Lavoro Reale"),          // Actual Work
    Some("Lavoro Restante"),       // Remaining Work
    Some("Ore sup."),              // Overtime Work
    Some("Variazione di Lavoro"),  // Work Variance
    Some("% Lavoro completato"),   // % Work Complete
    None,                          // -
    None,                          // -
    None,                          // -
    Some("Costo"),                 // Cost
    Some("Costo Pianificato"),     // Baseline Cost
    Some("Costo Reale"),           // Actual Cost
    Some("Costo Restante"),        // Remaining Cost
    Some("Variazione di Costo"),   // Cost Variance
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    Some("ID"),                    // ID
    Some("Unita' Massime"),        // Max Units
    Some("Tasse Standard"),        // Standard Rate
    Some("Tasse ore sup."),        // Overtime Rate
    Some("Costo per Utilizzo"),    // Cost Per Use
    Some("Aumentare a"),           // Accrue At
    Some("Sovra Assegnato"),       // Overallocated
    Some("Pointe"),                // Peak
    Some("Calendario di Base"),    // Base Calendar
    Some("ID Unico"),              // Unique ID
    Some("Oggetti"),               // Objects
    Some("Campi Collegati"),       // Linked Fields
];

/// Corresponds to the resource key `FILE_DELIMITER`
pub const FILE_DELIMITER: char = ';';
/// Corresponds to the resource key `PROGRAM_NAME`
pub const PROGRAM_NAME: &'static str = "Microsoft Project for Windows";
/// Corresponds to the resource key `FILE_VERSION`
pub const FILE_VERSION: &'static str = "4.0";
/// Corresponds to the resource key `CODE_PAGE`
pub const CODE_PAGE: CodePage = CodePage::Ansi;
/// Corresponds to the resource key `CURRENCY_SYMBOL` (an empty string in the Java source)
pub const CURRENCY_SYMBOL: &'static str = "";
/// Corresponds to the resource key `CURRENCY_SYMBOL_POSITION`
pub const CURRENCY_SYMBOL_POSITION: CurrencySymbolPosition = CurrencySymbolPosition::Before;
/// Corresponds to the resource key `CURRENCY_DIGITS`
pub const CURRENCY_DIGITS: i32 = 2;
/// Corresponds to the resource key `CURRENCY_THOUSANDS_SEPARATOR`
pub const CURRENCY_THOUSANDS_SEPARATOR: char = '.';
/// Corresponds to the resource key `CURRENCY_DECIMAL_SEPARATOR`
pub const CURRENCY_DECIMAL_SEPARATOR: char = ',';
/// Corresponds to the resource key `DATE_ORDER`
pub const DATE_ORDER: DateOrder = DateOrder::Dmy;
/// Corresponds to the resource key `TIME_FORMAT`
pub const TIME_FORMAT: ProjectTimeFormat = ProjectTimeFormat::TwentyFourHour;
/// Corresponds to the resource key `DATE_SEPARATOR`
pub const DATE_SEPARATOR: char = '/';
/// Corresponds to the resource key `TIME_SEPARATOR`
pub const TIME_SEPARATOR: char = ':';
/// Corresponds to the resource key `AM_TEXT`
pub const AM_TEXT: &'static str = "";
/// Corresponds to the resource key `PM_TEXT`
pub const PM_TEXT: &'static str = "";
/// Corresponds to the resource key `DATE_FORMAT`
pub const DATE_FORMAT: ProjectDateFormat = ProjectDateFormat::DdMmYyyy;
/// Corresponds to the resource key `BAR_TEXT_DATE_FORMAT`
pub const BAR_TEXT_DATE_FORMAT: i32 = 0;
/// Corresponds to the resource key `NA`
pub const NA: &'static str = "NA";
/// Corresponds to the resource key `YES`
pub const YES: &'static str = "Si";
/// Corresponds to the resource key `NO`
pub const NO: &'static str = "No";

/// Contents of the Italian resource bundle, corresponding to Java `LocaleData_it.RESOURCE_DATA`
/// (that is, the return value of `getContents()`).
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
    default_start_time: None,
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
        assert_eq!(TASK_NAMES_DATA[1], Some("Nome"));
        assert_eq!(TASK_NAMES_DATA[3], Some("Livello Esterno"));
        assert_eq!(TASK_NAMES_DATA[25], Some("% lavoro completo"));
        assert_eq!(TASK_NAMES_DATA[52], Some("Minimo Inizio"));
        assert_eq!(TASK_NAMES_DATA[95], Some("Priorita'"));
        assert_eq!(TASK_NAMES_DATA[152], Some("Continuare"));

        assert_eq!(RESOURCE_NAMES_DATA[11], Some("Indirizzo e-mail"));
        assert_eq!(RESOURCE_NAMES_DATA[24], Some("Ore sup."));
        assert_eq!(RESOURCE_NAMES_DATA[41], Some("Unita' Massime"));
        assert_eq!(RESOURCE_NAMES_DATA[51], Some("Campi Collegati"));

        assert_eq!(TIME_UNITS_ARRAY_DATA[0], ["r"].as_slice());
        assert_eq!(TIME_UNITS_ARRAY_DATA[13], ["t%"].as_slice());
        assert_eq!(ACCRUE_TYPES_DATA[1], Some("Fine"));
        assert_eq!(RELATION_TYPES_DATA[2], Some("SF"));
        assert_eq!(PRIORITY_TYPES_DATA[6], Some("Piu' Alto"));
        assert_eq!(CONSTRAINT_TYPES_DATA[7], Some("Finire entro"));
    }

    #[test]
    fn locale_data_returns_italian_tables() {
        assert_eq!(
            LocaleData::get_string_array(MpxLocale::It, LocaleData::TASK_NAMES),
            Some(TASK_NAMES_DATA.as_slice())
        );
        assert_eq!(LocaleData::get_string(MpxLocale::It, LocaleData::YES), Some("Si"));
        assert_eq!(LocaleData::get_string(MpxLocale::It, LocaleData::NA), Some("NA"));
        assert_eq!(LocaleData::get_code_page(MpxLocale::It), CodePage::Ansi);
        assert_eq!(LocaleData::get_date_format(MpxLocale::It), ProjectDateFormat::DdMmYyyy);
        // Keys not overridden fall back to English
        assert_eq!(
            LocaleData::get_integer(MpxLocale::It, LocaleData::DEFAULT_START_TIME),
            Some(480)
        );
    }
}
