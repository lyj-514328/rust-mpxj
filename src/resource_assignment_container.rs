//! 对应 Java: `org/mpxj/ResourceAssignmentContainer.java`
//!
//! 管理项目中的资源分配集合。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust |
//! |---|---|
//! | `extends ProjectEntityContainer<ResourceAssignment>` | 不组合 [`crate::project_entity_container::ProjectEntityContainer`]，改为内部 `Vec<Rc<RefCell<ResourceAssignment>>>` + 线性查找（见下） |
//! | `ResourceAssignmentContainer(ProjectFile projectFile)` | [`ResourceAssignmentContainer::new`] + [`ResourceAssignmentContainer::set_context`]（父文件由 [`crate::resource_assignment::ResourceAssignment`] 自行持有弱引用） |
//! | `@Override protected void added(ResourceAssignment assignment)` | 私有方法 `added`（由 [`ResourceAssignmentContainer::add`] 调用） |
//! | `@Override public void removed(ResourceAssignment assignment)` | 私有方法 `removed`（由 `remove` / `remove_by_unique_id` 调用） |
//! | `public Set<FieldType> getPopulatedFields()` | [`ResourceAssignmentContainer::get_populated_fields`]（由父文件传入默认值与自定义字段） |
//! | `public List<CustomField> getCustomFields()` | [`ResourceAssignmentContainer::get_custom_fields`]（经上下文的自定义字段容器） |
//! | `public FieldType getFieldTypeByAlias(String alias)` | [`ResourceAssignmentContainer::get_field_type_by_alias`]（返回 `Option<FieldKey>`） |
//! | `public List<ResourceAssignment> getByTaskUniqueID(Integer)` | [`ResourceAssignmentContainer::get_by_task_unique_id`] |
//! | `public List<ResourceAssignment> getByResourceUniqueID(Integer)` | [`ResourceAssignmentContainer::get_by_resource_unique_id`] |
//! | 继承自 `ProjectEntityContainer` 的 `getByUniqueID` / `add` / `size` / `isEmpty` / `iterator` / `get` | [`ResourceAssignmentContainer::get_by_unique_id`] / [`ResourceAssignmentContainer::add`] / [`ResourceAssignmentContainer::len`] / [`ResourceAssignmentContainer::is_empty`] / [`ResourceAssignmentContainer::iter`] / [`ResourceAssignmentContainer::get`] |
//! | 继承自 `ProjectEntityContainer` 的 `updateUniqueID(T element, Integer old, Integer new)` | [`ResourceAssignmentContainer::update_unique_id`]（Rust 无元素参数，见下） |
//! | 继承自 `ProjectEntityContainer` 的 `fixUniqueIdClashes` / `firstUniqueID` / `getSequenceProvider` | 未移植：见下「未组合基类」 |
//!
//! ## 未组合 `ProjectEntityContainer`
//!
//! Java 的 unique ID 索引（`m_uniqueIDMap`）与冲突列表来自基类。Rust 侧本容器改为
//! **按当前 unique ID 线性查找**：`ResourceAssignment::set_unique_id` 在更新自身字段后
//! 回调 [`ResourceAssignmentContainer::update_unique_id`] 同步序号生成器，而线性查找
//! 天然不会因 unique ID 变化而失效，因此无需维护索引与冲突列表，`fixUniqueIdClashes`
//! 一族方法也一并跳过（与
//! [`crate::project_calendar_container::ProjectCalendarContainer`] 的处理一致）。
//!
//! ## 其它差异
//!
//! * **元素形态**：Java 存 `ResourceAssignment` 引用；Rust 侧存
//!   `Rc<RefCell<ResourceAssignment>>`（符合「共享实体用 `Rc<RefCell<T>>`」的约定）。
//! * **`remove` 的元素参数**：Java 为 `ListWithCallbacks.remove(Object)`（引用相等）。
//!   Rust 侧接收 `&Rc<RefCell<ResourceAssignment>>`，用 [`Rc::ptr_eq`] 做指针同一性比较；
//!   另提供 [`ResourceAssignmentContainer::remove_by_unique_id`]（Rust 特有），供
//!   [`crate::resource_assignment::ResourceAssignment::remove`] 在拿不到自身 `Rc` 时使用。
//! * **上下文反向引用**：Java 字段 `m_projectFile` 是强引用；Rust 侧改为
//!   `Weak<RefCell<ProjectContext>>`，避免 `ProjectContext ↔ 容器` 形成 `Rc` 循环。
//! * **重入借用**：容器内部对上下文一律使用 `try_borrow_mut`，失败时退化为「不同步序号
//!   生成器」，不会 panic（Java 无此情形）。
//! * `getCustomFields` / `getFieldTypeByAlias` 在 Java 中经 `m_projectFile` 访问；Rust 侧
//!   经上下文（`ProjectFile.getCustomFields()` 在 Java 中即委托给上下文）访问。

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::{Rc, Weak};

