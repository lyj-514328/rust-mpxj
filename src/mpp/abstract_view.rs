//! 对应 Java: `org/mpxj/mpp/AbstractView.java`
//!
//! 所有视图的公共基类：保存视图 ID、名称、类型与「表格部分」的名称，并按名称
//! 在当前项目的表格容器里查出对应的表实例。
//!
//! 跳过的 Java 机制与原因：
//!
//! | Java | 原因 |
//! |---|---|
//! | 反射 / JDBC / XML(JAXB) / GUI / `javax.*` | 本类未使用 |
//! | `Serializable` | Rust 无该机制（本类在 Java 侧也未实现） |
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `protected final ProjectFile m_file` | 私有字段 `m_file: Weak<RefCell<ProjectFile>>` | `ProjectFile` 通过 `ViewContainer` 持有视图，视图再用强引用回指父文件会形成 `Rc` 循环，故按要求用弱引用；父文件已释放 / 正被可变借用时，[`AbstractView::get_table`] 返回 `None`（Java 会抛 `NullPointerException`） |
//! | `protected final TableContainer m_tables = parent.getTables()` | 不缓存，按需经父文件取 | Rust 侧 `TableContainer` 由 `ProjectFile` 内联持有（不是 `Rc`），无法在视图里长期借用，故每次经 `m_file` 解析（语义与 Java 相同） |
//! | `protected Integer m_id` 等 4 个可写字段 | 私有字段 + [`AbstractView::set_id`] / [`AbstractView::set_name`] / [`AbstractView::set_type`] / [`AbstractView::set_table_name`] | Rust 无继承，子类（如 `GenericView`、`View8`）无法直接写父类字段，改以 setter 表达 Java 的 protected 字段赋值 |
//! | `implements View` | `impl View for AbstractView` | 与 [`crate::view::View`] trait 一一对应 |
//! | `toString()` | `impl Display` | Java 用字符串拼接，`null` 渲染为 `"null"`，Rust 侧保持一致 |
//!
//! # 继承的还原方式：组合 + 转发
//!
//! Java 的 `AbstractView implements View` 是**被继承的基类**：`AbstractMppView extends
//! AbstractView`。Rust 无继承，`AbstractMppView`（见 `abstract_mpp_view.rs`）改为**持有**
//! 一个本类型的实例并把 5 个 `View` 方法与 4 个 setter 全部转发，语义等价。
//! 本类自身没有抽象方法，因此可直接实例化。

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::{Rc, Weak};

use crate::project_file::ProjectFile;
use crate::table::Table;
use crate::view::View;
use crate::view_type::ViewType;

/// 所有视图的公共基类。
///
/// 对应 Java: `public abstract class AbstractView implements View`
pub struct AbstractView {
    /// 父项目文件。
    ///
    /// 对应 Java 字段: `protected final ProjectFile m_file;`
    m_file: Weak<RefCell<ProjectFile>>,

    /// 视图 ID。
    ///
    /// 对应 Java 字段: `protected Integer m_id;`
    m_id: Option<i32>,

    /// 视图名称。
    ///
    /// 对应 Java 字段: `protected String m_name;`
    m_name: Option<String>,

    /// 视图类型。
    ///
    /// 对应 Java 字段: `protected ViewType m_type;`
    m_type: Option<ViewType>,

    /// 视图中表格部分的名称。
    ///
    /// 对应 Java 字段: `protected String m_tableName;`
    m_table_name: Option<String>,
}

impl AbstractView {
    /// 构造方法。
    ///
    /// 对应 Java: `public AbstractView(ProjectFile parent)`
    ///
    /// Java 侧同时把 `parent.getTables()` 缓存在 `m_tables`；Rust 侧不缓存，见文件头说明。
    pub fn new(parent: &Rc<RefCell<ProjectFile>>) -> Self {
        Self {
            m_file: Rc::downgrade(parent),
            m_id: None,
            m_name: None,
            m_type: None,
            m_table_name: None,
        }
    }

    /// 取父项目文件。
    ///
    /// Rust 特有：Java 的 `m_file` 是 protected 字段，由子类直接读取；父文件已释放时返回
    /// `None`。
    pub fn get_file(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.m_file.upgrade()
    }

    /// 设置视图 ID。
    ///
    /// Rust 特有：对应 Java 子类对 `protected Integer m_id` 的赋值。
    pub fn set_id(&mut self, id: Option<i32>) {
        self.m_id = id;
    }

    /// 设置视图名称。
    ///
    /// Rust 特有：对应 Java 子类对 `protected String m_name` 的赋值。
    pub fn set_name(&mut self, name: Option<String>) {
        self.m_name = name;
    }

    /// 设置视图类型。
    ///
    /// Rust 特有：对应 Java 子类对 `protected ViewType m_type` 的赋值。
    pub fn set_type(&mut self, type_: Option<ViewType>) {
        self.m_type = type_;
    }

    /// 设置视图中表格部分的名称。
    ///
    /// Rust 特有：对应 Java 子类对 `protected String m_tableName` 的赋值。
    pub fn set_table_name(&mut self, table_name: Option<String>) {
        self.m_table_name = table_name;
    }
}

impl View for AbstractView {
    /// 对应 Java: `@Override public Integer getID()`
    fn get_id(&self) -> Option<i32> {
        self.m_id
    }

