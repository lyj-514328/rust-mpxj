//! Corresponds to Java: `org/mpxj/mpp/ViewStateReader12.java`
//!
//! View state reader for MPP12 (and MPP14, which shares this reader) files: the property block comes from the
//! `(1, 6)` data item in var data and is parsed with [`Props12`].
//!
//! Skipped Java mechanisms and the reasons:
//!
//! | Java | Reason |
//! |---|---|
//! | `InputStream` / `ByteArrayInputStream` | per this project's convention, Java stream reading is changed to take `&[u8]` directly |
//! | Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable` | not used by this class |
//!
//! # Approach to restoring inheritance: composition + forwarding
//!
//! | Java | Rust |
//! |---|---|
//! | `ViewStateReader12 extends ViewStateReader` | [`ViewStateReader12`] **holds** a [`ViewStateReader`] and forwards `process` |
//! | `@Override protected Props getProps(Var2Data varData) throws IOException` | implements [`ViewStateReaderMethods::get_props`] (returns `Props`, see the trade-off note in `view_state_reader.rs`) |
//! | `private static final Integer PROPS_ID / PROPS_TYPE` | [`ViewStateReader12::PROPS_ID`] / [`ViewStateReader12::PROPS_TYPE`] |
//! | `throws IOException` | `MpxjResult` (`Err` when `Props12` parsing fails) |

use std::cell::RefCell;
use std::rc::Rc;

use crate::mpp::props::Props;
use crate::mpp::props12::Props12;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::view_state_reader::{ViewStateReader, ViewStateReaderMethods};
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;

/// MPP12 / MPP14 view state reader.
///
/// Corresponds to Java: `final class ViewStateReader12 extends ViewStateReader`
pub struct ViewStateReader12 {
/// Corresponds to Java: `extends ViewStateReader` (Rust uses composition instead of inheritance)
    reader: ViewStateReader,
}

impl ViewStateReader12 {
/// ID of the property block in var data.
    ///
    /// Corresponds to Java: `private static final Integer PROPS_ID = Integer.valueOf(1);`
    pub const PROPS_ID: i32 = 1;

/// Data type of the property block in var data.
    ///
    /// Corresponds to Java: `private static final Integer PROPS_TYPE = Integer.valueOf(6);`
    pub const PROPS_TYPE: i32 = 6;

/// Constructor.
    ///
/// Corresponds to Java: the implicit default constructor (the Java side does not declare a constructor explicitly).
    pub fn new() -> Self {
        Self {
            reader: ViewStateReader::new(),
        }
    }

/// Process the saved view state.
    ///
    /// Corresponds to Java: `ViewStateReader.process(ProjectFile file, Var2Data varData, byte[] fixedData) throws IOException`
    pub fn process(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        var_data: &Var2Data,
        fixed_data: &[u8],
    ) -> MpxjResult<()> {
// Java: the process inherited from ViewStateReader gets the property block from getProps (overridden by this class)
        self.reader.process(self, file, var_data, fixed_data)
    }

/// Get the internally held `ViewStateReader` instance.
    ///
/// Rust-specific: Java's inheritance is expressed via composition on the Rust side.
    pub fn as_view_state_reader(&self) -> &ViewStateReader {
        &self.reader
    }
}

impl Default for ViewStateReader12 {
/// Corresponds to Java: the implicit default constructor, equivalent to [`ViewStateReader12::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl ViewStateReaderMethods for ViewStateReader12 {
/// Parse the `(1, 6)` data item with `Props12`; returns `Ok(None)` when the data item is absent (Java returns `null`).
    ///
    /// Corresponds to Java: `@Override protected Props getProps(Var2Data varData) throws IOException`
    ///
/// Java returns the `Props12` instance itself (later used as the base class `Props`); the Rust side, because the return type needs to own
/// ownership, uses [`Props::copied_from`] to rebuild an equivalent [`Props`] from the parse result.
    fn get_props(&self, var_data: &Var2Data) -> MpxjResult<Option<Props>> {
        // Java: byte[] propsData = varData.getByteArray(PROPS_ID, PROPS_TYPE);
        match var_data.get_byte_array(Some(Self::PROPS_ID), Some(Self::PROPS_TYPE)) {
            // Java: if (propsData != null) { props = new Props12(new ByteArrayInputStream(propsData)); }
            None => Ok(None),
            Some(props_data) => {
                let props12 = Props12::new(&props_data)?;
                Ok(Some(Props::copied_from(&props12)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpp::props_key::PropsKey;
    use crate::mpp::var_meta9::VarMeta9;

/// Build a UTF-16LE string (including the trailing NUL).
    fn utf16(value: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in value.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00]);
        data
    }

/// Build data according to the property block layout (MPP12's `Props12` has the same layout as MPP9's `Props9`).
    fn props(entries: &[(i32, Vec<u8>)]) -> Vec<u8> {
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

/// Build a var data block from a `(id, type) -> data item` mapping.
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

/// Parse the property block from the MPP12 `(1, 6)` data item and write it into the view state.
    #[test]
    fn process_reads_props12_block() {
        let file = ProjectFile::new();
        let task = file.borrow_mut().get_tasks_mut().add();
        task.borrow_mut().set_unique_id(Some(2));

        let props = props(&[
            (PropsKey::ViewName.get_value(), utf16("Resource Sheet")),
            (
                PropsKey::ViewContents.get_value(),
                vec![0x02, 0x00, 0x00, 0x00],
            ),
        ]);
        let var_data = var_data(
            &file,
            &[(ViewStateReader12::PROPS_ID, ViewStateReader12::PROPS_TYPE, props)],
        );

// fixed data block: the filter ID at offset 128 is 0 (the block itself is zero-filled)
        let fixed_data = vec![0u8; 138];

        ViewStateReader12::new()
            .process(&file, &var_data, &fixed_data)
            .expect("should handle");

        let file_ref = file.borrow();
        let state = file_ref.get_views().get_view_state().expect("should write view state");
        assert_eq!(state.get_view_name(), Some("Resource Sheet"));
        assert_eq!(state.get_unique_id_list(), &[2]);
    }

/// Data type 1 (the MPP9 property block) should not be used by this reader.
    #[test]
    fn props9_type_is_ignored() {
        let file = ProjectFile::new();
        let props = props(&[(PropsKey::ViewName.get_value(), utf16("View"))]);
        let var_data = var_data(&file, &[(1, 1, props)]);

        assert!(ViewStateReader12::new()
            .get_props(&var_data)
            .expect("should get properties")
            .is_none());
    }

/// When the property block is absent, `process` leaves the view state unchanged.
    #[test]
    fn missing_props_block_is_ignored() {
        let file = ProjectFile::new();
        let var_data = var_data(&file, &[]);

        ViewStateReader12::new()
            .process(&file, &var_data, &[])
            .expect("should handle");

        assert!(file.borrow().get_views().get_view_state().is_none());
    }

/// Returns `Err` when the property block is corrupt, without panicking.
    #[test]
    fn corrupt_props_block_returns_error() {
        let file = ProjectFile::new();
// declares 2 entries but provides only 1
        let mut props = vec![0u8; 16];
        props[12..14].copy_from_slice(&2i16.to_le_bytes());
        props.extend_from_slice(&4i32.to_le_bytes());
        props.extend_from_slice(&PropsKey::ViewName.get_value().to_le_bytes());

        let var_data = var_data(&file, &[(1, 6, props)]);
        assert!(ViewStateReader12::new().get_props(&var_data).is_err());
    }
}
