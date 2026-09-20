//! 对应 Java: `org/mpxj/mpx/MPXJFormats.java`
//!
//! # 移植取舍（`DateTimeFormatter` / `DateTimeFormatterBuilder` / `NumberFormat`）
//!
//! Java 侧本类把 MPX 文件中的日期 / 时间格式配置编译成
//! `java.time.format.DateTimeFormatter`（借助 `DateTimeFormatterBuilder` 的
//! `appendPattern` / `appendValueReduced` / `appendText` / `optionalStart` 等），
//! 并用 `NumberFormat` 解析与格式化数值。Rust 的 chrono 没有等价的「模式串 → 解析器」
//! 设施（`StrftimeItems` 只覆盖另一套模式字母，且不支持可选段），因此本模块自建了一个
//! **轻量模式引擎**：
//!
//! - [`compile_pattern`] 把 Java 用到的模式串子集编译成 [`PatternNode`] 列表：字面量、
//!   数字字段（`d` / `M` / `y` / `h` / `H` / `m`）、文本字段（`MMM` / `MMMM` / `EEE`）、
//!   上午下午文本、周字段（`F` / `w`）以及可选段 `[...]`（支持嵌套）。
//! - 解析侧对应 Java 的 SMART / LENIENT 解析：数字字段允许 `1..N` 位（`dd` 同样接受 `5`，
//!   相当于 Java 非严格模式下的 `effMinWidth = 1`），可选段「整体匹配成功才前进，否则
//!   回退到段起点」，字面量按大小写不敏感匹配（对应 `parseCaseInsensitive`），
//!   解析结束后允许尾随字符（Java 同样不要求消费完整个字符串）。
//! - 打印侧对应 `format`：`dd` / `MM` 等按模式位数补前导零，`yy` 打印两位年份，
//!   `MMM` / `MMMM` / `EEE` 打印文本，`F` / `w` 打印月内第几个星期几 / 一年中的第几周。
//! - 解析结果的字段还原：需要「年 + 月 + 日」才能得到日期（Java 缺少其中任一字段时同样
//!   无法还原 `LocalDate`，会抛 `DateTimeParseException` → Rust 侧返回 `None`）。
//!
//! ## `YYYY` 等易错语义
//!
//! - 本类模式串中的 `yyyy` 是**日历年**（chrono 的 `Date::year()`），`yy` 是两位年份：
//!   打印取 `year % 100`；解析按 Java 的 `appendValueReduced(YEAR, 2, 2, baseDate)` 归约，
//!   基期取 [`LocalDateHelper::two_digit_year_base_date`]（`now - 80 年`），结果落在
//!   `[baseYear, baseYear + 99]` 窗口内（因此两位 `98` 在 2026 年解析为 1998）。
//!   **不存在** Java 模式中「周年」`YYYY`（`WeekFields`）的语义。
//! - `ww` 对应 Java 的 `w`（`WeekFields.ISO` 的 year-of-week-based-year 周序号），
//!   Rust 侧用 chrono 的 `NaiveDate::iso_week().week()`。
//! - 12 / 24 小时制：`hh` 为 1-12 的时钟小时（解析时配合 AM / PM 文本还原 24 小时值，
//!   无 AM / PM 时按上午处理，与 Java 对 `CLOCK_HOUR_OF_AMPM` 的默认处理一致）。
//!
//! ## 本地化月份 / 星期名
//!
//! Java 通过 `DateTimeFormatter.withLocale(m_locale)` 从 JDK `java.time` 取本地化的月份 /
//! 星期名。Rust 侧以 [`localized_names`] 自建 9 个受支持语种的表（数据取自 JDK 25 在对应
//! `Locale` 下 `MMM` / `MMMM` / `EEE` / `EEEE` 的输出），由 [`names_for_locale`] 按
//! [`MpxLocale`] 分派，结果与 Java 一致。注意 `crate::mpx::locale_data` 并不含这些名字
//! （Java 的 `org.mpxj.mpx.LocaleData` 里也没有），因此数据只能自建。
//! 上午 / 下午文本与 Java 一样取自项目属性的 `getAmText` / `getPmText`（Java 侧编入
//! `appendText` 的映射表），英文 `am` / `pm` 仅作兜底。
//!
//! ## 降级与未实现（MPX 用不到）
//!
//! - Java LENIENT 模式下的字段溢出归一化（如 `31/02` 被归一化）不实现：非法日期
//!   直接返回 `None`。
//! - `F` / `w` 只参与解析 / 打印，不参与日期还原（仅凭这两个字段 Java 也无法还原日期）。
//!
//! ## 其他结构性取舍
//!
//! - `getPrintDateBuilder` / `getParseDateBuilder` 在 Java 中返回 `DateTimeFormatterBuilder`，
//!   这里返回编译好的 [`PatternNode`] 列表；`[yyyy]` 与紧随其后的
//!   `appendValueReduced(YEAR, 2, 2, base)` 两段被合并为一个 `Year` 字段（解析时 1-2 位按
//!   窗口归约、3-4 位按日历年），以复现 Java 两段式解析的意图。
//! - `Locale` → [`MpxLocale`]；`ProjectFile` → `Weak<RefCell<ProjectFile>>`（本 crate 约定）；
//!   `NumberFormat` → [`MPXJNumberFormat`]。
//! - 空值：Java 在格式化器为 `null` 时抛 NPE，Rust 侧返回 `None`；`ParseException` /
//!   `DateTimeParseException` 同样返回 `None`（不 panic）。三个 `parse*` 方法按 Java 的
//!   `String` 入参取 `&str`（`null` 交由调用方用 `Option` 处理），`print*` 方法按 Java 的
//!   null 检查取 `Option`。
//! - 照搬的上游行为：`DD_MMM_HH_MM` 分支中 Java 调用 `createPrintFormatterWithTime` 但
//!   **丢弃返回值**，因此打印格式化器不会被更新（疑似上游 bug），这里照搬并注明。
//!
//! 未移植的成员：无。

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
// 模式引擎：编译
// ---------------------------------------------------------------------------

