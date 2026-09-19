//! 对应 Java: `org/mpxj/mpx/LocaleData_ru.java`
//!
//! # 移植取舍（`ResourceBundle` / `ListResourceBundle`）
//!
//! Java 的 `LocaleData_ru` 是俄语资源包，只覆盖部分键；未覆盖的键由 `ResourceBundle`
//! 从基类回落取值。Rust 侧把该语言实际覆盖的键写成模块级 `pub const`
//! （标量常量按 Java `RESOURCE_DATA` 的键名命名，数组常量沿用 Java 的字段名），
//! 并用 [`OVERRIDES`]（对应 `getContents()` 的返回值）汇总；未覆盖的键为 `None`，
//! 由 `locale_data` 模块回落到英文默认资源。取值逻辑见 `locale_data` 模块头部说明；
//! `ListResourceBundle` 的协议方法由常量表替代，不移植。
//!
//! 本语言未覆盖的键：`PROGRAM_NAME`、`FILE_VERSION`、`DEFAULT_START_TIME`。
//!
//! 需注意的上游数据（均按 Java 源码原样照抄）：
//! - `RESOURCE_NAMES_DATA` 只有 51 项：缺少 `Overtime Work` 一项，因此下标 24 之后
//!   的名表整体比英文表前移一位；
//! - 若干条目存在明显的拼写/复制问题（如 `Запалнированная`、`Запалнироанное`、
//!   `Finish1` 与 `Finish` 同名、`"Скрытое поле "` 末尾带空格）。
//!
//! 未移植的成员：无。

use crate::code_page::CodePage;
use crate::currency_symbol_position::CurrencySymbolPosition;
use crate::date_order::DateOrder;
use crate::mpx::locale_data::LocaleOverrides;
use crate::project_date_format::ProjectDateFormat;
use crate::project_time_format::ProjectTimeFormat;

/// 俄语时间单位别名表。
///
/// 对应 Java: `LocaleData_ru.TIME_UNITS_ARRAY_DATA`（Java 字面量顺序：分钟、小时、天、
/// 周、月、年、百分比、随后是各「已用」单位；下标 5 是年、下标 6 是百分比，
/// 与 `TimeUnit` 的取值顺序不同，此处照抄 Java）。
pub const TIME_UNITS_ARRAY_DATA: [&'static [&'static str]; 14] = [
    &["минута"],           // 0 TimeUnit.MINUTES
    &["час"],              // 1 TimeUnit.HOURS
    &["день"],             // 2 TimeUnit.DAYS
    &["неделя"],           // 3 TimeUnit.WEEKS
    &["месяц"],            // 4 TimeUnit.MONTHS
    &["год"],              // 5 TimeUnit.PERCENT（Java 字面量位置）
    &["%"],                // 6 TimeUnit.YEARS（Java 字面量位置）
    &["каждую минуту"],    // 7 TimeUnit.ELAPSED_MINUTES
    &["каждый час"],       // 8 TimeUnit.ELAPSED_HOURS
    &["каждый день"],      // 9 TimeUnit.ELAPSED_DAYS
    &["каждую неделю"],    // 10 TimeUnit.ELAPSED_WEEKS
    &["каждый месяц"],     // 11 TimeUnit.ELAPSED_MONTHS
    &["каждый год"],       // 12 TimeUnit.ELAPSED_YEARS
    &["каждый %"],         // 13 TimeUnit.ELAPSED_PERCENT
];

