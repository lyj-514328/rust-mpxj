//! Corresponds to Java: `org/mpxj/Resource.java`
//!
//! # Differences from Java
//!
//! - **Field storage (Java inheritance → Rust composition)**: Java's `Resource extends AbstractFieldContainer<Resource>`,
//!   `set` / `get` / `getCachedValue` are provided by the base class; Rust has no inheritance, so this class holds an
//!   [`AbstractFieldContainer`], forwarded by [`Resource::set`] / [`Resource::get`] /
//!   [`Resource::get_cached_value`] (see the notes in [`crate::abstract_field_container`]).
//!   Where the three Java abstract methods land in Rust: `handleFieldChange` →
//!   [`Resource::handle_field_change`];`getAlwaysCalculatedField` →
//!   [`is_always_calculated`]; `getCalculationMethod` → the `match` in [`Resource::calculate`].
//! - **Values that `FieldValue` cannot express**: [`crate::field_value::FieldValue`] currently only has scalar variants
//!   (no enum / `Notes` / `UUID` / collection variants), so the following Java fields use
//!   equivalent scalar encodings in the field container, with typed accessors handling conversion:
//!
//!   | Java field | Rust encoding |
//!   |---|---|
//!   | `ResourceField.TYPE` (`ResourceType`) | `FieldValue::Integer` (declaration-order index, `WORK` = 1) |
//!   | `ResourceField.ACCRUE_AT`(`AccrueType`) | `FieldValue::Integer`(`getValue()`) |
//!   | `ResourceField.WORKGROUP`(`WorkGroup`) | `FieldValue::Integer`(`getValue()`) |
//!   | `ResourceField.BOOKING_TYPE`(`BookingType`) | `FieldValue::Integer`(`getValue()`) |
//!   | `ResourceField.NOTES`(`Notes`) | `FieldValue::String`(`Notes.toString()`) |
//!   | `ResourceField.GUID`(`UUID`) | `FieldValue::String`(`UUID.toString()`) |
//!
//!   Therefore [`Resource::get_notes_object`] can only return plain-text [`PlainNotes`],
//!   and [`Resource::set_notes_object`] can only keep plain text; `get(GUID)` returns a string rather than a `Uuid`.
//!   `ResourceField.RESOURCE_CODE_VALUES` / `ROLE_CODE_VALUES`(`Map<ResourceCode, ...>`)
//!   No corresponding type exists, so it is not ported (see below).
//! - **Constructor and automatic numbering**: Java's `Resource(ProjectContext)` assigns
//!   unique ID and ID in the constructor according to configuration. On the Rust side `&mut self` cannot obtain its own `Rc` (`mapID` / `updateUniqueID`
//!   both need an element handle), and container methods are always called while the context is `borrow_mut`ed. Therefore on the Rust side
//!   [`Resource::new`] only initializes fields, and automatic numbering is done by [`crate::resource_container::ResourceContainer::add`]
//!   using the container's own sequence generator (see the notes in that file).
//! - **Back references**: Java's `private final ProjectContext m_context` is a strong reference; the Rust side uses
//!   `Weak<RefCell<ProjectContext>>` instead. There is also a new Rust-specific field `self_ref`
//!   (`Weak<RefCell<Resource>>`), which lets `handleFieldChange` call back into the container when the unique ID / ID changes
//!   (Java uses `this`); the container wires it up via `set_self_reference` after wrapping the `Rc`.
//! - **Collections**: `List<Resource>` → `Vec<Rc<RefCell<Resource>>>`;
//!   `Map<Resource, SkillLevel>` → `HashMap<i32, (Weak<RefCell<Resource>>, SkillLevel)>`
//!   (the key is `NumberHelper.getInt(role.getID())`, equivalent to Java using ID for `equals`/`hashCode`);
//!   [`Resource::get_role_assignments`] returns a `Vec<(Rc<RefCell<Resource>>, SkillLevel)>` snapshot.
//! - **AvailabilityTable**: Java directly holds a mutable object; the Rust side wraps it in a `RefCell<AvailabilityTable>`
//!   layer (`availableFrom` / `availableTo` need `&mut self`), so
//!   [`Resource::get_availability`] returns `Ref<'_, AvailabilityTable>`.
//! - **`get` does not write back to the cache**: Java's `get` `set`s the computed result of non-"always calculated" fields back into the field table;
//!   Rust's `get(&self)` cannot write back, so it recomputes each time (same result, just one fewer cache write).
//! - **Indexed overloads**: Rust has no overloading, so all Java methods that get/set by index (such as `getCost(int)`,
//!   `setBaselineWork(int, Duration)`) are given an `_at` suffix (such as [`Resource::get_cost_at`],
//!   [`Resource::set_baseline_work_at`]).
//! - **`selectField`**: Java throws `IllegalArgumentException` when out of bounds; the Rust side returns `None`,
//!   and the corresponding accessor returns `None` / keeps the original value (without panicking).
//! - **`compareTo`/`equals`/`hashCode`/`toString`** → `Ord`/`PartialEq`+`Eq`/`Hash`/`Display`.
//!
//! # Skipped Java Methods (and reasons)
//!
//! | Java | Reason |
//! |---|---|
//! | `getTimephasedDurationValues` / `getTimephasedNumericValues` | Provided by the default implementation of [`crate::field_container::FieldContainer`] (equivalent to Java's fallback when "the field is not in the `TIMEPHASED_*_FUNCTIONS` tables": an all-`None` list), because the timephased family in the next entry is not yet ported |
//! | `getTimephasedPlannedWork` and all other `getTimephased*`, `reduceTimephasedWork` / `reduceTimephasedCost` / `reduceTimephasedMaterial`, `TIMEPHASED_WORK_FUNCTIONS` / `TIMEPHASED_NUMERIC_FUNCTIONS` | `org.mpxj.TimephasedUtility` is not a separate module (`addTimephasedDurations` / `addTimephasedNumbers` are currently inlined in private functions in [`crate::resource_assignment`] and are not yet reusable); the timephased API of [`crate::resource_assignment::ResourceAssignment`] has landed |
//! | `getLocation` / `setLocation` | `Location` is P6/Primavera-specific data, and this project is only an MPP/MPX reader, so it is not ported |
//! | `getCurrency` / `setCurrency` | `Currency` is P6/Primavera-specific data (same as above) |
//! | `getResourceCodeValues` / `addResourceCodeValue` / `getRoleCodeValues` / `addRoleCodeValue` | `ResourceCode` / `RoleCode` / `ResourceCodeValue` / `RoleCodeValue` are P6/Primavera-specific data (same as above), and `FieldValue` has no collection variant either |
//!
//! Skipped Java mechanisms: reflection, `Serializable` (unused by this class).
//!
//! # Interface assumptions about related modules
//!
//! - `crate::resource_assignment::ResourceAssignment`:`get_start()` / `get_finish()` →
//!   `Option<NaiveDateTime>`,`get_planned_work()` → `Option<Duration>`,
//!   `get_planned_cost()` → `Option<f64>`,`set_resource_unique_id(Option<i32>)`.
//! - `crate::resource_assignment_container::ResourceAssignmentContainer`:
//!   `get_by_resource_unique_id(Option<i32>) -> Vec<Rc<RefCell<ResourceAssignment>>>`.
//! - `crate::project_file::ProjectFile`:`get_resource_assignments() -> &ResourceAssignmentContainer`.

use std::cell::{Ref, RefCell};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::{Rc, Weak};
use std::sync::OnceLock;

use chrono::{Local, NaiveDateTime};
use uuid::Uuid;

use crate::abstract_field_container::AbstractFieldContainer;
use crate::accrue_type::AccrueType;
use crate::availability::Availability;
use crate::availability_table::AvailabilityTable;
use crate::booking_type::BookingType;
use crate::common::boolean_helper::BooleanHelper;
use crate::common::number_helper::NumberHelper;
use crate::cost_rate_table::CostRateTable;
use crate::cost_rate_table_entry::CostRateTableEntry;
use crate::duration::Duration;
use crate::field_container::FieldContainer;
use crate::field_key::FieldKey;
use crate::field_value::FieldValue;
use crate::listener::field_listener::FieldListener;
use crate::mpxj_enum::MpxjEnum;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::notes::{Notes, PlainNotes};
use crate::project_calendar::ProjectCalendar;
use crate::project_context::ProjectContext;
use crate::project_entity_with_id::ProjectEntityWithID;
use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
use crate::rate::Rate;
use crate::resource_assignment::ResourceAssignment;
use crate::resource_field::ResourceField;
use crate::resource_type::ResourceType;
use crate::shift::Shift;
use crate::skill_level::SkillLevel;
use crate::unit_of_measure::UnitOfMeasure;
// Note: `TimeUnit` is only named explicitly in tests; in the main code the unit type is inferred from `Duration`.
use crate::work_group::WorkGroup;

/// Number of cost rate tables (Corresponds to Java: `CostRateTable.MAX_TABLES`).
const MAX_TABLES: usize = 5;

/// Default resource calendar name used in `Resource.addCalendar`.
///
/// Corresponds to Java: `"Unnamed Resource"`.
const UNNAMED_RESOURCE: &str = "Unnamed Resource";

/// Default base calendar name.
///
/// Corresponds to Java: the literal `"Standard"` in `Resource.setBaseCalendar`.
const DEFAULT_BASE_CALENDAR: &str = "Standard";

/// Default value of default availability (100%).
///
/// Corresponds to Java: `private static final Number DEFAULT_DEFAULT_UNITS = NumberHelper.DOUBLE_ONEHUNDRED;`
const DEFAULT_DEFAULT_UNITS: f64 = NumberHelper::DOUBLE_ONEHUNDRED;

/// Wraps a `ResourceField` into a unified [`FieldKey`].
fn key(field: ResourceField) -> FieldKey {
    FieldKey::Resource(field)
}

/// Corresponds to Java: `LocalDateTime.now()`.
fn now() -> NaiveDateTime {
    Local::now().naive_local()
}

/// Corresponds to Java: `Resource.selectField(ResourceField[] fields, int index)`.
///
/// Java throws `IllegalArgumentException` when out of bounds; the Rust side returns `None`.
fn select_field(fields: &[ResourceField], index: i32) -> Option<ResourceField> {
    if index < 1 || index as usize > fields.len() {
        return None;
    }
    fields.get(index as usize - 1).copied()
}

/// Corresponds to Java: `ResourceFieldLists.CUSTOM_TEXT` (indices 1-30).
const CUSTOM_TEXT: [ResourceField; 30] = [
    ResourceField::Text1,
    ResourceField::Text2,
    ResourceField::Text3,
    ResourceField::Text4,
    ResourceField::Text5,
    ResourceField::Text6,
    ResourceField::Text7,
    ResourceField::Text8,
    ResourceField::Text9,
    ResourceField::Text10,
    ResourceField::Text11,
    ResourceField::Text12,
    ResourceField::Text13,
    ResourceField::Text14,
    ResourceField::Text15,
    ResourceField::Text16,
    ResourceField::Text17,
    ResourceField::Text18,
    ResourceField::Text19,
    ResourceField::Text20,
    ResourceField::Text21,
    ResourceField::Text22,
    ResourceField::Text23,
    ResourceField::Text24,
    ResourceField::Text25,
    ResourceField::Text26,
    ResourceField::Text27,
    ResourceField::Text28,
    ResourceField::Text29,
    ResourceField::Text30,
];

/// Corresponds to Java: `ResourceFieldLists.CUSTOM_START` (indices 1-10).
const CUSTOM_START: [ResourceField; 10] = [
    ResourceField::Start1,
    ResourceField::Start2,
    ResourceField::Start3,
    ResourceField::Start4,
    ResourceField::Start5,
    ResourceField::Start6,
    ResourceField::Start7,
    ResourceField::Start8,
    ResourceField::Start9,
    ResourceField::Start10,
];

/// Corresponds to Java: `ResourceFieldLists.CUSTOM_FINISH` (indices 1-10).
const CUSTOM_FINISH: [ResourceField; 10] = [
    ResourceField::Finish1,
    ResourceField::Finish2,
    ResourceField::Finish3,
    ResourceField::Finish4,
    ResourceField::Finish5,
    ResourceField::Finish6,
    ResourceField::Finish7,
    ResourceField::Finish8,
    ResourceField::Finish9,
    ResourceField::Finish10,
];

/// Corresponds to Java: `ResourceFieldLists.CUSTOM_NUMBER` (indices 1-20).
const CUSTOM_NUMBER: [ResourceField; 20] = [
    ResourceField::Number1,
    ResourceField::Number2,
    ResourceField::Number3,
    ResourceField::Number4,
    ResourceField::Number5,
    ResourceField::Number6,
    ResourceField::Number7,
    ResourceField::Number8,
    ResourceField::Number9,
    ResourceField::Number10,
    ResourceField::Number11,
    ResourceField::Number12,
    ResourceField::Number13,
    ResourceField::Number14,
    ResourceField::Number15,
    ResourceField::Number16,
    ResourceField::Number17,
    ResourceField::Number18,
    ResourceField::Number19,
    ResourceField::Number20,
];

/// Corresponds to Java: `ResourceFieldLists.CUSTOM_DURATION` (indices 1-10).
const CUSTOM_DURATION: [ResourceField; 10] = [
    ResourceField::Duration1,
    ResourceField::Duration2,
    ResourceField::Duration3,
    ResourceField::Duration4,
    ResourceField::Duration5,
    ResourceField::Duration6,
    ResourceField::Duration7,
    ResourceField::Duration8,
    ResourceField::Duration9,
    ResourceField::Duration10,
];

/// Corresponds to Java: `ResourceFieldLists.CUSTOM_DATE` (indices 1-10).
const CUSTOM_DATE: [ResourceField; 10] = [
    ResourceField::Date1,
    ResourceField::Date2,
    ResourceField::Date3,
    ResourceField::Date4,
    ResourceField::Date5,
    ResourceField::Date6,
    ResourceField::Date7,
    ResourceField::Date8,
    ResourceField::Date9,
    ResourceField::Date10,
];

/// Corresponds to Java: `ResourceFieldLists.CUSTOM_COST` (indices 1-10).
const CUSTOM_COST: [ResourceField; 10] = [
    ResourceField::Cost1,
    ResourceField::Cost2,
    ResourceField::Cost3,
    ResourceField::Cost4,
    ResourceField::Cost5,
    ResourceField::Cost6,
    ResourceField::Cost7,
    ResourceField::Cost8,
    ResourceField::Cost9,
    ResourceField::Cost10,
];

/// Corresponds to Java: `ResourceFieldLists.CUSTOM_FLAG` (indices 1-20).
const CUSTOM_FLAG: [ResourceField; 20] = [
    ResourceField::Flag1,
    ResourceField::Flag2,
    ResourceField::Flag3,
    ResourceField::Flag4,
    ResourceField::Flag5,
    ResourceField::Flag6,
    ResourceField::Flag7,
    ResourceField::Flag8,
    ResourceField::Flag9,
    ResourceField::Flag10,
    ResourceField::Flag11,
    ResourceField::Flag12,
    ResourceField::Flag13,
    ResourceField::Flag14,
    ResourceField::Flag15,
    ResourceField::Flag16,
    ResourceField::Flag17,
    ResourceField::Flag18,
    ResourceField::Flag19,
    ResourceField::Flag20,
];

/// Corresponds to Java: `ResourceFieldLists.CUSTOM_OUTLINE_CODE` (indices 1-10).
const CUSTOM_OUTLINE_CODE: [ResourceField; 10] = [
    ResourceField::OutlineCode1,
    ResourceField::OutlineCode2,
    ResourceField::OutlineCode3,
    ResourceField::OutlineCode4,
    ResourceField::OutlineCode5,
    ResourceField::OutlineCode6,
    ResourceField::OutlineCode7,
    ResourceField::OutlineCode8,
    ResourceField::OutlineCode9,
    ResourceField::OutlineCode10,
];

/// Corresponds to Java: `ResourceFieldLists.ENTERPRISE_CUSTOM_COST` (indices 1-10).
const ENTERPRISE_CUSTOM_COST: [ResourceField; 10] = [
    ResourceField::EnterpriseCost1,
    ResourceField::EnterpriseCost2,
    ResourceField::EnterpriseCost3,
    ResourceField::EnterpriseCost4,
    ResourceField::EnterpriseCost5,
    ResourceField::EnterpriseCost6,
    ResourceField::EnterpriseCost7,
    ResourceField::EnterpriseCost8,
    ResourceField::EnterpriseCost9,
    ResourceField::EnterpriseCost10,
];

/// Corresponds to Java: `ResourceFieldLists.ENTERPRISE_CUSTOM_DATE` (indices 1-30).
const ENTERPRISE_CUSTOM_DATE: [ResourceField; 30] = [
    ResourceField::EnterpriseDate1,
    ResourceField::EnterpriseDate2,
    ResourceField::EnterpriseDate3,
    ResourceField::EnterpriseDate4,
    ResourceField::EnterpriseDate5,
    ResourceField::EnterpriseDate6,
    ResourceField::EnterpriseDate7,
    ResourceField::EnterpriseDate8,
    ResourceField::EnterpriseDate9,
    ResourceField::EnterpriseDate10,
    ResourceField::EnterpriseDate11,
    ResourceField::EnterpriseDate12,
    ResourceField::EnterpriseDate13,
    ResourceField::EnterpriseDate14,
    ResourceField::EnterpriseDate15,
    ResourceField::EnterpriseDate16,
    ResourceField::EnterpriseDate17,
    ResourceField::EnterpriseDate18,
    ResourceField::EnterpriseDate19,
    ResourceField::EnterpriseDate20,
    ResourceField::EnterpriseDate21,
    ResourceField::EnterpriseDate22,
    ResourceField::EnterpriseDate23,
    ResourceField::EnterpriseDate24,
    ResourceField::EnterpriseDate25,
    ResourceField::EnterpriseDate26,
    ResourceField::EnterpriseDate27,
    ResourceField::EnterpriseDate28,
    ResourceField::EnterpriseDate29,
    ResourceField::EnterpriseDate30,
];

/// Corresponds to Java: `ResourceFieldLists.ENTERPRISE_CUSTOM_DURATION` (indices 1-10).
const ENTERPRISE_CUSTOM_DURATION: [ResourceField; 10] = [
    ResourceField::EnterpriseDuration1,
    ResourceField::EnterpriseDuration2,
    ResourceField::EnterpriseDuration3,
    ResourceField::EnterpriseDuration4,
    ResourceField::EnterpriseDuration5,
    ResourceField::EnterpriseDuration6,
    ResourceField::EnterpriseDuration7,
    ResourceField::EnterpriseDuration8,
    ResourceField::EnterpriseDuration9,
    ResourceField::EnterpriseDuration10,
];

