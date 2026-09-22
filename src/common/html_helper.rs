//! Corresponds to Java: `org/mpxj/common/HtmlHelper.java`
//!
//! # Differences from Java (important)
//!
//! Java relies on [jsoup](https://jsoup.org/) to build a DOM tree and then uses `NodeTraversor`
//! to walk it and extract plain text. This repository does not introduce new dependencies, so
//! the Rust side uses a **lightweight streaming parser** to reproduce the behavior of
//! `FormattingVisitor`:
//!
//! - `head` phase: `li` → `"\n * "`, `dt` → `"  "`, `p` / `h1`..`h5` / `tr` → `"\n"`;
//! - `tail` phase: `br` / `dd` / `dt` / `p` / `h1`..`h5` → `"\n"`,
//!   `a` → ``" <" + href + ">"`` (jsoup's `absUrl("href")` is the raw attribute value when
//!   there is no base URI);
//! - Text nodes are appended in order, and `" "` is not duplicated with leading/trailing
//!   whitespace or newlines in the buffer;
//! - Character entities in text nodes (`&amp;`, `&#160;`, etc.) are decoded using a table of
//!   common entities; uncovered entities are left as-is (a difference from jsoup's complete
//!   entity table).
//!
//! Because no tree is built, `getPlainTextFromHtml` and `getPlainTextFromBodyFragment` take the
//! same parsing path on the Rust side; the output for the tags/text above matches jsoup's tree
//! traversal, but jsoup's tree-repair behavior for malformed HTML (e.g. the ownership of
//! automatically inserted closing tags) cannot be fully reproduced.
//!
//! The contents of `script` / `style` are `DataNode`s in jsoup (`nodeName()` is `#data`) and are
//! not appended as text; the Rust side likewise skips the contents of these tags.

/// HTML-related helper methods.
///
/// Corresponds to Java: `public final class HtmlHelper`
pub struct HtmlHelper;

impl HtmlHelper {
    /// Roughly convert plain text (preserving newlines) to an HTML document.
    ///
    /// Corresponds to Java: `public static String getHtmlFromPlainText(String text)`
    ///
    /// Java throws `NullPointerException` for `null`; the Rust parameter is `&str` and cannot be
    /// `null`.
    pub fn get_html_from_plain_text(text: &str) -> String {
        format!(
            "<html><body>{}</body></html>",
            text.replace('\n', "<br/>\n")
        )
    }

    /// Extract plain text from HTML.
    ///
    /// Corresponds to Java: `public static String strip(String html)`
    ///
    /// A `None` input returns `None`; an empty extraction result also returns `None`.
    /// Java's `replaceAll("(^\\h*)|(\\h*$)", "").trim()` is approximated on the Rust side with
    /// [`str::trim`] (Rust's `trim` likewise strips leading/trailing `\u{A0}`, with a slightly
    /// wider range).
    pub fn strip(html: Option<&str>) -> Option<String> {
        let html = html?;

        // Java: look for "<HTML>" first, then "<html>" if not found
        let html_index = html.find("<HTML>").or_else(|| html.find("<html>"));

        let result = match html_index {
            // Java: getPlainTextFromHtml(result.substring(htmlIndex))
            Some(index) => Self::get_plain_text_from_html(&html[index..]),
            // Java: when there is no <html> tag, treat it as a body fragment
            None => Self::get_plain_text_from_body_fragment(html),
        };

        let trimmed = result.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    }

    /// Extract plain text from a complete HTML document.
    ///
    /// Corresponds to Java: `public static String getPlainTextFromHtml(String html)`
    pub fn get_plain_text_from_html(html: &str) -> String {
        Self::get_plain_text(html)
    }

    /// Extract plain text from a text fragment that may contain HTML elements.
    ///
    /// Corresponds to Java: `public static String getPlainTextFromBodyFragment(String body)`
    pub fn get_plain_text_from_body_fragment(body: &str) -> String {
        Self::get_plain_text(body)
    }

    /// Walk (streaming parse) the document to extract plain text.
    ///
    /// Corresponds to Java: `private static String getPlainText(Document document)`
    fn get_plain_text(source: &str) -> String {
        let mut visitor = FormattingVisitor::new();
        tokenize(source, &mut visitor);
        visitor.into_string()
    }
}

