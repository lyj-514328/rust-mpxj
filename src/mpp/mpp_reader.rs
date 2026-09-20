//! 对应 Java: `org/mpxj/mpp/MPPReader.java`
//!
//! 通过读取 MPP 文件创建 [`ProjectFile`]。本类是 mpp 包的「总入口」：它从 OLE 复合文档的
//! `\1CompObj` 流里取出文件格式字符串（例如 `MSProject.MPP14`），据此挑出一个变体 reader
//! （[`MPP8Reader`] / [`MPP9Reader`] / [`MPP12Reader`] / [`MPP14Reader`]，契约见
//! [`MPPVariantReader`]），再把变体读完的 [`ProjectFile`] 统一做后处理。
//!
//! # 继承链的还原（组合 + 转发）
//!
//! Java: `public final class MPPReader extends AbstractProjectStreamReader`。Rust 无继承，
//! 按 [`crate::reader::abstract_project_stream_reader`] 文件头给出的模板：
//! [`MPPReader`] 保存监听器列表并同时 `impl` [`ProjectReader`]、[`AbstractProjectReader`]、
//! [`AbstractProjectStreamReader`]，把可复用的接口方法转调抽象层的 `_default` 方法。
//!
//! | Java | Rust |
//! |---|---|
//! | `read(InputStream)` | 入口方法是 [`MPPReader::read_stream`]（名字加 `_stream` 后缀以与 Java 的另一个重载 `read(POIFSFileSystem)` 区分），接口方法 [`ProjectReader::read_reader`] 转调它 |
//! | `read(File)` | 入口落在接口方法 [`ProjectReader::read_file`] 内（Java 的 MPPReader 覆盖了基类版本，以便直接对 `File` 建 `POIFSFileSystem`；本 crate 的具体 reader 都把「本层仍为抽象的」方法直接写在 trait 实现里，见 [`crate::reader::abstract_project_stream_reader`] 的接线模板） |
//! | `readAll(File)` | [`MPPReader::read_all_file`] → `AbstractProjectStreamReader::read_all_file_default` |
//! | `readAll(InputStream)` | [`MPPReader::read_all_reader`] → `AbstractProjectStreamReader::read_all_reader_default` |
//! | `addProjectListener(ProjectListener)` | [`MPPReader::add_project_listener`] → `AbstractProjectReader::add_project_listener_default` |
//! | `read(POIFSFileSystem)` | [`MPPReader::read`] |
//! | `static getFileFormat(POIFSFileSystem)` | [`MPPReader::get_file_format`] |
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `read(InputStream)` 里 `new POIFSFileSystem(is)` | 先经 [`read_stream_into_cursor`] 把流读入内存，再 `PoifsFileSystem::new` | [`PoifsFileSystem::new`] 要求输入源 `'static`，而 `&mut dyn Read` 不是；MPP 文件通常不大，代价可接受（理由见 `ole.rs` 文件头「输入源统一为 `Box<dyn ReadSeek>`」） |
//! | `read(File)` 里 `new POIFSFileSystem(file)` 以降低内存占用 | `PoifsFileSystem::new(File::open(path)?)` | 该优化仍是「按文件」路径（`std::fs::File` 满足 `Read + Seek`），未退化为「先读入内存」；`finally { AutoCloseableHelper.closeQuietly(fs); }` 由 `File` 的 `Drop` 取代 |
//! | `read(POIFSFileSystem)` 的 `catch (IOException \| InstantiationException \| IllegalAccessException)` | 无 `catch` | `IOException` 由 `?` 经 `From<std::io::Error>` 转成前缀为 `Error reading file` 的 [`MpxjError`]（与 `new MPXJException(READ_ERROR, ex)` 一致）；后两个异常来自反射 `newInstance()`，反射已跳过（见 [`file_class_map`]） |
//! | `FILE_CLASS_MAP`（`Map<String, Class<..>>`）+ `readerClass.newInstance()` | [`file_class_map`]：按格式名直接构造 | 反射在本工程被跳过；对外行为一致（未登记的格式 → `null` / `None`），并消去两个反射相关异常 |
//! | `static String getFileFormat(..)`（可能返回 `""` 或 `null`） | `Option<String>` | Java 的 `""`（根存储里没有 `\1CompObj` 流）与 `null`（CompObj 未记录 fileFormat）在 Rust 侧统一为 `None` —— 二者在唯一的使用者 `UniversalProjectReader` 中都被 `fileFormat == null \|\| !fileFormat.startsWith("MSProject")` 判为「不是 MPP」，行为等价 |
//! | `new CompObj(new DocumentInputStream((DocumentEntry) root.getEntry("\1CompObj")))` | [`MPPReader::read_comp_obj`]：`root.get_document(..)` 失败即 `Err` | POI 的 `getEntry` 找不到条目时抛异常，Rust 侧 [`DirectoryEntry::get_document`] 返回 `None`，此处转为 `Err(INVALID_FILE)`（Java 侧是 `NullPointerException`/`ClassCastException`，属未捕获的运行时异常） |
//! | `ProjectFile` 直接可变访问 | 一律 `Rc<RefCell<ProjectFile>>` + `try_borrow` / `try_borrow_mut` | Rust 借用冲突时要退化为 `Err` 而不能 panic（Java 是稳定引用，无此情形）；见 [`borrow_file`] / [`borrow_file_mut`] / [`borrow_context`] 与 [`borrow_conflict`] |
//! | `config.setAutoXXX(false)` 经 `ProjectFile.getProjectConfig()` | 经 `ProjectContext::get_project_config_mut()` | [`ProjectFile::get_project_config`] 在 Rust 侧返回只读的 `Ref<ProjectConfig>`（配置由上下文持有，见其文件头），改写必须走上下文 |
//! | `task.setSummary(..)` 与 `relation.getSuccessorTask().removePredecessor(..)` 在同一循环里 | 先快照任务列表再逐个处理 | `Task::remove_predecessor` 内部会 `file.borrow_mut()`，若外层仍持有 `ProjectFile` 借用会 panic（Java 无此限制）；先克隆出 `Vec<Rc<RefCell<Task>>>` 并释放文件借用即可等价 |
//! | `resourceCalendarMap.computeIfAbsent(calendar.getUniqueID(), k -> emptyList())` | `HashMap<Option<i32>, Vec<..>>` + `entry(..).or_default()` | `HashMap` 允许 `null` 键，故 Java 的键类型保持为可空的 `Integer`；Rust 用 `Option<i32>` 键忠实对应（`None` 键插入空列表 → `size() == 0`） |
//! | `projectFile.getCalendars().removeIf(predicate)` | 先收集待删下标、再逆序 [`ProjectCalendarContainer::remove`] | 容器未提供 `remove_if`，且 `remove` 接收 `&ProjectCalendar`；逆序按下标移除与 `removeIf` 的逐个移除语义一致 |
//! | `m_readPassword`（`String`，可为 `null`） | `Option<String>` | —— |
//!
//! # 对变体 reader 的接口假设（已核对：四个变体均已落地）
//!
//! - 变体类型位于 `crate::mpp::mpp{8,9,12,14}_reader`，类型名 `MPP8Reader` / `MPP9Reader` /
//!   `MPP12Reader` / `MPP14Reader`；
//! - 各提供**无参**构造方法 `new()`（对应 Java 的隐式无参构造
//!   `MPP9Reader reader = new MPP9Reader();`），并实现 [`MPPVariantReader`]；
//! - `MPPVariantReader::process(&self, reader: &MPPReader, file: &Rc<RefCell<ProjectFile>>,
//!   root: &DirectoryEntry)` 的签名以 [`crate::mpp::mpp_variant_reader`] 为准。
//!
//! 其中 `reader: &MPPReader`（而非 `&Rc<RefCell<MPPReader>>`）是刻意的选择：变体 reader 只需
//! 读五个开关，而「`&Rc<RefCell<MPPReader>>`」无法由 [`ProjectReader`] 的 `&mut self` 入口
//! 提供（`RefCell` 不在 `self: Rc<Self>` 之类合法接收者之列），会让 [`MPPReader::read`] 无法
//! 把 `this`（Java 语义）原样交给 `process`。四个变体 reader（含 `MPP8Reader`）均已按该签名落地，
//! 本文件的 `use crate::mpp::mpp8_reader::MPP8Reader` 可正常解析。
//!
//! # 跳过的 Java 代码及原因
//!
//! - **反射**：`FILE_CLASS_MAP` 的 `Class` 值 + `readerClass.newInstance()` →
//!   [`file_class_map`] 直接构造（见上表）；
//! - **`InstantiationException` / `IllegalAccessException`**：仅由反射抛出，随反射一并消失；
//! - **`import org.mpxj.TaskField` 的 `Map<TaskField, TaskField>`**：改为常量表
//!   [`TASK_ESTIMATED_BASELINE_FIELDS`]（`&[(TaskField, TaskField)]`），避免为静态查表引入
//!   全局可变容器；
//! - 本类未使用 JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`；也未引用依赖反射分发的
//!   `UniversalProjectReader` / `ProjectReaderUtility`（本次不移植）。

