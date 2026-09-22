//! OLE2 compound document access layer (Rust-specific).
//!
//! MPXJ's `org.mpxj.mpp` package uses Apache POI's `POIFSFileSystem` /
//! `DirectoryEntry` / `DocumentEntry` / `DocumentInputStream` to access MPP files.
//! The Rust side uses the [`cfb`](https://crates.io/crates/cfb) crate instead; this module packages the differences
//! into a set of types **with the same names** as POI, keeping the mpp package translation 1:1:
//!
//! | Apache POI | This module |
//! |---|---|
//! | `POIFSFileSystem` | [`PoifsFileSystem`] |
//! | `DirectoryEntry` | [`DirectoryEntry`] |
//! | `DocumentEntry` | [`DocumentEntry`] |
//! | `DocumentInputStream` | [`DocumentInputStream`] |
//!
//! # Differences from POI
//!
//! 1. **The input source is unified as `Box<dyn ReadSeek>`**: POI's overloads (`File` / `InputStream`)
//! are merged on the Rust side, avoiding the `PoifsFileSystem<R>` generic parameter spreading through the whole mpp package.
//! 2. **Documents are read entirely into a `Vec<u8>` on demand**: POI's `DocumentInputStream` is a lazy reader,
//! whereas `cfb`'s `open_stream` requires `&mut CompoundFile` and the returned `Stream` borrows the underlying
//! file, so it cannot be returned across a `RefCell` borrow boundary. MPP's consumers (`Props` / `Var2Data` /
//! `FixedData`, etc.) already read everything in one pass, so here the bytes are read directly and handed to [`DocumentInputStream`]
//! as a `Cursor`; the behavior is equivalent and the borrowing is simpler.
//! 3. **Lookup by name rather than path concatenation**: first enumerate the children of the current storage, match by name, then use the path carried by the child
//! to open the stream — this way names containing control characters such as `\1CompObj` do not break due to path concatenation.
//! 4. POI's `DirectoryEntry.getEntry(name)` throws for a nonexistent entry; the Rust side returns `None`
//! (use [`DirectoryEntry::get_entry_names`] to check for existence first).

use std::cell::RefCell;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::rc::Rc;

use cfb::CompoundFile;

/// An input source that is both readable and seekable.
///
/// Used to unify POI's two entry points `File` / `InputStream` into a single concrete type,
/// so that [`PoifsFileSystem`] and the whole mpp package do not have to carry a generic parameter.
pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

/// Corresponds to POI: `POIFSFileSystem`
pub struct PoifsFileSystem {
    file: Rc<RefCell<CompoundFile<Box<dyn ReadSeek>>>>,
}

impl PoifsFileSystem {
/// Opens the compound document from any readable and seekable input source.
    ///
/// Corresponds to POI: `new POIFSFileSystem(InputStream)` / `new POIFSFileSystem(File)`
    pub fn new<R: ReadSeek + 'static>(inner: R) -> std::io::Result<Self> {
        let file = CompoundFile::open(Box::new(inner) as Box<dyn ReadSeek>)?;
        Ok(Self {
            file: Rc::new(RefCell::new(file)),
        })
    }

/// Returns the root storage.
    ///
/// Corresponds to POI: `DirectoryEntry getRoot()`
    pub fn get_root(&self) -> DirectoryEntry {
        DirectoryEntry {
            file: Rc::clone(&self.file),
            path: None,
        }
    }

/// Tests whether a stream with the given name exists (to mirror POI's "check `getEntryNames()` first, then get" style).
    ///
/// Corresponds to POI: `root.getEntryNames().contains(name)`
    pub fn exists(&self, name: &str) -> bool {
        self.get_root().get_entry(name).is_some()
    }
}

/// An entry in the compound document: a storage (directory) or a stream (document).
///
/// Corresponds to the `getEntry()` return values of the two roles `DirectoryEntry` / `DocumentEntry` in POI.
pub enum Entry {
/// A child storage (directory).
    Directory(DirectoryEntry),
/// A stream (document).
    Document(DocumentEntry),
}

/// Corresponds to POI: `DirectoryEntry` (a storage/directory in the compound document)
#[derive(Clone)]
pub struct DirectoryEntry {
    file: Rc<RefCell<CompoundFile<Box<dyn ReadSeek>>>>,
/// `None` means the root storage (corresponding to POI's `fs.getRoot()`).
    path: Option<PathBuf>,
}

