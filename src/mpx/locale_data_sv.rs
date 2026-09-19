//! 对应 Java: `org/mpxj/mpx/LocaleData_sv.java`
//!
//! # 移植取舍（`ResourceBundle` / `ListResourceBundle`）
//!
//! Java 的 `LocaleData_sv` 是瑞典语资源包，只覆盖部分键；未覆盖的键由 `ResourceBundle`
//! 从基类回落取值。Rust 侧把该语言实际覆盖的键写成模块级 `pub const`
//! （标量常量按 Java `RESOURCE_DATA` 的键名命名，数组常量沿用 Java 的字段名），
//! 并用 [`OVERRIDES`]（对应 `getContents()` 的返回值）汇总；未覆盖的键为 `None`，
//! 由 `locale_data` 模块回落到英文默认资源。取值逻辑见 `locale_data` 模块头部说明；
//! `ListResourceBundle` 的协议方法由常量表替代，不移植。
//!
//! 本语言未覆盖的键：`PROGRAM_NAME`、`CODE_PAGE`、`CURRENCY_DIGITS`、`DEFAULT_START_TIME`、
//! `BAR_TEXT_DATE_FORMAT`、`NA`、`ACCRUE_TYPES`、`RELATION_TYPES`、`PRIORITY_TYPES`、
//! `CONSTRAINT_TYPES`。
//!
//! Java 源文件中部分条目仍写着英文原名（标注 `// Translate`），此处按源码字面量照抄。
//!
//! 未移植的成员：无。

use crate::currency_symbol_position::CurrencySymbolPosition;
use crate::date_order::DateOrder;
use crate::mpx::locale_data::LocaleOverrides;
use crate::project_date_format::ProjectDateFormat;
use crate::project_time_format::ProjectTimeFormat;

/// 瑞典语时间单位别名表。
///
/// 对应 Java: `LocaleData_sv.TIME_UNITS_ARRAY_DATA`（Java 字面量顺序：分钟、小时、天、
/// 周、月、年、百分比、随后是各「已用」单位；下标 5 是年、下标 6 是百分比，
/// 与 `TimeUnit` 的取值顺序不同，此处照抄 Java）。
pub const TIME_UNITS_ARRAY_DATA: [&'static [&'static str]; 14] = [
    &["m"],      // 0 TimeUnit.MINUTES
    &["t"],      // 1 TimeUnit.HOURS
    &["d"],      // 2 TimeUnit.DAYS
    &["w"],      // 3 TimeUnit.WEEKS
    &["mon"],    // 4 TimeUnit.MONTHS
    &["y"],      // 5 TimeUnit.PERCENT（Java 字面量位置）
    &["%"],      // 6 TimeUnit.YEARS（Java 字面量位置）
    &["em"],     // 7 TimeUnit.ELAPSED_MINUTES
    &["eh"],     // 8 TimeUnit.ELAPSED_HOURS
    &["ed"],     // 9 TimeUnit.ELAPSED_DAYS
    &["ew"],     // 10 TimeUnit.ELAPSED_WEEKS
    &["emon"],   // 11 TimeUnit.ELAPSED_MONTHS
    &["ey"],     // 12 TimeUnit.ELAPSED_YEARS
    &["e%"],     // 13 TimeUnit.ELAPSED_PERCENT
];

