//! Corresponds to Java: `org/mpxj/common/TaskFieldLists.java`

use crate::task_field::TaskField;

/// Collections of Task fields grouped logically.
///
/// Corresponds to Java: `public final class TaskFieldLists`
///
/// Porting notes:
/// - Each Java `TaskField[]` static array is ported as a fixed-size array constant `[TaskField; N]`, with elements and order exactly matching Java.
/// - Java's `CUSTOM_FIELDS` is built by a static initializer that `addAll`s 15 arrays in turn into an `ArrayList<TaskField>`; the Rust side expands it into a `&'static [TaskField]` constant in the same order as the Java `addAll` calls.
/// - This class has only static constants and no instance members; the Rust side uses a zero-field struct to carry these associated constants.
///
/// Skipped Java members: none.
pub struct TaskFieldLists;

impl TaskFieldLists {
    /// Corresponds to Java: `public static final TaskField[] CUSTOM_COST`
    pub const CUSTOM_COST: [TaskField; 10] = [
        TaskField::Cost1,
        TaskField::Cost2,
        TaskField::Cost3,
        TaskField::Cost4,
        TaskField::Cost5,
        TaskField::Cost6,
        TaskField::Cost7,
        TaskField::Cost8,
        TaskField::Cost9,
        TaskField::Cost10,
    ];

    /// Corresponds to Java: `public static final TaskField[] CUSTOM_DATE`
    pub const CUSTOM_DATE: [TaskField; 10] = [
        TaskField::Date1,
        TaskField::Date2,
        TaskField::Date3,
        TaskField::Date4,
        TaskField::Date5,
        TaskField::Date6,
        TaskField::Date7,
        TaskField::Date8,
        TaskField::Date9,
        TaskField::Date10,
    ];

    /// Corresponds to Java: `public static final TaskField[] CUSTOM_DURATION`
    pub const CUSTOM_DURATION: [TaskField; 10] = [
        TaskField::Duration1,
        TaskField::Duration2,
        TaskField::Duration3,
        TaskField::Duration4,
        TaskField::Duration5,
        TaskField::Duration6,
        TaskField::Duration7,
        TaskField::Duration8,
        TaskField::Duration9,
        TaskField::Duration10,
    ];

    /// Corresponds to Java: `public static final TaskField[] CUSTOM_DURATION_UNITS`
    pub const CUSTOM_DURATION_UNITS: [TaskField; 10] = [
        TaskField::Duration1Units,
        TaskField::Duration2Units,
        TaskField::Duration3Units,
        TaskField::Duration4Units,
        TaskField::Duration5Units,
        TaskField::Duration6Units,
        TaskField::Duration7Units,
        TaskField::Duration8Units,
        TaskField::Duration9Units,
        TaskField::Duration10Units,
    ];

    /// Corresponds to Java: `public static final TaskField[] CUSTOM_FLAG`
    pub const CUSTOM_FLAG: [TaskField; 20] = [
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
        TaskField::Flag11,
        TaskField::Flag12,
        TaskField::Flag13,
        TaskField::Flag14,
        TaskField::Flag15,
        TaskField::Flag16,
        TaskField::Flag17,
        TaskField::Flag18,
        TaskField::Flag19,
        TaskField::Flag20,
    ];

    /// Corresponds to Java: `public static final TaskField[] CUSTOM_FINISH`
    pub const CUSTOM_FINISH: [TaskField; 10] = [
        TaskField::Finish1,
        TaskField::Finish2,
        TaskField::Finish3,
        TaskField::Finish4,
        TaskField::Finish5,
        TaskField::Finish6,
        TaskField::Finish7,
        TaskField::Finish8,
        TaskField::Finish9,
        TaskField::Finish10,
    ];

    /// Corresponds to Java: `public static final TaskField[] CUSTOM_NUMBER`
    pub const CUSTOM_NUMBER: [TaskField; 20] = [
        TaskField::Number1,
        TaskField::Number2,
        TaskField::Number3,
        TaskField::Number4,
        TaskField::Number5,
        TaskField::Number6,
        TaskField::Number7,
        TaskField::Number8,
        TaskField::Number9,
        TaskField::Number10,
        TaskField::Number11,
        TaskField::Number12,
        TaskField::Number13,
        TaskField::Number14,
        TaskField::Number15,
        TaskField::Number16,
        TaskField::Number17,
        TaskField::Number18,
        TaskField::Number19,
        TaskField::Number20,
    ];

    /// Corresponds to Java: `public static final TaskField[] CUSTOM_START`
    pub const CUSTOM_START: [TaskField; 10] = [
        TaskField::Start1,
        TaskField::Start2,
        TaskField::Start3,
        TaskField::Start4,
        TaskField::Start5,
        TaskField::Start6,
        TaskField::Start7,
        TaskField::Start8,
        TaskField::Start9,
        TaskField::Start10,
    ];

