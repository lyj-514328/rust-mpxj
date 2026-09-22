//! Corresponds to Java: `org/mpxj/mpp/ViewFactory.java`
//!
//! The factory contract for "creating view objects from the view data of an MPP file", implemented by `ViewFactory9/12/14`.
//!
//! Skipped Java mechanisms and the reasons:
//!
//! | Java | Reason |
//! |---|---|
//! | Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable` | not used by this interface |
//!
//! # How the interface is restored
//!
//! Java's `interface ViewFactory` (package-private) is restored on the Rust side as the [`ViewFactory`] trait; the only
//! `createView` method corresponds to [`ViewFactory::create_view`]. Rust has no package-private visibility, so the trait is `pub`.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `View createView(ProjectFile file, byte[] fixedMeta, byte[] fixedData, Var2Data varData, Map<Integer, FontBase> fontBases) throws IOException` | [`ViewFactory::create_view`] | Parameter `byte[]` → `&[u8]`; `ProjectFile` → `&Rc<RefCell<ProjectFile>>` (this crate passes the file by shared handle); `Map<Integer, FontBase>` → `&HashMap<i32, FontBase>` (i.e. the type referred to by the `FontBaseMap` alias in `GanttChartView`; `FontBase` is immutable and stored in the map by value) |
//! | Returns the interface type `View` | returns `Rc<RefCell<dyn View>>` | Matches the element type used by this crate's [`crate::view_container::ViewContainer`] to store views, so callers can `add` directly |
//! | `throws IOException` | `MpxjResult<..>` (`Err(MpxjError)`) | Consistent with the error representation used by the rest of the crate's reader chain |

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::mpp::font_base::FontBase;
use crate::mpp::var2_data::Var2Data;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;
use crate::view::View;

/// Factory for creating view objects.
///
/// Corresponds to Java: `interface ViewFactory`
pub trait ViewFactory {
/// Create a view.
    ///
    /// Corresponds to Java: `View createView(ProjectFile file, byte[] fixedMeta, byte[] fixedData,
    /// Var2Data varData, Map<Integer, FontBase> fontBases) throws IOException`
    ///
/// - `file`: the parent MPP file
/// - `fixed_meta`: the view's FixedMeta data
/// - `fixed_data`: the view's FixedData data
/// - `var_data`: the view's Var2Data data
/// - `font_bases`: the font base table (`font index → FontBase`, same as `GanttChartView`'s
/// `FontBaseMap` alias type)
    fn create_view(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        fixed_meta: &[u8],
        fixed_data: &[u8],
        var_data: &Var2Data,
        font_bases: &HashMap<i32, FontBase>,
    ) -> MpxjResult<Rc<RefCell<dyn View>>>;
}
