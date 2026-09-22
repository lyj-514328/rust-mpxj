//! Corresponds to Java: `org/mpxj/mpx/LocaleData_pt.java`
//!
//! # Porting trade-offs (`ResourceBundle` / `ListResourceBundle`)
//!
//! Java's `LocaleData_pt` is a Portuguese resource bundle that overrides only some keys;
//! keys it does not override fall back to the base class via `ResourceBundle`. On the Rust
//! side the keys this language actually overrides are written as module-level `pub const`
//! (scalar constants named after the Java `RESOURCE_DATA` keys, array constants keeping the
//! Java field names) and aggregated in [`OVERRIDES`] (corresponding to the return value of
//! `getContents()`); keys that are not overridden are `None` and fall back to the English
//! default resource in the `locale_data` module. See the `locale_data` module header for the
//! lookup logic; the `ListResourceBundle` protocol methods are replaced by the constant
//! table and are not ported.
//!
//! Keys not overridden by this language: `PROGRAM_NAME`, `FILE_VERSION`, `CODE_PAGE`, `DEFAULT_START_TIME`.
//!
//! Skipped Java methods/members: none.

use crate::currency_symbol_position::CurrencySymbolPosition;
use crate::date_order::DateOrder;
use crate::mpx::locale_data::LocaleOverrides;
use crate::project_date_format::ProjectDateFormat;
use crate::project_time_format::ProjectTimeFormat;

/// Portuguese time unit alias table.
///
/// Corresponds to Java: `LocaleData_pt.TIME_UNITS_ARRAY_DATA` (Java literal order: minutes, hours,
/// days, weeks, months, years, percent, followed by the elapsed units; index 5 is years and
/// index 6 is percent, which differs from the `TimeUnit` value order; Java is copied verbatim here).
pub const TIME_UNITS_ARRAY_DATA: [&'static [&'static str]; 14] = [
    &["m"],     // 0 TimeUnit.MINUTES
    &["h"],     // 1 TimeUnit.HOURS
    &["d"],     // 2 TimeUnit.DAYS
    &["s"],     // 3 TimeUnit.WEEKS (semanas)
    &["mes"],   // 4 TimeUnit.MONTHS (meses)
    &["a"],     // 5 TimeUnit.PERCENT (Java literal position, anos)
    &["%"],     // 6 TimeUnit.YEARS (Java literal position)
    &["em"],    // 7 TimeUnit.ELAPSED_MINUTES
    &["eh"],    // 8 TimeUnit.ELAPSED_HOURS
    &["ed"],    // 9 TimeUnit.ELAPSED_DAYS
    &["es"],    // 10 TimeUnit.ELAPSED_WEEKS
    &["emes"],  // 11 TimeUnit.ELAPSED_MONTHS
    &["ea"],    // 12 TimeUnit.ELAPSED_YEARS
    &["e%"],    // 13 TimeUnit.ELAPSED_PERCENT
];

