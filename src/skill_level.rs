//! 对应 Java: `org/mpxj/SkillLevel.java`

/// 资源被分配到角色时的技能等级枚举。
///
/// 对应 Java: `public enum SkillLevel`
///
/// Java 侧该枚举不带 int 值、不实现 `MpxjEnum`，也没有 `getInstance`；
/// Rust 侧保持同样的形态（不实现 [`crate::mpxj_enum::MpxjEnum`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SkillLevel {
    /// 专家（大师）级。
    ///
    /// 对应 Java: `MASTER`
    Master,
    /// 熟练级。
    ///
    /// 对应 Java: `EXPERT`
    Expert,
    /// 有技能级。
    ///
    /// 对应 Java: `SKILLED`
    Skilled,
    /// 精通级。
    ///
    /// 对应 Java: `PROFICIENT`
    Proficient,
    /// 无经验级。
    ///
    /// 对应 Java: `INEXPERIENCED`
    Inexperienced,
}

impl SkillLevel {
    /// 全部取值，按 Java 声明顺序排列。
    ///
    /// 对应 Java: `SkillLevel.values()`
    pub const ALL: [SkillLevel; 5] = [
        SkillLevel::Master,
        SkillLevel::Expert,
        SkillLevel::Skilled,
        SkillLevel::Proficient,
        SkillLevel::Inexperienced,
    ];

    /// 按声明顺序（ordinal）取枚举实例。
    ///
    /// Java 侧无此方法；Rust 侧补充以便与 [`SkillLevel::ALL`] 配套查表，
    /// 对应 Java 中 `values()[value]` 的写法。Java 越界会抛
    /// `ArrayIndexOutOfBoundsException`，Rust 侧按本仓约定返回 `None`。
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
