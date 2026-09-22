//! Corresponds to Java: `org/mpxj/mpp/CriteriaReader.java`
//!
//! Shared implementation for reading filter / group / auto-filter criteria from an MPP file: the criteria data is a series
//! of fixed-size blocks linked into a tree via "child block offset / next sibling block offset"; this class recursively
//! restores them into a [`GenericCriteria`] tree.
//!
//! # Restoring inheritance (trait default methods)
//!
//! Java's `abstract class CriteriaReader` is extended by
//! [`FilterCriteriaReader9`](crate::mpp::filter_criteria_reader9::FilterCriteriaReader9) /
//! [`FilterCriteriaReader12`](crate::mpp::filter_criteria_reader12::FilterCriteriaReader12) /
//! [`FilterCriteriaReader14`](crate::mpp::filter_criteria_reader14::FilterCriteriaReader14)
//! Rust has no inheritance, so:
//!
//! - This class becomes the [`CriteriaReader`] **trait**: Java's 10 `protected abstract` methods
//! correspond to the trait's required methods; each subclass implements this trait with a zero-sized struct (no fields to inherit).
//! - Java's public method `process` is a **concrete method**; on the Rust side it becomes a **default method** of the trait,
//! so subclasses need not reimplement it. The call-time state needed for recursion lives in the private [`CriteriaState`].
//! - Java's `protected final Map<Integer, byte[]> m_criteriaBlockMap` is read directly by the subclass's
//! `getChildBlock` / `getListNextBlock`; a Rust trait cannot carry fields, so the map is
//! passed as a parameter to these two methods (see [`CriteriaBlockMap`]).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `protected ProjectFile m_file` | parameter `file: &Rc<RefCell<ProjectFile>>` | This crate shares the project file via `Rc<RefCell<ProjectFile>>` |
//! | `private ProjectProperties m_properties` | no stored reference; borrow via `try_borrow` on demand | `ProjectProperties` is held by value in `ProjectFile` (consistent with the existing trade-off in `Var2Data` / `CustomFieldValueReader`) |
//! | `protected final Map<Integer, byte[]> m_criteriaBlockMap` | [`CriteriaBlockMap`] (`BTreeMap`, matching the traversal order of Java's `TreeMap`) + method parameter | a trait cannot carry fields |
//! | `Object` (right-hand value) | [`Operand`] (`FieldType` -> `Operand::Field`, `GenericCriteriaPrompt` -> `Operand::Prompt`, scalar -> `Operand::Value`) | consistent with the right-hand value representation of [`GenericCriteria`] |
//! | `FieldType` | [`MppFieldType`], converted to [`FieldKey`] via `to_field_key()` when written into [`GenericCriteria`] | see `crate::common::mpp_field_type` |
//! | `List<GenericCriteriaPrompt>` / `List<FieldType>` / `boolean[]` (optional output parameters; `null` means do not collect) | `Option<&mut Vec<GenericCriteriaPrompt>>` / `Option<&mut Vec<FieldKey>>` / `Option<&mut [bool; 2]>` | `null` -> `None`; `boolean[]` is always length 2 |
//! | `ByteArrayHelper.getShort/getInt` throw `ArrayIndexOutOfBoundsException` when out of bounds | [`checked_short`] / [`checked_int`] (reusing the same-named helpers in `crate::mpp::custom_field_value_reader`) return `None` | when a block is truncated Java makes the whole read fail; the Rust side degrades to "this criterion cannot be parsed" and comments each such point |
//! | `TestOperator.getInstance(int)` throws when out of bounds (the reflection-built table indexes by value) | [`TestOperator::get_instance`] falls back to `IS_ANY_VALUE` | see the notes in `crate::test_operator` |
//! | `leftValue.getFieldTypeClass()` / `field.getDataType()` when `leftValue` is `null` (Java throws an NPE) | skip that branch (do not write `criteriaType` / do not fetch the data type) | follows the "never panic" convention |
//! | Java's `getPromptValue` does **not** add `m_dataOffset` when computing the text offset (inconsistent with the STRING branch of `getConstantValue`) | keep that behavior as-is | upstream keeps it as-is; the caller's (`FilterReader`) `dataOffset` is always 0, so it is unaffected |
//!
//! Skipped Java members: none (all methods have been translated).
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::common::mpp_field_type::MppFieldType;
use crate::data_type::DataType;
use crate::duration::Duration;
use crate::field_key::FieldKey;
use crate::field_type_class::FieldTypeClass;
use crate::field_value::FieldValue;
use crate::generic_criteria::GenericCriteria;
use crate::generic_criteria_prompt::GenericCriteriaPrompt;
use crate::mpp::custom_field_value_reader::{checked_int, checked_short};
use crate::mpp::mpp_utility::MPPUtility;
use crate::project_file::ProjectFile;
use crate::test_operator::{Operand, TestOperator};
use crate::time_unit::TimeUnit;

