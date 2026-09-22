//! Corresponds to Java: `org/mpxj/ExternalProjectContainer.java`
//!
//! Package-private utility class: locates and reads external (sub)project files and caches the results, so repeated reads of the same file
//! return directly from the cache.
//!
//! # Java → Rust method mapping
//!
//! | Java | Rust |
//! |---|---|
//! | `public ExternalProjectContainer(ProjectFile file)` | [`ExternalProjectContainer::new`] + [`ExternalProjectContainer::set_context`] (Java receives the parent file at construction; the Rust side splits this into two steps, see below) |
//! | `public void add(String fileName, ProjectFile projectFile)` | [`ExternalProjectContainer::add`] |
//! | `public ProjectFile read(String fileName)` | [`ExternalProjectContainer::read`] |
//! | `private File findFile(String fileName)` | [`ExternalProjectContainer::find_file`] |
//! | `private ProjectFile readFile(File file)` | [`ExternalProjectContainer::read_file`] (reading is done via a pluggable callback, see below) |
//!
//! # Differences from Java
//!
//! - **Back-reference to the parent file**: Java's constructor receives a `ProjectFile` and stores it as a strong reference
//! (`m_file`); a strong reference on the Rust side would form an `Rc` cycle with `ProjectFile → ExternalProjectContainer`
//! so it is changed to `Weak<RefCell<ProjectFile>>`, constructed first with an empty weak reference
//! and then wired up by [`ExternalProjectContainer::set_context`] (performed by
//! the construction flow of [`crate::project_file::ProjectFile`]).
//! - **`java.io.File` → [`std::path::PathBuf`]**: `File` equality is path equality,
//! and `PathBuf` matches; `File.exists()` corresponds to [`Path::exists`], and `new File(dir, name)`
//! corresponds to [`Path::join`]. Java's `fileName.lastIndexOf("\\")` corresponds to
//! `str::rfind('\\')` (the Java-side comment already states that "paths always use Windows separators").
//! - **Cached nulls**: Java's `m_fileMap` / `m_projectMap` cache `null` as well
//! (after a `containsKey` hit they return `null` directly without retrying). The Rust side uses
//! `HashMap<String, Option<PathBuf>>` / `HashMap<PathBuf, Option<Rc<..>>>` to express the same semantics.
//! - **The read action in `readFile` (a category A hard limitation)**: Java directly does
//! `new UniversalProjectReader().read(file)`. This crate does **not** have `UniversalProjectReader`
//! — Java relies on `Class.forName` to iterate over candidate readers under the `org.mpxj.reader` package and
//! dispatch by trial reading via `getFileFormat()`; Rust has no reflection, so this project classifies it as a categorical skip.
//! The concrete readers already implemented are [`crate::mpp::mpp_reader::MPPReader`] and
//!   [`crate::mpx::mpx_reader::MPXReader`]
//! (both implement [`crate::reader::project_reader::ProjectReader`]), so the Rust side abstracts
//! "reading" into a pluggable callback [`ExternalProjectContainer::set_project_reader`] (Rust-specific):
//! callers can dispatch by extension to the above readers themselves to equivalently reproduce Java's dispatch result;
//! when not wired up, `read_file` returns `None` (equivalent to the case where Java fails to read).
//! The rest of `read_file`'s logic (inheriting the subproject search directory) matches Java.
//! - **Exception handling**: Java uses `catch (MPXJException ex) { project = null; }`; the Rust side's
//! callback expresses read failure with `Option`, so no `catch` is needed.
//! - **Reentrant borrow**: `find_file` needs to read the parent file's `ProjectConfig` (the subproject working directory).
//! When the parent file is already borrowed (for example, the caller holds a mutable borrow of `ProjectFile`), the Rust side degrades to
//! "no search directories configured", without panicking (this case does not occur in Java).
//!
//! # Skipped Java mechanisms
//!
//! `org.mpxj.reader.UniversalProjectReader` (reflection dispatch, see above; `crate::reader` and
//! `crate::mpp::mpp_reader::MPPReader` / `crate::mpx::mpx_reader::MPXReader` are all implemented,
//! only this reflection dispatch layer is missing).
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

