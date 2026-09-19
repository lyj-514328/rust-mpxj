//! 对应 Java: `org/mpxj/ViewContainer.java`
//!
//! 跳过/未移植的 Java 方法与原因：
//!
//! | Java | Rust |
//! |---|---|
//! | `public void setViewState(ViewState viewState)` | [`ViewContainer::set_view_state`] |
//! | `public ViewState getViewState()` | [`ViewContainer::get_view_state`] |
//! | 继承自 `ListWithCallbacks<View>` 的 `add` / `add(int, T)` / `set` / `get` / `remove(int)` / `clear` / `size` / `isEmpty` / `iterator` | [`ViewContainer::add`] / [`ViewContainer::add_at`] / [`ViewContainer::set`] / [`ViewContainer::get`] / [`ViewContainer::remove`] / [`ViewContainer::clear`] / [`ViewContainer::len`] / [`ViewContainer::is_empty`] / [`ViewContainer::iter`]（另提供 [`ViewContainer::as_slice`]） |
//! | 继承自 `ListWithCallbacks<View>` 的 `remove(Object)` / `indexOf` / `contains` 等按元素查找的方法 | 未移植：Java 用 `equals`（对象同一性）查找，Rust 侧元素是 `Rc<RefCell<dyn View>>`，调用方可用 [`ViewContainer::as_slice`] 配合 `Rc::ptr_eq` 自行判断 |
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//!
//! # 继承关系的还原方式：组合 + 转发
//!
//! Java 的 `ViewContainer extends ListWithCallbacks<View>`。Rust 没有继承，本类改为
//! **组合**一个 [`crate::list_with_callbacks::ListWithCallbacks`] 并转发其公开行为。
//! Java 侧本类未覆写任何 `ListWithCallbacks` 钩子，因此组合后无需注册回调。
//!
//! # 与 Java 的差异
//!
//! - **元素形态**：`View` 是接口（Rust 侧为 [`crate::view::View`] trait），因此元素类型是
//!   `Rc<RefCell<dyn View>>`；Java 存的是 `View` 引用。`Rc<RefCell<..>>` 与本 crate 其它
//!   容器一致，便于列表与视图状态共享同一批实例。
//! - **`getViewState()`**：Java 可能返回 `null`，Rust 侧返回 `Option<&ViewState>`；
//!   需要取得所有权时调用方可自行 `Clone`（本类不额外提供 `_mut` 访问器，Java 也没有）。

use std::cell::RefCell;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use crate::list_with_callbacks::ListWithCallbacks;
use crate::view::View;
use crate::view_state::ViewState;

/// 项目下的视图集合（以及文件保存的视图状态）。
///
/// 对应 Java: `public class ViewContainer extends ListWithCallbacks<View>`
pub struct ViewContainer {
    /// 列表本体（对应 Java 的 `extends ListWithCallbacks<View>`）。
    items: ListWithCallbacks<Rc<RefCell<dyn View>>>,

    /// 本文件保存的视图状态。
    ///
    /// 对应 Java: `private ViewState m_viewState;`
    m_view_state: Option<ViewState>,
}

impl ViewContainer {
    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法（Java 侧未显式声明构造方法）。
    pub fn new() -> Self {
        Self {
            items: ListWithCallbacks::new(),
            m_view_state: None,
        }
    }

    /// 设置本文件保存的视图状态。
    ///
    /// 对应 Java: `public void setViewState(ViewState viewState)`
    pub fn set_view_state(&mut self, view_state: Option<ViewState>) {
        self.m_view_state = view_state;
    }

    /// 取本文件保存的视图状态。
    ///
    /// 对应 Java: `public ViewState getViewState()`
    pub fn get_view_state(&self) -> Option<&ViewState> {
        self.m_view_state.as_ref()
    }

    /// 追加一个视图。
    ///
    /// 对应 Java: `ListWithCallbacks.add(T element)`
    pub fn add(&mut self, view: Rc<RefCell<dyn View>>) {
        self.items.add(view);
    }

    /// 在指定下标插入一个视图。
    ///
    /// 对应 Java: `ListWithCallbacks.add(int index, T element)`
    ///
    /// 越界返回 `false`（Java 抛 `IndexOutOfBoundsException`）。
    pub fn add_at(&mut self, index: usize, view: Rc<RefCell<dyn View>>) -> bool {
        self.items.add_at(index, view)
    }