/// Criteria data block map: offset of a block in the data body -> block contents.
///
/// Corresponds to Java: `protected final Map<Integer, byte[]> m_criteriaBlockMap = new TreeMap<>();`
pub type CriteriaBlockMap = BTreeMap<i32, Vec<u8>>;

/// Criteria reader.
///
/// Corresponds to Java: `abstract class CriteriaReader`
pub trait CriteriaReader {
/// Get the start offset of the criteria data (position of the first block).
    ///
    /// Corresponds to Java: `protected abstract int getCriteriaStartOffset()`
    fn get_criteria_start_offset(&self) -> i32;

/// Get the size in bytes of a single criteria block.
    ///
    /// Corresponds to Java: `protected abstract int getCriteriaBlockSize()`
    fn get_criteria_block_size(&self) -> i32;

/// Get the child block of the current block (the next level in the tree structure).
    ///
    /// Corresponds to Java: `protected abstract byte[] getChildBlock(byte[] block)`
    ///
/// The Rust side additionally receives the block map (Java reads it from the `m_criteriaBlockMap` field).
    fn get_child_block(&self, block: &[u8], blocks: &CriteriaBlockMap) -> Option<Vec<u8>>;

/// Get the "next sibling block in the list" of the current block.
    ///
    /// Corresponds to Java: `protected abstract byte[] getListNextBlock(byte[] block)`
    fn get_list_next_block(&self, block: &[u8], blocks: &CriteriaBlockMap) -> Option<Vec<u8>>;

/// Get the offset of a text block within the criteria text area.
    ///
    /// Corresponds to Java: `protected abstract int getTextOffset(byte[] block)`
    fn get_text_offset(&self, block: &[u8]) -> i32;

/// Get the offset of the prompt text.
    ///
    /// Corresponds to Java: `protected abstract int getPromptOffset(byte[] block)`
    fn get_prompt_offset(&self, block: &[u8]) -> i32;

/// Get the offset of the field value.
    ///
    /// Corresponds to Java: `protected abstract int getValueOffset()`
    fn get_value_offset(&self) -> i32;

/// Get the offset of the duration units.
    ///
    /// Corresponds to Java: `protected abstract int getTimeUnitsOffset()`
    fn get_time_units_offset(&self) -> i32;

/// Get the offset, within a block, of the "criteria text area start position".
    ///
    /// Corresponds to Java: `protected abstract int getCriteriaTextStartOffset()`
    fn get_criteria_text_start_offset(&self) -> i32;

/// Get the field type described by the block.
    ///
    /// Corresponds to Java: `protected abstract FieldType getFieldType(byte[] block)`
    ///
/// An unrecognized field number is a placeholder object in Java (`getPlaceholder` of `FieldTypeHelper`); this crate
/// cannot express it, so it returns `None` (see the notes in `crate::common::field_type_helper` for details).
    fn get_field_type(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        block: &[u8],
    ) -> Option<MppFieldType>;

/// Main entry point for reading criteria data.
    ///
    /// Corresponds to Java: `public GenericCriteria process(ProjectFile file, byte[] data, int dataOffset,
    /// int entryOffset, List<GenericCriteriaPrompt> prompts, List<FieldType> fields,
    /// boolean[] criteriaType)`
    ///
/// - `data_offset`: start position of the criteria data within the large data block;
/// - `entry_offset`: block offset at which tree traversal starts; `-1` means start from `getCriteriaStartOffset()`;
/// - `prompts` / `fields` / `criteria_type`: optional output parameters (Java passes `null` to not collect);
/// - returns the first node of the tree; returns `None` when the data contains no criteria (Java returns `null`).
    fn process(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        data: &[u8],
        data_offset: i32,
        entry_offset: i32,
        prompts: Option<&mut Vec<GenericCriteriaPrompt>>,
        fields: Option<&mut Vec<FieldKey>>,
        criteria_type: Option<&mut [bool; 2]>,
    ) -> Option<GenericCriteria> {
        let mut state: CriteriaState<'_, Self> = CriteriaState {
            reader: self,
            file,
            data,
            data_offset,
            criteria_text_start: 0,
            prompts,
            fields,
            criteria_type,
            blocks: CriteriaBlockMap::new(),
        };

        state.process(entry_offset)
    }
}

/// State during a single `process` call.
///
/// Corresponds to the instance fields of Java `CriteriaReader` (`m_criteriaData` / `m_criteriaTextStart` /
/// `m_dataOffset` / `m_prompts` / `m_fields` / `m_criteriaType` / `m_criteriaBlockMap`).
/// The Rust side gathers it into a per-call local structure, keeping the trait stateless and safely reusable.
///
/// `R` is the concrete criteria reader type (`Self`, allowing `?Sized`, so it can be used for both concrete types and
/// `&dyn CriteriaReader`).
struct CriteriaState<'a, R: CriteriaReader + ?Sized> {
/// the criteria reader (provides how each offset / block pointer is resolved).
    reader: &'a R,

