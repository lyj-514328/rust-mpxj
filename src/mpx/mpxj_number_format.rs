//! Corresponds to Java: `org/mpxj/mpx/MPXJNumberFormat.java`
//!
//! # Porting trade-offs (`DecimalFormat` / `DecimalFormatSymbols` / `ParsePosition`)
//!
//! Java's `MPXJNumberFormat extends DecimalFormat` and relies on `java.text` number formatting
//! (pattern strings, locale symbols, exact decimal arithmetic via `BigDecimal`). The Rust standard
//! library has no equivalent, so this module handles it as follows:
//!
//! - A module-private [`NumberPattern`] carries the parts of the `DecimalFormat` pattern string that
//!   this class actually uses: positive / negative sub-pattern prefixes and suffixes, minimum integer
//!   digits, minimum / maximum fraction digits, the grouping (thousands) separator, and the percent
//!   sign. Unused pattern features are degraded as noted at [`compile_number_pattern`]: exponent
//!   notation (`E`) is ignored after parsing, and an unparsable pattern (Java throws
//!   `IllegalArgumentException`) is recorded as "no pattern" instead of panicking.
//! - `DecimalFormatSymbols` → this struct stores the decimal and grouping separators itself
//!   (equivalent to Java's `setDecimalFormatSymbols`), defaulting to the English locale (`.` and `,`).
//! - Only `parse(String)` and `format(double)` of the methods inherited from `DecimalFormat` are used:
//!   `parse` → [`MPXJNumberFormat::parse`] (returns `Option<f64>`),
//!   `format` → [`MPXJNumberFormat::format`]. Java uses `BigDecimal` for exact decimal arithmetic;
//!   this uses `f64` (`str::parse` / `format!`) instead, differing only in extreme-precision cases.
//! - `ParsePosition` → Rust uses `Option<f64>` to express Java's "parse succeeded (index advanced)" and
//!   "parse failed (index kept / set to -1)"; the private function [`MPXJNumberFormat::parse_at`]
//!   returns "value + consumed character count" to reproduce the alternative-pattern attempt logic.
//! - Grouping separator: Java's `DecimalFormat.isGroupingUsed()` defaults to `true`, so parsing always
//!   accepts the grouping separator (MPXJ additionally prepares `#,#` alternative patterns to cover
//!   differences in **printing** and in the presence of a currency symbol); this also always accepts it.
//! - `equals` / `hashCode` → `PartialEq` / `Eq` / `Hash`: Java's `equals` first calls
//!   `super.equals(obj)`, and since `DecimalFormat` does not override `equals`, it effectively degrades
//!   to **reference equality**; Rust does a structural per-field comparison (more intuitive, noted here).
//!
//! Skipped Java members: `hashCode()` / `equals(Object)` (replaced by deriving `PartialEq` / `Eq` / `Hash`).

/// Number format pattern (the parts of the `DecimalFormat` pattern string used by this class).
///
/// Corresponds to Java: the pattern parsed from `applyPattern(String)` inside `DecimalFormat`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NumberPattern {
    /// Positive prefix (literal)
    positive_prefix: String,
    /// Positive suffix (literal)
    positive_suffix: String,
    /// Negative prefix (when no negative sub-pattern is given, `-` + the positive prefix)
    negative_prefix: String,
    /// Negative suffix (same as the positive suffix when no negative sub-pattern is given)
    negative_suffix: String,
    /// Minimum integer digits (number of `0`s in the integer part of the pattern)
    min_integer_digits: usize,
    /// Minimum fraction digits (number of `0`s in the fraction part)
    min_fraction_digits: usize,
    /// Maximum fraction digits (sum of the `0`s and `#`s in the fraction part)
    max_fraction_digits: usize,
    /// Whether to use the grouping (thousands) separator
    grouping: bool,
    /// Grouping size (number of digits after the last grouping separator; Java defaults to 3)
    grouping_size: usize,
    /// Whether `%` appears in the pattern (divide by 100 when parsing, multiply by 100 when printing)
    percent: bool,
}