    /// Corresponds to Java: `public static final TaskField[] CUSTOM_TEXT`
    pub const CUSTOM_TEXT: [TaskField; 30] = [
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
        TaskField::Text11,
        TaskField::Text12,
        TaskField::Text13,
        TaskField::Text14,
        TaskField::Text15,
        TaskField::Text16,
        TaskField::Text17,
        TaskField::Text18,
        TaskField::Text19,
        TaskField::Text20,
        TaskField::Text21,
        TaskField::Text22,
        TaskField::Text23,
        TaskField::Text24,
        TaskField::Text25,
        TaskField::Text26,
        TaskField::Text27,
        TaskField::Text28,
        TaskField::Text29,
        TaskField::Text30,
    ];

    /// Corresponds to Java: `public static final TaskField[] CUSTOM_OUTLINE_CODE`
    pub const CUSTOM_OUTLINE_CODE: [TaskField; 10] = [
        TaskField::OutlineCode1,
        TaskField::OutlineCode2,
        TaskField::OutlineCode3,
        TaskField::OutlineCode4,
        TaskField::OutlineCode5,
        TaskField::OutlineCode6,
        TaskField::OutlineCode7,
        TaskField::OutlineCode8,
        TaskField::OutlineCode9,
        TaskField::OutlineCode10,
    ];

    /// Corresponds to Java: `public static final TaskField[] CUSTOM_OUTLINE_CODE_INDEX`
    pub const CUSTOM_OUTLINE_CODE_INDEX: [TaskField; 10] = [
        TaskField::OutlineCode1Index,
        TaskField::OutlineCode2Index,
        TaskField::OutlineCode3Index,
        TaskField::OutlineCode4Index,
        TaskField::OutlineCode5Index,
        TaskField::OutlineCode6Index,
        TaskField::OutlineCode7Index,
        TaskField::OutlineCode8Index,
        TaskField::OutlineCode9Index,
        TaskField::OutlineCode10Index,
    ];

    /// Corresponds to Java: `public static final TaskField[] ENTERPRISE_CUSTOM_COST`
    pub const ENTERPRISE_CUSTOM_COST: [TaskField; 10] = [
        TaskField::EnterpriseCost1,
        TaskField::EnterpriseCost2,
        TaskField::EnterpriseCost3,
        TaskField::EnterpriseCost4,
        TaskField::EnterpriseCost5,
        TaskField::EnterpriseCost6,
        TaskField::EnterpriseCost7,
        TaskField::EnterpriseCost8,
        TaskField::EnterpriseCost9,
        TaskField::EnterpriseCost10,
    ];

    /// Corresponds to Java: `public static final TaskField[] ENTERPRISE_CUSTOM_DATE`
    pub const ENTERPRISE_CUSTOM_DATE: [TaskField; 30] = [
        TaskField::EnterpriseDate1,
        TaskField::EnterpriseDate2,
        TaskField::EnterpriseDate3,
        TaskField::EnterpriseDate4,
        TaskField::EnterpriseDate5,
        TaskField::EnterpriseDate6,
        TaskField::EnterpriseDate7,
        TaskField::EnterpriseDate8,
        TaskField::EnterpriseDate9,
        TaskField::EnterpriseDate10,
        TaskField::EnterpriseDate11,
        TaskField::EnterpriseDate12,
        TaskField::EnterpriseDate13,
        TaskField::EnterpriseDate14,
        TaskField::EnterpriseDate15,
        TaskField::EnterpriseDate16,
        TaskField::EnterpriseDate17,
        TaskField::EnterpriseDate18,
        TaskField::EnterpriseDate19,
        TaskField::EnterpriseDate20,
        TaskField::EnterpriseDate21,
        TaskField::EnterpriseDate22,
        TaskField::EnterpriseDate23,
        TaskField::EnterpriseDate24,
        TaskField::EnterpriseDate25,
        TaskField::EnterpriseDate26,
        TaskField::EnterpriseDate27,
        TaskField::EnterpriseDate28,
        TaskField::EnterpriseDate29,
        TaskField::EnterpriseDate30,
    ];

    /// Corresponds to Java: `public static final TaskField[] ENTERPRISE_CUSTOM_DURATION`
    pub const ENTERPRISE_CUSTOM_DURATION: [TaskField; 10] = [
        TaskField::EnterpriseDuration1,
        TaskField::EnterpriseDuration2,
        TaskField::EnterpriseDuration3,
        TaskField::EnterpriseDuration4,
        TaskField::EnterpriseDuration5,
        TaskField::EnterpriseDuration6,
        TaskField::EnterpriseDuration7,
        TaskField::EnterpriseDuration8,
        TaskField::EnterpriseDuration9,
        TaskField::EnterpriseDuration10,
    ];

