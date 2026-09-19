//! OLE2 复合文档访问层（Rust 特有）。
//!
//! MPXJ 的 `org.mpxj.mpp` 包直接使用 Apache POI 的 `POIFSFileSystem` /
//! `DirectoryEntry` / `DocumentEntry` / `DocumentInputStream` 访问 MPP 文件。
//! Rust 侧改用 [`cfb`](https://crates.io/crates/cfb) crate，本模块把两者的差异
//! 封装成一组与 POI **同名**的类型，使 mpp 包的翻译保持 1:1：
//!
//! | Apache POI | 本模块 |
//! |---|---|
//! | `POIFSFileSystem` | [`PoifsFileSystem`] |
//! | `DirectoryEntry` | [`DirectoryEntry`] |
//! | `DocumentEntry` | [`DocumentEntry`] |
//! | `DocumentInputStream` | [`DocumentInputStream`] |
//!
//! # 与 POI 的差异
//!
//! 1. **输入源统一为 `Box<dyn ReadSeek>`**：POI 的重载（`File` / `InputStream`）
//!    在 Rust 侧合并，避免 `PoifsFileSystem<R>` 的泛型参数扩散到整个 mpp 包。
//! 2. **文档按需整体读入 `Vec<u8>`**：POI 的 `DocumentInputStream` 是惰性读取器，
//!    而 `cfb` 的 `open_stream` 需要 `&mut CompoundFile` 且返回的 `Stream` 借用底层
//!    文件，无法跨越 `RefCell` 借用边界返回。MPP 的消费者（`Props` / `Var2Data` /
//!    `FixedData` 等）本来就是一次性读完，故这里直接读出字节并交给 [`DocumentInputStream`]
//!    以 `Cursor` 形式提供，行为等价且借用关系简单。
//! 3. **按名查找而非拼路径**：先枚举当前存储的子项、按名字匹配，再用子项自带的路径
//!    打开流 —— 这样 `\1CompObj` 这类含控制字符的名字不会因路径拼接而出问题。
//! 4. POI 的 `DirectoryEntry.getEntry(name)` 对不存在的项抛异常；Rust 侧返回 `None`
//!    （用 [`DirectoryEntry::get_entry_names`] 可先判断是否存在）。

use std::cell::RefCell;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::rc::Rc;

use cfb::CompoundFile;

/// 同时可读可定位的输入源。
///
/// 用于把 POI 的 `File` / `InputStream` 两类入口统一成一个具体类型，
/// 使 [`PoifsFileSystem`] 及整个 mpp 包都不必携带泛型参数。
pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

/// 对应 POI: `POIFSFileSystem`
pub struct PoifsFileSystem {
    file: Rc<RefCell<CompoundFile<Box<dyn ReadSeek>>>>,
}

impl PoifsFileSystem {
    /// 从任意可读可定位的输入源打开复合文档。
    ///
    /// 对应 POI: `new POIFSFileSystem(InputStream)` / `new POIFSFileSystem(File)`
    pub fn new<R: ReadSeek + 'static>(inner: R) -> std::io::Result<Self> {
        let file = CompoundFile::open(Box::new(inner) as Box<dyn ReadSeek>)?;
        Ok(Self {
            file: Rc::new(RefCell::new(file)),
        })
    }

    /// 取根存储。
    ///
    /// 对应 POI: `DirectoryEntry getRoot()`
    pub fn get_root(&self) -> DirectoryEntry {
        DirectoryEntry {
            file: Rc::clone(&self.file),
            path: None,
        }
    }

    /// 判断给定名字的流是否存在（便于对应 POI 侧「先判 `getEntryNames()` 再取」的写法）。
    ///
    /// 对应 POI: `root.getEntryNames().contains(name)`
    pub fn exists(&self, name: &str) -> bool {
        self.get_root().get_entry(name).is_some()
    }
}

/// 复合文档中的一个条目：存储（目录）或流（文档）。
///
/// 对应 POI 里 `DirectoryEntry` / `DocumentEntry` 两种角色的 `getEntry()` 返回值。
pub enum Entry {
    /// 子存储（目录）。
    Directory(DirectoryEntry),
    /// 流（文档）。
    Document(DocumentEntry),
}

