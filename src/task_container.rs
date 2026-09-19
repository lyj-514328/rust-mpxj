//! 对应 Java: `org/mpxj/TaskContainer.java`
//!
//! 管理项目任务集合的容器，同时承载任务的**层级结构**（父子任务、大纲级别、WBS）。
//!
//! # Java → Rust 方法对照
//!
//! | Java | Rust |
//! |---|---|
//! | `public TaskContainer(ProjectFile projectFile)` | [`TaskContainer::new`] + [`TaskContainer::set_context`]（Java 在构造时拿到父文件，Rust 侧拆成两步，见下） |
//! | `@Override public List<Task> getChildTasks()`（`ChildTaskContainer`） | [`TaskContainer::get_child_tasks`] / [`TaskContainer::get_child_tasks_mut`]（Java 返回可变列表，Rust 侧拆成读写两个方法） |
//! | `@Override public Task addTask()`（`ChildTaskContainer`） | [`TaskContainer::add_task`]（方法体与 `add()` 相同，Rust 侧因 `add` 已被元素版占用故保留两个方法名） |
//! | `public Task add()` | [`TaskContainer::add`] |
//! | `@Override public void removed(Task task)` | [`TaskContainer::removed`]（由 [`TaskContainer::remove_at`] / [`TaskContainer::remove_task`] 调用） |
//! | `public void synchronizeTaskIDToHierarchy()` | [`TaskContainer::synchronize_task_id_to_hierarchy`] |
//! | `private int synchroizeTaskIDToHierarchy(ChildTaskContainer, int, int)` | 私有方法 [`TaskContainer::synchronize_task_id_to_hierarchy_for`]（参数改为「子任务列表」，见下） |
//! | `void updateStructure()` | [`TaskContainer::update_structure`]（`pub(crate)`，与 Java 的包级可见性对应） |
//! | `public Set<FieldType> getPopulatedFields()` | **未移植**，见下 |
//! | `public List<CustomField> getCustomFields()` | [`TaskContainer::get_custom_fields`] |
//! | `public FieldType getFieldTypeByAlias(String alias)` | [`TaskContainer::get_field_type_by_alias`]（返回 `Option<FieldKey>`，见下） |
//! | `@Override protected int firstUniqueID()` | [`TaskContainer::first_unique_id`] |
//! | 继承自 `ProjectEntityWithIDContainer` 的 `getByID` / `mapID` / `unmapID` / `getNextID` / `renumberIDs` | [`TaskContainer::get_by_id`] / [`TaskContainer::map_id`] / [`TaskContainer::unmap_id`] / [`TaskContainer::get_next_id`] / [`TaskContainer::renumber_ids`] |
//! | 继承自 `ProjectEntityContainer` 的 `getByUniqueID` / `updateUniqueID` / `fixUniqueIdClashes` | [`TaskContainer::get_by_unique_id`] / [`TaskContainer::update_unique_id`] / [`TaskContainer::fix_unique_id_clashes`] |
//! | 继承自 `ListWithCallbacks` 的 `add(T)` / `add(int, T)` / `set(int, T)` / `remove(int)` / `clear()` / `get(int)` / `size()` / `isEmpty()` / `iterator()` | [`TaskContainer::add_task`]（元素版）/ [`TaskContainer::add_at`] / [`TaskContainer::set`] / [`TaskContainer::remove_at`] / [`TaskContainer::clear`] / [`TaskContainer::get`] / [`TaskContainer::len`] / [`TaskContainer::is_empty`] / [`TaskContainer::iter`] |
//! | `ListWithCallbacks.remove(Object)` | [`TaskContainer::remove_task`]（Java 用引用相等定位，Rust 用 `Rc::ptr_eq`，语义一致） |
//! | `Collections.sort(this)` | [`TaskContainer::sort`]（按 `Task::get_id()`，等价 Java 的 `Task.compareTo`） |
//! | `implements ChildTaskContainer` | 见下「`ChildTaskContainer` 的还原」 |
//!
//! # 跳过的 Java 代码及原因
//!
//! | Java | 原因 |
//! |---|---|
//! | `public Set<FieldType> getPopulatedFields()`（内部用 `org.mpxj.common.PopulatedFields`） | `PopulatedFields` 未移植（`src/common` 无对应文件），且其构造函数需要 `ProjectFile` / `UserDefinedFieldContainer` 的完整接口。待其落地后补回。 |
//! | `removed(Task)` 中「删除 `getResourceAssignments()` 里 taskUniqueID 匹配的资源分配」 | `ResourceAssignment` / `ResourceAssignmentContainer` 未落地（`src` 无对应文件）。Java 原逻辑保留在方法注释中，待其落地后补回。 |
//! | 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable` | 本类未使用这些机制 |
//!
//! # 继承体系的还原：组合 + 转发
//!
//! Java 的 `TaskContainer extends ProjectEntityWithIDContainer<Task>`（后者又继承
//! `ProjectEntityContainer<Task>` → `ListWithCallbacks<Task>`），并实现 `ChildTaskContainer`。
//! Rust 没有继承，故按「组合 + 转发」还原：
//!
//! - 内部用 `Vec<Rc<RefCell<Task>>>` 保存全部任务（对应 `ListWithCallbacks` 的列表），
//!   把基类的公开行为逐个实现（见上表），与
//!   [`crate::project_calendar_container::ProjectCalendarContainer`] 的做法一致。
//! - `ChildTaskContainer` 的两个方法（`getChildTasks` / `addTask`）在本类型上以**固有方法**
//!   [`TaskContainer::get_child_tasks`] / [`TaskContainer::add_task`] 提供：`src/lib.rs`
//!   没有声明 `child_task_container` 模块（本批只落地三个文件），故不新建该 trait；
//!   待 `Task` / `ProjectFile` 落地后如需共享该接口，可再抽出 trait 并把这里的两个方法
//!   改为 trait 实现（行为不变）。
//! - Java 的私有递归 `synchroizeTaskIDToHierarchy(ChildTaskContainer parentTask, ...)` 同时被
//!   「项目文件」（`m_projectFile`，其 `getChildTasks()` 就是本容器的 `m_childTasks`）与
//!   「任务」（`Task implements ChildTaskContainer`）调用。Rust 侧不引入 `ChildTaskContainer`
//!   trait，改为私有方法 [`TaskContainer::synchronize_task_id_to_hierarchy_for`]
//!   接收「子任务列表」：根层传入本容器的 `m_childTasks`，任务层传入 `task.get_child_tasks()`，
//!   与 Java 的两处调用等价。
//!
//! # 未组合基类（`ProjectEntityContainer` / `ProjectEntityWithIDContainer`）
//!
//! 本 crate 已移植这两个基类，但它们要求元素实现 `ProjectEntityWithID` 且满足 `Ord`
//! （用于 `Collections.sort`），并以 weak 反向引用与回调组合工作；而本容器的
//! `m_idMap`、`m_childTasks`、`removed` 递归删除等都与任务自身的 setter（`setID` 会回调
//! `getTasks().mapID(...)`）交织，组合后会引入额外的借用重入问题。因此改为自行实现：
//!
//! - `m_uniqueIDMap`（unique ID 索引）→ **按 unique ID 线性查找**（[`TaskContainer::get_by_unique_id`]）；
//!   线性查找不会因 unique ID 变化而失效，故不再需要索引维护，`update_unique_id` 只维护
//!   冲突列表与序号生成器。
//! - `m_idMap`（任务 ID 索引）→ 保留 `HashMap<i32, Rc<RefCell<Task>>>`（Java 由 MPX/MPP
//!   reader 经 `mapID` 填充、`Task.setID` 维护），提供 [`TaskContainer::map_id`] /
//!   [`TaskContainer::unmap_id`] / [`TaskContainer::get_by_id`]。
//! - `m_idSequence` → [`crate::common::object_sequence::ObjectSequence`]，供
//!   [`TaskContainer::get_next_id`] 使用。
//!
//! # 与 Java 的差异
//!
//! - **父文件的反向引用**：Java 的构造方法接收 `ProjectFile` 并保存为强引用
//!   （`m_projectFile`，同时由基类作为序号生成器提供者）。Rust 侧若强引用会形成
//!   「`ProjectFile` → 容器 → `ProjectFile`」的 `Rc` 循环，故改为
//!   `Weak<RefCell<ProjectFile>>`，先以空弱引用构造，再由 [`TaskContainer::set_context`]
//!   接线（Java 在构造方法里完成）。
//! - **`ProjectFile.getChildTasks()` 的直接使用**：Java 的 `add()` / `removed()` /
//!   `updateStructure()` 通过 `m_projectFile.getChildTasks()` 访问**同一个** `m_childTasks`
//!   列表（`ProjectFile.getChildTasks()` 即 `m_tasks.getChildTasks()`）。Rust 侧
//!   直接访问本容器的 `child_tasks` 字段，结果完全等价（也避免了在持有 `&mut self` 时
//!   再去借用父文件）。
//! - **集合返回值**：Java 的 `getChildTasks()` 返回可变列表视图；Rust 侧返回切片
//!   [`TaskContainer::get_child_tasks`] 与可变引用 [`TaskContainer::get_child_tasks_mut`]。
//! - **`getFieldTypeByAlias`**：Java 返回 `FieldType`；Rust 侧按已落地的
//!   [`crate::custom_field_container::CustomFieldContainer`] 返回 `Option<FieldKey>`
//!   （`FieldKey` 是本 crate 对 `FieldType` 接口的运行时擦除）。
//! - **空值语义**：`NumberHelper.getInt(null)` 在 Java 中为 0，Rust 侧用 `unwrap_or(0)`
//!   等价表达；Java 对 `task.getChildTasks()` 等返回的非空集合直接使用，Rust 侧在借用
//!   失败（`try_borrow` 出错）时退化为空集合或跳过，不 panic。
//! - **重入借用**：Java 中 `Task.setID` 会回调 `m_projectFile.getTasks().mapID(...)`，
//!   `Task` 的构造方法（`new_in_file`）与 `Task.generateWBS` / `generateOutlineNumber` /
//!   `addChildTask` 都会读取父文件的任务数、配置或序号生成器；而 `updateStructure` /
//!   `synchronizeTaskIDToHierarchy` / `renumberIDs` 又由 `ProjectFile` 发起。
//!   在 Rust 中若调用方正可变借用 `ProjectFile`，上述任务方法内部再借父文件会失败。
//!   处理方式：
//!   - [`TaskContainer::update_structure`] 需要的三个自动编号开关改由调用方
//!     [`crate::ProjectFile::update_structure`] 传入（`ProjectConfig` 在 `ProjectContext`
//!     的 `RefCell` 中，读取它与持有 `ProjectFile` 借用不冲突），WBS / 大纲编号所需的
//!     任务计数则由容器自身提供；`Task::add_child_task_with_auto_outline_level` 同理。
//!     因此整条 `update_structure` 路径**完全不回调父文件**，可在持有借用时原地调用。
//!   - [`TaskContainer::add_task`] 登记任务自身的弱引用，`Task::set_id` 据此还原
//!     `mapID` / `unmapID` 回调；父文件被借用时该回调退化为「不维护索引」，不 panic。
//!   - 本文件**自身**的父文件访问（[`TaskContainer::custom_field_container`] /
//!     [`TaskContainer::sequence`]）一律使用 `try_borrow` 退化处理，不 panic。
//!   - 仍需注意：`Task::new_in_file` 使用 `borrow()` / `borrow_mut()`，因此
//!     [`TaskContainer::add`] / [`crate::ProjectFile::add_task`] 要求调用方不持有
//!     `ProjectFile` 借用（免借用入口见 `crate::ProjectFile::add_task_shared`）。

