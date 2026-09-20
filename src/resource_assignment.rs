//! 对应 Java: `org/mpxj/ResourceAssignment.java`
//!
//! 资源分配（任务 × 资源）实体。Java 侧继承 `AbstractFieldContainer` 并实现
//! `ProjectEntityWithMutableUniqueID` / `TimePeriodEntity`。
//!
//! # 与 Java 的差异（取舍一览）
//!
//! ## 1) 继承改为组合
//!
//! | Java | Rust |
//! |---|---|
//! | `extends AbstractFieldContainer<ResourceAssignment>` | 字段存储改为组合一个 [`AbstractFieldContainer`]，`set` / `get` / `get_cached_value` 转发给它 |
//! | `implements ProjectEntityWithMutableUniqueID` | [`ProjectEntityWithMutableUniqueID`]（含 [`ProjectEntityWithUniqueID`]） |
//! | `implements TimePeriodEntity` | 无对应 trait（`time_period_entity` 未在本仓声明），两个 timephased 取值方法作为普通方法提供 |
//! | `implements FieldContainer` | [`FieldContainer`]（`set` / `get` / `get_cached_value` / 监听器） |
//!
//! Java 的三个抽象方法按 [`crate::abstract_field_container`] 的约定落到本类：
//! `getAlwaysCalculatedField` → [`ResourceAssignment::is_always_calculated`]；
//! `getCalculationMethod` → [`ResourceAssignment::calculate`]（用 `match` 分派）；
//! `handleFieldChange` → [`ResourceAssignment::handle_field_change`]。
//!
//! ## 2) 对象引用一律用 `Weak` 断环（重要）
//!
//! Java 侧 `m_parentFile`、`getTask()` / `getResource()` 得到的是**强引用**。若 Rust 侧照搬
//! `Rc`，会形成 `ResourceAssignment → Task/Resource/ProjectFile → 容器 → ResourceAssignment`
//! 的 `Rc` 环，导致实体永不释放。因此本类一律持有 `Weak`：
//!
//! | Java | Rust 字段 |
//! |---|---|
//! | `private final ProjectFile m_parentFile` | `parent_file: Weak<RefCell<ProjectFile>>` |
//! | `ProjectContext`（Java 经 `ProjectFile` 间接访问） | `context: Weak<RefCell<ProjectContext>>`（Rust 特有，用于按别名解析自定义字段） |
//! | `getTask()`（按 unique ID 查父文件） | `task: Weak<RefCell<Task>>`，查不到时回退到父文件按 unique ID 查找 |
//! | `getResource()`（按 unique ID 查父文件） | `resource: Weak<RefCell<Resource>>`，同上回退 |
//!
//! Java 在构造方法里用 `this` / `file` 完成接线；Rust 无法在 `new()` 内取得自身的 `Rc`，
//! 故先以空 `Weak` 构造，再由 [`ResourceAssignment::set_parent_file`] /
//! [`ResourceAssignment::set_context`] / [`ResourceAssignment::set_task`] /
//! [`ResourceAssignment::set_resource`] 接线（Rust 特有方法）。
//!
//! ## 3) 字段值一律存放于字段表
//!
//! Java 的 `get` / `set` 把所有字段值（含 `WorkContour` / `RateSource` /
//! `ResourceRequestType` / `TimeUnit`(variable rate) / `UUID`(GUID) / `Notes` 对象 /
//! `List<TimephasedWork>` / `List<TimephasedCost>` / `List<LocalDateTimeRange>`）
//! 都放在 `AbstractFieldContainer` 的字段表里，Rust 侧同样如此，因此
//! [`ResourceAssignment::get_field`] / [`ResourceAssignment::peek_cached_value`]
//! 与 Java 的 `get` / `getCachedValue` 一致地能取到这些值
//! （求值入口见 [`ResourceAssignment::calculate`]，与 Java 的 `CALCULATED_FIELD_MAP` 对应）。
//!
//! 唯一例外是 `AssignmentField::RESOURCE_ASSIGNMENT_CODE_VALUES`：它依赖未移植的
//! `ResourceAssignmentCode` / `ResourceAssignmentCodeValue`，整体跳过。
//! 另外 Java 的 `m_workgroup`（MPX 记录 76 的工作组字段）**不在**字段表里，Rust 侧
//! 同样是本类的独立字段（见 [`ResourceAssignment::add_workgroup_assignment`]）。
//!
//! ## 4) 跳过的 Java 成员及原因
//!
//! | Java | 原因 |
//! |---|---|
//! | `getCostAccount()` / `setCostAccount(CostAccount)` | `CostAccount` / `CostAccountContainer` 未在本仓声明（`getCostAccountUniqueID` / `setCostAccountUniqueID` 已移植） |
//! | `getResourceAssignmentCodeValues()` / `addResourceAssignmentCodeValue(...)` | `ResourceAssignmentCode` / `ResourceAssignmentCodeValue` 未在本仓声明 |
//! | `public CostRateTable getCostRateTable()` | [`ResourceAssignment::with_cost_rate_table`]（Rust 特有回调形式：资源按值持有 `CostRateTable` 且该类型未实现 `Clone`，无法返回所有权；`getEffectiveRate` / timephased 成本的调用点语义不变） |
//! | `getEffectiveCalendar()` 里的 `CombinedCalendar` / `ManuallyScheduledTaskCalendar` 包装 | 两者位于 `org.mpxj.common`，本仓未声明；Rust 侧返回未包装的日历（见方法注释） |
//! | `calculatePercentWorkComplete` / `calculateStartVariance` / `calculateFinishVariance` 使用的 `ProjectProperties` | 经 `ProjectFile.get_project_properties()` 取值（与 `cpm::microsoft_slack_calculator` 的假设一致）；父文件未接线时退化为有效日历 / `TimeUnit::Days` |
//! | `getRatePerHour` 使用的 `RateHelper.convertToHours` | `RateHelper` 未移植；本文件内私有函数 [`rate_to_hours`] 按其逻辑等价实现 |
//! | `TimephasedUtility`（`org.mpxj.TimephasedUtility`，本仓未声明） | 在文件末尾以私有函数等价移植：`segment_work` / `segment_cost` / `segment_material` / `add_timephased_durations` / `add_timephased_numbers` |
//! | `TimephasedUtility` 的 `validateRanges` / `validateTimephasedWork` / `validateTimephasedCost` | Java 抛 `IllegalArgumentException`；Rust 库代码不 panic，故不校验 |
//! | `getFieldByAlias` / `setFieldByAlias` | 经上下文的自定义字段容器按别名解析（Java 经 `ProjectFile.getResourceAssignments().getFieldTypeByAlias()`，二者等价） |
//! | `toString()` | [`Display`]（Java 的 `null` 渲染为 `"null"`，与 Rust 一致） |
//! | 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable` | 本类未使用 |
//!
//! ## 5) 其它差异
//!
//! * **unique ID 变化时同步容器**：Java 的 `handleFieldChange` 用 `this` 调用
//!   `ResourceAssignmentContainer.updateUniqueID(this, old, new)`；Rust 无法从 `&mut self`
//!   取得 `Rc`，故改为按 unique ID 同步序号生成器（见
//!   [`crate::resource_assignment_container::ResourceAssignmentContainer::update_unique_id`]）。
//! * **`get` 的写回副作用**：Java 的 `get` 会把计算出的字段值写回字段表（并触发依赖清理）；
//!   Rust 的 `get_field(&self)` 无法写回，故只返回值、不缓存（返回值语义一致）。
//! * **`calculateWorkVariance` 的 `set(WORK_VARIANCE, ...)`** 同理不再在计算中写回。
//! * **方法重载**：Java 的重载在 Rust 中加后缀区分：`getBaselineWork(int)` →
//!   [`ResourceAssignment::get_baseline_work_by_index`]、`setStart(int, ...)` / `getStart(int)`
//!   → `*_by_index`、`getCost(int)` → [`ResourceAssignment::get_cost_by_index`]、
//!   `getTimephasedCost(List)` → [`ResourceAssignment::get_timephased_cost_total`]
//!   （私有重载 `getTimephasedCost(ranges, rateIndex, workSupplier)` 保留原名）。
//! * **异常**：Java 抛 `IllegalArgumentException`（`selectField` 越界）、`IndexOutOfBoundsException`
//!   （timephased 计提方法找不到相交区间）、`NullPointerException`（日历/日期缺失）的分支，
//!   Rust 侧一律退化为 `None` / 空值列表 / no-op（不 panic）。
//! * 未派生 `PartialEq` / `Eq` / `Hash`：Java 未覆写 `equals` / `hashCode`（保持引用相等语义）。
//!
//! ## 6) 本文件对并行翻译模块的接口假设（集成时如不一致需对齐）
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
//! // Task（与已落地的 task.rs 一致）
//! Task::get_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>>
//! Task::get_effective_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>>
//! Task::get_ignore_resource_calendar(&self) -> bool
//! Task::get_start(&self) -> Option<NaiveDateTime> / Task::get_finish(&self) -> Option<NaiveDateTime>
//! Task::get_name(&self) -> Option<String>
//! // Resource（与已落地的 resource.rs 一致）
//! Resource::get_type(&self) -> ResourceType
//! Resource::get_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>>
//! Resource::get_cost_rate_table(&self, i32) -> Option<&CostRateTable>
//! Resource::get_accrue_at(&self) -> Option<AccrueType>
//! Resource::get_cost_per_use(&self) -> Option<f64>
//! Resource::get_unique_id(&self) -> Option<i32> / Resource::get_name(&self) -> Option<String>
//! // TimephasedWork / TimephasedCost（均 Clone，字段可为空）
//! TimephasedWork::new() / TimephasedWork::from_work(&TimephasedWork, f64)   // Java: new TimephasedWork(src, factor)
//! TimephasedWork::get_start(&self) -> Option<NaiveDateTime> / set_start(Option<NaiveDateTime>)
//! TimephasedWork::get_finish(&self) -> Option<NaiveDateTime> / set_finish(Option<NaiveDateTime>)
//! TimephasedWork::get_total_amount(&self) -> Option<Duration> / set_total_amount(Option<Duration>)
//! TimephasedWork::get_amount_per_hour(&self) -> Option<Duration> / set_amount_per_hour(Option<Duration>)
//! TimephasedCost 同上，金额为 Option<f64>
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

/// 资源分配（任务与资源的关联记录）。
///
/// 对应 Java: `public class ResourceAssignment extends AbstractFieldContainer<ResourceAssignment>
/// implements ProjectEntityWithMutableUniqueID, TimePeriodEntity`
pub struct ResourceAssignment {
    /// 父项目（弱引用，断 `ProjectFile ↔ ResourceAssignment` 的 `Rc` 环）。
    ///
    /// 对应 Java 字段 `private final ProjectFile m_parentFile;`
    parent_file: Weak<RefCell<ProjectFile>>,

    /// 所属上下文（弱引用，Rust 特有：用于按别名解析自定义字段）。
    context: Weak<RefCell<ProjectContext>>,

    /// 关联的任务（弱引用，对应 Java `getTask()` 的强引用）。
    task: Weak<RefCell<Task>>,

    /// 关联的资源（弱引用，对应 Java `getResource()` 的强引用）。
    resource: Weak<RefCell<Resource>>,

    /// 字段存储（组合替代 Java 的继承）。
    ///
    /// 笔记对象（`AssignmentField.NOTES`）、工时分布（`WORK_CONTOUR`）、费率来源
    /// （`RATE_SOURCE`）、资源请求类型（`RESOURCE_REQUEST_TYPE`）、变动费率单位
    /// （`VARIABLE_RATE_UNITS`）、GUID 以及原始 timephased 列表（`RAW_TIMEPHASED_*`）
    /// 都与 Java 一致地存放在该字段表中（见 [`crate::field_value::FieldValue`]）。
    fields: AbstractFieldContainer,

    /// 工作组字段（MPX 记录 76）。
    ///
    /// 对应 Java: `private ResourceAssignmentWorkgroupFields m_workgroup;`
    ///
    /// Java 侧不是字段表成员，而是本类的独立字段（由 `addWorkgroupAssignment()` 惰性创建），
    /// 故 Rust 侧同样保留为独立字段。
    workgroup_assignment: Option<ResourceAssignmentWorkgroupFields>,
}

