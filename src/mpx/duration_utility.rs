//! Corresponds to Java: `org/mpxj/mpx/DurationUtility.java`
//!
//! # Porting trade-offs (`java.text.NumberFormat`)
//!
//! Java uses `java.text.NumberFormat` (on the MPX side, [`MPXJNumberFormat`]) to parse the numeric prefix of duration text,
//! so the decimal separator and grouping separator vary with the locale (German / French use `,` as the decimal separator).
//! The Rust side likewise takes "the format object passed by the caller + the locale" as input; the format object is
//! [`crate::mpx::mpxj_formats::MPXJFormats::get_duration_decimal_format`]
//! (pattern `#.##` + the locale's separator), so fractional durations such as `1,5t` parse the same as in Java.
//!
//! Java throws an exception on parse failure (`ParseException`) or when the units cannot be recognized (`MPXJException`);
//! the Rust side uniformly returns `None`.
//!
//! Skipped Java methods/members: none.

use crate::duration::Duration;
use crate::mpx::locale_utility::MpxLocale;
use crate::mpx::mpxj_number_format::MPXJNumberFormat;
use crate::mpx::time_unit_utility::TimeUnitUtility;
use crate::time_unit::TimeUnit;

/// Utility methods for managing `Duration` instances in MPX files.
///
/// Corresponds to Java: `final class DurationUtility` (a private constructor prevents instantiation; the Rust side uses a fieldless type).
pub struct DurationUtility;

impl DurationUtility {
    /// Parse MPX duration text (such as `"8h"` / `"2d"` / `"8"`) into a `Duration` instance.
    ///
    /// Corresponds to Java: `public static Duration getInstance(String dur, NumberFormat format, Locale locale)`
    ///
    /// Java's `dur` is a `String` (`null` triggers `NullPointerException`); the Rust side uses
    /// `Option<&str>`, and `None` returns `None`. With no unit suffix it is treated as days (for MPX3 compatibility).
    /// The number is parsed with `format` ([`MPXJNumberFormat`], that is, Java's `m_formats.getDurationDecimalFormat()`),
    /// and a parse failure returns `None` (Java throws `ParseException` -> `MPXJException`).
    pub fn get_instance(
        dur: Option<&str>,
        format: &MPXJNumberFormat,
        locale: MpxLocale,
    ) -> Option<Duration> {
        let chars: Vec<char> = dur?.chars().collect();
        if chars.is_empty() {
            // Java: format.parse("") throws ParseException
            return None;
        }

        let last_index = chars.len() - 1;
        let mut index = last_index;

        // Java: while ((index > 0) && !Character.isDigit(dur.charAt(index))) --index;
        // Note: Java's Character.isDigit recognizes Unicode digits; here ASCII digit detection is used.
        while index > 0 && !chars[index].is_ascii_digit() {
            index -= 1;
        }

        if index == last_index {
            // No unit suffix: treat as days for MPX3 compatibility
            let duration = parse_number(format, &chars[..])?;
            Some(Duration::get_instance(duration, TimeUnit::Days))
        } else {
            index += 1;
            let duration = parse_number(format, &chars[..index])?;

            // Java: while ((index < lastIndex) && Character.isWhitespace(dur.charAt(index))) ++index;
            while index < last_index && chars[index].is_whitespace() {
                index += 1;
            }

            let units_text: String = chars[index..].iter().collect();
            let units = TimeUnitUtility::get_instance(Some(units_text.as_str()), locale)?;

            Some(Duration::get_instance(duration, units))
        }
    }
}

