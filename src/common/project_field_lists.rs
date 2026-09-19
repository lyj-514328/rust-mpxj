//! 对应 Java: `org/mpxj/common/ProjectFieldLists.java`

use crate::project_field::ProjectField;

/// Project 字段按逻辑分组后的集合。
///
/// 对应 Java: `public final class ProjectFieldLists`
///
/// 移植说明：
/// - Java 的 `ProjectField[]` 静态数组落地为定长数组常量 `[ProjectField; N]`，元素与顺序与 Java 完全一致。
/// - 本类只有静态常量、没有实例成员，Rust 侧用零字段结构体承载这些关联常量。
///
/// 未移植的成员：无。
pub struct ProjectFieldLists;

impl ProjectFieldLists {
    /// 对应 Java: `public static final ProjectField[] BASELINE_DATES`
    pub const BASELINE_DATES: [ProjectField; 10] = [
        ProjectField::Baseline1Date,
        ProjectField::Baseline2Date,
        ProjectField::Baseline3Date,
        ProjectField::Baseline4Date,
        ProjectField::Baseline5Date,
        ProjectField::Baseline6Date,
        ProjectField::Baseline7Date,
        ProjectField::Baseline8Date,
        ProjectField::Baseline9Date,
        ProjectField::Baseline10Date,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_type::FieldType;

    #[test]
    fn array_lengths_match_java() {
        assert_eq!(ProjectFieldLists::BASELINE_DATES.len(), 10);
    }

    #[test]
    fn baseline_arrays_match_java() {
        assert_eq!(ProjectFieldLists::BASELINE_DATES[0].name(), "BASELINE1_DATE");
        assert_eq!(ProjectFieldLists::BASELINE_DATES[9].name(), "BASELINE10_DATE");
    }
}