/// Project file.
    ///
    /// Corresponds to Java: `protected ProjectFile m_file`
    file: &'a Rc<RefCell<ProjectFile>>,

/// Criteria data.
    ///
    /// Corresponds to Java: `private byte[] m_criteriaData`
    data: &'a [u8],

/// Start position of the criteria data within the large data block.
    ///
    /// Corresponds to Java: `private int m_dataOffset`
    data_offset: i32,

/// Criteria text area start position (relative to the data body).
    ///
    /// Corresponds to Java: `private int m_criteriaTextStart`
    criteria_text_start: i32,

/// Optional: collected prompts.
    ///
    /// Corresponds to Java: `private List<GenericCriteriaPrompt> m_prompts`
    prompts: Option<&'a mut Vec<GenericCriteriaPrompt>>,

/// Optional: collected fields.
    ///
    /// Corresponds to Java: `private List<FieldType> m_fields`
    fields: Option<&'a mut Vec<FieldKey>>,

/// Optional: criteria type (`[0]` task, `[1]` resource).
    ///
    /// Corresponds to Java: `private boolean[] m_criteriaType`
    criteria_type: Option<&'a mut [bool; 2]>,

/// Criteria block map.
    ///
    /// Corresponds to Java: `protected final Map<Integer, byte[]> m_criteriaBlockMap`
    blocks: CriteriaBlockMap,
}

impl<R: CriteriaReader + ?Sized> CriteriaState<'_, R> {
/// Main entry point (corresponds to the body of Java's `process`).
    fn process(&mut self, entry_offset: i32) -> Option<GenericCriteria> {
        // Java: if (m_criteriaType != null) { m_criteriaType[0] = true; m_criteriaType[1] = true; }
        if let Some(criteria_type) = self.criteria_type.as_mut() {
            criteria_type[0] = true;
            criteria_type[1] = true;
        }

        // Java: m_criteriaBlockMap.clear();
        self.blocks.clear();

        // Java: m_criteriaTextStart = ByteArrayHelper.getShort(m_criteriaData, m_dataOffset + getCriteriaTextStartOffset());
// Java throws when out of bounds (the whole read fails); the Rust side returns None (this filter has no readable criteria)
        let text_start_position = self
            .data_offset
            .checked_add(self.reader.get_criteria_text_start_offset())?;
        let criteria_text_start = checked_short(self.data, offset_usize(text_start_position)?)?;
        self.criteria_text_start = criteria_text_start;

        // Java: int criteriaStartOffset = getCriteriaStartOffset();
        //       int criteriaBlockSize = getCriteriaBlockSize();
        let criteria_block_size = self.reader.get_criteria_block_size();

        // Java: if (m_criteriaData.length <= m_criteriaTextStart) { return null; }(bad data)
        if (self.data.len() as i64) <= i64::from(criteria_text_start) {
            return None;
        }

        //
// Java: while (criteriaStartOffset + criteriaBlockSize <= m_criteriaTextStart) { copy blocks into the map }
        //
// Java throws when `System.arraycopy` goes out of bounds; the Rust side stops collecting on truncation (that block and the blocks after it
// never enter the map, and are later treated as "block does not exist").
        let mut criteria_start_offset = self.reader.get_criteria_start_offset();
        while criteria_block_size > 0
            && criteria_start_offset
                .checked_add(criteria_block_size)
                .is_some_and(|end| end <= criteria_text_start)
        {
            let Some(start) = self
                .data_offset
                .checked_add(criteria_start_offset)
                .and_then(offset_usize)
            else {
                break;
            };
            let Some(end) = start.checked_add(criteria_block_size as usize) else {
                break;
            };
            let Some(block) = self.data.get(start..end) else {
                break;
            };

            self.blocks.insert(criteria_start_offset, block.to_vec());
            criteria_start_offset += criteria_block_size;
        }

        // Java: if (entryOffset == -1) { entryOffset = getCriteriaStartOffset(); }
        let entry_offset = if entry_offset == -1 {
            self.reader.get_criteria_start_offset()
        } else {
            entry_offset
        };

        // Java: List<GenericCriteria> list = new ArrayList<>();
        //       processBlock(list, m_criteriaBlockMap.get(Integer.valueOf(entryOffset)));
        //       criteria = list.isEmpty() ? null : list.get(0);
        let mut list: Vec<GenericCriteria> = Vec::new();
        let block = self.blocks.get(&entry_offset).cloned();
        self.process_block(&mut list, block.as_deref());

        list.into_iter().next()
    }

