//! 对应 Java: `org/mpxj/mpp/FilterCriteriaReader14.java`
//!
//! MPP14 的筛选条件块读取器：告诉 [`CriteriaReader`] 14 版文件里条件块的布局。
//! 与 9 / 12 相比，14 版的块更短（36 字节）且各字段整体前移。
//!
//! # 继承的还原（trait 实现）
//!
//! Java 的 `final class FilterCriteriaReader14 extends CriteriaReader`。Rust 侧改为
//! **实现 [`CriteriaReader`] trait 的零尺寸结构体**；Java 继承来的公开方法 `process`
//! 由 trait 的默认方法提供。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `m_criteriaBlockMap`（继承自 `CriteriaReader` 的 `protected` 字段） | 方法入参 `blocks: &CriteriaBlockMap` | 见 [`crate::mpp::criteria_reader`] |
//! | `FieldTypeHelper.mapTextFields(FieldTypeHelper.getInstance(m_file, fieldIndex))` | [`FieldTypeHelper::map_text_fields`] | 把 `START_TEXT` / `FINISH_TEXT` / `DURATION_TEXT` 映射回非文本版字段 |
//! | `ByteArrayHelper.getShort/getInt` 越界抛异常 | [`checked_short`] / [`checked_int`] 返回 `None` | 遵循「不 panic」约定 |
//!
//! 跳过的 Java 成员：无。
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::mpp_field_type::MppFieldType;
use crate::mpp::criteria_reader::{CriteriaBlockMap, CriteriaReader};
use crate::mpp::custom_field_value_reader::{checked_int, checked_short};
use crate::project_file::ProjectFile;

/// MPP14 筛选条件块读取器。
///
/// 对应 Java: `final class FilterCriteriaReader14 extends CriteriaReader`
pub struct FilterCriteriaReader14;

impl FilterCriteriaReader14 {
    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法（Java 侧未显式声明构造方法）。
    pub fn new() -> Self {
        Self
    }
}

impl Default for FilterCriteriaReader14 {
    /// 对应 Java: 隐式默认构造方法，等价于 [`FilterCriteriaReader14::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl CriteriaReader for FilterCriteriaReader14 {
    /// 对应 Java: `@Override protected int getCriteriaStartOffset()`
    fn get_criteria_start_offset(&self) -> i32 {
        20
    }

    /// 对应 Java: `@Override protected int getCriteriaBlockSize()`
    fn get_criteria_block_size(&self) -> i32 {
        36
    }

    /// 对应 Java: `@Override protected byte[] getChildBlock(byte[] block)`
    fn get_child_block(&self, block: &[u8], blocks: &CriteriaBlockMap) -> Option<Vec<u8>> {
        // Java: int offset = ByteArrayHelper.getShort(block, 32); return m_criteriaBlockMap.get(Integer.valueOf(offset));
        let offset = checked_short(block, 32)?;
        blocks.get(&offset).cloned()
    }

    /// 对应 Java: `@Override protected byte[] getListNextBlock(byte[] block)`
    fn get_list_next_block(&self, block: &[u8], blocks: &CriteriaBlockMap) -> Option<Vec<u8>> {
        // Java: int offset = ByteArrayHelper.getShort(block, 34); return m_criteriaBlockMap.get(Integer.valueOf(offset));
        let offset = checked_short(block, 34)?;
        blocks.get(&offset).cloned()
    }

    /// 对应 Java: `@Override protected FieldType getFieldType(byte[] block)`
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

    /// 对应 Java: `@Override protected int getTextOffset(byte[] block)`
    fn get_text_offset(&self, block: &[u8]) -> i32 {
        // Java: return ByteArrayHelper.getShort(block, 26);（越界抛异常，Rust 侧按 0 处理）
        checked_short(block, 26).unwrap_or(0)
    }

    /// 对应 Java: `@Override protected int getPromptOffset(byte[] block)`
    fn get_prompt_offset(&self, block: &[u8]) -> i32 {
        // Java: return ByteArrayHelper.getShort(block, 30);（越界抛异常，Rust 侧按 0 处理）
        checked_short(block, 30).unwrap_or(0)
    }

    /// 对应 Java: `@Override protected int getValueOffset()`
    fn get_value_offset(&self) -> i32 {
        14
    }

    /// 对应 Java: `@Override protected int getTimeUnitsOffset()`
    fn get_time_units_offset(&self) -> i32 {
        24
    }

    /// 对应 Java: `@Override protected int getCriteriaTextStartOffset()`
    fn get_criteria_text_start_offset(&self) -> i32 {
        16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_key::FieldKey;
    use crate::task_field::TaskField;

    /// 构造一个 36 字节的块，写入块指针与字段号。
    fn block(child: i32, next: i32, field_id: i32) -> Vec<u8> {
        let mut block = vec![0u8; 36];
        block[8..12].copy_from_slice(&field_id.to_le_bytes());
        block[32..34].copy_from_slice(&(child as u16).to_le_bytes());
        block[34..36].copy_from_slice(&(next as u16).to_le_bytes());
        block[26..28].copy_from_slice(&6i32.to_le_bytes()[..2]);
        block[30..32].copy_from_slice(&10i32.to_le_bytes()[..2]);
        block
    }

    /// 偏移常量与 Java 一致。
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

    /// 块指针与字段号（含文本字段映射）。
    #[test]
    fn pointers_and_text_field_mapping() {
        let reader = FilterCriteriaReader14::new();
        let file = ProjectFile::new();

        let mut blocks = CriteriaBlockMap::new();
        blocks.insert(20, vec![0x44u8; 36]);
        blocks.insert(56, vec![0x55u8; 36]);

        // START_TEXT 会被映射回 START
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

        // 块太短 ⇒ None
        assert!(reader.get_child_block(&[0u8; 4], &blocks).is_none());
        assert!(reader.get_list_next_block(&[0u8; 4], &blocks).is_none());
        assert!(reader.get_field_type(&file, &[0u8; 4]).is_none());
    }

    /// 默认构造与 `new` 等价。
    #[test]
    fn default_matches_new() {
        assert_eq!(
            FilterCriteriaReader14::default().get_criteria_block_size(),
            FilterCriteriaReader14::new().get_criteria_block_size()
        );
    }
}
