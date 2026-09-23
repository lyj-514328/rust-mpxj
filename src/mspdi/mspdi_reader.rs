//! Corresponds to Java: `org/mpxj/mspdi/MSPDIReader.java`
//!
//! The reader that constructs a [`ProjectFile`] by reading an MSPDI (Microsoft Project XML) file.
//!
//! # Two-stage pipeline (Java → Rust)
//!
//! Java unmarshals the document into the JAXB model and then copies values from that model into the
//! domain objects. The Rust port keeps the same split:
//!
//! ```text
//! bytes ──decode with m_charset──► String ──roxmltree──► schema::project::Project
//!                                                              │
//!                                        ReaderContext::read_project (the ported readXxx methods)
//!                                                              ▼
//!                                                        ProjectFile
//! ```
//!
//! # Wiring (Java inheritance chain → Rust traits)
//!
//! Java is `public final class MSPDIReader extends AbstractProjectStreamReader implements
//! HasCharset`. Rust has no inheritance, so it is wired per the conventions in the header of
//! [`crate::reader::abstract_project_reader`]:
//!
//! - `impl ProjectReader for MSPDIReader`: each interface method is given individually.
//!   `add_project_listener` forwards to [`AbstractProjectReader::add_project_listener_default`],
//!   `read_file` / `read_all_file` / `read_all_reader` forward to [`AbstractProjectStreamReader`]'s
//!   `*_default`, and `read_reader` (Java's `read(InputStream)`) is implemented by this class;
//! - `impl AbstractProjectReader for MSPDIReader`: provides listener storage;
//! - `impl AbstractProjectStreamReader for MSPDIReader {}`: nothing left to implement.
//!
//! Java's `HasCharset` has no Rust counterpart; [`MSPDIReader::set_charset`] /
//! [`MSPDIReader::get_charset`] expose the same behaviour directly.
//!
//! # State: [`ReaderContext`] replaces the Java instance fields
//!
//! Java keeps `m_projectFile`, `m_lookupTableMap`, `m_customFieldValueItems` and
//! `m_externalProjects` as instance fields of `MSPDIReader`, sets them at the start of
//! `read(ProjectFile, Project)` and nulls them in a `finally` block. The Rust port moves them into
//! [`ReaderContext`], which is created by [`MSPDIReader::read`] and dropped when the read finishes
//! (same lifetime as Java's `finally`). The ported `readXxx` methods are `impl ReaderContext`
//! blocks split across the sibling modules ([`crate::mspdi::reader_project_properties`],
//! [`crate::mspdi::reader_calendars`], [`crate::mspdi::reader_resources`],
//! [`crate::mspdi::reader_tasks`], [`crate::mspdi::reader_assignments`],
//! [`crate::mspdi::reader_custom_fields`]) so that each `readXxx` section can be compared against
//! the corresponding Java line range.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `JAXBContext CONTEXT` created in a `static` block, with `CONTEXT_EXCEPTION` if creation failed; `read` throws `CONTEXT_EXCEPTION` when `CONTEXT == null` | not reproduced | There is no JAXB context to create. [`roxmltree::Document::parse`] reports its own errors |
//! | `UnmarshalHelper.unmarshal(...)` with a SAX parser, `NamespaceFilter`, and `!m_compatibleInput` as `ignoreValidationErrors` | [`roxmltree::Document::parse`] followed by [`crate::mspdi::schema::project::Project::from_node`] | See [`crate::mspdi::xml_node`] for the namespace handling. JAXB validation is not modelled at all, so [`MSPDIReader::set_microsoft_project_compatible_input`] has no effect in Rust (Java only used it to suppress validation warnings, and a document that fails JAXB validation still unmarshals) |
//! | `new InputStreamReader(stream, getCharset())` — the configured charset is used as-is, and a byte order mark stays in the character stream (which then makes SAX fail) | [`encoding_rs::Encoding::decode`], which sniffs and removes a byte order mark | Deliberately more tolerant: a document with a UTF-8 BOM reads correctly here. `UniversalProjectReader` is what strips the BOM in Java, and this crate does not port it |
//! | `read(InputStream)` catches `ParserConfigurationException` / `IOException` / `SAXException` / `JAXBException` and throws `MPXJException("Failed to parse file", ex)` | returns [`crate::MpxjError`] with the same message | `MpxjError::with_cause` needs a `&dyn Error`; `roxmltree::Error` and `std::io::Error` both qualify |
//! | `ProjectFile m_projectFile` etc. as instance fields, nulled in `finally` | [`ReaderContext`], owned by `read` | See above |
//! | `Map<Task, Project> m_externalProjects` (a `HashMap` whose iteration order is unspecified) | `Vec<(Rc<RefCell<Task>>, Project)>`, in insertion order | Same set of embedded projects, deterministic order |
//! | `ProjectConfig config` read from the file | same | The nine `setAutoXxx(false)` calls are reproduced one by one |
//! | `m_projectFile.getCalendars().removeIf(...)` for the derived-calendar prune | collects the doomed calendars first, then calls [`crate::project_calendar_container::ProjectCalendarContainer::remove`] | Java's `removeIf` mutates while iterating; the Rust side cannot borrow the container twice, so the filter runs first |
//!
//! # Skipped Java members + reason
//!
//! - `CONTEXT` / `CONTEXT_EXCEPTION` (the `static` JAXB context and its failure marker): there is no
//!   JAXB context to build, see the table above.
//! - `setCharset` / `getCharset` come from Java's `HasCharset` interface; Rust has no such interface,
//!   so they are plain inherent methods on [`MSPDIReader`] with the same behaviour.
//! - `MSPDIWriter` and `SaveVersion`: writer-only, and this crate is reader-only.
//!
//! Every other member of `MSPDIReader.java` is ported; the `readXxx` sections live in the sibling
//! `reader_*.rs` modules, and each of their headers lists the differences for that section.

