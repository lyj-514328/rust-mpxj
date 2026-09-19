//! 对应 Java: `org/mpxj/mpx/LocaleData_fr.java`
//!
//! # 移植取舍（`ResourceBundle` / `ListResourceBundle`）
//!
//! Java 的 `LocaleData_fr` 是法语资源包，只覆盖部分键；未覆盖的键由 `ResourceBundle`
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

/// 法语时间单位别名表。
///
/// 对应 Java: `LocaleData_fr.TIME_UNITS_ARRAY_DATA`（Java 字面量顺序：分钟、小时、天、
/// 周、月、年、百分比、随后是各「已用」单位；下标 5 是年、下标 6 是百分比，
/// 与 `TimeUnit` 的取值顺序不同，此处照抄 Java）。
/// 下标 12（`ELAPSED_YEARS`）在 Java 源码中就是占位符 `"???"`，原样保留。
pub const TIME_UNITS_ARRAY_DATA: [&'static [&'static str]; 14] = [
    &["m"],    // 0 TimeUnit.MINUTES
    &["h"],    // 1 TimeUnit.HOURS
    &["j"],    // 2 TimeUnit.DAYS（jours）
    &["s"],    // 3 TimeUnit.WEEKS（semaines）
    &["ms"],   // 4 TimeUnit.MONTHS（mois）
    &["a"],    // 5 TimeUnit.PERCENT（Java 字面量位置，années）
    &["%"],    // 6 TimeUnit.YEARS（Java 字面量位置）
    &["me"],   // 7 TimeUnit.ELAPSED_MINUTES
    &["he"],   // 8 TimeUnit.ELAPSED_HOURS
    &["je"],   // 9 TimeUnit.ELAPSED_DAYS
    &["se"],   // 10 TimeUnit.ELAPSED_WEEKS
    &["mse"],  // 11 TimeUnit.ELAPSED_MONTHS
    &["???"],  // 12 TimeUnit.ELAPSED_YEARS（Java 源码中的占位符）
    &["e%"],   // 13 TimeUnit.ELAPSED_PERCENT
];

