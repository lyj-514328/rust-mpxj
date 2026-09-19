//! 对应 Java: `org/mpxj/Task.java`

use std::cell::RefCell;
use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::rc::{Rc, Weak};

use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::abstract_field_container::AbstractFieldContainer;
use crate::accrue_type::AccrueType;
use crate::activity_status::ActivityStatus;
use crate::activity_type::ActivityType;
use crate::common::boolean_helper::BooleanHelper;
use crate::common::local_date_time_helper::LocalDateTimeHelper;
use crate::common::number_helper::NumberHelper;
use crate::constraint_type::ConstraintType;
use crate::duration::Duration;
use crate::earned_value_method::EarnedValueMethod;
use crate::field_container::FieldContainer;
use crate::field_key::FieldKey;
use crate::field_value::FieldValue;
use crate::listener::field_listener::FieldListener;
use crate::mpxj_enum::MpxjEnum;
use crate::notes::{Notes, PlainNotes};
use crate::percent_complete_type::PercentCompleteType;
use crate::priority::Priority;
use crate::project_calendar::ProjectCalendar;
use crate::project_entity_with_id::ProjectEntityWithID;
use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
use crate::project_file::ProjectFile;
use crate::relation::{Builder as RelationBuilder, Relation};
use crate::relation_type::RelationType;
use crate::resource::Resource;
use crate::resource_assignment::ResourceAssignment;
use crate::task_field::TaskField;
use crate::task_mode::TaskMode;
use crate::task_type::TaskType;
use crate::time_unit::TimeUnit;

// ---------------------------------------------------------------------------
// 文件头说明
// ---------------------------------------------------------------------------
//
// # Java 形态
//
// ```java
// public final class Task extends AbstractFieldContainer<Task>
//    implements Comparable<Task>, ProjectEntityWithID, ChildTaskContainer
// ```
//
// 字段（Java 侧只有 5 个成员变量，其余全部存在 `AbstractFieldContainer` 的字段表里）：
//
// | Java 字段 | Rust 字段 |
// |---|---|
// | `private final ProjectFile m_parentFile` | `parent_file: Weak<RefCell<ProjectFile>>` |
// | `private Task m_parent` | `parent: Weak<RefCell<Task>>` |
// | `private final List<Task> m_children` | `children: Vec<Rc<RefCell<Task>>>` |
// | `private RecurringTask m_recurringTask` | 未移植（见「跳过的 Java 方法」） |
// | `AbstractFieldContainer` 的 `m_fields` | `fields: AbstractFieldContainer` |
//
// # 引用关系与生命周期（Java → Rust）
//
// * `m_parentFile`：Java 由 GC 管理，`Task` 直接持有 `ProjectFile`。Rust 侧
//   `ProjectFile` 放在 `Rc<RefCell<ProjectFile>>` 中，任务反向持有其父文件，
//   若用强引用会与 `ProjectFile → TaskContainer → Task` 形成 `Rc` 循环，
//   故改为 [`Weak`]（见 [`Task::get_parent_file`]）。
// * `m_parent` / `m_children`：Java 中父子互相强引用（同一张对象图内），Rust 侧
//   子任务用 `Rc<RefCell<Task>>` 强引用持有（与 `TaskContainer` 共享同一实例），
//   父任务回指用 [`Weak`] 避免 `Rc` 循环。
// * Java 的 `Task` 没有对 `ProjectContext` 的直接引用，上下文经由
//   `m_parentFile` 获取；Rust 侧保持同样的形态，另提供
//   [`Task::get_project_context`] 作为便捷入口（Rust 特有）。
// * 由于父子接线需要实体自身的共享句柄，Java 中隐含的 `this` 在 Rust 侧无法省略，
//   因此 [`Task::add_task`] / [`Task::add_child_task`] / [`Task::add_child_task_before`]
//   / [`Task::remove_child_task`] / [`Task::clear_child_tasks`] / [`Task::remove`]
//   改为以 `this: &Rc<RefCell<Task>>` 作为首个参数的关联函数。
//
// # 字段存储（最关键）
//
// Java 的 `set` / `get` / `getCachedValue` 由基类 `AbstractFieldContainer` 提供，
// 三个抽象方法 `handleFieldChange` / `getAlwaysCalculatedField` / `getCalculationMethod`
// 由 `Task` 实现。Rust 侧改为**组合**：`Task` 持有
// [`AbstractFieldContainer`] 并把 `set` / `get` / `get_cached_value` 委托给它，
// 上述三个抽象方法的职责落在 `Task` 自己的实现里（见
// [`Task::handle_field_change`]、[`Task::get_always_calculated_field`]、
// [`Task::get_calculation_method`]）。
//
// Java 的静态 `CALCULATED_FIELD_MAP`（`Map<FieldType, Function<Task, Object>>`）与
// `DEPENDENCY_MAP`（`Map<FieldType, List<FieldType>>`）在 Rust 侧改写为 `match`
// 分派（[`Task::get_calculation_method`] / [`dependency_fields`]），不使用函数指针映射表。
//
// # 与 Java 的差异（除生命周期外）
//
// 1. **计算值不做写回缓存**：Java 的 `get(field)` 在算出非「始终计算」字段的值后
//    会 `set(field, result)` 写回字段表做缓存（`AbstractFieldContainer.get`）。
//    Rust 的 `&self` getter 无法写字段表，故每次按需重算。语义等价（字段表里已有的
//    取值优先返回），差别仅在于不做缓存、监听器不会收到计算结果的变更事件。
// 2. **字段变更事件与处理顺序**：Java 在 `AbstractFieldContainer.set` 中先调
//    `handleFieldChange` 再 `fireFieldChangeEvent`；Rust 侧的监听器广播在
//    [`AbstractFieldContainer::set`] 内部完成，因此 `Task::set` 只能在其之后调用
//    [`Task::handle_field_change`]。对字段依赖清理的结果无影响。
// 3. **`UNIQUE_ID` 的索引维护**：Java 的 `handleFieldChange` 在 `UNIQUE_ID` 变化时调用
//    `m_parentFile.getTasks().updateUniqueID(this, old, new)`，`setID` 还会调用
//    `mapID` / `unmapID`。这些都需要实体自身的共享引用，Rust 侧改由 `TaskContainer`
//    在 `add` 时建立索引（见 `crate::task_container`）。`set_id` / `set_unique_id`
//    因此只写字段值。
// 4. **类型强转**：Java 的 `(Duration) get(...)` 等强转在类型不符时抛
//    `ClassCastException`；Rust 侧统一返回 `None`（纯文本 / 日期 / 工期的读取辅助方法）。
// 5. **`Number` → `f64`**：Java 的 `Number` 参数/返回值在 Rust 侧统一为 `f64`
//    （与 [`crate::common::number_helper::NumberHelper`] 的约定一致）。
// 6. **`equals` / `hashCode` / `compareTo`**：Java 未重写 `equals` / `hashCode`，
//    即按对象同一性比较；Rust 的 `Task` 是普通 struct，无法表达同一性，故
//    **不实现** `PartialEq` / `Eq` / `Hash`（调用方以 `Rc::ptr_eq` 判断同一实体）。
//    Java 的 `compareTo` 按 ID 排序，与「同一性相等」不自洽，不满足 Rust
//    `Ord` 的契约（`a == b` 必须与 `a.cmp(&b) == Equal` 一致），因此改为
//    [`Task::compare_to`] 方法而非 `Ord` 实现。
// 7. **枚举 / UUID / Notes 的字段值编码**：`crate::field_value::FieldValue` 目前只有
//    标量变体（其文档说明实体引用与集合变体待后续补入）。为完整翻译访问器，Rust 侧
//    采用无损的编码约定，并在读取时还原：
//    * 枚举 → `FieldValue::Integer`（`MpxjEnum::value()`；`TaskType` 无 int 值，
//      用声明序号即 `TaskType::ALL` 的下标；`Priority` 用 `get_value()`）；
//    * `UUID` → `FieldValue::String`（`Uuid::to_string()` / `Uuid::parse_str`）；
//    * `Notes` → `FieldValue::String`（纯文本，见跳过的 Java 方法第 8 条）。
//    因此 `get_cached_value` 对这些字段返回的是编码后的标量，而非 Java 中的对象。
//    这是本文件唯一的「表示层」取舍，字段读写与计算语义不受影响。
// 8. **关系列表**：`TaskField.PREDECESSORS` / `TaskField.SUCCESSORS` 的取值是
//    `List<Relation>`，`FieldValue` 同样没有对应变体。这两个字段在 Java 中属于
//    「始终计算」集合（不读缓存、只现算），因此 Rust 侧的
//    [`Task::get_predecessors`] / [`Task::get_successors`] 直接向
//    `RelationContainer` 查询（对应 Java 的 `calculatePredecessors` /
//    `calculateSuccessors`），不再经过字段表；`is_predecessor` / `is_successor`
//    也随之可用。字段表层面这两个字段的 `get` 仍返回 `None`。
//
// # 依赖模块的接口（已按并行翻译落地的真实签名书写）
//
// 下列模块在编写本文件时已落地，调用形状与它们一致：
//
// ```text
// TaskContainer::add_task(&mut self, task: Rc<RefCell<Task>>)
// TaskContainer::get_next_id(&self) -> i32
// TaskContainer::get_field_type_by_alias(&self, alias: &str) -> Option<FieldKey>
//
// ResourceAssignmentContainer::add(&mut self, assignment: Rc<RefCell<ResourceAssignment>>)
// ResourceAssignmentContainer::get_by_task_unique_id(&self, id: Option<i32>) -> Vec<Rc<RefCell<ResourceAssignment>>>
// ResourceAssignment::with_parent_file(&Rc<RefCell<ProjectFile>>)
// ResourceAssignment::set_task_unique_id / set_resource_unique_id / set_work / set_units
// ResourceAssignment::get_resource(&self) -> Option<Rc<RefCell<Resource>>>
// ResourceAssignment::get_resource_unique_id(&self) -> Option<i32>
// （Java 的常量 `ResourceAssignment.DEFAULT_UNITS` 尚未提供，`addResourceAssignment` 处直接使用其值 1.0）
//
// RelationContainer::add_predecessor(&mut self, builder: Builder) -> Rc<RefCell<Relation>>
// RelationContainer::remove_predecessor(&mut self, successor: &Task, predecessor: &Task, RelationType, Option<Duration>) -> bool
// RelationContainer::get_predecessors(&self, task: &Task) -> Vec<Rc<RefCell<Relation>>>
// RelationContainer::get_successors(&self, task: &Task) -> Vec<Rc<RefCell<Relation>>>
// Relation::Builder::successor_task(&Rc<RefCell<Task>>) -> Builder
// Relation::get_predecessor_task(&self) / get_successor_task(&self) -> Option<Rc<RefCell<Task>>>
//
// Resource::get_unique_id(&self) -> Option<i32>            // 经 ProjectEntityWithUniqueID
// ProjectProperties::get_default_duration_units / get_critical_slack_limit / get_status_date / get_project_id
// ProjectProperties: impl TimeUnitDefaultsContainer
// ProjectCalendar::get_unique_id / get_date / get_next_work_start / get_work
// ProjectCalendar: impl TimeUnitDefaultsContainer
// ProjectCalendarContainer::get_by_unique_id(&self, Option<i32>) -> Option<Rc<RefCell<ProjectCalendar>>>
// Duration::convert_units(&self, TimeUnit, &dyn TimeUnitDefaultsContainer)
// ```
//
// 仍未落地、按「与 Java 等价的 Rust 签名」假设的是 `crate::project_file::ProjectFile`：
//
// ```text
// ProjectFile::get_project_config(&self) -> &ProjectConfig
// ProjectFile::get_project_properties(&self) -> &ProjectProperties
// ProjectFile::get_tasks(&self) -> &TaskContainer
// ProjectFile::get_tasks_mut(&mut self) -> &mut TaskContainer
// ProjectFile::get_resource_assignments(&self) -> &ResourceAssignmentContainer
// ProjectFile::get_resource_assignments_mut(&mut self) -> &mut ResourceAssignmentContainer
// ProjectFile::get_relations(&self) -> &RelationContainer
// ProjectFile::get_relations_mut(&mut self) -> &mut RelationContainer
// ProjectFile::get_calendars(&self) -> &ProjectCalendarContainer
// ProjectFile::get_default_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>>
// ProjectFile::get_child_tasks(&self) -> Vec<Rc<RefCell<Task>>>
// ProjectFile::get_task_by_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Task>>>
// ProjectFile::get_resource_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Resource>>>
// ProjectFile::get_project_context(&self) -> Rc<RefCell<ProjectContext>>
// ProjectFile::get_unique_id_object_sequence(&mut self, TypeId) -> Rc<ObjectSequence>
// ProjectFile::remove_task(&mut self, task: &Rc<RefCell<Task>>)
// ProjectFile::read_external_project(&mut self, file: Option<&str>) -> Option<Rc<RefCell<ProjectFile>>>
// ProjectFile::add_external_project(&mut self, file: Option<&str>, project: Option<Rc<RefCell<ProjectFile>>>)
// ```
//
// 另注：`crate::task_container::TaskContainer::add` 会调用
// `Task::new_in_file(&project_file, None)`，本文件已提供该构造方法名。
//
// # 已知的跨模块接口分歧（不改动他人文件，仅记录）
//
// * `crate::cpm::microsoft_slack_calculator` 按
//   `Task::get_parent_file(&self) -> Rc<RefCell<ProjectFile>>`、
//   `Task::get_constraint_type(&self) -> ConstraintType` 书写。本文件保持
//   Java 语义：父文件是弱引用（无父文件的任务存在），`get_constraint_type` 在字段
//   未设置时返回 `None`（Java 返回 `null`）。集成时该文件需相应处理 `Option`。
// * `crate::common::slack_helper` 按 `Task::set_start_slack(Duration)` /
//   `set_finish_slack(Duration)` / `set_total_slack(Duration)` 书写。本文件按
//   Java 的可空语义统一为 `Option<Duration>`，集成时该处需包一层 `Some(...)`。
// * `crate::slack_calculator` 的测试使用 `Task::new()`，本文件已提供该无参构造。
//
// # 跳过的 Java 方法（原因）
//
// | Java 方法 | 跳过原因 |
// |---|---|
// | `getTimephasedDurationValues` / `getTimephasedNumericValues` / `getTimephased*`（共 43 个） | 依赖 `ResourceAssignment` 的 timephased API 与 `TimephasedUtility`（均未移植），且不属于 reader 读入路径 |
// | `getWorkSplits` / `calculateWorkSplits` / `reduceWorkSplits` / `addWorkSplit` | 字段值 / 返回值为 `List<LocalDateTimeRange>`，`FieldValue` 无列表变体 |
// | `getActivityCodeValues` / `addActivityCodeValue` / `defaultActivityCodeValues` | `ActivityCode` / `ActivityCodeValue` 模块未在 `src/lib.rs` 声明 |
// | `getExpenseItems` / `setExpenseItems` / `defaultExpenseItems` | `ExpenseItem` 模块未在 `src/lib.rs` 声明 |
// | `getSteps` / `setSteps` / `defaultSteps` | `FieldValue` 无列表变体，无法存储 `List<Step>` |
// | `addRecurringTask` / `getRecurringTask` | `RecurringTask` 模块未在 `src/lib.rs` 声明（Java 侧 `RecurringTask extends Task implements RecurringData`，`crate::recurring_data` 只移植了 `RecurringData` 数据部分） |
// | `getLocation` / `setLocation` | `Location` 模块未在 `src/lib.rs` 声明 |
// | `getBaselineTask` / `getBaselineTask(int)` | Java 以 `Task` 对象（同一性）作 `Map` 键，Rust 无法表达；且 `ProjectFile` 的基线任务映射尚未落地 |
// | `setNotesObject` 的 `Notes` 子类型保真 | Rust 侧 `Notes` 是 trait，`FieldValue` 无法承载 trait 对象，故只保留纯文本；`RtfNotes` / `HtmlNotes` / `StructuredNotes` 的具体子类型信息丢失 |
// | `set(FieldType, boolean)`（私有重载） | Rust 无重载，由 `put_bool` 辅助方法承担 |
// | 静态 `TIMEPHASED_WORK_FUNCTIONS` / `TIMEPHASED_NUMERIC_FUNCTIONS` | 服务于上面已跳过的 timephased 方法 |
//
// 反射、JDBC、XML/JAXB、GUI、`javax.*`、`Serializable`：`Task` 中未出现，无跳过。

/// 任务记录。
///
/// 对应 Java: `public final class Task extends AbstractFieldContainer<Task>
/// implements Comparable<Task>, ProjectEntityWithID, ChildTaskContainer`
///
/// 继承与接口的还原方式见文件头说明。
pub struct Task {
    /// 对应 Java: `private final ProjectFile m_parentFile;`
    ///
    /// Rust 侧存弱引用，避免 `ProjectFile → TaskContainer → Task → ProjectFile`
    /// 形成 `Rc` 循环。
    parent_file: Weak<RefCell<ProjectFile>>,

    /// 对应 Java: `private Task m_parent;`
    ///
    /// Rust 侧存弱引用，避免父子任务互相强引用形成 `Rc` 循环。
    parent: Weak<RefCell<Task>>,

    /// 对应 Java: `private final List<Task> m_children = new ArrayList<>();`
    children: Vec<Rc<RefCell<Task>>>,

    /// 对应 Java: 继承自 `AbstractFieldContainer` 的 `m_fields`。
    ///
    /// Java 的继承在 Rust 侧改为组合，`set` / `get` / `get_cached_value` 委托给它。
    fields: AbstractFieldContainer,

    /// 对应 Java: `AbstractFieldContainer.m_clearDependentFieldsEnabled`。
    ///
    /// Java 的依赖清理递归地走 `set`（进而触发 `handleFieldChange`），因此本标志
    /// 需要在 `Task` 侧维护一份（`AbstractFieldContainer` 的同名标志同时保持同步）。
    clear_dependent_fields_enabled: bool,
}

impl Default for Task {
    /// 对应 Java: 无（Rust 特有，等价于 [`Task::new`]）。
    fn default() -> Self {
        Self::new()
    }
}

impl Task {
    /// 构造一个不属于任何项目文件的独立任务（Rust 特有）。
    ///
    /// Java 的构造方法是包私有的 `Task(ProjectFile file, Task parent)`，必须提供
    /// 父文件；Rust 侧另提供不带父文件的形态，便于测试与工具代码构造临时任务
    /// （此时依赖父文件的访问器返回空值而不 panic）。需要接入项目文件时请用
    /// [`Task::new_in_file`]。
    pub fn new() -> Self {
        Self {
            parent_file: Weak::new(),
            parent: Weak::new(),
            children: Vec::new(),
            fields: AbstractFieldContainer::new(),
            clear_dependent_fields_enabled: true,
        }
    }

    /// 构造方法。
    ///
    /// 对应 Java: `Task(ProjectFile file, Task parent)`
    ///
    /// Java 在构造方法中按 `ProjectConfig` 的开关自动生成 unique ID、ID、WBS、
    /// 大纲编号与大纲级别；Rust 侧保持一致。
    ///
    /// 方法名 `new_in_file` 为 Rust 侧命名（Java 的构造方法名与类名相同），
    /// 供 `crate::task_container::TaskContainer::add` 等调用方使用。
    pub fn new_in_file(
        file: &Rc<RefCell<ProjectFile>>,
        parent: Option<&Rc<RefCell<Task>>>,
    ) -> Self {
        let mut task = Self::new();
        task.parent_file = Rc::downgrade(file);
        task.parent = parent.map_or_else(Weak::new, Rc::downgrade);

        // Java: ProjectConfig config = file.getProjectConfig();
        let (auto_task_unique_id, auto_task_id, auto_wbs, auto_outline_number, auto_outline_level) = {
            let file_ref = file.borrow();
            let config = file_ref.get_project_config();
            (
                config.get_auto_task_unique_id(),
                config.get_auto_task_id(),
                config.get_auto_wbs(),
                config.get_auto_outline_number(),
                config.get_auto_outline_level(),
            )
        };

        if auto_task_unique_id {
            // Java: setUniqueID(file.getUniqueIdObjectSequence(Task.class).getNext());
            let sequence = file
                .borrow_mut()
                .get_unique_id_object_sequence(std::any::TypeId::of::<Task>());
            task.set_unique_id(Some(sequence.get_next()));
        }

        if auto_task_id {
            // Java: setID(file.getTasks().getNextID());
            let next_id = file.borrow().get_tasks().get_next_id();
            task.set_id(Some(next_id));
        }

        if auto_wbs {
            task.generate_wbs(parent);
        }

        if auto_outline_number {
            task.generate_outline_number(parent);
        }

        if auto_outline_level {
            let outline_level = match parent {
                None => 1,
                Some(parent) => NumberHelper::get_int(parent.borrow().get_outline_level().map(f64::from)) + 1,
            };
            task.set_outline_level(Some(outline_level));
        }

        task
    }

    /// 取当前任务所属的项目文件（弱引用升级；文件已释放时返回 `None`）。
    ///
    /// 对应 Java: `public ProjectFile getParentFile()`
    pub fn get_parent_file(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.parent_file.upgrade()
    }

    /// 取当前任务所属的 `ProjectContext`（Rust 特有便捷入口）。
    ///
    /// Java 侧 `Task` 不直接持有 `ProjectContext`，上下文经由 `m_parentFile` 获取；
    /// 此处保持同样的来源。
    pub fn get_project_context(&self) -> Option<Rc<RefCell<crate::project_context::ProjectContext>>> {
        self.parent_file
            .upgrade()
            .map(|file| file.borrow().get_project_context())
    }

    // -----------------------------------------------------------------------
    // 字段容器：set / get / get_cached_value 及三个「抽象方法」的落位
    // -----------------------------------------------------------------------

    /// 设置字段值（含计算字段的写回）。
    ///
    /// 对应 Java: `@Override public void set(FieldType field, Object value)`
    pub fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
        // Java 对 null 的 field 直接返回；Task 只接收 TaskField。
        if let FieldKey::Task(task_field) = field {
            self.set_task_field(task_field, value);
        }
    }

    /// 读取字段值（含计算字段）。
    ///
    /// 对应 Java: `@Override public Object get(FieldType field)`
    pub fn get(&self, field: FieldKey) -> Option<FieldValue> {
        if let FieldKey::Task(task_field) = field {
            return self.get_task_field(task_field);
        }
        None
    }

    /// 只读取已保存的字段值，不触发计算。
    ///
    /// 对应 Java: `@Override public Object getCachedValue(FieldType field)`
    pub fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
        self.fields.get_cached_value(field)
    }

    /// 注册字段变更监听器。
    ///
    /// 对应 Java: `@Override public void addFieldListener(FieldListener listener)`
    pub fn add_field_listener(&mut self, listener: Box<dyn FieldListener>) {
        self.fields.add_field_listener(listener);
    }

    /// 清空字段变更监听器。
    ///
    /// 对应 Java: `@Override public void removeFieldListener(FieldListener listener)`
    pub fn clear_field_listeners(&mut self) {
        self.fields.clear_field_listeners();
    }

    /// 关闭字段更新时的依赖字段清理与事件触发。
    ///
    /// 对应 Java: `public void disableEvents()`
    pub fn disable_events(&mut self) {
        self.clear_dependent_fields_enabled = false;
        self.fields.disable_events();
    }

    /// 开启字段更新时的依赖字段清理与事件触发（默认状态）。
    ///
    /// 对应 Java: `public void enableEvents()`
    pub fn enable_events(&mut self) {
        self.clear_dependent_fields_enabled = true;
        self.fields.enable_events();
    }

    /// 对应 Java: `@Override boolean getAlwaysCalculatedField(FieldType field)`
    ///
    /// Java 的 `ALWAYS_CALCULATED_FIELDS` 集合改写为 `match` 分派。
    pub fn get_always_calculated_field(field: TaskField) -> bool {
        matches!(
            field,
            TaskField::ParentTaskUniqueId
                | TaskField::Predecessors
                | TaskField::Successors
                | TaskField::SchedulePercentComplete
                | TaskField::WorkSplits
        )
    }

    /// 对应 Java: `@Override Function<Task, Object> getCalculationMethod(FieldType field)`
    ///
    /// Java 返回一个 `Function<Task, Object>`（来自静态 `CALCULATED_FIELD_MAP`），
    /// Rust 侧改为直接 `match` 分派并返回计算结果 —— 语义与
    /// `CALCULATED_FIELD_MAP.get(field).apply(this)` 等价，未命中的字段返回 `None`。
    fn get_calculation_method(&self, field: TaskField) -> Option<FieldValue> {
        match field {
            TaskField::ParentTaskUniqueId => {
                self.calculate_parent_task_unique_id().map(FieldValue::Integer)
            }
            TaskField::StartVariance => self.calculate_start_variance().map(FieldValue::Duration),
            TaskField::FinishVariance => self.calculate_finish_variance().map(FieldValue::Duration),
            TaskField::StartSlack => self.calculate_start_slack().map(FieldValue::Duration),
            TaskField::FinishSlack => self.calculate_finish_slack().map(FieldValue::Duration),
            TaskField::CostVariance => self.calculate_cost_variance().map(FieldValue::Double),
            TaskField::DurationVariance => {
                self.calculate_duration_variance().map(FieldValue::Duration)
            }
            TaskField::WorkVariance => self.calculate_work_variance().map(FieldValue::Duration),
            TaskField::Cv => self.calculate_cv().map(FieldValue::Double),
            TaskField::Sv => self.calculate_sv().map(FieldValue::Double),
            TaskField::FreeSlack => self.calculate_free_slack().map(FieldValue::Duration),
            TaskField::TotalSlack => self.calculate_total_slack().map(FieldValue::Duration),
            TaskField::Critical => self.calculate_critical().map(FieldValue::Boolean),
            TaskField::CompleteThrough => {
                self.calculate_complete_through().map(FieldValue::DateTime)
            }
            TaskField::ExternalProject => {
                self.calculate_external_project().map(FieldValue::Boolean)
            }
            // Java: CALCULATED_FIELD_MAP 中 PREDECESSORS / SUCCESSORS / WORK_SPLITS 的条目
            // 依赖 List<Relation> / List<LocalDateTimeRange> 字段值，见文件头「跳过的 Java 方法」。
            TaskField::ActivityPercentComplete => self
                .calculate_activity_percent_complete()
                .map(FieldValue::Double),
            TaskField::SchedulePercentComplete => self
                .calculate_schedule_percent_complete()
                .map(FieldValue::Double),
            TaskField::ActualRegularCost => self
                .calculate_regular_cost(
                    self.get_actual_cost(),
                    self.get_actual_overtime_cost(),
                )
                .map(FieldValue::Double),
            TaskField::RemainingRegularCost => self
                .calculate_regular_cost(
                    self.get_remaining_cost(),
                    self.get_remaining_overtime_cost(),
                )
                .map(FieldValue::Double),
            TaskField::ActualRegularWork => self
                .calculate_regular_work(self.get_actual_work(), self.get_actual_overtime_work())
                .map(FieldValue::Duration),
            TaskField::RemainingRegularWork => self
                .calculate_regular_work(
                    self.get_remaining_work(),
                    self.get_remaining_overtime_work(),
                )
                .map(FieldValue::Duration),
            TaskField::Active => Some(FieldValue::Boolean(Self::default_active())),
            TaskField::Type => Some(FieldValue::Integer(task_type_to_value(Self::default_type()))),
            TaskField::TaskMode => Some(FieldValue::Integer(Self::default_task_mode().value())),
            // Java: ACTIVITY_CODE_VALUES / EXPENSE_ITEMS / STEPS 的默认值分别为
            // 空 Map / 空 List / 空 List，依赖未移植类型，见文件头「跳过的 Java 方法」。
            TaskField::Expanded => Some(FieldValue::Boolean(Self::default_expanded())),
            TaskField::AutoComputeActuals => {
                Some(FieldValue::Boolean(Self::default_auto_compute_actuals()))
            }
            _ => None,
        }
    }

    /// 清空受本次字段变更影响的计算字段缓存。
    ///
    /// 对应 Java: `@Override void handleFieldChange(FieldType field, Object oldValue, Object newValue)`
    ///
    /// Java 在 `UNIQUE_ID` 变化时会同步 `TaskContainer` 的索引，该动作需要实体自身的
    /// 共享引用，Rust 侧改由 `TaskContainer` 负责（见文件头说明）。
    fn handle_field_change(&mut self, field: TaskField) {
        if field == TaskField::UniqueId {
            return;
        }

        // Java: clearDependentFields(DEPENDENCY_MAP, field);
        self.clear_dependent_fields(field);
    }

    /// 依据依赖表递归清理需要重算的字段。
    ///
    /// 对应 Java: `AbstractFieldContainer.clearDependentFields(Map, FieldType)`
    /// （依赖表为 `Task.DEPENDENCY_MAP`）
    ///
    /// Java 的依赖清理通过 `set` 递归，从而连带清理依赖的依赖；Rust 侧同样走
    /// [`Task::set_task_field`] 以保持该递归语义。
    fn clear_dependent_fields(&mut self, field: TaskField) {
        if !self.clear_dependent_fields_enabled {
            return;
        }

        let dependencies = dependency_fields(field);
        if dependencies.is_empty() {
            return;
        }

        for dependency in dependencies {
            self.set_task_field(*dependency, None);
        }
    }

    /// 写入字段值，并在值真正发生变化时执行字段变更处理。
    ///
    /// 对应 Java: `AbstractFieldContainer.set`（内含 `handleFieldChange` 与事件广播）
    fn set_task_field(&mut self, field: TaskField, value: Option<FieldValue>) {
        let changed = self.fields.get_cached_value(FieldKey::Task(field)).cloned() != value;
        self.fields.set(FieldKey::Task(field), value);

        if changed {
            self.handle_field_change(field);
        }
    }

    /// 读取字段值：优先返回字段表中的取值，否则按计算字段分派求值。
    ///
    /// 对应 Java: `AbstractFieldContainer.get`
    fn get_task_field(&self, field: TaskField) -> Option<FieldValue> {
        let always_calculated = Self::get_always_calculated_field(field);

        if !always_calculated {
            if let Some(value) = self.fields.get_cached_value(FieldKey::Task(field)) {
                return Some(value.clone());
            }
        }

        // Java: 计算结果非 null 且非 always-calculated 时会写回字段表做缓存；
        // Rust 的 &self getter 无法写缓存，改为每次重算（见文件头说明第 1 条）。
        self.get_calculation_method(field)
    }

    // -----------------------------------------------------------------------
    // 类型化读写辅助方法（对应 Java 的强制类型转换与 `set` 重载）
    // -----------------------------------------------------------------------

    /// 对应 Java: `(String) get(field)`（类型不符时 Java 抛 `ClassCastException`，Rust 返回 `None`）
    fn string_field(&self, field: TaskField) -> Option<String> {
        match self.get_task_field(field) {
            Some(FieldValue::String(value)) => Some(value),
            _ => None,
        }
    }

    /// 对应 Java: `(Integer) get(field)`
    fn integer_field(&self, field: TaskField) -> Option<i32> {
        self.get_task_field(field).and_then(|value| value.as_integer())
    }

    /// 对应 Java: `(Number) get(field)`，Rust 侧统一为 `f64`
    fn number_field(&self, field: TaskField) -> Option<f64> {
        self.get_task_field(field).and_then(|value| value.as_double())
    }

    /// 对应 Java: `BooleanHelper.getBoolean((Boolean) get(field))`
    fn boolean_field(&self, field: TaskField) -> bool {
        BooleanHelper::get_boolean(
            self.get_task_field(field)
                .and_then(|value| value.as_boolean()),
        )
    }

    /// 对应 Java: `(LocalDateTime) get(field)`
    fn date_time_field(&self, field: TaskField) -> Option<NaiveDateTime> {
        self.get_task_field(field)
            .and_then(|value| value.as_date_time())
    }

    /// 对应 Java: `(Duration) get(field)`
    fn duration_field(&self, field: TaskField) -> Option<Duration> {
        self.get_task_field(field)
            .and_then(|value| value.as_duration())
    }

    /// 对应 Java: `(UUID) get(field)`（Rust 侧以字符串承载，见文件头差异第 7 条）
    fn uuid_field(&self, field: TaskField) -> Option<Uuid> {
        self.string_field(field)
            .and_then(|value| Uuid::parse_str(&value).ok())
    }

    /// 对应 Java: `set(field, value)`（`Integer` 取值）
    fn put_integer(&mut self, field: TaskField, value: Option<i32>) {
        self.set_task_field(field, value.map(FieldValue::Integer));
    }

    /// 对应 Java: `set(field, value)`（`Number` 取值）
    fn put_number(&mut self, field: TaskField, value: Option<f64>) {
        self.set_task_field(field, value.map(FieldValue::Double));
    }

    /// 对应 Java: `set(field, value)`（`String` 取值）
    fn put_string(&mut self, field: TaskField, value: Option<String>) {
        self.set_task_field(field, value.map(FieldValue::String));
    }

    /// 对应 Java: `private void set(FieldType field, boolean value)`
    ///
    /// Java 是私有重载，Rust 无重载，改用具名辅助方法。
    fn put_bool(&mut self, field: TaskField, value: bool) {
        self.set_task_field(field, Some(FieldValue::Boolean(value)));
    }

    /// 对应 Java: `set(field, value)`（`LocalDateTime` 取值）
    fn put_date_time(&mut self, field: TaskField, value: Option<NaiveDateTime>) {
        self.set_task_field(field, value.map(FieldValue::DateTime));
    }

    /// 对应 Java: `set(field, value)`（`Duration` 取值）
    fn put_duration(&mut self, field: TaskField, value: Option<Duration>) {
        self.set_task_field(field, value.map(FieldValue::Duration));
    }

    /// 对应 Java: `set(field, value)`（枚举取值，Rust 侧编码为 int 值，见文件头差异第 7 条）
    fn put_enum(&mut self, field: TaskField, value: Option<i32>) {
        self.set_task_field(field, value.map(FieldValue::Integer));
    }

    /// 对应 Java: `set(field, value)`（`UUID` 取值，Rust 侧编码为字符串）
    fn put_uuid(&mut self, field: TaskField, value: Option<Uuid>) {
        self.put_string(field, value.map(|value| value.to_string()));
    }
}

