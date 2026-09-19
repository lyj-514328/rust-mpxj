//! 对应 Java: `org/mpxj/common/Tokenizer.java`
//!
//! 本类实现一个松散的 `java.io.StreamTokenizer` 替代品，专门用于正确解析 MPX 记录，
//! 尤其是由相邻分隔符表示的空字段。
//!
//! 移植取舍：
//! - Java 的 `Tokenizer` 是抽象类，由子类覆写 `protected abstract int read()` 提供字符来源
//!   （见 `ReaderTokenizer` / `InputStreamTokenizer`）。Rust 无继承，这里把「字符来源」抽成
//!   [`TokenSource`] trait，`Tokenizer` 通过泛型参数 `S` 组合并调用它；默认参数
//!   `Box<dyn TokenSource>` 用于复现 Java 中 `Tokenizer tk` 变量可指向任意子类的用法。
//! - `java.io.IOException` → [`crate::MpxjError`]（已实现 `From<std::io::Error>`），
//!   所有方法返回 [`crate::MpxjResult`]，调用方可用 `?` 直接传播。
//! - Java 的 `int m_nextCharacter`（单字符回退）与 `int m_nextToken`（单 token 回退）原样保留，
//!   这是 Java 侧回退语义的实现方式，Rust 侧不需要 `PushbackReader`。
//! - `StringBuilder m_buffer` → `String`；Java 的 `String getToken()` 返回新建 `String`，
//!   Rust 侧返回缓冲区借用 `&str`。
//! - `protected boolean startQuotedIsValid(StringBuilder)` 在 Rust 中没有 protected，
//!   改为 `pub`（原版中无子类覆写它）。
//!
//! 未移植的方法：无。

use crate::MpxjResult;

/// 对应 Java: `public static final int TT_EOL = '\n'`
pub const TT_EOL: i32 = '\n' as i32;

/// 对应 Java: `public static final int TT_EOF = -1`
pub const TT_EOF: i32 = -1;

/// 对应 Java: `public static final int TT_WORD = -3`
pub const TT_WORD: i32 = -3;

/// 字符来源，对应 Java: `protected abstract int read() throws IOException`。
///
/// Java 侧由 `ReaderTokenizer`（按字符读）与 `InputStreamTokenizer`（按字节读）覆写该方法；
/// Rust 侧由 [`super::reader_tokenizer::CharSource`] 与
/// [`super::input_stream_tokenizer::ByteSource`] 实现，再由各自的包装类型组合进 [`Tokenizer`]。
pub trait TokenSource {
    /// 读取下一个字符编码；数据源结束返回 [`TT_EOF`]（Java 中同为 -1）。
    ///
    /// 对应 Java: `protected abstract int read() throws IOException`
    fn read(&mut self) -> MpxjResult<i32>;
}

/// 让 trait object 本身也可作为 [`Tokenizer`] 的数据源，
/// 对应 Java 中 `Tokenizer tk` 变量指向任意子类实例的用法。
///
/// 对应 Java: 继承体系（`ReaderTokenizer`/`InputStreamTokenizer` → `Tokenizer`）
impl TokenSource for Box<dyn TokenSource> {
    fn read(&mut self) -> MpxjResult<i32> {
        (**self).read()
    }
}

/// 松散基于 `java.io.StreamTokenizer` 的 MPX 记录分词器。
///
/// 对应 Java: `public abstract class Tokenizer`
pub struct Tokenizer<S: TokenSource = Box<dyn TokenSource>> {
    /// 字符来源，对应 Java 中由子类覆写的 `read()`
    source: S,
    /// 对应 Java: `private char m_delimiter = ','`
    delimiter: char,
    /// 对应 Java: `private int m_nextToken`（单 token 回退）
    next_token: i32,
    /// 对应 Java: `private int m_nextCharacter = -1`（单字符回退）
    next_character: i32,
    /// 对应 Java: `private int m_type`
    type_: i32,
    /// 对应 Java: `private final StringBuilder m_buffer`
    buffer: String,
}

impl<S: TokenSource> Tokenizer<S> {
    /// 对应 Java: `public static final int TT_EOL = '\n'`
    pub const TT_EOL: i32 = crate::common::tokenizer::TT_EOL;

    /// 对应 Java: `public static final int TT_EOF = -1`
    pub const TT_EOF: i32 = crate::common::tokenizer::TT_EOF;

    /// 对应 Java: `public static final int TT_WORD = -3`
    pub const TT_WORD: i32 = crate::common::tokenizer::TT_WORD;