use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;
use std::rc::Rc;

use crate::calendar_type::CalendarType;
use crate::mpp::comp_obj::CompObj;
use crate::mpp::mpp12_reader::MPP12Reader;
use crate::mpp::mpp14_reader::MPP14Reader;
use crate::mpp::mpp8_reader::MPP8Reader;
use crate::mpp::mpp9_reader::MPP9Reader;
use crate::mpp::mpp_variant_reader::MPPVariantReader;
use crate::mpp::ole::{DirectoryEntry, PoifsFileSystem};
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_calendar::ProjectCalendar;
use crate::project_context::ProjectContext;
use crate::project_file::ProjectFile;
use crate::reader::abstract_project_reader::{AbstractProjectReader, SharedProjectListener};
use crate::reader::abstract_project_stream_reader::{
    read_stream_into_cursor, AbstractProjectStreamReader,
};
use crate::reader::project_reader::ProjectReader;
use crate::resource::Resource;
use crate::task::Task;
use crate::task_field::TaskField;

/// `\1CompObj` 流的名字（Java 里的 `"\1CompObj"`，即 `\u{1}` + `CompObj`）。
const COMP_OBJ: &str = "\u{1}CompObj";

/// 对应 Java: `projectProperties.setFileApplication("Microsoft Project Server")`。
const FILE_APPLICATION_PROJECT_SERVER: &str = "Microsoft Project Server";
/// 对应 Java: `projectProperties.setFileApplication("Microsoft")`。
const FILE_APPLICATION_MICROSOFT: &str = "Microsoft";
/// 对应 Java: `projectProperties.setFileType("MPP")`。
const FILE_TYPE_MPP: &str = "MPP";
/// 对应 Java: `projectFilePath.startsWith("<>\\")`（Project Server 的项目路径前缀）。
const PROJECT_SERVER_PATH_PREFIX: &str = "<>\\";
/// 对应 Java: `name = "Unnamed Resource";`
const UNNAMED_RESOURCE: &str = "Unnamed Resource";
/// 对应 Java: 字符串拼接 `null` 时的渲染（`INVALID_FILE + ": " + format`）。
const JAVA_NULL: &str = "null";
/// 对应 Java: `TaskField.BASELINE_DURATION` → `TaskField.BASELINE_ESTIMATED_DURATION` 等
/// `TASK_ESTIMATED_BASELINE_FIELDS` 静态表的全部条目（11 组基线 × 3 个字段）。
const TASK_ESTIMATED_BASELINE_FIELDS: &[(TaskField, TaskField)] = &[
    (TaskField::BaselineDuration, TaskField::BaselineEstimatedDuration),
    (TaskField::BaselineStart, TaskField::BaselineEstimatedStart),
    (TaskField::BaselineFinish, TaskField::BaselineEstimatedFinish),
    (TaskField::Baseline1Duration, TaskField::Baseline1EstimatedDuration),
    (TaskField::Baseline1Start, TaskField::Baseline1EstimatedStart),
    (TaskField::Baseline1Finish, TaskField::Baseline1EstimatedFinish),
    (TaskField::Baseline2Duration, TaskField::Baseline2EstimatedDuration),
    (TaskField::Baseline2Start, TaskField::Baseline2EstimatedStart),
    (TaskField::Baseline2Finish, TaskField::Baseline2EstimatedFinish),
    (TaskField::Baseline3Duration, TaskField::Baseline3EstimatedDuration),
    (TaskField::Baseline3Start, TaskField::Baseline3EstimatedStart),
    (TaskField::Baseline3Finish, TaskField::Baseline3EstimatedFinish),
    (TaskField::Baseline4Duration, TaskField::Baseline4EstimatedDuration),
    (TaskField::Baseline4Start, TaskField::Baseline4EstimatedStart),
    (TaskField::Baseline4Finish, TaskField::Baseline4EstimatedFinish),
    (TaskField::Baseline5Duration, TaskField::Baseline5EstimatedDuration),
    (TaskField::Baseline5Start, TaskField::Baseline5EstimatedStart),
    (TaskField::Baseline5Finish, TaskField::Baseline5EstimatedFinish),
    (TaskField::Baseline6Duration, TaskField::Baseline6EstimatedDuration),
    (TaskField::Baseline6Start, TaskField::Baseline6EstimatedStart),
    (TaskField::Baseline6Finish, TaskField::Baseline6EstimatedFinish),
    (TaskField::Baseline7Duration, TaskField::Baseline7EstimatedDuration),
    (TaskField::Baseline7Start, TaskField::Baseline7EstimatedStart),
    (TaskField::Baseline7Finish, TaskField::Baseline7EstimatedFinish),
    (TaskField::Baseline8Duration, TaskField::Baseline8EstimatedDuration),
    (TaskField::Baseline8Start, TaskField::Baseline8EstimatedStart),
    (TaskField::Baseline8Finish, TaskField::Baseline8EstimatedFinish),
    (TaskField::Baseline9Duration, TaskField::Baseline9EstimatedDuration),
    (TaskField::Baseline9Start, TaskField::Baseline9EstimatedStart),
    (TaskField::Baseline9Finish, TaskField::Baseline9EstimatedFinish),
    (TaskField::Baseline10Duration, TaskField::Baseline10EstimatedDuration),
    (TaskField::Baseline10Start, TaskField::Baseline10EstimatedStart),
    (TaskField::Baseline10Finish, TaskField::Baseline10EstimatedFinish),
];

