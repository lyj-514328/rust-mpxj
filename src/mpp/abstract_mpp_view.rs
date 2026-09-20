//! 对应 Java: `org/mpxj/mpp/AbstractMppView.java`
//!
//! 所有 MPP 视图的公共基类。Java 侧本类只提供一个构造方法（把 `parent` 交给 `AbstractView`），
//! 没有任何自有状态与行为。
//!
//! 跳过的 Java 机制与原因：
//!
//! | Java | 原因 |
//! |---|---|
//! | 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable` | 本类未使用 |
//!
//! # 继承的还原方式：组合 + 转发
//!
//! | Java | Rust |
//! |---|---|
//! | `AbstractMppView extends AbstractView` | [`AbstractMppView`] **持有**一个 [`AbstractView`]，并把 `View` 的 5 个方法与 4 个 setter 逐个转发（见各自的 `对应 Java:` 注释） |
//! | `public AbstractMppView(ProjectFile parent)` | [`AbstractMppView::new`] |
//! | `parent` 由 `AbstractView` 持有 | 同 [`AbstractView`]（`Weak<RefCell<ProjectFile>>`，避免 `Rc` 循环） |
//!
//! 与 Java 的差异：Java 的 `m_file` / `m_id` / `m_name` / `m_type` / `m_tableName` 是
//! `AbstractView` 的 protected 字段，子类（`View8`、`GenericView`）直接读写；Rust 侧改由
//! [`AbstractMppView::set_id`] 等 setter 与 [`AbstractMppView::as_abstract_view`]（需要底层
//! 实例时）表达。

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use crate::mpp::abstract_view::AbstractView;
use crate::project_file::ProjectFile;
use crate::table::Table;
use crate::view::View;
use crate::view_type::ViewType;

/// 所有 MPP 视图的公共基类。
///
/// 对应 Java: `public abstract class AbstractMppView extends AbstractView`
pub struct AbstractMppView {
    /// 对应 Java: `extends AbstractView`（Rust 用组合代替继承）
    view: AbstractView,
}

impl AbstractMppView {
    /// 构造方法。
    ///
    /// 对应 Java: `public AbstractMppView(ProjectFile parent)`
    ///
    /// Java: `super(parent);`
    pub fn new(parent: &Rc<RefCell<ProjectFile>>) -> Self {
        Self {
            view: AbstractView::new(parent),
        }
    }

    /// 取父项目文件。
    ///
    /// 对应 Java: `AbstractView.m_file`（protected 字段）
    pub fn get_file(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.view.get_file()
    }

    /// 设置视图 ID。
    ///
    /// 对应 Java: `AbstractView.m_id`（protected 字段赋值）
    pub fn set_id(&mut self, id: Option<i32>) {
        self.view.set_id(id);
    }

    /// 设置视图名称。
    ///
    /// 对应 Java: `AbstractView.m_name`（protected 字段赋值）
    pub fn set_name(&mut self, name: Option<String>) {
        self.view.set_name(name);
    }

    /// 设置视图类型。
    ///
    /// 对应 Java: `AbstractView.m_type`（protected 字段赋值）
    pub fn set_type(&mut self, type_: Option<ViewType>) {
        self.view.set_type(type_);
    }

    /// 设置视图中表格部分的名称。
    ///
    /// 对应 Java: `AbstractView.m_tableName`（protected 字段赋值）
    pub fn set_table_name(&mut self, table_name: Option<String>) {
        self.view.set_table_name(table_name);
    }

    /// 取内部持有的基类实例。
    ///
    /// Rust 特有：Java 的继承在 Rust 侧由组合表达，需要访问基类实例时用本方法。
    pub fn as_abstract_view(&self) -> &AbstractView {
        &self.view
    }

    /// 取内部持有的基类实例（可变）。
    ///
    /// Rust 特有：见 [`AbstractMppView::as_abstract_view`]。
    pub fn as_abstract_view_mut(&mut self) -> &mut AbstractView {
        &mut self.view
    }
}

impl View for AbstractMppView {
    /// 对应 Java: `AbstractView.getID()`（继承）
    fn get_id(&self) -> Option<i32> {
        self.view.get_id()
    }

    /// 对应 Java: `AbstractView.getName()`（继承）
    fn get_name(&self) -> Option<&str> {
        self.view.get_name()
    }

    /// 对应 Java: `AbstractView.getType()`（继承）
    fn get_type(&self) -> Option<ViewType> {
        self.view.get_type()
    }

    /// 对应 Java: `AbstractView.getTableName()`（继承）
    fn get_table_name(&self) -> Option<&str> {
        self.view.get_table_name()
    }

    /// 对应 Java: `AbstractView.getTable()`（继承）
    fn get_table(&self) -> Option<Rc<RefCell<Table>>> {
        self.view.get_table()
    }
}

impl Display for AbstractMppView {
    /// 对应 Java: `AbstractView.toString()`（本类未覆写）
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.view, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个带「Entry」表的项目文件。
    fn file_with_table() -> Rc<RefCell<ProjectFile>> {
        let file = ProjectFile::new();
        let mut table = Table::new();
        table.set_name(Some("Entry".to_owned()));
        file.borrow_mut()
            .get_tables_mut()
            .add(Rc::new(RefCell::new(table)));
        file
    }

    /// setter / getter 与 `toString()` 全部转发给内部基类实例。
    #[test]
    fn forwards_to_abstract_view() {
        let file = file_with_table();
        let mut view = AbstractMppView::new(&file);
        view.set_id(Some(1));
        view.set_name(Some("Resource Sheet".to_owned()));
        view.set_type(Some(ViewType::ResourceSheet));
        view.set_table_name(Some("Entry".to_owned()));

        assert_eq!(view.get_id(), Some(1));
        assert_eq!(view.get_name(), Some("Resource Sheet"));
        assert_eq!(view.get_type(), Some(ViewType::ResourceSheet));
        assert_eq!(view.get_table_name(), Some("Entry"));
        assert!(view.get_table().is_some());
        assert_eq!(
            view.to_string(),
            "[View id=1 type=RESOURCE_SHEET name=Resource Sheet table=Entry]"
        );
    }

    /// `as_abstract_view_mut` 与转发方法操作的是同一个实例。
    #[test]
    fn exposes_inner_abstract_view() {
        let file = file_with_table();
        let mut view = AbstractMppView::new(&file);

        view.as_abstract_view_mut().set_id(Some(9));
        assert_eq!(view.as_abstract_view().get_id(), Some(9));
        assert_eq!(view.get_id(), Some(9));
    }

    /// 未设置名称/类型时 `toString()` 渲染 `null` 且不 panic。
    #[test]
    fn empty_view_is_supported() {
        let file = file_with_table();
        let mut view = AbstractMppView::new(&file);

        assert_eq!(view.to_string(), "[View id=null type=null name=null]");
        assert!(view.get_file().is_some());
        view.set_table_name(Some("Missing".to_owned()));
        assert!(view.get_table().is_none());
    }
}
