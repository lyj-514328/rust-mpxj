//! Corresponds to Java: `org/mpxj/mspdi/schema/Project.java` (the `Project` root type only).
//!
//! `Project` is the JAXB generated root type of the MSPDI schema: the MSPDI reader unmarshals the
//! XML document into it and then copies the values into the `org.mpxj` domain objects. Rust has no
//! JAXB, so the same object tree is built by hand out of a read-only [`roxmltree::Node`] tree.
//!
//! # Where the nested classes live
//!
//! The Java nested classes referenced by `Project`'s own fields are modelled in separate modules:
//!
//! | Java nested class | Rust path |
//! |---|---|
//! | `Project.Assignments` | [`crate::mspdi::schema::assignments`] (wrapper flattened; `assignments::assignment::Assignment`) |
//! | `Project.Calendars` | [`crate::mspdi::schema::calendars`] (`Calendars`, `Calendar`) |
//! | `Project.ExtendedAttributes` | [`crate::mspdi::schema::extended_attributes`] (`ExtendedAttributes`, `ExtendedAttribute`, `ValueList`, `Value`) |
//! | `Project.OutlineCodes` | [`crate::mspdi::schema::outline_codes`] (`OutlineCodes`, `OutlineCode`, `Masks`, `Mask`, `Values`, `Value`) |
//! | `Project.Resources` | [`crate::mspdi::schema::resources`] (wrapper flattened; `resource::Resource`) |
//! | `Project.Tasks` | [`crate::mspdi::schema::tasks`] (wrapper flattened; `task::Task`) |
//! | `Project.WBSMasks` | [`crate::mspdi::schema::wbs_masks`] (`WBSMasks`, `WBSMask`) |
//! | `TimephasedDataType` (top level) | [`crate::mspdi::schema::timephased_data_type`] (`TimephasedDataType`) |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | JAXB unmarshaller builds `Project` from the XML document | [`Project::from_node`] reads the child elements straight out of a [`roxmltree::Node`] (see [`crate::mspdi::xml_node`]) | Rust has no JAXB; element matching is by local name, in the same order as the Java field declarations |
//! | `Project.Calendars` / `Tasks` / `Resources` / `Assignments` / `ExtendedAttributes` / `OutlineCodes` are element wrapper classes holding a single `List<X>` field | the corresponding [`Project`] fields are `Vec<X>` (flattened) and are filled from the wrapper element's child elements (`Calendars` → `Calendar`, `Tasks` → `Task`, …) | The wrappers exist only because of JAXB's element wrapper structure. Java's `getCalendars()` returns `null` when the wrapper element is absent (the reader guards every access with `!= null`), while Rust yields an empty `Vec`; the resulting iteration is identical. The sibling modules keep a wrapper struct where it has a use (`calendars::Calendars`) and flatten it away otherwise; `resources` additionally offers a `read_resources` helper that performs the same flattening, but this module reads the children directly so that all six wrappers are handled uniformly |
//! | `Project.WBSMasks` is a wrapper class that carries its own `VerifyUniqueCodes` / `GenerateCodes` / `Prefix` fields next to the `WBSMask` list | modelled as its own struct and kept as [`Project::wbs_masks`] `Option<WBSMasks>` (with `wbs_mask: Vec<WBSMask>` inside) | Flattening would have to lift those three fields onto `Project` and would lose the Java field names; the small struct is kept instead |
//! | `BigInteger` | `i64` | Parsed with a plain `i64` parse (there is no converter in [`DatatypeConverter`] because Java's adapter is the generic JAXB `xsd:integer` binding). A value that does not parse yields `None` instead of throwing the way JAXB would |
//! | `BigDecimal` | `f64` | Java's `Adapter2` is the generic JAXB `xsd:decimal` binding; the Rust model keeps the numeric value only |
//! | `java.time.LocalDateTime` / `LocalTime` | [`chrono::NaiveDateTime`] / [`chrono::NaiveTime`] | Conversion is done by [`DatatypeConverter::parse_date_time`] / [`DatatypeConverter::parse_time`], i.e. the same methods the JAXB adapters `Adapter4` / `Adapter5` call |
//! | `java.util.UUID` | [`uuid::Uuid`] | Conversion is done by [`DatatypeConverter::parse_uuid`], i.e. the method the JAXB adapter `Adapter6` calls |
//! | a present but empty element is handed to the JAXB adapter as `""` | [`crate::mspdi::xml_node::text`] returns `None` for an element with no character data, so the field stays `None` | Rust-specific branch; it keeps an empty element indistinguishable from an absent one instead of panicking |
//! | `getXxx()` / `setXxx()` accessor pairs | public fields | This crate is a reader; the accessors carry no behaviour of their own |
//! | Java field names `microsoftProjectServerURL`, `defaultTaskEVMethod`, `calendarUID`, `fyStartDate` | `microsoft_project_server_url`, `default_task_ev_method`, `calendar_uid`, `fy_start_date` | Mechanical snake_case of the Java identifiers: the acronyms are lower-cased as a whole word, matching the existing repository naming (`ProjectProperties::get_microsoft_project_server_url`) |
//! | the nested classes are declared inside `Project` | declared in their own modules (see the mapping table above) | Rust has no nested types; the coordinator declares the modules in `mod.rs` |
//!
//! # Skipped Java members + reason
//!
//! None. Every field declared by `Project` between lines 2238 and 2312 of the Java source is
//! present, and the getters/setters are replaced by the public fields. There are no
//! `@XmlAttribute` or `@XmlValue` fields on this class, so [`Project::from_node`] only reads child
//! elements.