/// Parse result for a single sub-pattern (positive or negative).
#[derive(Debug, Clone)]
struct SubPattern {
    /// Prefix literal
    prefix: String,
    /// Suffix literal
    suffix: String,
    /// Minimum integer digits
    min_integer_digits: usize,
    /// Minimum fraction digits
    min_fraction_digits: usize,
    /// Maximum fraction digits
    max_fraction_digits: usize,
    /// Whether to use the grouping separator
    grouping: bool,
    /// Grouping size
    grouping_size: usize,
    /// Whether `%` appears
    percent: bool,
}

/// Number format specific to MPX.
///
/// Corresponds to Java: `public final class MPXJNumberFormat extends DecimalFormat`
///
/// Java inherits `DecimalFormat` to get `parse` / `format` / `applyPattern`;
/// Rust has this struct own the pattern and separators (see the file header).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MPXJNumberFormat {
    /// Primary pattern (corresponds to Java `DecimalFormat`'s primary pattern)
    pattern: Option<NumberPattern>,
    /// Alternative pattern list (corresponds to Java's `m_alternativeFormats`)
    alternative_formats: Vec<MPXJNumberFormat>,
    /// Decimal separator (corresponds to Java `m_symbols.getDecimalSeparator()`)
    decimal_separator: char,
    /// Grouping separator (corresponds to Java `m_symbols.getGroupingSeparator()`)
    grouping_separator: char,
}

impl MPXJNumberFormat {
    /// Construct a number format.
    ///
    /// Corresponds to Java: `new DecimalFormat()` (implicit constructor)
    ///
    /// Java uses the JVM default locale's number pattern (under English, `#,##0.###`); Rust always uses
    /// the English defaults and separators, which `apply_pattern` overrides from the project properties.
    pub fn new() -> MPXJNumberFormat {
        MPXJNumberFormat {
            pattern: compile_number_pattern("#,##0.###"),
            alternative_formats: Vec::new(),
            decimal_separator: '.',
            grouping_separator: ',',
        }
    }

    /// Set the primary and alternative patterns and update the locale-related separators.
    ///
    /// Corresponds to Java:
    /// `public void applyPattern(String primaryPattern, String[] alternativePatterns, char decimalSeparator, char groupingSeparator)`
    ///
    /// Java's `String[]` may be `null` (meaning the alternative patterns are not updated); Rust uses
    /// `Option<&[String]>` to express this. When the number of alternative patterns changes, Java rebuilds
    /// the array; this also rebuilds it (when the length is unchanged it keeps the existing objects and
    /// overwrites the pattern and separators one by one, equivalent to Java's per-item `applyPattern`).
    pub fn apply_pattern(
        &mut self,
        primary_pattern: &str,
        alternative_patterns: Option<&[String]>,
        decimal_separator: char,
        grouping_separator: char,
    ) {
        self.decimal_separator = decimal_separator;
        self.grouping_separator = grouping_separator;

        self.apply_primary_pattern(primary_pattern);

        if let Some(alternative_patterns) = alternative_patterns {
            if !alternative_patterns.is_empty() {
                if self.alternative_formats.len() != alternative_patterns.len() {
                    self.alternative_formats = alternative_patterns
                        .iter()
                        .map(|_| MPXJNumberFormat::new())
                        .collect();
                }

                for (format, pattern) in self
                    .alternative_formats
                    .iter_mut()
                    .zip(alternative_patterns.iter())
                {
                    // Java: alternativeFormat.setDecimalFormatSymbols(m_symbols)
                    format.decimal_separator = decimal_separator;
                    format.grouping_separator = grouping_separator;
                    format.apply_primary_pattern(pattern);
                }
            }
        }
    }

