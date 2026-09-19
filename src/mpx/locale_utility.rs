//! 对应 Java: `org/mpxj/mpx/LocaleUtility.java`
//!
//! # 移植取舍（`java.util.Locale` / `ResourceBundle` / `DateFormatSymbols`）
//!
//! Java 侧 `LocaleUtility` 依赖 `java.util.Locale` 与 `ResourceBundle`：所有取值都通过
//! `LocaleData.getString/getChar/getObject/getInteger` 从资源包中读取，依据语言加载
//! `LocaleData_<lang>`（`LocaleData_en` 为空，即英文全部使用 `LocaleData` 里的默认值）。
//! Rust 标准库没有对应物，本模块的处理方式：
//!
//! - 用本模块定义的 [`MpxLocale`] 取代 `java.util.Locale`，只承载 MPX 关心的语言代码
//!   （EN/DE/FR/IT/PT/SV/ZH/ES/RU，即 Java 侧 `SUPPORTED_LOCALES`）。
//! - 各语言的资源表（Java 的 `LocaleData_XX`）由同包的 `locale_data_XX` 模块（并行移植）
//!   承载；本模块**不依赖**它们，[`LocaleUtility::set_locale`] 一律落到 Java 的英文默认资源
//!   （`LocaleData.RESOURCE_DATA`）。因此非英文区域设置会拿到英文的货币符号、分隔符、
//!   日期/时间格式等属性；需要完整本地化时由调用方在 `set_locale` 之后用 `locale_data_XX`
//!   的值覆盖。
//! - Java 的 `DateFormatSymbols` 在本类中并未使用（MPX 的日期/数字格式由 `MPXJFormats`
//!   处理），此处不涉及。
//!
//! 未移植的成员：无（`setLocale`、`getSupportedLocales` 均已移植并新增 `MpxLocale`）。

use chrono::NaiveTime;

use crate::code_page::CodePage;
use crate::currency_symbol_position::CurrencySymbolPosition;
use crate::date_order::DateOrder;
use crate::project_date_format::ProjectDateFormat;
use crate::project_properties::ProjectProperties;
use crate::project_time_format::ProjectTimeFormat;

/// MPX 支持的区域设置标识，替代 Java 的 `java.util.Locale`。
///
/// 对应 Java: `LocaleUtility.SUPPORTED_LOCALES` 中的 `new Locale("EN")` 等常量。
///
/// Java 的 `Locale` 还包含国家、变体与语言显示名，MPX 只需要语言代码，故这里只保留
/// 语言维度。Java 侧 `Locale` 实现了 `equals` / `hashCode`，Rust 侧用派生实现对应。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MpxLocale {
    /// 英语。
    En,
    /// 德语。
    De,
    /// 法语。
    Fr,
    /// 意大利语。
    It,
    /// 葡萄牙语。
    Pt,
    /// 瑞典语。
    Sv,
    /// 中文。
    Zh,
    /// 西班牙语。
    Es,
    /// 俄语。
    Ru,
}

impl MpxLocale {
    /// 全部支持的区域设置，顺序与 Java 侧 `SUPPORTED_LOCALES` 一致。
    ///
    /// 对应 Java: `private static final Locale[] SUPPORTED_LOCALES`
    pub const ALL: [MpxLocale; 9] = [
        MpxLocale::En,
        MpxLocale::De,
        MpxLocale::Fr,
        MpxLocale::It,
        MpxLocale::Pt,
        MpxLocale::Sv,
        MpxLocale::Zh,
        MpxLocale::Es,
        MpxLocale::Ru,
    ];

    /// 按语言代码取区域设置，大小写不敏感（Java 的 `Locale(String)` 会把语言码转为小写）。
    ///
    /// 对应 Java: `new Locale("EN")` 之类的构造。无法识别时返回 `None`
    /// （Java 侧会得到一个不在 `SUPPORTED_LOCALES` 中的 `Locale`，其资源解析回落到默认值，
    /// 与这里的英文回落等价）。
    pub fn get_instance(language: &str) -> Option<MpxLocale> {
        MpxLocale::ALL
            .iter()
            .find(|locale| language.eq_ignore_ascii_case(locale.get_language()))
            .copied()
    }

