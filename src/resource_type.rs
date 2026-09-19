//! 对应 Java: `org/mpxj/ResourceType.java`

/// 资源类型的枚举。
///
/// 对应 Java: `public enum ResourceType`
///
/// Java 侧该枚举不带 int 值、不实现 `MpxjEnum`，也没有 `getInstance`；
/// Rust 侧保持同样的形态（不实现 [`crate::mpxj_enum::MpxjEnum`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ResourceType {
    /// 材料资源。
    ///
    /// 对应 Java: `MATERIAL(false)`
    Material,
    /// 工时（人工）资源。
    ///
    /// 对应 Java: `WORK(true)`
    Work,
    /// 成本资源。
    ///
    /// 对应 Java: `COST(false)`
    Cost,
    /// 非人工资源。
    ///
    /// 对应 Java: `NON_LABOR(true)`
    NonLabor,
}

impl ResourceType {
    /// 全部取值，按 Java 声明顺序排列。
    ///
    /// 对应 Java: `ResourceType.values()`
    pub const ALL: [ResourceType; 4] = [
        ResourceType::Material,
        ResourceType::Work,
        ResourceType::Cost,
        ResourceType::NonLabor,
    ];

    /// 按声明顺序（ordinal）取枚举实例。
    ///
    /// Java 侧无此方法；Rust 侧补充以便与 [`ResourceType::ALL`] 配套查表，
    /// 对应 Java 中 `values()[value]` 的写法。Java 越界会抛
    /// `ArrayIndexOutOfBoundsException`，Rust 侧按本仓约定返回 `None`。
    pub fn get_instance(value: i32) -> Option<ResourceType> {
        usize::try_from(value)
            .ok()
            .and_then(|index| ResourceType::ALL.get(index))
            .copied()
    }

    /// 该资源类型是否基于时间。
    ///
    /// 对应 Java: `public boolean isTimeBased()`
    pub fn is_time_based(&self) -> bool {
        match self {
            ResourceType::Material => false,
            ResourceType::Work => true,
            ResourceType::Cost => false,
            ResourceType::NonLabor => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_type_round_trips() {
        for (index, item) in ResourceType::ALL.iter().enumerate() {
            assert_eq!(ResourceType::get_instance(index as i32), Some(*item));
        }
    }

    #[test]
    fn resource_type_all_matches_java_declaration_order() {
        assert_eq!(
            ResourceType::ALL,
            [
                ResourceType::Material,
                ResourceType::Work,
                ResourceType::Cost,
                ResourceType::NonLabor
            ]
        );
    }

    #[test]
    fn resource_type_get_instance_is_out_of_range_safe() {
        assert_eq!(ResourceType::get_instance(-1), None);
        assert_eq!(ResourceType::get_instance(4), None);
    }

    #[test]
    fn resource_type_time_based_matches_java() {
        assert!(!ResourceType::Material.is_time_based());
        assert!(ResourceType::Work.is_time_based());
        assert!(!ResourceType::Cost.is_time_based());
        assert!(ResourceType::NonLabor.is_time_based());
    }
}
