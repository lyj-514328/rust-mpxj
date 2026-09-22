//! Corresponds to Java: `org/mpxj/mpp/CalendarFactory.java`
//!
//! Unified entry point for the process of "reading calendar data from an MPP file": in Java this is `interface CalendarFactory`,
//! with only one method, `processCalendarData`.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `interface CalendarFactory` | `pub trait CalendarFactory` | interface → trait |
//! | `DirectoryEntry projectDir` | `&DirectoryEntry` | The storage handle of [`crate::mpp::ole`] on the Rust side |
//! | `HashMap<Integer, ProjectCalendar> resourceMap` | `&mut HashMap<i32, Rc<RefCell<ProjectCalendar>>>` | `ProjectCalendar` is shared via `Rc<RefCell<..>>`; the `HashMap` needs a mutable reference in order to `put` |
//! | `throws IOException` | `-> MpxjResult<()>` | Unified error type; a missing directory/stream returns `Err` |
//!
//! Java's `int` → Rust's `i32`, `Integer` → `i32` (used here as a map key, which is also necessarily non-null in Java).
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this interface does not use them.
//!
//! Skipped Java members: none.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::mpp::document_input_stream_factory::DocumentInputStreamFactory;
use crate::mpp::ole::DirectoryEntry;
use crate::mpp::props::Props;
use crate::mpxj_exception::MpxjResult;
use crate::project_calendar::ProjectCalendar;

/// Read calendar data from an MPP file.
///
/// Corresponds to Java: `interface CalendarFactory`
///
/// Java has only one implementation chain (`AbstractCalendarFactory` → `AbstractCalendarAndExceptionFactory`
/// → `MPP9/12/14CalendarFactory`); the three implementations of this trait are described in
/// [`crate::mpp::mpp9_calendar_factory::MPP9CalendarFactory`],
/// [`crate::mpp::mpp12_calendar_factory::MPP12CalendarFactory`],
/// [`crate::mpp::mpp14_calendar_factory::MPP14CalendarFactory`].
pub trait CalendarFactory {
/// Parse the `TBkndCal` storage, populate the calendars into `ProjectFile`, and update `resource_map`.
    ///
    /// Corresponds to Java: `void processCalendarData(DirectoryEntry projectDir, Props projectProps,
    /// DocumentInputStreamFactory inputStreamFactory, HashMap<Integer, ProjectCalendar> resourceMap)
    /// throws IOException`
    ///
/// `project_dir` is the project-level storage in the MPP file (for example `"   114"`),
/// and `resource_map` is the mapping "resource ID → calendar handle" (held by the caller and updated by reference).
    fn process_calendar_data(
        &self,
        project_dir: &DirectoryEntry,
        project_props: &Props,
        input_stream_factory: &DocumentInputStreamFactory,
        resource_map: &mut HashMap<i32, Rc<RefCell<ProjectCalendar>>>,
    ) -> MpxjResult<()>;
}
