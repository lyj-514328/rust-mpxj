//! 月份名 / 星期名的本地化文本表。
//!
//! 对应 Java: `MPXJFormats` 中 `DateTimeFormatter.withLocale(m_locale)` 所取用的
//! JDK `java.time` 本地化文本（Java 侧无独立类，本模块是 Rust 侧的数据承载）。
//!
//! # 数据来源
//!
//! Java 的本地化月份 / 星期名来自 **JDK `java.time`**，`org.mpxj.mpx.LocaleData`
//! 里并没有这些名字。Rust 侧无法访问 JDK 数据，因此这里自建 9 个受支持语种的表；
//! 每个条目的取值均取自 JDK 25 在对应 `Locale` 下对 `MMM` / `MMMM` / `EEE` / `EEEE`
//! 的输出（与 `MPXJFormats` 一致）。
//!
//! # 下标约定
//!
//! - 月份表下标 `0..12` 对应 1 月..12 月；
//! - 星期表下标 `0..7` 对应周一..周日，与 [`chrono::Weekday::num_days_from_monday`] 一致。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `withLocale(locale)` 动态取 JDK 文本 | [`names_for_locale`] 静态分派 | 数据已按 JDK 输出固化，取值一致；JDK 版本变化时需人工同步 |
//! | 仅 `Locale` 语言维度 | [`MpxLocale`] | 与 `LocaleUtility.SUPPORTED_LOCALES` 一致 |
//!
//! 跳过的 Java 成员：无（Java 侧没有对应类，本模块只是 Rust 侧的数据承载）。

use crate::mpx::locale_utility::MpxLocale;

