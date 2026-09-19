//! 对应 Java: `org/mpxj/mpx/MPXTaskField.java`
//!
//! MPX 文件中任务字段编号（MPX field ID）与 MPXJ 的
//! [`TaskField`] 之间的双向映射表。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `final class MPXTaskField`（私有构造器） | [`MPXTaskField`] | 空结构体作为命名空间，语义为无状态工具类 |
//! | `public static TaskField getMpxjField(int value)` | [`MPXTaskField::get_mpxj_field`] | Java 越界/未映射返回 `null` → `None` |
//! | `public static int getMpxField(int value)` | [`MPXTaskField::get_mpx_field`] | 入参是 `TaskField` 的 int 值（即枚举 ordinal），越界/未映射返回 `0` |
//! | `public static final int MAX_FIELDS` | [`MPXTaskField::MAX_FIELDS`] | |
//! | `private static final TaskField[] MPX_MPXJ_ARRAY`（静态初始化块逐项赋值） | [`MPXTaskField::get_mpxj_field`] 内的 `match` | 见下 |
//! | `private static final int[] MPXJ_MPX_ARRAY`（静态初始化块按 MPX 数组回填） | [`MPXTaskField::get_mpx_field`] + 模块私有函数 [`mpx_number_of`] 内的 `match` | 见下 |
//! | `public static final List<FieldType> CUSTOM_FIELDS` | [`MPXTaskField::CUSTOM_FIELDS`] | Java 类型是 `List<FieldType>`（元素全是 `TaskField`），Rust 侧用 `&'static [TaskField]` |
//!
//! 几点取舍：
//!
//! - Java 的 `MPX_MPXJ_ARRAY` 是定长 153 的稀疏数组（大量 `null`），Rust 侧用
//!   `match` 表达「MPX 字段号 → 字段」的稀疏映射，语义等价且不需要 153 项字面量。
//!   **顺序**：`match` 臂按 MPX 字段号升序排列，便于与 Java 静态块逐条比对。
//! - Java 的 `MPXJ_MPX_ARRAY` 由 `MPX_MPXJ_ARRAY` 按下标升序回填，因此
//!   **重复项以最后一个为准**：`MPX_MPXJ_ARRAY[151]` 与 `[152]` 都是
//!   `TaskField.RESUME`，故 `RESUME` 的反向值是 `152`（`STOP` 是 `150`）。
//!   本文件在 [`mpx_number_of`] 中显式保持该结果。
//! - `get_mpx_field` 用 [`TaskField::get_instance`] 做边界检查：Java 的
//!   `MPXJ_MPX_ARRAY.length == TaskField.MAX_VALUE`，与 `get_instance` 的合法区间一致，
//!   未命中的下标在 Java 中取数组默认值 `0`，Rust 侧同样返回 `0`。
//! - `CUSTOM_FIELDS`：Java 在静态块中用
//!   `Arrays.stream(MPX_MPXJ_ARRAY).filter(TaskFieldLists.CUSTOM_FIELDS::contains)`
//!   生成，即「出现在 MPX 映射表中的自定义字段」，元素顺序为 MPX 字段号升序。
//!   Rust 侧 `TaskFieldLists`（`src/common/task_field_lists.rs`）由其他模块并行翻译，
//!   为避免跨模块依赖，这里直接给出过滤结果（与 Java 完全一致，共 41 项）。
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//! 未移植的方法：无。

use crate::task_field::TaskField;

/// MPX 任务字段编号与 [`TaskField`] 的映射工具。
///
/// 对应 Java: `final class MPXTaskField`
pub struct MPXTaskField;

impl MPXTaskField {
    /// MPX 文件中任务字段编号的最大个数（可用编号为 `0..MAX_FIELDS`）。
    ///
    /// 对应 Java: `public static final int MAX_FIELDS`
    pub const MAX_FIELDS: usize = 153;

    /// 根据 MPX 文件中读到的字段编号取 [`TaskField`]。
    ///
    /// 对应 Java: `public static TaskField getMpxjField(int value)`
    ///
    /// Java 对越界或未映射的编号返回 `null`，Rust 侧返回 `None`。
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

    /// 取 [`TaskField`] 在 MPX 文件中对应的字段编号。
    ///
    /// 对应 Java: `public static int getMpxField(int value)`
    ///
    /// `value` 是 `TaskField` 的 int 值（`getValue()`，即本 crate 中的
    /// `*field as i32` / [`crate::mpxj_enum::MpxjEnum::value`]）；越界或未在
    /// MPX 映射表中出现的字段返回 `0`（与 Java 的数组默认值一致）。
    pub fn get_mpx_field(value: i32) -> i32 {
        match TaskField::get_instance(value) {
            Some(field) => mpx_number_of(field),
            None => 0,
        }
    }

