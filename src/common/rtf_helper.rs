//! Corresponds to Java: `org/mpxj/common/RtfHelper.java`
//!
//! # Differences from Java (important)
//!
//! Java uses [rtfparserkit](https://github.com/tim-ocallaghan/rtfparserkit)'s
//! `StringTextConverter` to convert RTF to plain text. This repository does not introduce new
//! dependencies, so the Rust side approximates it with a **lightweight RTF text extractor**:
//!
//! - Outputs body text, handling `\par` / `\line` (newlines), `\tab` (tab), `\\` / `\{` / `\}`
//!   (escaped characters), `\'hh` (ANSI hex escapes, approximated as Windows-1252), `\uN`
//!   (Unicode escapes, with the fallback character count given by `\ucN`) and several common
//!   symbol control words (`\emdash` / `\endash` / `\bullet` / `\lquote`, etc.);
//! - Skips destination groups such as the font table, color table, stylesheet, pictures,
//!   headers and footers, as well as ignorable groups in the `{\*\...}` form;
//! - Bare `\r` / `\n` in the RTF source text are treated as meaningless per the spec and
//!   discarded directly;
//! - Java's `IOException` branch (`result = ""`) is unreachable in the Rust implementation, so
//!   that branch is not kept.
//!
//! Compared with rtfparserkit, this implementation handles malformed input, embedded objects,
//! tables/lists and other complex structures more coarsely, and only guarantees matching
//! extraction results for common RTF note text.

/// A utility for handling RTF-encoded text.
///
/// Corresponds to Java: `public final class RtfHelper`
pub struct RtfHelper;

impl RtfHelper {
    /// Simple heuristic: determine whether the string is formal RTF (starts with `{\rtf`).
    ///
    /// Corresponds to Java: `private static boolean isFormalRTF(String text)`
    fn is_formal_rtf(text: &str) -> bool {
        text.starts_with("{\\rtf")
    }

    /// Remove RTF formatting information from the text and return plain text.
    ///
    /// Corresponds to Java: `public static String strip(String text)`
    ///
    /// `None` returns `None`, an empty string returns an empty string.
    pub fn strip(text: Option<&str>) -> Option<String> {
        let text = text?;

        if text.is_empty() {
            return Some(String::new());
        }

        let formal_rtf = Self::is_formal_rtf(text);
        let converted = convert(text);
        Some(Self::strip_extra_line_end(converted, formal_rtf))
    }

    /// Remove the trailing newline from the result of a formal RTF block.
    ///
    /// Corresponds to Java: `private static String stripExtraLineEnd(String text, boolean formalRTF)`
    fn strip_extra_line_end(text: String, formal_rtf: bool) -> String {
        if formal_rtf && text.ends_with('\n') {
            return text[..text.len() - 1].to_owned();
        }

        text
    }
}

/// Parser state: a group.
#[derive(Clone, Copy)]
struct GroupState {
    /// Whether the current group's content is skipped entirely.
    skip: bool,
    /// The number of `\uN` fallback characters given by `\ucN` (default 1).
    fallback_count: usize,
}

/// Corresponds to Java's `StringTextConverter`: extract RTF text into plain text.
fn convert(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::new();
    let mut stack: Vec<GroupState> = vec![GroupState {
        skip: false,
        fallback_count: 1,
    }];
    let mut fallback_remaining = 0usize;
    let mut index = 0usize;

    while index < chars.len() {
        match chars[index] {
            '{' => {
                let current = *stack.last().unwrap_or(&GroupState {
                    skip: false,
                    fallback_count: 1,
                });
                let (destination_skip, ignorable) = group_destination(&chars, index + 1);
                stack.push(GroupState {
                    skip: current.skip || destination_skip || ignorable,
                    fallback_count: current.fallback_count,
                });
                index += 1;
            }
            '}' => {
                if stack.len() > 1 {
                    stack.pop();
                }
                index += 1;
            }
            // Bare newlines inside a group are ignored per the RTF spec.
            '\r' | '\n' if !is_skipping(&stack) => {
                index += 1;
            }
            '\\' => {
                index = handle_control(
                    &chars,
                    index,
                    &mut stack,
                    &mut result,
                    &mut fallback_remaining,
                );
            }
            c => {
                if !is_skipping(&stack) {
                    if fallback_remaining > 0 {
                        fallback_remaining -= 1;
                    } else {
                        result.push(c);
                    }
                }
                index += 1;
            }
        }
    }

    result
}

/// Whether the current group needs to be skipped entirely.
fn is_skipping(stack: &[GroupState]) -> bool {
    stack.last().map(|state| state.skip).unwrap_or(false)
}

