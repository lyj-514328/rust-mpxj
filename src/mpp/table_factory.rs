//! 对应 Java: `org/mpxj/mpp/TableFactory.java`
//!
//! 从 MPP 文件的 `CTable` 存储里把「表格定义」还原成模型层的 [`Table`]：定长数据给出
//! 表格 ID / 名称 / 是否资源表，变长数据（标准表 / 企业表 / 基线表三种键，按优先级回退）
//! 给出列定义。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `TableFactory(Integer tableColumnDataStandard, Integer tableColumnDataEnterprise, Integer tableColumnDataBaseline)` | 同序构造，三个参数为 `Option<i32>` | Java 的 `Integer` 可为 `null`（`MPP9Reader` 的 `TABLE_COLUMN_DATA_BASELINE` 就是 `null`） |
//! | `Table createTable(ProjectFile file, byte[] data, VarMeta varMeta, Var2Data varData)` | `Rc<RefCell<Table>>`，`file` 为 `&Rc<RefCell<ProjectFile>>` | 模型层容器（`TableContainer::add`）按 `Rc<RefCell<Table>>` 存放，故直接在工厂里构造该句柄（Java 侧由调用方 `container.add(table)` 存入） |
//! | `new Column(file)` | `Column::new(Rc::downgrade(file))` | 本 crate 的 `Column` 以弱引用持有父项目（见 `crate::column`） |
//! | `FieldTypeHelper.getInstance(file, fieldID)` | `Option<MppFieldType>` → [`Column::set_field_type`] | 本 crate 的列字段类型用 [`FieldKey`] 表达 |
//! | `ByteArrayHelper.getShort/getInt` 越界抛 `ArrayIndexOutOfBoundsException` | [`get_short_safe`] / [`get_int_safe`] 返回 0；列数据不足一整列时结束循环 | 遵循「不 panic」约定（Java 对畸形数据会让整次读取失败） |
//! | `column.setAlignTitle(int)` 的 `if/else` 链 | 私有函数 [`column_alignment`] | 三档取值（左 / 中 / 右）用 `match` 等价表达 |
//!
//! 跳过的 Java 成员：无。
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

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

/// 每列定义占用的字节数（MPP9 / MPP12）。
///
/// 对应 Java: `index += 12;`
const COLUMN_BLOCK_SIZE: usize = 12;

/// 表格定义工厂（MPP9 / MPP12 的列布局）。
///
/// 对应 Java: `final class TableFactory`
pub struct TableFactory {
    /// 标准表列数据键。
    ///
    /// 对应 Java 字段: `private final Integer m_tableColumnDataStandard;`
    m_table_column_data_standard: Option<i32>,

    /// 企业表列数据键。
    ///
    /// 对应 Java 字段: `private final Integer m_tableColumnDataEnterprise;`
    m_table_column_data_enterprise: Option<i32>,

    /// 基线表列数据键。
    ///
    /// 对应 Java 字段: `private final Integer m_tableColumnDataBaseline;`
    m_table_column_data_baseline: Option<i32>,
}

impl TableFactory {
    /// 构造方法。
    ///
    /// 对应 Java: `public TableFactory(Integer tableColumnDataStandard,
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

    /// 由定长数据（表格属性）与变长数据（列定义）构造一个 [`Table`]。
    ///
    /// 对应 Java: `public Table createTable(ProjectFile file, byte[] data, VarMeta varMeta, Var2Data varData)`
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

    /// 按数据类型键取出列数据；键为 `None` 或数据缺失时返回 `None`。
    ///
    /// 对应 Java: `varData.getByteArray(varMeta.getOffset(tableID, type))`
    fn lookup_column_data(
        &self,
        table_id: i32,
        type_: Option<i32>,
        var_meta: &dyn VarMeta,
        var_data: &Var2Data,
    ) -> Option<Vec<u8>> {
        let type_ = type_?;
        // Java: getOffset 对未登记的 (id, type) 返回 null（Rust 侧为 None）
        var_data.get_byte_array_at_offset(var_meta.get_offset(Some(table_id), Some(type_)))
    }

    /// 把列定义加入表格。
    ///
    /// 对应 Java: `private void processColumnData(ProjectFile file, Table table, byte[] data)`
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
            // Java 在数据不足时抛 `ArrayIndexOutOfBoundsException`（整次读取失败）；
            // Rust 侧结束循环（不 panic），见文件头差异表
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

/// 把文件里的对齐编码映射为 [`Column`] 的常量。
///
/// 对应 Java: `processColumnData` 中的两段
/// `if ((alignment & 0x0F) == 0x00) { ... ALIGN_LEFT } else if (... == 0x01) { ... ALIGN_CENTER } else { ... ALIGN_RIGHT }`。
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

    /// 字段号（MPP9 / MPP12 的字段号体系）。
    fn field_id(field: TaskField) -> i32 {
        FieldTypeHelper::get_field_id(Some(&MppFieldType::Builtin(FieldKey::Task(field))))
    }

    /// 表格定长记录：ID / 名称 / 资源表标志（偏移 108）。
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

