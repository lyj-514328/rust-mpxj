//! 对应 Java: `org/mpxj/cpm/MicrosoftSlackCalculator.java`
//!
//! # 依赖说明（重要）
//!
//! `Task`、`Relation`、`ProjectFile`、`ProjectCalendar`、`ProjectProperties` 由并行任务
//! 翻译中，当前可能尚未落地。本文件按下列签名使用它们：
//!
//! ```text
//! Task::get_duration(&self) -> Option<Duration>
//! Task::get_constraint_type(&self) -> ConstraintType
//! Task::get_late_start / get_early_start / get_late_finish / get_early_finish
//!     / get_actual_start / get_actual_finish(&self) -> Option<NaiveDateTime>
//! Task::get_summary(&self) -> bool
//! Task::get_effective_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>>
//! Task::get_child_tasks(&self) -> Vec<Rc<RefCell<Task>>>
//! Task::get_successors(&self) -> Vec<Rc<RefCell<Relation>>>
//! Task::get_start_slack / get_finish_slack / get_total_slack(&self) -> Option<Duration>
//! Task::get_parent_file(&self) -> Rc<RefCell<ProjectFile>>
//! ProjectFile::get_project_properties(&self) -> &ProjectProperties
//! ProjectProperties::get_total_slack_calculation_type(&self) -> TotalSlackCalculationType
//! Relation::get_predecessor_task / get_successor_task(&self) -> Option<Rc<RefCell<Task>>>
//! Relation::get_type(&self) -> RelationType
//! Relation::get_lag(&self) -> Option<Duration>
//! ```
//!
//! # 与 Java 的差异
//!
//! - **方法重载**：Java 有 `calculateFreeSlack(Task)`（public）与
//!   `calculateFreeSlack(Relation)`（private）两个同名重载。Rust 不支持重载，
//!   私有的那个改名为 [`MicrosoftSlackCalculator::calculate_free_slack_relation`]。
//! - **空值处理**：Java 对 `relation.getSuccessorTask()` / `getPredecessorTask()` /
//!   `getLag()` 为 `null` 时会抛 `NullPointerException`；Rust 侧不 panic，
//!   分别按「跳过该关系 / 返回零工期 / 视为无 lag」处理。
//! - **日历借用**：`Task::get_effective_calendar` 返回 `Rc<RefCell<ProjectCalendar>>`。
//!   Java 通过 `LocalDateTimeHelper.getVariance` 计算日期差值，但
//!   `crate::common::local_date_time_helper` 目前未在 `crate::common::mod` 中启用
//!   （其注释写明待 `project_calendar` 落地后再开启）。因此本文件与
//!   `crate::availability_table` / `crate::cost_rate_table_entry` 一样，
//!   在私有方法 [`MicrosoftSlackCalculator::get_variance`] 中内联 `LocalDateTimeHelper.getVariance`
//!   的行为（日历缺失或日期缺失时返回 0 工期，否则调用 `ProjectCalendar::get_work`）。

use chrono::NaiveDateTime;

use crate::constraint_type::ConstraintType;
use crate::duration::Duration;
use crate::relation::Relation;
use crate::relation_type::RelationType;
use crate::slack_calculator::SlackCalculator;
use crate::task::Task;
use crate::time_unit::TimeUnit;
use crate::total_slack_calculation_type::TotalSlackCalculationType;

/// 与 Microsoft Project 一致的 slack 计算实现。
///
/// 对应 Java: `public class MicrosoftSlackCalculator implements SlackCalculator`
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MicrosoftSlackCalculator;

impl MicrosoftSlackCalculator {
    /// 构造方法（Java 未显式声明，等价于默认构造）。
    ///
    /// 对应 Java: 隐式默认构造方法
    pub fn new() -> Self {
        Self
    }

