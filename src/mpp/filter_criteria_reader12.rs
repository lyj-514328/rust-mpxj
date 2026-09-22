//! Corresponds to Java: `org/mpxj/mpp/FilterCriteriaReader12.java`
//!
//! The MPP12 filter criteria block reader: tells [`CriteriaReader`] the layout of the criteria block in version 12 files.
//!
//! # Restoring inheritance (trait implementation)
//!
//! Java's `final class FilterCriteriaReader12 extends CriteriaReader`. The Rust side instead uses
//! **a zero-sized struct implementing the [`CriteriaReader`] trait**; the public method `process` inherited from Java
//! is provided by the trait's default method.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `m_criteriaBlockMap` (a `protected` field inherited from `CriteriaReader`) | method parameter `blocks: &CriteriaBlockMap` | see [`crate::mpp::criteria_reader`] |
//! | `ByteArrayHelper.getShort/getInt` throw on out of bounds | [`checked_short`] / [`checked_int`] return `None` | follows the "never panic" convention |
//! | **Verbatim identical** to this crate's `FilterCriteriaReader9` implementation (likewise in Java) | the duplicate implementation is kept as well | The two Java classes have no inheritance relationship, so the Rust side does not introduce extra abstraction |
//!
//! Skipped Java members: none.
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::mpp_field_type::MppFieldType;
use crate::mpp::criteria_reader::{CriteriaBlockMap, CriteriaReader};
use crate::mpp::custom_field_value_reader::{checked_int, checked_short};
use crate::project_file::ProjectFile;

/// The MPP12 filter criteria block reader.
///
/// Corresponds to Java: `final class FilterCriteriaReader12 extends CriteriaReader`
pub struct FilterCriteriaReader12;

impl FilterCriteriaReader12 {
/// Constructor.
    ///
/// Corresponds to Java: implicit default constructor (the Java side does not declare a constructor explicitly).
    pub fn new() -> Self {
        Self
    }
}

impl Default for FilterCriteriaReader12 {
/// Corresponds to Java: implicit default constructor, equivalent to [`FilterCriteriaReader12::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl CriteriaReader for FilterCriteriaReader12 {
    /// Corresponds to Java: `@Override protected int getCriteriaStartOffset()`
    fn get_criteria_start_offset(&self) -> i32 {
        20
    }

    /// Corresponds to Java: `@Override protected int getCriteriaBlockSize()`
    fn get_criteria_block_size(&self) -> i32 {
        80
    }

    /// Corresponds to Java: `@Override protected byte[] getChildBlock(byte[] block)`
    fn get_child_block(&self, block: &[u8], blocks: &CriteriaBlockMap) -> Option<Vec<u8>> {
        // Java: int offset = ByteArrayHelper.getShort(block, 74); return m_criteriaBlockMap.get(Integer.valueOf(offset));
        let offset = checked_short(block, 74)?;
        blocks.get(&offset).cloned()
    }

    /// Corresponds to Java: `@Override protected byte[] getListNextBlock(byte[] block)`
    fn get_list_next_block(&self, block: &[u8], blocks: &CriteriaBlockMap) -> Option<Vec<u8>> {
        // Java: int offset = ByteArrayHelper.getShort(block, 76); return m_criteriaBlockMap.get(Integer.valueOf(offset));
        let offset = checked_short(block, 76)?;
        blocks.get(&offset).cloned()
    }

    /// Corresponds to Java: `@Override protected FieldType getFieldType(byte[] block)`
    fn get_field_type(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        block: &[u8],
    ) -> Option<MppFieldType> {
        // Java: int fieldIndex = ByteArrayHelper.getInt(block, 40); return FieldTypeHelper.getInstance(m_file, fieldIndex);
        let field_index = checked_int(block, 40)?;
        FieldTypeHelper::get_instance(file, field_index)
    }

    /// Corresponds to Java: `@Override protected int getTextOffset(byte[] block)`
    fn get_text_offset(&self, block: &[u8]) -> i32 {
// Java: return ByteArrayHelper.getShort(block, 68); (throws on out of bounds; the Rust side treats it as 0)
        checked_short(block, 68).unwrap_or(0)
    }

    /// Corresponds to Java: `@Override protected int getPromptOffset(byte[] block)`
    fn get_prompt_offset(&self, block: &[u8]) -> i32 {
// Java: return ByteArrayHelper.getShort(block, 72); (throws on out of bounds; the Rust side treats it as 0)
        checked_short(block, 72).unwrap_or(0)
    }

    /// Corresponds to Java: `@Override protected int getValueOffset()`
    fn get_value_offset(&self) -> i32 {
        32
    }

    /// Corresponds to Java: `@Override protected int getTimeUnitsOffset()`
    fn get_time_units_offset(&self) -> i32 {
        42
    }

    /// Corresponds to Java: `@Override protected int getCriteriaTextStartOffset()`
    fn get_criteria_text_start_offset(&self) -> i32 {
        16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_key::FieldKey;
    use crate::task_field::TaskField;

/// Construct an 80-byte block, writing the block pointer and field number.
    fn block(child: i32, field_id: i32) -> Vec<u8> {
        let mut block = vec![0u8; 80];
        block[74..76].copy_from_slice(&(child as u16).to_le_bytes());
        block[40..44].copy_from_slice(&field_id.to_le_bytes());
        block
    }

/// The offset constants match Java.
    #[test]
    fn offsets_match_java() {
        let reader = FilterCriteriaReader12::new();
        assert_eq!(reader.get_criteria_start_offset(), 20);
        assert_eq!(reader.get_criteria_block_size(), 80);
        assert_eq!(reader.get_value_offset(), 32);
        assert_eq!(reader.get_time_units_offset(), 42);
        assert_eq!(reader.get_criteria_text_start_offset(), 16);
    }

/// Sub-block parsing and field number mapping.
    #[test]
    fn child_block_and_field_type() {
        let reader = FilterCriteriaReader12::new();
        let file = ProjectFile::new();

        let mut blocks = CriteriaBlockMap::new();
        blocks.insert(100, vec![0x33u8; 80]);

        let field_id = FieldTypeHelper::get_field_id(Some(&MppFieldType::Builtin(FieldKey::Task(
            TaskField::Cost,
        ))));

// Java's Cost field is also CURRENCY in MPP12
        assert_eq!(
            reader
                .get_field_type(&file, &block(100, field_id))
                .and_then(|field| field.as_builtin()),
            Some(FieldKey::Task(TaskField::Cost))
        );
        assert_eq!(
            reader.get_child_block(&block(100, field_id), &blocks),
            Some(vec![0x33u8; 80])
        );
        assert!(reader.get_child_block(&[0u8; 4], &blocks).is_none());
    }

/// The default constructor is equivalent to `new`.
    #[test]
    fn default_matches_new() {
        assert_eq!(
            FilterCriteriaReader12::default().get_criteria_start_offset(),
            FilterCriteriaReader12::new().get_criteria_start_offset()
        );
    }
}
