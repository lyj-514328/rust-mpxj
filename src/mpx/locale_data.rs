//! Corresponds to Java: `org/mpxj/mpx/LocaleData.java`
//!
//! # Porting trade-offs (`ResourceBundle` / `ListResourceBundle` / `java.util.Locale`)
//!
//! Java's whole family of classes here are subclasses of `ResourceBundle`: `LocaleData` is the base
//! class and `RESOURCE_DATA` (`Object[][]`) holds the English defaults; `LocaleData_en/de/es/fr/it/pt/ru/sv/zh`
//! are the locale variants, whose `getContents()` lists only the keys that locale covers.
//! `ResourceBundle.getBundle` picks a variant by the requested locale, and keys missing from the variant
//! automatically fall back to the base class. The Rust standard library has no `ResourceBundle` / `Locale`,
//! so this module handles it as follows:
//!
//! - The locale parameter uses [`MpxLocale`] (a substitute for `java.util.Locale`, see `locale_utility`);
//! - The base-class defaults are this module's [`RESOURCE_DATA`], and the array constants keep the Java field names
//!   ([`TASK_NAMES_DATA`] / [`RESOURCE_NAMES_DATA`] / [`TIME_UNITS_ARRAY_DATA`] /
//!   [`ACCRUE_TYPES_DATA`] / [`RELATION_TYPES_DATA`] / [`PRIORITY_TYPES_DATA`] /
//!   [`CONSTRAINT_TYPES_DATA`]);
//! - Each locale variant writes the keys it actually covers as `pub const` values in its `locale_data_XX` module
//!   (scalar constants are named after the keys in Java `RESOURCE_DATA`, e.g. `locale_data_de::FILE_DELIMITER`)
//!   and aggregates them with [`LocaleOverrides`] (corresponding to the Java variant's `getContents()`);
//!   uncovered keys are `None` and fall back to [`RESOURCE_DATA`] when looked up, equivalent to
//!   `ResourceBundle`'s parent-bundle fallback;
//! - Java's `getString` / `getStringArray` / `getStringArrays` / `getMap` / `getInteger` / `getChar`
//!   map one-to-one to same-named `get_*` functions, with keys given as this type's `pub const` string
//!   constants, used as in Java's `LocaleData.ACCRUE_TYPES`; `getObject` relies on the caller casting the
//!   type, so Rust replaces it with the typed [`LocaleData::get_code_page`],
//!   [`LocaleData::get_currency_symbol_position`], [`LocaleData::get_date_order`],
//!   [`LocaleData::get_time_format`], [`LocaleData::get_date_format`];
//! - Java's `String[]` elements may be `null` (unnamed fields in `TASK_NAMES` / `RESOURCE_NAMES`), so
//!   `get_string_array` uniformly returns `&[Option<&str>]`; `TIME_UNITS_ARRAY` is a `String[][]` whose
//!   inner arrays have no `null`, returned by `get_string_arrays` as `&[&[&str]]`;
//! - `TIME_UNITS_MAP` is expanded in Java by a static block from `TIME_UNITS_ARRAY_DATA` into a `HashMap`;
//!   Rust expands it on demand via [`LocaleData::get_map`] (there is no `TIME_UNITS_MAP_DATA` constant in
//!   this module any more).
//!
//! Skipped and noted:
//! - `ResourceBundle`'s protocol methods `getContents` / `handleGetObject` / `getKeys` / `setParent` /
//!   `keySet`: replaced by this module's constant tables plus [`LocaleOverrides`]' fallback logic;
//! - `ListResourceBundle` and `java.util.Locale`: replaced by [`LocaleOverrides`] and [`MpxLocale`] respectively;
//! - `Serializable` (provided by `ResourceBundle`): not ported.
//!
//! Skipped Java members: none.
//!
//! # Inconsistencies preserved from the upstream data (copied verbatim from the Java source)
//!
//! - The `TIME_UNITS_ARRAY_DATA` literals of the non-English variants put "years" at index 5 and "percent"
//!   at index 6, whereas `TimeUnit`'s values are `PERCENT(5)` / `YEARS(6)` (the English base class assigns
//!   by `getValue()`, so index 5 is `"%"` and index 6 is `"y"`). The differing order is an upstream
//!   pre-existing condition; this port copies each item verbatim and does not "fix" it.
//! - `LocaleData_ru.RESOURCE_NAMES_DATA` has only 51 entries (it lacks `Overtime Work`, shifting the whole
//!   name table after index 24), also copied verbatim.

use std::collections::HashMap;

use crate::code_page::CodePage;
use crate::currency_symbol_position::CurrencySymbolPosition;
use crate::date_order::DateOrder;
use crate::mpx::locale_data_de;
use crate::mpx::locale_data_en;
use crate::mpx::locale_data_es;
use crate::mpx::locale_data_fr;
use crate::mpx::locale_data_it;
use crate::mpx::locale_data_pt;
use crate::mpx::locale_data_ru;
use crate::mpx::locale_data_sv;
use crate::mpx::locale_data_zh;
use crate::mpx::locale_utility::MpxLocale;
use crate::project_date_format::ProjectDateFormat;
use crate::project_time_format::ProjectTimeFormat;

