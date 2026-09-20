//! 对应 Java: `org/mpxj/mpp/ConstraintFactory.java`
//!
//! 从 MPP9 / MPP12 / MPP14 文件的 `TBkndCons` 存储中读出任务之间的「关系」
//! （Java 侧把这种记录称为 constraint，实际表示的是 predecessor/successor 关系），
//! 写入后继任务并触发关系读取事件。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `class ConstraintFactory`（包级可见） | `pub struct ConstraintFactory` | Rust 没有包级可见性（本 crate 的约定是公开类型） |
//! | `process(DirectoryEntry projectDir, ProjectFile file, DocumentInputStreamFactory inputStreamFactory) throws IOException` | [`ConstraintFactory::process`] 返回 `MpxjResult<()>` | Java 用 `IOException` 表示「流读取失败」；Rust 侧统一用 [`MpxjError`]（`io::Error` 可经 `?` 自动转换） |
//! | `new FixedMeta(new DocumentInputStream((DocumentEntry) consDir.getEntry("FixedMeta")), 10)` | `FixedMeta::with_item_size(&document.get_data(), 10)` | 按项目约定 `InputStream` 一律换成 `&[u8]`；条目不存在时 Java 抛 `FileNotFoundException`，Rust 返回 `Err` |
//! | `new FixedData(consFixedMeta, 20, inputStreamFactory.getInstance(consDir, "FixedData"))` | `FixedData::with_item_size(&meta, 20, &stream.to_bytes())` | 同上 |
//! | `eventManager.fireRelationReadEvent(relation)` | 经 `ProjectFile::get_project_context()` 取 `EventManager` 的可变引用后触发 | `ProjectFile::get_event_manager()` 只返回不可变 `Ref`，无法调用 `&mut self` 的事件方法 |
//! | `ByteArrayHelper.getShort/getInt` 越界 | 先判长度再取，不足则 `continue` | Java 会抛 `ArrayIndexOutOfBoundsException` |
//! | `file.getTaskByUniqueID(...)` 返回 `null` | `Option<Rc<RefCell<Task>>>` | 「`null` → `Option<T>`」约定 |
//!
//! 跳过的 Java 代码及原因：
//!
//! | Java | 原因 |
//! |---|---|
//! | 被注释掉的 `consFixed2Meta` / `consFixed2Data` 相关代码（Java 源码中已注释） | Java 源码中即为注释，不参与运行 |
//! | `task2.addPredecessor(...)` 的 unique ID 自动分配 | 见 `crate::relation::Builder` 的文件头说明（自动分配依赖 `ProjectConfig` 的序号生成器，Rust 侧改由调用方提供 `uniqueID`；此处 Java 本来就显式传入了 unique ID） |
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::application_version::ApplicationVersion;
use crate::mpp::document_input_stream_factory::DocumentInputStreamFactory;
use crate::mpp::fixed_data::FixedData;
use crate::mpp::fixed_meta::FixedMeta;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::ole::DirectoryEntry;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_file::ProjectFile;
use crate::relation::Builder as RelationBuilder;
use crate::relation_type::RelationType;
use crate::task::Task;

/// `TBkndCons` 存储名。
///
/// 对应 Java: `projectDir.getEntry("TBkndCons")`
const CONS_DIRECTORY: &str = "TBkndCons";

/// 关系数据块的最小长度。
///
/// 对应 Java: `if (data.length < 14) { continue; }`
const MINIMUM_DATA_LENGTH: usize = 14;

/// 公共的关系数据读取逻辑。
///
/// 对应 Java: `class ConstraintFactory`
pub struct ConstraintFactory;

