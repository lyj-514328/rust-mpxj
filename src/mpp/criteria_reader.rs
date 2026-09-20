//! 对应 Java: `org/mpxj/mpp/CriteriaReader.java`
//!
//! 从 MPP 文件里读出「筛选 / 分组 / 自动筛选」条件（criteria）的公共实现：条件数据是一串
//! 定长小块，块之间用「子块偏移 / 下一个兄弟块偏移」串成树，本类把它递归还原成
//! [`GenericCriteria`] 树。
//!
//! # 继承的还原（trait 默认方法）
//!
//! Java 的 `abstract class CriteriaReader` 由
//! [`FilterCriteriaReader9`](crate::mpp::filter_criteria_reader9::FilterCriteriaReader9) /
//! [`FilterCriteriaReader12`](crate::mpp::filter_criteria_reader12::FilterCriteriaReader12) /
//! [`FilterCriteriaReader14`](crate::mpp::filter_criteria_reader14::FilterCriteriaReader14)
//! 继承。Rust 没有继承，故：
//!
//! - 本类成为 [`CriteriaReader`] **trait**：Java 的 10 个 `protected abstract` 方法
//!   对应 trait 的必需方法；子类各自以零尺寸结构体实现本 trait（无字段可继承）。
//! - Java 的公开方法 `process` 是**具体方法**，Rust 侧改为 trait 的**默认方法**，
//!   子类无需重复实现。递归所需的调用期状态放在私有的 [`CriteriaState`] 里。
//! - Java 的 `protected final Map<Integer, byte[]> m_criteriaBlockMap` 被子类的
//!   `getChildBlock` / `getListNextBlock` 直接读取；Rust 的 trait 不能带字段，故把该映射
//!   作为参数传给这两个方法（见 [`CriteriaBlockMap`]）。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `protected ProjectFile m_file` | 参数 `file: &Rc<RefCell<ProjectFile>>` | 本 crate 以 `Rc<RefCell<ProjectFile>>` 共享项目文件 |
//! | `private ProjectProperties m_properties` | 不保存引用，按需 `try_borrow` 借用 | `ProjectProperties` 由 `ProjectFile` 按值持有（与 `Var2Data` / `CustomFieldValueReader` 的既有取舍一致） |
//! | `protected final Map<Integer, byte[]> m_criteriaBlockMap` | [`CriteriaBlockMap`]（`BTreeMap`，与 Java `TreeMap` 的遍历顺序一致）+ 方法入参 | trait 不能带字段 |
//! | `Object`（右值） | [`Operand`]（`FieldType` → `Operand::Field`，`GenericCriteriaPrompt` → `Operand::Prompt`，标量 → `Operand::Value`） | 与 [`GenericCriteria`] 的右值表示一致 |
//! | `FieldType` | [`MppFieldType`]，写入 [`GenericCriteria`] 时经 `to_field_key()` 转 [`FieldKey`] | 见 `crate::common::mpp_field_type` |
//! | `List<GenericCriteriaPrompt>` / `List<FieldType>` / `boolean[]`（可选出参，`null` 表示不收集） | `Option<&mut Vec<GenericCriteriaPrompt>>` / `Option<&mut Vec<FieldKey>>` / `Option<&mut [bool; 2]>` | `null` → `None`；`boolean[]` 恒为长度 2 |
//! | `ByteArrayHelper.getShort/getInt` 越界抛 `ArrayIndexOutOfBoundsException` | [`checked_short`] / [`checked_int`]（复用 `crate::mpp::custom_field_value_reader` 的同名工具）返回 `None` | 块被截断时 Java 会让整次读取失败；Rust 侧退化为「该条件解析不出」并逐点注释 |
//! | `TestOperator.getInstance(int)` 越界抛异常（反射建表按下标取值） | [`TestOperator::get_instance`] 回落到 `IS_ANY_VALUE` | 见 `crate::test_operator` 的说明 |
//! | `leftValue` 为 `null` 时的 `leftValue.getFieldTypeClass()` / `field.getDataType()`（Java 抛 NPE） | 跳过该分支（不写 `criteriaType` / 不取数据类型） | 遵循「不 panic」约定 |
//! | Java `getPromptValue` 计算文本偏移时**没有**加 `m_dataOffset`（与 `getConstantValue` 的 STRING 分支不一致） | 原样保留该行为 | 上游按原样保留；调用方（`FilterReader`）的 `dataOffset` 恒为 0，不受影响 |
//!
//! 跳过的 Java 成员：无（全部方法均已翻译）。
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

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