use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::io::Read;
use std::rc::Rc;

use encoding_rs::Encoding;
use roxmltree::Document;
use uuid::Uuid;

use crate::calendar_type::CalendarType;
use crate::common::mpp_field_type::MppFieldKey;
use crate::event_manager::EventManager;
use crate::listener::project_listener::ProjectListener;
use crate::mpp::custom_field_value_item::CustomFieldValueItem;
use crate::mspdi::schema::project::Project;
use crate::project_calendar::ProjectCalendar;
use crate::project_file::ProjectFile;
use crate::project_properties::ProjectProperties;
use crate::reader::abstract_project_reader::{AbstractProjectReader, SharedProjectListener};
use crate::reader::abstract_project_stream_reader::{
    AbstractProjectStreamReader, read_stream_into_cursor,
};
use crate::reader::project_reader::ProjectReader;
use crate::resource::Resource;
use crate::task::Task;
use crate::MpxjError;

/// Creates a new [`ProjectFile`] instance by reading an MSPDI file.
///
/// Corresponds to Java: `public final class MSPDIReader extends AbstractProjectStreamReader implements HasCharset`
pub struct MSPDIReader {
    /// Corresponds to Java: `m_projectListeners` (held by `AbstractProjectReader`).
    ///
    /// Rust-specific name; the storage is required by [`AbstractProjectReader`].
    listeners: Vec<SharedProjectListener>,
    /// Corresponds to Java: `private boolean m_compatibleInput = true`
    /// (`MSPDIReader.setMicrosoftProjectCompatibleInput` inverts it when calling the unmarshaller).
    m_compatible_input: bool,
    /// Corresponds to Java: `private boolean m_ignoreErrors = true`
    m_ignore_errors: bool,
    /// Corresponds to Java: `private Charset m_charset = CharsetHelper.UTF8`
    m_charset: &'static Encoding,
}