    /// 构造分词器。
    ///
    /// 对应 Java: `Tokenizer` 的隐式无参构造方法（Java 侧 `read()` 由子类覆写提供，
    /// Rust 侧改为构造时注入 [`TokenSource`]）。
    pub fn new(source: S) -> Self {
        Self {
            source,
            delimiter: ',',
            next_token: 0,
            next_character: -1,
            type_: 0,
            buffer: String::new(),
        }
    }

    /// 取得下一个 token，返回值代表 token 类型。
    ///
    /// 对应 Java: `public int nextToken() throws IOException`
    pub fn next_token(&mut self) -> MpxjResult<i32> {
        let mut quoted = false;
        let mut result = self.next_token;

        if self.next_token != 0 {
            self.next_token = 0;
        }

        self.buffer.clear();

        while result == 0 {
            // 对应 Java: 优先消费 `m_nextCharacter`（单字符回退），否则向数据源读取
            let c = if self.next_character != -1 {
                let c = self.next_character;
                self.next_character = -1;
                c
            } else {
                self.source.read()?
            };

            if c == Self::TT_EOF {
                // 对应 Java: `case TT_EOF:`
                if !self.buffer.is_empty() {
                    result = Self::TT_WORD;
                    self.next_token = Self::TT_EOF;
                } else {
                    result = Self::TT_EOF;
                }
            } else if c == '\r' as i32 {
                // 对应 Java: `case '\r':`——吃掉紧随的 LF，保证 CRLF 只产生一个 EOL
                self.next_character = self.source.read()?;
                if self.next_character == '\n' as i32 {
                    self.next_character = -1;
                }

                if self.buffer.is_empty() {
                    result = Self::TT_EOL;
                } else {
                    result = Self::TT_WORD;
                    self.next_token = Self::TT_EOL;
                }
            } else if c == '\n' as i32 {
                // 对应 Java: `case '\n':`
                if self.buffer.is_empty() {
                    result = Self::TT_EOL;
                } else {
                    result = Self::TT_WORD;
                    self.next_token = Self::TT_EOL;
                }
            } else if c == '"' as i32 {
                // 对应 Java: 引号处理分支（`char quote = '"'`）
                if !quoted && self.start_quoted_is_valid(&self.buffer) {
                    quoted = true;
                } else if !quoted {
                    append_code(&mut self.buffer, c);
                } else {
                    self.next_character = self.source.read()?;
                    if self.next_character == '"' as i32 {
                        // 引号内的双引号 `""` 表示一个字面引号
                        append_code(&mut self.buffer, c);
                        self.next_character = -1;
                    } else {
                        quoted = false;
                    }
                }
            } else if c == self.delimiter as i32 && !quoted {
                // 空白未跳过、相邻分隔符产生空字段
                result = Self::TT_WORD;
            } else {
                append_code(&mut self.buffer, c);
            }
        }

        self.type_ = result;

        Ok(result)
    }

    /// 控制引号语义：默认只有字段首字符处的引号才视为引号起始。
    ///
    /// 对应 Java: `protected boolean startQuotedIsValid(StringBuilder buffer)`
    pub fn start_quoted_is_valid(&self, buffer: &str) -> bool {
        buffer.is_empty()
    }

    /// 取得最后一个 token 的文本。
    ///
    /// 对应 Java: `public String getToken()`
    ///
    /// Java 返回新建的 `String`，Rust 侧返回内部缓冲区借用。
    pub fn get_token(&self) -> &str {
        &self.buffer
    }

    /// 取得最后一个 token 的类型。
    ///
    /// 对应 Java: `public int getType()`
    pub fn get_type(&self) -> i32 {
        self.type_
    }

    /// 设置分词器识别的分隔符。
    ///
    /// 对应 Java: `public void setDelimiter(char delimiter)`
    pub fn set_delimiter(&mut self, delimiter: char) {
        self.delimiter = delimiter;
    }

    /// 取得当前分隔符。
    ///
    /// 对应 Java: 无此方法（Java 的 `m_delimiter` 为私有字段）；Rust 侧供包装类型转发与测试使用。
    pub fn get_delimiter(&self) -> char {
        self.delimiter
    }

    /// 取得字符来源的可变引用。
    ///
    /// 对应 Java: 无此方法（Java 的子类直接访问自身字段）；Rust 侧供包装类型转发字节/字符级读取。
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    /// 取回字符来源。
    ///
    /// 对应 Java: 无此方法（Java 中 `InputStream`/`Reader` 由调用方持有）；
    /// Rust 侧用归还所有权的办法复现「同一个流先按字节探测、再按字符解码」的用法。
    pub fn into_source(self) -> S {
        self.source
    }
}

