//! Corresponds to Java: `org/mpxj/reader/AbstractProjectStreamReader.java`
//!
//! Abstract implementation for readers that read "stream by stream": this layer converts
//! "read by file" into "open the file to obtain an input stream, then read by stream", and
//! provides the default `readAll` behavior (containing only a single schedule).
//!
//! # Java inheritance chain → Rust trait default methods
//!
//! Java: `public abstract class AbstractProjectStreamReader extends AbstractProjectReader`.
//! Rust has no inheritance, so [`AbstractProjectStreamReader`] uses
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
//! | `ProjectFile read(String fileName)` | (no separate code) | Only adapts `new File(fileName)`; on the Rust side it is merged with `read(File)` into [`ProjectReader::read_file`] |
//! | `List<ProjectFile> readAll(String fileName)` | (no separate code) | Same as above; merged into [`ProjectReader::read_all_file`] |
//! | `ProjectFile read(File file)` | [`AbstractProjectStreamReader::read_file_default`] | Opens the file and forwards to `read(InputStream)` |
//! | `List<ProjectFile> readAll(File file)` | [`AbstractProjectStreamReader::read_all_file_default`] | Opens the file and forwards to `readAll(InputStream)` |
//! | `ProjectFile read(InputStream inputStream)` | (abstract, no default implementation) | Implemented by the concrete reader as [`ProjectReader::read_reader`] |
//! | `List<ProjectFile> readAll(InputStream inputStream)` | [`AbstractProjectStreamReader::read_all_reader_default`] | A wrapper around `read(inputStream)` |
//!
//! # Handling `InputStream`
//!
//! - **`FileInputStream` → [`std::fs::File`]**: Java throws `IOException` when opening the file
//!   fails, which is caught and wrapped as `new MPXJException(MPXJException.READ_ERROR, ex)`; on
//!   the Rust side a failure of [`std::fs::File::open`] is converted into `MpxjError` via
//!   `From<std::io::Error>` (the message prefix is the text of `MPXJException.READ_ERROR`) and
//!   can be `?`-propagated directly. Situations such as a missing path are reported uniformly
//!   here (Java's `read(String)` / `read(File)` likewise do no path validation and defer it to
//!   open time).
//! - **`fis.close()` and `finally { AutoCloseableHelper.closeQuietly(fis); }`**: Java explicitly
//!   closes the `FileInputStream` it opened; on the Rust side [`std::fs::File`] is closed
//!   automatically by [`Drop`] when it goes out of scope, so no corresponding code is needed.
//!   Java does **not** close the caller-supplied `inputStream` (ownership of the
//!   `read(InputStream)` stream belongs to the caller), and the Rust side likewise only borrows
//!   `&mut dyn Read`.
//! - **`BufferedInputStream` / `mark` / `reset`**: this class does not use them (confirmed from
//!   the Java source: only `new FileInputStream(file)`, no buffered wrapper, no `mark` /
//!   `reset`), so this layer needs no replacement. However, a concrete reader (MPX / MPP)
//!   usually needs to "peek" at some bytes before deciding the branch, which Java implements
//!   with `InputStream.mark()` / `reset()`; Rust's [`std::io::Read`] has **no** `mark` /
//!   `reset`, and the replacements are:
//!
//!   1. When only peeking is needed and no rewind is required, use [`std::io::BufReader`]'s
//!      `fill_buf` / `consume` to observe and "pre-consume" the buffered content;
//!   2. When a real rewind is needed (equivalent to `mark` + `reset`), use this module's
//!      [`read_stream_into_cursor`] to read the whole stream into a `Vec<u8>` and wrap it as a
//!      [`std::io::Cursor`]: `Cursor` implements [`std::io::Seek`], `seek(SeekFrom::Start(n))`
//!      is equivalent to `reset()` back to the `mark` position, and `position()` is equivalent
//!      to the offset recorded at `mark` time. The cost is that the whole data set resides in
//!      memory (MPX/MPP files are usually small; compared with the default 8KB buffer of Java's
//!      `BufferedInputStream`, this trades memory for functionality).
//!
//! # `null` return values
//!
//! Java's `readAll(InputStream)` writes `file == null ? emptyList() : singletonList(file)`.
//! On the Rust side `read_reader` expresses failure with `Result` (failure means `Err`), and on
//! success it necessarily yields some `Rc<RefCell<ProjectFile>>`, so the default implementation
//! always returns a single-element list and the null-check branch disappears.
//!
//! # Skipped Java code and rationale
//!
//! - `read(String)` / `readAll(String)`: pure parameter adaptation (`String` → `File`), already
//!   merged into the path-based methods;
//! - `fis.close()` / `closeQuietly(fis)`: replaced by Rust's RAII close semantics (see above);
//! - this class does not use reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`,
//!   nor does it reference `UniversalProjectReader` / `ProjectReaderUtility`, which rely on
//!   reflection-based dispatch and are not being ported.

