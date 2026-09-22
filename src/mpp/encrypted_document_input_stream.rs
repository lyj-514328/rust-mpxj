//! Corresponds to Java: `org/mpxj/mpp/EncryptedDocumentInputStream.java`
//!
//! Wraps `DocumentInputStream` and decrypts on read using MPP's XOR rule (corresponds to
//! [`MPPUtility::decode_buffer`], i.e. Java's `MPPUtility.decodeBuffer`).
//! It is only used for password-protected MPP files, and
//! [`crate::mpp::document_input_stream_factory::DocumentInputStreamFactory`] decides whether to create it based on
//! `PropsKey::PASSWORD_FLAG` / `PropsKey::ENCRYPTION_CODE`.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `extends InputStream`, internally holds POI's `DocumentInputStream` | a standalone struct holding `Vec<u8>` + read position, `impl Read` | by project convention, the Rust side always replaces `InputStream` with `&[u8]`; for the implementation approach see [`crate::mpp::ole::DocumentInputStream`] |
//! | `EncryptedDocumentInputStream(DocumentEntry entry, int mask)` | `EncryptedDocumentInputStream::new(&[u8], u8)` | the inputs change from "stream + mask" to "byte slice + mask" |
//! | `int read()` | [`EncryptedDocumentInputStream::read_byte`] returns `Option<u8>` | at end of stream Java should return `-1`; Rust uses `None` |
//! | `int read(byte[] b, int off, int len)` | `impl Read::read` | Rust's `Read` only provides the "read into a buffer" form |
//! | `long skip(long n)` | `skip(i64) -> u64` | returns 0 when `n <= 0` (same as `InputStream.skip`) |
//! | `synchronized void mark(int)` / `reset()` / `boolean markSupported()` | `mark(usize)` / `reset() -> io::Result<()>` / `mark_supported()` | no `synchronized` is needed in a single-threaded environment; when mark is not set Java throws `IOException` and Rust returns `Err` |
//! | `void close()` | `close()` | the data is already in memory and there is no resource to release (the method is kept to correspond to Java) |
//!
//! # A trade-off regarding a Java defect
//!
//! Java's `read()` (no arguments) performs `value ^= m_mask`, and `m_mask` is a **signed int**:
//! when the mask's most significant bit is 1 (for example `0xF5`, i.e. `-11`), `0x0A ^ 0xFFFFFFF5 == -1`,
//! which misjudges a normal byte as EOF. The array version `read(byte[], int, int)` keeps only
//! the low 8 bits due to the narrowing conversion of the compound assignment, and is therefore correct. The Rust side **XORs only the low 8 bits** in both reads (consistent with
//! [`MPPUtility::decode_buffer`]), i.e. the same semantics as the array version.
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use them.
//!
//! Skipped Java members: none.

use std::io::{self, Read};

use crate::mpp::mpp_utility::MPPUtility;

/// Decrypting input stream.
///
/// Corresponds to Java: `final class EncryptedDocumentInputStream extends InputStream`
pub struct EncryptedDocumentInputStream {
/// The (encrypted) data to be read.
    ///
/// Corresponds to Java: the data provided by `private final DocumentInputStream m_dis`.
    data: Vec<u8>,
/// Current read position.
    position: usize,
/// The position recorded by the most recent `mark`.
    ///
/// Rust-specific: used to implement the `mark` / `reset` that Java provides via POI's `DocumentInputStream`.
    mark_position: Option<usize>,
/// Decryption mask.
    ///
/// Corresponds to Java: `private final int m_mask` (the Rust side uses `u8`; XOR only looks at the low 8 bits, so the semantics are equivalent).
    mask: u8,
}

