//! Corresponds to Java: `org/mpxj/mpp/GanttBarStyleFactory.java`
//!
//! Factory interface for reading Gantt bar styles from an MPP file.
//!
//! # Porting trade-offs
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `interface GanttBarStyleFactory` (package-visible) | `pub trait GanttBarStyleFactory` | interface → `trait`; changed to `pub` because the factory type itself is also `pub`, and Rust does not allow a public type's trait implementation to reference a private trait |
//! | `ProjectFile file` | `&Rc<RefCell<ProjectFile>>` | Convention in this project: `ProjectFile` is shared via `Rc<RefCell<..>>` |
//! | `Props props` (base class) | `&Props` | On the Rust side, `Props9` / `Props12` / `Props14` are standalone types implemented as "composition + `Deref`" (see the module docs such as `crate::mpp::props9`); `&Props9` and the like are automatically coerced to `&Props` via `Deref`, so Java's "passing a subclass instance to a base class parameter" also holds on the Rust side, and the signature matches Java exactly |
//! | `GanttBarStyle[]` / `GanttBarStyleException[]` | `Vec<GanttBarStyle>` / `Vec<GanttBarStyleException>` | array → `Vec` |
//!
//! Java's two implementation classes `GanttBarStyleFactoryCommon` and `GanttBarStyleFactory14` both **directly**
//! `implements GanttBarStyleFactory`, with no inheritance relationship between them (the
//! "`GanttBarStyleFactory14 extends GanttBarStyleFactoryCommon`" mentioned in the task description does not exist in the actual Java source,
//! and the source has been taken as authoritative), so the Rust side does not need composition + forwarding.
//!
//! Skipped Java members: none.

use std::cell::RefCell;
use std::rc::Rc;

use crate::mpp::gantt_bar_style::GanttBarStyle;
use crate::mpp::gantt_bar_style_exception::GanttBarStyleException;
use crate::mpp::props::Props;
use crate::project_file::ProjectFile;

/// Factory for reading Gantt bar styles.
///
/// Corresponds to Java: `interface GanttBarStyleFactory`
pub trait GanttBarStyleFactory {
/// Read the default set of Gantt bar styles.
    ///
    /// Corresponds to Java: `GanttBarStyle[] processDefaultStyles(ProjectFile file, Props props)`
    fn process_default_styles(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &Props,
    ) -> Vec<GanttBarStyle>;

/// Read the set of single-task exception bar styles.
    ///
    /// Corresponds to Java: `GanttBarStyleException[] processExceptionStyles(ProjectFile file, Props props)`
    fn process_exception_styles(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &Props,
    ) -> Vec<GanttBarStyleException>;
}
