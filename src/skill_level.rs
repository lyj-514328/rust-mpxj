//! Corresponds to Java: `org/mpxj/SkillLevel.java`

/// Enum for the skill level of a resource assigned to a role.
///
/// Corresponds to Java: `public enum SkillLevel`
///
/// The Java enum has no int value, does not implement `MpxjEnum`, and has no `getInstance`;
/// the Rust side keeps the same shape (it does not implement [`crate::mpxj_enum::MpxjEnum`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SkillLevel {
    /// Master level.
    ///
    /// Corresponds to Java: `MASTER`
    Master,
    /// Expert level.
    ///
    /// Corresponds to Java: `EXPERT`
    Expert,
    /// Skilled level.
    ///
    /// Corresponds to Java: `SKILLED`
    Skilled,
    /// Proficient level.
    ///
    /// Corresponds to Java: `PROFICIENT`
    Proficient,
    /// Inexperienced level.
    ///
    /// Corresponds to Java: `INEXPERIENCED`
    Inexperienced,
}

impl SkillLevel {
    /// All values, in Java declaration order.
    ///
    /// Corresponds to Java: `SkillLevel.values()`
    pub const ALL: [SkillLevel; 5] = [
        SkillLevel::Master,
        SkillLevel::Expert,
        SkillLevel::Skilled,
        SkillLevel::Proficient,
        SkillLevel::Inexperienced,
    ];

    /// Returns the enum instance by declaration order (ordinal).
    ///
    /// No corresponding method in Java; added on the Rust side to pair with [`SkillLevel::ALL`]
    /// for lookups, corresponding to Java's `values()[value]` pattern. Java throws
    /// `ArrayIndexOutOfBoundsException` out of bounds, while the Rust side returns `None` per this repo's convention.
    pub fn get_instance(value: i32) -> Option<SkillLevel> {
        usize::try_from(value)
            .ok()
            .and_then(|index| SkillLevel::ALL.get(index))
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_level_round_trips() {
        for (index, item) in SkillLevel::ALL.iter().enumerate() {
            assert_eq!(SkillLevel::get_instance(index as i32), Some(*item));
        }
    }

    #[test]
    fn skill_level_all_matches_java_declaration_order() {
        assert_eq!(
            SkillLevel::ALL,
            [
                SkillLevel::Master,
                SkillLevel::Expert,
                SkillLevel::Skilled,
                SkillLevel::Proficient,
                SkillLevel::Inexperienced
            ]
        );
    }

    #[test]
    fn skill_level_get_instance_is_out_of_range_safe() {
        assert_eq!(SkillLevel::get_instance(-1), None);
        assert_eq!(SkillLevel::get_instance(5), None);
    }
}