/// 通过读取 MPP 文件创建 [`ProjectFile`]。
///
/// 对应 Java: `public final class MPPReader extends AbstractProjectStreamReader`
pub struct MPPReader {
    /// 已注册的监听器。
    ///
    /// 对应 Java: `AbstractProjectReader` 的私有字段
    /// `private List<ProjectListener> m_projectListeners`（Rust 侧 trait 无法带字段，故落在实现者身上）。
    listeners: Vec<SharedProjectListener>,

    /// 设为 `true` 时直接取用 MS Project 的原始时间分段数据，不做归一化。
    ///
    /// 对应 Java: `private boolean m_useRawTimephasedData`
    use_raw_timephased_data: bool,

    /// 是否读取展示数据（设为 `false` 可省时间与内存）。
    ///
    /// 对应 Java: `private boolean m_readPresentationData = true`
    read_presentation_data: bool,

    /// 是否只读取项目属性。
    ///
    /// 对应 Java: `private boolean m_readPropertiesOnly`
    read_properties_only: bool,

    /// 是否尊重密码保护。
    ///
    /// 对应 Java: `private boolean m_respectPasswordProtection = true`
    respect_password_protection: bool,

    /// 读取密码（Java 中可为 `null`）。
    ///
    /// 对应 Java: `private String m_readPassword`
    read_password: Option<String>,
}

