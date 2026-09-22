//! Corresponds to Java: `org/mpxj/ResourceAssignment.java`
//!
//! Resource assignment (task × resource) entity. On the Java side it extends `AbstractFieldContainer` and implements
//! `ProjectEntityWithMutableUniqueID` / `TimePeriodEntity`.
//!
//! # Differences from Java (trade-offs at a glance)
//!
//! ## 1) Inheritance replaced by composition
//!
//! | Java | Rust |
//! |---|---|
//! | `extends AbstractFieldContainer<ResourceAssignment>` | Field storage becomes composition of an [`AbstractFieldContainer`], to which `set` / `get` / `get_cached_value` forward |
//! | `implements ProjectEntityWithMutableUniqueID` | [`ProjectEntityWithMutableUniqueID`] (includes [`ProjectEntityWithUniqueID`]) |
//! | `implements TimePeriodEntity` | No corresponding trait (`time_period_entity` is not declared in this repository); the two timephased getters are provided as ordinary methods |
//! | `implements FieldContainer` | [`FieldContainer`] (`set` / `get` / `get_cached_value` / listeners) |
//!
//! Java's three abstract methods land in this class per the conventions of [`crate::abstract_field_container`]:
//! `getAlwaysCalculatedField` → [`ResourceAssignment::is_always_calculated`];
//! `getCalculationMethod` → [`ResourceAssignment::calculate`] (dispatched with `match`);
//! `handleFieldChange` → [`ResourceAssignment::handle_field_change`].
//!
//! ## 2) All object references use `Weak` to break cycles (important)
//!
//! On the Java side `m_parentFile`, `getTask()` / `getResource()` yield **strong references**. If the Rust side copied them as-is
//! `Rc`, it would form an `ResourceAssignment → Task/Resource/ProjectFile → container → ResourceAssignment`
//! `Rc` cycle, causing the entity to never be released. This class therefore always holds `Weak`:
//!
//! | Java | Rust field |
//! |---|---|
//! | `private final ProjectFile m_parentFile` | `parent_file: Weak<RefCell<ProjectFile>>` |
//! | `ProjectContext` (Java accesses it indirectly via `ProjectFile`) | `context: Weak<RefCell<ProjectContext>>` (Rust-specific, used to resolve custom fields by alias) |
//! | `getTask()` (looks up the parent file by unique ID) | `task: Weak<RefCell<Task>>`, falling back to the parent file lookup by unique ID when not found |
//! | `getResource()` (looks up the parent file by unique ID) | `resource: Weak<RefCell<Resource>>`, same fallback as above |
//!
//! Java wires things up in the constructor using `this` / `file`; Rust cannot obtain its own `Rc` inside `new()`,
//! so it is first constructed with empty `Weak`s, then wired by [`ResourceAssignment::set_parent_file`] /
//! [`ResourceAssignment::set_context`] / [`ResourceAssignment::set_task`] /
//! [`ResourceAssignment::set_resource`] (Rust-specific methods).
//!
//! ## 3) Field values are always stored in the field table
//!
//! Java's `get` / `set` store all field values (including `WorkContour` / `RateSource` /
//! `ResourceRequestType` / `TimeUnit`(variable rate) / `UUID`(GUID) / `Notes` objects /
//! `List<TimephasedWork>` / `List<TimephasedCost>` / `List<LocalDateTimeRange>`)
//! are all placed in the `AbstractFieldContainer` field table; the Rust side does the same, so
//! [`ResourceAssignment::get_field`] / [`ResourceAssignment::peek_cached_value`]
//! these values can be retrieved consistently with Java's `get` / `getCachedValue`
//! (the evaluation entry point is [`ResourceAssignment::calculate`], corresponding to Java's `CALCULATED_FIELD_MAP`).
//!
//! The only exception is `AssignmentField::RESOURCE_ASSIGNMENT_CODE_VALUES`: it depends on the unported
//! `ResourceAssignmentCode` / `ResourceAssignmentCodeValue`, so it is skipped entirely.
//! Also, Java's `m_workgroup` (the workgroup field of MPX record 76) is **not** in the field table; the Rust side
//! likewise keeps it as a standalone field of this class (see [`ResourceAssignment::add_workgroup_assignment`]).
//!
//! ## 4) Skipped Java members and reasons
//!
//! | Java | Reason |
//! |---|---|
//! | `getCostAccount()` / `setCostAccount(CostAccount)` | `CostAccount` / `CostAccountContainer` are not declared in this repository (`getCostAccountUniqueID` / `setCostAccountUniqueID` are ported) |
//! | `getResourceAssignmentCodeValues()` / `addResourceAssignmentCodeValue(...)` | `ResourceAssignmentCode` / `ResourceAssignmentCodeValue` are not declared in this repository |
//! | `public CostRateTable getCostRateTable()` | [`ResourceAssignment::with_cost_rate_table`] (Rust-specific callback form: a resource holds a `CostRateTable` by value and that type does not implement `Clone`, so ownership cannot be returned; the call-site semantics of `getEffectiveRate` / timephased cost are unchanged) |
//! | The `CombinedCalendar` wrapping in `getEffectiveCalendar()` | That combined calendar is not ported; the `ManuallyScheduledTaskCalendar` for manual scheduling is wired up (see the method docs) |
//! | The `ProjectProperties` used by `calculatePercentWorkComplete` / `calculateStartVariance` / `calculateFinishVariance` | Obtained via `ProjectFile.get_project_properties()` (consistent with the assumptions of `cpm::microsoft_slack_calculator`); falls back to the effective calendar / `TimeUnit::Days` when the parent file is not wired up |
//! | The `RateHelper.convertToHours` used by `getRatePerHour` | Calls [`crate::rate_helper::RateHelper::convert_to_hours`] directly (`getEffectiveCalendar()` on the Rust side is `Option<&Rc<RefCell<ProjectCalendar>>>`, returning the original value when not wired up or on borrow conflict) |
//! | `TimephasedUtility` (`org.mpxj.TimephasedUtility`, not declared in this repository) | Ported equivalently as private functions at the end of the file: `segment_work` / `segment_cost` / `segment_material` / `add_timephased_durations` / `add_timephased_numbers` |
//! | `TimephasedUtility`'s `validateRanges` / `validateTimephasedWork` / `validateTimephasedCost` | Java throws `IllegalArgumentException`; Rust library code must not panic, so no validation is performed |
//! | `getFieldByAlias` / `setFieldByAlias` | Resolved by alias via the context's custom field container (Java uses `ProjectFile.getResourceAssignments().getFieldTypeByAlias()`; the two are equivalent) |
//! | `toString()` | [`Display`] (Java renders `null` as `"null"`, consistent with Rust) |
//! | Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable` | Not used by this class |
//!
//! ## 5) Other differences
//!
//! * **Sync the container when the unique ID changes**: Java's `handleFieldChange` uses `this` to call
//! `ResourceAssignmentContainer.updateUniqueID(this, old, new)`; Rust cannot obtain one from `&mut self`
//! so it instead syncs the sequence generator by unique ID (see
//!   [`crate::resource_assignment_container::ResourceAssignmentContainer::update_unique_id`]).
//! * **The write-back side effect of `get`**: Java's `get` writes the computed field value back to the field table (and triggers dependent cleanup);
//! Rust's `get_field(&self)` cannot write back, so it only returns the value without caching (the returned value semantics are the same).
//! * **The `set(WORK_VARIANCE, ...)` in `calculateWorkVariance`** likewise no longer writes back during calculation.
//! * **Method overloading**: Java overloads are distinguished in Rust by suffixes: `getBaselineWork(int)` →
//!   [`ResourceAssignment::get_baseline_work_by_index`],`setStart(int, ...)` / `getStart(int)`
//!   → `*_by_index`,`getCost(int)` → [`ResourceAssignment::get_cost_by_index`],
//!   `getTimephasedCost(List)` → [`ResourceAssignment::get_timephased_cost_total`]
//! (the private overload `getTimephasedCost(ranges, rateIndex, workSupplier)` keeps its original name).
//! * **Exceptions**: Java throws `IllegalArgumentException` (`selectField` out of bounds), `IndexOutOfBoundsException`
//! (a timephased accrual method cannot find an intersecting range), `NullPointerException` (missing calendar/date) branches;
//! the Rust side uniformly degrades to `None` / an empty list / a no-op (without panicking).
//! * `PartialEq` / `Eq` / `Hash` are not derived: Java does not override `equals` / `hashCode` (preserving reference-equality semantics).
//!
//! ## 6) Interface assumptions this file makes about related modules (verified consistent)
//!
//! ```text
//! // ProjectFile
//! ProjectFile::get_project_config(&self) -> &ProjectConfig
//! ProjectFile::get_unique_id_object_sequence(&mut self, TypeId) -> Rc<ObjectSequence>  // UniqueIdObjectSequenceProvider
//! ProjectFile::get_resource_assignments(&self) -> &ResourceAssignmentContainer
//! ProjectFile::get_resource_assignments_mut(&mut self) -> &mut ResourceAssignmentContainer
//! ProjectFile::get_project_properties(&self) -> &ProjectProperties
//! ProjectFile::get_baseline_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>>
//! ProjectFile::get_resource_by_unique_id(&self, Option<i32>) -> Option<Rc<RefCell<Resource>>>
//! ProjectFile::get_task_by_unique_id(&self, Option<i32>) -> Option<Rc<RefCell<Task>>>
//! // Task (consistent with the landed task.rs)
//! Task::get_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>>
//! Task::get_effective_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>>
//! Task::get_ignore_resource_calendar(&self) -> bool
//! Task::get_start(&self) -> Option<NaiveDateTime> / Task::get_finish(&self) -> Option<NaiveDateTime>
//! Task::get_name(&self) -> Option<String>
//! // Resource (consistent with the landed resource.rs)
//! Resource::get_type(&self) -> ResourceType
//! Resource::get_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>>
//! Resource::get_cost_rate_table(&self, i32) -> Option<&CostRateTable>
//! Resource::get_accrue_at(&self) -> Option<AccrueType>
//! Resource::get_cost_per_use(&self) -> Option<f64>
//! Resource::get_unique_id(&self) -> Option<i32> / Resource::get_name(&self) -> Option<String>
//! // TimephasedWork / TimephasedCost (both Clone, fields may be empty)
//! TimephasedWork::new() / TimephasedWork::from_work(&TimephasedWork, f64)   // Java: new TimephasedWork(src, factor)
//! TimephasedWork::get_start(&self) -> Option<NaiveDateTime> / set_start(Option<NaiveDateTime>)
//! TimephasedWork::get_finish(&self) -> Option<NaiveDateTime> / set_finish(Option<NaiveDateTime>)
//! TimephasedWork::get_total_amount(&self) -> Option<Duration> / set_total_amount(Option<Duration>)
//! TimephasedWork::get_amount_per_hour(&self) -> Option<Duration> / set_amount_per_hour(Option<Duration>)
//! TimephasedCost is the same as above, with the amount as Option<f64>
//! ```

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::rc::{Rc, Weak};
use std::sync::OnceLock;

use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::abstract_field_container::AbstractFieldContainer;
use crate::accrue_type::AccrueType;
use crate::assignment_field::AssignmentField;
use crate::common::boolean_helper::BooleanHelper;
use crate::common::local_date_time_helper::LocalDateTimeHelper;
use crate::common::number_helper::NumberHelper;
use crate::cost_rate_table::CostRateTable;
use crate::cost_rate_table_entry::CostRateTableEntry;
use crate::duration::Duration;
use crate::field_container::FieldContainer;
use crate::field_key::FieldKey;
use crate::field_type_class::FieldTypeClass;
use crate::field_value::FieldValue;
use crate::listener::field_listener::FieldListener;
use crate::local_date_time_range::LocalDateTimeRange;
use crate::notes::{Notes, PlainNotes};
use crate::project_calendar::ProjectCalendar;
use crate::project_context::ProjectContext;
use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
use crate::project_file::ProjectFile;
use crate::rate::Rate;
use crate::rate_helper::RateHelper;
use crate::rate_source::RateSource;
use crate::resource::Resource;
use crate::resource_assignment_workgroup_fields::ResourceAssignmentWorkgroupFields;
use crate::resource_request_type::ResourceRequestType;
use crate::resource_type::ResourceType;
use crate::task::Task;
use crate::time_unit::TimeUnit;
use crate::timephased_cost::TimephasedCost;
use crate::timephased_work::TimephasedWork;
use crate::work_contour::WorkContour;

/// Resource assignment (the association record between a task and a resource).
///
/// Corresponds to Java: `public class ResourceAssignment extends AbstractFieldContainer<ResourceAssignment>
/// implements ProjectEntityWithMutableUniqueID, TimePeriodEntity`
pub struct ResourceAssignment {
/// Parent project (weak reference, breaking the `Rc` cycle of `ProjectFile ↔ ResourceAssignment`).
    ///
    /// Corresponds to Java field `private final ProjectFile m_parentFile;`
    parent_file: Weak<RefCell<ProjectFile>>,

/// Owning context (weak reference, Rust-specific: used to resolve custom fields by alias).
    context: Weak<RefCell<ProjectContext>>,

/// Associated task (weak reference, corresponding to the strong reference of Java `getTask()`).
    task: Weak<RefCell<Task>>,

/// Associated resource (weak reference, corresponding to the strong reference of Java `getResource()`).
    resource: Weak<RefCell<Resource>>,

/// Field storage (composition replaces Java's inheritance).
    ///
/// The notes object (`AssignmentField.NOTES`), work contour (`WORK_CONTOUR`), rate source
/// (`RATE_SOURCE`), resource request type (`RESOURCE_REQUEST_TYPE`), variable rate units
/// (`VARIABLE_RATE_UNITS`), GUID, and the raw timephased lists (`RAW_TIMEPHASED_*`)
/// are all stored in that field table consistently with Java (see [`crate::field_value::FieldValue`]).
    fields: AbstractFieldContainer,

/// Workgroup field (MPX record 76).
    ///
    /// Corresponds to Java: `private ResourceAssignmentWorkgroupFields m_workgroup;`
    ///
/// On the Java side this is not a field-table member but a standalone field of this class (lazily created by `addWorkgroupAssignment()`),
/// so the Rust side likewise keeps it as a standalone field.
    workgroup_assignment: Option<ResourceAssignmentWorkgroupFields>,
}

impl Default for ResourceAssignment {
/// Corresponds to Java: the "no parent file" form of the package-level constructor `ResourceAssignment(ProjectFile file)`
/// (see [`ResourceAssignment::new`]).
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceAssignment {
/// Constructor (Rust-specific form: does not automatically assign a unique ID).
    ///
/// Corresponds to Java: the package-level constructor `ResourceAssignment(ProjectFile file)` (which assigns
/// a unique ID according to `file.getProjectConfig().getAutoAssignmentUniqueID()`).
    ///
/// Rust cannot obtain its own `Rc` inside `new()` nor complete the wiring, so it does not automatically assign a unique ID;
/// use [`ResourceAssignment::with_parent_file`] when Java-equivalent construction behavior is needed.
    pub fn new() -> Self {
        Self {
            parent_file: Weak::new(),
            context: Weak::new(),
            task: Weak::new(),
            resource: Weak::new(),
            fields: AbstractFieldContainer::new(),
            workgroup_assignment: None,
        }
    }

/// The form equivalent to the Java constructor: wires up the parent file and automatically assigns a unique ID when needed.
    ///
    /// Corresponds to Java: `ResourceAssignment(ProjectFile file)`
    pub fn with_parent_file(parent_file: &Rc<RefCell<ProjectFile>>) -> Self {
        let mut assignment = Self::new();
        assignment.set_parent_file(parent_file);

        let next_unique_id = {
            let mut parent_file_ref = parent_file.borrow_mut();
            let auto = parent_file_ref
                .get_project_config()
                .get_auto_assignment_unique_id();

            if auto {
                Some(
                    parent_file_ref
                        .get_unique_id_object_sequence(
                            std::any::TypeId::of::<ResourceAssignment>(),
                        )
                        .get_next(),
                )
            } else {
                None
            }
        };

        if next_unique_id.is_some() {
            assignment.set_unique_id(next_unique_id);
        }

        assignment
    }

/// Wire up the parent file (Rust-specific: Java receives `file` directly in the constructor).
    ///
/// Corresponds to Java: `m_parentFile = file;` in the constructor
    pub fn set_parent_file(&mut self, parent_file: &Rc<RefCell<ProjectFile>>) {
        self.parent_file = Rc::downgrade(parent_file);
    }

/// Wire up the owning context (Rust-specific: Java accesses the context indirectly via `ProjectFile`).
    pub fn set_context(&mut self, context: &Rc<RefCell<ProjectContext>>) {
        self.context = Rc::downgrade(context);
    }

/// Wire up the associated task (Rust-specific: Java looks it up dynamically by unique ID in `getTask()`).
    pub fn set_task(&mut self, task: &Rc<RefCell<Task>>) {
        self.task = Rc::downgrade(task);
    }

/// Wire up the associated resource (Rust-specific: Java looks it up dynamically by unique ID in `getResource()`).
    pub fn set_resource(&mut self, resource: &Rc<RefCell<Resource>>) {
        self.resource = Rc::downgrade(resource);
    }

/// Get the parent project (upgrades the weak reference; returns `None` when the parent file has been released).
    ///
    /// Corresponds to Java: `public ProjectFile getParentFile()`
    pub fn get_parent_file(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.parent_file.upgrade()
    }

/// Get the owning context (Rust-specific accessor).
    ///
/// Corresponds to Java: none (Java accesses the context via `ProjectFile`).
    pub fn get_context(&self) -> Option<Rc<RefCell<ProjectContext>>> {
        self.context.upgrade()
    }

/// Create (or return, if already present) the workgroup field record.
    ///
    /// Corresponds to Java: `public ResourceAssignmentWorkgroupFields addWorkgroupAssignment()`
    ///
/// Java throws `MPXJException(MAXIMUM_RECORDS)` when a record already exists; Rust library code must not panic,
/// so it instead returns the existing record (see "Skipped Java members" in the file header).
    pub fn add_workgroup_assignment(&mut self) -> &mut ResourceAssignmentWorkgroupFields {
        self.workgroup_assignment
            .get_or_insert_with(ResourceAssignmentWorkgroupFields::new)
    }

/// Get the workgroup field record; returns `None` when it has not been created.
    ///
    /// Corresponds to Java: `public ResourceAssignmentWorkgroupFields getWorkgroupAssignment()`
    pub fn get_workgroup_assignment(&self) -> Option<&ResourceAssignmentWorkgroupFields> {
        self.workgroup_assignment.as_ref()
    }

/// Write the value into the field table and, when the value actually changes, perform Java's `handleFieldChange`.
    ///
    /// Corresponds to Java: `AbstractFieldContainer.set(FieldType field, Object value)`
/// (the internal order is: update the field table → `handleFieldChange` → `fireFieldChangeEvent`).
    fn set_key(&mut self, field: FieldKey, value: Option<FieldValue>) {
        let old_value = self.fields.get_cached_value(field).cloned();
        if old_value == value {
            return;
        }

        self.fields.set(field, value.clone());
        self.handle_field_change(field, old_value.as_ref(), value.as_ref());
    }

/// Set a field value (the public entry point of [`FieldContainer`]).
    ///
    /// Corresponds to Java: `public void set(FieldType field, Object value)`
    pub fn set_field(&mut self, field: FieldKey, value: Option<FieldValue>) {
        self.set_key(field, value);
    }

/// Clear the cached calculated values affected by this field change.
    ///
    /// Corresponds to Java: `@Override protected void handleFieldChange(FieldType field, Object oldValue, Object newValue)`
    fn handle_field_change(
        &mut self,
        field: FieldKey,
        old_value: Option<&FieldValue>,
        new_value: Option<&FieldValue>,
    ) {
        if field == FieldKey::Assignment(AssignmentField::UniqueId) {
            // Java: m_parentFile.getResourceAssignments().updateUniqueID(this, oldValue, newValue);
// The Rust side does not pass the element (it cannot obtain an `Rc` from `&mut self`) and instead syncs the sequence generator by unique ID.
            let old_unique_id = old_value.and_then(FieldValue::as_integer);
            let new_unique_id = new_value.and_then(FieldValue::as_integer);

            if let Some(parent_file) = self.parent_file.upgrade() {
                if let Ok(mut parent_file) = parent_file.try_borrow_mut() {
                    parent_file
                        .get_resource_assignments_mut()
                        .update_unique_id(old_unique_id, new_unique_id);
                }
            }
            return;
        }

        // Java: clearDependentFields(DEPENDENCY_MAP, field);
        self.fields.clear_dependent_fields(dependency_map(), field);
    }

/// Whether the field is "always recalculated" (does not read the cache).
    ///
    /// Corresponds to Java: `@Override boolean getAlwaysCalculatedField(FieldType field)`
    fn is_always_calculated(field: FieldKey) -> bool {
        matches!(
            field,
            FieldKey::Assignment(AssignmentField::Start)
                | FieldKey::Assignment(AssignmentField::Finish)
        )
    }

/// Read a field value from the field table (including calculated fields).
    ///
    /// Corresponds to Java: `public Object get(FieldType field)`
    ///
/// Note: the Java side writes the computed result back to the field table; Rust's `&self` cannot write back, so it computes without caching.
    pub fn get_field(&self, field: FieldKey) -> Option<FieldValue> {
        if !Self::is_always_calculated(field) {
            if let Some(value) = self.fields.get_cached_value(field) {
                return Some(value.clone());
            }
        }

        self.calculate(field)
    }

/// Disable dependent-field cleanup and event firing during field updates.
    ///
    /// Corresponds to Java: `public void disableEvents()`
    pub fn disable_events(&mut self) {
        self.fields.disable_events();
    }

/// Enable dependent-field cleanup and event firing during field updates (the default state).
    ///
    /// Corresponds to Java: `public void enableEvents()`
    pub fn enable_events(&mut self) {
        self.fields.enable_events();
    }

/// Read only the stored field value without triggering calculation.
    ///
    /// Corresponds to Java: `public Object getCachedValue(FieldType field)`
    pub fn peek_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
        self.fields.get_cached_value(field)
    }

/// Evaluate a calculated field; returns `None` for non-calculated fields.
    ///
    /// Corresponds to Java: `@Override Function<ResourceAssignment, Object> getCalculationMethod(FieldType field)`
/// Corresponds to `CALCULATED_FIELD_MAP`.
    fn calculate(&self, field: FieldKey) -> Option<FieldValue> {
        let field = match field {
            FieldKey::Assignment(field) => field,
            _ => return None,
        };

        use AssignmentField as F;
        match field {
            F::OvertimeCost => self.calculate_overtime_cost().map(FieldValue::Double),
            F::CostVariance => self.calculate_cost_variance().map(FieldValue::Double),
            F::Cv => self.calculate_cv().map(FieldValue::Double),
            F::Sv => self.calculate_sv().map(FieldValue::Double),
            F::StartVariance => self.calculate_start_variance().map(FieldValue::Duration),
            F::FinishVariance => self.calculate_finish_variance().map(FieldValue::Duration),
            F::PercentWorkComplete => self.calculate_percent_work_complete().map(FieldValue::Double),
            F::WorkVariance => self.calculate_work_variance().map(FieldValue::Duration),
            F::Start => self.calculate_start().map(FieldValue::DateTime),
            F::Finish => self.calculate_finish().map(FieldValue::DateTime),
            F::RemainingAssignmentUnits => self
                .calculate_remaining_assignment_units()
                .map(FieldValue::Double),
            F::ActualRegularCost => self.calculate_actual_regular_cost().map(FieldValue::Double),
            F::RemainingRegularCost => self
                .calculate_remaining_regular_cost()
                .map(FieldValue::Double),
            F::ActualRegularWork => self.calculate_actual_regular_work().map(FieldValue::Duration),
            F::RemainingRegularWork => self
                .calculate_remaining_regular_work()
                .map(FieldValue::Duration),
            F::PlannedMaterial => self
                .calculate_material(self.get_planned_work())
                .map(FieldValue::Double),
            F::ActualMaterial => self
                .calculate_material(self.get_actual_work())
                .map(FieldValue::Double),
            F::RemainingMaterial => self
                .calculate_material(self.get_remaining_work())
                .map(FieldValue::Double),
            F::Material => self.calculate_material(self.get_work()).map(FieldValue::Double),
            F::BaselineMaterial => self
                .calculate_material(self.get_baseline_work())
                .map(FieldValue::Double),
            F::Baseline1Material => self
                .calculate_material(self.get_baseline_work_by_index(1))
                .map(FieldValue::Double),
            F::Baseline2Material => self
                .calculate_material(self.get_baseline_work_by_index(2))
                .map(FieldValue::Double),
            F::Baseline3Material => self
                .calculate_material(self.get_baseline_work_by_index(3))
                .map(FieldValue::Double),
            F::Baseline4Material => self
                .calculate_material(self.get_baseline_work_by_index(4))
                .map(FieldValue::Double),
            F::Baseline5Material => self
                .calculate_material(self.get_baseline_work_by_index(5))
                .map(FieldValue::Double),
            F::Baseline6Material => self
                .calculate_material(self.get_baseline_work_by_index(6))
                .map(FieldValue::Double),
            F::Baseline7Material => self
                .calculate_material(self.get_baseline_work_by_index(7))
                .map(FieldValue::Double),
            F::Baseline8Material => self
                .calculate_material(self.get_baseline_work_by_index(8))
                .map(FieldValue::Double),
            F::Baseline9Material => self
                .calculate_material(self.get_baseline_work_by_index(9))
                .map(FieldValue::Double),
            F::Baseline10Material => self
                .calculate_material(self.get_baseline_work_by_index(10))
                .map(FieldValue::Double),
            F::RateIndex => self.default_rate_index().map(FieldValue::Integer),
            F::RateSource => Some(FieldValue::RateSource(Self::default_rate_source())),
            F::CalculateCostsFromUnits => self
                .default_calculate_costs_from_units()
                .map(FieldValue::Boolean),
            F::ResourceAssignmentCodeValues => None,
// Java: the `calculateTimephasedOvertimeWork` of `CALCULATED_FIELD_MAP`
            F::RawTimephasedRemainingOvertimeWork => {
                Some(FieldValue::TimephasedWork(self.calculate_timephased_overtime_work()))
            }
// Java: the default values of the remaining `RAW_TIMEPHASED_*` are respectively
// `defaultTimephasedWork()` / `defaultTimephasedCost()` (empty lists).
            F::RawTimephasedPlannedWork
            | F::RawTimephasedActualRegularWork
            | F::RawTimephasedRemainingRegularWork
            | F::RawTimephasedActualOvertimeWork
            | F::RawTimephasedBaselineWork
            | F::RawTimephasedBaseline1Work
            | F::RawTimephasedBaseline2Work
            | F::RawTimephasedBaseline3Work
            | F::RawTimephasedBaseline4Work
            | F::RawTimephasedBaseline5Work
            | F::RawTimephasedBaseline6Work
            | F::RawTimephasedBaseline7Work
            | F::RawTimephasedBaseline8Work
            | F::RawTimephasedBaseline9Work
            | F::RawTimephasedBaseline10Work
            | F::RawTimephasedBudgetWork
            | F::RawTimephasedBaselineBudgetWork
            | F::RawTimephasedBaseline1BudgetWork
            | F::RawTimephasedBaseline2BudgetWork
            | F::RawTimephasedBaseline3BudgetWork
            | F::RawTimephasedBaseline4BudgetWork
            | F::RawTimephasedBaseline5BudgetWork
            | F::RawTimephasedBaseline6BudgetWork
            | F::RawTimephasedBaseline7BudgetWork
            | F::RawTimephasedBaseline8BudgetWork
            | F::RawTimephasedBaseline9BudgetWork
            | F::RawTimephasedBaseline10BudgetWork => {
                Some(FieldValue::TimephasedWork(Vec::new()))
            }
            F::RawTimephasedBaselineCost
            | F::RawTimephasedBaseline1Cost
            | F::RawTimephasedBaseline2Cost
            | F::RawTimephasedBaseline3Cost
            | F::RawTimephasedBaseline4Cost
            | F::RawTimephasedBaseline5Cost
            | F::RawTimephasedBaseline6Cost
            | F::RawTimephasedBaseline7Cost
            | F::RawTimephasedBaseline8Cost
            | F::RawTimephasedBaseline9Cost
            | F::RawTimephasedBaseline10Cost
            | F::RawTimephasedBudgetCost
            | F::RawTimephasedBaselineBudgetCost
            | F::RawTimephasedBaseline1BudgetCost
            | F::RawTimephasedBaseline2BudgetCost
            | F::RawTimephasedBaseline3BudgetCost
            | F::RawTimephasedBaseline4BudgetCost
            | F::RawTimephasedBaseline5BudgetCost
            | F::RawTimephasedBaseline6BudgetCost
            | F::RawTimephasedBaseline7BudgetCost
            | F::RawTimephasedBaseline8BudgetCost
            | F::RawTimephasedBaseline9BudgetCost
            | F::RawTimephasedBaseline10BudgetCost => {
                Some(FieldValue::TimephasedCost(Vec::new()))
            }
            _ => None,
        }
    }

/// Read the specified property field (internal convenience method).
    fn assignment_value(&self, field: AssignmentField) -> Option<FieldValue> {
        self.get_field(FieldKey::Assignment(field))
    }

/// Read a property field as `f64` (corresponding to Java's `Number`).
    fn number_value(&self, field: AssignmentField) -> Option<f64> {
        self.assignment_value(field).and_then(|value| value.as_double())
    }

/// Read a property field as `i32` (corresponding to Java's `Integer`).
    fn integer_value(&self, field: AssignmentField) -> Option<i32> {
        self.assignment_value(field).and_then(|value| value.as_integer())
    }

/// Read a property field as boolean (corresponding to Java's `BooleanHelper.getBoolean`).
    fn boolean_value(&self, field: AssignmentField) -> bool {
        BooleanHelper::get_boolean(self.assignment_value(field).and_then(|value| value.as_boolean()))
    }

/// Read a property field as a string.
    fn string_value(&self, field: AssignmentField) -> Option<String> {
        self.assignment_value(field).and_then(|value| match value {
            FieldValue::String(text) => Some(text),
            _ => None,
        })
    }

/// Read a property field as a duration.
    fn duration_value(&self, field: AssignmentField) -> Option<Duration> {
        self.assignment_value(field).and_then(|value| value.as_duration())
    }

/// Read a property field as a date-time.
    fn date_time_value(&self, field: AssignmentField) -> Option<NaiveDateTime> {
        self.assignment_value(field).and_then(|value| value.as_date_time())
    }

/// Read a property field as a rate.
    fn rate_value(&self, field: AssignmentField) -> Option<Rate> {
        self.assignment_value(field).and_then(|value| value.as_rate())
    }

/// Write a property field.
    fn set_assignment_value(&mut self, field: AssignmentField, value: Option<FieldValue>) {
        self.set_key(FieldKey::Assignment(field), value);
    }

/// Write a string property field.
    fn set_string_value(&mut self, field: AssignmentField, value: Option<String>) {
        self.set_assignment_value(field, value.map(FieldValue::String));
    }

/// Write an `int` property field (Java's boxed `Integer`).
    fn set_integer_value(&mut self, field: AssignmentField, value: i32) {
        self.set_assignment_value(field, Some(FieldValue::Integer(value)));
    }

/// Write a boolean property field (Java's boxed `Boolean`).
    ///
    /// Corresponds to Java: `private void set(FieldType field, boolean value)`
    fn set_boolean_value(&mut self, field: AssignmentField, value: bool) {
        self.set_assignment_value(field, Some(FieldValue::Boolean(value)));
    }

/// Map an index (1-based) to a field enum.
    ///
    /// Corresponds to Java: `private AssignmentField selectField(AssignmentField[] fields, int index)`
    ///
/// Java throws `IllegalArgumentException` on out of bounds; the Rust side returns `None` (without panicking).
    fn select_field(fields: &[AssignmentField], index: i32) -> Option<AssignmentField> {
        if index < 1 || index as usize > fields.len() {
            return None;
        }
        fields.get((index - 1) as usize).copied()
    }

/// Get the resource assignment's unique ID.
    ///
    /// Corresponds to Java: `@Override public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.integer_value(AssignmentField::UniqueId)
    }

