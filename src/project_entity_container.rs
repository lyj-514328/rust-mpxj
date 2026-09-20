//! 对应 Java: `org/mpxj/ProjectEntityContainer.java`
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `removed(T)` 通过当前 unique ID 删除索引 | 按 `Rc` 身份删除索引 | 日历调用 `remove()` 时可能仍被可变借用，不能反向读取其 unique ID；索引被其它同号元素覆盖时保留该元素 |
//! | `fixUniqueIdClashes()` 从提供者取序列 | 另有传入真实序列的重载 | 容器在 `ProjectContext` 可变借用期间无法经适配器再次借用上下文 |
//!
//! # 跳过的 Java 成员 + 原因
//!
//! 无。

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use crate::common::object_sequence::ObjectSequence;
use crate::list_with_callbacks::ListWithCallbacks;
use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;

/// 项目实体容器的公共实现：存储、迭代与按 unique ID 查找。
///
/// 对应 Java:
/// `public abstract class ProjectEntityContainer<T extends ProjectEntityWithUniqueID> extends ListWithCallbacks<T>`
///
/// # 继承体系的还原方式
///
/// Java 用「继承 `ListWithCallbacks` + 覆写 `added` / `removed`」实现本类。Rust 没有继承，
/// 本类改为**组合 + 回调字段**：
///
/// | Java | Rust |
/// |---|---|
/// | `extends ListWithCallbacks<T>` | 内部持有 [`ListWithCallbacks<Rc<RefCell<T>>>`] 字段 |
/// | `protected void added(T element)` 覆写 | [`ListWithCallbacks::set_added_callback`] 注册的闭包 |
/// | `protected void removed(T element)` 覆写 | [`ListWithCallbacks::set_removed_callback`] 注册的闭包 |
/// | `private final Map<Integer, T> m_uniqueIDMap` | `Rc<RefCell<HashMap<i32, Rc<RefCell<T>>>>>`，与回调闭包共享 |
/// | `private final List<ProjectEntityWithMutableUniqueID> m_uniqueIDClashList` | `Rc<RefCell<Vec<Rc<RefCell<T>>>>>` |
///
/// 后续的 `ProjectEntityWithIDContainer` 等子类**无法**在 Rust 中继承本类，改为
/// 组合一个本类实例并转发方法（见
/// [`crate::project_entity_with_id_container::ProjectEntityWithIDContainer`]）。
///
/// # 与 Java 的差异
///
/// - **元素引用模型**：Java 的列表与映射共享同一批对象引用，`fixUniqueIdClashes` 等
///   方法就地修改元素。Rust 侧用 `Rc<RefCell<T>>` 表达这种共享与内部可变性（符合
///   「共享实体用 `Rc<RefCell<T>>`」的约定），因此 `get` / `iter` 返回的是 `Rc` 句柄。
/// - **元素类型约束**：Java 声明 `T extends ProjectEntityWithUniqueID`，但
///   `fixUniqueIdClashes` 内部用 `instanceof` 判断元素是否可变。Rust 侧结构体本身不带
///   约束，[`fix_unique_id_clashes`](ProjectEntityContainer::fix_unique_id_clashes) 用
///   **方法级** `where T: ProjectEntityWithMutableUniqueID` 等价地表达这一检查：对
///   unique ID 不可变的实体（如 `WorkContour`），冲突列表永远为空，该方法也不会被调用。
/// - **`null` 键**：Java 的 `Map<Integer, T>` 允许 `null` 键（`added` / `removed` 对
///   无 unique ID 的元素会使用它）。Rust 侧映射键为 `i32`，此时直接跳过（等价于「查不到」）。
/// - **`element.getClass()`**：Java 用元素的运行时类型取序号生成器；Rust 侧用
///   `TypeId::of::<T>()`（容器元素类型即元素类型）。
/// - **`firstUniqueID()`**：Java 为 `protected` 且可被 `TaskContainer` 覆写；Rust 侧提供
///   返回基类默认值 1 的公开方法 [`first_unique_id`](ProjectEntityContainer::first_unique_id)。
///
/// 被跳过的 Java 机制：无（`Serializable` 等未在本类中使用）。
///
/// 未移植的 Java 方法：无。
pub struct ProjectEntityContainer<T> {
    /// 列表内容与增减回调。
    ///
    /// 对应 Java: `ListWithCallbacks<T>` 基类继承而来的列表存储与 `added` / `removed` 钩子。
    list: ListWithCallbacks<Rc<RefCell<T>>>,
    /// 序号生成器提供者。
    ///
    /// 对应 Java: `protected final UniqueIdObjectSequenceProvider m_sequenceProvider;`
    sequence_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>,
    /// unique ID → 实体的索引。
    ///
    /// 对应 Java: `private final Map<Integer, T> m_uniqueIDMap = new HashMap<>();`
    unique_id_map: Rc<RefCell<HashMap<i32, Rc<RefCell<T>>>>>,
    /// 出现 unique ID 冲突的实体列表。
    ///
    /// 对应 Java: `private final List<ProjectEntityWithMutableUniqueID> m_uniqueIDClashList = new ArrayList<>();`
    unique_id_clash_list: Rc<RefCell<Vec<Rc<RefCell<T>>>>>,
}