impl Tokenizer<Box<dyn TokenSource>> {
    /// 按字节读取的字符来源构造分词器。
    ///
    /// 对应 Java: `Tokenizer tk = new InputStreamTokenizer(bis)`
    pub fn from_bytes<R: std::io::Read + 'static>(stream: R) -> Self {
        Self::new(Box::new(super::input_stream_tokenizer::ByteSource::new(stream)))
    }

    /// 按字符读取的字符来源构造分词器（UTF-8 解码）。
    ///
    /// 对应 Java: `Tokenizer tk = new ReaderTokenizer(reader)`
    pub fn from_chars<R: std::io::Read + 'static>(reader: R) -> Self {
        Self::new(Box::new(super::reader_tokenizer::CharSource::new(reader)))
    }

    /// 按字符读取的字符来源构造分词器，指定解码字符集。
    ///
    /// 对应 Java: `new ReaderTokenizer(new InputStreamReader(is, charset))`
    pub fn from_chars_with_encoding<R: std::io::Read + 'static>(
        reader: R,
        encoding: &'static encoding_rs::Encoding,
    ) -> Self {
        Self::new(Box::new(super::reader_tokenizer::CharSource::with_encoding(
            reader, encoding,
        )))
    }
}

/// 对应 Java: `m_buffer.append((char) c)`——把读取到的字符编码写入缓冲区。
///
/// Java 的 `(char) c` 会把 `int` 截断为 16 位并允许孤立代理项；Rust 的 `char` 不允许，
/// 因此非法码点（孤立代理项）回落为 U+FFFD。字节与字符来源产生的编码都在合法范围内。
fn append_code(buffer: &mut String, c: i32) {
    buffer.push(char::from_u32(c as u32).unwrap_or(char::REPLACEMENT_CHARACTER));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 以字符串作为数据源的测试实现，对应 Java 中覆写 `read()` 的测试子类。
    struct StringSource {
        codes: Vec<i32>,
        index: usize,
    }

    impl StringSource {
        fn new(text: &str) -> Self {
            Self {
                codes: text.chars().map(|c| c as u32 as i32).collect(),
                index: 0,
            }
        }
    }

    impl TokenSource for StringSource {
        fn read(&mut self) -> MpxjResult<i32> {
            match self.codes.get(self.index) {
                Some(code) => {
                    self.index += 1;
                    Ok(*code)
                }
                None => Ok(TT_EOF),
            }
        }
    }

    fn tokenizer(text: &str) -> Tokenizer<StringSource> {
        Tokenizer::new(StringSource::new(text))
    }

    /// 读取全部 token，返回 (类型, 文本) 列表，末尾一定包含 TT_EOF。
    fn read_all(tk: &mut Tokenizer<StringSource>) -> Vec<(i32, String)> {
        let mut result = Vec::new();
        loop {
            let token_type = tk.next_token().expect("测试数据源不会失败");
            result.push((token_type, tk.get_token().to_string()));
            if token_type == TT_EOF {
                break;
            }
        }
        result
    }

    #[test]
    fn constants_match_java() {
        assert_eq!(TT_EOL, '\n' as i32);
        assert_eq!(TT_EOF, -1);
        assert_eq!(TT_WORD, -3);
        assert_eq!(Tokenizer::<StringSource>::TT_EOL, TT_EOL);
        assert_eq!(Tokenizer::<StringSource>::TT_EOF, TT_EOF);
        assert_eq!(Tokenizer::<StringSource>::TT_WORD, TT_WORD);
    }

    #[test]
    fn splits_fields_on_default_delimiter() {
        let mut tk = tokenizer("1,2,3\n");
        let tokens = read_all(&mut tk);
        assert_eq!(
            tokens,
            vec![
                (TT_WORD, "1".to_string()),
                (TT_WORD, "2".to_string()),
                (TT_WORD, "3".to_string()),
                (TT_EOL, String::new()),
                (TT_EOF, String::new()),
            ]
        );
    }

    #[test]
    fn adjacent_delimiters_produce_empty_fields() {
        let mut tk = tokenizer("1,,3\n");
        let tokens = read_all(&mut tk);
        assert_eq!(
            tokens,
            vec![
                (TT_WORD, "1".to_string()),
                (TT_WORD, String::new()),
                (TT_WORD, "3".to_string()),
                (TT_EOL, String::new()),
                (TT_EOF, String::new()),
            ]
        );
    }

    #[test]
    fn custom_delimiter_is_recognised() {
        let mut tk = tokenizer("a\tb;c\n");
        tk.set_delimiter(';');
        assert_eq!(tk.get_delimiter(), ';');
        let tokens = read_all(&mut tk);
        assert_eq!(tokens[0], (TT_WORD, "a\tb".to_string()));
        assert_eq!(tokens[1], (TT_WORD, "c".to_string()));
    }

    #[test]
    fn whitespace_is_not_skipped() {
        let mut tk = tokenizer(" a b ,c\n");
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), " a b ");
    }

    #[test]
    fn quoted_field_may_contain_delimiter() {
        let mut tk = tokenizer("\"a,b\",c\n");
        let tokens = read_all(&mut tk);
        assert_eq!(tokens[0], (TT_WORD, "a,b".to_string()));
        assert_eq!(tokens[1], (TT_WORD, "c".to_string()));
        assert_eq!(tokens[2], (TT_EOL, String::new()));
    }

    #[test]
    fn doubled_quote_inside_quoted_field_is_literal() {
        let mut tk = tokenizer("\"a\"\"b\",c\n");
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "a\"b");
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "c");
    }

    #[test]
    fn unclosed_quote_keeps_delimiter_as_content() {
        let mut tk = tokenizer("\"a,b\n");
        let tokens = read_all(&mut tk);
        // 引号未闭合：分隔符被当作字段内容（与 Java 的 `c == m_delimiter && !quoted` 判断一致），
        // 但换行分支不检查 quoted，仍然结束当前 token
        assert_eq!(
            tokens,
            vec![
                (TT_WORD, "a,b".to_string()),
                (TT_EOL, String::new()),
                (TT_EOF, String::new()),
            ]
        );
    }

    #[test]
    fn quote_outside_first_position_is_literal() {
        let mut tk = tokenizer("a\"b,c\n");
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "a\"b");
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "c");
    }

    #[test]
    fn start_quoted_is_valid_only_at_buffer_start() {
        let tk = tokenizer("");
        assert!(tk.start_quoted_is_valid(""));
        assert!(!tk.start_quoted_is_valid("a"));
    }

    #[test]
    fn crlf_returns_word_then_eol() {
        let mut tk = tokenizer("a\r\nb\r\n");
        let tokens = read_all(&mut tk);
        assert_eq!(
            tokens,
            vec![
                (TT_WORD, "a".to_string()),
                (TT_EOL, String::new()),
                (TT_WORD, "b".to_string()),
                (TT_EOL, String::new()),
                (TT_EOF, String::new()),
            ]
        );
    }

    #[test]
    fn lone_cr_returns_eol_and_pushes_back_next_character() {
        let mut tk = tokenizer("a\rb");
        let tokens = read_all(&mut tk);
        // `\r` 分支读到的 `b` 通过 m_nextCharacter 回退，下一次读取时优先消费
        assert_eq!(
            tokens,
            vec![
                (TT_WORD, "a".to_string()),
                (TT_EOL, String::new()),
                (TT_WORD, "b".to_string()),
                (TT_EOF, String::new()),
            ]
        );
    }

    #[test]
    fn empty_lines_produce_eol_tokens() {
        let mut tk = tokenizer("\n\n");
        let tokens = read_all(&mut tk);
        assert_eq!(
            tokens,
            vec![
                (TT_EOL, String::new()),
                (TT_EOL, String::new()),
                (TT_EOF, String::new()),
            ]
        );
    }

    #[test]
    fn last_word_before_eof_is_pushed_back_as_eof() {
        let mut tk = tokenizer("word");
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "word");
        assert_eq!(tk.get_type(), TT_WORD);
        assert_eq!(tk.next_token().unwrap(), TT_EOF);
        assert_eq!(tk.get_token(), "");
        assert_eq!(tk.get_type(), TT_EOF);
    }

    #[test]
    fn empty_input_returns_eof() {
        let mut tk = tokenizer("");
        assert_eq!(tk.next_token().unwrap(), TT_EOF);
        assert_eq!(tk.get_type(), TT_EOF);
        // 到达 EOF 后可以反复读取
        assert_eq!(tk.next_token().unwrap(), TT_EOF);
    }

    #[test]
    fn get_type_tracks_last_token() {
        let mut tk = tokenizer("a\n");
        assert_eq!(tk.get_type(), 0);
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_type(), TT_WORD);
        assert_eq!(tk.next_token().unwrap(), TT_EOL);
        assert_eq!(tk.get_type(), TT_EOL);
    }

    #[test]
    fn boxed_sources_can_be_used_as_java_like_variable() {
        // 对应 Java: `Tokenizer tk = new InputStreamTokenizer(is);`
        let mut tk: Tokenizer = Tokenizer::from_bytes(std::io::Cursor::new(b"a,b".to_vec()));
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "a");
    }
}
