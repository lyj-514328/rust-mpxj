//! 对应 Java: `org/mpxj/DataLinkContainer.java`
//!
//! 管理属于某个项目的数据链接（[`DataLink`]）定义。
//!
//! # Java → Rust 方法对照
//!
//! Java 的 `DataLinkContainer` 是**空体类**（`extends ListWithCallbacks<DataLink>`，
//! 无任何自有成员），因此本文件只需把继承而来的公开行为逐个转发给
//! [`ListWithCallbacks`]：
//!
//! | Java（继承自 `ListWithCallbacks` / `AbstractList`） | Rust |
//! |---|---|
//! | `public boolean add(T e)` | [`DataLinkContainer::add`] |
//! | `public void add(int index, T element)` | [`DataLinkContainer::add_at`] |
//! | `public T set(int index, T element)` | [`DataLinkContainer::set`] |
//! | `public T remove(int index)` | [`DataLinkContainer::remove_at`] |
//! | `AbstractCollection.remove(Object)` | [`DataLinkContainer::remove_element`]（Java 用引用相等，Rust 用指针同一性，见下） |
//! | `public void clear()` | [`DataLinkContainer::clear`] |
//! | `public T get(int index)` | [`DataLinkContainer::get`] |
//! | `public int size()` | [`DataLinkContainer::len`] |
//! | `public boolean isEmpty()` | [`DataLinkContainer::is_empty`] |
//! | `AbstractList.iterator()` | [`DataLinkContainer::iter`] |
//!
//! # 继承体系的还原：组合 + 转发
//!
//! Java 用继承得到列表实现；Rust 没有继承，故本类型**组合**一个
//! [`ListWithCallbacks`]，与 [`crate::project_calendar_container::ProjectCalendarContainer`]
//! 「包装 + 转发」的做法一致。`ListWithCallbacks` 的三个回调（`added` / `removed` /
//! `replaced`）在本类中保持未注册状态——Java 的 `DataLinkContainer` 也没有覆写它们，
//! 行为完全一致（均为空实现）。
//!
//! # 与 Java 的差异
//!
//! - **元素形态**：Java 存 `DataLink` 引用（可变对象）；Rust 侧按本 crate
//!   「共享实体用 `Rc<RefCell<T>>`」的约定存 `Rc<RefCell<DataLink>>`，
//!   这样调用方仍可像 Java 一样持有并修改同一个链接对象。
//! - **`remove(Object)`**：Java 的 `AbstractCollection.remove(Object)` 用
//!   `equals`（`DataLink` 未覆写，即对象同一性）定位元素。Rust 侧无法表达「对象同一性」
//!   的比较语义（`&DataLink` 的 `==` 是值比较），故改用 [`RefCell::as_ptr`] 的
//!   **指针同一性**比较，与 Java 的默认 `equals` 等价。
//! - **越界不抛异常**：Java 的 `get` / `set` / `remove(int)` / `add(int, T)` 越界会抛
//!   `IndexOutOfBoundsException`；Rust 侧沿用 [`ListWithCallbacks`] 的约定
//!   （越界返回 `None` / `false`），不 panic。
//! - **`add(T)` 的返回值**：Java 恒返回 `true`；Rust 侧不返回值（与
//!   [`ListWithCallbacks::add`] 一致）。
//! - **`sort` 未提供**：Java 的 `ListWithCallbacks` 本身没有排序方法（排序由
//!   `ProjectEntityWithIDContainer` 等子类提供），`DataLinkContainer` 不涉及。
//!
//! # 跳过的 Java 机制
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::cell::RefCell;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use crate::data_link::DataLink;
use crate::list_with_callbacks::ListWithCallbacks;

/// 项目的数据链接集合。
///
/// 对应 Java: `public class DataLinkContainer extends ListWithCallbacks<DataLink>`
pub struct DataLinkContainer {
    /// 列表实现（Java 的 `extends ListWithCallbacks<DataLink>`）。
    list: ListWithCallbacks<Rc<RefCell<DataLink>>>,
}

impl DataLinkContainer {
    /// 构造方法。
    ///
    /// 对应 Java: `DataLinkContainer` 的隐式无参构造方法。
    pub fn new() -> Self {
        Self {
            list: ListWithCallbacks::new(),
        }
    }

    /// 在列表尾部追加一个链接。
    ///
    /// 对应 Java: `ListWithCallbacks.add(T e)`
    pub fn add(&mut self, link: Rc<RefCell<DataLink>>) {
        self.list.add(link);
    }

    /// 在指定下标处插入一个链接；越界返回 `false`。
    ///
    /// 对应 Java: `ListWithCallbacks.add(int index, T element)`
    pub fn add_at(&mut self, index: usize, link: Rc<RefCell<DataLink>>) -> bool {
        self.list.add_at(index, link)
    }

    /// 替换指定下标的链接，返回被替换掉的旧链接；越界返回 `None`。
    ///
    /// 对应 Java: `ListWithCallbacks.set(int index, T element)`
    pub fn set(
        &mut self,
        index: usize,
        link: Rc<RefCell<DataLink>>,
    ) -> Option<Rc<RefCell<DataLink>>> {
        self.list.set(index, link)
    }

    /// 移除指定下标的链接并返回它；越界返回 `None`。
    ///
    /// 对应 Java: `ListWithCallbacks.remove(int index)`
    pub fn remove_at(&mut self, index: usize) -> Option<Rc<RefCell<DataLink>>> {
        self.list.remove(index)
    }

