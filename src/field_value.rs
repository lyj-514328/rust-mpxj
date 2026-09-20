//! 对应 Java 中 `AbstractFieldContainer` 存储的 `Object` 字段值。
//!
//! Java 用 `Map<FieldType, Object>` 保存字段，值的实际类型由字段的
//! `DataType` 决定。Rust 侧需要一个具体枚举来承载这些值，因此引入
//! `FieldValue`。
//!
//! # 存在的变体与 Java 依据
//!
//! Java 的字段表用 `Object` 承载任意值，除标量外还会存放实体引用与集合。
//! 本枚举按 Java 实际的字段表用法逐项对应：
//!
//! | 变体 | Java 侧类型 | 依据（字段 / 方法） |
//! |---|---|---|
//! | 标量变体 | `Integer` / `Short` / `Double` / `Boolean` / `String` / `LocalDate` / `LocalTime` / `LocalDateTime` | 各 `get`/`set` 的既有用法 |
//! | [`FieldValue::Bytes`] | `byte[]` | OLE 文档的二进制自定义属性（如 `_PID_LINKBASE`） |
//! | [`FieldValue::Duration`] / [`FieldValue::Rate`] | `org.mpxj.Duration` / `org.mpxj.Rate` | 同上 |
//! | [`FieldValue::Uuid`] | `java.util.UUID` | `AssignmentField.GUID`、`TaskField.GUID` |
//! | [`FieldValue::TimeUnit`] | `org.mpxj.TimeUnit` | `AssignmentField.VARIABLE_RATE_UNITS` |
//! | [`FieldValue::WorkContour`] | `org.mpxj.WorkContour` | `AssignmentField.WORK_CONTOUR` |
//! | [`FieldValue::RateSource`] | `org.mpxj.RateSource` | `AssignmentField.RATE_SOURCE` |
//! | [`FieldValue::ResourceRequestType`] | `org.mpxj.ResourceRequestType` | `AssignmentField.RESOURCE_REQUEST_TYPE` |
//! | [`FieldValue::Notes`] | `org.mpxj.Notes`（含 `RtfNotes` / `HtmlNotes` / `StructuredNotes` / `ParentNotes`） | `TaskField.NOTES`、`AssignmentField.NOTES` |
//! | [`FieldValue::Steps`] | `List<Step>` | `TaskField.STEPS` |
//! | [`FieldValue::Relations`] | `List<Relation>` | `TaskField.PREDECESSORS` / `TaskField.SUCCESSORS` |
//! | [`FieldValue::Resource`] | `org.mpxj.Resource` | 字段表用 `Object`，可承载实体引用（`AssignmentField` 中并无 `RESOURCE` 字段：Java 的 `ResourceAssignment.getResource()` 是经父文件按 unique ID 查找；本变体保留给同类实体引用的字段值） |
//! | [`FieldValue::TimephasedWork`] / [`FieldValue::TimephasedCost`] | `List<TimephasedWork>` / `List<TimephasedCost>` | `AssignmentField.RAW_TIMEPHASED_*` |
//! | [`FieldValue::TimeRanges`] | `List<LocalDateTimeRange>` | `TaskField.WORK_SPLITS` |
//!
//! # `Clone` / `PartialEq` 语义（不可派生）
//!
//! Java 的 `AbstractFieldContainer.set` 判定「值是否变化」用的是
//! `oldValue == value` 与 `oldValue.equals(value)`（见该类的第 112-122 行）：
//! 基本类型与 `UUID` / `List` 走**值**比较，而 `Notes` / `WorkContour` / `Relation`
//! **未覆写 `equals`**，实际是**引用**比较。因此本枚举的 `PartialEq` 与 `Clone`
//! 均为手写实现，逐变体按 Java 的 `equals` 语义选择：
//!
//! * 标量、`UUID`、`TimeUnit`、`RateSource`、`ResourceRequestType`：值比较
//!   （Java 的枚举是单例，`equals` 等价于值比较）。
//! * `Bytes`：按字节值比较。Java 的 `byte[]` 使用引用比较；Rust 按值比较会使
//!   相同内容的重新赋值不触发字段事件，属于 Rust 特有的差异。
//! * `TimephasedWork` / `TimephasedCost` / `LocalDateTimeRange`：值比较 ——
//!   Java 的 `TimephasedItem` 覆写了 `equals`，`List.equals` 因此是逐元素值比较。
//! * `Notes` / `Resource` / `Steps`：`Rc::ptr_eq`（Java 侧 `Notes` 未覆写 `equals`，
//!   按引用比较；`Resource` 的 `equals` 按 ID，但字段表里存的是同一实体的共享引用，
//!   引用相等即等价）。
//! * `WorkContour`：按「唯一 ID + 名称」比较。Java 的 `WorkContour` 未覆写
//!   `equals`（按引用比较），而 Rust 侧存的是值，同一次读取链路中的同一实例
//!   必然具有相同的唯一 ID 与名称。
//! * `Relations`：逐元素 `Rc::ptr_eq`（Java 的 `List.equals` + `Relation` 的引用比较）。
//!
//! 判定点只有 [`crate::abstract_field_container::AbstractFieldContainer::update`]
//! （以及各实体在写字段前的 `old_value == value` 比较），它们都调用本枚举的
//! `PartialEq`，因此上述语义即为「值变化判定」的语义。

