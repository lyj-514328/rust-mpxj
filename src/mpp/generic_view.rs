//! Corresponds to Java: `org/mpxj/mpp/GenericView.java`
//!
//! "Instantiated" generic views in an MS Project file: view data is only written
//! to the MPP file when the user first views the view; views that have never been viewed have no corresponding information in the file. This class reads the ID, name, and
//! view type from the fixed data block, then retrieves the view's property block via var data and reads the table part name.
//!
//! Skipped Java mechanisms and reasons:
//!
//! | Java | Reason |
//! |---|---|
//! | `InputStream` / `ByteArrayInputStream` | Per this project's convention, Java's stream reading is changed to directly accept `&[u8]` (a byte array slice) |
//! | Reflection / JDBC / XML (JAXB) / GUI / `javax.*` / `Serializable` | Not used by this class |
//!
//! # How inheritance is restored: composition + forwarding
//!
//! | Java | Rust |
//! |---|---|
//! | `GenericView extends AbstractMppView` | [`GenericView`] **holds** an [`AbstractMppView`] and forwards its accessors / setters / `toString()` |
//! | `GenericView9/12/14 extends GenericView` | Each holds a [`GenericView`] (see `generic_view9.rs` etc.) |
//! | `protected abstract Integer getPropertiesID()` | The `properties_id: i32` parameter of [`GenericView::new`] + [`GenericView::get_properties_id`]; Rust has no virtual method dispatch, so the subclass supplies its own value when calling the constructor (equivalent to the virtual call in the Java constructor) |
//! | `public GenericView(ProjectFile parent, byte[] data, Var2Data varData) throws IOException` | [`GenericView::new`] (with one extra `properties_id` parameter), `IOException` → `Err(MpxjError)` |
//!
//! # Differences from Java
//!
//! - Out-of-bounds reads: Java calling `ByteArrayHelper.getInt` / `getShort` on a `data` array that is too short throws
//! `ArrayIndexOutOfBoundsException`; the Rust side returns 0 (with a comment) and does not panic.
//! - Construction of `Props9`: Java throws `IOException`, the Rust side returns `Err`.
//! - `parent` is held the same way as in [`AbstractView`] (weak reference, to avoid an `Rc` cycle).

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::abstract_mpp_view::AbstractMppView;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::props9::Props9;
use crate::mpp::props_key::PropsKey;
use crate::mpp::var2_data::Var2Data;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;
use crate::table::Table;
use crate::view::View;
use crate::view_type::ViewType;

/// Offset of the view type field in the fixed data block.
///
/// Corresponds to Java: `ByteArrayHelper.getShort(data, 112)`
const VIEW_TYPE_OFFSET: usize = 112;

/// Generic view.
///
/// Corresponds to Java: `public abstract class GenericView extends AbstractMppView`
pub struct GenericView {
/// Corresponds to Java: `extends AbstractMppView` (Rust uses composition instead of inheritance)
    view: AbstractMppView,

/// Data type ID of the property block.
    ///
/// Corresponds to Java: the return value of `protected abstract Integer getPropertiesID();`
/// (given as 1 / 6 / 6 by `GenericView9/12/14` respectively).
    properties_id: i32,
}

