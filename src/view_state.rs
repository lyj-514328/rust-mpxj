//! 对应 Java: `org/mpxj/ViewState.java`
//!
//! 跳过/未移植的 Java 方法与原因：
//!
//! | Java | Rust |
//! |---|---|
//! | `public ViewState(ProjectFile file, String viewName, List<Integer> uniqueIdList, int filterID)` | [`ViewState::new`]（Rust 侧接收 `Weak<RefCell<ProjectFile>>`，见下） |
//! | `public String getViewName()` | [`ViewState::get_view_name`] |
//! | `public List<Integer> getUniqueIdList()` | [`ViewState::get_unique_id_list`] |
//! | `public Filter getFilter()` | [`ViewState::get_filter`] |
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//!
//! # 与 Java 的差异
//!
//! - **父项目引用**：Java 的 `m_file` 是强引用；Rust 侧存
//!   `Weak<RefCell<ProjectFile>>`（`ProjectFile` 持有 `ViewContainer`，而
//!   `ViewContainer` 又持有 `ViewState`，用强引用会形成 `Rc` 循环）。
//!   [`ViewState::get_filter`] 在父文件已释放 / 已被借用时返回 `None`
//!   （Java 会抛 `NullPointerException`）。
//! - **对相关模块的接口假设**：`getFilter()` 依赖
//!   `ProjectFile::get_filters(&self) -> &FilterContainer`（Java 的
//!   `ProjectFile.getFilters()` 返回 `FilterContainer`），与本 crate 其它模块对
//!   `ProjectFile` 的取用方式一致。
//! - **`equals` / `hashCode`**：Java 未覆写（对象同一性）；Rust 侧不实现
//!   `PartialEq` / `Eq` / `Hash`。

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::filter::Filter;
use crate::project_file::ProjectFile;

/// 项目文件中保存的视图状态。
///
/// 对应 Java: `public final class ViewState`
pub struct ViewState {
    /// 父项目（弱引用）。
    ///
    /// 对应 Java: `private final ProjectFile m_file;`
    m_file: Weak<RefCell<ProjectFile>>,

    /// 视图名称。
    ///
    /// 对应 Java: `private final String m_viewName;`
    m_view_name: Option<String>,

    /// 视图内容的 unique ID 列表。
    ///
    /// 对应 Java: `private final List<Integer> m_uniqueIdList;`
    m_unique_id_list: Vec<i32>,

    /// 当前应用的筛选器 ID。
    ///
    /// 对应 Java: `private final Integer m_filterID;`
    m_filter_id: i32,
}

impl ViewState {
    /// 构造方法。
    ///
    /// 对应 Java: `public ViewState(ProjectFile file, String viewName, List<Integer> uniqueIdList, int filterID)`
    ///
    /// Rust 侧以弱引用接收父项目（见文件头说明）。
    pub fn new(
        file: Weak<RefCell<ProjectFile>>,
        view_name: Option<String>,
        unique_id_list: Vec<i32>,
        filter_id: i32,
    ) -> Self {
        Self {
            m_file: file,
            m_view_name: view_name,
            m_unique_id_list: unique_id_list,
            m_filter_id: filter_id,
        }
    }

    /// 取视图名称。
    ///
    /// 对应 Java: `public String getViewName()`
    pub fn get_view_name(&self) -> Option<&str> {
        self.m_view_name.as_deref()
    }

    /// 取视图内容的 unique ID 列表。
    ///
    /// 对应 Java: `public List<Integer> getUniqueIdList()`
    pub fn get_unique_id_list(&self) -> &[i32] {
        &self.m_unique_id_list
    }

    /// 取当前应用的筛选器。
    ///
    /// 对应 Java: `public Filter getFilter()`
    ///
    /// 父项目未接线 / 已释放 / 已被借用时返回 `None`（Java 抛
    /// `NullPointerException`）。
    pub fn get_filter(&self) -> Option<Rc<RefCell<Filter>>> {
        let file = self.m_file.upgrade()?;
        let file_ref = file.try_borrow().ok()?;

        file_ref.get_filters().get_filter_by_id(Some(self.m_filter_id))
    }
}

impl std::fmt::Debug for ViewState {
    /// Java 未覆写 `toString()`；Rust 侧输出字段（不输出父项目，避免 `Rc` 循环打印）。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ViewState")
            .field("view_name", &self.m_view_name)
            .field("unique_id_list", &self.m_unique_id_list)
            .field("filter_id", &self.m_filter_id)
            .field("file_is_set", &(self.m_file.strong_count() > 0))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessors_round_trip() {
        let state = ViewState::new(Weak::new(), Some("Gantt Chart".to_owned()), vec![1, 2, 3], 7);

        assert_eq!(state.get_view_name(), Some("Gantt Chart"));
        assert_eq!(state.get_unique_id_list(), &[1, 2, 3]);
    }

    #[test]
    fn empty_unique_id_list_is_supported() {
        let state = ViewState::new(Weak::new(), None, Vec::new(), 0);

        assert_eq!(state.get_view_name(), None);
        assert!(state.get_unique_id_list().is_empty());
    }

    #[test]
    fn get_filter_returns_none_without_parent_file() {
        let state = ViewState::new(Weak::new(), Some("View".to_owned()), Vec::new(), 1);
        assert!(state.get_filter().is_none());
    }

    #[test]
    fn debug_reports_file_state() {
        let state = ViewState::new(Weak::new(), None, vec![1], 2);
        let text = format!("{state:?}");
        assert!(text.contains("filter_id: 2"));
        assert!(text.contains("file_is_set: false"));
    }
}
