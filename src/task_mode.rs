//! 对应 Java: `org/mpxj/TaskMode.java`

use crate::mpxj_enum::MpxjEnum;

/// 任务排程模式的枚举。
///
/// 对应 Java: `public enum TaskMode implements MpxjEnum`
///
/// Java 侧用 `EnumHelper.createTypeArray` 反射生成 int → 枚举的数组；
/// Rust 侧不使用反射，改为 [`TaskMode::ALL`] 加 [`TaskMode::get_instance`] 的 `match`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TaskMode {
    /// 手动排程。
    ///
    /// 对应 Java: `MANUALLY_SCHEDULED(0)`
    ManuallyScheduled = 0,
    /// 自动排程。
    ///
    /// 对应 Java: `AUTO_SCHEDULED(1)`
    AutoScheduled = 1,
}

impl TaskMode {
    /// 全部取值，按 int 值升序排列。
    ///
    /// 对应 Java: `private static final TaskMode[] TYPE_VALUES`（不含反射部分）。
    pub const ALL: [TaskMode; 2] = [TaskMode::ManuallyScheduled, TaskMode::AutoScheduled];

    /// 根据 int 值取枚举实例，越界时回落到 `AUTO_SCHEDULED`。
    ///
    /// 对应 Java: `public static TaskMode getInstance(int type)`
    pub fn get_instance(value: i32) -> TaskMode {
        match value {
            0 => TaskMode::ManuallyScheduled,
            1 => TaskMode::AutoScheduled,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = AUTO_SCHEDULED.getValue(); }`
            _ => TaskMode::AutoScheduled,
        }
    }

    /// 对应 Java: `public static TaskMode getInstance(Number type)`
    ///
    /// Java 侧 `null` 走 `getInstance(-1)`，最终回落到 `AUTO_SCHEDULED`。
    pub fn get_instance_optional(value: Option<i32>) -> TaskMode {
        TaskMode::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for TaskMode {
    /// 返回该枚举关联的 int 值。
    ///
    /// 对应 Java: `@Override public int getValue()`
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