impl MPPReader {
    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法（`new MPPReader()`）
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
            use_raw_timephased_data: false,
            // Java: private boolean m_readPresentationData = true;
            read_presentation_data: true,
            read_properties_only: false,
            // Java: private boolean m_respectPasswordProtection = true;
            respect_password_protection: true,
            read_password: None,
        }
    }

    /// 从输入流读取一份 MPP 文件。
    ///
    /// 对应 Java: `@Override public ProjectFile read(InputStream is) throws MPXJException`
    ///
    /// Java 直接 `new POIFSFileSystem(is)`；Rust 侧 [`PoifsFileSystem::new`] 要求输入源
    /// `'static`，故先把流读入内存（见文件头差异表）。
    pub fn read_stream(&self, input_stream: &mut dyn Read) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        // Java: return read(new POIFSFileSystem(is));
        // （Java 的 catch (IOException ex) → new MPXJException(READ_ERROR, ex) 由 ? 承担）
        let cursor: Cursor<Vec<u8>> = read_stream_into_cursor(input_stream)?;
        let fs = PoifsFileSystem::new(cursor)?;
        self.read(&fs)
    }

    /// 窥探 OLE 复合文档，取出其中的文件格式名。
    ///
    /// 对应 Java: `public static String getFileFormat(POIFSFileSystem fs) throws IOException`
    ///
    /// 供 `UniversalProjectReader` 判断该 OLE 复合文档是否是 MPP、还是别的类型。
    /// Java 返回 `""`（无 `\1CompObj` 流）或 `null`（CompObj 未记录格式）；
    /// Rust 侧两者统一为 `None`（见文件头差异表）。
    pub fn get_file_format(fs: &PoifsFileSystem) -> Option<String> {
        let root = fs.get_root();

        // Java: if (root.getEntryNames().contains("\1CompObj")) { ... }
        if !root.get_entry_names().contains(&COMP_OBJ.to_owned()) {
            return None;
        }

        // Java: CompObj compObj = new CompObj(new DocumentInputStream((DocumentEntry) root.getEntry("\1CompObj")));
        //       fileFormat = compObj.getFileFormat();
        let document = root.get_document(COMP_OBJ)?;
        let comp_obj = CompObj::new(&document.get_data()).ok()?;
        comp_obj.get_file_format().map(str::to_owned)
    }

    /// 读取一份已经打开（或由调用方构造）的 OLE 复合文档。
    ///
    /// 对应 Java: `public ProjectFile read(POIFSFileSystem fs) throws MPXJException`
    pub fn read(&self, fs: &PoifsFileSystem) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        // Java: try { ... } catch (IOException | InstantiationException | IllegalAccessException ex)
        //       { throw new MPXJException(MPXJException.READ_ERROR, ex); }
        // Rust 侧 I/O 失败由 ? 转成 READ_ERROR；另外两个异常来自反射 newInstance()，反射已跳过。

        // Java: ProjectFile projectFile = new ProjectFile();
        let project_file = ProjectFile::new();

        // Java: ProjectConfig config = projectFile.getProjectConfig(); config.setAutoXXX(false) ...
        self.set_auto_ids(&project_file, false)?;

        // Java: addListenersToProject(projectFile);
        self.add_listeners_to_project(&project_file);

        //
        // Java: 打开文件系统并取根目录
        // DirectoryEntry root = fs.getRoot();
        //
        let root = fs.get_root();

        //
        // Java: 取出 CompObj 数据、校验文件格式、分派处理
        // CompObj compObj = new CompObj(new DocumentInputStream((DocumentEntry) root.getEntry("\1CompObj")));
        //
        let comp_obj = Self::read_comp_obj(&root)?;

        // Java: ProjectProperties projectProperties = projectFile.getProjectProperties();
        //       projectProperties.setFullApplicationName(compObj.getApplicationName());
        //       projectProperties.setApplicationVersion(compObj.getApplicationVersion());
        {
            let mut file_ref = borrow_file_mut(&project_file)?;
            let project_properties = file_ref.get_project_properties_mut();
            project_properties
                .set_full_application_name(Some(comp_obj.get_application_name().to_owned()));
            project_properties.set_application_version(comp_obj.get_application_version());
        }

        // Java:
        // String format = compObj.getFileFormat();
        // Class<? extends MPPVariantReader> readerClass = FILE_CLASS_MAP.get(format);
        // if (readerClass == null) { throw new MPXJException(MPXJException.INVALID_FILE + ": " + format); }
        let format = comp_obj.get_file_format();
        let Some(variant_reader) = file_class_map(format) else {
            return Err(MpxjError::new(format!(
                "{}: {}",
                MpxjError::INVALID_FILE,
                format.unwrap_or(JAVA_NULL)
            )));
        };

        // Java: MPPVariantReader reader = readerClass.newInstance();
        //       reader.process(this, projectFile, root);
        variant_reader.process(self, &project_file, &root)?;

        //
        // Java: 更新内部结构 —— 顺带为任务生成大纲编号（MPP 文件里似乎没有该数据）
        // config.setAutoOutlineNumber(true);
        // projectFile.updateStructure();
        // config.setAutoOutlineNumber(false);
        //
        self.set_auto_outline_number(&project_file, true)?;
        borrow_file_mut(&project_file)?.update_structure();
        self.set_auto_outline_number(&project_file, false)?;

        //
        // Java: 后处理 —— 设置汇总标志，并清理「拆分列表为空」的任务实例
        // for (Task task : projectFile.getTasks())
        // {
        //    task.setSummary(task.hasChildTasks() || task.getExternalProject());
        //    validationRelations(task);
        //    copyEstimatedBaselineFields(task);
        // }
        //
        // Rust 侧先克隆任务列表：`validationRelations` 最终会触发 `Task::removePredecessor`，
        // 后者内部要 `file.borrow_mut()`（Java 无此限制，见文件头差异表）。
        let tasks: Vec<Rc<RefCell<Task>>> = {
            let file_ref = borrow_file(&project_file)?;
            file_ref.get_tasks().iter().cloned().collect()
        };

        for task in &tasks {
            // Java: task.setSummary(task.hasChildTasks() || task.getExternalProject());
            let summary = match task.try_borrow() {
                Ok(task_ref) => task_ref.has_child_tasks() || task_ref.get_external_project(),
                // Java 无此分支（元素必然可读）；Rust 侧避免借用冲突 panic
                Err(_) => continue,
            };

            if let Ok(mut task_ref) = task.try_borrow_mut() {
                task_ref.set_summary(summary);
            }

            self.validation_relations(task);
            Self::copy_estimated_baseline_fields(task);
        }

        //
        // Java: 清理未被使用的资源日历
        // Map<Integer, List<Resource>> resourceCalendarMap = projectFile.getResources().stream()
        //    .filter(r -> r.getCalendarUniqueID() != null)
        //    .collect(Collectors.groupingBy(Resource::getCalendarUniqueID));
        // projectFile.getCalendars().removeIf(c -> c.isDerived() && !resourceCalendarMap.containsKey(c.getUniqueID()));
        //
        // 键类型为 `Option<i32>`：`HashMap` 允许 `null` 键，与 Java 的 `Integer` 键一致
        // （后续 `computeIfAbsent(null, ..)` 会插入一个空列表，见下）。
        let mut resource_calendar_map: HashMap<Option<i32>, Vec<Rc<RefCell<Resource>>>> = HashMap::new();
        {
            let context = borrow_context(&project_file)?;
            let Ok(context_ref) = context.try_borrow() else {
                return Err(borrow_conflict());
            };

            for resource in context_ref.get_resources().iter() {
                let Ok(resource_ref) = resource.try_borrow() else {
                    // Java 无此分支；Rust 侧避免借用冲突 panic
                    continue;
                };

                if let Some(unique_id) = resource_ref.get_calendar_unique_id() {
                    resource_calendar_map
                        .entry(Some(unique_id))
                        .or_default()
                        .push(Rc::clone(resource));
                }
            }
        }

        {
            let context = borrow_context(&project_file)?;
            let Ok(mut context_ref) = context.try_borrow_mut() else {
                return Err(borrow_conflict());
            };
            let calendars = context_ref.get_calendars_mut();

            // Java 的 removeIf 谓词
            let mut to_remove: Vec<usize> = Vec::new();
            for index in 0..calendars.len() {
                let Some(calendar) = calendars.get(index) else {
                    continue;
                };
                let Ok(calendar_ref) = calendar.try_borrow() else {
                    continue;
                };

                if calendar_ref.is_derived()
                    && !resource_calendar_map.contains_key(&calendar_ref.get_unique_id())
                {
                    to_remove.push(index);
                }
            }

            // 逆序按下标移除（等价于 removeIf 逐个移除）
            for index in to_remove.into_iter().rev() {
                let Some(calendar) = calendars.get(index) else {
                    continue;
                };
                if let Ok(calendar_ref) = calendar.try_borrow() {
                    calendars.remove(&calendar_ref);
                }
            }
        }

        //
        // Java: 资源后处理
        // UnitOfMeasureContainer unitsOfMeasure = projectFile.getUnitsOfMeasure();
        // for (Resource resource : projectFile.getResources()) { ... }
        //
        let resources: Vec<Rc<RefCell<Resource>>> = {
            let context = borrow_context(&project_file)?;
            let Ok(context_ref) = context.try_borrow() else {
                return Err(borrow_conflict());
            };
            context_ref.get_resources().iter().cloned().collect()
        };

        for resource in &resources {
            // Java: ProjectCalendar calendar = resource.getCalendar();
            let calendar: Option<Rc<RefCell<ProjectCalendar>>> = match resource.try_borrow() {
                Ok(resource_ref) => resource_ref.get_calendar(),
                Err(_) => None,
            };

            if let Some(calendar) = calendar {
                // Java: if (calendar.isDerived()) { calendar.setType(CalendarType.RESOURCE);
                //       calendar.setPersonal(resourceCalendarMap.computeIfAbsent(calendar.getUniqueID(), k -> Collections.emptyList()).size() == 1); }
                let is_derived = calendar
                    .try_borrow()
                    .map(|calendar_ref| calendar_ref.is_derived())
                    .unwrap_or(false);

                if is_derived {
                    let unique_id = calendar
                        .try_borrow()
                        .ok()
                        .and_then(|calendar_ref| calendar_ref.get_unique_id());
                    let personal = resource_calendar_map
                        .entry(unique_id)
                        .or_default()
                        .len()
                        == 1;

                    if let Ok(mut calendar_ref) = calendar.try_borrow_mut() {
                        calendar_ref.set_type(Some(CalendarType::Resource));
                        calendar_ref.set_personal(personal);
                    }
                }

                // Java: 资源日历没有名字时继承资源名
                // if (calendar.getName() == null || calendar.getName().isEmpty()) { ... }
                let name_missing = calendar
                    .try_borrow()
                    .map(|calendar_ref| {
                        calendar_ref
                            .get_name()
                            .map(|name| name.is_empty())
                            .unwrap_or(true)
                    })
                    .unwrap_or(true);

                if name_missing {
                    let name = match resource.try_borrow() {
                        Ok(resource_ref) => resource_ref.get_name(),
                        Err(_) => None,
                    };

                    let name = match name {
                        Some(name) if !name.is_empty() => name,
                        // Java: name = "Unnamed Resource";
                        _ => UNNAMED_RESOURCE.to_owned(),
                    };

                    if let Ok(mut calendar_ref) = calendar.try_borrow_mut() {
                        calendar_ref.set_name(Some(name));
                    }
                }
            }

            // Java: UnitOfMeasure uom = unitsOfMeasure.getOrCreateByAbbreviation(resource.getMaterialLabel());
            //       if (uom != null) { resource.setUnitOfMeasure(uom); }
            let material_label = match resource.try_borrow() {
                Ok(resource_ref) => resource_ref.get_material_label(),
                Err(_) => None,
            };

            let unit_of_measure = {
                let context = borrow_context(&project_file)?;
                let Ok(mut context_ref) = context.try_borrow_mut() else {
                    return Err(borrow_conflict());
                };
                context_ref
                    .get_units_of_measure_mut()
                    .get_or_create_by_abbreviation(material_label.as_deref())
                    .cloned()
            };

            if let Some(unit_of_measure) = unit_of_measure {
                if let Ok(mut resource_ref) = resource.try_borrow_mut() {
                    resource_ref.set_unit_of_measure(Some(&unit_of_measure));
                }
            }
        }

        //
        // Java: 补充分析信息
        // String projectFilePath = projectFile.getProjectProperties().getProjectFilePath();
        // if (projectFilePath != null && projectFilePath.startsWith("<>\\")) { ... }
        //
        {
            let file_application = {
                let file_ref = borrow_file(&project_file)?;
                let project_file_path = file_ref.get_project_properties().get_project_file_path();

                match project_file_path {
                    Some(path) if path.starts_with(PROJECT_SERVER_PATH_PREFIX) => {
                        FILE_APPLICATION_PROJECT_SERVER
                    }
                    _ => FILE_APPLICATION_MICROSOFT,
                }
            };

            let mut file_ref = borrow_file_mut(&project_file)?;
            let project_properties = file_ref.get_project_properties_mut();
            project_properties.set_file_application(Some(file_application.to_owned()));
            project_properties.set_file_type(Some(FILE_TYPE_MPP.to_owned()));
        }

        // Java: projectFile.readComplete();
        borrow_file_mut(&project_file)?.read_complete();

        // Java: return projectFile;
        Ok(project_file)
    }

    /// 校验任务的全部关系，剔除那些从 MPP 文件里错误读出、指向父任务的关系。
    ///
    /// 对应 Java: `private void validationRelations(Task task)`
    fn validation_relations(&self, task: &Rc<RefCell<Task>>) {
        // Java: List<Relation> predecessors = task.getPredecessors();
        //       if (predecessors.isEmpty()) { return; }
        let predecessors = match task.try_borrow() {
            Ok(task_ref) => task_ref.get_predecessors(),
            // Java 无此分支；Rust 侧避免借用冲突 panic
            Err(_) => return,
        };

        let mut invalid = Vec::new();
        for relation in &predecessors {
            // Java: Task sourceTask = relation.getSuccessorTask();
            //       Task targetTask = relation.getPredecessorTask();
            let (source_task, target_task) = match relation.try_borrow() {
                Ok(relation_ref) => (
                    relation_ref.get_successor_task(),
                    relation_ref.get_predecessor_task(),
                ),
                // Java 无此分支；Rust 侧避免借用冲突 panic
                Err(_) => continue,
            };

            let (Some(source_task), Some(target_task)) = (source_task, target_task) else {
                // Java 在 Task 为 null 时会在 getOutlineNumber() 处抛 NullPointerException；
                // Rust 侧按「无法判定」处理（不加入无效列表）
                continue;
            };

            // Java: String sourceOutlineNumber = sourceTask.getOutlineNumber();
            //       String targetOutlineNumber = targetTask.getOutlineNumber();
            let source_outline_number = source_task
                .try_borrow()
                .ok()
                .and_then(|task_ref| task_ref.get_outline_number());
            let target_outline_number = target_task
                .try_borrow()
                .ok()
                .and_then(|task_ref| task_ref.get_outline_number());

            // Java: if (sourceOutlineNumber != null && targetOutlineNumber != null
            //            && sourceOutlineNumber.startsWith(targetOutlineNumber + '.'))
            if let (Some(source_outline_number), Some(target_outline_number)) =
                (source_outline_number, target_outline_number)
            {
                if source_outline_number.starts_with(&format!("{target_outline_number}.")) {
                    invalid.push(Rc::clone(relation));
                }
            }
        }

        // Java: for (Relation relation : invalid)
        //       { relation.getSuccessorTask().removePredecessor(relation.getPredecessorTask(), relation.getType(), relation.getLag()); }
        for relation in invalid {
            let (successor_task, predecessor_task, relation_type, lag) =
                match relation.try_borrow() {
                    Ok(relation_ref) => (
                        relation_ref.get_successor_task(),
                        relation_ref.get_predecessor_task(),
                        relation_ref.get_type(),
                        relation_ref.get_lag(),
                    ),
                    // Java 无此分支；Rust 侧避免借用冲突 panic
                    Err(_) => continue,
                };

            if let Some(successor_task) = successor_task {
                if let Ok(successor_ref) = successor_task.try_borrow() {
                    successor_ref.remove_predecessor(
                        predecessor_task.as_ref(),
                        relation_type,
                        lag,
                    );
                }
            }
        }
    }

    /// 基线字段为空、但其「估算版本」有值时，回退到估算字段。
    ///
    /// 对应 Java: `private void copyEstimatedBaselineFields(Task task)`
    fn copy_estimated_baseline_fields(task: &Rc<RefCell<Task>>) {
        for (key, estimated_key) in TASK_ESTIMATED_BASELINE_FIELDS {
            // Java: Object value = task.getCachedValue(entry.getKey());
            //       if (value == null) { value = task.getCachedValue(entry.getValue());
            //                            if (value != null) { task.set(entry.getKey(), value); } }
            let value = {
                let Ok(task_ref) = task.try_borrow() else {
                    // Java 无此分支；Rust 侧避免借用冲突 panic
                    continue;
                };

                // 借用必须先释放才能 `set`，故此处直接把值克隆出来
                match task_ref.get_cached_value((*key).into()) {
                    Some(value) => Some(value.clone()),
                    None => task_ref
                        .get_cached_value((*estimated_key).into())
                        .map(|value| value.clone()),
                }
            };

            if let Some(value) = value {
                if let Ok(mut task_ref) = task.try_borrow_mut() {
                    task_ref.set((*key).into(), Some(value));
                }
            }
        }
    }

    /// 读取并解析 `\1CompObj` 流。
    ///
    /// Java 在 `read(POIFSFileSystem)` 内联完成；Rust 侧抽成私有辅助函数，
    /// 因为「取流」与「解析」都要处理失败路径（Java 是运行时异常）。
    fn read_comp_obj(root: &DirectoryEntry) -> MpxjResult<CompObj> {
        // Java: new CompObj(new DocumentInputStream((DocumentEntry) root.getEntry("\1CompObj")))
        let Some(document) = root.get_document(COMP_OBJ) else {
            // Java 侧是 NullPointerException / ClassCastException（未捕获的运行时异常）
            return Err(MpxjError::new(MpxjError::INVALID_FILE));
        };

        CompObj::new(&document.get_data())
    }

    /// 关闭/打开 `ProjectConfig` 的自动编号开关（`config.setAutoXXX(flag)`）。
    ///
    /// Java 在 `read(POIFSFileSystem)` 内直接对 `ProjectConfig` 逐个赋值；
    /// Rust 侧抽成辅助函数以减少重复的借取样板代码。
    fn set_auto_ids(&self, project_file: &Rc<RefCell<ProjectFile>>, flag: bool) -> MpxjResult<()> {
        let context = borrow_context(project_file)?;
        let Ok(mut context_ref) = context.try_borrow_mut() else {
            return Err(borrow_conflict());
        };
        let config = context_ref.get_project_config_mut();

        config.set_auto_task_id(flag);
        config.set_auto_task_unique_id(flag);
        config.set_auto_resource_id(flag);
        config.set_auto_resource_unique_id(flag);
        config.set_auto_outline_level(flag);
        config.set_auto_outline_number(flag);
        config.set_auto_wbs(flag);
        config.set_auto_calendar_unique_id(flag);
        config.set_auto_assignment_unique_id(flag);
        config.set_auto_relation_unique_id(flag);

        Ok(())
    }

    /// 设置 `config.setAutoOutlineNumber(flag)`。
    ///
    /// Java 在 `read(POIFSFileSystem)` 内直接赋值；Rust 侧抽成辅助函数。
    fn set_auto_outline_number(
        &self,
        project_file: &Rc<RefCell<ProjectFile>>,
        flag: bool,
    ) -> MpxjResult<()> {
        let context = borrow_context(project_file)?;
        let Ok(mut context_ref) = context.try_borrow_mut() else {
            return Err(borrow_conflict());
        };
        context_ref
            .get_project_config_mut()
            .set_auto_outline_number(flag);

        Ok(())
    }

    /// 若本标志为 `true`，则直接取用 MS Project 的原始时间分段数据，不做归一化。
    ///
    /// 对应 Java: `public boolean getUseRawTimephasedData()`
    pub fn get_use_raw_timephased_data(&self) -> bool {
        self.use_raw_timephased_data
    }

    /// 设置「直接取用原始时间分段数据」标志。
    ///
    /// 对应 Java: `public void setUseRawTimephasedData(boolean useRawTimephasedData)`
    pub fn set_use_raw_timephased_data(&mut self, use_raw_timephased_data: bool) {
        self.use_raw_timephased_data = use_raw_timephased_data;
    }

    /// 是否读取 MPP 文件中的展示数据（不读取可省时间与内存）。
    ///
    /// 对应 Java: `public boolean getReadPresentationData()`
    pub fn get_read_presentation_data(&self) -> bool {
        self.read_presentation_data
    }

    /// 设置「读取展示数据」标志。
    ///
    /// 对应 Java: `public void setReadPresentationData(boolean readPresentationData)`
    pub fn set_read_presentation_data(&mut self, read_presentation_data: bool) {
        self.read_presentation_data = read_presentation_data;
    }

    /// 是否只读取项目属性（用于快速访问文档属性）。
    ///
    /// 对应 Java: `public boolean getReadPropertiesOnly()`
    pub fn get_read_properties_only(&self) -> bool {
        self.read_properties_only
    }

    /// 设置「只读取项目属性」标志。
    ///
    /// 对应 Java: `public void setReadPropertiesOnly(boolean readPropertiesOnly)`
    pub fn set_read_properties_only(&mut self, read_properties_only: bool) {
        self.read_properties_only = read_properties_only;
    }

    /// 设置读取密码（读取受密码保护的 Project 文件时需要）。
    ///
    /// 对应 Java: `public void setReadPassword(String password)`
    ///
    /// Java 的 `null` → `None`。
    pub fn set_read_password(&mut self, password: Option<String>) {
        self.read_password = password;
    }

    /// 取读取密码（内部使用：受密码保护的 Project 文件需要它才能读取）。
    ///
    /// 对应 Java: `public String getReadPassword()`
    pub fn get_read_password(&self) -> Option<&str> {
        self.read_password.as_deref()
    }

    /// 在支持该选项的格式上设为 `false` 可忽略密码保护。
    ///
    /// 对应 Java: `public void setRespectPasswordProtection(boolean respectPasswordProtection)`
    pub fn set_respect_password_protection(&mut self, respect_password_protection: bool) {
        self.respect_password_protection = respect_password_protection;
    }

    /// 是否尊重密码保护。
    ///
    /// 对应 Java: `public boolean getRespectPasswordProtection()`
    pub fn get_respect_password_protection(&self) -> bool {
        self.respect_password_protection
    }
}

