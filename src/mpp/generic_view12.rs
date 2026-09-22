//! Corresponds to Java: `org/mpxj/mpp/GenericView12.java`
//!
//! Generic view for MPP12 files: identical to [`GenericView`], except that the data type ID of the properties block is fixed at 6.
//!
//! Skipped Java mechanisms and the reasons:
//!
//! | Java | Reason |
//! |---|---|
//! | `InputStream` (via the parent class) | Per this project's convention, changed to accept `&[u8]` directly |
//! | Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable` | Not used by this class |
//!
//! # How the inheritance is reproduced: composition + forwarding
//!
//! | Java | Rust |
//! |---|---|
//! | `GenericView12 extends GenericView` | [`GenericView12`] **holds** a [`GenericView`] and forwards accessors / setters / `toString()`; the 5 methods of `View` are forwarded level by level along the inheritance chain |
//! | `public GenericView12(ProjectFile parent, byte[] data, Var2Data varData) throws IOException` | [`GenericView12::new`](`IOException` → `Err(MpxjError)`) |
//! | `@Override protected Integer getPropertiesID()` | the [`GenericView12::PROPERTIES`] constant plus passing it to [`GenericView::new`] at construction; [`GenericView12::get_properties_id`] forwards the parent value |

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use crate::mpp::generic_view::GenericView;
use crate::mpp::var2_data::Var2Data;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;
use crate::table::Table;
use crate::view::View;
use crate::view_type::ViewType;

/// MPP12 generic view.
///
/// Corresponds to Java: `public class GenericView12 extends GenericView`
pub struct GenericView12 {
/// Corresponds to Java: `extends GenericView` (Rust uses composition instead of inheritance)
    view: GenericView,
}

impl GenericView12 {
/// The data type ID of the properties block.
    ///
    /// Corresponds to Java: `private static final Integer PROPERTIES = Integer.valueOf(6);`
    pub const PROPERTIES: i32 = 6;

/// Constructor.
    ///
    /// Corresponds to Java: `public GenericView12(ProjectFile parent, byte[] data, Var2Data varData) throws IOException`
    ///
    /// Java: `super(parent, data, varData);`
    pub fn new(
        parent: &Rc<RefCell<ProjectFile>>,
        data: &[u8],
        var_data: &Var2Data,
    ) -> MpxjResult<Self> {
        Ok(Self {
            view: GenericView::new(parent, data, var_data, Self::PROPERTIES)?,
        })
    }

/// Get the data type ID of the properties block.
    ///
    /// Corresponds to Java: `@Override protected Integer getPropertiesID()`
    pub fn get_properties_id(&self) -> i32 {
        self.view.get_properties_id()
    }

/// Get the parent project file.
    ///
/// Corresponds to Java: `AbstractView.m_file` (protected field)
    pub fn get_file(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.view.get_file()
    }

/// Set the view ID.
    ///
/// Corresponds to Java: `AbstractView.m_id` (protected field assignment)
    pub fn set_id(&mut self, id: Option<i32>) {
        self.view.set_id(id);
    }

/// Set the view name.
    ///
/// Corresponds to Java: `AbstractView.m_name` (protected field assignment)
    pub fn set_name(&mut self, name: Option<String>) {
        self.view.set_name(name);
    }

/// Set the view type.
    ///
/// Corresponds to Java: `AbstractView.m_type` (protected field assignment)
    pub fn set_type(&mut self, type_: Option<ViewType>) {
        self.view.set_type(type_);
    }

/// Set the name of the table part of the view.
    ///
/// Corresponds to Java: `AbstractView.m_tableName` (protected field assignment)
    pub fn set_table_name(&mut self, table_name: Option<String>) {
        self.view.set_table_name(table_name);
    }

/// Get the internally held `GenericView` instance.
    ///
/// Rust specific: Java's inheritance is expressed by composition on the Rust side.
    pub fn as_generic_view(&self) -> &GenericView {
        &self.view
    }

/// Get the internally held `GenericView` instance (mutable).
    ///
/// Rust specific: see [`GenericView12::as_generic_view`].
    pub fn as_generic_view_mut(&mut self) -> &mut GenericView {
        &mut self.view
    }
}

impl View for GenericView12 {
/// Corresponds to Java: `AbstractView.getID()` (inherited)
    fn get_id(&self) -> Option<i32> {
        self.view.get_id()
    }

/// Corresponds to Java: `AbstractView.getName()` (inherited)
    fn get_name(&self) -> Option<&str> {
        self.view.get_name()
    }

/// Corresponds to Java: `AbstractView.getType()` (inherited)
    fn get_type(&self) -> Option<ViewType> {
        self.view.get_type()
    }

/// Corresponds to Java: `AbstractView.getTableName()` (inherited)
    fn get_table_name(&self) -> Option<&str> {
        self.view.get_table_name()
    }

/// Corresponds to Java: `AbstractView.getTable()` (inherited)
    fn get_table(&self) -> Option<Rc<RefCell<Table>>> {
        self.view.get_table()
    }
}

impl Display for GenericView12 {
/// Corresponds to Java: `AbstractView.toString()` (not overridden by this class)
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.view, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpp::var_meta9::VarMeta9;

/// Build a fixed data block (ID + UTF-16LE name + view type at offset 112).
    fn fixed_data(id: i32, name: &str, view_type: i32) -> Vec<u8> {
        let mut data = vec![0u8; 116];
        data[0..4].copy_from_slice(&id.to_le_bytes());
        for (index, unit) in name.encode_utf16().enumerate() {
            let offset = 4 + index * 2;
            data[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
        }
        data[112..114].copy_from_slice(&(view_type as i16).to_le_bytes());
        data
    }

/// Empty var data block.
    fn empty_var_data(file: &Rc<RefCell<ProjectFile>>) -> Var2Data {
        let mut meta = Vec::new();
        meta.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());

        let meta = VarMeta9::new(&meta).expect("valid metadata");
        Var2Data::new(file, Rc::new(meta), &[])
    }

/// The properties block type ID is 6, and view properties are parsed at Java's offsets.
    #[test]
    fn properties_id_is_six() {
        let file = ProjectFile::new();
        let var_data = empty_var_data(&file);
        let view =
            GenericView12::new(&file, &fixed_data(9, "Gantt Chart", 1), &var_data).expect("should parse");

        assert_eq!(GenericView12::PROPERTIES, 6);
        assert_eq!(view.get_properties_id(), 6);
        assert_eq!(view.get_id(), Some(9));
        assert_eq!(view.get_name(), Some("Gantt Chart"));
        assert_eq!(view.get_type(), Some(ViewType::GanttChart));
        assert_eq!(view.get_table_name(), None);
    }

/// Does not panic when the data is too short.
    #[test]
    fn short_data_does_not_panic() {
        let file = ProjectFile::new();
        let var_data = empty_var_data(&file);
        let view = GenericView12::new(&file, &[0x00], &var_data).expect("should parse");

        assert_eq!(view.get_id(), Some(0));
        assert_eq!(view.get_name(), Some(""));
        assert_eq!(view.get_type(), Some(ViewType::Unknown));
    }
}