/// Corresponds to Java: `private static class HtmlHelper.FormattingVisitor`
struct FormattingVisitor {
    /// Corresponds to Java field `private final StringBuilder m_buffer`
    buffer: String,
}

impl FormattingVisitor {
    /// Corresponds to Java: `public FormattingVisitor()`
    fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Corresponds to Java: `@Override public void head(Node node, int depth)`
    ///
    /// Only appends to the buffer when the tag name matches one of the lists in Java.
    fn head_start_tag(&mut self, name: &str) {
        match name {
            "li" => self.append("\n * "),
            "dt" => self.append("  "),
            "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "tr" => self.append("\n"),
            _ => {}
        }
    }

    /// Corresponds to Java: `@Override public void tail(Node node, int depth)`
    fn tail_end_tag(&mut self, name: &str, href: Option<&str>) {
        match name {
            "br" | "dd" | "dt" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" => self.append("\n"),
            "a" => {
                let href = href.unwrap_or("");
                self.append(&format!(" <{href}>"));
            }
            _ => {}
        }
    }

    /// Corresponds to Java: appending after calling `((TextNode) node).text()` on a `TextNode` in `head`.
    fn text(&mut self, text: &str) {
        let decoded = decode_entities(text);
        self.append(&decoded);
    }

    /// Corresponds to Java: `private void append(String text)`
    ///
    /// Avoid consecutive whitespace: a text node consisting of a single space is dropped when
    /// the buffer is empty or ends with a space/newline.
    fn append(&mut self, text: &str) {
        if text == " "
            && (self.buffer.is_empty()
                || self.buffer.ends_with(' ')
                || self.buffer.ends_with('\n'))
        {
            return;
        }

        self.buffer.push_str(text);
    }

    /// Corresponds to Java: `@Override public String toString()`
    fn into_string(self) -> String {
        self.buffer
    }
}

/// An element in the DOM tree traversal that has not yet been closed.
struct OpenElement {
    /// Lowercase tag name.
    name: String,
    /// The `href` attribute value of an `a` tag (used for output in the `tail` phase).
    href: Option<String>,
}

/// Corresponds to the depth-first traversal of the node tree by
/// `NodeTraversor.traverse(formatter, document)` in Java.
///
/// Here it is changed to a streaming scan of the raw HTML: entering `head` on a start tag, and
/// emitting `tail` from the inside out on an end tag (or at end of input).
fn tokenize(source: &str, visitor: &mut FormattingVisitor) {
    let mut rest = source;
    let mut stack: Vec<OpenElement> = Vec::new();

    while let Some(tag_start) = rest.find('<') {
        visitor.text(&rest[..tag_start]);
        rest = &rest[tag_start..];

        // Comment: skip the entire comment; in jsoup it is a Comment node and produces no text.
        if let Some(after_open) = rest.strip_prefix("<!--") {
            match after_open.find("-->") {
                Some(index) => {
                    rest = &after_open[index + 3..];
                    continue;
                }
                None => break,
            }
        }

        // Document type declaration / other <! ... >: skip.
        if rest.starts_with("<!") {
            match rest.find('>') {
                Some(index) => {
                    rest = &rest[index + 1..];
                    continue;
                }
                None => {
                    rest = "";
                    break;
                }
            }
        }

        // No '>' after an unclosed '<': treat it as ordinary text.
        let Some(tag_end) = rest.find('>') else {
            visitor.text(rest);
            rest = "";
            break;
        };

        let tag_content = &rest[1..tag_end];
        rest = &rest[tag_end + 1..];

        if let Some(name) = tag_content.strip_prefix('/') {
            // End tag
            let name = name.trim().to_ascii_lowercase();
            close_element(&name, &mut stack, visitor);
            continue;
        }

        let self_closing = tag_content.ends_with('/');
        let name = tag_name(tag_content);
        if name.is_empty() {
            continue;
        }

        // The contents of script / style are DataNodes in jsoup (`nodeName()` is `#data`)
        // and are not appended as TextNodes; here the whole span is skipped to the matching end tag.
        if !self_closing && (name == "script" || name == "style") {
            rest = skip_raw_text_element(rest, &name);
            continue;
        }

        visitor.head_start_tag(&name);

        if self_closing || is_void_element(&name) {
            visitor.tail_end_tag(&name, href_for(tag_content, &name).as_deref());
        } else {
            stack.push(OpenElement {
                href: href_for(tag_content, &name),
                name,
            });
        }
    }

    // Trailing text remaining after the tags.
    if !rest.is_empty() {
        visitor.text(rest);
    }

    // After the input ends, jsoup implicitly closes the elements still on the stack, and the traversal likewise emits their tail.
    while let Some(element) = stack.pop() {
        visitor.tail_end_tag(&element.name, element.href.as_deref());
    }
}