/// Parse the numeric prefix, corresponding to Java's `NumberFormat.parse(...)`.
///
/// Java's `NumberFormat.parse` allows leading whitespace, allows grouping separators, and allows stopping after the number;
/// [`MPXJNumberFormat::parse`] behaves the same way (see that module's header), and a parse failure returns `None`
/// (Java throws `ParseException`).
fn parse_number(format: &MPXJNumberFormat, chars: &[char]) -> Option<f64> {
    let text: String = chars.iter().collect();
    format.parse(Some(text.as_str()))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::mpx::locale_utility::LocaleUtility;
    use crate::mpx::mpxj_formats::MPXJFormats;
    use crate::project_file::ProjectFile;

    /// Build the format set for a given locale, equivalent to what the reader does in `read`
    /// (first [`LocaleUtility::set_locale`], then `new MPXJFormats`).
    ///
    /// The returned `ProjectFile` must be held by the caller: `MPXJFormats` only stores a weak reference.
    fn formats_for(locale: MpxLocale) -> (Rc<RefCell<ProjectFile>>, MPXJFormats) {
        let file = ProjectFile::new();
        LocaleUtility::set_locale(file.borrow_mut().get_project_properties_mut(), locale);
        let formats = MPXJFormats::new(locale, "NA", Rc::downgrade(&file));
        (file, formats)
    }

    /// Parse a piece of text with the "duration decimal format" (equivalent to Java's
    /// `DurationUtility.getInstance(text, m_formats.getDurationDecimalFormat(), locale)`).
    fn parse(formats: &MPXJFormats, locale: MpxLocale, text: &str) -> Option<Duration> {
        DurationUtility::get_instance(Some(text), formats.get_duration_decimal_format(), locale)
    }

    /// Write a duration back in MPX text form (corresponding to the `value + unitName` assembled by the MPX writer
    /// using localized time unit names).
    ///
    /// Note: `TimeUnit::get_name()` returns `"mo"` / `"emo"` for MONTHS / ELAPSED_MONTHS,
    /// whereas the MPX English name table uses `"mon"` / `"emon"` (see `time_unit_utility`),
    /// so these two need a separate mapping.
    fn to_mpx_string(duration: Duration) -> String {
        let units = match duration.get_units() {
            TimeUnit::Months => "mon",
            TimeUnit::ElapsedMonths => "emon",
            units => units.get_name(),
        };
        format!("{}{}", duration.get_duration(), units)
    }

    #[test]
    fn duration_with_unit_suffix_round_trips() {
        let (_file, formats) = formats_for(MpxLocale::En);

        let cases = [
            ("8h", 8.0, TimeUnit::Hours),
            ("2d", 2.0, TimeUnit::Days),
            ("1.5w", 1.5, TimeUnit::Weeks),
            ("3mon", 3.0, TimeUnit::Months),
            ("30m", 30.0, TimeUnit::Minutes),
            ("2y", 2.0, TimeUnit::Years),
            ("50%", 50.0, TimeUnit::Percent),
            ("2eh", 2.0, TimeUnit::ElapsedHours),
            ("2ed", 2.0, TimeUnit::ElapsedDays),
            ("2emon", 2.0, TimeUnit::ElapsedMonths),
            ("2e%", 2.0, TimeUnit::ElapsedPercent),
        ];

        for (text, expected_value, expected_units) in cases {
            let duration = parse(&formats, MpxLocale::En, text)
                .unwrap_or_else(|| panic!("{text} should parse"));
            assert_eq!(
                duration.get_duration(),
                expected_value,
                "parsed numeric value for {text} does not match"
            );
            assert_eq!(
                duration.get_units(),
                expected_units,
                "parsed units for {text} do not match"
            );

            // model value -> MPX text -> model value
            let round_tripped = parse(&formats, MpxLocale::En, &to_mpx_string(duration))
                .unwrap_or_else(|| panic!("{text} should parse after round trip"));
            assert_eq!(round_tripped, duration);
        }
    }

    #[test]
    fn duration_without_unit_suffix_defaults_to_days() {
        let (_file, formats) = formats_for(MpxLocale::En);

        // Java: no unit suffix is treated as days, for MPX3 compatibility
        let duration = parse(&formats, MpxLocale::En, "2").expect("2 should parse");
        assert_eq!(duration, Duration::get_instance(2.0, TimeUnit::Days));

        let round_tripped = parse(&formats, MpxLocale::En, "2d").expect("2d should parse");
        assert_eq!(round_tripped, duration);
    }

    #[test]
    fn duration_allows_whitespace_before_units() {
        let (_file, formats) = formats_for(MpxLocale::En);

        assert_eq!(
            parse(&formats, MpxLocale::En, "8 h"),
            Some(Duration::get_instance(8.0, TimeUnit::Hours))
        );
        assert_eq!(
            parse(&formats, MpxLocale::En, "8   h"),
            Some(Duration::get_instance(8.0, TimeUnit::Hours))
        );
    }

    #[test]
    fn duration_unit_suffix_is_case_insensitive() {
        let (_file, formats) = formats_for(MpxLocale::En);

        assert_eq!(
            parse(&formats, MpxLocale::En, "8H"),
            Some(Duration::get_instance(8.0, TimeUnit::Hours))
        );
        assert_eq!(
            parse(&formats, MpxLocale::En, "8Days"),
            Some(Duration::get_instance(8.0, TimeUnit::Days))
        );
    }

    #[test]
    fn duration_rejects_invalid_values() {
        let (_file, formats) = formats_for(MpxLocale::En);

        // Empty string: Java throws ParseException
        assert_eq!(parse(&formats, MpxLocale::En, ""), None);
        // null: Java throws NullPointerException
        assert_eq!(
            DurationUtility::get_instance(
                None,
                formats.get_duration_decimal_format(),
                MpxLocale::En
            ),
            None
        );
        // Invalid units: Java throws MPXJException(INVALID_TIME_UNIT)
        assert_eq!(parse(&formats, MpxLocale::En, "8x"), None);
        // Invalid number
        assert_eq!(parse(&formats, MpxLocale::En, "abcd"), None);
        // Trailing whitespace becomes part of the unit suffix and cannot be recognized
        assert_eq!(parse(&formats, MpxLocale::En, "8h "), None);
        // Units only, with no number
        assert_eq!(parse(&formats, MpxLocale::En, "h"), None);
    }

    #[test]
    fn duration_parses_leading_whitespace_before_digits() {
        let (_file, formats) = formats_for(MpxLocale::En);

        // Java: NumberFormat.parse ignores leading whitespace
        assert_eq!(
            parse(&formats, MpxLocale::En, " 8h"),
            Some(Duration::get_instance(8.0, TimeUnit::Hours))
        );
    }

    #[test]
    fn duration_uses_locale_time_unit_table() {
        let (_file, formats) = formats_for(MpxLocale::De);

        // The unit suffix is resolved from the locale's name table (the Java German table uses "t" for days)
        assert_eq!(
            parse(&formats, MpxLocale::De, "2t"),
            Some(Duration::get_instance(2.0, TimeUnit::Days))
        );
        assert_eq!(parse(&formats, MpxLocale::De, "2d"), None);
        assert_eq!(
            parse(&formats, MpxLocale::En, "2d"),
            Some(Duration::get_instance(2.0, TimeUnit::Days))
        );
    }

    /// Regression: fractional durations in "decimal comma" locales such as German / French must parse.
    ///
    /// Before the fix, `DurationUtility` did not take a `NumberFormat` and substituted `f64::from_str`,
    /// so `1,5t` failed to parse and was silently dropped (see TECH_DEBT "8. Confirmed but pending wiring", item 2).
    #[test]
    fn duration_parses_locale_decimal_separator() {
        // German: decimal separator ',', grouping separator '.' (see `locale_data_de`)
        let (_file, formats) = formats_for(MpxLocale::De);
        assert_eq!(
            parse(&formats, MpxLocale::De, "1,5t"),
            Some(Duration::get_instance(1.5, TimeUnit::Days))
        );
        assert_eq!(
            parse(&formats, MpxLocale::De, "2,25h"),
            Some(Duration::get_instance(2.25, TimeUnit::Hours))
        );

        // French also uses a decimal comma
        let (_file_fr, formats_fr) = formats_for(MpxLocale::Fr);
        assert_eq!(
            parse(&formats_fr, MpxLocale::Fr, "1,5j"),
            Some(Duration::get_instance(1.5, TimeUnit::Days))
        );

        // In an English locale `,` is a grouping separator (Java's `DecimalFormat` also accepts grouping separators when parsing),
        // so `1,5d` yields 15 days -- exactly as in Java, with no "correction".
        let (_file_en, formats_en) = formats_for(MpxLocale::En);
        assert_eq!(
            parse(&formats_en, MpxLocale::En, "1,5d"),
            Some(Duration::get_instance(15.0, TimeUnit::Days))
        );
        assert_eq!(
            parse(&formats_en, MpxLocale::En, "1.5d"),
            Some(Duration::get_instance(1.5, TimeUnit::Days))
        );
    }
}
