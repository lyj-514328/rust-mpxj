//! Corresponds to Java: `org/mpxj/RateSource.java`

/// Source of the cost rates used by a resource assignment.
///
/// Corresponds to Java: `public enum RateSource`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RateSource {
    /// Corresponds to Java: `RESOURCE`
    Resource,
    /// Corresponds to Java: `ROLE`
    Role,
    /// Corresponds to Java: `OVERRIDE`
    Override,
}
