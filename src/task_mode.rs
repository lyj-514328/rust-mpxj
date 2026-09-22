//! Corresponds to Java: `org/mpxj/TaskMode.java`

use crate::mpxj_enum::MpxjEnum;

/// Enum for task scheduling modes.
///
/// Corresponds to Java: `public enum TaskMode implements MpxjEnum`
///
/// In Java, `EnumHelper.createTypeArray` builds the int → enum array via reflection;
/// the Rust side does not use reflection and instead uses [`TaskMode::ALL`] plus the `match` in [`TaskMode::get_instance`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TaskMode {
    /// Manually scheduled.
    ///
    /// Corresponds to Java: `MANUALLY_SCHEDULED(0)`
    ManuallyScheduled = 0,
    /// Auto scheduled.
    ///
    /// Corresponds to Java: `AUTO_SCHEDULED(1)`
    AutoScheduled = 1,
}

impl TaskMode {
    /// All values, sorted ascending by int value.
    ///
    /// Corresponds to Java: `private static final TaskMode[] TYPE_VALUES` (excluding the reflection part).
    pub const ALL: [TaskMode; 2] = [TaskMode::ManuallyScheduled, TaskMode::AutoScheduled];

    /// Returns the enum instance for the given int value, falling back to `AUTO_SCHEDULED` when out of bounds.
    ///
    /// Corresponds to Java: `public static TaskMode getInstance(int type)`
    pub fn get_instance(value: i32) -> TaskMode {
        match value {
            0 => TaskMode::ManuallyScheduled,
            1 => TaskMode::AutoScheduled,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = AUTO_SCHEDULED.getValue(); }`
            _ => TaskMode::AutoScheduled,
        }
    }

    /// Corresponds to Java: `public static TaskMode getInstance(Number type)`
    ///
    /// In Java, `null` goes through `getInstance(-1)` and ultimately falls back to `AUTO_SCHEDULED`.
    pub fn get_instance_optional(value: Option<i32>) -> TaskMode {
        TaskMode::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for TaskMode {
    /// Returns the int value associated with this enum.
    ///
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<TaskMode> for i32 {
    fn from(value: TaskMode) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_mode_round_trips() {
        for item in TaskMode::ALL {
            assert_eq!(TaskMode::get_instance(item.value()), item);
        }
    }

    #[test]
    fn task_mode_falls_back_to_auto_scheduled() {
        assert_eq!(TaskMode::get_instance(-1), TaskMode::AutoScheduled);
        assert_eq!(TaskMode::get_instance(2), TaskMode::AutoScheduled);
        assert_eq!(
            TaskMode::get_instance_optional(None),
            TaskMode::AutoScheduled
        );
    }
}
