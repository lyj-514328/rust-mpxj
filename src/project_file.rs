//! 对应 Java: `org/mpxj/ProjectFile.java`
//!
//! 表示一份项目计划（schedule）。它是模型层的收口：对外提供任务、资源、分配、
//! 关系、日历、自定义字段等全部容器，并委托 [`ProjectContext`] 提供跨项目共享的数据。
//!
//! # 继承与接口的还原（组合 + 转发）
//!
//! Java: `public final class ProjectFile implements ChildTaskContainer, ChildResourceContainer,
//! UniqueIdObjectSequenceProvider`。Rust 没有继承，接口按下列方式还原：
//!
//! | Java 接口 | Rust |
//! |---|---|
//! | `ChildTaskContainer`（`getChildTasks` / `addTask`） | 固有方法 [`ProjectFile::get_child_tasks`] / [`ProjectFile::add_task`]（`child_task_container` 模块未在 `src/lib.rs` 声明，故不新建 trait，见下） |
//! | `ChildResourceContainer`（`getChildResources` / `addResource`） | 固有方法 [`ProjectFile::get_child_resources`] / [`ProjectFile::add_resource`]（`child_resource_container` 模块未声明） |
//! | `UniqueIdObjectSequenceProvider`（`getUniqueIdObjectSequence`） | `impl UniqueIdObjectSequenceProvider for ProjectFile`，同时保留同名的固有方法（`crate::task` 未导入该 trait，见下） |
//!
//! # 循环引用与接线（核心设计）
//!
//! Java 里 `ProjectFile` 构造时 `new ProjectContext(this)`（`ProjectContext` 的字段逐个
//! 把自己的 `this` 交给子容器；`ProjectFile` 再把自身登记进 `m_context.getProjects()`）。
//! 这种双向强引用在 Rust 里必须拆开，否则 `Rc` 循环会导致内存泄漏：
//!
//! ```text
//! ProjectFile --(strong Rc)--> ProjectContext
//! ProjectContext --(Weak)--> ProjectFile           // m_projects 存弱引用
//! ProjectFile --(Weak)--> 实体（Task/Resource/…）   // 各实体经弱引用回指文件
//! 子容器（TaskContainer/RelationContainer/…） --(Weak)--> ProjectFile
//! 子容器（ResourceContainer/…） --(Weak)--> ProjectContext
//! ProjectFile --(Weak)--> 自身（self_ref，仅供 find_project 等取回 Rc 句柄）
//! ```
//!
//! 因此 [`ProjectFile::new`] **返回 `Rc<RefCell<ProjectFile>>`**（而不是值），
//! 以便先构造出 `Rc`、再完成所有反向引用接线。接线顺序（[`ProjectFile::new_with_context`]）：
//!
//! 1. 取得 `Rc<RefCell<ProjectContext>>`（[`ProjectFile::new`] 用
//!    [`ProjectContext::shared`]，它会接线日历容器与序号生成器适配器）；
//! 2. 以「空弱引用」构造全部字段：`ProjectProperties`（父文件弱引用）、`TaskContainer`、
//!    `ResourceAssignmentContainer`、`RelationContainer`、展示层容器、`DataLinkContainer`、
//!    `ExternalProjectContainer`，并直接持有 `Rc<RefCell<ProjectContext>>`；
//! 3. 包一层 `Rc::new(RefCell::new(..))` 得到文件句柄 `file`；
//! 4. 回填与接线（**必须**在 `Rc` 已存在之后）：
//!    `self_ref`（自身弱引用）、`properties`（`ProjectProperties::new` 需要父文件弱引用）、
//!    `tasks.set_context(&file)`、`tasks.set_project_context(&context)`、`relations.set_context(&file)`、
//!    `external_projects.set_context(&file)`、`assignments.set_context(&context)`；
//! 5. 向上下文登记本项目：`context.borrow_mut().add_project(&file)`；
//! 6. **补齐上下文中资源的接线**：`context.get_resources_mut().set_context(&context)`
//!    （`ProjectContext::shared` 目前只接线日历容器与序号提供者；`ResourceContainer`
//!    的负责人在其文件头明确要求由 `ProjectFile::new()` 补这一步）。同时补
//!    `calendars.set_context(&context)`，使调用方即使传入未经 `shared()` 的上下文
//!    （[`ProjectContext::new`]）也能正常工作（该调用幂等）。
//!
//! 注意：`ProjectContext` 内部的序号提供者适配器是私有字段，只有
//! [`ProjectContext::shared`] 能接线；因此 [`ProjectFile::new`] 一律使用 `shared()` 构造
//! 上下文，而 [`ProjectFile::new_with_context`] 接受调用方提供的上下文（若该上下文
//! 未经 `shared()`，序号提供者会退化为其自带的计数器，不 panic）。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `m_baselines: Map<Integer, ProjectFile>` | `HashMap<i32, Rc<RefCell<ProjectFile>>>` | 元素为共享句柄；Java 用 `TreeMap`，Rust 用 `HashMap`（键集合相同，仅迭代顺序不同） |
//! | `m_baselineTaskMap: Map<Integer, Map<Task, Task>>` | `HashMap<i32, HashMap<i32, Rc<RefCell<Task>>>>` | `Rc<RefCell<Task>>` 无法作 `HashMap` 键，改用**任务的 unique ID** 作内层键（同一 schedule 内 unique ID 唯一，语义等价，且 `DefaultBaselineStrategy` 已按此约定填充） |
//! | `m_ignoredErrors: List<Exception>` | `Vec<MpxjError>` | 与 `ProjectContext` 的既有取舍一致（统一错误类型） |
//! | `getViews/getTables/getFilters/getGroups` 返回可变容器 | 返回 `&T`（另有 `*_mut`） | Java 的 getter 返回可变集合对象，Rust 的 `&self` getter 无法提供该语义 |
//! | `getCalendars/getResources/getShifts/...`（委托上下文） | 返回 `Ref<'_, T>` | 上下文存放在 `RefCell` 中，无法返回 `&T`；`Ref` 通过 `Deref` 提供同样的字段访问。**若调用方正持有上下文的可变借用**（Java 无此情形），`RefCell::borrow` 会 panic |
//! | `getProjectConfig()` 返回 `ProjectConfig` | 返回 `Ref<'_, ProjectConfig>` | 同上（`ProjectConfig` 由上下文持有） |
//! | `getDefaultCalendar()/setDefaultCalendar()` | 同名 + `Option` | 委托 [`ProjectProperties`]（Java 里也是委托 `m_properties`） |
//! | `setBaseline(ProjectFile)` / `getBaseline()` / `clearBaseline()` | [`ProjectFile::set_default_baseline`] / [`ProjectFile::get_default_baseline`] / [`ProjectFile::clear_default_baseline`] | Rust 无重载，带 `int` 参数的版本分别命名为 `set_baseline` / `get_baseline` / `clear_baseline` |
//! | `getStartDate(Task)` / `getFinishDate(Task)`（私有） | 私有方法 `start_date` / `finish_date` | Rust 命名风格 |
//! | `getPopulatedFields` 使用四类容器无参方法 | 各容器接受父文件传入的默认值及自定义字段 | `ProjectFile` 持有父文件借用时避免通过容器再次借用父文件 |
//! | `toString()` | 未提供 | Java 未覆写 `toString()`（`Debug` 也未派生：依赖的 `ProjectContext` 等未实现 `Debug`） |
//!
//! # 跳过的 Java 方法及原因
//!
//! | Java | 原因 |
//! |---|---|
//! | `getActivityCodes()` / `getProjectCodes()` / `getResourceCodes()` / `getRoleCodes()` / `getResourceAssignmentCodes()` | 对应容器模块未在 `src/lib.rs` 声明（Primavera 等格式使用），`ProjectContext` 亦未提供 |
//! | `getExpenseCategories()` / `getCostAccounts()` / `getLocations()` / `getCurrencies()` | 同上（模块未声明） |
//! | `fixUniqueIdClashes()` 中对**资源分配**容器的调用 | `ResourceAssignmentContainer` 尚未提供 `fix_unique_id_clashes`；日历容器已接入基类索引与冲突修复 |
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。
//!
//! # 已知的重入借用限制（务必阅读）
//!
//! `Task` / `ResourceAssignment` 等实体以 `Weak<RefCell<ProjectFile>>` 回指父文件，
//! 并在需要时调用 `file.borrow()` / `file.borrow_mut()`（见 `crate::task`）。
//! 因此凡是「在 `&mut self`（或 `&self`）上直接触达这些实体的 ProjectFile 方法」，
//! 调用方一旦通过 `file.borrow_mut()` 进入，实体的再次借用就会 panic：
//!
//! - [`ProjectFile::add_task`]：`TaskContainer::add` 会在内部 `Task::new_in_file(&file, ..)`，
//!   而后者需要 `file.borrow()`。Rust 侧提供了等价的**免借用**入口
//!   [`ProjectFile::add_task_shared`]（关联函数，接收文件句柄），请优先使用它。
//! - [`ProjectFile::update_structure`]：`Task::generate_wbs` / `generate_outline_number`
//!   会读取 `file.get_child_tasks()`（即再次借用父文件），从 `&mut self` 进入时必然冲突。
//!   该问题需要 `Task` 侧改用 `try_borrow` 退化（或由调用方在不持有文件借用时驱动容器）
//!   才能真正解决，此处仅作记录。
//! - [`ProjectFile::expand_subprojects`] / `replace_external_tasks` / `replace_relations`：
//!   `Task::expand_subproject` / `Task::add_predecessor` 会再次借用父文件。
//!   外部项目读取回调（[`ExternalProjectContainer::set_project_reader`]）目前尚无调用方：
//!   Java 的 `readFile` 用 `UniversalProjectReader`，而本 crate 没有该类 —— 它靠
//!   `Class.forName` 反射遍历候选 reader 并按 `getFileFormat()` 试读分发，属类别性跳过项。
//!   已落地的 [`crate::mpp::mpp_reader::MPPReader`] /
//!   [`crate::mpx::mpx_reader::MPXReader`] 可由调用方按扩展名分派后接进该回调；
//!   未接线时 `read_file` 恒返回 `None`（见其文件头），外部项目分支因此不会被执行，
//!   接线后需要统一对齐借用策略。
//! - [`ProjectFile::fix_unique_id_clashes`]：容器需要经 `ProjectContext`（关系容器已接线）
//!   或父文件取序号生成器，两者都不可用时退化为「跳过」（容器内部使用 `try_borrow*`，不会 panic）。
//!
//! 这些限制与 `crate::task_container` / `crate::relation_container` 文件头记录的
//! 「重入借用」说明一致，集成阶段需统一处理。
//!
//! # 使用的模型层接口（均已落地，此处仅作索引）
//!
//! ```text
//! TableContainer::new() / FilterContainer::new() / GroupContainer::new() / ViewContainer::new()
//! ```
//! （对应 Java 的无参构造；展示层容器均已落地。）
//!
//! 另：[`ProjectFile::get_tasks`] 返回 `&TaskContainer`，而
//! `crate::default_baseline_strategy` 用 `for task in project.get_tasks()` 迭代，
//! 故本文件为 `&TaskContainer` 补了一个 `IntoIterator` 实现（见文件末尾），
//! 语义等价于 Java 端 `TaskContainer` 本身即可迭代。