/// Skip the raw contents of a `script` / `style` element and return the position after the
/// closing tag.
fn skip_raw_text_element<'a>(text: &'a str, name: &str) -> &'a str {
    let lower = text.to_ascii_lowercase();
    let closing = format!("</{name}");

    let Some(index) = lower.find(&closing) else {
        return "";
    };

    match text[index..].find('>') {
        Some(end) => &text[index + end + 1..],
        None => "",
    }
}

/// Close the element with the given name: pop and emit the tail of elements until (and including)
/// that element.
fn close_element(name: &str, stack: &mut Vec<OpenElement>, visitor: &mut FormattingVisitor) {
    let Some(position) = stack.iter().rposition(|element| element.name == name) else {
        return;
    };

    while stack.len() > position {
        if let Some(element) = stack.pop() {
            visitor.tail_end_tag(&element.name, element.href.as_deref());
        }
    }
}

/// Extract the lowercase tag name from the tag content (excluding attributes and the self-closing
/// slash).
fn tag_name(tag_content: &str) -> String {
    tag_content
        .trim_start()
        .split(|c: char| c.is_whitespace() || c == '/')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// Extract the `href` attribute value of an `a` tag (jsoup's `absUrl("href")` returns the raw
/// attribute value when there is no base URI).
fn href_for(tag_content: &str, name: &str) -> Option<String> {
    if name != "a" {
        return None;
    }

    let attribute_start = tag_content.find("href")? + "href".len();
    let after_name = tag_content[attribute_start..].trim_start();
    let after_equals = after_name.strip_prefix('=')?.trim_start();
    let quote = after_equals.chars().next()?;

    if quote == '"' || quote == '\'' {
        let remaining = &after_equals[quote.len_utf8()..];
        let end = remaining.find(quote)?;
        Some(remaining[..end].to_owned())
    } else {
        let end = after_equals
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .unwrap_or(after_equals.len());
        Some(after_equals[..end].to_owned())
    }
}

/// HTML void elements (no closing needed in jsoup; `tail` immediately follows `head`).
fn is_void_element(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

/// Decode character entities in a text node (jsoup decodes them during parsing).
///
/// Covers numeric entities and common named entities; uncovered named entities are left as-is.
fn decode_entities(text: &str) -> String {
    if !text.contains('&') {
        return text.to_owned();
    }

    let mut result = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(amp) = rest.find('&') {
        result.push_str(&rest[..amp]);
        let candidate = &rest[amp..];

        match decode_entity_at(candidate) {
            Some((decoded, length)) => {
                result.push(decoded);
                rest = &candidate[length..];
            }
            None => {
                result.push('&');
                rest = &candidate[1..];
            }
        }
    }

    result.push_str(rest);
    result
}

/// Try to decode the entity at the start, returning (character, number of bytes consumed).
fn decode_entity_at(candidate: &str) -> Option<(char, usize)> {
    let body = candidate.strip_prefix('&')?;
    let semicolon = body.find(';')?;
    // jsoup allows entities not terminated by ';', but named entities have a bounded length; the search range is limited here.
    if semicolon > 32 {
        return None;
    }

    let name = &body[..semicolon];
    let length = semicolon + 2;

    if let Some(digits) = name.strip_prefix('#') {
        let value = if let Some(hex) = digits
            .strip_prefix('x')
            .or_else(|| digits.strip_prefix('X'))
        {
            u32::from_str_radix(hex, 16).ok()?
        } else {
            digits.parse::<u32>().ok()?
        };
        return char::from_u32(value).map(|c| (c, length));
    }

    named_entity(name).map(|c| (c, length))
}

/// Common named character entity table (lowercase).
fn named_entity(name: &str) -> Option<char> {
    let value = match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => '\u{A0}',
        "copy" => '\u{A9}',
        "reg" => '\u{AE}',
        "trade" => '\u{2122}',
        "hellip" => '\u{2026}',
        "ndash" => '\u{2013}',
        "mdash" => '\u{2014}',
        "lsquo" => '\u{2018}',
        "rsquo" => '\u{2019}',
        "ldquo" => '\u{201C}',
        "rdquo" => '\u{201D}',
        "laquo" => '\u{AB}',
        "raquo" => '\u{BB}',
        "middot" => '\u{B7}',
        "bull" => '\u{2022}',
        "deg" => '\u{B0}',
        "times" => '\u{D7}',
        "divide" => '\u{F7}',
        "plusmn" => '\u{B1}',
        "euro" => '\u{20AC}',
        "pound" => '\u{A3}',
        "yen" => '\u{A5}',
        "sect" => '\u{A7}',
        "para" => '\u{B6}',
        "sup2" => '\u{B2}',
        "sup3" => '\u{B3}',
        "frac12" => '\u{BD}',
        _ => return None,
    };

    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_html_from_plain_text_preserves_line_breaks() {
        assert_eq!(
            HtmlHelper::get_html_from_plain_text("a\nb"),
            "<html><body>a<br/>\nb</body></html>"
        );
    }

    #[test]
    fn strip_returns_none_for_null() {
        assert_eq!(HtmlHelper::strip(None), None);
    }

    #[test]
    fn strip_plain_text_round_trips() {
        assert_eq!(
            HtmlHelper::strip(Some("plain text")),
            Some("plain text".to_owned())
        );
    }

    #[test]
    fn strip_full_html_document() {
        assert_eq!(
            HtmlHelper::strip(Some("<html><body><p>hello</p></body></html>")),
            Some("hello".to_owned())
        );
    }

    #[test]
    fn strip_body_fragment_without_html_tag() {
        assert_eq!(
            HtmlHelper::strip(Some("<b>bold</b> text")),
            Some("bold text".to_owned())
        );
    }

    #[test]
    fn strip_emits_newlines_for_paragraph_and_breaks() {
        // Corresponds to Java FormattingVisitor: the tail of <br> appends "\n"
        assert_eq!(
            HtmlHelper::strip(Some("<html><body>a<br/>b</body></html>")),
            Some("a\nb".to_owned())
        );
    }

    #[test]
    fn strip_lists_and_links() {
        assert_eq!(
            HtmlHelper::strip(Some("<ul><li>one</li><li>two</li></ul>")),
            Some("\n * one\n * two".trim().to_owned())
        );

        assert_eq!(
            HtmlHelper::strip(Some("<a href=\"http://example.com\">link</a>")),
            Some("link <http://example.com>".to_owned())
        );
    }

    #[test]
    fn strip_decodes_entities_and_reports_empty_as_none() {
        assert_eq!(
            HtmlHelper::strip(Some("<p>a&amp;b</p>")),
            Some("a&b".to_owned())
        );

        assert_eq!(HtmlHelper::strip(Some("<html><body>&nbsp;</body></html>")), None);
        assert_eq!(HtmlHelper::strip(Some("")), None);
    }

    #[test]
    fn get_plain_text_helpers_share_implementation() {
        assert_eq!(
            HtmlHelper::get_plain_text_from_html("<html><body>x</body></html>"),
            "x"
        );
        assert_eq!(HtmlHelper::get_plain_text_from_body_fragment("x"), "x");
    }

    #[test]
    fn script_and_style_contents_are_skipped() {
        assert_eq!(
            HtmlHelper::strip(Some("<body>a<script>var x = 1;</script>b</body>")),
            Some("ab".to_owned())
        );
    }

    #[test]
    fn collapsed_space_is_not_duplicated() {
        // The text " " is dropped when the buffer already ends with a space/newline
        assert_eq!(
            HtmlHelper::strip(Some("<body>a <b> </b>c</body>")),
            Some("a c".to_owned())
        );
    }
}
