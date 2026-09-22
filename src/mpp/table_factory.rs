//! Corresponds to Java: `org/mpxj/mpp/TableFactory.java`
//!
//! Restore a "table definition" from the MPP file's `CTable` storage into the model layer's [`Table`]: the fixed data gives
//! the table ID / name / whether it is a resource table, and the variable data (standard table / enterprise table / baseline table keys, falling back by priority)
//! gives the column definitions.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `TableFactory(Integer tableColumnDataStandard, Integer tableColumnDataEnterprise, Integer tableColumnDataBaseline)` | constructed in the same order, the three parameters are `Option<i32>` | Java's `Integer` may be `null` (`MPP9Reader`'s `TABLE_COLUMN_DATA_BASELINE` is `null`) |
//! | `Table createTable(ProjectFile file, byte[] data, VarMeta varMeta, Var2Data varData)` | `Rc<RefCell<Table>>`, `file` is `&Rc<RefCell<ProjectFile>>` | The model-layer container (`TableContainer::add`) stores by `Rc<RefCell<Table>>`, so the handle is constructed directly in the factory (in Java the caller stores it via `container.add(table)`) |
//! | `new Column(file)` | `Column::new(Rc::downgrade(file))` | This crate's `Column` holds its parent project by weak reference (see `crate::column`) |
//! | `FieldTypeHelper.getInstance(file, fieldID)` | `Option<MppFieldType>` → [`Column::set_field_type`] | This crate expresses column field types with [`FieldKey`] |
//! | `ByteArrayHelper.getShort/getInt` throws `ArrayIndexOutOfBoundsException` when out of bounds | [`get_short_safe`] / [`get_int_safe`] return 0; the loop ends when the column data is shorter than a full column | follows the "no panic" convention (Java lets malformed data fail the whole read) |
//! | the `if/else` chain in `column.setAlignTitle(int)` | the private function [`column_alignment`] | the three values (left / center / right) are expressed equivalently with `match` |
//!
//! Skipped Java members: none.
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: this class uses none of these mechanisms.

use std::cell::RefCell;
use std::rc::Rc;

use crate::column::Column;
use crate::common::field_type_helper::FieldTypeHelper;
use crate::mpp::abstract_calendar_factory::{get_int_safe, get_short_safe};
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_meta::VarMeta;
use crate::project_file::ProjectFile;
use crate::table::Table;

/// Number of bytes each column definition occupies (MPP9 / MPP12).
///
/// Corresponds to Java: `index += 12;`
const COLUMN_BLOCK_SIZE: usize = 12;

/// Table definition factory (MPP9 / MPP12 column layout).
///
/// Corresponds to Java: `final class TableFactory`
pub struct TableFactory {
/// Standard table column data key.
    ///
    /// Corresponds to Java field: `private final Integer m_tableColumnDataStandard;`
    m_table_column_data_standard: Option<i32>,

/// Enterprise table column data key.
    ///
    /// Corresponds to Java field: `private final Integer m_tableColumnDataEnterprise;`
    m_table_column_data_enterprise: Option<i32>,

/// Baseline table column data key.
    ///
    /// Corresponds to Java field: `private final Integer m_tableColumnDataBaseline;`
    m_table_column_data_baseline: Option<i32>,
}

