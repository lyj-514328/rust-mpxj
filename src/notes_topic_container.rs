//! 对应 Java: `org/mpxj/NotesTopicContainer.java`

use std::cell::RefCell;
use std::rc::Rc;

use crate::notes_topic::NotesTopic;
use crate::project_entity_container::ProjectEntityContainer;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;

/// 当前项目可用的注释主题（notes topic）集合。
///
/// 对应 Java: `public class NotesTopicContainer extends ProjectEntityContainer<NotesTopic>`
///
/// # 继承体系的还原方式：组合 + 转发
///
/// Rust 没有继承，无法让本类「是」[`ProjectEntityContainer`]，因此改为组合一个
/// `ProjectEntityContainer<NotesTopic>` 并转发其公开行为（`add` / `get` /
/// `get_by_unique_id` / `iter` 等）。Java 中本类只声明了构造方法与 `getDefaultTopic()`，
/// 其余方法都继承自 `ProjectEntityContainer`，因此这些转发方法即「继承而来的方法」。
///
/// # 对并行模块的依赖
///
/// Java 的 `NotesTopic implements ProjectEntityWithUniqueID`，`src/notes_topic.rs` 已提供对应的
/// `impl`，本文件因此可以直接使用 `ProjectEntityContainer<NotesTopic>`。
///
/// # 与 Java 的差异
///
/// - `getDefaultTopic()` 内部用 [`NotesTopic::default_topic`]（对应 Java 的静态常量
///   `NotesTopic.DEFAULT`）构造默认主题；Java 返回的是同一个静态实例，而
///   `NotesTopic::default_topic()` 每次返回**新建的值**（与该模块的既有约定一致），
///   因此重复调用会返回值相等但非同一 `Rc` 的实例；容器中只会加入一次。
/// - **未**转发 `fixUniqueIdClashes()`：Java 中只有 unique ID 可变的实体会进入冲突列表，
///   而 `NotesTopic` 的 unique ID 是 `final`，冲突列表恒为空。
///
/// 被跳过的 Java 代码：`fixUniqueIdClashes()`（见上）。
///
/// 未移植的 Java 方法：无。
pub struct NotesTopicContainer {
    /// 基类容器。
    ///
    /// 对应 Java 的 `extends ProjectEntityContainer<NotesTopic>`。
    container: ProjectEntityContainer<NotesTopic>,
}

impl NotesTopicContainer {
    /// 构造方法。
    ///
    /// 对应 Java: `public NotesTopicContainer(UniqueIdObjectSequenceProvider sequenceProvider)`
    pub fn new(sequence_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>) -> Self {
        Self {
            container: ProjectEntityContainer::new(sequence_provider),
        }
    }

    /// 取默认主题，容器中还没有该 unique ID 时先加入再返回。
    ///
    /// 对应 Java: `public NotesTopic getDefaultTopic()`
    ///
    /// Java 的实现：
    /// ```text
    /// if (getByUniqueID(NotesTopic.DEFAULT.getUniqueID()) == null) { add(NotesTopic.DEFAULT); }
    /// return NotesTopic.DEFAULT;
    /// ```
    pub fn get_default_topic(&mut self) -> Rc<RefCell<NotesTopic>> {
        let default_topic = Rc::new(RefCell::new(NotesTopic::default_topic()));
        let unique_id = default_topic.borrow().get_unique_id();

        if self.container.get_by_unique_id(unique_id).is_none() {
            self.container.add(Rc::clone(&default_topic));
        }

        default_topic
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

    /// 追加一个注释主题。
    ///
    /// 对应 Java: `ListWithCallbacks.add(T element)`
    pub fn add(&mut self, element: Rc<RefCell<NotesTopic>>) {
        self.container.add(element);
    }

    /// 在指定下标插入注释主题（越界返回 `false`）。
    ///
    /// 对应 Java: `ListWithCallbacks.add(int index, T element)`
    pub fn add_at(&mut self, index: usize, element: Rc<RefCell<NotesTopic>>) -> bool {
        self.container.add_at(index, element)
    }

    /// 按下标取注释主题。
    ///
    /// 对应 Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<Rc<RefCell<NotesTopic>>> {
        self.container.get(index)
    }

