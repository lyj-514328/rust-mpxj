//! 对应 Java: `org/mpxj/common/CharsetHelper.java`
//!
//! 常用字符集常量。Java 侧每个常量都是 `java.nio.charset.Charset`；Rust 侧统一用
//! `encoding_rs::Encoding`（与 [`crate::CodePage::get_charset`] 保持一致）。
//!
//! 移植取舍：
//! - Java 侧常量不可能为 null（`Charset.forName` 对未知名字抛异常）；Rust 的 `encoding_rs`
//!   只实现 WHATWG Encoding 标准内的编码，缺少 CP437、CP850，也没有「BOM 感知」的 UTF-16，
//!   因此常量类型为 `Option<&'static Encoding>`：无法映射的常量取 `None`（其 Java 名字仍可
//!   通过 [`CharsetHelper::get_charset`] 判定）。
//! - Java 的 `StandardCharsets.UTF_16` 会按 BOM 决定字节序（无 BOM 时按 BE）；`encoding_rs`
//!   没有对应编码，`UTF16` 返回 `None`，调用方探测 BOM 后改用 `UTF16LE` 或
//!   `encoding_rs::UTF_16BE`。
//! - `GB2312` 在 WHATWG 标准中由 `GBK` 承担，故映射到 [`encoding_rs::GBK`]
//!   （与 `CodePage::get_charset` 一致）。
//! - Java 的 `Charset.forName(String)` 在 Rust 中由 [`CharsetHelper::get_charset`] 承担，
//!   无法识别或无法映射时返回 `None`（不抛异常）。
//!
//! 未移植的方法：无。

use encoding_rs::Encoding;

/// 常用字符集常量。
///
/// 对应 Java: `public class CharsetHelper`
pub struct CharsetHelper;

impl CharsetHelper {
    /// 对应 Java: `public static final Charset UTF8 = StandardCharsets.UTF_8`
    pub const UTF8: Option<&'static Encoding> = Some(encoding_rs::UTF_8);

    /// 对应 Java: `public static final Charset UTF16 = StandardCharsets.UTF_16`
    ///
    /// Java 侧按 BOM 判定字节序，`encoding_rs` 无对应编码，故为 `None`。
    pub const UTF16: Option<&'static Encoding> = None;

    /// 对应 Java: `public static final Charset UTF16LE = StandardCharsets.UTF_16LE`
    pub const UTF16LE: Option<&'static Encoding> = Some(encoding_rs::UTF_16LE);

    /// 对应 Java: `public static final Charset CP1252 = Charset.forName("Cp1252")`
    pub const CP1252: Option<&'static Encoding> = Some(encoding_rs::WINDOWS_1252);

    /// 对应 Java: `public static final Charset MAC_ROMAN = Charset.forName("MacRoman")`
    pub const MAC_ROMAN: Option<&'static Encoding> = Some(encoding_rs::MACINTOSH);

    /// 对应 Java: `public static final Charset CP850 = Charset.forName("Cp850")`
    ///
    /// `encoding_rs` 未实现 CP850，故为 `None`。
    pub const CP850: Option<&'static Encoding> = None;

    /// 对应 Java: `public static final Charset CP437 = Charset.forName("Cp437")`
    ///
    /// `encoding_rs` 未实现 CP437，故为 `None`。
    pub const CP437: Option<&'static Encoding> = None;

    /// 对应 Java: `public static final Charset GB2312 = Charset.forName("GB2312")`
    pub const GB2312: Option<&'static Encoding> = Some(encoding_rs::GBK);

    /// 对应 Java: `public static final Charset CP1251 = Charset.forName("Cp1251")`
    pub const CP1251: Option<&'static Encoding> = Some(encoding_rs::WINDOWS_1251);

