//! 对应 Java: `org/mpxj/Resource.java`
//!
//! # 与 Java 的差异
//!
//! - **字段存储（Java 继承 → Rust 组合）**：Java 的 `Resource extends AbstractFieldContainer<Resource>`，
//!   `set` / `get` / `getCachedValue` 由基类提供；Rust 无继承，本类持有一个
//!   [`AbstractFieldContainer`]，由 [`Resource::set`] / [`Resource::get`] /
//!   [`Resource::get_cached_value`] 转发（见 [`crate::abstract_field_container`] 的说明）。
//!   Java 的三个抽象方法在 Rust 中的落位：`handleFieldChange` →
//!   [`Resource::handle_field_change`]；`getAlwaysCalculatedField` →
//!   [`is_always_calculated`]；`getCalculationMethod` → [`Resource::calculate`] 的 `match`。
//! - **无法用 `FieldValue` 表达的值**：[`crate::field_value::FieldValue`] 目前只有标量变体
//!   （无枚举 / `Notes` / `UUID` / 集合变体），且不得修改既有文件，故下列 Java 字段在字段
//!   容器中改用等价的标量编码，类型化访问器负责转换：
//!
//!   | Java 字段 | Rust 编码 |
//!   |---|---|
//!   | `ResourceField.TYPE`（`ResourceType`） | `FieldValue::Integer`（声明顺序下标，`WORK` = 1） |
//!   | `ResourceField.ACCRUE_AT`（`AccrueType`） | `FieldValue::Integer`（`getValue()`） |
//!   | `ResourceField.WORKGROUP`（`WorkGroup`） | `FieldValue::Integer`（`getValue()`） |
//!   | `ResourceField.BOOKING_TYPE`（`BookingType`） | `FieldValue::Integer`（`getValue()`） |
//!   | `ResourceField.NOTES`（`Notes`） | `FieldValue::String`（`Notes.toString()`） |
//!   | `ResourceField.GUID`（`UUID`） | `FieldValue::String`（`UUID.toString()`） |
//!
//!   因此 [`Resource::get_notes_object`] 只能返回纯文本 [`PlainNotes`]，
//!   [`Resource::set_notes_object`] 也只能保留纯文本；`get(GUID)` 返回字符串而非 `Uuid`。
//!   `ResourceField.RESOURCE_CODE_VALUES` / `ROLE_CODE_VALUES`（`Map<ResourceCode, ...>`）
//!   无对应类型，未移植（见下）。
//! - **构造方法与自动编号**：Java 的 `Resource(ProjectContext)` 在构造时按配置分配
//!   unique ID 与 ID。Rust 侧 `&mut self` 无法取得自身的 `Rc`（`mapID` / `updateUniqueID`
//!   都需要元素句柄），且容器方法总在上下文被 `borrow_mut` 时调用。因此 Rust 侧
//!   [`Resource::new`] 只做字段初始化，自动编号由 [`crate::resource_container::ResourceContainer::add`]
//!   用容器自带的序号生成器完成（见该文件的说明）。
//! - **反向引用**：Java 的 `private final ProjectContext m_context` 是强引用；Rust 侧改为
//!   `Weak<RefCell<ProjectContext>>`。另新增 Rust 专有字段 `self_ref`
//!   （`Weak<RefCell<Resource>>`），供 `handleFieldChange` 在 unique ID / ID 变化时回调容器
//!   （Java 用 `this`），由容器在包装 `Rc` 后通过 `set_self_reference` 接线。
//! - **集合**：`List<Resource>` → `Vec<Rc<RefCell<Resource>>>`；
//!   `Map<Resource, SkillLevel>` → `HashMap<i32, (Weak<RefCell<Resource>>, SkillLevel)>`
//!   （键取 `NumberHelper.getInt(role.getID())`，与 Java 以 ID 为 `equals`/`hashCode` 等价）；
//!   [`Resource::get_role_assignments`] 返回 `Vec<(Rc<RefCell<Resource>>, SkillLevel)>` 快照。
//! - **AvailabilityTable**：Java 直接持有可变对象；Rust 侧用 `RefCell<AvailabilityTable>`
//!   包一层（`availableFrom` / `availableTo` 需要 `&mut self`），故
//!   [`Resource::get_availability`] 返回 `Ref<'_, AvailabilityTable>`。
//! - **`get` 不写回缓存**：Java 的 `get` 会把非「始终计算」字段的计算结果 `set` 回字段表；
//!   Rust 的 `get(&self)` 无法写回，改为每次重新计算（结果相同，仅少一次缓存写入）。
//! - **索引型重载**：Rust 无重载，所有按索引取值/设值的 Java 方法（如 `getCost(int)`、
//!   `setBaselineWork(int, Duration)`）统一加 `_at` 后缀（如 [`Resource::get_cost_at`]、
//!   [`Resource::set_baseline_work_at`]）。
//! - **`selectField`**：Java 越界抛 `IllegalArgumentException`；Rust 侧返回 `None`，
//!   对应的访问器返回 `None` / 保持原值（不 panic）。
//! - **`compareTo`/`equals`/`hashCode`/`toString`** → `Ord`/`PartialEq`+`Eq`/`Hash`/`Display`。
//!
//! # 跳过的 Java 方法（及原因）
//!
//! | Java | 原因 |
//! |---|---|
//! | `getTimephasedDurationValues` / `getTimephasedNumericValues` | 依赖未在 `src/lib.rs` 声明的 `TimephasedDataFactory`（同 [`crate::field_container`] 的取舍） |
//! | `getTimephasedPlannedWork` 等全部 `getTimephased*`、`reduceTimephasedWork` / `reduceTimephasedCost` / `reduceTimephasedMaterial`、`TIMEPHASED_WORK_FUNCTIONS` / `TIMEPHASED_NUMERIC_FUNCTIONS` | 依赖未在 `src/lib.rs` 声明的 `timephased_utility`（`TimephasedUtility.addTimephasedDurations` / `addTimephasedNumbers`）与 `ResourceAssignment` 的时间分段 API |
//! | `getLocation` / `setLocation` | `Location` 未在 `src/lib.rs` 声明 |
//! | `getCurrency` / `setCurrency` | `Currency` 未在 `src/lib.rs` 声明 |
//! | `getResourceCodeValues` / `addResourceCodeValue` / `getRoleCodeValues` / `addRoleCodeValue` | `ResourceCode` / `RoleCode` / `ResourceCodeValue` / `RoleCodeValue` 未在 `src/lib.rs` 声明，`FieldValue` 亦无集合变体 |
//!
//! 被跳过的 Java 机制：反射、`Serializable`（本类未使用）。
//!
//! # 对并行模块的接口假设
//!
//! - `crate::resource_assignment::ResourceAssignment`：`get_start()` / `get_finish()` →
//!   `Option<NaiveDateTime>`，`get_planned_work()` → `Option<Duration>`，
//!   `get_planned_cost()` → `Option<f64>`，`set_resource_unique_id(Option<i32>)`。
//! - `crate::resource_assignment_container::ResourceAssignmentContainer`：
//!   `get_by_resource_unique_id(Option<i32>) -> Vec<Rc<RefCell<ResourceAssignment>>>`。
//! - `crate::project_file::ProjectFile`：`get_resource_assignments() -> &ResourceAssignmentContainer`。

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
// 注：`TimeUnit` 仅在测试中被显式命名，主代码中的单位类型由 `Duration` 推导得出。
use crate::work_group::WorkGroup;

/// 成本费率表数量（对应 Java: `CostRateTable.MAX_TABLES`）。
const MAX_TABLES: usize = 5;

/// `Resource.addCalendar` 中使用的默认资源日历名称。
///
/// 对应 Java: `"Unnamed Resource"`。
const UNNAMED_RESOURCE: &str = "Unnamed Resource";

/// 默认基准日历名称。
///
/// 对应 Java: `Resource.setBaseCalendar` 中的字面量 `"Standard"`。
const DEFAULT_BASE_CALENDAR: &str = "Standard";

/// 默认的默认可用性（100%）。
///
/// 对应 Java: `private static final Number DEFAULT_DEFAULT_UNITS = NumberHelper.DOUBLE_ONEHUNDRED;`
const DEFAULT_DEFAULT_UNITS: f64 = NumberHelper::DOUBLE_ONEHUNDRED;

/// 把 `ResourceField` 包装成统一的 [`FieldKey`]。
fn key(field: ResourceField) -> FieldKey {
    FieldKey::Resource(field)
}

/// 对应 Java: `LocalDateTime.now()`。
fn now() -> NaiveDateTime {
    Local::now().naive_local()
}

/// 对应 Java: `Resource.selectField(ResourceField[] fields, int index)`。
///
/// Java 越界抛 `IllegalArgumentException`；Rust 侧返回 `None`。
fn select_field(fields: &[ResourceField], index: i32) -> Option<ResourceField> {
    if index < 1 || index as usize > fields.len() {
        return None;
    }
    fields.get(index as usize - 1).copied()
}

/// 对应 Java: `ResourceFieldLists.CUSTOM_TEXT`（下标 1-30）。
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

/// 对应 Java: `ResourceFieldLists.CUSTOM_START`（下标 1-10）。
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

/// 对应 Java: `ResourceFieldLists.CUSTOM_FINISH`（下标 1-10）。
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

/// 对应 Java: `ResourceFieldLists.CUSTOM_NUMBER`（下标 1-20）。
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

/// 对应 Java: `ResourceFieldLists.CUSTOM_DURATION`（下标 1-10）。
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

/// 对应 Java: `ResourceFieldLists.CUSTOM_DATE`（下标 1-10）。
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

/// 对应 Java: `ResourceFieldLists.CUSTOM_COST`（下标 1-10）。
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

/// 对应 Java: `ResourceFieldLists.CUSTOM_FLAG`（下标 1-20）。
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

/// 对应 Java: `ResourceFieldLists.CUSTOM_OUTLINE_CODE`（下标 1-10）。
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

/// 对应 Java: `ResourceFieldLists.ENTERPRISE_CUSTOM_COST`（下标 1-10）。
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

/// 对应 Java: `ResourceFieldLists.ENTERPRISE_CUSTOM_DATE`（下标 1-30）。
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

/// 对应 Java: `ResourceFieldLists.ENTERPRISE_CUSTOM_DURATION`（下标 1-10）。
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

/// 对应 Java: `ResourceFieldLists.ENTERPRISE_CUSTOM_FLAG`（下标 1-20）。
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

/// 对应 Java: `ResourceFieldLists.ENTERPRISE_CUSTOM_NUMBER`（下标 1-40）。
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

/// 对应 Java: `ResourceFieldLists.ENTERPRISE_CUSTOM_TEXT`（下标 1-40）。
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

/// 对应 Java: `ResourceFieldLists.BASELINE_COSTS`（下标 1-10）。
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

/// 对应 Java: `ResourceFieldLists.BASELINE_WORKS`（下标 1-10）。
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

/// 对应 Java: `ResourceFieldLists.BASELINE_BUDGET_COSTS`（下标 1-10）。
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

/// 对应 Java: `ResourceFieldLists.BASELINE_BUDGET_WORKS`（下标 1-10）。
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

/// 对应 Java: `ResourceFieldLists.BASELINE_MATERIALS`（下标 1-10）。
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

/// 对应 Java: `Resource.ALWAYS_CALCULATED_FIELDS`。
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

/// 对应 Java: `Resource.DEPENDENCY_MAP`（由 `FieldContainerDependencies` 静态块构建）。
///
/// 键是「被修改的字段」，值是需要清理缓存的「计算字段」列表。
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

/// `ResourceType` 的存储下标（声明顺序）。
fn resource_type_index(value: ResourceType) -> i32 {
    match value {
        ResourceType::Material => 0,
        ResourceType::Work => 1,
        ResourceType::Cost => 2,
        ResourceType::NonLabor => 3,
    }
}

/// 项目中的资源。
///
/// 对应 Java: `public final class Resource extends AbstractFieldContainer<Resource> implements Comparable<Resource>, ProjectEntityWithID, ChildResourceContainer`
pub struct Resource {
    /// 父上下文（弱引用，避免 `Rc` 循环）。
    ///
    /// 对应 Java: `private final ProjectContext m_context;`
    context: Weak<RefCell<ProjectContext>>,

    /// 自身的弱引用（Rust 专有）。
    ///
    /// 对应 Java 中 `handleFieldChange` 里的 `this`：unique ID / ID 变化时需要把自身句柄
    /// 交给容器（`updateUniqueID` / `mapID`）。由
    /// [`crate::resource_container::ResourceContainer`] 在包装 `Rc` 后接线。
    self_ref: Weak<RefCell<Resource>>,

    /// 字段存储（Java 的继承基类 `AbstractFieldContainer`）。
    fields: AbstractFieldContainer,

    /// 是否为空资源。
    ///
    /// 对应 Java: `private boolean m_null;`
    null: bool,

    /// Active Directory GUID。
    ///
    /// 对应 Java: `private String m_activeDirectoryGUID;`
    active_directory_guid: Option<String>,

    /// 角色分配（角色 ID → 角色与技能等级）。
    ///
    /// 对应 Java: `private final Map<Resource, SkillLevel> m_roleAssignments = new HashMap<>();`
    role_assignments: HashMap<i32, (Weak<RefCell<Resource>>, SkillLevel)>,

