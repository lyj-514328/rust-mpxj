//! Corresponds to Java: `org/mpxj/common/ReaderTokenizer.java`
//!
//! Semantics match the Java version: **read by character** (decode the byte stream first, then
//! tokenize by character), with block-wise read-ahead (the Java comments note that
//! `Reader.read()` goes through `StreamDecoder` on every call, takes a lock, and allocates per
//! character, so it reads in blocks). Read-ahead means the Reader runs ahead of the current
//! token and cannot be shared with other readers afterwards — the Rust side keeps the same
//! constraint.
//!
//! Porting trade-offs:
//! - Java overrides `protected int read()` by inheriting `Tokenizer`; Rust has no inheritance,
//!   so this type **composes** a `Tokenizer<CharSource<R>>` and forwards its public methods
//!   (the base-class logic and state are fully reused, see the comments on each forwarding
//!   method).
//! - Java's `ReaderTokenizer(Reader r)` receives an "already-decoded character stream"; Rust's
//!   `std::io::Read` only provides bytes, so the decoding responsibility of Java's
//!   `new InputStreamReader(is, charset)` is folded into this type:
//!   [`ReaderTokenizer::with_encoding`] specifies the charset, defaulting to UTF-8.
//! - Java's `char[] m_readBuffer` + `m_index` + `m_length` (read buffer + position pointer) are
//!   kept as-is; this is how Java implements pushback/read-ahead, and the Rust side additionally
//!   uses `pending` to hold an incomplete multi-byte sequence spanning blocks (in Java that
//!   buffer lives inside `StreamDecoder`).
//! - A Java `char` is a UTF-16 code unit, while a Rust `char` is a Unicode scalar value:
//!   supplementary-plane characters are surrogate pairs in Java and a single `char` in Rust
//!   (the concatenated text is the same).
//! - `java.io.IOException` → `crate::MpxjError`; `PushbackReader`-style pushback is handled by
//!   the `next_character` / `next_token` fields inside [`Tokenizer`].
//!
//! Unported methods: none.

use super::tokenizer::{TT_EOF, TokenSource, Tokenizer};
use crate::MpxjResult;
use encoding_rs::{Decoder, Encoding};
use std::io::Read;

/// Corresponds to Java: `private static final int BUFFER_SIZE = 8192`
///
/// In Java this is the number of `char[]` elements; the Rust side uses it as the number of bytes
/// requested from `Read` each time.
const BUFFER_SIZE: usize = 8192;

/// A character-reading character source.
///
/// Corresponds to Java: the `protected int read()` overridden in `ReaderTokenizer` (and the
/// read-ahead of `Reader.read(char[])`).
pub struct CharSource<R: Read> {
    /// Byte data source, corresponding to Java's `Reader` (already decoded by `InputStreamReader` in Java)
    reader: R,
    /// Incremental decoder, taking on the decoding role of Java's `InputStreamReader` + `StreamDecoder`
    decoder: Decoder,
    /// Corresponds to Java: `private final char[] m_readBuffer = new char[BUFFER_SIZE]`
    read_buffer: Vec<char>,
    /// Corresponds to Java: `private int m_index`
    index: usize,
    /// Corresponds to Java: `private int m_length`
    length: usize,
    /// An incomplete multi-byte sequence spanning blocks (in Java held internally by `StreamDecoder`)
    pending: Vec<u8>,
    /// Whether the data source has reached the end, corresponding to Java's `m_length < 1`
    eof: bool,
}

impl<R: Read> CharSource<R> {
    /// Construct a character source with UTF-8 decoding.
    ///
    /// Corresponds to Java: `new InputStreamReader(is)` (Java uses the platform default charset;
    /// the Rust side standardizes on UTF-8).
    pub fn new(reader: R) -> Self {
        Self::with_encoding(reader, encoding_rs::UTF_8)
    }

    /// Construct a character source with the specified charset decoding.
    ///
    /// Corresponds to Java: `new InputStreamReader(is, charset)`
    pub fn with_encoding(reader: R, encoding: &'static Encoding) -> Self {
        Self {
            reader,
            // Java's InputStreamReader does not strip the BOM; BOM handling is likewise disabled here
            decoder: encoding.new_decoder_without_bom_handling(),
            read_buffer: Vec::new(),
            index: 0,
            length: 0,
            pending: Vec::new(),
            eof: false,
        }
    }

    /// Take back the underlying byte data source.
    ///
    /// Corresponds to Java: no such method (Java's `Reader`/`InputStream` is held by the caller).
    ///
    /// Note: this type has read-ahead, so on "take back" the data source position may already be
    /// past the last returned character.
    pub fn into_inner(self) -> R {
        self.reader
    }

