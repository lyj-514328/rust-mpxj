//! 对应 Java: `org/mpxj/common/ReaderTokenizer.java`
//!
//! 与 Java 版语义一致：**按字符读取**（先解码字节流，再按字符分词），并且成块预读
//! （Java 注释指出 `Reader.read()` 每次调用都要经过 `StreamDecoder`，加锁且每个字符都会分配，
//! 因此改为按块读取）。预读意味着 Reader 会超前于当前 token，不能被后续的其他读取者共享——
//! Rust 侧保持同样的约束。
//!
//! 移植取舍：
//! - Java 通过继承 `Tokenizer` 覆写 `protected int read()`；Rust 无继承，本类型**组合**一个
//!   `Tokenizer<CharSource<R>>` 并转发其公开方法（父类逻辑与状态完全复用，见各转发方法注释）。
//! - Java 的 `ReaderTokenizer(Reader r)` 接收的是「已经解码好的字符流」；Rust 的
//!   `std::io::Read` 只提供字节，因此把 Java 中 `new InputStreamReader(is, charset)` 的解码职责
//!   一并放进本类型：[`ReaderTokenizer::with_encoding`] 指定字符集，默认 UTF-8。
//! - Java 的 `char[] m_readBuffer` + `m_index` + `m_length`（读缓冲区 + 位置指针）原样保留，
//!   这是 Java 侧的回退/预读实现方式；Rust 侧额外用 `pending` 保存跨块残缺的多字节序列
//!   （Java 中该缓冲位于 `StreamDecoder` 内部）。
//! - Java 的一个 `char` 是 UTF-16 码元，Rust 的一个 `char` 是 Unicode 标量值：增补平面字符在
//!   Java 中是代理项对，在 Rust 中是单个 `char`（拼接出的文本相同）。
//! - `java.io.IOException` → `crate::MpxjError`，`PushbackReader` 式的回退由 [`Tokenizer`]
//!   内部的 `next_character` / `next_token` 字段完成。
//!
//! 未移植的方法：无。

use super::tokenizer::{TT_EOF, TokenSource, Tokenizer};
use crate::MpxjResult;
use encoding_rs::{Decoder, Encoding};
use std::io::Read;

/// 对应 Java: `private static final int BUFFER_SIZE = 8192`
///
/// Java 侧是 `char[]` 的元素个数，Rust 侧用作每次向 `Read` 请求的字节数。
const BUFFER_SIZE: usize = 8192;

/// 按字符读取的字符来源。
///
/// 对应 Java: `ReaderTokenizer` 中覆写的 `protected int read()`（以及 `Reader.read(char[])` 的预读）。
pub struct CharSource<R: Read> {
    /// 字节数据源，对应 Java 的 `Reader`（Java 中已由 `InputStreamReader` 解码）
    reader: R,
    /// 增量解码器，承担 Java 中 `InputStreamReader` + `StreamDecoder` 的解码职责
    decoder: Decoder,
    /// 对应 Java: `private final char[] m_readBuffer = new char[BUFFER_SIZE]`
    read_buffer: Vec<char>,
    /// 对应 Java: `private int m_index`
    index: usize,
    /// 对应 Java: `private int m_length`
    length: usize,
    /// 跨块残缺的多字节序列（Java 中由 `StreamDecoder` 内部持有）
    pending: Vec<u8>,
    /// 数据源是否已到达末尾，对应 Java 的 `m_length < 1`
    eof: bool,
}

impl<R: Read> CharSource<R> {
    /// 以 UTF-8 解码构造字符来源。
    ///
    /// 对应 Java: `new InputStreamReader(is)`（Java 使用平台默认字符集，Rust 侧统一为 UTF-8）。
    pub fn new(reader: R) -> Self {
        Self::with_encoding(reader, encoding_rs::UTF_8)
    }

    /// 以指定字符集解码构造字符来源。
    ///
    /// 对应 Java: `new InputStreamReader(is, charset)`
    pub fn with_encoding(reader: R, encoding: &'static Encoding) -> Self {
        Self {
            reader,
            // Java 的 InputStreamReader 不剥离 BOM，这里同样关闭 BOM 处理
            decoder: encoding.new_decoder_without_bom_handling(),
            read_buffer: Vec::new(),
            index: 0,
            length: 0,
            pending: Vec::new(),
            eof: false,
        }
    }

    /// 取回底层字节数据源。
    ///
    /// 对应 Java: 无此方法（Java 的 `Reader`/`InputStream` 由调用方持有）。
    ///
    /// 注意：本类型带预读，"取回"时数据源位置可能已经越过最后一个已返回的字符。
    pub fn into_inner(self) -> R {
        self.reader
    }

