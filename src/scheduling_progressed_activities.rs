//! 对应 Java: `org/mpxj/SchedulingProgressedActivities.java`

/// 已推进活动排程方式的枚举。
///
/// 对应 Java: `public enum SchedulingProgressedActivities`
///
/// Java 侧该枚举不带 int 值、不实现 `MpxjEnum`，也没有 `getInstance`；
/// Rust 侧保持同样的形态（不实现 [`crate::mpxj_enum::MpxjEnum`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SchedulingProgressedActivities {
    /// 保留逻辑关系。
    ///
    /// 对应 Java: `RETAINED_LOGIC`
    RetainedLogic,
    /// 进度覆盖。
    ///
    /// 对应 Java: `PROGRESS_OVERRIDE`
    ProgressOverride,
    /// 按实际日期。
    ///
    /// 对应 Java: `ACTUAL_DATES`
    ActualDates,
}

impl SchedulingProgressedActivities {
    /// 全部取值，按 Java 声明顺序排列。
    ///
    /// 对应 Java: `SchedulingProgressedActivities.values()`
    pub const ALL: [SchedulingProgressedActivities; 3] = [
        SchedulingProgressedActivities::RetainedLogic,
        SchedulingProgressedActivities::ProgressOverride,
        SchedulingProgressedActivities::ActualDates,
    ];

    /// 按声明顺序（ordinal）取枚举实例。
    ///
    /// Java 侧无此方法；Rust 侧补充以便与 [`SchedulingProgressedActivities::ALL`]
    /// 配套查表，对应 Java 中 `values()[value]` 的写法。Java 越界会抛
    /// `ArrayIndexOutOfBoundsException`，Rust 侧按本仓约定返回 `None`。
    pub fn get_instance(value: i32) -> Option<SchedulingProgressedActivities> {
        usize::try_from(value)
            .ok()
            .and_then(|index| SchedulingProgressedActivities::ALL.get(index))
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduling_progressed_activities_round_trips() {
        for (index, item) in SchedulingProgressedActivities::ALL.iter().enumerate() {
            assert_eq!(
                SchedulingProgressedActivities::get_instance(index as i32),
                Some(*item)
            );
        }
    }

    #[test]
    fn scheduling_progressed_activities_all_matches_java_declaration_order() {
        assert_eq!(
            SchedulingProgressedActivities::ALL,
            [
                SchedulingProgressedActivities::RetainedLogic,
                SchedulingProgressedActivities::ProgressOverride,
                SchedulingProgressedActivities::ActualDates
            ]
        );
    }

    #[test]
    fn scheduling_progressed_activities_get_instance_is_out_of_range_safe() {
        assert_eq!(SchedulingProgressedActivities::get_instance(-1), None);
        assert_eq!(SchedulingProgressedActivities::get_instance(3), None);
    }
}
