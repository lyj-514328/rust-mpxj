//! 对应 Java: `org/mpxj/DataLink.java`
//!
//! 表示两个字段之间的链接，可以位于同一个项目内，也可以跨项目。通常一条记录
//! 同时给出源与目标，有时也会拆成两条记录（源在一条、目标在另一条）。此外，
//! 当一条链接被移除后，可能留下「只有源、没有目标」的记录。
//!
//! # Java → Rust 方法对照
//!
//! | Java | Rust |
//! |---|---|
//! | `public DataLink(String id)` | [`DataLink::new`] |
//! | `public String getID()` | [`DataLink::get_id`]（返回 `&str`，Java 返回 `String`） |
//! | `public FieldType getSourceField()` | [`DataLink::get_source_field`]（返回 `Option<FieldKey>`，见下） |
//! | `public void setSourceField(FieldType)` | [`DataLink::set_source_field`] |
//! | `public Integer getSourceUniqueID()` | [`DataLink::get_source_unique_id`] |
//! | `public void setSourceUniqueID(Integer)` | [`DataLink::set_source_unique_id`] |
//! | `public FieldType getTargetField()` | [`DataLink::get_target_field`] |
//! | `public void setTargetField(FieldType)` | [`DataLink::set_target_field`] |
//! | `public Integer getTargetUniqueID()` | [`DataLink::get_target_unique_id`] |
//! | `public void setTargetUniqueID(Integer)` | [`DataLink::set_target_unique_id`] |
//! | `@Override public String toString()` | `impl Display for DataLink` |
//!
//! # 与 Java 的差异
//!
//! - **`FieldType` → [`FieldKey`]**：Java 的 `getSourceField()` / `getTargetField()`
//!   返回 `FieldType` 接口；Rust 侧按本 crate 的约定改用其运行时擦除形式
//!   [`FieldKey`]（见 `crate::field_key`），并用 `FieldType::get_field_type_class` /
//!   `FieldType::name` 复现 Java `toString()` 中 `FieldTypeClass.name()` 的输出。
//! - **可空字段 → `Option<T>`**：Java 中除 `m_id` 外均可为 `null`（`Integer` → `Option<i32>`，
//!   `FieldType` → `Option<FieldKey>`）。
//! - **未派生 `PartialEq` / `Eq` / `Hash`**：Java 的 `DataLink` 未覆写
//!   `equals` / `hashCode`（沿用对象同一性），因此 Rust 侧不提供结构相等，
//!   容器中的「按引用相等移除」由 [`crate::data_link_container::DataLinkContainer`]
//!   用指针同一性表达。
//! - **`toString()` → [`Display`]**：输出顺序与 Java 的 `StringBuilder` 拼接一致；
//!   Java 对可空整数字段打印 `"null"`，Rust 侧用 [`format_optional_integer`] 复现。
//!
//! # 跳过的 Java 机制
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::fmt::{self, Display, Formatter};

use crate::field_key::FieldKey;
use crate::field_type::FieldType;

/// 两个字段之间的链接。
///
/// 对应 Java: `public final class DataLink`
#[derive(Debug, Clone)]
pub struct DataLink {
    /// 链接标识。
    ///
    /// 对应 Java: `private final String m_id;`
    id: String,

    /// 源字段。
    ///
    /// 对应 Java: `private FieldType m_sourceField;`
    source_field: Option<FieldKey>,

    /// 源对象的 unique ID。
    ///
    /// 对应 Java: `private Integer m_sourceUniqueID;`
    source_unique_id: Option<i32>,

    /// 目标字段。
    ///
    /// 对应 Java: `private FieldType m_targetField;`
    target_field: Option<FieldKey>,

    /// 目标对象的 unique ID。
    ///
    /// 对应 Java: `private Integer m_targetUniqueID;`
    target_unique_id: Option<i32>,
}

impl DataLink {
    /// 构造方法。
    ///
    /// 对应 Java: `public DataLink(String id)`
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            source_field: None,
            source_unique_id: None,
            target_field: None,
            target_unique_id: None,
        }
    }

    /// 取链接标识。
    ///
    /// 对应 Java: `public String getID()`
    pub fn get_id(&self) -> &str {
        &self.id
    }

    /// 取源字段。
    ///
    /// 对应 Java: `public FieldType getSourceField()`
    pub fn get_source_field(&self) -> Option<FieldKey> {
        self.source_field
    }

    /// 设置源字段。
    ///
    /// 对应 Java: `public void setSourceField(FieldType sourceField)`
    pub fn set_source_field(&mut self, source_field: Option<FieldKey>) {
        self.source_field = source_field;
    }

    /// 取源对象的 unique ID。
    ///
    /// 对应 Java: `public Integer getSourceUniqueID()`
    pub fn get_source_unique_id(&self) -> Option<i32> {
        self.source_unique_id
    }

    /// 设置源对象的 unique ID。
    ///
    /// 对应 Java: `public void setSourceUniqueID(Integer sourceUniqueID)`
    pub fn set_source_unique_id(&mut self, source_unique_id: Option<i32>) {
        self.source_unique_id = source_unique_id;
    }

    /// 取目标字段。
    ///
    /// 对应 Java: `public FieldType getTargetField()`
    pub fn get_target_field(&self) -> Option<FieldKey> {
        self.target_field
    }

    /// 设置目标字段。
    ///
    /// 对应 Java: `public void setTargetField(FieldType targetField)`
    pub fn set_target_field(&mut self, target_field: Option<FieldKey>) {
        self.target_field = target_field;
    }

    /// 取目标对象的 unique ID。
    ///
    /// 对应 Java: `public Integer getTargetUniqueID()`
    pub fn get_target_unique_id(&self) -> Option<i32> {
        self.target_unique_id
    }

    /// 设置目标对象的 unique ID。
    ///
    /// 对应 Java: `public void setTargetUniqueID(Integer targetUniqueID)`
    pub fn set_target_unique_id(&mut self, target_unique_id: Option<i32>) {
        self.target_unique_id = target_unique_id;
    }
}

impl Display for DataLink {
    /// 对应 Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[DataLink id={}", self.id)?;

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

/// 按 Java 的 `StringBuilder.append(Integer)` 语义输出可空整数（`null` 打印为 `"null"`）。
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

        // 清除同样有效（对应 Java 传入 null）
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