    /// 返回语言代码（小写）。
    ///
    /// 对应 Java: `Locale.getLanguage()`
    pub fn get_language(&self) -> &'static str {
        match self {
            MpxLocale::En => "en",
            MpxLocale::De => "de",
            MpxLocale::Fr => "fr",
            MpxLocale::It => "it",
            MpxLocale::Pt => "pt",
            MpxLocale::Sv => "sv",
            MpxLocale::Zh => "zh",
            MpxLocale::Es => "es",
            MpxLocale::Ru => "ru",
        }
    }
}

impl std::fmt::Display for MpxLocale {
    /// 对应 Java: `Locale.toString()`（只含语言时为小写语言代码）。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_language())
    }
}

/// MPX 文件的区域设置工具方法。
///
/// 对应 Java: `final class LocaleUtility`（私有构造方法禁止实例化，Rust 侧用无字段类型表达）。
pub struct LocaleUtility;

impl LocaleUtility {
    /// 在父文件区域设置更新时，把与区域相关的属性重置为新区域设置的默认值。
    ///
    /// 对应 Java: `public static void setLocale(ProjectProperties properties, Locale locale)`
    ///
    /// 取舍：Java 通过 `ResourceBundle` 按 `locale` 取本地化资源；Rust 侧统一使用英文默认资源
    /// （见文件头说明），`locale` 用于选择资源表（目前除英语外都回落到英文）。
    pub fn set_locale(properties: &mut ProjectProperties, locale: MpxLocale) {
        let data = resources_for(locale);

        // MPX 特有的属性
        properties.set_mpx_delimiter(data.file_delimiter);
        properties.set_mpx_program_name(Some(data.program_name.to_owned()));
        properties.set_mpx_code_page(Some(data.code_page));
        properties.set_date_separator(data.date_separator);
        properties.set_time_separator(data.time_separator);
        properties.set_time_format(Some(data.time_format));
        properties.set_date_order(Some(data.date_order));
        properties.set_am_text(Some(data.am_text.to_owned()));
        properties.set_pm_text(Some(data.pm_text.to_owned()));
        properties.set_date_format(Some(data.date_format));
        properties.set_bar_text_date_format(Some(data.date_format));

        // 通用属性
        properties.set_currency_symbol(Some(data.currency_symbol.to_owned()));
        properties.set_symbol_position(Some(data.currency_symbol_position));
        properties.set_currency_digits(Some(data.currency_digits));
        properties.set_thousands_separator(data.currency_thousands_separator);
        properties.set_decimal_separator(data.currency_decimal_separator);

        // Java: LocalTime.ofSecondOfDay(LocaleData.getInteger(locale, DEFAULT_START_TIME) * 60L)
        // 越界时 Java 抛 DateTimeException，Rust 侧忽略该属性（返回 None）。
        if let Some(time) = time_of_day_from_minutes(data.default_start_time_minutes) {
            properties.set_default_start_time(Some(time));
        }
    }

    /// 返回 MPX 功能支持的区域设置。
    ///
    /// 对应 Java: `public static Locale[] getSupportedLocales()`
    pub fn get_supported_locales() -> &'static [MpxLocale] {
        &MpxLocale::ALL
    }
}