//! - **`updateStructure` 中的排序**：Java 用 `Collections.sort(this)`（`Task.compareTo`
//!   按任务 ID 比较）；Rust 侧用 [`TaskContainer::sort`]（按 `Task::get_id()`），结果一致。
//! - **`removed` 中的递归删除**：Java 用 `while (true) { ... remove(childTaskList.get(0)); }`，
//!   其终止依赖「`removed` 会把子任务从父任务的子任务列表移除」（即 `child.m_parent`）。
//!   Rust 侧改为遍历子任务快照后逐个移除：移除集合与递归顺序一致，且不依赖子任务的反向指针。
//! - **`updateStructure` 中的父任务回填**：Java 的 `parent.addChildTask(task)` 会同时设置
//!   `child.m_parent = this`，后文的 `lastTask.getParentTask()` 依赖这一点。Rust 侧同样
//!   依赖 `Task::add_child_task` 维护 `child.get_parent_task()`（见下「接口假设」）。
//!
//! # 本文件对并行模块接口的假设（集成时如不一致需对齐）
//!
//! `Task`（`src/task.rs`）已落地，本文件按它的实际签名使用：
//!
//! ```text
//! Task::new() -> Task
//! Task::new_in_file(&Rc<RefCell<ProjectFile>>, Option<&Rc<RefCell<Task>>>) -> Task
//! Task::get_id(&self) -> Option<i32> / set_id(&mut self, Option<i32>)
//! Task::get_unique_id(&self) -> Option<i32> / set_unique_id(&mut self, Option<i32>)
//! Task::get_outline_level(&self) -> Option<i32> / set_outline_level(&mut self, Option<i32>)
//! Task::get_parent_task(&self) -> Option<Rc<RefCell<Task>>>
//! Task::get_child_tasks(&self) -> Vec<Rc<RefCell<Task>>>
//! Task::get_null(&self) -> bool
//! Task::get_wbs(&self) -> Option<String>
//! Task::generate_wbs(&mut self, Option<&Rc<RefCell<Task>>>)
//! Task::generate_outline_number(&mut self, Option<&Rc<RefCell<Task>>>)
//! Task::add_child_task(&Rc<RefCell<Task>>, &Rc<RefCell<Task>>)     // `this` 为首参的关联函数
//! Task::remove_child_task(&Rc<RefCell<Task>>, &Rc<RefCell<Task>>)  // 同上
//! Task::clear_child_tasks(&Rc<RefCell<Task>>)                      // 同上
//! ```
//!
//! `ProjectFile`（`src/project_file.rs`）仍未落地，本文件按下列签名使用：
//!
//! ```text
//! ProjectFile::new() -> ProjectFile
//! ProjectFile::get_project_config(&self) -> &ProjectConfig
//! ProjectFile::get_custom_fields(&self) -> Rc<RefCell<CustomFieldContainer>>
//! ProjectFile::get_unique_id_object_sequence(&mut self, TypeId) -> Rc<ObjectSequence>
//!         // 或 `ProjectFile: UniqueIdObjectSequenceProvider`（Java 一致），
//!         // 本文件的方法调用形态对两种写法都成立
//! ```

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::{Rc, Weak};

use crate::common::object_sequence::ObjectSequence;
use crate::custom_field::CustomField;
use crate::custom_field_container::CustomFieldContainer;
use crate::field_key::FieldKey;
use crate::field_type_class::FieldTypeClass;
use crate::project_file::ProjectFile;
use crate::task::Task;
// `ProjectFile` 可能以「实现本 trait」或「提供同签名固有方法」两种形式给出（见文件头
// 「接口假设」）；前者需要本导入，后者不会用到它，故显式允许未使用。
#[allow(unused_imports)]
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;

