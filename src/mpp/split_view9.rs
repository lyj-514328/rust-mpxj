//! Corresponds to Java: `org/mpxj/mpp/SplitView9.java`
//!
//! The "user-defined combined view" in MS Project: formed by stacking two existing views vertically (split screen). This class,
//! building on [`GenericView9`], additionally reads the names of the upper and lower views from the properties block.
//!
//! Skipped Java mechanisms and reasons:
//!
//! | Java | Reason |
//! |---|---|
//! | `InputStream` / `ByteArrayInputStream` | Per this project's convention, Java's stream reading is changed to directly accepting `&[u8]` |
//! | Reflection / JDBC / XML (JAXB) / GUI / `javax.*` / `Serializable` | Not used by this class |
//!
//! # How inheritance is reproduced: composition + forwarding
//!
//! | Java | Rust |
//! |---|---|
//! | `SplitView9 extends GenericView9` | [`SplitView9`] **holds** a [`GenericView9`] and forwards behavior other than the accessors / setters / `toString()` |
//! | `SplitView9(ProjectFile parent, byte[] fixedData, Var2Data varData) throws IOException` (package-private) | [`SplitView9::new`] (`IOException` -> `Err(MpxjError)`; Rust has no package visibility, so it is `pub`) |
//! | `@Override public String toString()` | `impl Display` (Java overrides `toString()`, and Rust copies its format) |

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use crate::mpp::generic_view9::GenericView9;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::props9::Props9;
use crate::mpp::props_key::PropsKey;
use crate::mpp::var2_data::Var2Data;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;
use crate::table::Table;
use crate::view::View;
use crate::view_type::ViewType;

/// Split-screen view (a combination of an upper and a lower view).
///
/// Corresponds to Java: `public class SplitView9 extends GenericView9`
pub struct SplitView9 {
/// Corresponds to Java: `extends GenericView9` (Rust uses composition instead of inheritance)
    view: GenericView9,

/// Name of the upper view.
    ///
    /// Corresponds to Java field: `private String m_upperViewName;`
    m_upper_view_name: Option<String>,

/// Name of the lower view.
    ///
    /// Corresponds to Java field: `private String m_lowerViewName;`
    m_lower_view_name: Option<String>,
}

impl SplitView9 {
/// Data type ID of the properties block.
    ///
    /// Corresponds to Java: `private static final Integer PROPERTIES = Integer.valueOf(1);`
    pub const PROPERTIES: i32 = 1;

/// Constructor.
    ///
    /// Corresponds to Java: `SplitView9(ProjectFile parent, byte[] fixedData, Var2Data varData) throws IOException`
    pub fn new(
        parent: &Rc<RefCell<ProjectFile>>,
        fixed_data: &[u8],
        var_data: &Var2Data,
    ) -> MpxjResult<Self> {
        // Java: super(parent, fixedData, varData);
        let view = GenericView9::new(parent, fixed_data, var_data)?;
        let mut result = Self {
            view,
            m_upper_view_name: None,
            m_lower_view_name: None,
        };

        // Java: byte[] propsData = varData.getByteArray(m_id, PROPERTIES);
        if let Some(props_data) =
            var_data.get_byte_array(result.view.get_id(), Some(Self::PROPERTIES))
        {
            // Java: Props9 props = new Props9(new ByteArrayInputStream(propsData));
            let props = Props9::new(&props_data)?;

            // Java: byte[] upperViewName = props.getByteArray(PropsKey.UPPER_VIEW_NAME);
            if let Some(upper_view_name) = props.get_byte_array(PropsKey::UpperViewName) {
                // Java: m_upperViewName = MPPUtility.removeAmpersands(MPPUtility.getUnicodeString(upperViewName, 0));
                let name = MPPUtility::get_unicode_string(upper_view_name, 0);
                result.m_upper_view_name = MPPUtility::remove_ampersands(Some(name.as_str()));
            }

            // Java: byte[] lowerViewName = props.getByteArray(PropsKey.LOWER_VIEW_NAME);
            if let Some(lower_view_name) = props.get_byte_array(PropsKey::LowerViewName) {
                // Java: m_lowerViewName = MPPUtility.removeAmpersands(MPPUtility.getUnicodeString(lowerViewName, 0));
                let name = MPPUtility::get_unicode_string(lower_view_name, 0);
                result.m_lower_view_name = MPPUtility::remove_ampersands(Some(name.as_str()));
            }
        }

        Ok(result)
    }

/// Get the name of the lower view.
    ///
    /// Corresponds to Java: `public String getLowerViewName()`
    pub fn get_lower_view_name(&self) -> Option<&str> {
        self.m_lower_view_name.as_deref()
    }

/// Get the name of the upper view.
    ///
    /// Corresponds to Java: `public String getUpperViewName()`
    pub fn get_upper_view_name(&self) -> Option<&str> {
        self.m_upper_view_name.as_deref()
    }

/// Get the data type ID of the properties block.
    ///
/// Corresponds to Java: `GenericView9.getPropertiesID()` (inherited)
    pub fn get_properties_id(&self) -> i32 {
        self.view.get_properties_id()
    }

/// Get the parent project file.
    ///
/// Corresponds to Java: `AbstractView.m_file` (protected field)
    pub fn get_file(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.view.get_file()
    }

/// Get the internally held `GenericView9` instance.
    ///
/// Rust-specific: Java's inheritance is expressed on the Rust side by composition.
    pub fn as_generic_view9(&self) -> &GenericView9 {
        &self.view
    }
}

