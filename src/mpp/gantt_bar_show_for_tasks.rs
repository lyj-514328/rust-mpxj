//! Corresponds to Java: `org/mpxj/mpp/GanttBarShowForTasks.java`
//!
//! Enum of criteria defining "on which tasks a Gantt bar is shown".
//!
//! The Java comment explains: the `value` values are deliberately designed to distinguish a "normal form" from a "negated form" —
//! non-negative values (`< 64`) are normal form, `>= 64` is negated form; `value & 0x64` is 0 for normal form and
//! non-zero for negated form; `value & 0x63` converts a negated form back to normal form, and the converted value can be used with
//! `getInstance` to find the corresponding enum.
//!
//! # Porting trade-offs
//!
//! - Java uses `EnumHelper.createTypeArray(GanttBarShowForTasks.class, 26)` to build the table reflectively:
//! the array length = number of constants (98) + 26 = **124**, the index is `m_value`, and uncovered indexes are `null`
//! (for example 68 / 72 / 73 / 115..123 — the first three are the "Not Not Xxx" values unused by MSP).
//! Rust does not use reflection and uses a `match` in [`GanttBarShowForTasks::get_instance`] instead;
//! because Java's `getInstance` **may return `null`**, the Rust side returns `Option<Self>`
//! and keeps 124 as a valid range (`>= 124` falls back to `NORMAL`, just like negative values).
//! - Java's static block additionally maps index 70 to `NOTFINISHED` (the comment says this value theoretically means
//! "Not Not Finished" and should not occur, but it has occurred in real files); the Rust side keeps this mapping as well.
//! - `EnumHelper` / reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`:
//! none are used by this class other than `createTypeArray` (see `createTypeArray` above).
//!
//! Unported Java members: none.

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// Criterion for displaying a Gantt bar.
///
/// Corresponds to Java: `public enum GanttBarShowForTasks implements MpxjEnum`
///
/// Declaration order matches Java; the discriminant value is Java's `m_value` (the second negated-form section is 64..114).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GanttBarShowForTasks {
    /// Corresponds to Java: `NORMAL(0, "Normal")`
    Normal = 0,
    /// Corresponds to Java: `MILESTONE(1, "Milestone")`
    Milestone = 1,
    /// Corresponds to Java: `SUMMARY(2, "Summary")`
    Summary = 2,
    /// Corresponds to Java: `CRITICAL(3, "Critical")`
    Critical = 3,
    /// Corresponds to Java: `NONCRITICAL(4, "Noncritical")`
    Noncritical = 4,
    /// Corresponds to Java: `MARKED(5, "Marked")`
    Marked = 5,
    /// Corresponds to Java: `FINISHED(6, "Finished")`
    Finished = 6,
    /// Corresponds to Java: `INPROGRESS(7, "In Progress")`
    InProgress = 7,
    /// Corresponds to Java: `NOTFINISHED(8, "Not Finished")`
    NotFinished = 8,
    /// Corresponds to Java: `NOTSTARTED(9, "Not Started")`
    NotStarted = 9,
    /// Corresponds to Java: `STARTEDLATE(10, "Started Late")`
    StartedLate = 10,
    /// Corresponds to Java: `FINISHEDLATE(11, "Finished Late")`
    FinishedLate = 11,
    /// Corresponds to Java: `STARTEDEARLY(12, "Started Early")`
    StartedEarly = 12,
    /// Corresponds to Java: `FINISHEDEARLY(13, "Finished Early")`
    FinishedEarly = 13,
    /// Corresponds to Java: `STARTEDONTIME(14, "Started On Time")`
    StartedOnTime = 14,
    /// Corresponds to Java: `FINISHEDONTIME(15, "Finished On Time")`
    FinishedOnTime = 15,
    /// Corresponds to Java: `FLAG1(16, "Flag1")`
    Flag1 = 16,
    /// Corresponds to Java: `FLAG2(17, "Flag2")`
    Flag2 = 17,
    /// Corresponds to Java: `FLAG3(18, "Flag3")`
    Flag3 = 18,
    /// Corresponds to Java: `FLAG4(19, "Flag4")`
    Flag4 = 19,
    /// Corresponds to Java: `FLAG5(20, "Flag5")`
    Flag5 = 20,
    /// Corresponds to Java: `FLAG6(21, "Flag6")`
    Flag6 = 21,
    /// Corresponds to Java: `FLAG7(22, "Flag7")`
    Flag7 = 22,
    /// Corresponds to Java: `FLAG8(23, "Flag8")`
    Flag8 = 23,
    /// Corresponds to Java: `FLAG9(24, "Flag9")`
    Flag9 = 24,
    /// Corresponds to Java: `FLAG10(25, "Flag10")`
    Flag10 = 25,
    /// Corresponds to Java: `ROLLEDUP(26, "Rolled Up")`
    RolledUp = 26,
    /// Corresponds to Java: `PROJECTSUMMARY(27, "Project Summary")`
    ProjectSummary = 27,
    /// Corresponds to Java: `SPLIT(28, "Split")`
    Split = 28,
    /// Corresponds to Java: `EXTERNAL(29, "External Tasks")`
    External = 29,
    /// Corresponds to Java: `FLAG11(30, "Flag11")`
    Flag11 = 30,
    /// Corresponds to Java: `FLAG12(31, "Flag12")`
    Flag12 = 31,
    /// Corresponds to Java: `FLAG13(32, "Flag13")`
    Flag13 = 32,
    /// Corresponds to Java: `FLAG14(33, "Flag14")`
    Flag14 = 33,
    /// Corresponds to Java: `FLAG15(34, "Flag15")`
    Flag15 = 34,
    /// Corresponds to Java: `FLAG16(35, "Flag16")`
    Flag16 = 35,
    /// Corresponds to Java: `FLAG17(36, "Flag17")`
    Flag17 = 36,
    /// Corresponds to Java: `FLAG18(37, "Flag18")`
    Flag18 = 37,
    /// Corresponds to Java: `FLAG19(38, "Flag19")`
    Flag19 = 38,
    /// Corresponds to Java: `FLAG20(39, "Flag20")`
    Flag20 = 39,
    /// Corresponds to Java: `GROUPBYSUMMARY(40, "Group By Summary")`
    GroupBySummary = 40,
    /// Corresponds to Java: `DELIVERABLE(41, "Deliverable")`
    Deliverable = 41,
    /// Corresponds to Java: `DEPENDENCY(42, "Dependency")`
    Dependency = 42,
    /// Corresponds to Java: `ACTIVE(43, "Active")`
    Active = 43,
    /// Corresponds to Java: `MANUALLYSCHEDULED(44, "Manually Scheduled")`
    ManuallyScheduled = 44,
    /// Corresponds to Java: `WARNING(45, "Warning")`
    Warning = 45,
    /// Corresponds to Java: `PLACEHOLDERSTART(46, "Placeholder (Start)")`
    PlaceholderStart = 46,
    /// Corresponds to Java: `PLACEHOLDERFINISH(47, "Placeholder (Finish)")`
    PlaceholderFinish = 47,
    /// Corresponds to Java: `PLACEHOLDERDURATION(48, "Placeholder (Duration)")`
    PlaceholderDuration = 48,
    /// Corresponds to Java: `PLACEHOLDER(49, "Placeholder")`
    Placeholder = 49,
    /// Corresponds to Java: `LATE(50, "Late")`
    Late = 50,
    /// Corresponds to Java: `NOT_NORMAL(64, "Not Normal")`
    NotNormal = 64,
    /// Corresponds to Java: `NOT_MILESTONE(65, "Not Milestone")`
    NotMilestone = 65,
    /// Corresponds to Java: `NOT_SUMMARY(66, "Not Summary")`
    NotSummary = 66,
    /// Corresponds to Java: `NOT_CRITICAL(67, "Not Critical")`
    NotCritical = 67,
