//! 对应 Java: `org/mpxj/mpx/LocaleData.java`
//!
//! # 移植取舍（`ResourceBundle` / `ListResourceBundle` / `java.util.Locale`）
//!
//! Java 侧这一族类都是 `ResourceBundle` 的子类：`LocaleData` 是基类，`RESOURCE_DATA`
//! （`Object[][]`）存放英文默认值；`LocaleData_en/de/es/fr/it/pt/ru/sv/zh` 是各语言变体，
//! 其 `getContents()` 只列出该语言覆盖的键。`ResourceBundle.getBundle` 会按请求的语言
//! 挑选变体，变体中缺失的键自动回落到基类。Rust 标准库没有 `ResourceBundle` / `Locale`，
//! 本模块的处理方式：
//!
//! - 区域设置参数用 [`MpxLocale`]（`java.util.Locale` 的替代物，见 `locale_utility`）；
//! - 基类默认值即本模块的 [`RESOURCE_DATA`]，各数组常量沿用 Java 的字段名
//!   （[`TASK_NAMES_DATA`] / [`RESOURCE_NAMES_DATA`] / [`TIME_UNITS_ARRAY_DATA`] /
//!   [`ACCRUE_TYPES_DATA`] / [`RELATION_TYPES_DATA`] / [`PRIORITY_TYPES_DATA`] /
//!   [`CONSTRAINT_TYPES_DATA`]）；
//! - 各语言变体在 `locale_data_XX` 模块中用 `pub const` 写出它实际覆盖的键（标量常量按
//!   Java `RESOURCE_DATA` 里的键名命名，如 `locale_data_de::FILE_DELIMITER`），并用
//!   [`LocaleOverrides`] 汇总（对应 Java 变体的 `getContents()`）；未覆盖的键为 `None`，
//!   取值时回落到 [`RESOURCE_DATA`]，与 `ResourceBundle` 的父包回落等价；
//! - Java 的 `getString` / `getStringArray` / `getStringArrays` / `getMap` / `getInteger` /
//!   `getChar` 逐个对应到同名的 `get_*` 函数，键用本类型的 `pub const` 字符串常量，
//!   用法与 Java 的 `LocaleData.ACCRUE_TYPES` 一致；`getObject` 依赖调用方强转类型，
//!   Rust 侧改为按类型返回的 [`LocaleData::get_code_page`]、
//!   [`LocaleData::get_currency_symbol_position`]、[`LocaleData::get_date_order`]、
//!   [`LocaleData::get_time_format`]、[`LocaleData::get_date_format`]；
//! - Java 的 `String[]` 元素可为 `null`（`TASK_NAMES` / `RESOURCE_NAMES` 中未命名的字段），
//!   故 `get_string_array` 统一返回 `&[Option<&str>]`；`TIME_UNITS_ARRAY` 是 `String[][]`
//!   且内层无 `null`，由 `get_string_arrays` 返回 `&[&[&str]]`；
//! - `TIME_UNITS_MAP` 在 Java 中由静态块从 `TIME_UNITS_ARRAY_DATA` 展开成 `HashMap`，
//!   Rust 侧改由 [`LocaleData::get_map`] 按需展开（本模块中不再有 `TIME_UNITS_MAP_DATA`
//!   常量）。
//!
//! 跳过并注明：
//! - `ResourceBundle` 的协议方法 `getContents` / `handleGetObject` / `getKeys` /
//!   `setParent` / `keySet`：由本模块的常量表加 [`LocaleOverrides`] 的回落逻辑替代；
//! - `ListResourceBundle`、`java.util.Locale`：分别由 [`LocaleOverrides`] 与 [`MpxLocale`] 替代；
//! - `Serializable`（`ResourceBundle` 自带）：不移植。
//!
//! 未移植的成员：无。
//!
//! # 上游数据中已保留的不一致（按 Java 源码原样照抄）
//!
//! - 各非英文变体的 `TIME_UNITS_ARRAY_DATA` 字面量把「年」放在下标 5、「百分比」放在下标 6，
//!   而 `TimeUnit` 的取值为 `PERCENT(5)` / `YEARS(6)`（英文基类按 `getValue()` 赋值，
//!   因此下标 5 是 `"%"`、下标 6 是 `"y"`）。两处顺序不同是上游既有情况，
//!   本移植逐项照抄，不做「修正」。
//! - `LocaleData_ru.RESOURCE_NAMES_DATA` 只有 51 项（缺少 `Overtime Work` 一项，
//!   使下标 24 之后的名表整体前移），同样照抄。

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

