//! Corresponds to Java: `org/mpxj/mpx/MPXTaskField.java`
//!
//! Bidirectional mapping table between MPX file task field numbers (MPX field ID) and MPXJ's
//! [`TaskField`].
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `final class MPXTaskField` (private constructor) | [`MPXTaskField`] | An empty struct acts as a namespace, semantically a stateless utility class |
//! | `public static TaskField getMpxjField(int value)` | [`MPXTaskField::get_mpxj_field`] | Java returns `null` for out-of-bounds/unmapped -> `None` |
//! | `public static int getMpxField(int value)` | [`MPXTaskField::get_mpx_field`] | The argument is the int value of `TaskField` (that is, the enum ordinal); out-of-bounds/unmapped returns `0` |
//! | `public static final int MAX_FIELDS` | [`MPXTaskField::MAX_FIELDS`] | |
//! | `private static final TaskField[] MPX_MPXJ_ARRAY` (assigned item by item in a static initializer block) | The `match` inside [`MPXTaskField::get_mpxj_field`] | See below |
//! | `private static final int[] MPXJ_MPX_ARRAY` (back-filled from the MPX array in a static initializer block) | The `match` inside [`MPXTaskField::get_mpx_field`] + the module-private function [`mpx_number_of`] | See below |
//! | `public static final List<FieldType> CUSTOM_FIELDS` | [`MPXTaskField::CUSTOM_FIELDS`] | Java's type is `List<FieldType>` (all elements are `TaskField`); the Rust side uses `&'static [TaskField]` |
//!
//! Several trade-offs:
//!
//! - Java's `MPX_MPXJ_ARRAY` is a fixed-length sparse array of 153 entries (many `null`); the Rust side uses a
//!   `match` to express the sparse "MPX field number -> field" mapping, which is semantically equivalent and needs no 153-entry literal.
//!   **Order**: the `match` arms are ordered by ascending MPX field number, to ease item-by-item comparison with the Java static block.
//! - Java's `MPXJ_MPX_ARRAY` is back-filled from `MPX_MPXJ_ARRAY` by ascending index, so
//!   **duplicates are resolved in favor of the last one**: `MPX_MPXJ_ARRAY[151]` and `[152]` are both
//!   `TaskField.RESUME`, so the reverse value of `RESUME` is `152` (`STOP` is `150`).
//!   This file explicitly preserves that result in [`mpx_number_of`].
//! - `get_mpx_field` uses [`TaskField::get_instance`] for bounds checking: Java's
//!   `MPXJ_MPX_ARRAY.length == TaskField.MAX_VALUE`, which matches the valid range of `get_instance`;
//!   an index that does not match takes the array default value `0` in Java, and the Rust side likewise returns `0`.
//! - `CUSTOM_FIELDS`: Java generates it in a static block with
//!   `Arrays.stream(MPX_MPXJ_ARRAY).filter(TaskFieldLists.CUSTOM_FIELDS::contains)`,
//!   that is, "the custom fields appearing in the MPX mapping table", with elements ordered by ascending MPX field number.
//!   The Rust-side `TaskFieldLists` (`src/common/task_field_lists.rs`) is already in place; to avoid a
//!   cross-module dependency, this file still gives the filter result directly (exactly matching Java, 41 entries).
//!
//! Reflection / JDBC / XML (JAXB) / GUI / `javax.*` / `Serializable`: not used by this class, nothing skipped.
//! Skipped Java methods: none.

use crate::task_field::TaskField;

/// Mapping utility between MPX task field numbers and [`TaskField`].
///
/// Corresponds to Java: `final class MPXTaskField`
pub struct MPXTaskField;

impl MPXTaskField {
    /// Maximum number of task field numbers in an MPX file (usable numbers are `0..MAX_FIELDS`).
    ///
    /// Corresponds to Java: `public static final int MAX_FIELDS`
    pub const MAX_FIELDS: usize = 153;

