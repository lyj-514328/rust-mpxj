//! Corresponds to Java: `org/mpxj/mpp/MPPReader.java`
//!
//! Creates a [`ProjectFile`] by reading an MPP file. This class is the "main entry point" of the mpp package: from the OLE compound document's
//! `\1CompObj` stream it extracts the file format string (e.g. `MSProject.MPP14`) and picks a variant reader
//! ([`MPP8Reader`] / [`MPP9Reader`] / [`MPP12Reader`] / [`MPP14Reader`]; for the contract see
//! [`MPPVariantReader`]), then performs uniform post-processing on the [`ProjectFile`] produced by the variant.
//!
//! # Restoring the inheritance chain (composition + forwarding)
//!
//! Java: `public final class MPPReader extends AbstractProjectStreamReader`. Rust has no inheritance,
//! so it follows the template given in the file header of [`crate::reader::abstract_project_stream_reader`]:
//! [`MPPReader`] stores the listener list and simultaneously `impl`s [`ProjectReader`], [`AbstractProjectReader`],
//! and [`AbstractProjectStreamReader`], forwarding reusable interface methods to the abstract layer's `_default` methods.
//!
//! | Java | Rust |
//! |---|---|
//! | `read(InputStream)` | The entry method is [`MPPReader::read_stream`] (the `_stream` suffix distinguishes it from Java's other overload `read(POIFSFileSystem)`); the interface method [`ProjectReader::read_reader`] forwards to it |
//! | `read(File)` | The entry point lives inside the interface method [`ProjectReader::read_file`] (Java's MPPReader overrides the base class version so it can build a `POIFSFileSystem` directly from a `File`; concrete readers in this crate write the methods that are "still abstract at this layer" directly in the trait implementation, see the wiring template in [`crate::reader::abstract_project_stream_reader`]) |
//! | `readAll(File)` | [`MPPReader::read_all_file`] → `AbstractProjectStreamReader::read_all_file_default` |
//! | `readAll(InputStream)` | [`MPPReader::read_all_reader`] → `AbstractProjectStreamReader::read_all_reader_default` |
//! | `addProjectListener(ProjectListener)` | [`MPPReader::add_project_listener`] → `AbstractProjectReader::add_project_listener_default` |
//! | `read(POIFSFileSystem)` | [`MPPReader::read`] |
//! | `static getFileFormat(POIFSFileSystem)` | [`MPPReader::get_file_format`] |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `new POIFSFileSystem(is)` in `read(InputStream)` | First read the stream into memory via [`read_stream_into_cursor`], then `PoifsFileSystem::new` | [`PoifsFileSystem::new`] requires the input source to be `'static`, but `&mut dyn Read` is not; MPP files are usually small, so the cost is acceptable (see "input sources unified as `Box<dyn ReadSeek>`" in the `ole.rs` file header) |
//! | `new POIFSFileSystem(file)` in `read(File)` to reduce memory usage | `PoifsFileSystem::new(File::open(path)?)` | This optimization still uses the "per-file" path (`std::fs::File` satisfies `Read + Seek`) and does not degrade to "read into memory first"; `finally { AutoCloseableHelper.closeQuietly(fs); }` is replaced by `File`'s `Drop` |
//! | `catch (IOException \| InstantiationException \| IllegalAccessException)` in `read(POIFSFileSystem)` | No `catch` | `IOException` is converted by `?` via `From<std::io::Error>` into an [`MpxjError`] prefixed with `Error reading file` (consistent with `new MPXJException(READ_ERROR, ex)`); the latter two exceptions come from reflective `newInstance()`, and reflection has been skipped (see [`file_class_map`]) |
//! | `FILE_CLASS_MAP` (`Map<String, Class<..>>`) + `readerClass.newInstance()` | [`file_class_map`]: construct directly by format name | Reflection is skipped in this project; external behavior is identical (unregistered format → `null` / `None`), and the two reflection-related exceptions are eliminated |
//! | `static String getFileFormat(..)` (may return `""` or `null`) | `Option<String>` | Java's `""` (no `\1CompObj` stream in the root storage) and `null` (CompObj did not record a fileFormat) are unified as `None` on the Rust side -- in the only consumer, `UniversalProjectReader`, both are judged "not MPP" by `fileFormat == null \|\| !fileFormat.startsWith("MSProject")`, so the behavior is equivalent |
//! | `new CompObj(new DocumentInputStream((DocumentEntry) root.getEntry("\1CompObj")))` | [`MPPReader::read_comp_obj`]: if `root.get_document(..)` fails, `Err` | POI's `getEntry` throws when the entry is not found; on the Rust side [`DirectoryEntry::get_document`] returns `None`, which is converted here to `Err(INVALID_FILE)` (on the Java side it is `NullPointerException`/`ClassCastException`, an uncaught runtime exception) |
//! | Direct mutable access to `ProjectFile` | Always `Rc<RefCell<ProjectFile>>` + `try_borrow` / `try_borrow_mut` | A Rust borrow conflict must degrade to `Err` rather than panic (Java has stable references and no such case); see [`borrow_file`] / [`borrow_file_mut`] / [`borrow_context`] and [`borrow_conflict`] |
//! | `config.setAutoXXX(false)` via `ProjectFile.getProjectConfig()` | via `ProjectContext::get_project_config_mut()` | [`ProjectFile::get_project_config`] returns a read-only `Ref<ProjectConfig>` on the Rust side (the config is held by the context, see its file header), so modification must go through the context |
//! | `task.setSummary(..)` and `relation.getSuccessorTask().removePredecessor(..)` in the same loop | Snapshot the task list first, then process one by one | `Task::remove_predecessor` internally calls `file.borrow_mut()`, which would panic if the outer scope still holds a `ProjectFile` borrow (Java has no such restriction); cloning out a `Vec<Rc<RefCell<Task>>>` and releasing the file borrow is equivalent |
//! | `resourceCalendarMap.computeIfAbsent(calendar.getUniqueID(), k -> emptyList())` | `HashMap<Option<i32>, Vec<..>>` + `entry(..).or_default()` | `HashMap` allows `null` keys, so Java's key type stays the nullable `Integer`; Rust faithfully corresponds with an `Option<i32>` key (a `None` key inserts an empty list → `size() == 0`) |
//! | `projectFile.getCalendars().removeIf(predicate)` | Collect the indexes to remove first, then [`ProjectCalendarContainer::remove`] in reverse order | The container does not provide `remove_if`, and `remove` takes a `&ProjectCalendar`; removing by index in reverse order matches the one-by-one removal semantics of `removeIf` |
//! | `m_readPassword` (`String`, may be `null`) | `Option<String>` | -- |
//!
//! # Interface assumptions for the variant readers (verified: all four variants are in place)
//!
//! - The variant types live in `crate::mpp::mpp{8,9,12,14}_reader`, with type names `MPP8Reader` / `MPP9Reader` /
//!   `MPP12Reader` / `MPP14Reader`;
//! - Each provides a **no-argument** constructor `new()` (corresponding to Java's implicit no-argument constructor
//! `MPP9Reader reader = new MPP9Reader();`), and implements [`MPPVariantReader`];
//! - `MPPVariantReader::process(&self, reader: &MPPReader, file: &Rc<RefCell<ProjectFile>>,
//! root: &DirectoryEntry)`, refer to [`crate::mpp::mpp_variant_reader`] for the signature.
//!
//! Here `reader: &MPPReader` (rather than `&Rc<RefCell<MPPReader>>`) is a deliberate choice: a variant reader only needs
//! to read five flags, whereas "`&Rc<RefCell<MPPReader>>`" cannot be provided by [`ProjectReader`]'s `&mut self` entry point
//! (`RefCell` is not among legal receivers such as `self: Rc<Self>`), which would prevent [`MPPReader::read`] from
//! passing `this` (Java semantics) as-is to `process`. All four variant readers (including `MPP8Reader`) are in place with that signature,
//! and this file's `use crate::mpp::mpp8_reader::MPP8Reader` resolves normally.
//!
//! # Skipped Java code and rationale
//!
//! - **Reflection**: `FILE_CLASS_MAP`'s `Class` values + `readerClass.newInstance()` →
//! [`file_class_map`] constructs directly (see the table above);
//! - **`InstantiationException` / `IllegalAccessException`**: thrown only by reflection, and disappear along with it;
//! - **`Map<TaskField, TaskField>` from `import org.mpxj.TaskField`**: changed to a constant table
//! [`TASK_ESTIMATED_BASELINE_FIELDS`] (`&[(TaskField, TaskField)]`), to avoid introducing for a static lookup table
//! a global mutable container;
//! - This class does not use JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`; nor does it reference the reflection-based dispatch of
//! `UniversalProjectReader` / `ProjectReaderUtility` (not ported this time).

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