impl MSPDIReader {
    /// Corresponds to Java: implicit no-arg constructor.
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
            m_compatible_input: true,
            m_ignore_errors: true,
            m_charset: encoding_rs::UTF_8,
        }
    }

    /// Set the Charset used to read the file.
    ///
    /// Corresponds to Java: `@Override public void setCharset(Charset charset)`
    pub fn set_charset(&mut self, charset: &'static Encoding) {
        self.m_charset = charset;
    }

    /// Retrieve the Charset used to read the file.
    ///
    /// Corresponds to Java: `@Override public Charset getCharset()`
    pub fn get_charset(&self) -> &'static Encoding {
        self.m_charset
    }

    /// Configure whether input is expected to be written by Microsoft Project.
    ///
    /// Corresponds to Java: `public void setMicrosoftProjectCompatibleInput(boolean flag)`
    ///
    /// Java passes the inverse to the JAXB unmarshaller as `ignoreValidationErrors`. The Rust port
    /// has no validation step, so the flag is stored but never read; it is kept so that the API
    /// matches Java (see the file header).
    pub fn set_microsoft_project_compatible_input(&mut self, flag: bool) {
        self.m_compatible_input = flag;
    }

    /// Retrieve the "Microsoft Project compatible input" flag.
    ///
    /// Corresponds to Java: `public boolean getMicrosoftProjectCompatibleInput()`
    pub fn get_microsoft_project_compatible_input(&self) -> bool {
        self.m_compatible_input
    }

    /// Configure whether parse errors are collected rather than thrown.
    ///
    /// Corresponds to Java: `public void setIgnoreErrors(boolean ignoreErrors)`
    pub fn set_ignore_errors(&mut self, ignore_errors: bool) {
        self.m_ignore_errors = ignore_errors;
    }

    /// Retrieve the "ignore errors" flag.
    ///
    /// Corresponds to Java: `public boolean getIgnoreErrors()`
    pub fn get_ignore_errors(&self) -> bool {
        self.m_ignore_errors
    }

    /// Read a project file from an input stream.
    ///
    /// Corresponds to Java: `@Override public ProjectFile read(InputStream stream) throws MPXJException`
    pub fn read(&mut self, stream: &mut dyn Read) -> Result<Rc<RefCell<ProjectFile>>, MpxjError> {
        // Java: new InputSource(new InputStreamReader(stream, getCharset()))
        let cursor = read_stream_into_cursor(stream)?;
        let text = self.decode(cursor.into_inner());

        // Java: (JAXBElement<Project>) UnmarshalHelper.unmarshal(...).getValue()
        let document = Document::parse(&text)
            .map_err(|error| MpxjError::with_cause("Failed to parse file", &error))?;
        let project = Project::from_node(document.root_element());

        // Java: ProjectFile projectFile = new ProjectFile(); ... read(projectFile, project);
        let file = ProjectFile::new();

        // Java: addListenersToProject(m_projectFile);
        self.add_listeners_to_project(&file);

        let mut context = ReaderContext::new(Rc::clone(&file), self.m_ignore_errors);
        context.read_project(&project);

        Ok(file)
    }

    /// Decode the raw file bytes using the configured charset.
    ///
    /// Corresponds to Java: `new InputStreamReader(stream, getCharset())` — see the file header for
    /// the byte order mark difference.
    fn decode(&self, bytes: Vec<u8>) -> String {
        let (text, _, _) = self.m_charset.decode(&bytes);
        text.into_owned()
    }
}

impl Default for MSPDIReader {
    /// Corresponds to Java: implicit no-arg constructor.
    fn default() -> Self {
        Self::new()
    }
}

/// The Java instance state of `MSPDIReader` during a read.
///
/// Java keeps these as instance fields and nulls them in a `finally` block; the Rust port keeps them
/// in this context, which is created by [`MSPDIReader::read`] and dropped when the read finishes.
/// The ported `readXxx` methods are `impl ReaderContext` blocks in the sibling `reader_*` modules.
///
/// Rust-specific (no Java counterpart as a class; the fields correspond to Java's
/// `m_projectFile`, `m_eventManager`, `m_lookupTableMap`, `m_customFieldValueItems` and
/// `m_externalProjects`).
pub(crate) struct ReaderContext {
    /// Corresponds to Java: `private ProjectFile m_projectFile`
    pub(crate) file: Rc<RefCell<ProjectFile>>,
    /// Corresponds to Java: `private boolean m_ignoreErrors`
    ///
    /// Java stores this in `DatatypeConverter`'s `IGNORE_ERRORS` `ThreadLocal`; the Rust port keeps
    /// it here and passes it to [`crate::mspdi::datatype_converter::DatatypeConverter`] explicitly.
    pub(crate) ignore_errors: bool,
    /// Corresponds to Java: `private Map<UUID, FieldType> m_lookupTableMap`
    ///
    /// Java keys this map by `FieldType`; the Rust side uses the equivalent hashable key type
    /// [`MppFieldKey`] (see [`crate::field_key`]).
    pub(crate) lookup_table_map: HashMap<Uuid, MppFieldKey>,
    /// Corresponds to Java: `private Map<FieldType, Map<BigInteger, CustomFieldValueItem>> m_customFieldValueItems`
    pub(crate) custom_field_value_items: HashMap<MppFieldKey, HashMap<i64, CustomFieldValueItem>>,
    /// Corresponds to Java: `private Map<Task, Project> m_externalProjects`
    ///
    /// Java uses a `HashMap` whose iteration order is unspecified; the Rust side keeps insertion
    /// order in a `Vec` (same set of embedded projects).
    pub(crate) external_projects: Vec<(Rc<RefCell<Task>>, Project)>,
}