    /// Corresponds to Java: `public static final TaskField[] ENTERPRISE_CUSTOM_FLAG`
    pub const ENTERPRISE_CUSTOM_FLAG: [TaskField; 20] = [
        TaskField::EnterpriseFlag1,
        TaskField::EnterpriseFlag2,
        TaskField::EnterpriseFlag3,
        TaskField::EnterpriseFlag4,
        TaskField::EnterpriseFlag5,
        TaskField::EnterpriseFlag6,
        TaskField::EnterpriseFlag7,
        TaskField::EnterpriseFlag8,
        TaskField::EnterpriseFlag9,
        TaskField::EnterpriseFlag10,
        TaskField::EnterpriseFlag11,
        TaskField::EnterpriseFlag12,
        TaskField::EnterpriseFlag13,
        TaskField::EnterpriseFlag14,
        TaskField::EnterpriseFlag15,
        TaskField::EnterpriseFlag16,
        TaskField::EnterpriseFlag17,
        TaskField::EnterpriseFlag18,
        TaskField::EnterpriseFlag19,
        TaskField::EnterpriseFlag20,
    ];

    /// Corresponds to Java: `public static final TaskField[] ENTERPRISE_CUSTOM_NUMBER`
    pub const ENTERPRISE_CUSTOM_NUMBER: [TaskField; 40] = [
        TaskField::EnterpriseNumber1,
        TaskField::EnterpriseNumber2,
        TaskField::EnterpriseNumber3,
        TaskField::EnterpriseNumber4,
        TaskField::EnterpriseNumber5,
        TaskField::EnterpriseNumber6,
        TaskField::EnterpriseNumber7,
        TaskField::EnterpriseNumber8,
        TaskField::EnterpriseNumber9,
        TaskField::EnterpriseNumber10,
        TaskField::EnterpriseNumber11,
        TaskField::EnterpriseNumber12,
        TaskField::EnterpriseNumber13,
        TaskField::EnterpriseNumber14,
        TaskField::EnterpriseNumber15,
        TaskField::EnterpriseNumber16,
        TaskField::EnterpriseNumber17,
        TaskField::EnterpriseNumber18,
        TaskField::EnterpriseNumber19,
        TaskField::EnterpriseNumber20,
        TaskField::EnterpriseNumber21,
        TaskField::EnterpriseNumber22,
        TaskField::EnterpriseNumber23,
        TaskField::EnterpriseNumber24,
        TaskField::EnterpriseNumber25,
        TaskField::EnterpriseNumber26,
        TaskField::EnterpriseNumber27,
        TaskField::EnterpriseNumber28,
        TaskField::EnterpriseNumber29,
        TaskField::EnterpriseNumber30,
        TaskField::EnterpriseNumber31,
        TaskField::EnterpriseNumber32,
        TaskField::EnterpriseNumber33,
        TaskField::EnterpriseNumber34,
        TaskField::EnterpriseNumber35,
        TaskField::EnterpriseNumber36,
        TaskField::EnterpriseNumber37,
        TaskField::EnterpriseNumber38,
        TaskField::EnterpriseNumber39,
        TaskField::EnterpriseNumber40,
    ];

    /// Corresponds to Java: `public static final TaskField[] ENTERPRISE_CUSTOM_TEXT`
    pub const ENTERPRISE_CUSTOM_TEXT: [TaskField; 40] = [
        TaskField::EnterpriseText1,
        TaskField::EnterpriseText2,
        TaskField::EnterpriseText3,
        TaskField::EnterpriseText4,
        TaskField::EnterpriseText5,
        TaskField::EnterpriseText6,
        TaskField::EnterpriseText7,
        TaskField::EnterpriseText8,
        TaskField::EnterpriseText9,
        TaskField::EnterpriseText10,
        TaskField::EnterpriseText11,
        TaskField::EnterpriseText12,
        TaskField::EnterpriseText13,
        TaskField::EnterpriseText14,
        TaskField::EnterpriseText15,
        TaskField::EnterpriseText16,
        TaskField::EnterpriseText17,
        TaskField::EnterpriseText18,
        TaskField::EnterpriseText19,
        TaskField::EnterpriseText20,
        TaskField::EnterpriseText21,
        TaskField::EnterpriseText22,
        TaskField::EnterpriseText23,
        TaskField::EnterpriseText24,
        TaskField::EnterpriseText25,
        TaskField::EnterpriseText26,
        TaskField::EnterpriseText27,
        TaskField::EnterpriseText28,
        TaskField::EnterpriseText29,
        TaskField::EnterpriseText30,
        TaskField::EnterpriseText31,
        TaskField::EnterpriseText32,
        TaskField::EnterpriseText33,
        TaskField::EnterpriseText34,
        TaskField::EnterpriseText35,
        TaskField::EnterpriseText36,
        TaskField::EnterpriseText37,
        TaskField::EnterpriseText38,
        TaskField::EnterpriseText39,
        TaskField::EnterpriseText40,
    ];

    /// Corresponds to Java: `public static final TaskField[] BASELINE_COSTS`
    pub const BASELINE_COSTS: [TaskField; 10] = [
        TaskField::Baseline1Cost,
        TaskField::Baseline2Cost,
        TaskField::Baseline3Cost,
        TaskField::Baseline4Cost,
        TaskField::Baseline5Cost,
        TaskField::Baseline6Cost,
        TaskField::Baseline7Cost,
        TaskField::Baseline8Cost,
        TaskField::Baseline9Cost,
        TaskField::Baseline10Cost,
    ];