/// 取任务字段的依赖列表。
///
/// 对应 Java: `private static final Map<FieldType, List<FieldType>> DEPENDENCY_MAP`
/// （静态初始化块中的 `FieldContainerDependencies` 声明，改为 `match` 分派）
fn dependency_fields(field: TaskField) -> &'static [TaskField] {
    match field {
        TaskField::StartVariance => &[TaskField::Start, TaskField::BaselineStart],
        TaskField::FinishVariance => &[TaskField::Finish, TaskField::BaselineFinish],
        TaskField::StartSlack => &[TaskField::EarlyStart, TaskField::LateStart],
        TaskField::FinishSlack => &[TaskField::EarlyFinish, TaskField::LateFinish],
        TaskField::TotalSlack => &[
            TaskField::StartSlack,
            TaskField::FinishSlack,
            TaskField::ActualStart,
        ],
        TaskField::FreeSlack => &[TaskField::EarlyStart, TaskField::EarlyFinish],
        TaskField::CostVariance => &[TaskField::Cost, TaskField::BaselineCost],
        TaskField::DurationVariance => &[TaskField::Duration, TaskField::BaselineDuration],
        TaskField::WorkVariance => &[TaskField::Work, TaskField::BaselineWork],
        TaskField::Cv => &[TaskField::Bcwp, TaskField::Acwp],
        TaskField::Sv => &[TaskField::Bcwp, TaskField::Bcws],
        TaskField::Critical => &[TaskField::TotalSlack, TaskField::ActualFinish],
        TaskField::CompleteThrough => &[
            TaskField::Duration,
            TaskField::ActualStart,
            TaskField::PercentComplete,
        ],
        TaskField::ExternalProject => &[TaskField::SubprojectFile, TaskField::ExternalTask],
        TaskField::ActivityPercentComplete => &[
            TaskField::PercentCompleteType,
            TaskField::PercentComplete,
            TaskField::PercentWorkComplete,
            TaskField::PhysicalPercentComplete,
        ],
        TaskField::RemainingRegularCost => &[
            TaskField::RemainingCost,
            TaskField::RemainingOvertimeCost,
        ],
        TaskField::ActualRegularCost => &[TaskField::ActualCost, TaskField::ActualOvertimeCost],
        TaskField::RemainingRegularWork => &[
            TaskField::RemainingWork,
            TaskField::RemainingOvertimeWork,
        ],
        TaskField::ActualRegularWork => &[TaskField::ActualWork, TaskField::ActualOvertimeWork],
        _ => &[],
    }
}