use chrono::{NaiveDateTime, NaiveTime};
use roxmltree::Node;
use uuid::Uuid;

use crate::accrue_type::AccrueType;
use crate::currency_symbol_position::CurrencySymbolPosition;
use crate::mspdi::datatype_converter::DatatypeConverter;
use crate::mspdi::xml_node;
use crate::task_type::TaskType;

use super::assignments::assignment::Assignment;
use super::calendars::Calendar;
use super::extended_attributes::ExtendedAttribute;
use super::outline_codes::OutlineCode;
use super::resources::resource::Resource;
use super::tasks::task::Task;
use super::wbs_masks::WBSMasks;

/// Rust-specific helper replacing the JAXB binding of `xsd:integer` (Java `BigInteger`).
///
/// XML Schema's whitespace facet for `integer` is "collapse", hence the `trim` before parsing. A
/// value that cannot be parsed yields `None` rather than the `NumberFormatException` JAXB would
/// propagate.
fn parse_integer(value: &str) -> Option<i64> {
    value.trim().parse::<i64>().ok()
}

/// Rust-specific helper replacing the JAXB binding of `xsd:decimal` (Java `BigDecimal`).
///
/// The Rust model keeps the numeric value as an `f64` (the Java reader only ever uses the value);
/// XML Schema's whitespace facet for `decimal` is "collapse", hence the `trim`.
fn parse_decimal(value: &str) -> Option<f64> {
    value.trim().parse::<f64>().ok()
}

