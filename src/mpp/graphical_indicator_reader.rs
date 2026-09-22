//! Corresponds to Java: `org/mpxj/mpp/GraphicalIndicatorReader.java`
//!
//! Reads "graphical indicators" from the `TASK_FIELD_ATTRIBUTES` data block of `Props`
//! Definitions: one entry per field consisting of "a flag + a list of criteria for each of the three row types"; the left-hand value of each criterion is fixed to that field,
//! while the operator and right-hand value come from the data block.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `ProjectFile file` | `&Rc<RefCell<ProjectFile>>` | the project file is shared as `Rc<RefCell<..>>` in this crate |
//! | `private byte[] m_data` | private method parameter `data: &[u8]` | the data is used only within a single `process` call; Java's methods are all private, so passing it as a parameter does not change the public API |
//! | `private CustomFieldContainer m_container` | no reference is stored; `file.get_custom_fields()` is called on demand when writing back | writes to the custom field container consistently with Java (see below) |
//! | `private ProjectProperties m_properties` | no reference is stored; `Rc::downgrade(file)` is used when constructing [`GraphicalIndicatorCriteria`](crate::graphical_indicator_criteria::GraphicalIndicatorCriteria) | in this crate `GenericCriteria` uses a weak reference to the parent file for duration conversion, see `crate::generic_criteria` |
//! | `m_data[m_dataOffset]` | `data.get(offset).copied().unwrap_or(0)` | on out-of-bounds Java throws `ArrayIndexOutOfBoundsException`; the Rust side treats it as 0 |
//! | `ByteArrayHelper.getInt/getShort` throws on out-of-bounds | [`get_int_safe`] / [`get_short_safe`] return 0 | follows the "never panic" convention |
//! | `type.getDataType() != null` | processed when `FieldTypeHelper::get_instance(...)` is `Some` | this crate's [`MppFieldType::get_data_type`](crate::common::mpp_field_type::MppFieldType::get_data_type) always has a value (an unrecognised field number yields `None`, corresponding to Java's placeholder object / `null`) |
//! | `Object` right-hand value | [`Operand`] (scalar → `Operand::Value`, field reference → `Operand::Field`) | consistent with the right-hand value representation of [`GraphicalIndicatorCriteria`](crate::graphical_indicator_criteria::GraphicalIndicatorCriteria) |
//! | the case where `MPPUtility.getAdjustedDuration(...)` returns `null` | `Option<Duration>` → the right-hand value is `None` | see [`crate::mpp::mpp_utility::MPPUtility::get_adjusted_duration`] |
//!
//! # Writing back the parsed results
//!
//! Java's `processKnownType` uses
//! `GraphicalIndicator indicator = m_container.getOrCreate(type).getGraphicalIndicator();` to retrieve
//! **the existing instance in the custom field container** and fills criteria into it; the reader itself stores no results. The Rust side does the same:
//! [`crate::custom_field::CustomField`] already ports `m_indicator` / `get_graphical_indicator()`; the reader
//! obtains, via `file.get_custom_fields()` → [`CustomFieldContainer::get_or_create`],
//! the `Rc<RefCell<CustomField>>`, then
//! [`CustomField::get_graphical_indicator_mut`](crate::custom_field::CustomField::get_graphical_indicator_mut)
//! writes into it (the same instance is reused when a field appears multiple times, consistent with Java's `getOrCreate`).
//!
//! The only extra branch: when the parent file is currently mutably borrowed the container cannot be obtained, so the Rust side skips that column's indicator definitions (no such case in Java),
//! consistent with the same kind of trade-off in `crate::mpp::group_reader`.
//!
//! Skipped Java members: none (all methods have been translated).
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

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

/// Graphical indicator definition reader.
///
/// Corresponds to Java: `final class GraphicalIndicatorReader`
pub struct GraphicalIndicatorReader {
/// Read position of the column header.
    ///
    /// Corresponds to Java field: `private int m_headerOffset;`
    header_offset: i32,

/// Read position of the current field data block.
    ///
    /// Corresponds to Java field: `private int m_dataOffset;`
    data_offset: i32,
}