/// Process a single criteria block.
    ///
    /// Corresponds to Java: `private void processBlock(List<GenericCriteria> list, byte[] block)`
    fn process_block(&mut self, list: &mut Vec<GenericCriteria>, block: Option<&[u8]>) {
        // Java: if (block != null) { ... }
        let Some(block) = block else {
            return;
        };

        // Java: if (ByteArrayHelper.getShort(block, 0) > 0x3E6) { addCriteria(list, block); }
// Java throws when out of bounds; the Rust side returns immediately (the block is too short, nothing to read)
        let Some(header) = checked_short(block, 0) else {
            return;
        };

        if header > 0x3E6 {
            self.add_criteria(list, block);
            return;
        }

        // Java: switch (block[0]) { ... }
        match block[0] {
            // Java: case (byte) 0x0B: processBlock(list, getChildBlock(block));
            0x0B => {
                let child = self.reader.get_child_block(block, &self.blocks);
                self.process_block(list, child.as_deref());
            }

            // Java: case (byte) 0x06: processBlock(list, getListNextBlock(block));
            0x06 => {
                let next = self.reader.get_list_next_block(block, &self.blocks);
                self.process_block(list, next.as_deref());
            }

            // Java: case (byte) 0xED: // EQUALS
            0xED => self.add_criteria(list, block),

            // Java: case (byte) 0x19: case (byte) 0x1B: // AND
            0x19 | 0x1B => self.add_block(list, block, TestOperator::And),

            // Java: case (byte) 0x1A: case (byte) 0x1C: // OR
            0x1A | 0x1C => self.add_block(list, block, TestOperator::Or),

// Java: no default branch
            _ => {}
        }
    }

/// Append a `left value operator right value` criteria block.
    ///
    /// Corresponds to Java: `private void addCriteria(List<GenericCriteria> list, byte[] block)`
    fn add_criteria(&mut self, list: &mut Vec<GenericCriteria>, block: &[u8]) {
        // Java: byte[] leftBlock = getChildBlock(block);
        let left_block = self.reader.get_child_block(block, &self.blocks);

        // Java: byte[] rightBlock1 = getListNextBlock(leftBlock);
        //       byte[] rightBlock2 = getListNextBlock(rightBlock1);
// Java throws an NPE when leftBlock / rightBlock1 is null; the Rust side treats it as "no such block"
        let right_block1 = left_block
            .as_deref()
            .and_then(|block| self.reader.get_list_next_block(block, &self.blocks));
        let right_block2 = right_block1
            .as_deref()
            .and_then(|block| self.reader.get_list_next_block(block, &self.blocks));

        // Java: TestOperator operator = TestOperator.getInstance(ByteArrayHelper.getShort(block, 0) - 0x3E7);
// (Java's reflection-built table throws when out of bounds; the Rust side falls back to IS_ANY_VALUE, see crate::test_operator)
        let operator_value = checked_short(block, 0).unwrap_or(0) - 0x3E7;
        let operator = TestOperator::get_instance(operator_value);

        // Java: FieldType leftValue = getFieldType(leftBlock);
        let left_value = left_block
            .as_deref()
            .and_then(|block| self.reader.get_field_type(self.file, block));

        // Java: Object rightValue1 = getValue(leftValue, rightBlock1);
        let right_value1 = right_block1
            .as_deref()
            .and_then(|block| self.get_value(left_value.as_ref(), block));

        // Java: Object rightValue2 = rightBlock2 == null ? null : getValue(leftValue, rightBlock2);
        let right_value2 = match right_block2.as_deref() {
            Some(block) => self.get_value(left_value.as_ref(), block),
            None => None,
        };

        // Java: GenericCriteria criteria = new GenericCriteria(m_properties);
        //       criteria.setLeftValue(leftValue); criteria.setOperator(operator);
        //       criteria.setRightValue(0, rightValue1); criteria.setRightValue(1, rightValue2);
        //       list.add(criteria);
        let mut criteria = GenericCriteria::new(Rc::downgrade(self.file));
        criteria.set_left_value(left_value.as_ref().map(MppFieldType::to_field_key));
        criteria.set_operator(Some(operator));
        criteria.set_right_value(0, right_value1);
        criteria.set_right_value(1, right_value2);
        list.push(criteria);

        // Java: if (m_criteriaType != null) { ... } / if (m_fields != null) { m_fields.add(leftValue); }
// when `leftValue` is null Java throws an NPE; the Rust side skips these two branches
        if let Some(left_value) = left_value.as_ref() {
            let is_task = left_value.get_field_type_class() == FieldTypeClass::Task;

            if let Some(criteria_type) = self.criteria_type.as_mut() {
                criteria_type[0] = is_task;
                criteria_type[1] = !is_task;
            }

            if let Some(fields) = self.fields.as_mut() {
                fields.push(left_value.to_field_key());
            }
        }

        // Java: processBlock(list, getListNextBlock(block));
        let next = self.reader.get_list_next_block(block, &self.blocks);
        self.process_block(list, next.as_deref());
    }

