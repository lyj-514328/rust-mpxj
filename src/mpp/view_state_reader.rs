//! Corresponds to Java: `org/mpxj/mpp/ViewStateReader.java`
//!
//! Read the "view state" from an MPP file (the saved view name, the list of displayed task unique IDs, and the active
//! filter ID), and write it into `ProjectFile.getViews()`.
//!
//! Skipped Java mechanisms and reasons:
//!
//! | Java | Reason |
//! |---|---|
//! | `InputStream` / `ByteArrayInputStream` | Per project convention, Java's stream reading is changed to receiving `&[u8]` directly |
//! | Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable` | Not used by this class |
//!
//! # Restoring inheritance: composition + forwarding
//!
//! | Java | Rust |
//! |---|---|
//! | `abstract class ViewStateReader` | The [`ViewStateReader`] struct (a stateless abstract base class carrying only the shared logic `process`) |
//! | `protected abstract Props getProps(Var2Data varData) throws IOException` | [`ViewStateReaderMethods::get_props`] of the [`ViewStateReaderMethods`] trait (abstract methods are expressed as traits on the Rust side, consistent with this crate's `AbstractCalendarFactory`); `process` takes a "subclass that provides props" via `&dyn ViewStateReaderMethods` |
//! | `ViewStateReader9/12 extends ViewStateReader` | Each **holds** a [`ViewStateReader`] and forwards `process`, while implementing [`ViewStateReaderMethods`] (see `view_state_reader9.rs` / `view_state_reader12.rs`) |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `Props` (abstract base class) as the return type of `getProps` | [`Props`] (rebuilt from the parse result) | `Props9/12` implement "inheritance" via "composition + `Deref`": `&Props9` can auto-coerce to `&Props`, but `Deref` only borrows and cannot take the inner [`Props`] out by value, while this method's return type needs ownership, so [`Props::copied_from`] rebuilds it from the content (copying key/value entries one by one, semantically equivalent) |
//! | `throws IOException` | `MpxjResult<()>` | `Props9/12` return `Err` when parsing fails (Java throws `IOException`) |
//! | When the view content data index is out of bounds (length not a multiple of 4), Java throws `ArrayIndexOutOfBoundsException` | End the loop early (with a comment) | Do not panic |
//! | `file.getViews().setViewState(state)` | First `try_borrow_mut`; return `Err` if the file is already mutably borrowed | Java has no borrow checking; the Rust side makes this case an explicit error to avoid silently discarding the view state |

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::props::Props;
use crate::mpp::props_key::PropsKey;
use crate::mpp::var2_data::Var2Data;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_file::ProjectFile;
use crate::view_state::ViewState;

/// Offset of the "filter ID" in the fixed data block (two bytes, little-endian).
///
/// Corresponds to Java: `ByteArrayHelper.getShort(fixedData, 128)`
const FILTER_ID_OFFSET: usize = 128;

/// The set of abstract methods of the abstract base class `ViewStateReader`.
///
/// Corresponds to Java: the `protected abstract Props getProps(Var2Data varData)` of `abstract class ViewStateReader`
pub trait ViewStateReaderMethods {
/// Fetch the properties block in a file-format-specific way.
    ///
    /// Corresponds to Java: `protected abstract Props getProps(Var2Data varData) throws IOException`
    ///
/// Return `Ok(None)` when the properties block does not exist (Java returns `null`).
    fn get_props(&self, var_data: &Var2Data) -> MpxjResult<Option<Props>>;
}

/// Abstract base class for view state readers.
///
/// Corresponds to Java: `abstract class ViewStateReader`
pub struct ViewStateReader;

