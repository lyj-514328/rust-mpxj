//! Corresponds to Java: `org/mpxj/mpp/View8.java`
//!
//! View for MPP8 files: view data is written to the file only when the user first views the view; this class only reads
//! the ID, name and view type from the view data block (MPP8 has no property block, so there is no table name and similar information).
//!
//! Skipped Java mechanisms and reasons:
//!
//! | Java | Reason |
//! |---|---|
//! | Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable` | not used by this class |
//!
//! # How inheritance is reproduced: composition + forwarding
//!
//! | Java | Rust |
//! |---|---|
//! | `View8 extends AbstractMppView` | [`View8`] **holds** an [`AbstractMppView`] and forwards its accessors / setters / `toString()` |
//! | `public View8(ProjectFile parent, byte[] data)` | [`View8::new`] (Java throws no exception; Rust likewise returns `Self`) |
//!
//! # Differences from Java
//!
//! Out-of-bounds reads: calling `ByteArrayHelper.getInt` / `getShort` on `data` with insufficient bytes makes Java throw
//! `ArrayIndexOutOfBoundsException`; Rust returns 0 (and comments), without panicking.

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::abstract_mpp_view::AbstractMppView;
use crate::mpp::mpp_utility::MPPUtility;
use crate::project_file::ProjectFile;
use crate::table::Table;
use crate::view::View;
use crate::view_type::ViewType;

/// Offset of the view type field in the fixed data block (MPP8 and MPP9 differ by 4 bytes).
///
/// Corresponds to Java: `ByteArrayHelper.getShort(data, 116)`
const VIEW_TYPE_OFFSET: usize = 116;

/// MPP8 view.
///
/// Corresponds to Java: `public final class View8 extends AbstractMppView`
pub struct View8 {
/// Corresponds to Java: `extends AbstractMppView` (Rust uses composition instead of inheritance)
    view: AbstractMppView,
}

impl View8 {
/// Constructor: extracts view data from the view data block.
    ///
    /// Corresponds to Java: `public View8(ProjectFile parent, byte[] data)`
    pub fn new(parent: &Rc<RefCell<ProjectFile>>, data: &[u8]) -> Self {
        let mut view = AbstractMppView::new(parent);

        // Java: m_id = Integer.valueOf(ByteArrayHelper.getInt(data, 0));
// (Java throws AIOOBE when there are fewer than 4 bytes; Rust takes 0)
        let id = if data.len() >= 4 {
            ByteArrayHelper::get_int(data, 0)
        } else {
            0
        };
        view.set_id(Some(id));

        // Java: m_name = MPPUtility.removeAmpersands(MPPUtility.getUnicodeString(data, 4));
        let name = MPPUtility::get_unicode_string(data, 4);
        view.set_name(MPPUtility::remove_ampersands(Some(name.as_str())));

        // Java: m_type = ViewType.getInstance(ByteArrayHelper.getShort(data, 116));
// (Java throws AIOOBE when there are fewer than 118 bytes; Rust takes 0, i.e. ViewType.UNKNOWN)
        let type_value = if data.len() >= VIEW_TYPE_OFFSET + 2 {
            ByteArrayHelper::get_short(data, VIEW_TYPE_OFFSET)
        } else {
            0
        };
        view.set_type(Some(ViewType::get_instance(type_value)));

        Self { view }
    }

/// Gets the parent project file.
    ///
/// Corresponds to Java: `AbstractView.m_file` (protected field)
    pub fn get_file(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.view.get_file()
    }

/// Gets the internally held `AbstractMppView` instance.
    ///
/// Rust-specific: Java inheritance is expressed by composition on the Rust side.
    pub fn as_abstract_mpp_view(&self) -> &AbstractMppView {
        &self.view
    }
}

impl View for View8 {
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

/// Corresponds to Java: `AbstractView.getTableName()` (inherited; always `None` for MPP8)
    fn get_table_name(&self) -> Option<&str> {
        self.view.get_table_name()
    }

/// Corresponds to Java: `AbstractView.getTable()` (inherited; always `None` for MPP8)
    fn get_table(&self) -> Option<Rc<RefCell<Table>>> {
        self.view.get_table()
    }
}

impl Display for View8 {
/// Corresponds to Java: `AbstractView.toString()` (not overridden by this class)
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.view, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// Constructs the MPP8 view data block (ID + UTF-16LE name + view type at offset 116).
    fn view_data(id: i32, name: &str, view_type: i32) -> Vec<u8> {
        let mut data = vec![0u8; 120];
        data[0..4].copy_from_slice(&id.to_le_bytes());
        for (index, unit) in name.encode_utf16().enumerate() {
            let offset = 4 + index * 2;
            data[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
        }
        data[VIEW_TYPE_OFFSET..VIEW_TYPE_OFFSET + 2]
            .copy_from_slice(&(view_type as i16).to_le_bytes());
        data
    }

/// The ID, name and type are parsed using the Java offsets.
    #[test]
    fn parses_view_data() {
        let file = ProjectFile::new();
        let view = View8::new(&file, &view_data(5, "&Gantt Chart", 1));

        assert_eq!(view.get_id(), Some(5));
        assert_eq!(view.get_name(), Some("Gantt Chart"));
        assert_eq!(view.get_type(), Some(ViewType::GanttChart));
        assert_eq!(view.get_table_name(), None);
        assert!(view.get_table().is_none());
        assert!(view.get_file().is_some());
        assert_eq!(
            view.to_string(),
            "[View id=5 type=GANTT_CHART name=Gantt Chart]"
        );
    }

/// Unknown type values fall back to `UNKNOWN`.
    #[test]
    fn unknown_type_falls_back() {
        let file = ProjectFile::new();
        let view = View8::new(&file, &view_data(1, "View", 99));

        assert_eq!(view.get_type(), Some(ViewType::Unknown));
    }

/// Does not panic when the data block is too short (places where Java throws AIOOBE take default values on the Rust side).
    #[test]
    fn short_data_does_not_panic() {
        let file = ProjectFile::new();

        let view = View8::new(&file, &[0x01]);
        assert_eq!(view.get_id(), Some(0));
        assert_eq!(view.get_name(), Some(""));
        assert_eq!(view.get_type(), Some(ViewType::Unknown));

        let view = View8::new(&file, &[]);
        assert_eq!(view.get_id(), Some(0));
        assert_eq!(view.get_type(), Some(ViewType::Unknown));

// the name is long enough but the type field is exactly out of bounds (length 117)
        let mut data = view_data(2, "X", 1);
        data.truncate(VIEW_TYPE_OFFSET + 1);
        let view = View8::new(&file, &data);
        assert_eq!(view.get_id(), Some(2));
        assert_eq!(view.get_type(), Some(ViewType::Unknown));
    }
}
