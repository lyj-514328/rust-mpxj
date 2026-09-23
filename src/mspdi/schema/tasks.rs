//! Corresponds to Java: `org/mpxj/mspdi/schema/Project.java` (the `Project.Tasks` sub-tree)
//!
//! `org.mpxj.mspdi.MSPDIReader` first unmarshals the MSPDI document into the JAXB generated object
//! tree `org.mpxj.mspdi.schema.Project` and then copies the values into the `org.mpxj` domain
//! objects. This module is the Rust replacement for the JAXB generated classes below
//! `Project.Tasks` (declaration lines 23075 - 27661 of `Project.java`): it reads the same values
//! straight out of a read-only [`roxmltree`] tree with the helpers of [`crate::mspdi::xml_node`],
//! converting each value exactly as the class' `@XmlJavaTypeAdapter` does.
//!
//! Rust has no nested classes, so the Java nesting is reproduced with nested modules: one module
//! level per Java nesting level below `Project.Tasks`.
//!
//! # Java name -> Rust path
//!
//! | Java (declaration line) | Rust |
//! |---|---|
//! | `Project.Tasks` (23075) | flattened away, there is no Rust type - see the differences table |
//! | `Project.Tasks.Task` (23666) | [`task::Task`] |
//! | `Project.Tasks.Task.Baseline` (26715) | [`task::baseline::Baseline`] |
//! | `Project.Tasks.Task.ExtendedAttribute` (27136) | [`task::extended_attribute::ExtendedAttribute`] |
//! | `Project.Tasks.Task.OutlineCode` (27276) | [`task::outline_code::OutlineCode`] |
//! | `Project.Tasks.Task.PredecessorLink` (27434) | [`task::PredecessorLink`] |
//!
//! `Baseline`, `ExtendedAttribute` and `OutlineCode` also exist below `Resource` and below
//! `Assignments.Assignment`, so the module path is what keeps those classes apart:
//! `schema::tasks::task::baseline::Baseline` is not `schema::assignments::assignment::baseline::Baseline`.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | The JAXB unmarshaller (`JAXBContext` + `Unmarshaller` + `ObjectFactory`) builds the object tree element by element | every class has one Rust-specific constructor `from_node`, which reads the child elements by local name with [`crate::mspdi::xml_node`] | The field order of every `from_node` matches the Java field declaration order and each value is converted the way the field's `@XmlJavaTypeAdapter` converts it. `xml_node` matches local names only, which is what `NamespaceFilter` achieved on the Java side by rewriting the namespace URI |
//! | `Project.Tasks` is a class whose only field is `List<Project.Tasks.Task> task` | flattened away: the parent `Project` declares `Vec<task::Task>` directly | The wrapper class exists only because JAXB needs one class per `@XmlElement`. Because the field lives in the parent module, locating the `Tasks` **wrapper element** and collecting its `Task` children is the parent module's job. Java throws an NPE when `Project.getTasks()` is dereferenced while `<Tasks>` is absent; Rust yields an empty `Vec` |
//! | `getTask()` creates the backing `ArrayList` lazily, so the field stays `null` until the accessor is called | `from_node` always produces an empty `Vec` for an absent repeated element | The Java reader calls the list accessors, which never return `null`, so the observable behaviour is the same |
//! | `BigInteger` fields (`ID`, `OutlineLevel`, `Priority`, `DurationFormat`, `StartVariance`, `FinishVariance`, `FreeSlack`, `TotalSlack`, `StartSlack`, `FinishSlack`, `ConstraintType`, `CalendarUID`, `LevelingDelay`, `LevelingDelayFormat`, `EarnedValueMethod`, `CommitmentType`, the `Baseline` / `ExtendedAttribute` / `OutlineCode` / `PredecessorLink` integers) | `i64` | Java has arbitrary precision, MSPDI values are small. Text that is not an integer, or that does not fit `i64`, degrades to `None` instead of throwing `NumberFormatException` (hard convention: library code never panics) |
//! | `xsd:integer` text is parsed after whitespace collapsing (`whiteSpace="collapse"` in the schema) | `str::parse::<i64>()` on the trimmed text | Identical for well-formed documents, tolerant of the surrounding whitespace JAXB would also have ignored |
//! | `Task.UID` is an `Integer` parsed by `Adapter22` / `DatatypeConverter.parseTaskUID` | `i32` | `Integer.valueOf` throws `NumberFormatException` on malformed text, so unparsable text degrades to `None` |
//! | `BigDecimal` fields (`WorkVariance`, `FixedCost`, `Cost`, `OvertimeCost`, `ActualCost`, `ActualOvertimeCost`, `RemainingCost`, `RemainingOvertimeCost`, `ACWP`, `CV`, `BCWS`, `BCWP` and the `Baseline` costs) are parsed by `Adapter2` (`jakarta.xml.bind.DatatypeConverter.parseDecimal`) or, when the field carries no adapter (`Cost`, `ActualCost`, `ActualOvertimeCost`, `OvertimeCost`, `RemainingCost`, `RemainingOvertimeCost`, `Baseline.Cost`), by JAXB's own `xsd:decimal` conversion | `f64` | Both Java paths produce an arbitrary precision decimal; `f64` is the Rust counterpart of the domain layer's `double`. Malformed text degrades to `None` |
//! | `Number` fields (`PercentComplete`, `PercentWorkComplete`, `PhysicalPercentComplete`) use `Adapter26` / `Adapter27` / `Adapter28` (`DatatypeConverter.parsePercentComplete`) and hold either an `Integer` or a `Double` | `f64` | Both Java branches are narrowed to `f64`; malformed text degrades to `None` |
//! | `Adapter23` / `Adapter29` (`DatatypeConverter.parseUUID`) throws `IllegalArgumentException` on a malformed GUID | `DatatypeConverter::parse_uuid` yields `None` | Graceful degradation, hard convention: library code never panics |
//! | `Adapter1` / `DatatypeConverter.parseBoolean` maps a missing or non-`1` value to `Boolean.FALSE` | the adapter call is only made when the element is present, so an absent element stays `None` | This mirrors the Java field type `Boolean` (null when the element is absent) rather than the adapter's own fallback |
//! | `Adapter4` (`LocalDateTime`) returns `null` when the text cannot be parsed | `DatatypeConverter::parse_date_time` yields `None` | Same observable result |
//! | `LocalDateTime` | `chrono::NaiveDateTime` | The crate-wide mapping of the domain layer (see `src/local_time_range.rs`) |
//! | Every `xsd:duration` value (`Duration`, `Work`, `OvertimeWork`, `ActualDuration`, `ActualWork`, ... `ManualDuration`) is declared as `String` and goes through `Adapter3` / `DatatypeConverter.parseString`, which returns the text unchanged | kept as `Option<String>`, not converted | `MSPDIReader` converts these values itself (`DatatypeConverter.parseDuration(file, durationFormat, value)`, which needs the project defaults), so the intermediate model must keep the raw text and the separate `...Format` field. Deliberately **not** converted to a duration type |
//! | `Adapter24` / `Adapter25` convert the domain enums (`TaskType` for `Type`, `AccrueType` for `FixedCostAccrual`) | `Option<crate::task_type::TaskType>` / `Option<crate::accrue_type::AccrueType>` | Both fields are `Option` because the element may be absent. `Type` passes the converter result through `Option::from`, since Java's `parseTaskType` cannot return `null` (`TaskTypeHelper.getInstance` falls back to `FIXED_WORK`) and so need not be fallible on the Rust side, while `parseAccrueType` can return `null` (for the value `0`) and is combined with `and_then` |
//! | `String` fields hold the raw character data | `Option<String>` | [`crate::mspdi::xml_node::text`] returns a `Cow`; `from_node` stores the owned form |
//! | `Task.Project` is a field of type `Project` (the enclosing JAXB class) | `Option<crate::mspdi::schema::project::Project>` | `MSPDIReader.readTask` keeps this reference in `m_externalProjects` (line 1465), so the nested element is read into the same struct the top level uses; that struct lives in its own module |
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

