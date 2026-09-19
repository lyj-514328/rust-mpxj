//! 对应 Java: `org/mpxj/Table.java`
//!
//! 跳过/未移植的 Java 方法与原因：
//!
//! | Java | Rust |
//! |---|---|
//! | 隐式默认构造方法 | [`Table::new`] |
//! | `public int getID()` / `public void setID(int id)` | [`Table::get_id`] / [`Table::set_id`] |
//! | `public String getName()` / `public void setName(String name)` | [`Table::get_name`] / [`Table::set_name`] |
//! | `public boolean getResourceFlag()` / `public void setResourceFlag(boolean flag)` | [`Table::get_resource_flag`] / [`Table::set_resource_flag`] |
//! | `public void addColumn(Column column)` | [`Table::add_column`] |
//! | `public List<Column> getColumns()` | [`Table::get_columns`]（不可变切片）与 [`Table::get_columns_mut`]（对应 Java 返回的可变列表） |
//! | `@Override public String toString()` | [`Display`] |
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//!
//! # 与 Java 的差异
//!
//! - **列的持有方式**：Java 的 `ArrayList<Column> m_columns` 直接持有列对象；Rust 侧同样按值
//!   持有 `Vec<Column>`（`Column` 只回指父项目，不回指 `Table`，不会形成 `Rc` 循环）。
//!   Java 的 `getColumns()` 返回列表本身（调用方可直接 `add`），Rust 侧按本 crate 的既有
//!   约定拆成 [`Table::get_columns`]（只读视图）与 [`Table::get_columns_mut`]（可变列表）。
//! - **`name`**：Java 允许 `null`，Rust 侧用 `Option<String>`；[`Display`] 把 `None`
//!   渲染为 `"null"`，与 Java 字符串拼接一致。
//! - **`equals` / `hashCode`**：Java 未覆写（对象同一性）；Rust 侧不实现 `PartialEq` / `Eq` / `Hash`。

use std::fmt::{self, Display, Formatter};

use crate::column::Column;

/// MPP 文件中的表格定义。
///
/// 对应 Java: `public final class Table`
///
/// MS Project 中大量信息以表格形式呈现，本类描述表格的属性（id、名称、任务表还是资源表、
/// 列定义），例如标准甘特图左侧表格的属性即由此定义。
#[derive(Debug)]
pub struct Table {
    /// 表格在本文件中的唯一标识（不代表表格类型）。
    ///
    /// 对应 Java: `private int m_id;`
    m_id: i32,
    /// 表格名称（MPXJ 已剥离内部使用的 `&` 快捷键标记）。
    ///
    /// 对应 Java: `private String m_name;`
    m_name: Option<String>,
    /// 资源表标志：`false` 表示任务表，`true` 表示资源表。
    ///
    /// 对应 Java: `private boolean m_resourceFlag;`
    m_resource_flag: bool,
    /// 表格的列定义。
    ///
    /// 对应 Java: `private final ArrayList<Column> m_columns = new ArrayList<>();`
    m_columns: Vec<Column>,
}

impl Table {
    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法（Java 侧未显式声明构造方法）。
    pub fn new() -> Self {
        Self {
            m_id: 0,
            m_name: None,
            m_resource_flag: false,
            m_columns: Vec::new(),
        }
    }

    /// 取表格标识。
    ///
    /// 对应 Java: `public int getID()`
    pub fn get_id(&self) -> i32 {
        self.m_id
    }

    /// 设置表格标识。
    ///
    /// 对应 Java: `public void setID(int id)`
    pub fn set_id(&mut self, id: i32) {
        self.m_id = id;
    }

    /// 取表格名称。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.m_name.as_deref()
    }

    /// 设置表格名称。
    ///
    /// 对应 Java: `public void setName(String name)`
    pub fn set_name(&mut self, name: Option<String>) {
        self.m_name = name;
    }

    /// 取资源表标志（`true` 表示资源数据，`false` 表示任务数据）。
    ///
    /// 对应 Java: `public boolean getResourceFlag()`
    pub fn get_resource_flag(&self) -> bool {
        self.m_resource_flag
    }

    /// 设置资源表标志。
    ///
    /// 对应 Java: `public void setResourceFlag(boolean flag)`
    pub fn set_resource_flag(&mut self, flag: bool) {
        self.m_resource_flag = flag;
    }

    /// 向表格追加一列。
    ///
    /// 对应 Java: `public void addColumn(Column column)`
    pub fn add_column(&mut self, column: Column) {
        self.m_columns.push(column);
    }

    /// 取表格的列定义。
    ///
    /// 对应 Java: `public List<Column> getColumns()`
    pub fn get_columns(&self) -> &[Column] {
        &self.m_columns
    }

    /// 以可变方式取表格的列定义（对应 Java 中 `getColumns()` 返回的可变列表）。
    ///
    /// 对应 Java: `public List<Column> getColumns()`
    pub fn get_columns_mut(&mut self) -> &mut Vec<Column> {
        &mut self.m_columns
    }
}

impl Default for Table {
    /// 对应 Java: 隐式默认构造方法，等价于 [`Table::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl Display for Table {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// 与 Java 一致地输出尾随换行（Java 用 `pw.println`）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[TABLE id={} name={} resourceFlag={}\n",
            self.m_id,
            self.m_name.as_deref().unwrap_or("null"),
            self.m_resource_flag
        )?;

        for column in &self.m_columns {
            write!(f, "   {column}")?;
        }

        writeln!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::Column;
    use crate::field_key::FieldKey;
    use crate::task_field::TaskField;
    use std::rc::Weak;

    #[test]
    fn accessors_round_trip() {
        let mut table = Table::new();
        assert_eq!(table.get_id(), 0);
        assert_eq!(table.get_name(), None);
        assert!(!table.get_resource_flag());
        assert!(table.get_columns().is_empty());

        table.set_id(7);
        table.set_name(Some("My Table".to_owned()));
        table.set_resource_flag(true);

        assert_eq!(table.get_id(), 7);
        assert_eq!(table.get_name(), Some("My Table"));
        assert!(table.get_resource_flag());
    }

    #[test]
    fn add_column_appends_to_live_list() {
        let mut table = Table::new();

        let mut column = Column::new(Weak::new());
        column.set_field_type(Some(FieldKey::Task(TaskField::Name)));
        table.add_column(column);

        assert_eq!(table.get_columns().len(), 1);

        // Java 的 getColumns() 返回可变列表
        table
            .get_columns_mut()
            .push(Column::new(Weak::new()));
        assert_eq!(table.get_columns().len(), 2);
    }

    #[test]
    fn default_matches_new() {
        let table = Table::default();
        assert_eq!(table.get_id(), 0);
        assert!(table.get_columns().is_empty());
    }

    #[test]
    fn display_matches_java_format() {
        let mut table = Table::new();
        table.set_id(1);
        table.set_name(Some("Entry".to_owned()));
        table.set_resource_flag(false);

        let mut column = Column::new(Weak::new());
        column.set_width(10);
        column.set_title(Some("Task".to_owned()));
        table.add_column(column);

        assert_eq!(
            table.to_string(),
            "[TABLE id=1 name=Entry resourceFlag=false\n   \
             [Column type=null width=10 titleAlignment=RIGHT dataAlignment=RIGHT title=Task]\n]\n"
        );
    }

    #[test]
    fn display_renders_null_name() {
        let table = Table::new();
        assert_eq!(
            table.to_string(),
            "[TABLE id=0 name=null resourceFlag=false\n]\n"
        );
    }
}