use std::any::TypeId;
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use chrono::NaiveDateTime;

use crate::calendar_type::CalendarType;
use crate::common::number_helper::NumberHelper;
use crate::common::object_sequence::ObjectSequence;
use crate::custom_field_container::CustomFieldContainer;
use crate::data_link_container::DataLinkContainer;
use crate::event_manager::EventManager;
use crate::common::populated_fields::Defaults;
use crate::field_key::FieldKey;
use crate::external_project_container::ExternalProjectContainer;
use crate::filter_container::FilterContainer;
use crate::group_container::GroupContainer;
use crate::mpxj_exception::MpxjError;
use crate::notes_topic_container::NotesTopicContainer;
use crate::project_calendar::ProjectCalendar;
use crate::project_calendar_container::ProjectCalendarContainer;
use crate::project_config::ProjectConfig;
use crate::project_context::ProjectContext;
use crate::project_properties::ProjectProperties;
use crate::relation::Builder as RelationBuilder;
use crate::relation_container::RelationContainer;
use crate::resource::Resource;
use crate::resource_assignment_container::ResourceAssignmentContainer;
use crate::resource_container::ResourceContainer;
use crate::shift_container::ShiftContainer;
use crate::shift_period_container::ShiftPeriodContainer;
use crate::table_container::TableContainer;
use crate::task::Task;
use crate::task_container::TaskContainer;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;
use crate::unit_of_measure_container::UnitOfMeasureContainer;
use crate::user_defined_field_container::UserDefinedFieldContainer;
use crate::view_container::ViewContainer;
use crate::work_contour_container::WorkContourContainer;

/// 一份项目计划。
///
/// 对应 Java: `public final class ProjectFile implements ChildTaskContainer,
/// ChildResourceContainer, UniqueIdObjectSequenceProvider`
pub struct ProjectFile {
    /// 项目属性。
    ///
    /// 对应 Java 字段: `private final ProjectProperties m_properties = new ProjectProperties(this);`
    properties: ProjectProperties,

    /// 任务容器。
    ///
    /// 对应 Java 字段: `private final TaskContainer m_tasks = new TaskContainer(this);`
    tasks: TaskContainer,

    /// 资源分配容器。
    ///
    /// 对应 Java 字段: `private final ResourceAssignmentContainer m_assignments = new ResourceAssignmentContainer(this);`
    assignments: ResourceAssignmentContainer,

    /// 关系容器。
    ///
    /// 对应 Java 字段: `private final RelationContainer m_relations = new RelationContainer(this);`
    relations: RelationContainer,

    /// 表格容器。
    ///
    /// 对应 Java 字段: `private final TableContainer m_tables = new TableContainer();`
    tables: TableContainer,

    /// 过滤器容器。
    ///
    /// 对应 Java 字段: `private final FilterContainer m_filters = new FilterContainer();`
    filters: FilterContainer,

    /// 分组容器。
    ///
    /// 对应 Java 字段: `private final GroupContainer m_groups = new GroupContainer();`
    groups: GroupContainer,

    /// 视图容器。
    ///
    /// 对应 Java 字段: `private final ViewContainer m_views = new ViewContainer();`
    views: ViewContainer,

    /// 数据链接容器。
    ///
    /// 对应 Java 字段: `private final DataLinkContainer m_dataLinks = new DataLinkContainer();`
    data_links: DataLinkContainer,

    /// 外部（子）项目容器。
    ///
    /// 对应 Java 字段: `private final ExternalProjectContainer m_externalProjects = new ExternalProjectContainer(this);`
    external_projects: ExternalProjectContainer,

    /// 基线项目，按序号索引（0 为主基线，1..=10 为 Baseline1..Baseline10）。
    ///
    /// 对应 Java 字段: `private final Map<Integer, ProjectFile> m_baselines = new TreeMap<>();`
    baselines: HashMap<i32, Rc<RefCell<ProjectFile>>>,

    /// 当前项目的任务 unique ID → 基线项目任务（用于 `Task.getBaselineTask()`）。
    ///
    /// 对应 Java 字段: `private final Map<Integer, Map<Task, Task>> m_baselineTaskMap = new HashMap<>();`
    /// （内层键改用任务 unique ID，见文件头说明）
    baseline_task_map: HashMap<i32, HashMap<i32, Rc<RefCell<Task>>>>,

    /// 读取本项目时被忽略的错误。
    ///
    /// 对应 Java 字段: `private final List<Exception> m_ignoredErrors = new ArrayList<>();`
    ignored_errors: Vec<MpxjError>,

    /// 项目上下文（强引用；上下文对本文件只存弱引用，故不成环）。
    ///
    /// 对应 Java 字段: `private final ProjectContext m_context;`
    context: Rc<RefCell<ProjectContext>>,

    /// 自身句柄（弱引用，Rust 特有）：供 [`ProjectFile::get_self`] /
    /// [`ProjectFile::find_project`] 在 `&self` 下取回 `Rc` 句柄。
    self_ref: Weak<RefCell<ProjectFile>>,
}

impl ProjectFile {
    /// 构造方法。
    ///
    /// 对应 Java: `public ProjectFile()`
    ///
    /// Rust 侧返回 `Rc<RefCell<ProjectFile>>`（循环引用与接线见文件头）。
    pub fn new() -> Rc<RefCell<ProjectFile>> {
        // Java: this(new ProjectContext());
        ProjectFile::new_with_context(ProjectContext::shared())
    }

    /// 构造方法（接受一个项目上下文）。
    ///
    /// 对应 Java: `public ProjectFile(ProjectContext context)`
    ///
    /// 完整接线顺序见文件头。
    pub fn new_with_context(context: Rc<RefCell<ProjectContext>>) -> Rc<RefCell<ProjectFile>> {
        // 步骤 2：以空弱引用构造全部字段（此时 `Rc` 尚不存在）。
        let project = ProjectFile {
            properties: ProjectProperties::new(Weak::new()),
            tasks: TaskContainer::new(),
            assignments: ResourceAssignmentContainer::new(),
            relations: RelationContainer::new(),
            tables: TableContainer::new(),
            filters: FilterContainer::new(),
            groups: GroupContainer::new(),
            views: ViewContainer::new(),
            data_links: DataLinkContainer::new(),
            external_projects: ExternalProjectContainer::new(),
            baselines: HashMap::new(),
            baseline_task_map: HashMap::new(),
            ignored_errors: Vec::new(),
            context: Rc::clone(&context),
            self_ref: Weak::new(),
        };

        // 步骤 3：得到文件句柄。
        let file = Rc::new(RefCell::new(project));

        // 步骤 4：回填与接线（必须在 `Rc` 存在之后）。
        {
            let mut file_ref = file.borrow_mut();
            file_ref.self_ref = Rc::downgrade(&file);
            file_ref.properties = ProjectProperties::new(Rc::downgrade(&file));
            file_ref.tasks.set_context(&file);
            file_ref.tasks.set_project_context(&context);
            file_ref.relations.set_context(&file);
            // Rust 特有：`RelationContainer::set_context` 内部经 `try_borrow` 读上下文，
            // 而此刻 `file` 正被可变借用（上面的 `file_ref`），必然读不到；关系容器的
            // 自动 unique ID 需要 `ProjectContext`，故在此显式补接线。
            file_ref.relations.set_project_context(&context);
            file_ref.external_projects.set_context(&file);
            file_ref.assignments.set_context(&context);
        }

        // 步骤 5/6：登记到上下文，并补齐上下文各容器的反向接线（幂等）。
        {
            let mut context_ref = context.borrow_mut();
            context_ref.add_project(&file);
            context_ref.get_calendars_mut().set_context(&context);
            context_ref.get_resources_mut().set_context(&context);
        }

        file
    }

    /// 取自身句柄（Rust 特有：Java 用隐含的 `this`）。
    ///
    /// 文件已释放（正常调用路径下不会发生）时返回 `None`。
    pub fn get_self(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.self_ref.upgrade()
    }

    /// 取父项目上下文。
    ///
    /// 对应 Java: `public ProjectContext getProjectContext()`
    pub fn get_project_context(&self) -> Rc<RefCell<ProjectContext>> {
        Rc::clone(&self.context)
    }