    /// 子资源。
    ///
    /// 对应 Java: `private final List<Resource> m_children = new ArrayList<>();`
    children: Vec<Rc<RefCell<Resource>>>,

    /// 成本费率表（A-E）。
    ///
    /// 对应 Java: `private final CostRateTable[] m_costRateTables;`
    cost_rate_tables: Vec<CostRateTable>,

    /// 可用性表。
    ///
    /// 对应 Java: `private final AvailabilityTable m_availability = new AvailabilityTable();`
    ///
    /// Rust 侧用 `RefCell` 包一层，因为 `availableFrom` / `availableTo` 需要 `&mut self`
    /// （见文件头说明）。
    availability: RefCell<AvailabilityTable>,
}

impl Resource {
    /// 构造方法。
    ///
    /// 对应 Java: `Resource(ProjectContext context)`
    ///
    /// Rust 侧只做字段初始化；Java 在构造方法中按配置分配的 unique ID / ID 由
    /// [`crate::resource_container::ResourceContainer::add`] 完成（见文件头说明）。
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

    /// 接线自身的弱引用（Rust 专有，供容器在包装 `Rc` 后调用）。
    ///
    /// 对应 Java: `handleFieldChange` 中使用的 `this`。
    pub(crate) fn set_self_reference(&mut self, resource: &Rc<RefCell<Resource>>) {
        self.self_ref = Rc::downgrade(resource);
    }

    /// 取父上下文（Rust 侧返回升级后的强引用，Java 返回强引用字段）。
    ///
    /// 对应 Java: `public ProjectContext getProjectContext()`
    pub fn get_project_context(&self) -> Option<Rc<RefCell<ProjectContext>>> {
        self.context.upgrade()
    }

    /// 创建并加入一个子资源。
    ///
    /// 对应 Java: `@Override public Resource addResource()`
    ///
    /// Java 依赖非空的 `m_context`；Rust 侧上下文不可用（未接线或已被可变借用）时返回
    /// `None`（不 panic）。
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

    /// 把已有资源作为当前资源的子资源加入。
    ///
    /// 对应 Java: `public void addChildResource(Resource child)`
    ///
    /// Java 的 `child.setParentResource(this)` 写入的是父资源的 unique ID；Rust 侧
    /// `&mut self` 无需自身 `Rc` 即可取得该值，故直接写入。
    pub fn add_child_resource(&mut self, child: &Rc<RefCell<Resource>>) {
        let parent_unique_id = self.get_unique_id();
        child.borrow_mut().set_parent_resource_unique_id(parent_unique_id);
        self.children.push(Rc::clone(child));
    }

    /// 取子资源列表（Java 返回实时列表，Rust 返回快照）。
    ///
    /// 对应 Java: `@Override public List<Resource> getChildResources()`
    pub fn get_child_resources(&self) -> Vec<Rc<RefCell<Resource>>> {
        self.children.clone()
    }

    /// 取子资源列表的可变引用（Rust 专有，供容器 `updateStructure` 的
    /// `getChildResources().clear()` 使用）。
    pub fn get_child_resources_mut(&mut self) -> &mut Vec<Rc<RefCell<Resource>>> {
        &mut self.children
    }

    /// 移除一个子资源。
    ///
    /// 对应 Java: `public void removeChildResource(Resource child)`
    pub fn remove_child_resource(&mut self, child: &Rc<RefCell<Resource>>) {
        let before = self.children.len();
        self.children.retain(|item| !Rc::ptr_eq(item, child));
        if before != self.children.len() {
            child.borrow_mut().set_parent_resource_unique_id(None);
        }
    }

    /// 设置 Name 字段。
    ///
    /// 对应 Java: `public void setName(String val)`
    pub fn set_name(&mut self, val: Option<String>) {
        self.set_field_value(key(ResourceField::Name), val.map(FieldValue::String));
    }

