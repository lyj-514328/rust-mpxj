//! 对应 Java: `org/mpxj/common/StringHelper.java`

/// 字符串相关的辅助方法。
///
/// 对应 Java: `public final class StringHelper`
///
/// 移植说明：
/// - Java 按 UTF-16 `char` 遍历，Rust 按 Unicode 标量值遍历；`Character.isISOControl` 对应
///   `char::is_control`，两者覆盖的码点范围一致（U+0000–U+001F 与 U+007F–U+009F）。
/// - Java 的私有重载 `stripControlCharacters(String, int)` 对应模块内私有函数
///   [`strip_control_characters_from`]，`index` 的含义由 UTF-16 下标改为字节下标。
///
/// 未移植的方法：无。
pub struct StringHelper;

impl StringHelper {
    /// 对应 Java: `public static String stripControlCharacters(String text)`
    ///
    /// 去掉文本中的控制字符；null 返回 `None`，空串原样返回。
    pub fn strip_control_characters(text: Option<&str>) -> Option<String> {
        let text = text?;

        match text.char_indices().find(|(_, c)| c.is_control()) {
            None => Some(text.to_string()),
            Some((index, _)) => Some(strip_control_characters_from(text, index)),
        }
    }
}

/// 对应 Java: `private static String stripControlCharacters(String text, int index)`
///
/// `index` 为第一个控制字符的字节下标，该字符本身会被丢弃，其余字符中的控制字符也会被去掉。
fn strip_control_characters_from(text: &str, index: usize) -> String {
    let mut result = String::with_capacity(text.len());
    result.push_str(&text[..index]);

    for c in text[index..].chars().skip(1) {
        if !c.is_control() {
            result.push(c);
        }
    }

    result
}
