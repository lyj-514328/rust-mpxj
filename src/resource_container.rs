//! 对应 Java: `org/mpxj/ResourceContainer.java`
//!
//! # 继承体系如何复用（组合 + 转发）
//!
//! Java 的 `ResourceContainer extends ProjectEntityWithIDContainer<Resource> implements ChildResourceContainer`。
//! Rust 没有继承，本类按约定**组合**一个
//! [`ProjectEntityWithIDContainer<Resource>`]，并把基类的公开行为逐个转发（见各方法说明），
//! 只有本类新增的行为（资源创建、子资源列表、层级重建、移除清理、自定义字段查询）单独实现。
//! 这与 [`crate::project_calendar_container`]「未组合基类、按 unique ID 线性查找」的做法不同：
//! 该文件因 `ProjectCalendar` 未实现 `ProjectEntityWithUniqueID` 而无法组合，而
//! [`crate::resource::Resource`] 已实现 `ProjectEntityWithID` 与 `Ord`，可以完整复用基类，
//! 故这里以「组合 + 转发」为准。
//!
//! # 与 Java 的差异
//!
//! - **构造方法**：Java 为 `ResourceContainer(ProjectContext context)`。Rust 侧
//!   [`ResourceContainer::new`] 无参数（[`crate::project_context::ProjectContext::new`] 调用它时
//!   上下文尚未构建），再由 [`ResourceContainer::set_context`] 接线；
//!   `ProjectContext::shared()` 目前只接线日历容器与序号提供者适配器，
//!   **接线时需要在其中补一次 `resources.set_context(&context)`**（本文件不修改
//!   `project_context.rs`，接线点即 [`crate::project_context::ProjectContext::shared`]）。
//! - **序号生成器提供者**：Java 把 `ProjectContext` 自身（实现了
//!   `UniqueIdObjectSequenceProvider`）交给基类。Rust 侧若强引用上下文会形成 `Rc` 循环，
//!   故本文件内定义只持弱引用的 `ResourceSequenceProvider`（与 `project_context.rs` 内
//!   的 `ContextSequenceProvider` 同构）：上下文可借用时转发给它，否则退化为自带序号，
//!   保证不 panic。
//! - **上下文反向引用**：`private final ProjectContext m_context` → `Weak<RefCell<ProjectContext>>`。
//! - **元素形态**：`Resource` 为 `Rc<RefCell<Resource>>`（共享实体约定）。
//! - **重载（Rust 无重载）**：
//!
//!   | Java | Rust |
//!   |---|---|
//!   | `public Resource add()` | [`ResourceContainer::add`] |
//!   | `@Override public Resource addResource()` | [`ResourceContainer::add_resource`] |
//!   | `ListWithCallbacks.add(T element)`（继承） | [`ResourceContainer::add_resource_entry`]（加后缀区分同名的无参 `add()`） |
//!   | `ListWithCallbacks.remove(Object)` + `removed(T)` | [`ResourceContainer::remove_resource`]（接收元素句柄） |
//!   | `ListWithCallbacks.remove(int index)` | [`ResourceContainer::remove`] |
//! - **`add` 中的自动编号**：Java 的 `new Resource(m_context)` 在构造方法内分配 unique ID / ID；
//!   Rust 侧因 `&mut self` 无法取得自身 `Rc`（Java 用 `mapID(id, this)`），改由
//!   [`ResourceContainer::add`] 在包装 `Rc` 并接线自身弱引用后分配（见
//!   [`crate::resource`] 文件头说明）。自动编号开关取自 `ProjectConfig`；上下文不可用时
//!   按 Java 默认值（均为 `true`）处理。
//! - **`removeAssignments`**：Java 用 `Iterator.remove()` 逐个删除匹配的资源分配。
//!   Rust 侧调用 `ResourceAssignmentContainer` 的按资源 unique ID 删除方法
//!   （见下「接口假设」）。
//!
//! # 跳过的 Java 方法
//!
//! | Java | 原因 |
//! |---|---|
//! | `public Set<FieldType> getPopulatedFields()` | 依赖未在 `src/lib.rs` 声明的 `common::PopulatedFields` |
//!
//! 被跳过的 Java 机制：`Serializable`（本类未使用）。
//!
//! # 对相关模块的接口假设
//!
//! - `crate::project_file::ProjectFile`：`get_resource_assignments()` →
//!   `&ResourceAssignmentContainer`，`get_resource_assignments_mut()` →
//!   `&mut ResourceAssignmentContainer`。
//! - `crate::resource_assignment_container::ResourceAssignmentContainer`：
//!   `remove_by_resource_unique_id(Option<i32>)`（对应 Java `removeAssignments` 中的遍历删除）。

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::{Rc, Weak};