/// 条件数据块映射：块在数据体中的偏移 → 块内容。
///
/// 对应 Java: `protected final Map<Integer, byte[]> m_criteriaBlockMap = new TreeMap<>();`
pub type CriteriaBlockMap = BTreeMap<i32, Vec<u8>>;

/// 条件读取器。
///
/// 对应 Java: `abstract class CriteriaReader`
pub trait CriteriaReader {
    /// 取条件数据的起始偏移（第一块的位置）。
    ///
    /// 对应 Java: `protected abstract int getCriteriaStartOffset()`
    fn get_criteria_start_offset(&self) -> i32;

    /// 取单个条件块的字节数。
    ///
    /// 对应 Java: `protected abstract int getCriteriaBlockSize()`
    fn get_criteria_block_size(&self) -> i32;

    /// 取当前块的子块（树结构中的下一层）。
    ///
    /// 对应 Java: `protected abstract byte[] getChildBlock(byte[] block)`
    ///
    /// Rust 侧额外接收块映射（Java 从 `m_criteriaBlockMap` 字段读取）。
    fn get_child_block(&self, block: &[u8], blocks: &CriteriaBlockMap) -> Option<Vec<u8>>;

    /// 取当前块的「列表中的下一个兄弟块」。
    ///
    /// 对应 Java: `protected abstract byte[] getListNextBlock(byte[] block)`
    fn get_list_next_block(&self, block: &[u8], blocks: &CriteriaBlockMap) -> Option<Vec<u8>>;

    /// 取文本块在条件文本区中的偏移。
    ///
    /// 对应 Java: `protected abstract int getTextOffset(byte[] block)`
    fn get_text_offset(&self, block: &[u8]) -> i32;

    /// 取提示文本的偏移。
    ///
    /// 对应 Java: `protected abstract int getPromptOffset(byte[] block)`
    fn get_prompt_offset(&self, block: &[u8]) -> i32;

    /// 取字段取值的偏移。
    ///
    /// 对应 Java: `protected abstract int getValueOffset()`
    fn get_value_offset(&self) -> i32;

    /// 取工期单位的偏移。
    ///
    /// 对应 Java: `protected abstract int getTimeUnitsOffset()`
    fn get_time_units_offset(&self) -> i32;

    /// 取「条件文本区起始位置」在块中的偏移。
    ///
    /// 对应 Java: `protected abstract int getCriteriaTextStartOffset()`
    fn get_criteria_text_start_offset(&self) -> i32;

    /// 取块中描述的字段类型。
    ///
    /// 对应 Java: `protected abstract FieldType getFieldType(byte[] block)`
    ///
    /// 未识别的字段号在 Java 里是占位对象（`FieldTypeHelper` 的 `getPlaceholder`），本 crate
    /// 无法表达，故返回 `None`（详见 `crate::common::field_type_helper` 的说明）。
    fn get_field_type(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        block: &[u8],
    ) -> Option<MppFieldType>;