    /// Set the primary pattern.
    ///
    /// Corresponds to Java: `public void applyPattern(String pattern)` inherited from `DecimalFormat`
    ///
    /// Java throws `IllegalArgumentException` for an invalid pattern; Rust records the primary pattern as
    /// missing (subsequent [`MPXJNumberFormat::parse`] returns `None` and [`MPXJNumberFormat::format`] falls back).
    fn apply_primary_pattern(&mut self, pattern: &str) {
        self.pattern = compile_number_pattern(pattern);
    }

    /// Parse text into a number.
    ///
    /// Corresponds to Java: `@Override public Number parse(String str, ParsePosition parsePosition)`
    ///
    /// Behavior matches Java: `null` / empty text is not parsed; the primary pattern is tried first, and
    /// when it does not match the alternative patterns are tried in turn; if all fail, returns `None`
    /// (Java keeps `parsePosition` at 0 and returns `null`). As in Java, parsing does not require consuming
    /// the whole string (e.g. `"12abc"` parses as `12`).
    pub fn parse(&self, text: Option<&str>) -> Option<f64> {
        let text = text?.trim();
        if text.is_empty() {
            // Java: parsePosition.setIndex(-1)
            return None;
        }

        if let Some((value, _)) = self.parse_at(text) {
            return Some(value);
        }

        for alternative_format in &self.alternative_formats {
            if let Some((value, _)) = alternative_format.parse_at(text) {
                return Some(value);
            }
        }

        // Java: returns null after all alternative patterns end with index == 0
        None
    }

    /// Format a number using the primary pattern.
    ///
    /// Corresponds to Java: `public String format(double number)` inherited from `DecimalFormat`
    ///
    /// Minimum integer digits / minimum fraction digits are zero-padded, and digits beyond the maximum
    /// fraction digits are rounded `HALF_EVEN` (Java's `DecimalFormat` also defaults to `HALF_EVEN`, and
    /// Rust's `{:.N}` does the same).
    pub fn format(&self, value: f64) -> String {
        let Some(pattern) = self.pattern.as_ref() else {
            // Java: uses the locale default pattern when there is no pattern; Rust falls back to a plain decimal representation
            return format_fallback(value);
        };

        let value = if pattern.percent {
            value * 100.0
        } else {
            value
        };
        let negative = value < 0.0;
        let magnitude = value.abs();

        let fixed = format!("{:.*}", pattern.max_fraction_digits, magnitude);
        let (integer_text, fraction_text) = match fixed.split_once('.') {
            Some((integer_text, fraction_text)) => (integer_text.to_owned(), fraction_text.to_owned()),
            None => (fixed, String::new()),
        };

        let fraction_text = trim_fraction(&fraction_text, pattern.min_fraction_digits);
        let integer_text = pad_integer(
            &integer_text,
            pattern.min_integer_digits,
            !fraction_text.is_empty(),
        );
        let integer_text = if pattern.grouping {
            group_digits(&integer_text, pattern.grouping_size, self.grouping_separator)
        } else {
            integer_text
        };

        let mut result = String::new();
        result.push_str(if negative {
            &pattern.negative_prefix
        } else {
            &pattern.positive_prefix
        });
        result.push_str(&integer_text);
        if !fraction_text.is_empty() {
            result.push(self.decimal_separator);
            result.push_str(&fraction_text);
        }
        result.push_str(if negative {
            &pattern.negative_suffix
        } else {
            &pattern.positive_suffix
        });
        result
    }

