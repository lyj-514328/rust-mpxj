//! 对应 Java: `org/mpxj/ProjectContext.java`
//!
//! `ProjectContext` 是领域模型的中枢：被 `ProjectFile` 持有，向多个项目共享的资源、
//! 日历、自定义字段、班次、计量单位等容器提供统一入口。
//!
//! # 未移植的 Java 字段与 getter（模块未在 `src/lib.rs` 声明）
//!
//! 下列容器属于 MPP/MPX reader 之外的格式（Primavera / Asta 等）所使用的数据，
//! 对应模块未在 `src/lib.rs` 中声明，Rust 侧不提供字段与 getter：
//!
//! | Java 字段 / getter | 说明 |
//! |---|---|
//! | `m_locations` / `getLocations()` | `LocationContainer`（模块未声明） |
//! | `m_expenseCategories` / `getExpenseCategories()` | `ExpenseCategoryContainer`（模块未声明） |
//! | `m_costAccounts` / `getCostAccounts()` | `CostAccountContainer`（模块未声明） |
//! | `m_activityCodes` / `getActivityCodes()` | `ActivityCodeContainer`（模块未声明） |
//! | `m_projectCodes` / `getProjectCodes()` | `ProjectCodeContainer`（模块未声明） |
//! | `m_resourceCodes` / `getResourceCodes()` | `ResourceCodeContainer`（模块未声明） |
//! | `m_roleCodes` / `getRoleCodes()` | `RoleCodeContainer`（模块未声明） |
//! | `m_resourceAssignmentCodes` / `getResourceAssignmentCodes()` | `ResourceAssignmentCodeContainer`（模块未声明） |
//! | `m_currencies` / `getCurrencies()` | `CurrencyContainer`（模块未声明） |
//!
//! # 与 Java 的差异
//!
//! - **对象生命周期**：Java 的 `ProjectContext` 由 GC 管理，实体直接持有它的强引用；
//!   Rust 侧 `ProjectContext` 放在 `Rc<RefCell<ProjectContext>>` 中，实体以
//!   `Weak<RefCell<ProjectContext>>` 回指它（见 [`ProjectContext::shared`]）。
//! - **子容器持有方式**：`ProjectContext` 直接以普通字段持有各子容器（值语义）；
//!   子容器对上下文的回指（如 [`crate::project_calendar_container::ProjectCalendarContainer`]）
//!   用 `Weak`，由 [`ProjectContext::shared`] 完成接线。
//! - **可变访问器**：Java 的 `getCalendars()` 等返回的是可变集合对象，调用方可以直接
//!   `m_context.getCalendars().add(...)`。Rust 的 `&self` getter 无法提供该语义，
//!   因此对以值语义持有的容器（`ProjectCalendarContainer` 等），除同名 getter 外另提供
//!   `*_mut` 访问器（Rust 特有，非 Java 方法）；[`ProjectContext::get_custom_fields`]
//!   返回的是 `Rc<RefCell<CustomFieldContainer>>`，变异经由其 `RefCell` 完成，故不另设 `_mut`。
//! - **项目列表**：Java 的 `m_projects` 是 `ProjectFile` 的强引用（`ProjectFile`
//!   构造方法里 `m_context.getProjects().add(this)`）。若 Rust 侧也用强引用会与
//!   `ProjectFile → ProjectContext` 形成 `Rc` 循环，故改为 `Weak<RefCell<ProjectFile>>`
//!   并按 [`ProjectContext::add_project`] 登记、[`ProjectContext::get_projects`] 升级返回。
//! - **异常列表**：Java 的 `List<Exception>` 对应 Rust 侧的统一错误类型
//!   [`crate::mpxj_exception::MpxjError`]。
//! - **序号生成器提供者的接线**：Java 的 `new WorkContourContainer(this)` /
//!   `new NotesTopicContainer(this)` 直接把上下文自己交给容器。Rust 侧若让容器强引用
//!   上下文，会形成 `ProjectContext → 容器 → ProjectContext` 的 `Rc` 循环导致内存泄漏，
//!   因此 Rust 侧插入一个**只持有上下文弱引用**的适配器
//!   `ContextSequenceProvider`（见文件末尾），由 [`ProjectContext::shared`] 完成接线；
//!   它把请求转发给上下文，从而与 Java 一样共享同一批序号生成器。
//! - **`ProjectCalendarContainer` 的接线**：该容器同样只持有上下文弱引用，
//!   由 [`ProjectContext::shared`] 调用 `set_context` 完成接线。
//! - **日历编号的借用拆分**：Java 可在容器内回读上下文配置和序列；Rust 从
//!   `get_calendars_mut()` 取得容器时已借用上下文，故通过本类型的创建入口预先领取
//!   同一个 `ObjectSequence` 的 ID，再交给容器构造日历。
//! - **`Class<?>` 参数**：Java 的 `getUniqueIdObjectSequence(Class<?> c)` /
//!   `resetUniqueIdObjectSequence(Class<?> c)` 以 `c.getName()` 作为 map key。Rust 侧按已移植的
//!   [`crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider`] 改用
//!   [`std::any::TypeId`] 作键、返回共享的 [`ObjectSequence`]（`Rc<ObjectSequence>`，
//!   与 Java 侧 `computeIfAbsent` 返回同一实例的语义一致）。
//! - **跨模块接口已对齐**：`crate::structured_notes` 已改为按
//!   `get_unique_id_object_sequence::<StructuredNotes>(context)`（[`std::any::TypeId`] 键）
//!   取序列，并直接使用 `sync_or_get_next` 的 `i32` 返回值，与
//!   [`crate::unique_id_object_sequence_provider`] / [`crate::common::object_sequence`]
//!   完全一致（`crate::project_calendar` 同样已对齐到 `TypeId`）。
//! - 未派生 `Debug`：字段类型里的 `EventManager`（以及 `ProjectFile`）未实现 `Debug`，
//!   故本类型也只能手工实现（相关模块均已落地，缺的是各自的 `Debug` 实现）。
//!
//! # 用到的各容器构造签名（已核对一致）
//!
//! 下列构造表达式按各模块已落地的签名书写：
//! `UnitOfMeasureContainer::new()`、`WorkContourContainer::new(sequence_provider)`、
//! `NotesTopicContainer::new(sequence_provider)`、`ProjectCalendarContainer::new_with_provider(sequence_provider)`、`CustomFieldContainer::new()`（返回
//! `Rc<RefCell<Self>>`）、`UserDefinedFieldContainer::new()`、`ShiftContainer::new()`、
//! `ShiftPeriodContainer::new()`、`ProjectCalendarContainer::new()`、`ResourceContainer::new()`、
//! `TimeUnitDefaults::new()`、`ProjectConfig::new()`、`EventManager::new()`。
//! 单元测试还会调用各容器的 `is_empty()`（对应 Java `ArrayList.isEmpty()`）。

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use crate::common::object_sequence::ObjectSequence;
use crate::custom_field_container::CustomFieldContainer;
use crate::event_manager::EventManager;
use crate::mpxj_exception::MpxjError;
use crate::notes_topic_container::NotesTopicContainer;
use crate::project_calendar::ProjectCalendar;
use crate::project_calendar_container::ProjectCalendarContainer;
use crate::project_config::ProjectConfig;
use crate::project_file::ProjectFile;
use crate::resource_container::ResourceContainer;
use crate::shift_container::ShiftContainer;
use crate::shift_period_container::ShiftPeriodContainer;
use crate::time_unit_defaults::TimeUnitDefaults;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;
use crate::unit_of_measure_container::UnitOfMeasureContainer;
use crate::user_defined_field_container::UserDefinedFieldContainer;
use crate::work_contour_container::WorkContourContainer;

