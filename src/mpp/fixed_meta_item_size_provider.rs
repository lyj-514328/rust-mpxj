//! Corresponds to Java: `org/mpxj/mpp/FixedMetaItemSizeProvider.java`
//!
//! This strategy interface determines the record size to use for each record in a `FixedMeta` structure.
//!
//! Porting trade-offs:
//! - Java's `interface` corresponds to Rust's `pub trait` (it is package-visible in Java and public in Rust,
//! making it easier for other modules to provide custom implementations).
//! - All Java implementations are lambdas (inline implementations in the two constructors of `FixedMeta`); the Rust side provides
//! a blanket impl for `Fn(i32, i32) -> i32`, so closures can be passed directly as a provider,
//! preserving the `FixedMeta::new(data, &|file_size, item_count| ...)` form.
//! - Java's `int` parameters and return values are all kept as `i32`.
//! - Does not involve reflection, JDBC, XML/JAXB, GUI, `javax.*`, or `Serializable`.

/// Provide the record size used in a `FixedMeta` structure.
///
/// Corresponds to Java: `interface FixedMetaItemSizeProvider`
pub trait FixedMetaItemSizeProvider {
/// Obtain the record size to use for a `FixedMeta` structure.
    ///
    /// Corresponds to Java: `int getItemSize(int fileSize, int itemCount)`
    ///
/// - `file_size`: the size of the whole block containing the `FixedMeta` data (in Java this is `is.available()`)
/// - `item_count`: the number of records reported by the block header
    fn get_item_size(&self, file_size: i32, item_count: i32) -> i32;
}

/// Allow closures to be used as an item size provider (corresponding to Java's inline lambda implementations).
impl<F> FixedMetaItemSizeProvider for F
where
    F: Fn(i32, i32) -> i32,
{
/// Corresponds to Java: the lambda form of `FixedMetaItemSizeProvider#getItemSize`
    fn get_item_size(&self, file_size: i32, item_count: i32) -> i32 {
        self(file_size, item_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// A closure can be used as a provider, and the arguments are passed through unchanged.
    #[test]
    fn closure_implements_provider() {
        let provider = |file_size: i32, item_count: i32| file_size + item_count;
        assert_eq!(provider.get_item_size(16, 2), 18);
    }

/// It can also be passed as `&dyn`.
    #[test]
    fn provider_as_dyn() {
        let provider = |_file_size: i32, _item_count: i32| 8;
        let dynamic: &dyn FixedMetaItemSizeProvider = &provider;
        assert_eq!(dynamic.get_item_size(1024, 32), 8);
    }
}