/// Corresponds to Java: `NOT_MARKED(69, "Not Marked")` (68 is the `NOT_NONCRITICAL` commented out in Java and unused by MSP)
    NotMarked = 69,
/// Corresponds to Java: `NOT_INPROGRESS(71, "Not In Progress")` (see the type-level notes for 70, which is mapped to `NOTFINISHED`)
    NotInProgress = 71,
/// Corresponds to Java: `NOT_STARTEDLATE(74, "Not Started Late")` (72 and 73 are values commented out in Java)
    NotStartedLate = 74,
    /// Corresponds to Java: `NOT_FINISHEDLATE(75, "Not Finished Late")`
    NotFinishedLate = 75,
    /// Corresponds to Java: `NOT_STARTEDEARLY(76, "Not Started Early")`
    NotStartedEarly = 76,
    /// Corresponds to Java: `NOT_FINISHEDEARLY(77, "Not Finished Early")`
    NotFinishedEarly = 77,
    /// Corresponds to Java: `NOT_STARTEDONTIME(78, "Not Started On Time")`
    NotStartedOnTime = 78,
    /// Corresponds to Java: `NOT_FINISHEDONTIME(79, "Not Finished On Time")`
    NotFinishedOnTime = 79,
    /// Corresponds to Java: `NOT_FLAG1(80, "Not Flag1")`
    NotFlag1 = 80,
    /// Corresponds to Java: `NOT_FLAG2(81, "Not Flag2")`
    NotFlag2 = 81,
    /// Corresponds to Java: `NOT_FLAG3(82, "Not Flag3")`
    NotFlag3 = 82,
    /// Corresponds to Java: `NOT_FLAG4(83, "Not Flag4")`
    NotFlag4 = 83,
    /// Corresponds to Java: `NOT_FLAG5(84, "Not Flag5")`
    NotFlag5 = 84,
    /// Corresponds to Java: `NOT_FLAG6(85, "Not Flag6")`
    NotFlag6 = 85,
    /// Corresponds to Java: `NOT_FLAG7(86, "Not Flag7")`
    NotFlag7 = 86,
    /// Corresponds to Java: `NOT_FLAG8(87, "Not Flag8")`
    NotFlag8 = 87,
    /// Corresponds to Java: `NOT_FLAG9(88, "Not Flag9")`
    NotFlag9 = 88,
    /// Corresponds to Java: `NOT_FLAG10(89, "Not Flag10")`
    NotFlag10 = 89,
    /// Corresponds to Java: `NOT_ROLLEDUP(90, "Not Rolled Up")`
    NotRolledUp = 90,
    /// Corresponds to Java: `NOT_PROJECTSUMMARY(91, "Not Project Summary")`
    NotProjectSummary = 91,
    /// Corresponds to Java: `NOT_SPLIT(92, "Not Split")`
    NotSplit = 92,
    /// Corresponds to Java: `NOT_EXTERNAL(93, "Not External Tasks")`
    NotExternal = 93,
    /// Corresponds to Java: `NOT_FLAG11(94, "Not Flag11")`
    NotFlag11 = 94,
    /// Corresponds to Java: `NOT_FLAG12(95, "Not Flag12")`
    NotFlag12 = 95,
    /// Corresponds to Java: `NOT_FLAG13(96, "Not Flag13")`
    NotFlag13 = 96,
    /// Corresponds to Java: `NOT_FLAG14(97, "Not Flag14")`
    NotFlag14 = 97,
    /// Corresponds to Java: `NOT_FLAG15(98, "Not Flag15")`
    NotFlag15 = 98,
    /// Corresponds to Java: `NOT_FLAG16(99, "Not Flag16")`
    NotFlag16 = 99,
    /// Corresponds to Java: `NOT_FLAG17(100, "Not Flag17")`
    NotFlag17 = 100,
    /// Corresponds to Java: `NOT_FLAG18(101, "Not Flag18")`
    NotFlag18 = 101,
    /// Corresponds to Java: `NOT_FLAG19(102, "Not Flag19")`
    NotFlag19 = 102,
    /// Corresponds to Java: `NOT_FLAG20(103, "Not Flag20")`
    NotFlag20 = 103,
    /// Corresponds to Java: `NOT_GROUPBYSUMMARY(104, "Not Group By Summary")`
    NotGroupBySummary = 104,
    /// Corresponds to Java: `NOT_DELIVERABLE(105, "Not Deliverable")`
    NotDeliverable = 105,
    /// Corresponds to Java: `NOT_DEPENDENCY(106, "Not Dependency")`
    NotDependency = 106,
    /// Corresponds to Java: `NOT_ACTIVE(107, "Not Active")`
    NotActive = 107,
    /// Corresponds to Java: `NOT_MANUALLYSCHEDULED(108, "Not Manually Scheduled")`
    NotManuallyScheduled = 108,
    /// Corresponds to Java: `NOT_WARNING(109, "Not Warning")`
    NotWarning = 109,
    /// Corresponds to Java: `NOT_PLACEHOLDERSTART(110, "Not Placeholder (Start)")`
    NotPlaceholderStart = 110,
    /// Corresponds to Java: `NOT_PLACEHOLDERFINISH(111, "Not Placeholder (Finish)")`
    NotPlaceholderFinish = 111,