    /// 读取条件数据的主入口。
    ///
    /// 对应 Java: `public GenericCriteria process(ProjectFile file, byte[] data, int dataOffset,
    /// int entryOffset, List<GenericCriteriaPrompt> prompts, List<FieldType> fields,
    /// boolean[] criteriaType)`
    ///
    /// - `data_offset`：条件数据在大数据块中的起始位置；
    /// - `entry_offset`：遍历树的起始块偏移，`-1` 表示从 `getCriteriaStartOffset()` 开始；
    /// - `prompts` / `fields` / `criteria_type`：可选出参（Java 传 `null` 表示不收集）；
    /// - 返回树的第一个节点；数据不含任何条件时返回 `None`（Java 返回 `null`）。
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

/// 一次 `process` 调用期间的状态。
///
/// 对应 Java `CriteriaReader` 的实例字段（`m_criteriaData` / `m_criteriaTextStart` /
/// `m_dataOffset` / `m_prompts` / `m_fields` / `m_criteriaType` / `m_criteriaBlockMap`）。
/// Rust 侧把它收进一次调用的局部结构，从而让 trait 保持无状态、可安全复用。
///
/// `R` 是具体的条件读取器类型（`Self`，允许 `?Sized`，从而既能用于具体类型也能用于
/// `&dyn CriteriaReader`）。
struct CriteriaState<'a, R: CriteriaReader + ?Sized> {
    /// 条件读取器（提供各偏移 / 块指针的解析方式）。
    reader: &'a R,

    /// 项目文件。
    ///
    /// 对应 Java: `protected ProjectFile m_file`
    file: &'a Rc<RefCell<ProjectFile>>,

    /// 条件数据。
    ///
    /// 对应 Java: `private byte[] m_criteriaData`
    data: &'a [u8],

    /// 条件数据在大数据块中的起始位置。
    ///
    /// 对应 Java: `private int m_dataOffset`
    data_offset: i32,

    /// 条件文本区起始位置（相对数据体）。
    ///
    /// 对应 Java: `private int m_criteriaTextStart`
    criteria_text_start: i32,

    /// 可选：收集到的提示。
    ///
    /// 对应 Java: `private List<GenericCriteriaPrompt> m_prompts`
    prompts: Option<&'a mut Vec<GenericCriteriaPrompt>>,

    /// 可选：收集到的字段。
    ///
    /// 对应 Java: `private List<FieldType> m_fields`
    fields: Option<&'a mut Vec<FieldKey>>,

    /// 可选：条件类型（`[0]` 任务、`[1]` 资源）。
    ///
    /// 对应 Java: `private boolean[] m_criteriaType`
    criteria_type: Option<&'a mut [bool; 2]>,

    /// 条件块映射。
    ///
    /// 对应 Java: `protected final Map<Integer, byte[]> m_criteriaBlockMap`
    blocks: CriteriaBlockMap,
}

