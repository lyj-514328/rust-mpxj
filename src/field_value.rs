//! Corresponds to the `Object` field values stored by `AbstractFieldContainer` in Java.
//!
//! Java stores fields in a `Map<FieldType, Object>`, and the actual type of a value is determined by the field's
//! `DataType`. The Rust side needs a concrete enum to carry these values, hence
//! `FieldValue`.
//!
//! # Existing variants and their Java basis
//!
//! Java's field table uses `Object` to carry arbitrary values; besides scalars it also stores entity references and collections.
//! This enum maps item by item to Java's actual field table usage:
//!
//! | Variant | Java-side type | Basis (field / method) |
//! |---|---|---|
//! | Scalar variants | `Integer` / `Short` / `Double` / `Boolean` / `String` / `LocalDate` / `LocalTime` / `LocalDateTime` | existing usage of the various `get`/`set` methods |
//! | [`FieldValue::Bytes`] | `byte[]` | binary custom properties of OLE documents (e.g. `_PID_LINKBASE`) |
//! | [`FieldValue::Duration`] / [`FieldValue::Rate`] | `org.mpxj.Duration` / `org.mpxj.Rate` | as above |
//! | [`FieldValue::Uuid`] | `java.util.UUID` | `AssignmentField.GUID`,`TaskField.GUID` |
//! | [`FieldValue::TimeUnit`] | `org.mpxj.TimeUnit` | `AssignmentField.VARIABLE_RATE_UNITS` |
//! | [`FieldValue::WorkContour`] | `org.mpxj.WorkContour` | `AssignmentField.WORK_CONTOUR` |
//! | [`FieldValue::RateSource`] | `org.mpxj.RateSource` | `AssignmentField.RATE_SOURCE` |
//! | [`FieldValue::ResourceRequestType`] | `org.mpxj.ResourceRequestType` | `AssignmentField.RESOURCE_REQUEST_TYPE` |
//! | [`FieldValue::Notes`] | `org.mpxj.Notes` (including `RtfNotes` / `HtmlNotes` / `StructuredNotes` / `ParentNotes`) | `TaskField.NOTES`, `AssignmentField.NOTES` |
//! | [`FieldValue::Steps`] | `List<Step>` | `TaskField.STEPS` |
//! | [`FieldValue::Relations`] | `List<Relation>` | `TaskField.PREDECESSORS` / `TaskField.SUCCESSORS` |
//! | [`FieldValue::Resource`] | `org.mpxj.Resource` | the field table uses `Object`, which can carry entity references (there is no `RESOURCE` field in `AssignmentField`: Java's `ResourceAssignment.getResource()` looks it up by unique ID through the parent file; this variant is kept for field values of the same kind of entity reference) |
//! | [`FieldValue::TimephasedWork`] / [`FieldValue::TimephasedCost`] | `List<TimephasedWork>` / `List<TimephasedCost>` | `AssignmentField.RAW_TIMEPHASED_*` |
//! | [`FieldValue::TimeRanges`] | `List<LocalDateTimeRange>` | `TaskField.WORK_SPLITS` |
//!
//! # `Clone` / `PartialEq` semantics (cannot be derived)
//!
//! Java's `AbstractFieldContainer.set` determines "whether the value changed" using
//! `oldValue == value` and `oldValue.equals(value)` (see lines 112-122 of that class):
//! primitives and `UUID` / `List` use **value** comparison, whereas `Notes` / `WorkContour` / `Relation`
//! **do not override `equals`**, so it is actually **reference** comparison. Therefore this enum's `PartialEq` and `Clone`
//! are both hand-written, choosing per variant according to Java's `equals` semantics:
//!
//! * scalars, `UUID`, `TimeUnit`, `RateSource`, `ResourceRequestType`: value comparison
//! (Java enums are singletons, so `equals` is equivalent to value comparison).
//! * `Bytes`: compared by byte value. Java's `byte[]` uses reference comparison; comparing by value in Rust makes
//! reassigning identical content not trigger a field event, which is a Rust-specific difference.
//! * `TimephasedWork` / `TimephasedCost` / `LocalDateTimeRange`: value comparison —
//! Java's `TimephasedItem` overrides `equals`, so `List.equals` is an element-by-element value comparison.
//! * `Notes` / `Resource` / `Steps`: `Rc::ptr_eq` (on the Java side `Notes` does not override `equals`,
//! so it compares by reference; `Resource`'s `equals` is by ID, but the field table stores a shared reference to the same entity,
//! so reference equality is equivalent).
//! * `WorkContour`: compared by "unique ID + name". Java's `WorkContour` does not override
//! `equals` (compares by reference), whereas the Rust side stores a value, and the same instance within one read chain
//! necessarily has the same unique ID and name.
//! * `Relations`: element-by-element `Rc::ptr_eq` (Java's `List.equals` + `Relation`'s reference comparison).
//!
//! The only decision points are [`crate::abstract_field_container::AbstractFieldContainer::update`]
//! (and each entity's `old_value == value` comparison before writing a field); they all call this enum's
//! `PartialEq`, so the above semantics are the semantics of "value change detection".

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

