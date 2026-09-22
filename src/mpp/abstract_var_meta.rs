//! Corresponds to Java: `org/mpxj/mpp/AbstractVarMeta.java`
//!
//! Base class of the variable-length block system: after parsing a VarMeta stream, it holds the "item count, data area size, sorted offset array" as well as
//! the core index `unique ID → data type → offset`, used by [`crate::mpp::var2_data::Var2Data`] to locate variable-length fields.
//!
//! # Inheritance and interface emulation (composition + forwarding)
//!
//! Java: `abstract class AbstractVarMeta extends MPPComponent implements VarMeta`.
//!
//! - `implements VarMeta` → `impl VarMeta for AbstractVarMeta` (this file), while also keeping inherent methods of the same name,
//! so that an `AbstractVarMeta` value itself and a `dyn VarMeta` can both be called directly;
//! - `extends MPPComponent` → Java's `MPPComponent` provides only 4 read helpers
//! (`readByte` / `readShort` / `readInt` / `readByteArray`, all taking an `InputStream`),
//! which the Rust side has realized as [`crate::mpp::mpp_component::MPPComponent`] (the input source is uniformly
//! `&[u8]`); the three crate-visible free functions [`read_byte`] / [`read_short`] / [`read_int`] in this file
//! **forward directly** to it, so that `VarMeta9` / `VarMeta12` / `Var2Data` can keep their existing reference paths;
//! - `VarMeta9 / VarMeta12 extends AbstractVarMeta` → composition + forwarding (see their respective files).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `protected int m_itemCount` / `protected int m_dataSize` / `protected Map<...> m_table` | Fields of the same name with `pub(crate)` | Java's subclasses read and write these protected fields directly; the Rust side emulates them equivalently with same-crate-visible fields |
//! | `TreeMap<Integer, Map<Integer, Integer>>` | `BTreeMap<i32, BTreeMap<i32, i32>>` | `getUniqueIdentifierArray` / `getUniqueIdentifierSet` / `toString` all rely on `TreeMap`'s ascending traversal order, so the ordered `BTreeMap` is used (rather than the conventional `HashMap`) |
//! | `Integer` parameter (nullable) | `Option<i32>` | Follows the "`null` → `Option<T>`" convention |
//! | `Set<Integer>` | `BTreeSet<i32>` | The Rust side returns an ascending-order copy of the set rather than a live view of `TreeMap.keySet()` |
//! | `FieldMap` parameter | `Option<&dyn VarDataKeyResolver>` | `FieldMap` has been realized ([`crate::mpp::field_map::FieldMap`] and already `impl VarDataKeyResolver`), but this file still takes its parameter as [`VarDataKeyResolver`]: Java's `getFieldTypeFromVarDataKey` returns the `FieldType` interface, whereas this crate's `FieldType` trait has an `Option<Self>` return value and is not object-safe, so it cannot be returned as a trait object; therefore only the minimal capability needed by `toString` is declared (returning the field display name) |
//! | `toString()` | `impl Display` | Java's `toString()` internally calls `toString(null)` |
//! | Out-of-bounds read throws `EOFException` / `IndexOutOfBoundsException` | Return `None` and end parsing early | Follows the "library code must not panic" convention; see [`read_int`] |
//!
//! Skipped Java methods: none (`MPPComponent.readByteArray` is implemented by the caller slicing directly,
//! because the Rust-side `&[u8]` is itself an array).
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use crate::mpp::mpp_component::MPPComponent;
use crate::mpp::var_meta::{VarDataKeyResolver, VarMeta};

