//! Corresponds to Java: `org/mpxj/mpp/DocumentInputStreamFactory.java`
//!
//! Decides whether to return a **plain stream** or a **decrypting stream** based on `PASSWORD_FLAG` / `ENCRYPTION_CODE` in `Props`:
//! In password-protected MPP files, streams such as `Props` / `FixedData` are encrypted as a whole by XOR, requiring
//! [`EncryptedDocumentInputStream`] to decrypt while reading; unencrypted files yield the raw bytes directly.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `InputStream getInstance(DirectoryEntry, String)` | `get_instance(&DirectoryEntry, &str) -> MpxjResult<MppInputStream>` | Rust has no "common parent class for the two concrete stream types", so the enum [`MppInputStream`] expresses the two possible return types (both implement `Read`); POI throws `FileNotFoundException("Entry not found")` when the entry is missing, Rust returns `Err` |
//! | `byte getEncryptionCode()` | `get_encryption_code() -> u8` | Java returns a signed `byte`; Rust uses `u8`, which can be fed directly to `MPPUtility::decode_buffer` / `decode_password` (XOR only looks at the low 8 bits, so the semantics are equivalent) |
//! | Internal fields `m_encrypted` / `m_encryptionCode` | Private fields | Same as Java |
//!
//! The computation of `m_encryptionCode` matches Java word for word:
//! `code == 0x00 ? 0x00 : (byte) (0xFF - code)`; Java's `byte` is promoted to `int` and narrowed back to
//! `byte`, which is equivalent to Rust's wrapping subtraction on `u8`.
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: not used by this class.
//!
//! Skipped Java members: none.

use std::io::{self, Read};

use crate::mpp::encrypted_document_input_stream::EncryptedDocumentInputStream;
use crate::mpp::ole::{DirectoryEntry, DocumentInputStream};
use crate::mpp::props::Props;
use crate::mpp::props_key::PropsKey;
use crate::mpxj_exception::{MpxjError, MpxjResult};

/// Create a plain stream or a decrypting stream according to the conditions in `Props`.
///
/// Corresponds to Java: `class DocumentInputStreamFactory`
pub struct DocumentInputStreamFactory {
    /// Corresponds to Java: `private final boolean m_encrypted`
    encrypted: bool,
    /// Corresponds to Java: `private final byte m_encryptionCode`
    encryption_code: u8,
}

impl DocumentInputStreamFactory {
/// Constructor.
    ///
    /// Corresponds to Java: `public DocumentInputStreamFactory(Props props)`
    pub fn new(props: &Props) -> Self {
        // Java: m_encrypted = props.getByte(PropsKey.PASSWORD_FLAG) != 0;
        let encrypted = props.get_byte(PropsKey::PasswordFlag) != 0;

        // Java: byte code = props.getByte(PropsKey.ENCRYPTION_CODE);
        //       m_encryptionCode = (byte) (code == 0x00 ? 0x00 : (0xFF - code));
        let code = props.get_byte(PropsKey::EncryptionCode);
        let encryption_code = if code == 0x00 {
            0x00
        } else {
// Java promotes byte to int, subtracts, then narrows back to byte, equivalent to wrapping subtraction modulo 2^8
            0xFFu8.wrapping_sub(code as u8)
        };

        Self {
            encrypted,
            encryption_code,
        }
    }

/// Create the input stream: returns a decrypting stream for encrypted files, otherwise a plain stream.
    ///
    /// Corresponds to Java: `public InputStream getInstance(DirectoryEntry directory, String name) throws IOException`
    pub fn get_instance(&self, directory: &DirectoryEntry, name: &str) -> MpxjResult<MppInputStream> {
        // Java: DocumentEntry entry = (DocumentEntry) directory.getEntry(name);
// (POI throws FileNotFoundException("Entry not found") when the entry is missing)
        let Some(entry) = directory.get_document(name) else {
            return Err(MpxjError::new(format!("Entry not found: {name}")));
        };

        Ok(if self.encrypted {
            // Java: stream = new EncryptedDocumentInputStream(entry, m_encryptionCode);
            let data = entry.get_data();
            MppInputStream::Encrypted(EncryptedDocumentInputStream::new(
                &data,
                self.encryption_code,
            ))
        } else {
            // Java: stream = new DocumentInputStream(entry);
            MppInputStream::Plain(DocumentInputStream::new(&entry))
        })
    }

/// Get the encryption code.
    ///
    /// Corresponds to Java: `public byte getEncryptionCode()`
    pub fn get_encryption_code(&self) -> u8 {
        self.encryption_code
    }
}