/// All resources for one locale, i.e. the key/value set readable after Java `ResourceBundle` resolves.
///
/// Corresponds to Java: the base class `LocaleData.RESOURCE_DATA` (key/value pairs in `Object[][]` form);
/// typed fields are used here instead, replacing Java's `Object` casts with the field types.
#[derive(Debug, Clone, Copy)]
pub struct LocaleResources {
    /// Corresponds to resource key `FILE_DELIMITER`
    pub file_delimiter: char,
    /// Corresponds to resource key `PROGRAM_NAME`
    pub program_name: &'static str,
    /// Corresponds to resource key `FILE_VERSION`
    pub file_version: &'static str,
    /// Corresponds to resource key `CODE_PAGE`
    pub code_page: CodePage,
    /// Corresponds to resource key `CURRENCY_SYMBOL`
    pub currency_symbol: &'static str,
    /// Corresponds to resource key `CURRENCY_SYMBOL_POSITION`
    pub currency_symbol_position: CurrencySymbolPosition,
    /// Corresponds to resource key `CURRENCY_DIGITS`
    pub currency_digits: i32,
    /// Corresponds to resource key `CURRENCY_THOUSANDS_SEPARATOR`
    pub currency_thousands_separator: char,
    /// Corresponds to resource key `CURRENCY_DECIMAL_SEPARATOR`
    pub currency_decimal_separator: char,
    /// Corresponds to resource key `DATE_ORDER`
    pub date_order: DateOrder,
    /// Corresponds to resource key `TIME_FORMAT`
    pub time_format: ProjectTimeFormat,
    /// Corresponds to resource key `DEFAULT_START_TIME` (unit: minutes since midnight)
    pub default_start_time: i32,
    /// Corresponds to resource key `DATE_SEPARATOR`
    pub date_separator: char,
    /// Corresponds to resource key `TIME_SEPARATOR`
    pub time_separator: char,
    /// Corresponds to resource key `AM_TEXT`
    pub am_text: &'static str,
    /// Corresponds to resource key `PM_TEXT`
    pub pm_text: &'static str,
    /// Corresponds to resource key `DATE_FORMAT`
    pub date_format: ProjectDateFormat,
    /// Corresponds to resource key `BAR_TEXT_DATE_FORMAT`
    pub bar_text_date_format: i32,
    /// Corresponds to resource key `NA`
    pub na: &'static str,
    /// Corresponds to resource key `YES`
    pub yes: &'static str,
    /// Corresponds to resource key `NO`
    pub no: &'static str,
    /// Corresponds to resource key `TIME_UNITS_ARRAY` (the index is the int value of `TimeUnit`; the inner array holds that unit's aliases)
    pub time_units_array: &'static [&'static [&'static str]],
    /// Corresponds to resource key `ACCRUE_TYPES`
    pub accrue_types: &'static [Option<&'static str>],
    /// Corresponds to resource key `RELATION_TYPES`
    pub relation_types: &'static [Option<&'static str>],
    /// Corresponds to resource key `PRIORITY_TYPES`
    pub priority_types: &'static [Option<&'static str>],
    /// Corresponds to resource key `CONSTRAINT_TYPES`
    pub constraint_types: &'static [Option<&'static str>],
    /// Corresponds to resource key `TASK_NAMES` (indices correspond to MPX task field numbers; `None` means the field has no name)
    pub task_names: &'static [Option<&'static str>],
    /// Corresponds to resource key `RESOURCE_NAMES` (indices correspond to MPX resource field numbers; `None` means the field has no name)
    pub resource_names: &'static [Option<&'static str>],
}

/// English (default) time unit alias table.
///
/// Corresponds to Java: `LocaleData.TIME_UNITS_ARRAY_DATA` (Java assigns by the index `TimeUnit.getValue()`,
/// so here index 5 is `PERCENT` and index 6 is `YEARS`).
pub const TIME_UNITS_ARRAY_DATA: [&'static [&'static str]; 14] = [
    &["m", "mins"],        // TimeUnit.MINUTES(0)
    &["h", "hours"],       // TimeUnit.HOURS(1)
    &["d", "days"],        // TimeUnit.DAYS(2)
    &["w", "wk", "weeks"], // TimeUnit.WEEKS(3)
    &["mon", "months"],    // TimeUnit.MONTHS(4)
    &["%"],                // TimeUnit.PERCENT(5)
    &["y", "years"],       // TimeUnit.YEARS(6)
    &["em"],               // TimeUnit.ELAPSED_MINUTES(7)
    &["eh"],               // TimeUnit.ELAPSED_HOURS(8)
    &["ed"],               // TimeUnit.ELAPSED_DAYS(9)
    &["ew"],               // TimeUnit.ELAPSED_WEEKS(10)
    &["emon"],             // TimeUnit.ELAPSED_MONTHS(11)
    &["ey"],               // TimeUnit.ELAPSED_YEARS(12)
    &["e%"],               // TimeUnit.ELAPSED_PERCENT(13)
];