/// Corresponds to Java: `ResourceFieldLists.ENTERPRISE_CUSTOM_FLAG` (indices 1-20).
const ENTERPRISE_CUSTOM_FLAG: [ResourceField; 20] = [
    ResourceField::EnterpriseFlag1,
    ResourceField::EnterpriseFlag2,
    ResourceField::EnterpriseFlag3,
    ResourceField::EnterpriseFlag4,
    ResourceField::EnterpriseFlag5,
    ResourceField::EnterpriseFlag6,
    ResourceField::EnterpriseFlag7,
    ResourceField::EnterpriseFlag8,
    ResourceField::EnterpriseFlag9,
    ResourceField::EnterpriseFlag10,
    ResourceField::EnterpriseFlag11,
    ResourceField::EnterpriseFlag12,
    ResourceField::EnterpriseFlag13,
    ResourceField::EnterpriseFlag14,
    ResourceField::EnterpriseFlag15,
    ResourceField::EnterpriseFlag16,
    ResourceField::EnterpriseFlag17,
    ResourceField::EnterpriseFlag18,
    ResourceField::EnterpriseFlag19,
    ResourceField::EnterpriseFlag20,
];

/// Corresponds to Java: `ResourceFieldLists.ENTERPRISE_CUSTOM_NUMBER` (indices 1-40).
const ENTERPRISE_CUSTOM_NUMBER: [ResourceField; 40] = [
    ResourceField::EnterpriseNumber1,
    ResourceField::EnterpriseNumber2,
    ResourceField::EnterpriseNumber3,
    ResourceField::EnterpriseNumber4,
    ResourceField::EnterpriseNumber5,
    ResourceField::EnterpriseNumber6,
    ResourceField::EnterpriseNumber7,
    ResourceField::EnterpriseNumber8,
    ResourceField::EnterpriseNumber9,
    ResourceField::EnterpriseNumber10,
    ResourceField::EnterpriseNumber11,
    ResourceField::EnterpriseNumber12,
    ResourceField::EnterpriseNumber13,
    ResourceField::EnterpriseNumber14,
    ResourceField::EnterpriseNumber15,
    ResourceField::EnterpriseNumber16,
    ResourceField::EnterpriseNumber17,
    ResourceField::EnterpriseNumber18,
    ResourceField::EnterpriseNumber19,
    ResourceField::EnterpriseNumber20,
    ResourceField::EnterpriseNumber21,
    ResourceField::EnterpriseNumber22,
    ResourceField::EnterpriseNumber23,
    ResourceField::EnterpriseNumber24,
    ResourceField::EnterpriseNumber25,
    ResourceField::EnterpriseNumber26,
    ResourceField::EnterpriseNumber27,
    ResourceField::EnterpriseNumber28,
    ResourceField::EnterpriseNumber29,
    ResourceField::EnterpriseNumber30,
    ResourceField::EnterpriseNumber31,
    ResourceField::EnterpriseNumber32,
    ResourceField::EnterpriseNumber33,
    ResourceField::EnterpriseNumber34,
    ResourceField::EnterpriseNumber35,
    ResourceField::EnterpriseNumber36,
    ResourceField::EnterpriseNumber37,
    ResourceField::EnterpriseNumber38,
    ResourceField::EnterpriseNumber39,
    ResourceField::EnterpriseNumber40,
];

/// Corresponds to Java: `ResourceFieldLists.ENTERPRISE_CUSTOM_TEXT` (indices 1-40).
const ENTERPRISE_CUSTOM_TEXT: [ResourceField; 40] = [
    ResourceField::EnterpriseText1,
    ResourceField::EnterpriseText2,
    ResourceField::EnterpriseText3,
    ResourceField::EnterpriseText4,
    ResourceField::EnterpriseText5,
    ResourceField::EnterpriseText6,
    ResourceField::EnterpriseText7,
    ResourceField::EnterpriseText8,
    ResourceField::EnterpriseText9,
    ResourceField::EnterpriseText10,
    ResourceField::EnterpriseText11,
    ResourceField::EnterpriseText12,
    ResourceField::EnterpriseText13,
    ResourceField::EnterpriseText14,
    ResourceField::EnterpriseText15,
    ResourceField::EnterpriseText16,
    ResourceField::EnterpriseText17,
    ResourceField::EnterpriseText18,
    ResourceField::EnterpriseText19,
    ResourceField::EnterpriseText20,
    ResourceField::EnterpriseText21,
    ResourceField::EnterpriseText22,
    ResourceField::EnterpriseText23,
    ResourceField::EnterpriseText24,
    ResourceField::EnterpriseText25,
    ResourceField::EnterpriseText26,
    ResourceField::EnterpriseText27,
    ResourceField::EnterpriseText28,
    ResourceField::EnterpriseText29,
    ResourceField::EnterpriseText30,
    ResourceField::EnterpriseText31,
    ResourceField::EnterpriseText32,
    ResourceField::EnterpriseText33,
    ResourceField::EnterpriseText34,
    ResourceField::EnterpriseText35,
    ResourceField::EnterpriseText36,
    ResourceField::EnterpriseText37,
    ResourceField::EnterpriseText38,
    ResourceField::EnterpriseText39,
    ResourceField::EnterpriseText40,
];

/// Corresponds to Java: `ResourceFieldLists.BASELINE_COSTS` (indices 1-10).
const BASELINE_COSTS: [ResourceField; 10] = [
    ResourceField::Baseline1Cost,
    ResourceField::Baseline2Cost,
    ResourceField::Baseline3Cost,
    ResourceField::Baseline4Cost,
    ResourceField::Baseline5Cost,
    ResourceField::Baseline6Cost,
    ResourceField::Baseline7Cost,
    ResourceField::Baseline8Cost,
    ResourceField::Baseline9Cost,
    ResourceField::Baseline10Cost,
];

/// Corresponds to Java: `ResourceFieldLists.BASELINE_WORKS` (indices 1-10).
const BASELINE_WORKS: [ResourceField; 10] = [
    ResourceField::Baseline1Work,
    ResourceField::Baseline2Work,
    ResourceField::Baseline3Work,
    ResourceField::Baseline4Work,
    ResourceField::Baseline5Work,
    ResourceField::Baseline6Work,
    ResourceField::Baseline7Work,
    ResourceField::Baseline8Work,
    ResourceField::Baseline9Work,
    ResourceField::Baseline10Work,
];

/// Corresponds to Java: `ResourceFieldLists.BASELINE_BUDGET_COSTS` (indices 1-10).
const BASELINE_BUDGET_COSTS: [ResourceField; 10] = [
    ResourceField::Baseline1BudgetCost,
    ResourceField::Baseline2BudgetCost,
    ResourceField::Baseline3BudgetCost,
    ResourceField::Baseline4BudgetCost,
    ResourceField::Baseline5BudgetCost,
    ResourceField::Baseline6BudgetCost,
    ResourceField::Baseline7BudgetCost,
    ResourceField::Baseline8BudgetCost,
    ResourceField::Baseline9BudgetCost,
    ResourceField::Baseline10BudgetCost,
];

/// Corresponds to Java: `ResourceFieldLists.BASELINE_BUDGET_WORKS` (indices 1-10).
const BASELINE_BUDGET_WORKS: [ResourceField; 10] = [
    ResourceField::Baseline1BudgetWork,
    ResourceField::Baseline2BudgetWork,
    ResourceField::Baseline3BudgetWork,
    ResourceField::Baseline4BudgetWork,
    ResourceField::Baseline5BudgetWork,
    ResourceField::Baseline6BudgetWork,
    ResourceField::Baseline7BudgetWork,
    ResourceField::Baseline8BudgetWork,
    ResourceField::Baseline9BudgetWork,
    ResourceField::Baseline10BudgetWork,
];

/// Corresponds to Java: `ResourceFieldLists.BASELINE_MATERIALS` (indices 1-10).
const BASELINE_MATERIALS: [ResourceField; 10] = [
    ResourceField::Baseline1Material,
    ResourceField::Baseline2Material,
    ResourceField::Baseline3Material,
    ResourceField::Baseline4Material,
    ResourceField::Baseline5Material,
    ResourceField::Baseline6Material,
    ResourceField::Baseline7Material,
    ResourceField::Baseline8Material,
    ResourceField::Baseline9Material,
    ResourceField::Baseline10Material,
];

/// Corresponds to Java: `Resource.ALWAYS_CALCULATED_FIELDS`.
fn is_always_calculated(field: FieldKey) -> bool {
    matches!(
        field,
        FieldKey::Resource(
            ResourceField::StandardRate
                | ResourceField::OvertimeRate
                | ResourceField::CostPerUse
                | ResourceField::Start
                | ResourceField::Finish
                | ResourceField::MaxUnits
                | ResourceField::AvailableFrom
                | ResourceField::AvailableTo
                | ResourceField::PlannedWork
        )
    )
}

/// Corresponds to Java: `Resource.DEPENDENCY_MAP` (built by the `FieldContainerDependencies` static block).
///
/// Keys are "modified fields"; values are lists of "calculated fields" whose caches must be cleared.
fn dependency_map() -> &'static HashMap<FieldKey, Vec<FieldKey>> {
    static MAP: OnceLock<HashMap<FieldKey, Vec<FieldKey>>> = OnceLock::new();

    MAP.get_or_init(|| {
        let mut map: HashMap<FieldKey, Vec<FieldKey>> = HashMap::new();
        let mut add = |dependency: ResourceField, dependent: ResourceField| {
            map.entry(key(dependency))
                .or_default()
                .push(key(dependent));
        };

        add(ResourceField::Cost, ResourceField::CostVariance);
        add(ResourceField::BaselineCost, ResourceField::CostVariance);
        add(ResourceField::Bcwp, ResourceField::Cv);
        add(ResourceField::Acwp, ResourceField::Cv);
        add(ResourceField::Bcwp, ResourceField::Sv);
        add(ResourceField::Bcws, ResourceField::Sv);
        add(ResourceField::Peak, ResourceField::Overallocated);
        add(ResourceField::MaxUnits, ResourceField::Overallocated);
        add(ResourceField::UnitOfMeasureUniqueId, ResourceField::MaterialLabel);
        add(ResourceField::RemainingCost, ResourceField::RemainingRegularCost);
        add(
            ResourceField::RemainingOvertimeCost,
            ResourceField::RemainingRegularCost,
        );
        add(ResourceField::ActualCost, ResourceField::ActualRegularCost);
        add(
            ResourceField::ActualOvertimeCost,
            ResourceField::ActualRegularCost,
        );
        add(ResourceField::RemainingWork, ResourceField::RemainingRegularWork);
        add(
            ResourceField::RemainingOvertimeWork,
            ResourceField::RemainingRegularWork,
        );
        add(ResourceField::ActualWork, ResourceField::ActualRegularWork);
        add(
            ResourceField::ActualOvertimeWork,
            ResourceField::ActualRegularWork,
        );
        add(ResourceField::ActualWork, ResourceField::ActualMaterial);
        add(ResourceField::RemainingWork, ResourceField::RemainingMaterial);
        add(ResourceField::Work, ResourceField::Material);
        add(ResourceField::BaselineWork, ResourceField::BaselineMaterial);
        add(ResourceField::Baseline1Work, ResourceField::Baseline1Material);
        add(ResourceField::Baseline2Work, ResourceField::Baseline2Material);
        add(ResourceField::Baseline3Work, ResourceField::Baseline3Material);
        add(ResourceField::Baseline4Work, ResourceField::Baseline4Material);
        add(ResourceField::Baseline5Work, ResourceField::Baseline5Material);
        add(ResourceField::Baseline6Work, ResourceField::Baseline6Material);
        add(ResourceField::Baseline7Work, ResourceField::Baseline7Material);
        add(ResourceField::Baseline8Work, ResourceField::Baseline8Material);
        add(ResourceField::Baseline9Work, ResourceField::Baseline9Material);
        add(ResourceField::Baseline10Work, ResourceField::Baseline10Material);

        map
    })
}

/// Storage index of `ResourceType` (declaration order).
fn resource_type_index(value: ResourceType) -> i32 {
    match value {
        ResourceType::Material => 0,
        ResourceType::Work => 1,
        ResourceType::Cost => 2,
        ResourceType::NonLabor => 3,
    }
}

/// A resource in the project.
///
/// Corresponds to Java: `public final class Resource extends AbstractFieldContainer<Resource> implements Comparable<Resource>, ProjectEntityWithID, ChildResourceContainer`
pub struct Resource {
    /// Parent context (weak reference to avoid an `Rc` cycle).
    ///
    /// Corresponds to Java: `private final ProjectContext m_context;`
    context: Weak<RefCell<ProjectContext>>,

    /// Weak reference to itself (Rust-specific).
    ///
    /// Corresponds to `this` in Java's `handleFieldChange`: when the unique ID / ID changes, the element handle
    /// must be handed to the container (`updateUniqueID` / `mapID`). It is
    /// wired up by [`crate::resource_container::ResourceContainer`] after wrapping the `Rc`.
    self_ref: Weak<RefCell<Resource>>,

    /// Field storage (Java's inherited base class `AbstractFieldContainer`).
    fields: AbstractFieldContainer,

    /// Whether this is a null resource.
    ///
    /// Corresponds to Java: `private boolean m_null;`
    null: bool,

    /// Active Directory GUID.
    ///
    /// Corresponds to Java: `private String m_activeDirectoryGUID;`
    active_directory_guid: Option<String>,

    /// Role assignments (role ID → role and skill level).
    ///
    /// Corresponds to Java: `private final Map<Resource, SkillLevel> m_roleAssignments = new HashMap<>();`
    role_assignments: HashMap<i32, (Weak<RefCell<Resource>>, SkillLevel)>,

    /// Child resources.
    ///
    /// Corresponds to Java: `private final List<Resource> m_children = new ArrayList<>();`
    children: Vec<Rc<RefCell<Resource>>>,

    /// Cost rate tables (A-E).
    ///
    /// Corresponds to Java: `private final CostRateTable[] m_costRateTables;`
    cost_rate_tables: Vec<CostRateTable>,

    /// Availability table.
    ///
    /// Corresponds to Java: `private final AvailabilityTable m_availability = new AvailabilityTable();`
    ///
    /// The Rust side wraps it in a `RefCell` because `availableFrom` / `availableTo` need `&mut self`
    /// (see the notes at the top of the file).
    availability: RefCell<AvailabilityTable>,
}

impl Resource {
    /// Constructor.
    ///
    /// Corresponds to Java: `Resource(ProjectContext context)`
    ///
    /// The Rust side only initializes fields; the unique ID / ID that Java assigns in the constructor according to configuration are handled by
    /// [`crate::resource_container::ResourceContainer::add`] (see the notes at the top of the file).
    pub(crate) fn new(context: Weak<RefCell<ProjectContext>>) -> Self {
        let mut cost_rate_tables = Vec::with_capacity(MAX_TABLES);
        for _ in 0..MAX_TABLES {
            let mut table = CostRateTable::new();
            table.add(CostRateTableEntry::default_entry());
            cost_rate_tables.push(table);
        }

        Self {
            context,
            self_ref: Weak::new(),
            fields: AbstractFieldContainer::new(),
            null: false,
            active_directory_guid: None,
            role_assignments: HashMap::new(),
            children: Vec::new(),
            cost_rate_tables,
            availability: RefCell::new(AvailabilityTable::new()),
        }
    }

    /// Wires up the weak reference to itself (Rust-specific, called by the container after wrapping the `Rc`).
    ///
    /// Corresponds to Java: the `this` used in `handleFieldChange`.
    pub(crate) fn set_self_reference(&mut self, resource: &Rc<RefCell<Resource>>) {
        self.self_ref = Rc::downgrade(resource);
    }

    /// Gets the parent context (the Rust side returns the upgraded strong reference; Java returns the strong-reference field).
    ///
    /// Corresponds to Java: `public ProjectContext getProjectContext()`
    pub fn get_project_context(&self) -> Option<Rc<RefCell<ProjectContext>>> {
        self.context.upgrade()
    }

    /// Creates and adds a child resource.
    ///
    /// Corresponds to Java: `@Override public Resource addResource()`
    ///
    /// Java relies on a non-null `m_context`; the Rust side returns
    /// `None` when the context is unavailable (not wired up or already mutably borrowed), without panicking.
    pub fn add_resource(&mut self) -> Option<Rc<RefCell<Resource>>> {
        let context = self.context.upgrade()?;
        let resource = {
            let mut context_ref = context.try_borrow_mut().ok()?;
            context_ref.get_resources_mut().add()
        };

        let parent = self.self_ref.upgrade();
        resource
            .borrow_mut()
            .set_parent_resource(parent.as_ref());
        self.children.push(Rc::clone(&resource));
        Some(resource)
    }

    /// Adds an existing resource as a child of the current resource.
    ///
    /// Corresponds to Java: `public void addChildResource(Resource child)`
    ///
    /// Java's `child.setParentResource(this)` writes the parent resource's unique ID; on the Rust side
    /// `&mut self` can obtain that value without its own `Rc`, so it is written directly.
    pub fn add_child_resource(&mut self, child: &Rc<RefCell<Resource>>) {
        let parent_unique_id = self.get_unique_id();
        child.borrow_mut().set_parent_resource_unique_id(parent_unique_id);
        self.children.push(Rc::clone(child));
    }

    /// Gets the child resource list (Java returns the live list; Rust returns a snapshot).
    ///
    /// Corresponds to Java: `@Override public List<Resource> getChildResources()`
    pub fn get_child_resources(&self) -> Vec<Rc<RefCell<Resource>>> {
        self.children.clone()
    }

    /// Gets a mutable reference to the child resource list (Rust-specific, for the container's `updateStructure`
    /// `getChildResources().clear()`).
    pub fn get_child_resources_mut(&mut self) -> &mut Vec<Rc<RefCell<Resource>>> {
        &mut self.children
    }

    /// Removes a child resource.
    ///
    /// Corresponds to Java: `public void removeChildResource(Resource child)`
    pub fn remove_child_resource(&mut self, child: &Rc<RefCell<Resource>>) {
        let before = self.children.len();
        self.children.retain(|item| !Rc::ptr_eq(item, child));
        if before != self.children.len() {
            child.borrow_mut().set_parent_resource_unique_id(None);
        }
    }

    /// Sets the Name field.
    ///
    /// Corresponds to Java: `public void setName(String val)`
    pub fn set_name(&mut self, val: Option<String>) {
        self.set_field_value(key(ResourceField::Name), val.map(FieldValue::String));
    }

    /// Gets the Name field.
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> Option<String> {
        self.get_string(ResourceField::Name)
    }

    /// Sets the resource type.
    ///
    /// Corresponds to Java: `public void setType(ResourceType type)`
    pub fn set_type(&mut self, value: ResourceType) {
        self.set_field_value(
            key(ResourceField::Type),
            Some(FieldValue::Integer(resource_type_index(value))),
        );
    }

    /// Gets the resource type (returns the default `WORK` when unset).
    ///
    /// Corresponds to Java: `public ResourceType getType()`
    pub fn get_type(&self) -> ResourceType {
        self.get(key(ResourceField::Type))
            .and_then(|value| value.as_integer())
            .and_then(ResourceType::get_instance)
            .unwrap_or(ResourceType::Work)
    }