/// The name of the `\1CompObj` stream (Java's `"\1CompObj"`, i.e. `\u{1}` + `CompObj`).
const COMP_OBJ: &str = "\u{1}CompObj";

/// Corresponds to Java: `projectProperties.setFileApplication("Microsoft Project Server")`.
const FILE_APPLICATION_PROJECT_SERVER: &str = "Microsoft Project Server";
/// Corresponds to Java: `projectProperties.setFileApplication("Microsoft")`.
const FILE_APPLICATION_MICROSOFT: &str = "Microsoft";
/// Corresponds to Java: `projectProperties.setFileType("MPP")`.
const FILE_TYPE_MPP: &str = "MPP";
/// Corresponds to Java: `projectFilePath.startsWith("<>\\")` (the Project Server project path prefix).
const PROJECT_SERVER_PATH_PREFIX: &str = "<>\\";
/// Corresponds to Java: `name = "Unnamed Resource";`
const UNNAMED_RESOURCE: &str = "Unnamed Resource";
/// Corresponds to Java: rendering when string concatenation encounters `null` (`INVALID_FILE + ": " + format`).
const JAVA_NULL: &str = "null";
/// Corresponds to Java: `TaskField.BASELINE_DURATION` → `TaskField.BASELINE_ESTIMATED_DURATION` etc.
/// All entries of the `TASK_ESTIMATED_BASELINE_FIELDS` static table (11 baseline groups × 3 fields).
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