impl GraphicalIndicatorReader {
/// Constructor.
    ///
/// Corresponds to Java: implicit default constructor (Java does not declare a constructor explicitly).
    pub fn new() -> Self {
        Self {
            header_offset: 0,
            data_offset: 0,
        }
    }

/// Main entry point: parse all graphical indicator definitions.
    ///
    /// Corresponds to Java: `public void process(ProjectFile file, Props props)`
    pub fn process(&mut self, file: &Rc<RefCell<ProjectFile>>, props: &Props) {
        // Java: m_file = file; m_container = file.getCustomFields(); m_properties = file.getProjectProperties();
        //       m_data = props.getByteArray(PropsKey.TASK_FIELD_ATTRIBUTES);
// On the Rust side the project file / data block are used as parameters (see the differences table in the file header)
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

/// Process the indicator definitions of a single column.
    ///
    /// Corresponds to Java: `private void processColumns()`
    fn process_columns(&mut self, file: &Rc<RefCell<ProjectFile>>, data: &[u8]) {
        // Java: int fieldID = ByteArrayHelper.getInt(m_data, m_headerOffset); m_headerOffset += 4;
        let field_id = get_int_safe(data, offset(self.header_offset));
        self.header_offset += 4;

        // Java: m_dataOffset = ByteArrayHelper.getInt(m_data, m_headerOffset); m_headerOffset += 4;
        self.data_offset = get_int_safe(data, offset(self.header_offset));
        self.header_offset += 4;

        // Java: FieldType type = FieldTypeHelper.getInstance(m_file, fieldID);
        //       if (type.getDataType() != null) { processKnownType(type); }
// (an unrecognised field number is a placeholder object in Java with `getDataType()` equal to `null` ⇒ skip;
// the Rust side's `getInstance` returns `None` ⇒ likewise skip)
        if let Some(type_) = FieldTypeHelper::get_instance(file, field_id) {
            self.process_known_type(file, data, &type_);
        }
    }

/// Process the indicator definitions of a known field type.
    ///
    /// Corresponds to Java: `private void processKnownType(FieldType type)`
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
// Rust-specific: when the parent file is currently mutably borrowed the container cannot be obtained (no such case in Java) ⇒ skip that column
            return;
        };

        {
// Java's `getOrCreate` semantics: the same field yields the same `CustomField`,
// so what is filled in is exactly the `GraphicalIndicator` instance created when it was constructed
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

/// Get (creating if absent) the custom field configuration for the specified field.
    ///
/// Corresponds to Java: `m_container.getOrCreate(type)`, where
/// `m_container = file.getCustomFields()` (Java stores this reference at the start of `process`; the Rust side
/// does not store it and borrows it on demand).
    ///
/// Returns `None` when the parent file is currently mutably borrowed (no such case in Java); the caller skips that column.
    fn custom_field(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        field: FieldKey,
    ) -> Option<Rc<RefCell<CustomField>>> {
        let container = file.try_borrow().ok()?.get_custom_fields();

        Some(CustomFieldContainer::get_or_create(&container, field))
    }

/// Process a single indicator criterion.
    ///
    /// Corresponds to Java: `private GraphicalIndicatorCriteria processCriteria(FieldType type)`
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

/// Process one operand value in a criterion.
    ///
    /// Corresponds to Java: `private void processOperandValue(int index, FieldType type, GraphicalIndicatorCriteria criteria)`
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

// Java: m_dataOffset += 2; (skip the data type value; the read is commented out in Java)
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
// Java: m_properties = file.getProjectProperties(); the Rust side borrows on demand
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
// Java's `String.length()` is the number of UTF-16 code units
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

// Java: default: break; (does not read any data)
            _ => {}
        }
    }
}

