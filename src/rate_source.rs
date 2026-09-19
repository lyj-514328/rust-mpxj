//! 对应 Java: `org/mpxj/RateSource.java`

/// 资源分配所用成本费率的来源。
///
/// 对应 Java: `public enum RateSource`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RateSource {
    /// 对应 Java: `RESOURCE`
    Resource,
    /// 对应 Java: `ROLE`
    Role,
    /// 对应 Java: `OVERRIDE`
    Override,
}