    /// 取 Name 字段。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> Option<String> {
        self.get_string(ResourceField::Name)
    }

    /// 设置资源类型。
    ///
    /// 对应 Java: `public void setType(ResourceType type)`
    pub fn set_type(&mut self, value: ResourceType) {
        self.set_field_value(
            key(ResourceField::Type),
            Some(FieldValue::Integer(resource_type_index(value))),
        );
    }

    /// 取资源类型（未设置时返回默认值 `WORK`）。
    ///
    /// 对应 Java: `public ResourceType getType()`
    pub fn get_type(&self) -> ResourceType {
        self.get(key(ResourceField::Type))
            .and_then(|value| value.as_integer())
            .and_then(ResourceType::get_instance)
            .unwrap_or(ResourceType::Work)
    }

    /// 设置空资源标志。
    ///
    /// 对应 Java: `public void setIsNull(boolean isNull)`
    pub fn set_is_null(&mut self, is_null: bool) {
        self.null = is_null;
    }

    /// 取空资源标志。
    ///
    /// 对应 Java: `public boolean getNull()`
    pub fn get_null(&self) -> bool {
        self.null
    }

    /// 设置 Initials 字段。
    ///
    /// 对应 Java: `public void setInitials(String val)`
    pub fn set_initials(&mut self, val: Option<String>) {
        self.set_field_value(key(ResourceField::Initials), val.map(FieldValue::String));
    }

    /// 取 Initials 字段。
    ///
    /// 对应 Java: `public String getInitials()`
    pub fn get_initials(&self) -> Option<String> {
        self.get_string(ResourceField::Initials)
    }

    /// 设置日语读音（phonetics）。
    ///
    /// 对应 Java: `public void setPhonetics(String phonetics)`
    pub fn set_phonetics(&mut self, phonetics: Option<String>) {
        self.set_field_value(
            key(ResourceField::Phonetics),
            phonetics.map(FieldValue::String),
        );
    }

    /// 取日语读音（phonetics）。
    ///
    /// 对应 Java: `public String getPhonetics()`
    pub fn get_phonetics(&self) -> Option<String> {
        self.get_string(ResourceField::Phonetics)
    }

    /// 设置 Windows 账号名。
    ///
    /// 对应 Java: `public void setNtAccount(String ntAccount)`
    pub fn set_nt_account(&mut self, nt_account: Option<String>) {
        self.set_field_value(
            key(ResourceField::WindowsUserAccount),
            nt_account.map(FieldValue::String),
        );
    }

    /// 取 Windows 账号名。
    ///
    /// 对应 Java: `public String getNtAccount()`
    pub fn get_nt_account(&self) -> Option<String> {
        self.get_string(ResourceField::WindowsUserAccount)
    }

    /// 取材料资源的单位标签（由单位名称缩写计算）。
    ///
    /// 对应 Java: `public String getMaterialLabel()`
    pub fn get_material_label(&self) -> Option<String> {
        self.get_string(ResourceField::MaterialLabel)
    }

    /// 设置 Code 字段。
    ///
    /// 对应 Java: `public void setCode(String val)`
    pub fn set_code(&mut self, val: Option<String>) {
        self.set_field_value(key(ResourceField::Code), val.map(FieldValue::String));
    }

    /// 取 Code 字段。
    ///
    /// 对应 Java: `public String getCode()`
    pub fn get_code(&self) -> Option<String> {
        self.get_string(ResourceField::Code)
    }

    /// 设置 Group 字段。
    ///
    /// 对应 Java: `public void setGroup(String val)`
    pub fn set_group(&mut self, val: Option<String>) {
        self.set_field_value(key(ResourceField::Group), val.map(FieldValue::String));
    }

    /// 取 Group 字段。
    ///
    /// 对应 Java: `public String getGroup()`
    pub fn get_group(&self) -> Option<String> {
        self.get_string(ResourceField::Group)
    }

    /// 设置工作组（work group）。
    ///
    /// 对应 Java: `public void setWorkGroup(WorkGroup workGroup)`
    pub fn set_work_group(&mut self, work_group: WorkGroup) {
        self.set_field_value(
            key(ResourceField::Workgroup),
            Some(FieldValue::Integer(work_group.value())),
        );
    }

    /// 取工作组（work group）。
    ///
    /// 对应 Java: `public WorkGroup getWorkGroup()`
    pub fn get_work_group(&self) -> Option<WorkGroup> {
        self.get(key(ResourceField::Workgroup))
            .and_then(|value| value.as_integer())
            .map(WorkGroup::get_instance)
    }

    /// 设置电子邮箱地址。
    ///
    /// 对应 Java: `public void setEmailAddress(String emailAddress)`
    pub fn set_email_address(&mut self, email_address: Option<String>) {
        self.set_field_value(
            key(ResourceField::EmailAddress),
            email_address.map(FieldValue::String),
        );
    }

    /// 取电子邮箱地址。
    ///
    /// 对应 Java: `public String getEmailAddress()`
    pub fn get_email_address(&self) -> Option<String> {
        self.get_string(ResourceField::EmailAddress)
    }

    /// 设置超链接文本。
    ///
    /// 对应 Java: `public void setHyperlink(String hyperlink)`
    pub fn set_hyperlink(&mut self, hyperlink: Option<String>) {
        self.set_field_value(
            key(ResourceField::Hyperlink),
            hyperlink.map(FieldValue::String),
        );
    }

    /// 取超链接文本。
    ///
    /// 对应 Java: `public String getHyperlink()`
    pub fn get_hyperlink(&self) -> Option<String> {
        self.get_string(ResourceField::Hyperlink)
    }

    /// 设置超链接地址。
    ///
    /// 对应 Java: `public void setHyperlinkAddress(String hyperlinkAddress)`
    pub fn set_hyperlink_address(&mut self, hyperlink_address: Option<String>) {
        self.set_field_value(
            key(ResourceField::HyperlinkAddress),
            hyperlink_address.map(FieldValue::String),
        );
    }

    /// 取超链接地址。
    ///
    /// 对应 Java: `public String getHyperlinkAddress()`
    pub fn get_hyperlink_address(&self) -> Option<String> {
        self.get_string(ResourceField::HyperlinkAddress)
    }

    /// 设置超链接子地址。
    ///
    /// 对应 Java: `public void setHyperlinkSubAddress(String hyperlinkSubAddress)`
    pub fn set_hyperlink_sub_address(&mut self, hyperlink_sub_address: Option<String>) {
        self.set_field_value(
            key(ResourceField::HyperlinkSubaddress),
            hyperlink_sub_address.map(FieldValue::String),
        );
    }

    /// 取超链接子地址。
    ///
    /// 对应 Java: `public String getHyperlinkSubAddress()`
    pub fn get_hyperlink_sub_address(&self) -> Option<String> {
        self.get_string(ResourceField::HyperlinkSubaddress)
    }

    /// 设置超链接屏幕提示。
    ///
    /// 对应 Java: `public void setHyperlinkScreenTip(String text)`
    pub fn set_hyperlink_screen_tip(&mut self, text: Option<String>) {
        self.set_field_value(
            key(ResourceField::HyperlinkScreenTip),
            text.map(FieldValue::String),
        );
    }

    /// 取超链接屏幕提示。
    ///
    /// 对应 Java: `public String getHyperlinkScreenTip()`
    pub fn get_hyperlink_screen_tip(&self) -> Option<String> {
        self.get_string(ResourceField::HyperlinkScreenTip)
    }

    /// 设置默认可用性（默认 100）。
    ///
    /// 对应 Java: `public void setDefaultUnits(Number defaultUnits)`
    pub fn set_default_units(&mut self, default_units: Option<f64>) {
        self.set_field_value(
            key(ResourceField::DefaultUnits),
            default_units.map(FieldValue::Double),
        );
    }

    /// 取默认可用性。
    ///
    /// 对应 Java: `public Number getDefaultUnits()`
    pub fn get_default_units(&self) -> Option<f64> {
        self.get_f64(ResourceField::DefaultUnits)
    }

    /// 取当前日期的最大可用性。
    ///
    /// 对应 Java: `public Number getMaxUnits()`
    pub fn get_max_units(&self) -> Option<f64> {
        self.get_f64(ResourceField::MaxUnits)
    }

    /// 设置高峰利用率。
    ///
    /// 对应 Java: `public void setPeakUnits(Number peakUnits)`
    pub fn set_peak_units(&mut self, peak_units: Option<f64>) {
        self.set_field_value(
            key(ResourceField::Peak),
            peak_units.map(FieldValue::Double),
        );
    }

    /// 取高峰利用率。
    ///
    /// 对应 Java: `public Number getPeakUnits()`
    pub fn get_peak_units(&self) -> Option<f64> {
        self.get_f64(ResourceField::Peak)
    }

    /// 设置过度分配标志。
    ///
    /// 对应 Java: `public void setOverAllocated(boolean overallocated)`
    pub fn set_over_allocated(&mut self, overallocated: bool) {
        self.set_field_value(
            key(ResourceField::Overallocated),
            Some(FieldValue::Boolean(overallocated)),
        );
    }

    /// 取过度分配标志。
    ///
    /// 对应 Java: `public boolean getOverAllocated()`
    pub fn get_over_allocated(&self) -> bool {
        self.get_bool(ResourceField::Overallocated)
    }

    /// 取「可用起始」日期。
    ///
    /// 对应 Java: `public LocalDateTime getAvailableFrom()`
    pub fn get_available_from(&self) -> Option<NaiveDateTime> {
        self.get_date_time(ResourceField::AvailableFrom)
    }

    /// 取「可用截止」日期。
    ///
    /// 对应 Java: `public LocalDateTime getAvailableTo()`
    pub fn get_available_to(&self) -> Option<NaiveDateTime> {
        self.get_date_time(ResourceField::AvailableTo)
    }

    /// 取全部已分配任务的最早开始日期。
    ///
    /// 对应 Java: `public LocalDateTime getStart()`
    pub fn get_start(&self) -> Option<NaiveDateTime> {
        self.get_date_time(ResourceField::Start)
    }

    /// 取全部已分配任务的最晚完成日期。
    ///
    /// 对应 Java: `public LocalDateTime getFinish()`
    pub fn get_finish(&self) -> Option<NaiveDateTime> {
        self.get_date_time(ResourceField::Finish)
    }

    /// 设置是否可参与资源平衡。
    ///
    /// 对应 Java: `public void setCanLevel(boolean canLevel)`
    pub fn set_can_level(&mut self, can_level: bool) {
        self.set_field_value(
            key(ResourceField::CanLevel),
            Some(FieldValue::Boolean(can_level)),
        );
    }

    /// 取是否可参与资源平衡。
    ///
    /// 对应 Java: `public boolean getCanLevel()`
    pub fn get_can_level(&self) -> bool {
        self.get_bool(ResourceField::CanLevel)
    }

    /// 设置成本计提方式。
    ///
    /// 对应 Java: `public void setAccrueAt(AccrueType type)`
    pub fn set_accrue_at(&mut self, value: AccrueType) {
        self.set_field_value(
            key(ResourceField::AccrueAt),
            Some(FieldValue::Integer(value.value())),
        );
    }

    /// 取成本计提方式。
    ///
    /// 对应 Java: `public AccrueType getAccrueAt()`
    pub fn get_accrue_at(&self) -> Option<AccrueType> {
        self.get(key(ResourceField::AccrueAt))
            .and_then(|value| value.as_integer())
            .and_then(AccrueType::get_instance)
    }

    /// 设置 Work 字段（导入 MS Project 时忽略）。
    ///
    /// 对应 Java: `public void setWork(Duration val)`
    pub fn set_work(&mut self, val: Option<Duration>) {
        self.set_field_value(key(ResourceField::Work), val.map(FieldValue::Duration));
    }

    /// 取 Work 字段。
    ///
    /// 对应 Java: `public Duration getWork()`
    pub fn get_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::Work)
    }

    /// 取 Regular Work 字段（MPX 规范的扩展）。
    ///
    /// 对应 Java: `public Duration getRegularWork()`
    pub fn get_regular_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::RegularWork)
    }

    /// 设置 Regular Work 字段。
    ///
    /// 对应 Java: `public void setRegularWork(Duration duration)`
    pub fn set_regular_work(&mut self, duration: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::RegularWork),
            duration.map(FieldValue::Duration),
        );
    }

    /// 设置 Actual Work 字段。
    ///
    /// 对应 Java: `public void setActualWork(Duration val)`
    pub fn set_actual_work(&mut self, val: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::ActualWork),
            val.map(FieldValue::Duration),
        );
    }

    /// 取 Actual Work 字段。
    ///
    /// 对应 Java: `public Duration getActualWork()`
    pub fn get_actual_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::ActualWork)
    }

    /// 设置加班工时。
    ///
    /// 对应 Java: `public void setOvertimeWork(Duration overtimeWork)`
    pub fn set_overtime_work(&mut self, overtime_work: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::OvertimeWork),
            overtime_work.map(FieldValue::Duration),
        );
    }

    /// 取加班工时。
    ///
    /// 对应 Java: `public Duration getOvertimeWork()`
    pub fn get_overtime_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::OvertimeWork)
    }

    /// 设置 Remaining Work 字段（导入 MS Project 时忽略）。
    ///
    /// 对应 Java: `public void setRemainingWork(Duration val)`
    pub fn set_remaining_work(&mut self, val: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::RemainingWork),
            val.map(FieldValue::Duration),
        );
    }

    /// 取 Remaining Work 字段。
    ///
    /// 对应 Java: `public Duration getRemainingWork()`
    pub fn get_remaining_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::RemainingWork)
    }

    /// 取实际加班工时。
    ///
    /// 对应 Java: `public Duration getActualOvertimeWork()`
    pub fn get_actual_overtime_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::ActualOvertimeWork)
    }

    /// 设置实际加班工时。
    ///
    /// 对应 Java: `public void setActualOvertimeWork(Duration duration)`
    pub fn set_actual_overtime_work(&mut self, duration: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::ActualOvertimeWork),
            duration.map(FieldValue::Duration),
        );
    }

    /// 取剩余加班工时。
    ///
    /// 对应 Java: `public Duration getRemainingOvertimeWork()`
    pub fn get_remaining_overtime_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::RemainingOvertimeWork)
    }

    /// 设置剩余加班工时。
    ///
    /// 对应 Java: `public void setRemainingOvertimeWork(Duration duration)`
    pub fn set_remaining_overtime_work(&mut self, duration: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::RemainingOvertimeWork),
            duration.map(FieldValue::Duration),
        );
    }

    /// 设置完成百分比。
    ///
    /// 对应 Java: `public void setPercentWorkComplete(Number percentWorkComplete)`
    pub fn set_percent_work_complete(&mut self, percent_work_complete: Option<f64>) {
        self.set_field_value(
            key(ResourceField::PercentWorkComplete),
            percent_work_complete.map(FieldValue::Double),
        );
    }

    /// 取完成百分比。
    ///
    /// 对应 Java: `public Number getPercentWorkComplete()`
    pub fn get_percent_work_complete(&self) -> Option<f64> {
        self.get_f64(ResourceField::PercentWorkComplete)
    }

    /// 取标准费率（由当前成本费率表 A 计算）。
    ///
    /// 对应 Java: `public Rate getStandardRate()`
    pub fn get_standard_rate(&self) -> Option<Rate> {
        self.get_field_rate(ResourceField::StandardRate)
    }

    /// 设置成本字段。
    ///
    /// 对应 Java: `public void setCost(Number cost)`
    pub fn set_cost(&mut self, cost: Option<f64>) {
        self.set_field_value(key(ResourceField::Cost), cost.map(FieldValue::Double));
    }

    /// 取成本字段。
    ///
    /// 对应 Java: `public Number getCost()`
    pub fn get_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::Cost)
    }

    /// 取加班费率（由当前成本费率表 A 计算）。
    ///
    /// 对应 Java: `public Rate getOvertimeRate()`
    pub fn get_overtime_rate(&self) -> Option<Rate> {
        self.get_field_rate(ResourceField::OvertimeRate)
    }

    /// 取加班成本。
    ///
    /// 对应 Java: `public Number getOvertimeCost()`
    pub fn get_overtime_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::OvertimeCost)
    }

    /// 设置加班成本。
    ///
    /// 对应 Java: `public void setOvertimeCost(Number currency)`
    pub fn set_overtime_cost(&mut self, currency: Option<f64>) {
        self.set_field_value(
            key(ResourceField::OvertimeCost),
            currency.map(FieldValue::Double),
        );
    }

    /// 取每次使用成本（由当前成本费率表 A 计算）。
    ///
    /// 对应 Java: `public Number getCostPerUse()`
    pub fn get_cost_per_use(&self) -> Option<f64> {
        self.get_f64(ResourceField::CostPerUse)
    }

    /// 设置实际成本。
    ///
    /// 对应 Java: `public void setActualCost(Number actualCost)`
    pub fn set_actual_cost(&mut self, actual_cost: Option<f64>) {
        self.set_field_value(
            key(ResourceField::ActualCost),
            actual_cost.map(FieldValue::Double),
        );
    }

    /// 取实际成本。
    ///
    /// 对应 Java: `public Number getActualCost()`
    pub fn get_actual_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::ActualCost)
    }

    /// 取实际加班成本。
    ///
    /// 对应 Java: `public Number getActualOvertimeCost()`
    pub fn get_actual_overtime_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::ActualOvertimeCost)
    }

    /// 设置实际加班成本。
    ///
    /// 对应 Java: `public void setActualOvertimeCost(Number actualOvertimeCost)`
    pub fn set_actual_overtime_cost(&mut self, actual_overtime_cost: Option<f64>) {
        self.set_field_value(
            key(ResourceField::ActualOvertimeCost),
            actual_overtime_cost.map(FieldValue::Double),
        );
    }

    /// 设置剩余成本。
    ///
    /// 对应 Java: `public void setRemainingCost(Number remainingCost)`
    pub fn set_remaining_cost(&mut self, remaining_cost: Option<f64>) {
        self.set_field_value(
            key(ResourceField::RemainingCost),
            remaining_cost.map(FieldValue::Double),
        );
    }

    /// 取剩余成本。
    ///
    /// 对应 Java: `public Number getRemainingCost()`
    pub fn get_remaining_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::RemainingCost)
    }

    /// 取剩余加班成本。
    ///
    /// 对应 Java: `public Number getRemainingOvertimeCost()`
    pub fn get_remaining_overtime_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::RemainingOvertimeCost)
    }

    /// 设置剩余加班成本。
    ///
    /// 对应 Java: `public void setRemainingOvertimeCost(Number remainingOvertimeCost)`
    pub fn set_remaining_overtime_cost(&mut self, remaining_overtime_cost: Option<f64>) {
        self.set_field_value(
            key(ResourceField::RemainingOvertimeCost),
            remaining_overtime_cost.map(FieldValue::Double),
        );
    }

    /// 设置工时偏差。
    ///
    /// 对应 Java: `public void setWorkVariance(Duration workVariance)`
    pub fn set_work_variance(&mut self, work_variance: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::WorkVariance),
            work_variance.map(FieldValue::Duration),
        );
    }

    /// 取工时偏差。
    ///
    /// 对应 Java: `public Duration getWorkVariance()`
    pub fn get_work_variance(&self) -> Option<Duration> {
        self.get_duration(ResourceField::WorkVariance)
    }

    /// 设置成本偏差。
    ///
    /// 对应 Java: `public void setCostVariance(Number costVariance)`
    pub fn set_cost_variance(&mut self, cost_variance: Option<f64>) {
        self.set_field_value(
            key(ResourceField::CostVariance),
            cost_variance.map(FieldValue::Double),
        );
    }

    /// 取成本偏差。
    ///
    /// 对应 Java: `public Number getCostVariance()`
    pub fn get_cost_variance(&self) -> Option<f64> {
        self.get_f64(ResourceField::CostVariance)
    }

    /// 设置进度偏差（SV）。
    ///
    /// 对应 Java: `public void setSV(Number sv)`
    pub fn set_sv(&mut self, sv: Option<f64>) {
        self.set_field_value(key(ResourceField::Sv), sv.map(FieldValue::Double));
    }

    /// 取进度偏差（SV）。
    ///
    /// 对应 Java: `public Number getSV()`
    pub fn get_sv(&self) -> Option<f64> {
        self.get_f64(ResourceField::Sv)
    }

    /// 设置成本偏差（CV）。
    ///
    /// 对应 Java: `public void setCV(Number cv)`
    pub fn set_cv(&mut self, cv: Option<f64>) {
        self.set_field_value(key(ResourceField::Cv), cv.map(FieldValue::Double));
    }

    /// 取成本偏差（CV）。
    ///
    /// 对应 Java: `public Number getCV()`
    pub fn get_cv(&self) -> Option<f64> {
        self.get_f64(ResourceField::Cv)
    }

    /// 设置已完成工作实际成本（ACWP）。
    ///
    /// 对应 Java: `public void setACWP(Number acwp)`
    pub fn set_acwp(&mut self, acwp: Option<f64>) {
        self.set_field_value(key(ResourceField::Acwp), acwp.map(FieldValue::Double));
    }

    /// 取已完成工作实际成本（ACWP）。
    ///
    /// 对应 Java: `public Number getACWP()`
    pub fn get_acwp(&self) -> Option<f64> {
        self.get_f64(ResourceField::Acwp)
    }

    /// 设置纯文本备注。
    ///
    /// 对应 Java: `public void setNotes(String notes)`
    pub fn set_notes(&mut self, notes: Option<String>) {
        self.set_field_value(key(ResourceField::Notes), notes.map(FieldValue::String));
    }

    /// 取纯文本备注。
    ///
    /// 对应 Java: `public String getNotes()`
    pub fn get_notes(&self) -> String {
        self.get(key(ResourceField::Notes))
            .and_then(|value| value.as_string().map(str::to_owned))
            .unwrap_or_default()
    }

    /// 设置 `Notes` 实例（Rust 侧只能保留其纯文本表示，见文件头说明）。
    ///
    /// 对应 Java: `public void setNotesObject(Notes notes)`
    pub fn set_notes_object(&mut self, notes: Option<&dyn Notes>) {
        self.set_notes(notes.map(|notes| notes.to_string()));
    }

    /// 取纯文本备注对象（Rust 侧固定为 [`PlainNotes`]）。
    ///
    /// 对应 Java: `public Notes getNotesObject()`
    pub fn get_notes_object(&self) -> Option<PlainNotes> {
        self.get(key(ResourceField::Notes))
            .and_then(|value| value.as_string().map(|text| PlainNotes::new(Some(text.to_owned()))))
    }

    /// 设置计划工作预算成本（BCWS）。
    ///
    /// 对应 Java: `public void setBCWS(Number bcws)`
    pub fn set_bcws(&mut self, bcws: Option<f64>) {
        self.set_field_value(key(ResourceField::Bcws), bcws.map(FieldValue::Double));
    }

    /// 取计划工作预算成本（BCWS）。
    ///
    /// 对应 Java: `public Number getBCWS()`
    pub fn get_bcws(&self) -> Option<f64> {
        self.get_f64(ResourceField::Bcws)
    }

    /// 设置已完成工作预算成本（BCWP）。
    ///
    /// 对应 Java: `public void setBCWP(Number bcwp)`
    pub fn set_bcwp(&mut self, bcwp: Option<f64>) {
        self.set_field_value(key(ResourceField::Bcwp), bcwp.map(FieldValue::Double));
    }

    /// 取已完成工作预算成本（BCWP）。
    ///
    /// 对应 Java: `public Number getBCWP()`
    pub fn get_bcwp(&self) -> Option<f64> {
        self.get_f64(ResourceField::Bcwp)
    }

    /// 设置通用资源标志。
    ///
    /// 对应 Java: `public void setGeneric(boolean value)`
    pub fn set_generic(&mut self, value: bool) {
        self.set_field_value(key(ResourceField::Generic), Some(FieldValue::Boolean(value)));
    }

    /// 取通用资源标志。
    ///
    /// 对应 Java: `public boolean getGeneric()`
    pub fn get_generic(&self) -> bool {
        self.get_bool(ResourceField::Generic)
    }

    /// 设置活动标志。
    ///
    /// 对应 Java: `public void setActive(boolean value)`
    pub fn set_active(&mut self, value: bool) {
        self.set_field_value(key(ResourceField::Active), Some(FieldValue::Boolean(value)));
    }

    /// 取活动标志（默认 `true`）。
    ///
    /// 对应 Java: `public boolean getActive()`
    pub fn get_active(&self) -> bool {
        self.get_bool(ResourceField::Active)
    }

    /// 设置 Active Directory GUID。
    ///
    /// 对应 Java: `public void setActiveDirectoryGUID(String guid)`
    pub fn set_active_directory_guid(&mut self, guid: Option<String>) {
        self.active_directory_guid = guid;
    }

    /// 取 Active Directory GUID。
    ///
    /// 对应 Java: `public String getActiveDirectoryGUID()`
    pub fn get_active_directory_guid(&self) -> Option<String> {
        self.active_directory_guid.clone()
    }

    /// 设置实际加班工时上限（protected）。
    ///
    /// 对应 Java: `public void setActualOvertimeWorkProtected(Duration duration)`
    pub fn set_actual_overtime_work_protected(&mut self, duration: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::ActualOvertimeWorkProtected),
            duration.map(FieldValue::Duration),
        );
    }

    /// 取实际加班工时上限（protected）。
    ///
    /// 对应 Java: `public Duration getActualOvertimeWorkProtected()`
    pub fn get_actual_overtime_work_protected(&self) -> Option<Duration> {
        self.get_duration(ResourceField::ActualOvertimeWorkProtected)
    }

    /// 设置实际工时上限（protected）。
    ///
    /// 对应 Java: `public void setActualWorkProtected(Duration duration)`
    pub fn set_actual_work_protected(&mut self, duration: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::ActualWorkProtected),
            duration.map(FieldValue::Duration),
        );
    }

    /// 取实际工时上限（protected）。
    ///
    /// 对应 Java: `public Duration getActualWorkProtected()`
    pub fn get_actual_work_protected(&self) -> Option<Duration> {
        self.get_duration(ResourceField::ActualWorkProtected)
    }

    /// 设置预订类型。
    ///
    /// 对应 Java: `public void setBookingType(BookingType bookingType)`
    pub fn set_booking_type(&mut self, booking_type: BookingType) {
        self.set_field_value(
            key(ResourceField::BookingType),
            Some(FieldValue::Integer(booking_type.value())),
        );
    }

    /// 取预订类型。
    ///
    /// 对应 Java: `public BookingType getBookingType()`
    pub fn get_booking_type(&self) -> Option<BookingType> {
        self.get(key(ResourceField::BookingType))
            .and_then(|value| value.as_integer())
            .map(BookingType::get_instance)
    }

    /// 设置创建日期。
    ///
    /// 对应 Java: `public void setCreationDate(LocalDateTime creationDate)`
    pub fn set_creation_date(&mut self, creation_date: Option<NaiveDateTime>) {
        self.set_field_value(
            key(ResourceField::Created),
            creation_date.map(FieldValue::DateTime),
        );
    }

    /// 取创建日期。
    ///
    /// 对应 Java: `public LocalDateTime getCreationDate()`
    pub fn get_creation_date(&self) -> Option<NaiveDateTime> {
        self.get_date_time(ResourceField::Created)
    }

    /// 设置企业资源标志。
    ///
    /// 对应 Java: `public void setEnterprise(boolean enterprise)`
    pub fn set_enterprise(&mut self, enterprise: bool) {
        self.set_field_value(
            key(ResourceField::Enterprise),
            Some(FieldValue::Boolean(enterprise)),
        );
    }

    /// 取企业资源标志。
    ///
    /// 对应 Java: `public boolean getEnterprise()`
    pub fn get_enterprise(&self) -> bool {
        self.get_bool(ResourceField::Enterprise)
    }

    /// 取资源日历的 unique ID。
    ///
    /// 对应 Java: `public Integer getCalendarUniqueID()`
    pub fn get_calendar_unique_id(&self) -> Option<i32> {
        self.get_i32(ResourceField::CalendarUniqueId)
    }

    /// 设置资源日历的 unique ID。
    ///
    /// 对应 Java: `public void setCalendarUniqueID(Integer id)`
    pub fn set_calendar_unique_id(&mut self, id: Option<i32>) {
        self.set_field_value(
            key(ResourceField::CalendarUniqueId),
            id.map(FieldValue::Integer),
        );
    }

    /// 取资源关联的日历。
    ///
    /// 对应 Java: `public ProjectCalendar getCalendar()`
    pub fn get_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        let context = self.context.upgrade()?;
        let context_ref = context.try_borrow().ok()?;
        context_ref
            .get_calendars()
            .get_by_unique_id(self.get_calendar_unique_id())
    }

    /// 关联一个已有的资源日历。
    ///
    /// 对应 Java: `public void setCalendar(ProjectCalendar calendar)`
    pub fn set_calendar(&mut self, calendar: Option<&Rc<RefCell<ProjectCalendar>>>) {
        let unique_id = calendar.and_then(|calendar| calendar.borrow().get_unique_id());
        self.set_calendar_unique_id(unique_id);
    }

    /// 新建并关联一个资源日历。
    ///
    /// 对应 Java: `public ProjectCalendar addCalendar() throws MPXJException`
    ///
    /// Rust 侧上下文不可用（未接线或已被可变借用）时返回 `Err`（Java 不会出现该分支）。
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
            context_ref.get_calendars_mut().add()
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

    /// 设置基准日历名称（空值回落到 `"Standard"`）。
    ///
    /// 对应 Java: `public void setBaseCalendar(String val)`
    pub fn set_base_calendar(&mut self, val: Option<String>) {
        let value = match val {
            Some(value) if !value.is_empty() => value,
            _ => DEFAULT_BASE_CALENDAR.to_owned(),
        };
        self.set_field_value(key(ResourceField::BaseCalendar), Some(FieldValue::String(value)));
    }

    /// 设置基线成本（导入 MS Project 时忽略）。
    ///
    /// 对应 Java: `public void setBaselineCost(Number val)`
    pub fn set_baseline_cost(&mut self, val: Option<f64>) {
        self.set_field_value(
            key(ResourceField::BaselineCost),
            val.map(FieldValue::Double),
        );
    }

    /// 设置基线工时（导入 MS Project 时忽略）。
    ///
    /// 对应 Java: `public void setBaselineWork(Duration val)`
    pub fn set_baseline_work(&mut self, val: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::BaselineWork),
            val.map(FieldValue::Duration),
        );
    }

    /// 取该资源关联的基准日历名称。
    ///
    /// 对应 Java: `public String getBaseCalendar()`
    pub fn get_base_calendar(&self) -> Option<String> {
        self.get_string(ResourceField::BaseCalendar)
    }

    /// 取基线成本。
    ///
    /// 对应 Java: `public Number getBaselineCost()`
    pub fn get_baseline_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::BaselineCost)
    }

    /// 取基线工时。
    ///
    /// 对应 Java: `public Duration getBaselineWork()`
    pub fn get_baseline_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::BaselineWork)
    }

    /// 设置「链接字段」标志（导入 MS Project 时忽略）。
    ///
    /// 对应 Java: `public void setLinkedFields(boolean val)`
    pub fn set_linked_fields(&mut self, val: bool) {
        self.set_field_value(
            key(ResourceField::LinkedFields),
            Some(FieldValue::Boolean(val)),
        );
    }

    /// 取「链接字段」标志。
    ///
    /// 对应 Java: `public boolean getLinkedFields()`
    pub fn get_linked_fields(&self) -> bool {
        self.get_bool(ResourceField::LinkedFields)
    }

    /// 设置 Objects 字段。
    ///
    /// 对应 Java: `public void setObjects(Integer val)`
    pub fn set_objects(&mut self, val: Option<i32>) {
        self.set_field_value(key(ResourceField::Objects), val.map(FieldValue::Integer));
    }

    /// 取 Objects 字段。
    ///
    /// 对应 Java: `public Integer getObjects()`
    pub fn get_objects(&self) -> Option<i32> {
        self.get_i32(ResourceField::Objects)
    }

    /// 设置文本字段（下标 1-30）。
    ///
    /// 对应 Java: `public void setText(int index, String value)`
    pub fn set_text_at(&mut self, index: i32, value: Option<String>) {
        if let Some(field) = select_field(&CUSTOM_TEXT, index) {
            self.set_field_value(key(field), value.map(FieldValue::String));
        }
    }

    /// 取文本字段（下标 1-30）。
    ///
    /// 对应 Java: `public String getText(int index)`
    pub fn get_text_at(&self, index: i32) -> Option<String> {
        select_field(&CUSTOM_TEXT, index).and_then(|field| self.get_string(field))
    }

    /// 取父资源的 unique ID。
    ///
    /// 对应 Java: `public Integer getParentResourceUniqueID()`
    pub fn get_parent_resource_unique_id(&self) -> Option<i32> {
        self.get_i32(ResourceField::ParentId)
    }

    /// 设置父资源的 unique ID。
    ///
    /// 对应 Java: `public void setParentResourceUniqueID(Integer id)`
    pub fn set_parent_resource_unique_id(&mut self, id: Option<i32>) {
        self.set_field_value(key(ResourceField::ParentId), id.map(FieldValue::Integer));
    }

    /// 取父资源。
    ///
    /// 对应 Java: `public Resource getParentResource()`
    pub fn get_parent_resource(&self) -> Option<Rc<RefCell<Resource>>> {
        let context = self.context.upgrade()?;
        let context_ref = context.try_borrow().ok()?;
        context_ref
            .get_resources()
            .get_by_unique_id(self.get_parent_resource_unique_id())
    }

    /// 设置父资源。
    ///
    /// 对应 Java: `public void setParentResource(Resource resource)`
    pub fn set_parent_resource(&mut self, resource: Option<&Rc<RefCell<Resource>>>) {
        let unique_id = resource.and_then(|resource| resource.borrow().get_unique_id());
        self.set_parent_resource_unique_id(unique_id);
    }

    /// 设置开始日期字段（下标 1-10）。
    ///
    /// 对应 Java: `public void setStart(int index, LocalDateTime value)`
    pub fn set_start_at(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(&CUSTOM_START, index) {
            self.set_field_value(key(field), value.map(FieldValue::DateTime));
        }
    }

    /// 取开始日期字段（下标 1-10）。
    ///
    /// 对应 Java: `public LocalDateTime getStart(int index)`
    pub fn get_start_at(&self, index: i32) -> Option<NaiveDateTime> {
        select_field(&CUSTOM_START, index).and_then(|field| self.get_date_time(field))
    }

    /// 设置完成日期字段（下标 1-10）。
    ///
    /// 对应 Java: `public void setFinish(int index, LocalDateTime value)`
    pub fn set_finish_at(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(&CUSTOM_FINISH, index) {
            self.set_field_value(key(field), value.map(FieldValue::DateTime));
        }
    }

    /// 取完成日期字段（下标 1-10）。
    ///
    /// 对应 Java: `public LocalDateTime getFinish(int index)`
    pub fn get_finish_at(&self, index: i32) -> Option<NaiveDateTime> {
        select_field(&CUSTOM_FINISH, index).and_then(|field| self.get_date_time(field))
    }

    /// 设置数字字段（下标 1-20）。
    ///
    /// 对应 Java: `public void setNumber(int index, Number value)`
    pub fn set_number_at(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = select_field(&CUSTOM_NUMBER, index) {
            self.set_field_value(key(field), value.map(FieldValue::Double));
        }
    }

    /// 取数字字段（下标 1-20）。
    ///
    /// 对应 Java: `public Number getNumber(int index)`
    pub fn get_number_at(&self, index: i32) -> Option<f64> {
        select_field(&CUSTOM_NUMBER, index).and_then(|field| self.get_f64(field))
    }

    /// 设置工期字段（下标 1-10）。
    ///
    /// 对应 Java: `public void setDuration(int index, Duration value)`
    pub fn set_duration_at(&mut self, index: i32, value: Option<Duration>) {
        if let Some(field) = select_field(&CUSTOM_DURATION, index) {
            self.set_field_value(key(field), value.map(FieldValue::Duration));
        }
    }

    /// 取工期字段（下标 1-10）。
    ///
    /// 对应 Java: `public Duration getDuration(int index)`
    pub fn get_duration_at(&self, index: i32) -> Option<Duration> {
        select_field(&CUSTOM_DURATION, index).and_then(|field| self.get_duration(field))
    }

    /// 设置日期字段（下标 1-10）。
    ///
    /// 对应 Java: `public void setDate(int index, LocalDateTime value)`
    pub fn set_date_at(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(&CUSTOM_DATE, index) {
            self.set_field_value(key(field), value.map(FieldValue::DateTime));
        }
    }

    /// 取日期字段（下标 1-10）。
    ///
    /// 对应 Java: `public LocalDateTime getDate(int index)`
    pub fn get_date_at(&self, index: i32) -> Option<NaiveDateTime> {
        select_field(&CUSTOM_DATE, index).and_then(|field| self.get_date_time(field))
    }

    /// 设置成本字段（下标 1-10）。
    ///
    /// 对应 Java: `public void setCost(int index, Number value)`
    pub fn set_cost_at(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = select_field(&CUSTOM_COST, index) {
            self.set_field_value(key(field), value.map(FieldValue::Double));
        }
    }

    /// 取成本字段（下标 1-10）。
    ///
    /// 对应 Java: `public Number getCost(int index)`
    pub fn get_cost_at(&self, index: i32) -> Option<f64> {
        select_field(&CUSTOM_COST, index).and_then(|field| self.get_f64(field))
    }

    /// 设置标志字段（下标 1-20）。
    ///
    /// 对应 Java: `public void setFlag(int index, boolean value)`
    pub fn set_flag_at(&mut self, index: i32, value: bool) {
        if let Some(field) = select_field(&CUSTOM_FLAG, index) {
            self.set_field_value(key(field), Some(FieldValue::Boolean(value)));
        }
    }

    /// 取标志字段（下标 1-20）。
    ///
    /// 对应 Java: `public boolean getFlag(int index)`
    pub fn get_flag_at(&self, index: i32) -> bool {
        select_field(&CUSTOM_FLAG, index)
            .map(|field| self.get_bool(field))
            .unwrap_or(false)
    }

    /// 设置大纲代码字段（下标 1-10）。
    ///
    /// 对应 Java: `public void setOutlineCode(int index, String value)`
    pub fn set_outline_code_at(&mut self, index: i32, value: Option<String>) {
        if let Some(field) = select_field(&CUSTOM_OUTLINE_CODE, index) {
            self.set_field_value(key(field), value.map(FieldValue::String));
        }
    }

    /// 取大纲代码字段（下标 1-10）。
    ///
    /// 对应 Java: `public String getOutlineCode(int index)`
    pub fn get_outline_code_at(&self, index: i32) -> Option<String> {
        select_field(&CUSTOM_OUTLINE_CODE, index).and_then(|field| self.get_string(field))
    }

    /// 把本资源从项目中移除。
    ///
    /// 对应 Java: `public void remove()`
    ///
    /// Rust 侧依赖 [`Resource::set_self_reference`] 接线的自身弱引用；未接线时为空操作。
    pub fn remove(&mut self) {
        let Some(self_rc) = self.self_ref.upgrade() else {
            return;
        };
        let Some(context) = self.context.upgrade() else {
            return;
        };
        // 注：此处不能写成以 `if let` 作函数末尾语句的形式—— scrutinee 的临时值会在函数
        // 局部变量之后析构，导致编译器判定 `context` 生命周期不足。
        let Ok(mut context_ref) = context.try_borrow_mut() else {
            return;
        };
        context_ref.get_resources_mut().remove_resource(&self_rc);
    }

    /// 按字段别名取值。
    ///
    /// 对应 Java: `public Object getFieldByAlias(String alias)`
    pub fn get_field_by_alias(&self, alias: &str) -> Option<FieldValue> {
        let field = self.get_field_key_by_alias(alias)?;
        self.get(field)
    }

    /// 按字段别名设值。
    ///
    /// 对应 Java: `public void setFieldByAlias(String alias, Object value)`
    pub fn set_field_by_alias(&mut self, alias: &str, value: Option<FieldValue>) {
        if let Some(field) = self.get_field_key_by_alias(alias) {
            self.set_field_value(field, value);
        }
    }

    /// 取分配给本资源的全部资源分配。
    ///
    /// 对应 Java: `public List<ResourceAssignment> getTaskAssignments()`
    pub fn get_task_assignments(&self) -> Vec<Rc<RefCell<ResourceAssignment>>> {
        self.get_resource_assignment_stream(self.get_unique_id())
    }

    /// 为资源添加角色分配（替换该角色已有分配）。
    ///
    /// 对应 Java: `public void addRoleAssignment(Resource role, SkillLevel skillLevel)`
    pub fn add_role_assignment(
        &mut self,
        role: &Rc<RefCell<Resource>>,
        skill_level: SkillLevel,
    ) {
        let role_id = NumberHelper::get_int(role.borrow().get_id().map(f64::from));
        self.role_assignments
            .insert(role_id, (Rc::downgrade(role), skill_level));
    }

    /// 移除一个角色分配。
    ///
    /// 对应 Java: `public void removeRoleAssignment(Resource role)`
    pub fn remove_role_assignment(&mut self, role: &Rc<RefCell<Resource>>) {
        let role_id = NumberHelper::get_int(role.borrow().get_id().map(f64::from));
        self.role_assignments.remove(&role_id);
    }

    /// 取角色分配列表（Java 返回 `Map<Resource, SkillLevel>`，Rust 返回快照 `Vec`）。
    ///
    /// 对应 Java: `public Map<Resource, SkillLevel> getRoleAssignments()`
    pub fn get_role_assignments(&self) -> Vec<(Rc<RefCell<Resource>>, SkillLevel)> {
        self.role_assignments
            .values()
            .filter_map(|(role, skill_level)| role.upgrade().map(|role| (role, *skill_level)))
            .collect()
    }

    /// 取子项目中的资源 unique ID。
    ///
    /// 对应 Java: `public Integer getSubprojectResourceUniqueID()`
    pub fn get_subproject_resource_unique_id(&self) -> Option<i32> {
        self.get_i32(ResourceField::SubprojectResourceUniqueId)
    }

    /// 设置子项目中的资源 unique ID。
    ///
    /// 对应 Java: `public void setSubprojectResourceUniqueID(Integer subprojectUniqueResourceID)`
    pub fn set_subproject_resource_unique_id(&mut self, value: Option<i32>) {
        self.set_field_value(
            key(ResourceField::SubprojectResourceUniqueId),
            value.map(FieldValue::Integer),
        );
    }

    /// 取企业成本字段（下标 1-10）。
    ///
    /// 对应 Java: `public Number getEnterpriseCost(int index)`
    pub fn get_enterprise_cost_at(&self, index: i32) -> Option<f64> {
        select_field(&ENTERPRISE_CUSTOM_COST, index).and_then(|field| self.get_f64(field))
    }

    /// 设置企业成本字段（下标 1-10）。
    ///
    /// 对应 Java: `public void setEnterpriseCost(int index, Number value)`
    pub fn set_enterprise_cost_at(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = select_field(&ENTERPRISE_CUSTOM_COST, index) {
            self.set_field_value(key(field), value.map(FieldValue::Double));
        }
    }

    /// 取企业日期字段（下标 1-30）。
    ///
    /// 对应 Java: `public LocalDateTime getEnterpriseDate(int index)`
    pub fn get_enterprise_date_at(&self, index: i32) -> Option<NaiveDateTime> {
        select_field(&ENTERPRISE_CUSTOM_DATE, index).and_then(|field| self.get_date_time(field))
    }

    /// 设置企业日期字段（下标 1-30）。
    ///
    /// 对应 Java: `public void setEnterpriseDate(int index, LocalDateTime value)`
    pub fn set_enterprise_date_at(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(&ENTERPRISE_CUSTOM_DATE, index) {
            self.set_field_value(key(field), value.map(FieldValue::DateTime));
        }
    }

    /// 取企业工期字段（下标 1-10）。
    ///
    /// 对应 Java: `public Duration getEnterpriseDuration(int index)`
    pub fn get_enterprise_duration_at(&self, index: i32) -> Option<Duration> {
        select_field(&ENTERPRISE_CUSTOM_DURATION, index).and_then(|field| self.get_duration(field))
    }

    /// 设置企业工期字段（下标 1-10）。
    ///
    /// 对应 Java: `public void setEnterpriseDuration(int index, Duration value)`
    pub fn set_enterprise_duration_at(&mut self, index: i32, value: Option<Duration>) {
        if let Some(field) = select_field(&ENTERPRISE_CUSTOM_DURATION, index) {
            self.set_field_value(key(field), value.map(FieldValue::Duration));
        }
    }

    /// 取企业标志字段（下标 1-20）。
    ///
    /// 对应 Java: `public boolean getEnterpriseFlag(int index)`
    pub fn get_enterprise_flag_at(&self, index: i32) -> bool {
        select_field(&ENTERPRISE_CUSTOM_FLAG, index)
            .map(|field| self.get_bool(field))
            .unwrap_or(false)
    }

    /// 设置企业标志字段（下标 1-20）。
    ///
    /// 对应 Java: `public void setEnterpriseFlag(int index, boolean value)`
    pub fn set_enterprise_flag_at(&mut self, index: i32, value: bool) {
        if let Some(field) = select_field(&ENTERPRISE_CUSTOM_FLAG, index) {
            self.set_field_value(key(field), Some(FieldValue::Boolean(value)));
        }
    }

    /// 取企业数字字段（下标 1-40）。
    ///
    /// 对应 Java: `public Number getEnterpriseNumber(int index)`
    pub fn get_enterprise_number_at(&self, index: i32) -> Option<f64> {
        select_field(&ENTERPRISE_CUSTOM_NUMBER, index).and_then(|field| self.get_f64(field))
    }

    /// 设置企业数字字段（下标 1-40）。
    ///
    /// 对应 Java: `public void setEnterpriseNumber(int index, Number value)`
    pub fn set_enterprise_number_at(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = select_field(&ENTERPRISE_CUSTOM_NUMBER, index) {
            self.set_field_value(key(field), value.map(FieldValue::Double));
        }
    }

    /// 取企业文本字段（下标 1-40）。
    ///
    /// 对应 Java: `public String getEnterpriseText(int index)`
    pub fn get_enterprise_text_at(&self, index: i32) -> Option<String> {
        select_field(&ENTERPRISE_CUSTOM_TEXT, index).and_then(|field| self.get_string(field))
    }

    /// 设置企业文本字段（下标 1-40）。
    ///
    /// 对应 Java: `public void setEnterpriseText(int index, String value)`
    pub fn set_enterprise_text_at(&mut self, index: i32, value: Option<String>) {
        if let Some(field) = select_field(&ENTERPRISE_CUSTOM_TEXT, index) {
            self.set_field_value(key(field), value.map(FieldValue::String));
        }
    }

    /// 设置基线成本字段（下标 1-10）。
    ///
    /// 对应 Java: `public void setBaselineCost(int baselineNumber, Number value)`
    pub fn set_baseline_cost_at(&mut self, baseline_number: i32, value: Option<f64>) {
        if let Some(field) = select_field(&BASELINE_COSTS, baseline_number) {
            self.set_field_value(key(field), value.map(FieldValue::Double));
        }
    }

    /// 设置基线工时字段（下标 1-10）。
    ///
    /// 对应 Java: `public void setBaselineWork(int baselineNumber, Duration value)`
    pub fn set_baseline_work_at(&mut self, baseline_number: i32, value: Option<Duration>) {
        if let Some(field) = select_field(&BASELINE_WORKS, baseline_number) {
            self.set_field_value(key(field), value.map(FieldValue::Duration));
        }
    }

    /// 取基线成本字段（下标 1-10）。
    ///
    /// 对应 Java: `public Number getBaselineCost(int baselineNumber)`
    pub fn get_baseline_cost_at(&self, baseline_number: i32) -> Option<f64> {
        select_field(&BASELINE_COSTS, baseline_number).and_then(|field| self.get_f64(field))
    }

    /// 取基线工时字段（下标 1-10）。
    ///
    /// 对应 Java: `public Duration getBaselineWork(int baselineNumber)`
    pub fn get_baseline_work_at(&self, baseline_number: i32) -> Option<Duration> {
        select_field(&BASELINE_WORKS, baseline_number).and_then(|field| self.get_duration(field))
    }

    /// 取预算标志。
    ///
    /// 对应 Java: `public boolean getBudget()`
    pub fn get_budget(&self) -> bool {
        self.get_bool(ResourceField::Budget)
    }

    /// 设置预算标志。
    ///
    /// 对应 Java: `public void setBudget(boolean budget)`
    pub fn set_budget(&mut self, budget: bool) {
        self.set_field_value(key(ResourceField::Budget), Some(FieldValue::Boolean(budget)));
    }

    /// 取资源 GUID（Rust 侧从字符串编码解析）。
    ///
    /// 对应 Java: `public UUID getGUID()`
    pub fn get_guid(&self) -> Option<Uuid> {
        self.get(key(ResourceField::Guid))
            .and_then(|value| value.as_string().and_then(|text| Uuid::parse_str(text).ok()))
    }

    /// 设置资源 GUID。
    ///
    /// 对应 Java: `public void setGUID(UUID value)`
    pub fn set_guid(&mut self, value: Option<Uuid>) {
        self.set_field_value(
            key(ResourceField::Guid),
            value.map(|value| FieldValue::String(value.to_string())),
        );
    }

    /// 设置单位字段。
    ///
    /// 对应 Java: `public void setUnit(String value)`
    pub fn set_unit(&mut self, value: Option<String>) {
        self.set_field_value(key(ResourceField::Unit), value.map(FieldValue::String));
    }

    /// 取单位字段。
    ///
    /// 对应 Java: `public String getUnit()`
    pub fn get_unit(&self) -> Option<String> {
        self.get_string(ResourceField::Unit)
    }

    /// 设置供应参考字段。
    ///
    /// 对应 Java: `public void setSupplyReference(String value)`
    pub fn set_supply_reference(&mut self, value: Option<String>) {
        self.set_field_value(
            key(ResourceField::SupplyReference),
            value.map(FieldValue::String),
        );
    }

    /// 取供应参考字段。
    ///
    /// 对应 Java: `public String getSupplyReference()`
    pub fn get_supply_reference(&self) -> Option<String> {
        self.get_string(ResourceField::SupplyReference)
    }

    /// 设置描述字段。
    ///
    /// 对应 Java: `public void setDescription(String value)`
    pub fn set_description(&mut self, value: Option<String>) {
        self.set_field_value(key(ResourceField::Description), value.map(FieldValue::String));
    }

    /// 取描述字段。
    ///
    /// 对应 Java: `public String getDescription()`
    pub fn get_description(&self) -> Option<String> {
        self.get_string(ResourceField::Description)
    }

    /// 设置资源 ID 字段。
    ///
    /// 对应 Java: `public void setResourceID(String value)`
    pub fn set_resource_id(&mut self, value: Option<String>) {
        self.set_field_value(key(ResourceField::ResourceId), value.map(FieldValue::String));
    }

    /// 取资源 ID 字段。
    ///
    /// 对应 Java: `public String getResourceID()`
    pub fn get_resource_id(&self) -> Option<String> {
        self.get_string(ResourceField::ResourceId)
    }

    /// 设置 modify on integrate 字段。
    ///
    /// 对应 Java: `public void setModifyOnIntegrate(boolean value)`
    pub fn set_modify_on_integrate(&mut self, value: bool) {
        self.set_field_value(
            key(ResourceField::ModifyOnIntegrate),
            Some(FieldValue::Boolean(value)),
        );
    }

    /// 取 modify on integrate 字段。
    ///
    /// 对应 Java: `public boolean getModifyOnIntegrate()`
    pub fn get_modify_on_integrate(&self) -> bool {
        self.get_bool(ResourceField::ModifyOnIntegrate)
    }

    /// 设置 expenses only 字段。
    ///
    /// 对应 Java: `public void setExpensesOnly(boolean value)`
    pub fn set_expenses_only(&mut self, value: bool) {
        self.set_field_value(
            key(ResourceField::ExpensesOnly),
            Some(FieldValue::Boolean(value)),
        );
    }

    /// 取 expenses only 字段。
    ///
    /// 对应 Java: `public boolean getExpensesOnly()`
    pub fn get_expenses_only(&self) -> bool {
        self.get_bool(ResourceField::ExpensesOnly)
    }

    /// 设置 period dur 字段。
    ///
    /// 对应 Java: `public void setPeriodDur(Number value)`
    pub fn set_period_dur(&mut self, value: Option<f64>) {
        self.set_field_value(key(ResourceField::PeriodDur), value.map(FieldValue::Double));
    }

    /// 取 period dur 字段。
    ///
    /// 对应 Java: `public Number getPeriodDur()`
    pub fn get_period_dur(&self) -> Option<f64> {
        self.get_f64(ResourceField::PeriodDur)
    }

    /// 设置优先级字段。
    ///
    /// 对应 Java: `public void setPriority(Number value)`
    pub fn set_priority(&mut self, value: Option<f64>) {
        self.set_field_value(key(ResourceField::Priority), value.map(FieldValue::Double));
    }

    /// 取优先级字段。
    ///
    /// 对应 Java: `public Number getPriority()`
    pub fn get_priority(&self) -> Option<f64> {
        self.get_f64(ResourceField::Priority)
    }

    /// 设置费率字段（TurboProject 专用）。
    ///
    /// 对应 Java: `public void setRate(Number value)`
    pub fn set_rate(&mut self, value: Option<f64>) {
        self.set_field_value(key(ResourceField::Rate), value.map(FieldValue::Double));
    }

    /// 取费率字段（TurboProject 专用）。
    ///
    /// 对应 Java: `public Number getRate()`
    pub fn get_rate(&self) -> Option<f64> {
        self.get_f64(ResourceField::Rate)
    }

    /// 设置 pool 字段。
    ///
    /// 对应 Java: `public void setPool(Number value)`
    pub fn set_pool(&mut self, value: Option<f64>) {
        self.set_field_value(key(ResourceField::Pool), value.map(FieldValue::Double));
    }

    /// 取 pool 字段。
    ///
    /// 对应 Java: `public Number getPool()`
    pub fn get_pool(&self) -> Option<f64> {
        self.get_f64(ResourceField::Pool)
    }

    /// 设置 per day 字段。
    ///
    /// 对应 Java: `public void setPerDay(Number value)`
    pub fn set_per_day(&mut self, value: Option<f64>) {
        self.set_field_value(key(ResourceField::PerDay), value.map(FieldValue::Double));
    }

    /// 取 per day 字段。
    ///
    /// 对应 Java: `public Number getPerDay()`
    pub fn get_per_day(&self) -> Option<f64> {
        self.get_f64(ResourceField::PerDay)
    }

    /// 设置电话字段。
    ///
    /// 对应 Java: `public void setPhone(String value)`
    pub fn set_phone(&mut self, value: Option<String>) {
        self.set_field_value(key(ResourceField::Phone), value.map(FieldValue::String));
    }

    /// 取电话字段。
    ///
    /// 对应 Java: `public String getPhone()`
    pub fn get_phone(&self) -> Option<String> {
        self.get_string(ResourceField::Phone)
    }

    /// 设置角色标志。
    ///
    /// 对应 Java: `public void setRole(boolean value)`
    pub fn set_role(&mut self, value: bool) {
        self.set_field_value(key(ResourceField::Role), Some(FieldValue::Boolean(value)));
    }

    /// 取角色标志（默认 `false`）。
    ///
    /// 对应 Java: `public boolean getRole()`
    pub fn get_role(&self) -> bool {
        self.get_bool(ResourceField::Role)
    }

    /// 关联一个完整的成本费率表。
    ///
    /// 对应 Java: `public void setCostRateTable(int index, CostRateTable crt)`
    ///
    /// Java 越界抛 `ArrayIndexOutOfBoundsException`；Rust 侧越界为空操作。
    pub fn set_cost_rate_table(&mut self, index: i32, crt: CostRateTable) {
        if index >= 0 {
            if let Some(slot) = self.cost_rate_tables.get_mut(index as usize) {
                *slot = crt;
            }
        }
    }

    /// 取成本费率表。
    ///
    /// 对应 Java: `public CostRateTable getCostRateTable(int index)`
    pub fn get_cost_rate_table(&self, index: i32) -> Option<&CostRateTable> {
        if index < 0 || index >= CostRateTable::MAX_TABLES {
            return None;
        }
        self.cost_rate_tables.get(index as usize)
    }

    /// 取当前日期生效的成本费率表行。
    ///
    /// 对应 Java: `public CostRateTableEntry getCurrentCostRateTableEntry(int costRateTable)`
    pub fn get_current_cost_rate_table_entry(
        &self,
        cost_rate_table: i32,
    ) -> Option<&CostRateTableEntry> {
        self.get_cost_rate_table(cost_rate_table)
            .and_then(|table| table.get_entry_by_date(now()))
    }

    /// 取本资源的可用性表。
    ///
    /// 对应 Java: `public AvailabilityTable getAvailability()`
    pub fn get_availability(&self) -> Ref<'_, AvailabilityTable> {
        self.availability.borrow()
    }

    /// 取当前日期生效的可用性表行。
    ///
    /// 对应 Java: `public Availability getCurrentAvailabilityTableEntry()`
    pub fn get_current_availability_table_entry(&self) -> Option<Availability> {
        let date = now();
        self.availability
            .try_borrow()
            .ok()
            .and_then(|table| table.get_entry_by_date(date).copied())
    }

    /// 取预算成本。
    ///
    /// 对应 Java: `public Number getBudgetCost()`
    pub fn get_budget_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::BudgetCost)
    }

    /// 设置预算成本。
    ///
    /// 对应 Java: `public void setBudgetCost(Number value)`
    pub fn set_budget_cost(&mut self, value: Option<f64>) {
        self.set_field_value(key(ResourceField::BudgetCost), value.map(FieldValue::Double));
    }

    /// 取预算工时。
    ///
    /// 对应 Java: `public Duration getBudgetWork()`
    pub fn get_budget_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::BudgetWork)
    }

    /// 设置预算工时。
    ///
    /// 对应 Java: `public void setBudgetWork(Duration value)`
    pub fn set_budget_work(&mut self, value: Option<Duration>) {
        self.set_field_value(key(ResourceField::BudgetWork), value.map(FieldValue::Duration));
    }

    /// 取基线预算成本。
    ///
    /// 对应 Java: `public Number getBaselineBudgetCost()`
    pub fn get_baseline_budget_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::BaselineBudgetCost)
    }

    /// 设置基线预算成本。
    ///
    /// 对应 Java: `public void setBaselineBudgetCost(Number value)`
    pub fn set_baseline_budget_cost(&mut self, value: Option<f64>) {
        self.set_field_value(
            key(ResourceField::BaselineBudgetCost),
            value.map(FieldValue::Double),
        );
    }

    /// 取基线预算工时。
    ///
    /// 对应 Java: `public Duration getBaselineBudgetWork()`
    pub fn get_baseline_budget_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::BaselineBudgetWork)
    }

    /// 设置基线预算工时。
    ///
    /// 对应 Java: `public void setBaselineBudgetWork(Duration value)`
    pub fn set_baseline_budget_work(&mut self, value: Option<Duration>) {
        self.set_field_value(
            key(ResourceField::BaselineBudgetWork),
            value.map(FieldValue::Duration),
        );
    }

    /// 取基线预算成本字段（下标 1-10）。
    ///
    /// 对应 Java: `public Number getBaselineBudgetCost(int baselineNumber)`
    pub fn get_baseline_budget_cost_at(&self, baseline_number: i32) -> Option<f64> {
        select_field(&BASELINE_BUDGET_COSTS, baseline_number).and_then(|field| self.get_f64(field))
    }

    /// 设置基线预算成本字段（下标 1-10）。
    ///
    /// 对应 Java: `public void setBaselineBudgetCost(int baselineNumber, Number value)`
    pub fn set_baseline_budget_cost_at(&mut self, baseline_number: i32, value: Option<f64>) {
        if let Some(field) = select_field(&BASELINE_BUDGET_COSTS, baseline_number) {
            self.set_field_value(key(field), value.map(FieldValue::Double));
        }
    }

    /// 取基线预算工时字段（下标 1-10）。
    ///
    /// 对应 Java: `public Duration getBaselineBudgetWork(int baselineNumber)`
    pub fn get_baseline_budget_work_at(&self, baseline_number: i32) -> Option<Duration> {
        select_field(&BASELINE_BUDGET_WORKS, baseline_number).and_then(|field| self.get_duration(field))
    }

    /// 设置基线预算工时字段（下标 1-10）。
    ///
    /// 对应 Java: `public void setBaselineBudgetWork(int baselineNumber, Duration value)`
    pub fn set_baseline_budget_work_at(&mut self, baseline_number: i32, value: Option<Duration>) {
        if let Some(field) = select_field(&BASELINE_BUDGET_WORKS, baseline_number) {
            self.set_field_value(key(field), value.map(FieldValue::Duration));
        }
    }

    /// 取成本中心。
    ///
    /// 对应 Java: `public String getCostCenter()`
    pub fn get_cost_center(&self) -> Option<String> {
        self.get_string(ResourceField::CostCenter)
    }

    /// 设置成本中心。
    ///
    /// 对应 Java: `public void setCostCenter(String value)`
    pub fn set_cost_center(&mut self, value: Option<String>) {
        self.set_field_value(key(ResourceField::CostCenter), value.map(FieldValue::String));
    }

    /// 取「从单位计算成本」标志（默认 `true`）。
    ///
    /// 对应 Java: `public boolean getCalculateCostsFromUnits()`
    pub fn get_calculate_costs_from_units(&self) -> bool {
        self.get_bool(ResourceField::CalculateCostsFromUnits)
    }

    /// 设置「从单位计算成本」标志。
    ///
    /// 对应 Java: `public void setCalculateCostsFromUnits(boolean calculateCostsFromUnits)`
    pub fn set_calculate_costs_from_units(&mut self, calculate_costs_from_units: bool) {
        self.set_field_value(
            key(ResourceField::CalculateCostsFromUnits),
            Some(FieldValue::Boolean(calculate_costs_from_units)),
        );
    }

    /// 取资源序号。
    ///
    /// 对应 Java: `public Integer getSequenceNumber()`
    pub fn get_sequence_number(&self) -> Option<i32> {
        self.get_i32(ResourceField::SequenceNumber)
    }

    /// 设置资源序号。
    ///
    /// 对应 Java: `public void setSequenceNumber(Integer sequenceNumber)`
    pub fn set_sequence_number(&mut self, sequence_number: Option<i32>) {
        self.set_field_value(
            key(ResourceField::SequenceNumber),
            sequence_number.map(FieldValue::Integer),
        );
    }

    /// 取位置（location）的 unique ID。
    ///
    /// 对应 Java: `public Integer getLocationUniqueID()`
    pub fn get_location_unique_id(&self) -> Option<i32> {
        self.get_i32(ResourceField::LocationUniqueId)
    }

    /// 设置位置（location）的 unique ID。
    ///
    /// 对应 Java: `public void setLocationUniqueID(Integer uniqueID)`
    pub fn set_location_unique_id(&mut self, unique_id: Option<i32>) {
        self.set_field_value(
            key(ResourceField::LocationUniqueId),
            unique_id.map(FieldValue::Integer),
        );
    }

    /// 取班次 unique ID。
    ///
    /// 对应 Java: `public Integer getShiftUniqueID()`
    pub fn get_shift_unique_id(&self) -> Option<i32> {
        self.get_i32(ResourceField::ShiftUniqueId)
    }

    /// 设置班次 unique ID。
    ///
    /// 对应 Java: `public void setShiftUniqueID(Integer uniqueID)`
    pub fn set_shift_unique_id(&mut self, unique_id: Option<i32>) {
        self.set_field_value(
            key(ResourceField::ShiftUniqueId),
            unique_id.map(FieldValue::Integer),
        );
    }

    /// 取班次。
    ///
    /// 对应 Java: `public Shift getShift()`
    pub fn get_shift(&self) -> Option<Rc<Shift>> {
        let context = self.context.upgrade()?;
        let context_ref = context.try_borrow().ok()?;
        context_ref
            .get_shifts()
            .get_by_unique_id(self.get_shift_unique_id())
            .cloned()
    }

    /// 设置班次。
    ///
    /// 对应 Java: `public void setShift(Shift shift)`
    pub fn set_shift(&mut self, shift: Option<&Rc<Shift>>) {
        self.set_shift_unique_id(shift.and_then(|shift| shift.get_unique_id()));
    }

    /// 取计量单位 unique ID。
    ///
    /// 对应 Java: `public Integer getUnitOfMeasureUniqueID()`
    pub fn get_unit_of_measure_unique_id(&self) -> Option<i32> {
        self.get_i32(ResourceField::UnitOfMeasureUniqueId)
    }

    /// 设置计量单位 unique ID。
    ///
    /// 对应 Java: `public void setUnitOfMeasureUniqueID(Integer uniqueID)`
    pub fn set_unit_of_measure_unique_id(&mut self, unique_id: Option<i32>) {
        self.set_field_value(
            key(ResourceField::UnitOfMeasureUniqueId),
            unique_id.map(FieldValue::Integer),
        );
    }

    /// 取计量单位。
    ///
    /// 对应 Java: `public UnitOfMeasure getUnitOfMeasure()`
    pub fn get_unit_of_measure(&self) -> Option<UnitOfMeasure> {
        let context = self.context.upgrade()?;
        let context_ref = context.try_borrow().ok()?;
        context_ref
            .get_units_of_measure()
            .get_by_unique_id(self.get_unit_of_measure_unique_id())
            .cloned()
    }

    /// 设置计量单位。
    ///
    /// 对应 Java: `public void setUnitOfMeasure(UnitOfMeasure unitOfMeasure)`
    pub fn set_unit_of_measure(&mut self, unit_of_measure: Option<&UnitOfMeasure>) {
        self.set_unit_of_measure_unique_id(
            unit_of_measure.and_then(|unit_of_measure| unit_of_measure.get_unique_id()),
        );
    }

    /// 取主要角色 unique ID。
    ///
    /// 对应 Java: `public Integer getPrimaryRoleUniqueID()`
    pub fn get_primary_role_unique_id(&self) -> Option<i32> {
        self.get_i32(ResourceField::PrimaryRoleUniqueId)
    }

    /// 设置主要角色 unique ID。
    ///
    /// 对应 Java: `public void setPrimaryRoleUniqueID(Integer uniqueID)`
    pub fn set_primary_role_unique_id(&mut self, unique_id: Option<i32>) {
        self.set_field_value(
            key(ResourceField::PrimaryRoleUniqueId),
            unique_id.map(FieldValue::Integer),
        );
    }

    /// 取主要角色。
    ///
    /// 对应 Java: `public Resource getPrimaryRole()`
    pub fn get_primary_role(&self) -> Option<Rc<RefCell<Resource>>> {
        let context = self.context.upgrade()?;
        let context_ref = context.try_borrow().ok()?;
        context_ref
            .get_resources()
            .get_by_unique_id(self.get_primary_role_unique_id())
    }

    /// 设置主要角色。
    ///
    /// 对应 Java: `public void setPrimaryRole(Resource role)`
    pub fn set_primary_role(&mut self, role: Option<&Rc<RefCell<Resource>>>) {
        self.set_primary_role_unique_id(role.and_then(|role| role.borrow().get_unique_id()));
    }

    /// 取货币 unique ID。
    ///
    /// 对应 Java: `public Integer getCurrencyUniqueID()`
    pub fn get_currency_unique_id(&self) -> Option<i32> {
        self.get_i32(ResourceField::CurrencyUniqueId)
    }

    /// 设置货币 unique ID。
    ///
    /// 对应 Java: `public void setCurrencyUniqueID(Integer uniqueID)`
    pub fn set_currency_unique_id(&mut self, unique_id: Option<i32>) {
        self.set_field_value(
            key(ResourceField::CurrencyUniqueId),
            unique_id.map(FieldValue::Integer),
        );
    }

    /// 取计划工时。
    ///
    /// 对应 Java: `public Duration getPlannedWork()`
    pub fn get_planned_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::PlannedWork)
    }

    /// 取计划成本。
    ///
    /// 对应 Java: `public Number getPlannedCost()`
    pub fn get_planned_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::PlannedCost)
    }

    /// 取实际常规工时。
    ///
    /// 对应 Java: `public Duration getActualRegularWork()`
    pub fn get_actual_regular_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::ActualRegularWork)
    }

    /// 取剩余常规工时。
    ///
    /// 对应 Java: `public Duration getRemainingRegularWork()`
    pub fn get_remaining_regular_work(&self) -> Option<Duration> {
        self.get_duration(ResourceField::RemainingRegularWork)
    }

    /// 取实际常规成本。
    ///
    /// 对应 Java: `public Number getActualRegularCost()`
    pub fn get_actual_regular_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::ActualRegularCost)
    }

    /// 取剩余常规成本。
    ///
    /// 对应 Java: `public Number getRemainingRegularCost()`
    pub fn get_remaining_regular_cost(&self) -> Option<f64> {
        self.get_f64(ResourceField::RemainingRegularCost)
    }

    /// 取计划材料用量。
    ///
    /// 对应 Java: `public Number getPlannedMaterial()`
    pub fn get_planned_material(&self) -> Option<f64> {
        self.get_f64(ResourceField::PlannedMaterial)
    }

    /// 取实际材料用量。
    ///
    /// 对应 Java: `public Number getActualMaterial()`
    pub fn get_actual_material(&self) -> Option<f64> {
        self.get_f64(ResourceField::ActualMaterial)
    }

    /// 取剩余材料用量。
    ///
    /// 对应 Java: `public Number getRemainingMaterial()`
    pub fn get_remaining_material(&self) -> Option<f64> {
        self.get_f64(ResourceField::RemainingMaterial)
    }

    /// 取材料用量。
    ///
    /// 对应 Java: `public Number getMaterial()`
    pub fn get_material(&self) -> Option<f64> {
        self.get_f64(ResourceField::Material)
    }

    /// 取基线材料用量。
    ///
    /// 对应 Java: `public Number getBaselineMaterial()`
    ///
    /// Java 侧读取的是 `AssignmentField.BASELINE_MATERIAL`（原文如此），该键不会出现在
    /// 资源的字段表中，因此恒为 `null`；Rust 侧保持同样行为。
    pub fn get_baseline_material(&self) -> Option<f64> {
        self.get(key_for_assignment_baseline_material())
            .and_then(|value| value.as_double())
    }

    /// 取基线材料用量字段（下标 1-10）。
    ///
    /// 对应 Java: `public Number getBaselineMaterial(int baselineNumber)`
    pub fn get_baseline_material_at(&self, baseline_number: i32) -> Option<f64> {
        select_field(&BASELINE_MATERIALS, baseline_number).and_then(|field| self.get_f64(field))
    }

    /// 设置字段值（对应 Java 的 `set(FieldType, Object)` 与 `handleFieldChange`）。
    ///
    /// 传 `None` 等价于 Java 传 `null`。
    pub fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
        self.set_field_value(field, value);
    }

    /// 读取字段值（含计算字段）。对应 Java: `Object get(FieldType field)`
    pub fn get(&self, field: FieldKey) -> Option<FieldValue> {
        self.get_field_value(field)
    }

    /// 只读取已保存的字段值，不触发计算。对应 Java: `Object getCachedValue(FieldType field)`
    pub fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
        self.fields.get_cached_value(field)
    }

    /// 关闭字段更新时的依赖字段清理与事件触发。
    ///
    /// 对应 Java: `AbstractFieldContainer.disableEvents()`
    pub fn disable_events(&mut self) {
        self.fields.disable_events();
    }

    /// 开启字段更新时的依赖字段清理与事件触发（默认状态）。
    ///
    /// 对应 Java: `AbstractFieldContainer.enableEvents()`
    pub fn enable_events(&mut self) {
        self.fields.enable_events();
    }

    /// 按别名解析字段键。
    ///
    /// 对应 Java: `m_context.getResources().getFieldTypeByAlias(alias)`
    fn get_field_key_by_alias(&self, alias: &str) -> Option<FieldKey> {
        let context = self.context.upgrade()?;
        let context_ref = context.try_borrow().ok()?;
        context_ref.get_resources().get_field_type_by_alias(alias)
    }

    /// 实际写入字段值。
    fn set_field_value(&mut self, field: FieldKey, value: Option<FieldValue>) {
        let old_value = self.fields.get_cached_value(field).cloned();
        if old_value == value {
            return;
        }

        // Java 的顺序是「写入字段表 → handleFieldChange → 触发事件」；Rust 侧因
        // `AbstractFieldContainer` 的回调闭包不能同时可变借用 `Resource` 的其它字段，
        // 改为「handleFieldChange → 写入并触发事件」。被处理的字段逻辑不读取字段表本身，
        // 因此行为等价。
        self.handle_field_change(field, old_value, value.clone());
        self.fields.set(field, value);
    }

    /// 读取字段值（含计算字段）。
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

        // Java 会把非「始终计算」字段的结果写回字段表；Rust 的 `&self` 无法写回，
        // 每次重新计算（结果相同，见文件头说明）。
        self.calculate(field)
    }

    /// 对应 Java: `getCalculationMethod(field).apply(this)` 的分派实现。
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
            // Java 返回空的 HashMap；`FieldValue` 无集合变体，Rust 侧返回 `None`
            // （见文件头说明）。
            ResourceField::ResourceCodeValues | ResourceField::RoleCodeValues => None,
            _ => None,
        }
    }

    /// 对应 Java: `Resource.handleFieldChange(FieldType field, Object oldValue, Object newValue)`
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

    /// 计算成本偏差。
    ///
    /// 对应 Java: `Resource.calculateCostVariance()`
    fn calculate_cost_variance(&self) -> Option<f64> {
        let cost = self.get_cost()?;
        let baseline_cost = self.get_baseline_cost()?;
        Some(NumberHelper::get_double(Some(cost - baseline_cost)))
    }

    /// 计算工时偏差。
    ///
    /// 对应 Java: `Resource.calculateWorkVariance()`
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

    /// 计算成本偏差（CV = BCWP - ACWP）。
    ///
    /// 对应 Java: `Resource.calculateCV()`
    fn calculate_cv(&self) -> f64 {
        NumberHelper::get_double(self.get_bcwp()) - NumberHelper::get_double(self.get_acwp())
    }

    /// 计算进度偏差（SV = BCWP - BCWS）。
    ///
    /// 对应 Java: `Resource.calculateSV()`
    fn calculate_sv(&self) -> Option<f64> {
        let bcwp = self.get_bcwp()?;
        let bcws = self.get_bcws()?;
        Some(NumberHelper::get_double(Some(bcwp - bcws)))
    }

    /// 计算是否过度分配。
    ///
    /// 对应 Java: `Resource.calculateOverallocated()`
    fn calculate_overallocated(&self) -> bool {
        let peak_units = NumberHelper::get_double(self.get_peak_units());
        let max_units = NumberHelper::get_double(self.get_max_units());
        peak_units > max_units
    }

    /// 资源类型的默认值。
    ///
    /// 对应 Java: `Resource.defaultType()`
    fn default_type(&self) -> ResourceType {
        ResourceType::Work
    }

    /// 角色标志的默认值。
    ///
    /// 对应 Java: `Resource.defaultRoleFlag()`
    fn default_role_flag(&self) -> bool {
        false
    }

    /// 「从单位计算成本」标志的默认值。
    ///
    /// 对应 Java: `Resource.defaultCalculateCostsFromUnits()`
    fn default_calculate_costs_from_units(&self) -> bool {
        true
    }

    /// 活动标志的默认值。
    ///
    /// 对应 Java: `Resource.defaultActive()`
    fn default_active(&self) -> bool {
        true
    }

    /// 默认可用性的默认值。
    ///
    /// 对应 Java: `Resource.defaultDefaultUnits()`
    fn default_default_units(&self) -> f64 {
        DEFAULT_DEFAULT_UNITS
    }

    /// 计算当前费率表 A 的标准费率。
    ///
    /// 对应 Java: `Resource.calculateStandardRate()`
    fn calculate_standard_rate(&self) -> Option<crate::rate::Rate> {
        let entry = self.get_current_cost_rate_table_entry(0)?;
        Some(entry.get_standard_rate())
    }

    /// 计算当前费率表 A 的加班费率。
    ///
    /// 对应 Java: `Resource.calculateOvertimeRate()`
    fn calculate_overtime_rate(&self) -> Option<crate::rate::Rate> {
        let entry = self.get_current_cost_rate_table_entry(0)?;
        Some(entry.get_overtime_rate())
    }

    /// 计算当前费率表 A 的每次使用成本。
    ///
    /// 对应 Java: `Resource.calculateCostPerUse()`
    fn calculate_cost_per_use(&self) -> Option<f64> {
        let entry = self.get_current_cost_rate_table_entry(0)?;
        entry.get_cost_per_use()
    }

    /// 计算材料标签（单位名称缩写）。
    ///
    /// 对应 Java: `Resource.calculateMaterialLabel()`
    fn calculate_material_label(&self) -> Option<String> {
        let unit_of_measure = self.get_unit_of_measure()?;
        unit_of_measure.get_abbreviation().map(str::to_owned)
    }

    /// 计算全部已分配任务的最早开始日期。
    ///
    /// 对应 Java: `Resource.calculateStart()`
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

    /// 计算全部已分配任务的最晚完成日期。
    ///
    /// 对应 Java: `Resource.calculateFinish()`
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

    /// 汇总全部资源分配的计划工时。
    ///
    /// 对应 Java: `Resource.calculatePlannedWork()`
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

    /// 汇总全部资源分配的计划成本。
    ///
    /// 对应 Java: `Resource.calculatePlannedCost()`
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

    /// 计算当前可用性表行中的可用单位数。
    ///
    /// 对应 Java: `Resource.calculateMaxUnits()`
    fn calculate_max_units(&self) -> Option<f64> {
        let entry = self.get_current_availability_table_entry()?;
        entry.get_units()
    }

    /// 计算「可用起始」日期。
    ///
    /// 对应 Java: `Resource.calculateAvailableFrom()`
    fn calculate_available_from(&self) -> Option<NaiveDateTime> {
        let date = now();
        self.availability
            .try_borrow_mut()
            .ok()
            .and_then(|mut table| table.available_from(date))
    }

    /// 计算「可用截止」日期。
    ///
    /// 对应 Java: `Resource.calculateAvailableTo()`
    fn calculate_available_to(&self) -> Option<NaiveDateTime> {
        let date = now();
        self.availability
            .try_borrow_mut()
            .ok()
            .and_then(|mut table| table.available_to(date))
    }

    /// 计算常规成本（总成本 - 加班成本）。
    ///
    /// 对应 Java: `Resource.calculateRegularCost(Supplier<Number>, Supplier<Number>)`
    fn calculate_regular_cost(&self, total_cost: Option<f64>, overtime_cost: Option<f64>) -> Option<f64> {
        if total_cost.is_none() && overtime_cost.is_none() {
            return None;
        }

        Some(NumberHelper::get_double(total_cost) - NumberHelper::get_double(overtime_cost))
    }

    /// 计算常规工时（总工时 - 加班工时）。
    ///
    /// 对应 Java: `Resource.calculateRegularWork(Supplier<Duration>, Supplier<Duration>)`
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

    /// 计算材料用量（仅材料资源有值）。
    ///
    /// 对应 Java: `Resource.calculateMaterial(Supplier<Duration>)`
    fn calculate_material(&self, work: Option<Duration>) -> Option<f64> {
        if self.get_type() != ResourceType::Material {
            return None;
        }

        work.map(|work| work.get_duration())
    }

    /// 取单字段的整数缓存值（不做计算）。
    fn get_i32(&self, field: ResourceField) -> Option<i32> {
        self.get(key(field)).and_then(|value| value.as_integer())
    }

    /// 取单字段的浮点值（含计算字段）。
    fn get_f64(&self, field: ResourceField) -> Option<f64> {
        self.get(key(field)).and_then(|value| value.as_double())
    }

    /// 取单字段的布尔值（含计算字段，`null` 视为 `false`）。
    fn get_bool(&self, field: ResourceField) -> bool {
        BooleanHelper::get_boolean(self.get(key(field)).and_then(|value| value.as_boolean()))
    }

    /// 取单字段的字符串（含计算字段）。
    fn get_string(&self, field: ResourceField) -> Option<String> {
        self.get(key(field))
            .and_then(|value| value.as_string().map(str::to_owned))
    }

    /// 取单字段的工期（含计算字段）。
    fn get_duration(&self, field: ResourceField) -> Option<Duration> {
        self.get(key(field)).and_then(|value| value.as_duration())
    }

    /// 取单字段的日期时间（含计算字段）。
    fn get_date_time(&self, field: ResourceField) -> Option<NaiveDateTime> {
        self.get(key(field)).and_then(|value| value.as_date_time())
    }

    /// 取单字段的费率（含计算字段）。
    fn get_field_rate(&self, field: ResourceField) -> Option<Rate> {
        self.get(key(field)).and_then(|value| value.as_rate())
    }

    /// 取按 unique ID 索引的资源分配（Java 的 `getResourceAssignmentStream`）。
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