    /// Corresponds to Java: `public static final TaskField[] BASELINE_DURATIONS`
    pub const BASELINE_DURATIONS: [TaskField; 10] = [
        TaskField::Baseline1Duration,
        TaskField::Baseline2Duration,
        TaskField::Baseline3Duration,
        TaskField::Baseline4Duration,
        TaskField::Baseline5Duration,
        TaskField::Baseline6Duration,
        TaskField::Baseline7Duration,
        TaskField::Baseline8Duration,
        TaskField::Baseline9Duration,
        TaskField::Baseline10Duration,
    ];

    /// Corresponds to Java: `public static final TaskField[] BASELINE_ESTIMATED_DURATIONS`
    pub const BASELINE_ESTIMATED_DURATIONS: [TaskField; 10] = [
        TaskField::Baseline1EstimatedDuration,
        TaskField::Baseline2EstimatedDuration,
        TaskField::Baseline3EstimatedDuration,
        TaskField::Baseline4EstimatedDuration,
        TaskField::Baseline5EstimatedDuration,
        TaskField::Baseline6EstimatedDuration,
        TaskField::Baseline7EstimatedDuration,
        TaskField::Baseline8EstimatedDuration,
        TaskField::Baseline9EstimatedDuration,
        TaskField::Baseline10EstimatedDuration,
    ];

    /// Corresponds to Java: `public static final TaskField[] BASELINE_STARTS`
    pub const BASELINE_STARTS: [TaskField; 10] = [
        TaskField::Baseline1Start,
        TaskField::Baseline2Start,
        TaskField::Baseline3Start,
        TaskField::Baseline4Start,
        TaskField::Baseline5Start,
        TaskField::Baseline6Start,
        TaskField::Baseline7Start,
        TaskField::Baseline8Start,
        TaskField::Baseline9Start,
        TaskField::Baseline10Start,
    ];

    /// Corresponds to Java: `public static final TaskField[] BASELINE_ESTIMATED_STARTS`
    pub const BASELINE_ESTIMATED_STARTS: [TaskField; 10] = [
        TaskField::Baseline1EstimatedStart,
        TaskField::Baseline2EstimatedStart,
        TaskField::Baseline3EstimatedStart,
        TaskField::Baseline4EstimatedStart,
        TaskField::Baseline5EstimatedStart,
        TaskField::Baseline6EstimatedStart,
        TaskField::Baseline7EstimatedStart,
        TaskField::Baseline8EstimatedStart,
        TaskField::Baseline9EstimatedStart,
        TaskField::Baseline10EstimatedStart,
    ];

    /// Corresponds to Java: `public static final TaskField[] BASELINE_FINISHES`
    pub const BASELINE_FINISHES: [TaskField; 10] = [
        TaskField::Baseline1Finish,
        TaskField::Baseline2Finish,
        TaskField::Baseline3Finish,
        TaskField::Baseline4Finish,
        TaskField::Baseline5Finish,
        TaskField::Baseline6Finish,
        TaskField::Baseline7Finish,
        TaskField::Baseline8Finish,
        TaskField::Baseline9Finish,
        TaskField::Baseline10Finish,
    ];

    /// Corresponds to Java: `public static final TaskField[] BASELINE_ESTIMATED_FINISHES`
    pub const BASELINE_ESTIMATED_FINISHES: [TaskField; 10] = [
        TaskField::Baseline1EstimatedFinish,
        TaskField::Baseline2EstimatedFinish,
        TaskField::Baseline3EstimatedFinish,
        TaskField::Baseline4EstimatedFinish,
        TaskField::Baseline5EstimatedFinish,
        TaskField::Baseline6EstimatedFinish,
        TaskField::Baseline7EstimatedFinish,
        TaskField::Baseline8EstimatedFinish,
        TaskField::Baseline9EstimatedFinish,
        TaskField::Baseline10EstimatedFinish,
    ];

    /// Corresponds to Java: `public static final TaskField[] BASELINE_WORKS`
    pub const BASELINE_WORKS: [TaskField; 10] = [
        TaskField::Baseline1Work,
        TaskField::Baseline2Work,
        TaskField::Baseline3Work,
        TaskField::Baseline4Work,
        TaskField::Baseline5Work,
        TaskField::Baseline6Work,
        TaskField::Baseline7Work,
        TaskField::Baseline8Work,
        TaskField::Baseline9Work,
        TaskField::Baseline10Work,
    ];

    /// Corresponds to Java: `public static final TaskField[] BASELINE_FIXED_COSTS`
    pub const BASELINE_FIXED_COSTS: [TaskField; 10] = [
        TaskField::Baseline1FixedCost,
        TaskField::Baseline2FixedCost,
        TaskField::Baseline3FixedCost,
        TaskField::Baseline4FixedCost,
        TaskField::Baseline5FixedCost,
        TaskField::Baseline6FixedCost,
        TaskField::Baseline7FixedCost,
        TaskField::Baseline8FixedCost,
        TaskField::Baseline9FixedCost,
        TaskField::Baseline10FixedCost,
    ];