use std::cell::RefCell;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use uuid::Uuid;

use crate::duration::Duration;
use crate::local_date_time_range::LocalDateTimeRange;
use crate::notes::Notes;
use crate::rate::Rate;
use crate::rate_source::RateSource;
use crate::relation::Relation;
use crate::resource::Resource;
use crate::resource_request_type::ResourceRequestType;
use crate::step::Step;
use crate::time_unit::TimeUnit;
use crate::timephased_cost::TimephasedCost;
use crate::timephased_work::TimephasedWork;
use crate::work_contour::WorkContour;

/// 字段值，对应 Java 存放在 `Map<FieldType, Object>` 里的 `Object`。
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
    /// 对应 Java `byte[]`（OLE 自定义文档属性）。
    Bytes(Vec<u8>),
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
    /// 对应 Java `java.util.UUID`
    Uuid(Uuid),
    /// 对应 Java `org.mpxj.TimeUnit`
    TimeUnit(TimeUnit),
    /// 对应 Java `org.mpxj.WorkContour`
    WorkContour(WorkContour),
    /// 对应 Java `org.mpxj.RateSource`
    RateSource(RateSource),
    /// 对应 Java `org.mpxj.ResourceRequestType`
    ResourceRequestType(ResourceRequestType),
    /// 对应 Java `org.mpxj.Notes`（`RtfNotes` / `HtmlNotes` / `StructuredNotes` 等子类型经
    /// trait 对象保真）。
    Notes(Rc<dyn Notes>),
    /// 对应 Java `List<Step>`（可变列表，与 Java 的共享语义一致）。
    Steps(Rc<RefCell<Vec<Step>>>),
    /// 对应 Java `List<Relation>`。
    Relations(Vec<Rc<RefCell<Relation>>>),
    /// 对应 Java `org.mpxj.Resource` 实体引用。
    Resource(Rc<RefCell<Resource>>),
    /// 对应 Java `List<TimephasedWork>`。
    TimephasedWork(Vec<TimephasedWork>),
    /// 对应 Java `List<TimephasedCost>`。
    TimephasedCost(Vec<TimephasedCost>),
    /// 对应 Java `List<LocalDateTimeRange>`。
    TimeRanges(Vec<LocalDateTimeRange>),
}