    /// Read the next character; returns `None` at the end of the data source.
    ///
    /// Corresponds to Java: `protected int read() throws IOException` (Java returns the character
    /// code and `TT_EOF` at the end; the Rust side expresses the same semantics with
    /// `Option<char>`).
    pub fn read_char(&mut self) -> MpxjResult<Option<char>> {
        if self.index == self.length {
            self.fill_buffer()?;
        }

        match self.read_buffer.get(self.index).copied() {
            Some(c) => {
                self.index += 1;
                Ok(Some(c))
            }
            // Corresponds to Java: `if (m_length < 1) { m_length = 0; return TT_EOF; }`
            None => Ok(None),
        }
    }

    /// Fill the read buffer: corresponds to the call to `m_reader.read(m_readBuffer)` in Java's
    /// `read()`, and the reset of the position pointer `m_index = 0`.
    fn fill_buffer(&mut self) -> MpxjResult<()> {
        self.index = 0;
        self.length = 0;
        self.read_buffer.clear();

        while self.read_buffer.is_empty() {
            if self.eof && self.pending.is_empty() {
                break;
            }

            let mut bytes = [0u8; BUFFER_SIZE];
            let count = if self.eof {
                0
            } else {
                self.reader.read(&mut bytes)?
            };
            if count == 0 {
                self.eof = true;
            }

            // The incomplete sequence left unconsumed from the previous round is prepended to this round's data
            let mut input = std::mem::take(&mut self.pending);
            input.extend_from_slice(&bytes[..count]);

            let mut decoded = String::new();
            decoded.reserve(
                self.decoder
                    .max_utf8_buffer_length(input.len())
                    .unwrap_or(input.len().saturating_mul(4)),
            );
            let (_result, read, _replaced) =
                self.decoder
                    .decode_to_string(&input, &mut decoded, self.eof);
            if read < input.len() {
                self.pending.extend_from_slice(&input[read..]);
            }

            self.read_buffer.extend(decoded.chars());
        }

        self.length = self.read_buffer.len();
        Ok(())
    }
}

impl<R: Read> TokenSource for CharSource<R> {
    /// Corresponds to Java: the `protected int read()` overridden in `ReaderTokenizer`
    fn read(&mut self) -> MpxjResult<i32> {
        match self.read_char()? {
            Some(c) => Ok(c as u32 as i32),
            None => Ok(TT_EOF),
        }
    }
}

/// A tokenizer that reads by character from a `Read` and tokenizes.
///
/// Corresponds to Java: `public class ReaderTokenizer extends Tokenizer`
pub struct ReaderTokenizer<R: Read> {
    /// Composition + forwarding: in Java this is inheriting `Tokenizer` and overriding `read()`
    inner: Tokenizer<CharSource<R>>,
}

impl<R: Read> ReaderTokenizer<R> {
    /// Construct a tokenizer; the input is decoded as UTF-8.
    ///
    /// Corresponds to Java: `public ReaderTokenizer(Reader r)`
    pub fn new(reader: R) -> Self {
        Self {
            inner: Tokenizer::new(CharSource::new(reader)),
        }
    }

    /// Construct a tokenizer; the input is decoded with the specified charset.
    ///
    /// Corresponds to Java: `new ReaderTokenizer(new InputStreamReader(is, charset))`
    pub fn with_encoding(reader: R, encoding: &'static Encoding) -> Self {
        Self {
            inner: Tokenizer::new(CharSource::with_encoding(reader, encoding)),
        }
    }

    /// Take back the underlying data source.
    ///
    /// Corresponds to Java: no such method (in Java the `Reader` is held by the caller); the Rust
    /// side uses it for the hand-off of "probe the file header by byte first, then decode by
    /// character".
    pub fn into_inner(self) -> R {
        self.inner.into_source().into_inner()
    }

    /// Read the next character; returns `None` at the end of the data source.
    ///
    /// Corresponds to Java: `protected int read()` (protected in Java, available only to subclasses)
    pub fn read_char(&mut self) -> MpxjResult<Option<char>> {
        self.inner.source_mut().read_char()
    }

    /// Corresponds to Java: `public int nextToken() throws IOException`
    pub fn next_token(&mut self) -> MpxjResult<i32> {
        self.inner.next_token()
    }

    /// Corresponds to Java: `public String getToken()`
    pub fn get_token(&self) -> &str {
        self.inner.get_token()
    }

    /// Corresponds to Java: `public int getType()`
    pub fn get_type(&self) -> i32 {
        self.inner.get_type()
    }

    /// Corresponds to Java: `public void setDelimiter(char delimiter)`
    pub fn set_delimiter(&mut self, delimiter: char) {
        self.inner.set_delimiter(delimiter);
    }

    /// Corresponds to Java: no such method (`m_delimiter` is a private field)
    pub fn get_delimiter(&self) -> char {
        self.inner.get_delimiter()
    }

    /// Corresponds to Java: `public static final int TT_EOL = '\n'`
    pub const TT_EOL: i32 = super::tokenizer::TT_EOL;

    /// Corresponds to Java: `public static final int TT_EOF = -1`
    pub const TT_EOF: i32 = super::tokenizer::TT_EOF;

