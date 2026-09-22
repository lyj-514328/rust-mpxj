//! Corresponds to Java: `org/mpxj/reader/AbstractProjectFileReader.java`
//!
//! Abstract implementation for readers that read "file by file": this layer converts
//! "read by stream" into "read via a temporary file", and provides the default `readAll`
//! behavior (containing only a single schedule).
//!
//! # Java inheritance chain → Rust trait default methods
//!
//! Java: `public abstract class AbstractProjectFileReader extends AbstractProjectReader`.
//! Rust has no inheritance, so [`AbstractProjectFileReader`] uses
//! [`AbstractProjectReader`] as a supertrait and writes this class's implementation of the
//! `ProjectReader` interface methods as trait **default methods**; the default method names get
//! a `_default` suffix to avoid method resolution ambiguity with the same-named methods of
//! [`ProjectReader`] (the full explanation of the convention and the wiring template for a
//! concrete reader are in the file header of [`crate::reader::abstract_project_reader`]).
//!
//! # Java → Rust method mapping
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `ProjectFile read(String fileName)` | (no separate code) | Only adapts `new File(fileName)`; on the Rust side it is merged with `read(File)` into [`ProjectReader::read_file`] (`&Path` can be obtained directly from `&str`/`PathBuf`) |
//! | `List<ProjectFile> readAll(String fileName)` | (no separate code) | Same as above; merged into [`ProjectReader::read_all_file`] |
//! | `ProjectFile read(File file)` | (abstract, no default implementation) | Implemented by the concrete reader as [`ProjectReader::read_file`]; this layer provides no implementation (matching Java's `abstract`) |
//! | `List<ProjectFile> readAll(File file)` | [`AbstractProjectFileReader::read_all_file_default`] | A wrapper around `read(file)` |
//! | `ProjectFile read(InputStream inputStream)` | [`AbstractProjectFileReader::read_reader_default`] | Writes to a temporary file first, then goes through `read(File)` |
//! | `List<ProjectFile> readAll(InputStream inputStream)` | [`AbstractProjectFileReader::read_all_reader_default`] | Same as above, then goes through `readAll(File)` |
//!
//! # Handling `java.io.File` / `InputStream`
//!
//! - **`File` → [`std::path::Path`]**: Java's `new File(fileName)` does no existence check
//!   (validation is deferred to the actual file open), and on the Rust side [`std::path::Path`]
//!   is likewise just a path value, so the semantics match; actually opening the file is the
//!   concrete reader's responsibility (for example via
//!   [`crate::reader::abstract_project_stream_reader::AbstractProjectStreamReader::read_file_default`],
//!   where a failure of [`std::fs::File::open`] is converted into [`crate::MpxjError`] via
//!   `From<std::io::Error>`).
//! - **`file.toURI().toURL()` and the like**: this class has no such usage (confirmed from the
//!   Java source), so no corresponding implementation is needed.
//! - **Temporary file**: `InputStreamHelper.writeStreamToTempFile(inputStream, "tmp")`
//!   corresponds to this module's [`write_stream_to_temp_file`]
//!   (`Files.createTempFile("mpxj", "tmp")` → a file under `std::env::temp_dir()` named by
//!   process ID + nanosecond timestamp); `FileHelper.deleteQuietly(tempFile)` corresponds to
//!   [`delete_quietly`].
//! - **Closing in `finally`**: Java uses `AutoCloseableHelper.closeQuietly` / `finally` to ensure
//!   release; on the Rust side the temporary file handle is closed automatically by [`Drop`]
//!   when it goes out of scope (and is explicitly `flush`ed so that write errors surface).
//! - **`throws IOException` → `MpxjError`**: Java wraps `IOException` as
//!   `new MPXJException("Failed to read file", ex)`; on the Rust side
//!   [`write_stream_to_temp_file`] produces an error with the same text prefix when an I/O
//!   failure occurs ([`crate::MpxjError::with_cause`] appends the underlying I/O error text to
//!   the message).
//!
//! # `null` return values
//!
//! Java's `readAll(File)` writes `projectFile == null ? emptyList() : singletonList(projectFile)`.
//! On the Rust side `read_file` expresses failure with `Result` (failure means `Err`), and on
//! success it is necessarily some `Rc<RefCell<ProjectFile>>`, so the default implementation
//! always returns a single-element list and the null-check branch disappears.
//!
//! # Skipped Java code and rationale
//!
//! - `read(String)` / `readAll(String)`: pure parameter adaptation (`String` → `File`), already
//!   merged on the Rust side into the path-based methods (see the table above); there is no
//!   separate code to translate;
//! - this class does not use reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`,
//!   nor does it reference `UniversalProjectReader` / `ProjectReaderUtility`, which rely on
//!   reflection-based dispatch and are not being ported.