impl Clone for FieldValue {
    /// Java 的字段表按引用共享对象；Rust 侧的 `Clone` 对实体引用型变体做
    /// `Rc::clone`（共享），对其余变体做值拷贝 —— 语义见文件头。
    fn clone(&self) -> Self {
        match self {
            FieldValue::Integer(value) => FieldValue::Integer(*value),
            FieldValue::Short(value) => FieldValue::Short(*value),
            FieldValue::Double(value) => FieldValue::Double(*value),
            FieldValue::Boolean(value) => FieldValue::Boolean(*value),
            FieldValue::String(value) => FieldValue::String(value.clone()),
            FieldValue::Bytes(value) => FieldValue::Bytes(value.clone()),
            FieldValue::Date(value) => FieldValue::Date(*value),
            FieldValue::Time(value) => FieldValue::Time(*value),
            FieldValue::DateTime(value) => FieldValue::DateTime(*value),
            FieldValue::Duration(value) => FieldValue::Duration(*value),
            FieldValue::Rate(value) => FieldValue::Rate(*value),
            FieldValue::Uuid(value) => FieldValue::Uuid(*value),
            FieldValue::TimeUnit(value) => FieldValue::TimeUnit(*value),
            FieldValue::WorkContour(value) => FieldValue::WorkContour(value.clone()),
            FieldValue::RateSource(value) => FieldValue::RateSource(*value),
            FieldValue::ResourceRequestType(value) => FieldValue::ResourceRequestType(*value),
            FieldValue::Notes(value) => FieldValue::Notes(Rc::clone(value)),
            FieldValue::Steps(value) => FieldValue::Steps(Rc::clone(value)),
            FieldValue::Relations(value) => FieldValue::Relations(value.clone()),
            FieldValue::Resource(value) => FieldValue::Resource(Rc::clone(value)),
            FieldValue::TimephasedWork(value) => FieldValue::TimephasedWork(value.clone()),
            FieldValue::TimephasedCost(value) => FieldValue::TimephasedCost(value.clone()),
            FieldValue::TimeRanges(value) => FieldValue::TimeRanges(value.clone()),
        }
    }
}

impl PartialEq for FieldValue {
    /// 逐变体按 Java `equals` 的语义比较，见文件头「`Clone` / `PartialEq` 语义」。
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (FieldValue::Integer(a), FieldValue::Integer(b)) => a == b,
            (FieldValue::Short(a), FieldValue::Short(b)) => a == b,
            (FieldValue::Double(a), FieldValue::Double(b)) => a == b,
            (FieldValue::Boolean(a), FieldValue::Boolean(b)) => a == b,
            (FieldValue::String(a), FieldValue::String(b)) => a == b,
            (FieldValue::Bytes(a), FieldValue::Bytes(b)) => a == b,
            (FieldValue::Date(a), FieldValue::Date(b)) => a == b,
            (FieldValue::Time(a), FieldValue::Time(b)) => a == b,
            (FieldValue::DateTime(a), FieldValue::DateTime(b)) => a == b,
            (FieldValue::Duration(a), FieldValue::Duration(b)) => a == b,
            (FieldValue::Rate(a), FieldValue::Rate(b)) => a == b,
            (FieldValue::Uuid(a), FieldValue::Uuid(b)) => a == b,
            (FieldValue::TimeUnit(a), FieldValue::TimeUnit(b)) => a == b,
            (FieldValue::RateSource(a), FieldValue::RateSource(b)) => a == b,
            (FieldValue::ResourceRequestType(a), FieldValue::ResourceRequestType(b)) => a == b,
            (FieldValue::WorkContour(a), FieldValue::WorkContour(b)) => {
                a.get_unique_id() == b.get_unique_id() && a.get_name() == b.get_name()
            }
            (FieldValue::Notes(a), FieldValue::Notes(b)) => Rc::ptr_eq(a, b),
            (FieldValue::Steps(a), FieldValue::Steps(b)) => Rc::ptr_eq(a, b),
            (FieldValue::Resource(a), FieldValue::Resource(b)) => Rc::ptr_eq(a, b),
            (FieldValue::Relations(a), FieldValue::Relations(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| Rc::ptr_eq(a, b))
            }
            (FieldValue::TimephasedWork(a), FieldValue::TimephasedWork(b)) => a == b,
            (FieldValue::TimephasedCost(a), FieldValue::TimephasedCost(b)) => a == b,
            (FieldValue::TimeRanges(a), FieldValue::TimeRanges(b)) => a == b,
            _ => false,
        }
    }
}