/// 法语资源键 `ACCRUE_TYPES` 的名表。
///
/// 对应 Java: `LocaleData_fr.ACCRUE_TYPES_DATA`。
pub const ACCRUE_TYPES_DATA: [Option<&'static str>; 3] =
    [Some("Début"), Some("Fin"), Some("Proportion")];

/// 法语资源键 `RELATION_TYPES` 的名表。
///
/// 对应 Java: `LocaleData_fr.RELATION_TYPES_DATA`。
pub const RELATION_TYPES_DATA: [Option<&'static str>; 4] =
    [Some("FF"), Some("FD"), Some("DF"), Some("DD")];

/// 法语资源键 `PRIORITY_TYPES` 的名表。
///
/// 对应 Java: `LocaleData_fr.PRIORITY_TYPES_DATA`。
pub const PRIORITY_TYPES_DATA: [Option<&'static str>; 10] = [
    Some("Le Plus Bas"),
    Some("Très Bas"),
    Some("Plus Bas"),
    Some("Bas"),
    Some("Moyen"),
    Some("Elevé"),
    Some("Plus Elevé"),
    Some("Très Elevé"),
    Some("Le Plus Elevé"),
    Some("Ne Pas Niveler"),
];

/// 法语资源键 `CONSTRAINT_TYPES` 的名表。
///
/// 对应 Java: `LocaleData_fr.CONSTRAINT_TYPES_DATA`。
pub const CONSTRAINT_TYPES_DATA: [Option<&'static str>; 8] = [
    Some("Dès Que Possible"),
    Some("Le Plus Tard Possible"),
    Some("Doit Commencer Le"),
    Some("Doit Finir Le"),
    Some("Début Au Plus Tôt Le"),
    Some("Début Au Plus Tard Le"),
    Some("Fin Au Plus Tôt Le"),
    Some("Fin Au Plus Tard Le"),
];

/// 法语资源键 `TASK_NAMES` 的名表（下标与 MPX 任务字段号对应；行末注释为英文默认名）。
///
/// 对应 Java: `LocaleData_fr.TASK_NAMES_DATA`。
pub const TASK_NAMES_DATA: [Option<&'static str>; 153] = [
    None,                              // -
    Some("Nom"),                       // Name
    Some("WBS"),                       // WBS
    Some("Niveau hiérarchique"),       // Outline Level
    Some("Texte1"),                    // Text1
    Some("Texte2"),                    // Text2
    Some("Texte3"),                    // Text3
    Some("Texte4"),                    // Text4
    Some("Texte5"),                    // Text5
    Some("Texte6"),                    // Text6
    Some("Texte7"),                    // Text7
    Some("Texte8"),                    // Text8
    Some("Texte9"),                    // Text9
    Some("Texte10"),                   // Text10
    Some("Notes"),                     // Notes
    Some("Contact"),                   // Contact
    Some("Groupe de Ressources"),      // Resource Group
    None,                              // -
    None,                              // -
    None,                              // -
    Some("Travail"),                   // Work
    Some("Travail Normal"),            // Baseline Work
    Some("Travail Réel"),              // Actual Work
    Some("Travail Restant"),           // Remaining Work
    Some("Variation de Travail"),      // Work Variance
    Some("% Travail achevé"),          // % Work Complete
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    Some("Coût"),                      // Cost
    Some("Coût Planifié"),             // Baseline Cost
    Some("Coût Réel"),                 // Actual Cost
    Some("Coût Restant"),              // Remaining Cost
    Some("Variation de Coût"),         // Cost Variance
    Some("Coût Fixe"),                 // Fixed Cost
    Some("Coût1"),                     // Cost1
    Some("Coût2"),                     // Cost2
    Some("Coût3"),                     // Cost3
    None,                              // -
    Some("Durée"),                     // Duration
    Some("Durée Planifiée"),           // Baseline Duration
    Some("Durée Réelle"),              // Actual Duration
    Some("Durée Restante"),            // Remaining Duration
    Some("% Achevé"),                  // % Complete
    Some("Variation de Durée"),        // Duration Variance
    Some("Durée1"),                    // Duration1
    Some("Durée2"),                    // Duration2
    Some("Durée3"),                    // Duration3
    None,                              // -
    Some("Début"),                     // Start
    Some("Fin"),                       // Finish
    Some("Début Au Plus Tôt"),         // Early Start
    Some("Fin Au Plus Tôt"),           // Early Finish
    Some("Début Au Plus Tard"),        // Late Start
    Some("Fin Au Plus Tard"),          // Late Finish
    Some("Début Planifié"),            // Baseline Start
    Some("Fin Planifiée"),             // Baseline Finish
    Some("Début Réel"),                // Actual Start
    Some("Fin Réelle"),                // Actual Finish
    Some("Début1"),                    // Start1
    Some("Fin1"),                      // Finish1
    Some("Début2"),                    // Start2
    Some("Fin2"),                      // Finish2
    Some("Début3"),                    // Start3
    Some("Fin3"),                      // Finish3
    Some("Marge de Début"),            // Start Variance
    Some("Marge de Fin"),              // Finish Variance
    Some("Date Contrainte"),           // Constraint Date
    None,                              // -
    Some("Prédécesseurs"),             // Predecessors
    Some("Successeurs"),               // Successors
    Some("Noms Ressources"),           // Resource Names
    Some("Ressources Initiales"),      // Resource Initials
    Some("ID Unique des Prédécesseurs"), // Unique ID Predecessors
    Some("ID Unique des Successeurs"), // Unique ID Successors
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    Some("Fixe"),                      // Fixed
    Some("Jalon"),                     // Milestone
    Some("Critique"),                  // Critical
    Some("Marqué"),                    // Marked
    Some("Rollup"),                    // Rollup
    Some("BCWS"),                      // BCWS
    Some("BCWP"),                      // BCWP
    Some("SV"),                        // SV
    Some("CV"),                        // CV
    None,                              // -
    Some("ID"),                        // ID
    Some("Type de Contrainte"),        // Constraint Type
    Some("Délai"),                     // Delay
    Some("Marge Libre"),               // Free Slack
    Some("Marge Totale"),              // Total Slack
    Some("Priorité"),                  // Priority
    Some("Fichier de sous-projet"),    // Subproject File
    Some("Projet"),                    // Project
    Some("ID Unique"),                 // Unique ID
    Some("Numéro Externe"),            // Outline Number
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
    Some("Indicateur1"),               // Flag1
    Some("Indicateur2"),               // Flag2
    Some("Indicateur3"),               // Flag3
    Some("Indicateur4"),               // Flag4
    Some("Indicateur5"),               // Flag5
    Some("Indicateur6"),               // Flag6
    Some("Indicateur7"),               // Flag7
    Some("Indicateur8"),               // Flag8
    Some("Indicateur9"),               // Flag9
    Some("Indicateur10"),              // Flag10
    Some("Sommaire"),                  // Summary
    Some("Objets"),                    // Objects
    Some("Champs Liés"),               // Linked Fields
    Some("Cacher Barre"),              // Hide Bar
    None,                              // -
    Some("Créé"),                      // Created
    Some("Début4"),                    // Start4
    Some("Fin4"),                      // Finish4
    Some("Début5"),                    // Start5
    Some("Fin5"),                      // Finish5
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    Some("Confirmé"),                  // Confirmed
    Some("Mise à Jour Nécessaire"),    // Update Needed
    None,                              // -
    None,                              // -
    None,                              // -
    Some("Numero1"),                   // Number1
    Some("Numero2"),                   // Number2
    Some("Numero3"),                   // Number3
    Some("Numero4"),                   // Number4
    Some("Numero5"),                   // Number5
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    Some("Stop"),                      // Stop
    Some("Continuer Pas Plus Tôt Que"), // Resume No Earlier Than
    Some("Continuer"),                 // Resume
];

/// 法语资源键 `RESOURCE_NAMES` 的名表（下标与 MPX 资源字段号对应；行末注释为英文默认名）。
///
/// 对应 Java: `LocaleData_fr.RESOURCE_NAMES_DATA`。
pub const RESOURCE_NAMES_DATA: [Option<&'static str>; 52] = [
    None,                          // -
    Some("Nom"),                   // Name
    Some("Initiales"),             // Initials
    Some("Groupe"),                // Group
    Some("Code"),                  // Code
    Some("Texte1"),                // Text1
    Some("Texte2"),                // Text2
    Some("Texte3"),                // Text3
    Some("Texte4"),                // Text4
    Some("Texte5"),                // Text5
    Some("Notes"),                 // Notes
    Some("Adresse de messagerie"), // Email Address
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    Some("Travail"),               // Work
    Some("Travail Planifié"),      // Baseline Work
    Some("Travail Réel"),          // Actual Work
    Some("Travail Restant"),       // Remaining Work
    Some("Heures sup."),           // Overtime Work
    Some("Variation de Travail"),  // Work Variance
    Some("% Travail achevé"),      // % Work Complete
    None,                          // -
    None,                          // -
    None,                          // -
    Some("Coût"),                  // Cost
    Some("Coût Planifié"),         // Baseline Cost
    Some("Coût Réel"),             // Actual Cost
    Some("Coût Restant"),          // Remaining Cost
    Some("Variation de Coût"),     // Cost Variance
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    Some("ID"),                    // ID
    Some("Unités Maximales"),      // Max Units
    Some("Taux Standard"),         // Standard Rate
    Some("Taux heures sup."),      // Overtime Rate
    Some("Coût par Utilisation"),  // Cost Per Use
    Some("Allocation"),            // Accrue At
    Some("En Surcharge"),          // Overallocated
    Some("Pointe"),                // Peak
    Some("Calendrier de Base"),    // Base Calendar
    Some("ID Unique"),             // Unique ID
    Some("Objets"),                // Objects
    Some("Champs Liés"),           // Linked Fields
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
pub const NA: &'static str = "NC";
/// 对应资源键 `YES`
pub const YES: &'static str = "Oui";
/// 对应资源键 `NO`
pub const NO: &'static str = "Non";

/// 法语资源包的内容，对应 Java `LocaleData_fr.RESOURCE_DATA`
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
        assert_eq!(TASK_NAMES_DATA[1], Some("Nom"));
        assert_eq!(TASK_NAMES_DATA[3], Some("Niveau hiérarchique"));
        assert_eq!(TASK_NAMES_DATA[30], Some("Coût"));
        assert_eq!(TASK_NAMES_DATA[74], Some("ID Unique des Prédécesseurs"));
        assert_eq!(TASK_NAMES_DATA[136], Some("Mise à Jour Nécessaire"));
        assert_eq!(TASK_NAMES_DATA[152], Some("Continuer"));

        assert_eq!(RESOURCE_NAMES_DATA[11], Some("Adresse de messagerie"));
        assert_eq!(RESOURCE_NAMES_DATA[24], Some("Heures sup."));
        assert_eq!(RESOURCE_NAMES_DATA[34], Some("Variation de Coût"));
        assert_eq!(RESOURCE_NAMES_DATA[51], Some("Champs Liés"));

        assert_eq!(TIME_UNITS_ARRAY_DATA[2], ["j"].as_slice());
        assert_eq!(TIME_UNITS_ARRAY_DATA[12], ["???"].as_slice());
        assert_eq!(ACCRUE_TYPES_DATA[0], Some("Début"));
        assert_eq!(RELATION_TYPES_DATA[3], Some("DD"));
        assert_eq!(PRIORITY_TYPES_DATA[1], Some("Très Bas"));
        assert_eq!(CONSTRAINT_TYPES_DATA[7], Some("Fin Au Plus Tard Le"));
    }

    #[test]
    fn locale_data_returns_french_tables() {
        assert_eq!(
            LocaleData::get_string_array(MpxLocale::Fr, LocaleData::TASK_NAMES),
            Some(TASK_NAMES_DATA.as_slice())
        );
        assert_eq!(LocaleData::get_string(MpxLocale::Fr, LocaleData::NA), Some("NC"));
        assert_eq!(LocaleData::get_string(MpxLocale::Fr, LocaleData::YES), Some("Oui"));
        assert_eq!(LocaleData::get_string(MpxLocale::Fr, LocaleData::NO), Some("Non"));
        assert_eq!(LocaleData::get_string(MpxLocale::Fr, LocaleData::CURRENCY_SYMBOL), Some(""));
        // 未覆盖的键回落英文
        assert_eq!(
            LocaleData::get_integer(MpxLocale::Fr, LocaleData::DEFAULT_START_TIME),
            Some(480)
        );
    }
}