/// 俄语资源键 `ACCRUE_TYPES` 的名表。
///
/// 对应 Java: `LocaleData_ru.ACCRUE_TYPES_DATA`。
pub const ACCRUE_TYPES_DATA: [Option<&'static str>; 3] =
    [Some("Начало"), Some("Конец"), Some("Повседневный")];

/// 俄语资源键 `RELATION_TYPES` 的名表。
///
/// 对应 Java: `LocaleData_ru.RELATION_TYPES_DATA`。
pub const RELATION_TYPES_DATA: [Option<&'static str>; 4] =
    [Some("КК"), Some("КН"), Some("НК"), Some("НН")];

/// 俄语资源键 `PRIORITY_TYPES` 的名表。
///
/// 对应 Java: `LocaleData_ru.PRIORITY_TYPES_DATA`。
pub const PRIORITY_TYPES_DATA: [Option<&'static str>; 10] = [
    Some("Самый низкий"),
    Some("Очень низкий"),
    Some("Низкий"),
    Some("Ниже среднего"),
    Some("Средний"),
    Some("Выше среднего"),
    Some("Высокий"),
    Some("Очень высокий"),
    Some("Наивысший"),
    Some("Без приоритета"),
];

/// 俄语资源键 `CONSTRAINT_TYPES` 的名表。
///
/// 对应 Java: `LocaleData_ru.CONSTRAINT_TYPES_DATA`。
pub const CONSTRAINT_TYPES_DATA: [Option<&'static str>; 8] = [
    Some("Как можно раньше"),
    Some("Как можно позже"),
    Some("Должен начаться"),
    Some("Должен закончиться"),
    Some("Начаться не раньше"),
    Some("Начаться не позже"),
    Some("Закончиться не раньше"),
    Some("Закончиться не позже"),
];

/// 俄语资源键 `TASK_NAMES` 的名表（下标与 MPX 任务字段号对应；行末注释为英文默认名）。
///
/// 对应 Java: `LocaleData_ru.TASK_NAMES_DATA`。
pub const TASK_NAMES_DATA: [Option<&'static str>; 153] = [
    None,                                        // -
    Some("Название"),                            // Name
    Some("WBS"),                                 // WBS
    Some("Внешний уровень"),                     // Outline Level
    Some("Текст1"),                              // Text1
    Some("Текст2"),                              // Text2
    Some("Текст3"),                              // Text3
    Some("Текст4"),                              // Text4
    Some("Текст5"),                              // Text5
    Some("Текст6"),                              // Text6
    Some("Текст7"),                              // Text7
    Some("Текст8"),                              // Text8
    Some("Текст9"),                              // Text9
    Some("Текст10"),                             // Text10
    Some("Примечание"),                          // Notes
    Some("Контакт"),                             // Contact
    Some("Группа ресурсов"),                     // Resource Group
    None,                                        // -
    None,                                        // -
    None,                                        // -
    Some("Работа"),                              // Work
    Some("Запланировано работ"),                 // Baseline Work
    Some("Выполнено работ"),                     // Actual Work
    Some("Осталось работ"),                      // Remaining Work
    Some("Разбежность работ"),                   // Work Variance
    Some("% работ завершено"),                   // % Work Complete
    None,                                        // -
    None,                                        // -
    None,                                        // -
    None,                                        // -
    Some("Стоимость"),                           // Cost
    Some("Запалнированная стоимость"),           // Baseline Cost（Java 源码中的拼写）
    Some("Реальная стоимость"),                  // Actual Cost
    Some("Ожидаемая стоимость"),                 // Remaining Cost
    Some("Разбежнность стоимости"),              // Cost Variance（Java 源码拼写为双 н）
    Some("Фиксированная стоимость"),             // Fixed Cost
    Some("Стоимость1"),                          // Cost1
    Some("Стоимость2"),                          // Cost2
    Some("Стоимость3"),                          // Cost3
    None,                                        // -
    Some("Длительность"),                        // Duration
    Some("Запалнированная длительность"),        // Baseline Duration（Java 源码中的拼写）
    Some("Реальная длительность"),               // Actual Duration
    Some("Ожидаемая длительность"),              // Remaining Duration
    Some("% завершено"),                         // % Complete
    Some("Разбежнность длительности"),           // Duration Variance（Java 源码拼写为双 н）
    Some("Длительность1"),                       // Duration1
    Some("Длительность2"),                       // Duration2
    Some("Длительность3"),                       // Duration3
    None,                                        // -
    Some("Начало"),                              // Start
    Some("Окончание"),                           // Finish
    Some("Предыдущее начало"),                   // Early Start
    Some("Предыдущее окончание"),                // Early Finish
    Some("Следующее начало"),                    // Late Start
    Some("Следующее окончание"),                 // Late Finish
    Some("Запалнироанное начало"),               // Baseline Start（Java 源码中的拼写）
    Some("Запланированное окончание"),           // Baseline Finish
    Some("Реальное начало"),                     // Actual Start
    Some("Реальное окончание"),                  // Actual Finish
    Some("Начало1"),                             // Start1
    Some("Окончание"),                           // Finish1（Java 源码与 Finish 同名）
    Some("Начало2"),                             // Start2
    Some("Окончание2"),                          // Finish2
    Some("Начало3"),                             // Start3
    Some("Окончание3"),                          // Finish3
    Some("Смешение начала"),                     // Start Variance（Java 源码为 ш）
    Some("Смещение окончания"),                  // Finish Variance
    Some("Принудительная дата"),                 // Constraint Date
    None,                                        // -
    Some("Предшественники"),                     // Predecessors
    Some("Наследники"),                          // Successors
    Some("Имя ресурса"),                         // Resource Names
    Some("Инициалы ресурса"),                    // Resource Initials
    Some("Уникальный ID предшественника"),       // Unique ID Predecessors
    Some("Уникальный ID наследника"),            // Unique ID Successors
    None,                                        // -
    None,                                        // -
    None,                                        // -
    None,                                        // -
    Some("Фиксированный"),                       // Fixed
    Some("Веха"),                                // Milestone
    Some("Критическая"),                         // Critical
    Some("Помеченная"),                          // Marked
    Some("Повышенная"),                          // Rollup
    Some("Стоимость запланированных работ"),     // BCWS
    Some("Стоимость выполненных работ"),         // BCWP
    Some("Стоимость остаточных работ"),          // SV
    Some("Разбежность стоимостей"),              // CV
    None,                                        // -
    Some("ID"),                                  // ID
    Some("Принудительный Тип"),                  // Constraint Type
    Some("Задержка"),                            // Delay
    Some("Свободно"),                            // Free Slack
    Some("Всего свободно"),                      // Total Slack
    Some("Приоритет"),                           // Priority
    Some("Файл подпроекта"),                     // Subproject File
    Some("Проект"),                              // Project
    Some("Уникальный ID"),                       // Unique ID
    Some("Внешний Номер"),                       // Outline Number
    None,                                        // -
    None,                                        // -
    None,                                        // -
    None,                                        // -
    None,                                        // -
    None,                                        // -
    None,                                        // -
    None,                                        // -
    None,                                        // -
    None,                                        // -
    Some("Флаг1"),                               // Flag1
    Some("Флаг2"),                               // Flag2
    Some("Флаг3"),                               // Flag3
    Some("Флаг4"),                               // Flag4
    Some("Флаг5"),                               // Flag5
    Some("Флаг6"),                               // Flag6
    Some("Флаг7"),                               // Flag7
    Some("Флаг8"),                               // Flag8
    Some("Флаг9"),                               // Flag9
    Some("Флаг10"),                              // Flag10
    Some("Всего"),                               // Summary
    Some("Объекты"),                             // Objects
    Some("Отмечено полей"),                      // Linked Fields
    Some("Скрытое поле "),                       // Hide Bar（Java 源码末尾含空格）
    None,                                        // -
    Some("Создано"),                             // Created
    Some("Начало4"),                             // Start4
    Some("Окончание4"),                          // Finish4
    Some("Начало5"),                             // Start5
    Some("Окончание5"),                          // Finish5
    None,                                        // -
    None,                                        // -
    None,                                        // -
    None,                                        // -
    None,                                        // -
    Some("Утвержденный"),                        // Confirmed
    Some("Нуждается в обновлении"),              // Update Needed
    None,                                        // -
    None,                                        // -
    None,                                        // -
    Some("Номер1"),                              // Number1
    Some("Номер2"),                              // Number2
    Some("Номер3"),                              // Number3
    Some("Номер4"),                              // Number4
    Some("Номер5"),                              // Number5
    None,                                        // -
    None,                                        // -
    None,                                        // -
    None,                                        // -
    None,                                        // -
    Some("Стоп"),                                // Stop
    Some("Начать не раньше чем"),                // Resume No Earlier Than
    Some("Начать"),                              // Resume
];

/// 俄语资源键 `RESOURCE_NAMES` 的名表（下标与 MPX 资源字段号对应；行末注释为英文默认名）。
///
/// 对应 Java: `LocaleData_ru.RESOURCE_NAMES_DATA`。Java 源码中该表只有 51 项
/// （缺少 `Overtime Work`），所以下标 24 起比英文表前移一位。
pub const RESOURCE_NAMES_DATA: [Option<&'static str>; 51] = [
    None,                              // -
    Some("Имя"),                       // Name
    Some("Инициалы"),                  // Initials
    Some("Группа"),                    // Group
    Some("Код"),                       // Code
    Some("Текст1"),                    // Text1
    Some("Текст2"),                    // Text2
    Some("Текст3"),                    // Text3
    Some("Текст4"),                    // Text4
    Some("Текст5"),                    // Text5
    Some("Примечания"),                // Notes
    Some("Email"),                     // Email Address
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    Some("Работа"),                    // Work
    Some("Запланировано работ"),       // Baseline Work
    Some("Выполнено работ"),           // Actual Work
    Some("Осталось работ"),            // Remaining Work
    Some("Разбежность работ"),         // Work Variance（Java 源码缺 Overtime Work）
    Some("% работ завершено"),         // % Work Complete
    None,                              // -
    None,                              // -
    None,                              // -
    Some("Стоимость"),                 // Cost
    Some("Запалнированная стоимость"), // Baseline Cost（Java 源码中的拼写）
    Some("Реальная стоимость"),        // Actual Cost
    Some("Ожидаемая стоимость"),       // Remaining Cost
    Some("Разбежнность стоимости"),    // Cost Variance（Java 源码拼写为双 н）
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    None,                              // -
    Some("ID"),                        // ID
    Some("Максимальное значение"),     // Max Units
    Some("Стандартное значение"),      // Standard Rate
    Some("Просроченное значение"),     // Overtime Rate
    Some("Стоимость использования"),   // Cost Per Use
    Some("Произошло"),                 // Accrue At
    Some("Переназначено"),             // Overallocated
    Some("Пик"),                       // Peak
    Some("Основной календарь"),        // Base Calendar
    Some("Уникальный ID"),             // Unique ID
    Some("Объект"),                    // Objects
    Some("Связанные поля"),            // Linked Fields
];

/// 对应资源键 `FILE_DELIMITER`
pub const FILE_DELIMITER: char = ';';
/// 对应资源键 `CODE_PAGE`
pub const CODE_PAGE: CodePage = CodePage::Ru;
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
pub const NA: &'static str = "недоступно";
/// 对应资源键 `YES`
pub const YES: &'static str = "Да";
/// 对应资源键 `NO`
pub const NO: &'static str = "Нет";

/// 俄语资源包的内容，对应 Java `LocaleData_ru.RESOURCE_DATA`
/// （即 `getContents()` 的返回值）。
pub static OVERRIDES: LocaleOverrides = LocaleOverrides {
    file_delimiter: Some(FILE_DELIMITER),
    program_name: None,
    file_version: None,
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
        // Java: 该表缺少 Overtime Work，只有 51 项
        assert_eq!(RESOURCE_NAMES_DATA.len(), 51);
    }

    #[test]
    fn sampled_elements_match_java() {
        assert_eq!(TASK_NAMES_DATA[1], Some("Название"));
        assert_eq!(TASK_NAMES_DATA[16], Some("Группа ресурсов"));
        assert_eq!(TASK_NAMES_DATA[31], Some("Запалнированная стоимость"));
        assert_eq!(TASK_NAMES_DATA[34], Some("Разбежнность стоимости"));
        assert_eq!(TASK_NAMES_DATA[45], Some("Разбежнность длительности"));
        assert_eq!(TASK_NAMES_DATA[61], Some("Окончание"));
        assert_eq!(TASK_NAMES_DATA[66], Some("Смешение начала"));
        assert_eq!(TASK_NAMES_DATA[123], Some("Скрытое поле "));
        assert_eq!(TASK_NAMES_DATA[152], Some("Начать"));

        assert_eq!(RESOURCE_NAMES_DATA[1], Some("Имя"));
        assert_eq!(RESOURCE_NAMES_DATA[11], Some("Email"));
        // Java: 缺少 Overtime Work，下标 24 即 Work Variance
        assert_eq!(RESOURCE_NAMES_DATA[24], Some("Разбежность работ"));
        assert_eq!(RESOURCE_NAMES_DATA[29], Some("Стоимость"));
        assert_eq!(RESOURCE_NAMES_DATA[33], Some("Разбежнность стоимости"));
        assert_eq!(RESOURCE_NAMES_DATA[50], Some("Связанные поля"));

        assert_eq!(TIME_UNITS_ARRAY_DATA[0], ["минута"].as_slice());
        assert_eq!(TIME_UNITS_ARRAY_DATA[7], ["каждую минуту"].as_slice());
        assert_eq!(ACCRUE_TYPES_DATA[2], Some("Повседневный"));
        assert_eq!(RELATION_TYPES_DATA[3], Some("НН"));
        assert_eq!(PRIORITY_TYPES_DATA[0], Some("Самый низкий"));
        assert_eq!(CONSTRAINT_TYPES_DATA[1], Some("Как можно позже"));
    }

    #[test]
    fn locale_data_returns_russian_tables() {
        assert_eq!(
            LocaleData::get_string_array(MpxLocale::Ru, LocaleData::RESOURCE_NAMES),
            Some(RESOURCE_NAMES_DATA.as_slice())
        );
        assert_eq!(LocaleData::get_code_page(MpxLocale::Ru), CodePage::Ru);
        assert_eq!(LocaleData::get_string(MpxLocale::Ru, LocaleData::NA), Some("недоступно"));
        assert_eq!(LocaleData::get_string(MpxLocale::Ru, LocaleData::YES), Some("Да"));
        assert_eq!(LocaleData::get_string(MpxLocale::Ru, LocaleData::NO), Some("Нет"));
        // 未覆盖的键回落英文
        assert_eq!(
            LocaleData::get_string(MpxLocale::Ru, LocaleData::FILE_VERSION),
            Some("4.0")
        );
        assert_eq!(
            LocaleData::get_integer(MpxLocale::Ru, LocaleData::DEFAULT_START_TIME),
            Some(480)
        );
    }
}