/// Set the resource assignment's unique ID.
    ///
    /// Corresponds to Java: `@Override public void setUniqueID(Integer uniqueID)`
    pub fn set_unique_id(&mut self, unique_id: Option<i32>) {
        self.set_assignment_value(AssignmentField::UniqueId, unique_id.map(FieldValue::Integer));
    }

/// Get the assignment units (percentage).
    ///
    /// Corresponds to Java: `public Number getUnits()`
    pub fn get_units(&self) -> Option<f64> {
        self.number_value(AssignmentField::AssignmentUnits)
    }

/// Set the assignment units.
    ///
    /// Corresponds to Java: `public void setUnits(Number val)`
    pub fn set_units(&mut self, val: Option<f64>) {
        self.set_assignment_value(AssignmentField::AssignmentUnits, val.map(FieldValue::Double));
    }

/// Get the remaining assignment units.
    ///
    /// Corresponds to Java: `public Number getRemainingUnits()`
    pub fn get_remaining_units(&self) -> Option<f64> {
        self.number_value(AssignmentField::RemainingAssignmentUnits)
    }

/// Set the remaining assignment units.
    ///
    /// Corresponds to Java: `public void setRemainingUnits(Number val)`
    pub fn set_remaining_units(&mut self, val: Option<f64>) {
        self.set_assignment_value(
            AssignmentField::RemainingAssignmentUnits,
            val.map(FieldValue::Double),
        );
    }

/// Get the work.
    ///
    /// Corresponds to Java: `public Duration getWork()`
    pub fn get_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::Work)
    }

/// Set the work.
    ///
    /// Corresponds to Java: `public void setWork(Duration dur)`
    pub fn set_work(&mut self, dur: Option<Duration>) {
        self.set_assignment_value(AssignmentField::Work, dur.map(FieldValue::Duration));
    }

/// Get the baseline start.
    ///
    /// Corresponds to Java: `public LocalDateTime getBaselineStart()`
    pub fn get_baseline_start(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::BaselineStart)
    }

/// Set the baseline start.
    ///
    /// Corresponds to Java: `public void setBaselineStart(LocalDateTime start)`
    pub fn set_baseline_start(&mut self, start: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::BaselineStart, start.map(FieldValue::DateTime));
    }

/// Get the actual start.
    ///
    /// Corresponds to Java: `public LocalDateTime getActualStart()`
    pub fn get_actual_start(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::ActualStart)
    }

/// Set the actual start.
    ///
    /// Corresponds to Java: `public void setActualStart(LocalDateTime start)`
    pub fn set_actual_start(&mut self, start: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::ActualStart, start.map(FieldValue::DateTime));
    }

/// Get the baseline finish.
    ///
    /// Corresponds to Java: `public LocalDateTime getBaselineFinish()`
    pub fn get_baseline_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::BaselineFinish)
    }

/// Set the baseline finish.
    ///
    /// Corresponds to Java: `public void setBaselineFinish(LocalDateTime finish)`
    pub fn set_baseline_finish(&mut self, finish: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::BaselineFinish, finish.map(FieldValue::DateTime));
    }

/// Get the actual finish.
    ///
    /// Corresponds to Java: `public LocalDateTime getActualFinish()`
    pub fn get_actual_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::ActualFinish)
    }

/// Set the actual finish.
    ///
    /// Corresponds to Java: `public void setActualFinish(LocalDateTime finish)`
    pub fn set_actual_finish(&mut self, finish: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::ActualFinish, finish.map(FieldValue::DateTime));
    }

/// Get the baseline work.
    ///
    /// Corresponds to Java: `public Duration getBaselineWork()`
    pub fn get_baseline_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::BaselineWork)
    }

/// Set the baseline work.
    ///
    /// Corresponds to Java: `public void setBaselineWork(Duration val)`
    pub fn set_baseline_work(&mut self, val: Option<Duration>) {
        self.set_assignment_value(AssignmentField::BaselineWork, val.map(FieldValue::Duration));
    }

/// Get the actual work.
    ///
    /// Corresponds to Java: `public Duration getActualWork()`
    pub fn get_actual_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::ActualWork)
    }

/// Set the actual work.
    ///
    /// Corresponds to Java: `public void setActualWork(Duration val)`
    pub fn set_actual_work(&mut self, val: Option<Duration>) {
        self.set_assignment_value(AssignmentField::ActualWork, val.map(FieldValue::Duration));
    }

/// Get the overtime work.
    ///
    /// Corresponds to Java: `public Duration getOvertimeWork()`
    pub fn get_overtime_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::OvertimeWork)
    }

/// Set the overtime work.
    ///
    /// Corresponds to Java: `public void setOvertimeWork(Duration overtimeWork)`
    pub fn set_overtime_work(&mut self, overtime_work: Option<Duration>) {
        self.set_assignment_value(
            AssignmentField::OvertimeWork,
            overtime_work.map(FieldValue::Duration),
        );
    }

/// Get the cost.
    ///
    /// Corresponds to Java: `public Number getCost()`
    pub fn get_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::Cost)
    }

/// Set the cost.
    ///
    /// Corresponds to Java: `public void setCost(Number cost)`
    pub fn set_cost(&mut self, cost: Option<f64>) {
        self.set_assignment_value(AssignmentField::Cost, cost.map(FieldValue::Double));
    }

/// Get the baseline cost.
    ///
    /// Corresponds to Java: `public Number getBaselineCost()`
    pub fn get_baseline_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::BaselineCost)
    }

/// Set the baseline cost.
    ///
    /// Corresponds to Java: `public void setBaselineCost(Number val)`
    pub fn set_baseline_cost(&mut self, val: Option<f64>) {
        self.set_assignment_value(AssignmentField::BaselineCost, val.map(FieldValue::Double));
    }

/// Get the actual cost.
    ///
    /// Corresponds to Java: `public Number getActualCost()`
    pub fn get_actual_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::ActualCost)
    }

/// Set the actual cost.
    ///
    /// Corresponds to Java: `public void setActualCost(Number actualCost)`
    pub fn set_actual_cost(&mut self, actual_cost: Option<f64>) {
        self.set_assignment_value(AssignmentField::ActualCost, actual_cost.map(FieldValue::Double));
    }

/// Get the start (always recalculated).
    ///
    /// Corresponds to Java: `@Override public LocalDateTime getStart()`
    pub fn get_start(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::Start)
    }

/// Set the start.
    ///
    /// Corresponds to Java: `public void setStart(LocalDateTime val)`
    pub fn set_start(&mut self, val: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::Start, val.map(FieldValue::DateTime));
    }

/// Get the finish (always recalculated).
    ///
    /// Corresponds to Java: `@Override public LocalDateTime getFinish()`
    pub fn get_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::Finish)
    }

/// Set the finish.
    ///
    /// Corresponds to Java: `public void setFinish(LocalDateTime val)`
    pub fn set_finish(&mut self, val: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::Finish, val.map(FieldValue::DateTime));
    }

/// Get the assignment delay.
    ///
    /// Corresponds to Java: `public Duration getDelay()`
    pub fn get_delay(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::AssignmentDelay)
    }

/// Set the assignment delay.
    ///
    /// Corresponds to Java: `public void setDelay(Duration dur)`
    pub fn set_delay(&mut self, dur: Option<Duration>) {
        self.set_assignment_value(AssignmentField::AssignmentDelay, dur.map(FieldValue::Duration));
    }

/// Get the resource's unique ID.
    ///
    /// Corresponds to Java: `public Integer getResourceUniqueID()`
    pub fn get_resource_unique_id(&self) -> Option<i32> {
        self.integer_value(AssignmentField::ResourceUniqueId)
    }

/// Set the resource's unique ID.
    ///
    /// Corresponds to Java: `public void setResourceUniqueID(Integer val)`
    pub fn set_resource_unique_id(&mut self, val: Option<i32>) {
        self.set_assignment_value(AssignmentField::ResourceUniqueId, val.map(FieldValue::Integer));
    }

/// Get the task's unique ID.
    ///
    /// Corresponds to Java: `public Integer getTaskUniqueID()`
    pub fn get_task_unique_id(&self) -> Option<i32> {
        self.integer_value(AssignmentField::TaskUniqueId)
    }

/// Set the task's unique ID.
    ///
    /// Corresponds to Java: `public void setTaskUniqueID(Integer id)`
    pub fn set_task_unique_id(&mut self, id: Option<i32>) {
        self.set_assignment_value(AssignmentField::TaskUniqueId, id.map(FieldValue::Integer));
    }

/// Get the associated task.
    ///
    /// Corresponds to Java: `public Task getTask()`
    ///
/// The Rust side prefers the wired-up weak reference; when not wired up, it falls back to looking up the parent file by unique ID.
    pub fn get_task(&self) -> Option<Rc<RefCell<Task>>> {
        if let Some(task) = self.task.upgrade() {
            return Some(task);
        }

        let parent_file = self.parent_file.upgrade()?;
        let task_unique_id = self.get_task_unique_id();
        parent_file
            .try_borrow()
            .ok()
            .and_then(|parent_file| parent_file.get_task_by_unique_id(task_unique_id))
    }

/// Get the associated resource.
    ///
    /// Corresponds to Java: `public Resource getResource()`
    ///
/// The fallback strategy is the same as [`ResourceAssignment::get_task`].
    pub fn get_resource(&self) -> Option<Rc<RefCell<Resource>>> {
        if let Some(resource) = self.resource.upgrade() {
            return Some(resource);
        }

        let parent_file = self.parent_file.upgrade()?;
        let resource_unique_id = self.get_resource_unique_id();
        parent_file
            .try_borrow()
            .ok()
            .and_then(|parent_file| parent_file.get_resource_by_unique_id(resource_unique_id))
    }

/// Get the work contour type.
    ///
    /// Corresponds to Java: `public WorkContour getWorkContour()`
    pub fn get_work_contour(&self) -> Option<WorkContour> {
        match self.assignment_value(AssignmentField::WorkContour) {
            Some(FieldValue::WorkContour(work_contour)) => Some(work_contour),
            _ => None,
        }
    }

/// Set the work contour type.
    ///
    /// Corresponds to Java: `public void setWorkContour(WorkContour workContour)`
    pub fn set_work_contour(&mut self, work_contour: Option<WorkContour>) {
        self.set_assignment_value(
            AssignmentField::WorkContour,
            work_contour.map(FieldValue::WorkContour),
        );
    }

/// Remove this resource assignment from the project; returns `true` on success (Java has no return value).
    ///
    /// Corresponds to Java: `public void remove()`
    ///
/// Java locates the element by `this` reference equality; Rust cannot obtain an `Rc` from `&self`,
/// so it degrades to removal by unique ID (see section 5 of the file header).
    pub fn remove(&self) -> bool {
        let Some(parent_file) = self.parent_file.upgrade() else {
            return false;
        };

        let unique_id = self.get_unique_id();
        match parent_file.try_borrow_mut() {
            Ok(mut parent_file) => parent_file
                .get_resource_assignments_mut()
                .remove_by_unique_id(unique_id),
            Err(_) => false,
        }
    }

/// Get the remaining work.
    ///
    /// Corresponds to Java: `public Duration getRemainingWork()`
    pub fn get_remaining_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::RemainingWork)
    }

/// Set the remaining work.
    ///
    /// Corresponds to Java: `public void setRemainingWork(Duration remainingWork)`
    pub fn set_remaining_work(&mut self, remaining_work: Option<Duration>) {
        self.set_assignment_value(
            AssignmentField::RemainingWork,
            remaining_work.map(FieldValue::Duration),
        );
    }

/// Get the leveling delay.
    ///
    /// Corresponds to Java: `public Duration getLevelingDelay()`
    pub fn get_leveling_delay(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::LevelingDelay)
    }

/// Set the leveling delay.
    ///
    /// Corresponds to Java: `public void setLevelingDelay(Duration levelingDelay)`
    pub fn set_leveling_delay(&mut self, leveling_delay: Option<Duration>) {
        self.set_assignment_value(
            AssignmentField::LevelingDelay,
            leveling_delay.map(FieldValue::Duration),
        );
    }

/// Get the cost rate table index (default 0).
    ///
    /// Corresponds to Java: `public Integer getRateIndex()`
    pub fn get_rate_index(&self) -> Option<i32> {
        self.integer_value(AssignmentField::RateIndex)
    }

/// Set the cost rate table index.
    ///
    /// Corresponds to Java: `public void setRateIndex(Integer index)`
    pub fn set_rate_index(&mut self, index: Option<i32>) {
        self.set_assignment_value(AssignmentField::RateIndex, index.map(FieldValue::Integer));
    }

/// Get the role's unique ID.
    ///
    /// Corresponds to Java: `public Integer getRoleUniqueID()`
    pub fn get_role_unique_id(&self) -> Option<i32> {
        self.integer_value(AssignmentField::RoleUniqueId)
    }

/// Set the role's unique ID.
    ///
    /// Corresponds to Java: `public void setRoleUniqueID(Integer id)`
    pub fn set_role_unique_id(&mut self, id: Option<i32>) {
        self.set_assignment_value(AssignmentField::RoleUniqueId, id.map(FieldValue::Integer));
    }

/// Get the role (represented as a resource).
    ///
    /// Corresponds to Java: `public Resource getRole()`
    pub fn get_role(&self) -> Option<Rc<RefCell<Resource>>> {
        let parent_file = self.parent_file.upgrade()?;
        let role_unique_id = self.get_role_unique_id();
        parent_file
            .try_borrow()
            .ok()
            .and_then(|parent_file| parent_file.get_resource_by_unique_id(role_unique_id))
    }

/// Set the role (represented as a resource).
    ///
    /// Corresponds to Java: `public void setRole(Resource role)`
    pub fn set_role(&mut self, role: Option<&Rc<RefCell<Resource>>>) {
        let role_unique_id = role.and_then(|role| {
            role.try_borrow()
                .ok()
                .and_then(|role| role.get_unique_id())
        });
        self.set_assignment_value(
            AssignmentField::RoleUniqueId,
            role_unique_id.map(FieldValue::Integer),
        );
    }

/// Get the override rate.
    ///
    /// Corresponds to Java: `public Rate getOverrideRate()`
    pub fn get_override_rate(&self) -> Option<Rate> {
        self.rate_value(AssignmentField::OverrideRate)
    }

/// Set the override rate.
    ///
    /// Corresponds to Java: `public void setOverrideRate(Rate rate)`
    pub fn set_override_rate(&mut self, rate: Option<Rate>) {
        self.set_assignment_value(AssignmentField::OverrideRate, rate.map(FieldValue::Rate));
    }

/// Get the rate source (default `RateSource::Resource`).
    ///
    /// Corresponds to Java: `public RateSource getRateSource()`
    ///
/// Java's `get` falls back to the calculated field `defaultRateSource()` (`RateSource.RESOURCE`);
/// the Rust side likewise provides that default in [`ResourceAssignment::calculate`].
    pub fn get_rate_source(&self) -> RateSource {
        self.assignment_value(AssignmentField::RateSource)
            .and_then(|value| value.as_rate_source())
            .unwrap_or_else(Self::default_rate_source)
    }

/// Set the rate source.
    ///
    /// Corresponds to Java: `public void setRateSource(RateSource source)`
    pub fn set_rate_source(&mut self, source: Option<RateSource>) {
        self.set_assignment_value(
            AssignmentField::RateSource,
            source.map(FieldValue::RateSource),
        );
    }

/// The default value of the rate source.
    ///
    /// Corresponds to Java: `private RateSource defaultRateSource()`
    fn default_rate_source() -> RateSource {
        RateSource::Resource
    }

/// Get the effective calendar used by this assignment.
    ///
    /// Corresponds to Java: `public ProjectCalendar getEffectiveCalendar()`
    ///
    /// # Differences from Java
    ///
/// Java constructs a `CombinedCalendar` when "a task's explicit calendar and a resource calendar both exist";
/// this repository has not yet ported that combination; for manual scheduling it wraps another temporary calendar and falls back to the original calendar when date-times are missing.
    pub fn get_effective_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        let task = self.get_task()?;
        let task_calendar = task.try_borrow().ok().and_then(|task| task.get_calendar());
        let resource_calendar = self.resource_calendar(&task);

        let result = match task_calendar {
            None => match resource_calendar {
                None => task
                    .try_borrow()
                    .ok()
                    .and_then(|task| task.get_effective_calendar()),
                Some(resource_calendar) => Some(resource_calendar),
            },
            Some(task_calendar) => Some(task_calendar),
        };
        let manually_scheduled = task.try_borrow().ok()
            .and_then(|task| task.get_task_mode()) == Some(crate::task_mode::TaskMode::ManuallyScheduled);
        if manually_scheduled {
            result.and_then(|calendar| {
                crate::manually_scheduled_task_calendar::new(Rc::clone(&calendar), self)
                    .or(Some(calendar))
            })
        } else {
            result
        }
    }

/// Get the resource calendar available to the task (returns `None` when the task ignores resource calendars or the resource is not a time-based resource).
    ///
/// Corresponds to Java: the `resourceCalendar` retrieval logic in `getEffectiveCalendar()`.
    fn resource_calendar(&self, task: &Rc<RefCell<Task>>) -> Option<Rc<RefCell<ProjectCalendar>>> {
        let resource = self.get_resource()?;

        let ignore_resource_calendar = task
            .try_borrow()
            .ok()
            .map(|task| task.get_ignore_resource_calendar())
            .unwrap_or(false);
        if ignore_resource_calendar {
            return None;
        }

        let resource = resource.try_borrow().ok()?;
        if !resource.get_type().is_time_based() {
            return None;
        }

        resource.get_calendar()
    }

/// Get the variable rate units (returns `None` for a fixed rate).
    ///
    /// Corresponds to Java: `public TimeUnit getVariableRateUnits()`
    pub fn get_variable_rate_units(&self) -> Option<TimeUnit> {
        self.assignment_value(AssignmentField::VariableRateUnits)
            .and_then(|value| value.as_time_unit())
    }

/// Set the variable rate units.
    ///
    /// Corresponds to Java: `public void setVariableRateUnits(TimeUnit variableRateUnits)`
    pub fn set_variable_rate_units(&mut self, variable_rate_units: Option<TimeUnit>) {
        self.set_assignment_value(
            AssignmentField::VariableRateUnits,
            variable_rate_units.map(FieldValue::TimeUnit),
        );
    }

/// Get the budget cost.
    ///
    /// Corresponds to Java: `public Number getBudgetCost()`
    pub fn get_budget_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::BudgetCost)
    }

/// Set budget cost.
    ///
    /// Corresponds to Java: `public void setBudgetCost(Number cost)`
    pub fn set_budget_cost(&mut self, cost: Option<f64>) {
        self.set_assignment_value(AssignmentField::BudgetCost, cost.map(FieldValue::Double));
    }

/// Get budget work.
    ///
    /// Corresponds to Java: `public Duration getBudgetWork()`
    pub fn get_budget_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::BudgetWork)
    }

/// Set budget work.
    ///
    /// Corresponds to Java: `public void setBudgetWork(Duration work)`
    pub fn set_budget_work(&mut self, work: Option<Duration>) {
        self.set_assignment_value(AssignmentField::BudgetWork, work.map(FieldValue::Duration));
    }

/// Get baseline budget cost.
    ///
    /// Corresponds to Java: `public Number getBaselineBudgetCost()`
    pub fn get_baseline_budget_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::BaselineBudgetCost)
    }

/// Set baseline budget cost.
    ///
    /// Corresponds to Java: `public void setBaselineBudgetCost(Number cost)`
    pub fn set_baseline_budget_cost(&mut self, cost: Option<f64>) {
        self.set_assignment_value(
            AssignmentField::BaselineBudgetCost,
            cost.map(FieldValue::Double),
        );
    }

/// Get baseline budget work.
    ///
    /// Corresponds to Java: `public Duration getBaselineBudgetWork()`
    pub fn get_baseline_budget_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::BaselineBudgetWork)
    }

/// Set baseline budget work.
    ///
    /// Corresponds to Java: `public void setBaselineBudgetWork(Duration work)`
    pub fn set_baseline_budget_work(&mut self, work: Option<Duration>) {
        self.set_assignment_value(
            AssignmentField::BaselineBudgetWork,
            work.map(FieldValue::Duration),
        );
    }

/// Set baseline cost (index 1-10).
    ///
    /// Corresponds to Java: `public void setBaselineCost(int baselineNumber, Number value)`
    pub fn set_baseline_cost_by_index(&mut self, baseline_number: i32, value: Option<f64>) {
        if let Some(field) = Self::select_field(&BASELINE_COSTS, baseline_number) {
            self.set_assignment_value(field, value.map(FieldValue::Double));
        }
    }

/// Set baseline work (index 1-10).
    ///
    /// Corresponds to Java: `public void setBaselineWork(int baselineNumber, Duration value)`
    pub fn set_baseline_work_by_index(&mut self, baseline_number: i32, value: Option<Duration>) {
        if let Some(field) = Self::select_field(&BASELINE_WORKS, baseline_number) {
            self.set_assignment_value(field, value.map(FieldValue::Duration));
        }
    }

/// Get baseline work (index 1-10).
    ///
    /// Corresponds to Java: `public Duration getBaselineWork(int baselineNumber)`
    pub fn get_baseline_work_by_index(&self, baseline_number: i32) -> Option<Duration> {
        let field = Self::select_field(&BASELINE_WORKS, baseline_number)?;
        self.duration_value(field)
    }

/// Get baseline cost (index 1-10).
    ///
    /// Corresponds to Java: `public Number getBaselineCost(int baselineNumber)`
    pub fn get_baseline_cost_by_index(&self, baseline_number: i32) -> Option<f64> {
        let field = Self::select_field(&BASELINE_COSTS, baseline_number)?;
        self.number_value(field)
    }

/// Set baseline start (index 1-10).
    ///
    /// Corresponds to Java: `public void setBaselineStart(int baselineNumber, LocalDateTime value)`
    pub fn set_baseline_start_by_index(
        &mut self,
        baseline_number: i32,
        value: Option<NaiveDateTime>,
    ) {
        if let Some(field) = Self::select_field(&BASELINE_STARTS, baseline_number) {
            self.set_assignment_value(field, value.map(FieldValue::DateTime));
        }
    }

/// Get baseline start (index 1-10).
    ///
    /// Corresponds to Java: `public LocalDateTime getBaselineStart(int baselineNumber)`
    pub fn get_baseline_start_by_index(&self, baseline_number: i32) -> Option<NaiveDateTime> {
        let field = Self::select_field(&BASELINE_STARTS, baseline_number)?;
        self.date_time_value(field)
    }

/// Set baseline finish (index 1-10).
    ///
    /// Corresponds to Java: `public void setBaselineFinish(int baselineNumber, LocalDateTime value)`
    pub fn set_baseline_finish_by_index(
        &mut self,
        baseline_number: i32,
        value: Option<NaiveDateTime>,
    ) {
        if let Some(field) = Self::select_field(&BASELINE_FINISHES, baseline_number) {
            self.set_assignment_value(field, value.map(FieldValue::DateTime));
        }
    }

/// Get baseline finish (index 1-10).
    ///
    /// Corresponds to Java: `public LocalDateTime getBaselineFinish(int baselineNumber)`
    pub fn get_baseline_finish_by_index(&self, baseline_number: i32) -> Option<NaiveDateTime> {
        let field = Self::select_field(&BASELINE_FINISHES, baseline_number)?;
        self.date_time_value(field)
    }

/// Set baseline budget cost (index 1-10).
    ///
    /// Corresponds to Java: `public void setBaselineBudgetCost(int baselineNumber, Number value)`
    pub fn set_baseline_budget_cost_by_index(&mut self, baseline_number: i32, value: Option<f64>) {
        if let Some(field) = Self::select_field(&BASELINE_BUDGET_COSTS, baseline_number) {
            self.set_assignment_value(field, value.map(FieldValue::Double));
        }
    }

/// Set baseline budget work (index 1-10).
    ///
    /// Corresponds to Java: `public void setBaselineBudgetWork(int baselineNumber, Duration value)`
    pub fn set_baseline_budget_work_by_index(
        &mut self,
        baseline_number: i32,
        value: Option<Duration>,
    ) {
        if let Some(field) = Self::select_field(&BASELINE_BUDGET_WORKS, baseline_number) {
            self.set_assignment_value(field, value.map(FieldValue::Duration));
        }
    }

/// Get baseline budget work (index 1-10).
    ///
    /// Corresponds to Java: `public Duration getBaselineBudgetWork(int baselineNumber)`
    pub fn get_baseline_budget_work_by_index(&self, baseline_number: i32) -> Option<Duration> {
        let field = Self::select_field(&BASELINE_BUDGET_WORKS, baseline_number)?;
        self.duration_value(field)
    }

/// Get baseline budget cost (index 1-10).
    ///
    /// Corresponds to Java: `public Number getBaselineBudgetCost(int baselineNumber)`
    pub fn get_baseline_budget_cost_by_index(&self, baseline_number: i32) -> Option<f64> {
        let field = Self::select_field(&BASELINE_BUDGET_COSTS, baseline_number)?;
        self.number_value(field)
    }

/// Set custom text (index 1-30).
    ///
    /// Corresponds to Java: `public void setText(int index, String value)`
    pub fn set_text(&mut self, index: i32, value: Option<String>) {
        if let Some(field) = Self::select_field(&CUSTOM_TEXT, index) {
            self.set_string_value(field, value);
        }
    }

