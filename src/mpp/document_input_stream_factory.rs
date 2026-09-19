//! 对应 Java: `org/mpxj/mpp/DocumentInputStreamFactory.java`
//!
//! 按 `Props` 里的 `PASSWORD_FLAG` / `ENCRYPTION_CODE` 决定返回**普通流**还是**解密流**：
//! 被密码保护的 MPP 文件里，`Props` / `FixedData` 等流整体按 XOR 规则加密，需要
//! [`EncryptedDocumentInputStream`] 在读取时解密；未加密的文件直接给出原始字节。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `InputStream getInstance(DirectoryEntry, String)` | `get_instance(&DirectoryEntry, &str) -> MpxjResult<MppInputStream>` | Rust 没有「两种具体流类型的共同父类」，用枚举 [`MppInputStream`] 表达返回的两种类型（两者都实现 `Read`）；POI 找不到条目时抛 `FileNotFoundException("Entry not found")`，Rust 返回 `Err` |
//! | `byte getEncryptionCode()` | `get_encryption_code() -> u8` | Java 返回有符号 `byte`；Rust 用 `u8`，可直接喂给 `MPPUtility::decode_buffer` / `decode_password`（异或只看低 8 位，语义等价） |
//! | 内部字段 `m_encrypted` / `m_encryptionCode` | 私有字段 | 同 Java |
//!
//! `m_encryptionCode` 的计算与 Java 逐字一致：
//! `code == 0x00 ? 0x00 : (byte) (0xFF - code)`；Java 的 `byte` 提升为 `int` 再窄化回
//! `byte` 等价于 Rust 的 `u8` 回绕减法。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
//!
//! 跳过的 Java 成员：无。

use std::io::{self, Read};

use crate::mpp::encrypted_document_input_stream::EncryptedDocumentInputStream;
use crate::mpp::ole::{DirectoryEntry, DocumentInputStream};
use crate::mpp::props::Props;
use crate::mpp::props_key::PropsKey;
use crate::mpxj_exception::{MpxjError, MpxjResult};

/// 按 `Props` 的条件创建普通流或解密流。
///
/// 对应 Java: `class DocumentInputStreamFactory`
pub struct DocumentInputStreamFactory {
    /// 对应 Java: `private final boolean m_encrypted`
    encrypted: bool,
    /// 对应 Java: `private final byte m_encryptionCode`
    encryption_code: u8,
}

impl DocumentInputStreamFactory {
    /// 构造方法。
    ///
    /// 对应 Java: `public DocumentInputStreamFactory(Props props)`
    pub fn new(props: &Props) -> Self {
        // Java: m_encrypted = props.getByte(PropsKey.PASSWORD_FLAG) != 0;
        let encrypted = props.get_byte(PropsKey::PasswordFlag) != 0;

        // Java: byte code = props.getByte(PropsKey.ENCRYPTION_CODE);
        //       m_encryptionCode = (byte) (code == 0x00 ? 0x00 : (0xFF - code));
        let code = props.get_byte(PropsKey::EncryptionCode);
        let encryption_code = if code == 0x00 {
            0x00
        } else {
            // Java 把 byte 提升为 int 做减法后窄化回 byte，等价于按 2^8 回绕的减法
            0xFFu8.wrapping_sub(code as u8)
        };

        Self {
            encrypted,
            encryption_code,
        }
    }

    /// 创建输入流：加密文件返回解密流，否则返回普通流。
    ///
    /// 对应 Java: `public InputStream getInstance(DirectoryEntry directory, String name) throws IOException`
    pub fn get_instance(&self, directory: &DirectoryEntry, name: &str) -> MpxjResult<MppInputStream> {
        // Java: DocumentEntry entry = (DocumentEntry) directory.getEntry(name);
        // （POI 找不到条目时抛 FileNotFoundException("Entry not found")）
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

    /// 取加密码。
    ///
    /// 对应 Java: `public byte getEncryptionCode()`
    pub fn get_encryption_code(&self) -> u8 {
        self.encryption_code
    }
}

/// [`DocumentInputStreamFactory::get_instance`] 的返回值。
///
/// 对应 Java: `InputStream`（实际类型为 POI 的 `DocumentInputStream` 或
/// [`EncryptedDocumentInputStream`]）。Rust 没有继承，用枚举表达这两个可能的具体类型。
pub enum MppInputStream {
    /// 未加密文件的流。
    ///
    /// 对应 Java: `new DocumentInputStream(entry)`
    Plain(DocumentInputStream),
    /// 加密文件的解密流。
    ///
    /// 对应 Java: `new EncryptedDocumentInputStream(entry, m_encryptionCode)`
    Encrypted(EncryptedDocumentInputStream),
}

impl MppInputStream {
    /// 剩余可读字节数（对应两个分支各自的 `available()`）。
    pub fn available(&self) -> usize {
        match self {
            Self::Plain(stream) => stream.available(),
            Self::Encrypted(stream) => stream.available(),
        }
    }

    /// Rust 特有：一次性读出全部剩余字节。
    ///
    /// MPP 的消费者（`Props9` / `FixedData` 等）在 Rust 侧统一接收 `&[u8]`，
    /// 故提供该便捷方法；两个分支的 `Read` 实现都不会返回错误（数据已在内存中）。
    pub fn to_bytes(&mut self) -> Vec<u8> {
        let mut data = Vec::with_capacity(self.available());
        let _ = self.read_to_end(&mut data);
        data
    }
}

impl Read for MppInputStream {
    /// 对应 Java: `InputStream.read(byte[], int, int)`
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

