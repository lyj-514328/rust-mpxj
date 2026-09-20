//! 对应 Java: `org/mpxj/WorkContourContainer.java`

use std::cell::RefCell;
use std::rc::Rc;

use crate::project_entity_container::ProjectEntityContainer;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;
use crate::work_contour::WorkContour;

/// 当前项目可用的工时分布（resource curve）集合。
///
/// 对应 Java: `public class WorkContourContainer extends ProjectEntityContainer<WorkContour>`
///
/// # 继承体系的还原方式：组合 + 转发
///
/// Rust 没有继承，无法让本类「是」[`ProjectEntityContainer`]，因此改为组合一个
/// `ProjectEntityContainer<WorkContour>` 并转发其公开行为（`add` / `get` /
/// `get_by_unique_id` / `iter` 等）。Java 中本类只声明了构造方法，其余方法都继承自
/// `ProjectEntityContainer`，因此这些转发方法即「继承而来的方法」。
///
/// # 与 Java 的差异
///
/// - `fixUniqueIdClashes()` **未**转发：Java 中只有 unique ID 可变的实体才会进入冲突列表，
///   而 `WorkContour` 的 unique ID 是 `final`（其 `ProjectEntityWithUniqueID` 实现只有读取
///   方法），冲突列表恒为空，转发该方法没有意义。
/// - Java 的 `WorkContour implements ProjectEntityWithUniqueID`：Rust 侧的实现已归位到
///   [`crate::work_contour`]，本文件只保留容器逻辑。
///
/// 被跳过的 Java 代码：`fixUniqueIdClashes()`（见上）。
///
/// 未移植的 Java 方法：无。
pub struct WorkContourContainer {
    /// 基类容器。
    ///
    /// 对应 Java 的 `extends ProjectEntityContainer<WorkContour>`。
    container: ProjectEntityContainer<WorkContour>,
}

impl WorkContourContainer {
    /// 构造方法。
    ///
    /// 对应 Java: `public WorkContourContainer(UniqueIdObjectSequenceProvider sequenceProvider)`
    pub fn new(sequence_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>) -> Self {
        Self {
            container: ProjectEntityContainer::new(sequence_provider),
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

    /// 追加一个工时分布。
    ///
    /// 对应 Java: `ListWithCallbacks.add(T element)`
    pub fn add(&mut self, element: Rc<RefCell<WorkContour>>) {
        self.container.add(element);
    }

    /// 按下标取工时分布。
    ///
    /// 对应 Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<Rc<RefCell<WorkContour>>> {
        self.container.get(index)
    }

    /// 在指定下标插入工时分布（越界返回 `false`）。
    ///
    /// 对应 Java: `ListWithCallbacks.add(int index, T element)`
    pub fn add_at(&mut self, index: usize, element: Rc<RefCell<WorkContour>>) -> bool {
        self.container.add_at(index, element)
    }

    /// 替换指定下标的工时分布（越界返回 `None`）。
    ///
    /// 对应 Java: `ListWithCallbacks.set(int index, T element)`
    pub fn set(
        &mut self,
        index: usize,
        element: Rc<RefCell<WorkContour>>,
    ) -> Option<Rc<RefCell<WorkContour>>> {
        self.container.set(index, element)
    }

    /// 移除指定下标的工时分布（越界返回 `None`）。
    ///
    /// 对应 Java: `ListWithCallbacks.remove(int index)`
    pub fn remove(&mut self, index: usize) -> Option<Rc<RefCell<WorkContour>>> {
        self.container.remove(index)
    }

    /// 按 unique ID 取工时分布。
    ///
    /// 对应 Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<WorkContour>>> {
        self.container.get_by_unique_id(id)
    }

    /// 更新 unique ID 索引。
    ///
    /// 对应 Java: `ProjectEntityContainer.updateUniqueID(T element, Integer oldUniqueID, Integer newUniqueID)`
    pub fn update_unique_id(
        &mut self,
        element: &Rc<RefCell<WorkContour>>,
        old_unique_id: Option<i32>,
        new_unique_id: Option<i32>,
    ) {
        self.container
            .update_unique_id(element, old_unique_id, new_unique_id);
    }

