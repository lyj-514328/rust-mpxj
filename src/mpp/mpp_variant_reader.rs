//! 对应 Java: `org/mpxj/mpp/MPPVariantReader.java`
//!
//! 四个 MPP 变体 reader（[`MPP8Reader`](crate::mpp::mpp8_reader::MPP8Reader) /
//! [`MPP9Reader`](crate::mpp::mpp9_reader::MPP9Reader) /
//! [`MPP12Reader`](crate::mpp::mpp12_reader::MPP12Reader) /
//! [`MPP14Reader`](crate::mpp::mpp14_reader::MPP14Reader)）的共同契约：每个变体负责
//! 「按自己的字节布局读完整份 MPP 文件并填充 [`ProjectFile`]」。
//! [`MPPReader::read`](crate::mpp::mpp_reader::MPPReader::read) 从 `\1CompObj` 流拿到
//! 文件格式字符串后，据此选出一个变体并调用 [`MPPVariantReader::process`]。
//!
//! # 最终确定的 trait 签名（四个变体 reader 的对接点）
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
//! 逐项理由（Java → Rust）：
//!
//! | Java | Rust | 理由 |
//! |---|---|---|
//! | `interface MPPVariantReader`（包私有） | `pub trait MPPVariantReader` | Java 里接口与四个实现都在 `org.mpxj.mpp` 包内，包私有足够；Rust 的模块即包，跨模块使用必须 `pub` |
//! | `MPPReader reader` | `&MPPReader` | 变体 reader 对父 reader 只**读**五个开关（`getReadPropertiesOnly` / `getReadPresentationData` / `getRespectPasswordProtection` / `getReadPassword` / `getUseRawTimephasedData`），Java 源码中没有任何 `reader.setXxx(..)` 调用，故不需要 `&mut`。`&MPPReader` 还让 [`MPPReader::read`](crate::mpp::mpp_reader::MPPReader::read) 可以取 `&self`（Java 的 `read` 同样不修改 reader 自身状态），从而在 `read` 内部直接把 `self` 交给 `process` |
//! | `ProjectFile file` | `&Rc<RefCell<ProjectFile>>` | 按本工程约定，`ProjectFile` 一律以 `Rc<RefCell<..>>` 共享持有（见 [`crate::project_file`] 文件头）；按引用传入使 `MPPReader` 保留所有权，同时变体 reader 内部仍可 `Rc::clone` 到任意内部 reader |
//! | `DirectoryEntry root` | `&DirectoryEntry` | POI 的 `DirectoryEntry` 是一个句柄对象；Rust 侧 [`DirectoryEntry`] 是 `#[derive(Clone)]` 的轻量句柄，按引用传入即可，需要长期持有（例如转交内部 reader）时 `root.clone()` |
//! | `throws MPXJException, IOException` | `MpxjResult<()>` | 统一返回 [`MpxjResult`]；`IOException` 经 `From<std::io::Error>` 转为消息前缀 `Error reading file` 的 [`MpxjError`]（与 Java 侧 `new MPXJException(MPXJException.READ_ERROR, ex)` 一致） |
//! | `void process(MPPReader, ProjectFile, DirectoryEntry)`（实例状态可写） | `fn process(&self, ..)` | **本 trait 固定为 `&self`**。Java 侧 `MPP9Reader` 等无参构造 + 在 `populateMemberData` 里把 `m_reader` / `m_file` / `m_root` / `m_projectProps` 等写入实例字段，`process` 结束时 `clearMemberData` 清空 —— 这依赖可变实例状态。Rust 实现者三选一：(1) 把可变状态放进 `RefCell` / `Cell` 字段（`new()` 保持无参）；(2) `process` 内把状态装进一个局部结构体，再把 `&mut` 局部结构体传给各私有方法（最贴近 Java 的逐方法翻译，且不引入内部可变性）；(3) 在 `new(...)` 里接收固定引用，使 `process` 内的状态天然不可变。**签名不因实现风格而变化** |
//!
//! # 实现者必须遵守的三条约定
//!
//! 1. **借用**：`file` 是 `Rc<RefCell<ProjectFile>>`，`ProjectFile` 与 `ProjectContext`
//!    的所有访问一律用 `try_borrow` / `try_borrow_mut`，冲突时返回 `Err` 或跳过并在注释说明
//!    （Java 是稳定引用，Rust 会 panic）。
//! 2. **不要持有 `ProjectFile` 借用去调用会反向借用同一文件的方法**：例如
//!    `Task::remove_predecessor` 内部会 `file.borrow_mut()`；必须先克隆出
//!    `Vec<Rc<RefCell<Task>>>` 之类的快照并释放文件借用。
//! 3. **流的读取**：Java 的 `new DocumentInputStream((DocumentEntry) root.getEntry(name))` 在
//!    Rust 侧一律写成 `root.get_document(name)`（取 [`DocumentEntry`]）后
//!    `entry.get_data()` → `Rc<Vec<u8>>`，再按 `&[u8]` 交给 `Props` / `Var2Data` 等。
//!
//! # 会由 `MPPReader` 负责、变体 reader 不必重复做的事
//!
//! `MPPReader.read(POIFSFileSystem)` 在 `process` 返回后统一完成：`updateStructure()`、
//! `task.setSummary(...)` / 关系校验 / 估算基线回填、清理未使用的派生日历、资源日历与
//! 计量单位后处理、`readComplete()`。变体 reader 只需填数据。
//!
//! # 跳过的 Java 代码及原因
//!
//! 本接口未使用反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`，故无跳过项。