    /// 清空列表（与 Java 一致：不触发 `removed` 回调）。
    ///
    /// 对应 Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// 按下标取视图；越界返回 `None`。
    ///
    /// 对应 Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<&Rc<RefCell<dyn View>>> {
        self.items.get(index)
    }

    /// 替换指定下标的视图并返回旧元素。
    ///
    /// 对应 Java: `ListWithCallbacks.set(int index, T element)`
    pub fn set(
        &mut self,
        index: usize,
        view: Rc<RefCell<dyn View>>,
    ) -> Option<Rc<RefCell<dyn View>>> {
        self.items.set(index, view)
    }

    /// 移除指定下标的视图并返回它。
    ///
    /// 对应 Java: `ListWithCallbacks.remove(int index)`
    pub fn remove(&mut self, index: usize) -> Option<Rc<RefCell<dyn View>>> {
        self.items.remove(index)
    }

    /// 视图个数。
    ///
    /// 对应 Java: `ListWithCallbacks.size()`
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// 列表是否为空。
    ///
    /// 对应 Java: `ListWithCallbacks.isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 迭代列表中的视图。
    ///
    /// 对应 Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<dyn View>>> {
        self.items.iter()
    }

    /// 以切片形式访问列表中的视图。
    ///
    /// 对应 Java: `ListWithCallbacks` 的列表视图。
    pub fn as_slice(&self) -> &[Rc<RefCell<dyn View>>] {
        self.items.as_slice()
    }
}

impl Default for ViewContainer {
    /// 对应 Java: 隐式默认构造方法，等价于 [`ViewContainer::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for ViewContainer {
    /// Java 侧未覆写 `toString()`；Rust 侧输出规模与是否保存了视图状态
    /// （`dyn View` 未实现 `Debug`，故不输出元素内容）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewContainer")
            .field("size", &self.items.len())
            .field("view_state_is_set", &self.m_view_state.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::Table;
    use crate::view_type::ViewType;

    /// 测试用视图实现（Java 中对应的实现类位于 `org.mpxj.mpp`，本仓未移植）。
    struct MockView {
        id: Option<i32>,
        name: Option<String>,
    }

    impl View for MockView {
        fn get_id(&self) -> Option<i32> {
            self.id
        }

        fn get_name(&self) -> Option<&str> {
            self.name.as_deref()
        }

        fn get_type(&self) -> Option<ViewType> {
            None
        }

        fn get_table_name(&self) -> Option<&str> {
            None
        }

        fn get_table(&self) -> Option<Rc<RefCell<Table>>> {
            None
        }
    }

    /// 创建一个测试视图。
    fn view(id: i32, name: &str) -> Rc<RefCell<dyn View>> {
        Rc::new(RefCell::new(MockView {
            id: Some(id),
            name: Some(name.to_owned()),
        }))
    }

    #[test]
    fn container_basics() {
        let mut container = ViewContainer::new();
        assert!(container.is_empty());
        assert!(container.get_view_state().is_none());

        container.add(view(1, "Gantt Chart"));
        container.add(view(2, "Resource Sheet"));

        assert_eq!(container.len(), 2);
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert!(container.get(0).is_some());
        assert!(container.get(2).is_none());
        assert_eq!(
            container
                .get(1)
                .and_then(|view| view.borrow().get_name().map(str::to_owned)),
            Some("Resource Sheet".to_owned())
        );
    }

    #[test]
    fn view_state_round_trips() {
        let mut container = ViewContainer::new();
        container.set_view_state(Some(ViewState::new(
            std::rc::Weak::new(),
            Some("Gantt Chart".to_owned()),
            vec![1, 2],
            3,
        )));

        let state = container.get_view_state().expect("已设置视图状态");
        assert_eq!(state.get_view_name(), Some("Gantt Chart"));
        assert_eq!(state.get_unique_id_list(), &[1, 2]);

        container.set_view_state(None);
        assert!(container.get_view_state().is_none());
    }

    #[test]
    fn list_operations_forward_to_base_list() {
        let mut container = ViewContainer::new();
        container.add(view(1, "A"));

        assert!(container.add_at(0, view(2, "B")));
        assert_eq!(
            container
                .get(0)
                .and_then(|view| view.borrow().get_name().map(str::to_owned)),
            Some("B".to_owned())
        );

        let replaced = container.set(0, view(3, "C")).expect("替换成功");
        assert_eq!(
            replaced.borrow().get_name().map(str::to_owned),
            Some("B".to_owned())
        );

        assert!(container.remove(0).is_some());
        assert_eq!(container.len(), 1);

        container.clear();
        assert!(container.is_empty());

        assert!(!container.add_at(9, view(4, "D")));
    }

    #[test]
    fn default_matches_new() {
        let container = ViewContainer::default();
        assert!(container.is_empty());
    }

    #[test]
    fn debug_reports_size_and_state() {
        let mut container = ViewContainer::new();
        container.add(view(1, "A"));

        let text = format!("{container:?}");
        assert!(text.contains("size: 1"));
        assert!(text.contains("view_state_is_set: false"));
    }
}
