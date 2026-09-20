//! 对应 Java: `org/mpxj/RelationContainer.java`
//!
//! 当前项目全部 [`Relation`] 的容器（含「任务 → 前驱 / 后继关系」索引）。
//!
//! # Java → Rust 方法对照
//!
//! | Java | Rust |
//! |---|---|
//! | `public RelationContainer(ProjectFile projectFile)` | [`RelationContainer::new`] + [`RelationContainer::set_context`]（Java 在构造时拿到父文件，Rust 侧拆成两步，见下） |
//! | `public List<Relation> getPredecessors(Task task)` | [`RelationContainer::get_predecessors`]（返回快照 `Vec`） |
//! | `public List<Relation> getSuccessors(Task task)` | [`RelationContainer::get_successors`]（返回快照 `Vec`） |
//! | `@Override protected void added(Relation element)` | 私有方法 `added`（由 [`RelationContainer::add`] / [`RelationContainer::set`] 调用） |
//! | `@Override protected void removed(Relation element)` | 私有方法 `removed`（由 [`RelationContainer::remove`] / [`RelationContainer::remove_at`] / [`RelationContainer::set`] 调用） |
//! | `@Override protected void replaced(Relation oldElement, Relation newElement)` | 私有方法 `replaced`（由 [`RelationContainer::set`] 调用） |
//! | `public Relation addPredecessor(Relation.Builder builder)` | [`RelationContainer::add_predecessor`] |
//! | `public boolean removePredecessor(Task, Task, RelationType, Duration)` | [`RelationContainer::remove_predecessor`] |
//! | 继承自 `ProjectEntityContainer<Relation>` 的 `getByUniqueID` | [`RelationContainer::get_by_unique_id`] |
//! | 继承自 `ProjectEntityContainer` 的 `updateUniqueID(T, Integer, Integer)` | [`RelationContainer::update_unique_id`] |
//! | 继承自 `ProjectEntityContainer` 的 `fixUniqueIdClashes()` | [`RelationContainer::fix_unique_id_clashes`] |
//! | 继承自 `ProjectEntityContainer` 的 `firstUniqueID()` | [`RelationContainer::first_unique_id`] |
//! | 继承自 `ListWithCallbacks` 的 `add(T)` / `remove(int)` / `set(int, T)` / `clear()` / `get(int)` / `size()` / `isEmpty()` / `iterator()` | [`RelationContainer::add`] / [`RelationContainer::remove_at`] / [`RelationContainer::set`] / [`RelationContainer::clear`] / [`RelationContainer::get`] / [`RelationContainer::len`] / [`RelationContainer::is_empty`] / [`RelationContainer::iter`] |
//! | `ListWithCallbacks.remove(Object)` | [`RelationContainer::remove`]（Java 用引用相等定位，Rust 用指针同一性，见下） |
//! | `ProjectEntityContainer.getSequenceProvider()` | 未提供（Rust 侧由容器内部的 [`RelationContainer::sequence`] 私有方法经父文件取序号生成器） |
//! | `ProjectEntityContainer` 内部的 `m_uniqueIDMap` / `m_uniqueIDClashList` | 无索引（改为线性查找）+ `unique_id_clash_list`（与 Java 冲突列表同义） |
//!
//! # 跳过的 Java 代码及原因
//!
//! | Java | 原因 |
//! |---|---|
//! | `ProjectEntityContainer` 的 `getSequenceProvider()` | Rust 侧容器直接持有父 `ProjectFile` 与 `ProjectContext` 的弱引用，序号生成器经它们取得（见 [`RelationContainer::sequence`]），不再单独暴露提供者。 |
//! | 与 unique ID 索引相关的内部机制（`m_uniqueIDMap`） | 见下「未组合基类」。 |
//! | 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable` | 本类未使用这些机制 |
//!
//! # 未组合基类（`ProjectEntityContainer`）
//!
//! Java 的 `RelationContainer extends ProjectEntityContainer<Relation>`，基类提供列表存储、
//! `added` / `removed` 钩子、unique ID 索引与序号同步。本 crate 已移植
//! [`crate::project_entity_container::ProjectEntityContainer`]，但它要求元素实现
//! `ProjectEntityWithUniqueID` 并以 `Rc<RefCell<T>>` 存储、且 `sort` 要求元素 `Ord`；
//! 更重要的是它把「元素 → 容器」的 weak 反向引用留给上层接线，而本容器的
//! `added` / `removed` / `replaced` 与 `removePredecessor` 都直接操作**索引**（前驱 / 后继映射），
//! 与基类的回调组合起来会引入额外的借用重入问题。因此按
//! [`crate::project_calendar_container::ProjectCalendarContainer`] 的既有做法，
//! 本容器**自行**持有 `Vec<Rc<RefCell<Relation>>>`，把基类的公开行为逐个实现（见上表），
//! 并用**按 unique ID 线性查找**替代索引：线性查找天然不会因 unique ID 变化而失效，
//! 因此不再需要 `m_uniqueIDMap` 与配套方法。
//!
//! # 与 Java 的差异
//!
//! - **父文件的反向引用**：Java 的构造方法接收 `ProjectFile` 并以强引用保存在基类
//!   （`super(projectFile)`，同时也是基类的序号生成器提供者）。Rust 侧若强引用会形成
//!   「`ProjectFile` → 容器 → `ProjectFile`」的 `Rc` 循环，故改为
//!   `Weak<RefCell<ProjectFile>>`，先以空弱引用构造，再由
//!   [`RelationContainer::set_context`] 接线（Java 在构造方法里完成）。
//! - **自动 unique ID（Java 在 `Relation` 的构造方法里做）**：Java 的
//!   `Relation.Builder.build()` 会在 `uniqueID == null` 且
//!   `ProjectConfig.getAutoRelationUniqueID()` 为真时，经后继任务的父文件向
//!   `Relation` 序号生成器申请一个 ID，并调用容器的 `updateUniqueID` 登记。
//!   Rust 侧 [`RelationBuilder::build`](crate::relation::Builder::build) 返回裸值、
//!   且拿不到容器，故把这套步骤放在 [`RelationContainer::add_predecessor`] 中执行
//!   （语义与顺序完全对应 Java 的构造方法，见方法注释）。
//! - **`ProjectContext` 的弱引用（Rust 特有）**：Java 的容器经 `m_sequenceProvider`
//!   （即父 `ProjectFile`）取序号生成器；Rust 侧的容器若照此只经 `ProjectFile` 取，
//!   则在**调用方正持有 `ProjectFile` 借用**时必然取不到 —— 而
//!   `Task::add_predecessor` 正是 `file.borrow_mut().get_relations_mut().add_predecessor(..)`
//!   这种形态（也是所有 reader 的入口）。故与 [`crate::task_container::TaskContainer`]
//!   同法，额外记一份 `ProjectContext` 的弱引用（`ProjectContext` 是独立的 `RefCell`，
//!   不受 `ProjectFile` 借用影响），由 [`RelationContainer::set_project_context`] 接线。
//!   两个来源都取不到时退化为「不分配 / 跳过」（Java 此时必然可用），不 panic。
//! - **`Relation::set_unique_id` 的回调未接线**：Java 的 `setUniqueID` 会经
//!   `m_successorTask.getParentFile().getRelations().updateUniqueID(this, ..)` 通知容器。
//!   Rust 的 [`Relation::set_unique_id`](crate::relation::Relation::set_unique_id) 只有
//!   `&mut self`、拿不到自身的 `Rc` 句柄（`Relation` 无 `self_ref` 弱引用），无法构造
//!   `updateUniqueID` 的 `element` 参数，故该回调未接线：序号同步由容器自己的
//!   [`RelationContainer::update_unique_id`]（`add_predecessor` 与
//!   [`RelationContainer::fix_unique_id_clashes`] 会调用）完成。
//! - **前驱 / 后继索引的键**：Java 用 `Map<Task, List<Relation>>`，键为 `Task` 的对象身份
//!   （`Task` 未覆写 `hashCode` / `equals`）。Rust 侧 `Task` 不可作 `Hash` 键，改用
//!   **Task 值自身的内存地址**（`*const Task as usize`）作身份键，语义与 Java 的引用相等一致；
//!   键由 [`RelationContainer::added`] / [`RelationContainer::removed`] 维护，
//!   因此 [`RelationContainer::get_predecessors`] / [`RelationContainer::get_successors`]
//!   只需一个 `&Task`（无需 `Rc`，便于 `Task::get_successors(&self)` 之类的调用方使用）。
//! - **`add` / `remove` 的属性判定**：Java 的 `ListWithCallbacks.remove(Object)` 用默认
//!   `equals`（引用相等）定位元素；Rust 侧的 [`RelationContainer::remove`] 接收
//!   `&Relation`，用 `RefCell::as_ptr` 做**指针同一性**比较（等价于 Java 的默认 `equals`）。
//! - **集合返回值**：Java 的 `getPredecessors` / `getSuccessors` 返回内部列表视图
//!   （无匹配时是共享的 `EMPTY_LIST`）；Rust 侧返回**快照** `Vec`（元素为 `Rc` 句柄，
//!   元素本身仍是共享的）。
//! - **空值语义**：Java 对 `element.getSuccessorTask()` 为 `null` 时会抛
//!   `NullPointerException`；Rust 侧不 panic，退化为「不建立索引 / 查不到」。
//! - **元素形态**：元素为 `Rc<RefCell<Relation>>`（符合「共享实体用 `Rc<RefCell<T>>`」的约定），
//!   因此 `removePredecessor` 中命中后按 `Rc::ptr_eq` 定位并移除。
//! - **`update_unique_id` 的 `new` 为 `null`**：Java 会把 `null` 写入 map 键；Rust 侧
//!   不建立索引（映射键为 `i32`，等价于「查不到」），其余行为一致。
//! - **`clear()`**：Java 的 `ListWithCallbacks.clear()` 只清空列表，前驱 / 后继映射会残留
//!   已移除的关系（陈旧索引）。Rust 侧的 [`RelationContainer::clear`] 一并清空两个索引，
//!   使 `getPredecessors` / `getSuccessors` 不会返回已不在容器中的关系（更安全，正常读取
//!   路径的结果不变）。
//! - **`fix_unique_id_clashes` 依赖父文件**：Java 能保证 `m_sequenceProvider` 非空；
//!   Rust 侧两个来源（`ProjectContext` / `ProjectFile`）都取不到序号生成器时直接返回，
//!   并保留冲突列表待接线后重试（Java 无此情形）。

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::{Rc, Weak};