    /// Return the [`TaskField`] for a field number read from an MPX file.
    ///
    /// Corresponds to Java: `public static TaskField getMpxjField(int value)`
    ///
    /// Java returns `null` for an out-of-bounds or unmapped number; the Rust side returns `None`.
    pub fn get_mpxj_field(value: i32) -> Option<TaskField> {
        match value {
            1 => Some(TaskField::Name),
            2 => Some(TaskField::Wbs),
            3 => Some(TaskField::OutlineLevel),
            4 => Some(TaskField::Text1),
            5 => Some(TaskField::Text2),
            6 => Some(TaskField::Text3),
            7 => Some(TaskField::Text4),
            8 => Some(TaskField::Text5),
            9 => Some(TaskField::Text6),
            10 => Some(TaskField::Text7),
            11 => Some(TaskField::Text8),
            12 => Some(TaskField::Text9),
            13 => Some(TaskField::Text10),
            14 => Some(TaskField::Notes),
            15 => Some(TaskField::Contact),
            16 => Some(TaskField::ResourceGroup),
            20 => Some(TaskField::Work),
            21 => Some(TaskField::BaselineWork),
            22 => Some(TaskField::ActualWork),
            23 => Some(TaskField::RemainingWork),
            24 => Some(TaskField::WorkVariance),
            25 => Some(TaskField::PercentWorkComplete),
            30 => Some(TaskField::Cost),
            31 => Some(TaskField::BaselineCost),
            32 => Some(TaskField::ActualCost),
            33 => Some(TaskField::RemainingCost),
            34 => Some(TaskField::CostVariance),
            35 => Some(TaskField::FixedCost),
            36 => Some(TaskField::Cost1),
            37 => Some(TaskField::Cost2),
            38 => Some(TaskField::Cost3),
            40 => Some(TaskField::Duration),
            41 => Some(TaskField::BaselineDuration),
            42 => Some(TaskField::ActualDuration),
            43 => Some(TaskField::RemainingDuration),
            44 => Some(TaskField::PercentComplete),
            45 => Some(TaskField::DurationVariance),
            46 => Some(TaskField::Duration1),
            47 => Some(TaskField::Duration2),
            48 => Some(TaskField::Duration3),
            50 => Some(TaskField::Start),
            51 => Some(TaskField::Finish),
            52 => Some(TaskField::EarlyStart),
            53 => Some(TaskField::EarlyFinish),
            54 => Some(TaskField::LateStart),
            55 => Some(TaskField::LateFinish),
            56 => Some(TaskField::BaselineStart),
            57 => Some(TaskField::BaselineFinish),
            58 => Some(TaskField::ActualStart),
            59 => Some(TaskField::ActualFinish),
            60 => Some(TaskField::Start1),
            61 => Some(TaskField::Finish1),
            62 => Some(TaskField::Start2),
            63 => Some(TaskField::Finish2),
            64 => Some(TaskField::Start3),
            65 => Some(TaskField::Finish3),
            66 => Some(TaskField::StartVariance),
            67 => Some(TaskField::FinishVariance),
            68 => Some(TaskField::ConstraintDate),
            70 => Some(TaskField::Predecessors),
            71 => Some(TaskField::Successors),
            72 => Some(TaskField::ResourceNames),
            73 => Some(TaskField::ResourceInitials),
            74 => Some(TaskField::UniqueIdPredecessors),
            75 => Some(TaskField::UniqueIdSuccessors),
            80 => Some(TaskField::Type),
            81 => Some(TaskField::Milestone),
            82 => Some(TaskField::Critical),
            83 => Some(TaskField::Marked),
            84 => Some(TaskField::Rollup),
            85 => Some(TaskField::Bcws),
            86 => Some(TaskField::Bcwp),
            87 => Some(TaskField::Sv),
            88 => Some(TaskField::Cv),
            90 => Some(TaskField::Id),
            91 => Some(TaskField::ConstraintType),
            92 => Some(TaskField::LevelingDelay),
            93 => Some(TaskField::FreeSlack),
            94 => Some(TaskField::TotalSlack),
            95 => Some(TaskField::Priority),
            96 => Some(TaskField::SubprojectFile),
            97 => Some(TaskField::Project),
            98 => Some(TaskField::UniqueId),
            99 => Some(TaskField::OutlineNumber),
            110 => Some(TaskField::Flag1),
            111 => Some(TaskField::Flag2),
            112 => Some(TaskField::Flag3),
            113 => Some(TaskField::Flag4),
            114 => Some(TaskField::Flag5),
            115 => Some(TaskField::Flag6),
            116 => Some(TaskField::Flag7),
            117 => Some(TaskField::Flag8),
            118 => Some(TaskField::Flag9),
            119 => Some(TaskField::Flag10),
            120 => Some(TaskField::Summary),
            121 => Some(TaskField::Objects),
            122 => Some(TaskField::LinkedFields),
            123 => Some(TaskField::HideBar),
            125 => Some(TaskField::Created),
            126 => Some(TaskField::Start4),
            127 => Some(TaskField::Finish4),
            128 => Some(TaskField::Start5),
            129 => Some(TaskField::Finish5),
            135 => Some(TaskField::Confirmed),
            136 => Some(TaskField::UpdateNeeded),
            140 => Some(TaskField::Number1),
            141 => Some(TaskField::Number2),
            142 => Some(TaskField::Number3),
            143 => Some(TaskField::Number4),
            144 => Some(TaskField::Number5),
            150 => Some(TaskField::Stop),
            151 => Some(TaskField::Resume),
            152 => Some(TaskField::Resume),
            _ => None,
        }
    }