/// 取索引对应的任务字段，越界返回 `None`。
///
/// 对应 Java: `private TaskField selectField(TaskField[] fields, int index)`
/// （Java 对越界下标抛 `IllegalArgumentException`，Rust 侧返回 `None`）
fn select_field(fields: &'static [TaskField], index: i32) -> Option<TaskField> {
    let index = usize::try_from(index).ok()?;
    if index < 1 {
        return None;
    }
    fields.get(index - 1).copied()
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.CUSTOM_COST`
fn custom_cost_fields() -> &'static [TaskField] {
    &[
        TaskField::Cost1,
        TaskField::Cost2,
        TaskField::Cost3,
        TaskField::Cost4,
        TaskField::Cost5,
        TaskField::Cost6,
        TaskField::Cost7,
        TaskField::Cost8,
        TaskField::Cost9,
        TaskField::Cost10,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.CUSTOM_DATE`
fn custom_date_fields() -> &'static [TaskField] {
    &[
        TaskField::Date1,
        TaskField::Date2,
        TaskField::Date3,
        TaskField::Date4,
        TaskField::Date5,
        TaskField::Date6,
        TaskField::Date7,
        TaskField::Date8,
        TaskField::Date9,
        TaskField::Date10,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.CUSTOM_DURATION`
fn custom_duration_fields() -> &'static [TaskField] {
    &[
        TaskField::Duration1,
        TaskField::Duration2,
        TaskField::Duration3,
        TaskField::Duration4,
        TaskField::Duration5,
        TaskField::Duration6,
        TaskField::Duration7,
        TaskField::Duration8,
        TaskField::Duration9,
        TaskField::Duration10,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.CUSTOM_FLAG`
fn custom_flag_fields() -> &'static [TaskField] {
    &[
        TaskField::Flag1,
        TaskField::Flag2,
        TaskField::Flag3,
        TaskField::Flag4,
        TaskField::Flag5,
        TaskField::Flag6,
        TaskField::Flag7,
        TaskField::Flag8,
        TaskField::Flag9,
        TaskField::Flag10,
        TaskField::Flag11,
        TaskField::Flag12,
        TaskField::Flag13,
        TaskField::Flag14,
        TaskField::Flag15,
        TaskField::Flag16,
        TaskField::Flag17,
        TaskField::Flag18,
        TaskField::Flag19,
        TaskField::Flag20,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.CUSTOM_FINISH`
fn custom_finish_fields() -> &'static [TaskField] {
    &[
        TaskField::Finish1,
        TaskField::Finish2,
        TaskField::Finish3,
        TaskField::Finish4,
        TaskField::Finish5,
        TaskField::Finish6,
        TaskField::Finish7,
        TaskField::Finish8,
        TaskField::Finish9,
        TaskField::Finish10,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.CUSTOM_NUMBER`
fn custom_number_fields() -> &'static [TaskField] {
    &[
        TaskField::Number1,
        TaskField::Number2,
        TaskField::Number3,
        TaskField::Number4,
        TaskField::Number5,
        TaskField::Number6,
        TaskField::Number7,
        TaskField::Number8,
        TaskField::Number9,
        TaskField::Number10,
        TaskField::Number11,
        TaskField::Number12,
        TaskField::Number13,
        TaskField::Number14,
        TaskField::Number15,
        TaskField::Number16,
        TaskField::Number17,
        TaskField::Number18,
        TaskField::Number19,
        TaskField::Number20,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.CUSTOM_START`
fn custom_start_fields() -> &'static [TaskField] {
    &[
        TaskField::Start1,
        TaskField::Start2,
        TaskField::Start3,
        TaskField::Start4,
        TaskField::Start5,
        TaskField::Start6,
        TaskField::Start7,
        TaskField::Start8,
        TaskField::Start9,
        TaskField::Start10,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.CUSTOM_TEXT`
fn custom_text_fields() -> &'static [TaskField] {
    &[
        TaskField::Text1,
        TaskField::Text2,
        TaskField::Text3,
        TaskField::Text4,
        TaskField::Text5,
        TaskField::Text6,
        TaskField::Text7,
        TaskField::Text8,
        TaskField::Text9,
        TaskField::Text10,
        TaskField::Text11,
        TaskField::Text12,
        TaskField::Text13,
        TaskField::Text14,
        TaskField::Text15,
        TaskField::Text16,
        TaskField::Text17,
        TaskField::Text18,
        TaskField::Text19,
        TaskField::Text20,
        TaskField::Text21,
        TaskField::Text22,
        TaskField::Text23,
        TaskField::Text24,
        TaskField::Text25,
        TaskField::Text26,
        TaskField::Text27,
        TaskField::Text28,
        TaskField::Text29,
        TaskField::Text30,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.CUSTOM_OUTLINE_CODE`
fn custom_outline_code_fields() -> &'static [TaskField] {
    &[
        TaskField::OutlineCode1,
        TaskField::OutlineCode2,
        TaskField::OutlineCode3,
        TaskField::OutlineCode4,
        TaskField::OutlineCode5,
        TaskField::OutlineCode6,
        TaskField::OutlineCode7,
        TaskField::OutlineCode8,
        TaskField::OutlineCode9,
        TaskField::OutlineCode10,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.ENTERPRISE_CUSTOM_COST`
fn enterprise_cost_fields() -> &'static [TaskField] {
    &[
        TaskField::EnterpriseCost1,
        TaskField::EnterpriseCost2,
        TaskField::EnterpriseCost3,
        TaskField::EnterpriseCost4,
        TaskField::EnterpriseCost5,
        TaskField::EnterpriseCost6,
        TaskField::EnterpriseCost7,
        TaskField::EnterpriseCost8,
        TaskField::EnterpriseCost9,
        TaskField::EnterpriseCost10,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.ENTERPRISE_CUSTOM_DATE`
fn enterprise_date_fields() -> &'static [TaskField] {
    &[
        TaskField::EnterpriseDate1,
        TaskField::EnterpriseDate2,
        TaskField::EnterpriseDate3,
        TaskField::EnterpriseDate4,
        TaskField::EnterpriseDate5,
        TaskField::EnterpriseDate6,
        TaskField::EnterpriseDate7,
        TaskField::EnterpriseDate8,
        TaskField::EnterpriseDate9,
        TaskField::EnterpriseDate10,
        TaskField::EnterpriseDate11,
        TaskField::EnterpriseDate12,
        TaskField::EnterpriseDate13,
        TaskField::EnterpriseDate14,
        TaskField::EnterpriseDate15,
        TaskField::EnterpriseDate16,
        TaskField::EnterpriseDate17,
        TaskField::EnterpriseDate18,
        TaskField::EnterpriseDate19,
        TaskField::EnterpriseDate20,
        TaskField::EnterpriseDate21,
        TaskField::EnterpriseDate22,
        TaskField::EnterpriseDate23,
        TaskField::EnterpriseDate24,
        TaskField::EnterpriseDate25,
        TaskField::EnterpriseDate26,
        TaskField::EnterpriseDate27,
        TaskField::EnterpriseDate28,
        TaskField::EnterpriseDate29,
        TaskField::EnterpriseDate30,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.ENTERPRISE_CUSTOM_DURATION`
fn enterprise_duration_fields() -> &'static [TaskField] {
    &[
        TaskField::EnterpriseDuration1,
        TaskField::EnterpriseDuration2,
        TaskField::EnterpriseDuration3,
        TaskField::EnterpriseDuration4,
        TaskField::EnterpriseDuration5,
        TaskField::EnterpriseDuration6,
        TaskField::EnterpriseDuration7,
        TaskField::EnterpriseDuration8,
        TaskField::EnterpriseDuration9,
        TaskField::EnterpriseDuration10,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.ENTERPRISE_CUSTOM_FLAG`
fn enterprise_flag_fields() -> &'static [TaskField] {
    &[
        TaskField::EnterpriseFlag1,
        TaskField::EnterpriseFlag2,
        TaskField::EnterpriseFlag3,
        TaskField::EnterpriseFlag4,
        TaskField::EnterpriseFlag5,
        TaskField::EnterpriseFlag6,
        TaskField::EnterpriseFlag7,
        TaskField::EnterpriseFlag8,
        TaskField::EnterpriseFlag9,
        TaskField::EnterpriseFlag10,
        TaskField::EnterpriseFlag11,
        TaskField::EnterpriseFlag12,
        TaskField::EnterpriseFlag13,
        TaskField::EnterpriseFlag14,
        TaskField::EnterpriseFlag15,
        TaskField::EnterpriseFlag16,
        TaskField::EnterpriseFlag17,
        TaskField::EnterpriseFlag18,
        TaskField::EnterpriseFlag19,
        TaskField::EnterpriseFlag20,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.ENTERPRISE_CUSTOM_NUMBER`
fn enterprise_number_fields() -> &'static [TaskField] {
    &[
        TaskField::EnterpriseNumber1,
        TaskField::EnterpriseNumber2,
        TaskField::EnterpriseNumber3,
        TaskField::EnterpriseNumber4,
        TaskField::EnterpriseNumber5,
        TaskField::EnterpriseNumber6,
        TaskField::EnterpriseNumber7,
        TaskField::EnterpriseNumber8,
        TaskField::EnterpriseNumber9,
        TaskField::EnterpriseNumber10,
        TaskField::EnterpriseNumber11,
        TaskField::EnterpriseNumber12,
        TaskField::EnterpriseNumber13,
        TaskField::EnterpriseNumber14,
        TaskField::EnterpriseNumber15,
        TaskField::EnterpriseNumber16,
        TaskField::EnterpriseNumber17,
        TaskField::EnterpriseNumber18,
        TaskField::EnterpriseNumber19,
        TaskField::EnterpriseNumber20,
        TaskField::EnterpriseNumber21,
        TaskField::EnterpriseNumber22,
        TaskField::EnterpriseNumber23,
        TaskField::EnterpriseNumber24,
        TaskField::EnterpriseNumber25,
        TaskField::EnterpriseNumber26,
        TaskField::EnterpriseNumber27,
        TaskField::EnterpriseNumber28,
        TaskField::EnterpriseNumber29,
        TaskField::EnterpriseNumber30,
        TaskField::EnterpriseNumber31,
        TaskField::EnterpriseNumber32,
        TaskField::EnterpriseNumber33,
        TaskField::EnterpriseNumber34,
        TaskField::EnterpriseNumber35,
        TaskField::EnterpriseNumber36,
        TaskField::EnterpriseNumber37,
        TaskField::EnterpriseNumber38,
        TaskField::EnterpriseNumber39,
        TaskField::EnterpriseNumber40,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.ENTERPRISE_CUSTOM_TEXT`
fn enterprise_text_fields() -> &'static [TaskField] {
    &[
        TaskField::EnterpriseText1,
        TaskField::EnterpriseText2,
        TaskField::EnterpriseText3,
        TaskField::EnterpriseText4,
        TaskField::EnterpriseText5,
        TaskField::EnterpriseText6,
        TaskField::EnterpriseText7,
        TaskField::EnterpriseText8,
        TaskField::EnterpriseText9,
        TaskField::EnterpriseText10,
        TaskField::EnterpriseText11,
        TaskField::EnterpriseText12,
        TaskField::EnterpriseText13,
        TaskField::EnterpriseText14,
        TaskField::EnterpriseText15,
        TaskField::EnterpriseText16,
        TaskField::EnterpriseText17,
        TaskField::EnterpriseText18,
        TaskField::EnterpriseText19,
        TaskField::EnterpriseText20,
        TaskField::EnterpriseText21,
        TaskField::EnterpriseText22,
        TaskField::EnterpriseText23,
        TaskField::EnterpriseText24,
        TaskField::EnterpriseText25,
        TaskField::EnterpriseText26,
        TaskField::EnterpriseText27,
        TaskField::EnterpriseText28,
        TaskField::EnterpriseText29,
        TaskField::EnterpriseText30,
        TaskField::EnterpriseText31,
        TaskField::EnterpriseText32,
        TaskField::EnterpriseText33,
        TaskField::EnterpriseText34,
        TaskField::EnterpriseText35,
        TaskField::EnterpriseText36,
        TaskField::EnterpriseText37,
        TaskField::EnterpriseText38,
        TaskField::EnterpriseText39,
        TaskField::EnterpriseText40,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.BASELINE_COSTS`
fn baseline_cost_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1Cost,
        TaskField::Baseline2Cost,
        TaskField::Baseline3Cost,
        TaskField::Baseline4Cost,
        TaskField::Baseline5Cost,
        TaskField::Baseline6Cost,
        TaskField::Baseline7Cost,
        TaskField::Baseline8Cost,
        TaskField::Baseline9Cost,
        TaskField::Baseline10Cost,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.BASELINE_DURATIONS`
fn baseline_duration_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1Duration,
        TaskField::Baseline2Duration,
        TaskField::Baseline3Duration,
        TaskField::Baseline4Duration,
        TaskField::Baseline5Duration,
        TaskField::Baseline6Duration,
        TaskField::Baseline7Duration,
        TaskField::Baseline8Duration,
        TaskField::Baseline9Duration,
        TaskField::Baseline10Duration,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.BASELINE_ESTIMATED_DURATIONS`
fn baseline_estimated_duration_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1EstimatedDuration,
        TaskField::Baseline2EstimatedDuration,
        TaskField::Baseline3EstimatedDuration,
        TaskField::Baseline4EstimatedDuration,
        TaskField::Baseline5EstimatedDuration,
        TaskField::Baseline6EstimatedDuration,
        TaskField::Baseline7EstimatedDuration,
        TaskField::Baseline8EstimatedDuration,
        TaskField::Baseline9EstimatedDuration,
        TaskField::Baseline10EstimatedDuration,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.BASELINE_STARTS`
fn baseline_start_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1Start,
        TaskField::Baseline2Start,
        TaskField::Baseline3Start,
        TaskField::Baseline4Start,
        TaskField::Baseline5Start,
        TaskField::Baseline6Start,
        TaskField::Baseline7Start,
        TaskField::Baseline8Start,
        TaskField::Baseline9Start,
        TaskField::Baseline10Start,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.BASELINE_ESTIMATED_STARTS`
fn baseline_estimated_start_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1EstimatedStart,
        TaskField::Baseline2EstimatedStart,
        TaskField::Baseline3EstimatedStart,
        TaskField::Baseline4EstimatedStart,
        TaskField::Baseline5EstimatedStart,
        TaskField::Baseline6EstimatedStart,
        TaskField::Baseline7EstimatedStart,
        TaskField::Baseline8EstimatedStart,
        TaskField::Baseline9EstimatedStart,
        TaskField::Baseline10EstimatedStart,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.BASELINE_FINISHES`
fn baseline_finish_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1Finish,
        TaskField::Baseline2Finish,
        TaskField::Baseline3Finish,
        TaskField::Baseline4Finish,
        TaskField::Baseline5Finish,
        TaskField::Baseline6Finish,
        TaskField::Baseline7Finish,
        TaskField::Baseline8Finish,
        TaskField::Baseline9Finish,
        TaskField::Baseline10Finish,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.BASELINE_ESTIMATED_FINISHES`
fn baseline_estimated_finish_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1EstimatedFinish,
        TaskField::Baseline2EstimatedFinish,
        TaskField::Baseline3EstimatedFinish,
        TaskField::Baseline4EstimatedFinish,
        TaskField::Baseline5EstimatedFinish,
        TaskField::Baseline6EstimatedFinish,
        TaskField::Baseline7EstimatedFinish,
        TaskField::Baseline8EstimatedFinish,
        TaskField::Baseline9EstimatedFinish,
        TaskField::Baseline10EstimatedFinish,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.BASELINE_WORKS`
fn baseline_work_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1Work,
        TaskField::Baseline2Work,
        TaskField::Baseline3Work,
        TaskField::Baseline4Work,
        TaskField::Baseline5Work,
        TaskField::Baseline6Work,
        TaskField::Baseline7Work,
        TaskField::Baseline8Work,
        TaskField::Baseline9Work,
        TaskField::Baseline10Work,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.BASELINE_FIXED_COSTS`
fn baseline_fixed_cost_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1FixedCost,
        TaskField::Baseline2FixedCost,
        TaskField::Baseline3FixedCost,
        TaskField::Baseline4FixedCost,
        TaskField::Baseline5FixedCost,
        TaskField::Baseline6FixedCost,
        TaskField::Baseline7FixedCost,
        TaskField::Baseline8FixedCost,
        TaskField::Baseline9FixedCost,
        TaskField::Baseline10FixedCost,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.BASELINE_FIXED_COST_ACCRUALS`
fn baseline_fixed_cost_accrual_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1FixedCostAccrual,
        TaskField::Baseline2FixedCostAccrual,
        TaskField::Baseline3FixedCostAccrual,
        TaskField::Baseline4FixedCostAccrual,
        TaskField::Baseline5FixedCostAccrual,
        TaskField::Baseline6FixedCostAccrual,
        TaskField::Baseline7FixedCostAccrual,
        TaskField::Baseline8FixedCostAccrual,
        TaskField::Baseline9FixedCostAccrual,
        TaskField::Baseline10FixedCostAccrual,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.BASELINE_BUDGET_COSTS`
fn baseline_budget_cost_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1BudgetCost,
        TaskField::Baseline2BudgetCost,
        TaskField::Baseline3BudgetCost,
        TaskField::Baseline4BudgetCost,
        TaskField::Baseline5BudgetCost,
        TaskField::Baseline6BudgetCost,
        TaskField::Baseline7BudgetCost,
        TaskField::Baseline8BudgetCost,
        TaskField::Baseline9BudgetCost,
        TaskField::Baseline10BudgetCost,
    ]
}

/// 对应 Java: `org.mpxj.common.TaskFieldLists.BASELINE_BUDGET_WORKS`
fn baseline_budget_work_fields() -> &'static [TaskField] {
    &[
        TaskField::Baseline1BudgetWork,
        TaskField::Baseline2BudgetWork,
        TaskField::Baseline3BudgetWork,
        TaskField::Baseline4BudgetWork,
        TaskField::Baseline5BudgetWork,
        TaskField::Baseline6BudgetWork,
        TaskField::Baseline7BudgetWork,
        TaskField::Baseline8BudgetWork,
        TaskField::Baseline9BudgetWork,
        TaskField::Baseline10BudgetWork,
    ]
}

// ---------------------------------------------------------------------------
// 枚举 / 类型编码辅助（见文件头差异第 7 条）
// ---------------------------------------------------------------------------

/// 把 `TaskType` 编码为声明序号（Java 侧该枚举没有 int 值）。
fn task_type_to_value(value: TaskType) -> i32 {
    TaskType::ALL
        .iter()
        .position(|item| *item == value)
        .map_or(0, |index| index as i32)
}

/// 把字段中的 int 还原为 `TaskType`。
fn task_type_from_value(value: i32) -> Option<TaskType> {
    TaskType::get_instance(value)
}

/// 把字段中的 int 还原为 `ConstraintType`（越界回落 `AS_SOON_AS_POSSIBLE`，与 Java 一致）。
fn constraint_type_from_value(value: i32) -> ConstraintType {
    ConstraintType::get_instance(value)
}

// ---------------------------------------------------------------------------
// 自动生成 WBS / 大纲编号
// ---------------------------------------------------------------------------

impl Task {
    /// 自动生成 WBS 字段取值。
    ///
    /// 对应 Java: `public void generateWBS(Task parent)`
    ///
    /// Java 在 `parent.getWBS()` 为 `null` 时会抛 `NullPointerException`；
    /// Rust 侧按空串处理（不 panic）。
    pub fn generate_wbs(&mut self, parent: Option<&Rc<RefCell<Task>>>) {
        let wbs = match parent {
            None => {
                if NumberHelper::get_int(self.get_unique_id().map(f64::from)) == 0 {
                    "0".to_owned()
                } else {
                    let count = self
                        .get_parent_file()
                        .map_or(0, |file| file.borrow().get_child_tasks().len());
                    (count + 1).to_string()
                }
            }
            Some(parent) => {
                let mut wbs = parent.borrow().get_wbs().unwrap_or_default();
                let child_task_count = parent.borrow().get_child_tasks().len() + 1;
                if wbs == "0" {
                    wbs = child_task_count.to_string();
                } else {
                    wbs.push('.');
                    wbs.push_str(&child_task_count.to_string());
                }
                wbs
            }
        };

        self.set_wbs(Some(wbs));
    }

    /// 自动生成大纲编号字段取值。
    ///
    /// 对应 Java: `public void generateOutlineNumber(Task parent)`
    pub fn generate_outline_number(&mut self, parent: Option<&Rc<RefCell<Task>>>) {
        let outline = match parent {
            None => {
                if NumberHelper::get_int(self.get_unique_id().map(f64::from)) == 0 {
                    "0".to_owned()
                } else {
                    let count = self
                        .get_parent_file()
                        .map_or(0, |file| file.borrow().get_child_tasks().len());
                    (count + 1).to_string()
                }
            }
            Some(parent) => {
                let mut outline = parent.borrow().get_outline_number().unwrap_or_default();
                if let Some(index) = outline.rfind(".0") {
                    outline.truncate(index);
                }

                let child_task_count = parent.borrow().get_child_tasks().len() + 1;
                if outline == "0" {
                    outline = child_task_count.to_string();
                } else {
                    outline.push('.');
                    outline.push_str(&child_task_count.to_string());
                }
                outline
            }
        };

        self.set_outline_number(Some(outline));
    }
}

// ---------------------------------------------------------------------------
// Notes
// ---------------------------------------------------------------------------

impl Task {
    /// 为当前任务添加纯文本笔记。
    ///
    /// 对应 Java: `public void setNotes(String notes)`
    ///
    /// Java 会包装成 `new Notes(notes)` 后存入字段表；Rust 侧 `Notes` 是 trait，
    /// 字段值只保留纯文本（见文件头差异第 7 条），读取时以 [`PlainNotes`] 还原。
    pub fn set_notes(&mut self, notes: Option<String>) {
        self.put_string(TaskField::Notes, notes);
    }

    /// 设置代表任务笔记的 `Notes` 实例。
    ///
    /// 对应 Java: `public void setNotesObject(Notes notes)`
    ///
    /// `Notes` 在 Rust 侧是 trait 对象，`FieldValue` 无法承载；此处只保存其纯文本
    /// 表示，`RtfNotes` / `HtmlNotes` / `StructuredNotes` 的具体子类型信息丢失
    /// （见文件头「跳过的 Java 方法」最后一条）。
    pub fn set_notes_object(&mut self, notes: Option<&dyn Notes>) {
        self.put_string(TaskField::Notes, notes.map(|notes| notes.to_string()));
    }
}

// ---------------------------------------------------------------------------
// 层级结构（父子任务）
// ---------------------------------------------------------------------------

impl Task {
    /// 新增一个子任务（WBS 自动补全）。
    ///
    /// 对应 Java: `@Override public Task addTask()`
    ///
    /// Java 的实例方法访问了隐含的 `this`，Rust 侧需要实体自身的共享句柄，
    /// 故改为以 `this` 为首参的关联函数（见文件头说明）。
    pub fn add_task(this: &Rc<RefCell<Task>>) -> Option<Rc<RefCell<Task>>> {
        let file = this.borrow().parent_file.upgrade()?;

        let task = Rc::new(RefCell::new(Task::new_in_file(&file, Some(this))));

        this.borrow_mut().children.push(Rc::clone(&task));
        file.borrow_mut().get_tasks_mut().add_task(Rc::clone(&task));
        this.borrow_mut().set_summary(true);

        Some(task)
    }

    /// 按大纲级别把子任务挂到当前任务（读取 MPX 文件时构建层级用）。
    ///
    /// 对应 Java: `public void addChildTask(Task child, int childOutlineLevel)`
    pub fn add_child_task_at_level(
        this: &Rc<RefCell<Task>>,
        child: &Rc<RefCell<Task>>,
        child_outline_level: i32,
    ) {
        let outline_level = NumberHelper::get_int(this.borrow().get_outline_level().map(f64::from));

        let last_child = this.borrow().children.last().cloned();

        if (outline_level + 1) == child_outline_level {
            this.borrow_mut().children.push(Rc::clone(child));
            this.borrow_mut().set_summary(true);
        } else if let Some(last_child) = last_child {
            Task::add_child_task_at_level(&last_child, child, child_outline_level);
        }
    }

    /// 关联一个子任务，并按需自动设置子任务的大纲级别。
    ///
    /// 对应 Java: `public void addChildTask(Task child)`
    pub fn add_child_task(this: &Rc<RefCell<Task>>, child: &Rc<RefCell<Task>>) {
        child.borrow_mut().parent = Rc::downgrade(this);
        this.borrow_mut().children.push(Rc::clone(child));
        this.borrow_mut().set_summary(true);

        let auto_outline_level = this
            .borrow()
            .parent_file
            .upgrade()
            .map_or(false, |file| {
                file.borrow().get_project_config().get_auto_outline_level()
            });

        if auto_outline_level {
            let outline_level =
                NumberHelper::get_int(this.borrow().get_outline_level().map(f64::from)) + 1;
            child.borrow_mut().set_outline_level(Some(outline_level));
        }
    }

    /// 在指定兄弟任务之前插入一个子任务。
    ///
    /// 对应 Java: `public void addChildTaskBefore(Task child, Task previousSibling)`
    pub fn add_child_task_before(
        this: &Rc<RefCell<Task>>,
        child: &Rc<RefCell<Task>>,
        previous_sibling: &Rc<RefCell<Task>>,
    ) {
        let index = this
            .borrow()
            .children
            .iter()
            .position(|item| Rc::ptr_eq(item, previous_sibling));

        match index {
            None => this.borrow_mut().children.push(Rc::clone(child)),
            Some(index) => this.borrow_mut().children.insert(index, Rc::clone(child)),
        }

        child.borrow_mut().parent = Rc::downgrade(this);
        this.borrow_mut().set_summary(true);

        let auto_outline_level = this
            .borrow()
            .parent_file
            .upgrade()
            .map_or(false, |file| {
                file.borrow().get_project_config().get_auto_outline_level()
            });

        if auto_outline_level {
            let outline_level =
                NumberHelper::get_int(this.borrow().get_outline_level().map(f64::from)) + 1;
            child.borrow_mut().set_outline_level(Some(outline_level));
        }
    }

    /// 移除一个子任务。
    ///
    /// 对应 Java: `public void removeChildTask(Task child)`
    pub fn remove_child_task(this: &Rc<RefCell<Task>>, child: &Rc<RefCell<Task>>) {
        let removed = {
            let mut task = this.borrow_mut();
            match task
                .children
                .iter()
                .position(|item| Rc::ptr_eq(item, child))
            {
                Some(index) => {
                    task.children.remove(index);
                    true
                }
                None => false,
            }
        };

        if removed {
            child.borrow_mut().parent = Weak::new();
        }

        let is_empty = this.borrow().children.is_empty();
        this.borrow_mut().set_summary(!is_empty);
    }

    /// 清空子任务列表（为构建层级结构做准备）。
    ///
    /// 对应 Java: `public void clearChildTasks()`
    ///
    /// 仅在列表非空时动作，以保证 `summary` 标志在「无子活动的 WBS 条目」场景下不被改写。
    pub fn clear_child_tasks(this: &Rc<RefCell<Task>>) {
        if this.borrow().children.is_empty() {
            return;
        }

        this.borrow_mut().children.clear();
        this.borrow_mut().set_summary(false);
    }

    /// 取父任务（弱引用升级；无父任务或父任务已释放时返回 `None`）。
    ///
    /// 对应 Java: `public Task getParentTask()`
    pub fn get_parent_task(&self) -> Option<Rc<RefCell<Task>>> {
        self.parent.upgrade()
    }

    /// 取子任务列表（返回共享句柄的克隆快照）。
    ///
    /// 对应 Java: `@Override public List<Task> getChildTasks()`
    pub fn get_child_tasks(&self) -> Vec<Rc<RefCell<Task>>> {
        self.children.clone()
    }

    /// 判断任务是否有子任务。
    ///
    /// 对应 Java: `public boolean hasChildTasks()`
    pub fn has_child_tasks(&self) -> bool {
        !self.children.is_empty()
    }

    /// 从项目文件中移除本任务。
    ///
    /// 对应 Java: `public void remove()`
    pub fn remove(this: &Rc<RefCell<Task>>) {
        if let Some(file) = this.borrow().parent_file.upgrade() {
            file.borrow_mut().remove_task(this);
        }
    }
}

// ---------------------------------------------------------------------------
// 资源分配与关系（Relation）
// ---------------------------------------------------------------------------

impl Task {
    /// 为当前任务新增一个资源分配。
    ///
    /// 对应 Java: `public ResourceAssignment addResourceAssignment(Resource resource)`
    pub fn add_resource_assignment(
        &mut self,
        resource: Option<&Rc<RefCell<Resource>>>,
    ) -> Option<Rc<RefCell<ResourceAssignment>>> {
        let file = self.parent_file.upgrade()?;

        let assignment = Rc::new(RefCell::new(ResourceAssignment::with_parent_file(&file)));
        {
            let mut assignment_ref = assignment.borrow_mut();
            assignment_ref.set_task_unique_id(self.get_unique_id());
            assignment_ref.set_resource_unique_id(
                resource.and_then(|resource| resource.borrow().get_unique_id()),
            );
            assignment_ref.set_work(self.get_duration());
            // Java: assignment.setUnits(ResourceAssignment.DEFAULT_UNITS)
            // （`crate::resource_assignment` 尚未提供该常量，此处直接使用其值 1.0）
            assignment_ref.set_units(Some(1.0));
        }

        file.borrow_mut()
            .get_resource_assignments_mut()
            .add(Rc::clone(&assignment));

        Some(assignment)
    }

    /// 取已存在的资源分配，避免重复添加。
    ///
    /// 对应 Java: `public ResourceAssignment getExistingResourceAssignment(Resource resource)`
    pub fn get_existing_resource_assignment(
        &self,
        resource: Option<&Rc<RefCell<Resource>>>,
    ) -> Option<Rc<RefCell<ResourceAssignment>>> {
        let resource_unique_id = resource.and_then(|resource| resource.borrow().get_unique_id());

        self.get_resource_assignments().into_iter().find(|assignment| {
            let assignment = assignment.borrow();
            match (resource, assignment.get_resource()) {
                (None, None) => true,
                (Some(_), Some(_)) => {
                    resource_unique_id == assignment.get_resource_unique_id()
                }
                _ => false,
            }
        })
    }

    /// 取当前任务的全部资源分配。
    ///
    /// 对应 Java: `public List<ResourceAssignment> getResourceAssignments()`
    pub fn get_resource_assignments(&self) -> Vec<Rc<RefCell<ResourceAssignment>>> {
        match self.parent_file.upgrade() {
            Some(file) => file
                .borrow()
                .get_resource_assignments()
                .get_by_task_unique_id(self.get_unique_id()),
            None => Vec::new(),
        }
    }

    /// 为当前任务新增一个前置关系。
    ///
    /// 对应 Java: `public Relation addPredecessor(Relation.Builder builder)`
    ///
    /// Java 在方法体内使用隐含的 `this`（`builder.successorTask(this)`），Rust 侧
    /// 需要实体自身的共享句柄，故改为以 `this` 为首参的关联函数。
    pub fn add_predecessor(
        this: &Rc<RefCell<Task>>,
        builder: RelationBuilder,
    ) -> Option<Rc<RefCell<Relation>>> {
        let file = this.borrow().parent_file.upgrade()?;
        let builder = builder.successor_task(this);
        Some(file.borrow_mut().get_relations_mut().add_predecessor(builder))
    }

    /// 删除与给定目标任务、关系类型与延隔时间完全匹配的前置关系。
    ///
    /// 对应 Java: `public boolean removePredecessor(Task targetTask, RelationType type, Duration lag)`
    pub fn remove_predecessor(
        &self,
        target_task: Option<&Rc<RefCell<Task>>>,
        relation_type: RelationType,
        lag: Option<Duration>,
    ) -> bool {
        let Some(file) = self.parent_file.upgrade() else {
            return false;
        };

        let Some(target_task) = target_task else {
            return false;
        };
        let target_task = target_task.borrow();

        file.borrow_mut().get_relations_mut().remove_predecessor(
            self,
            &target_task,
            relation_type,
            lag,
        )
    }

    /// 取当前任务的前置关系列表。
    ///
    /// 对应 Java: `@SuppressWarnings("unchecked") public List<Relation> getPredecessors()`
    ///
    /// Java 经字段表 `TaskField.PREDECESSORS` 调 `calculatePredecessors()` 求值
    /// （该字段属于「始终计算」集合，不写回缓存）；Rust 侧直接向关系容器查询，
    /// 以绕开字段表缺少 `List<Relation>` 变体的问题（见文件头差异第 7 条）。
    pub fn get_predecessors(&self) -> Vec<Rc<RefCell<Relation>>> {
        self.calculate_predecessors()
    }

    /// 取当前任务的后继关系列表。
    ///
    /// 对应 Java: `@SuppressWarnings("unchecked") public List<Relation> getSuccessors()`
    pub fn get_successors(&self) -> Vec<Rc<RefCell<Relation>>> {
        self.calculate_successors()
    }

    /// 计算前置关系列表。
    ///
    /// 对应 Java: `private List<Relation> calculatePredecessors()`
    fn calculate_predecessors(&self) -> Vec<Rc<RefCell<Relation>>> {
        match self.parent_file.upgrade() {
            Some(file) => file.borrow().get_relations().get_predecessors(self),
            None => Vec::new(),
        }
    }

    /// 计算后继关系列表。
    ///
    /// 对应 Java: `private List<Relation> calculateSuccessors()`
    fn calculate_successors(&self) -> Vec<Rc<RefCell<Relation>>> {
        match self.parent_file.upgrade() {
            Some(file) => file.borrow().get_relations().get_successors(self),
            None => Vec::new(),
        }
    }

    /// 判断给定任务是否是当前任务的前置任务。
    ///
    /// 对应 Java: `public boolean isPredecessor(Task task)`
    pub fn is_predecessor(&self, task: Option<&Rc<RefCell<Task>>>) -> bool {
        let Some(task) = task else {
            return false;
        };

        let unique_id = task.borrow().get_unique_id();
        self.get_predecessors().iter().any(|relation| {
            relation
                .borrow()
                .get_predecessor_task()
                .map(|task| task.borrow().get_unique_id())
                == Some(unique_id)
        })
    }

    /// 判断给定任务是否是当前任务的后继任务。
    ///
    /// 对应 Java: `public boolean isSuccessor(Task task)`
    ///
    /// Java 的实现是 `task.isPredecessor(this)`；Rust 侧改为在本任务的后继列表中
    /// 反查（语义等价，且无需实体自身的共享句柄）。
    pub fn is_successor(&self, task: Option<&Rc<RefCell<Task>>>) -> bool {
        let Some(task) = task else {
            return false;
        };

        let unique_id = task.borrow().get_unique_id();
        self.get_successors().iter().any(|relation| {
            relation
                .borrow()
                .get_successor_task()
                .map(|task| task.borrow().get_unique_id())
                == Some(unique_id)
        })
    }
}

// ---------------------------------------------------------------------------
// 基本属性：百分比、成本、工期、日期
// ---------------------------------------------------------------------------

impl Task {
    /// 设置完成百分比。
    ///
    /// 对应 Java: `public void setPercentageComplete(Number val)`
    pub fn set_percentage_complete(&mut self, val: Option<f64>) {
        self.put_number(TaskField::PercentComplete, val);
    }

    /// 设置工时完成百分比。
    ///
    /// 对应 Java: `public void setPercentageWorkComplete(Number val)`
    pub fn set_percentage_work_complete(&mut self, val: Option<f64>) {
        self.put_number(TaskField::PercentWorkComplete, val);
    }

    /// 设置实际成本。
    ///
    /// 对应 Java: `public void setActualCost(Number val)`
    pub fn set_actual_cost(&mut self, val: Option<f64>) {
        self.put_number(TaskField::ActualCost, val);
    }

    /// 设置实际工期。
    ///
    /// 对应 Java: `public void setActualDuration(Duration val)`
    pub fn set_actual_duration(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::ActualDuration, val);
    }

    /// 设置实际完成日期。
    ///
    /// 对应 Java: `public void setActualFinish(LocalDateTime val)`
    pub fn set_actual_finish(&mut self, val: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::ActualFinish, val);
    }

    /// 设置实际开始日期。
    ///
    /// 对应 Java: `public void setActualStart(LocalDateTime val)`
    pub fn set_actual_start(&mut self, val: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::ActualStart, val);
    }

    /// 设置实际工时。
    ///
    /// 对应 Java: `public void setActualWork(Duration val)`
    pub fn set_actual_work(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::ActualWork, val);
    }

    /// 设置基线成本。
    ///
    /// 对应 Java: `public void setBaselineCost(Number val)`
    pub fn set_baseline_cost(&mut self, val: Option<f64>) {
        self.put_number(TaskField::BaselineCost, val);
    }

    /// 设置基线工期。
    ///
    /// 对应 Java: `public void setBaselineDuration(Duration val)`
    pub fn set_baseline_duration(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::BaselineDuration, val);
    }

    /// 设置基线完成日期。
    ///
    /// 对应 Java: `public void setBaselineFinish(LocalDateTime val)`
    pub fn set_baseline_finish(&mut self, val: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::BaselineFinish, val);
    }

    /// 设置基线开始日期。
    ///
    /// 对应 Java: `public void setBaselineStart(LocalDateTime val)`
    pub fn set_baseline_start(&mut self, val: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::BaselineStart, val);
    }

    /// 设置基线工时。
    ///
    /// 对应 Java: `public void setBaselineWork(Duration val)`
    pub fn set_baseline_work(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::BaselineWork, val);
    }

    /// 设置 BCWP（已完成工作预算成本）。
    ///
    /// 对应 Java: `public void setBCWP(Number val)`
    pub fn set_bcwp(&mut self, val: Option<f64>) {
        self.put_number(TaskField::Bcwp, val);
    }

    /// 设置 BCWS（计划工作预算成本）。
    ///
    /// 对应 Java: `public void setBCWS(Number val)`
    pub fn set_bcws(&mut self, val: Option<f64>) {
        self.put_number(TaskField::Bcws, val);
    }

    /// 设置「已确认」标志。
    ///
    /// 对应 Java: `public void setConfirmed(boolean val)`
    pub fn set_confirmed(&mut self, val: bool) {
        self.put_bool(TaskField::Confirmed, val);
    }

    /// 设置约束日期。
    ///
    /// 对应 Java: `public void setConstraintDate(LocalDateTime val)`
    pub fn set_constraint_date(&mut self, val: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::ConstraintDate, val);
    }

    /// 设置次要约束日期。
    ///
    /// 对应 Java: `public void setSecondaryConstraintDate(LocalDateTime date)`
    pub fn set_secondary_constraint_date(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::SecondaryConstraintDate, date);
    }

    /// 设置约束类型。
    ///
    /// 对应 Java: `public void setConstraintType(ConstraintType type)`
    pub fn set_constraint_type(&mut self, r#type: Option<ConstraintType>) {
        self.put_enum(TaskField::ConstraintType, r#type.map(|value| value.value()));
    }

    /// 设置次要约束类型。
    ///
    /// 对应 Java: `public void setSecondaryConstraintType(ConstraintType type)`
    pub fn set_secondary_constraint_type(&mut self, r#type: Option<ConstraintType>) {
        self.put_enum(TaskField::SecondaryConstraintType, r#type.map(|value| value.value()));
    }

    /// 设置联系人。
    ///
    /// 对应 Java: `public void setContact(String val)`
    pub fn set_contact(&mut self, val: Option<String>) {
        self.put_string(TaskField::Contact, val);
    }

    /// 设置总成本。
    ///
    /// 对应 Java: `public void setCost(Number val)`
    pub fn set_cost(&mut self, val: Option<f64>) {
        self.put_number(TaskField::Cost, val);
    }

    /// 设置自定义成本字段。
    ///
    /// 对应 Java: `public void setCost(int index, Number value)`
    pub fn set_cost_at(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = select_field(custom_cost_fields(), index) {
            self.put_number(field, value);
        }
    }

    /// 读取自定义成本字段。
    ///
    /// 对应 Java: `public Number getCost(int index)`
    pub fn get_cost_at(&self, index: i32) -> Option<f64> {
        select_field(custom_cost_fields(), index).and_then(|field| self.number_field(field))
    }

    /// 设置成本偏差。
    ///
    /// 对应 Java: `public void setCostVariance(Number val)`
    pub fn set_cost_variance(&mut self, val: Option<f64>) {
        self.put_number(TaskField::CostVariance, val);
    }

    /// 设置创建日期。
    ///
    /// 对应 Java: `public void setCreateDate(LocalDateTime val)`
    pub fn set_create_date(&mut self, val: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::Created, val);
    }

    /// 设置关键路径标志。
    ///
    /// 对应 Java: `public void setCritical(boolean val)`
    pub fn set_critical(&mut self, val: bool) {
        self.put_bool(TaskField::Critical, val);
    }

    /// 设置 CV（挣值成本偏差）。
    ///
    /// 对应 Java: `public void setCV(Number val)`
    pub fn set_cv(&mut self, val: Option<f64>) {
        self.put_number(TaskField::Cv, val);
    }

    /// 设置调配延隔时间。
    ///
    /// 对应 Java: `public void setLevelingDelay(Duration val)`
    pub fn set_leveling_delay(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::LevelingDelay, val);
    }

    /// 设置工期。
    ///
    /// 对应 Java: `public void setDuration(Duration val)`
    pub fn set_duration(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::Duration, val);
    }

    /// 设置手排任务的工期文本。
    ///
    /// 对应 Java: `public void setDurationText(String val)`
    pub fn set_duration_text(&mut self, val: Option<String>) {
        self.put_string(TaskField::DurationText, val);
    }

    /// 设置手工工期属性。
    ///
    /// 对应 Java: `public void setManualDuration(Duration dur)`
    pub fn set_manual_duration(&mut self, dur: Option<Duration>) {
        self.put_duration(TaskField::ManualDuration, dur);
    }

    /// 读取手工工期属性。
    ///
    /// 对应 Java: `public Duration getManualDuration()`
    pub fn get_manual_duration(&self) -> Option<Duration> {
        self.duration_field(TaskField::ManualDuration)
    }

    /// 设置工期偏差。
    ///
    /// 对应 Java: `public void setDurationVariance(Duration duration)`
    pub fn set_duration_variance(&mut self, duration: Option<Duration>) {
        self.put_duration(TaskField::DurationVariance, duration);
    }

    /// 设置最早完成日期。
    ///
    /// 对应 Java: `public void setEarlyFinish(LocalDateTime date)`
    pub fn set_early_finish(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::EarlyFinish, date);
    }

    /// 设置剩余最早完成日期。
    ///
    /// 对应 Java: `public void setRemainingEarlyFinish(LocalDateTime date)`
    pub fn set_remaining_early_finish(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::RemainingEarlyFinish, date);
    }

    /// 设置最早开始日期。
    ///
    /// 对应 Java: `public void setEarlyStart(LocalDateTime date)`
    pub fn set_early_start(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::EarlyStart, date);
    }

    /// 设置剩余最早开始日期。
    ///
    /// 对应 Java: `public void setRemainingEarlyStart(LocalDateTime date)`
    pub fn set_remaining_early_start(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::RemainingEarlyStart, date);
    }

    /// 设置完成日期。
    ///
    /// 对应 Java: `public void setFinish(LocalDateTime date)`
    pub fn set_finish(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::Finish, date);
    }

    /// 设置手排任务的完成文本。
    ///
    /// 对应 Java: `public void setFinishText(String val)`
    pub fn set_finish_text(&mut self, val: Option<String>) {
        self.put_string(TaskField::FinishText, val);
    }

    /// 设置完成偏差。
    ///
    /// 对应 Java: `public void setFinishVariance(Duration duration)`
    pub fn set_finish_variance(&mut self, duration: Option<Duration>) {
        self.put_duration(TaskField::FinishVariance, duration);
    }

    /// 设置固定成本。
    ///
    /// 对应 Java: `public void setFixedCost(Number val)`
    pub fn set_fixed_cost(&mut self, val: Option<f64>) {
        self.put_number(TaskField::FixedCost, val);
    }

    /// 设置自由时差。
    ///
    /// 对应 Java: `public void setFreeSlack(Duration duration)`
    pub fn set_free_slack(&mut self, duration: Option<Duration>) {
        self.put_duration(TaskField::FreeSlack, duration);
    }

    /// 设置「隐藏甘特条」标志。
    ///
    /// 对应 Java: `public void setHideBar(boolean flag)`
    pub fn set_hide_bar(&mut self, flag: bool) {
        self.put_bool(TaskField::HideBar, flag);
    }

    /// 设置任务 ID。
    ///
    /// 对应 Java: `@Override public void setID(Integer val)`
    ///
    /// Java 还会调用 `TaskContainer` 的 `unmapID` / `mapID` 维护 ID 索引，
    /// 该动作需要实体自身的共享引用，Rust 侧改由容器负责（见文件头说明第 3 条）。
    pub fn set_id(&mut self, val: Option<i32>) {
        self.put_integer(TaskField::Id, val);
    }

    /// 设置最晚完成日期。
    ///
    /// 对应 Java: `public void setLateFinish(LocalDateTime date)`
    pub fn set_late_finish(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::LateFinish, date);
    }

    /// 设置剩余最晚完成日期。
    ///
    /// 对应 Java: `public void setRemainingLateFinish(LocalDateTime date)`
    pub fn set_remaining_late_finish(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::RemainingLateFinish, date);
    }

    /// 设置最晚开始日期。
    ///
    /// 对应 Java: `public void setLateStart(LocalDateTime date)`
    pub fn set_late_start(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::LateStart, date);
    }

    /// 设置剩余最晚开始日期。
    ///
    /// 对应 Java: `public void setRemainingLateStart(LocalDateTime date)`
    pub fn set_remaining_late_start(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::RemainingLateStart, date);
    }

    /// 设置「存在 OLE 链接」标志。
    ///
    /// 对应 Java: `public void setLinkedFields(boolean flag)`
    pub fn set_linked_fields(&mut self, flag: bool) {
        self.put_bool(TaskField::LinkedFields, flag);
    }

    /// 设置「标记」标志。
    ///
    /// 对应 Java: `public void setMarked(boolean flag)`
    pub fn set_marked(&mut self, flag: bool) {
        self.put_bool(TaskField::Marked, flag);
    }

    /// 设置里程碑标志。
    ///
    /// 对应 Java: `public void setMilestone(boolean flag)`
    pub fn set_milestone(&mut self, flag: bool) {
        self.put_bool(TaskField::Milestone, flag);
    }

    /// 设置任务名称。
    ///
    /// 对应 Java: `public void setName(String name)`
    pub fn set_name(&mut self, name: Option<String>) {
        self.put_string(TaskField::Name, name);
    }

    /// 设置附加对象数量。
    ///
    /// 对应 Java: `public void setObjects(Integer val)`
    pub fn set_objects(&mut self, val: Option<i32>) {
        self.put_integer(TaskField::Objects, val);
    }

    /// 设置大纲级别。
    ///
    /// 对应 Java: `public void setOutlineLevel(Integer val)`
    pub fn set_outline_level(&mut self, val: Option<i32>) {
        self.put_integer(TaskField::OutlineLevel, val);
    }

    /// 设置大纲编号。
    ///
    /// 对应 Java: `public void setOutlineNumber(String val)`
    pub fn set_outline_number(&mut self, val: Option<String>) {
        self.put_string(TaskField::OutlineNumber, val);
    }

    /// 设置优先级。
    ///
    /// 对应 Java: `public void setPriority(Priority priority)`
    pub fn set_priority(&mut self, priority: Option<Priority>) {
        self.put_enum(
            TaskField::Priority,
            priority.map(|priority| priority.get_value()),
        );
    }

    /// 设置来源项目名称。
    ///
    /// 对应 Java: `public void setProject(String val)`
    pub fn set_project(&mut self, val: Option<String>) {
        self.put_string(TaskField::Project, val);
    }

    /// 设置剩余成本。
    ///
    /// 对应 Java: `public void setRemainingCost(Number val)`
    pub fn set_remaining_cost(&mut self, val: Option<f64>) {
        self.put_number(TaskField::RemainingCost, val);
    }

    /// 设置剩余工期。
    ///
    /// 对应 Java: `public void setRemainingDuration(Duration val)`
    pub fn set_remaining_duration(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::RemainingDuration, val);
    }

    /// 设置剩余工时。
    ///
    /// 对应 Java: `public void setRemainingWork(Duration val)`
    pub fn set_remaining_work(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::RemainingWork, val);
    }

    /// 设置资源组。
    ///
    /// 对应 Java: `public void setResourceGroup(String val)`
    pub fn set_resource_group(&mut self, val: Option<String>) {
        self.put_string(TaskField::ResourceGroup, val);
    }

    /// 设置资源缩写。
    ///
    /// 对应 Java: `public void setResourceInitials(String val)`
    pub fn set_resource_initials(&mut self, val: Option<String>) {
        self.put_string(TaskField::ResourceInitials, val);
    }

    /// 设置资源名称列表。
    ///
    /// 对应 Java: `public void setResourceNames(String val)`
    pub fn set_resource_names(&mut self, val: Option<String>) {
        self.put_string(TaskField::ResourceNames, val);
    }

    /// 设置恢复日期。
    ///
    /// 对应 Java: `public void setResume(LocalDateTime val)`
    pub fn set_resume(&mut self, val: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::Resume, val);
    }

    /// 设置汇总标志。
    ///
    /// 对应 Java: `public void setRollup(boolean val)`
    pub fn set_rollup(&mut self, val: bool) {
        self.put_bool(TaskField::Rollup, val);
    }

    /// 设置开始日期。
    ///
    /// 对应 Java: `public void setStart(LocalDateTime val)`
    pub fn set_start(&mut self, val: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::Start, val);
    }

    /// 设置手排任务的开始文本。
    ///
    /// 对应 Java: `public void setStartText(String val)`
    pub fn set_start_text(&mut self, val: Option<String>) {
        self.put_string(TaskField::StartText, val);
    }

    /// 设置开始偏差。
    ///
    /// 对应 Java: `public void setStartVariance(Duration val)`
    pub fn set_start_variance(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::StartVariance, val);
    }

    /// 设置停止日期。
    ///
    /// 对应 Java: `public void setStop(LocalDateTime val)`
    pub fn set_stop(&mut self, val: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::Stop, val);
    }

    /// 设置子项目文件路径。
    ///
    /// 对应 Java: `public void setSubprojectFile(String val)`
    pub fn set_subproject_file(&mut self, val: Option<String>) {
        self.put_string(TaskField::SubprojectFile, val);
    }

    /// 设置链接子项目文件的 GUID。
    ///
    /// 对应 Java: `public void setSubprojectGUID(UUID guid)`
    pub fn set_subproject_guid(&mut self, guid: Option<Uuid>) {
        self.put_uuid(TaskField::SubprojectGuid, guid);
    }

    /// 取链接子项目文件的 GUID。
    ///
    /// 对应 Java: `public UUID getSubprojectGUID()`
    pub fn get_subproject_guid(&self) -> Option<Uuid> {
        self.uuid_field(TaskField::SubprojectGuid)
    }

    /// 设置汇总任务标志。
    ///
    /// 对应 Java: `public void setSummary(boolean val)`
    pub fn set_summary(&mut self, val: bool) {
        self.put_bool(TaskField::Summary, val);
    }

    /// 设置 SV（挣值进度偏差）。
    ///
    /// 对应 Java: `public void setSV(Number val)`
    pub fn set_sv(&mut self, val: Option<f64>) {
        self.put_number(TaskField::Sv, val);
    }

    /// 设置总时差。
    ///
    /// 对应 Java: `public void setTotalSlack(Duration val)`
    pub fn set_total_slack(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::TotalSlack, val);
    }

    /// 设置唯一 ID。
    ///
    /// 对应 Java: `@Override public void setUniqueID(Integer val)`
    ///
    /// Java 在字段变更处理中会同步 `TaskContainer` 的 unique ID 索引，
    /// Rust 侧改由容器负责（见文件头说明第 3 条）。
    pub fn set_unique_id(&mut self, val: Option<i32>) {
        self.put_integer(TaskField::UniqueId, val);
    }

    /// 设置「需要更新」标志。
    ///
    /// 对应 Java: `public void setUpdateNeeded(boolean val)`
    pub fn set_update_needed(&mut self, val: bool) {
        self.put_bool(TaskField::UpdateNeeded, val);
    }

    /// 设置 WBS 编码。
    ///
    /// 对应 Java: `public void setWBS(String val)`
    pub fn set_wbs(&mut self, val: Option<String>) {
        self.put_string(TaskField::Wbs, val);
    }

    /// 设置工时。
    ///
    /// 对应 Java: `public void setWork(Duration val)`
    pub fn set_work(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::Work, val);
    }

    /// 设置工时偏差。
    ///
    /// 对应 Java: `public void setWorkVariance(Duration val)`
    pub fn set_work_variance(&mut self, val: Option<Duration>) {
        self.put_duration(TaskField::WorkVariance, val);
    }
}