impl GenericView {
/// Constructor: parses the view properties from the view data block.
    ///
    /// Corresponds to Java: `public GenericView(ProjectFile parent, byte[] data, Var2Data varData) throws IOException`
    ///
/// `properties_id` corresponds to the virtual method `getPropertiesID()` called in the Java constructor;
/// the path where Java throws `IOException` returns `Err` on the Rust side.
    pub fn new(
        parent: &Rc<RefCell<ProjectFile>>,
        data: &[u8],
        var_data: &Var2Data,
        properties_id: i32,
    ) -> MpxjResult<Self> {
        let mut view = AbstractMppView::new(parent);

        // Java: m_id = Integer.valueOf(ByteArrayHelper.getInt(data, 0));
// (Java throws AIOOBE when the data is shorter than 4 bytes; the Rust side returns 0)
        let id = if data.len() >= 4 {
            ByteArrayHelper::get_int(data, 0)
        } else {
            0
        };
        view.set_id(Some(id));

        // Java: m_name = MPPUtility.removeAmpersands(MPPUtility.getUnicodeString(data, 4));
        let name = MPPUtility::get_unicode_string(data, 4);
        view.set_name(MPPUtility::remove_ampersands(Some(name.as_str())));

        // Java: m_type = ViewType.getInstance(ByteArrayHelper.getShort(data, 112));
// (Java throws AIOOBE when the data is shorter than 114 bytes; the Rust side returns 0, i.e. ViewType.UNKNOWN)
        let type_value = if data.len() >= VIEW_TYPE_OFFSET + 2 {
            ByteArrayHelper::get_short(data, VIEW_TYPE_OFFSET)
        } else {
            0
        };
        view.set_type(Some(ViewType::get_instance(type_value)));

        // Java: byte[] propsData = varData.getByteArray(m_id, getPropertiesID());
        if let Some(props_data) = var_data.get_byte_array(Some(id), Some(properties_id)) {
            // Java: Props9 props = new Props9(new ByteArrayInputStream(propsData));
            let props = Props9::new(&props_data)?;

            // Java: byte[] tableName = props.getByteArray(PropsKey.TABLE_NAME);
            if let Some(table_name) = props.get_byte_array(PropsKey::TableName) {
                // Java: m_tableName = MPPUtility.removeAmpersands(MPPUtility.getUnicodeString(tableName, 0));
                let table_name = MPPUtility::get_unicode_string(table_name, 0);
                view.set_table_name(MPPUtility::remove_ampersands(Some(table_name.as_str())));
            }
        }

        Ok(Self {
            view,
            properties_id,
        })
    }

/// Returns the data type ID of the property block.
    ///
    /// Corresponds to Java: `protected abstract Integer getPropertiesID()`
    ///
/// The `Integer` return type in Java never has a `null` value in this project, so the Rust side uses `i32`.
    pub fn get_properties_id(&self) -> i32 {
        self.properties_id
    }

/// Returns the parent project file.
    ///
/// Corresponds to Java: `AbstractView.m_file` (protected field)
    pub fn get_file(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.view.get_file()
    }

/// Sets the view ID.
    ///
/// Corresponds to Java: `AbstractView.m_id` (protected field assignment)
    pub fn set_id(&mut self, id: Option<i32>) {
        self.view.set_id(id);
    }

/// Sets the view name.
    ///
/// Corresponds to Java: `AbstractView.m_name` (protected field assignment)
    pub fn set_name(&mut self, name: Option<String>) {
        self.view.set_name(name);
    }

/// Sets the view type.
    ///
/// Corresponds to Java: `AbstractView.m_type` (protected field assignment)
    pub fn set_type(&mut self, type_: Option<ViewType>) {
        self.view.set_type(type_);
    }

/// Sets the name of the table part in the view.
    ///
/// Corresponds to Java: `AbstractView.m_tableName` (protected field assignment)
    pub fn set_table_name(&mut self, table_name: Option<String>) {
        self.view.set_table_name(table_name);
    }

/// Returns the internally held `AbstractMppView` instance.
    ///
/// Rust-specific: Java's inheritance is expressed by composition on the Rust side.
    pub fn as_abstract_mpp_view(&self) -> &AbstractMppView {
        &self.view
    }

/// Returns the internally held `AbstractMppView` instance (mutable).
    ///
/// Rust-specific: see [`GenericView::as_abstract_mpp_view`].
    pub fn as_abstract_mpp_view_mut(&mut self) -> &mut AbstractMppView {
        &mut self.view
    }
}

impl View for GenericView {
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

impl Display for GenericView {
/// Corresponds to Java: `AbstractView.toString()` (not overridden by this class)
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.view, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpp::var_meta9::VarMeta9;

/// Builds the fixed data block: offset 0 is the ID, offset 4 is the UTF-16LE name, and offset 112 is the view type.
    fn fixed_data(id: i32, name: &str, view_type: i32) -> Vec<u8> {
        let mut data = vec![0u8; 116];
        data[0..4].copy_from_slice(&id.to_le_bytes());
        for (index, unit) in name.encode_utf16().enumerate() {
            let offset = 4 + index * 2;
            data[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
        }
        data[VIEW_TYPE_OFFSET..VIEW_TYPE_OFFSET + 2]
            .copy_from_slice(&(view_type as i16).to_le_bytes());
        data
    }

/// Builds a UTF-16LE string (including the terminating NUL).
    fn utf16(value: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in value.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00]);
        data
    }

/// Builds `Props9` data using the MPP9 property block layout.
    fn props9(entries: &[(i32, Vec<u8>)]) -> Vec<u8> {
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&(entries.len() as i16).to_le_bytes());

        for (key, value) in entries {
            data.extend_from_slice(&(value.len() as i32).to_le_bytes());
            data.extend_from_slice(&key.to_le_bytes());
            data.extend_from_slice(&0i32.to_le_bytes());
            data.extend_from_slice(value);
// Java: align to a two-byte boundary
            if value.len() % 2 != 0 {
                data.push(0);
            }
        }

        data
    }

/// Builds a var data block from a `(id, type) → data item` mapping.
    fn var_data(file: &Rc<RefCell<ProjectFile>>, items: &[(i32, i32, Vec<u8>)]) -> Var2Data {
        let mut data = Vec::new();
        let mut offsets = Vec::new();

        for (_, _, item) in items {
            offsets.push(data.len() as i32);
            data.extend_from_slice(&(item.len() as i32).to_le_bytes());
            data.extend_from_slice(item);
        }

// VarMeta9 block: magic + 3 reserved integers + data area size + an 8-byte index per entry
        let mut meta = Vec::new();
        meta.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&(items.len() as i32).to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&(data.len() as i32).to_le_bytes());

        for (index, (unique_id, type_, _)) in items.iter().enumerate() {
            meta.extend_from_slice(&unique_id.to_le_bytes()[..3]);
            meta.push(*type_ as u8);
            meta.extend_from_slice(&offsets[index].to_le_bytes());
        }

        let meta = VarMeta9::new(&meta).expect("valid metadata");
        Var2Data::new(file, Rc::new(meta), &data)
    }