    /// 按指针同一性移除一个链接（等效 Java 的 `AbstractCollection.remove(Object)`）。
    ///
    /// 对应 Java: `AbstractCollection.remove(Object o)`
    pub fn remove_element(&mut self, link: &DataLink) -> bool {
        let target = link as *const DataLink;
        let index = self
            .list
            .iter()
            .position(|candidate| candidate.as_ptr() as *const DataLink == target);

        match index {
            Some(index) => {
                self.list.remove(index);
                true
            }
            None => false,
        }
    }

    /// 清空列表（与 Java 一致：**不**触发 `removed` 回调）。
    ///
    /// 对应 Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.list.clear();
    }

    /// 按下标取链接；越界返回 `None`。
    ///
    /// 对应 Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<Rc<RefCell<DataLink>>> {
        self.list.get(index).cloned()
    }

    /// 链接个数。
    ///
    /// 对应 Java: `ListWithCallbacks.size()`
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// 列表是否为空。
    ///
    /// 对应 Java: `ListWithCallbacks.isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// 迭代列表内的链接。
    ///
    /// 对应 Java: `AbstractList.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<DataLink>>> {
        self.list.iter()
    }

    /// 以切片形式访问列表内容。
    ///
    /// 对应 Java: `AbstractList` 的列表视图。
    pub fn as_slice(&self) -> &[Rc<RefCell<DataLink>>] {
        self.list.as_slice()
    }
}

impl Default for DataLinkContainer {
    /// 对应 Java: `DataLinkContainer` 的隐式无参构造方法。
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for DataLinkContainer {
    /// Java 侧未覆写 `toString()`；Rust 侧输出列表内容（元素为 `Rc<RefCell<DataLink>>`，
    /// 其 `Debug` 输出被借用中的元素会显示为 `RefCell { value: <borrowed> }`，不会 panic）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataLinkContainer")
            .field("size", &self.list.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_key::FieldKey;
    use crate::task_field::TaskField;

    /// 构造一个链接句柄。
    fn link(id: &str) -> Rc<RefCell<DataLink>> {
        Rc::new(RefCell::new(DataLink::new(id)))
    }

    #[test]
    fn container_basics() {
        let mut container = DataLinkContainer::new();
        assert!(container.is_empty());
        assert_eq!(container.len(), 0);
        assert!(container.get(0).is_none());

        container.add(link("1"));
        container.add(link("2"));

        assert_eq!(container.len(), 2);
        assert!(!container.is_empty());
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert_eq!(container.get(0).map(|l| l.borrow().get_id().to_owned()), Some("1".to_owned()));
        assert!(container.get(2).is_none());
    }

    #[test]
    fn add_at_and_set_round_trip() {
        let mut container = DataLinkContainer::new();
        let first = link("1");
        let second = link("2");
        let third = link("3");

        container.add(Rc::clone(&first));
        container.add(Rc::clone(&third));

        assert!(container.add_at(1, Rc::clone(&second)));
        // 越界插入不修改列表（Java 抛 IndexOutOfBoundsException）
        assert!(!container.add_at(9, Rc::clone(&first)));
        assert_eq!(
            container.iter().map(|l| l.borrow().get_id().to_owned()).collect::<Vec<_>>(),
            vec!["1".to_owned(), "2".to_owned(), "3".to_owned()]
        );

        let replacement = link("9");
        let old = container.set(0, Rc::clone(&replacement)).expect("下标 0 存在");
        assert!(Rc::ptr_eq(&old, &first));
        assert!(container.set(9, Rc::clone(&first)).is_none());
    }

    #[test]
    fn remove_at_and_remove_element() {
        let mut container = DataLinkContainer::new();
        let first = link("1");
        let second = link("2");
        container.add(Rc::clone(&first));
        container.add(Rc::clone(&second));

        // Java: remove(int index)
        let removed = container.remove_at(0).expect("下标 0 存在");
        assert!(Rc::ptr_eq(&removed, &first));
        assert!(container.remove_at(9).is_none());

        // Java: remove(Object) —— 指针同一性
        let removed = {
            let second_ref = second.borrow();
            container.remove_element(&second_ref)
        };
        assert!(removed);
        assert!(container.is_empty());

        // 已经不在容器中：再次移除返回 false
        let removed = {
            let second_ref = second.borrow();
            container.remove_element(&second_ref)
        };
        assert!(!removed);
    }

    #[test]
    fn clear_drops_all_links() {
        let mut container = DataLinkContainer::new();
        container.add(link("1"));
        container.add(link("2"));

        container.clear();

        assert!(container.is_empty());
        assert_eq!(container.iter().count(), 0);
    }

    #[test]
    fn links_are_shared_with_callers_like_java() {
        let mut container = DataLinkContainer::new();
        let shared = link("1");
        container.add(Rc::clone(&shared));

        // 调用方仍可修改容器中的同一个对象（Java 语义）
        shared
            .borrow_mut()
            .set_source_field(Some(FieldKey::Task(TaskField::Name)));

        let from_container = container.get(0).expect("已加入容器");
        assert_eq!(
            from_container.borrow().get_source_field(),
            Some(FieldKey::Task(TaskField::Name))
        );
        assert!(Rc::ptr_eq(&from_container, &shared));
    }

    #[test]
    fn default_matches_new() {
        let container = DataLinkContainer::default();
        assert!(container.is_empty());
        assert!(format!("{container:?}").contains("size: 0"));
    }
}
