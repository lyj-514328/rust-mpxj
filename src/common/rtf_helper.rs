//! 对应 Java: `org/mpxj/common/RtfHelper.java`
//!
//! # 与 Java 的差异（重要）
//!
//! Java 侧用 [rtfparserkit](https://github.com/tim-ocallaghan/rtfparserkit) 的
//! `StringTextConverter` 把 RTF 转成纯文本。本仓库不引入新依赖，Rust 侧用一个
//! **轻量 RTF 文本抽取器**近似实现：
//!
//! - 输出正文文本，处理 `\par` / `\line`（换行）、`\tab`（制表符）、
//!   `\\` / `\{` / `\}`（转义字符）、`\'hh`（ANSI 十六进制转义，按 Windows-1252 近似解码）、
//!   `\uN`（Unicode 转义，配合 `\ucN` 指定的回退字符数）以及若干常用符号控制字
//!   （`\emdash` / `\endash` / `\bullet` / `\lquote` 等）；
//! - 跳过字体表、颜色表、样式表、图片、页眉页脚等目的（destination）组，
//!   以及 `{\*\...}` 形式的可忽略组；
//! - RTF 源文本中的裸 `\r` / `\n` 按规范视为无意义，直接丢弃；
//! - Java 的 `IOException` 分支（`result = ""`）在 Rust 实现中不可达，因此不保留该分支。
//!
//! 与 rtfparserkit 相比，本实现对畸形输入、嵌入对象、表格/列表等复杂结构的处理更粗略，
//! 仅保证常见 RTF 笔记文本的抽取结果一致。

/// RTF 编码文本的处理工具。
///
/// 对应 Java: `public final class RtfHelper`
pub struct RtfHelper;

impl RtfHelper {
    /// 简单启发式：判断字符串是否为正式 RTF（以 `{\rtf` 开头）。
    ///
    /// 对应 Java: `private static boolean isFormalRTF(String text)`
    fn is_formal_rtf(text: &str) -> bool {
        text.starts_with("{\\rtf")
    }

    /// 去掉文本中的 RTF 格式信息，返回纯文本。
    ///
    /// 对应 Java: `public static String strip(String text)`
    ///
    /// `None` 返回 `None`，空串返回空串。
    pub fn strip(text: Option<&str>) -> Option<String> {
        let text = text?;

        if text.is_empty() {
            return Some(String::new());
        }

        let formal_rtf = Self::is_formal_rtf(text);
        let converted = convert(text);
        Some(Self::strip_extra_line_end(converted, formal_rtf))
    }

    /// 去掉正式 RTF 块结果末尾的换行。
    ///
    /// 对应 Java: `private static String stripExtraLineEnd(String text, boolean formalRTF)`
    fn strip_extra_line_end(text: String, formal_rtf: bool) -> String {
        if formal_rtf && text.ends_with('\n') {
            return text[..text.len() - 1].to_owned();
        }

        text
    }
}

/// 解析器状态：一个组（group）。
#[derive(Clone, Copy)]
struct GroupState {
    /// 当前组内容是否整体跳过。
    skip: bool,
    /// `\ucN` 指定的 `\uN` 回退字符数（默认 1）。
    fallback_count: usize,
}

/// 对应 Java 的 `StringTextConverter`：把 RTF 文本抽取为纯文本。
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
            // 组内的裸换行按 RTF 规范忽略。
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

/// 当前组是否需要整体跳过。
fn is_skipping(stack: &[GroupState]) -> bool {
    stack.last().map(|state| state.skip).unwrap_or(false)
}

/// 处理一个 `\` 起始的控制字 / 控制符号，返回下一个待处理的下标。
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
        // 转义字符
        '\\' | '{' | '}' => {
            push_controlled(result, fallback_remaining, skipping, next);
            index + 2
        }
        // `\'hh` ANSI 十六进制转义
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
        // `\*` 可忽略目的：把当前组整体标记为跳过
        '*' => {
            if let Some(state) = stack.last_mut() {
                state.skip = true;
            }
            index + 2
        }
        // 其它控制符号
        _ if !next.is_ascii_alphabetic() => {
            if !skipping {
                let symbol = match next {
                    '~' => Some('\u{A0}'),
                    '_' => Some('\u{2011}'),
                    // `\-` 为可选连字符，连同其它未知符号一并忽略
                    _ => None,
                };

                if let Some(symbol) = symbol {
                    push_controlled(result, fallback_remaining, skipping, symbol);
                }
            }
            index + 2
        }
        // 控制字
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

            // 控制字后的单个空格是分隔符，不输出
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

/// 按「回退字符」计数决定是输出字符还是丢弃。
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

/// 判断一个组的首个控制字是否为需要跳过的目的（destination）。
///
/// 返回 `(是否跳过, 是否可忽略目的)`。
fn group_destination(chars: &[char], start: usize) -> (bool, bool) {
    let Some(first) = chars.get(start).copied() else {
        return (false, false);
    };

    if first != '\\' {
        return (false, false);
    }

    // `{\*\...}` 形式的可忽略目的
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

/// 需要整体跳过的 RTF 目的（destination）控制字。
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

/// 把 ANSI（Windows-1252）字节近似解码为字符。
///
/// 0x80-0x9F 段按 Windows-1252 映射，其余按 Latin-1 处理。
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
        // 控制字后的单个空格是分隔符，不进入输出
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
        // `\par` 产生末尾换行，正式 RTF 会把它去掉
        assert_eq!(
            RtfHelper::strip(Some("{\\rtf1 Hello\\par}")),
            Some("Hello".to_owned())
        );

        // 非正式 RTF：不做末尾换行处理
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