    /// 借用适配：把任务的有效日历借给 `LocalDateTimeHelper.getVariance`。
    ///
    /// 对应 Java: `LocalDateTimeHelper.getVariance(task.getEffectiveCalendar(), date1, date2, format)`
    ///
    /// 实现说明见文件头：`crate::common::local_date_time_helper` 尚未在 `common/mod.rs` 中启用，
    /// 故这里内联该方法的行为——日历或任一日期缺失时返回 0 工期，否则调用
    /// `ProjectCalendar::get_work`。
    fn get_variance(
        &self,
        task: &Task,
        date1: Option<NaiveDateTime>,
        date2: Option<NaiveDateTime>,
        format: TimeUnit,
    ) -> Duration {
        let Some(calendar) = task.get_effective_calendar() else {
            return Duration::get_instance(0.0, format);
        };

        let (Some(date1), Some(date2)) = (date1, date2) else {
            return Duration::get_instance(0.0, format);
        };

        // 用 `return` 让 `Ref` 临时值在使用后立即释放，避免借用检查报错。
        return calendar.borrow().get_work(date1, date2, format);
    }

    /// 计算一个任务的 early finish 与另一个任务的 early start 之间的差值（variance）。
    ///
    /// 对应 Java: `private Duration calculateVariance(Task t1, Task t2)`
    ///
    /// Java 的 `t2` 参数在方法体内并未使用，这里保留以对应 Java 签名，故命名为 `_t2`。
    fn calculate_variance(&self, t1: &Task, _t2: &Task) -> Option<Duration> {
        let variance = self.get_variance(
            t1,
            t1.get_early_finish(),
            t1.get_early_start(),
            TimeUnit::Hours,
        );

        if variance.get_duration() < 0.0 {
            return None;
        }

        Some(variance)
    }

    /// 计算两个任务之间的自由浮动时间（对应 Java 的私有重载）。
    ///
    /// 对应 Java: `private Duration calculateFreeSlack(Relation relation)`
    ///
    /// 因 Rust 不支持方法重载，这里以 `_relation` 后缀区分公开的
    /// [`SlackCalculator::calculate_free_slack`]。
    fn calculate_free_slack_relation(&self, relation: &Relation) -> Option<Duration> {
        let predecessor_task = relation.get_predecessor_task()?;
        let predecessor_task = predecessor_task.borrow();
        let successor_task = relation.get_successor_task()?;
        let successor_task = successor_task.borrow();

        let predecessor_start = if predecessor_task.get_constraint_type()
            == Some(ConstraintType::AsLateAsPossible)
        {
            predecessor_task.get_late_start()
        } else {
            predecessor_task.get_early_start()
        };

        let predecessor_finish = if predecessor_task.get_constraint_type()
            == Some(ConstraintType::AsLateAsPossible)
        {
            predecessor_task.get_late_finish()
        } else {
            predecessor_task.get_early_finish()
        };

        let successor_start = if successor_task.get_constraint_type()
            == Some(ConstraintType::AsLateAsPossible)
        {
            successor_task.get_late_start()
        } else {
            successor_task.get_early_start()
        };

        let successor_finish = if successor_task.get_constraint_type()
            == Some(ConstraintType::AsLateAsPossible)
        {
            successor_task.get_late_finish()
        } else {
            successor_task.get_early_finish()
        };

        match relation.get_type() {
            RelationType::FinishStart => Some(self.calculate_free_slack_variance(
                relation,
                predecessor_finish,
                successor_start,
            )),
            RelationType::StartStart => Some(self.calculate_free_slack_variance(
                relation,
                predecessor_start,
                successor_start,
            )),
            RelationType::FinishFinish => Some(self.calculate_free_slack_variance(
                relation,
                predecessor_finish,
                successor_finish,
            )),
            RelationType::StartFinish => Some(self.calculate_free_slack_variance(
                relation,
                predecessor_start,
                successor_finish,
            )),
        }
    }