/// Corresponds to Java: `public class Project` (the MSPDI root type).
///
/// Only the fields declared by `Project` itself are modelled here; the JAXB nested classes it
/// references live in the modules listed in the file header.
#[derive(Debug, Default, Clone)]
pub struct Project {
    /// Corresponds to Java: `Project.saveVersion` (`@XmlElement(name = "SaveVersion", required = true)`, `BigInteger`).
    pub save_version: Option<i64>,
    /// Corresponds to Java: `Project.uid` (`@XmlElement(name = "UID")`, `String`).
    pub uid: Option<String>,
    /// Corresponds to Java: `Project.name` (`@XmlElement(name = "Name")`, `String`).
    pub name: Option<String>,
    /// Corresponds to Java: `Project.guid` (`@XmlElement(name = "GUID")`, `UUID` via `Adapter6`).
    pub guid: Option<Uuid>,
    /// Corresponds to Java: `Project.title` (`@XmlElement(name = "Title")`, `String`).
    pub title: Option<String>,
    /// Corresponds to Java: `Project.subject` (`@XmlElement(name = "Subject")`, `String`).
    pub subject: Option<String>,
    /// Corresponds to Java: `Project.category` (`@XmlElement(name = "Category")`, `String`).
    pub category: Option<String>,
    /// Corresponds to Java: `Project.company` (`@XmlElement(name = "Company")`, `String`).
    pub company: Option<String>,
    /// Corresponds to Java: `Project.manager` (`@XmlElement(name = "Manager")`, `String`).
    pub manager: Option<String>,
    /// Corresponds to Java: `Project.author` (`@XmlElement(name = "Author")`, `String`).
    pub author: Option<String>,
    /// Corresponds to Java: `Project.creationDate` (`@XmlElement(name = "CreationDate")`, `LocalDateTime` via `Adapter4`).
    pub creation_date: Option<NaiveDateTime>,
    /// Corresponds to Java: `Project.revision` (`@XmlElement(name = "Revision")`, `BigInteger`).
    pub revision: Option<i64>,
    /// Corresponds to Java: `Project.lastSaved` (`@XmlElement(name = "LastSaved")`, `LocalDateTime` via `Adapter4`).
    pub last_saved: Option<NaiveDateTime>,
    /// Corresponds to Java: `Project.scheduleFromStart` (`@XmlElement(name = "ScheduleFromStart", defaultValue = "true")`, `Boolean` via `Adapter1`).
    pub schedule_from_start: Option<bool>,
    /// Corresponds to Java: `Project.startDate` (`@XmlElement(name = "StartDate")`, `LocalDateTime` via `Adapter4`).
    pub start_date: Option<NaiveDateTime>,
    /// Corresponds to Java: `Project.finishDate` (`@XmlElement(name = "FinishDate")`, `LocalDateTime` via `Adapter4`).
    pub finish_date: Option<NaiveDateTime>,
    /// Corresponds to Java: `Project.fyStartDate` (`@XmlElement(name = "FYStartDate")`, `BigInteger`).
    pub fy_start_date: Option<i64>,
    /// Corresponds to Java: `Project.criticalSlackLimit` (`@XmlElement(name = "CriticalSlackLimit")`, `BigInteger`).
    pub critical_slack_limit: Option<i64>,
    /// Corresponds to Java: `Project.currencyDigits` (`@XmlElement(name = "CurrencyDigits")`, `BigInteger`).
    pub currency_digits: Option<i64>,
    /// Corresponds to Java: `Project.currencySymbol` (`@XmlElement(name = "CurrencySymbol")`, `String`).
    pub currency_symbol: Option<String>,
    /// Corresponds to Java: `Project.currencyCode` (`@XmlElement(name = "CurrencyCode", required = true)`, `String`).
    pub currency_code: Option<String>,
    /// Corresponds to Java: `Project.currencySymbolPosition` (`@XmlElement(name = "CurrencySymbolPosition")`, `CurrencySymbolPosition` via `Adapter7`).
    pub currency_symbol_position: Option<CurrencySymbolPosition>,
    /// Corresponds to Java: `Project.calendarUID` (`@XmlElement(name = "CalendarUID")`, `BigInteger`).
    pub calendar_uid: Option<i64>,
    /// Corresponds to Java: `Project.defaultStartTime` (`@XmlElement(name = "DefaultStartTime")`, `LocalTime` via `Adapter5`).
    pub default_start_time: Option<NaiveTime>,
    /// Corresponds to Java: `Project.defaultFinishTime` (`@XmlElement(name = "DefaultFinishTime")`, `LocalTime` via `Adapter5`).
    pub default_finish_time: Option<NaiveTime>,
    /// Corresponds to Java: `Project.minutesPerDay` (`@XmlElement(name = "MinutesPerDay")`, `BigInteger`).
    pub minutes_per_day: Option<i64>,
    /// Corresponds to Java: `Project.minutesPerWeek` (`@XmlElement(name = "MinutesPerWeek")`, `BigInteger`).
    pub minutes_per_week: Option<i64>,
    /// Corresponds to Java: `Project.daysPerMonth` (`@XmlElement(name = "DaysPerMonth")`, `BigInteger`).
    pub days_per_month: Option<i64>,
    /// Corresponds to Java: `Project.defaultTaskType` (`@XmlElement(name = "DefaultTaskType", defaultValue = "1")`, `TaskType` via `Adapter8`).
    pub default_task_type: Option<TaskType>,
    /// Corresponds to Java: `Project.defaultFixedCostAccrual` (`@XmlElement(name = "DefaultFixedCostAccrual")`, `AccrueType` via `Adapter9`).
    pub default_fixed_cost_accrual: Option<AccrueType>,
    /// Corresponds to Java: `Project.defaultStandardRate` (`@XmlElement(name = "DefaultStandardRate")`, `BigDecimal` via `Adapter2`).
    pub default_standard_rate: Option<f64>,
    /// Corresponds to Java: `Project.defaultOvertimeRate` (`@XmlElement(name = "DefaultOvertimeRate")`, `BigDecimal` via `Adapter2`).
    pub default_overtime_rate: Option<f64>,
    /// Corresponds to Java: `Project.durationFormat` (`@XmlElement(name = "DurationFormat")`, `BigInteger`).
    pub duration_format: Option<i64>,
    /// Corresponds to Java: `Project.workFormat` (`@XmlElement(name = "WorkFormat")`, `BigInteger`).
    pub work_format: Option<i64>,
    /// Corresponds to Java: `Project.editableActualCosts` (`@XmlElement(name = "EditableActualCosts", defaultValue = "false")`, `Boolean` via `Adapter1`).
    pub editable_actual_costs: Option<bool>,
    /// Corresponds to Java: `Project.honorConstraints` (`@XmlElement(name = "HonorConstraints", defaultValue = "true")`, `Boolean` via `Adapter1`).
    pub honor_constraints: Option<bool>,
    /// Corresponds to Java: `Project.earnedValueMethod` (`@XmlElement(name = "EarnedValueMethod")`, `BigInteger`).
    pub earned_value_method: Option<i64>,
    /// Corresponds to Java: `Project.insertedProjectsLikeSummary` (`@XmlElement(name = "InsertedProjectsLikeSummary", defaultValue = "true")`, `Boolean` via `Adapter1`).
    pub inserted_projects_like_summary: Option<bool>,
    /// Corresponds to Java: `Project.multipleCriticalPaths` (`@XmlElement(name = "MultipleCriticalPaths", defaultValue = "false")`, `Boolean` via `Adapter1`).
    pub multiple_critical_paths: Option<bool>,
    /// Corresponds to Java: `Project.newTasksEffortDriven` (`@XmlElement(name = "NewTasksEffortDriven", defaultValue = "true")`, `Boolean` via `Adapter1`).
    pub new_tasks_effort_driven: Option<bool>,
    /// Corresponds to Java: `Project.newTasksEstimated` (`@XmlElement(name = "NewTasksEstimated", defaultValue = "true")`, `Boolean` via `Adapter1`).
    pub new_tasks_estimated: Option<bool>,
    /// Corresponds to Java: `Project.splitsInProgressTasks` (`@XmlElement(name = "SplitsInProgressTasks", defaultValue = "true")`, `Boolean` via `Adapter1`).
    pub splits_in_progress_tasks: Option<bool>,
    /// Corresponds to Java: `Project.spreadActualCost` (`@XmlElement(name = "SpreadActualCost", defaultValue = "true")`, `Boolean` via `Adapter1`).
    pub spread_actual_cost: Option<bool>,
    /// Corresponds to Java: `Project.spreadPercentComplete` (`@XmlElement(name = "SpreadPercentComplete", defaultValue = "false")`, `Boolean` via `Adapter1`).
    pub spread_percent_complete: Option<bool>,
    /// Corresponds to Java: `Project.taskUpdatesResource` (`@XmlElement(name = "TaskUpdatesResource")`, `Boolean` via `Adapter1`).
    pub task_updates_resource: Option<bool>,
    /// Corresponds to Java: `Project.fiscalYearStart` (`@XmlElement(name = "FiscalYearStart")`, `Boolean` via `Adapter1`).
    pub fiscal_year_start: Option<bool>,
    /// Corresponds to Java: `Project.weekStartDay` (`@XmlElement(name = "WeekStartDay")`, `BigInteger`).
    pub week_start_day: Option<i64>,
    /// Corresponds to Java: `Project.moveCompletedEndsBack` (`@XmlElement(name = "MoveCompletedEndsBack", defaultValue = "false")`, `Boolean` via `Adapter1`).
    pub move_completed_ends_back: Option<bool>,
    /// Corresponds to Java: `Project.moveRemainingStartsBack` (`@XmlElement(name = "MoveRemainingStartsBack", defaultValue = "false")`, `Boolean` via `Adapter1`).
    pub move_remaining_starts_back: Option<bool>,
    /// Corresponds to Java: `Project.moveRemainingStartsForward` (`@XmlElement(name = "MoveRemainingStartsForward", defaultValue = "false")`, `Boolean` via `Adapter1`).
    pub move_remaining_starts_forward: Option<bool>,
    /// Corresponds to Java: `Project.moveCompletedEndsForward` (`@XmlElement(name = "MoveCompletedEndsForward", defaultValue = "false")`, `Boolean` via `Adapter1`).
    pub move_completed_ends_forward: Option<bool>,
    /// Corresponds to Java: `Project.baselineForEarnedValue` (`@XmlElement(name = "BaselineForEarnedValue")`, `BigInteger`).
    pub baseline_for_earned_value: Option<i64>,
    /// Corresponds to Java: `Project.autoAddNewResourcesAndTasks` (`@XmlElement(name = "AutoAddNewResourcesAndTasks", defaultValue = "true")`, `Boolean` via `Adapter1`).
    pub auto_add_new_resources_and_tasks: Option<bool>,
    /// Corresponds to Java: `Project.statusDate` (`@XmlElement(name = "StatusDate")`, `LocalDateTime` via `Adapter4`).
    pub status_date: Option<NaiveDateTime>,
    /// Corresponds to Java: `Project.currentDate` (`@XmlElement(name = "CurrentDate")`, `LocalDateTime` via `Adapter4`).
    pub current_date: Option<NaiveDateTime>,
    /// Corresponds to Java: `Project.microsoftProjectServerURL` (`@XmlElement(name = "MicrosoftProjectServerURL")`, `Boolean` via `Adapter1`).
    pub microsoft_project_server_url: Option<bool>,
    /// Corresponds to Java: `Project.autolink` (`@XmlElement(name = "Autolink")`, `Boolean` via `Adapter1`).
    pub autolink: Option<bool>,
    /// Corresponds to Java: `Project.newTaskStartDate` (`@XmlElement(name = "NewTaskStartDate")`, `BigInteger`).
    pub new_task_start_date: Option<i64>,
    /// Corresponds to Java: `Project.newTasksAreManual` (`@XmlElement(name = "NewTasksAreManual")`, `Boolean` via `Adapter1`).
    pub new_tasks_are_manual: Option<bool>,
    /// Corresponds to Java: `Project.defaultTaskEVMethod` (`@XmlElement(name = "DefaultTaskEVMethod")`, `BigInteger`).
    pub default_task_ev_method: Option<i64>,
    /// Corresponds to Java: `Project.projectExternallyEdited` (`@XmlElement(name = "ProjectExternallyEdited")`, `Boolean` via `Adapter1`).
    pub project_externally_edited: Option<bool>,
    /// Corresponds to Java: `Project.extendedCreationDate` (`@XmlElement(name = "ExtendedCreationDate")`, `LocalDateTime` via `Adapter4`).
    pub extended_creation_date: Option<NaiveDateTime>,
    /// Corresponds to Java: `Project.actualsInSync` (`@XmlElement(name = "ActualsInSync")`, `Boolean` via `Adapter1`).
    pub actuals_in_sync: Option<bool>,
    /// Corresponds to Java: `Project.removeFileProperties` (`@XmlElement(name = "RemoveFileProperties")`, `Boolean` via `Adapter1`).
    pub remove_file_properties: Option<bool>,
    /// Corresponds to Java: `Project.adminProject` (`@XmlElement(name = "AdminProject")`, `Boolean` via `Adapter1`).
    pub admin_project: Option<bool>,
    /// Corresponds to Java: `Project.baselineCalendar` (`@XmlElement(name = "BaselineCalendar")`, `String`).
    pub baseline_calendar: Option<String>,
    /// Corresponds to Java: `Project.updateManuallyScheduledTasksWhenEditingLinks` (`@XmlElement(name = "UpdateManuallyScheduledTasksWhenEditingLinks")`, `Boolean` via `Adapter1`).
    pub update_manually_scheduled_tasks_when_editing_links: Option<bool>,
    /// Corresponds to Java: `Project.keepTaskOnNearestWorkingTimeWhenMadeAutoScheduled` (`@XmlElement(name = "KeepTaskOnNearestWorkingTimeWhenMadeAutoScheduled")`, `Boolean` via `Adapter1`).
    pub keep_task_on_nearest_working_time_when_made_auto_scheduled: Option<bool>,
    /// Corresponds to Java: `Project.outlineCodes` (`@XmlElement(name = "OutlineCodes")`), flattened from the `Project.OutlineCodes` wrapper to the wrapper's `OutlineCode` child elements.
    pub outline_codes: Vec<OutlineCode>,
    /// Corresponds to Java: `Project.wbsMasks` (`@XmlElement(name = "WBSMasks")`), see the differences table for why this is the one wrapper kept as a struct.
    pub wbs_masks: Option<WBSMasks>,
    /// Corresponds to Java: `Project.extendedAttributes` (`@XmlElement(name = "ExtendedAttributes")`), flattened from the `Project.ExtendedAttributes` wrapper to the wrapper's `ExtendedAttribute` child elements.
    pub extended_attributes: Vec<ExtendedAttribute>,
    /// Corresponds to Java: `Project.calendars` (`@XmlElement(name = "Calendars")`), flattened from the `Project.Calendars` wrapper to the wrapper's `Calendar` child elements.
    pub calendars: Vec<Calendar>,
    /// Corresponds to Java: `Project.tasks` (`@XmlElement(name = "Tasks")`), flattened from the `Project.Tasks` wrapper to the wrapper's `Task` child elements.
    pub tasks: Vec<Task>,
    /// Corresponds to Java: `Project.resources` (`@XmlElement(name = "Resources")`), flattened from the `Project.Resources` wrapper to the wrapper's `Resource` child elements.
    pub resources: Vec<Resource>,
    /// Corresponds to Java: `Project.assignments` (`@XmlElement(name = "Assignments")`), flattened from the `Project.Assignments` wrapper to the wrapper's `Assignment` child elements.
    pub assignments: Vec<Assignment>,
}

