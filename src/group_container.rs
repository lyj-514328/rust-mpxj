//! 对应 Java: `org/mpxj/GroupContainer.java`
//!
//! 跳过/未移植的 Java 方法与原因：
//!
//! | Java | Rust |
//! |---|---|
//! | `@Override protected void added(Group group)` | 私有回调（在 [`GroupContainer::new`] 中注册到 [`crate::list_with_callbacks::ListWithCallbacks::set_added_callback`]） |
//! | `@Override protected void removed(Group group)` | 私有回调（同上，注册到 `set_removed_callback`） |
//! | `public Group getByName(String name)` | [`GroupContainer::get_by_name`] |
//! | 继承自 `ListWithCallbacks<Group>` 的 `add` / `add(int, T)` / `set` / `get` / `remove(int)` / `clear` / `size` / `isEmpty` / `iterator` | [`GroupContainer::add`] / [`GroupContainer::add_at`] / [`GroupContainer::set`] / [`GroupContainer::get`] / [`GroupContainer::remove`] / [`GroupContainer::clear`] / [`GroupContainer::len`] / [`GroupContainer::is_empty`] / [`GroupContainer::iter`]（另提供 [`GroupContainer::as_slice`]） |
//! | 继承自 `ListWithCallbacks<Group>` 的 `remove(Object)` / `indexOf` / `contains` 等按元素查找的方法 | 未移植：Java 用 `equals`（对象同一性）查找，Rust 侧元素是 `Rc<RefCell<Group>>`，调用方可用 [`GroupContainer::as_slice`] 配合 `Rc::ptr_eq` 自行判断 |
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//!
//! # 继承关系的还原方式：组合 + 转发
//!
//! Java 的 `GroupContainer extends ListWithCallbacks<Group>`。Rust 没有继承，本类改为
//! **组合**一个 [`crate::list_with_callbacks::ListWithCallbacks`] 并转发其公开行为；
//! Java 中由 `added` / `removed` 两个覆写钩子维护的**名称索引**则改用
//! `ListWithCallbacks` 的回调字段实现（回调闭包捕获索引的 `Rc` 句柄，见
//! [`GroupContainer::new`]），语义与 Java 一致。
//!
//! # 与 Java 的差异
//!
//! - **元素形态**：Java 存 `Group` 引用；Rust 侧存 `Rc<RefCell<Group>>`（与本 crate
//!   其它容器一致），使名称索引与列表共享同一批实例。
//! - **名称为 `null` 的分组**：Java 的 `HashMap` 允许 `null` 键；Rust 的
//!   `HashMap<String, _>` 无法表达 `null` 键，无名分组**不入索引**
//!   （[`GroupContainer::get_by_name`] 因此恒为 `None`）。
//! - **`replaced` 回调**：Java 的 `replaced` 未被覆写，[`GroupContainer::set`] 替换元素后
//!   名称索引不会更新（与 Java 一致）。
//! - **索引借用失败时降级**：回调通过 `try_borrow` 读取分组名称；若调用方正持有该分组的
//!   可变借用（Java 无此情形），则该次索引更新被跳过，不会 panic。

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use crate::group::Group;
use crate::list_with_callbacks::ListWithCallbacks;

/// 按名称索引的分组表（列表与索引共享同一批 `Rc` 实例）。
type GroupIndex = Rc<RefCell<HashMap<String, Rc<RefCell<Group>>>>>;

/// 项目下的分组定义集合。
///
/// 对应 Java: `public class GroupContainer extends ListWithCallbacks<Group>`
pub struct GroupContainer {
    /// 列表本体（对应 Java 的 `extends ListWithCallbacks<Group>`）。
    items: ListWithCallbacks<Rc<RefCell<Group>>>,

    /// 按名称索引的分组。
    ///
    /// 对应 Java: `private final Map<String, Group> m_groupsByName = new HashMap<>();`
    groups_by_name: GroupIndex,
}

impl GroupContainer {
    /// 构造方法（并注册维护名称索引的 `added` / `removed` 回调）。
    ///
    /// 对应 Java: 隐式默认构造方法 + `added` / `removed` 两个覆写钩子。
    pub fn new() -> Self {
        let groups_by_name: GroupIndex = Rc::new(RefCell::new(HashMap::new()));
        let mut items = ListWithCallbacks::new();

        // Java: @Override protected void added(Group group) { m_groupsByName.put(group.getName(), group); }
        let added_index = Rc::clone(&groups_by_name);
        items.set_added_callback(Some(Box::new(move |group: &Rc<RefCell<Group>>| {
            let Some(name) = read_name(group) else {
                return;
            };

            added_index.borrow_mut().insert(name, Rc::clone(group));
        })));

        // Java: @Override protected void removed(Group group) { m_groupsByName.remove(group.getName()); }
        let removed_index = Rc::clone(&groups_by_name);
        items.set_removed_callback(Some(Box::new(move |group: &Rc<RefCell<Group>>| {
            let Some(name) = read_name(group) else {
                return;
            };

            removed_index.borrow_mut().remove(&name);
        })));

        Self {
            items,
            groups_by_name,
        }
    }

    /// 按名称取分组定义。
    ///
    /// 对应 Java: `public Group getByName(String name)`
    pub fn get_by_name(&self, name: Option<&str>) -> Option<Rc<RefCell<Group>>> {
        let name = name?;
        self.groups_by_name.borrow().get(name).map(Rc::clone)
    }