/// 一个区域设置所需的资源集合，对应 Java `LocaleData` 资源包中的键值。
///
/// 这里只包含 [`LocaleUtility::set_locale`] 用到的键；Java 的 `LocaleData` 还包含
/// `TIME_UNITS_ARRAY`、`ACCRUE_TYPES`、`TASK_NAMES` 等，由 `locale_data` / `locale_data_XX`
/// 模块承载（本模块不依赖）。
struct LocaleResources {
    /// 对应资源键 `FILE_DELIMITER`
    file_delimiter: char,
    /// 对应资源键 `PROGRAM_NAME`
    program_name: &'static str,
    /// 对应资源键 `CODE_PAGE`
    code_page: CodePage,
    /// 对应资源键 `CURRENCY_SYMBOL`
    currency_symbol: &'static str,
    /// 对应资源键 `CURRENCY_SYMBOL_POSITION`
    currency_symbol_position: CurrencySymbolPosition,
    /// 对应资源键 `CURRENCY_DIGITS`
    currency_digits: i32,
    /// 对应资源键 `CURRENCY_THOUSANDS_SEPARATOR`
    currency_thousands_separator: char,
    /// 对应资源键 `CURRENCY_DECIMAL_SEPARATOR`
    currency_decimal_separator: char,
    /// 对应资源键 `DATE_ORDER`
    date_order: DateOrder,
    /// 对应资源键 `TIME_FORMAT`
    time_format: ProjectTimeFormat,
    /// 对应资源键 `DEFAULT_START_TIME`（单位：自午夜起的分钟数）
    default_start_time_minutes: i32,
    /// 对应资源键 `DATE_SEPARATOR`
    date_separator: char,
    /// 对应资源键 `TIME_SEPARATOR`
    time_separator: char,
    /// 对应资源键 `AM_TEXT`
    am_text: &'static str,
    /// 对应资源键 `PM_TEXT`
    pm_text: &'static str,
    /// 对应资源键 `DATE_FORMAT`
    date_format: ProjectDateFormat,
}

/// 英文（默认）资源，取自 Java `LocaleData.RESOURCE_DATA`。
///
/// Java 的 `LocaleData_en` 资源包为空，即英语完全使用这组默认值。
static EN_RESOURCES: LocaleResources = LocaleResources {
    file_delimiter: ',',
    program_name: "Microsoft Project for Windows",
    code_page: CodePage::Ansi,
    currency_symbol: "$",
    currency_symbol_position: CurrencySymbolPosition::Before,
    currency_digits: 2,
    currency_thousands_separator: ',',
    currency_decimal_separator: '.',
    date_order: DateOrder::Dmy,
    time_format: ProjectTimeFormat::TwelveHour,
    default_start_time_minutes: 480,
    date_separator: '/',
    time_separator: ':',
    am_text: "am",
    pm_text: "pm",
    date_format: ProjectDateFormat::DdMmYyyy,
};

/// 取区域设置对应的资源集合。
///
/// 对应 Java: `ResourceBundle.getBundle(LocaleData.class.getName(), locale)`
///
/// 取舍：Java 会按语言加载 `LocaleData_de` / `LocaleData_fr` / … 覆盖默认值；Rust 侧这些
/// 本地化资源由并行的 `locale_data_XX` 模块承载，本模块不依赖它们，因此除英语外统一回落到
/// 英文默认资源（见文件头说明）。
fn resources_for(locale: MpxLocale) -> &'static LocaleResources {
    match locale {
        MpxLocale::En => &EN_RESOURCES,
        MpxLocale::De
        | MpxLocale::Fr
        | MpxLocale::It
        | MpxLocale::Pt
        | MpxLocale::Sv
        | MpxLocale::Zh
        | MpxLocale::Es
        | MpxLocale::Ru => &EN_RESOURCES,
    }
}

/// 把「自午夜起的分钟数」转换为时间，对应 Java `LocalTime.ofSecondOfDay(minutes * 60L)`。
///
/// Java 对越界值（负数或 >= 86400 秒）抛 `DateTimeException`；Rust 侧不 panic，返回 `None`。
fn time_of_day_from_minutes(minutes: i32) -> Option<NaiveTime> {
    let seconds = i64::from(minutes) * 60;
    if !(0..86_400).contains(&seconds) {
        return None;
    }

    NaiveTime::from_num_seconds_from_midnight_opt(seconds as u32, 0)
}

