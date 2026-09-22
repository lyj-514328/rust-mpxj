//! Corresponds to Java: `org/mpxj/mpp/ViewStateReader9.java`
//!
//! View state reader for MPP9 files: the property block comes from the `(1, 1)` var data item and is parsed with [`Props9`].
//!
//! Skipped Java mechanisms and reasons:
//!
//! | Java | Reason |
//! |---|---|
//! | `InputStream` / `ByteArrayInputStream` | per this project's convention, Java's stream reading is changed to receiving `&[u8]` directly |
//! | Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable` | not used by this class |
//!
//! # How inheritance is reproduced: composition + forwarding
//!
//! | Java | Rust |
//! |---|---|
//! | `ViewStateReader9 extends ViewStateReader` | [`ViewStateReader9`] **holds** a [`ViewStateReader`] and forwards `process` |
//! | `@Override protected Props getProps(Var2Data varData) throws IOException` | implements [`ViewStateReaderMethods::get_props`] (returns `Props`; see the trade-off note in `view_state_reader.rs`) |
//! | `private static final Integer PROPS_ID / PROPS_TYPE` | [`ViewStateReader9::PROPS_ID`] / [`ViewStateReader9::PROPS_TYPE`] |
//! | `throws IOException` | `MpxjResult` (`Err` when `Props9` parsing fails) |

use std::cell::RefCell;
use std::rc::Rc;

use crate::mpp::props::Props;
use crate::mpp::props9::Props9;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::view_state_reader::{ViewStateReader, ViewStateReaderMethods};
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;

/// MPP9 view state reader.
///
/// Corresponds to Java: `final class ViewStateReader9 extends ViewStateReader`
pub struct ViewStateReader9 {
/// Corresponds to Java: `extends ViewStateReader` (Rust uses composition instead of inheritance)
    reader: ViewStateReader,
}

impl ViewStateReader9 {
/// ID of the property block in var data.
    ///
    /// Corresponds to Java: `private static final Integer PROPS_ID = Integer.valueOf(1);`
    pub const PROPS_ID: i32 = 1;

/// Data type of the property block in var data.
    ///
    /// Corresponds to Java: `private static final Integer PROPS_TYPE = Integer.valueOf(1);`
    pub const PROPS_TYPE: i32 = 1;

/// Constructor.
    ///
/// Corresponds to Java: implicit default constructor (Java does not explicitly declare a constructor).
    pub fn new() -> Self {
        Self {
            reader: ViewStateReader::new(),
        }
    }

/// Processes the saved view state.
    ///
    /// Corresponds to Java: `ViewStateReader.process(ProjectFile file, Var2Data varData, byte[] fixedData) throws IOException`
    pub fn process(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        var_data: &Var2Data,
        fixed_data: &[u8],
    ) -> MpxjResult<()> {
// Java: the inherited ViewStateReader process gets its property block from getProps (overridden by this class)
        self.reader.process(self, file, var_data, fixed_data)
    }

/// Gets the internally held `ViewStateReader` instance.
    ///
/// Rust-specific: Java inheritance is expressed by composition on the Rust side.
    pub fn as_view_state_reader(&self) -> &ViewStateReader {
        &self.reader
    }
}

impl Default for ViewStateReader9 {
/// Corresponds to Java: implicit default constructor, equivalent to [`ViewStateReader9::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl ViewStateReaderMethods for ViewStateReader9 {
/// Parses the `(1, 1)` data item with `Props9`; returns `Ok(None)` when the data item is absent (Java returns `null`).
    ///
    /// Corresponds to Java: `@Override protected Props getProps(Var2Data varData) throws IOException`
    ///
/// Java returns the `Props9` instance itself (later used as the base class `Props`); on the Rust side, because the return type needs to own
/// ownership, [`Props::copied_from`] rebuilds an equivalent [`Props`] from the parsed result.
    fn get_props(&self, var_data: &Var2Data) -> MpxjResult<Option<Props>> {
        // Java: byte[] propsData = varData.getByteArray(PROPS_ID, PROPS_TYPE);
        match var_data.get_byte_array(Some(Self::PROPS_ID), Some(Self::PROPS_TYPE)) {
            // Java: if (propsData != null) { props = new Props9(new ByteArrayInputStream(propsData)); }
            None => Ok(None),
            Some(props_data) => {
                let props9 = Props9::new(&props_data)?;
                Ok(Some(Props::copied_from(&props9)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpp::props_key::PropsKey;
    use crate::mpp::var_meta9::VarMeta9;

/// Constructs a UTF-16LE string (including the trailing NUL).
    fn utf16(value: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in value.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00]);
        data
    }

/// Constructs `Props9` data according to the MPP9 property block layout.
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

/// Constructs a var data block from a `(id, type) → data item` map.
    fn var_data(file: &Rc<RefCell<ProjectFile>>, items: &[(i32, i32, Vec<u8>)]) -> Var2Data {
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

/// Parses the property block from the MPP9 `(1, 1)` data item and writes it to the view state.
    #[test]
    fn process_reads_props9_block() {
        let file = ProjectFile::new();
        let task = file.borrow_mut().get_tasks_mut().add();
        task.borrow_mut().set_unique_id(Some(1));

        let props = props9(&[
            (PropsKey::ViewName.get_value(), utf16("&Gantt Chart")),
            (PropsKey::ViewContents.get_value(), 1i32.to_le_bytes().to_vec()),
        ]);
        let var_data = var_data(&file, &[(ViewStateReader9::PROPS_ID, ViewStateReader9::PROPS_TYPE, props)]);

// Fixed data block: the filter ID at offset 128 is 0 (the block itself is zero-filled)
        let fixed_data = vec![0u8; 138];

        ViewStateReader9::new()
            .process(&file, &var_data, &fixed_data)
            .expect("should handle");

        let file_ref = file.borrow();
        let state = file_ref.get_views().get_view_state().expect("should write view state");
        assert_eq!(state.get_view_name(), Some("Gantt Chart"));
        assert_eq!(state.get_unique_id_list(), &[1]);
    }

/// When the data item is absent (the `(1, 1)` item is missing), `getProps` returns `None` and `process` leaves the view state unchanged.
    #[test]
    fn missing_props_block_is_ignored() {
        let file = ProjectFile::new();
        let var_data = var_data(&file, &[]);

        let reader = ViewStateReader9::new();
        assert!(reader.get_props(&var_data).expect("should get properties").is_none());

        reader.process(&file, &var_data, &[]).expect("should handle");
        assert!(file.borrow().get_views().get_view_state().is_none());
    }

/// A data type mismatch is likewise treated as absent.
    #[test]
    fn wrong_props_type_is_ignored() {
        let file = ProjectFile::new();
        let props = props9(&[(PropsKey::ViewName.get_value(), utf16("View"))]);
// type 6 rather than 1
        let var_data = var_data(&file, &[(1, 6, props)]);

        assert!(ViewStateReader9::new()
            .get_props(&var_data)
            .expect("should get properties")
            .is_none());
    }

/// Returns `Err` when the property block is corrupt (declared entry count exceeds actual data) and does not panic.
    #[test]
    fn corrupt_props_block_returns_error() {
        let file = ProjectFile::new();
        let mut props = vec![0u8; 16];
        props[12..14].copy_from_slice(&2i16.to_le_bytes());
        props.extend_from_slice(&4i32.to_le_bytes());
        props.extend_from_slice(&PropsKey::ViewName.get_value().to_le_bytes());

        let var_data = var_data(&file, &[(1, 1, props)]);
        assert!(ViewStateReader9::new().get_props(&var_data).is_err());
    }
}