impl<T: ProjectEntityWithUniqueID + 'static> ProjectEntityContainer<T> {
    /// 构造方法。
    ///
    /// 对应 Java: `public ProjectEntityContainer(UniqueIdObjectSequenceProvider sequenceProvider)`
    ///
    /// 构造时会把 `added` / `removed` 回调注册进内部列表，与 Java 中由子类覆写这两个钩子
    /// 的效果一致。
    pub fn new(sequence_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>) -> Self {
        let unique_id_map: Rc<RefCell<HashMap<i32, Rc<RefCell<T>>>>> =
            Rc::new(RefCell::new(HashMap::new()));
        let unique_id_clash_list: Rc<RefCell<Vec<Rc<RefCell<T>>>>> =
            Rc::new(RefCell::new(Vec::new()));
        let mut list: ListWithCallbacks<Rc<RefCell<T>>> = ListWithCallbacks::new();

        // Java: `@Override protected void added(T element)`
        {
            let map = Rc::clone(&unique_id_map);
            let clash_list = Rc::clone(&unique_id_clash_list);
            list.set_added_callback(Some(Box::new(move |element: &Rc<RefCell<T>>| {
                let unique_id = match element.try_borrow().ok().and_then(|element| element.get_unique_id()) {
                    Some(unique_id) => unique_id,
                    None => return,
                };

                let current_element = map.try_borrow().ok()
                    .and_then(|map| map.get(&unique_id).cloned());
                if let Some(current_element) = current_element {
                    // Java: 同一个实例重复加入不视为冲突
                    if Rc::ptr_eq(&current_element, element) {
                        return;
                    }

                    // Java: `if (currentElement instanceof ProjectEntityWithMutableUniqueID)`
                    // 记录冲突候选；对 unique ID 不可变的元素类型，冲突列表不会被消费
                    // （见 `fix_unique_id_clashes` 的方法级约束）。
                    if let Ok(mut clashes) = clash_list.try_borrow_mut() {
                        clashes.push(Rc::clone(element));
                    }
                }

                if let Ok(mut map) = map.try_borrow_mut() {
                    map.insert(unique_id, Rc::clone(element));
                }
            })));
        }

        // Java: `@Override protected void removed(T element)`
        {
            let map = Rc::clone(&unique_id_map);
            list.set_removed_callback(Some(Box::new(move |element: &Rc<RefCell<T>>| {
                // 日历可在其 `remove()` 持有可变借用时触发此回调。
                // 直接按元素身份移除，无需重入借用实体。
                if let Ok(mut map) = map.try_borrow_mut() {
                    map.retain(|_, candidate| !Rc::ptr_eq(candidate, element));
                }
            })));
        }

        Self {
            list,
            sequence_provider,
            unique_id_map,
            unique_id_clash_list,
        }
    }

    /// 取序号生成器提供者。
    ///
    /// 对应 Java: `public UniqueIdObjectSequenceProvider getSequenceProvider()`
    pub fn get_sequence_provider(&self) -> Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> {
        Rc::clone(&self.sequence_provider)
    }

    /// 重排 unique ID 时使用的第一个值。
    ///
    /// 对应 Java: `protected int firstUniqueID()`
    ///
    /// Java 侧被 `TaskContainer` 覆写（任务 0 存在时从 0 开始）；Rust 侧由各容器自行决定，
    /// 本方法提供基类默认值。
    pub fn first_unique_id(&self) -> i32 {
        1
    }

    /// 按 unique ID 查找实体，找不到时返回 `None`。
    ///
    /// 对应 Java: `public T getByUniqueID(Integer id)`
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<T>>> {
        let id = id?;
        self.unique_id_map.borrow().get(&id).cloned()
    }

    /// 追加实体。
    ///
    /// 对应 Java: `ListWithCallbacks.add(T element)`（经 `added` 钩子更新 unique ID 索引）。
    pub fn add(&mut self, element: Rc<RefCell<T>>) {
        self.list.add(element);
    }

    /// 在指定下标插入实体。
    ///
    /// 对应 Java: `ListWithCallbacks.add(int index, T element)`
    ///
    /// 越界返回 `false`。
    pub fn add_at(&mut self, index: usize, element: Rc<RefCell<T>>) -> bool {
        self.list.add_at(index, element)
    }

    /// 替换指定下标的实体，返回被替换掉的旧实体。
    ///
    /// 对应 Java: `ListWithCallbacks.set(int index, T element)`
    ///
    /// 越界返回 `None`。
    ///
    /// 与 Java 一致：替换**不会**更新 unique ID 索引（Java 的
    /// `ProjectEntityContainer` 只覆写了 `added` / `removed` 两个钩子，没有覆写
    /// `replaced`），需要同步索引时由调用方显式使用
    /// [`update_unique_id`](ProjectEntityContainer::update_unique_id)。
    pub fn set(&mut self, index: usize, element: Rc<RefCell<T>>) -> Option<Rc<RefCell<T>>> {
        self.list.set(index, element)
    }

    /// 移除指定下标的实体并返回它。
    ///
    /// 对应 Java: `ListWithCallbacks.remove(int index)`
    ///
    /// 越界返回 `None`。
    pub fn remove(&mut self, index: usize) -> Option<Rc<RefCell<T>>> {
        self.list.remove(index)
    }

    /// 清空容器（不触发 `removed` 回调，与 Java 一致）。
    ///
    /// 对应 Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.list.clear();
    }

    /// 按下标取实体。
    ///
    /// 对应 Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<Rc<RefCell<T>>> {
        self.list.get(index).cloned()
    }

    /// 实体个数。
    ///
    /// 对应 Java: `ListWithCallbacks.size()`
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// 容器是否为空。
    ///
    /// 对应 Java: `ListWithCallbacks.isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// 迭代容器内的实体。
    ///
    /// 对应 Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<T>>> {
        self.list.iter()
    }

    /// 以切片形式访问容器内的实体。
    ///
    /// 对应 Java: `ListWithCallbacks` 的列表视图。
    pub fn as_slice(&self) -> &[Rc<RefCell<T>>] {
        self.list.as_slice()
    }

    /// 按实体自身的顺序就地排序（不触发回调，见 [`ListWithCallbacks`] 的说明）。
    ///
    /// 对应 Java: `ProjectEntityWithIDContainer.renumberIDs()` 中的 `Collections.sort(this)`
    pub fn sort(&mut self)
    where
        T: Ord,
    {
        self.list.sort();
    }

    /// 更新 unique ID 索引：元素换用新的 unique ID 时调用。
    ///
    /// 对应 Java: `public void updateUniqueID(T element, Integer oldUniqueID, Integer newUniqueID)`
    ///
    /// Java 侧 `newUniqueID` 为 `null` 时会写入 `null` 键；Rust 侧的 `i32` 键无法表示，
    /// 此时只移除旧键、不建立新键（等价于「查不到」）。
    pub fn update_unique_id(
        &mut self,
        element: &Rc<RefCell<T>>,
        old_unique_id: Option<i32>,
        new_unique_id: Option<i32>,
    ) {
        if let Some(old_unique_id) = old_unique_id {
            self.unique_id_map.borrow_mut().remove(&old_unique_id);
        }

        let new_unique_id = match new_unique_id {
            Some(new_unique_id) => new_unique_id,
            None => return,
        };

        let current_element = self.unique_id_map.borrow().get(&new_unique_id).cloned();
        if let Some(current_element) = current_element {
            if Rc::ptr_eq(&current_element, element) {
                return;
            }

            self.unique_id_clash_list
                .borrow_mut()
                .push(Rc::clone(element));
        }

        self.unique_id_map
            .borrow_mut()
            .insert(new_unique_id, Rc::clone(element));

        self.sequence_provider
            .borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<T>())
            .sync(Some(new_unique_id));
    }

    /// 为所有出现 unique ID 冲突的实体重新分配 unique ID。
    ///
    /// 对应 Java: `public void fixUniqueIdClashes()`
    ///
    /// 只有 unique ID 可变的实体才可能产生冲突，因此本方法带
    /// `where T: ProjectEntityWithMutableUniqueID` 约束 —— 这等价于 Java 中通过
    /// `instanceof ProjectEntityWithMutableUniqueID` 做的运行时判断。
    pub fn fix_unique_id_clashes(&mut self)
    where
        T: ProjectEntityWithMutableUniqueID,
    {
        let sequence = self
            .sequence_provider
            .borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<T>());
        self.fix_unique_id_clashes_with_sequence(sequence);
    }

    /// 使用父上下文已取得的序列修复冲突。
    ///
    /// 对应 Java: `fixUniqueIdClashes()`；Rust 的日历容器在持有上下文可变借用时
    /// 无法让序号提供者再次借用同一上下文，故由调用者传入真实序列。
    pub fn fix_unique_id_clashes_with_sequence(&mut self, sequence: Rc<ObjectSequence>)
    where
        T: ProjectEntityWithMutableUniqueID,
    {
        let clash_list: Vec<Rc<RefCell<T>>> = self.unique_id_clash_list.borrow().clone();
        if clash_list.is_empty() {
            return;
        }

        for element in &clash_list {
            let next = sequence.get_next();
            element.borrow_mut().set_unique_id(Some(next));
        }

        self.unique_id_clash_list.borrow_mut().clear();

        let mut map = self.unique_id_map.borrow_mut();
        map.clear();
        for element in self.list.iter() {
            if let Some(unique_id) = element.borrow().get_unique_id() {
                map.insert(unique_id, Rc::clone(element));
            }
        }
    }

    /// 从当前元素重建 unique ID 索引与冲突列表。
    ///
    /// 对应 Java: `added(T)` 的逐元素索引行为；Rust 的日历 setter 遇到上下文
    /// 重入借用时无法回调容器，修复冲突前需补齐那段索引变化。
    pub fn rebuild_unique_id_index(&mut self) {
        let (Ok(mut map), Ok(mut clashes)) = (
            self.unique_id_map.try_borrow_mut(),
            self.unique_id_clash_list.try_borrow_mut(),
        ) else {
            return;
        };
        map.clear();
        clashes.clear();
        for item in self.list.iter() {
            let Ok(item_ref) = item.try_borrow() else {
                continue;
            };
            let Some(id) = item_ref.get_unique_id() else {
                continue;
            };
            if map.contains_key(&id) {
                clashes.push(Rc::clone(item));
            }
            map.insert(id, Rc::clone(item));
        }
    }
}