use std::cell::RefCell;
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;
use std::rc::Rc;

use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;
use crate::reader::abstract_project_reader::AbstractProjectReader;
use crate::reader::project_reader::ProjectReader;

/// Abstract implementation for readers that read "stream by stream".
///
/// Corresponds to Java: `public abstract class AbstractProjectStreamReader extends AbstractProjectReader`
///
/// The method that is still "abstract" in this layer is [`ProjectReader::read_reader`]
/// (Java `read(InputStream)`), which the concrete reader implements.
pub trait AbstractProjectStreamReader: AbstractProjectReader {
    /// Read a single schedule by path: open the file to obtain a reader, then read by stream.
    ///
    /// Corresponds to Java: `@Override public ProjectFile read(File file)`
    ///
    /// (Java's `read(String fileName)` only adapts `new File(fileName)` and is merged into this method.)
    fn read_file_default(&mut self, path: &Path) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        // Java: fis = new FileInputStream(file); catch (IOException ex) { throw new MPXJException(MPXJException.READ_ERROR, ex); }
        let mut file = File::open(path)?;

        // Java: ProjectFile projectFile = read(fis); fis.close(); return projectFile;
        // (fis.close() is handled by Rust's Drop)
        ProjectReader::read_reader(self, &mut file)
    }

    /// Read all schedules by path: open the file to obtain a reader, then read by stream.
    ///
    /// Corresponds to Java: `@Override public List<ProjectFile> readAll(File file)`
    ///
    /// (Java's `readAll(String fileName)` only adapts `new File(fileName)` and is merged into this method.)
    fn read_all_file_default(&mut self, path: &Path) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
        // Java: fis = new FileInputStream(file); catch (IOException ex) { throw new MPXJException(MPXJException.READ_ERROR, ex); }
        let mut file = File::open(path)?;

        // Java: List<ProjectFile> projectFiles = readAll(fis); fis.close(); return projectFiles;
        ProjectReader::read_all_reader(self, &mut file)
    }

    /// Read all schedules from the reader (this layer's format contains only a single schedule,
    /// so a single-element list is returned).
    ///
    /// Corresponds to Java: `@Override public List<ProjectFile> readAll(InputStream inputStream)`
    fn read_all_reader_default(
        &mut self,
        reader: &mut dyn Read,
    ) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
        // Java: ProjectFile file = read(inputStream);
        let file = ProjectReader::read_reader(self, reader)?;

        // Java: return file == null ? Collections.emptyList() : Collections.singletonList(file);
        Ok(vec![file])
    }
}

