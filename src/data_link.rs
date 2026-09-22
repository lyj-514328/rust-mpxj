//! Corresponds to Java: `org/mpxj/DataLink.java`
//!
//! Represents a link between two fields, either within the same project or across projects. Usually one record
//! gives both source and target, but sometimes it is split into two records (source in one, target in the other). In addition,
//! when a link is removed, a record with "only a source, no target" may be left behind.
//!
//! # Java -> Rust method mapping
//!
//! | Java | Rust |
//! |---|---|
//! | `public DataLink(String id)` | [`DataLink::new`] |
//! | `public String getID()` | [`DataLink::get_nullable_id`]; the legacy-API-compatible [`DataLink::get_id`] maps `null` to the empty string |
//! | `public FieldType getSourceField()` | [`DataLink::get_source_field`] (returns `Option<FieldKey>`, see below) |
//! | `public void setSourceField(FieldType)` | [`DataLink::set_source_field`] |
//! | `public Integer getSourceUniqueID()` | [`DataLink::get_source_unique_id`] |
//! | `public void setSourceUniqueID(Integer)` | [`DataLink::set_source_unique_id`] |
//! | `public FieldType getTargetField()` | [`DataLink::get_target_field`] |
//! | `public void setTargetField(FieldType)` | [`DataLink::set_target_field`] |
//! | `public Integer getTargetUniqueID()` | [`DataLink::get_target_unique_id`] |
//! | `public void setTargetUniqueID(Integer)` | [`DataLink::set_target_unique_id`] |
//! | `@Override public String toString()` | `impl Display for DataLink` |
//!
//! # Differences from Java
//!
//! - **`FieldType` -> [`FieldKey`]**: Java's `getSourceField()` / `getTargetField()`
//! return the `FieldType` interface; Rust follows this crate's convention and uses its runtime-erased form
//! [`FieldKey`] (see `crate::field_key`), and reproduces the output of `FieldTypeClass.name()` in Java's
//! `toString()` using `FieldType::get_field_type_class` / `FieldType::name`.
//! - **Nullable fields -> `Option<T>`**: Java fields may be `null` (`Integer` -> `Option<i32>`,
//!   `FieldType` → `Option<FieldKey>`).
//! - **`get_id` compatibility entry point**: Java's `m_id` may also be `null`; the model keeps
//! `Option<String>`, and the legacy `get_id()` represents `None` as the empty string for API compatibility;
//! call `get_nullable_id()` for exact semantics.
//! - **`PartialEq` / `Eq` / `Hash` are not derived**: Java's `DataLink` does not override
//! `equals` / `hashCode` (it uses object identity), so Rust does not provide structural equality,
//! and "remove by reference equality" in the container is expressed by [`crate::data_link_container::DataLinkContainer`]
//! using pointer identity.
//! - **`toString()` -> [`Display`]**: the output order matches Java's `StringBuilder` concatenation;
//! Java prints `"null"` for nullable integer fields, and Rust reproduces this with [`format_optional_integer`].
//!
//! # Skipped Java mechanisms
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::fmt::{self, Display, Formatter};

use crate::field_key::FieldKey;
use crate::field_type::FieldType;

/// A link between two fields.
///
/// Corresponds to Java: `public final class DataLink`
#[derive(Debug, Clone)]
pub struct DataLink {
/// Link identifier.
    ///
    /// Corresponds to Java: `private final String m_id;`
    id: Option<String>,

/// Source field.
    ///
    /// Corresponds to Java: `private FieldType m_sourceField;`
    source_field: Option<FieldKey>,

/// Unique ID of the source object.
    ///
    /// Corresponds to Java: `private Integer m_sourceUniqueID;`
    source_unique_id: Option<i32>,

/// Target field.
    ///
    /// Corresponds to Java: `private FieldType m_targetField;`
    target_field: Option<FieldKey>,

/// Unique ID of the target object.
    ///
    /// Corresponds to Java: `private Integer m_targetUniqueID;`
    target_unique_id: Option<i32>,
}