impl Default for MPPReader {
    /// 对应 Java: 隐式默认构造方法。
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectReader for MPPReader {
    /// 注册监听器。
    ///
    /// 对应 Java: `@Override public void addProjectListener(ProjectListener listener)`（继承自 `AbstractProjectReader`）
    fn add_project_listener(&mut self, listener: Box<dyn crate::listener::project_listener::ProjectListener>) {
        self.add_project_listener_default(listener);
    }

    /// 按路径读取一份 schedule。
    ///
    /// 对应 Java: `@Override public ProjectFile read(File file)`
    ///
    /// Java 特意覆盖基类版本（而不是走 `AbstractProjectStreamReader` 的「开文件得流」），
    /// 因为直接对 `File` 建 `POIFSFileSystem` 内存占用更低、也能打开更大的 MPP 文件。
    /// Rust 侧同样直接把 [`std::fs::File`] 交给 [`PoifsFileSystem::new`]。
    fn read_file(&mut self, path: &Path) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        // Java: fs = new POIFSFileSystem(file); return read(fs);
        // （Java 的 catch (IOException ex) → new MPXJException(READ_ERROR, ex) 由 ? 承担）
        let handle = File::open(path)?;
        let fs = PoifsFileSystem::new(handle)?;

        // Java: finally { AutoCloseableHelper.closeQuietly(fs); }
        // —— Rust 侧 `fs` 随作用域 Drop（`PoifsFileSystem` 持有文件句柄）。
        self.read(&fs)
    }