impl ViewStateReader {
/// Constructor.
    ///
/// Corresponds to Java: the implicit default constructor (Java declares no explicit constructor).
    pub fn new() -> Self {
        Self
    }

/// Process the saved view state.
    ///
    /// Corresponds to Java: `public void process(ProjectFile file, Var2Data varData, byte[] fixedData) throws IOException`
    ///
/// Rust-specific: Java's `getProps` is overridden by subclasses and virtually invoked from this method; the Rust side instead passes in explicitly
/// the properties block provider (a subclass instance, see [`ViewStateReaderMethods`]).
    pub fn process(
        &self,
        provider: &dyn ViewStateReaderMethods,
        file: &Rc<RefCell<ProjectFile>>,
        var_data: &Var2Data,
        fixed_data: &[u8],
    ) -> MpxjResult<()> {
        // Java: Props props = getProps(varData);
        let props = provider.get_props(var_data)?;

        // Java: if (props != null) { ... }
        let Some(props) = props else {
            return Ok(());
        };

        // Java: String viewName = MPPUtility.removeAmpersands(props.getUnicodeString(PropsKey.VIEW_NAME));
        let view_name =
            MPPUtility::remove_ampersands(props.get_unicode_string(PropsKey::ViewName).as_deref());

        // Java: byte[] listData = props.getByteArray(PropsKey.VIEW_CONTENTS);
        //       List<Integer> uniqueIdList = new ArrayList<>();
        //       if (listData != null) { for (int index = 0; index < listData.length; index += 4) { ... } }
        let mut unique_id_list: Vec<i32> = Vec::new();
        if let Some(list_data) = props.get_byte_array(PropsKey::ViewContents) {
// Java: file.getTaskByUniqueID(uniqueID) -- borrow once immutably and reuse it for the whole loop
            let file_ref = file.try_borrow().ok();
            let mut index = 0usize;

            while index < list_data.len() {
// Java throws when the last data item has fewer than 4 bytes (length not a multiple of 4)
// ArrayIndexOutOfBoundsException; the Rust side ends the loop early
                if index + 4 > list_data.len() {
                    break;
                }

                // Java: Integer uniqueID = Integer.valueOf(ByteArrayHelper.getInt(listData, index));
                let unique_id = ByteArrayHelper::get_int(list_data, index);

                // Java: if (file.getTaskByUniqueID(uniqueID) != null && (uniqueID.intValue() != 0 || index == 0))
// When the parent file is already borrowed (no such case in Java), treat it as "task does not exist" and skip the item.
                let task_exists = file_ref
                    .as_ref()
                    .is_some_and(|file| file.get_task_by_unique_id(Some(unique_id)).is_some());

                if task_exists && (unique_id != 0 || index == 0) {
                    unique_id_list.push(unique_id);
                }

                index += 4;
            }
        }

        // Java: int filterID = ByteArrayHelper.getShort(fixedData, 128);
// (Java throws AIOOBE when there are fewer than 130 bytes; the Rust side uses 0)
        let filter_id = if fixed_data.len() >= FILTER_ID_OFFSET + 2 {
            ByteArrayHelper::get_short(fixed_data, FILTER_ID_OFFSET)
        } else {
            0
        };

        // Java: ViewState state = new ViewState(file, viewName, uniqueIdList, filterID);
        //       file.getViews().setViewState(state);
        let state = ViewState::new(Rc::downgrade(file), view_name, unique_id_list, filter_id);
        let mut file_ref = file
            .try_borrow_mut()
            .map_err(|_| MpxjError::new("ProjectFile is already mutably borrowed"))?;
        file_ref.get_views_mut().set_view_state(Some(state));

        Ok(())
    }
}

impl Default for ViewStateReader {
/// Corresponds to Java: the implicit default constructor, equivalent to [`ViewStateReader::new`].
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::Filter;
    use crate::mpp::var_meta9::VarMeta9;

/// Properties block data for tests: provides only the two properties `process` needs.
    ///
/// Corresponds to the `Props9` constructed directly in the Java tests: the Rust side builds a [`Props`] according to the properties block layout.
    #[derive(Clone, Default)]
    struct TestProps {
        view_name: Option<String>,
        contents: Option<Vec<u8>>,
    }

    impl TestProps {
/// Construct the properties block (view name as UTF-16LE + NUL, contents as raw bytes).
        fn to_props(&self) -> Props {
            let mut props = Props::new();

            if let Some(view_name) = self.view_name.as_deref() {
                let mut value = Vec::new();
                for unit in view_name.encode_utf16() {
                    value.extend_from_slice(&unit.to_le_bytes());
                }
                value.extend_from_slice(&[0, 0]);
                props.put(PropsKey::ViewName.get_value(), value);
            }

            if let Some(contents) = self.contents.as_deref() {
                props.put(PropsKey::ViewContents.get_value(), contents.to_vec());
            }

            props
        }
    }

/// Test reader: provides the properties block from preset data (`None` corresponds to Java's `null`).
    #[derive(Default)]
    struct TestReader {
        props: Option<TestProps>,
    }