    /// Parse `text` from position 0 using this pattern, returning "value + consumed character count".
    ///
    /// Corresponds to Java: the `subparse` part of `DecimalFormat.parse(String, ParsePosition)`;
    /// returning `None` corresponds to Java's "`ParsePosition.getIndex() == 0`".
    fn parse_at(&self, text: &str) -> Option<(f64, usize)> {
        let pattern = self.pattern.as_ref()?;
        let chars: Vec<char> = text.chars().collect();
        let mut pos = 0;

        // Java: NumberFormat.parse allows leading whitespace
        while pos < chars.len() && chars[pos].is_whitespace() {
            pos += 1;
        }

        // Positive / negative prefix; Java still tries to parse digits when the prefix does not match, kept consistent here
        let mut negative = false;
        if let Some(end) = match_text(&chars, pos, &pattern.negative_prefix) {
            pos = end;
            negative = true;
        } else if let Some(end) = match_text(&chars, pos, &pattern.positive_prefix) {
            pos = end;
        }

        // Sign when there is no prefix
        if let Some(&sign) = chars.get(pos) {
            if sign == '-' {
                negative = true;
                pos += 1;
            } else if sign == '+' {
                pos += 1;
            }
        }

        let mut digits = String::new();
        let mut has_digit = false;

        while let Some(&current) = chars.get(pos) {
            if current.is_ascii_digit() {
                digits.push(current);
                has_digit = true;
                pos += 1;
            } else if current == self.grouping_separator {
                // Java: DecimalFormat.isGroupingUsed() defaults to true, so parsing always accepts the grouping separator
                pos += 1;
            } else {
                break;
            }
        }

        if let Some(&current) = chars.get(pos) {
            if current == self.decimal_separator {
                pos += 1;
                digits.push('.');
                while let Some(&current) = chars.get(pos) {
                    if current.is_ascii_digit() {
                        digits.push(current);
                        has_digit = true;
                        pos += 1;
                    } else {
                        break;
                    }
                }
            }
        }

        if !has_digit {
            return None;
        }

        let suffix = if negative {
            &pattern.negative_suffix
        } else {
            &pattern.positive_suffix
        };
        if let Some(end) = match_text(&chars, pos, suffix) {
            pos = end;
        }

        let value = digits.parse::<f64>().ok()?;
        let value = if negative { -value } else { value };
        let value = if pattern.percent { value / 100.0 } else { value };
        Some((value, pos))
    }
}

impl Default for MPXJNumberFormat {
    fn default() -> Self {
        MPXJNumberFormat::new()
    }
}

/// Compile a `DecimalFormat` pattern string into a [`NumberPattern`].
///
/// Corresponds to Java: `java.text.DecimalFormat.applyPattern(String)` (including `Pattern` validation)
///
/// Differences from Java:
/// - For an invalid pattern (no digit placeholder, unclosed quote, etc.) Java throws `IllegalArgumentException`,
///   while this returns `None`;
/// - Exponent notation (`E`) and digit-count differences in the negative sub-pattern are ignored after
///   parsing as in Java (MPX does not use them).
fn compile_number_pattern(pattern: &str) -> Option<NumberPattern> {
    let sub_patterns = split_sub_patterns(pattern);
    let positive = split_sub_pattern(sub_patterns.first()?)?;

    // Java: when there is no negative sub-pattern, the negative prefix is "-" + the positive prefix and the suffix is the positive suffix
    let negative = match sub_patterns.get(1) {
        Some(text) => split_sub_pattern(text)?,
        None => SubPattern {
            prefix: format!("-{}", positive.prefix),
            suffix: positive.suffix.clone(),
            ..positive.clone()
        },
    };

    Some(NumberPattern {
        positive_prefix: positive.prefix,
        positive_suffix: positive.suffix,
        negative_prefix: negative.prefix,
        negative_suffix: negative.suffix,
        min_integer_digits: positive.min_integer_digits,
        min_fraction_digits: positive.min_fraction_digits,
        max_fraction_digits: positive.max_fraction_digits,
        grouping: positive.grouping,
        grouping_size: positive.grouping_size,
        percent: positive.percent,
    })
}