    /// 取项目配置。
    ///
    /// 对应 Java: `public ProjectConfig getProjectConfig()`
    ///
    /// Rust 侧返回 `Ref`（配置由 [`ProjectContext`] 持有，见文件头差异表）。
    pub fn get_project_config(&self) -> Ref<'_, ProjectConfig> {
        // Java: return m_context.getProjectConfig();
        Ref::map(self.context.borrow(), |context| context.get_project_config())
    }

    /// 新建一个任务并加入文件（同时登记为项目顶层任务）。
    ///
    /// 对应 Java: `@Override public Task addTask()`
    ///
    /// 与 Java 一致，可在持有 `ProjectFile` 借用时调用 —— 容器侧建任务所需的配置与序号
    /// 取自 `ProjectContext`，不会回调父文件（见 [`crate::TaskContainer::add`]）。
    pub fn add_task(&mut self) -> Rc<RefCell<Task>> {
        // Java: return m_tasks.add();
        self.tasks.add()
    }

    /// 新建一个任务并加入文件（Rust 特有：只持有文件句柄时的入口）。
    ///
    /// 对应 Java: `@Override public Task addTask()`
    ///
    /// 与 [`ProjectFile::add_task`] 行为一致，区别只是接收 `Rc<RefCell<ProjectFile>>`
    /// 而非 `&mut ProjectFile`，便于在只有共享句柄的场景下使用。
    pub fn add_task_shared(file: &Rc<RefCell<ProjectFile>>) -> Rc<RefCell<Task>> {
        // Java: Task task = new Task(m_projectFile, null);
        let task = Rc::new(RefCell::new(Task::new_in_file(file, None)));

        // Java: TaskContainer.add() 的两步：加入容器 + 登记为项目顶层任务
        {
            let mut file_ref = file.borrow_mut();
            file_ref.tasks.add_task(Rc::clone(&task));
            file_ref
                .tasks
                .get_child_tasks_mut()
                .push(Rc::clone(&task));
        }

        task
    }

    /// 从项目中移除一个任务。
    ///
    /// 对应 Java: `public void removeTask(Task task)`
    pub fn remove_task(&mut self, task: &Rc<RefCell<Task>>) {
        // Java: m_tasks.remove(task);
        self.tasks.remove_task(task);
    }

    /// 取全部顶层任务（快照）。
    ///
    /// 对应 Java: `@Override public List<Task> getChildTasks()`
    pub fn get_child_tasks(&self) -> Vec<Rc<RefCell<Task>>> {
        // Java: return m_tasks.getChildTasks();
        self.tasks.get_child_tasks().to_vec()
    }

    /// 取全部顶层资源（快照）。
    ///
    /// 对应 Java: `@Override public List<Resource> getChildResources()`
    pub fn get_child_resources(&self) -> Vec<Rc<RefCell<Resource>>> {
        // Java: return m_context.getResources().getChildResources();
        self.context.borrow().get_resources().get_child_resources()
    }

    /// 取任务容器。
    ///
    /// 对应 Java: `public TaskContainer getTasks()`
    pub fn get_tasks(&self) -> &TaskContainer {
        &self.tasks
    }

    /// 取任务容器的可变引用（Rust 特有：Java 的 getter 返回可变容器）。
    pub fn get_tasks_mut(&mut self) -> &mut TaskContainer {
        &mut self.tasks
    }

    /// 新建一个日历。
    ///
    /// 对应 Java: `public ProjectCalendar addCalendar()`
    pub fn add_calendar(&mut self) -> Rc<RefCell<ProjectCalendar>> {
        // Java: return m_context.getCalendars().add();
        self.context.borrow_mut().get_calendars_mut().add()
    }

    /// 移除一个日历（返回是否命中；Java 返回 `void`）。
    ///
    /// 对应 Java: `public void removeCalendar(ProjectCalendar calendar)`
    pub fn remove_calendar(&mut self, calendar: &Rc<RefCell<ProjectCalendar>>) -> bool {
        // Java: m_context.getCalendars().remove(calendar);
        let context = Rc::clone(&self.context);
        let mut context_ref = context.borrow_mut();
        let Ok(calendar_ref) = calendar.try_borrow() else {
            // Java 无此分支；Rust 侧日历正被可变借用时无法做指针同一性比较
            return false;
        };
        context_ref.get_calendars_mut().remove(&calendar_ref)
    }

    /// 新建名为 “Standard” 的基准日历，并填入默认工作周与默认工作时段。
    ///
    /// 对应 Java: `public ProjectCalendar addDefaultBaseCalendar()`
    pub fn add_default_base_calendar(&mut self) -> Rc<RefCell<ProjectCalendar>> {
        // Java: return m_context.getCalendars().addDefaultBaseCalendar();
        self.context
            .borrow_mut()
            .get_calendars_mut()
            .add_default_base_calendar()
    }

    /// 新建一个默认派生日历。
    ///
    /// 对应 Java: `public ProjectCalendar addDefaultDerivedCalendar()`
    pub fn add_default_derived_calendar(&mut self) -> Rc<RefCell<ProjectCalendar>> {
        // Java: return m_context.getCalendars().addDefaultDerivedCalendar();
        self.context
            .borrow_mut()
            .get_calendars_mut()
            .add_default_derived_calendar()
    }

    /// 取日历容器。
    ///
    /// 对应 Java: `public ProjectCalendarContainer getCalendars()`
    pub fn get_calendars(&self) -> Ref<'_, ProjectCalendarContainer> {
        // Java: return m_context.getCalendars();
        Ref::map(self.context.borrow(), |context| context.get_calendars())
    }

    /// 取全部全局日历，以及与本项目相关的项目级日历。
    ///
    /// 对应 Java: `public List<ProjectCalendar> getCalendarsForProject()`
    pub fn get_calendars_for_project(&self) -> Vec<Rc<RefCell<ProjectCalendar>>> {
        // Java:
        // m_context.getCalendars().stream()
        //    .filter(c -> c.getType() != CalendarType.PROJECT
        //                 || c.getProjectUniqueID().equals(m_properties.getUniqueID()))
        //    .collect(Collectors.toList());
        let context = self.context.borrow();
        let unique_id = self.properties.get_unique_id();
        context
            .get_calendars()
            .iter()
            .filter(|calendar| match calendar.try_borrow() {
                Ok(calendar) => {
                    calendar.get_type() != CalendarType::Project
                        || calendar.get_project_unique_id() == unique_id
                }
                // Java 无此分支（元素必然可读）；Rust 侧避免借用冲突
                Err(_) => false,
            })
            .cloned()
            .collect()
    }

    /// 取项目属性。
    ///
    /// 对应 Java: `public ProjectProperties getProjectProperties()`
    pub fn get_project_properties(&self) -> &ProjectProperties {
        &self.properties
    }

    /// 取项目属性的可变引用（Rust 特有：Java 的 getter 返回可变对象）。
    pub fn get_project_properties_mut(&mut self) -> &mut ProjectProperties {
        &mut self.properties
    }

    /// 新建一个资源。
    ///
    /// 对应 Java: `@Override public Resource addResource()`
    pub fn add_resource(&mut self) -> Rc<RefCell<Resource>> {
        // Java: return m_context.getResources().add();
        self.context.borrow_mut().get_resources_mut().add()
    }

    /// 移除一个资源（返回是否命中；Java 返回 `void`）。
    ///
    /// 对应 Java: `public void removeResource(Resource resource)`
    pub fn remove_resource(&mut self, resource: &Rc<RefCell<Resource>>) -> bool {
        // Java: m_context.getResources().remove(resource);
        self.context
            .borrow_mut()
            .get_resources_mut()
            .remove_resource(resource)
    }

    /// 取资源容器。
    ///
    /// 对应 Java: `public ResourceContainer getResources()`
    pub fn get_resources(&self) -> Ref<'_, ResourceContainer> {
        // Java: return m_context.getResources();
        Ref::map(self.context.borrow(), |context| context.get_resources())
    }

    /// 取资源分配容器。
    ///
    /// 对应 Java: `public ResourceAssignmentContainer getResourceAssignments()`
    pub fn get_resource_assignments(&self) -> &ResourceAssignmentContainer {
        &self.assignments
    }

    /// 取资源分配容器的可变引用（Rust 特有）。
    pub fn get_resource_assignments_mut(&mut self) -> &mut ResourceAssignmentContainer {
        &mut self.assignments
    }

    /// 取关系容器。
    ///
    /// 对应 Java: `public RelationContainer getRelations()`
    pub fn get_relations(&self) -> &RelationContainer {
        &self.relations
    }

    /// 取关系容器的可变引用（Rust 特有）。
    pub fn get_relations_mut(&mut self) -> &mut RelationContainer {
        &mut self.relations
    }

    /// 按名称取日历；找不到时返回 `None`。
    ///
    /// 对应 Java: `public ProjectCalendar getCalendarByName(String calendarName)`
    pub fn get_calendar_by_name(&self, calendar_name: Option<&str>) -> Option<Rc<RefCell<ProjectCalendar>>> {
        // Java: return m_context.getCalendars().getByName(calendarName);
        self.context
            .borrow()
            .get_calendars()
            .get_by_name(calendar_name)
    }

    /// 按唯一 ID 取日历；找不到时返回 `None`。
    ///
    /// 对应 Java: `public ProjectCalendar getCalendarByUniqueID(Integer calendarID)`
    pub fn get_calendar_by_unique_id(
        &self,
        calendar_id: Option<i32>,
    ) -> Option<Rc<RefCell<ProjectCalendar>>> {
        // Java: return m_context.getCalendars().getByUniqueID(calendarID);
        self.context
            .borrow()
            .get_calendars()
            .get_by_unique_id(calendar_id)
    }

    /// 按 ID 取任务；找不到时返回 `None`。
    ///
    /// 对应 Java: `public Task getTaskByID(Integer id)`
    pub fn get_task_by_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Task>>> {
        // Java: return m_tasks.getByID(id);
        self.tasks.get_by_id(id)
    }

    /// 按唯一 ID 取任务；找不到时返回 `None`。
    ///
    /// 对应 Java: `public Task getTaskByUniqueID(Integer id)`
    pub fn get_task_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Task>>> {
        // Java: return m_tasks.getByUniqueID(id);
        self.tasks.get_by_unique_id(id)
    }

    /// 按 ID 取资源；找不到时返回 `None`。
    ///
    /// 对应 Java: `public Resource getResourceByID(Integer id)`
    pub fn get_resource_by_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Resource>>> {
        // Java: return m_context.getResources().getByID(id);
        self.context.borrow().get_resources().get_by_id(id)
    }

    /// 按唯一 ID 取资源；找不到时返回 `None`。
    ///
    /// 对应 Java: `public Resource getResourceByUniqueID(Integer id)`
    pub fn get_resource_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Resource>>> {
        // Java: return m_context.getResources().getByUniqueID(id);
        self.context.borrow().get_resources().get_by_unique_id(id)
    }

    /// 从头重建项目文件的任务 / 资源层级结构。
    ///
    /// 对应 Java: `public void updateStructure()`
    pub fn update_structure(&mut self) {
        // Java: m_tasks.updateStructure(); m_context.getResources().updateStructure();
        //
        // Rust 侧的重入借用限制：Java 的 `TaskContainer.updateStructure()` 内部会回调
        // `m_projectFile` 读取 `ProjectConfig` 的自动编号开关。`ProjectConfig` 存放在
        // `ProjectContext` 的 `RefCell` 中（与调用方持有的 `ProjectFile` 借用互不冲突），
        // 故先把三个开关读出来、再以参数形式传给容器；容器侧因此不再回调父文件，
        // 无需把任务容器临时移出 `ProjectFile`。
        let (auto_wbs, auto_outline_number, auto_outline_level) = {
            let config = self.get_project_config();
            (
                config.get_auto_wbs(),
                config.get_auto_outline_number(),
                config.get_auto_outline_level(),
            )
        };

        self.tasks
            .update_structure(auto_wbs, auto_outline_number, auto_outline_level);

        self.context
            .borrow_mut()
            .get_resources_mut()
            .update_structure();
    }

    /// 取最早的任务开始日期。
    ///
    /// 对应 Java: `public LocalDateTime getEarliestStartDate()`
    pub fn get_earliest_start_date(&self) -> Option<NaiveDateTime> {
        // Java: m_tasks.stream().map(this::getStartDate).filter(Objects::nonNull)
        //       .min(Comparator.naturalOrder()).orElse(null);
        self.tasks
            .iter()
            .filter_map(|task| self.start_date(task))
            .min()
    }

    /// 计算项目的实际开始日期。
    ///
    /// 对应 Java: `LocalDateTime getActualStart()`
    pub(crate) fn get_actual_start(&self) -> Option<NaiveDateTime> {
        // Java: m_tasks.stream().map(Task::getActualStart).filter(Objects::nonNull)
        //       .min(Comparator.naturalOrder()).orElse(null);
        self.tasks
            .iter()
            .filter_map(|task| read_task(task, |task| task.get_actual_start()).flatten())
            .min()
    }

    /// 计算项目的实际完成日期（任一任务缺少实际完成时间时返回 `None`）。
    ///
    /// 对应 Java: `LocalDateTime getActualFinish()`
    pub(crate) fn get_actual_finish(&self) -> Option<NaiveDateTime> {
        // Java:
        // if (m_tasks.stream().map(Task::getActualFinish).anyMatch(Objects::isNull)) { return null; }
        // return m_tasks.stream().map(Task::getActualFinish).filter(Objects::nonNull)
        //       .max(Comparator.naturalOrder()).orElse(null);
        let mut result: Option<NaiveDateTime> = None;
        for task in self.tasks.iter() {
            match read_task(task, |task| task.get_actual_finish()).flatten() {
                Some(value) => result = Some(match result {
                    Some(current) => current.max(value),
                    None => value,
                }),
                None => return None,
            }
        }
        result
    }

    /// 取最晚的任务完成日期。
    ///
    /// 对应 Java: `public LocalDateTime getLatestFinishDate()`
    pub fn get_latest_finish_date(&self) -> Option<NaiveDateTime> {
        // Java: m_tasks.stream().map(this::getFinishDate).filter(Objects::nonNull)
        //       .max(Comparator.naturalOrder()).orElse(null);
        self.tasks
            .iter()
            .filter_map(|task| self.finish_date(task))
            .max()
    }

    /// 取某个任务用于「项目开始日期」的日期。
    ///
    /// 对应 Java: `private LocalDateTime getStartDate(Task task)`
    fn start_date(&self, task: &Rc<RefCell<Task>>) -> Option<NaiveDateTime> {
        // Java: if (NumberHelper.getInt(task.getUniqueID()) == 0) { return null; }
        if NumberHelper::get_int(
            read_task(task, |task| task.get_unique_id()).flatten().map(f64::from),
        ) == 0
        {
            return None;
        }

        // Java: 里程碑取「实际完成时间，缺失时取计划完成时间」；非里程碑取
        //       「实际开始时间，缺失时取计划开始时间」。
        let milestone = read_task(task, |task| task.get_milestone()).unwrap_or(false);
        if milestone {
            read_task(task, |task| task.get_actual_finish())
                .flatten()
                .or_else(|| read_task(task, |task| task.get_finish()).flatten())
        } else {
            read_task(task, |task| task.get_actual_start())
                .flatten()
                .or_else(|| read_task(task, |task| task.get_start()).flatten())
        }
    }

    /// 取某个任务用于「项目完成日期」的日期。
    ///
    /// 对应 Java: `private LocalDateTime getFinishDate(Task task)`
    fn finish_date(&self, task: &Rc<RefCell<Task>>) -> Option<NaiveDateTime> {
        // Java: if (NumberHelper.getInt(task.getUniqueID()) == 0) { return null; }
        if NumberHelper::get_int(
            read_task(task, |task| task.get_unique_id()).flatten().map(f64::from),
        ) == 0
        {
            return None;
        }

        // Java: taskFinishDate = task.getActualFinish();
        //       if (taskFinishDate == null) { taskFinishDate = task.getFinish(); }
        read_task(task, |task| task.get_actual_finish())
            .flatten()
            .or_else(|| read_task(task, |task| task.get_finish()).flatten())
    }

    /// 取视图容器。
    ///
    /// 对应 Java: `public ViewContainer getViews()`
    pub fn get_views(&self) -> &ViewContainer {
        &self.views
    }

    /// 取视图容器的可变引用（Rust 特有）。
    pub fn get_views_mut(&mut self) -> &mut ViewContainer {
        &mut self.views
    }

    /// 取表格容器。
    ///
    /// 对应 Java: `public TableContainer getTables()`
    pub fn get_tables(&self) -> &TableContainer {
        &self.tables
    }

    /// 取表格容器的可变引用（Rust 特有）。
    pub fn get_tables_mut(&mut self) -> &mut TableContainer {
        &mut self.tables
    }

    /// 取过滤器容器。
    ///
    /// 对应 Java: `public FilterContainer getFilters()`
    pub fn get_filters(&self) -> &FilterContainer {
        &self.filters
    }

    /// 取过滤器容器的可变引用（Rust 特有）。
    pub fn get_filters_mut(&mut self) -> &mut FilterContainer {
        &mut self.filters
    }

    /// 取分组容器。
    ///
    /// 对应 Java: `public GroupContainer getGroups()`
    pub fn get_groups(&self) -> &GroupContainer {
        &self.groups
    }

    /// 取分组容器的可变引用（Rust 特有）。
    pub fn get_groups_mut(&mut self) -> &mut GroupContainer {
        &mut self.groups
    }

    /// 取事件管理器。
    ///
    /// 对应 Java: `public EventManager getEventManager()`
    pub fn get_event_manager(&self) -> Ref<'_, EventManager> {
        // Java: return m_context.getEventManager();
        Ref::map(self.context.borrow(), |context| context.get_event_manager())
    }

    /// 取自定义字段容器。
    ///
    /// 对应 Java: `public CustomFieldContainer getCustomFields()`
    ///
    /// [`CustomFieldContainer`] 由上下文以 `Rc<RefCell<..>>` 持有，故直接返回共享句柄。
    pub fn get_custom_fields(&self) -> Rc<RefCell<CustomFieldContainer>> {
        // Java: return m_context.getCustomFields();
        self.context.borrow().get_custom_fields()
    }

    /// 取班次容器。
    ///
    /// 对应 Java: `public ShiftContainer getShifts()`
    pub fn get_shifts(&self) -> Ref<'_, ShiftContainer> {
        // Java: return m_context.getShifts();
        Ref::map(self.context.borrow(), |context| context.get_shifts())
    }

    /// 取班次时段容器。
    ///
    /// 对应 Java: `public ShiftPeriodContainer getShiftPeriods()`
    pub fn get_shift_periods(&self) -> Ref<'_, ShiftPeriodContainer> {
        // Java: return m_context.getShiftPeriods();
        Ref::map(self.context.borrow(), |context| context.get_shift_periods())
    }

    /// 取数据链接容器。
    ///
    /// 对应 Java: `public DataLinkContainer getDataLinks()`
    pub fn get_data_links(&self) -> &DataLinkContainer {
        &self.data_links
    }

    /// 取数据链接容器的可变引用（Rust 特有）。
    pub fn get_data_links_mut(&mut self) -> &mut DataLinkContainer {
        &mut self.data_links
    }

    /// 取用户自定义字段容器。
    ///
    /// 对应 Java: `public UserDefinedFieldContainer getUserDefinedFields()`
    pub fn get_user_defined_fields(&self) -> Ref<'_, UserDefinedFieldContainer> {
        // Java: return m_context.getUserDefinedFields();
        Ref::map(self.context.borrow(), |context| {
            context.get_user_defined_fields()
        })
    }

    /// 返回任务、资源与分配中至少有一个非默认缓存值的字段。
    ///
    /// 对应 Java: `UserDefinedFieldMap` 构造函数依次合并三个容器的
    /// `getPopulatedFields()`；本方法在持有父文件借用时收集句柄并统一扫描。
    pub fn get_populated_entity_fields(&self) -> std::collections::HashSet<FieldKey> {
        let defaults = Defaults::from_project_properties(&self.properties);
        let (custom_tasks, custom_resources, custom_assignments) = {
            let Ok(context) = self.context.try_borrow() else {
                return Default::default();
            };
            let custom = context.get_user_defined_fields();
            (
                custom.get_task_fields().iter().map(FieldKey::from).collect::<Vec<_>>(),
                custom.get_resource_fields().iter().map(FieldKey::from).collect::<Vec<_>>(),
                custom.get_assignment_fields().iter().map(FieldKey::from).collect::<Vec<_>>(),
            )
        };
        let mut result = self.tasks.get_populated_fields(custom_tasks, defaults);
        if let Ok(context) = self.context.try_borrow() {
            result.extend(context.get_resources().get_populated_fields(custom_resources));
        }
        result.extend(self.assignments.get_populated_fields(custom_assignments, defaults));
        result
    }

    /// 返回项目四类实体中至少有一个非默认缓存值的字段。
    ///
    /// 对应 Java: `public Set<FieldType> getPopulatedFields()`。
    pub fn get_populated_fields(&self) -> std::collections::HashSet<FieldKey> {
        let mut result = self.get_populated_entity_fields();
        let custom = self.context.try_borrow().map(|context| {
            context.get_user_defined_fields().get_project_fields().iter()
                .map(FieldKey::from).collect::<Vec<_>>()
        }).unwrap_or_default();
        result.extend(self.properties.get_populated_fields(custom));
        result
    }

    /// 取工时分布容器。
    ///
    /// 对应 Java: `public WorkContourContainer getWorkContours()`
    pub fn get_work_contours(&self) -> Ref<'_, WorkContourContainer> {
        // Java: return m_context.getWorkContours();
        Ref::map(self.context.borrow(), |context| context.get_work_contours())
    }

    /// 取笔记主题容器。
    ///
    /// 对应 Java: `public NotesTopicContainer getNotesTopics()`
    pub fn get_notes_topics(&self) -> Ref<'_, NotesTopicContainer> {
        // Java: return m_context.getNotesTopics();
        Ref::map(self.context.borrow(), |context| context.get_notes_topics())
    }

    /// 取计量单位容器。
    ///
    /// 对应 Java: `public UnitOfMeasureContainer getUnitsOfMeasure()`
    pub fn get_units_of_measure(&self) -> Ref<'_, UnitOfMeasureContainer> {
        // Java: return m_context.getUnitsOfMeasure();
        Ref::map(self.context.borrow(), |context| context.get_units_of_measure())
    }

    /// 取默认日历：按项目属性中的日历名查找，找不到时取项目的第一个日历，
    /// 项目没有日历时新建一个默认基准日历。
    ///
    /// 对应 Java: `public ProjectCalendar getDefaultCalendar()`
    pub fn get_default_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        // Java: return getProjectProperties().getDefaultCalendar();
        self.properties.get_default_calendar()
    }

    /// 设置默认日历（传入 `None` 时不修改，等价 Java 的 `null` 判断）。
    ///
    /// 对应 Java: `public void setDefaultCalendar(ProjectCalendar calendar)`
    pub fn set_default_calendar(&mut self, calendar: Option<Rc<RefCell<ProjectCalendar>>>) {
        // Java: if (calendar != null) { m_properties.setDefaultCalendar(calendar); }
        if let Some(calendar) = calendar {
            self.properties.set_default_calendar(Some(calendar));
        }
    }

    /// 取用于时相基线计算的内部日历：先找基线日历名对应的日历，找不到时退回默认日历。
    ///
    /// 对应 Java: `public ProjectCalendar getBaselineCalendar()`
    pub fn get_baseline_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        // Java:
        // ProjectCalendar result = getCalendarByName(m_properties.getBaselineCalendarName());
        // if (result == null) { result = getDefaultCalendar(); }
        // return result;
        let name = self.properties.get_baseline_calendar_name();
        self.get_calendar_by_name(name.as_deref())
            .or_else(|| self.get_default_calendar())
    }

    /// 取本项目的基线项目（键为基线序号）。
    ///
    /// 对应 Java: `public Map<Integer, ProjectFile> getBaselines()`
    pub fn get_baselines(&self) -> &HashMap<i32, Rc<RefCell<ProjectFile>>> {
        &self.baselines
    }

    /// 把给定项目存为**默认**基线，并用它填充本项目任务的 baseline 属性。
    ///
    /// 对应 Java: `public void setBaseline(ProjectFile baseline)`
    pub fn set_default_baseline(&mut self, baseline: &Rc<RefCell<ProjectFile>>) {
        // Java: setBaseline(baseline, 0);
        self.set_baseline(baseline, 0);
    }

    /// 取默认基线项目。
    ///
    /// 对应 Java: `public ProjectFile getBaseline()`
    pub fn get_default_baseline(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        // Java: return getBaseline(0);
        self.get_baseline(0)
    }

    /// 把给定项目存为 baselineN，并用它填充本项目任务的 baseline 属性。
    ///
    /// 对应 Java: `public void setBaseline(ProjectFile baseline, int index)`
    ///
    /// `index` 为 `0` 时填充默认基线，`1..=10` 填充 Baseline1..Baseline10；
    /// 其余下标只保存、不解释（与 Java 一致）。
    pub fn set_baseline(&mut self, baseline: &Rc<RefCell<ProjectFile>>, index: i32) {
        // Java: m_baselines.put(Integer.valueOf(index), baseline);
        self.baselines.insert(index, Rc::clone(baseline));

        if (0..=10).contains(&index) {
            // Java:
            // if (index == 0) { m_properties.setBaselineDate(baseline.getProjectProperties().getCreationDate()); }
            // else { m_properties.setBaselineDate(index, baseline.getProjectProperties().getCreationDate()); }
            let creation_date = baseline
                .borrow()
                .get_project_properties()
                .get_creation_date();
            if index == 0 {
                self.properties.set_baseline_date(creation_date);
            } else {
                self.properties
                    .set_baseline_date_for_index(index, creation_date);
            }

            // Java: getProjectConfig().getBaselineStrategy().populateBaseline(this, baseline, index);
            //
            // Rust 侧先克隆上下文句柄，使策略引用不借用 `self`，从而可以在下面
            // 把 `self` 作为 `&mut ProjectFile` 传给策略（见文件头「重入借用」说明）。
            let context = Rc::clone(&self.context);
            let context_ref = context.borrow();
            let config = context_ref.get_project_config();
            let Some(strategy) = config.get_baseline_strategy() else {
                return;
            };
            let baseline_ref = baseline.borrow();
            strategy.populate_baseline(self, &baseline_ref, index);
        }
    }

    /// 取 baselineN（`0` 表示主基线，`1..=10` 表示 Baseline1..Baseline10）。
    ///
    /// 对应 Java: `public ProjectFile getBaseline(int index)`
    pub fn get_baseline(&self, index: i32) -> Option<Rc<RefCell<ProjectFile>>> {
        // Java: return m_baselines.get(Integer.valueOf(index));
        self.baselines.get(&index).cloned()
    }

    /// 清除默认基线。
    ///
    /// 对应 Java: `public void clearBaseline()`
    pub fn clear_default_baseline(&mut self) {
        // Java: clearBaseline(0);
        self.clear_baseline(0);
    }

    /// 清除 baselineN 的属性（不把它从 `m_baselines` 中移除，与 Java 一致）。
    ///
    /// 对应 Java: `public void clearBaseline(int index)`
    pub fn clear_baseline(&mut self, index: i32) {
        // Java: getProjectConfig().getBaselineStrategy().clearBaseline(this, index);
        let context = Rc::clone(&self.context);
        let context_ref = context.borrow();
        let config = context_ref.get_project_config();
        let Some(strategy) = config.get_baseline_strategy() else {
            return;
        };
        strategy.clear_baseline(self, index);
    }

    /// 设置「本项目任务 → 基线任务」的映射（新增基线时由策略填充）。
    ///
    /// 对应 Java: `void setBaselineTaskMap(int index, Map<Task, Task> map)`
    ///
    /// 内层键为当前项目任务的 unique ID（见文件头说明）。
    pub fn set_baseline_task_map(
        &mut self,
        index: i32,
        map: HashMap<i32, Rc<RefCell<Task>>>,
    ) {
        // Java: m_baselineTaskMap.put(Integer.valueOf(index), map);
        self.baseline_task_map.insert(index, map);
    }

    /// 取「本项目任务 unique ID → 基线任务」的映射；未设置时返回空映射。
    ///
    /// 对应 Java: `Map<Task, Task> getBaselineTaskMap(int index)`
    pub fn get_baseline_task_map(&self, index: i32) -> HashMap<i32, Rc<RefCell<Task>>> {
        // Java: return m_baselineTaskMap.getOrDefault(Integer.valueOf(index), Collections.emptyMap());
        self.baseline_task_map.get(&index).cloned().unwrap_or_default()
    }

    /// 递归展开当前文件及其子项目中的全部子项目（把子项目任务挂到原「子项目任务」下）。
    ///
    /// 对应 Java: `public void expandSubprojects(boolean replaceExternalTasks)`
    ///
    /// `replace_external_tasks` 为 `true` 时，还会把指向外部任务的
    /// 前驱 / 后继关系改写为指向原始任务，并移除不再需要的外部任务占位符。
    ///
    /// 注意重入借用限制（见文件头）。
    pub fn expand_subprojects(&mut self, replace_external_tasks: bool) {
        // Java: getTasks().stream().map(Task::expandSubproject).filter(Objects::nonNull)
        //       .forEach(p -> p.expandSubprojects(replaceExternalTasks));
        let tasks = self.tasks.as_slice().to_vec();
        let mut expanded_projects: Vec<Rc<RefCell<ProjectFile>>> = Vec::new();

        for task in tasks {
            // Java 的 Task.expandSubproject() 对非外部任务返回 null，这里等价地跳过。
            let subproject_file = match task.try_borrow() {
                Ok(task_ref) => {
                    if !task_ref.get_external_task() && !task_ref.get_external_project() {
                        continue;
                    }
                    task_ref.get_subproject_file()
                }
                // Java 无此分支；Rust 侧避免借用冲突
                Err(_) => continue,
            };

            // Java: Task.expandSubproject() 内部经 getSubprojectObject() 读取外部项目
            //（结果会被 ExternalProjectContainer 缓存）。读取不到时等价于 Java 返回 null。
            let Some(project) = self.external_projects.read(subproject_file.as_deref()) else {
                continue;
            };

            // Java: m_children.addAll(children); return subproject;
            if let Ok(mut task_ref) = task.try_borrow_mut() {
                if task_ref.expand_subproject().is_some() {
                    expanded_projects.push(project);
                }
            }
        }

        for project in expanded_projects {
            // Java: p.expandSubprojects(replaceExternalTasks);
            if let Ok(mut project_ref) = project.try_borrow_mut() {
                project_ref.expand_subprojects(replace_external_tasks);
            }
        }

        if replace_external_tasks {
            // Java: replaceExternalTasks();
            self.replace_external_tasks();
        }
    }

    /// 把指向外部任务的（前驱 / 后继）关系改写为指向跨项目的正确任务，并删除不再需要的
    /// 外部任务占位符。
    ///
    /// 对应 Java: `private void replaceExternalTasks()`
    fn replace_external_tasks(&mut self) {
        // Java:
        // List<Task> externalTasks = new ArrayList<>();
        // findExternalTasks(getChildTasks(), externalTasks);
        let mut external_tasks: Vec<Rc<RefCell<Task>>> = Vec::new();
        self.find_external_tasks(&self.get_child_tasks(), &mut external_tasks);

        // Java:
        // Set<Task> replacedTasks = externalTasks.stream().map(this::replaceRelations)
        //    .filter(Objects::nonNull).collect(Collectors.toSet());
        let mut replaced_tasks: Vec<Rc<RefCell<Task>>> = Vec::new();
        for external_task in external_tasks {
            if let Some(replaced) = self.replace_external_task_relations(&external_task) {
                if !contains_task(&replaced_tasks, &replaced) {
                    replaced_tasks.push(replaced);
                }
            }
        }

        // Java: removeExternalTasks(getChildTasks(), replacedTasks);
        self.remove_external_tasks(self.get_child_tasks(), &replaced_tasks);
    }

    /// 改写某个外部任务的前驱 / 后继关系；成功时返回该外部任务，否则返回 `None`。
    ///
    /// 对应 Java: `private Task replaceRelations(Task externalTask)`
    fn replace_external_task_relations(
        &mut self,
        external_task: &Rc<RefCell<Task>>,
    ) -> Option<Rc<RefCell<Task>>> {
        // Java: ProjectFile originalProjectFile = findProject(externalTask.getSubprojectFile());
        let subproject_file = read_task(external_task, |task| task.get_subproject_file()).flatten();
        let original_project_file = self.find_project(subproject_file.as_deref())?;

        // Java: Task originalTask = findTask(originalProjectFile, externalTask);
        let original_task = self.find_task(&original_project_file, external_task)?;

        // Java: replaceRelations(externalTask, originalTask);
        self.replace_relations(external_task, &original_task);

        // Java: return externalTask;
        Some(Rc::clone(external_task))
    }

    /// 按项目文件名查找相关的 [`ProjectFile`]；找不到时返回 `None`。
    ///
    /// 对应 Java: `private ProjectFile findProject(String name)`
    fn find_project(&mut self, name: Option<&str>) -> Option<Rc<RefCell<ProjectFile>>> {
        // Java:
        // if (name.equals(m_properties.getProjectFilePath())) { return this; }
        // return m_externalProjects.read(name);
        if let Some(name) = name {
            if Some(name) == self.properties.get_project_file_path().as_deref() {
                return self.get_self();
            }
        }

        self.external_projects.read(name)
    }

    /// 在指定项目中查找某个外部任务所代表的原始任务。
    ///
    /// 对应 Java: `private Task findTask(ProjectFile file, Task externalTask)`
    fn find_task(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        external_task: &Rc<RefCell<Task>>,
    ) -> Option<Rc<RefCell<Task>>> {
        // Java:
        // Integer id = externalTask.getSubprojectTaskUniqueID();
        // if (id != null) { Task result = file.getTaskByUniqueID(id); if (result != null) { return result; } }
        // id = externalTask.getSubprojectTaskID();
        // if (id != null) { return file.getTaskByID(id); }
        // return null;
        let (subproject_task_unique_id, subproject_task_id) = read_task(external_task, |task| {
            (task.get_subproject_task_unique_id(), task.get_subproject_task_id())
        })
        .unwrap_or((None, None));

        if let Some(unique_id) = subproject_task_unique_id {
            if let Some(result) = file.borrow().get_task_by_unique_id(Some(unique_id)) {
                return Some(result);
            }
        }

        if let Some(id) = subproject_task_id {
            return file.borrow().get_task_by_id(Some(id));
        }

        None
    }

    /// 把指向外部任务的前驱 / 后继关系改写为指向原始任务。
    ///
    /// 对应 Java: `private void replaceRelations(Task externalTask, Task originalTask)`
    ///
    /// Java 用 `externalTask.getParentFile().getRelations()`；Rust 侧直接使用本文件的
    /// 关系容器（外部任务必然是本文件的子任务，见文件头），结果等价。
    fn replace_relations(
        &mut self,
        external_task: &Rc<RefCell<Task>>,
        original_task: &Rc<RefCell<Task>>,
    ) {
        // Java: 为避免并发修改，先各复制一份关系列表
        let successors = match external_task.try_borrow() {
            Ok(task_ref) => self.relations.get_successors(&task_ref),
            Err(_) => return,
        };
        let predecessors = match external_task.try_borrow() {
            Ok(task_ref) => self.relations.get_predecessors(&task_ref),
            Err(_) => return,
        };

        for original_relation in successors {
            // Java: relations.remove(originalRelation);
            let removed = match original_relation.try_borrow() {
                Ok(relation) => self.relations.remove(&relation),
                Err(_) => false,
            };
            if !removed {
                continue;
            }

            // Java:
            // originalRelation.getSuccessorTask().addPredecessor(
            //    new Relation.Builder().from(originalRelation).predecessorTask(originalTask));
            let successor = original_relation
                .try_borrow()
                .ok()
                .and_then(|relation| relation.get_successor_task());
            let Some(successor) = successor else {
                continue;
            };

            let builder = match original_relation.try_borrow() {
                Ok(relation) => {
                    RelationBuilder::from(&relation).predecessor_task(original_task)
                }
                Err(_) => continue,
            };
            Task::add_predecessor(&successor, builder);
        }

        for original_relation in predecessors {
            // Java: relations.remove(originalRelation);
            let removed = match original_relation.try_borrow() {
                Ok(relation) => self.relations.remove(&relation),
                Err(_) => false,
            };
            if !removed {
                continue;
            }

            // Java: originalTask.addPredecessor(new Relation.Builder().from(originalRelation));
            let builder = match original_relation.try_borrow() {
                Ok(relation) => RelationBuilder::from(&relation),
                Err(_) => continue,
            };
            Task::add_predecessor(original_task, builder);
        }
    }

    /// 递归收集任务层级中的全部外部任务。
    ///
    /// 对应 Java: `private void findExternalTasks(List<Task> tasks, List<Task> externalTasks)`
    fn find_external_tasks(
        &self,
        tasks: &[Rc<RefCell<Task>>],
        external_tasks: &mut Vec<Rc<RefCell<Task>>>,
    ) {
        // Java: externalTasks.addAll(tasks.stream().filter(Task::getExternalTask).collect(...));
        for task in tasks {
            if read_task(task, |task| task.get_external_task()).unwrap_or(false) {
                external_tasks.push(Rc::clone(task));
            }
        }

        // Java: tasks.forEach(t -> findExternalTasks(t.getChildTasks(), externalTasks));
        for task in tasks {
            let children = read_task(task, |task| task.get_child_tasks()).unwrap_or_default();
            self.find_external_tasks(&children, external_tasks);
        }
    }

    /// 递归删除不再需要的外部任务。
    ///
    /// 对应 Java: `private void removeExternalTasks(List<Task> tasks, Set<Task> replacedTasks)`
    ///
    /// Java 用 `tasks.removeIf(replacedTasks::contains)` 直接改动**实时列表**；
    /// Rust 侧没有「实时列表」句柄，故按任务所属父列表移除：顶层任务从
    /// `TaskContainer` 的顶层任务列表移除，子任务用 [`Task::remove_child_task`] 移除。
    fn remove_external_tasks(
        &mut self,
        tasks: Vec<Rc<RefCell<Task>>>,
        replaced_tasks: &[Rc<RefCell<Task>>],
    ) {
        for task in tasks {
            if contains_task(replaced_tasks, &task) {
                let parent_task =
                    read_task(&task, |task| task.get_parent_task()).flatten();
                match parent_task {
                    Some(parent_task) => Task::remove_child_task(&parent_task, &task),
                    None => self
                        .tasks
                        .get_child_tasks_mut()
                        .retain(|child| !Rc::ptr_eq(child, &task)),
                }
                continue;
            }

            let children = read_task(&task, |task| task.get_child_tasks()).unwrap_or_default();
            self.remove_external_tasks(children, replaced_tasks);
        }
    }

    /// 读取器在读完一份 schedule 后调用。
    ///
    /// 对应 Java: `public void readComplete()`
    pub fn read_complete(&mut self) {
        // Java: fixUniqueIdClashes();
        self.fix_unique_id_clashes();
    }

    /// 为出现重复 unique ID 的实体重新编号。
    ///
    /// 对应 Java: `public void fixUniqueIdClashes()`
    ///
    /// Java 还会处理资源分配容器；该 Rust 容器尚未提供冲突修复。
    pub fn fix_unique_id_clashes(&mut self) {
        // Java: getTasks().fixUniqueIdClashes();
        self.tasks.fix_unique_id_clashes();

        // Java: getResources().fixUniqueIdClashes();
        self.context
            .borrow_mut()
            .get_resources_mut()
            .fix_unique_id_clashes();

        // Java: getCalendars().fixUniqueIdClashes();
        let calendar_sequence = self.context.borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>());
        self.context.borrow_mut().get_calendars_mut()
            .fix_unique_id_clashes(calendar_sequence);

        // Java: getResourceAssignments().fixUniqueIdClashes();
        // ResourceAssignmentContainer 未提供该方法（同上）。

        // Java: getRelations().fixUniqueIdClashes();
        self.relations.fix_unique_id_clashes();
    }

    /// 取用于为指定类型生成 unique ID 的序号生成器。
    ///
    /// 对应 Java: `@Override public ObjectSequence getUniqueIdObjectSequence(Class<?> c)`
    ///
    /// （同时提供了 `UniqueIdObjectSequenceProvider` 的 trait 实现，见
    /// `impl UniqueIdObjectSequenceProvider for ProjectFile`。）
    pub fn get_unique_id_object_sequence(&mut self, type_id: TypeId) -> Rc<ObjectSequence> {
        // Java: return m_context.getUniqueIdObjectSequence(c);
        self.context
            .borrow_mut()
            .get_unique_id_object_sequence(type_id)
    }

    /// 记录一个读取本项目时被忽略的错误。
    ///
    /// 对应 Java: `public void addIgnoredError(Exception ex)`
    pub fn add_ignored_error(&mut self, error: MpxjError) {
        // Java: m_ignoredErrors.add(ex);
        self.ignored_errors.push(error);
    }

    /// 取读取本项目时被忽略的错误列表。
    ///
    /// 对应 Java: `public List<Exception> getIgnoredErrors()`
    pub fn get_ignored_errors(&self) -> &[MpxjError] {
        // Java: return m_ignoredErrors;
        &self.ignored_errors
    }

    /// 登记一个已存在的外部（子）项目文件。
    ///
    /// 对应 Java: `void addExternalProject(String fileName, ProjectFile projectFile)`
    ///
    /// Rust 侧两个参数均可为 `None`（`None` 文件名或 `None` 项目不登记），
    /// 与调用方（`crate::task`）的可空语义一致。
    pub(crate) fn add_external_project(
        &mut self,
        file_name: Option<&str>,
        project_file: Option<Rc<RefCell<ProjectFile>>>,
    ) {
        // Java: m_externalProjects.add(fileName, projectFile);
        let Some(file_name) = file_name else {
            return;
        };
        self.external_projects.add(file_name, project_file);
    }

    /// 读取指定的外部（子）项目文件（结果会被缓存）。
    ///
    /// 对应 Java: `ProjectFile readExternalProject(String fileName)`
    pub(crate) fn read_external_project(
        &mut self,
        file_name: Option<&str>,
    ) -> Option<Rc<RefCell<ProjectFile>>> {
        // Java: return m_externalProjects.read(fileName);
        self.external_projects.read(file_name)
    }
}

