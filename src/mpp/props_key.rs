//! 对应 Java: `org/mpxj/mpp/PropsKey.java`

use crate::mpxj_enum::MpxjEnum;

/// `Props` 块中使用的键。
///
/// 对应 Java: `enum PropsKey`
///
/// 说明：
/// - 变体顺序与 Java 的声明顺序完全一致，名称按 `UPPER_SNAKE` → `PascalCase` 转换。
/// - Java 侧该枚举只有 `getValue()`（未声明 `implements MpxjEnum`）；Rust 侧按本工程
///   约定实现 [`MpxjEnum`]（`value()` 返回同一个键值），并额外提供 Java 中由编译器生成的
///   `values()` / `name()` / `toString()` 的等价物：[`PropsKey::ALL`]、
///   [`PropsKey::get_name`]、[`Display`]。
/// - [`PropsKey::get_instance`] 在 Java 侧没有对应方法（Java 用反射表 `EnumHelper` 的写法
///   在本枚举中也不存在），Rust 侧按「按值查找」提供，供上层按键值反查枚举使用。
/// - 键值存在重复（例如 `TABLE_NAME` 与 `UPPER_VIEW_NAME` 都是 574619658，
///   `KEYWORDS` 与 `BASELINE_DATE` 都是 37748749），与 Java 注释「a few duplicate values」
///   一致；`get_instance` 返回声明顺序中的第一个匹配项。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropsKey {
    /// 对应 Java: `ASSIGNMENT_FIELD_MAP(131095)`
    AssignmentFieldMap,
    /// 对应 Java: `ASSIGNMENT_FIELD_MAP2(50331671)`
    AssignmentFieldMap2,
    /// 对应 Java: `AUTO_FILTER(893386767)`
    AutoFilter,
    /// 对应 Java: `BASELINE10_DATE(37753759)`
    Baseline10Date,
    /// 对应 Java: `BASELINE1_DATE(37753750)`
    Baseline1Date,
    /// 对应 Java: `BASELINE2_DATE(37753751)`
    Baseline2Date,
    /// 对应 Java: `BASELINE3_DATE(37753752)`
    Baseline3Date,
    /// 对应 Java: `BASELINE4_DATE(37753753)`
    Baseline4Date,
    /// 对应 Java: `BASELINE5_DATE(37753754)`
    Baseline5Date,
    /// 对应 Java: `BASELINE6_DATE(37753755)`
    Baseline6Date,
    /// 对应 Java: `BASELINE7_DATE(37753756)`
    Baseline7Date,
    /// 对应 Java: `BASELINE8_DATE(37753757)`
    Baseline8Date,
    /// 对应 Java: `BASELINE9_DATE(37753758)`
    Baseline9Date,
    /// 对应 Java: `BASELINE_CALENDAR_NAME(37753747)`
    BaselineCalendarName,
    /// 对应 Java: `BASELINE_DATE(37753749)`
    BaselineDate,
    /// 对应 Java: `CRITICAL_SLACK_LIMIT(37748756)`
    CriticalSlackLimit,
    /// 对应 Java: `CURRENCY_CODE(37753787)`
    CurrencyCode,
    /// 对应 Java: `CURRENCY_DIGITS(37748754)`
    CurrencyDigits,
    /// 对应 Java: `CURRENCY_PLACEMENT(37748753)`
    CurrencyPlacement,
    /// 对应 Java: `CURRENCY_SYMBOL(37748752)`
    CurrencySymbol,
    /// 对应 Java: `CUSTOM_FIELDS(71303169)`
    CustomFields,
    /// 对应 Java: `DAYS_PER_MONTH(37753743)`
    DaysPerMonth,
    /// 对应 Java: `DEFAULT_CALENDAR_HOURS(37753736)`
    DefaultCalendarHours,
    /// 对应 Java: `DEFAULT_CALENDAR_NAME(37748750)`
    DefaultCalendarName,
    /// 对应 Java: `DEFAULT_TASK_TYPE(37748785)`
    DefaultTaskType,
    /// 对应 Java: `DURATION_UNITS(37748757)`
    DurationUnits,
    /// 对应 Java: `EDITABLE_ACTUAL_COSTS(37748802)`
    EditableActualCosts,
    /// 对应 Java: `ENCRYPTION_CODE(893386759)`
    EncryptionCode,
    /// 对应 Java: `END_TIME(37748769)`
    EndTime,
    /// 对应 Java: `ENTERPRISE_CUSTOM_FIELD_MAP(37753797)`
    EnterpriseCustomFieldMap,
    /// 对应 Java: `FISCAL_YEAR_START(37748801)`
    FiscalYearStart,
    /// 对应 Java: `FISCAL_YEAR_START_MONTH(37748780)`
    FiscalYearStartMonth,
    /// 对应 Java: `FONT_BASES(54525952)`
    FontBases,
    /// 对应 Java: `GUID(37748777)`
    Guid,
    /// 对应 Java: `HONOR_CONSTRAINTS(37748794)`
    HonorConstraints,
    /// 对应 Java: `HYPERLINK_BASE(37748810)`
    HyperlinkBase,
    /// 对应 Java: `MINUTES_PER_DAY(37748765)`
    MinutesPerDay,
    /// 对应 Java: `MINUTES_PER_WEEK(37748766)`
    MinutesPerWeek,
    /// 对应 Java: `MULTIPLE_CRITICAL_PATHS(37748793)`
    MultipleCriticalPaths,
    /// 对应 Java: `NEW_TASKS_ARE_MANUAL(37753800)`
    NewTasksAreManual,
    /// 对应 Java: `OVERTIME_RATE(37748768)`
    OvertimeRate,
    /// 对应 Java: `PASSWORD_FLAG(893386752)`
    PasswordFlag,
    /// 对应 Java: `PROJECT_FILE_PATH(893386760)`
    ProjectFilePath,
    /// 对应 Java: `PROJECT_FINISH_DATE(37748739)`
    ProjectFinishDate,
    /// 对应 Java: `PROJECT_START_DATE(37748738)`
    ProjectStartDate,
    /// 对应 Java: `PROTECTION_PASSWORD_HASH(893386756)`
    ProtectionPasswordHash,
    /// 对应 Java: `RELATION_FIELD_MAP(131094)`
    RelationFieldMap,
    /// 对应 Java: `RESOURCE_CREATION_DATE(205521219)`
    ResourceCreationDate,
    /// 对应 Java: `RESOURCE_FIELD_MAP(131093)`
    ResourceFieldMap,
    /// 对应 Java: `RESOURCE_FIELD_MAP2(50331669)`
    ResourceFieldMap2,
    /// 对应 Java: `RESOURCE_FIELD_NAME_ALIASES(1048578)`
    ResourceFieldNameAliases,
    /// 对应 Java: `RESOURCE_POOL(37748747)`
    ResourcePool,
    /// 对应 Java: `SCHEDULE_FROM(37748740)`
    ScheduleFrom,
    /// 对应 Java: `SHOW_PROJECT_SUMMARY_TASK(54525961)`
    ShowProjectSummaryTask,
    /// 对应 Java: `SPLIT_TASKS(37748762)`
    SplitTasks,
    /// 对应 Java: `STANDARD_RATE(37748767)`
    StandardRate,
    /// 对应 Java: `START_TIME(37748764)`
    StartTime,
    /// 对应 Java: `STATUS_DATE(37748805)`
    StatusDate,
    /// 对应 Java: `SUBPROJECT_COUNT(37748868)`
    SubprojectCount,
    /// 对应 Java: `SUBPROJECT_DATA(37748898)`
    SubprojectData,
    /// 对应 Java: `SUBPROJECT_TASK_COUNT(37748900)`
    SubprojectTaskCount,
    /// 对应 Java: `TASK_FIELD_ATTRIBUTES(37753744)`
    TaskFieldAttributes,
    /// 对应 Java: `TASK_FIELD_MAP(131092)`
    TaskFieldMap,
    /// 对应 Java: `TASK_FIELD_MAP2(50331668)`
    TaskFieldMap2,
    /// 对应 Java: `TASK_FIELD_NAME_ALIASES(1048577)`
    TaskFieldNameAliases,
    /// 对应 Java: `TASK_UPDATES_RESOURCE(37748761)`
    TaskUpdatesResource,
    /// 对应 Java: `WEEK_START_DAY(37748773)`
    WeekStartDay,
    /// 对应 Java: `WORK_UNITS(37748758)`
    WorkUnits,
    /// 对应 Java: `WRITE_RESERVATION_PASSWORD_HASH(893386757)`
    WriteReservationPasswordHash,
    /// 对应 Java: `STYLE_DATA(574619656)`
    StyleData,
    /// 对应 Java: `BAR_EXCEPTION_STYLES(574619661)`
    BarExceptionStyles,
    /// 对应 Java: `ALTERNATE_STYLE_DATA(574619686)`
    AlternateStyleData,
    /// 对应 Java: `TIMESCALE_PROPERTIES(574619678)`
    TimescaleProperties,
    /// 对应 Java: `TABLE_PROPERTIES(574619655)`
    TableProperties,
    /// 对应 Java: `FILTER_NAME(574619659)`
    FilterName,
    /// 对应 Java: `GROUP_NAME(574619672)`
    GroupName,
    /// 对应 Java: `COLUMN_PROPERTIES(574619660)`
    ColumnProperties,
    /// 对应 Java: `PROGRESS_LINE_PROPERTIES(574619671)`
    ProgressLineProperties,
    /// 对应 Java: `AUTO_FILTER_PROPERTIES(574619669)`
    AutoFilterProperties,
    /// 对应 Java: `VIEW_NAME(641728536)`
    ViewName,
    /// 对应 Java: `DATA_LINK_FIELD_TYPE(641728535)`
    DataLinkFieldType,
    /// 对应 Java: `DATA_LINK_UNIQUE_ID(641728548)`
    DataLinkUniqueId,
    /// 对应 Java: `DATA_LINK_PATH(641728561)`
    DataLinkPath,
    /// 对应 Java: `TABLE_NAME(574619658)`
    TableName,
    /// 对应 Java: `VIEW_CONTENTS(641728565)`
    ViewContents,
    /// 对应 Java: `UPPER_VIEW_NAME(574619658)`
    UpperViewName,
    /// 对应 Java: `LOWER_VIEW_NAME(574619659)`
    LowerViewName,
    /// 对应 Java: `COMPANY(37748736)`
    Company,
    /// 对应 Java: `MANAGER(37748737)`
    Manager,
    /// 对应 Java: `COMMENTS(37748741)`
    Comments,
    /// 对应 Java: `TITLE(37748744)`
    Title,
    /// 对应 Java: `SUBJECT(37748745)`
    Subject,
    /// 对应 Java: `AUTHOR(37748748)`
    Author,
    /// 对应 Java: `KEYWORDS(37748749)`
    Keywords,
    /// 对应 Java: `CATEGORY(37748809)`
    Category,
}

