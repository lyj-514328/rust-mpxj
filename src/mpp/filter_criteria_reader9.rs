//! 对应 Java: `org/mpxj/mpp/FilterCriteriaReader9.java`
//!
//! MPP9 的筛选条件块读取器：告诉 [`CriteriaReader`] 9 版文件里条件块的布局。
//!
//! # 继承的还原（trait 实现）
//!
//! Java 的 `class FilterCriteriaReader9 extends CriteriaReader`。Rust 侧改为
//! **实现 [`CriteriaReader`] trait 的零尺寸结构体**：Java 的 10 个
//! `@Override protected` 方法逐一对应 trait 的必需方法，Java 继承来的公开方法 `process`
//! 由 trait 的默认方法提供（无需在本类重复实现）。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `m_criteriaBlockMap`（继承自 `CriteriaReader` 的 `protected` 字段） | 方法入参 `blocks: &CriteriaBlockMap` | Rust 的 trait 不能带字段，见 [`crate::mpp::criteria_reader`] |
//! | `ByteArrayHelper.getShort(block, n)` 越界抛 `ArrayIndexOutOfBoundsException` | [`checked_short`] 返回 `None`（块指针解析不出 ⇒ 无子块 / 无兄弟块） | 遵循「不 panic」约定 |
//! | `ByteArrayHelper.getInt(block, 40)` 越界抛异常 | [`checked_int`] 返回 `None` ⇒ 字段类型为 `None` | 同上 |
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

/// MPP9 筛选条件块读取器。
///
/// 对应 Java: `class FilterCriteriaReader9 extends CriteriaReader`
pub struct FilterCriteriaReader9;

impl FilterCriteriaReader9 {
    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法（Java 侧未显式声明构造方法）。
    pub fn new() -> Self {
        Self
    }
}

impl Default for FilterCriteriaReader9 {
    /// 对应 Java: 隐式默认构造方法，等价于 [`FilterCriteriaReader9::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl CriteriaReader for FilterCriteriaReader9 {
    /// 对应 Java: `@Override protected int getCriteriaStartOffset()`
    fn get_criteria_start_offset(&self) -> i32 {
        20
    }

    /// 对应 Java: `@Override protected int getCriteriaBlockSize()`
    fn get_criteria_block_size(&self) -> i32 {
        80
    }

    /// 对应 Java: `@Override protected byte[] getChildBlock(byte[] block)`
    fn get_child_block(&self, block: &[u8], blocks: &CriteriaBlockMap) -> Option<Vec<u8>> {
        // Java: int offset = ByteArrayHelper.getShort(block, 74); return m_criteriaBlockMap.get(Integer.valueOf(offset));
        let offset = checked_short(block, 74)?;
        blocks.get(&offset).cloned()
    }

    /// 对应 Java: `@Override protected byte[] getListNextBlock(byte[] block)`
    fn get_list_next_block(&self, block: &[u8], blocks: &CriteriaBlockMap) -> Option<Vec<u8>> {
        // Java: int offset = ByteArrayHelper.getShort(block, 76); return m_criteriaBlockMap.get(Integer.valueOf(offset));
        let offset = checked_short(block, 76)?;
        blocks.get(&offset).cloned()
    }

    /// 对应 Java: `@Override protected FieldType getFieldType(byte[] block)`
    fn get_field_type(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        block: &[u8],
    ) -> Option<MppFieldType> {
        // Java: int fieldIndex = ByteArrayHelper.getInt(block, 40); return FieldTypeHelper.getInstance(m_file, fieldIndex);
        let field_index = checked_int(block, 40)?;
        FieldTypeHelper::get_instance(file, field_index)
    }

    /// 对应 Java: `@Override protected int getTextOffset(byte[] block)`
    fn get_text_offset(&self, block: &[u8]) -> i32 {
        // Java: return ByteArrayHelper.getShort(block, 68);（越界抛异常，Rust 侧按 0 处理）
        checked_short(block, 68).unwrap_or(0)
    }

    /// 对应 Java: `@Override protected int getPromptOffset(byte[] block)`
    fn get_prompt_offset(&self, block: &[u8]) -> i32 {
        // Java: return ByteArrayHelper.getShort(block, 72);（越界抛异常，Rust 侧按 0 处理）
        checked_short(block, 72).unwrap_or(0)
    }

    /// 对应 Java: `@Override protected int getValueOffset()`
    fn get_value_offset(&self) -> i32 {
        32
    }

    /// 对应 Java: `@Override protected int getTimeUnitsOffset()`
    fn get_time_units_offset(&self) -> i32 {
        42
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
    use crate::mpp::criteria_reader::CriteriaReader;
    use crate::task_field::TaskField;

    /// 构造一个 80 字节的块，写入块指针与字段号。
    ///
    /// 注意：**不要**同时写「工期单位」偏移（42），它与字段号（40 起 4 字节）在 Java 的
    /// 布局中重叠——`getFieldType` 读的是 40 处的 4 字节整数，而 `getTimeUnitsOffset()` 为 42。
    fn make_block(child: i32, next: i32, field_id: i32) -> Vec<u8> {
        let mut block = vec![0u8; 80];
        block[74..76].copy_from_slice(&(child as u16).to_le_bytes());
        block[76..78].copy_from_slice(&(next as u16).to_le_bytes());
        block[40..44].copy_from_slice(&field_id.to_le_bytes());
        block[68..70].copy_from_slice(&4i32.to_le_bytes()[..2]);
        block[72..74].copy_from_slice(&8i32.to_le_bytes()[..2]);
        block
    }

    /// 偏移常量与 Java 一致。
    #[test]
    fn offsets_match_java() {
        let reader = FilterCriteriaReader9::new();
        assert_eq!(reader.get_criteria_start_offset(), 20);
        assert_eq!(reader.get_criteria_block_size(), 80);
        assert_eq!(reader.get_value_offset(), 32);
        assert_eq!(reader.get_time_units_offset(), 42);
        assert_eq!(reader.get_criteria_text_start_offset(), 16);

        let block = make_block(0, 0, 0);
        // 文本偏移在 68、提示偏移在 72
        assert_eq!(reader.get_text_offset(&block), 4);
        assert_eq!(reader.get_prompt_offset(&block), 8);
    }

    /// 子块 / 兄弟块按块映射解析。
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

        // 指向不存在的偏移 ⇒ None
        let block = make_block(999, 999, 0);
        assert!(reader.get_child_block(&block, &blocks).is_none());
        assert!(reader.get_list_next_block(&block, &blocks).is_none());

        // 块太短（读不出指针）⇒ None，不 panic
        assert!(reader.get_child_block(&[0u8; 10], &blocks).is_none());
        assert!(reader.get_list_next_block(&[0u8; 10], &blocks).is_none());
    }

    /// 字段号映射为字段类型。
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

        // 未识别的字段号 ⇒ None
        let block = make_block(0, 0, 0x1234_0005);
        assert!(reader.get_field_type(&file, &block).is_none());

        // 块太短 ⇒ None
        assert!(reader.get_field_type(&file, &[0u8; 10]).is_none());
    }

    /// 默认构造与 `new` 等价。
    #[test]
    fn default_matches_new() {
        assert_eq!(
            FilterCriteriaReader9::default().get_criteria_block_size(),
            FilterCriteriaReader9::new().get_criteria_block_size()
        );
    }
}