    /// 替换指定下标的注释主题（越界返回 `None`）。
    ///
    /// 对应 Java: `ListWithCallbacks.set(int index, T element)`
    pub fn set(
        &mut self,
        index: usize,
        element: Rc<RefCell<NotesTopic>>,
    ) -> Option<Rc<RefCell<NotesTopic>>> {
        self.container.set(index, element)
    }

    /// 移除指定下标的注释主题（越界返回 `None`）。
    ///
    /// 对应 Java: `ListWithCallbacks.remove(int index)`
    pub fn remove(&mut self, index: usize) -> Option<Rc<RefCell<NotesTopic>>> {
        self.container.remove(index)
    }

    /// 按 unique ID 取注释主题。
    ///
    /// 对应 Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<NotesTopic>>> {
        self.container.get_by_unique_id(id)
    }

    /// 更新 unique ID 索引。
    ///
    /// 对应 Java: `ProjectEntityContainer.updateUniqueID(T element, Integer oldUniqueID, Integer newUniqueID)`
    pub fn update_unique_id(
        &mut self,
        element: &Rc<RefCell<NotesTopic>>,
        old_unique_id: Option<i32>,
        new_unique_id: Option<i32>,
    ) {
        self.container
            .update_unique_id(element, old_unique_id, new_unique_id);
    }

    /// 注释主题个数。
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

    /// 迭代容器内的注释主题。
    ///
    /// 对应 Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<NotesTopic>>> {
        self.container.iter()
    }

    /// 以切片形式访问容器内容。
    ///
    /// 对应 Java: `ListWithCallbacks` 的列表视图。
    pub fn as_slice(&self) -> &[Rc<RefCell<NotesTopic>>] {
        self.container.as_slice()
    }

    /// 清空容器（不触发 `removed` 回调，与 Java 一致）。
    ///
    /// 对应 Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.container.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn container() -> NotesTopicContainer {
        let provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            Rc::new(RefCell::new(TestProvider::default()));
        NotesTopicContainer::new(provider)
    }

    #[test]
    fn get_default_topic_adds_it_once() {
        let mut container = container();
        assert!(container.is_empty());

        let first = container.get_default_topic();
        assert_eq!(first.borrow().get_unique_id(), Some(1));
        assert_eq!(first.borrow().get_name(), Some("Notes"));
        assert_eq!(container.len(), 1);

        // 再次调用不会重复加入（Java 的 `getByUniqueID(DEFAULT.getUniqueID()) == null` 判断）
        let second = container.get_default_topic();
        assert_eq!(container.len(), 1);
        assert_eq!(*second.borrow(), *first.borrow());
    }

    #[test]
    fn container_stores_and_indexes_topics() {
        let mut container = container();
        let topic = Rc::new(RefCell::new(NotesTopic::default_topic()));
        container.add(Rc::clone(&topic));

        assert_eq!(container.len(), 1);
        assert_eq!(container.iter().count(), 1);
        assert_eq!(container.as_slice().len(), 1);
        assert_eq!(container.first_unique_id(), 1);
        assert!(Rc::ptr_eq(&container.get(0).expect("下标 0 存在"), &topic));
        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(1)).expect("已索引"),
            &topic
        ));
        assert!(container.get_by_unique_id(Some(2)).is_none());
        assert!(container.get_by_unique_id(None).is_none());

        // add_at 越界
        assert!(!container.add_at(
            5,
            Rc::new(RefCell::new(NotesTopic::default_topic()))
        ));
        assert_eq!(container.len(), 1);

        // remove 后索引同步清理
        let removed = container.remove(0).expect("下标存在");
        assert_eq!(removed.borrow().get_unique_id(), Some(1));
        assert!(container.is_empty());
        assert!(container.get_by_unique_id(Some(1)).is_none());
    }
}

