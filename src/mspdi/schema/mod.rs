//! Corresponds to Java: `org/mpxj/mspdi/schema` package
//!
//! The XML binding model: a data-only description of the MSPDI document, mirroring the
//! JAXB-generated classes in `Project.java` one field at a time. It carries no behaviour other than
//! [`crate::mspdi::schema::Project::from_node`], which replaces the JAXB unmarshaller.
//!
//! This layer exists because the Java reader is a two-stage pipeline: `MSPDIReader.read` first
//! unmarshals the whole document into this model, then copies values from it into the
//! [`crate::ProjectFile`] domain objects. The Rust port keeps the same split, so the ported
//! `readXxx` methods can be compared against Java method by method. (It plays the same role as the
//! `Props*` / `FixedData` / `VarData` block model that the MPP side of this crate preserves.)
//!
//! # Java class → Rust path
//!
//! | Java (declaration line in `Project.java`) | Rust |
//! |---|---|
//! | `Project` (2235) | [`project::Project`] |
//! | `Project.WBSMasks` (27662) / `WBSMask` (27816) | [`wbs_masks::WBSMasks`] / [`wbs_masks::WBSMask`] |
//! | `Project.ExtendedAttributes` (16508) / `ExtendedAttribute` (16693) | [`extended_attributes::ExtendedAttributes`] / [`extended_attributes::ExtendedAttribute`] |
//! | `ExtendedAttribute.ValueList` (17326) / `Value` (17386) | [`extended_attributes::ValueList`] / [`extended_attributes::Value`] |
//! | `Project.OutlineCodes` (17621) / `OutlineCode` (17775) | [`outline_codes::OutlineCodes`] / [`outline_codes::OutlineCode`] |
//! | `OutlineCode.Masks` (18201) / `Mask` (18274) | [`outline_codes::Masks`] / [`outline_codes::Mask`] |
//! | `OutlineCode.Values` (18440) / `Value` (18515) | [`outline_codes::Values`] / [`outline_codes::Value`] |
//! | `Project.Calendars` (13443) / `Calendar` (13818) | [`calendars::Calendars`] / [`calendars::Calendar`] |
//! | `Calendar.Exceptions.Exception` (14383), `Exception.TimePeriod` (14762), `Exception.WorkingTimes.WorkingTime` (14917) | [`calendars::exceptions::Exception`], [`calendars::exceptions::TimePeriod`], [`calendars::exceptions::working_times::WorkingTime`] |
//! | `Calendar.WeekDays.WeekDay` (15176), `WeekDay.TimePeriod` (15312), `WeekDay.WorkingTimes.WorkingTime` (15467) | [`calendars::week_days::WeekDay`], [`calendars::week_days::TimePeriod`], [`calendars::week_days::working_times::WorkingTime`] |
//! | `Calendar.WorkWeeks.WorkWeek` (15779), `WorkWeek.TimePeriod` (15888), `WorkWeek.WeekDays.WeekDay` (16116), `WeekDay.WorkingTimes.WorkingTime` (16296) | [`calendars::work_weeks::WorkWeek`], [`calendars::work_weeks::TimePeriod`], [`calendars::work_weeks::week_days::WeekDay`], [`calendars::work_weeks::week_days::working_times::WorkingTime`] |
//! | `Project.Tasks.Task` (23666) | [`tasks::task::Task`] |
//! | `Task.Baseline` (26715) / `ExtendedAttribute` (27136) / `OutlineCode` (27276) / `PredecessorLink` (27434) | [`tasks::task::baseline::Baseline`] / [`tasks::task::extended_attribute::ExtendedAttribute`] / [`tasks::task::outline_code::OutlineCode`] / [`tasks::task::PredecessorLink`] |
//! | `Project.Resources.Resource` (19578) | [`resources::resource::Resource`] |
//! | `Resource.AvailabilityPeriods.AvailabilityPeriod` (21669) / `Baseline` (21788) / `ExtendedAttribute` (21988) / `OutlineCode` (22128) / `Rates.Rate` (22390) | [`resources::resource::availability_periods::AvailabilityPeriod`] / [`resources::resource::baseline::Baseline`] / [`resources::resource::extended_attribute::ExtendedAttribute`] / [`resources::resource::outline_code::OutlineCode`] / [`resources::resource::rates::Rate`] |
//! | `Project.Assignments.Assignment` (5434) | [`assignments::assignment::Assignment`] |
//! | `Assignment.Baseline` (12698) / `ExtendedAttribute` (12984) | [`assignments::assignment::baseline::Baseline`] / [`assignments::assignment::extended_attribute::ExtendedAttribute`] |
//! | `TimephasedDataType` (own file) | [`timephased_data_type::TimephasedDataType`] |
//! | `ObjectFactory`, `Adapter1`..`Adapter33`, `package-info` | not ported, see below |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | JAXB-generated classes with `@XmlAccessorType(FIELD)` / `@XmlElement` / `@XmlJavaTypeAdapter` annotations, populated by the JAXB unmarshaller | plain structs with `pub` fields plus a Rust-specific `Project::from_node` per type | Rust has no JAXB. The field names, order and declared types mirror the Java declarations; the annotations' effect (element name → field, adapter → conversion) moves into `from_node` |
//! | Fields are `protected`, exposed through generated getters/setters | `pub` fields | The generated accessors carry no behaviour; the domain layer is the only consumer |
//! | `ObjectFactory`, `Adapter1`..`Adapter33`, `package-info.java` | not ported | `Adapter*` are pure string→typed conversions and live in [`crate::mspdi::datatype_converter`]; `ObjectFactory` only exists so JAXB can create instances; `package-info.java` declares the XML namespace, which [`crate::mspdi::xml_node`] matches by local name instead |
//! | Java nested classes (`Project.Tasks.Task`) | Rust nested modules | Rust has no nested types; the module path mirrors the Java nesting so that the repeated names (`Baseline`, `ExtendedAttribute`, `OutlineCode`, `Value`, `TimePeriod`, `WorkingTime`, `WeekDay`) stay distinct |
//! | JAXB element wrapper classes (`Project.Calendars`, `Project.Tasks`, `Calendar.Exceptions`, `WorkingTimes`, ...) | mostly flattened into a `Vec` field on the owner | A wrapper exists only to hold an element list; each file's header table records the flattening and notes that Java raises an `NullPointerException` where Rust yields an empty `Vec` |
//! | JAXB `defaultValue` in the schema | generally not reproduced | See the note in [`resources::resource::Resource::max_units`] |
//!
//! # Skipped Java members + reason
//!
//! - `ObjectFactory` (`@XmlRegistry`): it only exists so that JAXB can instantiate the model; the
//!   Rust side constructs the structs directly in [`project::Project::from_node`].
//! - `Adapter1`..`Adapter33`: pure `String` → typed conversions, moved to
//!   [`crate::mspdi::datatype_converter`] (the `DatatypeConverter.parseXxx` methods the adapters
//!   wrapped are the ones that were ported).
//! - `package-info.java`: declares the XML namespace, which [`crate::mspdi::xml_node`] handles by
//!   matching local names.
//!
//! Every field of every class in `Project.java` and `TimephasedDataType.java` is present in the
//! corresponding struct; the per-file headers record the field-level differences (types, wrapper
//! flattening, keyword escaping).

pub mod assignments;
pub mod calendars;
pub mod extended_attributes;
pub mod outline_codes;
pub mod project;
pub mod resources;
pub mod tasks;
pub mod timephased_data_type;
pub mod wbs_masks;
