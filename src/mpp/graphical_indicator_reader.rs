//! 对应 Java: `org/mpxj/mpp/GraphicalIndicatorReader.java`
//!
//! 从 `Props` 的 `TASK_FIELD_ATTRIBUTES` 数据块里读出「图形化指标」（graphical indicator）
//! 定义：每个字段一段「标志 + 三段行类型各自的判定条件列表」，条件里的左值固定为该字段，
//! 运算符与右值来自数据块。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `ProjectFile file` | `&Rc<RefCell<ProjectFile>>` | 项目文件在本 crate 中以 `Rc<RefCell<..>>` 共享 |
//! | `private byte[] m_data` | 私有方法入参 `data: &[u8]` | 数据只在一次 `process` 内使用；Java 的方法都是私有方法，改成传参不改变公开 API |
//! | `private CustomFieldContainer m_container` | 不保存引用，写回时按需 `file.get_custom_fields()` | 与 Java 一致地写入自定义字段容器（见下） |
//! | `private ProjectProperties m_properties` | 不保存引用，构造 [`GraphicalIndicatorCriteria`](crate::graphical_indicator_criteria::GraphicalIndicatorCriteria) 时用 `Rc::downgrade(file)` | `GenericCriteria` 在本 crate 里以父文件的弱引用做工期换算，见 `crate::generic_criteria` |
//! | `m_data[m_dataOffset]` | `data.get(offset).copied().unwrap_or(0)` | 越界时 Java 抛 `ArrayIndexOutOfBoundsException`，Rust 侧按 0 处理 |
//! | `ByteArrayHelper.getInt/getShort` 越界抛异常 | [`get_int_safe`] / [`get_short_safe`] 返回 0 | 遵循「不 panic」约定 |
//! | `type.getDataType() != null` | `FieldTypeHelper::get_instance(...)` 为 `Some` 时处理 | 本 crate 的 [`MppFieldType::get_data_type`](crate::common::mpp_field_type::MppFieldType::get_data_type) 恒有值（未识别的字段号得到 `None`，与 Java 的占位对象 / `null` 对应） |
//! | `Object` 右值 | [`Operand`]（标量 → `Operand::Value`，字段引用 → `Operand::Field`） | 与 [`GraphicalIndicatorCriteria`](crate::graphical_indicator_criteria::GraphicalIndicatorCriteria) 的右值表示一致 |
//! | `MPPUtility.getAdjustedDuration(...)` 返回 `null` 的情形 | `Option<Duration>` → 右值为 `None` | 见 [`crate::mpp::mpp_utility::MPPUtility::get_adjusted_duration`] |
//!
//! # 解析结果的写回
//!
//! Java 的 `processKnownType` 用
//! `GraphicalIndicator indicator = m_container.getOrCreate(type).getGraphicalIndicator();` 取出
//! **自定义字段容器中已有的那个实例**再往里面填条件，读取器自身不保存任何结果。Rust 侧同样如此：
//! [`crate::custom_field::CustomField`] 已移植 `m_indicator` / `get_graphical_indicator()`，读取器
//! 通过 `file.get_custom_fields()` → [`CustomFieldContainer::get_or_create`] 取到
//! `Rc<RefCell<CustomField>>`，再经
//! [`CustomField::get_graphical_indicator_mut`](crate::custom_field::CustomField::get_graphical_indicator_mut)
//! 写入（同一字段多次出现时沿用同一个实例，与 Java 的 `getOrCreate` 一致）。
//!
//! 唯一的多余分支：父文件正被可变借用时无法取容器，Rust 侧跳过该列的指标定义（Java 无此情形），
//! 与 `crate::mpp::group_reader` 的同类取舍一致。
//!
//! 跳过的 Java 成员：无（全部方法均已翻译）。
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::mpp_field_type::MppFieldType;
use crate::custom_field::CustomField;
use crate::custom_field_container::CustomFieldContainer;
use crate::data_type::DataType;
use crate::field_key::FieldKey;
use crate::field_value::FieldValue;
use crate::graphical_indicator_criteria::GraphicalIndicatorCriteria;
use crate::mpp::abstract_calendar_factory::{get_int_safe, get_short_safe};
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::props::Props;
use crate::mpp::props_key::PropsKey;
use crate::project_file::ProjectFile;
use crate::test_operator::{Operand, TestOperator};

