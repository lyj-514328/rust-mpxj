//! 对应 Java: `org/mpxj/FilterContainer.java`
//!
//! 跳过/未移植的 Java 方法与原因：
//!
//! | Java | Rust |
//! |---|---|
//! | 隐式默认构造方法 | [`FilterContainer::new`] |
//! | `public void addFilter(Filter filter)` | [`FilterContainer::add_filter`] |
//! | `public void removeFilter(String filterName)` | [`FilterContainer::remove_filter`] |
//! | `public List<Filter> getResourceFilters()` | [`FilterContainer::get_resource_filters`] |
//! | `public List<Filter> getTaskFilters()` | [`FilterContainer::get_task_filters`] |
//! | `public Filter getFilterByName(String name)` | [`FilterContainer::get_filter_by_name`] |
//! | `public Filter getFilterByID(Integer id)` | [`FilterContainer::get_filter_by_id`] |
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//!
//! # 与 Java 的差异
//!
//! - **元素形态**：Java 的四个集合存 `Filter` 引用；Rust 侧统一存
//!   `Rc<RefCell<Filter>>`（与本 crate 其它容器一致），使列表与两张索引共享同一批实例。
//! - **按引用相等移除**：Java 的 `List.remove(Object)` 用 `equals` 定位元素，而 `Filter`
//!   未覆写 `equals`（对象同一性）；Rust 侧用 `Rc::ptr_eq` 复刻同一性语义。
//! - **`null` 键**：Java 的 `HashMap` 允许 `null` 键，未命名的筛选器也会进入
//!   `m_filtersByName`；Rust 的键类型是 `String` / `i32`，因此名称或 ID 为 `None` 的
//!   筛选器**不入索引**（仍会按其任务/资源标志进入对应列表）。
//! - **`getFilterByName` / `getFilterByID` 的入参**：Java 接受可为 `null` 的参数，
//!   Rust 侧用 `Option<&str>` / `Option<i32>`。
//! - **列表访问**：Java 返回可变列表；Rust 侧返回只读切片，如需修改请用
//!   [`FilterContainer::add_filter`] / [`FilterContainer::remove_filter`]。

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use crate::filter::Filter;

/// 项目下的筛选器定义集合。
///
/// 对应 Java: `public class FilterContainer`
pub struct FilterContainer {
    /// 全部任务筛选器。
    ///
    /// 对应 Java: `private final List<Filter> m_taskFilters = new ArrayList<>();`
    m_task_filters: Vec<Rc<RefCell<Filter>>>,

    /// 全部资源筛选器。
    ///
    /// 对应 Java: `private final List<Filter> m_resourceFilters = new ArrayList<>();`
    m_resource_filters: Vec<Rc<RefCell<Filter>>>,

    /// 按名称索引的筛选器。
    ///
    /// 对应 Java: `private final Map<String, Filter> m_filtersByName = new HashMap<>();`
    m_filters_by_name: HashMap<String, Rc<RefCell<Filter>>>,

    /// 按 ID 索引的筛选器。
    ///
    /// 对应 Java: `private final Map<Integer, Filter> m_filtersByID = new HashMap<>();`
    m_filters_by_id: HashMap<i32, Rc<RefCell<Filter>>>,
}

impl FilterContainer {
    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法（Java 侧未显式声明构造方法）。
    pub fn new() -> Self {
        Self {
            m_task_filters: Vec::new(),
            m_resource_filters: Vec::new(),
            m_filters_by_name: HashMap::new(),
            m_filters_by_id: HashMap::new(),
        }
    }