// ---------------------------------------------------------------------------
// 基本属性读取
// ---------------------------------------------------------------------------

impl Task {
    /// 取完成百分比。
    ///
    /// 对应 Java: `public Number getPercentageComplete()`
    pub fn get_percentage_complete(&self) -> Option<f64> {
        self.number_field(TaskField::PercentComplete)
    }

    /// 取工时完成百分比。
    ///
    /// 对应 Java: `public Number getPercentageWorkComplete()`
    pub fn get_percentage_work_complete(&self) -> Option<f64> {
        self.number_field(TaskField::PercentWorkComplete)
    }

    /// 取实际成本。
    ///
    /// 对应 Java: `public Number getActualCost()`
    pub fn get_actual_cost(&self) -> Option<f64> {
        self.number_field(TaskField::ActualCost)
    }

    /// 取实际工期。
    ///
    /// 对应 Java: `public Duration getActualDuration()`
    pub fn get_actual_duration(&self) -> Option<Duration> {
        self.duration_field(TaskField::ActualDuration)
    }

    /// 取实际完成日期。
    ///
    /// 对应 Java: `public LocalDateTime getActualFinish()`
    pub fn get_actual_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::ActualFinish)
    }

    /// 取实际开始日期。
    ///
    /// 对应 Java: `public LocalDateTime getActualStart()`
    pub fn get_actual_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::ActualStart)
    }

    /// 取实际工时。
    ///
    /// 对应 Java: `public Duration getActualWork()`
    pub fn get_actual_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::ActualWork)
    }

    /// 取基线成本。
    ///
    /// 对应 Java: `public Number getBaselineCost()`
    pub fn get_baseline_cost(&self) -> Option<f64> {
        self.number_field(TaskField::BaselineCost)
    }

    /// 取基线工期。
    ///
    /// 对应 Java: `public Duration getBaselineDuration()`
    pub fn get_baseline_duration(&self) -> Option<Duration> {
        self.duration_field(TaskField::BaselineDuration)
    }

    /// 取基线工期文本。
    ///
    /// 对应 Java: `public String getBaselineDurationText()`
    pub fn get_baseline_duration_text(&self) -> Option<String> {
        self.string_field(TaskField::BaselineDuration)
    }

    /// 设置基线工期文本。
    ///
    /// 对应 Java: `public void setBaselineDurationText(String value)`
    pub fn set_baseline_duration_text(&mut self, value: Option<String>) {
        self.put_string(TaskField::BaselineDuration, value);
    }

    /// 取基线完成日期。
    ///
    /// 对应 Java: `public LocalDateTime getBaselineFinish()`
    pub fn get_baseline_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::BaselineFinish)
    }

    /// 取基线完成文本。
    ///
    /// 对应 Java: `public String getBaselineFinishText()`
    pub fn get_baseline_finish_text(&self) -> Option<String> {
        self.string_field(TaskField::BaselineFinish)
    }

    /// 设置基线完成文本。
    ///
    /// 对应 Java: `public void setBaselineFinishText(String value)`
    pub fn set_baseline_finish_text(&mut self, value: Option<String>) {
        self.put_string(TaskField::BaselineFinish, value);
    }

    /// 取基线开始日期。
    ///
    /// 对应 Java: `public LocalDateTime getBaselineStart()`
    pub fn get_baseline_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::BaselineStart)
    }

    /// 取基线开始文本。
    ///
    /// 对应 Java: `public String getBaselineStartText()`
    pub fn get_baseline_start_text(&self) -> Option<String> {
        self.string_field(TaskField::BaselineStart)
    }

    /// 设置基线开始文本。
    ///
    /// 对应 Java: `public void setBaselineStartText(String value)`
    pub fn set_baseline_start_text(&mut self, value: Option<String>) {
        self.put_string(TaskField::BaselineStart, value);
    }

    /// 取基线工时。
    ///
    /// 对应 Java: `public Duration getBaselineWork()`
    pub fn get_baseline_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::BaselineWork)
    }

    /// 取 BCWP。
    ///
    /// 对应 Java: `public Number getBCWP()`
    pub fn get_bcwp(&self) -> Option<f64> {
        self.number_field(TaskField::Bcwp)
    }

    /// 取 BCWS。
    ///
    /// 对应 Java: `public Number getBCWS()`
    pub fn get_bcws(&self) -> Option<f64> {
        self.number_field(TaskField::Bcws)
    }

    /// 取「已确认」标志。
    ///
    /// 对应 Java: `public boolean getConfirmed()`
    pub fn get_confirmed(&self) -> bool {
        self.boolean_field(TaskField::Confirmed)
    }

    /// 取约束日期。
    ///
    /// 对应 Java: `public LocalDateTime getConstraintDate()`
    pub fn get_constraint_date(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::ConstraintDate)
    }

    /// 取次要约束日期。
    ///
    /// 对应 Java: `public LocalDateTime getSecondaryConstraintDate()`
    pub fn get_secondary_constraint_date(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::SecondaryConstraintDate)
    }

    /// 取约束类型。
    ///
    /// 对应 Java: `public ConstraintType getConstraintType()`
    pub fn get_constraint_type(&self) -> Option<ConstraintType> {
        self.integer_field(TaskField::ConstraintType)
            .map(constraint_type_from_value)
    }

    /// 取次要约束类型。
    ///
    /// 对应 Java: `public ConstraintType getSecondaryConstraintType()`
    pub fn get_secondary_constraint_type(&self) -> Option<ConstraintType> {
        self.integer_field(TaskField::SecondaryConstraintType)
            .map(constraint_type_from_value)
    }

    /// 取联系人。
    ///
    /// 对应 Java: `public String getContact()`
    pub fn get_contact(&self) -> Option<String> {
        self.string_field(TaskField::Contact)
    }

    /// 取总成本。
    ///
    /// 对应 Java: `public Number getCost()`
    pub fn get_cost(&self) -> Option<f64> {
        self.number_field(TaskField::Cost)
    }

    /// 取成本偏差。
    ///
    /// 对应 Java: `public Number getCostVariance()`
    pub fn get_cost_variance(&self) -> Option<f64> {
        self.number_field(TaskField::CostVariance)
    }

    /// 取创建日期。
    ///
    /// 对应 Java: `public LocalDateTime getCreateDate()`
    pub fn get_create_date(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::Created)
    }

    /// 取关键路径标志。
    ///
    /// 对应 Java: `public boolean getCritical()`
    pub fn get_critical(&self) -> bool {
        self.boolean_field(TaskField::Critical)
    }

    /// 取 CV。
    ///
    /// 对应 Java: `public Number getCV()`
    pub fn get_cv(&self) -> Option<f64> {
        self.number_field(TaskField::Cv)
    }

    /// 取调配延隔时间。
    ///
    /// 对应 Java: `public Duration getLevelingDelay()`
    pub fn get_leveling_delay(&self) -> Option<Duration> {
        self.duration_field(TaskField::LevelingDelay)
    }

    /// 取工期。
    ///
    /// 对应 Java: `public Duration getDuration()`
    pub fn get_duration(&self) -> Option<Duration> {
        self.duration_field(TaskField::Duration)
    }

    /// 取手排任务的工期文本。
    ///
    /// 对应 Java: `public String getDurationText()`
    pub fn get_duration_text(&self) -> Option<String> {
        self.string_field(TaskField::DurationText)
    }

    /// 设置自定义工期字段。
    ///
    /// 对应 Java: `public void setDuration(int index, Duration value)`
    pub fn set_duration_at(&mut self, index: i32, value: Option<Duration>) {
        if let Some(field) = select_field(custom_duration_fields(), index) {
            self.put_duration(field, value);
        }
    }

    /// 读取自定义工期字段。
    ///
    /// 对应 Java: `public Duration getDuration(int index)`
    pub fn get_duration_at(&self, index: i32) -> Option<Duration> {
        select_field(custom_duration_fields(), index).and_then(|field| self.duration_field(field))
    }

    /// 取工期偏差。
    ///
    /// 对应 Java: `public Duration getDurationVariance()`
    pub fn get_duration_variance(&self) -> Option<Duration> {
        self.duration_field(TaskField::DurationVariance)
    }

    /// 取最早完成日期。
    ///
    /// 对应 Java: `public LocalDateTime getEarlyFinish()`
    pub fn get_early_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::EarlyFinish)
    }

    /// 取剩余最早完成日期。
    ///
    /// 对应 Java: `public LocalDateTime getRemainingEarlyFinish()`
    pub fn get_remaining_early_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::RemainingEarlyFinish)
    }

    /// 取最早开始日期。
    ///
    /// 对应 Java: `public LocalDateTime getEarlyStart()`
    pub fn get_early_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::EarlyStart)
    }

    /// 取剩余最早开始日期。
    ///
    /// 对应 Java: `public LocalDateTime getRemainingEarlyStart()`
    pub fn get_remaining_early_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::RemainingEarlyStart)
    }

    /// 取完成日期。
    ///
    /// 对应 Java: `public LocalDateTime getFinish()`
    pub fn get_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::Finish)
    }

    /// 取手排任务的完成文本。
    ///
    /// 对应 Java: `public String getFinishText()`
    pub fn get_finish_text(&self) -> Option<String> {
        self.string_field(TaskField::FinishText)
    }

    /// 设置自定义完成日期字段。
    ///
    /// 对应 Java: `public void setFinish(int index, LocalDateTime value)`
    pub fn set_finish_at(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(custom_finish_fields(), index) {
            self.put_date_time(field, value);
        }
    }

    /// 读取自定义完成日期字段。
    ///
    /// 对应 Java: `public LocalDateTime getFinish(int index)`
    pub fn get_finish_at(&self, index: i32) -> Option<NaiveDateTime> {
        select_field(custom_finish_fields(), index).and_then(|field| self.date_time_field(field))
    }

    /// 取完成偏差。
    ///
    /// 对应 Java: `public Duration getFinishVariance()`
    pub fn get_finish_variance(&self) -> Option<Duration> {
        self.duration_field(TaskField::FinishVariance)
    }

    /// 取固定成本。
    ///
    /// 对应 Java: `public Number getFixedCost()`
    pub fn get_fixed_cost(&self) -> Option<f64> {
        self.number_field(TaskField::FixedCost)
    }

    /// 设置自定义标志字段。
    ///
    /// 对应 Java: `public void setFlag(int index, boolean value)`
    pub fn set_flag(&mut self, index: i32, value: bool) {
        if let Some(field) = select_field(custom_flag_fields(), index) {
            self.put_bool(field, value);
        }
    }

    /// 读取自定义标志字段。
    ///
    /// 对应 Java: `public boolean getFlag(int index)`
    pub fn get_flag(&self, index: i32) -> bool {
        select_field(custom_flag_fields(), index).map_or(false, |field| self.boolean_field(field))
    }

    /// 取自由时差。
    ///
    /// 对应 Java: `public Duration getFreeSlack()`
    pub fn get_free_slack(&self) -> Option<Duration> {
        self.duration_field(TaskField::FreeSlack)
    }

    /// 取「隐藏甘特条」标志。
    ///
    /// 对应 Java: `public boolean getHideBar()`
    pub fn get_hide_bar(&self) -> bool {
        self.boolean_field(TaskField::HideBar)
    }

    /// 取任务 ID。
    ///
    /// 对应 Java: `@Override public Integer getID()`
    pub fn get_id(&self) -> Option<i32> {
        self.integer_field(TaskField::Id)
    }

    /// 取最晚完成日期。
    ///
    /// 对应 Java: `public LocalDateTime getLateFinish()`
    pub fn get_late_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::LateFinish)
    }

    /// 取剩余最晚完成日期。
    ///
    /// 对应 Java: `public LocalDateTime getRemainingLateFinish()`
    pub fn get_remaining_late_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::RemainingLateFinish)
    }

    /// 取最晚开始日期。
    ///
    /// 对应 Java: `public LocalDateTime getLateStart()`
    pub fn get_late_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::LateStart)
    }

    /// 取剩余最晚开始日期。
    ///
    /// 对应 Java: `public LocalDateTime getRemainingLateStart()`
    pub fn get_remaining_late_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::RemainingLateStart)
    }

    /// 取「存在 OLE 链接」标志。
    ///
    /// 对应 Java: `public boolean getLinkedFields()`
    pub fn get_linked_fields(&self) -> bool {
        self.boolean_field(TaskField::LinkedFields)
    }

    /// 取「标记」标志。
    ///
    /// 对应 Java: `public boolean getMarked()`
    pub fn get_marked(&self) -> bool {
        self.boolean_field(TaskField::Marked)
    }

    /// 取里程碑标志。
    ///
    /// 对应 Java: `public boolean getMilestone()`
    pub fn get_milestone(&self) -> bool {
        self.boolean_field(TaskField::Milestone)
    }

    /// 取任务名称。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> Option<String> {
        self.string_field(TaskField::Name)
    }

    /// 取任务笔记的纯文本表示（无笔记时返回空串）。
    ///
    /// 对应 Java: `public String getNotes()`
    pub fn get_notes(&self) -> String {
        self.string_field(TaskField::Notes).unwrap_or_default()
    }

    /// 取包含纯文本笔记的 `Notes` 实例。
    ///
    /// 对应 Java: `public Notes getNotesObject()`
    ///
    /// Rust 侧 `Notes` 是 trait，字段表只保存纯文本，故返回 [`PlainNotes`]
    /// （格式化笔记的子类型信息已丢失，见文件头差异第 7 条）。
    pub fn get_notes_object(&self) -> Option<Box<dyn Notes>> {
        self.string_field(TaskField::Notes)
            .map(|text| Box::new(PlainNotes::new(Some(text))) as Box<dyn Notes>)
    }

    /// 设置自定义数字字段。
    ///
    /// 对应 Java: `public void setNumber(int index, Number value)`
    pub fn set_number(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = select_field(custom_number_fields(), index) {
            self.put_number(field, value);
        }
    }

    /// 读取自定义数字字段。
    ///
    /// 对应 Java: `public Number getNumber(int index)`
    pub fn get_number(&self, index: i32) -> Option<f64> {
        select_field(custom_number_fields(), index).and_then(|field| self.number_field(field))
    }

    /// 取附加对象数量。
    ///
    /// 对应 Java: `public Integer getObjects()`
    pub fn get_objects(&self) -> Option<i32> {
        self.integer_field(TaskField::Objects)
    }

    /// 取大纲级别。
    ///
    /// 对应 Java: `public Integer getOutlineLevel()`
    pub fn get_outline_level(&self) -> Option<i32> {
        self.integer_field(TaskField::OutlineLevel)
    }

    /// 取大纲编号。
    ///
    /// 对应 Java: `public String getOutlineNumber()`
    pub fn get_outline_number(&self) -> Option<String> {
        self.string_field(TaskField::OutlineNumber)
    }

    /// 取优先级。
    ///
    /// 对应 Java: `public Priority getPriority()`
    pub fn get_priority(&self) -> Option<Priority> {
        self.integer_field(TaskField::Priority)
            .map(Priority::get_instance)
    }

    /// 取来源项目名称。
    ///
    /// 对应 Java: `public String getProject()`
    pub fn get_project(&self) -> Option<String> {
        self.string_field(TaskField::Project)
    }

    /// 取剩余成本。
    ///
    /// 对应 Java: `public Number getRemainingCost()`
    pub fn get_remaining_cost(&self) -> Option<f64> {
        self.number_field(TaskField::RemainingCost)
    }

    /// 取剩余工期。
    ///
    /// 对应 Java: `public Duration getRemainingDuration()`
    pub fn get_remaining_duration(&self) -> Option<Duration> {
        self.duration_field(TaskField::RemainingDuration)
    }

    /// 取剩余工时。
    ///
    /// 对应 Java: `public Duration getRemainingWork()`
    pub fn get_remaining_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::RemainingWork)
    }

    /// 取资源组。
    ///
    /// 对应 Java: `public String getResourceGroup()`
    pub fn get_resource_group(&self) -> Option<String> {
        self.string_field(TaskField::ResourceGroup)
    }

    /// 取资源缩写。
    ///
    /// 对应 Java: `public String getResourceInitials()`
    pub fn get_resource_initials(&self) -> Option<String> {
        self.string_field(TaskField::ResourceInitials)
    }

    /// 取资源名称列表。
    ///
    /// 对应 Java: `public String getResourceNames()`
    pub fn get_resource_names(&self) -> Option<String> {
        self.string_field(TaskField::ResourceNames)
    }

    /// 取恢复日期。
    ///
    /// 对应 Java: `public LocalDateTime getResume()`
    pub fn get_resume(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::Resume)
    }

    /// 取汇总标志。
    ///
    /// 对应 Java: `public boolean getRollup()`
    pub fn get_rollup(&self) -> bool {
        self.boolean_field(TaskField::Rollup)
    }

    /// 取开始日期。
    ///
    /// 对应 Java: `public LocalDateTime getStart()`
    pub fn get_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::Start)
    }

    /// 取手排任务的开始文本。
    ///
    /// 对应 Java: `public String getStartText()`
    pub fn get_start_text(&self) -> Option<String> {
        self.string_field(TaskField::StartText)
    }

    /// 设置自定义开始日期字段。
    ///
    /// 对应 Java: `public void setStart(int index, LocalDateTime value)`
    pub fn set_start_at(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(custom_start_fields(), index) {
            self.put_date_time(field, value);
        }
    }

    /// 读取自定义开始日期字段。
    ///
    /// 对应 Java: `public LocalDateTime getStart(int index)`
    pub fn get_start_at(&self, index: i32) -> Option<NaiveDateTime> {
        select_field(custom_start_fields(), index).and_then(|field| self.date_time_field(field))
    }

    /// 取开始偏差。
    ///
    /// 对应 Java: `public Duration getStartVariance()`
    pub fn get_start_variance(&self) -> Option<Duration> {
        self.duration_field(TaskField::StartVariance)
    }

    /// 取停止日期。
    ///
    /// 对应 Java: `public LocalDateTime getStop()`
    pub fn get_stop(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::Stop)
    }

    /// 取子项目文件路径。
    ///
    /// 对应 Java: `public String getSubprojectFile()`
    pub fn get_subproject_file(&self) -> Option<String> {
        self.string_field(TaskField::SubprojectFile)
    }

    /// 取汇总任务标志。
    ///
    /// 对应 Java: `public boolean getSummary()`
    pub fn get_summary(&self) -> bool {
        self.boolean_field(TaskField::Summary)
    }

    /// 取 SV。
    ///
    /// 对应 Java: `public Number getSV()`
    pub fn get_sv(&self) -> Option<f64> {
        self.number_field(TaskField::Sv)
    }

    /// 设置自定义文本字段。
    ///
    /// 对应 Java: `public void setText(int index, String value)`
    pub fn set_text(&mut self, index: i32, value: Option<String>) {
        if let Some(field) = select_field(custom_text_fields(), index) {
            self.put_string(field, value);
        }
    }

    /// 读取自定义文本字段。
    ///
    /// 对应 Java: `public String getText(int index)`
    pub fn get_text(&self, index: i32) -> Option<String> {
        select_field(custom_text_fields(), index).and_then(|field| self.string_field(field))
    }

    /// 设置自定义大纲代码字段。
    ///
    /// 对应 Java: `public void setOutlineCode(int index, String value)`
    pub fn set_outline_code(&mut self, index: i32, value: Option<String>) {
        if let Some(field) = select_field(custom_outline_code_fields(), index) {
            self.put_string(field, value);
        }
    }

    /// 读取自定义大纲代码字段。
    ///
    /// 对应 Java: `public String getOutlineCode(int index)`
    pub fn get_outline_code(&self, index: i32) -> Option<String> {
        select_field(custom_outline_code_fields(), index).and_then(|field| self.string_field(field))
    }

    /// 取总时差。
    ///
    /// 对应 Java: `public Duration getTotalSlack()`
    pub fn get_total_slack(&self) -> Option<Duration> {
        self.duration_field(TaskField::TotalSlack)
    }

    /// 取唯一 ID。
    ///
    /// 对应 Java: `@Override public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.integer_field(TaskField::UniqueId)
    }

    /// 取「需要更新」标志。
    ///
    /// 对应 Java: `public boolean getUpdateNeeded()`
    pub fn get_update_needed(&self) -> bool {
        self.boolean_field(TaskField::UpdateNeeded)
    }

    /// 取 WBS 编码。
    ///
    /// 对应 Java: `public String getWBS()`
    pub fn get_wbs(&self) -> Option<String> {
        self.string_field(TaskField::Wbs)
    }

    /// 取工时。
    ///
    /// 对应 Java: `public Duration getWork()`
    pub fn get_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::Work)
    }

    /// 取工时偏差。
    ///
    /// 对应 Java: `public Duration getWorkVariance()`
    pub fn get_work_variance(&self) -> Option<Duration> {
        self.duration_field(TaskField::WorkVariance)
    }

    /// 取父任务的唯一 ID。
    ///
    /// 对应 Java: `public Integer getParentTaskUniqueID()`
    pub fn get_parent_task_unique_id(&self) -> Option<i32> {
        self.integer_field(TaskField::ParentTaskUniqueId)
    }

    /// 按 ID 比较两个任务。
    ///
    /// 对应 Java: `@Override public int compareTo(Task o)`
    ///
    /// Java 的 `compareTo` 与「同一性相等」不自洽，无法映射为 Rust 的 `Ord`
    /// （见文件头差异第 6 条），故保留为具名方法。
    pub fn compare_to(&self, other: &Task) -> Ordering {
        let id1 = NumberHelper::get_int(self.get_id().map(f64::from));
        let id2 = NumberHelper::get_int(other.get_id().map(f64::from));
        id1.cmp(&id2)
    }
}

// ---------------------------------------------------------------------------
// 布尔 / 日期 / 枚举 / UUID 属性
// ---------------------------------------------------------------------------

impl Task {
    /// 取「工期为估算值」标志。
    ///
    /// 对应 Java: `public boolean getEstimated()`
    pub fn get_estimated(&self) -> bool {
        self.boolean_field(TaskField::Estimated)
    }

    /// 设置「工期为估算值」标志。
    ///
    /// 对应 Java: `public void setEstimated(boolean estimated)`
    pub fn set_estimated(&mut self, estimated: bool) {
        self.put_bool(TaskField::Estimated, estimated);
    }

