//! Corresponds to Java: `org/mpxj/mpx/MPXJFormats.java`
//!
//! # Porting trade-offs (`DateTimeFormatter` / `DateTimeFormatterBuilder` / `NumberFormat`)
//!
//! Java's class compiles the date / time format configuration from the MPX file into a
//! `java.time.format.DateTimeFormatter` (using `DateTimeFormatterBuilder`'s `appendPattern` /
//! `appendValueReduced` / `appendText` / `optionalStart`, etc.) and uses `NumberFormat` to parse
//! and format numbers. Rust's chrono has no equivalent "pattern string → parser" facility
//! (`StrftimeItems` covers a different set of pattern letters and does not support optional
//! sections), so this module builds its own **lightweight pattern engine**:
//!
//! - [`compile_pattern`] compiles the subset of pattern strings Java uses into a [`PatternNode`]
//!   list: literals, numeric fields (`d` / `M` / `y` / `h` / `H` / `m`), text fields
//!   (`MMM` / `MMMM` / `EEE`), AM/PM text, week fields (`F` / `w`), and optional sections
//!   `[...]` (which may be nested).
//! - The parse side corresponds to Java's SMART / LENIENT parsing: numeric fields accept `1..N`
//!   digits (`dd` also accepts `5`, equivalent to `effMinWidth = 1` in Java's non-strict mode),
//!   an optional section "advances only if it matches as a whole, otherwise rewinds to the section
//!   start", literals are matched case-insensitively (corresponding to `parseCaseInsensitive`), and
//!   trailing characters are allowed after parsing (Java likewise does not require consuming the
//!   whole string).
//! - The print side corresponds to `format`: `dd` / `MM` etc. zero-pad to the pattern width, `yy`
//!   prints a two-digit year, `MMM` / `MMMM` / `EEE` print text, and `F` / `w` print the day-of-week
//!   within the month / week of the year.
//! - Field reconstruction from the parse result: "year + month + day" are needed to obtain a date
//!   (Java likewise cannot reconstruct a `LocalDate` when any of these is missing, and throws
//!   `DateTimeParseException` → Rust returns `None`).
//!
//! ## Error-prone semantics such as `YYYY`
//!
//! - In this class's pattern strings, `yyyy` is the **calendar year** (chrono's `Date::year()`), and
//!   `yy` is a two-digit year: printing uses `year % 100`; parsing reduces per Java's
//!   `appendValueReduced(YEAR, 2, 2, baseDate)`, with the base taken from
//!   [`LocalDateHelper::two_digit_year_base_date`] (`now - 80 years`), so the result falls in the
//!   window `[baseYear, baseYear + 99]` (hence the two-digit `98` parses as 1998 in 2026).
//!   The semantics of Java's "week-based year" `YYYY` (`WeekFields`) **do not exist** here.
//! - `ww` corresponds to Java's `w` (`WeekFields.ISO`'s week-of-week-based-year), and Rust uses
//!   chrono's `NaiveDate::iso_week().week()`.
//! - 12 / 24-hour clock: `hh` is the clock hour 1-12 (combined with AM / PM text to reconstruct the
//!   24-hour value when parsing; treated as AM when there is no AM / PM, consistent with Java's
//!   default handling of `CLOCK_HOUR_OF_AMPM`).
//!
//! ## Localized month / weekday names
//!
//! Java takes localized month / weekday names from the JDK's `java.time` via
//! `DateTimeFormatter.withLocale(m_locale)`. Rust builds its own tables for the 9 supported languages
//! with [`localized_names`] (data taken from JDK 25's `MMM` / `MMMM` / `EEE` / `EEEE` output under the
//! corresponding `Locale`), dispatched by [`names_for_locale`] on [`MpxLocale`], matching Java. Note
//! that `crate::mpx::locale_data` does not contain these names (neither does Java's
//! `org.mpxj.mpx.LocaleData`), so the data can only be built here.
//! AM / PM text comes, as in Java, from the project properties' `getAmText` / `getPmText` (compiled in
//! Java into the `appendText` mapping), with English `am` / `pm` only as a fallback.
//!
//! ## Degradations and unimplemented parts (unused by MPX)
//!
//! - Java's LENIENT-mode field overflow normalization (e.g. `31/02` being normalized) is not
//!   implemented: an invalid date directly returns `None`.
//! - `F` / `w` only participate in parsing / printing, not in date reconstruction (Java cannot
//!   reconstruct a date from just these two fields either).
//!
//! ## Other structural trade-offs
//!
//! - `getPrintDateBuilder` / `getParseDateBuilder` return a `DateTimeFormatterBuilder` in Java; here
//!   they return a compiled [`PatternNode`] list; the two parts `[yyyy]` and the immediately
//!   following `appendValueReduced(YEAR, 2, 2, base)` are merged into a single `Year` field (1-2
//!   digits reduce via the window, 3-4 digits use the calendar year when parsing) to reproduce the
//!   intent of Java's two-part parsing.
//! - `Locale` → [`MpxLocale`]; `ProjectFile` → `Weak<RefCell<ProjectFile>>` (this crate's convention);
//!   `NumberFormat` → [`MPXJNumberFormat`].
//! - Null values: Java throws NPE when the formatter is `null`, while Rust returns `None`;
//!   `ParseException` / `DateTimeParseException` likewise return `None` (no panic). The three `parse*`
//!   methods take `&str` for Java's `String` argument (`null` is left to the caller via `Option`), and
//!   the `print*` methods take `Option` for Java's null checks.
//! - Copied upstream behavior: in the `DD_MMM_HH_MM` branch Java calls `createPrintFormatterWithTime`
//!   but **discards the return value**, so the print formatter is not updated (a suspected upstream
//!   bug); this is copied and noted here.
//!
//! Skipped Java members: none.

use std::cell::RefCell;
use std::rc::Weak;

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

use crate::common::local_date_helper::LocalDateHelper;
use crate::date_order::DateOrder;
use crate::mpx::locale_utility::MpxLocale;
use crate::mpx::localized_names::{AM, PM, TextTable};
use crate::mpx::mpxj_number_format::MPXJNumberFormat;
use crate::project_date_format::ProjectDateFormat;
use crate::project_file::ProjectFile;
use crate::project_properties::ProjectProperties;
use crate::project_time_format::ProjectTimeFormat;

// ---------------------------------------------------------------------------
// Pattern engine: compilation
// ---------------------------------------------------------------------------

/// Numeric fields in a pattern string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NumberField {
    /// Day of month (`d` / `dd`)
    Day,
    /// Month (`M` / `MM`)
    Month,
    /// Calendar year (`yyyy`, and the `[yyyy]` + `appendValueReduced` combination)
    Year,
    /// Two-digit year (`yy`, corresponding to Java's `appendValueReduced(YEAR, 2, 2, base)`)
    ReducedYear,
    /// 24-hour clock hour (`H` / `HH`)
    Hour24,
    /// 12-hour clock hour (`h` / `hh`)
    Hour12,
    /// Minute (`m` / `mm`)
    Minute,
}

/// A compiled pattern string element.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PatternNode {
    /// Literal text (matched case-insensitively when parsing)
    Literal(String),
    /// Numeric field
    Number {
        /// Field
        field: NumberField,
        /// Minimum width when printing
        min_width: usize,
    },
    /// Month name (`MMM` abbreviated / `MMMM` full)
    MonthText {
        /// Whether to use the full name
        full: bool,
    },
    /// Weekday name (`EEE`)
    DayName,
    /// AM / PM text (corresponding to Java `appendText(ChronoField.AMPM_OF_DAY, ...)`)
    AmPmText,
    /// Week of the year (`w`, ISO week)
    WeekOfYear {
        /// Minimum width when printing
        min_width: usize,
    },
    /// Day of the week within the month (`F`)
    DayOfWeekInMonth {
        /// Minimum width when printing
        min_width: usize,
    },
    /// Optional section `[...]`
    Optional(Vec<PatternNode>),
}

/// Compile a pattern string into a node list.
///
/// Corresponds to Java: `DateTimeFormatterBuilder.appendPattern(String)`
///
/// Java throws `IllegalArgumentException` for an unrecognized pattern letter; Rust does not panic,
/// treating unknown letters as literals and an unclosed `[` / `'` as extending to the end of the string.
fn compile_pattern(pattern: &str) -> Vec<PatternNode> {
    let chars: Vec<char> = pattern.chars().collect();
    compile_pattern_at(&chars, 0, false).0
}

/// Compile the pattern string starting at `start`, returning (node list, end position).
fn compile_pattern_at(chars: &[char], start: usize, in_optional: bool) -> (Vec<PatternNode>, usize) {
    let mut nodes = Vec::new();
    let mut literal = String::new();
    let mut index = start;

    while index < chars.len() {
        let current = chars[index];
        match current {
            '\'' => {
                // `''` represents a literal single quote; the content between `'...'` is taken as a literal verbatim
                if chars.get(index + 1) == Some(&'\'') {
                    literal.push('\'');
                    index += 2;
                } else {
                    index += 1;
                    while index < chars.len() && chars[index] != '\'' {
                        literal.push(chars[index]);
                        index += 1;
                    }
                    index += 1;
                }
            }
            '[' => {
                push_literal(&mut nodes, &mut literal);
                let (inner, end) = compile_pattern_at(chars, index + 1, true);
                nodes.push(PatternNode::Optional(inner));
                index = end;
            }
            ']' if in_optional => {
                push_literal(&mut nodes, &mut literal);
                return (nodes, index + 1);
            }
            'd' | 'M' | 'y' | 'h' | 'H' | 'm' | 'E' | 'F' | 'w' => {
                push_literal(&mut nodes, &mut literal);
                let mut count = 0;
                while index + count < chars.len() && chars[index + count] == current {
                    count += 1;
                }
                index += count;

                if let Some(node) = pattern_letter(current, count) {
                    nodes.push(node);
                }
            }
            _ => {
                literal.push(current);
                index += 1;
            }
        }
    }

    push_literal(&mut nodes, &mut literal);
    (nodes, index)
}

/// Push the pending literal into the node list.
fn push_literal(nodes: &mut Vec<PatternNode>, literal: &mut String) {
    if !literal.is_empty() {
        nodes.push(PatternNode::Literal(std::mem::take(literal)));
    }
}