impl UniqueIdObjectSequenceProvider for ProjectFile {
    /// 取（必要时创建）指定类型的序号生成器（转发给 [`ProjectContext`]）。
    ///
    /// 对应 Java: `@Override public ObjectSequence getUniqueIdObjectSequence(Class<?> c)`
    fn get_unique_id_object_sequence(&mut self, type_id: TypeId) -> Rc<ObjectSequence> {
        ProjectFile::get_unique_id_object_sequence(self, type_id)
    }
}

/// 以 `try_borrow` 读取任务字段，已被借用时返回 `None`（不 panic）。
fn read_task<T>(task: &Rc<RefCell<Task>>, read: impl FnOnce(&Task) -> T) -> Option<T> {
    task.try_borrow().ok().map(|task| read(&task))
}

/// 判断任务列表中是否包含指定任务（`Rc` 指针同一性，等价 Java 的引用相等）。
fn contains_task(tasks: &[Rc<RefCell<Task>>], task: &Rc<RefCell<Task>>) -> bool {
    tasks.iter().any(|candidate| Rc::ptr_eq(candidate, task))
}

impl<'a> IntoIterator for &'a TaskContainer {
    /// 迭代任务容器中的任务。
    ///
    /// Rust 特有（见文件头）：`crate::default_baseline_strategy` 按
    /// `for task in project.get_tasks()` 书写，而 [`ProjectFile::get_tasks`] 返回
    /// `&TaskContainer`，故在此为引用补上 `IntoIterator`（Java 侧 `TaskContainer`
    /// 本身即可迭代）。
    type Item = &'a Rc<RefCell<Task>>;
    type IntoIter = std::slice::Iter<'a, Rc<RefCell<Task>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// 注：Java 的 `m_context.getXxx()` 返回可变容器对象；Rust 侧上下文中的容器处于
// `RefCell` 之后，无法返回 `&mut T`。需要访问上下文中**可变**容器时，请经
// `ProjectFile::get_project_context` 取得上下文句柄，例如：
// `file.borrow().get_project_context().borrow_mut().get_calendars_mut()`。

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assignment_field::AssignmentField;
    use crate::data_link::DataLink;
    use crate::field_container::FieldContainer;
    use crate::field_key::FieldKey;
    use crate::field_value::FieldValue;
    use crate::project_entity_with_id::ProjectEntityWithID;
    use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
    use crate::resource_assignment::ResourceAssignment;
    use crate::resource_field::ResourceField;
    use crate::task_field::TaskField;

