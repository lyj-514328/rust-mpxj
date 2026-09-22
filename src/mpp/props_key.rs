//! Corresponds to Java: `org/mpxj/mpp/PropsKey.java`

use crate::mpxj_enum::MpxjEnum;

/// Keys used in `Props` blocks.
///
/// Corresponds to Java: `enum PropsKey`
///
/// Notes:
/// - The variant order exactly matches Java's declaration order, and names are converted from `UPPER_SNAKE` to `PascalCase`.
/// - On the Java side this enum has only `getValue()` (it does not declare `implements MpxjEnum`); the Rust side, per project
/// convention, implements [`MpxjEnum`] (`value()` returns the same key value) and additionally provides the equivalents of the compiler-generated
/// `values()` / `name()` / `toString()` in Java: [`PropsKey::ALL`],
///   [`PropsKey::get_name`],[`Display`].
/// - [`PropsKey::get_instance`] has no counterpart on the Java side (and Java's reflection-table `EnumHelper` approach
/// does not exist for this enum either); the Rust side provides it as "lookup by value" for upper layers to resolve the enum from a key value.
/// - Key values are duplicated (for example `TABLE_NAME` and `UPPER_VIEW_NAME` are both 574619658,
/// `KEYWORDS` and `BASELINE_DATE` are both 37748749), consistent with the Java comment "a few duplicate values";
/// `get_instance` returns the first match in declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropsKey {
    /// Corresponds to Java: `ASSIGNMENT_FIELD_MAP(131095)`
    AssignmentFieldMap,
    /// Corresponds to Java: `ASSIGNMENT_FIELD_MAP2(50331671)`
    AssignmentFieldMap2,
    /// Corresponds to Java: `AUTO_FILTER(893386767)`
    AutoFilter,
    /// Corresponds to Java: `BASELINE10_DATE(37753759)`
    Baseline10Date,
    /// Corresponds to Java: `BASELINE1_DATE(37753750)`
    Baseline1Date,
    /// Corresponds to Java: `BASELINE2_DATE(37753751)`
    Baseline2Date,
    /// Corresponds to Java: `BASELINE3_DATE(37753752)`
    Baseline3Date,
    /// Corresponds to Java: `BASELINE4_DATE(37753753)`
    Baseline4Date,
    /// Corresponds to Java: `BASELINE5_DATE(37753754)`
    Baseline5Date,
    /// Corresponds to Java: `BASELINE6_DATE(37753755)`
    Baseline6Date,
    /// Corresponds to Java: `BASELINE7_DATE(37753756)`
    Baseline7Date,
    /// Corresponds to Java: `BASELINE8_DATE(37753757)`
    Baseline8Date,
    /// Corresponds to Java: `BASELINE9_DATE(37753758)`
    Baseline9Date,
    /// Corresponds to Java: `BASELINE_CALENDAR_NAME(37753747)`
    BaselineCalendarName,
    /// Corresponds to Java: `BASELINE_DATE(37753749)`
    BaselineDate,
    /// Corresponds to Java: `CRITICAL_SLACK_LIMIT(37748756)`
    CriticalSlackLimit,
    /// Corresponds to Java: `CURRENCY_CODE(37753787)`
    CurrencyCode,
    /// Corresponds to Java: `CURRENCY_DIGITS(37748754)`
    CurrencyDigits,
    /// Corresponds to Java: `CURRENCY_PLACEMENT(37748753)`
    CurrencyPlacement,
    /// Corresponds to Java: `CURRENCY_SYMBOL(37748752)`
    CurrencySymbol,
    /// Corresponds to Java: `CUSTOM_FIELDS(71303169)`
    CustomFields,
    /// Corresponds to Java: `DAYS_PER_MONTH(37753743)`
    DaysPerMonth,
    /// Corresponds to Java: `DEFAULT_CALENDAR_HOURS(37753736)`
    DefaultCalendarHours,
    /// Corresponds to Java: `DEFAULT_CALENDAR_NAME(37748750)`
    DefaultCalendarName,
    /// Corresponds to Java: `DEFAULT_TASK_TYPE(37748785)`
    DefaultTaskType,
    /// Corresponds to Java: `DURATION_UNITS(37748757)`
    DurationUnits,
    /// Corresponds to Java: `EDITABLE_ACTUAL_COSTS(37748802)`
    EditableActualCosts,
    /// Corresponds to Java: `ENCRYPTION_CODE(893386759)`
    EncryptionCode,
    /// Corresponds to Java: `END_TIME(37748769)`
    EndTime,
    /// Corresponds to Java: `ENTERPRISE_CUSTOM_FIELD_MAP(37753797)`
    EnterpriseCustomFieldMap,
    /// Corresponds to Java: `FISCAL_YEAR_START(37748801)`
    FiscalYearStart,
    /// Corresponds to Java: `FISCAL_YEAR_START_MONTH(37748780)`
    FiscalYearStartMonth,
    /// Corresponds to Java: `FONT_BASES(54525952)`
    FontBases,
    /// Corresponds to Java: `GUID(37748777)`
    Guid,
    /// Corresponds to Java: `HONOR_CONSTRAINTS(37748794)`
    HonorConstraints,
    /// Corresponds to Java: `HYPERLINK_BASE(37748810)`
    HyperlinkBase,
    /// Corresponds to Java: `MINUTES_PER_DAY(37748765)`
    MinutesPerDay,
    /// Corresponds to Java: `MINUTES_PER_WEEK(37748766)`
    MinutesPerWeek,
    /// Corresponds to Java: `MULTIPLE_CRITICAL_PATHS(37748793)`
    MultipleCriticalPaths,
    /// Corresponds to Java: `NEW_TASKS_ARE_MANUAL(37753800)`
    NewTasksAreManual,
    /// Corresponds to Java: `OVERTIME_RATE(37748768)`
    OvertimeRate,
    /// Corresponds to Java: `PASSWORD_FLAG(893386752)`
    PasswordFlag,
    /// Corresponds to Java: `PROJECT_FILE_PATH(893386760)`
    ProjectFilePath,
    /// Corresponds to Java: `PROJECT_FINISH_DATE(37748739)`
    ProjectFinishDate,
    /// Corresponds to Java: `PROJECT_START_DATE(37748738)`
    ProjectStartDate,
    /// Corresponds to Java: `PROTECTION_PASSWORD_HASH(893386756)`
    ProtectionPasswordHash,
    /// Corresponds to Java: `RELATION_FIELD_MAP(131094)`
    RelationFieldMap,
    /// Corresponds to Java: `RESOURCE_CREATION_DATE(205521219)`
    ResourceCreationDate,
    /// Corresponds to Java: `RESOURCE_FIELD_MAP(131093)`
    ResourceFieldMap,
    /// Corresponds to Java: `RESOURCE_FIELD_MAP2(50331669)`
    ResourceFieldMap2,
    /// Corresponds to Java: `RESOURCE_FIELD_NAME_ALIASES(1048578)`
    ResourceFieldNameAliases,
    /// Corresponds to Java: `RESOURCE_POOL(37748747)`
    ResourcePool,
    /// Corresponds to Java: `SCHEDULE_FROM(37748740)`
    ScheduleFrom,
    /// Corresponds to Java: `SHOW_PROJECT_SUMMARY_TASK(54525961)`
    ShowProjectSummaryTask,
    /// Corresponds to Java: `SPLIT_TASKS(37748762)`
    SplitTasks,
    /// Corresponds to Java: `STANDARD_RATE(37748767)`
    StandardRate,
    /// Corresponds to Java: `START_TIME(37748764)`
    StartTime,
    /// Corresponds to Java: `STATUS_DATE(37748805)`
    StatusDate,
    /// Corresponds to Java: `SUBPROJECT_COUNT(37748868)`
    SubprojectCount,
    /// Corresponds to Java: `SUBPROJECT_DATA(37748898)`
    SubprojectData,
    /// Corresponds to Java: `SUBPROJECT_TASK_COUNT(37748900)`
    SubprojectTaskCount,
    /// Corresponds to Java: `TASK_FIELD_ATTRIBUTES(37753744)`
    TaskFieldAttributes,
    /// Corresponds to Java: `TASK_FIELD_MAP(131092)`
    TaskFieldMap,
    /// Corresponds to Java: `TASK_FIELD_MAP2(50331668)`
    TaskFieldMap2,
    /// Corresponds to Java: `TASK_FIELD_NAME_ALIASES(1048577)`
    TaskFieldNameAliases,
    /// Corresponds to Java: `TASK_UPDATES_RESOURCE(37748761)`
    TaskUpdatesResource,
    /// Corresponds to Java: `WEEK_START_DAY(37748773)`
    WeekStartDay,
    /// Corresponds to Java: `WORK_UNITS(37748758)`
    WorkUnits,
    /// Corresponds to Java: `WRITE_RESERVATION_PASSWORD_HASH(893386757)`
    WriteReservationPasswordHash,
    /// Corresponds to Java: `STYLE_DATA(574619656)`
    StyleData,
    /// Corresponds to Java: `BAR_EXCEPTION_STYLES(574619661)`
    BarExceptionStyles,
    /// Corresponds to Java: `ALTERNATE_STYLE_DATA(574619686)`
    AlternateStyleData,
    /// Corresponds to Java: `TIMESCALE_PROPERTIES(574619678)`
    TimescaleProperties,
    /// Corresponds to Java: `TABLE_PROPERTIES(574619655)`
    TableProperties,
    /// Corresponds to Java: `FILTER_NAME(574619659)`
    FilterName,
    /// Corresponds to Java: `GROUP_NAME(574619672)`
    GroupName,
    /// Corresponds to Java: `COLUMN_PROPERTIES(574619660)`
    ColumnProperties,
    /// Corresponds to Java: `PROGRESS_LINE_PROPERTIES(574619671)`
    ProgressLineProperties,
    /// Corresponds to Java: `AUTO_FILTER_PROPERTIES(574619669)`
    AutoFilterProperties,
    /// Corresponds to Java: `VIEW_NAME(641728536)`
    ViewName,
    /// Corresponds to Java: `DATA_LINK_FIELD_TYPE(641728535)`
    DataLinkFieldType,
    /// Corresponds to Java: `DATA_LINK_UNIQUE_ID(641728548)`
    DataLinkUniqueId,
    /// Corresponds to Java: `DATA_LINK_PATH(641728561)`
    DataLinkPath,
    /// Corresponds to Java: `TABLE_NAME(574619658)`
    TableName,
    /// Corresponds to Java: `VIEW_CONTENTS(641728565)`
    ViewContents,
    /// Corresponds to Java: `UPPER_VIEW_NAME(574619658)`
    UpperViewName,
    /// Corresponds to Java: `LOWER_VIEW_NAME(574619659)`
    LowerViewName,
    /// Corresponds to Java: `COMPANY(37748736)`
    Company,
    /// Corresponds to Java: `MANAGER(37748737)`
    Manager,
    /// Corresponds to Java: `COMMENTS(37748741)`
    Comments,
    /// Corresponds to Java: `TITLE(37748744)`
    Title,
    /// Corresponds to Java: `SUBJECT(37748745)`
    Subject,
    /// Corresponds to Java: `AUTHOR(37748748)`
    Author,
    /// Corresponds to Java: `KEYWORDS(37748749)`
    Keywords,
    /// Corresponds to Java: `CATEGORY(37748809)`
    Category,
}