/// 可在多个 `ProjectFile` 实例之间共享的公共数据容器。
///
/// 对应 Java: `public class ProjectContext implements UniqueIdObjectSequenceProvider`
pub struct ProjectContext {
    /// 对应 Java 字段 `private final UnitOfMeasureContainer m_unitsOfMeasure = new UnitOfMeasureContainer(this);`
    units_of_measure: UnitOfMeasureContainer,
    /// 对应 Java 字段 `private final WorkContourContainer m_workContours = new WorkContourContainer(this);`
    work_contours: WorkContourContainer,
    /// 对应 Java 字段 `private final NotesTopicContainer m_notesTopics = new NotesTopicContainer(this);`
    notes_topics: NotesTopicContainer,
    /// 对应 Java 字段 `private final CustomFieldContainer m_customFields = new CustomFieldContainer();`
    ///
    /// [`CustomFieldContainer::new`] 返回 `Rc<RefCell<Self>>`（`CustomField` 需要弱引用回指
    /// 父容器），故 Rust 侧以 `Rc<RefCell<..>>` 持有。
    custom_fields: Rc<RefCell<CustomFieldContainer>>,
    /// 对应 Java 字段 `private final UserDefinedFieldContainer m_userDefinedFields = new UserDefinedFieldContainer(m_customFields);`
    user_defined_fields: UserDefinedFieldContainer,
    /// 对应 Java 字段 `private final ShiftContainer m_shifts = new ShiftContainer(this);`
    shifts: ShiftContainer,
    /// 对应 Java 字段 `private final ShiftPeriodContainer m_shiftPeriods = new ShiftPeriodContainer(this);`
    shift_periods: ShiftPeriodContainer,
    /// 对应 Java 字段 `private final ProjectCalendarContainer m_calendars = new ProjectCalendarContainer(this);`
    calendars: ProjectCalendarContainer,
    /// 对应 Java 字段 `private final ResourceContainer m_resources = new ResourceContainer(this);`
    resources: ResourceContainer,
    /// 对应 Java 字段 `private final List<ProjectFile> m_projects = new ArrayList<>();`
    ///
    /// Rust 侧存弱引用，避免与 `ProjectFile → ProjectContext` 形成 `Rc` 循环。
    projects: Vec<Weak<RefCell<ProjectFile>>>,
    /// 对应 Java 字段 `private final TimeUnitDefaults m_timeUnitDefaults = new TimeUnitDefaults();`
    time_unit_defaults: TimeUnitDefaults,
    /// 对应 Java 字段 `private final ProjectConfig m_projectConfig = new ProjectConfig();`
    project_config: ProjectConfig,
    /// 对应 Java 字段 `private final Map<String, ObjectSequence> m_uniqueIdObjectSequences = new HashMap<>();`
    ///
    /// Java 的 key 是 `Class.getName()`，Rust 侧按已移植的 trait 改用
    /// [`std::any::TypeId`]，值为共享的 `Rc<ObjectSequence>`。
    unique_id_object_sequences: HashMap<TypeId, Rc<ObjectSequence>>,
    /// 对应 Java 字段 `private final List<Exception> m_ignoredErrors = new ArrayList<>();`
    ignored_errors: Vec<MpxjError>,
    /// 对应 Java 字段 `private final EventManager m_eventManager = new EventManager();`
    event_manager: EventManager,
    /// 供子容器使用的序号生成器提供者（Rust 特有的接线，见文件头说明）。
    ///
    /// 对应 Java: 各容器构造方法里传入的 `this`（`ProjectContext` 自身即
    /// `UniqueIdObjectSequenceProvider`）。
    sequence_provider: Rc<RefCell<ContextSequenceProvider>>,
}