    impl ViewStateReaderMethods for TestReader {
        fn get_props(&self, _var_data: &Var2Data) -> MpxjResult<Option<Props>> {
            Ok(self.props.as_ref().map(TestProps::to_props))
        }
    }

/// Construct the view state reader: `view_name` and `contents` are both optional (`None` means the property is missing).
    fn reader(view_name: Option<&str>, contents: Option<Vec<u8>>) -> TestReader {
        TestReader {
            props: Some(TestProps {
                view_name: view_name.map(str::to_owned),
                contents,
            }),
        }
    }

/// Byte form of the unique ID list.
    fn contents(values: &[i32]) -> Vec<u8> {
        let mut data = Vec::new();
        for value in values {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data
    }

/// Construct an empty var data block (the `process` tests do not depend on its data).
    fn var_data(file: &Rc<RefCell<ProjectFile>>) -> Var2Data {
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

/// Construct the fixed data block and write the filter ID at offset 128.
    fn fixed_data(filter_id: i32) -> Vec<u8> {
        let mut data = vec![0u8; 138];
        data[FILTER_ID_OFFSET..FILTER_ID_OFFSET + 2]
            .copy_from_slice(&(filter_id as i16).to_le_bytes());
        data
    }

/// Add a task with the given unique ID to the project file.
    fn add_task(file: &Rc<RefCell<ProjectFile>>, unique_id: i32) {
        let task = file.borrow_mut().get_tasks_mut().add();
        task.borrow_mut().set_unique_id(Some(unique_id));
    }

/// Do nothing when the properties block is `null` (Java).
    #[test]
    fn process_without_props_is_noop() {
        let file = ProjectFile::new();
        let var_data = var_data(&file);

        ViewStateReader::new()
            .process(&TestReader::default(), &file, &var_data, &fixed_data(0))
            .expect("should be able to process");

        assert!(file.borrow().get_views().get_view_state().is_none());
    }

/// The view name / unique ID list / filter ID are all written into the view state.
    #[test]
    fn process_writes_view_state() {
        let file = ProjectFile::new();
        add_task(&file, 1);
        add_task(&file, 2);

// Filter (ID 7): used to verify that filterID has been passed into ViewState
        let mut filter = Filter::new();
        filter.set_id(Some(7));
        filter.set_name(Some("Filter A".to_owned()));
        file.borrow_mut()
            .get_filters_mut()
            .add_filter(Rc::new(RefCell::new(filter)));

// Contents: [1, 2, 0, 99]: 1 and 2 have corresponding tasks; 0 is not first and has no task; 99 has no task
        let reader = reader(Some("&Gantt Chart"), Some(contents(&[1, 2, 0, 99])));
        let var_data = var_data(&file);

        ViewStateReader::new()
            .process(&reader, &file, &var_data, &fixed_data(7))
            .expect("should be able to process");

        let file_ref = file.borrow();
        let state = file_ref
            .get_views()
            .get_view_state()
            .expect("should write view state");
        assert_eq!(state.get_view_name(), Some("Gantt Chart"));
        assert_eq!(state.get_unique_id_list(), &[1, 2]);
        assert_eq!(
            state
                .get_filter()
                .map(|filter| filter.borrow().get_name().map(str::to_owned)),
            Some(Some("Filter A".to_owned()))
        );
    }

/// A unique ID of 0 is kept when it is in the first data item (Java's `index == 0` branch).
    ///
/// Java's condition also requires that the unique ID resolves to a task, so the file must contain a task with unique ID 0
/// (MS Project's project summary task).
    #[test]
    fn leading_zero_unique_id_is_kept() {
        let file = ProjectFile::new();
        add_task(&file, 0);
        add_task(&file, 3);

        let reader = reader(None, Some(contents(&[0, 3])));
        let var_data = var_data(&file);

        ViewStateReader::new()
            .process(&reader, &file, &var_data, &fixed_data(0))
            .expect("should be able to process");

        let file_ref = file.borrow();
        let state = file_ref.get_views().get_view_state().expect("should write view state");
        assert_eq!(state.get_view_name(), None);
        assert_eq!(state.get_unique_id_list(), &[0, 3]);
// No filter definition (filter_id = 0) -> getFilter() returns None
        assert!(state.get_filter().is_none());
    }

/// Do not panic when the view content length is not a multiple of 4; out-of-bounds trailing data is ignored.
    #[test]
    fn malformed_view_contents_is_ignored() {
        let file = ProjectFile::new();
        add_task(&file, 4);

        let reader = reader(
            None,
// 4 bytes of valid data + 2 trailing bytes
            Some(vec![0x04, 0x00, 0x00, 0x00, 0xAA, 0xBB]),
        );
        let var_data = var_data(&file);

        ViewStateReader::new()
            .process(&reader, &file, &var_data, &fixed_data(0))
            .expect("should be able to process");

        let file_ref = file.borrow();
        let state = file_ref.get_views().get_view_state().expect("should write view state");
        assert_eq!(state.get_unique_id_list(), &[4]);
    }

/// Do not panic when the fixed data is too short (filter ID is 0).
    #[test]
    fn short_fixed_data_does_not_panic() {
        let file = ProjectFile::new();

        let reader = reader(Some("View"), None);
        let var_data = var_data(&file);

        ViewStateReader::new()
            .process(&reader, &file, &var_data, &[])
            .expect("should be able to process");

        let file_ref = file.borrow();
        let state = file_ref.get_views().get_view_state().expect("should write view state");
        assert_eq!(state.get_view_name(), Some("View"));
        assert!(state.get_unique_id_list().is_empty());
    }

/// Return `Err` when the project file is already mutably borrowed (Rust-specific); do not panic.
    #[test]
    fn borrowed_file_returns_error() {
        let file = ProjectFile::new();
        let var_data = var_data(&file);

        let _borrow = file.borrow_mut();
        let result = ViewStateReader::new().process(
            &reader(None, None),
            &file,
            &var_data,
            &fixed_data(0),
        );

        assert!(result.is_err());
    }
}
