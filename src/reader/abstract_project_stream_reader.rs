//! 对应 Java: `org/mpxj/reader/AbstractProjectStreamReader.java`
//!
//! 面向「以流为单位读取」的 reader 的抽象实现：本层把「按文件读取」转换为「打开文件得到
//! 输入流，再按流读取」，并给出 `readAll` 的默认行为（只含单份 schedule）。
//!
//! # Java 继承链 → Rust trait 默认方法
//!
//! Java: `public abstract class AbstractProjectStreamReader extends AbstractProjectReader`。
//! Rust 没有继承，故 [`AbstractProjectStreamReader`] 以
//! [`AbstractProjectReader`] 为 supertrait，把本类对 `ProjectReader` 接口方法的实现写成
//! trait **默认方法**；默认方法名加以 `_default` 后缀，避免与 [`ProjectReader`] 的同名
//! 方法产生方法解析歧义（该约定的完整说明与具体 reader 的接线模板见
//! [`crate::reader::abstract_project_reader`] 文件头）。
//!
//! # Java → Rust 方法对照
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `ProjectFile read(String fileName)` | （无独立代码） | 仅做 `new File(fileName)` 适配；Rust 侧与 `read(File)` 合并为 [`ProjectReader::read_file`] |
//! | `List<ProjectFile> readAll(String fileName)` | （无独立代码） | 同上，合并为 [`ProjectReader::read_all_file`] |
//! | `ProjectFile read(File file)` | [`AbstractProjectStreamReader::read_file_default`] | 打开文件后转调 `read(InputStream)` |
//! | `List<ProjectFile> readAll(File file)` | [`AbstractProjectStreamReader::read_all_file_default`] | 打开文件后转调 `readAll(InputStream)` |
//! | `ProjectFile read(InputStream inputStream)` | （抽象，无默认实现） | 由具体 reader 实现 [`ProjectReader::read_reader`] |
//! | `List<ProjectFile> readAll(InputStream inputStream)` | [`AbstractProjectStreamReader::read_all_reader_default`] | `read(inputStream)` 的包装 |
//!
//! # `InputStream` 的处理
//!
//! - **`FileInputStream` → [`std::fs::File`]**：Java 打开文件失败时抛 `IOException`，
//!   被捕获后包成 `new MPXJException(MPXJException.READ_ERROR, ex)`；Rust 侧
//!   [`std::fs::File::open`] 的失败经 `From<std::io::Error>` 转成 `MpxjError`
//!   （消息前缀即 `MPXJException.READ_ERROR` 的文本），可直接 `?`。路径不存在等情形
//!   由此统一报错（Java 的 `read(String)` / `read(File)` 本身不做路径校验，同样推迟到打开时）。
//! - **`fis.close()` 与 `finally { AutoCloseableHelper.closeQuietly(fis); }`**：Java 显式关闭
//!   自己打开的 `FileInputStream`；Rust 侧 [`std::fs::File`] 随作用域 [`Drop`] 自动关闭，
//!   无需对应代码。Java **不会**关闭调用方传入的 `inputStream`（`read(InputStream)` 的
//!   流所有权属于调用方），Rust 侧同样只借用 `&mut dyn Read`。
//! - **`BufferedInputStream` / `mark` / `reset`**：本类并未使用（Java 源码确认：仅
//!   `new FileInputStream(file)`，无缓冲包装、无 `mark` / `reset`），因此本层无需替代方案。
//!   但具体 reader（MPX / MPP）通常需要先「窥探」若干个字节再决定分支，Java 侧靠
//!   `InputStream.mark()` / `reset()` 实现；Rust 的 [`std::io::Read`] **没有** `mark` / `reset`，
//!   替代方案为：
//!
//!   1. 只需窥探、不需回退时，用 [`std::io::BufReader`] 的 `fill_buf` / `consume` 观察并
//!      「预支」缓冲内容；
//!   2. 需要真正的回退（等价 `mark` + `reset`）时，用本模块的 [`read_stream_into_cursor`]
//!      把整个流读入 `Vec<u8>` 再包成 [`std::io::Cursor`]：`Cursor` 实现 [`std::io::Seek`]，
//!      `seek(SeekFrom::Start(n))` 等价于 `reset()` 回到 `mark` 的位置，`position()`
//!      等价于记录 `mark` 时的偏移。代价是整份数据驻留内存（MPX/MPP 文件通常不大，
//!      与 Java 侧 `BufferedInputStream` 的默认 8KB 缓冲相比是内存换功能）。
//!
//! # `null` 返回值
//!
//! Java 的 `readAll(InputStream)` 写 `file == null ? emptyList() : singletonList(file)`。
//! Rust 侧 `read_reader` 以 `Result` 表达失败（失败即 `Err`），成功时必然得到某个
//! `Rc<RefCell<ProjectFile>>`，故默认实现恒返回单元素列表，判空分支消失。
//!
//! # 跳过的 Java 代码及原因
//!
//! - `read(String)` / `readAll(String)`：纯参数适配（`String` → `File`），已合并进按路径的方法；
//! - `fis.close()` / `closeQuietly(fis)`：Rust 的 RAII 关闭语义替代（见上）；
//! - 本类未使用反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`，也未引用
//!   依赖反射分发、本次不移植的 `UniversalProjectReader` / `ProjectReaderUtility`。

use std::cell::RefCell;
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;
use std::rc::Rc;

use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;
use crate::reader::abstract_project_reader::AbstractProjectReader;
use crate::reader::project_reader::ProjectReader;