use crate::common::object_sequence::ObjectSequence;
use crate::duration::Duration;
use crate::project_file::ProjectFile;
use crate::relation::{lag_matches, Builder as RelationBuilder, Relation};
use crate::relation_type::RelationType;
use crate::task::Task;
// `ProjectFile` 可能以「实现本 trait」或「提供同签名固有方法」两种形式给出（见文件头
// 「接口假设」）；前者需要本导入，后者不会用到它，故显式允许未使用。
#[allow(unused_imports)]
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;

/// 当前项目全部关系的容器。
///
/// 对应 Java: `public class RelationContainer extends ProjectEntityContainer<Relation>`
pub struct RelationContainer {
    /// 容器内的关系。
    ///
    /// 对应 Java: 基类 `ProjectEntityContainer` 内部的列表存储。
    items: Vec<Rc<RefCell<Relation>>>,

    /// 「后继任务 → 指向它的关系（即该任务的前驱关系）」索引。
    ///
    /// 对应 Java 字段: `private final Map<Task, List<Relation>> m_predecessors = new HashMap<>();`
    /// （键为 Task 对象的身份，Rust 侧用 Task 值地址表达，见文件头）
    predecessors: HashMap<usize, Vec<Rc<RefCell<Relation>>>>,

    /// 「前驱任务 → 从它出发的关系（即该任务的后继关系）」索引。
    ///
    /// 对应 Java 字段: `private final Map<Task, List<Relation>> m_successors = new HashMap<>();`
    successors: HashMap<usize, Vec<Rc<RefCell<Relation>>>>,

    /// unique ID 冲突列表。
    ///
    /// 对应 Java 字段: `ProjectEntityContainer.m_uniqueIDClashList`
    unique_id_clash_list: Vec<Rc<RefCell<Relation>>>,

    /// 父项目文件（弱引用，避免 `Rc` 循环）。
    ///
    /// 对应 Java: `RelationContainer(ProjectFile projectFile)` 传入、由基类保存的
    /// `ProjectFile`（同时是基类的序号生成器提供者 `m_sequenceProvider`）。
    context: Weak<RefCell<ProjectFile>>,

    /// 项目上下文（弱引用）。
    ///
    /// Rust 特有：Java 只持父 `ProjectFile`，经它间接访问序号生成器与项目配置。
    /// Rust 侧单独记一份，是为了让「自动分配关系 unique ID」所需的 `ProjectConfig` 与
    /// `Relation` 序号生成器能在**不借用 `ProjectFile`** 的前提下读到 ——
    /// `ProjectContext` 是独立的 `RefCell`，因此调用方持有 `ProjectFile` 借用时
    /// （`Task::add_predecessor` 即如此）也能安全访问。见文件头说明。
    project_context: Weak<RefCell<crate::project_context::ProjectContext>>,
}