/// The return value of [`DocumentInputStreamFactory::get_instance`].
///
/// Corresponds to Java: `InputStream` (the actual type is either POI's `DocumentInputStream` or
/// [`EncryptedDocumentInputStream`]). Rust has no inheritance, so an enum expresses these two possible concrete types.
pub enum MppInputStream {
/// Stream for unencrypted files.
    ///
    /// Corresponds to Java: `new DocumentInputStream(entry)`
    Plain(DocumentInputStream),
/// Decrypting stream for encrypted files.
    ///
    /// Corresponds to Java: `new EncryptedDocumentInputStream(entry, m_encryptionCode)`
    Encrypted(EncryptedDocumentInputStream),
}

impl MppInputStream {
/// Number of remaining readable bytes (corresponding to each branch's `available()`).
    pub fn available(&self) -> usize {
        match self {
            Self::Plain(stream) => stream.available(),
            Self::Encrypted(stream) => stream.available(),
        }
    }

/// Rust-specific: read all remaining bytes at once.
    ///
/// MPP consumers (`Props9` / `FixedData`, etc.) uniformly receive `&[u8]` on the Rust side,
/// so this convenience method is provided; neither branch's `Read` implementation returns an error (the data is already in memory).
    pub fn to_bytes(&mut self) -> Vec<u8> {
        let mut data = Vec::with_capacity(self.available());
        let _ = self.read_to_end(&mut data);
        data
    }
}

impl Read for MppInputStream {
    /// Corresponds to Java: `InputStream.read(byte[], int, int)`
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.read(buf),
            Self::Encrypted(stream) => stream.read(buf),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    use cfb::CompoundFile;

    use crate::mpp::mpp_utility::MPPUtility;
    use crate::mpp::ole::PoifsFileSystem;

/// Plaintext content.
    const PLAIN: &[u8] = b"FixedData contents";

/// The value of `PropsKey.ENCRYPTION_CODE` (XOR only happens when it is non-zero).
    const ENCRYPTION_CODE: i8 = 0x0A;

/// The mask corresponding to the above value: `(byte) (0xFF - 0x0A)`.
    const EXPECTED_MASK: u8 = 0xF5;

/// The ciphertext after applying the mask.
    fn encoded() -> Vec<u8> {
        let mut data = PLAIN.to_vec();
        MPPUtility::decode_buffer(&mut data, EXPECTED_MASK);
        data
    }

/// Build a `Props` containing only the two `PropsKey` entries.
    fn props(password_flag: i8, encryption_code: i8) -> Props {
        let mut props = Props::new();
        props.put(PropsKey::PasswordFlag.get_value(), vec![password_flag as u8]);
        props.put(
            PropsKey::EncryptionCode.get_value(),
            vec![encryption_code as u8],
        );
        props
    }

/// Build a compound document containing the `/   114/FixedData` stream and retrieve that storage.
    fn project_directory(data: &[u8]) -> (PoifsFileSystem, DirectoryEntry) {
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        file.create_storage("/   114").unwrap();
        file.create_stream("/   114/FixedData")
            .unwrap()
            .write_all(data)
            .unwrap();
        let bytes = file.into_inner().into_inner();

        let fs = PoifsFileSystem::new(Cursor::new(bytes)).unwrap();
        let directory = fs.get_root().get_directory("   114").expect("project storage");
        (fs, directory)
    }

