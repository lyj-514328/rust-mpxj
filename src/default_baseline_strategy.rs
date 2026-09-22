//! Corresponds to Java: `org/mpxj/DefaultBaselineStrategy.java`
//!
//! # Differences from Java
//!
//! - **Inheritance**: Java's `DefaultBaselineStrategy extends AbstractBaselineStrategy`,
//! while Java's `AbstractBaselineStrategy` is not a separate module. Rust has no inheritance,
//! so the base class implementation (`clearBaseline` / `populateBaseline` / `getKeyForTask` /
//! `getBaselineFields` / `getSourceFields`) is merged into this type as is.
//! - **`TaskFieldLists`**: Java takes the baseline field arrays from `org.mpxj.common.TaskFieldLists`;
//! Rust references the same already-ported constants ([`crate::common::task_field_lists::TaskFieldLists`]),
//! rather than defining another copy in this file, to avoid the two sides drifting apart.
//! - **Singleton `INSTANCE`**: Java's `public static final DefaultBaselineStrategy INSTANCE`
//! is expressed on the Rust side as the same-named constant [`DefaultBaselineStrategy::INSTANCE`]. This type has no mutable fields,
//! so it can be constructed directly as a `const`, and no `OnceLock` / `instance()` function is needed
//! (`crate::project_config::ProjectConfig` also uses it as `DefaultBaselineStrategy::INSTANCE`).
//!
//! # `ProjectFile` / `Task` interfaces used
//!
//! ```text
//! ProjectFile::new() -> ProjectFile
//! ProjectFile::get_tasks(&self) -> Vec<Rc<RefCell<Task>>>        // Java: TaskContainer getTasks()
//! ProjectFile::set_baseline_task_map(&mut self, index: i32, map: HashMap<i32, Rc<RefCell<Task>>>)
//!                                                                // Java: void setBaselineTaskMap(int, Map<Task, Task>)
//! Task::get_guid(&self) -> Option<String>                        // Java: String getGUID()
//! Task::get_unique_id(&self) -> Option<i32>                      // Java: Integer getUniqueID()
//! Task::get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> // Java: Object getCachedValue(FieldType)
//! Task::set(&mut self, field: FieldKey, value: Option<FieldValue>)      // Java: void set(FieldType, Object)
//! ```
//!
//! Java's `Map<Task, Task>` (baselineTaskMap) uses the task's unique ID as the key on the Rust side,
//! which is semantically equivalent (a unique ID is unique within a single schedule).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::baseline_strategy::BaselineStrategy;
use crate::common::task_field_lists::TaskFieldLists;
use crate::field_key::FieldKey;
use crate::project_file::ProjectFile;
use crate::task::Task;
use crate::task_field::TaskField;

/// Populate the baseline fields of one project with the values of another project.
///
/// Corresponds to Java: `public final class DefaultBaselineStrategy extends AbstractBaselineStrategy`
///
/// This class has no mutable state, so it is implemented as a fieldless (unit) struct; Java's private constructor needs no explicit representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultBaselineStrategy;