/// Portuguese name table for resource key `ACCRUE_TYPES`.
///
/// Corresponds to Java: `LocaleData_pt.ACCRUE_TYPES_DATA` (the third entry is an empty string in the Java source).
pub const ACCRUE_TYPES_DATA: [Option<&'static str>; 3] = [Some("Inicio"), Some("Fim"), Some("")];

/// Portuguese name table for resource key `RELATION_TYPES`.
///
/// Corresponds to Java: `LocaleData_pt.RELATION_TYPES_DATA`.
pub const RELATION_TYPES_DATA: [Option<&'static str>; 4] =
    [Some("TT"), Some("TI"), Some("IT"), Some("II")];

/// Portuguese name table for resource key `PRIORITY_TYPES`.
///
/// Corresponds to Java: `LocaleData_pt.PRIORITY_TYPES_DATA` (indices 2 and 6 remain the original English names in the Java source).
pub const PRIORITY_TYPES_DATA: [Option<&'static str>; 10] = [
    Some("Muito Baixa"),
    Some("Muito Baixo"),
    Some("Lower"),
    Some("Baixo"),
    Some("Média"),
    Some("Alta"),
    Some("Higher"),
    Some("Muito Alta"),
    Some("Altíssimo"),
    Some("Não Nivelar"),
];

/// Portuguese name table for resource key `CONSTRAINT_TYPES`.
///
/// Corresponds to Java: `LocaleData_pt.CONSTRAINT_TYPES_DATA`.
pub const CONSTRAINT_TYPES_DATA: [Option<&'static str>; 8] = [
    Some("O Mais Breve Possível"),
    Some("O Mais Tarde Possível"),
    Some("Deve Iniciar Em"),
    Some("Deve Terminar Em"),
    Some("Não Iniciar Antes De"),
    Some("Não Iniciar Depois De"),
    Some("Não Terminar Antes De"),
    Some("Não Terminar Depois De"),
];

/// Portuguese name table for resource key `TASK_NAMES` (indices correspond to MPX task field numbers; the end-of-line comments are the English default names).
///
/// Corresponds to Java: `LocaleData_pt.TASK_NAMES_DATA`.
pub const TASK_NAMES_DATA: [Option<&'static str>; 153] = [
    None,                            // -
    Some("Nome"),                    // Name
    Some("WBS"),                     // WBS
    Some("Nível Externo"),           // Outline Level
    Some("Texto1"),                  // Text1
    Some("Texto2"),                  // Text2
    Some("Texto3"),                  // Text3
    Some("Texto4"),                  // Text4
    Some("Texto5"),                  // Text5
    Some("Texto6"),                  // Text6
    Some("Texto7"),                  // Text7
    Some("Texto8"),                  // Text8
    Some("Texto9"),                  // Text9
    Some("Texto10"),                 // Text10
    Some("Anotações"),               // Notes
    Some("Contacto"),                // Contact
    Some("Grupo de Recursos"),       // Resource Group
    None,                            // -
    None,                            // -
    None,                            // -
    Some("Trabalho"),                // Work
    Some("Linha Base de Trabalho"),  // Baseline Work
    Some("Trabalho Realizado"),      // Actual Work
    Some("Trabalho Restante"),       // Remaining Work
    Some("Variação Trabalho"),       // Work Variance
    Some("% Trabalho Completo"),     // % Work Complete
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    Some("Custo"),                   // Cost
    Some("Linha de Base de Custo"),  // Baseline Cost
    Some("Custo Real"),              // Actual Cost
    Some("Custo Restante"),          // Remaining Cost
    Some("Variação de Custo"),       // Cost Variance
    Some("Custo Fixo"),              // Fixed Cost
    Some("Custo1"),                  // Cost1
    Some("Custo2"),                  // Cost2
    Some("Custo3"),                  // Cost3
    None,                            // -
    Some("Duração"),                 // Duration
    Some("Linha Base de Duração"),   // Baseline Duration
    Some("Duração Real"),            // Actual Duration
    Some("Duração Restante"),        // Remaining Duration
    Some("% Completo"),              // % Complete
    Some("Variação de Duração"),     // Duration Variance
    Some("Duração1"),                // Duration1
    Some("Duração2"),                // Duration2
    Some("Duração3"),                // Duration3
    None,                            // -
    Some("Inicio"),                  // Start
    Some("Fim"),                     // Finish
    Some("Inicio Cedo"),             // Early Start
    Some("Fim Cedo"),                // Early Finish
    Some("Inicio Tardio"),           // Late Start
    Some("Fim Tardio"),              // Late Finish
    Some("Inicio Linha de Base"),    // Baseline Start
    Some("Fim Linha de Base"),       // Baseline Finish
    Some("Inicio Real"),             // Actual Start
    Some("Fim Real"),                // Actual Finish
    Some("Inicio1"),                 // Start1
    Some("Fim1"),                    // Finish1
    Some("Inicio2"),                 // Start2
    Some("Fim2"),                    // Finish2
    Some("Inicio3"),                 // Start3
    Some("Fim3"),                    // Finish3
    Some("Variação Inicio"),         // Start Variance
    Some("Variação Fim"),            // Finish Variance
    Some("Limitação de Data"),       // Constraint Date
    None,                            // -
    Some("Predecessores"),           // Predecessors
    Some("Sucessores"),              // Successors
    Some("Nome dos Recursos"),       // Resource Names
    Some("Iniciais dos Recursos"),   // Resource Initials
    Some("ID Unico dos Predecessores"), // Unique ID Predecessors
    Some("ID Unico dos Sucessores"), // Unique ID Successors
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    Some("Fixado"),                  // Fixed
    Some("Milestone"),               // Milestone
    Some("Crítico"),                 // Critical
    Some("Marcado"),                 // Marked
    Some("Rollup"),                  // Rollup
    Some("BCWS"),                    // BCWS
    Some("BCWP"),                    // BCWP
    Some("SV"),                      // SV
    Some("CV"),                      // CV
    None,                            // -
    Some("ID"),                      // ID
    Some("Tipo de Limitação"),       // Constraint Type
    Some("Espera"),                  // Delay
    Some("Folga Livre"),             // Free Slack
    Some("Total Folga"),             // Total Slack
    Some("Prioridade"),              // Priority
    Some("Arquivo Subprojeto"),      // Subproject File
    Some("Projeto"),                 // Project
    Some("ID Unico"),                // Unique ID
    Some("Número Externo"),          // Outline Number
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    Some("Flag1"),                   // Flag1
    Some("Flag2"),                   // Flag2
    Some("Flag3"),                   // Flag3
    Some("Flag4"),                   // Flag4
    Some("Flag5"),                   // Flag5
    Some("Flag6"),                   // Flag6
    Some("Flag7"),                   // Flag7
    Some("Flag8"),                   // Flag8
    Some("Flag9"),                   // Flag9
    Some("Flag10"),                  // Flag10
    Some("Sumário"),                 // Summary
    Some("Objetos"),                 // Objects
    Some("Campos Ligados"),          // Linked Fields
    Some("Esconder Barra"),          // Hide Bar
    None,                            // -
    Some("Criado"),                  // Created
    Some("Inicio4"),                 // Start4
    Some("Fim4"),                    // Finish4
    Some("Inicio5"),                 // Start5
    Some("Fim5"),                    // Finish5
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    Some("Confirmado"),              // Confirmed
    Some("Atualização Necessária"),  // Update Needed
    None,                            // -
    None,                            // -
    None,                            // -
    Some("Número1"),                 // Number1
    Some("Número2"),                 // Number2
    Some("Número3"),                 // Number3
    Some("Número4"),                 // Number4
    Some("Número5"),                 // Number5
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    Some("Pare"),                    // Stop
    Some("Não Continuar Antes de"),  // Resume No Earlier Than
    Some("Continuar"),               // Resume
];

/// Portuguese name table for resource key `RESOURCE_NAMES` (indices correspond to MPX resource field numbers; the end-of-line comments are the English default names).
///
/// Corresponds to Java: `LocaleData_pt.RESOURCE_NAMES_DATA`.
pub const RESOURCE_NAMES_DATA: [Option<&'static str>; 52] = [
    None,                          // -
    Some("Nome"),                  // Name
    Some("Iniciais"),              // Initials
    Some("Grupo"),                 // Group
    Some("Código"),                // Code
    Some("Texto1"),                // Text1
    Some("Texto2"),                // Text2
    Some("Texto3"),                // Text3
    Some("Texto4"),                // Text4
    Some("Texto5"),                // Text5
    Some("Anotações"),             // Notes
    Some("Endereço Email"),        // Email Address
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    Some("Trabaho"),               // Work (spelling in the Java source)
    Some("Linha Base de Trabalho"), // Baseline Work
    Some("Trabalho Real"),         // Actual Work
    Some("Trabalho Restante"),     // Remaining Work
    Some("Trabalho Extra"),        // Overtime Work
    Some("Variação Trabalho"),     // Work Variance
    Some("% Trabalho Completo"),   // % Work Complete
    None,                          // -
    None,                          // -
    None,                          // -
    Some("Custo"),                 // Cost
    Some("Linha Base de Custo"),   // Baseline Cost
    Some("Custo Real"),            // Actual Cost
    Some("Custo Restante"),        // Remaining Cost
    Some("Variação de Custo"),     // Cost Variance
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    Some("ID"),                    // ID
    Some("Máximo de Unidades"),    // Max Units
    Some("Taxa Padrão"),           // Standard Rate
    Some("Taxa de Tempo Extra"),   // Overtime Rate
    Some("Custo por Uso"),         // Cost Per Use
    Some("Resulte em"),            // Accrue At
    Some("Sobrecarregado"),        // Overallocated
    Some("Pico"),                  // Peak
    Some("Calendário Base"),       // Base Calendar
    Some("ID Unico"),              // Unique ID
    Some("Objetos"),               // Objects
    Some("Campos Ligados"),        // Linked Fields
];

/// Corresponds to resource key `FILE_DELIMITER`
pub const FILE_DELIMITER: char = ';';
/// Corresponds to resource key `CURRENCY_SYMBOL` (empty string in the Java source)
pub const CURRENCY_SYMBOL: &'static str = "";
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
/// Corresponds to resource key `DATE_SEPARATOR`
pub const DATE_SEPARATOR: char = '/';
/// Corresponds to resource key `TIME_SEPARATOR`
pub const TIME_SEPARATOR: char = ':';
/// Corresponds to resource key `AM_TEXT`
pub const AM_TEXT: &'static str = "";
/// Corresponds to resource key `PM_TEXT`
pub const PM_TEXT: &'static str = "";
/// Corresponds to resource key `DATE_FORMAT`
pub const DATE_FORMAT: ProjectDateFormat = ProjectDateFormat::DdMmYyyy;
/// Corresponds to resource key `BAR_TEXT_DATE_FORMAT`
pub const BAR_TEXT_DATE_FORMAT: i32 = 0;
/// Corresponds to resource key `NA`
pub const NA: &'static str = "NA";
/// Corresponds to resource key `YES`
pub const YES: &'static str = "Sim";
/// Corresponds to resource key `NO`
pub const NO: &'static str = "Não";

/// Contents of the Portuguese resource bundle, corresponding to Java `LocaleData_pt.RESOURCE_DATA`
/// (i.e. the return value of `getContents()`).
pub static OVERRIDES: LocaleOverrides = LocaleOverrides {
    file_delimiter: Some(FILE_DELIMITER),
    program_name: None,
    file_version: None,
    code_page: None,
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
    use crate::code_page::CodePage;
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
        assert_eq!(TASK_NAMES_DATA[3], Some("Nível Externo"));
        assert_eq!(TASK_NAMES_DATA[14], Some("Anotações"));
        assert_eq!(TASK_NAMES_DATA[68], Some("Limitação de Data"));
        assert_eq!(TASK_NAMES_DATA[151], Some("Não Continuar Antes de"));
        assert_eq!(TASK_NAMES_DATA[152], Some("Continuar"));

        assert_eq!(RESOURCE_NAMES_DATA[11], Some("Endereço Email"));
        assert_eq!(RESOURCE_NAMES_DATA[20], Some("Trabaho"));
        assert_eq!(RESOURCE_NAMES_DATA[42], Some("Taxa Padrão"));
        assert_eq!(RESOURCE_NAMES_DATA[51], Some("Campos Ligados"));

        assert_eq!(TIME_UNITS_ARRAY_DATA[4], ["mes"].as_slice());
        assert_eq!(TIME_UNITS_ARRAY_DATA[11], ["emes"].as_slice());
        assert_eq!(ACCRUE_TYPES_DATA[2], Some(""));
        assert_eq!(RELATION_TYPES_DATA[0], Some("TT"));
        assert_eq!(PRIORITY_TYPES_DATA[8], Some("Altíssimo"));
        assert_eq!(CONSTRAINT_TYPES_DATA[4], Some("Não Iniciar Antes De"));
    }

    #[test]
    fn locale_data_returns_portuguese_tables() {
        assert_eq!(
            LocaleData::get_string_array(MpxLocale::Pt, LocaleData::TASK_NAMES),
            Some(TASK_NAMES_DATA.as_slice())
        );
        assert_eq!(LocaleData::get_string(MpxLocale::Pt, LocaleData::YES), Some("Sim"));
        assert_eq!(LocaleData::get_string(MpxLocale::Pt, LocaleData::NO), Some("Não"));
        assert_eq!(LocaleData::get_string(MpxLocale::Pt, LocaleData::NA), Some("NA"));
        // Keys that are not overridden fall back to English
        assert_eq!(
            LocaleData::get_string(MpxLocale::Pt, LocaleData::PROGRAM_NAME),
            Some("Microsoft Project for Windows")
        );
        assert_eq!(
            LocaleData::get_string(MpxLocale::Pt, LocaleData::FILE_VERSION),
            Some("4.0")
        );
        assert_eq!(LocaleData::get_code_page(MpxLocale::Pt), CodePage::Ansi);
    }
}