/// Convert a "pattern letter + repeat count" into a node.
///
/// Corresponds to Java: the handling of a single pattern letter in `DateTimeFormatterBuilder.appendPattern`.
fn pattern_letter(letter: char, count: usize) -> Option<PatternNode> {
    match letter {
        'd' => Some(PatternNode::Number {
            field: NumberField::Day,
            min_width: count,
        }),
        'M' if count <= 2 => Some(PatternNode::Number {
            field: NumberField::Month,
            min_width: count,
        }),
        'M' => Some(PatternNode::MonthText { full: count >= 4 }),
        // Java: `yy` is handled by appendValueReduced (two-digit year reduction)
        'y' if count == 2 => Some(PatternNode::Number {
            field: NumberField::ReducedYear,
            min_width: 2,
        }),
        'y' => Some(PatternNode::Number {
            field: NumberField::Year,
            min_width: count.max(1),
        }),
        'h' => Some(PatternNode::Number {
            field: NumberField::Hour12,
            min_width: count,
        }),
        'H' => Some(PatternNode::Number {
            field: NumberField::Hour24,
            min_width: count,
        }),
        'm' => Some(PatternNode::Number {
            field: NumberField::Minute,
            min_width: count,
        }),
        'E' => Some(PatternNode::DayName),
        'F' => Some(PatternNode::DayOfWeekInMonth { min_width: count }),
        'w' => Some(PatternNode::WeekOfYear { min_width: count }),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Pattern engine: parsing
// ---------------------------------------------------------------------------

/// Field values accumulated during parsing (corresponding to the fields written to Java's `DateTimeParseContext` when parsing).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ParsedFields {
    /// Calendar year
    year: Option<i32>,
    /// Month
    month: Option<u32>,
    /// Day of month
    day: Option<u32>,
    /// 24-hour clock hour
    hour24: Option<u32>,
    /// 12-hour clock hour
    hour12: Option<u32>,
    /// Minute
    minute: Option<u32>,
    /// AM / PM (0 = AM, 1 = PM)
    ampm: Option<u32>,
}

/// Parse the text node by node, returning the end position; returns `None` if any node fails.
fn parse_nodes(
    nodes: &[PatternNode],
    chars: &[char],
    mut pos: usize,
    fields: &mut ParsedFields,
    context: &NameContext<'_>,
) -> Option<usize> {
    for node in nodes {
        match node {
            PatternNode::Literal(literal) => {
                pos = match_text_ignore_case(chars, pos, literal)?;
            }
            PatternNode::Number { field, .. } => {
                pos = parse_number(*field, chars, pos, fields)?;
            }
            PatternNode::MonthText { full } => {
                pos = parse_month_name(chars, pos, *full, fields, context)?;
            }
            PatternNode::DayName => {
                pos = parse_day_name(chars, pos, context)?;
            }
            PatternNode::AmPmText => {
                pos = parse_am_pm(chars, pos, fields, context)?;
            }
            PatternNode::WeekOfYear { .. } => {
                pos = parse_bounded_number(chars, pos, 2, 1, 53)?;
            }
            PatternNode::DayOfWeekInMonth { .. } => {
                pos = parse_bounded_number(chars, pos, 2, 1, 5)?;
            }
            PatternNode::Optional(inner) => {
                // Optional section: advance if it matches as a whole, otherwise rewind to the section start (consistent with Java's optional-section semantics)
                let saved = fields.clone();
                match parse_nodes(inner, chars, pos, fields, context) {
                    Some(end) => pos = end,
                    None => *fields = saved,
                }
            }
        }
    }

    Some(pos)
}

/// Parse a numeric field.
///
/// Corresponds to Java: non-strict parsing of `DateTimeFormatterBuilder.appendValue*` (`effMinWidth = 1`).
fn parse_number(
    field: NumberField,
    chars: &[char],
    pos: usize,
    fields: &mut ParsedFields,
) -> Option<usize> {
    let max_digits = match field {
        NumberField::Year => 4,
        _ => 2,
    };

    let (index, value) = read_digits(chars, pos, max_digits)?;

    match field {
        NumberField::Day => {
            if !(1..=31).contains(&value) {
                return None;
            }
            fields.day = Some(value as u32);
        }
        NumberField::Month => {
            if !(1..=12).contains(&value) {
                return None;
            }
            fields.month = Some(value as u32);
        }
        NumberField::Year | NumberField::ReducedYear => {
            let year = if index - pos <= 2 {
                reduce_two_digit_year(value)
            } else {
                value
            };
            if !(1..=9999).contains(&year) {
                return None;
            }
            fields.year = Some(year);
        }
        NumberField::Hour24 => {
            if !(0..=23).contains(&value) {
                return None;
            }
            fields.hour24 = Some(value as u32);
        }
        NumberField::Hour12 => {
            if !(1..=12).contains(&value) {
                return None;
            }
            fields.hour12 = Some(value as u32);
        }
        NumberField::Minute => {
            if !(0..=59).contains(&value) {
                return None;
            }
            fields.minute = Some(value as u32);
        }
    }

    Some(index)
}

/// Read (and range-check) a number without field semantics, used for fields such as `F` / `w` that only participate in matching.
fn parse_bounded_number(
    chars: &[char],
    pos: usize,
    max_digits: usize,
    min_value: i32,
    max_value: i32,
) -> Option<usize> {
    let (index, value) = read_digits(chars, pos, max_digits)?;
    if !(min_value..=max_value).contains(&value) {
        return None;
    }
    Some(index)
}

/// Read up to `max_digits` decimal digits, returning (end position, value).
fn read_digits(chars: &[char], pos: usize, max_digits: usize) -> Option<(usize, i32)> {
    let mut index = pos;
    let mut value = 0;
    let mut digits = 0;

    while digits < max_digits {
        match chars.get(index).and_then(|character| character.to_digit(10)) {
            Some(digit) => {
                value = value * 10 + digit as i32;
                digits += 1;
                index += 1;
            }
            None => break,
        }
    }

    if digits == 0 {
        return None;
    }

    Some((index, value))
}

/// Parse a month name.
///
/// Corresponds to Java: `DateTimeFormatter`'s text parsing of `MMM` / `MMMM` (case-insensitive).
fn parse_month_name(
    chars: &[char],
    pos: usize,
    full: bool,
    fields: &mut ParsedFields,
    context: &NameContext<'_>,
) -> Option<usize> {
    // Full names first, then abbreviations (`May` and other months where the two are the same are unaffected)
    let tables: [&[&str; 12]; 2] = if full {
        [
            &context.tables.months_full,
            &context.tables.months_abbreviated,
        ]
    } else {
        [
            &context.tables.months_abbreviated,
            &context.tables.months_full,
        ]
    };

    for table in tables {
        for (index, name) in table.iter().enumerate() {
            if let Some(end) = match_text_ignore_case(chars, pos, name) {
                fields.month = Some(index as u32 + 1);
                return Some(end);
            }
        }
    }

    None
}

/// Parse a weekday name (the result does not participate in date reconstruction, consistent with Java discarding weekday names).
fn parse_day_name(chars: &[char], pos: usize, context: &NameContext<'_>) -> Option<usize> {
    for table in [
        &context.tables.weekdays_abbreviated,
        &context.tables.weekdays_full,
    ] {
        for name in table.iter() {
            if let Some(end) = match_text_ignore_case(chars, pos, name) {
                return Some(end);
            }
        }
    }

    None
}

/// Parse AM / PM text.
///
/// Corresponds to Java: `DateTimeFormatterBuilder.appendText(ChronoField.AMPM_OF_DAY, ampmMap)`; the
/// mapping comes from the project properties' AM / PM text; English `am` / `pm` are additionally accepted here.
fn parse_am_pm(
    chars: &[char],
    pos: usize,
    fields: &mut ParsedFields,
    context: &NameContext<'_>,
) -> Option<usize> {
    let mut index = pos;
    while index < chars.len() && chars[index].is_whitespace() {
        index += 1;
    }

    // PM first: avoids the "pm" prefix interfering with custom text
    for (text, value) in [
        (context.pm, 1),
        (context.am, 0),
        (PM, 1),
        (AM, 0),
    ] {
        if text.is_empty() {
            continue;
        }

        if let Some(end) = match_text_ignore_case(chars, index, text) {
            fields.ampm = Some(value);
            return Some(end);
        }
    }

    None
}

/// Match text case-insensitively starting at `pos`, returning the new position on success.
fn match_text_ignore_case(chars: &[char], pos: usize, text: &str) -> Option<usize> {
    let mut index = pos;
    for expected in text.chars() {
        let current = chars.get(index)?;
        if !current.eq_ignore_ascii_case(&expected) {
            return None;
        }
        index += 1;
    }
    Some(index)
}

/// Reproduce the two-digit year reduction of Java's `appendValueReduced(YEAR, 2, 2, baseDate)`.
///
/// Java's base is `LocalDate.now().minusYears(80)`, and the result falls in `[baseYear, baseYear + 99]`.
fn reduce_two_digit_year(value: i32) -> i32 {
    let base_year = LocalDateHelper::two_digit_year_base_date().year();
    let range = 100;
    let last_part = base_year.rem_euclid(range);
    let value = if value < last_part { value + range } else { value };
    value - last_part + base_year
}

/// Reconstruct a date from the parsed fields.
fn resolve_date(fields: &ParsedFields) -> Option<NaiveDate> {
    let year = fields.year?;
    let month = fields.month?;
    let day = fields.day?;
    NaiveDate::from_ymd_opt(year, month, day)
}

/// Reconstruct a time from the parsed fields (returns `None` if the hour is missing).
fn resolve_time(fields: &ParsedFields) -> Option<NaiveTime> {
    let minute = fields.minute.unwrap_or(0);
    let hour = match fields.hour24 {
        Some(hour) => hour,
        None => {
            let hour12 = fields.hour12?;
            // Java: when the AMPM field is missing, treat as AM and map 12 o'clock to zero
            let afternoon = fields.ampm == Some(1);
            (hour12 % 12) + if afternoon { 12 } else { 0 }
        }
    };

    NaiveTime::from_hms_opt(hour, minute, 0)
}

/// Reconstruct a date-time from the parsed fields (uses midnight of that day when the time part is missing, corresponding to Java's `parseBest` fallback).
fn resolve_date_time(fields: &ParsedFields) -> Option<NaiveDateTime> {
    let date = resolve_date(fields)?;
    let time = resolve_time(fields).unwrap_or(NaiveTime::MIN);
    Some(NaiveDateTime::new(date, time))
}

// ---------------------------------------------------------------------------
// Pattern engine: printing
// ---------------------------------------------------------------------------

/// Format a date-time according to the node list.
fn format_date_time(
    nodes: &[PatternNode],
    value: &NaiveDateTime,
    context: &NameContext<'_>,
) -> String {
    let mut result = String::new();

    for node in nodes {
        match node {
            PatternNode::Literal(literal) => result.push_str(literal),
            PatternNode::Number { field, min_width } => {
                result.push_str(&format_number(*field, *min_width, value));
            }
            PatternNode::MonthText { full } => {
                result.push_str(month_name(context.tables, value.month(), *full));
            }
            PatternNode::DayName => {
                result.push_str(weekday_name(context.tables, value.weekday()));
            }
            PatternNode::AmPmText => result.push_str(if is_afternoon(value.hour()) {
                &context.pm
            } else {
                &context.am
            }),
            PatternNode::WeekOfYear { min_width } => {
                result.push_str(&pad(value.iso_week().week(), *min_width));
            }
            PatternNode::DayOfWeekInMonth { min_width } => {
                result.push_str(&pad(day_of_week_in_month(value.day()), *min_width));
            }
            // Java outputs the contents of optional sections normally when printing
            PatternNode::Optional(inner) => {
                result.push_str(&format_date_time(inner, value, context));
            }
        }
    }

    result
}

/// Format a numeric field.
fn format_number(field: NumberField, min_width: usize, value: &NaiveDateTime) -> String {
    match field {
        NumberField::Day => pad(value.day(), min_width),
        NumberField::Month => pad(value.month(), min_width),
        NumberField::Year => pad(value.year().max(0) as u32, min_width),
        // Java: `yy` prints the last two digits of the year
        NumberField::ReducedYear => pad((value.year().rem_euclid(100)) as u32, 2),
        NumberField::Hour24 => pad(value.hour(), min_width),
        NumberField::Hour12 => pad(clock_hour(value.hour()), min_width),
        NumberField::Minute => pad(value.minute(), min_width),
    }
}

/// Zero-pad on the left to `width` digits.
fn pad(value: u32, width: usize) -> String {
    format!("{value:0>width$}")
}

/// 12-hour clock hour (both 0 o'clock and 12 o'clock are 12).
fn clock_hour(hour: u32) -> u32 {
    match hour % 12 {
        0 => 12,
        value => value,
    }
}

/// Whether it is afternoon (corresponding to Java `ChronoField.AMPM_OF_DAY`).
fn is_afternoon(hour: u32) -> bool {
    hour >= 12
}

/// Day of the week within the month (corresponding to Java pattern letter `F`).
fn day_of_week_in_month(day: u32) -> u32 {
    ((day - 1) / 7) + 1
}

/// Month name text.
fn month_name(tables: &TextTable, month: u32, full: bool) -> &'static str {
    let index = month.saturating_sub(1) as usize;
    let table = if full {
        &tables.months_full
    } else {
        &tables.months_abbreviated
    };
    table.get(index).copied().unwrap_or("")
}

