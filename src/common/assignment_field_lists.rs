//! Corresponds to Java: `org/mpxj/common/AssignmentFieldLists.java`

use crate::assignment_field::AssignmentField;

/// Collections of Assignment fields grouped logically.
///
/// Corresponds to Java: `public final class AssignmentFieldLists`
///
/// Porting notes:
/// - Each Java `AssignmentField[]` static array is ported as a fixed-size array constant `[AssignmentField; N]`, with elements and order exactly matching Java.
/// - Java's `CUSTOM_FIELDS` is built by a static initializer that `addAll`s 16 arrays in turn into an `ArrayList<AssignmentField>`; the Rust side expands it into a `&'static [AssignmentField]` constant in the same order as the Java `addAll` calls.
/// - In the Java source `CUSTOM_FIELDS` is declared after `ENTERPRISE_RESOURCE_MULTI_VALUE` and before `RAW_TIMEPHASED_BASELINE_WORKS`; on the Rust side `CUSTOM_FIELDS` is placed uniformly after all `AssignmentField[]` constants for readability, without changing the ordering semantics.
///
/// Skipped Java members: none.
pub struct AssignmentFieldLists;

impl AssignmentFieldLists {
    /// Corresponds to Java: `public static final AssignmentField[] BASELINE_COSTS`
    pub const BASELINE_COSTS: [AssignmentField; 10] = [
        AssignmentField::Baseline1Cost,
        AssignmentField::Baseline2Cost,
        AssignmentField::Baseline3Cost,
        AssignmentField::Baseline4Cost,
        AssignmentField::Baseline5Cost,
        AssignmentField::Baseline6Cost,
        AssignmentField::Baseline7Cost,
        AssignmentField::Baseline8Cost,
        AssignmentField::Baseline9Cost,
        AssignmentField::Baseline10Cost,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] BASELINE_WORKS`
    pub const BASELINE_WORKS: [AssignmentField; 10] = [
        AssignmentField::Baseline1Work,
        AssignmentField::Baseline2Work,
        AssignmentField::Baseline3Work,
        AssignmentField::Baseline4Work,
        AssignmentField::Baseline5Work,
        AssignmentField::Baseline6Work,
        AssignmentField::Baseline7Work,
        AssignmentField::Baseline8Work,
        AssignmentField::Baseline9Work,
        AssignmentField::Baseline10Work,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] BASELINE_MATERIALS`
    pub const BASELINE_MATERIALS: [AssignmentField; 10] = [
        AssignmentField::Baseline1Material,
        AssignmentField::Baseline2Material,
        AssignmentField::Baseline3Material,
        AssignmentField::Baseline4Material,
        AssignmentField::Baseline5Material,
        AssignmentField::Baseline6Material,
        AssignmentField::Baseline7Material,
        AssignmentField::Baseline8Material,
        AssignmentField::Baseline9Material,
        AssignmentField::Baseline10Material,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] BASELINE_STARTS`
    pub const BASELINE_STARTS: [AssignmentField; 10] = [
        AssignmentField::Baseline1Start,
        AssignmentField::Baseline2Start,
        AssignmentField::Baseline3Start,
        AssignmentField::Baseline4Start,
        AssignmentField::Baseline5Start,
        AssignmentField::Baseline6Start,
        AssignmentField::Baseline7Start,
        AssignmentField::Baseline8Start,
        AssignmentField::Baseline9Start,
        AssignmentField::Baseline10Start,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] BASELINE_FINISHES`
    pub const BASELINE_FINISHES: [AssignmentField; 10] = [
        AssignmentField::Baseline1Finish,
        AssignmentField::Baseline2Finish,
        AssignmentField::Baseline3Finish,
        AssignmentField::Baseline4Finish,
        AssignmentField::Baseline5Finish,
        AssignmentField::Baseline6Finish,
        AssignmentField::Baseline7Finish,
        AssignmentField::Baseline8Finish,
        AssignmentField::Baseline9Finish,
        AssignmentField::Baseline10Finish,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] BASELINE_BUDGET_COSTS`
    pub const BASELINE_BUDGET_COSTS: [AssignmentField; 10] = [
        AssignmentField::Baseline1BudgetCost,
        AssignmentField::Baseline2BudgetCost,
        AssignmentField::Baseline3BudgetCost,
        AssignmentField::Baseline4BudgetCost,
        AssignmentField::Baseline5BudgetCost,
        AssignmentField::Baseline6BudgetCost,
        AssignmentField::Baseline7BudgetCost,
        AssignmentField::Baseline8BudgetCost,
        AssignmentField::Baseline9BudgetCost,
        AssignmentField::Baseline10BudgetCost,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] BASELINE_BUDGET_WORKS`
    pub const BASELINE_BUDGET_WORKS: [AssignmentField; 10] = [
        AssignmentField::Baseline1BudgetWork,
        AssignmentField::Baseline2BudgetWork,
        AssignmentField::Baseline3BudgetWork,
        AssignmentField::Baseline4BudgetWork,
        AssignmentField::Baseline5BudgetWork,
        AssignmentField::Baseline6BudgetWork,
        AssignmentField::Baseline7BudgetWork,
        AssignmentField::Baseline8BudgetWork,
        AssignmentField::Baseline9BudgetWork,
        AssignmentField::Baseline10BudgetWork,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] CUSTOM_TEXT`
    pub const CUSTOM_TEXT: [AssignmentField; 30] = [
        AssignmentField::Text1,
        AssignmentField::Text2,
        AssignmentField::Text3,
        AssignmentField::Text4,
        AssignmentField::Text5,
        AssignmentField::Text6,
        AssignmentField::Text7,
        AssignmentField::Text8,
        AssignmentField::Text9,
        AssignmentField::Text10,
        AssignmentField::Text11,
        AssignmentField::Text12,
        AssignmentField::Text13,
        AssignmentField::Text14,
        AssignmentField::Text15,
        AssignmentField::Text16,
        AssignmentField::Text17,
        AssignmentField::Text18,
        AssignmentField::Text19,
        AssignmentField::Text20,
        AssignmentField::Text21,
        AssignmentField::Text22,
        AssignmentField::Text23,
        AssignmentField::Text24,
        AssignmentField::Text25,
        AssignmentField::Text26,
        AssignmentField::Text27,
        AssignmentField::Text28,
        AssignmentField::Text29,
        AssignmentField::Text30,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] CUSTOM_START`
    pub const CUSTOM_START: [AssignmentField; 10] = [
        AssignmentField::Start1,
        AssignmentField::Start2,
        AssignmentField::Start3,
        AssignmentField::Start4,
        AssignmentField::Start5,
        AssignmentField::Start6,
        AssignmentField::Start7,
        AssignmentField::Start8,
        AssignmentField::Start9,
        AssignmentField::Start10,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] CUSTOM_FINISH`
    pub const CUSTOM_FINISH: [AssignmentField; 10] = [
        AssignmentField::Finish1,
        AssignmentField::Finish2,
        AssignmentField::Finish3,
        AssignmentField::Finish4,
        AssignmentField::Finish5,
        AssignmentField::Finish6,
        AssignmentField::Finish7,
        AssignmentField::Finish8,
        AssignmentField::Finish9,
        AssignmentField::Finish10,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] CUSTOM_DATE`
    pub const CUSTOM_DATE: [AssignmentField; 10] = [
        AssignmentField::Date1,
        AssignmentField::Date2,
        AssignmentField::Date3,
        AssignmentField::Date4,
        AssignmentField::Date5,
        AssignmentField::Date6,
        AssignmentField::Date7,
        AssignmentField::Date8,
        AssignmentField::Date9,
        AssignmentField::Date10,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] CUSTOM_NUMBER`
    pub const CUSTOM_NUMBER: [AssignmentField; 20] = [
        AssignmentField::Number1,
        AssignmentField::Number2,
        AssignmentField::Number3,
        AssignmentField::Number4,
        AssignmentField::Number5,
        AssignmentField::Number6,
        AssignmentField::Number7,
        AssignmentField::Number8,
        AssignmentField::Number9,
        AssignmentField::Number10,
        AssignmentField::Number11,
        AssignmentField::Number12,
        AssignmentField::Number13,
        AssignmentField::Number14,
        AssignmentField::Number15,
        AssignmentField::Number16,
        AssignmentField::Number17,
        AssignmentField::Number18,
        AssignmentField::Number19,
        AssignmentField::Number20,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] CUSTOM_DURATION`
    pub const CUSTOM_DURATION: [AssignmentField; 10] = [
        AssignmentField::Duration1,
        AssignmentField::Duration2,
        AssignmentField::Duration3,
        AssignmentField::Duration4,
        AssignmentField::Duration5,
        AssignmentField::Duration6,
        AssignmentField::Duration7,
        AssignmentField::Duration8,
        AssignmentField::Duration9,
        AssignmentField::Duration10,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] CUSTOM_DURATION_UNITS`
    pub const CUSTOM_DURATION_UNITS: [AssignmentField; 10] = [
        AssignmentField::Duration1Units,
        AssignmentField::Duration2Units,
        AssignmentField::Duration3Units,
        AssignmentField::Duration4Units,
        AssignmentField::Duration5Units,
        AssignmentField::Duration6Units,
        AssignmentField::Duration7Units,
        AssignmentField::Duration8Units,
        AssignmentField::Duration9Units,
        AssignmentField::Duration10Units,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] CUSTOM_COST`
    pub const CUSTOM_COST: [AssignmentField; 10] = [
        AssignmentField::Cost1,
        AssignmentField::Cost2,
        AssignmentField::Cost3,
        AssignmentField::Cost4,
        AssignmentField::Cost5,
        AssignmentField::Cost6,
        AssignmentField::Cost7,
        AssignmentField::Cost8,
        AssignmentField::Cost9,
        AssignmentField::Cost10,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] CUSTOM_FLAG`
    pub const CUSTOM_FLAG: [AssignmentField; 20] = [
        AssignmentField::Flag1,
        AssignmentField::Flag2,
        AssignmentField::Flag3,
        AssignmentField::Flag4,
        AssignmentField::Flag5,
        AssignmentField::Flag6,
        AssignmentField::Flag7,
        AssignmentField::Flag8,
        AssignmentField::Flag9,
        AssignmentField::Flag10,
        AssignmentField::Flag11,
        AssignmentField::Flag12,
        AssignmentField::Flag13,
        AssignmentField::Flag14,
        AssignmentField::Flag15,
        AssignmentField::Flag16,
        AssignmentField::Flag17,
        AssignmentField::Flag18,
        AssignmentField::Flag19,
        AssignmentField::Flag20,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] ENTERPRISE_CUSTOM_COST`
    pub const ENTERPRISE_CUSTOM_COST: [AssignmentField; 10] = [
        AssignmentField::EnterpriseCost1,
        AssignmentField::EnterpriseCost2,
        AssignmentField::EnterpriseCost3,
        AssignmentField::EnterpriseCost4,
        AssignmentField::EnterpriseCost5,
        AssignmentField::EnterpriseCost6,
        AssignmentField::EnterpriseCost7,
        AssignmentField::EnterpriseCost8,
        AssignmentField::EnterpriseCost9,
        AssignmentField::EnterpriseCost10,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] ENTERPRISE_CUSTOM_DATE`
    pub const ENTERPRISE_CUSTOM_DATE: [AssignmentField; 30] = [
        AssignmentField::EnterpriseDate1,
        AssignmentField::EnterpriseDate2,
        AssignmentField::EnterpriseDate3,
        AssignmentField::EnterpriseDate4,
        AssignmentField::EnterpriseDate5,
        AssignmentField::EnterpriseDate6,
        AssignmentField::EnterpriseDate7,
        AssignmentField::EnterpriseDate8,
        AssignmentField::EnterpriseDate9,
        AssignmentField::EnterpriseDate10,
        AssignmentField::EnterpriseDate11,
        AssignmentField::EnterpriseDate12,
        AssignmentField::EnterpriseDate13,
        AssignmentField::EnterpriseDate14,
        AssignmentField::EnterpriseDate15,
        AssignmentField::EnterpriseDate16,
        AssignmentField::EnterpriseDate17,
        AssignmentField::EnterpriseDate18,
        AssignmentField::EnterpriseDate19,
        AssignmentField::EnterpriseDate20,
        AssignmentField::EnterpriseDate21,
        AssignmentField::EnterpriseDate22,
        AssignmentField::EnterpriseDate23,
        AssignmentField::EnterpriseDate24,
        AssignmentField::EnterpriseDate25,
        AssignmentField::EnterpriseDate26,
        AssignmentField::EnterpriseDate27,
        AssignmentField::EnterpriseDate28,
        AssignmentField::EnterpriseDate29,
        AssignmentField::EnterpriseDate30,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] ENTERPRISE_CUSTOM_DURATION`
    pub const ENTERPRISE_CUSTOM_DURATION: [AssignmentField; 10] = [
        AssignmentField::EnterpriseDuration1,
        AssignmentField::EnterpriseDuration2,
        AssignmentField::EnterpriseDuration3,
        AssignmentField::EnterpriseDuration4,
        AssignmentField::EnterpriseDuration5,
        AssignmentField::EnterpriseDuration6,
        AssignmentField::EnterpriseDuration7,
        AssignmentField::EnterpriseDuration8,
        AssignmentField::EnterpriseDuration9,
        AssignmentField::EnterpriseDuration10,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] ENTERPRISE_CUSTOM_FLAG`
    pub const ENTERPRISE_CUSTOM_FLAG: [AssignmentField; 20] = [
        AssignmentField::EnterpriseFlag1,
        AssignmentField::EnterpriseFlag2,
        AssignmentField::EnterpriseFlag3,
        AssignmentField::EnterpriseFlag4,
        AssignmentField::EnterpriseFlag5,
        AssignmentField::EnterpriseFlag6,
        AssignmentField::EnterpriseFlag7,
        AssignmentField::EnterpriseFlag8,
        AssignmentField::EnterpriseFlag9,
        AssignmentField::EnterpriseFlag10,
        AssignmentField::EnterpriseFlag11,
        AssignmentField::EnterpriseFlag12,
        AssignmentField::EnterpriseFlag13,
        AssignmentField::EnterpriseFlag14,
        AssignmentField::EnterpriseFlag15,
        AssignmentField::EnterpriseFlag16,
        AssignmentField::EnterpriseFlag17,
        AssignmentField::EnterpriseFlag18,
        AssignmentField::EnterpriseFlag19,
        AssignmentField::EnterpriseFlag20,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] ENTERPRISE_CUSTOM_NUMBER`
    pub const ENTERPRISE_CUSTOM_NUMBER: [AssignmentField; 40] = [
        AssignmentField::EnterpriseNumber1,
        AssignmentField::EnterpriseNumber2,
        AssignmentField::EnterpriseNumber3,
        AssignmentField::EnterpriseNumber4,
        AssignmentField::EnterpriseNumber5,
        AssignmentField::EnterpriseNumber6,
        AssignmentField::EnterpriseNumber7,
        AssignmentField::EnterpriseNumber8,
        AssignmentField::EnterpriseNumber9,
        AssignmentField::EnterpriseNumber10,
        AssignmentField::EnterpriseNumber11,
        AssignmentField::EnterpriseNumber12,
        AssignmentField::EnterpriseNumber13,
        AssignmentField::EnterpriseNumber14,
        AssignmentField::EnterpriseNumber15,
        AssignmentField::EnterpriseNumber16,
        AssignmentField::EnterpriseNumber17,
        AssignmentField::EnterpriseNumber18,
        AssignmentField::EnterpriseNumber19,
        AssignmentField::EnterpriseNumber20,
        AssignmentField::EnterpriseNumber21,
        AssignmentField::EnterpriseNumber22,
        AssignmentField::EnterpriseNumber23,
        AssignmentField::EnterpriseNumber24,
        AssignmentField::EnterpriseNumber25,
        AssignmentField::EnterpriseNumber26,
        AssignmentField::EnterpriseNumber27,
        AssignmentField::EnterpriseNumber28,
        AssignmentField::EnterpriseNumber29,
        AssignmentField::EnterpriseNumber30,
        AssignmentField::EnterpriseNumber31,
        AssignmentField::EnterpriseNumber32,
        AssignmentField::EnterpriseNumber33,
        AssignmentField::EnterpriseNumber34,
        AssignmentField::EnterpriseNumber35,
        AssignmentField::EnterpriseNumber36,
        AssignmentField::EnterpriseNumber37,
        AssignmentField::EnterpriseNumber38,
        AssignmentField::EnterpriseNumber39,
        AssignmentField::EnterpriseNumber40,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] ENTERPRISE_CUSTOM_TEXT`
    pub const ENTERPRISE_CUSTOM_TEXT: [AssignmentField; 40] = [
        AssignmentField::EnterpriseText1,
        AssignmentField::EnterpriseText2,
        AssignmentField::EnterpriseText3,
        AssignmentField::EnterpriseText4,
        AssignmentField::EnterpriseText5,
        AssignmentField::EnterpriseText6,
        AssignmentField::EnterpriseText7,
        AssignmentField::EnterpriseText8,
        AssignmentField::EnterpriseText9,
        AssignmentField::EnterpriseText10,
        AssignmentField::EnterpriseText11,
        AssignmentField::EnterpriseText12,
        AssignmentField::EnterpriseText13,
        AssignmentField::EnterpriseText14,
        AssignmentField::EnterpriseText15,
        AssignmentField::EnterpriseText16,
        AssignmentField::EnterpriseText17,
        AssignmentField::EnterpriseText18,
        AssignmentField::EnterpriseText19,
        AssignmentField::EnterpriseText20,
        AssignmentField::EnterpriseText21,
        AssignmentField::EnterpriseText22,
        AssignmentField::EnterpriseText23,
        AssignmentField::EnterpriseText24,
        AssignmentField::EnterpriseText25,
        AssignmentField::EnterpriseText26,
        AssignmentField::EnterpriseText27,
        AssignmentField::EnterpriseText28,
        AssignmentField::EnterpriseText29,
        AssignmentField::EnterpriseText30,
        AssignmentField::EnterpriseText31,
        AssignmentField::EnterpriseText32,
        AssignmentField::EnterpriseText33,
        AssignmentField::EnterpriseText34,
        AssignmentField::EnterpriseText35,
        AssignmentField::EnterpriseText36,
        AssignmentField::EnterpriseText37,
        AssignmentField::EnterpriseText38,
        AssignmentField::EnterpriseText39,
        AssignmentField::EnterpriseText40,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] ENTERPRISE_RESOURCE_MULTI_VALUE`
    pub const ENTERPRISE_RESOURCE_MULTI_VALUE: [AssignmentField; 10] = [
        AssignmentField::EnterpriseResourceMultiValue20,
        AssignmentField::EnterpriseResourceMultiValue21,
        AssignmentField::EnterpriseResourceMultiValue22,
        AssignmentField::EnterpriseResourceMultiValue23,
        AssignmentField::EnterpriseResourceMultiValue24,
        AssignmentField::EnterpriseResourceMultiValue25,
        AssignmentField::EnterpriseResourceMultiValue26,
        AssignmentField::EnterpriseResourceMultiValue27,
        AssignmentField::EnterpriseResourceMultiValue28,
        AssignmentField::EnterpriseResourceMultiValue29,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] ENTERPRISE_RESOURCE_OUTLINE_CODE`
    pub const ENTERPRISE_RESOURCE_OUTLINE_CODE: [AssignmentField; 29] = [
        AssignmentField::EnterpriseResourceOutlineCode1,
        AssignmentField::EnterpriseResourceOutlineCode2,
        AssignmentField::EnterpriseResourceOutlineCode3,
        AssignmentField::EnterpriseResourceOutlineCode4,
        AssignmentField::EnterpriseResourceOutlineCode5,
        AssignmentField::EnterpriseResourceOutlineCode6,
        AssignmentField::EnterpriseResourceOutlineCode7,
        AssignmentField::EnterpriseResourceOutlineCode8,
        AssignmentField::EnterpriseResourceOutlineCode9,
        AssignmentField::EnterpriseResourceOutlineCode10,
        AssignmentField::EnterpriseResourceOutlineCode11,
        AssignmentField::EnterpriseResourceOutlineCode12,
        AssignmentField::EnterpriseResourceOutlineCode13,
        AssignmentField::EnterpriseResourceOutlineCode14,
        AssignmentField::EnterpriseResourceOutlineCode15,
        AssignmentField::EnterpriseResourceOutlineCode16,
        AssignmentField::EnterpriseResourceOutlineCode17,
        AssignmentField::EnterpriseResourceOutlineCode18,
        AssignmentField::EnterpriseResourceOutlineCode19,
        AssignmentField::EnterpriseResourceOutlineCode20,
        AssignmentField::EnterpriseResourceOutlineCode21,
        AssignmentField::EnterpriseResourceOutlineCode22,
        AssignmentField::EnterpriseResourceOutlineCode23,
        AssignmentField::EnterpriseResourceOutlineCode24,
        AssignmentField::EnterpriseResourceOutlineCode25,
        AssignmentField::EnterpriseResourceOutlineCode26,
        AssignmentField::EnterpriseResourceOutlineCode27,
        AssignmentField::EnterpriseResourceOutlineCode28,
        AssignmentField::EnterpriseResourceOutlineCode29,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] RAW_TIMEPHASED_BASELINE_WORKS`
    pub const RAW_TIMEPHASED_BASELINE_WORKS: [AssignmentField; 11] = [
        AssignmentField::RawTimephasedBaselineWork,
        AssignmentField::RawTimephasedBaseline1Work,
        AssignmentField::RawTimephasedBaseline2Work,
        AssignmentField::RawTimephasedBaseline3Work,
        AssignmentField::RawTimephasedBaseline4Work,
        AssignmentField::RawTimephasedBaseline5Work,
        AssignmentField::RawTimephasedBaseline6Work,
        AssignmentField::RawTimephasedBaseline7Work,
        AssignmentField::RawTimephasedBaseline8Work,
        AssignmentField::RawTimephasedBaseline9Work,
        AssignmentField::RawTimephasedBaseline10Work,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] RAW_TIMEPHASED_BASELINE_COSTS`
    pub const RAW_TIMEPHASED_BASELINE_COSTS: [AssignmentField; 11] = [
        AssignmentField::RawTimephasedBaselineCost,
        AssignmentField::RawTimephasedBaseline1Cost,
        AssignmentField::RawTimephasedBaseline2Cost,
        AssignmentField::RawTimephasedBaseline3Cost,
        AssignmentField::RawTimephasedBaseline4Cost,
        AssignmentField::RawTimephasedBaseline5Cost,
        AssignmentField::RawTimephasedBaseline6Cost,
        AssignmentField::RawTimephasedBaseline7Cost,
        AssignmentField::RawTimephasedBaseline8Cost,
        AssignmentField::RawTimephasedBaseline9Cost,
        AssignmentField::RawTimephasedBaseline10Cost,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] RAW_TIMEPHASED_BASELINE_BUDGET_WORKS`
    pub const RAW_TIMEPHASED_BASELINE_BUDGET_WORKS: [AssignmentField; 11] = [
        AssignmentField::RawTimephasedBaselineBudgetWork,
        AssignmentField::RawTimephasedBaseline1BudgetWork,
        AssignmentField::RawTimephasedBaseline2BudgetWork,
        AssignmentField::RawTimephasedBaseline3BudgetWork,
        AssignmentField::RawTimephasedBaseline4BudgetWork,
        AssignmentField::RawTimephasedBaseline5BudgetWork,
        AssignmentField::RawTimephasedBaseline6BudgetWork,
        AssignmentField::RawTimephasedBaseline7BudgetWork,
        AssignmentField::RawTimephasedBaseline8BudgetWork,
        AssignmentField::RawTimephasedBaseline9BudgetWork,
        AssignmentField::RawTimephasedBaseline10BudgetWork,
    ];

