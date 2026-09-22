//! Corresponds to Java: `org/mpxj/mpp/VarMeta.java`
//!
//! Common interface for the VarMeta family (`VarMeta9` / `VarMeta12`): describes the layout of a `Var2Data` variable-length data block
//! — the entry count, the data area size, and the "unique ID → data type → offset" index.
//!
//! # Reproducing the interface
//!
//! Java: `interface VarMeta`, implemented by `abstract class AbstractVarMeta implements VarMeta`,
//! with `VarMeta9` / `VarMeta12` further extending `AbstractVarMeta`. Rust has no inheritance, so it is reproduced as follows:
//!
//! | Java | Rust |
//! |---|---|
//! | `interface VarMeta` | the [`VarMeta`] trait in this file |
//! | `abstract class AbstractVarMeta implements VarMeta` | [`crate::mpp::abstract_var_meta::AbstractVarMeta`] provides `impl VarMeta` |
//! | `VarMeta9 / VarMeta12 extends AbstractVarMeta` | each **composes** an `AbstractVarMeta` and **forwards** all methods to it |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `Integer` parameter (nullable) | `Option<i32>` | follows the "`null` → `Option<T>`" convention; Java's out-of-bounds behavior for `null` (e.g. `TreeMap.get(null)` throwing an NPE) degrades to `None` / `false` on the Rust side |
//! | `Integer[] getUniqueIdentifierArray()` | `Vec<i32>` | element order matches Java's `TreeMap.keySet()` (ascending) |
//! | `Set<Integer> getUniqueIdentifierSet()` / `Set<Integer> getTypes(Integer)` | `BTreeSet<i32>` | Java returns an ascending **live view** of `TreeMap.keySet()`; Rust returns an ascending copy of the set |
//! | `int[] getOffsets()` | `&[i32]` | Java returns the internal array; Rust returns a borrow of the internal slice (avoids copying; the caller cannot modify it) |
//! | `String toString(FieldMap fieldMap)` | `to_string_with_field_map(Option<&dyn VarDataKeyResolver>)` | `FieldMap` is in place ([`crate::mpp::field_map::FieldMap`] and already `impl VarDataKeyResolver`), but the parameter is still taken as the [`VarDataKeyResolver`] that declares only the required capability: Java's `getFieldTypeFromVarDataKey` returns the `FieldType` interface, whereas this crate's `FieldType` trait has an `Option<Self>` return value and is not object safe, so it cannot be returned as a trait object; `null` → `None` |
//!
//! Skipped Java methods: none.
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this interface does not use these mechanisms.

use std::collections::BTreeSet;

/// Query interface for "variable-length data key → field type name" (Rust specific).
///
/// Corresponds to Java: `org.mpxj.mpp.FieldMap#getFieldTypeFromVarDataKey(Integer key)`
///
/// Java's `AbstractVarMeta#toString(FieldMap)` uses it only to translate VarData keys into field names (debug output).
/// On the Rust side [`crate::mpp::field_map::FieldMap`] is in place and already `impl VarDataKeyResolver`; this trait
/// still declares only the minimal capability required by that method (so `AbstractVarMeta` does not depend on the concrete type of `FieldMap`):
///
/// - Java returns `FieldType` (the `org.mpxj.FieldType` interface), but Rust's `FieldType` trait
/// has an `Option<Self>` return value and is not object safe, so it cannot be returned as a trait object; therefore it returns the **field display name** instead;
/// - returning `None` represents Java's `null` (the key is not a VarData field).
pub trait VarDataKeyResolver {
/// Translate a variable-length data key into a field display name.
    ///
    /// Corresponds to Java: `FieldType getFieldTypeFromVarDataKey(Integer key)`
    fn get_field_type_from_var_data_key(&self, key: i32) -> Option<String>;
}

/// Metadata for a variable-length data block.
///
/// Corresponds to Java: `interface VarMeta`
pub trait VarMeta {
/// Get the number of entries in the `Var2Data` block.
    ///
    /// Corresponds to Java: `int getItemCount()`
    fn get_item_count(&self) -> i32;

/// Get the data area size of the `Var2Data` block.
    ///
    /// Corresponds to Java: `int getDataSize()`
    fn get_data_size(&self) -> i32;

/// Get all unique IDs that have data (ascending).
    ///
    /// Corresponds to Java: `Integer[] getUniqueIdentifierArray()`
    fn get_unique_identifier_array(&self) -> Vec<i32>;

/// Get the set of all unique IDs that have data (ascending).
    ///
    /// Corresponds to Java: `Set<Integer> getUniqueIdentifierSet()`
    fn get_unique_identifier_set(&self) -> BTreeSet<i32>;

/// Get the offset of the entry in the `Var2Data` block by "unique ID + data type".
    ///
    /// Corresponds to Java: `Integer getOffset(Integer id, Integer type)`
    ///
/// Returns `None` when `id` / `type` is `None` (Java's `null`) or not present in the index.
    fn get_offset(&self, id: Option<i32>, type_: Option<i32>) -> Option<i32>;

/// Get the sorted offset array.
    ///
    /// Corresponds to Java: `int[] getOffsets()`
    fn get_offsets(&self) -> &[i32];

/// Get all data types defined under the specified unique ID (ascending).
    ///
    /// Corresponds to Java: `Set<Integer> getTypes(Integer id)`
    ///
/// Returns an empty set when the ID does not exist (matching Java).
    fn get_types(&self, id: Option<i32>) -> BTreeSet<i32>;

/// Determine whether the given unique ID exists in the index.
    ///
    /// Corresponds to Java: `boolean containsKey(Integer key)`
    ///
/// Java throws an NPE for `null` (`TreeMap.containsKey(null)`); the Rust side returns `false`.
    fn contains_key(&self, key: Option<i32>) -> bool;

/// Export the contents of this block as debug text.
    ///
    /// Corresponds to Java: `String toString(FieldMap fieldMap)`
    ///
/// When `field_map` is `None` (Java's `null`), only the numeric type identifiers are output.
    fn to_string_with_field_map(&self, field_map: Option<&dyn VarDataKeyResolver>) -> String;
}