impl ReaderContext {
    /// Rust-specific constructor (Java assigns the fields at the start of `read(ProjectFile, Project)`).
    pub(crate) fn new(file: Rc<RefCell<ProjectFile>>, ignore_errors: bool) -> Self {
        Self {
            file,
            ignore_errors,
            lookup_table_map: HashMap::new(),
            custom_field_value_items: HashMap::new(),
            external_projects: Vec::new(),
        }
    }

    /// Read a project from an already-unmarshalled model.
    ///
    /// Corresponds to Java: `void read(ProjectFile projectFile, Project project)`
    ///
    /// The [`ReaderContext::file`] field replaces the `projectFile` parameter and the `finally`
    /// block that nulls the Java instance fields.
    pub(crate) fn read_project(&mut self, project: &Project) {
        //
        // Java: ProjectConfig config = m_projectFile.getProjectConfig(); config.setAutoXxx(false);
        //
        {
            let context = self.file.borrow().get_project_context();
            let mut context_ref = context.borrow_mut();
            let config = context_ref.get_project_config_mut();
            config.set_auto_task_id(false);
            config.set_auto_task_unique_id(false);
            config.set_auto_resource_id(false);
            config.set_auto_resource_unique_id(false);
            config.set_auto_outline_level(false);
            config.set_auto_outline_number(false);
            config.set_auto_wbs(false);
            config.set_auto_calendar_unique_id(false);
            config.set_auto_assignment_unique_id(false);
        }

        let mut calendar_map: HashMap<i64, Rc<RefCell<ProjectCalendar>>> = HashMap::new();

        self.read_project_properties(project);
        self.read_extended_attribute_definitions(project);
        self.read_outline_code_definitions(project);
        self.read_calendars(project, &mut calendar_map);
        self.read_resources(project, &calendar_map);
        self.read_tasks(project);
        self.read_assignments(project);

        self.prune_unused_derived_calendars();
        self.post_process_resource_calendars();

        self.file.borrow_mut().read_complete();

        self.process_external_projects();
    }

    /// Prune derived calendars which are not referenced anywhere.
    ///
    /// Corresponds to Java: the "Prune unused derived calendars" block of `read(ProjectFile, Project)`
    ///
    /// Normally we'd just be getting rid of any odd resource calendars which might have slipped in
    /// which aren't actually associated with a resource. When applications other than Microsoft
    /// Project generate MSPDI files they can (incorrectly) produce more deeply nested hierarchies,
    /// so we need to be careful here to only remove calendars which are derived but are not
    /// referenced anywhere.
    fn prune_unused_derived_calendars(&mut self) {
        let (calendar_references, _) = self.calendar_references(CalendarReferenceParts::All);

        let doomed: Vec<Rc<RefCell<ProjectCalendar>>> = self
            .calendars()
            .into_iter()
            .filter(|calendar| {
                let calendar = calendar.borrow();
                calendar.is_derived()
                    && !calendar_references.contains(&calendar.get_unique_id().unwrap_or_default())
            })
            .collect();

        if doomed.is_empty() {
            return;
        }

        // Java: m_projectFile.getCalendars().removeIf(...) — the Rust container cannot be filtered
        // while it is borrowed, so the doomed calendars are collected first (see the file header).
        let context = self.file.borrow().get_project_context();
        let Ok(mut context) = context.try_borrow_mut() else {
            return;
        };

        for calendar in doomed {
            let calendar = calendar.borrow();
            let _ = context.get_calendars_mut().remove(&calendar);
        }
    }