impl RelationContainer {
    /// 构造方法。
    ///
    /// 对应 Java: `public RelationContainer(ProjectFile projectFile)`
    ///
    /// Rust 侧先以空父文件弱引用构造，由 [`RelationContainer::set_context`] 完成接线。
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            predecessors: HashMap::new(),
            successors: HashMap::new(),
            unique_id_clash_list: Vec::new(),
            context: Weak::new(),
            project_context: Weak::new(),
        }
    }

    /// 接线父项目文件（Rust 特有：Java 在构造方法中直接接收 `ProjectFile`）。
    ///
    /// 对应 Java: `super(projectFile)` 与基类字段 `m_sequenceProvider`
    pub(crate) fn set_context(&mut self, project_file: &Rc<RefCell<ProjectFile>>) {
        self.context = Rc::downgrade(project_file);

        // Rust 特有：同时记下 `ProjectContext`（见字段说明）。此处 `project_file`
        // 可能已被调用方可变借用（`ProjectFile::new_with_context` 的接线阶段即如此），
        // 那时读不到，由 `set_project_context` 补接线。
        self.project_context = match project_file.try_borrow() {
            Ok(file_ref) => Rc::downgrade(&file_ref.get_project_context()),
            Err(_) => Weak::new(),
        };
    }

    /// 接线项目上下文（Rust 特有：Java 经父 `ProjectFile` 间接访问）。
    ///
    /// 对应 Java: 基类 `ProjectEntityContainer` 的 `m_sequenceProvider`
    /// （构造时注入，容器据此取 `ProjectConfig` 与 `Relation` 序号生成器）。
    pub(crate) fn set_project_context(
        &mut self,
        context: &Rc<RefCell<crate::project_context::ProjectContext>>,
    ) {
        self.project_context = Rc::downgrade(context);
    }

    /// 取父项目文件（未接线时返回 `None`）。
    ///
    /// 对应 Java: 基类持有的 `ProjectFile`（`ProjectFile.getRelations()` 的反向关系）
    pub fn get_context(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.context.upgrade()
    }

    /// 向容器追加一条关系，并更新前驱 / 后继索引。
    ///
    /// 对应 Java: `ListWithCallbacks.add(T element)`（进而触发 `added` 回调）
    pub fn add(&mut self, relation: Rc<RefCell<Relation>>) {
        self.items.push(Rc::clone(&relation));
        self.added(&relation);
    }

    /// 取指定任务的前驱关系；没有时返回空集合。
    ///
    /// 对应 Java: `public List<Relation> getPredecessors(Task task)`
    pub fn get_predecessors(&self, task: &Task) -> Vec<Rc<RefCell<Relation>>> {
        self.predecessors
            .get(&task_key(task))
            .cloned()
            .unwrap_or_default()
    }

    /// 取指定任务的后继关系；没有时返回空集合。
    ///
    /// 对应 Java: `public List<Relation> getSuccessors(Task task)`
    pub fn get_successors(&self, task: &Task) -> Vec<Rc<RefCell<Relation>>> {
        self.successors
            .get(&task_key(task))
            .cloned()
            .unwrap_or_default()
    }

    /// 按构造器创建前驱关系并加入容器；若两任务之间已存在「同前驱 + 同类型 + 同 lag」的关系，
    /// 则直接返回已有关系（不新建）。
    ///
    /// 对应 Java: `public Relation addPredecessor(Relation.Builder builder)`
    pub fn add_predecessor(&mut self, builder: RelationBuilder) -> Rc<RefCell<Relation>> {
        //
        // Java: 先在 m_predecessors 中查找同前驱的关系，完全匹配则直接复用
        //
        if let Some(existing) = self.find_matching_predecessor(&builder) {
            return existing;
        }

        //
        // Java: 必要时创建新的前驱关系并加入容器
        //
        // Java 的 `Relation` 构造方法（即 `builder.build()`）在此处分三步完成
        // unique ID 的处理，这里逐条对应：
        //
        //   Integer uniqueID = builder.m_uniqueID;
        //   if (uniqueID == null) {
        //      ProjectFile project = m_successorTask.getParentFile();
        //      if (project.getProjectConfig().getAutoRelationUniqueID()) {
        //         uniqueID = project.getUniqueIdObjectSequence(Relation.class).getNext();
        //      }
        //   }
        //   if (uniqueID != null) {
        //      m_successorTask.getParentFile().getRelations().updateUniqueID(this, m_uniqueID, uniqueID);
        //      m_uniqueID = uniqueID;
        //   }
        //
        // （Rust 侧的 `Builder::build()` 返回裸值、也拿不到容器，故这几步落在容器中。）
        let mut relation = builder.build();

        // Java: if (uniqueID == null) { ... getUniqueIdObjectSequence(Relation.class).getNext(); }
        if relation.get_unique_id().is_none() && self.auto_relation_unique_id() {
            if let Some(sequence) = self.sequence() {
                relation.set_unique_id(Some(sequence.get_next()));
            }
        }

        let relation = Rc::new(RefCell::new(relation));

        // Java: if (uniqueID != null) { ...updateUniqueID(this, m_uniqueID, uniqueID); m_uniqueID = uniqueID; }
        // 此刻该关系尚未登记进容器（Java 亦然），`oldUniqueID` 为构造前的字段值 `null`。
        if let Some(unique_id) = read_unique_id(&relation) {
            self.update_unique_id(&relation, None, Some(unique_id));
        }

        self.add(Rc::clone(&relation));

        relation
    }

    /// 从指定后继任务上删除一条匹配的前驱关系。
    ///
    /// 对应 Java: `public boolean removePredecessor(Task successorTask, Task predecessorTask, RelationType type, Duration lag)`
    pub fn remove_predecessor(
        &mut self,
        successor_task: &Task,
        predecessor_task: &Task,
        relation_type: RelationType,
        lag: Option<Duration>,
    ) -> bool {
        //
        // Java: 取出该后继任务的前驱关系列表
        //
        let candidates = match self.predecessors.get(&task_key(successor_task)) {
            Some(candidates) => candidates.clone(),
            None => return false,
        };

        if candidates.is_empty() {
            return false;
        }

        //
        // Java: lag 为 null 时按 0 天处理（Rust 侧 None 即 0 天）
        //
        for relation in candidates {
            let matched = match relation.try_borrow() {
                Ok(relation) => {
                    let same_predecessor = relation
                        .get_predecessor_task()
                        .map(|task| same_task(&task, predecessor_task))
                        .unwrap_or(false);

                    same_predecessor
                        && relation.get_type() == relation_type
                        && lag_matches(relation.get_lag(), lag)
                }
                // Java 无此分支（元素必然可读）；Rust 侧避免借用冲突 panic
                Err(_) => false,
            };

            if matched {
                self.remove_element(&relation);
                return true;
            }
        }

        false
    }

    /// 移除容器中的一条关系（按引用相等定位）。
    ///
    /// 对应 Java: `ListWithCallbacks.remove(Object o)`（内部会调用 `removed(T element)`）
    pub fn remove(&mut self, relation: &Relation) -> bool {
        let target = relation as *const Relation;
        let index = self
            .items
            .iter()
            .position(|candidate| candidate.as_ptr() as *const Relation == target);

        match index {
            Some(index) => {
                self.remove_at(index);
                true
            }
            None => false,
        }
    }

    /// 移除指定下标的关系并返回它（越界返回 `None`）。
    ///
    /// 对应 Java: `ListWithCallbacks.remove(int index)`
    pub fn remove_at(&mut self, index: usize) -> Option<Rc<RefCell<Relation>>> {
        if index >= self.items.len() {
            return None;
        }

        let removed = self.items.remove(index);
        self.removed(&removed);
        Some(removed)
    }

    /// 替换指定下标的关系，返回被替换掉的旧关系（越界返回 `None`）。
    ///
    /// 对应 Java: `ListWithCallbacks.set(int index, T element)`（触发 `replaced`）
    pub fn set(&mut self, index: usize, relation: Rc<RefCell<Relation>>) -> Option<Rc<RefCell<Relation>>> {
        if index >= self.items.len() {
            return None;
        }

        let old_relation = std::mem::replace(&mut self.items[index], relation);
        let new_relation = Rc::clone(&self.items[index]);
        self.replaced(&old_relation, &new_relation);
        Some(old_relation)
    }

    /// 清空容器（不触发 `removed`，与 Java 一致）。
    ///
    /// 对应 Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.items.clear();
        self.predecessors.clear();
        self.successors.clear();
    }

    /// 按下标取关系；越界返回 `None`。
    ///
    /// 对应 Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<Rc<RefCell<Relation>>> {
        self.items.get(index).map(Rc::clone)
    }

    /// 关系个数。
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

    /// 迭代容器内的关系。
    ///
    /// 对应 Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<Relation>>> {
        self.items.iter()
    }

    /// 以切片形式访问容器内的关系。
    ///
    /// 对应 Java: 基类（`ArrayList`）的列表视图。
    pub fn as_slice(&self) -> &[Rc<RefCell<Relation>>] {
        self.items.as_slice()
    }

    /// 按 unique ID 查找关系，找不到时返回 `None`。
    ///
    /// 对应 Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    ///
    /// Rust 侧按 unique ID 线性查找（无索引，见文件头）。
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Relation>>> {
        let id = id?;
        self.items
            .iter()
            .find(|relation| read_unique_id(relation) == Some(id))
            .map(Rc::clone)
    }

    /// 重排 unique ID 时使用的第一个值（基类默认值）。
    ///
    /// 对应 Java: `ProjectEntityContainer.firstUniqueID()`（`RelationContainer` 未覆写）
    pub fn first_unique_id(&self) -> i32 {
        1
    }

    /// 关系的 unique ID 变化时更新容器状态。
    ///
    /// 对应 Java: `ProjectEntityContainer.updateUniqueID(T element, Integer oldUniqueID, Integer newUniqueID)`
    ///
    /// Rust 侧按 unique ID 线性查找，因此只需维护冲突列表与序号生成器；Java 中被调用方
    /// （`Relation.setUniqueID`）触发的调用在 Rust 侧由使用方自行发起（见
    /// [`crate::relation::Relation::set_unique_id`] 的说明）。
    pub fn update_unique_id(
        &mut self,
        element: &Rc<RefCell<Relation>>,
        old_unique_id: Option<i32>,
        new_unique_id: Option<i32>,
    ) {
        // Java: if (oldUniqueID != null) { m_uniqueIDMap.remove(oldUniqueID); }
        // Rust 侧为线性查找，没有需要移除的索引项。
        let _ = old_unique_id;

        // Java: T currentElement = m_uniqueIDMap.get(newUniqueID);
        //       if (currentElement == element) return;
        let current_element = self.get_by_unique_id(new_unique_id);
        if let Some(current_element) = &current_element {
            if Rc::ptr_eq(current_element, element) {
                return;
            }
        }

        // Java: if (currentElement instanceof ProjectEntityWithMutableUniqueID)
        //       { m_uniqueIDClashList.add(element); }
        // Relation 的 unique ID 可变，故与 Java 一致地记录冲突。
        if current_element.is_some() {
            self.unique_id_clash_list.push(Rc::clone(element));
        }

        // Java: m_uniqueIDMap.put(newUniqueID, element);
        //       m_sequenceProvider.getUniqueIdObjectSequence(element.getClass()).sync(newUniqueID);
        if let Some(sequence) = self.sequence() {
            sequence.sync(new_unique_id);
        }
    }

    /// 为出现 unique ID 冲突的关系重新分配 unique ID。
    ///
    /// 对应 Java: `ProjectEntityContainer.fixUniqueIdClashes()`
    pub fn fix_unique_id_clashes(&mut self) {
        if self.unique_id_clash_list.is_empty() {
            return;
        }

        // Java: ObjectSequence sequence = m_sequenceProvider.getUniqueIdObjectSequence(
        //          m_uniqueIDClashList.get(0).getClass());
        // Rust 侧两个来源（`ProjectContext` / `ProjectFile`）都取不到序号生成器时
        // 无法重排（Java 此时必然可用），保留冲突列表待接线后再处理。
        let Some(sequence) = self.sequence() else {
            return;
        };

        // Java: m_uniqueIDClashList.forEach(i -> i.setUniqueID(sequence.getNext()));
        for relation in std::mem::take(&mut self.unique_id_clash_list) {
            if let Ok(mut relation) = relation.try_borrow_mut() {
                relation.set_unique_id(Some(sequence.get_next()));
            }
        }

        // Java: m_uniqueIDMap.clear(); forEach(i -> m_uniqueIDMap.put(...));
        // Rust 侧按 unique ID 线性查找，没有索引需要重建。
    }

    /// 关系加入后的索引维护。
    ///
    /// 对应 Java: `@Override protected void added(Relation element)`
    fn added(&mut self, element: &Rc<RefCell<Relation>>) {
        // Java: m_predecessors.computeIfAbsent(element.getSuccessorTask(), t -> new ArrayList<>()).add(element);
        if let Some(key) = referenced_task_key(element, TaskRole::Successor) {
            self.predecessors
                .entry(key)
                .or_default()
                .push(Rc::clone(element));
        }

        // Java: m_successors.computeIfAbsent(element.getPredecessorTask(), t -> new ArrayList<>()).add(element);
        if let Some(key) = referenced_task_key(element, TaskRole::Predecessor) {
            self.successors
                .entry(key)
                .or_default()
                .push(Rc::clone(element));
        }
    }

    /// 关系移除后的索引维护。
    ///
    /// 对应 Java: `@Override protected void removed(Relation element)`
    fn removed(&mut self, element: &Rc<RefCell<Relation>>) {
        // Java: m_predecessors.getOrDefault(element.getSuccessorTask(), EMPTY_LIST).remove(element);
        if let Some(key) = referenced_task_key(element, TaskRole::Successor) {
            if let Some(list) = self.predecessors.get_mut(&key) {
                list.retain(|candidate| !Rc::ptr_eq(candidate, element));
            }
        }

        // Java: m_successors.getOrDefault(element.getPredecessorTask(), EMPTY_LIST).remove(element);
        if let Some(key) = referenced_task_key(element, TaskRole::Predecessor) {
            if let Some(list) = self.successors.get_mut(&key) {
                list.retain(|candidate| !Rc::ptr_eq(candidate, element));
            }
        }
    }

    /// 关系被替换时的索引维护。
    ///
    /// 对应 Java: `@Override protected void replaced(Relation oldElement, Relation newElement)`
    fn replaced(&mut self, old_element: &Rc<RefCell<Relation>>, new_element: &Rc<RefCell<Relation>>) {
        // Java: removed(oldElement); added(newElement);
        self.removed(old_element);
        self.added(new_element);
    }

    /// 在「后继任务的前驱关系」中查找与构造器完全匹配的已有关系（Java 的 `addPredecessor`
    /// 第一段逻辑）。
    fn find_matching_predecessor(
        &self,
        builder: &RelationBuilder,
    ) -> Option<Rc<RefCell<Relation>>> {
        // 持有 Rc 直到查找结束，避免任务被释放导致地址失效。
        let successor = builder.successor_task.upgrade()?;
        let predecessor = builder.predecessor_task.upgrade()?;

        let key = task_key_of(&successor)?;
        let predecessor_key = task_key_of(&predecessor)?;

        for relation in self.predecessors.get(&key)? {
            // Java: if (relation.getPredecessorTask() == builder.m_predecessorTask)
            let same_predecessor = match relation.try_borrow() {
                Ok(value) => value
                    .get_predecessor_task()
                    .map(|task| task_key_of(&task) == Some(predecessor_key))
                    .unwrap_or(false),
                Err(_) => false,
            };

            if !same_predecessor {
                continue;
            }

            // Java: if (relation.getType() == builder.m_type
            //             && relation.getLag().compareTo(builder.m_lag) == 0) { return relation; }
            // Java 只检查第一条同前驱的关系（未匹配则 break）。
            let matched = match relation.try_borrow() {
                Ok(value) => {
                    value.get_type() == builder.relation_type
                        && lag_matches(value.get_lag(), builder.lag)
                }
                Err(_) => false,
            };

            return if matched {
                Some(Rc::clone(relation))
            } else {
                None
            };
        }

        None
    }

    /// 按 `Rc` 身份移除元素并触发 `removed`。
    fn remove_element(&mut self, element: &Rc<RefCell<Relation>>) -> bool {
        let index = self
            .items
            .iter()
            .position(|candidate| Rc::ptr_eq(candidate, element));

        match index {
            Some(index) => {
                let removed = self.items.remove(index);
                self.removed(&removed);
                true
            }
            None => false,
        }
    }

    /// 取父项目文件提供的 `Relation` 序号生成器（两个来源都取不到时返回 `None`）。
    ///
    /// 对应 Java: `m_sequenceProvider.getUniqueIdObjectSequence(Relation.class)`
    ///
    /// Rust 侧优先经 [`RelationContainer::project_context`] 取：调用方（`Task::add_predecessor`
    /// 等）通常正持有 `ProjectFile` 的可变借用，此时只有独立的 `ProjectContext` 可借用。
    /// 退而求其次再经父 `ProjectFile` 取（`set_project_context` 未接线时的兜底）。
    fn sequence(&self) -> Option<Rc<ObjectSequence>> {
        if let Some(context) = self.project_context.upgrade() {
            if let Ok(mut context) = context.try_borrow_mut() {
                return Some(
                    crate::unique_id_object_sequence_provider::get_unique_id_object_sequence::<
                        Relation,
                    >(&mut *context),
                );
            }
        }

        let project_file = self.context.upgrade()?;
        let mut project_file = project_file.try_borrow_mut().ok()?;

        // 兼容两种形态：ProjectFile 作为 `UniqueIdObjectSequenceProvider` 的实现
        //（Java 一致），或提供同签名的固有方法（`crate::task` 的假设）。
        Some(project_file.get_unique_id_object_sequence(TypeId::of::<Relation>()))
    }

    /// 项目配置中的「自动分配关系 unique ID」开关（Java 默认 `true`）。
    ///
    /// 对应 Java: `m_successorTask.getParentFile().getProjectConfig().getAutoRelationUniqueID()`
    ///
    /// 取值路径与 [`RelationContainer::sequence`] 一致；两个来源都不可用时返回 `false`
    /// （此时序号生成器同样取不到，结果与 Java 一致：不分配 ID）。
    fn auto_relation_unique_id(&self) -> bool {
        if let Some(context) = self.project_context.upgrade() {
            if let Ok(context_ref) = context.try_borrow() {
                return context_ref
                    .get_project_config()
                    .get_auto_relation_unique_id();
            }
        }

        match self.context.upgrade() {
            Some(project_file) => match project_file.try_borrow() {
                Ok(project_file) => project_file
                    .get_project_config()
                    .get_auto_relation_unique_id(),
                Err(_) => false,
            },
            None => false,
        }
    }
}