    /// 在一条 Relation 的上下文中计算两个日期之间的差值。
    ///
    /// 对应 Java: `private Duration calculateFreeSlackVariance(Relation relation, LocalDateTime date1, LocalDateTime date2)`
    fn calculate_free_slack_variance(
        &self,
        relation: &Relation,
        date1: Option<NaiveDateTime>,
        date2: Option<NaiveDateTime>,
    ) -> Duration {
        // Java 在 predecessorTask 为 null 时会抛 NPE；Rust 侧不 panic，返回零工期。
        let Some(predecessor_task) = relation.get_predecessor_task() else {
            return Duration::get_instance_int(0, TimeUnit::Hours);
        };
        let predecessor_task = predecessor_task.borrow();

        let format = predecessor_task
            .get_duration()
            .map(|duration| duration.get_units())
            .unwrap_or(TimeUnit::Hours);

        if date1.is_none() || date2.is_none() {
            return Duration::get_instance_int(0, format);
        }

        let variance = self.get_variance(&predecessor_task, date1, date2, format);
        self.remove_lag(relation, variance)
    }

    /// 从工期中扣除 lag。
    ///
    /// 对应 Java: `private Duration removeLag(Relation relation, Duration duration)`
    fn remove_lag(&self, relation: &Relation, duration: Duration) -> Duration {
        // Java 在 lag 为 null 时会抛 NPE；Rust 侧视为无 lag。
        let Some(lag) = relation.get_lag() else {
            return duration;
        };

        if lag.get_duration() == 0.0 {
            return duration;
        }

        let lag_units = lag.get_units();
        let duration_units = duration.get_units();

        let lag = if lag_units != duration_units {
            if lag_units == TimeUnit::Percent {
                // Java 在 predecessorTask / duration 为 null 时会抛 NPE；
                // Rust 侧在取不到时视为无 lag。
                let Some(predecessor_duration) = relation
                    .get_predecessor_task()
                    .and_then(|task| task.borrow().get_duration())
                else {
                    return duration;
                };

                Duration::get_instance(
                    (predecessor_duration.get_duration() * lag.get_duration()) / 100.0,
                    predecessor_duration.get_units(),
                )
            } else {
                let calendar = relation
                    .get_predecessor_task()
                    .and_then(|task| task.borrow().get_effective_calendar());

                match calendar {
                    Some(calendar) => {
                        let calendar = calendar.borrow();
                        lag.convert_units(duration_units, &*calendar)
                    }
                    // 无法取到日历时保留原 lag（Java 会抛 NPE）
                    None => lag,
                }
            }
        } else {
            lag
        };

        Duration::get_instance(duration.get_duration() - lag.get_duration(), duration_units)
    }
}

impl SlackCalculator for MicrosoftSlackCalculator {
    /// 计算任务的开始浮动时间。
    ///
    /// 对应 Java: `@Override public Duration calculateStartSlack(Task task)`
    fn calculate_start_slack(&self, task: &Task) -> Option<Duration> {
        let duration = task.get_duration()?;

        if task.get_constraint_type() == Some(ConstraintType::AsLateAsPossible) {
            return Some(Duration::get_instance_int(0, duration.get_units()));
        }

        let late_start = task.get_late_start();
        let early_start = task.get_early_start();

        if late_start.is_none() || early_start.is_none() {
            return None;
        }

        Some(self.get_variance(task, early_start, late_start, duration.get_units()))
    }

    /// 计算任务的完成浮动时间。
    ///
    /// 对应 Java: `@Override public Duration calculateFinishSlack(Task task)`
    fn calculate_finish_slack(&self, task: &Task) -> Option<Duration> {
        let duration = task.get_duration()?;

        if task.get_constraint_type() == Some(ConstraintType::AsLateAsPossible) {
            return Some(Duration::get_instance_int(0, duration.get_units()));
        }

        let early_finish = task.get_early_finish();
        let late_finish = task.get_late_finish();

        if early_finish.is_none() || late_finish.is_none() {
            return None;
        }

        Some(self.get_variance(task, early_finish, late_finish, duration.get_units()))
    }

