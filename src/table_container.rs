//! 对应 Java: `org/mpxj/TableContainer.java`
//!
//! 跳过/未移植的 Java 方法与原因：
//!
//! | Java | Rust |
//! |---|---|
//! | `@Override protected void added(Table table)` | 私有回调（在 [`TableContainer::new`] 中注册到 [`crate::list_with_callbacks::ListWithCallbacks::set_added_callback`]） |
//! | `@Override protected void removed(Table table)` | 私有回调（同上，注册到 `set_removed_callback`） |
//! | `public Table getTaskTableByName(String name)` | [`TableContainer::get_task_table_by_name`] |
//! | `public Table getResourceTableByName(String name)` | [`TableContainer::get_resource_table_by_name`] |
//! | `private Map<String, Table> getIndex(Table table)` | 私有函数 `index_for` |
//! | 继承自 `ListWithCallbacks<Table>` 的 `add` / `add(int, T)` / `set` / `get` / `remove(int)` / `clear` / `size` / `isEmpty` / `iterator` | [`TableContainer::add`] / [`TableContainer::add_at`] / [`TableContainer::set`] / [`TableContainer::get`] / [`TableContainer::remove`] / [`TableContainer::clear`] / [`TableContainer::len`] / [`TableContainer::is_empty`] / [`TableContainer::iter`]（另提供 [`TableContainer::as_slice`]） |
//! | 继承自 `ListWithCallbacks<Table>` 的 `remove(Object)` / `indexOf` / `contains` 等按元素查找的方法 | 未移植：Java 用 `equals`（对象同一性）查找，Rust 侧元素是 `Rc<RefCell<Table>>`，调用方可用 [`TableContainer::as_slice`] 配合 `Rc::ptr_eq` 自行判断 |
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//!
//! # 继承关系的还原方式：组合 + 转发
//!
//! Java 的 `TableContainer extends ListWithCallbacks<Table>`。Rust 没有继承，本类改为
//! **组合**一个 [`crate::list_with_callbacks::ListWithCallbacks`] 并转发其公开行为；
//! Java 中由 `added` / `removed` 两个覆写钩子维护的**名称索引**则改用
//! `ListWithCallbacks` 的回调字段实现（回调闭包捕获索引的 `Rc` 句柄，见
//! [`TableContainer::new`]），语义与 Java 一致。
//!
//! # 与 Java 的差异
//!
//! - **元素形态**：Java 的列表存 `Table` 引用；Rust 侧存 `Rc<RefCell<Table>>`
//!   （与本 crate 其它容器一致），使名称索引可以与列表共享同一批实例。
//! - **名称为 `null` 的表格**：Java 的 `HashMap` 允许 `null` 键，因此未命名表格也能进入索引；
//!   Rust 的 `HashMap<String, _>` 无法表达 `null` 键，[`TableContainer::new`] 中注册的回调
//!   对无名表格**不入索引**（`get_task_table_by_name(None)` 因此恒为 `None`）。
//! - **`replaced` 回调**：Java 的 `ListWithCallbacks.replaced` 未被 `TableContainer` 覆写，
//!   因此 [`TableContainer::set`] 替换元素后名称索引**不会**更新（与 Java 一致，此处保持原样）。
//! - **索引借用失败时降级**：回调通过 `try_borrow` 读取表格名称 / 资源标志；若调用方正持有
//!   该表格的可变借用（Java 无此情形），则该次索引更新被跳过，不会 panic。
//! - **`getTaskTableByName` 的入参**：Java 接受可为 `null` 的 `String`，Rust 侧用 `Option<&str>`。

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use crate::list_with_callbacks::ListWithCallbacks;
use crate::table::Table;

/// 按名称索引的表格表（列表与索引共享同一批 `Rc` 实例）。
type TableIndex = Rc<RefCell<HashMap<String, Rc<RefCell<Table>>>>>;

/// 项目下的表格定义集合。
///
/// 对应 Java: `public class TableContainer extends ListWithCallbacks<Table>`
pub struct TableContainer {
    /// 列表本体（对应 Java 的 `extends ListWithCallbacks<Table>`）。
    items: ListWithCallbacks<Rc<RefCell<Table>>>,

    /// 按名称索引的任务表。
    ///
    /// 对应 Java: `private final Map<String, Table> m_taskTablesByName = new HashMap<>();`
    task_tables_by_name: TableIndex,

    /// 按名称索引的资源表。
    ///
    /// 对应 Java: `private final Map<String, Table> m_resourceTablesByName = new HashMap<>();`
    resource_tables_by_name: TableIndex,
}