/// English (default) resource key `ACCRUE_TYPES` name table (index + 1 is the int value of `AccrueType`).
///
/// Corresponds to Java: `LocaleData.ACCRUE_TYPES_DATA` (no `null` elements; `Option` is used to unify
/// the return type of `get_string_array`).
pub const ACCRUE_TYPES_DATA: [Option<&'static str>; 3] =
    [Some("Start"), Some("End"), Some("Prorated")];

/// English (default) resource key `RELATION_TYPES` name table (the index is the int value of `RelationType`).
///
/// Corresponds to Java: `LocaleData.RELATION_TYPES_DATA`.
pub const RELATION_TYPES_DATA: [Option<&'static str>; 4] =
    [Some("FF"), Some("FS"), Some("SF"), Some("SS")];

/// English (default) resource key `PRIORITY_TYPES` name table (index + 1, times 100, is the `Priority` value).
///
/// Corresponds to Java: `LocaleData.PRIORITY_TYPES_DATA`.
pub const PRIORITY_TYPES_DATA: [Option<&'static str>; 10] = [
    Some("Lowest"),
    Some("Very Low"),
    Some("Lower"),
    Some("Low"),
    Some("Medium"),
    Some("High"),
    Some("Higher"),
    Some("Very High"),
    Some("Highest"),
    Some("Do Not Level"),
];

/// English (default) resource key `CONSTRAINT_TYPES` name table (the index is the int value of `ConstraintType`).
///
/// Corresponds to Java: `LocaleData.CONSTRAINT_TYPES_DATA`.
pub const CONSTRAINT_TYPES_DATA: [Option<&'static str>; 8] = [
    Some("As Soon As Possible"),
    Some("As Late As Possible"),
    Some("Must Start On"),
    Some("Must Finish On"),
    Some("Start No Earlier Than"),
    Some("Start No Later Than"),
    Some("Finish No Earlier Than"),
    Some("Finish No Later Than"),
];

/// English (default) resource key `TASK_NAMES` name table, with indices corresponding one-to-one to MPX task field numbers.
///
/// Corresponds to Java: `LocaleData.TASK_NAMES_DATA` (a `null` element means the field has no name).
pub const TASK_NAMES_DATA: [Option<&'static str>; 153] = [
    None,                              // 0
    Some("Name"),                      // 1
    Some("WBS"),                       // 2
    Some("Outline Level"),             // 3
    Some("Text1"),                     // 4
    Some("Text2"),                     // 5
    Some("Text3"),                     // 6
    Some("Text4"),                     // 7
    Some("Text5"),                     // 8
    Some("Text6"),                     // 9
    Some("Text7"),                     // 10
    Some("Text8"),                     // 11
    Some("Text9"),                     // 12
    Some("Text10"),                    // 13
    Some("Notes"),                     // 14
    Some("Contact"),                   // 15
    Some("Resource Group"),            // 16
    None,                              // 17
    None,                              // 18
    None,                              // 19
    Some("Work"),                      // 20
    Some("Baseline Work"),             // 21
    Some("Actual Work"),               // 22
    Some("Remaining Work"),            // 23
    Some("Work Variance"),             // 24
    Some("% Work Complete"),           // 25
    None,                              // 26
    None,                              // 27
    None,                              // 28
    None,                              // 29
    Some("Cost"),                      // 30
    Some("Baseline Cost"),             // 31
    Some("Actual Cost"),               // 32
    Some("Remaining Cost"),            // 33
    Some("Cost Variance"),             // 34
    Some("Fixed Cost"),                // 35
    Some("Cost1"),                     // 36
    Some("Cost2"),                     // 37
    Some("Cost3"),                     // 38
    None,                              // 39
    Some("Duration"),                  // 40
    Some("Baseline Duration"),         // 41
    Some("Actual Duration"),           // 42
    Some("Remaining Duration"),        // 43
    Some("% Complete"),                // 44
    Some("Duration Variance"),         // 45
    Some("Duration1"),                 // 46
    Some("Duration2"),                 // 47
    Some("Duration3"),                 // 48
    None,                              // 49
    Some("Start"),                     // 50
    Some("Finish"),                    // 51
    Some("Early Start"),               // 52
    Some("Early Finish"),              // 53
    Some("Late Start"),                // 54
    Some("Late Finish"),               // 55
    Some("Baseline Start"),            // 56
    Some("Baseline Finish"),           // 57
    Some("Actual Start"),              // 58
    Some("Actual Finish"),             // 59
    Some("Start1"),                    // 60
    Some("Finish1"),                   // 61
    Some("Start2"),                    // 62
    Some("Finish2"),                   // 63
    Some("Start3"),                    // 64
    Some("Finish3"),                   // 65
    Some("Start Variance"),            // 66
    Some("Finish Variance"),           // 67
    Some("Constraint Date"),           // 68
    None,                              // 69
    Some("Predecessors"),              // 70
    Some("Successors"),                // 71
    Some("Resource Names"),            // 72
    Some("Resource Initials"),         // 73
    Some("Unique ID Predecessors"),    // 74
    Some("Unique ID Successors"),      // 75
    None,                              // 76
    None,                              // 77
    None,                              // 78
    None,                              // 79
    Some("Fixed"),                     // 80
    Some("Milestone"),                 // 81
    Some("Critical"),                  // 82
    Some("Marked"),                    // 83
    Some("Rollup"),                    // 84
    Some("BCWS"),                      // 85
    Some("BCWP"),                      // 86
    Some("SV"),                        // 87
    Some("CV"),                        // 88
    None,                              // 89
    Some("ID"),                        // 90
    Some("Constraint Type"),           // 91
    Some("Delay"),                     // 92
    Some("Free Slack"),                // 93
    Some("Total Slack"),               // 94
    Some("Priority"),                  // 95
    Some("Subproject File"),           // 96
    Some("Project"),                   // 97
    Some("Unique ID"),                 // 98
    Some("Outline Number"),            // 99
    None,                              // 100
    None,                              // 101
    None,                              // 102
    None,                              // 103
    None,                              // 104
    None,                              // 105
    None,                              // 106
    None,                              // 107
    None,                              // 108
    None,                              // 109
    Some("Flag1"),                     // 110
    Some("Flag2"),                     // 111
    Some("Flag3"),                     // 112
    Some("Flag4"),                     // 113
    Some("Flag5"),                     // 114
    Some("Flag6"),                     // 115
    Some("Flag7"),                     // 116
    Some("Flag8"),                     // 117
    Some("Flag9"),                     // 118
    Some("Flag10"),                    // 119
    Some("Summary"),                   // 120
    Some("Objects"),                   // 121
    Some("Linked Fields"),             // 122
    Some("Hide Bar"),                  // 123
    None,                              // 124
    Some("Created"),                   // 125
    Some("Start4"),                    // 126
    Some("Finish4"),                   // 127
    Some("Start5"),                    // 128
    Some("Finish5"),                   // 129
    None,                              // 130
    None,                              // 131
    None,                              // 132
    None,                              // 133
    None,                              // 134
    Some("Confirmed"),                 // 135
    Some("Update Needed"),             // 136
    None,                              // 137
    None,                              // 138
    None,                              // 139
    Some("Number1"),                   // 140
    Some("Number2"),                   // 141
    Some("Number3"),                   // 142
    Some("Number4"),                   // 143
    Some("Number5"),                   // 144
    None,                              // 145
    None,                              // 146
    None,                              // 147
    None,                              // 148
    None,                              // 149
    Some("Stop"),                      // 150
    Some("Resume No Earlier Than"),    // 151
    Some("Resume"),                    // 152
];

/// English (default) resource key `RESOURCE_NAMES` name table, with indices corresponding one-to-one to MPX resource field numbers.
///
/// Corresponds to Java: `LocaleData.RESOURCE_NAMES_DATA` (a `null` element means the field has no name).
pub const RESOURCE_NAMES_DATA: [Option<&'static str>; 52] = [
    None,                          // 0
    Some("Name"),                  // 1
    Some("Initials"),              // 2
    Some("Group"),                 // 3
    Some("Code"),                  // 4
    Some("Text1"),                 // 5
    Some("Text2"),                 // 6
    Some("Text3"),                 // 7
    Some("Text4"),                 // 8
    Some("Text5"),                 // 9
    Some("Notes"),                 // 10
    Some("Email Address"),         // 11
    None,                          // 12
    None,                          // 13
    None,                          // 14
    None,                          // 15
    None,                          // 16
    None,                          // 17
    None,                          // 18
    None,                          // 19
    Some("Work"),                  // 20
    Some("Baseline Work"),         // 21
    Some("Actual Work"),           // 22
    Some("Remaining Work"),        // 23
    Some("Overtime Work"),         // 24
    Some("Work Variance"),         // 25
    Some("% Work Complete"),       // 26
    None,                          // 27
    None,                          // 28
    None,                          // 29
    Some("Cost"),                  // 30
    Some("Baseline Cost"),         // 31
    Some("Actual Cost"),           // 32
    Some("Remaining Cost"),        // 33
    Some("Cost Variance"),         // 34
    None,                          // 35
    None,                          // 36
    None,                          // 37
    None,                          // 38
    None,                          // 39
    Some("ID"),                    // 40
    Some("Max Units"),             // 41
    Some("Standard Rate"),         // 42
    Some("Overtime Rate"),         // 43
    Some("Cost Per Use"),          // 44
    Some("Accrue At"),             // 45
    Some("Overallocated"),         // 46
    Some("Peak"),                  // 47
    Some("Base Calendar"),         // 48
    Some("Unique ID"),             // 49
    Some("Objects"),               // 50
    Some("Linked Fields"),         // 51
];

/// English (default) resources, corresponding to Java `LocaleData.RESOURCE_DATA` (the return value of `getContents()`).
///
/// Java's `LocaleData_en` resource bundle is empty; English uses these defaults entirely.
pub static RESOURCE_DATA: LocaleResources = LocaleResources {
    file_delimiter: ',',
    program_name: "Microsoft Project for Windows",
    file_version: "4.0",
    code_page: CodePage::Ansi,
    currency_symbol: "$",
    currency_symbol_position: CurrencySymbolPosition::Before,
    currency_digits: 2,
    currency_thousands_separator: ',',
    currency_decimal_separator: '.',
    date_order: DateOrder::Dmy,
    time_format: ProjectTimeFormat::TwelveHour,
    default_start_time: 480,
    date_separator: '/',
    time_separator: ':',
    am_text: "am",
    pm_text: "pm",
    date_format: ProjectDateFormat::DdMmYyyy,
    bar_text_date_format: 0,
    na: "NA",
    yes: "Yes",
    no: "No",
    time_units_array: &TIME_UNITS_ARRAY_DATA,
    accrue_types: &ACCRUE_TYPES_DATA,
    relation_types: &RELATION_TYPES_DATA,
    priority_types: &PRIORITY_TYPES_DATA,
    constraint_types: &CONSTRAINT_TYPES_DATA,
    task_names: &TASK_NAMES_DATA,
    resource_names: &RESOURCE_NAMES_DATA,
};

/// The resource keys covered by one locale variant, corresponding to Java `LocaleData_XX`'s `RESOURCE_DATA`
/// (i.e. the return value of `getContents()`).
///
/// A field being `None` means the variant does not cover that key and it falls back to [`RESOURCE_DATA`]
/// when looked up (equivalent to `ResourceBundle`'s parent-bundle fallback).
#[derive(Debug, Clone, Copy)]
pub struct LocaleOverrides {
    /// Corresponds to resource key `FILE_DELIMITER`
    pub file_delimiter: Option<char>,
    /// Corresponds to resource key `PROGRAM_NAME`
    pub program_name: Option<&'static str>,
    /// Corresponds to resource key `FILE_VERSION`
    pub file_version: Option<&'static str>,
    /// Corresponds to resource key `CODE_PAGE`
    pub code_page: Option<CodePage>,
    /// Corresponds to resource key `CURRENCY_SYMBOL`
    pub currency_symbol: Option<&'static str>,
    /// Corresponds to resource key `CURRENCY_SYMBOL_POSITION`
    pub currency_symbol_position: Option<CurrencySymbolPosition>,
    /// Corresponds to resource key `CURRENCY_DIGITS`
    pub currency_digits: Option<i32>,
    /// Corresponds to resource key `CURRENCY_THOUSANDS_SEPARATOR`
    pub currency_thousands_separator: Option<char>,
    /// Corresponds to resource key `CURRENCY_DECIMAL_SEPARATOR`
    pub currency_decimal_separator: Option<char>,
    /// Corresponds to resource key `DATE_ORDER`
    pub date_order: Option<DateOrder>,
    /// Corresponds to resource key `TIME_FORMAT`
    pub time_format: Option<ProjectTimeFormat>,
    /// Corresponds to resource key `DEFAULT_START_TIME`
    pub default_start_time: Option<i32>,
    /// Corresponds to resource key `DATE_SEPARATOR`
    pub date_separator: Option<char>,
    /// Corresponds to resource key `TIME_SEPARATOR`
    pub time_separator: Option<char>,
    /// Corresponds to resource key `AM_TEXT`
    pub am_text: Option<&'static str>,
    /// Corresponds to resource key `PM_TEXT`
    pub pm_text: Option<&'static str>,
    /// Corresponds to resource key `DATE_FORMAT`
    pub date_format: Option<ProjectDateFormat>,
    /// Corresponds to resource key `BAR_TEXT_DATE_FORMAT`
    pub bar_text_date_format: Option<i32>,
    /// Corresponds to resource key `NA`
    pub na: Option<&'static str>,
    /// Corresponds to resource key `YES`
    pub yes: Option<&'static str>,
    /// Corresponds to resource key `NO`
    pub no: Option<&'static str>,
    /// Corresponds to resource key `TIME_UNITS_ARRAY`
    pub time_units_array: Option<&'static [&'static [&'static str]]>,
    /// Corresponds to resource key `ACCRUE_TYPES`
    pub accrue_types: Option<&'static [Option<&'static str>]>,
    /// Corresponds to resource key `RELATION_TYPES`
    pub relation_types: Option<&'static [Option<&'static str>]>,
    /// Corresponds to resource key `PRIORITY_TYPES`
    pub priority_types: Option<&'static [Option<&'static str>]>,
    /// Corresponds to resource key `CONSTRAINT_TYPES`
    pub constraint_types: Option<&'static [Option<&'static str>]>,
    /// Corresponds to resource key `TASK_NAMES`
    pub task_names: Option<&'static [Option<&'static str>]>,
    /// Corresponds to resource key `RESOURCE_NAMES`
    pub resource_names: Option<&'static [Option<&'static str>]>,
}