/// Total number of variants.
///
/// Corresponds to Java: `PropsKey.values().length` (Java has no explicit constant).
pub const PROPS_KEY_COUNT: usize = 95;

/// The Java enum constant name for each variant (`UPPER_SNAKE`); the index is the variant number.
///
/// Corresponds to Java: `Enum.name()` / `Enum.toString()` (generated by the compiler).
const NAMES: [&str; PROPS_KEY_COUNT] = [
    "ASSIGNMENT_FIELD_MAP",
    "ASSIGNMENT_FIELD_MAP2",
    "AUTO_FILTER",
    "BASELINE10_DATE",
    "BASELINE1_DATE",
    "BASELINE2_DATE",
    "BASELINE3_DATE",
    "BASELINE4_DATE",
    "BASELINE5_DATE",
    "BASELINE6_DATE",
    "BASELINE7_DATE",
    "BASELINE8_DATE",
    "BASELINE9_DATE",
    "BASELINE_CALENDAR_NAME",
    "BASELINE_DATE",
    "CRITICAL_SLACK_LIMIT",
    "CURRENCY_CODE",
    "CURRENCY_DIGITS",
    "CURRENCY_PLACEMENT",
    "CURRENCY_SYMBOL",
    "CUSTOM_FIELDS",
    "DAYS_PER_MONTH",
    "DEFAULT_CALENDAR_HOURS",
    "DEFAULT_CALENDAR_NAME",
    "DEFAULT_TASK_TYPE",
    "DURATION_UNITS",
    "EDITABLE_ACTUAL_COSTS",
    "ENCRYPTION_CODE",
    "END_TIME",
    "ENTERPRISE_CUSTOM_FIELD_MAP",
    "FISCAL_YEAR_START",
    "FISCAL_YEAR_START_MONTH",
    "FONT_BASES",
    "GUID",
    "HONOR_CONSTRAINTS",
    "HYPERLINK_BASE",
    "MINUTES_PER_DAY",
    "MINUTES_PER_WEEK",
    "MULTIPLE_CRITICAL_PATHS",
    "NEW_TASKS_ARE_MANUAL",
    "OVERTIME_RATE",
    "PASSWORD_FLAG",
    "PROJECT_FILE_PATH",
    "PROJECT_FINISH_DATE",
    "PROJECT_START_DATE",
    "PROTECTION_PASSWORD_HASH",
    "RELATION_FIELD_MAP",
    "RESOURCE_CREATION_DATE",
    "RESOURCE_FIELD_MAP",
    "RESOURCE_FIELD_MAP2",
    "RESOURCE_FIELD_NAME_ALIASES",
    "RESOURCE_POOL",
    "SCHEDULE_FROM",
    "SHOW_PROJECT_SUMMARY_TASK",
    "SPLIT_TASKS",
    "STANDARD_RATE",
    "START_TIME",
    "STATUS_DATE",
    "SUBPROJECT_COUNT",
    "SUBPROJECT_DATA",
    "SUBPROJECT_TASK_COUNT",
    "TASK_FIELD_ATTRIBUTES",
    "TASK_FIELD_MAP",
    "TASK_FIELD_MAP2",
    "TASK_FIELD_NAME_ALIASES",
    "TASK_UPDATES_RESOURCE",
    "WEEK_START_DAY",
    "WORK_UNITS",
    "WRITE_RESERVATION_PASSWORD_HASH",
    "STYLE_DATA",
    "BAR_EXCEPTION_STYLES",
    "ALTERNATE_STYLE_DATA",
    "TIMESCALE_PROPERTIES",
    "TABLE_PROPERTIES",
    "FILTER_NAME",
    "GROUP_NAME",
    "COLUMN_PROPERTIES",
    "PROGRESS_LINE_PROPERTIES",
    "AUTO_FILTER_PROPERTIES",
    "VIEW_NAME",
    "DATA_LINK_FIELD_TYPE",
    "DATA_LINK_UNIQUE_ID",
    "DATA_LINK_PATH",
    "TABLE_NAME",
    "VIEW_CONTENTS",
    "UPPER_VIEW_NAME",
    "LOWER_VIEW_NAME",
    "COMPANY",
    "MANAGER",
    "COMMENTS",
    "TITLE",
    "SUBJECT",
    "AUTHOR",
    "KEYWORDS",
    "CATEGORY",
];