/// 一个区域设置的全部资源，即 Java `ResourceBundle` 解析完成后可读到的键值集合。
///
/// 对应 Java: 基类 `LocaleData.RESOURCE_DATA`（`Object[][]` 形式的键值对）；
/// 这里用带具体类型的字段表达，Java 的 `Object` 强转由字段类型取代。
#[derive(Debug, Clone, Copy)]
pub struct LocaleResources {
    /// 对应资源键 `FILE_DELIMITER`
    pub file_delimiter: char,
    /// 对应资源键 `PROGRAM_NAME`
    pub program_name: &'static str,
    /// 对应资源键 `FILE_VERSION`
    pub file_version: &'static str,
    /// 对应资源键 `CODE_PAGE`
    pub code_page: CodePage,
    /// 对应资源键 `CURRENCY_SYMBOL`
    pub currency_symbol: &'static str,
    /// 对应资源键 `CURRENCY_SYMBOL_POSITION`
    pub currency_symbol_position: CurrencySymbolPosition,
    /// 对应资源键 `CURRENCY_DIGITS`
    pub currency_digits: i32,
    /// 对应资源键 `CURRENCY_THOUSANDS_SEPARATOR`
    pub currency_thousands_separator: char,
    /// 对应资源键 `CURRENCY_DECIMAL_SEPARATOR`
    pub currency_decimal_separator: char,
    /// 对应资源键 `DATE_ORDER`
    pub date_order: DateOrder,
    /// 对应资源键 `TIME_FORMAT`
    pub time_format: ProjectTimeFormat,
    /// 对应资源键 `DEFAULT_START_TIME`（单位：自午夜起的分钟数）
    pub default_start_time: i32,
    /// 对应资源键 `DATE_SEPARATOR`
    pub date_separator: char,
    /// 对应资源键 `TIME_SEPARATOR`
    pub time_separator: char,
    /// 对应资源键 `AM_TEXT`
    pub am_text: &'static str,
    /// 对应资源键 `PM_TEXT`
    pub pm_text: &'static str,
    /// 对应资源键 `DATE_FORMAT`
    pub date_format: ProjectDateFormat,
    /// 对应资源键 `BAR_TEXT_DATE_FORMAT`
    pub bar_text_date_format: i32,
    /// 对应资源键 `NA`
    pub na: &'static str,
    /// 对应资源键 `YES`
    pub yes: &'static str,
    /// 对应资源键 `NO`
    pub no: &'static str,
    /// 对应资源键 `TIME_UNITS_ARRAY`（下标即 `TimeUnit` 的 int 值，内层为该单位的别名）
    pub time_units_array: &'static [&'static [&'static str]],
    /// 对应资源键 `ACCRUE_TYPES`
    pub accrue_types: &'static [Option<&'static str>],
    /// 对应资源键 `RELATION_TYPES`
    pub relation_types: &'static [Option<&'static str>],
    /// 对应资源键 `PRIORITY_TYPES`
    pub priority_types: &'static [Option<&'static str>],
    /// 对应资源键 `CONSTRAINT_TYPES`
    pub constraint_types: &'static [Option<&'static str>],
    /// 对应资源键 `TASK_NAMES`（下标与 MPX 任务字段号对应，`None` 表示该字段无名称）
    pub task_names: &'static [Option<&'static str>],
    /// 对应资源键 `RESOURCE_NAMES`（下标与 MPX 资源字段号对应，`None` 表示该字段无名称）
    pub resource_names: &'static [Option<&'static str>],
}

/// 英文（默认）时间单位别名表。
///
/// 对应 Java: `LocaleData.TIME_UNITS_ARRAY_DATA`（Java 侧按下标 `TimeUnit.getValue()` 赋值，
/// 所以这里的下标 5 是 `PERCENT`、下标 6 是 `YEARS`）。
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