/// The view ID, name (with `&` stripped), type, and table name are all parsed at the Java offsets.
    #[test]
    fn parses_view_data_and_table_name() {
        let file = ProjectFile::new();
        let data = fixed_data(7, "Gantt Cha&rt", 1);
        let var_data = var_data(
            &file,
            &[(
                7,
                1,
                props9(&[(PropsKey::TableName.get_value(), utf16("&Entry"))]),
            )],
        );

        let view = GenericView::new(&file, &data, &var_data, 1).expect("should parse");

        assert_eq!(view.get_id(), Some(7));
        assert_eq!(view.get_name(), Some("Gantt Chart"));
        assert_eq!(view.get_type(), Some(ViewType::GanttChart));
        assert_eq!(view.get_table_name(), Some("Entry"));
        assert_eq!(view.get_properties_id(), 1);
        assert!(view.get_file().is_some());
    }

/// The table name is `None` when the property block does not exist (the user has never viewed the view).
    #[test]
    fn missing_properties_data_yields_no_table_name() {
        let file = ProjectFile::new();
        let data = fixed_data(2, "Resource Sheet", ViewType::ResourceSheet as i32);
        let var_data = var_data(&file, &[]);

        let view = GenericView::new(&file, &data, &var_data, 6).expect("should parse");

        assert_eq!(view.get_table_name(), None);
        assert_eq!(view.get_type(), Some(ViewType::ResourceSheet));
    }

/// The table name is `None` when the property block exists but has no `TABLE_NAME` key.
    #[test]
    fn properties_without_table_name_yields_none() {
        let file = ProjectFile::new();
        let data = fixed_data(3, "Task Sheet", ViewType::TaskSheet as i32);
        let var_data = var_data(&file, &[(3, 6, props9(&[(PropsKey::Company.get_value(), vec![0x07])]))]);

        let view = GenericView::new(&file, &data, &var_data, 6).expect("should parse");

        assert_eq!(view.get_table_name(), None);
    }

/// Does not panic when the data block is too short: the ID is 0, the type falls back to UNKNOWN, and the name is treated as an empty string.
    #[test]
    fn short_data_does_not_panic() {
        let file = ProjectFile::new();
        let var_data = var_data(&file, &[]);

// Only 2 bytes: neither the ID nor the type field can be read (Java throws AIOOBE)
        let view = GenericView::new(&file, &[0x01, 0x02], &var_data, 1).expect("should parse");
        assert_eq!(view.get_id(), Some(0));
        assert_eq!(view.get_name(), Some(""));
        assert_eq!(view.get_type(), Some(ViewType::Unknown));

// An empty data block likewise does not panic
        let view = GenericView::new(&file, &[], &var_data, 1).expect("should parse");
        assert_eq!(view.get_id(), Some(0));
        assert_eq!(view.get_type(), Some(ViewType::Unknown));

// The type field is out of bounds (length 112, so the short at offset 112 cannot be read)
        let data = vec![0u8; 113];
        let view = GenericView::new(&file, &data, &var_data, 1).expect("should parse");
        assert_eq!(view.get_type(), Some(ViewType::Unknown));
    }

/// A corrupt property block (declared entry count greater than the actual data) returns `Err` and does not panic.
    #[test]
    fn corrupt_properties_data_returns_error() {
        let file = ProjectFile::new();
        let data = fixed_data(5, "View", 1);

        let mut props = vec![0u8; 16];
        props[12..14].copy_from_slice(&2i16.to_le_bytes());
        props.extend_from_slice(&4i32.to_le_bytes());
        props.extend_from_slice(&PropsKey::TableName.get_value().to_le_bytes());

        let var_data = var_data(&file, &[(5, 1, props)]);
        assert!(GenericView::new(&file, &data, &var_data, 1).is_err());
    }

/// `toString()` is provided by the base class (Java does not override it).
    #[test]
    fn display_uses_base_class_format() {
        let file = ProjectFile::new();
        let data = fixed_data(4, "Calendar", ViewType::Calendar as i32);
        let var_data = var_data(&file, &[]);

        let view = GenericView::new(&file, &data, &var_data, 1).expect("should parse");
        assert_eq!(
            view.to_string(),
            "[View id=4 type=CALENDAR name=Calendar]"
        );

        let view = view.as_abstract_mpp_view();
        assert_eq!(view.to_string(), "[View id=4 type=CALENDAR name=Calendar]");
    }
}