    /// 取截止日期。
    ///
    /// 对应 Java: `public LocalDateTime getDeadline()`
    pub fn get_deadline(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::Deadline)
    }

    /// 设置截止日期。
    ///
    /// 对应 Java: `public void setDeadline(LocalDateTime deadline)`
    pub fn set_deadline(&mut self, deadline: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::Deadline, deadline);
    }

    /// 取任务类型。
    ///
    /// 对应 Java: `public TaskType getType()`
    pub fn get_type(&self) -> Option<TaskType> {
        self.integer_field(TaskField::Type)
            .and_then(task_type_from_value)
    }

    /// 设置任务类型。
    ///
    /// 对应 Java: `public void setType(TaskType type)`
    pub fn set_type(&mut self, r#type: Option<TaskType>) {
        self.put_enum(TaskField::Type, r#type.map(task_type_to_value));
    }

    /// 取「空任务」标志。
    ///
    /// 对应 Java: `public boolean getNull()`
    pub fn get_null(&self) -> bool {
        self.boolean_field(TaskField::Null)
    }

    /// 设置「空任务」标志。
    ///
    /// 对应 Java: `public void setNull(boolean isNull)`
    pub fn set_null(&mut self, is_null: bool) {
        self.put_bool(TaskField::Null, is_null);
    }

    /// 取「恢复有效」标志。
    ///
    /// 对应 Java: `public boolean getResumeValid()`
    pub fn get_resume_valid(&self) -> bool {
        self.boolean_field(TaskField::ResumeValid)
    }

    /// 设置「恢复有效」标志。
    ///
    /// 对应 Java: `public void setResumeValid(boolean resumeValid)`
    pub fn set_resume_valid(&mut self, resume_valid: bool) {
        self.put_bool(TaskField::ResumeValid, resume_valid);
    }

    /// 取「重复任务」标志。
    ///
    /// 对应 Java: `public boolean getRecurring()`
    pub fn get_recurring(&self) -> bool {
        self.boolean_field(TaskField::Recurring)
    }

    /// 设置「重复任务」标志。
    ///
    /// 对应 Java: `public void setRecurring(boolean recurring)`
    pub fn set_recurring(&mut self, recurring: bool) {
        self.put_bool(TaskField::Recurring, recurring);
    }

    /// 取「资源过度分配」标志。
    ///
    /// 对应 Java: `public boolean getOverAllocated()`
    pub fn get_over_allocated(&self) -> bool {
        self.boolean_field(TaskField::Overallocated)
    }

    /// 设置「资源过度分配」标志。
    ///
    /// 对应 Java: `public void setOverAllocated(boolean overAllocated)`
    pub fn set_over_allocated(&mut self, over_allocated: bool) {
        self.put_bool(TaskField::Overallocated, over_allocated);
    }

    /// 取子项目中任务的唯一 ID。
    ///
    /// 对应 Java: `public Integer getSubprojectTaskUniqueID()`
    pub fn get_subproject_task_unique_id(&self) -> Option<i32> {
        self.integer_field(TaskField::SubprojectTaskUniqueId)
    }

    /// 设置子项目中任务的唯一 ID。
    ///
    /// 对应 Java: `public void setSubprojectTaskUniqueID(Integer subprojectUniqueTaskID)`
    pub fn set_subproject_task_unique_id(&mut self, subproject_unique_task_id: Option<i32>) {
        self.put_integer(TaskField::SubprojectTaskUniqueId, subproject_unique_task_id);
    }

    /// 取子项目中任务的 ID。
    ///
    /// 对应 Java: `public Integer getSubprojectTaskID()`
    pub fn get_subproject_task_id(&self) -> Option<i32> {
        self.integer_field(TaskField::SubprojectTaskId)
    }

    /// 设置子项目中任务的 ID。
    ///
    /// 对应 Java: `public void setSubprojectTaskID(Integer subprojectTaskID)`
    pub fn set_subproject_task_id(&mut self, subproject_task_id: Option<i32>) {
        self.put_integer(TaskField::SubprojectTaskId, subproject_task_id);
    }

    /// 设置子项目任务唯一 ID 的偏移量。
    ///
    /// 对应 Java: `public void setSubprojectTasksUniqueIDOffset(Integer offset)`
    pub fn set_subproject_tasks_unique_id_offset(&mut self, offset: Option<i32>) {
        self.put_integer(TaskField::SubprojectTasksUniqueidOffset, offset);
    }

    /// 取子项目任务唯一 ID 的偏移量。
    ///
    /// 对应 Java: `public Integer getSubprojectTasksUniqueIDOffset()`
    pub fn get_subproject_tasks_unique_id_offset(&self) -> Option<i32> {
        self.integer_field(TaskField::SubprojectTasksUniqueidOffset)
    }

    /// 取子项目只读标志。
    ///
    /// 对应 Java: `public boolean getSubprojectReadOnly()`
    pub fn get_subproject_read_only(&self) -> bool {
        self.boolean_field(TaskField::SubprojectReadOnly)
    }

    /// 设置子项目只读标志。
    ///
    /// 对应 Java: `public void setSubprojectReadOnly(boolean subprojectReadOnly)`
    pub fn set_subproject_read_only(&mut self, subproject_read_only: bool) {
        self.put_bool(TaskField::SubprojectReadOnly, subproject_read_only);
    }

    /// 取外部任务标志。
    ///
    /// 对应 Java: `public boolean getExternalTask()`
    pub fn get_external_task(&self) -> bool {
        self.boolean_field(TaskField::ExternalTask)
    }

    /// 设置外部任务标志。
    ///
    /// 对应 Java: `public void setExternalTask(boolean externalTask)`
    pub fn set_external_task(&mut self, external_task: bool) {
        self.put_bool(TaskField::ExternalTask, external_task);
    }

    /// 取外部项目标志。
    ///
    /// 对应 Java: `public boolean getExternalProject()`
    pub fn get_external_project(&self) -> bool {
        self.boolean_field(TaskField::ExternalProject)
    }

    /// 取 ACWP（已完成工作实际成本）。
    ///
    /// 对应 Java: `public Number getACWP()`
    pub fn get_acwp(&self) -> Option<f64> {
        self.number_field(TaskField::Acwp)
    }

    /// 设置 ACWP。
    ///
    /// 对应 Java: `public void setACWP(Number acwp)`
    pub fn set_acwp(&mut self, acwp: Option<f64>) {
        self.put_number(TaskField::Acwp, acwp);
    }

    /// 取调配延隔时间单位。
    ///
    /// 对应 Java: `public TimeUnit getLevelingDelayFormat()`
    pub fn get_leveling_delay_format(&self) -> Option<TimeUnit> {
        self.integer_field(TaskField::LevelingDelayUnits)
            .map(TimeUnit::get_instance)
    }

    /// 设置调配延隔时间单位。
    ///
    /// 对应 Java: `public void setLevelingDelayFormat(TimeUnit levelingDelayFormat)`
    pub fn set_leveling_delay_format(&mut self, leveling_delay_format: Option<TimeUnit>) {
        self.put_enum(
            TaskField::LevelingDelayUnits,
            leveling_delay_format.map(|value| value.value()),
        );
    }

    /// 取「忽略资源日历」标志。
    ///
    /// 对应 Java: `public boolean getIgnoreResourceCalendar()`
    pub fn get_ignore_resource_calendar(&self) -> bool {
        self.boolean_field(TaskField::IgnoreResourceCalendar)
    }

    /// 设置「忽略资源日历」标志。
    ///
    /// 对应 Java: `public void setIgnoreResourceCalendar(boolean ignoreResourceCalendar)`
    pub fn set_ignore_resource_calendar(&mut self, ignore_resource_calendar: bool) {
        self.put_bool(TaskField::IgnoreResourceCalendar, ignore_resource_calendar);
    }

    /// 取实际完成百分比。
    ///
    /// 对应 Java: `public Number getPhysicalPercentComplete()`
    pub fn get_physical_percent_complete(&self) -> Option<f64> {
        self.number_field(TaskField::PhysicalPercentComplete)
    }

    /// 设置实际完成百分比。
    ///
    /// 对应 Java: `public void setPhysicalPercentComplete(Number physicalPercentComplete)`
    pub fn set_physical_percent_complete(&mut self, physical_percent_complete: Option<f64>) {
        self.put_number(TaskField::PhysicalPercentComplete, physical_percent_complete);
    }

    /// 取估算权重。
    ///
    /// 对应 Java: `public Number getEstimatedWeight()`
    pub fn get_estimated_weight(&self) -> Option<f64> {
        self.number_field(TaskField::EstimatedWeight)
    }

    /// 设置估算权重。
    ///
    /// 对应 Java: `public void setEstimatedWeight(Number estimatedWeight)`
    pub fn set_estimated_weight(&mut self, estimated_weight: Option<f64>) {
        self.put_number(TaskField::EstimatedWeight, estimated_weight);
    }

    /// 取「自动计算实际值」标志。
    ///
    /// 对应 Java: `public boolean getAutoComputeActuals()`
    pub fn get_auto_compute_actuals(&self) -> bool {
        self.boolean_field(TaskField::AutoComputeActuals)
    }

    /// 设置「自动计算实际值」标志。
    ///
    /// 对应 Java: `public void setAutoComputeActuals(boolean autoComputeActuals)`
    pub fn set_auto_compute_actuals(&mut self, auto_compute_actuals: bool) {
        self.put_bool(TaskField::AutoComputeActuals, auto_compute_actuals);
    }

    /// 取挣值计算方法。
    ///
    /// 对应 Java: `public EarnedValueMethod getEarnedValueMethod()`
    pub fn get_earned_value_method(&self) -> Option<EarnedValueMethod> {
        self.integer_field(TaskField::EarnedValueMethod)
            .map(EarnedValueMethod::get_instance)
    }

    /// 设置挣值计算方法。
    ///
    /// 对应 Java: `public void setEarnedValueMethod(EarnedValueMethod earnedValueMethod)`
    pub fn set_earned_value_method(&mut self, earned_value_method: Option<EarnedValueMethod>) {
        self.put_enum(
            TaskField::EarnedValueMethod,
            earned_value_method.map(|value| value.value()),
        );
    }

    /// 取受保护的实际工时。
    ///
    /// 对应 Java: `public Duration getActualWorkProtected()`
    pub fn get_actual_work_protected(&self) -> Option<Duration> {
        self.duration_field(TaskField::ActualWorkProtected)
    }

    /// 设置受保护的实际工时。
    ///
    /// 对应 Java: `public void setActualWorkProtected(Duration actualWorkProtected)`
    pub fn set_actual_work_protected(&mut self, actual_work_protected: Option<Duration>) {
        self.put_duration(TaskField::ActualWorkProtected, actual_work_protected);
    }

    /// 取受保护的实际加班工时。
    ///
    /// 对应 Java: `public Duration getActualOvertimeWorkProtected()`
    pub fn get_actual_overtime_work_protected(&self) -> Option<Duration> {
        self.duration_field(TaskField::ActualOvertimeWorkProtected)
    }

    /// 设置受保护的实际加班工时。
    ///
    /// 对应 Java: `public void setActualOvertimeWorkProtected(Duration actualOvertimeWorkProtected)`
    pub fn set_actual_overtime_work_protected(
        &mut self,
        actual_overtime_work_protected: Option<Duration>,
    ) {
        self.put_duration(
            TaskField::ActualOvertimeWorkProtected,
            actual_overtime_work_protected,
        );
    }

    /// 取常规工时。
    ///
    /// 对应 Java: `public Duration getRegularWork()`
    pub fn get_regular_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::RegularWork)
    }

    /// 设置常规工时。
    ///
    /// 对应 Java: `public void setRegularWork(Duration regularWork)`
    pub fn set_regular_work(&mut self, regular_work: Option<Duration>) {
        self.put_duration(TaskField::RegularWork, regular_work);
    }

    /// 设置「投入比导向」标志。
    ///
    /// 对应 Java: `public void setEffortDriven(boolean flag)`
    pub fn set_effort_driven(&mut self, flag: bool) {
        self.put_bool(TaskField::EffortDriven, flag);
    }

    /// 取「投入比导向」标志。
    ///
    /// 对应 Java: `public boolean getEffortDriven()`
    pub fn get_effort_driven(&self) -> bool {
        self.boolean_field(TaskField::EffortDriven)
    }

    /// 设置自定义日期字段。
    ///
    /// 对应 Java: `public void setDate(int index, LocalDateTime value)`
    pub fn set_date(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(custom_date_fields(), index) {
            self.put_date_time(field, value);
        }
    }

    /// 读取自定义日期字段。
    ///
    /// 对应 Java: `public LocalDateTime getDate(int index)`
    pub fn get_date(&self, index: i32) -> Option<NaiveDateTime> {
        select_field(custom_date_fields(), index).and_then(|field| self.date_time_field(field))
    }

    /// 取加班成本。
    ///
    /// 对应 Java: `public Number getOvertimeCost()`
    pub fn get_overtime_cost(&self) -> Option<f64> {
        self.number_field(TaskField::OvertimeCost)
    }

    /// 设置加班成本。
    ///
    /// 对应 Java: `public void setOvertimeCost(Number number)`
    pub fn set_overtime_cost(&mut self, number: Option<f64>) {
        self.put_number(TaskField::OvertimeCost, number);
    }

    /// 取实际加班成本。
    ///
    /// 对应 Java: `public Number getActualOvertimeCost()`
    pub fn get_actual_overtime_cost(&self) -> Option<f64> {
        self.number_field(TaskField::ActualOvertimeCost)
    }

    /// 设置实际加班成本。
    ///
    /// 对应 Java: `public void setActualOvertimeCost(Number cost)`
    pub fn set_actual_overtime_cost(&mut self, cost: Option<f64>) {
        self.put_number(TaskField::ActualOvertimeCost, cost);
    }

    /// 取实际加班工时。
    ///
    /// 对应 Java: `public Duration getActualOvertimeWork()`
    pub fn get_actual_overtime_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::ActualOvertimeWork)
    }

    /// 设置实际加班工时。
    ///
    /// 对应 Java: `public void setActualOvertimeWork(Duration work)`
    pub fn set_actual_overtime_work(&mut self, work: Option<Duration>) {
        self.put_duration(TaskField::ActualOvertimeWork, work);
    }

    /// 取固定成本计入方式。
    ///
    /// 对应 Java: `public AccrueType getFixedCostAccrual()`
    pub fn get_fixed_cost_accrual(&self) -> Option<AccrueType> {
        self.integer_field(TaskField::FixedCostAccrual)
            .and_then(AccrueType::get_instance)
    }

    /// 设置固定成本计入方式。
    ///
    /// 对应 Java: `public void setFixedCostAccrual(AccrueType type)`
    pub fn set_fixed_cost_accrual(&mut self, r#type: Option<AccrueType>) {
        self.put_enum(
            TaskField::FixedCostAccrual,
            r#type.map(|value| value.value()),
        );
    }

    /// 取超链接文本。
    ///
    /// 对应 Java: `public String getHyperlink()`
    pub fn get_hyperlink(&self) -> Option<String> {
        self.string_field(TaskField::Hyperlink)
    }

    /// 取超链接地址。
    ///
    /// 对应 Java: `public String getHyperlinkAddress()`
    pub fn get_hyperlink_address(&self) -> Option<String> {
        self.string_field(TaskField::HyperlinkAddress)
    }

    /// 取超链接子地址。
    ///
    /// 对应 Java: `public String getHyperlinkSubAddress()`
    pub fn get_hyperlink_sub_address(&self) -> Option<String> {
        self.string_field(TaskField::HyperlinkSubaddress)
    }

    /// 取超链接屏幕提示。
    ///
    /// 对应 Java: `public String getHyperlinkScreenTip()`
    pub fn get_hyperlink_screen_tip(&self) -> Option<String> {
        self.string_field(TaskField::HyperlinkScreenTip)
    }

    /// 设置超链接文本。
    ///
    /// 对应 Java: `public void setHyperlink(String text)`
    pub fn set_hyperlink(&mut self, text: Option<String>) {
        self.put_string(TaskField::Hyperlink, text);
    }

    /// 设置超链接地址。
    ///
    /// 对应 Java: `public void setHyperlinkAddress(String text)`
    pub fn set_hyperlink_address(&mut self, text: Option<String>) {
        self.put_string(TaskField::HyperlinkAddress, text);
    }

    /// 设置超链接子地址。
    ///
    /// 对应 Java: `public void setHyperlinkSubAddress(String text)`
    pub fn set_hyperlink_sub_address(&mut self, text: Option<String>) {
        self.put_string(TaskField::HyperlinkSubaddress, text);
    }

    /// 设置超链接屏幕提示。
    ///
    /// 对应 Java: `public void setHyperlinkScreenTip(String text)`
    pub fn set_hyperlink_screen_tip(&mut self, text: Option<String>) {
        self.put_string(TaskField::HyperlinkScreenTip, text);
    }

    /// 取「可调配分配」标志。
    ///
    /// 对应 Java: `public boolean getLevelAssignments()`
    pub fn get_level_assignments(&self) -> bool {
        self.boolean_field(TaskField::LevelAssignments)
    }

    /// 设置「可调配分配」标志。
    ///
    /// 对应 Java: `public void setLevelAssignments(boolean flag)`
    pub fn set_level_assignments(&mut self, flag: bool) {
        self.put_bool(TaskField::LevelAssignments, flag);
    }

    /// 取「调配可拆分」标志。
    ///
    /// 对应 Java: `public boolean getLevelingCanSplit()`
    pub fn get_leveling_can_split(&self) -> bool {
        self.boolean_field(TaskField::LevelingCanSplit)
    }

    /// 设置「调配可拆分」标志。
    ///
    /// 对应 Java: `public void setLevelingCanSplit(boolean flag)`
    pub fn set_leveling_can_split(&mut self, flag: bool) {
        self.put_bool(TaskField::LevelingCanSplit, flag);
    }

    /// 取加班工时。
    ///
    /// 对应 Java: `public Duration getOvertimeWork()`
    pub fn get_overtime_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::OvertimeWork)
    }

    /// 设置加班工时。
    ///
    /// 对应 Java: `public void setOvertimeWork(Duration work)`
    pub fn set_overtime_work(&mut self, work: Option<Duration>) {
        self.put_duration(TaskField::OvertimeWork, work);
    }

    /// 取预调配开始日期。
    ///
    /// 对应 Java: `public LocalDateTime getPreleveledStart()`
    pub fn get_preleveled_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::PreleveledStart)
    }

    /// 取预调配完成日期。
    ///
    /// 对应 Java: `public LocalDateTime getPreleveledFinish()`
    pub fn get_preleveled_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::PreleveledFinish)
    }

    /// 设置预调配开始日期。
    ///
    /// 对应 Java: `public void setPreleveledStart(LocalDateTime date)`
    pub fn set_preleveled_start(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::PreleveledStart, date);
    }

    /// 设置预调配完成日期。
    ///
    /// 对应 Java: `public void setPreleveledFinish(LocalDateTime date)`
    pub fn set_preleveled_finish(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::PreleveledFinish, date);
    }

    /// 取剩余加班工时。
    ///
    /// 对应 Java: `public Duration getRemainingOvertimeWork()`
    pub fn get_remaining_overtime_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::RemainingOvertimeWork)
    }

    /// 设置剩余加班工时。
    ///
    /// 对应 Java: `public void setRemainingOvertimeWork(Duration work)`
    pub fn set_remaining_overtime_work(&mut self, work: Option<Duration>) {
        self.put_duration(TaskField::RemainingOvertimeWork, work);
    }

    /// 取剩余加班成本。
    ///
    /// 对应 Java: `public Number getRemainingOvertimeCost()`
    pub fn get_remaining_overtime_cost(&self) -> Option<f64> {
        self.number_field(TaskField::RemainingOvertimeCost)
    }

    /// 设置剩余加班成本。
    ///
    /// 对应 Java: `public void setRemainingOvertimeCost(Number cost)`
    pub fn set_remaining_overtime_cost(&mut self, cost: Option<f64>) {
        self.put_number(TaskField::RemainingOvertimeCost, cost);
    }

    /// 取任务关联的日历。
    ///
    /// 对应 Java: `public ProjectCalendar getCalendar()`
    pub fn get_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        let file = self.parent_file.upgrade()?;
        let calendar_unique_id = self.get_calendar_unique_id();
        file.borrow().get_calendars().get_by_unique_id(calendar_unique_id)
    }

    /// 设置日历唯一 ID。
    ///
    /// 对应 Java: `public void setCalendarUniqueID(Integer id)`
    pub fn set_calendar_unique_id(&mut self, id: Option<i32>) {
        self.put_integer(TaskField::CalendarUniqueId, id);
    }

    /// 取日历唯一 ID。
    ///
    /// 对应 Java: `public Integer getCalendarUniqueID()`
    pub fn get_calendar_unique_id(&self) -> Option<i32> {
        self.integer_field(TaskField::CalendarUniqueId)
    }

    /// 设置任务关联的日历。
    ///
    /// 对应 Java: `public void setCalendar(ProjectCalendar calendar)`
    pub fn set_calendar(&mut self, calendar: Option<&Rc<RefCell<ProjectCalendar>>>) {
        let unique_id = calendar.and_then(|calendar| calendar.borrow().get_unique_id());
        self.set_calendar_unique_id(unique_id);
    }

    /// 取「展开显示」标志。
    ///
    /// 对应 Java: `public boolean getExpanded()`
    pub fn get_expanded(&self) -> bool {
        self.boolean_field(TaskField::Expanded)
    }

    /// 设置「展开显示」标志。
    ///
    /// 对应 Java: `public void setExpanded(boolean expanded)`
    pub fn set_expanded(&mut self, expanded: bool) {
        self.put_bool(TaskField::Expanded, expanded);
    }

    /// 设置开始浮动时间。
    ///
    /// 对应 Java: `public void setStartSlack(Duration duration)`
    pub fn set_start_slack(&mut self, duration: Option<Duration>) {
        self.put_duration(TaskField::StartSlack, duration);
    }

    /// 设置完成浮动时间。
    ///
    /// 对应 Java: `public void setFinishSlack(Duration duration)`
    pub fn set_finish_slack(&mut self, duration: Option<Duration>) {
        self.put_duration(TaskField::FinishSlack, duration);
    }

    /// 取开始浮动时间。
    ///
    /// 对应 Java: `public Duration getStartSlack()`
    pub fn get_start_slack(&self) -> Option<Duration> {
        self.duration_field(TaskField::StartSlack)
    }

    /// 取完成浮动时间。
    ///
    /// 对应 Java: `public Duration getFinishSlack()`
    pub fn get_finish_slack(&self) -> Option<Duration> {
        self.duration_field(TaskField::FinishSlack)
    }

    /// 按字段别名读取字段值。
    ///
    /// 对应 Java: `public Object getFieldByAlias(String alias)`
    pub fn get_field_by_alias(&self, alias: &str) -> Option<FieldValue> {
        let file = self.parent_file.upgrade()?;
        let field = file.borrow().get_tasks().get_field_type_by_alias(alias)?;
        self.get(field)
    }

    /// 按字段别名设置字段值。
    ///
    /// 对应 Java: `public void setFieldByAlias(String alias, Object value)`
    pub fn set_field_by_alias(&mut self, alias: &str, value: Option<FieldValue>) {
        let Some(file) = self.parent_file.upgrade() else {
            return;
        };
        let field = file.borrow().get_tasks().get_field_type_by_alias(alias);
        if let Some(field) = field {
            self.set(field, value);
        }
    }
}

// ---------------------------------------------------------------------------
// 多基线（Baseline 1-10）与 Enterprise 自定义字段
// ---------------------------------------------------------------------------

impl Task {
    /// 设置基线成本（多基线）。
    ///
    /// 对应 Java: `public void setBaselineCost(int baselineNumber, Number value)`
    pub fn set_baseline_cost_at(&mut self, baseline_number: i32, value: Option<f64>) {
        if let Some(field) = select_field(baseline_cost_fields(), baseline_number) {
            self.put_number(field, value);
        }
    }

    /// 设置基线工期（多基线）。
    ///
    /// 对应 Java: `public void setBaselineDuration(int baselineNumber, Duration value)`
    pub fn set_baseline_duration_at(&mut self, baseline_number: i32, value: Option<Duration>) {
        if let Some(field) = select_field(baseline_duration_fields(), baseline_number) {
            self.put_duration(field, value);
        }
    }

    /// 设置基线完成日期（多基线）。
    ///
    /// 对应 Java: `public void setBaselineFinish(int baselineNumber, LocalDateTime value)`
    pub fn set_baseline_finish_at(&mut self, baseline_number: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(baseline_finish_fields(), baseline_number) {
            self.put_date_time(field, value);
        }
    }

    /// 设置基线开始日期（多基线）。
    ///
    /// 对应 Java: `public void setBaselineStart(int baselineNumber, LocalDateTime value)`
    pub fn set_baseline_start_at(&mut self, baseline_number: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(baseline_start_fields(), baseline_number) {
            self.put_date_time(field, value);
        }
    }

    /// 设置基线工时（多基线）。
    ///
    /// 对应 Java: `public void setBaselineWork(int baselineNumber, Duration value)`
    pub fn set_baseline_work_at(&mut self, baseline_number: i32, value: Option<Duration>) {
        if let Some(field) = select_field(baseline_work_fields(), baseline_number) {
            self.put_duration(field, value);
        }
    }

    /// 取基线成本（多基线）。
    ///
    /// 对应 Java: `public Number getBaselineCost(int baselineNumber)`
    pub fn get_baseline_cost_at(&self, baseline_number: i32) -> Option<f64> {
        select_field(baseline_cost_fields(), baseline_number)
            .and_then(|field| self.number_field(field))
    }

    /// 取基线工期（多基线）。
    ///
    /// 对应 Java: `public Duration getBaselineDuration(int baselineNumber)`
    pub fn get_baseline_duration_at(&self, baseline_number: i32) -> Option<Duration> {
        select_field(baseline_duration_fields(), baseline_number)
            .and_then(|field| self.duration_field(field))
    }

    /// 取基线工期文本（多基线）。
    ///
    /// 对应 Java: `public String getBaselineDurationText(int baselineNumber)`
    pub fn get_baseline_duration_text_at(&self, baseline_number: i32) -> Option<String> {
        select_field(baseline_duration_fields(), baseline_number)
            .and_then(|field| self.string_field(field))
    }

    /// 设置基线工期文本（多基线）。
    ///
    /// 对应 Java: `public void setBaselineDurationText(int baselineNumber, String value)`
    pub fn set_baseline_duration_text_at(&mut self, baseline_number: i32, value: Option<String>) {
        if let Some(field) = select_field(baseline_duration_fields(), baseline_number) {
            self.put_string(field, value);
        }
    }

    /// 取基线完成日期（多基线）。
    ///
    /// 对应 Java: `public LocalDateTime getBaselineFinish(int baselineNumber)`
    pub fn get_baseline_finish_at(&self, baseline_number: i32) -> Option<NaiveDateTime> {
        select_field(baseline_finish_fields(), baseline_number)
            .and_then(|field| self.date_time_field(field))
    }

    /// 取基线完成文本（多基线）。
    ///
    /// 对应 Java: `public String getBaselineFinishText(int baselineNumber)`
    pub fn get_baseline_finish_text_at(&self, baseline_number: i32) -> Option<String> {
        select_field(baseline_finish_fields(), baseline_number)
            .and_then(|field| self.string_field(field))
    }

    /// 设置基线完成文本（多基线）。
    ///
    /// 对应 Java: `public void setBaselineFinishText(int baselineNumber, String value)`
    pub fn set_baseline_finish_text_at(&mut self, baseline_number: i32, value: Option<String>) {
        if let Some(field) = select_field(baseline_finish_fields(), baseline_number) {
            self.put_string(field, value);
        }
    }

    /// 取基线开始日期（多基线）。
    ///
    /// 对应 Java: `public LocalDateTime getBaselineStart(int baselineNumber)`
    pub fn get_baseline_start_at(&self, baseline_number: i32) -> Option<NaiveDateTime> {
        select_field(baseline_start_fields(), baseline_number)
            .and_then(|field| self.date_time_field(field))
    }

    /// 取基线开始文本（多基线）。
    ///
    /// 对应 Java: `public String getBaselineStartText(int baselineNumber)`
    pub fn get_baseline_start_text_at(&self, baseline_number: i32) -> Option<String> {
        select_field(baseline_start_fields(), baseline_number)
            .and_then(|field| self.string_field(field))
    }

    /// 设置基线开始文本（多基线）。
    ///
    /// 对应 Java: `public void setBaselineStartText(int baselineNumber, String value)`
    pub fn set_baseline_start_text_at(&mut self, baseline_number: i32, value: Option<String>) {
        if let Some(field) = select_field(baseline_start_fields(), baseline_number) {
            self.put_string(field, value);
        }
    }

    /// 取基线工时（多基线）。
    ///
    /// 对应 Java: `public Duration getBaselineWork(int baselineNumber)`
    pub fn get_baseline_work_at(&self, baseline_number: i32) -> Option<Duration> {
        select_field(baseline_work_fields(), baseline_number)
            .and_then(|field| self.duration_field(field))
    }

    /// 取企业自定义成本字段。
    ///
    /// 对应 Java: `public Number getEnterpriseCost(int index)`
    pub fn get_enterprise_cost(&self, index: i32) -> Option<f64> {
        select_field(enterprise_cost_fields(), index).and_then(|field| self.number_field(field))
    }

    /// 设置企业自定义成本字段。
    ///
    /// 对应 Java: `public void setEnterpriseCost(int index, Number value)`
    pub fn set_enterprise_cost(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = select_field(enterprise_cost_fields(), index) {
            self.put_number(field, value);
        }
    }

    /// 取企业自定义日期字段。
    ///
    /// 对应 Java: `public LocalDateTime getEnterpriseDate(int index)`
    pub fn get_enterprise_date(&self, index: i32) -> Option<NaiveDateTime> {
        select_field(enterprise_date_fields(), index).and_then(|field| self.date_time_field(field))
    }

    /// 设置企业自定义日期字段。
    ///
    /// 对应 Java: `public void setEnterpriseDate(int index, LocalDateTime value)`
    pub fn set_enterprise_date(&mut self, index: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(enterprise_date_fields(), index) {
            self.put_date_time(field, value);
        }
    }

    /// 取企业自定义工期字段。
    ///
    /// 对应 Java: `public Duration getEnterpriseDuration(int index)`
    pub fn get_enterprise_duration(&self, index: i32) -> Option<Duration> {
        select_field(enterprise_duration_fields(), index)
            .and_then(|field| self.duration_field(field))
    }

    /// 设置企业自定义工期字段。
    ///
    /// 对应 Java: `public void setEnterpriseDuration(int index, Duration value)`
    pub fn set_enterprise_duration(&mut self, index: i32, value: Option<Duration>) {
        if let Some(field) = select_field(enterprise_duration_fields(), index) {
            self.put_duration(field, value);
        }
    }

    /// 取企业自定义标志字段。
    ///
    /// 对应 Java: `public boolean getEnterpriseFlag(int index)`
    pub fn get_enterprise_flag(&self, index: i32) -> bool {
        select_field(enterprise_flag_fields(), index).map_or(false, |field| self.boolean_field(field))
    }

    /// 设置企业自定义标志字段。
    ///
    /// 对应 Java: `public void setEnterpriseFlag(int index, boolean value)`
    pub fn set_enterprise_flag(&mut self, index: i32, value: bool) {
        if let Some(field) = select_field(enterprise_flag_fields(), index) {
            self.put_bool(field, value);
        }
    }

