//! 对应 Java: `org/mpxj/reader/AbstractProjectFileReader.java`
//!
//! 面向「以文件为单位读取」的 reader 的抽象实现：本层把「按流读取」转换为「按临时文件读取」，
//! 并给出 `readAll` 的默认行为（只含单份 schedule）。
//!
//! # Java 继承链 → Rust trait 默认方法
//!
//! Java: `public abstract class AbstractProjectFileReader extends AbstractProjectReader`。
//! Rust 没有继承，故 [`AbstractProjectFileReader`] 以
//! [`AbstractProjectReader`] 为 supertrait，把本类对 `ProjectReader` 接口方法的实现写成
//! trait **默认方法**；默认方法名加以 `_default` 后缀，避免与 [`ProjectReader`] 的同名
//! 方法产生方法解析歧义（该约定的完整说明与具体 reader 的接线模板见
//! [`crate::reader::abstract_project_reader`] 文件头）。
//!
//! # Java → Rust 方法对照
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `ProjectFile read(String fileName)` | （无独立代码） | 仅做 `new File(fileName)` 适配；Rust 侧与 `read(File)` 合并为 [`ProjectReader::read_file`]（`&Path` 可由 `&str`/`PathBuf` 直接得到） |
//! | `List<ProjectFile> readAll(String fileName)` | （无独立代码） | 同上，合并为 [`ProjectReader::read_all_file`] |
//! | `ProjectFile read(File file)` | （抽象，无默认实现） | 由具体 reader 实现 [`ProjectReader::read_file`]；本层不提供实现（与 Java 的 `abstract` 一致） |
//! | `List<ProjectFile> readAll(File file)` | [`AbstractProjectFileReader::read_all_file_default`] | `read(file)` 的包装 |
//! | `ProjectFile read(InputStream inputStream)` | [`AbstractProjectFileReader::read_reader_default`] | 先落盘为临时文件，再走 `read(File)` |
//! | `List<ProjectFile> readAll(InputStream inputStream)` | [`AbstractProjectFileReader::read_all_reader_default`] | 同上，再走 `readAll(File)` |
//!
//! # `java.io.File` / `InputStream` 的处理
//!
//! - **`File` → [`std::path::Path`]**：Java 的 `new File(fileName)` 不做存在性校验
//!   （校验推迟到实际打开文件时），Rust 侧 [`std::path::Path`] 同样只是路径值，
//!   语义一致；真正打开文件由具体 reader 负责（例如走
//!   [`crate::reader::abstract_project_stream_reader::AbstractProjectStreamReader::read_file_default`]，
//!   其中 [`std::fs::File::open`] 的失败会经 `From<std::io::Error>` 转成
//!   [`crate::MpxjError`]）。
//! - **`file.toURI().toURL()` 之类**：本类没有这类用法（Java 源码确认），故无需对应实现。
//! - **临时文件**：`InputStreamHelper.writeStreamToTempFile(inputStream, "tmp")` 对应本模块的
//!   [`write_stream_to_temp_file`]（`Files.createTempFile("mpxj", "tmp")` →
//!   `std::env::temp_dir()` 下以进程号 + 纳秒时间戳命名的文件）；
//!   `FileHelper.deleteQuietly(tempFile)` 对应 [`delete_quietly`]。
//! - **`finally` 中的关闭**：Java 用 `AutoCloseableHelper.closeQuietly` / `finally` 确保释放；
//!   Rust 侧临时文件句柄随作用域 [`Drop`] 自动关闭（并显式 `flush` 以便把写入错误暴露出来）。
//! - **`throws IOException` → `MpxjError`**：Java 把 `IOException` 包成
//!   `new MPXJException("Failed to read file", ex)`，Rust 侧由 [`write_stream_to_temp_file`]
//!   在发生 I/O 失败时给出同文本前缀的错误（[`crate::MpxjError::with_cause`] 会把底层
//!   I/O 错误文本附加在消息后）。
//!
//! # `null` 返回值
//!
//! Java 的 `readAll(File)` 写 `projectFile == null ? emptyList() : singletonList(projectFile)`。
//! Rust 侧 `read_file` 以 `Result` 表达失败（失败即 `Err`），成功时必然是某个
//! `Rc<RefCell<ProjectFile>>`，故默认实现恒返回单元素列表，判空分支消失。
//!
//! # 跳过的 Java 代码及原因
//!
//! - `read(String)` / `readAll(String)`：纯参数适配（`String` → `File`），Rust 侧已合并进
//!   按路径的方法（见上表），无独立代码可译；
//! - 本类未使用反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`，也未引用
//!   依赖反射分发、本次不移植的 `UniversalProjectReader` / `ProjectReaderUtility`。

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

/// 面向「以文件为单位读取」的 reader 的抽象实现。
///
/// 对应 Java: `public abstract class AbstractProjectFileReader extends AbstractProjectReader`
///
/// 本层仍为「抽象」的方法是 [`ProjectReader::read_file`]（Java `read(File)`），
/// 由具体 reader 提供实现。
pub trait AbstractProjectFileReader: AbstractProjectReader {
    /// 按路径读取全部 schedule（本层格式只含单份 schedule，故返回单元素列表）。
    ///
    /// 对应 Java: `@Override public List<ProjectFile> readAll(File file)`
    ///
    /// （Java 的 `readAll(String fileName)` 仅做 `new File(fileName)` 适配，已合并进本方法。）
    fn read_all_file_default(&mut self, path: &Path) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
        // Java: ProjectFile projectFile = read(file);
        let project = ProjectReader::read_file(self, path)?;

