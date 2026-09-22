//! Corresponds to Java: `org/mpxj/mpx/LocaleData_ru.java`
//!
//! # Porting trade-offs (`ResourceBundle` / `ListResourceBundle`)
//!
//! Java's `LocaleData_ru` is the Russian resource bundle and overrides only some keys; keys it does not
//! override are resolved by `ResourceBundle` by falling back from the base bundle. The Rust side writes the
//! keys this language actually overrides as module-level `pub const`s (scalar constants are named after the
//! keys in Java's `RESOURCE_DATA`, and array constants keep Java's field names), and aggregates them in
//! [`OVERRIDES`] (corresponding to the return value of `getContents()`); keys that are not overridden are
//! `None` and fall back to the English default resources via the `locale_data` module. See the module header
//! of `locale_data` for the value-lookup logic; the protocol methods of `ListResourceBundle` are replaced by
//! constant tables and are not ported.
//!
//! Keys not overridden by this language: `PROGRAM_NAME`, `FILE_VERSION`, `DEFAULT_START_TIME`.
//!
//! Upstream data to be aware of (all copied verbatim from the Java source):
//! - `RESOURCE_NAMES_DATA` has only 51 entries: it is missing `Overtime Work`, so after index 24 the
//!   name table is shifted one position earlier than the English table;
//! - Several entries have obvious spelling/copy issues (such as `Запалнированная`, `Запалнироанное`,
//!   `Finish1` sharing the name `Finish`, and `"Скрытое поле "` with a trailing space).
//!
//! Skipped Java methods/members: none.

use crate::code_page::CodePage;
use crate::currency_symbol_position::CurrencySymbolPosition;
use crate::date_order::DateOrder;
use crate::mpx::locale_data::LocaleOverrides;
use crate::project_date_format::ProjectDateFormat;
use crate::project_time_format::ProjectTimeFormat;

/// Russian time unit alias table.
///
/// Corresponds to Java: `LocaleData_ru.TIME_UNITS_ARRAY_DATA` (Java literal order: minutes, hours, days,
/// weeks, months, years, percent, followed by the "elapsed" units; index 5 is years and index 6 is percent,
/// which differs from the `TimeUnit` value order; copied verbatim from Java here).
pub const TIME_UNITS_ARRAY_DATA: [&'static [&'static str]; 14] = [
    &["минута"],           // 0 TimeUnit.MINUTES
    &["час"],              // 1 TimeUnit.HOURS
    &["день"],             // 2 TimeUnit.DAYS
    &["неделя"],           // 3 TimeUnit.WEEKS
    &["месяц"],            // 4 TimeUnit.MONTHS
    &["год"],              // 5 TimeUnit.PERCENT (Java literal position)
    &["%"],                // 6 TimeUnit.YEARS (Java literal position)
    &["каждую минуту"],    // 7 TimeUnit.ELAPSED_MINUTES
    &["каждый час"],       // 8 TimeUnit.ELAPSED_HOURS
    &["каждый день"],      // 9 TimeUnit.ELAPSED_DAYS
    &["каждую неделю"],    // 10 TimeUnit.ELAPSED_WEEKS
    &["каждый месяц"],     // 11 TimeUnit.ELAPSED_MONTHS
    &["каждый год"],       // 12 TimeUnit.ELAPSED_YEARS
    &["каждый %"],         // 13 TimeUnit.ELAPSED_PERCENT
];