    /// 按路径读取全部 schedule（MPP 只含单份 schedule，故返回单元素列表）。
    ///
    /// 对应 Java: `AbstractProjectStreamReader.readAll(File file)`
    fn read_all_file(&mut self, path: &Path) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
        self.read_all_file_default(path)
    }

    /// 从输入流读取一份 schedule。
    ///
    /// 对应 Java: `@Override public ProjectFile read(InputStream inputStream)`
    fn read_reader(&mut self, reader: &mut dyn Read) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        self.read_stream(reader)
    }

    /// 从输入流读取全部 schedule（MPP 只含单份 schedule，故返回单元素列表）。
    ///
    /// 对应 Java: `AbstractProjectStreamReader.readAll(InputStream inputStream)`
    fn read_all_reader(
        &mut self,
        reader: &mut dyn Read,
    ) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
        self.read_all_reader_default(reader)
    }
}

impl AbstractProjectReader for MPPReader {
    /// 取已注册的监听器列表。
    ///
    /// Rust 特有（Java 的 `m_projectListeners` 是私有字段）。
    fn project_listeners(&self) -> &[SharedProjectListener] {
        &self.listeners
    }

    /// 取已注册的监听器列表（可变）。
    ///
    /// Rust 特有（同上）。
    fn project_listeners_mut(&mut self) -> &mut Vec<SharedProjectListener> {
        &mut self.listeners
    }
}

impl AbstractProjectStreamReader for MPPReader {}

