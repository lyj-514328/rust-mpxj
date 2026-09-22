//! Corresponds to Java: `org/mpxj/mpp/VarMeta9.java`
//!
//! VarMeta block parsing for MPP 9 and earlier: after the block header come `item count` fixed-length records of "3-byte unique ID + 1-byte type +
//! 4-byte offset".
//!
//! # Inheritance reconstruction (composition + forwarding)
//!
//! Java: `final class VarMeta9 extends AbstractVarMeta`. Rust has no inheritance, so:
//! - [`VarMeta9`] **composes** an [`AbstractVarMeta`] (base class state and implementation);
//! - all public methods of the base class are **forwarded one by one** in this file, and `impl VarMeta` is provided so that `VarMeta9` and
//! `AbstractVarMeta` are equivalent in use (can be placed into `Rc<dyn VarMeta>` and handed to `Var2Data`).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `VarMeta9(InputStream is) throws IOException` | `VarMeta9::new(&[u8]) -> MpxjResult<VarMeta9>` | by convention `InputStream` -> `&[u8]`; `IOException` (bad magic) -> `MpxjError` |
//! | `readInt` / `readByte` (inherited from `MPPComponent`, throws `EOFException` on out-of-bounds) | [`read_int`] / [`read_byte`] return `None`, and the loop `break`s to end parsing early | follows the "library code never panics" convention (in Java construction fails at this point) |
//! | when `m_itemCount` is negative, `new int[m_itemCount]` throws `NegativeArraySizeException` | the loop does not execute and the offset array is empty | same as above |
//! | `m_table.computeIfAbsent(..)` | `BTreeMap::entry` + `or_default()` | equivalent |
//! | `3-byte unique ID` borrowed into a 4-byte array (last byte always 0) + parsed by `ByteArrayHelper.getInt` | same approach as Java, preserving its sign semantics | an MPP detail: the unique ID is stored as a 3-byte little-endian value |
//!
//! Skipped Java methods: none.
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::mpp::abstract_var_meta::{read_byte, read_int, AbstractVarMeta};
use crate::mpp::var_meta::{VarDataKeyResolver, VarMeta};

/// magic number (in Java `int MAGIC = 0xFADFADBA` is a negative constant; the same semantics are preserved here).
const MAGIC: i32 = 0xFADF_ADBA_u32 as i32;

/// Variable-length block metadata for MPP 9 and earlier.
///
/// Corresponds to Java: `final class VarMeta9 extends AbstractVarMeta`
pub struct VarMeta9 {
/// The base class part (in Java `VarMeta9` is itself an `AbstractVarMeta`).
    meta: AbstractVarMeta,
}