/// Weekday name text.
fn weekday_name(tables: &TextTable, weekday: chrono::Weekday) -> &'static str {
    let index = weekday.num_days_from_monday() as usize;
    tables
        .weekdays_abbreviated
        .get(index)
        .copied()
        .unwrap_or("")
}

/// Get the text constant table for a locale.
///
/// Corresponds to Java: `DateTimeFormatter.withLocale(m_locale)` (month / weekday name text).
/// The data is provided per language by [`crate::mpx::localized_names`].
fn names_for_locale(locale: MpxLocale) -> &'static TextTable {
    crate::mpx::localized_names::names_for_locale(locale)
}

/// Lookup table for text fields (AM / PM).
///
/// Corresponds to the `appendText(AMPM_OF_DAY, ampmMap)` mapping Java captures when building the
/// formatter, and the text source of `withLocale(m_locale)`.
#[derive(Debug, Clone, Copy)]
struct NameContext<'a> {
    /// AM text (from `ProjectProperties.getAmText()`)
    am: &'a str,
    /// PM text (from `ProjectProperties.getPmText()`)
    pm: &'a str,
    /// Month / weekday name table (see [`names_for_locale`])
    tables: &'static TextTable,
}

// ---------------------------------------------------------------------------
// MPXJFormats
// ---------------------------------------------------------------------------

/// The various format objects needed to parse / format an MPX file.
///
/// Corresponds to Java: `public final class MPXJFormats`
pub struct MPXJFormats {
    /// Target locale (corresponds to Java `m_locale`)
    locale: MpxLocale,
    /// Text meaning "not set", usually `NA` (corresponds to Java `m_nullText`)
    null_text: String,
    /// Parent file (corresponds to Java `m_projectFile`; Rust uses a weak reference per this crate's convention)
    project_file: Weak<RefCell<ProjectFile>>,
    /// Units decimal format (corresponds to Java `m_unitsDecimalFormat`)
    units_decimal_format: MPXJNumberFormat,
    /// Decimal format (corresponds to Java `m_decimalFormat`)
    decimal_format: MPXJNumberFormat,
    /// Currency format (corresponds to Java `m_currencyFormat`)
    currency_format: MPXJNumberFormat,
    /// Duration decimal format (corresponds to Java `m_durationDecimalFormat`)
    duration_decimal_format: MPXJNumberFormat,
    /// Percentage decimal format (corresponds to Java `m_percentageDecimalFormat`)
    percentage_decimal_format: MPXJNumberFormat,
    /// Whether to discard a leading weekday name when parsing a timestamp (corresponds to Java `m_parseDateTimeSkipDayName`)
    parse_date_time_skip_day_name: bool,
    /// Timestamp parse format (corresponds to Java `m_parseDateTimeFormat`)
    parse_date_time_format: Vec<PatternNode>,
    /// Timestamp print format (corresponds to Java `m_printDateTimeFormat`)
    print_date_time_format: Vec<PatternNode>,
    /// Date parse format (corresponds to Java `m_parseDateFormat`)
    parse_date_format: Vec<PatternNode>,
    /// Date print format (corresponds to Java `m_printDateFormat`)
    print_date_format: Vec<PatternNode>,
    /// Time parse format (corresponds to Java `m_parseTimeFormat`)
    parse_time_format: Vec<PatternNode>,
    /// Time print format (corresponds to Java `m_printTimeFormat`)
    print_time_format: Vec<PatternNode>,
    /// AM text (corresponds to the AM mapping entry Java compiles into the formatter; cached here during `update`)
    am_text: String,
    /// PM text (corresponds to the PM mapping entry Java compiles into the formatter; cached here during `update`)
    pm_text: String,
}

impl MPXJFormats {
    /// Construct the format object and initialize all formats immediately.
    ///
    /// Corresponds to Java: `public MPXJFormats(Locale locale, String nullText, ProjectFile file)`
    ///
    /// Trade-off: Java holds the `ProjectFile` directly; Rust accepts a `Weak<RefCell<ProjectFile>>`
    /// per this crate's convention (the caller must hold the file handle itself).
    pub fn new(
        locale: MpxLocale,
        null_text: &str,
        project_file: Weak<RefCell<ProjectFile>>,
    ) -> MPXJFormats {
        let mut result = MPXJFormats {
            locale,
            null_text: null_text.to_owned(),
            project_file,
            units_decimal_format: MPXJNumberFormat::new(),
            decimal_format: MPXJNumberFormat::new(),
            currency_format: MPXJNumberFormat::new(),
            duration_decimal_format: MPXJNumberFormat::new(),
            percentage_decimal_format: MPXJNumberFormat::new(),
            parse_date_time_skip_day_name: false,
            parse_date_time_format: Vec::new(),
            print_date_time_format: Vec::new(),
            parse_date_format: Vec::new(),
            print_date_format: Vec::new(),
            parse_time_format: Vec::new(),
            print_time_format: Vec::new(),
            am_text: String::new(),
            pm_text: String::new(),
        };
        result.update();
        result
    }

    /// Update the cached formats after a configuration change.
    ///
    /// Corresponds to Java: `public void update()`
    ///
    /// Trade-off: Java reads `m_projectFile.getProjectProperties()` directly (throwing NPE when the parent
    /// file is missing); Rust **skips the update** when the parent file has been dropped or is mutably
    /// borrowed (no panic, keeping the previous formats; on the first call it stays uninitialized).
    pub fn update(&mut self) {
        let Some(file) = self.project_file.upgrade() else {
            return;
        };
        let Ok(file) = file.try_borrow() else {
            return;
        };

        let properties = file.get_project_properties();
        let decimal_separator = properties.get_decimal_separator();
        let thousands_separator = properties.get_thousands_separator();

        // Java compiles the AM / PM text into the formatter in applyTime*Pattern; here it is cached along with the configuration
        self.am_text = properties.get_am_text();
        self.pm_text = properties.get_pm_text();

        self.update_numeric_formats(decimal_separator, thousands_separator);
        self.update_currency_formats(properties, decimal_separator, thousands_separator);
        self.update_date_time_formats(properties);
    }

    /// Update the numeric formats.
    ///
    /// Corresponds to Java: `private void updateNumericFormats(char decimalSeparator, char thousandsSeparator)`
    fn update_numeric_formats(&mut self, decimal_separator: char, thousands_separator: char) {
        self.units_decimal_format
            .apply_pattern("#.##", None, decimal_separator, thousands_separator);
        self.decimal_format
            .apply_pattern("0.00#", None, decimal_separator, thousands_separator);
        self.duration_decimal_format
            .apply_pattern("#.##", None, decimal_separator, thousands_separator);
        self.percentage_decimal_format
            .apply_pattern("##0.##", None, decimal_separator, thousands_separator);
    }

