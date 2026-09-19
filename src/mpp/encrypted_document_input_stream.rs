//! 对应 Java: `org/mpxj/mpp/EncryptedDocumentInputStream.java`
//!
//! 包装 `DocumentInputStream`，在读取时按 MPP 的 XOR 规则解密（对应
//! [`MPPUtility::decode_buffer`]，即 Java 的 `MPPUtility.decodeBuffer`）。
//! 只有被密码保护的 MPP 文件才会用到它，由
//! [`crate::mpp::document_input_stream_factory::DocumentInputStreamFactory`] 按
//! `PropsKey::PASSWORD_FLAG` / `PropsKey::ENCRYPTION_CODE` 决定是否创建。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `extends InputStream`，内部持有 POI 的 `DocumentInputStream` | 持有 `Vec<u8>` + 读取位置的独立 struct，`impl Read` | 按本批约定，Rust 侧 `InputStream` 一律换成 `&[u8]`；实现方式参考 [`crate::mpp::ole::DocumentInputStream`] |
//! | `EncryptedDocumentInputStream(DocumentEntry entry, int mask)` | `EncryptedDocumentInputStream::new(&[u8], u8)` | 入参由「流 + 掩码」变为「字节切片 + 掩码」 |
//! | `int read()` | [`EncryptedDocumentInputStream::read_byte`] 返回 `Option<u8>` | 到达末尾 Java 应返回 `-1`，Rust 用 `None` |
//! | `int read(byte[] b, int off, int len)` | `impl Read::read` | Rust 的 `Read` 只提供「读入缓冲区」这一种形态 |
//! | `long skip(long n)` | `skip(i64) -> u64` | `n <= 0` 返回 0（同 `InputStream.skip`） |
//! | `synchronized void mark(int)` / `reset()` / `boolean markSupported()` | `mark(usize)` / `reset() -> io::Result<()>` / `mark_supported()` | 单线程环境无需 `synchronized`；未设 mark 时 Java 抛 `IOException`，Rust 返回 `Err` |
//! | `void close()` | `close()` | 数据已在内存中，无资源可释放（保留方法以对应 Java） |
//!
//! # 一处 Java 缺陷的取舍
//!
//! Java 的 `read()`（无参）执行 `value ^= m_mask`，而 `m_mask` 是**有符号 int**：
//! 当掩码最高位为 1（例如 `0xF5` 即 `-11`）时，`0x0A ^ 0xFFFFFFF5 == -1`，
//! 会把正常字节误判成 EOF。数组版 `read(byte[], int, int)` 因复合赋值的窄化转换只保留
//! 低 8 位，才是正确的。Rust 侧两种读取**都只按低 8 位异或**（与
//! [`MPPUtility::decode_buffer`] 一致），即与数组版语义相同。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
//!
//! 跳过的 Java 成员：无。

use std::io::{self, Read};

use crate::mpp::mpp_utility::MPPUtility;

/// 解密输入流。
///
/// 对应 Java: `final class EncryptedDocumentInputStream extends InputStream`
pub struct EncryptedDocumentInputStream {
    /// 待读取（已加密）的数据。
    ///
    /// 对应 Java: `private final DocumentInputStream m_dis` 所提供的数据。
    data: Vec<u8>,
    /// 当前读取位置。
    position: usize,
    /// 最近一次 `mark` 记录的位置。
    ///
    /// Rust 特有：用于实现 Java 经由 POI `DocumentInputStream` 提供的 `mark` / `reset`。
    mark_position: Option<usize>,
    /// 解密掩码。
    ///
    /// 对应 Java: `private final int m_mask`（Rust 侧用 `u8`，异或只看低 8 位，语义等价）。
    mask: u8,
}

impl EncryptedDocumentInputStream {
    /// 构造方法。
    ///
    /// 对应 Java: `public EncryptedDocumentInputStream(DocumentEntry entry, int mask) throws IOException`
    ///
    /// Java 内部会 `new DocumentInputStream(entry)`（POI 的惰性读取器，构造时可能抛
    /// `IOException`）；Rust 侧直接接收已读出的字节切片，故构造不返回 `Result`。
    pub fn new(data: &[u8], mask: u8) -> Self {
        Self {
            data: data.to_vec(),
            position: 0,
            mark_position: None,
            mask,
        }
    }