impl TableFactory {
/// Constructor.
    ///
    /// Corresponds to Java: `public TableFactory(Integer tableColumnDataStandard,
    /// Integer tableColumnDataEnterprise, Integer tableColumnDataBaseline)`
    pub fn new(
        table_column_data_standard: Option<i32>,
        table_column_data_enterprise: Option<i32>,
        table_column_data_baseline: Option<i32>,
    ) -> Self {
        Self {
            m_table_column_data_standard: table_column_data_standard,
            m_table_column_data_enterprise: table_column_data_enterprise,
            m_table_column_data_baseline: table_column_data_baseline,
        }
    }

/// Construct a [`Table`] from the fixed data (table properties) and variable data (column definitions).
    ///
    /// Corresponds to Java: `public Table createTable(ProjectFile file, byte[] data, VarMeta varMeta, Var2Data varData)`
    pub fn create_table(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        data: &[u8],
        var_meta: &dyn VarMeta,
        var_data: &Var2Data,
    ) -> Rc<RefCell<Table>> {
        // Java: Table table = new Table();
        let mut table = Table::new();

        // Java: table.setID(ByteArrayHelper.getInt(data, 0));
        let table_id = get_int_safe(data, 0);
        table.set_id(table_id);

        // Java: table.setResourceFlag(ByteArrayHelper.getShort(data, 108) == 1);
        table.set_resource_flag(get_short_safe(data, 108) == 1);

        // Java: table.setName(MPPUtility.removeAmpersands(MPPUtility.getUnicodeString(data, 4)));
        let name = MPPUtility::get_unicode_string(data, 4);
        table.set_name(MPPUtility::remove_ampersands(Some(&name)));

        // Java: byte[] columnData = null; Integer tableID = Integer.valueOf(table.getID());
        //       if (m_tableColumnDataBaseline != null) { columnData = varData.getByteArray(varMeta.getOffset(tableID, m_tableColumnDataBaseline)); }
        let mut column_data = self.lookup_column_data(
            table_id,
            self.m_table_column_data_baseline,
            var_meta,
            var_data,
        );

        // Java: if (columnData == null) { columnData = varData.getByteArray(varMeta.getOffset(tableID, m_tableColumnDataEnterprise));
        //          if (columnData == null) { columnData = varData.getByteArray(varMeta.getOffset(tableID, m_tableColumnDataStandard)); } }
        if column_data.is_none() {
            column_data = self.lookup_column_data(
                table_id,
                self.m_table_column_data_enterprise,
                var_meta,
                var_data,
            );

            if column_data.is_none() {
                column_data = self.lookup_column_data(
                    table_id,
                    self.m_table_column_data_standard,
                    var_meta,
                    var_data,
                );
            }
        }

        // Java: processColumnData(file, table, columnData);
        self.process_column_data(file, &mut table, column_data.as_deref());

        Rc::new(RefCell::new(table))
    }

/// Retrieve column data by data type key; returns `None` when the key is `None` or the data is missing.
    ///
    /// Corresponds to Java: `varData.getByteArray(varMeta.getOffset(tableID, type))`
    fn lookup_column_data(
        &self,
        table_id: i32,
        type_: Option<i32>,
        var_meta: &dyn VarMeta,
        var_data: &Var2Data,
    ) -> Option<Vec<u8>> {
        let type_ = type_?;
// Java: getOffset returns null for an unregistered (id, type) (None on the Rust side)
        var_data.get_byte_array_at_offset(var_meta.get_offset(Some(table_id), Some(type_)))
    }

/// Add the column definitions to the table.
    ///
    /// Corresponds to Java: `private void processColumnData(ProjectFile file, Table table, byte[] data)`
    fn process_column_data(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        table: &mut Table,
        data: Option<&[u8]>,
    ) {
        // Java: if (data != null && data.length > 6)
        let Some(data) = data else {
            return;
        };
        if data.len() <= 6 {
            return;
        }

        // Java: int columnCount = ByteArrayHelper.getShort(data, 4) + 1;
        let column_count = get_short_safe(data, 4) + 1;

        // Java: int index = 8;
        let mut index = 8usize;

        // Java: for (int loop = 0; loop < columnCount; loop++)
        for _ in 0..column_count {
// Java throws `ArrayIndexOutOfBoundsException` when the data is insufficient (the whole read fails);
// the Rust side ends the loop (without panicking); see the file header's differences table
            if index.saturating_add(COLUMN_BLOCK_SIZE) > data.len() {
                break;
            }

            // Java: Column column = new Column(file);
            let mut column = Column::new(Rc::downgrade(file));

            // Java: column.setFieldType(FieldTypeHelper.getInstance(file, ByteArrayHelper.getInt(data, index)));
            let field_type =
                FieldTypeHelper::get_instance(file, get_int_safe(data, index));
            column.set_field_type(field_type.as_ref().map(|field| field.to_field_key()));

            // Java: column.setWidth(MPPUtility.getByte(data, index + 4));
            column.set_width(MPPUtility::get_byte(data, index + 4));

            // Java: columnTitleOffset = ByteArrayHelper.getShort(data, index + 6);
            //       if (columnTitleOffset != 0) { column.setTitle(MPPUtility.getUnicodeString(data, columnTitleOffset)); }
            let column_title_offset = get_short_safe(data, index + 6);
            if column_title_offset != 0 {
                let title = MPPUtility::get_unicode_string(data, column_title_offset as usize);
                column.set_title(Some(title));
            }

            // Java: alignment = MPPUtility.getByte(data, index + 8); → setAlignTitle(...)
            column.set_align_title(column_alignment(MPPUtility::get_byte(data, index + 8)));

            // Java: alignment = MPPUtility.getByte(data, index + 10); → setAlignData(...)
            column.set_align_data(column_alignment(MPPUtility::get_byte(data, index + 10)));

            // Java: table.addColumn(column);
            table.add_column(column);

            // Java: index += 12;
            index += COLUMN_BLOCK_SIZE;
        }
    }
}

