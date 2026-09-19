//! 对应 Java: `org/mpxj/ExternalProjectContainer.java`
//!
//! 包级私有工具类：负责定位并读取外部（子）项目文件，并缓存结果，使同一文件的
//! 多次读取直接从缓存返回。
//!
//! # Java → Rust 方法对照
//!
//! | Java | Rust |
//! |---|---|
//! | `public ExternalProjectContainer(ProjectFile file)` | [`ExternalProjectContainer::new`] + [`ExternalProjectContainer::set_context`]（Java 在构造时拿到父文件，Rust 侧拆成两步，见下） |
//! | `public void add(String fileName, ProjectFile projectFile)` | [`ExternalProjectContainer::add`] |
//! | `public ProjectFile read(String fileName)` | [`ExternalProjectContainer::read`] |
//! | `private File findFile(String fileName)` | [`ExternalProjectContainer::find_file`] |
//! | `private ProjectFile readFile(File file)` | [`ExternalProjectContainer::read_file`]（读取动作改为经由可插拔回调，见下） |
//!
//! # 与 Java 的差异
//!
//! - **父文件的反向引用**：Java 的构造方法接收 `ProjectFile` 并以强引用保存
//!   （`m_file`）；Rust 侧若强引用会与 `ProjectFile → ExternalProjectContainer`
//!   形成 `Rc` 循环，故改为 `Weak<RefCell<ProjectFile>>`，先以空弱引用构造，
//!   再由 [`ExternalProjectContainer::set_context`] 接线（由
//!   [`crate::project_file::ProjectFile`] 的构造流程完成）。
//! - **`java.io.File` → [`std::path::PathBuf`]**：`File` 的相等性是路径相等，
//!   `PathBuf` 一致；`File.exists()` 对应 [`Path::exists`]，`new File(dir, name)`
//!   对应 [`Path::join`]。Java 的 `fileName.lastIndexOf("\\")` 对应
//!   `str::rfind('\\')`（Java 侧注释已说明「路径始终使用 Windows 分隔符」）。
//! - **缓存的空值**：Java 的 `m_fileMap` / `m_projectMap` 都会把 `null` 也缓存下来
//!   （`containsKey` 命中后直接返回 `null`，不再重试）。Rust 侧用
//!   `HashMap<String, Option<PathBuf>>` / `HashMap<PathBuf, Option<Rc<..>>>` 表达同样的语义。
//! - **`readFile` 中的读取动作**：Java 直接 `new UniversalProjectReader().read(file)`。
//!   `reader` 包未在 `src/lib.rs` 中声明（本批次不移植），因此 Rust 侧把「读取」抽成
//!   可插拔回调 [`ExternalProjectContainer::set_project_reader`]（Rust 特有）；
//!   未接线时 `read_file` 返回 `None`（等价于 Java 读取失败的情形）。
//!   `read_file` 的其余逻辑（子项目继承搜索目录）与 Java 一致。
//! - **异常处理**：Java 用 `catch (MPXJException ex) { project = null; }`；Rust 侧的
//!   回调以 `Option` 表达读取失败，无需 `catch`。
//! - **重入借用**：`find_file` 需要读取父文件的 `ProjectConfig`（子项目工作目录）。
//!   父文件已被借用（例如调用方正持有 `ProjectFile` 的可变借用）时 Rust 侧退化为
//!   「未配置搜索目录」，不 panic（Java 无此情形）。
//!
//! # 跳过的 Java 机制
//!
//! `org.mpxj.reader.UniversalProjectReader`（`reader` 包未在 `src/lib.rs` 声明，见上）。
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

use crate::project_file::ProjectFile;

/// 读取项目文件的回调类型（Rust 特有，替代 Java 的
/// `new UniversalProjectReader().read(file)`）。
pub type ProjectReader = Box<dyn FnMut(&Path) -> Option<Rc<RefCell<ProjectFile>>>>;

/// 外部（子）项目文件的定位与缓存容器。
///
/// 对应 Java: `class ExternalProjectContainer`
pub struct ExternalProjectContainer {
    /// 父项目文件（弱引用，避免 `Rc` 循环）。
    ///
    /// 对应 Java: `private final ProjectFile m_file;`
    file: Weak<RefCell<ProjectFile>>,

    /// 原始路径 → 解析后的文件路径（命中过但找不到文件时缓存 `None`）。
    ///
    /// 对应 Java: `private final Map<String, File> m_fileMap = new HashMap<>();`
    file_map: HashMap<String, Option<PathBuf>>,

    /// 文件路径 → 项目文件（读取失败时缓存 `None`）。
    ///
    /// 对应 Java: `private final Map<File, ProjectFile> m_projectMap = new HashMap<>();`
    project_map: HashMap<PathBuf, Option<Rc<RefCell<ProjectFile>>>>,

    /// 读取项目文件的回调（Rust 特有，见文件头说明）。
    project_reader: Option<ProjectReader>,
}

impl ExternalProjectContainer {
    /// 构造方法。
    ///
    /// 对应 Java: `public ExternalProjectContainer(ProjectFile file)`
    ///
    /// Rust 侧先以空父文件弱引用构造，由 [`ExternalProjectContainer::set_context`] 接线。
    pub fn new() -> Self {
        Self {
            file: Weak::new(),
            file_map: HashMap::new(),
            project_map: HashMap::new(),
            project_reader: None,
        }
    }