/// 英文（默认）资源键 `ACCRUE_TYPES` 的名表（下标 + 1 即 `AccrueType` 的 int 值）。
///
/// 对应 Java: `LocaleData.ACCRUE_TYPES_DATA`（不含 `null` 元素，`Option` 用于统一
/// `get_string_array` 的返回类型）。
pub const ACCRUE_TYPES_DATA: [Option<&'static str>; 3] =
    [Some("Start"), Some("End"), Some("Prorated")];

/// 英文（默认）资源键 `RELATION_TYPES` 的名表（下标即 `RelationType` 的 int 值）。
///
/// 对应 Java: `LocaleData.RELATION_TYPES_DATA`。
pub const RELATION_TYPES_DATA: [Option<&'static str>; 4] =
    [Some("FF"), Some("FS"), Some("SF"), Some("SS")];

/// 英文（默认）资源键 `PRIORITY_TYPES` 的名表（下标 + 1 再乘 100 即 `Priority` 的值）。
///
/// 对应 Java: `LocaleData.PRIORITY_TYPES_DATA`。
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

/// 英文（默认）资源键 `CONSTRAINT_TYPES` 的名表（下标即 `ConstraintType` 的 int 值）。
///
/// 对应 Java: `LocaleData.CONSTRAINT_TYPES_DATA`。
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

/// 英文（默认）资源键 `TASK_NAMES` 的名表，下标与 MPX 任务字段号一一对应。
///
/// 对应 Java: `LocaleData.TASK_NAMES_DATA`（`null` 元素表示该字段没有名称）。
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

/// 英文（默认）资源键 `RESOURCE_NAMES` 的名表，下标与 MPX 资源字段号一一对应。
///
/// 对应 Java: `LocaleData.RESOURCE_NAMES_DATA`（`null` 元素表示该字段没有名称）。
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

/// 英文（默认）资源，对应 Java `LocaleData.RESOURCE_DATA`（`getContents()` 的返回值）。
///
/// Java 的 `LocaleData_en` 资源包为空，英语完全使用这组默认值。
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

/// 一个语言变体覆盖的资源键，对应 Java `LocaleData_XX` 的 `RESOURCE_DATA`
/// （即 `getContents()` 的返回值）。
///
/// 字段为 `None` 表示该变体未覆盖此键，取值时回落到 [`RESOURCE_DATA`]
/// （等价于 `ResourceBundle` 的父包回落）。
#[derive(Debug, Clone, Copy)]
pub struct LocaleOverrides {
    /// 对应资源键 `FILE_DELIMITER`
    pub file_delimiter: Option<char>,
    /// 对应资源键 `PROGRAM_NAME`
    pub program_name: Option<&'static str>,
    /// 对应资源键 `FILE_VERSION`
    pub file_version: Option<&'static str>,
    /// 对应资源键 `CODE_PAGE`
    pub code_page: Option<CodePage>,
    /// 对应资源键 `CURRENCY_SYMBOL`
    pub currency_symbol: Option<&'static str>,
    /// 对应资源键 `CURRENCY_SYMBOL_POSITION`
    pub currency_symbol_position: Option<CurrencySymbolPosition>,
    /// 对应资源键 `CURRENCY_DIGITS`
    pub currency_digits: Option<i32>,
    /// 对应资源键 `CURRENCY_THOUSANDS_SEPARATOR`
    pub currency_thousands_separator: Option<char>,
    /// 对应资源键 `CURRENCY_DECIMAL_SEPARATOR`
    pub currency_decimal_separator: Option<char>,
    /// 对应资源键 `DATE_ORDER`
    pub date_order: Option<DateOrder>,
    /// 对应资源键 `TIME_FORMAT`
    pub time_format: Option<ProjectTimeFormat>,
    /// 对应资源键 `DEFAULT_START_TIME`
    pub default_start_time: Option<i32>,
    /// 对应资源键 `DATE_SEPARATOR`
    pub date_separator: Option<char>,
    /// 对应资源键 `TIME_SEPARATOR`
    pub time_separator: Option<char>,
    /// 对应资源键 `AM_TEXT`
    pub am_text: Option<&'static str>,
    /// 对应资源键 `PM_TEXT`
    pub pm_text: Option<&'static str>,
    /// 对应资源键 `DATE_FORMAT`
    pub date_format: Option<ProjectDateFormat>,
    /// 对应资源键 `BAR_TEXT_DATE_FORMAT`
    pub bar_text_date_format: Option<i32>,
    /// 对应资源键 `NA`
    pub na: Option<&'static str>,
    /// 对应资源键 `YES`
    pub yes: Option<&'static str>,
    /// 对应资源键 `NO`
    pub no: Option<&'static str>,
    /// 对应资源键 `TIME_UNITS_ARRAY`
    pub time_units_array: Option<&'static [&'static [&'static str]]>,
    /// 对应资源键 `ACCRUE_TYPES`
    pub accrue_types: Option<&'static [Option<&'static str>]>,
    /// 对应资源键 `RELATION_TYPES`
    pub relation_types: Option<&'static [Option<&'static str>]>,
    /// 对应资源键 `PRIORITY_TYPES`
    pub priority_types: Option<&'static [Option<&'static str>]>,
    /// 对应资源键 `CONSTRAINT_TYPES`
    pub constraint_types: Option<&'static [Option<&'static str>]>,
    /// 对应资源键 `TASK_NAMES`
    pub task_names: Option<&'static [Option<&'static str>]>,
    /// 对应资源键 `RESOURCE_NAMES`
    pub resource_names: Option<&'static [Option<&'static str>]>,
}

