//! 对应 Java: `org/mpxj/mpp/FilterReader.java`
//!
//! 从 MPP 文件里读出筛选器定义：`FixedData` 给出每个筛选器的 ID 与名称，`Var2Data`
//! 给出该筛选器的条件数据，「是否显示相关摘要行」标志与提示列表也取自条件数据。
//!
//! # 继承的还原（trait 默认方法）
//!
//! Java 的 `abstract class FilterReader` 由
//! [`FilterReader9`](crate::mpp::filter_reader9::FilterReader9) /
//! [`FilterReader12`](crate::mpp::filter_reader12::FilterReader12) /
//! [`FilterReader14`](crate::mpp::filter_reader14::FilterReader14) 继承。Rust 没有继承，
//! 故：
//!
//! - 本类成为 [`FilterReader`] **trait**：Java 的 2 个 `protected abstract` 方法
//!   （`getVarDataType` / `getCriteriaReader`）对应 trait 的必需方法；
//! - Java 的公开方法 `process` 是**具体方法**，Rust 侧改为 trait 的**默认方法**；
//! - 子类各自持有一个条件读取器实例（Java 的 `private final CriteriaReader m_criteraReader`），
//!   并由 `get_criteria_reader` 以 `&dyn CriteriaReader` 暴露。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `protected abstract Integer getVarDataType()` | `fn get_var_data_type(&self) -> i32` | Java 侧该 `Integer` 恒非空（子类返回常量 1 / 6），与 `CalendarFactory` 的取舍一致 |
//! | `ProjectFile file` | `&Rc<RefCell<ProjectFile>>` | 项目文件在本 crate 中以 `Rc<RefCell<..>>` 共享 |
//! | `file.getFilters().addFilter(filter)` | `file.try_borrow_mut()` + `FilterContainer::add_filter` | 容器按 `Rc<RefCell<Filter>>` 存放；文件正被借用时跳过该筛选器（Java 无此情形），不 panic |
//! | `List<GenericCriteriaPrompt> prompts` | 局部 `Vec<GenericCriteriaPrompt>`（仍按 Java 的 `setPrompts` 写入筛选器） | 类型一致，无需改动 |
//! | `boolean[] criteriaType` | 局部 `[bool; 2]` | 每次循环复用同一个数组，与 Java 一致 |
//!
//! 跳过的 Java 成员：无。
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::filter::Filter;
use crate::generic_criteria_prompt::GenericCriteriaPrompt;
use crate::mpp::criteria_reader::CriteriaReader;
use crate::mpp::fixed_data::FixedData;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::var2_data::Var2Data;
use crate::project_file::ProjectFile;

/// 筛选器定义读取器。
///
/// 对应 Java: `abstract class FilterReader`
pub trait FilterReader {
    /// 取本版本文件用于读取筛选器条件数据的 `Var2Data` 数据类型键。
    ///
    /// 对应 Java: `protected abstract Integer getVarDataType()`
    fn get_var_data_type(&self) -> i32;

    /// 取本版本文件使用的条件读取器。
    ///
    /// 对应 Java: `protected abstract CriteriaReader getCriteriaReader()`
    fn get_criteria_reader(&self) -> &dyn CriteriaReader;

