//! Corresponds to Java: `org/mpxj/mpx/LocaleData_sv.java`
//!
//! # Porting trade-offs (`ResourceBundle` / `ListResourceBundle`)
//!
//! Java's `LocaleData_sv` is a Swedish resource bundle covering only some keys; keys it
//! does not cover fall back to the base class in `ResourceBundle`. On the Rust side the
//! keys this locale actually covers are written as module-level `pub const` values (scalar
//! constants are named after the Java `RESOURCE_DATA` keys, array constants keep the Java
//! field names) and aggregated by [`OVERRIDES`] (corresponding to the return value of
//! `getContents()`); uncovered keys are `None` and fall back to the English default
//! resources in the `locale_data` module. See the header of the `locale_data` module for
//! the lookup logic; the `ListResourceBundle` protocol methods are replaced by constant
//! tables and are not ported.
//!
//! Keys not covered by this locale: `PROGRAM_NAME`, `CODE_PAGE`, `CURRENCY_DIGITS`,
//! `DEFAULT_START_TIME`, `BAR_TEXT_DATE_FORMAT`, `NA`, `ACCRUE_TYPES`, `RELATION_TYPES`,
//! `PRIORITY_TYPES`, `CONSTRAINT_TYPES`.
//!
//! Some entries in the Java source still use their English original names (marked
//! `// Translate`); they are copied here verbatim from the source literals.
//!
//! Skipped Java members: none.

use crate::currency_symbol_position::CurrencySymbolPosition;
use crate::date_order::DateOrder;
use crate::mpx::locale_data::LocaleOverrides;
use crate::project_date_format::ProjectDateFormat;
use crate::project_time_format::ProjectTimeFormat;

/// Swedish time unit alias table.
///
/// Corresponds to Java: `LocaleData_sv.TIME_UNITS_ARRAY_DATA` (Java literal order: minutes,
/// hours, days, weeks, months, years, percent, followed by the elapsed units; index 5 is
/// years and index 6 is percent, differing from the `TimeUnit` value order, copied from Java).
pub const TIME_UNITS_ARRAY_DATA: [&'static [&'static str]; 14] = [
    &["m"],      // 0 TimeUnit.MINUTES
    &["t"],      // 1 TimeUnit.HOURS
    &["d"],      // 2 TimeUnit.DAYS
    &["w"],      // 3 TimeUnit.WEEKS
    &["mon"],    // 4 TimeUnit.MONTHS
    &["y"],      // 5 TimeUnit.PERCENT (Java literal position)
    &["%"],      // 6 TimeUnit.YEARS (Java literal position)
    &["em"],     // 7 TimeUnit.ELAPSED_MINUTES
    &["eh"],     // 8 TimeUnit.ELAPSED_HOURS
    &["ed"],     // 9 TimeUnit.ELAPSED_DAYS
    &["ew"],     // 10 TimeUnit.ELAPSED_WEEKS
    &["emon"],   // 11 TimeUnit.ELAPSED_MONTHS
    &["ey"],     // 12 TimeUnit.ELAPSED_YEARS
    &["e%"],     // 13 TimeUnit.ELAPSED_PERCENT
];

/// Name table for the Swedish resource key `TASK_NAMES` (indices correspond to MPX task field numbers; end-of-line comments give the English default name).
///
/// Corresponds to Java: `LocaleData_sv.TASK_NAMES_DATA`.
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
    Some("Notes"),                   // Notes (not translated in Java source)
    Some("Contact"),                 // Contact (not translated in Java source)
    Some("Resource"),                // Resource Group (not translated in Java source)
    None,                            // -
    None,                            // -
    None,                            // -
    Some("Arbete"),                  // Work
    Some("Originalarbete"),          // Baseline Work
    Some("Verkligt arbete"),         // Actual Work
    Some("Remaining Work"),          // Remaining Work (not translated in Java source)
    Some("Work Variance"),           // Work Variance (not translated in Java source)
    Some("% Work Complete"),         // % Work Complete (not translated in Java source)
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    Some("Kostnad"),                 // Cost
    Some("Originalkostnad"),         // Baseline Cost
    Some("Verklig kostnad"),         // Actual Cost
    Some("Återstående kostnad"),     // Remaining Cost
    Some("Cost Variance"),           // Cost Variance (not translated in Java source)
    Some("Fast kostnad"),            // Fixed Cost
    Some("Kostnad1"),                // Cost1
    Some("Kostnad2"),                // Cost2
    Some("Kostnad3"),                // Cost3
    None,                            // -
    Some("Varaktighet"),             // Duration
    Some("Originalvaraktighet"),     // Baseline Duration
    Some("Actual Duration"),         // Actual Duration (not translated in Java source)
    Some("Remaining Duration"),      // Remaining Duration (not translated in Java source)
    Some("% färdigt"),               // % Complete
    Some("Duration Variance"),       // Duration Variance (not translated in Java source)
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
    Some("Start1"),                  // Start1 (not translated in Java source)
    Some("Finish1"),                 // Finish1 (not translated in Java source)
    Some("Start2"),                  // Start2 (not translated in Java source)
    Some("Finish2"),                 // Finish2 (not translated in Java source)
    Some("Start3"),                  // Start3 (not translated in Java source)
    Some("Finish3"),                 // Finish3 (not translated in Java source)
    Some("Start Variance"),          // Start Variance (not translated in Java source)
    Some("Finish Variance"),         // Finish Variance (not translated in Java source)
    Some("Villkorsdatum"),           // Constraint Date
    None,                            // -
    Some("Föregående aktiviteter"),  // Predecessors
    Some("Successors"),              // Successors (not translated in Java source)
    Some("Resource Names"),          // Resource Names (not translated in Java source)
    Some("Resource Initials"),       // Resource Initials (not translated in Java source)
    Some("Unique ID Predecessors"),  // Unique ID Predecessors (not translated in Java source)
    Some("Unique ID Successors"),    // Unique ID Successors (not translated in Java source)
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    Some("Fast"),                    // Fixed
    Some("Milstolpe"),               // Milestone
    Some("Critical"),                // Critical (not translated in Java source)
    Some("Märkt"),                   // Marked
    Some("Upplyft"),                 // Rollup
    Some("BCWS"),                    // BCWS (not translated in Java source)
    Some("BCWP"),                    // BCWP (not translated in Java source)
    Some("SV"),                      // SV (not translated in Java source)
    Some("CV"),                      // CV (not translated in Java source)
    None,                            // -
    Some("ID"),                      // ID
    Some("Villkorstyp"),             // Constraint Type
    Some("Förskjutning"),            // Delay
    Some("Fritt slack"),             // Free Slack
    Some("Totalt slack"),            // Total Slack
    Some("Prioritet"),               // Priority
    Some("Underprojektsfil"),        // Subproject File
    Some("Project"),                 // Project (not translated in Java source)
    Some("Eget ID"),                 // Unique ID
    Some("Outline Number"),          // Outline Number (not translated in Java source)
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
    Some("Objects"),                 // Objects (not translated in Java source)
    Some("Linked Fields"),           // Linked Fields (not translated in Java source)
    Some("Hide Bar"),                // Hide Bar (not translated in Java source)
    None,                            // -
    Some("Skapad"),                  // Created
    Some("Start4"),                  // Start4 (not translated in Java source)
    Some("Finish4"),                 // Finish4 (not translated in Java source)
    Some("Start5"),                  // Start5 (not translated in Java source)
    Some("Finish5"),                 // Finish5 (not translated in Java source)
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    None,                            // -
    Some("Confirmed"),               // Confirmed (not translated in Java source)
    Some("Update Needed"),           // Update Needed (not translated in Java source)
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
    Some("Resume"),                  // Resume (not translated in Java source)
];

