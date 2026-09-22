//! Corresponds to Java: `org/mpxj/mpp/FilterReader.java`
//!
//! Reads filter definitions from an MPP file: `FixedData` gives each filter's ID and name, and `Var2Data`
//! gives that filter's criteria data; the "show related summary rows" flag and the prompt list also come from the criteria data.
//!
//! # Reproducing inheritance (trait default methods)
//!
//! Java's `abstract class FilterReader` is extended by
//! [`FilterReader9`](crate::mpp::filter_reader9::FilterReader9) /
//! [`FilterReader12`](crate::mpp::filter_reader12::FilterReader12) /
//! [`FilterReader14`](crate::mpp::filter_reader14::FilterReader14). Rust has no inheritance,
//! so:
//!
//! - This class becomes the [`FilterReader`] **trait**: Java's 2 `protected abstract` methods
//! (`getVarDataType` / `getCriteriaReader`) correspond to the trait's required methods;
//! - Java's public `process` method is a **concrete method**, changed on the Rust side to a **default method** of the trait;
//! - Each subclass holds a criteria reader instance (Java's `private final CriteriaReader m_criteraReader`),
//! exposed by `get_criteria_reader` as `&dyn CriteriaReader`.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `protected abstract Integer getVarDataType()` | `fn get_var_data_type(&self) -> i32` | On the Java side this `Integer` is always non-null (subclasses return the constants 1 / 6), consistent with the `CalendarFactory` trade-off |
//! | `ProjectFile file` | `&Rc<RefCell<ProjectFile>>` | The project file is shared in this crate as `Rc<RefCell<..>>` |
//! | `file.getFilters().addFilter(filter)` | `file.try_borrow_mut()` + `FilterContainer::add_filter` | The container stores entries as `Rc<RefCell<Filter>>`; the filter is skipped when the file is already borrowed (this case does not occur in Java), without panicking |
//! | `List<GenericCriteriaPrompt> prompts` | A local `Vec<GenericCriteriaPrompt>` (still written into the filter via Java's `setPrompts`) | Same type, no change needed |
//! | `boolean[] criteriaType` | A local `[bool; 2]` | The same array is reused each iteration, consistent with Java |
//!
//! Skipped Java members: none.
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

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

/// Filter definition reader.
///
/// Corresponds to Java: `abstract class FilterReader`
pub trait FilterReader {
/// Get the `Var2Data` data type key used by this file version to read filter criteria data.
    ///
    /// Corresponds to Java: `protected abstract Integer getVarDataType()`
    fn get_var_data_type(&self) -> i32;

/// Get the criteria reader used by this file version.
    ///
    /// Corresponds to Java: `protected abstract CriteriaReader getCriteriaReader()`
    fn get_criteria_reader(&self) -> &dyn CriteriaReader;

/// Read all filter definitions and add them to the project file.
    ///
    /// Corresponds to Java: `public void process(ProjectFile file, FixedData fixedData, Var2Data varData)`
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

// Java comment: the logic that determines whether a filter belongs to tasks or resources is only valid when the filter has criteria
            // Java: filter.setIsTaskFilter(criteriaType[0]); filter.setIsResourceFilter(criteriaType[1]);
            filter.set_is_task_filter(criteria_type[0]);
            filter.set_is_resource_filter(criteria_type[1]);

            // Java: filter.setPrompts(prompts);
            filter.set_prompts(prompts);

            // Java: file.getFilters().addFilter(filter);
// The Rust-side container stores `Rc<RefCell<Filter>>`; skips when the file is already borrowed (without panicking)
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

/// Filter fixed data: `[0..4)` = ID, `[4..)` = name (UTF-16LE + NUL).
    fn filter_fixed_record(id: i32, name: &str) -> Vec<u8> {
        let mut record = id.to_le_bytes().to_vec();
        for unit in name.encode_utf16() {
            record.extend_from_slice(&unit.to_le_bytes());
        }
        record.extend_from_slice(&[0, 0]);
        record
    }

/// Filter criteria data: `[4]` = whether to show related summary rows, with the criteria block starting at offset 0.
    fn filter_var_data(show_related_summary_rows: bool, criteria: &[u8]) -> Vec<u8> {
        let mut data = criteria.to_vec();
        if data.len() <= 4 {
            data.resize(6, 0);
        }
        data[4] = u8::from(show_related_summary_rows);
        data
    }

/// The `&` in the name is stripped, and the ID / flag / criteria / prompts are all written into the project file with Java semantics.
    #[test]
    fn process_reads_filter_definitions() {
        let (criteria, _) = criteria_data(0x1B, TaskField::Name, "AB");

        let fixed = fixed_data(&[
            filter_fixed_record(1, "&My Filter"),
// Fixed data is fewer than 4 bytes ⇒ skip (Java: continue)
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

        let criteria = filter.get_criteria().expect("criteria should be parsed");
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

/// Does not panic when the variable data is missing / the criteria data is empty, and the filter is still registered.
    #[test]
    fn process_tolerates_missing_var_data() {
        let fixed = fixed_data(&[filter_fixed_record(1, "Empty")]);

// There is no entry with ID = 1 in the variable data
        let (_, var_data) = var_data_with_meta(&[(2, 1, vec![0u8; 8])]);
        let file = ProjectFile::new();
        FilterReader9::new().process(&file, &fixed, &var_data);
        assert!(file.borrow().get_filters().get_task_filters().is_empty());

// The criteria data is all zeros: the criteria block is empty ⇒ no criteria, but the criteria type keeps Java's initialized value true/true
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

/// Empty fixed data produces no filters and does not panic.
    #[test]
    fn process_with_no_filters() {
        let fixed = crate::mpp::custom_field_value_reader::test_support::empty_fixed_data();
        let (_, var_data) = var_data_with_meta(&[]);

        let file = ProjectFile::new();
        FilterReader9::new().process(&file, &fixed, &var_data);
        assert!(file.borrow().get_filters().get_task_filters().is_empty());
    }
}