/// 对应 Java: `private static final Map<String, Class<? extends MPPVariantReader>> FILE_CLASS_MAP`
/// 与其静态初始化块，以及 `read(POIFSFileSystem)` 中的
/// `readerClass = FILE_CLASS_MAP.get(format)` / `readerClass.newInstance()`。
///
/// # 反射的还原
///
/// Java 保存 `Class` 对象并反射创建实例；Rust 没有反射，改为「按格式名匹配后直接构造」。
/// 对外行为与 Java 一致：已登记的格式名 → 对应变体 reader；未登记的格式（含 `null`）→ `None`
/// （对应 Java 的 `null`）。副作用是 Java 侧 `newInstance()` 可能抛出的
/// `InstantiationException` / `IllegalAccessException` 两个失败分支随之消失。
///
/// # 对相关模块的接口假设
///
/// 见本文件头「对变体 reader 的接口假设」：四个变体类型各有无参构造方法 `new()` 并实现
/// [`MPPVariantReader`]。
fn file_class_map(format: Option<&str>) -> Option<Box<dyn MPPVariantReader>> {
    // Java: FILE_CLASS_MAP.get(format) —— 键为 null 时返回 null
    let format = format?;

    match format {
        // FILE_CLASS_MAP.put("MSProject.MPP9", MPP9Reader.class);
        // FILE_CLASS_MAP.put("MSProject.MPT9", MPP9Reader.class);
        // FILE_CLASS_MAP.put("MSProject.GLOBAL9", MPP9Reader.class);
        "MSProject.MPP9" | "MSProject.MPT9" | "MSProject.GLOBAL9" => {
            Some(Box::new(MPP9Reader::new()))
        }

        // FILE_CLASS_MAP.put("MSProject.MPP8", MPP8Reader.class);
        // FILE_CLASS_MAP.put("MSProject.MPT8", MPP8Reader.class);
        "MSProject.MPP8" | "MSProject.MPT8" => Some(Box::new(MPP8Reader::new())),

        // FILE_CLASS_MAP.put("MSProject.MPP12", MPP12Reader.class);
        // FILE_CLASS_MAP.put("MSProject.MPT12", MPP12Reader.class);
        // FILE_CLASS_MAP.put("MSProject.GLOBAL12", MPP12Reader.class);
        "MSProject.MPP12" | "MSProject.MPT12" | "MSProject.GLOBAL12" => {
            Some(Box::new(MPP12Reader::new()))
        }

        // FILE_CLASS_MAP.put("MSProject.MPP14", MPP14Reader.class);
        // FILE_CLASS_MAP.put("MSProject.MPT14", MPP14Reader.class);
        // FILE_CLASS_MAP.put("MSProject.GLOBAL14", MPP14Reader.class);
        "MSProject.MPP14" | "MSProject.MPT14" | "MSProject.GLOBAL14" => {
            Some(Box::new(MPP14Reader::new()))
        }

        // Java: 未登记 → Map.get 返回 null → read 抛 INVALID_FILE + ": " + format
        _ => None,
    }
}

/// 借用项目文件（Rust 特有：Java 直接持有对象引用，无借用概念）。
///
/// 对应 Java: 无（`ProjectFile projectFile` 局部变量本身）
fn borrow_file(file: &Rc<RefCell<ProjectFile>>) -> MpxjResult<Ref<'_, ProjectFile>> {
    file.try_borrow().map_err(|_| borrow_conflict())
}

/// 可变借用项目文件（Rust 特有）。
///
/// 对应 Java: 无（同上）
fn borrow_file_mut(file: &Rc<RefCell<ProjectFile>>) -> MpxjResult<RefMut<'_, ProjectFile>> {
    file.try_borrow_mut().map_err(|_| borrow_conflict())
}

/// 取项目文件的项目上下文（Rust 特有）。
///
/// 对应 Java: `projectFile.getProjectContext()`（Java 无借用失败的可能）
fn borrow_context(file: &Rc<RefCell<ProjectFile>>) -> MpxjResult<Rc<RefCell<ProjectContext>>> {
    Ok(borrow_file(file)?.get_project_context())
}