use crate::common::object_sequence::ObjectSequence;
use crate::custom_field::CustomField;
use crate::field_key::FieldKey;
use crate::field_type_class::FieldTypeClass;
use crate::project_context::ProjectContext;
use crate::project_entity_with_id::ProjectEntityWithID;
use crate::project_entity_with_id_container::ProjectEntityWithIDContainer;
use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
use crate::project_file::ProjectFile;
use crate::resource::Resource;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;

/// 管理项目中的资源集合。
///
/// 对应 Java: `public class ResourceContainer extends ProjectEntityWithIDContainer<Resource> implements ChildResourceContainer`
pub struct ResourceContainer {
    /// 基类容器（Java 的 `extends ProjectEntityWithIDContainer<Resource>`）。
    container: ProjectEntityWithIDContainer<Resource>,

    /// 父上下文（弱引用，避免 `Rc` 循环）。
    ///
    /// 对应 Java: `private final ProjectContext m_context;`
    context: Weak<RefCell<ProjectContext>>,

    /// 交给基类的序号生成器提供者（见文件头说明，Java 直接传 `this`）。
    sequence_provider: Rc<RefCell<ResourceSequenceProvider>>,

    /// 顶层子资源列表。
    ///
    /// 对应 Java: `private final List<Resource> m_childResources = new ArrayList<>();`
    child_resources: Vec<Rc<RefCell<Resource>>>,
}

impl ResourceContainer {
    /// 构造方法。
    ///
    /// 对应 Java: `public ResourceContainer(ProjectContext context)`
    ///
    /// Rust 侧无参数构造，由 [`ResourceContainer::set_context`] 接线上下文。
    pub fn new() -> Self {
        let sequence_provider = Rc::new(RefCell::new(ResourceSequenceProvider::new()));
        let base_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            sequence_provider.clone();

        Self {
            container: ProjectEntityWithIDContainer::new(base_provider),
            context: Weak::new(),
            sequence_provider,
            child_resources: Vec::new(),
        }
    }

    /// 接线父上下文（Rust 特有：Java 在构造方法中直接接收 `this`）。
    ///
    /// 对应 Java: 构造方法中的 `m_context = context;`，以及基类构造方法的
    /// `super(context)`（序号生成器提供者）。
    pub(crate) fn set_context(&mut self, context: &Rc<RefCell<ProjectContext>>) {
        self.context = Rc::downgrade(context);
        self.sequence_provider.borrow_mut().set_context(context);
    }

    /// 取父上下文（Rust 特有：Java 没有该 getter）。
    pub fn get_context(&self) -> Option<Rc<RefCell<ProjectContext>>> {
        self.context.upgrade()
    }

    /// 取顶层子资源列表（Java 返回实时列表，Rust 返回快照）。
    ///
    /// 对应 Java: `@Override public List<Resource> getChildResources()`
    pub fn get_child_resources(&self) -> Vec<Rc<RefCell<Resource>>> {
        self.child_resources.clone()
    }

    /// 新建一个资源并加入容器。
    ///
    /// 对应 Java: `@Override public Resource addResource()`
    pub fn add_resource(&mut self) -> Rc<RefCell<Resource>> {
        self.add()
    }

    /// 新建一个资源并加入容器。
    ///
    /// 对应 Java: `public Resource add()`
    pub fn add(&mut self) -> Rc<RefCell<Resource>> {
        let resource = Rc::new(RefCell::new(Resource::new(self.context.clone())));
        resource.borrow_mut().set_self_reference(&resource);

        let (auto_unique_id, auto_id) = self.auto_id_flags();

        if auto_unique_id {
            let next_unique_id = self
                .container
                .get_sequence_provider()
                .borrow_mut()
                .get_unique_id_object_sequence(TypeId::of::<Resource>())
                .get_next();
            resource.borrow_mut().set_unique_id(Some(next_unique_id));
        }

        if auto_id {
            let next_id = self.container.get_next_id();
            resource.borrow_mut().set_id(Some(next_id));
            self.map_id(next_id, Rc::clone(&resource));
        }

        self.container.add(Rc::clone(&resource));
        self.child_resources.push(Rc::clone(&resource));
        resource
    }

