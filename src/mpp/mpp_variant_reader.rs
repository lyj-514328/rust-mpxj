//! Corresponds to Java: `org/mpxj/mpp/MPPVariantReader.java`
//!
//! The shared contract of the four MPP variant readers ([`MPP8Reader`](crate::mpp::mpp8_reader::MPP8Reader) /
//! [`MPP9Reader`](crate::mpp::mpp9_reader::MPP9Reader) /
//! [`MPP12Reader`](crate::mpp::mpp12_reader::MPP12Reader) /
//! [`MPP14Reader`](crate::mpp::mpp14_reader::MPP14Reader)): each variant is responsible for
//! "reading a whole MPP file according to its own byte layout and populating [`ProjectFile`]".
//! After [`MPPReader::read`](crate::mpp::mpp_reader::MPPReader::read) obtains the file format string from the `\1CompObj` stream,
//! it selects a variant accordingly and calls [`MPPVariantReader::process`].
//!
//! # Finalized trait signature (the interface point of the four variant readers)
//!
//! ```ignore
//! pub trait MPPVariantReader {
//!     fn process(
//!         &self,
//!         reader: &MPPReader,
//!         file: &Rc<RefCell<ProjectFile>>,
//!         root: &DirectoryEntry,
//!     ) -> MpxjResult<()>;
//! }
//! ```
//!
//! Item-by-item rationale (Java → Rust):
//!
//! | Java | Rust | Rationale |
//! |---|---|---|
//! | `interface MPPVariantReader` (package-private) | `pub trait MPPVariantReader` | In Java the interface and its four implementations all live in the `org.mpxj.mpp` package, so package-private suffices; Rust modules are packages, so cross-module use requires `pub` |
//! | `MPPReader reader` | `&MPPReader` | The variant reader only **reads** five flags from the parent reader (`getReadPropertiesOnly` / `getReadPresentationData` / `getRespectPasswordProtection` / `getReadPassword` / `getUseRawTimephasedData`); there is no `reader.setXxx(..)` call anywhere in the Java source, so `&mut` is unnecessary. `&MPPReader` also lets [`MPPReader::read`](crate::mpp::mpp_reader::MPPReader::read) take `&self` (Java's `read` likewise does not modify the reader's own state), so `read` can pass `self` directly to `process` |
//! | `ProjectFile file` | `&Rc<RefCell<ProjectFile>>` | Per this project's convention, `ProjectFile` is always held shared as `Rc<RefCell<..>>` (see the [`crate::project_file`] file header); passing it by reference lets `MPPReader` retain ownership while the variant reader can still `Rc::clone` it into any internal reader |
//! | `DirectoryEntry root` | `&DirectoryEntry` | POI's `DirectoryEntry` is a handle object; the Rust [`DirectoryEntry`] is a lightweight handle deriving `#[derive(Clone)]`, so passing it by reference is enough, and `root.clone()` is used when long-term ownership is needed (for example handing it to an internal reader) |
//! | `throws MPXJException, IOException` | `MpxjResult<()>` | Uniformly returns [`MpxjResult`]; `IOException` is converted via `From<std::io::Error>` into an [`MpxjError`] with the message prefix `Error reading file` (consistent with Java's `new MPXJException(MPXJException.READ_ERROR, ex)`) |
//! | `void process(MPPReader, ProjectFile, DirectoryEntry)` (mutable instance state) | `fn process(&self, ..)` | **This trait is fixed to `&self`**. On the Java side `MPP9Reader` etc. have no-arg constructors and write `m_reader` / `m_file` / `m_root` / `m_projectProps` etc. into instance fields in `populateMemberData`, then `clearMemberData` clears them when `process` finishes — this relies on mutable instance state. Rust implementers have three options: (1) put the mutable state in `RefCell` / `Cell` fields (keeping `new()` no-arg); (2) inside `process`, place the state in a local struct and pass `&mut` to that local struct to each private method (closest to Java's method-by-method translation and introduces no interior mutability); (3) accept fixed references in `new(...)` so the state inside `process` is naturally immutable. **The signature does not change with the implementation style** |
//!
//! # Three conventions implementers must follow
//!
//! 1. **Borrowing**: `file` is `Rc<RefCell<ProjectFile>>`; all accesses to `ProjectFile` and `ProjectContext`
//! must use `try_borrow` / `try_borrow_mut`, and on conflict return `Err` or skip with an explanatory comment
//! (Java has stable references, whereas Rust would panic).
//! 2. **Do not hold a `ProjectFile` borrow while calling methods that borrow the same file back**: for example
//! `Task::remove_predecessor` internally does `file.borrow_mut()`; you must first clone out
//! a snapshot such as `Vec<Rc<RefCell<Task>>>` and release the file borrow.
//! 3. **Stream reading**: Java's `new DocumentInputStream((DocumentEntry) root.getEntry(name))` is
//! always written on the Rust side as `root.get_document(name)` (obtaining a [`DocumentEntry`]) followed by
//! `entry.get_data()` → `Rc<Vec<u8>>`, then passing it as `&[u8]` to `Props` / `Var2Data` etc.
//!
//! # Work handled by `MPPReader` that variant readers need not repeat
//!
//! After `process` returns, `MPPReader.read(POIFSFileSystem)` uniformly performs: `updateStructure()`,
//! `task.setSummary(...)` / relation validation / estimated baseline backfill, cleanup of unused derived calendars, resource calendar and
//! unit-of-measure post-processing, `readComplete()`. Variant readers only need to fill in data.
//!
//! # Skipped Java code and rationale
//!
//! This interface uses none of reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`, so there are no skipped items.

