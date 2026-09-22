//! Corresponds to Java: `org/mpxj/mpp/FilterCriteriaReader9.java`
//!
//! Filter criteria block reader for MPP9: tells [`CriteriaReader`] the layout of the criteria block in version 9 files.
//!
//! # How inheritance is reproduced (trait implementation)
//!
//! Java's `class FilterCriteriaReader9 extends CriteriaReader`. The Rust side instead becomes
//! **a zero-sized struct implementing the [`CriteriaReader`] trait**: Java's 10
//! `@Override protected` methods correspond one-to-one to the trait's required methods, and the public `process` method inherited in Java
//! is provided by the trait's default method (no need to reimplement it in this class).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `m_criteriaBlockMap` (a `protected` field inherited from `CriteriaReader`) | method parameter `blocks: &CriteriaBlockMap` | Rust traits cannot have fields; see [`crate::mpp::criteria_reader`] |
//! | `ByteArrayHelper.getShort(block, n)` throws `ArrayIndexOutOfBoundsException` on out of bounds | [`checked_short`] returns `None` (the block pointer cannot be parsed ⇒ no child block / no sibling block) | follows the "no panic" convention |
//! | `ByteArrayHelper.getInt(block, 40)` throws on out of bounds | [`checked_int`] returns `None` ⇒ the field type is `None` | same as above |
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

/// MPP9 filter criteria block reader.
///
/// Corresponds to Java: `class FilterCriteriaReader9 extends CriteriaReader`
pub struct FilterCriteriaReader9;

impl FilterCriteriaReader9 {
/// Constructor.
    ///
/// Corresponds to Java: implicit default constructor (Java does not explicitly declare a constructor).
    pub fn new() -> Self {
        Self
    }
}

impl Default for FilterCriteriaReader9 {
/// Corresponds to Java: implicit default constructor, equivalent to [`FilterCriteriaReader9::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl CriteriaReader for FilterCriteriaReader9 {
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
// Java: return ByteArrayHelper.getShort(block, 68); (throws on out of bounds; Rust treats it as 0)
        checked_short(block, 68).unwrap_or(0)
    }

    /// Corresponds to Java: `@Override protected int getPromptOffset(byte[] block)`
    fn get_prompt_offset(&self, block: &[u8]) -> i32 {
// Java: return ByteArrayHelper.getShort(block, 72); (throws on out of bounds; Rust treats it as 0)
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
    use crate::mpp::criteria_reader::CriteriaReader;
    use crate::task_field::TaskField;

/// Constructs an 80-byte block and writes the block pointer and field number.
    ///
/// Note: do **not** also write the "time units" offset (42); it overlaps with the field number (4 bytes starting at 40) in Java's
/// layout — `getFieldType` reads the 4-byte integer at 40, while `getTimeUnitsOffset()` is 42.
    fn make_block(child: i32, next: i32, field_id: i32) -> Vec<u8> {
        let mut block = vec![0u8; 80];
        block[74..76].copy_from_slice(&(child as u16).to_le_bytes());
        block[76..78].copy_from_slice(&(next as u16).to_le_bytes());
        block[40..44].copy_from_slice(&field_id.to_le_bytes());
        block[68..70].copy_from_slice(&4i32.to_le_bytes()[..2]);
        block[72..74].copy_from_slice(&8i32.to_le_bytes()[..2]);
        block
    }

/// Offset constants match Java.
    #[test]
    fn offsets_match_java() {
        let reader = FilterCriteriaReader9::new();
        assert_eq!(reader.get_criteria_start_offset(), 20);
        assert_eq!(reader.get_criteria_block_size(), 80);
        assert_eq!(reader.get_value_offset(), 32);
        assert_eq!(reader.get_time_units_offset(), 42);
        assert_eq!(reader.get_criteria_text_start_offset(), 16);

        let block = make_block(0, 0, 0);
// text offset at 68, prompt offset at 72
        assert_eq!(reader.get_text_offset(&block), 4);
        assert_eq!(reader.get_prompt_offset(&block), 8);
    }

/// Child / sibling blocks are resolved through the block map.
    #[test]
    fn block_pointers_resolve_through_map() {
        let reader = FilterCriteriaReader9::new();
        let mut blocks = CriteriaBlockMap::new();
        blocks.insert(100, vec![0x11u8; 80]);
        blocks.insert(180, vec![0x22u8; 80]);

        let block = make_block(100, 180, 0);
        assert_eq!(reader.get_child_block(&block, &blocks), Some(vec![0x11u8; 80]));
        assert_eq!(
            reader.get_list_next_block(&block, &blocks),
            Some(vec![0x22u8; 80])
        );

// points to a nonexistent offset ⇒ None
        let block = make_block(999, 999, 0);
        assert!(reader.get_child_block(&block, &blocks).is_none());
        assert!(reader.get_list_next_block(&block, &blocks).is_none());

// block too short (pointer cannot be read) ⇒ None, no panic
        assert!(reader.get_child_block(&[0u8; 10], &blocks).is_none());
        assert!(reader.get_list_next_block(&[0u8; 10], &blocks).is_none());
    }

/// Maps the field number to a field type.
    #[test]
    fn field_type_is_resolved_from_block() {
        let reader = FilterCriteriaReader9::new();
        let file = ProjectFile::new();
        let field = FieldTypeHelper::get_field_id(Some(&MppFieldType::Builtin(FieldKey::Task(
            TaskField::Name,
        ))));

        let block = make_block(0, 0, field);
        assert_eq!(
            reader
                .get_field_type(&file, &block)
                .and_then(|field| field.as_builtin()),
            Some(FieldKey::Task(TaskField::Name))
        );

// unrecognized field number ⇒ None
        let block = make_block(0, 0, 0x1234_0005);
        assert!(reader.get_field_type(&file, &block).is_none());

// block too short ⇒ None
        assert!(reader.get_field_type(&file, &[0u8; 10]).is_none());
    }

/// Default construction is equivalent to `new`.
    #[test]
    fn default_matches_new() {
        assert_eq!(
            FilterCriteriaReader9::default().get_criteria_block_size(),
            FilterCriteriaReader9::new().get_criteria_block_size()
        );
    }
}