/// Get custom text (index 1-30).
    ///
    /// Corresponds to Java: `public String getText(int index)`
    pub fn get_text(&self, index: i32) -> Option<String> {
        let field = Self::select_field(&CUSTOM_TEXT, index)?;
        self.string_value(field)
    }

/// Set custom start (index 1-10).
    ///
    /// Corresponds to Java: `public void setStart(int index, LocalDateTime value)`
    pub fn set_start_by_index(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = Self::select_field(&CUSTOM_START, index) {
            self.set_assignment_value(field, value.map(FieldValue::DateTime));
        }
    }

/// Get custom start (index 1-10).
    ///
    /// Corresponds to Java: `public LocalDateTime getStart(int index)`
    pub fn get_start_by_index(&self, index: i32) -> Option<NaiveDateTime> {
        let field = Self::select_field(&CUSTOM_START, index)?;
        self.date_time_value(field)
    }

/// Set custom finish (index 1-10).
    ///
    /// Corresponds to Java: `public void setFinish(int index, LocalDateTime value)`
    pub fn set_finish_by_index(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = Self::select_field(&CUSTOM_FINISH, index) {
            self.set_assignment_value(field, value.map(FieldValue::DateTime));
        }
    }

/// Get custom finish (index 1-10).
    ///
    /// Corresponds to Java: `public LocalDateTime getFinish(int index)`
    pub fn get_finish_by_index(&self, index: i32) -> Option<NaiveDateTime> {
        let field = Self::select_field(&CUSTOM_FINISH, index)?;
        self.date_time_value(field)
    }

/// Set custom date (index 1-10).
    ///
    /// Corresponds to Java: `public void setDate(int index, LocalDateTime value)`
    pub fn set_date(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = Self::select_field(&CUSTOM_DATE, index) {
            self.set_assignment_value(field, value.map(FieldValue::DateTime));
        }
    }

/// Get custom date (index 1-10).
    ///
    /// Corresponds to Java: `public LocalDateTime getDate(int index)`
    pub fn get_date(&self, index: i32) -> Option<NaiveDateTime> {
        let field = Self::select_field(&CUSTOM_DATE, index)?;
        self.date_time_value(field)
    }

/// Set custom number (index 1-20).
    ///
    /// Corresponds to Java: `public void setNumber(int index, Number value)`
    pub fn set_number(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = Self::select_field(&CUSTOM_NUMBER, index) {
            self.set_assignment_value(field, value.map(FieldValue::Double));
        }
    }

/// Get custom number (index 1-20).
    ///
    /// Corresponds to Java: `public Number getNumber(int index)`
    pub fn get_number(&self, index: i32) -> Option<f64> {
        let field = Self::select_field(&CUSTOM_NUMBER, index)?;
        self.number_value(field)
    }

/// Set custom duration (index 1-10).
    ///
    /// Corresponds to Java: `public void setDuration(int index, Duration value)`
    pub fn set_duration(&mut self, index: i32, value: Option<Duration>) {
        if let Some(field) = Self::select_field(&CUSTOM_DURATION, index) {
            self.set_assignment_value(field, value.map(FieldValue::Duration));
        }
    }

/// Get custom duration (index 1-10).
    ///
    /// Corresponds to Java: `public Duration getDuration(int index)`
    pub fn get_duration(&self, index: i32) -> Option<Duration> {
        let field = Self::select_field(&CUSTOM_DURATION, index)?;
        self.duration_value(field)
    }

/// Set custom cost (index 1-10).
    ///
    /// Corresponds to Java: `public void setCost(int index, Number value)`
    pub fn set_cost_by_index(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = Self::select_field(&CUSTOM_COST, index) {
            self.set_assignment_value(field, value.map(FieldValue::Double));
        }
    }

/// Get custom cost (index 1-10).
    ///
    /// Corresponds to Java: `public Number getCost(int index)`
    pub fn get_cost_by_index(&self, index: i32) -> Option<f64> {
        let field = Self::select_field(&CUSTOM_COST, index)?;
        self.number_value(field)
    }

/// Set custom flag (index 1-20).
    ///
    /// Corresponds to Java: `public void setFlag(int index, boolean value)`
    pub fn set_flag(&mut self, index: i32, value: bool) {
        if let Some(field) = Self::select_field(&CUSTOM_FLAG, index) {
            self.set_boolean_value(field, value);
        }
    }

/// Get custom flag (index 1-20); returns `false` when unset or out of bounds.
    ///
    /// Corresponds to Java: `public boolean getFlag(int index)`
    pub fn get_flag(&self, index: i32) -> bool {
        match Self::select_field(&CUSTOM_FLAG, index) {
            Some(field) => self.boolean_value(field),
            None => false,
        }
    }

/// Set enterprise custom cost (index 1-10).
    ///
    /// Corresponds to Java: `public void setEnterpriseCost(int index, Number value)`
    pub fn set_enterprise_cost(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = Self::select_field(&ENTERPRISE_CUSTOM_COST, index) {
            self.set_assignment_value(field, value.map(FieldValue::Double));
        }
    }

/// Get enterprise custom cost (index 1-10).
    ///
    /// Corresponds to Java: `public Number getEnterpriseCost(int index)`
    pub fn get_enterprise_cost(&self, index: i32) -> Option<f64> {
        let field = Self::select_field(&ENTERPRISE_CUSTOM_COST, index)?;
        self.number_value(field)
    }

/// Set enterprise custom date (index 1-30).
    ///
    /// Corresponds to Java: `public void setEnterpriseDate(int index, LocalDateTime value)`
    pub fn set_enterprise_date(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = Self::select_field(&ENTERPRISE_CUSTOM_DATE, index) {
            self.set_assignment_value(field, value.map(FieldValue::DateTime));
        }
    }

/// Get enterprise custom date (index 1-30).
    ///
    /// Corresponds to Java: `public LocalDateTime getEnterpriseDate(int index)`
    pub fn get_enterprise_date(&self, index: i32) -> Option<NaiveDateTime> {
        let field = Self::select_field(&ENTERPRISE_CUSTOM_DATE, index)?;
        self.date_time_value(field)
    }

/// Set enterprise custom duration (index 1-10).
    ///
    /// Corresponds to Java: `public void setEnterpriseDuration(int index, Duration value)`
    pub fn set_enterprise_duration(&mut self, index: i32, value: Option<Duration>) {
        if let Some(field) = Self::select_field(&ENTERPRISE_CUSTOM_DURATION, index) {
            self.set_assignment_value(field, value.map(FieldValue::Duration));
        }
    }

/// Get enterprise custom duration (index 1-10).
    ///
    /// Corresponds to Java: `public Duration getEnterpriseDuration(int index)`
    pub fn get_enterprise_duration(&self, index: i32) -> Option<Duration> {
        let field = Self::select_field(&ENTERPRISE_CUSTOM_DURATION, index)?;
        self.duration_value(field)
    }

/// Set enterprise custom flag (index 1-20).
    ///
    /// Corresponds to Java: `public void setEnterpriseFlag(int index, boolean value)`
    pub fn set_enterprise_flag(&mut self, index: i32, value: bool) {
        if let Some(field) = Self::select_field(&ENTERPRISE_CUSTOM_FLAG, index) {
            self.set_boolean_value(field, value);
        }
    }

/// Get enterprise custom flag (index 1-20); returns `false` when unset or out of bounds.
    ///
    /// Corresponds to Java: `public boolean getEnterpriseFlag(int index)`
    pub fn get_enterprise_flag(&self, index: i32) -> bool {
        match Self::select_field(&ENTERPRISE_CUSTOM_FLAG, index) {
            Some(field) => self.boolean_value(field),
            None => false,
        }
    }

/// Set enterprise custom number (index 1-40).
    ///
    /// Corresponds to Java: `public void setEnterpriseNumber(int index, Number value)`
    pub fn set_enterprise_number(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = Self::select_field(&ENTERPRISE_CUSTOM_NUMBER, index) {
            self.set_assignment_value(field, value.map(FieldValue::Double));
        }
    }

/// Get enterprise custom number (index 1-40).
    ///
    /// Corresponds to Java: `public Number getEnterpriseNumber(int index)`
    pub fn get_enterprise_number(&self, index: i32) -> Option<f64> {
        let field = Self::select_field(&ENTERPRISE_CUSTOM_NUMBER, index)?;
        self.number_value(field)
    }

/// Set enterprise custom text (index 1-40).
    ///
    /// Corresponds to Java: `public void setEnterpriseText(int index, String value)`
    pub fn set_enterprise_text(&mut self, index: i32, value: Option<String>) {
        if let Some(field) = Self::select_field(&ENTERPRISE_CUSTOM_TEXT, index) {
            self.set_string_value(field, value);
        }
    }

/// Get enterprise custom text (index 1-40).
    ///
    /// Corresponds to Java: `public String getEnterpriseText(int index)`
    pub fn get_enterprise_text(&self, index: i32) -> Option<String> {
        let field = Self::select_field(&ENTERPRISE_CUSTOM_TEXT, index)?;
        self.string_value(field)
    }

/// Get regular work.
    ///
    /// Corresponds to Java: `public Duration getRegularWork()`
    pub fn get_regular_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::RegularWork)
    }

/// Set regular work.
    ///
    /// Corresponds to Java: `public void setRegularWork(Duration dur)`
    pub fn set_regular_work(&mut self, dur: Option<Duration>) {
        self.set_assignment_value(AssignmentField::RegularWork, dur.map(FieldValue::Duration));
    }

/// Get actual overtime work.
    ///
    /// Corresponds to Java: `public Duration getActualOvertimeWork()`
    pub fn get_actual_overtime_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::ActualOvertimeWork)
    }

/// Set actual overtime work.
    ///
    /// Corresponds to Java: `public void setActualOvertimeWork(Duration dur)`
    pub fn set_actual_overtime_work(&mut self, dur: Option<Duration>) {
        self.set_assignment_value(
            AssignmentField::ActualOvertimeWork,
            dur.map(FieldValue::Duration),
        );
    }

/// Get remaining overtime work.
    ///
    /// Corresponds to Java: `public Duration getRemainingOvertimeWork()`
    pub fn get_remaining_overtime_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::RemainingOvertimeWork)
    }

/// Set remaining overtime work.
    ///
    /// Corresponds to Java: `public void setRemainingOvertimeWork(Duration dur)`
    pub fn set_remaining_overtime_work(&mut self, dur: Option<Duration>) {
        self.set_assignment_value(
            AssignmentField::RemainingOvertimeWork,
            dur.map(FieldValue::Duration),
        );
    }

/// Get overtime cost (calculated field).
    ///
    /// Corresponds to Java: `public Number getOvertimeCost()`
    pub fn get_overtime_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::OvertimeCost)
    }

/// Set overtime cost.
    ///
    /// Corresponds to Java: `public void setOvertimeCost(Number cost)`
    pub fn set_overtime_cost(&mut self, cost: Option<f64>) {
        self.set_assignment_value(AssignmentField::OvertimeCost, cost.map(FieldValue::Double));
    }

/// Get remaining cost.
    ///
    /// Corresponds to Java: `public Number getRemainingCost()`
    pub fn get_remaining_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::RemainingCost)
    }

/// Set remaining cost.
    ///
    /// Corresponds to Java: `public void setRemainingCost(Number cost)`
    pub fn set_remaining_cost(&mut self, cost: Option<f64>) {
        self.set_assignment_value(AssignmentField::RemainingCost, cost.map(FieldValue::Double));
    }

/// Get actual overtime cost.
    ///
    /// Corresponds to Java: `public Number getActualOvertimeCost()`
    pub fn get_actual_overtime_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::ActualOvertimeCost)
    }

/// Set actual overtime cost.
    ///
    /// Corresponds to Java: `public void setActualOvertimeCost(Number cost)`
    pub fn set_actual_overtime_cost(&mut self, cost: Option<f64>) {
        self.set_assignment_value(
            AssignmentField::ActualOvertimeCost,
            cost.map(FieldValue::Double),
        );
    }

/// Get remaining overtime cost.
    ///
    /// Corresponds to Java: `public Number getRemainingOvertimeCost()`
    pub fn get_remaining_overtime_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::RemainingOvertimeCost)
    }

/// Set remaining overtime cost.
    ///
    /// Corresponds to Java: `public void setRemainingOvertimeCost(Number cost)`
    pub fn set_remaining_overtime_cost(&mut self, cost: Option<f64>) {
        self.set_assignment_value(
            AssignmentField::RemainingOvertimeCost,
            cost.map(FieldValue::Double),
        );
    }

/// Set BCWP (budgeted cost of work performed).
    ///
    /// Corresponds to Java: `public void setBCWP(Number val)`
    pub fn set_bcwp(&mut self, val: Option<f64>) {
        self.set_assignment_value(AssignmentField::Bcwp, val.map(FieldValue::Double));
    }

/// Get BCWP.
    ///
    /// Corresponds to Java: `public Number getBCWP()`
    pub fn get_bcwp(&self) -> Option<f64> {
        self.number_value(AssignmentField::Bcwp)
    }

/// Set BCWS (budgeted cost of work scheduled).
    ///
    /// Corresponds to Java: `public void setBCWS(Number val)`
    pub fn set_bcws(&mut self, val: Option<f64>) {
        self.set_assignment_value(AssignmentField::Bcws, val.map(FieldValue::Double));
    }

/// Get BCWS.
    ///
    /// Corresponds to Java: `public Number getBCWS()`
    pub fn get_bcws(&self) -> Option<f64> {
        self.number_value(AssignmentField::Bcws)
    }

/// Get ACWP (actual cost of work performed).
    ///
    /// Corresponds to Java: `public Number getACWP()`
    pub fn get_acwp(&self) -> Option<f64> {
        self.number_value(AssignmentField::Acwp)
    }

/// Set ACWP.
    ///
    /// Corresponds to Java: `public void setACWP(Number acwp)`
    pub fn set_acwp(&mut self, acwp: Option<f64>) {
        self.set_assignment_value(AssignmentField::Acwp, acwp.map(FieldValue::Double));
    }

/// Set SV (schedule variance).
    ///
    /// Corresponds to Java: `public void setSV(Number val)`
    pub fn set_sv(&mut self, val: Option<f64>) {
        self.set_assignment_value(AssignmentField::Sv, val.map(FieldValue::Double));
    }

/// Get SV (calculated field).
    ///
    /// Corresponds to Java: `public Number getSV()`
    pub fn get_sv(&self) -> Option<f64> {
        self.number_value(AssignmentField::Sv)
    }

/// Set CV (cost variance).
    ///
    /// Corresponds to Java: `public void setCV(Number val)`
    pub fn set_cv(&mut self, val: Option<f64>) {
        self.set_assignment_value(AssignmentField::Cv, val.map(FieldValue::Double));
    }

/// Get CV (calculated field).
    ///
    /// Corresponds to Java: `public Number getCV()`
    pub fn get_cv(&self) -> Option<f64> {
        self.number_value(AssignmentField::Cv)
    }

/// Set cost variance (VAC).
    ///
    /// Corresponds to Java: `public void setCostVariance(Number val)`
    pub fn set_cost_variance(&mut self, val: Option<f64>) {
        self.set_assignment_value(AssignmentField::CostVariance, val.map(FieldValue::Double));
    }

/// Get cost variance (calculated field).
    ///
    /// Corresponds to Java: `public Number getCostVariance()`
    pub fn get_cost_variance(&self) -> Option<f64> {
        self.number_value(AssignmentField::CostVariance)
    }

/// Set percent work complete.
    ///
    /// Corresponds to Java: `public void setPercentageWorkComplete(Number val)`
    pub fn set_percentage_work_complete(&mut self, val: Option<f64>) {
        self.set_assignment_value(
            AssignmentField::PercentWorkComplete,
            val.map(FieldValue::Double),
        );
    }

/// Get percent work complete (calculated field).
    ///
    /// Corresponds to Java: `public Number getPercentageWorkComplete()`
    pub fn get_percentage_work_complete(&self) -> Option<f64> {
        self.number_value(AssignmentField::PercentWorkComplete)
    }

/// Set plain text notes (Java stores `new Notes(notes)`; the Rust side uses `PlainNotes`).
    ///
    /// Corresponds to Java: `public void setNotes(String notes)`
    pub fn set_notes(&mut self, notes: Option<String>) {
        let value = notes.map(|text| {
            let notes: Rc<dyn Notes> = Rc::new(PlainNotes::new(Some(text)));
            FieldValue::Notes(notes)
        });
        self.set_assignment_value(AssignmentField::Notes, value);
    }

/// Get the plain text representation of the notes; returns an empty string when there are no notes.
    ///
    /// Corresponds to Java: `public String getNotes()`
    pub fn get_notes(&self) -> String {
        self.notes_object()
            .map(|notes| notes.to_string())
            .unwrap_or_default()
    }

/// Set the notes object (subtypes such as `RtfNotes` / `HtmlNotes` / `StructuredNotes` are stored
/// faithfully via `Rc<dyn Notes>`).
    ///
    /// Corresponds to Java: `public void setNotesObject(Notes notes)`
    pub fn set_notes_object(&mut self, notes: Option<Box<dyn Notes>>) {
        let value = notes.map(|notes| FieldValue::Notes(Rc::from(notes)));
        self.set_assignment_value(AssignmentField::Notes, value);
    }

/// Get the notes object.
    ///
    /// Corresponds to Java: `public Notes getNotesObject()`
    pub fn get_notes_object(&self) -> Option<Rc<dyn Notes>> {
        self.notes_object()
    }

/// Get the notes object (internal convenience method).
    fn notes_object(&self) -> Option<Rc<dyn Notes>> {
        self.assignment_value(AssignmentField::Notes)
            .and_then(|value| value.as_notes().cloned())
    }

/// Set the "confirmed" flag.
    ///
    /// Corresponds to Java: `public void setConfirmed(boolean val)`
    pub fn set_confirmed(&mut self, val: bool) {
        self.set_boolean_value(AssignmentField::Confirmed, val);
    }

/// Get the "confirmed" flag.
    ///
    /// Corresponds to Java: `public boolean getConfirmed()`
    pub fn get_confirmed(&self) -> bool {
        self.boolean_value(AssignmentField::Confirmed)
    }

/// Set the "update needed" flag.
    ///
    /// Corresponds to Java: `public void setUpdateNeeded(boolean val)`
    pub fn set_update_needed(&mut self, val: bool) {
        self.set_boolean_value(AssignmentField::UpdateNeeded, val);
    }

/// Get the "update needed" flag.
    ///
    /// Corresponds to Java: `public boolean getUpdateNeeded()`
    pub fn get_update_needed(&self) -> bool {
        self.boolean_value(AssignmentField::UpdateNeeded)
    }

/// Set the "linked fields" flag.
    ///
    /// Corresponds to Java: `public void setLinkedFields(boolean flag)`
    pub fn set_linked_fields(&mut self, flag: bool) {
        self.set_boolean_value(AssignmentField::LinkedFields, flag);
    }

/// Get the "linked fields" flag.
    ///
    /// Corresponds to Java: `public boolean getLinkedFields()`
    pub fn get_linked_fields(&self) -> bool {
        self.boolean_value(AssignmentField::LinkedFields)
    }

/// Get the hyperlink.
    ///
    /// Corresponds to Java: `public String getHyperlink()`
    pub fn get_hyperlink(&self) -> Option<String> {
        self.string_value(AssignmentField::Hyperlink)
    }

/// Get the hyperlink address.
    ///
    /// Corresponds to Java: `public String getHyperlinkAddress()`
    pub fn get_hyperlink_address(&self) -> Option<String> {
        self.string_value(AssignmentField::HyperlinkAddress)
    }

/// Get the hyperlink subaddress.
    ///
    /// Corresponds to Java: `public String getHyperlinkSubAddress()`
    pub fn get_hyperlink_sub_address(&self) -> Option<String> {
        self.string_value(AssignmentField::HyperlinkSubaddress)
    }

/// Set the hyperlink.
    ///
    /// Corresponds to Java: `public void setHyperlink(String text)`
    pub fn set_hyperlink(&mut self, text: Option<String>) {
        self.set_string_value(AssignmentField::Hyperlink, text);
    }

/// Set the hyperlink address.
    ///
    /// Corresponds to Java: `public void setHyperlinkAddress(String text)`
    pub fn set_hyperlink_address(&mut self, text: Option<String>) {
        self.set_string_value(AssignmentField::HyperlinkAddress, text);
    }

/// Set the hyperlink subaddress.
    ///
    /// Corresponds to Java: `public void setHyperlinkSubAddress(String text)`
    pub fn set_hyperlink_sub_address(&mut self, text: Option<String>) {
        self.set_string_value(AssignmentField::HyperlinkSubaddress, text);
    }

/// Set work variance.
    ///
    /// Corresponds to Java: `public void setWorkVariance(Duration val)`
    pub fn set_work_variance(&mut self, val: Option<Duration>) {
        self.set_assignment_value(AssignmentField::WorkVariance, val.map(FieldValue::Duration));
    }

/// Get work variance (calculated field).
    ///
    /// Corresponds to Java: `public Duration getWorkVariance()`
    pub fn get_work_variance(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::WorkVariance)
    }

/// Set start variance.
    ///
    /// Corresponds to Java: `public void setStartVariance(Duration val)`
    pub fn set_start_variance(&mut self, val: Option<Duration>) {
        self.set_assignment_value(AssignmentField::StartVariance, val.map(FieldValue::Duration));
    }

/// Get start variance (calculated field).
    ///
    /// Corresponds to Java: `public Duration getStartVariance()`
    pub fn get_start_variance(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::StartVariance)
    }

/// Set finish variance.
    ///
    /// Corresponds to Java: `public void setFinishVariance(Duration duration)`
    pub fn set_finish_variance(&mut self, duration: Option<Duration>) {
        self.set_assignment_value(
            AssignmentField::FinishVariance,
            duration.map(FieldValue::Duration),
        );
    }

/// Get finish variance (calculated field).
    ///
    /// Corresponds to Java: `public Duration getFinishVariance()`
    pub fn get_finish_variance(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::FinishVariance)
    }

/// Get the creation time.
    ///
    /// Corresponds to Java: `public LocalDateTime getCreateDate()`
    pub fn get_create_date(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::Created)
    }

/// Set the creation time.
    ///
    /// Corresponds to Java: `public void setCreateDate(LocalDateTime val)`
    pub fn set_create_date(&mut self, val: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::Created, val.map(FieldValue::DateTime));
    }

/// Get the assignment GUID.
    ///
    /// Corresponds to Java: `public UUID getGUID()`
    pub fn get_guid(&self) -> Option<Uuid> {
        self.assignment_value(AssignmentField::Guid)
            .and_then(|value| value.as_uuid())
    }

/// Set the assignment GUID.
    ///
    /// Corresponds to Java: `public void setGUID(UUID value)`
    pub fn set_guid(&mut self, value: Option<Uuid>) {
        self.set_assignment_value(AssignmentField::Guid, value.map(FieldValue::Uuid));
    }

/// Set the "response pending" flag.
    ///
    /// Corresponds to Java: `public void setResponsePending(boolean val)`
    pub fn set_response_pending(&mut self, val: bool) {
        self.set_boolean_value(AssignmentField::ResponsePending, val);
    }

/// Get the "response pending" flag.
    ///
    /// Corresponds to Java: `public boolean getResponsePending()`
    pub fn get_response_pending(&self) -> bool {
        self.boolean_value(AssignmentField::ResponsePending)
    }

/// Set the "team status pending" flag.
    ///
    /// Corresponds to Java: `public void setTeamStatusPending(boolean val)`
    pub fn set_team_status_pending(&mut self, val: bool) {
        self.set_boolean_value(AssignmentField::TeamStatusPending, val);
    }

/// Get the "team status pending" flag.
    ///
    /// Corresponds to Java: `public boolean getTeamStatusPending()`
    pub fn get_team_status_pending(&self) -> bool {
        self.boolean_value(AssignmentField::TeamStatusPending)
    }

/// Set VAC.
    ///
    /// Corresponds to Java: `public void setVAC(Number value)`
    pub fn set_vac(&mut self, value: Option<f64>) {
        self.set_assignment_value(AssignmentField::Vac, value.map(FieldValue::Double));
    }

/// Get VAC.
    ///
    /// Corresponds to Java: `public Number getVAC()`
    pub fn get_vac(&self) -> Option<f64> {
        self.number_value(AssignmentField::Vac)
    }

/// Set the cost rate table index (the `int` version).
    ///
    /// Corresponds to Java: `public void setCostRateTableIndex(int index)`
    pub fn set_cost_rate_table_index(&mut self, index: i32) {
        self.set_integer_value(AssignmentField::CostRateTable, index);
    }

/// Get the cost rate table index; returns 0 when out of bounds.
    ///
    /// Corresponds to Java: `public int getCostRateTableIndex()`
    pub fn get_cost_rate_table_index(&self) -> i32 {
        let value = NumberHelper::get_int(self.number_value(AssignmentField::CostRateTable));
        if value < 0 || value >= CostRateTable::MAX_TABLES {
            0
        } else {
            value
        }
    }

/// Perform the given operation on the cost rate table used by this assignment.
    ///
    /// Corresponds to Java: `public CostRateTable getCostRateTable()`
    ///
    /// # Differences from Java
    ///
/// Java returns a `CostRateTable` reference; the Rust side holds rate tables by value
/// (`Resource::get_cost_rate_table` returns a borrow) and `CostRateTable` does not implement `Clone`,
/// so returning ownership is not possible; hence the callback form (see section 4 of the file header). When the table is absent, invoke the callback with `None`
/// (corresponding to Java returning `null`).
    pub fn with_cost_rate_table<T>(&self, f: impl FnOnce(Option<&CostRateTable>) -> T) -> T {
// Java: there is no rate table when the rate source is override
        if self.get_rate_source() == RateSource::Override {
            return f(None);
        }

// Java: returns null when no resource/role is found
        let resource = if self.get_rate_source() == RateSource::Role {
            self.get_role()
        } else {
            self.get_resource()
        };

        let Some(resource) = resource else {
            return f(None);
        };

        let index = self.get_cost_rate_table_index();
        match resource.try_borrow() {
            Ok(resource) => f(resource.get_cost_rate_table(index)),
            Err(_) => f(None),
        }
    }

/// Get the hyperlink screen tip.
    ///
    /// Corresponds to Java: `public String getHyperlinkScreenTip()`
    pub fn get_hyperlink_screen_tip(&self) -> Option<String> {
        self.string_value(AssignmentField::HyperlinkScreenTip)
    }

/// Set the hyperlink screen tip.
    ///
    /// Corresponds to Java: `public void setHyperlinkScreenTip(String text)`
    pub fn set_hyperlink_screen_tip(&mut self, text: Option<String>) {
        self.set_string_value(AssignmentField::HyperlinkScreenTip, text);
    }

/// Get the resource request type.
    ///
    /// Corresponds to Java: `public ResourceRequestType getResourceRequestType()`
    pub fn get_resource_request_type(&self) -> Option<ResourceRequestType> {
        self.assignment_value(AssignmentField::ResourceRequestType)
            .and_then(|value| value.as_resource_request_type())
    }

/// Set the resource request type.
    ///
    /// Corresponds to Java: `public void setResourceRequestType(ResourceRequestType type)`
    pub fn set_resource_request_type(&mut self, r#type: Option<ResourceRequestType>) {
        self.set_assignment_value(
            AssignmentField::ResourceRequestType,
            r#type.map(FieldValue::ResourceRequestType),
        );
    }

/// Get the stop time.
    ///
    /// Corresponds to Java: `public LocalDateTime getStop()`
    pub fn get_stop(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::Stop)
    }

/// Set the stop time.
    ///
    /// Corresponds to Java: `public void setStop(LocalDateTime stop)`
    pub fn set_stop(&mut self, stop: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::Stop, stop.map(FieldValue::DateTime));
    }

/// Get the resume time.
    ///
    /// Corresponds to Java: `public LocalDateTime getResume()`
    pub fn get_resume(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::Resume)
    }

/// Set the resume time.
    ///
    /// Corresponds to Java: `public void setResume(LocalDateTime resume)`
    pub fn set_resume(&mut self, resume: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::Resume, resume.map(FieldValue::DateTime));
    }

/// Get planned work.
    ///
    /// Corresponds to Java: `public Duration getPlannedWork()`
    pub fn get_planned_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::PlannedWork)
    }

/// Set planned work.
    ///
    /// Corresponds to Java: `public void setPlannedWork(Duration value)`
    pub fn set_planned_work(&mut self, value: Option<Duration>) {
        self.set_assignment_value(AssignmentField::PlannedWork, value.map(FieldValue::Duration));
    }

