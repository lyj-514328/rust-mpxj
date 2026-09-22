//! Corresponds to Java: `org/mpxj/mpp/ConstraintFactory.java`
//!
//! Read the "relations" between tasks from the `TBkndCons` storage of MPP9 / MPP12 / MPP14 files
//! (Java calls such a record a constraint, but it actually represents a predecessor/successor relation),
//! write them into the successor task, and fire the relation read event.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `class ConstraintFactory` (package-visible) | `pub struct ConstraintFactory` | Rust has no package visibility (this crate's convention is public types) |
//! | `process(DirectoryEntry projectDir, ProjectFile file, DocumentInputStreamFactory inputStreamFactory) throws IOException` | [`ConstraintFactory::process`] returns `MpxjResult<()>` | Java uses `IOException` for "stream read failure"; the Rust side uniformly uses [`MpxjError`] (`io::Error` converts automatically via `?`) |
//! | `new FixedMeta(new DocumentInputStream((DocumentEntry) consDir.getEntry("FixedMeta")), 10)` | `FixedMeta::with_item_size(&document.get_data(), 10)` | Per project convention `InputStream` is always replaced by `&[u8]`; when the entry does not exist Java throws `FileNotFoundException` and Rust returns `Err` |
//! | `new FixedData(consFixedMeta, 20, inputStreamFactory.getInstance(consDir, "FixedData"))` | `FixedData::with_item_size(&meta, 20, &stream.to_bytes())` | same as above |
//! | `eventManager.fireRelationReadEvent(relation)` | fired after obtaining a mutable reference to `EventManager` via `ProjectFile::get_project_context()` | `ProjectFile::get_event_manager()` only returns an immutable `Ref`, so it cannot call event methods taking `&mut self` |
//! | `ByteArrayHelper.getShort/getInt` out of bounds | check the length first, then read, and `continue` when insufficient | Java throws `ArrayIndexOutOfBoundsException` |
//! | `file.getTaskByUniqueID(...)` returns `null` | `Option<Rc<RefCell<Task>>>` | the "`null` → `Option<T>`" convention |
//!
//! Skipped Java code and the reason:
//!
//! | Java | Reason |
//! |---|---|
//! | the commented-out `consFixed2Meta` / `consFixed2Data` code (already commented out in the Java source) | it is a comment in the Java source and does not run |
//! | the automatic unique ID allocation of `task2.addPredecessor(...)` | see the file header of `crate::relation::Builder` (automatic allocation relies on `ProjectConfig`'s sequence generator; on the Rust side the caller provides `uniqueID`; here Java already passes the unique ID explicitly) |
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class uses none of these mechanisms.

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

/// The `TBkndCons` storage name.
///
/// Corresponds to Java: `projectDir.getEntry("TBkndCons")`
const CONS_DIRECTORY: &str = "TBkndCons";

/// Minimum length of the relation data block.
///
/// Corresponds to Java: `if (data.length < 14) { continue; }`
const MINIMUM_DATA_LENGTH: usize = 14;

/// Common relation data reading logic.
///
/// Corresponds to Java: `class ConstraintFactory`
pub struct ConstraintFactory;

impl ConstraintFactory {
/// Main entry point for processing relation data.
    ///
    /// Corresponds to Java: `public void process(DirectoryEntry projectDir, ProjectFile file, DocumentInputStreamFactory inputStreamFactory) throws IOException`
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
// Java obtains the properties once before the loop; on the Rust side this borrows once only to compute the two flags below.
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
// SourceForge bug 2209477: Java comment — what is read here is actually a short,
// and the deletion flag is only half an integer.
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
// Is there enough data?
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
// a relation with the project summary task is invalid
            //
            if task_id1 == 0 || task_id2 == 0 {
                continue;
            }

            //
// a circular relation is invalid
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

// the property object is held by `ProjectFile`; this borrows it temporarily once (Java holds a stable reference)
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

/// Relation read event log (shares the same `Rc` as the listener).
    type RelationLog = Rc<RefCell<Vec<Rc<RefCell<Relation>>>>>;

/// A listener that records relation read events (isomorphic to `Recorder` in the `crate::event_manager` test).
    struct RelationRecorder {
        log: RelationLog,
    }