    /// Sets the null resource flag.
    ///
    /// Corresponds to Java: `public void setIsNull(boolean isNull)`
    pub fn set_is_null(&mut self, is_null: bool) {
        self.null = is_null;
    }

    /// Gets the null resource flag.
    ///
    /// Corresponds to Java: `public boolean getNull()`
    pub fn get_null(&self) -> bool {
        self.null
    }

    /// Sets the Initials field.
    ///
    /// Corresponds to Java: `public void setInitials(String val)`
    pub fn set_initials(&mut self, val: Option<String>) {
        self.set_field_value(key(ResourceField::Initials), val.map(FieldValue::String));
    }

    /// Gets the Initials field.
    ///
    /// Corresponds to Java: `public String getInitials()`
    pub fn get_initials(&self) -> Option<String> {
        self.get_string(ResourceField::Initials)
    }

    /// Sets the Japanese phonetics.
    ///
    /// Corresponds to Java: `public void setPhonetics(String phonetics)`
    pub fn set_phonetics(&mut self, phonetics: Option<String>) {
        self.set_field_value(
            key(ResourceField::Phonetics),
            phonetics.map(FieldValue::String),
        );
    }

    /// Gets the Japanese phonetics.
    ///
    /// Corresponds to Java: `public String getPhonetics()`
    pub fn get_phonetics(&self) -> Option<String> {
        self.get_string(ResourceField::Phonetics)
    }

    /// Sets the Windows account name.
    ///
    /// Corresponds to Java: `public void setNtAccount(String ntAccount)`
    pub fn set_nt_account(&mut self, nt_account: Option<String>) {
        self.set_field_value(
            key(ResourceField::WindowsUserAccount),
            nt_account.map(FieldValue::String),
        );
    }

    /// Gets the Windows account name.
    ///
    /// Corresponds to Java: `public String getNtAccount()`
    pub fn get_nt_account(&self) -> Option<String> {
        self.get_string(ResourceField::WindowsUserAccount)
    }

    /// Gets the material resource's unit label (calculated from the unit abbreviation).
    ///
    /// Corresponds to Java: `public String getMaterialLabel()`
    pub fn get_material_label(&self) -> Option<String> {
        self.get_string(ResourceField::MaterialLabel)
    }

    /// Sets the Code field.
    ///
    /// Corresponds to Java: `public void setCode(String val)`
    pub fn set_code(&mut self, val: Option<String>) {
        self.set_field_value(key(ResourceField::Code), val.map(FieldValue::String));
    }

    /// Gets the Code field.
    ///
    /// Corresponds to Java: `public String getCode()`
    pub fn get_code(&self) -> Option<String> {
        self.get_string(ResourceField::Code)
    }

    /// Sets the Group field.
    ///
    /// Corresponds to Java: `public void setGroup(String val)`
    pub fn set_group(&mut self, val: Option<String>) {
        self.set_field_value(key(ResourceField::Group), val.map(FieldValue::String));
    }

    /// Gets the Group field.
    ///
    /// Corresponds to Java: `public String getGroup()`
    pub fn get_group(&self) -> Option<String> {
        self.get_string(ResourceField::Group)
    }

    /// Sets the work group.
    ///
    /// Corresponds to Java: `public void setWorkGroup(WorkGroup workGroup)`
    pub fn set_work_group(&mut self, work_group: WorkGroup) {
        self.set_field_value(
            key(ResourceField::Workgroup),
            Some(FieldValue::Integer(work_group.value())),
        );
    }

    /// Gets the work group.
    ///
    /// Corresponds to Java: `public WorkGroup getWorkGroup()`
    pub fn get_work_group(&self) -> Option<WorkGroup> {
        self.get(key(ResourceField::Workgroup))
            .and_then(|value| value.as_integer())
            .map(WorkGroup::get_instance)
    }

    /// Sets the email address.
    ///
    /// Corresponds to Java: `public void setEmailAddress(String emailAddress)`
    pub fn set_email_address(&mut self, email_address: Option<String>) {
        self.set_field_value(
            key(ResourceField::EmailAddress),
            email_address.map(FieldValue::String),
        );
    }

    /// Gets the email address.
    ///
    /// Corresponds to Java: `public String getEmailAddress()`
    pub fn get_email_address(&self) -> Option<String> {
        self.get_string(ResourceField::EmailAddress)
    }

    /// Sets the hyperlink text.
    ///
    /// Corresponds to Java: `public void setHyperlink(String hyperlink)`
    pub fn set_hyperlink(&mut self, hyperlink: Option<String>) {
        self.set_field_value(
            key(ResourceField::Hyperlink),
            hyperlink.map(FieldValue::String),
        );
    }

    /// Gets the hyperlink text.
    ///
    /// Corresponds to Java: `public String getHyperlink()`
    pub fn get_hyperlink(&self) -> Option<String> {
        self.get_string(ResourceField::Hyperlink)
    }

    /// Sets the hyperlink address.
    ///
    /// Corresponds to Java: `public void setHyperlinkAddress(String hyperlinkAddress)`
    pub fn set_hyperlink_address(&mut self, hyperlink_address: Option<String>) {
        self.set_field_value(
            key(ResourceField::HyperlinkAddress),
            hyperlink_address.map(FieldValue::String),
        );
    }

    /// Gets the hyperlink address.
    ///
    /// Corresponds to Java: `public String getHyperlinkAddress()`
    pub fn get_hyperlink_address(&self) -> Option<String> {
        self.get_string(ResourceField::HyperlinkAddress)
    }

    /// Sets the hyperlink sub-address.
    ///
    /// Corresponds to Java: `public void setHyperlinkSubAddress(String hyperlinkSubAddress)`
    pub fn set_hyperlink_sub_address(&mut self, hyperlink_sub_address: Option<String>) {
        self.set_field_value(
            key(ResourceField::HyperlinkSubaddress),
            hyperlink_sub_address.map(FieldValue::String),
        );
    }

    /// Gets the hyperlink sub-address.
    ///
    /// Corresponds to Java: `public String getHyperlinkSubAddress()`
    pub fn get_hyperlink_sub_address(&self) -> Option<String> {
        self.get_string(ResourceField::HyperlinkSubaddress)
    }

    /// Sets the hyperlink screen tip.
    ///
    /// Corresponds to Java: `public void setHyperlinkScreenTip(String text)`
    pub fn set_hyperlink_screen_tip(&mut self, text: Option<String>) {
        self.set_field_value(
            key(ResourceField::HyperlinkScreenTip),
            text.map(FieldValue::String),
        );
    }

    /// Gets the hyperlink screen tip.
    ///
    /// Corresponds to Java: `public String getHyperlinkScreenTip()`
    pub fn get_hyperlink_screen_tip(&self) -> Option<String> {
        self.get_string(ResourceField::HyperlinkScreenTip)
    }

    /// Sets the default availability (default 100).
    ///
    /// Corresponds to Java: `public void setDefaultUnits(Number defaultUnits)`
    pub fn set_default_units(&mut self, default_units: Option<f64>) {
        self.set_field_value(
            key(ResourceField::DefaultUnits),
            default_units.map(FieldValue::Double),
        );
    }

    /// Gets the default availability.
    ///
    /// Corresponds to Java: `public Number getDefaultUnits()`
    pub fn get_default_units(&self) -> Option<f64> {
        self.get_f64(ResourceField::DefaultUnits)
    }

    /// Gets the maximum availability for the current date.
    ///
    /// Corresponds to Java: `public Number getMaxUnits()`
    pub fn get_max_units(&self) -> Option<f64> {
        self.get_f64(ResourceField::MaxUnits)
    }

    /// Sets the peak utilization.
    ///
    /// Corresponds to Java: `public void setPeakUnits(Number peakUnits)`
    pub fn set_peak_units(&mut self, peak_units: Option<f64>) {
        self.set_field_value(
            key(ResourceField::Peak),
            peak_units.map(FieldValue::Double),
        );
    }

    /// Gets the peak utilization.
    ///
    /// Corresponds to Java: `public Number getPeakUnits()`
    pub fn get_peak_units(&self) -> Option<f64> {
        self.get_f64(ResourceField::Peak)
    }

    /// Sets the over-allocation flag.
    ///
    /// Corresponds to Java: `public void setOverAllocated(boolean overallocated)`
    pub fn set_over_allocated(&mut self, overallocated: bool) {
        self.set_field_value(
            key(ResourceField::Overallocated),
            Some(FieldValue::Boolean(overallocated)),
        );
    }

    /// Gets the over-allocation flag.
    ///
    /// Corresponds to Java: `public boolean getOverAllocated()`
    pub fn get_over_allocated(&self) -> bool {
        self.get_bool(ResourceField::Overallocated)
    }

    /// Gets the "available from" date.
    ///
    /// Corresponds to Java: `public LocalDateTime getAvailableFrom()`
    pub fn get_available_from(&self) -> Option<NaiveDateTime> {
        self.get_date_time(ResourceField::AvailableFrom)
    }

    /// Gets the "available to" date.
    ///
    /// Corresponds to Java: `public LocalDateTime getAvailableTo()`
    pub fn get_available_to(&self) -> Option<NaiveDateTime> {
        self.get_date_time(ResourceField::AvailableTo)
    }

    /// Gets the earliest start date of all assigned tasks.
    ///
    /// Corresponds to Java: `public LocalDateTime getStart()`
    pub fn get_start(&self) -> Option<NaiveDateTime> {
        self.get_date_time(ResourceField::Start)
    }

    /// Gets the latest finish date of all assigned tasks.
    ///
    /// Corresponds to Java: `public LocalDateTime getFinish()`
    pub fn get_finish(&self) -> Option<NaiveDateTime> {
        self.get_date_time(ResourceField::Finish)
    }

    /// Sets whether the resource can be leveled.
    ///
    /// Corresponds to Java: `public void setCanLevel(boolean canLevel)`
    pub fn set_can_level(&mut self, can_level: bool) {
        self.set_field_value(
            key(ResourceField::CanLevel),
            Some(FieldValue::Boolean(can_level)),
        );
    }

    /// Gets whether the resource can be leveled.
    ///
    /// Corresponds to Java: `public boolean getCanLevel()`
    pub fn get_can_level(&self) -> bool {
        self.get_bool(ResourceField::CanLevel)
    }

    /// Sets the cost accrual method.
    ///
    /// Corresponds to Java: `public void setAccrueAt(AccrueType type)`
    pub fn set_accrue_at(&mut self, value: AccrueType) {
        self.set_field_value(
            key(ResourceField::AccrueAt),
            Some(FieldValue::Integer(value.value())),
        );
    }

    /// Gets the cost accrual method.
    ///
    /// Corresponds to Java: `public AccrueType getAccrueAt()`
    pub fn get_accrue_at(&self) -> Option<AccrueType> {
        self.get(key(ResourceField::AccrueAt))
            .and_then(|value| value.as_integer())
            .and_then(AccrueType::get_instance)
    }

    /// Sets the Work field (ignored when reading MS Project files).
    ///
    /// Corresponds to Java: `public void setWork(Duration val)`
    pub fn set_work(&mut self, val: Option<Duration>) {
        self.set_field_value(key(ResourceField::Work), val.map(FieldValue::Duration));
    }

