//! 对应 Java: `org/mpxj/ListWithCallbacks.java`

use std::fmt::{self, Debug, Formatter};

/// 由 `Vec` 支撑、并在增删改时触发回调的列表。
///
/// 对应 Java: `public abstract class ListWithCallbacks<T> extends AbstractList<T>`
///
/// # 与 Java 的差异：用「回调字段」替代「子类覆写」
///
/// Java 侧是抽象类，子类（如 `ProjectEntityContainer`）通过覆写 `protected` 的
/// `added` / `removed` / `replaced` 三个钩子获得通知。Rust 没有继承，改为在结构体里
/// 用 `Option<Box<dyn FnMut(..)>>` 承载回调，由持有方通过
/// [`ListWithCallbacks::set_added_callback`] 等方法注册；未注册时的行为与 Java 中的空实现
/// 完全一致（什么都不做）。
///
/// 回调在列表变更**之后**触发，参数与被改动的元素一致：
///
/// | Java | Rust |
/// |---|---|
/// | `protected void added(T element)` | [`ListWithCallbacks::set_added_callback`] |
/// | `protected void removed(T element)` | [`ListWithCallbacks::set_removed_callback`] |
/// | `protected void replaced(T oldElement, T newElement)` | [`ListWithCallbacks::set_replaced_callback`] |
///
/// # 与 Java 的差异：越界不再抛异常
///
/// Java 侧 `get` / `set` / `remove` / `add(int, T)` 越界会抛
/// `IndexOutOfBoundsException`；Rust 侧不 panic：
/// * [`get`](ListWithCallbacks::get) / [`remove`](ListWithCallbacks::remove) 越界返回 `None`；
/// * [`set`](ListWithCallbacks::set) 越界返回 `None` 且不修改列表；
/// * [`add_at`](ListWithCallbacks::add_at) 越界返回 `false` 且不修改列表。
///
/// # 与 Java 的差异：`Collections.sort`
///
/// Java 侧 `AbstractList` 没有排序方法，`ProjectEntityWithIDContainer` 用
/// `Collections.sort(this)`，其内部经由 `set(int, T)` 逐个替换元素，因此会触发
/// `replaced` 回调。Rust 侧的 [`sort`](ListWithCallbacks::sort) 直接对内部 `Vec` 排序，
/// 不触发回调；对本 crate 的容器而言 `replaced` 回调均为空实现，行为等价。
pub struct ListWithCallbacks<T> {
    /// 列表内容。
    ///
    /// 对应 Java: `private final List<T> m_list = new ArrayList<>();`
    items: Vec<T>,
    /// 元素新增回调。
    ///
    /// 对应 Java: `protected void added(T element)`
    added: Option<Box<dyn FnMut(&T)>>,
    /// 元素移除回调。
    ///
    /// 对应 Java: `protected void removed(T element)`
    removed: Option<Box<dyn FnMut(&T)>>,
    /// 元素替换回调。
    ///
    /// 对应 Java: `protected void replaced(T oldElement, T newElement)`
    replaced: Option<Box<dyn FnMut(&T, &T)>>,
}