use crate::custom_field::CustomField;
use crate::field_key::FieldKey;
use crate::field_type_class::FieldTypeClass;
use crate::project_context::ProjectContext;
use crate::resource_assignment::ResourceAssignment;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;

/// 项目中的资源分配集合。
///
/// 对应 Java: `public class ResourceAssignmentContainer extends ProjectEntityContainer<ResourceAssignment>`
pub struct ResourceAssignmentContainer {
    /// 容器内的资源分配。
    ///
    /// 对应 Java: `ProjectEntityContainer` / `ListWithCallbacks` 内部的列表存储。
    items: Vec<Rc<RefCell<ResourceAssignment>>>,

    /// 按资源 unique ID 建立的索引。
    ///
    /// 对应 Java 字段 `private final HashMap<Integer, List<ResourceAssignment>> m_resourceIndex = new HashMap<>();`
    resource_index: HashMap<i32, Vec<Rc<RefCell<ResourceAssignment>>>>,

    /// 按任务 unique ID 建立的索引。
    ///
    /// 对应 Java 字段 `private final HashMap<Integer, List<ResourceAssignment>> m_taskIndex = new HashMap<>();`
    task_index: HashMap<i32, Vec<Rc<RefCell<ResourceAssignment>>>>,

    /// 所属上下文（弱引用，避免 `Rc` 循环）。
    ///
    /// 对应 Java 字段 `private final ProjectFile m_projectFile;`（Rust 侧改持上下文，
    /// 见文件头说明）。
    context: Weak<RefCell<ProjectContext>>,
}

impl ResourceAssignmentContainer {
    /// 返回资源分配中至少有一个非默认缓存值的字段。
    ///
    /// 对应 Java: `public Set<FieldType> getPopulatedFields()`；项目默认值及自定义
    /// 字段由父文件调用者传入，以免在父文件已有借用时再次借用。
    pub fn get_populated_fields(
        &self,
        custom: impl IntoIterator<Item = FieldKey>,
        defaults: crate::common::populated_fields::Defaults,
    ) -> std::collections::HashSet<FieldKey> {
        crate::common::populated_fields::get_populated_fields(
            crate::common::populated_fields::merge_fields(
                crate::assignment_field::AssignmentField::ALL.map(FieldKey::Assignment), custom,
            ),
            self.iter().cloned(),
            defaults,
        )
    }

