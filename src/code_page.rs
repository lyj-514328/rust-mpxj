//! 对应 Java: `org/mpxj/CodePage.java`

use std::fmt::{self, Display, Formatter};

/// MPX 文件使用的代码页。
///
/// 对应 Java: `public enum CodePage`
///
/// Java 侧每个常量携带 `(String value, Charset charset)` 两个字段，并用静态块把
/// `value` → 枚举写入 `NAME_MAP`。Rust 侧：
/// * `value`（MPX 文件中的代码页名）由 [`CodePage::get_value`] / [`Display`] 返回；
/// * `charset` 由 [`CodePage::get_charset`] 返回（`encoding_rs::Encoding`）；
/// * 映射改由 [`CodePage::get_instance`] 的 `match` 完成，不再需要 `NAME_MAP`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodePage {
    /// 对应 Java: `ANSI("ANSI", CharsetHelper.CP1252)`
    Ansi,
    /// 对应 Java: `MAC("MAC", CharsetHelper.MAC_ROMAN)`
    Mac,
    /// 对应 Java: `LATIN("850", CharsetHelper.CP850)`
    Latin,
    /// 对应 Java: `US("437", CharsetHelper.CP437)`
    Us,
    /// 对应 Java: `ZH("ZH", CharsetHelper.GB2312)`
    Zh,
    /// 对应 Java: `RU("RU", CharsetHelper.CP1251)`
    Ru,
}

impl CodePage {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `EnumSet.allOf(CodePage.class)` 遍历到的全部常量。
    pub const ALL: [CodePage; 6] = [
        CodePage::Ansi,
        CodePage::Mac,
        CodePage::Latin,
        CodePage::Us,
        CodePage::Zh,
        CodePage::Ru,
    ];

    /// 按 MPX 代码页名取枚举实例，无法识别时回落到 `ANSI`。
    ///
    /// 对应 Java: `public static CodePage getInstance(String value)`，即
    /// `NAME_MAP.getOrDefault(value, ANSI)`。
    pub fn get_instance(value: &str) -> CodePage {
        match value {
            "ANSI" => CodePage::Ansi,
            "MAC" => CodePage::Mac,
            "850" => CodePage::Latin,
            "437" => CodePage::Us,
            "ZH" => CodePage::Zh,
            "RU" => CodePage::Ru,
            _ => CodePage::Ansi,
        }
    }

    /// 按 MPX 代码页名取枚举实例，`None` 回落到 `ANSI`。
    ///
    /// 对应 Java: `NAME_MAP.getOrDefault(null, ANSI)` 的行为（`HashMap` 允许 null 键，
    /// 未命中时返回 `ANSI`）。
    pub fn get_instance_optional(value: Option<&str>) -> CodePage {
        match value {
            Some(value) => CodePage::get_instance(value),
            None => CodePage::Ansi,
        }
    }

    /// 返回该代码页对应的 Java 字符集名（`CharsetHelper` 中 `Charset.forName` 的参数）。
    ///
    /// 对应 Java: `private final Charset m_charset` 所对应的字符集标识。
    pub fn get_charset_name(&self) -> &'static str {
        match self {
            CodePage::Ansi => "Cp1252",
            CodePage::Mac => "MacRoman",
            CodePage::Latin => "Cp850",
            CodePage::Us => "Cp437",
            CodePage::Zh => "GB2312",
            CodePage::Ru => "Cp1251",
        }
    }

    /// 返回该代码页对应的字符集。
    ///
    /// 对应 Java: `public Charset getCharset()`
    ///
    /// `encoding_rs` 只实现 WHATWG Encoding 标准内的编码，其中不含 CP437 与 CP850，
    /// 因此这两种代码页返回 `None`（其字符集名仍可通过 [`CodePage::get_charset_name`]
    /// 取得）。`GB2312` 在 WHATWG 标准中由 `GBK` 承担，故映射到 [`encoding_rs::GBK`]。
    pub fn get_charset(&self) -> Option<&'static encoding_rs::Encoding> {
        match self {
            CodePage::Ansi => Some(encoding_rs::WINDOWS_1252),
            CodePage::Mac => Some(encoding_rs::MACINTOSH),
            // encoding_rs 没有 CP850 实现
            CodePage::Latin => None,
            // encoding_rs 没有 CP437 实现
            CodePage::Us => None,
            CodePage::Zh => Some(encoding_rs::GBK),
            CodePage::Ru => Some(encoding_rs::WINDOWS_1251),
        }
    }

    /// 返回 MPX 文件中的代码页名。
    ///
    /// 对应 Java: `private final String m_value`。
    pub fn get_value(&self) -> &'static str {
        match self {
            CodePage::Ansi => "ANSI",
            CodePage::Mac => "MAC",
            CodePage::Latin => "850",
            CodePage::Us => "437",
            CodePage::Zh => "ZH",
            CodePage::Ru => "RU",
        }
    }
}

impl Display for CodePage {
    /// 对应 Java: `@Override public String toString()`，返回 `m_value`。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_page_round_trips() {
        for item in CodePage::ALL {
            assert_eq!(CodePage::get_instance(item.get_value()), item);
            assert_eq!(CodePage::get_instance(&item.to_string()), item);
        }
    }

    #[test]
    fn code_page_falls_back_to_ansi() {
        assert_eq!(CodePage::get_instance("unknown"), CodePage::Ansi);
        assert_eq!(CodePage::get_instance(""), CodePage::Ansi);
        assert_eq!(CodePage::get_instance_optional(None), CodePage::Ansi);
    }

    #[test]
    fn code_page_charsets() {
        assert_eq!(CodePage::Ansi.get_charset(), Some(encoding_rs::WINDOWS_1252));
        assert_eq!(CodePage::Mac.get_charset(), Some(encoding_rs::MACINTOSH));
        assert_eq!(CodePage::Zh.get_charset(), Some(encoding_rs::GBK));
        assert_eq!(CodePage::Ru.get_charset(), Some(encoding_rs::WINDOWS_1251));
        // encoding_rs 未实现 CP437 / CP850
        assert_eq!(CodePage::Us.get_charset(), None);
        assert_eq!(CodePage::Latin.get_charset(), None);
        assert_eq!(CodePage::Latin.get_charset_name(), "Cp850");
    }
}