    /// Return the field number corresponding to a [`TaskField`] in an MPX file.
    ///
    /// Corresponds to Java: `public static int getMpxField(int value)`
    ///
    /// `value` is the int value of `TaskField` (`getValue()`, that is, in this crate
    /// `*field as i32` / [`crate::mpxj_enum::MpxjEnum::value`]); a field that is out of bounds or absent from the
    /// MPX mapping table returns `0` (consistent with Java's array default).
    pub fn get_mpx_field(value: i32) -> i32 {
        match TaskField::get_instance(value) {
            Some(field) => mpx_number_of(field),
            None => 0,
        }
    }

    /// List of fields in the MPX mapping table that are "custom fields", ordered by ascending MPX field number.
    ///
    /// Corresponds to Java: `public static final List<FieldType> CUSTOM_FIELDS`
    ///
    /// Matches Java's static initializer block `Arrays.stream(MPX_MPXJ_ARRAY).filter(
    /// TaskFieldLists.CUSTOM_FIELDS::contains).forEach(CUSTOM_FIELDS::add)`.
    pub const CUSTOM_FIELDS: &'static [TaskField] = &[
        // MPX 4..13: TEXT1..TEXT10
        TaskField::Text1,
        TaskField::Text2,
        TaskField::Text3,
        TaskField::Text4,
        TaskField::Text5,
        TaskField::Text6,
        TaskField::Text7,
        TaskField::Text8,
        TaskField::Text9,
        TaskField::Text10,
        // MPX 36..38: COST1..COST3
        TaskField::Cost1,
        TaskField::Cost2,
        TaskField::Cost3,
        // MPX 46..48: DURATION1..DURATION3
        TaskField::Duration1,
        TaskField::Duration2,
        TaskField::Duration3,
        // MPX 60..65: START1..START3, FINISH1..FINISH3
        TaskField::Start1,
        TaskField::Finish1,
        TaskField::Start2,
        TaskField::Finish2,
        TaskField::Start3,
        TaskField::Finish3,
        // MPX 110..119: FLAG1..FLAG10
        TaskField::Flag1,
        TaskField::Flag2,
        TaskField::Flag3,
        TaskField::Flag4,
        TaskField::Flag5,
        TaskField::Flag6,
        TaskField::Flag7,
        TaskField::Flag8,
        TaskField::Flag9,
        TaskField::Flag10,
        // MPX 126..129: START4, FINISH4, START5, FINISH5
        TaskField::Start4,
        TaskField::Finish4,
        TaskField::Start5,
        TaskField::Finish5,
        // MPX 140..144: NUMBER1..NUMBER5
        TaskField::Number1,
        TaskField::Number2,
        TaskField::Number3,
        TaskField::Number4,
        TaskField::Number5,
    ];
}