    /// Corresponds to Java: `public static final TaskField[] BASELINE_FIXED_COST_ACCRUALS`
    pub const BASELINE_FIXED_COST_ACCRUALS: [TaskField; 10] = [
        TaskField::Baseline1FixedCostAccrual,
        TaskField::Baseline2FixedCostAccrual,
        TaskField::Baseline3FixedCostAccrual,
        TaskField::Baseline4FixedCostAccrual,
        TaskField::Baseline5FixedCostAccrual,
        TaskField::Baseline6FixedCostAccrual,
        TaskField::Baseline7FixedCostAccrual,
        TaskField::Baseline8FixedCostAccrual,
        TaskField::Baseline9FixedCostAccrual,
        TaskField::Baseline10FixedCostAccrual,
    ];

    /// Corresponds to Java: `public static final TaskField[] BASELINE_BUDGET_COSTS`
    pub const BASELINE_BUDGET_COSTS: [TaskField; 10] = [
        TaskField::Baseline1BudgetCost,
        TaskField::Baseline2BudgetCost,
        TaskField::Baseline3BudgetCost,
        TaskField::Baseline4BudgetCost,
        TaskField::Baseline5BudgetCost,
        TaskField::Baseline6BudgetCost,
        TaskField::Baseline7BudgetCost,
        TaskField::Baseline8BudgetCost,
        TaskField::Baseline9BudgetCost,
        TaskField::Baseline10BudgetCost,
    ];

    /// Corresponds to Java: `public static final TaskField[] BASELINE_BUDGET_WORKS`
    pub const BASELINE_BUDGET_WORKS: [TaskField; 10] = [
        TaskField::Baseline1BudgetWork,
        TaskField::Baseline2BudgetWork,
        TaskField::Baseline3BudgetWork,
        TaskField::Baseline4BudgetWork,
        TaskField::Baseline5BudgetWork,
        TaskField::Baseline6BudgetWork,
        TaskField::Baseline7BudgetWork,
        TaskField::Baseline8BudgetWork,
        TaskField::Baseline9BudgetWork,
        TaskField::Baseline10BudgetWork,
    ];