impl ProjectContext {
    /// 构造方法（各字段就地初始化）。
    ///
    /// 对应 Java: 隐式无参构造方法 `ProjectContext()`。
    ///
    /// 需要跨实体共享的用户应改用 [`ProjectContext::shared`]，它会额外完成子容器与上下文
    /// 之间的反向引用接线。
    pub fn new() -> Self {
        // Java: 各容器构造时传入 `this`；Rust 侧改为传入只持有上下文弱引用的适配器
        // （由 ProjectContext::shared 接线，见文件头说明）。
        let sequence_provider = Rc::new(RefCell::new(ContextSequenceProvider::new()));
        let work_contour_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            sequence_provider.clone();
        let notes_topic_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            sequence_provider.clone();
        let calendar_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            sequence_provider.clone();

        Self {
            units_of_measure: UnitOfMeasureContainer::new(),
            work_contours: WorkContourContainer::new(work_contour_provider),
            notes_topics: NotesTopicContainer::new(notes_topic_provider),
            custom_fields: CustomFieldContainer::new(),
            user_defined_fields: UserDefinedFieldContainer::new(),
            shifts: ShiftContainer::new(),
            shift_periods: ShiftPeriodContainer::new(),
            calendars: ProjectCalendarContainer::new_with_provider(calendar_provider),
            resources: ResourceContainer::new(),
            projects: Vec::new(),
            time_unit_defaults: TimeUnitDefaults::new(),
            project_config: ProjectConfig::new(),
            unique_id_object_sequences: HashMap::new(),
            ignored_errors: Vec::new(),
            event_manager: EventManager::new(),
            sequence_provider,
        }
    }

