//! Corresponds to Java: `org/mpxj/reader/ProjectReader.java`
//!
//! All readers that can read a project file and produce a [`ProjectFile`] from it implement
//! this trait.
//!
//! # Java to Rust method mapping
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `void addProjectListener(ProjectListener listener)` | [`ProjectReader::add_project_listener`] | The listener is transferred with ownership as `Box<dyn ProjectListener>` |
//! | `ProjectFile read(String fileName)` | [`ProjectReader::read_file`] | Merged with `read(File)` |
//! | `ProjectFile read(File file)` | [`ProjectReader::read_file`] | Same as above (Rust has no overloading; the `String` / `File` overloads are merged into a single path-based form) |
//! | `List<ProjectFile> readAll(String fileName)` | [`ProjectReader::read_all_file`] | Merged with `readAll(File)` |
//! | `List<ProjectFile> readAll(File file)` | [`ProjectReader::read_all_file`] | Same as above |
//! | `ProjectFile read(InputStream inputStream)` | [`ProjectReader::read_reader`] | `InputStream` → `&mut dyn std::io::Read` (changed to a single reader-based form) |
//! | `List<ProjectFile> readAll(InputStream inputStream)` | [`ProjectReader::read_all_reader`] | Same as above |
//!
//! # Restoring the inheritance chain and interface
//!
//! The Java inheritance chain is
//! `ProjectReader` (interface) ← `AbstractProjectReader` ← `AbstractProjectFileReader` /
//! `AbstractProjectStreamReader` ← concrete readers (`MPXReader`, `MPPReader`, etc.).
//! Rust has no inheritance; the mapping is:
//!
//! - This file: interface → trait [`ProjectReader`], which only declares methods and provides
//!   no default implementation (matching the Java interface);
//! - [`crate::reader::abstract_project_reader`]: the trait
//!   [`crate::reader::abstract_project_reader::AbstractProjectReader`] uses `ProjectReader` as a
//!   supertrait and carries the common implementation of the Java abstract base class (default methods);
//! - [`crate::reader::abstract_project_file_reader`] /
//!   [`crate::reader::abstract_project_stream_reader`]: these use `AbstractProjectReader` as a
//!   supertrait and provide the file-based and stream-based default implementations respectively;
//! - a concrete reader both `impl ProjectReader` (forwarding the interface methods to the
//!   corresponding `_default` methods of the abstract layer, or providing its own implementation)
//!   and implements each abstract-layer trait. The naming convention and template are described
//!   in the file header of [`crate::reader::abstract_project_reader`].
//!
//! # Differences from Java
//!
//! - **`throws MPXJException` → [`crate::MpxjResult`]**: all read methods return
//!   `MpxjResult<..>`; Java's `MPXJException` corresponds to [`crate::MpxjError`].
//! - **Semantics of returning `null`**: Java's `read(..)` may return `null`
//!   (`AbstractProjectStreamReader.readAll` returns an empty list or a single-element list
//!   depending on whether `read` returns `null`). Rust has no `null`: a read failure is expressed
//!   as `Err`, and the degraded path where "nothing can be read" is expressed by each abstract
//!   layer's default method as an empty `Vec` (see those file headers).
//! - **`List<ProjectFile>` → `Vec<Rc<RefCell<ProjectFile>>>`**: on the Rust side [`ProjectFile`]
//!   is always held shared as `Rc<RefCell<ProjectFile>>` (see the file header of
//!   [`crate::project_file`] for the reason).
//! - **Object safety**: all methods take `&mut self` and parameters `&Path` / `&mut dyn Read`,
//!   so this trait is object safe and can be written as `&mut dyn ProjectReader`
//!   (`AbstractProjectReader::add_listeners_to_reader` relies on this).
//!
//! # Skipped Java code and rationale
//!
//! This interface does not use reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`,
//! nor does it reference `UniversalProjectReader` / `ProjectReaderUtility`, which rely on
//! reflection-based dispatch and are not being ported, so there are no skipped items.

use std::cell::RefCell;
use std::io::Read;
use std::path::Path;
use std::rc::Rc;

use crate::listener::project_listener::ProjectListener;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;

/// A reader that can read a project file and produce a [`ProjectFile`].
///
/// Corresponds to Java: `public interface ProjectReader`
pub trait ProjectReader {
    /// Register a listener: events produced during reading are forwarded to it.
    ///
    /// Corresponds to Java: `void addProjectListener(ProjectListener listener)`
    ///
    /// Java keeps a reference to the listener object; the Rust side transfers ownership as
    /// `Box<dyn ProjectListener>`, and the abstract layer then wraps it in a shared handle so it
    /// can be dispatched to multiple places (see the file header of
    /// [`crate::reader::abstract_project_reader`]).
    fn add_project_listener(&mut self, listener: Box<dyn ProjectListener>);

    /// Read a single schedule by path.
    ///
    /// Corresponds to Java: `ProjectFile read(String fileName)` / `ProjectFile read(File file)`
    /// (Rust has no overloading; the two overloads are merged into this method).
    fn read_file(&mut self, path: &Path) -> MpxjResult<Rc<RefCell<ProjectFile>>>;

    /// Read all schedules by path (returns a single-element list when the file format does not
    /// support multiple schedules).
    ///
    /// Corresponds to Java: `List<ProjectFile> readAll(String fileName)` /
    /// `List<ProjectFile> readAll(File file)`
    fn read_all_file(&mut self, path: &Path) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>>;

    /// Read a single schedule from the given reader.
    ///
    /// Corresponds to Java: `ProjectFile read(InputStream inputStream)`
    fn read_reader(&mut self, reader: &mut dyn Read) -> MpxjResult<Rc<RefCell<ProjectFile>>>;

    /// Read all schedules from the given reader.
    ///
    /// Corresponds to Java: `List<ProjectFile> readAll(InputStream inputStream)`
    fn read_all_reader(&mut self, reader: &mut dyn Read)
    -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpxj_exception::MpxjError;

    /// Minimal implementation: only verifies that this trait is usable (including dynamic
    /// dispatch via `&mut dyn ProjectReader`).
    struct StubReader {
        /// The most recently received path, used to assert that the call was forwarded to this
        /// implementation.
        last_path: Option<std::path::PathBuf>,
    }

    impl ProjectReader for StubReader {
        fn add_project_listener(&mut self, _listener: Box<dyn ProjectListener>) {}

        fn read_file(&mut self, path: &Path) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
            self.last_path = Some(path.to_path_buf());
            Ok(ProjectFile::new())
        }

        fn read_all_file(&mut self, path: &Path) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
            Ok(vec![self.read_file(path)?])
        }

        fn read_reader(&mut self, _reader: &mut dyn Read) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
            Err(MpxjError::new(MpxjError::INVALID_FILE))
        }

        fn read_all_reader(
            &mut self,
            reader: &mut dyn Read,
        ) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
            Ok(vec![self.read_reader(reader)?])
        }
    }

    #[test]
    fn trait_is_object_safe() {
        let mut reader = StubReader { last_path: None };

        {
            let dynamic: &mut dyn ProjectReader = &mut reader;
            dynamic
                .read_file(Path::new("plan.mpp"))
                .expect("stub always succeeds");
            let error = dynamic
                .read_reader(&mut std::io::empty())
                .err()
                .expect("stub reports INVALID_FILE by convention");
            assert_eq!(error.message(), MpxjError::INVALID_FILE);
        }

        assert_eq!(reader.last_path.as_deref(), Some(Path::new("plan.mpp")));
    }
}
