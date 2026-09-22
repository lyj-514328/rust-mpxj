//! Corresponds to Java: `org/mpxj/FieldTypeClass.java`

/// Represents the entity type to which a `FieldType` instance can belong.
///
/// Corresponds to Java: `public enum FieldTypeClass`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldTypeClass {
    Task,
    Resource,
    Assignment,
    Constraint,
    Project,
    Unknown,
}
