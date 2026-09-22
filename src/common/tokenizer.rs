//! Corresponds to Java: `org/mpxj/common/Tokenizer.java`
//!
//! This class implements a loose replacement for `java.io.StreamTokenizer`, specifically for
//! correctly parsing MPX records, especially empty fields represented by adjacent delimiters.
//!
//! Porting trade-offs:
//! - Java's `Tokenizer` is an abstract class whose subclasses override
//!   `protected abstract int read()` to provide the character source (see `ReaderTokenizer` /
//!   `InputStreamTokenizer`). Rust has no inheritance, so the "character source" is extracted
//!   into the [`TokenSource`] trait, which `Tokenizer` composes and calls via the generic
//!   parameter `S`; the default parameter `Box<dyn TokenSource>` reproduces Java's use of a
//!   `Tokenizer tk` variable that can point to any subclass.
//! - `java.io.IOException` → [`crate::MpxjError`] (which already implements
//!   `From<std::io::Error>`); all methods return [`crate::MpxjResult`], so callers can
//!   propagate with `?` directly.
//! - Java's `int m_nextCharacter` (single-character pushback) and `int m_nextToken`
//!   (single-token pushback) are kept as-is; this is how Java implements pushback semantics,
//!   and the Rust side does not need a `PushbackReader`.
//! - `StringBuilder m_buffer` → `String`; Java's `String getToken()` returns a newly created
//!   `String`, while the Rust side returns a borrowed `&str` of the buffer.
//! - `protected boolean startQuotedIsValid(StringBuilder)` has no protected in Rust, so it
//!   becomes `pub` (no subclass overrides it in the original).
//!
//! Unported methods: none.

use crate::MpxjResult;

/// Corresponds to Java: `public static final int TT_EOL = '\n'`
pub const TT_EOL: i32 = '\n' as i32;

/// Corresponds to Java: `public static final int TT_EOF = -1`
pub const TT_EOF: i32 = -1;

/// Corresponds to Java: `public static final int TT_WORD = -3`
pub const TT_WORD: i32 = -3;

/// Character source, Corresponds to Java: `protected abstract int read() throws IOException`.
///
/// In Java this method is overridden by `ReaderTokenizer` (reads by character) and
/// `InputStreamTokenizer` (reads by byte); on the Rust side it is implemented by
/// [`super::reader_tokenizer::CharSource`] and
/// [`super::input_stream_tokenizer::ByteSource`], which are then composed into [`Tokenizer`]
/// by their respective wrapper types.
pub trait TokenSource {
    /// Read the next character code; returns [`TT_EOF`] at the end of the data source (also -1
    /// in Java).
    ///
    /// Corresponds to Java: `protected abstract int read() throws IOException`
    fn read(&mut self) -> MpxjResult<i32>;
}

/// Allow a trait object itself to serve as a data source for [`Tokenizer`],
/// corresponding to Java's use of a `Tokenizer tk` variable pointing to any subclass instance.
///
/// Corresponds to Java: the inheritance hierarchy (`ReaderTokenizer`/`InputStreamTokenizer` → `Tokenizer`)
impl TokenSource for Box<dyn TokenSource> {
    fn read(&mut self) -> MpxjResult<i32> {
        (**self).read()
    }
}

/// An MPX record tokenizer loosely based on `java.io.StreamTokenizer`.
///
/// Corresponds to Java: `public abstract class Tokenizer`
pub struct Tokenizer<S: TokenSource = Box<dyn TokenSource>> {
    /// Character source, corresponding to Java's `read()` overridden by subclasses
    source: S,
    /// Corresponds to Java: `private char m_delimiter = ','`
    delimiter: char,
    /// Corresponds to Java: `private int m_nextToken` (single-token pushback)
    next_token: i32,
    /// Corresponds to Java: `private int m_nextCharacter = -1` (single-character pushback)
    next_character: i32,
    /// Corresponds to Java: `private int m_type`
    type_: i32,
    /// Corresponds to Java: `private final StringBuilder m_buffer`
    buffer: String,
}

impl<S: TokenSource> Tokenizer<S> {
    /// Corresponds to Java: `public static final int TT_EOL = '\n'`
    pub const TT_EOL: i32 = crate::common::tokenizer::TT_EOL;

    /// Corresponds to Java: `public static final int TT_EOF = -1`
    pub const TT_EOF: i32 = crate::common::tokenizer::TT_EOF;

    /// Corresponds to Java: `public static final int TT_WORD = -3`
    pub const TT_WORD: i32 = crate::common::tokenizer::TT_WORD;

    /// Construct a tokenizer.
    ///
    /// Corresponds to Java: the implicit no-argument constructor of `Tokenizer` (in Java
    /// `read()` is provided by a subclass override; the Rust side injects a [`TokenSource`] at
    /// construction time).
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

