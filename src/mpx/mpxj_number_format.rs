//! 对应 Java: `org/mpxj/mpx/MPXJNumberFormat.java`
//!
//! # 移植取舍（`DecimalFormat` / `DecimalFormatSymbols` / `ParsePosition`）
//!
//! Java 侧 `MPXJNumberFormat extends DecimalFormat`，依赖 `java.text` 的数字格式化能力
//! （模式串、区域设置符号、`BigDecimal` 精确十进制运算）。Rust 标准库没有对应物，
//! 本模块的处理方式：
//!
//! - 用本模块私有的 [`NumberPattern`] 承载 `DecimalFormat` 模式串中本类真正用到的部分：
//!   正 / 负子模式的前后缀、最少整数位、最少 / 最多小数位、分组（千分位）分隔符与百分号。
//!   模式中未用到的能力做如下降级并在 [`compile_number_pattern`] 处注明：指数记法（`E`）
//!   解析后忽略；无法解析的模式（Java 抛 `IllegalArgumentException`）记为「无模式」而不 panic。
//! - `DecimalFormatSymbols` → 本结构自行保存小数分隔符与分组分隔符（等价于 Java 侧的
//!   `setDecimalFormatSymbols`），默认值取英文区域（`.` 与 `,`）。
//! - 继承自 `DecimalFormat` 的方法只用到 `parse(String)` 与 `format(double)`：
//!   `parse` → [`MPXJNumberFormat::parse`]（返回 `Option<f64>`），
//!   `format` → [`MPXJNumberFormat::format`]。Java 用 `BigDecimal` 保证十进制精确，
//!   这里改用 `f64`（`str::parse` / `format!`），仅在极端精度场景有差异。
//! - `ParsePosition` → Rust 侧用 `Option<f64>` 表达 Java 的「解析成功（index 前进）」与
//!   「解析失败（index 保持 / 置 -1）」，内部私有函数 [`MPXJNumberFormat::parse_at`]
//!   返回「值 + 已消费字符数」以复现替代模式的尝试逻辑。
//! - 分组分隔符：Java 的 `DecimalFormat.isGroupingUsed()` 默认为 `true`，因此解析时
//!   始终接受分组分隔符（MPXJ 额外准备 `#,#` 形式的替代模式是为了覆盖**打印**与
//!   货币符号有无的差异），这里同样始终接受。
//! - `equals` / `hashCode` → `PartialEq` / `Eq` / `Hash`：Java 的 `equals` 先调用
//!   `super.equals(obj)`，而 `DecimalFormat` 未覆写 `equals`，实际退化为**引用相等**；
//!   Rust 侧按字段做结构化比较（更符合直觉，差异在此注明）。
//!
//! 未移植的成员：`hashCode()` / `equals(Object)`（改用 `PartialEq` / `Eq` / `Hash` 派生）。

/// 数字格式模式（`DecimalFormat` 模式串中本类用到的部分）。
///
/// 对应 Java: `DecimalFormat` 内部由 `applyPattern(String)` 解析出的模式。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NumberPattern {
    /// 正数前缀（字面量）
    positive_prefix: String,
    /// 正数后缀（字面量）
    positive_suffix: String,
    /// 负数前缀（未显式给出负子模式时为 `-` + 正数前缀）
    negative_prefix: String,
    /// 负数后缀（未显式给出负子模式时同正数后缀）
    negative_suffix: String,
    /// 最少整数位（模式中整数部分的 `0` 个数）
    min_integer_digits: usize,
    /// 最少小数位（小数部分 `0` 的个数）
    min_fraction_digits: usize,
    /// 最多小数位（小数部分 `0` 与 `#` 的个数之和）
    max_fraction_digits: usize,
    /// 是否使用分组（千分位）分隔符
    grouping: bool,
    /// 分组长度（最后一个分组分隔符之后的数字位数；Java 默认 3）
    grouping_size: usize,
    /// 模式中是否出现 `%`（解析时除以 100，打印时乘以 100）
    percent: bool,
}