/// 模式串中的数字字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NumberField {
    /// 月内日（`d` / `dd`）
    Day,
    /// 月（`M` / `MM`）
    Month,
    /// 日历年（`yyyy`，以及 `[yyyy]` + `appendValueReduced` 的组合）
    Year,
    /// 两位年份（`yy`，对应 Java 的 `appendValueReduced(YEAR, 2, 2, base)`）
    ReducedYear,
    /// 24 小时制小时（`H` / `HH`）
    Hour24,
    /// 12 小时制小时（`h` / `hh`）
    Hour12,
    /// 分钟（`m` / `mm`）
    Minute,
}

/// 编译后的模式串元素。
#[derive(Debug, Clone, PartialEq, Eq)]
enum PatternNode {
    /// 字面量文本（解析时大小写不敏感）
    Literal(String),
    /// 数字字段
    Number {
        /// 字段
        field: NumberField,
        /// 打印时的最小位数
        min_width: usize,
    },
    /// 月份名（`MMM` 缩写 / `MMMM` 全称）
    MonthText {
        /// 是否使用全称
        full: bool,
    },
    /// 星期名（`EEE`）
    DayName,
    /// 上午 / 下午文本（对应 Java `appendText(ChronoField.AMPM_OF_DAY, ...)`）
    AmPmText,
    /// 一年中的第几周（`w`，ISO 周）
    WeekOfYear {
        /// 打印时的最小位数
        min_width: usize,
    },
    /// 月内第几个星期几（`F`）
    DayOfWeekInMonth {
        /// 打印时的最小位数
        min_width: usize,
    },
    /// 可选段 `[...]`
    Optional(Vec<PatternNode>),
}

/// 把模式串编译为节点列表。
///
/// 对应 Java: `DateTimeFormatterBuilder.appendPattern(String)`
///
/// Java 对无法识别的模式字母抛 `IllegalArgumentException`；Rust 侧不 panic，未知字母按
/// 字面量处理，未闭合的 `[` / `'` 视为直到串尾。
fn compile_pattern(pattern: &str) -> Vec<PatternNode> {
    let chars: Vec<char> = pattern.chars().collect();
    compile_pattern_at(&chars, 0, false).0
}