impl PropsKey {
/// All values, in the same order as the Java declaration order.
    ///
    /// Corresponds to Java: `PropsKey.values()`
    pub const ALL: [PropsKey; PROPS_KEY_COUNT] = [
        PropsKey::AssignmentFieldMap,
        PropsKey::AssignmentFieldMap2,
        PropsKey::AutoFilter,
        PropsKey::Baseline10Date,
        PropsKey::Baseline1Date,
        PropsKey::Baseline2Date,
        PropsKey::Baseline3Date,
        PropsKey::Baseline4Date,
        PropsKey::Baseline5Date,
        PropsKey::Baseline6Date,
        PropsKey::Baseline7Date,
        PropsKey::Baseline8Date,
        PropsKey::Baseline9Date,
        PropsKey::BaselineCalendarName,
        PropsKey::BaselineDate,
        PropsKey::CriticalSlackLimit,
        PropsKey::CurrencyCode,
        PropsKey::CurrencyDigits,
        PropsKey::CurrencyPlacement,
        PropsKey::CurrencySymbol,
        PropsKey::CustomFields,
        PropsKey::DaysPerMonth,
        PropsKey::DefaultCalendarHours,
        PropsKey::DefaultCalendarName,
        PropsKey::DefaultTaskType,
        PropsKey::DurationUnits,
        PropsKey::EditableActualCosts,
        PropsKey::EncryptionCode,
        PropsKey::EndTime,
        PropsKey::EnterpriseCustomFieldMap,
        PropsKey::FiscalYearStart,
        PropsKey::FiscalYearStartMonth,
        PropsKey::FontBases,
        PropsKey::Guid,
        PropsKey::HonorConstraints,
        PropsKey::HyperlinkBase,
        PropsKey::MinutesPerDay,
        PropsKey::MinutesPerWeek,
        PropsKey::MultipleCriticalPaths,
        PropsKey::NewTasksAreManual,
        PropsKey::OvertimeRate,
        PropsKey::PasswordFlag,
        PropsKey::ProjectFilePath,
        PropsKey::ProjectFinishDate,
        PropsKey::ProjectStartDate,
        PropsKey::ProtectionPasswordHash,
        PropsKey::RelationFieldMap,
        PropsKey::ResourceCreationDate,
        PropsKey::ResourceFieldMap,
        PropsKey::ResourceFieldMap2,
        PropsKey::ResourceFieldNameAliases,
        PropsKey::ResourcePool,
        PropsKey::ScheduleFrom,
        PropsKey::ShowProjectSummaryTask,
        PropsKey::SplitTasks,
        PropsKey::StandardRate,
        PropsKey::StartTime,
        PropsKey::StatusDate,
        PropsKey::SubprojectCount,
        PropsKey::SubprojectData,
        PropsKey::SubprojectTaskCount,
        PropsKey::TaskFieldAttributes,
        PropsKey::TaskFieldMap,
        PropsKey::TaskFieldMap2,
        PropsKey::TaskFieldNameAliases,
        PropsKey::TaskUpdatesResource,
        PropsKey::WeekStartDay,
        PropsKey::WorkUnits,
        PropsKey::WriteReservationPasswordHash,
        PropsKey::StyleData,
        PropsKey::BarExceptionStyles,
        PropsKey::AlternateStyleData,
        PropsKey::TimescaleProperties,
        PropsKey::TableProperties,
        PropsKey::FilterName,
        PropsKey::GroupName,
        PropsKey::ColumnProperties,
        PropsKey::ProgressLineProperties,
        PropsKey::AutoFilterProperties,
        PropsKey::ViewName,
        PropsKey::DataLinkFieldType,
        PropsKey::DataLinkUniqueId,
        PropsKey::DataLinkPath,
        PropsKey::TableName,
        PropsKey::ViewContents,
        PropsKey::UpperViewName,
        PropsKey::LowerViewName,
        PropsKey::Company,
        PropsKey::Manager,
        PropsKey::Comments,
        PropsKey::Title,
        PropsKey::Subject,
        PropsKey::Author,
        PropsKey::Keywords,
        PropsKey::Category,
    ];

/// Get the key's integer key value.
    ///
    /// Corresponds to Java: `public Integer getValue()`
    pub fn get_value(&self) -> i32 {
        match self {
            PropsKey::AssignmentFieldMap => 131095,
            PropsKey::AssignmentFieldMap2 => 50331671,
            PropsKey::AutoFilter => 893386767,
            PropsKey::Baseline10Date => 37753759,
            PropsKey::Baseline1Date => 37753750,
            PropsKey::Baseline2Date => 37753751,
            PropsKey::Baseline3Date => 37753752,
            PropsKey::Baseline4Date => 37753753,
            PropsKey::Baseline5Date => 37753754,
            PropsKey::Baseline6Date => 37753755,
            PropsKey::Baseline7Date => 37753756,
            PropsKey::Baseline8Date => 37753757,
            PropsKey::Baseline9Date => 37753758,
            PropsKey::BaselineCalendarName => 37753747,
            PropsKey::BaselineDate => 37753749,
            PropsKey::CriticalSlackLimit => 37748756,
            PropsKey::CurrencyCode => 37753787,
            PropsKey::CurrencyDigits => 37748754,
            PropsKey::CurrencyPlacement => 37748753,
            PropsKey::CurrencySymbol => 37748752,
            PropsKey::CustomFields => 71303169,
            PropsKey::DaysPerMonth => 37753743,
            PropsKey::DefaultCalendarHours => 37753736,
            PropsKey::DefaultCalendarName => 37748750,
            PropsKey::DefaultTaskType => 37748785,
            PropsKey::DurationUnits => 37748757,
            PropsKey::EditableActualCosts => 37748802,
            PropsKey::EncryptionCode => 893386759,
            PropsKey::EndTime => 37748769,
            PropsKey::EnterpriseCustomFieldMap => 37753797,
            PropsKey::FiscalYearStart => 37748801,
            PropsKey::FiscalYearStartMonth => 37748780,
            PropsKey::FontBases => 54525952,
            PropsKey::Guid => 37748777,
            PropsKey::HonorConstraints => 37748794,
            PropsKey::HyperlinkBase => 37748810,
            PropsKey::MinutesPerDay => 37748765,
            PropsKey::MinutesPerWeek => 37748766,
            PropsKey::MultipleCriticalPaths => 37748793,
            PropsKey::NewTasksAreManual => 37753800,
            PropsKey::OvertimeRate => 37748768,
            PropsKey::PasswordFlag => 893386752,
            PropsKey::ProjectFilePath => 893386760,
            PropsKey::ProjectFinishDate => 37748739,
            PropsKey::ProjectStartDate => 37748738,
            PropsKey::ProtectionPasswordHash => 893386756,
            PropsKey::RelationFieldMap => 131094,
            PropsKey::ResourceCreationDate => 205521219,
            PropsKey::ResourceFieldMap => 131093,
            PropsKey::ResourceFieldMap2 => 50331669,
            PropsKey::ResourceFieldNameAliases => 1048578,
            PropsKey::ResourcePool => 37748747,
            PropsKey::ScheduleFrom => 37748740,
            PropsKey::ShowProjectSummaryTask => 54525961,
            PropsKey::SplitTasks => 37748762,
            PropsKey::StandardRate => 37748767,
            PropsKey::StartTime => 37748764,
            PropsKey::StatusDate => 37748805,
            PropsKey::SubprojectCount => 37748868,
            PropsKey::SubprojectData => 37748898,
            PropsKey::SubprojectTaskCount => 37748900,
            PropsKey::TaskFieldAttributes => 37753744,
            PropsKey::TaskFieldMap => 131092,
            PropsKey::TaskFieldMap2 => 50331668,
            PropsKey::TaskFieldNameAliases => 1048577,
            PropsKey::TaskUpdatesResource => 37748761,
            PropsKey::WeekStartDay => 37748773,
            PropsKey::WorkUnits => 37748758,
            PropsKey::WriteReservationPasswordHash => 893386757,
            PropsKey::StyleData => 574619656,
            PropsKey::BarExceptionStyles => 574619661,
            PropsKey::AlternateStyleData => 574619686,
            PropsKey::TimescaleProperties => 574619678,
            PropsKey::TableProperties => 574619655,
            PropsKey::FilterName => 574619659,
            PropsKey::GroupName => 574619672,
            PropsKey::ColumnProperties => 574619660,
            PropsKey::ProgressLineProperties => 574619671,
            PropsKey::AutoFilterProperties => 574619669,
            PropsKey::ViewName => 641728536,
            PropsKey::DataLinkFieldType => 641728535,
            PropsKey::DataLinkUniqueId => 641728548,
            PropsKey::DataLinkPath => 641728561,
            PropsKey::TableName => 574619658,
            PropsKey::ViewContents => 641728565,
            PropsKey::UpperViewName => 574619658,
            PropsKey::LowerViewName => 574619659,
            PropsKey::Company => 37748736,
            PropsKey::Manager => 37748737,
            PropsKey::Comments => 37748741,
            PropsKey::Title => 37748744,
            PropsKey::Subject => 37748745,
            PropsKey::Author => 37748748,
            PropsKey::Keywords => 37748749,
            PropsKey::Category => 37748809,
        }
    }

/// Get the enum instance by key value (for duplicate key values, return the first in declaration order); return `None` if not found.
    ///
/// No counterpart on the Java side (Java does not look up by key value).
    pub fn get_instance(value: i32) -> Option<PropsKey> {
        Self::ALL.iter().copied().find(|key| key.get_value() == value)
    }

/// Get the Java-side enum constant name.
    ///
/// Corresponds to Java: `Enum.name()` (`Props`'s `toString()` uses `Enum.toString()`)
    pub fn get_name(&self) -> &'static str {
        NAMES[*self as usize]
    }
}