    /// 新建一份项目文件。
    fn project() -> Rc<RefCell<ProjectFile>> {
        ProjectFile::new()
    }

    /// 在指定文件中新建一个任务（走免借用入口）。
    fn add_task(file: &Rc<RefCell<ProjectFile>>) -> Rc<RefCell<Task>> {
        ProjectFile::add_task_shared(file)
    }

    // -----------------------------------------------------------------------
    // 根容器装配
    // -----------------------------------------------------------------------

    #[test]
    fn new_registers_project_in_context_and_wires_resources() {
        let file = project();
        let context = file.borrow().get_project_context();

        // Java: 构造方法中的 m_context.getProjects().add(this)
        let projects = context.borrow().get_projects();
        assert_eq!(projects.len(), 1);
        assert!(Rc::ptr_eq(&projects[0], &file));

        // ProjectContext::shared 只接线日历容器；资源容器由 ProjectFile::new 补齐
        assert!(context.borrow().get_resources().get_context().is_some());

        // 资源分配容器持有上下文弱引用（构造时接线）
        assert!(file
            .borrow()
            .get_resource_assignments()
            .get_context()
            .is_some());
    }

    /// 创建任务时读取真实项目配置并使用上下文共享序号。
    #[test]
    fn new_tasks_share_the_context_unique_id_sequence() {
        let file = project();
        let first = add_task(&file);
        let second = add_task(&file);
        assert_eq!(first.borrow().get_unique_id(), Some(1));
        assert_eq!(second.borrow().get_unique_id(), Some(2));
        let context = file.borrow().get_project_context();
        let sequence = context.borrow_mut().get_unique_id_object_sequence(TypeId::of::<Task>());
        assert_eq!(sequence.get_next(), 3);
    }