impl LocaleOverrides {
    /// All keys uncovered, corresponding to Java `LocaleData_en.RESOURCE_DATA = {}`.
    pub const EMPTY: LocaleOverrides = LocaleOverrides {
        file_delimiter: None,
        program_name: None,
        file_version: None,
        code_page: None,
        currency_symbol: None,
        currency_symbol_position: None,
        currency_digits: None,
        currency_thousands_separator: None,
        currency_decimal_separator: None,
        date_order: None,
        time_format: None,
        default_start_time: None,
        date_separator: None,
        time_separator: None,
        am_text: None,
        pm_text: None,
        date_format: None,
        bar_text_date_format: None,
        na: None,
        yes: None,
        no: None,
        time_units_array: None,
        accrue_types: None,
        relation_types: None,
        priority_types: None,
        constraint_types: None,
        task_names: None,
        resource_names: None,
    };
}

/// Resource bundle handling utility and the English default resources.
///
/// Corresponds to Java: `public final class LocaleData extends ListResourceBundle`
/// (the private constructor forbids instantiation; Rust expresses it with a fieldless type).
pub struct LocaleData;

impl LocaleData {
    /// Corresponds to Java: `public static final String FILE_DELIMITER`
    pub const FILE_DELIMITER: &'static str = "FILE_DELIMITER";
    /// Corresponds to Java: `public static final String PROGRAM_NAME`
    pub const PROGRAM_NAME: &'static str = "PROGRAM_NAME";
    /// Corresponds to Java: `public static final String FILE_VERSION`
    pub const FILE_VERSION: &'static str = "FILE_VERSION";
    /// Corresponds to Java: `public static final String CODE_PAGE`
    pub const CODE_PAGE: &'static str = "CODE_PAGE";

    /// Corresponds to Java: `public static final String YES`
    pub const YES: &'static str = "YES";
    /// Corresponds to Java: `public static final String NO`
    pub const NO: &'static str = "NO";

    /// Corresponds to Java: `public static final String CURRENCY_SYMBOL`
    pub const CURRENCY_SYMBOL: &'static str = "CURRENCY_SYMBOL";
    /// Corresponds to Java: `public static final String CURRENCY_SYMBOL_POSITION`
    pub const CURRENCY_SYMBOL_POSITION: &'static str = "CURRENCY_SYMBOL_POSITION";
    /// Corresponds to Java: `public static final String CURRENCY_DIGITS`
    pub const CURRENCY_DIGITS: &'static str = "CURRENCY_DIGITS";
    /// Corresponds to Java: `public static final String CURRENCY_THOUSANDS_SEPARATOR`
    pub const CURRENCY_THOUSANDS_SEPARATOR: &'static str = "CURRENCY_THOUSANDS_SEPARATOR";
    /// Corresponds to Java: `public static final String CURRENCY_DECIMAL_SEPARATOR`
    pub const CURRENCY_DECIMAL_SEPARATOR: &'static str = "CURRENCY_DECIMAL_SEPARATOR";