/// Creates a [`ProjectFile`] by reading an MPP file.
///
/// Corresponds to Java: `public final class MPPReader extends AbstractProjectStreamReader`
pub struct MPPReader {
/// The registered listeners.
    ///
/// Corresponds to Java: the private field of `AbstractProjectReader`
/// `private List<ProjectListener> m_projectListeners` (a Rust trait cannot carry fields, so it lives on the implementor).
    listeners: Vec<SharedProjectListener>,

/// When set to `true`, use MS Project's raw timephased data directly without normalization.
    ///
    /// Corresponds to Java: `private boolean m_useRawTimephasedData`
    use_raw_timephased_data: bool,

/// Whether to read presentation data (setting to `false` saves time and memory).
    ///
    /// Corresponds to Java: `private boolean m_readPresentationData = true`
    read_presentation_data: bool,

/// Whether to read only the project properties.
    ///
    /// Corresponds to Java: `private boolean m_readPropertiesOnly`
    read_properties_only: bool,

/// Whether to respect password protection.
    ///
    /// Corresponds to Java: `private boolean m_respectPasswordProtection = true`
    respect_password_protection: bool,

/// The read password (may be `null` in Java).
    ///
    /// Corresponds to Java: `private String m_readPassword`
    read_password: Option<String>,
}

impl MPPReader {
/// Constructor.
    ///
/// Corresponds to Java: the implicit default constructor (`new MPPReader()`)
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

/// Reads an MPP file from an input stream.
    ///
    /// Corresponds to Java: `@Override public ProjectFile read(InputStream is) throws MPXJException`
    ///
/// Java directly calls `new POIFSFileSystem(is)`; on the Rust side [`PoifsFileSystem::new`] requires the input source
/// to be `'static`, so the stream is first read into memory (see the file header difference table).
    pub fn read_stream(&self, input_stream: &mut dyn Read) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        // Java: return read(new POIFSFileSystem(is));
// (Java's catch (IOException ex) → new MPXJException(READ_ERROR, ex) is handled by ?)
        let cursor: Cursor<Vec<u8>> = read_stream_into_cursor(input_stream)?;
        let fs = PoifsFileSystem::new(cursor)?;
        self.read(&fs)
    }

/// Peeks into an OLE compound document and extracts the file format name.
    ///
    /// Corresponds to Java: `public static String getFileFormat(POIFSFileSystem fs) throws IOException`
    ///
/// Used by `UniversalProjectReader` to determine whether the OLE compound document is MPP or another type.
/// Java returns `""` (no `\1CompObj` stream) or `null` (CompObj did not record a format);
/// the Rust side unifies both as `None` (see the file header difference table).
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

/// Reads an already-opened (or caller-constructed) OLE compound document.
    ///
    /// Corresponds to Java: `public ProjectFile read(POIFSFileSystem fs) throws MPXJException`
    pub fn read(&self, fs: &PoifsFileSystem) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        // Java: try { ... } catch (IOException | InstantiationException | IllegalAccessException ex)
        //       { throw new MPXJException(MPXJException.READ_ERROR, ex); }
