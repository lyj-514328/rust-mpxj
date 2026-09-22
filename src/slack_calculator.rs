//! Corresponds to Java: `org/mpxj/SlackCalculator.java`
//!
//! # Dependency Notes
//!
//! The `Task` parameter type of this trait ([`crate::task::Task`]) has landed; this file only
//! uses it by `&Task` reference and does not depend on its concrete methods.

use crate::duration::Duration;
use crate::task::Task;

/// Strategy interface providing slack calculation for a project.
///
/// Corresponds to Java: `public interface SlackCalculator`
///
/// # Differences from Java
///
/// - Java's `Task` is a mutable object reference; each method on the Rust side only reads task
///   data, so the parameter is `&Task` (consistent with the usage in
///   `crate::common::slack_helper::SlackHelper::infer_slack`).
/// - Java's return type `Duration` may be `null`; the Rust side uses `Option<Duration>`.
pub trait SlackCalculator {
    /// Calculate the task's start slack.
    ///
    /// Corresponds to Java: `Duration calculateStartSlack(Task task)`
    fn calculate_start_slack(&self, task: &Task) -> Option<Duration>;

    /// Calculate the task's finish slack.
    ///
    /// Corresponds to Java: `Duration calculateFinishSlack(Task task)`
    fn calculate_finish_slack(&self, task: &Task) -> Option<Duration>;

    /// Calculate the task's free slack.
    ///
    /// Corresponds to Java: `Duration calculateFreeSlack(Task task)`
    fn calculate_free_slack(&self, task: &Task) -> Option<Duration>;

    /// Calculate the task's total slack.
    ///
    /// Corresponds to Java: `Duration calculateTotalSlack(Task task)`
    fn calculate_total_slack(&self, task: &Task) -> Option<Duration>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_unit::TimeUnit;

    /// A mock implementation returning constant values, used to verify the trait shape.
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

        // Trait object dispatch
        let dynamic: &dyn SlackCalculator = &ConstantCalculator;
        assert_eq!(
            dynamic.calculate_finish_slack(&task),
            Some(Duration::get_instance_int(2, TimeUnit::Hours))
        );
    }
}