/// Handle a control word / control symbol starting with `\`, returning the next index to process.
fn handle_control(
    chars: &[char],
    index: usize,
    stack: &mut Vec<GroupState>,
    result: &mut String,
    fallback_remaining: &mut usize,
) -> usize {
    let skipping = is_skipping(stack);

    let Some(next) = chars.get(index + 1).copied() else {
        return index + 1;
    };

    match next {
        // Escape character
        '\\' | '{' | '}' => {
            push_controlled(result, fallback_remaining, skipping, next);
            index + 2
        }
        // `\'hh` ANSI hex escape
        '\'' => {
            let digits: String = chars.iter().skip(index + 2).take(2).collect();
            if digits.len() == 2 {
                if let Ok(value) = u8::from_str_radix(&digits, 16) {
                    push_controlled(result, fallback_remaining, skipping, decode_ansi(value));
                }
                index + 4
            } else {
                index + 2
            }
        }
        // `\*` ignorable destination: mark the current group as skipped entirely
        '*' => {
            if let Some(state) = stack.last_mut() {
                state.skip = true;
            }
            index + 2
        }
        // Other control symbols
        _ if !next.is_ascii_alphabetic() => {
            if !skipping {
                let symbol = match next {
                    '~' => Some('\u{A0}'),
                    '_' => Some('\u{2011}'),
                    // `\-` is an optional hyphen; ignored along with other unknown symbols
                    _ => None,
                };

                if let Some(symbol) = symbol {
                    push_controlled(result, fallback_remaining, skipping, symbol);
                }
            }
            index + 2
        }
        // Control word
        _ => {
            let mut word_end = index + 1;
            while word_end < chars.len() && chars[word_end].is_ascii_alphabetic() {
                word_end += 1;
            }
            let word: String = chars[index + 1..word_end].iter().collect();

            let mut param_end = word_end;
            let mut negative = false;
            if param_end < chars.len() && chars[param_end] == '-' {
                negative = true;
                param_end += 1;
            }
            let digits_start = param_end;
            while param_end < chars.len() && chars[param_end].is_ascii_digit() {
                param_end += 1;
            }
            let param = if param_end > digits_start {
                chars[digits_start..param_end]
                    .iter()
                    .collect::<String>()
                    .parse::<i32>()
                    .ok()
                    .map(|value| if negative { -value } else { value })
            } else {
                None
            };

            // A single space after a control word is a delimiter and is not output
            let mut next_index = param_end;
            if next_index < chars.len() && chars[next_index] == ' ' {
                next_index += 1;
            }

            if word == "uc" {
                if let (Some(state), Some(count)) = (stack.last_mut(), param) {
                    state.fallback_count = usize::try_from(count.max(0)).unwrap_or(1);
                }
                return next_index;
            }

            if !skipping {
                match word.as_str() {
                    "par" | "line" => result.push('\n'),
                    "tab" => result.push('\t'),
                    "u" => {
                        if let Some(value) = param {
                            let code_point = if value < 0 {
                                (value + 65536) as u32
                            } else {
                                value as u32
                            };
                            push_controlled(
                                result,
                                fallback_remaining,
                                skipping,
                                char::from_u32(code_point).unwrap_or('\u{FFFD}'),
                            );
                            *fallback_remaining =
                                stack.last().map(|state| state.fallback_count).unwrap_or(1);
                        }
                    }
                    "emdash" => result.push('\u{2014}'),
                    "endash" => result.push('\u{2013}'),
                    "bullet" => result.push('\u{2022}'),
                    "lquote" => result.push('\u{2018}'),
                    "rquote" => result.push('\u{2019}'),
                    "ldblquote" => result.push('\u{201C}'),
                    "rdblquote" => result.push('\u{201D}'),
                    "emspace" | "enspace" => result.push(' '),
                    "nbsp" => result.push('\u{A0}'),
                    _ => {}
                }
            }

            next_index
        }
    }
}

/// Decide whether to output a character or discard it based on the "fallback character" count.
fn push_controlled(
    result: &mut String,
    fallback_remaining: &mut usize,
    skipping: bool,
    value: char,
) {
    if skipping {
        return;
    }

    if *fallback_remaining > 0 {
        *fallback_remaining -= 1;
    } else {
        result.push(value);
    }
}

/// Determine whether the first control word of a group is a destination that must be skipped.
///
/// Returns `(whether to skip, whether it is an ignorable destination)`.
fn group_destination(chars: &[char], start: usize) -> (bool, bool) {
    let Some(first) = chars.get(start).copied() else {
        return (false, false);
    };

    if first != '\\' {
        return (false, false);
    }

    // Ignorable destination in the `{\*\...}` form
    if chars.get(start + 1).copied() == Some('*') {
        return (false, true);
    }

    if chars.get(start + 1).copied().is_none_or(|c| !c.is_ascii_alphabetic()) {
        return (false, false);
    }

    let mut end = start + 1;
    while end < chars.len() && chars[end].is_ascii_alphabetic() {
        end += 1;
    }
    let word: String = chars[start + 1..end].iter().collect();

    (is_skippable_destination(&word), false)
}