    /// 把已有资源加入基类容器（Java 继承自 `ListWithCallbacks` 的 `add(T element)`）。
    ///
    /// Rust 侧无重载，故与同名的无参 [`ResourceContainer::add`] 以 `_entry` 后缀区分。
    pub fn add_resource_entry(&mut self, resource: Rc<RefCell<Resource>>) {
        self.container.add(resource);
    }

    /// 从容器移除一个资源，并执行移除后的清理。
    ///
    /// 对应 Java: `ListWithCallbacks.remove(Object)`（内部调用 `@Override protected void removed(Resource)`）
    ///
    /// Java 用引用相等定位元素；Rust 侧用 `Rc::ptr_eq` 做指针同一性比较。
    pub fn remove_resource(&mut self, resource: &Rc<RefCell<Resource>>) -> bool {
        let index = self
            .container
            .iter()
            .position(|candidate| Rc::ptr_eq(candidate, resource));

        let Some(index) = index else {
            return false;
        };

        // Java: super.removed(resource) —— 基类的 removed 回调在 remove(index) 中触发。
        self.container.remove(index);
        self.removed(resource);
        true
    }

    /// 资源被移除后的清理。
    ///
    /// 对应 Java: `@Override protected void removed(Resource resource)`
    fn removed(&mut self, resource: &Rc<RefCell<Resource>>) {
        // Java: m_idMap.remove(resource.getID());
        self.unmap_id(resource.borrow().get_id());

        // Java:
        // Resource parentResource = resource.getParentResource();
        // if (parentResource != null) { parentResource.removeChildResource(resource); }
        // else { m_childResources.remove(resource); }
        //
        // Java 的 getParentResource() 走上下文的资源容器；Rust 侧改为走本容器自身的
        // unique ID 索引：容器在多数调用场景下正处于被可变借用状态（调用方持有上下文的
        // `borrow_mut`），此时再借上下文会失败（见文件头说明）。
        let parent_unique_id = resource.borrow().get_parent_resource_unique_id();
        let parent = self.get_by_unique_id(parent_unique_id);
        match parent {
            Some(parent) => {
                if let Ok(mut parent_ref) = parent.try_borrow_mut() {
                    parent_ref.remove_child_resource(resource);
                }
            }
            None => self.child_resources.retain(|item| !Rc::ptr_eq(item, resource)),
        }

        // Java: m_context.getProjects().forEach(p -> removeAssignments(p, resource));
        let projects = match self.context.upgrade() {
            Some(context) => match context.try_borrow() {
                Ok(context_ref) => context_ref.get_projects(),
                Err(_) => Vec::new(),
            },
            None => Vec::new(),
        };
        for project in projects {
            self.remove_assignments(&project, resource);
        }

        // Java:
        // ProjectCalendar calendar = resource.getCalendar();
        // if (calendar != null) { calendar.remove(); }
        let calendar = resource.borrow().get_calendar();
        if let Some(calendar) = calendar {
            if let Ok(mut calendar_ref) = calendar.try_borrow_mut() {
                calendar_ref.remove();
            }
        }
    }

    /// 删除某个项目下属于指定资源的全部资源分配。
    ///
    /// 对应 Java: `private void removeAssignments(ProjectFile project, Resource resource)`
    fn remove_assignments(
        &mut self,
        project: &Rc<RefCell<ProjectFile>>,
        resource: &Rc<RefCell<Resource>>,
    ) {
        let resource_unique_id = resource.borrow().get_unique_id();
        if let Ok(mut project_ref) = project.try_borrow_mut() {
            project_ref
                .get_resource_assignments_mut()
                .remove_by_resource_unique_id(resource_unique_id);
        }
    }

