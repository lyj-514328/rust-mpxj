//! Corresponds to Java: `org/mpxj/common/InputStreamTokenizer.java`
//!
//! Semantics match the Java version: **read by byte**; a high byte read is mapped by `(char) c`
//! (i.e., U+0000–U+00FF), with no charset decoding. MPX reading relies on exactly this to first
//! read the file header by byte (including the code page field) and then switch to a
//! character-decoding tokenizer (see `MPXReader`).
//!
//! Porting trade-offs:
//! - Java overrides `protected int read()` by inheriting `Tokenizer`; Rust has no inheritance,
//!   so this type **composes** a `Tokenizer<ByteSource<R>>` and forwards its public methods (the
//!   base-class logic and state are fully reused, see the comments on each forwarding method).
//! - Java's `InputStream.read()` reads only one byte at a time and does no buffering, so it does
//!   not swallow subsequent data; the Rust side likewise reads byte by byte
//!   (`ByteSource::read_byte`), ensuring the hand-off of "probe the same data source by byte
//!   first, then hand it to the character decoder" loses no bytes.
//! - The byte version additionally provides [`InputStreamTokenizer::read_byte`] (`u8`) and
//!   [`InputStreamTokenizer::get_token_bytes`] (`Vec<u8>`), corresponding to the raw byte view
//!   that `InputStream.read()` / `(char) c` cannot express in Java.
//! - `java.io.IOException` → `crate::MpxjError`; pushback (Java's `m_nextCharacter` /
//!   `m_nextToken`) is implemented by fields inside [`Tokenizer`].
//!
//! Unported methods: none.

use super::tokenizer::{TT_EOF, TokenSource, Tokenizer};
use crate::MpxjResult;
use std::io::Read;

/// A byte-reading character source.
///
/// Corresponds to Java: the `protected int read()` overridden in `InputStreamTokenizer`.
pub struct ByteSource<R: Read> {
    /// Byte data source, corresponding to Java's `InputStream m_stream`
    stream: R,
}

impl<R: Read> ByteSource<R> {
    /// Construct a byte source.
    ///
    /// Corresponds to Java: `public InputStreamTokenizer(InputStream is)`
    pub fn new(stream: R) -> Self {
        Self { stream }
    }

    /// Read the next byte; returns `None` at the end of the data source.
    ///
    /// Corresponds to Java: `m_stream.read()` (Java returns 0..=255 and -1 at the end of the data source).
    ///
    /// Reads byte by byte with no read-ahead: corresponding to the behavior of Java's
    /// `InputStream.read()`, ensuring the caller can still continue reading from the same data
    /// source afterwards.
    pub fn read_byte(&mut self) -> MpxjResult<Option<u8>> {
        let mut byte = [0u8; 1];
        match self.stream.read(&mut byte)? {
            0 => Ok(None),
            _ => Ok(Some(byte[0])),
        }
    }

    /// Take back the underlying byte data source.
    ///
    /// Corresponds to Java: no such method (in Java the `InputStream` is held by the caller).
    pub fn into_inner(self) -> R {
        self.stream
    }
}

impl<R: Read> TokenSource for ByteSource<R> {
    /// Corresponds to Java: the `protected int read()` overridden in `InputStreamTokenizer`
    fn read(&mut self) -> MpxjResult<i32> {
        match self.read_byte()? {
            // Corresponds to Java: `(char) c`—the byte value 0..=255 is used directly as the character code
            Some(byte) => Ok(i32::from(byte)),
            None => Ok(TT_EOF),
        }
    }
}

/// A tokenizer that reads by byte from a `Read` and tokenizes.
///
/// Corresponds to Java: `public class InputStreamTokenizer extends Tokenizer`
pub struct InputStreamTokenizer<R: Read> {
    /// Composition + forwarding: in Java this is inheriting `Tokenizer` and overriding `read()`
    inner: Tokenizer<ByteSource<R>>,
}

impl<R: Read> InputStreamTokenizer<R> {
    /// Construct a tokenizer.
    ///
    /// Corresponds to Java: `public InputStreamTokenizer(InputStream is)`
    pub fn new(stream: R) -> Self {
        Self {
            inner: Tokenizer::new(ByteSource::new(stream)),
        }
    }

    /// Take back the underlying data source.
    ///
    /// Corresponds to Java: no such method (in Java the `InputStream` is held by the caller); the
    /// Rust side uses it for the hand-off of "probe the file header by byte first, then decode by
    /// character".
    pub fn into_inner(self) -> R {
        self.inner.into_source().into_inner()
    }