    /// 构造方法。
    ///
    /// 对应 Java: `public ResourceAssignmentContainer(ProjectFile projectFile)`
    ///
    /// Rust 侧先以空上下文弱引用构造，由 [`ResourceAssignmentContainer::set_context`] 接线。
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            resource_index: HashMap::new(),
            task_index: HashMap::new(),
            context: Weak::new(),
        }
    }

    /// 接线所属上下文（Rust 特有：Java 在构造方法中直接接收父文件）。
    ///
    /// 对应 Java: 构造方法中的 `m_projectFile = projectFile;`
    pub fn set_context(&mut self, context: &Rc<RefCell<ProjectContext>>) {
        self.context = Rc::downgrade(context);
    }

    /// 取所属上下文。
    ///
    /// Rust 特有访问器（Java 无对应方法，字段为父文件）。
    pub fn get_context(&self) -> Option<Rc<RefCell<ProjectContext>>> {
        self.context.upgrade()
    }

    /// 向容器加入一个资源分配。
    ///
    /// 对应 Java: `ListWithCallbacks.add(E element)`（进而触发
    /// `ProjectEntityContainer.added` 与 `ResourceAssignmentContainer.added`）
    pub fn add(&mut self, assignment: Rc<RefCell<ResourceAssignment>>) {
        self.added(&assignment);
        self.items.push(assignment);
    }

    /// 分配被加入后的钩子：更新任务/资源索引。
    ///
    /// 对应 Java: `@Override protected void added(ResourceAssignment assignment)`
    fn added(&mut self, assignment: &Rc<RefCell<ResourceAssignment>>) {
        // Java: super.added(assignment); —— 基类维护 unique ID 索引，
        // Rust 侧改为线性查找，没有需要维护的索引。

        let (task_unique_id, resource_unique_id) = match assignment.try_borrow() {
            Ok(assignment) => (
                assignment.get_task_unique_id(),
                assignment.get_resource_unique_id(),
            ),
            Err(_) => return,
        };

        if let Some(resource_unique_id) = resource_unique_id {
            self.resource_index
                .entry(resource_unique_id)
                .or_default()
                .push(Rc::clone(assignment));
        }

        if let Some(task_unique_id) = task_unique_id {
            self.task_index
                .entry(task_unique_id)
                .or_default()
                .push(Rc::clone(assignment));
        }
    }

    /// 按 `Rc` 指针同一性移除资源分配。
    ///
    /// 对应 Java: `ListWithCallbacks.remove(Object o)`（内部会调用 `removed(T element)`）
    ///
    /// Java 用引用相等定位元素，Rust 侧用 [`Rc::ptr_eq`] 表达同一语义。
    pub fn remove(&mut self, assignment: &Rc<RefCell<ResourceAssignment>>) -> bool {
        let index = self
            .items
            .iter()
            .position(|candidate| Rc::ptr_eq(candidate, assignment));

        let Some(index) = index else {
            return false;
        };

        let removed = self.items.remove(index);
        self.removed(&removed);
        true
    }

    /// 按 unique ID 移除资源分配（Rust 特有，见文件头说明）。
    ///
    /// 对应 Java: `ListWithCallbacks.remove(Object o)` 的一种退化实现
    /// （供 [`crate::resource_assignment::ResourceAssignment::remove`] 使用，
    /// unique ID 为 `None` 时不做任何事并返回 `false`）。
    pub fn remove_by_unique_id(&mut self, unique_id: Option<i32>) -> bool {
        let unique_id = match unique_id {
            Some(unique_id) => unique_id,
            None => return false,
        };

        let index = self
            .items
            .iter()
            .position(|candidate| read_unique_id(candidate) == Some(unique_id));

        let Some(index) = index else {
            return false;
        };

        let removed = self.items.remove(index);
        self.removed(&removed);
        true
    }

    /// 删除某个资源名下的全部资源分配（Rust 特有：供
    /// [`crate::resource_container::ResourceContainer`] 的 `removeAssignments` 调用）。
    ///
    /// 对应 Java: `ResourceContainer.removeAssignments(ProjectFile project, Resource resource)`
    /// 中对资源分配的遍历删除。返回被删除的条数。
    pub fn remove_by_resource_unique_id(&mut self, resource_unique_id: Option<i32>) -> usize {
        let Some(resource_unique_id) = resource_unique_id else {
            return 0;
        };

        let targets: Vec<Rc<RefCell<ResourceAssignment>>> = self
            .items
            .iter()
            .filter(|assignment| {
                assignment
                    .try_borrow()
                    .ok()
                    .and_then(|assignment| assignment.get_resource_unique_id())
                    == Some(resource_unique_id)
            })
            .cloned()
            .collect();

        let mut removed = 0;
        for target in targets {
            if self.remove(&target) {
                removed += 1;
            }
        }

        removed
    }

    /// 分配被移除后的钩子：从任务/资源索引中删除。
    ///
    /// 对应 Java: `@Override public void removed(ResourceAssignment assignment)`
    fn removed(&mut self, assignment: &Rc<RefCell<ResourceAssignment>>) {
        // Java: super.removed(assignment); —— 基类维护 unique ID 索引，
        // Rust 侧改为线性查找，没有需要维护的索引。

        let (task_unique_id, resource_unique_id) = match assignment.try_borrow() {
            Ok(assignment) => (
                assignment.get_task_unique_id(),
                assignment.get_resource_unique_id(),
            ),
            Err(_) => return,
        };

        if let Some(resource_unique_id) = resource_unique_id {
            remove_from_index(&mut self.resource_index, resource_unique_id, assignment);
        }

        if let Some(task_unique_id) = task_unique_id {
            remove_from_index(&mut self.task_index, task_unique_id, assignment);
        }
    }

    /// 更新 unique ID 变更后的状态。
    ///
    /// 对应 Java: `ProjectEntityContainer.updateUniqueID(T element, Integer oldUniqueID, Integer newUniqueID)`
    /// 的末尾一步 `m_sequenceProvider.getUniqueIdObjectSequence(element.getClass()).sync(newUniqueID)`
    ///
    /// Rust 侧容器按当前 unique ID 线性查找（无索引与冲突列表，见文件头），因此只需要把
    /// 序号生成器同步到新的 unique ID。调用时机见
    /// [`crate::resource_assignment::ResourceAssignment::set_unique_id`]。
    pub fn update_unique_id(&mut self, old_unique_id: Option<i32>, new_unique_id: Option<i32>) {
        let _ = old_unique_id;
        let Some(new_unique_id) = new_unique_id else {
            return;
        };

        // Java: m_sequenceProvider.getUniqueIdObjectSequence(element.getClass()).sync(newUniqueID);
        if let Some(context) = self.context.upgrade() {
            if let Ok(mut context) = context.try_borrow_mut() {
                context
                    .get_unique_id_object_sequence(TypeId::of::<ResourceAssignment>())
                    .sync(Some(new_unique_id));
            }
        }
    }

    /// 取资源分配的自定义字段。
    ///
    /// 对应 Java: `public List<CustomField> getCustomFields()`
    pub fn get_custom_fields(&self) -> Vec<Rc<RefCell<CustomField>>> {
        let Some(context) = self.context.upgrade() else {
            return Vec::new();
        };

        let custom_fields = match context.try_borrow() {
            Ok(context) => context.get_custom_fields(),
            Err(_) => return Vec::new(),
        };

        match custom_fields.try_borrow() {
            Ok(custom_fields) => {
                custom_fields.get_custom_fields_by_field_type_class(FieldTypeClass::Assignment)
            }
            Err(_) => Vec::new(),
        }
    }

    /// 按别名取字段标识。
    ///
    /// 对应 Java: `public FieldType getFieldTypeByAlias(String alias)`
    ///
    /// Rust 侧返回 `Option<FieldKey>`（找不到时为 `None`）。
    pub fn get_field_type_by_alias(&self, alias: &str) -> Option<FieldKey> {
        let context = self.context.upgrade()?;
        let custom_fields = match context.try_borrow() {
            Ok(context) => context.get_custom_fields(),
            Err(_) => return None,
        };

        match custom_fields.try_borrow() {
            Ok(custom_fields) => {
                custom_fields.get_field_type_by_alias(FieldTypeClass::Assignment, alias)
            }
            Err(_) => None,
        }
    }

    /// 按任务 unique ID 取资源分配；无匹配时返回空列表。
    ///
    /// 对应 Java: `public List<ResourceAssignment> getByTaskUniqueID(Integer taskUniqueID)`
    pub fn get_by_task_unique_id(&self, task_unique_id: Option<i32>) -> Vec<Rc<RefCell<ResourceAssignment>>> {
        match task_unique_id.and_then(|id| self.task_index.get(&id)) {
            Some(assignments) => assignments.clone(),
            None => Vec::new(),
        }
    }

    /// 按资源 unique ID 取资源分配；无匹配时返回空列表。
    ///
    /// 对应 Java: `public List<ResourceAssignment> getByResourceUniqueID(Integer resourceUniqueID)`
    pub fn get_by_resource_unique_id(
        &self,
        resource_unique_id: Option<i32>,
    ) -> Vec<Rc<RefCell<ResourceAssignment>>> {
        match resource_unique_id.and_then(|id| self.resource_index.get(&id)) {
            Some(assignments) => assignments.clone(),
            None => Vec::new(),
        }
    }

    /// 按 unique ID 查找资源分配，找不到时返回 `None`。
    ///
    /// 对应 Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<ResourceAssignment>>> {
        let id = id?;
        self.items
            .iter()
            .find(|assignment| read_unique_id(assignment) == Some(id))
            .map(Rc::clone)
    }

    /// 按下标取资源分配；越界返回 `None`。
    ///
    /// 对应 Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<Rc<RefCell<ResourceAssignment>>> {
        self.items.get(index).map(Rc::clone)
    }

    /// 资源分配个数。
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

    /// 迭代容器内的资源分配。
    ///
    /// 对应 Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<ResourceAssignment>>> {
        self.items.iter()
    }

    /// 以切片形式访问容器内的资源分配。
    ///
    /// 对应 Java: `ListWithCallbacks` 的列表视图。
    pub fn as_slice(&self) -> &[Rc<RefCell<ResourceAssignment>>] {
        self.items.as_slice()
    }
}

