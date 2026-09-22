//! Corresponds to Java: `org/mpxj/mpp/GroupReader9.java`
//!
//! Group definition reader for MPP9: it only declares the `Var2Data` data type key used by this version (1), all other logic
//! is inherited from [`GroupReader`].
//!
//! # Reproducing inheritance (trait implementation)
//!
//! Java's `final class GroupReader9 extends GroupReader`. Rust has no inheritance, so this class is
//! **a zero-sized struct implementing the [`GroupReader`] trait**: Java's `@Override protected` methods
//! correspond to the trait's required methods, and the public method `process` inherited from Java is provided by the trait's default method.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `private static final Integer GROUP_DATA = Integer.valueOf(1);` | [`GroupReader9::GROUP_DATA`] (`i32`) | The constant name follows Java; Java's `Integer` is always non-null |
//! | `getVarDataType()` returns `Integer` | `get_var_data_type()` returns `i32` | Same as above |
//!
//! Skipped Java members: none.
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use crate::mpp::group_reader::GroupReader;

/// MPP9 group definition reader.
///
/// Corresponds to Java: `final class GroupReader9 extends GroupReader`
pub struct GroupReader9;

impl GroupReader9 {
/// The `Var2Data` data type key used by files of this version to read group data.
    ///
    /// Corresponds to Java: `private static final Integer GROUP_DATA = Integer.valueOf(1);`
    pub const GROUP_DATA: i32 = 1;

/// Constructor.
    ///
/// Corresponds to Java: implicit default constructor (Java does not declare a constructor explicitly).
    pub fn new() -> Self {
        Self
    }
}

impl Default for GroupReader9 {
/// Corresponds to Java: implicit default constructor, equivalent to [`GroupReader9::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl GroupReader for GroupReader9 {
    /// Corresponds to Java: `@Override protected Integer getVarDataType()`
    fn get_var_data_type(&self) -> i32 {
        Self::GROUP_DATA
    }
}

#[cfg(test)]
mod tests {
    use crate::mpp::group_reader::GroupReader;
    use crate::mpp::group_reader9::GroupReader9;

/// The data type key matches Java.
    #[test]
    fn constants_match_java() {
        assert_eq!(GroupReader9::GROUP_DATA, 1);
        assert_eq!(GroupReader9::new().get_var_data_type(), 1);
    }

/// The default constructor is equivalent to `new`.
    #[test]
    fn default_matches_new() {
        assert_eq!(
            GroupReader9::default().get_var_data_type(),
            GroupReader9::new().get_var_data_type()
        );
    }
}
