//! Corresponds to Java: `org/mpxj/mpp/FilterReader12.java`
//!
//! The MPP12 filter definition reader: it only declares the `Var2Data` data type key (6) used by this version and the criteria reader
//! ([`FilterCriteriaReader12`]); all other logic is inherited from [`FilterReader`].
//!
//! # Restoring inheritance (composition + forwarding)
//!
//! Java's `final class FilterReader12 extends FilterReader`. The Rust side, like
//! [`FilterReader9`](crate::mpp::filter_reader9::FilterReader9), takes the same approach: implement
//! the [`FilterReader`] trait and restore Java's `private final CriteriaReader m_criteraReader`
//! as a concrete field held by value plus `&dyn CriteriaReader` forwarding.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `private static final Integer FILTER_DATA = Integer.valueOf(6);` | [`FilterReader12::FILTER_DATA`] (`i32`) | Java's `Integer` is always non-null |
//! | `getVarDataType()` returns `Integer` | `get_var_data_type()` returns `i32` | same as above |
//!
//! Skipped Java members: none.
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use crate::mpp::criteria_reader::CriteriaReader;
use crate::mpp::filter_criteria_reader12::FilterCriteriaReader12;
use crate::mpp::filter_reader::FilterReader;

/// The MPP12 filter definition reader.
///
/// Corresponds to Java: `final class FilterReader12 extends FilterReader`
pub struct FilterReader12 {
/// The criteria reader.
    ///
    /// Corresponds to Java field: `private final CriteriaReader m_criteraReader = new FilterCriteriaReader12();`
    criteria_reader: FilterCriteriaReader12,
}

impl FilterReader12 {
/// The `Var2Data` data type key used by this version's files to read filter criteria data.
    ///
    /// Corresponds to Java: `private static final Integer FILTER_DATA = Integer.valueOf(6);`
    pub const FILTER_DATA: i32 = 6;

/// Constructor.
    ///
/// Corresponds to Java: implicit default constructor (the Java side does not declare a constructor explicitly).
    pub fn new() -> Self {
        Self {
            criteria_reader: FilterCriteriaReader12::new(),
        }
    }
}

impl Default for FilterReader12 {
/// Corresponds to Java: implicit default constructor, equivalent to [`FilterReader12::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl FilterReader for FilterReader12 {
    /// Corresponds to Java: `@Override protected Integer getVarDataType()`
    fn get_var_data_type(&self) -> i32 {
        Self::FILTER_DATA
    }

    /// Corresponds to Java: `@Override protected CriteriaReader getCriteriaReader()`
    fn get_criteria_reader(&self) -> &dyn CriteriaReader {
        &self.criteria_reader
    }
}

#[cfg(test)]
mod tests {
    use crate::mpp::filter_reader::FilterReader;
    use crate::mpp::filter_reader12::FilterReader12;

/// The data type key and criteria reader match Java.
    #[test]
    fn constants_match_java() {
        let reader = FilterReader12::new();
        assert_eq!(FilterReader12::FILTER_DATA, 6);
        assert_eq!(reader.get_var_data_type(), 6);

        let criteria_reader = reader.get_criteria_reader();
        assert_eq!(criteria_reader.get_criteria_block_size(), 80);
        assert_eq!(criteria_reader.get_criteria_start_offset(), 20);
    }

/// The default constructor is equivalent to `new`.
    #[test]
    fn default_matches_new() {
        assert_eq!(
            FilterReader12::default().get_var_data_type(),
            FilterReader12::new().get_var_data_type()
        );
    }
}