    /// 取企业自定义数字字段。
    ///
    /// 对应 Java: `public Number getEnterpriseNumber(int index)`
    pub fn get_enterprise_number(&self, index: i32) -> Option<f64> {
        select_field(enterprise_number_fields(), index).and_then(|field| self.number_field(field))
    }

    /// 设置企业自定义数字字段。
    ///
    /// 对应 Java: `public void setEnterpriseNumber(int index, Number value)`
    pub fn set_enterprise_number(&mut self, index: i32, value: Option<f64>) {
        if let Some(field) = select_field(enterprise_number_fields(), index) {
            self.put_number(field, value);
        }
    }

    /// 取企业自定义文本字段。
    ///
    /// 对应 Java: `public String getEnterpriseText(int index)`
    pub fn get_enterprise_text(&self, index: i32) -> Option<String> {
        select_field(enterprise_text_fields(), index).and_then(|field| self.string_field(field))
    }

    /// 设置企业自定义文本字段。
    ///
    /// 对应 Java: `public void setEnterpriseText(int index, String value)`
    pub fn set_enterprise_text(&mut self, index: i32, value: Option<String>) {
        if let Some(field) = select_field(enterprise_text_fields(), index) {
            self.put_string(field, value);
        }
    }

    /// 设置基线固定成本。
    ///
    /// 对应 Java: `public void setBaselineFixedCost(Number val)`
    pub fn set_baseline_fixed_cost(&mut self, val: Option<f64>) {
        self.put_number(TaskField::BaselineFixedCost, val);
    }

    /// 取基线固定成本。
    ///
    /// 对应 Java: `public Number getBaselineFixedCost()`
    pub fn get_baseline_fixed_cost(&self) -> Option<f64> {
        self.number_field(TaskField::BaselineFixedCost)
    }

    /// 设置基线固定成本（多基线）。
    ///
    /// 对应 Java: `public void setBaselineFixedCost(int baselineNumber, Number value)`
    pub fn set_baseline_fixed_cost_at(&mut self, baseline_number: i32, value: Option<f64>) {
        if let Some(field) = select_field(baseline_fixed_cost_fields(), baseline_number) {
            self.put_number(field, value);
        }
    }

    /// 取基线固定成本（多基线）。
    ///
    /// 对应 Java: `public Number getBaselineFixedCost(int baselineNumber)`
    pub fn get_baseline_fixed_cost_at(&self, baseline_number: i32) -> Option<f64> {
        select_field(baseline_fixed_cost_fields(), baseline_number)
            .and_then(|field| self.number_field(field))
    }

    /// 取基线固定成本计入方式。
    ///
    /// 对应 Java: `public AccrueType getBaselineFixedCostAccrual()`
    pub fn get_baseline_fixed_cost_accrual(&self) -> Option<AccrueType> {
        self.integer_field(TaskField::BaselineFixedCostAccrual)
            .and_then(AccrueType::get_instance)
    }

    /// 设置基线固定成本计入方式。
    ///
    /// 对应 Java: `public void setBaselineFixedCostAccrual(AccrueType type)`
    pub fn set_baseline_fixed_cost_accrual(&mut self, r#type: Option<AccrueType>) {
        self.put_enum(
            TaskField::BaselineFixedCostAccrual,
            r#type.map(|value| value.value()),
        );
    }

    /// 设置基线固定成本计入方式（多基线）。
    ///
    /// 对应 Java: `public void setBaselineFixedCostAccrual(int baselineNumber, AccrueType value)`
    pub fn set_baseline_fixed_cost_accrual_at(
        &mut self,
        baseline_number: i32,
        value: Option<AccrueType>,
    ) {
        if let Some(field) = select_field(baseline_fixed_cost_accrual_fields(), baseline_number) {
            self.put_enum(field, value.map(|value| value.value()));
        }
    }

    /// 取基线固定成本计入方式（多基线）。
    ///
    /// 对应 Java: `public AccrueType getBaselineFixedCostAccrual(int baselineNumber)`
    pub fn get_baseline_fixed_cost_accrual_at(&self, baseline_number: i32) -> Option<AccrueType> {
        select_field(baseline_fixed_cost_accrual_fields(), baseline_number)
            .and_then(|field| self.integer_field(field))
            .and_then(AccrueType::get_instance)
    }

    /// 取基线预算成本。
    ///
    /// 对应 Java: `public Number getBaselineBudgetCost()`
    pub fn get_baseline_budget_cost(&self) -> Option<f64> {
        self.number_field(TaskField::BaselineBudgetCost)
    }

    /// 设置基线预算成本。
    ///
    /// 对应 Java: `public void setBaselineBudgetCost(Number value)`
    pub fn set_baseline_budget_cost(&mut self, value: Option<f64>) {
        self.put_number(TaskField::BaselineBudgetCost, value);
    }

    /// 取基线预算工时。
    ///
    /// 对应 Java: `public Duration getBaselineBudgetWork()`
    pub fn get_baseline_budget_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::BaselineBudgetWork)
    }

    /// 设置基线预算工时。
    ///
    /// 对应 Java: `public void setBaselineBudgetWork(Duration value)`
    pub fn set_baseline_budget_work(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::BaselineBudgetWork, value);
    }

    /// 取基线预算成本（多基线）。
    ///
    /// 对应 Java: `public Number getBaselineBudgetCost(int baselineNumber)`
    pub fn get_baseline_budget_cost_at(&self, baseline_number: i32) -> Option<f64> {
        select_field(baseline_budget_cost_fields(), baseline_number)
            .and_then(|field| self.number_field(field))
    }

    /// 设置基线预算成本（多基线）。
    ///
    /// 对应 Java: `public void setBaselineBudgetCost(int baselineNumber, Number value)`
    pub fn set_baseline_budget_cost_at(&mut self, baseline_number: i32, value: Option<f64>) {
        if let Some(field) = select_field(baseline_budget_cost_fields(), baseline_number) {
            self.put_number(field, value);
        }
    }

    /// 取基线预算工时（多基线）。
    ///
    /// 对应 Java: `public Duration getBaselineBudgetWork(int baselineNumber)`
    pub fn get_baseline_budget_work_at(&self, baseline_number: i32) -> Option<Duration> {
        select_field(baseline_budget_work_fields(), baseline_number)
            .and_then(|field| self.duration_field(field))
    }

    /// 设置基线预算工时（多基线）。
    ///
    /// 对应 Java: `public void setBaselineBudgetWork(int baselineNumber, Duration value)`
    pub fn set_baseline_budget_work_at(&mut self, baseline_number: i32, value: Option<Duration>) {
        if let Some(field) = select_field(baseline_budget_work_fields(), baseline_number) {
            self.put_duration(field, value);
        }
    }
}

// ---------------------------------------------------------------------------
// 基线估算值 / 完成进度 / GUID / 任务模式
// ---------------------------------------------------------------------------

impl Task {
    /// 取基线估算工期。
    ///
    /// 对应 Java: `public Duration getBaselineEstimatedDuration()`
    pub fn get_baseline_estimated_duration(&self) -> Option<Duration> {
        self.duration_field(TaskField::BaselineEstimatedDuration)
    }

    /// 设置基线估算工期。
    ///
    /// 对应 Java: `public void setBaselineEstimatedDuration(Duration duration)`
    pub fn set_baseline_estimated_duration(&mut self, duration: Option<Duration>) {
        self.put_duration(TaskField::BaselineEstimatedDuration, duration);
    }

    /// 设置基线估算工期（多基线）。
    ///
    /// 对应 Java: `public void setBaselineEstimatedDuration(int baselineNumber, Duration value)`
    pub fn set_baseline_estimated_duration_at(&mut self, baseline_number: i32, value: Option<Duration>) {
        if let Some(field) = select_field(baseline_estimated_duration_fields(), baseline_number) {
            self.put_duration(field, value);
        }
    }

    /// 取基线估算工期（多基线）。
    ///
    /// 对应 Java: `public Duration getBaselineEstimatedDuration(int baselineNumber)`
    pub fn get_baseline_estimated_duration_at(&self, baseline_number: i32) -> Option<Duration> {
        select_field(baseline_estimated_duration_fields(), baseline_number)
            .and_then(|field| self.duration_field(field))
    }

    /// 取基线估算开始日期。
    ///
    /// 对应 Java: `public LocalDateTime getBaselineEstimatedStart()`
    pub fn get_baseline_estimated_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::BaselineEstimatedStart)
    }

    /// 设置基线估算开始日期。
    ///
    /// 对应 Java: `public void setBaselineEstimatedStart(LocalDateTime date)`
    pub fn set_baseline_estimated_start(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::BaselineEstimatedStart, date);
    }

    /// 取基线估算开始日期（多基线）。
    ///
    /// 对应 Java: `public LocalDateTime getBaselineEstimatedStart(int baselineNumber)`
    pub fn get_baseline_estimated_start_at(&self, baseline_number: i32) -> Option<NaiveDateTime> {
        select_field(baseline_estimated_start_fields(), baseline_number)
            .and_then(|field| self.date_time_field(field))
    }

    /// 设置基线估算开始日期（多基线）。
    ///
    /// 对应 Java: `public void setBaselineEstimatedStart(int baselineNumber, LocalDateTime value)`
    pub fn set_baseline_estimated_start_at(&mut self, baseline_number: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(baseline_estimated_start_fields(), baseline_number) {
            self.put_date_time(field, value);
        }
    }

    /// 取基线估算完成日期。
    ///
    /// 对应 Java: `public LocalDateTime getBaselineEstimatedFinish()`
    pub fn get_baseline_estimated_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::BaselineEstimatedFinish)
    }

    /// 设置基线估算完成日期。
    ///
    /// 对应 Java: `public void setBaselineEstimatedFinish(LocalDateTime date)`
    pub fn set_baseline_estimated_finish(&mut self, date: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::BaselineEstimatedFinish, date);
    }

    /// 取基线估算完成日期（多基线）。
    ///
    /// 对应 Java: `public LocalDateTime getBaselineEstimatedFinish(int baselineNumber)`
    pub fn get_baseline_estimated_finish_at(&self, baseline_number: i32) -> Option<NaiveDateTime> {
        select_field(baseline_estimated_finish_fields(), baseline_number)
            .and_then(|field| self.date_time_field(field))
    }

    /// 设置基线估算完成日期（多基线）。
    ///
    /// 对应 Java: `public void setBaselineEstimatedFinish(int baselineNumber, LocalDateTime value)`
    pub fn set_baseline_estimated_finish_at(&mut self, baseline_number: i32, value: Option<NaiveDateTime>) {
        if let Some(field) = select_field(baseline_estimated_finish_fields(), baseline_number) {
            self.put_date_time(field, value);
        }
    }

    /// 取「完成到」日期。
    ///
    /// 对应 Java: `public LocalDateTime getCompleteThrough()`
    pub fn get_complete_through(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::CompleteThrough)
    }

    /// 设置「完成到」日期。
    ///
    /// 对应 Java: `public void setCompleteThrough(LocalDateTime value)`
    pub fn set_complete_through(&mut self, value: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::CompleteThrough, value);
    }

    /// 取汇总进度日期。
    ///
    /// 对应 Java: `public LocalDateTime getSummaryProgress()`
    pub fn get_summary_progress(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::SummaryProgress)
    }

    /// 设置汇总进度日期。
    ///
    /// 对应 Java: `public void setSummaryProgress(LocalDateTime value)`
    pub fn set_summary_progress(&mut self, value: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::SummaryProgress, value);
    }

    /// 取任务 GUID。
    ///
    /// 对应 Java: `public UUID getGUID()`
    pub fn get_guid(&self) -> Option<Uuid> {
        self.uuid_field(TaskField::Guid)
    }

    /// 设置任务 GUID。
    ///
    /// 对应 Java: `public void setGUID(UUID value)`
    pub fn set_guid(&mut self, value: Option<Uuid>) {
        self.put_uuid(TaskField::Guid, value);
    }

    /// 取任务模式。
    ///
    /// 对应 Java: `public TaskMode getTaskMode()`
    pub fn get_task_mode(&self) -> Option<TaskMode> {
        self.integer_field(TaskField::TaskMode)
            .map(TaskMode::get_instance)
    }

    /// 设置任务模式。
    ///
    /// 对应 Java: `public void setTaskMode(TaskMode mode)`
    pub fn set_task_mode(&mut self, mode: Option<TaskMode>) {
        self.put_enum(TaskField::TaskMode, mode.map(|value| value.value()));
    }

    /// 取活动标志。
    ///
    /// 对应 Java: `public boolean getActive()`
    pub fn get_active(&self) -> bool {
        self.boolean_field(TaskField::Active)
    }

    /// 设置活动标志。
    ///
    /// 对应 Java: `public void setActive(boolean active)`
    pub fn set_active(&mut self, active: bool) {
        self.put_bool(TaskField::Active, active);
    }

    /// 取活动 ID 的「规范化」版本（汇总任务用 `PROJECT` 替换项目 ID 前缀）。
    ///
    /// 对应 Java: `public String getCanonicalActivityID()`
    pub fn get_canonical_activity_id(&self) -> Option<String> {
        let activity_id = self.get_activity_id();
        if !self.get_summary() {
            return activity_id;
        }

        let Some(activity_id) = activity_id else {
            return None;
        };

        let project_id = self
            .parent_file
            .upgrade()
            .and_then(|file| file.borrow().get_project_properties().get_project_id());

        match project_id {
            Some(project_id) if !project_id.is_empty() && activity_id.starts_with(&project_id) => {
                Some(format!("PROJECT{}", &activity_id[project_id.len()..]))
            }
            _ => Some(activity_id),
        }
    }

    /// 取活动 ID。
    ///
    /// 对应 Java: `public String getActivityID()`
    pub fn get_activity_id(&self) -> Option<String> {
        self.string_field(TaskField::ActivityId)
    }

    /// 设置活动 ID。
    ///
    /// 对应 Java: `public void setActivityID(String value)`
    pub fn set_activity_id(&mut self, value: Option<String>) {
        self.put_string(TaskField::ActivityId, value);
    }

    /// 设置完成百分比类型。
    ///
    /// 对应 Java: `public void setPercentCompleteType(PercentCompleteType value)`
    pub fn set_percent_complete_type(&mut self, value: Option<PercentCompleteType>) {
        self.put_enum(
            TaskField::PercentCompleteType,
            value.map(|value| value.value()),
        );
    }

    /// 取完成百分比类型。
    ///
    /// 对应 Java: `public PercentCompleteType getPercentCompleteType()`
    pub fn get_percent_complete_type(&self) -> Option<PercentCompleteType> {
        self.integer_field(TaskField::PercentCompleteType)
            .and_then(PercentCompleteType::get_instance)
    }

    /// 取活动状态。
    ///
    /// 对应 Java: `public ActivityStatus getActivityStatus()`
    pub fn get_activity_status(&self) -> Option<ActivityStatus> {
        self.integer_field(TaskField::ActivityStatus)
            .and_then(ActivityStatus::get_instance)
    }

    /// 设置活动状态。
    ///
    /// 对应 Java: `public void setActivityStatus(ActivityStatus value)`
    pub fn set_activity_status(&mut self, value: Option<ActivityStatus>) {
        self.put_enum(TaskField::ActivityStatus, value.map(|value| value.value()));
    }

    /// 取活动类型。
    ///
    /// 对应 Java: `public ActivityType getActivityType()`
    pub fn get_activity_type(&self) -> Option<ActivityType> {
        self.integer_field(TaskField::ActivityType)
            .and_then(ActivityType::get_instance)
    }

    /// 设置活动类型。
    ///
    /// 对应 Java: `public void setActivityType(ActivityType value)`
    pub fn set_activity_type(&mut self, value: Option<ActivityType>) {
        self.put_enum(TaskField::ActivityType, value.map(|value| value.value()));
    }

    /// 取「属于最长路径」标志。
    ///
    /// 对应 Java: `public boolean getLongestPath()`
    pub fn get_longest_path(&self) -> bool {
        self.boolean_field(TaskField::LongestPath)
    }

    /// 设置「属于最长路径」标志。
    ///
    /// 对应 Java: `public void setLongestPath(boolean value)`
    pub fn set_longest_path(&mut self, value: bool) {
        self.put_bool(TaskField::LongestPath, value);
    }

    /// 取外部最早开始日期。
    ///
    /// 对应 Java: `public LocalDateTime getExternalEarlyStart()`
    pub fn get_external_early_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::ExternalEarlyStart)
    }

    /// 设置外部最早开始日期。
    ///
    /// 对应 Java: `public void setExternalEarlyStart(LocalDateTime value)`
    pub fn set_external_early_start(&mut self, value: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::ExternalEarlyStart, value);
    }

    /// 取外部最晚完成日期。
    ///
    /// 对应 Java: `public LocalDateTime getExternalLateFinish()`
    pub fn get_external_late_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::ExternalLateFinish)
    }

    /// 设置外部最晚完成日期。
    ///
    /// 对应 Java: `public void setExternalLateFinish(LocalDateTime value)`
    pub fn set_external_late_finish(&mut self, value: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::ExternalLateFinish, value);
    }

    /// 取 Sprint ID。
    ///
    /// 对应 Java: `public Integer getSprintID()`
    pub fn get_sprint_id(&self) -> Option<i32> {
        self.integer_field(TaskField::SprintId)
    }

    /// 设置 Sprint ID。
    ///
    /// 对应 Java: `public void setSprintID(Integer value)`
    pub fn set_sprint_id(&mut self, value: Option<i32>) {
        self.put_integer(TaskField::SprintId, value);
    }

    /// 取看板状态 ID。
    ///
    /// 对应 Java: `public Integer getBoardStatusID()`
    pub fn get_board_status_id(&self) -> Option<i32> {
        self.integer_field(TaskField::BoardStatusId)
    }

    /// 设置看板状态 ID。
    ///
    /// 对应 Java: `public void setBoardStatusID(Integer value)`
    pub fn set_board_status_id(&mut self, value: Option<i32>) {
        self.put_integer(TaskField::BoardStatusId, value);
    }

    /// 取「等待响应」标志。
    ///
    /// 对应 Java: `public boolean getResponsePending()`
    pub fn get_response_pending(&self) -> bool {
        self.boolean_field(TaskField::ResponsePending)
    }

    /// 设置「等待响应」标志。
    ///
    /// 对应 Java: `public void setResponsePending(boolean value)`
    pub fn set_response_pending(&mut self, value: bool) {
        self.put_bool(TaskField::ResponsePending, value);
    }

    /// 取计划开始日期。
    ///
    /// 对应 Java: `public LocalDateTime getScheduledStart()`
    pub fn get_scheduled_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::ScheduledStart)
    }

    /// 设置计划开始日期。
    ///
    /// 对应 Java: `public void setScheduledStart(LocalDateTime value)`
    pub fn set_scheduled_start(&mut self, value: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::ScheduledStart, value);
    }

    /// 取计划完成日期。
    ///
    /// 对应 Java: `public LocalDateTime getScheduledFinish()`
    pub fn get_scheduled_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::ScheduledFinish)
    }

    /// 设置计划完成日期。
    ///
    /// 对应 Java: `public void setScheduledFinish(LocalDateTime value)`
    pub fn set_scheduled_finish(&mut self, value: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::ScheduledFinish, value);
    }

    /// 取计划工期。
    ///
    /// 对应 Java: `public Duration getScheduledDuration()`
    pub fn get_scheduled_duration(&self) -> Option<Duration> {
        self.duration_field(TaskField::ScheduledDuration)
    }

    /// 设置计划工期。
    ///
    /// 对应 Java: `public void setScheduledDuration(Duration value)`
    pub fn set_scheduled_duration(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::ScheduledDuration, value);
    }

    /// 取预算成本。
    ///
    /// 对应 Java: `public Number getBudgetCost()`
    pub fn get_budget_cost(&self) -> Option<f64> {
        self.number_field(TaskField::BudgetCost)
    }

    /// 设置预算成本。
    ///
    /// 对应 Java: `public void setBudgetCost(Number value)`
    pub fn set_budget_cost(&mut self, value: Option<f64>) {
        self.put_number(TaskField::BudgetCost, value);
    }

    /// 取预算工时。
    ///
    /// 对应 Java: `public Duration getBudgetWork()`
    pub fn get_budget_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::BudgetWork)
    }

    /// 设置预算工时。
    ///
    /// 对应 Java: `public void setBudgetWork(Duration value)`
    pub fn set_budget_work(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::BudgetWork, value);
    }

    /// 设置任务序号。
    ///
    /// 对应 Java: `public void setSequenceNumber(Integer sequenceNumber)`
    pub fn set_sequence_number(&mut self, sequence_number: Option<i32>) {
        self.put_integer(TaskField::SequenceNumber, sequence_number);
    }

    /// 取任务序号。
    ///
    /// 对应 Java: `public Integer getSequenceNumber()`
    pub fn get_sequence_number(&self) -> Option<i32> {
        self.integer_field(TaskField::SequenceNumber)
    }

    /// 取位置唯一 ID。
    ///
    /// 对应 Java: `public Integer getLocationUniqueID()`
    pub fn get_location_unique_id(&self) -> Option<i32> {
        self.integer_field(TaskField::LocationUniqueId)
    }

    /// 设置位置唯一 ID。
    ///
    /// 对应 Java: `public void setLocationUniqueID(Integer uniqueID)`
    pub fn set_location_unique_id(&mut self, unique_id: Option<i32>) {
        self.put_integer(TaskField::LocationUniqueId, unique_id);
    }

    /// 取 Asta Powerproject 甘特条名称。
    ///
    /// 对应 Java: `public String getBarName()`
    pub fn get_bar_name(&self) -> Option<String> {
        self.string_field(TaskField::BarName)
    }

    /// 设置 Asta Powerproject 甘特条名称。
    ///
    /// 对应 Java: `public void setBarName(String value)`
    pub fn set_bar_name(&mut self, value: Option<String>) {
        self.put_string(TaskField::BarName, value);
    }

    /// 取预计完成日期。
    ///
    /// 对应 Java: `public LocalDateTime getExpectedFinish()`
    pub fn get_expected_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::ExpectedFinish)
    }

    /// 设置预计完成日期。
    ///
    /// 对应 Java: `public void setExpectedFinish(LocalDateTime value)`
    pub fn set_expected_finish(&mut self, value: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::ExpectedFinish, value);
    }
}

// ---------------------------------------------------------------------------
// 工时 / 成本的构成（人工、非人工、材料、费用）
// ---------------------------------------------------------------------------