use std::cell::RefCell;
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_file::ProjectFile;
use crate::reader::abstract_project_reader::AbstractProjectReader;
use crate::reader::project_reader::ProjectReader;

/// Abstract implementation for readers that read "file by file".
///
/// Corresponds to Java: `public abstract class AbstractProjectFileReader extends AbstractProjectReader`
///
/// The method that is still "abstract" in this layer is [`ProjectReader::read_file`]
/// (Java `read(File)`), which the concrete reader implements.
pub trait AbstractProjectFileReader: AbstractProjectReader {
    /// Read all schedules by path (this layer's format contains only a single schedule, so a
    /// single-element list is returned).
    ///
    /// Corresponds to Java: `@Override public List<ProjectFile> readAll(File file)`
    ///
    /// (Java's `readAll(String fileName)` only adapts `new File(fileName)` and is merged into this method.)
    fn read_all_file_default(&mut self, path: &Path) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
        // Java: ProjectFile projectFile = read(file);
        let project = ProjectReader::read_file(self, path)?;

        // Java: return projectFile == null ? Collections.emptyList() : Collections.singletonList(projectFile);
        Ok(vec![project])
    }

    /// Read a single schedule from the reader: write to a temporary file first, then read by path.
    ///
    /// Corresponds to Java: `@Override public ProjectFile read(InputStream inputStream)`
    fn read_reader_default(
        &mut self,
        reader: &mut dyn Read,
    ) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        // Java: tempFile = InputStreamHelper.writeStreamToTempFile(inputStream, "tmp");
        let temp_file = write_stream_to_temp_file(reader, "tmp")?;

        // Java: return read(tempFile);
        let result = ProjectReader::read_file(self, &temp_file);

        // Java: finally { FileHelper.deleteQuietly(tempFile); }
        delete_quietly(&temp_file);

        result
    }

    /// Read all schedules from the reader: write to a temporary file first, then read by path.
    ///
    /// Corresponds to Java: `@Override public List<ProjectFile> readAll(InputStream inputStream)`
    fn read_all_reader_default(
        &mut self,
        reader: &mut dyn Read,
    ) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
        // Java: tempFile = InputStreamHelper.writeStreamToTempFile(inputStream, "tmp");
        let temp_file = write_stream_to_temp_file(reader, "tmp")?;

        // Java: return readAll(tempFile);
        let result = self.read_all_file_default(&temp_file);

        // Java: finally { FileHelper.deleteQuietly(tempFile); }
        delete_quietly(&temp_file);

        result
    }
}

/// Write all data from the reader into a new temporary file and return the file path.
///
/// Corresponds to Java: `InputStreamHelper.writeStreamToTempFile(InputStream inputStream, String tempFileSuffix)`
/// (internally `Files.createTempFile("mpxj", suffix)` + `writeInputStreamToOutputStream`, the
/// latter equivalent to [`io::copy`]); on I/O failure it corresponds to Java's
/// `throw new MPXJException("Failed to read file", ex)`.
fn write_stream_to_temp_file(reader: &mut dyn Read, suffix: &str) -> MpxjResult<PathBuf> {
    let path = temp_file_path(suffix);

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| MpxjError::with_cause("Failed to read file", &error))?;

    match io::copy(reader, &mut file).and_then(|_| file.flush()) {
        Ok(_) => Ok(path),
        Err(error) => {
            // Java: on write failure the temp file has not yet been assigned to tempFile, so finally does not delete it (the Rust side cleans it up here)
            delete_quietly(&path);
            Err(MpxjError::with_cause("Failed to read file", &error))
        }
    }
}

/// Generate a temporary file path (corresponds to `Files.createTempFile("mpxj", suffix)`).
///
/// Java relies on `Files.createTempFile` to guarantee the new file is unique; the Rust side
/// names it by "process ID + nanosecond timestamp", combined with the caller's `create_new` to
/// ensure existing files are not overwritten.
fn temp_file_path(suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();

    // Java: Files.createTempFile("mpxj", tempFileSuffix) -- prefix "mpxj", suffix appended directly after the unique part
    std::env::temp_dir().join(format!("mpxj-{}-{}{}", std::process::id(), nanos, suffix))
}