    /// Corresponds to Java: `public static final int TT_WORD = -3`
    pub const TT_WORD: i32 = super::tokenizer::TT_WORD;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::tokenizer::{TT_EOL, TT_WORD};
    use std::io::Cursor;

    fn source(text: &str) -> ReaderTokenizer<Cursor<Vec<u8>>> {
        ReaderTokenizer::new(Cursor::new(text.as_bytes().to_vec()))
    }

    /// Read all tokens and return a list of (type, text).
    fn read_all(tk: &mut ReaderTokenizer<Cursor<Vec<u8>>>) -> Vec<(i32, String)> {
        let mut result = Vec::new();
        loop {
            let token_type = tk.next_token().expect("in-memory data source never fails");
            result.push((token_type, tk.get_token().to_string()));
            if token_type == TT_EOF {
                break;
            }
        }
        result
    }

    #[test]
    fn reads_utf8_characters() {
        let mut tk = source("héllo,wörld\n");
        let tokens = read_all(&mut tk);
        assert_eq!(
            tokens,
            vec![
                (TT_WORD, "héllo".to_string()),
                (TT_WORD, "wörld".to_string()),
                (TT_EOL, String::new()),
                (TT_EOF, String::new()),
            ]
        );
    }

    #[test]
    fn crlf_returns_word_then_eol() {
        let mut tk = source("a\r\nb\r\n");
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
    fn quoted_field_may_contain_delimiter() {
        let mut tk = source("\"a,b\",c\n");
        let tokens = read_all(&mut tk);
        assert_eq!(tokens[0], (TT_WORD, "a,b".to_string()));
        assert_eq!(tokens[1], (TT_WORD, "c".to_string()));
    }

    #[test]
    fn custom_delimiter_is_forwarded() {
        let mut tk = source("a;b\n");
        tk.set_delimiter(';');
        assert_eq!(tk.get_delimiter(), ';');
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "a");
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "b");
        assert_eq!(tk.get_type(), TT_WORD);
    }

    #[test]
    fn adjacent_delimiters_produce_empty_fields() {
        let mut tk = source("1,,3\n");
        let tokens = read_all(&mut tk);
        assert_eq!(tokens[1], (TT_WORD, String::new()));
        assert_eq!(tokens[2], (TT_WORD, "3".to_string()));
    }

    #[test]
    fn empty_input_returns_eof() {
        let mut tk = source("");
        assert_eq!(tk.next_token().unwrap(), TT_EOF);
        assert_eq!(tk.read_char().unwrap(), None);
    }

    #[test]
    fn read_char_returns_characters_then_none() {
        let mut tk = source("ab");
        assert_eq!(tk.read_char().unwrap(), Some('a'));
        assert_eq!(tk.read_char().unwrap(), Some('b'));
        assert_eq!(tk.read_char().unwrap(), None);
    }

    #[test]
    fn long_field_crosses_the_read_buffer_boundary() {
        let mut text = "a".repeat(20_000);
        text.push_str(",tail\n");
        let mut tk = source(&text);
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token().len(), 20_000);
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "tail");
    }

    #[test]
    fn multi_byte_character_split_across_blocks_is_reassembled() {
        // 8191 ASCII characters + one 3-byte UTF-8 character, exactly crossing the 8192-byte read buffer boundary
        let mut text = "a".repeat(8191);
        text.push('中');
        text.push_str(",x\n");

        let mut tk = source(&text);
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token().chars().count(), 8192);
        assert!(tk.get_token().ends_with('中'));
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "x");
    }

    #[test]
    fn decodes_with_explicit_encoding() {
        // GBK-encoded "中文"
        let bytes = vec![0xD6, 0xD0, 0xCE, 0xC4];
        let mut tk = ReaderTokenizer::with_encoding(Cursor::new(bytes), encoding_rs::GBK);
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "中文");

        // Windows-1252-encoded "é"
        let mut tk = ReaderTokenizer::with_encoding(Cursor::new(vec![0xE9]), encoding_rs::WINDOWS_1252);
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "é");
    }

    #[test]
    fn invalid_bytes_become_replacement_characters() {
        // A lone high byte is invalid in UTF-8
        let mut tk = ReaderTokenizer::new(Cursor::new(vec![0xFF, b'a']));
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "\u{FFFD}a");
    }

    #[test]
    fn into_inner_returns_the_underlying_reader() {
        let mut tk = source("a,b\n");
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "a");
        // Read-ahead advances the data source to the end (Java has the same "Reader is read ahead" constraint)
        let cursor = tk.into_inner();
        assert_eq!(cursor.position(), 4);
    }

    #[test]
    fn constants_match_java() {
        assert_eq!(ReaderTokenizer::<Cursor<Vec<u8>>>::TT_EOL, '\n' as i32);
        assert_eq!(ReaderTokenizer::<Cursor<Vec<u8>>>::TT_EOF, -1);
        assert_eq!(ReaderTokenizer::<Cursor<Vec<u8>>>::TT_WORD, -3);
    }
}
