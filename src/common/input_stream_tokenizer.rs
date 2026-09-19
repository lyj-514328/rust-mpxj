//! 对应 Java: `org/mpxj/common/InputStreamTokenizer.java`
//!
//! 与 Java 版语义一致：**按字节读取**，读到的高位字节按 `(char) c` 映射（即 U+0000–U+00FF），
//! 不做任何字符集解码。MPX 读取正是靠这一点先按字节读出文件头（含代码页字段），
//! 之后再切换到按字符解码的 tokenizer（见 `MPXReader`）。
//!
//! 移植取舍：
//! - Java 通过继承 `Tokenizer` 覆写 `protected int read()`；Rust 无继承，本类型**组合**一个
//!   `Tokenizer<ByteSource<R>>` 并转发其公开方法（父类逻辑与状态完全复用，见各转发方法注释）。
//! - Java 的 `InputStream.read()` 每次只读一个字节、不做缓冲，因此不会吞掉后续数据；
//!   Rust 侧同样逐字节读取（`ByteSource::read_byte`），以保证「同一数据源先按字节探测、
//!   再交给字符解码器」的交接不会丢字节。
//! - 字节版额外提供 [`InputStreamTokenizer::read_byte`]（`u8`）与
//!   [`InputStreamTokenizer::get_token_bytes`]（`Vec<u8>`），对应 Java 中
//!   `InputStream.read()` / `(char) c` 无法表达的原始字节视图。
//! - `java.io.IOException` → `crate::MpxjError`；回退（Java 的 `m_nextCharacter` / `m_nextToken`）
//!   由 [`Tokenizer`] 内部字段实现。
//!
//! 未移植的方法：无。

use super::tokenizer::{TT_EOF, TokenSource, Tokenizer};
use crate::MpxjResult;
use std::io::Read;

/// 按字节读取的字符来源。
///
/// 对应 Java: `InputStreamTokenizer` 中覆写的 `protected int read()`。
pub struct ByteSource<R: Read> {
    /// 字节数据源，对应 Java 的 `InputStream m_stream`
    stream: R,
}

impl<R: Read> ByteSource<R> {
    /// 构造字节来源。
    ///
    /// 对应 Java: `public InputStreamTokenizer(InputStream is)`
    pub fn new(stream: R) -> Self {
        Self { stream }
    }

    /// 读取下一个字节；数据源结束时返回 `None`。
    ///
    /// 对应 Java: `m_stream.read()`（Java 返回 0..=255，数据源结束返回 -1）。
    ///
    /// 逐字节读取、不做预读：对应 Java 中 `InputStream.read()` 的行为，
    /// 保证调用方之后仍可从同一数据源继续读取。
    pub fn read_byte(&mut self) -> MpxjResult<Option<u8>> {
        let mut byte = [0u8; 1];
        match self.stream.read(&mut byte)? {
            0 => Ok(None),
            _ => Ok(Some(byte[0])),
        }
    }

    /// 取回底层字节数据源。
    ///
    /// 对应 Java: 无此方法（Java 中 `InputStream` 由调用方持有）。
    pub fn into_inner(self) -> R {
        self.stream
    }
}

impl<R: Read> TokenSource for ByteSource<R> {
    /// 对应 Java: `InputStreamTokenizer` 中覆写的 `protected int read()`
    fn read(&mut self) -> MpxjResult<i32> {
        match self.read_byte()? {
            // 对应 Java: `(char) c`——字节值 0..=255 直接作为字符编码
            Some(byte) => Ok(i32::from(byte)),
            None => Ok(TT_EOF),
        }
    }
}

/// 从 `Read` 按字节读取并分词的 tokenizer。
///
/// 对应 Java: `public class InputStreamTokenizer extends Tokenizer`
pub struct InputStreamTokenizer<R: Read> {
    /// 组合 + 转发：Java 侧为继承 `Tokenizer` 并覆写 `read()`
    inner: Tokenizer<ByteSource<R>>,
}

impl<R: Read> InputStreamTokenizer<R> {
    /// 构造 tokenizer。
    ///
    /// 对应 Java: `public InputStreamTokenizer(InputStream is)`
    pub fn new(stream: R) -> Self {
        Self {
            inner: Tokenizer::new(ByteSource::new(stream)),
        }
    }

    /// 取回底层数据源。
    ///
    /// 对应 Java: 无此方法（Java 中 `InputStream` 由调用方持有）；Rust 侧用于「先按字节探测文件头、
    /// 再按字符解码」的交接。
    pub fn into_inner(self) -> R {
        self.inner.into_source().into_inner()
    }

    /// 读取下一个原始字节，数据源结束时返回 `None`。
    ///
    /// 对应 Java: `protected int read()`（Java 中受 protected 限制，仅子类可用）
    pub fn read_byte(&mut self) -> MpxjResult<Option<u8>> {
        self.inner.source_mut().read_byte()
    }

    /// 以字节形式取回最后一个 token 的内容。
    ///
    /// 对应 Java: 无此方法（Java 的 `getToken()` 已把字节转成字符）。
    ///
    /// 字节来源写入缓冲区的每个字符都在 U+0000–U+00FF 范围内，因此转回字节无损。
    pub fn get_token_bytes(&self) -> Vec<u8> {
        self.inner
            .get_token()
            .chars()
            .map(|c| c as u32 as u8)
            .collect()
    }

    /// 对应 Java: `public int nextToken() throws IOException`
    pub fn next_token(&mut self) -> MpxjResult<i32> {
        self.inner.next_token()
    }

    /// 对应 Java: `public String getToken()`
    pub fn get_token(&self) -> &str {
        self.inner.get_token()
    }

    /// 对应 Java: `public int getType()`
    pub fn get_type(&self) -> i32 {
        self.inner.get_type()
    }

    /// 对应 Java: `public void setDelimiter(char delimiter)`
    pub fn set_delimiter(&mut self, delimiter: char) {
        self.inner.set_delimiter(delimiter);
    }

    /// 对应 Java: 无此方法（`m_delimiter` 为私有字段）
    pub fn get_delimiter(&self) -> char {
        self.inner.get_delimiter()
    }

    /// 对应 Java: `public static final int TT_EOL = '\n'`
    pub const TT_EOL: i32 = super::tokenizer::TT_EOL;

    /// 对应 Java: `public static final int TT_EOF = -1`
    pub const TT_EOF: i32 = super::tokenizer::TT_EOF;

    /// 对应 Java: `public static final int TT_WORD = -3`
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

    /// 读取全部 token，返回 (类型, 文本) 列表。
    fn read_all(tk: &mut InputStreamTokenizer<Cursor<Vec<u8>>>) -> Vec<(i32, String)> {
        let mut result = Vec::new();
        loop {
            let token_type = tk.next_token().expect("内存数据源不会失败");
            result.push((token_type, tk.get_token().to_string()));
            if token_type == TT_EOF {
                break;
            }
        }
        result
    }

    #[test]
    fn reads_bytes_without_decoding() {
        // 0xE9 在 CP1252 中是 é，但字节版不做解码，直接得到 U+00E9
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
        // 与 MPXReader 的用法一致：文件头记录读完（含行结束符）后，
        // 剩余字节必须仍然留在数据源中，供后续的字符解码器读取
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