/// 从 `start` 开始编译模式串，返回（节点列表, 结束位置）。
fn compile_pattern_at(chars: &[char], start: usize, in_optional: bool) -> (Vec<PatternNode>, usize) {
    let mut nodes = Vec::new();
    let mut literal = String::new();
    let mut index = start;

    while index < chars.len() {
        let current = chars[index];
        match current {
            '\'' => {
                // `''` 表示一个字面单引号，`'...'` 之间的内容原样作为字面量
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

/// 把待处理的字面量推入节点列表。
fn push_literal(nodes: &mut Vec<PatternNode>, literal: &mut String) {
    if !literal.is_empty() {
        nodes.push(PatternNode::Literal(std::mem::take(literal)));
    }
}

/// 把「模式字母 + 重复次数」转换为节点。
///
/// 对应 Java: `DateTimeFormatterBuilder.appendPattern` 中单个模式字母的处理。
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
        // Java: `yy` 由 appendValueReduced 处理（两位年份归约）
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
// 模式引擎：解析
// ---------------------------------------------------------------------------

/// 解析过程中累积的字段值（对应 Java 解析时写入 `DateTimeParseContext` 的字段）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ParsedFields {
    /// 日历年
    year: Option<i32>,
    /// 月
    month: Option<u32>,
    /// 月内日
    day: Option<u32>,
    /// 24 小时制小时
    hour24: Option<u32>,
    /// 12 小时制小时
    hour12: Option<u32>,
    /// 分钟
    minute: Option<u32>,
    /// 上午 / 下午（0 = AM，1 = PM）
    ampm: Option<u32>,
}

/// 逐个节点解析文本，返回结束位置；任一节点失败返回 `None`。
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
                // 可选段：整体匹配成功则前进，否则回退到段起点（与 Java 的可选段语义一致）
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

/// 解析数字字段。
///
/// 对应 Java: `DateTimeFormatterBuilder.appendValue*` 的非严格解析（`effMinWidth = 1`）。
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

/// 读取（并校验范围）一个不带字段语义的数字，用于 `F` / `w` 等只参与匹配的字段。
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

/// 读取最多 `max_digits` 位十进制数字，返回（结束位置, 数值）。
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

/// 解析月份名。
///
/// 对应 Java: `DateTimeFormatter` 对 `MMM` / `MMMM` 的文本解析（大小写不敏感）。
fn parse_month_name(
    chars: &[char],
    pos: usize,
    full: bool,
    fields: &mut ParsedFields,
    context: &NameContext<'_>,
) -> Option<usize> {
    // 全称优先，缩写次之（`May` 等两者相同的月份不受影响）
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

/// 解析星期名（解析结果不参与日期还原，与 Java 丢弃星期名一致）。
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

/// 解析上午 / 下午文本。
///
/// 对应 Java: `DateTimeFormatterBuilder.appendText(ChronoField.AMPM_OF_DAY, ampmMap)`，
/// 映射表来自项目属性的 AM / PM 文本；此处额外接受英文 `am` / `pm`。
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

    // PM 在前：避免 "pm" 的前缀与自定义文本互相干扰
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

/// 从 `pos` 起按大小写不敏感匹配文本，成功返回新的位置。
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

/// 复现 Java `appendValueReduced(YEAR, 2, 2, baseDate)` 的两位年份归约。
///
/// Java 的基期是 `LocalDate.now().minusYears(80)`，结果落在 `[baseYear, baseYear + 99]`。
fn reduce_two_digit_year(value: i32) -> i32 {
    let base_year = LocalDateHelper::two_digit_year_base_date().year();
    let range = 100;
    let last_part = base_year.rem_euclid(range);
    let value = if value < last_part { value + range } else { value };
    value - last_part + base_year
}

/// 由解析出的字段还原日期。
fn resolve_date(fields: &ParsedFields) -> Option<NaiveDate> {
    let year = fields.year?;
    let month = fields.month?;
    let day = fields.day?;
    NaiveDate::from_ymd_opt(year, month, day)
}

/// 由解析出的字段还原时间（缺少小时则返回 `None`）。
fn resolve_time(fields: &ParsedFields) -> Option<NaiveTime> {
    let minute = fields.minute.unwrap_or(0);
    let hour = match fields.hour24 {
        Some(hour) => hour,
        None => {
            let hour12 = fields.hour12?;
            // Java：缺少 AMPM 字段时按上午处理，12 点归零
            let afternoon = fields.ampm == Some(1);
            (hour12 % 12) + if afternoon { 12 } else { 0 }
        }
    };

    NaiveTime::from_hms_opt(hour, minute, 0)
}

/// 由解析出的字段还原日期时间（缺时间部分时取当日零点，对应 Java 的 `parseBest` 回落）。
fn resolve_date_time(fields: &ParsedFields) -> Option<NaiveDateTime> {
    let date = resolve_date(fields)?;
    let time = resolve_time(fields).unwrap_or(NaiveTime::MIN);
    Some(NaiveDateTime::new(date, time))
}

// ---------------------------------------------------------------------------
// 模式引擎：打印
// ---------------------------------------------------------------------------

/// 按节点列表格式化日期时间。
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
            // Java 打印时可选段的内容照常输出
            PatternNode::Optional(inner) => {
                result.push_str(&format_date_time(inner, value, context));
            }
        }
    }

    result
}

/// 格式化数字字段。
fn format_number(field: NumberField, min_width: usize, value: &NaiveDateTime) -> String {
    match field {
        NumberField::Day => pad(value.day(), min_width),
        NumberField::Month => pad(value.month(), min_width),
        NumberField::Year => pad(value.year().max(0) as u32, min_width),
        // Java: `yy` 打印年份的后两位
        NumberField::ReducedYear => pad((value.year().rem_euclid(100)) as u32, 2),
        NumberField::Hour24 => pad(value.hour(), min_width),
        NumberField::Hour12 => pad(clock_hour(value.hour()), min_width),
        NumberField::Minute => pad(value.minute(), min_width),
    }
}

/// 左侧补零到 `width` 位。
fn pad(value: u32, width: usize) -> String {
    format!("{value:0>width$}")
}

/// 12 小时制的时钟小时（0 点 / 12 点均为 12）。
fn clock_hour(hour: u32) -> u32 {
    match hour % 12 {
        0 => 12,
        value => value,
    }
}

/// 是否为下午（对应 Java `ChronoField.AMPM_OF_DAY`）。
fn is_afternoon(hour: u32) -> bool {
    hour >= 12
}

/// 月内第几个星期几（对应 Java 模式字母 `F`）。
fn day_of_week_in_month(day: u32) -> u32 {
    ((day - 1) / 7) + 1
}

/// 月份名文本。
fn month_name(tables: &TextTable, month: u32, full: bool) -> &'static str {
    let index = month.saturating_sub(1) as usize;
    let table = if full {
        &tables.months_full
    } else {
        &tables.months_abbreviated
    };
    table.get(index).copied().unwrap_or("")
}

