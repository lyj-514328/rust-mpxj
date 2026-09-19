//! 对应 Java: `org/mpxj/ProjectEntityWithIDContainer.java`

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use crate::common::object_sequence::ObjectSequence;
use crate::project_entity_container::ProjectEntityContainer;
use crate::project_entity_with_id::ProjectEntityWithID;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;

/// 在 [`ProjectEntityContainer`] 之上增加「ID → 实体」索引的容器。
///
/// 对应 Java:
/// `public abstract class ProjectEntityWithIDContainer<T extends ProjectEntityWithID & Comparable<T>> extends ProjectEntityContainer<T>`
///
/// # 继承体系的还原方式：组合 + 转发
///
/// Rust 没有继承，无法让本类「是」[`ProjectEntityContainer`]。这里采用**组合 + 转发**：
/// 内部持有一个 [`ProjectEntityContainer<T>`]，把基类的公开行为逐个转发（见各方法说明），
/// 再叠加本类新增的 ID 索引与重编号逻辑。
///
/// | Java | Rust |
/// |---|---|
/// | `extends ProjectEntityContainer<T>` | 字段 `container: ProjectEntityContainer<T>` |
/// | `private final ObjectSequence m_idSequence = new ObjectSequence(1)` | 字段 `id_sequence: ObjectSequence` |
/// | `protected final Map<Integer, T> m_idMap` | 字段 `id_map: HashMap<i32, Rc<RefCell<T>>>` |
///
/// 之所以选择组合而不是「用一个 trait 提供基类行为」，是因为 Java 中基类的
/// `added` / `removed` 钩子与 unique ID 索引是**状态**，用 trait 默认方法无法持有这些状态
/// （trait 无法有字段）；组合则能完整保留行为与状态。
///
/// # 与 Java 的差异
///
/// - **元素引用模型**：与基类一致，元素为 `Rc<RefCell<T>>`。
/// - **`Comparable<T>`**：Java 用 `Collections.sort(this)` 依据元素自身的自然序排序；Rust 侧
///   约束为 `T: Ord`，对应元素实现的 `compareTo`。
/// - **`null` 键**：`m_idMap` 在 Java 中允许 `null` 键，Rust 侧键为 `i32`，不提供 `null` 键
///   （等价于「查不到」）。
/// - **`NumberHelper.getInt(null)`**：Java 对 `null` 的 ID 取 0；Rust 侧用 `unwrap_or(0)` 等价表达。
///
/// 被跳过的 Java 机制：无。
///
/// 未移植的 Java 方法：无。
pub struct ProjectEntityWithIDContainer<T> {
    /// 基类容器（Java 的 `extends ProjectEntityContainer<T>`）。
    container: ProjectEntityContainer<T>,
    /// ID 序号生成器，从 1 开始。
    ///
    /// 对应 Java: `private final ObjectSequence m_idSequence = new ObjectSequence(1);`
    id_sequence: ObjectSequence,
    /// ID → 实体索引，由调用方通过
    /// [`map_id`](ProjectEntityWithIDContainer::map_id) 维护。
    ///
    /// 对应 Java: `protected final Map<Integer, T> m_idMap = new HashMap<>();`
    id_map: HashMap<i32, Rc<RefCell<T>>>,
}

