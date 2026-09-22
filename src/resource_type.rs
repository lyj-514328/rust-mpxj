//! Corresponds to Java: `org/mpxj/ResourceType.java`

/// Enum for resource types.
///
/// Corresponds to Java: `public enum ResourceType`
///
/// The Java enum has no int value, does not implement `MpxjEnum`, and has no `getInstance`;
/// the Rust side keeps the same shape (it does not implement [`crate::mpxj_enum::MpxjEnum`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ResourceType {
    /// Material resource.
    ///
    /// Corresponds to Java: `MATERIAL(false)`
    Material,
    /// Work (labor) resource.
    ///
    /// Corresponds to Java: `WORK(true)`
    Work,
    /// Cost resource.
    ///
    /// Corresponds to Java: `COST(false)`
    Cost,
    /// Non-labor resource.
    ///
    /// Corresponds to Java: `NON_LABOR(true)`
    NonLabor,
}

impl ResourceType {
    /// All values, in Java declaration order.
    ///
    /// Corresponds to Java: `ResourceType.values()`
    pub const ALL: [ResourceType; 4] = [
        ResourceType::Material,
        ResourceType::Work,
        ResourceType::Cost,
        ResourceType::NonLabor,
    ];

    /// Returns the enum instance by declaration order (ordinal).
    ///
    /// No corresponding method in Java; added on the Rust side to pair with [`ResourceType::ALL`]
    /// for lookups, corresponding to Java's `values()[value]` pattern. Java throws
    /// `ArrayIndexOutOfBoundsException` out of bounds, while the Rust side returns `None` per this repo's convention.
    pub fn get_instance(value: i32) -> Option<ResourceType> {
        usize::try_from(value)
            .ok()
            .and_then(|index| ResourceType::ALL.get(index))
            .copied()
    }

    /// Whether this resource type is time based.
    ///
    /// Corresponds to Java: `public boolean isTimeBased()`
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