/// Return the MPX field number for a [`TaskField`], or `0` when it does not appear in the MPX mapping table.
///
/// Corresponds to Java: the static initializer block of `private static final int[] MPXJ_MPX_ARRAY`
/// (back-filled by ascending `MPX_MPXJ_ARRAY` index, with duplicates resolved in favor of the last one,
/// so `RESUME` takes `152` rather than `151`).
///
/// Arm order matches the writing order in Java's static block (alphabetical by Java enum constant name).
fn mpx_number_of(field: TaskField) -> i32 {
    match field {
        TaskField::ActualCost => 32,
        TaskField::ActualDuration => 42,
        TaskField::ActualFinish => 59,
        TaskField::ActualStart => 58,
        TaskField::ActualWork => 22,
        TaskField::BaselineCost => 31,
        TaskField::BaselineDuration => 41,
        TaskField::BaselineFinish => 57,
        TaskField::BaselineStart => 56,
        TaskField::BaselineWork => 21,
        TaskField::Bcwp => 86,
        TaskField::Bcws => 85,
        TaskField::Confirmed => 135,
        TaskField::ConstraintDate => 68,
        TaskField::ConstraintType => 91,
        TaskField::Contact => 15,
        TaskField::Cost => 30,
        TaskField::CostVariance => 34,
        TaskField::Cost1 => 36,
        TaskField::Cost2 => 37,
        TaskField::Cost3 => 38,
        TaskField::Created => 125,
        TaskField::Critical => 82,
        TaskField::Cv => 88,
        TaskField::LevelingDelay => 92,
        TaskField::Duration => 40,
        TaskField::DurationVariance => 45,
        TaskField::Duration1 => 46,
        TaskField::Duration2 => 47,
        TaskField::Duration3 => 48,
        TaskField::EarlyFinish => 53,
        TaskField::EarlyStart => 52,
        TaskField::Finish => 51,
        TaskField::FinishVariance => 67,
        TaskField::Finish1 => 61,
        TaskField::Finish2 => 63,
        TaskField::Finish3 => 65,
        TaskField::Finish4 => 127,
        TaskField::Finish5 => 129,
        TaskField::FixedCost => 35,
        TaskField::Flag1 => 110,
        TaskField::Flag10 => 119,
        TaskField::Flag2 => 111,
        TaskField::Flag3 => 112,
        TaskField::Flag4 => 113,
        TaskField::Flag5 => 114,
        TaskField::Flag6 => 115,
        TaskField::Flag7 => 116,
        TaskField::Flag8 => 117,
        TaskField::Flag9 => 118,
        TaskField::FreeSlack => 93,
        TaskField::HideBar => 123,
        TaskField::Id => 90,
        TaskField::LateFinish => 55,
        TaskField::LateStart => 54,
        TaskField::LinkedFields => 122,
        TaskField::Marked => 83,
        TaskField::Milestone => 81,
        TaskField::Name => 1,
        TaskField::Notes => 14,
        TaskField::Number1 => 140,
        TaskField::Number2 => 141,
        TaskField::Number3 => 142,
        TaskField::Number4 => 143,
        TaskField::Number5 => 144,
        TaskField::Objects => 121,
        TaskField::OutlineLevel => 3,
        TaskField::OutlineNumber => 99,
        TaskField::PercentComplete => 44,
        TaskField::PercentWorkComplete => 25,
        TaskField::Predecessors => 70,
        TaskField::Priority => 95,
        TaskField::Project => 97,
        TaskField::RemainingCost => 33,
        TaskField::RemainingDuration => 43,
        TaskField::RemainingWork => 23,
        TaskField::ResourceGroup => 16,
        TaskField::ResourceInitials => 73,
        TaskField::ResourceNames => 72,
        TaskField::Resume => 152,
        TaskField::Rollup => 84,
        TaskField::Start => 50,
        TaskField::StartVariance => 66,
        TaskField::Start1 => 60,
        TaskField::Start2 => 62,
        TaskField::Start3 => 64,
        TaskField::Start4 => 126,
        TaskField::Start5 => 128,
        TaskField::Stop => 150,
        TaskField::SubprojectFile => 96,
        TaskField::Successors => 71,
        TaskField::Summary => 120,
        TaskField::Sv => 87,
        TaskField::Text1 => 4,
        TaskField::Text10 => 13,
        TaskField::Text2 => 5,
        TaskField::Text3 => 6,
        TaskField::Text4 => 7,
        TaskField::Text5 => 8,
        TaskField::Text6 => 9,
        TaskField::Text7 => 10,
        TaskField::Text8 => 11,
        TaskField::Text9 => 12,
        TaskField::TotalSlack => 94,
        TaskField::Type => 80,
        TaskField::UniqueId => 98,
        TaskField::UniqueIdPredecessors => 74,
        TaskField::UniqueIdSuccessors => 75,
        TaskField::UpdateNeeded => 136,
        TaskField::Wbs => 2,
        TaskField::Work => 20,
        TaskField::WorkVariance => 24,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpxj_enum::MpxjEnum;
    use std::collections::HashMap;

    /// All assignments to `MPX_MPXJ_ARRAY[...]` in Java's `MPXTaskField` static initializer block
    /// (113 entries, where both `151` and `152` point to `TaskField.RESUME`),
    /// arranged here by ascending MPX field number.
    const JAVA_TABLE: [(i32, TaskField); 113] = [
        (1, TaskField::Name),
        (2, TaskField::Wbs),
        (3, TaskField::OutlineLevel),
        (4, TaskField::Text1),
        (5, TaskField::Text2),
        (6, TaskField::Text3),
        (7, TaskField::Text4),
        (8, TaskField::Text5),
        (9, TaskField::Text6),
        (10, TaskField::Text7),
        (11, TaskField::Text8),
        (12, TaskField::Text9),
        (13, TaskField::Text10),
        (14, TaskField::Notes),
        (15, TaskField::Contact),
        (16, TaskField::ResourceGroup),
        (20, TaskField::Work),
        (21, TaskField::BaselineWork),
        (22, TaskField::ActualWork),
        (23, TaskField::RemainingWork),
        (24, TaskField::WorkVariance),
        (25, TaskField::PercentWorkComplete),
        (30, TaskField::Cost),
        (31, TaskField::BaselineCost),
        (32, TaskField::ActualCost),
        (33, TaskField::RemainingCost),
        (34, TaskField::CostVariance),
        (35, TaskField::FixedCost),
        (36, TaskField::Cost1),
        (37, TaskField::Cost2),
        (38, TaskField::Cost3),
        (40, TaskField::Duration),
        (41, TaskField::BaselineDuration),
        (42, TaskField::ActualDuration),
        (43, TaskField::RemainingDuration),
        (44, TaskField::PercentComplete),
        (45, TaskField::DurationVariance),
        (46, TaskField::Duration1),
        (47, TaskField::Duration2),
        (48, TaskField::Duration3),
        (50, TaskField::Start),
        (51, TaskField::Finish),
        (52, TaskField::EarlyStart),
        (53, TaskField::EarlyFinish),
        (54, TaskField::LateStart),
        (55, TaskField::LateFinish),
        (56, TaskField::BaselineStart),
        (57, TaskField::BaselineFinish),
        (58, TaskField::ActualStart),
        (59, TaskField::ActualFinish),
        (60, TaskField::Start1),
        (61, TaskField::Finish1),
        (62, TaskField::Start2),
        (63, TaskField::Finish2),
        (64, TaskField::Start3),
        (65, TaskField::Finish3),
        (66, TaskField::StartVariance),
        (67, TaskField::FinishVariance),
        (68, TaskField::ConstraintDate),
        (70, TaskField::Predecessors),
        (71, TaskField::Successors),
        (72, TaskField::ResourceNames),
        (73, TaskField::ResourceInitials),
        (74, TaskField::UniqueIdPredecessors),
        (75, TaskField::UniqueIdSuccessors),
        (80, TaskField::Type),
        (81, TaskField::Milestone),
        (82, TaskField::Critical),
        (83, TaskField::Marked),
        (84, TaskField::Rollup),
        (85, TaskField::Bcws),
        (86, TaskField::Bcwp),
        (87, TaskField::Sv),
        (88, TaskField::Cv),
        (90, TaskField::Id),
        (91, TaskField::ConstraintType),
        (92, TaskField::LevelingDelay),
        (93, TaskField::FreeSlack),
        (94, TaskField::TotalSlack),
        (95, TaskField::Priority),
        (96, TaskField::SubprojectFile),
        (97, TaskField::Project),
        (98, TaskField::UniqueId),
        (99, TaskField::OutlineNumber),
        (110, TaskField::Flag1),
        (111, TaskField::Flag2),
        (112, TaskField::Flag3),
        (113, TaskField::Flag4),
        (114, TaskField::Flag5),
        (115, TaskField::Flag6),
        (116, TaskField::Flag7),
        (117, TaskField::Flag8),
        (118, TaskField::Flag9),
        (119, TaskField::Flag10),
        (120, TaskField::Summary),
        (121, TaskField::Objects),
        (122, TaskField::LinkedFields),
        (123, TaskField::HideBar),
        (125, TaskField::Created),
        (126, TaskField::Start4),
        (127, TaskField::Finish4),
        (128, TaskField::Start5),
        (129, TaskField::Finish5),
        (135, TaskField::Confirmed),
        (136, TaskField::UpdateNeeded),
        (140, TaskField::Number1),
        (141, TaskField::Number2),
        (142, TaskField::Number3),
        (143, TaskField::Number4),
        (144, TaskField::Number5),
        (150, TaskField::Stop),
        (151, TaskField::Resume),
        (152, TaskField::Resume),
    ];

    /// Filter result of Java's `Arrays.stream(MPX_MPXJ_ARRAY).filter(
    /// TaskFieldLists.CUSTOM_FIELDS::contains)`.
    const JAVA_CUSTOM_FIELDS: [TaskField; 41] = [
        TaskField::Text1,
        TaskField::Text2,
        TaskField::Text3,
        TaskField::Text4,
        TaskField::Text5,
        TaskField::Text6,
        TaskField::Text7,
        TaskField::Text8,
        TaskField::Text9,
        TaskField::Text10,
        TaskField::Cost1,
        TaskField::Cost2,
        TaskField::Cost3,
        TaskField::Duration1,
        TaskField::Duration2,
        TaskField::Duration3,
        TaskField::Start1,
        TaskField::Finish1,
        TaskField::Start2,
        TaskField::Finish2,
        TaskField::Start3,
        TaskField::Finish3,
        TaskField::Flag1,
        TaskField::Flag2,
        TaskField::Flag3,
        TaskField::Flag4,
        TaskField::Flag5,
        TaskField::Flag6,
        TaskField::Flag7,
        TaskField::Flag8,
        TaskField::Flag9,
        TaskField::Flag10,
        TaskField::Start4,
        TaskField::Finish4,
        TaskField::Start5,
        TaskField::Finish5,
        TaskField::Number1,
        TaskField::Number2,
        TaskField::Number3,
        TaskField::Number4,
        TaskField::Number5,
    ];

    /// Verify the MPX field number -> `TaskField` mapping item by item (including that every unmapped number returns `None`).
    #[test]
    fn get_mpxj_field_matches_java_table() {
        assert_eq!(MPXTaskField::MAX_FIELDS, 153);

        let mut expected = vec![None; MPXTaskField::MAX_FIELDS];
        for (number, field) in JAVA_TABLE {
            expected[number as usize] = Some(field);
        }

        for number in 0..MPXTaskField::MAX_FIELDS as i32 {
            assert_eq!(
                MPXTaskField::get_mpxj_field(number),
                expected[number as usize],
                "MPX field number {number}"
            );
        }

        assert_eq!(MPXTaskField::get_mpxj_field(-1), None);
        assert_eq!(
            MPXTaskField::get_mpxj_field(MPXTaskField::MAX_FIELDS as i32),
            None
        );
    }

    /// Verify the reverse mapping `TaskField` -> MPX field number item by item.
    ///
    /// Java's `MPXJ_MPX_ARRAY` is back-filled by the static block in ascending `MPX_MPXJ_ARRAY` index order,
    /// with duplicates resolved in favor of the last one; a `HashMap` is used here to simulate the same process.
    #[test]
    fn get_mpx_field_matches_java_table() {
        let mut expected: HashMap<TaskField, i32> = HashMap::new();
        for (number, field) in JAVA_TABLE {
            expected.insert(field, number);
        }

        // In Java RESUME occupies both 151 and 152, ultimately taking 152
        assert_eq!(MPXTaskField::get_mpx_field(TaskField::Resume.value()), 152);
        assert_eq!(MPXTaskField::get_mpx_field(TaskField::Stop.value()), 150);

        for field in TaskField::ALL {
            let want = expected.get(&field).copied().unwrap_or(0);
            assert_eq!(MPXTaskField::get_mpx_field(field.value()), want, "{field:?}");
        }

        assert_eq!(MPXTaskField::get_mpx_field(-1), 0);
        assert_eq!(MPXTaskField::get_mpx_field(TaskField::MAX_VALUE as i32), 0);
    }

    /// Fields not covered by the reverse mapping must return `0` (Java's array default).
    #[test]
    fn unmapped_fields_return_zero() {
        // Custom fields that do not appear in the MPX mapping table
        for field in [
            TaskField::Text11,
            TaskField::Text30,
            TaskField::Cost4,
            TaskField::Cost10,
            TaskField::Flag11,
            TaskField::Flag20,
            TaskField::Number6,
            TaskField::Number20,
            TaskField::Start6,
            TaskField::Finish10,
            TaskField::Duration4,
            TaskField::Duration10,
            TaskField::Date1,
            TaskField::OutlineCode1,
            TaskField::Baseline1Cost,
            TaskField::EnterpriseText1,
        ] {
            assert_eq!(MPXTaskField::get_mpx_field(field.value()), 0, "{field:?}");
        }
    }

    /// Round trip in both directions: mapped fields must return to themselves, and unmapped fields fall to `None`.
    #[test]
    fn mappings_round_trip() {
        for field in TaskField::ALL {
            let number = MPXTaskField::get_mpx_field(field.value());
            let back = MPXTaskField::get_mpxj_field(number);
            if number == 0 {
                assert_eq!(back, None, "{field:?}");
            } else {
                assert_eq!(back, Some(field), "{field:?}");
            }
        }

        for (number, field) in JAVA_TABLE {
            assert_eq!(MPXTaskField::get_mpxj_field(number), Some(field));
            assert_eq!(
                MPXTaskField::get_mpxj_field(MPXTaskField::get_mpx_field(field.value())),
                Some(field),
                "MPX field number {number}"
            );
        }
    }

    /// The custom field list matches Java's filter result and is ordered by ascending MPX field number.
    #[test]
    fn custom_fields_match_java() {
        assert_eq!(MPXTaskField::CUSTOM_FIELDS, &JAVA_CUSTOM_FIELDS);

        let numbers: Vec<i32> = MPXTaskField::CUSTOM_FIELDS
            .iter()
            .map(|field| MPXTaskField::get_mpx_field(field.value()))
            .collect();
        assert!(numbers.iter().all(|number| *number > 0));
        assert!(numbers.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