impl<T: ProjectEntityWithID + Ord + 'static> ProjectEntityWithIDContainer<T> {
    /// 构造方法。
    ///
    /// 对应 Java: `public ProjectEntityWithIDContainer(UniqueIdObjectSequenceProvider projectFile)`
    pub fn new(sequence_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>) -> Self {
        Self {
            container: ProjectEntityContainer::new(sequence_provider),
            id_sequence: ObjectSequence::new(1),
            id_map: HashMap::new(),
        }
    }

    /// 让所有实体的 ID 连续排列。
    ///
    /// 对应 Java: `public void renumberIDs()`
    ///
    /// 先按自然序排序，再依次赋 ID：如果排序后第一个实体的 ID 非 0 则从 1 开始，
    /// 否则从 0 开始（与 Java 一致）。
    pub fn renumber_ids(&mut self) {
        if self.container.is_empty() {
            return;
        }

        // Java: `Collections.sort(this)`
        self.container.sort();

        // Java: `int id = NumberHelper.getInt(firstEntity.getID()); if (id != 0) { id = 1; }`
        let first_id = self
            .container
            .get(0)
            .and_then(|entity| entity.borrow().get_id())
            .unwrap_or(0);
        let mut id = if first_id != 0 { 1 } else { 0 };

        for entity in self.container.iter() {
            entity.borrow_mut().set_id(Some(id));
            id += 1;
        }
    }

    /// 按 ID 查找实体，找不到时返回 `None`。
    ///
    /// 对应 Java: `public T getByID(Integer id)`
    pub fn get_by_id(&self, id: Option<i32>) -> Option<Rc<RefCell<T>>> {
        let id = id?;
        self.id_map.get(&id).cloned()
    }

    /// 删除一条 ID 映射。
    ///
    /// 对应 Java: `public void unmapID(Integer id)`
    pub fn unmap_id(&mut self, id: Option<i32>) {
        if let Some(id) = id {
            self.id_map.remove(&id);
        }
    }

    /// 添加一条 ID 映射。
    ///
    /// 对应 Java: `public void mapID(Integer id, T entity)`
    pub fn map_id(&mut self, id: i32, entity: Rc<RefCell<T>>) {
        self.id_map.insert(id, entity);
    }

    /// 取下一个可用 ID。
    ///
    /// 对应 Java: `public Integer getNextID()`
    pub fn get_next_id(&self) -> i32 {
        self.id_sequence.get_next()
    }

    /// 访问内部的基类容器。
    ///
    /// 对应 Java 的 `extends ProjectEntityContainer<T>` 关系；Rust 侧用组合表达，
    /// 因此需要显式暴露基类实例。
    pub fn container(&self) -> &ProjectEntityContainer<T> {
        &self.container
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

    /// 按 unique ID 查找实体。
    ///
    /// 对应 Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<T>>> {
        self.container.get_by_unique_id(id)
    }

    /// 追加实体。
    ///
    /// 对应 Java: `ListWithCallbacks.add(T element)`
    pub fn add(&mut self, element: Rc<RefCell<T>>) {
        self.container.add(element);
    }

    /// 在指定下标插入实体（越界返回 `false`）。
    ///
    /// 对应 Java: `ListWithCallbacks.add(int index, T element)`
    pub fn add_at(&mut self, index: usize, element: Rc<RefCell<T>>) -> bool {
        self.container.add_at(index, element)
    }

    /// 替换指定下标的实体（越界返回 `None`）。
    ///
    /// 对应 Java: `ListWithCallbacks.set(int index, T element)`
    pub fn set(&mut self, index: usize, element: Rc<RefCell<T>>) -> Option<Rc<RefCell<T>>> {
        self.container.set(index, element)
    }

    /// 移除指定下标的实体（越界返回 `None`）。
    ///
    /// 对应 Java: `ListWithCallbacks.remove(int index)`
    pub fn remove(&mut self, index: usize) -> Option<Rc<RefCell<T>>> {
        self.container.remove(index)
    }

    /// 清空实体列表（不触发 `removed` 回调，与 Java 一致）。
    ///
    /// 对应 Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.container.clear();
    }

    /// 按下标取实体。
    ///
    /// 对应 Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<Rc<RefCell<T>>> {
        self.container.get(index)
    }

    /// 实体个数。
    ///
    /// 对应 Java: `ListWithCallbacks.size()`
    pub fn len(&self) -> usize {
        self.container.len()
    }

    /// 实体列表是否为空。
    ///
    /// 对应 Java: `ListWithCallbacks.isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.container.is_empty()
    }

    /// 迭代实体列表。
    ///
    /// 对应 Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<T>>> {
        self.container.iter()
    }

    /// 以切片形式访问实体列表。
    ///
    /// 对应 Java: `ListWithCallbacks` 的列表视图。
    pub fn as_slice(&self) -> &[Rc<RefCell<T>>] {
        self.container.as_slice()
    }

    /// 按元素自然序排序。
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
        element: &Rc<RefCell<T>>,
        old_unique_id: Option<i32>,
        new_unique_id: Option<i32>,
    ) {
        self.container
            .update_unique_id(element, old_unique_id, new_unique_id);
    }

    /// 为所有 unique ID 冲突的实体重新分配 unique ID。
    ///
    /// 对应 Java: `ProjectEntityContainer.fixUniqueIdClashes()`
    pub fn fix_unique_id_clashes(&mut self) {
        self.container.fix_unique_id_clashes();
    }
}

impl<T: Debug> Debug for ProjectEntityWithIDContainer<T> {
    /// Java 侧未覆写 `toString()`；Rust 侧输出内部容器与 ID 索引。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProjectEntityWithIDContainer")
            .field("container", &self.container)
            .field("id_map", &self.id_map)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
    use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
    use std::any::TypeId;