    /// 依据父资源 ID 重建资源层级结构。
    ///
    /// 对应 Java: `void updateStructure()`
    pub fn update_structure(&mut self) {
        if self.len() <= 1 {
            return;
        }

        let resources: Vec<Rc<RefCell<Resource>>> = self.container.iter().cloned().collect();

        // Java: m_childResources.clear(); this.forEach(r -> r.getChildResources().clear());
        self.child_resources.clear();
        for resource in &resources {
            resource.borrow_mut().get_child_resources_mut().clear();
        }

        // Java: this.forEach(r -> { ... });
        for resource in &resources {
            // Java 用 r.getParentResource()（走上下文的资源容器）；Rust 侧走本容器自身的
            // unique ID 索引，理由同 `removed`（避免在上下文被可变借用时再借上下文）。
            let parent_unique_id = resource.borrow().get_parent_resource_unique_id();
            let parent = self.get_by_unique_id(parent_unique_id);
            match parent {
                None => self.child_resources.push(Rc::clone(resource)),
                Some(parent) => {
                    if let Ok(mut parent_ref) = parent.try_borrow_mut() {
                        parent_ref.add_child_resource(resource);
                    }
                }
            }
        }
    }

    /// 取某一实体类型下的全部自定义字段配置。
    ///
    /// 对应 Java: `public List<CustomField> getCustomFields()`
    ///
    /// 上下文不可用时返回空列表（Java 不会出现该分支）。
    pub fn get_custom_fields(&self) -> Vec<Rc<RefCell<CustomField>>> {
        match self.context.upgrade() {
            Some(context) => match context.try_borrow() {
                Ok(context_ref) => context_ref
                    .get_custom_fields()
                    .borrow()
                    .get_custom_fields_by_field_type_class(FieldTypeClass::Resource),
                Err(_) => Vec::new(),
            },
            None => Vec::new(),
        }
    }

    /// 按别名取字段类型。
    ///
    /// 对应 Java: `public FieldType getFieldTypeByAlias(String alias)`
    pub fn get_field_type_by_alias(&self, alias: &str) -> Option<FieldKey> {
        let context = self.context.upgrade()?;
        let context_ref = context.try_borrow().ok()?;
        context_ref
            .get_custom_fields()
            .borrow()
            .get_field_type_by_alias(FieldTypeClass::Resource, alias)
    }

    /// 读取自动编号开关（`ProjectConfig.getAutoResourceUniqueID` / `getAutoResourceID`）。
    ///
    /// 上下文不可用时按 Java 的默认值（均为 `true`）处理。
    fn auto_id_flags(&self) -> (bool, bool) {
        match self.context.upgrade() {
            Some(context) => match context.try_borrow() {
                Ok(context_ref) => {
                    let config = context_ref.get_project_config();
                    (
                        config.get_auto_resource_unique_id(),
                        config.get_auto_resource_id(),
                    )
                }
                Err(_) => (true, true),
            },
            None => (true, true),
        }
    }

    /// 取序号生成器提供者。
    ///
    /// 对应 Java: `ProjectEntityContainer.getSequenceProvider()`
    pub fn get_sequence_provider(&self) -> Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> {
        self.container.get_sequence_provider()
    }

    /// 重排 unique ID 时使用的第一个值。
    ///
    /// 对应 Java: `ProjectEntityContainer.firstUniqueID()`
    pub fn first_unique_id(&self) -> i32 {
        self.container.first_unique_id()
    }