    /// 明文内容。
    const PLAIN: &[u8] = b"FixedData contents";

    /// `PropsKey.ENCRYPTION_CODE` 的取值（非 0 才会真正异或）。
    const ENCRYPTION_CODE: i8 = 0x0A;

    /// 上述取值对应的掩码：`(byte) (0xFF - 0x0A)`。
    const EXPECTED_MASK: u8 = 0xF5;

    /// 掩码作用后的密文。
    fn encoded() -> Vec<u8> {
        let mut data = PLAIN.to_vec();
        MPPUtility::decode_buffer(&mut data, EXPECTED_MASK);
        data
    }

    /// 构造只含 `PropsKey` 两项的 `Props`。
    fn props(password_flag: i8, encryption_code: i8) -> Props {
        let mut props = Props::new();
        props.put(PropsKey::PasswordFlag.get_value(), vec![password_flag as u8]);
        props.put(
            PropsKey::EncryptionCode.get_value(),
            vec![encryption_code as u8],
        );
        props
    }

    /// 构造一个含 `/   114/FixedData` 流的复合文档并取回该存储。
    fn project_directory(data: &[u8]) -> (PoifsFileSystem, DirectoryEntry) {
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        file.create_storage("/   114").unwrap();
        file.create_stream("/   114/FixedData")
            .unwrap()
            .write_all(data)
            .unwrap();
        let bytes = file.into_inner().into_inner();

        let fs = PoifsFileSystem::new(Cursor::new(bytes)).unwrap();
        let directory = fs.get_root().get_directory("   114").expect("项目存储");
        (fs, directory)
    }

    #[test]
    fn unencrypted_file_returns_the_raw_bytes() {
        let (_fs, directory) = project_directory(PLAIN);
        let factory = DocumentInputStreamFactory::new(&props(0, 0));

        let mut stream = factory
            .get_instance(&directory, "FixedData")
            .expect("应能取到 FixedData");

        // 未加密：走 DocumentInputStream 分支，字节原样返回
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
            .expect("应能取到 FixedData");

        // 加密：走 EncryptedDocumentInputStream 分支，读到的是明文
        assert!(matches!(stream, MppInputStream::Encrypted(_)));
        assert_eq!(stream.available(), PLAIN.len());
        assert_eq!(stream.to_bytes(), PLAIN.to_vec());
    }

    #[test]
    fn encryption_code_calculation_follows_java() {
        // code == 0 => 掩码为 0（即使 PASSWORD_FLAG 为真也不改变数据）
        assert_eq!(
            DocumentInputStreamFactory::new(&props(1, 0x00)).get_encryption_code(),
            0x00
        );
        // code == 0x0A => 0xFF - 0x0A
        assert_eq!(
            DocumentInputStreamFactory::new(&props(1, 0x0A)).get_encryption_code(),
            0xF5
        );
        // code == 0xFF（Java 的 byte 为 -1）=> 0xFF - (-1) = 0x100，窄化回 byte 为 0
        assert_eq!(
            DocumentInputStreamFactory::new(&props(1, -1)).get_encryption_code(),
            0x00
        );
        // 掩码最高位为 1 时同样只保留低 8 位
        assert_eq!(
            DocumentInputStreamFactory::new(&props(1, 0x7F)).get_encryption_code(),
            0x80
        );
    }

    #[test]
    fn zero_encryption_code_leaves_the_data_untouched() {
        let (_fs, directory) = project_directory(PLAIN);
        // PASSWORD_FLAG 为真但 ENCRYPTION_CODE 为 0：走解密流，掩码 0 时数据不变
        let factory = DocumentInputStreamFactory::new(&props(1, 0x00));

        let mut stream = factory
            .get_instance(&directory, "FixedData")
            .expect("应能取到 FixedData");

        assert!(matches!(stream, MppInputStream::Encrypted(_)));
        assert_eq!(stream.to_bytes(), PLAIN.to_vec());
    }

    #[test]
    fn missing_entry_returns_an_error() {
        let (_fs, directory) = project_directory(PLAIN);
        let factory = DocumentInputStreamFactory::new(&props(0, 0));

        // Java: POI 抛 FileNotFoundException("Entry not found")
        assert!(factory.get_instance(&directory, "NoSuchStream").is_err());
    }

    #[test]
    fn mpp_input_stream_streams_incrementally() {
        let (_fs, directory) = project_directory(&encoded());
        let factory = DocumentInputStreamFactory::new(&props(1, ENCRYPTION_CODE));

        let mut stream = factory
            .get_instance(&directory, "FixedData")
            .expect("应能取到 FixedData");

        let mut buf = [0u8; 4];
        assert_eq!(stream.read(&mut buf).unwrap(), 4);
        assert_eq!(&buf[..], &PLAIN[..4]);
        assert_eq!(stream.available(), PLAIN.len() - 4);

        let mut rest = Vec::new();
        stream.read_to_end(&mut rest).unwrap();
        assert_eq!(rest, PLAIN[4..].to_vec());
    }
}
