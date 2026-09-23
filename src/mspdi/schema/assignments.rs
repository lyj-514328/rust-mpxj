//! Corresponds to Java: `org/mpxj/mspdi/schema/Project.java` (the `Project.Assignments` sub-tree)
//!
//! `org.mpxj.mspdi.MSPDIReader` first unmarshals the MSPDI document into the JAXB generated object
//! tree `org.mpxj.mspdi.schema.Project` and then copies the values into the `org.mpxj` domain
//! objects. This module is the Rust replacement for the JAXB generated classes below
//! `Project.Assignments` (declaration lines 4703 - 13442 of `Project.java`): it reads the same
//! values straight out of a read-only [`roxmltree`] tree with the helpers of
//! [`crate::mspdi::xml_node`], converting each value exactly as the class' `@XmlJavaTypeAdapter`
//! does.
//!
//! Rust has no nested classes, so the Java nesting is reproduced with nested modules: one module
//! level per Java nesting level below `Project.Assignments`.
//!
//! # Java name -> Rust path
//!
//! | Java (declaration line) | Rust |
//! |---|---|
//! | `Project.Assignments` (4703) | flattened away, there is no Rust type - see the differences table |
//! | `Project.Assignments.Assignment` (5434) | [`assignment::Assignment`] |
//! | `Project.Assignments.Assignment.Baseline` (12698) | [`assignment::baseline::Baseline`] |
//! | `Project.Assignments.Assignment.ExtendedAttribute` (12984) | [`assignment::extended_attribute::ExtendedAttribute`] |
//!
//! `Baseline` and `ExtendedAttribute` also exist below `Project.Tasks.Task` and below `Resource`,
//! so the module path is what keeps those classes apart:
//! `schema::assignments::assignment::baseline::Baseline` is not `schema::tasks::task::baseline::Baseline`.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | The JAXB unmarshaller (`JAXBContext` + `Unmarshaller` + `ObjectFactory`) builds the object tree element by element | every class has one Rust-specific constructor `from_node`, which reads the child elements by local name with [`crate::mspdi::xml_node`] | The field order of every `from_node` matches the Java field declaration order and each value is converted the way the field's `@XmlJavaTypeAdapter` converts it. `xml_node` matches local names only, which is what `NamespaceFilter` achieved on the Java side by rewriting the namespace URI |
//! | `Project.Assignments` is a class whose only field is `List<Project.Assignments.Assignment> assignment` | flattened away: the parent `Project` declares `Vec<assignment::Assignment>` directly | The wrapper class exists only because JAXB needs one class per `@XmlElement`. Because the field lives in the parent module, locating the `Assignments` **wrapper element** and collecting its `Assignment` children is the parent module's job. Java throws an NPE when `Project.getAssignments()` is dereferenced while `<Assignments>` is absent; Rust yields an empty `Vec` |
//! | `getAssignment()` creates the backing `ArrayList` lazily, so the field stays `null` until the accessor is called | `from_node` always produces an empty `Vec` for an absent repeated element | The Java reader calls the list accessors, which never return `null`, so the observable behaviour is the same |
//! | `BigInteger` fields (`UID`, `TaskUID`, `ResourceUID`, `CostRateTable`, `Delay`, `FinishVariance`, `LevelingDelay`, `LevelingDelayFormat`, `RateScale`, `StartVariance`, `BookingType`, `DurationFormat` of the nested classes) | `i64` | Java has arbitrary precision, MSPDI values are small. Text that is not an integer, or that does not fit `i64`, degrades to `None` instead of throwing `NumberFormatException` (hard convention: library code never panics) |
//! | `xsd:integer` text is parsed after whitespace collapsing (`whiteSpace="collapse"` in the schema) | `str::parse::<i64>()` on the trimmed text | Identical for well-formed documents, tolerant of the surrounding whitespace JAXB would also have ignored |
//! | `BigDecimal` fields (`ActualCost`, `ActualOvertimeCost`, `ACWP`, `Cost`, `CostVariance`, `CV`, `OvertimeCost`, `PeakUnits`, `RemainingCost`, `RemainingOvertimeCost`, `SV`, `Units`, `VAC`, `BCWS`, `BCWP`, `BudgetCost` and the `Baseline` earned value fields) are parsed by `Adapter2` (`jakarta.xml.bind.DatatypeConverter.parseDecimal`) or, when the field carries no adapter (`Cost`, `ActualCost`, `ActualOvertimeCost`, `OvertimeCost`, `RemainingCost`, `RemainingOvertimeCost`), by JAXB's own `xsd:decimal` conversion | `f64` | Both Java paths produce an arbitrary precision decimal; `f64` is the Rust counterpart of the domain layer's `double`. Malformed text degrades to `None` |
//! | `PercentWorkComplete` is a `Number` parsed by `Adapter11` (`DatatypeConverter.parsePercentComplete`) and holds either an `Integer` or a `Double` | `f64` | Both Java branches are narrowed to `f64`; malformed text degrades to `None` |
//! | `Adapter10` / `Adapter13` (`DatatypeConverter.parseUUID`) throws `IllegalArgumentException` on a malformed GUID | `DatatypeConverter::parse_uuid` yields `None` | Graceful degradation, hard convention: library code never panics |
//! | `Adapter1` / `DatatypeConverter.parseBoolean` maps a missing or non-`1` value to `Boolean.FALSE` | the adapter call is only made when the element is present, so an absent element stays `None` | This mirrors the Java field type `Boolean` (null when the element is absent) rather than the adapter's own fallback |
//! | `Adapter4` (`LocalDateTime`) returns `null` when the text cannot be parsed | `DatatypeConverter::parse_date_time` yields `None` | Same observable result |
//! | `LocalDateTime` | `chrono::NaiveDateTime` | The crate-wide mapping of the domain layer (see `src/local_time_range.rs`) |
//! | Every `xsd:duration` value (`ActualOvertimeWork`, `ActualWork`, `OvertimeWork`, `RegularWork`, `RemainingOvertimeWork`, `RemainingWork`, `Work`, `ActualWorkProtected`, `ActualOvertimeWorkProtected`, `BudgetWork`, and `Work` of the nested `Baseline`) goes through `Adapter3` / `DatatypeConverter.parseString`, which returns the text unchanged | kept as `Option<String>`, not converted | `MSPDIReader` converts these values itself (`DatatypeConverter.parseDuration(file, durationFormat, value)`, which needs the project defaults), so the intermediate model must keep the raw text. Deliberately **not** converted to a duration type |
//! | `WorkContour` (a Java class, not an enum: `WorkContour.FLAT` .. `WorkContour.CONTOURED`) is converted by `Adapter12` (`DatatypeConverter.parseWorkContour`) | `Option<crate::work_contour::WorkContour>` | The Rust type is not nullable, so an unrecognised value degrades to `None` |
//! | `String` fields hold the raw character data | `Option<String>` | [`crate::mspdi::xml_node::text`] returns a `Cow`; `from_node` stores the owned form |
//! | `Assignment.Baseline.Number`, `.Start`, `.Finish` and `.Cost` are plain `String` fields (no adapter), unlike the same named fields of `Project.Tasks.Task.Baseline` | `Option<String>` | The two Java classes are distinct and are kept distinct on the Rust side (`assignment::baseline::Baseline` vs `tasks::task::baseline::Baseline`) |
//! | `Assignment.TimephasedData` and `Assignment.Baseline.TimephasedData` are `List<TimephasedDataType>`, the top-level class `Project.TimephasedDataType` | `Vec<crate::mspdi::schema::timephased_data_type::TimephasedDataType>` | The type lives in its own module |
//! | 201 placeholder fields `Object f404000` .. `Object f4040C8` (`protected Object f4040NN;` with `@XmlElement(name = "f4040NN")` only when the name contains a letter, `xsd:anyType`, lines 5502 - 5702) | `Option<String>`, named after the **element local name** (`f404000` .. `f4040c8`) | The Java identifier's mixed case (`f4040Ba`, `f4040Bc`) is the JAXB generator's own invention for the element `f4040ba` / `f4040bc`; naming the Rust field after the element keeps the field name identical to the name `from_node` looks up. JAXB stores `xsd:anyType` as a raw `Object` (the reader never looks at these fields), so the character data is the closest Rust counterpart. Child *elements* inside such a placeholder are not collected |
//!
//! # Skipped Java members + reason
//!
//! - The JAXB generated `getXxx()` / `setXxx()` accessors and the lazily created `ArrayList` behind
//!   the list accessors. The Rust port stores the values in public fields and reads them directly
//!   (the same choice as the rest of the crate); `from_node` fills every `Vec` once.
//! - The `marshal` halves of the adapters. This crate is reader-only (see `AGENTS.md`).
//! - `ObjectFactory`'s factory methods for these classes; they are replaced by the `from_node`
//!   constructors.