impl LocaleOverrides {
    /// 所有键均未覆盖，对应 Java `LocaleData_en.RESOURCE_DATA = {}`。
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

/// 资源包处理工具与英文默认资源。
///
/// 对应 Java: `public final class LocaleData extends ListResourceBundle`
/// （私有构造方法禁止实例化，Rust 侧用无字段类型表达）。
pub struct LocaleData;

impl LocaleData {
    /// 对应 Java: `public static final String FILE_DELIMITER`
    pub const FILE_DELIMITER: &'static str = "FILE_DELIMITER";
    /// 对应 Java: `public static final String PROGRAM_NAME`
    pub const PROGRAM_NAME: &'static str = "PROGRAM_NAME";
    /// 对应 Java: `public static final String FILE_VERSION`
    pub const FILE_VERSION: &'static str = "FILE_VERSION";
    /// 对应 Java: `public static final String CODE_PAGE`
    pub const CODE_PAGE: &'static str = "CODE_PAGE";

    /// 对应 Java: `public static final String YES`
    pub const YES: &'static str = "YES";
    /// 对应 Java: `public static final String NO`
    pub const NO: &'static str = "NO";

    /// 对应 Java: `public static final String CURRENCY_SYMBOL`
    pub const CURRENCY_SYMBOL: &'static str = "CURRENCY_SYMBOL";
    /// 对应 Java: `public static final String CURRENCY_SYMBOL_POSITION`
    pub const CURRENCY_SYMBOL_POSITION: &'static str = "CURRENCY_SYMBOL_POSITION";
    /// 对应 Java: `public static final String CURRENCY_DIGITS`
    pub const CURRENCY_DIGITS: &'static str = "CURRENCY_DIGITS";
    /// 对应 Java: `public static final String CURRENCY_THOUSANDS_SEPARATOR`
    pub const CURRENCY_THOUSANDS_SEPARATOR: &'static str = "CURRENCY_THOUSANDS_SEPARATOR";
    /// 对应 Java: `public static final String CURRENCY_DECIMAL_SEPARATOR`
    pub const CURRENCY_DECIMAL_SEPARATOR: &'static str = "CURRENCY_DECIMAL_SEPARATOR";

