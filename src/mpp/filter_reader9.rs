//! Corresponds to Java: `org/mpxj/mpp/FilterReader9.java`
//!
//! Filter definition reader for MPP9: declares only the `Var2Data` data type key (1) used by this version and the criteria reader
//! ([`FilterCriteriaReader9`]); all other logic is inherited from [`FilterReader`].
//!
//! # How the inheritance is restored (composition + forwarding)
//!
//! Java's `final class FilterReader9 extends FilterReader`. Rust has no inheritance, so:
//!
//! - This class **implements the [`FilterReader`] trait**: Java's 2 `@Override protected` methods correspond one by one to
//! the trait's required methods, and the public method `process` inherited from Java is provided by the trait's default method;
//! - Java's `private final CriteriaReader m_criteraReader = new FilterCriteriaReader9();`
//! is a **concrete field held by value**, `criteria_reader`, on the Rust side, forwarded by
//! [`FilterReader9::get_criteria_reader`] as `&dyn CriteriaReader` (equivalent to Java's
//! upcast).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `private static final Integer FILTER_DATA = Integer.valueOf(1);` | [`FilterReader9::FILTER_DATA`] (`i32`) | The constant name follows Java; Java's `Integer` is always non-null |
//! | `getVarDataType()` returns `Integer` | `get_var_data_type()` returns `i32` | Same as above |
//!
//! Skipped Java members: none.
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use crate::mpp::criteria_reader::CriteriaReader;
use crate::mpp::filter_criteria_reader9::FilterCriteriaReader9;
use crate::mpp::filter_reader::FilterReader;

/// MPP9 filter definition reader.
///
/// Corresponds to Java: `final class FilterReader9 extends FilterReader`
pub struct FilterReader9 {
/// Criteria reader.
    ///
    /// Corresponds to Java field: `private final CriteriaReader m_criteraReader = new FilterCriteriaReader9();`
/// (the field name keeps the typo `m_criteraReader` from the Java source)
    criteria_reader: FilterCriteriaReader9,
}

impl FilterReader9 {
/// The `Var2Data` data type key used by this version's files to read filter criteria data.
    ///
    /// Corresponds to Java: `private static final Integer FILTER_DATA = Integer.valueOf(1);`
    pub const FILTER_DATA: i32 = 1;

/// Constructor.
    ///
/// Corresponds to Java: implicit default constructor (Java does not declare a constructor explicitly).
    pub fn new() -> Self {
        Self {
            criteria_reader: FilterCriteriaReader9::new(),
        }
    }
}

impl Default for FilterReader9 {
/// Corresponds to Java: implicit default constructor, equivalent to [`FilterReader9::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl FilterReader for FilterReader9 {
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
    use crate::mpp::filter_reader9::FilterReader9;

/// The data type key and criteria reader are consistent with Java.
    #[test]
    fn constants_match_java() {
        let reader = FilterReader9::new();
        assert_eq!(FilterReader9::FILTER_DATA, 1);
        assert_eq!(reader.get_var_data_type(), 1);

// The criteria reader uses the version 9 layout: block length 80, start offset 20
        let criteria_reader = reader.get_criteria_reader();
        assert_eq!(criteria_reader.get_criteria_block_size(), 80);
        assert_eq!(criteria_reader.get_criteria_start_offset(), 20);
    }

/// The default constructor is equivalent to `new`.
    #[test]
    fn default_matches_new() {
        assert_eq!(
            FilterReader9::default().get_var_data_type(),
            FilterReader9::new().get_var_data_type()
        );
    }
}
