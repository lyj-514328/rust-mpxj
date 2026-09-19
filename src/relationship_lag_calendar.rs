//! 对应 Java: `org/mpxj/RelationshipLagCalendar.java`

/// 关系延隔（lag）计算所用日历的枚举。
///
/// 对应 Java: `public enum RelationshipLagCalendar`
///
/// Java 侧该枚举不带 int 值、不实现 `MpxjEnum`，也没有 `getInstance`；
/// Rust 侧保持同样的形态（不实现 [`crate::mpxj_enum::MpxjEnum`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RelationshipLagCalendar {
    /// 对应 Java: `PREDECESSOR`
    Predecessor,
    /// 对应 Java: `SUCCESSOR`
    Successor,
    /// 对应 Java: `PROJECT_DEFAULT`
    ProjectDefault,
    /// 对应 Java: `TWENTY_FOUR_HOUR`
    TwentyFourHour,
}

impl RelationshipLagCalendar {
    /// 全部取值，按 Java 声明顺序排列。
    ///
    /// 对应 Java: `RelationshipLagCalendar.values()`
    pub const ALL: [RelationshipLagCalendar; 4] = [
        RelationshipLagCalendar::Predecessor,
        RelationshipLagCalendar::Successor,
        RelationshipLagCalendar::ProjectDefault,
        RelationshipLagCalendar::TwentyFourHour,
    ];

    /// 按声明顺序（ordinal）取枚举实例。
    ///
    /// Java 侧无此方法；Rust 侧补充以便与 [`RelationshipLagCalendar::ALL`] 配套查表，
    /// 对应 Java 中 `values()[value]` 的写法。Java 越界会抛
    /// `ArrayIndexOutOfBoundsException`，Rust 侧按本仓约定返回 `None`。
    pub fn get_instance(value: i32) -> Option<RelationshipLagCalendar> {
        usize::try_from(value)
            .ok()
            .and_then(|index| RelationshipLagCalendar::ALL.get(index))
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relationship_lag_calendar_round_trips() {
        for (index, item) in RelationshipLagCalendar::ALL.iter().enumerate() {
            assert_eq!(
                RelationshipLagCalendar::get_instance(index as i32),
                Some(*item)
            );
        }
    }

    #[test]
    fn relationship_lag_calendar_all_matches_java_declaration_order() {
        assert_eq!(
            RelationshipLagCalendar::ALL,
            [
                RelationshipLagCalendar::Predecessor,
                RelationshipLagCalendar::Successor,
                RelationshipLagCalendar::ProjectDefault,
                RelationshipLagCalendar::TwentyFourHour
            ]
        );
    }

    #[test]
    fn relationship_lag_calendar_get_instance_is_out_of_range_safe() {
        assert_eq!(RelationshipLagCalendar::get_instance(-1), None);
        assert_eq!(RelationshipLagCalendar::get_instance(4), None);
    }
}