/// 图形化指标定义读取器。
///
/// 对应 Java: `final class GraphicalIndicatorReader`
pub struct GraphicalIndicatorReader {
    /// 列头部的读取位置。
    ///
    /// 对应 Java 字段: `private int m_headerOffset;`
    header_offset: i32,

    /// 当前字段数据块的读取位置。
    ///
    /// 对应 Java 字段: `private int m_dataOffset;`
    data_offset: i32,
}

impl GraphicalIndicatorReader {
    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法（Java 侧未显式声明构造方法）。
    pub fn new() -> Self {
        Self {
            header_offset: 0,
            data_offset: 0,
        }
    }

    /// 主入口：解析全部图形化指标定义。
    ///
    /// 对应 Java: `public void process(ProjectFile file, Props props)`
    pub fn process(&mut self, file: &Rc<RefCell<ProjectFile>>, props: &Props) {
        // Java: m_file = file; m_container = file.getCustomFields(); m_properties = file.getProjectProperties();
        //       m_data = props.getByteArray(PropsKey.TASK_FIELD_ATTRIBUTES);
        // Rust 侧的项目文件 / 数据块都以参数形式使用（见文件头差异表）
        let Some(data) = props.get_byte_array(PropsKey::TaskFieldAttributes) else {
            return;
        };

        // Java: if (m_data != null) { int columnsCount = ByteArrayHelper.getInt(m_data, 4); m_headerOffset = 8;
        //          for (int loop = 0; loop < columnsCount; loop++) { processColumns(); } }
        let columns_count = get_int_safe(data, 4);
        self.header_offset = 8;

        for _ in 0..columns_count {
            self.process_columns(file, data);
        }
    }

    /// 处理单个列的指标定义。
    ///
    /// 对应 Java: `private void processColumns()`
    fn process_columns(&mut self, file: &Rc<RefCell<ProjectFile>>, data: &[u8]) {
        // Java: int fieldID = ByteArrayHelper.getInt(m_data, m_headerOffset); m_headerOffset += 4;
        let field_id = get_int_safe(data, offset(self.header_offset));
        self.header_offset += 4;

        // Java: m_dataOffset = ByteArrayHelper.getInt(m_data, m_headerOffset); m_headerOffset += 4;
        self.data_offset = get_int_safe(data, offset(self.header_offset));
        self.header_offset += 4;

        // Java: FieldType type = FieldTypeHelper.getInstance(m_file, fieldID);
        //       if (type.getDataType() != null) { processKnownType(type); }
        //（未识别的字段号在 Java 里是占位对象、`getDataType()` 为 `null` ⇒ 跳过；
        //  Rust 侧 `getInstance` 返回 `None` ⇒ 同样跳过）
        if let Some(type_) = FieldTypeHelper::get_instance(file, field_id) {
            self.process_known_type(file, data, &type_);
        }
    }