/// Corresponds to Java: `NOT_PLACEHOLDERDURATION(112, "Placeholder (Duration)")` (Java's name is missing the "Not" prefix; kept as-is)
    NotPlaceholderDuration = 112,
    /// Corresponds to Java: `NOT_PLACEHOLDER(113, "Not Placeholder")`
    NotPlaceholder = 113,
    /// Corresponds to Java: `NOT_LATE(114, "Not Late")`
    NotLate = 114,
}

impl GanttBarShowForTasks {
/// All values, in the same order as the Java declaration order (i.e. ascending `m_value`).
    ///
/// Corresponds to Java: the populated entries in `private static final GanttBarShowForTasks[] TYPE_VALUES`.
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

/// The Java lookup table length (98 constants + `createTypeArray`'s `arraySizeOffset` 26).
    ///
/// Corresponds to Java: the array length produced by `EnumHelper.createTypeArray(GanttBarShowForTasks.class, 26)`.
    pub const TYPE_VALUES_LENGTH: i32 = 124;

/// Get the enum instance for an int value.
    ///
    /// Corresponds to Java: `public static GanttBarShowForTasks getInstance(int type)`
    ///
/// Unlike other enums, the Java side **may return `null`** (indexes 68 / 72 / 73 /
/// 115..123 are not covered by the lookup table), and the Rust side correspondingly returns `None`; negative values and `>= 124` fall back to `NORMAL` per Java logic.
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
// Java: static block `TYPE_VALUES[70] = GanttBarShowForTasks.NOTFINISHED;`
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
// Java: indexes not covered by TYPE_VALUES are null (values unused by MSP)
            68 | 72 | 73 => None,
            115..=123 => None,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { result = NORMAL; }`
            _ => Some(GanttBarShowForTasks::Normal),
        }
    }

    /// Corresponds to Java: `public static GanttBarShowForTasks getInstance(Number type)`
    ///
/// On the Java side `null` goes through `getInstance(-1)` and ultimately falls back to `NORMAL`.
    pub fn get_instance_optional(value: Option<i32>) -> Option<GanttBarShowForTasks> {
        GanttBarShowForTasks::get_instance(value.unwrap_or(-1))
    }

/// Get the criterion name (e.g. `"In Progress"`); consistent with Java, not localized.
    ///
    /// Corresponds to Java: `public String getName()`
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
// Java: NOT_PLACEHOLDERDURATION(112, "Placeholder (Duration)"); the name is kept as-is
            GanttBarShowForTasks::NotPlaceholderDuration => "Placeholder (Duration)",
            GanttBarShowForTasks::NotPlaceholder => "Not Placeholder",
            GanttBarShowForTasks::NotLate => "Not Late",
        }
    }
}

impl MpxjEnum for GanttBarShowForTasks {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for GanttBarShowForTasks {
/// Corresponds to Java: `@Override public String toString()`, i.e. `getName()`.
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
// Java: these indexes are null in TYPE_VALUES
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
// Java's `getInstance(Number)` goes through -1 for null
        assert_eq!(
            GanttBarShowForTasks::get_instance_optional(None),
            Some(GanttBarShowForTasks::Normal)
        );
    }

    #[test]
    fn slot_70_is_remapped_to_not_finished() {
// Java static block: TYPE_VALUES[70] = NOTFINISHED
        assert_eq!(
            GanttBarShowForTasks::get_instance(70),
            Some(GanttBarShowForTasks::NotFinished)
        );
    }

    #[test]
    fn negated_types_are_the_normal_types_plus_64() {
// Encoding described by the Java comment: negated form = corresponding normal form + 64 (0x40).
// MSP does not define all combinations: 68 / 72 / 73 are missing and 70 has a separate mapping.
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
            assert!(negated.is_some(), "missing negated type: {}", item.get_name());
        }

// the two values commented out in Java have no corresponding enums
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
// Java's NOT_PLACEHOLDERDURATION name is missing the "Not" prefix
        assert_eq!(
            GanttBarShowForTasks::NotPlaceholderDuration.to_string(),
            "Placeholder (Duration)"
        );
    }
}