/// `RefCell` 借用冲突时的错误（Rust 特有：Java 是稳定引用，不存在该情形）。
///
/// 借用冲突意味着「调用方已持有可变/不可变引用」，读取无法继续，故按读取失败返回。
fn borrow_conflict() -> MpxjError {
    MpxjError::new(MpxjError::READ_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Write;

    use cfb::CompoundFile;

    /// 在复合文档里写入指定的流（名字含控制字符也没问题）。
    fn compound_file_with_streams(streams: &[(&str, &[u8])]) -> Vec<u8> {
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).expect("创建复合文档");

        for (name, data) in streams {
            file.create_stream(name)
                .expect("创建流")
                .write_all(data)
                .expect("写入流");
        }

        file.into_inner().into_inner()
    }

    /// 按 CompObj 的布局拼出 `\1CompObj` 流的内容。
    ///
    /// 与 `comp_obj.rs` 的测试常量保持一致：28 字节填充 + 长度前缀字符串。
    fn comp_obj_payload(application_name: &str, file_format: Option<&str>) -> Vec<u8> {
        let mut data = vec![0u8; 28];
        append_string(&mut data, application_name);

        if let Some(file_format) = file_format {
            append_string(&mut data, file_format);
            // applicationID（长度 0 表示没有）
            data.extend_from_slice(&0i32.to_le_bytes());
        }

        data
    }

    /// 追加「长度（含结尾 0 字节）+ 内容 + 0 字节」。
    fn append_string(data: &mut Vec<u8>, value: &str) {
        data.extend_from_slice(&((value.len() + 1) as i32).to_le_bytes());
        data.extend_from_slice(value.as_bytes());
        data.push(0);
    }

    /// 打开一份内存中的复合文档。
    fn open(bytes: Vec<u8>) -> MpxjResult<PoifsFileSystem> {
        Ok(PoifsFileSystem::new(Cursor::new(bytes))?)
    }

    #[test]
    fn get_file_format_reads_the_comp_obj_stream() {
        let bytes = compound_file_with_streams(&[(
            "\u{1}CompObj",
            &comp_obj_payload("Microsoft Project 14.0", Some("MSProject.MPP14")),
        )]);
        let fs = open(bytes).expect("打开复合文档");

        // Java: root.getEntryNames().contains("\1CompObj") 成立 → 解析 CompObj
        assert_eq!(
            MPPReader::get_file_format(&fs),
            Some("MSProject.MPP14".to_owned())
        );
    }

    #[test]
    fn get_file_format_is_none_without_the_comp_obj_stream() {
        // 没有任何流的复合文档：Java 返回 ""（Rust 侧统一为 None）
        let fs = open(compound_file_with_streams(&[])).expect("打开复合文档");
        assert_eq!(MPPReader::get_file_format(&fs), None);

        // 有别的流、但没有 \1CompObj：同样返回 None
        let fs = open(compound_file_with_streams(&[("Props", b"props")])).expect("打开复合文档");
        assert_eq!(MPPReader::get_file_format(&fs), None);
    }

    #[test]
    fn get_file_format_is_none_when_comp_obj_has_no_format() {
        // applicationName 不是 "Microsoft Project 4.0" 且 fileFormat 长度为 0
        let mut payload = vec![0u8; 28];
        append_string(&mut payload, "Microsoft Project 9.0");
        payload.extend_from_slice(&0i32.to_le_bytes());

        let bytes = compound_file_with_streams(&[("\u{1}CompObj", &payload)]);
        let fs = open(bytes).expect("打开复合文档");

        // Java: fileFormat = compObj.getFileFormat()（null）→ 返回 null
        assert_eq!(MPPReader::get_file_format(&fs), None);
    }

    #[test]
    fn get_file_format_is_none_for_a_damaged_comp_obj_stream() {
        // 截断的 \1CompObj：Java 抛 EOFException；Rust 侧返回 None（不 panic）
        let bytes = compound_file_with_streams(&[("\u{1}CompObj", &[0u8; 4])]);
        let fs = open(bytes).expect("打开复合文档");

        assert_eq!(MPPReader::get_file_format(&fs), None);
    }

    #[test]
    fn read_reports_invalid_file_for_an_unregistered_format() {
        let bytes = compound_file_with_streams(&[(
            "\u{1}CompObj",
            &comp_obj_payload("Microsoft Project 14.0", Some("MSProject.MPP99")),
        )]);
        let fs = open(bytes).expect("打开复合文档");

        let error = MPPReader::new()
            .read(&fs)
            .err()
            .expect("未登记的格式应报错");

        // Java: throw new MPXJException(MPXJException.INVALID_FILE + ": " + format);
        assert_eq!(
            error.message(),
            "Invalid file format: MSProject.MPP99".to_owned()
        );
    }

    #[test]
    fn read_reports_invalid_file_when_the_format_is_null() {
        // fileFormat 缺失 → Java 的 FILE_CLASS_MAP.get(null) 返回 null → "Invalid file format: null"
        let mut payload = vec![0u8; 28];
        append_string(&mut payload, "Microsoft Project 9.0");
        payload.extend_from_slice(&0i32.to_le_bytes());

        let bytes = compound_file_with_streams(&[("\u{1}CompObj", &payload)]);
        let fs = open(bytes).expect("打开复合文档");

        let error = MPPReader::new().read(&fs).err().expect("应报错");
        assert_eq!(error.message(), "Invalid file format: null".to_owned());
    }

    #[test]
    fn read_reports_an_error_when_the_comp_obj_stream_is_missing() {
        // Java 侧此处是 NullPointerException（未捕获）；Rust 侧转为 Err，不 panic
        let fs = open(compound_file_with_streams(&[])).expect("打开复合文档");

        let error = MPPReader::new().read(&fs).err().expect("应报错");
        assert_eq!(error.message(), MpxjError::INVALID_FILE);
    }

    #[test]
    fn empty_and_damaged_files_are_reported_as_read_errors() {
        // 空输入：POI 构造 POIFSFileSystem 时抛 IOException → READ_ERROR
        assert!(PoifsFileSystem::new(Cursor::new(Vec::new())).is_err());
        // 16 个 0 字节的垃圾数据：同样打不开（不 panic）
        assert!(PoifsFileSystem::new(Cursor::new(vec![0u8; 16])).is_err());

        // `read(InputStream)` 路径：走 read_stream_into_cursor + PoifsFileSystem::new
        let mut reader = MPPReader::new();
        let error = reader
            .read_reader(&mut &b"not an OLE compound document"[..])
            .err()
            .expect("应报错");
        assert!(error.message().starts_with(MpxjError::READ_ERROR));

        // 空流
        assert!(reader.read_reader(&mut std::io::empty()).is_err());
    }

    #[test]
    fn read_file_uses_the_file_based_entry_point() {
        let path = std::env::temp_dir().join(format!(
            "mpxj-mpp-reader-test-{}.mpp",
            std::process::id()
        ));
        let bytes = compound_file_with_streams(&[(
            "\u{1}CompObj",
            &comp_obj_payload("Microsoft Project 9.0", Some("MSProject.MPP99")),
        )]);
        std::fs::write(&path, bytes).expect("写入测试文件");

        let error = MPPReader::new()
            .read_file(&path)
            .err()
            .expect("未登记的格式应报错");

        assert_eq!(
            error.message(),
            "Invalid file format: MSProject.MPP99".to_owned()
        );

        let _ = std::fs::remove_file(&path);

        // 不存在的路径：Java 抛 IOException → READ_ERROR
        let error = MPPReader::new()
            .read_file(Path::new("no-such-dir/no-such-file.mpp"))
            .err()
            .expect("不存在的路径应报错");
        assert!(error.message().starts_with(MpxjError::READ_ERROR));
    }

    #[test]
    fn reader_flags_default_and_round_trip() {
        let mut reader = MPPReader::new();

        // Java: private boolean m_readPresentationData = true; m_respectPasswordProtection = true;
        assert!(!reader.get_use_raw_timephased_data());
        assert!(reader.get_read_presentation_data());
        assert!(!reader.get_read_properties_only());
        assert!(reader.get_respect_password_protection());
        assert_eq!(reader.get_read_password(), None);

        reader.set_use_raw_timephased_data(true);
        reader.set_read_presentation_data(false);
        reader.set_read_properties_only(true);
        reader.set_respect_password_protection(false);
        reader.set_read_password(Some("secret".to_owned()));

        assert!(reader.get_use_raw_timephased_data());
        assert!(!reader.get_read_presentation_data());
        assert!(reader.get_read_properties_only());
        assert!(!reader.get_respect_password_protection());
        assert_eq!(reader.get_read_password(), Some("secret"));

        // Java 的 setReadPassword(null) 等价于 None
        reader.set_read_password(None);
        assert_eq!(reader.get_read_password(), None);
    }

    #[test]
    fn file_class_map_covers_exactly_the_java_entries() {
        // FILE_CLASS_MAP 的 11 个键都应能选出变体 reader
        for format in [
            "MSProject.MPP9",
            "MSProject.MPT9",
            "MSProject.GLOBAL9",
            "MSProject.MPP8",
            "MSProject.MPT8",
            "MSProject.MPP12",
            "MSProject.MPT12",
            "MSProject.GLOBAL12",
            "MSProject.MPP14",
            "MSProject.MPT14",
            "MSProject.GLOBAL14",
        ] {
            assert!(
                file_class_map(Some(format)).is_some(),
                "{format} 应能选出变体 reader"
            );
        }

        // 未登记的格式与 null（Java 的 Map.get 返回 null）
        assert!(file_class_map(Some("MSProject.MPP4")).is_none());
        assert!(file_class_map(Some("")).is_none());
        assert!(file_class_map(None).is_none());
    }

    #[test]
    fn baseline_field_table_matches_the_java_static_block() {
        // Java 的 TASK_ESTIMATED_BASELINE_FIELDS 共 11 组 × 3 个字段
        assert_eq!(TASK_ESTIMATED_BASELINE_FIELDS.len(), 33);

        assert_eq!(
            TASK_ESTIMATED_BASELINE_FIELDS[0],
            (TaskField::BaselineDuration, TaskField::BaselineEstimatedDuration)
        );
        assert_eq!(
            TASK_ESTIMATED_BASELINE_FIELDS[32],
            (
                TaskField::Baseline10Finish,
                TaskField::Baseline10EstimatedFinish
            )
        );
    }
}
