//! 对应 Java: `org/mpxj/Group.java`
//!
//! 跳过/未移植的 Java 方法与原因：
//!
//! | Java | Rust |
//! |---|---|
//! | `public Group(Integer id, String name, boolean showSummaryTasks)` | [`Group::new`] |
//! | `public Integer getID()` | [`Group::get_id`] |
//! | `public String getName()` | [`Group::get_name`] |
//! | `public boolean getShowSummaryTasks()` | [`Group::get_show_summary_tasks`] |
//! | `public void addGroupClause(GroupClause clause)` | [`Group::add_group_clause`] |
//! | `public List<GroupClause> getGroupClauses()` | [`Group::get_group_clauses`]（不可变切片）与 [`Group::get_group_clauses_mut`]（对应 Java 返回的可变列表） |
//! | `@Override public String toString()` | [`Display`] |
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过
//! （其成员 `GroupClause` 内部对 GUI / `org.mpxj.mpp` 类型的引用已在
//! `crate::group_clause` 的文件头说明）。
//!
//! # 与 Java 的差异
//!
//! - **不可变字段**：Java 的 `m_id` / `m_name` / `m_showSummaryTasks` 均为 `final`，
//!   只能经构造方法设置；Rust 侧保持同样形态（字段私有、无 setter）。
//! - **`getGroupClauses()`**：Java 返回可变的 `ArrayList`；Rust 侧按本 crate 的既有约定
//!   拆成只读切片与 [`Group::get_group_clauses_mut`]。
//! - **`toString()` 的换行**：Java 全部用 `pw.println`，输出含尾随换行；Rust 的 [`Display`]
//!   保持同样的输出。
//! - **`equals` / `hashCode`**：Java 未覆写（对象同一性）；Rust 侧不实现
//!   `PartialEq` / `Eq` / `Hash`。

use std::fmt::{self, Display, Formatter};

use crate::group_clause::GroupClause;

/// 视图中用于组织数据的分组定义。
///
/// 对应 Java: `public final class Group`
pub struct Group {
    /// 分组标识。
    ///
    /// 对应 Java: `private final Integer m_id;`
    m_id: Option<i32>,

    /// 分组名称。
    ///
    /// 对应 Java: `private final String m_name;`
    m_name: Option<String>,

    /// 是否显示摘要任务。
    ///
    /// 对应 Java: `private final boolean m_showSummaryTasks;`
    m_show_summary_tasks: bool,

    /// 分组子句。
    ///
    /// 对应 Java: `private final List<GroupClause> m_clauses = new ArrayList<>();`
    m_clauses: Vec<GroupClause>,
}

impl Group {
    /// 构造方法。
    ///
    /// 对应 Java: `public Group(Integer id, String name, boolean showSummaryTasks)`
    pub fn new(id: Option<i32>, name: Option<String>, show_summary_tasks: bool) -> Self {
        Self {
            m_id: id,
            m_name: name,
            m_show_summary_tasks: show_summary_tasks,
            m_clauses: Vec::new(),
        }
    }

    /// 取分组标识。
    ///
    /// 对应 Java: `public Integer getID()`
    pub fn get_id(&self) -> Option<i32> {
        self.m_id
    }

    /// 取分组名称。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.m_name.as_deref()
    }

    /// 取「是否显示摘要任务」标志。
    ///
    /// 对应 Java: `public boolean getShowSummaryTasks()`
    pub fn get_show_summary_tasks(&self) -> bool {
        self.m_show_summary_tasks
    }

    /// 追加一条分组子句。
    ///
    /// 对应 Java: `public void addGroupClause(GroupClause clause)`
    pub fn add_group_clause(&mut self, clause: GroupClause) {
        self.m_clauses.push(clause);
    }

    /// 取全部分组子句。
    ///
    /// 对应 Java: `public List<GroupClause> getGroupClauses()`
    pub fn get_group_clauses(&self) -> &[GroupClause] {
        &self.m_clauses
    }

    /// 以可变方式取全部分组子句（对应 Java 返回的可变列表）。
    ///
    /// 对应 Java: `public List<GroupClause> getGroupClauses()`
    pub fn get_group_clauses_mut(&mut self) -> &mut Vec<GroupClause> {
        &mut self.m_clauses
    }
}

impl Display for Group {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// 与 Java 一致地逐行输出（含尾随换行）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut result = String::from("[Group \n");

        result.push_str(&format!(
            " id={}\n",
            self.m_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "null".to_owned())
        ));
        result.push_str(&format!(
            " name={}\n",
            self.m_name.as_deref().unwrap_or("null")
        ));
        result.push_str(&format!(
            " showSummaryTasks={}\n",
            self.m_show_summary_tasks
        ));
        result.push_str(" [Clauses=\n");

        for clause in &self.m_clauses {
            result.push_str(&format!("  {clause}\n"));
        }

        result.push_str(" ]\n");
        result.push_str("]\n");

        f.write_str(&result)
    }
}

impl fmt::Debug for Group {
    /// Java 未覆写调试输出；Rust 侧提供 `Debug`（复用 [`Display`]）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_key::FieldKey;
    use crate::task_field::TaskField;

    #[test]
    fn accessors_round_trip() {
        let group = Group::new(Some(3), Some("My Group".to_owned()), true);

        assert_eq!(group.get_id(), Some(3));
        assert_eq!(group.get_name(), Some("My Group"));
        assert!(group.get_show_summary_tasks());
        assert!(group.get_group_clauses().is_empty());
    }

    #[test]
    fn add_group_clause_appends_to_live_list() {
        let mut group = Group::new(None, None, false);

        let mut clause = GroupClause::new();
        clause.set_field(Some(FieldKey::Task(TaskField::Name)));
        group.add_group_clause(clause);

        assert_eq!(group.get_group_clauses().len(), 1);

        group.get_group_clauses_mut().push(GroupClause::new());
        assert_eq!(group.get_group_clauses().len(), 2);
    }

    #[test]
    fn display_matches_java_format() {
        let mut group = Group::new(Some(1), Some("G".to_owned()), false);

        let mut clause = GroupClause::new();
        clause.set_field(Some(FieldKey::Task(TaskField::Name)));
        clause.set_ascending(true);
        group.add_group_clause(clause);

        let expected = concat!(
            "[Group \n",
            " id=1\n",
            " name=G\n",
            " showSummaryTasks=false\n",
            " [Clauses=\n",
            "  [GroupClause field=Task Name ascending=true font=null color=null pattern=null groupOn=0 startAt=null groupInterval=null\n",
            " ]\n",
            "]\n",
        );

        assert_eq!(group.to_string(), expected);
    }

    #[test]
    fn display_renders_nulls() {
        let group = Group::new(None, None, true);
        let expected = concat!(
            "[Group \n",
            " id=null\n",
            " name=null\n",
            " showSummaryTasks=true\n",
            " [Clauses=\n",
            " ]\n",
            "]\n",
        );

        assert_eq!(group.to_string(), expected);
    }
}