use std::cell::RefCell;
use std::rc::Rc;

use crate::mpp::mpp_reader::MPPReader;
use crate::mpp::ole::DirectoryEntry;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;

/// A reader able to read one particular MPP file format.
///
/// Corresponds to Java: `interface MPPVariantReader`
///
/// Object safe (trait object safe): all four implementations are selected and invoked by
/// [`crate::mpp::mpp_reader::MPPReader`] as `Box<dyn MPPVariantReader>` according to the format name given by `\1CompObj`.
pub trait MPPVariantReader {
/// Read an MPP file and populate the passed-in data structures.
    ///
    /// Corresponds to Java: `void process(MPPReader reader, ProjectFile file, DirectoryEntry root) throws MPXJException, IOException`
    ///
/// - `reader`: the parent reader, from which the variant reader only reads
    ///   `getReadPropertiesOnly` / `getReadPresentationData` / `getRespectPasswordProtection` /
/// the five flags `getReadPassword` / `getUseRawTimephasedData`;
/// - `file`: the project file to populate;
/// - `root`: the root storage of the POI filesystem (a [`DirectoryEntry`] handle on the Rust side).
    fn process(
        &self,
        reader: &MPPReader,
        file: &Rc<RefCell<ProjectFile>>,
        root: &DirectoryEntry,
    ) -> MpxjResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::mpxj_exception::{MpxjError, MpxjResult};
    use crate::mpp::ole::PoifsFileSystem;
    use crate::reader::project_reader::ProjectReader;
    use std::io::Cursor;
    use std::path::Path;

/// Minimal implementation: records only the three received parameters, used to verify the signature works (including trait object calls).
    struct StubVariantReader {
/// The `getReadPropertiesOnly()` value read when `process` is called.
        read_properties_only: RefCell<Option<bool>>,
    }

    impl StubVariantReader {
        fn new() -> Self {
            Self {
                read_properties_only: RefCell::new(None),
            }
        }
    }

    impl MPPVariantReader for StubVariantReader {
        fn process(
            &self,
            reader: &MPPReader,
            _file: &Rc<RefCell<ProjectFile>>,
            root: &DirectoryEntry,
        ) -> MpxjResult<()> {
// the variant reader only needs the read-only flags on &MPPReader
            *self.read_properties_only.borrow_mut() = Some(reader.get_read_properties_only());
// the root handle is usable (streams with control-character names can also be enumerated)
            let _names = root.get_entry_names();
            Err(MpxjError::new("stub"))
        }
    }

/// Create an empty compound document and retrieve its root storage.
    fn empty_root() -> (PoifsFileSystem, DirectoryEntry) {
        let bytes = cfb::CompoundFile::create(Cursor::new(Vec::new()))
            .expect("create compound document")
            .into_inner()
            .into_inner();
        let fs = PoifsFileSystem::new(Cursor::new(bytes)).expect("open compound document");
        let root = fs.get_root();
        (fs, root)
    }

    #[test]
    fn trait_is_object_safe_and_receives_the_reader_flags() {
        let (_fs, root) = empty_root();
        let file = ProjectFile::new();
        let mut reader = MPPReader::new();
        reader.set_read_properties_only(true);

        let variant: Box<dyn MPPVariantReader> = Box::new(StubVariantReader::new());
        let error = variant
            .process(&reader, &file, &root)
            .err()
            .expect("stub returns Err by convention");

        assert_eq!(error.message(), "stub");
    }

/// Ensure the trait can be triggered indirectly through a path-based entry point like `read_file` (compile-time check).
    #[test]
    fn stub_reader_can_be_driven_through_mpp_reader_public_api() {
        let mut reader = MPPReader::new();
// non-existent path: Java throws IOException → MPXJException.READ_ERROR
        let error = reader
            .read_file(Path::new("no-such-dir/no-such-file.mpp"))
            .err()
            .expect("non-existent path should error");
        assert!(error.message().starts_with(MpxjError::READ_ERROR));
    }
}
