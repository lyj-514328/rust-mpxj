//! 对应 Java: `org/mpxj/common/HtmlHelper.java`
//!
//! # 与 Java 的差异（重要）
//!
//! Java 侧依赖 [jsoup](https://jsoup.org/) 建 DOM 树，再用 `NodeTraversor` 遍历
//! 抽取纯文本。本仓库不引入新依赖，因此 Rust 侧用一个**轻量流式解析器**复现
//! `FormattingVisitor` 的行为：
//!
//! - `head` 阶段：`li` → `"\n * "`，`dt` → `"  "`，`p` / `h1`..`h5` / `tr` → `"\n"`；
//! - `tail` 阶段：`br` / `dd` / `dt` / `p` / `h1`..`h5` → `"\n"`，
//!   `a` → ``" <" + href + ">"``（jsoup 的 `absUrl("href")` 在无 base URI 时即属性原值）；
//! - 文本节点按顺序追加，`" "` 不与缓冲区首尾的空白/换行重复；
//! - 文本节点中的字符实体（`&amp;`、`&#160;` 等）会被解码，实现采用常见实体表，
//!   未覆盖的实体原样保留（与 jsoup 的完整实体表存在差异）。
//!
//! 由于不建树，`getPlainTextFromHtml` 与 `getPlainTextFromBodyFragment` 在
//! Rust 侧走同一条解析路径；对上述这些标签/文本的输出结果与 jsoup 树遍历一致，
//! 但 jsoup 对畸形 HTML 的树修复行为（例如自动补全闭合标签的归属）无法完全复现。
//!
//! `script` / `style` 的内容在 jsoup 中是 `DataNode`（`nodeName()` 为 `#data`），
//! 不会被当作文本追加；Rust 侧对这些标签的内容同样跳过。

/// HTML 相关的辅助方法。
///
/// 对应 Java: `public final class HtmlHelper`
pub struct HtmlHelper;

impl HtmlHelper {
    /// 把纯文本（保留换行）粗略转换为 HTML 文档。
    ///
    /// 对应 Java: `public static String getHtmlFromPlainText(String text)`
    ///
    /// Java 对 `null` 会抛 `NullPointerException`；Rust 侧参数为 `&str`，不可为 `null`。
    pub fn get_html_from_plain_text(text: &str) -> String {
        format!(
            "<html><body>{}</body></html>",
            text.replace('\n', "<br/>\n")
        )
    }

    /// 从 HTML 中抽取纯文本。
    ///
    /// 对应 Java: `public static String strip(String html)`
    ///
    /// `None` 入参返回 `None`；抽取结果为空串时同样返回 `None`。
    /// Java 的 `replaceAll("(^\\h*)|(\\h*$)", "").trim()` 在 Rust 侧用
    /// [`str::trim`] 近似（Rust 的 `trim` 同样会去掉首尾的 `\u{A0}`，且范围略宽）。
    pub fn strip(html: Option<&str>) -> Option<String> {
        let html = html?;

        // Java: 先找 "<HTML>"，找不到再找 "<html>"
        let html_index = html.find("<HTML>").or_else(|| html.find("<html>"));

        let result = match html_index {
            // Java: getPlainTextFromHtml(result.substring(htmlIndex))
            Some(index) => Self::get_plain_text_from_html(&html[index..]),
            // Java: 无 <html> 标签时按 body fragment 处理
            None => Self::get_plain_text_from_body_fragment(html),
        };

        let trimmed = result.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    }

    /// 从完整 HTML 文档中抽取纯文本。
    ///
    /// 对应 Java: `public static String getPlainTextFromHtml(String html)`
    pub fn get_plain_text_from_html(html: &str) -> String {
        Self::get_plain_text(html)
    }

    /// 从可能含 HTML 元素的文本片段中抽取纯文本。
    ///
    /// 对应 Java: `public static String getPlainTextFromBodyFragment(String body)`
    pub fn get_plain_text_from_body_fragment(body: &str) -> String {
        Self::get_plain_text(body)
    }

    /// 遍历（流式解析）文档抽取纯文本。
    ///
    /// 对应 Java: `private static String getPlainText(Document document)`
    fn get_plain_text(source: &str) -> String {
        let mut visitor = FormattingVisitor::new();
        tokenize(source, &mut visitor);
        visitor.into_string()
    }
}

/// 对应 Java: `private static class HtmlHelper.FormattingVisitor`
struct FormattingVisitor {
    /// 对应 Java 字段 `private final StringBuilder m_buffer`
    buffer: String,
}

impl FormattingVisitor {
    /// 对应 Java: `public FormattingVisitor()`
    fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// 对应 Java: `@Override public void head(Node node, int depth)`
    ///
    /// 只在标签名命中 Java 的列表时向缓冲区追加内容。
    fn head_start_tag(&mut self, name: &str) {
        match name {
            "li" => self.append("\n * "),
            "dt" => self.append("  "),
            "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "tr" => self.append("\n"),
            _ => {}
        }
    }

