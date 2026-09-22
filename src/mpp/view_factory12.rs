//! Corresponds to Java: `org/mpxj/mpp/ViewFactory12.java`
//!
//! View factory for MPP12 files: based on the "split view flag" and "view type" in the fixed data block, decides whether to create
//! [`SplitView9`], `GanttChartView12` or [`GenericView12`].
//!
//! Skipped Java mechanisms and reasons:
//!
//! | Java | Reason |
//! |---|---|
//! | Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable` | not used by this class |
//!
//! # How inheritance / interfaces are reproduced
//!
//! | Java | Rust |
//! |---|---|
//! | `ViewFactory12 implements ViewFactory` | [`ViewFactory12`] implements the [`ViewFactory`] trait |
//! | `createView(ProjectFile, byte[], byte[], Var2Data, Map<Integer, FontBase>) throws IOException` | [`ViewFactory::create_view`] (`&Rc<RefCell<ProjectFile>>`, `&[u8]`, `&HashMap<i32, FontBase>`, returning `MpxjResult<Rc<RefCell<dyn View>>>`) |
//!
//! # Differences from Java
//!
//! - Out-of-bounds reads: Java throws when calling `ByteArrayHelper.getShort` on `fixedData` with fewer than 112 / 114 bytes
//! `ArrayIndexOutOfBoundsException`; Rust takes 0 (and comments), without panicking.
//! - The split view branch reuses [`SplitView9`] just as Java does (reading the data type 1 property block), so it differs from
//! [`GenericView12`] (which reads data type 6) in the property block type.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::font_base::FontBase;
use crate::mpp::gantt_chart_view12::GanttChartView12;
use crate::mpp::generic_view12::GenericView12;
use crate::mpp::split_view9::SplitView9;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::view_factory::ViewFactory;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;
use crate::view::View;
use crate::view_type::ViewType;

/// Offset of the "split view flag" in the fixed data block.
///
/// Corresponds to Java: `ByteArrayHelper.getShort(fixedData, 110)`
const SPLIT_VIEW_FLAG_OFFSET: usize = 110;

/// Offset of the "view type" in the fixed data block.
///
/// Corresponds to Java: `ByteArrayHelper.getShort(fixedData, 112)`
const VIEW_TYPE_OFFSET: usize = 112;

/// MPP12 view factory.
///
/// Corresponds to Java: `class ViewFactory12 implements ViewFactory`
pub struct ViewFactory12;

impl ViewFactory12 {
/// Constructor.
    ///
/// Corresponds to Java: implicit default constructor (Java does not explicitly declare a constructor).
    pub fn new() -> Self {
        Self
    }
}