use crate::project_file::ProjectFile;

/// The callback type for reading project files (Rust-specific, replacing Java's
/// `new UniversalProjectReader().read(file)`).
pub type ProjectReader = Box<dyn FnMut(&Path) -> Option<Rc<RefCell<ProjectFile>>>>;

/// Container for locating and caching external (sub)project files.
///
/// Corresponds to Java: `class ExternalProjectContainer`
pub struct ExternalProjectContainer {
/// The parent project file (weak reference, avoiding an `Rc` cycle).
    ///
    /// Corresponds to Java: `private final ProjectFile m_file;`
    file: Weak<RefCell<ProjectFile>>,

/// Original path → resolved file path (caches `None` when it was probed but no file was found).
    ///
    /// Corresponds to Java: `private final Map<String, File> m_fileMap = new HashMap<>();`
    file_map: HashMap<String, Option<PathBuf>>,

/// File path → project file (caches `None` when reading fails).
    ///
    /// Corresponds to Java: `private final Map<File, ProjectFile> m_projectMap = new HashMap<>();`
    project_map: HashMap<PathBuf, Option<Rc<RefCell<ProjectFile>>>>,

/// Callback for reading project files (Rust-specific, see the file header).
    project_reader: Option<ProjectReader>,
}

impl ExternalProjectContainer {
/// Constructor.
    ///
    /// Corresponds to Java: `public ExternalProjectContainer(ProjectFile file)`
    ///
/// On the Rust side the object is first constructed with an empty weak parent-file reference and wired up by [`ExternalProjectContainer::set_context`].
    pub fn new() -> Self {
        Self {
            file: Weak::new(),
            file_map: HashMap::new(),
            project_map: HashMap::new(),
            project_reader: None,
        }
    }

/// Wire up the parent project file (Rust-specific: Java receives `ProjectFile` directly in the constructor).
    ///
/// Corresponds to Java: `m_file = file;` in the constructor
    pub(crate) fn set_context(&mut self, file: &Rc<RefCell<ProjectFile>>) {
        self.file = Rc::downgrade(file);
    }

/// Get the parent project file (returns `None` when not wired up).
    ///
/// Rust-specific accessor (on the Java side `m_file` is a private field with no corresponding method).
    pub fn get_context(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.file.upgrade()
    }

/// Wire up the callback for reading project files (Rust-specific, see the file header).
    ///
/// Corresponds to Java: `new UniversalProjectReader().read(file)` in `readFile`
    pub fn set_project_reader(&mut self, reader: Option<ProjectReader>) {
        self.project_reader = reader;
    }

/// Register an already existing project file instance.
    ///
    /// Corresponds to Java: `public void add(String fileName, ProjectFile projectFile)`
    pub fn add(&mut self, file_name: &str, project_file: Option<Rc<RefCell<ProjectFile>>>) {
        // Java: File file = new File(fileName);
        let file = PathBuf::from(file_name);
        // Java: m_fileMap.put(fileName, file);
        self.file_map
            .insert(file_name.to_owned(), Some(file.clone()));
        // Java: m_projectMap.put(file, projectFile);
        self.project_map.insert(file, project_file);
    }

/// Read the project file at the given path (the result is cached).
    ///
    /// Corresponds to Java: `public ProjectFile read(String fileName)`
    pub fn read(&mut self, file_name: Option<&str>) -> Option<Rc<RefCell<ProjectFile>>> {
        // Java: if (fileName == null || fileName.isEmpty()) { return null; }
        let file_name = match file_name {
            Some(file_name) if !file_name.is_empty() => file_name,
            _ => return None,
        };

        // Java:
        // File file;
        // if (m_fileMap.containsKey(fileName)) { file = m_fileMap.get(fileName); }
        // else { file = findFile(fileName); m_fileMap.put(fileName, file); }
        let file = match self.file_map.get(file_name) {
            Some(file) => file.clone(),
            None => {
                let file = self.find_file(file_name);
                self.file_map.insert(file_name.to_owned(), file.clone());
                file
            }
        };

        // Java: if (file == null) { return null; }
        let file = file?;

        // Java: if (m_projectMap.containsKey(file)) { return m_projectMap.get(file); }
        if let Some(project) = self.project_map.get(&file) {
            return project.clone();
        }

        // Java: ProjectFile project = readFile(file); m_projectMap.put(file, project); return project;
        let project = self.read_file(&file);
        self.project_map.insert(file, project.clone());
        project
    }

/// Locate a file with the given name: first search by the full path, then try the process working directory or the search directories specified by the caller.
    ///
    /// Corresponds to Java: `private File findFile(String fileName)`
    pub fn find_file(&self, file_name: &str) -> Option<PathBuf> {
        // Java: File file = new File(fileName); if (file.exists()) { ... }
        let file = PathBuf::from(file_name);
        if file.exists() {
            return Some(file);
        }

        // Java: int index = fileName.lastIndexOf("\\"); if (index == -1) { return null; }
// (The Java comment states: the path here always uses Windows separators)
        let index = file_name.rfind('\\')?;

        // Java: String name = fileName.substring(index + 1);
        let name = &file_name[index + 1..];

        // Java: File subprojectWorkingDirectory = m_file.getProjectConfig().getSubprojectWorkingDirectory();
        //       file = subprojectWorkingDirectory == null ? new File(name) : new File(subprojectWorkingDirectory, name);
        let file = match self.subproject_working_directory() {
            Some(working_directory) => working_directory.join(name),
            None => PathBuf::from(name),
        };

        // Java: if (!file.exists()) { return null; } return file;
        if file.exists() {
            Some(file)
        } else {
            None
        }
    }

/// Read a project file.
    ///
    /// Corresponds to Java: `private ProjectFile readFile(File file)`
    ///
/// Java reads with `new UniversalProjectReader().read(file)` and signals failure with `MPXJException`;
/// on the Rust side the read is performed by the callback wired up via [`ExternalProjectContainer::set_project_reader`],
/// and when it is not wired up or the callback returns `None` it returns `None` (see the file header).
    fn read_file(&mut self, file: &Path) -> Option<Rc<RefCell<ProjectFile>>> {
        // Java: project = new UniversalProjectReader().read(file);
        let project = match self.project_reader.as_mut() {
            Some(reader) => reader(file),
            None => return None,
        };

        let project = project?;

        // Java:
        // if (project != null) {
        //    // subproject inherits the search directory
        //    project.getProjectConfig().setSubprojectWorkingDirectory(
        //       m_file.getProjectConfig().getSubprojectWorkingDirectory());
        // }
        let working_directory = self.subproject_working_directory();
        let context = project.borrow().get_project_context();
        if let Ok(mut context_ref) = context.try_borrow_mut() {
            context_ref
                .get_project_config_mut()
                .set_subproject_working_directory(working_directory);
        }

        Some(project)
    }

/// Get the "subproject search directories" from the parent file's configuration (returns `None` when the parent file is not wired up, is already borrowed, or has none configured).
    ///
    /// Corresponds to Java: `m_file.getProjectConfig().getSubprojectWorkingDirectory()`
    fn subproject_working_directory(&self) -> Option<PathBuf> {
        let file = self.file.upgrade()?;
        let file_ref = file.try_borrow().ok()?;
        let config = file_ref.get_project_config();
        config.get_subproject_working_directory().map(Path::to_path_buf)
    }
}