/// 对应 POI: `DirectoryEntry`（复合文档中的存储/目录）
#[derive(Clone)]
pub struct DirectoryEntry {
    file: Rc<RefCell<CompoundFile<Box<dyn ReadSeek>>>>,
    /// `None` 表示根存储（对应 POI 的 `fs.getRoot()`）。
    path: Option<PathBuf>,
}

impl DirectoryEntry {
    /// 取指定名字的子项；不存在时返回 `None`。
    ///
    /// 对应 POI: `Entry getEntry(String name)`（POI 找不到时抛异常，Rust 侧返回 `None`）
    pub fn get_entry(&self, name: &str) -> Option<Entry> {
        let child_path = self.find_child_path(name)?;
        let is_stream = self.file.borrow().is_stream(&child_path);

        if is_stream {
            let data = self.read_stream(&child_path)?;
            Some(Entry::Document(DocumentEntry {
                name: name.to_owned(),
                data: Rc::new(data),
            }))
        } else {
            Some(Entry::Directory(DirectoryEntry {
                file: Rc::clone(&self.file),
                path: Some(child_path),
            }))
        }
    }

    /// 取指定名字的子存储（目录）；不存在或不是目录时返回 `None`。
    ///
    /// 对应 POI: `(DirectoryEntry) directory.getEntry(name)`
    pub fn get_directory(&self, name: &str) -> Option<DirectoryEntry> {
        match self.get_entry(name) {
            Some(Entry::Directory(directory)) => Some(directory),
            _ => None,
        }
    }

    /// 取指定名字的流；不存在或不是流时返回 `None`。
    ///
    /// 对应 POI: `(DocumentEntry) directory.getEntry(name)`
    pub fn get_document(&self, name: &str) -> Option<DocumentEntry> {
        match self.get_entry(name) {
            Some(Entry::Document(document)) => Some(document),
            _ => None,
        }
    }

    /// 列出本存储下全部子项的名字。
    ///
    /// 对应 POI: `Set<String> getEntryNames()`
    pub fn get_entry_names(&self) -> Vec<String> {
        let file = self.file.borrow();
        let entries = match &self.path {
            None => file.read_root_storage(),
            Some(path) => match file.read_storage(path) {
                Ok(entries) => entries,
                Err(_) => return Vec::new(),
            },
        };

        entries.map(|entry| entry.name().to_owned()).collect()
    }

    /// 在本存储的所有子项里按名字找出对应的路径。
    ///
    /// 用「枚举 + 按名匹配」而不是手工拼接路径，是为了让含控制字符的名字
    /// （例如 `\1CompObj`）也能正确解析。
    fn find_child_path(&self, name: &str) -> Option<PathBuf> {
        let file = self.file.borrow();
        let entries = match &self.path {
            None => file.read_root_storage(),
            Some(path) => file.read_storage(path).ok()?,
        };

        entries
            .filter(|entry| entry.name() == name)
            .map(|entry| entry.path().to_owned())
            .next()
    }

    /// 完整读取一个流的内容。
    fn read_stream(&self, path: &PathBuf) -> Option<Vec<u8>> {
        let mut file = self.file.borrow_mut();
        let mut stream = file.open_stream(path).ok()?;
        let mut data = Vec::new();
        stream.read_to_end(&mut data).ok()?;
        Some(data)
    }
}

/// 对应 POI: `DocumentEntry`（复合文档中的流/文档）
#[derive(Clone)]
pub struct DocumentEntry {
    name: String,
    data: Rc<Vec<u8>>,
}

impl DocumentEntry {
    /// 取流名。
    ///
    /// 对应 POI: `String getName()`
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// 取流的字节长度。
    ///
    /// 对应 POI: `int getSize()`
    pub fn get_size(&self) -> usize {
        self.data.len()
    }

    /// 取流的字节内容（共享句柄，不复制）。
    pub fn get_data(&self) -> Rc<Vec<u8>> {
        Rc::clone(&self.data)
    }
}