impl Debug for FieldValue {
    /// `Rc<dyn Notes>` / `Relation` 等引用型变体未实现 `Debug`，故手写实现：
    /// 笔记输出其纯文本（等价 Java 的 `toString()`），实体引用输出其地址。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            FieldValue::Integer(value) => f.debug_tuple("Integer").field(value).finish(),
            FieldValue::Short(value) => f.debug_tuple("Short").field(value).finish(),
            FieldValue::Double(value) => f.debug_tuple("Double").field(value).finish(),
            FieldValue::Boolean(value) => f.debug_tuple("Boolean").field(value).finish(),
            FieldValue::String(value) => f.debug_tuple("String").field(value).finish(),
            FieldValue::Bytes(value) => f.debug_tuple("Bytes").field(value).finish(),
            FieldValue::Date(value) => f.debug_tuple("Date").field(value).finish(),
            FieldValue::Time(value) => f.debug_tuple("Time").field(value).finish(),
            FieldValue::DateTime(value) => f.debug_tuple("DateTime").field(value).finish(),
            FieldValue::Duration(value) => f.debug_tuple("Duration").field(value).finish(),
            FieldValue::Rate(value) => f.debug_tuple("Rate").field(value).finish(),
            FieldValue::Uuid(value) => f.debug_tuple("Uuid").field(value).finish(),
            FieldValue::TimeUnit(value) => f.debug_tuple("TimeUnit").field(value).finish(),
            FieldValue::WorkContour(value) => f.debug_tuple("WorkContour").field(value).finish(),
            FieldValue::RateSource(value) => f.debug_tuple("RateSource").field(value).finish(),
            FieldValue::ResourceRequestType(value) => {
                f.debug_tuple("ResourceRequestType").field(value).finish()
            }
            FieldValue::Notes(value) => f
                .debug_tuple("Notes")
                .field(&value.to_string())
                .finish(),
            FieldValue::Steps(value) => f
                .debug_tuple("Steps")
                .field(&value.borrow().len())
                .finish(),
            FieldValue::Relations(value) => f
                .debug_tuple("Relations")
                .field(&value.len())
                .finish(),
            FieldValue::Resource(value) => f
                .debug_tuple("Resource")
                .field(&(Rc::as_ptr(value) as *const u8))
                .finish(),
            FieldValue::TimephasedWork(value) => {
                f.debug_tuple("TimephasedWork").field(&value.len()).finish()
            }
            FieldValue::TimephasedCost(value) => {
                f.debug_tuple("TimephasedCost").field(&value.len()).finish()
            }
            FieldValue::TimeRanges(value) => f
                .debug_tuple("TimeRanges")
                .field(&value.len())
                .finish(),
        }
    }
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

    /// 取 GUID，类型不匹配时返回 `None`。
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            FieldValue::Uuid(value) => Some(*value),
            _ => None,
        }
    }

    /// 取时间单位，类型不匹配时返回 `None`。
    pub fn as_time_unit(&self) -> Option<TimeUnit> {
        match self {
            FieldValue::TimeUnit(value) => Some(*value),
            _ => None,
        }
    }

    /// 取工时分布，类型不匹配时返回 `None`。
    pub fn as_work_contour(&self) -> Option<&WorkContour> {
        match self {
            FieldValue::WorkContour(value) => Some(value),
            _ => None,
        }
    }

    /// 取费率来源，类型不匹配时返回 `None`。
    pub fn as_rate_source(&self) -> Option<RateSource> {
        match self {
            FieldValue::RateSource(value) => Some(*value),
            _ => None,
        }
    }

    /// 取资源请求类型，类型不匹配时返回 `None`。
    pub fn as_resource_request_type(&self) -> Option<ResourceRequestType> {
        match self {
            FieldValue::ResourceRequestType(value) => Some(*value),
            _ => None,
        }
    }

    /// 取笔记对象（共享句柄），类型不匹配时返回 `None`。
    pub fn as_notes(&self) -> Option<&Rc<dyn Notes>> {
        match self {
            FieldValue::Notes(value) => Some(value),
            _ => None,
        }
    }

    /// 取步骤列表（共享列表），类型不匹配时返回 `None`。
    pub fn as_steps(&self) -> Option<&Rc<RefCell<Vec<Step>>>> {
        match self {
            FieldValue::Steps(value) => Some(value),
            _ => None,
        }
    }

    /// 取关系列表，类型不匹配时返回 `None`。
    pub fn as_relations(&self) -> Option<&[Rc<RefCell<Relation>>]> {
        match self {
            FieldValue::Relations(value) => Some(value),
            _ => None,
        }
    }

    /// 取资源实体引用，类型不匹配时返回 `None`。
    pub fn as_resource(&self) -> Option<&Rc<RefCell<Resource>>> {
        match self {
            FieldValue::Resource(value) => Some(value),
            _ => None,
        }
    }

    /// 取 timephased 工时列表，类型不匹配时返回 `None`。
    pub fn as_timephased_work(&self) -> Option<&[TimephasedWork]> {
        match self {
            FieldValue::TimephasedWork(value) => Some(value),
            _ => None,
        }
    }

    /// 取 timephased 成本列表，类型不匹配时返回 `None`。
    pub fn as_timephased_cost(&self) -> Option<&[TimephasedCost]> {
        match self {
            FieldValue::TimephasedCost(value) => Some(value),
            _ => None,
        }
    }

    /// 取时间段列表，类型不匹配时返回 `None`。
    pub fn as_time_ranges(&self) -> Option<&[LocalDateTimeRange]> {
        match self {
            FieldValue::TimeRanges(value) => Some(value),
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
            FieldValue::Bytes(_) => "byte[]",
            FieldValue::Date(_) => "LocalDate",
            FieldValue::Time(_) => "LocalTime",
            FieldValue::DateTime(_) => "LocalDateTime",
            FieldValue::Duration(_) => "Duration",
            FieldValue::Rate(_) => "Rate",
            FieldValue::Uuid(_) => "UUID",
            FieldValue::TimeUnit(_) => "TimeUnit",
            FieldValue::WorkContour(_) => "WorkContour",
            FieldValue::RateSource(_) => "RateSource",
            FieldValue::ResourceRequestType(_) => "ResourceRequestType",
            FieldValue::Notes(_) => "Notes",
            FieldValue::Steps(_) => "List<Step>",
            FieldValue::Relations(_) => "List<Relation>",
            FieldValue::Resource(_) => "Resource",
            FieldValue::TimephasedWork(_) => "List<TimephasedWork>",
            FieldValue::TimephasedCost(_) => "List<TimephasedCost>",
            FieldValue::TimeRanges(_) => "List<LocalDateTimeRange>",
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
    use crate::notes::PlainNotes;
    use crate::rtf_notes::RtfNotes;
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

    #[test]
    fn scalar_variants_compare_by_value() {
        assert_eq!(FieldValue::Uuid(Uuid::from_u128(7)), FieldValue::Uuid(Uuid::from_u128(7)));
        assert_eq!(
            FieldValue::TimeUnit(TimeUnit::Hours),
            FieldValue::TimeUnit(TimeUnit::Hours)
        );
        assert_ne!(
            FieldValue::RateSource(RateSource::Resource),
            FieldValue::RateSource(RateSource::Override)
        );
        assert_eq!(
            FieldValue::ResourceRequestType(ResourceRequestType::None),
            FieldValue::ResourceRequestType(ResourceRequestType::None)
        );
    }

    #[test]
    fn notes_variant_preserves_subtype_and_compares_by_reference() {
        let rtf: Rc<dyn Notes> = Rc::new(RtfNotes::new(Some("{\\rtf1 hello}".to_owned())));
        let value = FieldValue::Notes(Rc::clone(&rtf));

        // 子类型信息未丢失：仍是 RtfNotes 的文本表示
        assert_eq!(
            value.as_notes().map(|notes| notes.to_string()),
            Some("hello".to_owned())
        );
        // Java 的 Notes 未覆写 equals ⇒ 引用比较
        assert_eq!(value, FieldValue::Notes(Rc::clone(&rtf)));
        assert_ne!(
            value,
            FieldValue::Notes(Rc::new(PlainNotes::new(Some("hello".to_owned()))))
        );
    }

    #[test]
    fn clone_shares_references() {
        let notes: Rc<dyn Notes> = Rc::new(PlainNotes::new(Some("text".to_owned())));
        let value = FieldValue::Notes(Rc::clone(&notes));
        assert_eq!(value.clone(), value);
        assert_eq!(value.type_name(), "Notes");
    }

    #[test]
    fn work_contour_compares_by_identity_fields() {
        let first = FieldValue::WorkContour(WorkContour::flat());
        let second = FieldValue::WorkContour(WorkContour::flat());
        assert_eq!(first, second);
        assert_eq!(first.type_name(), "WorkContour");
    }

    #[test]
    fn debug_renders_reference_variants_without_panicking() {
        let notes: Rc<dyn Notes> = Rc::new(PlainNotes::new(Some("text".to_owned())));
        assert!(format!("{:?}", FieldValue::Notes(notes)).contains("text"));
        assert!(format!("{:?}", FieldValue::TimephasedWork(Vec::new())).contains("TimephasedWork"));
    }
}