/// 瑞典语资源键 `TASK_NAMES` 的名表（下标与 MPX 任务字段号对应；行末注释为英文默认名）。
///
/// 对应 Java: `LocaleData_sv.TASK_NAMES_DATA`。
pub const TASK_NAMES_DATA: [Option<&'static str>; 153] = [
    None,                            // -
    Some("Namn"),                    // Name
    Some("WBS"),                     // WBS
    Some("Dispositionsnivå"),        // Outline Level
    Some("Text1"),                   // Text1
    Some("Text2"),                   // Text2
    Some("Text3"),                   // Text3
    Some("Text4"),                   // Text4
    Some("Text5"),                   // Text5
    Some("Text6"),                   // Text6
    Some("Text7"),                   // Text7
    Some("Text8"),                   // Text8
    Some("Text9"),                   // Text9
    Some("Text10"),                  // Text10
    Some("Notes"),                   // Notes（Java 源码未翻译）
    Some("Contact"),                 // Contact（Java 源码未翻译）
    Some("Resource"),                // Resource Group（Java 源码未翻译）
    None,                            // -
    None,                            // -
    None,                            // -
    Some("Arbete"),                  // Work
    Some("Originalarbete"),          // Baseline Work
    Some("Verkligt arbete"),         // Actual Work
    Some("Remaining Work"),          // Remaining Work（Java 源码未翻译）
    Some("Work Variance"),           // Work Variance（Java 源码未翻译）
    Some("% Work Complete"),         // % Work Complete（Java 源码未翻译）
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    Some("Kostnad"),                 // Cost
    Some("Originalkostnad"),         // Baseline Cost
    Some("Verklig kostnad"),         // Actual Cost
    Some("Återstående kostnad"),     // Remaining Cost
    Some("Cost Variance"),           // Cost Variance（Java 源码未翻译）
    Some("Fast kostnad"),            // Fixed Cost
    Some("Kostnad1"),                // Cost1
    Some("Kostnad2"),                // Cost2
    Some("Kostnad3"),                // Cost3
    None,                            // -
    Some("Varaktighet"),             // Duration
    Some("Originalvaraktighet"),     // Baseline Duration
    Some("Actual Duration"),         // Actual Duration（Java 源码未翻译）
    Some("Remaining Duration"),      // Remaining Duration（Java 源码未翻译）
    Some("% färdigt"),               // % Complete
    Some("Duration Variance"),       // Duration Variance（Java 源码未翻译）
    Some("Varaktighet1"),            // Duration1
    Some("Varaktighet2"),            // Duration2
    Some("Varaktighet3"),            // Duration3
    None,                            // -
    Some("Start"),                   // Start
    Some("Slut"),                    // Finish
    Some("Tidig start"),             // Early Start
    Some("Tidigt slut"),             // Early Finish
    Some("Sen start"),               // Late Start
    Some("Sent slut"),               // Late Finish
    Some("Originalstart"),           // Baseline Start
    Some("Originalslut"),            // Baseline Finish
    Some("Verklig start"),           // Actual Start
    Some("Verkligt slut"),           // Actual Finish
    Some("Start1"),                  // Start1（Java 源码未翻译）
    Some("Finish1"),                 // Finish1（Java 源码未翻译）
    Some("Start2"),                  // Start2（Java 源码未翻译）
    Some("Finish2"),                 // Finish2（Java 源码未翻译）
    Some("Start3"),                  // Start3（Java 源码未翻译）
    Some("Finish3"),                 // Finish3（Java 源码未翻译）
    Some("Start Variance"),          // Start Variance（Java 源码未翻译）
    Some("Finish Variance"),         // Finish Variance（Java 源码未翻译）
    Some("Villkorsdatum"),           // Constraint Date
    None,                            // -
    Some("Föregående aktiviteter"),  // Predecessors
    Some("Successors"),              // Successors（Java 源码未翻译）
    Some("Resource Names"),          // Resource Names（Java 源码未翻译）
    Some("Resource Initials"),       // Resource Initials（Java 源码未翻译）
    Some("Unique ID Predecessors"),  // Unique ID Predecessors（Java 源码未翻译）
    Some("Unique ID Successors"),    // Unique ID Successors（Java 源码未翻译）
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    Some("Fast"),                    // Fixed
    Some("Milstolpe"),               // Milestone
    Some("Critical"),                // Critical（Java 源码未翻译）
    Some("Märkt"),                   // Marked
    Some("Upplyft"),                 // Rollup
    Some("BCWS"),                    // BCWS（Java 源码未翻译）
    Some("BCWP"),                    // BCWP（Java 源码未翻译）
    Some("SV"),                      // SV（Java 源码未翻译）
    Some("CV"),                      // CV（Java 源码未翻译）
    None,                            // -
    Some("ID"),                      // ID
    Some("Villkorstyp"),             // Constraint Type
    Some("Förskjutning"),            // Delay
    Some("Fritt slack"),             // Free Slack
    Some("Totalt slack"),            // Total Slack
    Some("Prioritet"),               // Priority
    Some("Underprojektsfil"),        // Subproject File
    Some("Project"),                 // Project（Java 源码未翻译）
    Some("Eget ID"),                 // Unique ID
    Some("Outline Number"),          // Outline Number（Java 源码未翻译）
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
    Some("Flagga1"),                 // Flag1
    Some("Flagga2"),                 // Flag2
    Some("Flagga3"),                 // Flag3
    Some("Flagga4"),                 // Flag4
    Some("Flagga5"),                 // Flag5
    Some("Flagga6"),                 // Flag6
    Some("Flagga7"),                 // Flag7
    Some("Flagga8"),                 // Flag8
    Some("Flagga9"),                 // Flag9
    Some("Flagga10"),                // Flag10
    Some("Sammanfattning"),          // Summary
    Some("Objects"),                 // Objects（Java 源码未翻译）
    Some("Linked Fields"),           // Linked Fields（Java 源码未翻译）
    Some("Hide Bar"),                // Hide Bar（Java 源码未翻译）
    None,                            // -
    Some("Skapad"),                  // Created
    Some("Start4"),                  // Start4（Java 源码未翻译）
    Some("Finish4"),                 // Finish4（Java 源码未翻译）
    Some("Start5"),                  // Start5（Java 源码未翻译）
    Some("Finish5"),                 // Finish5（Java 源码未翻译）
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    Some("Confirmed"),               // Confirmed（Java 源码未翻译）
    Some("Update Needed"),           // Update Needed（Java 源码未翻译）
    None,                            // -
    None,                            // -
    None,                            // -
    Some("Tal1"),                    // Number1
    Some("Tal2"),                    // Number2
    Some("Tal3"),                    // Number3
    Some("Tal4"),                    // Number4
    Some("Tal5"),                    // Number5
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    Some("Stopp"),                   // Stop
    Some("Fortsätt tidigast"),       // Resume No Earlier Than
    Some("Resume"),                  // Resume（Java 源码未翻译）
];