impl<T: Debug> Debug for ProjectEntityContainer<T> {
    /// Java 侧未覆写 `toString()`；Rust 侧输出列表与 unique ID 索引，忽略无法 `Debug` 的
    /// 序号生成器提供者。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProjectEntityContainer")
            .field("list", &self.list)
            .field("unique_id_map", &self.unique_id_map.borrow())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::object_sequence::ObjectSequence;
    use std::collections::HashMap as StdHashMap;

    /// 测试用实体：unique ID 可读可写。
    #[derive(Debug, PartialEq, Eq)]
    struct Entity {
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

    /// 测试用序号生成器提供者，行为与 Java 的 `ProjectContext` 一致。
    #[derive(Default)]
    struct TestProvider {
        sequences: StdHashMap<TypeId, Rc<ObjectSequence>>,
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

    /// 构造一个带测试用提供者的容器。
    fn container() -> (
        ProjectEntityContainer<Entity>,
        Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>,
    ) {
        let provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            Rc::new(RefCell::new(TestProvider::default()));
        (ProjectEntityContainer::new(Rc::clone(&provider)), provider)
    }

    /// 构造一个实体。
    fn entity(unique_id: Option<i32>) -> Rc<RefCell<Entity>> {
        Rc::new(RefCell::new(Entity { unique_id }))
    }