impl Default for RelationContainer {
    /// 对应 Java: 显式声明的 `RelationContainer(ProjectFile)`，
    /// Rust 侧等价于 [`RelationContainer::new`]（父文件由 `set_context` 接线）。
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for RelationContainer {
    /// Java 侧未覆写 `toString()`；Rust 侧输出容器规模与索引规模
    /// （避免依赖 `Task` / `Relation` 的 `Debug` 细节）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("RelationContainer")
            .field("size", &self.items.len())
            .field("predecessor_index_size", &self.predecessors.len())
            .field("successor_index_size", &self.successors.len())
            .field("context_is_set", &(self.context.strong_count() > 0))
            .finish()
    }
}

/// 关系中任务引用的角色（用于取对应的索引键）。
enum TaskRole {
    /// 后继任务（对应 `m_predecessors` 的键）。
    Successor,
    /// 前驱任务（对应 `m_successors` 的键）。
    Predecessor,
}

/// 取关系中某个任务引用的身份键（任务已释放或正被可变借用时返回 `None`）。
fn referenced_task_key(relation: &Rc<RefCell<Relation>>, role: TaskRole) -> Option<usize> {
    let task = match relation.try_borrow() {
        Ok(relation) => match role {
            TaskRole::Successor => relation.get_successor_task(),
            TaskRole::Predecessor => relation.get_predecessor_task(),
        },
        Err(_) => None,
    };

    task.and_then(|task| task_key_of(&task))
}