    /// 读取一个字节并解密。
    ///
    /// 对应 Java: `@Override public int read() throws IOException`
    ///
    /// Java 返回 `0..=255`，到达末尾返回 `-1`；Rust 用 `Option<u8>` 表达（`None` = EOF）。
    pub fn read_byte(&mut self) -> Option<u8> {
        let value = *self.data.get(self.position)?;
        self.position += 1;
        // Java: value ^= m_mask（只保留低 8 位，见文件头的说明）
        Some(value ^ self.mask)
    }

    /// 剩余可读字节数。
    ///
    /// 对应 Java: `@Override public int available()`
    pub fn available(&self) -> usize {
        self.data.len().saturating_sub(self.position)
    }

    /// 关闭流。
    ///
    /// 对应 Java: `@Override public void close()`
    ///
    /// 数据已在内存中，无底层资源需要释放，故为空实现。
    pub fn close(&mut self) {}

    /// 记录当前位置，供 [`Self::reset`] 回退。
    ///
    /// 对应 Java: `@Override public synchronized void mark(int readlimit)`
    ///
    /// POI 的 `DocumentInputStream.mark` 会忽略 `readlimit`（不做失效判断），Rust 侧同样忽略。
    pub fn mark(&mut self, read_limit: usize) {
        let _ = read_limit;
        self.mark_position = Some(self.position);
    }

    /// 是否支持 `mark` / `reset`。
    ///
    /// 对应 Java: `@Override public boolean markSupported()`
    ///
    /// POI 的 `DocumentInputStream.markSupported()` 恒为 `true`。
    pub fn mark_supported(&self) -> bool {
        true
    }

    /// 回退到最近一次 [`Self::mark`] 的位置。
    ///
    /// 对应 Java: `@Override public synchronized void reset() throws IOException`
    ///
    /// 未调用过 `mark` 时 POI 抛 `IOException("cannot reset")`，Rust 侧返回 `Err`。
    pub fn reset(&mut self) -> io::Result<()> {
        match self.mark_position {
            Some(position) => {
                self.position = position;
                Ok(())
            }
            None => Err(io::Error::other("cannot reset")),
        }
    }

    /// 跳过 `n` 个字节，返回实际跳过的字节数。
    ///
    /// 对应 Java: `@Override public long skip(long n) throws IOException`
    ///
    /// `n <= 0` 时返回 0（`InputStream.skip` 的行为），跳过长度不超过可读范围。
    pub fn skip(&mut self, n: i64) -> u64 {
        if n <= 0 {
            return 0;
        }

        let start = self.position;
        self.position = (self.position + n as usize).min(self.data.len());
        (self.position - start) as u64
    }
}

impl Read for EncryptedDocumentInputStream {
    /// 对应 Java: `@Override public int read(byte[] b, int off, int len) throws IOException`
    ///
    /// Java 对 `len` 个字节全部异或（包括本次未读到的部分），Rust 侧只解密实际读到的
    /// 字节（未读到的字节留给下一次读取时解密），解密结果一致。解密本身不会失败，
    /// 故不会返回 `Err`。
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let start = self.position.min(self.data.len());
        let remaining = &self.data[start..];
        let count = remaining.len().min(buf.len());
        buf[..count].copy_from_slice(&remaining[..count]);
        self.position = start + count;

        // Java: for (int loop = 0; loop < len; loop++) { b[loop + off] ^= m_mask; }
        MPPUtility::decode_buffer(&mut buf[..count], self.mask);

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 掩码取 `0xFF - code` 的典型结果（最高位为 1，可暴露 Java `read()` 的符号扩展问题）。
    const MASK: u8 = 0xF5;

