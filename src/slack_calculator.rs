//! 对应 Java: `org/mpxj/SlackCalculator.java`
//!
//! # 依赖说明
//!
//! 本 trait 的参数类型 `Task`（[`crate::task::Task`]）已落地；本文件只按 `&Task`
//! 引用使用它，不依赖其具体方法。

use crate::duration::Duration;
use crate::task::Task;

/// 为 project 提供 slack（时差）计算的策略接口。
///
/// 对应 Java: `public interface SlackCalculator`
///
/// # 与 Java 的差异
///
/// - Java 的 `Task` 是可变对象引用；Rust 侧各方法只读取任务数据，故参数为 `&Task`
///   （与 `crate::common::slack_helper::SlackHelper::infer_slack` 的用法保持一致）。
/// - Java 的返回类型 `Duration` 允许为 `null`，Rust 侧对应 `Option<Duration>`。
pub trait SlackCalculator {
    /// 计算任务的开始浮动时间（start slack）。
    ///
    /// 对应 Java: `Duration calculateStartSlack(Task task)`
    fn calculate_start_slack(&self, task: &Task) -> Option<Duration>;

    /// 计算任务的完成浮动时间（finish slack）。
    ///
    /// 对应 Java: `Duration calculateFinishSlack(Task task)`
    fn calculate_finish_slack(&self, task: &Task) -> Option<Duration>;

    /// 计算任务的自由浮动时间（free slack）。
    ///
    /// 对应 Java: `Duration calculateFreeSlack(Task task)`
    fn calculate_free_slack(&self, task: &Task) -> Option<Duration>;

    /// 计算任务的总浮动时间（total slack）。
    ///
    /// 对应 Java: `Duration calculateTotalSlack(Task task)`
    fn calculate_total_slack(&self, task: &Task) -> Option<Duration>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_unit::TimeUnit;

    /// 返回固定常量的 mock 实现，用于验证 trait 形状。
    struct ConstantCalculator;

    impl SlackCalculator for ConstantCalculator {
        fn calculate_start_slack(&self, _task: &Task) -> Option<Duration> {
            Some(Duration::get_instance_int(1, TimeUnit::Hours))
        }

        fn calculate_finish_slack(&self, _task: &Task) -> Option<Duration> {
            Some(Duration::get_instance_int(2, TimeUnit::Hours))
        }

        fn calculate_free_slack(&self, _task: &Task) -> Option<Duration> {
            Some(Duration::get_instance_int(3, TimeUnit::Hours))
        }

        fn calculate_total_slack(&self, _task: &Task) -> Option<Duration> {
            None
        }
    }

    #[test]
    fn trait_can_be_implemented_and_dispatched_dynamically() {
        let calculator = ConstantCalculator;
        let task = Task::new();

        assert_eq!(
            calculator.calculate_start_slack(&task),
            Some(Duration::get_instance_int(1, TimeUnit::Hours))
        );
        assert_eq!(
            calculator.calculate_free_slack(&task),
            Some(Duration::get_instance_int(3, TimeUnit::Hours))
        );
        assert_eq!(calculator.calculate_total_slack(&task), None);

        // trait 对象分发
        let dynamic: &dyn SlackCalculator = &ConstantCalculator;
        assert_eq!(
            dynamic.calculate_finish_slack(&task),
            Some(Duration::get_instance_int(2, TimeUnit::Hours))
        );
    }
}