/// Read all data from the reader into memory and return a cursor that supports rewind (random access).
///
/// Rust-specific (Java has no corresponding method): Java's `InputStream.mark()` / `reset()`
/// have no counterpart in [`std::io::Read`], and [`std::io::Cursor`] together with
/// [`std::io::Seek`] is the replacement -- see the "`BufferedInputStream` / `mark` / `reset`"
/// section in the file header.
pub fn read_stream_into_cursor(reader: &mut dyn Read) -> MpxjResult<Cursor<Vec<u8>>> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    // Cursor implements Read + Seek: seek(SeekFrom::Start(n)) is equivalent to Java's reset()
    // (back to the mark position), and position() is equivalent to Java's mark record.
    Ok(Cursor::new(buffer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::listener::project_listener::ProjectListener;
    use crate::mpxj_exception::MpxjError;
    use crate::reader::abstract_project_reader::SharedProjectListener;
    use std::io::{Seek, SeekFrom, Write};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Minimal stream reader: `read_reader` consumes all reader contents and returns a new project.
    #[derive(Default)]
    struct StubStreamReader {
        listeners: Vec<SharedProjectListener>,
        /// All bytes received by the most recent `read_reader`.
        last_contents: Option<Vec<u8>>,
    }

    impl ProjectReader for StubStreamReader {
        fn add_project_listener(&mut self, listener: Box<dyn ProjectListener>) {
            self.add_project_listener_default(listener);
        }

        fn read_file(&mut self, path: &Path) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
            self.read_file_default(path)
        }

        fn read_all_file(&mut self, path: &Path) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
            self.read_all_file_default(path)
        }

        fn read_reader(&mut self, reader: &mut dyn Read) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
            let mut contents = Vec::new();
            reader.read_to_end(&mut contents)?;
            self.last_contents = Some(contents);
            Ok(ProjectFile::new())
        }

        fn read_all_reader(
            &mut self,
            reader: &mut dyn Read,
        ) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
            self.read_all_reader_default(reader)
        }
    }

    impl AbstractProjectReader for StubStreamReader {
        fn project_listeners(&self) -> &[SharedProjectListener] {
            &self.listeners
        }

        fn project_listeners_mut(&mut self) -> &mut Vec<SharedProjectListener> {
            &mut self.listeners
        }
    }

    impl AbstractProjectStreamReader for StubStreamReader {}

    /// Create a test file with deterministic contents and return its path.
    fn temp_file_with(contents: &[u8]) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "mpxj-stream-reader-test-{}-{}.tmp",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));

        let mut file = File::create(&path).expect("create test file");
        file.write_all(contents).expect("write test file");
        path
    }

    #[test]
    fn read_file_default_opens_the_file_and_delegates_to_the_stream_reader() {
        let path = temp_file_with(b"fake schedule");
        let mut reader = StubStreamReader::default();

        let project = reader.read_file_default(&path).expect("stub always succeeds");

        assert_eq!(reader.last_contents.as_deref(), Some(&b"fake schedule"[..]));
        assert_eq!(
            project
                .borrow()
                .get_project_context()
                .borrow()
                .get_projects()
                .len(),
            1
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn read_file_default_reports_missing_paths_as_read_errors() {
        let mut reader = StubStreamReader::default();

        let error = reader
            .read_file_default(Path::new("no-such-directory/no-such-file.mpp"))
            .err()
            .expect("non-existent path should error");

        // Java: catch (IOException ex) { throw new MPXJException(MPXJException.READ_ERROR, ex); }
        assert!(error.message().starts_with(MpxjError::READ_ERROR));
    }

    #[test]
    fn read_all_file_default_opens_the_file_and_returns_a_single_project() {
        let path = temp_file_with(b"fake schedule");
        let mut reader = StubStreamReader::default();

        let projects = reader.read_all_file_default(&path).expect("stub always succeeds");

        assert_eq!(projects.len(), 1);
        assert_eq!(reader.last_contents.as_deref(), Some(&b"fake schedule"[..]));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn read_all_reader_default_wraps_a_single_project() {
        let mut reader = StubStreamReader::default();

        let projects = reader
            .read_all_reader_default(&mut &b"fake schedule"[..])
            .expect("stub always succeeds");

        assert_eq!(projects.len(), 1);
        assert_eq!(reader.last_contents.as_deref(), Some(&b"fake schedule"[..]));
    }

    #[test]
    fn cursor_replaces_mark_and_reset() {
        let source = b"0123456789";
        let mut cursor = read_stream_into_cursor(&mut &source[..]).expect("read into memory");

        // Java: calls stream.mark(...) before int mark = stream.read()
        let mark = cursor.stream_position().expect("get current position");
        let mut head = [0u8; 4];
        cursor.read_exact(&mut head).expect("read first 4 bytes");
        assert_eq!(&head, b"0123");

        // Java: stream.reset() -- Rust: seek back to the offset recorded at mark
        cursor.seek(SeekFrom::Start(mark)).expect("seek back to mark");
        let mut again = [0u8; 4];
        cursor.read_exact(&mut again).expect("read first 4 bytes again");
        assert_eq!(&again, b"0123");

        // Continue reading the remaining content; the behavior matches a stream that was never rewound
        let mut tail = Vec::new();
        cursor.read_to_end(&mut tail).expect("read remaining content");
        assert_eq!(&tail, b"456789");
    }
}
