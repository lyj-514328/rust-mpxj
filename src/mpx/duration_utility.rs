//! 对应 Java: `org/mpxj/mpx/DurationUtility.java`
//!
//! # 移植取舍（`java.text.NumberFormat`）
//!
//! Java 用 `java.text.NumberFormat`（MPX 侧即 [`MPXJNumberFormat`]）解析工期文本中的数字前缀，
//! 因而小数分隔符、千分位等随区域设置变化（德语 / 法语用 `,` 作小数分隔符）。
//! Rust 侧同样以「调用方传入的格式对象 + 区域设置」为入参，格式对象即
//! [`crate::mpx::mpxj_formats::MPXJFormats::get_duration_decimal_format`]
//! （模式 `#.##` + 该区域的分隔符），因此 `1,5t` 之类的小数工期与 Java 解析结果一致。
//!
//! Java 解析失败（`ParseException`）或单位无法识别（`MPXJException`）都会抛出异常，
//! Rust 侧统一返回 `None`。
//!
//! 未移植的成员：无。

use crate::duration::Duration;
use crate::mpx::locale_utility::MpxLocale;
use crate::mpx::mpxj_number_format::MPXJNumberFormat;
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
    /// 数字用 `format`（[`MPXJNumberFormat`]，即 Java 的 `m_formats.getDurationDecimalFormat()`）
    /// 解析，解析失败返回 `None`（Java 抛 `ParseException` → `MPXJException`）。
    pub fn get_instance(
        dur: Option<&str>,
        format: &MPXJNumberFormat,
        locale: MpxLocale,
    ) -> Option<Duration> {
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

/// 解析数字前缀，对应 Java 的 `NumberFormat.parse(...)`。
///
/// Java 的 `NumberFormat.parse` 允许前导空白、允许千分位，并允许在数字后停止解析；
/// [`MPXJNumberFormat::parse`] 的行为与之一致（见该模块文件头），解析失败返回 `None`
/// （Java 抛 `ParseException`）。
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

    /// 构造指定区域设置的格式集合，等价于读取器在 `read` 中的做法
    /// （先 [`LocaleUtility::set_locale`] 再 `new MPXJFormats`）。
    ///
    /// 返回的 `ProjectFile` 必须由调用方持有：`MPXJFormats` 只保存弱引用。
    fn formats_for(locale: MpxLocale) -> (Rc<RefCell<ProjectFile>>, MPXJFormats) {
        let file = ProjectFile::new();
        LocaleUtility::set_locale(file.borrow_mut().get_project_properties_mut(), locale);
        let formats = MPXJFormats::new(locale, "NA", Rc::downgrade(&file));
        (file, formats)
    }

    /// 用「工期小数格式」解析一段文本（等价于 Java 的
    /// `DurationUtility.getInstance(text, m_formats.getDurationDecimalFormat(), locale)`）。
    fn parse(formats: &MPXJFormats, locale: MpxLocale, text: &str) -> Option<Duration> {
        DurationUtility::get_instance(Some(text), formats.get_duration_decimal_format(), locale)
    }

    /// 把工期按 MPX 文本形式写回（对应 MPX 写入侧用本地化时间单位名拼出的
    /// `value + unitName`）。
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
                .unwrap_or_else(|| panic!("{text} 应当可以解析"));
            assert_eq!(
                duration.get_duration(),
                expected_value,
                "解析 {text} 的数值不符"
            );
            assert_eq!(
                duration.get_units(),
                expected_units,
                "解析 {text} 的单位不符"
            );

            // 模型值 → MPX 文本 → 模型值
            let round_tripped = parse(&formats, MpxLocale::En, &to_mpx_string(duration))
                .unwrap_or_else(|| panic!("{text} 往返后应当可以解析"));
            assert_eq!(round_tripped, duration);
        }
    }

    #[test]
    fn duration_without_unit_suffix_defaults_to_days() {
        let (_file, formats) = formats_for(MpxLocale::En);

        // Java: 无单位后缀按天处理，以兼容 MPX3
        let duration = parse(&formats, MpxLocale::En, "2").expect("2 应当可以解析");
        assert_eq!(duration, Duration::get_instance(2.0, TimeUnit::Days));

        let round_tripped = parse(&formats, MpxLocale::En, "2d").expect("2d 应当可以解析");
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

        // 空串：Java 抛 ParseException
        assert_eq!(parse(&formats, MpxLocale::En, ""), None);
        // null：Java 抛 NullPointerException
        assert_eq!(
            DurationUtility::get_instance(
                None,
                formats.get_duration_decimal_format(),
                MpxLocale::En
            ),
            None
        );
        // 非法单位：Java 抛 MPXJException(INVALID_TIME_UNIT)
        assert_eq!(parse(&formats, MpxLocale::En, "8x"), None);
        // 非法数字
        assert_eq!(parse(&formats, MpxLocale::En, "abcd"), None);
        // 结尾空白会被当作单位后缀的一部分，无法识别
        assert_eq!(parse(&formats, MpxLocale::En, "8h "), None);
        // 只有单位没有数字
        assert_eq!(parse(&formats, MpxLocale::En, "h"), None);
    }

    #[test]
    fn duration_parses_leading_whitespace_before_digits() {
        let (_file, formats) = formats_for(MpxLocale::En);

        // Java: NumberFormat.parse 忽略前导空白
        assert_eq!(
            parse(&formats, MpxLocale::En, " 8h"),
            Some(Duration::get_instance(8.0, TimeUnit::Hours))
        );
    }

    #[test]
    fn duration_uses_locale_time_unit_table() {
        let (_file, formats) = formats_for(MpxLocale::De);

        // 单位后缀按区域设置的名字表解析（Java 德语名表用 "t" 表示天）
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

    /// 回归：德语 / 法语等「小数逗号」区域的小数工期必须能解析。
    ///
    /// 修复前 `DurationUtility` 不接收 `NumberFormat`，用 `f64::from_str` 顶替，
    /// `1,5t` 会解析失败并被静默丢弃（见 TECH_DEBT「八、已确认但待接线」第 2 条）。
    #[test]
    fn duration_parses_locale_decimal_separator() {
        // 德语：小数分隔符 ','、分组分隔符 '.'（见 `locale_data_de`）
        let (_file, formats) = formats_for(MpxLocale::De);
        assert_eq!(
            parse(&formats, MpxLocale::De, "1,5t"),
            Some(Duration::get_instance(1.5, TimeUnit::Days))
        );
        assert_eq!(
            parse(&formats, MpxLocale::De, "2,25h"),
            Some(Duration::get_instance(2.25, TimeUnit::Hours))
        );

        // 法语同样使用小数逗号
        let (_file_fr, formats_fr) = formats_for(MpxLocale::Fr);
        assert_eq!(
            parse(&formats_fr, MpxLocale::Fr, "1,5j"),
            Some(Duration::get_instance(1.5, TimeUnit::Days))
        );

        // 英文区域下 `,` 是分组分隔符（Java 的 `DecimalFormat` 解析时同样接受分组分隔符），
        // 因此 `1,5d` 得到 15 天 —— 与 Java 完全一致，不做「纠正」。
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