    /// Corresponds to Java: `public static final AssignmentField[] RAW_TIMEPHASED_BASELINE_BUDGET_COSTS`
    pub const RAW_TIMEPHASED_BASELINE_BUDGET_COSTS: [AssignmentField; 11] = [
        AssignmentField::RawTimephasedBaselineBudgetCost,
        AssignmentField::RawTimephasedBaseline1BudgetCost,
        AssignmentField::RawTimephasedBaseline2BudgetCost,
        AssignmentField::RawTimephasedBaseline3BudgetCost,
        AssignmentField::RawTimephasedBaseline4BudgetCost,
        AssignmentField::RawTimephasedBaseline5BudgetCost,
        AssignmentField::RawTimephasedBaseline6BudgetCost,
        AssignmentField::RawTimephasedBaseline7BudgetCost,
        AssignmentField::RawTimephasedBaseline8BudgetCost,
        AssignmentField::RawTimephasedBaseline9BudgetCost,
        AssignmentField::RawTimephasedBaseline10BudgetCost,
    ];

    /// Corresponds to Java: `public static final List<AssignmentField> CUSTOM_FIELDS`
    ///
    /// Java concatenates via `addAll` in the static initializer in the order `CUSTOM_TEXT`, `CUSTOM_DATE`, `CUSTOM_START`, `CUSTOM_FINISH`, `CUSTOM_COST`, `CUSTOM_FLAG`, `CUSTOM_NUMBER`, `CUSTOM_DURATION`, `ENTERPRISE_CUSTOM_TEXT`, `ENTERPRISE_CUSTOM_DATE`, `ENTERPRISE_CUSTOM_COST`, `ENTERPRISE_CUSTOM_FLAG`, `ENTERPRISE_CUSTOM_NUMBER`, `ENTERPRISE_CUSTOM_DURATION`, `ENTERPRISE_RESOURCE_OUTLINE_CODE`, `ENTERPRISE_RESOURCE_MULTI_VALUE`; the same order is kept here.
    pub const CUSTOM_FIELDS: &'static [AssignmentField] = &[
        AssignmentField::Text1,
        AssignmentField::Text2,
        AssignmentField::Text3,
        AssignmentField::Text4,
        AssignmentField::Text5,
        AssignmentField::Text6,
        AssignmentField::Text7,
        AssignmentField::Text8,
        AssignmentField::Text9,
        AssignmentField::Text10,
        AssignmentField::Text11,
        AssignmentField::Text12,
        AssignmentField::Text13,
        AssignmentField::Text14,
        AssignmentField::Text15,
        AssignmentField::Text16,
        AssignmentField::Text17,
        AssignmentField::Text18,
        AssignmentField::Text19,
        AssignmentField::Text20,
        AssignmentField::Text21,
        AssignmentField::Text22,
        AssignmentField::Text23,
        AssignmentField::Text24,
        AssignmentField::Text25,
        AssignmentField::Text26,
        AssignmentField::Text27,
        AssignmentField::Text28,
        AssignmentField::Text29,
        AssignmentField::Text30,
        AssignmentField::Date1,
        AssignmentField::Date2,
        AssignmentField::Date3,
        AssignmentField::Date4,
        AssignmentField::Date5,
        AssignmentField::Date6,
        AssignmentField::Date7,
        AssignmentField::Date8,
        AssignmentField::Date9,
        AssignmentField::Date10,
        AssignmentField::Start1,
        AssignmentField::Start2,
        AssignmentField::Start3,
        AssignmentField::Start4,
        AssignmentField::Start5,
        AssignmentField::Start6,
        AssignmentField::Start7,
        AssignmentField::Start8,
        AssignmentField::Start9,
        AssignmentField::Start10,
        AssignmentField::Finish1,
        AssignmentField::Finish2,
        AssignmentField::Finish3,
        AssignmentField::Finish4,
        AssignmentField::Finish5,
        AssignmentField::Finish6,
        AssignmentField::Finish7,
        AssignmentField::Finish8,
        AssignmentField::Finish9,
        AssignmentField::Finish10,
        AssignmentField::Cost1,
        AssignmentField::Cost2,
        AssignmentField::Cost3,
        AssignmentField::Cost4,
        AssignmentField::Cost5,
        AssignmentField::Cost6,
        AssignmentField::Cost7,
        AssignmentField::Cost8,
        AssignmentField::Cost9,
        AssignmentField::Cost10,
        AssignmentField::Flag1,
        AssignmentField::Flag2,
        AssignmentField::Flag3,
        AssignmentField::Flag4,
        AssignmentField::Flag5,
        AssignmentField::Flag6,
        AssignmentField::Flag7,
        AssignmentField::Flag8,
        AssignmentField::Flag9,
        AssignmentField::Flag10,
        AssignmentField::Flag11,
        AssignmentField::Flag12,
        AssignmentField::Flag13,
        AssignmentField::Flag14,
        AssignmentField::Flag15,
        AssignmentField::Flag16,
        AssignmentField::Flag17,
        AssignmentField::Flag18,
        AssignmentField::Flag19,
        AssignmentField::Flag20,
        AssignmentField::Number1,
        AssignmentField::Number2,
        AssignmentField::Number3,
        AssignmentField::Number4,
        AssignmentField::Number5,
        AssignmentField::Number6,
        AssignmentField::Number7,
        AssignmentField::Number8,
        AssignmentField::Number9,
        AssignmentField::Number10,
        AssignmentField::Number11,
        AssignmentField::Number12,
        AssignmentField::Number13,
        AssignmentField::Number14,
        AssignmentField::Number15,
        AssignmentField::Number16,
        AssignmentField::Number17,
        AssignmentField::Number18,
        AssignmentField::Number19,
        AssignmentField::Number20,
        AssignmentField::Duration1,
        AssignmentField::Duration2,
        AssignmentField::Duration3,
        AssignmentField::Duration4,
        AssignmentField::Duration5,
        AssignmentField::Duration6,
        AssignmentField::Duration7,
        AssignmentField::Duration8,
        AssignmentField::Duration9,
        AssignmentField::Duration10,
        AssignmentField::EnterpriseText1,
        AssignmentField::EnterpriseText2,
        AssignmentField::EnterpriseText3,
        AssignmentField::EnterpriseText4,
        AssignmentField::EnterpriseText5,
        AssignmentField::EnterpriseText6,
        AssignmentField::EnterpriseText7,
        AssignmentField::EnterpriseText8,
        AssignmentField::EnterpriseText9,
        AssignmentField::EnterpriseText10,
        AssignmentField::EnterpriseText11,
        AssignmentField::EnterpriseText12,
        AssignmentField::EnterpriseText13,
        AssignmentField::EnterpriseText14,
        AssignmentField::EnterpriseText15,
        AssignmentField::EnterpriseText16,
        AssignmentField::EnterpriseText17,
        AssignmentField::EnterpriseText18,
        AssignmentField::EnterpriseText19,
        AssignmentField::EnterpriseText20,
        AssignmentField::EnterpriseText21,
        AssignmentField::EnterpriseText22,
        AssignmentField::EnterpriseText23,
        AssignmentField::EnterpriseText24,
        AssignmentField::EnterpriseText25,
        AssignmentField::EnterpriseText26,
        AssignmentField::EnterpriseText27,
        AssignmentField::EnterpriseText28,
        AssignmentField::EnterpriseText29,
        AssignmentField::EnterpriseText30,
        AssignmentField::EnterpriseText31,
        AssignmentField::EnterpriseText32,
        AssignmentField::EnterpriseText33,
        AssignmentField::EnterpriseText34,
        AssignmentField::EnterpriseText35,
        AssignmentField::EnterpriseText36,
        AssignmentField::EnterpriseText37,
        AssignmentField::EnterpriseText38,
        AssignmentField::EnterpriseText39,
        AssignmentField::EnterpriseText40,
        AssignmentField::EnterpriseDate1,
        AssignmentField::EnterpriseDate2,
        AssignmentField::EnterpriseDate3,
        AssignmentField::EnterpriseDate4,
        AssignmentField::EnterpriseDate5,
        AssignmentField::EnterpriseDate6,
        AssignmentField::EnterpriseDate7,
        AssignmentField::EnterpriseDate8,
        AssignmentField::EnterpriseDate9,
        AssignmentField::EnterpriseDate10,
        AssignmentField::EnterpriseDate11,
        AssignmentField::EnterpriseDate12,
        AssignmentField::EnterpriseDate13,
        AssignmentField::EnterpriseDate14,
        AssignmentField::EnterpriseDate15,
        AssignmentField::EnterpriseDate16,
        AssignmentField::EnterpriseDate17,
        AssignmentField::EnterpriseDate18,
        AssignmentField::EnterpriseDate19,
        AssignmentField::EnterpriseDate20,
        AssignmentField::EnterpriseDate21,
        AssignmentField::EnterpriseDate22,
        AssignmentField::EnterpriseDate23,
        AssignmentField::EnterpriseDate24,
        AssignmentField::EnterpriseDate25,
        AssignmentField::EnterpriseDate26,
        AssignmentField::EnterpriseDate27,
        AssignmentField::EnterpriseDate28,
        AssignmentField::EnterpriseDate29,
        AssignmentField::EnterpriseDate30,
        AssignmentField::EnterpriseCost1,
        AssignmentField::EnterpriseCost2,
        AssignmentField::EnterpriseCost3,
        AssignmentField::EnterpriseCost4,
        AssignmentField::EnterpriseCost5,
        AssignmentField::EnterpriseCost6,
        AssignmentField::EnterpriseCost7,
        AssignmentField::EnterpriseCost8,
        AssignmentField::EnterpriseCost9,
        AssignmentField::EnterpriseCost10,
        AssignmentField::EnterpriseFlag1,
        AssignmentField::EnterpriseFlag2,
        AssignmentField::EnterpriseFlag3,
        AssignmentField::EnterpriseFlag4,
        AssignmentField::EnterpriseFlag5,
        AssignmentField::EnterpriseFlag6,
        AssignmentField::EnterpriseFlag7,
        AssignmentField::EnterpriseFlag8,
        AssignmentField::EnterpriseFlag9,
        AssignmentField::EnterpriseFlag10,
        AssignmentField::EnterpriseFlag11,
        AssignmentField::EnterpriseFlag12,
        AssignmentField::EnterpriseFlag13,
        AssignmentField::EnterpriseFlag14,
        AssignmentField::EnterpriseFlag15,
        AssignmentField::EnterpriseFlag16,
        AssignmentField::EnterpriseFlag17,
        AssignmentField::EnterpriseFlag18,
        AssignmentField::EnterpriseFlag19,
        AssignmentField::EnterpriseFlag20,
        AssignmentField::EnterpriseNumber1,
        AssignmentField::EnterpriseNumber2,
        AssignmentField::EnterpriseNumber3,
        AssignmentField::EnterpriseNumber4,
        AssignmentField::EnterpriseNumber5,
        AssignmentField::EnterpriseNumber6,
        AssignmentField::EnterpriseNumber7,
        AssignmentField::EnterpriseNumber8,
        AssignmentField::EnterpriseNumber9,
        AssignmentField::EnterpriseNumber10,
        AssignmentField::EnterpriseNumber11,
        AssignmentField::EnterpriseNumber12,
        AssignmentField::EnterpriseNumber13,
        AssignmentField::EnterpriseNumber14,
        AssignmentField::EnterpriseNumber15,
        AssignmentField::EnterpriseNumber16,
        AssignmentField::EnterpriseNumber17,
        AssignmentField::EnterpriseNumber18,
        AssignmentField::EnterpriseNumber19,
        AssignmentField::EnterpriseNumber20,
        AssignmentField::EnterpriseNumber21,
        AssignmentField::EnterpriseNumber22,
        AssignmentField::EnterpriseNumber23,
        AssignmentField::EnterpriseNumber24,
        AssignmentField::EnterpriseNumber25,
        AssignmentField::EnterpriseNumber26,
        AssignmentField::EnterpriseNumber27,
        AssignmentField::EnterpriseNumber28,
        AssignmentField::EnterpriseNumber29,
        AssignmentField::EnterpriseNumber30,
        AssignmentField::EnterpriseNumber31,
        AssignmentField::EnterpriseNumber32,
        AssignmentField::EnterpriseNumber33,
        AssignmentField::EnterpriseNumber34,
        AssignmentField::EnterpriseNumber35,
        AssignmentField::EnterpriseNumber36,
        AssignmentField::EnterpriseNumber37,
        AssignmentField::EnterpriseNumber38,
        AssignmentField::EnterpriseNumber39,
        AssignmentField::EnterpriseNumber40,
        AssignmentField::EnterpriseDuration1,
        AssignmentField::EnterpriseDuration2,
        AssignmentField::EnterpriseDuration3,
        AssignmentField::EnterpriseDuration4,
        AssignmentField::EnterpriseDuration5,
        AssignmentField::EnterpriseDuration6,
        AssignmentField::EnterpriseDuration7,
        AssignmentField::EnterpriseDuration8,
        AssignmentField::EnterpriseDuration9,
        AssignmentField::EnterpriseDuration10,
        AssignmentField::EnterpriseResourceOutlineCode1,
        AssignmentField::EnterpriseResourceOutlineCode2,
        AssignmentField::EnterpriseResourceOutlineCode3,
        AssignmentField::EnterpriseResourceOutlineCode4,
        AssignmentField::EnterpriseResourceOutlineCode5,
        AssignmentField::EnterpriseResourceOutlineCode6,
        AssignmentField::EnterpriseResourceOutlineCode7,
        AssignmentField::EnterpriseResourceOutlineCode8,
        AssignmentField::EnterpriseResourceOutlineCode9,
        AssignmentField::EnterpriseResourceOutlineCode10,
        AssignmentField::EnterpriseResourceOutlineCode11,
        AssignmentField::EnterpriseResourceOutlineCode12,
        AssignmentField::EnterpriseResourceOutlineCode13,
        AssignmentField::EnterpriseResourceOutlineCode14,
        AssignmentField::EnterpriseResourceOutlineCode15,
        AssignmentField::EnterpriseResourceOutlineCode16,
        AssignmentField::EnterpriseResourceOutlineCode17,
        AssignmentField::EnterpriseResourceOutlineCode18,
        AssignmentField::EnterpriseResourceOutlineCode19,
        AssignmentField::EnterpriseResourceOutlineCode20,
        AssignmentField::EnterpriseResourceOutlineCode21,
        AssignmentField::EnterpriseResourceOutlineCode22,
        AssignmentField::EnterpriseResourceOutlineCode23,
        AssignmentField::EnterpriseResourceOutlineCode24,
        AssignmentField::EnterpriseResourceOutlineCode25,
        AssignmentField::EnterpriseResourceOutlineCode26,
        AssignmentField::EnterpriseResourceOutlineCode27,
        AssignmentField::EnterpriseResourceOutlineCode28,
        AssignmentField::EnterpriseResourceOutlineCode29,
        AssignmentField::EnterpriseResourceMultiValue20,
        AssignmentField::EnterpriseResourceMultiValue21,
        AssignmentField::EnterpriseResourceMultiValue22,
        AssignmentField::EnterpriseResourceMultiValue23,
        AssignmentField::EnterpriseResourceMultiValue24,
        AssignmentField::EnterpriseResourceMultiValue25,
        AssignmentField::EnterpriseResourceMultiValue26,
        AssignmentField::EnterpriseResourceMultiValue27,
        AssignmentField::EnterpriseResourceMultiValue28,
        AssignmentField::EnterpriseResourceMultiValue29,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_type::FieldType;