impl View for SplitView9 {
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

impl Display for SplitView9 {
    /// Corresponds to Java: `@Override public String toString()`
    ///
    /// Java: `"[SplitView9 upperViewName=" + m_upperViewName + " lowerViewName=" + m_lowerViewName + "]"`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[SplitView9 upperViewName={} lowerViewName={}]",
            self.m_upper_view_name.as_deref().unwrap_or("null"),
            self.m_lower_view_name.as_deref().unwrap_or("null")
        )
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

/// Build a UTF-16LE string (including the trailing NUL).
    fn utf16(value: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in value.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00]);
        data
    }

/// Build `Props9` data using the MPP9 properties block layout.
    fn props9(entries: &[(i32, Vec<u8>)]) -> Vec<u8> {
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&(entries.len() as i16).to_le_bytes());

        for (key, value) in entries {
            data.extend_from_slice(&(value.len() as i32).to_le_bytes());
            data.extend_from_slice(&key.to_le_bytes());
            data.extend_from_slice(&0i32.to_le_bytes());
            data.extend_from_slice(value);
            if value.len() % 2 != 0 {
                data.push(0);
            }
        }

        data
    }

/// Build a var data block from an `(id, type) -> data item` mapping.
    fn var_data_block(file: &Rc<RefCell<ProjectFile>>, items: &[(i32, i32, Vec<u8>)]) -> Var2Data {
        let mut data = Vec::new();
        let mut offsets = Vec::new();

        for (_, _, item) in items {
            offsets.push(data.len() as i32);
            data.extend_from_slice(&(item.len() as i32).to_le_bytes());
            data.extend_from_slice(item);
        }

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

/// The upper and lower view names are resolved using Java's keys, stripping `&`.
    #[test]
    fn parses_upper_and_lower_view_names() {
        let file = ProjectFile::new();
        let data = fixed_data(1, "Split View", 1);
        let props = props9(&[
            (PropsKey::UpperViewName.get_value(), utf16("&Gantt Chart")),
            (PropsKey::LowerViewName.get_value(), utf16("&Task Form")),
        ]);
        let var_data = var_data_block(&file, &[(1, 1, props)]);

        let view = SplitView9::new(&file, &data, &var_data).expect("should parse");

        assert_eq!(view.get_upper_view_name(), Some("Gantt Chart"));
        assert_eq!(view.get_lower_view_name(), Some("Task Form"));
// Note: `PropsKey::TableName` and `PropsKey::UpperViewName` are the same key value
// (574619658); the properties block is a "key value -> byte array" table, so both read the same data.
        assert_eq!(view.get_table_name(), Some("Gantt Chart"));
        assert_eq!(view.get_id(), Some(1));
        assert_eq!(
            view.to_string(),
            "[SplitView9 upperViewName=Gantt Chart lowerViewName=Task Form]"
        );
    }

/// When the properties block is missing, both view names are `None` (Java's `null`).
    #[test]
    fn missing_properties_yields_none() {
        let file = ProjectFile::new();
        let data = fixed_data(2, "Split View", 1);
        let var_data = var_data_block(&file, &[]);

        let view = SplitView9::new(&file, &data, &var_data).expect("should parse");

        assert_eq!(view.get_upper_view_name(), None);
        assert_eq!(view.get_lower_view_name(), None);
        assert_eq!(
            view.to_string(),
            "[SplitView9 upperViewName=null lowerViewName=null]"
        );
    }

/// When the properties block has only one of the keys, the other is `None`.
    #[test]
    fn partial_properties_are_supported() {
        let file = ProjectFile::new();
        let data = fixed_data(3, "Split View", 1);
        let props = props9(&[(PropsKey::LowerViewName.get_value(), utf16("Resource Sheet"))]);
        let var_data = var_data_block(&file, &[(3, 1, props)]);

        let view = SplitView9::new(&file, &data, &var_data).expect("should parse");

        assert_eq!(view.get_upper_view_name(), None);
        assert_eq!(view.get_lower_view_name(), Some("Resource Sheet"));
    }

/// No panic when the data is too short / the properties block is corrupted (a corrupted props returns `Err`).
    #[test]
    fn short_data_and_corrupt_props_do_not_panic() {
        let file = ProjectFile::new();
        let empty_var_data = var_data_block(&file, &[]);
        let view = SplitView9::new(&file, &[], &empty_var_data).expect("should parse");
        assert_eq!(view.get_id(), Some(0));
        assert_eq!(view.get_upper_view_name(), None);

// Declares 2 entries but has only 1 -> Props9 parsing fails
        let mut props = vec![0u8; 16];
        props[12..14].copy_from_slice(&2i16.to_le_bytes());
        props.extend_from_slice(&4i32.to_le_bytes());
        props.extend_from_slice(&PropsKey::UpperViewName.get_value().to_le_bytes());
        let var_data = var_data_block(&file, &[(1, 1, props)]);
        assert!(SplitView9::new(&file, &fixed_data(1, "Split", 1), &var_data).is_err());
    }
}