    /// Corresponds to Java: `public static final List<TaskField> CUSTOM_FIELDS`
    ///
    /// Java concatenates via `addAll` in the static initializer in the order `CUSTOM_TEXT`, `CUSTOM_DATE`, `CUSTOM_START`, `CUSTOM_FINISH`, `CUSTOM_COST`, `CUSTOM_FLAG`, `CUSTOM_NUMBER`, `CUSTOM_DURATION`, `CUSTOM_OUTLINE_CODE`, `ENTERPRISE_CUSTOM_TEXT`, `ENTERPRISE_CUSTOM_DATE`, `ENTERPRISE_CUSTOM_COST`, `ENTERPRISE_CUSTOM_FLAG`, `ENTERPRISE_CUSTOM_NUMBER`, `ENTERPRISE_CUSTOM_DURATION`; the same order is kept here.
    pub const CUSTOM_FIELDS: &'static [TaskField] = &[
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
        TaskField::Text11,
        TaskField::Text12,
        TaskField::Text13,
        TaskField::Text14,
        TaskField::Text15,
        TaskField::Text16,
        TaskField::Text17,
        TaskField::Text18,
        TaskField::Text19,
        TaskField::Text20,
        TaskField::Text21,
        TaskField::Text22,
        TaskField::Text23,
        TaskField::Text24,
        TaskField::Text25,
        TaskField::Text26,
        TaskField::Text27,
        TaskField::Text28,
        TaskField::Text29,
        TaskField::Text30,
        TaskField::Date1,
        TaskField::Date2,
        TaskField::Date3,
        TaskField::Date4,
        TaskField::Date5,
        TaskField::Date6,
        TaskField::Date7,
        TaskField::Date8,
        TaskField::Date9,
        TaskField::Date10,
        TaskField::Start1,
        TaskField::Start2,
        TaskField::Start3,
        TaskField::Start4,
        TaskField::Start5,
        TaskField::Start6,
        TaskField::Start7,
        TaskField::Start8,
        TaskField::Start9,
        TaskField::Start10,
        TaskField::Finish1,
        TaskField::Finish2,
        TaskField::Finish3,
        TaskField::Finish4,
        TaskField::Finish5,
        TaskField::Finish6,
        TaskField::Finish7,
        TaskField::Finish8,
        TaskField::Finish9,
        TaskField::Finish10,
        TaskField::Cost1,
        TaskField::Cost2,
        TaskField::Cost3,
        TaskField::Cost4,
        TaskField::Cost5,
        TaskField::Cost6,
        TaskField::Cost7,
        TaskField::Cost8,
        TaskField::Cost9,
        TaskField::Cost10,
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
        TaskField::Flag11,
        TaskField::Flag12,
        TaskField::Flag13,
        TaskField::Flag14,
        TaskField::Flag15,
        TaskField::Flag16,
        TaskField::Flag17,
        TaskField::Flag18,
        TaskField::Flag19,
        TaskField::Flag20,
        TaskField::Number1,
        TaskField::Number2,
        TaskField::Number3,
        TaskField::Number4,
        TaskField::Number5,
        TaskField::Number6,
        TaskField::Number7,
        TaskField::Number8,
        TaskField::Number9,
        TaskField::Number10,
        TaskField::Number11,
        TaskField::Number12,
        TaskField::Number13,
        TaskField::Number14,
        TaskField::Number15,
        TaskField::Number16,
        TaskField::Number17,
        TaskField::Number18,
        TaskField::Number19,
        TaskField::Number20,
        TaskField::Duration1,
        TaskField::Duration2,
        TaskField::Duration3,
        TaskField::Duration4,
        TaskField::Duration5,
        TaskField::Duration6,
        TaskField::Duration7,
        TaskField::Duration8,
        TaskField::Duration9,
        TaskField::Duration10,
        TaskField::OutlineCode1,
        TaskField::OutlineCode2,
        TaskField::OutlineCode3,
        TaskField::OutlineCode4,
        TaskField::OutlineCode5,
        TaskField::OutlineCode6,
        TaskField::OutlineCode7,
        TaskField::OutlineCode8,
        TaskField::OutlineCode9,
        TaskField::OutlineCode10,
        TaskField::EnterpriseText1,
        TaskField::EnterpriseText2,
        TaskField::EnterpriseText3,
        TaskField::EnterpriseText4,
        TaskField::EnterpriseText5,
        TaskField::EnterpriseText6,
        TaskField::EnterpriseText7,
        TaskField::EnterpriseText8,
        TaskField::EnterpriseText9,
        TaskField::EnterpriseText10,
        TaskField::EnterpriseText11,
        TaskField::EnterpriseText12,
        TaskField::EnterpriseText13,
        TaskField::EnterpriseText14,
        TaskField::EnterpriseText15,
        TaskField::EnterpriseText16,
        TaskField::EnterpriseText17,
        TaskField::EnterpriseText18,
        TaskField::EnterpriseText19,
        TaskField::EnterpriseText20,
        TaskField::EnterpriseText21,
        TaskField::EnterpriseText22,
        TaskField::EnterpriseText23,
        TaskField::EnterpriseText24,
        TaskField::EnterpriseText25,
        TaskField::EnterpriseText26,
        TaskField::EnterpriseText27,
        TaskField::EnterpriseText28,
        TaskField::EnterpriseText29,
        TaskField::EnterpriseText30,
        TaskField::EnterpriseText31,
        TaskField::EnterpriseText32,
        TaskField::EnterpriseText33,
        TaskField::EnterpriseText34,
        TaskField::EnterpriseText35,
        TaskField::EnterpriseText36,
        TaskField::EnterpriseText37,
        TaskField::EnterpriseText38,
        TaskField::EnterpriseText39,
        TaskField::EnterpriseText40,
        TaskField::EnterpriseDate1,
        TaskField::EnterpriseDate2,
        TaskField::EnterpriseDate3,
        TaskField::EnterpriseDate4,
        TaskField::EnterpriseDate5,
        TaskField::EnterpriseDate6,
        TaskField::EnterpriseDate7,
        TaskField::EnterpriseDate8,
        TaskField::EnterpriseDate9,
        TaskField::EnterpriseDate10,
        TaskField::EnterpriseDate11,
        TaskField::EnterpriseDate12,
        TaskField::EnterpriseDate13,
        TaskField::EnterpriseDate14,
        TaskField::EnterpriseDate15,
        TaskField::EnterpriseDate16,
        TaskField::EnterpriseDate17,
        TaskField::EnterpriseDate18,
        TaskField::EnterpriseDate19,
        TaskField::EnterpriseDate20,
        TaskField::EnterpriseDate21,
        TaskField::EnterpriseDate22,
        TaskField::EnterpriseDate23,
        TaskField::EnterpriseDate24,
        TaskField::EnterpriseDate25,
        TaskField::EnterpriseDate26,
        TaskField::EnterpriseDate27,
        TaskField::EnterpriseDate28,
        TaskField::EnterpriseDate29,
        TaskField::EnterpriseDate30,
        TaskField::EnterpriseCost1,
        TaskField::EnterpriseCost2,
        TaskField::EnterpriseCost3,
        TaskField::EnterpriseCost4,
        TaskField::EnterpriseCost5,
        TaskField::EnterpriseCost6,
        TaskField::EnterpriseCost7,
        TaskField::EnterpriseCost8,
        TaskField::EnterpriseCost9,
        TaskField::EnterpriseCost10,
        TaskField::EnterpriseFlag1,
        TaskField::EnterpriseFlag2,
        TaskField::EnterpriseFlag3,
        TaskField::EnterpriseFlag4,
        TaskField::EnterpriseFlag5,
        TaskField::EnterpriseFlag6,
        TaskField::EnterpriseFlag7,
        TaskField::EnterpriseFlag8,
        TaskField::EnterpriseFlag9,
        TaskField::EnterpriseFlag10,
        TaskField::EnterpriseFlag11,
        TaskField::EnterpriseFlag12,
        TaskField::EnterpriseFlag13,
        TaskField::EnterpriseFlag14,
        TaskField::EnterpriseFlag15,
        TaskField::EnterpriseFlag16,
        TaskField::EnterpriseFlag17,
        TaskField::EnterpriseFlag18,
        TaskField::EnterpriseFlag19,
        TaskField::EnterpriseFlag20,
        TaskField::EnterpriseNumber1,
        TaskField::EnterpriseNumber2,
        TaskField::EnterpriseNumber3,
        TaskField::EnterpriseNumber4,
        TaskField::EnterpriseNumber5,
        TaskField::EnterpriseNumber6,
        TaskField::EnterpriseNumber7,
        TaskField::EnterpriseNumber8,
        TaskField::EnterpriseNumber9,
        TaskField::EnterpriseNumber10,
        TaskField::EnterpriseNumber11,
        TaskField::EnterpriseNumber12,
        TaskField::EnterpriseNumber13,
        TaskField::EnterpriseNumber14,
        TaskField::EnterpriseNumber15,
        TaskField::EnterpriseNumber16,
        TaskField::EnterpriseNumber17,
        TaskField::EnterpriseNumber18,
        TaskField::EnterpriseNumber19,
        TaskField::EnterpriseNumber20,
        TaskField::EnterpriseNumber21,
        TaskField::EnterpriseNumber22,
        TaskField::EnterpriseNumber23,
        TaskField::EnterpriseNumber24,
        TaskField::EnterpriseNumber25,
        TaskField::EnterpriseNumber26,
        TaskField::EnterpriseNumber27,
        TaskField::EnterpriseNumber28,
        TaskField::EnterpriseNumber29,
        TaskField::EnterpriseNumber30,
        TaskField::EnterpriseNumber31,
        TaskField::EnterpriseNumber32,
        TaskField::EnterpriseNumber33,
        TaskField::EnterpriseNumber34,
        TaskField::EnterpriseNumber35,
        TaskField::EnterpriseNumber36,
        TaskField::EnterpriseNumber37,
        TaskField::EnterpriseNumber38,
        TaskField::EnterpriseNumber39,
        TaskField::EnterpriseNumber40,
        TaskField::EnterpriseDuration1,
        TaskField::EnterpriseDuration2,
        TaskField::EnterpriseDuration3,
        TaskField::EnterpriseDuration4,
        TaskField::EnterpriseDuration5,
        TaskField::EnterpriseDuration6,
        TaskField::EnterpriseDuration7,
        TaskField::EnterpriseDuration8,
        TaskField::EnterpriseDuration9,
        TaskField::EnterpriseDuration10,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_type::FieldType;