    /// Corresponds to Java: `public static final String DATE_ORDER`
    pub const DATE_ORDER: &'static str = "DATE_ORDER";
    /// Corresponds to Java: `public static final String TIME_FORMAT`
    pub const TIME_FORMAT: &'static str = "TIME_FORMAT";
    /// Corresponds to Java: `public static final String DEFAULT_START_TIME`
    pub const DEFAULT_START_TIME: &'static str = "DEFAULT_START_TIME";
    /// Corresponds to Java: `public static final String DATE_SEPARATOR`
    pub const DATE_SEPARATOR: &'static str = "DATE_SEPARATOR";
    /// Corresponds to Java: `public static final String TIME_SEPARATOR`
    pub const TIME_SEPARATOR: &'static str = "TIME_SEPARATOR";
    /// Corresponds to Java: `public static final String AM_TEXT`
    pub const AM_TEXT: &'static str = "AM_TEXT";
    /// Corresponds to Java: `public static final String PM_TEXT`
    pub const PM_TEXT: &'static str = "PM_TEXT";
    /// Corresponds to Java: `public static final String DATE_FORMAT`
    pub const DATE_FORMAT: &'static str = "DATE_FORMAT";
    /// Corresponds to Java: `public static final String BAR_TEXT_DATE_FORMAT`
    pub const BAR_TEXT_DATE_FORMAT: &'static str = "BAR_TEXT_DATE_FORMAT";
    /// Corresponds to Java: `public static final String NA`
    pub const NA: &'static str = "NA";

    /// Corresponds to Java: `public static final String TIME_UNITS_ARRAY`
    pub const TIME_UNITS_ARRAY: &'static str = "TIME_UNITS_ARRAY";
    /// Corresponds to Java: `public static final String TIME_UNITS_MAP`
    pub const TIME_UNITS_MAP: &'static str = "TIME_UNITS_MAP";

    /// Corresponds to Java: `public static final String ACCRUE_TYPES`
    pub const ACCRUE_TYPES: &'static str = "ACCRUE_TYPES";
    /// Corresponds to Java: `public static final String RELATION_TYPES`
    pub const RELATION_TYPES: &'static str = "RELATION_TYPES";
    /// Corresponds to Java: `public static final String PRIORITY_TYPES`
    pub const PRIORITY_TYPES: &'static str = "PRIORITY_TYPES";
    /// Corresponds to Java: `public static final String CONSTRAINT_TYPES`
    pub const CONSTRAINT_TYPES: &'static str = "CONSTRAINT_TYPES";

    /// Corresponds to Java: `public static final String TASK_NAMES`
    pub const TASK_NAMES: &'static str = "TASK_NAMES";
    /// Corresponds to Java: `public static final String RESOURCE_NAMES`
    pub const RESOURCE_NAMES: &'static str = "RESOURCE_NAMES";

    /// Get a string resource.
    ///
    /// Corresponds to Java: `public static final String getString(Locale locale, String key)`
    ///
    /// Java throws `MissingResourceException` when the key does not exist and `ClassCastException` when
    /// the value is not a string; Rust uniformly returns `None`.
    pub fn get_string(locale: MpxLocale, key: &str) -> Option<&'static str> {
        let overrides = overrides(locale);