/// Get planned cost.
    ///
    /// Corresponds to Java: `public Number getPlannedCost()`
    pub fn get_planned_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::PlannedCost)
    }

/// Set planned cost.
    ///
    /// Corresponds to Java: `public void setPlannedCost(Number value)`
    pub fn set_planned_cost(&mut self, value: Option<f64>) {
        self.set_assignment_value(AssignmentField::PlannedCost, value.map(FieldValue::Double));
    }

/// Get planned start.
    ///
    /// Corresponds to Java: `public LocalDateTime getPlannedStart()`
    pub fn get_planned_start(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::PlannedStart)
    }

/// Set planned start.
    ///
    /// Corresponds to Java: `public void setPlannedStart(LocalDateTime value)`
    pub fn set_planned_start(&mut self, value: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::PlannedStart, value.map(FieldValue::DateTime));
    }

/// Get planned finish.
    ///
    /// Corresponds to Java: `public LocalDateTime getPlannedFinish()`
    pub fn get_planned_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::PlannedFinish)
    }

/// Set planned finish.
    ///
    /// Corresponds to Java: `public void setPlannedFinish(LocalDateTime value)`
    pub fn set_planned_finish(&mut self, value: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::PlannedFinish, value.map(FieldValue::DateTime));
    }

/// Get the "cost per unit" flag (default `true`).
    ///
    /// Corresponds to Java: `public boolean getCalculateCostsFromUnits()`
    pub fn get_calculate_costs_from_units(&self) -> bool {
        self.boolean_value(AssignmentField::CalculateCostsFromUnits)
    }

/// Set the "cost per unit" flag.
    ///
    /// Corresponds to Java: `public void setCalculateCostsFromUnits(boolean calculateCostsFromUnits)`
    pub fn set_calculate_costs_from_units(&mut self, calculate_costs_from_units: bool) {
        self.set_boolean_value(
            AssignmentField::CalculateCostsFromUnits,
            calculate_costs_from_units,
        );
    }

/// Get the cost account unique ID.
    ///
    /// Corresponds to Java: `public Integer getCostAccountUniqueID()`
    pub fn get_cost_account_unique_id(&self) -> Option<i32> {
        self.integer_value(AssignmentField::CostAccountUniqueId)
    }

/// Set the cost account unique ID.
    ///
    /// Corresponds to Java: `public void setCostAccountUniqueID(Integer id)`
    pub fn set_cost_account_unique_id(&mut self, id: Option<i32>) {
        self.set_assignment_value(
            AssignmentField::CostAccountUniqueId,
            id.map(FieldValue::Integer),
        );
    }

/// Get remaining late finish.
    ///
    /// Corresponds to Java: `public LocalDateTime getRemainingLateFinish()`
    pub fn get_remaining_late_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::RemainingLateFinish)
    }

/// Set remaining late finish.
    ///
    /// Corresponds to Java: `public void setRemainingLateFinish(LocalDateTime date)`
    pub fn set_remaining_late_finish(&mut self, date: Option<NaiveDateTime>) {
        self.set_assignment_value(
            AssignmentField::RemainingLateFinish,
            date.map(FieldValue::DateTime),
        );
    }

/// Get remaining late start.
    ///
    /// Corresponds to Java: `public LocalDateTime getRemainingLateStart()`
    pub fn get_remaining_late_start(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::RemainingLateStart)
    }

/// Set remaining late start.
    ///
    /// Corresponds to Java: `public void setRemainingLateStart(LocalDateTime date)`
    pub fn set_remaining_late_start(&mut self, date: Option<NaiveDateTime>) {
        self.set_assignment_value(
            AssignmentField::RemainingLateStart,
            date.map(FieldValue::DateTime),
        );
    }

/// Get remaining early finish.
    ///
    /// Corresponds to Java: `public LocalDateTime getRemainingEarlyFinish()`
    pub fn get_remaining_early_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::RemainingEarlyFinish)
    }

/// Set remaining early finish.
    ///
    /// Corresponds to Java: `public void setRemainingEarlyFinish(LocalDateTime date)`
    pub fn set_remaining_early_finish(&mut self, date: Option<NaiveDateTime>) {
        self.set_assignment_value(
            AssignmentField::RemainingEarlyFinish,
            date.map(FieldValue::DateTime),
        );
    }

/// Get remaining early start.
    ///
    /// Corresponds to Java: `public LocalDateTime getRemainingEarlyStart()`
    pub fn get_remaining_early_start(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::RemainingEarlyStart)
    }

/// Set remaining early start.
    ///
    /// Corresponds to Java: `public void setRemainingEarlyStart(LocalDateTime date)`
    pub fn set_remaining_early_start(&mut self, date: Option<NaiveDateTime>) {
        self.set_assignment_value(
            AssignmentField::RemainingEarlyStart,
            date.map(FieldValue::DateTime),
        );
    }

/// Get the cost rate in effect on the given date.
    ///
    /// Corresponds to Java: `public Rate getEffectiveRate(LocalDateTime date)`
    pub fn get_effective_rate(&self, date: NaiveDateTime) -> Option<Rate> {
        if self.get_rate_source() == RateSource::Override {
            return self.get_override_rate();
        }

        let index = usize::try_from(self.get_rate_index().unwrap_or(0)).unwrap_or(0);
        self.with_cost_rate_table(|table| {
            let table = table?;
            let entry = table.get_entry_by_date(date)?;
            entry.get_rate(index)
        })
    }

/// Get actual regular cost (calculated field).
    ///
    /// Corresponds to Java: `public Number getActualRegularCost()`
    pub fn get_actual_regular_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::ActualRegularCost)
    }

/// Get remaining regular cost (calculated field).
    ///
    /// Corresponds to Java: `public Number getRemainingRegularCost()`
    pub fn get_remaining_regular_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::RemainingRegularCost)
    }

/// Get actual regular work (calculated field).
    ///
    /// Corresponds to Java: `public Duration getActualRegularWork()`
    pub fn get_actual_regular_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::ActualRegularWork)
    }

/// Get remaining regular work (calculated field).
    ///
    /// Corresponds to Java: `public Duration getRemainingRegularWork()`
    pub fn get_remaining_regular_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::RemainingRegularWork)
    }

/// Get planned material quantity (calculated field, only populated for material resources).
    ///
    /// Corresponds to Java: `public Number getPlannedMaterial()`
    pub fn get_planned_material(&self) -> Option<f64> {
        self.number_value(AssignmentField::PlannedMaterial)
    }

/// Get actual material quantity (calculated field).
    ///
    /// Corresponds to Java: `public Number getActualMaterial()`
    pub fn get_actual_material(&self) -> Option<f64> {
        self.number_value(AssignmentField::ActualMaterial)
    }

/// Get remaining material quantity (calculated field).
    ///
    /// Corresponds to Java: `public Number getRemainingMaterial()`
    pub fn get_remaining_material(&self) -> Option<f64> {
        self.number_value(AssignmentField::RemainingMaterial)
    }

/// Get material quantity (calculated field).
    ///
    /// Corresponds to Java: `public Number getMaterial()`
    pub fn get_material(&self) -> Option<f64> {
        self.number_value(AssignmentField::Material)
    }

/// Get baseline material quantity (calculated field).
    ///
    /// Corresponds to Java: `public Number getBaselineMaterial()`
    pub fn get_baseline_material(&self) -> Option<f64> {
        self.number_value(AssignmentField::BaselineMaterial)
    }

/// Get baseline material quantity (index 1-10, calculated field).
    ///
    /// Corresponds to Java: `public Number getBaselineMaterial(int baselineNumber)`
    pub fn get_baseline_material_by_index(&self, baseline_number: i32) -> Option<f64> {
        let field = Self::select_field(&BASELINE_MATERIALS, baseline_number)?;
        self.number_value(field)
    }

/// Get the field value by alias.
    ///
    /// Corresponds to Java: `public Object getFieldByAlias(String alias)`
    pub fn get_field_by_alias(&self, alias: &str) -> Option<FieldValue> {
        let field = self.field_type_by_alias(alias)?;
        self.get_field(field)
    }

/// Set the field value by alias.
    ///
    /// Corresponds to Java: `public void setFieldByAlias(String alias, Object value)`
    pub fn set_field_by_alias(&mut self, alias: &str, value: Option<FieldValue>) {
        if let Some(field) = self.field_type_by_alias(alias) {
            self.set_key(field, value);
        }
    }

/// Resolve the field identifier by alias.
    ///
    /// Corresponds to Java: `m_parentFile.getResourceAssignments().getFieldTypeByAlias(alias)`
    ///
/// Resolved on the Rust side via the context's custom field container (see section 4 of the file header).
    fn field_type_by_alias(&self, alias: &str) -> Option<FieldKey> {
        let context = self.context.upgrade()?;
        let custom_fields = {
            let context = context.try_borrow().ok()?;
            context.get_custom_fields()
        };
        custom_fields
            .try_borrow()
            .ok()?
            .get_field_type_by_alias(FieldTypeClass::Assignment, alias)
    }

/// Calculate the overtime cost.
    ///
    /// Corresponds to Java: `private Number calculateOvertimeCost()`
    fn calculate_overtime_cost(&self) -> Option<f64> {
        match (
            self.get_actual_overtime_cost(),
            self.get_remaining_overtime_cost(),
        ) {
            (Some(actual), Some(remaining)) => Some(actual + remaining),
            _ => None,
        }
    }

/// Calculate schedule variance (SV = BCWP - BCWS).
    ///
    /// Corresponds to Java: `private Double calculateSV()`
    fn calculate_sv(&self) -> Option<f64> {
        match (self.get_bcwp(), self.get_bcws()) {
            (Some(bcwp), Some(bcws)) => Some(bcwp - bcws),
            _ => None,
        }
    }

/// Calculate cost variance (CV = BCWP - ACWP).
    ///
    /// Corresponds to Java: `private Double calculateCV()`
    fn calculate_cv(&self) -> Option<f64> {
        Some(NumberHelper::get_double(self.get_bcwp()) - NumberHelper::get_double(self.get_acwp()))
    }

/// Calculate cost variance (cost - baseline cost).
    ///
    /// Corresponds to Java: `private Double calculateCostVariance()`
    fn calculate_cost_variance(&self) -> Option<f64> {
        match (self.get_cost(), self.get_baseline_cost()) {
            (Some(cost), Some(baseline_cost)) => Some(cost - baseline_cost),
            _ => None,
        }
    }

/// Calculate the percent work complete.
    ///
    /// Corresponds to Java: `private Double calculatePercentWorkComplete()`
    fn calculate_percent_work_complete(&self) -> Option<f64> {
        match (self.get_actual_work(), self.get_work()) {
            (Some(actual_work), Some(work)) if work.get_duration() != 0.0 => {
                let converted = self.convert_units_with_defaults(work, actual_work.get_units());
                Some((actual_work.get_duration() * 100.0) / converted.get_duration())
            }
            _ => None,
        }
    }

/// Calculate work variance.
    ///
    /// Corresponds to Java: `private Duration calculateWorkVariance()`
    ///
/// Java writes back via `set(AssignmentField.WORK_VARIANCE, variance)` at the end of the calculation; Rust's `&self`
/// cannot write back (see section 5 of the file header).
    fn calculate_work_variance(&self) -> Option<Duration> {
        match (self.get_work(), self.get_baseline_work()) {
            (Some(work), Some(baseline_work)) => {
                let baseline = self.convert_units_with_defaults(baseline_work, work.get_units());
                Some(Duration::get_instance(
                    work.get_duration() - baseline.get_duration(),
                    work.get_units(),
                ))
            }
            _ => None,
        }
    }

/// Calculate start variance.
    ///
    /// Corresponds to Java: `private Duration calculateStartVariance()`
    fn calculate_start_variance(&self) -> Option<Duration> {
        let calendar = self.get_effective_calendar();
        let calendar_ref = calendar
            .as_ref()
            .and_then(|calendar| calendar.try_borrow().ok());
        Some(LocalDateTimeHelper::get_variance(
            calendar_ref.as_deref(),
            self.get_baseline_start(),
            self.get_start(),
            self.default_duration_units(),
        ))
    }

/// Calculate finish variance.
    ///
    /// Corresponds to Java: `private Duration calculateFinishVariance()`
    fn calculate_finish_variance(&self) -> Option<Duration> {
        let calendar = self.get_effective_calendar();
        let calendar_ref = calendar
            .as_ref()
            .and_then(|calendar| calendar.try_borrow().ok());
        Some(LocalDateTimeHelper::get_variance(
            calendar_ref.as_deref(),
            self.get_baseline_finish(),
            self.get_finish(),
            self.default_duration_units(),
        ))
    }

/// Convert the duration using the default values supplied by the project properties.
    ///
    /// Corresponds to Java: `duration.convertUnits(units, m_parentFile.getProjectProperties())`
    ///
/// Falls back to the effective calendar when the parent file is not wired up (or is currently borrowed) (see section 4 of the file header).
    fn convert_units_with_defaults(&self, duration: Duration, units: TimeUnit) -> Duration {
        if let Some(parent_file) = self.parent_file.upgrade() {
            if let Ok(parent_file) = parent_file.try_borrow() {
                return duration.convert_units(units, parent_file.get_project_properties());
            }
        }

        convert_units(duration, units, self.get_effective_calendar().as_ref())
    }

/// Get the project default duration unit.
    ///
    /// Corresponds to Java: `m_parentFile.getProjectProperties().getDefaultDurationUnits()`
    fn default_duration_units(&self) -> TimeUnit {
        if let Some(parent_file) = self.parent_file.upgrade() {
            if let Ok(parent_file) = parent_file.try_borrow() {
                return parent_file.get_project_properties().get_default_duration_units();
            }
        }

        DEFAULT_DURATION_UNITS
    }

/// Calculate the start time (falls back to the task start when not set explicitly).
    ///
    /// Corresponds to Java: `private LocalDateTime calculateStart()`
    fn calculate_start(&self) -> Option<NaiveDateTime> {
        if let Some(start) = self
            .peek_cached_value(FieldKey::Assignment(AssignmentField::Start))
            .and_then(|value| value.as_date_time())
        {
            return Some(start);
        }

        self.get_task()
            .and_then(|task| task.try_borrow().ok().and_then(|task| task.get_start()))
    }

/// Calculate the finish time (falls back to the task finish when not set explicitly).
    ///
    /// Corresponds to Java: `private LocalDateTime calculateFinish()`
    fn calculate_finish(&self) -> Option<NaiveDateTime> {
        if let Some(finish) = self
            .peek_cached_value(FieldKey::Assignment(AssignmentField::Finish))
            .and_then(|value| value.as_date_time())
        {
            return Some(finish);
        }

        self.get_task()
            .and_then(|task| task.try_borrow().ok().and_then(|task| task.get_finish()))
    }

/// Calculate remaining assignment units (falls back to planned units when there is no remaining units value).
    ///
    /// Corresponds to Java: `private Number calculateRemainingAssignmentUnits()`
    fn calculate_remaining_assignment_units(&self) -> Option<f64> {
        self.get_units()
    }

/// Calculate timephased overtime work (proportionally scaled by remaining regular work).
    ///
    /// Corresponds to Java: `private List<TimephasedWork> calculateTimephasedOvertimeWork()`
    fn calculate_timephased_overtime_work(&self) -> Vec<TimephasedWork> {
        let (Some(total_remaining_work), Some(remaining_overtime_work)) =
            (self.get_remaining_work(), self.get_remaining_overtime_work())
        else {
            return Vec::new();
        };

        let calendar = self.get_effective_calendar();
        let total_remaining_minutes =
            convert_units(total_remaining_work, TimeUnit::Minutes, calendar.as_ref()).get_duration();
        let remaining_overtime_minutes =
            convert_units(remaining_overtime_work, TimeUnit::Minutes, calendar.as_ref())
                .get_duration();
        let remaining_regular_minutes = total_remaining_minutes - remaining_overtime_minutes;

        if remaining_regular_minutes == 0.0 || remaining_overtime_minutes == 0.0 {
            return Vec::new();
        }

        let factor = remaining_overtime_minutes / remaining_regular_minutes;
        self.get_raw_timephased_remaining_regular_work()
            .iter()
            .map(|item| TimephasedWork::from_work(item, factor))
            .collect()
    }

/// Calculate "actual regular cost".
    ///
/// Corresponds to Java: entry in `CALCULATED_FIELD_MAP`
    /// `a -> a.calculateRegularCost(a::getActualCost, a::getActualOvertimeCost)`
    fn calculate_actual_regular_cost(&self) -> Option<f64> {
        self.calculate_regular_cost(self.get_actual_cost(), self.get_actual_overtime_cost())
    }

/// Calculate "remaining regular cost".
    ///
/// Corresponds to Java: entry in `CALCULATED_FIELD_MAP`
    /// `a -> a.calculateRegularCost(a::getRemainingCost, a::getRemainingOvertimeCost)`
    fn calculate_remaining_regular_cost(&self) -> Option<f64> {
        self.calculate_regular_cost(self.get_remaining_cost(), self.get_remaining_overtime_cost())
    }

/// Calculate "actual regular work".
    ///
/// Corresponds to Java: entry in `CALCULATED_FIELD_MAP`
    /// `a -> a.calculateRegularWork(a::getActualWork, a::getActualOvertimeWork)`
    fn calculate_actual_regular_work(&self) -> Option<Duration> {
        self.calculate_regular_work(self.get_actual_work(), self.get_actual_overtime_work())
    }

/// Calculate "remaining regular work".
    ///
/// Corresponds to Java: entry in `CALCULATED_FIELD_MAP`
    /// `a -> a.calculateRegularWork(a::getRemainingWork, a::getRemainingOvertimeWork)`
    fn calculate_remaining_regular_work(&self) -> Option<Duration> {
        self.calculate_regular_work(self.get_remaining_work(), self.get_remaining_overtime_work())
    }

/// Calculate regular cost from total cost and overtime cost.
    ///
    /// Corresponds to Java: `private Number calculateRegularCost(Supplier<Number>, Supplier<Number>)`
    fn calculate_regular_cost(&self, total_cost: Option<f64>, overtime_cost: Option<f64>) -> Option<f64> {
        if total_cost.is_none() && overtime_cost.is_none() {
            return None;
        }

        Some(NumberHelper::get_double(total_cost) - NumberHelper::get_double(overtime_cost))
    }

/// Calculate regular work from total work and overtime work.
    ///
    /// Corresponds to Java: `private Duration calculateRegularWork(Supplier<Duration>, Supplier<Duration>)`
    fn calculate_regular_work(
        &self,
        total_work: Option<Duration>,
        overtime_work: Option<Duration>,
    ) -> Option<Duration> {
        let units = match (total_work, overtime_work) {
            (Some(total_work), _) => total_work.get_units(),
            (None, Some(overtime_work)) => overtime_work.get_units(),
            (None, None) => return None,
        };

        let calendar = self.get_effective_calendar();
        let total = match total_work {
            Some(total_work) => convert_units(total_work, units, calendar.as_ref()).get_duration(),
            None => 0.0,
        };
        let overtime = match overtime_work {
            Some(overtime_work) => {
                convert_units(overtime_work, units, calendar.as_ref()).get_duration()
            }
            None => 0.0,
        };

        Some(Duration::get_instance(total - overtime, units))
    }

/// Calculate material quantity from work (only populated for material resources).
    ///
    /// Corresponds to Java: `private Number calculateMaterial(Supplier<Duration> fn)`
    fn calculate_material(&self, work: Option<Duration>) -> Option<f64> {
        if !self.is_material_resource() {
            return None;
        }

        work.map(|work| work.get_duration())
    }

/// Default value for the cost rate table index (0).
    ///
    /// Corresponds to Java: `private Integer defaultRateIndex()`
    fn default_rate_index(&self) -> Option<i32> {
        Some(0)
    }

/// Default value for "cost per unit" (`true`).
    ///
    /// Corresponds to Java: `private Boolean defaultCalculateCostsFromUnits()`
    fn default_calculate_costs_from_units(&self) -> Option<bool> {
        Some(true)
    }

/// Whether the resource for this assignment is a material resource.
    fn is_material_resource(&self) -> bool {
        match self.get_resource() {
            Some(resource) => resource
                .try_borrow()
                .ok()
                .map(|resource| resource.get_type() == ResourceType::Material)
                .unwrap_or(false),
            None => false,
        }
    }

/// Get the accrual method of the resource used by this assignment; falls back to `AccrueType::Prorated` when absent.
    ///
    /// Corresponds to Java: `private AccrueType getResourceAccrueType()`
    fn get_resource_accrue_type(&self) -> AccrueType {
        let Some(resource) = self.get_resource() else {
            return AccrueType::Prorated;
        };

        resource
            .try_borrow()
            .ok()
            .and_then(|resource| resource.get_accrue_at())
            .unwrap_or(AccrueType::Prorated)
    }

// ---- timephased raw data accessors ----

/// Get raw timephased planned work.
    ///
    /// Corresponds to Java: `public List<TimephasedWork> getRawTimephasedPlannedWork()`
    ///
/// Returns an empty list when unset, consistent with Java (the `CALCULATED_FIELD_MAP`
    /// `defaultTimephasedWork()`).
    pub fn get_raw_timephased_planned_work(&self) -> Vec<TimephasedWork> {
        self.timephased_work_value(AssignmentField::RawTimephasedPlannedWork)
    }

/// Set the raw timephased planned work (readers use it to write back; semantically equivalent to Java's
    /// `getRawTimephasedPlannedWork().addAll(...)`).
    pub fn set_raw_timephased_planned_work(&mut self, work: Vec<TimephasedWork>) {
        self.set_timephased_work_value(AssignmentField::RawTimephasedPlannedWork, work);
    }

/// Get the raw timephased actual regular work.
    ///
    /// Corresponds to Java: `public List<TimephasedWork> getRawTimephasedActualRegularWork()`
    pub fn get_raw_timephased_actual_regular_work(&self) -> Vec<TimephasedWork> {
        self.timephased_work_value(AssignmentField::RawTimephasedActualRegularWork)
    }

/// Set the raw timephased actual regular work.
    pub fn set_raw_timephased_actual_regular_work(&mut self, work: Vec<TimephasedWork>) {
        self.set_timephased_work_value(AssignmentField::RawTimephasedActualRegularWork, work);
    }

/// Get the raw timephased remaining regular work.
    ///
    /// Corresponds to Java: `public List<TimephasedWork> getRawTimephasedRemainingRegularWork()`
    pub fn get_raw_timephased_remaining_regular_work(&self) -> Vec<TimephasedWork> {
        self.timephased_work_value(AssignmentField::RawTimephasedRemainingRegularWork)
    }

/// Set the raw timephased remaining regular work.
    pub fn set_raw_timephased_remaining_regular_work(&mut self, work: Vec<TimephasedWork>) {
        self.set_timephased_work_value(AssignmentField::RawTimephasedRemainingRegularWork, work);
    }

/// Get the raw timephased actual overtime work.
    ///
    /// Corresponds to Java: `public List<TimephasedWork> getRawTimephasedActualOvertimeWork()`
    pub fn get_raw_timephased_actual_overtime_work(&self) -> Vec<TimephasedWork> {
        self.timephased_work_value(AssignmentField::RawTimephasedActualOvertimeWork)
    }

/// Set the raw timephased actual overtime work.
    pub fn set_raw_timephased_actual_overtime_work(&mut self, work: Vec<TimephasedWork>) {
        self.set_timephased_work_value(AssignmentField::RawTimephasedActualOvertimeWork, work);
    }

/// Get the raw timephased remaining overtime work; when absent it is provided by the calculated field
/// `calculateTimephasedOvertimeWork`.
    ///
    /// Corresponds to Java: `public List<TimephasedWork> getRawTimephasedRemainingOvertimeWork()`
    pub fn get_raw_timephased_remaining_overtime_work(&self) -> Vec<TimephasedWork> {
        self.timephased_work_value(AssignmentField::RawTimephasedRemainingOvertimeWork)
    }

/// Set the raw timephased remaining overtime work.
    pub fn set_raw_timephased_remaining_overtime_work(&mut self, work: Vec<TimephasedWork>) {
        self.set_timephased_work_value(AssignmentField::RawTimephasedRemainingOvertimeWork, work);
    }

/// Get the raw timephased baseline work (index 0 = Baseline, 1..=10 = Baseline1..10).
    ///
    /// Corresponds to Java: `public List<TimephasedWork> getRawTimephasedBaselineWork(int index)`
    ///
/// Returns an empty list when out of bounds (Java throws `ArrayIndexOutOfBoundsException`).
    pub fn get_raw_timephased_baseline_work(&self, index: i32) -> Vec<TimephasedWork> {
        raw_timephased_baseline_work_field(index)
            .map(|field| self.timephased_work_value(field))
            .unwrap_or_default()
    }

/// Set the raw timephased baseline work.
    pub fn set_raw_timephased_baseline_work(&mut self, index: i32, work: Vec<TimephasedWork>) {
        if let Some(field) = raw_timephased_baseline_work_field(index) {
            self.set_timephased_work_value(field, work);
        }
    }

/// Get the raw timephased baseline budget work (same indices as above).
    ///
    /// Corresponds to Java: `public List<TimephasedWork> getRawTimephasedBaselineBudgetWork(int index)`
    pub fn get_raw_timephased_baseline_budget_work(&self, index: i32) -> Vec<TimephasedWork> {
        raw_timephased_baseline_budget_work_field(index)
            .map(|field| self.timephased_work_value(field))
            .unwrap_or_default()
    }

/// Set the raw timephased baseline budget work.
    pub fn set_raw_timephased_baseline_budget_work(&mut self, index: i32, work: Vec<TimephasedWork>) {
        if let Some(field) = raw_timephased_baseline_budget_work_field(index) {
            self.set_timephased_work_value(field, work);
        }
    }

/// Get the raw timephased baseline cost (same indices as above).
    ///
    /// Corresponds to Java: `public List<TimephasedCost> getRawTimephasedBaselineCost(int index)`
    pub fn get_raw_timephased_baseline_cost(&self, index: i32) -> Vec<TimephasedCost> {
        raw_timephased_baseline_cost_field(index)
            .map(|field| self.timephased_cost_value(field))
            .unwrap_or_default()
    }

/// Set the raw timephased baseline cost.
    pub fn set_raw_timephased_baseline_cost(&mut self, index: i32, cost: Vec<TimephasedCost>) {
        if let Some(field) = raw_timephased_baseline_cost_field(index) {
            self.set_timephased_cost_value(field, cost);
        }
    }

/// Get the raw timephased baseline budget cost (same indices as above).
    ///
    /// Corresponds to Java: `public List<TimephasedCost> getRawTimephasedBaselineBudgetCost(int index)`
    pub fn get_raw_timephased_baseline_budget_cost(&self, index: i32) -> Vec<TimephasedCost> {
        raw_timephased_baseline_budget_cost_field(index)
            .map(|field| self.timephased_cost_value(field))
            .unwrap_or_default()
    }

/// Set the raw timephased baseline budget cost.
    pub fn set_raw_timephased_baseline_budget_cost(
        &mut self,
        index: i32,
        cost: Vec<TimephasedCost>,
    ) {
        if let Some(field) = raw_timephased_baseline_budget_cost_field(index) {
            self.set_timephased_cost_value(field, cost);
        }
    }

/// Get the raw timephased budget cost.
    ///
    /// Corresponds to Java: `public List<TimephasedCost> getRawTimephasedBudgetCost()`
    pub fn get_raw_timephased_budget_cost(&self) -> Vec<TimephasedCost> {
        self.timephased_cost_value(AssignmentField::RawTimephasedBudgetCost)
    }

/// Set the raw timephased budget cost.
    pub fn set_raw_timephased_budget_cost(&mut self, cost: Vec<TimephasedCost>) {
        self.set_timephased_cost_value(AssignmentField::RawTimephasedBudgetCost, cost);
    }

/// Get the raw timephased budget work.
    ///
    /// Corresponds to Java: `public List<TimephasedWork> getRawTimephasedBudgetWork()`
    pub fn get_raw_timephased_budget_work(&self) -> Vec<TimephasedWork> {
        self.timephased_work_value(AssignmentField::RawTimephasedBudgetWork)
    }

/// Set the raw timephased budget work.
    pub fn set_raw_timephased_budget_work(&mut self, work: Vec<TimephasedWork>) {
        self.set_timephased_work_value(AssignmentField::RawTimephasedBudgetWork, work);
    }

/// Get the timephased work list from the field table (empty list when not set).
    fn timephased_work_value(&self, field: AssignmentField) -> Vec<TimephasedWork> {
        self.assignment_value(field)
            .and_then(|value| value.as_timephased_work().map(<[TimephasedWork]>::to_vec))
            .unwrap_or_default()
    }

