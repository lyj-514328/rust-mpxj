//! 对应 Java: `org/mpxj/mpx/LocaleData_zh.java`
//!
//! # 移植取舍（`ResourceBundle` / `ListResourceBundle`）
//!
//! Java 的 `LocaleData_zh` 是中文资源包，只覆盖部分键；未覆盖的键由 `ResourceBundle`
//! 从基类回落取值。Rust 侧把该语言实际覆盖的键写成模块级 `pub const`
//! （标量常量按 Java `RESOURCE_DATA` 的键名命名，数组常量沿用 Java 的字段名），
//! 并用 [`OVERRIDES`]（对应 `getContents()` 的返回值）汇总；未覆盖的键为 `None`，
//! 由 `locale_data` 模块回落到英文默认资源。取值逻辑见 `locale_data` 模块头部说明；
//! `ListResourceBundle` 的协议方法由常量表替代，不移植。
//!
//! 本语言未覆盖的键：`DEFAULT_START_TIME`。
//!
//! 需注意的上游数据（均按 Java 源码原样照抄）：
//! - `PRIORITY_TYPES_DATA` 在 Java 中仍是英文原名；
//! - `TASK_NAMES_DATA` 下标 66 是 `"开时间差异"`（源码漏字）、下标 95 是 `"优先级'"`；
//! - `RESOURCE_NAMES_DATA` 下标 24 与 26 的值同为 `"工时完成百分比"`。
//!
//! 未移植的成员：无。

use crate::code_page::CodePage;
use crate::currency_symbol_position::CurrencySymbolPosition;
use crate::date_order::DateOrder;
use crate::mpx::locale_data::LocaleOverrides;
use crate::project_date_format::ProjectDateFormat;
use crate::project_time_format::ProjectTimeFormat;