    /// Update the currency formats.
    ///
    /// Corresponds to Java:
    /// `private void updateCurrencyFormats(ProjectProperties properties, char decimalSeparator, char thousandsSeparator)`
    fn update_currency_formats(
        &mut self,
        properties: &ProjectProperties,
        decimal_separator: char,
        thousands_separator: char,
    ) {
        let mut prefix = String::new();
        let mut suffix = String::new();
        let currency_symbol = quote_format_characters(&properties.get_currency_symbol());

        match properties.get_symbol_position() {
            crate::currency_symbol_position::CurrencySymbolPosition::After => {
                suffix = currency_symbol;
            }
            crate::currency_symbol_position::CurrencySymbolPosition::Before => {
                prefix = currency_symbol;
            }
            crate::currency_symbol_position::CurrencySymbolPosition::AfterWithSpace => {
                suffix = format!(" {currency_symbol}");
            }
            crate::currency_symbol_position::CurrencySymbolPosition::BeforeWithSpace => {
                prefix = format!("{currency_symbol} ");
            }
        }

        let mut pattern = String::new();
        pattern.push_str(&prefix);
        pattern.push_str("#0");

        let digits = properties.get_currency_digits();
        if digits > 0 {
            pattern.push('.');
            for _ in 0..digits {
                pattern.push('0');
            }
        }

        pattern.push_str(&suffix);
        let primary_pattern = pattern.clone();

        let mut alternative_patterns = vec![String::new(); 7];
        alternative_patterns[0] = format!("{primary_pattern};({primary_pattern})");

        pattern.insert_str(prefix.len(), "#,#");
        let secondary_pattern = pattern.clone();
        alternative_patterns[1] = secondary_pattern.clone();
        alternative_patterns[2] = format!("{secondary_pattern};({secondary_pattern})");

        pattern.clear();
        pattern.push_str("#0");
        if digits > 0 {
            pattern.push('.');
            for _ in 0..digits {
                pattern.push('0');
            }
        }

        let no_symbol_primary_pattern = pattern.clone();
        alternative_patterns[3] = no_symbol_primary_pattern.clone();
        alternative_patterns[4] = format!("{no_symbol_primary_pattern};({no_symbol_primary_pattern})");

        pattern.insert_str(0, "#,#");
        let no_symbol_secondary_pattern = pattern;
        alternative_patterns[5] = no_symbol_secondary_pattern.clone();
        alternative_patterns[6] =
            format!("{no_symbol_secondary_pattern};({no_symbol_secondary_pattern})");

        self.currency_format.apply_pattern(
            &primary_pattern,
            Some(&alternative_patterns),
            decimal_separator,
            thousands_separator,
        );
    }

    /// Update the date and time formats.
    ///
    /// Corresponds to Java: `private void updateDateTimeFormats(ProjectProperties properties)`
    fn update_date_time_formats(&mut self, properties: &ProjectProperties) {
        self.populate_time_patterns(properties);
        self.populate_date_patterns(properties);
        self.populate_date_time_patterns(properties);
    }

    /// Build the "date + time" print format.
    ///
    /// Corresponds to Java:
    /// `private DateTimeFormatter createPrintFormatterWithTime(ProjectProperties properties, String pattern)`
    ///
    /// Trade-off: Java returns a `DateTimeFormatter`; this returns a compiled node list (see the file header).
    fn create_print_formatter_with_time(
        properties: &ProjectProperties,
        pattern: &str,
    ) -> Vec<PatternNode> {
        let mut nodes = compile_pattern(pattern);
        nodes.push(PatternNode::Literal(" ".to_owned()));
        Self::apply_time_print_pattern(properties, &mut nodes);
        nodes
    }

    /// Build the "date only" print format.
    ///
    /// Corresponds to Java: `private void createPrintFormatter(String pattern)`
    fn create_print_formatter(&mut self, pattern: &str) {
        self.print_date_time_format = compile_pattern(pattern);
    }

    /// Generate the parse pattern for the time part.
    ///
    /// Corresponds to Java:
    /// `private void applyTimeParsePattern(ProjectProperties properties, DateTimeFormatterBuilder builder)`
    ///
    /// The 12-hour form corresponds to Java's `optionalStart(); appendPattern("[hh][h]" + timesep + "mm[ ]");`
    /// appendText(AMPM_OF_DAY, map); optionalEnd(); appendPattern("[[HH][H]" + timesep + "mm]")`;
    /// the 24-hour form corresponds to `appendPattern("[HH][H][" + timesep + "mm]")`.
    fn apply_time_parse_pattern(properties: &ProjectProperties, builder: &mut Vec<PatternNode>) {
        let timesep = properties.get_time_separator();
        let format = properties.get_time_format();

        if format == ProjectTimeFormat::TwelveHour {
            let mut group = Vec::new();
            group.push(optional_hour(NumberField::Hour12, 2));
            group.push(optional_hour(NumberField::Hour12, 1));
            group.push(PatternNode::Literal(timesep.to_string()));
            group.push(PatternNode::Number {
                field: NumberField::Minute,
                min_width: 2,
            });
            group.push(PatternNode::Optional(vec![PatternNode::Literal(
                " ".to_owned(),
            )]));
            group.push(PatternNode::AmPmText);
            builder.push(PatternNode::Optional(group));

            // "[[HH][H]" + timesep + "mm]": the outer optional section contains two more optional hour fields
            let mut alternative = Vec::new();
            alternative.push(optional_hour(NumberField::Hour24, 2));
            alternative.push(optional_hour(NumberField::Hour24, 1));
            alternative.push(PatternNode::Literal(timesep.to_string()));
            alternative.push(PatternNode::Number {
                field: NumberField::Minute,
                min_width: 2,
            });
            builder.push(PatternNode::Optional(alternative));
        } else {
            builder.push(optional_hour(NumberField::Hour24, 2));
            builder.push(optional_hour(NumberField::Hour24, 1));
            builder.push(PatternNode::Optional(vec![
                PatternNode::Literal(timesep.to_string()),
                PatternNode::Number {
                    field: NumberField::Minute,
                    min_width: 2,
                },
            ]));
        }
    }

    /// Generate the print pattern for the time part.
    ///
    /// Corresponds to Java:
    /// `private void applyTimePrintPattern(ProjectProperties properties, DateTimeFormatterBuilder printBuilder)`
    fn apply_time_print_pattern(properties: &ProjectProperties, builder: &mut Vec<PatternNode>) {
        let timesep = properties.get_time_separator();
        let format = properties.get_time_format();

        if format == ProjectTimeFormat::TwelveHour {
            builder.push(PatternNode::Number {
                field: NumberField::Hour12,
                min_width: 2,
            });
            builder.push(PatternNode::Literal(timesep.to_string()));
            builder.push(PatternNode::Number {
                field: NumberField::Minute,
                min_width: 2,
            });
            builder.push(PatternNode::Literal(" ".to_owned()));
            builder.push(PatternNode::AmPmText);
        } else {
            builder.push(PatternNode::Number {
                field: NumberField::Hour24,
                min_width: 2,
            });
            builder.push(PatternNode::Literal(timesep.to_string()));
            builder.push(PatternNode::Number {
                field: NumberField::Minute,
                min_width: 2,
            });
        }
    }

    /// Generate the date parse format.
    ///
    /// Corresponds to Java: `private DateTimeFormatterBuilder getParseDateBuilder(ProjectProperties properties)`
    ///
    /// Trade-off: Java first does `[yyyy]` and then appends the optional `appendValueReduced(YEAR, 2, 2, base)`;
    /// this merges them into a single `Year` field (1-2 digits reduce as a two-digit year, 3-4 as the calendar
    /// year; see the file header).
    fn get_parse_date_builder(properties: &ProjectProperties) -> Vec<PatternNode> {
        let datesep = properties.get_date_separator();
        let year = PatternNode::Optional(vec![PatternNode::Number {
            field: NumberField::Year,
            min_width: 4,
        }]);

        match properties.get_date_order() {
            DateOrder::Dmy => compile_pattern(&format!(
                "[dd][d]{datesep}[MM][M]{datesep}"
            ))
            .into_iter()
            .chain(std::iter::once(year))
            .collect(),
            DateOrder::Mdy => compile_pattern(&format!(
                "[MM][M]{datesep}[dd][d]{datesep}"
            ))
            .into_iter()
            .chain(std::iter::once(year))
            .collect(),
            DateOrder::Ymd => std::iter::once(year)
                .chain(compile_pattern(&format!(
                    "{datesep}[MM][M]{datesep}[dd][d]"
                )))
                .collect(),
        }
    }

    /// Generate the date print format.
    ///
    /// Corresponds to Java: `private DateTimeFormatterBuilder getPrintDateBuilder(ProjectProperties properties)`
    fn get_print_date_builder(properties: &ProjectProperties) -> Vec<PatternNode> {
        let datesep = properties.get_date_separator();

        match properties.get_date_order() {
            DateOrder::Dmy => compile_pattern(&format!("dd{datesep}MM{datesep}yy")),
            DateOrder::Mdy => compile_pattern(&format!("MM{datesep}dd{datesep}yy")),
            DateOrder::Ymd => compile_pattern(&format!("yy{datesep}MM{datesep}dd")),
        }
    }

