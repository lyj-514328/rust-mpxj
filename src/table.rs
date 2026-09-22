//! Corresponds to Java: `org/mpxj/Table.java`
//!
//! Skipped/unported Java methods and reasons:
//!
//! | Java | Rust |
//! |---|---|
//! | implicit default constructor | [`Table::new`] |
//! | `public int getID()` / `public void setID(int id)` | [`Table::get_id`] / [`Table::set_id`] |
//! | `public String getName()` / `public void setName(String name)` | [`Table::get_name`] / [`Table::set_name`] |
//! | `public boolean getResourceFlag()` / `public void setResourceFlag(boolean flag)` | [`Table::get_resource_flag`] / [`Table::set_resource_flag`] |
//! | `public void addColumn(Column column)` | [`Table::add_column`] |
//! | `public List<Column> getColumns()` | [`Table::get_columns`] (immutable slice) and [`Table::get_columns_mut`] (corresponds to the mutable list returned by Java) |
//! | `@Override public String toString()` | [`Display`] |
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: not used by this class, nothing skipped.
//!
//! # Differences from Java
//!
//! - **How columns are held**: Java's `ArrayList<Column> m_columns` holds column objects directly; Rust likewise holds
//! a `Vec<Column>` by value (`Column` only refers back to the parent project, not to `Table`, so no `Rc` cycle is formed).
//! Java's `getColumns()` returns the list itself (callers can `add` to it directly); Rust follows this crate's existing
//! convention of splitting it into [`Table::get_columns`] (read-only view) and [`Table::get_columns_mut`] (mutable list).
//! - **`name`**: Java allows `null`; Rust uses `Option<String>`; [`Display`] renders `None`
//! as `"null"`, matching Java string concatenation.
//! - **`equals` / `hashCode`**: not overridden in Java (object identity); Rust does not implement `PartialEq` / `Eq` / `Hash`.

use std::fmt::{self, Display, Formatter};

use crate::column::Column;

/// Table definition in an MPP file.
///
/// Corresponds to Java: `public final class Table`
///
/// Much information in MS Project is presented in tables; this class describes a table's attributes (id, name, whether it is a task or resource table,
/// column definitions); for example, the attributes of the standard Gantt chart's left-hand table are defined by this.
#[derive(Debug)]
pub struct Table {
/// Unique ID of the table within this file (does not denote the table type).
    ///
    /// Corresponds to Java: `private int m_id;`
    m_id: i32,
/// Table name (MPXJ has already stripped the internal `&` accelerator marker).
    ///
    /// Corresponds to Java: `private String m_name;`
    m_name: Option<String>,
/// Resource table flag: `false` means a task table, `true` means a resource table.
    ///
    /// Corresponds to Java: `private boolean m_resourceFlag;`
    m_resource_flag: bool,
/// Column definitions of the table.
    ///
    /// Corresponds to Java: `private final ArrayList<Column> m_columns = new ArrayList<>();`
    m_columns: Vec<Column>,
}

impl Table {
/// Constructor.
    ///
/// Corresponds to Java: implicit default constructor (Java does not explicitly declare a constructor).
    pub fn new() -> Self {
        Self {
            m_id: 0,
            m_name: None,
            m_resource_flag: false,
            m_columns: Vec::new(),
        }
    }

/// Gets the table ID.
    ///
    /// Corresponds to Java: `public int getID()`
    pub fn get_id(&self) -> i32 {
        self.m_id
    }

/// Sets the table ID.
    ///
    /// Corresponds to Java: `public void setID(int id)`
    pub fn set_id(&mut self, id: i32) {
        self.m_id = id;
    }

/// Gets the table name.
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.m_name.as_deref()
    }

/// Sets the table name.
    ///
    /// Corresponds to Java: `public void setName(String name)`
    pub fn set_name(&mut self, name: Option<String>) {
        self.m_name = name;
    }

/// Gets the resource table flag (`true` means resource data, `false` means task data).
    ///
    /// Corresponds to Java: `public boolean getResourceFlag()`
    pub fn get_resource_flag(&self) -> bool {
        self.m_resource_flag
    }

/// Sets the resource table flag.
    ///
    /// Corresponds to Java: `public void setResourceFlag(boolean flag)`
    pub fn set_resource_flag(&mut self, flag: bool) {
        self.m_resource_flag = flag;
    }

/// Appends a column to the table.
    ///
    /// Corresponds to Java: `public void addColumn(Column column)`
    pub fn add_column(&mut self, column: Column) {
        self.m_columns.push(column);
    }

/// Gets the column definitions of the table.
    ///
    /// Corresponds to Java: `public List<Column> getColumns()`
    pub fn get_columns(&self) -> &[Column] {
        &self.m_columns
    }

/// Gets the column definitions of the table as mutable (corresponds to the mutable list returned by `getColumns()` in Java).
    ///
    /// Corresponds to Java: `public List<Column> getColumns()`
    pub fn get_columns_mut(&mut self) -> &mut Vec<Column> {
        &mut self.m_columns
    }
}

impl Default for Table {
/// Corresponds to Java: implicit default constructor, equivalent to [`Table::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl Display for Table {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Emits a trailing newline as Java does (Java uses `pw.println`).
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

// Java's getColumns() returns a mutable list
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