/// 变体总数。
///
/// 对应 Java: `PropsKey.values().length`（Java 无显式常量）。
pub const PROPS_KEY_COUNT: usize = 95;

/// 各变体对应的 Java 枚举常量名（`UPPER_SNAKE`），下标即变体序号。
///
/// 对应 Java: `Enum.name()` / `Enum.toString()`（由编译器生成）。
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
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `PropsKey.values()`
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

    /// 取键的整数键值。
    ///
    /// 对应 Java: `public Integer getValue()`
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

    /// 按键值取枚举实例（重复键值返回声明顺序中的第一个），找不到返回 `None`。
    ///
    /// Java 侧无对应方法（Java 不使用按键值反查）。
    pub fn get_instance(value: i32) -> Option<PropsKey> {
        Self::ALL.iter().copied().find(|key| key.get_value() == value)
    }

    /// 取 Java 侧的枚举常量名。
    ///
    /// 对应 Java: `Enum.name()`（`Props` 的 `toString()` 用到 `Enum.toString()`）
    pub fn get_name(&self) -> &'static str {
        NAMES[*self as usize]
    }
}

impl MpxjEnum for PropsKey {
    /// 返回键值。
    ///
    /// 对应 Java: `public Integer getValue()`
    ///
    /// 注意：与其它枚举不同，本枚举的 `value()` 不是声明序号，而是 MPP 文件中的键值。
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
    /// 对应 Java: `Enum.toString()`
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

        // 变体序号与常量名表一一对应
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
        // 键值有重复（见 duplicate_values_return_first_declaration），
        // 因此只对「首个声明」保证往返一致，其余保证键值相同
        for key in PropsKey::ALL {
            let found = PropsKey::get_instance(key.get_value()).expect("应能找到");
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
        // Java: TABLE_NAME(574619658) 与 UPPER_VIEW_NAME(574619658)
        assert_eq!(
            PropsKey::get_instance(574619658),
            Some(PropsKey::TableName)
        );
        // Java: FILTER_NAME(574619659) 与 LOWER_VIEW_NAME(574619659)
        assert_eq!(PropsKey::get_instance(574619659), Some(PropsKey::FilterName));
        // Java: BASELINE_DATE(37753749) 与 KEYWORDS(37748749) 值不同，仅名称相似
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