    /// 处理已知字段类型的指标定义。
    ///
    /// 对应 Java: `private void processKnownType(FieldType type)`
    fn process_known_type(
        &mut self,
        file: &Rc<RefCell<ProjectFile>>,
        data: &[u8],
        type_: &MppFieldType,
    ) {
        // Java: int flags = m_data[m_dataOffset];
        //       if ((flags & 0x02) == 0) { return; }
        let flags = data.get(offset(self.data_offset)).copied().unwrap_or(0);
        if (flags & 0x02) == 0 {
            return;
        }

        // Java: GraphicalIndicator indicator = m_container.getOrCreate(type).getGraphicalIndicator();
        //       indicator.setProjectSummaryInheritsFromSummaryRows((flags & 0x08) != 0);
        //       indicator.setSummaryRowsInheritFromNonSummaryRows((flags & 0x04) != 0);
        //       indicator.setDisplayGraphicalIndicators((flags & 0x02) != 0);
        //       indicator.setShowDataValuesInToolTips((flags & 0x01) != 0);
        let field_key = type_.to_field_key();
        let Some(custom_field) = self.custom_field(file, field_key) else {
            // Rust 特有：父文件正被可变借用时取不到容器（Java 无此情形）⇒ 跳过该列
            return;
        };

        {
            // Java 的 `getOrCreate` 语义：同一字段取出的是同一个 `CustomField`，
            // 因此填写的也正是它构造时创建的那个 `GraphicalIndicator` 实例
            let mut field = custom_field.borrow_mut();
            let indicator = field.get_graphical_indicator_mut();
            indicator.set_project_summary_inherits_from_summary_rows((flags & 0x08) != 0);
            indicator.set_summary_rows_inherit_from_non_summary_rows((flags & 0x04) != 0);
            indicator.set_display_graphical_indicators((flags & 0x02) != 0);
            indicator.set_show_data_values_in_tool_tips((flags & 0x01) != 0);
        }

        // Java: m_dataOffset += 20;
        self.data_offset += 20;

        // Java: int nonSummaryRowOffset = ByteArrayHelper.getInt(m_data, m_dataOffset) - 36; m_dataOffset += 4;
        let non_summary_row_offset = get_int_safe(data, offset(self.data_offset)) - 36;
        self.data_offset += 4;

        // Java: int summaryRowOffset = ByteArrayHelper.getInt(m_data, m_dataOffset) - 36; m_dataOffset += 4;
        let summary_row_offset = get_int_safe(data, offset(self.data_offset)) - 36;
        self.data_offset += 4;

        // Java: int projectSummaryOffset = ByteArrayHelper.getInt(m_data, m_dataOffset) - 36; m_dataOffset += 4;
        let project_summary_offset = get_int_safe(data, offset(self.data_offset)) - 36;
        self.data_offset += 4;

        // Java: int dataSize = ByteArrayHelper.getInt(m_data, m_dataOffset) - 36; m_dataOffset += 4;
        let data_size = get_int_safe(data, offset(self.data_offset)) - 36;
        self.data_offset += 4;

        // Java: int maxNonSummaryRowOffset = m_dataOffset + summaryRowOffset;
        //       int maxSummaryRowOffset = m_dataOffset + projectSummaryOffset;
        //       int maxProjectSummaryOffset = m_dataOffset + dataSize;
        //       m_dataOffset += nonSummaryRowOffset;
        let max_non_summary_row_offset = self.data_offset + summary_row_offset;
        let max_summary_row_offset = self.data_offset + project_summary_offset;
        let max_project_summary_offset = self.data_offset + data_size;
        self.data_offset += non_summary_row_offset;

        // Java: while (m_dataOffset + 2 < maxNonSummaryRowOffset) { indicator.addNonSummaryRowCriteria(processCriteria(type)); }
        while self.data_offset + 2 < max_non_summary_row_offset {
            let criteria = self.process_criteria(file, data, type_);
            custom_field
                .borrow_mut()
                .get_graphical_indicator_mut()
                .add_non_summary_row_criteria(criteria);
        }

        // Java: while (m_dataOffset + 2 < maxSummaryRowOffset) { indicator.addSummaryRowCriteria(processCriteria(type)); }
        while self.data_offset + 2 < max_summary_row_offset {
            let criteria = self.process_criteria(file, data, type_);
            custom_field
                .borrow_mut()
                .get_graphical_indicator_mut()
                .add_summary_row_criteria(criteria);
        }

        // Java: while (m_dataOffset + 2 < maxProjectSummaryOffset) { indicator.addProjectSummaryCriteria(processCriteria(type)); }
        while self.data_offset + 2 < max_project_summary_offset {
            let criteria = self.process_criteria(file, data, type_);
            custom_field
                .borrow_mut()
                .get_graphical_indicator_mut()
                .add_project_summary_criteria(criteria);
        }
    }

