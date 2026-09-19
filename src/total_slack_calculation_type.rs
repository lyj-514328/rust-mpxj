//! 对应 Java: `org/mpxj/TotalSlackCalculationType.java`

/// 总浮动时间（total slack）计算方式的枚举。
///
/// 对应 Java: `public enum TotalSlackCalculationType`
///
/// Java 侧该枚举不带 int 值、不实现 `MpxjEnum`，也没有 `getInstance`；
/// Rust 侧保持同样的形态（不实现 [`crate::mpxj_enum::MpxjEnum`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TotalSlackCalculationType {
    /// 取最小浮动时间。
    ///
    /// 对应 Java: `SMALLEST_SLACK`
    SmallestSlack,
    /// 取开始浮动时间。
    ///
    /// 对应 Java: `START_SLACK`
    StartSlack,
    /// 取完成浮动时间。
    ///
    /// 对应 Java: `FINISH_SLACK`
    FinishSlack,
}

impl TotalSlackCalculationType {
    /// 全部取值，按 Java 声明顺序排列。
    ///
    /// 对应 Java: `TotalSlackCalculationType.values()`
    pub const ALL: [TotalSlackCalculationType; 3] = [
        TotalSlackCalculationType::SmallestSlack,
        TotalSlackCalculationType::StartSlack,
        TotalSlackCalculationType::FinishSlack,
    ];

    /// 按声明顺序（ordinal）取枚举实例。
    ///
    /// Java 侧无此方法；Rust 侧补充以便与 [`TotalSlackCalculationType::ALL`]
    /// 配套查表，对应 Java 中 `values()[value]` 的写法。Java 越界会抛
    /// `ArrayIndexOutOfBoundsException`，Rust 侧按本仓约定返回 `None`。
    pub fn get_instance(value: i32) -> Option<TotalSlackCalculationType> {
        usize::try_from(value)
            .ok()
            .and_then(|index| TotalSlackCalculationType::ALL.get(index))
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_slack_calculation_type_round_trips() {
        for (index, item) in TotalSlackCalculationType::ALL.iter().enumerate() {
            assert_eq!(
                TotalSlackCalculationType::get_instance(index as i32),
                Some(*item)
            );
        }
    }

    #[test]
    fn total_slack_calculation_type_all_matches_java_declaration_order() {
        assert_eq!(
            TotalSlackCalculationType::ALL,
            [
                TotalSlackCalculationType::SmallestSlack,
                TotalSlackCalculationType::StartSlack,
                TotalSlackCalculationType::FinishSlack
            ]
        );
    }

    #[test]
    fn total_slack_calculation_type_get_instance_is_out_of_range_safe() {
        assert_eq!(TotalSlackCalculationType::get_instance(-1), None);
        assert_eq!(TotalSlackCalculationType::get_instance(3), None);
    }
}