    /// 按 unique ID 查找资源，找不到时返回 `None`。
    ///
    /// 对应 Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Resource>>> {
        self.container.get_by_unique_id(id)
    }

    /// 按 ID 查找资源，找不到时返回 `None`。
    ///
    /// 对应 Java: `ProjectEntityWithIDContainer.getByID(Integer id)`
    pub fn get_by_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Resource>>> {
        self.container.get_by_id(id)
    }

    /// 删除一条 ID 映射。
    ///
    /// 对应 Java: `ProjectEntityWithIDContainer.unmapID(Integer id)`
    pub fn unmap_id(&mut self, id: Option<i32>) {
        self.container.unmap_id(id);
    }

    /// 添加一条 ID 映射。
    ///
    /// 对应 Java: `ProjectEntityWithIDContainer.mapID(Integer id, T entity)`
    pub fn map_id(&mut self, id: i32, resource: Rc<RefCell<Resource>>) {
        self.container.map_id(id, resource);
    }

    /// 取下一个可用 ID。
    ///
    /// 对应 Java: `ProjectEntityWithIDContainer.getNextID()`
    pub fn get_next_id(&self) -> i32 {
        self.container.get_next_id()
    }

    /// 在指定下标插入资源（越界返回 `false`）。
    ///
    /// 对应 Java: `ListWithCallbacks.add(int index, T element)`
    pub fn add_at(&mut self, index: usize, resource: Rc<RefCell<Resource>>) -> bool {
        self.container.add_at(index, resource)
    }

    /// 替换指定下标的资源（越界返回 `None`）。
    ///
    /// 对应 Java: `ListWithCallbacks.set(int index, T element)`
    pub fn set(&mut self, index: usize, resource: Rc<RefCell<Resource>>) -> Option<Rc<RefCell<Resource>>> {
        self.container.set(index, resource)
    }

    /// 移除指定下标的资源（越界返回 `None`）。
    ///
    /// 对应 Java: `ListWithCallbacks.remove(int index)`
    pub fn remove(&mut self, index: usize) -> Option<Rc<RefCell<Resource>>> {
        self.container.remove(index)
    }

    /// 清空资源列表（不触发 `removed` 回调，与 Java 一致）。
    ///
    /// 对应 Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.container.clear();
    }

    /// 按下标取资源。
    ///
    /// 对应 Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<Rc<RefCell<Resource>>> {
        self.container.get(index)
    }

    /// 资源个数。
    ///
    /// 对应 Java: `ListWithCallbacks.size()`
    pub fn len(&self) -> usize {
        self.container.len()
    }

    /// 资源列表是否为空。
    ///
    /// 对应 Java: `ListWithCallbacks.isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.container.is_empty()
    }

    /// 迭代资源列表。
    ///
    /// 对应 Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<Resource>>> {
        self.container.iter()
    }

    /// 以切片形式访问资源列表。
    ///
    /// 对应 Java: `ListWithCallbacks` 的列表视图。
    pub fn as_slice(&self) -> &[Rc<RefCell<Resource>>] {
        self.container.as_slice()
    }

    /// 按资源自身的顺序（`compareTo`，即 ID）排序。
    ///
    /// 对应 Java: `Collections.sort(this)`
    pub fn sort(&mut self) {
        self.container.sort();
    }

    /// 更新 unique ID 索引。
    ///
    /// 对应 Java: `ProjectEntityContainer.updateUniqueID(T element, Integer oldUniqueID, Integer newUniqueID)`
    pub fn update_unique_id(
        &mut self,
        resource: &Rc<RefCell<Resource>>,
        old_unique_id: Option<i32>,
        new_unique_id: Option<i32>,
    ) {
        self.container
            .update_unique_id(resource, old_unique_id, new_unique_id);
    }

    /// 为所有 unique ID 冲突的资源重新分配 unique ID。
    ///
    /// 对应 Java: `ProjectEntityContainer.fixUniqueIdClashes()`
    pub fn fix_unique_id_clashes(&mut self) {
        self.container.fix_unique_id_clashes();
    }

    /// 让所有资源的 ID 连续排列。
    ///
    /// 对应 Java: `ProjectEntityWithIDContainer.renumberIDs()`
    pub fn renumber_ids(&mut self) {
        self.container.renumber_ids();
    }

    /// 访问内部的基类容器。
    ///
    /// 对应 Java 的 `extends ProjectEntityWithIDContainer<Resource>` 关系；Rust 侧用组合表达，
    /// 因此需要显式暴露基类实例。
    pub fn container(&self) -> &ProjectEntityWithIDContainer<Resource> {
        &self.container
    }
}

impl Default for ResourceContainer {
    /// 对应 Java: 显式声明的 `ResourceContainer(ProjectContext)`，
    /// Rust 侧等价于 [`ResourceContainer::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for ResourceContainer {
    /// Java 侧未覆写 `toString()`；Rust 侧输出容器规模、子资源数与上下文接线情况。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceContainer")
            .field("container", &self.container)
            .field("child_resources", &self.child_resources.len())
            .field("context_is_set", &(self.context.strong_count() > 0))
            .finish()
    }
}