    /// Configure the derived resource calendars and give unnamed calendars their resource's name.
    ///
    /// Corresponds to Java: the "Resource calendar post processing" block of
    /// `read(ProjectFile, Project)`
    fn post_process_resource_calendars(&mut self) {
        let (_, resource_calendar_map) = self.calendar_references(CalendarReferenceParts::ResourceCalendars);

        for resource in self.resources() {
            let calendar = resource.borrow().get_calendar();
            let Some(calendar) = calendar else {
                continue;
            };

            let (unique_id, is_derived, name) = {
                let calendar_ref = calendar.borrow();
                (
                    calendar_ref.get_unique_id(),
                    calendar_ref.is_derived(),
                    calendar_ref.get_name().map(str::to_owned),
                )
            };

            if is_derived {
                // Java: calendar.setPersonal(resourceCalendarMap.computeIfAbsent(
                //          calendar.getUniqueID(), k -> Collections.emptyList()).size() == 1);
                let personal = resource_calendar_map
                    .get(&unique_id.unwrap_or_default())
                    .map(|resources| resources.len() == 1)
                    .unwrap_or(true);

                let mut calendar_ref = calendar.borrow_mut();
                calendar_ref.set_type(Some(CalendarType::Resource));
                calendar_ref.set_personal(personal);
            }

            // Resource calendars without names inherit the resource name
            let needs_name = name.map(|name| name.is_empty()).unwrap_or(true);
            if needs_name {
                let mut resource_name = resource.borrow().get_name();
                if resource_name.as_deref().map(str::is_empty).unwrap_or(true) {
                    resource_name = Some("Unnamed Resource".to_owned());
                }
                calendar.borrow_mut().set_name(resource_name);
            }
        }
    }

    /// Process any embedded external projects.
    ///
    /// Corresponds to Java: the "Process any embedded external projects" block of
    /// `read(ProjectFile, Project)`
    ///
    /// Java creates a fresh `MSPDIReader` for each embedded project (`new MSPDIReader()`), so the
    /// embedded file is read with default charset / flags rather than this reader's; the Rust side
    /// matches that.
    fn process_external_projects(&mut self) {
        let external_projects = std::mem::take(&mut self.external_projects);

        for (task, project) in external_projects {
            let mut reader = MSPDIReader::new();
            let nested_file = ProjectFile::new();
            let mut context = ReaderContext::new(Rc::clone(&nested_file), reader.m_ignore_errors);
            context.read_project(&project);

            // The nested reader is only used for its charset / flag defaults, so it is kept alive
            // for the duration of the read above (mirrors Java's `reader.read(file, ...)`).
            reader.set_charset(reader.get_charset());

            task.borrow_mut().set_subproject_object(Some(nested_file));
        }
    }

    /// Collect the calendar Unique IDs referenced by the project.
    ///
    /// Corresponds to Java: the `resourceCalendarMap` / `calendarReferences` setup of
    /// `read(ProjectFile, Project)`
    ///
    /// Java builds the resource calendar map and the reference set in one pass; the Rust side
    /// exposes the two halves so that each caller only builds what it needs (the resource calendar
    /// map is only needed by the post-processing step).
    fn calendar_references(
        &self,
        _parts: CalendarReferenceParts,
    ) -> (std::collections::HashSet<i32>, HashMap<i32, Vec<Rc<RefCell<Resource>>>>) {
        let mut resource_calendar_map: HashMap<i32, Vec<Rc<RefCell<Resource>>>> = HashMap::new();
        let mut calendar_references = std::collections::HashSet::new();

        // Java builds the resource calendar map as part of the common reference pass. The Rust
        // helper originally skipped it for `All`, which caused valid derived resource calendars
        // to be pruned before post-processing.
        for resource in self.resources() {
            let unique_id = resource.borrow().get_calendar_unique_id();
            if let Some(unique_id) = unique_id {
                resource_calendar_map
                    .entry(unique_id)
                    .or_default()
                    .push(resource);
            }
        }

        calendar_references
            .insert(self.file.borrow().get_project_properties().get_default_calendar_unique_id().unwrap_or_default());

        for calendar in self.calendars() {
            if let Some(unique_id) = calendar.borrow().get_parent_unique_id() {
                calendar_references.insert(unique_id);
            }
        }

        for task in self.tasks() {
            if let Some(unique_id) = task.borrow().get_calendar_unique_id() {
                calendar_references.insert(unique_id);
            }
        }

        calendar_references.extend(resource_calendar_map.keys().copied());

        (calendar_references, resource_calendar_map)
    }