/// 单个子模式（正或负）的解析结果。
#[derive(Debug, Clone)]
struct SubPattern {
    /// 前缀字面量
    prefix: String,
    /// 后缀字面量
    suffix: String,
    /// 最少整数位
    min_integer_digits: usize,
    /// 最少小数位
    min_fraction_digits: usize,
    /// 最多小数位
    max_fraction_digits: usize,
    /// 是否使用分组分隔符
    grouping: bool,
    /// 分组长度
    grouping_size: usize,
    /// 是否出现 `%`
    percent: bool,
}

/// MPX 专用的数字格式。
///
/// 对应 Java: `public final class MPXJNumberFormat extends DecimalFormat`
///
/// Java 侧继承 `DecimalFormat` 获得 `parse` / `format` / `applyPattern`；
/// Rust 侧以本结构自持模式与分隔符（见文件头）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MPXJNumberFormat {
    /// 主模式（对应 Java `DecimalFormat` 的主模式）
    pattern: Option<NumberPattern>,
    /// 替代模式列表（对应 Java 的 `m_alternativeFormats`）
    alternative_formats: Vec<MPXJNumberFormat>,
    /// 小数分隔符（对应 Java `m_symbols.getDecimalSeparator()`）
    decimal_separator: char,
    /// 分组分隔符（对应 Java `m_symbols.getGroupingSeparator()`）
    grouping_separator: char,
}

impl MPXJNumberFormat {
    /// 构造一个数字格式。
    ///
    /// 对应 Java: `new DecimalFormat()`（隐式构造方法）
    ///
    /// Java 使用 JVM 默认区域设置的数字模式（英文下为 `#,##0.###`）；Rust 侧固定使用
    /// 英文默认值与分隔符，`apply_pattern` 会按项目属性覆盖它们。
    pub fn new() -> MPXJNumberFormat {
        MPXJNumberFormat {
            pattern: compile_number_pattern("#,##0.###"),
            alternative_formats: Vec::new(),
            decimal_separator: '.',
            grouping_separator: ',',
        }
    }

    /// 设置主模式与替代模式，并更新区域设置相关的分隔符。
    ///
    /// 对应 Java:
    /// `public void applyPattern(String primaryPattern, String[] alternativePatterns, char decimalSeparator, char groupingSeparator)`
    ///
    /// Java 的 `String[]` 可传 `null`（表示不更新替代模式），Rust 侧用 `Option<&[String]>`
    /// 表达；替代模式数量变化时 Java 会重建数组，这里同样重建（长度不变时保留已有对象，
    /// 逐个覆盖模式与分隔符，与 Java 逐项 `applyPattern` 等价）。
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

    /// 设置主模式。
    ///
    /// 对应 Java: 继承自 `DecimalFormat` 的 `public void applyPattern(String pattern)`
    ///
    /// Java 对非法模式抛 `IllegalArgumentException`；Rust 侧记为主模式缺失
    /// （后续 [`MPXJNumberFormat::parse`] 返回 `None`、[`MPXJNumberFormat::format`] 回落）。
    fn apply_primary_pattern(&mut self, pattern: &str) {
        self.pattern = compile_number_pattern(pattern);
    }

    /// 解析文本为数值。
    ///
    /// 对应 Java: `@Override public Number parse(String str, ParsePosition parsePosition)`
    ///
    /// 行为与 Java 一致：`null` / 空串不解析；先按主模式解析，主模式未匹配时依次尝试
    /// 替代模式；全部失败返回 `None`（Java 侧对应 `parsePosition` 保持 0 并返回 `null`）。
    /// 与 Java 相同，解析不要求消费完整个字符串（例如 `"12abc"` 解析为 `12`）。
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