    /// 取（不存在时创建）指定字段的自定义字段配置。
    ///
    /// 对应 Java: `m_container.getOrCreate(type)`，其中
    /// `m_container = file.getCustomFields()`（Java 在 `process` 开头保存该引用，Rust 侧
    /// 不保存、按需借用）。
    ///
    /// 父文件正被可变借用时返回 `None`（Java 无此情形），由调用方跳过该列。
    fn custom_field(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        field: FieldKey,
    ) -> Option<Rc<RefCell<CustomField>>> {
        let container = file.try_borrow().ok()?.get_custom_fields();

        Some(CustomFieldContainer::get_or_create(&container, field))
    }

    /// 处理单条指标判定条件。
    ///
    /// 对应 Java: `private GraphicalIndicatorCriteria processCriteria(FieldType type)`
    fn process_criteria(
        &mut self,
        file: &Rc<RefCell<ProjectFile>>,
        data: &[u8],
        type_: &MppFieldType,
    ) -> GraphicalIndicatorCriteria {
        // Java: GraphicalIndicatorCriteria criteria = new GraphicalIndicatorCriteria(m_properties);
        //       criteria.setLeftValue(type);
        let mut criteria = GraphicalIndicatorCriteria::new(Rc::downgrade(file));
        criteria.set_left_value(Some(type_.to_field_key()));

        // Java: int indicatorType = ByteArrayHelper.getInt(m_data, m_dataOffset); m_dataOffset += 4;
        //       criteria.setIndicator(indicatorType);
        criteria.set_indicator(get_int_safe(data, offset(self.data_offset)));
        self.data_offset += 4;

        // Java: if (m_dataOffset + 4 < m_data.length) { ... }
        if self.data_offset + 4 < data.len() as i32 {
            // Java: int operatorValue = ByteArrayHelper.getInt(m_data, m_dataOffset); m_dataOffset += 4;
            //       TestOperator operator = (operatorValue == 0 ? TestOperator.IS_ANY_VALUE : TestOperator.getInstance(operatorValue - 0x3E7));
            let operator_value = get_int_safe(data, offset(self.data_offset));
            self.data_offset += 4;
            let operator = if operator_value == 0 {
                TestOperator::IsAnyValue
            } else {
                TestOperator::get_instance(operator_value - 0x3E7)
            };
            criteria.set_operator(Some(operator));

            if operator != TestOperator::IsAnyValue {
                // Java: processOperandValue(0, type, criteria);
                self.process_operand_value(0, file, data, type_, &mut criteria);

                // Java: if (operator == IS_WITHIN || operator == IS_NOT_WITHIN) { processOperandValue(1, type, criteria); }
                if operator == TestOperator::IsWithin || operator == TestOperator::IsNotWithin {
                    self.process_operand_value(1, file, data, type_, &mut criteria);
                }
            }
        }

        criteria
    }

