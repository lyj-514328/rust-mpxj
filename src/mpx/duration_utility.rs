//! 对应 Java: `org/mpxj/mpx/DurationUtility.java`
//!
//! # 移植取舍（`NumberFormat`）
//!
//! Java 用 `java.text.NumberFormat`（MPX 侧为 `MPXJNumberFormat`，按区域设置解析千分位、
//! 小数分隔符等）解析工期中的数字。Rust 标准库没有对应物，本模块改用 `f64` 的标准解析
//! （`str::parse`，只接受 `.` 作小数分隔符、不接受千分位）。因此非英文区域设置下带
//! 千分位或 `,` 小数分隔符的工期值会解析失败（返回 `None`）；区域设置仅用于解析单位后缀
//! （见 [`crate::mpx::time_unit_utility::TimeUnitUtility::get_instance`]）。
//!
//! Java 解析失败（`ParseException`）或单位无法识别（`MPXJException`）都会抛出异常，
//! Rust 侧统一返回 `None`。
//!
//! 未移植的成员：无。

use crate::duration::Duration;
use crate::mpx::locale_utility::MpxLocale;
use crate::mpx::time_unit_utility::TimeUnitUtility;
use crate::time_unit::TimeUnit;

/// 管理 MPX 文件中 `Duration` 实例的工具方法。
///
/// 对应 Java: `final class DurationUtility`（私有构造方法禁止实例化，Rust 侧用无字段类型表达）。
pub struct DurationUtility;

impl DurationUtility {
    /// 把 MPX 工期文本（如 `"8h"` / `"2d"` / `"8"`）解析为 `Duration` 实例。
    ///
    /// 对应 Java: `public static Duration getInstance(String dur, NumberFormat format, Locale locale)`
    ///
    /// Java 的 `dur` 为 `String`（`null` 会触发 `NullPointerException`），Rust 侧用
    /// `Option<&str>` 承载，`None` 返回 `None`。没有单位后缀时按「天」处理（兼容 MPX3）。
    pub fn get_instance(dur: Option<&str>, locale: MpxLocale) -> Option<Duration> {
        let chars: Vec<char> = dur?.chars().collect();
        if chars.is_empty() {
            // Java: format.parse("") 抛 ParseException
            return None;
        }

        let last_index = chars.len() - 1;
        let mut index = last_index;

        // Java: while ((index > 0) && !Character.isDigit(dur.charAt(index))) --index;
        // 注：Java 的 Character.isDigit 识别 Unicode 数字，这里用 ASCII 数字判定。
        while index > 0 && !chars[index].is_ascii_digit() {
            index -= 1;
        }

        if index == last_index {
            // 没有单位后缀，按天处理以兼容 MPX3
            let duration = parse_number(&chars[..])?;
            Some(Duration::get_instance(duration, TimeUnit::Days))
        } else {
            index += 1;
            let duration = parse_number(&chars[..index])?;

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

/// 解析数字前缀，对应 Java 的 `NumberFormat.parse(...)`。
///
/// Java 的 `NumberFormat.parse` 允许前导空白并允许在数字后停止解析；Rust 侧用 `trim` +
/// 整体解析近似（本地化格式、千分位暂不支持，见文件头说明）。
fn parse_number(chars: &[char]) -> Option<f64> {
    let text: String = chars.iter().collect();
    text.trim().parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 把工期按 MPX 文本形式写回（对应 MPX 写入侧用本地化时间单位名拼出的 `value + unitName`）。
    ///
    /// 注意：`TimeUnit::get_name()` 对 MONTHS / ELAPSED_MONTHS 返回 `"mo"` / `"emo"`，
    /// 而 MPX 英文名表用的是 `"mon"` / `"emon"`（见 `time_unit_utility`），
    /// 因此这两者需要单独映射。
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
            let duration = DurationUtility::get_instance(Some(text), MpxLocale::En)
                .unwrap_or_else(|| panic!("{text} 应当可以解析"));
            assert_eq!(duration.get_duration(), expected_value, "解析 {text} 的数值不符");
            assert_eq!(duration.get_units(), expected_units, "解析 {text} 的单位不符");

            // 模型值 → MPX 文本 → 模型值
            let round_tripped =
                DurationUtility::get_instance(Some(&to_mpx_string(duration)), MpxLocale::En)
                    .unwrap_or_else(|| panic!("{text} 往返后应当可以解析"));
            assert_eq!(round_tripped, duration);
        }
    }

    #[test]
    fn duration_without_unit_suffix_defaults_to_days() {
        // Java: 无单位后缀按天处理，以兼容 MPX3
        let duration = DurationUtility::get_instance(Some("2"), MpxLocale::En).unwrap();
        assert_eq!(duration, Duration::get_instance(2.0, TimeUnit::Days));

        let round_tripped = DurationUtility::get_instance(Some("2d"), MpxLocale::En).unwrap();
        assert_eq!(round_tripped, duration);
    }

    #[test]
    fn duration_allows_whitespace_before_units() {
        assert_eq!(
            DurationUtility::get_instance(Some("8 h"), MpxLocale::En),
            Some(Duration::get_instance(8.0, TimeUnit::Hours))
        );
        assert_eq!(
            DurationUtility::get_instance(Some("8   h"), MpxLocale::En),
            Some(Duration::get_instance(8.0, TimeUnit::Hours))
        );
    }

    #[test]
    fn duration_unit_suffix_is_case_insensitive() {
        assert_eq!(
            DurationUtility::get_instance(Some("8H"), MpxLocale::En),
            Some(Duration::get_instance(8.0, TimeUnit::Hours))
        );
        assert_eq!(
            DurationUtility::get_instance(Some("8Days"), MpxLocale::En),
            Some(Duration::get_instance(8.0, TimeUnit::Days))
        );
    }

    #[test]
    fn duration_rejects_invalid_values() {
        // 空串：Java 抛 ParseException
        assert_eq!(DurationUtility::get_instance(Some(""), MpxLocale::En), None);
        // null：Java 抛 NullPointerException
        assert_eq!(DurationUtility::get_instance(None, MpxLocale::En), None);
        // 非法单位：Java 抛 MPXJException(INVALID_TIME_UNIT)
        assert_eq!(DurationUtility::get_instance(Some("8x"), MpxLocale::En), None);
        // 非法数字
        assert_eq!(DurationUtility::get_instance(Some("abcd"), MpxLocale::En), None);
        // 结尾空白会被当作单位后缀的一部分，无法识别
        assert_eq!(DurationUtility::get_instance(Some("8h "), MpxLocale::En), None);
        // 只有单位没有数字
        assert_eq!(DurationUtility::get_instance(Some("h"), MpxLocale::En), None);
    }

    #[test]
    fn duration_parses_leading_whitespace_before_digits() {
        // Java: NumberFormat.parse 忽略前导空白
        assert_eq!(
            DurationUtility::get_instance(Some(" 8h"), MpxLocale::En),
            Some(Duration::get_instance(8.0, TimeUnit::Hours))
        );
    }

    #[test]
    fn duration_uses_locale_time_unit_table() {
        // 单位后缀按区域设置的名字表解析（Java 德语名表用 "t" 表示天）
        assert_eq!(
            DurationUtility::get_instance(Some("2t"), MpxLocale::De),
            Some(Duration::get_instance(2.0, TimeUnit::Days))
        );
        assert_eq!(
            DurationUtility::get_instance(Some("2d"), MpxLocale::De),
            None
        );
        assert_eq!(
            DurationUtility::get_instance(Some("2d"), MpxLocale::En),
            Some(Duration::get_instance(2.0, TimeUnit::Days))
        );
    }
}