    /// 工时分布个数。
    ///
    /// 对应 Java: `ListWithCallbacks.size()`
    pub fn len(&self) -> usize {
        self.container.len()
    }

    /// 容器是否为空。
    ///
    /// 对应 Java: `ListWithCallbacks.isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.container.is_empty()
    }

    /// 迭代容器内的工时分布。
    ///
    /// 对应 Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<WorkContour>>> {
        self.container.iter()
    }

    /// 以切片形式访问容器内容。
    ///
    /// 对应 Java: `ListWithCallbacks` 的列表视图。
    pub fn as_slice(&self) -> &[Rc<RefCell<WorkContour>>] {
        self.container.as_slice()
    }

    /// 清空容器（不触发 `removed` 回调，与 Java 一致）。
    ///
    /// 对应 Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.container.clear();
    }
}

impl std::fmt::Debug for WorkContourContainer {
    /// Java 侧未覆写 `toString()`；Rust 侧输出内部容器。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkContourContainer")
            .field("container", &self.container)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
    use crate::common::object_sequence::ObjectSequence;
    use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;
    use std::any::TypeId;
    use std::collections::HashMap;

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
    fn container() -> WorkContourContainer {
        let provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            Rc::new(RefCell::new(TestProvider::default()));
        WorkContourContainer::new(provider)
    }

    #[test]
    fn work_contour_exposes_unique_id_through_the_trait() {
        let flat = WorkContour::flat();

        assert_eq!(ProjectEntityWithUniqueID::get_unique_id(&flat), Some(1));
        assert_eq!(flat.get_unique_id(), Some(1));
    }

    #[test]
    fn container_stores_and_indexes_work_contours() {
        let mut container = container();
        assert!(container.is_empty());

        let flat = Rc::new(RefCell::new(WorkContour::flat()));
        let bell = Rc::new(RefCell::new(WorkContour::bell()));

        container.add(Rc::clone(&flat));
        container.add(Rc::clone(&bell));

        assert_eq!(container.len(), 2);
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert_eq!(container.first_unique_id(), 1);
        assert_eq!(
            container.get(0).map(|item| item.borrow().get_name().map(str::to_owned)),
            Some(Some("FLAT".to_owned()))
        );

        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(7)).expect("BELL 已索引"),
            &bell
        ));
        assert!(container.get_by_unique_id(Some(1)).is_some());
        assert!(container.get_by_unique_id(Some(99)).is_none());
        assert!(container.get_by_unique_id(None).is_none());
    }

    #[test]
    fn removing_and_clearing_updates_the_container() {
        let mut container = container();
        let flat = Rc::new(RefCell::new(WorkContour::flat()));
        container.add(Rc::clone(&flat));

        // add_at 越界
        assert!(!container.add_at(5, Rc::new(RefCell::new(WorkContour::bell()))));
        assert_eq!(container.len(), 1);

        // remove 后索引同步清理
        let removed = container.remove(0).expect("下标存在");
        assert_eq!(removed.borrow().get_name(), Some("FLAT"));
        assert!(container.is_empty());
        assert!(container.get_by_unique_id(Some(1)).is_none());
        assert!(container.remove(0).is_none());

        // clear 不触发 removed 回调
        let bell = Rc::new(RefCell::new(WorkContour::bell()));
        container.add(Rc::clone(&bell));
        container.clear();
        assert!(container.is_empty());
        assert!(container.set(0, Rc::clone(&bell)).is_none());
    }

    #[test]
    fn update_unique_id_moves_the_index_entry() {
        let mut container = container();
        let flat = Rc::new(RefCell::new(WorkContour::flat()));
        container.add(Rc::clone(&flat));

        container.update_unique_id(&flat, Some(1), Some(100));

        assert!(container.get_by_unique_id(Some(1)).is_none());
        assert!(container.get_by_unique_id(Some(100)).is_some());
    }
}