// On the Rust side, I/O failures are converted by ? to READ_ERROR; the other two exceptions come from reflective newInstance(), and reflection has been skipped.

        // Java: ProjectFile projectFile = new ProjectFile();
        let project_file = ProjectFile::new();

        // Java: ProjectConfig config = projectFile.getProjectConfig(); config.setAutoXXX(false) ...
        self.set_auto_ids(&project_file, false)?;

        // Java: addListenersToProject(projectFile);
        self.add_listeners_to_project(&project_file);

        //
// Java: open the file system and get the root directory
        // DirectoryEntry root = fs.getRoot();
        //
        let root = fs.get_root();

        //
// Java: extract the CompObj data, validate the file format, dispatch processing
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
// Java: update internal structures -- incidentally generate outline numbers for tasks (MPP files appear to lack this data)
        // config.setAutoOutlineNumber(true);
        // projectFile.updateStructure();
        // config.setAutoOutlineNumber(false);
        //
        self.set_auto_outline_number(&project_file, true)?;
        borrow_file_mut(&project_file)?.update_structure();
        self.set_auto_outline_number(&project_file, false)?;

        //
// Java: post-processing -- set the summary flag and clean up task instances whose "split list is empty"
        // for (Task task : projectFile.getTasks())
        // {
        //    task.setSummary(task.hasChildTasks() || task.getExternalProject());
        //    validationRelations(task);
        //    copyEstimatedBaselineFields(task);
        // }
        //
// The Rust side clones the task list first: `validationRelations` eventually triggers `Task::removePredecessor`,
// which internally needs `file.borrow_mut()` (Java has no such restriction, see the file header difference table).
        let tasks: Vec<Rc<RefCell<Task>>> = {
            let file_ref = borrow_file(&project_file)?;
            file_ref.get_tasks().iter().cloned().collect()
        };

        for task in &tasks {
            // Java: task.setSummary(task.hasChildTasks() || task.getExternalProject());
            let summary = match task.try_borrow() {
                Ok(task_ref) => task_ref.has_child_tasks() || task_ref.get_external_project(),
// Java has no such branch (elements are always readable); the Rust side avoids a borrow-conflict panic
                Err(_) => continue,
            };

            if let Ok(mut task_ref) = task.try_borrow_mut() {
                task_ref.set_summary(summary);
            }

            self.validation_relations(task);
            Self::copy_estimated_baseline_fields(task);
        }

        //
// Java: clean up unused resource calendars
        // Map<Integer, List<Resource>> resourceCalendarMap = projectFile.getResources().stream()
        //    .filter(r -> r.getCalendarUniqueID() != null)
        //    .collect(Collectors.groupingBy(Resource::getCalendarUniqueID));
        // projectFile.getCalendars().removeIf(c -> c.isDerived() && !resourceCalendarMap.containsKey(c.getUniqueID()));
        //