impl Default for GraphicalIndicatorReader {
/// Corresponds to Java: implicit default constructor, equivalent to [`GraphicalIndicatorReader::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a Java `int` offset to a slice index (negative values are treated as 0).
fn offset(value: i32) -> usize {
    usize::try_from(value).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duration::Duration;
    use crate::task_field::TaskField;
    use crate::time_unit::TimeUnit;

/// Field number.
    fn field_id(field: TaskField) -> i32 {
        FieldTypeHelper::get_field_id(Some(&MppFieldType::Builtin(FieldKey::Task(field))))
    }

/// Construct a `TASK_FIELD_ATTRIBUTES` data block:
    ///
    /// ```text
/// [0..4)   reserved
/// [4..8)   column count
/// [8..12)  field number
/// [12..16) absolute offset of the field data block (16)
/// 16       flag byte
/// 36..52   offsets of the four ranges (Java subtracts 36 from each)
/// 52..     criteria records
    /// ```
    ///
/// `criteria` is the content of the criteria area; the ranges of the three row types are, in order,
/// `[0, criteria.len())`, `[len, len)`, `[len, len)` (i.e. only non-summary rows have criteria).
    fn field_attributes(flags: u8, field: TaskField, criteria: &[u8]) -> Vec<u8> {
        let base = 52usize;
        let mut data = vec![0u8; base + criteria.len()];

        data[4..8].copy_from_slice(&1i32.to_le_bytes());
        data[8..12].copy_from_slice(&field_id(field).to_le_bytes());
        data[12..16].copy_from_slice(&16i32.to_le_bytes());

        data[16] = flags;
// From 36: the start (relative) of non-summary rows is 0; the end of both summary rows and project summary rows equals the criteria area length
        data[36..40].copy_from_slice(&(36i32).to_le_bytes());
        data[40..44].copy_from_slice(&(36i32 + criteria.len() as i32).to_le_bytes());
        data[44..48].copy_from_slice(&(36i32 + criteria.len() as i32).to_le_bytes());
        data[48..52].copy_from_slice(&(36i32 + criteria.len() as i32).to_le_bytes());

        data[base..].copy_from_slice(criteria);
        data
    }

/// Get the custom field configuration for a field in the file (returns `None` if not created).
    ///
    /// Corresponds to Java: `file.getCustomFields().get(field)`.
    fn custom_field(
        file: &Rc<RefCell<ProjectFile>>,
        field: FieldKey,
    ) -> Option<Rc<RefCell<CustomField>>> {
        file.borrow().get_custom_fields().borrow().get(field)
    }

/// The number of custom fields registered in the file.
    ///
    /// Corresponds to Java: `file.getCustomFields().size()`.
    fn custom_field_count(file: &Rc<RefCell<ProjectFile>>) -> usize {
        file.borrow().get_custom_fields().borrow().size()
    }

/// A criterion record for "duration > 8 hours".
    fn duration_criteria() -> Vec<u8> {
        let mut data = Vec::new();
// indicator number
        data.extend_from_slice(&7i32.to_le_bytes());
// operator: EQUALS (0x3E7 + 6)
        data.extend_from_slice(&(0x3E7 + 6i32).to_le_bytes());
// operand: constant flag
        data.extend_from_slice(&1i32.to_le_bytes());
// skipped data type value
        data.extend_from_slice(&3i16.to_le_bytes());
// duration: 4800 tenths of a minute = 8 hours; unit 5 = Hours
        data.extend_from_slice(&4800i32.to_le_bytes());
        data.extend_from_slice(&5i16.to_le_bytes());
        data
    }

/// The indicator definition is read out in full (four flags + non-summary row criteria).
    #[test]
    fn process_reads_indicator_definition() {
        let mut props = Props::new();
        props.put(
            PropsKey::TaskFieldAttributes.get_value(),
            field_attributes(0x0F, TaskField::Duration, &duration_criteria()),
        );

        let file = ProjectFile::new();
        let field = FieldKey::Task(TaskField::Duration);

// Java: m_container.getOrCreate(type) — first register the configuration; the reader should fill the same instance
        let container = file.borrow().get_custom_fields();
        let existing = CustomFieldContainer::get_or_create(&container, field);
        drop(container);

        let mut reader = GraphicalIndicatorReader::new();
        reader.process(&file, &props);

        assert_eq!(custom_field_count(&file), 1);

        let populated = custom_field(&file, field).expect("should have custom field configuration");
        assert!(Rc::ptr_eq(&existing, &populated), "should populate the same CustomField instance");

        let indicator = populated.borrow();
        let indicator = indicator.get_graphical_indicator();
        assert!(indicator.get_project_summary_inherits_from_summary_rows());
        assert!(indicator.get_summary_rows_inherit_from_non_summary_rows());
        assert!(indicator.get_display_graphical_indicators());
        assert!(indicator.get_show_data_values_in_tool_tips());

// only non-summary rows have criteria
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
// operators other than IS_WITHIN do not read a second operand
        assert_eq!(criteria.get_value(1), None);

// unknown fields have no indicator definition
        assert!(custom_field(&file, FieldKey::Task(TaskField::Name)).is_none());
    }

/// No indicator definition is created when the flags are 0 / do not contain 0x02.
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

// Java: when the flags do not contain 0x02 it returns before getOrCreate, so the container is not touched
        assert_eq!(custom_field_count(&file), 0);
    }

/// IS_ANY_VALUE (operator value 0) does not read any operand.
    #[test]
    fn any_value_operator_has_no_operand() {
        let mut criteria = Vec::new();
        criteria.extend_from_slice(&3i32.to_le_bytes());
        criteria.extend_from_slice(&0i32.to_le_bytes());
// Java's condition for deciding whether to read the operator is `m_dataOffset + 4 < m_data.length`,
// so one extra byte is appended afterwards to ensure the operator can be read (the end of the data block usually has other column data)
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
            .expect("should have custom field configuration");
        let indicator = populated.borrow();
        let indicator = indicator.get_graphical_indicator();
        assert!(indicator.get_display_graphical_indicators());
        assert!(!indicator.get_show_data_values_in_tool_tips());
        let criteria = &indicator.get_non_summary_row_criteria()[0];
        assert_eq!(criteria.get_indicator(), 3);
        assert_eq!(criteria.get_operator(), Some(TestOperator::IsAnyValue));
        assert_eq!(criteria.get_value(0), None);
    }

/// When the operand is a field reference, no inline value is read.
    #[test]
    fn field_reference_operand() {
        let mut criteria = Vec::new();
        criteria.extend_from_slice(&1i32.to_le_bytes());
        criteria.extend_from_slice(&(0x3E7 + 6i32).to_le_bytes());
// constant flag 0 ⇒ field reference
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
            .expect("should have custom field configuration");
        let indicator = populated.borrow();
        assert_eq!(
            indicator
                .get_graphical_indicator()
                .get_non_summary_row_criteria()[0]
                .get_value(0),
            Some(&Operand::Field(FieldKey::Task(TaskField::Duration)))
        );
    }

/// Does not panic when data is missing / truncated / the attribute is absent.
    #[test]
    fn malformed_data_is_tolerated() {
        let file = ProjectFile::new();

// no TASK_FIELD_ATTRIBUTES at all
        let mut reader = GraphicalIndicatorReader::new();
        reader.process(&file, &Props::new());
        assert_eq!(custom_field_count(&file), 0);

// only 4 bytes (the column count cannot be read)
        let mut props = Props::new();
        props.put(PropsKey::TaskFieldAttributes.get_value(), vec![0u8; 4]);
        let mut reader = GraphicalIndicatorReader::new();
        reader.process(&file, &props);
        assert_eq!(custom_field_count(&file), 0);

// column count is 0
        let mut block = vec![0u8; 8];
        block[4..8].copy_from_slice(&0i32.to_le_bytes());
        let mut props = Props::new();
        props.put(PropsKey::TaskFieldAttributes.get_value(), block);
        let mut reader = GraphicalIndicatorReader::new();
        reader.process(&file, &props);
        assert_eq!(custom_field_count(&file), 0);

// column data offset out of bounds: the flag byte is treated as 0 ⇒ skip
        let mut block = vec![0u8; 16];
        block[4..8].copy_from_slice(&1i32.to_le_bytes());
        block[8..12].copy_from_slice(&field_id(TaskField::Duration).to_le_bytes());
        block[12..16].copy_from_slice(&9999i32.to_le_bytes());
        let mut props = Props::new();
        props.put(PropsKey::TaskFieldAttributes.get_value(), block);
        let mut reader = GraphicalIndicatorReader::new();
        reader.process(&file, &props);
        assert_eq!(custom_field_count(&file), 0);

// unrecognised field number ⇒ skip
        let mut block = field_attributes(0x0F, TaskField::Duration, &duration_criteria());
        block[8..12].copy_from_slice(&0x1234_0005i32.to_le_bytes());
        let mut props = Props::new();
        props.put(PropsKey::TaskFieldAttributes.get_value(), block);
        let mut reader = GraphicalIndicatorReader::new();
        reader.process(&file, &props);
        assert_eq!(custom_field_count(&file), 0);

// criteria area truncated (declared range longer than actual data) ⇒ the loop converges by +2 and does not panic
        let mut props = Props::new();
        props.put(
            PropsKey::TaskFieldAttributes.get_value(),
            field_attributes(0x02, TaskField::Duration, &[0u8; 3]),
        );
        let mut reader = GraphicalIndicatorReader::new();
        reader.process(&file, &props);
        let populated = custom_field(&file, FieldKey::Task(TaskField::Duration))
            .expect("should have custom field configuration");
        assert_eq!(
            populated
                .borrow()
                .get_graphical_indicator()
                .get_non_summary_row_criteria()
                .len(),
            1
        );
    }

/// Default construction is equivalent to `new`.
    #[test]
    fn default_matches_new() {
        let file = ProjectFile::new();
        let mut reader = GraphicalIndicatorReader::default();
        reader.process(&file, &Props::new());
        assert_eq!(custom_field_count(&file), 0);
    }
}
