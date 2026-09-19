//! 对应 Java: `org/mpxj/ActivityStatus.java`

use crate::mpxj_enum::MpxjEnum;

/// 活动（Activity）的执行状态。
///
/// 对应 Java: `public enum ActivityStatus`
///
/// Java 侧该枚举不实现 `MpxjEnum`，也没有 `getInstance`；Rust 侧按声明顺序提供
/// [`ActivityStatus::ALL`]，并用「序号（`ordinal()`）」实现 [`MpxjEnum::value`]，
/// 使 int 与枚举可以互转（`get_instance` 在 Java 侧无对应方法）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActivityStatus {
    /// 对应 Java: `NOT_STARTED`
    NotStarted,
    /// 对应 Java: `IN_PROGRESS`
    InProgress,
    /// 对应 Java: `COMPLETED`
    Completed,
}

impl ActivityStatus {
    /// 全部取值，顺序与 Java 声明顺序一致。
    pub const ALL: [ActivityStatus; 3] = [
        ActivityStatus::NotStarted,
        ActivityStatus::InProgress,
        ActivityStatus::Completed,
    ];

    /// 按序号（Java 的 `ordinal()`）取枚举实例，越界返回 `None`。
    ///
    /// Java 侧无对应方法（Java 直接使用枚举常量或 `values()[n]`）。
    pub fn get_instance(value: i32) -> Option<ActivityStatus> {
        match value {
            0 => Some(ActivityStatus::NotStarted),
            1 => Some(ActivityStatus::InProgress),
            2 => Some(ActivityStatus::Completed),
            _ => None,
        }
    }
}

impl MpxjEnum for ActivityStatus {
    /// 返回声明顺序对应的序号。
    ///
    /// Java 侧无 `getValue()`，此处对应 `Enum.ordinal()`。
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<ActivityStatus> for i32 {
    fn from(value: ActivityStatus) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_status_round_trips() {
        for item in ActivityStatus::ALL {
            assert_eq!(ActivityStatus::get_instance(item.value()), Some(item));
        }
    }

    #[test]
    fn activity_status_out_of_range() {
        assert_eq!(ActivityStatus::get_instance(-1), None);
        assert_eq!(ActivityStatus::get_instance(3), None);
    }
}