    #[test]
    fn unencrypted_file_returns_the_raw_bytes() {
        let (_fs, directory) = project_directory(PLAIN);
        let factory = DocumentInputStreamFactory::new(&props(0, 0));

        let mut stream = factory
            .get_instance(&directory, "FixedData")
            .expect("should get FixedData");

// Unencrypted: takes the DocumentInputStream branch, bytes returned as-is
        assert!(matches!(stream, MppInputStream::Plain(_)));
        assert_eq!(stream.available(), PLAIN.len());
        assert_eq!(stream.to_bytes(), PLAIN.to_vec());
    }

    #[test]
    fn encrypted_file_returns_decrypted_bytes() {
        let (_fs, directory) = project_directory(&encoded());
        let factory = DocumentInputStreamFactory::new(&props(1, ENCRYPTION_CODE));

        assert_eq!(factory.get_encryption_code(), EXPECTED_MASK);

        let mut stream = factory
            .get_instance(&directory, "FixedData")
            .expect("should get FixedData");

// Encrypted: takes the EncryptedDocumentInputStream branch, what is read is plaintext
        assert!(matches!(stream, MppInputStream::Encrypted(_)));
        assert_eq!(stream.available(), PLAIN.len());
        assert_eq!(stream.to_bytes(), PLAIN.to_vec());
    }

    #[test]
    fn encryption_code_calculation_follows_java() {
// code == 0 => mask is 0 (data is unchanged even if PASSWORD_FLAG is true)
        assert_eq!(
            DocumentInputStreamFactory::new(&props(1, 0x00)).get_encryption_code(),
            0x00
        );
        // code == 0x0A => 0xFF - 0x0A
        assert_eq!(
            DocumentInputStreamFactory::new(&props(1, 0x0A)).get_encryption_code(),
            0xF5
        );
// code == 0xFF (Java's byte is -1) => 0xFF - (-1) = 0x100, narrowing back to byte gives 0
        assert_eq!(
            DocumentInputStreamFactory::new(&props(1, -1)).get_encryption_code(),
            0x00
        );
// When the mask's highest bit is 1, only the low 8 bits are kept as well
        assert_eq!(
            DocumentInputStreamFactory::new(&props(1, 0x7F)).get_encryption_code(),
            0x80
        );
    }

    #[test]
    fn zero_encryption_code_leaves_the_data_untouched() {
        let (_fs, directory) = project_directory(PLAIN);
// PASSWORD_FLAG is true but ENCRYPTION_CODE is 0: takes the decrypting stream, data unchanged when the mask is 0
        let factory = DocumentInputStreamFactory::new(&props(1, 0x00));

        let mut stream = factory
            .get_instance(&directory, "FixedData")
            .expect("should get FixedData");

        assert!(matches!(stream, MppInputStream::Encrypted(_)));
        assert_eq!(stream.to_bytes(), PLAIN.to_vec());
    }

    #[test]
    fn missing_entry_returns_an_error() {
        let (_fs, directory) = project_directory(PLAIN);
        let factory = DocumentInputStreamFactory::new(&props(0, 0));

// Java: POI throws FileNotFoundException("Entry not found")
        assert!(factory.get_instance(&directory, "NoSuchStream").is_err());
    }

    #[test]
    fn mpp_input_stream_streams_incrementally() {
        let (_fs, directory) = project_directory(&encoded());
        let factory = DocumentInputStreamFactory::new(&props(1, ENCRYPTION_CODE));

        let mut stream = factory
            .get_instance(&directory, "FixedData")
            .expect("should get FixedData");

        let mut buf = [0u8; 4];
        assert_eq!(stream.read(&mut buf).unwrap(), 4);
        assert_eq!(&buf[..], &PLAIN[..4]);
        assert_eq!(stream.available(), PLAIN.len() - 4);

        let mut rest = Vec::new();
        stream.read_to_end(&mut rest).unwrap();
        assert_eq!(rest, PLAIN[4..].to_vec());
    }
}