/// A field value, corresponding to the `Object` stored in Java's `Map<FieldType, Object>`.
pub enum FieldValue {
/// Corresponds to Java `Integer` / `int`
    Integer(i32),
/// Corresponds to Java `Short` / `short`
    Short(i16),
/// Corresponds to Java `Double` / `double`
    Double(f64),
/// Corresponds to Java `Boolean` / `boolean`
    Boolean(bool),
/// Corresponds to Java `String`
    String(String),
/// Corresponds to Java `byte[]` (OLE custom document properties).
    Bytes(Vec<u8>),
/// Corresponds to Java `java.time.LocalDate`
    Date(NaiveDate),
/// Corresponds to Java `java.time.LocalTime`
    Time(NaiveTime),
/// Corresponds to Java `java.time.LocalDateTime`
    DateTime(NaiveDateTime),
/// Corresponds to Java `org.mpxj.Duration`
    Duration(Duration),
/// Corresponds to Java `org.mpxj.Rate`
    Rate(Rate),
/// Corresponds to Java `java.util.UUID`
    Uuid(Uuid),
/// Corresponds to Java `org.mpxj.TimeUnit`
    TimeUnit(TimeUnit),
/// Corresponds to Java `org.mpxj.WorkContour`
    WorkContour(WorkContour),
/// Corresponds to Java `org.mpxj.RateSource`
    RateSource(RateSource),
/// Corresponds to Java `org.mpxj.ResourceRequestType`
    ResourceRequestType(ResourceRequestType),
/// Corresponds to Java `org.mpxj.Notes` (subtypes such as `RtfNotes` / `HtmlNotes` / `StructuredNotes` are preserved
/// faithfully via a trait object).
    Notes(Rc<dyn Notes>),
/// Corresponds to Java `List<Step>` (a mutable list, consistent with Java's sharing semantics).
    Steps(Rc<RefCell<Vec<Step>>>),
/// Corresponds to Java `List<Relation>`.
    Relations(Vec<Rc<RefCell<Relation>>>),
/// Corresponds to a Java `org.mpxj.Resource` entity reference.
    Resource(Rc<RefCell<Resource>>),
/// Corresponds to Java `List<TimephasedWork>`.
    TimephasedWork(Vec<TimephasedWork>),
/// Corresponds to Java `List<TimephasedCost>`.
    TimephasedCost(Vec<TimephasedCost>),
/// Corresponds to Java `List<LocalDateTimeRange>`.
    TimeRanges(Vec<LocalDateTimeRange>),
}

impl Clone for FieldValue {
/// Java's field table shares objects by reference; the Rust side's `Clone` performs
/// `Rc::clone` (sharing) for entity-reference variants and a value copy for the remaining variants — see the file header for the semantics.
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
/// Compares per variant according to Java `equals` semantics; see "`Clone` / `PartialEq` semantics" in the file header.
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
/// Reference-type variants such as `Rc<dyn Notes>` / `Relation` do not implement `Debug`, so it is hand-written:
/// notes output their plain text (equivalent to Java's `toString()`), and entity references output their address.
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
/// Get the integer; returns `None` when the type does not match.
    pub fn as_integer(&self) -> Option<i32> {
        match self {
            FieldValue::Integer(value) => Some(*value),
            FieldValue::Short(value) => Some(i32::from(*value)),
            _ => None,
        }
    }

/// Get the double-precision float; returns `None` when the type does not match.
    pub fn as_double(&self) -> Option<f64> {
        match self {
            FieldValue::Double(value) => Some(*value),
            FieldValue::Integer(value) => Some(f64::from(*value)),
            FieldValue::Short(value) => Some(f64::from(*value)),
            _ => None,
        }
    }

/// Get the boolean; returns `None` when the type does not match.
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            FieldValue::Boolean(value) => Some(*value),
            _ => None,
        }
    }

/// Get a string reference; returns `None` when the type does not match.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            FieldValue::String(value) => Some(value),
            _ => None,
        }
    }