    /// 对应 Java: `@Override public void tail(Node node, int depth)`
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

    /// 对应 Java: `head` 中对 `TextNode` 调用 `((TextNode) node).text()` 后追加。
    fn text(&mut self, text: &str) {
        let decoded = decode_entities(text);
        self.append(&decoded);
    }

    /// 对应 Java: `private void append(String text)`
    ///
    /// 避免出现连续空白：单个空格的文本节点在缓冲区为空、或以空格/换行结尾时被丢弃。
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

    /// 对应 Java: `@Override public String toString()`
    fn into_string(self) -> String {
        self.buffer
    }
}

/// DOM 树遍历中一个尚未闭合的元素。
struct OpenElement {
    /// 小写标签名。
    name: String,
    /// `a` 标签的 `href` 属性值（用于 `tail` 阶段输出）。
    href: Option<String>,
}

/// 对应 Java 中 `NodeTraversor.traverse(formatter, document)` 对节点树的深度优先遍历。
///
/// 这里改为对原始 HTML 的流式扫描：遇到开始标签进入 `head`，遇到结束标签（或输入结束）
/// 按「由内向外」的顺序补出 `tail`。
fn tokenize(source: &str, visitor: &mut FormattingVisitor) {
    let mut rest = source;
    let mut stack: Vec<OpenElement> = Vec::new();

    while let Some(tag_start) = rest.find('<') {
        visitor.text(&rest[..tag_start]);
        rest = &rest[tag_start..];

        // 注释：跳过整个注释，jsoup 中它是 Comment 节点，不产生文本。
        if let Some(after_open) = rest.strip_prefix("<!--") {
            match after_open.find("-->") {
                Some(index) => {
                    rest = &after_open[index + 3..];
                    continue;
                }
                None => break,
            }
        }

        // 文档类型声明 / 其它 <! ... >：跳过。
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

        // 未闭合的 '<' 之后没有 '>'：按普通文本处理。
        let Some(tag_end) = rest.find('>') else {
            visitor.text(rest);
            rest = "";
            break;
        };

        let tag_content = &rest[1..tag_end];
        rest = &rest[tag_end + 1..];

        if let Some(name) = tag_content.strip_prefix('/') {
            // 结束标签
            let name = name.trim().to_ascii_lowercase();
            close_element(&name, &mut stack, visitor);
            continue;
        }

        let self_closing = tag_content.ends_with('/');
        let name = tag_name(tag_content);
        if name.is_empty() {
            continue;
        }

        // script / style 的内容在 jsoup 中是 DataNode（`nodeName()` 为 `#data`），
        // 不会当作 TextNode 追加；这里整段跳过到对应的结束标签。
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

    // 标签之后剩余的尾文本。
    if !rest.is_empty() {
        visitor.text(rest);
    }

    // 输入结束后，jsoup 会隐式闭合仍在栈中的元素，遍历同样会补出它们的 tail。
    while let Some(element) = stack.pop() {
        visitor.tail_end_tag(&element.name, element.href.as_deref());
    }
}

/// 跳过 `script` / `style` 元素的原始内容，返回闭合标签之后的位置。
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

/// 关闭指定名称的元素：弹出并补出 tail 的元素直到（含）该元素。
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

/// 从标签内容中取出小写标签名（不含属性与自闭合斜杠）。
fn tag_name(tag_content: &str) -> String {
    tag_content
        .trim_start()
        .split(|c: char| c.is_whitespace() || c == '/')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// 取出 `a` 标签的 `href` 属性值（jsoup 的 `absUrl("href")` 在无 base URI 时返回属性原值）。
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

/// HTML 的空元素（jsoup 中无需闭合，`head` 之后立即 `tail`）。
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

/// 解码文本节点中的字符实体（jsoup 在解析阶段完成解码）。
///
/// 覆盖数字实体与常见命名实体；未覆盖的命名实体原样保留。
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

/// 尝试解码开头的实体，返回（字符, 消耗的字节数）。
fn decode_entity_at(candidate: &str) -> Option<(char, usize)> {
    let body = candidate.strip_prefix('&')?;
    let semicolon = body.find(';')?;
    // jsoup 允许未以 ';' 结尾的实体，但命名实体长度有限；这里限制搜索范围。
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

/// 常见命名字符实体表（小写）。
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
        // 对应 Java FormattingVisitor：<br> 的 tail 追加 "\n"
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
        // 文本 " " 在缓冲区末尾已有空格/换行时被丢弃
        assert_eq!(
            HtmlHelper::strip(Some("<body>a <b> </b>c</body>")),
            Some("a c".to_owned())
        );
    }
}