use crate::accrue_type::AccrueType;
use crate::mspdi::datatype_converter::DatatypeConverter;
use crate::mspdi::schema::project::Project;
use crate::mspdi::schema::timephased_data_type::TimephasedDataType;
use crate::mspdi::xml_node::{child, child_text, children};
use crate::task_type::TaskType;

/// Corresponds to Java: `Project.Tasks.Task` (`Project.java` line 23666).
pub mod task {
    use super::{
        child, child_text, children, parse_big_integer, parse_boolean, parse_decimal, AccrueType,
        DatatypeConverter, NaiveDateTime, Node, Project, TaskType, TimephasedDataType, Uuid,
    };

    /// Corresponds to Java: `Project.Tasks.Task` (`Project.java` line 23666).
    #[derive(Debug, Default, Clone)]
    pub struct Task {
        /// Corresponds to Java: `@XmlElement(name = "UID", required = true, type = String.class) @XmlJavaTypeAdapter(Adapter22.class) @XmlSchemaType(name = "integer") protected Integer uid;`
        pub uid: Option<i32>,
        /// Corresponds to Java: `@XmlElement(name = "GUID", type = String.class) @XmlJavaTypeAdapter(Adapter23.class) protected UUID guid;`
        pub guid: Option<Uuid>,
        /// Corresponds to Java: `@XmlElement(name = "ID") protected BigInteger id;`
        pub id: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "Name") protected String name;`
        pub name: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "Active", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean active;`
        pub active: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "Manual", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean manual;`
        pub manual: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "Type", type = String.class) @XmlJavaTypeAdapter(Adapter24.class) protected TaskType type;`
        ///
        /// `type` is a Rust keyword, hence the raw identifier (the crate-wide convention for
        /// Java's `type`).
        pub r#type: Option<TaskType>,
        /// Corresponds to Java: `@XmlElement(name = "IsNull", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean isNull;`
        pub is_null: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "CreateDate", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime createDate;`
        pub create_date: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "Contact") protected String contact;`
        pub contact: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "WBS") protected String wbs;`
        pub wbs: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "WBSLevel") protected String wbsLevel;`
        pub wbs_level: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "OutlineNumber") protected String outlineNumber;`
        pub outline_number: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "OutlineLevel") protected BigInteger outlineLevel;`
        pub outline_level: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "Priority") protected BigInteger priority;`
        pub priority: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "Start", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime start;`
        pub start: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "Finish", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime finish;`
        pub finish: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "Duration") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String duration;`
        pub duration: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "DurationFormat") protected BigInteger durationFormat;`
        pub duration_format: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "Work") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String work;`
        pub work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "Stop", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime stop;`
        pub stop: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "Resume", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime resume;`
        pub resume: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "ResumeValid", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean resumeValid;`
        pub resume_valid: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "EffortDriven", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean effortDriven;`
        pub effort_driven: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "Recurring", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean recurring;`
        pub recurring: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "OverAllocated", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean overAllocated;`
        pub over_allocated: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "Estimated", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean estimated;`
        pub estimated: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "Milestone", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean milestone;`
        pub milestone: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "Summary", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean summary;`
        pub summary: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "DisplayAsSummary", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean displayAsSummary;`
        pub display_as_summary: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "Critical", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean critical;`
        pub critical: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "IsSubproject", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean isSubproject;`
        pub is_subproject: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "IsSubprojectReadOnly", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean isSubprojectReadOnly;`
        pub is_subproject_read_only: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "SubprojectName") protected String subprojectName;`
        pub subproject_name: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "ExternalTask", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean externalTask;`
        pub external_task: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "ExternalTaskProject") protected String externalTaskProject;`
        pub external_task_project: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "EarlyStart", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime earlyStart;`
        pub early_start: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "EarlyFinish", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime earlyFinish;`
        pub early_finish: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "LateStart", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime lateStart;`
        pub late_start: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "LateFinish", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime lateFinish;`
        pub late_finish: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "StartVariance") protected BigInteger startVariance;`
        pub start_variance: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "FinishVariance") protected BigInteger finishVariance;`
        pub finish_variance: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "WorkVariance", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal workVariance;`
        pub work_variance: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "FreeSlack") protected BigInteger freeSlack;`
        pub free_slack: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "TotalSlack") protected BigInteger totalSlack;`
        pub total_slack: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "StartSlack") protected BigInteger startSlack;`
        pub start_slack: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "FinishSlack") protected BigInteger finishSlack;`
        pub finish_slack: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "FixedCost", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal fixedCost;`
        pub fixed_cost: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "FixedCostAccrual", type = String.class) @XmlJavaTypeAdapter(Adapter25.class) protected AccrueType fixedCostAccrual;`
        pub fixed_cost_accrual: Option<AccrueType>,
        /// Corresponds to Java: `@XmlElement(name = "PercentComplete", type = String.class) @XmlJavaTypeAdapter(Adapter26.class) @XmlSchemaType(name = "integer") protected Number percentComplete;`
        pub percent_complete: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "PercentWorkComplete", type = String.class) @XmlJavaTypeAdapter(Adapter27.class) @XmlSchemaType(name = "integer") protected Number percentWorkComplete;`
        pub percent_work_complete: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "Cost") protected BigDecimal cost;`
        pub cost: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "OvertimeCost") protected BigDecimal overtimeCost;`
        pub overtime_cost: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "OvertimeWork") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String overtimeWork;`
        pub overtime_work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "ActualStart", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime actualStart;`
        pub actual_start: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "ActualFinish", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime actualFinish;`
        pub actual_finish: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "ActualDuration") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String actualDuration;`
        pub actual_duration: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "ActualCost") protected BigDecimal actualCost;`
        pub actual_cost: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "ActualOvertimeCost") protected BigDecimal actualOvertimeCost;`
        pub actual_overtime_cost: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "ActualWork") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String actualWork;`
        pub actual_work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "ActualOvertimeWork") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String actualOvertimeWork;`
        pub actual_overtime_work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "RegularWork") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String regularWork;`
        pub regular_work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "RemainingDuration") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String remainingDuration;`
        pub remaining_duration: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "RemainingCost") protected BigDecimal remainingCost;`
        pub remaining_cost: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "RemainingWork") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String remainingWork;`
        pub remaining_work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "RemainingOvertimeCost") protected BigDecimal remainingOvertimeCost;`
        pub remaining_overtime_cost: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "RemainingOvertimeWork") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String remainingOvertimeWork;`
        pub remaining_overtime_work: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "ACWP", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal acwp;`
        pub acwp: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "CV", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal cv;`
        pub cv: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "ConstraintType") protected BigInteger constraintType;`
        pub constraint_type: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "CalendarUID") protected BigInteger calendarUID;`
        pub calendar_uid: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "ConstraintDate", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime constraintDate;`
        pub constraint_date: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "Deadline", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime deadline;`
        pub deadline: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "LevelAssignments", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean levelAssignments;`
        pub level_assignments: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "LevelingCanSplit", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean levelingCanSplit;`
        pub leveling_can_split: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "LevelingDelay") protected BigInteger levelingDelay;`
        pub leveling_delay: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "LevelingDelayFormat") protected BigInteger levelingDelayFormat;`
        pub leveling_delay_format: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "PreLeveledStart", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime preLeveledStart;`
        pub pre_leveled_start: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "PreLeveledFinish", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime preLeveledFinish;`
        pub pre_leveled_finish: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "Hyperlink") protected String hyperlink;`
        pub hyperlink: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "HyperlinkAddress") protected String hyperlinkAddress;`
        pub hyperlink_address: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "HyperlinkSubAddress") protected String hyperlinkSubAddress;`
        pub hyperlink_sub_address: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "IgnoreResourceCalendar", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean ignoreResourceCalendar;`
        pub ignore_resource_calendar: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "Notes") protected String notes;`
        pub notes: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "HideBar", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean hideBar;`
        pub hide_bar: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "Rollup", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean rollup;`
        pub rollup: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "BCWS", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal bcws;`
        pub bcws: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "BCWP", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal bcwp;`
        pub bcwp: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "PhysicalPercentComplete", type = String.class) @XmlJavaTypeAdapter(Adapter28.class) @XmlSchemaType(name = "integer") protected Number physicalPercentComplete;`
        pub physical_percent_complete: Option<f64>,
        /// Corresponds to Java: `@XmlElement(name = "EarnedValueMethod") protected BigInteger earnedValueMethod;`
        pub earned_value_method: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "PredecessorLink") protected List<Project.Tasks.Task.PredecessorLink> predecessorLink;`
        pub predecessor_link: Vec<PredecessorLink>,
        /// Corresponds to Java: `@XmlElement(name = "ActualWorkProtected") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String actualWorkProtected;`
        pub actual_work_protected: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "ActualOvertimeWorkProtected") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String actualOvertimeWorkProtected;`
        pub actual_overtime_work_protected: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "ExtendedAttribute") protected List<Project.Tasks.Task.ExtendedAttribute> extendedAttribute;`
        pub extended_attribute: Vec<extended_attribute::ExtendedAttribute>,
        /// Corresponds to Java: `@XmlElement(name = "Baseline") protected List<Project.Tasks.Task.Baseline> baseline;`
        pub baseline: Vec<baseline::Baseline>,
        /// Corresponds to Java: `@XmlElement(name = "OutlineCode") protected List<Project.Tasks.Task.OutlineCode> outlineCode;`
        pub outline_code: Vec<outline_code::OutlineCode>,
        /// Corresponds to Java: `@XmlElement(name = "IsPublished", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean isPublished;`
        pub is_published: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "StatusManager") protected String statusManager;`
        pub status_manager: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "CommitmentStart", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime commitmentStart;`
        pub commitment_start: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "CommitmentFinish", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime commitmentFinish;`
        pub commitment_finish: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "CommitmentType") protected BigInteger commitmentType;`
        pub commitment_type: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "StartText") protected String startText;`
        pub start_text: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "FinishText") protected String finishText;`
        pub finish_text: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "DurationText") protected String durationText;`
        pub duration_text: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "ManualStart", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime manualStart;`
        pub manual_start: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "ManualFinish", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime manualFinish;`
        pub manual_finish: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "ManualDuration") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String manualDuration;`
        pub manual_duration: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "TimephasedData") protected List<TimephasedDataType> timephasedData;`
        pub timephased_data: Vec<TimephasedDataType>,
        /// Corresponds to Java: `@XmlElement(name = "Project") protected Project project;`
        ///
        /// The Java type is the enclosing JAXB class `org.mpxj.mspdi.schema.Project`, which the
        /// Rust port keeps in its own module.
        pub project: Option<Project>,
    }

    impl Task {
        /// Rust-specific constructor replacing the JAXB unmarshaller for `Project.Tasks.Task`.
        pub fn from_node(node: Node<'_, '_>) -> Self {
            Self {
                uid: child_text(node, "UID").and_then(|value| DatatypeConverter::parse_task_uid(&value)),
                guid: child_text(node, "GUID").and_then(|value| DatatypeConverter::parse_uuid(&value)),
                id: parse_big_integer(child_text(node, "ID")),
                name: child_text(node, "Name").map(|value| value.into_owned()),
                active: parse_boolean(child_text(node, "Active")),
                manual: parse_boolean(child_text(node, "Manual")),
                r#type: child_text(node, "Type")
                    .and_then(|value| DatatypeConverter::parse_task_type(&value)),
                is_null: parse_boolean(child_text(node, "IsNull")),
                create_date: child_text(node, "CreateDate").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                contact: child_text(node, "Contact").map(|value| value.into_owned()),
                wbs: child_text(node, "WBS").map(|value| value.into_owned()),
                wbs_level: child_text(node, "WBSLevel").map(|value| value.into_owned()),
                outline_number: child_text(node, "OutlineNumber").map(|value| value.into_owned()),
                outline_level: parse_big_integer(child_text(node, "OutlineLevel")),
                priority: parse_big_integer(child_text(node, "Priority")),
                start: child_text(node, "Start").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                finish: child_text(node, "Finish").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                duration: child_text(node, "Duration").map(|value| value.into_owned()),
                duration_format: parse_big_integer(child_text(node, "DurationFormat")),
                work: child_text(node, "Work").map(|value| value.into_owned()),
                stop: child_text(node, "Stop").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                resume: child_text(node, "Resume").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                resume_valid: parse_boolean(child_text(node, "ResumeValid")),
                effort_driven: parse_boolean(child_text(node, "EffortDriven")),
                recurring: parse_boolean(child_text(node, "Recurring")),
                over_allocated: parse_boolean(child_text(node, "OverAllocated")),
                estimated: parse_boolean(child_text(node, "Estimated")),
                milestone: parse_boolean(child_text(node, "Milestone")),
                summary: parse_boolean(child_text(node, "Summary")),
                display_as_summary: parse_boolean(child_text(node, "DisplayAsSummary")),
                critical: parse_boolean(child_text(node, "Critical")),
                is_subproject: parse_boolean(child_text(node, "IsSubproject")),
                is_subproject_read_only: parse_boolean(child_text(node, "IsSubprojectReadOnly")),
                subproject_name: child_text(node, "SubprojectName").map(|value| value.into_owned()),
                external_task: parse_boolean(child_text(node, "ExternalTask")),
                external_task_project: child_text(node, "ExternalTaskProject").map(|value| value.into_owned()),
                early_start: child_text(node, "EarlyStart").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                early_finish: child_text(node, "EarlyFinish").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                late_start: child_text(node, "LateStart").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                late_finish: child_text(node, "LateFinish").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                start_variance: parse_big_integer(child_text(node, "StartVariance")),
                finish_variance: parse_big_integer(child_text(node, "FinishVariance")),
                work_variance: parse_decimal(child_text(node, "WorkVariance")),
                free_slack: parse_big_integer(child_text(node, "FreeSlack")),
                total_slack: parse_big_integer(child_text(node, "TotalSlack")),
                start_slack: parse_big_integer(child_text(node, "StartSlack")),
                finish_slack: parse_big_integer(child_text(node, "FinishSlack")),
                fixed_cost: parse_decimal(child_text(node, "FixedCost")),
                fixed_cost_accrual: child_text(node, "FixedCostAccrual").and_then(|value| DatatypeConverter::parse_accrue_type(&value)),
                percent_complete: child_text(node, "PercentComplete").and_then(|value| DatatypeConverter::parse_percent_complete(&value)),
                percent_work_complete: child_text(node, "PercentWorkComplete").and_then(|value| DatatypeConverter::parse_percent_complete(&value)),
                cost: parse_decimal(child_text(node, "Cost")),
                overtime_cost: parse_decimal(child_text(node, "OvertimeCost")),
                overtime_work: child_text(node, "OvertimeWork").map(|value| value.into_owned()),
                actual_start: child_text(node, "ActualStart").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                actual_finish: child_text(node, "ActualFinish").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                actual_duration: child_text(node, "ActualDuration").map(|value| value.into_owned()),
                actual_cost: parse_decimal(child_text(node, "ActualCost")),
                actual_overtime_cost: parse_decimal(child_text(node, "ActualOvertimeCost")),
                actual_work: child_text(node, "ActualWork").map(|value| value.into_owned()),
                actual_overtime_work: child_text(node, "ActualOvertimeWork").map(|value| value.into_owned()),
                regular_work: child_text(node, "RegularWork").map(|value| value.into_owned()),
                remaining_duration: child_text(node, "RemainingDuration").map(|value| value.into_owned()),
                remaining_cost: parse_decimal(child_text(node, "RemainingCost")),
                remaining_work: child_text(node, "RemainingWork").map(|value| value.into_owned()),
                remaining_overtime_cost: parse_decimal(child_text(node, "RemainingOvertimeCost")),
                remaining_overtime_work: child_text(node, "RemainingOvertimeWork").map(|value| value.into_owned()),
                acwp: parse_decimal(child_text(node, "ACWP")),
                cv: parse_decimal(child_text(node, "CV")),
                constraint_type: parse_big_integer(child_text(node, "ConstraintType")),
                calendar_uid: parse_big_integer(child_text(node, "CalendarUID")),
                constraint_date: child_text(node, "ConstraintDate").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                deadline: child_text(node, "Deadline").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                level_assignments: parse_boolean(child_text(node, "LevelAssignments")),
                leveling_can_split: parse_boolean(child_text(node, "LevelingCanSplit")),
                leveling_delay: parse_big_integer(child_text(node, "LevelingDelay")),
                leveling_delay_format: parse_big_integer(child_text(node, "LevelingDelayFormat")),
                pre_leveled_start: child_text(node, "PreLeveledStart").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                pre_leveled_finish: child_text(node, "PreLeveledFinish").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                hyperlink: child_text(node, "Hyperlink").map(|value| value.into_owned()),
                hyperlink_address: child_text(node, "HyperlinkAddress").map(|value| value.into_owned()),
                hyperlink_sub_address: child_text(node, "HyperlinkSubAddress").map(|value| value.into_owned()),
                ignore_resource_calendar: parse_boolean(child_text(node, "IgnoreResourceCalendar")),
                notes: child_text(node, "Notes").map(|value| value.into_owned()),
                hide_bar: parse_boolean(child_text(node, "HideBar")),
                rollup: parse_boolean(child_text(node, "Rollup")),
                bcws: parse_decimal(child_text(node, "BCWS")),
                bcwp: parse_decimal(child_text(node, "BCWP")),
                physical_percent_complete: child_text(node, "PhysicalPercentComplete").and_then(|value| DatatypeConverter::parse_percent_complete(&value)),
                earned_value_method: parse_big_integer(child_text(node, "EarnedValueMethod")),
                predecessor_link: children(node, "PredecessorLink").map(PredecessorLink::from_node).collect(),
                actual_work_protected: child_text(node, "ActualWorkProtected").map(|value| value.into_owned()),
                actual_overtime_work_protected: child_text(node, "ActualOvertimeWorkProtected").map(|value| value.into_owned()),
                extended_attribute: children(node, "ExtendedAttribute").map(extended_attribute::ExtendedAttribute::from_node).collect(),
                baseline: children(node, "Baseline").map(baseline::Baseline::from_node).collect(),
                outline_code: children(node, "OutlineCode").map(outline_code::OutlineCode::from_node).collect(),
                is_published: parse_boolean(child_text(node, "IsPublished")),
                status_manager: child_text(node, "StatusManager").map(|value| value.into_owned()),
                commitment_start: child_text(node, "CommitmentStart").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                commitment_finish: child_text(node, "CommitmentFinish").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                commitment_type: parse_big_integer(child_text(node, "CommitmentType")),
                start_text: child_text(node, "StartText").map(|value| value.into_owned()),
                finish_text: child_text(node, "FinishText").map(|value| value.into_owned()),
                duration_text: child_text(node, "DurationText").map(|value| value.into_owned()),
                manual_start: child_text(node, "ManualStart").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                manual_finish: child_text(node, "ManualFinish").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                manual_duration: child_text(node, "ManualDuration").map(|value| value.into_owned()),
                timephased_data: children(node, "TimephasedData").map(TimephasedDataType::from_node).collect(),
                project: child(node, "Project").map(Project::from_node),
            }
        }
    }

    /// Corresponds to Java: `Project.Tasks.Task.Baseline` (`Project.java` line 26715).
    pub mod baseline {
        use super::{child_text, children, parse_big_integer, parse_boolean, parse_decimal, DatatypeConverter, NaiveDateTime, Node, TimephasedDataType};

        /// Corresponds to Java: `Project.Tasks.Task.Baseline` (`Project.java` line 26715).
        ///
        /// This is *not* [`crate::mspdi::schema::assignments::assignment::baseline::Baseline`]:
        /// the assignment baseline stores `Number`, `Start`, `Finish` and `Cost` as plain strings,
        /// while this class uses `BigInteger` / `LocalDateTime` / `BigDecimal`.
        #[derive(Debug, Default, Clone)]
        pub struct Baseline {
            /// Corresponds to Java: `@XmlElement(name = "TimephasedData") protected List<TimephasedDataType> timephasedData;`
            pub timephased_data: Vec<TimephasedDataType>,
            /// Corresponds to Java: `@XmlElement(name = "Number") protected BigInteger number;`
            pub number: Option<i64>,
            /// Corresponds to Java: `@XmlElement(name = "Interim", type = String.class, defaultValue = "false") @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean interim;`
            pub interim: Option<bool>,
            /// Corresponds to Java: `@XmlElement(name = "Start", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime start;`
            pub start: Option<NaiveDateTime>,
            /// Corresponds to Java: `@XmlElement(name = "Finish", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) @XmlSchemaType(name = "dateTime") protected LocalDateTime finish;`
            pub finish: Option<NaiveDateTime>,
            /// Corresponds to Java: `@XmlElement(name = "Duration") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String duration;`
            pub duration: Option<String>,
            /// Corresponds to Java: `@XmlElement(name = "DurationFormat") protected BigInteger durationFormat;`
            pub duration_format: Option<i64>,
            /// Corresponds to Java: `@XmlElement(name = "EstimatedDuration", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean estimatedDuration;`
            pub estimated_duration: Option<bool>,
            /// Corresponds to Java: `@XmlElement(name = "Work") @XmlJavaTypeAdapter(Adapter3.class) @XmlSchemaType(name = "duration") protected String work;`
            pub work: Option<String>,
            /// Corresponds to Java: `@XmlElement(name = "Cost") protected BigDecimal cost;`
            pub cost: Option<f64>,
            /// Corresponds to Java: `@XmlElement(name = "BCWS", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal bcws;`
            pub bcws: Option<f64>,
            /// Corresponds to Java: `@XmlElement(name = "BCWP", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal bcwp;`
            pub bcwp: Option<f64>,
            /// Corresponds to Java: `@XmlElement(name = "FixedCost", type = String.class) @XmlJavaTypeAdapter(Adapter2.class) @XmlSchemaType(name = "float") protected BigDecimal fixedCost;`
            pub fixed_cost: Option<f64>,
        }

        impl Baseline {
            /// Rust-specific constructor replacing the JAXB unmarshaller for
            /// `Project.Tasks.Task.Baseline`.
            pub fn from_node(node: Node<'_, '_>) -> Self {
                Self {
                    timephased_data: children(node, "TimephasedData").map(TimephasedDataType::from_node).collect(),
                    number: parse_big_integer(child_text(node, "Number")),
                    interim: parse_boolean(child_text(node, "Interim")),
                    start: child_text(node, "Start").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                    finish: child_text(node, "Finish").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                    duration: child_text(node, "Duration").map(|value| value.into_owned()),
                    duration_format: parse_big_integer(child_text(node, "DurationFormat")),
                    estimated_duration: parse_boolean(child_text(node, "EstimatedDuration")),
                    work: child_text(node, "Work").map(|value| value.into_owned()),
                    cost: parse_decimal(child_text(node, "Cost")),
                    bcws: parse_decimal(child_text(node, "BCWS")),
                    bcwp: parse_decimal(child_text(node, "BCWP")),
                    fixed_cost: parse_decimal(child_text(node, "FixedCost")),
                }
            }
        }
    }

    /// Corresponds to Java: `Project.Tasks.Task.ExtendedAttribute` (`Project.java` line 27136).
    pub mod extended_attribute {
        use super::{child_text, parse_big_integer, DatatypeConverter, Node, Uuid};

        /// Corresponds to Java: `Project.Tasks.Task.ExtendedAttribute` (`Project.java` line 27136).
        ///
        /// This is *not* [`crate::mspdi::schema::assignments::assignment::extended_attribute::ExtendedAttribute`]:
        /// the two Java classes are structurally identical but distinct, and their `ValueGUID`
        /// fields use different adapters (`Adapter29` here, `Adapter13` for the assignment).
        #[derive(Debug, Default, Clone)]
        pub struct ExtendedAttribute {
            /// Corresponds to Java: `@XmlElement(name = "FieldID") protected String fieldID;`
            pub field_id: Option<String>,
            /// Corresponds to Java: `@XmlElement(name = "Value") protected String value;`
            pub value: Option<String>,
            /// Corresponds to Java: `@XmlElement(name = "ValueGUID", type = String.class) @XmlJavaTypeAdapter(Adapter29.class) protected UUID valueGUID;`
            pub value_guid: Option<Uuid>,
            /// Corresponds to Java: `@XmlElement(name = "DurationFormat") protected BigInteger durationFormat;`
            pub duration_format: Option<i64>,
        }

        impl ExtendedAttribute {
            /// Rust-specific constructor replacing the JAXB unmarshaller for
            /// `Project.Tasks.Task.ExtendedAttribute`.
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

    /// Corresponds to Java: `Project.Tasks.Task.OutlineCode` (`Project.java` line 27276).
    pub mod outline_code {
        use super::{child_text, parse_big_integer, Node};

        /// Corresponds to Java: `Project.Tasks.Task.OutlineCode` (`Project.java` line 27276).
        ///
        /// `ValueGUID` is a plain `String` here (there is no adapter on the Java field), unlike the
        /// `ValueGUID` of [`super::extended_attribute::ExtendedAttribute`].
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
            /// `Project.Tasks.Task.OutlineCode`.
            pub fn from_node(node: Node<'_, '_>) -> Self {
                Self {
                    field_id: child_text(node, "FieldID").map(|value| value.into_owned()),
                    value_id: parse_big_integer(child_text(node, "ValueID")),
                    value_guid: child_text(node, "ValueGUID").map(|value| value.into_owned()),
                }
            }
        }
    }

    /// Corresponds to Java: `Project.Tasks.Task.PredecessorLink` (`Project.java` line 27434).
    #[derive(Debug, Default, Clone)]
    pub struct PredecessorLink {
        /// Corresponds to Java: `@XmlElement(name = "PredecessorUID") protected BigInteger predecessorUID;`
        pub predecessor_uid: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "Type") protected BigInteger type;`
        ///
        /// `type` is a Rust keyword, hence the raw identifier (the crate-wide convention for
        /// Java's `type`).
        pub r#type: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "CrossProject", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) @XmlSchemaType(name = "boolean") protected Boolean crossProject;`
        pub cross_project: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "CrossProjectName") protected String crossProjectName;`
        pub cross_project_name: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "LinkLag") protected BigInteger linkLag;`
        pub link_lag: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "LagFormat") protected BigInteger lagFormat;`
        pub lag_format: Option<i64>,
    }

    impl PredecessorLink {
        /// Rust-specific constructor replacing the JAXB unmarshaller for
        /// `Project.Tasks.Task.PredecessorLink`.
        pub fn from_node(node: Node<'_, '_>) -> Self {
            Self {
                predecessor_uid: parse_big_integer(child_text(node, "PredecessorUID")),
                r#type: parse_big_integer(child_text(node, "Type")),
                cross_project: parse_boolean(child_text(node, "CrossProject")),
                cross_project_name: child_text(node, "CrossProjectName").map(|value| value.into_owned()),
                link_lag: parse_big_integer(child_text(node, "LinkLag")),
                lag_format: parse_big_integer(child_text(node, "LagFormat")),
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
    use super::task::{baseline, extended_attribute, outline_code, PredecessorLink, Task};
    use crate::accrue_type::AccrueType;
    use crate::task_type::TaskType;
    use chrono::{NaiveDate, NaiveDateTime};
    use roxmltree::Document;
    use uuid::Uuid;

    /// A task with every part of the sub-tree populated: scalars (including an `xsd:duration`
    /// kept as a string and a decimal percent complete), one baseline, one predecessor link, one
    /// extended attribute, one outline code and one `f404xxx` placeholder.
    const TASK_XML: &str = r#"<Task xmlns="http://schemas.microsoft.com/project">
        <UID>7</UID>
        <GUID>01234567-89AB-CDEF-0123-456789ABCDEF</GUID>
        <ID>3</ID>
        <Name>Design</Name>
        <Active>1</Active>
        <Manual>0</Manual>
        <Type>1</Type>
        <Start>2024-01-02T08:00:00</Start>
        <Finish>2024-01-05T17:00:00</Finish>
        <Duration>PT24H0M0S</Duration>
        <DurationFormat>7</DurationFormat>
        <PercentComplete>50</PercentComplete>
        <PercentWorkComplete>25.5</PercentWorkComplete>
        <FixedCostAccrual>2</FixedCostAccrual>
        <FixedCost>12.5</FixedCost>
        <Baseline>
            <TimephasedData><Type>1</Type></TimephasedData>
            <Number>0</Number>
            <Interim>0</Interim>
            <Start>2024-01-02T08:00:00</Start>
            <Finish>2024-01-05T17:00:00</Finish>
            <Duration>PT24H0M0S</Duration>
            <DurationFormat>7</DurationFormat>
            <EstimatedDuration>1</EstimatedDuration>
            <Work>PT24H0M0S</Work>
            <Cost>100</Cost>
            <BCWS>10.5</BCWS>
            <BCWP>20.25</BCWP>
            <FixedCost>1.5</FixedCost>
        </Baseline>
        <PredecessorLink>
            <PredecessorUID>4</PredecessorUID>
            <Type>1</Type>
            <CrossProject>1</CrossProject>
            <CrossProjectName>Other.mpp</CrossProjectName>
            <LinkLag>4800</LinkLag>
            <LagFormat>7</LagFormat>
        </PredecessorLink>
        <ExtendedAttribute>
            <FieldID>188743731</FieldID>
            <Value>Text value</Value>
            <ValueGUID>11234567-89AB-CDEF-0123-456789ABCDEF</ValueGUID>
            <DurationFormat>7</DurationFormat>
        </ExtendedAttribute>
        <OutlineCode>
            <FieldID>188744000</FieldID>
            <ValueID>2</ValueID>
            <ValueGUID>21234567-89AB-CDEF-0123-456789ABCDEF</ValueGUID>
        </OutlineCode>
    </Task>"#;

    fn date_time(year: i32, month: u32, day: u32, hour: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .expect("valid date")
            .and_hms_opt(hour, 0, 0)
            .expect("valid time")
    }

    fn task() -> Task {
        let doc = Document::parse(TASK_XML).expect("valid XML");
        Task::from_node(doc.root_element())
    }

    /// The scalar fields are converted exactly like the Java adapters do, including the
    /// `xsd:duration` value that stays a string.
    #[test]
    fn task_reads_scalar_fields() {
        let task = task();

        // Adapter22 (parseTaskUID) and Adapter23 (parseUUID).
        assert_eq!(task.uid, Some(7));
        assert_eq!(
            task.guid,
            Some(Uuid::parse_str("01234567-89AB-CDEF-0123-456789ABCDEF").expect("valid UUID"))
        );
        // BigInteger and String fields.
        assert_eq!(task.id, Some(3));
        assert_eq!(task.name.as_deref(), Some("Design"));
        // Adapter1 (parseBoolean).
        assert_eq!(task.active, Some(true));
        assert_eq!(task.manual, Some(false));
        // Adapter24 (parseTaskType) / Adapter25 (parseAccrueType).
        assert_eq!(task.r#type, Some(TaskType::FixedDuration));
        assert_eq!(task.fixed_cost_accrual, Some(AccrueType::End));
        // Adapter4 (parseDateTime).
        assert_eq!(task.start, Some(date_time(2024, 1, 2, 8)));
        assert_eq!(task.finish, Some(date_time(2024, 1, 5, 17)));
        // Adapter3 / parseString keeps the xsd:duration text as it is.
        assert_eq!(task.duration.as_deref(), Some("PT24H0M0S"));
        assert_eq!(task.duration_format, Some(7));
        // Adapter26 / Adapter27 (parsePercentComplete) narrow Integer and Double to f64.
        assert_eq!(task.percent_complete, Some(50.0));
        assert_eq!(task.percent_work_complete, Some(25.5));
        // Adapter2 (parseDecimal) for the BigDecimal fields.
        assert_eq!(task.fixed_cost, Some(12.5));
    }

    /// An `xsd:duration` field is not converted: it keeps the raw text so that `MSPDIReader` can
    /// convert it with the units of the matching `...Format` field.
    #[test]
    fn task_keeps_duration_as_text() {
        let task = task();
        assert_eq!(task.duration.as_deref(), Some("PT24H0M0S"));
        assert_eq!(task.baseline[0].work.as_deref(), Some("PT24H0M0S"));
        assert_eq!(task.baseline[0].duration.as_deref(), Some("PT24H0M0S"));
    }

    /// Every baseline field is read, including its repeated `TimephasedData` child.
    #[test]
    fn baseline_is_read() {
        let task = task();
        let baseline: &baseline::Baseline = &task.baseline[0];

        assert_eq!(task.baseline.len(), 1);
        assert_eq!(baseline.timephased_data.len(), 1);
        assert_eq!(baseline.number, Some(0));
        assert_eq!(baseline.interim, Some(false));
        assert_eq!(baseline.start, Some(date_time(2024, 1, 2, 8)));
        assert_eq!(baseline.finish, Some(date_time(2024, 1, 5, 17)));
        assert_eq!(baseline.duration_format, Some(7));
        assert_eq!(baseline.estimated_duration, Some(true));
        assert_eq!(baseline.cost, Some(100.0));
        assert_eq!(baseline.bcws, Some(10.5));
        assert_eq!(baseline.bcwp, Some(20.25));
        assert_eq!(baseline.fixed_cost, Some(1.5));
    }

    /// Every predecessor link field is read.
    #[test]
    fn predecessor_link_is_read() {
        let task = task();
        let link: &PredecessorLink = &task.predecessor_link[0];

        assert_eq!(task.predecessor_link.len(), 1);
        assert_eq!(link.predecessor_uid, Some(4));
        assert_eq!(link.r#type, Some(1));
        assert_eq!(link.cross_project, Some(true));
        assert_eq!(link.cross_project_name.as_deref(), Some("Other.mpp"));
        assert_eq!(link.link_lag, Some(4800));
        assert_eq!(link.lag_format, Some(7));
    }

    /// An extended attribute keeps its `UUID` value, and the `OutlineCode` of the same name is a
    /// different class whose `ValueGUID` stays a string.
    #[test]
    fn extended_attribute_and_outline_code_are_read() {
        let task = task();
        let attribute: &extended_attribute::ExtendedAttribute = &task.extended_attribute[0];
        let outline: &outline_code::OutlineCode = &task.outline_code[0];

        assert_eq!(attribute.field_id.as_deref(), Some("188743731"));
        assert_eq!(attribute.value.as_deref(), Some("Text value"));
        assert_eq!(
            attribute.value_guid,
            Some(Uuid::parse_str("11234567-89AB-CDEF-0123-456789ABCDEF").expect("valid UUID"))
        );
        assert_eq!(attribute.duration_format, Some(7));

        assert_eq!(outline.field_id.as_deref(), Some("188744000"));
        assert_eq!(outline.value_id, Some(2));
        assert_eq!(outline.value_guid.as_deref(), Some("21234567-89AB-CDEF-0123-456789ABCDEF"));
    }

    /// Absent optional elements stay `None` and absent repeated elements stay empty.
    #[test]
    fn absent_optional_fields_are_none() {
        let task = task();

        assert_eq!(task.notes, None);
        assert_eq!(task.contact, None);
        assert_eq!(task.create_date, None);
        assert_eq!(task.cost, None);
        assert!(task.timephased_data.is_empty());
        assert!(task.project.is_none());
    }
}