/// Write the timephased work list to the field table.
    fn set_timephased_work_value(&mut self, field: AssignmentField, work: Vec<TimephasedWork>) {
        self.set_assignment_value(field, Some(FieldValue::TimephasedWork(work)));
    }

/// Get the timephased cost list from the field table (empty list when not set).
    fn timephased_cost_value(&self, field: AssignmentField) -> Vec<TimephasedCost> {
        self.assignment_value(field)
            .and_then(|value| value.as_timephased_cost().map(<[TimephasedCost]>::to_vec))
            .unwrap_or_default()
    }

/// Write the timephased cost list to the field table.
    fn set_timephased_cost_value(&mut self, field: AssignmentField, cost: Vec<TimephasedCost>) {
        self.set_assignment_value(field, Some(FieldValue::TimephasedCost(cost)));
    }

/// Get the working time segments within this assignment's range.
    ///
    /// Corresponds to Java: `public List<LocalDateTimeRange> getWorkSplits()`
    pub fn get_work_splits(&self) -> Vec<LocalDateTimeRange> {
        let actual_work = self.get_raw_timephased_actual_regular_work();
        let remaining_work = self.get_raw_timephased_remaining_regular_work();
        if actual_work.is_empty() && remaining_work.is_empty() {
            return Vec::new();
        }

        let calendar = self.get_effective_calendar();
        let mut result: Vec<LocalDateTimeRange> = Vec::new();
        let mut last_range: Option<LocalDateTimeRange> = None;

// Java applies the same merge logic to both batches of data
        for work in actual_work.iter().chain(remaining_work.iter()) {
// Java: blocks representing non-working time are skipped directly and break the merge
            let total_amount = work
                .get_total_amount()
                .map(|amount| amount.get_duration())
                .unwrap_or(0.0);
            if total_amount == 0.0 {
                last_range = None;
                continue;
            }

            if let Some(previous) = last_range {
                let contiguous = match (previous.get_end(), work.get_start()) {
                    (Some(end), Some(start)) => {
                        end == start
                            || calendar_work(
                                calendar.as_ref(),
                                previous.get_end(),
                                work.get_start(),
                                TimeUnit::Hours,
                            )
                            .get_duration()
                                == 0.0
                    }
                    _ => false,
                };

                if contiguous {
                    result.pop();
                    let merged = LocalDateTimeRange::new(previous.get_start(), work.get_finish());
                    result.push(merged);
                    last_range = Some(merged);
                    continue;
                }
            }

            let range = LocalDateTimeRange::new(work.get_start(), work.get_finish());
            result.push(range);
            last_range = Some(range);
        }

        result
    }

/// Get the timephased duration value for the specified field.
    ///
    /// Corresponds to Java: `@Override public List<Duration> getTimephasedDurationValues(FieldType field, List<LocalDateTimeRange> ranges, TimeUnit units)`
    ///
/// Rust's [`FieldContainer`] trait does not declare these two timephased methods (see
/// the note in `field_container.rs`), so they are provided as ordinary methods.
    pub fn get_timephased_duration_values(
        &self,
        field: FieldKey,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        let field = match field {
            FieldKey::Assignment(field) => field,
            _ => return empty_durations(ranges.len()),
        };

        use AssignmentField as F;
        match field {
            F::PlannedWork => self.get_timephased_planned_work(ranges, units),
            F::ActualRegularWork => self.get_timephased_actual_regular_work(ranges, units),
            F::ActualOvertimeWork => self.get_timephased_actual_overtime_work(ranges, units),
            F::ActualWork => self.get_timephased_actual_work(ranges, units),
            F::RemainingRegularWork => self.get_timephased_remaining_regular_work(ranges, units),
            F::RemainingOvertimeWork => {
                self.get_timephased_remaining_overtime_work(ranges, units)
            }
            F::RemainingWork => self.get_timephased_remaining_work(ranges, units),
            F::Work => self.get_timephased_work(ranges, units),
            F::BudgetWork => self.get_timephased_budget_work(ranges, units),
            F::BaselineWork => self.get_timephased_baseline_work(0, ranges, units),
            F::Baseline1Work => self.get_timephased_baseline_work(1, ranges, units),
            F::Baseline2Work => self.get_timephased_baseline_work(2, ranges, units),
            F::Baseline3Work => self.get_timephased_baseline_work(3, ranges, units),
            F::Baseline4Work => self.get_timephased_baseline_work(4, ranges, units),
            F::Baseline5Work => self.get_timephased_baseline_work(5, ranges, units),
            F::Baseline6Work => self.get_timephased_baseline_work(6, ranges, units),
            F::Baseline7Work => self.get_timephased_baseline_work(7, ranges, units),
            F::Baseline8Work => self.get_timephased_baseline_work(8, ranges, units),
            F::Baseline9Work => self.get_timephased_baseline_work(9, ranges, units),
            F::Baseline10Work => self.get_timephased_baseline_work(10, ranges, units),
            F::BaselineBudgetWork => self.get_timephased_baseline_budget_work(0, ranges, units),
            F::Baseline1BudgetWork => self.get_timephased_baseline_budget_work(1, ranges, units),
            F::Baseline2BudgetWork => self.get_timephased_baseline_budget_work(2, ranges, units),
            F::Baseline3BudgetWork => self.get_timephased_baseline_budget_work(3, ranges, units),
            F::Baseline4BudgetWork => self.get_timephased_baseline_budget_work(4, ranges, units),
            F::Baseline5BudgetWork => self.get_timephased_baseline_budget_work(5, ranges, units),
            F::Baseline6BudgetWork => self.get_timephased_baseline_budget_work(6, ranges, units),
            F::Baseline7BudgetWork => self.get_timephased_baseline_budget_work(7, ranges, units),
            F::Baseline8BudgetWork => self.get_timephased_baseline_budget_work(8, ranges, units),
            F::Baseline9BudgetWork => self.get_timephased_baseline_budget_work(9, ranges, units),
            F::Baseline10BudgetWork => self.get_timephased_baseline_budget_work(10, ranges, units),
            _ => empty_durations(ranges.len()),
        }
    }

/// Get the timephased numeric value for the specified field.
    ///
    /// Corresponds to Java: `@Override public List<Number> getTimephasedNumericValues(FieldType field, List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_numeric_values(
        &self,
        field: FieldKey,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        let field = match field {
            FieldKey::Assignment(field) => field,
            _ => return empty_numbers(ranges.len()),
        };

        use AssignmentField as F;
        match field {
            F::PlannedCost => self.get_timephased_planned_cost(ranges),
            F::RemainingRegularCost => self.get_timephased_remaining_regular_cost(ranges),
            F::RemainingOvertimeCost => self.get_timephased_remaining_overtime_cost(ranges),
            F::RemainingCost => self.get_timephased_remaining_cost(ranges),
            F::ActualRegularCost => self.get_timephased_actual_regular_cost(ranges),
            F::ActualOvertimeCost => self.get_timephased_actual_overtime_cost(ranges),
            F::ActualCost => self.get_timephased_actual_cost(ranges),
            F::Cost => self.get_timephased_cost_total(ranges),
            F::PlannedMaterial => self.get_timephased_planned_material(ranges),
            F::ActualMaterial => self.get_timephased_actual_material(ranges),
            F::RemainingMaterial => self.get_timephased_remaining_material(ranges),
            F::Material => self.get_timephased_material(ranges),
            F::BudgetCost => self.get_timephased_budget_cost(ranges),
            F::BaselineCost => self.get_timephased_baseline_cost(0, ranges),
            F::Baseline1Cost => self.get_timephased_baseline_cost(1, ranges),
            F::Baseline2Cost => self.get_timephased_baseline_cost(2, ranges),
            F::Baseline3Cost => self.get_timephased_baseline_cost(3, ranges),
            F::Baseline4Cost => self.get_timephased_baseline_cost(4, ranges),
            F::Baseline5Cost => self.get_timephased_baseline_cost(5, ranges),
            F::Baseline6Cost => self.get_timephased_baseline_cost(6, ranges),
            F::Baseline7Cost => self.get_timephased_baseline_cost(7, ranges),
            F::Baseline8Cost => self.get_timephased_baseline_cost(8, ranges),
            F::Baseline9Cost => self.get_timephased_baseline_cost(9, ranges),
            F::Baseline10Cost => self.get_timephased_baseline_cost(10, ranges),
            F::BaselineMaterial => self.get_timephased_baseline_material(0, ranges),
            F::Baseline1Material => self.get_timephased_baseline_material(1, ranges),
            F::Baseline2Material => self.get_timephased_baseline_material(2, ranges),
            F::Baseline3Material => self.get_timephased_baseline_material(3, ranges),
            F::Baseline4Material => self.get_timephased_baseline_material(4, ranges),
            F::Baseline5Material => self.get_timephased_baseline_material(5, ranges),
            F::Baseline6Material => self.get_timephased_baseline_material(6, ranges),
            F::Baseline7Material => self.get_timephased_baseline_material(7, ranges),
            F::Baseline8Material => self.get_timephased_baseline_material(8, ranges),
            F::Baseline9Material => self.get_timephased_baseline_material(9, ranges),
            F::Baseline10Material => self.get_timephased_baseline_material(10, ranges),
            F::BaselineBudgetCost => self.get_timephased_baseline_budget_cost(0, ranges),
            F::Baseline1BudgetCost => self.get_timephased_baseline_budget_cost(1, ranges),
            F::Baseline2BudgetCost => self.get_timephased_baseline_budget_cost(2, ranges),
            F::Baseline3BudgetCost => self.get_timephased_baseline_budget_cost(3, ranges),
            F::Baseline4BudgetCost => self.get_timephased_baseline_budget_cost(4, ranges),
            F::Baseline5BudgetCost => self.get_timephased_baseline_budget_cost(5, ranges),
            F::Baseline6BudgetCost => self.get_timephased_baseline_budget_cost(6, ranges),
            F::Baseline7BudgetCost => self.get_timephased_baseline_budget_cost(7, ranges),
            F::Baseline8BudgetCost => self.get_timephased_baseline_budget_cost(8, ranges),
            F::Baseline9BudgetCost => self.get_timephased_baseline_budget_cost(9, ranges),
            F::Baseline10BudgetCost => self.get_timephased_baseline_budget_cost(10, ranges),
            _ => empty_numbers(ranges.len()),
        }
    }

/// Get the timephased planned work.
    ///
    /// Corresponds to Java: `public List<Duration> getTimephasedPlannedWork(List<LocalDateTimeRange> ranges, TimeUnit units)`
    pub fn get_timephased_planned_work(
        &self,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        let work = self.get_or_create_raw_timephased_planned_work();
        segment_work(self.get_effective_calendar().as_ref(), &work, ranges, units)
    }

/// Get the timephased actual regular work.
    ///
    /// Corresponds to Java: `public List<Duration> getTimephasedActualRegularWork(List<LocalDateTimeRange> ranges, TimeUnit units)`
    pub fn get_timephased_actual_regular_work(
        &self,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        let work = self.get_or_create_raw_timephased_actual_work(
            self.get_raw_timephased_actual_regular_work().to_vec(),
            self.get_actual_regular_work(),
        );
        segment_work(self.get_effective_calendar().as_ref(), &work, ranges, units)
    }

/// Get the timephased actual overtime work.
    ///
    /// Corresponds to Java: `public List<Duration> getTimephasedActualOvertimeWork(List<LocalDateTimeRange> ranges, TimeUnit units)`
    pub fn get_timephased_actual_overtime_work(
        &self,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        let work = self.get_or_create_raw_timephased_actual_work(
            self.get_raw_timephased_actual_overtime_work().to_vec(),
            self.get_actual_overtime_work(),
        );
        segment_work(self.get_effective_calendar().as_ref(), &work, ranges, units)
    }

/// Get the timephased actual work (regular + overtime).
    ///
    /// Corresponds to Java: `public List<Duration> getTimephasedActualWork(List<LocalDateTimeRange> ranges, TimeUnit units)`
    pub fn get_timephased_actual_work(
        &self,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        let regular = self.get_timephased_actual_regular_work(ranges, units);
        let overtime = self.get_timephased_actual_overtime_work(ranges, units);
        add_timephased_durations(regular, overtime)
    }

/// Get the timephased remaining regular work.
    ///
    /// Corresponds to Java: `public List<Duration> getTimephasedRemainingRegularWork(List<LocalDateTimeRange> ranges, TimeUnit units)`
    pub fn get_timephased_remaining_regular_work(
        &self,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        let work = self.get_or_create_raw_timephased_remaining_work(
            self.get_raw_timephased_remaining_regular_work().to_vec(),
            self.get_remaining_regular_work(),
        );
        segment_work(self.get_effective_calendar().as_ref(), &work, ranges, units)
    }

/// Get the timephased remaining overtime work.
    ///
    /// Corresponds to Java: `public List<Duration> getTimephasedRemainingOvertimeWork(List<LocalDateTimeRange> ranges, TimeUnit units)`
    pub fn get_timephased_remaining_overtime_work(
        &self,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        let work = self.get_or_create_raw_timephased_remaining_work(
            self.get_raw_timephased_remaining_overtime_work(),
            self.get_remaining_overtime_work(),
        );
        segment_work(self.get_effective_calendar().as_ref(), &work, ranges, units)
    }

/// Get the timephased remaining work (regular + overtime).
    ///
    /// Corresponds to Java: `public List<Duration> getTimephasedRemainingWork(List<LocalDateTimeRange> ranges, TimeUnit units)`
    pub fn get_timephased_remaining_work(
        &self,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        let regular = self.get_timephased_remaining_regular_work(ranges, units);
        let overtime = self.get_timephased_remaining_overtime_work(ranges, units);
        add_timephased_durations(regular, overtime)
    }

/// Get the timephased work (actual + remaining).
    ///
    /// Corresponds to Java: `public List<Duration> getTimephasedWork(List<LocalDateTimeRange> ranges, TimeUnit units)`
    pub fn get_timephased_work(
        &self,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        let actual = self.get_timephased_actual_work(ranges, units);
        let remaining = self.get_timephased_remaining_work(ranges, units);
        add_timephased_durations(actual, remaining)
    }

/// Get the timephased baseline work (index 0 = Baseline, 1..=10 = Baseline1..10).
    ///
    /// Corresponds to Java: `public List<Duration> getTimephasedBaselineWork(int index, List<LocalDateTimeRange> ranges, TimeUnit units)`
    pub fn get_timephased_baseline_work(
        &self,
        index: i32,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        let calendar = self.get_baseline_calendar();
        let work = self.get_raw_timephased_baseline_work(index).to_vec();
        segment_work(calendar.as_ref(), &work, ranges, units)
    }

/// Get the timephased baseline budget work.
    ///
    /// Corresponds to Java: `public List<Duration> getTimephasedBaselineBudgetWork(int index, List<LocalDateTimeRange> ranges, TimeUnit units)`
    pub fn get_timephased_baseline_budget_work(
        &self,
        index: i32,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        let calendar = self.get_baseline_calendar();
        let work = self.get_raw_timephased_baseline_budget_work(index).to_vec();
        segment_work(calendar.as_ref(), &work, ranges, units)
    }

/// Get the timephased budget work.
    ///
    /// Corresponds to Java: `public List<Duration> getTimephasedBudgetWork(List<LocalDateTimeRange> ranges, TimeUnit units)`
    pub fn get_timephased_budget_work(
        &self,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        let work = self.get_raw_timephased_budget_work().to_vec();
        segment_work(self.get_effective_calendar().as_ref(), &work, ranges, units)
    }

/// Get the parent project's base calendar.
    ///
    /// Corresponds to Java: `m_parentFile.getBaselineCalendar()`
    fn get_baseline_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        self.parent_file
            .upgrade()
            .and_then(|parent_file| parent_file.try_borrow().ok()?.get_baseline_calendar())
    }

/// Get the timephased planned material quantity.
    ///
    /// Corresponds to Java: `public List<Number> getTimephasedPlannedMaterial(List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_planned_material(&self, ranges: &[LocalDateTimeRange]) -> Vec<Option<f64>> {
        let calendar = self.get_effective_calendar();
        let work = self.get_or_create_raw_timephased_planned_work();
        segment_material(self.is_material_resource(), calendar.as_ref(), &work, ranges)
    }

/// Get the timephased actual material quantity.
    ///
    /// Corresponds to Java: `public List<Number> getTimephasedActualMaterial(List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_actual_material(&self, ranges: &[LocalDateTimeRange]) -> Vec<Option<f64>> {
        let calendar = self.get_effective_calendar();
        let work = self.get_raw_timephased_actual_regular_work().to_vec();
        segment_material(self.is_material_resource(), calendar.as_ref(), &work, ranges)
    }

/// Get the timephased remaining material quantity.
    ///
    /// Corresponds to Java: `public List<Number> getTimephasedRemainingMaterial(List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_remaining_material(
        &self,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        let calendar = self.get_effective_calendar();
        let work = self.get_raw_timephased_remaining_regular_work().to_vec();
        segment_material(self.is_material_resource(), calendar.as_ref(), &work, ranges)
    }

/// Get the timephased material quantity (actual + remaining).
    ///
    /// Corresponds to Java: `public List<Number> getTimephasedMaterial(List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_material(&self, ranges: &[LocalDateTimeRange]) -> Vec<Option<f64>> {
        let actual = self.get_timephased_actual_material(ranges);
        let remaining = self.get_timephased_remaining_material(ranges);
        add_timephased_numbers(actual, remaining)
    }

/// Get the timephased baseline material quantity.
    ///
    /// Corresponds to Java: `public List<Number> getTimephasedBaselineMaterial(int index, List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_baseline_material(
        &self,
        index: i32,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        let calendar = self.get_baseline_calendar();
        let work = self.get_raw_timephased_baseline_work(index).to_vec();
        segment_material(self.is_material_resource(), calendar.as_ref(), &work, ranges)
    }

/// Get the timephased budget cost.
    ///
    /// Corresponds to Java: `public List<Number> getTimephasedBudgetCost(List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_budget_cost(&self, ranges: &[LocalDateTimeRange]) -> Vec<Option<f64>> {
        let calendar = self.get_effective_calendar();
        let cost = self.get_raw_timephased_budget_cost().to_vec();
        segment_cost(calendar.as_ref(), &cost, ranges)
    }

// ---- timephased cost ----

/// Get the timephased remaining regular cost.
    ///
    /// Corresponds to Java: `public List<Number> getTimephasedRemainingRegularCost(List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_remaining_regular_cost(
        &self,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        if ranges.is_empty() {
            return Vec::new();
        }

        let assignment_range = LocalDateTimeRange::new(self.get_start(), self.get_finish());
        if out_of_assignment_range(ranges, &assignment_range) {
            return empty_numbers(ranges.len());
        }

        let resource_type = self.resource_type();
        if resource_type == ResourceType::Cost {
            return self.get_timephased_cost_resource_remaining_cost(ranges);
        }

        let total_cost = Some(
            NumberHelper::get_double(self.get_cost())
                - NumberHelper::get_double(self.get_overtime_cost()),
        );
        let remaining_cost = Some(
            NumberHelper::get_double(self.get_remaining_cost())
                - NumberHelper::get_double(self.get_remaining_overtime_cost()),
        );

        match self.get_resource_accrue_type() {
            AccrueType::Start => {
                self.get_timephased_remaining_cost_accrued_at_start(ranges, total_cost, remaining_cost)
            }
            AccrueType::End => {
                self.get_timephased_remaining_cost_accrued_at_end(ranges, total_cost, remaining_cost)
            }
            _ => {
                let costs = self.get_timephased_cost(ranges, 0, &|ranges, units| {
                    self.get_timephased_remaining_regular_work(ranges, units)
                });
                self.add_timephased_remaining_cost_per_use(ranges, costs)
            }
        }
    }

/// Get the timephased remaining overtime cost.
    ///
    /// Corresponds to Java: `public List<Number> getTimephasedRemainingOvertimeCost(List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_remaining_overtime_cost(
        &self,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        if ranges.is_empty() {
            return Vec::new();
        }

        let assignment_range = LocalDateTimeRange::new(self.get_start(), self.get_finish());
        if out_of_assignment_range(ranges, &assignment_range) {
            return empty_numbers(ranges.len());
        }

        if self.resource_type() == ResourceType::Cost {
// Java: cost resources have no overtime
            return empty_numbers(ranges.len());
        }

        match self.get_resource_accrue_type() {
            AccrueType::Start => self.get_timephased_remaining_cost_accrued_at_start(
                ranges,
                self.get_overtime_cost(),
                self.get_remaining_overtime_cost(),
            ),
            AccrueType::End => self.get_timephased_remaining_cost_accrued_at_end(
                ranges,
                self.get_overtime_cost(),
                self.get_remaining_overtime_cost(),
            ),
            _ => self.get_timephased_cost(ranges, 1, &|ranges, units| {
                self.get_timephased_remaining_overtime_work(ranges, units)
            }),
        }
    }

/// Get the timephased remaining cost (regular + overtime).
    ///
    /// Corresponds to Java: `public List<Number> getTimephasedRemainingCost(List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_remaining_cost(
        &self,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        let regular = self.get_timephased_remaining_regular_cost(ranges);
        let overtime = self.get_timephased_remaining_overtime_cost(ranges);
        add_timephased_numbers(regular, overtime)
    }

/// Get the timephased actual regular cost.
    ///
    /// Corresponds to Java: `public List<Number> getTimephasedActualRegularCost(List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_actual_regular_cost(
        &self,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        if ranges.is_empty() {
            return Vec::new();
        }

        let resource_type = self.resource_type();
        if self.get_actual_start().is_none() && resource_type != ResourceType::Cost {
            return empty_numbers(ranges.len());
        }

        let assignment_range = LocalDateTimeRange::new(self.get_start(), self.get_finish());
        if out_of_assignment_range(ranges, &assignment_range) {
            return empty_numbers(ranges.len());
        }

        if resource_type == ResourceType::Cost {
            return self.get_timephased_cost_resource_actual_cost(ranges);
        }

        let cost = Some(
            NumberHelper::get_double(self.get_actual_cost())
                - NumberHelper::get_double(self.get_actual_overtime_cost()),
        );

        match self.get_resource_accrue_type() {
            AccrueType::Start => self.get_timephased_cost_accrued_at_start(
                ranges,
                cost,
                &|ranges, units| self.get_timephased_actual_work(ranges, units),
            ),
            AccrueType::End => self.get_timephased_cost_accrued_at_end(
                ranges,
                cost,
                &|ranges, units| self.get_timephased_actual_work(ranges, units),
            ),
            _ => {
                let costs = self.get_timephased_cost(ranges, 0, &|ranges, units| {
                    self.get_timephased_actual_regular_work(ranges, units)
                });
                self.add_timephased_actual_cost_per_use(ranges, costs)
            }
        }
    }

/// Get the timephased actual overtime cost.
    ///
    /// Corresponds to Java: `public List<Number> getTimephasedActualOvertimeCost(List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_actual_overtime_cost(
        &self,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        if ranges.is_empty() {
            return Vec::new();
        }

        let resource_type = self.resource_type();
        if self.get_actual_start().is_none() && resource_type != ResourceType::Cost {
            return empty_numbers(ranges.len());
        }

        let assignment_range = LocalDateTimeRange::new(self.get_start(), self.get_finish());
        if out_of_assignment_range(ranges, &assignment_range) {
            return empty_numbers(ranges.len());
        }

        if resource_type == ResourceType::Cost {
// Java: cost resources have no overtime
            return empty_numbers(ranges.len());
        }

        match self.get_resource_accrue_type() {
            AccrueType::Start => self.get_timephased_cost_accrued_at_start(
                ranges,
                self.get_actual_overtime_cost(),
                &|ranges, units| self.get_timephased_actual_work(ranges, units),
            ),
            AccrueType::End => self.get_timephased_cost_accrued_at_end(
                ranges,
                self.get_actual_overtime_cost(),
                &|ranges, units| self.get_timephased_actual_work(ranges, units),
            ),
            _ => self.get_timephased_cost(ranges, 1, &|ranges, units| {
                self.get_timephased_actual_overtime_work(ranges, units)
            }),
        }
    }

/// Get the timephased planned cost.
    ///
    /// Corresponds to Java: `public List<Number> getTimephasedPlannedCost(List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_planned_cost(&self, ranges: &[LocalDateTimeRange]) -> Vec<Option<f64>> {
        if ranges.is_empty() {
            return Vec::new();
        }

        let assignment_range = LocalDateTimeRange::new(self.get_start(), self.get_finish());
        if out_of_assignment_range(ranges, &assignment_range) {
            return empty_numbers(ranges.len());
        }

        match self.get_resource_accrue_type() {
            AccrueType::Start => self.get_timephased_cost_accrued_at_start(
                ranges,
                self.get_planned_cost(),
                &|ranges, units| self.get_timephased_planned_work(ranges, units),
            ),
            AccrueType::End => self.get_timephased_cost_accrued_at_end(
                ranges,
                self.get_planned_cost(),
                &|ranges, units| self.get_timephased_planned_work(ranges, units),
            ),
            _ => self.get_timephased_cost(ranges, 0, &|ranges, units| {
                self.get_timephased_planned_work(ranges, units)
            }),
        }
    }

/// Get the timephased actual cost (regular + overtime).
    ///
    /// Corresponds to Java: `public List<Number> getTimephasedActualCost(List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_actual_cost(&self, ranges: &[LocalDateTimeRange]) -> Vec<Option<f64>> {
        let regular = self.get_timephased_actual_regular_cost(ranges);
        let overtime = self.get_timephased_actual_overtime_cost(ranges);
        add_timephased_numbers(regular, overtime)
    }

/// Get the timephased cost (actual + remaining).
    ///
    /// Corresponds to Java: `public List<Number> getTimephasedCost(List<LocalDateTimeRange> ranges)`
    ///
/// Rust has no method overloading, so it is distinguished from the private method of the same name [`ResourceAssignment::get_timephased_cost`] by the
/// `_total` suffix (corresponding to the `getTimephasedCost(ranges)` overload in Java).
    pub fn get_timephased_cost_total(&self, ranges: &[LocalDateTimeRange]) -> Vec<Option<f64>> {
        let actual = self.get_timephased_actual_cost(ranges);
        let remaining = self.get_timephased_remaining_cost(ranges);
        add_timephased_numbers(actual, remaining)
    }

/// Get the timephased baseline cost.
    ///
    /// Corresponds to Java: `public List<Number> getTimephasedBaselineCost(int index, List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_baseline_cost(
        &self,
        index: i32,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        let calendar = self.get_baseline_calendar();
        let cost = self.get_raw_timephased_baseline_cost(index).to_vec();
        segment_cost(calendar.as_ref(), &cost, ranges)
    }

/// Get the timephased baseline budget cost.
    ///
    /// Corresponds to Java: `public List<Number> getTimephasedBaselineBudgetCost(int index, List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_baseline_budget_cost(
        &self,
        index: i32,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        let calendar = self.get_baseline_calendar();
        let cost = self.get_raw_timephased_baseline_budget_cost(index).to_vec();
        segment_cost(calendar.as_ref(), &cost, ranges)
    }

/// Resource type (when there is no resource, use Java's default `ResourceType.WORK`).
    fn resource_type(&self) -> ResourceType {
        self.get_resource()
            .and_then(|resource| resource.try_borrow().ok().map(|resource| resource.get_type()))
            .unwrap_or(ResourceType::Work)
    }