    /// MPX 映射表中属于「自定义字段」的字段列表，按 MPX 字段号升序。
    ///
    /// 对应 Java: `public static final List<FieldType> CUSTOM_FIELDS`
    ///
    /// 与 Java 的静态初始化块 `Arrays.stream(MPX_MPXJ_ARRAY).filter(
    /// TaskFieldLists.CUSTOM_FIELDS::contains).forEach(CUSTOM_FIELDS::add)` 结果一致。
    pub const CUSTOM_FIELDS: &'static [TaskField] = &[
        // MPX 4..13：TEXT1..TEXT10
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
        // MPX 36..38：COST1..COST3
        TaskField::Cost1,
        TaskField::Cost2,
        TaskField::Cost3,
        // MPX 46..48：DURATION1..DURATION3
        TaskField::Duration1,
        TaskField::Duration2,
        TaskField::Duration3,
        // MPX 60..65：START1..START3、FINISH1..FINISH3
        TaskField::Start1,
        TaskField::Finish1,
        TaskField::Start2,
        TaskField::Finish2,
        TaskField::Start3,
        TaskField::Finish3,
        // MPX 110..119：FLAG1..FLAG10
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
        // MPX 126..129：START4、FINISH4、START5、FINISH5
        TaskField::Start4,
        TaskField::Finish4,
        TaskField::Start5,
        TaskField::Finish5,
        // MPX 140..144：NUMBER1..NUMBER5
        TaskField::Number1,
        TaskField::Number2,
        TaskField::Number3,
        TaskField::Number4,
        TaskField::Number5,
    ];
}

/// 取 [`TaskField`] 对应的 MPX 字段编号，未在 MPX 映射表中出现时返回 `0`。
///
/// 对应 Java: `private static final int[] MPXJ_MPX_ARRAY` 的静态初始化块
/// （按 `MPX_MPXJ_ARRAY` 下标升序回填，重复项以最后一个为准，
/// 因此 `RESUME` 取 `152` 而不是 `151`）。
///
/// 臂的顺序与 Java 静态块中的书写顺序一致（按 Java 枚举常量名字母序）。
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

    /// Java `MPXTaskField` 静态初始化块中 `MPX_MPXJ_ARRAY[...]` 的全部赋值
    /// （共 113 条，其中 `151` 与 `152` 都指向 `TaskField.RESUME`），
    /// 此处按 MPX 字段号升序排列。
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

    /// Java `Arrays.stream(MPX_MPXJ_ARRAY).filter(TaskFieldLists.CUSTOM_FIELDS::contains)`
    /// 的过滤结果。
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

    /// 逐一核对 MPX 字段号 → `TaskField` 的映射（含全部未映射编号都返回 `None`）。
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
                "MPX 字段号 {number}"
            );
        }

        assert_eq!(MPXTaskField::get_mpxj_field(-1), None);
        assert_eq!(
            MPXTaskField::get_mpxj_field(MPXTaskField::MAX_FIELDS as i32),
            None
        );
    }

    /// 逐一核对 `TaskField` → MPX 字段号的反向映射。
    ///
    /// Java 的 `MPXJ_MPX_ARRAY` 由静态块按 `MPX_MPXJ_ARRAY` 下标升序回填，
    /// 重复项以最后一个为准，这里用 `HashMap` 模拟同一过程。
    #[test]
    fn get_mpx_field_matches_java_table() {
        let mut expected: HashMap<TaskField, i32> = HashMap::new();
        for (number, field) in JAVA_TABLE {
            expected.insert(field, number);
        }

        // Java 中 RESUME 同时占据 151 与 152，最终取 152
        assert_eq!(MPXTaskField::get_mpx_field(TaskField::Resume.value()), 152);
        assert_eq!(MPXTaskField::get_mpx_field(TaskField::Stop.value()), 150);

        for field in TaskField::ALL {
            let want = expected.get(&field).copied().unwrap_or(0);
            assert_eq!(MPXTaskField::get_mpx_field(field.value()), want, "{field:?}");
        }

        assert_eq!(MPXTaskField::get_mpx_field(-1), 0);
        assert_eq!(MPXTaskField::get_mpx_field(TaskField::MAX_VALUE as i32), 0);
    }

    /// 反向映射未覆盖的字段必须返回 `0`（Java 数组默认值）。
    #[test]
    fn unmapped_fields_return_zero() {
        // 自定义字段中未出现在 MPX 映射表里的项
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

    /// 两个方向互相往返：有映射的字段必须回到自身，无映射的字段落到 `None`。
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
                "MPX 字段号 {number}"
            );
        }
    }

    /// 自定义字段列表与 Java 的过滤结果一致，且按 MPX 字段号升序。
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
