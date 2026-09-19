//! 对应 Java: `org/mpxj/DataType.java`

use crate::mpxj_enum::MpxjEnum;

/// 表示某个属性的数据类型。
///
/// 对应 Java: `public enum DataType implements MpxjEnum`
///
/// Java 侧用一个 `EnumHelper.createTypeArray` 反射生成的数组做 int → 枚举映射；
/// Rust 侧不需要反射，直接在 [`DataType::get_instance`] 里用 `match` 完成。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataType {
    String = 1,
    Date = 2,
    Currency = 3,
    Boolean = 4,
    Numeric = 5,
    Duration = 6,
    Units = 7,
    Percentage = 8,
    Accrue = 9,
    Constraint = 10,
    Rate = 11,
    Priority = 12,
    RelationList = 13,
    TaskType = 14,
    ResourceType = 15,
    Work = 16,
    Integer = 17,
    Notes = 18,
    Short = 19,
    Binary = 20,
    Delay = 21,
    WorkUnits = 22,
    Workgroup = 23,
    Guid = 24,
    RateUnits = 25,
    EarnedValueMethod = 26,
    ResourceRequestType = 27,
    CurrencySymbolPosition = 28,
    Char = 29,
    DateOrder = 30,
    ProjectTimeFormat = 31,
    ProjectDateFormat = 32,
    ScheduleFrom = 33,
    Day = 34,
    Map = 35,
    MpxFileVersion = 36,
    MpxCodePage = 37,
    BookingType = 38,
    TimeUnits = 39,
    DateRangeList = 40,
    Subproject = 41,
    WorkContour = 42,
    ExpenseItemList = 43,
    PercentCompleteType = 44,
    ActivityStatus = 45,
    ActivityType = 46,
    CriticalActivityType = 47,
    /// Java 注释标记为 `// TODO: No longer used`
    ActivityCodeList = 48,
    Time = 49,
    Custom = 50,
    RateSource = 51,
    TaskMode = 52,
    StepList = 53,
    TotalSlackType = 54,
    RelationshipLagCalendar = 55,
    SchedulingProgressedActivities = 56,
    ActivityCodeValues = 57,
    CodeValues = 58,
    TimephasedWorkList = 59,
    TimephasedCostList = 60,
}

impl DataType {
    /// 全部取值，按 int 值升序排列。
    pub const ALL: [DataType; 60] = [
        DataType::String,
        DataType::Date,
        DataType::Currency,
        DataType::Boolean,
        DataType::Numeric,
        DataType::Duration,
        DataType::Units,
        DataType::Percentage,
        DataType::Accrue,
        DataType::Constraint,
        DataType::Rate,
        DataType::Priority,
        DataType::RelationList,
        DataType::TaskType,
        DataType::ResourceType,
        DataType::Work,
        DataType::Integer,
        DataType::Notes,
        DataType::Short,
        DataType::Binary,
        DataType::Delay,
        DataType::WorkUnits,
        DataType::Workgroup,
        DataType::Guid,
        DataType::RateUnits,
        DataType::EarnedValueMethod,
        DataType::ResourceRequestType,
        DataType::CurrencySymbolPosition,
        DataType::Char,
        DataType::DateOrder,
        DataType::ProjectTimeFormat,
        DataType::ProjectDateFormat,
        DataType::ScheduleFrom,
        DataType::Day,
        DataType::Map,
        DataType::MpxFileVersion,
        DataType::MpxCodePage,
        DataType::BookingType,
        DataType::TimeUnits,
        DataType::DateRangeList,
        DataType::Subproject,
        DataType::WorkContour,
        DataType::ExpenseItemList,
        DataType::PercentCompleteType,
        DataType::ActivityStatus,
        DataType::ActivityType,
        DataType::CriticalActivityType,
        DataType::ActivityCodeList,
        DataType::Time,
        DataType::Custom,
        DataType::RateSource,
        DataType::TaskMode,
        DataType::StepList,
        DataType::TotalSlackType,
        DataType::RelationshipLagCalendar,
        DataType::SchedulingProgressedActivities,
        DataType::ActivityCodeValues,
        DataType::CodeValues,
        DataType::TimephasedWorkList,
        DataType::TimephasedCostList,
    ];

    /// 根据 int 值取枚举实例，越界时回落到 `STRING`。
    ///
    /// 对应 Java: `public static DataType getInstance(int type)`
    pub fn get_instance(value: i32) -> DataType {
        match value {
            1 => DataType::String,
            2 => DataType::Date,
            3 => DataType::Currency,
            4 => DataType::Boolean,
            5 => DataType::Numeric,
            6 => DataType::Duration,
            7 => DataType::Units,
            8 => DataType::Percentage,
            9 => DataType::Accrue,
            10 => DataType::Constraint,
            11 => DataType::Rate,
            12 => DataType::Priority,
            13 => DataType::RelationList,
            14 => DataType::TaskType,
            15 => DataType::ResourceType,
            16 => DataType::Work,
            17 => DataType::Integer,
            18 => DataType::Notes,
            19 => DataType::Short,
            20 => DataType::Binary,
            21 => DataType::Delay,
            22 => DataType::WorkUnits,
            23 => DataType::Workgroup,
            24 => DataType::Guid,
            25 => DataType::RateUnits,
            26 => DataType::EarnedValueMethod,
            27 => DataType::ResourceRequestType,
            28 => DataType::CurrencySymbolPosition,
            29 => DataType::Char,
            30 => DataType::DateOrder,
            31 => DataType::ProjectTimeFormat,
            32 => DataType::ProjectDateFormat,
            33 => DataType::ScheduleFrom,
            34 => DataType::Day,
            35 => DataType::Map,
            36 => DataType::MpxFileVersion,
            37 => DataType::MpxCodePage,
            38 => DataType::BookingType,
            39 => DataType::TimeUnits,
            40 => DataType::DateRangeList,
            41 => DataType::Subproject,
            42 => DataType::WorkContour,
            43 => DataType::ExpenseItemList,
            44 => DataType::PercentCompleteType,
            45 => DataType::ActivityStatus,
            46 => DataType::ActivityType,
            47 => DataType::CriticalActivityType,
            48 => DataType::ActivityCodeList,
            49 => DataType::Time,
            50 => DataType::Custom,
            51 => DataType::RateSource,
            52 => DataType::TaskMode,
            53 => DataType::StepList,
            54 => DataType::TotalSlackType,
            55 => DataType::RelationshipLagCalendar,
            56 => DataType::SchedulingProgressedActivities,
            57 => DataType::ActivityCodeValues,
            58 => DataType::CodeValues,
            59 => DataType::TimephasedWorkList,
            60 => DataType::TimephasedCostList,
            _ => DataType::String,
        }
    }

    /// 对应 Java: `public static DataType getInstance(Number type)`
    ///
    /// Java 侧 `null` 会走 `getInstance(-1)` 从而回落到 `STRING`。
    pub fn get_instance_optional(value: Option<i32>) -> DataType {
        DataType::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for DataType {
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<DataType> for i32 {
    fn from(value: DataType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_type_round_trips() {
        for item in DataType::ALL {
            assert_eq!(DataType::get_instance(item.value()), item);
        }
    }

    #[test]
    fn data_type_falls_back_to_string() {
        assert_eq!(DataType::get_instance(0), DataType::String);
        assert_eq!(DataType::get_instance(61), DataType::String);
        assert_eq!(DataType::get_instance(-1), DataType::String);
        assert_eq!(DataType::get_instance_optional(None), DataType::String);
    }
}