use std::cell::RefCell;
use std::rc::Rc;

use crate::mpp::mpp_reader::MPPReader;
use crate::mpp::ole::DirectoryEntry;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;

/// 能够读取某一种 MPP 文件格式的 reader。
///
/// 对应 Java: `interface MPPVariantReader`
///
/// 对象安全（trait object safe）：四个实现均以 `Box<dyn MPPVariantReader>` 由
/// [`crate::mpp::mpp_reader::MPPReader`] 按 `\1CompObj` 给出的格式名选择并调用。
pub trait MPPVariantReader {
    /// 读取一份 MPP 文件并填充传入的数据结构。
    ///
    /// 对应 Java: `void process(MPPReader reader, ProjectFile file, DirectoryEntry root) throws MPXJException, IOException`
    ///
    /// - `reader`：父 reader，变体 reader 只从中读取
    ///   `getReadPropertiesOnly` / `getReadPresentationData` / `getRespectPasswordProtection` /
    ///   `getReadPassword` / `getUseRawTimephasedData` 五个开关；
    /// - `file`：待填充的项目文件；
    /// - `root`：POI 文件系统的根存储（Rust 侧为 [`DirectoryEntry`] 句柄）。
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

    /// 最小实现：只记录收到的三个参数，用于验证签名可用（含 trait object 调用）。
    struct StubVariantReader {
        /// 调用 `process` 时读到的 `getReadPropertiesOnly()`。
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
            // 变体 reader 只需要 &MPPReader 上的只读开关
            *self.read_properties_only.borrow_mut() = Some(reader.get_read_properties_only());
            // root 句柄可用（含控制字符名的流同样能被枚举出来）
            let _names = root.get_entry_names();
            Err(MpxjError::new("stub"))
        }
    }

    /// 构造一个空复合文档，取回其根存储。
    fn empty_root() -> (PoifsFileSystem, DirectoryEntry) {
        let bytes = cfb::CompoundFile::create(Cursor::new(Vec::new()))
            .expect("创建复合文档")
            .into_inner()
            .into_inner();
        let fs = PoifsFileSystem::new(Cursor::new(bytes)).expect("打开复合文档");
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
            .expect("stub 按约定返回 Err");

        assert_eq!(error.message(), "stub");
    }

    /// 保证 trait 可用 `read_file` 那种「按路径读取」的入口间接触发（编译期检查）。
    #[test]
    fn stub_reader_can_be_driven_through_mpp_reader_public_api() {
        let mut reader = MPPReader::new();
        // 不存在的路径：Java 抛 IOException → MPXJException.READ_ERROR
        let error = reader
            .read_file(Path::new("no-such-dir/no-such-file.mpp"))
            .err()
            .expect("不存在的路径应报错");
        assert!(error.message().starts_with(MpxjError::READ_ERROR));
    }
}