/// 星期名文本。
fn weekday_name(tables: &TextTable, weekday: chrono::Weekday) -> &'static str {
    let index = weekday.num_days_from_monday() as usize;
    tables
        .weekdays_abbreviated
        .get(index)
        .copied()
        .unwrap_or("")
}

/// 取区域设置对应的文本常量表。
///
/// 对应 Java: `DateTimeFormatter.withLocale(m_locale)`（月份 / 星期名文本）。
/// 数据由 [`crate::mpx::localized_names`] 按语种提供。
fn names_for_locale(locale: MpxLocale) -> &'static TextTable {
    crate::mpx::localized_names::names_for_locale(locale)
}

/// 文本字段（上午 / 下午）的查找表。
///
/// 对应 Java 在构造格式化器时捕获的 `appendText(AMPM_OF_DAY, ampmMap)` 映射与
/// `withLocale(m_locale)` 的文本来源。
#[derive(Debug, Clone, Copy)]
struct NameContext<'a> {
    /// 上午文本（来自 `ProjectProperties.getAmText()`）
    am: &'a str,
    /// 下午文本（来自 `ProjectProperties.getPmText()`）
    pm: &'a str,
    /// 月份名 / 星期名表（见 [`names_for_locale`]）
    tables: &'static TextTable,
}

// ---------------------------------------------------------------------------
// MPXJFormats
// ---------------------------------------------------------------------------

/// MPX 文件解析 / 格式化所需的各类格式对象。
///
/// 对应 Java: `public final class MPXJFormats`
pub struct MPXJFormats {
    /// 目标区域设置（对应 Java `m_locale`）
    locale: MpxLocale,
    /// 表示「未设置」的文本，通常为 `NA`（对应 Java `m_nullText`）
    null_text: String,
    /// 父文件（对应 Java `m_projectFile`；Rust 侧按本 crate 约定使用弱引用）
    project_file: Weak<RefCell<ProjectFile>>,
    /// 单位小数格式（对应 Java `m_unitsDecimalFormat`）
    units_decimal_format: MPXJNumberFormat,
    /// 小数格式（对应 Java `m_decimalFormat`）
    decimal_format: MPXJNumberFormat,
    /// 货币格式（对应 Java `m_currencyFormat`）
    currency_format: MPXJNumberFormat,
    /// 工期小数格式（对应 Java `m_durationDecimalFormat`）
    duration_decimal_format: MPXJNumberFormat,
    /// 百分比小数格式（对应 Java `m_percentageDecimalFormat`）
    percentage_decimal_format: MPXJNumberFormat,
    /// 解析时间戳时是否先丢弃开头的星期名（对应 Java `m_parseDateTimeSkipDayName`）
    parse_date_time_skip_day_name: bool,
    /// 时间戳解析格式（对应 Java `m_parseDateTimeFormat`）
    parse_date_time_format: Vec<PatternNode>,
    /// 时间戳打印格式（对应 Java `m_printDateTimeFormat`）
    print_date_time_format: Vec<PatternNode>,
    /// 日期解析格式（对应 Java `m_parseDateFormat`）
    parse_date_format: Vec<PatternNode>,
    /// 日期打印格式（对应 Java `m_printDateFormat`）
    print_date_format: Vec<PatternNode>,
    /// 时间解析格式（对应 Java `m_parseTimeFormat`）
    parse_time_format: Vec<PatternNode>,
    /// 时间打印格式（对应 Java `m_printTimeFormat`）
    print_time_format: Vec<PatternNode>,
    /// 上午文本（对应 Java 编入格式化器的 AM 映射项，这里在 `update` 时缓存）
    am_text: String,
    /// 下午文本（对应 Java 编入格式化器的 PM 映射项，这里在 `update` 时缓存）
    pm_text: String,
}

impl MPXJFormats {
    /// 构造格式对象并立即初始化各格式。
    ///
    /// 对应 Java: `public MPXJFormats(Locale locale, String nullText, ProjectFile file)`
    ///
    /// 取舍：Java 直接持有 `ProjectFile`；Rust 侧按本 crate 约定接收
    /// `Weak<RefCell<ProjectFile>>`（调用方需自行持有文件句柄）。
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

    /// 在配置变化后更新缓存的格式。
    ///
    /// 对应 Java: `public void update()`
    ///
    /// 取舍：Java 直接读取 `m_projectFile.getProjectProperties()`（父文件缺失时抛 NPE）；
    /// Rust 侧在父文件已释放或被可变借用时**跳过更新**（不 panic，保留上一次的格式，
    /// 首次调用时则保持未初始化状态）。
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