impl DirectoryEntry {
/// Returns the child entry with the given name; returns `None` when it does not exist.
    ///
/// Corresponds to POI: `Entry getEntry(String name)` (POI throws when not found; the Rust side returns `None`)
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

/// Returns the child storage (directory) with the given name; returns `None` when it does not exist or is not a directory.
    ///
/// Corresponds to POI: `(DirectoryEntry) directory.getEntry(name)`
    pub fn get_directory(&self, name: &str) -> Option<DirectoryEntry> {
        match self.get_entry(name) {
            Some(Entry::Directory(directory)) => Some(directory),
            _ => None,
        }
    }

/// Returns the stream with the given name; returns `None` when it does not exist or is not a stream.
    ///
/// Corresponds to POI: `(DocumentEntry) directory.getEntry(name)`
    pub fn get_document(&self, name: &str) -> Option<DocumentEntry> {
        match self.get_entry(name) {
            Some(Entry::Document(document)) => Some(document),
            _ => None,
        }
    }

/// Lists the names of all child entries under this storage.
    ///
/// Corresponds to POI: `Set<String> getEntryNames()`
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

/// Finds the corresponding path by name among all child entries of this storage.
    ///
/// "Enumerate + match by name" is used instead of manual path concatenation so that names containing control characters
/// (for example `\1CompObj`) can also be resolved correctly.
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

/// Reads the entire contents of a stream.
    fn read_stream(&self, path: &PathBuf) -> Option<Vec<u8>> {
        let mut file = self.file.borrow_mut();
        let mut stream = file.open_stream(path).ok()?;
        let mut data = Vec::new();
        stream.read_to_end(&mut data).ok()?;
        Some(data)
    }
}

/// Corresponds to POI: `DocumentEntry` (a stream/document in the compound document)
#[derive(Clone)]
pub struct DocumentEntry {
    name: String,
    data: Rc<Vec<u8>>,
}

impl DocumentEntry {
/// Returns the stream name.
    ///
/// Corresponds to POI: `String getName()`
    pub fn get_name(&self) -> &str {
        &self.name
    }

/// Returns the byte length of the stream.
    ///
/// Corresponds to POI: `int getSize()`
    pub fn get_size(&self) -> usize {
        self.data.len()
    }

/// Returns the byte contents of the stream (shared handle, no copy).
    pub fn get_data(&self) -> Rc<Vec<u8>> {
        Rc::clone(&self.data)
    }
}

/// Corresponds to POI: `DocumentInputStream` — a readable byte stream.
///
/// POI's constructor is `new DocumentInputStream(DocumentEntry)`; the Rust side is likewise constructed from
/// [`DocumentEntry`], providing `Read + Seek` over a shared byte buffer.
///
/// The implementation does not directly use `std::io::Cursor<Rc<Vec<u8>>>`, because `Cursor<T>: Read` requires
/// `T: AsRef<[u8]>`, while `Rc<Vec<u8>>` only implements `AsRef<Vec<u8>>`, which does not satisfy that bound.
pub struct DocumentInputStream {
    data: Rc<Vec<u8>>,
    position: usize,
}

impl DocumentInputStream {
/// Corresponds to POI: `new DocumentInputStream(DocumentEntry entry)`
    pub fn new(entry: &DocumentEntry) -> Self {
        Self {
            data: Rc::clone(&entry.data),
            position: 0,
        }
    }

/// Constructs directly from bytes (Rust-specific, for testing and internal reuse).
    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self {
            data: Rc::new(data),
            position: 0,
        }
    }

/// Returns the remaining length of the stream.
    ///
/// Corresponds to POI: `int available()`
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

/// Builds a minimally usable compound document (for reuse in this module's and the mpp package's tests).
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

        let project = root.get_directory("   114").expect("should find project storage");
        let task = project.get_directory("TBkndTask").expect("should find task storage");
        let document = task.get_document("VarMeta").expect("should find VarMeta stream");

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
// `   114` is a storage rather than a stream, and vice versa
        assert!(root.get_document("   114").is_none());
        assert!(root.get_directory("\u{1}CompObj").is_none());
    }
}