/// Split the pattern into positive / negative sub-patterns on **unquoted** `;`.
///
/// Corresponds to Java: `DecimalFormat`'s handling of `;` as a sub-pattern separator.
fn split_sub_patterns(pattern: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut chars = pattern.chars();

    while let Some(character) = chars.next() {
        match character {
            '\'' => {
                // The quoted section is kept as a whole; a ';' inside it is not a separator
                current.push(character);
                for quoted in chars.by_ref() {
                    current.push(quoted);
                    if quoted == '\'' {
                        break;
                    }
                }
            }
            ';' => result.push(std::mem::take(&mut current)),
            _ => current.push(character),
        }
    }

    result.push(current);
    result
}

/// Parse a single sub-pattern: separate the prefix / suffix / digit placeholders.
///
/// Corresponds to Java: `DecimalFormat`'s sub-pattern parsing (`Pattern` syntax).
fn split_sub_pattern(text: &str) -> Option<SubPattern> {
    // Expand character by character: `(character, is-pattern-syntax-character)`; everything inside a quoted section is a literal
    let mut items: Vec<(char, bool)> = Vec::new();
    let mut percent = false;
    let mut chars = text.chars();

    while let Some(character) = chars.next() {
        if character == '\'' {
            if chars.as_str().starts_with('\'') {
                // `''` → a single literal quote
                chars.next();
                items.push(('\'', false));
                continue;
            }

            for quoted in chars.by_ref() {
                if quoted == '\'' {
                    break;
                }
                items.push((quoted, false));
            }
            continue;
        }

        if character == '%' || character == '\u{2030}' {
            // Percent / per-mille sign: kept in the prefix or suffix while recording the scaling flag
            percent = true;
        }

        items.push((character, matches!(character, '#' | '0' | '.' | ',' | 'E')));
    }

    let first = items.iter().position(|(_, syntax)| *syntax)?;
    let last = items.iter().rposition(|(_, syntax)| *syntax)?;

    let core = &items[first..=last];
    if !core.iter().any(|(character, _)| *character == '#' || *character == '0') {
        // Java: the pattern must contain at least one digit placeholder
        return None;
    }

    let prefix: String = items[..first].iter().map(|(character, _)| *character).collect();
    let suffix: String = items[last + 1..].iter().map(|(character, _)| *character).collect();

    let mut min_integer_digits = 0;
    let mut min_fraction_digits = 0;
    let mut max_fraction_digits = 0;
    let mut grouping = false;
    let mut digits_after_grouping = 0;
    let mut fraction = false;

    for (character, syntax) in core {
        if !syntax {
            // Quoted characters are literals even if they fall inside the digit core range (MPX patterns never do this)
            continue;
        }

        match character {
            '.' => fraction = true,
            ',' => {
                grouping = true;
                digits_after_grouping = 0;
            }
            '#' => {
                if fraction {
                    max_fraction_digits += 1;
                } else {
                    digits_after_grouping += 1;
                }
            }
            '0' => {
                if fraction {
                    min_fraction_digits += 1;
                    max_fraction_digits += 1;
                } else {
                    min_integer_digits += 1;
                    digits_after_grouping += 1;
                }
            }
            // 'E': exponent notation, ignored by this port (MPX does not use it)
            _ => {}
        }
    }

    Some(SubPattern {
        prefix,
        suffix,
        min_integer_digits,
        min_fraction_digits,
        max_fraction_digits,
        grouping,
        // Java: grouping size is the number of digits after the last grouping separator, defaulting to 3
        grouping_size: if digits_after_grouping == 0 {
            3
        } else {
            digits_after_grouping
        },
        percent,
    })
}

/// Match literal text starting at `pos`; returns the new position on success.
///
/// Corresponds to Java: `String.regionMatches` / prefix-suffix matching in `DecimalFormat.parse`.
fn match_text(chars: &[char], pos: usize, text: &str) -> Option<usize> {
    let mut index = pos;
    for expected in text.chars() {
        if chars.get(index).copied() != Some(expected) {
            return None;
        }
        index += 1;
    }
    Some(index)
}