impl Default for ViewFactory12 {
/// Corresponds to Java: implicit default constructor, equivalent to [`ViewFactory12::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl ViewFactory for ViewFactory12 {
/// Creates a view.
    ///
    /// Corresponds to Java: `@Override public View createView(ProjectFile file, byte[] fixedMeta,
    /// byte[] fixedData, Var2Data varData, Map<Integer, FontBase> fontBases) throws IOException`
    fn create_view(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        fixed_meta: &[u8],
        fixed_data: &[u8],
        var_data: &Var2Data,
        font_bases: &HashMap<i32, FontBase>,
    ) -> MpxjResult<Rc<RefCell<dyn View>>> {
        // Java: int splitViewFlag = ByteArrayHelper.getShort(fixedData, 110);
// (Java throws AIOOBE when there are fewer than 112 bytes; Rust takes 0)
        let split_view_flag = if fixed_data.len() >= SPLIT_VIEW_FLAG_OFFSET + 2 {
            ByteArrayHelper::get_short(fixed_data, SPLIT_VIEW_FLAG_OFFSET)
        } else {
            0
        };

        if split_view_flag == 1 {
            // Java: view = new SplitView9(file, fixedData, varData);
            let view: Rc<RefCell<dyn View>> =
                Rc::new(RefCell::new(SplitView9::new(file, fixed_data, var_data)?));
            return Ok(view);
        }

        // Java: ViewType type = ViewType.getInstance(ByteArrayHelper.getShort(fixedData, 112));
// (Java throws AIOOBE when there are fewer than 114 bytes; Rust takes 0, i.e. ViewType.UNKNOWN)
        let type_value = if fixed_data.len() >= VIEW_TYPE_OFFSET + 2 {
            ByteArrayHelper::get_short(fixed_data, VIEW_TYPE_OFFSET)
        } else {
            0
        };
        let view_type = ViewType::get_instance(type_value);

        let view: Rc<RefCell<dyn View>> = if view_type == ViewType::GanttChart {
            // Java: view = new GanttChartView12(file, fixedMeta, fixedData, varData, fontBases);
            Rc::new(RefCell::new(GanttChartView12::new(
                file, fixed_meta, fixed_data, var_data, font_bases,
            )?))
        } else {
            // Java: view = new GenericView12(file, fixedData, varData);
            Rc::new(RefCell::new(GenericView12::new(
                file, fixed_data, var_data,
            )?))
        };

        Ok(view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpp::props_key::PropsKey;
    use crate::mpp::var_meta9::VarMeta9;

/// Constructs the fixed data block, optionally specifying the split view flag at offset 110 and the view type at offset 112.
    fn fixed_data(id: i32, split_view_flag: i32, view_type: i32) -> Vec<u8> {
        let mut data = vec![0u8; 138];
        data[0..4].copy_from_slice(&id.to_le_bytes());
        for (index, unit) in "Split".encode_utf16().enumerate() {
            let offset = 4 + index * 2;
            data[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
        }
        data[SPLIT_VIEW_FLAG_OFFSET..SPLIT_VIEW_FLAG_OFFSET + 2]
            .copy_from_slice(&(split_view_flag as i16).to_le_bytes());
        data[VIEW_TYPE_OFFSET..VIEW_TYPE_OFFSET + 2]
            .copy_from_slice(&(view_type as i16).to_le_bytes());
        data
    }

/// Constructs a UTF-16LE string (including the trailing NUL).
    fn utf16(value: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in value.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00]);
        data
    }

/// Constructs property data using the MPP9 property block layout (MPP12's property block is parsed with `Props12`, but the layout is compatible).
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

/// Creates [`SplitView9`] when the split view flag is 1 (reads the data type 1 property block).
    #[test]
    fn split_view_flag_creates_split_view() {
        let file = ProjectFile::new();
        let data = fixed_data(1, 1, 1);
        let var_data = var_data(
            &file,
            &[
                (
                    1,
                    1,
                    props(&[(PropsKey::TableName.get_value(), utf16("Entry1"))]),
                ),
                (
                    1,
                    6,
                    props(&[(PropsKey::TableName.get_value(), utf16("Entry6"))]),
                ),
            ],
        );

        let view = ViewFactory12::new()
            .create_view(&file, &[], &data, &var_data, &HashMap::new())
            .expect("should create view");

        assert_eq!(view.borrow().get_id(), Some(1));
        assert_eq!(view.borrow().get_table_name(), Some("Entry1"));
    }

/// Creates [`GenericView12`] when the type is not a Gantt chart and the view is not split (reads the data type 6 property block).
    #[test]
    fn non_gantt_type_creates_generic_view() {
        let file = ProjectFile::new();
        let data = fixed_data(2, 0, ViewType::TaskSheet as i32);
        let var_data = var_data(
            &file,
            &[
                (
                    2,
                    1,
                    props(&[(PropsKey::TableName.get_value(), utf16("Entry1"))]),
                ),
                (
                    2,
                    6,
                    props(&[(PropsKey::TableName.get_value(), utf16("Entry6"))]),
                ),
            ],
        );

        let view = ViewFactory12::new()
            .create_view(&file, &[], &data, &var_data, &HashMap::new())
            .expect("should create view");

        assert_eq!(view.borrow().get_table_name(), Some("Entry6"));
        assert_eq!(view.borrow().get_type(), Some(ViewType::TaskSheet));
    }

/// Does not panic when the fixed data is too short.
    #[test]
    fn short_fixed_data_does_not_panic() {
        let file = ProjectFile::new();
        let var_data = var_data(&file, &[]);

        let view = ViewFactory12::new()
            .create_view(&file, &[], &[], &var_data, &HashMap::new())
            .expect("should create view");

        assert_eq!(view.borrow().get_id(), Some(0));
        assert_eq!(view.borrow().get_type(), Some(ViewType::Unknown));
    }
}