    impl RelationRecorder {
/// Return the listener and its event log.
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

/// `TBkndCons` data: consistent with Java's read offsets.
    ///
/// - FixedMeta: 16-byte block header + a 10-byte record each (bytes 0..2 are the deletion flag,
/// bytes 4..8 are the offset of that record in FixedData)
/// - FixedData: 20 bytes each (offset 0 is the relation unique ID, 4 is the predecessor task unique ID,
/// 8 is the successor task unique ID, 12 is the relation type, 14/16 is the lag, 18 is the lag unit)
    fn build_cons_streams() -> (Vec<u8>, Vec<u8>) {
        let mut meta = Vec::new();
        meta.extend_from_slice(&0xFADFADBAu32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&2i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());

// record 1: deleted (the reserved flag is non-zero)
        let mut deleted = vec![0u8; 10];
        deleted[0..2].copy_from_slice(&1u16.to_le_bytes());
        deleted[4..8].copy_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&deleted);

// record 2: points to offset 0
        let mut item = vec![0u8; 10];
        item[4..8].copy_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&item);

        let mut data = vec![0u8; 20];
        data[0..4].copy_from_slice(&77i32.to_le_bytes()); // relation unique ID
        data[4..8].copy_from_slice(&1i32.to_le_bytes()); // predecessor task unique ID
        data[8..12].copy_from_slice(&2i32.to_le_bytes()); // successor task unique ID
        data[12..14].copy_from_slice(&3u16.to_le_bytes()); // StartStart
// non-2013 layout: the unit is at offset 14 (2 bytes) and the lag is at offset 16 (4 bytes)
        data[14..16].copy_from_slice(&7u16.to_le_bytes()); // DAYS
        data[16..20].copy_from_slice(&4800i32.to_le_bytes()); // 1 day (480 minutes/day)

        (meta, data)
    }

/// Build a compound document containing `TBkndCons` storage and return its project storage.
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
        let directory = fs.get_root().get_directory("   114").expect("project store");
        (fs, directory)
    }

/// Create two tasks (unique ID 1 / 2) and put them into the project file.
    fn file_with_two_tasks() -> Rc<RefCell<ProjectFile>> {
        let file = ProjectFile::new();

        for unique_id in [1, 2] {
            let task = file.borrow_mut().get_tasks_mut().add();
            task.borrow_mut().set_unique_id(Some(unique_id));
        }

        file
    }

/// Read the relation: the type, lag, unique ID, and task references are all correct, and the event is fired.
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
            .expect("relation read successfully");

        let task1 = file.borrow().get_task_by_unique_id(Some(1)).expect("task 1");
        let task2 = file.borrow().get_task_by_unique_id(Some(2)).expect("task 2");

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
            &relation.get_predecessor_task().expect("predecessor task"),
            &task1
        ));
        assert!(Rc::ptr_eq(
            &relation.get_successor_task().expect("successor task"),
            &task2
        ));
        drop(relation);

        // Java: eventManager.fireRelationReadEvent(relation);
        assert_eq!(log.borrow().len(), 1);
    }

/// When the `TBkndCons` storage is missing, logs "ignore error" and returns normally (consistent with Java).
    #[test]
    fn missing_directory_is_recorded_as_ignored_error() {
        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        compound.create_storage("/   114").unwrap();
        let bytes = compound.into_inner().into_inner();
        let fs = PoifsFileSystem::new(Cursor::new(bytes)).unwrap();
        let directory = fs.get_root().get_directory("   114").expect("project store");

        let file = ProjectFile::new();
        let factory = DocumentInputStreamFactory::new(&Props::new());
        ConstraintFactory
            .process(&directory, &file, &factory)
            .expect("missing store is not an error");

        assert_eq!(file.borrow().get_ignored_errors().len(), 1);
    }

/// Returns an error when the `FixedMeta` stream is missing (Java throws FileNotFoundException).
    #[test]
    fn missing_fixed_meta_returns_error() {
        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        compound.create_storage("/   114").unwrap();
        compound.create_storage("/   114/TBkndCons").unwrap();
        let bytes = compound.into_inner().into_inner();
        let fs = PoifsFileSystem::new(Cursor::new(bytes)).unwrap();
        let directory = fs.get_root().get_directory("   114").expect("project store");

        let file = ProjectFile::new();
        let factory = DocumentInputStreamFactory::new(&Props::new());
        assert!(
            ConstraintFactory
                .process(&directory, &file, &factory)
                .is_err()
        );
    }

/// A FixedMeta with bad magic returns an error (Java throws IOException) without panicking.
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