/// 瑞典语资源键 `RESOURCE_NAMES` 的名表（下标与 MPX 资源字段号对应；行末注释为英文默认名）。
///
/// 对应 Java: `LocaleData_sv.RESOURCE_NAMES_DATA`。
pub const RESOURCE_NAMES_DATA: [Option<&'static str>; 52] = [
    None,                       // -
    Some("Namn"),               // Name
    Some("Initialer"),          // Initials
    Some("Grupp"),              // Group
    Some("Kod"),                // Code
    Some("Text1"),              // Text1
    Some("Text2"),              // Text2
    Some("Text3"),              // Text3
    Some("Text4"),              // Text4
    Some("Text5"),              // Text5
    Some("Notes"),              // Notes（Java 源码未翻译）
    Some("e-post"),             // Email Address
    None,                       // -
    None,                       // -
    None,                       // -
    None,                       // -
    None,                       // -
    None,                       // -
    None,                       // -
    None,                       // -
    Some("Arbete"),             // Work
    Some("Originalarbete"),     // Baseline Work
    Some("Verkligt arbete"),    // Actual Work
    Some("Remaining Work"),     // Remaining Work（Java 源码未翻译）
    Some("Övertidsarbete"),     // Overtime Work
    Some("Work Variance"),      // Work Variance（Java 源码未翻译）
    Some("% Work Complete"),    // % Work Complete（Java 源码未翻译）
    None,                       // -
    None,                       // -
    None,                       // -
    Some("Kostnad"),            // Cost
    Some("Originalkostnad"),    // Baseline Cost
    Some("Verklig kostnad"),    // Actual Cost
    Some("Remaining Cost"),     // Remaining Cost（Java 源码未翻译）
    Some("Cost Variance"),      // Cost Variance（Java 源码未翻译）
    None,                       // -
    None,                       // -
    None,                       // -
    None,                       // -
    None,                       // -
    Some("ID"),                 // ID
    Some("Max enheter"),        // Max Units
    Some("Standardkostnad"),    // Standard Rate
    Some("Övertidskostnad"),    // Overtime Rate
    Some("Kostnad per tillfälle"), // Cost Per Use
    Some("Påförs"),             // Accrue At
    Some("Overallocated"),      // Overallocated（Java 源码未翻译）
    Some("Peak"),               // Peak（Java 源码未翻译）
    Some("Base Calendar"),      // Base Calendar（Java 源码未翻译）
    Some("Eget ID"),            // Unique ID
    Some("Objects"),            // Objects（Java 源码未翻译）
    Some("Linked Fields"),      // Linked Fields（Java 源码未翻译）
];