    /// 读取下一个字符；数据源结束时返回 `None`。
    ///
    /// 对应 Java: `protected int read() throws IOException`（Java 返回字符编码，
    /// 结束返回 `TT_EOF`；Rust 侧以 `Option<char>` 表达同一语义）。
    pub fn read_char(&mut self) -> MpxjResult<Option<char>> {
        if self.index == self.length {
            self.fill_buffer()?;
        }

        match self.read_buffer.get(self.index).copied() {
            Some(c) => {
                self.index += 1;
                Ok(Some(c))
            }
            // 对应 Java: `if (m_length < 1) { m_length = 0; return TT_EOF; }`
            None => Ok(None),
        }
    }

    /// 填满读缓冲区：对应 Java 中 `read()` 里对 `m_reader.read(m_readBuffer)` 的调用，
    /// 以及 `m_index = 0` 的位置指针复位。
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

            // 上一轮未消费的残缺序列拼到本轮数据之前
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
    /// 对应 Java: `ReaderTokenizer` 中覆写的 `protected int read()`
    fn read(&mut self) -> MpxjResult<i32> {
        match self.read_char()? {
            Some(c) => Ok(c as u32 as i32),
            None => Ok(TT_EOF),
        }
    }
}

/// 从 `Read` 按字符读取并分词的 tokenizer。
///
/// 对应 Java: `public class ReaderTokenizer extends Tokenizer`
pub struct ReaderTokenizer<R: Read> {
    /// 组合 + 转发：Java 侧为继承 `Tokenizer` 并覆写 `read()`
    inner: Tokenizer<CharSource<R>>,
}

impl<R: Read> ReaderTokenizer<R> {
    /// 构造 tokenizer，输入按 UTF-8 解码。
    ///
    /// 对应 Java: `public ReaderTokenizer(Reader r)`
    pub fn new(reader: R) -> Self {
        Self {
            inner: Tokenizer::new(CharSource::new(reader)),
        }
    }

    /// 构造 tokenizer，输入按指定字符集解码。
    ///
    /// 对应 Java: `new ReaderTokenizer(new InputStreamReader(is, charset))`
    pub fn with_encoding(reader: R, encoding: &'static Encoding) -> Self {
        Self {
            inner: Tokenizer::new(CharSource::with_encoding(reader, encoding)),
        }
    }

    /// 取回底层数据源。
    ///
    /// 对应 Java: 无此方法（Java 中 `Reader` 由调用方持有）；Rust 侧用于「先按字节探测文件头、
    /// 再按字符解码」的交接。
    pub fn into_inner(self) -> R {
        self.inner.into_source().into_inner()
    }

    /// 读取下一个字符，数据源结束时返回 `None`。
    ///
    /// 对应 Java: `protected int read()`（Java 中受 protected 限制，仅子类可用）
    pub fn read_char(&mut self) -> MpxjResult<Option<char>> {
        self.inner.source_mut().read_char()
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

    fn source(text: &str) -> ReaderTokenizer<Cursor<Vec<u8>>> {
        ReaderTokenizer::new(Cursor::new(text.as_bytes().to_vec()))
    }

    /// 读取全部 token，返回 (类型, 文本) 列表。
    fn read_all(tk: &mut ReaderTokenizer<Cursor<Vec<u8>>>) -> Vec<(i32, String)> {
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
        // 8191 个 ASCII 字符 + 一个 3 字节 UTF-8 字符，正好跨过 8192 字节的读缓冲区边界
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
        // GBK 编码的「中文」
        let bytes = vec![0xD6, 0xD0, 0xCE, 0xC4];
        let mut tk = ReaderTokenizer::with_encoding(Cursor::new(bytes), encoding_rs::GBK);
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "中文");

        // Windows-1252 编码的「é」
        let mut tk = ReaderTokenizer::with_encoding(Cursor::new(vec![0xE9]), encoding_rs::WINDOWS_1252);
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "é");
    }

    #[test]
    fn invalid_bytes_become_replacement_characters() {
        // 孤立的高位字节在 UTF-8 中非法
        let mut tk = ReaderTokenizer::new(Cursor::new(vec![0xFF, b'a']));
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "\u{FFFD}a");
    }

    #[test]
    fn into_inner_returns_the_underlying_reader() {
        let mut tk = source("a,b\n");
        assert_eq!(tk.next_token().unwrap(), TT_WORD);
        assert_eq!(tk.get_token(), "a");
        // 预读会把数据源推进到末尾（Java 侧同样是「Reader 被读超前」的约束）
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