impl VarMeta9 {
/// Constructor: parse the metadata from the VarMeta block's byte stream.
    ///
    /// Corresponds to Java: `VarMeta9(InputStream is) throws IOException`
    ///
/// The block header is a fixed 24 bytes: magic(4) + unknown(4) + item count(4) + unknown(4) + unknown(4) + data area size(4);
/// after that each record is 8 bytes: unique ID(3) + type(1) + offset(4).
    pub fn new(data: &[u8]) -> MpxjResult<Self> {
        let mut position = 0usize;

        // Java: int magic = readInt(is); if (magic != MAGIC) { throw new IOException("Bad magic number: " + magic); }
// (Java throws EOFException when fewer than 4 bytes are available; the Rust side substitutes 0)
        let magic = read_int(data, &mut position).unwrap_or(0);
        if magic != MAGIC {
            return Err(MpxjError::new(format!("Bad magic number: {magic}")));
        }

        // Java: /*m_unknown1 =*/ readInt(is);
        let _unknown1 = read_int(data, &mut position);
        // Java: m_itemCount = readInt(is);
        let item_count = read_int(data, &mut position).unwrap_or(0);
        // Java: /*m_unknown2 =*/ readInt(is); /*m_unknown3 =*/ readInt(is);
        let _unknown2 = read_int(data, &mut position);
        let _unknown3 = read_int(data, &mut position);
        // Java: m_dataSize = readInt(is);
        let data_size = read_int(data, &mut position).unwrap_or(0);

        let mut table: BTreeMap<i32, BTreeMap<i32, i32>> = BTreeMap::new();
        let mut offsets: Vec<i32> = Vec::new();

        //
// Java comment: Microsoft is a bit odd here -- the unique ID is stored as a 3-byte integer.
// Follow Java's approach: read into a 4-byte array with the last byte always 0, then parse as little-endian.
        //
        let mut unique_id_array = [0u8; 4];

        // Java: for (int loop = 0; loop < m_itemCount; loop++)
// (when item_count is negative the Rust range is empty; see the file header for the difference from Java's "construction fails")
        for _ in 0..item_count {
            // Java: InputStreamHelper.read(is, uniqueIDArray, 3);
            if position + 3 > data.len() {
// Java throws EOFException (construction fails); the Rust side ends parsing early without panicking.
                break;
            }
            unique_id_array[..3].copy_from_slice(&data[position..position + 3]);
            position += 3;

            // Java: uniqueID = Integer.valueOf(ByteArrayHelper.getInt(uniqueIDArray, 0));
            let unique_id = ByteArrayHelper::get_int(&unique_id_array, 0);

            // Java: type = Integer.valueOf(readByte(is));
            let Some(type_) = read_byte(data, &mut position) else {
                break;
            };

            // Java: offset = Integer.valueOf(readInt(is));
            let Some(offset) = read_int(data, &mut position) else {
                break;
            };

            // Java: map = m_table.computeIfAbsent(uniqueID, k -> new TreeMap<>()); map.put(type, offset);
            table.entry(unique_id).or_default().insert(type_, offset);

            // Java: offsets[loop] = offset.intValue();
            offsets.push(offset);
        }

        // Java: Arrays.sort(offsets); setOffsets(offsets);
        offsets.sort();

        let mut meta = AbstractVarMeta::new();
        meta.item_count = item_count;
        meta.data_size = data_size;
        meta.table = table;
        meta.set_offsets(offsets);

        Ok(Self { meta })
    }

/// Get the base class part (Rust-specific: in Java `VarMeta9` is itself an [`AbstractVarMeta`]).
    pub fn get_meta(&self) -> &AbstractVarMeta {
        &self.meta
    }

/// Get the item count (forwards to the base class).
    ///
    /// Corresponds to Java: `AbstractVarMeta#getItemCount()`
    pub fn get_item_count(&self) -> i32 {
        self.meta.get_item_count()
    }

/// Get the data area size (forwards to the base class).
    ///
    /// Corresponds to Java: `AbstractVarMeta#getDataSize()`
    pub fn get_data_size(&self) -> i32 {
        self.meta.get_data_size()
    }

/// Get all unique IDs (ascending, forwards to the base class).
    ///
    /// Corresponds to Java: `AbstractVarMeta#getUniqueIdentifierArray()`
    pub fn get_unique_identifier_array(&self) -> Vec<i32> {
        self.meta.get_unique_identifier_array()
    }

/// Get the set of all unique IDs (ascending, forwards to the base class).
    ///
    /// Corresponds to Java: `AbstractVarMeta#getUniqueIdentifierSet()`
    pub fn get_unique_identifier_set(&self) -> BTreeSet<i32> {
        self.meta.get_unique_identifier_set()
    }

/// Get the offset by "unique ID + type" (forwards to the base class).
    ///
    /// Corresponds to Java: `AbstractVarMeta#getOffset(Integer id, Integer type)`
    pub fn get_offset(&self, id: Option<i32>, type_: Option<i32>) -> Option<i32> {
        self.meta.get_offset(id, type_)
    }

/// Get the sorted offset array (forwards to the base class).
    ///
    /// Corresponds to Java: `AbstractVarMeta#getOffsets()`
    pub fn get_offsets(&self) -> &[i32] {
        self.meta.get_offsets()
    }

/// Get the types defined under the given unique ID (ascending, forwards to the base class).
    ///
    /// Corresponds to Java: `AbstractVarMeta#getTypes(Integer id)`
    pub fn get_types(&self, id: Option<i32>) -> BTreeSet<i32> {
        self.meta.get_types(id)
    }

/// Check whether the given unique ID exists in the index (forwards to the base class).
    ///
    /// Corresponds to Java: `AbstractVarMeta#containsKey(Integer key)`
    pub fn contains_key(&self, key: Option<i32>) -> bool {
        self.meta.contains_key(key)
    }

/// Export as debug text (forwards to the base class).
    ///
    /// Corresponds to Java: `AbstractVarMeta#toString(FieldMap fieldMap)`
    pub fn to_string_with_field_map(&self, field_map: Option<&dyn VarDataKeyResolver>) -> String {
        self.meta.to_string_with_field_map(field_map)
    }
}

impl VarMeta for VarMeta9 {
/// Forwards to the base class implementation.
    ///
    /// Corresponds to Java: `AbstractVarMeta#getItemCount()`
    fn get_item_count(&self) -> i32 {
        VarMeta9::get_item_count(self)
    }

/// Forwards to the base class implementation.
    ///
    /// Corresponds to Java: `AbstractVarMeta#getDataSize()`
    fn get_data_size(&self) -> i32 {
        VarMeta9::get_data_size(self)
    }

/// Forwards to the base class implementation.
    ///
    /// Corresponds to Java: `AbstractVarMeta#getUniqueIdentifierArray()`
    fn get_unique_identifier_array(&self) -> Vec<i32> {
        VarMeta9::get_unique_identifier_array(self)
    }

/// Forwards to the base class implementation.
    ///
    /// Corresponds to Java: `AbstractVarMeta#getUniqueIdentifierSet()`
    fn get_unique_identifier_set(&self) -> BTreeSet<i32> {
        VarMeta9::get_unique_identifier_set(self)
    }

/// Forwards to the base class implementation.
    ///
    /// Corresponds to Java: `AbstractVarMeta#getOffset(Integer id, Integer type)`
    fn get_offset(&self, id: Option<i32>, type_: Option<i32>) -> Option<i32> {
        VarMeta9::get_offset(self, id, type_)
    }

/// Forwards to the base class implementation.
    ///
    /// Corresponds to Java: `AbstractVarMeta#getOffsets()`
    fn get_offsets(&self) -> &[i32] {
        VarMeta9::get_offsets(self)
    }

/// Forwards to the base class implementation.
    ///
    /// Corresponds to Java: `AbstractVarMeta#getTypes(Integer id)`
    fn get_types(&self, id: Option<i32>) -> BTreeSet<i32> {
        VarMeta9::get_types(self, id)
    }

/// Forwards to the base class implementation.
    ///
    /// Corresponds to Java: `AbstractVarMeta#containsKey(Integer key)`
    fn contains_key(&self, key: Option<i32>) -> bool {
        VarMeta9::contains_key(self, key)
    }

/// Forwards to the base class implementation.
    ///
    /// Corresponds to Java: `AbstractVarMeta#toString(FieldMap fieldMap)`
    fn to_string_with_field_map(&self, field_map: Option<&dyn VarDataKeyResolver>) -> String {
        VarMeta9::to_string_with_field_map(self, field_map)
    }
}