    #[test]
    fn array_lengths_match_java() {
        assert_eq!(TaskFieldLists::CUSTOM_COST.len(), 10);
        assert_eq!(TaskFieldLists::CUSTOM_DATE.len(), 10);
        assert_eq!(TaskFieldLists::CUSTOM_DURATION.len(), 10);
        assert_eq!(TaskFieldLists::CUSTOM_DURATION_UNITS.len(), 10);
        assert_eq!(TaskFieldLists::CUSTOM_FLAG.len(), 20);
        assert_eq!(TaskFieldLists::CUSTOM_FINISH.len(), 10);
        assert_eq!(TaskFieldLists::CUSTOM_NUMBER.len(), 20);
        assert_eq!(TaskFieldLists::CUSTOM_START.len(), 10);
        assert_eq!(TaskFieldLists::CUSTOM_TEXT.len(), 30);
        assert_eq!(TaskFieldLists::CUSTOM_OUTLINE_CODE.len(), 10);
        assert_eq!(TaskFieldLists::CUSTOM_OUTLINE_CODE_INDEX.len(), 10);
        assert_eq!(TaskFieldLists::ENTERPRISE_CUSTOM_COST.len(), 10);
        assert_eq!(TaskFieldLists::ENTERPRISE_CUSTOM_DATE.len(), 30);
        assert_eq!(TaskFieldLists::ENTERPRISE_CUSTOM_DURATION.len(), 10);
        assert_eq!(TaskFieldLists::ENTERPRISE_CUSTOM_FLAG.len(), 20);
        assert_eq!(TaskFieldLists::ENTERPRISE_CUSTOM_NUMBER.len(), 40);
        assert_eq!(TaskFieldLists::ENTERPRISE_CUSTOM_TEXT.len(), 40);
        assert_eq!(TaskFieldLists::BASELINE_COSTS.len(), 10);
        assert_eq!(TaskFieldLists::BASELINE_DURATIONS.len(), 10);
        assert_eq!(TaskFieldLists::BASELINE_ESTIMATED_DURATIONS.len(), 10);
        assert_eq!(TaskFieldLists::BASELINE_STARTS.len(), 10);
        assert_eq!(TaskFieldLists::BASELINE_ESTIMATED_STARTS.len(), 10);
        assert_eq!(TaskFieldLists::BASELINE_FINISHES.len(), 10);
        assert_eq!(TaskFieldLists::BASELINE_ESTIMATED_FINISHES.len(), 10);
        assert_eq!(TaskFieldLists::BASELINE_WORKS.len(), 10);
        assert_eq!(TaskFieldLists::BASELINE_FIXED_COSTS.len(), 10);
        assert_eq!(TaskFieldLists::BASELINE_FIXED_COST_ACCRUALS.len(), 10);
        assert_eq!(TaskFieldLists::BASELINE_BUDGET_COSTS.len(), 10);
        assert_eq!(TaskFieldLists::BASELINE_BUDGET_WORKS.len(), 10);
        assert_eq!(TaskFieldLists::CUSTOM_FIELDS.len(), 280);
    }