/// 供 [`ResourceContainer`] 使用的序号生成器提供者（Rust 特有，见文件头说明）。
///
/// 与 [`crate::project_context`] 内部的 `ContextSequenceProvider` 同构：优先转发给上下文，
/// 上下文不可用（未接线或已被借用）时退化为自带序号，保证不 panic。
struct ResourceSequenceProvider {
    /// 目标上下文（弱引用，避免 `Rc` 循环）。
    context: Weak<RefCell<ProjectContext>>,
    /// 上下文不可用时的退化序号生成器。
    fallback: HashMap<TypeId, Rc<ObjectSequence>>,
}

impl ResourceSequenceProvider {
    fn new() -> Self {
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

impl UniqueIdObjectSequenceProvider for ResourceSequenceProvider {
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

    /// 构造一个已接线上下文的容器。
    fn container() -> (Rc<RefCell<ProjectContext>>, ResourceContainer) {
        let context = ProjectContext::shared();
        let mut container = ResourceContainer::new();
        container.set_context(&context);
        (context, container)
    }

    #[test]
    fn container_basics() {
        let (_, mut container) = container();
        assert!(container.is_empty());
        assert_eq!(container.first_unique_id(), 1);

        container.add();
        container.add();

        assert_eq!(container.len(), 2);
        assert!(!container.is_empty());
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert!(container.get(0).is_some());
        assert!(container.get(2).is_none());
    }

    #[test]
    fn add_assigns_ids_and_registers_child_resource() {
        let (_, mut container) = container();

        let resource = container.add();

        let unique_id = resource.borrow().get_unique_id();
        let id = resource.borrow().get_id();
        assert!(unique_id.is_some());
        assert!(id.is_some());
        assert_eq!(container.len(), 1);
        assert_eq!(container.get_child_resources().len(), 1);

        // unique ID / ID 索引都可查找
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(unique_id).expect("unique ID 已索引"),
            &resource
        ));
        assert!(Rc::ptr_eq(
            &container.get_by_id(id).expect("ID 已映射"),
            &resource
        ));

        // addResource() 与 add() 等价
        let second = container.add_resource();
        assert_eq!(container.len(), 2);
        assert_ne!(
            second.borrow().get_unique_id(),
            unique_id,
            "自动编号不应重复"
        );
    }