impl DefaultBaselineStrategy {
/// Java's singleton constant `INSTANCE`.
    ///
    /// Corresponds to Java: `public static final DefaultBaselineStrategy INSTANCE`
    pub const INSTANCE: DefaultBaselineStrategy = DefaultBaselineStrategy;

/// Generate the key used to match tasks between the current schedule and the baseline schedule.
    ///
    /// Corresponds to Java: `protected Object getKeyForTask(Task task)`
    ///
/// Java returns `task.getGUID()`; tasks returning `null` do not participate in matching.
    pub fn get_key_for_task(&self, task: &Task) -> Option<String> {
        task.get_guid().map(|guid| guid.to_string())
    }

/// Determine the set of baseline fields to populate.
    ///
    /// Corresponds to Java: `protected TaskField[] getBaselineFields(int index)`
    ///
/// Returns the main baseline fields when `index` is `0`, otherwise the baseline 1-10 fields.
/// Java throws `ArrayIndexOutOfBoundsException` for an out-of-bounds index; Rust does not panic and returns an empty set.
    pub fn get_baseline_fields(&self, index: i32) -> Vec<TaskField> {
        if index == 0 {
            return BASELINE0_FIELDS.to_vec();
        }

// Java: after `--index;`, take the baseline 1-10 fields using a 0-based index
        let Some(offset) = usize::try_from(index - 1).ok() else {
            return Vec::new();
        };

        let fields = [
            TaskFieldLists::BASELINE_COSTS.get(offset),
            TaskFieldLists::BASELINE_DURATIONS.get(offset),
            TaskFieldLists::BASELINE_FINISHES.get(offset),
            TaskFieldLists::BASELINE_FIXED_COST_ACCRUALS.get(offset),
            TaskFieldLists::BASELINE_FIXED_COSTS.get(offset),
            TaskFieldLists::BASELINE_STARTS.get(offset),
            TaskFieldLists::BASELINE_WORKS.get(offset),
        ];

        fields.into_iter().flatten().copied().collect()
    }

/// Source fields whose values are taken from the baseline schedule.
    ///
    /// Corresponds to Java: `protected TaskField[] getSourceFields()`
    pub fn get_source_fields(&self) -> Vec<TaskField> {
        SOURCE_FIELDS.to_vec()
    }

/// Populate another task's baseline fields with the values of one task (or `None`).
    ///
    /// Corresponds to Java: `private void populateBaseline(Task task, Task baseline, TaskField[] baselineFields)`
    fn populate_baseline_for_task(
        &self,
        task: &mut Task,
        baseline: Option<&Task>,
        baseline_fields: &[TaskField],
    ) {
        let source_fields = self.get_source_fields();

        for (index, source_field) in source_fields.iter().enumerate() {
            let Some(baseline_field) = baseline_fields.get(index) else {
                break;
            };

            // Java: baseline == null ? null : baseline.getCachedValue(sourceFields[i])
            let value = baseline
                .and_then(|baseline| baseline.get_cached_value(FieldKey::Task(*source_field)))
                .cloned();

            task.set(FieldKey::Task(*baseline_field), value);
        }
    }
}

impl BaselineStrategy for DefaultBaselineStrategy {
/// Clear the baseline with the given number in the given project.
    ///
    /// Corresponds to Java: `@Override public void clearBaseline(ProjectFile project, int index)`
    fn clear_baseline(&self, project: &mut ProjectFile, index: i32) {
        let baseline_fields = self.get_baseline_fields(index);

        for task in project.get_tasks() {
            self.populate_baseline_for_task(&mut task.borrow_mut(), None, &baseline_fields);
        }
    }

/// Populate the baseline with the given number in `project` with the values of the `baseline` project.
    ///
    /// Corresponds to Java: `@Override public void populateBaseline(ProjectFile project, ProjectFile baseline, int index)`
    fn populate_baseline(&self, project: &mut ProjectFile, baseline: &ProjectFile, index: i32) {
        let baseline_fields = self.get_baseline_fields(index);

// Build the "baseline task key -> baseline task" mapping.
// Java uses `Collectors.toMap(..., (u, v) -> null)`: on a key collision the key's value is null,
// and later filtering by `map.get(key) != null` means tasks with duplicate keys are not matched.
        let mut baseline_by_key: HashMap<String, Option<Rc<RefCell<Task>>>> = HashMap::new();
        for task in baseline.get_tasks() {
            let Some(key) = self.get_key_for_task(&task.borrow()) else {
                continue;
            };

            baseline_by_key
                .entry(key)
                .and_modify(|value| *value = None)
                .or_insert_with(|| Some(task.clone()));
        }

// Build the "current project task unique ID -> baseline task" mapping (corresponds to Java's Map<Task, Task>).
        let mut baseline_task_map: HashMap<i32, Rc<RefCell<Task>>> = HashMap::new();
        for task in project.get_tasks() {
            let key = self.get_key_for_task(&task.borrow());
            let Some(Some(baseline_task)) = key.and_then(|key| baseline_by_key.get(&key).cloned())
            else {
                continue;
            };

            let Some(unique_id) = task.borrow().get_unique_id() else {
                continue;
            };

            baseline_task_map.insert(unique_id, baseline_task);
        }

// Cache this mapping for use by Task.getBaselineTask().
        project.set_baseline_task_map(index, baseline_task_map.clone());

// Iterate over **all** tasks (not the mapping itself) to ensure that tasks without a baseline have their
// baseline fields set to null.
        for task in project.get_tasks() {
            let unique_id = task.borrow().get_unique_id();
            let baseline_task = unique_id.and_then(|unique_id| baseline_task_map.get(&unique_id));
            let baseline_borrow = baseline_task.map(|task| task.borrow());

            self.populate_baseline_for_task(
                &mut task.borrow_mut(),
                baseline_borrow.as_deref(),
                &baseline_fields,
            );
        }
    }
}