    /// 向项目文件加入一个筛选器定义（按标志分别进入任务/资源列表，并建立名称与 ID 索引）。
    ///
    /// 对应 Java: `public void addFilter(Filter filter)`
    pub fn add_filter(&mut self, filter: Rc<RefCell<Filter>>) {
        // 读取标志 / 名称 / ID；已被可变借用时退化为「不入任何集合」。
        let Ok(filter_ref) = filter.try_borrow() else {
            return;
        };

        let is_task_filter = filter_ref.is_task_filter();
        let is_resource_filter = filter_ref.is_resource_filter();
        let name = filter_ref.get_name().map(str::to_owned);
        let id = filter_ref.get_id();
        drop(filter_ref);

        if is_task_filter {
            self.m_task_filters.push(Rc::clone(&filter));
        }

        if is_resource_filter {
            self.m_resource_filters.push(Rc::clone(&filter));
        }

        // Java 的 HashMap 允许 null 键；Rust 侧跳过（见文件头说明）
        if let Some(name) = name {
            self.m_filters_by_name.insert(name, Rc::clone(&filter));
        }

        if let Some(id) = id {
            self.m_filters_by_id.insert(id, Rc::clone(&filter));
        }
    }

    /// 按名称移除一个筛选器。
    ///
    /// 对应 Java: `public void removeFilter(String filterName)`
    pub fn remove_filter(&mut self, filter_name: Option<&str>) {
        let Some(filter) = self.get_filter_by_name(filter_name) else {
            return;
        };

        let (is_task_filter, is_resource_filter, id) = match filter.try_borrow() {
            Ok(filter_ref) => (
                filter_ref.is_task_filter(),
                filter_ref.is_resource_filter(),
                filter_ref.get_id(),
            ),
            Err(_) => (false, false, None),
        };

        if is_task_filter {
            remove_by_identity(&mut self.m_task_filters, &filter);
        }

        if is_resource_filter {
            remove_by_identity(&mut self.m_resource_filters, &filter);
        }

        if let Some(filter_name) = filter_name {
            self.m_filters_by_name.remove(filter_name);
        }

        if let Some(id) = id {
            self.m_filters_by_id.remove(&id);
        }
    }

    /// 取全部资源筛选器。
    ///
    /// 对应 Java: `public List<Filter> getResourceFilters()`
    pub fn get_resource_filters(&self) -> &[Rc<RefCell<Filter>>] {
        &self.m_resource_filters
    }

    /// 取全部任务筛选器。
    ///
    /// 对应 Java: `public List<Filter> getTaskFilters()`
    pub fn get_task_filters(&self) -> &[Rc<RefCell<Filter>>] {
        &self.m_task_filters
    }

    /// 按名称取筛选器。
    ///
    /// 对应 Java: `public Filter getFilterByName(String name)`
    pub fn get_filter_by_name(&self, name: Option<&str>) -> Option<Rc<RefCell<Filter>>> {
        let name = name?;
        self.m_filters_by_name.get(name).map(Rc::clone)
    }

    /// 按 ID 取筛选器。
    ///
    /// 对应 Java: `public Filter getFilterByID(Integer id)`
    pub fn get_filter_by_id(&self, id: Option<i32>) -> Option<Rc<RefCell<Filter>>> {
        let id = id?;
        self.m_filters_by_id.get(&id).map(Rc::clone)
    }
}

impl Default for FilterContainer {
    /// 对应 Java: 隐式默认构造方法，等价于 [`FilterContainer::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for FilterContainer {
    /// Java 侧未覆写 `toString()`；Rust 侧输出各集合规模，便于诊断。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilterContainer")
            .field("task_filters", &self.m_task_filters.len())
            .field("resource_filters", &self.m_resource_filters.len())
            .field("filters_by_name", &self.m_filters_by_name.len())
            .field("filters_by_id", &self.m_filters_by_id.len())
            .finish()
    }
}