impl ConstraintFactory {
    /// 处理关系数据的主入口。
    ///
    /// 对应 Java: `public void process(DirectoryEntry projectDir, ProjectFile file, DocumentInputStreamFactory inputStreamFactory) throws IOException`
    pub fn process(
        &self,
        project_dir: &DirectoryEntry,
        file: &Rc<RefCell<ProjectFile>>,
        input_stream_factory: &DocumentInputStreamFactory,
    ) -> MpxjResult<()> {
        // Java: try { consDir = (DirectoryEntry) projectDir.getEntry("TBkndCons"); }
        //       catch (FileNotFoundException ex) { file.addIgnoredError(ex); consDir = null; }
        let Some(cons_dir) = project_dir.get_directory(CONS_DIRECTORY) else {
            file.borrow_mut().add_ignored_error(MpxjError::new(
                "Entry not found: TBkndCons".to_owned(),
            ));
            return Ok(());
        };

        // Java: FixedMeta consFixedMeta = new FixedMeta(new DocumentInputStream(((DocumentEntry) consDir.getEntry("FixedMeta"))), 10);
        let Some(meta_document) = cons_dir.get_document("FixedMeta") else {
            return Err(MpxjError::new("Entry not found: FixedMeta".to_owned()));
        };
        let meta_data = meta_document.get_data();
        let cons_fixed_meta = FixedMeta::with_item_size(&meta_data, 10)?;

        // Java: FixedData consFixedData = new FixedData(consFixedMeta, 20, inputStreamFactory.getInstance(consDir, "FixedData"));
        let mut fixed_data_stream = input_stream_factory.get_instance(&cons_dir, "FixedData")?;
        let fixed_data_bytes = fixed_data_stream.to_bytes();
        let cons_fixed_data = FixedData::with_item_size(&cons_fixed_meta, 20, &fixed_data_bytes);

        // Java: int count = consFixedMeta.getItemCount();
        let count = cons_fixed_meta.get_item_count();

        // Java: ProjectProperties properties = file.getProjectProperties();
        // Java 在循环前一次性取得属性；Rust 侧此处只为计算下面两个标志借用一次。
        let (mpp_file_type, application_version) = {
            let file_ref = file.borrow();
            let properties = file_ref.get_project_properties();
            (
                properties.get_mpp_file_type(),
                properties.get_application_version(),
            )
        };

        // Java: boolean project15 = NumberHelper.getInt(properties.getMppFileType()) == 14 && NumberHelper.getInt(properties.getApplicationVersion()) > ApplicationVersion.PROJECT_2010;
        let project15 = mpp_file_type.unwrap_or(0) == 14
            && application_version.unwrap_or(0) > ApplicationVersion::PROJECT_2010;
        // Java: int durationUnitsOffset = project15 ? 18 : 14;
        let duration_units_offset: usize = if project15 { 18 } else { 14 };
        // Java: int durationOffset = project15 ? 14 : 16;
        let duration_offset: usize = if project15 { 14 } else { 16 };

        for loop_index in 0..count {
            // Java: byte[] metaData = consFixedMeta.getByteArrayValue(loop);
            let Some(meta_data) = cons_fixed_meta.get_byte_array_value(loop_index) else {
                continue;
            };

            //
            // SourceForge bug 2209477: Java 注释 —— 这里读的其实是 short，
            // 删除标志只是半个整数。
            //
            // Java: if (ByteArrayHelper.getShort(metaData, 0) != 0) { continue; }
            if meta_data.len() < 2 {
                continue;
            }
            if ByteArrayHelper::get_short(meta_data, 0) != 0 {
                continue;
            }

            // Java: int index = consFixedData.getIndexFromOffset(ByteArrayHelper.getInt(metaData, 4));
            if meta_data.len() < 8 {
                continue;
            }
            let index = cons_fixed_data.get_index_from_offset(ByteArrayHelper::get_int(meta_data, 4));
            if index == -1 {
                continue;
            }

            //
            // 数据是否足够？
            //
            // Java: byte[] data = consFixedData.getByteArrayValue(index);
            //       if (data.length < 14) { continue; }
            let Some(data) = cons_fixed_data.get_byte_array_value(index) else {
                continue;
            };
            if data.len() < MINIMUM_DATA_LENGTH {
                continue;
            }

            // Java: int taskID1 = ByteArrayHelper.getInt(data, 4);
            //       int taskID2 = ByteArrayHelper.getInt(data, 8);
            let task_id1 = ByteArrayHelper::get_int(data, 4);
            let task_id2 = ByteArrayHelper::get_int(data, 8);

            //
            // 与项目摘要任务之间的关系无效
            //
            if task_id1 == 0 || task_id2 == 0 {
                continue;
            }

            //
            // 循环关系无效
            //
            if task_id1 == task_id2 {
                continue;
            }

            // Java: Task task1 = file.getTaskByUniqueID(Integer.valueOf(taskID1));
            //       Task task2 = file.getTaskByUniqueID(Integer.valueOf(taskID2));
            let task1 = file.borrow().get_task_by_unique_id(Some(task_id1));
            let task2 = file.borrow().get_task_by_unique_id(Some(task_id2));

            if let (Some(task1), Some(task2)) = (task1, task2) {
                // Java:
                // Relation relation = task2.addPredecessor(new Relation.Builder()
                //    .predecessorTask(task1)
                //    .type(RelationType.getInstance(ByteArrayHelper.getShort(data, 12)))
                //    .lag(MPPUtility.getAdjustedDuration(properties, ByteArrayHelper.getInt(data, durationOffset),
                //         MPPUtility.getDurationTimeUnits(ByteArrayHelper.getShort(data, durationUnitsOffset))))
                //    .uniqueID(Integer.valueOf(ByteArrayHelper.getInt(data, 0))));
                let duration = ByteArrayHelper::get_int(data, duration_offset);
                let time_units =
                    MPPUtility::get_duration_time_units(ByteArrayHelper::get_short(data, duration_units_offset));

                // 属性对象由 `ProjectFile` 持有，这里临时借用一次（Java 持有稳定引用）
                let lag = {
                    let file_ref = file.borrow();
                    MPPUtility::get_adjusted_duration(
                        file_ref.get_project_properties(),
                        duration,
                        time_units,
                    )
                };

                let builder = RelationBuilder::new()
                    .predecessor_task(&task1)
                    .r#type(RelationType::get_instance(ByteArrayHelper::get_short(data, 12)))
                    .lag(lag)
                    .unique_id(Some(ByteArrayHelper::get_int(data, 0)));

                if let Some(relation) = Task::add_predecessor(&task2, builder) {
                    // Java: eventManager.fireRelationReadEvent(relation);
                    let context = file.borrow().get_project_context();
                    context
                        .borrow_mut()
                        .get_event_manager_mut()
                        .fire_relation_read_event(&relation);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::listener::project_listener::ProjectListener;
    use crate::mpp::ole::PoifsFileSystem;
    use crate::mpp::props::Props;
    use crate::project_calendar::ProjectCalendar;
    use crate::relation::Relation;
    use crate::relation_type::RelationType;
    use crate::resource::Resource;
    use crate::resource_assignment::ResourceAssignment;
    use crate::time_unit::TimeUnit;
    use std::io::{Cursor, Write};

    use cfb::CompoundFile;

    /// 关系读取事件日志（与监听器共享同一 `Rc`）。
    type RelationLog = Rc<RefCell<Vec<Rc<RefCell<Relation>>>>>;

    /// 记录关系读取事件的监听器（与 `crate::event_manager` 测试中的 `Recorder` 同构）。
    struct RelationRecorder {
        log: RelationLog,
    }

    impl RelationRecorder {
        /// 返回监听器与其事件日志。
        fn shared() -> (Self, RelationLog) {
            let log: RelationLog = Rc::new(RefCell::new(Vec::new()));
            (Self { log: log.clone() }, log)
        }
    }

    impl ProjectListener for RelationRecorder {
        fn task_read(&mut self, _task: &Rc<RefCell<Task>>) {}

        fn task_written(&mut self, _task: &Rc<RefCell<Task>>) {}

        fn resource_read(&mut self, _resource: &Rc<RefCell<Resource>>) {}

        fn resource_written(&mut self, _resource: &Rc<RefCell<Resource>>) {}

        fn calendar_read(&mut self, _calendar: &Rc<RefCell<ProjectCalendar>>) {}

        fn calendar_written(&mut self, _calendar: &Rc<RefCell<ProjectCalendar>>) {}

        fn assignment_read(&mut self, _assignment: &Rc<RefCell<ResourceAssignment>>) {}

        fn assignment_written(&mut self, _assignment: &Rc<RefCell<ResourceAssignment>>) {}

        fn relation_read(&mut self, relation: &Rc<RefCell<Relation>>) {
            self.log.borrow_mut().push(Rc::clone(relation));
        }

        fn relation_written(&mut self, _relation: &Rc<RefCell<Relation>>) {}
    }

    /// `TBkndCons` 数据：与 Java 的读取偏移一致。
    ///
    /// - FixedMeta：16 字节块头 + 每条 10 字节记录（第 0..2 字节为删除标志、
    ///   第 4..8 字节为该条数据在 FixedData 中的偏移）
    /// - FixedData：每条 20 字节（偏移 0 处为关系 unique ID、4 处为前驱任务 unique ID、
    ///   8 处为后继任务 unique ID、12 处为关系类型、14/16 处为 lag、18 处为 lag 单位）
    fn build_cons_streams() -> (Vec<u8>, Vec<u8>) {
        let mut meta = Vec::new();
        meta.extend_from_slice(&0xFADFADBAu32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&2i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());

        // 第 1 条：已删除（保留标志非 0）
        let mut deleted = vec![0u8; 10];
        deleted[0..2].copy_from_slice(&1u16.to_le_bytes());
        deleted[4..8].copy_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&deleted);

        // 第 2 条：指向偏移 0
        let mut item = vec![0u8; 10];
        item[4..8].copy_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&item);

        let mut data = vec![0u8; 20];
        data[0..4].copy_from_slice(&77i32.to_le_bytes()); // 关系 unique ID
        data[4..8].copy_from_slice(&1i32.to_le_bytes()); // 前驱任务 unique ID
        data[8..12].copy_from_slice(&2i32.to_le_bytes()); // 后继任务 unique ID
        data[12..14].copy_from_slice(&3u16.to_le_bytes()); // StartStart
        // 非 2013 布局：单位在偏移 14（2 字节），lag 在偏移 16（4 字节）
        data[14..16].copy_from_slice(&7u16.to_le_bytes()); // DAYS
        data[16..20].copy_from_slice(&4800i32.to_le_bytes()); // 1 天（480 分钟/天）

        (meta, data)
    }

    /// 构造一个含 `TBkndCons` 存储的复合文档，并返回其项目存储。
    fn project_directory(meta: &[u8], data: &[u8]) -> (PoifsFileSystem, DirectoryEntry) {
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        file.create_storage("/   114").unwrap();
        file.create_storage("/   114/TBkndCons").unwrap();
        file.create_stream("/   114/TBkndCons/FixedMeta")
            .unwrap()
            .write_all(meta)
            .unwrap();
        file.create_stream("/   114/TBkndCons/FixedData")
            .unwrap()
            .write_all(data)
            .unwrap();
        let bytes = file.into_inner().into_inner();

        let fs = PoifsFileSystem::new(Cursor::new(bytes)).unwrap();
        let directory = fs.get_root().get_directory("   114").expect("项目存储");
        (fs, directory)
    }

    /// 建两个任务（unique ID 1 / 2）并把它们放进项目文件。
    fn file_with_two_tasks() -> Rc<RefCell<ProjectFile>> {
        let file = ProjectFile::new();

        for unique_id in [1, 2] {
            let task = file.borrow_mut().get_tasks_mut().add();
            task.borrow_mut().set_unique_id(Some(unique_id));
        }

        file
    }

    /// 读取关系：类型、lag、unique ID 与任务引用都正确，并触发了事件。
    #[test]
    fn reads_relations_and_fires_events() {
        let (meta, data) = build_cons_streams();
        let (_fs, directory) = project_directory(&meta, &data);
        let file = file_with_two_tasks();

        let (recorder, log) = RelationRecorder::shared();
        file.borrow()
            .get_project_context()
            .borrow_mut()
            .get_event_manager_mut()
            .add_project_listener(Box::new(recorder));

        let factory = DocumentInputStreamFactory::new(&Props::new());
        ConstraintFactory
            .process(&directory, &file, &factory)
            .expect("关系读取成功");

        let task1 = file.borrow().get_task_by_unique_id(Some(1)).expect("任务 1");
        let task2 = file.borrow().get_task_by_unique_id(Some(2)).expect("任务 2");

        let predecessors = task2.borrow().get_predecessors();
        assert_eq!(predecessors.len(), 1);
        let relation = predecessors[0].borrow();
        assert_eq!(relation.get_type(), RelationType::StartStart);
        assert_eq!(relation.get_unique_id(), Some(77));
        assert_eq!(
            relation
                .get_lag()
                .map(|lag| (lag.get_duration(), lag.get_units())),
            Some((1.0, TimeUnit::Days))
        );
        assert!(Rc::ptr_eq(
            &relation.get_predecessor_task().expect("前驱任务"),
            &task1
        ));
        assert!(Rc::ptr_eq(
            &relation.get_successor_task().expect("后继任务"),
            &task2
        ));
        drop(relation);

        // Java: eventManager.fireRelationReadEvent(relation);
        assert_eq!(log.borrow().len(), 1);
    }

    /// 缺少 `TBkndCons` 存储时记录「忽略错误」并正常返回（与 Java 一致）。
    #[test]
    fn missing_directory_is_recorded_as_ignored_error() {
        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        compound.create_storage("/   114").unwrap();
        let bytes = compound.into_inner().into_inner();
        let fs = PoifsFileSystem::new(Cursor::new(bytes)).unwrap();
        let directory = fs.get_root().get_directory("   114").expect("项目存储");

        let file = ProjectFile::new();
        let factory = DocumentInputStreamFactory::new(&Props::new());
        ConstraintFactory
            .process(&directory, &file, &factory)
            .expect("缺少存储不算错误");

        assert_eq!(file.borrow().get_ignored_errors().len(), 1);
    }

    /// 缺少 `FixedMeta` 流时返回错误（Java 抛 FileNotFoundException）。
    #[test]
    fn missing_fixed_meta_returns_error() {
        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        compound.create_storage("/   114").unwrap();
        compound.create_storage("/   114/TBkndCons").unwrap();
        let bytes = compound.into_inner().into_inner();
        let fs = PoifsFileSystem::new(Cursor::new(bytes)).unwrap();
        let directory = fs.get_root().get_directory("   114").expect("项目存储");

        let file = ProjectFile::new();
        let factory = DocumentInputStreamFactory::new(&Props::new());
        assert!(
            ConstraintFactory
                .process(&directory, &file, &factory)
                .is_err()
        );
    }

    /// 坏 magic 的 FixedMeta 返回错误（Java 抛 IOException），不 panic。
    #[test]
    fn bad_fixed_meta_magic_returns_error() {
        let (mut meta, data) = build_cons_streams();
        meta[0] = 0x00;
        let (_fs, directory) = project_directory(&meta, &data);
        let file = ProjectFile::new();

        let factory = DocumentInputStreamFactory::new(&Props::new());
        assert!(
            ConstraintFactory
                .process(&directory, &file, &factory)
                .is_err()
        );
    }
}
