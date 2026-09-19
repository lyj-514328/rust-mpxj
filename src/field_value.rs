//! 对应 Java 中 `AbstractFieldContainer` 存储的 `Object` 字段值。
//!
//! Java 用 `Map<FieldType, Object>` 保存字段，值的实际类型由字段的
//! `DataType` 决定。Rust 侧需要一个具体枚举来承载这些值，因此引入
//! `FieldValue`。
//!
//! # 增量扩展说明
//!
//! 目前只包含标量类型。Java 侧还会往字段表里存实体引用与集合
//! （例如 `AssignmentField.RESOURCE` 存 `Resource`、
//! `TaskField.PREDECESSORS` 存 `List<Relation>`、
//! `AssignmentField.RAW_TIMEPHASED_PLANNED_WORK` 存 `List<TimephasedWork>`）。
//! 这些变体会在对应的领域类型移植完成后补入本枚举。

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

use crate::duration::Duration;
use crate::rate::Rate;

/// 字段值，对应 Java 存放在 `Map<FieldType, Object>` 里的 `Object`。
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    /// 对应 Java `Integer` / `int`
    Integer(i32),
    /// 对应 Java `Short` / `short`
    Short(i16),
    /// 对应 Java `Double` / `double`
    Double(f64),
    /// 对应 Java `Boolean` / `boolean`
    Boolean(bool),
    /// 对应 Java `String`
    String(String),
    /// 对应 Java `java.time.LocalDate`
    Date(NaiveDate),
    /// 对应 Java `java.time.LocalTime`
    Time(NaiveTime),
    /// 对应 Java `java.time.LocalDateTime`
    DateTime(NaiveDateTime),
    /// 对应 Java `org.mpxj.Duration`
    Duration(Duration),
    /// 对应 Java `org.mpxj.Rate`
    Rate(Rate),
}

impl FieldValue {
    /// 取整数，类型不匹配时返回 `None`。
    pub fn as_integer(&self) -> Option<i32> {
        match self {
            FieldValue::Integer(value) => Some(*value),
            FieldValue::Short(value) => Some(i32::from(*value)),
            _ => None,
        }
    }

    /// 取双精度浮点，类型不匹配时返回 `None`。
    pub fn as_double(&self) -> Option<f64> {
        match self {
            FieldValue::Double(value) => Some(*value),
            FieldValue::Integer(value) => Some(f64::from(*value)),
            FieldValue::Short(value) => Some(f64::from(*value)),
            _ => None,
        }
    }

    /// 取布尔值，类型不匹配时返回 `None`。
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            FieldValue::Boolean(value) => Some(*value),
            _ => None,
        }
    }

    /// 取字符串引用，类型不匹配时返回 `None`。
    pub fn as_string(&self) -> Option<&str> {
        match self {
            FieldValue::String(value) => Some(value),
            _ => None,
        }
    }

    /// 取日期，类型不匹配时返回 `None`。
    pub fn as_date(&self) -> Option<NaiveDate> {
        match self {
            FieldValue::Date(value) => Some(*value),
            FieldValue::DateTime(value) => Some(value.date()),
            _ => None,
        }
    }

    /// 取时间，类型不匹配时返回 `None`。
    pub fn as_time(&self) -> Option<NaiveTime> {
        match self {
            FieldValue::Time(value) => Some(*value),
            FieldValue::DateTime(value) => Some(value.time()),
            _ => None,
        }
    }

    /// 取日期时间，类型不匹配时返回 `None`。
    pub fn as_date_time(&self) -> Option<NaiveDateTime> {
        match self {
            FieldValue::DateTime(value) => Some(*value),
            FieldValue::Date(value) => value.and_hms_opt(0, 0, 0),
            _ => None,
        }
    }

    /// 取工期，类型不匹配时返回 `None`。
    pub fn as_duration(&self) -> Option<Duration> {
        match self {
            FieldValue::Duration(value) => Some(*value),
            _ => None,
        }
    }

    /// 取费率，类型不匹配时返回 `None`。
    pub fn as_rate(&self) -> Option<Rate> {
        match self {
            FieldValue::Rate(value) => Some(*value),
            _ => None,
        }
    }

    /// 返回该值在 Java 侧对应的类型名，便于日志与错误信息。
    pub fn type_name(&self) -> &'static str {
        match self {
            FieldValue::Integer(_) => "Integer",
            FieldValue::Short(_) => "Short",
            FieldValue::Double(_) => "Double",
            FieldValue::Boolean(_) => "Boolean",
            FieldValue::String(_) => "String",
            FieldValue::Date(_) => "LocalDate",
            FieldValue::Time(_) => "LocalTime",
            FieldValue::DateTime(_) => "LocalDateTime",
            FieldValue::Duration(_) => "Duration",
            FieldValue::Rate(_) => "Rate",
        }
    }
}

impl From<i32> for FieldValue {
    fn from(value: i32) -> Self {
        FieldValue::Integer(value)
    }
}

impl From<i16> for FieldValue {
    fn from(value: i16) -> Self {
        FieldValue::Short(value)
    }
}

impl From<f64> for FieldValue {
    fn from(value: f64) -> Self {
        FieldValue::Double(value)
    }
}

impl From<bool> for FieldValue {
    fn from(value: bool) -> Self {
        FieldValue::Boolean(value)
    }
}

impl From<String> for FieldValue {
    fn from(value: String) -> Self {
        FieldValue::String(value)
    }
}

impl From<&str> for FieldValue {
    fn from(value: &str) -> Self {
        FieldValue::String(value.to_owned())
    }
}

impl From<NaiveDate> for FieldValue {
    fn from(value: NaiveDate) -> Self {
        FieldValue::Date(value)
    }
}

impl From<NaiveTime> for FieldValue {
    fn from(value: NaiveTime) -> Self {
        FieldValue::Time(value)
    }
}

impl From<NaiveDateTime> for FieldValue {
    fn from(value: NaiveDateTime) -> Self {
        FieldValue::DateTime(value)
    }
}

impl From<Duration> for FieldValue {
    fn from(value: Duration) -> Self {
        FieldValue::Duration(value)
    }
}

impl From<Rate> for FieldValue {
    fn from(value: Rate) -> Self {
        FieldValue::Rate(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_unit::TimeUnit;

    #[test]
    fn numeric_accessors_widen() {
        assert_eq!(FieldValue::Integer(3).as_double(), Some(3.0));
        assert_eq!(FieldValue::Short(3).as_integer(), Some(3));
        assert_eq!(FieldValue::Double(3.5).as_integer(), None);
    }

    #[test]
    fn date_time_accessors_cross_convert() {
        let date = NaiveDate::from_ymd_opt(2024, 5, 1).unwrap();
        let value = FieldValue::Date(date);
        assert_eq!(value.as_date(), Some(date));
        assert_eq!(
            value.as_date_time(),
            Some(date.and_hms_opt(0, 0, 0).unwrap())
        );
    }

    #[test]
    fn duration_is_carried_through() {
        let duration = Duration::get_instance(2.0, TimeUnit::Days);
        assert_eq!(
            FieldValue::Duration(duration).as_duration(),
            Some(duration)
        );
        assert_eq!(FieldValue::Duration(duration).type_name(), "Duration");
    }
}