    /// 构造可共享的上下文（Rust 特有）。
    ///
    /// Java 用 `new ProjectContext()` 得到可直接共享的引用，并在各子容器构造时把 `this`
    /// 传给它们。Rust 无法在 `Self::new()` 内取得自身的 `Rc`，因此这里先构造
    /// `Rc<RefCell<ProjectContext>>`，再为需要回指上下文的子容器（目前是
    /// [`ProjectCalendarContainer`]）完成接线。
    pub fn shared() -> Rc<RefCell<ProjectContext>> {
        let context = Rc::new(RefCell::new(ProjectContext::new()));

        {
            let mut context_ref = context.borrow_mut();
            // Java: `new ProjectCalendarContainer(this)` —— 由本方法补上反向引用接线。
            context_ref.calendars.set_context(&context);

            // Java: `new ResourceContainer(this)` —— 同上（`ResourceContainer` 的序号生成器
            // 提供者也需要上下文才能与 Java 一样共享同一套 `ObjectSequence`）。
            context_ref.resources.set_context(&context);

            // Java: `new WorkContourContainer(this)` / `new NotesTopicContainer(this)`
            // —— 把序号生成器提供者适配器接到本上下文（只写入弱引用，不会再借用上下文）。
            let sequence_provider = Rc::clone(&context_ref.sequence_provider);
            sequence_provider.borrow_mut().set_context(&context);
        }

        context
    }

    /// 取当前上下文可用的计量单位容器。
    ///
    /// 对应 Java: `public UnitOfMeasureContainer getUnitsOfMeasure()`
    pub fn get_units_of_measure(&self) -> &UnitOfMeasureContainer {
        &self.units_of_measure
    }

    /// 取计量单位容器的可变引用（Rust 特有：Java 的 getter 返回可变集合对象）。
    pub fn get_units_of_measure_mut(&mut self) -> &mut UnitOfMeasureContainer {
        &mut self.units_of_measure
    }

    /// 取当前上下文可用的工时分布容器。
    ///
    /// 对应 Java: `public WorkContourContainer getWorkContours()`
    pub fn get_work_contours(&self) -> &WorkContourContainer {
        &self.work_contours
    }

    /// 取工时分布容器的可变引用（Rust 特有）。
    pub fn get_work_contours_mut(&mut self) -> &mut WorkContourContainer {
        &mut self.work_contours
    }

    /// 取当前上下文可用的笔记主题容器。
    ///
    /// 对应 Java: `public NotesTopicContainer getNotesTopics()`
    pub fn get_notes_topics(&self) -> &NotesTopicContainer {
        &self.notes_topics
    }

    /// 取笔记主题容器的可变引用（Rust 特有）。
    pub fn get_notes_topics_mut(&mut self) -> &mut NotesTopicContainer {
        &mut self.notes_topics
    }

    /// 取当前上下文可用的自定义字段容器。
    ///
    /// 对应 Java: `public CustomFieldContainer getCustomFields()`
    ///
    /// [`crate::custom_field_container::CustomFieldContainer::new`] 返回 `Rc<RefCell<Self>>`，
    /// 故 Rust 侧返回共享句柄的克隆；增删字段通过返回值的 `RefCell` 完成，
    /// 因此不额外提供 `_mut` 访问器。
    pub fn get_custom_fields(&self) -> Rc<RefCell<CustomFieldContainer>> {
        Rc::clone(&self.custom_fields)
    }

    /// 取当前上下文可用的用户自定义字段容器。
    ///
    /// 对应 Java: `public UserDefinedFieldContainer getUserDefinedFields()`
    pub fn get_user_defined_fields(&self) -> &UserDefinedFieldContainer {
        &self.user_defined_fields
    }

    /// 取用户自定义字段容器的可变引用（Rust 特有）。
    pub fn get_user_defined_fields_mut(&mut self) -> &mut UserDefinedFieldContainer {
        &mut self.user_defined_fields
    }

    /// 取当前上下文可用的班次容器。
    ///
    /// 对应 Java: `public ShiftContainer getShifts()`
    pub fn get_shifts(&self) -> &ShiftContainer {
        &self.shifts
    }