/// Read a 4-byte integer from a byte slice in little-endian order and advance the cursor.
///
/// Corresponds to Java: `MPPComponent.readInt(InputStream)` (Rust-specific: the input source is changed from `InputStream` to `&[u8]`)
///
/// The implementation forwards to the already-realized [`MPPComponent::read_int`]; this file no longer duplicates the byte-parsing logic;
/// This function is kept so that `VarMeta9` / `VarMeta12` / `Var2Data` can keep using the "get the read helper from the module"
/// existing reference path (`crate::mpp::abstract_var_meta::read_int`).
///
/// When fewer than 4 bytes remain Java throws `EOFException`; the Rust side returns `None` (the cursor does not move), leaving it to the caller to decide
/// how to end parsing early -- never panics.
pub(crate) fn read_int(data: &[u8], position: &mut usize) -> Option<i32> {
    MPPComponent::read_int(data, position)
}

/// Read a 2-byte integer from a byte slice in little-endian order and advance the cursor.
///
/// Corresponds to Java: `MPPComponent.readShort(InputStream)` (Rust-specific: the input source is changed from `InputStream` to `&[u8]`)
///
/// The implementation forwards to [`MPPComponent::read_short`]; the return value is `0..=65535`,
/// and it returns `None` when fewer than 2 bytes remain (Java throws `EOFException`).
pub(crate) fn read_short(data: &[u8], position: &mut usize) -> Option<i32> {
    MPPComponent::read_short(data, position)
}

/// Read an unsigned byte (`0..=255`) from a byte slice and advance the cursor.
///
/// Corresponds to Java: `MPPComponent.readByte(InputStream)` (Rust-specific: the input source is changed from `InputStream` to `&[u8]`)
///
/// The implementation forwards to [`MPPComponent::read_byte`]; it returns `None` at the end (Java throws `EOFException`).
pub(crate) fn read_byte(data: &[u8], position: &mut usize) -> Option<i32> {
    MPPComponent::read_byte(data, position)
}

/// Metadata of a variable-length data block (the base class implementation of `VarMeta`).
///
/// Corresponds to Java: `abstract class AbstractVarMeta extends MPPComponent implements VarMeta`
pub struct AbstractVarMeta {
/// Item count.
    ///
    /// Corresponds to Java field: `protected int m_itemCount;`
    ///
/// `pub(crate)`: Java's subclasses write this protected field directly (see the file header).
    pub(crate) item_count: i32,

/// Data area size of the `Var2Data` block.
    ///
    /// Corresponds to Java field: `protected int m_dataSize;`
    pub(crate) data_size: i32,

/// Sorted array of item offsets.
    ///
    /// Corresponds to Java field: `private int[] m_offsets;`
    offsets: Vec<i32>,

/// The `unique ID → data type → offset` index.
    ///
    /// Corresponds to Java field: `protected final Map<Integer, Map<Integer, Integer>> m_table = new TreeMap<>();`
    pub(crate) table: BTreeMap<i32, BTreeMap<i32, i32>>,
}

