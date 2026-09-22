//! Corresponds to Java: `org/mpxj/mpp/TableFactory14.java`
//!
//! Reconstructs "table definitions" from the `CTable` storage of an MPP14 file into the model-layer [`Table`].
//! Compared with MPP9 / MPP12's [`TableFactory`]: the column area starts at offset 12, each column is 115 bytes, and the column title is
//! inlined directly in the column block (`index + 13`), with alignment flags at `index + 5` / `index + 7`.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `TableFactory14(Integer, Integer, Integer)` | Same-order constructor, with three `Option<i32>` parameters | Java's `Integer` can be `null` |
//! | `Table createTable(ProjectFile file, byte[] data, VarMeta varMeta, Var2Data varData)` | `Rc<RefCell<Table>>`, with `file` as `&Rc<RefCell<ProjectFile>>` | The model-layer container (`TableContainer::add`) stores entries as `Rc<RefCell<Table>>` |
//! | `new Column(file)` | `Column::new(Rc::downgrade(file))` | This crate's `Column` holds the parent project by weak reference (see `crate::column`) |
//! | `FieldTypeHelper.getInstance(file, fieldID)` | `Option<MppFieldType>` → [`Column::set_field_type`] | Column field types in this crate are expressed with [`FieldKey`] |
//! | `ByteArrayHelper.getShort/getInt` throws `ArrayIndexOutOfBoundsException` when out of bounds | [`get_short_safe`] / [`get_int_safe`] return 0; the loop ends when the column data is shorter than a full column | Follows the "no panic" convention |
//! | The `if/else` chain of `column.setAlignTitle/AlignData` | Private function [`column_alignment`] | The three possible values are expressed equivalently with `match` |
//!
//! Note: this class **does not extend** [`TableFactory`] in Java (it is an independent `final class`, with a separate
//! implementation for each of versions 9 / 12), so the Rust side is likewise an independent struct, with no inheritance to reproduce.
//!
//! Skipped Java members: none.
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

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

/// Number of bytes occupied by each column definition (MPP14).
///
/// Corresponds to Java: `index += 115;`
const COLUMN_BLOCK_SIZE: usize = 115;

/// Table definition factory (MPP14 column layout).
///
/// Corresponds to Java: `final class TableFactory14`
pub struct TableFactory14 {
/// Column data key for the standard table.
    ///
    /// Corresponds to Java field: `private final Integer m_tableColumnDataStandard;`
    m_table_column_data_standard: Option<i32>,

/// Column data key for the enterprise table.
    ///
    /// Corresponds to Java field: `private final Integer m_tableColumnDataEnterprise;`
    m_table_column_data_enterprise: Option<i32>,

/// Column data key for the baseline table.
    ///
    /// Corresponds to Java field: `private final Integer m_tableColumnDataBaseline;`
    m_table_column_data_baseline: Option<i32>,
}

impl TableFactory14 {
/// Constructor.
    ///
    /// Corresponds to Java: `public TableFactory14(Integer tableColumnDataStandard,
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

/// Build a [`Table`] from fixed data (table properties) and variable data (column definitions).
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

// Java: if (columnData == null) { enterprise table; if still null, use the standard table }
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

        // Java: int index = 12;
        let mut index = 12usize;