    /// Generate the timestamp formats from the project configuration.
    ///
    /// Corresponds to Java: `private void populateDateTimePatterns(ProjectProperties properties)`
    ///
    /// Note: in the `DD_MMM_HH_MM` branch Java discards the return value of `createPrintFormatterWithTime`,
    /// so the print formatter is not updated (a suspected upstream bug); copied here.
    fn populate_date_time_patterns(&mut self, properties: &ProjectProperties) {
        let datesep = properties.get_date_separator();
        let date_order = properties.get_date_order();

        match properties.get_date_format() {
            ProjectDateFormat::DdMmYyHhMm => match date_order {
                DateOrder::Dmy => {
                    self.create_parse_formatter_with_time(
                        properties,
                        &[&format!("dd{datesep}MM{datesep}"), "yy"],
                    );
                    self.print_date_time_format = Self::create_print_formatter_with_time(
                        properties,
                        &format!("dd{datesep}MM{datesep}yy"),
                    );
                }
                DateOrder::Mdy => {
                    self.create_parse_formatter_with_time(
                        properties,
                        &[&format!("MM{datesep}dd{datesep}"), "yy"],
                    );
                    self.print_date_time_format = Self::create_print_formatter_with_time(
                        properties,
                        &format!("MM{datesep}dd{datesep}yy"),
                    );
                }
                DateOrder::Ymd => {
                    self.create_parse_formatter_with_time(
                        properties,
                        &["yy", &format!("{datesep}MM{datesep}dd")],
                    );
                    self.print_date_time_format = Self::create_print_formatter_with_time(
                        properties,
                        &format!("yy{datesep}MM{datesep}dd"),
                    );
                }
            },
            ProjectDateFormat::DdMmYy => match date_order {
                DateOrder::Dmy => {
                    self.create_parse_formatter_with_time(
                        properties,
                        &[&format!("dd{datesep}MM{datesep}"), "yy"],
                    );
                    self.create_print_formatter(&format!("dd{datesep}MM{datesep}yy"));
                }
                DateOrder::Mdy => {
                    self.create_parse_formatter_with_time(
                        properties,
                        &[&format!("MM{datesep}dd{datesep}"), "yy"],
                    );
                    self.create_print_formatter(&format!("MM{datesep}dd{datesep}yy"));
                }
                DateOrder::Ymd => {
                    self.create_parse_formatter_with_time(
                        properties,
                        &["yy", &format!("{datesep}MM{datesep}dd")],
                    );
                    self.create_print_formatter(&format!("yy{datesep}MM{datesep}dd"));
                }
            },
            ProjectDateFormat::DdMmmmmYyyyHhMm => match date_order {
                DateOrder::Dmy => {
                    self.create_parse_formatter_with_time(properties, &["dd MMMM yyyy"]);
                    self.print_date_time_format =
                        Self::create_print_formatter_with_time(properties, "dd MMMM yyyy");
                }
                DateOrder::Mdy => {
                    self.create_parse_formatter_with_time(properties, &["MMMM dd yyyy"]);
                    self.print_date_time_format =
                        Self::create_print_formatter_with_time(properties, "MMMM dd yyyy");
                }
                DateOrder::Ymd => {
                    self.create_parse_formatter_with_time(properties, &["yyyy MMMM dd"]);
                    self.print_date_time_format =
                        Self::create_print_formatter_with_time(properties, "yyyy MMMM dd");
                }
            },
            ProjectDateFormat::DdMmmmmYyyy => match date_order {
                DateOrder::Dmy => {
                    self.create_parse_formatter_with_time(properties, &["dd MMMM yyyy"]);
                    self.create_print_formatter("dd MMMM yyyy");
                }
                DateOrder::Mdy => {
                    self.create_parse_formatter_with_time(properties, &["MMMM dd yyyy"]);
                    self.create_print_formatter("MMMM dd yyyy");
                }
                DateOrder::Ymd => {
                    self.create_parse_formatter_with_time(properties, &["yyyy MMMM dd"]);
                    self.create_print_formatter("yyyy MMMM dd");
                }
            },
            ProjectDateFormat::DdMmmHhMm => match date_order {
                DateOrder::Dmy => {
                    self.create_parse_formatter_with_time(properties, &["dd MMM"]);
                    // Java: the return value of `createPrintFormatterWithTime(properties, "dd MMM");` is discarded
                    let _ = Self::create_print_formatter_with_time(properties, "dd MMM");
                }
                DateOrder::Ymd | DateOrder::Mdy => {
                    self.create_parse_formatter_with_time(properties, &["MMM dd"]);
                    // Java: the return value of `createPrintFormatterWithTime(properties, "MMM dd");` is discarded
                    let _ = Self::create_print_formatter_with_time(properties, "MMM dd");
                }
            },
            ProjectDateFormat::DdMmmYy => match date_order {
                DateOrder::Dmy => {
                    self.create_parse_formatter_with_time(properties, &["dd MMM ''", "yy"]);
                    self.create_print_formatter("dd MMM ''yy");
                }
                DateOrder::Mdy => {
                    self.create_parse_formatter_with_time(properties, &["MMM dd ''", "yy"]);
                    self.create_print_formatter("MMM dd ''yy");
                }
                DateOrder::Ymd => {
                    self.create_parse_formatter_with_time(properties, &["''", "yy", " MMM dd"]);
                    self.create_print_formatter("''yy MMM dd");
                }
            },
            ProjectDateFormat::DdMmmmm => match date_order {
                DateOrder::Dmy => {
                    self.create_parse_formatter_with_time(properties, &["dd MMMM"]);
                    self.create_print_formatter("dd MMMM");
                }
                DateOrder::Ymd | DateOrder::Mdy => {
                    self.create_parse_formatter_with_time(properties, &["MMMM dd"]);
                    self.create_print_formatter("MMMM dd");
                }
            },
            ProjectDateFormat::DdMmm => match date_order {
                DateOrder::Dmy => {
                    self.create_parse_formatter_with_time(properties, &["dd MMM"]);
                    self.create_print_formatter("dd MMM");
                }
                DateOrder::Ymd | DateOrder::Mdy => {
                    self.create_parse_formatter_with_time(properties, &["MMM dd"]);
                    self.create_print_formatter("MMM dd");
                }
            },
            ProjectDateFormat::EeeDdMmYyHhMm => match date_order {
                DateOrder::Dmy => {
                    self.create_parse_formatter_with_time(
                        properties,
                        &[&format!("[EEE ]dd{datesep}MM{datesep}"), "yy"],
                    );
                    self.print_date_time_format = Self::create_print_formatter_with_time(
                        properties,
                        &format!("EEE dd{datesep}MM{datesep}yy"),
                    );
                }
                DateOrder::Mdy => {
                    self.create_parse_formatter_with_time(
                        properties,
                        &[&format!("[EEE ]MM{datesep}dd{datesep}"), "yy"],
                    );
                    self.print_date_time_format = Self::create_print_formatter_with_time(
                        properties,
                        &format!("EEE MM{datesep}dd{datesep}yy"),
                    );
                }
                DateOrder::Ymd => {
                    self.create_parse_formatter_with_time(
                        properties,
                        &["[EEE ]", "yy", &format!("{datesep}MM{datesep}dd")],
                    );
                    self.print_date_time_format = Self::create_print_formatter_with_time(
                        properties,
                        &format!("EEE yy{datesep}MM{datesep}dd"),
                    );
                }
            },
            ProjectDateFormat::EeeDdMmYy => match date_order {
                DateOrder::Dmy => {
                    self.create_parse_formatter_with_time(
                        properties,
                        &[&format!("[EEE ]dd{datesep}MM{datesep}"), "yy"],
                    );
                    self.create_print_formatter(&format!("EEE dd{datesep}MM{datesep}yy"));
                }
                DateOrder::Mdy => {
                    self.create_parse_formatter_with_time(
                        properties,
                        &[&format!("[EEE ]MM{datesep}dd{datesep}"), "yy"],
                    );
                    self.create_print_formatter(&format!("EEE MM{datesep}dd{datesep}yy"));
                }
                DateOrder::Ymd => {
                    self.create_parse_formatter_with_time(
                        properties,
                        &["[EEE ]", "yy", &format!("{datesep}MM{datesep}dd")],
                    );
                    self.create_print_formatter(&format!("EEE yy{datesep}MM{datesep}dd"));
                }
            },
            ProjectDateFormat::EeeDdMmmYy => match date_order {
                DateOrder::Dmy => {
                    self.create_parse_formatter_with_time(properties, &["[EEE ]dd MMM ''", "yy"]);
                    self.create_print_formatter("EEE dd MMM ''yy");
                }
                DateOrder::Mdy => {
                    self.create_parse_formatter_with_time(properties, &["[EEE ]MMM dd ''", "yy"]);
                    self.create_print_formatter("EEE MMM dd ''yy");
                }
                DateOrder::Ymd => {
                    self.create_parse_formatter_with_time(
                        properties,
                        &["[EEE ]''", "yy", " MMM dd"],
                    );
                    self.create_print_formatter("EEE ''yy MMM dd");
                }
            },
            ProjectDateFormat::EeeHhMm => {
                self.create_parse_formatter_with_time(properties, &["[EEE ]"]);
                self.print_date_time_format =
                    Self::create_print_formatter_with_time(properties, "EEE ");
            }
            ProjectDateFormat::DdMm => match date_order {
                DateOrder::Dmy => {
                    self.create_parse_formatter_with_time(
                        properties,
                        &[&format!("dd{datesep}MM")],
                    );
                    self.create_print_formatter(&format!("dd{datesep}MM"));
                }
                DateOrder::Ymd | DateOrder::Mdy => {
                    self.create_parse_formatter_with_time(
                        properties,
                        &[&format!("MM{datesep}dd")],
                    );
                    self.create_print_formatter(&format!("MM{datesep}dd"));
                }
            },
            ProjectDateFormat::Dd => {
                self.create_parse_formatter_with_time(properties, &["dd"]);
                self.create_print_formatter("dd");
            }
            ProjectDateFormat::HhMm => {
                let mut parse_nodes = Vec::new();
                Self::apply_time_parse_pattern(properties, &mut parse_nodes);
                self.parse_date_time_format = parse_nodes;

                let mut print_nodes = Vec::new();
                Self::apply_time_print_pattern(properties, &mut print_nodes);
                self.print_date_time_format = print_nodes;
            }
            ProjectDateFormat::EeeDdMmm => match date_order {
                DateOrder::Dmy => {
                    self.create_parse_formatter_with_time(properties, &["[EEE ]dd MMM"]);
                    self.create_print_formatter("EEE dd MMM");
                }
                DateOrder::Ymd | DateOrder::Mdy => {
                    self.create_parse_formatter_with_time(properties, &["[EEE ]MMM dd"]);
                    self.create_print_formatter("EEE MMM dd");
                }
            },
            ProjectDateFormat::EeeDdMm => match date_order {
                DateOrder::Dmy => {
                    self.create_parse_formatter_with_time(
                        properties,
                        &[&format!("[EEE ]dd{datesep}MM")],
                    );
                    self.create_print_formatter(&format!("EEE dd{datesep}MM"));
                }
                DateOrder::Ymd | DateOrder::Mdy => {
                    self.create_parse_formatter_with_time(
                        properties,
                        &[&format!("[EEE ]MM{datesep}dd")],
                    );
                    self.create_print_formatter(&format!("EEE MM{datesep}dd"));
                }
            },
            ProjectDateFormat::EeeDd => {
                self.create_parse_formatter_with_time(properties, &["[EEE ]dd"]);
                self.create_print_formatter("EEE dd");
            }
            ProjectDateFormat::DdWww => {
                self.create_parse_formatter_with_time(
                    properties,
                    &[&format!("F{datesep}'W'ww")],
                );
                self.create_print_formatter(&format!("F{datesep}'W'ww"));
            }
            ProjectDateFormat::DdWwwYyHhMm => {
                self.create_parse_formatter_with_time(
                    properties,
                    &[&format!("F{datesep}'W'ww{datesep}"), "yy"],
                );
                self.print_date_time_format = Self::create_print_formatter_with_time(
                    properties,
                    &format!("F{datesep}'W'ww{datesep}yy"),
                );
            }
            ProjectDateFormat::DdMmYyyy => match date_order {
                DateOrder::Dmy => {
                    self.create_parse_formatter_with_time(
                        properties,
                        &[&format!("dd{datesep}MM{datesep}yyyy")],
                    );
                    self.create_print_formatter(&format!("dd{datesep}MM{datesep}yyyy"));
                }
                DateOrder::Mdy => {
                    self.create_parse_formatter_with_time(
                        properties,
                        &[&format!("MM{datesep}dd{datesep}yyyy")],
                    );
                    self.create_print_formatter(&format!("MM{datesep}dd{datesep}yyyy"));
                }
                DateOrder::Ymd => {
                    self.create_parse_formatter_with_time(
                        properties,
                        &[&format!("yyyy{datesep}MM{datesep}dd")],
                    );
                    self.create_print_formatter(&format!("yyyy{datesep}MM{datesep}dd"));
                }
            },
        }
    }