/// Name table for the Russian resource key `ACCRUE_TYPES`.
///
/// Corresponds to Java: `LocaleData_ru.ACCRUE_TYPES_DATA`.
pub const ACCRUE_TYPES_DATA: [Option<&'static str>; 3] =
    [Some("Начало"), Some("Конец"), Some("Повседневный")];

/// Name table for the Russian resource key `RELATION_TYPES`.
///
/// Corresponds to Java: `LocaleData_ru.RELATION_TYPES_DATA`.
pub const RELATION_TYPES_DATA: [Option<&'static str>; 4] =
    [Some("КК"), Some("КН"), Some("НК"), Some("НН")];

/// Name table for the Russian resource key `PRIORITY_TYPES`.
///
/// Corresponds to Java: `LocaleData_ru.PRIORITY_TYPES_DATA`.
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

/// Name table for the Russian resource key `CONSTRAINT_TYPES`.
///
/// Corresponds to Java: `LocaleData_ru.CONSTRAINT_TYPES_DATA`.
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

/// Name table for the Russian resource key `TASK_NAMES` (indices correspond to MPX task field numbers; end-of-line comments are the English default names).
///
/// Corresponds to Java: `LocaleData_ru.TASK_NAMES_DATA`.
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
    Some("Запалнированная стоимость"),           // Baseline Cost (spelling in the Java source)
    Some("Реальная стоимость"),                  // Actual Cost
    Some("Ожидаемая стоимость"),                 // Remaining Cost
    Some("Разбежнность стоимости"),              // Cost Variance (spelled with a double н in the Java source)
    Some("Фиксированная стоимость"),             // Fixed Cost
    Some("Стоимость1"),                          // Cost1
    Some("Стоимость2"),                          // Cost2
    Some("Стоимость3"),                          // Cost3
    None,                                        // -
    Some("Длительность"),                        // Duration
    Some("Запалнированная длительность"),        // Baseline Duration (spelling in the Java source)
    Some("Реальная длительность"),               // Actual Duration
    Some("Ожидаемая длительность"),              // Remaining Duration
    Some("% завершено"),                         // % Complete
    Some("Разбежнность длительности"),           // Duration Variance (spelled with a double н in the Java source)
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
    Some("Запалнироанное начало"),               // Baseline Start (spelling in the Java source)
    Some("Запланированное окончание"),           // Baseline Finish
    Some("Реальное начало"),                     // Actual Start
    Some("Реальное окончание"),                  // Actual Finish
    Some("Начало1"),                             // Start1
    Some("Окончание"),                           // Finish1 (same name as Finish in the Java source)
    Some("Начало2"),                             // Start2
    Some("Окончание2"),                          // Finish2
    Some("Начало3"),                             // Start3
    Some("Окончание3"),                          // Finish3
    Some("Смешение начала"),                     // Start Variance (ш in the Java source)
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
    Some("Скрытое поле "),                       // Hide Bar (trailing space in the Java source)
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

/// Name table for the Russian resource key `RESOURCE_NAMES` (indices correspond to MPX resource field numbers; end-of-line comments are the English default names).
///
/// Corresponds to Java: `LocaleData_ru.RESOURCE_NAMES_DATA`. In the Java source this table has only 51 entries
/// (it is missing `Overtime Work`), so from index 24 it is shifted one position earlier than the English table.
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
    Some("Разбежность работ"),         // Work Variance (Overtime Work missing in the Java source)
    Some("% работ завершено"),         // % Work Complete
    None,                              // -
    None,                              // -
    None,                              // -
    Some("Стоимость"),                 // Cost
    Some("Запалнированная стоимость"), // Baseline Cost (spelling in the Java source)
    Some("Реальная стоимость"),        // Actual Cost
    Some("Ожидаемая стоимость"),       // Remaining Cost
    Some("Разбежнность стоимости"),    // Cost Variance (spelled with a double н in the Java source)
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

/// Corresponds to the resource key `FILE_DELIMITER`
pub const FILE_DELIMITER: char = ';';
/// Corresponds to the resource key `CODE_PAGE`
pub const CODE_PAGE: CodePage = CodePage::Ru;
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
pub const NA: &'static str = "недоступно";
/// Corresponds to the resource key `YES`
pub const YES: &'static str = "Да";
/// Corresponds to the resource key `NO`
pub const NO: &'static str = "Нет";

/// Contents of the Russian resource bundle, corresponding to Java `LocaleData_ru.RESOURCE_DATA`
/// (that is, the return value of `getContents()`).
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
        // Java: this table is missing Overtime Work and has only 51 entries
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
        // Java: Overtime Work is missing, so index 24 is Work Variance
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
        // Keys not overridden fall back to English
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