    /// 对应 Java: `public static final String DATE_ORDER`
    pub const DATE_ORDER: &'static str = "DATE_ORDER";
    /// 对应 Java: `public static final String TIME_FORMAT`
    pub const TIME_FORMAT: &'static str = "TIME_FORMAT";
    /// 对应 Java: `public static final String DEFAULT_START_TIME`
    pub const DEFAULT_START_TIME: &'static str = "DEFAULT_START_TIME";
    /// 对应 Java: `public static final String DATE_SEPARATOR`
    pub const DATE_SEPARATOR: &'static str = "DATE_SEPARATOR";
    /// 对应 Java: `public static final String TIME_SEPARATOR`
    pub const TIME_SEPARATOR: &'static str = "TIME_SEPARATOR";
    /// 对应 Java: `public static final String AM_TEXT`
    pub const AM_TEXT: &'static str = "AM_TEXT";
    /// 对应 Java: `public static final String PM_TEXT`
    pub const PM_TEXT: &'static str = "PM_TEXT";
    /// 对应 Java: `public static final String DATE_FORMAT`
    pub const DATE_FORMAT: &'static str = "DATE_FORMAT";
    /// 对应 Java: `public static final String BAR_TEXT_DATE_FORMAT`
    pub const BAR_TEXT_DATE_FORMAT: &'static str = "BAR_TEXT_DATE_FORMAT";
    /// 对应 Java: `public static final String NA`
    pub const NA: &'static str = "NA";

    /// 对应 Java: `public static final String TIME_UNITS_ARRAY`
    pub const TIME_UNITS_ARRAY: &'static str = "TIME_UNITS_ARRAY";
    /// 对应 Java: `public static final String TIME_UNITS_MAP`
    pub const TIME_UNITS_MAP: &'static str = "TIME_UNITS_MAP";

    /// 对应 Java: `public static final String ACCRUE_TYPES`
    pub const ACCRUE_TYPES: &'static str = "ACCRUE_TYPES";
    /// 对应 Java: `public static final String RELATION_TYPES`
    pub const RELATION_TYPES: &'static str = "RELATION_TYPES";
    /// 对应 Java: `public static final String PRIORITY_TYPES`
    pub const PRIORITY_TYPES: &'static str = "PRIORITY_TYPES";
    /// 对应 Java: `public static final String CONSTRAINT_TYPES`
    pub const CONSTRAINT_TYPES: &'static str = "CONSTRAINT_TYPES";

    /// 对应 Java: `public static final String TASK_NAMES`
    pub const TASK_NAMES: &'static str = "TASK_NAMES";
    /// 对应 Java: `public static final String RESOURCE_NAMES`
    pub const RESOURCE_NAMES: &'static str = "RESOURCE_NAMES";

    /// 取字符串资源。
    ///
    /// 对应 Java: `public static final String getString(Locale locale, String key)`
    ///
    /// Java 侧键不存在时抛 `MissingResourceException`、键值不是字符串时抛
    /// `ClassCastException`，Rust 侧统一返回 `None`。
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

    /// 取字符串数组资源（元素可为 `None`，对应 Java `String[]` 里的 `null`）。
    ///
    /// 对应 Java: `public static final String[] getStringArray(Locale locale, String key)`
    ///
    /// `TIME_UNITS_ARRAY` 是 `String[][]`，Java 用本方法取它会抛 `ClassCastException`，
    /// Rust 侧返回 `None`（请改用 [`LocaleData::get_string_arrays`]）。
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

    /// 取二维字符串数组资源。
    ///
    /// 对应 Java: `public static final String[][] getStringArrays(Locale locale, String key)`
    ///
    /// 只有 `TIME_UNITS_ARRAY` 属于此类；其他键 Java 会抛 `ClassCastException`，
    /// Rust 侧返回 `None`。
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

    /// 取 Map 资源，即「时间单位名（小写）→ `TimeUnit` 的 int 值」的映射。
    ///
    /// 对应 Java: `public static final Map getMap(Locale locale, String key)`
    ///
    /// Java 的 `TIME_UNITS_MAP` 由静态块从 `TIME_UNITS_ARRAY_DATA` 展开，
    /// 同名时后写覆盖先写；Rust 侧在每次调用时按同样顺序展开（只有该键属于 Map，
    /// 其他键 Java 会抛 `ClassCastException`，Rust 侧返回 `None`）。
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

    /// 取整数资源。
    ///
    /// 对应 Java: `public static final Integer getInteger(Locale locale, String key)`
    ///
    /// Java 侧键不存在时抛 `MissingResourceException`、键值不是整数时抛
    /// `ClassCastException`，Rust 侧统一返回 `None`。
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