    #[test]
    fn array_lengths_match_java() {
        assert_eq!(AssignmentFieldLists::BASELINE_COSTS.len(), 10);
        assert_eq!(AssignmentFieldLists::BASELINE_WORKS.len(), 10);
        assert_eq!(AssignmentFieldLists::BASELINE_MATERIALS.len(), 10);
        assert_eq!(AssignmentFieldLists::BASELINE_STARTS.len(), 10);
        assert_eq!(AssignmentFieldLists::BASELINE_FINISHES.len(), 10);
        assert_eq!(AssignmentFieldLists::BASELINE_BUDGET_COSTS.len(), 10);
        assert_eq!(AssignmentFieldLists::BASELINE_BUDGET_WORKS.len(), 10);
        assert_eq!(AssignmentFieldLists::CUSTOM_TEXT.len(), 30);
        assert_eq!(AssignmentFieldLists::CUSTOM_START.len(), 10);
        assert_eq!(AssignmentFieldLists::CUSTOM_FINISH.len(), 10);
        assert_eq!(AssignmentFieldLists::CUSTOM_DATE.len(), 10);
        assert_eq!(AssignmentFieldLists::CUSTOM_NUMBER.len(), 20);
        assert_eq!(AssignmentFieldLists::CUSTOM_DURATION.len(), 10);
        assert_eq!(AssignmentFieldLists::CUSTOM_DURATION_UNITS.len(), 10);
        assert_eq!(AssignmentFieldLists::CUSTOM_COST.len(), 10);
        assert_eq!(AssignmentFieldLists::CUSTOM_FLAG.len(), 20);
        assert_eq!(AssignmentFieldLists::ENTERPRISE_CUSTOM_COST.len(), 10);
        assert_eq!(AssignmentFieldLists::ENTERPRISE_CUSTOM_DATE.len(), 30);
        assert_eq!(AssignmentFieldLists::ENTERPRISE_CUSTOM_DURATION.len(), 10);
        assert_eq!(AssignmentFieldLists::ENTERPRISE_CUSTOM_FLAG.len(), 20);
        assert_eq!(AssignmentFieldLists::ENTERPRISE_CUSTOM_NUMBER.len(), 40);
        assert_eq!(AssignmentFieldLists::ENTERPRISE_CUSTOM_TEXT.len(), 40);
        assert_eq!(AssignmentFieldLists::ENTERPRISE_RESOURCE_MULTI_VALUE.len(), 10);
        assert_eq!(AssignmentFieldLists::ENTERPRISE_RESOURCE_OUTLINE_CODE.len(), 29);
        assert_eq!(AssignmentFieldLists::RAW_TIMEPHASED_BASELINE_WORKS.len(), 11);
        assert_eq!(AssignmentFieldLists::RAW_TIMEPHASED_BASELINE_COSTS.len(), 11);
        assert_eq!(AssignmentFieldLists::RAW_TIMEPHASED_BASELINE_BUDGET_WORKS.len(), 11);
        assert_eq!(AssignmentFieldLists::RAW_TIMEPHASED_BASELINE_BUDGET_COSTS.len(), 11);
        assert_eq!(AssignmentFieldLists::CUSTOM_FIELDS.len(), 309);
    }