    /// 取班次容器的可变引用（Rust 特有）。
    pub fn get_shifts_mut(&mut self) -> &mut ShiftContainer {
        &mut self.shifts
    }

    /// 取当前上下文可用的班次时段容器。
    ///
    /// 对应 Java: `public ShiftPeriodContainer getShiftPeriods()`
    pub fn get_shift_periods(&self) -> &ShiftPeriodContainer {
        &self.shift_periods
    }

    /// 取班次时段容器的可变引用（Rust 特有）。
    pub fn get_shift_periods_mut(&mut self) -> &mut ShiftPeriodContainer {
        &mut self.shift_periods
    }

    /// 取当前上下文可用的日历容器。
    ///
    /// 对应 Java: `public ProjectCalendarContainer getCalendars()`
    pub fn get_calendars(&self) -> &ProjectCalendarContainer {
        &self.calendars
    }

    /// 取日历容器的可变引用（Rust 特有）。
    pub fn get_calendars_mut(&mut self) -> &mut ProjectCalendarContainer {
        &mut self.calendars
    }

    /// 创建日历，并在上下文尚可访问时按配置领取唯一 ID。
    ///
    /// 对应 Java: `getCalendars().add()`；Rust 特有的借用拆分入口。
    pub(crate) fn add_calendar(&mut self) -> Rc<RefCell<ProjectCalendar>> {
        let unique_id = self.next_calendar_unique_id(false);
        self.calendars.add_with_unique_id(unique_id)
    }

    /// 创建默认基准日历。
    ///
    /// 对应 Java: `getCalendars().addDefaultBaseCalendar()`；Rust 特有的借用拆分入口。
    pub(crate) fn add_default_base_calendar(&mut self) -> Rc<RefCell<ProjectCalendar>> {
        let unique_id = self.next_calendar_unique_id(false);
        self.calendars.add_default_base_calendar_with_unique_id(unique_id)
    }

    /// 创建默认派生日历。
    ///
    /// 对应 Java: `getCalendars().addDefaultDerivedCalendar()`；Rust 特有的借用拆分入口。
    pub(crate) fn add_default_derived_calendar(&mut self) -> Rc<RefCell<ProjectCalendar>> {
        let unique_id = self.next_calendar_unique_id(false);
        self.calendars.add_default_derived_calendar_with_unique_id(unique_id)
    }

    /// 查找默认日历；空容器时即使自动编号关闭也按 Java 行为分配 ID。
    ///
    /// 对应 Java: `getCalendars().findOrCreateDefaultCalendar()`；Rust 特有的借用拆分入口。
    pub(crate) fn find_or_create_default_calendar(
        &mut self,
    ) -> Option<Rc<RefCell<ProjectCalendar>>> {
        let unique_id = if self.calendars.is_empty() {
            self.next_calendar_unique_id(true)
        } else {
            None
        };
        self.calendars.find_or_create_default_calendar_with_unique_id(unique_id)
    }

