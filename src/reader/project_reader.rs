//! 对应 Java: `org/mpxj/reader/ProjectReader.java`
//!
//! 所有能读取项目文件、并据此生成 [`ProjectFile`] 的 reader 都实现本 trait。
//!
//! # Java → Rust 方法对照
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `void addProjectListener(ProjectListener listener)` | [`ProjectReader::add_project_listener`] | 监听器以 `Box<dyn ProjectListener>` 移交所有权 |
//! | `ProjectFile read(String fileName)` | [`ProjectReader::read_file`] | 与 `read(File)` 合并 |
//! | `ProjectFile read(File file)` | [`ProjectReader::read_file`] | 同上（Rust 无重载，`String` / `File` 两个重载合并为「按路径」一套） |
//! | `List<ProjectFile> readAll(String fileName)` | [`ProjectReader::read_all_file`] | 与 `readAll(File)` 合并 |
//! | `List<ProjectFile> readAll(File file)` | [`ProjectReader::read_all_file`] | 同上 |
//! | `ProjectFile read(InputStream inputStream)` | [`ProjectReader::read_reader`] | `InputStream` → `&mut dyn std::io::Read`（改为「按读取器」一套） |
//! | `List<ProjectFile> readAll(InputStream inputStream)` | [`ProjectReader::read_all_reader`] | 同上 |
//!
//! # 继承链与接口的还原
//!
//! Java 的继承链是
//! `ProjectReader`（接口）← `AbstractProjectReader` ← `AbstractProjectFileReader` /
//! `AbstractProjectStreamReader` ← 具体 reader（`MPXReader`、`MPPReader` 等）。
//! Rust 没有继承，还原方式：
//!
//! - 本文件：接口 → trait [`ProjectReader`]，只声明方法、不给默认实现（与 Java 接口一致）；
//! - [`crate::reader::abstract_project_reader`]：trait [`crate::reader::abstract_project_reader::AbstractProjectReader`]
//!   以 `ProjectReader` 为 supertrait，承载 Java 抽象基类的公共实现（默认方法）；
//! - [`crate::reader::abstract_project_file_reader`] / [`crate::reader::abstract_project_stream_reader`]：
//!   再以 `AbstractProjectReader` 为 supertrait，分别给出「按文件」与「按流」的默认实现；
//! - 具体 reader 同时 `impl ProjectReader`（把接口方法转调对应抽象层的 `_default`
//!   方法或给出自己的实现）与各抽象层 trait。命名约定与模板见
//!   [`crate::reader::abstract_project_reader`] 文件头。
//!
//! # 与 Java 的差异
//!
//! - **`throws MPXJException` → [`crate::MpxjResult`]**：全部读取方法返回
//!   `MpxjResult<..>`；Java 的 `MPXJException` 对应 [`crate::MpxjError`]。
//! - **返回 `null` 的语义**：Java 的 `read(..)` 允许返回 `null`（`AbstractProjectStreamReader.readAll`
//!   即按 `read` 是否返回 `null` 给出空列表或单元素列表）。Rust 没有 `null`：读取失败以
//!   `Err` 表达，"读不出内容" 的退化路径由各抽象层的默认方法按空 `Vec` 表达（见那些文件头）。
//! - **`List<ProjectFile>` → `Vec<Rc<RefCell<ProjectFile>>>`**：Rust 侧 [`ProjectFile`]
//!   一律以 `Rc<RefCell<ProjectFile>>` 共享持有（原因见 [`crate::project_file`] 文件头）。
//! - **对象安全**：所有方法取 `&mut self`、参数为 `&Path` / `&mut dyn Read`，因此本 trait
//!   是对象安全的，可写作 `&mut dyn ProjectReader`（`AbstractProjectReader::add_listeners_to_reader`
//!   即依赖这一点）。
//!
//! # 跳过的 Java 代码及原因
//!
//! 本接口未使用反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`，也未引用
//! 依赖反射分发、本次不移植的 `UniversalProjectReader` / `ProjectReaderUtility`，故无跳过项。

use std::cell::RefCell;
use std::io::Read;
use std::path::Path;
use std::rc::Rc;

use crate::listener::project_listener::ProjectListener;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;

/// 可以读取项目文件并生成 [`ProjectFile`] 的 reader。
///
/// 对应 Java: `public interface ProjectReader`
pub trait ProjectReader {
    /// 注册一个监听器：读取过程中产生的事件会转发给它。
    ///
    /// 对应 Java: `void addProjectListener(ProjectListener listener)`
    ///
    /// Java 保存监听器对象的引用；Rust 侧以 `Box<dyn ProjectListener>` 移交所有权，
    /// 抽象层再把它装入共享句柄以便分发到多处（见
    /// [`crate::reader::abstract_project_reader`] 文件头）。
    fn add_project_listener(&mut self, listener: Box<dyn ProjectListener>);

    /// 按路径读取一个 schedule。
    ///
    /// 对应 Java: `ProjectFile read(String fileName)` / `ProjectFile read(File file)`
    /// （Rust 无重载，两个重载合并为本方法）。
    fn read_file(&mut self, path: &Path) -> MpxjResult<Rc<RefCell<ProjectFile>>>;

    /// 按路径读取全部 schedule（文件格式不支持多 schedule 时返回单元素列表）。
    ///
    /// 对应 Java: `List<ProjectFile> readAll(String fileName)` /
    /// `List<ProjectFile> readAll(File file)`
    fn read_all_file(&mut self, path: &Path) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>>;

    /// 从给定的读取器读取一个 schedule。
    ///
    /// 对应 Java: `ProjectFile read(InputStream inputStream)`
    fn read_reader(&mut self, reader: &mut dyn Read) -> MpxjResult<Rc<RefCell<ProjectFile>>>;

    /// 从给定的读取器读取全部 schedule。
    ///
    /// 对应 Java: `List<ProjectFile> readAll(InputStream inputStream)`
    fn read_all_reader(&mut self, reader: &mut dyn Read)
    -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpxj_exception::MpxjError;

    /// 最小实现：只验证本 trait 可用（含以 `&mut dyn ProjectReader` 动态调用）。
    struct StubReader {
        /// 最近一次收到的路径，用于断言调用被转发到本实现。
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
                .expect("stub 总是成功");
            let error = dynamic
                .read_reader(&mut std::io::empty())
                .err()
                .expect("stub 按约定报 INVALID_FILE");
            assert_eq!(error.message(), MpxjError::INVALID_FILE);
        }

        assert_eq!(reader.last_path.as_deref(), Some(Path::new("plan.mpp")));
    }
}