impl<T> Default for ListWithCallbacks<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ListWithCallbacks<T> {
    /// 构造一个空列表。
    ///
    /// 对应 Java: `ArrayList` 的默认构造方法。
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            added: None,
            removed: None,
            replaced: None,
        }
    }

    /// 注册元素新增回调。
    ///
    /// 对应 Java: 子类覆写 `protected void added(T element)`
    pub fn set_added_callback(&mut self, callback: Option<Box<dyn FnMut(&T)>>) {
        self.added = callback;
    }

    /// 注册元素移除回调。
    ///
    /// 对应 Java: 子类覆写 `protected void removed(T element)`
    pub fn set_removed_callback(&mut self, callback: Option<Box<dyn FnMut(&T)>>) {
        self.removed = callback;
    }

    /// 注册元素替换回调。
    ///
    /// 对应 Java: 子类覆写 `protected void replaced(T oldElement, T newElement)`
    pub fn set_replaced_callback(&mut self, callback: Option<Box<dyn FnMut(&T, &T)>>) {
        self.replaced = callback;
    }

    /// 清空列表。与 Java 一致：**不**触发 `removed` 回调。
    ///
    /// 对应 Java: `@Override public void clear()`
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// 按下标取元素；越界返回 `None`（Java 抛 `IndexOutOfBoundsException`）。
    ///
    /// 对应 Java: `@Override public T get(int index)`
    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }

    /// 元素个数。
    ///
    /// 对应 Java: `@Override public int size()`
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// 列表是否为空。
    ///
    /// 对应 Java: `@Override public boolean isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 替换指定下标的元素，返回被替换掉的旧元素。
    ///
    /// 对应 Java: `@Override public T set(int index, T element)`
    ///
    /// 替换成功时触发 `replaced` 回调；越界返回 `None` 且不修改列表。
    pub fn set(&mut self, index: usize, element: T) -> Option<T> {
        if index >= self.items.len() {
            return None;
        }

        let old_element = std::mem::replace(&mut self.items[index], element);

        if let Some(callback) = self.replaced.as_mut() {
            callback(&old_element, &self.items[index]);
        }

        Some(old_element)
    }

    /// 在列表尾部追加元素，并触发 `added` 回调。
    ///
    /// 对应 Java: `@Override public boolean add(T e)`
    ///
    /// Java 侧恒返回 `true`，Rust 侧不返回值。
    pub fn add(&mut self, element: T) {
        self.items.push(element);

        let index = self.items.len() - 1;
        if let Some(callback) = self.added.as_mut() {
            callback(&self.items[index]);
        }
    }

    /// 在指定下标处插入元素，并触发 `added` 回调。
    ///
    /// 对应 Java: `@Override public void add(int index, T element)`
    ///
    /// 越界（`index > len`）返回 `false` 且不修改列表（Java 抛
    /// `IndexOutOfBoundsException`），插入成功返回 `true`。
    pub fn add_at(&mut self, index: usize, element: T) -> bool {
        if index > self.items.len() {
            return false;
        }

        self.items.insert(index, element);

        if let Some(callback) = self.added.as_mut() {
            callback(&self.items[index]);
        }

        true
    }

    /// 移除指定下标的元素并返回它，同时触发 `removed` 回调。
    ///
    /// 对应 Java: `@Override public T remove(int index)`
    ///
    /// 越界返回 `None`（Java 抛 `IndexOutOfBoundsException`）。
    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= self.items.len() {
            return None;
        }

        let removed = self.items.remove(index);

        if let Some(callback) = self.removed.as_mut() {
            callback(&removed);
        }

        Some(removed)
    }

    /// 按下标迭代列表。
    ///
    /// 对应 Java: `AbstractList.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }

    /// 以切片形式访问列表内容。
    ///
    /// 对应 Java: `AbstractList` 的列表视图。
    pub fn as_slice(&self) -> &[T] {
        &self.items
    }

    /// 按 `Ord` 就地排序（不触发回调，见类型说明）。
    ///
    /// 对应 Java: `ProjectEntityWithIDContainer` 中的 `Collections.sort(this)`
    pub fn sort(&mut self)
    where
        T: Ord,
    {
        self.items.sort();
    }
}

impl<T: Debug> Debug for ListWithCallbacks<T> {
    /// Java 侧 `AbstractList` 提供 `toString()`；Rust 侧用 `Debug` 输出内部元素。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.items.iter()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// 创建一个记录回调事件的日志。
    fn log() -> Rc<RefCell<Vec<String>>> {
        Rc::new(RefCell::new(Vec::new()))
    }

