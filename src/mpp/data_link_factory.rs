//! Corresponds to Java: `org/mpxj/mpp/DataLinkFactory.java`
//!
//! Extract "data links" (`DataLink`) from the fixed-length / variable-length data blocks of an MPP file: each link describes two fields
//! and their correspondence, possibly within the same project or across projects.
//!
//! Each fixed-length data block record corresponds to one link record, where the integer at offset 0 is the unique ID of the variable-length data block,
//! and the 0x20 bit at offset 112 indicates whether the record is a "link properties" block (if set, skip it); the variable-length data block
//! stores a `Props` entry (key 1 for MPP9, key 6 for other versions), which gives the link path, field type,
//! and row unique ID.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `DataLinkFactory(ProjectFile project, FixedData fixedData, Var2Data varData)` | [`DataLinkFactory::new`] takes **ownership** of the three parameters | Java only stores references (`m_fixedData` / `m_varData`); Rust's `FixedData` / `Var2Data` have no shared handle, while `ProjectFile` uses `Rc<RefCell<..>>` |
//! | `process() throws IOException` | [`DataLinkFactory::process`] returns `MpxjResult<()>` | Java's `IOException` comes from the `Props14` constructor; Rust uses [`MpxjError`] uniformly |
//! | `Map<String, DataLink> map = new HashMap<>()` | `BTreeMap<Option<String>, Rc<RefCell<DataLink>>>` | Java's `HashMap` iteration order is unspecified; Rust uses `BTreeMap` to make the order stable and keeps `null` distinct from the empty string as keys |
//! | `new DataLink(dataLinkID)` (`dataLinkID` may be `null`) | `DataLink::new_nullable(id)` | Preserves `null` as Java does |
//! | `m_project.getDataLinks().addAll(...)` | `DataLinkContainer::add` one by one | Rust's container has no `addAll` |
//! | `data[112]` (Java throws `ArrayIndexOutOfBoundsException` when the array is shorter than 113 bytes) | Skip the record when the length is insufficient | No panic |
//!
//! Skipped Java members: none (`process()` / the private `process(byte[], Map)` are both translated one by one,
//! and the private method is renamed to [`DataLinkFactory::process_data_link`] to avoid clashing with the public `process`).
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::field_type_helper::FieldTypeHelper;
use crate::data_link::DataLink;
use crate::mpp::fixed_data::FixedData;
use crate::mpp::props14::Props14;
use crate::mpp::props_key::PropsKey;
use crate::mpp::var2_data::Var2Data;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;

/// Variable-length data key used by MPP9.
///
/// Corresponds to Java: `private static final Integer PROPS9 = Integer.valueOf(1)`
const PROPS9: i32 = 1;

/// Variable-length data key used by other versions.
///
/// Corresponds to Java: `private static final Integer PROPS = Integer.valueOf(6)`
const PROPS: i32 = 6;

/// Byte offset of the "link properties" flag in the fixed-length data block.
///
/// Corresponds to Java: `data[112]`
const LINK_PROPERTIES_OFFSET: usize = 112;

/// The "link properties" flag bit in the fixed-length data block.
///
/// Corresponds to Java: `(data[112] & 0x20) == 0`
const LINK_PROPERTIES_MASK: u8 = 0x20;

/// Data link extraction.
///
/// Corresponds to Java: `class DataLinkFactory`
pub struct DataLinkFactory {
    /// Corresponds to Java: `private final ProjectFile m_project`
    project: Rc<RefCell<ProjectFile>>,
    /// Corresponds to Java: `private final FixedData m_fixedData`
    fixed_data: FixedData,
    /// Corresponds to Java: `private final Var2Data m_varData`
    var_data: Var2Data,
}

