//! 对应 Java: `org/mpxj/TaskType.java`

/// 任务类型的枚举。
///
/// 对应 Java: `public enum TaskType`
///
/// Java 侧该枚举不带 int 值、不实现 `MpxjEnum`，也没有 `getInstance`
/// （int ↔ `TaskType` 的映射在 Java 中由 `org.mpxj.mpp.TaskTypeHelper` 等辅助类完成）；
/// Rust 侧保持同样的形态（不实现 [`crate::mpxj_enum::MpxjEnum`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TaskType {
    /// 固定单位。
    ///
    /// 对应 Java: `FIXED_UNITS`
    FixedUnits,
    /// 固定工期。
    ///
    /// 对应 Java: `FIXED_DURATION`
    FixedDuration,
    /// 固定工时。
    ///
    /// 对应 Java: `FIXED_WORK`
    FixedWork,
    /// 同时固定工期与单位。
    ///
    /// 对应 Java: `FIXED_DURATION_AND_UNITS`
    FixedDurationAndUnits,
}

impl TaskType {
    /// 全部取值，按 Java 声明顺序排列。
    ///
    /// 对应 Java: `TaskType.values()`
    pub const ALL: [TaskType; 4] = [
        TaskType::FixedUnits,
        TaskType::FixedDuration,
        TaskType::FixedWork,
        TaskType::FixedDurationAndUnits,
    ];

    /// 按声明顺序（ordinal）取枚举实例。
    ///
    /// Java 侧无此方法；Rust 侧补充以便与 [`TaskType::ALL`] 配套查表，
    /// 对应 Java 中 `values()[value]` 的写法。Java 越界会抛
    /// `ArrayIndexOutOfBoundsException`，Rust 侧按本仓约定返回 `None`。
    pub fn get_instance(value: i32) -> Option<TaskType> {
        usize::try_from(value)
            .ok()
            .and_then(|index| TaskType::ALL.get(index))
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_type_round_trips() {
        for (index, item) in TaskType::ALL.iter().enumerate() {
            assert_eq!(TaskType::get_instance(index as i32), Some(*item));
        }
    }

    #[test]
    fn task_type_all_matches_java_declaration_order() {
        assert_eq!(
            TaskType::ALL,
            [
                TaskType::FixedUnits,
                TaskType::FixedDuration,
                TaskType::FixedWork,
                TaskType::FixedDurationAndUnits
            ]
        );
    }

    #[test]
    fn task_type_get_instance_is_out_of_range_safe() {
        assert_eq!(TaskType::get_instance(-1), None);
        assert_eq!(TaskType::get_instance(4), None);
    }
}