/// Calculate the timephased cost using the given work provider function.
    ///
    /// Corresponds to Java: `private List<Number> getTimephasedCost(List<LocalDateTimeRange> ranges, int rateIndex, BiFunction<...> workSupplier)`
    fn get_timephased_cost(
        &self,
        ranges: &[LocalDateTimeRange],
        rate_index: usize,
        work_supplier: &dyn Fn(&[LocalDateTimeRange], TimeUnit) -> Vec<Option<Duration>>,
    ) -> Vec<Option<f64>> {
// Java: returns null when there is no work
        let hours = work_supplier(ranges, TimeUnit::Hours);
        if hours.is_empty() {
            return empty_numbers(ranges.len());
        }

        let (Some(first), Some(last)) = (ranges.first(), ranges.last()) else {
            return empty_numbers(ranges.len());
        };
        let rates_range = LocalDateTimeRange::new(first.get_start(), last.get_end());
        let calendar = self.get_effective_calendar();

        self.with_cost_rate_table(|cost_rate_table| {
            let mut result = empty_numbers(ranges.len());

            let Some(cost_rate_table) = cost_rate_table else {
                return result;
            };

            let rates = cost_rate_table.get_entries_by_range(&rates_range);
            if rates.is_empty() {
                return result;
            }

// Java: assumes the rate table rows are ordered
            let mut cost_rate_table_entry_index = 0usize;
            let mut current_rate = rates[0];

            for index in 0..ranges.len() {
                let Some(work) = hours.get(index).copied().flatten() else {
                    continue;
                };

                if work.get_duration() == 0.0 {
                    result[index] = Some(0.0);
                }

                let range = ranges[index];
                let (Some(range_start), Some(range_end)) = (range.get_start(), range.get_end())
                else {
                    continue;
                };

                while cost_rate_table_entry_index + 1 < rates.len()
                    && current_rate
                        .get_end_date()
                        .map(|end_date| end_date <= range_start)
                        .unwrap_or(false)
                {
                    cost_rate_table_entry_index += 1;
                    current_rate = rates[cost_rate_table_entry_index];
                }

// Java: the rate table rows run out before the range ends -- return the existing result directly
                if cost_rate_table_entry_index >= rates.len() {
                    break;
                }

// Java: the range does not pass the current rate row's end time, so it can be computed in one pass
                if current_rate
                    .get_end_date()
                    .map(|end_date| range_end <= end_date)
                    .unwrap_or(true)
                {
                    let rate =
                        self.get_rate_per_hour(current_rate.get_rate(rate_index), calendar.as_ref());
                    result[index] = Some(work.get_duration() * rate.get_amount());
                    continue;
                }

// spans multiple rate rows within one range
                let mut total = 0.0;
                let mut sub_range = LocalDateTimeRange::new(
                    Some(range_start),
                    get_next_rate_start(&rates, cost_rate_table_entry_index),
                );
                loop {
                    if let Some(Some(sub_work)) = work_supplier(&[sub_range], TimeUnit::Hours).first()
                    {
                        let rate = self
                            .get_rate_per_hour(current_rate.get_rate(rate_index), calendar.as_ref());
                        total += sub_work.get_duration() * rate.get_amount();
                    }

                    if sub_range.get_end() == Some(range_end) {
                        break;
                    }

                    cost_rate_table_entry_index += 1;
                    let Some(rate) = rates.get(cost_rate_table_entry_index).copied() else {
                        break;
                    };
                    current_rate = rate;

                    let end_date = get_next_rate_start(&rates, cost_rate_table_entry_index);
                    let new_end = match end_date {
                        Some(end_date) if end_date > range_end => range_end,
                        Some(end_date) => end_date,
                        None => range_end,
                    };
                    sub_range = LocalDateTimeRange::new(current_rate.get_start_date(), Some(new_end));
                }

                result[index] = Some(total);
            }

            result
        })
    }

/// Convert a rate to an "hourly" rate.
    ///
    /// Corresponds to Java: `private Rate getRatePerHour(Rate rate)`
    fn get_rate_per_hour(
        &self,
        rate: Option<Rate>,
        calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
    ) -> Rate {
        let Some(rate) = rate else {
            return Rate::ZERO;
        };

        if rate.get_units() == TimeUnit::Hours {
            return rate;
        }

        // Java: RateHelper.convertToHours(getEffectiveCalendar(), rate)
        let amount = match calendar.and_then(|calendar| calendar.try_borrow().ok()) {
            Some(calendar) => RateHelper::convert_to_hours(&*calendar, &rate),
// Java's getEffectiveCalendar() is never null (the calendar reference is stable); on the Rust side, when not wired or
// when there is a borrow conflict, return the original value without panicking
            None => rate.get_amount(),
        };

        Rate::value_of(Some(amount), TimeUnit::Hours).unwrap_or(Rate::ZERO)
    }

/// Get the timephased remaining cost of a cost resource.
    ///
    /// Corresponds to Java: `private List<Number> getTimephasedCostResourceRemainingCost(List<LocalDateTimeRange> ranges)`
    fn get_timephased_cost_resource_remaining_cost(
        &self,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        match self.get_resource_accrue_type() {
            AccrueType::Start => self.get_timephased_remaining_cost_accrued_at_start(
                ranges,
                self.get_cost(),
                self.get_remaining_cost(),
            ),
            AccrueType::End => self.get_timephased_remaining_cost_accrued_at_end(
                ranges,
                self.get_cost(),
                self.get_remaining_cost(),
            ),
            _ => self.get_timephased_remaining_cost_pro_rata(ranges),
        }
    }

/// Get the timephased actual cost of a cost resource.
    ///
    /// Corresponds to Java: `private List<Number> getTimephasedCostResourceActualCost(List<LocalDateTimeRange> ranges)`
    fn get_timephased_cost_resource_actual_cost(
        &self,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        match self.get_resource_accrue_type() {
            AccrueType::Start => self.get_timephased_cost_accrued_at_start(
                ranges,
                self.get_actual_cost(),
                &|ranges, units| self.get_timephased_actual_work(ranges, units),
            ),
            AccrueType::End => self.get_timephased_cost_accrued_at_end(
                ranges,
                self.get_actual_cost(),
                &|ranges, units| self.get_timephased_actual_work(ranges, units),
            ),
            _ => self.get_timephased_actual_cost_prorata(ranges),
        }
    }

/// Get the timephased remaining cost that is "incurred at the start of the assignment".
    ///
    /// Corresponds to Java: `private List<Number> getTimephasedRemainingCostAccruedAtStart(List<LocalDateTimeRange> ranges, Supplier<Number> totalCost, Supplier<Number> remainingCost)`
    ///
/// Java throws an out-of-bounds exception when no range intersecting the assignment is found; the Rust side degrades gracefully to returning an empty list.
    fn get_timephased_remaining_cost_accrued_at_start(
        &self,
        ranges: &[LocalDateTimeRange],
        total_cost: Option<f64>,
        remaining_cost: Option<f64>,
    ) -> Vec<Option<f64>> {
        let assignment_range = LocalDateTimeRange::new(self.get_start(), self.get_finish());
        let mut result = empty_numbers(ranges.len());

        let mut range_index = first_intersecting_range(ranges, &assignment_range);
        let remaining_work = self.get_timephased_remaining_work(ranges, TimeUnit::Hours);

        if NumberHelper::get_double(remaining_cost) == 0.0 {
// Java: the assignment has started, actual cost covers the entire cost, and remaining cost is all 0
            while range_index < ranges.len()
                && ranges[range_index].intersects_with(&assignment_range)
            {
                result[range_index] = match remaining_work.get(range_index).copied().flatten() {
                    Some(_) => Some(0.0),
                    None => None,
                };
                range_index += 1;
            }
            return result;
        }

// Java: the assignment has not started, so the cost is included in the assignment's first range
        if range_compare_to_date(&ranges[range_index.max(0)], self.get_start())
            .map(|compare| compare == 0)
            .unwrap_or(false)
        {
            result[range_index] = total_cost;
            range_index += 1;
        }

        while range_index < ranges.len() && ranges[range_index].intersects_with(&assignment_range) {
            result[range_index] = match remaining_work.get(range_index).copied().flatten() {
                Some(_) => Some(0.0),
                None => None,
            };
            range_index += 1;
        }

        result
    }

/// Get the timephased remaining cost that is "incurred at the end of the assignment".
    ///
    /// Corresponds to Java: `private List<Number> getTimephasedRemainingCostAccruedAtEnd(List<LocalDateTimeRange> ranges, Supplier<Number> totalCost, Supplier<Number> remainingCost)`
    fn get_timephased_remaining_cost_accrued_at_end(
        &self,
        ranges: &[LocalDateTimeRange],
        total_cost: Option<f64>,
        remaining_cost: Option<f64>,
    ) -> Vec<Option<f64>> {
        let assignment_range = LocalDateTimeRange::new(self.get_start(), self.get_finish());
        let mut result = empty_numbers(ranges.len());

        let mut range_index = first_intersecting_range(ranges, &assignment_range);

// Java: the ranges covering the assignment range are first all filled with 0
        let remaining_work = self.get_timephased_remaining_work(ranges, TimeUnit::Hours);
        while range_index < ranges.len() && ranges[range_index].intersects_with(&assignment_range) {
            result[range_index] = match remaining_work.get(range_index).copied().flatten() {
                Some(_) => Some(0.0),
                None => None,
            };
            range_index += 1;
        }

        if NumberHelper::get_double(remaining_cost) == 0.0 {
// Java: the assignment has ended, actual cost covers the entire cost
            return result;
        }

        if range_index == 0 || range_index > ranges.len() {
            return result;
        }

// Java: the last intersecting range contains the assignment end time, so the cost is included in that range
        if range_compare_to_date(&ranges[range_index - 1], self.get_finish())
            .map(|compare| compare == 0)
            .unwrap_or(false)
        {
            result[range_index - 1] = total_cost;
        }

        result
    }

/// Get the timephased remaining cost prorated by duration.
    ///
    /// Corresponds to Java: `private List<Number> getTimephasedRemainingCostProRata(List<LocalDateTimeRange> ranges)`
    fn get_timephased_remaining_cost_pro_rata(
        &self,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        let mut result = empty_numbers(ranges.len());

        if NumberHelper::get_double(self.get_remaining_cost()) == 0.0 {
            return result;
        }

        let assignment_range = LocalDateTimeRange::new(self.get_resume(), self.get_finish());
        let mut range_index = first_intersecting_range(ranges, &assignment_range);

        let calendar = self.get_effective_calendar();
        let working_hours = calendar_work(
            calendar.as_ref(),
            self.get_start(),
            self.get_finish(),
            TimeUnit::Hours,
        )
        .get_duration();
        let amount_per_hour = if working_hours == 0.0 {
            0.0
        } else {
            NumberHelper::get_double(self.get_cost()) / working_hours
        };

        while range_index < ranges.len() {
            let Some(intersection) = assignment_range.intersection(&ranges[range_index]) else {
                break;
            };

            let intersection_hours = calendar_work(
                calendar.as_ref(),
                intersection.get_start(),
                intersection.get_end(),
                TimeUnit::Hours,
            )
            .get_duration();
            if intersection_hours != 0.0 {
                result[range_index] = Some(intersection_hours * amount_per_hour);
            }
            range_index += 1;
        }

        result
    }

/// Get the timephased cost that is "incurred at the start of the assignment".
    ///
    /// Corresponds to Java: `private List<Number> getTimephasedCostAccruedAtStart(List<LocalDateTimeRange> ranges, Supplier<Number> costSupplier, BiFunction<...> workSupplier)`
    fn get_timephased_cost_accrued_at_start(
        &self,
        ranges: &[LocalDateTimeRange],
        cost: Option<f64>,
        work_supplier: &dyn Fn(&[LocalDateTimeRange], TimeUnit) -> Vec<Option<Duration>>,
    ) -> Vec<Option<f64>> {
        let assignment_range = LocalDateTimeRange::new(self.get_start(), self.get_finish());
        let mut result = empty_numbers(ranges.len());

        let mut range_index = first_intersecting_range(ranges, &assignment_range);

// Java: the first intersecting range contains the assignment start time, so the cost is included in that range
        if range_index < ranges.len()
            && range_compare_to_date(&ranges[range_index], self.get_start())
                .map(|compare| compare == 0)
                .unwrap_or(false)
        {
            result[range_index] = Some(cost.unwrap_or(0.0));
            range_index += 1;
        }

// Java: the remaining intersecting ranges have a cost of 0
        let work = work_supplier(ranges, TimeUnit::Hours);
        while range_index < ranges.len() && ranges[range_index].intersects_with(&assignment_range) {
            result[range_index] = match work.get(range_index).copied().flatten() {
                Some(_) => Some(0.0),
                None => None,
            };
            range_index += 1;
        }

        result
    }

/// Get the timephased cost that is "incurred at the end of the assignment".
    ///
    /// Corresponds to Java: `private List<Number> getTimephasedCostAccruedAtEnd(List<LocalDateTimeRange> ranges, Supplier<Number> costSupplier, BiFunction<...> workSupplier)`
    fn get_timephased_cost_accrued_at_end(
        &self,
        ranges: &[LocalDateTimeRange],
        cost: Option<f64>,
        work_supplier: &dyn Fn(&[LocalDateTimeRange], TimeUnit) -> Vec<Option<Duration>>,
    ) -> Vec<Option<f64>> {
        let assignment_range = LocalDateTimeRange::new(self.get_start(), self.get_finish());
        let mut result = empty_numbers(ranges.len());

        let mut range_index = first_intersecting_range(ranges, &assignment_range);

        let work = work_supplier(ranges, TimeUnit::Hours);
        while range_index < ranges.len() && ranges[range_index].intersects_with(&assignment_range) {
            let first_assignment_range = range_compare_to_date(&ranges[range_index], self.get_start())
                .map(|compare| compare == 0)
                .unwrap_or(false);
            let has_work = work.get(range_index).copied().flatten().is_some();
            result[range_index] = if !first_assignment_range && !has_work {
                None
            } else {
                Some(0.0)
            };
            range_index += 1;
        }

// Java: the last range contains the assignment end time, so the cost is included in that range
        if range_index > 0 {
            let last_has_work = work.get(range_index - 1).copied().flatten().is_some();
            let ends_with_finish = range_compare_to_date(&ranges[range_index - 1], self.get_finish())
                .map(|compare| compare == 0)
                .unwrap_or(false);
            if ends_with_finish && last_has_work {
                result[range_index - 1] = Some(cost.unwrap_or(0.0));
            }
        }

        result
    }

/// Get the timephased actual cost prorated by duration.
    ///
    /// Corresponds to Java: `private List<Number> getTimephasedActualCostProrata(List<LocalDateTimeRange> ranges)`
    fn get_timephased_actual_cost_prorata(
        &self,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        if NumberHelper::get_double(self.get_actual_cost()) == 0.0 {
            return empty_numbers(ranges.len());
        }

        let assignment_range = LocalDateTimeRange::new(self.get_actual_start(), self.get_resume());
        let Some(first_range) = ranges.first() else {
            return Vec::new();
        };
        let (Some(first_start), Some(first_end)) = (first_range.get_start(), first_range.get_end())
        else {
            return empty_numbers(ranges.len());
        };
        let (Some(start), Some(finish)) = (self.get_start(), self.get_finish()) else {
            return empty_numbers(ranges.len());
        };

// Java: find the starting point aligned with the caller's range and covering the entire assignment
        let minutes_per_range = (first_end - first_start).num_minutes();
        let mut assignment_start_range = first_start;
        if first_start > start {
            while assignment_start_range > start {
                assignment_start_range -= chrono::Duration::minutes(minutes_per_range);
            }
        } else {
            while assignment_start_range + chrono::Duration::minutes(minutes_per_range) < start {
                assignment_start_range += chrono::Duration::minutes(minutes_per_range);
            }
        }

// Generate ranges covering the entire assignment
        let mut assignment_ranges: Vec<LocalDateTimeRange> = Vec::new();
        loop {
            let range = LocalDateTimeRange::new(
                Some(assignment_start_range),
                Some(assignment_start_range + chrono::Duration::minutes(minutes_per_range)),
            );
            assignment_ranges.push(range);
            let Some(end) = range.get_end() else {
                break;
            };
            if end == finish {
                break;
            }
            assignment_start_range = end;
        }

// Count the ranges that contain working time
        let calendar = self.get_effective_calendar();
        let working_ranges = assignment_ranges
            .iter()
            .filter(|range| {
                calendar_work(
                    calendar.as_ref(),
                    range.get_start(),
                    range.get_end(),
                    TimeUnit::Hours,
                )
                .get_duration()
                    != 0.0
            })
            .count();

        let cost_per_working_range = if working_ranges == 0 {
            0.0
        } else {
            NumberHelper::get_double(self.get_cost()) / working_ranges as f64
        };

        let mut range_index = first_intersecting_range(ranges, &assignment_range);
        let mut result = empty_numbers(ranges.len());
        while range_index < ranges.len() && ranges[range_index].intersects_with(&assignment_range) {
            let range = ranges[range_index];
            let end = match (range.get_end(), assignment_range.get_end()) {
                (Some(range_end), Some(assignment_end)) if range_end > assignment_end => {
                    Some(assignment_end)
                }
                (range_end, _) => range_end,
            };
            if calendar_work(calendar.as_ref(), range.get_start(), end, TimeUnit::Hours)
                .get_duration()
                != 0.0
            {
                result[range_index] = Some(cost_per_working_range);
            }
            range_index += 1;
        }

        result
    }

/// Append the "remaining cost per use" to the timephased cost.
    ///
    /// Corresponds to Java: `private List<Number> addTimephasedRemainingCostPerUse(List<LocalDateTimeRange> ranges, List<Number> costs)`
    fn add_timephased_remaining_cost_per_use(
        &self,
        ranges: &[LocalDateTimeRange],
        mut costs: Vec<Option<f64>>,
    ) -> Vec<Option<f64>> {
// Java: if it has started, the cost per use is already included in the actual cost
        if self.get_actual_start().is_some() {
            return costs;
        }

        let Some(resource) = self.get_resource() else {
            return costs;
        };

        let mut cost_per_use = NumberHelper::get_double(
            resource
                .try_borrow()
                .ok()
                .and_then(|resource| resource.get_cost_per_use()),
        );
        if cost_per_use == 0.0 {
            return costs;
        }

// Java: for non-material resources with units other than 100%, scale by the units
        let units = NumberHelper::get_double(self.get_units());
        let time_based = !self.is_material_resource();
        if time_based && units != 100.0 {
            cost_per_use = (cost_per_use * units) / 100.0;
        }

        for range_index in 0..ranges.len() {
            if range_compare_to_date(&ranges[range_index], self.get_start())
                .map(|compare| compare == 0)
                .unwrap_or(false)
            {
                costs[range_index] = Some(NumberHelper::get_double(costs[range_index]) + cost_per_use);
                break;
            }
        }

        costs
    }

/// Append the "actual cost per use" to the timephased cost.
    ///
    /// Corresponds to Java: `private List<Number> addTimephasedActualCostPerUse(List<LocalDateTimeRange> ranges, List<Number> costs)`
    fn add_timephased_actual_cost_per_use(
        &self,
        ranges: &[LocalDateTimeRange],
        mut costs: Vec<Option<f64>>,
    ) -> Vec<Option<f64>> {
// Java: if it has not started, the cost per use is included in the remaining cost
        if self.get_actual_start().is_none() {
            return costs;
        }

        let Some(resource) = self.get_resource() else {
            return costs;
        };

        let Some(cost_per_use) = resource
            .try_borrow()
            .ok()
            .and_then(|resource| resource.get_cost_per_use())
        else {
            return costs;
        };
        if cost_per_use == 0.0 {
            return costs;
        }

        for range_index in 0..ranges.len() {
            if range_compare_to_date(&ranges[range_index], self.get_start())
                .map(|compare| compare == 0)
                .unwrap_or(false)
            {
                costs[range_index] = Some(NumberHelper::get_double(costs[range_index]) + cost_per_use);
                break;
            }
        }

        costs
    }

/// Get the raw timephased planned work; when absent, generate it from the planned work and planned start/end times.
    ///
    /// Corresponds to Java: `private List<TimephasedWork> getOrCreateRawTimephasedPlannedWork()`
    fn get_or_create_raw_timephased_planned_work(&self) -> Vec<TimephasedWork> {
        let planned_work = self.get_raw_timephased_planned_work();
        if !planned_work.is_empty() {
            return planned_work;
        }

        let Some(work) = self.get_planned_work() else {
            return Vec::new();
        };
        if work.get_duration() == 0.0 {
            return Vec::new();
        }

        let calendar = self.get_effective_calendar();
        let start = self.get_planned_start();
        let finish = self.get_planned_finish();
        let working_hours =
            calendar_work(calendar.as_ref(), start, finish, TimeUnit::Hours).get_duration();
        let planned_hours = convert_units(work, TimeUnit::Hours, calendar.as_ref()).get_duration();

        let amount_per_hour = if working_hours == 0.0 {
            planned_hours
        } else {
            planned_hours / working_hours
        };

        let mut item = TimephasedWork::new();
        item.set_start(start);
        item.set_finish(finish);
        item.set_total_amount(Some(work));
        item.set_amount_per_hour(Some(Duration::get_instance(amount_per_hour, TimeUnit::Hours)));
        vec![item]
    }

/// Get the raw timephased actual work; when absent, generate it from the actual work and actual start/end times.
    ///
    /// Corresponds to Java: `private List<TimephasedWork> getOrCreateRawTimephasedActualWork(List<TimephasedWork> timephasedWork, Supplier<Duration> workSupplier)`
    fn get_or_create_raw_timephased_actual_work(
        &self,
        timephased_work: Vec<TimephasedWork>,
        work: Option<Duration>,
    ) -> Vec<TimephasedWork> {
        if !timephased_work.is_empty() || self.get_actual_start().is_none() {
            return timephased_work;
        }

        let Some(work) = work else {
            return timephased_work;
        };
        if work.get_duration() == 0.0 {
            return timephased_work;
        }

        let calendar = self.get_effective_calendar();
        let start = self.get_actual_start();
        let mut finish = self.get_actual_finish();
        if finish.is_none() {
            let units = NumberHelper::get_double(self.get_units());
            if units == 0.0 {
// Java: XER files may have units of 0, in which case generation is abandoned directly
                return timephased_work;
            }

            let mut elapsed_work = work;
            if units != 100.0 {
                elapsed_work = Duration::get_instance(
                    (elapsed_work.get_duration() * 100.0) / units,
                    elapsed_work.get_units(),
                );
            }

            finish = match (start, calendar.as_ref()) {
                (Some(start), Some(calendar)) => match calendar.try_borrow_mut() {
                    Ok(mut calendar) => Some(calendar.get_date(start, elapsed_work)),
                    Err(_) => None,
                },
                _ => None,
            };
        }

        let working_hours =
            calendar_work(calendar.as_ref(), start, finish, TimeUnit::Hours).get_duration();
        let actual_hours = convert_units(work, TimeUnit::Hours, calendar.as_ref()).get_duration();

        let amount_per_hour = if working_hours == 0.0 {
            actual_hours
        } else {
            actual_hours / working_hours
        };

        let mut item = TimephasedWork::new();
        item.set_start(start);
        item.set_finish(finish);
        item.set_total_amount(Some(work));
        item.set_amount_per_hour(Some(Duration::get_instance(amount_per_hour, TimeUnit::Hours)));
        vec![item]
    }

/// Get the raw timephased remaining work; when absent, generate it from the remaining work and remaining start/end times.
    ///
    /// Corresponds to Java: `private List<TimephasedWork> getOrCreateRawTimephasedRemainingWork(List<TimephasedWork> timephasedWork, Supplier<Duration> workSupplier)`
    fn get_or_create_raw_timephased_remaining_work(
        &self,
        timephased_work: Vec<TimephasedWork>,
        work: Option<Duration>,
    ) -> Vec<TimephasedWork> {
        if !timephased_work.is_empty() || self.get_actual_finish().is_some() {
            return timephased_work;
        }

        let Some(work) = work else {
            return timephased_work;
        };
        if work.get_duration() == 0.0 {
            return timephased_work;
        }

        let calendar = self.get_effective_calendar();
        let actual_work = self.get_actual_work();

// Java: when it has started and actual work has been generated, continue from the "remaining earliest start time";
// when there is no remaining earliest start time, derive the next work start moment from the actual start time and actual work.
        let start = match (self.get_actual_start(), actual_work) {
            (Some(actual_start), Some(actual_work)) if actual_work.get_duration() != 0.0 => {
                match self.get_remaining_early_start() {
                    Some(remaining_early_start) => Some(remaining_early_start),
                    None => match calendar.as_ref() {
                        Some(calendar) => match calendar.try_borrow_mut() {
                            Ok(mut calendar) => {
                                let date = calendar.get_date(actual_start, actual_work);
                                Some(calendar.get_next_work_start(date))
                            }
                            Err(_) => None,
                        },
                        None => None,
                    },
                }
            }
            _ => self.get_start(),
        };

        let finish = self.get_finish();
        let working_hours =
            calendar_work(calendar.as_ref(), start, finish, TimeUnit::Hours).get_duration();
        let remaining_hours = convert_units(work, TimeUnit::Hours, calendar.as_ref()).get_duration();

        let amount_per_hour = if working_hours == 0.0 {
            remaining_hours
        } else {
            remaining_hours / working_hours
        };

        let mut item = TimephasedWork::new();
        item.set_start(start);
        item.set_finish(finish);
        item.set_total_amount(Some(work));
        item.set_amount_per_hour(Some(Duration::get_instance(amount_per_hour, TimeUnit::Hours)));
        vec![item]
    }
}

impl FieldContainer for ResourceAssignment {
    /// Corresponds to Java: `public void set(FieldType field, Object value)`
    fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
        self.set_key(field, value);
    }

    /// Corresponds to Java: `public Object get(FieldType field)`
    fn get(&self, field: FieldKey) -> Option<FieldValue> {
        self.get_field(field)
    }

    /// Corresponds to Java: `public Object getCachedValue(FieldType field)`
    fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
        self.fields.get_cached_value(field)
    }

    /// Corresponds to Java: `@Override public List<Duration> getTimephasedDurationValues(FieldType field, List<LocalDateTimeRange> ranges, TimeUnit units)`
    ///
/// Java dispatches via `TIMEPHASED_WORK_FUNCTIONS`; the Rust side is handled by an ordinary method of the same name in this class (see section 5 in the file header).
    fn get_timephased_duration_values(
        &self,
        field: FieldKey,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        ResourceAssignment::get_timephased_duration_values(self, field, ranges, units)
    }

    /// Corresponds to Java: `@Override public List<Number> getTimephasedNumericValues(FieldType field, List<LocalDateTimeRange> ranges)`
    ///
/// Java dispatches via `TIMEPHASED_NUMERIC_FUNCTIONS`; the Rust side is handled by an ordinary method of the same name in this class.
    fn get_timephased_numeric_values(
        &self,
        field: FieldKey,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        ResourceAssignment::get_timephased_numeric_values(self, field, ranges)
    }

    /// Corresponds to Java: `public void addFieldListener(FieldListener listener)`
    fn add_field_listener(&mut self, listener: Box<dyn FieldListener>) {
        self.fields.add_field_listener(listener);
    }

    /// Corresponds to Java: `public void removeFieldListener(FieldListener listener)`
    fn clear_field_listeners(&mut self) {
        self.fields.clear_field_listeners();
    }
}

impl ProjectEntityWithUniqueID for ResourceAssignment {
    /// Corresponds to Java: `@Override public Integer getUniqueID()`
    fn get_unique_id(&self) -> Option<i32> {
        ResourceAssignment::get_unique_id(self)
    }
}

impl ProjectEntityWithMutableUniqueID for ResourceAssignment {
    /// Corresponds to Java: `@Override public void setUniqueID(Integer uniqueID)`
    fn set_unique_id(&mut self, id: Option<i32>) {
        ResourceAssignment::set_unique_id(self, id);
    }
}

impl Display for ResourceAssignment {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java renders null values with `null`; the Rust side keeps the same text (`NaiveDateTime` uses a space as separator,
/// whereas Java uses `T`).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let task_name = match self.get_task() {
            Some(task) => task
                .try_borrow()
                .ok()
                .and_then(|task| task.get_name())
                .unwrap_or_else(|| "null".to_owned()),
            None => "null".to_owned(),
        };

        let resource_name = match self.get_resource() {
            Some(resource) => resource
                .try_borrow()
                .ok()
                .and_then(|resource| resource.get_name())
                .unwrap_or_else(|| "Unassigned".to_owned()),
            None => "Unassigned".to_owned(),
        };

        write!(
            f,
            "[Resource Assignment task={} resource={} start={} finish={} work={} units={} workContour={}]",
            task_name,
            resource_name,
            display_option(self.get_start().as_ref()),
            display_option(self.get_finish().as_ref()),
            display_option(self.get_work().as_ref()),
            display_option(self.get_units().as_ref()),
            display_option(self.get_work_contour().as_ref()),
        )
    }
}

/// Default duration units when calculating the time variance (degraded value when the parent file is not wired).
///
/// Corresponds to Java's `ProjectProperties.getDefaultDurationUnits()` (the MSP default is days).
const DEFAULT_DURATION_UNITS: TimeUnit = TimeUnit::Days;

/// Baseline cost fields (indices 1-10).
///
/// Corresponds to Java: `AssignmentFieldLists.BASELINE_COSTS`
const BASELINE_COSTS: [AssignmentField; 10] = [
    AssignmentField::Baseline1Cost,
    AssignmentField::Baseline2Cost,
    AssignmentField::Baseline3Cost,
    AssignmentField::Baseline4Cost,
    AssignmentField::Baseline5Cost,
    AssignmentField::Baseline6Cost,
    AssignmentField::Baseline7Cost,
    AssignmentField::Baseline8Cost,
    AssignmentField::Baseline9Cost,
    AssignmentField::Baseline10Cost,
];

