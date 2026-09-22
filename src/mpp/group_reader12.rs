//! Corresponds to Java: `org/mpxj/mpp/GroupReader12.java`
//!
//! Group definition reader for MPP12: it only declares the `Var2Data` data type key used by this version (6), all other logic
//! is inherited from [`GroupReader`].
//!
//! # Reproducing inheritance (trait implementation)
//!
//! Java's `final class GroupReader12 extends GroupReader`. The Rust side uses the same approach as
//! [`GroupReader9`](crate::mpp::group_reader9::GroupReader9): implementing
//! the [`GroupReader`] trait; the public method `process` inherited from Java is provided by the trait's default method.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `private static final Integer GROUP_DATA = Integer.valueOf(6);` | [`GroupReader12::GROUP_DATA`] (`i32`) | The constant name follows Java; Java's `Integer` is always non-null |
//! | `getVarDataType()` returns `Integer` | `get_var_data_type()` returns `i32` | Same as above |
//!
//! Skipped Java members: none.
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use crate::mpp::group_reader::GroupReader;

/// MPP12 group definition reader.
///
/// Corresponds to Java: `final class GroupReader12 extends GroupReader`
pub struct GroupReader12;

impl GroupReader12 {
/// The `Var2Data` data type key used by files of this version to read group data.
    ///
    /// Corresponds to Java: `private static final Integer GROUP_DATA = Integer.valueOf(6);`
    pub const GROUP_DATA: i32 = 6;

/// Constructor.
    ///
/// Corresponds to Java: implicit default constructor (Java does not declare a constructor explicitly).
    pub fn new() -> Self {
        Self
    }
}

impl Default for GroupReader12 {
/// Corresponds to Java: implicit default constructor, equivalent to [`GroupReader12::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl GroupReader for GroupReader12 {
    /// Corresponds to Java: `@Override protected Integer getVarDataType()`
    fn get_var_data_type(&self) -> i32 {
        Self::GROUP_DATA
    }
}

#[cfg(test)]
mod tests {
    use crate::mpp::group_reader::GroupReader;
    use crate::mpp::group_reader12::GroupReader12;

/// The data type key matches Java.
    #[test]
    fn constants_match_java() {
        assert_eq!(GroupReader12::GROUP_DATA, 6);
        assert_eq!(GroupReader12::new().get_var_data_type(), 6);
    }

/// The default constructor is equivalent to `new`.
    #[test]
    fn default_matches_new() {
        assert_eq!(
            GroupReader12::default().get_var_data_type(),
            GroupReader12::new().get_var_data_type()
        );
    }
}