    #[test]
    fn add_resource_entry_registers_existing_resource() {
        let (context, mut container) = container();

        let resource = Rc::new(RefCell::new(Resource::new(Rc::downgrade(&context))));
        resource.borrow_mut().set_unique_id(Some(9));

        container.add_resource_entry(Rc::clone(&resource));

        assert_eq!(container.len(), 1);
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(9)).expect("已索引"),
            &resource
        ));
        assert!(container.get_child_resources().is_empty());
    }

    #[test]
    fn remove_resource_drops_indexes_and_child_entry() {
        let (_, mut container) = container();
        let resource = container.add();
        let id = resource.borrow().get_id();

        assert!(container.remove_resource(&resource));
        assert!(container.is_empty());
        assert!(container.get_by_id(id).is_none());
        assert!(container.get_child_resources().is_empty());

        // 再次移除返回 false
        assert!(!container.remove_resource(&resource));
    }

    #[test]
    fn remove_by_index_is_forwarded() {
        let (_, mut container) = container();
        container.add();
        container.add();

        let removed = container.remove(0).expect("下标 0 存在");
        assert!(removed.borrow().get_id().is_some());
        assert_eq!(container.len(), 1);
        assert!(container.remove(5).is_none());
    }

    #[test]
    fn update_structure_rebuilds_hierarchy_from_parent_ids() {
        let context = ProjectContext::shared();
        let mut container = ResourceContainer::new();
        container.set_context(&context);

        let parent = Rc::new(RefCell::new(Resource::new(Rc::downgrade(&context))));
        let child = Rc::new(RefCell::new(Resource::new(Rc::downgrade(&context))));
        let orphan = Rc::new(RefCell::new(Resource::new(Rc::downgrade(&context))));

        parent.borrow_mut().set_unique_id(Some(1));
        child.borrow_mut().set_unique_id(Some(2));
        orphan.borrow_mut().set_unique_id(Some(3));
        child.borrow_mut().set_parent_resource(Some(&parent));

        container.add_resource_entry(Rc::clone(&parent));
        container.add_resource_entry(Rc::clone(&child));
        container.add_resource_entry(Rc::clone(&orphan));

        container.update_structure();

        // 顶层：parent 与 orphan（child 挂在 parent 下）
        let top_level = container.get_child_resources();
        assert_eq!(top_level.len(), 2);
        assert_eq!(parent.borrow().get_child_resources().len(), 1);
        assert!(Rc::ptr_eq(
            &parent.borrow().get_child_resources()[0],
            &child
        ));
    }

    #[test]
    fn unique_id_clashes_are_detected_and_fixed() {
        let context = ProjectContext::shared();
        let mut container = ResourceContainer::new();
        container.set_context(&context);

        let first = Rc::new(RefCell::new(Resource::new(Rc::downgrade(&context))));
        let second = Rc::new(RefCell::new(Resource::new(Rc::downgrade(&context))));
        first.borrow_mut().set_unique_id(Some(1));
        second.borrow_mut().set_unique_id(Some(2));

        container.add_resource_entry(Rc::clone(&first));
        container.add_resource_entry(Rc::clone(&second));

        // 制造冲突：second 改用与 first 相同的 unique ID（同时把序号推进到 2）
        container.update_unique_id(&second, Some(2), Some(1));
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(1)).expect("已索引"),
            &second
        ));

        container.fix_unique_id_clashes();

        // 冲突者被重新编号
        assert_eq!(second.borrow().get_unique_id(), Some(2));
        assert_eq!(first.borrow().get_unique_id(), Some(1));
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(2)).expect("已索引"),
            &second
        ));
    }

    #[test]
    fn map_and_unmap_id_are_forwarded() {
        let (context, mut container) = container();
        let resource = Rc::new(RefCell::new(Resource::new(Rc::downgrade(&context))));

        container.map_id(7, Rc::clone(&resource));
        assert!(Rc::ptr_eq(
            &container.get_by_id(Some(7)).expect("已映射"),
            &resource
        ));

        container.unmap_id(Some(7));
        assert!(container.get_by_id(Some(7)).is_none());
        assert!(container.get_by_id(None).is_none());
    }

    #[test]
    fn renumber_ids_is_forwarded() {
        let (context, mut container) = container();
        let first = Rc::new(RefCell::new(Resource::new(Rc::downgrade(&context))));
        let second = Rc::new(RefCell::new(Resource::new(Rc::downgrade(&context))));
        first.borrow_mut().set_id(Some(5));
        second.borrow_mut().set_id(Some(9));

        container.add_resource_entry(Rc::clone(&first));
        container.add_resource_entry(Rc::clone(&second));
        container.renumber_ids();

        let ids: Vec<Option<i32>> = container
            .iter()
            .map(|resource| resource.borrow().get_id())
            .collect();
        assert_eq!(ids, vec![Some(1), Some(2)]);
    }

    #[test]
    fn context_backed_queries_degrade_without_context() {
        let mut container = ResourceContainer::new();

        assert!(container.get_context().is_none());
        assert!(container.get_custom_fields().is_empty());
        assert!(container.get_field_type_by_alias("alias").is_none());

        // 未接线上下文时仍可创建资源（使用退化序号生成器与 Java 默认自动编号）
        let resource = container.add();
        assert!(resource.borrow().get_unique_id().is_some());
        assert!(resource.borrow().get_id().is_some());
    }

    #[test]
    fn context_backed_queries_work_when_wired() {
        let (_context, container) = container();

        assert!(container.get_context().is_some());
        // 空的自定义字段容器
        assert!(container.get_custom_fields().is_empty());
        assert!(container.get_field_type_by_alias("missing").is_none());
    }

    #[test]
    fn default_matches_new() {
        let container = ResourceContainer::default();
        assert!(container.is_empty());
    }

    #[test]
    fn debug_reports_size_and_context_state() {
        let mut container = ResourceContainer::new();
        assert!(format!("{container:?}").contains("context_is_set: false"));

        container.add();
        let text = format!("{container:?}");
        assert!(text.contains("ResourceContainer"));
    }
}