/// Append a logical operator block (AND / OR).
    ///
    /// Corresponds to Java: `private void addBlock(List<GenericCriteria> list, byte[] block, TestOperator operator)`
    fn add_block(&mut self, list: &mut Vec<GenericCriteria>, block: &[u8], operator: TestOperator) {
        // Java: GenericCriteria result = new GenericCriteria(m_properties);
        //       result.setOperator(operator); list.add(result);
        let mut result = GenericCriteria::new(Rc::downgrade(self.file));
        result.set_operator(Some(operator));
        list.push(result);

        // Java: processBlock(result.getCriteriaList(), getChildBlock(block));
// equivalent to filling in child criteria in place on the last element just added
        let child = self.reader.get_child_block(block, &self.blocks);
        if let Some(last) = list.last_mut() {
            self.process_block(last.get_criteria_list_mut(), child.as_deref());
        }

        // Java: processBlock(list, getListNextBlock(block));
        let next = self.reader.get_list_next_block(block, &self.blocks);
        self.process_block(list, next.as_deref());
    }

/// Get the right-hand value in a criteria expression.
    ///
    /// Corresponds to Java: `private Object getValue(FieldType field, byte[] block)`
    fn get_value(&mut self, field: Option<&MppFieldType>, block: &[u8]) -> Option<Operand> {
// Java throws for an empty block / null; the Rust side returns None
        match *block.first()? {
            // Java: case 0x07: // Field —— result = getFieldType(block);
            0x07 => self
                .reader
                .get_field_type(self.file, block)
                .map(|field| Operand::Field(field.to_field_key())),

            // Java: case 0x01: // Constant value
            0x01 => self.get_constant_value(field, block),

            // Java: case 0x00: // Prompt
            0x00 => self.get_prompt_value(field, block),

// Java: no default branch -- result stays null
            _ => None,
        }
    }

/// Get the constant value.
    ///
    /// Corresponds to Java: `private Object getConstantValue(FieldType type, byte[] block)`
    fn get_constant_value(
        &mut self,
        field: Option<&MppFieldType>,
        block: &[u8],
    ) -> Option<Operand> {
        // Java: DataType dataType = type.getDataType();
        //       if (dataType == null) { value = null; }
        let data_type = field.map(MppFieldType::get_data_type);

        match data_type {
            // Java: case DURATION:
            //   value = MPPUtility.getAdjustedDuration(m_properties, ByteArrayHelper.getInt(block, getValueOffset()),
            //       MPPUtility.getDurationTimeUnits(ByteArrayHelper.getShort(block, getTimeUnitsOffset())));
            Some(DataType::Duration) => {
                let value = checked_int(block, self.value_offset())?;
                let units = MPPUtility::get_duration_time_units(
                    checked_short(block, self.time_units_offset()).unwrap_or(0),
                );
                let duration = self.adjusted_duration(value, units)?;
                Some(Operand::Value(FieldValue::Duration(duration)))
            }

            // Java: case NUMERIC: value = Double.valueOf(MPPUtility.getDouble(block, getValueOffset()));
            Some(DataType::Numeric) => Some(Operand::Value(FieldValue::Double(
                MPPUtility::get_double(block, self.value_offset()),
            ))),

            // Java: case PERCENTAGE: value = Double.valueOf(ByteArrayHelper.getShort(block, getValueOffset()));
            Some(DataType::Percentage) => Some(Operand::Value(FieldValue::Double(f64::from(
                checked_short(block, self.value_offset()).unwrap_or(0),
            )))),

            // Java: case CURRENCY: value = Double.valueOf(MPPUtility.getDouble(block, getValueOffset()) / 100);
            Some(DataType::Currency) => Some(Operand::Value(FieldValue::Double(
                MPPUtility::get_double(block, self.value_offset()) / 100.0,
            ))),

            // Java: case STRING:
            //   int textOffset = getTextOffset(block);
            //   value = MPPUtility.getUnicodeString(m_criteriaData, m_dataOffset + m_criteriaTextStart + textOffset);
            Some(DataType::String) => {
                let text_offset = self.reader.get_text_offset(block);
                let position = self
                    .data_offset
                    .checked_add(self.criteria_text_start)?
                    .checked_add(text_offset)?;
                let offset = offset_usize(position)?;
                Some(Operand::Value(FieldValue::String(
                    MPPUtility::get_unicode_string(self.data, offset),
                )))
            }

            // Java: case BOOLEAN:
            //   int intValue = ByteArrayHelper.getShort(block, getValueOffset());
            //   value = (intValue == 1 ? Boolean.TRUE : Boolean.FALSE);
            Some(DataType::Boolean) => Some(Operand::Value(FieldValue::Boolean(
                checked_short(block, self.value_offset()).unwrap_or(0) == 1,
            ))),

            // Java: case DATE: value = MPPUtility.getTimestamp(block, getValueOffset());
            Some(DataType::Date) => MPPUtility::get_timestamp(block, self.value_offset())
                .map(|value| Operand::Value(FieldValue::DateTime(value))),

// Java: default: value = null; (including the case where getDataType() is null)
            _ => None,
        }
    }