    /// 对应 Java: `@Override public String getName()`
    fn get_name(&self) -> Option<&str> {
        self.m_name.as_deref()
    }

    /// 对应 Java: `@Override public ViewType getType()`
    fn get_type(&self) -> Option<ViewType> {
        self.m_type
    }

    /// 对应 Java: `@Override public String getTableName()`
    fn get_table_name(&self) -> Option<&str> {
        self.m_table_name.as_deref()
    }

    /// 取视图中表格部分对应的表实例。
    ///
    /// 对应 Java: `@Override public Table getTable()`
    ///
    /// Java: `return m_tables.getTaskTableByName(m_tableName);`
    /// 父文件已释放或正被可变借用时返回 `None`（Java 会抛 `NullPointerException`）。
    fn get_table(&self) -> Option<Rc<RefCell<Table>>> {
        let file = self.m_file.upgrade()?;
        let file_ref = file.try_borrow().ok()?;

        file_ref
            .get_tables()
            .get_task_table_by_name(self.m_table_name.as_deref())
    }
}

impl Display for AbstractView {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java: `"[View id=" + m_id + " type=" + m_type + " name=" + m_name
    /// + (m_tableName == null ? "" : " table=" + m_tableName) + "]"`
    ///
    /// 拼接时 Java 把 `null` 渲染为 `"null"`（`ViewType` 走其 `toString()`，即常量名）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[View id={} type={} name={}", java_int(self.m_id), java_view_type(self.m_type), java_str(self.m_name.as_deref()))?;

        if let Some(table_name) = &self.m_table_name {
            write!(f, " table={table_name}")?;
        }

        f.write_str("]")
    }
}

/// 按 Java 的字符串拼接规则渲染可空整数（`null` → `"null"`）。
fn java_int(value: Option<i32>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

/// 按 Java 的字符串拼接规则渲染可空 `ViewType`（`null` → `"null"`，否则用 `toString()`）。
fn java_view_type(value: Option<ViewType>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

/// 按 Java 的字符串拼接规则渲染可空字符串（`null` → `"null"`）。
fn java_str(value: Option<&str>) -> &str {
    value.unwrap_or("null")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个带「Entry」表与「Resource」表的项目文件。
    fn file_with_tables() -> Rc<RefCell<ProjectFile>> {
        let file = ProjectFile::new();

        let mut entry = Table::new();
        entry.set_name(Some("Entry".to_owned()));
        let mut resource = Table::new();
        resource.set_name(Some("Resource".to_owned()));

        {
            let mut file_ref = file.borrow_mut();
            let tables = file_ref.get_tables_mut();
            tables.add(Rc::new(RefCell::new(entry)));
            tables.add(Rc::new(RefCell::new(resource)));
        }

        file
    }

    /// 构造方法、访问器与 `toString()` 的输出与 Java 一致。
    #[test]
    fn accessors_and_display_match_java() {
        let file = file_with_tables();
        let mut view = AbstractView::new(&file);
        view.set_id(Some(3));
        view.set_name(Some("Gantt Chart".to_owned()));
        view.set_type(Some(ViewType::GanttChart));
        view.set_table_name(Some("Entry".to_owned()));

        assert_eq!(view.get_id(), Some(3));
        assert_eq!(view.get_name(), Some("Gantt Chart"));
        assert_eq!(view.get_type(), Some(ViewType::GanttChart));
        assert_eq!(view.get_table_name(), Some("Entry"));
        assert_eq!(
            view.to_string(),
            "[View id=3 type=GANTT_CHART name=Gantt Chart table=Entry]"
        );
    }

    /// 未赋值的字段在 `toString()` 中渲染成 `"null"`，且不输出 `table=`。
    #[test]
    fn empty_view_display_matches_java() {
        let file = file_with_tables();
        let view = AbstractView::new(&file);

        assert_eq!(view.get_id(), None);
        assert_eq!(view.get_name(), None);
        assert_eq!(view.get_type(), None);
        assert_eq!(view.get_table_name(), None);
        assert_eq!(view.to_string(), "[View id=null type=null name=null]");
    }

    /// `getTable()` 按名称命中表格容器中的任务表。
    #[test]
    fn get_table_resolves_by_name() {
        let file = file_with_tables();
        let mut view = AbstractView::new(&file);
        view.set_table_name(Some("Entry".to_owned()));

        let expected = file
            .borrow()
            .get_tables()
            .get_task_table_by_name(Some("Entry"))
            .expect("表中存在 Entry");
        let found = view.get_table().expect("应能查到表");
        assert!(Rc::ptr_eq(&found, &expected));

        // 名称不存在 / 未设置名称 → None
        view.set_table_name(Some("Missing".to_owned()));
        assert!(view.get_table().is_none());
        view.set_table_name(None);
        assert!(view.get_table().is_none());
    }

    /// 父文件被释放后取表返回 `None`（Java 会抛 `NullPointerException`），不 panic。
    #[test]
    fn dropped_parent_yields_none() {
        let file = file_with_tables();
        let mut view = AbstractView::new(&file);
        view.set_table_name(Some("Entry".to_owned()));

        assert!(view.get_file().is_some());
        drop(file);

        assert!(view.get_file().is_none());
        assert!(view.get_table().is_none());
    }
}