    /// 列数据：`[4..6)` = 列数 - 1，之后每列 12 字节（字段号 / 宽度 / 标题偏移 / 对齐），
    /// 标题文本放在列区之后。
    ///
    /// 参数为 `(字段号, 宽度, 标题对齐, 数据对齐, 标题)`。
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

    /// 表格属性与列定义被完整读出。
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
        let table = factory.create_table(&file, fixed.get_byte_array_value(0).expect("有数据"), &*var_meta, &var_data);

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
        // 列标题由数据块中的偏移给出（0 偏移表示未设置 ⇒ 回落到字段名）
        assert_eq!(columns[0].get_title(), Some("Task".to_owned()));

        assert_eq!(columns[1].get_field_type(), Some(FieldKey::Task(TaskField::Cost)));
        assert_eq!(columns[1].get_width(), 10);
        assert_eq!(columns[1].get_align_title(), Column::ALIGN_RIGHT);
        assert_eq!(columns[1].get_align_data(), Column::ALIGN_CENTER);
        // 标题为空串时与 Java 一致地保留空串（`m_title` 非 null 就不再回落字段名）
        assert_eq!(columns[1].get_title(), Some(String::new()));
    }

    /// 列数据的回退顺序：基线 → 企业 → 标准。
    #[test]
    fn column_data_falls_back_in_java_order() {
        let table_fixed = fixed_data(&[table_record(9, "T", true)]);
        let record = table_fixed.get_byte_array_value(0).expect("有数据");

        // 三种键都有数据：优先取基线（键 3）
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

        // 基线键为 null ⇒ 取企业（键 2）
        let file = ProjectFile::new();
        let factory = TableFactory::new(Some(1), Some(2), None);
        let table = factory.create_table(&file, record, &*var_meta, &var_data);
        assert_eq!(table.borrow().get_columns()[0].get_width(), 2);

        // 基线与企业键都为 null ⇒ 取标准（键 1）
        let file = ProjectFile::new();
        let factory = TableFactory::new(Some(1), None, None);
        let table = factory.create_table(&file, record, &*var_meta, &var_data);
        assert_eq!(table.borrow().get_columns()[0].get_width(), 1);
    }

    /// 列数据缺失 / 截断 / 定长数据过短时不 panic。
    #[test]
    fn malformed_data_is_tolerated() {
        let file = ProjectFile::new();
        let factory = TableFactory::new(Some(1), Some(2), Some(3));

        // 变长数据里没有该表格的列数据 ⇒ 只有属性，没有列
        let fixed = fixed_data(&[table_record(1, "T", false)]);
        let (var_meta, var_data) = var_data_with_meta(&[]);
        let table = factory.create_table(
            &file,
            fixed.get_byte_array_value(0).expect("有数据"),
            &*var_meta,
            &var_data,
        );
        assert_eq!(table.borrow().get_id(), 1);
        assert!(table.borrow().get_columns().is_empty());

        // 列数据只有 4 字节（length <= 6）⇒ 直接返回
        let (var_meta, var_data) = var_data_with_meta(&[(1, 3, vec![0u8; 4])]);
        let table = factory.create_table(
            &file,
            fixed.get_byte_array_value(0).expect("有数据"),
            &*var_meta,
            &var_data,
        );
        assert!(table.borrow().get_columns().is_empty());

        // 声明 3 列但数据不足：读满可用列后结束循环
        let mut truncated = vec![0u8; 8 + 12];
        truncated[4..6].copy_from_slice(&2u16.to_le_bytes());
        truncated[8..12].copy_from_slice(&field_id(TaskField::Name).to_le_bytes());
        let (var_meta, var_data) = var_data_with_meta(&[(1, 3, truncated)]);
        let table = factory.create_table(
            &file,
            fixed.get_byte_array_value(0).expect("有数据"),
            &*var_meta,
            &var_data,
        );
        let table = table.borrow();
        assert_eq!(table.get_columns().len(), 1);
        assert_eq!(
            table.get_columns()[0].get_field_type(),
            Some(FieldKey::Task(TaskField::Name))
        );

        // 定长数据不足 4 字节（Java 抛异常）⇒ ID 为 0、名称为空串，不 panic
        let (var_meta, var_data) = var_data_with_meta(&[]);
        let table = factory.create_table(&file, &[0u8; 4], &*var_meta, &var_data);
        assert_eq!(table.borrow().get_id(), 0);
        assert_eq!(table.borrow().get_name(), Some(""));
        assert!(!table.borrow().get_resource_flag());
    }

    /// 对齐编码映射（Java 的 if/else 链）。
    #[test]
    fn alignment_mapping_matches_java() {
        assert_eq!(column_alignment(0x00), Column::ALIGN_LEFT);
        assert_eq!(column_alignment(0x01), Column::ALIGN_CENTER);
        assert_eq!(column_alignment(0x02), Column::ALIGN_RIGHT);
        assert_eq!(column_alignment(0x0F), Column::ALIGN_RIGHT);
    }
}