impl Task {
    /// 设置实际工时中的人工部分。
    ///
    /// 对应 Java: `public void setActualWorkLabor(Duration value)`
    pub fn set_actual_work_labor(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::ActualWorkLabor, value);
    }

    /// 取实际工时中的人工部分。
    ///
    /// 对应 Java: `public Duration getActualWorkLabor()`
    pub fn get_actual_work_labor(&self) -> Option<Duration> {
        self.duration_field(TaskField::ActualWorkLabor)
    }

    /// 设置实际工时中的非人工部分（已废弃的 Java 旧方法名）。
    ///
    /// 对应 Java: `@Deprecated public void setActualWorkNonlabor(Duration value)`
    pub fn set_actual_work_nonlabor(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::ActualWorkNonlabor, value);
    }

    /// 设置实际工时中的非人工部分。
    ///
    /// 对应 Java: `public void setActualWorkNonLabor(Duration value)`
    pub fn set_actual_work_non_labor(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::ActualWorkNonlabor, value);
    }

    /// 取实际工时中的非人工部分（已废弃的 Java 旧方法名）。
    ///
    /// 对应 Java: `@Deprecated public Duration getActualWorkNonlabor()`
    pub fn get_actual_work_nonlabor(&self) -> Option<Duration> {
        self.duration_field(TaskField::ActualWorkNonlabor)
    }

    /// 取实际工时中的非人工部分。
    ///
    /// 对应 Java: `public Duration getActualWorkNonLabor()`
    pub fn get_actual_work_non_labor(&self) -> Option<Duration> {
        self.duration_field(TaskField::ActualWorkNonlabor)
    }

    /// 设置计划工时中的人工部分。
    ///
    /// 对应 Java: `public void setPlannedWorkLabor(Duration value)`
    pub fn set_planned_work_labor(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::PlannedWorkLabor, value);
    }

    /// 取计划工时中的人工部分。
    ///
    /// 对应 Java: `public Duration getPlannedWorkLabor()`
    pub fn get_planned_work_labor(&self) -> Option<Duration> {
        self.duration_field(TaskField::PlannedWorkLabor)
    }

    /// 设置计划工时中的非人工部分（已废弃的 Java 旧方法名）。
    ///
    /// 对应 Java: `@Deprecated public void setPlannedWorkNonlabor(Duration value)`
    pub fn set_planned_work_nonlabor(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::PlannedWorkNonlabor, value);
    }

    /// 设置计划工时中的非人工部分。
    ///
    /// 对应 Java: `public void setPlannedWorkNonLabor(Duration value)`
    pub fn set_planned_work_non_labor(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::PlannedWorkNonlabor, value);
    }

    /// 取计划工时中的非人工部分（已废弃的 Java 旧方法名）。
    ///
    /// 对应 Java: `@Deprecated public Duration getPlannedWorkNonlabor()`
    pub fn get_planned_work_nonlabor(&self) -> Option<Duration> {
        self.duration_field(TaskField::PlannedWorkNonlabor)
    }

    /// 取计划工时中的非人工部分。
    ///
    /// 对应 Java: `public Duration getPlannedWorkNonLabor()`
    pub fn get_planned_work_non_labor(&self) -> Option<Duration> {
        self.duration_field(TaskField::PlannedWorkNonlabor)
    }

    /// 设置剩余工时中的人工部分。
    ///
    /// 对应 Java: `public void setRemainingWorkLabor(Duration value)`
    pub fn set_remaining_work_labor(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::RemainingWorkLabor, value);
    }

    /// 取剩余工时中的人工部分。
    ///
    /// 对应 Java: `public Duration getRemainingWorkLabor()`
    pub fn get_remaining_work_labor(&self) -> Option<Duration> {
        self.duration_field(TaskField::RemainingWorkLabor)
    }

    /// 设置剩余工时中的非人工部分（已废弃的 Java 旧方法名）。
    ///
    /// 对应 Java: `@Deprecated public void setRemainingWorkNonlabor(Duration value)`
    pub fn set_remaining_work_nonlabor(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::RemainingWorkNonlabor, value);
    }

    /// 设置剩余工时中的非人工部分。
    ///
    /// 对应 Java: `public void setRemainingWorkNonLabor(Duration value)`
    pub fn set_remaining_work_non_labor(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::RemainingWorkNonlabor, value);
    }

    /// 取剩余工时中的非人工部分（已废弃的 Java 旧方法名）。
    ///
    /// 对应 Java: `@Deprecated public Duration getRemainingWorkNonlabor()`
    pub fn get_remaining_work_nonlabor(&self) -> Option<Duration> {
        self.duration_field(TaskField::RemainingWorkNonlabor)
    }

    /// 取剩余工时中的非人工部分。
    ///
    /// 对应 Java: `public Duration getRemainingWorkNonLabor()`
    pub fn get_remaining_work_non_labor(&self) -> Option<Duration> {
        self.duration_field(TaskField::RemainingWorkNonlabor)
    }

    /// 判断手排任务是否应显示开始文本。
    ///
    /// 对应 Java: `public boolean getShowStartText()`
    pub fn get_show_start_text(&self) -> bool {
        self.boolean_field(TaskField::ShowStartText)
    }

    /// 判断手排任务是否应显示完成文本。
    ///
    /// 对应 Java: `public boolean getShowFinishText()`
    pub fn get_show_finish_text(&self) -> bool {
        self.boolean_field(TaskField::ShowFinishText)
    }

    /// 判断手排任务是否应显示工期文本。
    ///
    /// 对应 Java: `public boolean getShowDurationText()`
    pub fn get_show_duration_text(&self) -> bool {
        self.boolean_field(TaskField::ShowDurationText)
    }

    /// 取按完成百分比类型确定的完成百分比。
    ///
    /// 对应 Java: `public Number getActivityPercentComplete()`
    pub fn get_activity_percent_complete(&self) -> Option<f64> {
        self.number_field(TaskField::ActivityPercentComplete)
    }

    /// 取方法论 GUID。
    ///
    /// 对应 Java: `public UUID getMethodologyGUID()`
    pub fn get_methodology_guid(&self) -> Option<Uuid> {
        self.uuid_field(TaskField::MethodologyGuid)
    }

    /// 设置方法论 GUID。
    ///
    /// 对应 Java: `public void setMethodologyGUID(UUID value)`
    pub fn set_methodology_guid(&mut self, value: Option<Uuid>) {
        self.put_uuid(TaskField::MethodologyGuid, value);
    }

    /// 取浮动路径号。
    ///
    /// 对应 Java: `public Integer getFloatPath()`
    pub fn get_float_path(&self) -> Option<i32> {
        self.integer_field(TaskField::FloatPath)
    }

    /// 设置浮动路径号。
    ///
    /// 对应 Java: `public void setFloatPath(Integer value)`
    pub fn set_float_path(&mut self, value: Option<i32>) {
        self.put_integer(TaskField::FloatPath, value);
    }

    /// 取浮动路径顺序。
    ///
    /// 对应 Java: `public Integer getFloatPathOrder()`
    pub fn get_float_path_order(&self) -> Option<i32> {
        self.integer_field(TaskField::FloatPathOrder)
    }

    /// 设置浮动路径顺序。
    ///
    /// 对应 Java: `public void setFloatPathOrder(Integer value)`
    pub fn set_float_path_order(&mut self, value: Option<i32>) {
        self.put_integer(TaskField::FloatPathOrder, value);
    }

    /// 取原始预算。
    ///
    /// 对应 Java: `public Number getOriginalBudget()`
    pub fn get_original_budget(&self) -> Option<f64> {
        self.number_field(TaskField::OriginalBudget)
    }

    /// 设置原始预算。
    ///
    /// 对应 Java: `public void setOriginalBudget(Number originalBudget)`
    pub fn set_original_budget(&mut self, original_budget: Option<f64>) {
        self.put_number(TaskField::OriginalBudget, original_budget);
    }

    /// 取进度百分比完成度。
    ///
    /// 对应 Java: `public Number getSchedulePercentComplete()`
    pub fn get_schedule_percent_complete(&self) -> Option<f64> {
        self.number_field(TaskField::SchedulePercentComplete)
    }

    /// 取有效日历：任务自身日历为空时回落到项目默认日历。
    ///
    /// 对应 Java: `public ProjectCalendar getEffectiveCalendar()`
    pub fn get_effective_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        if let Some(calendar) = self.get_calendar() {
            return Some(calendar);
        }

        self.parent_file
            .upgrade()
            .and_then(|file| file.borrow().get_default_calendar())
    }

    /// 取实际常规工时。
    ///
    /// 对应 Java: `public Duration getActualRegularWork()`
    pub fn get_actual_regular_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::ActualRegularWork)
    }

    /// 取剩余常规工时。
    ///
    /// 对应 Java: `public Duration getRemainingRegularWork()`
    pub fn get_remaining_regular_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::RemainingRegularWork)
    }

    /// 取实际常规成本。
    ///
    /// 对应 Java: `public Number getActualRegularCost()`
    pub fn get_actual_regular_cost(&self) -> Option<f64> {
        self.number_field(TaskField::ActualRegularCost)
    }

    /// 取剩余常规成本。
    ///
    /// 对应 Java: `public Number getRemainingRegularCost()`
    pub fn get_remaining_regular_cost(&self) -> Option<f64> {
        self.number_field(TaskField::RemainingRegularCost)
    }

    /// 设置计划成本中的人工部分。
    ///
    /// 对应 Java: `public void setPlannedCostLabor(Number value)`
    pub fn set_planned_cost_labor(&mut self, value: Option<f64>) {
        self.put_number(TaskField::PlannedCostLabor, value);
    }

    /// 取计划成本中的人工部分。
    ///
    /// 对应 Java: `public Number getPlannedCostLabor()`
    pub fn get_planned_cost_labor(&self) -> Option<f64> {
        self.number_field(TaskField::PlannedCostLabor)
    }

    /// 设置计划成本中的非人工部分。
    ///
    /// 对应 Java: `public void setPlannedCostNonLabor(Number value)`
    pub fn set_planned_cost_non_labor(&mut self, value: Option<f64>) {
        self.put_number(TaskField::PlannedCostNonLabor, value);
    }

    /// 取计划成本中的非人工部分。
    ///
    /// 对应 Java: `public Number getPlannedCostNonLabor()`
    pub fn get_planned_cost_non_labor(&self) -> Option<f64> {
        self.number_field(TaskField::PlannedCostNonLabor)
    }

    /// 设置计划成本中的材料部分。
    ///
    /// 对应 Java: `public void setPlannedCostMaterial(Number value)`
    pub fn set_planned_cost_material(&mut self, value: Option<f64>) {
        self.put_number(TaskField::PlannedCostMaterial, value);
    }

    /// 取计划成本中的材料部分。
    ///
    /// 对应 Java: `public Number getPlannedCostMaterial()`
    pub fn get_planned_cost_material(&self) -> Option<f64> {
        self.number_field(TaskField::PlannedCostMaterial)
    }

    /// 设置计划成本中的费用部分。
    ///
    /// 对应 Java: `public void setPlannedCostExpense(Number value)`
    pub fn set_planned_cost_expense(&mut self, value: Option<f64>) {
        self.put_number(TaskField::PlannedCostExpense, value);
    }

    /// 取计划成本中的费用部分。
    ///
    /// 对应 Java: `public Number getPlannedCostExpense()`
    pub fn get_planned_cost_expense(&self) -> Option<f64> {
        self.number_field(TaskField::PlannedCostExpense)
    }

    /// 设置实际成本中的人工部分。
    ///
    /// 对应 Java: `public void setActualCostLabor(Number value)`
    pub fn set_actual_cost_labor(&mut self, value: Option<f64>) {
        self.put_number(TaskField::ActualCostLabor, value);
    }

    /// 取实际成本中的人工部分。
    ///
    /// 对应 Java: `public Number getActualCostLabor()`
    pub fn get_actual_cost_labor(&self) -> Option<f64> {
        self.number_field(TaskField::ActualCostLabor)
    }

    /// 设置实际成本中的非人工部分。
    ///
    /// 对应 Java: `public void setActualCostNonLabor(Number value)`
    pub fn set_actual_cost_non_labor(&mut self, value: Option<f64>) {
        self.put_number(TaskField::ActualCostNonLabor, value);
    }

    /// 取实际成本中的非人工部分。
    ///
    /// 对应 Java: `public Number getActualCostNonLabor()`
    pub fn get_actual_cost_non_labor(&self) -> Option<f64> {
        self.number_field(TaskField::ActualCostNonLabor)
    }

    /// 设置实际成本中的材料部分。
    ///
    /// 对应 Java: `public void setActualCostMaterial(Number value)`
    pub fn set_actual_cost_material(&mut self, value: Option<f64>) {
        self.put_number(TaskField::ActualCostMaterial, value);
    }

    /// 取实际成本中的材料部分。
    ///
    /// 对应 Java: `public Number getActualCostMaterial()`
    pub fn get_actual_cost_material(&self) -> Option<f64> {
        self.number_field(TaskField::ActualCostMaterial)
    }

    /// 设置实际成本中的费用部分。
    ///
    /// 对应 Java: `public void setActualCostExpense(Number value)`
    pub fn set_actual_cost_expense(&mut self, value: Option<f64>) {
        self.put_number(TaskField::ActualCostExpense, value);
    }

    /// 取实际成本中的费用部分。
    ///
    /// 对应 Java: `public Number getActualCostExpense()`
    pub fn get_actual_cost_expense(&self) -> Option<f64> {
        self.number_field(TaskField::ActualCostExpense)
    }

    /// 设置剩余成本中的人工部分。
    ///
    /// 对应 Java: `public void setRemainingCostLabor(Number value)`
    pub fn set_remaining_cost_labor(&mut self, value: Option<f64>) {
        self.put_number(TaskField::RemainingCostLabor, value);
    }

    /// 取剩余成本中的人工部分。
    ///
    /// 对应 Java: `public Number getRemainingCostLabor()`
    pub fn get_remaining_cost_labor(&self) -> Option<f64> {
        self.number_field(TaskField::RemainingCostLabor)
    }

    /// 设置剩余成本中的非人工部分。
    ///
    /// 对应 Java: `public void setRemainingCostNonLabor(Number value)`
    pub fn set_remaining_cost_non_labor(&mut self, value: Option<f64>) {
        self.put_number(TaskField::RemainingCostNonLabor, value);
    }

    /// 取剩余成本中的非人工部分。
    ///
    /// 对应 Java: `public Number getRemainingCostNonLabor()`
    pub fn get_remaining_cost_non_labor(&self) -> Option<f64> {
        self.number_field(TaskField::RemainingCostNonLabor)
    }

    /// 设置剩余成本中的材料部分。
    ///
    /// 对应 Java: `public void setRemainingCostMaterial(Number value)`
    pub fn set_remaining_cost_material(&mut self, value: Option<f64>) {
        self.put_number(TaskField::RemainingCostMaterial, value);
    }

    /// 取剩余成本中的材料部分。
    ///
    /// 对应 Java: `public Number getRemainingCostMaterial()`
    pub fn get_remaining_cost_material(&self) -> Option<f64> {
        self.number_field(TaskField::RemainingCostMaterial)
    }

    /// 设置剩余成本中的费用部分。
    ///
    /// 对应 Java: `public void setRemainingCostExpense(Number value)`
    pub fn set_remaining_cost_expense(&mut self, value: Option<f64>) {
        self.put_number(TaskField::RemainingCostExpense, value);
    }

    /// 取剩余成本中的费用部分。
    ///
    /// 对应 Java: `public Number getRemainingCostExpense()`
    pub fn get_remaining_cost_expense(&self) -> Option<f64> {
        self.number_field(TaskField::RemainingCostExpense)
    }
}

// ---------------------------------------------------------------------------
// 剩余工程量与 Asta / Primavera 扩展字段
// ---------------------------------------------------------------------------

impl Task {
    /// 设置已存储材料量。
    ///
    /// 对应 Java: `public void setStoredMaterial(Number value)`
    pub fn set_stored_material(&mut self, value: Option<f64>) {
        self.put_number(TaskField::StoredMaterial, value);
    }

    /// 取已存储材料量。
    ///
    /// 对应 Java: `public Number getStoredMaterial()`
    pub fn get_stored_material(&self) -> Option<f64> {
        self.number_field(TaskField::StoredMaterial)
    }

    /// 设置工程项（feature of work）。
    ///
    /// 对应 Java: `public void setFeatureOfWork(String value)`
    pub fn set_feature_of_work(&mut self, value: Option<String>) {
        self.put_string(TaskField::FeatureOfWork, value);
    }

    /// 取工程项（feature of work）。
    ///
    /// 对应 Java: `public String getFeatureOfWork()`
    pub fn get_feature_of_work(&self) -> Option<String> {
        self.string_field(TaskField::FeatureOfWork)
    }

    /// 设置工作类别。
    ///
    /// 对应 Java: `public void setCategoryOfWork(String value)`
    pub fn set_category_of_work(&mut self, value: Option<String>) {
        self.put_string(TaskField::CategoryOfWork, value);
    }

    /// 取工作类别。
    ///
    /// 对应 Java: `public String getCategoryOfWork()`
    pub fn get_category_of_work(&self) -> Option<String> {
        self.string_field(TaskField::CategoryOfWork)
    }

    /// 设置工作阶段。
    ///
    /// 对应 Java: `public void setPhaseOfWork(String value)`
    pub fn set_phase_of_work(&mut self, value: Option<String>) {
        self.put_string(TaskField::PhaseOfWork, value);
    }

    /// 取工作阶段。
    ///
    /// 对应 Java: `public String getPhaseOfWork()`
    pub fn get_phase_of_work(&self) -> Option<String> {
        self.string_field(TaskField::PhaseOfWork)
    }

    /// 设置投标项。
    ///
    /// 对应 Java: `public void setBidItem(String value)`
    pub fn set_bid_item(&mut self, value: Option<String>) {
        self.put_string(TaskField::BidItem, value);
    }

    /// 取投标项。
    ///
    /// 对应 Java: `public String getBidItem()`
    pub fn get_bid_item(&self) -> Option<String> {
        self.string_field(TaskField::BidItem)
    }

    /// 设置变更 / 索赔编号。
    ///
    /// 对应 Java: `public void setModOrClaimNumber(String value)`
    pub fn set_mod_or_claim_number(&mut self, value: Option<String>) {
        self.put_string(TaskField::ModOrClaimNumber, value);
    }

    /// 取变更 / 索赔编号。
    ///
    /// 对应 Java: `public String getModOrClaimNumber()`
    pub fn get_mod_or_claim_number(&self) -> Option<String> {
        self.string_field(TaskField::ModOrClaimNumber)
    }

    /// 设置工区代码。
    ///
    /// 对应 Java: `public void setWorkAreaCode(String value)`
    pub fn set_work_area_code(&mut self, value: Option<String>) {
        self.put_string(TaskField::WorkAreaCode, value);
    }

    /// 取工区代码。
    ///
    /// 对应 Java: `public String getWorkAreaCode()`
    pub fn get_work_area_code(&self) -> Option<String> {
        self.string_field(TaskField::WorkAreaCode)
    }

    /// 设置责任代码。
    ///
    /// 对应 Java: `public void setResponsibilityCode(String value)`
    pub fn set_responsibility_code(&mut self, value: Option<String>) {
        self.put_string(TaskField::ResponsibilityCode, value);
    }

    /// 取责任代码。
    ///
    /// 对应 Java: `public String getResponsibilityCode()`
    pub fn get_responsibility_code(&self) -> Option<String> {
        self.string_field(TaskField::ResponsibilityCode)
    }

    /// 设置每日工人数。
    ///
    /// 对应 Java: `public void setWorkersPerDay(Integer value)`
    pub fn set_workers_per_day(&mut self, value: Option<i32>) {
        self.put_integer(TaskField::WorkersPerDay, value);
    }

    /// 取每日工人数。
    ///
    /// 对应 Java: `public Integer getWorkersPerDay()`
    pub fn get_workers_per_day(&self) -> Option<i32> {
        self.integer_field(TaskField::WorkersPerDay)
    }

    /// 设置吊床代码标志。
    ///
    /// 对应 Java: `public void setHammockCode(boolean value)`
    pub fn set_hammock_code(&mut self, value: bool) {
        self.put_bool(TaskField::HammockCode, value);
    }

    /// 取吊床代码标志。
    ///
    /// 对应 Java: `public boolean getHammockCode()`
    pub fn get_hammock_code(&self) -> bool {
        self.boolean_field(TaskField::HammockCode)
    }

    /// 设置邮件字段。
    ///
    /// 对应 Java: `public void setMail(String value)`
    pub fn set_mail(&mut self, value: Option<String>) {
        self.put_string(TaskField::Mail, value);
    }

    /// 取邮件字段。
    ///
    /// 对应 Java: `public String getMail()`
    pub fn get_mail(&self) -> Option<String> {
        self.string_field(TaskField::Mail)
    }

    /// 设置分部字段。
    ///
    /// 对应 Java: `public void setSection(String value)`
    pub fn set_section(&mut self, value: Option<String>) {
        self.put_string(TaskField::Section, value);
    }

    /// 取分部字段。
    ///
    /// 对应 Java: `public String getSection()`
    pub fn get_section(&self) -> Option<String> {
        self.string_field(TaskField::Section)
    }

    /// 设置经理字段。
    ///
    /// 对应 Java: `public void setManager(String value)`
    pub fn set_manager(&mut self, value: Option<String>) {
        self.put_string(TaskField::Manager, value);
    }

    /// 取经理字段。
    ///
    /// 对应 Java: `public String getManager()`
    pub fn get_manager(&self) -> Option<String> {
        self.string_field(TaskField::Manager)
    }

    /// 设置部门字段。
    ///
    /// 对应 Java: `public void setDepartment(String value)`
    pub fn set_department(&mut self, value: Option<String>) {
        self.put_string(TaskField::Department, value);
    }

    /// 取部门字段。
    ///
    /// 对应 Java: `public String getDepartment()`
    pub fn get_department(&self) -> Option<String> {
        self.string_field(TaskField::Department)
    }

    /// 设置总体完成百分比。
    ///
    /// 对应 Java: `public void setOverallPercentComplete(Number value)`
    pub fn set_overall_percent_complete(&mut self, value: Option<f64>) {
        self.put_number(TaskField::OverallPercentComplete, value);
    }

    /// 取总体完成百分比。
    ///
    /// 对应 Java: `public Number getOverallPercentComplete()`
    pub fn get_overall_percent_complete(&self) -> Option<f64> {
        self.number_field(TaskField::OverallPercentComplete)
    }

    /// 设置计划完成日期。
    ///
    /// 对应 Java: `public void setPlannedFinish(LocalDateTime value)`
    pub fn set_planned_finish(&mut self, value: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::PlannedFinish, value);
    }

    /// 取计划完成日期。
    ///
    /// 对应 Java: `public LocalDateTime getPlannedFinish()`
    pub fn get_planned_finish(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::PlannedFinish)
    }

    /// 设置计划开始日期。
    ///
    /// 对应 Java: `public void setPlannedStart(LocalDateTime value)`
    pub fn set_planned_start(&mut self, value: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::PlannedStart, value);
    }

    /// 取计划开始日期。
    ///
    /// 对应 Java: `public LocalDateTime getPlannedStart()`
    pub fn get_planned_start(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::PlannedStart)
    }

    /// 设置计划工期。
    ///
    /// 对应 Java: `public void setPlannedDuration(Duration value)`
    pub fn set_planned_duration(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::PlannedDuration, value);
    }

    /// 取计划工期。
    ///
    /// 对应 Java: `public Duration getPlannedDuration()`
    pub fn get_planned_duration(&self) -> Option<Duration> {
        self.duration_field(TaskField::PlannedDuration)
    }

    /// 设置计划工时。
    ///
    /// 对应 Java: `public void setPlannedWork(Duration value)`
    pub fn set_planned_work(&mut self, value: Option<Duration>) {
        self.put_duration(TaskField::PlannedWork, value);
    }

    /// 取计划工时。
    ///
    /// 对应 Java: `public Duration getPlannedWork()`
    pub fn get_planned_work(&self) -> Option<Duration> {
        self.duration_field(TaskField::PlannedWork)
    }

    /// 设置计划成本。
    ///
    /// 对应 Java: `public void setPlannedCost(Number value)`
    pub fn set_planned_cost(&mut self, value: Option<f64>) {
        self.put_number(TaskField::PlannedCost, value);
    }

    /// 取计划成本。
    ///
    /// 对应 Java: `public Number getPlannedCost()`
    pub fn get_planned_cost(&self) -> Option<f64> {
        self.number_field(TaskField::PlannedCost)
    }

    /// 设置挂起日期。
    ///
    /// 对应 Java: `public void setSuspendDate(LocalDateTime value)`
    pub fn set_suspend_date(&mut self, value: Option<NaiveDateTime>) {
        self.put_date_time(TaskField::SuspendDate, value);
    }

    /// 取挂起日期。
    ///
    /// 对应 Java: `public LocalDateTime getSuspendDate()`
    pub fn get_suspend_date(&self) -> Option<NaiveDateTime> {
        self.date_time_field(TaskField::SuspendDate)
    }

    /// 设置主资源唯一 ID。
    ///
    /// 对应 Java: `public void setPrimaryResourceUniqueID(Integer value)`
    pub fn set_primary_resource_unique_id(&mut self, value: Option<i32>) {
        self.put_integer(TaskField::PrimaryResourceUniqueId, value);
    }

    /// 取主资源唯一 ID。
    ///
    /// 对应 Java: `public Integer getPrimaryResourceUniqueID()`
    pub fn get_primary_resource_unique_id(&self) -> Option<i32> {
        self.integer_field(TaskField::PrimaryResourceUniqueId)
    }

    /// 取主资源。
    ///
    /// 对应 Java: `public Resource getPrimaryResource()`
    pub fn get_primary_resource(&self) -> Option<Rc<RefCell<Resource>>> {
        let file = self.parent_file.upgrade()?;
        let unique_id = self.get_primary_resource_unique_id();
        file.borrow().get_resource_by_unique_id(unique_id)
    }

    /// 设置主资源。
    ///
    /// 对应 Java: `public void setPrimaryResource(Resource resource)`
    pub fn set_primary_resource(&mut self, resource: Option<&Rc<RefCell<Resource>>>) {
        let unique_id = resource.and_then(|resource| resource.borrow().get_unique_id());
        self.set_primary_resource_unique_id(unique_id);
    }
}

// ---------------------------------------------------------------------------
// 子项目（外部项目）
// ---------------------------------------------------------------------------

impl Task {
    /// 若本任务代表外部项目（子项目），展开并把子项目任务挂为子任务。
    ///
    /// 对应 Java: `public ProjectFile expandSubproject()`
    pub fn expand_subproject(&mut self) -> Option<Rc<RefCell<ProjectFile>>> {
        if !self.get_external_project() || self.get_subproject_object().is_none() {
            return None;
        }

        let subproject = self.get_subproject_object()?;

        let children = {
            let summary_task = subproject.borrow().get_task_by_id(Some(0));
            match summary_task {
                None => subproject.borrow().get_child_tasks(),
                Some(summary_task) => summary_task.borrow().get_child_tasks(),
            }
        };

        self.children.extend(children);
        Some(subproject)
    }

    /// 读取本任务引用的外部项目文件（结果由项目文件缓存）。
    ///
    /// 对应 Java: `public ProjectFile getSubprojectObject()`
    pub fn get_subproject_object(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        if !self.get_external_task() && !self.get_external_project() {
            return None;
        }

        let file = self.parent_file.upgrade()?;
        let subproject_file = self.get_subproject_file();
        file.borrow_mut().read_external_project(subproject_file.as_deref())
    }

    /// 把已读取的项目文件挂到本任务引用的外部项目上。
    ///
    /// 对应 Java: `public void setSubprojectObject(ProjectFile projectFile)`
    pub fn set_subproject_object(&mut self, project_file: Option<Rc<RefCell<ProjectFile>>>) {
        let Some(file) = self.parent_file.upgrade() else {
            return;
        };

        let subproject_file = self.get_subproject_file();
        file.borrow_mut()
            .add_external_project(subproject_file.as_deref(), project_file);
    }
}

// ---------------------------------------------------------------------------
// 计算字段（对应 Java 的 calculateXxx / defaultXxx 方法）
// ---------------------------------------------------------------------------

impl Task {
    /// 取父任务的唯一 ID。
    ///
    /// 对应 Java: `private Integer calculateParentTaskUniqueID()`
    fn calculate_parent_task_unique_id(&self) -> Option<i32> {
        self.parent.upgrade()?.borrow().get_unique_id()
    }

    /// 计算开始偏差。
    ///
    /// 对应 Java: `private Duration calculateStartVariance()`
    ///
    /// 缺少项目文件时无法取得默认工期单位，返回 `None`（Java 会抛 `NullPointerException`）。
    fn calculate_start_variance(&self) -> Option<Duration> {
        let format = {
            let file = self.parent_file.upgrade()?;
            let file_ref = file.borrow();
            file_ref.get_project_properties().get_default_duration_units()
        };

        let calendar = self.get_effective_calendar();
        let variance = match calendar.as_ref() {
            Some(calendar) => {
                let calendar_ref = calendar.borrow();
                LocalDateTimeHelper::get_variance(
                    Some(&calendar_ref),
                    self.get_baseline_start(),
                    self.get_start(),
                    format,
                )
            }
            None => LocalDateTimeHelper::get_variance(
                None,
                self.get_baseline_start(),
                self.get_start(),
                format,
            ),
        };

        Some(variance)
    }

    /// 计算完成偏差。
    ///
    /// 对应 Java: `private Duration calculateFinishVariance()`
    fn calculate_finish_variance(&self) -> Option<Duration> {
        let format = {
            let file = self.parent_file.upgrade()?;
            let file_ref = file.borrow();
            file_ref.get_project_properties().get_default_duration_units()
        };

        let calendar = self.get_effective_calendar();
        let variance = match calendar.as_ref() {
            Some(calendar) => {
                let calendar_ref = calendar.borrow();
                LocalDateTimeHelper::get_variance(
                    Some(&calendar_ref),
                    self.get_baseline_finish(),
                    self.get_finish(),
                    format,
                )
            }
            None => LocalDateTimeHelper::get_variance(
                None,
                self.get_baseline_finish(),
                self.get_finish(),
                format,
            ),
        };

        Some(variance)
    }

    /// 计算开始浮动时间（委托给 `ProjectConfig` 的 slack 计算器）。
    ///
    /// 对应 Java: `private Duration calculateStartSlack()`
    fn calculate_start_slack(&self) -> Option<Duration> {
        self.parent_file
            .upgrade()?
            .borrow()
            .get_project_config()
            .get_slack_calculator()
            .and_then(|calculator| calculator.calculate_start_slack(self))
    }

    /// 计算完成浮动时间（委托给 `ProjectConfig` 的 slack 计算器）。
    ///
    /// 对应 Java: `private Duration calculateFinishSlack()`
    fn calculate_finish_slack(&self) -> Option<Duration> {
        self.parent_file
            .upgrade()?
            .borrow()
            .get_project_config()
            .get_slack_calculator()
            .and_then(|calculator| calculator.calculate_finish_slack(self))
    }

    /// 计算成本偏差。
    ///
    /// 对应 Java: `private Double calculateCostVariance()`
    fn calculate_cost_variance(&self) -> Option<f64> {
        let cost = self.get_cost()?;
        let baseline_cost = self.get_baseline_cost()?;
        Some(cost - baseline_cost)
    }

    /// 计算工期偏差。
    ///
    /// 对应 Java: `private Duration calculateDurationVariance()`
    ///
    /// 缺少项目文件时无法换算基线工期单位，返回 `None`。
    fn calculate_duration_variance(&self) -> Option<Duration> {
        let duration = self.get_duration()?;
        let baseline_duration = self.get_baseline_duration()?;

        let file = self.parent_file.upgrade()?;
        let file_ref = file.borrow();
        let baseline =
            baseline_duration.convert_units(duration.get_units(), file_ref.get_project_properties());

        Some(Duration::get_instance(
            duration.get_duration() - baseline.get_duration(),
            duration.get_units(),
        ))
    }

    /// 计算工时偏差。
    ///
    /// 对应 Java: `private Duration calculateWorkVariance()`
    fn calculate_work_variance(&self) -> Option<Duration> {
        let work = self.get_work()?;
        let baseline_work = self.get_baseline_work()?;

        let file = self.parent_file.upgrade()?;
        let file_ref = file.borrow();
        let baseline =
            baseline_work.convert_units(work.get_units(), file_ref.get_project_properties());

        Some(Duration::get_instance(
            work.get_duration() - baseline.get_duration(),
            work.get_units(),
        ))
    }

    /// 计算 CV（BCWP - ACWP）。
    ///
    /// 对应 Java: `private Double calculateCV()`
    fn calculate_cv(&self) -> Option<f64> {
        Some(NumberHelper::get_double(self.get_bcwp()) - NumberHelper::get_double(self.get_acwp()))
    }

    /// 计算 SV（BCWP - BCWS）。
    ///
    /// 对应 Java: `private Double calculateSV()`
    fn calculate_sv(&self) -> Option<f64> {
        let bcwp = self.get_bcwp()?;
        let bcws = self.get_bcws()?;
        Some(bcwp - bcws)
    }

    /// 计算自由时差（委托给 `ProjectConfig` 的 slack 计算器）。
    ///
    /// 对应 Java: `private Duration calculateFreeSlack()`
    fn calculate_free_slack(&self) -> Option<Duration> {
        self.parent_file
            .upgrade()?
            .borrow()
            .get_project_config()
            .get_slack_calculator()
            .and_then(|calculator| calculator.calculate_free_slack(self))
    }

    /// 计算总时差（委托给 `ProjectConfig` 的 slack 计算器）。
    ///
    /// 对应 Java: `private Duration calculateTotalSlack()`
    fn calculate_total_slack(&self) -> Option<Duration> {
        self.parent_file
            .upgrade()?
            .borrow()
            .get_project_config()
            .get_slack_calculator()
            .and_then(|calculator| calculator.calculate_total_slack(self))
    }

    /// 计算关键路径标志。
    ///
    /// 对应 Java: `private Boolean calculateCritical()`
    ///
    /// 缺少项目文件时无法取得「关键时差上限」，返回 `None`（`getCritical()` 因此为 `false`）。
    fn calculate_critical(&self) -> Option<bool> {
        if self.get_actual_finish().is_some() {
            return Some(false);
        }

        let mut total_slack = self.get_total_slack()?;

        let file = self.parent_file.upgrade()?;
        let file_ref = file.borrow();
        let critical_slack_limit = file_ref.get_project_properties().get_critical_slack_limit();

        if critical_slack_limit.get_duration() != 0.0
            && total_slack.get_duration() != 0.0
            && total_slack.get_units() != critical_slack_limit.get_units()
        {
            let calendar = self.get_effective_calendar();
            if let Some(calendar) = calendar {
                total_slack =
                    total_slack.convert_units(critical_slack_limit.get_units(), &*calendar.borrow());
            }
        }

        Some(
            total_slack.get_duration() <= critical_slack_limit.get_duration()
                && NumberHelper::get_int(self.get_percentage_complete()) != 100
                && (self.get_task_mode() == Some(TaskMode::AutoScheduled)
                    || (self.get_duration_text().is_none()
                        && self.get_start_text().is_none()
                        && self.get_finish_text().is_none())),
        )
    }