impl<R: CriteriaReader + ?Sized> CriteriaState<'_, R> {
    /// 主入口（对应 Java `process` 的实体部分）。
    fn process(&mut self, entry_offset: i32) -> Option<GenericCriteria> {
        // Java: if (m_criteriaType != null) { m_criteriaType[0] = true; m_criteriaType[1] = true; }
        if let Some(criteria_type) = self.criteria_type.as_mut() {
            criteria_type[0] = true;
            criteria_type[1] = true;
        }

        // Java: m_criteriaBlockMap.clear();
        self.blocks.clear();

        // Java: m_criteriaTextStart = ByteArrayHelper.getShort(m_criteriaData, m_dataOffset + getCriteriaTextStartOffset());
        // Java 越界时抛异常（整次读取失败）；Rust 侧返回 None（该筛选器没有可读条件）
        let text_start_position = self
            .data_offset
            .checked_add(self.reader.get_criteria_text_start_offset())?;
        let criteria_text_start = checked_short(self.data, offset_usize(text_start_position)?)?;
        self.criteria_text_start = criteria_text_start;

        // Java: int criteriaStartOffset = getCriteriaStartOffset();
        //       int criteriaBlockSize = getCriteriaBlockSize();
        let criteria_block_size = self.reader.get_criteria_block_size();

        // Java: if (m_criteriaData.length <= m_criteriaTextStart) { return null; }（bad data）
        if (self.data.len() as i64) <= i64::from(criteria_text_start) {
            return None;
        }

        //
        // Java: while (criteriaStartOffset + criteriaBlockSize <= m_criteriaTextStart) { 复制块到映射 }
        //
        // Java 用 `System.arraycopy` 越界时抛异常；Rust 侧遇到截断即结束收集（该块及其后的块
        // 都不会进入映射，后续按「块不存在」处理）。
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

    /// 处理单个条件块。
    ///
    /// 对应 Java: `private void processBlock(List<GenericCriteria> list, byte[] block)`
    fn process_block(&mut self, list: &mut Vec<GenericCriteria>, block: Option<&[u8]>) {
        // Java: if (block != null) { ... }
        let Some(block) = block else {
            return;
        };

        // Java: if (ByteArrayHelper.getShort(block, 0) > 0x3E6) { addCriteria(list, block); }
        // Java 越界时抛异常；Rust 侧直接返回（块太短，无内容可读）
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

            // Java: 无 default 分支
            _ => {}
        }
    }

    /// 追加一个 `左值 运算符 右值` 条件块。
    ///
    /// 对应 Java: `private void addCriteria(List<GenericCriteria> list, byte[] block)`
    fn add_criteria(&mut self, list: &mut Vec<GenericCriteria>, block: &[u8]) {
        // Java: byte[] leftBlock = getChildBlock(block);
        let left_block = self.reader.get_child_block(block, &self.blocks);

        // Java: byte[] rightBlock1 = getListNextBlock(leftBlock);
        //       byte[] rightBlock2 = getListNextBlock(rightBlock1);
        // Java 在 leftBlock / rightBlock1 为 null 时抛 NPE；Rust 侧按「没有该块」处理
        let right_block1 = left_block
            .as_deref()
            .and_then(|block| self.reader.get_list_next_block(block, &self.blocks));
        let right_block2 = right_block1
            .as_deref()
            .and_then(|block| self.reader.get_list_next_block(block, &self.blocks));

        // Java: TestOperator operator = TestOperator.getInstance(ByteArrayHelper.getShort(block, 0) - 0x3E7);
        //（Java 的反射建表在越界时抛异常；Rust 侧回落到 IS_ANY_VALUE，见 crate::test_operator）
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
        // `leftValue` 为 null 时 Java 抛 NPE，Rust 侧跳过这两个分支
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

    /// 追加一个逻辑运算符块（AND / OR）。
    ///
    /// 对应 Java: `private void addBlock(List<GenericCriteria> list, byte[] block, TestOperator operator)`
    fn add_block(&mut self, list: &mut Vec<GenericCriteria>, block: &[u8], operator: TestOperator) {
        // Java: GenericCriteria result = new GenericCriteria(m_properties);
        //       result.setOperator(operator); list.add(result);
        let mut result = GenericCriteria::new(Rc::downgrade(self.file));
        result.set_operator(Some(operator));
        list.push(result);

        // Java: processBlock(result.getCriteriaList(), getChildBlock(block));
        // 等价于对刚加入的最后一个元素就地填充子条件
        let child = self.reader.get_child_block(block, &self.blocks);
        if let Some(last) = list.last_mut() {
            self.process_block(last.get_criteria_list_mut(), child.as_deref());
        }

        // Java: processBlock(list, getListNextBlock(block));
        let next = self.reader.get_list_next_block(block, &self.blocks);
        self.process_block(list, next.as_deref());
    }

    /// 取条件表达式中的右值。
    ///
    /// 对应 Java: `private Object getValue(FieldType field, byte[] block)`
    fn get_value(&mut self, field: Option<&MppFieldType>, block: &[u8]) -> Option<Operand> {
        // Java 对空块 / null 会抛异常；Rust 侧返回 None
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

            // Java: 无 default 分支 —— result 保持 null
            _ => None,
        }
    }

    /// 取常量取值。
    ///
    /// 对应 Java: `private Object getConstantValue(FieldType type, byte[] block)`
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

            // Java: default: value = null;（含 getDataType() 为 null 的情形）
            _ => None,
        }
    }

    /// 取提示取值。
    ///
    /// 对应 Java: `private GenericCriteriaPrompt getPromptValue(FieldType field, byte[] block)`
    fn get_prompt_value(
        &mut self,
        field: Option<&MppFieldType>,
        block: &[u8],
    ) -> Option<Operand> {
        // Java: int textOffset = getPromptOffset(block);
        let text_offset = self.reader.get_prompt_offset(block);

        // Java: String value = MPPUtility.getUnicodeString(m_criteriaData, m_criteriaTextStart + textOffset);
        //（Java 原样：此处缺少 m_dataOffset，与 getConstantValue 的 STRING 分支不一致）
        let position = self.criteria_text_start.checked_add(text_offset)?;
        let offset = offset_usize(position)?;
        let value = MPPUtility::get_unicode_string(self.data, offset);

        // Java: GenericCriteriaPrompt prompt = new GenericCriteriaPrompt(field.getDataType(), value);
        //（Java 对 `field` 为 null 会抛 NPE；Rust 侧回落到 STRING，不影响提示文本本身）
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

    /// 取字段取值的偏移（非负下标）。
    ///
    /// 对应 Java: `getValueOffset()`（`protected abstract`）。
    fn value_offset(&self) -> usize {
        offset_usize(self.reader.get_value_offset()).unwrap_or(0)
    }

    /// 取工期单位的偏移（非负下标）。
    ///
    /// 对应 Java: `getTimeUnitsOffset()`（`protected abstract`）。
    fn time_units_offset(&self) -> usize {
        offset_usize(self.reader.get_time_units_offset()).unwrap_or(0)
    }

    /// 借用项目属性并按 MPP 规则换算工期。
    ///
    /// 对应 Java: `MPPUtility.getAdjustedDuration(m_properties, duration, timeUnit)`。
    /// 父文件正被可变借用时退化为 `None`（Java 无此情形）。
    fn adjusted_duration(&self, value: i32, units: TimeUnit) -> Option<Duration> {
        let project = self.file.try_borrow().ok()?;
        MPPUtility::get_adjusted_duration(project.get_project_properties(), value, units)
    }
}