use std::borrow::Cow;

use chrono::NaiveDateTime;
use roxmltree::Node;
use uuid::Uuid;

use crate::mspdi::datatype_converter::DatatypeConverter;
use crate::mspdi::schema::timephased_data_type::TimephasedDataType;
use crate::mspdi::xml_node::{child_text, children};
use crate::work_contour::WorkContour;

/// Corresponds to Java: `Project.Assignments.Assignment` (`Project.java` line 5434).
pub mod assignment {
    use super::{
        child_text, children, parse_big_integer, parse_boolean, parse_decimal, DatatypeConverter,
        NaiveDateTime, Node, TimephasedDataType, Uuid, WorkContour,
    };

    /// Corresponds to Java: `Project.Assignments.Assignment` (`Project.java` line 5434).
    ///
    /// The 201 `f404xxx` placeholders of this class are declared after `baseline` and before
    /// `timephased_data`, exactly as in Java; see the differences table.
    #[derive(Debug, Default, Clone)]
    pub struct Assignment {
        /// Corresponds to Java: `@XmlElement(name = "UID", required = true) protected BigInteger uid;`
        pub uid: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "GUID", type = String.class) @XmlJavaTypeAdapter(Adapter10.class) protected UUID guid;`
        pub guid: Option<Uuid>,
        /// Corresponds to Java: `@XmlElement(name = "TaskUID") protected BigInteger taskUID;`
        pub task_uid: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "ResourceUID") protected BigInteger resourceUID;`
        pub resource_uid: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "PercentWorkComplete", type = String.class) @XmlJavaTypeAdapter(Adapter11.class) @XmlSchemaType(name = "integer") protected Number percentWorkComplete;`
        pub percent_work_complete: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "ActualCost") protected BigDecimal actualCost;`
        pub actual_cost: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "ActualFinish", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime actualFinish;`
        pub actual_finish: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "ActualOvertimeCost") protected BigDecimal actualOvertimeCost;`
        pub actual_overtime_cost: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "ActualOvertimeWork") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String actualOvertimeWork;`
        pub actual_overtime_work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "ActualStart", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime actualStart;`
        pub actual_start: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "ActualWork") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String actualWork;`
        pub actual_work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "ACWP", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal acwp;`
        pub acwp: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "Confirmed", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean confirmed;`
        pub confirmed: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "Cost") protected BigDecimal cost;`
        pub cost: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "CostRateTable") protected BigInteger costRateTable;`
        pub cost_rate_table: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "CostVariance", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal costVariance;`
        pub cost_variance: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "CV", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal cv;`
        pub cv: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "Delay") protected BigInteger delay;`
        pub delay: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "Finish", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime finish;`
        pub finish: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "FinishVariance") protected BigInteger finishVariance;`
        pub finish_variance: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "Hyperlink") protected String hyperlink;`
        pub hyperlink: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "HyperlinkAddress") protected String hyperlinkAddress;`
        pub hyperlink_address: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "HyperlinkSubAddress") protected String hyperlinkSubAddress;`
        pub hyperlink_sub_address: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "WorkVariance", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal workVariance;`
        pub work_variance: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "HasFixedRateUnits", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean hasFixedRateUnits;`
        pub has_fixed_rate_units: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "FixedMaterial", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean fixedMaterial;`
        pub fixed_material: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "LevelingDelay") protected BigInteger levelingDelay;`
        pub leveling_delay: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "LevelingDelayFormat") protected BigInteger levelingDelayFormat;`
        pub leveling_delay_format: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "LinkedFields", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean linkedFields;`
        pub linked_fields: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "Milestone", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean milestone;`
        pub milestone: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "Notes") protected String notes;`
        pub notes: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "Overallocated", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean overallocated;`
        pub overallocated: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "OvertimeCost") protected BigDecimal overtimeCost;`
        pub overtime_cost: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "OvertimeWork") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String overtimeWork;`
        pub overtime_work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "PeakUnits", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal peakUnits;`
        pub peak_units: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "RateScale") protected BigInteger rateScale;`
        pub rate_scale: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "RegularWork") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String regularWork;`
        pub regular_work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "RemainingCost") protected BigDecimal remainingCost;`
        pub remaining_cost: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "RemainingOvertimeCost") protected BigDecimal remainingOvertimeCost;`
        pub remaining_overtime_cost: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "RemainingOvertimeWork") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String remainingOvertimeWork;`
        pub remaining_overtime_work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "RemainingWork") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String remainingWork;`
        pub remaining_work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "ResponsePending", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean responsePending;`
        pub response_pending: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "Start", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime start;`
        pub start: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "Stop", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime stop;`
        pub stop: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "Resume", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime resume;`
        pub resume: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "StartVariance") protected BigInteger startVariance;`
        pub start_variance: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "Summary", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean summary;`
        pub summary: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "SV", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal sv;`
        pub sv: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "Units", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal units;`
        pub units: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "UpdateNeeded", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean updateNeeded;`
        pub update_needed: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "VAC", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal vac;`
        pub vac: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "Work") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String work;`
        pub work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "WorkContour", type = String.class) @XmlJavaTypeAdapter(Adapter12.class) protected WorkContour workContour;`
        pub work_contour: Option<WorkContour>,
        /// Corresponds to Java: `@XmlElement(name = "BCWS", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal bcws;`
        pub bcws: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "BCWP", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal bcwp;`
        pub bcwp: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "BookingType") protected BigInteger bookingType;`
        pub booking_type: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "ActualWorkProtected") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String actualWorkProtected;`
        pub actual_work_protected: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "ActualOvertimeWorkProtected") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String actualOvertimeWorkProtected;`
        pub actual_overtime_work_protected: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "CreationDate", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime creationDate;`
        pub creation_date: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "AssnOwner") protected String assnOwner;`
        pub assn_owner: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "AssnOwnerGuid") protected String assnOwnerGuid;`
        pub assn_owner_guid: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "BudgetCost") protected BigDecimal budgetCost;`
        pub budget_cost: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "BudgetWork") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String budgetWork;`
        pub budget_work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "ExtendedAttribute") protected List<Project.Assignments.Assignment.ExtendedAttribute> extendedAttribute;`
        pub extended_attribute: Vec<extended_attribute::ExtendedAttribute>,
        /// Corresponds to Java: `@XmlElement(name = "Baseline") protected List<Project.Assignments.Assignment.Baseline> baseline;`
        pub baseline: Vec<baseline::Baseline>,
        /// Corresponds to Java: `protected Object f404000;`
        pub f404000: Option<String>,
        /// Corresponds to Java: `protected Object f404001;`
        pub f404001: Option<String>,
        /// Corresponds to Java: `protected Object f404002;`
        pub f404002: Option<String>,
        /// Corresponds to Java: `protected Object f404003;`
        pub f404003: Option<String>,
        /// Corresponds to Java: `protected Object f404004;`
        pub f404004: Option<String>,
        /// Corresponds to Java: `protected Object f404005;`
        pub f404005: Option<String>,
        /// Corresponds to Java: `protected Object f404006;`
        pub f404006: Option<String>,
        /// Corresponds to Java: `protected Object f404007;`
        pub f404007: Option<String>,
        /// Corresponds to Java: `protected Object f404008;`
        pub f404008: Option<String>,
        /// Corresponds to Java: `protected Object f404009;`
        pub f404009: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40400a") protected Object f40400A;`
        pub f40400a: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40400b") protected Object f40400B;`
        pub f40400b: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40400c") protected Object f40400C;`
        pub f40400c: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40400d") protected Object f40400D;`
        pub f40400d: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40400e") protected Object f40400E;`
        pub f40400e: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40400f") protected Object f40400F;`
        pub f40400f: Option<String>,
        /// Corresponds to Java: `protected Object f404010;`
        pub f404010: Option<String>,
        /// Corresponds to Java: `protected Object f404011;`
        pub f404011: Option<String>,
        /// Corresponds to Java: `protected Object f404012;`
        pub f404012: Option<String>,
        /// Corresponds to Java: `protected Object f404013;`
        pub f404013: Option<String>,
        /// Corresponds to Java: `protected Object f404014;`
        pub f404014: Option<String>,
        /// Corresponds to Java: `protected Object f404015;`
        pub f404015: Option<String>,
        /// Corresponds to Java: `protected Object f404016;`
        pub f404016: Option<String>,
        /// Corresponds to Java: `protected Object f404017;`
        pub f404017: Option<String>,
        /// Corresponds to Java: `protected Object f404018;`
        pub f404018: Option<String>,
        /// Corresponds to Java: `protected Object f404019;`
        pub f404019: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40401a") protected Object f40401A;`
        pub f40401a: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40401b") protected Object f40401B;`
        pub f40401b: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40401c") protected Object f40401C;`
        pub f40401c: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40401d") protected Object f40401D;`
        pub f40401d: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40401e") protected Object f40401E;`
        pub f40401e: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40401f") protected Object f40401F;`
        pub f40401f: Option<String>,
        /// Corresponds to Java: `protected Object f404020;`
        pub f404020: Option<String>,
        /// Corresponds to Java: `protected Object f404021;`
        pub f404021: Option<String>,
        /// Corresponds to Java: `protected Object f404022;`
        pub f404022: Option<String>,
        /// Corresponds to Java: `protected Object f404023;`
        pub f404023: Option<String>,
        /// Corresponds to Java: `protected Object f404024;`
        pub f404024: Option<String>,
        /// Corresponds to Java: `protected Object f404025;`
        pub f404025: Option<String>,
        /// Corresponds to Java: `protected Object f404026;`
        pub f404026: Option<String>,
        /// Corresponds to Java: `protected Object f404027;`
        pub f404027: Option<String>,
        /// Corresponds to Java: `protected Object f404028;`
        pub f404028: Option<String>,
        /// Corresponds to Java: `protected Object f404029;`
        pub f404029: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40402a") protected Object f40402A;`
        pub f40402a: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40402b") protected Object f40402B;`
        pub f40402b: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40402c") protected Object f40402C;`
        pub f40402c: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40402d") protected Object f40402D;`
        pub f40402d: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40402e") protected Object f40402E;`
        pub f40402e: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40402f") protected Object f40402F;`
        pub f40402f: Option<String>,
        /// Corresponds to Java: `protected Object f404030;`
        pub f404030: Option<String>,
        /// Corresponds to Java: `protected Object f404031;`
        pub f404031: Option<String>,
        /// Corresponds to Java: `protected Object f404032;`
        pub f404032: Option<String>,
        /// Corresponds to Java: `protected Object f404033;`
        pub f404033: Option<String>,
        /// Corresponds to Java: `protected Object f404034;`
        pub f404034: Option<String>,
        /// Corresponds to Java: `protected Object f404035;`
        pub f404035: Option<String>,
        /// Corresponds to Java: `protected Object f404036;`
        pub f404036: Option<String>,
        /// Corresponds to Java: `protected Object f404037;`
        pub f404037: Option<String>,
        /// Corresponds to Java: `protected Object f404038;`
        pub f404038: Option<String>,
        /// Corresponds to Java: `protected Object f404039;`
        pub f404039: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40403a") protected Object f40403A;`
        pub f40403a: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40403b") protected Object f40403B;`
        pub f40403b: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40403c") protected Object f40403C;`
        pub f40403c: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40403d") protected Object f40403D;`
        pub f40403d: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40403e") protected Object f40403E;`
        pub f40403e: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40403f") protected Object f40403F;`
        pub f40403f: Option<String>,
        /// Corresponds to Java: `protected Object f404040;`
        pub f404040: Option<String>,
        /// Corresponds to Java: `protected Object f404041;`
        pub f404041: Option<String>,
        /// Corresponds to Java: `protected Object f404042;`
        pub f404042: Option<String>,
        /// Corresponds to Java: `protected Object f404043;`
        pub f404043: Option<String>,
        /// Corresponds to Java: `protected Object f404044;`
        pub f404044: Option<String>,
        /// Corresponds to Java: `protected Object f404045;`
        pub f404045: Option<String>,
        /// Corresponds to Java: `protected Object f404046;`
        pub f404046: Option<String>,
        /// Corresponds to Java: `protected Object f404047;`
        pub f404047: Option<String>,
        /// Corresponds to Java: `protected Object f404048;`
        pub f404048: Option<String>,
        /// Corresponds to Java: `protected Object f404049;`
        pub f404049: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40404a") protected Object f40404A;`
        pub f40404a: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40404b") protected Object f40404B;`
        pub f40404b: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40404c") protected Object f40404C;`
        pub f40404c: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40404d") protected Object f40404D;`
        pub f40404d: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40404e") protected Object f40404E;`
        pub f40404e: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40404f") protected Object f40404F;`
        pub f40404f: Option<String>,
        /// Corresponds to Java: `protected Object f404050;`
        pub f404050: Option<String>,
        /// Corresponds to Java: `protected Object f404051;`
        pub f404051: Option<String>,
        /// Corresponds to Java: `protected Object f404052;`
        pub f404052: Option<String>,
        /// Corresponds to Java: `protected Object f404053;`
        pub f404053: Option<String>,
        /// Corresponds to Java: `protected Object f404054;`
        pub f404054: Option<String>,
        /// Corresponds to Java: `protected Object f404055;`
        pub f404055: Option<String>,
        /// Corresponds to Java: `protected Object f404056;`
        pub f404056: Option<String>,
        /// Corresponds to Java: `protected Object f404057;`
        pub f404057: Option<String>,
        /// Corresponds to Java: `protected Object f404058;`
        pub f404058: Option<String>,
        /// Corresponds to Java: `protected Object f404059;`
        pub f404059: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40405a") protected Object f40405A;`
        pub f40405a: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40405b") protected Object f40405B;`
        pub f40405b: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40405c") protected Object f40405C;`
        pub f40405c: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40405d") protected Object f40405D;`
        pub f40405d: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40405e") protected Object f40405E;`
        pub f40405e: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40405f") protected Object f40405F;`
        pub f40405f: Option<String>,
        /// Corresponds to Java: `protected Object f404060;`
        pub f404060: Option<String>,
        /// Corresponds to Java: `protected Object f404061;`
        pub f404061: Option<String>,
        /// Corresponds to Java: `protected Object f404062;`
        pub f404062: Option<String>,
        /// Corresponds to Java: `protected Object f404063;`
        pub f404063: Option<String>,
        /// Corresponds to Java: `protected Object f404064;`
        pub f404064: Option<String>,
        /// Corresponds to Java: `protected Object f404065;`
        pub f404065: Option<String>,
        /// Corresponds to Java: `protected Object f404066;`
        pub f404066: Option<String>,
        /// Corresponds to Java: `protected Object f404067;`
        pub f404067: Option<String>,
        /// Corresponds to Java: `protected Object f404068;`
        pub f404068: Option<String>,
        /// Corresponds to Java: `protected Object f404069;`
        pub f404069: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40406a") protected Object f40406A;`
        pub f40406a: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40406b") protected Object f40406B;`
        pub f40406b: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40406c") protected Object f40406C;`
        pub f40406c: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40406d") protected Object f40406D;`
        pub f40406d: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40406e") protected Object f40406E;`
        pub f40406e: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40406f") protected Object f40406F;`
        pub f40406f: Option<String>,
        /// Corresponds to Java: `protected Object f404070;`
        pub f404070: Option<String>,
        /// Corresponds to Java: `protected Object f404071;`
        pub f404071: Option<String>,
        /// Corresponds to Java: `protected Object f404072;`
        pub f404072: Option<String>,
        /// Corresponds to Java: `protected Object f404073;`
        pub f404073: Option<String>,
        /// Corresponds to Java: `protected Object f404074;`
        pub f404074: Option<String>,
        /// Corresponds to Java: `protected Object f404075;`
        pub f404075: Option<String>,
        /// Corresponds to Java: `protected Object f404076;`
        pub f404076: Option<String>,
        /// Corresponds to Java: `protected Object f404077;`
        pub f404077: Option<String>,
        /// Corresponds to Java: `protected Object f404078;`
        pub f404078: Option<String>,
        /// Corresponds to Java: `protected Object f404079;`
        pub f404079: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40407a") protected Object f40407A;`
        pub f40407a: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40407b") protected Object f40407B;`
        pub f40407b: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40407c") protected Object f40407C;`
        pub f40407c: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40407d") protected Object f40407D;`
        pub f40407d: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40407e") protected Object f40407E;`
        pub f40407e: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40407f") protected Object f40407F;`
        pub f40407f: Option<String>,
        /// Corresponds to Java: `protected Object f404080;`
        pub f404080: Option<String>,
        /// Corresponds to Java: `protected Object f404081;`
        pub f404081: Option<String>,
        /// Corresponds to Java: `protected Object f404082;`
        pub f404082: Option<String>,
        /// Corresponds to Java: `protected Object f404083;`
        pub f404083: Option<String>,
        /// Corresponds to Java: `protected Object f404084;`
        pub f404084: Option<String>,
        /// Corresponds to Java: `protected Object f404085;`
        pub f404085: Option<String>,
        /// Corresponds to Java: `protected Object f404086;`
        pub f404086: Option<String>,
        /// Corresponds to Java: `protected Object f404087;`
        pub f404087: Option<String>,
        /// Corresponds to Java: `protected Object f404088;`
        pub f404088: Option<String>,
        /// Corresponds to Java: `protected Object f404089;`
        pub f404089: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40408a") protected Object f40408A;`
        pub f40408a: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40408b") protected Object f40408B;`
        pub f40408b: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40408c") protected Object f40408C;`
        pub f40408c: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40408d") protected Object f40408D;`
        pub f40408d: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40408e") protected Object f40408E;`
        pub f40408e: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40408f") protected Object f40408F;`
        pub f40408f: Option<String>,
        /// Corresponds to Java: `protected Object f404090;`
        pub f404090: Option<String>,
        /// Corresponds to Java: `protected Object f404091;`
        pub f404091: Option<String>,
        /// Corresponds to Java: `protected Object f404092;`
        pub f404092: Option<String>,
        /// Corresponds to Java: `protected Object f404093;`
        pub f404093: Option<String>,
        /// Corresponds to Java: `protected Object f404094;`
        pub f404094: Option<String>,
        /// Corresponds to Java: `protected Object f404095;`
        pub f404095: Option<String>,
        /// Corresponds to Java: `protected Object f404096;`
        pub f404096: Option<String>,
        /// Corresponds to Java: `protected Object f404097;`
        pub f404097: Option<String>,
        /// Corresponds to Java: `protected Object f404098;`
        pub f404098: Option<String>,
        /// Corresponds to Java: `protected Object f404099;`
        pub f404099: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40409a") protected Object f40409A;`
        pub f40409a: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40409b") protected Object f40409B;`
        pub f40409b: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40409c") protected Object f40409C;`
        pub f40409c: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40409d") protected Object f40409D;`
        pub f40409d: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40409e") protected Object f40409E;`
        pub f40409e: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f40409f") protected Object f40409F;`
        pub f40409f: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040a0") protected Object f4040A0;`
        pub f4040a0: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040a1") protected Object f4040A1;`
        pub f4040a1: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040a2") protected Object f4040A2;`
        pub f4040a2: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040a3") protected Object f4040A3;`
        pub f4040a3: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040a4") protected Object f4040A4;`
        pub f4040a4: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040a5") protected Object f4040A5;`
        pub f4040a5: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040a6") protected Object f4040A6;`
        pub f4040a6: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040a7") protected Object f4040A7;`
        pub f4040a7: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040a8") protected Object f4040A8;`
        pub f4040a8: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040a9") protected Object f4040A9;`
        pub f4040a9: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040aa") protected Object f4040Aa;`
        pub f4040aa: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040ab") protected Object f4040Ab;`
        pub f4040ab: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040ac") protected Object f4040Ac;`
        pub f4040ac: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040ad") protected Object f4040Ad;`
        pub f4040ad: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040ae") protected Object f4040Ae;`
        pub f4040ae: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040af") protected Object f4040Af;`
        pub f4040af: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040b0") protected Object f4040B0;`
        pub f4040b0: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040b1") protected Object f4040B1;`
        pub f4040b1: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040b2") protected Object f4040B2;`
        pub f4040b2: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040b3") protected Object f4040B3;`
        pub f4040b3: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040b4") protected Object f4040B4;`
        pub f4040b4: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040b5") protected Object f4040B5;`
        pub f4040b5: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040b6") protected Object f4040B6;`
        pub f4040b6: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040b7") protected Object f4040B7;`
        pub f4040b7: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040b8") protected Object f4040B8;`
        pub f4040b8: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040b9") protected Object f4040B9;`
        pub f4040b9: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040ba") protected Object f4040Ba;`
        pub f4040ba: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040bb") protected Object f4040Bb;`
        pub f4040bb: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040bc") protected Object f4040Bc;`
        pub f4040bc: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040bd") protected Object f4040Bd;`
        pub f4040bd: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040be") protected Object f4040Be;`
        pub f4040be: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040bf") protected Object f4040Bf;`
        pub f4040bf: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040c0") protected Object f4040C0;`
        pub f4040c0: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040c1") protected Object f4040C1;`
        pub f4040c1: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040c2") protected Object f4040C2;`
        pub f4040c2: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040c3") protected Object f4040C3;`
        pub f4040c3: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040c4") protected Object f4040C4;`
        pub f4040c4: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040c5") protected Object f4040C5;`
        pub f4040c5: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040c6") protected Object f4040C6;`
        pub f4040c6: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040c7") protected Object f4040C7;`
        pub f4040c7: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "f4040c8") protected Object f4040C8;`
        pub f4040c8: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "TimephasedData") protected List<TimephasedDataType> timephasedData;`
        pub timephased_data: Vec<TimephasedDataType>,
    }

    impl Assignment {
        /// Rust-specific constructor replacing the JAXB unmarshaller for
        /// `Project.Assignments.Assignment`.
        pub fn from_node(node: Node<'_, '_>) -> Self {
            Self {
                uid: parse_big_integer(child_text(node, "UID")),
                guid: child_text(node, "GUID").and_then(|value| DatatypeConverter::parse_uuid(&value)),
                task_uid: parse_big_integer(child_text(node, "TaskUID")),
                resource_uid: parse_big_integer(child_text(node, "ResourceUID")),
                percent_work_complete: child_text(node, "PercentWorkComplete").and_then(|value| DatatypeConverter::parse_percent_complete(&value)),
                actual_cost: parse_decimal(child_text(node, "ActualCost")),
                actual_finish: child_text(node, "ActualFinish").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                actual_overtime_cost: parse_decimal(child_text(node, "ActualOvertimeCost")),
                actual_overtime_work: child_text(node, "ActualOvertimeWork").map(|value| value.into_owned()),
                actual_start: child_text(node, "ActualStart").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                actual_work: child_text(node, "ActualWork").map(|value| value.into_owned()),
                acwp: parse_decimal(child_text(node, "ACWP")),
                confirmed: parse_boolean(child_text(node, "Confirmed")),
                cost: parse_decimal(child_text(node, "Cost")),
                cost_rate_table: parse_big_integer(child_text(node, "CostRateTable")),
                cost_variance: parse_decimal(child_text(node, "CostVariance")),
                cv: parse_decimal(child_text(node, "CV")),
                delay: parse_big_integer(child_text(node, "Delay")),
                finish: child_text(node, "Finish").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                finish_variance: parse_big_integer(child_text(node, "FinishVariance")),
                hyperlink: child_text(node, "Hyperlink").map(|value| value.into_owned()),
                hyperlink_address: child_text(node, "HyperlinkAddress").map(|value| value.into_owned()),
                hyperlink_sub_address: child_text(node, "HyperlinkSubAddress").map(|value| value.into_owned()),
                work_variance: parse_decimal(child_text(node, "WorkVariance")),
                has_fixed_rate_units: parse_boolean(child_text(node, "HasFixedRateUnits")),
                fixed_material: parse_boolean(child_text(node, "FixedMaterial")),
                leveling_delay: parse_big_integer(child_text(node, "LevelingDelay")),
                leveling_delay_format: parse_big_integer(child_text(node, "LevelingDelayFormat")),
                linked_fields: parse_boolean(child_text(node, "LinkedFields")),
                milestone: parse_boolean(child_text(node, "Milestone")),
                notes: child_text(node, "Notes").map(|value| value.into_owned()),
                overallocated: parse_boolean(child_text(node, "Overallocated")),
                overtime_cost: parse_decimal(child_text(node, "OvertimeCost")),
                overtime_work: child_text(node, "OvertimeWork").map(|value| value.into_owned()),
                peak_units: parse_decimal(child_text(node, "PeakUnits")),
                rate_scale: parse_big_integer(child_text(node, "RateScale")),
                regular_work: child_text(node, "RegularWork").map(|value| value.into_owned()),
                remaining_cost: parse_decimal(child_text(node, "RemainingCost")),
                remaining_overtime_cost: parse_decimal(child_text(node, "RemainingOvertimeCost")),
                remaining_overtime_work: child_text(node, "RemainingOvertimeWork").map(|value| value.into_owned()),
                remaining_work: child_text(node, "RemainingWork").map(|value| value.into_owned()),
                response_pending: parse_boolean(child_text(node, "ResponsePending")),
                start: child_text(node, "Start").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                stop: child_text(node, "Stop").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                resume: child_text(node, "Resume").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                start_variance: parse_big_integer(child_text(node, "StartVariance")),
                summary: parse_boolean(child_text(node, "Summary")),
                sv: parse_decimal(child_text(node, "SV")),
                units: parse_decimal(child_text(node, "Units")),
                update_needed: parse_boolean(child_text(node, "UpdateNeeded")),
                vac: parse_decimal(child_text(node, "VAC")),
                work: child_text(node, "Work").map(|value| value.into_owned()),
                work_contour: child_text(node, "WorkContour").and_then(|value| DatatypeConverter::parse_work_contour(&value)),
                bcws: parse_decimal(child_text(node, "BCWS")),
                bcwp: parse_decimal(child_text(node, "BCWP")),
                booking_type: parse_big_integer(child_text(node, "BookingType")),
                actual_work_protected: child_text(node, "ActualWorkProtected").map(|value| value.into_owned()),
                actual_overtime_work_protected: child_text(node, "ActualOvertimeWorkProtected").map(|value| value.into_owned()),
                creation_date: child_text(node, "CreationDate").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                assn_owner: child_text(node, "AssnOwner").map(|value| value.into_owned()),
                assn_owner_guid: child_text(node, "AssnOwnerGuid").map(|value| value.into_owned()),
                budget_cost: parse_decimal(child_text(node, "BudgetCost")),
                budget_work: child_text(node, "BudgetWork").map(|value| value.into_owned()),
                extended_attribute: children(node, "ExtendedAttribute").map(extended_attribute::ExtendedAttribute::from_node).collect(),
                baseline: children(node, "Baseline").map(baseline::Baseline::from_node).collect(),
                f404000: child_text(node, "f404000").map(|value| value.into_owned()),
                f404001: child_text(node, "f404001").map(|value| value.into_owned()),
                f404002: child_text(node, "f404002").map(|value| value.into_owned()),
                f404003: child_text(node, "f404003").map(|value| value.into_owned()),
                f404004: child_text(node, "f404004").map(|value| value.into_owned()),
                f404005: child_text(node, "f404005").map(|value| value.into_owned()),
                f404006: child_text(node, "f404006").map(|value| value.into_owned()),
                f404007: child_text(node, "f404007").map(|value| value.into_owned()),
                f404008: child_text(node, "f404008").map(|value| value.into_owned()),
                f404009: child_text(node, "f404009").map(|value| value.into_owned()),
                f40400a: child_text(node, "f40400a").map(|value| value.into_owned()),
                f40400b: child_text(node, "f40400b").map(|value| value.into_owned()),
                f40400c: child_text(node, "f40400c").map(|value| value.into_owned()),
                f40400d: child_text(node, "f40400d").map(|value| value.into_owned()),
                f40400e: child_text(node, "f40400e").map(|value| value.into_owned()),
                f40400f: child_text(node, "f40400f").map(|value| value.into_owned()),
                f404010: child_text(node, "f404010").map(|value| value.into_owned()),
                f404011: child_text(node, "f404011").map(|value| value.into_owned()),
                f404012: child_text(node, "f404012").map(|value| value.into_owned()),
                f404013: child_text(node, "f404013").map(|value| value.into_owned()),
                f404014: child_text(node, "f404014").map(|value| value.into_owned()),
                f404015: child_text(node, "f404015").map(|value| value.into_owned()),
                f404016: child_text(node, "f404016").map(|value| value.into_owned()),
                f404017: child_text(node, "f404017").map(|value| value.into_owned()),
                f404018: child_text(node, "f404018").map(|value| value.into_owned()),
                f404019: child_text(node, "f404019").map(|value| value.into_owned()),
                f40401a: child_text(node, "f40401a").map(|value| value.into_owned()),
                f40401b: child_text(node, "f40401b").map(|value| value.into_owned()),
                f40401c: child_text(node, "f40401c").map(|value| value.into_owned()),
                f40401d: child_text(node, "f40401d").map(|value| value.into_owned()),
                f40401e: child_text(node, "f40401e").map(|value| value.into_owned()),
                f40401f: child_text(node, "f40401f").map(|value| value.into_owned()),
                f404020: child_text(node, "f404020").map(|value| value.into_owned()),
                f404021: child_text(node, "f404021").map(|value| value.into_owned()),
                f404022: child_text(node, "f404022").map(|value| value.into_owned()),
                f404023: child_text(node, "f404023").map(|value| value.into_owned()),
                f404024: child_text(node, "f404024").map(|value| value.into_owned()),
                f404025: child_text(node, "f404025").map(|value| value.into_owned()),
                f404026: child_text(node, "f404026").map(|value| value.into_owned()),
                f404027: child_text(node, "f404027").map(|value| value.into_owned()),
                f404028: child_text(node, "f404028").map(|value| value.into_owned()),
                f404029: child_text(node, "f404029").map(|value| value.into_owned()),
                f40402a: child_text(node, "f40402a").map(|value| value.into_owned()),
                f40402b: child_text(node, "f40402b").map(|value| value.into_owned()),
                f40402c: child_text(node, "f40402c").map(|value| value.into_owned()),
                f40402d: child_text(node, "f40402d").map(|value| value.into_owned()),
                f40402e: child_text(node, "f40402e").map(|value| value.into_owned()),
                f40402f: child_text(node, "f40402f").map(|value| value.into_owned()),
                f404030: child_text(node, "f404030").map(|value| value.into_owned()),
                f404031: child_text(node, "f404031").map(|value| value.into_owned()),
                f404032: child_text(node, "f404032").map(|value| value.into_owned()),
                f404033: child_text(node, "f404033").map(|value| value.into_owned()),
                f404034: child_text(node, "f404034").map(|value| value.into_owned()),
                f404035: child_text(node, "f404035").map(|value| value.into_owned()),
                f404036: child_text(node, "f404036").map(|value| value.into_owned()),
                f404037: child_text(node, "f404037").map(|value| value.into_owned()),
                f404038: child_text(node, "f404038").map(|value| value.into_owned()),
                f404039: child_text(node, "f404039").map(|value| value.into_owned()),
                f40403a: child_text(node, "f40403a").map(|value| value.into_owned()),
                f40403b: child_text(node, "f40403b").map(|value| value.into_owned()),
                f40403c: child_text(node, "f40403c").map(|value| value.into_owned()),
                f40403d: child_text(node, "f40403d").map(|value| value.into_owned()),
                f40403e: child_text(node, "f40403e").map(|value| value.into_owned()),
                f40403f: child_text(node, "f40403f").map(|value| value.into_owned()),
                f404040: child_text(node, "f404040").map(|value| value.into_owned()),
                f404041: child_text(node, "f404041").map(|value| value.into_owned()),
                f404042: child_text(node, "f404042").map(|value| value.into_owned()),
                f404043: child_text(node, "f404043").map(|value| value.into_owned()),
                f404044: child_text(node, "f404044").map(|value| value.into_owned()),
                f404045: child_text(node, "f404045").map(|value| value.into_owned()),
                f404046: child_text(node, "f404046").map(|value| value.into_owned()),
                f404047: child_text(node, "f404047").map(|value| value.into_owned()),
                f404048: child_text(node, "f404048").map(|value| value.into_owned()),
                f404049: child_text(node, "f404049").map(|value| value.into_owned()),
                f40404a: child_text(node, "f40404a").map(|value| value.into_owned()),
                f40404b: child_text(node, "f40404b").map(|value| value.into_owned()),
                f40404c: child_text(node, "f40404c").map(|value| value.into_owned()),
                f40404d: child_text(node, "f40404d").map(|value| value.into_owned()),
                f40404e: child_text(node, "f40404e").map(|value| value.into_owned()),
                f40404f: child_text(node, "f40404f").map(|value| value.into_owned()),
                f404050: child_text(node, "f404050").map(|value| value.into_owned()),
                f404051: child_text(node, "f404051").map(|value| value.into_owned()),
                f404052: child_text(node, "f404052").map(|value| value.into_owned()),
                f404053: child_text(node, "f404053").map(|value| value.into_owned()),
                f404054: child_text(node, "f404054").map(|value| value.into_owned()),
                f404055: child_text(node, "f404055").map(|value| value.into_owned()),
                f404056: child_text(node, "f404056").map(|value| value.into_owned()),
                f404057: child_text(node, "f404057").map(|value| value.into_owned()),
                f404058: child_text(node, "f404058").map(|value| value.into_owned()),
                f404059: child_text(node, "f404059").map(|value| value.into_owned()),
                f40405a: child_text(node, "f40405a").map(|value| value.into_owned()),
                f40405b: child_text(node, "f40405b").map(|value| value.into_owned()),
                f40405c: child_text(node, "f40405c").map(|value| value.into_owned()),
                f40405d: child_text(node, "f40405d").map(|value| value.into_owned()),
                f40405e: child_text(node, "f40405e").map(|value| value.into_owned()),
                f40405f: child_text(node, "f40405f").map(|value| value.into_owned()),
                f404060: child_text(node, "f404060").map(|value| value.into_owned()),
                f404061: child_text(node, "f404061").map(|value| value.into_owned()),
                f404062: child_text(node, "f404062").map(|value| value.into_owned()),
                f404063: child_text(node, "f404063").map(|value| value.into_owned()),
                f404064: child_text(node, "f404064").map(|value| value.into_owned()),
                f404065: child_text(node, "f404065").map(|value| value.into_owned()),
                f404066: child_text(node, "f404066").map(|value| value.into_owned()),
                f404067: child_text(node, "f404067").map(|value| value.into_owned()),
                f404068: child_text(node, "f404068").map(|value| value.into_owned()),
                f404069: child_text(node, "f404069").map(|value| value.into_owned()),
                f40406a: child_text(node, "f40406a").map(|value| value.into_owned()),
                f40406b: child_text(node, "f40406b").map(|value| value.into_owned()),
                f40406c: child_text(node, "f40406c").map(|value| value.into_owned()),
                f40406d: child_text(node, "f40406d").map(|value| value.into_owned()),
                f40406e: child_text(node, "f40406e").map(|value| value.into_owned()),
                f40406f: child_text(node, "f40406f").map(|value| value.into_owned()),
                f404070: child_text(node, "f404070").map(|value| value.into_owned()),
                f404071: child_text(node, "f404071").map(|value| value.into_owned()),
                f404072: child_text(node, "f404072").map(|value| value.into_owned()),
                f404073: child_text(node, "f404073").map(|value| value.into_owned()),
                f404074: child_text(node, "f404074").map(|value| value.into_owned()),
                f404075: child_text(node, "f404075").map(|value| value.into_owned()),
                f404076: child_text(node, "f404076").map(|value| value.into_owned()),
                f404077: child_text(node, "f404077").map(|value| value.into_owned()),
                f404078: child_text(node, "f404078").map(|value| value.into_owned()),
                f404079: child_text(node, "f404079").map(|value| value.into_owned()),
                f40407a: child_text(node, "f40407a").map(|value| value.into_owned()),
                f40407b: child_text(node, "f40407b").map(|value| value.into_owned()),
                f40407c: child_text(node, "f40407c").map(|value| value.into_owned()),
                f40407d: child_text(node, "f40407d").map(|value| value.into_owned()),
                f40407e: child_text(node, "f40407e").map(|value| value.into_owned()),
                f40407f: child_text(node, "f40407f").map(|value| value.into_owned()),
                f404080: child_text(node, "f404080").map(|value| value.into_owned()),
                f404081: child_text(node, "f404081").map(|value| value.into_owned()),
                f404082: child_text(node, "f404082").map(|value| value.into_owned()),
                f404083: child_text(node, "f404083").map(|value| value.into_owned()),
                f404084: child_text(node, "f404084").map(|value| value.into_owned()),
                f404085: child_text(node, "f404085").map(|value| value.into_owned()),
                f404086: child_text(node, "f404086").map(|value| value.into_owned()),
                f404087: child_text(node, "f404087").map(|value| value.into_owned()),
                f404088: child_text(node, "f404088").map(|value| value.into_owned()),
                f404089: child_text(node, "f404089").map(|value| value.into_owned()),
                f40408a: child_text(node, "f40408a").map(|value| value.into_owned()),
                f40408b: child_text(node, "f40408b").map(|value| value.into_owned()),
                f40408c: child_text(node, "f40408c").map(|value| value.into_owned()),
                f40408d: child_text(node, "f40408d").map(|value| value.into_owned()),
                f40408e: child_text(node, "f40408e").map(|value| value.into_owned()),
                f40408f: child_text(node, "f40408f").map(|value| value.into_owned()),
                f404090: child_text(node, "f404090").map(|value| value.into_owned()),
                f404091: child_text(node, "f404091").map(|value| value.into_owned()),
                f404092: child_text(node, "f404092").map(|value| value.into_owned()),
                f404093: child_text(node, "f404093").map(|value| value.into_owned()),
                f404094: child_text(node, "f404094").map(|value| value.into_owned()),
                f404095: child_text(node, "f404095").map(|value| value.into_owned()),
                f404096: child_text(node, "f404096").map(|value| value.into_owned()),
                f404097: child_text(node, "f404097").map(|value| value.into_owned()),
                f404098: child_text(node, "f404098").map(|value| value.into_owned()),
                f404099: child_text(node, "f404099").map(|value| value.into_owned()),
                f40409a: child_text(node, "f40409a").map(|value| value.into_owned()),
                f40409b: child_text(node, "f40409b").map(|value| value.into_owned()),
                f40409c: child_text(node, "f40409c").map(|value| value.into_owned()),
                f40409d: child_text(node, "f40409d").map(|value| value.into_owned()),
                f40409e: child_text(node, "f40409e").map(|value| value.into_owned()),
                f40409f: child_text(node, "f40409f").map(|value| value.into_owned()),
                f4040a0: child_text(node, "f4040a0").map(|value| value.into_owned()),
                f4040a1: child_text(node, "f4040a1").map(|value| value.into_owned()),
                f4040a2: child_text(node, "f4040a2").map(|value| value.into_owned()),
                f4040a3: child_text(node, "f4040a3").map(|value| value.into_owned()),
                f4040a4: child_text(node, "f4040a4").map(|value| value.into_owned()),
                f4040a5: child_text(node, "f4040a5").map(|value| value.into_owned()),
                f4040a6: child_text(node, "f4040a6").map(|value| value.into_owned()),
                f4040a7: child_text(node, "f4040a7").map(|value| value.into_owned()),
                f4040a8: child_text(node, "f4040a8").map(|value| value.into_owned()),
                f4040a9: child_text(node, "f4040a9").map(|value| value.into_owned()),
                f4040aa: child_text(node, "f4040aa").map(|value| value.into_owned()),
                f4040ab: child_text(node, "f4040ab").map(|value| value.into_owned()),
                f4040ac: child_text(node, "f4040ac").map(|value| value.into_owned()),
                f4040ad: child_text(node, "f4040ad").map(|value| value.into_owned()),
                f4040ae: child_text(node, "f4040ae").map(|value| value.into_owned()),
                f4040af: child_text(node, "f4040af").map(|value| value.into_owned()),
                f4040b0: child_text(node, "f4040b0").map(|value| value.into_owned()),
                f4040b1: child_text(node, "f4040b1").map(|value| value.into_owned()),
                f4040b2: child_text(node, "f4040b2").map(|value| value.into_owned()),
                f4040b3: child_text(node, "f4040b3").map(|value| value.into_owned()),
                f4040b4: child_text(node, "f4040b4").map(|value| value.into_owned()),
                f4040b5: child_text(node, "f4040b5").map(|value| value.into_owned()),
                f4040b6: child_text(node, "f4040b6").map(|value| value.into_owned()),
                f4040b7: child_text(node, "f4040b7").map(|value| value.into_owned()),
                f4040b8: child_text(node, "f4040b8").map(|value| value.into_owned()),
                f4040b9: child_text(node, "f4040b9").map(|value| value.into_owned()),
                f4040ba: child_text(node, "f4040ba").map(|value| value.into_owned()),
                f4040bb: child_text(node, "f4040bb").map(|value| value.into_owned()),
                f4040bc: child_text(node, "f4040bc").map(|value| value.into_owned()),
                f4040bd: child_text(node, "f4040bd").map(|value| value.into_owned()),
                f4040be: child_text(node, "f4040be").map(|value| value.into_owned()),
                f4040bf: child_text(node, "f4040bf").map(|value| value.into_owned()),
                f4040c0: child_text(node, "f4040c0").map(|value| value.into_owned()),
                f4040c1: child_text(node, "f4040c1").map(|value| value.into_owned()),
                f4040c2: child_text(node, "f4040c2").map(|value| value.into_owned()),
                f4040c3: child_text(node, "f4040c3").map(|value| value.into_owned()),
                f4040c4: child_text(node, "f4040c4").map(|value| value.into_owned()),
                f4040c5: child_text(node, "f4040c5").map(|value| value.into_owned()),
                f4040c6: child_text(node, "f4040c6").map(|value| value.into_owned()),
                f4040c7: child_text(node, "f4040c7").map(|value| value.into_owned()),
                f4040c8: child_text(node, "f4040c8").map(|value| value.into_owned()),
                timephased_data: children(node, "TimephasedData").map(TimephasedDataType::from_node).collect(),
            }
        }
    }

    /// Corresponds to Java: `Project.Assignments.Assignment.Baseline` (`Project.java` line 12698).
    pub mod baseline {
        use super::{child_text, children, parse_decimal, Node, TimephasedDataType};

        /// Corresponds to Java: `Project.Assignments.Assignment.Baseline` (`Project.java` line 12698).
        ///
        /// This is *not* [`crate::mspdi::schema::tasks::task::baseline::Baseline`]: here `Number`,
        /// `Start`, `Finish` and `Cost` are plain `String` fields, and there is no
        /// `DurationFormat` / `EstimatedDuration` / `FixedCost`.
        #[derive(Debug, Default, Clone)]
        pub struct Baseline {
            /// Corresponds to Java: `@XmlElement(name = "TimephasedData") protected List<TimephasedDataType> timephasedData;`
            pub timephased_data: Vec<TimephasedDataType>,
            /// Corresponds to Java: `@XmlElement(name = "Number", required = true) protected String number;`
            pub number: Option<String>,
            /// Corresponds to Java: `@XmlElement(name = "Start") protected String start;`
            pub start: Option<String>,
            /// Corresponds to Java: `@XmlElement(name = "Finish") protected String finish;`
            pub finish: Option<String>,
            /// Corresponds to Java: `@XmlElement(name = "Work") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String work;`
            pub work: Option<String>,
            /// Corresponds to Java: `@XmlElement(name = "Cost") protected String cost;`
            pub cost: Option<String>,
            /// Corresponds to Java: `@XmlElement(name = "BCWS", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal bcws;`
            pub bcws: Option<f64>,
            /// Corresponds to Java: `@XmlElement(name = "BCWP", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal bcwp;`
            pub bcwp: Option<f64>,
        }

        impl Baseline {
            /// Rust-specific constructor replacing the JAXB unmarshaller for
            /// `Project.Assignments.Assignment.Baseline`.
            pub fn from_node(node: Node<'_, '_>) -> Self {
                Self {
                    timephased_data: children(node, "TimephasedData").map(TimephasedDataType::from_node).collect(),
                    number: child_text(node, "Number").map(|value| value.into_owned()),
                    start: child_text(node, "Start").map(|value| value.into_owned()),
                    finish: child_text(node, "Finish").map(|value| value.into_owned()),
                    work: child_text(node, "Work").map(|value| value.into_owned()),
                    cost: child_text(node, "Cost").map(|value| value.into_owned()),
                    bcws: parse_decimal(child_text(node, "BCWS")),
                    bcwp: parse_decimal(child_text(node, "BCWP")),
                }
            }
        }
    }

    /// Corresponds to Java: `Project.Assignments.Assignment.ExtendedAttribute`
    /// (`Project.java` line 12984).
    pub mod extended_attribute {
        use super::{child_text, parse_big_integer, DatatypeConverter, Node, Uuid};

        /// Corresponds to Java: `Project.Assignments.Assignment.ExtendedAttribute`
        /// (`Project.java` line 12984).
        ///
        /// This is *not* [`crate::mspdi::schema::tasks::task::extended_attribute::ExtendedAttribute`]:
        /// the two Java classes are structurally identical but distinct, and their `ValueGUID`
        /// fields use different adapters (`Adapter13` here, `Adapter29` for the task).
        #[derive(Debug, Default, Clone)]
        pub struct ExtendedAttribute {
            /// Corresponds to Java: `@XmlElement(name = "FieldID") protected String fieldID;`
            pub field_id: Option<String>,
            /// Corresponds to Java: `@XmlElement(name = "Value") protected String value;`
            pub value: Option<String>,
            /// Corresponds to Java: `@XmlElement(name = "ValueGUID", type = String.class) @XmlJavaTypeAdapter(Adapter13.class) protected UUID valueGUID;`
            pub value_guid: Option<Uuid>,
            /// Corresponds to Java: `@XmlElement(name = "DurationFormat") protected BigInteger durationFormat;`
            pub duration_format: Option<i64>,
        }

        impl ExtendedAttribute {
            /// Rust-specific constructor replacing the JAXB unmarshaller for
            /// `Project.Assignments.Assignment.ExtendedAttribute`.
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
}

/// Rust-specific: parses the text of a Java `BigInteger` field.
///
/// JAXB parses `xsd:integer` (which has `whiteSpace="collapse"`) into a `BigInteger`; the Rust port
/// narrows it to `i64` and yields `None` for text that is neither empty nor a valid `i64`.
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
    use super::assignment::{baseline, extended_attribute, Assignment};
    use chrono::{NaiveDate, NaiveDateTime};
    use roxmltree::Document;
    use uuid::Uuid;

    /// An assignment with the built-in fields of the MSPDI element (including an `xsd:duration`
    /// kept as a string), one baseline, one extended attribute and one `f404xxx` placeholder.
    const ASSIGNMENT_XML: &str = r#"<Assignment xmlns="http://schemas.microsoft.com/project">
        <UID>2</UID>
        <GUID>31234567-89AB-CDEF-0123-456789ABCDEF</GUID>
        <TaskUID>7</TaskUID>
        <ResourceUID>3</ResourceUID>
        <PercentWorkComplete>40</PercentWorkComplete>
        <ActualCost>25.5</ActualCost>
        <Start>2024-01-02T08:00:00</Start>
        <Finish>2024-01-05T17:00:00</Finish>
        <Work>PT16H0M0S</Work>
        <WorkContour>1</WorkContour>
        <Units>0.5</Units>
        <Confirmed>1</Confirmed>
        <Notes>Assignment note</Notes>
        <f404000>placeholder</f404000>
        <f40400a>placeholder with letter</f40400a>
        <ExtendedAttribute>
            <FieldID>188743731</FieldID>
            <Value>Text value</Value>
            <ValueGUID>41234567-89AB-CDEF-0123-456789ABCDEF</ValueGUID>
            <DurationFormat>7</DurationFormat>
        </ExtendedAttribute>
        <Baseline>
            <TimephasedData><Type>1</Type></TimephasedData>
            <Number>1</Number>
            <Start>2024-01-02T08:00:00</Start>
            <Finish>2024-01-05T17:00:00</Finish>
            <Work>PT16H0M0S</Work>
            <Cost>30.5</Cost>
            <BCWS>1.25</BCWS>
            <BCWP>2.5</BCWP>
        </Baseline>
        <TimephasedData><Type>1</Type></TimephasedData>
    </Assignment>"#;

    fn date_time(year: i32, month: u32, day: u32, hour: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .expect("valid date")
            .and_hms_opt(hour, 0, 0)
            .expect("valid time")
    }

    fn assignment() -> Assignment {
        let doc = Document::parse(ASSIGNMENT_XML).expect("valid XML");
        Assignment::from_node(doc.root_element())
    }

    /// The built-in fields are converted exactly like the Java adapters do, including the
    /// `xsd:duration` value that stays a string.
    #[test]
    fn assignment_reads_scalar_fields() {
        let assignment = assignment();

        // BigInteger fields.
        assert_eq!(assignment.uid, Some(2));
        assert_eq!(assignment.task_uid, Some(7));
        assert_eq!(assignment.resource_uid, Some(3));
        // Adapter10 (parseUUID).
        assert_eq!(
            assignment.guid,
            Some(Uuid::parse_str("31234567-89AB-CDEF-0123-456789ABCDEF").expect("valid UUID"))
        );
        // Adapter11 (parsePercentComplete) narrows the Java Number to f64.
        assert_eq!(assignment.percent_work_complete, Some(40.0));
        // Adapter2 (parseDecimal) for the BigDecimal fields.
        assert_eq!(assignment.actual_cost, Some(25.5));
        assert_eq!(assignment.units, Some(0.5));
        // Adapter4 (parseDateTime).
        assert_eq!(assignment.start, Some(date_time(2024, 1, 2, 8)));
        assert_eq!(assignment.finish, Some(date_time(2024, 1, 5, 17)));
        // Adapter3 / parseString keeps the xsd:duration text as it is.
        assert_eq!(assignment.work.as_deref(), Some("PT16H0M0S"));
        // Adapter12 (parseWorkContour): the XML value is an index into
        // `WorkContourHelper.TYPE_VALUES`, so "1" is `BACK_LOADED`, whose unique ID is 2 (the Rust
        // `WorkContour::flat()` constant is unique ID 1).
        assert_eq!(
            assignment.work_contour.as_ref().and_then(|contour| contour.get_unique_id()),
            Some(2)
        );
        // Adapter1 (parseBoolean) and a String field.
        assert_eq!(assignment.confirmed, Some(true));
        assert_eq!(assignment.notes.as_deref(), Some("Assignment note"));
        // The repeated `TimephasedData` child is collected (the type lives in its own module).
        assert_eq!(assignment.timephased_data.len(), 1);
    }

    /// Every baseline field is read; `Number`, `Start`, `Finish` and `Cost` stay strings here
    /// (unlike the task baseline, which uses `BigInteger` / `LocalDateTime` / `BigDecimal`).
    #[test]
    fn assignment_baseline_is_read() {
        let assignment = assignment();
        let baseline: &baseline::Baseline = &assignment.baseline[0];

        assert_eq!(assignment.baseline.len(), 1);
        assert_eq!(baseline.timephased_data.len(), 1);
        assert_eq!(baseline.number.as_deref(), Some("1"));
        assert_eq!(baseline.start.as_deref(), Some("2024-01-02T08:00:00"));
        assert_eq!(baseline.finish.as_deref(), Some("2024-01-05T17:00:00"));
        assert_eq!(baseline.work.as_deref(), Some("PT16H0M0S"));
        assert_eq!(baseline.cost.as_deref(), Some("30.5"));
        assert_eq!(baseline.bcws, Some(1.25));
        assert_eq!(baseline.bcwp, Some(2.5));
    }

    /// The extended attribute keeps its `UUID` value.
    #[test]
    fn assignment_extended_attribute_is_read() {
        let assignment = assignment();
        let attribute: &extended_attribute::ExtendedAttribute = &assignment.extended_attribute[0];

        assert_eq!(attribute.field_id.as_deref(), Some("188743731"));
        assert_eq!(attribute.value.as_deref(), Some("Text value"));
        assert_eq!(
            attribute.value_guid,
            Some(Uuid::parse_str("41234567-89AB-CDEF-0123-456789ABCDEF").expect("valid UUID"))
        );
        assert_eq!(attribute.duration_format, Some(7));
    }

    /// The `f404xxx` placeholders are read by their element local name; an absent one stays `None`
    /// (every placeholder element of the schema is optional).
    #[test]
    fn placeholder_fields_are_read() {
        let assignment = assignment();

        assert_eq!(assignment.f404000.as_deref(), Some("placeholder"));
        assert_eq!(assignment.f40400a.as_deref(), Some("placeholder with letter"));
        assert_eq!(assignment.f4040c8, None);
    }

    /// Absent optional elements stay `None` and absent repeated elements stay empty.
    #[test]
    fn absent_optional_fields_are_none() {
        let assignment = assignment();

        assert_eq!(assignment.actual_finish, None);
        assert_eq!(assignment.hyperlink, None);
        assert_eq!(assignment.budget_cost, None);
        assert_eq!(assignment.delay, None);
    }
}