    /// 从 Java 日历构造方法使用的共享序列领取下一个 ID。
    ///
    /// 对应 Java: `ProjectCalendar(ProjectContext, boolean)` 中的自动编号；
    /// `findOrCreateDefaultCalendar()` 空容器时可强制领取一次。
    fn next_calendar_unique_id(&mut self, force: bool) -> Option<i32> {
        if force || self.project_config.get_auto_calendar_unique_id() {
            Some(
                self.get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>())
                    .get_next(),
            )
        } else {
            None
        }
    }

    /// 取当前上下文可用的资源容器。
    ///
    /// 对应 Java: `public ResourceContainer getResources()`
    pub fn get_resources(&self) -> &ResourceContainer {
        &self.resources
    }

    /// 取资源容器的可变引用（Rust 特有）。
    pub fn get_resources_mut(&mut self) -> &mut ResourceContainer {
        &mut self.resources
    }

    /// 取本上下文中的项目列表（Rust 侧返回已升级的强引用快照，已释放的项目会被忽略）。
    ///
    /// 对应 Java: `public List<ProjectFile> getProjects()`
    pub fn get_projects(&self) -> Vec<Rc<RefCell<ProjectFile>>> {
        self.projects.iter().filter_map(Weak::upgrade).collect()
    }

    /// 把项目登记到本上下文中（Rust 特有入口）。
    ///
    /// 对应 Java: `ProjectFile` 构造方法中的 `m_context.getProjects().add(this)`
    pub fn add_project(&mut self, project: &Rc<RefCell<ProjectFile>>) {
        self.projects.push(Rc::downgrade(project));
    }

    /// 取上下文中时长单位相关的默认值。
    ///
    /// 对应 Java: `public TimeUnitDefaults getTimeUnitDefaults()`
    pub fn get_time_unit_defaults(&self) -> &TimeUnitDefaults {
        &self.time_unit_defaults
    }

    /// 取时长单位默认值的可变引用（Rust 特有）。
    pub fn get_time_unit_defaults_mut(&mut self) -> &mut TimeUnitDefaults {
        &mut self.time_unit_defaults
    }

    /// 取本上下文各项目共享的 MPXJ 配置。
    ///
    /// 对应 Java: `public ProjectConfig getProjectConfig()`
    pub fn get_project_config(&self) -> &ProjectConfig {
        &self.project_config
    }

    /// 取 MPXJ 配置的可变引用（Rust 特有）。
    pub fn get_project_config_mut(&mut self) -> &mut ProjectConfig {
        &mut self.project_config
    }

    /// 记录一个读取上下文数据时被忽略的错误。
    ///
    /// 对应 Java: `public void addIgnoredError(Exception ex)`
    pub fn add_ignored_error(&mut self, error: MpxjError) {
        self.ignored_errors.push(error);
    }

    /// 取读取上下文数据时被忽略的错误列表。
    ///
    /// 对应 Java: `public List<Exception> getIgnoredErrors()`
    pub fn get_ignored_errors(&self) -> &[MpxjError] {
        &self.ignored_errors
    }

    /// 取事件管理器。
    ///
    /// 对应 Java: `public EventManager getEventManager()`
    pub fn get_event_manager(&self) -> &EventManager {
        &self.event_manager
    }

    /// 取事件管理器的可变引用（Rust 特有：触发事件需要可变访问）。
    pub fn get_event_manager_mut(&mut self) -> &mut EventManager {
        &mut self.event_manager
    }

    /// 移除指定类型的 unique ID 序列，使该类型的编号重新从 1 开始。
    ///
    /// 对应 Java: `public void resetUniqueIdObjectSequence(Class<?> c)`
    ///
    /// Java 用 `c.getName()` 作为 map key，Rust 侧按已移植的 trait 传入 [`TypeId`]。
    pub fn reset_unique_id_object_sequence(&mut self, type_id: TypeId) {
        self.unique_id_object_sequences.remove(&type_id);
    }
}

impl Default for ProjectContext {
    /// 对应 Java: 隐式无参构造方法 `ProjectContext()`。
    fn default() -> Self {
        Self::new()
    }
}

impl UniqueIdObjectSequenceProvider for ProjectContext {
    /// 取（不存在时创建）指定类型的 unique ID 序列。
    ///
    /// 对应 Java: `@Override public ObjectSequence getUniqueIdObjectSequence(Class<?> c)`
    ///
    /// Java 的 map key 是 `Class.getName()`，Rust 侧按 trait 定义改用 [`TypeId`]；
    /// Java 侧 `computeIfAbsent` 返回缓存中的同一个 `ObjectSequence` 实例，
    /// Rust 侧用 `Rc<ObjectSequence>` 表达同一实例的共享。
    fn get_unique_id_object_sequence(&mut self, type_id: TypeId) -> Rc<ObjectSequence> {
        sequence_for(&mut self.unique_id_object_sequences, type_id)
    }
}

/// 取（或创建）指定类型对应的 unique ID 序列。
///
/// 对应 Java: `m_uniqueIdObjectSequences.computeIfAbsent(c.getName(), x -> new ObjectSequence(1))`
fn sequence_for(
    sequences: &mut HashMap<TypeId, Rc<ObjectSequence>>,
    type_id: TypeId,
) -> Rc<ObjectSequence> {
    Rc::clone(
        sequences
            .entry(type_id)
            .or_insert_with(|| Rc::new(ObjectSequence::new(1))),
    )
}