/// 面向「以流为单位读取」的 reader 的抽象实现。
///
/// 对应 Java: `public abstract class AbstractProjectStreamReader extends AbstractProjectReader`
///
/// 本层仍为「抽象」的方法是 [`ProjectReader::read_reader`]（Java `read(InputStream)`），
/// 由具体 reader 提供实现。
pub trait AbstractProjectStreamReader: AbstractProjectReader {
    /// 按路径读取一个 schedule：打开文件得到读取器后按流读取。
    ///
    /// 对应 Java: `@Override public ProjectFile read(File file)`
    ///
    /// （Java 的 `read(String fileName)` 仅做 `new File(fileName)` 适配，已合并进本方法。）
    fn read_file_default(&mut self, path: &Path) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        // Java: fis = new FileInputStream(file); catch (IOException ex) { throw new MPXJException(MPXJException.READ_ERROR, ex); }
        let mut file = File::open(path)?;

        // Java: ProjectFile projectFile = read(fis); fis.close(); return projectFile;
        // （fis.close() 由 Rust 的 Drop 完成）
        ProjectReader::read_reader(self, &mut file)
    }

    /// 按路径读取全部 schedule：打开文件得到读取器后按流读取。
    ///
    /// 对应 Java: `@Override public List<ProjectFile> readAll(File file)`
    ///
    /// （Java 的 `readAll(String fileName)` 仅做 `new File(fileName)` 适配，已合并进本方法。）
    fn read_all_file_default(&mut self, path: &Path) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
        // Java: fis = new FileInputStream(file); catch (IOException ex) { throw new MPXJException(MPXJException.READ_ERROR, ex); }
        let mut file = File::open(path)?;

        // Java: List<ProjectFile> projectFiles = readAll(fis); fis.close(); return projectFiles;
        ProjectReader::read_all_reader(self, &mut file)
    }

    /// 从读取器读取全部 schedule（本层格式只含单份 schedule，故返回单元素列表）。
    ///
    /// 对应 Java: `@Override public List<ProjectFile> readAll(InputStream inputStream)`
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

/// 把读取器的全部数据读入内存，返回支持回退（随机访问）的游标。
///
/// Rust 特有（Java 无对应方法）：Java 的 `InputStream.mark()` / `reset()` 在
/// [`std::io::Read`] 中没有对应能力，[`std::io::Cursor`] 配合 [`std::io::Seek`] 是其替代 ——
/// 见文件头「`BufferedInputStream` / `mark` / `reset`」一节。
pub fn read_stream_into_cursor(reader: &mut dyn Read) -> MpxjResult<Cursor<Vec<u8>>> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    // Cursor 实现 Read + Seek：seek(SeekFrom::Start(n)) 等价 Java 的 reset()（回到 mark 处），
    // position() 等价 Java 的 mark 记录。
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

    /// 最小流式 reader：`read_reader` 把读取器内容全部收下并返回一个新项目。
    #[derive(Default)]
    struct StubStreamReader {
        listeners: Vec<SharedProjectListener>,
        /// 最近一次 `read_reader` 收到的全部字节。
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

    /// 创建一个内容确定的测试文件，返回其路径。
    fn temp_file_with(contents: &[u8]) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "mpxj-stream-reader-test-{}-{}.tmp",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));

        let mut file = File::create(&path).expect("创建测试文件");
        file.write_all(contents).expect("写入测试文件");
        path
    }

    #[test]
    fn read_file_default_opens_the_file_and_delegates_to_the_stream_reader() {
        let path = temp_file_with(b"fake schedule");
        let mut reader = StubStreamReader::default();

        let project = reader.read_file_default(&path).expect("stub 总是成功");

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
            .expect("不存在的路径应报错");

        // Java: catch (IOException ex) { throw new MPXJException(MPXJException.READ_ERROR, ex); }
        assert!(error.message().starts_with(MpxjError::READ_ERROR));
    }

    #[test]
    fn read_all_file_default_opens_the_file_and_returns_a_single_project() {
        let path = temp_file_with(b"fake schedule");
        let mut reader = StubStreamReader::default();

        let projects = reader.read_all_file_default(&path).expect("stub 总是成功");

        assert_eq!(projects.len(), 1);
        assert_eq!(reader.last_contents.as_deref(), Some(&b"fake schedule"[..]));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn read_all_reader_default_wraps_a_single_project() {
        let mut reader = StubStreamReader::default();

        let projects = reader
            .read_all_reader_default(&mut &b"fake schedule"[..])
            .expect("stub 总是成功");

        assert_eq!(projects.len(), 1);
        assert_eq!(reader.last_contents.as_deref(), Some(&b"fake schedule"[..]));
    }

    #[test]
    fn cursor_replaces_mark_and_reset() {
        let source = b"0123456789";
        let mut cursor = read_stream_into_cursor(&mut &source[..]).expect("读入内存");

        // Java: int mark = stream.read() 之前调用 stream.mark(...)
        let mark = cursor.stream_position().expect("取当前位置");
        let mut head = [0u8; 4];
        cursor.read_exact(&mut head).expect("读取前 4 字节");
        assert_eq!(&head, b"0123");

        // Java: stream.reset() —— Rust: 回到 mark 记录的偏移
        cursor.seek(SeekFrom::Start(mark)).expect("回退到 mark");
        let mut again = [0u8; 4];
        cursor.read_exact(&mut again).expect("再次读取前 4 字节");
        assert_eq!(&again, b"0123");

        // 继续读取剩余内容，行为与未回退过的流一致
        let mut tail = Vec::new();
        cursor.read_to_end(&mut tail).expect("读取剩余内容");
        assert_eq!(&tail, b"456789");
    }
}