/// 对应资源键 `FILE_DELIMITER`
pub const FILE_DELIMITER: char = ';';
/// 对应资源键 `FILE_VERSION`
pub const FILE_VERSION: &'static str = "4,0";
/// 对应资源键 `YES`
pub const YES: &'static str = "Ja";
/// 对应资源键 `NO`
pub const NO: &'static str = "Nej";
/// 对应资源键 `CURRENCY_SYMBOL`
pub const CURRENCY_SYMBOL: &'static str = "kr";
/// 对应资源键 `CURRENCY_SYMBOL_POSITION`
pub const CURRENCY_SYMBOL_POSITION: CurrencySymbolPosition = CurrencySymbolPosition::AfterWithSpace;
/// 对应资源键 `CURRENCY_THOUSANDS_SEPARATOR`（Java 侧为单个空格）
pub const CURRENCY_THOUSANDS_SEPARATOR: char = ' ';
/// 对应资源键 `CURRENCY_DECIMAL_SEPARATOR`
pub const CURRENCY_DECIMAL_SEPARATOR: char = ',';
/// 对应资源键 `DATE_ORDER`
pub const DATE_ORDER: DateOrder = DateOrder::Ymd;
/// 对应资源键 `TIME_FORMAT`
pub const TIME_FORMAT: ProjectTimeFormat = ProjectTimeFormat::TwentyFourHour;
/// 对应资源键 `DATE_SEPARATOR`
pub const DATE_SEPARATOR: char = '-';
/// 对应资源键 `TIME_SEPARATOR`
pub const TIME_SEPARATOR: char = ':';
/// 对应资源键 `AM_TEXT`
pub const AM_TEXT: &'static str = "";
/// 对应资源键 `PM_TEXT`
pub const PM_TEXT: &'static str = "";
/// 对应资源键 `DATE_FORMAT`
pub const DATE_FORMAT: ProjectDateFormat = ProjectDateFormat::EeeDdMmYy;

/// 瑞典语资源包的内容，对应 Java `LocaleData_sv.RESOURCE_DATA`
/// （即 `getContents()` 的返回值）。
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
    date_order: Some(DATE_ORDER),
    time_format: Some(TIME_FORMAT),
    default_start_time: None,
    date_separator: Some(DATE_SEPARATOR),
    time_separator: Some(TIME_SEPARATOR),
    am_text: Some(AM_TEXT),
    pm_text: Some(PM_TEXT),
    date_format: Some(DATE_FORMAT),
    bar_text_date_format: None,
    na: None,
    yes: Some(YES),
    no: Some(NO),
    time_units_array: Some(&TIME_UNITS_ARRAY_DATA),
    accrue_types: None,
    relation_types: None,
    priority_types: None,
    constraint_types: None,
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
        assert_eq!(TASK_NAMES_DATA.len(), 153);
        assert_eq!(RESOURCE_NAMES_DATA.len(), 52);
    }

    #[test]
    fn sampled_elements_match_java() {
        assert_eq!(TASK_NAMES_DATA[1], Some("Namn"));
        assert_eq!(TASK_NAMES_DATA[3], Some("Dispositionsnivå"));
        assert_eq!(TASK_NAMES_DATA[14], Some("Notes"));
        assert_eq!(TASK_NAMES_DATA[33], Some("Återstående kostnad"));
        assert_eq!(TASK_NAMES_DATA[100], None);
        assert_eq!(TASK_NAMES_DATA[120], Some("Sammanfattning"));
        assert_eq!(TASK_NAMES_DATA[152], Some("Resume"));

        assert_eq!(RESOURCE_NAMES_DATA[2], Some("Initialer"));
        assert_eq!(RESOURCE_NAMES_DATA[23], Some("Remaining Work"));
        assert_eq!(RESOURCE_NAMES_DATA[33], Some("Remaining Cost"));
        assert_eq!(RESOURCE_NAMES_DATA[51], Some("Linked Fields"));

        assert_eq!(TIME_UNITS_ARRAY_DATA[1], ["t"].as_slice());
        assert_eq!(TIME_UNITS_ARRAY_DATA[13], ["e%"].as_slice());
    }

    #[test]
    fn locale_data_returns_swedish_tables() {
        assert_eq!(
            LocaleData::get_string_array(MpxLocale::Sv, LocaleData::TASK_NAMES),
            Some(TASK_NAMES_DATA.as_slice())
        );
        assert_eq!(LocaleData::get_string(MpxLocale::Sv, LocaleData::NO), Some("Nej"));
        assert_eq!(LocaleData::get_date_order(MpxLocale::Sv), DateOrder::Ymd);
        assert_eq!(LocaleData::get_char(MpxLocale::Sv, LocaleData::DATE_SEPARATOR), Some('-'));
        assert_eq!(
            LocaleData::get_char(MpxLocale::Sv, LocaleData::CURRENCY_THOUSANDS_SEPARATOR),
            Some(' ')
        );
        // 未覆盖的键回落英文
        assert_eq!(
            LocaleData::get_string_array(MpxLocale::Sv, LocaleData::ACCRUE_TYPES),
            LocaleData::get_string_array(MpxLocale::En, LocaleData::ACCRUE_TYPES)
        );
        assert_eq!(LocaleData::get_string(MpxLocale::Sv, LocaleData::NA), Some("NA"));
        assert_eq!(
            LocaleData::get_integer(MpxLocale::Sv, LocaleData::DEFAULT_START_TIME),
            Some(480)
        );
    }
}
