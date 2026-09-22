//! Corresponds to Java: `org/mpxj/ResourceField.java`

use crate::data_type::DataType;
use crate::field_type::FieldType;
use crate::field_type_class::FieldTypeClass;
use crate::mpxj_enum::MpxjEnum;

/// A field on the resource (Resource) entity.
///
/// Corresponds to Java: `public enum ResourceField implements FieldType`
///
/// The variant order matches the Java declaration order, so `*self as i32` equals Java's
/// `getValue()` (Java assigns values in declaration order in the static initializer block).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceField {
    Start,
    Duration1Units,
    Duration2Units,
    Duration3Units,
    Duration4Units,
    Duration5Units,
    Duration6Units,
    Duration7Units,
    Duration8Units,
    Duration9Units,
    Duration10Units,
    Id,
    Name,
    Initials,
    Group,
    MaxUnits,
    BaseCalendar,
    StandardRate,
    OvertimeRate,
    Text1,
    Text2,
    Code,
    ActualCost,
    Cost,
    Work,
    ActualWork,
    BaselineWork,
    OvertimeWork,
    BaselineCost,
    CostPerUse,
    AccrueAt,
    RemainingCost,
    RemainingWork,
    WorkVariance,
    CostVariance,
    Overallocated,
    Peak,
    UniqueId,
    Notes,
    PercentWorkComplete,
    Text3,
    Text4,
    Text5,
    Objects,
    LinkedFields,
    EmailAddress,
    RegularWork,
    ActualOvertimeWork,
    RemainingOvertimeWork,
    OvertimeCost,
    ActualOvertimeCost,
    RemainingOvertimeCost,
    Bcws,
    Bcwp,
    Acwp,
    Sv,
    AvailableFrom,
    AvailableTo,
    Indicators,
    Text6,
    Text7,
    Text8,
    Text9,
    Text10,
    Start1,
    Start2,
    Start3,
    Start4,
    Start5,
    Finish1,
    Finish2,
    Finish3,
    Finish4,
    Finish5,
    Number1,
    Number2,
    Number3,
    Number4,
    Number5,
    Duration1,
    Duration2,
    Duration3,
    Cost1,
    Cost2,
    Cost3,
    Flag10,
    Flag1,
    Flag2,
    Flag3,
    Flag4,
    Flag5,
    Flag6,
    Flag7,
    Flag8,
    Flag9,
    Hyperlink,
    HyperlinkAddress,
    HyperlinkSubaddress,
    HyperlinkHref,
    Assignment,
    TaskSummaryName,
    CanLevel,
    WorkContour,
    Cost4,
    Cost5,
    Cost6,
    Cost7,
    Cost8,
    Cost9,
    Cost10,
    Date1,
    Date2,
    Date3,
    Date4,
    Date5,
    Date6,
    Date7,
    Date8,
    Date9,
    Date10,
    Duration4,
    Duration5,
    Duration6,
    Duration7,
    Duration8,
    Duration9,
    Duration10,
    Finish6,
    Finish7,
    Finish8,
    Finish9,
    Finish10,
    Flag11,
    Flag12,
    Flag13,
    Flag14,
    Flag15,
    Flag16,
    Flag17,
    Flag18,
    Flag19,
    Flag20,
    Number6,
    Number7,
    Number8,
    Number9,
    Number10,
    Number11,
    Number12,
    Number13,
    Number14,
    Number15,
    Number16,
    Number17,
    Number18,
    Number19,
    Number20,
    Start6,
    Start7,
    Start8,
    Start9,
    Start10,
    Text11,
    Text12,
    Text13,
    Text14,
    Text15,
    Text16,
    Text17,
    Text18,
    Text19,
    Text20,
    Text21,
    Text22,
    Text23,
    Text24,
    Text25,
    Text26,
    Text27,
    Text28,
    Text29,
    Text30,
    Phonetics,
    AssignmentDelay,
    AssignmentUnits,
    BaselineStart,
    BaselineFinish,
    Confirmed,
    LevelingDelay,
    ResponsePending,
    TeamStatusPending,
    Cv,
    UpdateNeeded,
    CostRateTable,
    ActualStart,
    ActualFinish,
    Workgroup,
    Project,
    OutlineCode1,
    OutlineCode2,
    OutlineCode3,
    OutlineCode4,
    OutlineCode5,
    OutlineCode6,
    OutlineCode7,
    OutlineCode8,
    OutlineCode9,
    OutlineCode10,
    MaterialLabel,
    Type,
    Vac,
    GroupBySummary,
    WindowsUserAccount,
    Baseline1Work,
    Baseline1Cost,
    Baseline2Work,
    Baseline2Cost,
    Baseline3Work,
    Baseline3Cost,
    Baseline4Work,
    Baseline4Cost,
    Baseline5Work,
    Baseline5Cost,
    Baseline6Work,
    Baseline6Cost,
    Baseline7Work,
    Baseline7Cost,
    Baseline8Work,
    Baseline8Cost,
    Baseline9Work,
    Baseline9Cost,
    Baseline10Work,
    Baseline10Cost,
    EnterpriseCost1,
    EnterpriseCost2,
    EnterpriseCost3,
    EnterpriseCost4,
    EnterpriseCost5,
    EnterpriseCost6,
    EnterpriseCost7,
    EnterpriseCost8,
    EnterpriseCost9,
    EnterpriseCost10,
    EnterpriseDate1,
    EnterpriseDate2,
    EnterpriseDate3,
    EnterpriseDate4,
    EnterpriseDate5,
    EnterpriseDate6,
    EnterpriseDate7,
    EnterpriseDate8,
    EnterpriseDate9,
    EnterpriseDate10,
    EnterpriseDate11,
    EnterpriseDate12,
    EnterpriseDate13,
    EnterpriseDate14,
    EnterpriseDate15,
    EnterpriseDate16,
    EnterpriseDate17,
    EnterpriseDate18,
    EnterpriseDate19,
    EnterpriseDate20,
    EnterpriseDate21,
    EnterpriseDate22,
    EnterpriseDate23,
    EnterpriseDate24,
    EnterpriseDate25,
    EnterpriseDate26,
    EnterpriseDate27,
    EnterpriseDate28,
    EnterpriseDate29,
    EnterpriseDate30,
    EnterpriseDuration1Units,
    EnterpriseDuration2Units,
    EnterpriseDuration3Units,
    EnterpriseDuration4Units,
    EnterpriseDuration5Units,
    EnterpriseDuration6Units,
    EnterpriseDuration7Units,
    EnterpriseDuration8Units,
    EnterpriseDuration9Units,
    EnterpriseDuration10Units,
    EnterpriseDuration1,
    EnterpriseDuration2,
    EnterpriseDuration3,
    EnterpriseDuration4,
    EnterpriseDuration5,
    EnterpriseDuration6,
    EnterpriseDuration7,
    EnterpriseDuration8,
    EnterpriseDuration9,
    EnterpriseDuration10,
    EnterpriseFlag1,
    EnterpriseFlag2,
    EnterpriseFlag3,
    EnterpriseFlag4,
    EnterpriseFlag5,
    EnterpriseFlag6,
    EnterpriseFlag7,
    EnterpriseFlag8,
    EnterpriseFlag9,
    EnterpriseFlag10,
    EnterpriseFlag11,
    EnterpriseFlag12,
    EnterpriseFlag13,
    EnterpriseFlag14,
    EnterpriseFlag15,
    EnterpriseFlag16,
    EnterpriseFlag17,
    EnterpriseFlag18,
    EnterpriseFlag19,
    EnterpriseFlag20,
    EnterpriseNumber1,
    EnterpriseNumber2,
    EnterpriseNumber3,
    EnterpriseNumber4,
    EnterpriseNumber5,
    EnterpriseNumber6,
    EnterpriseNumber7,
    EnterpriseNumber8,
    EnterpriseNumber9,
    EnterpriseNumber10,
    EnterpriseNumber11,
    EnterpriseNumber12,
    EnterpriseNumber13,
    EnterpriseNumber14,
    EnterpriseNumber15,
    EnterpriseNumber16,
    EnterpriseNumber17,
    EnterpriseNumber18,
    EnterpriseNumber19,
    EnterpriseNumber20,
    EnterpriseNumber21,
    EnterpriseNumber22,
    EnterpriseNumber23,
    EnterpriseNumber24,
    EnterpriseNumber25,
    EnterpriseNumber26,
    EnterpriseNumber27,
    EnterpriseNumber28,
    EnterpriseNumber29,
    EnterpriseNumber30,
    EnterpriseNumber31,
    EnterpriseNumber32,
    EnterpriseNumber33,
    EnterpriseNumber34,
    EnterpriseNumber35,
    EnterpriseNumber36,
    EnterpriseNumber37,
    EnterpriseNumber38,
    EnterpriseNumber39,
    EnterpriseNumber40,
    EnterpriseText1,
    EnterpriseText2,
    EnterpriseText3,
    EnterpriseText4,
    EnterpriseText5,
    EnterpriseText6,
    EnterpriseText7,
    EnterpriseText8,
    EnterpriseText9,
    EnterpriseText10,
    EnterpriseText11,
    EnterpriseText12,
    EnterpriseText13,
    EnterpriseText14,
    EnterpriseText15,
    EnterpriseText16,
    EnterpriseText17,
    EnterpriseText18,
    EnterpriseText19,
    EnterpriseText20,
    EnterpriseText21,
    EnterpriseText22,
    EnterpriseText23,
    EnterpriseText24,
    EnterpriseText25,
    EnterpriseText26,
    EnterpriseText27,
    EnterpriseText28,
    EnterpriseText29,
    EnterpriseText30,
    EnterpriseText31,
    EnterpriseText32,
    EnterpriseText33,
    EnterpriseText34,
    EnterpriseText35,
    EnterpriseText36,
    EnterpriseText37,
    EnterpriseText38,
    EnterpriseText39,
    EnterpriseText40,
    Enterprise,
    Import,
    Created,
    Budget,
    TaskOutlineNumber,
    Guid,
    Active,
    AssignmentOwner,
    BaselineBudgetCost,
    BaselineBudgetWork,
    Baseline1BudgetCost,
    Baseline1BudgetWork,
    Baseline1Finish,
    Baseline1Start,
    Baseline2BudgetCost,
    Baseline2BudgetWork,
    Baseline2Finish,
    Baseline2Start,
    Baseline3BudgetCost,
    Baseline3BudgetWork,
    Baseline3Finish,
    Baseline3Start,
    Baseline4BudgetCost,
    Baseline4BudgetWork,
    Baseline4Finish,
    Baseline4Start,
    Baseline5BudgetCost,
    Baseline5BudgetWork,
    Baseline5Finish,
    Baseline5Start,
    Baseline6BudgetCost,
    Baseline6BudgetWork,
    Baseline6Finish,
    Baseline6Start,
    Baseline7BudgetCost,
    Baseline7BudgetWork,
    Baseline7Finish,
    Baseline7Start,
    Baseline8BudgetCost,
    Baseline8BudgetWork,
    Baseline8Finish,
    Baseline8Start,
    Baseline9BudgetCost,
    Baseline9BudgetWork,
    Baseline9Finish,
    Baseline9Start,
    Baseline10BudgetCost,
    Baseline10BudgetWork,
    Baseline10Finish,
    Baseline10Start,
    BookingType,
    BudgetCost,
    BudgetWork,
    CalendarGuid,
    CostCenter,
    DefaultAssignmentOwner,
    EnterpriseBaseCalendar,
    EnterpriseRequiredValues,
    EnterpriseTeamMember,
    EnterpriseUniqueId,
    ErrorMessage,
    Generic,
    Inactive,
    RequestDemand,
    Summary,
    TeamAssignmentPool,
    Wbs,
    Unavailable,
    SubprojectResourceUniqueId,
    HyperlinkData,
    OutlineCode1Index,
    OutlineCode2Index,
    OutlineCode3Index,
    OutlineCode4Index,
    OutlineCode5Index,
    OutlineCode6Index,
    OutlineCode7Index,
    OutlineCode8Index,
    OutlineCode9Index,
    OutlineCode10Index,
    CostRateA,
    CostRateB,
    CostRateC,
    CostRateD,
    CostRateE,
    AvailabilityData,
    EnterpriseData,
    OvertimeRateUnits,
    StandardRateUnits,
    Index,
    HyperlinkScreenTip,
    EnterpriseOutlineCode1,
    EnterpriseOutlineCode2,
    EnterpriseOutlineCode3,
    EnterpriseOutlineCode4,
    EnterpriseOutlineCode5,
    EnterpriseOutlineCode6,
    EnterpriseOutlineCode7,
    EnterpriseOutlineCode8,
    EnterpriseOutlineCode9,
    EnterpriseOutlineCode10,
    EnterpriseOutlineCode11,
    EnterpriseOutlineCode12,
    EnterpriseOutlineCode13,
    EnterpriseOutlineCode14,
    EnterpriseOutlineCode15,
    EnterpriseOutlineCode16,
    EnterpriseOutlineCode17,
    EnterpriseOutlineCode18,
    EnterpriseOutlineCode19,
    EnterpriseOutlineCode20,
    EnterpriseOutlineCode21,
    EnterpriseOutlineCode22,
    EnterpriseOutlineCode23,
    EnterpriseOutlineCode24,
    EnterpriseOutlineCode25,
    EnterpriseOutlineCode26,
    EnterpriseOutlineCode27,
    EnterpriseOutlineCode28,
    EnterpriseOutlineCode29,
    EnterpriseRbs,
    EnterpriseNameUsed,
    EnterpriseIsCheckedOut,
    EnterpriseCheckedOutBy,
    EnterpriseLastModifiedDate,
    EnterpriseMultiValue20,
    EnterpriseMultiValue21,
    EnterpriseMultiValue22,
    EnterpriseMultiValue23,
    EnterpriseMultiValue24,
    EnterpriseMultiValue25,
    EnterpriseMultiValue26,
    EnterpriseMultiValue27,
    EnterpriseMultiValue28,
    EnterpriseMultiValue29,
    ActualWorkProtected,
    ActualOvertimeWorkProtected,
    ParentId,
    CalendarUniqueId,
    ProposedStart,
    ProposedFinish,
    ProposedMaxUnits,
    EngagementStatus,
    Unit,
    SupplyReference,
    Description,
    ResourceId,
    ModifyOnIntegrate,
    ExpensesOnly,
    PeriodDur,
    Priority,
    Rate,
    Pool,
    PerDay,
    Phone,
    Role,
    CalculateCostsFromUnits,
    SequenceNumber,
    LocationUniqueId,
    UnitOfMeasureUniqueId,
    DefaultUnits,
    ShiftUniqueId,
    PrimaryRoleUniqueId,
    Finish,
    ResourceCodeValues,
    RoleCodeValues,
    CurrencyUniqueId,
    PlannedWork,
    ActualRegularWork,
    RemainingRegularWork,
    ActualRegularCost,
    RemainingRegularCost,
    PlannedMaterial,
    ActualMaterial,
    RemainingMaterial,
    Material,
    BaselineMaterial,
    Baseline1Material,
    Baseline2Material,
    Baseline3Material,
    Baseline4Material,
    Baseline5Material,
    Baseline6Material,
    Baseline7Material,
    Baseline8Material,
    Baseline9Material,
    Baseline10Material,
    PlannedCost,
}

