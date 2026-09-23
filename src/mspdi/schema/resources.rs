//! Corresponds to Java: `org/mpxj/mspdi/schema/Project.java` (the `Project.Resources` sub-tree)
//!
//! `org.mpxj.mspdi.MSPDIReader` first unmarshals the MSPDI document into the JAXB generated object
//! tree `org.mpxj.mspdi.schema.Project` and then copies the values into the `org.mpxj` domain
//! objects. This module is the Rust replacement for the JAXB generated classes below
//! `Project.Resources` (the declaration lines 19098 - 22390 of `Project.java`): it reads the same
//! values straight out of a read-only [`roxmltree`] tree with the helpers of
//! [`crate::mspdi::xml_node`], converting each value exactly as the class'
//! `@XmlJavaTypeAdapter` does.
//!
//! Rust has no nested classes, so the Java nesting is reproduced with nested modules: one module
//! level per Java nesting level below `Project.Resources`.
//!
//! # Java name -> Rust path
//!
//! | Java (declaration line) | Rust |
//! |---|---|
//! | `Project.Resources` (19098) | flattened away, there is no Rust type - see the differences table |
//! | `Project.Resources.Resource` (19578) | [`resource::Resource`] |
//! | `Project.Resources.Resource.AvailabilityPeriods` (21607) | flattened away, there is no Rust type - see the differences table |
//! | `Project.Resources.Resource.AvailabilityPeriods.AvailabilityPeriod` (21669) | [`resource::availability_periods::AvailabilityPeriod`] |
//! | `Project.Resources.Resource.Baseline` (21788) | [`resource::baseline::Baseline`] |
//! | `Project.Resources.Resource.ExtendedAttribute` (21988) | [`resource::extended_attribute::ExtendedAttribute`] |
//! | `Project.Resources.Resource.OutlineCode` (22128) | [`resource::outline_code::OutlineCode`] |
//! | `Project.Resources.Resource.Rates` (22286) | flattened away, there is no Rust type - see the differences table |
//! | `Project.Resources.Resource.Rates.Rate` (22390) | [`resource::rates::Rate`] |
//!
//! `Baseline`, `ExtendedAttribute` and `OutlineCode` also exist below `Project.Tasks.Task` and
//! below `Project.Assignments.Assignment`, so the module path is what keeps those classes apart:
//! `schema::resources::resource::baseline::Baseline` is not
//! `schema::tasks::task::baseline::Baseline`.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | The JAXB unmarshaller (`JAXBContext` + `Unmarshaller` + `ObjectFactory`) builds the object tree element by element | every class has one Rust-specific constructor `from_node`, which reads the child elements by local name with [`crate::mspdi::xml_node`] | The field order of every `from_node` matches the Java field declaration order and each value is converted the way the field's `@XmlJavaTypeAdapter` converts it. `xml_node` matches local names only, which is what `NamespaceFilter` achieved on the Java side by rewriting the namespace URI |
//! | `Project.Resources` is a class whose only field is `List<Project.Resources.Resource> resource` | flattened away: the parent `Project` declares `Vec<resource::Resource>` directly | The wrapper class exists only because JAXB needs one class per `@XmlElement`. Because the field lives in the parent module, locating the `Resources` **wrapper element** and collecting its `Resource` children is the parent module's job. Java throws an NPE when `Project.getResources()` is dereferenced while `<Resources>` is absent; Rust yields an empty `Vec` |
//! | `Resource.AvailabilityPeriods` (21607) and `Resource.Rates` (22286) are wrapper classes holding a single `List<X>` field | flattened away: `Resource` declares `Vec<availability_periods::AvailabilityPeriod>` / `Vec<rates::Rate>` directly, plus `rates_present` | The repeated values are flattened, while the presence flag preserves Java's distinction between an absent wrapper (`null`) and an empty `<Rates/>` wrapper |
//! | `getResource()` / `getAvailabilityPeriod()` / `getRate()` create the backing `ArrayList` lazily, so the field stays `null` until the accessor is called | `from_node` always produces an empty `Vec` for an absent repeated element | The Java reader calls the list accessors, which never return `null`, so the observable behaviour is the same |
//! | `Resource.UID` is an `Integer` parsed by `Adapter14` / `DatatypeConverter.parseResourceUID` | `i32` | `Integer.valueOf` throws `NumberFormatException` on malformed text, so unparsable text degrades to `None` |
//! | `BigInteger` fields (`ID`, `StandardRateFormat`, `OvertimeRateFormat`, `CalendarUID`, the `Baseline` / `ExtendedAttribute` / `OutlineCode` / `Rate` integers) | `i64` | Java has arbitrary precision, MSPDI values are small. Text that is not an integer, or that does not fit `i64`, degrades to `None` instead of throwing `NumberFormatException` (hard convention: library code never panics) |
//! | `xsd:integer` text is parsed after whitespace collapsing (`whiteSpace="collapse"` in the schema) | `str::parse::<i64>()` on the trimmed text | Identical for well-formed documents, tolerant of the surrounding whitespace JAXB would also have ignored |
//! | `BigDecimal` fields (`MaxUnits`, `PeakUnits`, `StandardRate`, `Cost`, `OvertimeRate`, `OvertimeCost`, `CostPerUse`, `ActualCost`, `ActualOvertimeCost`, `RemainingCost`, `RemainingOvertimeCost`, `WorkVariance`, `CostVariance`, `SV`, `CV`, `ACWP`, `BCWS`, `BCWP`, the `Baseline` costs, `AvailabilityPeriod.AvailableUnits` and the `Rate` amounts) use `Adapter2` (`jakarta.xml.bind.DatatypeConverter.parseDecimal`) | `f64` | `f64` is the Rust counterpart of the domain layer's `double`; malformed text degrades to `None` |
//! | `Number` field `PercentWorkComplete` uses `Adapter19` (`DatatypeConverter.parsePercentComplete`) and holds either an `Integer` or a `Double` | `f64` | Both Java branches are narrowed to `f64`; malformed text degrades to `None` |
//! | `Adapter15` / `Adapter21` (`DatatypeConverter.parseUUID`) throws `IllegalArgumentException` on a malformed GUID | `DatatypeConverter::parse_uuid` yields `None` | Graceful degradation, hard convention: library code never panics |
//! | `Adapter1` / `DatatypeConverter.parseBoolean` maps a missing or non-`1` value to `Boolean.FALSE` | the adapter call is only made when the element is present, so an absent element stays `None` | This mirrors the Java field type `Boolean` (null when the element is absent) rather than the adapter's own fallback |
//! | `Adapter4` (`LocalDateTime`) returns `null` when the text cannot be parsed | `DatatypeConverter::parse_date_time` yields `None` | Same observable result |
//! | `LocalDateTime` | `chrono::NaiveDateTime` | The crate-wide mapping of the domain layer |
//! | Every `xsd:duration` value (`Work`, `RegularWork`, `OvertimeWork`, `ActualWork`, `RemainingWork`, `ActualOvertimeWork`, `RemainingOvertimeWork`, `ActualWorkProtected`, `ActualOvertimeWorkProtected`, `Baseline.Work`) is declared as `String` and goes through `Adapter3` / `DatatypeConverter.parseString`, which returns the text unchanged | kept as `Option<String>`, not converted | `MSPDIReader` converts these values itself (`DatatypeConverter.parseDuration(...)`, which needs the project defaults), so the intermediate model must keep the raw text. Deliberately **not** converted to a duration type |
//! | `Adapter16` / `Adapter17` / `Adapter18` / `Adapter20` convert the domain enums (`ResourceType`, `WorkGroup`, `AccrueType`, `BookingType`) | `Option<crate::resource_type::ResourceType>` / `Option<crate::work_group::WorkGroup>` / `Option<crate::accrue_type::AccrueType>` / `Option<crate::booking_type::BookingType>` | The Rust enums are not nullable, so an unrecognised value degrades to `None` even though Java's `parseResourceType` cannot fail (`ResourceType.WORK` is the fallback) |
//! | `@XmlElement(name = "MaxUnits", type = String.class, defaultValue = "1.0")` | an absent `<MaxUnits>` stays `None` | JAXB substitutes the declared `defaultValue` for a missing element; this model keeps every field nullable and never applies the default. Recorded here as the one place where a JAXB default is not reproduced |
//! | `Resource.type` is a Java field called `type` | field `r#type` | `type` is a Rust keyword, hence the raw identifier (the crate-wide convention for Java's `type`, see `ProjectCalendar::r#type`) |
//! | `String` fields hold the raw character data | `Option<String>` | [`crate::mspdi::xml_node::text`] returns a `Cow`; `from_node` stores the owned form |
//!
//! # Skipped Java members + reason
//!
//! - The JAXB generated `getXxx()` / `setXxx()` accessors and the lazily created `ArrayList` behind
//!   the list accessors. The Rust port stores the values in public fields and reads them directly
//!   (the same choice as the rest of the crate); `from_node` fills every `Vec` once.
//! - The `marshal` halves of the adapters. This crate is reader-only (see `AGENTS.md`).
//! - `ObjectFactory`'s factory methods for these classes; they are replaced by the `from_node`
//!   constructors.
//!
//! No field of any class in this sub-tree is skipped: `Resource` (73 fields), `AvailabilityPeriod`
//! (3), `Baseline` (5), `ExtendedAttribute` (4), `OutlineCode` (3) and `Rate` (8) are complete.