    /// Build the timestamp parse format.
    ///
    /// Corresponds to Java:
    /// `private void createParseFormatterWithTime(ProjectProperties properties, String... patterns)`
    ///
    /// Java's `String...` varargs are expressed as a slice on the Rust side; a fragment equal to `yy`
    /// corresponds to Java's `appendValueReduced(ChronoField.YEAR, 2, 2, base)`, other fragments are compiled
    /// as pattern strings, and finally an optional section of "space + time parse pattern" is appended.
    fn create_parse_formatter_with_time(
        &mut self,
        properties: &ProjectProperties,
        patterns: &[&str],
    ) {
        let mut nodes = Vec::new();

        for pattern in patterns {
            if *pattern == "yy" {
                nodes.push(PatternNode::Number {
                    field: NumberField::ReducedYear,
                    min_width: 2,
                });
            } else {
                nodes.extend(compile_pattern(pattern));
            }
        }

        let mut time_nodes = vec![PatternNode::Literal(" ".to_owned())];
        Self::apply_time_parse_pattern(properties, &mut time_nodes);
        nodes.push(PatternNode::Optional(time_nodes));

        self.parse_date_time_format = nodes;

        // Java: when the pattern starts with "[EEE ]", the weekday name at the start of the text must be discarded first
        if patterns.first().is_some_and(|pattern| pattern.starts_with("[EEE ]")) {
            self.parse_date_time_skip_day_name = true;
        }
    }

    /// Generate the time formats.
    ///
    /// Corresponds to Java: `private void populateTimePatterns(ProjectProperties properties)`
    fn populate_time_patterns(&mut self, properties: &ProjectProperties) {
        let mut parse_nodes = Vec::new();
        Self::apply_time_parse_pattern(properties, &mut parse_nodes);
        self.parse_time_format = parse_nodes;

        let mut print_nodes = Vec::new();
        Self::apply_time_print_pattern(properties, &mut print_nodes);
        self.print_time_format = print_nodes;
    }

    /// Generate the date formats.
    ///
    /// Corresponds to Java: `private void populateDatePatterns(ProjectProperties properties)`
    fn populate_date_patterns(&mut self, properties: &ProjectProperties) {
        self.parse_date_format = Self::get_parse_date_builder(properties);
        self.print_date_format = Self::get_print_date_builder(properties);
    }

    /// Get the units decimal format.
    ///
    /// Corresponds to Java: `public NumberFormat getUnitsDecimalFormat()`
    pub fn get_units_decimal_format(&self) -> &MPXJNumberFormat {
        &self.units_decimal_format
    }

    /// Get the decimal format.
    ///
    /// Corresponds to Java: `public NumberFormat getDecimalFormat()`
    pub fn get_decimal_format(&self) -> &MPXJNumberFormat {
        &self.decimal_format
    }

    /// Get the currency format.
    ///
    /// Corresponds to Java: `public NumberFormat getCurrencyFormat()`
    pub fn get_currency_format(&self) -> &MPXJNumberFormat {
        &self.currency_format
    }

    /// Get the duration decimal format.
    ///
    /// Corresponds to Java: `public NumberFormat getDurationDecimalFormat()`
    pub fn get_duration_decimal_format(&self) -> &MPXJNumberFormat {
        &self.duration_decimal_format
    }

    /// Get the percentage decimal format.
    ///
    /// Corresponds to Java: `public NumberFormat getPercentageDecimalFormat()`
    pub fn get_percentage_decimal_format(&self) -> &MPXJNumberFormat {
        &self.percentage_decimal_format
    }

    /// Parse a time using the configured format.
    ///
    /// Corresponds to Java: `public LocalTime parseTime(String value)`
    ///
    /// Returns `None` on parse failure (Java throws `DateTimeParseException`).
    pub fn parse_time(&self, value: &str) -> Option<NaiveTime> {
        let fields = self.parse_fields(&self.parse_time_format, value)?;
        resolve_time(&fields)
    }

    /// Print a time using the configured format.
    ///
    /// Corresponds to Java: `public String printTime(LocalTime value)`
    pub fn print_time(&self, value: Option<NaiveTime>) -> Option<String> {
        let value = value?;
        if self.print_time_format.is_empty() {
            return None;
        }

        let date_time = NaiveDateTime::new(NaiveDate::MIN, value);
        Some(format_date_time(
            &self.print_time_format,
            &date_time,
            &self.name_context(),
        ))
    }

    /// Parse a date using the configured format.
    ///
    /// Corresponds to Java: `public LocalDateTime parseDate(String value)`
    ///
    /// Returns `None` when the text equals `getNullText()` (usually `NA`) or parsing fails;
    /// the result is taken at midnight of that day (corresponding to Java's `atStartOfDay()`).
    pub fn parse_date(&self, value: &str) -> Option<NaiveDateTime> {
        if value == self.null_text {
            return None;
        }

        let fields = self.parse_fields(&self.parse_date_format, value)?;
        let date = resolve_date(&fields)?;
        Some(NaiveDateTime::new(date, NaiveTime::MIN))
    }

    /// Print a date using the configured format.
    ///
    /// Corresponds to Java: `public String printDate(LocalDate value)`
    pub fn print_date(&self, value: Option<NaiveDate>) -> Option<String> {
        let value = value?;
        if self.print_date_format.is_empty() {
            return None;
        }

        let date_time = NaiveDateTime::new(value, NaiveTime::MIN);
        Some(format_date_time(
            &self.print_date_format,
            &date_time,
            &self.name_context(),
        ))
    }

    /// Parse a timestamp using the configured format.
    ///
    /// Corresponds to Java: `public LocalDateTime parseDateTime(String value)`
    ///
    /// As in Java: returns `None` when the text equals `getNullText()`; when the parse format includes a
    /// weekday (starts with `[EEE ]`), the content before the first space is discarded first; when
    /// year / month / day are missing the date cannot be reconstructed (Java throws
    /// `DateTimeParseException`), so `None` is returned.
    pub fn parse_date_time(&self, value: &str) -> Option<NaiveDateTime> {
        if value == self.null_text {
            return None;
        }

        let value = if self.parse_date_time_skip_day_name {
            // Java: value.substring(value.indexOf(' ') + 1)
            match value.find(' ') {
                Some(index) => &value[index + 1..],
                None => value,
            }
        } else {
            value
        };

        let fields = self.parse_fields(&self.parse_date_time_format, value)?;
        resolve_date_time(&fields)
    }

    /// Print a timestamp using the configured format.
    ///
    /// Corresponds to Java: `public String printDateTime(TemporalAccessor value)` (`LocalDateTime` branch)
    pub fn print_date_time(&self, value: Option<&NaiveDateTime>) -> Option<String> {
        let value = value?;
        if self.print_date_time_format.is_empty() {
            return None;
        }

        Some(format_date_time(
            &self.print_date_time_format,
            value,
            &self.name_context(),
        ))
    }

    /// Print a date using the configured format (corresponding to Java's branch that passes a `LocalDate` to `printDateTime`).
    ///
    /// Corresponds to Java: `public String printDateTime(TemporalAccessor value)` (`LocalDate` branch)
    ///
    /// Trade-off: Java throws `UnsupportedTemporalTypeException` when passing a `LocalDate` to a pattern with
    /// time fields; this treats it as midnight of that day.
    pub fn print_date_time_for_date(&self, value: Option<&NaiveDate>) -> Option<String> {
        let value = value?;
        self.print_date_time(Some(&NaiveDateTime::new(*value, NaiveTime::MIN)))
    }

    /// Get the text meaning "not set".
    ///
    /// Corresponds to Java: `public String getNullText()`
    pub fn get_null_text(&self) -> &str {
        &self.null_text
    }

    /// Parse text into a set of fields.
    ///
    /// Corresponds to Java: `DateTimeFormatter.parse(CharSequence)`
    fn parse_fields(&self, nodes: &[PatternNode], value: &str) -> Option<ParsedFields> {
        if nodes.is_empty() {
            // Java: throws NPE when the formatter is null (uninitialized)
            return None;
        }

        let chars: Vec<char> = value.chars().collect();
        let mut fields = ParsedFields::default();
        parse_nodes(nodes, &chars, 0, &mut fields, &self.name_context())?;
        Some(fields)
    }

    /// Build the text field lookup table.
    fn name_context(&self) -> NameContext<'_> {
        NameContext {
            am: &self.am_text,
            pm: &self.pm_text,
            tables: names_for_locale(self.locale),
        }
    }
}

/// Generate an "optional hour field" node, corresponding to Java patterns such as `[hh]` / `[H]`.
fn optional_hour(field: NumberField, min_width: usize) -> PatternNode {
    PatternNode::Optional(vec![PatternNode::Number { field, min_width }])
}