/// RTF destinations whose control words require skipping the whole group.
fn is_skippable_destination(word: &str) -> bool {
    matches!(
        word,
        "fonttbl"
            | "colortbl"
            | "stylesheet"
            | "info"
            | "pict"
            | "shppict"
            | "nonshppict"
            | "object"
            | "header"
            | "footer"
            | "headerl"
            | "headerr"
            | "headerf"
            | "footerl"
            | "footerr"
            | "footerf"
            | "footnote"
            | "annotation"
            | "listtable"
            | "listoverridetable"
            | "revtbl"
            | "rsidtbl"
            | "generator"
            | "xmlnstbl"
            | "datastore"
            | "themedata"
            | "colorschememapping"
            | "latentstyles"
            | "docvar"
            | "pnseclvl"
            | "mmath"
            | "private"
    )
}

/// Approximately decode an ANSI (Windows-1252) byte to a character.
///
/// The 0x80-0x9F range is mapped per Windows-1252, and the rest is treated as Latin-1.
fn decode_ansi(value: u8) -> char {
    let mapped = match value {
        0x80 => '\u{20AC}',
        0x82 => '\u{201A}',
        0x83 => '\u{0192}',
        0x84 => '\u{201E}',
        0x85 => '\u{2026}',
        0x86 => '\u{2020}',
        0x87 => '\u{2021}',
        0x88 => '\u{02C6}',
        0x89 => '\u{2030}',
        0x8A => '\u{0160}',
        0x8B => '\u{2039}',
        0x8C => '\u{0152}',
        0x8E => '\u{017D}',
        0x91 => '\u{2018}',
        0x92 => '\u{2019}',
        0x93 => '\u{201C}',
        0x94 => '\u{201D}',
        0x95 => '\u{2022}',
        0x96 => '\u{2013}',
        0x97 => '\u{2014}',
        0x98 => '\u{02DC}',
        0x99 => '\u{2122}',
        0x9A => '\u{0161}',
        0x9B => '\u{203A}',
        0x9C => '\u{0153}',
        0x9E => '\u{017E}',
        0x9F => '\u{0178}',
        _ => return value as char,
    };

    mapped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_returns_none_for_null_and_empty_for_empty() {
        assert_eq!(RtfHelper::strip(None), None);
        assert_eq!(RtfHelper::strip(Some("")), Some(String::new()));
    }

    #[test]
    fn strip_leaves_plain_text_unchanged() {
        assert_eq!(
            RtfHelper::strip(Some("plain text")),
            Some("plain text".to_owned())
        );
    }

    #[test]
    fn is_formal_rtf_matches_java_heuristic() {
        assert!(RtfHelper::is_formal_rtf("{\\rtf1\\ansi}"));
        assert!(!RtfHelper::is_formal_rtf("plain"));
        assert!(!RtfHelper::is_formal_rtf("{ rtf"));
    }

    #[test]
    fn strip_handles_paragraphs_and_escapes() {
        // A single space after a control word is a delimiter and does not enter the output
        assert_eq!(
            RtfHelper::strip(Some("{\\rtf1 Hello\\par World}")),
            Some("Hello\nWorld".to_owned())
        );

        assert_eq!(
            RtfHelper::strip(Some("{\\rtf1 a\\{b\\}c}")),
            Some("a{b}c".to_owned())
        );
    }

    #[test]
    fn strip_removes_trailing_line_end_for_formal_rtf() {
        // \par produces a trailing newline, which formal RTF removes
        assert_eq!(
            RtfHelper::strip(Some("{\\rtf1 Hello\\par}")),
            Some("Hello".to_owned())
        );

        // Non-formal RTF: no trailing-newline handling
        assert_eq!(
            RtfHelper::strip(Some("Hello\\par")),
            Some("Hello\n".to_owned())
        );
    }

    #[test]
    fn strip_decodes_hex_and_unicode_escapes() {
        assert_eq!(
            RtfHelper::strip(Some("{\\rtf1 caf\\'e9}")),
            Some("café".to_owned())
        );

        assert_eq!(
            RtfHelper::strip(Some("{\\rtf1 \\u233?}")),
            Some("é".to_owned())
        );
    }

    #[test]
    fn strip_skips_destinations_and_ignorable_groups() {
        assert_eq!(
            RtfHelper::strip(Some(
                "{\\rtf1{\\fonttbl{\\f0 Arial;}}Hello{\\*\\generator Test;} world}"
            )),
            Some("Hello world".to_owned())
        );
    }

    #[test]
    fn strip_handles_tabs() {
        assert_eq!(
            RtfHelper::strip(Some("{\\rtf1 a\\tab b}")),
            Some("a\tb".to_owned())
        );
    }
}