/// 把 Java 的 `int` 偏移转成可用的切片下标；负数返回 `None`。
fn offset_usize(value: i32) -> Option<usize> {
    usize::try_from(value).ok()
}

/// 测试用的条件数据构造工具（供本模块与 `FilterReader` 等模块的测试复用）。
///
/// 对应 Java: 无（Java 的测试用例直接内联字节数组）。
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use crate::common::field_type_helper::FieldTypeHelper;
    use crate::task_field::TaskField;

    /// 条件文本区起始位置（块区为 `20..TEXT_START`，步长 80）。
    pub(crate) const TEXT_START: i32 = 420;

    /// 顶层逻辑块偏移。
    pub(crate) const BLOCK_TOP: i32 = 20;
    /// 条件块偏移。
    pub(crate) const BLOCK_CRITERIA: i32 = 100;
    /// 左值（字段）块偏移。
    pub(crate) const BLOCK_LEFT: i32 = 180;
    /// 右值块偏移。
    pub(crate) const BLOCK_RIGHT: i32 = 260;

    /// 在指定偏移写入 4 字节小端整数。
    pub(crate) fn put_int(data: &mut [u8], offset: usize, value: i32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// 在指定偏移写入 2 字节小端短整数。
    pub(crate) fn put_short(data: &mut [u8], offset: usize, value: i32) {
        data[offset..offset + 2].copy_from_slice(&(value as u16).to_le_bytes());
    }

    /// 内置任务字段的字段号（MPP9/12 的字段号体系）。
    pub(crate) fn field_id(field: TaskField) -> i32 {
        FieldTypeHelper::get_field_id(Some(&MppFieldType::Builtin(FieldKey::Task(field))))
    }

    /// 构造 MPP9 布局的条件数据：
    ///
    /// ```text
    /// [16..18)             条件文本区起始位置（TEXT_START）
    /// 20   +-----------------------------+
    ///      | 逻辑块（block_tag）          | child -> 100
    /// 100  +-----------------------------+
    ///      | 条件块（运算符 = EQUALS）     | child -> 180, next -> 0
    /// 180  +-----------------------------+
    ///      | 左值块（字段号 @40）          | next -> 260
    /// 260  +-----------------------------+
    ///      | 右值块（常量 @0x01，文本 @32）|
    /// 420  +-----------------------------+
    ///      | 文本区（UTF-16LE + NUL）      |
    /// ```
    ///
    /// 返回 `(数据, 入口块偏移)`。
    pub(crate) fn criteria_data(block_tag: u8, field: TaskField, text: &str) -> (Vec<u8>, i32) {
        let mut data = vec![0u8; (TEXT_START as usize) + 32];

        // 条件文本区起始位置
        put_short(&mut data, 16, TEXT_START);

        // 顶层逻辑块：0x19/0x1B = AND，0x1A/0x1C = OR
        data[BLOCK_TOP as usize] = block_tag;
        put_short(&mut data, BLOCK_TOP as usize + 74, BLOCK_CRITERIA);

        // 条件块：短整数 > 0x3E6 ⇒ EQUALS（0x3E7 + 6）
        put_short(&mut data, BLOCK_CRITERIA as usize, 0x3E7 + 6);
        put_short(&mut data, BLOCK_CRITERIA as usize + 74, BLOCK_LEFT);

        // 左值块：字段号在 40，next 指向右值块
        put_int(&mut data, BLOCK_LEFT as usize + 40, field_id(field));
        put_short(&mut data, BLOCK_LEFT as usize + 76, BLOCK_RIGHT);

        // 右值块：0x01 = 常量，文本偏移在 68
        data[BLOCK_RIGHT as usize] = 0x01;
        put_short(&mut data, BLOCK_RIGHT as usize + 68, 0);

        // 文本区：UTF-16LE + NUL
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

    /// AND / OR 组合块被还原成带子条件的逻辑条件。
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
                .expect("应解析出条件");

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
            // Java: rightBlock2 为 null ⇒ 右值 1 为 null
            assert_eq!(child.get_value(1), None);

            // 出参：提示为空、字段为左值、条件类型为任务
            assert!(prompts.is_empty());
            assert_eq!(fields, vec![FieldKey::Task(TaskField::Name)]);
            assert_eq!(criteria_type, [true, false]);
        }
    }

    /// 直接以条件块为入口（`entryOffset` 指向条件块）。
    #[test]
    fn criteria_block_as_entry() {
        let (data, _) = criteria_data(0x19, TaskField::Name, "AB");
        let file = ProjectFile::new();

        let criteria = FilterCriteriaReader9::new()
            .process(&file, &data, 0, BLOCK_CRITERIA, None, None, None)
            .expect("应解析出条件");

        assert_eq!(criteria.get_operator(), Some(TestOperator::Equals));
        assert_eq!(criteria.get_left_value(), Some(FieldKey::Task(TaskField::Name)));
    }

    /// `entryOffset` 为 -1 时从 `getCriteriaStartOffset()` 开始。
    #[test]
    fn entry_offset_minus_one_uses_start_offset() {
        let (data, _) = criteria_data(0x1B, TaskField::Name, "AB");
        let file = ProjectFile::new();

        let criteria = FilterCriteriaReader9::new()
            .process(&file, &data, 0, -1, None, None, None)
            .expect("应解析出条件");

        assert_eq!(criteria.get_operator(), Some(TestOperator::And));
    }

    /// 数值型右值（含 CURRENCY 的百分之一换算）。
    #[test]
    fn numeric_constant_value() {
        let (mut data, _) = criteria_data(0x19, TaskField::Name, "AB");

        // 右值块改为“常量 + 数值字段 + 双精度 250”
        put_int(&mut data, BLOCK_LEFT as usize + 40, field_id(TaskField::Cost));
        data[BLOCK_RIGHT as usize] = 0x01;
        put_int(&mut data, BLOCK_RIGHT as usize + 32, 0);
        // 250.0 的双精度位模式
        data[BLOCK_RIGHT as usize + 32..BLOCK_RIGHT as usize + 40]
            .copy_from_slice(&250.0f64.to_le_bytes());

        let file = ProjectFile::new();
        let criteria = FilterCriteriaReader9::new()
            .process(&file, &data, 0, BLOCK_CRITERIA, None, None, None)
            .expect("应解析出条件");

        assert_eq!(criteria.get_left_value(), Some(FieldKey::Task(TaskField::Cost)));
        assert_eq!(
            criteria.get_value(0),
            Some(&Operand::Value(FieldValue::Double(2.5)))
        );
    }

    /// 右值为字段引用（0x07）时按符号值收集。
    #[test]
    fn field_reference_value() {
        let (mut data, _) = criteria_data(0x19, TaskField::Name, "AB");

        // 右值块改为字段引用：0x07，字段号在 40
        data[BLOCK_RIGHT as usize] = 0x07;
        put_int(&mut data, BLOCK_RIGHT as usize + 40, field_id(TaskField::Cost));

        let file = ProjectFile::new();
        let criteria = FilterCriteriaReader9::new()
            .process(&file, &data, 0, BLOCK_CRITERIA, None, None, None)
            .expect("应解析出条件");

        assert_eq!(
            criteria.get_value(0),
            Some(&Operand::Field(FieldKey::Task(TaskField::Cost)))
        );
    }

    /// 右值为提示（0x00）时同时写入出参列表。
    #[test]
    fn prompt_value() {
        let (mut data, _) = criteria_data(0x19, TaskField::Name, "AB");

        // 右值块改为提示：0x00，提示文本偏移在 72
        data[BLOCK_RIGHT as usize] = 0x00;
        put_short(&mut data, BLOCK_RIGHT as usize + 72, 0);

        let file = ProjectFile::new();
        let mut prompts = Vec::new();
        let criteria = FilterCriteriaReader9::new()
            .process(&file, &data, 0, BLOCK_CRITERIA, Some(&mut prompts), None, None)
            .expect("应解析出条件");

        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].get_prompt(), Some("AB"));
        assert_eq!(prompts[0].get_type(), DataType::String);
        assert_eq!(
            criteria.get_value(0),
            Some(&Operand::Prompt(prompts[0].clone()))
        );
    }

    /// 数据缺失 / 截断 / 块指针指向不存在的块时不 panic。
    #[test]
    fn malformed_data_is_tolerated() {
        let file = ProjectFile::new();
        let reader = FilterCriteriaReader9::new();

        // 空数据
        assert!(reader.process(&file, &[], 0, -1, None, None, None).is_none());

        // 数据比文本区起始位置还短（Java: bad data → null）
        let mut data = vec![0u8; 20];
        put_short(&mut data, 16, 1000);
        assert!(reader
            .process(&file, &data, 0, -1, None, None, None)
            .is_none());

        // 条件文本区起始位置读不出来（不足 18 字节）
        assert!(reader
            .process(&file, &[0u8; 10], 0, -1, None, None, None)
            .is_none());

        // 文本区起始位置为 0（小于起始块偏移）⇒ 无块可读 ⇒ 无条件
        let data = vec![0u8; 40];
        assert!(reader
            .process(&file, &data, 0, -1, None, None, None)
            .is_none());

        // 块指针指向不存在的偏移
        let (data, _) = criteria_data(0x19, TaskField::Name, "AB");
        assert!(reader
            .process(&file, &data, 0, 9999, None, None, None)
            .is_none());

        // 块内容被截断（文本区起始位置合法，但块区不足一整块）
        let mut data = vec![0u8; 100];
        put_short(&mut data, 16, 90);
        assert!(reader
            .process(&file, &data, 0, -1, None, None, None)
            .is_none());
    }

    /// 未识别的字段号：仍生成条件，但不更新出参（Java 抛 NPE）。
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
            .expect("应解析出条件");

        assert_eq!(criteria.get_left_value(), None);
        assert!(fields.is_empty());
        // Java 在这里抛 NPE；Rust 侧保留 process 开头的初始化值
        assert_eq!(criteria_type, [true, true]);
    }
}