    /// 取单字符资源（Java 取字符串的首字符）。
    ///
    /// 对应 Java: `public static final char getChar(Locale locale, String key)`
    ///
    /// Java 的 `getChar` 对键值不是字符串的键抛 `ClassCastException`、对空字符串抛
    /// `StringIndexOutOfBoundsException`；Rust 侧这些情况统一返回 `None`。
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

    /// 取代码页。
    ///
    /// 对应 Java: `(CodePage) LocaleData.getObject(locale, LocaleData.CODE_PAGE)`
    pub fn get_code_page(locale: MpxLocale) -> CodePage {
        overrides(locale).code_page.unwrap_or(RESOURCE_DATA.code_page)
    }

    /// 取货币符号位置。
    ///
    /// 对应 Java: `(CurrencySymbolPosition) LocaleData.getObject(locale, LocaleData.CURRENCY_SYMBOL_POSITION)`
    pub fn get_currency_symbol_position(locale: MpxLocale) -> CurrencySymbolPosition {
        overrides(locale)
            .currency_symbol_position
            .unwrap_or(RESOURCE_DATA.currency_symbol_position)
    }

    /// 取日期顺序。
    ///
    /// 对应 Java: `(DateOrder) LocaleData.getObject(locale, LocaleData.DATE_ORDER)`
    pub fn get_date_order(locale: MpxLocale) -> DateOrder {
        overrides(locale).date_order.unwrap_or(RESOURCE_DATA.date_order)
    }

    /// 取时间格式。
    ///
    /// 对应 Java: `(ProjectTimeFormat) LocaleData.getObject(locale, LocaleData.TIME_FORMAT)`
    pub fn get_time_format(locale: MpxLocale) -> ProjectTimeFormat {
        overrides(locale).time_format.unwrap_or(RESOURCE_DATA.time_format)
    }

    /// 取日期格式。
    ///
    /// 对应 Java: `(ProjectDateFormat) LocaleData.getObject(locale, LocaleData.DATE_FORMAT)`
    pub fn get_date_format(locale: MpxLocale) -> ProjectDateFormat {
        overrides(locale).date_format.unwrap_or(RESOURCE_DATA.date_format)
    }
}

/// 取区域设置对应的资源覆盖项。
///
/// 对应 Java: `ResourceBundle.getBundle(LocaleData.class.getName(), locale)` 选中语言变体的
/// 过程；`MpxLocale` 只有 `SUPPORTED_LOCALES` 中的 9 种取值，因此这里的 `match` 是穷举的。
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
            // Java: LocaleData_ru.RESOURCE_NAMES_DATA 只有 51 项（缺 Overtime Work）
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
        // Java: 俄语名表缺少 Overtime Work，下标 24 是 Work Variance
        assert_eq!(
            russian_resource_names.and_then(|names| names.get(24)).copied(),
            Some(Some("Разбежность работ"))
        );
    }

    #[test]
    fn uncovered_keys_fall_back_to_english() {
        // sv 未覆盖 ACCRUE_TYPES / RELATION_TYPES / PRIORITY_TYPES / CONSTRAINT_TYPES
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

        // de 未覆盖 PROGRAM_NAME / CODE_PAGE / CURRENCY_DIGITS / DATE_ORDER /
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

        // pt / ru 未覆盖 PROGRAM_NAME 与 FILE_VERSION
        assert_eq!(
            LocaleData::get_string(MpxLocale::Pt, LocaleData::FILE_VERSION),
            Some("4.0")
        );
        assert_eq!(
            LocaleData::get_string(MpxLocale::Ru, LocaleData::PROGRAM_NAME),
            Some("Microsoft Project for Windows")
        );
        // sv 未覆盖 NA
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

        // Java: getString(ACCRUE_TYPES) 抛 ClassCastException
        assert_eq!(LocaleData::get_string(MpxLocale::En, LocaleData::ACCRUE_TYPES), None);
        // Java: getStringArray(TIME_UNITS_ARRAY) 抛 ClassCastException
        assert_eq!(
            LocaleData::get_string_array(MpxLocale::En, LocaleData::TIME_UNITS_ARRAY),
            None
        );
        // Java: getChar(AM_TEXT) 抛 StringIndexOutOfBoundsException（空字符串）
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

        // Java 德语字面量顺序：下标 5 是 Years（"y"），下标 6 是 Percent（"%"）
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