        // Java 在 applyTime*Pattern 中把 AM / PM 文本编入格式化器，这里随配置一起缓存
        self.am_text = properties.get_am_text();
        self.pm_text = properties.get_pm_text();

        self.update_numeric_formats(decimal_separator, thousands_separator);
        self.update_currency_formats(properties, decimal_separator, thousands_separator);
        self.update_date_time_formats(properties);
    }

    /// 更新数值格式。
    ///
    /// 对应 Java: `private void updateNumericFormats(char decimalSeparator, char thousandsSeparator)`
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

    /// 更新货币格式。
    ///
    /// 对应 Java:
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

    /// 更新日期与时间格式。
    ///
    /// 对应 Java: `private void updateDateTimeFormats(ProjectProperties properties)`
    fn update_date_time_formats(&mut self, properties: &ProjectProperties) {
        self.populate_time_patterns(properties);
        self.populate_date_patterns(properties);
        self.populate_date_time_patterns(properties);
    }

    /// 构造「日期 + 时间」的打印格式。
    ///
    /// 对应 Java:
    /// `private DateTimeFormatter createPrintFormatterWithTime(ProjectProperties properties, String pattern)`
    ///
    /// 取舍：Java 返回 `DateTimeFormatter`；这里返回编译后的节点列表（见文件头）。
    fn create_print_formatter_with_time(
        properties: &ProjectProperties,
        pattern: &str,
    ) -> Vec<PatternNode> {
        let mut nodes = compile_pattern(pattern);
        nodes.push(PatternNode::Literal(" ".to_owned()));
        Self::apply_time_print_pattern(properties, &mut nodes);
        nodes
    }

    /// 构造「纯日期」的打印格式。
    ///
    /// 对应 Java: `private void createPrintFormatter(String pattern)`
    fn create_print_formatter(&mut self, pattern: &str) {
        self.print_date_time_format = compile_pattern(pattern);
    }

    /// 生成时间部分的解析模式。
    ///
    /// 对应 Java:
    /// `private void applyTimeParsePattern(ProjectProperties properties, DateTimeFormatterBuilder builder)`
    ///
    /// 12 小时制对应 Java 的 `optionalStart(); appendPattern("[hh][h]" + timesep + "mm[ ]");
    /// appendText(AMPM_OF_DAY, map); optionalEnd(); appendPattern("[[HH][H]" + timesep + "mm]")`；
    /// 24 小时制对应 `appendPattern("[HH][H][" + timesep + "mm]")`。
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

            // "[[HH][H]" + timesep + "mm]"：外层可选段内再含两个可选小时字段
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

    /// 生成时间部分的打印模式。
    ///
    /// 对应 Java:
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

    /// 生成日期解析格式。
    ///
    /// 对应 Java: `private DateTimeFormatterBuilder getParseDateBuilder(ProjectProperties properties)`
    ///
    /// 取舍：Java 先 `[yyyy]` 再追加可选的 `appendValueReduced(YEAR, 2, 2, base)`，
    /// 这里合并成单个 `Year` 字段（1-2 位按两位年份归约、3-4 位按日历年，见文件头）。
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

    /// 生成日期打印格式。
    ///
    /// 对应 Java: `private DateTimeFormatterBuilder getPrintDateBuilder(ProjectProperties properties)`
    fn get_print_date_builder(properties: &ProjectProperties) -> Vec<PatternNode> {
        let datesep = properties.get_date_separator();

        match properties.get_date_order() {
            DateOrder::Dmy => compile_pattern(&format!("dd{datesep}MM{datesep}yy")),
            DateOrder::Mdy => compile_pattern(&format!("MM{datesep}dd{datesep}yy")),
            DateOrder::Ymd => compile_pattern(&format!("yy{datesep}MM{datesep}dd")),
        }
    }

    /// 根据项目配置生成时间戳格式。
    ///
    /// 对应 Java: `private void populateDateTimePatterns(ProjectProperties properties)`
    ///
    /// 说明：Java 在 `DD_MMM_HH_MM` 分支丢弃了 `createPrintFormatterWithTime` 的返回值，
    /// 打印格式化器不会被更新（疑似上游 bug），此处照搬。
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
                    // Java: `createPrintFormatterWithTime(properties, "dd MMM");` 返回值被丢弃
                    let _ = Self::create_print_formatter_with_time(properties, "dd MMM");
                }
                DateOrder::Ymd | DateOrder::Mdy => {
                    self.create_parse_formatter_with_time(properties, &["MMM dd"]);
                    // Java: `createPrintFormatterWithTime(properties, "MMM dd");` 返回值被丢弃
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

    /// 构造时间戳解析格式。
    ///
    /// 对应 Java:
    /// `private void createParseFormatterWithTime(ProjectProperties properties, String... patterns)`
    ///
    /// Java 的 `String...` 变长参数在 Rust 侧用切片表达；模式中等于 `yy` 的片段对应
    /// Java 的 `appendValueReduced(ChronoField.YEAR, 2, 2, base)`，其余片段按模式串编译；
    /// 最后追加「空格 + 时间解析模式」的可选段。
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

        // Java: 模式以 "[EEE ]" 开头时需要先丢弃文本中的星期名
        if patterns.first().is_some_and(|pattern| pattern.starts_with("[EEE ]")) {
            self.parse_date_time_skip_day_name = true;
        }
    }

    /// 生成时间格式。
    ///
    /// 对应 Java: `private void populateTimePatterns(ProjectProperties properties)`
    fn populate_time_patterns(&mut self, properties: &ProjectProperties) {
        let mut parse_nodes = Vec::new();
        Self::apply_time_parse_pattern(properties, &mut parse_nodes);
        self.parse_time_format = parse_nodes;

        let mut print_nodes = Vec::new();
        Self::apply_time_print_pattern(properties, &mut print_nodes);
        self.print_time_format = print_nodes;
    }

    /// 生成日期格式。
    ///
    /// 对应 Java: `private void populateDatePatterns(ProjectProperties properties)`
    fn populate_date_patterns(&mut self, properties: &ProjectProperties) {
        self.parse_date_format = Self::get_parse_date_builder(properties);
        self.print_date_format = Self::get_print_date_builder(properties);
    }

    /// 取单位小数格式。
    ///
    /// 对应 Java: `public NumberFormat getUnitsDecimalFormat()`
    pub fn get_units_decimal_format(&self) -> &MPXJNumberFormat {
        &self.units_decimal_format
    }

    /// 取小数格式。
    ///
    /// 对应 Java: `public NumberFormat getDecimalFormat()`
    pub fn get_decimal_format(&self) -> &MPXJNumberFormat {
        &self.decimal_format
    }

    /// 取货币格式。
    ///
    /// 对应 Java: `public NumberFormat getCurrencyFormat()`
    pub fn get_currency_format(&self) -> &MPXJNumberFormat {
        &self.currency_format
    }

    /// 取工期小数格式。
    ///
    /// 对应 Java: `public NumberFormat getDurationDecimalFormat()`
    pub fn get_duration_decimal_format(&self) -> &MPXJNumberFormat {
        &self.duration_decimal_format
    }

    /// 取百分比小数格式。
    ///
    /// 对应 Java: `public NumberFormat getPercentageDecimalFormat()`
    pub fn get_percentage_decimal_format(&self) -> &MPXJNumberFormat {
        &self.percentage_decimal_format
    }

    /// 按配置的格式解析时间。
    ///
    /// 对应 Java: `public LocalTime parseTime(String value)`
    ///
    /// 解析失败返回 `None`（Java 抛 `DateTimeParseException`）。
    pub fn parse_time(&self, value: &str) -> Option<NaiveTime> {
        let fields = self.parse_fields(&self.parse_time_format, value)?;
        resolve_time(&fields)
    }

    /// 按配置的格式打印时间。
    ///
    /// 对应 Java: `public String printTime(LocalTime value)`
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

    /// 按配置的格式解析日期。
    ///
    /// 对应 Java: `public LocalDateTime parseDate(String value)`
    ///
    /// 文本等于 `getNullText()`（通常为 `NA`）或解析失败都返回 `None`；
    /// 解析结果取当日零点（对应 Java 的 `atStartOfDay()`）。
    pub fn parse_date(&self, value: &str) -> Option<NaiveDateTime> {
        if value == self.null_text {
            return None;
        }

        let fields = self.parse_fields(&self.parse_date_format, value)?;
        let date = resolve_date(&fields)?;
        Some(NaiveDateTime::new(date, NaiveTime::MIN))
    }

    /// 按配置的格式打印日期。
    ///
    /// 对应 Java: `public String printDate(LocalDate value)`
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

    /// 按配置的格式解析时间戳。
    ///
    /// 对应 Java: `public LocalDateTime parseDateTime(String value)`
    ///
    /// 与 Java 一致：文本等于 `getNullText()` 时返回 `None`；解析格式带星期的
    /// （`[EEE ]` 开头）会先丢弃第一个空格之前的内容；缺少年 / 月 / 日时无法还原
    /// 日期（Java 抛 `DateTimeParseException`），返回 `None`。
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

    /// 按配置的格式打印时间戳。
    ///
    /// 对应 Java: `public String printDateTime(TemporalAccessor value)`（`LocalDateTime` 分支）
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

    /// 按配置的格式打印日期（对应 Java 中把 `LocalDate` 传给 `printDateTime` 的分支）。
    ///
    /// 对应 Java: `public String printDateTime(TemporalAccessor value)`（`LocalDate` 分支）
    ///
    /// 取舍：Java 对含时间字段的模式传入 `LocalDate` 会抛
    /// `UnsupportedTemporalTypeException`；这里按当日零点处理。
    pub fn print_date_time_for_date(&self, value: Option<&NaiveDate>) -> Option<String> {
        let value = value?;
        self.print_date_time(Some(&NaiveDateTime::new(*value, NaiveTime::MIN)))
    }

    /// 取表示「未设置」的文本。
    ///
    /// 对应 Java: `public String getNullText()`
    pub fn get_null_text(&self) -> &str {
        &self.null_text
    }

    /// 解析文本为字段集合。
    ///
    /// 对应 Java: `DateTimeFormatter.parse(CharSequence)`
    fn parse_fields(&self, nodes: &[PatternNode], value: &str) -> Option<ParsedFields> {
        if nodes.is_empty() {
            // Java: 格式化器为 null（未初始化）时会抛 NPE
            return None;
        }

        let chars: Vec<char> = value.chars().collect();
        let mut fields = ParsedFields::default();
        parse_nodes(nodes, &chars, 0, &mut fields, &self.name_context())?;
        Some(fields)
    }

    /// 构造文本字段查找表。
    fn name_context(&self) -> NameContext<'_> {
        NameContext {
            am: &self.am_text,
            pm: &self.pm_text,
            tables: names_for_locale(self.locale),
        }
    }
}

