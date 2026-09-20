//! 对应 Java: `org/mpxj/DefaultBaselineStrategy.java`
//!
//! # 与 Java 的差异
//!
//! - **继承**：Java 的 `DefaultBaselineStrategy extends AbstractBaselineStrategy`，
//!   而 `AbstractBaselineStrategy` 不在本次移植范围内。Rust 无继承，
//!   故把基类的实现（`clearBaseline` / `populateBaseline` / `getKeyForTask` /
//!   `getBaselineFields` / `getSourceFields`）原样并入本类型。
//! - **`TaskFieldLists`**：Java 从 `org.mpxj.common.TaskFieldLists` 取 baseline 字段数组；
//!   Rust 侧引用同一份已落地的常量（[`crate::common::task_field_lists::TaskFieldLists`]），
//!   不在本文件内另定义一份，避免两边取值漂移。
//! - **单例 `INSTANCE`**：Java 的 `public static final DefaultBaselineStrategy INSTANCE`
//!   在 Rust 侧用同名常量 [`DefaultBaselineStrategy::INSTANCE`] 表达。本类型无可变字段，
//!   可以直接 `const` 构造，因此无需 `OnceLock` / `instance()` 函数
//!   （`crate::project_config::ProjectConfig` 也按 `DefaultBaselineStrategy::INSTANCE` 使用）。
//!
//! # 用到的 `ProjectFile` / `Task` 接口
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
//! Java 的 `Map<Task, Task>`（baselineTaskMap）在 Rust 侧改用任务的唯一 ID 作为键，
//! 语义等价（唯一 ID 在同一个 schedule 内唯一）。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::baseline_strategy::BaselineStrategy;
use crate::common::task_field_lists::TaskFieldLists;
use crate::field_key::FieldKey;
use crate::project_file::ProjectFile;
use crate::task::Task;
use crate::task_field::TaskField;

/// 用一份 project 的值填充另一份 project 的 baseline 字段。
///
/// 对应 Java: `public final class DefaultBaselineStrategy extends AbstractBaselineStrategy`
///
/// 该类无可变状态，故实现为无字段（unit）结构体；Java 的私有构造方法无需显式表达。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultBaselineStrategy;

impl DefaultBaselineStrategy {
    /// Java 的单例常量 `INSTANCE`。
    ///
    /// 对应 Java: `public static final DefaultBaselineStrategy INSTANCE`
    pub const INSTANCE: DefaultBaselineStrategy = DefaultBaselineStrategy;

    /// 生成用于在当前 schedule 与 baseline schedule 之间匹配任务的键。
    ///
    /// 对应 Java: `protected Object getKeyForTask(Task task)`
    ///
    /// Java 返回 `task.getGUID()`；返回 `null` 的任务不参与匹配。
    pub fn get_key_for_task(&self, task: &Task) -> Option<String> {
        task.get_guid().map(|guid| guid.to_string())
    }

    /// 确定要填充的 baseline 字段集合。
    ///
    /// 对应 Java: `protected TaskField[] getBaselineFields(int index)`
    ///
    /// `index` 为 `0` 时返回主 baseline 字段，否则返回 baseline 1-10 的字段。
    /// Java 对越界下标会抛 `ArrayIndexOutOfBoundsException`；Rust 侧不 panic，返回空集合。
    pub fn get_baseline_fields(&self, index: i32) -> Vec<TaskField> {
        if index == 0 {
            return BASELINE0_FIELDS.to_vec();
        }

        // Java: `--index;` 后按 0-based 下标取 baseline 1-10 的字段
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

    /// 从 baseline schedule 中取值的源字段。
    ///
    /// 对应 Java: `protected TaskField[] getSourceFields()`
    pub fn get_source_fields(&self) -> Vec<TaskField> {
        SOURCE_FIELDS.to_vec()
    }

    /// 用一个任务（或 `None`）的值填充另一个任务的 baseline 字段。
    ///
    /// 对应 Java: `private void populateBaseline(Task task, Task baseline, TaskField[] baselineFields)`
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
    /// 清空指定 project 中指定序号的 baseline。
    ///
    /// 对应 Java: `@Override public void clearBaseline(ProjectFile project, int index)`
    fn clear_baseline(&self, project: &mut ProjectFile, index: i32) {
        let baseline_fields = self.get_baseline_fields(index);

        for task in project.get_tasks() {
            self.populate_baseline_for_task(&mut task.borrow_mut(), None, &baseline_fields);
        }
    }

    /// 用 `baseline` project 的值填充 `project` 中指定序号的 baseline。
    ///
    /// 对应 Java: `@Override public void populateBaseline(ProjectFile project, ProjectFile baseline, int index)`
    fn populate_baseline(&self, project: &mut ProjectFile, baseline: &ProjectFile, index: i32) {
        let baseline_fields = self.get_baseline_fields(index);

        // 建立「baseline 任务键 → baseline 任务」的映射。
        // Java 用 `Collectors.toMap(..., (u, v) -> null)`：键冲突时该键的值为 null，
        // 后续按 `map.get(key) != null` 过滤，因此重复键的任务不会被匹配。
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

        // 建立「当前 project 任务唯一 ID → baseline 任务」的映射（对应 Java 的 Map<Task, Task>）。
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

        // 缓存该映射，供 Task.getBaselineTask() 使用。
        project.set_baseline_task_map(index, baseline_task_map.clone());

        // 遍历**全部**任务（而不是映射本身），确保没有 baseline 的任务其
        // baseline 字段被置为 null。
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

/// baseline schedule 中取值的源字段。
///
/// 对应 Java: `private static final TaskField[] SOURCE_FIELDS`
const SOURCE_FIELDS: [TaskField; 7] = [
    TaskField::Cost,
    TaskField::Duration,
    TaskField::Finish,
    TaskField::FixedCostAccrual,
    TaskField::FixedCost,
    TaskField::Start,
    TaskField::Work,
];

/// 主 baseline（index 0）字段。
///
/// 对应 Java: `private static final TaskField[] BASELINE0_FIELDS`
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
        // Java 的 `INSTANCE` 是无状态单例；Rust 侧为可直接拷贝的常量值。
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

    /// 去重回归：baseline 1-10 的字段必须与 [`TaskFieldLists`] 的 7 个常量**同源**。
    ///
    /// 本文件曾自备一份 `BASELINE_*` 常量数组，与 `TaskFieldLists` 的取值各自演化；
    /// 本测试把「两边取值一致」锁死。
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
                "baseline {index} 的字段应与 TaskFieldLists 的同名常量一致"
            );
        }
    }
}