    /// Gets the Work field.
    ///
    /// Corresponds to Java: `public Duration getWork()`
    pub fn get_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::Work)
    }

    /// Gets the Regular Work field (an extension in the MPX specification).
    ///
    /// Corresponds to Java: `public Duration getRegularWork()`
    pub fn get_regular_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::RegularWork)
    }

    /// Sets the Regular Work field.
    ///
    /// Corresponds to Java: `public void setRegularWork(Duration duration)`
    pub fn set_regular_work(&mut self, duration: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::RegularWork),
            duration.map(FieldValue::Duration),
        );
    }

    /// Sets the Actual Work field.
    ///
    /// Corresponds to Java: `public void setActualWork(Duration val)`
    pub fn set_actual_work(&mut self, val: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::ActualWork),
            val.map(FieldValue::Duration),
        );
    }

    /// Gets the Actual Work field.
    ///
    /// Corresponds to Java: `public Duration getActualWork()`
    pub fn get_actual_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::ActualWork)
    }

    /// Sets the overtime work.
    ///
    /// Corresponds to Java: `public void setOvertimeWork(Duration overtimeWork)`
    pub fn set_overtime_work(&mut self, overtime_work: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::OvertimeWork),
            overtime_work.map(FieldValue::Duration),
        );
    }

    /// Gets the overtime work.
    ///
    /// Corresponds to Java: `public Duration getOvertimeWork()`
    pub fn get_overtime_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::OvertimeWork)
    }

    /// Sets the Remaining Work field (ignored when reading MS Project files).
    ///
    /// Corresponds to Java: `public void setRemainingWork(Duration val)`
    pub fn set_remaining_work(&mut self, val: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::RemainingWork),
            val.map(FieldValue::Duration),
        );
    }

    /// Gets the Remaining Work field.
    ///
    /// Corresponds to Java: `public Duration getRemainingWork()`
    pub fn get_remaining_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::RemainingWork)
    }

    /// Gets the actual overtime work.
    ///
    /// Corresponds to Java: `public Duration getActualOvertimeWork()`
    pub fn get_actual_overtime_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::ActualOvertimeWork)
    }

    /// Sets the actual overtime work.
    ///
    /// Corresponds to Java: `public void setActualOvertimeWork(Duration duration)`
    pub fn set_actual_overtime_work(&mut self, duration: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::ActualOvertimeWork),
            duration.map(FieldValue::Duration),
        );
    }

    /// Gets the remaining overtime work.
    ///
    /// Corresponds to Java: `public Duration getRemainingOvertimeWork()`
    pub fn get_remaining_overtime_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::RemainingOvertimeWork)
    }

    /// Sets the remaining overtime work.
    ///
    /// Corresponds to Java: `public void setRemainingOvertimeWork(Duration duration)`
    pub fn set_remaining_overtime_work(&mut self, duration: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::RemainingOvertimeWork),
            duration.map(FieldValue::Duration),
        );
    }

    /// Sets the percentage complete.
    ///
    /// Corresponds to Java: `public void setPercentWorkComplete(Number percentWorkComplete)`
    pub fn set_percent_work_complete(&mut self, percent_work_complete: Option<f64>) {
        self.set_field_value(
            key(ResourceField::PercentWorkComplete),
            percent_work_complete.map(FieldValue::Double),
        );
    }

    /// Gets the percentage complete.
    ///
    /// Corresponds to Java: `public Number getPercentWorkComplete()`
    pub fn get_percent_work_complete(&self) -> Option<f64> {
        self.get_f64(ResourceField::PercentWorkComplete)
    }

    /// Gets the standard rate (calculated from the current cost rate table A).
    ///
    /// Corresponds to Java: `public Rate getStandardRate()`
    pub fn get_standard_rate(&self) -> Option<Rate> {
        self.get_field_rate(ResourceField::StandardRate)
    }

    /// Sets the Cost field.
    ///
    /// Corresponds to Java: `public void setCost(Number cost)`
    pub fn set_cost(&mut self, cost: Option<f64>) {
        self.set_field_value(key(ResourceField::Cost), cost.map(FieldValue::Double));
    }

    /// Gets the Cost field.
    ///
    /// Corresponds to Java: `public Number getCost()`
    pub fn get_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::Cost)
    }

    /// Gets the overtime rate (calculated from the current cost rate table A).
    ///
    /// Corresponds to Java: `public Rate getOvertimeRate()`
    pub fn get_overtime_rate(&self) -> Option<Rate> {
        self.get_field_rate(ResourceField::OvertimeRate)
    }

    /// Gets the overtime cost.
    ///
    /// Corresponds to Java: `public Number getOvertimeCost()`
    pub fn get_overtime_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::OvertimeCost)
    }

    /// Sets the overtime cost.
    ///
    /// Corresponds to Java: `public void setOvertimeCost(Number currency)`
    pub fn set_overtime_cost(&mut self, currency: Option<f64>) {
        self.set_field_value(
            key(ResourceField::OvertimeCost),
            currency.map(FieldValue::Double),
        );
    }

    /// Gets the cost per use (calculated from the current cost rate table A).
    ///
    /// Corresponds to Java: `public Number getCostPerUse()`
    pub fn get_cost_per_use(&self) -> Option<f64> {
        self.get_f64(ResourceField::CostPerUse)
    }

    /// Sets the actual cost.
    ///
    /// Corresponds to Java: `public void setActualCost(Number actualCost)`
    pub fn set_actual_cost(&mut self, actual_cost: Option<f64>) {
        self.set_field_value(
            key(ResourceField::ActualCost),
            actual_cost.map(FieldValue::Double),
        );
    }

    /// Gets the actual cost.
    ///
    /// Corresponds to Java: `public Number getActualCost()`
    pub fn get_actual_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::ActualCost)
    }

    /// Gets the actual overtime cost.
    ///
    /// Corresponds to Java: `public Number getActualOvertimeCost()`
    pub fn get_actual_overtime_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::ActualOvertimeCost)
    }

    /// Sets the actual overtime cost.
    ///
    /// Corresponds to Java: `public void setActualOvertimeCost(Number actualOvertimeCost)`
    pub fn set_actual_overtime_cost(&mut self, actual_overtime_cost: Option<f64>) {
        self.set_field_value(
            key(ResourceField::ActualOvertimeCost),
            actual_overtime_cost.map(FieldValue::Double),
        );
    }

    /// Sets the remaining cost.
    ///
    /// Corresponds to Java: `public void setRemainingCost(Number remainingCost)`
    pub fn set_remaining_cost(&mut self, remaining_cost: Option<f64>) {
        self.set_field_value(
            key(ResourceField::RemainingCost),
            remaining_cost.map(FieldValue::Double),
        );
    }

    /// Gets the remaining cost.
    ///
    /// Corresponds to Java: `public Number getRemainingCost()`
    pub fn get_remaining_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::RemainingCost)
    }

    /// Gets the remaining overtime cost.
    ///
    /// Corresponds to Java: `public Number getRemainingOvertimeCost()`
    pub fn get_remaining_overtime_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::RemainingOvertimeCost)
    }

    /// Sets the remaining overtime cost.
    ///
    /// Corresponds to Java: `public void setRemainingOvertimeCost(Number remainingOvertimeCost)`
    pub fn set_remaining_overtime_cost(&mut self, remaining_overtime_cost: Option<f64>) {
        self.set_field_value(
            key(ResourceField::RemainingOvertimeCost),
            remaining_overtime_cost.map(FieldValue::Double),
        );
    }

    /// Sets the work variance.
    ///
    /// Corresponds to Java: `public void setWorkVariance(Duration workVariance)`
    pub fn set_work_variance(&mut self, work_variance: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::WorkVariance),
            work_variance.map(FieldValue::Duration),
        );
    }

    /// Gets the work variance.
    ///
    /// Corresponds to Java: `public Duration getWorkVariance()`
    pub fn get_work_variance(&self) -> Option<Duration> {
        self.get_duration(ResourceField::WorkVariance)
    }

    /// Sets the cost variance.
    ///
    /// Corresponds to Java: `public void setCostVariance(Number costVariance)`
    pub fn set_cost_variance(&mut self, cost_variance: Option<f64>) {
        self.set_field_value(
            key(ResourceField::CostVariance),
            cost_variance.map(FieldValue::Double),
        );
    }

    /// Gets the cost variance.
    ///
    /// Corresponds to Java: `public Number getCostVariance()`
    pub fn get_cost_variance(&self) -> Option<f64> {
        self.get_f64(ResourceField::CostVariance)
    }

    /// Sets the schedule variance (SV).
    ///
    /// Corresponds to Java: `public void setSV(Number sv)`
    pub fn set_sv(&mut self, sv: Option<f64>) {
        self.set_field_value(key(ResourceField::Sv), sv.map(FieldValue::Double));
    }

    /// Gets the schedule variance (SV).
    ///
    /// Corresponds to Java: `public Number getSV()`
    pub fn get_sv(&self) -> Option<f64> {
        self.get_f64(ResourceField::Sv)
    }

    /// Sets the cost variance (CV).
    ///
    /// Corresponds to Java: `public void setCV(Number cv)`
    pub fn set_cv(&mut self, cv: Option<f64>) {
        self.set_field_value(key(ResourceField::Cv), cv.map(FieldValue::Double));
    }

    /// Gets the cost variance (CV).
    ///
    /// Corresponds to Java: `public Number getCV()`
    pub fn get_cv(&self) -> Option<f64> {
        self.get_f64(ResourceField::Cv)
    }

    /// Sets the actual cost of work performed (ACWP).
    ///
    /// Corresponds to Java: `public void setACWP(Number acwp)`
    pub fn set_acwp(&mut self, acwp: Option<f64>) {
        self.set_field_value(key(ResourceField::Acwp), acwp.map(FieldValue::Double));
    }

    /// Gets the actual cost of work performed (ACWP).
    ///
    /// Corresponds to Java: `public Number getACWP()`
    pub fn get_acwp(&self) -> Option<f64> {
        self.get_f64(ResourceField::Acwp)
    }

    /// Sets the plain-text notes.
    ///
    /// Corresponds to Java: `public void setNotes(String notes)`
    pub fn set_notes(&mut self, notes: Option<String>) {
        self.set_field_value(key(ResourceField::Notes), notes.map(FieldValue::String));
    }

    /// Gets the plain-text notes.
    ///
    /// Corresponds to Java: `public String getNotes()`
    pub fn get_notes(&self) -> String {
        self.get(key(ResourceField::Notes))
            .and_then(|value| value.as_string().map(str::to_owned))
            .unwrap_or_default()
    }

    /// Sets the `Notes` instance (the Rust side can only keep its plain-text representation; see the notes at the top of the file).
    ///
    /// Corresponds to Java: `public void setNotesObject(Notes notes)`
    pub fn set_notes_object(&mut self, notes: Option<&dyn Notes>) {
        self.set_notes(notes.map(|notes| notes.to_string()));
    }

    /// Gets the plain-text notes object (fixed to [`PlainNotes`] on the Rust side).
    ///
    /// Corresponds to Java: `public Notes getNotesObject()`
    pub fn get_notes_object(&self) -> Option<PlainNotes> {
        self.get(key(ResourceField::Notes))
            .and_then(|value| value.as_string().map(|text| PlainNotes::new(Some(text.to_owned()))))
    }

    /// Sets the budgeted cost of work scheduled (BCWS).
    ///
    /// Corresponds to Java: `public void setBCWS(Number bcws)`
    pub fn set_bcws(&mut self, bcws: Option<f64>) {
        self.set_field_value(key(ResourceField::Bcws), bcws.map(FieldValue::Double));
    }

    /// Gets the budgeted cost of work scheduled (BCWS).
    ///
    /// Corresponds to Java: `public Number getBCWS()`
    pub fn get_bcws(&self) -> Option<f64> {
        self.get_f64(ResourceField::Bcws)
    }

    /// Sets the budgeted cost of work performed (BCWP).
    ///
    /// Corresponds to Java: `public void setBCWP(Number bcwp)`
    pub fn set_bcwp(&mut self, bcwp: Option<f64>) {
        self.set_field_value(key(ResourceField::Bcwp), bcwp.map(FieldValue::Double));
    }

    /// Gets the budgeted cost of work performed (BCWP).
    ///
    /// Corresponds to Java: `public Number getBCWP()`
    pub fn get_bcwp(&self) -> Option<f64> {
        self.get_f64(ResourceField::Bcwp)
    }

    /// Sets the generic resource flag.
    ///
    /// Corresponds to Java: `public void setGeneric(boolean value)`
    pub fn set_generic(&mut self, value: bool) {
        self.set_field_value(key(ResourceField::Generic), Some(FieldValue::Boolean(value)));
    }

    /// Gets the generic resource flag.
    ///
    /// Corresponds to Java: `public boolean getGeneric()`
    pub fn get_generic(&self) -> bool {
        self.get_bool(ResourceField::Generic)
    }

    /// Sets the active flag.
    ///
    /// Corresponds to Java: `public void setActive(boolean value)`
    pub fn set_active(&mut self, value: bool) {
        self.set_field_value(key(ResourceField::Active), Some(FieldValue::Boolean(value)));
    }

    /// Gets the active flag (defaults to `true`).
    ///
    /// Corresponds to Java: `public boolean getActive()`
    pub fn get_active(&self) -> bool {
        self.get_bool(ResourceField::Active)
    }

    /// Sets the Active Directory GUID.
    ///
    /// Corresponds to Java: `public void setActiveDirectoryGUID(String guid)`
    pub fn set_active_directory_guid(&mut self, guid: Option<String>) {
        self.active_directory_guid = guid;
    }

    /// Gets the Active Directory GUID.
    ///
    /// Corresponds to Java: `public String getActiveDirectoryGUID()`
    pub fn get_active_directory_guid(&self) -> Option<String> {
        self.active_directory_guid.clone()
    }

    /// Sets the maximum actual overtime work (protected).
    ///
    /// Corresponds to Java: `public void setActualOvertimeWorkProtected(Duration duration)`
    pub fn set_actual_overtime_work_protected(&mut self, duration: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::ActualOvertimeWorkProtected),
            duration.map(FieldValue::Duration),
        );
    }

    /// Gets the maximum actual overtime work (protected).
    ///
    /// Corresponds to Java: `public Duration getActualOvertimeWorkProtected()`
    pub fn get_actual_overtime_work_protected(&self) -> Option<Duration> {
        self.get_duration(ResourceField::ActualOvertimeWorkProtected)
    }

    /// Sets the maximum actual work (protected).
    ///
    /// Corresponds to Java: `public void setActualWorkProtected(Duration duration)`
    pub fn set_actual_work_protected(&mut self, duration: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::ActualWorkProtected),
            duration.map(FieldValue::Duration),
        );
    }

    /// Gets the maximum actual work (protected).
    ///
    /// Corresponds to Java: `public Duration getActualWorkProtected()`
    pub fn get_actual_work_protected(&self) -> Option<Duration> {
        self.get_duration(ResourceField::ActualWorkProtected)
    }

    /// Sets the booking type.
    ///
    /// Corresponds to Java: `public void setBookingType(BookingType bookingType)`
    pub fn set_booking_type(&mut self, booking_type: BookingType) {
        self.set_field_value(
            key(ResourceField::BookingType),
            Some(FieldValue::Integer(booking_type.value())),
        );
    }

    /// Gets the booking type.
    ///
    /// Corresponds to Java: `public BookingType getBookingType()`
    pub fn get_booking_type(&self) -> Option<BookingType> {
        self.get(key(ResourceField::BookingType))
            .and_then(|value| value.as_integer())
            .map(BookingType::get_instance)
    }

    /// Sets the creation date.
    ///
    /// Corresponds to Java: `public void setCreationDate(LocalDateTime creationDate)`
    pub fn set_creation_date(&mut self, creation_date: Option<NaiveDateTime>) {
        self.set_field_value(
            key(ResourceField::Created),
            creation_date.map(FieldValue::DateTime),
        );
    }

    /// Gets the creation date.
    ///
    /// Corresponds to Java: `public LocalDateTime getCreationDate()`
    pub fn get_creation_date(&self) -> Option<NaiveDateTime> {
        self.get_date_time(ResourceField::Created)
    }

    /// Sets the enterprise resource flag.
    ///
    /// Corresponds to Java: `public void setEnterprise(boolean enterprise)`
    pub fn set_enterprise(&mut self, enterprise: bool) {
        self.set_field_value(
            key(ResourceField::Enterprise),
            Some(FieldValue::Boolean(enterprise)),
        );
    }

    /// Gets the enterprise resource flag.
    ///
    /// Corresponds to Java: `public boolean getEnterprise()`
    pub fn get_enterprise(&self) -> bool {
        self.get_bool(ResourceField::Enterprise)
    }

    /// Gets the resource calendar's unique ID.
    ///
    /// Corresponds to Java: `public Integer getCalendarUniqueID()`
    pub fn get_calendar_unique_id(&self) -> Option<i32> {
        self.get_i32(ResourceField::CalendarUniqueId)
    }

    /// Sets the resource calendar's unique ID.
    ///
    /// Corresponds to Java: `public void setCalendarUniqueID(Integer id)`
    pub fn set_calendar_unique_id(&mut self, id: Option<i32>) {
        self.set_field_value(
            key(ResourceField::CalendarUniqueId),
            id.map(FieldValue::Integer),
        );
    }

    /// Gets the calendar associated with the resource.
    ///
    /// Corresponds to Java: `public ProjectCalendar getCalendar()`
    pub fn get_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        let context = self.context.upgrade()?;
        let context_ref = context.try_borrow().ok()?;
        context_ref
            .get_calendars()
            .get_by_unique_id(self.get_calendar_unique_id())
    }

    /// Associates an existing resource calendar.
    ///
    /// Corresponds to Java: `public void setCalendar(ProjectCalendar calendar)`
    pub fn set_calendar(&mut self, calendar: Option<&Rc<RefCell<ProjectCalendar>>>) {
        let unique_id = calendar.and_then(|calendar| calendar.borrow().get_unique_id());
        self.set_calendar_unique_id(unique_id);
    }

    /// Creates and associates a new resource calendar.
    ///
    /// Corresponds to Java: `public ProjectCalendar addCalendar() throws MPXJException`
    ///
    /// The Rust side returns `Err` when the context is unavailable (not wired up or already mutably borrowed); this branch cannot occur in Java.
    pub fn add_calendar(&mut self) -> MpxjResult<Rc<RefCell<ProjectCalendar>>> {
        if self.get_calendar().is_some() {
            return Err(MpxjError::new(MpxjError::MAXIMUM_RECORDS));
        }

        let context = self
            .context
            .upgrade()
            .ok_or_else(|| MpxjError::new("Resource is not attached to a ProjectContext"))?;

        let calendar = {
            let mut context_ref = context
                .try_borrow_mut()
                .map_err(|_| MpxjError::new("ProjectContext is already borrowed"))?;
            context_ref.add_calendar()
        };

        let name = match self.get_name() {
            Some(name) if !name.is_empty() => name,
            _ => UNNAMED_RESOURCE.to_owned(),
        };

        if let Ok(mut calendar_ref) = calendar.try_borrow_mut() {
            calendar_ref.set_name(Some(name));
        }

        self.set_calendar(Some(&calendar));
        Ok(calendar)
    }

    /// Sets the base calendar name (falls back to `"Standard"` when empty).
    ///
    /// Corresponds to Java: `public void setBaseCalendar(String val)`
    pub fn set_base_calendar(&mut self, val: Option<String>) {
        let value = match val {
            Some(value) if !value.is_empty() => value,
            _ => DEFAULT_BASE_CALENDAR.to_owned(),
        };
        self.set_field_value(key(ResourceField::BaseCalendar), Some(FieldValue::String(value)));
    }

    /// Sets the baseline cost (ignored when reading MS Project files).
    ///
    /// Corresponds to Java: `public void setBaselineCost(Number val)`
    pub fn set_baseline_cost(&mut self, val: Option<f64>) {
        self.set_field_value(
            key(ResourceField::BaselineCost),
            val.map(FieldValue::Double),
        );
    }

    /// Sets the baseline work (ignored when reading MS Project files).
    ///
    /// Corresponds to Java: `public void setBaselineWork(Duration val)`
    pub fn set_baseline_work(&mut self, val: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::BaselineWork),
            val.map(FieldValue::Duration),
        );
    }

    /// Gets the base calendar name associated with this resource.
    ///
    /// Corresponds to Java: `public String getBaseCalendar()`
    pub fn get_base_calendar(&self) -> Option<String> {
        self.get_string(ResourceField::BaseCalendar)
    }

    /// Gets the baseline cost.
    ///
    /// Corresponds to Java: `public Number getBaselineCost()`
    pub fn get_baseline_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::BaselineCost)
    }

    /// Gets the baseline work.
    ///
    /// Corresponds to Java: `public Duration getBaselineWork()`
    pub fn get_baseline_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::BaselineWork)
    }

    /// Sets the "linked fields" flag (ignored when reading MS Project files).
    ///
    /// Corresponds to Java: `public void setLinkedFields(boolean val)`
    pub fn set_linked_fields(&mut self, val: bool) {
        self.set_field_value(
            key(ResourceField::LinkedFields),
            Some(FieldValue::Boolean(val)),
        );
    }

    /// Gets the "linked fields" flag.
    ///
    /// Corresponds to Java: `public boolean getLinkedFields()`
    pub fn get_linked_fields(&self) -> bool {
        self.get_bool(ResourceField::LinkedFields)
    }

    /// Sets the Objects field.
    ///
    /// Corresponds to Java: `public void setObjects(Integer val)`
    pub fn set_objects(&mut self, val: Option<i32>) {
        self.set_field_value(key(ResourceField::Objects), val.map(FieldValue::Integer));
    }

    /// Gets the Objects field.
    ///
    /// Corresponds to Java: `public Integer getObjects()`
    pub fn get_objects(&self) -> Option<i32> {
        self.get_i32(ResourceField::Objects)
    }

    /// Sets the text field (indices 1-30).
    ///
    /// Corresponds to Java: `public void setText(int index, String value)`
    pub fn set_text_at(&mut self, index: i32, value: Option<String>) {
        if let Some(field) = select_field(&CUSTOM_TEXT, index) {
            self.set_field_value(key(field), value.map(FieldValue::String));
        }
    }

    /// Gets the text field (indices 1-30).
    ///
    /// Corresponds to Java: `public String getText(int index)`
    pub fn get_text_at(&self, index: i32) -> Option<String> {
        select_field(&CUSTOM_TEXT, index).and_then(|field| self.get_string(field))
    }

    /// Gets the parent resource's unique ID.
    ///
    /// Corresponds to Java: `public Integer getParentResourceUniqueID()`
    pub fn get_parent_resource_unique_id(&self) -> Option<i32> {
        self.get_i32(ResourceField::ParentId)
    }

    /// Sets the parent resource's unique ID.
    ///
    /// Corresponds to Java: `public void setParentResourceUniqueID(Integer id)`
    pub fn set_parent_resource_unique_id(&mut self, id: Option<i32>) {
        self.set_field_value(key(ResourceField::ParentId), id.map(FieldValue::Integer));
    }

    /// Gets the parent resource.
    ///
    /// Corresponds to Java: `public Resource getParentResource()`
    pub fn get_parent_resource(&self) -> Option<Rc<RefCell<Resource>>> {
        let context = self.context.upgrade()?;
        let context_ref = context.try_borrow().ok()?;
        context_ref
            .get_resources()
            .get_by_unique_id(self.get_parent_resource_unique_id())
    }

    /// Sets the parent resource.
    ///
    /// Corresponds to Java: `public void setParentResource(Resource resource)`
    pub fn set_parent_resource(&mut self, resource: Option<&Rc<RefCell<Resource>>>) {
        let unique_id = resource.and_then(|resource| resource.borrow().get_unique_id());
        self.set_parent_resource_unique_id(unique_id);
    }

    /// Sets the start date field (indices 1-10).
    ///
    /// Corresponds to Java: `public void setStart(int index, LocalDateTime value)`
    pub fn set_start_at(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(&CUSTOM_START, index) {
            self.set_field_value(key(field), value.map(FieldValue::DateTime));
        }
    }

    /// Gets the start date field (indices 1-10).
    ///
    /// Corresponds to Java: `public LocalDateTime getStart(int index)`
    pub fn get_start_at(&self, index: i32) -> Option<NaiveDateTime> {
        select_field(&CUSTOM_START, index).and_then(|field| self.get_date_time(field))
    }

    /// Sets the finish date field (indices 1-10).
    ///
    /// Corresponds to Java: `public void setFinish(int index, LocalDateTime value)`
    pub fn set_finish_at(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(&CUSTOM_FINISH, index) {
            self.set_field_value(key(field), value.map(FieldValue::DateTime));
        }
    }

    /// Gets the finish date field (indices 1-10).
    ///
    /// Corresponds to Java: `public LocalDateTime getFinish(int index)`
    pub fn get_finish_at(&self, index: i32) -> Option<NaiveDateTime> {
        select_field(&CUSTOM_FINISH, index).and_then(|field| self.get_date_time(field))
    }

    /// Sets the number field (indices 1-20).
    ///
    /// Corresponds to Java: `public void setNumber(int index, Number value)`
    pub fn set_number_at(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = select_field(&CUSTOM_NUMBER, index) {
            self.set_field_value(key(field), value.map(FieldValue::Double));
        }
    }

    /// Gets the number field (indices 1-20).
    ///
    /// Corresponds to Java: `public Number getNumber(int index)`
    pub fn get_number_at(&self, index: i32) -> Option<f64> {
        select_field(&CUSTOM_NUMBER, index).and_then(|field| self.get_f64(field))
    }

    /// Sets the duration field (indices 1-10).
    ///
    /// Corresponds to Java: `public void setDuration(int index, Duration value)`
    pub fn set_duration_at(&mut self, index: i32, value: Option<Duration>) {
        if let Some(field) = select_field(&CUSTOM_DURATION, index) {
            self.set_field_value(key(field), value.map(FieldValue::Duration));
        }
    }

    /// Gets the duration field (indices 1-10).
    ///
    /// Corresponds to Java: `public Duration getDuration(int index)`
    pub fn get_duration_at(&self, index: i32) -> Option<Duration> {
        select_field(&CUSTOM_DURATION, index).and_then(|field| self.get_duration(field))
    }

    /// Sets the date field (indices 1-10).
    ///
    /// Corresponds to Java: `public void setDate(int index, LocalDateTime value)`
    pub fn set_date_at(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(&CUSTOM_DATE, index) {
            self.set_field_value(key(field), value.map(FieldValue::DateTime));
        }
    }

    /// Gets the date field (indices 1-10).
    ///
    /// Corresponds to Java: `public LocalDateTime getDate(int index)`
    pub fn get_date_at(&self, index: i32) -> Option<NaiveDateTime> {
        select_field(&CUSTOM_DATE, index).and_then(|field| self.get_date_time(field))
    }

    /// Sets the cost field (indices 1-10).
    ///
    /// Corresponds to Java: `public void setCost(int index, Number value)`
    pub fn set_cost_at(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = select_field(&CUSTOM_COST, index) {
            self.set_field_value(key(field), value.map(FieldValue::Double));
        }
    }

    /// Gets the cost field (indices 1-10).
    ///
    /// Corresponds to Java: `public Number getCost(int index)`
    pub fn get_cost_at(&self, index: i32) -> Option<f64> {
        select_field(&CUSTOM_COST, index).and_then(|field| self.get_f64(field))
    }

    /// Sets the flag field (indices 1-20).
    ///
    /// Corresponds to Java: `public void setFlag(int index, boolean value)`
    pub fn set_flag_at(&mut self, index: i32, value: bool) {
        if let Some(field) = select_field(&CUSTOM_FLAG, index) {
            self.set_field_value(key(field), Some(FieldValue::Boolean(value)));
        }
    }

    /// Gets the flag field (indices 1-20).
    ///
    /// Corresponds to Java: `public boolean getFlag(int index)`
    pub fn get_flag_at(&self, index: i32) -> bool {
        select_field(&CUSTOM_FLAG, index)
            .map(|field| self.get_bool(field))
            .unwrap_or(false)
    }

    /// Sets the outline code field (indices 1-10).
    ///
    /// Corresponds to Java: `public void setOutlineCode(int index, String value)`
    pub fn set_outline_code_at(&mut self, index: i32, value: Option<String>) {
        if let Some(field) = select_field(&CUSTOM_OUTLINE_CODE, index) {
            self.set_field_value(key(field), value.map(FieldValue::String));
        }
    }

    /// Gets the outline code field (indices 1-10).
    ///
    /// Corresponds to Java: `public String getOutlineCode(int index)`
    pub fn get_outline_code_at(&self, index: i32) -> Option<String> {
        select_field(&CUSTOM_OUTLINE_CODE, index).and_then(|field| self.get_string(field))
    }

    /// Removes this resource from the project.
    ///
    /// Corresponds to Java: `public void remove()`
    ///
    /// The Rust side relies on the self weak reference wired up by [`Resource::set_self_reference`]; it is a no-op when not wired up.
    pub fn remove(&mut self) {
        let Some(self_rc) = self.self_ref.upgrade() else {
            return;
        };
        let Some(context) = self.context.upgrade() else {
            return;
        };
        // Note: this cannot be written with `if let` as the final statement of the function — the scrutinee's temporaries are dropped
        // after the function's locals, causing the compiler to decide that `context` does not live long enough.
        let Ok(mut context_ref) = context.try_borrow_mut() else {
            return;
        };
        context_ref.get_resources_mut().remove_resource(&self_rc);
    }

    /// Gets a value by field alias.
    ///
    /// Corresponds to Java: `public Object getFieldByAlias(String alias)`
    pub fn get_field_by_alias(&self, alias: &str) -> Option<FieldValue> {
        let field = self.get_field_key_by_alias(alias)?;
        self.get(field)
    }

    /// Sets a value by field alias.
    ///
    /// Corresponds to Java: `public void setFieldByAlias(String alias, Object value)`
    pub fn set_field_by_alias(&mut self, alias: &str, value: Option<FieldValue>) {
        if let Some(field) = self.get_field_key_by_alias(alias) {
            self.set_field_value(field, value);
        }
    }

    /// Gets all resource assignments assigned to this resource.
    ///
    /// Corresponds to Java: `public List<ResourceAssignment> getTaskAssignments()`
    pub fn get_task_assignments(&self) -> Vec<Rc<RefCell<ResourceAssignment>>> {
        self.get_resource_assignment_stream(self.get_unique_id())
    }

    /// Adds a role assignment for the resource (replacing the existing assignment for that role).
    ///
    /// Corresponds to Java: `public void addRoleAssignment(Resource role, SkillLevel skillLevel)`
    pub fn add_role_assignment(
        &mut self,
        role: &Rc<RefCell<Resource>>,
        skill_level: SkillLevel,
    ) {
        let role_id = NumberHelper::get_int(role.borrow().get_id().map(f64::from));
        self.role_assignments
            .insert(role_id, (Rc::downgrade(role), skill_level));
    }

    /// Removes a role assignment.
    ///
    /// Corresponds to Java: `public void removeRoleAssignment(Resource role)`
    pub fn remove_role_assignment(&mut self, role: &Rc<RefCell<Resource>>) {
        let role_id = NumberHelper::get_int(role.borrow().get_id().map(f64::from));
        self.role_assignments.remove(&role_id);
    }

    /// Gets the role assignment list (Java returns a `Map<Resource, SkillLevel>`; Rust returns a snapshot `Vec`).
    ///
    /// Corresponds to Java: `public Map<Resource, SkillLevel> getRoleAssignments()`
    pub fn get_role_assignments(&self) -> Vec<(Rc<RefCell<Resource>>, SkillLevel)> {
        self.role_assignments
            .values()
            .filter_map(|(role, skill_level)| role.upgrade().map(|role| (role, *skill_level)))
            .collect()
    }

    /// Gets the resource unique ID within the subproject.
    ///
    /// Corresponds to Java: `public Integer getSubprojectResourceUniqueID()`
    pub fn get_subproject_resource_unique_id(&self) -> Option<i32> {
        self.get_i32(ResourceField::SubprojectResourceUniqueId)
    }

    /// Sets the resource unique ID within the subproject.
    ///
    /// Corresponds to Java: `public void setSubprojectResourceUniqueID(Integer subprojectUniqueResourceID)`
    pub fn set_subproject_resource_unique_id(&mut self, value: Option<i32>) {
        self.set_field_value(
            key(ResourceField::SubprojectResourceUniqueId),
            value.map(FieldValue::Integer),
        );
    }

    /// Gets the enterprise cost field (indices 1-10).
    ///
    /// Corresponds to Java: `public Number getEnterpriseCost(int index)`
    pub fn get_enterprise_cost_at(&self, index: i32) -> Option<f64> {
        select_field(&ENTERPRISE_CUSTOM_COST, index).and_then(|field| self.get_f64(field))
    }

    /// Sets the enterprise cost field (indices 1-10).
    ///
    /// Corresponds to Java: `public void setEnterpriseCost(int index, Number value)`
    pub fn set_enterprise_cost_at(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = select_field(&ENTERPRISE_CUSTOM_COST, index) {
            self.set_field_value(key(field), value.map(FieldValue::Double));
        }
    }

    /// Gets the enterprise date field (indices 1-30).
    ///
    /// Corresponds to Java: `public LocalDateTime getEnterpriseDate(int index)`
    pub fn get_enterprise_date_at(&self, index: i32) -> Option<NaiveDateTime> {
        select_field(&ENTERPRISE_CUSTOM_DATE, index).and_then(|field| self.get_date_time(field))
    }

    /// Sets the enterprise date field (indices 1-30).
    ///
    /// Corresponds to Java: `public void setEnterpriseDate(int index, LocalDateTime value)`
    pub fn set_enterprise_date_at(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(&ENTERPRISE_CUSTOM_DATE, index) {
            self.set_field_value(key(field), value.map(FieldValue::DateTime));
        }
    }

    /// Gets the enterprise duration field (indices 1-10).
    ///
    /// Corresponds to Java: `public Duration getEnterpriseDuration(int index)`
    pub fn get_enterprise_duration_at(&self, index: i32) -> Option<Duration> {
        select_field(&ENTERPRISE_CUSTOM_DURATION, index).and_then(|field| self.get_duration(field))
    }

    /// Sets the enterprise duration field (indices 1-10).
    ///
    /// Corresponds to Java: `public void setEnterpriseDuration(int index, Duration value)`
    pub fn set_enterprise_duration_at(&mut self, index: i32, value: Option<Duration>) {
        if let Some(field) = select_field(&ENTERPRISE_CUSTOM_DURATION, index) {
            self.set_field_value(key(field), value.map(FieldValue::Duration));
        }
    }

    /// Gets the enterprise flag field (indices 1-20).
    ///
    /// Corresponds to Java: `public boolean getEnterpriseFlag(int index)`
    pub fn get_enterprise_flag_at(&self, index: i32) -> bool {
        select_field(&ENTERPRISE_CUSTOM_FLAG, index)
            .map(|field| self.get_bool(field))
            .unwrap_or(false)
    }

    /// Sets the enterprise flag field (indices 1-20).
    ///
    /// Corresponds to Java: `public void setEnterpriseFlag(int index, boolean value)`
    pub fn set_enterprise_flag_at(&mut self, index: i32, value: bool) {
        if let Some(field) = select_field(&ENTERPRISE_CUSTOM_FLAG, index) {
            self.set_field_value(key(field), Some(FieldValue::Boolean(value)));
        }
    }

    /// Gets the enterprise number field (indices 1-40).
    ///
    /// Corresponds to Java: `public Number getEnterpriseNumber(int index)`
    pub fn get_enterprise_number_at(&self, index: i32) -> Option<f64> {
        select_field(&ENTERPRISE_CUSTOM_NUMBER, index).and_then(|field| self.get_f64(field))
    }

    /// Sets the enterprise number field (indices 1-40).
    ///
    /// Corresponds to Java: `public void setEnterpriseNumber(int index, Number value)`
    pub fn set_enterprise_number_at(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = select_field(&ENTERPRISE_CUSTOM_NUMBER, index) {
            self.set_field_value(key(field), value.map(FieldValue::Double));
        }
    }

    /// Gets the enterprise text field (indices 1-40).
    ///
    /// Corresponds to Java: `public String getEnterpriseText(int index)`
    pub fn get_enterprise_text_at(&self, index: i32) -> Option<String> {
        select_field(&ENTERPRISE_CUSTOM_TEXT, index).and_then(|field| self.get_string(field))
    }

    /// Sets the enterprise text field (indices 1-40).
    ///
    /// Corresponds to Java: `public void setEnterpriseText(int index, String value)`
    pub fn set_enterprise_text_at(&mut self, index: i32, value: Option<String>) {
        if let Some(field) = select_field(&ENTERPRISE_CUSTOM_TEXT, index) {
            self.set_field_value(key(field), value.map(FieldValue::String));
        }
    }

    /// Sets the baseline cost field (indices 1-10).
    ///
    /// Corresponds to Java: `public void setBaselineCost(int baselineNumber, Number value)`
    pub fn set_baseline_cost_at(&mut self, baseline_number: i32, value: Option<f64>) {
        if let Some(field) = select_field(&BASELINE_COSTS, baseline_number) {
            self.set_field_value(key(field), value.map(FieldValue::Double));
        }
    }

    /// Sets the baseline work field (indices 1-10).
    ///
    /// Corresponds to Java: `public void setBaselineWork(int baselineNumber, Duration value)`
    pub fn set_baseline_work_at(&mut self, baseline_number: i32, value: Option<Duration>) {
        if let Some(field) = select_field(&BASELINE_WORKS, baseline_number) {
            self.set_field_value(key(field), value.map(FieldValue::Duration));
        }
    }

    /// Gets the baseline cost field (indices 1-10).
    ///
    /// Corresponds to Java: `public Number getBaselineCost(int baselineNumber)`
    pub fn get_baseline_cost_at(&self, baseline_number: i32) -> Option<f64> {
        select_field(&BASELINE_COSTS, baseline_number).and_then(|field| self.get_f64(field))
    }

    /// Gets the baseline work field (indices 1-10).
    ///
    /// Corresponds to Java: `public Duration getBaselineWork(int baselineNumber)`
    pub fn get_baseline_work_at(&self, baseline_number: i32) -> Option<Duration> {
        select_field(&BASELINE_WORKS, baseline_number).and_then(|field| self.get_duration(field))
    }

    /// Gets the budget flag.
    ///
    /// Corresponds to Java: `public boolean getBudget()`
    pub fn get_budget(&self) -> bool {
        self.get_bool(ResourceField::Budget)
    }

    /// Sets the budget flag.
    ///
    /// Corresponds to Java: `public void setBudget(boolean budget)`
    pub fn set_budget(&mut self, budget: bool) {
        self.set_field_value(key(ResourceField::Budget), Some(FieldValue::Boolean(budget)));
    }

    /// Gets the resource GUID (parsed from the string encoding on the Rust side).
    ///
    /// Corresponds to Java: `public UUID getGUID()`
    pub fn get_guid(&self) -> Option<Uuid> {
        self.get(key(ResourceField::Guid))
            .and_then(|value| value.as_string().and_then(|text| Uuid::parse_str(text).ok()))
    }

    /// Sets the resource GUID.
    ///
    /// Corresponds to Java: `public void setGUID(UUID value)`
    pub fn set_guid(&mut self, value: Option<Uuid>) {
        self.set_field_value(
            key(ResourceField::Guid),
            value.map(|value| FieldValue::String(value.to_string())),
        );
    }

    /// Sets the units field.
    ///
    /// Corresponds to Java: `public void setUnit(String value)`
    pub fn set_unit(&mut self, value: Option<String>) {
        self.set_field_value(key(ResourceField::Unit), value.map(FieldValue::String));
    }

    /// Gets the units field.
    ///
    /// Corresponds to Java: `public String getUnit()`
    pub fn get_unit(&self) -> Option<String> {
        self.get_string(ResourceField::Unit)
    }

    /// Sets the supply reference field.
    ///
    /// Corresponds to Java: `public void setSupplyReference(String value)`
    pub fn set_supply_reference(&mut self, value: Option<String>) {
        self.set_field_value(
            key(ResourceField::SupplyReference),
            value.map(FieldValue::String),
        );
    }

    /// Gets the supply reference field.
    ///
    /// Corresponds to Java: `public String getSupplyReference()`
    pub fn get_supply_reference(&self) -> Option<String> {
        self.get_string(ResourceField::SupplyReference)
    }

    /// Sets the description field.
    ///
    /// Corresponds to Java: `public void setDescription(String value)`
    pub fn set_description(&mut self, value: Option<String>) {
        self.set_field_value(key(ResourceField::Description), value.map(FieldValue::String));
    }

    /// Gets the description field.
    ///
    /// Corresponds to Java: `public String getDescription()`
    pub fn get_description(&self) -> Option<String> {
        self.get_string(ResourceField::Description)
    }

    /// Sets the resource ID field.
    ///
    /// Corresponds to Java: `public void setResourceID(String value)`
    pub fn set_resource_id(&mut self, value: Option<String>) {
        self.set_field_value(key(ResourceField::ResourceId), value.map(FieldValue::String));
    }

    /// Gets the resource ID field.
    ///
    /// Corresponds to Java: `public String getResourceID()`
    pub fn get_resource_id(&self) -> Option<String> {
        self.get_string(ResourceField::ResourceId)
    }

    /// Sets the modify on integrate field.
    ///
    /// Corresponds to Java: `public void setModifyOnIntegrate(boolean value)`
    pub fn set_modify_on_integrate(&mut self, value: bool) {
        self.set_field_value(
            key(ResourceField::ModifyOnIntegrate),
            Some(FieldValue::Boolean(value)),
        );
    }

    /// Gets the modify on integrate field.
    ///
    /// Corresponds to Java: `public boolean getModifyOnIntegrate()`
    pub fn get_modify_on_integrate(&self) -> bool {
        self.get_bool(ResourceField::ModifyOnIntegrate)
    }

    /// Sets the expenses only field.
    ///
    /// Corresponds to Java: `public void setExpensesOnly(boolean value)`
    pub fn set_expenses_only(&mut self, value: bool) {
        self.set_field_value(
            key(ResourceField::ExpensesOnly),
            Some(FieldValue::Boolean(value)),
        );
    }

    /// Gets the expenses only field.
    ///
    /// Corresponds to Java: `public boolean getExpensesOnly()`
    pub fn get_expenses_only(&self) -> bool {
        self.get_bool(ResourceField::ExpensesOnly)
    }

    /// Sets the period dur field.
    ///
    /// Corresponds to Java: `public void setPeriodDur(Number value)`
    pub fn set_period_dur(&mut self, value: Option<f64>) {
        self.set_field_value(key(ResourceField::PeriodDur), value.map(FieldValue::Double));
    }

    /// Gets the period dur field.
    ///
    /// Corresponds to Java: `public Number getPeriodDur()`
    pub fn get_period_dur(&self) -> Option<f64> {
        self.get_f64(ResourceField::PeriodDur)
    }

    /// Sets the priority field.
    ///
    /// Corresponds to Java: `public void setPriority(Number value)`
    pub fn set_priority(&mut self, value: Option<f64>) {
        self.set_field_value(key(ResourceField::Priority), value.map(FieldValue::Double));
    }

    /// Gets the priority field.
    ///
    /// Corresponds to Java: `public Number getPriority()`
    pub fn get_priority(&self) -> Option<f64> {
        self.get_f64(ResourceField::Priority)
    }

    /// Sets the rate field (TurboProject-specific).
    ///
    /// Corresponds to Java: `public void setRate(Number value)`
    pub fn set_rate(&mut self, value: Option<f64>) {
        self.set_field_value(key(ResourceField::Rate), value.map(FieldValue::Double));
    }

    /// Gets the rate field (TurboProject-specific).
    ///
    /// Corresponds to Java: `public Number getRate()`
    pub fn get_rate(&self) -> Option<f64> {
        self.get_f64(ResourceField::Rate)
    }

    /// Sets the pool field.
    ///
    /// Corresponds to Java: `public void setPool(Number value)`
    pub fn set_pool(&mut self, value: Option<f64>) {
        self.set_field_value(key(ResourceField::Pool), value.map(FieldValue::Double));
    }

    /// Gets the pool field.
    ///
    /// Corresponds to Java: `public Number getPool()`
    pub fn get_pool(&self) -> Option<f64> {
        self.get_f64(ResourceField::Pool)
    }

    /// Sets the per day field.
    ///
    /// Corresponds to Java: `public void setPerDay(Number value)`
    pub fn set_per_day(&mut self, value: Option<f64>) {
        self.set_field_value(key(ResourceField::PerDay), value.map(FieldValue::Double));
    }

    /// Gets the per day field.
    ///
    /// Corresponds to Java: `public Number getPerDay()`
    pub fn get_per_day(&self) -> Option<f64> {
        self.get_f64(ResourceField::PerDay)
    }

    /// Sets the phone field.
    ///
    /// Corresponds to Java: `public void setPhone(String value)`
    pub fn set_phone(&mut self, value: Option<String>) {
        self.set_field_value(key(ResourceField::Phone), value.map(FieldValue::String));
    }

    /// Gets the phone field.
    ///
    /// Corresponds to Java: `public String getPhone()`
    pub fn get_phone(&self) -> Option<String> {
        self.get_string(ResourceField::Phone)
    }

    /// Sets the role flag.
    ///
    /// Corresponds to Java: `public void setRole(boolean value)`
    pub fn set_role(&mut self, value: bool) {
        self.set_field_value(key(ResourceField::Role), Some(FieldValue::Boolean(value)));
    }

    /// Gets the role flag (defaults to `false`).
    ///
    /// Corresponds to Java: `public boolean getRole()`
    pub fn get_role(&self) -> bool {
        self.get_bool(ResourceField::Role)
    }

    /// Associates a complete cost rate table.
    ///
    /// Corresponds to Java: `public void setCostRateTable(int index, CostRateTable crt)`
    ///
    /// Java throws `ArrayIndexOutOfBoundsException` when out of bounds; the Rust side is a no-op when out of bounds.
    pub fn set_cost_rate_table(&mut self, index: i32, crt: CostRateTable) {
        if index >= 0 {
            if let Some(slot) = self.cost_rate_tables.get_mut(index as usize) {
                *slot = crt;
            }
        }
    }

    /// Gets the cost rate table.
    ///
    /// Corresponds to Java: `public CostRateTable getCostRateTable(int index)`
    pub fn get_cost_rate_table(&self, index: i32) -> Option<&CostRateTable> {
        if index < 0 || index >= CostRateTable::MAX_TABLES {
            return None;
        }
        self.cost_rate_tables.get(index as usize)
    }

    /// Gets the cost rate table row in effect on the current date.
    ///
    /// Corresponds to Java: `public CostRateTableEntry getCurrentCostRateTableEntry(int costRateTable)`
    pub fn get_current_cost_rate_table_entry(
        &self,
        cost_rate_table: i32,
    ) -> Option<&CostRateTableEntry> {
        self.get_cost_rate_table(cost_rate_table)
            .and_then(|table| table.get_entry_by_date(now()))
    }

    /// Gets this resource's availability table.
    ///
    /// Corresponds to Java: `public AvailabilityTable getAvailability()`
    pub fn get_availability(&self) -> Ref<'_, AvailabilityTable> {
        self.availability.borrow()
    }

    /// Appends a row to this resource's availability table (Rust-specific).
    ///
    /// Corresponds to Java: `resource.getAvailability().add(entry)`
    ///
    /// Java's `getAvailability()` directly returns the **mutable** `m_availability`, and callers append in place;
    /// on the Rust side [`Resource::get_availability`] only lends out a read-only `Ref`, so this entry point is added as the equivalent write channel.
    /// The MPP/MPX readers all go through this method when filling the availability table (Java's ordering and sorting logic is guaranteed by the caller and is not handled separately here).
    pub fn add_availability(&mut self, entry: Availability) {
        self.availability.get_mut().add(entry);
    }

    /// Gets the availability table row in effect on the current date.
    ///
    /// Corresponds to Java: `public Availability getCurrentAvailabilityTableEntry()`
    pub fn get_current_availability_table_entry(&self) -> Option<Availability> {
        let date = now();
        self.availability
            .try_borrow()
            .ok()
            .and_then(|table| table.get_entry_by_date(date).copied())
    }

    /// Gets the budget cost.
    ///
    /// Corresponds to Java: `public Number getBudgetCost()`
    pub fn get_budget_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::BudgetCost)
    }

    /// Sets the budget cost.
    ///
    /// Corresponds to Java: `public void setBudgetCost(Number value)`
    pub fn set_budget_cost(&mut self, value: Option<f64>) {
        self.set_field_value(key(ResourceField::BudgetCost), value.map(FieldValue::Double));
    }

    /// Gets the budget work.
    ///
    /// Corresponds to Java: `public Duration getBudgetWork()`
    pub fn get_budget_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::BudgetWork)
    }

    /// Sets the budget work.
    ///
    /// Corresponds to Java: `public void setBudgetWork(Duration value)`
    pub fn set_budget_work(&mut self, value: Option<Duration>) {
        self.set_field_value(key(ResourceField::BudgetWork), value.map(FieldValue::Duration));
    }

    /// Gets the baseline budget cost.
    ///
    /// Corresponds to Java: `public Number getBaselineBudgetCost()`
    pub fn get_baseline_budget_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::BaselineBudgetCost)
    }

    /// Sets the baseline budget cost.
    ///
    /// Corresponds to Java: `public void setBaselineBudgetCost(Number value)`
    pub fn set_baseline_budget_cost(&mut self, value: Option<f64>) {
        self.set_field_value(
            key(ResourceField::BaselineBudgetCost),
            value.map(FieldValue::Double),
        );
    }

    /// Gets the baseline budget work.
    ///
    /// Corresponds to Java: `public Duration getBaselineBudgetWork()`
    pub fn get_baseline_budget_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::BaselineBudgetWork)
    }

    /// Sets the baseline budget work.
    ///
    /// Corresponds to Java: `public void setBaselineBudgetWork(Duration value)`
    pub fn set_baseline_budget_work(&mut self, value: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::BaselineBudgetWork),
            value.map(FieldValue::Duration),
        );
    }

    /// Gets the baseline budget cost field (indices 1-10).
    ///
    /// Corresponds to Java: `public Number getBaselineBudgetCost(int baselineNumber)`
    pub fn get_baseline_budget_cost_at(&self, baseline_number: i32) -> Option<f64> {
        select_field(&BASELINE_BUDGET_COSTS, baseline_number).and_then(|field| self.get_f64(field))
    }

    /// Sets the baseline budget cost field (indices 1-10).
    ///
    /// Corresponds to Java: `public void setBaselineBudgetCost(int baselineNumber, Number value)`
    pub fn set_baseline_budget_cost_at(&mut self, baseline_number: i32, value: Option<f64>) {
        if let Some(field) = select_field(&BASELINE_BUDGET_COSTS, baseline_number) {
            self.set_field_value(key(field), value.map(FieldValue::Double));
        }
    }

    /// Gets the baseline budget work field (indices 1-10).
    ///
    /// Corresponds to Java: `public Duration getBaselineBudgetWork(int baselineNumber)`
    pub fn get_baseline_budget_work_at(&self, baseline_number: i32) -> Option<Duration> {
        select_field(&BASELINE_BUDGET_WORKS, baseline_number).and_then(|field| self.get_duration(field))
    }

    /// Sets the baseline budget work field (indices 1-10).
    ///
    /// Corresponds to Java: `public void setBaselineBudgetWork(int baselineNumber, Duration value)`
    pub fn set_baseline_budget_work_at(&mut self, baseline_number: i32, value: Option<Duration>) {
        if let Some(field) = select_field(&BASELINE_BUDGET_WORKS, baseline_number) {
            self.set_field_value(key(field), value.map(FieldValue::Duration));
        }
    }

    /// Gets the cost center.
    ///
    /// Corresponds to Java: `public String getCostCenter()`
    pub fn get_cost_center(&self) -> Option<String> {
        self.get_string(ResourceField::CostCenter)
    }

    /// Sets the cost center.
    ///
    /// Corresponds to Java: `public void setCostCenter(String value)`
    pub fn set_cost_center(&mut self, value: Option<String>) {
        self.set_field_value(key(ResourceField::CostCenter), value.map(FieldValue::String));
    }

    /// Gets the "calculate costs from units" flag (defaults to `true`).
    ///
    /// Corresponds to Java: `public boolean getCalculateCostsFromUnits()`
    pub fn get_calculate_costs_from_units(&self) -> bool {
        self.get_bool(ResourceField::CalculateCostsFromUnits)
    }

    /// Sets the "calculate costs from units" flag.
    ///
    /// Corresponds to Java: `public void setCalculateCostsFromUnits(boolean calculateCostsFromUnits)`
    pub fn set_calculate_costs_from_units(&mut self, calculate_costs_from_units: bool) {
        self.set_field_value(
            key(ResourceField::CalculateCostsFromUnits),
            Some(FieldValue::Boolean(calculate_costs_from_units)),
        );
    }

    /// Gets the resource sequence number.
    ///
    /// Corresponds to Java: `public Integer getSequenceNumber()`
    pub fn get_sequence_number(&self) -> Option<i32> {
        self.get_i32(ResourceField::SequenceNumber)
    }

    /// Sets the resource sequence number.
    ///
    /// Corresponds to Java: `public void setSequenceNumber(Integer sequenceNumber)`
    pub fn set_sequence_number(&mut self, sequence_number: Option<i32>) {
        self.set_field_value(
            key(ResourceField::SequenceNumber),
            sequence_number.map(FieldValue::Integer),
        );
    }

    /// Gets the location unique ID.
    ///
    /// Corresponds to Java: `public Integer getLocationUniqueID()`
    pub fn get_location_unique_id(&self) -> Option<i32> {
        self.get_i32(ResourceField::LocationUniqueId)
    }

    /// Sets the location unique ID.
    ///
    /// Corresponds to Java: `public void setLocationUniqueID(Integer uniqueID)`
    pub fn set_location_unique_id(&mut self, unique_id: Option<i32>) {
        self.set_field_value(
            key(ResourceField::LocationUniqueId),
            unique_id.map(FieldValue::Integer),
        );
    }

    /// Gets the shift unique ID.
    ///
    /// Corresponds to Java: `public Integer getShiftUniqueID()`
    pub fn get_shift_unique_id(&self) -> Option<i32> {
        self.get_i32(ResourceField::ShiftUniqueId)
    }

    /// Sets the shift unique ID.
    ///
    /// Corresponds to Java: `public void setShiftUniqueID(Integer uniqueID)`
    pub fn set_shift_unique_id(&mut self, unique_id: Option<i32>) {
        self.set_field_value(
            key(ResourceField::ShiftUniqueId),
            unique_id.map(FieldValue::Integer),
        );
    }

    /// Gets the shift.
    ///
    /// Corresponds to Java: `public Shift getShift()`
    pub fn get_shift(&self) -> Option<Rc<Shift>> {
        let context = self.context.upgrade()?;
        let context_ref = context.try_borrow().ok()?;
        context_ref
            .get_shifts()
            .get_by_unique_id(self.get_shift_unique_id())
            .cloned()
    }

    /// Sets the shift.
    ///
    /// Corresponds to Java: `public void setShift(Shift shift)`
    pub fn set_shift(&mut self, shift: Option<&Rc<Shift>>) {
        self.set_shift_unique_id(shift.and_then(|shift| shift.get_unique_id()));
    }

    /// Gets the unit of measure unique ID.
    ///
    /// Corresponds to Java: `public Integer getUnitOfMeasureUniqueID()`
    pub fn get_unit_of_measure_unique_id(&self) -> Option<i32> {
        self.get_i32(ResourceField::UnitOfMeasureUniqueId)
    }

    /// Sets the unit of measure unique ID.
    ///
    /// Corresponds to Java: `public void setUnitOfMeasureUniqueID(Integer uniqueID)`
    pub fn set_unit_of_measure_unique_id(&mut self, unique_id: Option<i32>) {
        self.set_field_value(
            key(ResourceField::UnitOfMeasureUniqueId),
            unique_id.map(FieldValue::Integer),
        );
    }

    /// Gets the unit of measure.
    ///
    /// Corresponds to Java: `public UnitOfMeasure getUnitOfMeasure()`
    pub fn get_unit_of_measure(&self) -> Option<UnitOfMeasure> {
        let context = self.context.upgrade()?;
        let context_ref = context.try_borrow().ok()?;
        context_ref
            .get_units_of_measure()
            .get_by_unique_id(self.get_unit_of_measure_unique_id())
            .cloned()
    }

    /// Sets the unit of measure.
    ///
    /// Corresponds to Java: `public void setUnitOfMeasure(UnitOfMeasure unitOfMeasure)`
    pub fn set_unit_of_measure(&mut self, unit_of_measure: Option<&UnitOfMeasure>) {
        self.set_unit_of_measure_unique_id(
            unit_of_measure.and_then(|unit_of_measure| unit_of_measure.get_unique_id()),
        );
    }

    /// Gets the primary role unique ID.
    ///
    /// Corresponds to Java: `public Integer getPrimaryRoleUniqueID()`
    pub fn get_primary_role_unique_id(&self) -> Option<i32> {
        self.get_i32(ResourceField::PrimaryRoleUniqueId)
    }

    /// Sets the primary role unique ID.
    ///
    /// Corresponds to Java: `public void setPrimaryRoleUniqueID(Integer uniqueID)`
    pub fn set_primary_role_unique_id(&mut self, unique_id: Option<i32>) {
        self.set_field_value(
            key(ResourceField::PrimaryRoleUniqueId),
            unique_id.map(FieldValue::Integer),
        );
    }

    /// Gets the primary role.
    ///
    /// Corresponds to Java: `public Resource getPrimaryRole()`
    pub fn get_primary_role(&self) -> Option<Rc<RefCell<Resource>>> {
        let context = self.context.upgrade()?;
        let context_ref = context.try_borrow().ok()?;
        context_ref
            .get_resources()
            .get_by_unique_id(self.get_primary_role_unique_id())
    }

    /// Sets the primary role.
    ///
    /// Corresponds to Java: `public void setPrimaryRole(Resource role)`
    pub fn set_primary_role(&mut self, role: Option<&Rc<RefCell<Resource>>>) {
        self.set_primary_role_unique_id(role.and_then(|role| role.borrow().get_unique_id()));
    }

    /// Gets the currency unique ID.
    ///
    /// Corresponds to Java: `public Integer getCurrencyUniqueID()`
    pub fn get_currency_unique_id(&self) -> Option<i32> {
        self.get_i32(ResourceField::CurrencyUniqueId)
    }

    /// Sets the currency unique ID.
    ///
    /// Corresponds to Java: `public void setCurrencyUniqueID(Integer uniqueID)`
    pub fn set_currency_unique_id(&mut self, unique_id: Option<i32>) {
        self.set_field_value(
            key(ResourceField::CurrencyUniqueId),
            unique_id.map(FieldValue::Integer),
        );
    }

    /// Gets the planned work.
    ///
    /// Corresponds to Java: `public Duration getPlannedWork()`
    pub fn get_planned_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::PlannedWork)
    }

    /// Gets the planned cost.
    ///
    /// Corresponds to Java: `public Number getPlannedCost()`
    pub fn get_planned_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::PlannedCost)
    }

    /// Gets the actual regular work.
    ///
    /// Corresponds to Java: `public Duration getActualRegularWork()`
    pub fn get_actual_regular_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::ActualRegularWork)
    }

    /// Gets the remaining regular work.
    ///
    /// Corresponds to Java: `public Duration getRemainingRegularWork()`
    pub fn get_remaining_regular_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::RemainingRegularWork)
    }

    /// Gets the actual regular cost.
    ///
    /// Corresponds to Java: `public Number getActualRegularCost()`
    pub fn get_actual_regular_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::ActualRegularCost)
    }

    /// Gets the remaining regular cost.
    ///
    /// Corresponds to Java: `public Number getRemainingRegularCost()`
    pub fn get_remaining_regular_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::RemainingRegularCost)
    }

    /// Gets the planned material quantity.
    ///
    /// Corresponds to Java: `public Number getPlannedMaterial()`
    pub fn get_planned_material(&self) -> Option<f64> {
        self.get_f64(ResourceField::PlannedMaterial)
    }

    /// Gets the actual material quantity.
    ///
    /// Corresponds to Java: `public Number getActualMaterial()`
    pub fn get_actual_material(&self) -> Option<f64> {
        self.get_f64(ResourceField::ActualMaterial)
    }

    /// Gets the remaining material quantity.
    ///
    /// Corresponds to Java: `public Number getRemainingMaterial()`
    pub fn get_remaining_material(&self) -> Option<f64> {
        self.get_f64(ResourceField::RemainingMaterial)
    }

    /// Gets the material quantity.
    ///
    /// Corresponds to Java: `public Number getMaterial()`
    pub fn get_material(&self) -> Option<f64> {
        self.get_f64(ResourceField::Material)
    }

    /// Gets the baseline material quantity.
    ///
    /// Corresponds to Java: `public Number getBaselineMaterial()`
    ///
    /// The Java side reads `AssignmentField.BASELINE_MATERIAL` (sic), and that key never appears in
    /// the resource's field table, so it is always `null`; the Rust side keeps the same behavior.
    pub fn get_baseline_material(&self) -> Option<f64> {
        self.get(key_for_assignment_baseline_material())
            .and_then(|value| value.as_double())
    }

    /// Gets the baseline material quantity field (indices 1-10).
    ///
    /// Corresponds to Java: `public Number getBaselineMaterial(int baselineNumber)`
    pub fn get_baseline_material_at(&self, baseline_number: i32) -> Option<f64> {
        select_field(&BASELINE_MATERIALS, baseline_number).and_then(|field| self.get_f64(field))
    }

    /// Sets a field value (corresponds to Java's `set(FieldType, Object)` and `handleFieldChange`).
    ///
    /// Passing `None` is equivalent to passing `null` in Java.
    pub fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
        self.set_field_value(field, value);
    }

    /// Reads a field value (including calculated fields). Corresponds to Java: `Object get(FieldType field)`
    pub fn get(&self, field: FieldKey) -> Option<FieldValue> {
        self.get_field_value(field)
    }

    /// Reads only the stored field value without triggering calculation. Corresponds to Java: `Object getCachedValue(FieldType field)`
    pub fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
        self.fields.get_cached_value(field)
    }

    /// Disables dependent-field cleanup and event firing on field updates.
    ///
    /// Corresponds to Java: `AbstractFieldContainer.disableEvents()`
    pub fn disable_events(&mut self) {
        self.fields.disable_events();
    }

    /// Enables dependent-field cleanup and event firing on field updates (the default state).
    ///
    /// Corresponds to Java: `AbstractFieldContainer.enableEvents()`
    pub fn enable_events(&mut self) {
        self.fields.enable_events();
    }

    /// Resolves a field key by alias.
    ///
    /// Corresponds to Java: `m_context.getResources().getFieldTypeByAlias(alias)`
    fn get_field_key_by_alias(&self, alias: &str) -> Option<FieldKey> {
        let context = self.context.upgrade()?;
        let context_ref = context.try_borrow().ok()?;
        context_ref.get_resources().get_field_type_by_alias(alias)
    }

    /// Actually writes the field value.
    fn set_field_value(&mut self, field: FieldKey, value: Option<FieldValue>) {
        let old_value = self.fields.get_cached_value(field).cloned();
        if old_value == value {
            return;
        }

        // Java's order is "write to field table → handleFieldChange → fire event"; on the Rust side, because
        // the callback closure of `AbstractFieldContainer` cannot mutably borrow the other fields of `Resource` at the same time,
        // it is changed to "handleFieldChange → write and fire event". The handled field logic does not read the field table itself,
        // so the behavior is equivalent.
        self.handle_field_change(field, old_value, value.clone());
        self.fields.set(field, value);
    }

    /// Reads a field value (including calculated fields).
    fn get_field_value(&self, field: FieldKey) -> Option<FieldValue> {
        let always_calculated = is_always_calculated(field);
        let cached = if always_calculated {
            None
        } else {
            self.fields.get_cached_value(field).cloned()
        };

        if cached.is_some() {
            return cached;
        }

        // Java writes the result of non-"always calculated" fields back to the field table; Rust's `&self` cannot write back,
        // so it recomputes each time (same result; see the notes at the top of the file).
        self.calculate(field)
    }

    /// Corresponds to Java: the dispatch implementation of `getCalculationMethod(field).apply(this)`.
    fn calculate(&self, field: FieldKey) -> Option<FieldValue> {
        let resource_field = match field {
            FieldKey::Resource(field) => field,
            _ => return None,
        };

        match resource_field {
            ResourceField::CostVariance => self
                .calculate_cost_variance()
                .map(FieldValue::Double),
            ResourceField::WorkVariance => self
                .calculate_work_variance()
                .map(FieldValue::Duration),
            ResourceField::Cv => Some(FieldValue::Double(self.calculate_cv())),
            ResourceField::Sv => self.calculate_sv().map(FieldValue::Double),
            ResourceField::Overallocated => {
                Some(FieldValue::Boolean(self.calculate_overallocated()))
            }
            ResourceField::StandardRate => {
                self.calculate_standard_rate().map(FieldValue::Rate)
            }
            ResourceField::OvertimeRate => {
                self.calculate_overtime_rate().map(FieldValue::Rate)
            }
            ResourceField::CostPerUse => self.calculate_cost_per_use().map(FieldValue::Double),
            ResourceField::MaterialLabel => {
                self.calculate_material_label().map(FieldValue::String)
            }
            ResourceField::MaxUnits => self.calculate_max_units().map(FieldValue::Double),
            ResourceField::AvailableFrom => {
                self.calculate_available_from().map(FieldValue::DateTime)
            }
            ResourceField::AvailableTo => {
                self.calculate_available_to().map(FieldValue::DateTime)
            }
            ResourceField::Start => self.calculate_start().map(FieldValue::DateTime),
            ResourceField::Finish => self.calculate_finish().map(FieldValue::DateTime),
            ResourceField::PlannedWork => self.calculate_planned_work().map(FieldValue::Duration),
            ResourceField::PlannedCost => self.calculate_planned_cost().map(FieldValue::Double),
            ResourceField::ActualRegularCost => self
                .calculate_regular_cost(self.get_actual_cost(), self.get_actual_overtime_cost())
                .map(FieldValue::Double),
            ResourceField::RemainingRegularCost => self
                .calculate_regular_cost(self.get_remaining_cost(), self.get_remaining_overtime_cost())
                .map(FieldValue::Double),
            ResourceField::ActualRegularWork => self
                .calculate_regular_work(self.get_actual_work(), self.get_actual_overtime_work())
                .map(FieldValue::Duration),
            ResourceField::RemainingRegularWork => self
                .calculate_regular_work(
                    self.get_remaining_work(),
                    self.get_remaining_overtime_work(),
                )
                .map(FieldValue::Duration),
            ResourceField::PlannedMaterial => self
                .calculate_material(self.get_planned_work())
                .map(FieldValue::Double),
            ResourceField::ActualMaterial => self
                .calculate_material(self.get_actual_work())
                .map(FieldValue::Double),
            ResourceField::RemainingMaterial => self
                .calculate_material(self.get_remaining_work())
                .map(FieldValue::Double),
            ResourceField::Material => self
                .calculate_material(self.get_work())
                .map(FieldValue::Double),
            ResourceField::BaselineMaterial => self
                .calculate_material(self.get_baseline_work())
                .map(FieldValue::Double),
            ResourceField::Baseline1Material => self
                .calculate_material(self.get_baseline_work_at(1))
                .map(FieldValue::Double),
            ResourceField::Baseline2Material => self
                .calculate_material(self.get_baseline_work_at(2))
                .map(FieldValue::Double),
            ResourceField::Baseline3Material => self
                .calculate_material(self.get_baseline_work_at(3))
                .map(FieldValue::Double),
            ResourceField::Baseline4Material => self
                .calculate_material(self.get_baseline_work_at(4))
                .map(FieldValue::Double),
            ResourceField::Baseline5Material => self
                .calculate_material(self.get_baseline_work_at(5))
                .map(FieldValue::Double),
            ResourceField::Baseline6Material => self
                .calculate_material(self.get_baseline_work_at(6))
                .map(FieldValue::Double),
            ResourceField::Baseline7Material => self
                .calculate_material(self.get_baseline_work_at(7))
                .map(FieldValue::Double),
            ResourceField::Baseline8Material => self
                .calculate_material(self.get_baseline_work_at(8))
                .map(FieldValue::Double),
            ResourceField::Baseline9Material => self
                .calculate_material(self.get_baseline_work_at(9))
                .map(FieldValue::Double),
            ResourceField::Baseline10Material => self
                .calculate_material(self.get_baseline_work_at(10))
                .map(FieldValue::Double),
            ResourceField::Type => Some(FieldValue::Integer(resource_type_index(
                self.default_type(),
            ))),
            ResourceField::Role => Some(FieldValue::Boolean(self.default_role_flag())),
            ResourceField::CalculateCostsFromUnits => Some(FieldValue::Boolean(
                self.default_calculate_costs_from_units(),
            )),
            ResourceField::Active => Some(FieldValue::Boolean(self.default_active())),
            ResourceField::DefaultUnits => {
                Some(FieldValue::Double(self.default_default_units()))
            }
            // Java returns an empty HashMap; `FieldValue` has no collection variant, so the Rust side returns `None`
            // (see the notes at the top of the file).
            ResourceField::ResourceCodeValues | ResourceField::RoleCodeValues => None,
            _ => None,
        }
    }

    /// Corresponds to Java: `Resource.handleFieldChange(FieldType field, Object oldValue, Object newValue)`
    fn handle_field_change(
        &mut self,
        field: FieldKey,
        old_value: Option<FieldValue>,
        new_value: Option<FieldValue>,
    ) {
        if field == key(ResourceField::UniqueId) {
            let old_id = old_value.as_ref().and_then(FieldValue::as_integer);
            let new_id = new_value.as_ref().and_then(FieldValue::as_integer);

            // Java: m_context.getResources().updateUniqueID(this, oldValue, newValue);
            if let Some(self_rc) = self.self_ref.upgrade() {
                if let Some(context) = self.context.upgrade() {
                    if let Ok(mut context_ref) = context.try_borrow_mut() {
                        context_ref
                            .get_resources_mut()
                            .update_unique_id(&self_rc, old_id, new_id);
                    }
                }
            }

            // Java: getResourceAssignmentStream(oldValue).forEach(a -> a.setResourceUniqueID(newValue));
            for assignment in self.get_resource_assignment_stream(old_id) {
                if let Ok(mut assignment_ref) = assignment.try_borrow_mut() {
                    assignment_ref.set_resource_unique_id(new_id);
                }
            }
            return;
        }

        // Java: clearDependentFields(DEPENDENCY_MAP, field);
        self.fields.clear_dependent_fields(dependency_map(), field);
    }

    /// Calculates the cost variance.
    ///
    /// Corresponds to Java: `Resource.calculateCostVariance()`
    fn calculate_cost_variance(&self) -> Option<f64> {
        let cost = self.get_cost()?;
        let baseline_cost = self.get_baseline_cost()?;
        Some(NumberHelper::get_double(Some(cost - baseline_cost)))
    }

    /// Calculates the work variance.
    ///
    /// Corresponds to Java: `Resource.calculateWorkVariance()`
    fn calculate_work_variance(&self) -> Option<Duration> {
        let work = self.get_work()?;
        let baseline_work = self.get_baseline_work()?;

        let units = work.get_units();
        let baseline_value = match self.context.upgrade() {
            Some(context) => match context.try_borrow() {
                Ok(context_ref) => baseline_work
                    .convert_units(units, context_ref.get_time_unit_defaults())
                    .get_duration(),
                Err(_) => baseline_work.get_duration(),
            },
            None => baseline_work.get_duration(),
        };

        Some(Duration::get_instance(
            work.get_duration() - baseline_value,
            units,
        ))
    }

    /// Calculates the cost variance (CV = BCWP - ACWP).
    ///
    /// Corresponds to Java: `Resource.calculateCV()`
    fn calculate_cv(&self) -> f64 {
        NumberHelper::get_double(self.get_bcwp()) - NumberHelper::get_double(self.get_acwp())
    }

    /// Calculates the schedule variance (SV = BCWP - BCWS).
    ///
    /// Corresponds to Java: `Resource.calculateSV()`
    fn calculate_sv(&self) -> Option<f64> {
        let bcwp = self.get_bcwp()?;
        let bcws = self.get_bcws()?;
        Some(NumberHelper::get_double(Some(bcwp - bcws)))
    }

    /// Calculates whether the resource is over-allocated.
    ///
    /// Corresponds to Java: `Resource.calculateOverallocated()`
    fn calculate_overallocated(&self) -> bool {
        let peak_units = NumberHelper::get_double(self.get_peak_units());
        let max_units = NumberHelper::get_double(self.get_max_units());
        peak_units > max_units
    }

    /// Default value of the resource type.
    ///
    /// Corresponds to Java: `Resource.defaultType()`
    fn default_type(&self) -> ResourceType {
        ResourceType::Work
    }

    /// Default value of the role flag.
    ///
    /// Corresponds to Java: `Resource.defaultRoleFlag()`
    fn default_role_flag(&self) -> bool {
        false
    }

    /// Default value of the "calculate costs from units" flag.
    ///
    /// Corresponds to Java: `Resource.defaultCalculateCostsFromUnits()`
    fn default_calculate_costs_from_units(&self) -> bool {
        true
    }

    /// Default value of the active flag.
    ///
    /// Corresponds to Java: `Resource.defaultActive()`
    fn default_active(&self) -> bool {
        true
    }

    /// Default value of default availability.
    ///
    /// Corresponds to Java: `Resource.defaultDefaultUnits()`
    fn default_default_units(&self) -> f64 {
        DEFAULT_DEFAULT_UNITS
    }

    /// Calculates the standard rate from the current rate table A.
    ///
    /// Corresponds to Java: `Resource.calculateStandardRate()`
    fn calculate_standard_rate(&self) -> Option<crate::rate::Rate> {
        let entry = self.get_current_cost_rate_table_entry(0)?;
        Some(entry.get_standard_rate())
    }

    /// Calculates the overtime rate from the current rate table A.
    ///
    /// Corresponds to Java: `Resource.calculateOvertimeRate()`
    fn calculate_overtime_rate(&self) -> Option<crate::rate::Rate> {
        let entry = self.get_current_cost_rate_table_entry(0)?;
        Some(entry.get_overtime_rate())
    }

    /// Calculates the cost per use from the current rate table A.
    ///
    /// Corresponds to Java: `Resource.calculateCostPerUse()`
    fn calculate_cost_per_use(&self) -> Option<f64> {
        let entry = self.get_current_cost_rate_table_entry(0)?;
        entry.get_cost_per_use()
    }

    /// Calculates the material label (unit abbreviation).
    ///
    /// Corresponds to Java: `Resource.calculateMaterialLabel()`
    fn calculate_material_label(&self) -> Option<String> {
        let unit_of_measure = self.get_unit_of_measure()?;
        unit_of_measure.get_abbreviation().map(str::to_owned)
    }

    /// Calculates the earliest start date of all assigned tasks.
    ///
    /// Corresponds to Java: `Resource.calculateStart()`
    fn calculate_start(&self) -> Option<NaiveDateTime> {
        self.get_resource_assignment_stream(self.get_unique_id())
            .iter()
            .filter_map(|assignment| {
                assignment
                    .try_borrow()
                    .ok()
                    .and_then(|assignment| assignment.get_start())
            })
            .min()
    }

    /// Calculates the latest finish date of all assigned tasks.
    ///
    /// Corresponds to Java: `Resource.calculateFinish()`
    fn calculate_finish(&self) -> Option<NaiveDateTime> {
        self.get_resource_assignment_stream(self.get_unique_id())
            .iter()
            .filter_map(|assignment| {
                assignment
                    .try_borrow()
                    .ok()
                    .and_then(|assignment| assignment.get_finish())
            })
            .max()
    }

    /// Aggregates the planned work of all resource assignments.
    ///
    /// Corresponds to Java: `Resource.calculatePlannedWork()`
    fn calculate_planned_work(&self) -> Option<Duration> {
        let list: Vec<Duration> = self
            .get_resource_assignment_stream(self.get_unique_id())
            .iter()
            .filter_map(|assignment| {
                assignment
                    .try_borrow()
                    .ok()
                    .and_then(|assignment| assignment.get_planned_work())
            })
            .collect();

        let unit = list.first()?.get_units();
        let calendar = self.get_calendar();

        let sum: f64 = list
            .iter()
            .map(|duration| match &calendar {
                Some(calendar) => match calendar.try_borrow() {
                    Ok(calendar_ref) => duration.convert_units(unit, &*calendar_ref).get_duration(),
                    Err(_) => duration.get_duration(),
                },
                None => duration.get_duration(),
            })
            .sum();

        Some(Duration::get_instance(sum, unit))
    }

    /// Aggregates the planned cost of all resource assignments.
    ///
    /// Corresponds to Java: `Resource.calculatePlannedCost()`
    fn calculate_planned_cost(&self) -> Option<f64> {
        let list: Vec<f64> = self
            .get_resource_assignment_stream(self.get_unique_id())
            .iter()
            .filter_map(|assignment| {
                assignment
                    .try_borrow()
                    .ok()
                    .and_then(|assignment| assignment.get_planned_cost())
            })
            .collect();

        if list.is_empty() {
            return None;
        }

        Some(list.iter().sum())
    }

    /// Calculates the available units in the current availability table row.
    ///
    /// Corresponds to Java: `Resource.calculateMaxUnits()`
    fn calculate_max_units(&self) -> Option<f64> {
        let entry = self.get_current_availability_table_entry()?;
        entry.get_units()
    }

    /// Calculates the "available from" date.
    ///
    /// Corresponds to Java: `Resource.calculateAvailableFrom()`
    fn calculate_available_from(&self) -> Option<NaiveDateTime> {
        let date = now();
        self.availability
            .try_borrow_mut()
            .ok()
            .and_then(|mut table| table.available_from(date))
    }

    /// Calculates the "available to" date.
    ///
    /// Corresponds to Java: `Resource.calculateAvailableTo()`
    fn calculate_available_to(&self) -> Option<NaiveDateTime> {
        let date = now();
        self.availability
            .try_borrow_mut()
            .ok()
            .and_then(|mut table| table.available_to(date))
    }

    /// Calculates the regular cost (total cost - overtime cost).
    ///
    /// Corresponds to Java: `Resource.calculateRegularCost(Supplier<Number>, Supplier<Number>)`
    fn calculate_regular_cost(&self, total_cost: Option<f64>, overtime_cost: Option<f64>) -> Option<f64> {
        if total_cost.is_none() && overtime_cost.is_none() {
            return None;
        }

        Some(NumberHelper::get_double(total_cost) - NumberHelper::get_double(overtime_cost))
    }

    /// Calculates the regular work (total work - overtime work).
    ///
    /// Corresponds to Java: `Resource.calculateRegularWork(Supplier<Duration>, Supplier<Duration>)`
    fn calculate_regular_work(
        &self,
        total_work: Option<Duration>,
        overtime_work: Option<Duration>,
    ) -> Option<Duration> {
        if total_work.is_none() && overtime_work.is_none() {
            return None;
        }

        let units = match total_work {
            Some(total_work) => total_work.get_units(),
            None => match overtime_work {
                Some(overtime_work) => overtime_work.get_units(),
                None => return None,
            },
        };

        let calendar = self.get_calendar();
        let convert = |duration: Duration| -> f64 {
            match &calendar {
                Some(calendar) => match calendar.try_borrow() {
                    Ok(calendar_ref) => duration.convert_units(units, &*calendar_ref).get_duration(),
                    Err(_) => duration.get_duration(),
                },
                None => duration.get_duration(),
            }
        };

        let total = total_work.map(convert).unwrap_or(0.0);
        let overtime = overtime_work.map(convert).unwrap_or(0.0);

        Some(Duration::get_instance(total - overtime, units))
    }

    /// Calculates the material quantity (has a value only for material resources).
    ///
    /// Corresponds to Java: `Resource.calculateMaterial(Supplier<Duration>)`
    fn calculate_material(&self, work: Option<Duration>) -> Option<f64> {
        if self.get_type() != ResourceType::Material {
            return None;
        }

        work.map(|work| work.get_duration())
    }

    /// Gets the integer cached value of a single field (without calculation).
    fn get_i32(&self, field: ResourceField) -> Option<i32> {
        self.get(key(field)).and_then(|value| value.as_integer())
    }

    /// Gets the floating-point value of a single field (including calculated fields).
    fn get_f64(&self, field: ResourceField) -> Option<f64> {
        self.get(key(field)).and_then(|value| value.as_double())
    }

    /// Gets the boolean value of a single field (including calculated fields; `null` is treated as `false`).
    fn get_bool(&self, field: ResourceField) -> bool {
        BooleanHelper::get_boolean(self.get(key(field)).and_then(|value| value.as_boolean()))
    }

    /// Gets the string value of a single field (including calculated fields).
    fn get_string(&self, field: ResourceField) -> Option<String> {
        self.get(key(field))
            .and_then(|value| value.as_string().map(str::to_owned))
    }

    /// Gets the duration of a single field (including calculated fields).
    fn get_duration(&self, field: ResourceField) -> Option<Duration> {
        self.get(key(field)).and_then(|value| value.as_duration())
    }

    /// Gets the date/time of a single field (including calculated fields).
    fn get_date_time(&self, field: ResourceField) -> Option<NaiveDateTime> {
        self.get(key(field)).and_then(|value| value.as_date_time())
    }

    /// Gets the rate of a single field (including calculated fields).
    fn get_field_rate(&self, field: ResourceField) -> Option<Rate> {
        self.get(key(field)).and_then(|value| value.as_rate())
    }

    /// Gets resource assignments indexed by unique ID (Java's `getResourceAssignmentStream`).
    fn get_resource_assignment_stream(
        &self,
        id: Option<i32>,
    ) -> Vec<Rc<RefCell<ResourceAssignment>>> {
        let Some(id) = id else {
            return Vec::new();
        };
        let Some(context) = self.context.upgrade() else {
            return Vec::new();
        };
        let projects = match context.try_borrow() {
            Ok(context_ref) => context_ref.get_projects(),
            Err(_) => return Vec::new(),
        };

        let mut result = Vec::new();
        for project in projects {
            if let Ok(project_ref) = project.try_borrow() {
                result.extend(
                    project_ref
                        .get_resource_assignments()
                        .get_by_resource_unique_id(Some(id))
                        .iter()
                        .cloned(),
                );
            }
        }
        result
    }
}