impl Default for ResourceAssignment {
    /// 对应 Java: 包级构造方法 `ResourceAssignment(ProjectFile file)` 的「无父文件」形态
    /// （见 [`ResourceAssignment::new`]）。
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceAssignment {
    /// 构造方法（Rust 特有形态：不自动分配 unique ID）。
    ///
    /// 对应 Java: 包级构造方法 `ResourceAssignment(ProjectFile file)`（其中会按
    /// `file.getProjectConfig().getAutoAssignmentUniqueID()` 分配 unique ID）。
    ///
    /// Rust 无法在 `new()` 内取得自身的 `Rc`，也无法完成接线，故不自动分配 unique ID；
    /// 需要与 Java 等价的构造行为时请用 [`ResourceAssignment::with_parent_file`]。
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

    /// 与 Java 构造方法等价的形式：接线父文件，并在需要时自动分配 unique ID。
    ///
    /// 对应 Java: `ResourceAssignment(ProjectFile file)`
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

    /// 接线父文件（Rust 特有：Java 在构造方法中直接接收 `file`）。
    ///
    /// 对应 Java: 构造方法中的 `m_parentFile = file;`
    pub fn set_parent_file(&mut self, parent_file: &Rc<RefCell<ProjectFile>>) {
        self.parent_file = Rc::downgrade(parent_file);
    }

    /// 接线所属上下文（Rust 特有：Java 经 `ProjectFile` 间接访问上下文）。
    pub fn set_context(&mut self, context: &Rc<RefCell<ProjectContext>>) {
        self.context = Rc::downgrade(context);
    }

    /// 接线关联任务（Rust 特有：Java 由 `getTask()` 按 unique ID 动态查找）。
    pub fn set_task(&mut self, task: &Rc<RefCell<Task>>) {
        self.task = Rc::downgrade(task);
    }

    /// 接线关联资源（Rust 特有：Java 由 `getResource()` 按 unique ID 动态查找）。
    pub fn set_resource(&mut self, resource: &Rc<RefCell<Resource>>) {
        self.resource = Rc::downgrade(resource);
    }

    /// 取父项目（弱引用升级；父文件已释放时返回 `None`）。
    ///
    /// 对应 Java: `public ProjectFile getParentFile()`
    pub fn get_parent_file(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.parent_file.upgrade()
    }

    /// 取所属上下文（Rust 特有访问器）。
    ///
    /// 对应 Java: 无（Java 经 `ProjectFile` 访问上下文）。
    pub fn get_context(&self) -> Option<Rc<RefCell<ProjectContext>>> {
        self.context.upgrade()
    }

    /// 创建（或在已有记录时返回）工作组字段记录。
    ///
    /// 对应 Java: `public ResourceAssignmentWorkgroupFields addWorkgroupAssignment()`
    ///
    /// Java 在已有记录时抛 `MPXJException(MAXIMUM_RECORDS)`；Rust 库代码不 panic，
    /// 改为返回既有记录（见文件头「跳过的 Java 成员」）。
    pub fn add_workgroup_assignment(&mut self) -> &mut ResourceAssignmentWorkgroupFields {
        self.workgroup_assignment
            .get_or_insert_with(ResourceAssignmentWorkgroupFields::new)
    }

    /// 取工作组字段记录；未创建时返回 `None`。
    ///
    /// 对应 Java: `public ResourceAssignmentWorkgroupFields getWorkgroupAssignment()`
    pub fn get_workgroup_assignment(&self) -> Option<&ResourceAssignmentWorkgroupFields> {
        self.workgroup_assignment.as_ref()
    }

    /// 把值写入字段表，并在值真正变化时执行 Java 的 `handleFieldChange`。
    ///
    /// 对应 Java: `AbstractFieldContainer.set(FieldType field, Object value)`
    ///（内部顺序为：更新字段表 → `handleFieldChange` → `fireFieldChangeEvent`）。
    fn set_key(&mut self, field: FieldKey, value: Option<FieldValue>) {
        let old_value = self.fields.get_cached_value(field).cloned();
        if old_value == value {
            return;
        }

        self.fields.set(field, value.clone());
        self.handle_field_change(field, old_value.as_ref(), value.as_ref());
    }

    /// 设置字段值（[`FieldContainer`] 的公开入口）。
    ///
    /// 对应 Java: `public void set(FieldType field, Object value)`
    pub fn set_field(&mut self, field: FieldKey, value: Option<FieldValue>) {
        self.set_key(field, value);
    }

    /// 清空受本次字段变化影响的缓存计算值。
    ///
    /// 对应 Java: `@Override protected void handleFieldChange(FieldType field, Object oldValue, Object newValue)`
    fn handle_field_change(
        &mut self,
        field: FieldKey,
        old_value: Option<&FieldValue>,
        new_value: Option<&FieldValue>,
    ) {
        if field == FieldKey::Assignment(AssignmentField::UniqueId) {
            // Java: m_parentFile.getResourceAssignments().updateUniqueID(this, oldValue, newValue);
            // Rust 侧不传元素（无法从 `&mut self` 取得 `Rc`），改为按 unique ID 同步序号生成器。
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

    /// 该字段是否「总是重新计算」（不读缓存）。
    ///
    /// 对应 Java: `@Override boolean getAlwaysCalculatedField(FieldType field)`
    fn is_always_calculated(field: FieldKey) -> bool {
        matches!(
            field,
            FieldKey::Assignment(AssignmentField::Start)
                | FieldKey::Assignment(AssignmentField::Finish)
        )
    }

    /// 从字段表读取字段值（含计算字段）。
    ///
    /// 对应 Java: `public Object get(FieldType field)`
    ///
    /// 注：Java 侧会把计算结果写回字段表；Rust 的 `&self` 无法写回，故只计算不缓存。
    pub fn get_field(&self, field: FieldKey) -> Option<FieldValue> {
        if !Self::is_always_calculated(field) {
            if let Some(value) = self.fields.get_cached_value(field) {
                return Some(value.clone());
            }
        }

        self.calculate(field)
    }

    /// 关闭字段更新时的依赖字段清理与事件触发。
    ///
    /// 对应 Java: `public void disableEvents()`
    pub fn disable_events(&mut self) {
        self.fields.disable_events();
    }

    /// 开启字段更新时的依赖字段清理与事件触发（默认状态）。
    ///
    /// 对应 Java: `public void enableEvents()`
    pub fn enable_events(&mut self) {
        self.fields.enable_events();
    }

    /// 只读取已保存的字段值，不触发计算。
    ///
    /// 对应 Java: `public Object getCachedValue(FieldType field)`
    pub fn peek_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
        self.fields.get_cached_value(field)
    }

    /// 计算字段取值；非计算字段返回 `None`。
    ///
    /// 对应 Java: `@Override Function<ResourceAssignment, Object> getCalculationMethod(FieldType field)`
    /// 与 `CALCULATED_FIELD_MAP`。
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
            // Java: `CALCULATED_FIELD_MAP` 的 `calculateTimephasedOvertimeWork`
            F::RawTimephasedRemainingOvertimeWork => {
                Some(FieldValue::TimephasedWork(self.calculate_timephased_overtime_work()))
            }
            // Java: 其余 `RAW_TIMEPHASED_*` 的默认值分别为
            // `defaultTimephasedWork()` / `defaultTimephasedCost()`（空列表）。
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

    /// 读取指定属性字段（内部便捷方法）。
    fn assignment_value(&self, field: AssignmentField) -> Option<FieldValue> {
        self.get_field(FieldKey::Assignment(field))
    }

    /// 按 `f64` 读取属性字段（对应 Java 的 `Number`）。
    fn number_value(&self, field: AssignmentField) -> Option<f64> {
        self.assignment_value(field).and_then(|value| value.as_double())
    }

    /// 按 `i32` 读取属性字段（对应 Java 的 `Integer`）。
    fn integer_value(&self, field: AssignmentField) -> Option<i32> {
        self.assignment_value(field).and_then(|value| value.as_integer())
    }

    /// 按布尔读取属性字段（对应 Java 的 `BooleanHelper.getBoolean`）。
    fn boolean_value(&self, field: AssignmentField) -> bool {
        BooleanHelper::get_boolean(self.assignment_value(field).and_then(|value| value.as_boolean()))
    }

    /// 按字符串读取属性字段。
    fn string_value(&self, field: AssignmentField) -> Option<String> {
        self.assignment_value(field).and_then(|value| match value {
            FieldValue::String(text) => Some(text),
            _ => None,
        })
    }

    /// 按工期读取属性字段。
    fn duration_value(&self, field: AssignmentField) -> Option<Duration> {
        self.assignment_value(field).and_then(|value| value.as_duration())
    }

    /// 按日期时间读取属性字段。
    fn date_time_value(&self, field: AssignmentField) -> Option<NaiveDateTime> {
        self.assignment_value(field).and_then(|value| value.as_date_time())
    }

    /// 按费率读取属性字段。
    fn rate_value(&self, field: AssignmentField) -> Option<Rate> {
        self.assignment_value(field).and_then(|value| value.as_rate())
    }

    /// 写入属性字段。
    fn set_assignment_value(&mut self, field: AssignmentField, value: Option<FieldValue>) {
        self.set_key(FieldKey::Assignment(field), value);
    }

    /// 写入字符串属性字段。
    fn set_string_value(&mut self, field: AssignmentField, value: Option<String>) {
        self.set_assignment_value(field, value.map(FieldValue::String));
    }

    /// 写入 `int` 属性字段（Java 的装箱 `Integer`）。
    fn set_integer_value(&mut self, field: AssignmentField, value: i32) {
        self.set_assignment_value(field, Some(FieldValue::Integer(value)));
    }

    /// 写入布尔属性字段（Java 的装箱 `Boolean`）。
    ///
    /// 对应 Java: `private void set(FieldType field, boolean value)`
    fn set_boolean_value(&mut self, field: AssignmentField, value: bool) {
        self.set_assignment_value(field, Some(FieldValue::Boolean(value)));
    }

    /// 把下标（1 起）映射为字段枚举。
    ///
    /// 对应 Java: `private AssignmentField selectField(AssignmentField[] fields, int index)`
    ///
    /// Java 越界抛 `IllegalArgumentException`，Rust 侧返回 `None`（不 panic）。
    fn select_field(fields: &[AssignmentField], index: i32) -> Option<AssignmentField> {
        if index < 1 || index as usize > fields.len() {
            return None;
        }
        fields.get((index - 1) as usize).copied()
    }

    /// 取资源分配的 unique ID。
    ///
    /// 对应 Java: `@Override public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.integer_value(AssignmentField::UniqueId)
    }

    /// 设置资源分配的 unique ID。
    ///
    /// 对应 Java: `@Override public void setUniqueID(Integer uniqueID)`
    pub fn set_unique_id(&mut self, unique_id: Option<i32>) {
        self.set_assignment_value(AssignmentField::UniqueId, unique_id.map(FieldValue::Integer));
    }

    /// 取分配单位（百分比）。
    ///
    /// 对应 Java: `public Number getUnits()`
    pub fn get_units(&self) -> Option<f64> {
        self.number_value(AssignmentField::AssignmentUnits)
    }

    /// 设置分配单位。
    ///
    /// 对应 Java: `public void setUnits(Number val)`
    pub fn set_units(&mut self, val: Option<f64>) {
        self.set_assignment_value(AssignmentField::AssignmentUnits, val.map(FieldValue::Double));
    }

    /// 取剩余分配单位。
    ///
    /// 对应 Java: `public Number getRemainingUnits()`
    pub fn get_remaining_units(&self) -> Option<f64> {
        self.number_value(AssignmentField::RemainingAssignmentUnits)
    }

    /// 设置剩余分配单位。
    ///
    /// 对应 Java: `public void setRemainingUnits(Number val)`
    pub fn set_remaining_units(&mut self, val: Option<f64>) {
        self.set_assignment_value(
            AssignmentField::RemainingAssignmentUnits,
            val.map(FieldValue::Double),
        );
    }

    /// 取工时。
    ///
    /// 对应 Java: `public Duration getWork()`
    pub fn get_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::Work)
    }

    /// 设置工时。
    ///
    /// 对应 Java: `public void setWork(Duration dur)`
    pub fn set_work(&mut self, dur: Option<Duration>) {
        self.set_assignment_value(AssignmentField::Work, dur.map(FieldValue::Duration));
    }

    /// 取基准开始时间。
    ///
    /// 对应 Java: `public LocalDateTime getBaselineStart()`
    pub fn get_baseline_start(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::BaselineStart)
    }

    /// 设置基准开始时间。
    ///
    /// 对应 Java: `public void setBaselineStart(LocalDateTime start)`
    pub fn set_baseline_start(&mut self, start: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::BaselineStart, start.map(FieldValue::DateTime));
    }

    /// 取实际开始时间。
    ///
    /// 对应 Java: `public LocalDateTime getActualStart()`
    pub fn get_actual_start(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::ActualStart)
    }

    /// 设置实际开始时间。
    ///
    /// 对应 Java: `public void setActualStart(LocalDateTime start)`
    pub fn set_actual_start(&mut self, start: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::ActualStart, start.map(FieldValue::DateTime));
    }

    /// 取基准完成时间。
    ///
    /// 对应 Java: `public LocalDateTime getBaselineFinish()`
    pub fn get_baseline_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::BaselineFinish)
    }

    /// 设置基准完成时间。
    ///
    /// 对应 Java: `public void setBaselineFinish(LocalDateTime finish)`
    pub fn set_baseline_finish(&mut self, finish: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::BaselineFinish, finish.map(FieldValue::DateTime));
    }

    /// 取实际完成时间。
    ///
    /// 对应 Java: `public LocalDateTime getActualFinish()`
    pub fn get_actual_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::ActualFinish)
    }

    /// 设置实际完成时间。
    ///
    /// 对应 Java: `public void setActualFinish(LocalDateTime finish)`
    pub fn set_actual_finish(&mut self, finish: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::ActualFinish, finish.map(FieldValue::DateTime));
    }

    /// 取基准工时。
    ///
    /// 对应 Java: `public Duration getBaselineWork()`
    pub fn get_baseline_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::BaselineWork)
    }

    /// 设置基准工时。
    ///
    /// 对应 Java: `public void setBaselineWork(Duration val)`
    pub fn set_baseline_work(&mut self, val: Option<Duration>) {
        self.set_assignment_value(AssignmentField::BaselineWork, val.map(FieldValue::Duration));
    }

    /// 取实际工时。
    ///
    /// 对应 Java: `public Duration getActualWork()`
    pub fn get_actual_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::ActualWork)
    }

    /// 设置实际工时。
    ///
    /// 对应 Java: `public void setActualWork(Duration val)`
    pub fn set_actual_work(&mut self, val: Option<Duration>) {
        self.set_assignment_value(AssignmentField::ActualWork, val.map(FieldValue::Duration));
    }

    /// 取加班工时。
    ///
    /// 对应 Java: `public Duration getOvertimeWork()`
    pub fn get_overtime_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::OvertimeWork)
    }

    /// 设置加班工时。
    ///
    /// 对应 Java: `public void setOvertimeWork(Duration overtimeWork)`
    pub fn set_overtime_work(&mut self, overtime_work: Option<Duration>) {
        self.set_assignment_value(
            AssignmentField::OvertimeWork,
            overtime_work.map(FieldValue::Duration),
        );
    }

    /// 取成本。
    ///
    /// 对应 Java: `public Number getCost()`
    pub fn get_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::Cost)
    }

    /// 设置成本。
    ///
    /// 对应 Java: `public void setCost(Number cost)`
    pub fn set_cost(&mut self, cost: Option<f64>) {
        self.set_assignment_value(AssignmentField::Cost, cost.map(FieldValue::Double));
    }

    /// 取基准成本。
    ///
    /// 对应 Java: `public Number getBaselineCost()`
    pub fn get_baseline_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::BaselineCost)
    }

    /// 设置基准成本。
    ///
    /// 对应 Java: `public void setBaselineCost(Number val)`
    pub fn set_baseline_cost(&mut self, val: Option<f64>) {
        self.set_assignment_value(AssignmentField::BaselineCost, val.map(FieldValue::Double));
    }

    /// 取实际成本。
    ///
    /// 对应 Java: `public Number getActualCost()`
    pub fn get_actual_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::ActualCost)
    }

    /// 设置实际成本。
    ///
    /// 对应 Java: `public void setActualCost(Number actualCost)`
    pub fn set_actual_cost(&mut self, actual_cost: Option<f64>) {
        self.set_assignment_value(AssignmentField::ActualCost, actual_cost.map(FieldValue::Double));
    }

    /// 取开始时间（总是重新计算）。
    ///
    /// 对应 Java: `@Override public LocalDateTime getStart()`
    pub fn get_start(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::Start)
    }

    /// 设置开始时间。
    ///
    /// 对应 Java: `public void setStart(LocalDateTime val)`
    pub fn set_start(&mut self, val: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::Start, val.map(FieldValue::DateTime));
    }

    /// 取完成时间（总是重新计算）。
    ///
    /// 对应 Java: `@Override public LocalDateTime getFinish()`
    pub fn get_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::Finish)
    }

    /// 设置完成时间。
    ///
    /// 对应 Java: `public void setFinish(LocalDateTime val)`
    pub fn set_finish(&mut self, val: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::Finish, val.map(FieldValue::DateTime));
    }

    /// 取分配延迟。
    ///
    /// 对应 Java: `public Duration getDelay()`
    pub fn get_delay(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::AssignmentDelay)
    }

    /// 设置分配延迟。
    ///
    /// 对应 Java: `public void setDelay(Duration dur)`
    pub fn set_delay(&mut self, dur: Option<Duration>) {
        self.set_assignment_value(AssignmentField::AssignmentDelay, dur.map(FieldValue::Duration));
    }

    /// 取资源的 unique ID。
    ///
    /// 对应 Java: `public Integer getResourceUniqueID()`
    pub fn get_resource_unique_id(&self) -> Option<i32> {
        self.integer_value(AssignmentField::ResourceUniqueId)
    }

    /// 设置资源的 unique ID。
    ///
    /// 对应 Java: `public void setResourceUniqueID(Integer val)`
    pub fn set_resource_unique_id(&mut self, val: Option<i32>) {
        self.set_assignment_value(AssignmentField::ResourceUniqueId, val.map(FieldValue::Integer));
    }

    /// 取任务的 unique ID。
    ///
    /// 对应 Java: `public Integer getTaskUniqueID()`
    pub fn get_task_unique_id(&self) -> Option<i32> {
        self.integer_value(AssignmentField::TaskUniqueId)
    }

    /// 设置任务的 unique ID。
    ///
    /// 对应 Java: `public void setTaskUniqueID(Integer id)`
    pub fn set_task_unique_id(&mut self, id: Option<i32>) {
        self.set_assignment_value(AssignmentField::TaskUniqueId, id.map(FieldValue::Integer));
    }

    /// 取关联任务。
    ///
    /// 对应 Java: `public Task getTask()`
    ///
    /// Rust 侧优先使用接线好的弱引用；未接线时回退到父文件按 unique ID 查找。
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

    /// 取关联资源。
    ///
    /// 对应 Java: `public Resource getResource()`
    ///
    /// 回退策略同 [`ResourceAssignment::get_task`]。
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

    /// 取工时分布类型。
    ///
    /// 对应 Java: `public WorkContour getWorkContour()`
    pub fn get_work_contour(&self) -> Option<WorkContour> {
        match self.assignment_value(AssignmentField::WorkContour) {
            Some(FieldValue::WorkContour(work_contour)) => Some(work_contour),
            _ => None,
        }
    }

    /// 设置工时分布类型。
    ///
    /// 对应 Java: `public void setWorkContour(WorkContour workContour)`
    pub fn set_work_contour(&mut self, work_contour: Option<WorkContour>) {
        self.set_assignment_value(
            AssignmentField::WorkContour,
            work_contour.map(FieldValue::WorkContour),
        );
    }

    /// 从项目中移除本资源分配；成功返回 `true`（Java 无返回值）。
    ///
    /// 对应 Java: `public void remove()`
    ///
    /// Java 通过 `this` 的引用相等定位元素；Rust 无法从 `&self` 取得 `Rc`，
    /// 故退化为按 unique ID 移除（见文件头第 5 节）。
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

    /// 取剩余工时。
    ///
    /// 对应 Java: `public Duration getRemainingWork()`
    pub fn get_remaining_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::RemainingWork)
    }

    /// 设置剩余工时。
    ///
    /// 对应 Java: `public void setRemainingWork(Duration remainingWork)`
    pub fn set_remaining_work(&mut self, remaining_work: Option<Duration>) {
        self.set_assignment_value(
            AssignmentField::RemainingWork,
            remaining_work.map(FieldValue::Duration),
        );
    }

    /// 取调配延迟。
    ///
    /// 对应 Java: `public Duration getLevelingDelay()`
    pub fn get_leveling_delay(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::LevelingDelay)
    }

    /// 设置调配延迟。
    ///
    /// 对应 Java: `public void setLevelingDelay(Duration levelingDelay)`
    pub fn set_leveling_delay(&mut self, leveling_delay: Option<Duration>) {
        self.set_assignment_value(
            AssignmentField::LevelingDelay,
            leveling_delay.map(FieldValue::Duration),
        );
    }

    /// 取成本费率表下标（默认 0）。
    ///
    /// 对应 Java: `public Integer getRateIndex()`
    pub fn get_rate_index(&self) -> Option<i32> {
        self.integer_value(AssignmentField::RateIndex)
    }

    /// 设置成本费率表下标。
    ///
    /// 对应 Java: `public void setRateIndex(Integer index)`
    pub fn set_rate_index(&mut self, index: Option<i32>) {
        self.set_assignment_value(AssignmentField::RateIndex, index.map(FieldValue::Integer));
    }

    /// 取角色的 unique ID。
    ///
    /// 对应 Java: `public Integer getRoleUniqueID()`
    pub fn get_role_unique_id(&self) -> Option<i32> {
        self.integer_value(AssignmentField::RoleUniqueId)
    }

    /// 设置角色的 unique ID。
    ///
    /// 对应 Java: `public void setRoleUniqueID(Integer id)`
    pub fn set_role_unique_id(&mut self, id: Option<i32>) {
        self.set_assignment_value(AssignmentField::RoleUniqueId, id.map(FieldValue::Integer));
    }

    /// 取角色（以资源形态表示）。
    ///
    /// 对应 Java: `public Resource getRole()`
    pub fn get_role(&self) -> Option<Rc<RefCell<Resource>>> {
        let parent_file = self.parent_file.upgrade()?;
        let role_unique_id = self.get_role_unique_id();
        parent_file
            .try_borrow()
            .ok()
            .and_then(|parent_file| parent_file.get_resource_by_unique_id(role_unique_id))
    }

    /// 设置角色（以资源形态表示）。
    ///
    /// 对应 Java: `public void setRole(Resource role)`
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

    /// 取覆盖费率。
    ///
    /// 对应 Java: `public Rate getOverrideRate()`
    pub fn get_override_rate(&self) -> Option<Rate> {
        self.rate_value(AssignmentField::OverrideRate)
    }

    /// 设置覆盖费率。
    ///
    /// 对应 Java: `public void setOverrideRate(Rate rate)`
    pub fn set_override_rate(&mut self, rate: Option<Rate>) {
        self.set_assignment_value(AssignmentField::OverrideRate, rate.map(FieldValue::Rate));
    }

    /// 取费率来源（默认 `RateSource::Resource`）。
    ///
    /// 对应 Java: `public RateSource getRateSource()`
    ///
    /// Java 的 `get` 会回落到计算字段 `defaultRateSource()`（`RateSource.RESOURCE`）；
    /// Rust 侧同样在 [`ResourceAssignment::calculate`] 中提供该默认值。
    pub fn get_rate_source(&self) -> RateSource {
        self.assignment_value(AssignmentField::RateSource)
            .and_then(|value| value.as_rate_source())
            .unwrap_or_else(Self::default_rate_source)
    }

    /// 设置费率来源。
    ///
    /// 对应 Java: `public void setRateSource(RateSource source)`
    pub fn set_rate_source(&mut self, source: Option<RateSource>) {
        self.set_assignment_value(
            AssignmentField::RateSource,
            source.map(FieldValue::RateSource),
        );
    }

    /// 费率来源的默认取值。
    ///
    /// 对应 Java: `private RateSource defaultRateSource()`
    fn default_rate_source() -> RateSource {
        RateSource::Resource
    }

    /// 取本分配使用的有效日历。
    ///
    /// 对应 Java: `public ProjectCalendar getEffectiveCalendar()`
    ///
    /// # 与 Java 的差异
    ///
    /// Java 在「任务显式日历与资源日历同时存在」时构造 `CombinedCalendar`，并在任务为手动
    /// 排程时再包一层 `ManuallyScheduledTaskCalendar`；这两个类型位于 `org.mpxj.common`，
    /// 本仓未声明，故 Rust 侧返回未包装的日历（见文件头第 4 节）。
    pub fn get_effective_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        let task = self.get_task()?;
        let task_calendar = task.try_borrow().ok().and_then(|task| task.get_calendar());
        let resource_calendar = self.resource_calendar(&task);

        match task_calendar {
            None => match resource_calendar {
                None => task
                    .try_borrow()
                    .ok()
                    .and_then(|task| task.get_effective_calendar()),
                Some(resource_calendar) => Some(resource_calendar),
            },
            Some(task_calendar) => Some(task_calendar),
        }
    }

    /// 取任务可用的资源日历（任务忽略资源日历、或资源不是基于时间的资源时为 `None`）。
    ///
    /// 对应 Java: `getEffectiveCalendar()` 中 `resourceCalendar` 的取值逻辑。
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

    /// 取变动费率单位（固定费率时为 `None`）。
    ///
    /// 对应 Java: `public TimeUnit getVariableRateUnits()`
    pub fn get_variable_rate_units(&self) -> Option<TimeUnit> {
        self.assignment_value(AssignmentField::VariableRateUnits)
            .and_then(|value| value.as_time_unit())
    }

    /// 设置变动费率单位。
    ///
    /// 对应 Java: `public void setVariableRateUnits(TimeUnit variableRateUnits)`
    pub fn set_variable_rate_units(&mut self, variable_rate_units: Option<TimeUnit>) {
        self.set_assignment_value(
            AssignmentField::VariableRateUnits,
            variable_rate_units.map(FieldValue::TimeUnit),
        );
    }

    /// 取预算成本。
    ///
    /// 对应 Java: `public Number getBudgetCost()`
    pub fn get_budget_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::BudgetCost)
    }

    /// 设置预算成本。
    ///
    /// 对应 Java: `public void setBudgetCost(Number cost)`
    pub fn set_budget_cost(&mut self, cost: Option<f64>) {
        self.set_assignment_value(AssignmentField::BudgetCost, cost.map(FieldValue::Double));
    }

    /// 取预算工时。
    ///
    /// 对应 Java: `public Duration getBudgetWork()`
    pub fn get_budget_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::BudgetWork)
    }

    /// 设置预算工时。
    ///
    /// 对应 Java: `public void setBudgetWork(Duration work)`
    pub fn set_budget_work(&mut self, work: Option<Duration>) {
        self.set_assignment_value(AssignmentField::BudgetWork, work.map(FieldValue::Duration));
    }

    /// 取基准预算成本。
    ///
    /// 对应 Java: `public Number getBaselineBudgetCost()`
    pub fn get_baseline_budget_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::BaselineBudgetCost)
    }

    /// 设置基准预算成本。
    ///
    /// 对应 Java: `public void setBaselineBudgetCost(Number cost)`
    pub fn set_baseline_budget_cost(&mut self, cost: Option<f64>) {
        self.set_assignment_value(
            AssignmentField::BaselineBudgetCost,
            cost.map(FieldValue::Double),
        );
    }

    /// 取基准预算工时。
    ///
    /// 对应 Java: `public Duration getBaselineBudgetWork()`
    pub fn get_baseline_budget_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::BaselineBudgetWork)
    }

    /// 设置基准预算工时。
    ///
    /// 对应 Java: `public void setBaselineBudgetWork(Duration work)`
    pub fn set_baseline_budget_work(&mut self, work: Option<Duration>) {
        self.set_assignment_value(
            AssignmentField::BaselineBudgetWork,
            work.map(FieldValue::Duration),
        );
    }

    /// 设置基准成本（下标 1-10）。
    ///
    /// 对应 Java: `public void setBaselineCost(int baselineNumber, Number value)`
    pub fn set_baseline_cost_by_index(&mut self, baseline_number: i32, value: Option<f64>) {
        if let Some(field) = Self::select_field(&BASELINE_COSTS, baseline_number) {
            self.set_assignment_value(field, value.map(FieldValue::Double));
        }
    }

    /// 设置基准工时（下标 1-10）。
    ///
    /// 对应 Java: `public void setBaselineWork(int baselineNumber, Duration value)`
    pub fn set_baseline_work_by_index(&mut self, baseline_number: i32, value: Option<Duration>) {
        if let Some(field) = Self::select_field(&BASELINE_WORKS, baseline_number) {
            self.set_assignment_value(field, value.map(FieldValue::Duration));
        }
    }

    /// 取基准工时（下标 1-10）。
    ///
    /// 对应 Java: `public Duration getBaselineWork(int baselineNumber)`
    pub fn get_baseline_work_by_index(&self, baseline_number: i32) -> Option<Duration> {
        let field = Self::select_field(&BASELINE_WORKS, baseline_number)?;
        self.duration_value(field)
    }

    /// 取基准成本（下标 1-10）。
    ///
    /// 对应 Java: `public Number getBaselineCost(int baselineNumber)`
    pub fn get_baseline_cost_by_index(&self, baseline_number: i32) -> Option<f64> {
        let field = Self::select_field(&BASELINE_COSTS, baseline_number)?;
        self.number_value(field)
    }

    /// 设置基准开始时间（下标 1-10）。
    ///
    /// 对应 Java: `public void setBaselineStart(int baselineNumber, LocalDateTime value)`
    pub fn set_baseline_start_by_index(
        &mut self,
        baseline_number: i32,
        value: Option<NaiveDateTime>,
    ) {
        if let Some(field) = Self::select_field(&BASELINE_STARTS, baseline_number) {
            self.set_assignment_value(field, value.map(FieldValue::DateTime));
        }
    }

    /// 取基准开始时间（下标 1-10）。
    ///
    /// 对应 Java: `public LocalDateTime getBaselineStart(int baselineNumber)`
    pub fn get_baseline_start_by_index(&self, baseline_number: i32) -> Option<NaiveDateTime> {
        let field = Self::select_field(&BASELINE_STARTS, baseline_number)?;
        self.date_time_value(field)
    }

    /// 设置基准完成时间（下标 1-10）。
    ///
    /// 对应 Java: `public void setBaselineFinish(int baselineNumber, LocalDateTime value)`
    pub fn set_baseline_finish_by_index(
        &mut self,
        baseline_number: i32,
        value: Option<NaiveDateTime>,
    ) {
        if let Some(field) = Self::select_field(&BASELINE_FINISHES, baseline_number) {
            self.set_assignment_value(field, value.map(FieldValue::DateTime));
        }
    }

    /// 取基准完成时间（下标 1-10）。
    ///
    /// 对应 Java: `public LocalDateTime getBaselineFinish(int baselineNumber)`
    pub fn get_baseline_finish_by_index(&self, baseline_number: i32) -> Option<NaiveDateTime> {
        let field = Self::select_field(&BASELINE_FINISHES, baseline_number)?;
        self.date_time_value(field)
    }

    /// 设置基准预算成本（下标 1-10）。
    ///
    /// 对应 Java: `public void setBaselineBudgetCost(int baselineNumber, Number value)`
    pub fn set_baseline_budget_cost_by_index(&mut self, baseline_number: i32, value: Option<f64>) {
        if let Some(field) = Self::select_field(&BASELINE_BUDGET_COSTS, baseline_number) {
            self.set_assignment_value(field, value.map(FieldValue::Double));
        }
    }

    /// 设置基准预算工时（下标 1-10）。
    ///
    /// 对应 Java: `public void setBaselineBudgetWork(int baselineNumber, Duration value)`
    pub fn set_baseline_budget_work_by_index(
        &mut self,
        baseline_number: i32,
        value: Option<Duration>,
    ) {
        if let Some(field) = Self::select_field(&BASELINE_BUDGET_WORKS, baseline_number) {
            self.set_assignment_value(field, value.map(FieldValue::Duration));
        }
    }

    /// 取基准预算工时（下标 1-10）。
    ///
    /// 对应 Java: `public Duration getBaselineBudgetWork(int baselineNumber)`
    pub fn get_baseline_budget_work_by_index(&self, baseline_number: i32) -> Option<Duration> {
        let field = Self::select_field(&BASELINE_BUDGET_WORKS, baseline_number)?;
        self.duration_value(field)
    }

    /// 取基准预算成本（下标 1-10）。
    ///
    /// 对应 Java: `public Number getBaselineBudgetCost(int baselineNumber)`
    pub fn get_baseline_budget_cost_by_index(&self, baseline_number: i32) -> Option<f64> {
        let field = Self::select_field(&BASELINE_BUDGET_COSTS, baseline_number)?;
        self.number_value(field)
    }

    /// 设置自定义文本（下标 1-30）。
    ///
    /// 对应 Java: `public void setText(int index, String value)`
    pub fn set_text(&mut self, index: i32, value: Option<String>) {
        if let Some(field) = Self::select_field(&CUSTOM_TEXT, index) {
            self.set_string_value(field, value);
        }
    }

    /// 取自定义文本（下标 1-30）。
    ///
    /// 对应 Java: `public String getText(int index)`
    pub fn get_text(&self, index: i32) -> Option<String> {
        let field = Self::select_field(&CUSTOM_TEXT, index)?;
        self.string_value(field)
    }

    /// 设置自定义开始时间（下标 1-10）。
    ///
    /// 对应 Java: `public void setStart(int index, LocalDateTime value)`
    pub fn set_start_by_index(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = Self::select_field(&CUSTOM_START, index) {
            self.set_assignment_value(field, value.map(FieldValue::DateTime));
        }
    }

    /// 取自定义开始时间（下标 1-10）。
    ///
    /// 对应 Java: `public LocalDateTime getStart(int index)`
    pub fn get_start_by_index(&self, index: i32) -> Option<NaiveDateTime> {
        let field = Self::select_field(&CUSTOM_START, index)?;
        self.date_time_value(field)
    }

    /// 设置自定义完成时间（下标 1-10）。
    ///
    /// 对应 Java: `public void setFinish(int index, LocalDateTime value)`
    pub fn set_finish_by_index(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = Self::select_field(&CUSTOM_FINISH, index) {
            self.set_assignment_value(field, value.map(FieldValue::DateTime));
        }
    }

    /// 取自定义完成时间（下标 1-10）。
    ///
    /// 对应 Java: `public LocalDateTime getFinish(int index)`
    pub fn get_finish_by_index(&self, index: i32) -> Option<NaiveDateTime> {
        let field = Self::select_field(&CUSTOM_FINISH, index)?;
        self.date_time_value(field)
    }

    /// 设置自定义日期（下标 1-10）。
    ///
    /// 对应 Java: `public void setDate(int index, LocalDateTime value)`
    pub fn set_date(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = Self::select_field(&CUSTOM_DATE, index) {
            self.set_assignment_value(field, value.map(FieldValue::DateTime));
        }
    }

    /// 取自定义日期（下标 1-10）。
    ///
    /// 对应 Java: `public LocalDateTime getDate(int index)`
    pub fn get_date(&self, index: i32) -> Option<NaiveDateTime> {
        let field = Self::select_field(&CUSTOM_DATE, index)?;
        self.date_time_value(field)
    }

    /// 设置自定义数字（下标 1-20）。
    ///
    /// 对应 Java: `public void setNumber(int index, Number value)`
    pub fn set_number(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = Self::select_field(&CUSTOM_NUMBER, index) {
            self.set_assignment_value(field, value.map(FieldValue::Double));
        }
    }

    /// 取自定义数字（下标 1-20）。
    ///
    /// 对应 Java: `public Number getNumber(int index)`
    pub fn get_number(&self, index: i32) -> Option<f64> {
        let field = Self::select_field(&CUSTOM_NUMBER, index)?;
        self.number_value(field)
    }

    /// 设置自定义工期（下标 1-10）。
    ///
    /// 对应 Java: `public void setDuration(int index, Duration value)`
    pub fn set_duration(&mut self, index: i32, value: Option<Duration>) {
        if let Some(field) = Self::select_field(&CUSTOM_DURATION, index) {
            self.set_assignment_value(field, value.map(FieldValue::Duration));
        }
    }

    /// 取自定义工期（下标 1-10）。
    ///
    /// 对应 Java: `public Duration getDuration(int index)`
    pub fn get_duration(&self, index: i32) -> Option<Duration> {
        let field = Self::select_field(&CUSTOM_DURATION, index)?;
        self.duration_value(field)
    }

    /// 设置自定义成本（下标 1-10）。
    ///
    /// 对应 Java: `public void setCost(int index, Number value)`
    pub fn set_cost_by_index(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = Self::select_field(&CUSTOM_COST, index) {
            self.set_assignment_value(field, value.map(FieldValue::Double));
        }
    }

    /// 取自定义成本（下标 1-10）。
    ///
    /// 对应 Java: `public Number getCost(int index)`
    pub fn get_cost_by_index(&self, index: i32) -> Option<f64> {
        let field = Self::select_field(&CUSTOM_COST, index)?;
        self.number_value(field)
    }

    /// 设置自定义布尔标志（下标 1-20）。
    ///
    /// 对应 Java: `public void setFlag(int index, boolean value)`
    pub fn set_flag(&mut self, index: i32, value: bool) {
        if let Some(field) = Self::select_field(&CUSTOM_FLAG, index) {
            self.set_boolean_value(field, value);
        }
    }

    /// 取自定义布尔标志（下标 1-20）；未设置或越界时返回 `false`。
    ///
    /// 对应 Java: `public boolean getFlag(int index)`
    pub fn get_flag(&self, index: i32) -> bool {
        match Self::select_field(&CUSTOM_FLAG, index) {
            Some(field) => self.boolean_value(field),
            None => false,
        }
    }

    /// 设置企业自定义成本（下标 1-10）。
    ///
    /// 对应 Java: `public void setEnterpriseCost(int index, Number value)`
    pub fn set_enterprise_cost(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = Self::select_field(&ENTERPRISE_CUSTOM_COST, index) {
            self.set_assignment_value(field, value.map(FieldValue::Double));
        }
    }

    /// 取企业自定义成本（下标 1-10）。
    ///
    /// 对应 Java: `public Number getEnterpriseCost(int index)`
    pub fn get_enterprise_cost(&self, index: i32) -> Option<f64> {
        let field = Self::select_field(&ENTERPRISE_CUSTOM_COST, index)?;
        self.number_value(field)
    }

    /// 设置企业自定义日期（下标 1-30）。
    ///
    /// 对应 Java: `public void setEnterpriseDate(int index, LocalDateTime value)`
    pub fn set_enterprise_date(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = Self::select_field(&ENTERPRISE_CUSTOM_DATE, index) {
            self.set_assignment_value(field, value.map(FieldValue::DateTime));
        }
    }

    /// 取企业自定义日期（下标 1-30）。
    ///
    /// 对应 Java: `public LocalDateTime getEnterpriseDate(int index)`
    pub fn get_enterprise_date(&self, index: i32) -> Option<NaiveDateTime> {
        let field = Self::select_field(&ENTERPRISE_CUSTOM_DATE, index)?;
        self.date_time_value(field)
    }

    /// 设置企业自定义工期（下标 1-10）。
    ///
    /// 对应 Java: `public void setEnterpriseDuration(int index, Duration value)`
    pub fn set_enterprise_duration(&mut self, index: i32, value: Option<Duration>) {
        if let Some(field) = Self::select_field(&ENTERPRISE_CUSTOM_DURATION, index) {
            self.set_assignment_value(field, value.map(FieldValue::Duration));
        }
    }

    /// 取企业自定义工期（下标 1-10）。
    ///
    /// 对应 Java: `public Duration getEnterpriseDuration(int index)`
    pub fn get_enterprise_duration(&self, index: i32) -> Option<Duration> {
        let field = Self::select_field(&ENTERPRISE_CUSTOM_DURATION, index)?;
        self.duration_value(field)
    }

    /// 设置企业自定义布尔标志（下标 1-20）。
    ///
    /// 对应 Java: `public void setEnterpriseFlag(int index, boolean value)`
    pub fn set_enterprise_flag(&mut self, index: i32, value: bool) {
        if let Some(field) = Self::select_field(&ENTERPRISE_CUSTOM_FLAG, index) {
            self.set_boolean_value(field, value);
        }
    }

    /// 取企业自定义布尔标志（下标 1-20）；未设置或越界时返回 `false`。
    ///
    /// 对应 Java: `public boolean getEnterpriseFlag(int index)`
    pub fn get_enterprise_flag(&self, index: i32) -> bool {
        match Self::select_field(&ENTERPRISE_CUSTOM_FLAG, index) {
            Some(field) => self.boolean_value(field),
            None => false,
        }
    }

    /// 设置企业自定义数字（下标 1-40）。
    ///
    /// 对应 Java: `public void setEnterpriseNumber(int index, Number value)`
    pub fn set_enterprise_number(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = Self::select_field(&ENTERPRISE_CUSTOM_NUMBER, index) {
            self.set_assignment_value(field, value.map(FieldValue::Double));
        }
    }

    /// 取企业自定义数字（下标 1-40）。
    ///
    /// 对应 Java: `public Number getEnterpriseNumber(int index)`
    pub fn get_enterprise_number(&self, index: i32) -> Option<f64> {
        let field = Self::select_field(&ENTERPRISE_CUSTOM_NUMBER, index)?;
        self.number_value(field)
    }

    /// 设置企业自定义文本（下标 1-40）。
    ///
    /// 对应 Java: `public void setEnterpriseText(int index, String value)`
    pub fn set_enterprise_text(&mut self, index: i32, value: Option<String>) {
        if let Some(field) = Self::select_field(&ENTERPRISE_CUSTOM_TEXT, index) {
            self.set_string_value(field, value);
        }
    }

    /// 取企业自定义文本（下标 1-40）。
    ///
    /// 对应 Java: `public String getEnterpriseText(int index)`
    pub fn get_enterprise_text(&self, index: i32) -> Option<String> {
        let field = Self::select_field(&ENTERPRISE_CUSTOM_TEXT, index)?;
        self.string_value(field)
    }

    /// 取常规工时。
    ///
    /// 对应 Java: `public Duration getRegularWork()`
    pub fn get_regular_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::RegularWork)
    }

    /// 设置常规工时。
    ///
    /// 对应 Java: `public void setRegularWork(Duration dur)`
    pub fn set_regular_work(&mut self, dur: Option<Duration>) {
        self.set_assignment_value(AssignmentField::RegularWork, dur.map(FieldValue::Duration));
    }

    /// 取实际加班工时。
    ///
    /// 对应 Java: `public Duration getActualOvertimeWork()`
    pub fn get_actual_overtime_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::ActualOvertimeWork)
    }

    /// 设置实际加班工时。
    ///
    /// 对应 Java: `public void setActualOvertimeWork(Duration dur)`
    pub fn set_actual_overtime_work(&mut self, dur: Option<Duration>) {
        self.set_assignment_value(
            AssignmentField::ActualOvertimeWork,
            dur.map(FieldValue::Duration),
        );
    }

    /// 取剩余加班工时。
    ///
    /// 对应 Java: `public Duration getRemainingOvertimeWork()`
    pub fn get_remaining_overtime_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::RemainingOvertimeWork)
    }

    /// 设置剩余加班工时。
    ///
    /// 对应 Java: `public void setRemainingOvertimeWork(Duration dur)`
    pub fn set_remaining_overtime_work(&mut self, dur: Option<Duration>) {
        self.set_assignment_value(
            AssignmentField::RemainingOvertimeWork,
            dur.map(FieldValue::Duration),
        );
    }

    /// 取加班成本（计算字段）。
    ///
    /// 对应 Java: `public Number getOvertimeCost()`
    pub fn get_overtime_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::OvertimeCost)
    }

    /// 设置加班成本。
    ///
    /// 对应 Java: `public void setOvertimeCost(Number cost)`
    pub fn set_overtime_cost(&mut self, cost: Option<f64>) {
        self.set_assignment_value(AssignmentField::OvertimeCost, cost.map(FieldValue::Double));
    }

    /// 取剩余成本。
    ///
    /// 对应 Java: `public Number getRemainingCost()`
    pub fn get_remaining_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::RemainingCost)
    }

    /// 设置剩余成本。
    ///
    /// 对应 Java: `public void setRemainingCost(Number cost)`
    pub fn set_remaining_cost(&mut self, cost: Option<f64>) {
        self.set_assignment_value(AssignmentField::RemainingCost, cost.map(FieldValue::Double));
    }

    /// 取实际加班成本。
    ///
    /// 对应 Java: `public Number getActualOvertimeCost()`
    pub fn get_actual_overtime_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::ActualOvertimeCost)
    }

    /// 设置实际加班成本。
    ///
    /// 对应 Java: `public void setActualOvertimeCost(Number cost)`
    pub fn set_actual_overtime_cost(&mut self, cost: Option<f64>) {
        self.set_assignment_value(
            AssignmentField::ActualOvertimeCost,
            cost.map(FieldValue::Double),
        );
    }

    /// 取剩余加班成本。
    ///
    /// 对应 Java: `public Number getRemainingOvertimeCost()`
    pub fn get_remaining_overtime_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::RemainingOvertimeCost)
    }

    /// 设置剩余加班成本。
    ///
    /// 对应 Java: `public void setRemainingOvertimeCost(Number cost)`
    pub fn set_remaining_overtime_cost(&mut self, cost: Option<f64>) {
        self.set_assignment_value(
            AssignmentField::RemainingOvertimeCost,
            cost.map(FieldValue::Double),
        );
    }

    /// 设置 BCWP（已完成工作预算成本）。
    ///
    /// 对应 Java: `public void setBCWP(Number val)`
    pub fn set_bcwp(&mut self, val: Option<f64>) {
        self.set_assignment_value(AssignmentField::Bcwp, val.map(FieldValue::Double));
    }

    /// 取 BCWP。
    ///
    /// 对应 Java: `public Number getBCWP()`
    pub fn get_bcwp(&self) -> Option<f64> {
        self.number_value(AssignmentField::Bcwp)
    }

    /// 设置 BCWS（计划工作预算成本）。
    ///
    /// 对应 Java: `public void setBCWS(Number val)`
    pub fn set_bcws(&mut self, val: Option<f64>) {
        self.set_assignment_value(AssignmentField::Bcws, val.map(FieldValue::Double));
    }

    /// 取 BCWS。
    ///
    /// 对应 Java: `public Number getBCWS()`
    pub fn get_bcws(&self) -> Option<f64> {
        self.number_value(AssignmentField::Bcws)
    }

    /// 取 ACWP（已完成工作实际成本）。
    ///
    /// 对应 Java: `public Number getACWP()`
    pub fn get_acwp(&self) -> Option<f64> {
        self.number_value(AssignmentField::Acwp)
    }

    /// 设置 ACWP。
    ///
    /// 对应 Java: `public void setACWP(Number acwp)`
    pub fn set_acwp(&mut self, acwp: Option<f64>) {
        self.set_assignment_value(AssignmentField::Acwp, acwp.map(FieldValue::Double));
    }

    /// 设置 SV（进度偏差）。
    ///
    /// 对应 Java: `public void setSV(Number val)`
    pub fn set_sv(&mut self, val: Option<f64>) {
        self.set_assignment_value(AssignmentField::Sv, val.map(FieldValue::Double));
    }

    /// 取 SV（计算字段）。
    ///
    /// 对应 Java: `public Number getSV()`
    pub fn get_sv(&self) -> Option<f64> {
        self.number_value(AssignmentField::Sv)
    }

    /// 设置 CV（成本偏差）。
    ///
    /// 对应 Java: `public void setCV(Number val)`
    pub fn set_cv(&mut self, val: Option<f64>) {
        self.set_assignment_value(AssignmentField::Cv, val.map(FieldValue::Double));
    }

    /// 取 CV（计算字段）。
    ///
    /// 对应 Java: `public Number getCV()`
    pub fn get_cv(&self) -> Option<f64> {
        self.number_value(AssignmentField::Cv)
    }

    /// 设置成本偏差（VAC）。
    ///
    /// 对应 Java: `public void setCostVariance(Number val)`
    pub fn set_cost_variance(&mut self, val: Option<f64>) {
        self.set_assignment_value(AssignmentField::CostVariance, val.map(FieldValue::Double));
    }

    /// 取成本偏差（计算字段）。
    ///
    /// 对应 Java: `public Number getCostVariance()`
    pub fn get_cost_variance(&self) -> Option<f64> {
        self.number_value(AssignmentField::CostVariance)
    }

    /// 设置工时完成百分比。
    ///
    /// 对应 Java: `public void setPercentageWorkComplete(Number val)`
    pub fn set_percentage_work_complete(&mut self, val: Option<f64>) {
        self.set_assignment_value(
            AssignmentField::PercentWorkComplete,
            val.map(FieldValue::Double),
        );
    }

    /// 取工时完成百分比（计算字段）。
    ///
    /// 对应 Java: `public Number getPercentageWorkComplete()`
    pub fn get_percentage_work_complete(&self) -> Option<f64> {
        self.number_value(AssignmentField::PercentWorkComplete)
    }

    /// 设置纯文本笔记（Java 存 `new Notes(notes)`；Rust 侧用 `PlainNotes`）。
    ///
    /// 对应 Java: `public void setNotes(String notes)`
    pub fn set_notes(&mut self, notes: Option<String>) {
        let value = notes.map(|text| {
            let notes: Rc<dyn Notes> = Rc::new(PlainNotes::new(Some(text)));
            FieldValue::Notes(notes)
        });
        self.set_assignment_value(AssignmentField::Notes, value);
    }

    /// 取笔记的纯文本表示；无笔记时返回空串。
    ///
    /// 对应 Java: `public String getNotes()`
    pub fn get_notes(&self) -> String {
        self.notes_object()
            .map(|notes| notes.to_string())
            .unwrap_or_default()
    }

    /// 设置笔记对象（`RtfNotes` / `HtmlNotes` / `StructuredNotes` 等子类型经
    /// `Rc<dyn Notes>` 保真保存）。
    ///
    /// 对应 Java: `public void setNotesObject(Notes notes)`
    pub fn set_notes_object(&mut self, notes: Option<Box<dyn Notes>>) {
        let value = notes.map(|notes| FieldValue::Notes(Rc::from(notes)));
        self.set_assignment_value(AssignmentField::Notes, value);
    }

    /// 取笔记对象。
    ///
    /// 对应 Java: `public Notes getNotesObject()`
    pub fn get_notes_object(&self) -> Option<Rc<dyn Notes>> {
        self.notes_object()
    }

    /// 取笔记对象（内部便捷方法）。
    fn notes_object(&self) -> Option<Rc<dyn Notes>> {
        self.assignment_value(AssignmentField::Notes)
            .and_then(|value| value.as_notes().cloned())
    }

    /// 设置「已确认」标志。
    ///
    /// 对应 Java: `public void setConfirmed(boolean val)`
    pub fn set_confirmed(&mut self, val: bool) {
        self.set_boolean_value(AssignmentField::Confirmed, val);
    }

    /// 取「已确认」标志。
    ///
    /// 对应 Java: `public boolean getConfirmed()`
    pub fn get_confirmed(&self) -> bool {
        self.boolean_value(AssignmentField::Confirmed)
    }

    /// 设置「需要更新」标志。
    ///
    /// 对应 Java: `public void setUpdateNeeded(boolean val)`
    pub fn set_update_needed(&mut self, val: bool) {
        self.set_boolean_value(AssignmentField::UpdateNeeded, val);
    }

    /// 取「需要更新」标志。
    ///
    /// 对应 Java: `public boolean getUpdateNeeded()`
    pub fn get_update_needed(&self) -> bool {
        self.boolean_value(AssignmentField::UpdateNeeded)
    }

    /// 设置「链接字段」标志。
    ///
    /// 对应 Java: `public void setLinkedFields(boolean flag)`
    pub fn set_linked_fields(&mut self, flag: bool) {
        self.set_boolean_value(AssignmentField::LinkedFields, flag);
    }

    /// 取「链接字段」标志。
    ///
    /// 对应 Java: `public boolean getLinkedFields()`
    pub fn get_linked_fields(&self) -> bool {
        self.boolean_value(AssignmentField::LinkedFields)
    }

    /// 取超链接。
    ///
    /// 对应 Java: `public String getHyperlink()`
    pub fn get_hyperlink(&self) -> Option<String> {
        self.string_value(AssignmentField::Hyperlink)
    }

    /// 取超链接地址。
    ///
    /// 对应 Java: `public String getHyperlinkAddress()`
    pub fn get_hyperlink_address(&self) -> Option<String> {
        self.string_value(AssignmentField::HyperlinkAddress)
    }

    /// 取超链接子地址。
    ///
    /// 对应 Java: `public String getHyperlinkSubAddress()`
    pub fn get_hyperlink_sub_address(&self) -> Option<String> {
        self.string_value(AssignmentField::HyperlinkSubaddress)
    }

    /// 设置超链接。
    ///
    /// 对应 Java: `public void setHyperlink(String text)`
    pub fn set_hyperlink(&mut self, text: Option<String>) {
        self.set_string_value(AssignmentField::Hyperlink, text);
    }

    /// 设置超链接地址。
    ///
    /// 对应 Java: `public void setHyperlinkAddress(String text)`
    pub fn set_hyperlink_address(&mut self, text: Option<String>) {
        self.set_string_value(AssignmentField::HyperlinkAddress, text);
    }

    /// 设置超链接子地址。
    ///
    /// 对应 Java: `public void setHyperlinkSubAddress(String text)`
    pub fn set_hyperlink_sub_address(&mut self, text: Option<String>) {
        self.set_string_value(AssignmentField::HyperlinkSubaddress, text);
    }

    /// 设置工时偏差。
    ///
    /// 对应 Java: `public void setWorkVariance(Duration val)`
    pub fn set_work_variance(&mut self, val: Option<Duration>) {
        self.set_assignment_value(AssignmentField::WorkVariance, val.map(FieldValue::Duration));
    }

    /// 取工时偏差（计算字段）。
    ///
    /// 对应 Java: `public Duration getWorkVariance()`
    pub fn get_work_variance(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::WorkVariance)
    }

    /// 设置开始时间偏差。
    ///
    /// 对应 Java: `public void setStartVariance(Duration val)`
    pub fn set_start_variance(&mut self, val: Option<Duration>) {
        self.set_assignment_value(AssignmentField::StartVariance, val.map(FieldValue::Duration));
    }

    /// 取开始时间偏差（计算字段）。
    ///
    /// 对应 Java: `public Duration getStartVariance()`
    pub fn get_start_variance(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::StartVariance)
    }

    /// 设置完成时间偏差。
    ///
    /// 对应 Java: `public void setFinishVariance(Duration duration)`
    pub fn set_finish_variance(&mut self, duration: Option<Duration>) {
        self.set_assignment_value(
            AssignmentField::FinishVariance,
            duration.map(FieldValue::Duration),
        );
    }

    /// 取完成时间偏差（计算字段）。
    ///
    /// 对应 Java: `public Duration getFinishVariance()`
    pub fn get_finish_variance(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::FinishVariance)
    }

    /// 取创建时间。
    ///
    /// 对应 Java: `public LocalDateTime getCreateDate()`
    pub fn get_create_date(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::Created)
    }

    /// 设置创建时间。
    ///
    /// 对应 Java: `public void setCreateDate(LocalDateTime val)`
    pub fn set_create_date(&mut self, val: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::Created, val.map(FieldValue::DateTime));
    }

    /// 取分配的 GUID。
    ///
    /// 对应 Java: `public UUID getGUID()`
    pub fn get_guid(&self) -> Option<Uuid> {
        self.assignment_value(AssignmentField::Guid)
            .and_then(|value| value.as_uuid())
    }

    /// 设置分配的 GUID。
    ///
    /// 对应 Java: `public void setGUID(UUID value)`
    pub fn set_guid(&mut self, value: Option<Uuid>) {
        self.set_assignment_value(AssignmentField::Guid, value.map(FieldValue::Uuid));
    }

    /// 设置「响应待处理」标志。
    ///
    /// 对应 Java: `public void setResponsePending(boolean val)`
    pub fn set_response_pending(&mut self, val: bool) {
        self.set_boolean_value(AssignmentField::ResponsePending, val);
    }

    /// 取「响应待处理」标志。
    ///
    /// 对应 Java: `public boolean getResponsePending()`
    pub fn get_response_pending(&self) -> bool {
        self.boolean_value(AssignmentField::ResponsePending)
    }

    /// 设置「团队状态待处理」标志。
    ///
    /// 对应 Java: `public void setTeamStatusPending(boolean val)`
    pub fn set_team_status_pending(&mut self, val: bool) {
        self.set_boolean_value(AssignmentField::TeamStatusPending, val);
    }

    /// 取「团队状态待处理」标志。
    ///
    /// 对应 Java: `public boolean getTeamStatusPending()`
    pub fn get_team_status_pending(&self) -> bool {
        self.boolean_value(AssignmentField::TeamStatusPending)
    }

    /// 设置 VAC。
    ///
    /// 对应 Java: `public void setVAC(Number value)`
    pub fn set_vac(&mut self, value: Option<f64>) {
        self.set_assignment_value(AssignmentField::Vac, value.map(FieldValue::Double));
    }

    /// 取 VAC。
    ///
    /// 对应 Java: `public Number getVAC()`
    pub fn get_vac(&self) -> Option<f64> {
        self.number_value(AssignmentField::Vac)
    }

    /// 设置成本费率表下标（`int` 版本）。
    ///
    /// 对应 Java: `public void setCostRateTableIndex(int index)`
    pub fn set_cost_rate_table_index(&mut self, index: i32) {
        self.set_integer_value(AssignmentField::CostRateTable, index);
    }

    /// 取成本费率表下标；越界时返回 0。
    ///
    /// 对应 Java: `public int getCostRateTableIndex()`
    pub fn get_cost_rate_table_index(&self) -> i32 {
        let value = NumberHelper::get_int(self.number_value(AssignmentField::CostRateTable));
        if value < 0 || value >= CostRateTable::MAX_TABLES {
            0
        } else {
            value
        }
    }

    /// 在本分配使用的成本费率表上执行给定操作。
    ///
    /// 对应 Java: `public CostRateTable getCostRateTable()`
    ///
    /// # 与 Java 的差异
    ///
    /// Java 返回 `CostRateTable` 引用；Rust 侧资源按值持有费率表
    /// （`Resource::get_cost_rate_table` 返回借用）且 `CostRateTable` 未实现 `Clone`，
    /// 无法返回所有权，故改为回调形式（见文件头第 4 节）。表不存在时以 `None` 调用回调
    /// （对应 Java 返回 `null`）。
    pub fn with_cost_rate_table<T>(&self, f: impl FnOnce(Option<&CostRateTable>) -> T) -> T {
        // Java: 费率来源为 override 时没有费率表
        if self.get_rate_source() == RateSource::Override {
            return f(None);
        }

        // Java: 找不到资源/角色时返回 null
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

    /// 取超链接屏幕提示。
    ///
    /// 对应 Java: `public String getHyperlinkScreenTip()`
    pub fn get_hyperlink_screen_tip(&self) -> Option<String> {
        self.string_value(AssignmentField::HyperlinkScreenTip)
    }

    /// 设置超链接屏幕提示。
    ///
    /// 对应 Java: `public void setHyperlinkScreenTip(String text)`
    pub fn set_hyperlink_screen_tip(&mut self, text: Option<String>) {
        self.set_string_value(AssignmentField::HyperlinkScreenTip, text);
    }

    /// 取资源请求类型。
    ///
    /// 对应 Java: `public ResourceRequestType getResourceRequestType()`
    pub fn get_resource_request_type(&self) -> Option<ResourceRequestType> {
        self.assignment_value(AssignmentField::ResourceRequestType)
            .and_then(|value| value.as_resource_request_type())
    }

    /// 设置资源请求类型。
    ///
    /// 对应 Java: `public void setResourceRequestType(ResourceRequestType type)`
    pub fn set_resource_request_type(&mut self, r#type: Option<ResourceRequestType>) {
        self.set_assignment_value(
            AssignmentField::ResourceRequestType,
            r#type.map(FieldValue::ResourceRequestType),
        );
    }

    /// 取停止时间。
    ///
    /// 对应 Java: `public LocalDateTime getStop()`
    pub fn get_stop(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::Stop)
    }

    /// 设置停止时间。
    ///
    /// 对应 Java: `public void setStop(LocalDateTime stop)`
    pub fn set_stop(&mut self, stop: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::Stop, stop.map(FieldValue::DateTime));
    }

    /// 取恢复时间。
    ///
    /// 对应 Java: `public LocalDateTime getResume()`
    pub fn get_resume(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::Resume)
    }

    /// 设置恢复时间。
    ///
    /// 对应 Java: `public void setResume(LocalDateTime resume)`
    pub fn set_resume(&mut self, resume: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::Resume, resume.map(FieldValue::DateTime));
    }

    /// 取计划工时。
    ///
    /// 对应 Java: `public Duration getPlannedWork()`
    pub fn get_planned_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::PlannedWork)
    }

    /// 设置计划工时。
    ///
    /// 对应 Java: `public void setPlannedWork(Duration value)`
    pub fn set_planned_work(&mut self, value: Option<Duration>) {
        self.set_assignment_value(AssignmentField::PlannedWork, value.map(FieldValue::Duration));
    }

    /// 取计划成本。
    ///
    /// 对应 Java: `public Number getPlannedCost()`
    pub fn get_planned_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::PlannedCost)
    }

    /// 设置计划成本。
    ///
    /// 对应 Java: `public void setPlannedCost(Number value)`
    pub fn set_planned_cost(&mut self, value: Option<f64>) {
        self.set_assignment_value(AssignmentField::PlannedCost, value.map(FieldValue::Double));
    }

    /// 取计划开始时间。
    ///
    /// 对应 Java: `public LocalDateTime getPlannedStart()`
    pub fn get_planned_start(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::PlannedStart)
    }

    /// 设置计划开始时间。
    ///
    /// 对应 Java: `public void setPlannedStart(LocalDateTime value)`
    pub fn set_planned_start(&mut self, value: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::PlannedStart, value.map(FieldValue::DateTime));
    }

    /// 取计划完成时间。
    ///
    /// 对应 Java: `public LocalDateTime getPlannedFinish()`
    pub fn get_planned_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::PlannedFinish)
    }

    /// 设置计划完成时间。
    ///
    /// 对应 Java: `public void setPlannedFinish(LocalDateTime value)`
    pub fn set_planned_finish(&mut self, value: Option<NaiveDateTime>) {
        self.set_assignment_value(AssignmentField::PlannedFinish, value.map(FieldValue::DateTime));
    }

    /// 取「按单位计算成本」标志（默认 `true`）。
    ///
    /// 对应 Java: `public boolean getCalculateCostsFromUnits()`
    pub fn get_calculate_costs_from_units(&self) -> bool {
        self.boolean_value(AssignmentField::CalculateCostsFromUnits)
    }

    /// 设置「按单位计算成本」标志。
    ///
    /// 对应 Java: `public void setCalculateCostsFromUnits(boolean calculateCostsFromUnits)`
    pub fn set_calculate_costs_from_units(&mut self, calculate_costs_from_units: bool) {
        self.set_boolean_value(
            AssignmentField::CalculateCostsFromUnits,
            calculate_costs_from_units,
        );
    }

    /// 取成本科目的 unique ID。
    ///
    /// 对应 Java: `public Integer getCostAccountUniqueID()`
    pub fn get_cost_account_unique_id(&self) -> Option<i32> {
        self.integer_value(AssignmentField::CostAccountUniqueId)
    }

    /// 设置成本科目的 unique ID。
    ///
    /// 对应 Java: `public void setCostAccountUniqueID(Integer id)`
    pub fn set_cost_account_unique_id(&mut self, id: Option<i32>) {
        self.set_assignment_value(
            AssignmentField::CostAccountUniqueId,
            id.map(FieldValue::Integer),
        );
    }

    /// 取剩余最晚完成时间。
    ///
    /// 对应 Java: `public LocalDateTime getRemainingLateFinish()`
    pub fn get_remaining_late_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::RemainingLateFinish)
    }

    /// 设置剩余最晚完成时间。
    ///
    /// 对应 Java: `public void setRemainingLateFinish(LocalDateTime date)`
    pub fn set_remaining_late_finish(&mut self, date: Option<NaiveDateTime>) {
        self.set_assignment_value(
            AssignmentField::RemainingLateFinish,
            date.map(FieldValue::DateTime),
        );
    }

    /// 取剩余最晚开始时间。
    ///
    /// 对应 Java: `public LocalDateTime getRemainingLateStart()`
    pub fn get_remaining_late_start(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::RemainingLateStart)
    }

    /// 设置剩余最晚开始时间。
    ///
    /// 对应 Java: `public void setRemainingLateStart(LocalDateTime date)`
    pub fn set_remaining_late_start(&mut self, date: Option<NaiveDateTime>) {
        self.set_assignment_value(
            AssignmentField::RemainingLateStart,
            date.map(FieldValue::DateTime),
        );
    }

    /// 取剩余最早完成时间。
    ///
    /// 对应 Java: `public LocalDateTime getRemainingEarlyFinish()`
    pub fn get_remaining_early_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::RemainingEarlyFinish)
    }

    /// 设置剩余最早完成时间。
    ///
    /// 对应 Java: `public void setRemainingEarlyFinish(LocalDateTime date)`
    pub fn set_remaining_early_finish(&mut self, date: Option<NaiveDateTime>) {
        self.set_assignment_value(
            AssignmentField::RemainingEarlyFinish,
            date.map(FieldValue::DateTime),
        );
    }

    /// 取剩余最早开始时间。
    ///
    /// 对应 Java: `public LocalDateTime getRemainingEarlyStart()`
    pub fn get_remaining_early_start(&self) -> Option<NaiveDateTime> {
        self.date_time_value(AssignmentField::RemainingEarlyStart)
    }

    /// 设置剩余最早开始时间。
    ///
    /// 对应 Java: `public void setRemainingEarlyStart(LocalDateTime date)`
    pub fn set_remaining_early_start(&mut self, date: Option<NaiveDateTime>) {
        self.set_assignment_value(
            AssignmentField::RemainingEarlyStart,
            date.map(FieldValue::DateTime),
        );
    }

    /// 取指定日期生效的成本费率。
    ///
    /// 对应 Java: `public Rate getEffectiveRate(LocalDateTime date)`
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

    /// 取实际常规成本（计算字段）。
    ///
    /// 对应 Java: `public Number getActualRegularCost()`
    pub fn get_actual_regular_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::ActualRegularCost)
    }

    /// 取剩余常规成本（计算字段）。
    ///
    /// 对应 Java: `public Number getRemainingRegularCost()`
    pub fn get_remaining_regular_cost(&self) -> Option<f64> {
        self.number_value(AssignmentField::RemainingRegularCost)
    }

    /// 取实际常规工时（计算字段）。
    ///
    /// 对应 Java: `public Duration getActualRegularWork()`
    pub fn get_actual_regular_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::ActualRegularWork)
    }

    /// 取剩余常规工时（计算字段）。
    ///
    /// 对应 Java: `public Duration getRemainingRegularWork()`
    pub fn get_remaining_regular_work(&self) -> Option<Duration> {
        self.duration_value(AssignmentField::RemainingRegularWork)
    }

    /// 取计划材料用量（计算字段，仅材料资源有值）。
    ///
    /// 对应 Java: `public Number getPlannedMaterial()`
    pub fn get_planned_material(&self) -> Option<f64> {
        self.number_value(AssignmentField::PlannedMaterial)
    }

    /// 取实际材料用量（计算字段）。
    ///
    /// 对应 Java: `public Number getActualMaterial()`
    pub fn get_actual_material(&self) -> Option<f64> {
        self.number_value(AssignmentField::ActualMaterial)
    }

    /// 取剩余材料用量（计算字段）。
    ///
    /// 对应 Java: `public Number getRemainingMaterial()`
    pub fn get_remaining_material(&self) -> Option<f64> {
        self.number_value(AssignmentField::RemainingMaterial)
    }

    /// 取材料用量（计算字段）。
    ///
    /// 对应 Java: `public Number getMaterial()`
    pub fn get_material(&self) -> Option<f64> {
        self.number_value(AssignmentField::Material)
    }

    /// 取基准材料用量（计算字段）。
    ///
    /// 对应 Java: `public Number getBaselineMaterial()`
    pub fn get_baseline_material(&self) -> Option<f64> {
        self.number_value(AssignmentField::BaselineMaterial)
    }

    /// 取基准材料用量（下标 1-10，计算字段）。
    ///
    /// 对应 Java: `public Number getBaselineMaterial(int baselineNumber)`
    pub fn get_baseline_material_by_index(&self, baseline_number: i32) -> Option<f64> {
        let field = Self::select_field(&BASELINE_MATERIALS, baseline_number)?;
        self.number_value(field)
    }

    /// 按别名取字段值。
    ///
    /// 对应 Java: `public Object getFieldByAlias(String alias)`
    pub fn get_field_by_alias(&self, alias: &str) -> Option<FieldValue> {
        let field = self.field_type_by_alias(alias)?;
        self.get_field(field)
    }

    /// 按别名设置字段值。
    ///
    /// 对应 Java: `public void setFieldByAlias(String alias, Object value)`
    pub fn set_field_by_alias(&mut self, alias: &str, value: Option<FieldValue>) {
        if let Some(field) = self.field_type_by_alias(alias) {
            self.set_key(field, value);
        }
    }

    /// 按别名解析字段标识。
    ///
    /// 对应 Java: `m_parentFile.getResourceAssignments().getFieldTypeByAlias(alias)`
    ///
    /// Rust 侧经上下文的自定义字段容器解析（见文件头第 4 节）。
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

    /// 计算加班成本。
    ///
    /// 对应 Java: `private Number calculateOvertimeCost()`
    fn calculate_overtime_cost(&self) -> Option<f64> {
        match (
            self.get_actual_overtime_cost(),
            self.get_remaining_overtime_cost(),
        ) {
            (Some(actual), Some(remaining)) => Some(actual + remaining),
            _ => None,
        }
    }

    /// 计算进度偏差（SV = BCWP - BCWS）。
    ///
    /// 对应 Java: `private Double calculateSV()`
    fn calculate_sv(&self) -> Option<f64> {
        match (self.get_bcwp(), self.get_bcws()) {
            (Some(bcwp), Some(bcws)) => Some(bcwp - bcws),
            _ => None,
        }
    }

    /// 计算成本偏差（CV = BCWP - ACWP）。
    ///
    /// 对应 Java: `private Double calculateCV()`
    fn calculate_cv(&self) -> Option<f64> {
        Some(NumberHelper::get_double(self.get_bcwp()) - NumberHelper::get_double(self.get_acwp()))
    }

    /// 计算成本偏差（成本 - 基准成本）。
    ///
    /// 对应 Java: `private Double calculateCostVariance()`
    fn calculate_cost_variance(&self) -> Option<f64> {
        match (self.get_cost(), self.get_baseline_cost()) {
            (Some(cost), Some(baseline_cost)) => Some(cost - baseline_cost),
            _ => None,
        }
    }

    /// 计算工时完成百分比。
    ///
    /// 对应 Java: `private Double calculatePercentWorkComplete()`
    fn calculate_percent_work_complete(&self) -> Option<f64> {
        match (self.get_actual_work(), self.get_work()) {
            (Some(actual_work), Some(work)) if work.get_duration() != 0.0 => {
                let converted = self.convert_units_with_defaults(work, actual_work.get_units());
                Some((actual_work.get_duration() * 100.0) / converted.get_duration())
            }
            _ => None,
        }
    }

    /// 计算工时偏差。
    ///
    /// 对应 Java: `private Duration calculateWorkVariance()`
    ///
    /// Java 在计算末尾会 `set(AssignmentField.WORK_VARIANCE, variance)` 回写；Rust 的 `&self`
    /// 无法写回（见文件头第 5 节）。
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

    /// 计算开始时间偏差。
    ///
    /// 对应 Java: `private Duration calculateStartVariance()`
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

    /// 计算完成时间偏差。
    ///
    /// 对应 Java: `private Duration calculateFinishVariance()`
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

    /// 用项目属性提供的默认值换算工期。
    ///
    /// 对应 Java: `duration.convertUnits(units, m_parentFile.getProjectProperties())`
    ///
    /// 父文件未接线（或正被借用）时退化为有效日历（见文件头第 4 节）。
    fn convert_units_with_defaults(&self, duration: Duration, units: TimeUnit) -> Duration {
        if let Some(parent_file) = self.parent_file.upgrade() {
            if let Ok(parent_file) = parent_file.try_borrow() {
                return duration.convert_units(units, parent_file.get_project_properties());
            }
        }

        convert_units(duration, units, self.get_effective_calendar().as_ref())
    }

    /// 取项目默认工期单位。
    ///
    /// 对应 Java: `m_parentFile.getProjectProperties().getDefaultDurationUnits()`
    fn default_duration_units(&self) -> TimeUnit {
        if let Some(parent_file) = self.parent_file.upgrade() {
            if let Ok(parent_file) = parent_file.try_borrow() {
                return parent_file.get_project_properties().get_default_duration_units();
            }
        }

        DEFAULT_DURATION_UNITS
    }

    /// 计算开始时间（未显式设置时取任务开始时间）。
    ///
    /// 对应 Java: `private LocalDateTime calculateStart()`
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

    /// 计算完成时间（未显式设置时取任务完成时间）。
    ///
    /// 对应 Java: `private LocalDateTime calculateFinish()`
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

    /// 计算剩余分配单位（无剩余单位值时退化为计划单位）。
    ///
    /// 对应 Java: `private Number calculateRemainingAssignmentUnits()`
    fn calculate_remaining_assignment_units(&self) -> Option<f64> {
        self.get_units()
    }

    /// 计算 timephased 加班工时（按剩余常规工时等比缩放）。
    ///
    /// 对应 Java: `private List<TimephasedWork> calculateTimephasedOvertimeWork()`
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

    /// 计算「实际常规成本」。
    ///
    /// 对应 Java: `CALCULATED_FIELD_MAP` 中的
    /// `a -> a.calculateRegularCost(a::getActualCost, a::getActualOvertimeCost)`
    fn calculate_actual_regular_cost(&self) -> Option<f64> {
        self.calculate_regular_cost(self.get_actual_cost(), self.get_actual_overtime_cost())
    }

    /// 计算「剩余常规成本」。
    ///
    /// 对应 Java: `CALCULATED_FIELD_MAP` 中的
    /// `a -> a.calculateRegularCost(a::getRemainingCost, a::getRemainingOvertimeCost)`
    fn calculate_remaining_regular_cost(&self) -> Option<f64> {
        self.calculate_regular_cost(self.get_remaining_cost(), self.get_remaining_overtime_cost())
    }

    /// 计算「实际常规工时」。
    ///
    /// 对应 Java: `CALCULATED_FIELD_MAP` 中的
    /// `a -> a.calculateRegularWork(a::getActualWork, a::getActualOvertimeWork)`
    fn calculate_actual_regular_work(&self) -> Option<Duration> {
        self.calculate_regular_work(self.get_actual_work(), self.get_actual_overtime_work())
    }

    /// 计算「剩余常规工时」。
    ///
    /// 对应 Java: `CALCULATED_FIELD_MAP` 中的
    /// `a -> a.calculateRegularWork(a::getRemainingWork, a::getRemainingOvertimeWork)`
    fn calculate_remaining_regular_work(&self) -> Option<Duration> {
        self.calculate_regular_work(self.get_remaining_work(), self.get_remaining_overtime_work())
    }

    /// 由总成本与加班成本计算常规成本。
    ///
    /// 对应 Java: `private Number calculateRegularCost(Supplier<Number>, Supplier<Number>)`
    fn calculate_regular_cost(&self, total_cost: Option<f64>, overtime_cost: Option<f64>) -> Option<f64> {
        if total_cost.is_none() && overtime_cost.is_none() {
            return None;
        }

        Some(NumberHelper::get_double(total_cost) - NumberHelper::get_double(overtime_cost))
    }

    /// 由总工时与加班工时计算常规工时。
    ///
    /// 对应 Java: `private Duration calculateRegularWork(Supplier<Duration>, Supplier<Duration>)`
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

    /// 由工时计算材料用量（仅材料资源有值）。
    ///
    /// 对应 Java: `private Number calculateMaterial(Supplier<Duration> fn)`
    fn calculate_material(&self, work: Option<Duration>) -> Option<f64> {
        if !self.is_material_resource() {
            return None;
        }

        work.map(|work| work.get_duration())
    }

    /// 费率表下标的默认值（0）。
    ///
    /// 对应 Java: `private Integer defaultRateIndex()`
    fn default_rate_index(&self) -> Option<i32> {
        Some(0)
    }

    /// 「按单位计算成本」的默认值（`true`）。
    ///
    /// 对应 Java: `private Boolean defaultCalculateCostsFromUnits()`
    fn default_calculate_costs_from_units(&self) -> Option<bool> {
        Some(true)
    }

    /// 本分配的资源是否为材料资源。
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

    /// 取本分配所用资源的计提方式；缺失时退化为 `AccrueType::Prorated`。
    ///
    /// 对应 Java: `private AccrueType getResourceAccrueType()`
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

    // ---- timephased 原始数据访问器 ----

    /// 取原始 timephased 计划工时。
    ///
    /// 对应 Java: `public List<TimephasedWork> getRawTimephasedPlannedWork()`
    ///
    /// 未设置时与 Java 一致返回空列表（`CALCULATED_FIELD_MAP` 的
    /// `defaultTimephasedWork()`）。
    pub fn get_raw_timephased_planned_work(&self) -> Vec<TimephasedWork> {
        self.timephased_work_value(AssignmentField::RawTimephasedPlannedWork)
    }

    /// 设置原始 timephased 计划工时（读者用它写回，语义等价 Java 的
    /// `getRawTimephasedPlannedWork().addAll(...)`）。
    pub fn set_raw_timephased_planned_work(&mut self, work: Vec<TimephasedWork>) {
        self.set_timephased_work_value(AssignmentField::RawTimephasedPlannedWork, work);
    }

    /// 取原始 timephased 实际常规工时。
    ///
    /// 对应 Java: `public List<TimephasedWork> getRawTimephasedActualRegularWork()`
    pub fn get_raw_timephased_actual_regular_work(&self) -> Vec<TimephasedWork> {
        self.timephased_work_value(AssignmentField::RawTimephasedActualRegularWork)
    }

    /// 设置原始 timephased 实际常规工时。
    pub fn set_raw_timephased_actual_regular_work(&mut self, work: Vec<TimephasedWork>) {
        self.set_timephased_work_value(AssignmentField::RawTimephasedActualRegularWork, work);
    }

    /// 取原始 timephased 剩余常规工时。
    ///
    /// 对应 Java: `public List<TimephasedWork> getRawTimephasedRemainingRegularWork()`
    pub fn get_raw_timephased_remaining_regular_work(&self) -> Vec<TimephasedWork> {
        self.timephased_work_value(AssignmentField::RawTimephasedRemainingRegularWork)
    }

    /// 设置原始 timephased 剩余常规工时。
    pub fn set_raw_timephased_remaining_regular_work(&mut self, work: Vec<TimephasedWork>) {
        self.set_timephased_work_value(AssignmentField::RawTimephasedRemainingRegularWork, work);
    }

    /// 取原始 timephased 实际加班工时。
    ///
    /// 对应 Java: `public List<TimephasedWork> getRawTimephasedActualOvertimeWork()`
    pub fn get_raw_timephased_actual_overtime_work(&self) -> Vec<TimephasedWork> {
        self.timephased_work_value(AssignmentField::RawTimephasedActualOvertimeWork)
    }

    /// 设置原始 timephased 实际加班工时。
    pub fn set_raw_timephased_actual_overtime_work(&mut self, work: Vec<TimephasedWork>) {
        self.set_timephased_work_value(AssignmentField::RawTimephasedActualOvertimeWork, work);
    }

    /// 取原始 timephased 剩余加班工时；无数据时由计算字段
    /// `calculateTimephasedOvertimeWork` 提供。
    ///
    /// 对应 Java: `public List<TimephasedWork> getRawTimephasedRemainingOvertimeWork()`
    pub fn get_raw_timephased_remaining_overtime_work(&self) -> Vec<TimephasedWork> {
        self.timephased_work_value(AssignmentField::RawTimephasedRemainingOvertimeWork)
    }

    /// 设置原始 timephased 剩余加班工时。
    pub fn set_raw_timephased_remaining_overtime_work(&mut self, work: Vec<TimephasedWork>) {
        self.set_timephased_work_value(AssignmentField::RawTimephasedRemainingOvertimeWork, work);
    }

    /// 取原始 timephased 基准工时（下标 0 = Baseline，1..=10 = Baseline1..10）。
    ///
    /// 对应 Java: `public List<TimephasedWork> getRawTimephasedBaselineWork(int index)`
    ///
    /// 越界时返回空列表（Java 会抛 `ArrayIndexOutOfBoundsException`）。
    pub fn get_raw_timephased_baseline_work(&self, index: i32) -> Vec<TimephasedWork> {
        raw_timephased_baseline_work_field(index)
            .map(|field| self.timephased_work_value(field))
            .unwrap_or_default()
    }

    /// 设置原始 timephased 基准工时。
    pub fn set_raw_timephased_baseline_work(&mut self, index: i32, work: Vec<TimephasedWork>) {
        if let Some(field) = raw_timephased_baseline_work_field(index) {
            self.set_timephased_work_value(field, work);
        }
    }

    /// 取原始 timephased 基准预算工时（下标同上）。
    ///
    /// 对应 Java: `public List<TimephasedWork> getRawTimephasedBaselineBudgetWork(int index)`
    pub fn get_raw_timephased_baseline_budget_work(&self, index: i32) -> Vec<TimephasedWork> {
        raw_timephased_baseline_budget_work_field(index)
            .map(|field| self.timephased_work_value(field))
            .unwrap_or_default()
    }

    /// 设置原始 timephased 基准预算工时。
    pub fn set_raw_timephased_baseline_budget_work(&mut self, index: i32, work: Vec<TimephasedWork>) {
        if let Some(field) = raw_timephased_baseline_budget_work_field(index) {
            self.set_timephased_work_value(field, work);
        }
    }

    /// 取原始 timephased 基准成本（下标同上）。
    ///
    /// 对应 Java: `public List<TimephasedCost> getRawTimephasedBaselineCost(int index)`
    pub fn get_raw_timephased_baseline_cost(&self, index: i32) -> Vec<TimephasedCost> {
        raw_timephased_baseline_cost_field(index)
            .map(|field| self.timephased_cost_value(field))
            .unwrap_or_default()
    }

    /// 设置原始 timephased 基准成本。
    pub fn set_raw_timephased_baseline_cost(&mut self, index: i32, cost: Vec<TimephasedCost>) {
        if let Some(field) = raw_timephased_baseline_cost_field(index) {
            self.set_timephased_cost_value(field, cost);
        }
    }

    /// 取原始 timephased 基准预算成本（下标同上）。
    ///
    /// 对应 Java: `public List<TimephasedCost> getRawTimephasedBaselineBudgetCost(int index)`
    pub fn get_raw_timephased_baseline_budget_cost(&self, index: i32) -> Vec<TimephasedCost> {
        raw_timephased_baseline_budget_cost_field(index)
            .map(|field| self.timephased_cost_value(field))
            .unwrap_or_default()
    }

    /// 设置原始 timephased 基准预算成本。
    pub fn set_raw_timephased_baseline_budget_cost(
        &mut self,
        index: i32,
        cost: Vec<TimephasedCost>,
    ) {
        if let Some(field) = raw_timephased_baseline_budget_cost_field(index) {
            self.set_timephased_cost_value(field, cost);
        }
    }

    /// 取原始 timephased 预算成本。
    ///
    /// 对应 Java: `public List<TimephasedCost> getRawTimephasedBudgetCost()`
    pub fn get_raw_timephased_budget_cost(&self) -> Vec<TimephasedCost> {
        self.timephased_cost_value(AssignmentField::RawTimephasedBudgetCost)
    }

    /// 设置原始 timephased 预算成本。
    pub fn set_raw_timephased_budget_cost(&mut self, cost: Vec<TimephasedCost>) {
        self.set_timephased_cost_value(AssignmentField::RawTimephasedBudgetCost, cost);
    }

    /// 取原始 timephased 预算工时。
    ///
    /// 对应 Java: `public List<TimephasedWork> getRawTimephasedBudgetWork()`
    pub fn get_raw_timephased_budget_work(&self) -> Vec<TimephasedWork> {
        self.timephased_work_value(AssignmentField::RawTimephasedBudgetWork)
    }

    /// 设置原始 timephased 预算工时。
    pub fn set_raw_timephased_budget_work(&mut self, work: Vec<TimephasedWork>) {
        self.set_timephased_work_value(AssignmentField::RawTimephasedBudgetWork, work);
    }

    /// 取字段表中的 timephased 工时列表（未设置时为空列表）。
    fn timephased_work_value(&self, field: AssignmentField) -> Vec<TimephasedWork> {
        self.assignment_value(field)
            .and_then(|value| value.as_timephased_work().map(<[TimephasedWork]>::to_vec))
            .unwrap_or_default()
    }

    /// 把 timephased 工时列表写入字段表。
    fn set_timephased_work_value(&mut self, field: AssignmentField, work: Vec<TimephasedWork>) {
        self.set_assignment_value(field, Some(FieldValue::TimephasedWork(work)));
    }

    /// 取字段表中的 timephased 成本列表（未设置时为空列表）。
    fn timephased_cost_value(&self, field: AssignmentField) -> Vec<TimephasedCost> {
        self.assignment_value(field)
            .and_then(|value| value.as_timephased_cost().map(<[TimephasedCost]>::to_vec))
            .unwrap_or_default()
    }

    /// 把 timephased 成本列表写入字段表。
    fn set_timephased_cost_value(&mut self, field: AssignmentField, cost: Vec<TimephasedCost>) {
        self.set_assignment_value(field, Some(FieldValue::TimephasedCost(cost)));
    }

    /// 取本分配区间内的工作时间段。
    ///
    /// 对应 Java: `public List<LocalDateTimeRange> getWorkSplits()`
    pub fn get_work_splits(&self) -> Vec<LocalDateTimeRange> {
        let actual_work = self.get_raw_timephased_actual_regular_work();
        let remaining_work = self.get_raw_timephased_remaining_regular_work();
        if actual_work.is_empty() && remaining_work.is_empty() {
            return Vec::new();
        }

        let calendar = self.get_effective_calendar();
        let mut result: Vec<LocalDateTimeRange> = Vec::new();
        let mut last_range: Option<LocalDateTimeRange> = None;

        // Java 对两批数据用同一套合并逻辑
        for work in actual_work.iter().chain(remaining_work.iter()) {
            // Java: 表示非工作时间的块直接跳过，并打断合并
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

    /// 取指定字段的 timephased 工期值。
    ///
    /// 对应 Java: `@Override public List<Duration> getTimephasedDurationValues(FieldType field, List<LocalDateTimeRange> ranges, TimeUnit units)`
    ///
    /// Rust 的 [`FieldContainer`] trait 未声明这两个 timephased 方法（见
    /// `field_container.rs` 的说明），故以普通方法提供。
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

    /// 取指定字段的 timephased 数值。
    ///
    /// 对应 Java: `@Override public List<Number> getTimephasedNumericValues(FieldType field, List<LocalDateTimeRange> ranges)`
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

    /// 取 timephased 计划工时。
    ///
    /// 对应 Java: `public List<Duration> getTimephasedPlannedWork(List<LocalDateTimeRange> ranges, TimeUnit units)`
    pub fn get_timephased_planned_work(
        &self,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        let work = self.get_or_create_raw_timephased_planned_work();
        segment_work(self.get_effective_calendar().as_ref(), &work, ranges, units)
    }

    /// 取 timephased 实际常规工时。
    ///
    /// 对应 Java: `public List<Duration> getTimephasedActualRegularWork(List<LocalDateTimeRange> ranges, TimeUnit units)`
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

    /// 取 timephased 实际加班工时。
    ///
    /// 对应 Java: `public List<Duration> getTimephasedActualOvertimeWork(List<LocalDateTimeRange> ranges, TimeUnit units)`
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

    /// 取 timephased 实际工时（常规 + 加班）。
    ///
    /// 对应 Java: `public List<Duration> getTimephasedActualWork(List<LocalDateTimeRange> ranges, TimeUnit units)`
    pub fn get_timephased_actual_work(
        &self,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        let regular = self.get_timephased_actual_regular_work(ranges, units);
        let overtime = self.get_timephased_actual_overtime_work(ranges, units);
        add_timephased_durations(regular, overtime)
    }

    /// 取 timephased 剩余常规工时。
    ///
    /// 对应 Java: `public List<Duration> getTimephasedRemainingRegularWork(List<LocalDateTimeRange> ranges, TimeUnit units)`
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

    /// 取 timephased 剩余加班工时。
    ///
    /// 对应 Java: `public List<Duration> getTimephasedRemainingOvertimeWork(List<LocalDateTimeRange> ranges, TimeUnit units)`
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

    /// 取 timephased 剩余工时（常规 + 加班）。
    ///
    /// 对应 Java: `public List<Duration> getTimephasedRemainingWork(List<LocalDateTimeRange> ranges, TimeUnit units)`
    pub fn get_timephased_remaining_work(
        &self,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        let regular = self.get_timephased_remaining_regular_work(ranges, units);
        let overtime = self.get_timephased_remaining_overtime_work(ranges, units);
        add_timephased_durations(regular, overtime)
    }

    /// 取 timephased 工时（实际 + 剩余）。
    ///
    /// 对应 Java: `public List<Duration> getTimephasedWork(List<LocalDateTimeRange> ranges, TimeUnit units)`
    pub fn get_timephased_work(
        &self,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        let actual = self.get_timephased_actual_work(ranges, units);
        let remaining = self.get_timephased_remaining_work(ranges, units);
        add_timephased_durations(actual, remaining)
    }

    /// 取 timephased 基准工时（下标 0 = Baseline，1..=10 = Baseline1..10）。
    ///
    /// 对应 Java: `public List<Duration> getTimephasedBaselineWork(int index, List<LocalDateTimeRange> ranges, TimeUnit units)`
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

    /// 取 timephased 基准预算工时。
    ///
    /// 对应 Java: `public List<Duration> getTimephasedBaselineBudgetWork(int index, List<LocalDateTimeRange> ranges, TimeUnit units)`
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

    /// 取 timephased 预算工时。
    ///
    /// 对应 Java: `public List<Duration> getTimephasedBudgetWork(List<LocalDateTimeRange> ranges, TimeUnit units)`
    pub fn get_timephased_budget_work(
        &self,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        let work = self.get_raw_timephased_budget_work().to_vec();
        segment_work(self.get_effective_calendar().as_ref(), &work, ranges, units)
    }

    /// 取父项目的基准日历。
    ///
    /// 对应 Java: `m_parentFile.getBaselineCalendar()`
    fn get_baseline_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        self.parent_file
            .upgrade()
            .and_then(|parent_file| parent_file.try_borrow().ok()?.get_baseline_calendar())
    }

    /// 取 timephased 计划材料用量。
    ///
    /// 对应 Java: `public List<Number> getTimephasedPlannedMaterial(List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_planned_material(&self, ranges: &[LocalDateTimeRange]) -> Vec<Option<f64>> {
        let calendar = self.get_effective_calendar();
        let work = self.get_or_create_raw_timephased_planned_work();
        segment_material(self.is_material_resource(), calendar.as_ref(), &work, ranges)
    }

    /// 取 timephased 实际材料用量。
    ///
    /// 对应 Java: `public List<Number> getTimephasedActualMaterial(List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_actual_material(&self, ranges: &[LocalDateTimeRange]) -> Vec<Option<f64>> {
        let calendar = self.get_effective_calendar();
        let work = self.get_raw_timephased_actual_regular_work().to_vec();
        segment_material(self.is_material_resource(), calendar.as_ref(), &work, ranges)
    }

    /// 取 timephased 剩余材料用量。
    ///
    /// 对应 Java: `public List<Number> getTimephasedRemainingMaterial(List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_remaining_material(
        &self,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        let calendar = self.get_effective_calendar();
        let work = self.get_raw_timephased_remaining_regular_work().to_vec();
        segment_material(self.is_material_resource(), calendar.as_ref(), &work, ranges)
    }

    /// 取 timephased 材料用量（实际 + 剩余）。
    ///
    /// 对应 Java: `public List<Number> getTimephasedMaterial(List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_material(&self, ranges: &[LocalDateTimeRange]) -> Vec<Option<f64>> {
        let actual = self.get_timephased_actual_material(ranges);
        let remaining = self.get_timephased_remaining_material(ranges);
        add_timephased_numbers(actual, remaining)
    }

    /// 取 timephased 基准材料用量。
    ///
    /// 对应 Java: `public List<Number> getTimephasedBaselineMaterial(int index, List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_baseline_material(
        &self,
        index: i32,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        let calendar = self.get_baseline_calendar();
        let work = self.get_raw_timephased_baseline_work(index).to_vec();
        segment_material(self.is_material_resource(), calendar.as_ref(), &work, ranges)
    }

    /// 取 timephased 预算成本。
    ///
    /// 对应 Java: `public List<Number> getTimephasedBudgetCost(List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_budget_cost(&self, ranges: &[LocalDateTimeRange]) -> Vec<Option<f64>> {
        let calendar = self.get_effective_calendar();
        let cost = self.get_raw_timephased_budget_cost().to_vec();
        segment_cost(calendar.as_ref(), &cost, ranges)
    }

    // ---- timephased 成本 ----

    /// 取 timephased 剩余常规成本。
    ///
    /// 对应 Java: `public List<Number> getTimephasedRemainingRegularCost(List<LocalDateTimeRange> ranges)`
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

    /// 取 timephased 剩余加班成本。
    ///
    /// 对应 Java: `public List<Number> getTimephasedRemainingOvertimeCost(List<LocalDateTimeRange> ranges)`
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
            // Java: 成本资源没有加班
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

    /// 取 timephased 剩余成本（常规 + 加班）。
    ///
    /// 对应 Java: `public List<Number> getTimephasedRemainingCost(List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_remaining_cost(
        &self,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        let regular = self.get_timephased_remaining_regular_cost(ranges);
        let overtime = self.get_timephased_remaining_overtime_cost(ranges);
        add_timephased_numbers(regular, overtime)
    }

    /// 取 timephased 实际常规成本。
    ///
    /// 对应 Java: `public List<Number> getTimephasedActualRegularCost(List<LocalDateTimeRange> ranges)`
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

    /// 取 timephased 实际加班成本。
    ///
    /// 对应 Java: `public List<Number> getTimephasedActualOvertimeCost(List<LocalDateTimeRange> ranges)`
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
            // Java: 成本资源没有加班
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

    /// 取 timephased 计划成本。
    ///
    /// 对应 Java: `public List<Number> getTimephasedPlannedCost(List<LocalDateTimeRange> ranges)`
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

    /// 取 timephased 实际成本（常规 + 加班）。
    ///
    /// 对应 Java: `public List<Number> getTimephasedActualCost(List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_actual_cost(&self, ranges: &[LocalDateTimeRange]) -> Vec<Option<f64>> {
        let regular = self.get_timephased_actual_regular_cost(ranges);
        let overtime = self.get_timephased_actual_overtime_cost(ranges);
        add_timephased_numbers(regular, overtime)
    }

    /// 取 timephased 成本（实际 + 剩余）。
    ///
    /// 对应 Java: `public List<Number> getTimephasedCost(List<LocalDateTimeRange> ranges)`
    ///
    /// Rust 无方法重载，故与同名的私有方法 [`ResourceAssignment::get_timephased_cost`] 以
    /// `_total` 后缀区分（对应 Java 中 `getTimephasedCost(ranges)` 的重载）。
    pub fn get_timephased_cost_total(&self, ranges: &[LocalDateTimeRange]) -> Vec<Option<f64>> {
        let actual = self.get_timephased_actual_cost(ranges);
        let remaining = self.get_timephased_remaining_cost(ranges);
        add_timephased_numbers(actual, remaining)
    }

    /// 取 timephased 基准成本。
    ///
    /// 对应 Java: `public List<Number> getTimephasedBaselineCost(int index, List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_baseline_cost(
        &self,
        index: i32,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        let calendar = self.get_baseline_calendar();
        let cost = self.get_raw_timephased_baseline_cost(index).to_vec();
        segment_cost(calendar.as_ref(), &cost, ranges)
    }

    /// 取 timephased 基准预算成本。
    ///
    /// 对应 Java: `public List<Number> getTimephasedBaselineBudgetCost(int index, List<LocalDateTimeRange> ranges)`
    pub fn get_timephased_baseline_budget_cost(
        &self,
        index: i32,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        let calendar = self.get_baseline_calendar();
        let cost = self.get_raw_timephased_baseline_budget_cost(index).to_vec();
        segment_cost(calendar.as_ref(), &cost, ranges)
    }

    /// 资源类型（无资源时按 Java 的默认值 `ResourceType.WORK`）。
    fn resource_type(&self) -> ResourceType {
        self.get_resource()
            .and_then(|resource| resource.try_borrow().ok().map(|resource| resource.get_type()))
            .unwrap_or(ResourceType::Work)
    }

    /// 用给定的工时提供函数计算 timephased 成本。
    ///
    /// 对应 Java: `private List<Number> getTimephasedCost(List<LocalDateTimeRange> ranges, int rateIndex, BiFunction<...> workSupplier)`
    fn get_timephased_cost(
        &self,
        ranges: &[LocalDateTimeRange],
        rate_index: usize,
        work_supplier: &dyn Fn(&[LocalDateTimeRange], TimeUnit) -> Vec<Option<Duration>>,
    ) -> Vec<Option<f64>> {
        // Java: 没有工时则返回空值
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

            // Java: 假定费率表行按顺序排列
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

                // Java: 费率表行在区间结束前就用完 —— 直接返回已有结果
                if cost_rate_table_entry_index >= rates.len() {
                    break;
                }

                // Java: 区间未越过当前费率行的结束时间，可一次算完
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

                // 一个区间内跨多个费率行
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

    /// 把费率换算为「每小时」费率。
    ///
    /// 对应 Java: `private Rate getRatePerHour(Rate rate)`
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

        Rate::value_of(Some(rate_to_hours(Some(rate), calendar)), TimeUnit::Hours)
            .unwrap_or(Rate::ZERO)
    }

    /// 取成本资源的 timephased 剩余成本。
    ///
    /// 对应 Java: `private List<Number> getTimephasedCostResourceRemainingCost(List<LocalDateTimeRange> ranges)`
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

    /// 取成本资源的 timephased 实际成本。
    ///
    /// 对应 Java: `private List<Number> getTimephasedCostResourceActualCost(List<LocalDateTimeRange> ranges)`
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

    /// 取「在分配开始时计提」的 timephased 剩余成本。
    ///
    /// 对应 Java: `private List<Number> getTimephasedRemainingCostAccruedAtStart(List<LocalDateTimeRange> ranges, Supplier<Number> totalCost, Supplier<Number> remainingCost)`
    ///
    /// Java 在找不到与分配相交的区间时会抛越界异常；Rust 侧退化为返回空值列表。
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
            // Java: 分配已开始，实际成本已覆盖全部成本，剩余成本全为 0
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

        // Java: 分配尚未开始，成本计入分配的第一个区间
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

    /// 取「在分配结束时计提」的 timephased 剩余成本。
    ///
    /// 对应 Java: `private List<Number> getTimephasedRemainingCostAccruedAtEnd(List<LocalDateTimeRange> ranges, Supplier<Number> totalCost, Supplier<Number> remainingCost)`
    fn get_timephased_remaining_cost_accrued_at_end(
        &self,
        ranges: &[LocalDateTimeRange],
        total_cost: Option<f64>,
        remaining_cost: Option<f64>,
    ) -> Vec<Option<f64>> {
        let assignment_range = LocalDateTimeRange::new(self.get_start(), self.get_finish());
        let mut result = empty_numbers(ranges.len());

        let mut range_index = first_intersecting_range(ranges, &assignment_range);

        // Java: 覆盖分配区间的区间先全部填 0
        let remaining_work = self.get_timephased_remaining_work(ranges, TimeUnit::Hours);
        while range_index < ranges.len() && ranges[range_index].intersects_with(&assignment_range) {
            result[range_index] = match remaining_work.get(range_index).copied().flatten() {
                Some(_) => Some(0.0),
                None => None,
            };
            range_index += 1;
        }

        if NumberHelper::get_double(remaining_cost) == 0.0 {
            // Java: 分配已结束，实际成本已覆盖全部成本
            return result;
        }

        if range_index == 0 || range_index > ranges.len() {
            return result;
        }

        // Java: 最后一个相交区间包含分配结束时间，成本计入该区间
        if range_compare_to_date(&ranges[range_index - 1], self.get_finish())
            .map(|compare| compare == 0)
            .unwrap_or(false)
        {
            result[range_index - 1] = total_cost;
        }

        result
    }

    /// 取按时长比例分摊的 timephased 剩余成本。
    ///
    /// 对应 Java: `private List<Number> getTimephasedRemainingCostProRata(List<LocalDateTimeRange> ranges)`
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

    /// 取「在分配开始时计提」的 timephased 成本。
    ///
    /// 对应 Java: `private List<Number> getTimephasedCostAccruedAtStart(List<LocalDateTimeRange> ranges, Supplier<Number> costSupplier, BiFunction<...> workSupplier)`
    fn get_timephased_cost_accrued_at_start(
        &self,
        ranges: &[LocalDateTimeRange],
        cost: Option<f64>,
        work_supplier: &dyn Fn(&[LocalDateTimeRange], TimeUnit) -> Vec<Option<Duration>>,
    ) -> Vec<Option<f64>> {
        let assignment_range = LocalDateTimeRange::new(self.get_start(), self.get_finish());
        let mut result = empty_numbers(ranges.len());

        let mut range_index = first_intersecting_range(ranges, &assignment_range);

        // Java: 第一个相交区间包含分配开始时间，成本计入该区间
        if range_index < ranges.len()
            && range_compare_to_date(&ranges[range_index], self.get_start())
                .map(|compare| compare == 0)
                .unwrap_or(false)
        {
            result[range_index] = Some(cost.unwrap_or(0.0));
            range_index += 1;
        }

        // Java: 其余相交区间成本为 0
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

    /// 取「在分配结束时计提」的 timephased 成本。
    ///
    /// 对应 Java: `private List<Number> getTimephasedCostAccruedAtEnd(List<LocalDateTimeRange> ranges, Supplier<Number> costSupplier, BiFunction<...> workSupplier)`
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

        // Java: 最后一个区间包含分配结束时间，成本计入该区间
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

    /// 取按时长比例分摊的 timephased 实际成本。
    ///
    /// 对应 Java: `private List<Number> getTimephasedActualCostProrata(List<LocalDateTimeRange> ranges)`
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

        // Java: 找到与调用方区间对齐、且覆盖整个分配的起点
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

        // 生成覆盖整个分配的区间
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

        // 统计包含工作时间的区间数
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

    /// 为 timephased 成本追加「剩余每次使用成本」。
    ///
    /// 对应 Java: `private List<Number> addTimephasedRemainingCostPerUse(List<LocalDateTimeRange> ranges, List<Number> costs)`
    fn add_timephased_remaining_cost_per_use(
        &self,
        ranges: &[LocalDateTimeRange],
        mut costs: Vec<Option<f64>>,
    ) -> Vec<Option<f64>> {
        // Java: 已开始则每次使用成本已计入实际成本
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

        // Java: 非材料资源且单位不为 100% 时按单位折算
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

    /// 为 timephased 成本追加「实际每次使用成本」。
    ///
    /// 对应 Java: `private List<Number> addTimephasedActualCostPerUse(List<LocalDateTimeRange> ranges, List<Number> costs)`
    fn add_timephased_actual_cost_per_use(
        &self,
        ranges: &[LocalDateTimeRange],
        mut costs: Vec<Option<f64>>,
    ) -> Vec<Option<f64>> {
        // Java: 未开始则每次使用成本计入剩余成本
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

    /// 取原始 timephased 计划工时，缺失时按计划工时与计划起止时间生成。
    ///
    /// 对应 Java: `private List<TimephasedWork> getOrCreateRawTimephasedPlannedWork()`
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

    /// 取原始 timephased 实际工时，缺失时按实际工时与实际起止时间生成。
    ///
    /// 对应 Java: `private List<TimephasedWork> getOrCreateRawTimephasedActualWork(List<TimephasedWork> timephasedWork, Supplier<Duration> workSupplier)`
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
                // Java: XER 文件中可能出现单位为 0 的情况，直接放弃生成
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

    /// 取原始 timephased 剩余工时，缺失时按剩余工时与剩余起止时间生成。
    ///
    /// 对应 Java: `private List<TimephasedWork> getOrCreateRawTimephasedRemainingWork(List<TimephasedWork> timephasedWork, Supplier<Duration> workSupplier)`
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

        // Java: 已开始且已产生实际工时时，从「剩余最早开始时间」继续；
        // 没有剩余最早开始时间则由实际开始时间与实际工时推算下一个工作开始时刻。
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
    /// 对应 Java: `public void set(FieldType field, Object value)`
    fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
        self.set_key(field, value);
    }

    /// 对应 Java: `public Object get(FieldType field)`
    fn get(&self, field: FieldKey) -> Option<FieldValue> {
        self.get_field(field)
    }

    /// 对应 Java: `public Object getCachedValue(FieldType field)`
    fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
        self.fields.get_cached_value(field)
    }

    /// 对应 Java: `@Override public List<Duration> getTimephasedDurationValues(FieldType field, List<LocalDateTimeRange> ranges, TimeUnit units)`
    ///
    /// Java 经 `TIMEPHASED_WORK_FUNCTIONS` 分派；Rust 侧由本类同名普通方法承担（见文件头第 5 节）。
    fn get_timephased_duration_values(
        &self,
        field: FieldKey,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        ResourceAssignment::get_timephased_duration_values(self, field, ranges, units)
    }

    /// 对应 Java: `@Override public List<Number> getTimephasedNumericValues(FieldType field, List<LocalDateTimeRange> ranges)`
    ///
    /// Java 经 `TIMEPHASED_NUMERIC_FUNCTIONS` 分派；Rust 侧由本类同名普通方法承担。
    fn get_timephased_numeric_values(
        &self,
        field: FieldKey,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        ResourceAssignment::get_timephased_numeric_values(self, field, ranges)
    }

    /// 对应 Java: `public void addFieldListener(FieldListener listener)`
    fn add_field_listener(&mut self, listener: Box<dyn FieldListener>) {
        self.fields.add_field_listener(listener);
    }

    /// 对应 Java: `public void removeFieldListener(FieldListener listener)`
    fn clear_field_listeners(&mut self) {
        self.fields.clear_field_listeners();
    }
}

