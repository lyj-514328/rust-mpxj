//! 对应 Java: `org/mpxj/mpx/LocaleData_it.java`
//!
//! # 移植取舍（`ResourceBundle` / `ListResourceBundle`）
//!
//! Java 的 `LocaleData_it` 是意大利语资源包，只覆盖部分键；未覆盖的键由 `ResourceBundle`
//! 从基类回落取值。Rust 侧把该语言实际覆盖的键写成模块级 `pub const`
//! （标量常量按 Java `RESOURCE_DATA` 的键名命名，数组常量沿用 Java 的字段名），
//! 并用 [`OVERRIDES`]（对应 `getContents()` 的返回值）汇总；未覆盖的键为 `None`，
//! 由 `locale_data` 模块回落到英文默认资源。取值逻辑见 `locale_data` 模块头部说明；
//! `ListResourceBundle` 的协议方法由常量表替代，不移植。
//!
//! 本语言未覆盖的键：`DEFAULT_START_TIME`。
//!
//! 未移植的成员：无。

use crate::code_page::CodePage;
use crate::currency_symbol_position::CurrencySymbolPosition;
use crate::date_order::DateOrder;
use crate::mpx::locale_data::LocaleOverrides;
use crate::project_date_format::ProjectDateFormat;
use crate::project_time_format::ProjectTimeFormat;

/// 意大利语时间单位别名表。
///
/// 对应 Java: `LocaleData_it.TIME_UNITS_ARRAY_DATA`（Java 字面量顺序：分钟、小时、天、
/// 周、月、年、百分比、随后是各「已用」单位；下标 5 是年、下标 6 是百分比，
/// 与 `TimeUnit` 的取值顺序不同，此处照抄 Java）。
pub const TIME_UNITS_ARRAY_DATA: [&'static [&'static str]; 14] = [
    &["r"],   // 0 TimeUnit.MINUTES
    &["o"],   // 1 TimeUnit.HOURS
    &["g"],   // 2 TimeUnit.DAYS（giorni）
    &["s"],   // 3 TimeUnit.WEEKS（settimane）
    &["m"],   // 4 TimeUnit.MONTHS（mesi）
    &["a"],   // 5 TimeUnit.PERCENT（Java 字面量位置，anni）
    &["%"],   // 6 TimeUnit.YEARS（Java 字面量位置）
    &["tr"],  // 7 TimeUnit.ELAPSED_MINUTES
    &["to"],  // 8 TimeUnit.ELAPSED_HOURS
    &["tg"],  // 9 TimeUnit.ELAPSED_DAYS
    &["ts"],  // 10 TimeUnit.ELAPSED_WEEKS
    &["tm"],  // 11 TimeUnit.ELAPSED_MONTHS
    &["ta"],  // 12 TimeUnit.ELAPSED_YEARS
    &["t%"],  // 13 TimeUnit.ELAPSED_PERCENT
];

/// 意大利语资源键 `ACCRUE_TYPES` 的名表。
///
/// 对应 Java: `LocaleData_it.ACCRUE_TYPES_DATA`。
pub const ACCRUE_TYPES_DATA: [Option<&'static str>; 3] =
    [Some("Inizio"), Some("Fine"), Some("Proporzione")];

/// 意大利语资源键 `RELATION_TYPES` 的名表。
///
/// 对应 Java: `LocaleData_it.RELATION_TYPES_DATA`（与英文默认值相同，但 Java 变体中显式覆盖）。
pub const RELATION_TYPES_DATA: [Option<&'static str>; 4] =
    [Some("FF"), Some("FS"), Some("SF"), Some("SS")];

/// 意大利语资源键 `PRIORITY_TYPES` 的名表。
///
/// 对应 Java: `LocaleData_it.PRIORITY_TYPES_DATA`。
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

/// 意大利语资源键 `CONSTRAINT_TYPES` 的名表。
///
/// 对应 Java: `LocaleData_it.CONSTRAINT_TYPES_DATA`。
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

/// 意大利语资源键 `TASK_NAMES` 的名表（下标与 MPX 任务字段号对应；行末注释为英文默认名）。
///
/// 对应 Java: `LocaleData_it.TASK_NAMES_DATA`。
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

/// 意大利语资源键 `RESOURCE_NAMES` 的名表（下标与 MPX 资源字段号对应；行末注释为英文默认名）。
///
/// 对应 Java: `LocaleData_it.RESOURCE_NAMES_DATA`。
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

/// 对应资源键 `FILE_DELIMITER`
pub const FILE_DELIMITER: char = ';';
/// 对应资源键 `PROGRAM_NAME`
pub const PROGRAM_NAME: &'static str = "Microsoft Project for Windows";
/// 对应资源键 `FILE_VERSION`
pub const FILE_VERSION: &'static str = "4.0";
/// 对应资源键 `CODE_PAGE`
pub const CODE_PAGE: CodePage = CodePage::Ansi;
/// 对应资源键 `CURRENCY_SYMBOL`（Java 源码为空字符串）
pub const CURRENCY_SYMBOL: &'static str = "";
/// 对应资源键 `CURRENCY_SYMBOL_POSITION`
pub const CURRENCY_SYMBOL_POSITION: CurrencySymbolPosition = CurrencySymbolPosition::Before;
/// 对应资源键 `CURRENCY_DIGITS`
pub const CURRENCY_DIGITS: i32 = 2;
/// 对应资源键 `CURRENCY_THOUSANDS_SEPARATOR`
pub const CURRENCY_THOUSANDS_SEPARATOR: char = '.';
/// 对应资源键 `CURRENCY_DECIMAL_SEPARATOR`
pub const CURRENCY_DECIMAL_SEPARATOR: char = ',';
/// 对应资源键 `DATE_ORDER`
pub const DATE_ORDER: DateOrder = DateOrder::Dmy;
/// 对应资源键 `TIME_FORMAT`
pub const TIME_FORMAT: ProjectTimeFormat = ProjectTimeFormat::TwentyFourHour;
/// 对应资源键 `DATE_SEPARATOR`
pub const DATE_SEPARATOR: char = '/';
/// 对应资源键 `TIME_SEPARATOR`
pub const TIME_SEPARATOR: char = ':';
/// 对应资源键 `AM_TEXT`
pub const AM_TEXT: &'static str = "";
/// 对应资源键 `PM_TEXT`
pub const PM_TEXT: &'static str = "";
/// 对应资源键 `DATE_FORMAT`
pub const DATE_FORMAT: ProjectDateFormat = ProjectDateFormat::DdMmYyyy;
/// 对应资源键 `BAR_TEXT_DATE_FORMAT`
pub const BAR_TEXT_DATE_FORMAT: i32 = 0;
/// 对应资源键 `NA`
pub const NA: &'static str = "NA";
/// 对应资源键 `YES`
pub const YES: &'static str = "Si";
/// 对应资源键 `NO`
pub const NO: &'static str = "No";

/// 意大利语资源包的内容，对应 Java `LocaleData_it.RESOURCE_DATA`
/// （即 `getContents()` 的返回值）。
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
        // 未覆盖的键回落英文
        assert_eq!(
            LocaleData::get_integer(MpxLocale::It, LocaleData::DEFAULT_START_TIME),
            Some(480)
        );
    }
}