/// Get the prompt value.
    ///
    /// Corresponds to Java: `private GenericCriteriaPrompt getPromptValue(FieldType field, byte[] block)`
    fn get_prompt_value(
        &mut self,
        field: Option<&MppFieldType>,
        block: &[u8],
    ) -> Option<Operand> {
        // Java: int textOffset = getPromptOffset(block);
        let text_offset = self.reader.get_prompt_offset(block);

        // Java: String value = MPPUtility.getUnicodeString(m_criteriaData, m_criteriaTextStart + textOffset);
// (Java as-is: m_dataOffset is missing here, inconsistent with the STRING branch of getConstantValue)
        let position = self.criteria_text_start.checked_add(text_offset)?;
        let offset = offset_usize(position)?;
        let value = MPPUtility::get_unicode_string(self.data, offset);

        // Java: GenericCriteriaPrompt prompt = new GenericCriteriaPrompt(field.getDataType(), value);
// (Java throws an NPE when `field` is null; the Rust side falls back to STRING, which does not affect the prompt text itself)
        let data_type = field
            .map(MppFieldType::get_data_type)
            .unwrap_or(DataType::String);
        let prompt = GenericCriteriaPrompt::new(data_type, Some(value));

        // Java: if (m_prompts != null) { m_prompts.add(prompt); }
        if let Some(prompts) = self.prompts.as_mut() {
            prompts.push(prompt.clone());
        }

        Some(Operand::Prompt(prompt))
    }

/// Get the offset of the field value (a non-negative index).
    ///
    /// Corresponds to Java: `getValueOffset()`(`protected abstract`).
    fn value_offset(&self) -> usize {
        offset_usize(self.reader.get_value_offset()).unwrap_or(0)
    }

/// Get the offset of the duration units (a non-negative index).
    ///
    /// Corresponds to Java: `getTimeUnitsOffset()`(`protected abstract`).
    fn time_units_offset(&self) -> usize {
        offset_usize(self.reader.get_time_units_offset()).unwrap_or(0)
    }

/// Borrow the project properties and convert the duration per MPP rules.
    ///
    /// Corresponds to Java: `MPPUtility.getAdjustedDuration(m_properties, duration, timeUnit)`.
/// Degrades to `None` when the parent file is mutably borrowed (Java has no such case).
    fn adjusted_duration(&self, value: i32, units: TimeUnit) -> Option<Duration> {
        let project = self.file.try_borrow().ok()?;
        MPPUtility::get_adjusted_duration(project.get_project_properties(), value, units)
    }
}

/// Convert Java's `int` offset into a usable slice index; a negative value returns `None`.
fn offset_usize(value: i32) -> Option<usize> {
    usize::try_from(value).ok()
}

/// Criteria data construction helper for tests (reused by tests in this module and modules such as `FilterReader`).
///
/// Corresponds to Java: none (Java test cases inline the byte arrays directly).
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use crate::common::field_type_helper::FieldTypeHelper;
    use crate::task_field::TaskField;

/// Criteria text area start position (the block area is `20..TEXT_START`, step 80).
    pub(crate) const TEXT_START: i32 = 420;

/// Top-level logical block offset.
    pub(crate) const BLOCK_TOP: i32 = 20;
/// Criteria block offset.
    pub(crate) const BLOCK_CRITERIA: i32 = 100;
/// Left-hand value (field) block offset.
    pub(crate) const BLOCK_LEFT: i32 = 180;
/// Right-hand value block offset.
    pub(crate) const BLOCK_RIGHT: i32 = 260;