impl Display for VarMeta9 {
/// Output the same text as Java's `toString()`.
    ///
    /// Corresponds to Java: `AbstractVarMeta#toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.meta.to_string_with_field_map(None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// Assemble a VarMeta9 block: block header + each record "unique ID(3) + type(1) + offset(4)".
    fn var_meta9_bytes(items: &[(i32, i32, i32)], data_size: i32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&MAGIC.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(items.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&data_size.to_le_bytes());

        for (unique_id, type_, offset) in items {
            data.extend_from_slice(&unique_id.to_le_bytes()[..3]);
            data.push(*type_ as u8);
            data.extend_from_slice(&offset.to_le_bytes());
        }

        data
    }

/// Parse the item count, index, and offset array.
    #[test]
    fn parses_items_offsets_and_types() {
// Deliberately supplied out of order to verify the offset array is sorted and unique IDs are output ascending
        let bytes = var_meta9_bytes(&[(2, 0x12, 40), (1, 0x34, 4), (1, 0x12, 8)], 64);
        let meta = VarMeta9::new(&bytes).expect("valid block header");

        assert_eq!(meta.get_item_count(), 3);
        assert_eq!(meta.get_data_size(), 64);
        assert_eq!(meta.get_offsets(), [4, 8, 40]);
        assert_eq!(meta.get_unique_identifier_array(), vec![1, 2]);
        assert_eq!(meta.get_unique_identifier_set(), BTreeSet::from([1, 2]));
        assert_eq!(meta.get_types(Some(1)), BTreeSet::from([0x12, 0x34]));
        assert_eq!(meta.get_types(Some(2)), BTreeSet::from([0x12]));
        assert!(meta.get_types(Some(3)).is_empty());
        assert_eq!(meta.get_offset(Some(1), Some(0x34)), Some(4));
        assert_eq!(meta.get_offset(Some(1), Some(0x99)), None);
        assert!(meta.contains_key(Some(2)));
        assert!(!meta.contains_key(Some(3)));

// Forwarding and Display
        assert_eq!(meta.get_meta().get_item_count(), 3);
        assert!(meta.to_string().starts_with("BEGIN: VarMeta\n   Item count: 3\n"));
// Under the same unique ID, output in ascending data type order (consistent with Java's TreeMap)
        assert!(meta
            .to_string()
            .contains("Entries for Unique ID: 1\n      Type=18 Offset=8\n      Type=52 Offset=4\n"));
    }

/// The unique ID is stored as 3 bytes (the 4th byte is always 0); verify the sign semantics.
    #[test]
    fn unique_id_is_read_from_three_bytes() {
// The most significant byte of 0x00FFFFFF is 0, so it is the positive number 16777215
        let bytes = var_meta9_bytes(&[(0xFFFFFF, 1, 0)], 1);
        let meta = VarMeta9::new(&bytes).expect("valid block header");

        assert_eq!(meta.get_unique_identifier_array(), vec![0xFFFFFF]);
    }

/// Bad magic number -> error (Java throws IOException).
    #[test]
    fn rejects_bad_magic_number() {
        let mut bytes = var_meta9_bytes(&[], 0);
        bytes[..4].copy_from_slice(&0x01020304_u32.to_le_bytes());

        let error = VarMeta9::new(&bytes).err().expect("magic mismatch should error");
        assert_eq!(error.message(), "Bad magic number: 16909060");
    }

/// Truncated / empty input does not panic (Java throws EOFException).
    #[test]
    fn truncated_input_does_not_panic() {
        assert!(VarMeta9::new(&[]).is_err());

// The block header is complete, but only one record is supplied (itemCount claims 3)
        let mut bytes = var_meta9_bytes(&[(1, 0x12, 4), (2, 0x12, 8), (3, 0x12, 12)], 16);
        bytes.truncate(24 + 8 + 3);

        let meta = VarMeta9::new(&bytes).expect("valid block header");
        assert_eq!(meta.get_item_count(), 3);
        assert_eq!(meta.get_unique_identifier_array(), vec![1]);
        assert_eq!(meta.get_offsets(), [4]);
    }
}