/// Delete a file quietly (no error is reported on failure).
///
/// Corresponds to Java: `FileHelper.deleteQuietly(File file)`
/// (this handles only regular files; the Java version also recursively deletes directories, but
/// the call sites in this class only involve temporary files.)
fn delete_quietly(path: &Path) {
    let _ = std::fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::listener::project_listener::ProjectListener;
    use crate::reader::abstract_project_reader::SharedProjectListener;

    /// Minimal file-based reader: `read_file` records the received path and returns a project or
    /// an error as needed.
    #[derive(Default)]
    struct StubFileReader {
        listeners: Vec<SharedProjectListener>,
        /// The path received by the most recent `read_file`.
        last_path: Option<PathBuf>,
        /// The contents of that path at the most recent `read_file` (used to verify the temporary
        /// file was fully written).
        last_contents: Option<Vec<u8>>,
    }

    impl ProjectReader for StubFileReader {
        fn add_project_listener(&mut self, listener: Box<dyn ProjectListener>) {
            self.add_project_listener_default(listener);
        }

        fn read_file(&mut self, path: &Path) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
            self.last_path = Some(path.to_path_buf());
            self.last_contents = std::fs::read(path).ok();
            Ok(ProjectFile::new())
        }

        fn read_all_file(&mut self, path: &Path) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
            self.read_all_file_default(path)
        }

        fn read_reader(&mut self, reader: &mut dyn Read) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
            self.read_reader_default(reader)
        }

        fn read_all_reader(
            &mut self,
            reader: &mut dyn Read,
        ) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
            self.read_all_reader_default(reader)
        }
    }

    impl AbstractProjectReader for StubFileReader {
        fn project_listeners(&self) -> &[SharedProjectListener] {
            &self.listeners
        }

        fn project_listeners_mut(&mut self) -> &mut Vec<SharedProjectListener> {
            &mut self.listeners
        }
    }

    impl AbstractProjectFileReader for StubFileReader {}

    #[test]
    fn read_all_file_default_wraps_a_single_project() {
        let mut reader = StubFileReader::default();

        let projects = reader
            .read_all_file_default(Path::new("plan.mpp"))
            .expect("stub always succeeds");

        assert_eq!(projects.len(), 1);
        assert_eq!(reader.last_path.as_deref(), Some(Path::new("plan.mpp")));
    }

    #[test]
    fn read_reader_default_goes_through_a_temp_file_and_cleans_up() {
        let mut reader = StubFileReader::default();

        let project = reader
            .read_reader_default(&mut &b"fake schedule"[..])
            .expect("stub always succeeds");

        // The returned project is indeed the one the stub created (registered in its own context)
        assert_eq!(
            project.borrow().get_project_context().borrow().get_projects().len(),
            1
        );

        // The supplied reader contents are fully written to the temporary file
        assert_eq!(reader.last_contents.as_deref(), Some(&b"fake schedule"[..]));

        // The temp file is in the system temp directory, has the same "tmp" suffix as the Java side, and is deleted after the call
        let temp_path = reader.last_path.clone().expect("path received");
        assert!(temp_path.starts_with(std::env::temp_dir()));
        assert!(temp_path.to_string_lossy().ends_with("tmp"));
        assert!(!temp_path.exists());
    }

    #[test]
    fn read_all_reader_default_wraps_a_single_project() {
        let mut reader = StubFileReader::default();

        let projects = reader
            .read_all_reader_default(&mut &b"fake schedule"[..])
            .expect("stub always succeeds");

        assert_eq!(projects.len(), 1);
        // Also goes through a temporary file
        let temp_path = reader.last_path.clone().expect("path received");
        assert!(!temp_path.exists());
    }

    #[test]
    fn write_stream_to_temp_file_reports_read_failures() {
        /// A reader that always fails on read.
        struct FailingReader;

        impl Read for FailingReader {
            fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::Other, "boom"))
            }
        }

        let error = write_stream_to_temp_file(&mut FailingReader, "tmp")
            .expect_err("write failure should error");

        // Java: throw new MPXJException("Failed to read file", ex)
        assert!(error.message().starts_with("Failed to read file"));
    }
}