/// Map the alignment codes in the file to [`Column`] constants.
///
/// Corresponds to Java: the two sections in `processColumnData`
/// `if ((alignment & 0x0F) == 0x00) { ... ALIGN_LEFT } else if (... == 0x01) { ... ALIGN_CENTER } else { ... ALIGN_RIGHT }`.
fn column_alignment(alignment: i32) -> i32 {
    match alignment & 0x0F {
        0x00 => Column::ALIGN_LEFT,
        0x01 => Column::ALIGN_CENTER,
        _ => Column::ALIGN_RIGHT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::mpp_field_type::MppFieldType;
    use crate::field_key::FieldKey;
    use crate::mpp::custom_field_value_reader::test_support::{fixed_data, var_data_with_meta};
    use crate::task_field::TaskField;

/// Field number (the MPP9 / MPP12 field numbering scheme).
    fn field_id(field: TaskField) -> i32 {
        FieldTypeHelper::get_field_id(Some(&MppFieldType::Builtin(FieldKey::Task(field))))
    }

/// Table fixed record: ID / name / resource-table flag (offset 108).
    fn table_record(id: i32, name: &str, resource: bool) -> Vec<u8> {
        let mut record = vec![0u8; 116];
        record[0..4].copy_from_slice(&id.to_le_bytes());
        record[108..110].copy_from_slice(&(u16::from(resource)).to_le_bytes());

        let mut offset = 4;
        for unit in name.encode_utf16() {
            record[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
            offset += 2;
        }
        record
    }

/// Column data: `[4..6)` = number of columns - 1, then every column is 12 bytes (field number / width / title offset / alignment),
/// with the title text placed after the column area.
    ///
/// The parameters are `(field number, width, title alignment, data alignment, title)`.
    fn column_data(columns: &[(i32, i32, i32, i32, &str)]) -> Vec<u8> {
        let base = 8 + columns.len() * COLUMN_BLOCK_SIZE;
        let mut titles = Vec::new();
        let mut title_offsets = Vec::new();

        for (_, _, _, _, title) in columns {
            title_offsets.push(base + titles.len());
            for unit in title.encode_utf16() {
                titles.extend_from_slice(&unit.to_le_bytes());
            }
            titles.extend_from_slice(&[0, 0]);
        }

        let mut data = vec![0u8; base];
        data[4..6].copy_from_slice(&((columns.len() - 1) as u16).to_le_bytes());

        for (index, (field, width, align_title, align_data, _)) in columns.iter().enumerate() {
            let offset = 8 + index * COLUMN_BLOCK_SIZE;
            data[offset..offset + 4].copy_from_slice(&field.to_le_bytes());
            data[offset + 4] = *width as u8;
            data[offset + 6..offset + 8]
                .copy_from_slice(&(title_offsets[index] as u16).to_le_bytes());
            data[offset + 8] = *align_title as u8;
            data[offset + 10] = *align_data as u8;
        }

        data.extend_from_slice(&titles);
        data
    }

/// The table properties and column definitions are read out in full.
    #[test]
    fn create_table_reads_properties_and_columns() {
        let data = column_data(&[
            (field_id(TaskField::Name), 20, 1, 0, "Task"),
            (field_id(TaskField::Cost), 10, 2, 1, ""),
        ]);

        let fixed = fixed_data(&[table_record(1, "&Entry", false)]);
        let (var_meta, var_data) = var_data_with_meta(&[(1, 4, data)]);

        let file = ProjectFile::new();
        let factory = TableFactory::new(Some(4), Some(5), None);
        let table = factory.create_table(&file, fixed.get_byte_array_value(0).expect("has data"), &*var_meta, &var_data);

        let table = table.borrow();
        assert_eq!(table.get_id(), 1);
        assert_eq!(table.get_name(), Some("Entry"));
        assert!(!table.get_resource_flag());

        let columns = table.get_columns();
        assert_eq!(columns.len(), 2);

        assert_eq!(columns[0].get_field_type(), Some(FieldKey::Task(TaskField::Name)));
        assert_eq!(columns[0].get_width(), 20);
        assert_eq!(columns[0].get_align_title(), Column::ALIGN_CENTER);
        assert_eq!(columns[0].get_align_data(), Column::ALIGN_LEFT);
// the column title is given by an offset in the data block (offset 0 means unset ⇒ fall back to the field name)
        assert_eq!(columns[0].get_title(), Some("Task".to_owned()));

        assert_eq!(columns[1].get_field_type(), Some(FieldKey::Task(TaskField::Cost)));
        assert_eq!(columns[1].get_width(), 10);
        assert_eq!(columns[1].get_align_title(), Column::ALIGN_RIGHT);
        assert_eq!(columns[1].get_align_data(), Column::ALIGN_CENTER);
// when the title is an empty string it is kept as an empty string, consistent with Java (`m_title` being non-null means it does not fall back to the field name)
        assert_eq!(columns[1].get_title(), Some(String::new()));
    }

/// Fallback order of column data: baseline → enterprise → standard.
    #[test]
    fn column_data_falls_back_in_java_order() {
        let table_fixed = fixed_data(&[table_record(9, "T", true)]);
        let record = table_fixed.get_byte_array_value(0).expect("has data");

// all three keys have data: prefer baseline (key 3)
        let (var_meta, var_data) = var_data_with_meta(&[
            (9, 1, column_data(&[(field_id(TaskField::Name), 1, 0, 0, "S")])),
            (9, 2, column_data(&[(field_id(TaskField::Name), 2, 0, 0, "E")])),
            (9, 6, column_data(&[(field_id(TaskField::Name), 3, 0, 0, "B")])),
        ]);
        let file = ProjectFile::new();
        let factory = TableFactory::new(Some(1), Some(2), Some(6));
        let table = factory.create_table(&file, record, &*var_meta, &var_data);
        let table = table.borrow();
        assert_eq!(table.get_columns()[0].get_width(), 3);
        assert!(table.get_resource_flag());
        drop(table);

// the baseline key is null ⇒ take enterprise (key 2)
        let file = ProjectFile::new();
        let factory = TableFactory::new(Some(1), Some(2), None);
        let table = factory.create_table(&file, record, &*var_meta, &var_data);
        assert_eq!(table.borrow().get_columns()[0].get_width(), 2);

// both baseline and enterprise keys are null ⇒ take standard (key 1)
        let file = ProjectFile::new();
        let factory = TableFactory::new(Some(1), None, None);
        let table = factory.create_table(&file, record, &*var_meta, &var_data);
        assert_eq!(table.borrow().get_columns()[0].get_width(), 1);
    }

/// Do not panic when the column data is missing / truncated / the fixed data is too short.
    #[test]
    fn malformed_data_is_tolerated() {
        let file = ProjectFile::new();
        let factory = TableFactory::new(Some(1), Some(2), Some(3));

// the variable data has no column data for this table ⇒ properties only, no columns
        let fixed = fixed_data(&[table_record(1, "T", false)]);
        let (var_meta, var_data) = var_data_with_meta(&[]);
        let table = factory.create_table(
            &file,
            fixed.get_byte_array_value(0).expect("has data"),
            &*var_meta,
            &var_data,
        );
        assert_eq!(table.borrow().get_id(), 1);
        assert!(table.borrow().get_columns().is_empty());

// the column data is only 4 bytes (length <= 6) ⇒ return directly
        let (var_meta, var_data) = var_data_with_meta(&[(1, 3, vec![0u8; 4])]);
        let table = factory.create_table(
            &file,
            fixed.get_byte_array_value(0).expect("has data"),
            &*var_meta,
            &var_data,
        );
        assert!(table.borrow().get_columns().is_empty());

// 3 columns are declared but the data is insufficient: end the loop after reading the available columns
        let mut truncated = vec![0u8; 8 + 12];
        truncated[4..6].copy_from_slice(&2u16.to_le_bytes());
        truncated[8..12].copy_from_slice(&field_id(TaskField::Name).to_le_bytes());
        let (var_meta, var_data) = var_data_with_meta(&[(1, 3, truncated)]);
        let table = factory.create_table(
            &file,
            fixed.get_byte_array_value(0).expect("has data"),
            &*var_meta,
            &var_data,
        );
        let table = table.borrow();
        assert_eq!(table.get_columns().len(), 1);
        assert_eq!(
            table.get_columns()[0].get_field_type(),
            Some(FieldKey::Task(TaskField::Name))
        );

// the fixed data is shorter than 4 bytes (Java throws) ⇒ ID is 0, name is an empty string, no panic
        let (var_meta, var_data) = var_data_with_meta(&[]);
        let table = factory.create_table(&file, &[0u8; 4], &*var_meta, &var_data);
        assert_eq!(table.borrow().get_id(), 0);
        assert_eq!(table.borrow().get_name(), Some(""));
        assert!(!table.borrow().get_resource_flag());
    }

/// Alignment code mapping (Java's if/else chain).
    #[test]
    fn alignment_mapping_matches_java() {
        assert_eq!(column_alignment(0x00), Column::ALIGN_LEFT);
        assert_eq!(column_alignment(0x01), Column::ALIGN_CENTER);
        assert_eq!(column_alignment(0x02), Column::ALIGN_RIGHT);
        assert_eq!(column_alignment(0x0F), Column::ALIGN_RIGHT);
    }
}