impl TableContainer {
    /// 构造方法（并注册维护名称索引的 `added` / `removed` 回调）。
    ///
    /// 对应 Java: 隐式默认构造方法 + `added` / `removed` 两个覆写钩子。
    pub fn new() -> Self {
        let task_tables_by_name: TableIndex = Rc::new(RefCell::new(HashMap::new()));
        let resource_tables_by_name: TableIndex = Rc::new(RefCell::new(HashMap::new()));

        let mut items = ListWithCallbacks::new();

        // Java: @Override protected void added(Table table) { getIndex(table).put(table.getName(), table); }
        let added_task_index = Rc::clone(&task_tables_by_name);
        let added_resource_index = Rc::clone(&resource_tables_by_name);
        items.set_added_callback(Some(Box::new(
            move |table: &Rc<RefCell<Table>>| {
                let Some(name) = read_name(table) else {
                    return;
                };

                let index = index_for(table, &added_task_index, &added_resource_index);
                index.borrow_mut().insert(name, Rc::clone(table));
            },
        )));

        // Java: @Override protected void removed(Table table) { getIndex(table).remove(table.getName()); }
        let removed_task_index = Rc::clone(&task_tables_by_name);
        let removed_resource_index = Rc::clone(&resource_tables_by_name);
        items.set_removed_callback(Some(Box::new(
            move |table: &Rc<RefCell<Table>>| {
                let Some(name) = read_name(table) else {
                    return;
                };

                let index = index_for(table, &removed_task_index, &removed_resource_index);
                index.borrow_mut().remove(&name);
            },
        )));

        Self {
            items,
            task_tables_by_name,
            resource_tables_by_name,
        }
    }

    /// 按名称取任务表定义。
    ///
    /// 对应 Java: `public Table getTaskTableByName(String name)`
    pub fn get_task_table_by_name(&self, name: Option<&str>) -> Option<Rc<RefCell<Table>>> {
        let name = name?;
        self.task_tables_by_name.borrow().get(name).map(Rc::clone)
    }

    /// 按名称取资源表定义。
    ///
    /// 对应 Java: `public Table getResourceTableByName(String name)`
    pub fn get_resource_table_by_name(&self, name: Option<&str>) -> Option<Rc<RefCell<Table>>> {
        let name = name?;
        self.resource_tables_by_name
            .borrow()
            .get(name)
            .map(Rc::clone)
    }

    /// 追加一个表格，并更新名称索引。
    ///
    /// 对应 Java: `ListWithCallbacks.add(T element)`
    pub fn add(&mut self, table: Rc<RefCell<Table>>) {
        self.items.add(table);
    }

    /// 在指定下标插入一个表格，并更新名称索引。
    ///
    /// 对应 Java: `ListWithCallbacks.add(int index, T element)`
    ///
    /// 越界返回 `false`（Java 抛 `IndexOutOfBoundsException`）。
    pub fn add_at(&mut self, index: usize, table: Rc<RefCell<Table>>) -> bool {
        self.items.add_at(index, table)
    }

    /// 清空列表（与 Java 一致：不触发 `removed` 回调，名称索引因此不会清空）。
    ///
    /// 对应 Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// 按下标取表格；越界返回 `None`。
    ///
    /// 对应 Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<&Rc<RefCell<Table>>> {
        self.items.get(index)
    }

    /// 替换指定下标的表格并返回旧元素（不更新名称索引，与 Java 一致）。
    ///
    /// 对应 Java: `ListWithCallbacks.set(int index, T element)`
    pub fn set(
        &mut self,
        index: usize,
        table: Rc<RefCell<Table>>,
    ) -> Option<Rc<RefCell<Table>>> {
        self.items.set(index, table)
    }

    /// 移除指定下标的表格并返回它，同时更新名称索引。
    ///
    /// 对应 Java: `ListWithCallbacks.remove(int index)`
    pub fn remove(&mut self, index: usize) -> Option<Rc<RefCell<Table>>> {
        self.items.remove(index)
    }

    /// 表格个数。
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

    /// 迭代列表中的表格。
    ///
    /// 对应 Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<Table>>> {
        self.items.iter()
    }

    /// 以切片形式访问列表中的表格。
    ///
    /// 对应 Java: `ListWithCallbacks` 的列表视图。
    pub fn as_slice(&self) -> &[Rc<RefCell<Table>>] {
        self.items.as_slice()
    }
}

impl Default for TableContainer {
    /// 对应 Java: 隐式默认构造方法，等价于 [`TableContainer::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for TableContainer {
    /// Java 侧未覆写 `toString()`；Rust 侧输出列表规模与两张名称索引的键，
    /// 便于诊断（不输出 `Table` 内容）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let read_keys = |index: &TableIndex| -> Vec<String> {
            let mut keys: Vec<String> = index.borrow().keys().cloned().collect();
            keys.sort();
            keys
        };

        f.debug_struct("TableContainer")
            .field("size", &self.items.len())
            .field("task_tables", &read_keys(&self.task_tables_by_name))
            .field("resource_tables", &read_keys(&self.resource_tables_by_name))
            .finish()
    }
}