    #[test]
    fn add_indexes_by_unique_id() {
        let (mut container, _) = container();
        assert!(container.is_empty());

        let first = entity(Some(5));
        let second = entity(Some(6));
        container.add(Rc::clone(&first));
        container.add(Rc::clone(&second));

        assert_eq!(container.len(), 2);
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(5)).expect("已索引"),
            &first
        ));
        assert!(container.get_by_unique_id(Some(7)).is_none());
        assert!(container.get_by_unique_id(None).is_none());
    }

    #[test]
    fn elements_without_unique_id_are_not_indexed() {
        let (mut container, _) = container();
        container.add(entity(None));

        assert_eq!(container.len(), 1);
        assert!(container.get_by_unique_id(Some(0)).is_none());
    }

    #[test]
    fn removing_an_element_drops_its_index_entry() {
        let (mut container, _) = container();
        container.add(entity(Some(5)));

        let removed = container.remove(0).expect("下标 0 存在");
        assert_eq!(removed.borrow().get_unique_id(), Some(5));
        assert!(container.get_by_unique_id(Some(5)).is_none());
    }

    #[test]
    fn re_adding_the_same_instance_is_not_a_clash() {
        let (mut container, _) = container();
        let first = entity(Some(5));

        container.add(Rc::clone(&first));
        container.add(Rc::clone(&first));

        assert_eq!(container.len(), 2);
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(5)).expect("已索引"),
            &first
        ));

        // 同一个实例重复加入不会进入冲突列表
        container.fix_unique_id_clashes();
        assert_eq!(first.borrow().get_unique_id(), Some(5));
    }

    #[test]
    fn clashing_unique_ids_are_renumbered() {
        let (mut container, provider) = container();

        let first = entity(Some(5));
        let second = entity(Some(5));
        container.add(Rc::clone(&first));
        container.add(Rc::clone(&second));

        // 冲突时后者覆盖索引
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(5)).expect("已索引"),
            &second
        ));

        // 修复冲突：冲突者从序列取新值（序列从 1 开始）
        container.fix_unique_id_clashes();
        assert_eq!(second.borrow().get_unique_id(), Some(1));
        assert_eq!(first.borrow().get_unique_id(), Some(5));

        // 索引被重建
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(1)).expect("已索引"),
            &second
        ));
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(5)).expect("已索引"),
            &first
        ));

        // 序列已推进
        assert_eq!(
            provider
                .borrow_mut()
                .get_unique_id_object_sequence(TypeId::of::<Entity>())
                .get_next(),
            2
        );
    }

    #[test]
    fn fixing_clashes_on_empty_list_is_a_no_op() {
        let (mut container, provider) = container();
        container.add(entity(Some(1)));

        container.fix_unique_id_clashes();

        // 序列未被消耗
        assert_eq!(
            provider
                .borrow_mut()
                .get_unique_id_object_sequence(TypeId::of::<Entity>())
                .get_next(),
            1
        );
    }

    #[test]
    fn update_unique_id_moves_the_index_entry_and_syncs_sequence() {
        let (mut container, provider) = container();
        let first = entity(Some(5));
        container.add(Rc::clone(&first));

        container.update_unique_id(&first, Some(5), Some(9));

        assert!(container.get_by_unique_id(Some(5)).is_none());
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(9)).expect("已索引"),
            &first
        ));

        // Java: 序号生成器与新的 unique ID 同步
        assert_eq!(
            provider
                .borrow_mut()
                .get_unique_id_object_sequence(TypeId::of::<Entity>())
                .get_next(),
            10
        );
    }

    #[test]
    fn update_unique_id_detects_clash_with_other_element() {
        let (mut container, _) = container();
        let first = entity(Some(1));
        let second = entity(Some(2));
        container.add(Rc::clone(&first));
        container.add(Rc::clone(&second));

        // second 改用与 first 相同的 unique ID
        container.update_unique_id(&second, Some(2), Some(1));
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(1)).expect("已索引"),
            &second
        ));

        // 修复后 second 取序列中的下一个值（1 已被占用后同步为 2）
        container.fix_unique_id_clashes();
        assert_eq!(second.borrow().get_unique_id(), Some(2));
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(1)).expect("已索引"),
            &first
        ));
    }

    #[test]
    fn update_unique_id_with_null_new_value_only_removes_old_entry() {
        let (mut container, _) = container();
        let first = entity(Some(5));
        container.add(Rc::clone(&first));

        container.update_unique_id(&first, Some(5), None);

        assert!(container.get_by_unique_id(Some(5)).is_none());
        assert_eq!(container.len(), 1);
    }

    #[test]
    fn list_operations_keep_the_index_in_sync() {
        let (mut container, _) = container();

        // add_at 插入并建立索引
        assert!(container.add_at(0, entity(Some(1))));
        assert!(!container.add_at(2, entity(Some(3))));
        assert_eq!(container.len(), 1);
        assert!(container.get_by_unique_id(Some(1)).is_some());

        // set 替换元素：与 Java 一致，unique ID 索引**不会**随之更新
        // （Java 中 `ProjectEntityContainer` 只覆写了 `added` / `removed`）
        let replacement = entity(Some(2));
        let old = container.set(0, Rc::clone(&replacement)).expect("下标 0 存在");
        assert_eq!(old.borrow().get_unique_id(), Some(1));
        assert!(container.get_by_unique_id(Some(2)).is_none());
        assert!(container.get_by_unique_id(Some(1)).is_some());
        assert!(Rc::ptr_eq(
            &container.get(0).expect("下标 0 存在"),
            &replacement
        ));

        // clear 不触发 removed 回调，索引保留旧内容（与 Java 的 clear 实现一致）
        container.clear();
        assert!(container.is_empty());
        assert!(container.get_by_unique_id(Some(1)).is_some());
    }
}
