//! Corresponds to Java: `org/mpxj/mpp/FilterReader14.java`
//!
//! The MPP14 filter definition reader: it only declares the `Var2Data` data type key (6) used by this version and the criteria reader
//! ([`FilterCriteriaReader14`]); all other logic is inherited from [`FilterReader`].
//!
//! # Restoring inheritance (composition + forwarding)
//!
//! Java's `final class FilterReader14 extends FilterReader`. The Rust side, like
//! [`FilterReader9`](crate::mpp::filter_reader9::FilterReader9), takes the same approach: implement
//! the [`FilterReader`] trait and restore Java's `private final CriteriaReader m_criteraReader`
//! as a concrete field held by value plus `&dyn CriteriaReader` forwarding.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `private static final Integer FILTER_DATA = Integer.valueOf(6);` | [`FilterReader14::FILTER_DATA`] (`i32`) | Same value as version 12 (likewise in Java); Java's `Integer` is always non-null |
//! | `getVarDataType()` returns `Integer` | `get_var_data_type()` returns `i32` | same as above |
//!
//! Skipped Java members: none.
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use crate::mpp::criteria_reader::CriteriaReader;
use crate::mpp::filter_criteria_reader14::FilterCriteriaReader14;
use crate::mpp::filter_reader::FilterReader;

/// The MPP14 filter definition reader.
///
/// Corresponds to Java: `final class FilterReader14 extends FilterReader`
pub struct FilterReader14 {
/// The criteria reader.
    ///
    /// Corresponds to Java field: `private final CriteriaReader m_criteraReader = new FilterCriteriaReader14();`
    criteria_reader: FilterCriteriaReader14,
}

impl FilterReader14 {
/// The `Var2Data` data type key used by this version's files to read filter criteria data.
    ///
    /// Corresponds to Java: `private static final Integer FILTER_DATA = Integer.valueOf(6);`
    pub const FILTER_DATA: i32 = 6;

/// Constructor.
    ///
/// Corresponds to Java: implicit default constructor (the Java side does not declare a constructor explicitly).
    pub fn new() -> Self {
        Self {
            criteria_reader: FilterCriteriaReader14::new(),
        }
    }
}

impl Default for FilterReader14 {
/// Corresponds to Java: implicit default constructor, equivalent to [`FilterReader14::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl FilterReader for FilterReader14 {
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
    use crate::mpp::filter_reader14::FilterReader14;

/// The data type key and criteria reader match Java.
    #[test]
    fn constants_match_java() {
        let reader = FilterReader14::new();
        assert_eq!(FilterReader14::FILTER_DATA, 6);
        assert_eq!(reader.get_var_data_type(), 6);

// Version 14 block length is 36
        let criteria_reader = reader.get_criteria_reader();
        assert_eq!(criteria_reader.get_criteria_block_size(), 36);
        assert_eq!(criteria_reader.get_value_offset(), 14);
    }

/// The default constructor is equivalent to `new`.
    #[test]
    fn default_matches_new() {
        assert_eq!(
            FilterReader14::default().get_var_data_type(),
            FilterReader14::new().get_var_data_type()
        );
    }
}