/// Trim trailing `0`s from the fraction part, keeping at least `min_digits` digits.
fn trim_fraction(fraction: &str, min_digits: usize) -> String {
    let mut end = fraction.len();
    while end > min_digits && fraction[..end].ends_with('0') {
        end -= 1;
    }
    fraction[..end].to_owned()
}

/// Pad the integer part with leading zeros to the minimum integer digits.
///
/// Corresponds to Java's `#` semantics: when the minimum integer digits is 0 and a fraction part exists,
/// the integer part's `0` is also omitted (e.g. pattern `#.##` formatting `0.5` yields `.5`).
fn pad_integer(integer: &str, min_digits: usize, has_fraction: bool) -> String {
    if min_digits == 0 && integer == "0" && has_fraction {
        return String::new();
    }

    if integer.len() >= min_digits {
        return integer.to_owned();
    }

    let mut result = String::new();
    for _ in 0..(min_digits - integer.len()) {
        result.push('0');
    }
    result.push_str(integer);
    result
}

/// Insert grouping separators by grouping size (integer part only).
fn group_digits(integer: &str, grouping_size: usize, separator: char) -> String {
    if grouping_size == 0 || integer.len() <= grouping_size {
        return integer.to_owned();
    }

    let chars: Vec<char> = integer.chars().collect();
    let mut result = String::new();
    for (index, character) in chars.iter().enumerate() {
        if index > 0 && (chars.len() - index) % grouping_size == 0 {
            result.push(separator);
        }
        result.push(*character);
    }
    result
}