/// Task 的身份键：Task 值自身的内存地址（等价于 Java 的引用相等）。
fn task_key(task: &Task) -> usize {
    task as *const Task as usize
}

/// 取 `Rc<RefCell<Task>>` 包装的任务的身份键（正被可变借用时返回 `None`）。
fn task_key_of(task: &Rc<RefCell<Task>>) -> Option<usize> {
    task.try_borrow().ok().map(|task| task_key(&task))
}

/// 判断 `Rc<RefCell<Task>>` 与 `&Task` 是否指向同一个任务。
fn same_task(task: &Rc<RefCell<Task>>, other: &Task) -> bool {
    task.try_borrow()
        .map(|task| std::ptr::eq(&*task, other))
        .unwrap_or(false)
}

/// 读取关系的 unique ID（正被可变借用时返回 `None`）。
fn read_unique_id(relation: &Rc<RefCell<Relation>>) -> Option<i32> {
    relation
        .try_borrow()
        .ok()
        .and_then(|relation| relation.get_unique_id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_unit::TimeUnit;

    /// 构造一个独立任务。
    fn task() -> Rc<RefCell<Task>> {
        Rc::new(RefCell::new(Task::new()))
    }

    /// 构造一条关系。
    fn relation(
        predecessor: &Rc<RefCell<Task>>,
        successor: &Rc<RefCell<Task>>,
        relation_type: RelationType,
        lag: Option<Duration>,
    ) -> Rc<RefCell<Relation>> {
        Rc::new(RefCell::new(
            RelationBuilder::new()
                .predecessor_task(predecessor)
                .successor_task(successor)
                .r#type(relation_type)
                .lag(lag)
                .build(),
        ))
    }

    /// 构造一个未接线的容器。
    fn container() -> RelationContainer {
        RelationContainer::new()
    }

    /// 构造一个接线到新建父文件的容器。
    fn wired_container() -> (RelationContainer, Rc<RefCell<ProjectFile>>) {
        let project_file = ProjectFile::new();
        let mut container = RelationContainer::new();
        container.set_context(&project_file);
        (container, project_file)
    }

    #[test]
    fn add_indexes_predecessors_and_successors() {
        let first = task();
        let second = task();
        let third = task();

        let mut container = container();
        let ab = relation(&first, &second, RelationType::FinishStart, None);
        let cb = relation(&third, &second, RelationType::FinishStart, None);

        container.add(Rc::clone(&ab));
        container.add(Rc::clone(&cb));

        assert_eq!(container.len(), 2);
        assert!(!container.is_empty());
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert!(Rc::ptr_eq(&container.get(0).expect("下标 0 存在"), &ab));
        assert!(container.get(2).is_none());

        // 第二个任务有两条前驱关系（A -> B、C -> B）
        let predecessors = container.get_predecessors(&second.borrow());
        assert_eq!(predecessors.len(), 2);
        assert!(predecessors.iter().any(|item| Rc::ptr_eq(item, &ab)));
        assert!(predecessors.iter().any(|item| Rc::ptr_eq(item, &cb)));

        // 第一个任务有一条后继关系
        let successors = container.get_successors(&first.borrow());
        assert_eq!(successors.len(), 1);
        assert!(Rc::ptr_eq(&successors[0], &ab));

        // 没有关系的任务返回空集合
        assert!(container.get_predecessors(&first.borrow()).is_empty());
        assert!(container.get_successors(&task().borrow()).is_empty());

        // third 的后继关系是 C -> B，first 的前驱关系为空
        let third_successors = container.get_successors(&third.borrow());
        assert_eq!(third_successors.len(), 1);
        assert!(Rc::ptr_eq(&third_successors[0], &cb));
    }

    #[test]
    fn add_predecessor_deduplicates_identical_relations() {
        let predecessor = task();
        let successor = task();

        let mut container = container();

        let builder = || {
            RelationBuilder::new()
                .predecessor_task(&predecessor)
                .successor_task(&successor)
                .r#type(RelationType::FinishStart)
                .lag(Some(Duration::get_instance_int(2, TimeUnit::Days)))
        };

        let first = container.add_predecessor(builder());
        assert_eq!(container.len(), 1);

        // Java: 同前驱 + 同类型 + 同 lag → 复用已有关系
        let second = container.add_predecessor(builder());
        assert!(Rc::ptr_eq(&first, &second));
        assert_eq!(container.len(), 1);

        // 类型不同 → 新建
        let different_type = container.add_predecessor(builder().r#type(RelationType::StartStart));
        assert!(!Rc::ptr_eq(&first, &different_type));
        assert_eq!(container.len(), 2);

        // lag 不同 → 新建（1 天 ≠ 2 天）
        let different_lag = container.add_predecessor(builder().lag(Some(Duration::get_instance_int(1, TimeUnit::Days))));
        assert!(!Rc::ptr_eq(&first, &different_lag));
        assert_eq!(container.len(), 3);

        // Java: 只检查第一条同前驱的关系，未完全匹配即跳出循环（不会再找后面的关系）
        let zero_lag_implicit = container.add_predecessor(
            RelationBuilder::new()
                .predecessor_task(&predecessor)
                .successor_task(&successor)
                .r#type(RelationType::FinishFinish),
        );
        assert_eq!(container.len(), 4);

        let zero_lag_explicit = container.add_predecessor(
            RelationBuilder::new()
                .predecessor_task(&predecessor)
                .successor_task(&successor)
                .r#type(RelationType::FinishFinish)
                .lag(Some(Duration::get_instance_int(0, TimeUnit::Days))),
        );
        assert!(!Rc::ptr_eq(&zero_lag_implicit, &zero_lag_explicit));
        assert_eq!(container.len(), 5);

        // 未设置 lag（等价 0 天）与显式 0 天视为相同（第一条同前驱的关系即为 FF）
        // （这里不再调用 container()，其上文已存在同名局部变量）
        let mut fresh_container = RelationContainer::new();
        let implicit_zero = fresh_container.add_predecessor(
            RelationBuilder::new()
                .predecessor_task(&predecessor)
                .successor_task(&successor)
                .r#type(RelationType::FinishFinish),
        );
        let explicit_zero = fresh_container.add_predecessor(
            RelationBuilder::new()
                .predecessor_task(&predecessor)
                .successor_task(&successor)
                .r#type(RelationType::FinishFinish)
                .lag(Some(Duration::get_instance_int(0, TimeUnit::Days))),
        );
        assert!(Rc::ptr_eq(&implicit_zero, &explicit_zero));
        assert_eq!(fresh_container.len(), 1);
    }

    #[test]
    fn remove_predecessor_matches_type_and_lag() {
        let predecessor = task();
        let other_predecessor = task();
        let successor = task();

        let mut container = container();
        container.add(relation(
            &predecessor,
            &successor,
            RelationType::FinishStart,
            Some(Duration::get_instance_int(1, TimeUnit::Days)),
        ));
        container.add(relation(
            &other_predecessor,
            &successor,
            RelationType::FinishStart,
            None,
        ));
        assert_eq!(container.len(), 2);

        // 类型不匹配 → 不删除
        assert!(!container.remove_predecessor(
            &successor.borrow(),
            &predecessor.borrow(),
            RelationType::StartStart,
            Some(Duration::get_instance_int(1, TimeUnit::Days)),
        ));
        assert_eq!(container.len(), 2);

        // lag 不匹配 → 不删除
        assert!(!container.remove_predecessor(
            &successor.borrow(),
            &predecessor.borrow(),
            RelationType::FinishStart,
            Some(Duration::get_instance_int(2, TimeUnit::Days)),
        ));
        assert_eq!(container.len(), 2);

        // 前驱任务不匹配 → 不删除
        assert!(!container.remove_predecessor(
            &successor.borrow(),
            &task().borrow(),
            RelationType::FinishStart,
            Some(Duration::get_instance_int(1, TimeUnit::Days)),
        ));
        assert_eq!(container.len(), 2);

        // 完全匹配 → 删除
        assert!(container.remove_predecessor(
            &successor.borrow(),
            &predecessor.borrow(),
            RelationType::FinishStart,
            Some(Duration::get_instance_int(1, TimeUnit::Days)),
        ));
        assert_eq!(container.len(), 1);
        assert_eq!(container.get_predecessors(&successor.borrow()).len(), 1);

        // lag 为 None 等价于 0 天
        assert!(container.remove_predecessor(
            &successor.borrow(),
            &other_predecessor.borrow(),
            RelationType::FinishStart,
            None,
        ));
        assert!(container.is_empty());
        assert!(container.get_successors(&predecessor.borrow()).is_empty());
    }

    #[test]
    fn remove_predecessor_on_task_without_relations_returns_false() {
        let mut container = container();
        assert!(!container.remove_predecessor(
            &task().borrow(),
            &task().borrow(),
            RelationType::FinishStart,
            None,
        ));
    }

    #[test]
    fn remove_uses_reference_identity_and_updates_index() {
        let predecessor = task();
        let successor = task();

        let mut container = container();
        let stored = relation(&predecessor, &successor, RelationType::FinishStart, None);
        container.add(Rc::clone(&stored));

        // 不在容器中的关系：返回 false
        let standalone = relation(&predecessor, &successor, RelationType::FinishStart, None);
        let removed_unknown = {
            let value = standalone.borrow();
            container.remove(&value)
        };
        assert!(!removed_unknown);
        assert_eq!(container.len(), 1);

        // 容器中的关系：按指针同一性删除，并清理索引
        let removed = {
            let value = stored.borrow();
            container.remove(&value)
        };
        assert!(removed);
        assert!(container.is_empty());
        assert!(container.get_predecessors(&successor.borrow()).is_empty());
    }

    #[test]
    fn set_replaces_and_reindexes() {
        let first_predecessor = task();
        let second_predecessor = task();
        let successor = task();

        let mut container = container();
        let original = relation(&first_predecessor, &successor, RelationType::FinishStart, None);
        container.add(Rc::clone(&original));

        let replacement = relation(&second_predecessor, &successor, RelationType::FinishFinish, None);
        let replaced = container.set(0, Rc::clone(&replacement)).expect("下标 0 存在");

        assert!(Rc::ptr_eq(&replaced, &original));
        assert!(Rc::ptr_eq(&container.get(0).expect("下标 0 存在"), &replacement));

        // Java: replaced(old, new) = removed(old) + added(new)
        assert!(container.get_successors(&first_predecessor.borrow()).is_empty());
        assert_eq!(container.get_successors(&second_predecessor.borrow()).len(), 1);
        assert_eq!(container.get_predecessors(&successor.borrow()).len(), 1);

        // 越界不修改容器
        assert!(container
            .set(5, relation(&first_predecessor, &successor, RelationType::FinishStart, None))
            .is_none());
        assert_eq!(container.len(), 1);
    }

    #[test]
    fn remove_at_cleans_up_index() {
        let predecessor = task();
        let successor = task();

        let mut container = container();
        container.add(relation(&predecessor, &successor, RelationType::FinishStart, None));

        assert!(container.remove_at(1).is_none());
        let removed = container.remove_at(0).expect("下标 0 存在");
        assert_eq!(removed.borrow().get_type(), RelationType::FinishStart);
        assert!(container.get_predecessors(&successor.borrow()).is_empty());
    }

    #[test]
    fn clear_drops_relations_and_index() {
        let predecessor = task();
        let successor = task();

        let mut container = container();
        container.add(relation(&predecessor, &successor, RelationType::FinishStart, None));
        container.clear();

        assert!(container.is_empty());
        assert!(container.get_predecessors(&successor.borrow()).is_empty());
        assert!(container.get_successors(&predecessor.borrow()).is_empty());
    }

    #[test]
    fn unique_id_lookup_and_clash_resolution() {
        let (mut container, project_file) = wired_container();

        let predecessor = task();
        let successor = task();

        let first = relation(&predecessor, &successor, RelationType::FinishStart, None);
        let second = relation(&predecessor, &successor, RelationType::StartStart, None);

        first.borrow_mut().set_unique_id(Some(4));
        second.borrow_mut().set_unique_id(Some(5));
        container.add(Rc::clone(&first));
        container.add(Rc::clone(&second));

        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(4)).expect("已登记"),
            &first
        ));
        assert!(container.get_by_unique_id(Some(6)).is_none());
        assert!(container.get_by_unique_id(None).is_none());
        assert_eq!(container.first_unique_id(), 1);

        // 把 second 的 unique ID 改成与 first 相同 → 进入冲突列表
        container.update_unique_id(&second, Some(5), Some(4));
        container.fix_unique_id_clashes();

        // 冲突者取序号生成器中的下一个值（序号从 1 开始）
        let renumbered = read_unique_id(&second).expect("仍持有 unique ID");
        assert_ne!(renumbered, 4);
        assert_eq!(read_unique_id(&first), Some(4));

        // 序号生成器位于父文件（ProjectFile 实现 UniqueIdObjectSequenceProvider）
        let next = UniqueIdObjectSequenceProvider::get_unique_id_object_sequence(
            &mut *project_file.borrow_mut(),
            TypeId::of::<Relation>(),
        )
        .get_next();
        assert!(next > renumbered);
    }

    /// Java: `Relation` 的构造方法在 `uniqueID == null` 且 `AutoRelationUniqueID` 为真
    /// （`ProjectConfig` 的默认值）时，向 `Relation` 序号生成器申请一个 unique ID。
    #[test]
    fn add_predecessor_allocates_unique_ids_when_configured() {
        let (mut container, project_file) = wired_container();

        let predecessor = task();
        let successor = task();

        let first = container.add_predecessor(
            RelationBuilder::new()
                .predecessor_task(&predecessor)
                .successor_task(&successor)
                .r#type(RelationType::FinishStart),
        );
        let second = container.add_predecessor(
            RelationBuilder::new()
                .predecessor_task(&predecessor)
                .successor_task(&successor)
                .r#type(RelationType::StartStart),
        );

        // 序号从 1 开始
        assert_eq!(read_unique_id(&first), Some(1));
        assert_eq!(read_unique_id(&second), Some(2));
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(1)).expect("已登记"),
            &first
        ));

        // Java: 分配动作经 `updateUniqueID` 完成，序号生成器同步到分配值
        let next = UniqueIdObjectSequenceProvider::get_unique_id_object_sequence(
            &mut *project_file.borrow_mut(),
            TypeId::of::<Relation>(),
        )
        .get_next();
        assert_eq!(next, 3);
    }

    /// builder 显式给出 unique ID 时沿用该值，并把它同步进序号生成器
    /// （Java: `updateUniqueID` 末尾的 `m_sequenceProvider...sync(newUniqueID)`）。
    #[test]
    fn add_predecessor_keeps_explicit_unique_id() {
        let (mut container, project_file) = wired_container();

        let predecessor = task();
        let successor = task();

        let explicit = container.add_predecessor(
            RelationBuilder::new()
                .predecessor_task(&predecessor)
                .successor_task(&successor)
                .unique_id(Some(5)),
        );
        assert_eq!(read_unique_id(&explicit), Some(5));

        let next = UniqueIdObjectSequenceProvider::get_unique_id_object_sequence(
            &mut *project_file.borrow_mut(),
            TypeId::of::<Relation>(),
        )
        .get_next();
        assert_eq!(next, 6);
    }

    /// `AutoRelationUniqueID = false` 时不分配（Java: 保持 `null`）。
    #[test]
    fn add_predecessor_without_auto_ids_leaves_unique_id_unset() {
        let (mut container, project_file) = wired_container();
        project_file
            .borrow()
            .get_project_context()
            .borrow_mut()
            .get_project_config_mut()
            .set_auto_relation_unique_id(false);

        let predecessor = task();
        let successor = task();

        let relation = container.add_predecessor(
            RelationBuilder::new()
                .predecessor_task(&predecessor)
                .successor_task(&successor),
        );

        assert_eq!(read_unique_id(&relation), None);
    }

    /// 关键回归：`Task::add_predecessor` 是
    /// `file.borrow_mut().get_relations_mut().add_predecessor(builder)` ——
    /// 调用期间 `ProjectFile` 处于**可变借用**状态，此时序号生成器只能经
    /// `ProjectContext`（独立 `RefCell`）取得，见文件头说明。
    #[test]
    fn add_predecessor_allocates_unique_ids_while_the_file_is_borrowed() {
        let project_file = ProjectFile::new();
        let predecessor = Rc::new(RefCell::new(Task::new_in_file(&project_file, None)));
        let successor = Rc::new(RefCell::new(Task::new_in_file(&project_file, None)));

        let relation = project_file
            .borrow_mut()
            .get_relations_mut()
            .add_predecessor(
                RelationBuilder::new()
                    .predecessor_task(&predecessor)
                    .successor_task(&successor),
            );

        assert_eq!(read_unique_id(&relation), Some(1));
        assert_eq!(project_file.borrow().get_relations().len(), 1);
    }

    #[test]
    fn unique_id_update_is_idempotent_for_same_element() {
        let (mut container, _) = wired_container();

        let predecessor = task();
        let successor = task();
        let relation = relation(&predecessor, &successor, RelationType::FinishStart, None);
        relation.borrow_mut().set_unique_id(Some(2));
        container.add(Rc::clone(&relation));

        // Java: currentElement == element 时不进入冲突列表
        container.update_unique_id(&relation, Some(2), Some(2));
        container.fix_unique_id_clashes();

        assert_eq!(read_unique_id(&relation), Some(2));
    }

    #[test]
    fn fix_unique_id_clashes_without_context_keeps_clash_list() {
        // 父文件未接线：Java 会因序号生成器不可用而抛异常，Rust 侧退化为不做任何事
        let mut container = container();

        let predecessor = task();
        let successor = task();
        let first = relation(&predecessor, &successor, RelationType::FinishStart, None);
        let second = relation(&predecessor, &successor, RelationType::StartStart, None);
        first.borrow_mut().set_unique_id(Some(1));
        second.borrow_mut().set_unique_id(Some(1));
        container.add(Rc::clone(&first));
        container.add(Rc::clone(&second));

        container.update_unique_id(&second, Some(1), Some(1));
        container.fix_unique_id_clashes();

        assert_eq!(read_unique_id(&second), Some(1));
    }

    #[test]
    fn default_matches_new_and_debug_reports_sizes() {
        let container = RelationContainer::default();
        assert!(container.is_empty());

        let text = format!("{container:?}");
        assert!(text.contains("size: 0"));
        assert!(text.contains("context_is_set: false"));
    }

    #[test]
    fn context_can_be_read_back() {
        let (container, project_file) = wired_container();
        assert!(Rc::ptr_eq(
            &container.get_context().expect("已接线"),
            &project_file
        ));
        assert!(RelationContainer::new().get_context().is_none());
    }
}
