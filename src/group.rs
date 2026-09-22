//! Corresponds to Java: `org/mpxj/Group.java`
//!
//! Skipped / unported Java methods and reasons:
//!
//! | Java | Rust |
//! |---|---|
//! | `public Group(Integer id, String name, boolean showSummaryTasks)` | [`Group::new`] |
//! | `public Integer getID()` | [`Group::get_id`] |
//! | `public String getName()` | [`Group::get_name`] |
//! | `public boolean getShowSummaryTasks()` | [`Group::get_show_summary_tasks`] |
//! | `public void addGroupClause(GroupClause clause)` | [`Group::add_group_clause`] |
//! | `public List<GroupClause> getGroupClauses()` | [`Group::get_group_clauses`] (immutable slice) and [`Group::get_group_clauses_mut`] (corresponding to the mutable list Java returns) |
//! | `@Override public String toString()` | [`Display`] |
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: not used by this class, nothing skipped
//! (the references to GUI / `org.mpxj.mpp` types inside its member `GroupClause` are documented in
//! the file header of `crate::group_clause`).
//!
//! # Differences from Java
//!
//! - **Immutable fields**: Java's `m_id` / `m_name` / `m_showSummaryTasks` are all `final` and
//! can only be set via the constructor; Rust keeps the same shape (private fields, no setters).
//! - **`getGroupClauses()`**: Java returns a mutable `ArrayList`; following this crate's existing convention, the Rust side
//! splits it into a read-only slice and [`Group::get_group_clauses_mut`].
//! - **Newlines in `toString()`**: Java uses `pw.println` throughout, so the output has a trailing newline; Rust's [`Display`]
//! keeps the same output.
//! - **`equals` / `hashCode`**: Java does not override them (object identity); the Rust side does not implement
//!   `PartialEq` / `Eq` / `Hash`.

use std::fmt::{self, Display, Formatter};

use crate::group_clause::GroupClause;

/// A group definition used to organize data in a view.
///
/// Corresponds to Java: `public final class Group`
pub struct Group {
/// Group ID.
    ///
    /// Corresponds to Java: `private final Integer m_id;`
    m_id: Option<i32>,

/// Group name.
    ///
    /// Corresponds to Java: `private final String m_name;`
    m_name: Option<String>,

/// Whether to show summary tasks.
    ///
    /// Corresponds to Java: `private final boolean m_showSummaryTasks;`
    m_show_summary_tasks: bool,

/// Group clauses.
    ///
    /// Corresponds to Java: `private final List<GroupClause> m_clauses = new ArrayList<>();`
    m_clauses: Vec<GroupClause>,
}

impl Group {
/// Constructor.
    ///
    /// Corresponds to Java: `public Group(Integer id, String name, boolean showSummaryTasks)`
    pub fn new(id: Option<i32>, name: Option<String>, show_summary_tasks: bool) -> Self {
        Self {
            m_id: id,
            m_name: name,
            m_show_summary_tasks: show_summary_tasks,
            m_clauses: Vec::new(),
        }
    }

/// Get the group ID.
    ///
    /// Corresponds to Java: `public Integer getID()`
    pub fn get_id(&self) -> Option<i32> {
        self.m_id
    }

/// Get the group name.
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.m_name.as_deref()
    }

/// Get the "show summary tasks" flag.
    ///
    /// Corresponds to Java: `public boolean getShowSummaryTasks()`
    pub fn get_show_summary_tasks(&self) -> bool {
        self.m_show_summary_tasks
    }

/// Append a group clause.
    ///
    /// Corresponds to Java: `public void addGroupClause(GroupClause clause)`
    pub fn add_group_clause(&mut self, clause: GroupClause) {
        self.m_clauses.push(clause);
    }

/// Get all group clauses.
    ///
    /// Corresponds to Java: `public List<GroupClause> getGroupClauses()`
    pub fn get_group_clauses(&self) -> &[GroupClause] {
        &self.m_clauses
    }

/// Get all group clauses mutably (corresponding to the mutable list Java returns).
    ///
    /// Corresponds to Java: `public List<GroupClause> getGroupClauses()`
    pub fn get_group_clauses_mut(&mut self) -> &mut Vec<GroupClause> {
        &mut self.m_clauses
    }
}

impl Display for Group {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Output line by line consistently with Java (including the trailing newline).
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
/// Java does not override debug output; the Rust side provides `Debug` (reusing [`Display`]).
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