/// Number representation when there is no pattern (Java uses the locale default number pattern; this uses a plain decimal representation).
fn format_fallback(value: f64) -> String {
    format!("{value}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Construct a number format with the given pattern applied.
    fn format_with(pattern: &str, decimal_separator: char, grouping_separator: char) -> MPXJNumberFormat {
        let mut format = MPXJNumberFormat::new();
        format.apply_pattern(pattern, None, decimal_separator, grouping_separator);
        format
    }

    #[test]
    fn parse_handles_null_and_empty_text() {
        let format = format_with("0.00#", '.', ',');
        assert_eq!(format.parse(None), None);
        assert_eq!(format.parse(Some("")), None);
        assert_eq!(format.parse(Some("   ")), None);
        assert_eq!(format.parse(Some("abc")), None);
    }

    #[test]
    fn parse_reads_plain_numbers() {
        let format = format_with("0.00#", '.', ',');
        assert_eq!(format.parse(Some("12.5")), Some(12.5));
        assert_eq!(format.parse(Some("12.50")), Some(12.5));
        // Java: NumberFormat.parse allows leading whitespace and does not require consuming the whole string
        assert_eq!(format.parse(Some("  12.5 kilograms")), Some(12.5));
        assert_eq!(format.parse(Some("-12.5")), Some(-12.5));
    }

    #[test]
    fn parse_accepts_grouping_separators() {
        // Java's DecimalFormat.isGroupingUsed() defaults to true
        let format = format_with("#0.00", '.', ',');
        assert_eq!(format.parse(Some("1,234.50")), Some(1234.5));
    }

    #[test]
    fn parse_uses_alternative_patterns() {
        // Currency format: the primary pattern has the symbol, the alternative patterns do not
        let alternatives = vec![
            "$#0.00;($#0.00)".to_owned(),
            "$#,#0.00".to_owned(),
            "$#,#0.00;($#,#0.00)".to_owned(),
            "#0.00".to_owned(),
            "#0.00;(#0.00)".to_owned(),
            "#,#0.00".to_owned(),
            "#,#0.00;(#,#0.00)".to_owned(),
        ];
        let mut format = MPXJNumberFormat::new();
        format.apply_pattern("$#0.00", Some(&alternatives), '.', ',');

        assert_eq!(format.parse(Some("$1,234.50")), Some(1234.5));
        // No symbol → uses an alternative pattern
        assert_eq!(format.parse(Some("1,234.50")), Some(1234.5));
        // Parentheses denote a negative → uses an alternative pattern
        assert_eq!(format.parse(Some("($1,234.50)")), Some(-1234.5));
        assert_eq!(format.parse(Some("(1,234.50)")), Some(-1234.5));
    }

    #[test]
    fn parse_uses_locale_separators() {
        // German and similar locales: decimal separator ',' and grouping separator '.'
        let format = format_with("#.##", ',', '.');
        assert_eq!(format.parse(Some("1.234,5")), Some(1234.5));
        assert_eq!(format.parse(Some("0,5")), Some(0.5));
    }

    #[test]
    fn format_pads_and_trims_fraction_digits() {
        // Pattern "0.00#": at least two fraction digits, at most three
        let format = format_with("0.00#", '.', ',');
        assert_eq!(format.format(12.5), "12.50");
        assert_eq!(format.format(12.567), "12.567");
        assert_eq!(format.format(12.5678), "12.568");

        // Pattern "#.##": the decimal point is omitted when there is no fraction, and "#" is not zero-padded
        let format = format_with("#.##", '.', ',');
        assert_eq!(format.format(12.5), "12.5");
        assert_eq!(format.format(100.0), "100");
        assert_eq!(format.format(2.345), "2.35");
    }

    #[test]
    fn format_follows_locale_separators_and_grouping() {
        let format = format_with("#,##0.00", ',', '.');
        assert_eq!(format.format(1234.5), "1.234,50");

        // The primary currency pattern has no grouping separator → no grouping
        let format = format_with("$#0.00", '.', ',');
        assert_eq!(format.format(1234.5), "$1234.50");
        assert_eq!(format.format(-1234.5), "-$1234.50");
    }

    #[test]
    fn format_uses_negative_sub_pattern() {
        let mut format = MPXJNumberFormat::new();
        format.apply_pattern("$#0.00;($#0.00)", None, '.', ',');
        assert_eq!(format.format(1234.5), "$1234.50");
        assert_eq!(format.format(-1234.5), "($1234.50)");
    }

    #[test]
    fn format_and_parse_round_trip() {
        let patterns = ["#.##", "0.00#", "##0.##", "#,##0.00"];
        for pattern in patterns {
            let format = format_with(pattern, '.', ',');
            for value in [0.0, 0.5, 1.0, 12.5, 1234.56, -7.25] {
                let text = format.format(value);
                let parsed = format
                    .parse(Some(text.as_str()))
                    .unwrap_or_else(|| panic!("output {text} of pattern {pattern} should parse"));
                assert!(
                    (parsed - value).abs() < 0.005,
                    "pattern {pattern}：{value} → {text} → {parsed}"
                );
            }
        }
    }

    #[test]
    fn invalid_pattern_is_reported_as_missing_pattern() {
        let mut format = MPXJNumberFormat::new();
        format.apply_pattern("abc", None, '.', ',');
        // Java throws IllegalArgumentException; Rust does not panic, parsing fails and formatting falls back
        assert_eq!(format.parse(Some("12")), None);
        assert_eq!(format.format(12.5), "12.5");
    }

    #[test]
    fn quoted_characters_are_literals() {
        // Corresponds to the output of MPXJFormats.quoteFormatCharacters: the currency symbol "." is wrapped in quotes
        // so it is not treated as a decimal separator (quoteFormatCharacters(".") == "'.'")
        let mut format = MPXJNumberFormat::new();
        format.apply_pattern("'.'#0.00", None, '.', ',');
        assert_eq!(format.format(12.5), ".12.50");
        assert_eq!(format.parse(Some(".12.50")), Some(12.5));
    }

    #[test]
    fn equality_is_structural() {
        let lhs = format_with("0.00#", '.', ',');
        let rhs = format_with("0.00#", '.', ',');
        assert_eq!(lhs, rhs);

        let other = format_with("0.00#", ',', '.');
        assert_ne!(lhs, other);
    }
}
