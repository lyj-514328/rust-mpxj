//! Corresponds to Java: `org/mpxj/common/StringHelper.java`

/// String-related utility methods.
///
/// Corresponds to Java: `public final class StringHelper`
///
/// Porting notes:
/// - Java iterates over UTF-16 `char`s while Rust iterates over Unicode scalar values;
///   `Character.isISOControl` corresponds to `char::is_control`, and both cover the same code
///   point ranges (U+0000–U+001F and U+007F–U+009F).
/// - Java's private overload `stripControlCharacters(String, int)` corresponds to the
///   module-private function [`strip_control_characters_from`]; the meaning of `index` changes
///   from a UTF-16 index to a byte index.
///
/// Skipped Java methods: none.
pub struct StringHelper;

impl StringHelper {
    /// Corresponds to Java: `public static String stripControlCharacters(String text)`
    ///
    /// Removes control characters from the text; null returns `None`, and an empty string is returned unchanged.
    pub fn strip_control_characters(text: Option<&str>) -> Option<String> {
        let text = text?;

        match text.char_indices().find(|(_, c)| c.is_control()) {
            None => Some(text.to_string()),
            Some((index, _)) => Some(strip_control_characters_from(text, index)),
        }
    }
}

/// Corresponds to Java: `private static String stripControlCharacters(String text, int index)`
///
/// `index` is the byte index of the first control character; that character itself is dropped, and control characters in the remaining characters are also removed.
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