/// Escape special characters in the currency symbol.
///
/// Corresponds to Java: `private String quoteFormatCharacters(String literal)`
fn quote_format_characters(literal: &str) -> String {
    let mut result = String::new();

    for character in literal.chars() {
        match character {
            '0' | '#' | '.' | '-' | ',' | 'E' | ';' | '%' => {
                result.push('\'');
                result.push(character);
                result.push('\'');
            }
            _ => result.push(character),
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;

    /// Construct a format object. The caller must hold the returned file handle (`MPXJFormats` holds a weak reference internally).
    fn formats(config: impl FnOnce(&mut ProjectProperties)) -> (Rc<RefCell<ProjectFile>>, MPXJFormats) {
        let file = ProjectFile::new();
        {
            let mut borrowed = file.borrow_mut();
            config(borrowed.get_project_properties_mut());
        }

        let formats = MPXJFormats::new(MpxLocale::En, "NA", Rc::downgrade(&file));
        (file, formats)
    }

    /// Date 1998-08-25 14:35:00.
    fn sample() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(1998, 8, 25)
            .unwrap()
            .and_hms_opt(14, 35, 0)
            .unwrap()
    }

    #[test]
    fn time_formats_use_twelve_hour_clock() {
        let (_file, formats) = formats(|properties| {
            properties.set_time_format(Some(ProjectTimeFormat::TwelveHour));
            properties.set_am_text(Some("am".to_owned()));
            properties.set_pm_text(Some("pm".to_owned()));
        });

        // 12-hour printing zero-pads and appends AM/PM text
        assert_eq!(
            formats.print_time(NaiveTime::from_hms_opt(9, 5, 0)),
            Some("09:05 am".to_owned())
        );
        assert_eq!(
            formats.print_time(NaiveTime::from_hms_opt(17, 30, 0)),
            Some("05:30 pm".to_owned())
        );
        assert_eq!(
            formats.print_time(NaiveTime::from_hms_opt(0, 0, 0)),
            Some("12:00 am".to_owned())
        );
        assert_eq!(
            formats.print_time(NaiveTime::from_hms_opt(12, 0, 0)),
            Some("12:00 pm".to_owned())
        );

        // Parsing: AM/PM, missing leading zeros, no leading zeros at all, and the 24-hour form are all accepted
        for (text, expected) in [
            ("09:05 am", NaiveTime::from_hms_opt(9, 5, 0)),
            ("9:05 am", NaiveTime::from_hms_opt(9, 5, 0)),
            ("09:05 AM", NaiveTime::from_hms_opt(9, 5, 0)),
            ("05:30 pm", NaiveTime::from_hms_opt(17, 30, 0)),
            ("5:30 pm", NaiveTime::from_hms_opt(17, 30, 0)),
            ("12:00 am", NaiveTime::from_hms_opt(0, 0, 0)),
            ("12:00 pm", NaiveTime::from_hms_opt(12, 0, 0)),
            ("17:30", NaiveTime::from_hms_opt(17, 30, 0)),
        ] {
            assert_eq!(
                formats.parse_time(text),
                expected,
                "failed to parse {text} or result mismatch"
            );
        }

        // Round trip: print → parse
        for time in [
            NaiveTime::from_hms_opt(0, 0, 0),
            NaiveTime::from_hms_opt(9, 5, 0),
            NaiveTime::from_hms_opt(12, 0, 0),
            NaiveTime::from_hms_opt(23, 59, 0),
        ] {
            let printed = formats.print_time(time).expect("should be printable");
            assert_eq!(formats.parse_time(&printed), time);
        }
    }

    #[test]
    fn time_formats_use_twenty_four_hour_clock() {
        let (_file, formats) = formats(|properties| {
            properties.set_time_format(Some(ProjectTimeFormat::TwentyFourHour));
        });

        assert_eq!(
            formats.print_time(NaiveTime::from_hms_opt(9, 5, 0)),
            Some("09:05".to_owned())
        );
        assert_eq!(
            formats.print_time(NaiveTime::from_hms_opt(17, 30, 0)),
            Some("17:30".to_owned())
        );

        // Both hour / minute may be omitted when parsing, and no leading zeros are allowed
        assert_eq!(
            formats.parse_time("17:30"),
            NaiveTime::from_hms_opt(17, 30, 0)
        );
        assert_eq!(
            formats.parse_time("7:30"),
            NaiveTime::from_hms_opt(7, 30, 0)
        );
        assert_eq!(formats.parse_time("17"), NaiveTime::from_hms_opt(17, 0, 0));

        for time in [
            NaiveTime::from_hms_opt(0, 0, 0),
            NaiveTime::from_hms_opt(9, 5, 0),
            NaiveTime::from_hms_opt(23, 59, 0),
        ] {
            let printed = formats.print_time(time).expect("should be printable");
            assert_eq!(formats.parse_time(&printed), time);
        }
    }

    #[test]
    fn time_parsing_rejects_invalid_input() {
        let (_file, formats) = formats(|properties| {
            properties.set_time_format(Some(ProjectTimeFormat::TwentyFourHour));
        });

        assert_eq!(formats.parse_time(""), None);
        assert_eq!(formats.parse_time("abc"), None);
        // Java likewise cannot parse text with no time
        assert_eq!(formats.parse_time("25/08/98"), None);
    }

    #[test]
    fn date_formats_follow_date_order() {
        // Day-month-year
        let (_file, dmy) = formats(|properties| {
            properties.set_date_order(Some(DateOrder::Dmy));
        });
        // Month-day-year
        let (_file2, mdy) = formats(|properties| {
            properties.set_date_order(Some(DateOrder::Mdy));
        });
        // Year-month-day
        let (_file3, ymd) = formats(|properties| {
            properties.set_date_order(Some(DateOrder::Ymd));
        });

        let expected = NaiveDate::from_ymd_opt(1998, 3, 4)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        // The same text means different things under DMY / MDY (March 4 vs April 3)
        assert_eq!(dmy.parse_date("04/03/1998"), Some(expected));
        assert_eq!(
            mdy.parse_date("04/03/1998"),
            NaiveDate::from_ymd_opt(1998, 4, 3)
                .unwrap()
                .and_hms_opt(0, 0, 0)
        );
        assert_eq!(ymd.parse_date("1998/03/04"), Some(expected));

        // The print format matches the date order
        assert_eq!(
            dmy.print_date(NaiveDate::from_ymd_opt(1998, 3, 4)),
            Some("04/03/98".to_owned())
        );
        assert_eq!(
            mdy.print_date(NaiveDate::from_ymd_opt(1998, 3, 4)),
            Some("03/04/98".to_owned())
        );
        assert_eq!(
            ymd.print_date(NaiveDate::from_ymd_opt(1998, 3, 4)),
            Some("98/03/04".to_owned())
        );
    }

    #[test]
    fn date_formats_accept_optional_leading_zeros_and_four_digit_years() {
        let (_file, formats) = formats(|properties| {
            properties.set_date_order(Some(DateOrder::Dmy));
        });

        let cases = [
            ("25/08/1998", (1998, 8, 25)),
            ("25/8/1998", (1998, 8, 25)),
            ("5/8/1998", (1998, 8, 5)),
            ("05/08/1998", (1998, 8, 5)),
            // Two-digit years reduce via the window (base is the current year - 80)
            ("25/08/98", (1998, 8, 25)),
            ("1/1/99", (1999, 1, 1)),
        ];

        for (text, (year, month, day)) in cases {
            assert_eq!(
                formats.parse_date(text),
                NaiveDate::from_ymd_opt(year, month, day)
                    .unwrap()
                    .and_hms_opt(0, 0, 0),
                "failed to parse {text} or result mismatch"
            );
        }
    }

    #[test]
    fn date_formats_round_trip_across_boundaries() {
        let (_file, formats) = formats(|properties| {
            properties.set_date_order(Some(DateOrder::Dmy));
        });

        // Month / year boundaries
        for (year, month, day) in [
            (1998, 12, 31),
            (1999, 1, 1),
            (2000, 2, 29),
            (2026, 3, 1),
            (1996, 12, 1),
        ] {
            let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
            let printed = formats.print_date(Some(date)).expect("should be printable");
            assert_eq!(
                formats.parse_date(&printed),
                Some(NaiveDateTime::new(date, NaiveTime::MIN)),
                "{date} → {printed}"
            );
        }
    }

    #[test]
    fn date_parsing_rejects_invalid_input() {
        let (_file, formats) = formats(|properties| {
            properties.set_date_order(Some(DateOrder::Dmy));
        });

        assert_eq!(formats.parse_date(""), None);
        // Null text
        assert_eq!(formats.parse_date("NA"), None);
        // Month / day out of range
        assert_eq!(formats.parse_date("25/13/1998"), None);
        assert_eq!(formats.parse_date("32/12/1998"), None);
        // The date itself is invalid (Java LENIENT normalizes it; treated as invalid here)
        assert_eq!(formats.parse_date("31/02/1998"), None);
        // Unparsable text
        assert_eq!(formats.parse_date("hello"), None);
    }

    #[test]
    fn date_time_formats_round_trip_or_report_unresolvable() {
        // (format, printable, parseable, has time part)
        //
        // "Not parseable" is consistent with Java: the parse formatter cannot gather year / month / day
        // (Java throws DateTimeParseException), or the print formatter was not initialized because upstream
        // discarded the return value.
        let cases: [(ProjectDateFormat, bool, bool, bool); 21] = [
            (ProjectDateFormat::DdMmYyHhMm, true, true, true),
            (ProjectDateFormat::DdMmYy, true, true, false),
            (ProjectDateFormat::DdMmmmmYyyyHhMm, true, true, true),
            (ProjectDateFormat::DdMmmmmYyyy, true, true, false),
            // Upstream discards the return value of createPrintFormatterWithTime
            (ProjectDateFormat::DdMmmHhMm, false, false, true),
            (ProjectDateFormat::DdMmmYy, true, true, false),
            (ProjectDateFormat::DdMmmmm, true, false, false),
            (ProjectDateFormat::DdMmm, true, false, false),
            (ProjectDateFormat::EeeDdMmYyHhMm, true, true, true),
            (ProjectDateFormat::EeeDdMmYy, true, true, false),
            (ProjectDateFormat::EeeDdMmmYy, true, true, false),
            (ProjectDateFormat::EeeHhMm, true, false, true),
            (ProjectDateFormat::DdMm, true, false, false),
            (ProjectDateFormat::Dd, true, false, false),
            (ProjectDateFormat::HhMm, true, false, true),
            (ProjectDateFormat::EeeDdMmm, true, false, false),
            (ProjectDateFormat::EeeDdMm, true, false, false),
            (ProjectDateFormat::EeeDd, true, false, false),
            (ProjectDateFormat::DdWww, true, false, false),
            (ProjectDateFormat::DdWwwYyHhMm, true, false, true),
            (ProjectDateFormat::DdMmYyyy, true, true, false),
        ];

        let value = sample();

        for (format, printable, parseable, has_time) in cases {
            let (_file, formats) = formats(|properties| {
                properties.set_date_order(Some(DateOrder::Dmy));
                properties.set_date_format(Some(format));
            });

            let printed = formats.print_date_time(Some(&value));
            if !printable {
                assert_eq!(printed, None, "{format:?} should not have a printed result");
                continue;
            }

            let printed = printed.unwrap_or_else(|| panic!("{format:?} should be printable"));
            let parsed = formats.parse_date_time(&printed);
            if !parseable {
                assert_eq!(parsed, None, "{format:?} should not parse back to a date after printing {printed}");
                continue;
            }

            let parsed = parsed.unwrap_or_else(|| panic!("{format:?} {printed} should be parseable"));
            if has_time {
                assert_eq!(parsed, value, "{format:?} round-trip mismatch (printed {printed})");
            } else {
                assert_eq!(parsed.date(), value.date(), "{format:?} (printed {printed})");
                assert_eq!(parsed.time(), NaiveTime::MIN);
            }
        }
    }

    #[test]
    fn date_time_printing_uses_month_and_weekday_names() {
        let (_file, formats) = formats(|properties| {
            properties.set_date_order(Some(DateOrder::Dmy));
            properties.set_date_format(Some(ProjectDateFormat::EeeDdMmmYy));
        });

        // 1998-08-25 is a Tuesday
        assert_eq!(
            formats.print_date_time(Some(&sample())),
            Some("Tue 25 Aug '98".to_owned())
        );
        // The leading weekday name is discarded first when parsing
        assert_eq!(
            formats.parse_date_time("Tue 25 Aug '98"),
            Some(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(1998, 8, 25).unwrap(),
                NaiveTime::MIN
            ))
        );
    }

    /// Construct a format object for the given locale.
    fn formats_for_locale(
        locale: MpxLocale,
        config: impl FnOnce(&mut ProjectProperties),
    ) -> (Rc<RefCell<ProjectFile>>, MPXJFormats) {
        let file = ProjectFile::new();
        {
            let mut borrowed = file.borrow_mut();
            config(borrowed.get_project_properties_mut());
        }

        let formats = MPXJFormats::new(locale, "NA", Rc::downgrade(&file));
        (file, formats)
    }

    /// Printing uses each language's own month / weekday names (regression A19: previously all languages fell back to English).
    ///
    /// The expected values match the JDK's output under the corresponding `Locale` (see `localized_names`).
    #[test]
    fn date_time_printing_uses_locale_specific_names() {
        // 1998-08-25 is a Tuesday
        let cases = [
            (MpxLocale::En, "Tue 25 Aug '98"),
            (MpxLocale::De, "Di. 25 Aug. '98"),
            (MpxLocale::Fr, "mar. 25 août '98"),
            (MpxLocale::It, "mar 25 ago '98"),
            (MpxLocale::Es, "mar 25 ago '98"),
            (MpxLocale::Pt, "ter. 25 ago. '98"),
            (MpxLocale::Sv, "tis 25 aug. '98"),
            (MpxLocale::Ru, "вт 25 авг. '98"),
            (MpxLocale::Zh, "周二 25 8月 '98"),
        ];

        for (locale, expected) in cases {
            let (_file, formats) = formats_for_locale(locale, |properties| {
                properties.set_date_order(Some(DateOrder::Dmy));
                properties.set_date_format(Some(ProjectDateFormat::EeeDdMmmYy));
            });

            assert_eq!(
                formats.print_date_time(Some(&sample())),
                Some(expected.to_owned()),
                "{locale} print result mismatch"
            );
        }
    }

    /// The German default date format `EEE_DD_MM_YY` prints a localized weekday name (aligned with `sample.de.mpx`).
    #[test]
    fn german_default_date_format_prints_localised_weekday() {
        let (_file, formats) = formats_for_locale(MpxLocale::De, |properties| {
            properties.set_date_order(Some(DateOrder::Dmy));
            properties.set_date_separator('.');
            properties.set_date_format(Some(ProjectDateFormat::EeeDdMmYy));
        });

        assert_eq!(
            formats.print_date_time(Some(&sample())),
            Some("Di. 25.08.98".to_owned())
        );
        // Parsing discards the leading weekday name first, so text with a localized weekday name is likewise parseable
        assert_eq!(
            formats.parse_date_time("Di. 25.08.98"),
            Some(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(1998, 8, 25).unwrap(),
                NaiveTime::MIN
            ))
        );
    }

    /// Russian month name parsing (non-ASCII text, verifying the localized name table is usable on the parse path).
    #[test]
    fn localized_month_names_are_parsed() {
        let (_file, formats) = formats_for_locale(MpxLocale::Ru, |properties| {
            properties.set_date_order(Some(DateOrder::Dmy));
            properties.set_date_format(Some(ProjectDateFormat::EeeDdMmmYy));
        });

        // "25 авг. '98": the weekday name is discarded first, and the month name "авг." must be recognized
        assert_eq!(
            formats.parse_date_time("вт 25 авг. '98"),
            Some(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(1998, 8, 25).unwrap(),
                NaiveTime::MIN
            ))
        );
    }

    #[test]
    fn date_time_formats_use_variable_date_separator() {
        let (_file, formats) = formats(|properties| {
            properties.set_date_order(Some(DateOrder::Dmy));
            properties.set_date_separator('-');
            properties.set_date_format(Some(ProjectDateFormat::DdMmYyyy));
        });

        assert_eq!(
            formats.print_date_time(Some(&sample())),
            Some("25-08-1998".to_owned())
        );
        assert_eq!(
            formats.parse_date_time("25-08-1998"),
            Some(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(1998, 8, 25).unwrap(),
                NaiveTime::MIN
            ))
        );
    }

    #[test]
    fn date_time_parsing_rejects_null_text_and_unparsable_values() {
        let (_file, formats) = formats(|properties| {
            properties.set_date_order(Some(DateOrder::Dmy));
            properties.set_date_format(Some(ProjectDateFormat::DdMmYyyy));
        });

        assert_eq!(formats.parse_date_time("NA"), None);
        assert_eq!(formats.parse_date_time(""), None);
        assert_eq!(formats.parse_date_time("hello"), None);
        assert_eq!(formats.get_null_text(), "NA");
    }

    #[test]
    fn numeric_formats_follow_project_separators() {
        let (_file, formats) = formats(|properties| {
            properties.set_decimal_separator(',');
            properties.set_thousands_separator('.');
            properties.set_currency_symbol(Some("EUR".to_owned()));
            properties.set_symbol_position(Some(
                crate::currency_symbol_position::CurrencySymbolPosition::AfterWithSpace,
            ));
            properties.set_currency_digits(Some(2));
        });

        // Duration / units / decimal formats
        assert_eq!(
            formats.get_duration_decimal_format().parse(Some("2,5")),
            Some(2.5)
        );
        assert_eq!(formats.get_decimal_format().format(2.5), "2,50");
        assert_eq!(
            formats.get_decimal_format().parse(Some("2,50")),
            Some(2.5)
        );

        // Currency format: symbol after, with a space
        assert_eq!(formats.get_currency_format().format(1234.5), "1234,50 EUR");
        assert_eq!(
            formats.get_currency_format().parse(Some("1.234,50 EUR")),
            Some(1234.5)
        );
    }

    #[test]
    fn currency_format_handles_all_symbol_positions() {
        use crate::currency_symbol_position::CurrencySymbolPosition;

        let cases = [
            (CurrencySymbolPosition::Before, "$1,234.50"),
            (CurrencySymbolPosition::After, "1,234.50$"),
            (CurrencySymbolPosition::BeforeWithSpace, "$ 1,234.50"),
            (CurrencySymbolPosition::AfterWithSpace, "1,234.50 $"),
        ];

        for (position, expected) in cases {
            let (_file, formats) = formats(|properties| {
                properties.set_currency_symbol(Some("$".to_owned()));
                properties.set_symbol_position(Some(position));
                properties.set_currency_digits(Some(2));
            });

            // The primary pattern has no grouping separator when printing
            let text = formats.get_currency_format().format(1234.5);
            assert_eq!(text, expected.replace(",", ""), "{position:?}");

            // The grouping separator is optional when parsing
            assert_eq!(
                formats.get_currency_format().parse(Some(expected)),
                Some(1234.5),
                "{position:?}"
            );
            assert_eq!(
                formats.get_currency_format().parse(Some(text.as_str())),
                Some(1234.5),
                "{position:?}"
            );
        }
    }

    #[test]
    fn currency_format_is_updated_after_property_change() {
        let file = ProjectFile::new();
        let mut formats = MPXJFormats::new(MpxLocale::En, "NA", Rc::downgrade(&file));

        assert_eq!(formats.get_currency_format().format(1234.5), "$1234.50");

        {
            let mut borrowed = file.borrow_mut();
            let properties = borrowed.get_project_properties_mut();
            properties.set_currency_symbol(Some("GBP".to_owned()));
            properties.set_currency_digits(Some(0));
        }
        formats.update();

        assert_eq!(formats.get_currency_format().format(1234.5), "GBP1234");
    }

    #[test]
    fn formats_without_parent_file_do_not_panic() {
        // With no parent file (Java would NPE), no panic occurs and the formats stay uninitialized
        let formats = MPXJFormats::new(MpxLocale::En, "NA", Weak::new());
        assert_eq!(formats.print_date(Some(NaiveDate::MIN)), None);
        assert_eq!(formats.parse_date("25/08/1998"), None);
        assert_eq!(formats.get_null_text(), "NA");
    }

    #[test]
    fn number_and_date_formats_keep_english_defaults() {
        let (_file, formats) = formats(|_properties| {});

        // Default properties: decimal separator '.', thousands separator ',', currency symbol '$', 2 currency digits
        assert_eq!(formats.get_units_decimal_format().format(0.5), ".5");
        assert_eq!(formats.get_percentage_decimal_format().format(50.0), "50");
        assert_eq!(formats.get_currency_format().format(1234.5), "$1234.50");
    }

    #[test]
    fn quote_format_characters_escapes_pattern_syntax() {
        assert_eq!(quote_format_characters("$"), "$");
        assert_eq!(quote_format_characters("DM."), "DM'.'");
        assert_eq!(quote_format_characters("0#"), "'0''#'");
    }
}