/// 对应 POI: `DocumentInputStream` —— 可读的字节流。
///
/// POI 侧构造方法是 `new DocumentInputStream(DocumentEntry)`；Rust 侧同样由
/// [`DocumentEntry`] 构造，在共享的字节缓冲上提供 `Read + Seek`。
///
/// 实现上没有直接用 `std::io::Cursor<Rc<Vec<u8>>>`，因为 `Cursor<T>: Read` 要求
/// `T: AsRef<[u8]>`，而 `Rc<Vec<u8>>` 只实现 `AsRef<Vec<u8>>`，不满足该约束。
pub struct DocumentInputStream {
    data: Rc<Vec<u8>>,
    position: usize,
}

impl DocumentInputStream {
    /// 对应 POI: `new DocumentInputStream(DocumentEntry entry)`
    pub fn new(entry: &DocumentEntry) -> Self {
        Self {
            data: Rc::clone(&entry.data),
            position: 0,
        }
    }

    /// 直接由字节构造（Rust 特有，便于测试与内部复用）。
    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self {
            data: Rc::new(data),
            position: 0,
        }
    }

    /// 取流的剩余长度。
    ///
    /// 对应 POI: `int available()`
    pub fn available(&self) -> usize {
        self.data.len().saturating_sub(self.position)
    }
}

impl Read for DocumentInputStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let start = self.position.min(self.data.len());
        let remaining = &self.data[start..];
        let count = remaining.len().min(buf.len());
        buf[..count].copy_from_slice(&remaining[..count]);
        self.position = start + count;
        Ok(count)
    }
}

impl Seek for DocumentInputStream {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let len = self.data.len() as i64;
        let target = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::End(offset) => len + offset,
            SeekFrom::Current(offset) => self.position as i64 + offset,
        };

        self.position = usize::try_from(target.max(0)).unwrap_or(usize::MAX);
        Ok(self.position as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    /// 构造一个最小可用的复合文档（供本模块与 mpp 包测试复用）。
    fn sample_compound_file() -> Vec<u8> {
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).unwrap();

        file.create_storage("/   114").unwrap();
        file.create_storage("/   114/TBkndTask").unwrap();
        file.create_stream("/   114/TBkndTask/VarMeta")
            .unwrap()
            .write_all(b"varmeta-bytes")
            .unwrap();
        file.create_stream("/   114/TBkndTask/Var2Data")
            .unwrap()
            .write_all(b"var2data-bytes")
            .unwrap();
        file.create_stream("/\u{1}CompObj")
            .unwrap()
            .write_all(b"compobj")
            .unwrap();

        file.into_inner().into_inner()
    }

    #[test]
    fn reads_a_stream_through_a_directory() {
        let bytes = sample_compound_file();
        let fs = PoifsFileSystem::new(Cursor::new(bytes)).unwrap();
        let root = fs.get_root();

        let project = root.get_directory("   114").expect("应能找到项目存储");
        let task = project.get_directory("TBkndTask").expect("应能找到任务存储");
        let document = task.get_document("VarMeta").expect("应能找到 VarMeta 流");

        assert_eq!(document.get_name(), "VarMeta");
        assert_eq!(document.get_size(), 13);

        let mut stream = DocumentInputStream::new(&document);
        let mut text = String::new();
        stream.read_to_string(&mut text).unwrap();
        assert_eq!(text, "varmeta-bytes");
        assert_eq!(stream.available(), 0);
    }

    #[test]
    fn entry_names_include_control_character_names() {
        let bytes = sample_compound_file();
        let fs = PoifsFileSystem::new(Cursor::new(bytes)).unwrap();

        let names = fs.get_root().get_entry_names();
        assert!(names.contains(&"   114".to_owned()));
        assert!(names.contains(&"\u{1}CompObj".to_owned()));
        assert!(fs.exists("\u{1}CompObj"));
    }

    #[test]
    fn missing_entries_return_none() {
        let bytes = sample_compound_file();
        let fs = PoifsFileSystem::new(Cursor::new(bytes)).unwrap();
        let root = fs.get_root();

        assert!(root.get_entry("NOT_THERE").is_none());
        // `   114` 是存储而非流，反之亦然
        assert!(root.get_document("   114").is_none());
        assert!(root.get_directory("\u{1}CompObj").is_none());
    }
}