// The key type is `Option<i32>`: `HashMap` allows `null` keys, consistent with Java's `Integer` key
// (a later `computeIfAbsent(null, ..)` inserts an empty list, see below).
        let mut resource_calendar_map: HashMap<Option<i32>, Vec<Rc<RefCell<Resource>>>> = HashMap::new();
        {
            let context = borrow_context(&project_file)?;
            let Ok(context_ref) = context.try_borrow() else {
                return Err(borrow_conflict());
            };

            for resource in context_ref.get_resources().iter() {
                let Ok(resource_ref) = resource.try_borrow() else {
// Java has no such branch; the Rust side avoids a borrow-conflict panic
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

// Java's removeIf predicate
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

// Remove by index in reverse order (equivalent to removeIf removing one by one)
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
// Java: resource post-processing
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

// Java: a resource calendar without a name inherits the resource name
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
// Java: add analysis information
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

/// Validates all of a task's relations and removes those incorrectly read from the MPP file that point to a parent task.
    ///
    /// Corresponds to Java: `private void validationRelations(Task task)`
    fn validation_relations(&self, task: &Rc<RefCell<Task>>) {
        // Java: List<Relation> predecessors = task.getPredecessors();
        //       if (predecessors.isEmpty()) { return; }
        let predecessors = match task.try_borrow() {
            Ok(task_ref) => task_ref.get_predecessors(),
// Java has no such branch; the Rust side avoids a borrow-conflict panic
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
// Java has no such branch; the Rust side avoids a borrow-conflict panic
                Err(_) => continue,
            };

            let (Some(source_task), Some(target_task)) = (source_task, target_task) else {
// When Task is null, Java throws a NullPointerException at getOutlineNumber();
// the Rust side treats it as "cannot be determined" (does not add it to the invalid list)
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
// Java has no such branch; the Rust side avoids a borrow-conflict panic
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

/// When a baseline field is empty but its "estimated version" has a value, fall back to the estimated field.
    ///
    /// Corresponds to Java: `private void copyEstimatedBaselineFields(Task task)`
    fn copy_estimated_baseline_fields(task: &Rc<RefCell<Task>>) {
        for (key, estimated_key) in TASK_ESTIMATED_BASELINE_FIELDS {
            // Java: Object value = task.getCachedValue(entry.getKey());
            //       if (value == null) { value = task.getCachedValue(entry.getValue());
            //                            if (value != null) { task.set(entry.getKey(), value); } }
            let value = {
                let Ok(task_ref) = task.try_borrow() else {
// Java has no such branch; the Rust side avoids a borrow-conflict panic
                    continue;
                };

// The borrow must be released before `set`, so the value is cloned out directly here
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

/// Reads and parses the `\1CompObj` stream.
    ///
/// Java does this inline in `read(POIFSFileSystem)`; the Rust side extracts it into a private helper function,
/// because both "obtaining the stream" and "parsing" must handle failure paths (runtime exceptions in Java).
    fn read_comp_obj(root: &DirectoryEntry) -> MpxjResult<CompObj> {
        // Java: new CompObj(new DocumentInputStream((DocumentEntry) root.getEntry("\1CompObj")))
        let Some(document) = root.get_document(COMP_OBJ) else {
// On the Java side it is NullPointerException / ClassCastException (uncaught runtime exceptions)
            return Err(MpxjError::new(MpxjError::INVALID_FILE));
        };

        CompObj::new(&document.get_data())
    }

/// Turns the automatic numbering flags of `ProjectConfig` off/on (`config.setAutoXXX(flag)`).
    ///
/// Java assigns to `ProjectConfig` one by one directly inside `read(POIFSFileSystem)`;
/// the Rust side extracts helper functions to reduce repetitive borrow boilerplate.
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

/// Sets `config.setAutoOutlineNumber(flag)`.
    ///
/// Java assigns directly inside `read(POIFSFileSystem)`; the Rust side extracts a helper function.
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

/// If this flag is `true`, use MS Project's raw timephased data directly without normalization.
    ///
    /// Corresponds to Java: `public boolean getUseRawTimephasedData()`
    pub fn get_use_raw_timephased_data(&self) -> bool {
        self.use_raw_timephased_data
    }

/// Sets the "use raw timephased data directly" flag.
    ///
    /// Corresponds to Java: `public void setUseRawTimephasedData(boolean useRawTimephasedData)`
    pub fn set_use_raw_timephased_data(&mut self, use_raw_timephased_data: bool) {
        self.use_raw_timephased_data = use_raw_timephased_data;
    }

/// Whether to read presentation data in the MPP file (not reading it saves time and memory).
    ///
    /// Corresponds to Java: `public boolean getReadPresentationData()`
    pub fn get_read_presentation_data(&self) -> bool {
        self.read_presentation_data
    }

/// Sets the "read presentation data" flag.
    ///
    /// Corresponds to Java: `public void setReadPresentationData(boolean readPresentationData)`
    pub fn set_read_presentation_data(&mut self, read_presentation_data: bool) {
        self.read_presentation_data = read_presentation_data;
    }

/// Whether to read only the project properties (for fast access to document properties).
    ///
    /// Corresponds to Java: `public boolean getReadPropertiesOnly()`
    pub fn get_read_properties_only(&self) -> bool {
        self.read_properties_only
    }

/// Sets the "read only project properties" flag.
    ///
    /// Corresponds to Java: `public void setReadPropertiesOnly(boolean readPropertiesOnly)`
    pub fn set_read_properties_only(&mut self, read_properties_only: bool) {
        self.read_properties_only = read_properties_only;
    }

/// Sets the read password (needed to read a password-protected Project file).
    ///
    /// Corresponds to Java: `public void setReadPassword(String password)`
    ///
/// Java's `null` → `None`.
    pub fn set_read_password(&mut self, password: Option<String>) {
        self.read_password = password;
    }

/// Gets the read password (internal use: a password-protected Project file needs it to be read).
    ///
    /// Corresponds to Java: `public String getReadPassword()`
    pub fn get_read_password(&self) -> Option<&str> {
        self.read_password.as_deref()
    }

/// Setting it to `false` on formats that support the option ignores password protection.
    ///
    /// Corresponds to Java: `public void setRespectPasswordProtection(boolean respectPasswordProtection)`
    pub fn set_respect_password_protection(&mut self, respect_password_protection: bool) {
        self.respect_password_protection = respect_password_protection;
    }

/// Whether to respect password protection.
    ///
    /// Corresponds to Java: `public boolean getRespectPasswordProtection()`
    pub fn get_respect_password_protection(&self) -> bool {
        self.respect_password_protection
    }
}

impl Default for MPPReader {
/// Corresponds to Java: the implicit default constructor.
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectReader for MPPReader {
/// Registers a listener.
    ///
/// Corresponds to Java: `@Override public void addProjectListener(ProjectListener listener)` (inherited from `AbstractProjectReader`)
    fn add_project_listener(&mut self, listener: Box<dyn crate::listener::project_listener::ProjectListener>) {
        self.add_project_listener_default(listener);
    }

/// Reads a schedule from a path.
    ///
    /// Corresponds to Java: `@Override public ProjectFile read(File file)`
    ///
/// Java deliberately overrides the base class version (instead of going through `AbstractProjectStreamReader`'s "open file, get stream"),
/// because building a `POIFSFileSystem` directly from a `File` uses less memory and can open larger MPP files.
/// The Rust side likewise passes [`std::fs::File`] directly to [`PoifsFileSystem::new`].
    fn read_file(&mut self, path: &Path) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        // Java: fs = new POIFSFileSystem(file); return read(fs);
// (Java's catch (IOException ex) → new MPXJException(READ_ERROR, ex) is handled by ?)
        let handle = File::open(path)?;
        let fs = PoifsFileSystem::new(handle)?;

        // Java: finally { AutoCloseableHelper.closeQuietly(fs); }
// -- on the Rust side `fs` is dropped with the scope (`PoifsFileSystem` holds the file handle).
        self.read(&fs)
    }

/// Reads all schedules from a path (an MPP contains a single schedule, so a single-element list is returned).
    ///
    /// Corresponds to Java: `AbstractProjectStreamReader.readAll(File file)`
    fn read_all_file(&mut self, path: &Path) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
        self.read_all_file_default(path)
    }

/// Reads a schedule from an input stream.
    ///
    /// Corresponds to Java: `@Override public ProjectFile read(InputStream inputStream)`
    fn read_reader(&mut self, reader: &mut dyn Read) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        self.read_stream(reader)
    }

/// Reads all schedules from an input stream (an MPP contains a single schedule, so a single-element list is returned).
    ///
    /// Corresponds to Java: `AbstractProjectStreamReader.readAll(InputStream inputStream)`
    fn read_all_reader(
        &mut self,
        reader: &mut dyn Read,
    ) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
        self.read_all_reader_default(reader)
    }
}

impl AbstractProjectReader for MPPReader {
/// Gets the registered listener list.
    ///
/// Rust-specific (Java's `m_projectListeners` is a private field).
    fn project_listeners(&self) -> &[SharedProjectListener] {
        &self.listeners
    }

/// Gets the registered listener list (mutable).
    ///
/// Rust-specific (same as above).
    fn project_listeners_mut(&mut self) -> &mut Vec<SharedProjectListener> {
        &mut self.listeners
    }
}

impl AbstractProjectStreamReader for MPPReader {}

/// Corresponds to Java: `private static final Map<String, Class<? extends MPPVariantReader>> FILE_CLASS_MAP`
/// and its static initializer block, as well as in `read(POIFSFileSystem)`
/// `readerClass = FILE_CLASS_MAP.get(format)` / `readerClass.newInstance()`.
///
/// # Restoring reflection
///
/// Java stores `Class` objects and creates instances reflectively; Rust has no reflection, so it is changed to "match by format name and construct directly".
/// External behavior is identical to Java: a registered format name → the corresponding variant reader; an unregistered format (including `null`) → `None`
/// (corresponding to Java's `null`). The side effect is that the two failure branches of `newInstance()` on the Java side --
/// `InstantiationException` / `IllegalAccessException` -- disappear as well.
///
/// # Interface assumptions for related modules
///
/// See "Interface assumptions for the variant readers" in this file header: each of the four variant types has a no-argument constructor `new()` and implements
/// [`MPPVariantReader`].
fn file_class_map(format: Option<&str>) -> Option<Box<dyn MPPVariantReader>> {
// Java: FILE_CLASS_MAP.get(format) -- returns null when the key is null
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

// Java: unregistered → Map.get returns null → read throws INVALID_FILE + ": " + format
        _ => None,
    }
}

/// Borrows the project file (Rust-specific: Java holds an object reference directly and has no concept of borrowing).
///
/// Corresponds to Java: none (the `ProjectFile projectFile` local variable itself)
fn borrow_file(file: &Rc<RefCell<ProjectFile>>) -> MpxjResult<Ref<'_, ProjectFile>> {
    file.try_borrow().map_err(|_| borrow_conflict())
}

/// Mutably borrows the project file (Rust-specific).
///
/// Corresponds to Java: none (same as above)
fn borrow_file_mut(file: &Rc<RefCell<ProjectFile>>) -> MpxjResult<RefMut<'_, ProjectFile>> {
    file.try_borrow_mut().map_err(|_| borrow_conflict())
}

/// Gets the project context of the project file (Rust-specific).
///
/// Corresponds to Java: `projectFile.getProjectContext()` (Java has no possibility of borrow failure)
fn borrow_context(file: &Rc<RefCell<ProjectFile>>) -> MpxjResult<Rc<RefCell<ProjectContext>>> {
    Ok(borrow_file(file)?.get_project_context())
}

/// The error for a `RefCell` borrow conflict (Rust-specific: Java has stable references and no such case).
///
/// A borrow conflict means "the caller already holds a mutable/immutable reference", so reading cannot continue and it is returned as a read failure.
fn borrow_conflict() -> MpxjError {
    MpxjError::new(MpxjError::READ_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Write;

    use cfb::CompoundFile;

/// Writes the specified stream into the compound document (a name containing control characters is fine).
    fn compound_file_with_streams(streams: &[(&str, &[u8])]) -> Vec<u8> {
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).expect("create compound document");

        for (name, data) in streams {
            file.create_stream(name)
                .expect("create stream")
                .write_all(data)
                .expect("write stream");
        }

        file.into_inner().into_inner()
    }

/// Assembles the content of the `\1CompObj` stream according to the CompObj layout.
    ///
/// Kept consistent with the test constants in `comp_obj.rs`: 28 bytes of padding + length-prefixed strings.
    fn comp_obj_payload(application_name: &str, file_format: Option<&str>) -> Vec<u8> {
        let mut data = vec![0u8; 28];
        append_string(&mut data, application_name);

        if let Some(file_format) = file_format {
            append_string(&mut data, file_format);
// applicationID (a length of 0 means none)
            data.extend_from_slice(&0i32.to_le_bytes());
        }

        data
    }

/// Appends "length (including the trailing 0 byte) + content + 0 byte".
    fn append_string(data: &mut Vec<u8>, value: &str) {
        data.extend_from_slice(&((value.len() + 1) as i32).to_le_bytes());
        data.extend_from_slice(value.as_bytes());
        data.push(0);
    }

/// Opens an in-memory compound document.
    fn open(bytes: Vec<u8>) -> MpxjResult<PoifsFileSystem> {
        Ok(PoifsFileSystem::new(Cursor::new(bytes))?)
    }

    #[test]
    fn get_file_format_reads_the_comp_obj_stream() {
        let bytes = compound_file_with_streams(&[(
            "\u{1}CompObj",
            &comp_obj_payload("Microsoft Project 14.0", Some("MSProject.MPP14")),
        )]);
        let fs = open(bytes).expect("open compound document");

// Java: root.getEntryNames().contains("\1CompObj") holds → parse CompObj
        assert_eq!(
            MPPReader::get_file_format(&fs),
            Some("MSProject.MPP14".to_owned())
        );
    }

    #[test]
    fn get_file_format_is_none_without_the_comp_obj_stream() {
// A compound document with no streams: Java returns "" (unified as None on the Rust side)
        let fs = open(compound_file_with_streams(&[])).expect("open compound document");
        assert_eq!(MPPReader::get_file_format(&fs), None);

// Other streams exist but no \1CompObj: also returns None
        let fs = open(compound_file_with_streams(&[("Props", b"props")])).expect("open compound document");
        assert_eq!(MPPReader::get_file_format(&fs), None);
    }

    #[test]
    fn get_file_format_is_none_when_comp_obj_has_no_format() {
// applicationName is not "Microsoft Project 4.0" and the fileFormat length is 0
        let mut payload = vec![0u8; 28];
        append_string(&mut payload, "Microsoft Project 9.0");
        payload.extend_from_slice(&0i32.to_le_bytes());

        let bytes = compound_file_with_streams(&[("\u{1}CompObj", &payload)]);
        let fs = open(bytes).expect("open compound document");

// Java: fileFormat = compObj.getFileFormat() (null) → returns null
        assert_eq!(MPPReader::get_file_format(&fs), None);
    }

    #[test]
    fn get_file_format_is_none_for_a_damaged_comp_obj_stream() {
// Truncated \1CompObj: Java throws EOFException; the Rust side returns None (no panic)
        let bytes = compound_file_with_streams(&[("\u{1}CompObj", &[0u8; 4])]);
        let fs = open(bytes).expect("open compound document");

        assert_eq!(MPPReader::get_file_format(&fs), None);
    }

    #[test]
    fn read_reports_invalid_file_for_an_unregistered_format() {
        let bytes = compound_file_with_streams(&[(
            "\u{1}CompObj",
            &comp_obj_payload("Microsoft Project 14.0", Some("MSProject.MPP99")),
        )]);
        let fs = open(bytes).expect("open compound document");

        let error = MPPReader::new()
            .read(&fs)
            .err()
            .expect("unregistered format should error");

        // Java: throw new MPXJException(MPXJException.INVALID_FILE + ": " + format);
        assert_eq!(
            error.message(),
            "Invalid file format: MSProject.MPP99".to_owned()
        );
    }

    #[test]
    fn read_reports_invalid_file_when_the_format_is_null() {
// fileFormat missing → Java's FILE_CLASS_MAP.get(null) returns null → "Invalid file format: null"
        let mut payload = vec![0u8; 28];
        append_string(&mut payload, "Microsoft Project 9.0");
        payload.extend_from_slice(&0i32.to_le_bytes());

        let bytes = compound_file_with_streams(&[("\u{1}CompObj", &payload)]);
        let fs = open(bytes).expect("open compound document");

        let error = MPPReader::new().read(&fs).err().expect("should error");
        assert_eq!(error.message(), "Invalid file format: null".to_owned());
    }

    #[test]
    fn read_reports_an_error_when_the_comp_obj_stream_is_missing() {
// On the Java side this is a NullPointerException (uncaught); the Rust side converts it to Err without panicking
        let fs = open(compound_file_with_streams(&[])).expect("open compound document");

        let error = MPPReader::new().read(&fs).err().expect("should error");
        assert_eq!(error.message(), MpxjError::INVALID_FILE);
    }

    #[test]
    fn empty_and_damaged_files_are_reported_as_read_errors() {
// Empty input: POI throws IOException when constructing POIFSFileSystem → READ_ERROR
        assert!(PoifsFileSystem::new(Cursor::new(Vec::new())).is_err());
// 16 zero bytes of garbage data: likewise cannot be opened (no panic)
        assert!(PoifsFileSystem::new(Cursor::new(vec![0u8; 16])).is_err());

// `read(InputStream)` path: goes through read_stream_into_cursor + PoifsFileSystem::new
        let mut reader = MPPReader::new();
        let error = reader
            .read_reader(&mut &b"not an OLE compound document"[..])
            .err()
            .expect("should error");
        assert!(error.message().starts_with(MpxjError::READ_ERROR));

// Empty stream
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
        std::fs::write(&path, bytes).expect("write test file");

        let error = MPPReader::new()
            .read_file(&path)
            .err()
            .expect("unregistered format should error");

        assert_eq!(
            error.message(),
            "Invalid file format: MSProject.MPP99".to_owned()
        );

        let _ = std::fs::remove_file(&path);

// Nonexistent path: Java throws IOException → READ_ERROR
        let error = MPPReader::new()
            .read_file(Path::new("no-such-dir/no-such-file.mpp"))
            .err()
            .expect("non-existent path should error");
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

// Java's setReadPassword(null) is equivalent to None
        reader.set_read_password(None);
        assert_eq!(reader.get_read_password(), None);
    }

    #[test]
    fn file_class_map_covers_exactly_the_java_entries() {
// All 11 keys of FILE_CLASS_MAP should select a variant reader
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
                "{format} should select a variant reader"
            );
        }

// Unregistered format and null (Java's Map.get returns null)
        assert!(file_class_map(Some("MSProject.MPP4")).is_none());
        assert!(file_class_map(Some("")).is_none());
        assert!(file_class_map(None).is_none());
    }

    #[test]
    fn baseline_field_table_matches_the_java_static_block() {
// Java's TASK_ESTIMATED_BASELINE_FIELDS has 11 groups × 3 fields
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