impl DataLinkFactory {
/// Constructor.
    ///
    /// Corresponds to Java: `public DataLinkFactory(ProjectFile project, FixedData fixedData, Var2Data varData)`
    pub fn new(project: &Rc<RefCell<ProjectFile>>, fixed_data: FixedData, var_data: Var2Data) -> Self {
        Self {
            project: Rc::clone(project),
            fixed_data,
            var_data,
        }
    }

/// Extract all data links.
    ///
    /// Corresponds to Java: `public void process() throws IOException`
    pub fn process(&self) -> MpxjResult<()> {
        // Java: Integer key = m_project.getProjectProperties().getMppFileType().intValue() == 9 ? PROPS9 : PROPS;
        let key = if self.project.borrow().get_project_properties().get_mpp_file_type() == Some(9) {
            PROPS9
        } else {
            PROPS
        };

        // Java: Map<String, DataLink> map = new HashMap<>();
        let mut map: BTreeMap<Option<String>, Rc<RefCell<DataLink>>> = BTreeMap::new();

        // Java: int itemCount = m_fixedData.getItemCount();
        let item_count = self.fixed_data.get_item_count();
        for index in 0..item_count {
            // Java: byte[] data = m_fixedData.getByteArrayValue(index);
            let Some(data) = self.fixed_data.get_byte_array_value(index) else {
                continue;
            };

            // Java: if (data != null && (data[112] & 0x20) == 0)
// When the data is shorter than 113 bytes Java throws ArrayIndexOutOfBoundsException; Rust skips it.
            if data.len() <= LINK_PROPERTIES_OFFSET {
                continue;
            }
            if (data[LINK_PROPERTIES_OFFSET] & LINK_PROPERTIES_MASK) != 0 {
                continue;
            }

            // Java: int id = ByteArrayHelper.getInt(data, 0);
            //       byte[] propsData = m_varData.getByteArray(Integer.valueOf(id), key);
            let id = ByteArrayHelper::get_int(data, 0);
            let Some(props_data) = self.var_data.get_byte_array(Some(id), Some(key)) else {
                continue;
            };

            // Java: process(propsData, map);
            self.process_data_link(&props_data, &mut map)?;
        }

        // Java: DataLinkContainer container = m_project.getDataLinks(); container.addAll(map.values());
        let mut file = self.project.borrow_mut();
        let container = file.get_data_links_mut();
        for link in map.into_values() {
            container.add(link);
        }

        Ok(())
    }

/// Extract a single data link.
    ///
    /// Corresponds to Java: `private void process(byte[] data, Map<String, DataLink> map) throws IOException`
    ///
/// Rust has no overloading, so it is renamed to `process_data_link`.
    fn process_data_link(
        &self,
        data: &[u8],
        map: &mut BTreeMap<Option<String>, Rc<RefCell<DataLink>>>,
    ) -> MpxjResult<()> {
        // Java: Props props = new Props14(m_project, new ByteArrayInputStream(data));
        let props = Props14::new(&self.project, data)?;

        // Java: String dataLinkID = props.getUnicodeString(PropsKey.DATA_LINK_PATH);
        //       DataLink dataLink = map.computeIfAbsent(dataLinkID, DataLink::new);
        let data_link_id = props.get_unicode_string(PropsKey::DataLinkPath);
        let data_link = Rc::clone(map.entry(data_link_id.clone()).or_insert_with(|| {
            Rc::new(RefCell::new(DataLink::new_nullable(data_link_id.clone())))
        }));

        // Java: Integer rowUniqueID = Integer.valueOf(props.getInt(PropsKey.DATA_LINK_UNIQUE_ID));
        let row_unique_id = Some(props.get_int(PropsKey::DataLinkUniqueId));
        // Java: FieldType fieldType = FieldTypeHelper.getInstance(m_project, props.getInt(PropsKey.DATA_LINK_FIELD_TYPE));
        let field_type = FieldTypeHelper::get_instance(
            &self.project,
            props.get_int(PropsKey::DataLinkFieldType),
        )
        .map(|field_type| field_type.to_field_key());

        // Java:
        // if (props.getUnicodeString(PropsKey.VIEW_NAME) == null)
        // { dataLink.setTargetField(fieldType); dataLink.setTargetUniqueID(rowUniqueID); }
        // else
        // { dataLink.setSourceField(fieldType); dataLink.setSourceUniqueID(rowUniqueID); }
        let mut data_link = data_link.borrow_mut();
        if props.get_unicode_string(PropsKey::ViewName).is_none() {
            data_link.set_target_field(field_type);
            data_link.set_target_unique_id(row_unique_id);
        } else {
            data_link.set_source_field(field_type);
            data_link.set_source_unique_id(row_unique_id);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::mpp_task_field::MPPTaskField;
    use crate::field_key::FieldKey;
    use crate::mpp::fixed_meta::FixedMeta;
    use crate::mpp::var_meta::VarMeta;
    use crate::mpp::var_meta9::VarMeta9;
    use crate::task_field::TaskField;

/// Props data key in the variable-length data block (non-MPP9 files).
    const VAR_DATA_KEY: i32 = PROPS;

/// Entry unique ID in the variable-length data block (same as the value at offset 0 of the fixed-length data block).
    const VAR_DATA_ID: i32 = 5;

/// Length of the fixed-length data block (must exceed 112 bytes and be covered by the 0x20 flag check).
    const FIXED_DATA_SIZE: usize = 200;

/// Manually build a `Props14` payload (16-byte block header + several "length + key + reserved + data" records).
    fn props14_data(entries: &[(i32, Vec<u8>)]) -> Vec<u8> {
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&(entries.len() as u16).to_le_bytes());

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

/// UTF-16LE (NUL-terminated) encoding.
    fn utf16(value: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in value.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00]);
        data
    }

/// Build a VarMeta9 block (structure matches the tests in `crate::mpp::var2_data`).
    fn var_meta9_bytes(entries: &[(i32, i32, usize)], offsets: &[i32], data_size: i32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(entries.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&data_size.to_le_bytes());

        for (unique_id, type_, index) in entries {
            data.extend_from_slice(&unique_id.to_le_bytes()[..3]);
            data.push(*type_ as u8);
            data.extend_from_slice(&offsets[*index].to_le_bytes());
        }

        data
    }

/// Build a fixed-length data block (offset 0 holds the variable-length data unique ID, and the flag at offset 112 is set by the parameter).
    fn fixed_data(link_properties_flag: bool) -> FixedData {
        let mut data = vec![0u8; FIXED_DATA_SIZE];
        data[0..4].copy_from_slice(&VAR_DATA_ID.to_le_bytes());
        if link_properties_flag {
            data[LINK_PROPERTIES_OFFSET] = 0x20;
        }
        FixedData::with_fixed_size(FIXED_DATA_SIZE as i32, &data)
    }

/// Build a variable-length data block containing only one Props entry.
    ///
/// When `entries` is empty, **no** entry is registered in VarMeta, used to exercise the "no matching entry in the variable-length data block" branch.
    fn var_data(file: &Rc<RefCell<ProjectFile>>, entries: &[(i32, Vec<u8>)]) -> Var2Data {
        let payload = props14_data(entries);

        let mut data = Vec::new();
        data.extend_from_slice(&(payload.len() as i32).to_le_bytes());
        data.extend_from_slice(&payload);

        let meta_entries: Vec<(i32, i32, usize)> = if entries.is_empty() {
            Vec::new()
        } else {
            vec![(VAR_DATA_ID, VAR_DATA_KEY, 0)]
        };
        let meta_bytes = var_meta9_bytes(&meta_entries, &[0], data.len() as i32);
        let meta: Rc<dyn VarMeta> = Rc::new(VarMeta9::new(&meta_bytes).expect("valid metadata"));

        Var2Data::new(file, meta, &data)
    }

/// A set of Props records that form a "target link pointing at a task field".
    fn target_link_entries() -> Vec<(i32, Vec<u8>)> {
        vec![
            (
                PropsKey::DataLinkPath.get_value(),
                utf16("/path/to/file.mpp"),
            ),
            (
                PropsKey::DataLinkUniqueId.get_value(),
                42i32.to_le_bytes().to_vec(),
            ),
            (
                PropsKey::DataLinkFieldType.get_value(),
                (MPPTaskField::TASK_FIELD_BASE | 86).to_le_bytes().to_vec(),
            ),
        ]
    }

/// Target link: writes the target field when there is no VIEW_NAME.
    #[test]
    fn extracts_a_target_data_link() {
        let file = ProjectFile::new();
        let factory = DataLinkFactory::new(
            &file,
            fixed_data(false),
            var_data(&file, &target_link_entries()),
        );

        factory.process().expect("link extraction succeeded");

        let file_ref = file.borrow();
        let links = file_ref.get_data_links();
        assert_eq!(links.len(), 1);

        let link = links.get(0).expect("link 0");
        let link = link.borrow();
        assert_eq!(link.get_id(), "/path/to/file.mpp");
        assert_eq!(
            link.get_target_field(),
            Some(FieldKey::Task(TaskField::UniqueId))
        );
        assert_eq!(link.get_target_unique_id(), Some(42));
        assert!(link.get_source_field().is_none());
        assert!(link.get_source_unique_id().is_none());
    }

/// Writes the source field when VIEW_NAME is present.
    #[test]
    fn extracts_a_source_data_link() {
        let file = ProjectFile::new();
        let mut entries = target_link_entries();
        entries.push((
            PropsKey::ViewName.get_value(),
            utf16("Gantt Chart"),
        ));

        let factory = DataLinkFactory::new(
            &file,
            fixed_data(false),
            var_data(&file, &entries),
        );
        factory.process().expect("link extraction succeeded");

        let file_ref = file.borrow();
        let links = file_ref.get_data_links();
        assert_eq!(links.len(), 1);

        let link = links.get(0).expect("link 0");
        let link = link.borrow();
        assert_eq!(
            link.get_source_field(),
            Some(FieldKey::Task(TaskField::UniqueId))
        );
        assert_eq!(link.get_source_unique_id(), Some(42));
        assert!(link.get_target_field().is_none());
    }

/// Multiple records with the same path are merged into one `DataLink` (Java's `computeIfAbsent`).
    #[test]
    fn repeated_paths_share_a_single_data_link() {
        let file = ProjectFile::new();
        let fixed_data = {
            let mut data = vec![0u8; FIXED_DATA_SIZE * 2];
            data[0..4].copy_from_slice(&VAR_DATA_ID.to_le_bytes());
            data[FIXED_DATA_SIZE..FIXED_DATA_SIZE + 4]
                .copy_from_slice(&VAR_DATA_ID.to_le_bytes());
            FixedData::with_fixed_size(FIXED_DATA_SIZE as i32, &data)
        };

        let factory = DataLinkFactory::new(&file, fixed_data, var_data(&file, &target_link_entries()));
        factory.process().expect("link extraction succeeded");

        let file_ref = file.borrow();
        assert_eq!(file_ref.get_data_links().len(), 1);
    }

/// Records with the 0x20 flag set are skipped (no link produced).
    #[test]
    fn link_properties_flag_skips_the_record() {
        let file = ProjectFile::new();
        let factory = DataLinkFactory::new(
            &file,
            fixed_data(true),
            var_data(&file, &target_link_entries()),
        );

        factory.process().expect("link extraction succeeded");

        assert_eq!(file.borrow().get_data_links().len(), 0);
    }

/// No link is produced when there is no matching entry in the variable-length data block (Java's `propsData == null` branch).
    #[test]
    fn missing_var_data_entry_produces_no_link() {
        let file = ProjectFile::new();
        let factory = DataLinkFactory::new(&file, fixed_data(false), var_data(&file, &[]));

        factory.process().expect("link extraction succeeded");

        assert_eq!(file.borrow().get_data_links().len(), 0);
    }

/// No panic when the fixed-length data block is too short (Java throws ArrayIndexOutOfBoundsException).
    #[test]
    fn short_fixed_data_does_not_panic() {
        let file = ProjectFile::new();
        let fixed_data = {
            let mut data = vec![0u8; 10];
            data[0..4].copy_from_slice(&VAR_DATA_ID.to_le_bytes());
            FixedData::with_fixed_size(10, &data)
        };

        let factory = DataLinkFactory::new(&file, fixed_data, var_data(&file, &target_link_entries()));
        factory.process().expect("link extraction succeeded");

        assert_eq!(file.borrow().get_data_links().len(), 0);
    }

/// The `FixedMeta` reference is only used to construct `FixedData` (the same construction chain as Java).
    #[test]
    fn fixed_meta_path_is_supported() {
        let mut meta_bytes = Vec::new();
        meta_bytes.extend_from_slice(&0xFADFADBAu32.to_le_bytes());
        meta_bytes.extend_from_slice(&0i32.to_le_bytes());
        meta_bytes.extend_from_slice(&1i32.to_le_bytes());
        meta_bytes.extend_from_slice(&0i32.to_le_bytes());
        let mut item = vec![0u8; 200];
        item[4..8].copy_from_slice(&0i32.to_le_bytes());
        meta_bytes.extend_from_slice(&item);

        let meta = FixedMeta::with_item_size(&meta_bytes, 200).expect("valid metadata");
        let mut data = vec![0u8; FIXED_DATA_SIZE];
        data[0..4].copy_from_slice(&VAR_DATA_ID.to_le_bytes());
        let fixed_data = FixedData::with_item_size(&meta, FIXED_DATA_SIZE as i32, &data);

        let file = ProjectFile::new();
        let factory = DataLinkFactory::new(&file, fixed_data, var_data(&file, &target_link_entries()));
        factory.process().expect("link extraction succeeded");

        assert_eq!(file.borrow().get_data_links().len(), 1);
    }
}