impl DataLink {
/// Constructor.
    ///
    /// Corresponds to Java: `public DataLink(String id)`
    pub fn new(id: impl Into<String>) -> Self {
        Self::new_nullable(Some(id.into()))
    }

/// Construct a link that allows a null identifier, preserving Java's distinction between `null` and the empty string.
    ///
    /// Corresponds to Java: `public DataLink(String id)`(`id == null`).
    pub fn new_nullable(id: Option<String>) -> Self {
        Self {
            id,
            source_field: None,
            source_unique_id: None,
            target_field: None,
            target_unique_id: None,
        }
    }

/// Get the link identifier.
    ///
    /// Corresponds to Java: `public String getID()`
    pub fn get_id(&self) -> &str {
        self.id.as_deref().unwrap_or("")
    }

/// Get the nullable identifier, for callers that need to distinguish Java `null` from the empty string.
    ///
    /// Corresponds to Java: `public String getID()`.
    pub fn get_nullable_id(&self) -> Option<&str> {
        self.id.as_deref()
    }

/// Get the source field.
    ///
    /// Corresponds to Java: `public FieldType getSourceField()`
    pub fn get_source_field(&self) -> Option<FieldKey> {
        self.source_field
    }

/// Set the source field.
    ///
    /// Corresponds to Java: `public void setSourceField(FieldType sourceField)`
    pub fn set_source_field(&mut self, source_field: Option<FieldKey>) {
        self.source_field = source_field;
    }

/// Get the unique ID of the source object.
    ///
    /// Corresponds to Java: `public Integer getSourceUniqueID()`
    pub fn get_source_unique_id(&self) -> Option<i32> {
        self.source_unique_id
    }

/// Set the unique ID of the source object.
    ///
    /// Corresponds to Java: `public void setSourceUniqueID(Integer sourceUniqueID)`
    pub fn set_source_unique_id(&mut self, source_unique_id: Option<i32>) {
        self.source_unique_id = source_unique_id;
    }

/// Get the target field.
    ///
    /// Corresponds to Java: `public FieldType getTargetField()`
    pub fn get_target_field(&self) -> Option<FieldKey> {
        self.target_field
    }

/// Set the target field.
    ///
    /// Corresponds to Java: `public void setTargetField(FieldType targetField)`
    pub fn set_target_field(&mut self, target_field: Option<FieldKey>) {
        self.target_field = target_field;
    }

/// Get the unique ID of the target object.
    ///
    /// Corresponds to Java: `public Integer getTargetUniqueID()`
    pub fn get_target_unique_id(&self) -> Option<i32> {
        self.target_unique_id
    }

/// Set the unique ID of the target object.
    ///
    /// Corresponds to Java: `public void setTargetUniqueID(Integer targetUniqueID)`
    pub fn set_target_unique_id(&mut self, target_unique_id: Option<i32>) {
        self.target_unique_id = target_unique_id;
    }
}

impl Display for DataLink {
    /// Corresponds to Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[DataLink id={}", self.id.as_deref().unwrap_or("null"))?;

        if let Some(source_field) = self.source_field {
            write!(
                f,
                " sourceField={:?}.{} sourceUniqueID={}",
                source_field.get_field_type_class(),
                source_field.name(),
                format_optional_integer(self.source_unique_id)
            )?;
        }

        if let Some(target_field) = self.target_field {
            write!(
                f,
                " targetField={:?}.{} targetUniqueID={}",
                target_field.get_field_type_class(),
                target_field.name(),
                format_optional_integer(self.target_unique_id)
            )?;
        }

        f.write_str("]")
    }
}

/// Output a nullable integer following Java's `StringBuilder.append(Integer)` semantics (`null` prints as `"null"`).
fn format_optional_integer(value: Option<i32>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_field::TaskField;

    #[test]
    fn new_link_only_carries_id() {
        let link = DataLink::new("1");

        assert_eq!(link.get_id(), "1");
        assert!(link.get_source_field().is_none());
        assert!(link.get_source_unique_id().is_none());
        assert!(link.get_target_field().is_none());
        assert!(link.get_target_unique_id().is_none());
    }

    #[test]
    fn nullable_link_id_distinguishes_null_from_empty() {
        let missing = DataLink::new_nullable(None);
        let empty = DataLink::new("");
        assert_eq!(missing.get_nullable_id(), None);
        assert_eq!(empty.get_nullable_id(), Some(""));
        assert_eq!(missing.to_string(), "[DataLink id=null]");
    }

    #[test]
    fn fields_round_trip() {
        let mut link = DataLink::new("link-1");

        link.set_source_field(Some(FieldKey::Task(TaskField::Name)));
        link.set_source_unique_id(Some(7));
        link.set_target_field(Some(FieldKey::Task(TaskField::Cost)));
        link.set_target_unique_id(Some(9));

        assert_eq!(link.get_source_field(), Some(FieldKey::Task(TaskField::Name)));
        assert_eq!(link.get_source_unique_id(), Some(7));
        assert_eq!(link.get_target_field(), Some(FieldKey::Task(TaskField::Cost)));
        assert_eq!(link.get_target_unique_id(), Some(9));

// Clearing is equally valid (corresponds to Java passing null)
        link.set_source_field(None);
        link.set_source_unique_id(None);
        assert!(link.get_source_field().is_none());
        assert!(link.get_source_unique_id().is_none());
    }

    #[test]
    fn display_matches_java_string_builder_layout() {
        let mut link = DataLink::new("1");
        assert_eq!(link.to_string(), "[DataLink id=1]");

        link.set_source_field(Some(FieldKey::Task(TaskField::Name)));
        link.set_source_unique_id(Some(3));
        link.set_target_field(Some(FieldKey::Task(TaskField::Cost)));
        link.set_target_unique_id(Some(4));

        assert_eq!(
            link.to_string(),
            "[DataLink id=1 sourceField=Task.NAME sourceUniqueID=3 targetField=Task.COST targetUniqueID=4]"
        );
    }

    #[test]
    fn display_uses_null_for_missing_unique_id() {
        let mut link = DataLink::new("2");
        link.set_source_field(Some(FieldKey::Task(TaskField::Name)));

        // Java: StringBuilder.append((Integer) null) → "null"
        assert_eq!(
            link.to_string(),
            "[DataLink id=2 sourceField=Task.NAME sourceUniqueID=null]"
        );
    }
}