    /// 字段扫描只收集非默认缓存值，并涵盖四类字段容器。
    #[test]
    fn populated_fields_distinguish_defaults_across_containers() {
        let file = project();
        let task = add_task(&file);
        task.borrow_mut().set(FieldKey::Task(TaskField::Text1), Some(FieldValue::String(String::new())));
        task.borrow_mut().set(FieldKey::Task(TaskField::Text2), Some(FieldValue::String("内容".into())));
        task.borrow_mut().set(FieldKey::Task(TaskField::Flag1), Some(FieldValue::Boolean(false)));
        let context = file.borrow().get_project_context();
        let resource = context.borrow_mut().get_resources_mut().add();
        resource.borrow_mut().set(FieldKey::Resource(ResourceField::Number1), Some(FieldValue::Double(0.0)));
        resource.borrow_mut().set(FieldKey::Resource(ResourceField::Text1), Some(FieldValue::String("资源".into())));
        let assignment = Rc::new(RefCell::new(ResourceAssignment::new()));
        assignment.borrow_mut().set(FieldKey::Assignment(AssignmentField::Flag1), Some(FieldValue::Boolean(true)));
        file.borrow_mut().get_resource_assignments_mut().add(assignment);

        let fields = file.borrow().get_populated_fields();
        assert!(fields.contains(&FieldKey::Task(TaskField::Text2)));
        assert!(fields.contains(&FieldKey::Resource(ResourceField::Text1)));
        assert!(fields.contains(&FieldKey::Assignment(AssignmentField::Flag1)));
        assert!(!fields.contains(&FieldKey::Task(TaskField::Text1)));
        assert!(!fields.contains(&FieldKey::Task(TaskField::Flag1)));
        assert!(!fields.contains(&FieldKey::Resource(ResourceField::Number1)));
    }