    #[test]
    fn baseline_arrays_match_java() {
        assert_eq!(TaskFieldLists::BASELINE_COSTS[0].name(), "BASELINE1_COST");
        assert_eq!(TaskFieldLists::BASELINE_COSTS[9].name(), "BASELINE10_COST");
        assert_eq!(TaskFieldLists::BASELINE_DURATIONS[0].name(), "BASELINE1_DURATION");
        assert_eq!(TaskFieldLists::BASELINE_DURATIONS[9].name(), "BASELINE10_DURATION");
        assert_eq!(TaskFieldLists::BASELINE_ESTIMATED_DURATIONS[0].name(), "BASELINE1_ESTIMATED_DURATION");
        assert_eq!(TaskFieldLists::BASELINE_ESTIMATED_DURATIONS[9].name(), "BASELINE10_ESTIMATED_DURATION");
        assert_eq!(TaskFieldLists::BASELINE_STARTS[0].name(), "BASELINE1_START");
        assert_eq!(TaskFieldLists::BASELINE_STARTS[9].name(), "BASELINE10_START");
        assert_eq!(TaskFieldLists::BASELINE_ESTIMATED_STARTS[0].name(), "BASELINE1_ESTIMATED_START");
        assert_eq!(TaskFieldLists::BASELINE_ESTIMATED_STARTS[9].name(), "BASELINE10_ESTIMATED_START");
        assert_eq!(TaskFieldLists::BASELINE_FINISHES[0].name(), "BASELINE1_FINISH");
        assert_eq!(TaskFieldLists::BASELINE_FINISHES[9].name(), "BASELINE10_FINISH");
        assert_eq!(TaskFieldLists::BASELINE_ESTIMATED_FINISHES[0].name(), "BASELINE1_ESTIMATED_FINISH");
        assert_eq!(TaskFieldLists::BASELINE_ESTIMATED_FINISHES[9].name(), "BASELINE10_ESTIMATED_FINISH");
        assert_eq!(TaskFieldLists::BASELINE_WORKS[0].name(), "BASELINE1_WORK");
        assert_eq!(TaskFieldLists::BASELINE_WORKS[9].name(), "BASELINE10_WORK");
        assert_eq!(TaskFieldLists::BASELINE_FIXED_COSTS[0].name(), "BASELINE1_FIXED_COST");
        assert_eq!(TaskFieldLists::BASELINE_FIXED_COSTS[9].name(), "BASELINE10_FIXED_COST");
        assert_eq!(TaskFieldLists::BASELINE_FIXED_COST_ACCRUALS[0].name(), "BASELINE1_FIXED_COST_ACCRUAL");
        assert_eq!(TaskFieldLists::BASELINE_FIXED_COST_ACCRUALS[9].name(), "BASELINE10_FIXED_COST_ACCRUAL");
        assert_eq!(TaskFieldLists::BASELINE_BUDGET_COSTS[0].name(), "BASELINE1_BUDGET_COST");
        assert_eq!(TaskFieldLists::BASELINE_BUDGET_COSTS[9].name(), "BASELINE10_BUDGET_COST");
        assert_eq!(TaskFieldLists::BASELINE_BUDGET_WORKS[0].name(), "BASELINE1_BUDGET_WORK");
        assert_eq!(TaskFieldLists::BASELINE_BUDGET_WORKS[9].name(), "BASELINE10_BUDGET_WORK");
    }

    #[test]
    fn custom_fields_match_java_concatenation_order() {
        assert_eq!(TaskFieldLists::CUSTOM_FIELDS[0].name(), "TEXT1");
        assert_eq!(TaskFieldLists::CUSTOM_FIELDS[279].name(), "ENTERPRISE_DURATION10");
    }
}