/// 一组本地化文本常量。
///
/// 取舍：Java 通过 `DateTimeFormatter.withLocale(m_locale)` 取 JDK `java.time` 的本地化文本；
/// Rust 侧没有该数据源，故按语种固化为本模块的常量表。
#[derive(Debug)]
pub(crate) struct TextTable {
    /// 月份缩写（对应 JDK 的 `MMM`）
    pub(crate) months_abbreviated: [&'static str; 12],
    /// 月份全称（对应 JDK 的 `MMMM`）
    pub(crate) months_full: [&'static str; 12],
    /// 星期缩写（下标与 `Weekday::num_days_from_monday` 一致，对应 `EEE`）
    pub(crate) weekdays_abbreviated: [&'static str; 7],
    /// 星期全称（对应 JDK 的 `EEEE`）
    pub(crate) weekdays_full: [&'static str; 7],
}

/// 英文上午文本（作为项目属性 AM 文本之外的兜底）。
pub(crate) const AM: &str = "am";
/// 英文下午文本（作为项目属性 PM 文本之外的兜底）。
pub(crate) const PM: &str = "pm";

/// 取区域设置对应的文本常量表。
///
/// 对应 Java: `DateTimeFormatter.withLocale(m_locale)`（月份 / 星期名文本）。
pub(crate) fn names_for_locale(locale: MpxLocale) -> &'static TextTable {
    match locale {
        MpxLocale::En => &EN,
        MpxLocale::De => &DE,
        MpxLocale::Fr => &FR,
        MpxLocale::It => &IT,
        MpxLocale::Pt => &PT,
        MpxLocale::Sv => &SV,
        MpxLocale::Zh => &ZH,
        MpxLocale::Es => &ES,
        MpxLocale::Ru => &RU,
    }
}

/// 英语（JDK `Locale.ENGLISH`）。
static EN: TextTable = TextTable {
    months_abbreviated: [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ],
    months_full: [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ],
    weekdays_abbreviated: ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
    weekdays_full: [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ],
};

/// 德语（`Locale.GERMAN`）。
static DE: TextTable = TextTable {
    months_abbreviated: [
        "Jan.", "Feb.", "März", "Apr.", "Mai", "Juni", "Juli", "Aug.", "Sept.", "Okt.", "Nov.",
        "Dez.",
    ],
    months_full: [
        "Januar",
        "Februar",
        "März",
        "April",
        "Mai",
        "Juni",
        "Juli",
        "August",
        "September",
        "Oktober",
        "November",
        "Dezember",
    ],
    weekdays_abbreviated: ["Mo.", "Di.", "Mi.", "Do.", "Fr.", "Sa.", "So."],
    weekdays_full: [
        "Montag",
        "Dienstag",
        "Mittwoch",
        "Donnerstag",
        "Freitag",
        "Samstag",
        "Sonntag",
    ],
};

/// 法语（`Locale.FRENCH`）。
static FR: TextTable = TextTable {
    months_abbreviated: [
        "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
        "déc.",
    ],
    months_full: [
        "janvier",
        "février",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "août",
        "septembre",
        "octobre",
        "novembre",
        "décembre",
    ],
    weekdays_abbreviated: ["lun.", "mar.", "mer.", "jeu.", "ven.", "sam.", "dim."],
    weekdays_full: [
        "lundi",
        "mardi",
        "mercredi",
        "jeudi",
        "vendredi",
        "samedi",
        "dimanche",
    ],
};

/// 意大利语（`Locale.ITALIAN`）。
static IT: TextTable = TextTable {
    months_abbreviated: [
        "gen", "feb", "mar", "apr", "mag", "giu", "lug", "ago", "set", "ott", "nov", "dic",
    ],
    months_full: [
        "gennaio",
        "febbraio",
        "marzo",
        "aprile",
        "maggio",
        "giugno",
        "luglio",
        "agosto",
        "settembre",
        "ottobre",
        "novembre",
        "dicembre",
    ],
    weekdays_abbreviated: ["lun", "mar", "mer", "gio", "ven", "sab", "dom"],
    weekdays_full: [
        "lunedì",
        "martedì",
        "mercoledì",
        "giovedì",
        "venerdì",
        "sabato",
        "domenica",
    ],
};

/// 葡萄牙语（`new Locale("pt")`）。
static PT: TextTable = TextTable {
    months_abbreviated: [
        "jan.", "fev.", "mar.", "abr.", "mai.", "jun.", "jul.", "ago.", "set.", "out.", "nov.",
        "dez.",
    ],
    months_full: [
        "janeiro",
        "fevereiro",
        "março",
        "abril",
        "maio",
        "junho",
        "julho",
        "agosto",
        "setembro",
        "outubro",
        "novembro",
        "dezembro",
    ],
    weekdays_abbreviated: ["seg.", "ter.", "qua.", "qui.", "sex.", "sáb.", "dom."],
    weekdays_full: [
        "segunda-feira",
        "terça-feira",
        "quarta-feira",
        "quinta-feira",
        "sexta-feira",
        "sábado",
        "domingo",
    ],
};

/// 瑞典语（`new Locale("sv")`）。
static SV: TextTable = TextTable {
    months_abbreviated: [
        "jan.", "feb.", "mars", "apr.", "maj", "juni", "juli", "aug.", "sep.", "okt.", "nov.",
        "dec.",
    ],
    months_full: [
        "januari",
        "februari",
        "mars",
        "april",
        "maj",
        "juni",
        "juli",
        "augusti",
        "september",
        "oktober",
        "november",
        "december",
    ],
    weekdays_abbreviated: ["mån", "tis", "ons", "tors", "fre", "lör", "sön"],
    weekdays_full: [
        "måndag",
        "tisdag",
        "onsdag",
        "torsdag",
        "fredag",
        "lördag",
        "söndag",
    ],
};

/// 中文（`Locale.CHINESE`）。
static ZH: TextTable = TextTable {
    months_abbreviated: [
        "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
    ],
    months_full: [
        "一月",
        "二月",
        "三月",
        "四月",
        "五月",
        "六月",
        "七月",
        "八月",
        "九月",
        "十月",
        "十一月",
        "十二月",
    ],
    weekdays_abbreviated: ["周一", "周二", "周三", "周四", "周五", "周六", "周日"],
    weekdays_full: [
        "星期一",
        "星期二",
        "星期三",
        "星期四",
        "星期五",
        "星期六",
        "星期日",
    ],
};

/// 西班牙语（`new Locale("es")`）。
static ES: TextTable = TextTable {
    months_abbreviated: [
        "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic",
    ],
    months_full: [
        "enero",
        "febrero",
        "marzo",
        "abril",
        "mayo",
        "junio",
        "julio",
        "agosto",
        "septiembre",
        "octubre",
        "noviembre",
        "diciembre",
    ],
    weekdays_abbreviated: ["lun", "mar", "mié", "jue", "vie", "sáb", "dom"],
    weekdays_full: [
        "lunes",
        "martes",
        "miércoles",
        "jueves",
        "viernes",
        "sábado",
        "domingo",
    ],
};

/// 俄语（`new Locale("ru")`）。
static RU: TextTable = TextTable {
    months_abbreviated: [
        "янв.", "февр.", "мар.", "апр.", "мая", "июн.", "июл.", "авг.", "сент.", "окт.", "нояб.",
        "дек.",
    ],
    months_full: [
        "января",
        "февраля",
        "марта",
        "апреля",
        "мая",
        "июня",
        "июля",
        "августа",
        "сентября",
        "октября",
        "ноября",
        "декабря",
    ],
    weekdays_abbreviated: ["пн", "вт", "ср", "чт", "пт", "сб", "вс"],
    weekdays_full: [
        "понедельник",
        "вторник",
        "среда",
        "четверг",
        "пятница",
        "суббота",
        "воскресенье",
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    /// 每个受支持语种都必须分派到各自的表，且英文与德语等取值不同。
    #[test]
    fn every_supported_locale_has_its_own_table() {
        assert_eq!(names_for_locale(MpxLocale::En).months_abbreviated[0], "Jan");
        assert_eq!(names_for_locale(MpxLocale::De).months_abbreviated[0], "Jan.");
        assert_eq!(names_for_locale(MpxLocale::Fr).months_abbreviated[0], "janv.");
        assert_eq!(names_for_locale(MpxLocale::It).months_abbreviated[0], "gen");
        assert_eq!(names_for_locale(MpxLocale::Pt).months_abbreviated[0], "jan.");
        assert_eq!(names_for_locale(MpxLocale::Sv).months_abbreviated[0], "jan.");
        assert_eq!(names_for_locale(MpxLocale::Zh).months_abbreviated[0], "1月");
        assert_eq!(names_for_locale(MpxLocale::Es).months_abbreviated[0], "ene");
        assert_eq!(names_for_locale(MpxLocale::Ru).months_abbreviated[0], "янв.");

        // 德语、法语、意大利语的缩写互不相同（确认不是共用一张英文表）
        let de = names_for_locale(MpxLocale::De).months_abbreviated[0];
        let fr = names_for_locale(MpxLocale::Fr).months_abbreviated[0];
        let it = names_for_locale(MpxLocale::It).months_abbreviated[0];
        assert_ne!(de, fr);
        assert_ne!(de, it);
        assert_ne!(fr, it);
    }

    /// 星期表下标 0 是周一（与 `Weekday::num_days_from_monday` 一致）。
    #[test]
    fn weekday_tables_start_on_monday() {
        assert_eq!(names_for_locale(MpxLocale::En).weekdays_abbreviated[0], "Mon");
        assert_eq!(names_for_locale(MpxLocale::De).weekdays_abbreviated[0], "Mo.");
        // 2024-01-07 是周日，对应下标 6
        assert_eq!(names_for_locale(MpxLocale::En).weekdays_full[6], "Sunday");
        assert_eq!(names_for_locale(MpxLocale::Zh).weekdays_full[6], "星期日");
    }

    /// 所有表都恰好覆盖 12 个月 / 7 天，避免出现空串占位。
    #[test]
    fn all_tables_are_complete() {
        for locale in MpxLocale::ALL {
            let table = names_for_locale(locale);
            for name in table.months_abbreviated.iter().chain(&table.months_full) {
                assert!(!name.is_empty(), "{locale} 的月份名不应为空");
            }
            for name in table
                .weekdays_abbreviated
                .iter()
                .chain(&table.weekdays_full)
            {
                assert!(!name.is_empty(), "{locale} 的星期名不应为空");
            }
        }
    }
}