        // Java: return projectFile == null ? Collections.emptyList() : Collections.singletonList(projectFile);
        Ok(vec![project])
    }

    /// 从读取器读取一个 schedule：先落盘为临时文件，再按路径读取。
    ///
    /// 对应 Java: `@Override public ProjectFile read(InputStream inputStream)`
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

    /// 从读取器读取全部 schedule：先落盘为临时文件，再按路径读取。
    ///
    /// 对应 Java: `@Override public List<ProjectFile> readAll(InputStream inputStream)`
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

/// 把读取器中的全部数据写入一个新临时文件，返回该文件路径。
///
/// 对应 Java: `InputStreamHelper.writeStreamToTempFile(InputStream inputStream, String tempFileSuffix)`
/// （内部为 `Files.createTempFile("mpxj", suffix)` + `writeInputStreamToOutputStream`，
/// 后者等价于 [`io::copy`]），I/O 失败时对应 Java 的
/// `throw new MPXJException("Failed to read file", ex)`。
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
            // Java: 写入失败时临时文件尚未赋给 tempFile，finally 中不会删除（Rust 侧顺手清理）
            delete_quietly(&path);
            Err(MpxjError::with_cause("Failed to read file", &error))
        }
    }
}

/// 生成临时文件路径（对应 `Files.createTempFile("mpxj", suffix)`）。
///
/// Java 由 `Files.createTempFile` 保证新建文件唯一；Rust 侧用「进程号 + 纳秒时间戳」命名，
/// 配合调用方的 `create_new` 保证不会覆盖既有文件。
fn temp_file_path(suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();

    // Java: Files.createTempFile("mpxj", tempFileSuffix) —— 前缀 "mpxj"，后缀直接拼在唯一部分之后
    std::env::temp_dir().join(format!("mpxj-{}-{}{}", std::process::id(), nanos, suffix))
}

/// 静默删除文件（失败时不报错）。
///
/// 对应 Java: `FileHelper.deleteQuietly(File file)`
/// （本处只处理普通文件；Java 版本还会递归删除目录，本类的调用点只涉及临时文件。）
fn delete_quietly(path: &Path) {
    let _ = std::fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::listener::project_listener::ProjectListener;
    use crate::reader::abstract_project_reader::SharedProjectListener;

    /// 最小文件型 reader：`read_file` 记录收到的路径并按需返回项目或错误。
    #[derive(Default)]
    struct StubFileReader {
        listeners: Vec<SharedProjectListener>,
        /// 最近一次 `read_file` 收到的路径。
        last_path: Option<PathBuf>,
        /// 最近一次 `read_file` 时该路径的内容（用于验证临时文件已写满数据）。
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
            .expect("stub 总是成功");

        assert_eq!(projects.len(), 1);
        assert_eq!(reader.last_path.as_deref(), Some(Path::new("plan.mpp")));
    }

    #[test]
    fn read_reader_default_goes_through_a_temp_file_and_cleans_up() {
        let mut reader = StubFileReader::default();

        let project = reader
            .read_reader_default(&mut &b"fake schedule"[..])
            .expect("stub 总是成功");

        // 返回的确实是 stub 新建的项目文件（已登记到它自己的上下文）
        assert_eq!(
            project.borrow().get_project_context().borrow().get_projects().len(),
            1
        );

        // 传入的读取器内容被完整写入临时文件
        assert_eq!(reader.last_contents.as_deref(), Some(&b"fake schedule"[..]));

        // 临时文件位于系统临时目录、带 Java 侧同样的 "tmp" 后缀，且调用后被删除
        let temp_path = reader.last_path.clone().expect("已收到路径");
        assert!(temp_path.starts_with(std::env::temp_dir()));
        assert!(temp_path.to_string_lossy().ends_with("tmp"));
        assert!(!temp_path.exists());
    }

    #[test]
    fn read_all_reader_default_wraps_a_single_project() {
        let mut reader = StubFileReader::default();

        let projects = reader
            .read_all_reader_default(&mut &b"fake schedule"[..])
            .expect("stub 总是成功");

        assert_eq!(projects.len(), 1);
        // 同样经临时文件中转
        let temp_path = reader.last_path.clone().expect("已收到路径");
        assert!(!temp_path.exists());
    }

    #[test]
    fn write_stream_to_temp_file_reports_read_failures() {
        /// 读取时恒报错的读取器。
        struct FailingReader;

        impl Read for FailingReader {
            fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::Other, "boom"))
            }
        }

        let error = write_stream_to_temp_file(&mut FailingReader, "tmp")
            .expect_err("写入失败应报错");

        // Java: throw new MPXJException("Failed to read file", ex)
        assert!(error.message().starts_with("Failed to read file"));
    }
}