impl Default for ResourceAssignmentContainer {
    /// 对应 Java: 显式声明的 `ResourceAssignmentContainer(ProjectFile)`，
    /// Rust 侧等价于 [`ResourceAssignmentContainer::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for ResourceAssignmentContainer {
    /// Java 侧未覆写 `toString()`；Rust 侧输出容器规模、索引规模与上下文接线情况
    /// （[`crate::resource_assignment::ResourceAssignment`] 未实现 `Debug`，故不输出元素内容）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceAssignmentContainer")
            .field("size", &self.items.len())
            .field("task_index_size", &self.task_index.len())
            .field("resource_index_size", &self.resource_index.len())
            .field("context_is_set", &(self.context.strong_count() > 0))
            .finish()
    }
}

/// 按 `Rc` 指针同一性从索引中删除一项。
fn remove_from_index(
    index: &mut HashMap<i32, Vec<Rc<RefCell<ResourceAssignment>>>>,
    key: i32,
    assignment: &Rc<RefCell<ResourceAssignment>>,
) {
    if let Some(assignments) = index.get_mut(&key) {
        assignments.retain(|candidate| !Rc::ptr_eq(candidate, assignment));
    }
}

/// 读取资源分配的 unique ID，已被借用时返回 `None`。
fn read_unique_id(assignment: &Rc<RefCell<ResourceAssignment>>) -> Option<i32> {
    assignment
        .try_borrow()
        .ok()
        .and_then(|assignment| assignment.get_unique_id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duration::Duration;
    use crate::time_unit::TimeUnit;

    /// 构造一个带上下文接线、已设置 unique ID 的资源分配。
    fn assignment(
        container: &mut ResourceAssignmentContainer,
        unique_id: i32,
        task_unique_id: i32,
        resource_unique_id: i32,
    ) -> Rc<RefCell<ResourceAssignment>> {
        let mut item = ResourceAssignment::new();
        item.set_unique_id(Some(unique_id));
        item.set_task_unique_id(Some(task_unique_id));
        item.set_resource_unique_id(Some(resource_unique_id));

        let item = Rc::new(RefCell::new(item));
        container.add(Rc::clone(&item));
        item
    }

    #[test]
    fn container_basics() {
        let mut container = ResourceAssignmentContainer::new();
        assert!(container.is_empty());

        assignment(&mut container, 1, 10, 20);
        assignment(&mut container, 2, 10, 21);

        assert_eq!(container.len(), 2);
        assert!(!container.is_empty());
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert!(container.get(0).is_some());
        assert!(container.get(2).is_none());
    }

    #[test]
    fn lookup_by_unique_id_and_indices() {
        let mut container = ResourceAssignmentContainer::new();
        let first = assignment(&mut container, 1, 10, 20);
        let second = assignment(&mut container, 2, 10, 21);

        assert!(container
            .get_by_unique_id(Some(1))
            .map(|found| Rc::ptr_eq(&found, &first))
            .unwrap_or(false));
        assert!(container.get_by_unique_id(Some(3)).is_none());
        assert!(container.get_by_unique_id(None).is_none());

        // 按任务 unique ID 取到两条
        let by_task = container.get_by_task_unique_id(Some(10));
        assert_eq!(by_task.len(), 2);

        // 按资源 unique ID 各取到一条
        let by_resource = container.get_by_resource_unique_id(Some(20));
        assert_eq!(by_resource.len(), 1);
        assert!(Rc::ptr_eq(&by_resource[0], &first));
        assert_eq!(container.get_by_resource_unique_id(Some(21)).len(), 1);
        assert!(container.get_by_resource_unique_id(Some(99)).is_empty());
        assert!(container.get_by_task_unique_id(None).is_empty());

        // second 未被资源索引命中
        assert!(container
            .get_by_resource_unique_id(Some(21))
            .first()
            .map(|found| Rc::ptr_eq(found, &second))
            .unwrap_or(false));
    }

    #[test]
    fn remove_updates_indices() {
        let mut container = ResourceAssignmentContainer::new();
        let first = assignment(&mut container, 1, 10, 20);
        let second = assignment(&mut container, 2, 10, 20);

        assert_eq!(container.get_by_task_unique_id(Some(10)).len(), 2);
        assert_eq!(container.get_by_resource_unique_id(Some(20)).len(), 2);

        assert!(container.remove(&first));
        assert_eq!(container.len(), 1);
        assert_eq!(container.get_by_task_unique_id(Some(10)).len(), 1);
        assert_eq!(container.get_by_resource_unique_id(Some(20)).len(), 1);
        assert!(container.get_by_unique_id(Some(1)).is_none());

        // 再次移除同一元素返回 false
        assert!(!container.remove(&first));

        // 用 Rc 指针同一性定位：另一个 Rc 指向同一对象时也应命中
        assert!(container.remove(&Rc::clone(&second)));
        assert!(container.is_empty());
        assert!(container.get_by_resource_unique_id(Some(20)).is_empty());
    }

    #[test]
    fn remove_by_unique_id_drops_element() {
        let mut container = ResourceAssignmentContainer::new();
        assignment(&mut container, 1, 10, 20);

        assert!(!container.remove_by_unique_id(None));
        assert!(!container.remove_by_unique_id(Some(2)));
        assert!(container.remove_by_unique_id(Some(1)));
        assert!(container.is_empty());
    }

    #[test]
    fn resource_assignment_has_no_parent_file() {
        // 未接线的分配无法自我移除（Java 的 remove() 依赖父文件）
        let mut assignment = ResourceAssignment::new();
        assignment.set_unique_id(Some(1));
        assignment.set_work(Some(Duration::get_instance(1.0, TimeUnit::Hours)));

        assert!(!assignment.remove());
    }

    #[test]
    fn context_wiring_and_sequence_sync() {
        let context = ProjectContext::shared();
        let mut container = ResourceAssignmentContainer::new();
        container.set_context(&context);

        assert!(container.get_context().is_some());
        assert!(container.get_by_unique_id(Some(1)).is_none());
        assert!(container.get_custom_fields().is_empty());
        assert!(container.get_field_type_by_alias("alias").is_none());

        container.update_unique_id(Some(1), Some(20));

        let next = context
            .borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<ResourceAssignment>())
            .get_next();
        assert_eq!(next, 21);
    }

    #[test]
    fn update_unique_id_without_context_is_a_no_op() {
        let mut container = ResourceAssignmentContainer::new();
        container.update_unique_id(Some(1), Some(2));
        assert!(container.get_context().is_none());
    }

    #[test]
    fn default_matches_new() {
        let container = ResourceAssignmentContainer::default();
        assert!(container.is_empty());
    }

    #[test]
    fn debug_reports_size_and_context_state() {
        let mut container = ResourceAssignmentContainer::new();
        assert!(format!("{container:?}").contains("context_is_set: false"));

        assignment(&mut container, 1, 10, 20);
        let text = format!("{container:?}");
        assert!(text.contains("size: 1"));
        assert!(text.contains("task_index_size: 1"));
        assert!(text.contains("resource_index_size: 1"));
    }
}