/// 取应使用的名称索引（Java 的 `getIndex(Table)`）。
fn index_for(
    table: &Rc<RefCell<Table>>,
    task_tables: &TableIndex,
    resource_tables: &TableIndex,
) -> TableIndex {
    if read_resource_flag(table) {
        Rc::clone(resource_tables)
    } else {
        Rc::clone(task_tables)
    }
}

/// 读取表格名称；已被可变借用时返回 `None`。
fn read_name(table: &Rc<RefCell<Table>>) -> Option<String> {
    table
        .try_borrow()
        .ok()
        .and_then(|table| table.get_name().map(str::to_owned))
}

/// 读取资源表标志；已被可变借用时按 `false`（任务表）处理。
fn read_resource_flag(table: &Rc<RefCell<Table>>) -> bool {
    table
        .try_borrow()
        .map(|table| table.get_resource_flag())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建一个带名称与资源标志的表格。
    fn table(name: &str, resource: bool) -> Rc<RefCell<Table>> {
        let mut value = Table::new();
        value.set_name(Some(name.to_owned()));
        value.set_resource_flag(resource);
        Rc::new(RefCell::new(value))
    }

    #[test]
    fn container_basics() {
        let mut container = TableContainer::new();
        assert!(container.is_empty());

        container.add(table("Entry", false));
        container.add(table("Resources", true));

        assert_eq!(container.len(), 2);
        assert!(!container.is_empty());
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert!(container.get(0).is_some());
        assert!(container.get(2).is_none());
    }

    #[test]
    fn name_indexes_are_separated_by_resource_flag() {
        let mut container = TableContainer::new();
        let task_table = table("Entry", false);
        let resource_table = table("Entry", true);
        container.add(Rc::clone(&task_table));
        container.add(Rc::clone(&resource_table));

        assert!(container
            .get_task_table_by_name(Some("Entry"))
            .map(|found| Rc::ptr_eq(&found, &task_table))
            .unwrap_or(false));
        assert!(container
            .get_resource_table_by_name(Some("Entry"))
            .map(|found| Rc::ptr_eq(&found, &resource_table))
            .unwrap_or(false));

        assert!(container.get_task_table_by_name(Some("Other")).is_none());
        assert!(container.get_task_table_by_name(None).is_none());
    }

    #[test]
    fn last_table_with_same_name_wins() {
        let mut container = TableContainer::new();
        container.add(table("Entry", false));
        let second = table("Entry", false);
        container.add(Rc::clone(&second));

        // Java 的 HashMap.put 以最后加入者为准
        assert!(container
            .get_task_table_by_name(Some("Entry"))
            .map(|found| Rc::ptr_eq(&found, &second))
            .unwrap_or(false));
    }

    #[test]
    fn remove_updates_name_index() {
        let mut container = TableContainer::new();
        container.add(table("Entry", false));

        let removed = container.remove(0).expect("移除成功");
        assert!(container.is_empty());
        assert!(!removed.borrow().get_resource_flag());
        assert!(container.get_task_table_by_name(Some("Entry")).is_none());
    }

    #[test]
    fn unnamed_table_is_not_indexed() {
        let mut container = TableContainer::new();
        container.add(Rc::new(RefCell::new(Table::new())));

        assert_eq!(container.len(), 1);
        assert!(container.get_task_table_by_name(None).is_none());
        assert!(container.get_resource_table_by_name(None).is_none());
    }

    #[test]
    fn add_at_and_set_forward_to_list() {
        let mut container = TableContainer::new();
        container.add(table("A", false));

        assert!(container.add_at(0, table("B", false)));
        assert_eq!(container.len(), 2);
        assert_eq!(
            container
                .get(0)
                .and_then(|table| table.borrow().get_name().map(str::to_owned)),
            Some("B".to_owned())
        );

        let replaced = container.set(0, table("C", false)).expect("替换成功");
        assert_eq!(replaced.borrow().get_name(), Some("B"));

        // 越界不修改列表
        assert!(!container.add_at(9, table("D", false)));
        assert_eq!(container.len(), 2);
    }

    #[test]
    fn default_matches_new() {
        let container = TableContainer::default();
        assert!(container.is_empty());
    }

    #[test]
    fn debug_reports_indexes() {
        let mut container = TableContainer::new();
        container.add(table("Entry", false));
        container.add(table("Resources", true));

        let text = format!("{container:?}");
        assert!(text.contains("size: 2"));
        assert!(text.contains("Entry"));
        assert!(text.contains("Resources"));
    }
}