/// The key read by Java's `getBaselineMaterial()` (`AssignmentField.BASELINE_MATERIAL`).
fn key_for_assignment_baseline_material() -> FieldKey {
    FieldKey::Assignment(crate::assignment_field::AssignmentField::BaselineMaterial)
}

impl ProjectEntityWithUniqueID for Resource {
    /// Corresponds to Java: `@Override public Integer getUniqueID()`
    fn get_unique_id(&self) -> Option<i32> {
        self.get(key(ResourceField::UniqueId))
            .and_then(|value| value.as_integer())
    }
}

impl ProjectEntityWithMutableUniqueID for Resource {
    /// Corresponds to Java: `@Override public void setUniqueID(Integer val)`
    fn set_unique_id(&mut self, id: Option<i32>) {
        self.set_field_value(key(ResourceField::UniqueId), id.map(FieldValue::Integer));
    }
}

impl ProjectEntityWithID for Resource {
    /// Corresponds to Java: `@Override public Integer getID()`
    fn get_id(&self) -> Option<i32> {
        self.get(key(ResourceField::Id)).and_then(|value| value.as_integer())
    }

    /// Corresponds to Java: `@Override public void setID(Integer val)`
    ///
    /// Java removes the old ID mapping and registers the new ID mapping before writing the field; the Rust side likewise performs
    /// the two steps before writing (relying on the self weak reference wired up by [`Resource::set_self_reference`]; mapping maintenance is skipped when not wired up).
    fn set_id(&mut self, id: Option<i32>) {
        let previous = self.get_id();
        if let Some(self_rc) = self.self_ref.upgrade() {
            if let Some(context) = self.context.upgrade() {
                if let Ok(mut context_ref) = context.try_borrow_mut() {
                    let resources = context_ref.get_resources_mut();
                    resources.unmap_id(previous);
                    if let Some(id) = id {
                        resources.map_id(id, self_rc);
                    }
                }
            }
        }

        self.set_field_value(key(ResourceField::Id), id.map(FieldValue::Integer));
    }
}