impl Project {
    /// Rust-specific: replaces the JAXB unmarshaller, so there is no Java method to correspond to.
    ///
    /// Reads the child elements of `node` (the `Project` root element) by local name, in the same
    /// order as the Java field declarations. The element name is the one given by the Java
    /// `@XmlElement(name = ...)` annotation, which is why e.g. `save_version` reads `SaveVersion`.
    ///
    /// The six element wrapper classes are flattened here: the children of the `Calendars`,
    /// `Tasks`, `Resources`, `Assignments`, `ExtendedAttributes` and `OutlineCodes` elements are
    /// read directly. A missing wrapper element leaves the field as an empty `Vec`, which is what
    /// the Java reader's `!= null` guards produce.
    pub fn from_node(node: Node<'_, '_>) -> Self {
        Self {
            save_version: xml_node::child_text(node, "SaveVersion")
                .and_then(|value| parse_integer(&value)),
            uid: xml_node::child_text(node, "UID").map(|value| value.into_owned()),
            name: xml_node::child_text(node, "Name").map(|value| value.into_owned()),
            guid: xml_node::child_text(node, "GUID")
                .and_then(|value| DatatypeConverter::parse_uuid(&value)),
            title: xml_node::child_text(node, "Title").map(|value| value.into_owned()),
            subject: xml_node::child_text(node, "Subject").map(|value| value.into_owned()),
            category: xml_node::child_text(node, "Category").map(|value| value.into_owned()),
            company: xml_node::child_text(node, "Company").map(|value| value.into_owned()),
            manager: xml_node::child_text(node, "Manager").map(|value| value.into_owned()),
            author: xml_node::child_text(node, "Author").map(|value| value.into_owned()),
            creation_date: xml_node::child_text(node, "CreationDate")
                .and_then(|value| DatatypeConverter::parse_date_time(&value)),
            revision: xml_node::child_text(node, "Revision")
                .and_then(|value| parse_integer(&value)),
            last_saved: xml_node::child_text(node, "LastSaved")
                .and_then(|value| DatatypeConverter::parse_date_time(&value)),
            schedule_from_start: xml_node::child_text(node, "ScheduleFromStart")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            start_date: xml_node::child_text(node, "StartDate")
                .and_then(|value| DatatypeConverter::parse_date_time(&value)),
            finish_date: xml_node::child_text(node, "FinishDate")
                .and_then(|value| DatatypeConverter::parse_date_time(&value)),
            fy_start_date: xml_node::child_text(node, "FYStartDate")
                .and_then(|value| parse_integer(&value)),
            critical_slack_limit: xml_node::child_text(node, "CriticalSlackLimit")
                .and_then(|value| parse_integer(&value)),
            currency_digits: xml_node::child_text(node, "CurrencyDigits")
                .and_then(|value| parse_integer(&value)),
            currency_symbol: xml_node::child_text(node, "CurrencySymbol")
                .map(|value| value.into_owned()),
            currency_code: xml_node::child_text(node, "CurrencyCode")
                .map(|value| value.into_owned()),
            currency_symbol_position: xml_node::child_text(node, "CurrencySymbolPosition")
                .and_then(|value| DatatypeConverter::parse_currency_symbol_position(&value)),
            calendar_uid: xml_node::child_text(node, "CalendarUID")
                .and_then(|value| parse_integer(&value)),
            default_start_time: xml_node::child_text(node, "DefaultStartTime")
                .and_then(|value| DatatypeConverter::parse_time(&value)),
            default_finish_time: xml_node::child_text(node, "DefaultFinishTime")
                .and_then(|value| DatatypeConverter::parse_time(&value)),
            minutes_per_day: xml_node::child_text(node, "MinutesPerDay")
                .and_then(|value| parse_integer(&value)),
            minutes_per_week: xml_node::child_text(node, "MinutesPerWeek")
                .and_then(|value| parse_integer(&value)),
            days_per_month: xml_node::child_text(node, "DaysPerMonth")
                .and_then(|value| parse_integer(&value)),
            default_task_type: xml_node::child_text(node, "DefaultTaskType")
                .and_then(|value| DatatypeConverter::parse_task_type(&value)),
            default_fixed_cost_accrual: xml_node::child_text(node, "DefaultFixedCostAccrual")
                .and_then(|value| DatatypeConverter::parse_accrue_type(&value)),
            default_standard_rate: xml_node::child_text(node, "DefaultStandardRate")
                .and_then(|value| parse_decimal(&value)),
            default_overtime_rate: xml_node::child_text(node, "DefaultOvertimeRate")
                .and_then(|value| parse_decimal(&value)),
            duration_format: xml_node::child_text(node, "DurationFormat")
                .and_then(|value| parse_integer(&value)),
            work_format: xml_node::child_text(node, "WorkFormat")
                .and_then(|value| parse_integer(&value)),
            editable_actual_costs: xml_node::child_text(node, "EditableActualCosts")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            honor_constraints: xml_node::child_text(node, "HonorConstraints")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            earned_value_method: xml_node::child_text(node, "EarnedValueMethod")
                .and_then(|value| parse_integer(&value)),
            inserted_projects_like_summary: xml_node::child_text(
                node,
                "InsertedProjectsLikeSummary",
            )
            .map(|value| DatatypeConverter::parse_boolean(&value)),
            multiple_critical_paths: xml_node::child_text(node, "MultipleCriticalPaths")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            new_tasks_effort_driven: xml_node::child_text(node, "NewTasksEffortDriven")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            new_tasks_estimated: xml_node::child_text(node, "NewTasksEstimated")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            splits_in_progress_tasks: xml_node::child_text(node, "SplitsInProgressTasks")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            spread_actual_cost: xml_node::child_text(node, "SpreadActualCost")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            spread_percent_complete: xml_node::child_text(node, "SpreadPercentComplete")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            task_updates_resource: xml_node::child_text(node, "TaskUpdatesResource")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            fiscal_year_start: xml_node::child_text(node, "FiscalYearStart")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            week_start_day: xml_node::child_text(node, "WeekStartDay")
                .and_then(|value| parse_integer(&value)),
            move_completed_ends_back: xml_node::child_text(node, "MoveCompletedEndsBack")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            move_remaining_starts_back: xml_node::child_text(node, "MoveRemainingStartsBack")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            move_remaining_starts_forward: xml_node::child_text(node, "MoveRemainingStartsForward")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            move_completed_ends_forward: xml_node::child_text(node, "MoveCompletedEndsForward")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            baseline_for_earned_value: xml_node::child_text(node, "BaselineForEarnedValue")
                .and_then(|value| parse_integer(&value)),
            auto_add_new_resources_and_tasks: xml_node::child_text(
                node,
                "AutoAddNewResourcesAndTasks",
            )
            .map(|value| DatatypeConverter::parse_boolean(&value)),
            status_date: xml_node::child_text(node, "StatusDate")
                .and_then(|value| DatatypeConverter::parse_date_time(&value)),
            current_date: xml_node::child_text(node, "CurrentDate")
                .and_then(|value| DatatypeConverter::parse_date_time(&value)),
            microsoft_project_server_url: xml_node::child_text(node, "MicrosoftProjectServerURL")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            autolink: xml_node::child_text(node, "Autolink")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            new_task_start_date: xml_node::child_text(node, "NewTaskStartDate")
                .and_then(|value| parse_integer(&value)),
            new_tasks_are_manual: xml_node::child_text(node, "NewTasksAreManual")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            default_task_ev_method: xml_node::child_text(node, "DefaultTaskEVMethod")
                .and_then(|value| parse_integer(&value)),
            project_externally_edited: xml_node::child_text(node, "ProjectExternallyEdited")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            extended_creation_date: xml_node::child_text(node, "ExtendedCreationDate")
                .and_then(|value| DatatypeConverter::parse_date_time(&value)),
            actuals_in_sync: xml_node::child_text(node, "ActualsInSync")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            remove_file_properties: xml_node::child_text(node, "RemoveFileProperties")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            admin_project: xml_node::child_text(node, "AdminProject")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            baseline_calendar: xml_node::child_text(node, "BaselineCalendar")
                .map(|value| value.into_owned()),
            update_manually_scheduled_tasks_when_editing_links: xml_node::child_text(
                node,
                "UpdateManuallyScheduledTasksWhenEditingLinks",
            )
            .map(|value| DatatypeConverter::parse_boolean(&value)),
            keep_task_on_nearest_working_time_when_made_auto_scheduled: xml_node::child_text(
                node,
                "KeepTaskOnNearestWorkingTimeWhenMadeAutoScheduled",
            )
            .map(|value| DatatypeConverter::parse_boolean(&value)),
            outline_codes: xml_node::child(node, "OutlineCodes")
                .map(|wrapper| {
                    xml_node::children(wrapper, "OutlineCode")
                        .map(OutlineCode::from_node)
                        .collect()
                })
                .unwrap_or_default(),
            wbs_masks: xml_node::child(node, "WBSMasks").map(WBSMasks::from_node),
            extended_attributes: xml_node::child(node, "ExtendedAttributes")
                .map(|wrapper| {
                    xml_node::children(wrapper, "ExtendedAttribute")
                        .map(ExtendedAttribute::from_node)
                        .collect()
                })
                .unwrap_or_default(),
            calendars: xml_node::child(node, "Calendars")
                .map(|wrapper| {
                    xml_node::children(wrapper, "Calendar")
                        .map(Calendar::from_node)
                        .collect()
                })
                .unwrap_or_default(),
            tasks: xml_node::child(node, "Tasks")
                .map(|wrapper| {
                    xml_node::children(wrapper, "Task")
                        .map(Task::from_node)
                        .collect()
                })
                .unwrap_or_default(),
            resources: xml_node::child(node, "Resources")
                .map(|wrapper| {
                    xml_node::children(wrapper, "Resource")
                        .map(Resource::from_node)
                        .collect()
                })
                .unwrap_or_default(),
            assignments: xml_node::child(node, "Assignments")
                .map(|wrapper| {
                    xml_node::children(wrapper, "Assignment")
                        .map(Assignment::from_node)
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AccrueType, CurrencySymbolPosition, NaiveTime, Project, TaskType, Uuid};
    use chrono::NaiveDate;
    use roxmltree::Document;

    /// Parses a document and returns the root element's `Project`.
    fn project(xml: &str) -> Project {
        let document = Document::parse(xml).unwrap();
        Project::from_node(document.root_element())
    }

    /// The scalar fields are read from the elements named by the Java `@XmlElement` annotations,
    /// and a field whose element is absent stays `None`.
    #[test]
    fn reads_scalar_fields() {
        let project = project(
            r#"<Project>
                 <SaveVersion>14</SaveVersion>
                 <UID>1</UID>
                 <Name>Sample</Name>
                 <GUID>0e4f1a7b-8c9d-4e5f-a6b7-8c9d0e1f2a3b</GUID>
                 <CreationDate>2024-01-02T03:04:05</CreationDate>
                 <LastSaved>2024-01-03T04:05:06</LastSaved>
                 <ScheduleFromStart>0</ScheduleFromStart>
                 <DefaultStartTime>08:00:00</DefaultStartTime>
                 <DefaultTaskType>1</DefaultTaskType>
                 <DefaultFixedCostAccrual>3</DefaultFixedCostAccrual>
                 <CurrencySymbolPosition>1</CurrencySymbolPosition>
                 <DefaultStandardRate>12.5</DefaultStandardRate>
                 <MicrosoftProjectServerURL>1</MicrosoftProjectServerURL>
                 <BaselineCalendar>Standard</BaselineCalendar>
                 <MinutesPerDay>480</MinutesPerDay>
               </Project>"#,
        );

        assert_eq!(project.save_version, Some(14));
        assert_eq!(project.uid.as_deref(), Some("1"));
        assert_eq!(project.name.as_deref(), Some("Sample"));
        assert_eq!(
            project.guid,
            Some(Uuid::parse_str("0e4f1a7b-8c9d-4e5f-a6b7-8c9d0e1f2a3b").unwrap())
        );
        assert_eq!(
            project.creation_date,
            NaiveDate::from_ymd_opt(2024, 1, 2)
                .unwrap()
                .and_hms_opt(3, 4, 5)
        );
        assert_eq!(
            project.last_saved,
            NaiveDate::from_ymd_opt(2024, 1, 3)
                .unwrap()
                .and_hms_opt(4, 5, 6)
        );
        assert_eq!(project.schedule_from_start, Some(false));
        assert_eq!(project.default_start_time, NaiveTime::from_hms_opt(8, 0, 0));
        assert_eq!(project.default_task_type, Some(TaskType::FixedDuration));
        assert_eq!(
            project.default_fixed_cost_accrual,
            Some(AccrueType::Prorated)
        );
        // Java's `DatatypeConverter.parseCurrencySymbolPosition` is a hand-written switch whose
        // values differ from the enum's own int values: 1 maps to AFTER, not to BEFORE.
        assert_eq!(
            project.currency_symbol_position,
            Some(CurrencySymbolPosition::After)
        );
        assert_eq!(project.default_standard_rate, Some(12.5));
        assert_eq!(project.microsoft_project_server_url, Some(true));
        assert_eq!(project.baseline_calendar.as_deref(), Some("Standard"));
        assert_eq!(project.minutes_per_day, Some(480));

        // Absent optional elements leave the field as `None`.
        assert_eq!(project.title, None);
        assert_eq!(project.default_overtime_rate, None);
        // The flattened wrapper fields are empty when the wrapper element is missing.
        assert!(project.outline_codes.is_empty());
        // No `WBSMasks` element, so the optional wrapper stays `None`.
        assert!(project.wbs_masks.is_none());
    }

    /// The element wrappers are flattened: the children of the wrapper elements become the
    /// `Project` fields, in document order.
    #[test]
    fn flattens_element_wrappers() {
        let project = project(
            r#"<Project>
                 <OutlineCodes>
                   <OutlineCode><FieldName>First</FieldName></OutlineCode>
                   <OutlineCode><FieldName>Second</FieldName></OutlineCode>
                 </OutlineCodes>
                 <WBSMasks>
                   <VerifyUniqueCodes>1</VerifyUniqueCodes>
                   <Prefix>WBS</Prefix>
                 </WBSMasks>
                 <Tasks><Task/><Task/></Tasks>
               </Project>"#,
        );

        assert_eq!(project.outline_codes.len(), 2);
        assert_eq!(
            project.outline_codes[0].field_name.as_deref(),
            Some("First")
        );
        assert_eq!(
            project.outline_codes[1].field_name.as_deref(),
            Some("Second")
        );

        let wbs_masks = project.wbs_masks.expect("WBSMasks element is present");
        assert_eq!(wbs_masks.verify_unique_codes, Some(true));
        assert_eq!(wbs_masks.prefix.as_deref(), Some("WBS"));

        assert_eq!(project.tasks.len(), 2);
    }
}