    /// 接线父项目文件（Rust 特有：Java 在构造方法中直接接收 `ProjectFile`）。
    ///
    /// 对应 Java: 构造方法中的 `m_file = file;`
    pub(crate) fn set_context(&mut self, file: &Rc<RefCell<ProjectFile>>) {
        self.file = Rc::downgrade(file);
    }

    /// 取父项目文件（未接线时返回 `None`）。
    ///
    /// Rust 特有访问器（Java 侧 `m_file` 为私有字段，无对应方法）。
    pub fn get_context(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.file.upgrade()
    }

    /// 接线读取项目文件的回调（Rust 特有，见文件头说明）。
    ///
    /// 对应 Java: `readFile` 中的 `new UniversalProjectReader().read(file)`
    pub fn set_project_reader(&mut self, reader: Option<ProjectReader>) {
        self.project_reader = reader;
    }

    /// 登记一个已存在的项目文件实例。
    ///
    /// 对应 Java: `public void add(String fileName, ProjectFile projectFile)`
    pub fn add(&mut self, file_name: &str, project_file: Option<Rc<RefCell<ProjectFile>>>) {
        // Java: File file = new File(fileName);
        let file = PathBuf::from(file_name);
        // Java: m_fileMap.put(fileName, file);
        self.file_map
            .insert(file_name.to_owned(), Some(file.clone()));
        // Java: m_projectMap.put(file, projectFile);
        self.project_map.insert(file, project_file);
    }

    /// 读取指定路径的项目文件（结果会被缓存）。
    ///
    /// 对应 Java: `public ProjectFile read(String fileName)`
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

    /// 定位指定名称的文件：先按完整路径查找，再尝试进程工作目录或调用方指定的搜索目录。
    ///
    /// 对应 Java: `private File findFile(String fileName)`
    pub fn find_file(&self, file_name: &str) -> Option<PathBuf> {
        // Java: File file = new File(fileName); if (file.exists()) { ... }
        let file = PathBuf::from(file_name);
        if file.exists() {
            return Some(file);
        }

        // Java: int index = fileName.lastIndexOf("\\"); if (index == -1) { return null; }
        // （Java 注释说明：这里的路径始终使用 Windows 分隔符）
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

    /// 读取一个项目文件。
    ///
    /// 对应 Java: `private ProjectFile readFile(File file)`
    ///
    /// Java 用 `new UniversalProjectReader().read(file)` 读取并以 `MPXJException` 表示失败；
    /// Rust 侧由 [`ExternalProjectContainer::set_project_reader`] 接线的回调完成读取，
    /// 未接线或回调返回 `None` 时返回 `None`（见文件头说明）。
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

    /// 取父文件配置中的「子项目搜索目录」（父文件未接线、已被借用或未配置时返回 `None`）。
    ///
    /// 对应 Java: `m_file.getProjectConfig().getSubprojectWorkingDirectory()`
    fn subproject_working_directory(&self) -> Option<PathBuf> {
        let file = self.file.upgrade()?;
        let file_ref = file.try_borrow().ok()?;
        let config = file_ref.get_project_config();
        config.get_subproject_working_directory().map(Path::to_path_buf)
    }
}

impl Default for ExternalProjectContainer {
    /// 对应 Java: 显式声明的 `ExternalProjectContainer(ProjectFile)`，
    /// Rust 侧等价于 [`ExternalProjectContainer::new`]（父文件由 `set_context` 接线）。
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_file::ProjectFile;

    /// 构造一个已接线父文件的容器。
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

        // Java: m_fileMap / m_projectMap 命中后直接返回缓存
        let found = container.read(Some("X:\\projects\\subproject.mpp"));
        assert!(found
            .map(|found| Rc::ptr_eq(&found, &project))
            .unwrap_or(false));

        // 二次读取仍命中缓存
        let again = container.read(Some("X:\\projects\\subproject.mpp"));
        assert!(again.is_some());
    }

    #[test]
    fn read_missing_file_returns_none_and_caches_the_miss() {
        let (_, mut container) = container();

        // 完整路径不存在，且路径中没有 Windows 分隔符 → Java 返回 null
        assert!(container.read(Some("no-such-file.mpp")).is_none());
        assert!(container.file_map.contains_key("no-such-file.mpp"));

        // 再次读取：命中 fileMap 中的缓存 null
        assert!(container.read(Some("no-such-file.mpp")).is_none());
    }

    #[test]
    fn find_file_falls_back_to_file_name_when_directory_is_unknown() {
        let (_, container) = container();

        // 路径中带分隔符，但文件不存在：依次尝试「原路径」与「文件名」后失败
        assert!(container.find_file("X:\\projects\\missing.mpp").is_none());
    }

    #[test]
    fn read_file_uses_the_plugged_in_reader_and_inherits_search_directory() {
        let file = ProjectFile::new();

        // 父文件配置搜索目录
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

        // 直接驱动 read_file（Java 中由 read 在缓存未命中时调用）
        let project = container
            .read_file(Path::new("X:\\projects\\child.mpp"))
            .expect("回调返回项目文件");
        assert!(Rc::ptr_eq(&project, &subproject));

        // Java: 子项目继承搜索目录
        assert_eq!(
            subproject
                .borrow()
                .get_project_context()
                .borrow()
                .get_project_config()
                .get_subproject_working_directory(),
            Some(Path::new("X:\\projects"))
        );

        // 回调未命中时返回 None
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
        assert!(Rc::ptr_eq(&container.get_context().expect("已接线"), &file));
        assert!(ExternalProjectContainer::new().get_context().is_none());
        assert!(ExternalProjectContainer::default().get_context().is_none());
    }
}