/// 项目任务集合。
///
/// 对应 Java:
/// `public class TaskContainer extends ProjectEntityWithIDContainer<Task> implements ChildTaskContainer`
pub struct TaskContainer {
    /// 容器内的全部任务。
    ///
    /// 对应 Java: `ProjectEntityWithIDContainer` / `ProjectEntityContainer` /
    /// `ListWithCallbacks` 继承而来的列表存储。
    items: Vec<Rc<RefCell<Task>>>,

    /// 项目的顶层任务列表（不含子任务）。
    ///
    /// 对应 Java 字段: `private final List<Task> m_childTasks = new ArrayList<>();`
    /// （也是 `ProjectFile.getChildTasks()` 返回的列表）
    child_tasks: Vec<Rc<RefCell<Task>>>,

    /// 任务 ID → 任务 索引。
    ///
    /// 对应 Java 字段: `ProjectEntityWithIDContainer.m_idMap`
    id_map: HashMap<i32, Rc<RefCell<Task>>>,

    /// 任务 ID 序号生成器。
    ///
    /// 对应 Java 字段: `ProjectEntityWithIDContainer.m_idSequence`
    id_sequence: ObjectSequence,

    /// unique ID 冲突列表。
    ///
    /// 对应 Java 字段: `ProjectEntityContainer.m_uniqueIDClashList`
    unique_id_clash_list: Vec<Rc<RefCell<Task>>>,

    /// 父项目文件（弱引用，避免 `Rc` 循环）。
    ///
    /// 对应 Java 字段: `private final ProjectFile m_projectFile;`
    /// （同时是基类的序号生成器提供者 `m_sequenceProvider`）
    context: Weak<RefCell<ProjectFile>>,
}