impl AbstractVarMeta {
/// Construct empty metadata (Rust-specific: corresponds to Java's implicit `protected AbstractVarMeta()`,
/// after which the subclass writes the protected fields directly; see the file header).
    pub fn new() -> Self {
        Self {
            item_count: 0,
            data_size: 0,
            offsets: Vec::new(),
            table: BTreeMap::new(),
        }
    }

/// Get the item count.
    ///
    /// Corresponds to Java: `@Override public int getItemCount()`
    pub fn get_item_count(&self) -> i32 {
        self.item_count
    }

/// Get the data area size.
    ///
    /// Corresponds to Java: `@Override public int getDataSize()`
    pub fn get_data_size(&self) -> i32 {
        self.data_size
    }

/// Get all unique IDs for which data occurred (ascending, consistent with the order of Java's `TreeMap.keySet()`).
    ///
    /// Corresponds to Java: `@Override public Integer[] getUniqueIdentifierArray()`
    pub fn get_unique_identifier_array(&self) -> Vec<i32> {
        self.table.keys().copied().collect()
    }

/// Get the set of all unique IDs for which data occurred (ascending).
    ///
    /// Corresponds to Java: `@Override public Set<Integer> getUniqueIdentifierSet()`
    pub fn get_unique_identifier_set(&self) -> BTreeSet<i32> {
        self.table.keys().copied().collect()
    }

/// Get the offset by `unique ID + data type`.
    ///
    /// Corresponds to Java: `@Override public Integer getOffset(Integer id, Integer type)`
    ///
/// Java returns `null` for `type == null`; for `id == null` it throws an NPE because of `TreeMap.get(null)`,
/// while the Rust side returns `None` in both cases.
    pub fn get_offset(&self, id: Option<i32>, type_: Option<i32>) -> Option<i32> {
        let id = id?;
        let type_ = type_?;

        // Java: Map<Integer, Integer> map = m_table.get(id);
        //       if (map != null && type != null) { result = map.get(type); }
        self.table.get(&id).and_then(|map| map.get(&type_).copied())
    }

/// Set the sorted offset array (called by the subclass after parsing completes).
    ///
    /// Corresponds to Java: `protected void setOffsets(int[] offsets)`
    pub fn set_offsets(&mut self, offsets: Vec<i32>) {
        self.offsets = offsets;
    }

/// Get the sorted offset array.
    ///
    /// Corresponds to Java: `@Override public int[] getOffsets()`
    pub fn get_offsets(&self) -> &[i32] {
        &self.offsets
    }

/// Get all data types defined under the given unique ID (ascending).
    ///
    /// Corresponds to Java: `@Override public Set<Integer> getTypes(Integer id)`
    ///
/// Return an empty set when the ID does not exist (or is `null`).
    pub fn get_types(&self, id: Option<i32>) -> BTreeSet<i32> {
        // Java: Map<Integer, Integer> map = m_table.get(id);
        //       result = map != null ? map.keySet() : new HashSet<>();
        id.and_then(|id| self.table.get(&id))
            .map(|map| map.keys().copied().collect())
            .unwrap_or_default()
    }

/// Determine whether the given unique ID exists in the index.
    ///
    /// Corresponds to Java: `@Override public boolean containsKey(Integer key)`
    ///
/// Java throws an NPE for `null` because of `TreeMap.containsKey(null)`; the Rust side returns `false`.
    pub fn contains_key(&self, key: Option<i32>) -> bool {
        key.is_some_and(|key| self.table.contains_key(&key))
    }

/// Export this block's contents as debug text (field names are not translated when `field_map` is `None`).
    ///
    /// Corresponds to Java: `@Override public String toString(FieldMap fieldMap)`
    pub fn to_string_with_field_map(&self, field_map: Option<&dyn VarDataKeyResolver>) -> String {
        let mut result = String::new();

        // Java: pw.println("BEGIN: VarMeta"); pw.println("   Item count: " + m_itemCount); ...
        result.push_str("BEGIN: VarMeta\n");
        result.push_str(&format!("   Item count: {}\n", self.item_count));
        result.push_str(&format!("   Data size: {}\n", self.data_size));

        for (unique_id, map) in &self.table {
            result.push_str(&format!("   Entries for Unique ID: {unique_id}\n"));

            for (type_, offset) in map {
                // Java: FieldType fieldType = fieldMap == null ? null : fieldMap.getFieldTypeFromVarDataKey(entry.getKey());
                let field_type = field_map
                    .and_then(|field_map| field_map.get_field_type_from_var_data_key(*type_));

                // Java: pw.println("      Type=" + (fieldType == null ? entry.getKey() : fieldType) + " Offset=" + entry.getValue());
                let label = match field_type {
                    Some(name) => name,
                    None => type_.to_string(),
                };
                result.push_str(&format!("      Type={label} Offset={offset}\n"));
            }
        }

        // Java: pw.println("END: VarMeta"); pw.println();
        result.push_str("END: VarMeta\n\n");
        result
    }
}