/// Baseline work fields (indices 1-10).
///
/// Corresponds to Java: `AssignmentFieldLists.BASELINE_WORKS`
const BASELINE_WORKS: [AssignmentField; 10] = [
    AssignmentField::Baseline1Work,
    AssignmentField::Baseline2Work,
    AssignmentField::Baseline3Work,
    AssignmentField::Baseline4Work,
    AssignmentField::Baseline5Work,
    AssignmentField::Baseline6Work,
    AssignmentField::Baseline7Work,
    AssignmentField::Baseline8Work,
    AssignmentField::Baseline9Work,
    AssignmentField::Baseline10Work,
];

/// Baseline material quantity fields (indices 1-10).
///
/// Corresponds to Java: `AssignmentFieldLists.BASELINE_MATERIALS`
const BASELINE_MATERIALS: [AssignmentField; 10] = [
    AssignmentField::Baseline1Material,
    AssignmentField::Baseline2Material,
    AssignmentField::Baseline3Material,
    AssignmentField::Baseline4Material,
    AssignmentField::Baseline5Material,
    AssignmentField::Baseline6Material,
    AssignmentField::Baseline7Material,
    AssignmentField::Baseline8Material,
    AssignmentField::Baseline9Material,
    AssignmentField::Baseline10Material,
];

/// Baseline start time fields (indices 1-10).
///
/// Corresponds to Java: `AssignmentFieldLists.BASELINE_STARTS`
const BASELINE_STARTS: [AssignmentField; 10] = [
    AssignmentField::Baseline1Start,
    AssignmentField::Baseline2Start,
    AssignmentField::Baseline3Start,
    AssignmentField::Baseline4Start,
    AssignmentField::Baseline5Start,
    AssignmentField::Baseline6Start,
    AssignmentField::Baseline7Start,
    AssignmentField::Baseline8Start,
    AssignmentField::Baseline9Start,
    AssignmentField::Baseline10Start,
];

/// Baseline finish time fields (indices 1-10).
///
/// Corresponds to Java: `AssignmentFieldLists.BASELINE_FINISHES`
const BASELINE_FINISHES: [AssignmentField; 10] = [
    AssignmentField::Baseline1Finish,
    AssignmentField::Baseline2Finish,
    AssignmentField::Baseline3Finish,
    AssignmentField::Baseline4Finish,
    AssignmentField::Baseline5Finish,
    AssignmentField::Baseline6Finish,
    AssignmentField::Baseline7Finish,
    AssignmentField::Baseline8Finish,
    AssignmentField::Baseline9Finish,
    AssignmentField::Baseline10Finish,
];

/// Baseline budget cost fields (indices 1-10).
///
/// Corresponds to Java: `AssignmentFieldLists.BASELINE_BUDGET_COSTS`
const BASELINE_BUDGET_COSTS: [AssignmentField; 10] = [
    AssignmentField::Baseline1BudgetCost,
    AssignmentField::Baseline2BudgetCost,
    AssignmentField::Baseline3BudgetCost,
    AssignmentField::Baseline4BudgetCost,
    AssignmentField::Baseline5BudgetCost,
    AssignmentField::Baseline6BudgetCost,
    AssignmentField::Baseline7BudgetCost,
    AssignmentField::Baseline8BudgetCost,
    AssignmentField::Baseline9BudgetCost,
    AssignmentField::Baseline10BudgetCost,
];

/// Baseline budget work fields (indices 1-10).
///
/// Corresponds to Java: `AssignmentFieldLists.BASELINE_BUDGET_WORKS`
const BASELINE_BUDGET_WORKS: [AssignmentField; 10] = [
    AssignmentField::Baseline1BudgetWork,
    AssignmentField::Baseline2BudgetWork,
    AssignmentField::Baseline3BudgetWork,
    AssignmentField::Baseline4BudgetWork,
    AssignmentField::Baseline5BudgetWork,
    AssignmentField::Baseline6BudgetWork,
    AssignmentField::Baseline7BudgetWork,
    AssignmentField::Baseline8BudgetWork,
    AssignmentField::Baseline9BudgetWork,
    AssignmentField::Baseline10BudgetWork,
];

/// Custom text fields (indices 1-30).
///
/// Corresponds to Java: `AssignmentFieldLists.CUSTOM_TEXT`
const CUSTOM_TEXT: [AssignmentField; 30] = [
    AssignmentField::Text1,
    AssignmentField::Text2,
    AssignmentField::Text3,
    AssignmentField::Text4,
    AssignmentField::Text5,
    AssignmentField::Text6,
    AssignmentField::Text7,
    AssignmentField::Text8,
    AssignmentField::Text9,
    AssignmentField::Text10,
    AssignmentField::Text11,
    AssignmentField::Text12,
    AssignmentField::Text13,
    AssignmentField::Text14,
    AssignmentField::Text15,
    AssignmentField::Text16,
    AssignmentField::Text17,
    AssignmentField::Text18,
    AssignmentField::Text19,
    AssignmentField::Text20,
    AssignmentField::Text21,
    AssignmentField::Text22,
    AssignmentField::Text23,
    AssignmentField::Text24,
    AssignmentField::Text25,
    AssignmentField::Text26,
    AssignmentField::Text27,
    AssignmentField::Text28,
    AssignmentField::Text29,
    AssignmentField::Text30,
];

/// Custom start time fields (indices 1-10).
///
/// Corresponds to Java: `AssignmentFieldLists.CUSTOM_START`
const CUSTOM_START: [AssignmentField; 10] = [
    AssignmentField::Start1,
    AssignmentField::Start2,
    AssignmentField::Start3,
    AssignmentField::Start4,
    AssignmentField::Start5,
    AssignmentField::Start6,
    AssignmentField::Start7,
    AssignmentField::Start8,
    AssignmentField::Start9,
    AssignmentField::Start10,
];

/// Custom finish time fields (indices 1-10).
///
/// Corresponds to Java: `AssignmentFieldLists.CUSTOM_FINISH`
const CUSTOM_FINISH: [AssignmentField; 10] = [
    AssignmentField::Finish1,
    AssignmentField::Finish2,
    AssignmentField::Finish3,
    AssignmentField::Finish4,
    AssignmentField::Finish5,
    AssignmentField::Finish6,
    AssignmentField::Finish7,
    AssignmentField::Finish8,
    AssignmentField::Finish9,
    AssignmentField::Finish10,
];

/// Custom date fields (indices 1-10).
///
/// Corresponds to Java: `AssignmentFieldLists.CUSTOM_DATE`
const CUSTOM_DATE: [AssignmentField; 10] = [
    AssignmentField::Date1,
    AssignmentField::Date2,
    AssignmentField::Date3,
    AssignmentField::Date4,
    AssignmentField::Date5,
    AssignmentField::Date6,
    AssignmentField::Date7,
    AssignmentField::Date8,
    AssignmentField::Date9,
    AssignmentField::Date10,
];

/// Custom number fields (indices 1-20).
///
/// Corresponds to Java: `AssignmentFieldLists.CUSTOM_NUMBER`
const CUSTOM_NUMBER: [AssignmentField; 20] = [
    AssignmentField::Number1,
    AssignmentField::Number2,
    AssignmentField::Number3,
    AssignmentField::Number4,
    AssignmentField::Number5,
    AssignmentField::Number6,
    AssignmentField::Number7,
    AssignmentField::Number8,
    AssignmentField::Number9,
    AssignmentField::Number10,
    AssignmentField::Number11,
    AssignmentField::Number12,
    AssignmentField::Number13,
    AssignmentField::Number14,
    AssignmentField::Number15,
    AssignmentField::Number16,
    AssignmentField::Number17,
    AssignmentField::Number18,
    AssignmentField::Number19,
    AssignmentField::Number20,
];

/// Custom duration fields (indices 1-10).
///
/// Corresponds to Java: `AssignmentFieldLists.CUSTOM_DURATION`
const CUSTOM_DURATION: [AssignmentField; 10] = [
    AssignmentField::Duration1,
    AssignmentField::Duration2,
    AssignmentField::Duration3,
    AssignmentField::Duration4,
    AssignmentField::Duration5,
    AssignmentField::Duration6,
    AssignmentField::Duration7,
    AssignmentField::Duration8,
    AssignmentField::Duration9,
    AssignmentField::Duration10,
];

/// Custom cost fields (indices 1-10).
///
/// Corresponds to Java: `AssignmentFieldLists.CUSTOM_COST`
const CUSTOM_COST: [AssignmentField; 10] = [
    AssignmentField::Cost1,
    AssignmentField::Cost2,
    AssignmentField::Cost3,
    AssignmentField::Cost4,
    AssignmentField::Cost5,
    AssignmentField::Cost6,
    AssignmentField::Cost7,
    AssignmentField::Cost8,
    AssignmentField::Cost9,
    AssignmentField::Cost10,
];

/// Custom boolean flag fields (indices 1-20).
///
/// Corresponds to Java: `AssignmentFieldLists.CUSTOM_FLAG`
const CUSTOM_FLAG: [AssignmentField; 20] = [
    AssignmentField::Flag1,
    AssignmentField::Flag2,
    AssignmentField::Flag3,
    AssignmentField::Flag4,
    AssignmentField::Flag5,
    AssignmentField::Flag6,
    AssignmentField::Flag7,
    AssignmentField::Flag8,
    AssignmentField::Flag9,
    AssignmentField::Flag10,
    AssignmentField::Flag11,
    AssignmentField::Flag12,
    AssignmentField::Flag13,
    AssignmentField::Flag14,
    AssignmentField::Flag15,
    AssignmentField::Flag16,
    AssignmentField::Flag17,
    AssignmentField::Flag18,
    AssignmentField::Flag19,
    AssignmentField::Flag20,
];

/// Enterprise custom cost fields (indices 1-10).
///
/// Corresponds to Java: `AssignmentFieldLists.ENTERPRISE_CUSTOM_COST`
const ENTERPRISE_CUSTOM_COST: [AssignmentField; 10] = [
    AssignmentField::EnterpriseCost1,
    AssignmentField::EnterpriseCost2,
    AssignmentField::EnterpriseCost3,
    AssignmentField::EnterpriseCost4,
    AssignmentField::EnterpriseCost5,
    AssignmentField::EnterpriseCost6,
    AssignmentField::EnterpriseCost7,
    AssignmentField::EnterpriseCost8,
    AssignmentField::EnterpriseCost9,
    AssignmentField::EnterpriseCost10,
];

/// Enterprise custom date fields (indices 1-30).
///
/// Corresponds to Java: `AssignmentFieldLists.ENTERPRISE_CUSTOM_DATE`
const ENTERPRISE_CUSTOM_DATE: [AssignmentField; 30] = [
    AssignmentField::EnterpriseDate1,
    AssignmentField::EnterpriseDate2,
    AssignmentField::EnterpriseDate3,
    AssignmentField::EnterpriseDate4,
    AssignmentField::EnterpriseDate5,
    AssignmentField::EnterpriseDate6,
    AssignmentField::EnterpriseDate7,
    AssignmentField::EnterpriseDate8,
    AssignmentField::EnterpriseDate9,
    AssignmentField::EnterpriseDate10,
    AssignmentField::EnterpriseDate11,
    AssignmentField::EnterpriseDate12,
    AssignmentField::EnterpriseDate13,
    AssignmentField::EnterpriseDate14,
    AssignmentField::EnterpriseDate15,
    AssignmentField::EnterpriseDate16,
    AssignmentField::EnterpriseDate17,
    AssignmentField::EnterpriseDate18,
    AssignmentField::EnterpriseDate19,
    AssignmentField::EnterpriseDate20,
    AssignmentField::EnterpriseDate21,
    AssignmentField::EnterpriseDate22,
    AssignmentField::EnterpriseDate23,
    AssignmentField::EnterpriseDate24,
    AssignmentField::EnterpriseDate25,
    AssignmentField::EnterpriseDate26,
    AssignmentField::EnterpriseDate27,
    AssignmentField::EnterpriseDate28,
    AssignmentField::EnterpriseDate29,
    AssignmentField::EnterpriseDate30,
];

/// Enterprise custom duration fields (indices 1-10).
///
/// Corresponds to Java: `AssignmentFieldLists.ENTERPRISE_CUSTOM_DURATION`
const ENTERPRISE_CUSTOM_DURATION: [AssignmentField; 10] = [
    AssignmentField::EnterpriseDuration1,
    AssignmentField::EnterpriseDuration2,
    AssignmentField::EnterpriseDuration3,
    AssignmentField::EnterpriseDuration4,
    AssignmentField::EnterpriseDuration5,
    AssignmentField::EnterpriseDuration6,
    AssignmentField::EnterpriseDuration7,
    AssignmentField::EnterpriseDuration8,
    AssignmentField::EnterpriseDuration9,
    AssignmentField::EnterpriseDuration10,
];

/// Enterprise custom boolean flag fields (indices 1-20).
///
/// Corresponds to Java: `AssignmentFieldLists.ENTERPRISE_CUSTOM_FLAG`
const ENTERPRISE_CUSTOM_FLAG: [AssignmentField; 20] = [
    AssignmentField::EnterpriseFlag1,
    AssignmentField::EnterpriseFlag2,
    AssignmentField::EnterpriseFlag3,
    AssignmentField::EnterpriseFlag4,
    AssignmentField::EnterpriseFlag5,
    AssignmentField::EnterpriseFlag6,
    AssignmentField::EnterpriseFlag7,
    AssignmentField::EnterpriseFlag8,
    AssignmentField::EnterpriseFlag9,
    AssignmentField::EnterpriseFlag10,
    AssignmentField::EnterpriseFlag11,
    AssignmentField::EnterpriseFlag12,
    AssignmentField::EnterpriseFlag13,
    AssignmentField::EnterpriseFlag14,
    AssignmentField::EnterpriseFlag15,
    AssignmentField::EnterpriseFlag16,
    AssignmentField::EnterpriseFlag17,
    AssignmentField::EnterpriseFlag18,
    AssignmentField::EnterpriseFlag19,
    AssignmentField::EnterpriseFlag20,
];

/// Enterprise custom number fields (indices 1-40).
///
/// Corresponds to Java: `AssignmentFieldLists.ENTERPRISE_CUSTOM_NUMBER`
const ENTERPRISE_CUSTOM_NUMBER: [AssignmentField; 40] = [
    AssignmentField::EnterpriseNumber1,
    AssignmentField::EnterpriseNumber2,
    AssignmentField::EnterpriseNumber3,
    AssignmentField::EnterpriseNumber4,
    AssignmentField::EnterpriseNumber5,
    AssignmentField::EnterpriseNumber6,
    AssignmentField::EnterpriseNumber7,
    AssignmentField::EnterpriseNumber8,
    AssignmentField::EnterpriseNumber9,
    AssignmentField::EnterpriseNumber10,
    AssignmentField::EnterpriseNumber11,
    AssignmentField::EnterpriseNumber12,
    AssignmentField::EnterpriseNumber13,
    AssignmentField::EnterpriseNumber14,
    AssignmentField::EnterpriseNumber15,
    AssignmentField::EnterpriseNumber16,
    AssignmentField::EnterpriseNumber17,
    AssignmentField::EnterpriseNumber18,
    AssignmentField::EnterpriseNumber19,
    AssignmentField::EnterpriseNumber20,
    AssignmentField::EnterpriseNumber21,
    AssignmentField::EnterpriseNumber22,
    AssignmentField::EnterpriseNumber23,
    AssignmentField::EnterpriseNumber24,
    AssignmentField::EnterpriseNumber25,
    AssignmentField::EnterpriseNumber26,
    AssignmentField::EnterpriseNumber27,
    AssignmentField::EnterpriseNumber28,
    AssignmentField::EnterpriseNumber29,
    AssignmentField::EnterpriseNumber30,
    AssignmentField::EnterpriseNumber31,
    AssignmentField::EnterpriseNumber32,
    AssignmentField::EnterpriseNumber33,
    AssignmentField::EnterpriseNumber34,
    AssignmentField::EnterpriseNumber35,
    AssignmentField::EnterpriseNumber36,
    AssignmentField::EnterpriseNumber37,
    AssignmentField::EnterpriseNumber38,
    AssignmentField::EnterpriseNumber39,
    AssignmentField::EnterpriseNumber40,
];

/// Enterprise custom text fields (indices 1-40).
///
/// Corresponds to Java: `AssignmentFieldLists.ENTERPRISE_CUSTOM_TEXT`
const ENTERPRISE_CUSTOM_TEXT: [AssignmentField; 40] = [
    AssignmentField::EnterpriseText1,
    AssignmentField::EnterpriseText2,
    AssignmentField::EnterpriseText3,
    AssignmentField::EnterpriseText4,
    AssignmentField::EnterpriseText5,
    AssignmentField::EnterpriseText6,
    AssignmentField::EnterpriseText7,
    AssignmentField::EnterpriseText8,
    AssignmentField::EnterpriseText9,
    AssignmentField::EnterpriseText10,
    AssignmentField::EnterpriseText11,
    AssignmentField::EnterpriseText12,
    AssignmentField::EnterpriseText13,
    AssignmentField::EnterpriseText14,
    AssignmentField::EnterpriseText15,
    AssignmentField::EnterpriseText16,
    AssignmentField::EnterpriseText17,
    AssignmentField::EnterpriseText18,
    AssignmentField::EnterpriseText19,
    AssignmentField::EnterpriseText20,
    AssignmentField::EnterpriseText21,
    AssignmentField::EnterpriseText22,
    AssignmentField::EnterpriseText23,
    AssignmentField::EnterpriseText24,
    AssignmentField::EnterpriseText25,
    AssignmentField::EnterpriseText26,
    AssignmentField::EnterpriseText27,
    AssignmentField::EnterpriseText28,
    AssignmentField::EnterpriseText29,
    AssignmentField::EnterpriseText30,
    AssignmentField::EnterpriseText31,
    AssignmentField::EnterpriseText32,
    AssignmentField::EnterpriseText33,
    AssignmentField::EnterpriseText34,
    AssignmentField::EnterpriseText35,
    AssignmentField::EnterpriseText36,
    AssignmentField::EnterpriseText37,
    AssignmentField::EnterpriseText38,
    AssignmentField::EnterpriseText39,
    AssignmentField::EnterpriseText40,
];

/// Field dependency table: fields that must be cleared (recalculated) after a given field is modified.
///
/// Corresponds to Java: `private static final Map<FieldType, List<FieldType>> DEPENDENCY_MAP`
/// Corresponds to `FieldContainerDependencies.calculatedField(X).dependsOn(A, B)`
/// (meaning: clear the cached value of `X` when `A` / `B` changes).
fn dependency_map() -> &'static HashMap<FieldKey, Vec<FieldKey>> {
    static DEPENDENCY_MAP: OnceLock<HashMap<FieldKey, Vec<FieldKey>>> = OnceLock::new();

    DEPENDENCY_MAP.get_or_init(|| {
        let mut map: HashMap<FieldKey, Vec<FieldKey>> = HashMap::new();
        let mut depend = |dependent: AssignmentField, sources: &[AssignmentField]| {
            for source in sources {
                map.entry(FieldKey::Assignment(*source))
                    .or_default()
                    .push(FieldKey::Assignment(dependent));
            }
        };

        use AssignmentField as F;
        depend(
            F::OvertimeCost,
            &[F::ActualOvertimeCost, F::RemainingOvertimeCost],
        );
        depend(F::CostVariance, &[F::Cost, F::BaselineCost]);
        depend(F::Cv, &[F::Bcwp, F::Acwp]);
        depend(F::Sv, &[F::Bcwp, F::Bcws]);
        depend(F::StartVariance, &[F::Start, F::BaselineStart]);
        depend(F::FinishVariance, &[F::Finish, F::BaselineFinish]);
        depend(F::PercentWorkComplete, &[F::ActualWork, F::Work]);
        depend(F::WorkVariance, &[F::Work, F::BaselineWork]);
        depend(
            F::RemainingRegularCost,
            &[F::RemainingCost, F::RemainingOvertimeCost],
        );
        depend(F::ActualRegularCost, &[F::ActualCost, F::ActualOvertimeCost]);
        depend(
            F::RemainingRegularWork,
            &[F::RemainingWork, F::RemainingOvertimeWork],
        );
        depend(F::ActualRegularWork, &[F::ActualWork, F::ActualOvertimeWork]);
        depend(F::ActualMaterial, &[F::ActualWork]);
        depend(F::RemainingMaterial, &[F::RemainingWork]);
        depend(F::Material, &[F::Work]);
        depend(F::BaselineMaterial, &[F::BaselineWork]);
        for (material, work) in [
            (F::Baseline1Material, F::Baseline1Work),
            (F::Baseline2Material, F::Baseline2Work),
            (F::Baseline3Material, F::Baseline3Work),
            (F::Baseline4Material, F::Baseline4Work),
            (F::Baseline5Material, F::Baseline5Work),
            (F::Baseline6Material, F::Baseline6Work),
            (F::Baseline7Material, F::Baseline7Work),
            (F::Baseline8Material, F::Baseline8Work),
            (F::Baseline9Material, F::Baseline9Work),
            (F::Baseline10Material, F::Baseline10Work),
        ] {
            depend(material, &[work]);
        }

        map
    })
}

/// Get the "raw timephased baseline work" field by index.
///
/// Corresponds to Java: `AssignmentFieldLists.RAW_TIMEPHASED_BASELINE_WORKS[index]`
/// (index 0 = Baseline, 1..=10 = Baseline1..10); returns `None` when out of bounds
/// (Java throws `ArrayIndexOutOfBoundsException`).
fn raw_timephased_baseline_work_field(index: i32) -> Option<AssignmentField> {
    const FIELDS: [AssignmentField; 11] = [
        AssignmentField::RawTimephasedBaselineWork,
        AssignmentField::RawTimephasedBaseline1Work,
        AssignmentField::RawTimephasedBaseline2Work,
        AssignmentField::RawTimephasedBaseline3Work,
        AssignmentField::RawTimephasedBaseline4Work,
        AssignmentField::RawTimephasedBaseline5Work,
        AssignmentField::RawTimephasedBaseline6Work,
        AssignmentField::RawTimephasedBaseline7Work,
        AssignmentField::RawTimephasedBaseline8Work,
        AssignmentField::RawTimephasedBaseline9Work,
        AssignmentField::RawTimephasedBaseline10Work,
    ];

    usize::try_from(index).ok().and_then(|index| FIELDS.get(index).copied())
}

/// Get the "raw timephased baseline budget work" field by index.
///
/// Corresponds to Java: `AssignmentFieldLists.RAW_TIMEPHASED_BASELINE_BUDGET_WORKS[index]`
fn raw_timephased_baseline_budget_work_field(index: i32) -> Option<AssignmentField> {
    const FIELDS: [AssignmentField; 11] = [
        AssignmentField::RawTimephasedBaselineBudgetWork,
        AssignmentField::RawTimephasedBaseline1BudgetWork,
        AssignmentField::RawTimephasedBaseline2BudgetWork,
        AssignmentField::RawTimephasedBaseline3BudgetWork,
        AssignmentField::RawTimephasedBaseline4BudgetWork,
        AssignmentField::RawTimephasedBaseline5BudgetWork,
        AssignmentField::RawTimephasedBaseline6BudgetWork,
        AssignmentField::RawTimephasedBaseline7BudgetWork,
        AssignmentField::RawTimephasedBaseline8BudgetWork,
        AssignmentField::RawTimephasedBaseline9BudgetWork,
        AssignmentField::RawTimephasedBaseline10BudgetWork,
    ];

    usize::try_from(index).ok().and_then(|index| FIELDS.get(index).copied())
}

/// Get the "raw timephased baseline cost" field by index.
///
/// Corresponds to Java: `AssignmentFieldLists.RAW_TIMEPHASED_BASELINE_COSTS[index]`
fn raw_timephased_baseline_cost_field(index: i32) -> Option<AssignmentField> {
    const FIELDS: [AssignmentField; 11] = [
        AssignmentField::RawTimephasedBaselineCost,
        AssignmentField::RawTimephasedBaseline1Cost,
        AssignmentField::RawTimephasedBaseline2Cost,
        AssignmentField::RawTimephasedBaseline3Cost,
        AssignmentField::RawTimephasedBaseline4Cost,
        AssignmentField::RawTimephasedBaseline5Cost,
        AssignmentField::RawTimephasedBaseline6Cost,
        AssignmentField::RawTimephasedBaseline7Cost,
        AssignmentField::RawTimephasedBaseline8Cost,
        AssignmentField::RawTimephasedBaseline9Cost,
        AssignmentField::RawTimephasedBaseline10Cost,
    ];

    usize::try_from(index).ok().and_then(|index| FIELDS.get(index).copied())
}

/// Get the "raw timephased baseline budget cost" field by index.
///
/// Corresponds to Java: `AssignmentFieldLists.RAW_TIMEPHASED_BASELINE_BUDGET_COSTS[index]`
fn raw_timephased_baseline_budget_cost_field(index: i32) -> Option<AssignmentField> {
    const FIELDS: [AssignmentField; 11] = [
        AssignmentField::RawTimephasedBaselineBudgetCost,
        AssignmentField::RawTimephasedBaseline1BudgetCost,
        AssignmentField::RawTimephasedBaseline2BudgetCost,
        AssignmentField::RawTimephasedBaseline3BudgetCost,
        AssignmentField::RawTimephasedBaseline4BudgetCost,
        AssignmentField::RawTimephasedBaseline5BudgetCost,
        AssignmentField::RawTimephasedBaseline6BudgetCost,
        AssignmentField::RawTimephasedBaseline7BudgetCost,
        AssignmentField::RawTimephasedBaseline8BudgetCost,
        AssignmentField::RawTimephasedBaseline9BudgetCost,
        AssignmentField::RawTimephasedBaseline10BudgetCost,
    ];

    usize::try_from(index).ok().and_then(|index| FIELDS.get(index).copied())
}

/// A duration list that is entirely `None`.
fn empty_durations(len: usize) -> Vec<Option<Duration>> {
    vec![None; len]
}

/// A numeric list that is entirely `None`.
fn empty_numbers(len: usize) -> Vec<Option<f64>> {
    vec![None; len]
}

/// Compare a range with an optional date.
///
/// Corresponds to Java's `LocalDateTimeRange.compareTo(LocalDateTime)`; returns `None` when the date is null.
fn range_compare_to_date(range: &LocalDateTimeRange, date: Option<NaiveDateTime>) -> Option<i32> {
    date.map(|date| range.compare_to_date(date))
}

/// Whether the range list falls entirely outside the assignment range.
///
/// Corresponds to Java: `ranges.get(ranges.size() - 1).isBefore(assignmentRange) || ranges.get(0).isAfter(assignmentRange)`
fn out_of_assignment_range(
    ranges: &[LocalDateTimeRange],
    assignment_range: &LocalDateTimeRange,
) -> bool {
    match (ranges.last(), ranges.first()) {
        (Some(last), Some(first)) => {
            last.is_before(assignment_range) || first.is_after(assignment_range)
        }
        _ => false,
    }
}

/// Find the index of the first range intersecting the target range; returns `ranges.len()` when there is none.
///
/// Corresponds to Java's `while (rangeIndex < ranges.size() && !ranges.get(rangeIndex).intersectsWith(...)) ++rangeIndex;`
fn first_intersecting_range(ranges: &[LocalDateTimeRange], range: &LocalDateTimeRange) -> usize {
    let mut index = 0;
    while index < ranges.len() && !ranges[index].intersects_with(range) {
        index += 1;
    }
    index
}

/// Get the work between two instants on the calendar; returns 0 when the calendar is missing or already borrowed (Java throws an NPE).
fn calendar_work(
    calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
    start: Option<NaiveDateTime>,
    finish: Option<NaiveDateTime>,
    units: TimeUnit,
) -> Duration {
    match (calendar, start, finish) {
        (Some(calendar), Some(start), Some(finish)) => match calendar.try_borrow() {
            Ok(calendar) => calendar.get_work(start, finish, units),
            Err(_) => Duration::get_instance(0.0, units),
        },
        _ => Duration::get_instance(0.0, units),
    }
}

/// Convert a duration using the default unit conversion provided by the calendar; returns the original value when the calendar is missing (Java throws an NPE).
fn convert_units(
    duration: Duration,
    units: TimeUnit,
    calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
) -> Duration {
    match calendar {
        Some(calendar) => match calendar.try_borrow() {
            Ok(calendar) => duration.convert_units(units, &*calendar),
            Err(_) => duration,
        },
        None => duration,
    }
}

/// Get the start time of the next rate row; uses `LocalDateTimeHelper.END_DATE_NA` when there is no next row.
///
/// Corresponds to Java: `private LocalDateTime getNextRateStart(List<CostRateTableEntry> rates, int costRateTableIndex)`
fn get_next_rate_start(rates: &[&CostRateTableEntry], index: usize) -> Option<NaiveDateTime> {
    if index + 1 < rates.len() {
        rates[index + 1].get_start_date()
    } else {
        Some(LocalDateTimeHelper::end_date_na())
    }
}