    /// 读取全部筛选器定义并加入项目文件。
    ///
    /// 对应 Java: `public void process(ProjectFile file, FixedData fixedData, Var2Data varData)`
    fn process(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        fixed_data: &FixedData,
        var_data: &Var2Data,
    ) {
        // Java: int filterCount = fixedData.getItemCount();
        let filter_count = fixed_data.get_item_count();

        // Java: boolean[] criteriaType = new boolean[2];
        let mut criteria_type = [false; 2];

        // Java: CriteriaReader criteriaReader = getCriteriaReader();
        let criteria_reader = self.get_criteria_reader();

        // Java: for (int filterLoop = 0; filterLoop < filterCount; filterLoop++)
        for filter_loop in 0..filter_count {
            // Java: byte[] filterFixedData = fixedData.getByteArrayValue(filterLoop);
            //       if (filterFixedData == null || filterFixedData.length < 4) { continue; }
            let Some(filter_fixed_data) = fixed_data.get_byte_array_value(filter_loop) else {
                continue;
            };
            if filter_fixed_data.len() < 4 {
                continue;
            }

            // Java: Filter filter = new Filter();
            //       filter.setID(Integer.valueOf(ByteArrayHelper.getInt(filterFixedData, 0)));
            //       filter.setName(MPPUtility.removeAmpersands(MPPUtility.getUnicodeString(filterFixedData, 4)));
            let id = ByteArrayHelper::get_int(filter_fixed_data, 0);
            let mut filter = Filter::new();
            filter.set_id(Some(id));
            let name = MPPUtility::get_unicode_string(filter_fixed_data, 4);
            filter.set_name(MPPUtility::remove_ampersands(Some(&name)));

            // Java: byte[] filterVarData = varData.getByteArray(filter.getID(), getVarDataType());
            //       if (filterVarData == null) { continue; }
            let Some(filter_var_data) =
                var_data.get_byte_array(Some(id), Some(self.get_var_data_type()))
            else {
                continue;
            };

            // Java: List<GenericCriteriaPrompt> prompts = new ArrayList<>();
            let mut prompts: Vec<GenericCriteriaPrompt> = Vec::new();

            // Java: filter.setShowRelatedSummaryRows(MPPUtility.getByte(filterVarData, 4) != 0);
            filter.set_show_related_summary_rows(MPPUtility::get_byte(&filter_var_data, 4) != 0);

            // Java: filter.setCriteria(criteriaReader.process(file, filterVarData, 0, -1, prompts, null, criteriaType));
            filter.set_criteria(criteria_reader.process(
                file,
                &filter_var_data,
                0,
                -1,
                Some(&mut prompts),
                None,
                Some(&mut criteria_type),
            ));

            // Java 注释：判断筛选器属于任务还是资源的逻辑，只有在筛选器有条件时才有效
            // Java: filter.setIsTaskFilter(criteriaType[0]); filter.setIsResourceFilter(criteriaType[1]);
            filter.set_is_task_filter(criteria_type[0]);
            filter.set_is_resource_filter(criteria_type[1]);

            // Java: filter.setPrompts(prompts);
            filter.set_prompts(prompts);

            // Java: file.getFilters().addFilter(filter);
            // Rust 侧容器存 `Rc<RefCell<Filter>>`；文件正被借用时跳过（不 panic）
            if let Ok(mut project) = file.try_borrow_mut() {
                project
                    .get_filters_mut()
                    .add_filter(Rc::new(RefCell::new(filter)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_key::FieldKey;
    use crate::field_value::FieldValue;
    use crate::mpp::criteria_reader::test_support::criteria_data;
    use crate::mpp::custom_field_value_reader::test_support::{fixed_data, var_data_with_meta};
    use crate::mpp::filter_reader9::FilterReader9;
    use crate::task_field::TaskField;
    use crate::test_operator::{Operand, TestOperator};

    /// 筛选器定长数据：`[0..4)` = ID，`[4..)` = 名称（UTF-16LE + NUL）。
    fn filter_fixed_record(id: i32, name: &str) -> Vec<u8> {
        let mut record = id.to_le_bytes().to_vec();
        for unit in name.encode_utf16() {
            record.extend_from_slice(&unit.to_le_bytes());
        }
        record.extend_from_slice(&[0, 0]);
        record
    }

    /// 筛选器条件数据：`[4]` = 是否显示相关摘要行，条件块自偏移 0 起。
    fn filter_var_data(show_related_summary_rows: bool, criteria: &[u8]) -> Vec<u8> {
        let mut data = criteria.to_vec();
        if data.len() <= 4 {
            data.resize(6, 0);
        }
        data[4] = u8::from(show_related_summary_rows);
        data
    }

    /// 名称中的 `&` 被剥掉，ID / 标志 / 条件 / 提示均按 Java 语义写入项目文件。
    #[test]
    fn process_reads_filter_definitions() {
        let (criteria, _) = criteria_data(0x1B, TaskField::Name, "AB");

        let fixed = fixed_data(&[
            filter_fixed_record(1, "&My Filter"),
            // 定长数据少于 4 字节 ⇒ 跳过（Java: continue）
            vec![0u8; 3],
        ]);
        let (_, var_data) = var_data_with_meta(&[(1, 1, filter_var_data(true, &criteria))]);

        let file = ProjectFile::new();
        FilterReader9::new().process(&file, &fixed, &var_data);

        let project = file.borrow();
        let filters = project.get_filters();
        assert_eq!(filters.get_task_filters().len(), 1);
        assert!(filters.get_resource_filters().is_empty());

        let filter = filters.get_task_filters()[0].borrow();
        assert_eq!(filter.get_id(), Some(1));
        assert_eq!(filter.get_name(), Some("My Filter"));
        assert!(filter.get_show_related_summary_rows());
        assert!(filter.is_task_filter());
        assert!(!filter.is_resource_filter());
        assert!(filter.get_prompts().is_empty());

        let criteria = filter.get_criteria().expect("应解析出条件");
        assert_eq!(criteria.get_operator(), Some(TestOperator::And));
        let children = criteria.get_criteria_list();
        assert_eq!(children.len(), 1);
        assert_eq!(
            children[0].get_left_value(),
            Some(FieldKey::Task(TaskField::Name))
        );
        assert_eq!(
            children[0].get_value(0),
            Some(&Operand::Value(FieldValue::String("AB".to_owned())))
        );
    }

    /// 变长数据缺失 / 条件数据为空时不 panic，且筛选器仍被登记。
    #[test]
    fn process_tolerates_missing_var_data() {
        let fixed = fixed_data(&[filter_fixed_record(1, "Empty")]);

        // 变长数据里没有 ID = 1 的条目
        let (_, var_data) = var_data_with_meta(&[(2, 1, vec![0u8; 8])]);
        let file = ProjectFile::new();
        FilterReader9::new().process(&file, &fixed, &var_data);
        assert!(file.borrow().get_filters().get_task_filters().is_empty());

        // 条件数据全零：条件块为空 ⇒ 无条件，但条件类型保持 Java 的初始化值 true/true
        let (_, var_data) = var_data_with_meta(&[(1, 1, vec![0u8; 32])]);
        let file = ProjectFile::new();
        FilterReader9::new().process(&file, &fixed, &var_data);

        let project = file.borrow();
        let filters = project.get_filters();
        assert_eq!(filters.get_task_filters().len(), 1);
        assert_eq!(filters.get_resource_filters().len(), 1);

        let filter = filters.get_task_filters()[0].borrow();
        assert_eq!(filter.get_name(), Some("Empty"));
        assert!(!filter.get_show_related_summary_rows());
        assert!(filter.get_criteria().is_none());
    }

    /// 空的定长数据不会有任何筛选器，也不 panic。
    #[test]
    fn process_with_no_filters() {
        let fixed = crate::mpp::custom_field_value_reader::test_support::empty_fixed_data();
        let (_, var_data) = var_data_with_meta(&[]);

        let file = ProjectFile::new();
        FilterReader9::new().process(&file, &fixed, &var_data);
        assert!(file.borrow().get_filters().get_task_filters().is_empty());
    }
}