    /// 计算「完成到」日期。
    ///
    /// 对应 Java: `private LocalDateTime calculateCompleteThrough()`
    fn calculate_complete_through(&self) -> Option<NaiveDateTime> {
        let percent_complete = NumberHelper::get_int(self.get_percentage_complete());

        if percent_complete == 0 {
            return None;
        }

        if percent_complete == 100 {
            return self.get_actual_finish();
        }

        let actual_start = self.get_actual_start()?;
        let duration = self.get_duration()?;

        let duration = Duration::get_instance(
            (duration.get_duration() * f64::from(percent_complete)) / 100.0,
            duration.get_units(),
        );
        let calendar = self.get_effective_calendar()?;
        let value = calendar.borrow_mut().get_date(actual_start, duration);

        let complete_through_is_next_work_start = self
            .parent_file
            .upgrade()
            .map_or(false, |file| {
                file.borrow()
                    .get_project_config()
                    .get_complete_through_is_next_work_start()
            });

        if complete_through_is_next_work_start {
            // Java: calendar.getNextWorkStart(value)
            let calendar = self.get_effective_calendar()?;
            return Some(calendar.borrow().get_next_work_start(value));
        }

        Some(value)
    }

    /// 计算外部项目标志。
    ///
    /// 对应 Java: `private Boolean calculateExternalProject()`
    fn calculate_external_project(&self) -> Option<bool> {
        Some(self.get_subproject_file().is_some() && !self.get_external_task())
    }

    /// 计算完成百分比（按 `PercentCompleteType` 选择来源）。
    ///
    /// 对应 Java: `private Number calculateActivityPercentComplete()`
    fn calculate_activity_percent_complete(&self) -> Option<f64> {
        match self.get_percent_complete_type() {
            None => self.get_percentage_complete(),
            Some(PercentCompleteType::Units) => self.get_percentage_work_complete(),
            Some(PercentCompleteType::Physical) => self.get_physical_percent_complete(),
            Some(_) => self.get_percentage_complete(),
        }
    }

    /// 计算进度百分比完成度。
    ///
    /// 对应 Java: `private Number calculateSchedulePercentComplete()`
    ///
    /// Java 注释中标注为 `TODO: review and implement Schedule Percent Complete for WBS`。
    fn calculate_schedule_percent_complete(&self) -> Option<f64> {
        if self.get_summary() {
            return None;
        }

        let file = self.parent_file.upgrade()?;
        let file_ref = file.borrow();
        let data_date = file_ref.get_project_properties().get_status_date()?;
        let baseline_start = self.get_baseline_start()?;
        let baseline_finish = self.get_baseline_finish()?;

        if data_date < baseline_start {
            return Some(NumberHelper::DOUBLE_ZERO);
        }

        if data_date > baseline_finish {
            return Some(NumberHelper::DOUBLE_ONEHUNDRED);
        }

        let calendar = self.get_effective_calendar()?;
        let total_work = calendar
            .borrow()
            .get_work(baseline_start, baseline_finish, TimeUnit::Hours)
            .get_duration();
        if total_work == 0.0 {
            return None;
        }

        let current_work = calendar
            .borrow()
            .get_work(baseline_start, data_date, TimeUnit::Hours)
            .get_duration();

        Some((current_work * 100.0) / total_work)
    }

    /// 计算常规成本（总成本 - 加班成本）。
    ///
    /// 对应 Java: `private Number calculateRegularCost(Supplier<Number> totalCostSupplier, Supplier<Number> overtimeCostSupplier)`
    ///
    /// Java 用 `Supplier` 惰性取值，Rust 侧由调用方直接传入取值结果。
    fn calculate_regular_cost(
        &self,
        total_cost: Option<f64>,
        overtime_cost: Option<f64>,
    ) -> Option<f64> {
        if total_cost.is_none() && overtime_cost.is_none() {
            return None;
        }

        Some(
            NumberHelper::get_double(total_cost) - NumberHelper::get_double(overtime_cost),
        )
    }

    /// 计算常规工时（总工时 - 加班工时）。
    ///
    /// 对应 Java: `private Duration calculateRegularWork(Supplier<Duration> totalWorkSupplier, Supplier<Duration> overtimeWorkSupplier)`
    ///
    /// Java 用 `getCalendar()` 作为单位换算的默认值来源；日历为空时 Rust 侧跳过换算
    /// （Java 会抛 `NullPointerException`）。
    fn calculate_regular_work(
        &self,
        total_work: Option<Duration>,
        overtime_work: Option<Duration>,
    ) -> Option<Duration> {
        if total_work.is_none() && overtime_work.is_none() {
            return None;
        }

        let units = match (total_work, overtime_work) {
            (Some(total_work), _) => total_work.get_units(),
            (None, Some(overtime_work)) => overtime_work.get_units(),
            // 上面已排除两者均为 None 的情形
            (None, None) => return None,
        };

        let calendar = self.get_calendar();
        let convert = |work: Option<Duration>| -> f64 {
            match (work, calendar.as_ref()) {
                (Some(work), Some(calendar)) => {
                    work.convert_units(units, &*calendar.borrow()).get_duration()
                }
                (Some(work), None) => work.get_duration(),
                (None, _) => 0.0,
            }
        };

        let total = convert(total_work);
        let overtime = convert(overtime_work);

        Some(Duration::get_instance(total - overtime, units))
    }

    /// 取 `active` 字段的默认值。
    ///
    /// 对应 Java: `private Boolean defaultActive()`
    fn default_active() -> bool {
        true
    }

    /// 取 `type` 字段的默认值。
    ///
    /// 对应 Java: `private TaskType defaultType()`
    fn default_type() -> TaskType {
        TaskType::FixedUnits
    }

    /// 取 `taskMode` 字段的默认值。
    ///
    /// 对应 Java: `private TaskMode defaultTaskMode()`
    fn default_task_mode() -> TaskMode {
        TaskMode::AutoScheduled
    }

    /// 取 `expanded` 字段的默认值。
    ///
    /// 对应 Java: `private Boolean defaultExpanded()`
    fn default_expanded() -> bool {
        true
    }

    /// 取 `autoComputeActuals` 字段的默认值。
    ///
    /// 对应 Java: `private Boolean defaultAutoComputeActuals()`
    fn default_auto_compute_actuals() -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// Display / 接口实现
// ---------------------------------------------------------------------------

impl Display for Task {
    /// 对应 Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[Task id={}", optional_display(self.get_id()))?;
        write!(f, " uniqueID={}", optional_display(self.get_unique_id()))?;

        if let Some(activity_id) = self.get_activity_id() {
            write!(f, " activityID={activity_id}")?;
        }

        write!(f, " name={}", optional_display(self.get_name()))?;

        if self.get_external_task() {
            write!(
                f,
                " [EXTERNAL uid={} id={}]",
                optional_display(self.get_subproject_task_unique_id()),
                optional_display(self.get_subproject_task_id())
            )
        } else {
            write!(f, "]")
        }
    }
}

/// 对应 Java 中把可空值直接拼进字符串的输出（`null` 输出 `"null"`）。
fn optional_display<T: Display>(value: Option<T>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| value.to_string())
}

impl FieldContainer for Task {
    /// 对应 Java: `@Override public void set(FieldType field, Object value)`
    fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
        Task::set(self, field, value);
    }

    /// 对应 Java: `@Override public Object get(FieldType field)`
    fn get(&self, field: FieldKey) -> Option<FieldValue> {
        Task::get(self, field)
    }

    /// 对应 Java: `@Override public Object getCachedValue(FieldType field)`
    fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
        Task::get_cached_value(self, field)
    }

    /// 对应 Java: `@Override public void addFieldListener(FieldListener listener)`
    fn add_field_listener(&mut self, listener: Box<dyn FieldListener>) {
        Task::add_field_listener(self, listener);
    }

    /// 对应 Java: `@Override public void removeFieldListener(FieldListener listener)`
    fn clear_field_listeners(&mut self) {
        Task::clear_field_listeners(self);
    }
}

impl ProjectEntityWithUniqueID for Task {
    /// 对应 Java: `@Override public Integer getUniqueID()`
    fn get_unique_id(&self) -> Option<i32> {
        self.integer_field(TaskField::UniqueId)
    }
}

impl ProjectEntityWithMutableUniqueID for Task {
    /// 对应 Java: `@Override public void setUniqueID(Integer val)`
    fn set_unique_id(&mut self, id: Option<i32>) {
        self.put_integer(TaskField::UniqueId, id);
    }
}

impl ProjectEntityWithID for Task {
    /// 对应 Java: `@Override public Integer getID()`
    fn get_id(&self) -> Option<i32> {
        self.integer_field(TaskField::Id)
    }

    /// 对应 Java: `@Override public void setID(Integer val)`
    fn set_id(&mut self, id: Option<i32>) {
        self.put_integer(TaskField::Id, id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::PlainNotes;

    fn datetime(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .expect("合法日期")
            .and_hms_opt(hour, minute, 0)
            .expect("合法时间")
    }

    fn days(value: f64) -> Duration {
        Duration::get_instance(value, TimeUnit::Days)
    }

    /// 注：需要父 `ProjectFile` 的构造方法（[`Task::new_in_file`]）与 `add_task` /
    /// `get_calendar` / `get_resource_assignments` 等访问器无法在单元测试中构造
    /// 依赖（`ProjectFile` 尚未落地），故此处只覆盖不依赖父文件的路径。
    #[test]
    fn field_values_round_trip() {
        let mut task = Task::new();

        task.set_name(Some("Design".to_owned()));
        task.set_wbs(Some("1.2".to_owned()));
        task.set_duration(Some(days(3.0)));
        task.set_start(Some(datetime(2024, 1, 1, 9, 0)));
        task.set_finish(Some(datetime(2024, 1, 5, 17, 0)));
        task.set_cost(Some(1200.0));
        task.set_percentage_complete(Some(50.0));
        task.set_milestone(true);
        task.set_unique_id(Some(7));
        task.set_id(Some(2));

        assert_eq!(task.get_name(), Some("Design".to_owned()));
        assert_eq!(task.get_wbs(), Some("1.2".to_owned()));
        assert_eq!(task.get_duration(), Some(days(3.0)));
        assert_eq!(task.get_start(), Some(datetime(2024, 1, 1, 9, 0)));
        assert_eq!(task.get_finish(), Some(datetime(2024, 1, 5, 17, 0)));
        assert_eq!(task.get_cost(), Some(1200.0));
        assert_eq!(task.get_percentage_complete(), Some(50.0));
        assert!(task.get_milestone());
        assert_eq!(task.get_unique_id(), Some(7));
        assert_eq!(task.get_id(), Some(2));

        // 未设置的字段返回 None / false
        assert_eq!(task.get_contact(), None);
        assert!(!task.get_confirmed());

        // 传 None 等价于 Java 的 set(field, null)：把字段从字段表中移除
        task.set_name(None);
        assert_eq!(task.get_name(), None);
        assert_eq!(task.get_cached_value(FieldKey::Task(TaskField::Name)), None);
    }

    #[test]
    fn field_container_is_delegated_and_cached() {
        let mut task = Task::new();

        task.set(
            FieldKey::Task(TaskField::Notes),
            Some(FieldValue::String("note".to_owned())),
        );
        assert_eq!(
            task.get_cached_value(FieldKey::Task(TaskField::Notes)),
            Some(&FieldValue::String("note".to_owned()))
        );
        assert_eq!(
            Task::get(&task, FieldKey::Task(TaskField::Notes)),
            Some(FieldValue::String("note".to_owned()))
        );

        // 计算出的默认值不写回字段表（见文件头差异第 1 条）
        assert_eq!(
            task.get(FieldKey::Task(TaskField::AutoComputeActuals)),
            Some(FieldValue::Boolean(true))
        );
        assert_eq!(
            task.get_cached_value(FieldKey::Task(TaskField::AutoComputeActuals)),
            None
        );
    }

    #[test]
    fn field_container_trait_dispatches_to_task() {
        let mut task = Task::new();

        {
            let container: &mut dyn FieldContainer = &mut task;
            container.set(
                FieldKey::Task(TaskField::Name),
                Some(FieldValue::String("Trait".to_owned())),
            );
        }

        assert_eq!(task.get_name(), Some("Trait".to_owned()));
    }

    #[test]
    fn calculated_fields_are_computed_on_demand() {
        let mut task = Task::new();

        // Java: calculateCostVariance —— 需要 cost 与 baselineCost 同时存在
        assert_eq!(task.get_cost_variance(), None);
        task.set_cost(Some(100.0));
        task.set_baseline_cost(Some(80.0));
        assert_eq!(task.get_cost_variance(), Some(20.0));

        // Java: calculateSV / calculateCV
        task.set_bcwp(Some(30.0));
        task.set_bcws(Some(10.0));
        task.set_acwp(Some(5.0));
        assert_eq!(task.get_sv(), Some(20.0));
        assert_eq!(task.get_cv(), Some(25.0));

        // Java: calculateExternalProject
        assert!(!task.get_external_project());
        task.set_subproject_file(Some("sub.mpp".to_owned()));
        assert!(task.get_external_project());
        task.set_external_task(true);
        assert!(!task.get_external_project());

        // Java: calculateActivityPercentComplete
        task.set_percentage_complete(Some(10.0));
        task.set_percentage_work_complete(Some(20.0));
        task.set_physical_percent_complete(Some(30.0));
        assert_eq!(task.get_activity_percent_complete(), Some(10.0));
        task.set_percent_complete_type(Some(PercentCompleteType::Units));
        assert_eq!(task.get_activity_percent_complete(), Some(20.0));
        task.set_percent_complete_type(Some(PercentCompleteType::Physical));
        assert_eq!(task.get_activity_percent_complete(), Some(30.0));
    }

    #[test]
    fn default_calculated_values_match_java() {
        let mut task = Task::new();

        // Java 的 CALCULATED_FIELD_MAP 默认值：active / type / taskMode / expanded /
        // autoComputeActuals
        assert!(task.get_active());
        assert_eq!(task.get_type(), Some(TaskType::FixedUnits));
        assert_eq!(task.get_task_mode(), Some(TaskMode::AutoScheduled));
        assert!(task.get_expanded());
        assert!(task.get_auto_compute_actuals());

        // 显式设置后覆盖默认值
        task.set_active(false);
        task.set_type(Some(TaskType::FixedWork));
        task.set_task_mode(Some(TaskMode::ManuallyScheduled));
        task.set_expanded(false);
        task.set_auto_compute_actuals(false);

        assert!(!task.get_active());
        assert_eq!(task.get_type(), Some(TaskType::FixedWork));
        assert_eq!(task.get_task_mode(), Some(TaskMode::ManuallyScheduled));
        assert!(!task.get_expanded());
        assert!(!task.get_auto_compute_actuals());
    }

    #[test]
    fn always_calculated_fields_are_not_cached() {
        assert!(Task::get_always_calculated_field(TaskField::WorkSplits));
        assert!(Task::get_always_calculated_field(TaskField::Predecessors));
        assert!(!Task::get_always_calculated_field(TaskField::Name));

        let task = Task::new();

        // 无父文件时 SCHEDULE_PERCENT_COMPLETE 无法计算，返回 None 且不进入字段表
        assert_eq!(
            task.get(FieldKey::Task(TaskField::SchedulePercentComplete)),
            None
        );
        assert_eq!(
            task.get_cached_value(FieldKey::Task(TaskField::SchedulePercentComplete)),
            None
        );
    }

    #[test]
    fn enum_and_uuid_fields_round_trip() {
        let mut task = Task::new();

        task.set_constraint_type(Some(ConstraintType::MustStartOn));
        assert_eq!(
            task.get_constraint_type(),
            Some(ConstraintType::MustStartOn)
        );
        task.set_secondary_constraint_type(Some(ConstraintType::FinishNoLaterThan));
        assert_eq!(
            task.get_secondary_constraint_type(),
            Some(ConstraintType::FinishNoLaterThan)
        );

        task.set_priority(Some(Priority::get_instance(Priority::HIGH)));
        assert_eq!(
            task.get_priority().map(|value| value.get_value()),
            Some(Priority::HIGH)
        );

        task.set_leveling_delay_format(Some(TimeUnit::ElapsedDays));
        assert_eq!(task.get_leveling_delay_format(), Some(TimeUnit::ElapsedDays));

        task.set_fixed_cost_accrual(Some(AccrueType::Start));
        assert_eq!(task.get_fixed_cost_accrual(), Some(AccrueType::Start));

        task.set_earned_value_method(Some(EarnedValueMethod::PhysicalPercentComplete));
        assert_eq!(
            task.get_earned_value_method(),
            Some(EarnedValueMethod::PhysicalPercentComplete)
        );

        task.set_activity_status(Some(ActivityStatus::Completed));
        assert_eq!(
            task.get_activity_status(),
            Some(ActivityStatus::Completed)
        );

        // UUID 以字符串承载（见文件头差异第 7 条）
        let guid = Uuid::parse_str("2f9d1c2e-6d5b-4f0e-9a3c-1b2d3e4f5a6b").expect("合法 UUID");
        task.set_guid(Some(guid));
        assert_eq!(task.get_guid(), Some(guid));
        assert_eq!(
            task.get_cached_value(FieldKey::Task(TaskField::Guid)),
            Some(&FieldValue::String(guid.to_string()))
        );

        task.set_guid(None);
        assert_eq!(task.get_guid(), None);
    }

    #[test]
    fn notes_are_stored_as_plain_text() {
        let mut task = Task::new();

        assert_eq!(task.get_notes(), "");
        assert!(task.get_notes_object().is_none());

        task.set_notes(Some("hello".to_owned()));
        assert_eq!(task.get_notes(), "hello");
        assert_eq!(
            task.get_notes_object().map(|notes| notes.to_string()),
            Some("hello".to_owned())
        );

        let plain = PlainNotes::new(Some("object".to_owned()));
        task.set_notes_object(Some(&plain));
        assert_eq!(task.get_notes(), "object");
    }

    #[test]
    fn indexed_fields_map_to_the_java_field_lists() {
        let mut task = Task::new();

        task.set_text(1, Some("first".to_owned()));
        task.set_text(30, Some("last".to_owned()));
        assert_eq!(task.get_text(1), Some("first".to_owned()));
        assert_eq!(task.get_text(30), Some("last".to_owned()));

        // Java 抛 IllegalArgumentException，Rust 侧返回 None / false
        assert_eq!(task.get_text(0), None);
        assert_eq!(task.get_text(31), None);
        task.set_text(31, Some("ignored".to_owned()));

        task.set_cost_at(2, Some(10.0));
        assert_eq!(task.get_cost_at(2), Some(10.0));
        assert_eq!(task.get_cost_at(11), None);

        task.set_flag(20, true);
        assert!(task.get_flag(20));
        assert!(!task.get_flag(21));

        task.set_duration_at(10, Some(days(1.0)));
        assert_eq!(task.get_duration_at(10), Some(days(1.0)));
        task.set_date(1, Some(datetime(2024, 3, 1, 8, 0)));
        assert_eq!(task.get_date(1), Some(datetime(2024, 3, 1, 8, 0)));
        task.set_start_at(1, Some(datetime(2024, 3, 1, 8, 0)));
        assert_eq!(task.get_start_at(1), Some(datetime(2024, 3, 1, 8, 0)));
        task.set_finish_at(1, Some(datetime(2024, 3, 2, 8, 0)));
        assert_eq!(task.get_finish_at(1), Some(datetime(2024, 3, 2, 8, 0)));
        task.set_number(1, Some(1.5));
        assert_eq!(task.get_number(1), Some(1.5));
        task.set_outline_code(1, Some("OC".to_owned()));
        assert_eq!(task.get_outline_code(1), Some("OC".to_owned()));
        task.set_enterprise_text(1, Some("ET".to_owned()));
        assert_eq!(task.get_enterprise_text(1), Some("ET".to_owned()));
        task.set_enterprise_flag(1, true);
        assert!(task.get_enterprise_flag(1));

        // 多基线字段
        task.set_baseline_cost_at(1, Some(5.0));
        assert_eq!(task.get_baseline_cost_at(1), Some(5.0));
        assert_eq!(task.get_baseline_cost_at(0), None);
        task.set_baseline_work_at(3, Some(days(2.0)));
        assert_eq!(task.get_baseline_work_at(3), Some(days(2.0)));
    }

    #[test]
    fn baseline_duration_and_text_share_one_field() {
        let mut task = Task::new();

        // Java: BASELINE_DURATION 字段既可存 Duration 也可存 String，
        // 读取时按 instanceof 判断（Rust 侧按取值类型返回）
        task.set_baseline_duration(Some(days(4.0)));
        assert_eq!(task.get_baseline_duration(), Some(days(4.0)));
        assert_eq!(task.get_baseline_duration_text(), None);

        task.set_baseline_duration_text(Some("4d".to_owned()));
        assert_eq!(task.get_baseline_duration_text(), Some("4d".to_owned()));
        assert_eq!(task.get_baseline_duration(), None);
    }

    #[test]
    fn hierarchy_links_parent_and_children() {
        let parent = Rc::new(RefCell::new(Task::new()));
        parent.borrow_mut().set_outline_level(Some(1));
        parent.borrow_mut().set_unique_id(Some(7));

        let child = Rc::new(RefCell::new(Task::new()));
        Task::add_child_task(&parent, &child);

        assert!(parent.borrow().has_child_tasks());
        assert_eq!(parent.borrow().get_child_tasks().len(), 1);
        assert!(parent.borrow().get_summary());
        assert_eq!(child.borrow().get_parent_task_unique_id(), Some(7));

        let recovered = child.borrow().get_parent_task().expect("父任务仍然存在");
        assert!(Rc::ptr_eq(&recovered, &parent));

        // 移除子任务后父任务不再是汇总任务，且回指被清空
        Task::remove_child_task(&parent, &child);
        assert!(!parent.borrow().has_child_tasks());
        assert!(!parent.borrow().get_summary());
        assert!(child.borrow().get_parent_task().is_none());
        assert_eq!(child.borrow().get_parent_task_unique_id(), None);
    }

    #[test]
    fn child_tasks_can_be_inserted_and_cleared() {
        let parent = Rc::new(RefCell::new(Task::new()));
        let first = Rc::new(RefCell::new(Task::new()));
        let second = Rc::new(RefCell::new(Task::new()));

        Task::add_child_task(&parent, &first);
        Task::add_child_task(&parent, &second);

        let third = Rc::new(RefCell::new(Task::new()));
        Task::add_child_task_before(&parent, &third, &second);

        let children = parent.borrow().get_child_tasks();
        assert_eq!(children.len(), 3);
        assert!(Rc::ptr_eq(&children[0], &first));
        assert!(Rc::ptr_eq(&children[1], &third));
        assert!(Rc::ptr_eq(&children[2], &second));

        // 预置同级任务不在列表中时追加到末尾
        let outsider = Rc::new(RefCell::new(Task::new()));
        let fourth = Rc::new(RefCell::new(Task::new()));
        Task::add_child_task_before(&parent, &fourth, &outsider);
        let children = parent.borrow().get_child_tasks();
        assert_eq!(children.len(), 4);
        assert!(Rc::ptr_eq(children.last().expect("非空"), &fourth));

        Task::clear_child_tasks(&parent);
        assert!(!parent.borrow().has_child_tasks());
        assert!(!parent.borrow().get_summary());
        assert!(third.borrow().get_parent_task().is_some());

        // 列表为空时 clearChildTasks 不改写 summary 标志
        parent.borrow_mut().set_summary(true);
        Task::clear_child_tasks(&parent);
        assert!(parent.borrow().get_summary());
    }

    #[test]
    fn child_tasks_can_be_attached_by_outline_level() {
        let root = Rc::new(RefCell::new(Task::new()));
        root.borrow_mut().set_outline_level(Some(1));

        let level2 = Rc::new(RefCell::new(Task::new()));
        level2.borrow_mut().set_outline_level(Some(2));
        Task::add_child_task_at_level(&root, &level2, 2);
        assert_eq!(root.borrow().get_child_tasks().len(), 1);

        // 更深一层应挂到最后一个子任务上
        let level3 = Rc::new(RefCell::new(Task::new()));
        Task::add_child_task_at_level(&root, &level3, 3);
        assert_eq!(root.borrow().get_child_tasks().len(), 1);
        assert_eq!(level2.borrow().get_child_tasks().len(), 1);

        // 与当前层级不匹配且没有子任务时不做任何事
        let other = Rc::new(RefCell::new(Task::new()));
        other.borrow_mut().set_outline_level(Some(5));
        let orphan = Rc::new(RefCell::new(Task::new()));
        Task::add_child_task_at_level(&other, &orphan, 2);
        assert!(!other.borrow().has_child_tasks());
    }

    #[test]
    fn generated_wbs_uses_parent_numbering() {
        let mut root = Task::new();
        root.set_unique_id(Some(0));
        root.generate_wbs(None);
        assert_eq!(root.get_wbs(), Some("0".to_owned()));
        root.generate_outline_number(None);
        assert_eq!(root.get_outline_number(), Some("0".to_owned()));

        // 父任务 WBS 为 "0" 时直接取「子任务数 + 1」
        let mut zero_parent = Task::new();
        zero_parent.set_wbs(Some("0".to_owned()));
        let zero_parent = Rc::new(RefCell::new(zero_parent));
        let mut child = Task::new();
        child.generate_wbs(Some(&zero_parent));
        assert_eq!(child.get_wbs(), Some("1".to_owned()));

        // 父任务 WBS 非 "0" 时追加 ".n"；大纲编号会先去掉结尾的 ".0"
        let mut parent = Task::new();
        parent.set_wbs(Some("1".to_owned()));
        parent.set_outline_number(Some("1.0".to_owned()));
        let parent = Rc::new(RefCell::new(parent));
        child.generate_wbs(Some(&parent));
        assert_eq!(child.get_wbs(), Some("1.1".to_owned()));
        child.generate_outline_number(Some(&parent));
        assert_eq!(child.get_outline_number(), Some("1.1".to_owned()));

        // 顶层任务按「项目文件子任务数 + 1」编号（无父文件时为 1）
        let mut top = Task::new();
        top.set_unique_id(Some(1));
        top.generate_wbs(None);
        assert_eq!(top.get_wbs(), Some("1".to_owned()));
    }

    #[test]
    fn compare_to_orders_by_id() {
        let mut first = Task::new();
        first.set_id(Some(1));
        let mut second = Task::new();
        second.set_id(Some(2));

        assert_eq!(first.compare_to(&second), Ordering::Less);
        assert_eq!(second.compare_to(&first), Ordering::Greater);
        assert_eq!(first.compare_to(&first), Ordering::Equal);

        // 未设置 ID 时按 0 处理（Java: NumberHelper.getInt(null) == 0）
        let third = Task::new();
        assert_eq!(third.compare_to(&first), Ordering::Less);
    }

    #[test]
    fn display_matches_java_to_string() {
        let mut task = Task::new();
        task.set_id(Some(1));
        task.set_unique_id(Some(2));
        task.set_name(Some("X".to_owned()));

        assert_eq!(task.to_string(), "[Task id=1 uniqueID=2 name=X]");

        task.set_activity_id(Some("A".to_owned()));
        assert_eq!(task.to_string(), "[Task id=1 uniqueID=2 activityID=A name=X]");

        task.set_external_task(true);
        task.set_subproject_task_unique_id(Some(9));
        task.set_subproject_task_id(Some(3));
        // Java: 外部任务分支不再追加额外的 "]"
        assert_eq!(
            task.to_string(),
            "[Task id=1 uniqueID=2 activityID=A name=X [EXTERNAL uid=9 id=3]"
        );
    }

    #[test]
    fn events_can_be_disabled() {
        let mut task = Task::new();

        task.disable_events();
        task.set_cost(Some(10.0));
        task.set_baseline_cost(Some(1.0));
        // 依赖清理被关闭，但仍按需求值（Rust 侧不缓存计算结果）
        assert_eq!(task.get_cost_variance(), Some(9.0));
        task.enable_events();
    }

    #[test]
    fn project_entity_traits_are_implemented() {
        let mut task = Task::new();

        ProjectEntityWithMutableUniqueID::set_unique_id(&mut task, Some(11));
        assert_eq!(ProjectEntityWithUniqueID::get_unique_id(&task), Some(11));

        ProjectEntityWithID::set_id(&mut task, Some(3));
        assert_eq!(ProjectEntityWithID::get_id(&task), Some(3));

        // 固有的同名方法访问同一字段
        assert_eq!(task.get_unique_id(), Some(11));
        assert_eq!(task.get_id(), Some(3));
    }
}