/// Get the date; returns `None` when the type does not match.
    pub fn as_date(&self) -> Option<NaiveDate> {
        match self {
            FieldValue::Date(value) => Some(*value),
            FieldValue::DateTime(value) => Some(value.date()),
            _ => None,
        }
    }

/// Get the time; returns `None` when the type does not match.
    pub fn as_time(&self) -> Option<NaiveTime> {
        match self {
            FieldValue::Time(value) => Some(*value),
            FieldValue::DateTime(value) => Some(value.time()),
            _ => None,
        }
    }

/// Get the date-time; returns `None` when the type does not match.
    pub fn as_date_time(&self) -> Option<NaiveDateTime> {
        match self {
            FieldValue::DateTime(value) => Some(*value),
            FieldValue::Date(value) => value.and_hms_opt(0, 0, 0),
            _ => None,
        }
    }

/// Get the duration; returns `None` when the type does not match.
    pub fn as_duration(&self) -> Option<Duration> {
        match self {
            FieldValue::Duration(value) => Some(*value),
            _ => None,
        }
    }

/// Get the rate; returns `None` when the type does not match.
    pub fn as_rate(&self) -> Option<Rate> {
        match self {
            FieldValue::Rate(value) => Some(*value),
            _ => None,
        }
    }

/// Get the GUID; returns `None` when the type does not match.
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            FieldValue::Uuid(value) => Some(*value),
            _ => None,
        }
    }

/// Get the time unit; returns `None` when the type does not match.
    pub fn as_time_unit(&self) -> Option<TimeUnit> {
        match self {
            FieldValue::TimeUnit(value) => Some(*value),
            _ => None,
        }
    }

/// Get the work contour; returns `None` when the type does not match.
    pub fn as_work_contour(&self) -> Option<&WorkContour> {
        match self {
            FieldValue::WorkContour(value) => Some(value),
            _ => None,
        }
    }

/// Get the rate source; returns `None` when the type does not match.
    pub fn as_rate_source(&self) -> Option<RateSource> {
        match self {
            FieldValue::RateSource(value) => Some(*value),
            _ => None,
        }
    }

/// Get the resource request type; returns `None` when the type does not match.
    pub fn as_resource_request_type(&self) -> Option<ResourceRequestType> {
        match self {
            FieldValue::ResourceRequestType(value) => Some(*value),
            _ => None,
        }
    }

/// Get the notes object (shared handle); returns `None` when the type does not match.
    pub fn as_notes(&self) -> Option<&Rc<dyn Notes>> {
        match self {
            FieldValue::Notes(value) => Some(value),
            _ => None,
        }
    }

/// Get the step list (shared list); returns `None` when the type does not match.
    pub fn as_steps(&self) -> Option<&Rc<RefCell<Vec<Step>>>> {
        match self {
            FieldValue::Steps(value) => Some(value),
            _ => None,
        }
    }

/// Get the relation list; returns `None` when the type does not match.
    pub fn as_relations(&self) -> Option<&[Rc<RefCell<Relation>>]> {
        match self {
            FieldValue::Relations(value) => Some(value),
            _ => None,
        }
    }

/// Get the resource entity reference; returns `None` when the type does not match.
    pub fn as_resource(&self) -> Option<&Rc<RefCell<Resource>>> {
        match self {
            FieldValue::Resource(value) => Some(value),
            _ => None,
        }
    }

/// Get the timephased work list; returns `None` when the type does not match.
    pub fn as_timephased_work(&self) -> Option<&[TimephasedWork]> {
        match self {
            FieldValue::TimephasedWork(value) => Some(value),
            _ => None,
        }
    }

/// Get the timephased cost list; returns `None` when the type does not match.
    pub fn as_timephased_cost(&self) -> Option<&[TimephasedCost]> {
        match self {
            FieldValue::TimephasedCost(value) => Some(value),
            _ => None,
        }
    }

/// Get the date-time range list; returns `None` when the type does not match.
    pub fn as_time_ranges(&self) -> Option<&[LocalDateTimeRange]> {
        match self {
            FieldValue::TimeRanges(value) => Some(value),
            _ => None,
        }
    }

/// Return the type name corresponding to this value on the Java side, for logging and error messages.
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

// Subtype information is not lost: it is still the text representation of RtfNotes
        assert_eq!(
            value.as_notes().map(|notes| notes.to_string()),
            Some("hello".to_owned())
        );
// Java's Notes does not override equals ⇒ reference comparison
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