    #[test]
    fn decryption_round_trip() {
        let plain = b"hello encrypted world".to_vec();

        // 加密：同一套 XOR 规则（Java 侧由 MPPUtility.decodeBuffer 就地解密）
        let mut encoded = plain.clone();
        MPPUtility::decode_buffer(&mut encoded, MASK);
        assert_ne!(encoded, plain);

        let mut stream = EncryptedDocumentInputStream::new(&encoded, MASK);
        assert_eq!(stream.available(), plain.len());

        let mut result = Vec::new();
        stream.read_to_end(&mut result).unwrap();

        assert_eq!(result, plain);
        assert_eq!(stream.available(), 0);
    }

    #[test]
    fn reads_single_bytes_and_reports_eof() {
        let encoded = [0x01u8 ^ MASK, 0xFF ^ MASK];

        let mut stream = EncryptedDocumentInputStream::new(&encoded, MASK);
        assert_eq!(stream.read_byte(), Some(0x01));
        assert_eq!(stream.read_byte(), Some(0xFF));
        // 到达末尾（Java 应返回 -1）
        assert_eq!(stream.read_byte(), None);
    }

    #[test]
    fn zero_mask_is_a_pass_through() {
        let plain = b"plain".to_vec();
        let mut stream = EncryptedDocumentInputStream::new(&plain, 0x00);

        let mut result = Vec::new();
        stream.read_to_end(&mut result).unwrap();
        assert_eq!(result, plain);
    }

    #[test]
    fn skip_mark_and_reset() {
        let plain = b"0123456789".to_vec();
        let mut encoded = plain.clone();
        MPPUtility::decode_buffer(&mut encoded, MASK);

        let mut stream = EncryptedDocumentInputStream::new(&encoded, MASK);

        // InputStream.skip 对非正数返回 0
        assert_eq!(stream.skip(0), 0);
        assert_eq!(stream.skip(-5), 0);

        assert_eq!(stream.skip(4), 4);
        assert_eq!(stream.available(), 6);

        // 跳过长度上限为剩余字节数
        assert_eq!(stream.skip(100), 6);
        assert_eq!(stream.available(), 0);

        // mark / reset（reset 回到 mark 的位置）
        let mut stream = EncryptedDocumentInputStream::new(&encoded, MASK);
        assert!(stream.mark_supported());
        assert_eq!(stream.skip(3), 3);
        stream.mark(0);
        assert_eq!(stream.skip(2), 2);
        assert!(stream.reset().is_ok());

        let mut rest = Vec::new();
        stream.read_to_end(&mut rest).unwrap();
        assert_eq!(rest, b"3456789".to_vec());
    }

    #[test]
    fn reset_without_mark_is_an_error() {
        let mut stream = EncryptedDocumentInputStream::new(&[0x00], MASK);
        // Java: POI 的 DocumentInputStream.reset() 抛 IOException("cannot reset")
        assert!(stream.reset().is_err());
    }

    #[test]
    fn close_is_a_no_op() {
        let mut stream = EncryptedDocumentInputStream::new(&[0x00], MASK);
        stream.close();
        assert!(stream.read_byte().is_some());
    }

    #[test]
    fn empty_stream_reads_nothing() {
        let mut stream = EncryptedDocumentInputStream::new(&[], MASK);
        assert_eq!(stream.available(), 0);
        assert_eq!(stream.read_byte(), None);

        let mut buf = [0u8; 4];
        assert_eq!(stream.read(&mut buf).unwrap(), 0);
    }

    #[test]
    fn read_fills_only_the_available_bytes() {
        let plain = b"abcd".to_vec();
        let mut encoded = plain.clone();
        MPPUtility::decode_buffer(&mut encoded, MASK);

        let mut stream = EncryptedDocumentInputStream::new(&encoded, MASK);
        let mut buf = [0u8; 2];

        assert_eq!(stream.read(&mut buf).unwrap(), 2);
        assert_eq!(buf, *b"ab");
        assert_eq!(stream.read(&mut buf).unwrap(), 2);
        assert_eq!(buf, *b"cd");
        assert_eq!(stream.read(&mut buf).unwrap(), 0);
    }
}