/// 生成「可选的小时字段」节点，对应 Java 模式中的 `[hh]` / `[H]` 之类。
fn optional_hour(field: NumberField, min_width: usize) -> PatternNode {
    PatternNode::Optional(vec![PatternNode::Number { field, min_width }])
}

/// 转义货币符号中的特殊字符。
///
/// 对应 Java: `private String quoteFormatCharacters(String literal)`
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

    /// 构造格式对象。调用方必须持有返回的文件句柄（`MPXJFormats` 内部是弱引用）。
    fn formats(config: impl FnOnce(&mut ProjectProperties)) -> (Rc<RefCell<ProjectFile>>, MPXJFormats) {
        let file = ProjectFile::new();
        {
            let mut borrowed = file.borrow_mut();
            config(borrowed.get_project_properties_mut());
        }

        let formats = MPXJFormats::new(MpxLocale::En, "NA", Rc::downgrade(&file));
        (file, formats)
    }

    /// 日期 1998-08-25 14:35:00。
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

        // 12 小时制打印补前导零 + AM/PM 文本
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

        // 解析：带 AM/PM、缺前导零、全无前导零、以及 24 小时写法都能接受
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
                "解析 {text} 失败或结果不符"
            );
        }

        // 往返：打印 → 解析
        for time in [
            NaiveTime::from_hms_opt(0, 0, 0),
            NaiveTime::from_hms_opt(9, 5, 0),
            NaiveTime::from_hms_opt(12, 0, 0),
            NaiveTime::from_hms_opt(23, 59, 0),
        ] {
            let printed = formats.print_time(time).expect("应可打印");
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

        // 解析时小时 / 分钟都可缺省，且允许无前导零
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
            let printed = formats.print_time(time).expect("应可打印");
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
        // Java 同样无法解析没有时间的文本
        assert_eq!(formats.parse_time("25/08/98"), None);
    }

    #[test]
    fn date_formats_follow_date_order() {
        // 日月年
        let (_file, dmy) = formats(|properties| {
            properties.set_date_order(Some(DateOrder::Dmy));
        });
        // 月日年
        let (_file2, mdy) = formats(|properties| {
            properties.set_date_order(Some(DateOrder::Mdy));
        });
        // 年月日
        let (_file3, ymd) = formats(|properties| {
            properties.set_date_order(Some(DateOrder::Ymd));
        });

        let expected = NaiveDate::from_ymd_opt(1998, 3, 4)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        // 同一条文本在 DMY / MDY 下含义不同（3 月 4 日 vs 4 月 3 日）
        assert_eq!(dmy.parse_date("04/03/1998"), Some(expected));
        assert_eq!(
            mdy.parse_date("04/03/1998"),
            NaiveDate::from_ymd_opt(1998, 4, 3)
                .unwrap()
                .and_hms_opt(0, 0, 0)
        );
        assert_eq!(ymd.parse_date("1998/03/04"), Some(expected));

        // 打印格式与日期顺序一致
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
            // 两位年份按 window 归约（基期为当前年份 - 80）
            ("25/08/98", (1998, 8, 25)),
            ("1/1/99", (1999, 1, 1)),
        ];

        for (text, (year, month, day)) in cases {
            assert_eq!(
                formats.parse_date(text),
                NaiveDate::from_ymd_opt(year, month, day)
                    .unwrap()
                    .and_hms_opt(0, 0, 0),
                "解析 {text} 失败或结果不符"
            );
        }
    }

    #[test]
    fn date_formats_round_trip_across_boundaries() {
        let (_file, formats) = formats(|properties| {
            properties.set_date_order(Some(DateOrder::Dmy));
        });

        // 跨月 / 跨年边界
        for (year, month, day) in [
            (1998, 12, 31),
            (1999, 1, 1),
            (2000, 2, 29),
            (2026, 3, 1),
            (1996, 12, 1),
        ] {
            let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
            let printed = formats.print_date(Some(date)).expect("应可打印");
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
        // 空值文本
        assert_eq!(formats.parse_date("NA"), None);
        // 月 / 日越界
        assert_eq!(formats.parse_date("25/13/1998"), None);
        assert_eq!(formats.parse_date("32/12/1998"), None);
        // 日期本身非法（Java LENIENT 会归一化，这里判为非法）
        assert_eq!(formats.parse_date("31/02/1998"), None);
        // 无法解析的文本
        assert_eq!(formats.parse_date("hello"), None);
    }

    #[test]
    fn date_time_formats_round_trip_or_report_unresolvable() {
        // (格式, 是否可打印, 是否可解析, 是否含时间部分)
        //
        // 「不可解析」与 Java 一致：解析格式化器凑不齐年 / 月 / 日（Java 抛
        // DateTimeParseException），或打印格式化器因上游丢弃返回值而未初始化。
        let cases: [(ProjectDateFormat, bool, bool, bool); 21] = [
            (ProjectDateFormat::DdMmYyHhMm, true, true, true),
            (ProjectDateFormat::DdMmYy, true, true, false),
            (ProjectDateFormat::DdMmmmmYyyyHhMm, true, true, true),
            (ProjectDateFormat::DdMmmmmYyyy, true, true, false),
            // 上游丢弃了 createPrintFormatterWithTime 的返回值
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
                assert_eq!(printed, None, "{format:?} 不应有打印结果");
                continue;
            }

            let printed = printed.unwrap_or_else(|| panic!("{format:?} 应可打印"));
            let parsed = formats.parse_date_time(&printed);
            if !parseable {
                assert_eq!(parsed, None, "{format:?} 打印 {printed} 后不应能解析回日期");
                continue;
            }

            let parsed = parsed.unwrap_or_else(|| panic!("{format:?} 的 {printed} 应可解析"));
            if has_time {
                assert_eq!(parsed, value, "{format:?} 往返不一致（打印 {printed}）");
            } else {
                assert_eq!(parsed.date(), value.date(), "{format:?}（打印 {printed}）");
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

        // 1998-08-25 是星期二
        assert_eq!(
            formats.print_date_time(Some(&sample())),
            Some("Tue 25 Aug '98".to_owned())
        );
        // 解析时会先丢弃开头的星期名
        assert_eq!(
            formats.parse_date_time("Tue 25 Aug '98"),
            Some(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(1998, 8, 25).unwrap(),
                NaiveTime::MIN
            ))
        );
    }

    /// 构造指定区域设置的格式对象。
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

    /// 打印使用各语种自己的月份 / 星期名（回归 A19：此前所有语种都回落英文）。
    ///
    /// 期望值与 JDK 在对应 `Locale` 下的输出一致（见 `localized_names`）。
    #[test]
    fn date_time_printing_uses_locale_specific_names() {
        // 1998-08-25 是星期二
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
                "{locale} 的打印结果不符"
            );
        }
    }

    /// 德语默认日期格式 `EEE_DD_MM_YY` 打印本地化星期名（对齐 `sample.de.mpx`）。
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
        // 解析会先丢弃开头的星期名，故带本地化星期名的文本同样可解析
        assert_eq!(
            formats.parse_date_time("Di. 25.08.98"),
            Some(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(1998, 8, 25).unwrap(),
                NaiveTime::MIN
            ))
        );
    }

    /// 俄语月份名解析（非 ASCII 文本，验证本地化名表在解析路径上可用）。
    #[test]
    fn localized_month_names_are_parsed() {
        let (_file, formats) = formats_for_locale(MpxLocale::Ru, |properties| {
            properties.set_date_order(Some(DateOrder::Dmy));
            properties.set_date_format(Some(ProjectDateFormat::EeeDdMmmYy));
        });

        // 「25 авг. '98」：星期名先被丢弃，月份名「авг.」需能识别
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

        // 工期 / 单位 / 小数格式
        assert_eq!(
            formats.get_duration_decimal_format().parse(Some("2,5")),
            Some(2.5)
        );
        assert_eq!(formats.get_decimal_format().format(2.5), "2,50");
        assert_eq!(
            formats.get_decimal_format().parse(Some("2,50")),
            Some(2.5)
        );

        // 货币格式：符号在后且带空格
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

            // 打印时主模式不含分组分隔符
            let text = formats.get_currency_format().format(1234.5);
            assert_eq!(text, expected.replace(",", ""), "{position:?}");

            // 解析时分组分隔符可有可无
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
        // 父文件缺失时（Java 会 NPE）不 panic，格式保持未初始化
        let formats = MPXJFormats::new(MpxLocale::En, "NA", Weak::new());
        assert_eq!(formats.print_date(Some(NaiveDate::MIN)), None);
        assert_eq!(formats.parse_date("25/08/1998"), None);
        assert_eq!(formats.get_null_text(), "NA");
    }

    #[test]
    fn number_and_date_formats_keep_english_defaults() {
        let (_file, formats) = formats(|_properties| {});

        // 默认属性：小数分隔符 '.'、千分位 ','、货币符号 '$'、货币位数 2
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