impl ProjectEntityWithUniqueID for ResourceAssignment {
    /// 对应 Java: `@Override public Integer getUniqueID()`
    fn get_unique_id(&self) -> Option<i32> {
        ResourceAssignment::get_unique_id(self)
    }
}

impl ProjectEntityWithMutableUniqueID for ResourceAssignment {
    /// 对应 Java: `@Override public void setUniqueID(Integer uniqueID)`
    fn set_unique_id(&mut self, id: Option<i32>) {
        ResourceAssignment::set_unique_id(self, id);
    }
}

impl Display for ResourceAssignment {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 用 `null` 渲染空值；Rust 侧保持同样的文本（`NaiveDateTime` 的分隔符为空格，
    /// Java 为 `T`）。
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

/// 计算时间偏差时的默认工期单位（父文件未接线时的退化值）。
///
/// 对应 Java 的 `ProjectProperties.getDefaultDurationUnits()`（MSP 默认值为天）。
const DEFAULT_DURATION_UNITS: TimeUnit = TimeUnit::Days;

/// 基准成本字段（下标 1-10）。
///
/// 对应 Java: `AssignmentFieldLists.BASELINE_COSTS`
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

/// 基准工时字段（下标 1-10）。
///
/// 对应 Java: `AssignmentFieldLists.BASELINE_WORKS`
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

/// 基准材料用量字段（下标 1-10）。
///
/// 对应 Java: `AssignmentFieldLists.BASELINE_MATERIALS`
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

/// 基准开始时间字段（下标 1-10）。
///
/// 对应 Java: `AssignmentFieldLists.BASELINE_STARTS`
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

/// 基准完成时间字段（下标 1-10）。
///
/// 对应 Java: `AssignmentFieldLists.BASELINE_FINISHES`
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

/// 基准预算成本字段（下标 1-10）。
///
/// 对应 Java: `AssignmentFieldLists.BASELINE_BUDGET_COSTS`
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

/// 基准预算工时字段（下标 1-10）。
///
/// 对应 Java: `AssignmentFieldLists.BASELINE_BUDGET_WORKS`
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

/// 自定义文本字段（下标 1-30）。
///
/// 对应 Java: `AssignmentFieldLists.CUSTOM_TEXT`
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

/// 自定义开始时间字段（下标 1-10）。
///
/// 对应 Java: `AssignmentFieldLists.CUSTOM_START`
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

/// 自定义完成时间字段（下标 1-10）。
///
/// 对应 Java: `AssignmentFieldLists.CUSTOM_FINISH`
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

/// 自定义日期字段（下标 1-10）。
///
/// 对应 Java: `AssignmentFieldLists.CUSTOM_DATE`
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

/// 自定义数字字段（下标 1-20）。
///
/// 对应 Java: `AssignmentFieldLists.CUSTOM_NUMBER`
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

/// 自定义工期字段（下标 1-10）。
///
/// 对应 Java: `AssignmentFieldLists.CUSTOM_DURATION`
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

/// 自定义成本字段（下标 1-10）。
///
/// 对应 Java: `AssignmentFieldLists.CUSTOM_COST`
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

/// 自定义布尔标志字段（下标 1-20）。
///
/// 对应 Java: `AssignmentFieldLists.CUSTOM_FLAG`
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

/// 企业自定义成本字段（下标 1-10）。
///
/// 对应 Java: `AssignmentFieldLists.ENTERPRISE_CUSTOM_COST`
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

/// 企业自定义日期字段（下标 1-30）。
///
/// 对应 Java: `AssignmentFieldLists.ENTERPRISE_CUSTOM_DATE`
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

/// 企业自定义工期字段（下标 1-10）。
///
/// 对应 Java: `AssignmentFieldLists.ENTERPRISE_CUSTOM_DURATION`
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

/// 企业自定义布尔标志字段（下标 1-20）。
///
/// 对应 Java: `AssignmentFieldLists.ENTERPRISE_CUSTOM_FLAG`
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

/// 企业自定义数字字段（下标 1-40）。
///
/// 对应 Java: `AssignmentFieldLists.ENTERPRISE_CUSTOM_NUMBER`
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

/// 企业自定义文本字段（下标 1-40）。
///
/// 对应 Java: `AssignmentFieldLists.ENTERPRISE_CUSTOM_TEXT`
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

/// 字段依赖表：修改某字段后需要清理（重算）的字段。
///
/// 对应 Java: `private static final Map<FieldType, List<FieldType>> DEPENDENCY_MAP`
/// 与 `FieldContainerDependencies.calculatedField(X).dependsOn(A, B)`
///（语义为：`A` / `B` 变化时清除 `X` 的缓存值）。
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

/// 按下标取「原始 timephased 基准工时」字段。
///
/// 对应 Java: `AssignmentFieldLists.RAW_TIMEPHASED_BASELINE_WORKS[index]`
///（下标 0 = Baseline，1..=10 = Baseline1..10）；越界返回 `None`
///（Java 会抛 `ArrayIndexOutOfBoundsException`）。
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

/// 按下标取「原始 timephased 基准预算工时」字段。
///
/// 对应 Java: `AssignmentFieldLists.RAW_TIMEPHASED_BASELINE_BUDGET_WORKS[index]`
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

/// 按下标取「原始 timephased 基准成本」字段。
///
/// 对应 Java: `AssignmentFieldLists.RAW_TIMEPHASED_BASELINE_COSTS[index]`
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

/// 按下标取「原始 timephased 基准预算成本」字段。
///
/// 对应 Java: `AssignmentFieldLists.RAW_TIMEPHASED_BASELINE_BUDGET_COSTS[index]`
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

/// 全为 `None` 的工期列表。
fn empty_durations(len: usize) -> Vec<Option<Duration>> {
    vec![None; len]
}

/// 全为 `None` 的数值列表。
fn empty_numbers(len: usize) -> Vec<Option<f64>> {
    vec![None; len]
}

/// 把区间与可选日期比较。
///
/// 对应 Java 的 `LocalDateTimeRange.compareTo(LocalDateTime)`；日期为空时返回 `None`。
fn range_compare_to_date(range: &LocalDateTimeRange, date: Option<NaiveDateTime>) -> Option<i32> {
    date.map(|date| range.compare_to_date(date))
}

/// 区间列表是否完全落在分配区间之外。
///
/// 对应 Java: `ranges.get(ranges.size() - 1).isBefore(assignmentRange) || ranges.get(0).isAfter(assignmentRange)`
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

/// 找第一个与目标区间相交的下标；没有时返回 `ranges.len()`。
///
/// 对应 Java 中 `while (rangeIndex < ranges.size() && !ranges.get(rangeIndex).intersectsWith(...)) ++rangeIndex;`
fn first_intersecting_range(ranges: &[LocalDateTimeRange], range: &LocalDateTimeRange) -> usize {
    let mut index = 0;
    while index < ranges.len() && !ranges[index].intersects_with(range) {
        index += 1;
    }
    index
}

/// 取日历上两个时刻之间的工时；日历缺失或已被借用时返回 0（Java 会抛 NPE）。
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

/// 按日历提供的单位换算默认值换算工期；日历缺失时按原值返回（Java 会抛 NPE）。
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

/// 把费率换算为「每小时金额」。
///
/// 对应 Java: `RateHelper.convertToHours(TimeUnitDefaultsContainer defaults, Rate rate)`
///（`RateHelper` 未移植，按等价逻辑就地实现）。
fn rate_to_hours(rate: Option<Rate>, calendar: Option<&Rc<RefCell<ProjectCalendar>>>) -> f64 {
    let Some(rate) = rate else {
        return 0.0;
    };

    let amount = rate.get_amount();

    // 对应 Java 的 `defaults.getMinutesPerDay()` 等取值
    let (minutes_per_day, minutes_per_week, minutes_per_month) = match calendar {
        Some(calendar) => match calendar.try_borrow() {
            Ok(calendar) => (
                calendar.get_minutes_per_day().map(f64::from).unwrap_or(0.0),
                calendar.get_minutes_per_week().map(f64::from).unwrap_or(0.0),
                calendar.get_minutes_per_month().map(f64::from).unwrap_or(0.0),
            ),
            Err(_) => (0.0, 0.0, 0.0),
        },
        None => (0.0, 0.0, 0.0),
    };

    // 除数为 0 时按原值返回（Java 会抛 NPE）
    let scale = |amount: f64, divisor: f64| {
        if divisor == 0.0 {
            amount
        } else {
            (amount * 60.0) / divisor
        }
    };

    match rate.get_units() {
        TimeUnit::Minutes => amount * 60.0,
        TimeUnit::Days => scale(amount, minutes_per_day),
        TimeUnit::Weeks => scale(amount, minutes_per_week),
        TimeUnit::Months => scale(amount, minutes_per_month),
        TimeUnit::Years => {
            let divisor = minutes_per_week * 52.0;
            scale(amount, divisor)
        }
        _ => amount,
    }
}

/// 取下一个费率行的开始时间；没有下一行时取 `LocalDateTimeHelper.END_DATE_NA`。
///
/// 对应 Java: `private LocalDateTime getNextRateStart(List<CostRateTableEntry> rates, int costRateTableIndex)`
fn get_next_rate_start(rates: &[&CostRateTableEntry], index: usize) -> Option<NaiveDateTime> {
    if index + 1 < rates.len() {
        rates[index + 1].get_start_date()
    } else {
        Some(LocalDateTimeHelper::end_date_na())
    }
}

/// 该 timephased 工时项是否覆盖非工作时间。
///
/// 对应 Java: `TimephasedUtility.itemIsNonWorking(ProjectCalendar calendar, TimephasedItem<?> item)`
fn item_is_non_working(
    calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
    item: &TimephasedWork,
) -> bool {
    calendar_work(calendar, item.get_start(), item.get_finish(), TimeUnit::Hours).get_duration() == 0.0
}

/// 该 timephased 成本项是否覆盖非工作时间。
///
/// 对应 Java: `TimephasedUtility.itemIsNonWorking(...)`
fn cost_item_is_non_working(
    calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
    item: &TimephasedCost,
) -> bool {
    calendar_work(calendar, item.get_start(), item.get_finish(), TimeUnit::Hours).get_duration() == 0.0
}

/// 把原始 timephased 工时按时间区间切分。
///
/// 对应 Java: `TimephasedUtility.segmentWork(ProjectCalendar calendar, List<TimephasedWork> work, List<LocalDateTimeRange> ranges, TimeUnit targetUnits)`
///
/// Java 的入参校验（`validateRanges` / `validateTimephasedWork`）抛异常，Rust 侧不校验。
fn segment_work(
    calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
    work: &[TimephasedWork],
    ranges: &[LocalDateTimeRange],
    target_units: TimeUnit,
) -> Vec<Option<Duration>> {
    if work.is_empty() {
        return empty_durations(ranges.len());
    }

    // Java: Microsoft Project 特例 —— 只有一段、起止相同且总量非 0 时，工时计入最后一个区间
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

    // Java 用 -1 表示 null，最后统一映射
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
            // Java: 区间早于当前 timephased 项 —— 该区间没有工时
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
            // Java: 区间与本项相交
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
                // Java: 当前区间仍有剩余时间需要计算
                current_range = LocalDateTimeRange::new(Some(work_finish), Some(range_end));
            }
        }

        if !more_time_in_range {
            // Java: 当前区间处理完毕，前进
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

/// 把原始 timephased 成本按时间区间切分。
///
/// 对应 Java: `TimephasedUtility.segmentCost(ProjectCalendar calendar, List<TimephasedCost> cost, List<LocalDateTimeRange> ranges)`
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

        // Java: 区间与本项相交
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

/// 把原始 timephased 工时按时间区间切分为材料用量。
///
/// 对应 Java: `TimephasedUtility.segmentMaterial(ResourceAssignment assignment, ProjectCalendar calendar, List<TimephasedWork> work, List<LocalDateTimeRange> ranges)`
///
/// Rust 侧用 `is_material` 布尔参数表达 Java 的「资源为空或类型不是 MATERIAL」判断。
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

/// 逐项相加两个 timephased 工期列表。
///
/// 对应 Java: `TimephasedUtility.addTimephasedDurations(List<Duration> list1, List<Duration> list2)`
///
/// Java 在长度不一致时抛 `RuntimeException`；Rust 侧原样返回第一个列表。
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

/// 逐项相加两个 timephased 数值列表。
///
/// 对应 Java: `TimephasedUtility.addTimephasedNumbers(List<Number> list1, List<Number> list2)`
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

/// 以 Java 的 `null` 语义渲染可选值。
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

    /// 构造一个未接线的分配（用于字段读写测试）。
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

        // unique ID 也可经 FieldContainer 接口读写
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

        // 传 None 等价于 Java 传 null（移除字段）
        assignment.set_units(None);
        assert_eq!(assignment.get_units(), None);
    }

    #[test]
    fn boolean_and_flag_fields_use_java_defaults() {
        let mut assignment = assignment();

        // Java: getCalculateCostsFromUnits() 的默认值为 true
        assert!(assignment.get_calculate_costs_from_units());
        // Java: 未设置的布尔字段默认 false
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

        // Java 越界会抛 IllegalArgumentException；Rust 侧为 None / no-op
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
        // Java: REMAINING_ASSIGNMENT_UNITS 无值时退化为计划单位
        assert_eq!(assignment.get_remaining_units(), Some(100.0));
        assert_eq!(assignment.get_units(), Some(100.0));
        // Java: RATE_INDEX 默认 0
        assert_eq!(assignment.get_rate_index(), Some(0));
        // Java: RATE_SOURCE 默认 Resource（由计算字段 defaultRateSource 提供）
        assert_eq!(assignment.get_rate_source(), RateSource::Resource);
        // Java: COST_RATE_TABLE 越界时取 0
        assert_eq!(assignment.get_cost_rate_table_index(), 0);
        assignment.set_cost_rate_table_index(7);
        assert_eq!(assignment.get_cost_rate_table_index(), 0);
        assignment.set_cost_rate_table_index(2);
        assert_eq!(assignment.get_cost_rate_table_index(), 2);
        // 无资源/角色时没有费率表
        assert!(assignment.with_cost_rate_table(|table| table.is_none()));
    }

    #[test]
    fn dependency_map_clears_cached_calculated_values() {
        let mut assignment = assignment();

        // Java: COST_VARIANCE 依赖 COST 与 BASELINE_COST，修改后应清除缓存
        assignment
            .set_assignment_value(AssignmentField::CostVariance, Some(FieldValue::Double(1.0)));
        assert_eq!(assignment.get_cost_variance(), Some(1.0));

        assignment.set_cost(Some(10.0));
        assert_eq!(
            assignment.peek_cached_value(FieldKey::Assignment(AssignmentField::CostVariance)),
            None
        );
        // 缓存被清除后按计算值返回（BASELINE_COST 为空 → 无值）
        assert_eq!(assignment.get_cost_variance(), None);
    }

    #[test]
    fn timephased_segment_work_maps_ranges_without_calendar() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1)
            .and_then(|date| date.and_hms_opt(9, 0, 0))
            .expect("合法日期");
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

        // 无日历时按时间比例切分（实际工时按小时计）
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

        // 长度不一致时不再相加（Java 抛 RuntimeException）
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

        // 未设置时按默认空列表返回
        assert!(assignment.get_raw_timephased_budget_work().is_empty());
        assert!(assignment.get_raw_timephased_remaining_overtime_work().is_empty());
    }

    #[test]
    fn notes_and_guid_use_field_table() {
        let mut assignment = assignment();

        assignment.set_notes(Some("hello".to_owned()));
        assert_eq!(assignment.get_notes(), "hello");
        assert!(assignment.get_notes_object().is_some());
        // Java 的 get(AssignmentField.NOTES) 取到的就是该对象
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

        // RATE_SOURCE 未显式设置时由计算字段给出默认值（Java 的 defaultRateSource）
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

        // Java 的 get(AssignmentField.RAW_TIMEPHASED_PLANNED_WORK) 取到的就是该列表
        assert!(matches!(
            assignment.peek_cached_value(FieldKey::Assignment(
                AssignmentField::RawTimephasedPlannedWork
            )),
            Some(FieldValue::TimephasedWork(work)) if work.len() == 1
        ));

        // 未设置的 timephased 字段按 Java 的 defaultTimephasedWork / defaultTimephasedCost 回落为空列表
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

    /// 经 [`FieldContainer`] trait 入口查询 timephased 数据（Java 的
    /// `getTimephasedDurationValues` / `getTimephasedNumericValues`）。
    #[test]
    fn timephased_queries_via_field_container_trait() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1)
            .and_then(|date| date.and_hms_opt(9, 0, 0))
            .expect("合法日期");
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

        // 字段不在分派表中时与 Java 一致返回全 None 列表
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
        // Java 在已有记录时抛 MPXJException；Rust 侧返回既有记录
        assignment
            .add_workgroup_assignment()
            .set_schedule_id(Some("SCHED-9".to_owned()));

        let workgroup = assignment
            .get_workgroup_assignment()
            .expect("工作组字段已创建");
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
                .expect("合法日期")
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