/// 供子容器使用的序号生成器提供者（Rust 特有的接线，见文件头说明）。
///
/// Java 中 `new WorkContourContainer(this)` / `new NotesTopicContainer(this)` 把
/// `ProjectContext` 自身（实现了 `UniqueIdObjectSequenceProvider`）交给容器；Rust 侧若让容器
/// 强引用上下文，会形成 `ProjectContext → 容器 → ProjectContext` 的 `Rc` 循环导致内存泄漏。
/// 因此这里提供一个**只持有上下文弱引用**的适配器：把请求转发给上下文的
/// `getUniqueIdObjectSequence`，与 Java 一样共享同一批序号生成器。
///
/// 上下文尚未接线（[`ProjectContext::new`] 构造期间）或已被借用时，退化为本适配器自带的
/// 独立序号生成器，保证不 panic、不失败（Java 无此情形）。
pub(crate) struct ContextSequenceProvider {
    /// 目标上下文（弱引用，避免 `Rc` 循环）。
    context: Weak<RefCell<ProjectContext>>,
    /// 上下文不可用时的退化序号生成器。
    fallback: HashMap<TypeId, Rc<ObjectSequence>>,
}

impl ContextSequenceProvider {
    pub(crate) fn new() -> Self {
        Self {
            context: Weak::new(),
            fallback: HashMap::new(),
        }
    }

    /// 接线目标上下文（只写入弱引用）。
    fn set_context(&mut self, context: &Rc<RefCell<ProjectContext>>) {
        self.context = Rc::downgrade(context);
    }
}