impl FieldContainer for Resource {
    /// Corresponds to Java: `AbstractFieldContainer.set(FieldType, Object)`
    fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
        self.set_field_value(field, value);
    }

    /// Corresponds to Java: `AbstractFieldContainer.get(FieldType)`
    fn get(&self, field: FieldKey) -> Option<FieldValue> {
        self.get_field_value(field)
    }

    /// Corresponds to Java: `AbstractFieldContainer.getCachedValue(FieldType)`
    fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
        self.fields.get_cached_value(field)
    }

    /// Corresponds to Java: `AbstractFieldContainer.addFieldListener(FieldListener)`
    fn add_field_listener(&mut self, listener: Box<dyn FieldListener>) {
        self.fields.add_field_listener(listener);
    }

    /// Corresponds to Java: `AbstractFieldContainer.removeFieldListener(FieldListener)`
    fn clear_field_listeners(&mut self) {
        self.fields.clear_field_listeners();
    }
}

impl PartialEq for Resource {
    /// Corresponds to Java: `@Override public boolean equals(Object o)` (compares by ID)
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Resource {}

impl Ord for Resource {
    /// Corresponds to Java: `@Override public int compareTo(Resource o)`
    fn cmp(&self, other: &Self) -> Ordering {
        let id1 = NumberHelper::get_int(self.get_id().map(f64::from));
        let id2 = NumberHelper::get_int(other.get_id().map(f64::from));
        id1.cmp(&id2)
    }
}

impl PartialOrd for Resource {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Resource {
    /// Corresponds to Java: `@Override public int hashCode()`
    fn hash<H: Hasher>(&self, state: &mut H) {
        NumberHelper::get_int(self.get_id().map(f64::from)).hash(state);
    }
}

impl Display for Resource {
    /// Corresponds to Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[Resource id={} uniqueID={} name={}]",
            optional_display(self.get_id()),
            optional_display(self.get_unique_id()),
            optional_display(self.get_name())
        )
    }
}