impl MpxjEnum for PropsKey {
/// Return the key value.
    ///
    /// Corresponds to Java: `public Integer getValue()`
    ///
/// Note: unlike other enums, this enum's `value()` is not the declaration ordinal but the key value in the MPP file.
    fn value(&self) -> i32 {
        self.get_value()
    }
}

impl From<PropsKey> for i32 {
    fn from(value: PropsKey) -> i32 {
        value.get_value()
    }
}

impl std::fmt::Display for PropsKey {
    /// Corresponds to Java: `Enum.toString()`
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_matches_declaration_order_and_names() {
        assert_eq!(PropsKey::ALL.len(), PROPS_KEY_COUNT);
        assert_eq!(NAMES.len(), PROPS_KEY_COUNT);

// The variant number corresponds one-to-one with the constant name table
        for key in PropsKey::ALL {
            assert_eq!(key.get_name(), NAMES[key as usize]);
        }

        assert_eq!(PropsKey::ALL[0], PropsKey::AssignmentFieldMap);
        assert_eq!(PropsKey::ALL[PROPS_KEY_COUNT - 1], PropsKey::Category);
    }

    #[test]
    fn values_match_java_source() {
        assert_eq!(PropsKey::AssignmentFieldMap.get_value(), 131095);
        assert_eq!(PropsKey::TaskFieldMap.get_value(), 131092);
        assert_eq!(PropsKey::Baseline1Date.get_value(), 37753750);
        assert_eq!(PropsKey::Baseline10Date.get_value(), 37753759);
        assert_eq!(PropsKey::Company.get_value(), 37748736);
        assert_eq!(PropsKey::Category.get_value(), 37748809);
        assert_eq!(PropsKey::StyleData.get_value(), 574619656);
        assert_eq!(PropsKey::ViewContents.get_value(), 641728565);
        assert_eq!(PropsKey::ProtectionPasswordHash.get_value(), 893386756);
        assert_eq!(PropsKey::AutoFilter.get_value(), 893386767);
        assert_eq!(PropsKey::ResourceCreationDate.get_value(), 205521219);
    }