    /// Read the next raw byte; returns `None` at the end of the data source.
    ///
    /// Corresponds to Java: `protected int read()` (protected in Java, available only to subclasses)
    pub fn read_byte(&mut self) -> MpxjResult<Option<u8>> {
        self.inner.source_mut().read_byte()
    }

    /// Get the content of the last token in byte form.
    ///
    /// Corresponds to Java: no such method (Java's `getToken()` has already converted the bytes to characters).
    ///
    /// Every character written to the buffer by the byte source is in the range U+0000–U+00FF, so
    /// converting back to bytes is lossless.
    pub fn get_token_bytes(&self) -> Vec<u8> {
        self.inner
            .get_token()
            .chars()
            .map(|c| c as u32 as u8)
            .collect()
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

    fn source(bytes: &[u8]) -> InputStreamTokenizer<Cursor<Vec<u8>>> {
        InputStreamTokenizer::new(Cursor::new(bytes.to_vec()))
    }

    /// Read all tokens and return a list of (type, text).
    fn read_all(tk: &mut InputStreamTokenizer<Cursor<Vec<u8>>>) -> Vec<(i32, String)> {
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
    fn reads_bytes_without_decoding() {
        // 0xE9 is é in CP1252, but the byte version does no decoding and directly yields U+00E9
        let mut tk = source(&[b'A', b',', 0xE9, b'\n']);
        let tokens = read_all(&mut tk);
        assert_eq!(
            tokens,
            vec![
                (TT_WORD, "A".to_string()),
                (TT_WORD, "é".to_string()),
                (TT_EOL, String::new()),
                (TT_EOF, String::new()),
            ]
        );
    }

    #[test]
    fn get_token_bytes_preserves_high_bytes() {
        let mut tk = source(&[0xE9, 0xFF, b'\n']);
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token_bytes(), vec![0xE9, 0xFF]);
        assert_eq!(tk.get_token().chars().count(), 2);
    }

    #[test]
    fn read_byte_returns_raw_bytes_then_none() {
        let mut tk = source(&[0x01, 0x80]);
        assert_eq!(tk.read_byte().unwrap(), Some(0x01));
        assert_eq!(tk.read_byte().unwrap(), Some(0x80));
        assert_eq!(tk.read_byte().unwrap(), None);
    }

    #[test]
    fn adjacent_delimiters_produce_empty_fields() {
        let mut tk = source(b"1,,3\n");
        let tokens = read_all(&mut tk);
        assert_eq!(tokens[1], (TT_WORD, String::new()));
        assert_eq!(tokens[2], (TT_WORD, "3".to_string()));
    }

    #[test]
    fn quoted_field_may_contain_delimiter() {
        let mut tk = source(b"\"a,b\",c\n");
        let tokens = read_all(&mut tk);
        assert_eq!(tokens[0], (TT_WORD, "a,b".to_string()));
        assert_eq!(tokens[1], (TT_WORD, "c".to_string()));
    }

    #[test]
    fn custom_delimiter_is_forwarded() {
        let mut tk = source(b"a|b\n");
        tk.set_delimiter('|');
        assert_eq!(tk.get_delimiter(), '|');
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "a");
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "b");
        assert_eq!(tk.get_type(), TT_WORD);
    }

    #[test]
    fn crlf_returns_word_then_eol() {
        let mut tk = source(b"a\r\nb\r\n");
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
    fn empty_input_returns_eof() {
        let mut tk = source(b"");
        assert_eq!(tk.next_token().unwrap(), TT_EOF);
        assert_eq!(tk.get_type(), TT_EOF);
    }

    #[test]
    fn does_not_read_ahead_of_the_current_record() {
        // Consistent with MPXReader's usage: after the file-header record is fully read (including the line terminator),
        // the remaining bytes must still be left in the data source for the subsequent character decoder to read
        let mut tk = source(b"A,B\nrest of file");
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "A");
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "B");
        assert_eq!(tk.next_token().unwrap(), TT_EOL);

        let cursor = tk.into_inner();
        let mut remaining = Vec::new();
        let mut cursor = cursor;
        cursor.read_to_end(&mut remaining).unwrap();
        assert_eq!(remaining, b"rest of file".to_vec());
    }

    #[test]
    fn constants_match_java() {
        assert_eq!(InputStreamTokenizer::<Cursor<Vec<u8>>>::TT_EOL, '\n' as i32);
        assert_eq!(InputStreamTokenizer::<Cursor<Vec<u8>>>::TT_EOF, -1);
        assert_eq!(InputStreamTokenizer::<Cursor<Vec<u8>>>::TT_WORD, -3);
    }
}