        match key {
            Self::PROGRAM_NAME => Some(overrides.program_name.unwrap_or(RESOURCE_DATA.program_name)),
            Self::FILE_VERSION => Some(overrides.file_version.unwrap_or(RESOURCE_DATA.file_version)),
            Self::CURRENCY_SYMBOL => {
                Some(overrides.currency_symbol.unwrap_or(RESOURCE_DATA.currency_symbol))
            }
            Self::AM_TEXT => Some(overrides.am_text.unwrap_or(RESOURCE_DATA.am_text)),
            Self::PM_TEXT => Some(overrides.pm_text.unwrap_or(RESOURCE_DATA.pm_text)),
            Self::NA => Some(overrides.na.unwrap_or(RESOURCE_DATA.na)),
            Self::YES => Some(overrides.yes.unwrap_or(RESOURCE_DATA.yes)),
            Self::NO => Some(overrides.no.unwrap_or(RESOURCE_DATA.no)),
            _ => None,
        }
    }

    /// Get a string array resource (elements may be `None`, corresponding to `null` in Java's `String[]`).
    ///
    /// Corresponds to Java: `public static final String[] getStringArray(Locale locale, String key)`
    ///
    /// `TIME_UNITS_ARRAY` is a `String[][]`; Java throws `ClassCastException` if this method is used to
    /// fetch it, and Rust returns `None` (use [`LocaleData::get_string_arrays`] instead).
    pub fn get_string_array(
        locale: MpxLocale,
        key: &str,
    ) -> Option<&'static [Option<&'static str>]> {
        let overrides = overrides(locale);

        match key {
            Self::ACCRUE_TYPES => Some(overrides.accrue_types.unwrap_or(RESOURCE_DATA.accrue_types)),
            Self::RELATION_TYPES => {
                Some(overrides.relation_types.unwrap_or(RESOURCE_DATA.relation_types))
            }
            Self::PRIORITY_TYPES => {
                Some(overrides.priority_types.unwrap_or(RESOURCE_DATA.priority_types))
            }
            Self::CONSTRAINT_TYPES => {
                Some(overrides.constraint_types.unwrap_or(RESOURCE_DATA.constraint_types))
            }
            Self::TASK_NAMES => Some(overrides.task_names.unwrap_or(RESOURCE_DATA.task_names)),
            Self::RESOURCE_NAMES => {
                Some(overrides.resource_names.unwrap_or(RESOURCE_DATA.resource_names))
            }
            _ => None,
        }
    }

    /// Get a two-dimensional string array resource.
    ///
    /// Corresponds to Java: `public static final String[][] getStringArrays(Locale locale, String key)`
    ///
    /// Only `TIME_UNITS_ARRAY` is of this kind; for other keys Java throws `ClassCastException` and Rust
    /// returns `None`.
    pub fn get_string_arrays(
        locale: MpxLocale,
        key: &str,
    ) -> Option<&'static [&'static [&'static str]]> {
        match key {
            Self::TIME_UNITS_ARRAY => Some(
                overrides(locale)
                    .time_units_array
                    .unwrap_or(RESOURCE_DATA.time_units_array),
            ),
            _ => None,
        }
    }

    /// Get a Map resource, i.e. the "time unit name (lowercase) → int value of `TimeUnit`" mapping.
    ///
    /// Corresponds to Java: `public static final Map getMap(Locale locale, String key)`
    ///
    /// Java's `TIME_UNITS_MAP` is expanded by a static block from `TIME_UNITS_ARRAY_DATA`, with later
    /// entries overwriting earlier ones on duplicate names; Rust expands it in the same order on each call
    /// (only this key is a Map; for other keys Java throws `ClassCastException` and Rust returns `None`).
    pub fn get_map(locale: MpxLocale, key: &str) -> Option<HashMap<&'static str, i32>> {
        let unit_names = match key {
            Self::TIME_UNITS_MAP => Self::get_string_arrays(locale, Self::TIME_UNITS_ARRAY)?,
            _ => return None,
        };

        let mut result = HashMap::new();
        for (value, names) in unit_names.iter().enumerate() {
            for name in names.iter() {
                result.insert(*name, value as i32);
            }
        }

        Some(result)
    }

    /// Get an integer resource.
    ///
    /// Corresponds to Java: `public static final Integer getInteger(Locale locale, String key)`
    ///
    /// Java throws `MissingResourceException` when the key does not exist and `ClassCastException` when
    /// the value is not an integer; Rust uniformly returns `None`.
    pub fn get_integer(locale: MpxLocale, key: &str) -> Option<i32> {
        let overrides = overrides(locale);

        match key {
            Self::CURRENCY_DIGITS => {
                Some(overrides.currency_digits.unwrap_or(RESOURCE_DATA.currency_digits))
            }
            Self::DEFAULT_START_TIME => Some(
                overrides
                    .default_start_time
                    .unwrap_or(RESOURCE_DATA.default_start_time),
            ),
            Self::BAR_TEXT_DATE_FORMAT => Some(
                overrides
                    .bar_text_date_format
                    .unwrap_or(RESOURCE_DATA.bar_text_date_format),
            ),
            _ => None,
        }
    }

    /// Get a single-character resource (Java takes the first character of a string).
    ///
    /// Corresponds to Java: `public static final char getChar(Locale locale, String key)`
    ///
    /// Java's `getChar` throws `ClassCastException` for keys whose value is not a string, and
    /// `StringIndexOutOfBoundsException` for an empty string; Rust uniformly returns `None` in these cases.
    pub fn get_char(locale: MpxLocale, key: &str) -> Option<char> {
        let overrides = overrides(locale);

        match key {
            Self::FILE_DELIMITER => {
                Some(overrides.file_delimiter.unwrap_or(RESOURCE_DATA.file_delimiter))
            }
            Self::CURRENCY_THOUSANDS_SEPARATOR => Some(
                overrides
                    .currency_thousands_separator
                    .unwrap_or(RESOURCE_DATA.currency_thousands_separator),
            ),
            Self::CURRENCY_DECIMAL_SEPARATOR => Some(
                overrides
                    .currency_decimal_separator
                    .unwrap_or(RESOURCE_DATA.currency_decimal_separator),
            ),
            Self::DATE_SEPARATOR => {
                Some(overrides.date_separator.unwrap_or(RESOURCE_DATA.date_separator))
            }
            Self::TIME_SEPARATOR => {
                Some(overrides.time_separator.unwrap_or(RESOURCE_DATA.time_separator))
            }
            _ => None,
        }
    }

    /// Get the code page.
    ///
    /// Corresponds to Java: `(CodePage) LocaleData.getObject(locale, LocaleData.CODE_PAGE)`
    pub fn get_code_page(locale: MpxLocale) -> CodePage {
        overrides(locale).code_page.unwrap_or(RESOURCE_DATA.code_page)
    }

    /// Get the currency symbol position.
    ///
    /// Corresponds to Java: `(CurrencySymbolPosition) LocaleData.getObject(locale, LocaleData.CURRENCY_SYMBOL_POSITION)`
    pub fn get_currency_symbol_position(locale: MpxLocale) -> CurrencySymbolPosition {
        overrides(locale)
            .currency_symbol_position
            .unwrap_or(RESOURCE_DATA.currency_symbol_position)
    }

    /// Get the date order.
    ///
    /// Corresponds to Java: `(DateOrder) LocaleData.getObject(locale, LocaleData.DATE_ORDER)`
    pub fn get_date_order(locale: MpxLocale) -> DateOrder {
        overrides(locale).date_order.unwrap_or(RESOURCE_DATA.date_order)
    }

    /// Get the time format.
    ///
    /// Corresponds to Java: `(ProjectTimeFormat) LocaleData.getObject(locale, LocaleData.TIME_FORMAT)`
    pub fn get_time_format(locale: MpxLocale) -> ProjectTimeFormat {
        overrides(locale).time_format.unwrap_or(RESOURCE_DATA.time_format)
    }

    /// Get the date format.
    ///
    /// Corresponds to Java: `(ProjectDateFormat) LocaleData.getObject(locale, LocaleData.DATE_FORMAT)`
    pub fn get_date_format(locale: MpxLocale) -> ProjectDateFormat {
        overrides(locale).date_format.unwrap_or(RESOURCE_DATA.date_format)
    }
}

