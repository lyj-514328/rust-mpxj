//! Corresponds to Java: `org/mpxj/common/ProjectFieldLists.java`

use crate::project_field::ProjectField;

/// Collections of Project fields grouped logically.
///
/// Corresponds to Java: `public final class ProjectFieldLists`
///
/// Porting notes:
/// - Each Java `ProjectField[]` static array is ported as a fixed-size array constant `[ProjectField; N]`, with elements and order exactly matching Java.
/// - This class has only static constants and no instance members; the Rust side uses a zero-field struct to carry these associated constants.
///
/// Skipped Java members: none.
pub struct ProjectFieldLists;

impl ProjectFieldLists {
    /// Corresponds to Java: `public static final ProjectField[] BASELINE_DATES`
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
