//! 对应 Java: `org/mpxj/ActivityType.java`

use crate::mpxj_enum::MpxjEnum;

/// P6/PPX 的活动类型。
///
/// 对应 Java: `public enum ActivityType`
///
/// Java 侧该枚举不实现 `MpxjEnum`，也没有 `getInstance`；Rust 侧按声明顺序提供
/// [`ActivityType::ALL`]，并用「序号（`ordinal()`）」实现 [`MpxjEnum::value`]，
/// 使 int 与枚举可以互转（`get_instance` 在 Java 侧无对应方法）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActivityType {
    /// 对应 Java: `TASK_DEPENDENT`
    TaskDependent,
    /// 对应 Java: `RESOURCE_DEPENDENT`
    ResourceDependent,
    /// 对应 Java: `LEVEL_OF_EFFORT`
    LevelOfEffort,
    /// 对应 Java: `START_MILESTONE`
    StartMilestone,
    /// 对应 Java: `FINISH_MILESTONE`
    FinishMilestone,
    /// 对应 Java: `WBS_SUMMARY`
    WbsSummary,
    /// 对应 Java: `HAMMOCK`
    Hammock,
    /// 对应 Java: `START_FLAG`
    StartFlag,
    /// 对应 Java: `FINISH_FLAG`
    FinishFlag,
}

impl ActivityType {
    /// 全部取值，顺序与 Java 声明顺序一致。
    pub const ALL: [ActivityType; 9] = [
        ActivityType::TaskDependent,
        ActivityType::ResourceDependent,
        ActivityType::LevelOfEffort,
        ActivityType::StartMilestone,
        ActivityType::FinishMilestone,
        ActivityType::WbsSummary,
        ActivityType::Hammock,
        ActivityType::StartFlag,
        ActivityType::FinishFlag,
    ];

    /// 按序号（Java 的 `ordinal()`）取枚举实例，越界返回 `None`。
    ///
    /// Java 侧无对应方法（Java 直接使用枚举常量或 `values()[n]`）。
    pub fn get_instance(value: i32) -> Option<ActivityType> {
        match value {
            0 => Some(ActivityType::TaskDependent),
            1 => Some(ActivityType::ResourceDependent),
            2 => Some(ActivityType::LevelOfEffort),
            3 => Some(ActivityType::StartMilestone),
            4 => Some(ActivityType::FinishMilestone),
            5 => Some(ActivityType::WbsSummary),
            6 => Some(ActivityType::Hammock),
            7 => Some(ActivityType::StartFlag),
            8 => Some(ActivityType::FinishFlag),
            _ => None,
        }
    }
}

impl MpxjEnum for ActivityType {
    /// 返回声明顺序对应的序号。
    ///
    /// Java 侧无 `getValue()`，此处对应 `Enum.ordinal()`。
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<ActivityType> for i32 {
    fn from(value: ActivityType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_type_round_trips() {
        for item in ActivityType::ALL {
            assert_eq!(ActivityType::get_instance(item.value()), Some(item));
        }
    }

    #[test]
    fn activity_type_out_of_range() {
        assert_eq!(ActivityType::get_instance(-1), None);
        assert_eq!(ActivityType::get_instance(9), None);
    }
}