/// Write a 4-byte little-endian integer at the given offset.
    pub(crate) fn put_int(data: &mut [u8], offset: usize, value: i32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

/// Write a 2-byte little-endian short integer at the given offset.
    pub(crate) fn put_short(data: &mut [u8], offset: usize, value: i32) {
        data[offset..offset + 2].copy_from_slice(&(value as u16).to_le_bytes());
    }

/// Field number of a built-in task field (the field numbering scheme of MPP9/12).
    pub(crate) fn field_id(field: TaskField) -> i32 {
        FieldTypeHelper::get_field_id(Some(&MppFieldType::Builtin(FieldKey::Task(field))))
    }

/// Construct criteria data with the MPP9 layout:
    ///
    /// ```text
/// [16..18)             criteria text area start position (TEXT_START)
    /// 20   +-----------------------------+
/// | logical block (block_tag)          | child -> 100
    /// 100  +-----------------------------+
/// | criteria block (operator = EQUALS)     | child -> 180, next -> 0
    /// 180  +-----------------------------+
/// | left value block (field number @40)          | next -> 260
    /// 260  +-----------------------------+
/// | right value block (constant @0x01, text @32)|
    /// 420  +-----------------------------+
/// | text area (UTF-16LE + NUL)      |
    /// ```
    ///
/// Returns `(data, entry block offset)`.
    pub(crate) fn criteria_data(block_tag: u8, field: TaskField, text: &str) -> (Vec<u8>, i32) {
        let mut data = vec![0u8; (TEXT_START as usize) + 32];

// criteria text area start position
        put_short(&mut data, 16, TEXT_START);

// top-level logical block: 0x19/0x1B = AND, 0x1A/0x1C = OR
        data[BLOCK_TOP as usize] = block_tag;
        put_short(&mut data, BLOCK_TOP as usize + 74, BLOCK_CRITERIA);

// criteria block: short > 0x3E6 => EQUALS (0x3E7 + 6)
        put_short(&mut data, BLOCK_CRITERIA as usize, 0x3E7 + 6);
        put_short(&mut data, BLOCK_CRITERIA as usize + 74, BLOCK_LEFT);

// left value block: field number at 40, next points to the right value block
        put_int(&mut data, BLOCK_LEFT as usize + 40, field_id(field));
        put_short(&mut data, BLOCK_LEFT as usize + 76, BLOCK_RIGHT);

// right value block: 0x01 = constant, text offset at 68
        data[BLOCK_RIGHT as usize] = 0x01;
        put_short(&mut data, BLOCK_RIGHT as usize + 68, 0);

// text area: UTF-16LE + NUL
        let mut offset = TEXT_START as usize;
        for unit in text.encode_utf16() {
            data[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
            offset += 2;
        }
        data[offset..offset + 2].copy_from_slice(&[0, 0]);

        (data, BLOCK_TOP)
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{
        criteria_data, field_id, put_int, put_short, BLOCK_CRITERIA, BLOCK_LEFT, BLOCK_RIGHT,
    };
    use super::*;
    use crate::mpp::filter_criteria_reader9::FilterCriteriaReader9;
    use crate::task_field::TaskField;

/// AND / OR combination blocks are restored into a logical criterion with child criteria.
    #[test]
    fn logical_block_nests_criteria() {
        for (tag, operator) in [(0x19u8, TestOperator::And), (0x1Cu8, TestOperator::Or)] {
            let (data, entry) = criteria_data(tag, TaskField::Name, "AB");
            let file = ProjectFile::new();
            let mut prompts = Vec::new();
            let mut fields = Vec::new();
            let mut criteria_type = [false; 2];

            let criteria = FilterCriteriaReader9::new()
                .process(
                    &file,
                    &data,
                    0,
                    entry,
                    Some(&mut prompts),
                    Some(&mut fields),
                    Some(&mut criteria_type),
                )
                .expect("should parse criteria");

            assert_eq!(criteria.get_operator(), Some(operator));
            let children = criteria.get_criteria_list();
            assert_eq!(children.len(), 1);

            let child = &children[0];
            assert_eq!(child.get_operator(), Some(TestOperator::Equals));
            assert_eq!(child.get_left_value(), Some(FieldKey::Task(TaskField::Name)));
            assert_eq!(
                child.get_value(0),
                Some(&Operand::Value(FieldValue::String("AB".to_owned())))
            );
// Java: rightBlock2 is null => right value 1 is null
            assert_eq!(child.get_value(1), None);

// output parameters: prompts empty, field is the left value, criteria type is task
            assert!(prompts.is_empty());
            assert_eq!(fields, vec![FieldKey::Task(TaskField::Name)]);
            assert_eq!(criteria_type, [true, false]);
        }
    }

/// Use a criteria block directly as the entry (`entryOffset` points to the criteria block).
    #[test]
    fn criteria_block_as_entry() {
        let (data, _) = criteria_data(0x19, TaskField::Name, "AB");
        let file = ProjectFile::new();

        let criteria = FilterCriteriaReader9::new()
            .process(&file, &data, 0, BLOCK_CRITERIA, None, None, None)
            .expect("should parse criteria");

        assert_eq!(criteria.get_operator(), Some(TestOperator::Equals));
        assert_eq!(criteria.get_left_value(), Some(FieldKey::Task(TaskField::Name)));
    }

/// When `entryOffset` is -1, start from `getCriteriaStartOffset()`.
    #[test]
    fn entry_offset_minus_one_uses_start_offset() {
        let (data, _) = criteria_data(0x1B, TaskField::Name, "AB");
        let file = ProjectFile::new();

        let criteria = FilterCriteriaReader9::new()
            .process(&file, &data, 0, -1, None, None, None)
            .expect("should parse criteria");

        assert_eq!(criteria.get_operator(), Some(TestOperator::And));
    }

/// Numeric right-hand value (including the one-hundredth conversion for CURRENCY).
    #[test]
    fn numeric_constant_value() {
        let (mut data, _) = criteria_data(0x19, TaskField::Name, "AB");

// change the right value block to "constant + numeric field + double 250"
        put_int(&mut data, BLOCK_LEFT as usize + 40, field_id(TaskField::Cost));
        data[BLOCK_RIGHT as usize] = 0x01;
        put_int(&mut data, BLOCK_RIGHT as usize + 32, 0);
// the double bit pattern of 250.0
        data[BLOCK_RIGHT as usize + 32..BLOCK_RIGHT as usize + 40]
            .copy_from_slice(&250.0f64.to_le_bytes());

        let file = ProjectFile::new();
        let criteria = FilterCriteriaReader9::new()
            .process(&file, &data, 0, BLOCK_CRITERIA, None, None, None)
            .expect("should parse criteria");

        assert_eq!(criteria.get_left_value(), Some(FieldKey::Task(TaskField::Cost)));
        assert_eq!(
            criteria.get_value(0),
            Some(&Operand::Value(FieldValue::Double(2.5)))
        );
    }

/// When the right value is a field reference (0x07), collect it as a signed value.
    #[test]
    fn field_reference_value() {
        let (mut data, _) = criteria_data(0x19, TaskField::Name, "AB");

// change the right value block to a field reference: 0x07, field number at 40
        data[BLOCK_RIGHT as usize] = 0x07;
        put_int(&mut data, BLOCK_RIGHT as usize + 40, field_id(TaskField::Cost));

        let file = ProjectFile::new();
        let criteria = FilterCriteriaReader9::new()
            .process(&file, &data, 0, BLOCK_CRITERIA, None, None, None)
            .expect("should parse criteria");

        assert_eq!(
            criteria.get_value(0),
            Some(&Operand::Field(FieldKey::Task(TaskField::Cost)))
        );
    }

/// When the right value is a prompt (0x00), also write it into the output parameter lists.
    #[test]
    fn prompt_value() {
        let (mut data, _) = criteria_data(0x19, TaskField::Name, "AB");

// change the right value block to a prompt: 0x00, prompt text offset at 72
        data[BLOCK_RIGHT as usize] = 0x00;
        put_short(&mut data, BLOCK_RIGHT as usize + 72, 0);

        let file = ProjectFile::new();
        let mut prompts = Vec::new();
        let criteria = FilterCriteriaReader9::new()
            .process(&file, &data, 0, BLOCK_CRITERIA, Some(&mut prompts), None, None)
            .expect("should parse criteria");

        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].get_prompt(), Some("AB"));
        assert_eq!(prompts[0].get_type(), DataType::String);
        assert_eq!(
            criteria.get_value(0),
            Some(&Operand::Prompt(prompts[0].clone()))
        );
    }

/// Does not panic when data is missing / truncated / a block pointer points to a nonexistent block.
    #[test]
    fn malformed_data_is_tolerated() {
        let file = ProjectFile::new();
        let reader = FilterCriteriaReader9::new();

// empty data
        assert!(reader.process(&file, &[], 0, -1, None, None, None).is_none());

// data is shorter than the text area start position (Java: bad data -> null)
        let mut data = vec![0u8; 20];
        put_short(&mut data, 16, 1000);
        assert!(reader
            .process(&file, &data, 0, -1, None, None, None)
            .is_none());

// the criteria text area start position cannot be read (fewer than 18 bytes)
        assert!(reader
            .process(&file, &[0u8; 10], 0, -1, None, None, None)
            .is_none());

// text area start position is 0 (less than the start block offset) => no block to read => no criteria
        let data = vec![0u8; 40];
        assert!(reader
            .process(&file, &data, 0, -1, None, None, None)
            .is_none());

// a block pointer points to a nonexistent offset
        let (data, _) = criteria_data(0x19, TaskField::Name, "AB");
        assert!(reader
            .process(&file, &data, 0, 9999, None, None, None)
            .is_none());

// block contents are truncated (the text area start position is valid, but the block area is less than one whole block)
        let mut data = vec![0u8; 100];
        put_short(&mut data, 16, 90);
        assert!(reader
            .process(&file, &data, 0, -1, None, None, None)
            .is_none());
    }

/// Unrecognized field number: a criterion is still generated, but the output parameters are not updated (Java throws an NPE).
    #[test]
    fn unknown_field_does_not_panic() {
        let (mut data, _) = criteria_data(0x19, TaskField::Name, "AB");
        put_int(&mut data, BLOCK_LEFT as usize + 40, 0x1234_0005);

        let file = ProjectFile::new();
        let mut fields = Vec::new();
        let mut criteria_type = [false; 2];

        let criteria = FilterCriteriaReader9::new()
            .process(
                &file,
                &data,
                0,
                BLOCK_CRITERIA,
                None,
                Some(&mut fields),
                Some(&mut criteria_type),
            )
            .expect("should parse criteria");

        assert_eq!(criteria.get_left_value(), None);
        assert!(fields.is_empty());
// Java throws an NPE here; the Rust side keeps the initialization values from the start of process
        assert_eq!(criteria_type, [true, true]);
    }
}