        // Java: for (int loop = 0; loop < columnCount; loop++)
        for _ in 0..column_count {
// Java throws `ArrayIndexOutOfBoundsException` when there is insufficient data (the whole read fails);
// the Rust side ends the loop (without panicking), see the file header differences table
            if index.saturating_add(COLUMN_BLOCK_SIZE) > data.len() {
                break;
            }

            // Java: Column column = new Column(file);
            let mut column = Column::new(Rc::downgrade(file));

            // Java: column.setFieldType(FieldTypeHelper.getInstance(file, ByteArrayHelper.getInt(data, index)));
            let field_type = FieldTypeHelper::get_instance(file, get_int_safe(data, index));
            column.set_field_type(field_type.as_ref().map(|field| field.to_field_key()));

            // Java: column.setWidth(MPPUtility.getByte(data, index + 4));
            column.set_width(MPPUtility::get_byte(data, index + 4));

            // Java: String columnTitle = MPPUtility.getUnicodeString(data, index + 13);
            //       if (!columnTitle.isEmpty()) { column.setTitle(columnTitle); }
            let column_title = MPPUtility::get_unicode_string(data, index + 13);
            if !column_title.is_empty() {
                column.set_title(Some(column_title));
            }

            // Java: alignment = MPPUtility.getByte(data, index + 5); → setAlignTitle(...)
            column.set_align_title(column_alignment(MPPUtility::get_byte(data, index + 5)));

            // Java: alignment = MPPUtility.getByte(data, index + 7); → setAlignData(...)
            column.set_align_data(column_alignment(MPPUtility::get_byte(data, index + 7)));

            // Java: table.addColumn(column);
            table.add_column(column);

            // Java: index += 115;
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
    use crate::field_type::FieldType;
    use crate::mpp::custom_field_value_reader::test_support::{fixed_data, var_data_with_meta};
    use crate::task_field::TaskField;

/// Field ID.
    fn field_id(field: TaskField) -> i32 {
        FieldTypeHelper::get_field_id(Some(&MppFieldType::Builtin(FieldKey::Task(field))))
    }

/// Fixed-length table record: ID / name / resource table flag (offset 108).
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

/// Column data: `[4..6)` = column count - 1, followed by 115 bytes per column (field ID / width / title / alignment).
    ///
/// The parameters are `(field ID, width, title alignment, data alignment, title)`.
    fn column_data(columns: &[(i32, i32, i32, i32, &str)]) -> Vec<u8> {
        let mut data = vec![0u8; 12 + columns.len() * COLUMN_BLOCK_SIZE];
        data[4..6].copy_from_slice(&((columns.len() - 1) as u16).to_le_bytes());

        for (index, (field, width, align_title, align_data, title)) in columns.iter().enumerate() {
            let offset = 12 + index * COLUMN_BLOCK_SIZE;
            data[offset..offset + 4].copy_from_slice(&field.to_le_bytes());
            data[offset + 4] = *width as u8;
            data[offset + 5] = *align_title as u8;
            data[offset + 7] = *align_data as u8;

// The title is inlined in the column block (index + 13)
            let mut title_offset = offset + 13;
            for unit in title.encode_utf16() {
                data[title_offset..title_offset + 2].copy_from_slice(&unit.to_le_bytes());
                title_offset += 2;
            }
        }

        data
    }

/// Table properties and the 115-byte column block are read out completely.
    #[test]
    fn create_table_reads_properties_and_columns() {
        let data = column_data(&[
            (field_id(TaskField::Name), 30, 0, 1, "Task"),
            (field_id(TaskField::Duration), 12, 2, 2, ""),
        ]);

        let fixed = fixed_data(&[table_record(5, "&Gantt", true)]);
        let (var_meta, var_data) = var_data_with_meta(&[(5, 6, data)]);

        let file = ProjectFile::new();
        let factory = TableFactory14::new(Some(6), Some(7), Some(8));
        let table = factory.create_table(
            &file,
            fixed.get_byte_array_value(0).expect("has data"),
            &*var_meta,
            &var_data,
        );

        let table = table.borrow();
        assert_eq!(table.get_id(), 5);
        assert_eq!(table.get_name(), Some("Gantt"));
        assert!(table.get_resource_flag());

        let columns = table.get_columns();
        assert_eq!(columns.len(), 2);

        assert_eq!(columns[0].get_field_type(), Some(FieldKey::Task(TaskField::Name)));
        assert_eq!(columns[0].get_width(), 30);
        assert_eq!(columns[0].get_align_title(), Column::ALIGN_LEFT);
        assert_eq!(columns[0].get_align_data(), Column::ALIGN_CENTER);
        assert_eq!(columns[0].get_title(), Some("Task".to_owned()));

        assert_eq!(
            columns[1].get_field_type(),
            Some(FieldKey::Task(TaskField::Duration))
        );
        assert_eq!(columns[1].get_width(), 12);
        assert_eq!(columns[1].get_align_title(), Column::ALIGN_RIGHT);
// Empty title ⇒ consistently with Java, **do not** call setTitle, falling back to the field's default display name
        assert_eq!(
            columns[1].get_title(),
            Some(FieldKey::Task(TaskField::Duration).get_name().to_owned())
        );
    }

/// Does not panic when the column data is missing / truncated.
    #[test]
    fn malformed_data_is_tolerated() {
        let file = ProjectFile::new();
        let factory = TableFactory14::new(Some(6), Some(7), Some(8));
        let fixed = fixed_data(&[table_record(1, "T", false)]);
        let record = fixed.get_byte_array_value(0).expect("has data");

// There is no column data in the variable data
        let (var_meta, var_data) = var_data_with_meta(&[]);
        let table = factory.create_table(&file, record, &*var_meta, &var_data);
        assert!(table.borrow().get_columns().is_empty());

// The column data is only 4 bytes (length <= 6)
        let (var_meta, var_data) = var_data_with_meta(&[(1, 8, vec![0u8; 4])]);
        let table = factory.create_table(&file, record, &*var_meta, &var_data);
        assert!(table.borrow().get_columns().is_empty());

// Declares 2 columns but has data for only 1 column
        let mut truncated = column_data(&[(field_id(TaskField::Name), 1, 0, 0, "A")]);
        truncated[4..6].copy_from_slice(&1u16.to_le_bytes());
        let (var_meta, var_data) = var_data_with_meta(&[(1, 8, truncated)]);
        let table = factory.create_table(&file, record, &*var_meta, &var_data);
        let table = table.borrow();
        assert_eq!(table.get_columns().len(), 1);
        assert_eq!(table.get_columns()[0].get_title(), Some("A".to_owned()));
    }

/// Alignment code mapping (Java's if/else chain).
    #[test]
    fn alignment_mapping_matches_java() {
        assert_eq!(column_alignment(0x00), Column::ALIGN_LEFT);
        assert_eq!(column_alignment(0x01), Column::ALIGN_CENTER);
        assert_eq!(column_alignment(0x03), Column::ALIGN_RIGHT);
    }
}