    /// 按字符集名取字符集，大小写不敏感，无法识别或无法映射时返回 `None`。
    ///
    /// 对应 Java: `Charset.forName(String)`（Java 对未知名字抛
    /// `UnsupportedCharsetException`，Rust 侧改为返回 `None`）。
    ///
    /// 名字沿用 Java / MPX 文件中的写法（如 `Cp1252`、`GB2312`，见
    /// [`crate::CodePage::get_charset_name`]）。`Cp437`、`Cp850` 与 BOM 感知的 `UTF-16`
    /// 在 `encoding_rs` 中无实现，同样返回 `None`。
    pub fn get_charset(name: &str) -> Option<&'static Encoding> {
        match name.to_ascii_lowercase().as_str() {
            "utf-8" | "utf8" => Some(encoding_rs::UTF_8),
            "utf-16le" | "utf16le" => Some(encoding_rs::UTF_16LE),
            "cp1252" | "windows-1252" => Some(encoding_rs::WINDOWS_1252),
            "macroman" | "macintosh" => Some(encoding_rs::MACINTOSH),
            "gb2312" | "gbk" => Some(encoding_rs::GBK),
            "cp1251" | "windows-1251" => Some(encoding_rs::WINDOWS_1251),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CodePage;

    #[test]
    fn constants_match_java_declarations() {
        assert_eq!(CharsetHelper::UTF8, Some(encoding_rs::UTF_8));
        assert_eq!(CharsetHelper::UTF16LE, Some(encoding_rs::UTF_16LE));
        assert_eq!(CharsetHelper::CP1252, Some(encoding_rs::WINDOWS_1252));
        assert_eq!(CharsetHelper::MAC_ROMAN, Some(encoding_rs::MACINTOSH));
        assert_eq!(CharsetHelper::GB2312, Some(encoding_rs::GBK));
        assert_eq!(CharsetHelper::CP1251, Some(encoding_rs::WINDOWS_1251));
    }

    #[test]
    fn encodings_missing_from_encoding_rs_are_none() {
        // CP437 / CP850 在 encoding_rs 中无实现
        assert_eq!(CharsetHelper::CP437, None);
        assert_eq!(CharsetHelper::CP850, None);
        // Java 的 UTF_16 按 BOM 判定字节序，encoding_rs 无对应编码
        assert_eq!(CharsetHelper::UTF16, None);
    }

    #[test]
    fn get_charset_resolves_java_names() {
        assert_eq!(CharsetHelper::get_charset("Cp1252"), Some(encoding_rs::WINDOWS_1252));
        assert_eq!(CharsetHelper::get_charset("cp1252"), Some(encoding_rs::WINDOWS_1252));
        assert_eq!(CharsetHelper::get_charset("MacRoman"), Some(encoding_rs::MACINTOSH));
        assert_eq!(CharsetHelper::get_charset("GB2312"), Some(encoding_rs::GBK));
        assert_eq!(CharsetHelper::get_charset("Cp1251"), Some(encoding_rs::WINDOWS_1251));
        assert_eq!(CharsetHelper::get_charset("UTF-16LE"), Some(encoding_rs::UTF_16LE));
        assert_eq!(CharsetHelper::get_charset("UTF-8"), Some(encoding_rs::UTF_8));
    }

    #[test]
    fn get_charset_returns_none_for_unmappable_names() {
        assert_eq!(CharsetHelper::get_charset("Cp437"), None);
        assert_eq!(CharsetHelper::get_charset("Cp850"), None);
        assert_eq!(CharsetHelper::get_charset("UTF-16"), None);
        assert_eq!(CharsetHelper::get_charset("no-such-charset"), None);
    }

    #[test]
    fn get_charset_matches_code_page_charsets() {
        // `CodePage::get_charset_name` 产出的名字应能被 `CharsetHelper` 还原为同一字符集
        for page in CodePage::ALL {
            assert_eq!(
                CharsetHelper::get_charset(page.get_charset_name()),
                page.get_charset(),
                "代码页 {:?} 的字符集映射不一致",
                page
            );
        }
    }

    #[test]
    fn constants_can_decode_bytes() {
        let encoding = CharsetHelper::CP1252.expect("CP1252 有实现");
        let (decoded, _, _) = encoding.decode(&[0xE9]);
        assert_eq!(decoded, "é");

        let encoding = CharsetHelper::GB2312.expect("GB2312 有实现");
        let (decoded, _, _) = encoding.decode(&[0xD6, 0xD0]);
        assert_eq!(decoded, "中");
    }
}
