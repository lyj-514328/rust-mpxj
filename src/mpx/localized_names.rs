//! Localized text tables for month and weekday names.
//!
//! Corresponds to Java: the JDK `java.time` localized text used by `MPXJFormats` through
//! `DateTimeFormatter.withLocale(m_locale)` (Java has no standalone class; this module is the Rust-side data holder).
//!
//! # Data source
//!
//! Java's localized month / weekday names come from the **JDK `java.time`**; `org.mpxj.mpx.LocaleData`
//! does not contain these names. The Rust side cannot access JDK data, so tables for the 9 supported
//! languages are built here; each entry is taken from JDK 25's output for `MMM` / `MMMM` / `EEE` / `EEEE`
//! under the corresponding `Locale` (consistent with `MPXJFormats`).
//!
//! # Index conventions
//!
//! - Month table indices `0..12` correspond to January..December;
//! - Weekday table indices `0..7` correspond to Monday..Sunday, matching [`chrono::Weekday::num_days_from_monday`].
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `withLocale(locale)` dynamically fetches JDK text | [`names_for_locale`] static dispatch | Data is frozen to the JDK output and values match; manual sync is needed when the JDK version changes |
//! | Only the `Locale` language dimension | [`MpxLocale`] | Matches `LocaleUtility.SUPPORTED_LOCALES` |
//!
//! Skipped Java methods/members: none (Java has no corresponding class; this module is merely the Rust-side data holder).

use crate::mpx::locale_utility::MpxLocale;

/// A set of localized text constants.
///
/// Trade-off: Java obtains the localized JDK `java.time` text through `DateTimeFormatter.withLocale(m_locale)`;
/// the Rust side has no such data source, so the text is frozen into this module's constant tables per language.
#[derive(Debug)]
pub(crate) struct TextTable {
    /// Abbreviated month names (corresponding to the JDK's `MMM`)
    pub(crate) months_abbreviated: [&'static str; 12],
    /// Full month names (corresponding to the JDK's `MMMM`)
    pub(crate) months_full: [&'static str; 12],
    /// Abbreviated weekday names (indices match `Weekday::num_days_from_monday`, corresponding to `EEE`)
    pub(crate) weekdays_abbreviated: [&'static str; 7],
    /// Full weekday names (corresponding to the JDK's `EEEE`)
    pub(crate) weekdays_full: [&'static str; 7],
}

/// English AM text (a fallback beyond the project properties' AM text).
pub(crate) const AM: &str = "am";
/// English PM text (a fallback beyond the project properties' PM text).
pub(crate) const PM: &str = "pm";

/// Return the text constant table for a locale.
///
/// Corresponds to Java: `DateTimeFormatter.withLocale(m_locale)` (month / weekday name text).
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

/// English (JDK `Locale.ENGLISH`).
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

/// German (`Locale.GERMAN`).
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

/// French (`Locale.FRENCH`).
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

/// Italian (`Locale.ITALIAN`).
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

/// Portuguese (`new Locale("pt")`).
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

/// Swedish (`new Locale("sv")`).
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

/// Chinese (`Locale.CHINESE`).
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

/// Spanish (`new Locale("es")`).
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

/// Russian (`new Locale("ru")`).
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

    /// Every supported locale must dispatch to its own table, and values such as English and German must differ.
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

        // German, French, and Italian abbreviations all differ (confirming they do not share an English table)
        let de = names_for_locale(MpxLocale::De).months_abbreviated[0];
        let fr = names_for_locale(MpxLocale::Fr).months_abbreviated[0];
        let it = names_for_locale(MpxLocale::It).months_abbreviated[0];
        assert_ne!(de, fr);
        assert_ne!(de, it);
        assert_ne!(fr, it);
    }

    /// Weekday table index 0 is Monday (matching `Weekday::num_days_from_monday`).
    #[test]
    fn weekday_tables_start_on_monday() {
        assert_eq!(names_for_locale(MpxLocale::En).weekdays_abbreviated[0], "Mon");
        assert_eq!(names_for_locale(MpxLocale::De).weekdays_abbreviated[0], "Mo.");
        // 2024-01-07 is a Sunday, corresponding to index 6
        assert_eq!(names_for_locale(MpxLocale::En).weekdays_full[6], "Sunday");
        assert_eq!(names_for_locale(MpxLocale::Zh).weekdays_full[6], "星期日");
    }

    /// All tables cover exactly 12 months / 7 days, avoiding empty-string placeholders.
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