/// 中文时间单位别名表。
///
/// 对应 Java: `LocaleData_zh.TIME_UNITS_ARRAY_DATA`（Java 字面量顺序：分钟、小时、天、
/// 周、月、年、百分比、随后是各「已用」单位；下标 5 是年、下标 6 是百分比，
/// 与 `TimeUnit` 的取值顺序不同，此处照抄 Java）。
pub const TIME_UNITS_ARRAY_DATA: [&'static [&'static str]; 14] = [
    &["m"],      // 0 TimeUnit.MINUTES
    &["h"],      // 1 TimeUnit.HOURS
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

/// 中文资源键 `ACCRUE_TYPES` 的名表。
///
/// 对应 Java: `LocaleData_zh.ACCRUE_TYPES_DATA`。
pub const ACCRUE_TYPES_DATA: [Option<&'static str>; 3] =
    [Some("开始"), Some("结束"), Some("按比例")];

/// 中文资源键 `RELATION_TYPES` 的名表。
///
/// 对应 Java: `LocaleData_zh.RELATION_TYPES_DATA`（与英文默认值相同，但 Java 变体中显式覆盖）。
pub const RELATION_TYPES_DATA: [Option<&'static str>; 4] =
    [Some("FF"), Some("FS"), Some("SF"), Some("SS")];

/// 中文资源键 `PRIORITY_TYPES` 的名表。
///
/// 对应 Java: `LocaleData_zh.PRIORITY_TYPES_DATA`（Java 源码中为英文原名）。
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

/// 中文资源键 `CONSTRAINT_TYPES` 的名表。
///
/// 对应 Java: `LocaleData_zh.CONSTRAINT_TYPES_DATA`。
pub const CONSTRAINT_TYPES_DATA: [Option<&'static str>; 8] = [
    Some("越早越好"),
    Some("越晚越好"),
    Some("必须开始于"),
    Some("必须完成于"),
    Some("不得早于...开始"),
    Some("不得晚于...开始"),
    Some("不得早于...完成"),
    Some("不得晚于...完成"),
];

/// 中文资源键 `TASK_NAMES` 的名表（下标与 MPX 任务字段号对应；行末注释为英文默认名）。
///
/// 对应 Java: `LocaleData_zh.TASK_NAMES_DATA`。
pub const TASK_NAMES_DATA: [Option<&'static str>; 153] = [
    None,                              // -
    Some("名称"),                      // Name
    Some("WBS"),                       // WBS
    Some("大纲级别"),                  // Outline Level
    Some("文本1"),                     // Text1
    Some("文本2"),                     // Text2
    Some("文本3"),                     // Text3
    Some("文本4"),                     // Text4
    Some("文本5"),                     // Text5
    Some("文本6"),                     // Text6
    Some("文本7"),                     // Text7
    Some("文本8"),                     // Text8
    Some("文本9"),                     // Text9
    Some("文本10"),                    // Text10
    Some("备注"),                      // Notes
    Some("联系人"),                    // Contact
    Some("资源组"),                    // Resource Group
    None,                              // -
    None,                              // -
    None,                              // -
    Some("工时"),                      // Work
    Some("比较基准工时"),              // Baseline Work
    Some("实际工时"),                  // Actual Work
    Some("剩余工时"),                  // Remaining Work
    Some("工时差异"),                  // Work Variance
    Some("工时完成百分比"),            // % Work Complete
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    Some("成本"),                      // Cost
    Some("比较基准成本"),              // Baseline Cost
    Some("实际成本"),                  // Actual Cost
    Some("剩余成本"),                  // Remaining Cost
    Some("成本差异"),                  // Cost Variance
    Some("固定成本"),                  // Fixed Cost
    Some("成本1"),                     // Cost1
    Some("成本2"),                     // Cost2
    Some("成本3"),                     // Cost3
    None,                              // -
    Some("工期"),                      // Duration
    Some("比较基准工期"),              // Baseline Duration
    Some("实际工期"),                  // Actual Duration
    Some("剩余工期"),                  // Remaining Duration
    Some("完成百分比"),                // % Complete
    Some("工期差异"),                  // Duration Variance
    Some("工期1"),                     // Duration1
    Some("工期2"),                     // Duration2
    Some("工期3"),                     // Duration3
    None,                              // -
    Some("开始时间"),                  // Start
    Some("完成时间"),                  // Finish
    Some("最早开始时间"),              // Early Start
    Some("最早完成时间"),              // Early Finish
    Some("最晚开始时间"),              // Late Start
    Some("最晚完成时间"),              // Late Finish
    Some("比较基准开始时间"),          // Baseline Start
    Some("比较基准完成时间"),          // Baseline Finish
    Some("实际开始时间"),              // Actual Start
    Some("实际完成时间"),              // Actual Finish
    Some("开始时间1"),                 // Start1
    Some("完成时间1"),                 // Finish1
    Some("开始时间2"),                 // Start2
    Some("完成时间2"),                 // Finish2
    Some("开始时间3"),                 // Start3
    Some("完成时间3"),                 // Finish3
    Some("开时间差异"),                // Start Variance（Java 源码漏字）
    Some("完成时间差异"),              // Finish Variance
    Some("限制日期"),                  // Constraint Date
    None,                              // -
    Some("前置任务"),                  // Predecessors
    Some("后续任务"),                  // Successors
    Some("资源名称"),                  // Resource Names
    Some("资源缩写"),                  // Resource Initials
    Some("唯一标识号前置任务"),        // Unique ID Predecessors
    Some("唯一标识号后续任务"),        // Unique ID Successors
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    Some("固定"),                      // Fixed
    Some("里程碑"),                    // Milestone
    Some("关键"),                      // Critical
    Some("已标记"),                    // Marked
    Some("总成型任务"),                // Rollup
    Some("BCWS"),                      // BCWS
    Some("BCWP"),                      // BCWP
    Some("SV"),                        // SV
    Some("CV"),                        // CV
    None,                              // -
    Some("标识号"),                    // ID
    Some("限制类型"),                  // Constraint Type
    Some("延迟"),                      // Delay
    Some("可用时差"),                  // Free Slack
    Some("总时差"),                    // Total Slack
    Some("优先级'"),                   // Priority（Java 源码含单引号）
    Some("子项目文件"),                // Subproject File
    Some("项目"),                      // Project
    Some("唯一标识号"),                // Unique ID
    Some("大纲数字"),                  // Outline Number
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
    Some("标志1"),                     // Flag1
    Some("标志2"),                     // Flag2
    Some("标志3"),                     // Flag3
    Some("标志4"),                     // Flag4
    Some("标志5"),                     // Flag5
    Some("标志6"),                     // Flag6
    Some("标志7"),                     // Flag7
    Some("标志8"),                     // Flag8
    Some("标志9"),                     // Flag9
    Some("标志10"),                    // Flag10
    Some("摘要"),                      // Summary
    Some("对象数目"),                  // Objects
    Some("链接域"),                    // Linked Fields
    Some("隐藏条形图"),                // Hide Bar
    None,                              // -
    Some("创建日期"),                  // Created
    Some("开始时间4"),                 // Start4
    Some("完成时间4"),                 // Finish4
    Some("开始时间5"),                 // Start5
    Some("完成时间5"),                 // Finish5
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    Some("已确认"),                    // Confirmed
    Some("需要更新"),                  // Update Needed
    None,                              // -
    None,                              // -
    None,                              // -
    Some("数字1"),                     // Number1
    Some("数字2"),                     // Number2
    Some("数字3"),                     // Number3
    Some("数字4"),                     // Number4
    Some("数字5"),                     // Number5
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    Some("停止"),                      // Stop
    Some("不早于...重新开始"),         // Resume No Earlier Than
    Some("重新开始"),                  // Resume
];

/// 中文资源键 `RESOURCE_NAMES` 的名表（下标与 MPX 资源字段号对应；行末注释为英文默认名）。
///
/// 对应 Java: `LocaleData_zh.RESOURCE_NAMES_DATA`。
pub const RESOURCE_NAMES_DATA: [Option<&'static str>; 52] = [
    None,                          // -
    Some("名称"),                  // Name
    Some("缩写"),                  // Initials
    Some("组"),                    // Group
    Some("代码"),                  // Code
    Some("文本1"),                 // Text1
    Some("文本2"),                 // Text2
    Some("文本3"),                 // Text3
    Some("文本4"),                 // Text4
    Some("文本5"),                 // Text5
    Some("备注"),                  // Notes
    Some("电子邮件地址"),          // Email Address
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    Some("工时"),                  // Work
    Some("比较基准工时"),          // Baseline Work
    Some("剩余工时"),              // Remaining Work（Java 源码位置 22）
    Some("加班工时"),              // Overtime Work（Java 源码位置 23）
    Some("工时完成百分比"),        // % Work Complete（Java 源码位置 24）
    Some("工时差异"),              // Work Variance（Java 源码位置 25）
    Some("工时完成百分比"),        // % Work Complete（Java 源码位置 26，与下标 24 重复）
    None,                          // -
    None,                          // -
    None,                          // -
    Some("成本"),                  // Cost
    Some("比较基准成本"),          // Baseline Cost
    Some("实际成本"),              // Actual Cost
    Some("剩余成本"),              // Remaining Cost
    Some("成本差异"),              // Cost Variance
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    None,                          // -
    Some("标识号"),                // ID
    Some("最大单位"),              // Max Units
    Some("标准费率"),              // Standard Rate
    Some("加班费率"),              // Overtime Rate
    Some("每次使用成本"),          // Cost Per Use
    Some("成本累算"),              // Accrue At
    Some("过度分配"),              // Overallocated
    Some("最大使用量"),            // Peak
    Some("基准日历"),              // Base Calendar
    Some("唯一标识号"),            // Unique ID
    Some("对象数目"),              // Objects
    Some("链接域"),                // Linked Fields
];

/// 对应资源键 `FILE_DELIMITER`
pub const FILE_DELIMITER: char = ';';
/// 对应资源键 `PROGRAM_NAME`
pub const PROGRAM_NAME: &'static str = "Microsoft Project for Windows";
/// 对应资源键 `FILE_VERSION`
pub const FILE_VERSION: &'static str = "4.0";
/// 对应资源键 `CODE_PAGE`
pub const CODE_PAGE: CodePage = CodePage::Zh;
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
pub const YES: &'static str = "是";
/// 对应资源键 `NO`
pub const NO: &'static str = "否";

/// 中文资源包的内容，对应 Java `LocaleData_zh.RESOURCE_DATA`
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
        assert_eq!(TASK_NAMES_DATA[1], Some("名称"));
        assert_eq!(TASK_NAMES_DATA[16], Some("资源组"));
        assert_eq!(TASK_NAMES_DATA[31], Some("比较基准成本"));
        assert_eq!(TASK_NAMES_DATA[66], Some("开时间差异"));
        assert_eq!(TASK_NAMES_DATA[95], Some("优先级'"));
        assert_eq!(TASK_NAMES_DATA[152], Some("重新开始"));

        assert_eq!(RESOURCE_NAMES_DATA[11], Some("电子邮件地址"));
        assert_eq!(RESOURCE_NAMES_DATA[23], Some("加班工时"));
        // Java: 下标 24 与 26 的值相同
        assert_eq!(RESOURCE_NAMES_DATA[24], Some("工时完成百分比"));
        assert_eq!(RESOURCE_NAMES_DATA[26], Some("工时完成百分比"));
        assert_eq!(RESOURCE_NAMES_DATA[51], Some("链接域"));

        assert_eq!(TIME_UNITS_ARRAY_DATA[4], ["mon"].as_slice());
        assert_eq!(ACCRUE_TYPES_DATA[2], Some("按比例"));
        assert_eq!(RELATION_TYPES_DATA[3], Some("SS"));
        assert_eq!(PRIORITY_TYPES_DATA[0], Some("Lowest"));
        assert_eq!(CONSTRAINT_TYPES_DATA[0], Some("越早越好"));
    }

    #[test]
    fn locale_data_returns_chinese_tables() {
        assert_eq!(
            LocaleData::get_string_array(MpxLocale::Zh, LocaleData::TASK_NAMES),
            Some(TASK_NAMES_DATA.as_slice())
        );
        assert_eq!(LocaleData::get_code_page(MpxLocale::Zh), CodePage::Zh);
        assert_eq!(LocaleData::get_string(MpxLocale::Zh, LocaleData::YES), Some("是"));
        assert_eq!(LocaleData::get_string(MpxLocale::Zh, LocaleData::NO), Some("否"));
        assert_eq!(LocaleData::get_string(MpxLocale::Zh, LocaleData::NA), Some("NA"));
        // 未覆盖的键回落英文
        assert_eq!(
            LocaleData::get_integer(MpxLocale::Zh, LocaleData::DEFAULT_START_TIME),
            Some(480)
        );
    }
}