    /// Rust-specific helper: a snapshot of the project's calendars.
    ///
    /// The domain containers live behind `Rc<RefCell<_>>`, so a snapshot avoids holding a borrow of
    /// the project context while the ported methods (which borrow it themselves) run.
    pub(crate) fn calendars(&self) -> Vec<Rc<RefCell<ProjectCalendar>>> {
        let context = self.file.borrow().get_project_context();
        let Ok(context) = context.try_borrow() else {
            return Vec::new();
        };
        context.get_calendars().iter().cloned().collect()
    }

    /// Rust-specific helper: a snapshot of the project's resources.
    pub(crate) fn resources(&self) -> Vec<Rc<RefCell<Resource>>> {
        self.file.borrow().get_resources().iter().cloned().collect()
    }

    /// Rust-specific helper: a snapshot of the project's tasks.
    pub(crate) fn tasks(&self) -> Vec<Rc<RefCell<Task>>> {
        self.file.borrow().get_tasks().iter().cloned().collect()
    }

    /// Rust-specific helper: borrow the project properties immutably.
    pub(crate) fn properties(&self) -> Option<Ref<'_, ProjectProperties>> {
        let file = self.file.try_borrow().ok()?;
        Some(Ref::map(file, |file| file.get_project_properties()))
    }

    /// Rust-specific helper: borrow the project properties mutably.
    pub(crate) fn properties_mut(&self) -> Option<RefMut<'_, ProjectProperties>> {
        let file = self.file.try_borrow_mut().ok()?;
        Some(RefMut::map(file, |file| file.get_project_properties_mut()))
    }

    /// Fire an event through the project's `EventManager`.
    ///
    /// Rust-specific helper (Java calls `m_eventManager.fireXxx` directly; see
    /// [`crate::mpx::mpx_reader`] for the same pattern).
    pub(crate) fn with_event_manager(&self, fire: impl FnOnce(&mut EventManager)) {
        let context = match self.file.try_borrow() {
            Ok(file) => file.get_project_context(),
            Err(_) => return,
        };

        match context.try_borrow_mut() {
            Ok(mut context) => fire(context.get_event_manager_mut()),
            Err(_) => return,
        }
    }
}

/// Which half of the Java `calendarReferences` setup a caller needs.
///
/// Rust-specific (Java computes both in one block of `read(ProjectFile, Project)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CalendarReferenceParts {
    /// Build both the reference set and the resource calendar map.
    All,
    /// Build only the resource calendar map.
    ResourceCalendars,
}

impl ProjectReader for MSPDIReader {
    /// Register a listener.
    ///
    /// Corresponds to Java: `@Override public void addProjectListener(ProjectListener listener)`
    fn add_project_listener(&mut self, listener: Box<dyn ProjectListener>) {
        self.add_project_listener_default(listener);
    }

    /// Read one schedule by path.
    ///
    /// Corresponds to Java: `@Override public ProjectFile read(File file)` (defined in
    /// `AbstractProjectStreamReader`)
    fn read_file(&mut self, path: &std::path::Path) -> Result<Rc<RefCell<ProjectFile>>, MpxjError> {
        AbstractProjectStreamReader::read_file_default(self, path)
    }

    /// Read all schedules by path (MSPDI has only a single schedule).
    ///
    /// Corresponds to Java: `@Override public List<ProjectFile> readAll(File file)`
    fn read_all_file(
        &mut self,
        path: &std::path::Path,
    ) -> Result<Vec<Rc<RefCell<ProjectFile>>>, MpxjError> {
        AbstractProjectStreamReader::read_all_file_default(self, path)
    }

