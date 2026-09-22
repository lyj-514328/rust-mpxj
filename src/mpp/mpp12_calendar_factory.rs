//! Corresponds to Java: `org/mpxj/mpp/MPP12CalendarFactory.java`
//!
//! MPP12 (Microsoft Project 2007) calendar reading: the working calendar block starts at offset 0,
//! and the exception date block reuses the implementation in [`crate::mpp::abstract_calendar_and_exception_factory`].
//!
//! # Restoring the inheritance relationship
//!
//! Java: `class MPP12CalendarFactory extends AbstractCalendarAndExceptionFactory`. Rust has no inheritance,
//! so it is restored via "composition + forwarding" (consistent with `VarMeta12` / `AbstractVarMeta`):
//!
//! | Java | Rust |
//! |---|---|
//! | `extends AbstractCalendarAndExceptionFactory` | field `base: AbstractCalendarAndExceptionFactory` (composition) |
//! | inherited `processCalendarData` | [`MPP12CalendarFactory::process_calendar_data`] forwards to `base` (which in turn forwards to `AbstractCalendarFactory`) |
//! | inherited `processCalendarExceptions` | [`MPP12CalendarFactory::process_calendar_exceptions`] forwards to `base.process_calendar_exceptions` |
//! | the 6 overridden accessors + `getCalendarVarMeta` | the corresponding implementations in [`AbstractCalendarFactoryMethods`] |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `ProjectFile file` | `Rc<RefCell<ProjectFile>>` | shared per the crate-wide convention |
//! | `DocumentInputStream` | `&[u8]` (`&document.get_data()`) | constructors uniformly take a byte slice |
//! | `Integer getCalendarNameVarDataType()` | `i32 get_calendar_name_var_data_type()` | Java's `Integer` is always non-null |
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: not used by this class.
//!
//! Skipped Java members: none.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::mpp::abstract_calendar_and_exception_factory::AbstractCalendarAndExceptionFactory;
use crate::mpp::abstract_calendar_factory::AbstractCalendarFactoryMethods;
use crate::mpp::calendar_factory::CalendarFactory;
use crate::mpp::document_input_stream_factory::DocumentInputStreamFactory;
use crate::mpp::ole::DirectoryEntry;
use crate::mpp::props::Props;
use crate::mpp::var_meta::VarMeta;
use crate::mpp::var_meta12::VarMeta12;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_calendar::ProjectCalendar;
use crate::project_file::ProjectFile;

/// VarData type holding the calendar name.
///
/// Corresponds to Java: `private static final Integer CALENDAR_NAME = Integer.valueOf(1);`
const CALENDAR_NAME: i32 = 1;

/// VarData type holding the calendar data.
///
/// Corresponds to Java: `private static final Integer CALENDAR_DATA = Integer.valueOf(8);`
const CALENDAR_DATA: i32 = 8;

/// MPP12-specific calendar factory.
///
/// Corresponds to Java: `class MPP12CalendarFactory extends AbstractCalendarAndExceptionFactory`
pub struct MPP12CalendarFactory {
/// The base class part (in Java this class is `AbstractCalendarAndExceptionFactory`).
    base: AbstractCalendarAndExceptionFactory,
}

impl MPP12CalendarFactory {
/// Constructor.
    ///
    /// Corresponds to Java: `public MPP12CalendarFactory(ProjectFile file)`
    pub fn new(file: Rc<RefCell<ProjectFile>>) -> Self {
        Self {
            base: AbstractCalendarAndExceptionFactory::new(file),
        }
    }

/// Get the composed base class part.
    ///
/// Rust-specific: Java uses base class members directly through inheritance, while the Rust side uses composition and so requires explicit access.
    pub fn get_base(&self) -> &AbstractCalendarAndExceptionFactory {
        &self.base
    }

/// Structure of the calendar data block: 4-byte block header + 7 x 60 bytes (seven days a week) + exception date block.
    ///
/// Corresponds to Java: `AbstractCalendarFactory#processCalendarData(...)` (not overridden by this class, inherited directly)
    pub fn process_calendar_data(
        &self,
        project_dir: &DirectoryEntry,
        project_props: &Props,
        input_stream_factory: &DocumentInputStreamFactory,
        resource_map: &mut HashMap<i32, Rc<RefCell<ProjectCalendar>>>,
    ) -> MpxjResult<()> {
        self.base
            .process_calendar_data(self, project_dir, project_props, input_stream_factory, resource_map)
    }

/// Parse the exception dates in the calendar data block.
    ///
    /// Corresponds to Java: `AbstractCalendarAndExceptionFactory#processCalendarExceptions(...)`
/// (not overridden by this class, inherited directly; the Rust side forwards explicitly because it has no inheritance)
    pub fn process_calendar_exceptions(&self, data: &[u8], cal: &Rc<RefCell<ProjectCalendar>>) {
        self.base.process_calendar_exceptions(data, cal);
    }
}