/// Whether this timephased work item covers non-working time.
///
/// Corresponds to Java: `TimephasedUtility.itemIsNonWorking(ProjectCalendar calendar, TimephasedItem<?> item)`
fn item_is_non_working(
    calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
    item: &TimephasedWork,
) -> bool {
    calendar_work(calendar, item.get_start(), item.get_finish(), TimeUnit::Hours).get_duration() == 0.0
}

/// Whether this timephased cost item covers non-working time.
///
/// Corresponds to Java: `TimephasedUtility.itemIsNonWorking(...)`
fn cost_item_is_non_working(
    calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
    item: &TimephasedCost,
) -> bool {
    calendar_work(calendar, item.get_start(), item.get_finish(), TimeUnit::Hours).get_duration() == 0.0
}

/// Split the raw timephased work by time ranges.
///
/// Corresponds to Java: `TimephasedUtility.segmentWork(ProjectCalendar calendar, List<TimephasedWork> work, List<LocalDateTimeRange> ranges, TimeUnit targetUnits)`
///
/// Java's argument validation (`validateRanges` / `validateTimephasedWork`) throws exceptions; the Rust side does not validate.
fn segment_work(
    calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
    work: &[TimephasedWork],
    ranges: &[LocalDateTimeRange],
    target_units: TimeUnit,
) -> Vec<Option<Duration>> {
    if work.is_empty() {
        return empty_durations(ranges.len());
    }

// Java: Microsoft Project special case -- when there is a single segment with identical start and end and a non-zero total, the work is included in the last range
    if work.len() == 1 {
        if let Some(item) = work.first() {
            let same_instant = matches!(
                (item.get_start(), item.get_finish()),
                (Some(start), Some(finish)) if start == finish
            );
            let total = item
                .get_total_amount()
                .map(|amount| amount.get_duration())
                .unwrap_or(0.0);
            if same_instant && total != 0.0 {
                let mut result = empty_durations(ranges.len());
                if let (Some(last), Some(total)) = (result.last_mut(), item.get_total_amount()) {
                    *last = Some(convert_units(total, target_units, calendar));
                }
                return result;
            }
        }
    }

// Java uses -1 to represent null and maps it uniformly at the end
    let mut result: Vec<Option<f64>> = vec![None; ranges.len()];

    let mut current_item_index = 0usize;
    let mut current_item = &work[0];
    let units = current_item
        .get_amount_per_hour()
        .map(|rate| rate.get_units())
        .unwrap_or(TimeUnit::Hours);
    let mut current_item_is_non_working = item_is_non_working(calendar, current_item);
    let mut current_item_work_per_hour = current_item
        .get_amount_per_hour()
        .map(|rate| rate.get_duration())
        .unwrap_or(0.0);

    let mut current_range_index: isize = 0;
    let mut current_range = ranges[0];

    loop {
        let (Some(item_start), Some(_item_finish)) =
            (current_item.get_start(), current_item.get_finish())
        else {
            break;
        };
        let (Some(range_start), Some(range_end)) = (current_range.get_start(), current_range.get_end())
        else {
            break;
        };

        if range_end <= item_start {
// Java: the range is earlier than the current timephased item -- that range has no work
            current_range_index += 1;
            if current_range_index as usize >= ranges.len() {
                break;
            }
            current_range = ranges[current_range_index as usize];
            continue;
        }

        // Java: while (!currentRange.getStart().isBefore(currentItem.getFinish()))
        let mut exhausted = false;
        loop {
            let Some(next_item_finish) = current_item.get_finish() else {
                exhausted = true;
                break;
            };
            if range_start < next_item_finish {
                break;
            }

            if current_item_index + 1 == work.len() {
                exhausted = true;
                break;
            }

            current_item_index += 1;
            current_item = &work[current_item_index];
            current_item_is_non_working = item_is_non_working(calendar, current_item);
            current_item_work_per_hour = current_item
                .get_amount_per_hour()
                .map(|rate| rate.get_duration())
                .unwrap_or(0.0);
        }

        if exhausted {
            current_range_index = -1;
        }

        if current_range_index == -1 {
            break;
        }

        let (Some(item_start), Some(item_finish)) =
            (current_item.get_start(), current_item.get_finish())
        else {
            break;
        };
        let (Some(range_start), Some(range_end)) = (current_range.get_start(), current_range.get_end())
        else {
            break;
        };

        let mut more_time_in_range = false;
        if range_end > item_start {
// Java: the range intersects this item
            let work_start = if range_start > item_start {
                range_start
            } else {
                item_start
            };
            let work_finish = if range_end > item_finish {
                item_finish
            } else {
                range_end
            };
            let actual_work_hours =
                calendar_work(calendar, Some(work_start), Some(work_finish), TimeUnit::Hours)
                    .get_duration();
            let calculated_work_hours = if current_item_is_non_working {
                (work_finish - work_start).num_minutes() as f64 / 60.0
            } else {
                actual_work_hours
            };

            if calculated_work_hours != 0.0 {
                let work_amount = current_item_work_per_hour * calculated_work_hours;
                if work_amount != 0.0 || actual_work_hours != 0.0 {
                    let range_index = current_range_index as usize;
                    let current_range_work = result[range_index].unwrap_or(0.0);
                    result[range_index] = Some(current_range_work + work_amount);
                }
            }

            more_time_in_range = work_finish < range_end;
            if more_time_in_range {
// Java: the current range still has remaining time to compute
                current_range = LocalDateTimeRange::new(Some(work_finish), Some(range_end));
            }
        }

        if !more_time_in_range {
// Java: the current range is finished, advance
            current_range_index += 1;
            if current_range_index as usize >= ranges.len() {
                break;
            }
            current_range = ranges[current_range_index as usize];
        }
    }

    result
        .into_iter()
        .map(|value| {
            value.map(|value| {
                convert_units(Duration::get_instance(value, units), target_units, calendar)
            })
        })
        .collect()
}

/// Split the raw timephased cost by time ranges.
///
/// Corresponds to Java: `TimephasedUtility.segmentCost(ProjectCalendar calendar, List<TimephasedCost> cost, List<LocalDateTimeRange> ranges)`
fn segment_cost(
    calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
    cost: &[TimephasedCost],
    ranges: &[LocalDateTimeRange],
) -> Vec<Option<f64>> {
    if cost.is_empty() {
        return empty_numbers(ranges.len());
    }

    let mut result: Vec<Option<f64>> = vec![None; ranges.len()];

    let mut current_item_index = 0usize;
    let mut current_item = &cost[0];
    let mut current_item_is_non_working = cost_item_is_non_working(calendar, current_item);
    let mut current_item_cost_per_hour = current_item.get_amount_per_hour().unwrap_or(0.0);

    let mut current_range_index: isize = 0;
    let mut current_range = ranges[0];

    loop {
        let (Some(item_start), Some(_item_finish)) =
            (current_item.get_start(), current_item.get_finish())
        else {
            break;
        };
        let (Some(range_start), Some(range_end)) = (current_range.get_start(), current_range.get_end())
        else {
            break;
        };

        if range_end <= item_start {
            current_range_index += 1;
            if current_range_index as usize >= ranges.len() {
                break;
            }
            current_range = ranges[current_range_index as usize];
            continue;
        }

        let mut exhausted = false;
        loop {
            let Some(next_item_finish) = current_item.get_finish() else {
                exhausted = true;
                break;
            };
            if range_start < next_item_finish {
                break;
            }

            if current_item_index + 1 == cost.len() {
                exhausted = true;
                break;
            }

            current_item_index += 1;
            current_item = &cost[current_item_index];
            current_item_is_non_working = cost_item_is_non_working(calendar, current_item);
            current_item_cost_per_hour = current_item.get_amount_per_hour().unwrap_or(0.0);
        }

        if exhausted {
            current_range_index = -1;
        }

        if current_range_index == -1 {
            break;
        }

        let (Some(item_start), Some(item_finish)) =
            (current_item.get_start(), current_item.get_finish())
        else {
            break;
        };
        let (Some(range_start), Some(range_end)) = (current_range.get_start(), current_range.get_end())
        else {
            break;
        };

// Java: the range intersects this item
        let work_start = if range_start > item_start {
            range_start
        } else {
            item_start
        };
        let work_finish = if range_end > item_finish {
            item_finish
        } else {
            range_end
        };
        let actual_work_hours =
            calendar_work(calendar, Some(work_start), Some(work_finish), TimeUnit::Hours)
                .get_duration();
        let calculated_work_hours = if current_item_is_non_working {
            (work_finish - work_start).num_minutes() as f64 / 60.0
        } else {
            actual_work_hours
        };

        if calculated_work_hours != 0.0 {
            let cost_amount = current_item_cost_per_hour * calculated_work_hours;
            if cost_amount != 0.0 || actual_work_hours != 0.0 {
                let range_index = current_range_index as usize;
                let current_range_cost = result[range_index].unwrap_or(0.0);
                result[range_index] = Some(current_range_cost + cost_amount);
            }
        }

        if work_finish < range_end {
            current_range = LocalDateTimeRange::new(Some(work_finish), Some(range_end));
        } else {
            current_range_index += 1;
            if current_range_index as usize >= ranges.len() {
                break;
            }
            current_range = ranges[current_range_index as usize];
        }
    }

    result
}

/// Split the raw timephased work by time ranges into material quantities.
///
/// Corresponds to Java: `TimephasedUtility.segmentMaterial(ResourceAssignment assignment, ProjectCalendar calendar, List<TimephasedWork> work, List<LocalDateTimeRange> ranges)`
///
/// The Rust side uses the `is_material` boolean parameter to express Java's "the resource is null or its type is not MATERIAL" check.
fn segment_material(
    is_material: bool,
    calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
    work: &[TimephasedWork],
    ranges: &[LocalDateTimeRange],
) -> Vec<Option<f64>> {
    if !is_material {
        return empty_numbers(ranges.len());
    }

    segment_work(calendar, work, ranges, TimeUnit::Hours)
        .into_iter()
        .map(|duration| duration.map(|duration| duration.get_duration()))
        .collect()
}

/// Add two timephased duration lists element by element.
///
/// Corresponds to Java: `TimephasedUtility.addTimephasedDurations(List<Duration> list1, List<Duration> list2)`
///
/// Java throws `RuntimeException` when the lengths differ; the Rust side returns the first list unchanged.
fn add_timephased_durations(
    list1: Vec<Option<Duration>>,
    list2: Vec<Option<Duration>>,
) -> Vec<Option<Duration>> {
    if list1.len() != list2.len() {
        return list1;
    }

    list1
        .into_iter()
        .zip(list2)
        .map(|(value1, value2)| match (value1, value2) {
            (None, None) => None,
            (value1, value2) => {
                let units = value1
                    .or(value2)
                    .map(|value| value.get_units())
                    .unwrap_or(TimeUnit::Hours);
                let total = NumberHelper::get_double(value1.map(|value| value.get_duration()))
                    + NumberHelper::get_double(value2.map(|value| value.get_duration()));
                Some(Duration::get_instance(total, units))
            }
        })
        .collect()
}

/// Add two timephased numeric lists element by element.
///
/// Corresponds to Java: `TimephasedUtility.addTimephasedNumbers(List<Number> list1, List<Number> list2)`
fn add_timephased_numbers(list1: Vec<Option<f64>>, list2: Vec<Option<f64>>) -> Vec<Option<f64>> {
    if list1.len() != list2.len() {
        return list1;
    }

    list1
        .into_iter()
        .zip(list2)
        .map(|(value1, value2)| match (value1, value2) {
            (None, None) => None,
            (value1, value2) => {
                Some(NumberHelper::get_double(value1) + NumberHelper::get_double(value2))
            }
        })
        .collect()
}

/// Render an optional value with Java's `null` semantics.
fn display_option<T: Display>(value: Option<&T>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

/// Construct an unwired assignment (for field read/write tests).
    fn assignment() -> ResourceAssignment {
        ResourceAssignment::new()
    }

    #[test]
    fn field_round_trip() {
        let mut assignment = assignment();

        assignment.set_unique_id(Some(7));
        assignment.set_task_unique_id(Some(3));
        assignment.set_resource_unique_id(Some(5));
        assignment.set_units(Some(100.0));
        assignment.set_work(Some(Duration::get_instance(16.0, TimeUnit::Hours)));
        assignment.set_cost(Some(1_000.0));
        assignment.set_start(Some(NaiveDateTime::default()));

        assert_eq!(assignment.get_unique_id(), Some(7));
        assert_eq!(assignment.get_task_unique_id(), Some(3));
        assert_eq!(assignment.get_resource_unique_id(), Some(5));
        assert_eq!(assignment.get_units(), Some(100.0));
        assert_eq!(
            assignment.get_work(),
            Some(Duration::get_instance(16.0, TimeUnit::Hours))
        );
        assert_eq!(assignment.get_cost(), Some(1_000.0));

// The unique ID can also be read and written via the FieldContainer interface
        {
            let container: &mut dyn FieldContainer = &mut assignment;
            container.set(
                FieldKey::Assignment(AssignmentField::UniqueId),
                Some(FieldValue::Integer(9)),
            );
            assert_eq!(
                container.get_cached_value(FieldKey::Assignment(AssignmentField::UniqueId)),
                Some(&FieldValue::Integer(9))
            );
            assert_eq!(
                container.get(FieldKey::Assignment(AssignmentField::UniqueId)),
                Some(FieldValue::Integer(9))
            );
        }
        assert_eq!(assignment.get_unique_id(), Some(9));

// Passing None is equivalent to passing null in Java (removes the field)
        assignment.set_units(None);
        assert_eq!(assignment.get_units(), None);
    }

    #[test]
    fn boolean_and_flag_fields_use_java_defaults() {
        let mut assignment = assignment();

// Java: the default value of getCalculateCostsFromUnits() is true
        assert!(assignment.get_calculate_costs_from_units());
// Java: unset boolean fields default to false
        assert!(!assignment.get_confirmed());

        assignment.set_confirmed(true);
        assignment.set_flag(1, true);
        assignment.set_enterprise_flag(2, true);

        assert!(assignment.get_confirmed());
        assert!(assignment.get_flag(1));
        assert!(assignment.get_enterprise_flag(2));
        assert!(!assignment.get_flag(2));
    }

    #[test]
    fn indexed_accessors_select_the_right_field() {
        let mut assignment = assignment();

        assignment.set_baseline_work_by_index(1, Some(Duration::get_instance(8.0, TimeUnit::Hours)));
        assignment
            .set_baseline_work_by_index(10, Some(Duration::get_instance(9.0, TimeUnit::Hours)));
        assignment.set_baseline_cost_by_index(3, Some(300.0));
        assignment.set_text(2, Some("second".to_owned()));
        assignment.set_number(20, Some(20.0));
        assignment.set_date(1, Some(NaiveDateTime::default()));

        assert_eq!(
            assignment.get_baseline_work_by_index(1),
            Some(Duration::get_instance(8.0, TimeUnit::Hours))
        );
        assert_eq!(
            assignment.get_baseline_work_by_index(10),
            Some(Duration::get_instance(9.0, TimeUnit::Hours))
        );
        assert_eq!(assignment.get_baseline_cost_by_index(3), Some(300.0));
        assert_eq!(assignment.get_text(2), Some("second".to_owned()));
        assert_eq!(assignment.get_number(20), Some(20.0));
        assert!(assignment.get_date(1).is_some());

// Java throws IllegalArgumentException when out of bounds; the Rust side returns None / is a no-op
        assert_eq!(assignment.get_baseline_work_by_index(0), None);
        assert_eq!(assignment.get_baseline_work_by_index(11), None);
        assert_eq!(assignment.get_text(31), None);
        assignment.set_text(31, Some("ignored".to_owned()));
    }

    #[test]
    fn calculated_fields_use_stored_values() {
        let mut assignment = assignment();

        assignment.set_units(Some(100.0));
        assignment.set_actual_overtime_cost(Some(10.0));
        assignment.set_remaining_overtime_cost(Some(5.0));
        assignment.set_cost(Some(100.0));
        assignment.set_baseline_cost(Some(80.0));
        assignment.set_bcwp(Some(50.0));
        assignment.set_bcws(Some(30.0));
        assignment.set_acwp(Some(20.0));

        // Java: OVERTIME_COST = ACTUAL_OVERTIME_COST + REMAINING_OVERTIME_COST
        assert_eq!(assignment.get_overtime_cost(), Some(15.0));
        // Java: COST_VARIANCE = COST - BASELINE_COST
        assert_eq!(assignment.get_cost_variance(), Some(20.0));
        // Java: SV = BCWP - BCWS
        assert_eq!(assignment.get_sv(), Some(20.0));
        // Java: CV = BCWP - ACWP
        assert_eq!(assignment.get_cv(), Some(30.0));
// Java: when REMAINING_ASSIGNMENT_UNITS has no value, it degrades gracefully to the planned units
        assert_eq!(assignment.get_remaining_units(), Some(100.0));
        assert_eq!(assignment.get_units(), Some(100.0));
// Java: RATE_INDEX defaults to 0
        assert_eq!(assignment.get_rate_index(), Some(0));
// Java: RATE_SOURCE defaults to Resource (provided by the calculated field defaultRateSource)
        assert_eq!(assignment.get_rate_source(), RateSource::Resource);
// Java: COST_RATE_TABLE yields 0 when out of bounds
        assert_eq!(assignment.get_cost_rate_table_index(), 0);
        assignment.set_cost_rate_table_index(7);
        assert_eq!(assignment.get_cost_rate_table_index(), 0);
        assignment.set_cost_rate_table_index(2);
        assert_eq!(assignment.get_cost_rate_table_index(), 2);
// There is no rate table when there is no resource/role
        assert!(assignment.with_cost_rate_table(|table| table.is_none()));
    }

    #[test]
    fn dependency_map_clears_cached_calculated_values() {
        let mut assignment = assignment();

// Java: COST_VARIANCE depends on COST and BASELINE_COST, so its cache should be cleared after modification
        assignment
            .set_assignment_value(AssignmentField::CostVariance, Some(FieldValue::Double(1.0)));
        assert_eq!(assignment.get_cost_variance(), Some(1.0));

        assignment.set_cost(Some(10.0));
        assert_eq!(
            assignment.peek_cached_value(FieldKey::Assignment(AssignmentField::CostVariance)),
            None
        );
// After the cache is cleared, it returns the calculated value (BASELINE_COST empty -> no value)
        assert_eq!(assignment.get_cost_variance(), None);
    }

    #[test]
    fn timephased_segment_work_maps_ranges_without_calendar() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1)
            .and_then(|date| date.and_hms_opt(9, 0, 0))
            .expect("valid date");
        let finish = start + chrono::Duration::hours(8);

        let mut item = TimephasedWork::new();
        item.set_start(Some(start));
        item.set_finish(Some(finish));
        item.set_total_amount(Some(Duration::get_instance(8.0, TimeUnit::Hours)));
        item.set_amount_per_hour(Some(Duration::get_instance(1.0, TimeUnit::Hours)));

        let ranges = [LocalDateTimeRange::new(
            Some(start),
            Some(start + chrono::Duration::hours(4)),
        )];

// When there is no calendar, split by time proportion (actual work is counted in hours)
        let result = segment_work(None, &[item], &ranges, TimeUnit::Hours);
        assert_eq!(result.len(), 1);
        assert!(result[0].is_some());
    }

    #[test]
    fn timephased_add_helpers_merge_lists() {
        let hours = TimeUnit::Hours;
        let list1 = vec![Some(Duration::get_instance(1.0, hours)), None];
        let list2 = vec![Some(Duration::get_instance(2.0, hours)), None];

        let merged = add_timephased_durations(list1, list2);
        assert_eq!(merged[0], Some(Duration::get_instance(3.0, hours)));
        assert_eq!(merged[1], None);

        let numbers = add_timephased_numbers(vec![Some(1.0), None], vec![Some(2.5), None]);
        assert_eq!(numbers, vec![Some(3.5), None]);

// When the lengths differ, no longer add (Java throws RuntimeException)
        let mismatched = add_timephased_numbers(vec![Some(1.0)], vec![Some(2.0), Some(3.0)]);
        assert_eq!(mismatched, vec![Some(1.0)]);
    }

    #[test]
    fn raw_timephased_accessors_round_trip() {
        let mut assignment = assignment();

        let work = vec![{
            let mut item = TimephasedWork::new();
            item.set_amount_per_hour(Some(Duration::get_instance(1.0, TimeUnit::Hours)));
            item
        }];

        assignment.set_raw_timephased_planned_work(work.clone());
        assert_eq!(assignment.get_raw_timephased_planned_work().len(), 1);

        assignment.set_raw_timephased_baseline_work(1, work);
        assert_eq!(assignment.get_raw_timephased_baseline_work(1).len(), 1);
        assert!(assignment.get_raw_timephased_baseline_work(11).is_empty());

// When not set, returns the default empty list
        assert!(assignment.get_raw_timephased_budget_work().is_empty());
        assert!(assignment.get_raw_timephased_remaining_overtime_work().is_empty());
    }

    #[test]
    fn notes_and_guid_use_field_table() {
        let mut assignment = assignment();

        assignment.set_notes(Some("hello".to_owned()));
        assert_eq!(assignment.get_notes(), "hello");
        assert!(assignment.get_notes_object().is_some());
// Java's get(AssignmentField.NOTES) retrieves exactly this object
        assert_eq!(
            assignment
                .get_field(FieldKey::Assignment(AssignmentField::Notes))
                .map(|value| value.type_name()),
            Some("Notes")
        );

        assignment.set_notes(None);
        assert_eq!(assignment.get_notes(), "");
        assert!(assignment.get_field(FieldKey::Assignment(AssignmentField::Notes)).is_none());

        let guid = Uuid::nil();
        assignment.set_guid(Some(guid));
        assert_eq!(assignment.get_guid(), Some(guid));
        assert!(matches!(
            assignment.get_field(FieldKey::Assignment(AssignmentField::Guid)),
            Some(FieldValue::Uuid(value)) if value == guid
        ));

        assignment.set_work_contour(Some(WorkContour::flat()));
        assert_eq!(
            assignment
                .get_work_contour()
                .and_then(|contour| contour.get_name().map(str::to_owned)),
            Some("FLAT".to_owned())
        );
        assert!(matches!(
            assignment.get_field(FieldKey::Assignment(AssignmentField::WorkContour)),
            Some(FieldValue::WorkContour(_))
        ));

        assignment.set_variable_rate_units(Some(TimeUnit::Hours));
        assert_eq!(assignment.get_variable_rate_units(), Some(TimeUnit::Hours));
        assert!(matches!(
            assignment.get_field(FieldKey::Assignment(AssignmentField::VariableRateUnits)),
            Some(FieldValue::TimeUnit(TimeUnit::Hours))
        ));

        assignment.set_resource_request_type(Some(ResourceRequestType::None));
        assert_eq!(
            assignment.get_resource_request_type(),
            Some(ResourceRequestType::None)
        );
        assert!(matches!(
            assignment.get_field(FieldKey::Assignment(AssignmentField::ResourceRequestType)),
            Some(FieldValue::ResourceRequestType(ResourceRequestType::None))
        ));

// When RATE_SOURCE is not explicitly set, the default value is provided by the calculated field (Java's defaultRateSource)
        assert_eq!(
            assignment
                .get_field(FieldKey::Assignment(AssignmentField::RateSource))
                .and_then(|value| value.as_rate_source()),
            Some(RateSource::Resource)
        );
        assignment.set_rate_source(Some(RateSource::Override));
        assert_eq!(assignment.get_rate_source(), RateSource::Override);
        assert!(assignment
            .peek_cached_value(FieldKey::Assignment(AssignmentField::RateSource))
            .is_some());
    }

    #[test]
    fn raw_timephased_values_are_visible_in_field_table() {
        let mut assignment = assignment();

        let mut item = TimephasedWork::new();
        item.set_amount_per_hour(Some(Duration::get_instance(1.0, TimeUnit::Hours)));
        assignment.set_raw_timephased_planned_work(vec![item]);

// Java's get(AssignmentField.RAW_TIMEPHASED_PLANNED_WORK) retrieves exactly this list
        assert!(matches!(
            assignment.peek_cached_value(FieldKey::Assignment(
                AssignmentField::RawTimephasedPlannedWork
            )),
            Some(FieldValue::TimephasedWork(work)) if work.len() == 1
        ));

// Unset timephased fields fall back to an empty list per Java's defaultTimephasedWork / defaultTimephasedCost
        assert!(matches!(
            assignment.get_field(FieldKey::Assignment(
                AssignmentField::RawTimephasedBudgetWork
            )),
            Some(FieldValue::TimephasedWork(work)) if work.is_empty()
        ));
        assert!(matches!(
            assignment.get_field(FieldKey::Assignment(
                AssignmentField::RawTimephasedBaselineBudgetCost
            )),
            Some(FieldValue::TimephasedCost(cost)) if cost.is_empty()
        ));
    }

/// Query timephased data via the [`FieldContainer`] trait entry point (Java's
    /// `getTimephasedDurationValues` / `getTimephasedNumericValues`).
    #[test]
    fn timephased_queries_via_field_container_trait() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1)
            .and_then(|date| date.and_hms_opt(9, 0, 0))
            .expect("valid date");
        let finish = start + chrono::Duration::hours(8);

        let ranges = [LocalDateTimeRange::new(
            Some(start),
            Some(start + chrono::Duration::hours(4)),
        )];

        let mut assignment = assignment();
        let mut work = TimephasedWork::new();
        work.set_start(Some(start));
        work.set_finish(Some(finish));
        work.set_total_amount(Some(Duration::get_instance(8.0, TimeUnit::Hours)));
        work.set_amount_per_hour(Some(Duration::get_instance(1.0, TimeUnit::Hours)));
        assignment.set_raw_timephased_planned_work(vec![work]);

        let durations = FieldContainer::get_timephased_duration_values(
            &assignment,
            FieldKey::Assignment(AssignmentField::PlannedWork),
            &ranges,
            TimeUnit::Hours,
        );
        assert_eq!(durations.len(), 1);
        assert!(durations[0].is_some());

        let mut cost = TimephasedCost::new();
        cost.set_start(Some(start));
        cost.set_finish(Some(finish));
        cost.set_total_amount(Some(100.0));
        cost.set_amount_per_hour(Some(12.5));
        assignment.set_raw_timephased_baseline_cost(0, vec![cost]);

        let costs = FieldContainer::get_timephased_numeric_values(
            &assignment,
            FieldKey::Assignment(AssignmentField::BaselineCost),
            &ranges,
        );
        assert_eq!(costs.len(), 1);
        assert!(costs[0].is_some());

// When the field is not in the dispatch table, returns an all-None list consistent with Java
        let unsupported = FieldContainer::get_timephased_numeric_values(
            &assignment,
            FieldKey::Assignment(AssignmentField::TaskUniqueId),
            &ranges,
        );
        assert_eq!(unsupported, vec![None]);
    }

    #[test]
    fn workgroup_assignment_round_trip() {
        let mut assignment = assignment();
        assert!(assignment.get_workgroup_assignment().is_none());

        assignment
            .add_workgroup_assignment()
            .set_message_unique_id(Some("MSG-1".to_owned()));
// Java throws MPXJException when a record already exists; the Rust side returns the existing record
        assignment
            .add_workgroup_assignment()
            .set_schedule_id(Some("SCHED-9".to_owned()));

        let workgroup = assignment
            .get_workgroup_assignment()
            .expect("workgroup field created");
        assert_eq!(workgroup.get_message_unique_id(), Some("MSG-1"));
        assert_eq!(workgroup.get_schedule_id(), Some("SCHED-9"));
    }

    #[test]
    fn display_matches_java_layout() {
        let assignment = assignment();
        let text = assignment.to_string();

        assert!(text.starts_with("[Resource Assignment task=null resource=Unassigned"));
        assert!(text.contains("start=null"));
        assert!(text.contains("workContour=null"));
    }

    #[test]
    fn out_of_assignment_range_detects_disjoint_ranges() {
        let date = |day: u32| {
            NaiveDate::from_ymd_opt(2024, 1, day)
                .and_then(|date| date.and_hms_opt(0, 0, 0))
                .expect("valid date")
        };

        let assignment_range = LocalDateTimeRange::new(Some(date(10)), Some(date(20)));
        assert!(!out_of_assignment_range(
            &[LocalDateTimeRange::new(Some(date(12)), Some(date(13)))],
            &assignment_range
        ));
        assert!(out_of_assignment_range(
            &[LocalDateTimeRange::new(Some(date(1)), Some(date(2)))],
            &assignment_range
        ));
        assert!(out_of_assignment_range(
            &[LocalDateTimeRange::new(Some(date(25)), Some(date(26)))],
            &assignment_range
        ));
    }
}