/// Java `getBaselineMaterial()` 读取的键（`AssignmentField.BASELINE_MATERIAL`）。
fn key_for_assignment_baseline_material() -> FieldKey {
    FieldKey::Assignment(crate::assignment_field::AssignmentField::BaselineMaterial)
}

impl ProjectEntityWithUniqueID for Resource {
    /// 对应 Java: `@Override public Integer getUniqueID()`
    fn get_unique_id(&self) -> Option<i32> {
        self.get(key(ResourceField::UniqueId))
            .and_then(|value| value.as_integer())
    }
}

impl ProjectEntityWithMutableUniqueID for Resource {
    /// 对应 Java: `@Override public void setUniqueID(Integer val)`
    fn set_unique_id(&mut self, id: Option<i32>) {
        self.set_field_value(key(ResourceField::UniqueId), id.map(FieldValue::Integer));
    }
}

impl ProjectEntityWithID for Resource {
    /// 对应 Java: `@Override public Integer getID()`
    fn get_id(&self) -> Option<i32> {
        self.get(key(ResourceField::Id)).and_then(|value| value.as_integer())
    }

    /// 对应 Java: `@Override public void setID(Integer val)`
    ///
    /// Java 在写入字段前会先解除旧 ID 映射、登记新 ID 映射；Rust 侧同样在写入前完成
    /// 两步（依赖 [`Resource::set_self_reference`] 接线的自身弱引用，未接线时跳过映射维护）。
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
    /// 对应 Java: `AbstractFieldContainer.set(FieldType, Object)`
    fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
        self.set_field_value(field, value);
    }

    /// 对应 Java: `AbstractFieldContainer.get(FieldType)`
    fn get(&self, field: FieldKey) -> Option<FieldValue> {
        self.get_field_value(field)
    }

    /// 对应 Java: `AbstractFieldContainer.getCachedValue(FieldType)`
    fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
        self.fields.get_cached_value(field)
    }

    /// 对应 Java: `AbstractFieldContainer.addFieldListener(FieldListener)`
    fn add_field_listener(&mut self, listener: Box<dyn FieldListener>) {
        self.fields.add_field_listener(listener);
    }

    /// 对应 Java: `AbstractFieldContainer.removeFieldListener(FieldListener)`
    fn clear_field_listeners(&mut self) {
        self.fields.clear_field_listeners();
    }
}