/// The English column name table for Java `LocaleData.RESOURCE_COLUMNS_ARRAY`, arranged by `getValue()` index.
///
/// Indices not assigned on the Java side are represented by the empty string `""`; this table has been verified to have no gaps.
const RESOURCE_COLUMN_NAMES: [&str; ResourceField::MAX_VALUE] = [
    "Start",
    "Duration1 Units",
    "Duration2 Units",
    "Duration3 Units",
    "Duration4 Units",
    "Duration5 Units",
    "Duration6 Units",
    "Duration7 Units",
    "Duration8 Units",
    "Duration9 Units",
    "Duration10 Units",
    "ID",
    "Name",
    "Initials",
    "Group",
    "Max Units",
    "Base Calendar",
    "Standard Rate",
    "Overtime Rate",
    "Text1",
    "Text2",
    "Code",
    "Actual Cost",
    "Cost",
    "Work",
    "Actual Work",
    "Baseline Work",
    "Overtime Work",
    "Baseline Cost",
    "Cost Per Use",
    "Accrue At",
    "Remaining Cost",
    "Remaining Work",
    "Work Variance",
    "Cost Variance",
    "Overallocated",
    "Peak",
    "Unique ID",
    "Notes",
    "% Work Complete",
    "Text3",
    "Text4",
    "Text5",
    "Objects",
    "Linked Fields",
    "Email Address",
    "Regular Work",
    "Actual Overtime Work",
    "Remaining Overtime Work",
    "Overtime Cost",
    "Actual Overtime Cost",
    "Remaining Overtime Cost",
    "BCWS",
    "BCWP",
    "ACWP",
    "SV",
    "Available From",
    "Available To",
    "Indicators",
    "Text6",
    "Text7",
    "Text8",
    "Text9",
    "Text10",
    "Start1",
    "Start2",
    "Start3",
    "Start4",
    "Start5",
    "Finish1",
    "Finish2",
    "Finish3",
    "Finish4",
    "Finish5",
    "Number1",
    "Number2",
    "Number3",
    "Number4",
    "Number5",
    "Duration1",
    "Duration2",
    "Duration3",
    "Cost1",
    "Cost2",
    "Cost3",
    "Flag10",
    "Flag1",
    "Flag2",
    "Flag3",
    "Flag4",
    "Flag5",
    "Flag6",
    "Flag7",
    "Flag8",
    "Flag9",
    "Hyperlink",
    "Hyperlink Address",
    "Hyperlink SubAddress",
    "Hyperlink Href",
    "Assignment",
    "Task Summary Name",
    "Can Level",
    "Work Contour",
    "Cost4",
    "Cost5",
    "Cost6",
    "Cost7",
    "Cost8",
    "Cost9",
    "Cost10",
    "Date1",
    "Date2",
    "Date3",
    "Date4",
    "Date5",
    "Date6",
    "Date7",
    "Date8",
    "Date9",
    "Date10",
    "Duration4",
    "Duration5",
    "Duration6",
    "Duration7",
    "Duration8",
    "Duration9",
    "Duration10",
    "Finish6",
    "Finish7",
    "Finish8",
    "Finish9",
    "Finish10",
    "Flag11",
    "Flag12",
    "Flag13",
    "Flag14",
    "Flag15",
    "Flag16",
    "Flag17",
    "Flag18",
    "Flag19",
    "Flag20",
    "Number6",
    "Number7",
    "Number8",
    "Number9",
    "Number10",
    "Number11",
    "Number12",
    "Number13",
    "Number14",
    "Number15",
    "Number16",
    "Number17",
    "Number18",
    "Number19",
    "Number20",
    "Start6",
    "Start7",
    "Start8",
    "Start9",
    "Start10",
    "Text11",
    "Text12",
    "Text13",
    "Text14",
    "Text15",
    "Text16",
    "Text17",
    "Text18",
    "Text19",
    "Text20",
    "Text21",
    "Text22",
    "Text23",
    "Text24",
    "Text25",
    "Text26",
    "Text27",
    "Text28",
    "Text29",
    "Text30",
    "Phonetics",
    "Assignment Delay",
    "Assignment Units",
    "Baseline Start",
    "Baseline Finish",
    "Confirmed",
    "Leveling Delay",
    "Response Pending",
    "TeamStatus Pending",
    "CV",
    "Update Needed",
    "Cost Rate Table",
    "Actual Start",
    "Actual Finish",
    "Workgroup",
    "Project",
    "Outline Code1",
    "Outline Code2",
    "Outline Code3",
    "Outline Code4",
    "Outline Code5",
    "Outline Code6",
    "Outline Code7",
    "Outline Code8",
    "Outline Code9",
    "Outline Code10",
    "Material Label",
    "Type",
    "VAC",
    "Group By Summary",
    "Windows User Account",
    "Baseline1 Work",
    "Baseline1 Cost",
    "Baseline2 Work",
    "Baseline2 Cost",
    "Baseline3 Work",
    "Baseline3 Cost",
    "Baseline4 Work",
    "Baseline4 Cost",
    "Baseline5 Work",
    "Baseline5 Cost",
    "Baseline6 Work",
    "Baseline6 Cost",
    "Baseline7 Work",
    "Baseline7 Cost",
    "Baseline8 Work",
    "Baseline8 Cost",
    "Baseline9 Work",
    "Baseline9 Cost",
    "Baseline10 Work",
    "Baseline10 Cost",
    "Enterprise Cost1",
    "Enterprise Cost2",
    "Enterprise Cost3",
    "Enterprise Cost4",
    "Enterprise Cost5",
    "Enterprise Cost6",
    "Enterprise Cost7",
    "Enterprise Cost8",
    "Enterprise Cost9",
    "Enterprise Cost10",
    "Enterprise Date1",
    "Enterprise Date2",
    "Enterprise Date3",
    "Enterprise Date4",
    "Enterprise Date5",
    "Enterprise Date6",
    "Enterprise Date7",
    "Enterprise Date8",
    "Enterprise Date9",
    "Enterprise Date10",
    "Enterprise Date11",
    "Enterprise Date12",
    "Enterprise Date13",
    "Enterprise Date14",
    "Enterprise Date15",
    "Enterprise Date16",
    "Enterprise Date17",
    "Enterprise Date18",
    "Enterprise Date19",
    "Enterprise Date20",
    "Enterprise Date21",
    "Enterprise Date22",
    "Enterprise Date23",
    "Enterprise Date24",
    "Enterprise Date25",
    "Enterprise Date26",
    "Enterprise Date27",
    "Enterprise Date28",
    "Enterprise Date29",
    "Enterprise Date30",
    "Enterprise Duration1 Units",
    "Enterprise Duration2 Units",
    "Enterprise Duration3 Units",
    "Enterprise Duration4 Units",
    "Enterprise Duration5 Units",
    "Enterprise Duration6 Units",
    "Enterprise Duration7 Units",
    "Enterprise Duration8 Units",
    "Enterprise Duration9 Units",
    "Enterprise Duration10 Units",
    "Enterprise Duration1",
    "Enterprise Duration2",
    "Enterprise Duration3",
    "Enterprise Duration4",
    "Enterprise Duration5",
    "Enterprise Duration6",
    "Enterprise Duration7",
    "Enterprise Duration8",
    "Enterprise Duration9",
    "Enterprise Duration10",
    "Enterprise Flag1",
    "Enterprise Flag2",
    "Enterprise Flag3",
    "Enterprise Flag4",
    "Enterprise Flag5",
    "Enterprise Flag6",
    "Enterprise Flag7",
    "Enterprise Flag8",
    "Enterprise Flag9",
    "Enterprise Flag10",
    "Enterprise Flag11",
    "Enterprise Flag12",
    "Enterprise Flag13",
    "Enterprise Flag14",
    "Enterprise Flag15",
    "Enterprise Flag16",
    "Enterprise Flag17",
    "Enterprise Flag18",
    "Enterprise Flag19",
    "Enterprise Flag20",
    "Enterprise Number1",
    "Enterprise Number2",
    "Enterprise Number3",
    "Enterprise Number4",
    "Enterprise Number5",
    "Enterprise Number6",
    "Enterprise Number7",
    "Enterprise Number8",
    "Enterprise Number9",
    "Enterprise Number10",
    "Enterprise Number11",
    "Enterprise Number12",
    "Enterprise Number13",
    "Enterprise Number14",
    "Enterprise Number15",
    "Enterprise Number16",
    "Enterprise Number17",
    "Enterprise Number18",
    "Enterprise Number19",
    "Enterprise Number20",
    "Enterprise Number21",
    "Enterprise Number22",
    "Enterprise Number23",
    "Enterprise Number24",
    "Enterprise Number25",
    "Enterprise Number26",
    "Enterprise Number27",
    "Enterprise Number28",
    "Enterprise Number29",
    "Enterprise Number30",
    "Enterprise Number31",
    "Enterprise Number32",
    "Enterprise Number33",
    "Enterprise Number34",
    "Enterprise Number35",
    "Enterprise Number36",
    "Enterprise Number37",
    "Enterprise Number38",
    "Enterprise Number39",
    "Enterprise Number40",
    "Enterprise Text1",
    "Enterprise Text2",
    "Enterprise Text3",
    "Enterprise Text4",
    "Enterprise Text5",
    "Enterprise Text6",
    "Enterprise Text7",
    "Enterprise Text8",
    "Enterprise Text9",
    "Enterprise Text10",
    "Enterprise Text11",
    "Enterprise Text12",
    "Enterprise Text13",
    "Enterprise Text14",
    "Enterprise Text15",
    "Enterprise Text16",
    "Enterprise Text17",
    "Enterprise Text18",
    "Enterprise Text19",
    "Enterprise Text20",
    "Enterprise Text21",
    "Enterprise Text22",
    "Enterprise Text23",
    "Enterprise Text24",
    "Enterprise Text25",
    "Enterprise Text26",
    "Enterprise Text27",
    "Enterprise Text28",
    "Enterprise Text29",
    "Enterprise Text30",
    "Enterprise Text31",
    "Enterprise Text32",
    "Enterprise Text33",
    "Enterprise Text34",
    "Enterprise Text35",
    "Enterprise Text36",
    "Enterprise Text37",
    "Enterprise Text38",
    "Enterprise Text39",
    "Enterprise Text40",
    "Enterprise",
    "Import",
    "Created",
    "Budget",
    "Task Outline Number",
    "GUID",
    "Active",
    "Assignment Owner",
    "Baseline Budget Cost",
    "Baseline Budget Work",
    "Baseline1 Budget Cost",
    "Baseline1 Budget Work",
    "Baseline1 Finish",
    "Baseline1 Start",
    "Baseline2 Budget Cost",
    "Baseline2 Budget Work",
    "Baseline2 Finish",
    "Baseline2 Start",
    "Baseline3 Budget Cost",
    "Baseline3 Budget Work",
    "Baseline3 Finish",
    "Baseline3 Start",
    "Baseline4 Budget Cost",
    "Baseline4 Budget Work",
    "Baseline4 Finish",
    "Baseline4 Start",
    "Baseline5 Budget Cost",
    "Baseline5 Budget Work",
    "Baseline5 Finish",
    "Baseline5 Start",
    "Baseline6 Budget Cost",
    "Baseline6 Budget Work",
    "Baseline6 Finish",
    "Baseline6 Start",
    "Baseline7 Budget Cost",
    "Baseline7 Budget Work",
    "Baseline7 Finish",
    "Baseline7 Start",
    "Baseline8 Budget Cost",
    "Baseline8 Budget Work",
    "Baseline8 Finish",
    "Baseline8 Start",
    "Baseline9 Budget Cost",
    "Baseline9 Budget Work",
    "Baseline9 Finish",
    "Baseline9 Start",
    "Baseline10 Budget Cost",
    "Baseline10 Budget Work",
    "Baseline10 Finish",
    "Baseline10 Start",
    "Booking Type",
    "Budget Cost",
    "Budget Work",
    "Calendar GUID",
    "Cost Center",
    "Default Assignment Owner",
    "Enterprise Base Calendar",
    "Enterprise Required Values",
    "Enterprise Team Member",
    "Enterprise Unique ID",
    "Error Message",
    "Generic",
    "Inactive",
    "Request/Demand",
    "Summary",
    "Team Assignment Pool",
    "WBS",
    "<Unavailable>",
    "Subproject Unique Resource ID",
    "Hyperlink Data",
    "Outline Code1 Index",
    "Outline Code2 Index",
    "Outline Code3 Index",
    "Outline Code4 Index",
    "Outline Code5 Index",
    "Outline Code6 Index",
    "Outline Code7 Index",
    "Outline Code8 Index",
    "Outline Code9 Index",
    "Outline Code10 Index",
    "Cost Rate A",
    "Cost Rate B",
    "Cost Rate C",
    "Cost Rate D",
    "Cost Rate E",
    "Availability Data",
    "Enterprise Data",
    "Overtime Rate Units",
    "Standard Rate Units",
    "Index",
    "Hyperlink Screen Tip",
    "Enterprise Outline Code1",
    "Enterprise Outline Code2",
    "Enterprise Outline Code3",
    "Enterprise Outline Code4",
    "Enterprise Outline Code5",
    "Enterprise Outline Code6",
    "Enterprise Outline Code7",
    "Enterprise Outline Code8",
    "Enterprise Outline Code9",
    "Enterprise Outline Code10",
    "Enterprise Outline Code11",
    "Enterprise Outline Code12",
    "Enterprise Outline Code13",
    "Enterprise Outline Code14",
    "Enterprise Outline Code15",
    "Enterprise Outline Code16",
    "Enterprise Outline Code17",
    "Enterprise Outline Code18",
    "Enterprise Outline Code19",
    "Enterprise Outline Code20",
    "Enterprise Outline Code21",
    "Enterprise Outline Code22",
    "Enterprise Outline Code23",
    "Enterprise Outline Code24",
    "Enterprise Outline Code25",
    "Enterprise Outline Code26",
    "Enterprise Outline Code27",
    "Enterprise Outline Code28",
    "Enterprise Outline Code29",
    "Enterprise RBS",
    "Enterprise Name Used",
    "Enterprise Is Checked Out",
    "Enterprise Checked Out By",
    "Enterprise Last Modified Date",
    "Enterprise Multi Value20",
    "Enterprise Multi Value21",
    "Enterprise Multi Value22",
    "Enterprise Multi Value23",
    "Enterprise Multi Value24",
    "Enterprise Multi Value25",
    "Enterprise Multi Value26",
    "Enterprise Multi Value27",
    "Enterprise Multi Value28",
    "Enterprise Multi Value29",
    "Actual Work Protected",
    "Actual Overtime Work Protected",
    "Parent ID",
    "Calendar Unique ID",
    "Proposed Start",
    "Proposed Finish",
    "Proposed Max Units",
    "Engagement Status",
    "Unit",
    "Supply Reference",
    "Description",
    "Resource ID",
    "Modify On Integrate",
    "Expenses Only",
    "Period Dur",
    "Priority",
    "Rate",
    "Pool",
    "Per Day",
    "Phone",
    "Role",
    "Calculate Costs From Units",
    "Sequence Number",
    "Location Unique ID",
    "Unit of Measure Unique ID",
    "Default Units",
    "Shift Unique ID",
    "Primary Role Unique ID",
    "Finish",
    "Resource Code Values",
    "Role Code Values",
    "Currency Unique ID",
    "Planned Work",
    "Actual Regular Work",
    "Remaining Regular Work",
    "Actual Regular Cost",
    "Remaining Regular Cost",
    "Planned Material",
    "Actual Material",
    "Remaining Material",
    "Material",
    "Baseline Material",
    "Baseline1 Material",
    "Baseline2 Material",
    "Baseline3 Material",
    "Baseline4 Material",
    "Baseline5 Material",
    "Baseline6 Material",
    "Baseline7 Material",
    "Baseline8 Material",
    "Baseline9 Material",
    "Baseline10 Material",
    "Planned Cost",
];

impl ResourceField {
    /// Total number of variants.
    ///
    /// Corresponds to Java: `public static final int MAX_VALUE`
    pub const MAX_VALUE: usize = 581;