    /// 计算任务的自由浮动时间。
    ///
    /// 对应 Java: `@Override public Duration calculateFreeSlack(Task task)`
    fn calculate_free_slack(&self, task: &Task) -> Option<Duration> {
        // 任务已完成时，自由浮动时间恒为零。
        if task.get_actual_finish().is_some() {
            let units = task
                .get_duration()
                .map(|duration| duration.get_units())
                .unwrap_or(TimeUnit::Hours);
            return Some(Duration::get_instance_int(0, units));
        }

        if task.get_summary() {
            let mut free_float: Option<Duration> = None;
            for child in task.get_child_tasks() {
                for relation in child.borrow().get_successors() {
                    let relation = relation.borrow();
                    let Some(successor) = relation.get_successor_task() else {
                        continue;
                    };
                    let successor = successor.borrow();

                    // 忽略已完成的后续任务
                    if successor.get_actual_finish().is_some() {
                        continue;
                    }

                    if let Some(variance) = self.calculate_variance(task, &successor) {
                        free_float = Some(match free_float {
                            Some(current) => current.min(variance),
                            None => variance,
                        });
                    }
                }
            }

            let free_float = free_float.or_else(|| task.get_total_slack());

            if let Some(free_float) = free_float {
                if free_float.get_duration() < 0.0 {
                    return Some(Duration::get_instance_int(0, free_float.get_units()));
                }
            }

            return free_float;
        }

        let mut free_float: Option<Duration> = None;
        for relation in task.get_successors() {
            let relation = relation.borrow();
            let Some(successor) = relation.get_successor_task() else {
                continue;
            };

            // 忽略已完成的后续任务
            if successor.borrow().get_actual_finish().is_some() {
                continue;
            }

            if let Some(variance) = self.calculate_free_slack_relation(&relation) {
                free_float = Some(match free_float {
                    Some(current) => current.min(variance),
                    None => variance,
                });
            }
        }

        let free_float = free_float.or_else(|| task.get_total_slack());

        if let Some(free_float) = free_float {
            if free_float.get_duration() < 0.0 {
                return Some(Duration::get_instance_int(0, TimeUnit::Hours));
            }
        }

        free_float
    }

    /// 计算任务的总浮动时间。
    ///
    /// 对应 Java: `@Override public Duration calculateTotalSlack(Task task)`
    fn calculate_total_slack(&self, task: &Task) -> Option<Duration> {
        // 先取出这些值，避免在计算过程中清空 total slack。
        let duration = task.get_duration();
        let start_slack = task.get_start_slack();
        let finish_slack = task.get_finish_slack();

        let parent_file = task.get_parent_file()?;
        let parent_file = parent_file.borrow();
        let properties = parent_file.get_project_properties();
        let calculation_type = properties.get_total_slack_calculation_type();

        if calculation_type == TotalSlackCalculationType::StartSlack {
            return start_slack;
        }

        if calculation_type == TotalSlackCalculationType::FinishSlack {
            return finish_slack;
        }

        if task.get_actual_start().is_some() {
            return finish_slack;
        }

        let duration = duration?;
        let mut start_slack = start_slack?;
        let mut finish_slack = finish_slack?;

        let units = duration.get_units();
        if start_slack.get_units() != units {
            start_slack = start_slack.convert_units(units, properties);
        }

        if finish_slack.get_units() != units {
            finish_slack = finish_slack.convert_units(units, properties);
        }

        if start_slack.get_duration() < finish_slack.get_duration() {
            Some(start_slack)
        } else {
            Some(finish_slack)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_default_are_equivalent() {
        assert_eq!(MicrosoftSlackCalculator::new(), MicrosoftSlackCalculator);
        assert_eq!(
            MicrosoftSlackCalculator::default(),
            MicrosoftSlackCalculator::new()
        );
    }

    #[test]
    fn empty_task_has_no_slack() {
        let calculator = MicrosoftSlackCalculator::new();
        let task = Task::new();

        // 无工期 / 无日期 / 无后继任务的空任务，各 slack 均为 None
        assert_eq!(calculator.calculate_start_slack(&task), None);
        assert_eq!(calculator.calculate_finish_slack(&task), None);
        assert_eq!(calculator.calculate_free_slack(&task), None);
    }

    #[test]
    fn trait_object_dispatch_matches_inherent_calls() {
        let calculator: &dyn SlackCalculator = &MicrosoftSlackCalculator::new();
        let task = Task::new();

        assert_eq!(
            calculator.calculate_start_slack(&task),
            MicrosoftSlackCalculator::new().calculate_start_slack(&task)
        );
    }
}