    /// Read one schedule from a reader.
    ///
    /// Corresponds to Java: `@Override public ProjectFile read(InputStream is)`
    fn read_reader(&mut self, reader: &mut dyn Read) -> Result<Rc<RefCell<ProjectFile>>, MpxjError> {
        self.read(reader)
    }

    /// Read all schedules from a reader (MSPDI has only a single schedule).
    ///
    /// Corresponds to Java: `@Override public List<ProjectFile> readAll(InputStream inputStream)`
    fn read_all_reader(
        &mut self,
        reader: &mut dyn Read,
    ) -> Result<Vec<Rc<RefCell<ProjectFile>>>, MpxjError> {
        AbstractProjectStreamReader::read_all_reader_default(self, reader)
    }
}

impl AbstractProjectReader for MSPDIReader {
    /// Get the registered listeners.
    ///
    /// Rust-specific (Java's `m_projectListeners` is a private field, see the
    /// [`crate::reader::abstract_project_reader`] file header).
    fn project_listeners(&self) -> &[SharedProjectListener] {
        &self.listeners
    }

    /// Get the registered listeners (mutable).
    ///
    /// Rust-specific (as above).
    fn project_listeners_mut(&mut self) -> &mut Vec<SharedProjectListener> {
        &mut self.listeners
    }
}

impl AbstractProjectStreamReader for MSPDIReader {}

#[cfg(test)]
mod tests {
    use super::{MSPDIReader, ProjectReader};
    use std::io::Cursor;

    /// A minimal document: the reader accepts it and turns the project config's automatic
    /// numbering off (`read_project`).
    #[test]
    fn reads_a_minimal_document() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Project xmlns="http://schemas.microsoft.com/project"><SaveVersion>14</SaveVersion></Project>"#;

        let mut reader = MSPDIReader::new();
        let file = reader.read(&mut Cursor::new(xml.as_bytes())).unwrap();

        let file_ref = file.borrow();
        let context = file_ref.get_project_context();
        let mut context_ref = context.borrow_mut();
        let config = context_ref.get_project_config_mut();

        assert!(!config.get_auto_task_id());
        assert!(!config.get_auto_task_unique_id());
        assert!(!config.get_auto_resource_id());
        assert!(!config.get_auto_resource_unique_id());
        assert!(!config.get_auto_outline_level());
        assert!(!config.get_auto_outline_number());
        assert!(!config.get_auto_wbs());
        assert!(!config.get_auto_calendar_unique_id());
        assert!(!config.get_auto_assignment_unique_id());
    }

    /// A byte order mark is tolerated (Java relies on `UniversalProjectReader` to strip it).
    #[test]
    fn tolerates_a_byte_order_mark() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"<Project><SaveVersion>14</SaveVersion></Project>");

        let mut reader = MSPDIReader::new();
        assert!(reader.read(&mut Cursor::new(bytes)).is_ok());
    }

    /// Malformed XML is reported as "Failed to parse file" rather than panicking.
    #[test]
    fn reports_malformed_xml() {
        let mut reader = MSPDIReader::new();
        let error = reader
            .read(&mut Cursor::new(b"<Project><SaveVersion></Project>".as_slice()))
            .err()
            .unwrap();

        assert!(error.message().starts_with("Failed to parse file"));
    }

    /// The charset can be changed (Java's `HasCharset`).
    #[test]
    fn charset_round_trip() {
        let mut reader = MSPDIReader::new();
        assert_eq!(reader.get_charset().name(), "UTF-8");

        reader.set_charset(encoding_rs::WINDOWS_1252);
        assert_eq!(reader.get_charset().name(), "windows-1252");
    }

    /// A document that reaches the stream entry point unchanged (`read_reader` forwards to `read`).
    #[test]
    fn read_reader_forwards_to_read() {
        let mut reader = MSPDIReader::new();
        let result = reader
            .read_reader(&mut Cursor::new(b"<Project/>".as_slice()))
            .unwrap();

        assert_eq!(result.borrow().get_tasks().len(), 0);
    }
}