    /// All values, in declaration order (i.e. ascending `getValue()`).
    pub const ALL: [ResourceField; ResourceField::MAX_VALUE] = [
        ResourceField::Start,
        ResourceField::Duration1Units,
        ResourceField::Duration2Units,
        ResourceField::Duration3Units,
        ResourceField::Duration4Units,
        ResourceField::Duration5Units,
        ResourceField::Duration6Units,
        ResourceField::Duration7Units,
        ResourceField::Duration8Units,
        ResourceField::Duration9Units,
        ResourceField::Duration10Units,
        ResourceField::Id,
        ResourceField::Name,
        ResourceField::Initials,
        ResourceField::Group,
        ResourceField::MaxUnits,
        ResourceField::BaseCalendar,
        ResourceField::StandardRate,
        ResourceField::OvertimeRate,
        ResourceField::Text1,
        ResourceField::Text2,
        ResourceField::Code,
        ResourceField::ActualCost,
        ResourceField::Cost,
        ResourceField::Work,
        ResourceField::ActualWork,
        ResourceField::BaselineWork,
        ResourceField::OvertimeWork,
        ResourceField::BaselineCost,
        ResourceField::CostPerUse,
        ResourceField::AccrueAt,
        ResourceField::RemainingCost,
        ResourceField::RemainingWork,
        ResourceField::WorkVariance,
        ResourceField::CostVariance,
        ResourceField::Overallocated,
        ResourceField::Peak,
        ResourceField::UniqueId,
        ResourceField::Notes,
        ResourceField::PercentWorkComplete,
        ResourceField::Text3,
        ResourceField::Text4,
        ResourceField::Text5,
        ResourceField::Objects,
        ResourceField::LinkedFields,
        ResourceField::EmailAddress,
        ResourceField::RegularWork,
        ResourceField::ActualOvertimeWork,
        ResourceField::RemainingOvertimeWork,
        ResourceField::OvertimeCost,
        ResourceField::ActualOvertimeCost,
        ResourceField::RemainingOvertimeCost,
        ResourceField::Bcws,
        ResourceField::Bcwp,
        ResourceField::Acwp,
        ResourceField::Sv,
        ResourceField::AvailableFrom,
        ResourceField::AvailableTo,
        ResourceField::Indicators,
        ResourceField::Text6,
        ResourceField::Text7,
        ResourceField::Text8,
        ResourceField::Text9,
        ResourceField::Text10,
        ResourceField::Start1,
        ResourceField::Start2,
        ResourceField::Start3,
        ResourceField::Start4,
        ResourceField::Start5,
        ResourceField::Finish1,
        ResourceField::Finish2,
        ResourceField::Finish3,
        ResourceField::Finish4,
        ResourceField::Finish5,
        ResourceField::Number1,
        ResourceField::Number2,
        ResourceField::Number3,
        ResourceField::Number4,
        ResourceField::Number5,
        ResourceField::Duration1,
        ResourceField::Duration2,
        ResourceField::Duration3,
        ResourceField::Cost1,
        ResourceField::Cost2,
        ResourceField::Cost3,
        ResourceField::Flag10,
        ResourceField::Flag1,
        ResourceField::Flag2,
        ResourceField::Flag3,
        ResourceField::Flag4,
        ResourceField::Flag5,
        ResourceField::Flag6,
        ResourceField::Flag7,
        ResourceField::Flag8,
        ResourceField::Flag9,
        ResourceField::Hyperlink,
        ResourceField::HyperlinkAddress,
        ResourceField::HyperlinkSubaddress,
        ResourceField::HyperlinkHref,
        ResourceField::Assignment,
        ResourceField::TaskSummaryName,
        ResourceField::CanLevel,
        ResourceField::WorkContour,
        ResourceField::Cost4,
        ResourceField::Cost5,
        ResourceField::Cost6,
        ResourceField::Cost7,
        ResourceField::Cost8,
        ResourceField::Cost9,
        ResourceField::Cost10,
        ResourceField::Date1,
        ResourceField::Date2,
        ResourceField::Date3,
        ResourceField::Date4,
        ResourceField::Date5,
        ResourceField::Date6,
        ResourceField::Date7,
        ResourceField::Date8,
        ResourceField::Date9,
        ResourceField::Date10,
        ResourceField::Duration4,
        ResourceField::Duration5,
        ResourceField::Duration6,
        ResourceField::Duration7,
        ResourceField::Duration8,
        ResourceField::Duration9,
        ResourceField::Duration10,
        ResourceField::Finish6,
        ResourceField::Finish7,
        ResourceField::Finish8,
        ResourceField::Finish9,
        ResourceField::Finish10,
        ResourceField::Flag11,
        ResourceField::Flag12,
        ResourceField::Flag13,
        ResourceField::Flag14,
        ResourceField::Flag15,
        ResourceField::Flag16,
        ResourceField::Flag17,
        ResourceField::Flag18,
        ResourceField::Flag19,
        ResourceField::Flag20,
        ResourceField::Number6,
        ResourceField::Number7,
        ResourceField::Number8,
        ResourceField::Number9,
        ResourceField::Number10,
        ResourceField::Number11,
        ResourceField::Number12,
        ResourceField::Number13,
        ResourceField::Number14,
        ResourceField::Number15,
        ResourceField::Number16,
        ResourceField::Number17,
        ResourceField::Number18,
        ResourceField::Number19,
        ResourceField::Number20,
        ResourceField::Start6,
        ResourceField::Start7,
        ResourceField::Start8,
        ResourceField::Start9,
        ResourceField::Start10,
        ResourceField::Text11,
        ResourceField::Text12,
        ResourceField::Text13,
        ResourceField::Text14,
        ResourceField::Text15,
        ResourceField::Text16,
        ResourceField::Text17,
        ResourceField::Text18,
        ResourceField::Text19,
        ResourceField::Text20,
        ResourceField::Text21,
        ResourceField::Text22,
        ResourceField::Text23,
        ResourceField::Text24,
        ResourceField::Text25,
        ResourceField::Text26,
        ResourceField::Text27,
        ResourceField::Text28,
        ResourceField::Text29,
        ResourceField::Text30,
        ResourceField::Phonetics,
        ResourceField::AssignmentDelay,
        ResourceField::AssignmentUnits,
        ResourceField::BaselineStart,
        ResourceField::BaselineFinish,
        ResourceField::Confirmed,
        ResourceField::LevelingDelay,
        ResourceField::ResponsePending,
        ResourceField::TeamStatusPending,
        ResourceField::Cv,
        ResourceField::UpdateNeeded,
        ResourceField::CostRateTable,
        ResourceField::ActualStart,
        ResourceField::ActualFinish,
        ResourceField::Workgroup,
        ResourceField::Project,
        ResourceField::OutlineCode1,
        ResourceField::OutlineCode2,
        ResourceField::OutlineCode3,
        ResourceField::OutlineCode4,
        ResourceField::OutlineCode5,
        ResourceField::OutlineCode6,
        ResourceField::OutlineCode7,
        ResourceField::OutlineCode8,
        ResourceField::OutlineCode9,
        ResourceField::OutlineCode10,
        ResourceField::MaterialLabel,
        ResourceField::Type,
        ResourceField::Vac,
        ResourceField::GroupBySummary,
        ResourceField::WindowsUserAccount,
        ResourceField::Baseline1Work,
        ResourceField::Baseline1Cost,
        ResourceField::Baseline2Work,
        ResourceField::Baseline2Cost,
        ResourceField::Baseline3Work,
        ResourceField::Baseline3Cost,
        ResourceField::Baseline4Work,
        ResourceField::Baseline4Cost,
        ResourceField::Baseline5Work,
        ResourceField::Baseline5Cost,
        ResourceField::Baseline6Work,
        ResourceField::Baseline6Cost,
        ResourceField::Baseline7Work,
        ResourceField::Baseline7Cost,
        ResourceField::Baseline8Work,
        ResourceField::Baseline8Cost,
        ResourceField::Baseline9Work,
        ResourceField::Baseline9Cost,
        ResourceField::Baseline10Work,
        ResourceField::Baseline10Cost,
        ResourceField::EnterpriseCost1,
        ResourceField::EnterpriseCost2,
        ResourceField::EnterpriseCost3,
        ResourceField::EnterpriseCost4,
        ResourceField::EnterpriseCost5,
        ResourceField::EnterpriseCost6,
        ResourceField::EnterpriseCost7,
        ResourceField::EnterpriseCost8,
        ResourceField::EnterpriseCost9,
        ResourceField::EnterpriseCost10,
        ResourceField::EnterpriseDate1,
        ResourceField::EnterpriseDate2,
        ResourceField::EnterpriseDate3,
        ResourceField::EnterpriseDate4,
        ResourceField::EnterpriseDate5,
        ResourceField::EnterpriseDate6,
        ResourceField::EnterpriseDate7,
        ResourceField::EnterpriseDate8,
        ResourceField::EnterpriseDate9,
        ResourceField::EnterpriseDate10,
        ResourceField::EnterpriseDate11,
        ResourceField::EnterpriseDate12,
        ResourceField::EnterpriseDate13,
        ResourceField::EnterpriseDate14,
        ResourceField::EnterpriseDate15,
        ResourceField::EnterpriseDate16,
        ResourceField::EnterpriseDate17,
        ResourceField::EnterpriseDate18,
        ResourceField::EnterpriseDate19,
        ResourceField::EnterpriseDate20,
        ResourceField::EnterpriseDate21,
        ResourceField::EnterpriseDate22,
        ResourceField::EnterpriseDate23,
        ResourceField::EnterpriseDate24,
        ResourceField::EnterpriseDate25,
        ResourceField::EnterpriseDate26,
        ResourceField::EnterpriseDate27,
        ResourceField::EnterpriseDate28,
        ResourceField::EnterpriseDate29,
        ResourceField::EnterpriseDate30,
        ResourceField::EnterpriseDuration1Units,
        ResourceField::EnterpriseDuration2Units,
        ResourceField::EnterpriseDuration3Units,
        ResourceField::EnterpriseDuration4Units,
        ResourceField::EnterpriseDuration5Units,
        ResourceField::EnterpriseDuration6Units,
        ResourceField::EnterpriseDuration7Units,
        ResourceField::EnterpriseDuration8Units,
        ResourceField::EnterpriseDuration9Units,
        ResourceField::EnterpriseDuration10Units,
        ResourceField::EnterpriseDuration1,
        ResourceField::EnterpriseDuration2,
        ResourceField::EnterpriseDuration3,
        ResourceField::EnterpriseDuration4,
        ResourceField::EnterpriseDuration5,
        ResourceField::EnterpriseDuration6,
        ResourceField::EnterpriseDuration7,
        ResourceField::EnterpriseDuration8,
        ResourceField::EnterpriseDuration9,
        ResourceField::EnterpriseDuration10,
        ResourceField::EnterpriseFlag1,
        ResourceField::EnterpriseFlag2,
        ResourceField::EnterpriseFlag3,
        ResourceField::EnterpriseFlag4,
        ResourceField::EnterpriseFlag5,
        ResourceField::EnterpriseFlag6,
        ResourceField::EnterpriseFlag7,
        ResourceField::EnterpriseFlag8,
        ResourceField::EnterpriseFlag9,
        ResourceField::EnterpriseFlag10,
        ResourceField::EnterpriseFlag11,
        ResourceField::EnterpriseFlag12,
        ResourceField::EnterpriseFlag13,
        ResourceField::EnterpriseFlag14,
        ResourceField::EnterpriseFlag15,
        ResourceField::EnterpriseFlag16,
        ResourceField::EnterpriseFlag17,
        ResourceField::EnterpriseFlag18,
        ResourceField::EnterpriseFlag19,
        ResourceField::EnterpriseFlag20,
        ResourceField::EnterpriseNumber1,
        ResourceField::EnterpriseNumber2,
        ResourceField::EnterpriseNumber3,
        ResourceField::EnterpriseNumber4,
        ResourceField::EnterpriseNumber5,
        ResourceField::EnterpriseNumber6,
        ResourceField::EnterpriseNumber7,
        ResourceField::EnterpriseNumber8,
        ResourceField::EnterpriseNumber9,
        ResourceField::EnterpriseNumber10,
        ResourceField::EnterpriseNumber11,
        ResourceField::EnterpriseNumber12,
        ResourceField::EnterpriseNumber13,
        ResourceField::EnterpriseNumber14,
        ResourceField::EnterpriseNumber15,
        ResourceField::EnterpriseNumber16,
        ResourceField::EnterpriseNumber17,
        ResourceField::EnterpriseNumber18,
        ResourceField::EnterpriseNumber19,
        ResourceField::EnterpriseNumber20,
        ResourceField::EnterpriseNumber21,
        ResourceField::EnterpriseNumber22,
        ResourceField::EnterpriseNumber23,
        ResourceField::EnterpriseNumber24,
        ResourceField::EnterpriseNumber25,
        ResourceField::EnterpriseNumber26,
        ResourceField::EnterpriseNumber27,
        ResourceField::EnterpriseNumber28,
        ResourceField::EnterpriseNumber29,
        ResourceField::EnterpriseNumber30,
        ResourceField::EnterpriseNumber31,
        ResourceField::EnterpriseNumber32,
        ResourceField::EnterpriseNumber33,
        ResourceField::EnterpriseNumber34,
        ResourceField::EnterpriseNumber35,
        ResourceField::EnterpriseNumber36,
        ResourceField::EnterpriseNumber37,
        ResourceField::EnterpriseNumber38,
        ResourceField::EnterpriseNumber39,
        ResourceField::EnterpriseNumber40,
        ResourceField::EnterpriseText1,
        ResourceField::EnterpriseText2,
        ResourceField::EnterpriseText3,
        ResourceField::EnterpriseText4,
        ResourceField::EnterpriseText5,
        ResourceField::EnterpriseText6,
        ResourceField::EnterpriseText7,
        ResourceField::EnterpriseText8,
        ResourceField::EnterpriseText9,
        ResourceField::EnterpriseText10,
        ResourceField::EnterpriseText11,
        ResourceField::EnterpriseText12,
        ResourceField::EnterpriseText13,
        ResourceField::EnterpriseText14,
        ResourceField::EnterpriseText15,
        ResourceField::EnterpriseText16,
        ResourceField::EnterpriseText17,
        ResourceField::EnterpriseText18,
        ResourceField::EnterpriseText19,
        ResourceField::EnterpriseText20,
        ResourceField::EnterpriseText21,
        ResourceField::EnterpriseText22,
        ResourceField::EnterpriseText23,
        ResourceField::EnterpriseText24,
        ResourceField::EnterpriseText25,
        ResourceField::EnterpriseText26,
        ResourceField::EnterpriseText27,
        ResourceField::EnterpriseText28,
        ResourceField::EnterpriseText29,
        ResourceField::EnterpriseText30,
        ResourceField::EnterpriseText31,
        ResourceField::EnterpriseText32,
        ResourceField::EnterpriseText33,
        ResourceField::EnterpriseText34,
        ResourceField::EnterpriseText35,
        ResourceField::EnterpriseText36,
        ResourceField::EnterpriseText37,
        ResourceField::EnterpriseText38,
        ResourceField::EnterpriseText39,
        ResourceField::EnterpriseText40,
        ResourceField::Enterprise,
        ResourceField::Import,
        ResourceField::Created,
        ResourceField::Budget,
        ResourceField::TaskOutlineNumber,
        ResourceField::Guid,
        ResourceField::Active,
        ResourceField::AssignmentOwner,
        ResourceField::BaselineBudgetCost,
        ResourceField::BaselineBudgetWork,
        ResourceField::Baseline1BudgetCost,
        ResourceField::Baseline1BudgetWork,
        ResourceField::Baseline1Finish,
        ResourceField::Baseline1Start,
        ResourceField::Baseline2BudgetCost,
        ResourceField::Baseline2BudgetWork,
        ResourceField::Baseline2Finish,
        ResourceField::Baseline2Start,
        ResourceField::Baseline3BudgetCost,
        ResourceField::Baseline3BudgetWork,
        ResourceField::Baseline3Finish,
        ResourceField::Baseline3Start,
        ResourceField::Baseline4BudgetCost,
        ResourceField::Baseline4BudgetWork,
        ResourceField::Baseline4Finish,
        ResourceField::Baseline4Start,
        ResourceField::Baseline5BudgetCost,
        ResourceField::Baseline5BudgetWork,
        ResourceField::Baseline5Finish,
        ResourceField::Baseline5Start,
        ResourceField::Baseline6BudgetCost,
        ResourceField::Baseline6BudgetWork,
        ResourceField::Baseline6Finish,
        ResourceField::Baseline6Start,
        ResourceField::Baseline7BudgetCost,
        ResourceField::Baseline7BudgetWork,
        ResourceField::Baseline7Finish,
        ResourceField::Baseline7Start,
        ResourceField::Baseline8BudgetCost,
        ResourceField::Baseline8BudgetWork,
        ResourceField::Baseline8Finish,
        ResourceField::Baseline8Start,
        ResourceField::Baseline9BudgetCost,
        ResourceField::Baseline9BudgetWork,
        ResourceField::Baseline9Finish,
        ResourceField::Baseline9Start,
        ResourceField::Baseline10BudgetCost,
        ResourceField::Baseline10BudgetWork,
        ResourceField::Baseline10Finish,
        ResourceField::Baseline10Start,
        ResourceField::BookingType,
        ResourceField::BudgetCost,
        ResourceField::BudgetWork,
        ResourceField::CalendarGuid,
        ResourceField::CostCenter,
        ResourceField::DefaultAssignmentOwner,
        ResourceField::EnterpriseBaseCalendar,
        ResourceField::EnterpriseRequiredValues,
        ResourceField::EnterpriseTeamMember,
        ResourceField::EnterpriseUniqueId,
        ResourceField::ErrorMessage,
        ResourceField::Generic,
        ResourceField::Inactive,
        ResourceField::RequestDemand,
        ResourceField::Summary,
        ResourceField::TeamAssignmentPool,
        ResourceField::Wbs,
        ResourceField::Unavailable,
        ResourceField::SubprojectResourceUniqueId,
        ResourceField::HyperlinkData,
        ResourceField::OutlineCode1Index,
        ResourceField::OutlineCode2Index,
        ResourceField::OutlineCode3Index,
        ResourceField::OutlineCode4Index,
        ResourceField::OutlineCode5Index,
        ResourceField::OutlineCode6Index,
        ResourceField::OutlineCode7Index,
        ResourceField::OutlineCode8Index,
        ResourceField::OutlineCode9Index,
        ResourceField::OutlineCode10Index,
        ResourceField::CostRateA,
        ResourceField::CostRateB,
        ResourceField::CostRateC,
        ResourceField::CostRateD,
        ResourceField::CostRateE,
        ResourceField::AvailabilityData,
        ResourceField::EnterpriseData,
        ResourceField::OvertimeRateUnits,
        ResourceField::StandardRateUnits,
        ResourceField::Index,
        ResourceField::HyperlinkScreenTip,
        ResourceField::EnterpriseOutlineCode1,
        ResourceField::EnterpriseOutlineCode2,
        ResourceField::EnterpriseOutlineCode3,
        ResourceField::EnterpriseOutlineCode4,
        ResourceField::EnterpriseOutlineCode5,
        ResourceField::EnterpriseOutlineCode6,
        ResourceField::EnterpriseOutlineCode7,
        ResourceField::EnterpriseOutlineCode8,
        ResourceField::EnterpriseOutlineCode9,
        ResourceField::EnterpriseOutlineCode10,
        ResourceField::EnterpriseOutlineCode11,
        ResourceField::EnterpriseOutlineCode12,
        ResourceField::EnterpriseOutlineCode13,
        ResourceField::EnterpriseOutlineCode14,
        ResourceField::EnterpriseOutlineCode15,
        ResourceField::EnterpriseOutlineCode16,
        ResourceField::EnterpriseOutlineCode17,
        ResourceField::EnterpriseOutlineCode18,
        ResourceField::EnterpriseOutlineCode19,
        ResourceField::EnterpriseOutlineCode20,
        ResourceField::EnterpriseOutlineCode21,
        ResourceField::EnterpriseOutlineCode22,
        ResourceField::EnterpriseOutlineCode23,
        ResourceField::EnterpriseOutlineCode24,
        ResourceField::EnterpriseOutlineCode25,
        ResourceField::EnterpriseOutlineCode26,
        ResourceField::EnterpriseOutlineCode27,
        ResourceField::EnterpriseOutlineCode28,
        ResourceField::EnterpriseOutlineCode29,
        ResourceField::EnterpriseRbs,
        ResourceField::EnterpriseNameUsed,
        ResourceField::EnterpriseIsCheckedOut,
        ResourceField::EnterpriseCheckedOutBy,
        ResourceField::EnterpriseLastModifiedDate,
        ResourceField::EnterpriseMultiValue20,
        ResourceField::EnterpriseMultiValue21,
        ResourceField::EnterpriseMultiValue22,
        ResourceField::EnterpriseMultiValue23,
        ResourceField::EnterpriseMultiValue24,
        ResourceField::EnterpriseMultiValue25,
        ResourceField::EnterpriseMultiValue26,
        ResourceField::EnterpriseMultiValue27,
        ResourceField::EnterpriseMultiValue28,
        ResourceField::EnterpriseMultiValue29,
        ResourceField::ActualWorkProtected,
        ResourceField::ActualOvertimeWorkProtected,
        ResourceField::ParentId,
        ResourceField::CalendarUniqueId,
        ResourceField::ProposedStart,
        ResourceField::ProposedFinish,
        ResourceField::ProposedMaxUnits,
        ResourceField::EngagementStatus,
        ResourceField::Unit,
        ResourceField::SupplyReference,
        ResourceField::Description,
        ResourceField::ResourceId,
        ResourceField::ModifyOnIntegrate,
        ResourceField::ExpensesOnly,
        ResourceField::PeriodDur,
        ResourceField::Priority,
        ResourceField::Rate,
        ResourceField::Pool,
        ResourceField::PerDay,
        ResourceField::Phone,
        ResourceField::Role,
        ResourceField::CalculateCostsFromUnits,
        ResourceField::SequenceNumber,
        ResourceField::LocationUniqueId,
        ResourceField::UnitOfMeasureUniqueId,
        ResourceField::DefaultUnits,
        ResourceField::ShiftUniqueId,
        ResourceField::PrimaryRoleUniqueId,
        ResourceField::Finish,
        ResourceField::ResourceCodeValues,
        ResourceField::RoleCodeValues,
        ResourceField::CurrencyUniqueId,
        ResourceField::PlannedWork,
        ResourceField::ActualRegularWork,
        ResourceField::RemainingRegularWork,
        ResourceField::ActualRegularCost,
        ResourceField::RemainingRegularCost,
        ResourceField::PlannedMaterial,
        ResourceField::ActualMaterial,
        ResourceField::RemainingMaterial,
        ResourceField::Material,
        ResourceField::BaselineMaterial,
        ResourceField::Baseline1Material,
        ResourceField::Baseline2Material,
        ResourceField::Baseline3Material,
        ResourceField::Baseline4Material,
        ResourceField::Baseline5Material,
        ResourceField::Baseline6Material,
        ResourceField::Baseline7Material,
        ResourceField::Baseline8Material,
        ResourceField::Baseline9Material,
        ResourceField::Baseline10Material,
        ResourceField::PlannedCost,
    ];