/// 大小写不敏感的字符串比较。
///
/// 对应 Java: `java.lang.String.equalsIgnoreCase`（JDK 方法，非 MPXJ 成员）。
/// Java 侧用 `Character.toLowerCase/toUpperCase` 逐字符比较，Rust 侧用
/// `str::to_lowercase` 近似（两者对极少数字符的大小写折叠结果可能不同）。
/// `null` 入参对应 `None`：Java 的 `a.equalsIgnoreCase(null)` 返回 `false`。
pub(crate) fn equals_ignore_case(name: &str, value: Option<&str>) -> bool {
    match value {
        None => false,
        Some(value) => name.to_lowercase() == value.to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Weak;

    use super::*;

    /// 构造一个用于测试的项目属性（无父文件）。
    fn properties() -> ProjectProperties {
        ProjectProperties::new(Weak::new())
    }

    #[test]
    fn mpx_locale_round_trips_through_all() {
        assert_eq!(LocaleUtility::get_supported_locales().len(), 9);
        for locale in MpxLocale::ALL {
            assert_eq!(MpxLocale::get_instance(locale.get_language()), Some(locale));
            // Java: new Locale("EN") 会把语言码转小写，构造参数大小写不敏感。
            assert_eq!(
                MpxLocale::get_instance(&locale.get_language().to_uppercase()),
                Some(locale)
            );
            assert_eq!(locale.to_string(), locale.get_language());
        }
    }

    #[test]
    fn mpx_locale_rejects_unknown_languages() {
        assert_eq!(MpxLocale::get_instance("zz"), None);
        assert_eq!(MpxLocale::get_instance(""), None);
        assert_eq!(MpxLocale::get_instance("eng"), None);
    }

    #[test]
    fn set_locale_applies_english_defaults() {
        let mut properties = properties();
        LocaleUtility::set_locale(&mut properties, MpxLocale::En);

        assert_eq!(properties.get_mpx_delimiter(), ',');
        assert_eq!(properties.get_mpx_program_name(), "Microsoft Project for Windows");
        assert_eq!(properties.get_mpx_code_page(), CodePage::Ansi);
        assert_eq!(properties.get_date_separator(), '/');
        assert_eq!(properties.get_time_separator(), ':');
        assert_eq!(properties.get_currency_symbol(), "$");
        assert_eq!(
            properties.get_default_start_time(),
            NaiveTime::from_hms_opt(8, 0, 0).unwrap()
        );
    }

    #[test]
    fn set_locale_keeps_working_for_non_english_locales() {
        // 取舍：非英文区域设置目前回落到英文默认资源（见文件头）。
        let mut properties = properties();
        LocaleUtility::set_locale(&mut properties, MpxLocale::De);
        assert_eq!(properties.get_mpx_delimiter(), ',');
        assert_eq!(properties.get_mpx_code_page(), CodePage::Ansi);
    }

    #[test]
    fn time_of_day_conversion_matches_java() {
        assert_eq!(time_of_day_from_minutes(480), NaiveTime::from_hms_opt(8, 0, 0));
        assert_eq!(time_of_day_from_minutes(0), NaiveTime::from_hms_opt(0, 0, 0));
        // Java: LocalTime.ofSecondOfDay 对越界值抛 DateTimeException。
        assert_eq!(time_of_day_from_minutes(-1), None);
        assert_eq!(time_of_day_from_minutes(24 * 60), None);
    }

    #[test]
    fn equals_ignore_case_matches_java_semantics() {
        assert!(equals_ignore_case("Start", Some("start")));
        assert!(equals_ignore_case("Start", Some("START")));
        assert!(!equals_ignore_case("Start", Some("End")));
        // Java: "Start".equalsIgnoreCase(null) == false
        assert!(!equals_ignore_case("Start", None));
    }
}
