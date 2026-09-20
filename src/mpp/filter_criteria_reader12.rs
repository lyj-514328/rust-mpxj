//! 对应 Java: `org/mpxj/mpp/FilterCriteriaReader12.java`
//!
//! MPP12 的筛选条件块读取器：告诉 [`CriteriaReader`] 12 版文件里条件块的布局。
//!
//! # 继承的还原（trait 实现）
//!
//! Java 的 `final class FilterCriteriaReader12 extends CriteriaReader`。Rust 侧改为
//! **实现 [`CriteriaReader`] trait 的零尺寸结构体**；Java 继承来的公开方法 `process`
//! 由 trait 的默认方法提供。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `m_criteriaBlockMap`（继承自 `CriteriaReader` 的 `protected` 字段） | 方法入参 `blocks: &CriteriaBlockMap` | 见 [`crate::mpp::criteria_reader`] |
//! | `ByteArrayHelper.getShort/getInt` 越界抛异常 | [`checked_short`] / [`checked_int`] 返回 `None` | 遵循「不 panic」约定 |
//! | 与本 crate 的 `FilterCriteriaReader9` **实现逐字相同**（Java 侧同样如此） | 同样保留重复实现 | Java 两个类之间没有继承关系，Rust 侧不额外引入抽象 |
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

/// MPP12 筛选条件块读取器。
///
/// 对应 Java: `final class FilterCriteriaReader12 extends CriteriaReader`
pub struct FilterCriteriaReader12;

impl FilterCriteriaReader12 {
    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法（Java 侧未显式声明构造方法）。
    pub fn new() -> Self {
        Self
    }
}

impl Default for FilterCriteriaReader12 {
    /// 对应 Java: 隐式默认构造方法，等价于 [`FilterCriteriaReader12::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl CriteriaReader for FilterCriteriaReader12 {
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
    use crate::task_field::TaskField;

    /// 构造一个 80 字节的块，写入块指针与字段号。
    fn block(child: i32, field_id: i32) -> Vec<u8> {
        let mut block = vec![0u8; 80];
        block[74..76].copy_from_slice(&(child as u16).to_le_bytes());
        block[40..44].copy_from_slice(&field_id.to_le_bytes());
        block
    }

    /// 偏移常量与 Java 一致。
    #[test]
    fn offsets_match_java() {
        let reader = FilterCriteriaReader12::new();
        assert_eq!(reader.get_criteria_start_offset(), 20);
        assert_eq!(reader.get_criteria_block_size(), 80);
        assert_eq!(reader.get_value_offset(), 32);
        assert_eq!(reader.get_time_units_offset(), 42);
        assert_eq!(reader.get_criteria_text_start_offset(), 16);
    }

    /// 子块解析与字段号映射。
    #[test]
    fn child_block_and_field_type() {
        let reader = FilterCriteriaReader12::new();
        let file = ProjectFile::new();

        let mut blocks = CriteriaBlockMap::new();
        blocks.insert(100, vec![0x33u8; 80]);

        let field_id = FieldTypeHelper::get_field_id(Some(&MppFieldType::Builtin(FieldKey::Task(
            TaskField::Cost,
        ))));

        // Java 的 Cost 字段在 MPP12 里同样是 CURRENCY
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

    /// 默认构造与 `new` 等价。
    #[test]
    fn default_matches_new() {
        assert_eq!(
            FilterCriteriaReader12::default().get_criteria_start_offset(),
            FilterCriteriaReader12::new().get_criteria_start_offset()
        );
    }
}