    /// 测试用实体：同时具备 ID 与 unique ID。
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct Entity {
        id: Option<i32>,
        unique_id: Option<i32>,
    }

    impl ProjectEntityWithUniqueID for Entity {
        fn get_unique_id(&self) -> Option<i32> {
            self.unique_id
        }
    }

    impl ProjectEntityWithMutableUniqueID for Entity {
        fn set_unique_id(&mut self, id: Option<i32>) {
            self.unique_id = id;
        }
    }

    impl ProjectEntityWithID for Entity {
        fn get_id(&self) -> Option<i32> {
            self.id
        }

        fn set_id(&mut self, id: Option<i32>) {
            self.id = id;
        }
    }

    /// 测试用序号生成器提供者。
    #[derive(Default)]
    struct TestProvider {
        sequences: HashMap<TypeId, Rc<ObjectSequence>>,
    }

    impl UniqueIdObjectSequenceProvider for TestProvider {
        fn get_unique_id_object_sequence(&mut self, type_id: TypeId) -> Rc<ObjectSequence> {
            Rc::clone(
                self.sequences
                    .entry(type_id)
                    .or_insert_with(|| Rc::new(ObjectSequence::new(1))),
            )
        }
    }

    /// 构造一个容器。
    fn container() -> ProjectEntityWithIDContainer<Entity> {
        let provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            Rc::new(RefCell::new(TestProvider::default()));
        ProjectEntityWithIDContainer::new(provider)
    }

    /// 构造一个实体。
    fn entity(id: i32, unique_id: i32) -> Rc<RefCell<Entity>> {
        Rc::new(RefCell::new(Entity {
            id: Some(id),
            unique_id: Some(unique_id),
        }))
    }

    #[test]
    fn base_container_behaviour_is_forwarded() {
        let mut container = container();
        assert!(container.is_empty());

        container.add(entity(1, 10));
        assert_eq!(container.len(), 1);
        assert_eq!(container.iter().count(), 1);
        assert_eq!(container.as_slice().len(), 1);
        assert!(container.get(0).is_some());
        assert!(container.get(1).is_none());
        assert!(container.get_by_unique_id(Some(10)).is_some());
        assert!(container.get_by_id(Some(1)).is_none());
        assert_eq!(container.first_unique_id(), 1);
    }

    #[test]
    fn id_map_can_be_populated_and_removed() {
        let mut container = container();
        let first = entity(1, 10);
        container.add(Rc::clone(&first));

        container.map_id(1, Rc::clone(&first));
        assert!(Rc::ptr_eq(
            &container.get_by_id(Some(1)).expect("已映射"),
            &first
        ));

        container.unmap_id(Some(1));
        assert!(container.get_by_id(Some(1)).is_none());
        assert!(container.get_by_id(None).is_none());
    }

    #[test]
    fn get_next_id_starts_at_one() {
        let container = container();

        assert_eq!(container.get_next_id(), 1);
        assert_eq!(container.get_next_id(), 2);
        assert_eq!(container.get_next_id(), 3);
    }

    #[test]
    fn renumber_ids_fills_gaps_from_one() {
        let mut container = container();
        container.add(entity(5, 50));
        container.add(entity(1, 10));
        container.add(entity(3, 30));

        container.renumber_ids();

        // 排序后最小 ID 为 1（非 0），因此从 1 开始连续编号
        let ids: Vec<Option<i32>> = container
            .iter()
            .map(|entity| entity.borrow().get_id())
            .collect();
        assert_eq!(ids, vec![Some(1), Some(2), Some(3)]);
    }

    #[test]
    fn renumber_ids_starts_at_zero_when_first_id_is_zero() {
        let mut container = container();
        container.add(entity(0, 0));
        container.add(entity(4, 40));

        container.renumber_ids();

        let ids: Vec<Option<i32>> = container
            .iter()
            .map(|entity| entity.borrow().get_id())
            .collect();
        assert_eq!(ids, vec![Some(0), Some(1)]);
    }

    #[test]
    fn renumber_ids_on_empty_container_is_a_no_op() {
        let mut container = container();
        container.renumber_ids();
        assert!(container.is_empty());
    }

    #[test]
    fn unique_id_clash_handling_is_forwarded() {
        let mut container = container();
        let first = entity(1, 10);
        let second = entity(2, 10);
        container.add(Rc::clone(&first));
        container.add(Rc::clone(&second));

        container.fix_unique_id_clashes();

        // 冲突者取序列中的下一个值
        assert_eq!(second.borrow().get_unique_id(), Some(1));
        assert_eq!(first.borrow().get_unique_id(), Some(10));
    }
}