    /// Get the next token; the return value represents the token type.
    ///
    /// Corresponds to Java: `public int nextToken() throws IOException`
    pub fn next_token(&mut self) -> MpxjResult<i32> {
        let mut quoted = false;
        let mut result = self.next_token;

        if self.next_token != 0 {
            self.next_token = 0;
        }

        self.buffer.clear();

        while result == 0 {
            // Corresponds to Java: consume `m_nextCharacter` first (single-character pushback), otherwise read from the data source
            let c = if self.next_character != -1 {
                let c = self.next_character;
                self.next_character = -1;
                c
            } else {
                self.source.read()?
            };

            if c == Self::TT_EOF {
                // Corresponds to Java: `case TT_EOF:`
                if !self.buffer.is_empty() {
                    result = Self::TT_WORD;
                    self.next_token = Self::TT_EOF;
                } else {
                    result = Self::TT_EOF;
                }
            } else if c == '\r' as i32 {
                // Corresponds to Java: `case '\r':`—consume the immediately following LF so CRLF produces only one EOL
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
                // Corresponds to Java: `case '\n':`
                if self.buffer.is_empty() {
                    result = Self::TT_EOL;
                } else {
                    result = Self::TT_WORD;
                    self.next_token = Self::TT_EOL;
                }
            } else if c == '"' as i32 {
                // Corresponds to Java: quote handling branch (`char quote = '"'`)
                if !quoted && self.start_quoted_is_valid(&self.buffer) {
                    quoted = true;
                } else if !quoted {
                    append_code(&mut self.buffer, c);
                } else {
                    self.next_character = self.source.read()?;
                    if self.next_character == '"' as i32 {
                        // A doubled quote `""` inside a quoted field represents a literal quote
                        append_code(&mut self.buffer, c);
                        self.next_character = -1;
                    } else {
                        quoted = false;
                    }
                }
            } else if c == self.delimiter as i32 && !quoted {
                // Whitespace is not skipped; adjacent delimiters produce empty fields
                result = Self::TT_WORD;
            } else {
                append_code(&mut self.buffer, c);
            }
        }

        self.type_ = result;

        Ok(result)
    }

    /// Control quote semantics: by default only a quote at the first character of a field is
    /// treated as the start of a quote.
    ///
    /// Corresponds to Java: `protected boolean startQuotedIsValid(StringBuilder buffer)`
    pub fn start_quoted_is_valid(&self, buffer: &str) -> bool {
        buffer.is_empty()
    }

    /// Get the text of the last token.
    ///
    /// Corresponds to Java: `public String getToken()`
    ///
    /// Java returns a newly created `String`; the Rust side returns a borrow of the internal buffer.
    pub fn get_token(&self) -> &str {
        &self.buffer
    }

    /// Get the type of the last token.
    ///
    /// Corresponds to Java: `public int getType()`
    pub fn get_type(&self) -> i32 {
        self.type_
    }

    /// Set the delimiter recognized by the tokenizer.
    ///
    /// Corresponds to Java: `public void setDelimiter(char delimiter)`
    pub fn set_delimiter(&mut self, delimiter: char) {
        self.delimiter = delimiter;
    }

    /// Get the current delimiter.
    ///
    /// Corresponds to Java: no such method (Java's `m_delimiter` is a private field); the Rust
    /// side provides it for wrapper-type forwarding and testing.
    pub fn get_delimiter(&self) -> char {
        self.delimiter
    }

    /// Get a mutable reference to the character source.
    ///
    /// Corresponds to Java: no such method (Java's subclasses access their own fields directly);
    /// the Rust side provides it so wrapper types can forward byte/character-level reads.
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    /// Take back the character source.
    ///
    /// Corresponds to Java: no such method (in Java the `InputStream`/`Reader` is held by the
    /// caller); the Rust side reproduces "probe the same stream by byte first, then decode by
    /// character" by returning ownership.
    pub fn into_source(self) -> S {
        self.source
    }
}

impl Tokenizer<Box<dyn TokenSource>> {
    /// Construct a tokenizer from a byte-reading character source.
    ///
    /// Corresponds to Java: `Tokenizer tk = new InputStreamTokenizer(bis)`
    pub fn from_bytes<R: std::io::Read + 'static>(stream: R) -> Self {
        Self::new(Box::new(super::input_stream_tokenizer::ByteSource::new(stream)))
    }

    /// Construct a tokenizer from a character-reading character source (UTF-8 decoding).
    ///
    /// Corresponds to Java: `Tokenizer tk = new ReaderTokenizer(reader)`
    pub fn from_chars<R: std::io::Read + 'static>(reader: R) -> Self {
        Self::new(Box::new(super::reader_tokenizer::CharSource::new(reader)))
    }

    /// Construct a tokenizer from a character-reading character source with the specified
    /// decoding charset.
    ///
    /// Corresponds to Java: `new ReaderTokenizer(new InputStreamReader(is, charset))`
    pub fn from_chars_with_encoding<R: std::io::Read + 'static>(
        reader: R,
        encoding: &'static encoding_rs::Encoding,
    ) -> Self {
        Self::new(Box::new(super::reader_tokenizer::CharSource::with_encoding(
            reader, encoding,
        )))
    }
}

/// Corresponds to Java: `m_buffer.append((char) c)`—write the read character code into the buffer.
///
/// Java's `(char) c` truncates the `int` to 16 bits and allows lone surrogates; Rust's `char`
/// does not, so an invalid code point (lone surrogate) falls back to U+FFFD. The codes produced
/// by the byte and character sources are all within the valid range.
fn append_code(buffer: &mut String, c: i32) {
    buffer.push(char::from_u32(c as u32).unwrap_or(char::REPLACEMENT_CHARACTER));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test implementation using a string as the data source, corresponding to a test
    /// subclass overriding `read()` in Java.
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

    /// Read all tokens and return a list of (type, text); always ends with TT_EOF.
    fn read_all(tk: &mut Tokenizer<StringSource>) -> Vec<(i32, String)> {
        let mut result = Vec::new();
        loop {
            let token_type = tk.next_token().expect("test data source should not fail");
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
        // Unclosed quote: the delimiter is treated as field content (consistent with Java's `c == m_delimiter && !quoted` check),
        // but the newline branch does not check quoted and still ends the current token
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
        // The `b` read by the `\r` branch is pushed back via m_nextCharacter and consumed first on the next read
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
        // After reaching EOF it can be read repeatedly
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
        // Corresponds to Java: `Tokenizer tk = new InputStreamTokenizer(is);`
        let mut tk: Tokenizer = Tokenizer::from_bytes(std::io::Cursor::new(b"a,b".to_vec()));
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "a");
    }
}