impl Debug for Resource {
    /// Java does not override debug output; the Rust side outputs the key identifying fields (`Box<dyn Notes>` and other internal state are not output).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Resource")
            .field("id", &self.get_id())
            .field("unique_id", &self.get_unique_id())
            .field("name", &self.get_name())
            .finish()
    }
}

/// Renders an `Option<T>` the way Java string concatenation does (`null` → `"null"`).
fn optional_display<T: Display>(value: Option<T>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_unit::TimeUnit;

    /// Constructs a resource that is not attached to a context (the factory method is used only in tests).
    fn resource() -> Resource {
        Resource::new(Weak::new())
    }

    /// Constructs a resource attached to a context.
    fn attached_resource(context: &Rc<RefCell<ProjectContext>>) -> Rc<RefCell<Resource>> {
        let resource = Rc::new(RefCell::new(Resource::new(Rc::downgrade(context))));
        resource.borrow_mut().set_self_reference(&resource);
        resource
    }

    #[test]
    fn plain_fields_round_trip_through_the_field_container() {
        let mut resource = resource();

        resource.set_name(Some("Fred".to_owned()));
        resource.set_initials(Some("F".to_owned()));
        resource.set_code(Some("C1".to_owned()));
        resource.set_group(Some("G1".to_owned()));
        resource.set_email_address(Some("fred@example.com".to_owned()));
        resource.set_percent_work_complete(Some(25.0));

        assert_eq!(resource.get_name(), Some("Fred".to_owned()));
        assert_eq!(resource.get_initials(), Some("F".to_owned()));
        assert_eq!(resource.get_code(), Some("C1".to_owned()));
        assert_eq!(resource.get_group(), Some("G1".to_owned()));
        assert_eq!(
            resource.get_email_address(),
            Some("fred@example.com".to_owned())
        );
        assert_eq!(resource.get_percent_work_complete(), Some(25.0));

        // The generic API and the typed API see the same data
        assert_eq!(
            resource.get(key(ResourceField::Name)),
            Some(FieldValue::String("Fred".to_owned()))
        );

        // Passing None is equivalent to passing null in Java; the field is removed
        resource.set_name(None);
        assert_eq!(resource.get_name(), None);
        assert!(resource.get_cached_value(key(ResourceField::Name)).is_none());
    }

    #[test]
    fn id_and_unique_id_are_independent() {
        let mut resource = resource();

        resource.set_id(Some(3));
        resource.set_unique_id(Some(30));

        assert_eq!(resource.get_id(), Some(3));
        assert_eq!(resource.get_unique_id(), Some(30));
    }

    #[test]
    fn enum_backed_fields_round_trip() {
        let mut resource = resource();

        resource.set_type(ResourceType::Material);
        resource.set_accrue_at(AccrueType::End);
        resource.set_work_group(WorkGroup::Email);
        resource.set_booking_type(BookingType::Proposed);

        assert_eq!(resource.get_type(), ResourceType::Material);
        assert_eq!(resource.get_accrue_at(), Some(AccrueType::End));
        assert_eq!(resource.get_work_group(), Some(WorkGroup::Email));
        assert_eq!(resource.get_booking_type(), Some(BookingType::Proposed));
    }

    #[test]
    fn defaults_match_java() {
        let resource = resource();

        assert_eq!(resource.get_type(), ResourceType::Work);
        assert!(!resource.get_role());
        assert!(resource.get_calculate_costs_from_units());
        assert!(resource.get_active());
        assert_eq!(resource.get_default_units(), Some(100.0));
    }

    #[test]
    fn notes_and_guid_are_encoded_as_scalars() {
        let mut resource = resource();

        resource.set_notes(Some("hello".to_owned()));
        assert_eq!(resource.get_notes(), "hello");
        assert_eq!(
            resource
                .get_notes_object()
                .map(|notes| notes.to_string()),
            Some("hello".to_owned())
        );

        let guid = Uuid::from_u128(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef);
        resource.set_guid(Some(guid));
        assert_eq!(resource.get_guid(), Some(guid));

        resource.set_guid(None);
        assert_eq!(resource.get_guid(), None);
    }

    #[test]
    fn availability_and_cost_rate_tables_are_initialised_like_java() {
        let resource = resource();

        // The Java constructor puts one default rate row into each of the 5 tables
        for index in 0..CostRateTable::MAX_TABLES {
            let table = resource.get_cost_rate_table(index).expect("table exists");
            assert_eq!(table.len(), 1);
        }
        assert!(resource.get_cost_rate_table(-1).is_none());
        assert!(resource.get_cost_rate_table(CostRateTable::MAX_TABLES).is_none());

        // The availability table starts out empty
        assert!(resource.get_availability().is_empty());
        assert!(resource.get_current_availability_table_entry().is_none());
    }

    #[test]
    fn availability_table_drives_max_units_and_available_dates() {
        fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
            NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(year, month, day).expect("valid date"),
                chrono::NaiveTime::from_hms_opt(hour, minute, 0).expect("valid time"),
            )
        }

        let resource = resource();
        {
            let mut availability = resource.availability.borrow_mut();
            availability.add(Availability::new(
                Some(date_time(2024, 1, 1, 8, 0)),
                Some(date_time(2024, 1, 31, 17, 0)),
                Some(0.5),
            ));
        }

        // The current date falls outside the interval (January 2024 has passed), so no "current" row is found
        assert!(resource.get_current_availability_table_entry().is_none());
        // The target date is later than the only interval: availableFrom returns the end of that interval + 1 minute, and availableTo has no value
        assert_eq!(
            resource.get_available_from(),
            Some(date_time(2024, 1, 31, 17, 1))
        );
        assert_eq!(resource.get_available_to(), None);
    }

    #[test]
    fn custom_rate_table_drives_standard_rate() {
        use crate::cost_rate_table_entry::CostRateTableEntry as Entry;
        use crate::rate::Rate;

        let mut resource = resource();
        let mut table = CostRateTable::new();
        table.add(Entry::new(
            None,
            Some(now() + chrono::Duration::days(1)),
            Some(4.0),
            &[Rate::new(12.0, TimeUnit::Hours), Rate::new(18.0, TimeUnit::Hours)],
        ));
        resource.set_cost_rate_table(0, table);

        assert_eq!(
            resource.get_standard_rate(),
            Some(Rate::new(12.0, TimeUnit::Hours))
        );
        assert_eq!(
            resource.get_overtime_rate(),
            Some(Rate::new(18.0, TimeUnit::Hours))
        );
        assert_eq!(resource.get_cost_per_use(), Some(4.0));
    }

    #[test]
    fn cost_and_work_variances_are_calculated() {
        let mut resource = resource();

        resource.set_cost(Some(150.0));
        resource.set_baseline_cost(Some(100.0));
        assert_eq!(resource.get_cost_variance(), Some(50.0));

        resource.set_work(Some(Duration::get_instance(10.0, TimeUnit::Days)));
        resource.set_baseline_work(Some(Duration::get_instance(8.0, TimeUnit::Days)));
        assert_eq!(
            resource.get_work_variance(),
            Some(Duration::get_instance(2.0, TimeUnit::Days))
        );
    }

    #[test]
    fn earned_value_fields_are_calculated() {
        let mut resource = resource();

        resource.set_bcwp(Some(80.0));
        resource.set_acwp(Some(60.0));
        resource.set_bcws(Some(70.0));

        assert_eq!(resource.get_cv(), Some(20.0));
        assert_eq!(resource.get_sv(), Some(10.0));

        // An explicitly set value takes precedence over the calculated value (consistent with Java's cache semantics)
        resource.set_cv(Some(1.0));
        assert_eq!(resource.get_cv(), Some(1.0));
    }

    #[test]
    fn overallocated_compares_peak_and_max_units() {
        let mut resource = resource();

        // Java does not provide a setMaxUnits (MAX_UNITS is calculated from the availability table); with no availability table it is treated as 0.
        resource.set_peak_units(Some(120.0));
        assert!(resource.get_over_allocated());

        resource.set_peak_units(Some(0.0));
        assert!(!resource.get_over_allocated());
    }

    #[test]
    fn material_calculations_require_a_material_resource() {
        let mut resource = resource();

        resource.set_work(Some(Duration::get_instance(6.0, TimeUnit::Hours)));
        resource.set_baseline_work(Some(Duration::get_instance(4.0, TimeUnit::Hours)));

        // The default (WORK) resource has no material quantity
        assert_eq!(resource.get_material(), None);

        resource.set_type(ResourceType::Material);
        assert_eq!(resource.get_material(), Some(6.0));
        resource.set_baseline_work_at(1, Some(Duration::get_instance(4.0, TimeUnit::Hours)));
        assert_eq!(resource.get_baseline_material_at(1), Some(4.0));
    }

    #[test]
    fn regular_work_and_cost_are_calculated() {
        let mut resource = resource();

        resource.set_actual_cost(Some(30.0));
        resource.set_actual_overtime_cost(Some(10.0));
        assert_eq!(resource.get_actual_regular_cost(), Some(20.0));

        resource.set_actual_work(Some(Duration::get_instance(5.0, TimeUnit::Days)));
        resource.set_actual_overtime_work(Some(Duration::get_instance(1.0, TimeUnit::Days)));
        assert_eq!(
            resource.get_actual_regular_work(),
            Some(Duration::get_instance(4.0, TimeUnit::Days))
        );
    }

    #[test]
    fn dependent_fields_are_cleared_when_dependencies_change() {
        let mut resource = resource();

        // Triggers a calculation and confirms it can be calculated again
        resource.set_cost(Some(10.0));
        resource.set_baseline_cost(Some(4.0));
        assert_eq!(resource.get_cost_variance(), Some(6.0));

        // After a dependent field changes the cache is cleared, and recomputation produces the new value
        resource.set_cost(Some(20.0));
        assert_eq!(resource.get_cost_variance(), Some(16.0));
    }

    #[test]
    fn indexed_field_accessors_round_trip() {
        let mut resource = resource();

        resource.set_text_at(1, Some("T1".to_owned()));
        resource.set_number_at(20, Some(20.0));
        resource.set_duration_at(3, Some(Duration::get_instance(3.0, TimeUnit::Hours)));
        resource.set_cost_at(10, Some(10.0));
        resource.set_flag_at(2, true);
        resource.set_outline_code_at(1, Some("OC".to_owned()));

        assert_eq!(resource.get_text_at(1), Some("T1".to_owned()));
        assert_eq!(resource.get_number_at(20), Some(20.0));
        assert_eq!(
            resource.get_duration_at(3),
            Some(Duration::get_instance(3.0, TimeUnit::Hours))
        );
        assert_eq!(resource.get_cost_at(10), Some(10.0));
        assert!(resource.get_flag_at(2));
        assert_eq!(resource.get_outline_code_at(1), Some("OC".to_owned()));

        // An out-of-bounds index does not panic
        assert_eq!(resource.get_text_at(0), None);
        assert_eq!(resource.get_text_at(31), None);
        assert!(!resource.get_flag_at(21));
    }

    #[test]
    fn baseline_and_enterprise_indexed_fields_round_trip() {
        let mut resource = resource();

        resource.set_baseline_cost_at(1, Some(11.0));
        resource.set_baseline_work_at(10, Some(Duration::get_instance(7.0, TimeUnit::Days)));
        resource.set_baseline_budget_cost_at(2, Some(22.0));
        resource.set_baseline_budget_work_at(
            2,
            Some(Duration::get_instance(2.0, TimeUnit::Days)),
        );
        resource.set_enterprise_cost_at(1, Some(33.0));
        resource.set_enterprise_text_at(40, Some("E40".to_owned()));

        assert_eq!(resource.get_baseline_cost_at(1), Some(11.0));
        assert_eq!(
            resource.get_baseline_work_at(10),
            Some(Duration::get_instance(7.0, TimeUnit::Days))
        );
        assert_eq!(resource.get_baseline_budget_cost_at(2), Some(22.0));
        assert_eq!(
            resource.get_baseline_budget_work_at(2),
            Some(Duration::get_instance(2.0, TimeUnit::Days))
        );
        assert_eq!(resource.get_enterprise_cost_at(1), Some(33.0));
        assert_eq!(resource.get_enterprise_text_at(40), Some("E40".to_owned()));
    }

    #[test]
    fn child_resources_are_tracked() {
        let mut parent = resource();
        let child = Rc::new(RefCell::new(resource()));

        parent.add_child_resource(&child);
        assert_eq!(parent.get_child_resources().len(), 1);

        parent.remove_child_resource(&child);
        assert!(parent.get_child_resources().is_empty());
    }

    #[test]
    fn role_assignments_are_keyed_by_role_id() {
        let role = Rc::new(RefCell::new(resource()));
        role.borrow_mut().set_id(Some(7));
        let mut resource = resource();

        resource.add_role_assignment(&role, SkillLevel::Expert);
        let assignments = resource.get_role_assignments();
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].1, SkillLevel::Expert);

        // Assigning the same role repeatedly replaces the previous assignment
        resource.add_role_assignment(&role, SkillLevel::Master);
        assert_eq!(resource.get_role_assignments()[0].1, SkillLevel::Master);

        resource.remove_role_assignment(&role);
        assert!(resource.get_role_assignments().is_empty());
    }

    #[test]
    fn ordering_equality_and_display_follow_the_id() {
        let mut first = resource();
        let mut second = resource();

        first.set_id(Some(1));
        first.set_name(Some("A".to_owned()));
        second.set_id(Some(2));
        second.set_name(Some("B".to_owned()));

        assert!(first < second);
        assert_ne!(first, second);
        assert_eq!(first, {
            let mut same = resource();
            same.set_id(Some(1));
            same
        });

        assert_eq!(
            first.to_string(),
            "[Resource id=1 uniqueID=null name=A]"
        );
        assert_eq!(format!("{first:?}").contains("Resource"), true);
    }

    #[test]
    fn context_backed_accessors_degrade_gracefully_without_context() {
        let mut resource = resource();

        assert!(resource.get_project_context().is_none());
        assert!(resource.get_calendar().is_none());
        assert!(resource.get_parent_resource().is_none());
        assert!(resource.get_shift().is_none());
        assert!(resource.get_unit_of_measure().is_none());
        assert!(resource.get_primary_role().is_none());
        assert!(resource.get_task_assignments().is_empty());
        assert!(resource.add_resource().is_none());
        assert!(resource.add_calendar().is_err());

        // remove() is a no-op without a self weak reference
        resource.remove();
    }

    #[test]
    fn attached_resource_uses_context_containers() {
        let context = ProjectContext::shared();
        let resource = attached_resource(&context);
        let child = attached_resource(&context);

        {
            let mut context_ref = context.borrow_mut();
            let _calendar = context_ref.get_calendars_mut().add();
            context_ref
                .get_resources_mut()
                .add_resource_entry(Rc::clone(&resource));
            context_ref
                .get_resources_mut()
                .add_resource_entry(Rc::clone(&child));
        }

        // The resource is associated with a context; when no calendar is associated, get_calendar returns None
        assert!(resource.borrow().get_project_context().is_some());
        assert!(resource.borrow().get_calendar().is_none());

        // A unique ID change is synchronized to the container's unique ID index
        resource.borrow_mut().set_unique_id(Some(5));
        child.borrow_mut().set_unique_id(Some(6));

        // The parent resource is resolved by unique ID through the context's resource container
        child.borrow_mut().set_parent_resource(Some(&resource));
        assert!(Rc::ptr_eq(
            &child.borrow().get_parent_resource().expect("parent resource registered"),
            &resource
        ));
    }

    #[test]
    fn display_helper_renders_none_as_null() {
        assert_eq!(optional_display(None::<i32>), "null");
        assert_eq!(optional_display(Some(5)), "5");
    }
}