impl PartialEq for Resource {
    /// 对应 Java: `@Override public boolean equals(Object o)`（按 ID 比较）
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Resource {}

impl Ord for Resource {
    /// 对应 Java: `@Override public int compareTo(Resource o)`
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
    /// 对应 Java: `@Override public int hashCode()`
    fn hash<H: Hasher>(&self, state: &mut H) {
        NumberHelper::get_int(self.get_id().map(f64::from)).hash(state);
    }
}

impl Display for Resource {
    /// 对应 Java: `@Override public String toString()`
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
    /// Java 侧未覆写调试输出；Rust 侧输出关键标识字段（`Box<dyn Notes>` 等内部状态不输出）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Resource")
            .field("id", &self.get_id())
            .field("unique_id", &self.get_unique_id())
            .field("name", &self.get_name())
            .finish()
    }
}

/// 把 `Option<T>` 渲染成 Java `String` 拼接的形态（`null` → `"null"`）。
fn optional_display<T: Display>(value: Option<T>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_unit::TimeUnit;

    /// 构造一个未挂接到上下文的资源（工厂方法仅在测试中使用）。
    fn resource() -> Resource {
        Resource::new(Weak::new())
    }

    /// 构造一个挂接到上下文的资源。
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

        // 通用 API 与类型化 API 看到同一份数据
        assert_eq!(
            resource.get(key(ResourceField::Name)),
            Some(FieldValue::String("Fred".to_owned()))
        );

        // 传 None 等价于 Java 传 null，字段被移除
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

        // Java 构造方法为 5 张表各放入一条默认表行
        for index in 0..CostRateTable::MAX_TABLES {
            let table = resource.get_cost_rate_table(index).expect("表存在");
            assert_eq!(table.len(), 1);
        }
        assert!(resource.get_cost_rate_table(-1).is_none());
        assert!(resource.get_cost_rate_table(CostRateTable::MAX_TABLES).is_none());

        // 可用性表初始为空
        assert!(resource.get_availability().is_empty());
        assert!(resource.get_current_availability_table_entry().is_none());
    }

    #[test]
    fn availability_table_drives_max_units_and_available_dates() {
        fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
            NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(year, month, day).expect("合法日期"),
                chrono::NaiveTime::from_hms_opt(hour, minute, 0).expect("合法时间"),
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

        // 当前日期落在区间之外（2024 年 1 月已过），因此取不到「当前」行
        assert!(resource.get_current_availability_table_entry().is_none());
        // 目标日期晚于唯一区间：availableFrom 返回该区间结束 + 1 分钟，availableTo 无值
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

        // 显式设置的值优先于计算值（与 Java 的缓存语义一致）
        resource.set_cv(Some(1.0));
        assert_eq!(resource.get_cv(), Some(1.0));
    }

    #[test]
    fn overallocated_compares_peak_and_max_units() {
        let mut resource = resource();

        // Java 未提供 setMaxUnits（MAX_UNITS 由可用性表计算）；无可用性表时按 0 处理。
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

        // 默认（WORK）资源没有材料用量
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

        // 触发一次计算并确认可以再计算
        resource.set_cost(Some(10.0));
        resource.set_baseline_cost(Some(4.0));
        assert_eq!(resource.get_cost_variance(), Some(6.0));

        // 依赖字段变更后缓存被清理，重新计算得到新值
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

        // 越界索引不 panic
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

        // 同一角色重复分配会替换
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

        // remove() 在没有自身弱引用时为空操作
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

        // 资源已关联上下文；未关联日历时 get_calendar 返回 None
        assert!(resource.borrow().get_project_context().is_some());
        assert!(resource.borrow().get_calendar().is_none());

        // unique ID 变化会同步到容器的 unique ID 索引
        resource.borrow_mut().set_unique_id(Some(5));
        child.borrow_mut().set_unique_id(Some(6));

        // 父资源通过上下文的资源容器按 unique ID 解析
        child.borrow_mut().set_parent_resource(Some(&resource));
        assert!(Rc::ptr_eq(
            &child.borrow().get_parent_resource().expect("父资源已注册"),
            &resource
        ));
    }

    #[test]
    fn display_helper_renders_none_as_null() {
        assert_eq!(optional_display(None::<i32>), "null");
        assert_eq!(optional_display(Some(5)), "5");
    }
}