use std::borrow::Cow;

use chrono::NaiveDateTime;
use roxmltree::Node;
use uuid::Uuid;

use crate::accrue_type::AccrueType;
use crate::booking_type::BookingType;
use crate::mspdi::datatype_converter::DatatypeConverter;
use crate::mspdi::schema::timephased_data_type::TimephasedDataType;
use crate::mspdi::xml_node::{child, child_text, children};
use crate::resource_type::ResourceType;
use crate::work_group::WorkGroup;

/// Corresponds to Java: `Project.Resources.Resource` (`Project.java` line 19578).
pub mod resource {
    use super::{
        child, child_text, children, parse_big_integer, parse_boolean, parse_decimal, AccrueType,
        BookingType, DatatypeConverter, NaiveDateTime, Node, ResourceType, TimephasedDataType, Uuid,
        WorkGroup,
    };

    use self::availability_periods::AvailabilityPeriod;
    use self::baseline::Baseline;
    use self::extended_attribute::ExtendedAttribute;
    use self::outline_code::OutlineCode;
    use self::rates::Rate;

    /// Corresponds to Java: `Project.Resources.Resource` (`Project.java` line 19578).
    ///
    /// One resource of the document's `<Resources>` element.
    #[derive(Debug, Default, Clone)]
    pub struct Resource {
        /// Corresponds to Java: `@XmlElement(name = "UID", required = true, type = String.class) @XmlJavaTypeAdapter(Adapter14.class) @XmlSchemaType(name = "integer") protected Integer uid;`
        pub uid: Option<i32>,
        /// Corresponds to Java: `@XmlElement(name = "GUID", type = String.class) @XmlJavaTypeAdapter(Adapter15.class) protected UUID guid;`
        pub guid: Option<Uuid>,
        /// Corresponds to Java: `@XmlElement(name = "ID") protected BigInteger id;`
        pub id: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "Name") protected String name;`
        pub name: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "Type", type = String.class) @XmlJavaTypeAdapter(Adapter16.class) protected ResourceType type;`
        ///
        /// `type` is a Rust keyword, hence the raw identifier (the crate-wide convention for
        /// Java's `type`).
        pub r#type: Option<ResourceType>,
        /// Corresponds to Java: `@XmlElement(name = "IsNull", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean isNull;`
        pub is_null: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "Initials") protected String initials;`
        pub initials: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "Phonetics") protected String phonetics;`
        pub phonetics: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "NTAccount") protected String ntAccount;`
        pub nt_account: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "MaterialLabel") protected String materialLabel;`
        pub material_label: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "Code") protected String code;`
        pub code: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "Group") protected String group;`
        pub group: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "WorkGroup", type = String.class) @XmlJavaTypeAdapter(Adapter17.class) protected WorkGroup workGroup;`
        pub work_group: Option<WorkGroup>,
        /// Corresponds to Java: `@XmlElement(name = "EmailAddress") protected String emailAddress;`
        pub email_address: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "Hyperlink") protected String hyperlink;`
        pub hyperlink: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "HyperlinkAddress") protected String hyperlinkAddress;`
        pub hyperlink_address: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "HyperlinkSubAddress") protected String hyperlinkSubAddress;`
        pub hyperlink_sub_address: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "MaxUnits", type = String.class, defaultValue = "1.0") @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal maxUnits;`
        ///
        /// An absent element stays `None`; see the `defaultValue` note in the differences table.
        pub max_units: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "PeakUnits", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal peakUnits;`
        pub peak_units: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "OverAllocated", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean overAllocated;`
        pub over_allocated: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "AvailableFrom", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime availableFrom;`
        pub available_from: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "AvailableTo", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime availableTo;`
        pub available_to: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "Start", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime start;`
        pub start: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "Finish", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime finish;`
        pub finish: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "CanLevel", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean canLevel;`
        pub can_level: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "AccrueAt", type = String.class) @XmlJavaTypeAdapter(Adapter18.class) protected AccrueType accrueAt;`
        pub accrue_at: Option<AccrueType>,
        /// Corresponds to Java: `@XmlElement(name = "Work") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String work;`
        pub work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "RegularWork") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String regularWork;`
        pub regular_work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "OvertimeWork") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String overtimeWork;`
        pub overtime_work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "ActualWork") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String actualWork;`
        pub actual_work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "RemainingWork") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String remainingWork;`
        pub remaining_work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "ActualOvertimeWork") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String actualOvertimeWork;`
        pub actual_overtime_work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "RemainingOvertimeWork") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String remainingOvertimeWork;`
        pub remaining_overtime_work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "PercentWorkComplete", type = String.class) @XmlJavaTypeAdapter(Adapter19.class) @XmlSchemaType(name = "integer") protected Number percentWorkComplete;`
        pub percent_work_complete: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "StandardRate") protected BigDecimal standardRate;`
        pub standard_rate: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "StandardRateFormat") protected BigInteger standardRateFormat;`
        pub standard_rate_format: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "Cost") protected BigDecimal cost;`
        pub cost: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "OvertimeRate") protected BigDecimal overtimeRate;`
        pub overtime_rate: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "OvertimeRateFormat") protected BigInteger overtimeRateFormat;`
        pub overtime_rate_format: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "OvertimeCost") protected BigDecimal overtimeCost;`
        pub overtime_cost: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "CostPerUse") protected BigDecimal costPerUse;`
        pub cost_per_use: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "ActualCost") protected BigDecimal actualCost;`
        pub actual_cost: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "ActualOvertimeCost") protected BigDecimal actualOvertimeCost;`
        pub actual_overtime_cost: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "RemainingCost") protected BigDecimal remainingCost;`
        pub remaining_cost: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "RemainingOvertimeCost") protected BigDecimal remainingOvertimeCost;`
        pub remaining_overtime_cost: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "WorkVariance", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal workVariance;`
        pub work_variance: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "CostVariance", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal costVariance;`
        pub cost_variance: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "SV", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal sv;`
        pub sv: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "CV", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal cv;`
        pub cv: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "ACWP", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal acwp;`
        pub acwp: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "CalendarUID") protected BigInteger calendarUID;`
        pub calendar_uid: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "Notes") protected String notes;`
        pub notes: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "BCWS", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal bcws;`
        pub bcws: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "BCWP", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal bcwp;`
        pub bcwp: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "IsGeneric", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean isGeneric;`
        pub is_generic: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "IsInactive", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean isInactive;`
        pub is_inactive: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "IsEnterprise", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean isEnterprise;`
        pub is_enterprise: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "BookingType", type = String.class) @XmlJavaTypeAdapter(Adapter20.class) protected BookingType bookingType;`
        pub booking_type: Option<BookingType>,
        /// Corresponds to Java: `@XmlElement(name = "ActualWorkProtected") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String actualWorkProtected;`
        pub actual_work_protected: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "ActualOvertimeWorkProtected") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String actualOvertimeWorkProtected;`
        pub actual_overtime_work_protected: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "ActiveDirectoryGUID") protected String activeDirectoryGUID;`
        pub active_directory_guid: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "CreationDate", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime creationDate;`
        pub creation_date: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "ExtendedAttribute") protected List<Project.Resources.Resource.ExtendedAttribute> extendedAttribute;`
        pub extended_attribute: Vec<ExtendedAttribute>,
        /// Corresponds to Java: `@XmlElement(name = "Baseline") protected List<Project.Resources.Resource.Baseline> baseline;`
        pub baseline: Vec<Baseline>,
        /// Corresponds to Java: `@XmlElement(name = "OutlineCode") protected List<Project.Resources.Resource.OutlineCode> outlineCode;`
        pub outline_code: Vec<OutlineCode>,
        /// Corresponds to Java: `@XmlElement(name = "CostCenter") protected String costCenter;`
        pub cost_center: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "IsCostResource", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean isCostResource;`
        pub is_cost_resource: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "AssnOwner") protected String assnOwner;`
        pub assn_owner: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "AssnOwnerGuid") protected String assnOwnerGuid;`
        pub assn_owner_guid: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "IsBudget", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean isBudget;`
        pub is_budget: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "AvailabilityPeriods") protected Project.Resources.Resource.AvailabilityPeriods availabilityPeriods;`
        ///
        /// Flattened from the `AvailabilityPeriods` wrapper to the wrapper's `AvailabilityPeriod`
        /// child elements.
        pub availability_periods: Vec<AvailabilityPeriod>,
        /// Corresponds to Java: `@XmlElement(name = "Rates") protected Project.Resources.Resource.Rates rates;`
        ///
        /// Flattened from the `Rates` wrapper to the wrapper's `Rate` child elements.
        pub rates: Vec<Rate>,
        /// Whether the source contained a `Rates` wrapper, including an empty one.
        ///
        /// Corresponds to Java: `protected Rates rates;` (`null` when `<Rates>` is absent).
        pub rates_present: bool,
        /// Corresponds to Java: `@XmlElement(name = "TimephasedData") protected List<TimephasedDataType> timephasedData;`
        pub timephased_data: Vec<TimephasedDataType>,
    }

    impl Resource {
        /// Rust-specific constructor replacing the JAXB unmarshaller for
        /// `Project.Resources.Resource`.
        ///
        /// The field order matches the Java field declaration order; the two flattened wrappers
        /// (`AvailabilityPeriods` and `Rates`) are located by their wrapper element.
        pub fn from_node(node: Node<'_, '_>) -> Self {
            Self {
                uid: child_text(node, "UID").and_then(|value| DatatypeConverter::parse_resource_uid(&value)),
                guid: child_text(node, "GUID").and_then(|value| DatatypeConverter::parse_uuid(&value)),
                id: parse_big_integer(child_text(node, "ID")),
                name: child_text(node, "Name").map(|value| value.into_owned()),
                r#type: child_text(node, "Type").and_then(|value| DatatypeConverter::parse_resource_type(&value)),
                is_null: parse_boolean(child_text(node, "IsNull")),
                initials: child_text(node, "Initials").map(|value| value.into_owned()),
                phonetics: child_text(node, "Phonetics").map(|value| value.into_owned()),
                nt_account: child_text(node, "NTAccount").map(|value| value.into_owned()),
                material_label: child_text(node, "MaterialLabel").map(|value| value.into_owned()),
                code: child_text(node, "Code").map(|value| value.into_owned()),
                group: child_text(node, "Group").map(|value| value.into_owned()),
                work_group: child_text(node, "WorkGroup").and_then(|value| DatatypeConverter::parse_work_group(&value)),
                email_address: child_text(node, "EmailAddress").map(|value| value.into_owned()),
                hyperlink: child_text(node, "Hyperlink").map(|value| value.into_owned()),
                hyperlink_address: child_text(node, "HyperlinkAddress").map(|value| value.into_owned()),
                hyperlink_sub_address: child_text(node, "HyperlinkSubAddress").map(|value| value.into_owned()),
                max_units: parse_decimal(child_text(node, "MaxUnits")),
                peak_units: parse_decimal(child_text(node, "PeakUnits")),
                over_allocated: parse_boolean(child_text(node, "OverAllocated")),
                available_from: child_text(node, "AvailableFrom").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                available_to: child_text(node, "AvailableTo").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                start: child_text(node, "Start").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                finish: child_text(node, "Finish").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                can_level: parse_boolean(child_text(node, "CanLevel")),
                accrue_at: child_text(node, "AccrueAt").and_then(|value| DatatypeConverter::parse_accrue_type(&value)),
                work: child_text(node, "Work").map(|value| value.into_owned()),
                regular_work: child_text(node, "RegularWork").map(|value| value.into_owned()),
                overtime_work: child_text(node, "OvertimeWork").map(|value| value.into_owned()),
                actual_work: child_text(node, "ActualWork").map(|value| value.into_owned()),
                remaining_work: child_text(node, "RemainingWork").map(|value| value.into_owned()),
                actual_overtime_work: child_text(node, "ActualOvertimeWork").map(|value| value.into_owned()),
                remaining_overtime_work: child_text(node, "RemainingOvertimeWork").map(|value| value.into_owned()),
                percent_work_complete: child_text(node, "PercentWorkComplete").and_then(|value| DatatypeConverter::parse_percent_complete(&value)),
                standard_rate: parse_decimal(child_text(node, "StandardRate")),
                standard_rate_format: parse_big_integer(child_text(node, "StandardRateFormat")),
                cost: parse_decimal(child_text(node, "Cost")),
                overtime_rate: parse_decimal(child_text(node, "OvertimeRate")),
                overtime_rate_format: parse_big_integer(child_text(node, "OvertimeRateFormat")),
                overtime_cost: parse_decimal(child_text(node, "OvertimeCost")),
                cost_per_use: parse_decimal(child_text(node, "CostPerUse")),
                actual_cost: parse_decimal(child_text(node, "ActualCost")),
                actual_overtime_cost: parse_decimal(child_text(node, "ActualOvertimeCost")),
                remaining_cost: parse_decimal(child_text(node, "RemainingCost")),
                remaining_overtime_cost: parse_decimal(child_text(node, "RemainingOvertimeCost")),
                work_variance: parse_decimal(child_text(node, "WorkVariance")),
                cost_variance: parse_decimal(child_text(node, "CostVariance")),
                sv: parse_decimal(child_text(node, "SV")),
                cv: parse_decimal(child_text(node, "CV")),
                acwp: parse_decimal(child_text(node, "ACWP")),
                calendar_uid: parse_big_integer(child_text(node, "CalendarUID")),
                notes: child_text(node, "Notes").map(|value| value.into_owned()),
                bcws: parse_decimal(child_text(node, "BCWS")),
                bcwp: parse_decimal(child_text(node, "BCWP")),
                is_generic: parse_boolean(child_text(node, "IsGeneric")),
                is_inactive: parse_boolean(child_text(node, "IsInactive")),
                is_enterprise: parse_boolean(child_text(node, "IsEnterprise")),
                booking_type: child_text(node, "BookingType").and_then(|value| DatatypeConverter::parse_booking_type(&value)),
                actual_work_protected: child_text(node, "ActualWorkProtected").map(|value| value.into_owned()),
                actual_overtime_work_protected: child_text(node, "ActualOvertimeWorkProtected").map(|value| value.into_owned()),
                active_directory_guid: child_text(node, "ActiveDirectoryGUID").map(|value| value.into_owned()),
                creation_date: child_text(node, "CreationDate").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                extended_attribute: children(node, "ExtendedAttribute").map(ExtendedAttribute::from_node).collect(),
                baseline: children(node, "Baseline").map(Baseline::from_node).collect(),
                outline_code: children(node, "OutlineCode").map(OutlineCode::from_node).collect(),
                cost_center: child_text(node, "CostCenter").map(|value| value.into_owned()),
                is_cost_resource: parse_boolean(child_text(node, "IsCostResource")),
                assn_owner: child_text(node, "AssnOwner").map(|value| value.into_owned()),
                assn_owner_guid: child_text(node, "AssnOwnerGuid").map(|value| value.into_owned()),
                is_budget: parse_boolean(child_text(node, "IsBudget")),
                availability_periods: child(node, "AvailabilityPeriods")
                    .map(|wrapper| {
                        children(wrapper, "AvailabilityPeriod")
                            .map(AvailabilityPeriod::from_node)
                            .collect()
                    })
                    .unwrap_or_default(),
                rates_present: child(node, "Rates").is_some(),
                rates: child(node, "Rates")
                    .map(|wrapper| children(wrapper, "Rate").map(Rate::from_node).collect())
                    .unwrap_or_default(),
                timephased_data: children(node, "TimephasedData").map(TimephasedDataType::from_node).collect(),
            }
        }
    }

    /// Corresponds to Java: `Project.Resources.Resource.Baseline` (`Project.java` line 21788).
    ///
    /// This is *not* [`crate::mspdi::schema::tasks::task::baseline::Baseline`]: the task baseline
    /// carries twelve more fields (`Start`, `Finish`, `Duration`, ...) which the resource baseline
    /// does not have.
    pub mod baseline {
        use super::{child_text, parse_big_integer, parse_decimal, Node};

        /// Corresponds to Java: `Project.Resources.Resource.Baseline` (`Project.java` line 21788).
        #[derive(Debug, Default, Clone)]
        pub struct Baseline {
            /// Corresponds to Java: `@XmlElement(name = "Number", required = true) protected BigInteger number;`
            pub number: Option<i64>,
            /// Corresponds to Java: `@XmlElement(name = "Work") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String work;`
            pub work: Option<String>,
            /// Corresponds to Java: `@XmlElement(name = "Cost", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal cost;`
            pub cost: Option<f64>,
            /// Corresponds to Java: `@XmlElement(name = "BCWS", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal bcws;`
            pub bcws: Option<f64>,
            /// Corresponds to Java: `@XmlElement(name = "BCWP", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal bcwp;`
            pub bcwp: Option<f64>,
        }

        impl Baseline {
            /// Rust-specific constructor replacing the JAXB unmarshaller for
            /// `Project.Resources.Resource.Baseline`.
            pub fn from_node(node: Node<'_, '_>) -> Self {
                Self {
                    number: parse_big_integer(child_text(node, "Number")),
                    work: child_text(node, "Work").map(|value| value.into_owned()),
                    cost: parse_decimal(child_text(node, "Cost")),
                    bcws: parse_decimal(child_text(node, "BCWS")),
                    bcwp: parse_decimal(child_text(node, "BCWP")),
                }
            }
        }
    }

    /// Corresponds to Java: `Project.Resources.Resource.ExtendedAttribute` (`Project.java` line
    /// 21988).
    ///
    /// This is *not* [`crate::mspdi::schema::tasks::task::extended_attribute::ExtendedAttribute`]:
    /// the two Java classes are structurally identical but distinct, and their `ValueGUID` fields
    /// use different adapters (`Adapter21` here, `Adapter29` for the task).
    pub mod extended_attribute {
        use super::{child_text, parse_big_integer, DatatypeConverter, Node, Uuid};

        /// Corresponds to Java: `Project.Resources.Resource.ExtendedAttribute` (`Project.java` line
        /// 21988).
        #[derive(Debug, Default, Clone)]
        pub struct ExtendedAttribute {
            /// Corresponds to Java: `@XmlElement(name = "FieldID") protected String fieldID;`
            pub field_id: Option<String>,
            /// Corresponds to Java: `@XmlElement(name = "Value") protected String value;`
            pub value: Option<String>,
            /// Corresponds to Java: `@XmlElement(name = "ValueGUID", type = String.class) @XmlJavaTypeAdapter(Adapter21.class) protected UUID valueGUID;`
            pub value_guid: Option<Uuid>,
            /// Corresponds to Java: `@XmlElement(name = "DurationFormat") protected BigInteger durationFormat;`
            pub duration_format: Option<i64>,
        }

        impl ExtendedAttribute {
            /// Rust-specific constructor replacing the JAXB unmarshaller for
            /// `Project.Resources.Resource.ExtendedAttribute`.
            pub fn from_node(node: Node<'_, '_>) -> Self {
                Self {
                    field_id: child_text(node, "FieldID").map(|value| value.into_owned()),
                    value: child_text(node, "Value").map(|value| value.into_owned()),
                    value_guid: child_text(node, "ValueGUID").and_then(|value| DatatypeConverter::parse_uuid(&value)),
                    duration_format: parse_big_integer(child_text(node, "DurationFormat")),
                }
            }
        }
    }

    /// Corresponds to Java: `Project.Resources.Resource.OutlineCode` (`Project.java` line 22128).
    ///
    /// This is *not* [`crate::mspdi::schema::tasks::task::outline_code::OutlineCode`] (nor
    /// [`crate::mspdi::schema::outline_codes::OutlineCode`], the definition class): the Java
    /// classes are declared separately below each owner type.
    ///
    /// `ValueGUID` is a plain `String` here (there is no adapter on the Java field), unlike the
    /// `ValueGUID` of [`super::extended_attribute::ExtendedAttribute`].
    pub mod outline_code {
        use super::{child_text, parse_big_integer, Node};

        /// Corresponds to Java: `Project.Resources.Resource.OutlineCode` (`Project.java` line 22128).
        #[derive(Debug, Default, Clone)]
        pub struct OutlineCode {
            /// Corresponds to Java: `@XmlElement(name = "FieldID") protected String fieldID;`
            pub field_id: Option<String>,
            /// Corresponds to Java: `@XmlElement(name = "ValueID") protected BigInteger valueID;`
            pub value_id: Option<i64>,
            /// Corresponds to Java: `@XmlElement(name = "ValueGUID") protected String valueGUID;`
            pub value_guid: Option<String>,
        }

        impl OutlineCode {
            /// Rust-specific constructor replacing the JAXB unmarshaller for
            /// `Project.Resources.Resource.OutlineCode`.
            pub fn from_node(node: Node<'_, '_>) -> Self {
                Self {
                    field_id: child_text(node, "FieldID").map(|value| value.into_owned()),
                    value_id: parse_big_integer(child_text(node, "ValueID")),
                    value_guid: child_text(node, "ValueGUID").map(|value| value.into_owned()),
                }
            }
        }
    }

    /// Corresponds to Java: `Project.Resources.Resource.AvailabilityPeriods` (`Project.java` line
    /// 21607).
    ///
    /// The wrapper class is flattened away (`Resource.availability_periods` holds the
    /// `AvailabilityPeriod` elements directly, see the differences table), but the module is kept
    /// so that the nesting of the Java classes stays visible.
    pub mod availability_periods {
        use super::{child_text, parse_decimal, DatatypeConverter, NaiveDateTime, Node};

        /// Corresponds to Java: `Project.Resources.Resource.AvailabilityPeriods.AvailabilityPeriod`
        /// (`Project.java` line 21669).
        ///
        /// One window in which the resource is available for the given number of units.
        #[derive(Debug, Default, Clone)]
        pub struct AvailabilityPeriod {
            /// Corresponds to Java: `@XmlElement(name = "AvailableFrom", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime availableFrom;`
            pub available_from: Option<NaiveDateTime>,
            /// Corresponds to Java: `@XmlElement(name = "AvailableTo", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime availableTo;`
            pub available_to: Option<NaiveDateTime>,
            /// Corresponds to Java: `@XmlElement(name = "AvailableUnits", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal availableUnits;`
            pub available_units: Option<f64>,
        }

        impl AvailabilityPeriod {
            /// Rust-specific constructor replacing the JAXB unmarshaller for
            /// `Project.Resources.Resource.AvailabilityPeriods.AvailabilityPeriod`.
            pub fn from_node(node: Node<'_, '_>) -> Self {
                Self {
                    available_from: child_text(node, "AvailableFrom").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                    available_to: child_text(node, "AvailableTo").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                    available_units: parse_decimal(child_text(node, "AvailableUnits")),
                }
            }
        }
    }

    /// Corresponds to Java: `Project.Resources.Resource.Rates` (`Project.java` line 22286).
    ///
    /// The wrapper class is flattened away (`Resource.rates` holds the `Rate` elements directly,
    /// see the differences table), but the module is kept so that the nesting of the Java classes
    /// stays visible.
    pub mod rates {
        use super::{child_text, parse_big_integer, parse_decimal, DatatypeConverter, NaiveDateTime, Node};

        /// Corresponds to Java: `Project.Resources.Resource.Rates.Rate` (`Project.java` line 22390).
        ///
        /// One entry of one of the resource's cost rate tables (`RateTable` selects the table).
        #[derive(Debug, Default, Clone)]
        pub struct Rate {
            /// Corresponds to Java: `@XmlElement(name = "RatesFrom", required = true, type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime ratesFrom;`
            pub rates_from: Option<NaiveDateTime>,
            /// Corresponds to Java: `@XmlElement(name = "RatesTo", required = true, type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime ratesTo;`
            pub rates_to: Option<NaiveDateTime>,
            /// Corresponds to Java: `@XmlElement(name = "RateTable") protected BigInteger rateTable;`
            pub rate_table: Option<i64>,
            /// Corresponds to Java: `@XmlElement(name = "StandardRate") protected BigDecimal standardRate;`
            pub standard_rate: Option<f64>,
            /// Corresponds to Java: `@XmlElement(name = "StandardRateFormat") protected BigInteger standardRateFormat;`
            pub standard_rate_format: Option<i64>,
            /// Corresponds to Java: `@XmlElement(name = "OvertimeRate") protected BigDecimal overtimeRate;`
            pub overtime_rate: Option<f64>,
            /// Corresponds to Java: `@XmlElement(name = "OvertimeRateFormat") protected BigInteger overtimeRateFormat;`
            pub overtime_rate_format: Option<i64>,
            /// Corresponds to Java: `@XmlElement(name = "CostPerUse") protected BigDecimal costPerUse;`
            pub cost_per_use: Option<f64>,
        }

        impl Rate {
            /// Rust-specific constructor replacing the JAXB unmarshaller for
            /// `Project.Resources.Resource.Rates.Rate`.
            pub fn from_node(node: Node<'_, '_>) -> Self {
                Self {
                    rates_from: child_text(node, "RatesFrom").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                    rates_to: child_text(node, "RatesTo").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                    rate_table: parse_big_integer(child_text(node, "RateTable")),
                    standard_rate: parse_decimal(child_text(node, "StandardRate")),
                    standard_rate_format: parse_big_integer(child_text(node, "StandardRateFormat")),
                    overtime_rate: parse_decimal(child_text(node, "OvertimeRate")),
                    overtime_rate_format: parse_big_integer(child_text(node, "OvertimeRateFormat")),
                    cost_per_use: parse_decimal(child_text(node, "CostPerUse")),
                }
            }
        }
    }
}

/// Rust-specific: parses the text of a Java `BigInteger` field.
///
/// JAXB parses `xsd:integer` (whose `whiteSpace` facet is `collapse`) into a `BigInteger`; the
/// Rust port narrows it to `i64` and yields `None` for text that is not a valid `i64`.
fn parse_big_integer(value: Option<Cow<'_, str>>) -> Option<i64> {
    value.and_then(|value| value.trim().parse::<i64>().ok())
}

/// Rust-specific: parses the text of a Java `BigDecimal` field.
///
/// JAXB parses these fields with `Adapter2` (`jakarta.xml.bind.DatatypeConverter.parseDecimal`);
/// the Rust port narrows the result to `f64` and yields `None` for text that is neither empty nor a
/// valid `f64`.
fn parse_decimal(value: Option<Cow<'_, str>>) -> Option<f64> {
    value.and_then(|value| value.trim().parse::<f64>().ok())
}

/// Rust-specific: runs the text of a Java `Boolean` field through `Adapter1` /
/// `DatatypeConverter.parseBoolean`, keeping `None` for an absent element (the Java field stays
/// `null` in that case).
fn parse_boolean(value: Option<Cow<'_, str>>) -> Option<bool> {
    value.map(|value| DatatypeConverter::parse_boolean(&value))
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveDateTime};
    use roxmltree::Document;
    use uuid::Uuid;

    use super::resource::availability_periods::AvailabilityPeriod;
    use super::resource::baseline::Baseline;
    use super::resource::extended_attribute::ExtendedAttribute;
    use super::resource::outline_code::OutlineCode;
    use super::resource::rates::Rate;
    use super::resource::Resource;
    use crate::accrue_type::AccrueType;
    use crate::booking_type::BookingType;
    use crate::resource_type::ResourceType;
    use crate::work_group::WorkGroup;

    /// A resource with every part of the sub-tree populated: the scalar fields (including the two
    /// `xsd:duration` values kept as text), one `Rates` wrapper with two rate table entries, one
    /// `AvailabilityPeriods` wrapper with two periods, one baseline, one extended attribute, one
    /// outline code and one timephased data item.
    const RESOURCE_XML: &str = r#"<Resource xmlns="http://schemas.microsoft.com/project">
        <UID>7</UID>
        <GUID>01234567-89AB-CDEF-0123-456789ABCDEF</GUID>
        <ID>3</ID>
        <Name>Wade Golden</Name>
        <Type>1</Type>
        <IsNull>0</IsNull>
        <Initials>WG</Initials>
        <Phonetics>Wade</Phonetics>
        <NTAccount>DOMAIN\wg</NTAccount>
        <MaterialLabel>m</MaterialLabel>
        <Code>10</Code>
        <Group>Steelray</Group>
        <WorkGroup>2</WorkGroup>
        <EmailAddress>wade.golden@steelray.com</EmailAddress>
        <Hyperlink>http://example.com</Hyperlink>
        <HyperlinkAddress>http://example.com/address</HyperlinkAddress>
        <HyperlinkSubAddress>sub</HyperlinkSubAddress>
        <MaxUnits>1.00</MaxUnits>
        <PeakUnits>2.00</PeakUnits>
        <OverAllocated>1</OverAllocated>
        <AvailableFrom>2020-01-02T08:00:00</AvailableFrom>
        <AvailableTo>2020-02-02T17:00:00</AvailableTo>
        <Start>2020-01-02T08:00:00</Start>
        <Finish>2020-01-31T17:00:00</Finish>
        <CanLevel>1</CanLevel>
        <AccrueAt>2</AccrueAt>
        <Work>PT80H0M0S</Work>
        <RegularWork>PT76H0M0S</RegularWork>
        <OvertimeWork>PT4H0M0S</OvertimeWork>
        <ActualWork>PT40H0M0S</ActualWork>
        <RemainingWork>PT40H0M0S</RemainingWork>
        <ActualOvertimeWork>PT2H0M0S</ActualOvertimeWork>
        <RemainingOvertimeWork>PT2H0M0S</RemainingOvertimeWork>
        <PercentWorkComplete>25.5</PercentWorkComplete>
        <StandardRate>12.5</StandardRate>
        <StandardRateFormat>2</StandardRateFormat>
        <Cost>100.25</Cost>
        <OvertimeRate>18.75</OvertimeRate>
        <OvertimeRateFormat>5</OvertimeRateFormat>
        <OvertimeCost>1.5</OvertimeCost>
        <CostPerUse>3.5</CostPerUse>
        <ActualCost>40.5</ActualCost>
        <ActualOvertimeCost>0.75</ActualOvertimeCost>
        <RemainingCost>59.75</RemainingCost>
        <RemainingOvertimeCost>0.75</RemainingOvertimeCost>
        <WorkVariance>4800</WorkVariance>
        <CostVariance>-1.25</CostVariance>
        <SV>-1.25</SV>
        <CV>-1.5</CV>
        <ACWP>40.5</ACWP>
        <CalendarUID>4</CalendarUID>
        <Notes>Resource note</Notes>
        <BCWS>10.5</BCWS>
        <BCWP>20.25</BCWP>
        <IsGeneric>0</IsGeneric>
        <IsInactive>0</IsInactive>
        <IsEnterprise>1</IsEnterprise>
        <BookingType>1</BookingType>
        <ActualWorkProtected>PT0H0M0S</ActualWorkProtected>
        <ActualOvertimeWorkProtected>PT0H0M0S</ActualOvertimeWorkProtected>
        <ActiveDirectoryGUID>1111</ActiveDirectoryGUID>
        <CreationDate>2020-03-04T05:06:07</CreationDate>
        <ExtendedAttribute>
            <FieldID>188743731</FieldID>
            <Value>Text value</Value>
            <ValueGUID>11234567-89AB-CDEF-0123-456789ABCDEF</ValueGUID>
            <DurationFormat>7</DurationFormat>
        </ExtendedAttribute>
        <Baseline>
            <Number>0</Number>
            <Work>PT120H0M0S</Work>
            <Cost>2000.5</Cost>
            <BCWS>1500</BCWS>
            <BCWP>1200</BCWP>
        </Baseline>
        <OutlineCode>
            <FieldID>188743733</FieldID>
            <ValueID>5</ValueID>
            <ValueGUID>Some code value</ValueGUID>
        </OutlineCode>
        <CostCenter>CC1</CostCenter>
        <IsCostResource>1</IsCostResource>
        <AssnOwner>Owner</AssnOwner>
        <AssnOwnerGuid>21234567-89AB-CDEF-0123-456789ABCDEF</AssnOwnerGuid>
        <IsBudget>0</IsBudget>
        <AvailabilityPeriods>
            <AvailabilityPeriod>
                <AvailableFrom>2020-06-01T00:00:00</AvailableFrom>
                <AvailableTo>2020-06-30T00:00:00</AvailableTo>
                <AvailableUnits>0.5</AvailableUnits>
            </AvailabilityPeriod>
            <AvailabilityPeriod>
                <AvailableFrom>2020-08-01T00:00:00</AvailableFrom>
                <AvailableTo>2020-08-31T00:00:00</AvailableTo>
                <AvailableUnits>1.0</AvailableUnits>
            </AvailabilityPeriod>
        </AvailabilityPeriods>
        <Rates>
            <Rate>
                <RatesFrom>2020-01-01T00:00:00</RatesFrom>
                <RatesTo>2021-01-01T00:00:00</RatesTo>
                <RateTable>1</RateTable>
                <StandardRate>25.5</StandardRate>
                <StandardRateFormat>2</StandardRateFormat>
                <OvertimeRate>40</OvertimeRate>
                <OvertimeRateFormat>5</OvertimeRateFormat>
                <CostPerUse>3.5</CostPerUse>
            </Rate>
            <Rate>
                <RatesFrom>2021-01-01T00:00:00</RatesFrom>
                <RatesTo>2022-01-01T00:00:00</RatesTo>
                <RateTable>2</RateTable>
                <StandardRate>30</StandardRate>
            </Rate>
        </Rates>
        <TimephasedData><Type>2</Type></TimephasedData>
    </Resource>"#;

    fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .expect("valid date")
            .and_hms_opt(hour, minute, second)
            .expect("valid time")
    }

    fn resource() -> Resource {
        let doc = Document::parse(RESOURCE_XML).expect("valid XML");
        Resource::from_node(doc.root_element())
    }

    /// Parses a `<Resource>` element given as a literal.
    fn resource_from(xml: &str) -> Resource {
        let doc = Document::parse(xml).expect("valid XML");
        Resource::from_node(doc.root_element())
    }

    /// The scalar fields are converted exactly like the Java adapters do, including the
    /// `xsd:duration` values that stay strings.
    #[test]
    fn resource_reads_scalar_fields() {
        let resource = resource();

        // Adapter14 (parseResourceUID) and Adapter15 (parseUUID).
        assert_eq!(resource.uid, Some(7));
        assert_eq!(
            resource.guid,
            Some(Uuid::parse_str("01234567-89AB-CDEF-0123-456789ABCDEF").expect("valid UUID"))
        );
        // BigInteger and String fields.
        assert_eq!(resource.id, Some(3));
        assert_eq!(resource.name.as_deref(), Some("Wade Golden"));
        assert_eq!(resource.initials.as_deref(), Some("WG"));
        assert_eq!(resource.nt_account.as_deref(), Some("DOMAIN\\wg"));
        assert_eq!(resource.material_label.as_deref(), Some("m"));
        assert_eq!(resource.code.as_deref(), Some("10"));
        assert_eq!(resource.group.as_deref(), Some("Steelray"));
        assert_eq!(resource.email_address.as_deref(), Some("wade.golden@steelray.com"));
        assert_eq!(resource.hyperlink.as_deref(), Some("http://example.com"));
        assert_eq!(resource.hyperlink_address.as_deref(), Some("http://example.com/address"));
        assert_eq!(resource.hyperlink_sub_address.as_deref(), Some("sub"));
        assert_eq!(resource.notes.as_deref(), Some("Resource note"));
        assert_eq!(resource.cost_center.as_deref(), Some("CC1"));
        assert_eq!(resource.assn_owner.as_deref(), Some("Owner"));
        assert_eq!(resource.assn_owner_guid.as_deref(), Some("21234567-89AB-CDEF-0123-456789ABCDEF"));
        assert_eq!(resource.active_directory_guid.as_deref(), Some("1111"));
        // Adapter16 (parseResourceType) and the dictionary fields.
        assert_eq!(resource.r#type, Some(ResourceType::Work));
        assert_eq!(resource.work_group, Some(WorkGroup::Email));
        assert_eq!(resource.accrue_at, Some(AccrueType::End));
        assert_eq!(resource.booking_type, Some(BookingType::Proposed));
        // Adapter1 (parseBoolean).
        assert_eq!(resource.is_null, Some(false));
        assert_eq!(resource.over_allocated, Some(true));
        assert_eq!(resource.can_level, Some(true));
        assert_eq!(resource.is_generic, Some(false));
        assert_eq!(resource.is_inactive, Some(false));
        assert_eq!(resource.is_enterprise, Some(true));
        assert_eq!(resource.is_cost_resource, Some(true));
        assert_eq!(resource.is_budget, Some(false));
        assert!(resource.rates_present);
        // Adapter4 (parseDateTime).
        assert_eq!(resource.available_from, Some(date_time(2020, 1, 2, 8, 0, 0)));
        assert_eq!(resource.available_to, Some(date_time(2020, 2, 2, 17, 0, 0)));
        assert_eq!(resource.start, Some(date_time(2020, 1, 2, 8, 0, 0)));
        assert_eq!(resource.finish, Some(date_time(2020, 1, 31, 17, 0, 0)));
        assert_eq!(resource.creation_date, Some(date_time(2020, 3, 4, 5, 6, 7)));
        // Adapter3 / parseString keeps the xsd:duration text as it is.
        assert_eq!(resource.work.as_deref(), Some("PT80H0M0S"));
        assert_eq!(resource.regular_work.as_deref(), Some("PT76H0M0S"));
        assert_eq!(resource.overtime_work.as_deref(), Some("PT4H0M0S"));
        assert_eq!(resource.actual_work.as_deref(), Some("PT40H0M0S"));
        assert_eq!(resource.remaining_work.as_deref(), Some("PT40H0M0S"));
        assert_eq!(resource.actual_overtime_work.as_deref(), Some("PT2H0M0S"));
        assert_eq!(resource.remaining_overtime_work.as_deref(), Some("PT2H0M0S"));
        assert_eq!(resource.actual_work_protected.as_deref(), Some("PT0H0M0S"));
        assert_eq!(resource.actual_overtime_work_protected.as_deref(), Some("PT0H0M0S"));
        // Adapter19 (parsePercentComplete) narrows Integer and Double to f64.
        assert_eq!(resource.percent_work_complete, Some(25.5));
        // Adapter2 (parseDecimal) for the BigDecimal fields.
        assert_eq!(resource.max_units, Some(1.0));
        assert_eq!(resource.peak_units, Some(2.0));
        assert_eq!(resource.standard_rate, Some(12.5));
        assert_eq!(resource.standard_rate_format, Some(2));
        assert_eq!(resource.cost, Some(100.25));
        assert_eq!(resource.overtime_rate, Some(18.75));
        assert_eq!(resource.overtime_rate_format, Some(5));
        assert_eq!(resource.overtime_cost, Some(1.5));
        assert_eq!(resource.cost_per_use, Some(3.5));
        assert_eq!(resource.actual_cost, Some(40.5));
        assert_eq!(resource.actual_overtime_cost, Some(0.75));
        assert_eq!(resource.remaining_cost, Some(59.75));
        assert_eq!(resource.remaining_overtime_cost, Some(0.75));
        assert_eq!(resource.work_variance, Some(4800.0));
        assert_eq!(resource.cost_variance, Some(-1.25));
        assert_eq!(resource.sv, Some(-1.25));
        assert_eq!(resource.cv, Some(-1.5));
        assert_eq!(resource.acwp, Some(40.5));
        assert_eq!(resource.bcws, Some(10.5));
        assert_eq!(resource.bcwp, Some(20.25));
        // BigInteger fields.
        assert_eq!(resource.calendar_uid, Some(4));
        // The nested classes.
        assert_eq!(resource.extended_attribute.len(), 1);
        assert_eq!(resource.baseline.len(), 1);
        assert_eq!(resource.outline_code.len(), 1);
        assert_eq!(resource.availability_periods.len(), 2);
        assert_eq!(resource.rates.len(), 2);
        assert_eq!(resource.timephased_data.len(), 1);
    }

    #[test]
    fn resource_preserves_rates_wrapper_presence() {
        let absent = resource_from("<Resource/>");
        assert!(!absent.rates_present);
        assert!(absent.rates.is_empty());

        let empty = resource_from("<Resource><Rates/></Resource>");
        assert!(empty.rates_present);
        assert!(empty.rates.is_empty());
    }

    /// `ResourceType` (0 = material, 2 = cost, anything else = work), `WorkGroup`, `AccrueType` and
    /// `BookingType` are read through the matching dictionary function; an absent element stays
    /// `None` (which is what the Java field, a nullable enum, holds).
    #[test]
    fn resource_converts_the_dictionary_fields() {
        let material = resource_from("<Resource><UID>1</UID><Type>0</Type><WorkGroup>0</WorkGroup><AccrueAt>1</AccrueAt><BookingType>0</BookingType></Resource>");
        assert_eq!(material.r#type, Some(ResourceType::Material));
        assert_eq!(material.work_group, Some(WorkGroup::Default));
        assert_eq!(material.accrue_at, Some(AccrueType::Start));
        assert_eq!(material.booking_type, Some(BookingType::Committed));

        let cost = resource_from("<Resource><UID>2</UID><Type>2</Type><WorkGroup>1</WorkGroup><AccrueAt>3</AccrueAt></Resource>");
        assert_eq!(cost.r#type, Some(ResourceType::Cost));
        assert_eq!(cost.work_group, Some(WorkGroup::None));
        assert_eq!(cost.accrue_at, Some(AccrueType::Prorated));
        assert_eq!(cost.booking_type, None);

        let web = resource_from("<Resource><UID>3</UID><WorkGroup>3</WorkGroup></Resource>");
        assert_eq!(web.work_group, Some(WorkGroup::Web));

        // Nothing but the UID: every optional field stays `None`.
        let empty = resource_from("<Resource><UID>4</UID></Resource>");
        assert_eq!(empty.r#type, None);
        assert_eq!(empty.work_group, None);
        assert_eq!(empty.accrue_at, None);
        assert_eq!(empty.booking_type, None);
    }

    /// A resource holds one `Rate` element per cost rate table entry, and the flattened list is
    /// empty when the `Rates` wrapper is absent.
    #[test]
    fn resource_reads_cost_rate_table_entries() {
        let resource = resource();

        let first: &Rate = &resource.rates[0];
        assert_eq!(first.rates_from, Some(date_time(2020, 1, 1, 0, 0, 0)));
        assert_eq!(first.rates_to, Some(date_time(2021, 1, 1, 0, 0, 0)));
        assert_eq!(first.rate_table, Some(1));
        assert_eq!(first.standard_rate, Some(25.5));
        assert_eq!(first.standard_rate_format, Some(2));
        assert_eq!(first.overtime_rate, Some(40.0));
        assert_eq!(first.overtime_rate_format, Some(5));
        assert_eq!(first.cost_per_use, Some(3.5));

        // The second entry omits the optional values.
        let second: &Rate = &resource.rates[1];
        assert_eq!(second.rates_from, Some(date_time(2021, 1, 1, 0, 0, 0)));
        assert_eq!(second.rate_table, Some(2));
        assert_eq!(second.standard_rate, Some(30.0));
        assert_eq!(second.standard_rate_format, None);
        assert_eq!(second.overtime_rate, None);
        assert_eq!(second.overtime_rate_format, None);
        assert_eq!(second.cost_per_use, None);

        assert!(resource_from("<Resource><UID>1</UID></Resource>").rates.is_empty());
    }

    /// The `AvailabilityPeriods` wrapper is flattened into a list of `AvailabilityPeriod`.
    #[test]
    fn resource_reads_availability_periods() {
        let resource = resource();

        let first: &AvailabilityPeriod = &resource.availability_periods[0];
        assert_eq!(first.available_from, Some(date_time(2020, 6, 1, 0, 0, 0)));
        assert_eq!(first.available_to, Some(date_time(2020, 6, 30, 0, 0, 0)));
        assert_eq!(first.available_units, Some(0.5));

        let second: &AvailabilityPeriod = &resource.availability_periods[1];
        assert_eq!(second.available_from, Some(date_time(2020, 8, 1, 0, 0, 0)));
        assert_eq!(second.available_to, Some(date_time(2020, 8, 31, 0, 0, 0)));
        assert_eq!(second.available_units, Some(1.0));

        assert!(resource_from("<Resource><UID>1</UID></Resource>").availability_periods.is_empty());
    }

    /// Two `Baseline` elements produce two entries; the second one has an absent optional field.
    #[test]
    fn resource_reads_baselines() {
        let parsed = resource_from(r#"<Resource>
            <UID>1</UID>
            <Baseline>
                <Number>0</Number>
                <Work>PT120H0M0S</Work>
                <Cost>2000.5</Cost>
                <BCWS>1500</BCWS>
                <BCWP>1200</BCWP>
            </Baseline>
            <Baseline>
                <Number>1</Number>
                <Work>PT40H0M0S</Work>
            </Baseline>
        </Resource>"#);

        assert_eq!(parsed.baseline.len(), 2);

        let first: &Baseline = &parsed.baseline[0];
        assert_eq!(first.number, Some(0));
        assert_eq!(first.work.as_deref(), Some("PT120H0M0S"));
        assert_eq!(first.cost, Some(2000.5));
        assert_eq!(first.bcws, Some(1500.0));
        assert_eq!(first.bcwp, Some(1200.0));

        let second: &Baseline = &parsed.baseline[1];
        assert_eq!(second.number, Some(1));
        assert_eq!(second.work.as_deref(), Some("PT40H0M0S"));
        assert_eq!(second.cost, None);
        assert_eq!(second.bcws, None);
        assert_eq!(second.bcwp, None);

        // The fixture itself holds a single baseline.
        let fixture = resource();
        assert_eq!(fixture.baseline.len(), 1);
        assert_eq!(fixture.baseline[0].number, Some(0));
        assert_eq!(fixture.baseline[0].cost, Some(2000.5));
    }

    /// The extended attribute and outline code values are read as a list each; `ValueGUID` of the
    /// outline code is a plain string, while the extended attribute's one is a UUID.
    #[test]
    fn resource_reads_extended_attribute_and_outline_code() {
        let resource = resource();

        assert_eq!(resource.extended_attribute.len(), 1);
        let attribute: &ExtendedAttribute = &resource.extended_attribute[0];
        assert_eq!(attribute.field_id.as_deref(), Some("188743731"));
        assert_eq!(attribute.value.as_deref(), Some("Text value"));
        assert_eq!(
            attribute.value_guid,
            Some(Uuid::parse_str("11234567-89AB-CDEF-0123-456789ABCDEF").expect("valid UUID"))
        );
        assert_eq!(attribute.duration_format, Some(7));

        assert_eq!(resource.outline_code.len(), 1);
        let outline_code: &OutlineCode = &resource.outline_code[0];
        assert_eq!(outline_code.field_id.as_deref(), Some("188743733"));
        assert_eq!(outline_code.value_id, Some(5));
        assert_eq!(outline_code.value_guid.as_deref(), Some("Some code value"));
    }

    /// An absent element leaves a nullable field `None` and an absent repeated element an empty
    /// list, so a resource carrying only a UID parses without a single value.
    #[test]
    fn resource_absent_optional_fields_stay_none() {
        let resource = resource_from("<Resource/>");

        assert_eq!(resource.uid, None);
        assert_eq!(resource.guid, None);
        assert_eq!(resource.name, None);
        assert_eq!(resource.r#type, None);
        assert_eq!(resource.max_units, None);
        assert_eq!(resource.available_from, None);
        assert_eq!(resource.percent_work_complete, None);
        assert_eq!(resource.work, None);
        assert_eq!(resource.booking_type, None);
        assert!(resource.extended_attribute.is_empty());
        assert!(resource.baseline.is_empty());
        assert!(resource.outline_code.is_empty());
        assert!(resource.availability_periods.is_empty());
        assert!(resource.rates.is_empty());
        assert!(resource.timephased_data.is_empty());
    }
}