impl UniqueIdObjectSequenceProvider for ContextSequenceProvider {
    fn get_unique_id_object_sequence(&mut self, type_id: TypeId) -> Rc<ObjectSequence> {
        if let Some(context) = self.context.upgrade() {
            if let Ok(mut context_ref) = context.try_borrow_mut() {
                return context_ref.get_unique_id_object_sequence(type_id);
            }
        }

        Rc::clone(
            self.fallback
                .entry(type_id)
                .or_insert_with(|| Rc::new(ObjectSequence::new(1))),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_calendar::{ProjectCalendar, DEFAULT_BASE_CALENDAR_NAME};
    use crate::time_unit_defaults_container::TimeUnitDefaultsContainer;

    /// 测试用「实体」类型，仅用于区分 `TypeId`。
    struct Alpha;

    /// 测试用「实体」类型，仅用于区分 `TypeId`。
    struct Beta;

    #[test]
    fn new_context_containers_are_empty() {
        let context = ProjectContext::new();

        assert!(context.get_units_of_measure().is_empty());
        assert!(context.get_work_contours().is_empty());
        assert!(context.get_notes_topics().is_empty());
        assert!(context.get_custom_fields().borrow().is_empty());
        assert!(context.get_user_defined_fields().is_empty());
        assert!(context.get_shifts().is_empty());
        assert!(context.get_shift_periods().is_empty());
        assert!(context.get_calendars().is_empty());
        assert!(context.get_resources().is_empty());
        assert!(context.get_projects().is_empty());
        assert!(context.get_ignored_errors().is_empty());
    }

    #[test]
    fn time_unit_defaults_and_project_config_match_java_defaults() {
        let context = ProjectContext::new();

        assert_eq!(
            context.get_time_unit_defaults().get_minutes_per_day(),
            Some(480)
        );
        assert!(context.get_project_config().get_auto_calendar_unique_id());
    }

    #[test]
    fn mutable_accessors_reach_the_same_containers() {
        let mut context = ProjectContext::new();

        context.get_calendars_mut().add_calendar(Rc::new(
            RefCell::new(ProjectCalendar::new(Weak::new(), false)),
        ));
        assert_eq!(context.get_calendars().len(), 1);

        context.get_project_config_mut().set_auto_wbs(false);
        assert!(!context.get_project_config().get_auto_wbs());

        context
            .get_time_unit_defaults_mut()
            .set_minutes_per_day(Some(600));
        assert_eq!(
            context.get_time_unit_defaults().get_minutes_per_day(),
            Some(600)
        );
    }

    #[test]
    fn unique_id_sequences_are_per_type_and_resettable() {
        let mut context = ProjectContext::new();

        // 同一类型共享同一序列
        let sequence = context.get_unique_id_object_sequence(TypeId::of::<Alpha>());
        assert_eq!(sequence.get_next(), 1);
        assert_eq!(sequence.get_next(), 2);
        assert!(Rc::ptr_eq(
            &sequence,
            &context.get_unique_id_object_sequence(TypeId::of::<Alpha>())
        ));

        // 不同类型各自独立计数
        assert_eq!(
            context
                .get_unique_id_object_sequence(TypeId::of::<Beta>())
                .get_next(),
            1
        );

        // 与已知在用的编号同步后，序列跟随其后
        let sequence = context.get_unique_id_object_sequence(TypeId::of::<Alpha>());
        assert_eq!(sequence.sync_or_get_next(Some(10)), 10);
        assert_eq!(sequence.get_next(), 11);

        // reset 后重新从 1 开始
        context.reset_unique_id_object_sequence(TypeId::of::<Alpha>());
        assert_eq!(
            context
                .get_unique_id_object_sequence(TypeId::of::<Alpha>())
                .get_next(),
            1
        );
    }

    #[test]
    fn shared_context_implements_unique_id_object_sequence_provider() {
        let context = ProjectContext::shared();
        let mut guard = context.borrow_mut();
        let provider: &mut ProjectContext = &mut guard;

        assert_eq!(
            UniqueIdObjectSequenceProvider::get_unique_id_object_sequence(
                provider,
                TypeId::of::<Alpha>()
            )
            .get_next(),
            1
        );
    }

    #[test]
    fn ignored_errors_are_collected() {
        let mut context = ProjectContext::new();

        context.add_ignored_error(MpxjError::new("first"));
        context.add_ignored_error(MpxjError::new("second"));

        assert_eq!(context.get_ignored_errors().len(), 2);
        assert_eq!(
            context.get_ignored_errors().first().map(MpxjError::message),
            Some("first")
        );
    }

    #[test]
    fn shared_context_calendar_container_is_usable() {
        let context = ProjectContext::shared();

        let created = {
            let mut context_ref = context.borrow_mut();
            let calendar = context_ref.find_or_create_default_calendar();
            assert_eq!(context_ref.get_calendars().len(), 1);
            calendar
        }
        .expect("空上下文会创建默认基准日历");

        assert_eq!(
            created.borrow().get_name(),
            Some(DEFAULT_BASE_CALENDAR_NAME)
        );
        assert_eq!(created.borrow().get_unique_id(), Some(1));

        // 上下文本身仍然可用（弱引用未造成泄漏或悬垂）
        assert_eq!(context.borrow().get_calendars().len(), 1);
    }

    /// 对应 Java: `ProjectCalendarContainer.add()` 与构造器共用上下文日历序列。
    #[test]
    fn calendar_creation_uses_shared_sequence_while_context_is_borrowed() {
        let context = ProjectContext::shared();
        let (first, second, derived) = {
            let mut context_ref = context.borrow_mut();
            (
                context_ref.add_calendar(),
                context_ref.add_default_base_calendar(),
                context_ref.add_default_derived_calendar(),
            )
        };

        assert_eq!(first.borrow().get_unique_id(), Some(1));
        assert_eq!(second.borrow().get_unique_id(), Some(2));
        assert_eq!(derived.borrow().get_unique_id(), Some(3));
        assert!(context.borrow().get_calendars().get_by_unique_id(Some(2)).is_some());
        assert_eq!(
            context
                .borrow_mut()
                .get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>())
                .get_next(),
            4,
        );
    }

    /// 对应 Java: 自动编号关闭时只有 `findOrCreateDefaultCalendar` 强制补上 ID。
    #[test]
    fn default_calendar_gets_id_even_when_auto_ids_are_disabled() {
        let context = ProjectContext::shared();
        context
            .borrow_mut()
            .get_project_config_mut()
            .set_auto_calendar_unique_id(false);
        let default = context
            .borrow_mut()
            .find_or_create_default_calendar()
            .expect("默认日历");
        let ordinary = context.borrow_mut().add_calendar();

        assert_eq!(default.borrow().get_unique_id(), Some(1));
        assert_eq!(ordinary.borrow().get_unique_id(), None);
        assert_eq!(
            context
                .borrow_mut()
                .get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>())
                .get_next(),
            2
        );
    }

    #[test]
    fn default_matches_new() {
        let context = ProjectContext::default();
        assert!(context.get_calendars().is_empty());
        assert!(context.get_shifts().is_empty());
    }
}
