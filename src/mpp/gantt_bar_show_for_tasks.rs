//! 对应 Java: `org/mpxj/mpp/GanttBarShowForTasks.java`
//!
//! 定义「甘特图条形在何种任务上显示」的判据枚举。
//!
//! Java 注释说明：`value` 的取值刻意设计成能区分「正常型」与「取反型」——
//! 非负数（`< 64`）为正常型，`>= 64` 为取反型；`value & 0x64` 对正常型为 0，
//! 对取反型非 0；`value & 0x63` 可把取反型转回正常型，转换后的值可用
//! `getInstance` 查到对应枚举。
//!
//! # 移植取舍
//!
//! - Java 用 `EnumHelper.createTypeArray(GanttBarShowForTasks.class, 26)` 反射建表：
//!   数组长度 = 常量个数（98） + 26 = **124**，下标为 `m_value`，未覆盖的下标为 `null`
//!   （例如 68 / 72 / 73 / 115..123 —— 前三个是 MSP 未使用的「Not Not Xxx」取值）。
//!   Rust 不用反射，改为 [`GanttBarShowForTasks::get_instance`] 的 `match`；
//!   因为 Java 的 `getInstance` **可能返回 `null`**，Rust 侧返回 `Option<Self>`，
//!   并保留 124 这一有效区间（`>= 124` 与负值一样回落到 `NORMAL`）。
//! - Java 的静态块把下标 70 额外映射为 `NOTFINISHED`（注释说该值理论上表示
//!   「Not Not Finished」，不应出现，但在真实文件里出现过），Rust 侧同样保留该映射。
//! - `EnumHelper` / 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：
//!   除 `createTypeArray` 外本类未使用（`createTypeArray` 见上）。
//!
//! 未移植的 Java 成员：无。

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// 甘特图条形的显示判据。
///
/// 对应 Java: `public enum GanttBarShowForTasks implements MpxjEnum`
///
/// 声明顺序与 Java 一致，判别值即 Java 的 `m_value`（第二段取反型为 64..114）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GanttBarShowForTasks {
    /// 对应 Java: `NORMAL(0, "Normal")`
    Normal = 0,
    /// 对应 Java: `MILESTONE(1, "Milestone")`
    Milestone = 1,
    /// 对应 Java: `SUMMARY(2, "Summary")`
    Summary = 2,
    /// 对应 Java: `CRITICAL(3, "Critical")`
    Critical = 3,
    /// 对应 Java: `NONCRITICAL(4, "Noncritical")`
    Noncritical = 4,
    /// 对应 Java: `MARKED(5, "Marked")`
    Marked = 5,
    /// 对应 Java: `FINISHED(6, "Finished")`
    Finished = 6,
    /// 对应 Java: `INPROGRESS(7, "In Progress")`
    InProgress = 7,
    /// 对应 Java: `NOTFINISHED(8, "Not Finished")`
    NotFinished = 8,
    /// 对应 Java: `NOTSTARTED(9, "Not Started")`
    NotStarted = 9,
    /// 对应 Java: `STARTEDLATE(10, "Started Late")`
    StartedLate = 10,
    /// 对应 Java: `FINISHEDLATE(11, "Finished Late")`
    FinishedLate = 11,
    /// 对应 Java: `STARTEDEARLY(12, "Started Early")`
    StartedEarly = 12,
    /// 对应 Java: `FINISHEDEARLY(13, "Finished Early")`
    FinishedEarly = 13,
    /// 对应 Java: `STARTEDONTIME(14, "Started On Time")`
    StartedOnTime = 14,
    /// 对应 Java: `FINISHEDONTIME(15, "Finished On Time")`
    FinishedOnTime = 15,
    /// 对应 Java: `FLAG1(16, "Flag1")`
    Flag1 = 16,
    /// 对应 Java: `FLAG2(17, "Flag2")`
    Flag2 = 17,
    /// 对应 Java: `FLAG3(18, "Flag3")`
    Flag3 = 18,
    /// 对应 Java: `FLAG4(19, "Flag4")`
    Flag4 = 19,
    /// 对应 Java: `FLAG5(20, "Flag5")`
    Flag5 = 20,
    /// 对应 Java: `FLAG6(21, "Flag6")`
    Flag6 = 21,
    /// 对应 Java: `FLAG7(22, "Flag7")`
    Flag7 = 22,
    /// 对应 Java: `FLAG8(23, "Flag8")`
    Flag8 = 23,
    /// 对应 Java: `FLAG9(24, "Flag9")`
    Flag9 = 24,
    /// 对应 Java: `FLAG10(25, "Flag10")`
    Flag10 = 25,
    /// 对应 Java: `ROLLEDUP(26, "Rolled Up")`
    RolledUp = 26,
    /// 对应 Java: `PROJECTSUMMARY(27, "Project Summary")`
    ProjectSummary = 27,
    /// 对应 Java: `SPLIT(28, "Split")`
    Split = 28,
    /// 对应 Java: `EXTERNAL(29, "External Tasks")`
    External = 29,
    /// 对应 Java: `FLAG11(30, "Flag11")`
    Flag11 = 30,
    /// 对应 Java: `FLAG12(31, "Flag12")`
    Flag12 = 31,
    /// 对应 Java: `FLAG13(32, "Flag13")`
    Flag13 = 32,
    /// 对应 Java: `FLAG14(33, "Flag14")`
    Flag14 = 33,
    /// 对应 Java: `FLAG15(34, "Flag15")`
    Flag15 = 34,
    /// 对应 Java: `FLAG16(35, "Flag16")`
    Flag16 = 35,
    /// 对应 Java: `FLAG17(36, "Flag17")`
    Flag17 = 36,
    /// 对应 Java: `FLAG18(37, "Flag18")`
    Flag18 = 37,
    /// 对应 Java: `FLAG19(38, "Flag19")`
    Flag19 = 38,
    /// 对应 Java: `FLAG20(39, "Flag20")`
    Flag20 = 39,
    /// 对应 Java: `GROUPBYSUMMARY(40, "Group By Summary")`
    GroupBySummary = 40,
    /// 对应 Java: `DELIVERABLE(41, "Deliverable")`
    Deliverable = 41,
    /// 对应 Java: `DEPENDENCY(42, "Dependency")`
    Dependency = 42,
    /// 对应 Java: `ACTIVE(43, "Active")`
    Active = 43,
    /// 对应 Java: `MANUALLYSCHEDULED(44, "Manually Scheduled")`
    ManuallyScheduled = 44,
    /// 对应 Java: `WARNING(45, "Warning")`
    Warning = 45,
    /// 对应 Java: `PLACEHOLDERSTART(46, "Placeholder (Start)")`
    PlaceholderStart = 46,
    /// 对应 Java: `PLACEHOLDERFINISH(47, "Placeholder (Finish)")`
    PlaceholderFinish = 47,
    /// 对应 Java: `PLACEHOLDERDURATION(48, "Placeholder (Duration)")`
    PlaceholderDuration = 48,
    /// 对应 Java: `PLACEHOLDER(49, "Placeholder")`
    Placeholder = 49,
    /// 对应 Java: `LATE(50, "Late")`
    Late = 50,
    /// 对应 Java: `NOT_NORMAL(64, "Not Normal")`
    NotNormal = 64,
    /// 对应 Java: `NOT_MILESTONE(65, "Not Milestone")`
    NotMilestone = 65,
    /// 对应 Java: `NOT_SUMMARY(66, "Not Summary")`
    NotSummary = 66,
    /// 对应 Java: `NOT_CRITICAL(67, "Not Critical")`
    NotCritical = 67,
    /// 对应 Java: `NOT_MARKED(69, "Not Marked")`（68 为 Java 注释掉的 `NOT_NONCRITICAL`，MSP 不使用）
    NotMarked = 69,
    /// 对应 Java: `NOT_INPROGRESS(71, "Not In Progress")`（70 见类型级说明，映射为 `NOTFINISHED`）
    NotInProgress = 71,
    /// 对应 Java: `NOT_STARTEDLATE(74, "Not Started Late")`（72、73 为 Java 注释掉的取值）
    NotStartedLate = 74,
    /// 对应 Java: `NOT_FINISHEDLATE(75, "Not Finished Late")`
    NotFinishedLate = 75,
    /// 对应 Java: `NOT_STARTEDEARLY(76, "Not Started Early")`
    NotStartedEarly = 76,
    /// 对应 Java: `NOT_FINISHEDEARLY(77, "Not Finished Early")`
    NotFinishedEarly = 77,
    /// 对应 Java: `NOT_STARTEDONTIME(78, "Not Started On Time")`
    NotStartedOnTime = 78,
    /// 对应 Java: `NOT_FINISHEDONTIME(79, "Not Finished On Time")`
    NotFinishedOnTime = 79,
    /// 对应 Java: `NOT_FLAG1(80, "Not Flag1")`
    NotFlag1 = 80,
    /// 对应 Java: `NOT_FLAG2(81, "Not Flag2")`
    NotFlag2 = 81,
    /// 对应 Java: `NOT_FLAG3(82, "Not Flag3")`
    NotFlag3 = 82,
    /// 对应 Java: `NOT_FLAG4(83, "Not Flag4")`
    NotFlag4 = 83,
    /// 对应 Java: `NOT_FLAG5(84, "Not Flag5")`
    NotFlag5 = 84,
    /// 对应 Java: `NOT_FLAG6(85, "Not Flag6")`
    NotFlag6 = 85,
    /// 对应 Java: `NOT_FLAG7(86, "Not Flag7")`
    NotFlag7 = 86,
    /// 对应 Java: `NOT_FLAG8(87, "Not Flag8")`
    NotFlag8 = 87,
    /// 对应 Java: `NOT_FLAG9(88, "Not Flag9")`
    NotFlag9 = 88,
    /// 对应 Java: `NOT_FLAG10(89, "Not Flag10")`
    NotFlag10 = 89,
    /// 对应 Java: `NOT_ROLLEDUP(90, "Not Rolled Up")`
    NotRolledUp = 90,
    /// 对应 Java: `NOT_PROJECTSUMMARY(91, "Not Project Summary")`
    NotProjectSummary = 91,
    /// 对应 Java: `NOT_SPLIT(92, "Not Split")`
    NotSplit = 92,
    /// 对应 Java: `NOT_EXTERNAL(93, "Not External Tasks")`
    NotExternal = 93,
    /// 对应 Java: `NOT_FLAG11(94, "Not Flag11")`
    NotFlag11 = 94,
    /// 对应 Java: `NOT_FLAG12(95, "Not Flag12")`
    NotFlag12 = 95,
    /// 对应 Java: `NOT_FLAG13(96, "Not Flag13")`
    NotFlag13 = 96,
    /// 对应 Java: `NOT_FLAG14(97, "Not Flag14")`
    NotFlag14 = 97,
    /// 对应 Java: `NOT_FLAG15(98, "Not Flag15")`
    NotFlag15 = 98,
    /// 对应 Java: `NOT_FLAG16(99, "Not Flag16")`
    NotFlag16 = 99,
    /// 对应 Java: `NOT_FLAG17(100, "Not Flag17")`
    NotFlag17 = 100,
    /// 对应 Java: `NOT_FLAG18(101, "Not Flag18")`
    NotFlag18 = 101,
    /// 对应 Java: `NOT_FLAG19(102, "Not Flag19")`
    NotFlag19 = 102,
    /// 对应 Java: `NOT_FLAG20(103, "Not Flag20")`
    NotFlag20 = 103,
    /// 对应 Java: `NOT_GROUPBYSUMMARY(104, "Not Group By Summary")`
    NotGroupBySummary = 104,
    /// 对应 Java: `NOT_DELIVERABLE(105, "Not Deliverable")`
    NotDeliverable = 105,
    /// 对应 Java: `NOT_DEPENDENCY(106, "Not Dependency")`
    NotDependency = 106,
    /// 对应 Java: `NOT_ACTIVE(107, "Not Active")`
    NotActive = 107,
    /// 对应 Java: `NOT_MANUALLYSCHEDULED(108, "Not Manually Scheduled")`
    NotManuallyScheduled = 108,
    /// 对应 Java: `NOT_WARNING(109, "Not Warning")`
    NotWarning = 109,
    /// 对应 Java: `NOT_PLACEHOLDERSTART(110, "Not Placeholder (Start)")`
    NotPlaceholderStart = 110,
    /// 对应 Java: `NOT_PLACEHOLDERFINISH(111, "Not Placeholder (Finish)")`
    NotPlaceholderFinish = 111,
    /// 对应 Java: `NOT_PLACEHOLDERDURATION(112, "Placeholder (Duration)")`（Java 的名称漏了 "Not" 前缀，按原样保留）
    NotPlaceholderDuration = 112,
    /// 对应 Java: `NOT_PLACEHOLDER(113, "Not Placeholder")`
    NotPlaceholder = 113,
    /// 对应 Java: `NOT_LATE(114, "Not Late")`
    NotLate = 114,
}