impl Default for ExternalProjectContainer {
/// Corresponds to Java: the explicitly declared `ExternalProjectContainer(ProjectFile)`,
/// the Rust side equivalent of [`ExternalProjectContainer::new`] (the parent file is wired up by `set_context`).
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_file::ProjectFile;

/// Construct a container with the parent file already wired up.
    fn container() -> (Rc<RefCell<ProjectFile>>, ExternalProjectContainer) {
        let file = ProjectFile::new();
        let mut container = ExternalProjectContainer::new();
        container.set_context(&file);
        (file, container)
    }

    #[test]
    fn read_ignores_empty_names() {
        let (_, mut container) = container();

        assert!(container.read(None).is_none());
        assert!(container.read(Some("")).is_none());
    }

    #[test]
    fn added_projects_are_returned_from_cache() {
        let (_, mut container) = container();
        let project = ProjectFile::new();

        container.add("X:\\projects\\subproject.mpp", Some(Rc::clone(&project)));

// Java: m_fileMap / m_projectMap return the cache directly on a hit
        let found = container.read(Some("X:\\projects\\subproject.mpp"));
        assert!(found
            .map(|found| Rc::ptr_eq(&found, &project))
            .unwrap_or(false));

// A second read still hits the cache
        let again = container.read(Some("X:\\projects\\subproject.mpp"));
        assert!(again.is_some());
    }