impl TaskContainer {
    /// 构造方法。
    ///
    /// 对应 Java: `public TaskContainer(ProjectFile projectFile)`
    ///
    /// Rust 侧先以空父文件弱引用构造，由 [`TaskContainer::set_context`] 完成接线。
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            child_tasks: Vec::new(),
            id_map: HashMap::new(),
            id_sequence: ObjectSequence::new(1),
            unique_id_clash_list: Vec::new(),
            context: Weak::new(),
        }
    }

    /// 接线父项目文件（Rust 特有：Java 在构造方法中直接接收 `ProjectFile`）。
    ///
    /// 对应 Java: 构造方法中的 `m_projectFile = projectFile;` 与基类的 `super(projectFile)`
    pub(crate) fn set_context(&mut self, project_file: &Rc<RefCell<ProjectFile>>) {
        self.context = Rc::downgrade(project_file);
    }

    /// 取父项目文件（未接线时返回 `None`）。
    ///
    /// 对应 Java: `private final ProjectFile m_projectFile;`
    pub fn get_context(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.context.upgrade()
    }

    /// 取项目顶层任务列表。
    ///
    /// 对应 Java: `@Override public List<Task> getChildTasks()`（`ChildTaskContainer`）
    pub fn get_child_tasks(&self) -> &[Rc<RefCell<Task>>] {
        &self.child_tasks
    }

    /// 取项目顶层任务列表的可变引用（Rust 特有：Java 的 `getChildTasks()` 返回可变列表）。
    pub fn get_child_tasks_mut(&mut self) -> &mut Vec<Rc<RefCell<Task>>> {
        &mut self.child_tasks
    }

    /// 向容器追加一个已有任务。
    ///
    /// 对应 Java: `ListWithCallbacks.add(T element)`（进而触发基类的 `added`）
    ///
    /// Rust 侧无重载，元素版与无参版 [`TaskContainer::add`] 以名称区分
    /// （与 [`crate::project_calendar_container::ProjectCalendarContainer::add_calendar`] 的做法一致）。
    /// 注意：本方法**不**把任务加入项目的顶层任务列表（Java 中由调用方另行调用
    /// `m_projectFile.getChildTasks().add(task)`），需要该行为时用
    /// [`TaskContainer::get_child_tasks_mut`]。
    pub fn add_task(&mut self, task: Rc<RefCell<Task>>) {
        // Rust 特有：登记任务自身的共享句柄，供 `Task::set_id` 还原 Java
        // `setID` 中 `mapID(val, this)` 的回调（见 `Task::set_self_reference`）。
        if let Ok(mut task_ref) = task.try_borrow_mut() {
            task_ref.set_self_reference(&task);
        }

        self.items.push(Rc::clone(&task));
        self.added(&task);
    }

    /// 新建一个任务并加入容器（同时登记为项目顶层任务）。
    ///
    /// 对应 Java: `public Task add()`（`ChildTaskContainer.addTask()` 的实现体）
    pub fn add(&mut self) -> Rc<RefCell<Task>> {
        // Java: Task task = new Task(m_projectFile, null);
        let task = Rc::new(RefCell::new(match self.context.upgrade() {
            Some(project_file) => Task::new_in_file(&project_file, None),
            // Java 中 m_projectFile 必然存在；Rust 侧未接线时退化为无父文件的任务（见文件头）
            None => Task::new(),
        }));

        self.add_task(Rc::clone(&task));

        // Java: m_projectFile.getChildTasks().add(task);
        // ProjectFile.getChildTasks() 即本容器的 m_childTasks（见文件头）
        self.child_tasks.push(Rc::clone(&task));

        task
    }

    /// 任务被移除后的清理：从 ID 索引、父任务的子任务列表、项目顶层任务列表移除，
    /// 并递归移除其子任务。
    ///
    /// 对应 Java: `@Override public void removed(Task task)`
    pub fn removed(&mut self, task: &Rc<RefCell<Task>>) {
        //
        // Java: super.removed(task); —— 基类把该元素从 unique ID 索引中移除。
        // Rust 侧按 unique ID 线性查找（见文件头），没有需要维护的索引。
        //

        // Java: m_idMap.remove(task.getID());
        if let Some(id) = read_task(task, |task| task.get_id()).flatten() {
            self.id_map.remove(&id);
        }

        // Java: Task parentTask = task.getParentTask();
        //       if (parentTask != null) { parentTask.removeChildTask(task); }
        //       else { m_projectFile.getChildTasks().remove(task); }
        let parent_task = read_task(task, |task| task.get_parent_task()).flatten();
        match parent_task {
            // Java: parentTask.removeChildTask(task);
            // （`Task::remove_child_task` 为以 `this` 为首参的关联函数，见 `crate::task` 文件头）
            Some(parent_task) => Task::remove_child_task(&parent_task, task),
            // Java: m_projectFile.getChildTasks().remove(task);
            None => {
                self.child_tasks.retain(|child| !Rc::ptr_eq(child, task));
            }
        }

        //
        // Java:
        // Iterator<ResourceAssignment> iter = m_projectFile.getResourceAssignments().iterator();
        // while (iter.hasNext()) { ResourceAssignment assignment = iter.next();
        //    if (assignment.getTaskUniqueID().equals(task.getUniqueID())) { iter.remove(); } }
        //
        // ResourceAssignment / ResourceAssignmentContainer 尚未落地（见文件头），
        // 待其落地后在此补回。

        //
        // Java: 递归移除子任务
        // while (true) { List<Task> childTaskList = task.getChildTasks();
        //    if (childTaskList.isEmpty()) break; remove(childTaskList.get(0)); }
        //
        // Java 的循环依赖“removed 会把子任务从**父任务**的子任务列表移除”（通过 child.m_parent），
        // Rust 侧改为遍历子任务快照：移除的集合与递归顺序一致，且不依赖子任务的反向指针。
        let child_tasks: Vec<Rc<RefCell<Task>>> =
            read_task(task, |task| task.get_child_tasks().iter().cloned().collect())
                .unwrap_or_default();

        for child_task in child_tasks {
            self.remove_task(&child_task);
        }
    }

    /// 按 ID 顺序重排任务的 ID（Microsoft Project 依据 ID 决定显示顺序），
    /// 并同步任务的大纲级别。
    ///
    /// 对应 Java: `public void synchronizeTaskIDToHierarchy()`
    pub fn synchronize_task_id_to_hierarchy(&mut self) {
        // Java: int currentID = (getByID(Integer.valueOf(0)) == null ? 1 : 0);
        let current_id = self.first_unique_id();

        // Java: synchroizeTaskIDToHierarchy(m_projectFile, currentID, currentID);
        // ProjectFile.getChildTasks() 即本容器的 m_childTasks（见文件头）
        let child_tasks = self.child_tasks.clone();
        self.synchronize_task_id_to_hierarchy_for(child_tasks, current_id, current_id);
    }

    /// 递归重排子任务的 ID 并更新大纲级别。
    ///
    /// 对应 Java: `private int synchroizeTaskIDToHierarchy(ChildTaskContainer parentTask, int currentID, int currentLevel)`
    ///
    /// Java 用 `ChildTaskContainer` 统一「项目文件」与「任务」两种父容器；Rust 侧改为直接
    /// 接收子任务列表（见文件头）。
    fn synchronize_task_id_to_hierarchy_for(
        &mut self,
        tasks: Vec<Rc<RefCell<Task>>>,
        current_id: i32,
        current_level: i32,
    ) -> i32 {
        let mut current_id = current_id;

        for task in tasks {
            // Java: task.setID(Integer.valueOf(currentID)); task.setOutlineLevel(Integer.valueOf(currentLevel));
            if let Ok(mut task_ref) = task.try_borrow_mut() {
                task_ref.set_id(Some(current_id));
                task_ref.set_outline_level(Some(current_level));
            }

            // Java: currentID = synchroizeTaskIDToHierarchy(task, currentID + 1, currentLevel + 1);
            let children: Vec<Rc<RefCell<Task>>> =
                read_task(&task, |task| task.get_child_tasks().iter().cloned().collect())
                    .unwrap_or_default();
            current_id =
                self.synchronize_task_id_to_hierarchy_for(children, current_id + 1, current_level + 1);
        }

        current_id
    }

    /// 依据任务的 `outlineLevel` 字段重建项目的层级结构（父 / 子任务、WBS、大纲编号）。
    ///
    /// 对应 Java: `void updateStructure()`
    ///
    /// Java 在本方法内部回调 `m_projectFile.getProjectConfig()` 读取三个自动编号开关；
    /// Rust 侧改由调用方 [`crate::ProjectFile::update_structure`] 显式传入 ——
    /// `ProjectConfig` 存放在 `ProjectContext` 的 `RefCell` 中，调用方读取它不会与
    /// 「持有 `ProjectFile` 借用」冲突，而容器自身在被可变借用时无法安全回调父文件。
    ///
    /// 因此本方法**完全不回调父文件**：可以在 `ProjectFile` 已被借用的情况下直接对
    /// 容器取可变借用执行，无需把容器临时移出 `ProjectFile`。
    pub(crate) fn update_structure(
        &mut self,
        auto_wbs: bool,
        auto_outline_number: bool,
        auto_outline_level: bool,
    ) {
        if self.items.len() <= 1 {
            return;
        }

        // Java: Collections.sort(this);（Task.compareTo 按 ID 比较）
        self.sort();

        // Java: m_projectFile.getChildTasks().clear();
        // ProjectFile.getChildTasks() 即本容器的 m_childTasks（见文件头）
        self.child_tasks.clear();

        let mut last_task: Option<Rc<RefCell<Task>>> = None;
        let mut last_level: i32 = -1;

        // 先克隆一份列表：循环体内会修改 child_tasks（Java 迭代的是同一列表，但只读）
        for task in self.items.clone() {
            // Java: task.clearChildTasks();
            // （`Task::clear_child_tasks` 为以 `this` 为首参的关联函数）
            Task::clear_child_tasks(&task);

            let mut parent: Option<Rc<RefCell<Task>>> = None;

            // Java: if (!task.getNull()) { ... }
            let is_null = read_task(&task, |task| task.get_null()).unwrap_or(false);
            if !is_null {
                let mut level = read_task(&task, |task| task.get_outline_level())
                    .flatten()
                    .unwrap_or(0);

                if let Some(last_task_ref) = &last_task {
                    if level == last_level {
                        // Java: parent = lastTask.getParentTask(); level = lastLevel;
                        parent =
                            read_task(last_task_ref, |last| last.get_parent_task()).flatten();
                        level = last_level;
                    } else if level > last_level {
                        // Java: parent = lastTask;
                        parent = Some(Rc::clone(last_task_ref));
                    } else {
                        // Java: while (level <= lastLevel) { parent = lastTask.getParentTask();
                        //          if (parent == null) break;
                        //          lastLevel = NumberHelper.getInt(parent.getOutlineLevel());
                        //          lastTask = parent; }
                        let mut cursor = Rc::clone(last_task_ref);
                        while level <= last_level {
                            let next = read_task(&cursor, |cursor| cursor.get_parent_task()).flatten();
                            let Some(next) = next else {
                                parent = None;
                                break;
                            };

                            last_level = read_task(&next, |next| next.get_outline_level())
                                .flatten()
                                .unwrap_or(0);
                            cursor = Rc::clone(&next);
                            parent = Some(Rc::clone(&next));
                        }
                    }
                }

                last_task = Some(Rc::clone(&task));
                last_level = level;

                // Java: if (autoWbs || task.getWBS() == null) { task.generateWBS(parent); }
                //
                // Java 的 `generateWBS(null)` 读取 `m_projectFile.getChildTasks().size()`；
                // 本方法正是该计数的权威来源（`TaskContainer.child_tasks` 即
                // `ProjectFile.getChildTasks()`），且此刻调用方可能正持有 `ProjectFile`
                // 的借用而无法安全回调，故显式传入（与 Java 取值完全一致：当前任务尚未
                // 登记进 child_tasks，因此 len() 即「前面已登记的顶层任务数」）。
                let has_wbs = read_task(&task, |task| task.get_wbs().is_some()).unwrap_or(false);
                if auto_wbs || !has_wbs {
                    if let Ok(mut task_ref) = task.try_borrow_mut() {
                        task_ref.generate_wbs_with_count(parent.as_ref(), self.child_tasks.len());
                    }
                }

                // Java: if (autoOutlineNumber) { task.generateOutlineNumber(parent); }
                if auto_outline_number {
                    if let Ok(mut task_ref) = task.try_borrow_mut() {
                        task_ref.generate_outline_number_with_count(
                            parent.as_ref(),
                            self.child_tasks.len(),
                        );
                    }
                }
            }

            // Java: if (parent == null) { m_projectFile.getChildTasks().add(task); }
            //       else { parent.addChildTask(task); }
            match parent {
                // Java: parent.addChildTask(task);
                // （`Task::add_child_task` 为以 `this` 为首参的关联函数；
                //   这里传入已读取好的 `autoOutlineLevel`，避免回调父文件）
                Some(parent) => {
                    Task::add_child_task_with_auto_outline_level(&parent, &task, auto_outline_level)
                }
                // Java: m_projectFile.getChildTasks().add(task);
                None => self.child_tasks.push(Rc::clone(&task)),
            }
        }
    }

    /// 取任务自定义字段列表。
    ///
    /// 对应 Java: `public List<CustomField> getCustomFields()`
    ///
    /// Java: `m_projectFile.getCustomFields().getCustomFieldsByFieldTypeClass(FieldTypeClass.TASK)`；
    /// 父文件未接线时返回空列表（Java 必然可用）。
    pub fn get_custom_fields(&self) -> Vec<Rc<RefCell<CustomField>>> {
        let Some(custom_fields) = self.custom_field_container() else {
            return Vec::new();
        };

        let result = custom_fields
            .borrow()
            .get_custom_fields_by_field_type_class(FieldTypeClass::Task);
        result
    }

    /// 按别名取字段类型。
    ///
    /// 对应 Java: `public FieldType getFieldTypeByAlias(String alias)`
    ///
    /// Java 返回 `FieldType`，Rust 侧返回 [`FieldKey`]（见文件头）；父文件未接线时返回 `None`。
    pub fn get_field_type_by_alias(&self, alias: &str) -> Option<FieldKey> {
        let custom_fields = self.custom_field_container()?;
        let field_type = custom_fields
            .borrow()
            .get_field_type_by_alias(FieldTypeClass::Task, alias);
        field_type
    }

    /// 重排 unique ID 时使用的第一个值：存在 ID 为 0 的任务时从 0 开始，否则从 1 开始。
    ///
    /// 对应 Java: `@Override protected int firstUniqueID()`
    pub fn first_unique_id(&self) -> i32 {
        // Java: Task firstEntity = getByID(Integer.valueOf(0));
        //       return firstEntity == null ? 1 : 0;
        if self.get_by_id(Some(0)).is_none() {
            1
        } else {
            0
        }
    }

    /// 按 ID 查找任务，找不到时返回 `None`。
    ///
    /// 对应 Java: `ProjectEntityWithIDContainer.getByID(Integer id)`
    pub fn get_by_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Task>>> {
        let id = id?;
        self.id_map.get(&id).cloned()
    }

    /// 登记一条 ID 映射。
    ///
    /// 对应 Java: `ProjectEntityWithIDContainer.mapID(Integer id, T entity)`
    pub fn map_id(&mut self, id: i32, task: Rc<RefCell<Task>>) {
        self.id_map.insert(id, task);
    }

    /// 删除一条 ID 映射。
    ///
    /// 对应 Java: `ProjectEntityWithIDContainer.unmapID(Integer id)`
    pub fn unmap_id(&mut self, id: Option<i32>) {
        if let Some(id) = id {
            self.id_map.remove(&id);
        }
    }

    /// 取下一个可用的任务 ID。
    ///
    /// 对应 Java: `ProjectEntityWithIDContainer.getNextID()`
    pub fn get_next_id(&self) -> i32 {
        self.id_sequence.get_next()
    }

    /// 让所有任务的 ID 连续排列（先按 ID 排序，再依次赋值）。
    ///
    /// 对应 Java: `ProjectEntityWithIDContainer.renumberIDs()`
    ///
    /// Java 中 `Collections.sort` 会经 `set(int, T)` 触发 `replaced` 回调；`TaskContainer`
    /// 未覆写 `replaced`，故无额外效果，Rust 侧直接排序。
    pub fn renumber_ids(&mut self) {
        if self.items.is_empty() {
            return;
        }

        // Java: Collections.sort(this);
        self.sort();

        // Java: int id = NumberHelper.getInt(firstEntity.getID()); if (id != 0) { id = 1; }
        let first_id = self
            .items
            .first()
            .and_then(|task| read_task(task, |task| task.get_id()))
            .flatten()
            .unwrap_or(0);
        let mut id = if first_id != 0 { 1 } else { 0 };

        for task in self.items.clone() {
            if let Ok(mut task_ref) = task.try_borrow_mut() {
                task_ref.set_id(Some(id));
            }
            id += 1;
        }
    }

    /// 按 unique ID 查找任务，找不到时返回 `None`。
    ///
    /// 对应 Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    ///
    /// Rust 侧按 unique ID 线性查找（无索引，见文件头）。
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Task>>> {
        let id = id?;
        self.items
            .iter()
            .find(|task| read_task(task, |task| task.get_unique_id()).flatten() == Some(id))
            .map(Rc::clone)
    }

    /// 任务的 unique ID 变化时更新容器状态。
    ///
    /// 对应 Java: `ProjectEntityContainer.updateUniqueID(T element, Integer oldUniqueID, Integer newUniqueID)`
    ///
    /// Rust 侧按 unique ID 线性查找，因此只需维护冲突列表与序号生成器。
    pub fn update_unique_id(
        &mut self,
        element: &Rc<RefCell<Task>>,
        old_unique_id: Option<i32>,
        new_unique_id: Option<i32>,
    ) {
        // Java: if (oldUniqueID != null) { m_uniqueIDMap.remove(oldUniqueID); }
        // Rust 侧为线性查找，没有需要移除的索引项。
        let _ = old_unique_id;

        // Java: T currentElement = m_uniqueIDMap.get(newUniqueID);
        //       if (currentElement == element) return;
        //       if (currentElement instanceof ProjectEntityWithMutableUniqueID)
        //       { m_uniqueIDClashList.add(element); }
        // Task 的 unique ID 可变，故与 Java 一致地记录冲突。
        if self
            .find_other_with_unique_id(new_unique_id, element)
            .is_some()
        {
            self.unique_id_clash_list.push(Rc::clone(element));
        }

        // Java: m_uniqueIDMap.put(newUniqueID, element);
        //       m_sequenceProvider.getUniqueIdObjectSequence(element.getClass()).sync(newUniqueID);
        if let Some(sequence) = self.sequence() {
            sequence.sync(new_unique_id);
        }
    }

    /// 为出现 unique ID 冲突的任务重新分配 unique ID。
    ///
    /// 对应 Java: `ProjectEntityContainer.fixUniqueIdClashes()`
    pub fn fix_unique_id_clashes(&mut self) {
        if self.unique_id_clash_list.is_empty() {
            return;
        }

        // Java: ObjectSequence sequence = m_sequenceProvider.getUniqueIdObjectSequence(
        //          m_uniqueIDClashList.get(0).getClass());
        // Rust 侧父文件未接线时无法取得序号生成器（Java 此时必然可用），
        // 保留冲突列表待接线后再处理。
        let Some(sequence) = self.sequence() else {
            return;
        };

        // Java: m_uniqueIDClashList.forEach(i -> i.setUniqueID(sequence.getNext()));
        for task in std::mem::take(&mut self.unique_id_clash_list) {
            if let Ok(mut task_ref) = task.try_borrow_mut() {
                task_ref.set_unique_id(Some(sequence.get_next()));
            }
        }

        // Java: m_uniqueIDMap.clear(); forEach(i -> m_uniqueIDMap.put(...));
        // Rust 侧按 unique ID 线性查找，没有索引需要重建。
    }

    /// 在指定下标插入任务（越界返回 `false`）。
    ///
    /// 对应 Java: `ListWithCallbacks.add(int index, T element)`
    pub fn add_at(&mut self, index: usize, task: Rc<RefCell<Task>>) -> bool {
        if index > self.items.len() {
            return false;
        }

        self.items.insert(index, Rc::clone(&task));
        self.added(&task);
        true
    }

    /// 替换指定下标的任务，返回被替换掉的旧任务（越界返回 `None`）。
    ///
    /// 对应 Java: `ListWithCallbacks.set(int index, T element)`
    ///
    /// 与 Java 一致：`TaskContainer` 未覆写 `replaced`，故替换**不**更新 ID / unique ID 索引。
    pub fn set(&mut self, index: usize, task: Rc<RefCell<Task>>) -> Option<Rc<RefCell<Task>>> {
        if index >= self.items.len() {
            return None;
        }

        Some(std::mem::replace(&mut self.items[index], task))
    }

    /// 移除指定下标的任务并返回它（越界返回 `None`）。
    ///
    /// 对应 Java: `ListWithCallbacks.remove(int index)`
    pub fn remove_at(&mut self, index: usize) -> Option<Rc<RefCell<Task>>> {
        if index >= self.items.len() {
            return None;
        }

        let removed = self.items.remove(index);
        self.removed(&removed);
        Some(removed)
    }

    /// 按任务身份移除一个任务（等效于 Java 的 `ListWithCallbacks.remove(Object)`）。
    ///
    /// 对应 Java: `ListWithCallbacks.remove(Object o)`
    pub fn remove_task(&mut self, task: &Rc<RefCell<Task>>) -> bool {
        let index = self
            .items
            .iter()
            .position(|candidate| Rc::ptr_eq(candidate, task));

        match index {
            Some(index) => {
                let removed = self.items.remove(index);
                self.removed(&removed);
                true
            }
            None => false,
        }
    }

    /// 清空任务列表（不触发 `removed`，与 Java 一致）。
    ///
    /// 对应 Java: `ListWithCallbacks.clear()`
    ///
    /// 与 Java 相同：`clear()` 不清空 `m_idMap`（因此不清空 Rust 侧的 `id_map`）。
    /// 顶层任务列表也按 Java 的行为保持不变（`updateStructure` 会显式清空它）。
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// 按下标取任务；越界返回 `None`。
    ///
    /// 对应 Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<Rc<RefCell<Task>>> {
        self.items.get(index).map(Rc::clone)
    }

    /// 任务个数。
    ///
    /// 对应 Java: `ListWithCallbacks.size()`
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// 容器是否为空。
    ///
    /// 对应 Java: `ListWithCallbacks.isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 迭代容器内的任务。
    ///
    /// 对应 Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<Task>>> {
        self.items.iter()
    }

    /// 以切片形式访问容器内的任务。
    ///
    /// 对应 Java: 基类（`ArrayList`）的列表视图。
    pub fn as_slice(&self) -> &[Rc<RefCell<Task>>] {
        self.items.as_slice()
    }

    /// 按任务 ID 就地排序（等价于 Java 的 `Collections.sort(this)`）。
    ///
    /// 对应 Java: `ProjectEntityWithIDContainer` 中的 `Collections.sort(this)`
    /// （经 `Task.compareTo` 按 ID 比较）
    pub fn sort(&mut self) {
        self.items
            .sort_by_key(|task| read_task(task, |task| task.get_id()).flatten().unwrap_or(0));
    }

    /// 任务加入后的索引维护。
    ///
    /// 对应 Java: `ProjectEntityContainer.added(T element)`
    fn added(&mut self, element: &Rc<RefCell<Task>>) {
        // Java: if (element.getUniqueID() == null) return;
        let Some(unique_id) = read_task(element, |task| task.get_unique_id()).flatten() else {
            return;
        };

        // Java: T currentElement = m_uniqueIDMap.get(uniqueID); if (currentElement == element) return;
        //       if (currentElement instanceof ProjectEntityWithMutableUniqueID)
        //       { m_uniqueIDClashList.add(element); }
        // Task 的 unique ID 可变，故与 Java 一致地记录冲突。
        if self.find_other_with_unique_id(Some(unique_id), element).is_some() {
            self.unique_id_clash_list.push(Rc::clone(element));
        }

        // Java: m_uniqueIDMap.put(element.getUniqueID(), element);
        // Rust 侧按 unique ID 线性查找，没有索引需要写入。
    }

    /// 取父项目文件提供的 `Task` 序号生成器（父文件未接线或已被借用时返回 `None`）。
    ///
    /// 对应 Java: `m_sequenceProvider.getUniqueIdObjectSequence(Task.class)`
    fn sequence(&self) -> Option<Rc<ObjectSequence>> {
        let project_file = self.context.upgrade()?;
        let mut project_file = project_file.try_borrow_mut().ok()?;

        // 兼容两种形态：ProjectFile 作为 `UniqueIdObjectSequenceProvider` 的实现
        //（Java 一致），或提供同签名的固有方法（`crate::task` 的假设）。
        Some(project_file.get_unique_id_object_sequence(TypeId::of::<Task>()))
    }

    /// 取父项目文件的自定义字段容器。
    ///
    /// 对应 Java: `m_projectFile.getCustomFields()`
    fn custom_field_container(&self) -> Option<Rc<RefCell<CustomFieldContainer>>> {
        let project_file = self.context.upgrade()?;
        let project_file = project_file.try_borrow().ok()?;
        Some(project_file.get_custom_fields())
    }

    /// 查找另一个占用指定 unique ID 的任务（Java 用 unique ID 索引判断「该 ID 已被占用」）。
    ///
    /// Java 的索引在同一 unique ID 被多次登记时保存**最后**加入的元素；Rust 侧按列表顺序取
    /// 第一个同 ID 的**其他**元素，等价表达「已被占用」的判定（仅可能影响冲突列表中元素的
    /// 顺序，不影响最终分配结果）。
    fn find_other_with_unique_id(
        &self,
        unique_id: Option<i32>,
        exclude: &Rc<RefCell<Task>>,
    ) -> Option<Rc<RefCell<Task>>> {
        let unique_id = unique_id?;
        self.items
            .iter()
            .find(|task| {
                !Rc::ptr_eq(task, exclude)
                    && read_task(task, |task| task.get_unique_id()).flatten() == Some(unique_id)
            })
            .map(Rc::clone)
    }
}

