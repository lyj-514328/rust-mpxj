//! Corresponds to Java: `org/mpxj/mpp/FilterCriteriaReader14.java`
//!
//! Filter criteria block reader for MPP14: tells [`CriteriaReader`] the layout of the criteria block in version 14 files.
//! Compared with 9 / 12, the version 14 block is shorter (36 bytes) and the fields are shifted forward overall.
//!
//! # How the inheritance is restored (trait implementation)
//!
//! Java's `final class FilterCriteriaReader14 extends CriteriaReader`. The Rust side is changed to
//! **a zero-sized struct implementing the [`CriteriaReader`] trait**; the public method `process` inherited from Java
//! is provided by the trait's default method.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `m_criteriaBlockMap` (`protected` field inherited from `CriteriaReader`) | Method parameter `blocks: &CriteriaBlockMap` | See [`crate::mpp::criteria_reader`] |
//! | `FieldTypeHelper.mapTextFields(FieldTypeHelper.getInstance(m_file, fieldIndex))` | [`FieldTypeHelper::map_text_fields`] | Map `START_TEXT` / `FINISH_TEXT` / `DURATION_TEXT` back to the non-text fields |
//! | `ByteArrayHelper.getShort/getInt` throws on out of bounds | [`checked_short`] / [`checked_int`] return `None` | Follows the "do not panic" convention |
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

/// MPP14 filter criteria block reader.
///
/// Corresponds to Java: `final class FilterCriteriaReader14 extends CriteriaReader`
pub struct FilterCriteriaReader14;

impl FilterCriteriaReader14 {
/// Constructor.
    ///
/// Corresponds to Java: implicit default constructor (Java does not declare a constructor explicitly).
    pub fn new() -> Self {
        Self
    }
}

impl Default for FilterCriteriaReader14 {
/// Corresponds to Java: implicit default constructor, equivalent to [`FilterCriteriaReader14::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl CriteriaReader for FilterCriteriaReader14 {
    /// Corresponds to Java: `@Override protected int getCriteriaStartOffset()`
    fn get_criteria_start_offset(&self) -> i32 {
        20
    }

    /// Corresponds to Java: `@Override protected int getCriteriaBlockSize()`
    fn get_criteria_block_size(&self) -> i32 {
        36
    }

    /// Corresponds to Java: `@Override protected byte[] getChildBlock(byte[] block)`
    fn get_child_block(&self, block: &[u8], blocks: &CriteriaBlockMap) -> Option<Vec<u8>> {
        // Java: int offset = ByteArrayHelper.getShort(block, 32); return m_criteriaBlockMap.get(Integer.valueOf(offset));
        let offset = checked_short(block, 32)?;
        blocks.get(&offset).cloned()
    }

    /// Corresponds to Java: `@Override protected byte[] getListNextBlock(byte[] block)`
    fn get_list_next_block(&self, block: &[u8], blocks: &CriteriaBlockMap) -> Option<Vec<u8>> {
        // Java: int offset = ByteArrayHelper.getShort(block, 34); return m_criteriaBlockMap.get(Integer.valueOf(offset));
        let offset = checked_short(block, 34)?;
        blocks.get(&offset).cloned()
    }

    /// Corresponds to Java: `@Override protected FieldType getFieldType(byte[] block)`
    fn get_field_type(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        block: &[u8],
    ) -> Option<MppFieldType> {
        // Java: int fieldIndex = ByteArrayHelper.getInt(block, 8);
        //       return FieldTypeHelper.mapTextFields(FieldTypeHelper.getInstance(m_file, fieldIndex));
        let field_index = checked_int(block, 8)?;
        FieldTypeHelper::get_instance(file, field_index).map(FieldTypeHelper::map_text_fields)
    }

    /// Corresponds to Java: `@Override protected int getTextOffset(byte[] block)`
    fn get_text_offset(&self, block: &[u8]) -> i32 {
// Java: return ByteArrayHelper.getShort(block, 26); (throws on out of bounds; the Rust side treats it as 0)
        checked_short(block, 26).unwrap_or(0)
    }

    /// Corresponds to Java: `@Override protected int getPromptOffset(byte[] block)`
    fn get_prompt_offset(&self, block: &[u8]) -> i32 {
// Java: return ByteArrayHelper.getShort(block, 30); (throws on out of bounds; the Rust side treats it as 0)
        checked_short(block, 30).unwrap_or(0)
    }

    /// Corresponds to Java: `@Override protected int getValueOffset()`
    fn get_value_offset(&self) -> i32 {
        14
    }

    /// Corresponds to Java: `@Override protected int getTimeUnitsOffset()`
    fn get_time_units_offset(&self) -> i32 {
        24
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

/// Build a 36-byte block, writing the block pointer and field number.
    fn block(child: i32, next: i32, field_id: i32) -> Vec<u8> {
        let mut block = vec![0u8; 36];
        block[8..12].copy_from_slice(&field_id.to_le_bytes());
        block[32..34].copy_from_slice(&(child as u16).to_le_bytes());
        block[34..36].copy_from_slice(&(next as u16).to_le_bytes());
        block[26..28].copy_from_slice(&6i32.to_le_bytes()[..2]);
        block[30..32].copy_from_slice(&10i32.to_le_bytes()[..2]);
        block
    }

/// The offset constants are consistent with Java.
    #[test]
    fn offsets_match_java() {
        let reader = FilterCriteriaReader14::new();
        assert_eq!(reader.get_criteria_start_offset(), 20);
        assert_eq!(reader.get_criteria_block_size(), 36);
        assert_eq!(reader.get_value_offset(), 14);
        assert_eq!(reader.get_time_units_offset(), 24);
        assert_eq!(reader.get_criteria_text_start_offset(), 16);

        let block = block(0, 0, 0);
        assert_eq!(reader.get_text_offset(&block), 6);
        assert_eq!(reader.get_prompt_offset(&block), 10);
    }

/// Block pointer and field number (including text field mapping).
    #[test]
    fn pointers_and_text_field_mapping() {
        let reader = FilterCriteriaReader14::new();
        let file = ProjectFile::new();

        let mut blocks = CriteriaBlockMap::new();
        blocks.insert(20, vec![0x44u8; 36]);
        blocks.insert(56, vec![0x55u8; 36]);

// START_TEXT is mapped back to START
        let start_text_id =
            FieldTypeHelper::get_field_id(Some(&MppFieldType::Builtin(FieldKey::Task(
                TaskField::StartText,
            ))));
        assert_eq!(
            reader
                .get_field_type(&file, &block(20, 0, start_text_id))
                .and_then(|field| field.as_builtin()),
            Some(FieldKey::Task(TaskField::Start))
        );

        let block = block(20, 56, 0);
        assert_eq!(reader.get_child_block(&block, &blocks), Some(vec![0x44u8; 36]));
        assert_eq!(
            reader.get_list_next_block(&block, &blocks),
            Some(vec![0x55u8; 36])
        );

// Block too short ⇒ None
        assert!(reader.get_child_block(&[0u8; 4], &blocks).is_none());
        assert!(reader.get_list_next_block(&[0u8; 4], &blocks).is_none());
        assert!(reader.get_field_type(&file, &[0u8; 4]).is_none());
    }

/// The default constructor is equivalent to `new`.
    #[test]
    fn default_matches_new() {
        assert_eq!(
            FilterCriteriaReader14::default().get_criteria_block_size(),
            FilterCriteriaReader14::new().get_criteria_block_size()
        );
    }
}