    #[test]
    fn baseline_arrays_match_java() {
        assert_eq!(AssignmentFieldLists::BASELINE_COSTS[0].name(), "BASELINE1_COST");
        assert_eq!(AssignmentFieldLists::BASELINE_COSTS[9].name(), "BASELINE10_COST");
        assert_eq!(AssignmentFieldLists::BASELINE_WORKS[0].name(), "BASELINE1_WORK");
        assert_eq!(AssignmentFieldLists::BASELINE_WORKS[9].name(), "BASELINE10_WORK");
        assert_eq!(AssignmentFieldLists::BASELINE_MATERIALS[0].name(), "BASELINE1_MATERIAL");
        assert_eq!(AssignmentFieldLists::BASELINE_MATERIALS[9].name(), "BASELINE10_MATERIAL");
        assert_eq!(AssignmentFieldLists::BASELINE_STARTS[0].name(), "BASELINE1_START");
        assert_eq!(AssignmentFieldLists::BASELINE_STARTS[9].name(), "BASELINE10_START");
        assert_eq!(AssignmentFieldLists::BASELINE_FINISHES[0].name(), "BASELINE1_FINISH");
        assert_eq!(AssignmentFieldLists::BASELINE_FINISHES[9].name(), "BASELINE10_FINISH");
        assert_eq!(AssignmentFieldLists::BASELINE_BUDGET_COSTS[0].name(), "BASELINE1_BUDGET_COST");
        assert_eq!(AssignmentFieldLists::BASELINE_BUDGET_COSTS[9].name(), "BASELINE10_BUDGET_COST");
        assert_eq!(AssignmentFieldLists::BASELINE_BUDGET_WORKS[0].name(), "BASELINE1_BUDGET_WORK");
        assert_eq!(AssignmentFieldLists::BASELINE_BUDGET_WORKS[9].name(), "BASELINE10_BUDGET_WORK");
    }

    #[test]
    fn custom_fields_match_java_concatenation_order() {
        assert_eq!(AssignmentFieldLists::CUSTOM_FIELDS[0].name(), "TEXT1");
        assert_eq!(AssignmentFieldLists::CUSTOM_FIELDS[308].name(), "ENTERPRISE_RESOURCE_MULTI_VALUE29");
    }
}