    /// 处理条件中的一个操作数取值。
    ///
    /// 对应 Java: `private void processOperandValue(int index, FieldType type, GraphicalIndicatorCriteria criteria)`
    fn process_operand_value(
        &mut self,
        index: usize,
        file: &Rc<RefCell<ProjectFile>>,
        data: &[u8],
        type_: &MppFieldType,
        criteria: &mut GraphicalIndicatorCriteria,
    ) {
        // Java: boolean valueFlag = (ByteArrayHelper.getInt(m_data, m_dataOffset) == 1); m_dataOffset += 4;
        let value_flag = get_int_safe(data, offset(self.data_offset)) == 1;
        self.data_offset += 4;

        if !value_flag {
            // Java: int fieldID = ByteArrayHelper.getInt(m_data, m_dataOffset);
            //       criteria.setRightValue(index, FieldTypeHelper.getInstance(m_file, fieldID));
            //       m_dataOffset += 4;
            let field_id = get_int_safe(data, offset(self.data_offset));
            self.data_offset += 4;
            criteria.set_right_value(
                index,
                FieldTypeHelper::get_instance(file, field_id)
                    .map(|field| Operand::Field(field.to_field_key())),
            );
            return;
        }

        // Java: m_dataOffset += 2;（跳过数据类型值，Java 中已注释掉读取）
        self.data_offset += 2;

        match type_.get_data_type() {
            // Java: case DURATION:
            //   Duration value = MPPUtility.getAdjustedDuration(m_properties, ByteArrayHelper.getInt(m_data, m_dataOffset),
            //       MPPUtility.getDurationTimeUnits(ByteArrayHelper.getShort(m_data, m_dataOffset + 4)));
            //   m_dataOffset += 6;
            DataType::Duration => {
                let value = get_int_safe(data, offset(self.data_offset));
                let units = MPPUtility::get_duration_time_units(get_short_safe(
                    data,
                    offset(self.data_offset + 4),
                ));
                self.data_offset += 6;

                let duration = {
                    // Java: m_properties = file.getProjectProperties()；Rust 侧按需借用
                    match file.try_borrow() {
                        Ok(project) => MPPUtility::get_adjusted_duration(
                            project.get_project_properties(),
                            value,
                            units,
                        ),
                        Err(_) => None,
                    }
                };
                criteria.set_right_value(
                    index,
                    duration.map(|duration| Operand::Value(FieldValue::Duration(duration))),
                );
            }

            // Java: case NUMERIC: Double value = Double.valueOf(MPPUtility.getDouble(m_data, m_dataOffset)); m_dataOffset += 8;
            DataType::Numeric => {
                let value = MPPUtility::get_double(data, offset(self.data_offset));
                self.data_offset += 8;
                criteria.set_right_value(index, Some(Operand::Value(FieldValue::Double(value))));
            }

            // Java: case CURRENCY: Double value = Double.valueOf(MPPUtility.getDouble(m_data, m_dataOffset) / 100); m_dataOffset += 8;
            DataType::Currency => {
                let value = MPPUtility::get_double(data, offset(self.data_offset)) / 100.0;
                self.data_offset += 8;
                criteria.set_right_value(index, Some(Operand::Value(FieldValue::Double(value))));
            }

            // Java: case STRING: String value = MPPUtility.getUnicodeString(m_data, m_dataOffset); m_dataOffset += ((value.length() + 1) * 2);
            DataType::String => {
                let value = MPPUtility::get_unicode_string(data, offset(self.data_offset));
                // Java 的 `String.length()` 是 UTF-16 码元数
                let units = value.encode_utf16().count() + 1;
                self.data_offset += (units * 2) as i32;
                criteria.set_right_value(index, Some(Operand::Value(FieldValue::String(value))));
            }

            // Java: case BOOLEAN: int value = ByteArrayHelper.getShort(m_data, m_dataOffset); m_dataOffset += 2;
            //       criteria.setRightValue(index, value == 1 ? Boolean.TRUE : Boolean.FALSE);
            DataType::Boolean => {
                let value = get_short_safe(data, offset(self.data_offset));
                self.data_offset += 2;
                criteria.set_right_value(
                    index,
                    Some(Operand::Value(FieldValue::Boolean(value == 1))),
                );
            }

            // Java: case DATE: LocalDateTime value = MPPUtility.getTimestamp(m_data, m_dataOffset); m_dataOffset += 4;
            DataType::Date => {
                let value = MPPUtility::get_timestamp(data, offset(self.data_offset));
                self.data_offset += 4;
                criteria.set_right_value(
                    index,
                    value.map(|value| Operand::Value(FieldValue::DateTime(value))),
                );
            }

            // Java: default: break;（不读取任何数据）
            _ => {}
        }
    }
}

impl Default for GraphicalIndicatorReader {
    /// 对应 Java: 隐式默认构造方法，等价于 [`GraphicalIndicatorReader::new`]。
    fn default() -> Self {
        Self::new()
    }
}