    #[test]
    fn read_missing_file_returns_none_and_caches_the_miss() {
        let (_, mut container) = container();

// The full path does not exist, and the path contains no Windows separator → Java returns null
        assert!(container.read(Some("no-such-file.mpp")).is_none());
        assert!(container.file_map.contains_key("no-such-file.mpp"));

// Read again: hits the cached null in fileMap
        assert!(container.read(Some("no-such-file.mpp")).is_none());
    }

    #[test]
    fn find_file_falls_back_to_file_name_when_directory_is_unknown() {
        let (_, container) = container();

// The path contains a separator but the file does not exist: tries "original path" and then "file name" and fails
        assert!(container.find_file("X:\\projects\\missing.mpp").is_none());
    }

    #[test]
    fn read_file_uses_the_plugged_in_reader_and_inherits_search_directory() {
        let file = ProjectFile::new();

// Parent file configured search directories
        {
            let context = file.borrow().get_project_context();
            context
                .borrow_mut()
                .get_project_config_mut()
                .set_subproject_working_directory(Some(PathBuf::from("X:\\projects")));
        }

        let mut container = ExternalProjectContainer::new();
        container.set_context(&file);

        let subproject = ProjectFile::new();
        let reader_target = Rc::clone(&subproject);
        container.set_project_reader(Some(Box::new(move |path: &Path| {
            if path == Path::new("X:\\projects\\child.mpp") {
                Some(Rc::clone(&reader_target))
            } else {
                None
            }
        })));

// Drives read_file directly (in Java it is called by read when the cache misses)
        let project = container
            .read_file(Path::new("X:\\projects\\child.mpp"))
            .expect("callback returns project file");
        assert!(Rc::ptr_eq(&project, &subproject));

// Java: subproject inherits the search directories
        assert_eq!(
            subproject
                .borrow()
                .get_project_context()
                .borrow()
                .get_project_config()
                .get_subproject_working_directory(),
            Some(Path::new("X:\\projects"))
        );

// Returns None when the callback misses
        assert!(container.read_file(Path::new("X:\\other.mpp")).is_none());
    }

    #[test]
    fn read_file_without_reader_returns_none() {
        let (_, mut container) = container();
        assert!(container.read_file(Path::new("X:\\projects\\child.mpp")).is_none());
    }

    #[test]
    fn context_can_be_read_back() {
        let (file, container) = container();
        assert!(Rc::ptr_eq(&container.get_context().expect("wired up"), &file));
        assert!(ExternalProjectContainer::new().get_context().is_none());
        assert!(ExternalProjectContainer::default().get_context().is_none());
    }
}