    #[test]
    fn get_instance_round_trips() {
// Key values are duplicated (see duplicate_values_return_first_declaration),
// so round-trip consistency is guaranteed only for the "first declaration"; for the rest only the key value is guaranteed the same
        for key in PropsKey::ALL {
            let found = PropsKey::get_instance(key.get_value()).expect("should be found");
            assert_eq!(found.get_value(), key.get_value());
        }

        assert_eq!(
            PropsKey::get_instance(PropsKey::Company.get_value()),
            Some(PropsKey::Company)
        );
        assert_eq!(PropsKey::get_instance(0), None);
        assert_eq!(PropsKey::get_instance(-1), None);
    }

    #[test]
    fn duplicate_values_return_first_declaration() {
// Java: TABLE_NAME(574619658) and UPPER_VIEW_NAME(574619658)
        assert_eq!(
            PropsKey::get_instance(574619658),
            Some(PropsKey::TableName)
        );
// Java: FILTER_NAME(574619659) and LOWER_VIEW_NAME(574619659)
        assert_eq!(PropsKey::get_instance(574619659), Some(PropsKey::FilterName));
// Java: BASELINE_DATE(37753749) and KEYWORDS(37748749) have different values; only the names are similar
        assert_eq!(
            PropsKey::get_instance(37753749),
            Some(PropsKey::BaselineDate)
        );
    }

    #[test]
    fn value_and_display() {
        assert_eq!(MpxjEnum::value(&PropsKey::Company), 37748736);
        assert_eq!(i32::from(PropsKey::Company), 37748736);
        assert_eq!(PropsKey::Company.to_string(), "COMPANY");
        assert_eq!(PropsKey::TaskFieldMap2.to_string(), "TASK_FIELD_MAP2");
    }
}
