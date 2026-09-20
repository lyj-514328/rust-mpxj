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
//!   （EN/DE/FR/IT/PT/SV/ZH/ES/RU，即 Java 侧 `SUPPORTED_LOCALES`）；
//! - 资源包的「变体覆盖 + 父包回落」语义由 [`crate::mpx::locale_data::LocaleData`] 承载
//!   （各语言变体的 `OVERRIDES` 覆盖 [`crate::mpx::locale_data::RESOURCE_DATA`] 的默认值），
//!   [`LocaleUtility::set_locale`] 逐键调用其 `get_string` / `get_char` / `get_integer` /
//!   `get_code_page` 等，与 Java 的 `ResourceBundle` 取值一一对应，因此德语、法语等
//!   非英语区域设置会得到各自的货币符号、分隔符、日期 / 时间格式等属性；
//! - Java 的 `DateFormatSymbols` 在本类中并未使用（MPX 的日期/数字格式由 `MPXJFormats`
//!   处理），此处不涉及。
//!
//! 未移植的成员：无（`setLocale`、`getSupportedLocales` 均已移植并新增 `MpxLocale`）。

use chrono::NaiveTime;

use crate::mpx::locale_data::LocaleData;
use crate::project_properties::ProjectProperties;

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
    /// 取舍：Java 经 `ResourceBundle` 按 `locale` 取本地化资源（变体覆盖 + 英文默认值回落）；
    /// Rust 侧由 [`LocaleData`] 的同名取键方法承担该语义，逐项对应 Java 的
    /// `LocaleData.getString/getChar/getObject/getInteger`。
    ///
    /// 注：Java 的 `getChar` / `getObject` 对缺失键抛 `MissingResourceException`（这些键在
    /// 资源包中必定存在）；Rust 侧的 `get_char` 对未取到值返回 `None`，此处为不 panic
    /// 而只在取到值时设置。
    pub fn set_locale(properties: &mut ProjectProperties, locale: MpxLocale) {
        // ---- MPX 特有的属性 ----
        // Java: properties.setMpxDelimiter(LocaleData.getChar(locale, LocaleData.FILE_DELIMITER));
        if let Some(delimiter) = LocaleData::get_char(locale, LocaleData::FILE_DELIMITER) {
            properties.set_mpx_delimiter(delimiter);
        }

        // Java: properties.setMpxProgramName(LocaleData.getString(locale, LocaleData.PROGRAM_NAME));
        properties.set_mpx_program_name(
            LocaleData::get_string(locale, LocaleData::PROGRAM_NAME).map(str::to_owned),
        );

        // Java: properties.setMpxCodePage((CodePage) LocaleData.getObject(locale, LocaleData.CODE_PAGE));
        properties.set_mpx_code_page(Some(LocaleData::get_code_page(locale)));

        // Java: properties.setDateSeparator(LocaleData.getChar(locale, LocaleData.DATE_SEPARATOR));
        if let Some(separator) = LocaleData::get_char(locale, LocaleData::DATE_SEPARATOR) {
            properties.set_date_separator(separator);
        }

        // Java: properties.setTimeSeparator(LocaleData.getChar(locale, LocaleData.TIME_SEPARATOR));
        if let Some(separator) = LocaleData::get_char(locale, LocaleData::TIME_SEPARATOR) {
            properties.set_time_separator(separator);
        }

        // Java: properties.setTimeFormat((ProjectTimeFormat) LocaleData.getObject(locale, LocaleData.TIME_FORMAT));
        properties.set_time_format(Some(LocaleData::get_time_format(locale)));

        // Java: properties.setDateOrder((DateOrder) LocaleData.getObject(locale, LocaleData.DATE_ORDER));
        properties.set_date_order(Some(LocaleData::get_date_order(locale)));

        // Java: properties.setAMText(LocaleData.getString(locale, LocaleData.AM_TEXT));
        properties
            .set_am_text(LocaleData::get_string(locale, LocaleData::AM_TEXT).map(str::to_owned));

        // Java: properties.setPMText(LocaleData.getString(locale, LocaleData.PM_TEXT));
        properties
            .set_pm_text(LocaleData::get_string(locale, LocaleData::PM_TEXT).map(str::to_owned));

        // Java: properties.setDateFormat((ProjectDateFormat) LocaleData.getObject(locale, LocaleData.DATE_FORMAT));
        //       properties.setBarTextDateFormat((ProjectDateFormat) LocaleData.getObject(locale, LocaleData.DATE_FORMAT));
        let date_format = LocaleData::get_date_format(locale);
        properties.set_date_format(Some(date_format));
        properties.set_bar_text_date_format(Some(date_format));

        // ---- 通用属性 ----
        // Java: properties.setCurrencySymbol(LocaleData.getString(locale, LocaleData.CURRENCY_SYMBOL));
        properties.set_currency_symbol(
            LocaleData::get_string(locale, LocaleData::CURRENCY_SYMBOL).map(str::to_owned),
        );

        // Java: properties.setSymbolPosition((CurrencySymbolPosition) LocaleData.getObject(locale, LocaleData.CURRENCY_SYMBOL_POSITION));
        properties.set_symbol_position(Some(LocaleData::get_currency_symbol_position(locale)));

        // Java: properties.setCurrencyDigits(LocaleData.getInteger(locale, LocaleData.CURRENCY_DIGITS));
        properties
            .set_currency_digits(LocaleData::get_integer(locale, LocaleData::CURRENCY_DIGITS));

        // Java: properties.setThousandsSeparator(LocaleData.getChar(locale, LocaleData.CURRENCY_THOUSANDS_SEPARATOR));
        if let Some(separator) =
            LocaleData::get_char(locale, LocaleData::CURRENCY_THOUSANDS_SEPARATOR)
        {
            properties.set_thousands_separator(separator);
        }

        // Java: properties.setDecimalSeparator(LocaleData.getChar(locale, LocaleData.CURRENCY_DECIMAL_SEPARATOR));
        if let Some(separator) =
            LocaleData::get_char(locale, LocaleData::CURRENCY_DECIMAL_SEPARATOR)
        {
            properties.set_decimal_separator(separator);
        }

        // Java: LocalTime.ofSecondOfDay(LocaleData.getInteger(locale, DEFAULT_START_TIME) * 60L)
        // 越界时 Java 抛 DateTimeException，Rust 侧忽略该属性（返回 None）。
        if let Some(time) = LocaleData::get_integer(locale, LocaleData::DEFAULT_START_TIME)
            .and_then(time_of_day_from_minutes)
        {
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
    use crate::code_page::CodePage;

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

    /// 非英语区域设置按其本地化资源取值（修复前一律回落到英文默认资源）。
    ///
    /// 期望值与 Java 的 `LocaleData_de` / `LocaleData_fr` 一致（见 `locale_data_de` /
    /// `locale_data_fr` 的常量）。
    #[test]
    fn set_locale_applies_localised_values_for_non_english_locales() {
        // 德语：分隔符 ';'、日期分隔符 '.'、24 小时制、货币符号后置且带空格
        let mut german_properties = properties();
        LocaleUtility::set_locale(&mut german_properties, MpxLocale::De);

        assert_eq!(german_properties.get_mpx_delimiter(), ';');
        assert_eq!(german_properties.get_mpx_code_page(), CodePage::Ansi);
        assert_eq!(german_properties.get_date_separator(), '.');
        assert_eq!(german_properties.get_time_separator(), ':');
        assert_eq!(german_properties.get_thousands_separator(), '.');
        assert_eq!(german_properties.get_decimal_separator(), ',');
        assert_eq!(german_properties.get_currency_symbol(), "€");
        assert_eq!(
            german_properties.get_default_start_time(),
            NaiveTime::from_hms_opt(8, 0, 0).unwrap()
        );

        // 法语：分隔符 ';'、货币符号为空字符串
        let mut french_properties = properties();
        LocaleUtility::set_locale(&mut french_properties, MpxLocale::Fr);

        assert_eq!(french_properties.get_mpx_delimiter(), ';');
        assert_eq!(french_properties.get_currency_symbol(), "");
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