impl GanttBarShowForTasks {
    /// 全部取值，顺序与 Java 声明顺序一致（即 `m_value` 升序）。
    ///
    /// 对应 Java: `private static final GanttBarShowForTasks[] TYPE_VALUES` 中被填充的项。
    pub const ALL: [GanttBarShowForTasks; 98] = [
        GanttBarShowForTasks::Normal,
        GanttBarShowForTasks::Milestone,
        GanttBarShowForTasks::Summary,
        GanttBarShowForTasks::Critical,
        GanttBarShowForTasks::Noncritical,
        GanttBarShowForTasks::Marked,
        GanttBarShowForTasks::Finished,
        GanttBarShowForTasks::InProgress,
        GanttBarShowForTasks::NotFinished,
        GanttBarShowForTasks::NotStarted,
        GanttBarShowForTasks::StartedLate,
        GanttBarShowForTasks::FinishedLate,
        GanttBarShowForTasks::StartedEarly,
        GanttBarShowForTasks::FinishedEarly,
        GanttBarShowForTasks::StartedOnTime,
        GanttBarShowForTasks::FinishedOnTime,
        GanttBarShowForTasks::Flag1,
        GanttBarShowForTasks::Flag2,
        GanttBarShowForTasks::Flag3,
        GanttBarShowForTasks::Flag4,
        GanttBarShowForTasks::Flag5,
        GanttBarShowForTasks::Flag6,
        GanttBarShowForTasks::Flag7,
        GanttBarShowForTasks::Flag8,
        GanttBarShowForTasks::Flag9,
        GanttBarShowForTasks::Flag10,
        GanttBarShowForTasks::RolledUp,
        GanttBarShowForTasks::ProjectSummary,
        GanttBarShowForTasks::Split,
        GanttBarShowForTasks::External,
        GanttBarShowForTasks::Flag11,
        GanttBarShowForTasks::Flag12,
        GanttBarShowForTasks::Flag13,
        GanttBarShowForTasks::Flag14,
        GanttBarShowForTasks::Flag15,
        GanttBarShowForTasks::Flag16,
        GanttBarShowForTasks::Flag17,
        GanttBarShowForTasks::Flag18,
        GanttBarShowForTasks::Flag19,
        GanttBarShowForTasks::Flag20,
        GanttBarShowForTasks::GroupBySummary,
        GanttBarShowForTasks::Deliverable,
        GanttBarShowForTasks::Dependency,
        GanttBarShowForTasks::Active,
        GanttBarShowForTasks::ManuallyScheduled,
        GanttBarShowForTasks::Warning,
        GanttBarShowForTasks::PlaceholderStart,
        GanttBarShowForTasks::PlaceholderFinish,
        GanttBarShowForTasks::PlaceholderDuration,
        GanttBarShowForTasks::Placeholder,
        GanttBarShowForTasks::Late,
        GanttBarShowForTasks::NotNormal,
        GanttBarShowForTasks::NotMilestone,
        GanttBarShowForTasks::NotSummary,
        GanttBarShowForTasks::NotCritical,
        GanttBarShowForTasks::NotMarked,
        GanttBarShowForTasks::NotInProgress,
        GanttBarShowForTasks::NotStartedLate,
        GanttBarShowForTasks::NotFinishedLate,
        GanttBarShowForTasks::NotStartedEarly,
        GanttBarShowForTasks::NotFinishedEarly,
        GanttBarShowForTasks::NotStartedOnTime,
        GanttBarShowForTasks::NotFinishedOnTime,
        GanttBarShowForTasks::NotFlag1,
        GanttBarShowForTasks::NotFlag2,
        GanttBarShowForTasks::NotFlag3,
        GanttBarShowForTasks::NotFlag4,
        GanttBarShowForTasks::NotFlag5,
        GanttBarShowForTasks::NotFlag6,
        GanttBarShowForTasks::NotFlag7,
        GanttBarShowForTasks::NotFlag8,
        GanttBarShowForTasks::NotFlag9,
        GanttBarShowForTasks::NotFlag10,
        GanttBarShowForTasks::NotRolledUp,
        GanttBarShowForTasks::NotProjectSummary,
        GanttBarShowForTasks::NotSplit,
        GanttBarShowForTasks::NotExternal,
        GanttBarShowForTasks::NotFlag11,
        GanttBarShowForTasks::NotFlag12,
        GanttBarShowForTasks::NotFlag13,
        GanttBarShowForTasks::NotFlag14,
        GanttBarShowForTasks::NotFlag15,
        GanttBarShowForTasks::NotFlag16,
        GanttBarShowForTasks::NotFlag17,
        GanttBarShowForTasks::NotFlag18,
        GanttBarShowForTasks::NotFlag19,
        GanttBarShowForTasks::NotFlag20,
        GanttBarShowForTasks::NotGroupBySummary,
        GanttBarShowForTasks::NotDeliverable,
        GanttBarShowForTasks::NotDependency,
        GanttBarShowForTasks::NotActive,
        GanttBarShowForTasks::NotManuallyScheduled,
        GanttBarShowForTasks::NotWarning,
        GanttBarShowForTasks::NotPlaceholderStart,
        GanttBarShowForTasks::NotPlaceholderFinish,
        GanttBarShowForTasks::NotPlaceholderDuration,
        GanttBarShowForTasks::NotPlaceholder,
        GanttBarShowForTasks::NotLate,
    ];