/// Name table for the Swedish resource key `RESOURCE_NAMES` (indices correspond to MPX resource field numbers; end-of-line comments give the English default name).
///
/// Corresponds to Java: `LocaleData_sv.RESOURCE_NAMES_DATA`.
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
    Some("Notes"),              // Notes (not translated in Java source)
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
    Some("Remaining Work"),     // Remaining Work (not translated in Java source)
    Some("Övertidsarbete"),     // Overtime Work
    Some("Work Variance"),      // Work Variance (not translated in Java source)
    Some("% Work Complete"),    // % Work Complete (not translated in Java source)
    None,                       // -
    None,                       // -
    None,                       // -
    Some("Kostnad"),            // Cost
    Some("Originalkostnad"),    // Baseline Cost
    Some("Verklig kostnad"),    // Actual Cost
    Some("Remaining Cost"),     // Remaining Cost (not translated in Java source)
    Some("Cost Variance"),      // Cost Variance (not translated in Java source)
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
    Some("Overallocated"),      // Overallocated (not translated in Java source)
    Some("Peak"),               // Peak (not translated in Java source)
    Some("Base Calendar"),      // Base Calendar (not translated in Java source)
    Some("Eget ID"),            // Unique ID
    Some("Objects"),            // Objects (not translated in Java source)
    Some("Linked Fields"),      // Linked Fields (not translated in Java source)
];

/// Corresponds to resource key `FILE_DELIMITER`
pub const FILE_DELIMITER: char = ';';
/// Corresponds to resource key `FILE_VERSION`
pub const FILE_VERSION: &'static str = "4,0";
/// Corresponds to resource key `YES`
pub const YES: &'static str = "Ja";
/// Corresponds to resource key `NO`
pub const NO: &'static str = "Nej";
/// Corresponds to resource key `CURRENCY_SYMBOL`
pub const CURRENCY_SYMBOL: &'static str = "kr";
/// Corresponds to resource key `CURRENCY_SYMBOL_POSITION`
pub const CURRENCY_SYMBOL_POSITION: CurrencySymbolPosition = CurrencySymbolPosition::AfterWithSpace;
/// Corresponds to resource key `CURRENCY_THOUSANDS_SEPARATOR` (a single space on the Java side)
pub const CURRENCY_THOUSANDS_SEPARATOR: char = ' ';
/// Corresponds to resource key `CURRENCY_DECIMAL_SEPARATOR`
pub const CURRENCY_DECIMAL_SEPARATOR: char = ',';
/// Corresponds to resource key `DATE_ORDER`
pub const DATE_ORDER: DateOrder = DateOrder::Ymd;
/// Corresponds to resource key `TIME_FORMAT`
pub const TIME_FORMAT: ProjectTimeFormat = ProjectTimeFormat::TwentyFourHour;
/// Corresponds to resource key `DATE_SEPARATOR`
pub const DATE_SEPARATOR: char = '-';
/// Corresponds to resource key `TIME_SEPARATOR`
pub const TIME_SEPARATOR: char = ':';
/// Corresponds to resource key `AM_TEXT`
pub const AM_TEXT: &'static str = "";
/// Corresponds to resource key `PM_TEXT`
pub const PM_TEXT: &'static str = "";
/// Corresponds to resource key `DATE_FORMAT`
pub const DATE_FORMAT: ProjectDateFormat = ProjectDateFormat::EeeDdMmYy;

/// Contents of the Swedish resource bundle, corresponding to Java `LocaleData_sv.RESOURCE_DATA`
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
        // Uncovered keys fall back to English
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