impl CalendarFactory for MPP12CalendarFactory {
/// Corresponds to Java: `CalendarFactory#processCalendarData(...)` (this class implements that interface via inheritance)
    ///
/// Forwards to the method of the same name as in Java to avoid duplicating logic.
    fn process_calendar_data(
        &self,
        project_dir: &DirectoryEntry,
        project_props: &Props,
        input_stream_factory: &DocumentInputStreamFactory,
        resource_map: &mut HashMap<i32, Rc<RefCell<ProjectCalendar>>>,
    ) -> MpxjResult<()> {
        MPP12CalendarFactory::process_calendar_data(
            self,
            project_dir,
            project_props,
            input_stream_factory,
            resource_map,
        )
    }
}

impl AbstractCalendarFactoryMethods for MPP12CalendarFactory {
    /// Corresponds to Java: `@Override protected int getCalendarIDOffset()`
    fn get_calendar_id_offset(&self) -> usize {
        0
    }

    /// Corresponds to Java: `@Override protected int getBaseIDOffset()`
    fn get_base_id_offset(&self) -> usize {
        4
    }

    /// Corresponds to Java: `@Override protected int getResourceIDOffset()`
    fn get_resource_id_offset(&self) -> usize {
        8
    }

    /// Corresponds to Java: `@Override protected int getCalendarHoursOffset()`
    fn get_calendar_hours_offset(&self) -> usize {
        0
    }

    /// Corresponds to Java: `@Override protected Integer getCalendarNameVarDataType()`
    fn get_calendar_name_var_data_type(&self) -> i32 {
        CALENDAR_NAME
    }

    /// Corresponds to Java: `@Override protected Integer getCalendarDataVarDataType()`
    fn get_calendar_data_var_data_type(&self) -> i32 {
        CALENDAR_DATA
    }

/// Read `VarMeta12`.
    ///
    /// Corresponds to Java: `@Override protected VarMeta getCalendarVarMeta(DirectoryEntry directory) throws IOException`
    /// → `return new VarMeta12(new DocumentInputStream(((DocumentEntry) directory.getEntry("VarMeta"))));`
    fn get_calendar_var_meta(&self, directory: &DirectoryEntry) -> MpxjResult<Rc<dyn VarMeta>> {
// Java throws an exception (POI) or NullPointerException when an entry is missing; the Rust side returns Err.
        let Some(document) = directory.get_document("VarMeta") else {
            return Err(MpxjError::new("Missing VarMeta"));
        };

// By convention: Java constructors that take an InputStream are changed to take &[u8]
        Ok(Rc::new(VarMeta12::new(&document.get_data())?))
    }

/// Forward to the base class implementation (Java obtains this implementation through inheritance).
    ///
    /// Corresponds to Java: `AbstractCalendarAndExceptionFactory#processCalendarExceptions(byte[] data, ProjectCalendar cal)`
    fn process_calendar_exceptions(&self, data: &[u8], cal: &Rc<RefCell<ProjectCalendar>>) {
        self.base.process_calendar_exceptions(data, cal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_match_the_mpp12_layout() {
        let factory = MPP12CalendarFactory::new(ProjectFile::new());
        assert_eq!(factory.get_calendar_id_offset(), 0);
        assert_eq!(factory.get_base_id_offset(), 4);
        assert_eq!(factory.get_resource_id_offset(), 8);
        assert_eq!(factory.get_calendar_hours_offset(), 0);
        assert_eq!(factory.get_calendar_name_var_data_type(), CALENDAR_NAME);
        assert_eq!(factory.get_calendar_data_var_data_type(), CALENDAR_DATA);
// The base class is accessible (the composition relationship holds)
        let _base: &crate::mpp::abstract_calendar_factory::AbstractCalendarFactory =
            factory.get_base().get_base();
    }

    #[test]
    fn exceptions_are_forwarded_to_the_base_implementation() {
        let cal = Rc::new(RefCell::new(ProjectCalendar::new(
            std::rc::Weak::new(),
            false,
        )));
        let factory = MPP12CalendarFactory::new(ProjectFile::new());

// fewer than 421 bytes: the base class implementation returns directly
        factory.process_calendar_exceptions(&vec![0u8; 100], &cal);
        assert_eq!(cal.borrow_mut().get_calendar_exceptions().len(), 0);
    }
}