        // Java: 所有替代模式都以 index == 0 结束后返回 null
        None
    }

    /// 按主模式格式化数值。
    ///
    /// 对应 Java: 继承自 `DecimalFormat` 的 `public String format(double number)`
    ///
    /// 最小整数位 / 最少小数位补零，最多小数位之外的位数按 `HALF_EVEN` 舍入
    /// （Java 的 `DecimalFormat` 默认舍入模式也是 `HALF_EVEN`，Rust 的 `{:.N}` 同样如此）。
    pub fn format(&self, value: f64) -> String {
        let Some(pattern) = self.pattern.as_ref() else {
            // Java: 无模式时使用区域默认模式；Rust 侧回落为普通十进制表示
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

    /// 从 `text` 的位置 0 开始按本模式解析，返回「值 + 已消费的字符数」。
    ///
    /// 对应 Java: `DecimalFormat.parse(String, ParsePosition)` 的 `subparse` 部分；
    /// 返回 `None` 对应 Java 的「`ParsePosition.getIndex() == 0`」。
    fn parse_at(&self, text: &str) -> Option<(f64, usize)> {
        let pattern = self.pattern.as_ref()?;
        let chars: Vec<char> = text.chars().collect();
        let mut pos = 0;

        // Java: NumberFormat.parse 允许前导空白
        while pos < chars.len() && chars[pos].is_whitespace() {
            pos += 1;
        }

        // 正 / 负前缀；Java 在前缀不匹配时仍会继续尝试解析数字，这里保持一致
        let mut negative = false;
        if let Some(end) = match_text(&chars, pos, &pattern.negative_prefix) {
            pos = end;
            negative = true;
        } else if let Some(end) = match_text(&chars, pos, &pattern.positive_prefix) {
            pos = end;
        }

        // 无前缀时的正负号
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
                // Java: DecimalFormat.isGroupingUsed() 默认为 true，解析时始终接受分组分隔符
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

/// 把 `DecimalFormat` 模式串编译为 [`NumberPattern`]。
///
/// 对应 Java: `java.text.DecimalFormat.applyPattern(String)`（含 `Pattern` 校验）
///
/// 与 Java 的差异：
/// - 模式非法（没有数字占位符、引号未闭合等）时 Java 抛 `IllegalArgumentException`，
///   这里返回 `None`；
/// - 指数记法（`E`）与负子模式的位数差异按 Java 解析后忽略（MPX 用不到）。
fn compile_number_pattern(pattern: &str) -> Option<NumberPattern> {
    let sub_patterns = split_sub_patterns(pattern);
    let positive = split_sub_pattern(sub_patterns.first()?)?;

    // Java: 没有负子模式时，负数前缀为 "-" + 正数前缀，后缀同正数后缀
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

/// 按**未被引号包裹**的 `;` 把模式拆成正 / 负子模式。
///
/// 对应 Java: `DecimalFormat` 对 `;` 作为子模式分隔符的处理。
fn split_sub_patterns(pattern: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut chars = pattern.chars();

    while let Some(character) = chars.next() {
        match character {
            '\'' => {
                // 引号段整体保留，段内的 ';' 不是分隔符
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

/// 解析单个子模式：分离前缀 / 后缀 / 数字占位符。
///
/// 对应 Java: `DecimalFormat` 的子模式解析（`Pattern` 语法）。
fn split_sub_pattern(text: &str) -> Option<SubPattern> {
    // 逐字符展开：`(字符, 是否为模式语法字符)`，引号段内一律为字面量
    let mut items: Vec<(char, bool)> = Vec::new();
    let mut percent = false;
    let mut chars = text.chars();

    while let Some(character) = chars.next() {
        if character == '\'' {
            if chars.as_str().starts_with('\'') {
                // `''` → 一个字面单引号
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
            // 百分号 / 千分号：保留在前后缀中，同时记录倍增标记
            percent = true;
        }

        items.push((character, matches!(character, '#' | '0' | '.' | ',' | 'E')));
    }

    let first = items.iter().position(|(_, syntax)| *syntax)?;
    let last = items.iter().rposition(|(_, syntax)| *syntax)?;

    let core = &items[first..=last];
    if !core.iter().any(|(character, _)| *character == '#' || *character == '0') {
        // Java: 模式必须至少包含一个数字占位符
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
            // 引号包裹的字符即使落在数字核心区间内也只是字面量（MPX 的模式不会出现该情形）
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
            // 'E'：指数记法，本移植忽略（MPX 不使用）
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
        // Java: 分组长度取最后一个分组分隔符之后的数字位数，缺省为 3
        grouping_size: if digits_after_grouping == 0 {
            3
        } else {
            digits_after_grouping
        },
        percent,
    })
}

/// 从 `pos` 起匹配字面量文本，成功返回新的位置。
///
/// 对应 Java: `String.regionMatches` / `DecimalFormat.parse` 的前后缀匹配。
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

/// 截断小数部分末尾多余的 `0`，但至少保留 `min_digits` 位。
fn trim_fraction(fraction: &str, min_digits: usize) -> String {
    let mut end = fraction.len();
    while end > min_digits && fraction[..end].ends_with('0') {
        end -= 1;
    }
    fraction[..end].to_owned()
}

/// 按最少整数位补前导零。
///
/// 对应 Java 的 `#` 语义：最少整数位为 0 且存在小数部分时，整数部分的 `0` 也被省略
/// （例如模式 `#.##` 格式化 `0.5` 得到 `.5`）。
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

/// 按分组长度插入分组分隔符（仅整数部分）。
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

/// 无模式时的数值表示（Java 使用区域默认数字模式，这里用普通十进制表示）。
fn format_fallback(value: f64) -> String {
    format!("{value}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个应用了给定模式的数字格式。
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
        // Java: NumberFormat.parse 允许前导空白，且不要求消费完整个字符串
        assert_eq!(format.parse(Some("  12.5 kilograms")), Some(12.5));
        assert_eq!(format.parse(Some("-12.5")), Some(-12.5));
    }

    #[test]
    fn parse_accepts_grouping_separators() {
        // Java 的 DecimalFormat.isGroupingUsed() 默认为 true
        let format = format_with("#0.00", '.', ',');
        assert_eq!(format.parse(Some("1,234.50")), Some(1234.5));
    }

    #[test]
    fn parse_uses_alternative_patterns() {
        // 货币格式：主模式带符号，替代模式不带符号
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
        // 无符号 → 走替代模式
        assert_eq!(format.parse(Some("1,234.50")), Some(1234.5));
        // 括号表示负数 → 走替代模式
        assert_eq!(format.parse(Some("($1,234.50)")), Some(-1234.5));
        assert_eq!(format.parse(Some("(1,234.50)")), Some(-1234.5));
    }

    #[test]
    fn parse_uses_locale_separators() {
        // 德语等区域：小数分隔符 ','、分组分隔符 '.'
        let format = format_with("#.##", ',', '.');
        assert_eq!(format.parse(Some("1.234,5")), Some(1234.5));
        assert_eq!(format.parse(Some("0,5")), Some(0.5));
    }

    #[test]
    fn format_pads_and_trims_fraction_digits() {
        // 模式 "0.00#"：至少两位小数，最多三位
        let format = format_with("0.00#", '.', ',');
        assert_eq!(format.format(12.5), "12.50");
        assert_eq!(format.format(12.567), "12.567");
        assert_eq!(format.format(12.5678), "12.568");

        // 模式 "#.##"：无小数时省略小数点，"#" 不补零
        let format = format_with("#.##", '.', ',');
        assert_eq!(format.format(12.5), "12.5");
        assert_eq!(format.format(100.0), "100");
        assert_eq!(format.format(2.345), "2.35");
    }

    #[test]
    fn format_follows_locale_separators_and_grouping() {
        let format = format_with("#,##0.00", ',', '.');
        assert_eq!(format.format(1234.5), "1.234,50");

        // 主货币模式没有分组分隔符 → 不分组
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
                    .unwrap_or_else(|| panic!("模式 {pattern} 的输出 {text} 应可解析"));
                assert!(
                    (parsed - value).abs() < 0.005,
                    "模式 {pattern}：{value} → {text} → {parsed}"
                );
            }
        }
    }

    #[test]
    fn invalid_pattern_is_reported_as_missing_pattern() {
        let mut format = MPXJNumberFormat::new();
        format.apply_pattern("abc", None, '.', ',');
        // Java 抛 IllegalArgumentException；Rust 侧不 panic，解析失败、格式化回落
        assert_eq!(format.parse(Some("12")), None);
        assert_eq!(format.format(12.5), "12.5");
    }

    #[test]
    fn quoted_characters_are_literals() {
        // 对应 MPXJFormats.quoteFormatCharacters 的产出：货币符号 "." 会被引号包裹，
        // 避免被当作小数分隔符（quoteFormatCharacters(".") == "'.'"）
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