/// Get the resource overrides for a locale.
///
/// Corresponds to Java: the process by which `ResourceBundle.getBundle(LocaleData.class.getName(), locale)`
/// selects a locale variant; `MpxLocale` has only the 9 values in `SUPPORTED_LOCALES`, so the `match` here is exhaustive.
fn overrides(locale: MpxLocale) -> &'static LocaleOverrides {
    match locale {
        MpxLocale::En => &locale_data_en::OVERRIDES,
        MpxLocale::De => &locale_data_de::OVERRIDES,
        MpxLocale::Es => &locale_data_es::OVERRIDES,
        MpxLocale::Fr => &locale_data_fr::OVERRIDES,
        MpxLocale::It => &locale_data_it::OVERRIDES,
        MpxLocale::Pt => &locale_data_pt::OVERRIDES,
        MpxLocale::Ru => &locale_data_ru::OVERRIDES,
        MpxLocale::Sv => &locale_data_sv::OVERRIDES,
        MpxLocale::Zh => &locale_data_zh::OVERRIDES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_arrays_have_java_lengths() {
        assert_eq!(TIME_UNITS_ARRAY_DATA.len(), 14);
        assert_eq!(ACCRUE_TYPES_DATA.len(), 3);
        assert_eq!(RELATION_TYPES_DATA.len(), 4);
        assert_eq!(PRIORITY_TYPES_DATA.len(), 10);
        assert_eq!(CONSTRAINT_TYPES_DATA.len(), 8);
        assert_eq!(TASK_NAMES_DATA.len(), 153);
        assert_eq!(RESOURCE_NAMES_DATA.len(), 52);
    }

    #[test]
    fn english_task_names_match_java() {
        assert_eq!(TASK_NAMES_DATA.first().copied(), Some(None));
        assert_eq!(TASK_NAMES_DATA[1], Some("Name"));
        assert_eq!(TASK_NAMES_DATA[2], Some("WBS"));
        assert_eq!(TASK_NAMES_DATA[16], Some("Resource Group"));
        assert_eq!(TASK_NAMES_DATA[17], None);
        assert_eq!(TASK_NAMES_DATA[68], Some("Constraint Date"));
        assert_eq!(TASK_NAMES_DATA[80], Some("Fixed"));
        assert_eq!(TASK_NAMES_DATA[90], Some("ID"));
        assert_eq!(TASK_NAMES_DATA[110], Some("Flag1"));
        assert_eq!(TASK_NAMES_DATA[120], Some("Summary"));
        assert_eq!(TASK_NAMES_DATA[135], Some("Confirmed"));
        assert_eq!(TASK_NAMES_DATA[150], Some("Stop"));
        assert_eq!(TASK_NAMES_DATA.last().copied(), Some(Some("Resume")));
    }

    #[test]
    fn english_resource_names_match_java() {
        assert_eq!(RESOURCE_NAMES_DATA.first().copied(), Some(None));
        assert_eq!(RESOURCE_NAMES_DATA[1], Some("Name"));
        assert_eq!(RESOURCE_NAMES_DATA[11], Some("Email Address"));
        assert_eq!(RESOURCE_NAMES_DATA[12], None);
        assert_eq!(RESOURCE_NAMES_DATA[24], Some("Overtime Work"));
        assert_eq!(RESOURCE_NAMES_DATA[40], Some("ID"));
        assert_eq!(RESOURCE_NAMES_DATA[49], Some("Unique ID"));
        assert_eq!(RESOURCE_NAMES_DATA.last().copied(), Some(Some("Linked Fields")));
    }

    #[test]
    fn english_time_units_are_indexed_by_time_unit_value() {
        // Java: TimeUnit.PERCENT(5) / TimeUnit.YEARS(6)
        assert_eq!(TIME_UNITS_ARRAY_DATA[0], ["m", "mins"].as_slice());
        assert_eq!(TIME_UNITS_ARRAY_DATA[3], ["w", "wk", "weeks"].as_slice());
        assert_eq!(TIME_UNITS_ARRAY_DATA[5], ["%"].as_slice());
        assert_eq!(TIME_UNITS_ARRAY_DATA[6], ["y", "years"].as_slice());
        assert_eq!(TIME_UNITS_ARRAY_DATA[13], ["e%"].as_slice());
    }

    #[test]
    fn every_locale_has_the_java_array_lengths() {
        for locale in MpxLocale::ALL {
            assert_eq!(
                LocaleData::get_string_array(locale, LocaleData::TASK_NAMES).map(<[Option<&str>]>::len),
                Some(153),
                "{locale}"
            );
            // Java: LocaleData_ru.RESOURCE_NAMES_DATA has only 51 entries (missing Overtime Work)
            let resource_names_length = if locale == MpxLocale::Ru { 51 } else { 52 };
            assert_eq!(
                LocaleData::get_string_array(locale, LocaleData::RESOURCE_NAMES)
                    .map(<[Option<&str>]>::len),
                Some(resource_names_length),
                "{locale}"
            );
            assert_eq!(
                LocaleData::get_string_arrays(locale, LocaleData::TIME_UNITS_ARRAY)
                    .map(<[&[&str]]>::len),
                Some(14),
                "{locale}"
            );
            assert_eq!(
                LocaleData::get_string_array(locale, LocaleData::ACCRUE_TYPES)
                    .map(<[Option<&str>]>::len),
                Some(3),
                "{locale}"
            );
            assert_eq!(
                LocaleData::get_string_array(locale, LocaleData::RELATION_TYPES)
                    .map(<[Option<&str>]>::len),
                Some(4),
                "{locale}"
            );
            assert_eq!(
                LocaleData::get_string_array(locale, LocaleData::PRIORITY_TYPES)
                    .map(<[Option<&str>]>::len),
                Some(10),
                "{locale}"
            );
            assert_eq!(
                LocaleData::get_string_array(locale, LocaleData::CONSTRAINT_TYPES)
                    .map(<[Option<&str>]>::len),
                Some(8),
                "{locale}"
            );
        }
    }

    #[test]
    fn localised_tables_are_returned_for_supported_locales() {
        let german_accrue_types: &[Option<&str>] =
            &[Some("Anfang"), Some("Ende"), Some("Anteilig")];
        assert_eq!(
            LocaleData::get_string_array(MpxLocale::De, LocaleData::ACCRUE_TYPES),
            Some(german_accrue_types)
        );

        let french_relation_types: &[Option<&str>] =
            &[Some("FF"), Some("FD"), Some("DF"), Some("DD")];
        assert_eq!(
            LocaleData::get_string_array(MpxLocale::Fr, LocaleData::RELATION_TYPES),
            Some(french_relation_types)
        );

        let swedish_time_units: &[&[&str]] = &[
            &["m"],
            &["t"],
            &["d"],
            &["w"],
            &["mon"],
            &["y"],
            &["%"],
            &["em"],
            &["eh"],
            &["ed"],
            &["ew"],
            &["emon"],
            &["ey"],
            &["e%"],
        ];
        assert_eq!(
            LocaleData::get_string_arrays(MpxLocale::Sv, LocaleData::TIME_UNITS_ARRAY),
            Some(swedish_time_units)
        );

        let chinese_task_names = LocaleData::get_string_array(MpxLocale::Zh, LocaleData::TASK_NAMES);
        assert_eq!(chinese_task_names.and_then(|names| names.get(1)).copied(), Some(Some("名称")));
        assert_eq!(chinese_task_names.and_then(|names| names.last()).copied(), Some(Some("重新开始")));

        let russian_resource_names =
            LocaleData::get_string_array(MpxLocale::Ru, LocaleData::RESOURCE_NAMES);
        assert_eq!(
            russian_resource_names.and_then(|names| names.get(1)).copied(),
            Some(Some("Имя"))
        );
        // Java: the Russian name table lacks Overtime Work, so index 24 is Work Variance
        assert_eq!(
            russian_resource_names.and_then(|names| names.get(24)).copied(),
            Some(Some("Разбежность работ"))
        );
    }

    #[test]
    fn uncovered_keys_fall_back_to_english() {
        // sv does not cover ACCRUE_TYPES / RELATION_TYPES / PRIORITY_TYPES / CONSTRAINT_TYPES
        for key in [
            LocaleData::ACCRUE_TYPES,
            LocaleData::RELATION_TYPES,
            LocaleData::PRIORITY_TYPES,
            LocaleData::CONSTRAINT_TYPES,
        ] {
            assert_eq!(
                LocaleData::get_string_array(MpxLocale::Sv, key),
                LocaleData::get_string_array(MpxLocale::En, key),
                "{key}"
            );
        }

        // de does not cover PROGRAM_NAME / CODE_PAGE / CURRENCY_DIGITS / DATE_ORDER /
        // DEFAULT_START_TIME / TIME_SEPARATOR / BAR_TEXT_DATE_FORMAT
        assert_eq!(
            LocaleData::get_string(MpxLocale::De, LocaleData::PROGRAM_NAME),
            Some("Microsoft Project for Windows")
        );
        assert_eq!(LocaleData::get_code_page(MpxLocale::De), CodePage::Ansi);
        assert_eq!(
            LocaleData::get_integer(MpxLocale::De, LocaleData::CURRENCY_DIGITS),
            Some(2)
        );
        assert_eq!(LocaleData::get_date_order(MpxLocale::De), DateOrder::Dmy);
        assert_eq!(
            LocaleData::get_integer(MpxLocale::De, LocaleData::DEFAULT_START_TIME),
            Some(480)
        );
        assert_eq!(
            LocaleData::get_char(MpxLocale::De, LocaleData::TIME_SEPARATOR),
            Some(':')
        );
        assert_eq!(
            LocaleData::get_integer(MpxLocale::De, LocaleData::BAR_TEXT_DATE_FORMAT),
            Some(0)
        );
        assert_eq!(LocaleData::get_string(MpxLocale::De, LocaleData::NA), Some("NV"));

        // pt / ru do not cover PROGRAM_NAME and FILE_VERSION
        assert_eq!(
            LocaleData::get_string(MpxLocale::Pt, LocaleData::FILE_VERSION),
            Some("4.0")
        );
        assert_eq!(
            LocaleData::get_string(MpxLocale::Ru, LocaleData::PROGRAM_NAME),
            Some("Microsoft Project for Windows")
        );
        // sv does not cover NA
        assert_eq!(LocaleData::get_string(MpxLocale::Sv, LocaleData::NA), Some("NA"));
    }

    #[test]
    fn covered_keys_use_localised_values() {
        assert_eq!(LocaleData::get_string(MpxLocale::De, LocaleData::FILE_VERSION), Some("4,0"));
        assert_eq!(LocaleData::get_char(MpxLocale::De, LocaleData::FILE_DELIMITER), Some(';'));
        assert_eq!(
            LocaleData::get_currency_symbol_position(MpxLocale::De),
            CurrencySymbolPosition::AfterWithSpace
        );
        assert_eq!(
            LocaleData::get_time_format(MpxLocale::De),
            ProjectTimeFormat::TwentyFourHour
        );
        assert_eq!(
            LocaleData::get_date_format(MpxLocale::De),
            ProjectDateFormat::EeeDdMmYy
        );
        assert_eq!(LocaleData::get_char(MpxLocale::De, LocaleData::DATE_SEPARATOR), Some('.'));

        assert_eq!(LocaleData::get_date_order(MpxLocale::Sv), DateOrder::Ymd);
        assert_eq!(LocaleData::get_char(MpxLocale::Sv, LocaleData::DATE_SEPARATOR), Some('-'));
        assert_eq!(
            LocaleData::get_string(MpxLocale::Sv, LocaleData::NO),
            Some("Nej")
        );
        assert_eq!(
            LocaleData::get_char(MpxLocale::Sv, LocaleData::CURRENCY_THOUSANDS_SEPARATOR),
            Some(' ')
        );

        assert_eq!(LocaleData::get_code_page(MpxLocale::Ru), CodePage::Ru);
        assert_eq!(LocaleData::get_code_page(MpxLocale::Zh), CodePage::Zh);
        assert_eq!(LocaleData::get_string(MpxLocale::Fr, LocaleData::NA), Some("NC"));
        assert_eq!(LocaleData::get_string(MpxLocale::Pt, LocaleData::NA), Some("NA"));
        assert_eq!(LocaleData::get_string(MpxLocale::Zh, LocaleData::YES), Some("是"));
        assert_eq!(LocaleData::get_string(MpxLocale::Zh, LocaleData::NO), Some("否"));
        assert_eq!(LocaleData::get_string(MpxLocale::Zh, LocaleData::PROGRAM_NAME), Some("Microsoft Project for Windows"));
    }

    #[test]
    fn unknown_keys_or_mismatched_accessors_return_none() {
        assert_eq!(LocaleData::get_string(MpxLocale::En, "NO_SUCH_KEY"), None);
        assert_eq!(LocaleData::get_string_array(MpxLocale::En, "NO_SUCH_KEY"), None);
        assert_eq!(LocaleData::get_string_arrays(MpxLocale::En, "NO_SUCH_KEY"), None);
        assert_eq!(LocaleData::get_integer(MpxLocale::En, "NO_SUCH_KEY"), None);
        assert_eq!(LocaleData::get_char(MpxLocale::En, "NO_SUCH_KEY"), None);
        assert_eq!(LocaleData::get_map(MpxLocale::En, "NO_SUCH_KEY"), None);

        // Java: getString(ACCRUE_TYPES) throws ClassCastException
        assert_eq!(LocaleData::get_string(MpxLocale::En, LocaleData::ACCRUE_TYPES), None);
        // Java: getStringArray(TIME_UNITS_ARRAY) throws ClassCastException
        assert_eq!(
            LocaleData::get_string_array(MpxLocale::En, LocaleData::TIME_UNITS_ARRAY),
            None
        );
        // Java: getChar(AM_TEXT) throws StringIndexOutOfBoundsException (empty string)
        assert_eq!(LocaleData::get_char(MpxLocale::De, LocaleData::AM_TEXT), None);
    }

    #[test]
    fn time_units_map_is_expanded_from_the_locale_table() {
        assert_eq!(
            LocaleData::get_map(MpxLocale::En, LocaleData::TIME_UNITS_MAP)
                .and_then(|map| map.get("days").copied()),
            Some(2)
        );
        assert_eq!(
            LocaleData::get_map(MpxLocale::En, LocaleData::TIME_UNITS_MAP)
                .and_then(|map| map.get("e%").copied()),
            Some(13)
        );
        assert_eq!(
            LocaleData::get_map(MpxLocale::En, LocaleData::TIME_UNITS_MAP)
                .and_then(|map| map.get("emos").copied()),
            None
        );

        // Java German literal order: index 5 is Years ("y") and index 6 is Percent ("%")
        assert_eq!(
            LocaleData::get_map(MpxLocale::De, LocaleData::TIME_UNITS_MAP)
                .and_then(|map| map.get("t").copied()),
            Some(2)
        );
        assert_eq!(
            LocaleData::get_map(MpxLocale::De, LocaleData::TIME_UNITS_MAP)
                .and_then(|map| map.get("y").copied()),
            Some(5)
        );
        assert_eq!(
            LocaleData::get_map(MpxLocale::De, LocaleData::TIME_UNITS_MAP)
                .and_then(|map| map.get("%").copied()),
            Some(6)
        );
    }
}