/// 按 `Rc` 同一性从列表中移除元素（Java 的 `List.remove(Object)`）。
fn remove_by_identity(filters: &mut Vec<Rc<RefCell<Filter>>>, filter: &Rc<RefCell<Filter>>) {
    if let Some(index) = filters
        .iter()
        .position(|candidate| Rc::ptr_eq(candidate, filter))
    {
        filters.remove(index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建一个筛选器。
    fn filter(id: i32, name: &str, task: bool, resource: bool) -> Rc<RefCell<Filter>> {
        let mut value = Filter::new();
        value.set_id(Some(id));
        value.set_name(Some(name.to_owned()));
        value.set_is_task_filter(task);
        value.set_is_resource_filter(resource);
        Rc::new(RefCell::new(value))
    }

    #[test]
    fn container_basics() {
        let mut container = FilterContainer::new();
        assert!(container.get_task_filters().is_empty());
        assert!(container.get_resource_filters().is_empty());

        container.add_filter(filter(1, "Tasks", true, false));
        container.add_filter(filter(2, "Resources", false, true));

        assert_eq!(container.get_task_filters().len(), 1);
        assert_eq!(container.get_resource_filters().len(), 1);
        assert!(container.get_filter_by_id(None).is_none());
        assert!(container.get_filter_by_name(None).is_none());
        assert!(container.get_filter_by_id(Some(9)).is_none());
        assert!(container.get_filter_by_name(Some("Missing")).is_none());
    }

    #[test]
    fn both_flags_put_filter_in_both_lists() {
        let mut container = FilterContainer::new();
        let both = filter(1, "Both", true, true);
        container.add_filter(Rc::clone(&both));

        assert_eq!(container.get_task_filters().len(), 1);
        assert_eq!(container.get_resource_filters().len(), 1);
    }

    #[test]
    fn name_and_id_indexes_are_populated() {
        let mut container = FilterContainer::new();
        let entry = filter(4, "Entry", true, false);
        container.add_filter(Rc::clone(&entry));

        assert!(container
            .get_filter_by_name(Some("Entry"))
            .map(|found| Rc::ptr_eq(&found, &entry))
            .unwrap_or(false));
        assert!(container
            .get_filter_by_id(Some(4))
            .map(|found| Rc::ptr_eq(&found, &entry))
            .unwrap_or(false));

        // 同名筛选器以最后加入者为准（Java 的 HashMap.put）
        let replacement = filter(5, "Entry", true, false);
        container.add_filter(Rc::clone(&replacement));
        assert!(container
            .get_filter_by_name(Some("Entry"))
            .map(|found| Rc::ptr_eq(&found, &replacement))
            .unwrap_or(false));
    }

    #[test]
    fn remove_filter_updates_all_collections() {
        let mut container = FilterContainer::new();
        container.add_filter(filter(1, "Entry", true, true));
        container.add_filter(filter(2, "Other", true, false));

        container.remove_filter(Some("Entry"));

        assert_eq!(container.get_task_filters().len(), 1);
        assert!(container.get_resource_filters().is_empty());
        assert!(container.get_filter_by_name(Some("Entry")).is_none());
        assert!(container.get_filter_by_id(Some(1)).is_none());
        assert_eq!(
            container
                .get_task_filters()
                .first()
                .and_then(|filter| filter.borrow().get_name().map(str::to_owned)),
            Some("Other".to_owned())
        );
    }

    #[test]
    fn remove_unknown_filter_is_noop() {
        let mut container = FilterContainer::new();
        container.add_filter(filter(1, "Entry", true, false));

        container.remove_filter(Some("Missing"));
        container.remove_filter(None);

        assert_eq!(container.get_task_filters().len(), 1);
    }

    #[test]
    fn unnamed_filter_is_not_indexed_but_listed() {
        let mut container = FilterContainer::new();
        let mut value = Filter::new();
        value.set_is_task_filter(true);
        let unnamed = Rc::new(RefCell::new(value));
        container.add_filter(Rc::clone(&unnamed));

        assert_eq!(container.get_task_filters().len(), 1);
        assert!(container.get_filter_by_name(None).is_none());
        assert!(container.get_filter_by_id(None).is_none());
    }

    #[test]
    fn debug_reports_collection_sizes() {
        let mut container = FilterContainer::new();
        container.add_filter(filter(1, "Entry", true, false));

        let text = format!("{container:?}");
        assert!(text.contains("task_filters: 1"));
        assert!(text.contains("resource_filters: 0"));
    }

    #[test]
    fn default_matches_new() {
        let container = FilterContainer::default();
        assert!(container.get_task_filters().is_empty());
    }
}