    /// 给列表注册三个会写日志的回调。
    fn with_logging_callbacks(list: &mut ListWithCallbacks<i32>, log: Rc<RefCell<Vec<String>>>) {
        let added = Rc::clone(&log);
        list.set_added_callback(Some(Box::new(move |element| {
            added.borrow_mut().push(format!("added:{element}"));
        })));

        let removed = Rc::clone(&log);
        list.set_removed_callback(Some(Box::new(move |element| {
            removed.borrow_mut().push(format!("removed:{element}"));
        })));

        list.set_replaced_callback(Some(Box::new(move |old, new| {
            log.borrow_mut().push(format!("replaced:{old}->{new}"));
        })));
    }

    #[test]
    fn add_appends_and_notifies() {
        let mut list = ListWithCallbacks::new();
        let log = log();
        with_logging_callbacks(&mut list, Rc::clone(&log));

        list.add(1);
        list.add(2);

        assert_eq!(list.len(), 2);
        assert_eq!(list.get(0), Some(&1));
        assert_eq!(list.get(1), Some(&2));
        assert_eq!(list.get(2), None);
        assert_eq!(*log.borrow(), vec!["added:1", "added:2"]);
    }

    #[test]
    fn add_at_inserts_at_index() {
        let mut list = ListWithCallbacks::new();
        let log = log();
        with_logging_callbacks(&mut list, Rc::clone(&log));

        list.add(1);
        list.add(3);
        assert!(list.add_at(1, 2));

        assert_eq!(list.as_slice(), &[1, 2, 3]);
        assert_eq!(*log.borrow(), vec!["added:1", "added:3", "added:2"]);
    }

    #[test]
    fn add_at_out_of_range_is_ignored() {
        let mut list: ListWithCallbacks<i32> = ListWithCallbacks::new();
        let log = log();
        with_logging_callbacks(&mut list, Rc::clone(&log));

        assert!(!list.add_at(1, 1));
        assert!(list.is_empty());
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn set_returns_old_value_and_notifies() {
        let mut list = ListWithCallbacks::new();
        let log = log();
        with_logging_callbacks(&mut list, Rc::clone(&log));

        list.add(1);
        log.borrow_mut().clear();

        assert_eq!(list.set(0, 9), Some(1));
        assert_eq!(list.get(0), Some(&9));
        assert_eq!(*log.borrow(), vec!["replaced:1->9"]);

        // 越界不修改列表、不触发回调
        assert_eq!(list.set(5, 10), None);
        assert_eq!(list.as_slice(), &[9]);
        assert_eq!(log.borrow().len(), 1);
    }

    #[test]
    fn remove_returns_element_and_notifies() {
        let mut list = ListWithCallbacks::new();
        let log = log();
        with_logging_callbacks(&mut list, Rc::clone(&log));

        list.add(1);
        list.add(2);
        log.borrow_mut().clear();

        assert_eq!(list.remove(0), Some(1));
        assert_eq!(list.as_slice(), &[2]);
        assert_eq!(*log.borrow(), vec!["removed:1"]);

        // 越界不触发回调
        assert_eq!(list.remove(5), None);
        assert_eq!(log.borrow().len(), 1);
    }

    #[test]
    fn clear_does_not_notify() {
        let mut list = ListWithCallbacks::new();
        let log = log();
        with_logging_callbacks(&mut list, Rc::clone(&log));

        list.add(1);
        list.add(2);
        log.borrow_mut().clear();

        list.clear();

        assert!(list.is_empty());
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn sort_orders_contents_without_notifying() {
        let mut list = ListWithCallbacks::new();
        let log = log();
        with_logging_callbacks(&mut list, Rc::clone(&log));

        list.add(3);
        list.add(1);
        list.add(2);
        log.borrow_mut().clear();

        list.sort();

        assert_eq!(list.as_slice(), &[1, 2, 3]);
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn callbacks_are_optional() {
        let mut list = ListWithCallbacks::new();

        list.add(1);
        assert_eq!(list.set(0, 2), Some(1));
        assert_eq!(list.remove(0), Some(2));
        assert!(list.is_empty());
    }
}