/// 把 Java 的 `int` 偏移转成切片下标（负数按 0 处理）。
fn offset(value: i32) -> usize {
    usize::try_from(value).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duration::Duration;
    use crate::task_field::TaskField;
    use crate::time_unit::TimeUnit;

    /// 字段号。
    fn field_id(field: TaskField) -> i32 {
        FieldTypeHelper::get_field_id(Some(&MppFieldType::Builtin(FieldKey::Task(field))))
    }

    /// 构造 `TASK_FIELD_ATTRIBUTES` 数据块：
    ///
    /// ```text
    /// [0..4)   保留
    /// [4..8)   列数
    /// [8..12)  字段号
    /// [12..16) 字段数据块的绝对偏移（16）
    /// 16       标志字节
    /// 36..52   四段区间的偏移（Java 侧各自减 36）
    /// 52..     条件记录
    /// ```
    ///
    /// `criteria` 为条件区的内容；三段行类型的区间依次为
    /// `[0, criteria.len())`、`[len, len)`、`[len, len)`（即只有非摘要行有条件）。
    fn field_attributes(flags: u8, field: TaskField, criteria: &[u8]) -> Vec<u8> {
        let base = 52usize;
        let mut data = vec![0u8; base + criteria.len()];

        data[4..8].copy_from_slice(&1i32.to_le_bytes());
        data[8..12].copy_from_slice(&field_id(field).to_le_bytes());
        data[12..16].copy_from_slice(&16i32.to_le_bytes());

        data[16] = flags;
        // 36 起：非摘要行起点（相对）为 0；摘要行 / 项目摘要行的终点都等于条件区长度
        data[36..40].copy_from_slice(&(36i32).to_le_bytes());
        data[40..44].copy_from_slice(&(36i32 + criteria.len() as i32).to_le_bytes());
        data[44..48].copy_from_slice(&(36i32 + criteria.len() as i32).to_le_bytes());
        data[48..52].copy_from_slice(&(36i32 + criteria.len() as i32).to_le_bytes());

        data[base..].copy_from_slice(criteria);
        data
    }

    /// 取文件中某字段的自定义字段配置（未创建时返回 `None`）。
    ///
    /// 对应 Java: `file.getCustomFields().get(field)`。
    fn custom_field(
        file: &Rc<RefCell<ProjectFile>>,
        field: FieldKey,
    ) -> Option<Rc<RefCell<CustomField>>> {
        file.borrow().get_custom_fields().borrow().get(field)
    }

    /// 文件中已登记的自定义字段数量。
    ///
    /// 对应 Java: `file.getCustomFields().size()`。
    fn custom_field_count(file: &Rc<RefCell<ProjectFile>>) -> usize {
        file.borrow().get_custom_fields().borrow().size()
    }

    /// 一条「工期 > 8 小时」的条件记录。
    fn duration_criteria() -> Vec<u8> {
        let mut data = Vec::new();
        // 指标号
        data.extend_from_slice(&7i32.to_le_bytes());
        // 运算符：EQUALS（0x3E7 + 6）
        data.extend_from_slice(&(0x3E7 + 6i32).to_le_bytes());
        // 操作数：常量标志
        data.extend_from_slice(&1i32.to_le_bytes());
        // 跳过的数据类型值
        data.extend_from_slice(&3i16.to_le_bytes());
        // 工期：4800 个十分之一分钟 = 8 小时；单位 5 = Hours
        data.extend_from_slice(&4800i32.to_le_bytes());
        data.extend_from_slice(&5i16.to_le_bytes());
        data
    }

    /// 指标定义被完整读出（四个标志 + 非摘要行条件）。
    #[test]
    fn process_reads_indicator_definition() {
        let mut props = Props::new();
        props.put(
            PropsKey::TaskFieldAttributes.get_value(),
            field_attributes(0x0F, TaskField::Duration, &duration_criteria()),
        );

        let file = ProjectFile::new();
        let field = FieldKey::Task(TaskField::Duration);

        // Java: m_container.getOrCreate(type) —— 先登记配置，读取器应填充同一个实例
        let container = file.borrow().get_custom_fields();
        let existing = CustomFieldContainer::get_or_create(&container, field);
        drop(container);

        let mut reader = GraphicalIndicatorReader::new();
        reader.process(&file, &props);

        assert_eq!(custom_field_count(&file), 1);

        let populated = custom_field(&file, field).expect("应有自定义字段配置");
        assert!(Rc::ptr_eq(&existing, &populated), "应填充同一个 CustomField 实例");

        let indicator = populated.borrow();
        let indicator = indicator.get_graphical_indicator();
        assert!(indicator.get_project_summary_inherits_from_summary_rows());
        assert!(indicator.get_summary_rows_inherit_from_non_summary_rows());
        assert!(indicator.get_display_graphical_indicators());
        assert!(indicator.get_show_data_values_in_tool_tips());

        // 只有非摘要行有条件
        assert_eq!(indicator.get_non_summary_row_criteria().len(), 1);
        assert!(indicator.get_summary_row_criteria().is_empty());
        assert!(indicator.get_project_summary_criteria().is_empty());

        let criteria = &indicator.get_non_summary_row_criteria()[0];
        assert_eq!(criteria.get_indicator(), 7);
        assert_eq!(criteria.get_left_value(), Some(field));
        assert_eq!(criteria.get_operator(), Some(TestOperator::Equals));
        assert_eq!(
            criteria.get_value(0),
            Some(&Operand::Value(FieldValue::Duration(Duration::get_instance(
                8.0,
                TimeUnit::Hours
            ))))
        );
        // 非 IS_WITHIN 运算符不读取第二个操作数
        assert_eq!(criteria.get_value(1), None);

        // 未知字段没有指标定义
        assert!(custom_field(&file, FieldKey::Task(TaskField::Name)).is_none());
    }

    /// 标志位为 0 / 不含 0x02 时不建立指标定义。
    #[test]
    fn flags_without_indicator_are_skipped() {
        let mut props = Props::new();
        props.put(
            PropsKey::TaskFieldAttributes.get_value(),
            field_attributes(0x01, TaskField::Duration, &[]),
        );

        let file = ProjectFile::new();
        let mut reader = GraphicalIndicatorReader::new();
        reader.process(&file, &props);

        // Java: 标志位不含 0x02 时在 getOrCreate 之前就返回，容器不被触碰
        assert_eq!(custom_field_count(&file), 0);
    }

    /// IS_ANY_VALUE（运算符值为 0）不读取任何操作数。
    #[test]
    fn any_value_operator_has_no_operand() {
        let mut criteria = Vec::new();
        criteria.extend_from_slice(&3i32.to_le_bytes());
        criteria.extend_from_slice(&0i32.to_le_bytes());
        // Java 判定是否读取运算符的条件是 `m_dataOffset + 4 < m_data.length`，
        // 故在其后补一个字节，保证运算符能被读到（数据块尾部通常还有别的列数据）
        criteria.push(0);

        let mut props = Props::new();
        props.put(
            PropsKey::TaskFieldAttributes.get_value(),
            field_attributes(0x02, TaskField::Duration, &criteria),
        );

        let file = ProjectFile::new();
        let mut reader = GraphicalIndicatorReader::new();
        reader.process(&file, &props);

        let populated = custom_field(&file, FieldKey::Task(TaskField::Duration))
            .expect("应有自定义字段配置");
        let indicator = populated.borrow();
        let indicator = indicator.get_graphical_indicator();
        assert!(indicator.get_display_graphical_indicators());
        assert!(!indicator.get_show_data_values_in_tool_tips());
        let criteria = &indicator.get_non_summary_row_criteria()[0];
        assert_eq!(criteria.get_indicator(), 3);
        assert_eq!(criteria.get_operator(), Some(TestOperator::IsAnyValue));
        assert_eq!(criteria.get_value(0), None);
    }

    /// 操作数为字段引用时不读取内联取值。
    #[test]
    fn field_reference_operand() {
        let mut criteria = Vec::new();
        criteria.extend_from_slice(&1i32.to_le_bytes());
        criteria.extend_from_slice(&(0x3E7 + 6i32).to_le_bytes());
        // 常量标志为 0 ⇒ 字段引用
        criteria.extend_from_slice(&0i32.to_le_bytes());
        criteria.extend_from_slice(&field_id(TaskField::Duration).to_le_bytes());

        let mut props = Props::new();
        props.put(
            PropsKey::TaskFieldAttributes.get_value(),
            field_attributes(0x02, TaskField::Duration, &criteria),
        );

        let file = ProjectFile::new();
        let mut reader = GraphicalIndicatorReader::new();
        reader.process(&file, &props);

        let populated = custom_field(&file, FieldKey::Task(TaskField::Duration))
            .expect("应有自定义字段配置");
        let indicator = populated.borrow();
        assert_eq!(
            indicator
                .get_graphical_indicator()
                .get_non_summary_row_criteria()[0]
                .get_value(0),
            Some(&Operand::Field(FieldKey::Task(TaskField::Duration)))
        );
    }

    /// 数据缺失 / 截断 / 无该属性时不 panic。
    #[test]
    fn malformed_data_is_tolerated() {
        let file = ProjectFile::new();

        // 完全没有 TASK_FIELD_ATTRIBUTES
        let mut reader = GraphicalIndicatorReader::new();
        reader.process(&file, &Props::new());
        assert_eq!(custom_field_count(&file), 0);

        // 只有 4 字节（读不出列数）
        let mut props = Props::new();
        props.put(PropsKey::TaskFieldAttributes.get_value(), vec![0u8; 4]);
        let mut reader = GraphicalIndicatorReader::new();
        reader.process(&file, &props);
        assert_eq!(custom_field_count(&file), 0);

        // 列数为 0
        let mut block = vec![0u8; 8];
        block[4..8].copy_from_slice(&0i32.to_le_bytes());
        let mut props = Props::new();
        props.put(PropsKey::TaskFieldAttributes.get_value(), block);
        let mut reader = GraphicalIndicatorReader::new();
        reader.process(&file, &props);
        assert_eq!(custom_field_count(&file), 0);

        // 列数据偏移越界：标志字节按 0 处理 ⇒ 跳过
        let mut block = vec![0u8; 16];
        block[4..8].copy_from_slice(&1i32.to_le_bytes());
        block[8..12].copy_from_slice(&field_id(TaskField::Duration).to_le_bytes());
        block[12..16].copy_from_slice(&9999i32.to_le_bytes());
        let mut props = Props::new();
        props.put(PropsKey::TaskFieldAttributes.get_value(), block);
        let mut reader = GraphicalIndicatorReader::new();
        reader.process(&file, &props);
        assert_eq!(custom_field_count(&file), 0);

        // 未识别的字段号 ⇒ 跳过
        let mut block = field_attributes(0x0F, TaskField::Duration, &duration_criteria());
        block[8..12].copy_from_slice(&0x1234_0005i32.to_le_bytes());
        let mut props = Props::new();
        props.put(PropsKey::TaskFieldAttributes.get_value(), block);
        let mut reader = GraphicalIndicatorReader::new();
        reader.process(&file, &props);
        assert_eq!(custom_field_count(&file), 0);

        // 条件区被截断（区间声明长于实际数据）⇒ 循环按 +2 收敛，不 panic
        let mut props = Props::new();
        props.put(
            PropsKey::TaskFieldAttributes.get_value(),
            field_attributes(0x02, TaskField::Duration, &[0u8; 3]),
        );
        let mut reader = GraphicalIndicatorReader::new();
        reader.process(&file, &props);
        let populated = custom_field(&file, FieldKey::Task(TaskField::Duration))
            .expect("应有自定义字段配置");
        assert_eq!(
            populated
                .borrow()
                .get_graphical_indicator()
                .get_non_summary_row_criteria()
                .len(),
            1
        );
    }

    /// 默认构造与 `new` 等价。
    #[test]
    fn default_matches_new() {
        let file = ProjectFile::new();
        let mut reader = GraphicalIndicatorReader::default();
        reader.process(&file, &Props::new());
        assert_eq!(custom_field_count(&file), 0);
    }
}