    /// Returns the enum instance for the given int value, returning `None` when out of bounds.
    ///
    /// Corresponds to Java: `public static ResourceField getInstance(int type)`
    /// (Java returns `null` out of bounds).
    pub fn get_instance(value: i32) -> Option<ResourceField> {
        if value >= 0 && (value as usize) < Self::MAX_VALUE {
            Some(Self::ALL[value as usize])
        } else {
            None
        }
    }
}

impl FieldType for ResourceField {
    /// Corresponds to Java: `public FieldTypeClass getFieldTypeClass()`
    fn get_field_type_class(&self) -> FieldTypeClass {
        FieldTypeClass::Resource
    }

    /// Corresponds to the `name()` provided by the Java enum
    fn name(&self) -> &'static str {
        match self {
            ResourceField::Start => "START",
            ResourceField::Duration1Units => "DURATION1_UNITS",
            ResourceField::Duration2Units => "DURATION2_UNITS",
            ResourceField::Duration3Units => "DURATION3_UNITS",
            ResourceField::Duration4Units => "DURATION4_UNITS",
            ResourceField::Duration5Units => "DURATION5_UNITS",
            ResourceField::Duration6Units => "DURATION6_UNITS",
            ResourceField::Duration7Units => "DURATION7_UNITS",
            ResourceField::Duration8Units => "DURATION8_UNITS",
            ResourceField::Duration9Units => "DURATION9_UNITS",
            ResourceField::Duration10Units => "DURATION10_UNITS",
            ResourceField::Id => "ID",
            ResourceField::Name => "NAME",
            ResourceField::Initials => "INITIALS",
            ResourceField::Group => "GROUP",
            ResourceField::MaxUnits => "MAX_UNITS",
            ResourceField::BaseCalendar => "BASE_CALENDAR",
            ResourceField::StandardRate => "STANDARD_RATE",
            ResourceField::OvertimeRate => "OVERTIME_RATE",
            ResourceField::Text1 => "TEXT1",
            ResourceField::Text2 => "TEXT2",
            ResourceField::Code => "CODE",
            ResourceField::ActualCost => "ACTUAL_COST",
            ResourceField::Cost => "COST",
            ResourceField::Work => "WORK",
            ResourceField::ActualWork => "ACTUAL_WORK",
            ResourceField::BaselineWork => "BASELINE_WORK",
            ResourceField::OvertimeWork => "OVERTIME_WORK",
            ResourceField::BaselineCost => "BASELINE_COST",
            ResourceField::CostPerUse => "COST_PER_USE",
            ResourceField::AccrueAt => "ACCRUE_AT",
            ResourceField::RemainingCost => "REMAINING_COST",
            ResourceField::RemainingWork => "REMAINING_WORK",
            ResourceField::WorkVariance => "WORK_VARIANCE",
            ResourceField::CostVariance => "COST_VARIANCE",
            ResourceField::Overallocated => "OVERALLOCATED",
            ResourceField::Peak => "PEAK",
            ResourceField::UniqueId => "UNIQUE_ID",
            ResourceField::Notes => "NOTES",
            ResourceField::PercentWorkComplete => "PERCENT_WORK_COMPLETE",
            ResourceField::Text3 => "TEXT3",
            ResourceField::Text4 => "TEXT4",
            ResourceField::Text5 => "TEXT5",
            ResourceField::Objects => "OBJECTS",
            ResourceField::LinkedFields => "LINKED_FIELDS",
            ResourceField::EmailAddress => "EMAIL_ADDRESS",
            ResourceField::RegularWork => "REGULAR_WORK",
            ResourceField::ActualOvertimeWork => "ACTUAL_OVERTIME_WORK",
            ResourceField::RemainingOvertimeWork => "REMAINING_OVERTIME_WORK",
            ResourceField::OvertimeCost => "OVERTIME_COST",
            ResourceField::ActualOvertimeCost => "ACTUAL_OVERTIME_COST",
            ResourceField::RemainingOvertimeCost => "REMAINING_OVERTIME_COST",
            ResourceField::Bcws => "BCWS",
            ResourceField::Bcwp => "BCWP",
            ResourceField::Acwp => "ACWP",
            ResourceField::Sv => "SV",
            ResourceField::AvailableFrom => "AVAILABLE_FROM",
            ResourceField::AvailableTo => "AVAILABLE_TO",
            ResourceField::Indicators => "INDICATORS",
            ResourceField::Text6 => "TEXT6",
            ResourceField::Text7 => "TEXT7",
            ResourceField::Text8 => "TEXT8",
            ResourceField::Text9 => "TEXT9",
            ResourceField::Text10 => "TEXT10",
            ResourceField::Start1 => "START1",
            ResourceField::Start2 => "START2",
            ResourceField::Start3 => "START3",
            ResourceField::Start4 => "START4",
            ResourceField::Start5 => "START5",
            ResourceField::Finish1 => "FINISH1",
            ResourceField::Finish2 => "FINISH2",
            ResourceField::Finish3 => "FINISH3",
            ResourceField::Finish4 => "FINISH4",
            ResourceField::Finish5 => "FINISH5",
            ResourceField::Number1 => "NUMBER1",
            ResourceField::Number2 => "NUMBER2",
            ResourceField::Number3 => "NUMBER3",
            ResourceField::Number4 => "NUMBER4",
            ResourceField::Number5 => "NUMBER5",
            ResourceField::Duration1 => "DURATION1",
            ResourceField::Duration2 => "DURATION2",
            ResourceField::Duration3 => "DURATION3",
            ResourceField::Cost1 => "COST1",
            ResourceField::Cost2 => "COST2",
            ResourceField::Cost3 => "COST3",
            ResourceField::Flag10 => "FLAG10",
            ResourceField::Flag1 => "FLAG1",
            ResourceField::Flag2 => "FLAG2",
            ResourceField::Flag3 => "FLAG3",
            ResourceField::Flag4 => "FLAG4",
            ResourceField::Flag5 => "FLAG5",
            ResourceField::Flag6 => "FLAG6",
            ResourceField::Flag7 => "FLAG7",
            ResourceField::Flag8 => "FLAG8",
            ResourceField::Flag9 => "FLAG9",
            ResourceField::Hyperlink => "HYPERLINK",
            ResourceField::HyperlinkAddress => "HYPERLINK_ADDRESS",
            ResourceField::HyperlinkSubaddress => "HYPERLINK_SUBADDRESS",
            ResourceField::HyperlinkHref => "HYPERLINK_HREF",
            ResourceField::Assignment => "ASSIGNMENT",
            ResourceField::TaskSummaryName => "TASK_SUMMARY_NAME",
            ResourceField::CanLevel => "CAN_LEVEL",
            ResourceField::WorkContour => "WORK_CONTOUR",
            ResourceField::Cost4 => "COST4",
            ResourceField::Cost5 => "COST5",
            ResourceField::Cost6 => "COST6",
            ResourceField::Cost7 => "COST7",
            ResourceField::Cost8 => "COST8",
            ResourceField::Cost9 => "COST9",
            ResourceField::Cost10 => "COST10",
            ResourceField::Date1 => "DATE1",
            ResourceField::Date2 => "DATE2",
            ResourceField::Date3 => "DATE3",
            ResourceField::Date4 => "DATE4",
            ResourceField::Date5 => "DATE5",
            ResourceField::Date6 => "DATE6",
            ResourceField::Date7 => "DATE7",
            ResourceField::Date8 => "DATE8",
            ResourceField::Date9 => "DATE9",
            ResourceField::Date10 => "DATE10",
            ResourceField::Duration4 => "DURATION4",
            ResourceField::Duration5 => "DURATION5",
            ResourceField::Duration6 => "DURATION6",
            ResourceField::Duration7 => "DURATION7",
            ResourceField::Duration8 => "DURATION8",
            ResourceField::Duration9 => "DURATION9",
            ResourceField::Duration10 => "DURATION10",
            ResourceField::Finish6 => "FINISH6",
            ResourceField::Finish7 => "FINISH7",
            ResourceField::Finish8 => "FINISH8",
            ResourceField::Finish9 => "FINISH9",
            ResourceField::Finish10 => "FINISH10",
            ResourceField::Flag11 => "FLAG11",
            ResourceField::Flag12 => "FLAG12",
            ResourceField::Flag13 => "FLAG13",
            ResourceField::Flag14 => "FLAG14",
            ResourceField::Flag15 => "FLAG15",
            ResourceField::Flag16 => "FLAG16",
            ResourceField::Flag17 => "FLAG17",
            ResourceField::Flag18 => "FLAG18",
            ResourceField::Flag19 => "FLAG19",
            ResourceField::Flag20 => "FLAG20",
            ResourceField::Number6 => "NUMBER6",
            ResourceField::Number7 => "NUMBER7",
            ResourceField::Number8 => "NUMBER8",
            ResourceField::Number9 => "NUMBER9",
            ResourceField::Number10 => "NUMBER10",
            ResourceField::Number11 => "NUMBER11",
            ResourceField::Number12 => "NUMBER12",
            ResourceField::Number13 => "NUMBER13",
            ResourceField::Number14 => "NUMBER14",
            ResourceField::Number15 => "NUMBER15",
            ResourceField::Number16 => "NUMBER16",
            ResourceField::Number17 => "NUMBER17",
            ResourceField::Number18 => "NUMBER18",
            ResourceField::Number19 => "NUMBER19",
            ResourceField::Number20 => "NUMBER20",
            ResourceField::Start6 => "START6",
            ResourceField::Start7 => "START7",
            ResourceField::Start8 => "START8",
            ResourceField::Start9 => "START9",
            ResourceField::Start10 => "START10",
            ResourceField::Text11 => "TEXT11",
            ResourceField::Text12 => "TEXT12",
            ResourceField::Text13 => "TEXT13",
            ResourceField::Text14 => "TEXT14",
            ResourceField::Text15 => "TEXT15",
            ResourceField::Text16 => "TEXT16",
            ResourceField::Text17 => "TEXT17",
            ResourceField::Text18 => "TEXT18",
            ResourceField::Text19 => "TEXT19",
            ResourceField::Text20 => "TEXT20",
            ResourceField::Text21 => "TEXT21",
            ResourceField::Text22 => "TEXT22",
            ResourceField::Text23 => "TEXT23",
            ResourceField::Text24 => "TEXT24",
            ResourceField::Text25 => "TEXT25",
            ResourceField::Text26 => "TEXT26",
            ResourceField::Text27 => "TEXT27",
            ResourceField::Text28 => "TEXT28",
            ResourceField::Text29 => "TEXT29",
            ResourceField::Text30 => "TEXT30",
            ResourceField::Phonetics => "PHONETICS",
            ResourceField::AssignmentDelay => "ASSIGNMENT_DELAY",
            ResourceField::AssignmentUnits => "ASSIGNMENT_UNITS",
            ResourceField::BaselineStart => "BASELINE_START",
            ResourceField::BaselineFinish => "BASELINE_FINISH",
            ResourceField::Confirmed => "CONFIRMED",
            ResourceField::LevelingDelay => "LEVELING_DELAY",
            ResourceField::ResponsePending => "RESPONSE_PENDING",
            ResourceField::TeamStatusPending => "TEAM_STATUS_PENDING",
            ResourceField::Cv => "CV",
            ResourceField::UpdateNeeded => "UPDATE_NEEDED",
            ResourceField::CostRateTable => "COST_RATE_TABLE",
            ResourceField::ActualStart => "ACTUAL_START",
            ResourceField::ActualFinish => "ACTUAL_FINISH",
            ResourceField::Workgroup => "WORKGROUP",
            ResourceField::Project => "PROJECT",
            ResourceField::OutlineCode1 => "OUTLINE_CODE1",
            ResourceField::OutlineCode2 => "OUTLINE_CODE2",
            ResourceField::OutlineCode3 => "OUTLINE_CODE3",
            ResourceField::OutlineCode4 => "OUTLINE_CODE4",
            ResourceField::OutlineCode5 => "OUTLINE_CODE5",
            ResourceField::OutlineCode6 => "OUTLINE_CODE6",
            ResourceField::OutlineCode7 => "OUTLINE_CODE7",
            ResourceField::OutlineCode8 => "OUTLINE_CODE8",
            ResourceField::OutlineCode9 => "OUTLINE_CODE9",
            ResourceField::OutlineCode10 => "OUTLINE_CODE10",
            ResourceField::MaterialLabel => "MATERIAL_LABEL",
            ResourceField::Type => "TYPE",
            ResourceField::Vac => "VAC",
            ResourceField::GroupBySummary => "GROUP_BY_SUMMARY",
            ResourceField::WindowsUserAccount => "WINDOWS_USER_ACCOUNT",
            ResourceField::Baseline1Work => "BASELINE1_WORK",
            ResourceField::Baseline1Cost => "BASELINE1_COST",
            ResourceField::Baseline2Work => "BASELINE2_WORK",
            ResourceField::Baseline2Cost => "BASELINE2_COST",
            ResourceField::Baseline3Work => "BASELINE3_WORK",
            ResourceField::Baseline3Cost => "BASELINE3_COST",
            ResourceField::Baseline4Work => "BASELINE4_WORK",
            ResourceField::Baseline4Cost => "BASELINE4_COST",
            ResourceField::Baseline5Work => "BASELINE5_WORK",
            ResourceField::Baseline5Cost => "BASELINE5_COST",
            ResourceField::Baseline6Work => "BASELINE6_WORK",
            ResourceField::Baseline6Cost => "BASELINE6_COST",
            ResourceField::Baseline7Work => "BASELINE7_WORK",
            ResourceField::Baseline7Cost => "BASELINE7_COST",
            ResourceField::Baseline8Work => "BASELINE8_WORK",
            ResourceField::Baseline8Cost => "BASELINE8_COST",
            ResourceField::Baseline9Work => "BASELINE9_WORK",
            ResourceField::Baseline9Cost => "BASELINE9_COST",
            ResourceField::Baseline10Work => "BASELINE10_WORK",
            ResourceField::Baseline10Cost => "BASELINE10_COST",
            ResourceField::EnterpriseCost1 => "ENTERPRISE_COST1",
            ResourceField::EnterpriseCost2 => "ENTERPRISE_COST2",
            ResourceField::EnterpriseCost3 => "ENTERPRISE_COST3",
            ResourceField::EnterpriseCost4 => "ENTERPRISE_COST4",
            ResourceField::EnterpriseCost5 => "ENTERPRISE_COST5",
            ResourceField::EnterpriseCost6 => "ENTERPRISE_COST6",
            ResourceField::EnterpriseCost7 => "ENTERPRISE_COST7",
            ResourceField::EnterpriseCost8 => "ENTERPRISE_COST8",
            ResourceField::EnterpriseCost9 => "ENTERPRISE_COST9",
            ResourceField::EnterpriseCost10 => "ENTERPRISE_COST10",
            ResourceField::EnterpriseDate1 => "ENTERPRISE_DATE1",
            ResourceField::EnterpriseDate2 => "ENTERPRISE_DATE2",
            ResourceField::EnterpriseDate3 => "ENTERPRISE_DATE3",
            ResourceField::EnterpriseDate4 => "ENTERPRISE_DATE4",
            ResourceField::EnterpriseDate5 => "ENTERPRISE_DATE5",
            ResourceField::EnterpriseDate6 => "ENTERPRISE_DATE6",
            ResourceField::EnterpriseDate7 => "ENTERPRISE_DATE7",
            ResourceField::EnterpriseDate8 => "ENTERPRISE_DATE8",
            ResourceField::EnterpriseDate9 => "ENTERPRISE_DATE9",
            ResourceField::EnterpriseDate10 => "ENTERPRISE_DATE10",
            ResourceField::EnterpriseDate11 => "ENTERPRISE_DATE11",
            ResourceField::EnterpriseDate12 => "ENTERPRISE_DATE12",
            ResourceField::EnterpriseDate13 => "ENTERPRISE_DATE13",
            ResourceField::EnterpriseDate14 => "ENTERPRISE_DATE14",
            ResourceField::EnterpriseDate15 => "ENTERPRISE_DATE15",
            ResourceField::EnterpriseDate16 => "ENTERPRISE_DATE16",
            ResourceField::EnterpriseDate17 => "ENTERPRISE_DATE17",
            ResourceField::EnterpriseDate18 => "ENTERPRISE_DATE18",
            ResourceField::EnterpriseDate19 => "ENTERPRISE_DATE19",
            ResourceField::EnterpriseDate20 => "ENTERPRISE_DATE20",
            ResourceField::EnterpriseDate21 => "ENTERPRISE_DATE21",
            ResourceField::EnterpriseDate22 => "ENTERPRISE_DATE22",
            ResourceField::EnterpriseDate23 => "ENTERPRISE_DATE23",
            ResourceField::EnterpriseDate24 => "ENTERPRISE_DATE24",
            ResourceField::EnterpriseDate25 => "ENTERPRISE_DATE25",
            ResourceField::EnterpriseDate26 => "ENTERPRISE_DATE26",
            ResourceField::EnterpriseDate27 => "ENTERPRISE_DATE27",
            ResourceField::EnterpriseDate28 => "ENTERPRISE_DATE28",
            ResourceField::EnterpriseDate29 => "ENTERPRISE_DATE29",
            ResourceField::EnterpriseDate30 => "ENTERPRISE_DATE30",
            ResourceField::EnterpriseDuration1Units => "ENTERPRISE_DURATION1_UNITS",
            ResourceField::EnterpriseDuration2Units => "ENTERPRISE_DURATION2_UNITS",
            ResourceField::EnterpriseDuration3Units => "ENTERPRISE_DURATION3_UNITS",
            ResourceField::EnterpriseDuration4Units => "ENTERPRISE_DURATION4_UNITS",
            ResourceField::EnterpriseDuration5Units => "ENTERPRISE_DURATION5_UNITS",
            ResourceField::EnterpriseDuration6Units => "ENTERPRISE_DURATION6_UNITS",
            ResourceField::EnterpriseDuration7Units => "ENTERPRISE_DURATION7_UNITS",
            ResourceField::EnterpriseDuration8Units => "ENTERPRISE_DURATION8_UNITS",
            ResourceField::EnterpriseDuration9Units => "ENTERPRISE_DURATION9_UNITS",
            ResourceField::EnterpriseDuration10Units => "ENTERPRISE_DURATION10_UNITS",
            ResourceField::EnterpriseDuration1 => "ENTERPRISE_DURATION1",
            ResourceField::EnterpriseDuration2 => "ENTERPRISE_DURATION2",
            ResourceField::EnterpriseDuration3 => "ENTERPRISE_DURATION3",
            ResourceField::EnterpriseDuration4 => "ENTERPRISE_DURATION4",
            ResourceField::EnterpriseDuration5 => "ENTERPRISE_DURATION5",
            ResourceField::EnterpriseDuration6 => "ENTERPRISE_DURATION6",
            ResourceField::EnterpriseDuration7 => "ENTERPRISE_DURATION7",
            ResourceField::EnterpriseDuration8 => "ENTERPRISE_DURATION8",
            ResourceField::EnterpriseDuration9 => "ENTERPRISE_DURATION9",
            ResourceField::EnterpriseDuration10 => "ENTERPRISE_DURATION10",
            ResourceField::EnterpriseFlag1 => "ENTERPRISE_FLAG1",
            ResourceField::EnterpriseFlag2 => "ENTERPRISE_FLAG2",
            ResourceField::EnterpriseFlag3 => "ENTERPRISE_FLAG3",
            ResourceField::EnterpriseFlag4 => "ENTERPRISE_FLAG4",
            ResourceField::EnterpriseFlag5 => "ENTERPRISE_FLAG5",
            ResourceField::EnterpriseFlag6 => "ENTERPRISE_FLAG6",
            ResourceField::EnterpriseFlag7 => "ENTERPRISE_FLAG7",
            ResourceField::EnterpriseFlag8 => "ENTERPRISE_FLAG8",
            ResourceField::EnterpriseFlag9 => "ENTERPRISE_FLAG9",
            ResourceField::EnterpriseFlag10 => "ENTERPRISE_FLAG10",
            ResourceField::EnterpriseFlag11 => "ENTERPRISE_FLAG11",
            ResourceField::EnterpriseFlag12 => "ENTERPRISE_FLAG12",
            ResourceField::EnterpriseFlag13 => "ENTERPRISE_FLAG13",
            ResourceField::EnterpriseFlag14 => "ENTERPRISE_FLAG14",
            ResourceField::EnterpriseFlag15 => "ENTERPRISE_FLAG15",
            ResourceField::EnterpriseFlag16 => "ENTERPRISE_FLAG16",
            ResourceField::EnterpriseFlag17 => "ENTERPRISE_FLAG17",
            ResourceField::EnterpriseFlag18 => "ENTERPRISE_FLAG18",
            ResourceField::EnterpriseFlag19 => "ENTERPRISE_FLAG19",
            ResourceField::EnterpriseFlag20 => "ENTERPRISE_FLAG20",
            ResourceField::EnterpriseNumber1 => "ENTERPRISE_NUMBER1",
            ResourceField::EnterpriseNumber2 => "ENTERPRISE_NUMBER2",
            ResourceField::EnterpriseNumber3 => "ENTERPRISE_NUMBER3",
            ResourceField::EnterpriseNumber4 => "ENTERPRISE_NUMBER4",
            ResourceField::EnterpriseNumber5 => "ENTERPRISE_NUMBER5",
            ResourceField::EnterpriseNumber6 => "ENTERPRISE_NUMBER6",
            ResourceField::EnterpriseNumber7 => "ENTERPRISE_NUMBER7",
            ResourceField::EnterpriseNumber8 => "ENTERPRISE_NUMBER8",
            ResourceField::EnterpriseNumber9 => "ENTERPRISE_NUMBER9",
            ResourceField::EnterpriseNumber10 => "ENTERPRISE_NUMBER10",
            ResourceField::EnterpriseNumber11 => "ENTERPRISE_NUMBER11",
            ResourceField::EnterpriseNumber12 => "ENTERPRISE_NUMBER12",
            ResourceField::EnterpriseNumber13 => "ENTERPRISE_NUMBER13",
            ResourceField::EnterpriseNumber14 => "ENTERPRISE_NUMBER14",
            ResourceField::EnterpriseNumber15 => "ENTERPRISE_NUMBER15",
            ResourceField::EnterpriseNumber16 => "ENTERPRISE_NUMBER16",
            ResourceField::EnterpriseNumber17 => "ENTERPRISE_NUMBER17",
            ResourceField::EnterpriseNumber18 => "ENTERPRISE_NUMBER18",
            ResourceField::EnterpriseNumber19 => "ENTERPRISE_NUMBER19",
            ResourceField::EnterpriseNumber20 => "ENTERPRISE_NUMBER20",
            ResourceField::EnterpriseNumber21 => "ENTERPRISE_NUMBER21",
            ResourceField::EnterpriseNumber22 => "ENTERPRISE_NUMBER22",
            ResourceField::EnterpriseNumber23 => "ENTERPRISE_NUMBER23",
            ResourceField::EnterpriseNumber24 => "ENTERPRISE_NUMBER24",
            ResourceField::EnterpriseNumber25 => "ENTERPRISE_NUMBER25",
            ResourceField::EnterpriseNumber26 => "ENTERPRISE_NUMBER26",
            ResourceField::EnterpriseNumber27 => "ENTERPRISE_NUMBER27",
            ResourceField::EnterpriseNumber28 => "ENTERPRISE_NUMBER28",
            ResourceField::EnterpriseNumber29 => "ENTERPRISE_NUMBER29",
            ResourceField::EnterpriseNumber30 => "ENTERPRISE_NUMBER30",
            ResourceField::EnterpriseNumber31 => "ENTERPRISE_NUMBER31",
            ResourceField::EnterpriseNumber32 => "ENTERPRISE_NUMBER32",
            ResourceField::EnterpriseNumber33 => "ENTERPRISE_NUMBER33",
            ResourceField::EnterpriseNumber34 => "ENTERPRISE_NUMBER34",
            ResourceField::EnterpriseNumber35 => "ENTERPRISE_NUMBER35",
            ResourceField::EnterpriseNumber36 => "ENTERPRISE_NUMBER36",
            ResourceField::EnterpriseNumber37 => "ENTERPRISE_NUMBER37",
            ResourceField::EnterpriseNumber38 => "ENTERPRISE_NUMBER38",
            ResourceField::EnterpriseNumber39 => "ENTERPRISE_NUMBER39",
            ResourceField::EnterpriseNumber40 => "ENTERPRISE_NUMBER40",
            ResourceField::EnterpriseText1 => "ENTERPRISE_TEXT1",
            ResourceField::EnterpriseText2 => "ENTERPRISE_TEXT2",
            ResourceField::EnterpriseText3 => "ENTERPRISE_TEXT3",
            ResourceField::EnterpriseText4 => "ENTERPRISE_TEXT4",
            ResourceField::EnterpriseText5 => "ENTERPRISE_TEXT5",
            ResourceField::EnterpriseText6 => "ENTERPRISE_TEXT6",
            ResourceField::EnterpriseText7 => "ENTERPRISE_TEXT7",
            ResourceField::EnterpriseText8 => "ENTERPRISE_TEXT8",
            ResourceField::EnterpriseText9 => "ENTERPRISE_TEXT9",
            ResourceField::EnterpriseText10 => "ENTERPRISE_TEXT10",
            ResourceField::EnterpriseText11 => "ENTERPRISE_TEXT11",
            ResourceField::EnterpriseText12 => "ENTERPRISE_TEXT12",
            ResourceField::EnterpriseText13 => "ENTERPRISE_TEXT13",
            ResourceField::EnterpriseText14 => "ENTERPRISE_TEXT14",
            ResourceField::EnterpriseText15 => "ENTERPRISE_TEXT15",
            ResourceField::EnterpriseText16 => "ENTERPRISE_TEXT16",
            ResourceField::EnterpriseText17 => "ENTERPRISE_TEXT17",
            ResourceField::EnterpriseText18 => "ENTERPRISE_TEXT18",
            ResourceField::EnterpriseText19 => "ENTERPRISE_TEXT19",
            ResourceField::EnterpriseText20 => "ENTERPRISE_TEXT20",
            ResourceField::EnterpriseText21 => "ENTERPRISE_TEXT21",
            ResourceField::EnterpriseText22 => "ENTERPRISE_TEXT22",
            ResourceField::EnterpriseText23 => "ENTERPRISE_TEXT23",
            ResourceField::EnterpriseText24 => "ENTERPRISE_TEXT24",
            ResourceField::EnterpriseText25 => "ENTERPRISE_TEXT25",
            ResourceField::EnterpriseText26 => "ENTERPRISE_TEXT26",
            ResourceField::EnterpriseText27 => "ENTERPRISE_TEXT27",
            ResourceField::EnterpriseText28 => "ENTERPRISE_TEXT28",
            ResourceField::EnterpriseText29 => "ENTERPRISE_TEXT29",
            ResourceField::EnterpriseText30 => "ENTERPRISE_TEXT30",
            ResourceField::EnterpriseText31 => "ENTERPRISE_TEXT31",
            ResourceField::EnterpriseText32 => "ENTERPRISE_TEXT32",
            ResourceField::EnterpriseText33 => "ENTERPRISE_TEXT33",
            ResourceField::EnterpriseText34 => "ENTERPRISE_TEXT34",
            ResourceField::EnterpriseText35 => "ENTERPRISE_TEXT35",
            ResourceField::EnterpriseText36 => "ENTERPRISE_TEXT36",
            ResourceField::EnterpriseText37 => "ENTERPRISE_TEXT37",
            ResourceField::EnterpriseText38 => "ENTERPRISE_TEXT38",
            ResourceField::EnterpriseText39 => "ENTERPRISE_TEXT39",
            ResourceField::EnterpriseText40 => "ENTERPRISE_TEXT40",
            ResourceField::Enterprise => "ENTERPRISE",
            ResourceField::Import => "IMPORT",
            ResourceField::Created => "CREATED",
            ResourceField::Budget => "BUDGET",
            ResourceField::TaskOutlineNumber => "TASK_OUTLINE_NUMBER",
            ResourceField::Guid => "GUID",
            ResourceField::Active => "ACTIVE",
            ResourceField::AssignmentOwner => "ASSIGNMENT_OWNER",
            ResourceField::BaselineBudgetCost => "BASELINE_BUDGET_COST",
            ResourceField::BaselineBudgetWork => "BASELINE_BUDGET_WORK",
            ResourceField::Baseline1BudgetCost => "BASELINE1_BUDGET_COST",
            ResourceField::Baseline1BudgetWork => "BASELINE1_BUDGET_WORK",
            ResourceField::Baseline1Finish => "BASELINE1_FINISH",
            ResourceField::Baseline1Start => "BASELINE1_START",
            ResourceField::Baseline2BudgetCost => "BASELINE2_BUDGET_COST",
            ResourceField::Baseline2BudgetWork => "BASELINE2_BUDGET_WORK",
            ResourceField::Baseline2Finish => "BASELINE2_FINISH",
            ResourceField::Baseline2Start => "BASELINE2_START",
            ResourceField::Baseline3BudgetCost => "BASELINE3_BUDGET_COST",
            ResourceField::Baseline3BudgetWork => "BASELINE3_BUDGET_WORK",
            ResourceField::Baseline3Finish => "BASELINE3_FINISH",
            ResourceField::Baseline3Start => "BASELINE3_START",
            ResourceField::Baseline4BudgetCost => "BASELINE4_BUDGET_COST",
            ResourceField::Baseline4BudgetWork => "BASELINE4_BUDGET_WORK",
            ResourceField::Baseline4Finish => "BASELINE4_FINISH",
            ResourceField::Baseline4Start => "BASELINE4_START",
            ResourceField::Baseline5BudgetCost => "BASELINE5_BUDGET_COST",
            ResourceField::Baseline5BudgetWork => "BASELINE5_BUDGET_WORK",
            ResourceField::Baseline5Finish => "BASELINE5_FINISH",
            ResourceField::Baseline5Start => "BASELINE5_START",
            ResourceField::Baseline6BudgetCost => "BASELINE6_BUDGET_COST",
            ResourceField::Baseline6BudgetWork => "BASELINE6_BUDGET_WORK",
            ResourceField::Baseline6Finish => "BASELINE6_FINISH",
            ResourceField::Baseline6Start => "BASELINE6_START",
            ResourceField::Baseline7BudgetCost => "BASELINE7_BUDGET_COST",
            ResourceField::Baseline7BudgetWork => "BASELINE7_BUDGET_WORK",
            ResourceField::Baseline7Finish => "BASELINE7_FINISH",
            ResourceField::Baseline7Start => "BASELINE7_START",
            ResourceField::Baseline8BudgetCost => "BASELINE8_BUDGET_COST",
            ResourceField::Baseline8BudgetWork => "BASELINE8_BUDGET_WORK",
            ResourceField::Baseline8Finish => "BASELINE8_FINISH",
            ResourceField::Baseline8Start => "BASELINE8_START",
            ResourceField::Baseline9BudgetCost => "BASELINE9_BUDGET_COST",
            ResourceField::Baseline9BudgetWork => "BASELINE9_BUDGET_WORK",
            ResourceField::Baseline9Finish => "BASELINE9_FINISH",
            ResourceField::Baseline9Start => "BASELINE9_START",
            ResourceField::Baseline10BudgetCost => "BASELINE10_BUDGET_COST",
            ResourceField::Baseline10BudgetWork => "BASELINE10_BUDGET_WORK",
            ResourceField::Baseline10Finish => "BASELINE10_FINISH",
            ResourceField::Baseline10Start => "BASELINE10_START",
            ResourceField::BookingType => "BOOKING_TYPE",
            ResourceField::BudgetCost => "BUDGET_COST",
            ResourceField::BudgetWork => "BUDGET_WORK",
            ResourceField::CalendarGuid => "CALENDAR_GUID",
            ResourceField::CostCenter => "COST_CENTER",
            ResourceField::DefaultAssignmentOwner => "DEFAULT_ASSIGNMENT_OWNER",
            ResourceField::EnterpriseBaseCalendar => "ENTERPRISE_BASE_CALENDAR",
            ResourceField::EnterpriseRequiredValues => "ENTERPRISE_REQUIRED_VALUES",
            ResourceField::EnterpriseTeamMember => "ENTERPRISE_TEAM_MEMBER",
            ResourceField::EnterpriseUniqueId => "ENTERPRISE_UNIQUE_ID",
            ResourceField::ErrorMessage => "ERROR_MESSAGE",
            ResourceField::Generic => "GENERIC",
            ResourceField::Inactive => "INACTIVE",
            ResourceField::RequestDemand => "REQUEST_DEMAND",
            ResourceField::Summary => "SUMMARY",
            ResourceField::TeamAssignmentPool => "TEAM_ASSIGNMENT_POOL",
            ResourceField::Wbs => "WBS",
            ResourceField::Unavailable => "UNAVAILABLE",
            ResourceField::SubprojectResourceUniqueId => "SUBPROJECT_RESOURCE_UNIQUE_ID",
            ResourceField::HyperlinkData => "HYPERLINK_DATA",
            ResourceField::OutlineCode1Index => "OUTLINE_CODE1_INDEX",
            ResourceField::OutlineCode2Index => "OUTLINE_CODE2_INDEX",
            ResourceField::OutlineCode3Index => "OUTLINE_CODE3_INDEX",
            ResourceField::OutlineCode4Index => "OUTLINE_CODE4_INDEX",
            ResourceField::OutlineCode5Index => "OUTLINE_CODE5_INDEX",
            ResourceField::OutlineCode6Index => "OUTLINE_CODE6_INDEX",
            ResourceField::OutlineCode7Index => "OUTLINE_CODE7_INDEX",
            ResourceField::OutlineCode8Index => "OUTLINE_CODE8_INDEX",
            ResourceField::OutlineCode9Index => "OUTLINE_CODE9_INDEX",
            ResourceField::OutlineCode10Index => "OUTLINE_CODE10_INDEX",
            ResourceField::CostRateA => "COST_RATE_A",
            ResourceField::CostRateB => "COST_RATE_B",
            ResourceField::CostRateC => "COST_RATE_C",
            ResourceField::CostRateD => "COST_RATE_D",
            ResourceField::CostRateE => "COST_RATE_E",
            ResourceField::AvailabilityData => "AVAILABILITY_DATA",
            ResourceField::EnterpriseData => "ENTERPRISE_DATA",
            ResourceField::OvertimeRateUnits => "OVERTIME_RATE_UNITS",
            ResourceField::StandardRateUnits => "STANDARD_RATE_UNITS",
            ResourceField::Index => "INDEX",
            ResourceField::HyperlinkScreenTip => "HYPERLINK_SCREEN_TIP",
            ResourceField::EnterpriseOutlineCode1 => "ENTERPRISE_OUTLINE_CODE1",
            ResourceField::EnterpriseOutlineCode2 => "ENTERPRISE_OUTLINE_CODE2",
            ResourceField::EnterpriseOutlineCode3 => "ENTERPRISE_OUTLINE_CODE3",
            ResourceField::EnterpriseOutlineCode4 => "ENTERPRISE_OUTLINE_CODE4",
            ResourceField::EnterpriseOutlineCode5 => "ENTERPRISE_OUTLINE_CODE5",
            ResourceField::EnterpriseOutlineCode6 => "ENTERPRISE_OUTLINE_CODE6",
            ResourceField::EnterpriseOutlineCode7 => "ENTERPRISE_OUTLINE_CODE7",
            ResourceField::EnterpriseOutlineCode8 => "ENTERPRISE_OUTLINE_CODE8",
            ResourceField::EnterpriseOutlineCode9 => "ENTERPRISE_OUTLINE_CODE9",
            ResourceField::EnterpriseOutlineCode10 => "ENTERPRISE_OUTLINE_CODE10",
            ResourceField::EnterpriseOutlineCode11 => "ENTERPRISE_OUTLINE_CODE11",
            ResourceField::EnterpriseOutlineCode12 => "ENTERPRISE_OUTLINE_CODE12",
            ResourceField::EnterpriseOutlineCode13 => "ENTERPRISE_OUTLINE_CODE13",
            ResourceField::EnterpriseOutlineCode14 => "ENTERPRISE_OUTLINE_CODE14",
            ResourceField::EnterpriseOutlineCode15 => "ENTERPRISE_OUTLINE_CODE15",
            ResourceField::EnterpriseOutlineCode16 => "ENTERPRISE_OUTLINE_CODE16",
            ResourceField::EnterpriseOutlineCode17 => "ENTERPRISE_OUTLINE_CODE17",
            ResourceField::EnterpriseOutlineCode18 => "ENTERPRISE_OUTLINE_CODE18",
            ResourceField::EnterpriseOutlineCode19 => "ENTERPRISE_OUTLINE_CODE19",
            ResourceField::EnterpriseOutlineCode20 => "ENTERPRISE_OUTLINE_CODE20",
            ResourceField::EnterpriseOutlineCode21 => "ENTERPRISE_OUTLINE_CODE21",
            ResourceField::EnterpriseOutlineCode22 => "ENTERPRISE_OUTLINE_CODE22",
            ResourceField::EnterpriseOutlineCode23 => "ENTERPRISE_OUTLINE_CODE23",
            ResourceField::EnterpriseOutlineCode24 => "ENTERPRISE_OUTLINE_CODE24",
            ResourceField::EnterpriseOutlineCode25 => "ENTERPRISE_OUTLINE_CODE25",
            ResourceField::EnterpriseOutlineCode26 => "ENTERPRISE_OUTLINE_CODE26",
            ResourceField::EnterpriseOutlineCode27 => "ENTERPRISE_OUTLINE_CODE27",
            ResourceField::EnterpriseOutlineCode28 => "ENTERPRISE_OUTLINE_CODE28",
            ResourceField::EnterpriseOutlineCode29 => "ENTERPRISE_OUTLINE_CODE29",
            ResourceField::EnterpriseRbs => "ENTERPRISE_RBS",
            ResourceField::EnterpriseNameUsed => "ENTERPRISE_NAME_USED",
            ResourceField::EnterpriseIsCheckedOut => "ENTERPRISE_IS_CHECKED_OUT",
            ResourceField::EnterpriseCheckedOutBy => "ENTERPRISE_CHECKED_OUT_BY",
            ResourceField::EnterpriseLastModifiedDate => "ENTERPRISE_LAST_MODIFIED_DATE",
            ResourceField::EnterpriseMultiValue20 => "ENTERPRISE_MULTI_VALUE20",
            ResourceField::EnterpriseMultiValue21 => "ENTERPRISE_MULTI_VALUE21",
            ResourceField::EnterpriseMultiValue22 => "ENTERPRISE_MULTI_VALUE22",
            ResourceField::EnterpriseMultiValue23 => "ENTERPRISE_MULTI_VALUE23",
            ResourceField::EnterpriseMultiValue24 => "ENTERPRISE_MULTI_VALUE24",
            ResourceField::EnterpriseMultiValue25 => "ENTERPRISE_MULTI_VALUE25",
            ResourceField::EnterpriseMultiValue26 => "ENTERPRISE_MULTI_VALUE26",
            ResourceField::EnterpriseMultiValue27 => "ENTERPRISE_MULTI_VALUE27",
            ResourceField::EnterpriseMultiValue28 => "ENTERPRISE_MULTI_VALUE28",
            ResourceField::EnterpriseMultiValue29 => "ENTERPRISE_MULTI_VALUE29",
            ResourceField::ActualWorkProtected => "ACTUAL_WORK_PROTECTED",
            ResourceField::ActualOvertimeWorkProtected => "ACTUAL_OVERTIME_WORK_PROTECTED",
            ResourceField::ParentId => "PARENT_ID",
            ResourceField::CalendarUniqueId => "CALENDAR_UNIQUE_ID",
            ResourceField::ProposedStart => "PROPOSED_START",
            ResourceField::ProposedFinish => "PROPOSED_FINISH",
            ResourceField::ProposedMaxUnits => "PROPOSED_MAX_UNITS",
            ResourceField::EngagementStatus => "ENGAGEMENT_STATUS",
            ResourceField::Unit => "UNIT",
            ResourceField::SupplyReference => "SUPPLY_REFERENCE",
            ResourceField::Description => "DESCRIPTION",
            ResourceField::ResourceId => "RESOURCE_ID",
            ResourceField::ModifyOnIntegrate => "MODIFY_ON_INTEGRATE",
            ResourceField::ExpensesOnly => "EXPENSES_ONLY",
            ResourceField::PeriodDur => "PERIOD_DUR",
            ResourceField::Priority => "PRIORITY",
            ResourceField::Rate => "RATE",
            ResourceField::Pool => "POOL",
            ResourceField::PerDay => "PER_DAY",
            ResourceField::Phone => "PHONE",
            ResourceField::Role => "ROLE",
            ResourceField::CalculateCostsFromUnits => "CALCULATE_COSTS_FROM_UNITS",
            ResourceField::SequenceNumber => "SEQUENCE_NUMBER",
            ResourceField::LocationUniqueId => "LOCATION_UNIQUE_ID",
            ResourceField::UnitOfMeasureUniqueId => "UNIT_OF_MEASURE_UNIQUE_ID",
            ResourceField::DefaultUnits => "DEFAULT_UNITS",
            ResourceField::ShiftUniqueId => "SHIFT_UNIQUE_ID",
            ResourceField::PrimaryRoleUniqueId => "PRIMARY_ROLE_UNIQUE_ID",
            ResourceField::Finish => "FINISH",
            ResourceField::ResourceCodeValues => "RESOURCE_CODE_VALUES",
            ResourceField::RoleCodeValues => "ROLE_CODE_VALUES",
            ResourceField::CurrencyUniqueId => "CURRENCY_UNIQUE_ID",
            ResourceField::PlannedWork => "PLANNED_WORK",
            ResourceField::ActualRegularWork => "ACTUAL_REGULAR_WORK",
            ResourceField::RemainingRegularWork => "REMAINING_REGULAR_WORK",
            ResourceField::ActualRegularCost => "ACTUAL_REGULAR_COST",
            ResourceField::RemainingRegularCost => "REMAINING_REGULAR_COST",
            ResourceField::PlannedMaterial => "PLANNED_MATERIAL",
            ResourceField::ActualMaterial => "ACTUAL_MATERIAL",
            ResourceField::RemainingMaterial => "REMAINING_MATERIAL",
            ResourceField::Material => "MATERIAL",
            ResourceField::BaselineMaterial => "BASELINE_MATERIAL",
            ResourceField::Baseline1Material => "BASELINE1_MATERIAL",
            ResourceField::Baseline2Material => "BASELINE2_MATERIAL",
            ResourceField::Baseline3Material => "BASELINE3_MATERIAL",
            ResourceField::Baseline4Material => "BASELINE4_MATERIAL",
            ResourceField::Baseline5Material => "BASELINE5_MATERIAL",
            ResourceField::Baseline6Material => "BASELINE6_MATERIAL",
            ResourceField::Baseline7Material => "BASELINE7_MATERIAL",
            ResourceField::Baseline8Material => "BASELINE8_MATERIAL",
            ResourceField::Baseline9Material => "BASELINE9_MATERIAL",
            ResourceField::Baseline10Material => "BASELINE10_MATERIAL",
            ResourceField::PlannedCost => "PLANNED_COST",
        }
    }

    /// Corresponds to Java: `public String getName()`
    fn get_name(&self) -> &'static str {
        RESOURCE_COLUMN_NAMES[*self as usize]
    }

    /// Corresponds to Java: `public DataType getDataType()`
    fn get_data_type(&self) -> DataType {
        match self {
            ResourceField::Start
            | ResourceField::AvailableFrom
            | ResourceField::AvailableTo
            | ResourceField::Start1
            | ResourceField::Start2
            | ResourceField::Start3
            | ResourceField::Start4
            | ResourceField::Start5
            | ResourceField::Finish1
            | ResourceField::Finish2
            | ResourceField::Finish3
            | ResourceField::Finish4
            | ResourceField::Finish5
            | ResourceField::Date1
            | ResourceField::Date2
            | ResourceField::Date3
            | ResourceField::Date4
            | ResourceField::Date5
            | ResourceField::Date6
            | ResourceField::Date7
            | ResourceField::Date8
            | ResourceField::Date9
            | ResourceField::Date10
            | ResourceField::Finish6
            | ResourceField::Finish7
            | ResourceField::Finish8
            | ResourceField::Finish9
            | ResourceField::Finish10
            | ResourceField::Start6
            | ResourceField::Start7
            | ResourceField::Start8
            | ResourceField::Start9
            | ResourceField::Start10
            | ResourceField::BaselineStart
            | ResourceField::BaselineFinish
            | ResourceField::ActualStart
            | ResourceField::ActualFinish
            | ResourceField::Created
            | ResourceField::Baseline1Finish
            | ResourceField::Baseline1Start
            | ResourceField::Baseline2Finish
            | ResourceField::Baseline2Start
            | ResourceField::Baseline3Finish
            | ResourceField::Baseline3Start
            | ResourceField::Baseline4Finish
            | ResourceField::Baseline4Start
            | ResourceField::Baseline5Finish
            | ResourceField::Baseline5Start
            | ResourceField::Baseline6Finish
            | ResourceField::Baseline6Start
            | ResourceField::Baseline7Finish
            | ResourceField::Baseline7Start
            | ResourceField::Baseline8Finish
            | ResourceField::Baseline8Start
            | ResourceField::Baseline9Finish
            | ResourceField::Baseline9Start
            | ResourceField::Baseline10Finish
            | ResourceField::Baseline10Start
            | ResourceField::EnterpriseDate1
            | ResourceField::EnterpriseDate2
            | ResourceField::EnterpriseDate3
            | ResourceField::EnterpriseDate4
            | ResourceField::EnterpriseDate5
            | ResourceField::EnterpriseDate6
            | ResourceField::EnterpriseDate7
            | ResourceField::EnterpriseDate8
            | ResourceField::EnterpriseDate9
            | ResourceField::EnterpriseDate10
            | ResourceField::EnterpriseDate11
            | ResourceField::EnterpriseDate12
            | ResourceField::EnterpriseDate13
            | ResourceField::EnterpriseDate14
            | ResourceField::EnterpriseDate15
            | ResourceField::EnterpriseDate16
            | ResourceField::EnterpriseDate17
            | ResourceField::EnterpriseDate18
            | ResourceField::EnterpriseDate19
            | ResourceField::EnterpriseDate20
            | ResourceField::EnterpriseDate21
            | ResourceField::EnterpriseDate22
            | ResourceField::EnterpriseDate23
            | ResourceField::EnterpriseDate24
            | ResourceField::EnterpriseDate25
            | ResourceField::EnterpriseDate26
            | ResourceField::EnterpriseDate27
            | ResourceField::EnterpriseDate28
            | ResourceField::EnterpriseDate29
            | ResourceField::EnterpriseDate30
            | ResourceField::EnterpriseLastModifiedDate
            | ResourceField::ProposedStart
            | ResourceField::ProposedFinish
            | ResourceField::Finish => DataType::Date,

            ResourceField::Duration1Units
            | ResourceField::Duration2Units
            | ResourceField::Duration3Units
            | ResourceField::Duration4Units
            | ResourceField::Duration5Units
            | ResourceField::Duration6Units
            | ResourceField::Duration7Units
            | ResourceField::Duration8Units
            | ResourceField::Duration9Units
            | ResourceField::Duration10Units
            | ResourceField::EnterpriseDuration1Units
            | ResourceField::EnterpriseDuration2Units
            | ResourceField::EnterpriseDuration3Units
            | ResourceField::EnterpriseDuration4Units
            | ResourceField::EnterpriseDuration5Units
            | ResourceField::EnterpriseDuration6Units
            | ResourceField::EnterpriseDuration7Units
            | ResourceField::EnterpriseDuration8Units
            | ResourceField::EnterpriseDuration9Units
            | ResourceField::EnterpriseDuration10Units => DataType::TimeUnits,

            ResourceField::Id
            | ResourceField::UniqueId
            | ResourceField::EnterpriseUniqueId
            | ResourceField::SubprojectResourceUniqueId
            | ResourceField::OutlineCode1Index
            | ResourceField::OutlineCode2Index
            | ResourceField::OutlineCode3Index
            | ResourceField::OutlineCode4Index
            | ResourceField::OutlineCode5Index
            | ResourceField::OutlineCode6Index
            | ResourceField::OutlineCode7Index
            | ResourceField::OutlineCode8Index
            | ResourceField::OutlineCode9Index
            | ResourceField::OutlineCode10Index
            | ResourceField::Index
            | ResourceField::ParentId
            | ResourceField::CalendarUniqueId
            | ResourceField::SequenceNumber
            | ResourceField::LocationUniqueId
            | ResourceField::UnitOfMeasureUniqueId
            | ResourceField::ShiftUniqueId
            | ResourceField::PrimaryRoleUniqueId
            | ResourceField::CurrencyUniqueId => DataType::Integer,

            ResourceField::Name
            | ResourceField::Initials
            | ResourceField::Group
            | ResourceField::BaseCalendar
            | ResourceField::Text1
            | ResourceField::Text2
            | ResourceField::Code
            | ResourceField::Text3
            | ResourceField::Text4
            | ResourceField::Text5
            | ResourceField::EmailAddress
            | ResourceField::Indicators
            | ResourceField::Text6
            | ResourceField::Text7
            | ResourceField::Text8
            | ResourceField::Text9
            | ResourceField::Text10
            | ResourceField::Hyperlink
            | ResourceField::HyperlinkAddress
            | ResourceField::HyperlinkSubaddress
            | ResourceField::HyperlinkHref
            | ResourceField::TaskSummaryName
            | ResourceField::Text11
            | ResourceField::Text12
            | ResourceField::Text13
            | ResourceField::Text14
            | ResourceField::Text15
            | ResourceField::Text16
            | ResourceField::Text17
            | ResourceField::Text18
            | ResourceField::Text19
            | ResourceField::Text20
            | ResourceField::Text21
            | ResourceField::Text22
            | ResourceField::Text23
            | ResourceField::Text24
            | ResourceField::Text25
            | ResourceField::Text26
            | ResourceField::Text27
            | ResourceField::Text28
            | ResourceField::Text29
            | ResourceField::Text30
            | ResourceField::Phonetics
            | ResourceField::Project
            | ResourceField::OutlineCode1
            | ResourceField::OutlineCode2
            | ResourceField::OutlineCode3
            | ResourceField::OutlineCode4
            | ResourceField::OutlineCode5
            | ResourceField::OutlineCode6
            | ResourceField::OutlineCode7
            | ResourceField::OutlineCode8
            | ResourceField::OutlineCode9
            | ResourceField::OutlineCode10
            | ResourceField::MaterialLabel
            | ResourceField::WindowsUserAccount
            | ResourceField::EnterpriseText1
            | ResourceField::EnterpriseText2
            | ResourceField::EnterpriseText3
            | ResourceField::EnterpriseText4
            | ResourceField::EnterpriseText5
            | ResourceField::EnterpriseText6
            | ResourceField::EnterpriseText7
            | ResourceField::EnterpriseText8
            | ResourceField::EnterpriseText9
            | ResourceField::EnterpriseText10
            | ResourceField::EnterpriseText11
            | ResourceField::EnterpriseText12
            | ResourceField::EnterpriseText13
            | ResourceField::EnterpriseText14
            | ResourceField::EnterpriseText15
            | ResourceField::EnterpriseText16
            | ResourceField::EnterpriseText17
            | ResourceField::EnterpriseText18
            | ResourceField::EnterpriseText19
            | ResourceField::EnterpriseText20
            | ResourceField::EnterpriseText21
            | ResourceField::EnterpriseText22
            | ResourceField::EnterpriseText23
            | ResourceField::EnterpriseText24
            | ResourceField::EnterpriseText25
            | ResourceField::EnterpriseText26
            | ResourceField::EnterpriseText27
            | ResourceField::EnterpriseText28
            | ResourceField::EnterpriseText29
            | ResourceField::EnterpriseText30
            | ResourceField::EnterpriseText31
            | ResourceField::EnterpriseText32
            | ResourceField::EnterpriseText33
            | ResourceField::EnterpriseText34
            | ResourceField::EnterpriseText35
            | ResourceField::EnterpriseText36
            | ResourceField::EnterpriseText37
            | ResourceField::EnterpriseText38
            | ResourceField::EnterpriseText39
            | ResourceField::EnterpriseText40
            | ResourceField::TaskOutlineNumber
            | ResourceField::AssignmentOwner
            | ResourceField::CostCenter
            | ResourceField::DefaultAssignmentOwner
            | ResourceField::ErrorMessage
            | ResourceField::RequestDemand
            | ResourceField::Summary
            | ResourceField::Wbs
            | ResourceField::Unavailable
            | ResourceField::HyperlinkScreenTip
            | ResourceField::EnterpriseOutlineCode1
            | ResourceField::EnterpriseOutlineCode2
            | ResourceField::EnterpriseOutlineCode3
            | ResourceField::EnterpriseOutlineCode4
            | ResourceField::EnterpriseOutlineCode5
            | ResourceField::EnterpriseOutlineCode6
            | ResourceField::EnterpriseOutlineCode7
            | ResourceField::EnterpriseOutlineCode8
            | ResourceField::EnterpriseOutlineCode9
            | ResourceField::EnterpriseOutlineCode10
            | ResourceField::EnterpriseOutlineCode11
            | ResourceField::EnterpriseOutlineCode12
            | ResourceField::EnterpriseOutlineCode13
            | ResourceField::EnterpriseOutlineCode14
            | ResourceField::EnterpriseOutlineCode15
            | ResourceField::EnterpriseOutlineCode16
            | ResourceField::EnterpriseOutlineCode17
            | ResourceField::EnterpriseOutlineCode18
            | ResourceField::EnterpriseOutlineCode19
            | ResourceField::EnterpriseOutlineCode20
            | ResourceField::EnterpriseOutlineCode21
            | ResourceField::EnterpriseOutlineCode22
            | ResourceField::EnterpriseOutlineCode23
            | ResourceField::EnterpriseOutlineCode24
            | ResourceField::EnterpriseOutlineCode25
            | ResourceField::EnterpriseOutlineCode26
            | ResourceField::EnterpriseOutlineCode27
            | ResourceField::EnterpriseOutlineCode28
            | ResourceField::EnterpriseOutlineCode29
            | ResourceField::EnterpriseRbs
            | ResourceField::EnterpriseNameUsed
            | ResourceField::EnterpriseCheckedOutBy
            | ResourceField::EnterpriseMultiValue20
            | ResourceField::EnterpriseMultiValue21
            | ResourceField::EnterpriseMultiValue22
            | ResourceField::EnterpriseMultiValue23
            | ResourceField::EnterpriseMultiValue24
            | ResourceField::EnterpriseMultiValue25
            | ResourceField::EnterpriseMultiValue26
            | ResourceField::EnterpriseMultiValue27
            | ResourceField::EnterpriseMultiValue28
            | ResourceField::EnterpriseMultiValue29
            | ResourceField::EngagementStatus
            | ResourceField::Unit
            | ResourceField::SupplyReference
            | ResourceField::Description
            | ResourceField::ResourceId
            | ResourceField::Phone => DataType::String,

            ResourceField::MaxUnits
            | ResourceField::Peak
            | ResourceField::AssignmentUnits
            | ResourceField::ProposedMaxUnits
            | ResourceField::DefaultUnits => DataType::Units,

            ResourceField::StandardRate | ResourceField::OvertimeRate => DataType::Rate,

            ResourceField::ActualCost
            | ResourceField::Cost
            | ResourceField::BaselineCost
            | ResourceField::CostPerUse
            | ResourceField::RemainingCost
            | ResourceField::CostVariance
            | ResourceField::OvertimeCost
            | ResourceField::ActualOvertimeCost
            | ResourceField::RemainingOvertimeCost
            | ResourceField::Bcws
            | ResourceField::Bcwp
            | ResourceField::Acwp
            | ResourceField::Sv
            | ResourceField::Cost1
            | ResourceField::Cost2
            | ResourceField::Cost3
            | ResourceField::Cost4
            | ResourceField::Cost5
            | ResourceField::Cost6
            | ResourceField::Cost7
            | ResourceField::Cost8
            | ResourceField::Cost9
            | ResourceField::Cost10
            | ResourceField::Baseline1Cost
            | ResourceField::Baseline2Cost
            | ResourceField::Baseline3Cost
            | ResourceField::Baseline4Cost
            | ResourceField::Baseline5Cost
            | ResourceField::Baseline6Cost
            | ResourceField::Baseline7Cost
            | ResourceField::Baseline8Cost
            | ResourceField::Baseline9Cost
            | ResourceField::Baseline10Cost
            | ResourceField::EnterpriseCost1
            | ResourceField::EnterpriseCost2
            | ResourceField::EnterpriseCost3
            | ResourceField::EnterpriseCost4
            | ResourceField::EnterpriseCost5
            | ResourceField::EnterpriseCost6
            | ResourceField::EnterpriseCost7
            | ResourceField::EnterpriseCost8
            | ResourceField::EnterpriseCost9
            | ResourceField::EnterpriseCost10
            | ResourceField::Cv
            | ResourceField::Vac
            | ResourceField::BaselineBudgetCost
            | ResourceField::Baseline1BudgetCost
            | ResourceField::Baseline2BudgetCost
            | ResourceField::Baseline3BudgetCost
            | ResourceField::Baseline4BudgetCost
            | ResourceField::Baseline5BudgetCost
            | ResourceField::Baseline6BudgetCost
            | ResourceField::Baseline7BudgetCost
            | ResourceField::Baseline8BudgetCost
            | ResourceField::Baseline9BudgetCost
            | ResourceField::Baseline10BudgetCost
            | ResourceField::BudgetCost
            | ResourceField::ActualRegularCost
            | ResourceField::RemainingRegularCost
            | ResourceField::PlannedCost => DataType::Currency,

            ResourceField::Work
            | ResourceField::ActualWork
            | ResourceField::BaselineWork
            | ResourceField::OvertimeWork
            | ResourceField::RemainingWork
            | ResourceField::RegularWork
            | ResourceField::ActualOvertimeWork
            | ResourceField::RemainingOvertimeWork
            | ResourceField::Baseline1Work
            | ResourceField::Baseline2Work
            | ResourceField::Baseline3Work
            | ResourceField::Baseline4Work
            | ResourceField::Baseline5Work
            | ResourceField::Baseline6Work
            | ResourceField::Baseline7Work
            | ResourceField::Baseline8Work
            | ResourceField::Baseline9Work
            | ResourceField::Baseline10Work
            | ResourceField::BaselineBudgetWork
            | ResourceField::Baseline1BudgetWork
            | ResourceField::Baseline2BudgetWork
            | ResourceField::Baseline3BudgetWork
            | ResourceField::Baseline4BudgetWork
            | ResourceField::Baseline5BudgetWork
            | ResourceField::Baseline6BudgetWork
            | ResourceField::Baseline7BudgetWork
            | ResourceField::Baseline8BudgetWork
            | ResourceField::Baseline9BudgetWork
            | ResourceField::Baseline10BudgetWork
            | ResourceField::BudgetWork
            | ResourceField::ActualWorkProtected
            | ResourceField::ActualOvertimeWorkProtected
            | ResourceField::PlannedWork
            | ResourceField::ActualRegularWork
            | ResourceField::RemainingRegularWork => DataType::Work,

            ResourceField::WorkVariance
            | ResourceField::LevelingDelay
            | ResourceField::Duration1
            | ResourceField::Duration2
            | ResourceField::Duration3
            | ResourceField::Duration4
            | ResourceField::Duration5
            | ResourceField::Duration6
            | ResourceField::Duration7
            | ResourceField::Duration8
            | ResourceField::Duration9
            | ResourceField::Duration10
            | ResourceField::EnterpriseDuration1
            | ResourceField::EnterpriseDuration2
            | ResourceField::EnterpriseDuration3
            | ResourceField::EnterpriseDuration4
            | ResourceField::EnterpriseDuration5
            | ResourceField::EnterpriseDuration6
            | ResourceField::EnterpriseDuration7
            | ResourceField::EnterpriseDuration8
            | ResourceField::EnterpriseDuration9
            | ResourceField::EnterpriseDuration10 => DataType::Duration,

            ResourceField::AccrueAt => DataType::Accrue,

            ResourceField::Overallocated
            | ResourceField::LinkedFields
            | ResourceField::Flag10
            | ResourceField::Flag1
            | ResourceField::Flag2
            | ResourceField::Flag3
            | ResourceField::Flag4
            | ResourceField::Flag5
            | ResourceField::Flag6
            | ResourceField::Flag7
            | ResourceField::Flag8
            | ResourceField::Flag9
            | ResourceField::Assignment
            | ResourceField::CanLevel
            | ResourceField::Flag11
            | ResourceField::Flag12
            | ResourceField::Flag13
            | ResourceField::Flag14
            | ResourceField::Flag15
            | ResourceField::Flag16
            | ResourceField::Flag17
            | ResourceField::Flag18
            | ResourceField::Flag19
            | ResourceField::Flag20
            | ResourceField::Confirmed
            | ResourceField::ResponsePending
            | ResourceField::TeamStatusPending
            | ResourceField::UpdateNeeded
            | ResourceField::GroupBySummary
            | ResourceField::Enterprise
            | ResourceField::Import
            | ResourceField::Budget
            | ResourceField::Active
            | ResourceField::EnterpriseFlag1
            | ResourceField::EnterpriseFlag2
            | ResourceField::EnterpriseFlag3
            | ResourceField::EnterpriseFlag4
            | ResourceField::EnterpriseFlag5
            | ResourceField::EnterpriseFlag6
            | ResourceField::EnterpriseFlag7
            | ResourceField::EnterpriseFlag8
            | ResourceField::EnterpriseFlag9
            | ResourceField::EnterpriseFlag10
            | ResourceField::EnterpriseFlag11
            | ResourceField::EnterpriseFlag12
            | ResourceField::EnterpriseFlag13
            | ResourceField::EnterpriseFlag14
            | ResourceField::EnterpriseFlag15
            | ResourceField::EnterpriseFlag16
            | ResourceField::EnterpriseFlag17
            | ResourceField::EnterpriseFlag18
            | ResourceField::EnterpriseFlag19
            | ResourceField::EnterpriseFlag20
            | ResourceField::EnterpriseBaseCalendar
            | ResourceField::EnterpriseRequiredValues
            | ResourceField::EnterpriseTeamMember
            | ResourceField::Generic
            | ResourceField::Inactive
            | ResourceField::TeamAssignmentPool
            | ResourceField::EnterpriseIsCheckedOut
            | ResourceField::ModifyOnIntegrate
            | ResourceField::ExpensesOnly
            | ResourceField::Role
            | ResourceField::CalculateCostsFromUnits => DataType::Boolean,

            ResourceField::Notes => DataType::Notes,

            ResourceField::PercentWorkComplete => DataType::Percentage,

            ResourceField::Objects
            | ResourceField::Number1
            | ResourceField::Number2
            | ResourceField::Number3
            | ResourceField::Number4
            | ResourceField::Number5
            | ResourceField::Number6
            | ResourceField::Number7
            | ResourceField::Number8
            | ResourceField::Number9
            | ResourceField::Number10
            | ResourceField::Number11
            | ResourceField::Number12
            | ResourceField::Number13
            | ResourceField::Number14
            | ResourceField::Number15
            | ResourceField::Number16
            | ResourceField::Number17
            | ResourceField::Number18
            | ResourceField::Number19
            | ResourceField::Number20
            | ResourceField::EnterpriseNumber1
            | ResourceField::EnterpriseNumber2
            | ResourceField::EnterpriseNumber3
            | ResourceField::EnterpriseNumber4
            | ResourceField::EnterpriseNumber5
            | ResourceField::EnterpriseNumber6
            | ResourceField::EnterpriseNumber7
            | ResourceField::EnterpriseNumber8
            | ResourceField::EnterpriseNumber9
            | ResourceField::EnterpriseNumber10
            | ResourceField::EnterpriseNumber11
            | ResourceField::EnterpriseNumber12
            | ResourceField::EnterpriseNumber13
            | ResourceField::EnterpriseNumber14
            | ResourceField::EnterpriseNumber15
            | ResourceField::EnterpriseNumber16
            | ResourceField::EnterpriseNumber17
            | ResourceField::EnterpriseNumber18
            | ResourceField::EnterpriseNumber19
            | ResourceField::EnterpriseNumber20
            | ResourceField::EnterpriseNumber21
            | ResourceField::EnterpriseNumber22
            | ResourceField::EnterpriseNumber23
            | ResourceField::EnterpriseNumber24
            | ResourceField::EnterpriseNumber25
            | ResourceField::EnterpriseNumber26
            | ResourceField::EnterpriseNumber27
            | ResourceField::EnterpriseNumber28
            | ResourceField::EnterpriseNumber29
            | ResourceField::EnterpriseNumber30
            | ResourceField::EnterpriseNumber31
            | ResourceField::EnterpriseNumber32
            | ResourceField::EnterpriseNumber33
            | ResourceField::EnterpriseNumber34
            | ResourceField::EnterpriseNumber35
            | ResourceField::EnterpriseNumber36
            | ResourceField::EnterpriseNumber37
            | ResourceField::EnterpriseNumber38
            | ResourceField::EnterpriseNumber39
            | ResourceField::EnterpriseNumber40
            | ResourceField::PeriodDur
            | ResourceField::Priority
            | ResourceField::Rate
            | ResourceField::Pool
            | ResourceField::PerDay
            | ResourceField::PlannedMaterial
            | ResourceField::ActualMaterial
            | ResourceField::RemainingMaterial
            | ResourceField::Material
            | ResourceField::BaselineMaterial
            | ResourceField::Baseline1Material
            | ResourceField::Baseline2Material
            | ResourceField::Baseline3Material
            | ResourceField::Baseline4Material
            | ResourceField::Baseline5Material
            | ResourceField::Baseline6Material
            | ResourceField::Baseline7Material
            | ResourceField::Baseline8Material
            | ResourceField::Baseline9Material
            | ResourceField::Baseline10Material => DataType::Numeric,

            ResourceField::CostRateTable => DataType::Short,

            ResourceField::Workgroup => DataType::Workgroup,

            ResourceField::Type => DataType::ResourceType,

            ResourceField::AssignmentDelay => DataType::Delay,

            ResourceField::WorkContour => DataType::WorkContour,

            ResourceField::Guid | ResourceField::CalendarGuid => DataType::Guid,

            ResourceField::BookingType => DataType::BookingType,

            ResourceField::HyperlinkData
            | ResourceField::CostRateA
            | ResourceField::CostRateB
            | ResourceField::CostRateC
            | ResourceField::CostRateD
            | ResourceField::CostRateE
            | ResourceField::AvailabilityData
            | ResourceField::EnterpriseData => DataType::Binary,

            ResourceField::OvertimeRateUnits | ResourceField::StandardRateUnits => DataType::RateUnits,

            ResourceField::ResourceCodeValues | ResourceField::RoleCodeValues => DataType::CodeValues,
        }
    }

    /// Corresponds to Java: `public FieldType getUnitsType()` (Java returns `null`)
    fn get_units_type(&self) -> Option<Self> {
        match self {
            ResourceField::Duration1 => Some(ResourceField::Duration1Units),
            ResourceField::Duration2 => Some(ResourceField::Duration2Units),
            ResourceField::Duration3 => Some(ResourceField::Duration3Units),
            ResourceField::Duration4 => Some(ResourceField::Duration4Units),
            ResourceField::Duration5 => Some(ResourceField::Duration5Units),
            ResourceField::Duration6 => Some(ResourceField::Duration6Units),
            ResourceField::Duration7 => Some(ResourceField::Duration7Units),
            ResourceField::Duration8 => Some(ResourceField::Duration8Units),
            ResourceField::Duration9 => Some(ResourceField::Duration9Units),
            ResourceField::Duration10 => Some(ResourceField::Duration10Units),
            ResourceField::EnterpriseDuration1 => Some(ResourceField::EnterpriseDuration1Units),
            ResourceField::EnterpriseDuration2 => Some(ResourceField::EnterpriseDuration2Units),
            ResourceField::EnterpriseDuration3 => Some(ResourceField::EnterpriseDuration3Units),
            ResourceField::EnterpriseDuration4 => Some(ResourceField::EnterpriseDuration4Units),
            ResourceField::EnterpriseDuration5 => Some(ResourceField::EnterpriseDuration5Units),
            ResourceField::EnterpriseDuration6 => Some(ResourceField::EnterpriseDuration6Units),
            ResourceField::EnterpriseDuration7 => Some(ResourceField::EnterpriseDuration7Units),
            ResourceField::EnterpriseDuration8 => Some(ResourceField::EnterpriseDuration8Units),
            ResourceField::EnterpriseDuration9 => Some(ResourceField::EnterpriseDuration9Units),
            ResourceField::EnterpriseDuration10 => Some(ResourceField::EnterpriseDuration10Units),
            _ => None,
        }
    }
}

impl MpxjEnum for ResourceField {
    /// Corresponds to Java: `public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl std::fmt::Display for ResourceField {
    /// Corresponds to Java: `public String toString()`
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_field_round_trips() {
        for item in ResourceField::ALL {
            assert_eq!(ResourceField::get_instance(item.value()), Some(item));
        }
    }

    #[test]
    fn resource_field_out_of_range_returns_none() {
        assert_eq!(ResourceField::get_instance(-1), None);
        assert_eq!(ResourceField::get_instance(ResourceField::MAX_VALUE as i32), None);
    }

    #[test]
    fn resource_field_max_value_matches_variant_count() {
        assert_eq!(ResourceField::ALL.len(), ResourceField::MAX_VALUE);
    }
}