impl VarMeta for AbstractVarMeta {
/// Forward to the inherent method.
    ///
    /// Corresponds to Java: `@Override public int getItemCount()`
    fn get_item_count(&self) -> i32 {
        AbstractVarMeta::get_item_count(self)
    }

/// Forward to the inherent method.
    ///
    /// Corresponds to Java: `@Override public int getDataSize()`
    fn get_data_size(&self) -> i32 {
        AbstractVarMeta::get_data_size(self)
    }

/// Forward to the inherent method.
    ///
    /// Corresponds to Java: `@Override public Integer[] getUniqueIdentifierArray()`
    fn get_unique_identifier_array(&self) -> Vec<i32> {
        AbstractVarMeta::get_unique_identifier_array(self)
    }

/// Forward to the inherent method.
    ///
    /// Corresponds to Java: `@Override public Set<Integer> getUniqueIdentifierSet()`
    fn get_unique_identifier_set(&self) -> BTreeSet<i32> {
        AbstractVarMeta::get_unique_identifier_set(self)
    }

/// Forward to the inherent method.
    ///
    /// Corresponds to Java: `@Override public Integer getOffset(Integer id, Integer type)`
    fn get_offset(&self, id: Option<i32>, type_: Option<i32>) -> Option<i32> {
        AbstractVarMeta::get_offset(self, id, type_)
    }

/// Forward to the inherent method.
    ///
    /// Corresponds to Java: `@Override public int[] getOffsets()`
    fn get_offsets(&self) -> &[i32] {
        AbstractVarMeta::get_offsets(self)
    }

/// Forward to the inherent method.
    ///
    /// Corresponds to Java: `@Override public Set<Integer> getTypes(Integer id)`
    fn get_types(&self, id: Option<i32>) -> BTreeSet<i32> {
        AbstractVarMeta::get_types(self, id)
    }

/// Forward to the inherent method.
    ///
    /// Corresponds to Java: `@Override public boolean containsKey(Integer key)`
    fn contains_key(&self, key: Option<i32>) -> bool {
        AbstractVarMeta::contains_key(self, key)
    }

/// Forward to the inherent method.
    ///
    /// Corresponds to Java: `@Override public String toString(FieldMap fieldMap)`
    fn to_string_with_field_map(&self, field_map: Option<&dyn VarDataKeyResolver>) -> String {
        AbstractVarMeta::to_string_with_field_map(self, field_map)
    }
}