    /// 追加一个分组，并更新名称索引。
    ///
    /// 对应 Java: `ListWithCallbacks.add(T element)`
    pub fn add(&mut self, group: Rc<RefCell<Group>>) {
        self.items.add(group);
    }

    /// 在指定下标插入一个分组，并更新名称索引。
    ///
    /// 对应 Java: `ListWithCallbacks.add(int index, T element)`
    ///
    /// 越界返回 `false`（Java 抛 `IndexOutOfBoundsException`）。
    pub fn add_at(&mut self, index: usize, group: Rc<RefCell<Group>>) -> bool {
        self.items.add_at(index, group)
    }

    /// 清空列表（与 Java 一致：不触发 `removed` 回调，名称索引因此不会清空）。
    ///
    /// 对应 Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// 按下标取分组；越界返回 `None`。
    ///
    /// 对应 Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<&Rc<RefCell<Group>>> {
        self.items.get(index)
    }

    /// 替换指定下标的分组并返回旧元素（不更新名称索引，与 Java 一致）。
    ///
    /// 对应 Java: `ListWithCallbacks.set(int index, T element)`
    pub fn set(&mut self, index: usize, group: Rc<RefCell<Group>>) -> Option<Rc<RefCell<Group>>> {
        self.items.set(index, group)
    }

    /// 移除指定下标的分组并返回它，同时更新名称索引。
    ///
    /// 对应 Java: `ListWithCallbacks.remove(int index)`
    pub fn remove(&mut self, index: usize) -> Option<Rc<RefCell<Group>>> {
        self.items.remove(index)
    }

    /// 分组个数。
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

    /// 迭代列表中的分组。
    ///
    /// 对应 Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<Group>>> {
        self.items.iter()
    }

    /// 以切片形式访问列表中的分组。
    ///
    /// 对应 Java: `ListWithCallbacks` 的列表视图。
    pub fn as_slice(&self) -> &[Rc<RefCell<Group>>] {
        self.items.as_slice()
    }
}

impl Default for GroupContainer {
    /// 对应 Java: 隐式默认构造方法，等价于 [`GroupContainer::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for GroupContainer {
    /// Java 侧未覆写 `toString()`；Rust 侧输出列表规模与名称索引键，便于诊断。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut keys: Vec<String> = self.groups_by_name.borrow().keys().cloned().collect();
        keys.sort();

        f.debug_struct("GroupContainer")
            .field("size", &self.items.len())
            .field("groups_by_name", &keys)
            .finish()
    }
}

/// 读取分组名称；已被可变借用时返回 `None`。
fn read_name(group: &Rc<RefCell<Group>>) -> Option<String> {
    group
        .try_borrow()
        .ok()
        .and_then(|group| group.get_name().map(str::to_owned))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建一个带名称的分组。
    fn group(name: &str) -> Rc<RefCell<Group>> {
        Rc::new(RefCell::new(Group::new(
            Some(1),
            Some(name.to_owned()),
            false,
        )))
    }

    #[test]
    fn container_basics() {
        let mut container = GroupContainer::new();
        assert!(container.is_empty());

        container.add(group("A"));
        container.add(group("B"));

        assert_eq!(container.len(), 2);
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert!(container.get(0).is_some());
        assert!(container.get(2).is_none());
    }

    #[test]
    fn get_by_name_finds_group() {
        let mut container = GroupContainer::new();
        let entry = group("Entry");
        container.add(Rc::clone(&entry));

        assert!(container
            .get_by_name(Some("Entry"))
            .map(|found| Rc::ptr_eq(&found, &entry))
            .unwrap_or(false));
        assert!(container.get_by_name(Some("Missing")).is_none());
        assert!(container.get_by_name(None).is_none());
    }

    #[test]
    fn last_group_with_same_name_wins() {
        let mut container = GroupContainer::new();
        container.add(group("Entry"));
        let second = group("Entry");
        container.add(Rc::clone(&second));

        assert!(container
            .get_by_name(Some("Entry"))
            .map(|found| Rc::ptr_eq(&found, &second))
            .unwrap_or(false));
    }

    #[test]
    fn remove_updates_name_index() {
        let mut container = GroupContainer::new();
        container.add(group("Entry"));

        assert!(container.remove(0).is_some());
        assert!(container.is_empty());
        assert!(container.get_by_name(Some("Entry")).is_none());
    }

    #[test]
    fn unnamed_group_is_not_indexed() {
        let mut container = GroupContainer::new();
        container.add(Rc::new(RefCell::new(Group::new(None, None, false))));

        assert_eq!(container.len(), 1);
        assert!(container.get_by_name(None).is_none());
    }

    #[test]
    fn add_at_set_and_clear_forward_to_list() {
        let mut container = GroupContainer::new();
        container.add(group("A"));

        assert!(container.add_at(0, group("B")));
        assert_eq!(
            container
                .get(0)
                .and_then(|group| group.borrow().get_name().map(str::to_owned)),
            Some("B".to_owned())
        );

        let replaced = container.set(0, group("C")).expect("替换成功");
        assert_eq!(replaced.borrow().get_name(), Some("B"));

        // 越界不修改列表
        assert!(!container.add_at(9, group("D")));
        assert_eq!(container.len(), 2);

        container.clear();
        assert!(container.is_empty());
    }

    #[test]
    fn default_matches_new() {
        let container = GroupContainer::default();
        assert!(container.is_empty());
    }

    #[test]
    fn debug_reports_index() {
        let mut container = GroupContainer::new();
        container.add(group("Entry"));

        let text = format!("{container:?}");
        assert!(text.contains("size: 1"));
        assert!(text.contains("Entry"));
    }
}