    #[test]
    fn new_with_context_reuses_the_supplied_context() {
        let context = ProjectContext::shared();
        let first = ProjectFile::new_with_context(Rc::clone(&context));
        let second = ProjectFile::new_with_context(Rc::clone(&context));

        assert_eq!(context.borrow().get_projects().len(), 2);

        // 两个项目共享同一上下文（与 Java 一致）
        assert!(Rc::ptr_eq(
            &first.borrow().get_project_context(),
            &second.borrow().get_project_context()
        ));

        // 共享的序号生成器
        let next = first
            .borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<Task>())
            .get_next();
        let next_again = second
            .borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<Task>())
            .get_next();
        assert_eq!(next, 1);
        assert_eq!(next_again, 2);
    }

    #[test]
    fn containers_start_empty_and_are_reachable() {
        let file = project();
        let file_ref = file.borrow();

        assert!(file_ref.get_tasks().is_empty());
        assert!(file_ref.get_child_tasks().is_empty());
        assert!(file_ref.get_child_resources().is_empty());
        assert!(file_ref.get_resource_assignments().is_empty());
        assert!(file_ref.get_relations().is_empty());
        assert!(file_ref.get_calendars().is_empty());
        assert!(file_ref.get_resources().is_empty());
        assert!(file_ref.get_tables().is_empty());
        // FilterContainer 未提供 `len` / `is_empty`（其作者按名称 / ID 查找提供访问）
        let _ = file_ref.get_filters();
        assert!(file_ref.get_groups().is_empty());
        assert!(file_ref.get_views().is_empty());
        assert!(file_ref.get_data_links().is_empty());
        assert!(file_ref.get_shifts().is_empty());
        assert!(file_ref.get_shift_periods().is_empty());
        assert!(file_ref.get_units_of_measure().is_empty());
        assert!(file_ref.get_work_contours().is_empty());
        assert!(file_ref.get_notes_topics().is_empty());
        assert!(file_ref.get_user_defined_fields().is_empty());
        assert!(file_ref.get_baselines().is_empty());
        assert!(file_ref.get_ignored_errors().is_empty());

        // 项目配置来自上下文（`Ref` 经 `Deref` 提供同样的访问方式）
        assert!(file_ref.get_project_config().get_auto_task_id());

        // 事件管理器可用（`Ref<EventManager>`，来自上下文）
        let _ = file_ref.get_event_manager();
    }

    #[test]
    fn data_links_are_owned_by_the_file() {
        let file = project();

        {
            let mut file_ref = file.borrow_mut();
            file_ref
                .get_data_links_mut()
                .add(Rc::new(RefCell::new(DataLink::new("1"))));
        }

        let file_ref = file.borrow();
        assert_eq!(file_ref.get_data_links().len(), 1);
        assert_eq!(
            file_ref.get_data_links().get(0).map(|link| link.borrow().get_id().to_owned()),
            Some("1".to_owned())
        );
    }

    #[test]
    fn ignored_errors_are_collected() {
        let file = project();

        {
            let mut file_ref = file.borrow_mut();
            file_ref.add_ignored_error(MpxjError::new("first"));
            file_ref.add_ignored_error(MpxjError::new("second"));
        }

        let file_ref = file.borrow();
        assert_eq!(file_ref.get_ignored_errors().len(), 2);
        assert_eq!(
            file_ref.get_ignored_errors().first().map(MpxjError::message),
            Some("first")
        );
    }

    // -----------------------------------------------------------------------
    // 任务 / 资源 / 分配 增删
    // -----------------------------------------------------------------------

    #[test]
    fn add_and_remove_task() {
        let file = project();

        let task = add_task(&file);
        assert_eq!(file.borrow().get_tasks().len(), 1);
        assert_eq!(file.borrow().get_child_tasks().len(), 1);

        // Java: 自动分配 unique ID 与 ID（ProjectConfig 默认开启）
        let unique_id = task.borrow().get_unique_id();
        let id = task.borrow().get_id();
        assert!(unique_id.is_some());
        assert!(id.is_some());

        // 按 ID / unique ID 查找
        assert!(Rc::ptr_eq(
            &file.borrow().get_task_by_unique_id(unique_id).expect("已登记"),
            &task
        ));

        // ID 索引（Java 的 `m_idMap`）由 reader 经 `mapID` 填充；`addTask()` 自身不登记 ID
        file.borrow_mut()
            .get_tasks_mut()
            .map_id(id.expect("已分配 ID"), Rc::clone(&task));
        assert!(Rc::ptr_eq(
            &file.borrow().get_task_by_id(id).expect("已登记"),
            &task
        ));

        file.borrow_mut().remove_task(&task);
        assert!(file.borrow().get_tasks().is_empty());
        assert!(file.borrow().get_child_tasks().is_empty());
    }

    #[test]
    fn add_and_remove_resource() {
        let file = project();

        let resource = file.borrow_mut().add_resource();
        assert_eq!(file.borrow().get_resources().len(), 1);
        assert_eq!(file.borrow().get_child_resources().len(), 1);

        let unique_id = resource.borrow().get_unique_id();
        let id = resource.borrow().get_id();
        assert!(unique_id.is_some());
        assert!(id.is_some());

        assert!(Rc::ptr_eq(
            &file
                .borrow()
                .get_resource_by_unique_id(unique_id)
                .expect("已登记"),
            &resource
        ));
        assert!(Rc::ptr_eq(
            &file.borrow().get_resource_by_id(id).expect("已登记"),
            &resource
        ));

        assert!(file.borrow_mut().remove_resource(&resource));
        assert!(file.borrow().get_resources().is_empty());
        assert!(file.borrow().get_child_resources().is_empty());

        // 重复移除返回 false
        assert!(!file.borrow_mut().remove_resource(&resource));
    }

    #[test]
    fn add_and_remove_assignment() {
        let file = project();

        let task = add_task(&file);
        let resource = file.borrow_mut().add_resource();

        let assignment = Rc::new(RefCell::new(ResourceAssignment::with_parent_file(&file)));
        {
            let mut assignment_ref = assignment.borrow_mut();
            assignment_ref.set_task_unique_id(task.borrow().get_unique_id());
            assignment_ref.set_resource_unique_id(resource.borrow().get_unique_id());
        }

        file.borrow_mut()
            .get_resource_assignments_mut()
            .add(Rc::clone(&assignment));
        assert_eq!(file.borrow().get_resource_assignments().len(), 1);

        // 按任务 / 资源 unique ID 索引
        assert_eq!(
            file.borrow()
                .get_resource_assignments()
                .get_by_task_unique_id(task.borrow().get_unique_id())
                .len(),
            1
        );

        let unique_id = assignment.borrow().get_unique_id();
        assert!(Rc::ptr_eq(
            &file
                .borrow()
                .get_resource_assignments()
                .get_by_unique_id(unique_id)
                .expect("已登记"),
            &assignment
        ));

        assert!(file
            .borrow_mut()
            .get_resource_assignments_mut()
            .remove(&assignment));
        assert!(file.borrow().get_resource_assignments().is_empty());
    }

    #[test]
    fn project_dates_are_calculated_from_tasks() {
        let file = project();

        let first = add_task(&file);
        let second = add_task(&file);

        let start = NaiveDateTime::parse_from_str("2024-01-01 08:00", "%Y-%m-%d %H:%M").ok();
        let finish = NaiveDateTime::parse_from_str("2024-01-05 17:00", "%Y-%m-%d %H:%M").ok();
        first.borrow_mut().set_start(start);
        first.borrow_mut().set_finish(finish);
        second.borrow_mut().set_start(NaiveDateTime::parse_from_str(
            "2023-12-01 08:00",
            "%Y-%m-%d %H:%M",
        )
        .ok());

        let file_ref = file.borrow();
        assert_eq!(
            file_ref.get_earliest_start_date(),
            NaiveDateTime::parse_from_str("2023-12-01 08:00", "%Y-%m-%d %H:%M").ok()
        );
        assert_eq!(file_ref.get_latest_finish_date(), finish);

        // 实际时间缺失时 get_actual_start / get_actual_finish 行为与 Java 一致
        assert_eq!(file_ref.get_actual_start(), None);
        first
            .borrow_mut()
            .set_actual_start(NaiveDateTime::parse_from_str(
                "2024-01-01 08:00",
                "%Y-%m-%d %H:%M",
            )
            .ok());
        assert!(file_ref.get_actual_start().is_some());

        // 任一任务缺少 actualFinish 时整体返回 None
        assert_eq!(file_ref.get_actual_finish(), None);
        second
            .borrow_mut()
            .set_actual_finish(NaiveDateTime::parse_from_str(
                "2024-01-03 17:00",
                "%Y-%m-%d %H:%M",
            )
            .ok());
        first
            .borrow_mut()
            .set_actual_finish(NaiveDateTime::parse_from_str(
                "2024-01-05 17:00",
                "%Y-%m-%d %H:%M",
            )
            .ok());
        assert_eq!(
            file_ref.get_actual_finish(),
            NaiveDateTime::parse_from_str("2024-01-05 17:00", "%Y-%m-%d %H:%M").ok()
        );
    }

    // -----------------------------------------------------------------------
    // 日历解析
    // -----------------------------------------------------------------------

    #[test]
    fn default_base_calendar_is_registered_and_resolvable() {
        let file = project();

        let calendar = file.borrow_mut().add_default_base_calendar();
        assert_eq!(
            calendar.borrow().get_name(),
            Some(crate::project_calendar::DEFAULT_BASE_CALENDAR_NAME)
        );

        let file_ref = file.borrow();
        assert_eq!(file_ref.get_calendars().len(), 1);

        // 按名称 / 唯一 ID 解析
        let by_name = file_ref
            .get_calendar_by_name(Some(crate::project_calendar::DEFAULT_BASE_CALENDAR_NAME))
            .expect("按名称可查到");
        assert!(Rc::ptr_eq(&by_name, &calendar));

        // `addDefaultBaseCalendar` 自身不分配唯一 ID（Java 由 `findOrCreateDefaultCalendar`
        // 或 reader 分配），这里显式设置后验证按唯一 ID 的解析
        drop(file_ref);
        calendar.borrow_mut().set_unique_id(Some(7));
        let file_ref = file.borrow();

        let by_id = file_ref
            .get_calendar_by_unique_id(Some(7))
            .expect("按唯一 ID 可查到");
        assert!(Rc::ptr_eq(&by_id, &calendar));

        assert!(file_ref.get_calendar_by_name(Some("Missing")).is_none());
        assert!(file_ref.get_calendar_by_unique_id(Some(999)).is_none());
    }

    #[test]
    fn default_calendar_and_baseline_calendar_resolution() {
        let file = project();

        let calendar = file.borrow_mut().add_default_base_calendar();
        file.borrow_mut()
            .set_default_calendar(Some(Rc::clone(&calendar)));

        let file_ref = file.borrow();
        assert!(Rc::ptr_eq(
            &file_ref.get_default_calendar().expect("已设置默认日历"),
            &calendar
        ));

        // 基线日历：ProjectProperties 中未记录基线日历名时退回默认日历
        assert!(Rc::ptr_eq(
            &file_ref.get_baseline_calendar().expect("退回默认日历"),
            &calendar
        ));
    }

    #[test]
    fn calendars_for_project_filters_project_type_calendars() {
        let file = project();
        let project_unique_id = file.borrow().get_project_properties().get_unique_id();

        let global = file.borrow_mut().add_calendar();
        {
            let mut global_ref = global.borrow_mut();
            global_ref.set_type(Some(CalendarType::Global));
            global_ref.set_name(Some("Global".to_owned()));
        }

        let mine = file.borrow_mut().add_calendar();
        {
            let mut mine_ref = mine.borrow_mut();
            mine_ref.set_type(Some(CalendarType::Project));
            mine_ref.set_project_unique_id(project_unique_id);
            mine_ref.set_name(Some("Mine".to_owned()));
        }

        let other = file.borrow_mut().add_calendar();
        {
            let mut other_ref = other.borrow_mut();
            other_ref.set_type(Some(CalendarType::Project));
            other_ref.set_project_unique_id(Some(999));
            other_ref.set_name(Some("Other".to_owned()));
        }

        let calendars = file.borrow().get_calendars_for_project();
        // 全局日历全部保留；项目级日历只保留属于本项目的
        assert_eq!(calendars.len(), 2);
        assert!(calendars.iter().any(|calendar| Rc::ptr_eq(calendar, &global)));
        assert!(calendars.iter().any(|calendar| Rc::ptr_eq(calendar, &mine)));
        assert!(!calendars.iter().any(|calendar| Rc::ptr_eq(calendar, &other)));
    }

    #[test]
    fn calendar_can_be_removed() {
        let file = project();
        let calendar = file.borrow_mut().add_default_base_calendar();

        assert!(file.borrow_mut().remove_calendar(&calendar));
        assert!(file.borrow().get_calendars().is_empty());
        assert!(!file.borrow_mut().remove_calendar(&calendar));
    }

    // -----------------------------------------------------------------------
    // 基线
    // -----------------------------------------------------------------------

    /// 建立「当前项目任务 ↔ 基线项目任务」可匹配的一对文件（GUID 相同）。
    fn project_with_baseline() -> (
        Rc<RefCell<ProjectFile>>,
        Rc<RefCell<ProjectFile>>,
        Rc<RefCell<Task>>,
        Rc<RefCell<Task>>,
    ) {
        let file = project();
        let current_task = add_task(&file);

        let baseline = project();
        let baseline_task = add_task(&baseline);

        // DefaultBaselineStrategy 以任务的 GUID 作为匹配键，两侧必须一致
        let guid = uuid::Uuid::parse_str("11111111-1111-1111-1111-111111111111").ok();
        current_task.borrow_mut().set_guid(guid);
        baseline_task.borrow_mut().set_guid(guid);

        (file, baseline, current_task, baseline_task)
    }

    #[test]
    fn set_baseline_stores_project_and_populates_task_map() {
        let (file, baseline, current_task, baseline_task) = project_with_baseline();

        let task_guid = current_task.borrow().get_guid();
        assert!(task_guid.is_some(), "任务应自动分配 GUID");

        file.borrow_mut().set_default_baseline(&baseline);

        // Java: m_baselines.put(0, baseline)
        assert!(Rc::ptr_eq(
            &file.borrow().get_default_baseline().expect("已设置默认基线"),
            &baseline
        ));
        assert!(Rc::ptr_eq(
            &file.borrow().get_baseline(0).expect("已设置基线 0"),
            &baseline
        ));
        assert!(file.borrow().get_baseline(5).is_none());

        // Java: populateBaseline 会填充 baselineTaskMap（Rust 侧内层键为任务 unique ID）
        let map = file.borrow().get_baseline_task_map(0);
        let current_unique_id = current_task.borrow().get_unique_id().expect("有 unique ID");
        let mapped = map.get(&current_unique_id).expect("基线任务已映射");
        assert!(Rc::ptr_eq(mapped, &baseline_task));

        // 未设置的序号返回空映射
        assert!(file.borrow().get_baseline_task_map(3).is_empty());
    }

    #[test]
    fn baseline_attributes_are_copied_from_the_baseline_project() {
        let (file, baseline, current_task, baseline_task) = project_with_baseline();

        let cost = Some(123.0);
        baseline_task.borrow_mut().set_cost(cost);

        file.borrow_mut().set_default_baseline(&baseline);

        // Java: populateBaseline 把 baseline 的 Cost 写入当前任务的 BaselineCost
        assert_eq!(current_task.borrow().get_baseline_cost(), cost);
        assert_eq!(
            current_task
                .borrow()
                .get_cached_value(FieldKey::Task(TaskField::BaselineCost))
                .cloned(),
            Some(crate::field_value::FieldValue::Double(123.0))
        );

        // Java: 基线日期取基线项目的创建日期
        let creation_date = baseline
            .borrow()
            .get_project_properties()
            .get_creation_date();
        assert_eq!(
            file.borrow().get_project_properties().get_baseline_date(),
            creation_date
        );
    }

    #[test]
    fn numbered_baseline_is_stored_and_cleared() {
        let (file, baseline, current_task, baseline_task) = project_with_baseline();
        baseline_task.borrow_mut().set_cost(Some(50.0));

        file.borrow_mut().set_baseline(&baseline, 3);
        assert!(Rc::ptr_eq(
            &file.borrow().get_baseline(3).expect("已设置基线 3"),
            &baseline
        ));
        // 序号 3 填充的是 Baseline3Cost（而不是默认基线的 BaselineCost）
        assert_eq!(
            current_task
                .borrow()
                .get_cached_value(FieldKey::Task(TaskField::Baseline3Cost))
                .cloned(),
            Some(crate::field_value::FieldValue::Double(50.0))
        );
        assert_eq!(current_task.borrow().get_baseline_cost(), None);

        // Java: clearBaseline(index) 通过策略把对应字段清空
        file.borrow_mut().clear_baseline(3);
        assert_eq!(
            current_task
                .borrow()
                .get_cached_value(FieldKey::Task(TaskField::Baseline3Cost))
                .cloned(),
            None
        );

        // 越界的序号只保存不解释
        let other = project();
        file.borrow_mut().set_baseline(&other, 11);
        assert!(Rc::ptr_eq(
            &file.borrow().get_baseline(11).expect("已保存"),
            &other
        ));
        assert!(current_task.borrow().get_baseline_cost().is_none());
    }

    #[test]
    fn clear_default_baseline_clears_attributes_but_keeps_the_map() {
        let (file, baseline, current_task, baseline_task) = project_with_baseline();
        baseline_task.borrow_mut().set_cost(Some(75.0));
        file.borrow_mut().set_default_baseline(&baseline);
        assert_eq!(current_task.borrow().get_baseline_cost(), Some(75.0));

        file.borrow_mut().clear_default_baseline();
        assert_eq!(current_task.borrow().get_baseline_cost(), None);
        // Java: clearBaseline 不会把基线从 m_baselines 中移除
        assert!(file.borrow().get_default_baseline().is_some());
    }

    // -----------------------------------------------------------------------
    // 结构重建 / unique ID / 外部项目
    // -----------------------------------------------------------------------

    #[test]
    fn read_complete_runs_unique_id_clash_repair() {
        let file = project();

        let root = add_task(&file);
        let child = add_task(&file);
        root.borrow_mut().set_id(Some(1));
        child.borrow_mut().set_id(Some(2));

        // Java: readComplete() → fixUniqueIdClashes()
        // （Rust 侧各容器使用 try_borrow，借用冲突时退化为跳过，不会 panic）
        file.borrow_mut().read_complete();

        assert_eq!(file.borrow().get_tasks().len(), 2);
        assert!(root.borrow().get_unique_id().is_some());
        assert!(child.borrow().get_unique_id().is_some());
    }

    #[test]
    fn update_structure_is_a_noop_for_a_single_task() {
        let file = project();
        let only = add_task(&file);
        only.borrow_mut().set_outline_level(Some(0));

        // Java: TaskContainer.updateStructure() 在任务数 <= 1 时直接返回
        file.borrow_mut().update_structure();

        assert_eq!(file.borrow().get_tasks().len(), 1);
        // `addTask()` 已把该任务登记为顶层任务（Java 的 `m_childTasks`），早退不会清空它
        assert_eq!(file.borrow().get_child_tasks().len(), 1);
    }

    #[test]
    fn external_projects_round_trip() {
        let file = project();
        let subproject = project();

        {
            let mut file_ref = file.borrow_mut();
            file_ref.add_external_project(Some("X:\\projects\\child.mpp"), Some(Rc::clone(&subproject)));
        }

        let found = file.borrow_mut().read_external_project(Some("X:\\projects\\child.mpp"));
        assert!(found
            .map(|found| Rc::ptr_eq(&found, &subproject))
            .unwrap_or(false));

        // 未登记的文件名返回 None（没有接线读取回调）
        assert!(file
            .borrow_mut()
            .read_external_project(Some("X:\\projects\\other.mpp"))
            .is_none());
    }

    #[test]
    fn expand_subprojects_without_external_tasks_is_a_noop() {
        let file = project();
        add_task(&file);
        add_task(&file);

        file.borrow_mut().expand_subprojects(true);

        assert_eq!(file.borrow().get_tasks().len(), 2);
        assert_eq!(file.borrow().get_child_tasks().len(), 2);
    }

    #[test]
    fn relationships_can_be_created_and_removed() {
        let file = project();
        let first = add_task(&file);
        let second = add_task(&file);

        // Task::add_predecessor 会借用父文件，因此这里先释放 file 借用
        let relation = Task::add_predecessor(
            &second,
            RelationBuilder::new().predecessor_task(&first),
        )
        .expect("文件未搬空时可用");

        assert_eq!(file.borrow().get_relations().len(), 1);
        assert_eq!(second.borrow().get_predecessors().len(), 1);
        assert_eq!(first.borrow().get_successors().len(), 1);

        {
            let relation_ref = relation.borrow();
            assert!(file.borrow_mut().get_relations_mut().remove(&relation_ref));
        }
        assert!(file.borrow().get_relations().is_empty());
    }

    #[test]
    fn get_self_returns_the_shared_handle() {
        let file = project();
        assert!(Rc::ptr_eq(&file.borrow().get_self().expect("有句柄"), &file));
    }
}