impl Display for AbstractVarMeta {
/// Output the same text as Java's `toString()` (internally calls `toString(null)`).
    ///
    /// Corresponds to Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_with_field_map(None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// Default values for empty metadata.
    #[test]
    fn empty_meta_has_zeroed_defaults() {
        let meta = AbstractVarMeta::new();

        assert_eq!(meta.get_item_count(), 0);
        assert_eq!(meta.get_data_size(), 0);
        assert!(meta.get_offsets().is_empty());
        assert!(meta.get_unique_identifier_array().is_empty());
        assert!(meta.get_unique_identifier_set().is_empty());
        assert!(!meta.contains_key(Some(1)));
        assert_eq!(meta.get_offset(Some(1), Some(2)), None);
        assert!(meta.get_types(Some(1)).is_empty());
    }

/// Sorting and query semantics of the index (consistent with Java's `TreeMap`).
    #[test]
    fn table_lookup_and_ordering_match_java_treemap() {
        let mut meta = AbstractVarMeta::new();
        meta.item_count = 3;
        meta.data_size = 64;
// Insert deliberately out of order to verify that the output order is ascending
        meta.table.insert(9, BTreeMap::from([(0x12, 40)]));
        meta.table.insert(1, BTreeMap::from([(0x12, 4), (0x34, 8)]));
        meta.set_offsets(vec![4, 8, 40]);

        assert_eq!(meta.get_item_count(), 3);
        assert_eq!(meta.get_data_size(), 64);
        assert_eq!(meta.get_unique_identifier_array(), vec![1, 9]);
        assert_eq!(
            meta.get_unique_identifier_set(),
            BTreeSet::from([1, 9])
        );
        assert_eq!(meta.get_offsets(), [4, 8, 40]);
        assert_eq!(meta.get_offset(Some(1), Some(0x34)), Some(8));
        assert_eq!(meta.get_offset(Some(1), Some(0x99)), None);
        assert_eq!(meta.get_offset(Some(7), Some(0x12)), None);
// Java: type == null → null; id == null → NPE (the Rust side degrades gracefully to None)
        assert_eq!(meta.get_offset(Some(1), None), None);
        assert_eq!(meta.get_offset(None, Some(0x12)), None);
        assert_eq!(meta.get_types(Some(1)), BTreeSet::from([0x12, 0x34]));
        assert!(meta.contains_key(Some(9)));
        assert!(!meta.contains_key(Some(10)));
        assert!(!meta.contains_key(None));
    }

/// Text format of `toString()` / `toString(FieldMap)`.
    #[test]
    fn to_string_matches_java_format() {
        let mut meta = AbstractVarMeta::new();
        meta.item_count = 1;
        meta.data_size = 8;
        meta.table.insert(7, BTreeMap::from([(0x12, 4)]));
        meta.set_offsets(vec![4]);

        assert_eq!(
            meta.to_string(),
            "BEGIN: VarMeta\n   Item count: 1\n   Data size: 8\n   \
             Entries for Unique ID: 7\n      Type=18 Offset=4\nEND: VarMeta\n\n"
        );

// When field_map provides a field name, the type identifier is replaced by the field display name
        struct Resolver;
        impl VarDataKeyResolver for Resolver {
            fn get_field_type_from_var_data_key(&self, key: i32) -> Option<String> {
                if key == 0x12 {
                    Some("Notes".to_owned())
                } else {
                    None
                }
            }
        }

        assert!(meta
            .to_string_with_field_map(Some(&Resolver))
            .contains("      Type=Notes Offset=4\n"));
    }

/// The read helpers return `None` when out of bounds instead of panicking (Java throws `EOFException`).
    #[test]
    fn read_helpers_return_none_past_the_end() {
        let data = [0x01u8, 0x02, 0x03];

        let mut position = 0;
        assert_eq!(read_byte(&data, &mut position), Some(1));
        assert_eq!(position, 1);

// Only 2 bytes remain, so reading a 4-byte integer fails and the cursor does not move
        assert_eq!(read_int(&data, &mut position), None);
        assert_eq!(position, 1);

        assert_eq!(read_short(&data, &mut position), Some(0x0302));
        assert_eq!(position, 3);
        assert_eq!(read_byte(&data, &mut position), None);
        assert_eq!(position, 3);
    }

/// De-duplication regression: the read helpers in this file and the same-named methods of [`MPPComponent`] return identical values and advance the cursor identically.
    ///
/// These two modules once each held an equivalent implementation (there is also `props.rs`'s `InputStreamCursor`),
/// and now the former **forwards** to the latter; this test locks down "both sides behave identically".
    #[test]
    fn read_helpers_match_mpp_component() {
        let data = [0xFFu8, 0x34, 0x12, 0x78, 0x56, 0x34, 0x12, 0x01];

        let mut helper_position = 0usize;
        let mut component_position = 0usize;

        assert_eq!(
            read_byte(&data, &mut helper_position),
            MPPComponent::read_byte(&data, &mut component_position)
        );
        assert_eq!(helper_position, component_position);

        assert_eq!(
            read_short(&data, &mut helper_position),
            MPPComponent::read_short(&data, &mut component_position)
        );
        assert_eq!(helper_position, component_position);

        assert_eq!(
            read_int(&data, &mut helper_position),
            MPPComponent::read_int(&data, &mut component_position)
        );
        assert_eq!(helper_position, component_position);
        // 1(byte)+ 2(short)+ 4(int)= 7
        assert_eq!(helper_position, 7);

// Out of bounds: both sides return None likewise and neither moves the cursor
        let short = [0x01u8];
        let mut helper_position = 1usize;
        let mut component_position = 1usize;
        assert_eq!(
            read_int(&short, &mut helper_position),
            MPPComponent::read_int(&short, &mut component_position)
        );
        assert_eq!(helper_position, component_position);
        assert_eq!(helper_position, 1);
    }
}
