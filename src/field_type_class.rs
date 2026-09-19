//! 对应 Java: `org/mpxj/FieldTypeClass.java`

/// 表示某个 `FieldType` 实例可以归属的实体类型。
///
/// 对应 Java: `public enum FieldTypeClass`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldTypeClass {
    Task,
    Resource,
    Assignment,
    Constraint,
    Project,
    Unknown,
}