    /// Java 查找表长度（常量个数 98 + `createTypeArray` 的 `arraySizeOffset` 26）。
    ///
    /// 对应 Java: `EnumHelper.createTypeArray(GanttBarShowForTasks.class, 26)` 得到的数组长度。
    pub const TYPE_VALUES_LENGTH: i32 = 124;

    /// 根据 int 值取枚举实例。
    ///
    /// 对应 Java: `public static GanttBarShowForTasks getInstance(int type)`
    ///
    /// 与其它枚举不同，Java 侧**可能返回 `null`**（查找表未覆盖的下标 68 / 72 / 73 /
    /// 115..123），Rust 侧对应返回 `None`；负值与 `>= 124` 按 Java 逻辑回落到 `NORMAL`。
    pub fn get_instance(value: i32) -> Option<GanttBarShowForTasks> {
        match value {
            0 => Some(GanttBarShowForTasks::Normal),
            1 => Some(GanttBarShowForTasks::Milestone),
            2 => Some(GanttBarShowForTasks::Summary),
            3 => Some(GanttBarShowForTasks::Critical),
            4 => Some(GanttBarShowForTasks::Noncritical),
            5 => Some(GanttBarShowForTasks::Marked),
            6 => Some(GanttBarShowForTasks::Finished),
            7 => Some(GanttBarShowForTasks::InProgress),
            8 => Some(GanttBarShowForTasks::NotFinished),
            9 => Some(GanttBarShowForTasks::NotStarted),
            10 => Some(GanttBarShowForTasks::StartedLate),
            11 => Some(GanttBarShowForTasks::FinishedLate),
            12 => Some(GanttBarShowForTasks::StartedEarly),
            13 => Some(GanttBarShowForTasks::FinishedEarly),
            14 => Some(GanttBarShowForTasks::StartedOnTime),
            15 => Some(GanttBarShowForTasks::FinishedOnTime),
            16 => Some(GanttBarShowForTasks::Flag1),
            17 => Some(GanttBarShowForTasks::Flag2),
            18 => Some(GanttBarShowForTasks::Flag3),
            19 => Some(GanttBarShowForTasks::Flag4),
            20 => Some(GanttBarShowForTasks::Flag5),
            21 => Some(GanttBarShowForTasks::Flag6),
            22 => Some(GanttBarShowForTasks::Flag7),
            23 => Some(GanttBarShowForTasks::Flag8),
            24 => Some(GanttBarShowForTasks::Flag9),
            25 => Some(GanttBarShowForTasks::Flag10),
            26 => Some(GanttBarShowForTasks::RolledUp),
            27 => Some(GanttBarShowForTasks::ProjectSummary),
            28 => Some(GanttBarShowForTasks::Split),
            29 => Some(GanttBarShowForTasks::External),
            30 => Some(GanttBarShowForTasks::Flag11),
            31 => Some(GanttBarShowForTasks::Flag12),
            32 => Some(GanttBarShowForTasks::Flag13),
            33 => Some(GanttBarShowForTasks::Flag14),
            34 => Some(GanttBarShowForTasks::Flag15),
            35 => Some(GanttBarShowForTasks::Flag16),
            36 => Some(GanttBarShowForTasks::Flag17),
            37 => Some(GanttBarShowForTasks::Flag18),
            38 => Some(GanttBarShowForTasks::Flag19),
            39 => Some(GanttBarShowForTasks::Flag20),
            40 => Some(GanttBarShowForTasks::GroupBySummary),
            41 => Some(GanttBarShowForTasks::Deliverable),
            42 => Some(GanttBarShowForTasks::Dependency),
            43 => Some(GanttBarShowForTasks::Active),
            44 => Some(GanttBarShowForTasks::ManuallyScheduled),
            45 => Some(GanttBarShowForTasks::Warning),
            46 => Some(GanttBarShowForTasks::PlaceholderStart),
            47 => Some(GanttBarShowForTasks::PlaceholderFinish),
            48 => Some(GanttBarShowForTasks::PlaceholderDuration),
            49 => Some(GanttBarShowForTasks::Placeholder),
            50 => Some(GanttBarShowForTasks::Late),
            64 => Some(GanttBarShowForTasks::NotNormal),
            65 => Some(GanttBarShowForTasks::NotMilestone),
            66 => Some(GanttBarShowForTasks::NotSummary),
            67 => Some(GanttBarShowForTasks::NotCritical),
            // Java: 静态块 `TYPE_VALUES[70] = GanttBarShowForTasks.NOTFINISHED;`
            70 => Some(GanttBarShowForTasks::NotFinished),
            69 => Some(GanttBarShowForTasks::NotMarked),
            71 => Some(GanttBarShowForTasks::NotInProgress),
            74 => Some(GanttBarShowForTasks::NotStartedLate),
            75 => Some(GanttBarShowForTasks::NotFinishedLate),
            76 => Some(GanttBarShowForTasks::NotStartedEarly),
            77 => Some(GanttBarShowForTasks::NotFinishedEarly),
            78 => Some(GanttBarShowForTasks::NotStartedOnTime),
            79 => Some(GanttBarShowForTasks::NotFinishedOnTime),
            80 => Some(GanttBarShowForTasks::NotFlag1),
            81 => Some(GanttBarShowForTasks::NotFlag2),
            82 => Some(GanttBarShowForTasks::NotFlag3),
            83 => Some(GanttBarShowForTasks::NotFlag4),
            84 => Some(GanttBarShowForTasks::NotFlag5),
            85 => Some(GanttBarShowForTasks::NotFlag6),
            86 => Some(GanttBarShowForTasks::NotFlag7),
            87 => Some(GanttBarShowForTasks::NotFlag8),
            88 => Some(GanttBarShowForTasks::NotFlag9),
            89 => Some(GanttBarShowForTasks::NotFlag10),
            90 => Some(GanttBarShowForTasks::NotRolledUp),
            91 => Some(GanttBarShowForTasks::NotProjectSummary),
            92 => Some(GanttBarShowForTasks::NotSplit),
            93 => Some(GanttBarShowForTasks::NotExternal),
            94 => Some(GanttBarShowForTasks::NotFlag11),
            95 => Some(GanttBarShowForTasks::NotFlag12),
            96 => Some(GanttBarShowForTasks::NotFlag13),
            97 => Some(GanttBarShowForTasks::NotFlag14),
            98 => Some(GanttBarShowForTasks::NotFlag15),
            99 => Some(GanttBarShowForTasks::NotFlag16),
            100 => Some(GanttBarShowForTasks::NotFlag17),
            101 => Some(GanttBarShowForTasks::NotFlag18),
            102 => Some(GanttBarShowForTasks::NotFlag19),
            103 => Some(GanttBarShowForTasks::NotFlag20),
            104 => Some(GanttBarShowForTasks::NotGroupBySummary),
            105 => Some(GanttBarShowForTasks::NotDeliverable),
            106 => Some(GanttBarShowForTasks::NotDependency),
            107 => Some(GanttBarShowForTasks::NotActive),
            108 => Some(GanttBarShowForTasks::NotManuallyScheduled),
            109 => Some(GanttBarShowForTasks::NotWarning),
            110 => Some(GanttBarShowForTasks::NotPlaceholderStart),
            111 => Some(GanttBarShowForTasks::NotPlaceholderFinish),
            112 => Some(GanttBarShowForTasks::NotPlaceholderDuration),
            113 => Some(GanttBarShowForTasks::NotPlaceholder),
            114 => Some(GanttBarShowForTasks::NotLate),
            // Java: TYPE_VALUES 未覆盖的下标为 null（MSP 不使用的取值）
            68 | 72 | 73 => None,
            115..=123 => None,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { result = NORMAL; }`
            _ => Some(GanttBarShowForTasks::Normal),
        }
    }

    /// 对应 Java: `public static GanttBarShowForTasks getInstance(Number type)`
    ///
    /// Java 侧 `null` 走 `getInstance(-1)`，最终回落到 `NORMAL`。
    pub fn get_instance_optional(value: Option<i32>) -> Option<GanttBarShowForTasks> {
        GanttBarShowForTasks::get_instance(value.unwrap_or(-1))
    }

    /// 取判据名称（如 `"In Progress"`），与 Java 一致，未做本地化。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            GanttBarShowForTasks::Normal => "Normal",
            GanttBarShowForTasks::Milestone => "Milestone",
            GanttBarShowForTasks::Summary => "Summary",
            GanttBarShowForTasks::Critical => "Critical",
            GanttBarShowForTasks::Noncritical => "Noncritical",
            GanttBarShowForTasks::Marked => "Marked",
            GanttBarShowForTasks::Finished => "Finished",
            GanttBarShowForTasks::InProgress => "In Progress",
            GanttBarShowForTasks::NotFinished => "Not Finished",
            GanttBarShowForTasks::NotStarted => "Not Started",
            GanttBarShowForTasks::StartedLate => "Started Late",
            GanttBarShowForTasks::FinishedLate => "Finished Late",
            GanttBarShowForTasks::StartedEarly => "Started Early",
            GanttBarShowForTasks::FinishedEarly => "Finished Early",
            GanttBarShowForTasks::StartedOnTime => "Started On Time",
            GanttBarShowForTasks::FinishedOnTime => "Finished On Time",
            GanttBarShowForTasks::Flag1 => "Flag1",
            GanttBarShowForTasks::Flag2 => "Flag2",
            GanttBarShowForTasks::Flag3 => "Flag3",
            GanttBarShowForTasks::Flag4 => "Flag4",
            GanttBarShowForTasks::Flag5 => "Flag5",
            GanttBarShowForTasks::Flag6 => "Flag6",
            GanttBarShowForTasks::Flag7 => "Flag7",
            GanttBarShowForTasks::Flag8 => "Flag8",
            GanttBarShowForTasks::Flag9 => "Flag9",
            GanttBarShowForTasks::Flag10 => "Flag10",
            GanttBarShowForTasks::RolledUp => "Rolled Up",
            GanttBarShowForTasks::ProjectSummary => "Project Summary",
            GanttBarShowForTasks::Split => "Split",
            GanttBarShowForTasks::External => "External Tasks",
            GanttBarShowForTasks::Flag11 => "Flag11",
            GanttBarShowForTasks::Flag12 => "Flag12",
            GanttBarShowForTasks::Flag13 => "Flag13",
            GanttBarShowForTasks::Flag14 => "Flag14",
            GanttBarShowForTasks::Flag15 => "Flag15",
            GanttBarShowForTasks::Flag16 => "Flag16",
            GanttBarShowForTasks::Flag17 => "Flag17",
            GanttBarShowForTasks::Flag18 => "Flag18",
            GanttBarShowForTasks::Flag19 => "Flag19",
            GanttBarShowForTasks::Flag20 => "Flag20",
            GanttBarShowForTasks::GroupBySummary => "Group By Summary",
            GanttBarShowForTasks::Deliverable => "Deliverable",
            GanttBarShowForTasks::Dependency => "Dependency",
            GanttBarShowForTasks::Active => "Active",
            GanttBarShowForTasks::ManuallyScheduled => "Manually Scheduled",
            GanttBarShowForTasks::Warning => "Warning",
            GanttBarShowForTasks::PlaceholderStart => "Placeholder (Start)",
            GanttBarShowForTasks::PlaceholderFinish => "Placeholder (Finish)",
            GanttBarShowForTasks::PlaceholderDuration => "Placeholder (Duration)",
            GanttBarShowForTasks::Placeholder => "Placeholder",
            GanttBarShowForTasks::Late => "Late",
            GanttBarShowForTasks::NotNormal => "Not Normal",
            GanttBarShowForTasks::NotMilestone => "Not Milestone",
            GanttBarShowForTasks::NotSummary => "Not Summary",
            GanttBarShowForTasks::NotCritical => "Not Critical",
            GanttBarShowForTasks::NotMarked => "Not Marked",
            GanttBarShowForTasks::NotInProgress => "Not In Progress",
            GanttBarShowForTasks::NotStartedLate => "Not Started Late",
            GanttBarShowForTasks::NotFinishedLate => "Not Finished Late",
            GanttBarShowForTasks::NotStartedEarly => "Not Started Early",
            GanttBarShowForTasks::NotFinishedEarly => "Not Finished Early",
            GanttBarShowForTasks::NotStartedOnTime => "Not Started On Time",
            GanttBarShowForTasks::NotFinishedOnTime => "Not Finished On Time",
            GanttBarShowForTasks::NotFlag1 => "Not Flag1",
            GanttBarShowForTasks::NotFlag2 => "Not Flag2",
            GanttBarShowForTasks::NotFlag3 => "Not Flag3",
            GanttBarShowForTasks::NotFlag4 => "Not Flag4",
            GanttBarShowForTasks::NotFlag5 => "Not Flag5",
            GanttBarShowForTasks::NotFlag6 => "Not Flag6",
            GanttBarShowForTasks::NotFlag7 => "Not Flag7",
            GanttBarShowForTasks::NotFlag8 => "Not Flag8",
            GanttBarShowForTasks::NotFlag9 => "Not Flag9",
            GanttBarShowForTasks::NotFlag10 => "Not Flag10",
            GanttBarShowForTasks::NotRolledUp => "Not Rolled Up",
            GanttBarShowForTasks::NotProjectSummary => "Not Project Summary",
            GanttBarShowForTasks::NotSplit => "Not Split",
            GanttBarShowForTasks::NotExternal => "Not External Tasks",
            GanttBarShowForTasks::NotFlag11 => "Not Flag11",
            GanttBarShowForTasks::NotFlag12 => "Not Flag12",
            GanttBarShowForTasks::NotFlag13 => "Not Flag13",
            GanttBarShowForTasks::NotFlag14 => "Not Flag14",
            GanttBarShowForTasks::NotFlag15 => "Not Flag15",
            GanttBarShowForTasks::NotFlag16 => "Not Flag16",
            GanttBarShowForTasks::NotFlag17 => "Not Flag17",
            GanttBarShowForTasks::NotFlag18 => "Not Flag18",
            GanttBarShowForTasks::NotFlag19 => "Not Flag19",
            GanttBarShowForTasks::NotFlag20 => "Not Flag20",
            GanttBarShowForTasks::NotGroupBySummary => "Not Group By Summary",
            GanttBarShowForTasks::NotDeliverable => "Not Deliverable",
            GanttBarShowForTasks::NotDependency => "Not Dependency",
            GanttBarShowForTasks::NotActive => "Not Active",
            GanttBarShowForTasks::NotManuallyScheduled => "Not Manually Scheduled",
            GanttBarShowForTasks::NotWarning => "Not Warning",
            GanttBarShowForTasks::NotPlaceholderStart => "Not Placeholder (Start)",
            GanttBarShowForTasks::NotPlaceholderFinish => "Not Placeholder (Finish)",
            // Java: NOT_PLACEHOLDERDURATION(112, "Placeholder (Duration)")，名称按原样保留
            GanttBarShowForTasks::NotPlaceholderDuration => "Placeholder (Duration)",
            GanttBarShowForTasks::NotPlaceholder => "Not Placeholder",
            GanttBarShowForTasks::NotLate => "Not Late",
        }
    }
}

impl MpxjEnum for GanttBarShowForTasks {
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for GanttBarShowForTasks {
    /// 对应 Java: `@Override public String toString()`，即 `getName()`。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl From<GanttBarShowForTasks> for i32 {
    fn from(value: GanttBarShowForTasks) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_get_instance() {
        for item in GanttBarShowForTasks::ALL {
            assert_eq!(GanttBarShowForTasks::get_instance(item.value()), Some(item));
        }
    }

    #[test]
    fn unmapped_slots_return_none() {
        // Java: TYPE_VALUES 中这些下标为 null
        for value in [68, 72, 73, 115, 123] {
            assert_eq!(GanttBarShowForTasks::get_instance(value), None);
        }
    }

    #[test]
    fn out_of_range_falls_back_to_normal() {
        assert_eq!(
            GanttBarShowForTasks::get_instance(-1),
            Some(GanttBarShowForTasks::Normal)
        );
        assert_eq!(
            GanttBarShowForTasks::get_instance(GanttBarShowForTasks::TYPE_VALUES_LENGTH),
            Some(GanttBarShowForTasks::Normal)
        );
        // Java 的 `getInstance(Number)` 对 null 走 -1
        assert_eq!(
            GanttBarShowForTasks::get_instance_optional(None),
            Some(GanttBarShowForTasks::Normal)
        );
    }

    #[test]
    fn slot_70_is_remapped_to_not_finished() {
        // Java 静态块：TYPE_VALUES[70] = NOTFINISHED
        assert_eq!(
            GanttBarShowForTasks::get_instance(70),
            Some(GanttBarShowForTasks::NotFinished)
        );
    }

    #[test]
    fn negated_types_are_the_normal_types_plus_64() {
        // Java 注释所述编码：取反型 = 对应正常型 + 64（0x40）。
        // MSP 并未定义全部组合：68 / 72 / 73 缺失，70 另有映射。
        for item in [
            GanttBarShowForTasks::Normal,
            GanttBarShowForTasks::Milestone,
            GanttBarShowForTasks::Summary,
            GanttBarShowForTasks::Critical,
            GanttBarShowForTasks::Marked,
            GanttBarShowForTasks::InProgress,
            GanttBarShowForTasks::StartedLate,
            GanttBarShowForTasks::FinishedLate,
            GanttBarShowForTasks::StartedEarly,
            GanttBarShowForTasks::FinishedEarly,
            GanttBarShowForTasks::StartedOnTime,
            GanttBarShowForTasks::FinishedOnTime,
            GanttBarShowForTasks::Flag1,
            GanttBarShowForTasks::Flag20,
            GanttBarShowForTasks::RolledUp,
            GanttBarShowForTasks::ProjectSummary,
            GanttBarShowForTasks::Split,
            GanttBarShowForTasks::External,
            GanttBarShowForTasks::GroupBySummary,
            GanttBarShowForTasks::Deliverable,
            GanttBarShowForTasks::Dependency,
            GanttBarShowForTasks::Active,
            GanttBarShowForTasks::ManuallyScheduled,
            GanttBarShowForTasks::Warning,
            GanttBarShowForTasks::PlaceholderStart,
            GanttBarShowForTasks::PlaceholderFinish,
            GanttBarShowForTasks::PlaceholderDuration,
            GanttBarShowForTasks::Placeholder,
            GanttBarShowForTasks::Late,
        ] {
            let negated = GanttBarShowForTasks::get_instance(item.value() + 0x40);
            assert!(negated.is_some(), "缺少取反型: {}", item.get_name());
        }

        // 被 Java 注释掉的两个取值没有对应枚举
        assert_eq!(GanttBarShowForTasks::get_instance(68), None);
        assert_eq!(GanttBarShowForTasks::get_instance(72), None);
        assert_eq!(GanttBarShowForTasks::get_instance(73), None);
    }

    #[test]
    fn names_match_java() {
        assert_eq!(GanttBarShowForTasks::Normal.to_string(), "Normal");
        assert_eq!(GanttBarShowForTasks::InProgress.to_string(), "In Progress");
        assert_eq!(
            GanttBarShowForTasks::External.to_string(),
            "External Tasks"
        );
        // Java 的 NOT_PLACEHOLDERDURATION 名称漏了 "Not" 前缀
        assert_eq!(
            GanttBarShowForTasks::NotPlaceholderDuration.to_string(),
            "Placeholder (Duration)"
        );
    }
}