impl EncryptedDocumentInputStream {
/// Constructor.
    ///
    /// Corresponds to Java: `public EncryptedDocumentInputStream(DocumentEntry entry, int mask) throws IOException`
    ///
/// Java internally calls `new DocumentInputStream(entry)` (POI's lazy reader, which may throw
/// `IOException` at construction); the Rust side directly receives the already-read byte slice, so the constructor does not return `Result`.
    pub fn new(data: &[u8], mask: u8) -> Self {
        Self {
            data: data.to_vec(),
            position: 0,
            mark_position: None,
            mask,
        }
    }

/// Read one byte and decrypt it.
    ///
    /// Corresponds to Java: `@Override public int read() throws IOException`
    ///
/// Java returns `0..=255` and returns `-1` at end of stream; Rust expresses this with `Option<u8>` (`None` = EOF).
    pub fn read_byte(&mut self) -> Option<u8> {
        let value = *self.data.get(self.position)?;
        self.position += 1;
// Java: value ^= m_mask (keeps only the low 8 bits; see the note in the file header)
        Some(value ^ self.mask)
    }

/// Number of remaining readable bytes.
    ///
    /// Corresponds to Java: `@Override public int available()`
    pub fn available(&self) -> usize {
        self.data.len().saturating_sub(self.position)
    }

/// Close the stream.
    ///
    /// Corresponds to Java: `@Override public void close()`
    ///
/// The data is already in memory and there is no underlying resource to release, so this is an empty implementation.
    pub fn close(&mut self) {}

/// Record the current position for [`Self::reset`] to rewind to.
    ///
    /// Corresponds to Java: `@Override public synchronized void mark(int readlimit)`
    ///
/// POI's `DocumentInputStream.mark` ignores `readlimit` (no invalidation check); the Rust side ignores it as well.
    pub fn mark(&mut self, read_limit: usize) {
        let _ = read_limit;
        self.mark_position = Some(self.position);
    }

/// Whether `mark` / `reset` are supported.
    ///
    /// Corresponds to Java: `@Override public boolean markSupported()`
    ///
/// POI's `DocumentInputStream.markSupported()` is always `true`.
    pub fn mark_supported(&self) -> bool {
        true
    }

/// Rewind to the position of the most recent [`Self::mark`].
    ///
    /// Corresponds to Java: `@Override public synchronized void reset() throws IOException`
    ///
/// When `mark` has not been called, POI throws `IOException("cannot reset")`; the Rust side returns `Err`.
    pub fn reset(&mut self) -> io::Result<()> {
        match self.mark_position {
            Some(position) => {
                self.position = position;
                Ok(())
            }
            None => Err(io::Error::other("cannot reset")),
        }
    }

/// Skip `n` bytes and return the number of bytes actually skipped.
    ///
    /// Corresponds to Java: `@Override public long skip(long n) throws IOException`
    ///
/// Returns 0 when `n <= 0` (the behavior of `InputStream.skip`); the skip length never exceeds the readable range.
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
    /// Corresponds to Java: `@Override public int read(byte[] b, int off, int len) throws IOException`
    ///
/// Java XORs all `len` bytes (including the part not read this time); the Rust side decrypts only the bytes actually read
/// (the unread bytes are left to be decrypted on the next read), giving the same decryption result. Decryption itself cannot fail,
/// so it never returns `Err`.
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

/// The mask is the typical result of `0xFF - code` (most significant bit is 1, which exposes the sign-extension issue in Java's `read()`).
    const MASK: u8 = 0xF5;

    #[test]
    fn decryption_round_trip() {
        let plain = b"hello encrypted world".to_vec();

// Encrypt: the same XOR rule (on the Java side MPPUtility.decodeBuffer decrypts in place)
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
// End of stream reached (Java should return -1)
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

// InputStream.skip returns 0 for non-positive values
        assert_eq!(stream.skip(0), 0);
        assert_eq!(stream.skip(-5), 0);

        assert_eq!(stream.skip(4), 4);
        assert_eq!(stream.available(), 6);

// The skip length is capped at the number of remaining bytes
        assert_eq!(stream.skip(100), 6);
        assert_eq!(stream.available(), 0);

// mark / reset (reset returns to the mark position)
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
// Java: POI's DocumentInputStream.reset() throws IOException("cannot reset")
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