impl Default for TaskContainer {
    /// 对应 Java: 显式声明的 `TaskContainer(ProjectFile)`，
    /// Rust 侧等价于 [`TaskContainer::new`]（父文件由 `set_context` 接线）。
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for TaskContainer {
    /// Java 侧未覆写 `toString()`；Rust 侧输出容器规模、顶层任务数与父文件接线情况
    /// （避免依赖并行模块 `Task` 的 `Debug` 实现）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskContainer")
            .field("size", &self.items.len())
            .field("child_task_count", &self.child_tasks.len())
            .field("context_is_set", &(self.context.strong_count() > 0))
            .finish()
    }
}

/// 以 `try_borrow` 读取任务字段，已被可变借用时返回 `None`（不 panic）。
fn read_task<T>(task: &Rc<RefCell<Task>>, read: impl FnOnce(&Task) -> T) -> Option<T> {
    task.try_borrow().ok().map(|task| read(&task))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 接线到新建父文件的任务容器。
    fn container() -> (TaskContainer, Rc<RefCell<ProjectFile>>) {
        let project_file = ProjectFile::new();
        let mut container = TaskContainer::new();
        container.set_context(&project_file);
        (container, project_file)
    }

    /// 在指定父文件下新建一个任务（不入容器）。
    fn task(project_file: &Rc<RefCell<ProjectFile>>) -> Rc<RefCell<Task>> {
        Rc::new(RefCell::new(Task::new_in_file(project_file, None)))
    }

    /// 在指定父文件下新建一个带 ID 与大纲级别的任务，并加入容器。
    fn add_task(
        container: &mut TaskContainer,
        project_file: &Rc<RefCell<ProjectFile>>,
        id: i32,
        outline_level: i32,
    ) -> Rc<RefCell<Task>> {
        let task = task(project_file);
        {
            let mut task_ref = task.borrow_mut();
            task_ref.set_id(Some(id));
            task_ref.set_outline_level(Some(outline_level));
        }
        container.add_task(Rc::clone(&task));
        task
    }

    #[test]
    fn add_creates_task_and_registers_project_child_task() {
        let (mut container, _) = container();
        assert!(container.is_empty());

        let task = container.add();

        assert_eq!(container.len(), 1);
        assert!(!container.is_empty());
        assert_eq!(container.get_child_tasks().len(), 1);
        assert!(Rc::ptr_eq(
            &container.get(0).expect("下标 0 存在"),
            &task
        ));
        assert!(Rc::ptr_eq(
            &container.get_child_tasks()[0],
            &container.get(0).expect("下标 0 存在")
        ));
        assert!(container.get(1).is_none());
    }

    #[test]
    fn add_without_context_still_creates_task() {
        // Java 中 m_projectFile 必然存在；Rust 侧未接线时退化为无父文件的任务（见文件头）
        let mut container = TaskContainer::new();
        let task = container.add();

        assert_eq!(container.len(), 1);
        assert_eq!(container.get_child_tasks().len(), 1);
        assert_eq!(read_task(&task, |task| task.get_id()).flatten(), None);
    }

    #[test]
    fn list_operations_round_trip() {
        let (mut container, project_file) = container();
        let first = task(&project_file);
        let second = task(&project_file);
        let third = task(&project_file);

        container.add_task(Rc::clone(&first));
        container.add_task(Rc::clone(&second));
        assert_eq!(container.len(), 2);
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);

        // add_at 插入
        assert!(container.add_at(1, Rc::clone(&third)));
        assert!(Rc::ptr_eq(&container.get(1).expect("下标 1 存在"), &third));
        assert!(!container.add_at(9, Rc::clone(&first)));

        // set 替换（与 Java 一致：不更新索引）
        let replacement = task(&project_file);
        let old = container.set(0, Rc::clone(&replacement)).expect("下标 0 存在");
        assert!(Rc::ptr_eq(&old, &first));
        assert!(container.set(9, Rc::clone(&first)).is_none());

        // remove_at 移除
        let removed = container.remove_at(0).expect("下标 0 存在");
        assert!(Rc::ptr_eq(&removed, &replacement));
        assert!(container.remove_at(9).is_none());

        // remove_task 按身份移除
        assert!(container.remove_task(&third));
        assert!(!container.remove_task(&third));

        // clear 清空列表（与 Java 一致：不清空 ID 索引）
        container.clear();
        assert!(container.is_empty());
    }

    #[test]
    fn id_index_and_sequence() {
        let (mut container, project_file) = container();
        let task = task(&project_file);

        assert!(container.get_by_id(Some(7)).is_none());
        assert!(container.get_by_id(None).is_none());

        container.map_id(7, Rc::clone(&task));
        assert!(Rc::ptr_eq(
            &container.get_by_id(Some(7)).expect("已映射"),
            &task
        ));

        container.unmap_id(Some(7));
        assert!(container.get_by_id(Some(7)).is_none());
        container.unmap_id(None);

        // Java: ProjectEntityWithIDContainer.m_idSequence 从 1 开始
        assert_eq!(container.get_next_id(), 1);
        assert_eq!(container.get_next_id(), 2);
    }

    #[test]
    fn first_unique_id_depends_on_task_zero() {
        let (mut container, project_file) = container();
        assert_eq!(container.first_unique_id(), 1);

        let zero = task(&project_file);
        zero.borrow_mut().set_id(Some(0));
        container.map_id(0, Rc::clone(&zero));
        assert_eq!(container.first_unique_id(), 0);
    }

    #[test]
    fn unique_id_lookup_and_clash_resolution() {
        let (mut container, project_file) = container();

        let first = task(&project_file);
        let second = task(&project_file);
        first.borrow_mut().set_unique_id(Some(3));
        second.borrow_mut().set_unique_id(Some(4));

        container.add_task(Rc::clone(&first));
        container.add_task(Rc::clone(&second));

        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(3)).expect("已登记"),
            &first
        ));
        assert!(container.get_by_unique_id(Some(9)).is_none());
        assert!(container.get_by_unique_id(None).is_none());

        // 把 second 的 unique ID 改成与 first 相同 → 进入冲突列表
        container.update_unique_id(&second, Some(4), Some(3));
        assert!(container
            .get_by_unique_id(Some(3))
            .map(|task| Rc::ptr_eq(&task, &first))
            .unwrap_or(false));

        container.fix_unique_id_clashes();

        let renumbered =
            read_task(&second, |task| task.get_unique_id()).flatten().expect("仍有 unique ID");
        assert_ne!(renumbered, 3);
        assert_eq!(
            read_task(&first, |task| task.get_unique_id()).flatten(),
            Some(3)
        );

        // 序号生成器来自父文件（ProjectFile 实现 UniqueIdObjectSequenceProvider）
        let next = UniqueIdObjectSequenceProvider::get_unique_id_object_sequence(
            &mut *project_file.borrow_mut(),
            TypeId::of::<Task>(),
        )
        .get_next();
        assert!(next > renumbered);
    }

    #[test]
    fn fix_unique_id_clashes_without_context_keeps_clash_list() {
        // 父文件未接线：Java 会因序号生成器不可用而抛异常，Rust 侧退化为不做任何事
        let mut container = TaskContainer::new();
        let project_file = ProjectFile::new();

        let first = task(&project_file);
        let second = task(&project_file);
        first.borrow_mut().set_unique_id(Some(1));
        second.borrow_mut().set_unique_id(Some(1));
        container.add_task(Rc::clone(&first));
        container.add_task(Rc::clone(&second));

        container.update_unique_id(&second, Some(1), Some(1));
        container.fix_unique_id_clashes();

        assert_eq!(
            read_task(&second, |task| task.get_unique_id()).flatten(),
            Some(1)
        );
    }

    #[test]
    fn removed_drops_task_and_recurses_into_children() {
        let (mut container, project_file) = container();

        let parent = add_task(&mut container, &project_file, 1, 0);
        let child = task(&project_file);
        let grandchild = task(&project_file);
        Task::add_child_task(&child, &grandchild);
        container.add_task(Rc::clone(&child));
        container.add_task(Rc::clone(&grandchild));
        container.get_child_tasks_mut().push(Rc::clone(&parent));

        // 将 child 挂到 parent 之下（Java: Task.addChildTask）
        Task::add_child_task(&parent, &child);

        assert_eq!(container.len(), 3);

        // Java: removed 会递归移除子任务，并从父 / 项目顶层列表移除
        container.remove_task(&parent);

        assert!(container.is_empty());
        assert!(container.get_child_tasks().is_empty());
        assert!(parent.borrow().get_child_tasks().is_empty());
    }

    #[test]
    fn update_structure_builds_hierarchy_from_outline_levels() {
        let (mut container, project_file) = container();

        // 层级：0 / 1 / 2 / 0（最后一个任务回到顶层）
        let root = add_task(&mut container, &project_file, 1, 0);
        let child = add_task(&mut container, &project_file, 2, 1);
        let grandchild = add_task(&mut container, &project_file, 3, 2);
        let second_root = add_task(&mut container, &project_file, 4, 0);

        container.update_structure(true, true, true);

        // 顶层任务：root 与 second_root
        let root_tasks = container.get_child_tasks();
        assert_eq!(root_tasks.len(), 2);
        assert!(Rc::ptr_eq(&root_tasks[0], &root));
        assert!(Rc::ptr_eq(&root_tasks[1], &second_root));

        // root → child → grandchild
        let root_children = root.borrow().get_child_tasks();
        assert_eq!(root_children.len(), 1);
        assert!(Rc::ptr_eq(&root_children[0], &child));

        let child_children = child.borrow().get_child_tasks();
        assert_eq!(child_children.len(), 1);
        assert!(Rc::ptr_eq(&child_children[0], &grandchild));

        // Java: autoWbs / autoOutlineNumber 默认开启 → 生成 WBS 与大纲编号
        assert!(read_task(&root, |task| task.get_wbs().is_some()).unwrap_or(false));
        assert!(read_task(&child, |task| task.get_wbs().is_some()).unwrap_or(false));
        assert!(read_task(&grandchild, |task| task.get_wbs().is_some()).unwrap_or(false));
    }

    #[test]
    fn update_structure_is_noop_for_single_task() {
        let (mut container, project_file) = container();
        let only = add_task(&mut container, &project_file, 1, 0);

        let wbs_before = read_task(&only, |task| task.get_wbs()).flatten();

        container.update_structure(true, true, true);

        assert_eq!(container.len(), 1);
        assert_eq!(container.get_child_tasks().len(), 0);
        // Java: autoWbs 默认开启，任务创建时就已生成 WBS；updateStructure 对单任务不再改变它
        assert_eq!(
            read_task(&only, |task| task.get_wbs()).flatten(),
            wbs_before
        );
    }

    #[test]
    fn update_structure_generates_sequential_wbs_for_top_level_tasks() {
        let (mut container, project_file) = container();
        let mut tasks = Vec::new();

        // 模拟「文件里没有 WBS / 大纲编号列」：清掉构造时自动生成的值
        for id in 1..=3 {
            let task = add_task(&mut container, &project_file, id, 0);
            {
                let mut task_ref = task.borrow_mut();
                task_ref.set_wbs(None);
                task_ref.set_outline_number(None);
            }
            tasks.push(task);
        }

        container.update_structure(true, true, true);

        let wbs: Vec<Option<String>> = tasks
            .iter()
            .map(|task| read_task(task, |task| task.get_wbs()).flatten())
            .collect();
        let outline: Vec<Option<String>> = tasks
            .iter()
            .map(|task| read_task(task, |task| task.get_outline_number()).flatten())
            .collect();

        // Java: 顶层任务按调用时刻的 getChildTasks().size() + 1 依次得到 1、2、3
        assert_eq!(
            wbs,
            vec![
                Some("1".to_owned()),
                Some("2".to_owned()),
                Some("3".to_owned())
            ]
        );
        assert_eq!(outline, wbs);
    }

    #[test]
    fn set_id_registers_the_task_in_the_parent_file_index() {
        let (mut container, project_file) = container();
        let task = add_task(&mut container, &project_file, 7, 0);

        // ID 变更发生在任务入容器之后 —— 此时 `Task::set_self_reference` 已接线，
        // `set_id` 应像 Java 的 `setID` 一样回调容器的 map_id / unmap_id
        task.borrow_mut().set_id(Some(9));

        let file = project_file.borrow();
        assert!(
            file.get_task_by_id(Some(9)).is_some(),
            "新 ID 应登记进父文件的任务索引"
        );
        assert!(
            file.get_task_by_id(Some(7)).is_none(),
            "旧 ID 应从父文件的任务索引中解除"
        );
    }

    #[test]
    fn update_structure_is_safe_while_the_file_is_borrowed() {
        // 回归：修复前 `TaskContainer::update_structure` 会经
        // `Task::generate_wbs` / `Task::add_child_task` 重入借用父文件，若调用方正持有
        // `ProjectFile` 的可变借用就会 panic，调用方只能把任务容器临时移出文件再处理
        // （代价是顶层任务计数丢失、WBS 恒为 1）。此测试直接复现该调用姿势。
        let (mut container, project_file) = container();

        for id in 1..=3 {
            let task = add_task(&mut container, &project_file, id, 0);
            let mut task_ref = task.borrow_mut();
            // 模拟「文件里没有 WBS / 大纲编号列」
            task_ref.set_wbs(None);
            task_ref.set_outline_number(None);
        }

        // 把容器装回父文件，然后**持有其可变借用**原地调用
        *project_file.borrow_mut().get_tasks_mut() = container;
        project_file.borrow_mut().update_structure();

        let file = project_file.borrow();
        let wbs: Vec<Option<String>> = file
            .get_tasks()
            .as_slice()
            .iter()
            .map(|task| task.borrow().get_wbs())
            .collect();

        assert_eq!(
            wbs,
            vec![
                Some("1".to_owned()),
                Some("2".to_owned()),
                Some("3".to_owned())
            ]
        );
    }

    #[test]
    fn synchronize_task_id_to_hierarchy_renumbers_depth_first() {
        let (mut container, project_file) = container();

        // ID 与大纲级别一致：5（顶层）→ 6 → 7
        let root = add_task(&mut container, &project_file, 5, 0);
        let child = add_task(&mut container, &project_file, 6, 1);
        let grandchild = add_task(&mut container, &project_file, 7, 2);

        container.update_structure(true, true, true);
        container.synchronize_task_id_to_hierarchy();

        // Java: currentID 从 firstUniqueID()（1）开始，深度优先依次编号
        assert_eq!(read_task(&root, |task| task.get_id()).flatten(), Some(1));
        assert_eq!(read_task(&child, |task| task.get_id()).flatten(), Some(2));
        assert_eq!(read_task(&grandchild, |task| task.get_id()).flatten(), Some(3));

        // Java: 同时更新大纲级别（currentLevel 初始值等于 currentID，即 1）
        assert_eq!(
            read_task(&root, |task| task.get_outline_level()).flatten(),
            Some(1)
        );
        assert_eq!(
            read_task(&child, |task| task.get_outline_level()).flatten(),
            Some(2)
        );
        assert_eq!(
            read_task(&grandchild, |task| task.get_outline_level()).flatten(),
            Some(3)
        );
    }

    #[test]
    fn renumber_ids_starts_at_one_or_zero() {
        let (mut container, project_file) = container();
        let first = add_task(&mut container, &project_file, 5, 0);
        let second = add_task(&mut container, &project_file, 2, 0);

        container.renumber_ids();

        assert_eq!(read_task(&second, |task| task.get_id()).flatten(), Some(1));
        assert_eq!(read_task(&first, |task| task.get_id()).flatten(), Some(2));

        // ID 为 0 的任务存在时从 0 开始
        // （这里不再调用 container()，其上文的同名局部变量会遮蔽该测试辅助函数）
        let second_project_file = ProjectFile::new();
        let mut second_container = TaskContainer::new();
        second_container.set_context(&second_project_file);
        let zero = add_task(&mut second_container, &second_project_file, 0, 0);
        let other = add_task(&mut second_container, &second_project_file, 3, 0);

        second_container.renumber_ids();

        assert_eq!(read_task(&zero, |task| task.get_id()).flatten(), Some(0));
        assert_eq!(read_task(&other, |task| task.get_id()).flatten(), Some(1));
    }

    #[test]
    fn custom_fields_are_empty_without_configuration() {
        let (container, _) = container();

        assert!(container.get_custom_fields().is_empty());
        assert_eq!(container.get_field_type_by_alias("MyAlias"), None);

        // 父文件未接线时同样退化为空
        let container = TaskContainer::new();
        assert!(container.get_custom_fields().is_empty());
        assert_eq!(container.get_field_type_by_alias("MyAlias"), None);
    }

    #[test]
    fn default_matches_new_and_debug_reports_sizes() {
        let container = TaskContainer::default();
        assert!(container.is_empty());
        assert!(container.get_child_tasks().is_empty());

        let text = format!("{container:?}");
        assert!(text.contains("size: 0"));
        assert!(text.contains("child_task_count: 0"));
        assert!(text.contains("context_is_set: false"));
    }

    #[test]
    fn context_can_be_read_back() {
        let (container, project_file) = container();
        assert!(Rc::ptr_eq(
            &container.get_context().expect("已接线"),
            &project_file
        ));
        assert!(TaskContainer::new().get_context().is_none());
    }
}