/// Source fields whose values are taken from the baseline schedule.
///
/// Corresponds to Java: `private static final TaskField[] SOURCE_FIELDS`
const SOURCE_FIELDS: [TaskField; 7] = [
    TaskField::Cost,
    TaskField::Duration,
    TaskField::Finish,
    TaskField::FixedCostAccrual,
    TaskField::FixedCost,
    TaskField::Start,
    TaskField::Work,
];

/// Main baseline (index 0) fields.
///
/// Corresponds to Java: `private static final TaskField[] BASELINE0_FIELDS`
const BASELINE0_FIELDS: [TaskField; 7] = [
    TaskField::BaselineCost,
    TaskField::BaselineDuration,
    TaskField::BaselineFinish,
    TaskField::BaselineFixedCostAccrual,
    TaskField::BaselineFixedCost,
    TaskField::BaselineStart,
    TaskField::BaselineWork,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_constant_is_usable_as_a_value() {
// Java's `INSTANCE` is a stateless singleton; on the Rust side it is a directly copyable constant value.
        let strategy = DefaultBaselineStrategy::INSTANCE;
        assert_eq!(strategy, DefaultBaselineStrategy::INSTANCE);
        assert_eq!(strategy.get_source_fields().len(), 7);
    }

    #[test]
    fn source_fields_match_java() {
        assert_eq!(
            DefaultBaselineStrategy::INSTANCE.get_source_fields(),
            vec![
                TaskField::Cost,
                TaskField::Duration,
                TaskField::Finish,
                TaskField::FixedCostAccrual,
                TaskField::FixedCost,
                TaskField::Start,
                TaskField::Work,
            ]
        );
    }

    #[test]
    fn baseline_zero_fields_match_java() {
        assert_eq!(
            DefaultBaselineStrategy::INSTANCE.get_baseline_fields(0),
            vec![
                TaskField::BaselineCost,
                TaskField::BaselineDuration,
                TaskField::BaselineFinish,
                TaskField::BaselineFixedCostAccrual,
                TaskField::BaselineFixedCost,
                TaskField::BaselineStart,
                TaskField::BaselineWork,
            ]
        );
    }

    #[test]
    fn numbered_baseline_fields_match_java() {
        let strategy = DefaultBaselineStrategy::INSTANCE;

        assert_eq!(
            strategy.get_baseline_fields(1),
            vec![
                TaskField::Baseline1Cost,
                TaskField::Baseline1Duration,
                TaskField::Baseline1Finish,
                TaskField::Baseline1FixedCostAccrual,
                TaskField::Baseline1FixedCost,
                TaskField::Baseline1Start,
                TaskField::Baseline1Work,
            ]
        );

        assert_eq!(
            strategy.get_baseline_fields(10),
            vec![
                TaskField::Baseline10Cost,
                TaskField::Baseline10Duration,
                TaskField::Baseline10Finish,
                TaskField::Baseline10FixedCostAccrual,
                TaskField::Baseline10FixedCost,
                TaskField::Baseline10Start,
                TaskField::Baseline10Work,
            ]
        );
    }

    #[test]
    fn out_of_range_index_yields_no_fields() {
        let strategy = DefaultBaselineStrategy::INSTANCE;
        assert!(strategy.get_baseline_fields(-1).is_empty());
        assert!(strategy.get_baseline_fields(11).is_empty());
    }

/// Deduplication regression: the baseline 1-10 fields must be **from the same source** as the 7 constants in [`TaskFieldLists`].
    ///
/// This file once kept its own `BASELINE_*` constant arrays that evolved separately from the values in `TaskFieldLists`;
/// this test locks down "the two sides have identical values".
    #[test]
    fn numbered_baseline_fields_track_task_field_lists() {
        let strategy = DefaultBaselineStrategy::INSTANCE;

        for index in 1..=10 {
            let offset = (index - 1) as usize;
            assert_eq!(
                strategy.get_baseline_fields(index),
                vec![
                    TaskFieldLists::BASELINE_COSTS[offset],
                    TaskFieldLists::BASELINE_DURATIONS[offset],
                    TaskFieldLists::BASELINE_FINISHES[offset],
                    TaskFieldLists::BASELINE_FIXED_COST_ACCRUALS[offset],
                    TaskFieldLists::BASELINE_FIXED_COSTS[offset],
                    TaskFieldLists::BASELINE_STARTS[offset],
                    TaskFieldLists::BASELINE_WORKS[offset],
                ],
                "baseline {index} fields should match the same-named constants in TaskFieldLists"
            );
        }
    }
}
