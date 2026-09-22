//! Corresponds to Java: `org/mpxj/mpp/VarMeta12.java`
//!
//! VarMeta block parsing for MPP 12 and later: after the block header come `item count` records of "4-byte unique ID + 4-byte offset +
//! 2-byte type + 2-byte unknown" (12 bytes each).
//!
//! # Restoring inheritance (composition + forwarding)
//!
//! Java: `final class VarMeta12 extends AbstractVarMeta`. Rust has no inheritance, therefore:
//! - [`VarMeta12`] **composes** an [`AbstractVarMeta`] (base class state and implementation);
//! - all public methods of the base class are **forwarded one by one** in this file, and `impl VarMeta` makes it placeable in
//! `Rc<dyn VarMeta>` handed to [`crate::mpp::var2_data::Var2Data`].
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `VarMeta12(InputStream is) throws IOException` | `VarMeta12::new(&[u8]) -> MpxjResult<VarMeta12>` | By convention `InputStream` → `&[u8]`; `IOException` (bad magic) → `MpxjError` |
//! | `break` when `is.available() < 12` | `break` when `data.len() - position < 12` | Equivalent (Rust has no stream, so the remaining byte count is used instead) |
//! | `readInt` / `readShort` (inherited from `MPPComponent`, throws `EOFException` when out of bounds) | [`read_int`] / [`read_short`] return `None` and the loop `break`s to end parsing early | Follows the "library code must not panic" convention |
//! | `new int[m_itemCount]` throws `NegativeArraySizeException` when `m_itemCount` is negative | The loop does not execute and the offset array is empty | Same as above |
//! | `m_table.computeIfAbsent(..)` | `BTreeMap::entry` + `or_default()` | Equivalent |
//!
//! Note: Java's comment states that MS Project itself can read some files whose magic is 0, so **0 is also a valid
//! magic number** (unlike [`crate::mpp::var_meta9::VarMeta9`]).
//!
//! Skipped Java methods: none.
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::mpp::abstract_var_meta::{read_int, read_short, AbstractVarMeta};
use crate::mpp::var_meta::{VarDataKeyResolver, VarMeta};

/// magic number (in Java `int MAGIC = 0xFADFADBA` is a negative constant; the same semantics are preserved here).
const MAGIC: i32 = 0xFADF_ADBA_u32 as i32;

/// Number of bytes per record (4 + 4 + 2 + 2).
///
/// Corresponds to Java: the literal 12 in `if (is.available() < 12) { break; }`.
const ITEM_SIZE: usize = 12;

/// Var-length block metadata for MPP 12 and later.
///
/// Corresponds to Java: `final class VarMeta12 extends AbstractVarMeta`
pub struct VarMeta12 {
/// The base class portion (in Java `VarMeta12` itself is `AbstractVarMeta`).
    meta: AbstractVarMeta,
}

impl VarMeta12 {
/// Constructor: parse metadata from the VarMeta block byte stream.
    ///
    /// Corresponds to Java: `VarMeta12(InputStream is) throws IOException`
    ///
/// The block header is a fixed 24 bytes: magic(4) + unknown(4) + item count(4) + unknown(4) + unknown(4) + data area size(4);
/// followed by 12 bytes per record: unique ID(4) + offset(4) + type(2) + unknown(2).
    pub fn new(data: &[u8]) -> MpxjResult<Self> {
        let mut position = 0usize;

// Java comment: in one sample the VarMeta block magic, which should have been valid, was 0 and MS Project could still read it, so 0 is treated as valid.
        // Java: int magic = readInt(is); if (magic != 0 && magic != MAGIC) { throw new IOException("Bad magic number: " + magic); }
        let magic = read_int(data, &mut position).unwrap_or(0);
        if magic != 0 && magic != MAGIC {
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

        for _ in 0..item_count {
            // Java: if (is.available() < 12) { break; }
            if data.len().saturating_sub(position) < ITEM_SIZE {
                break;
            }

            // Java: Integer uniqueID = Integer.valueOf(readInt(is));
            let Some(unique_id) = read_int(data, &mut position) else {
                break;
            };
            // Java: Integer offset = Integer.valueOf(readInt(is));
            let Some(offset) = read_int(data, &mut position) else {
                break;
            };
            // Java: Integer type = Integer.valueOf(readShort(is));
            let Some(type_) = read_short(data, &mut position) else {
                break;
            };
            // Java: readShort(is); // unknown 2 bytes
            let _unknown = read_short(data, &mut position);

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

/// Get the base class portion (Rust-specific: in Java `VarMeta12` itself is [`AbstractVarMeta`]).
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

impl VarMeta for VarMeta12 {
/// Forwards to the base class implementation.
    ///
    /// Corresponds to Java: `AbstractVarMeta#getItemCount()`
    fn get_item_count(&self) -> i32 {
        VarMeta12::get_item_count(self)
    }

/// Forwards to the base class implementation.
    ///
    /// Corresponds to Java: `AbstractVarMeta#getDataSize()`
    fn get_data_size(&self) -> i32 {
        VarMeta12::get_data_size(self)
    }

/// Forwards to the base class implementation.
    ///
    /// Corresponds to Java: `AbstractVarMeta#getUniqueIdentifierArray()`
    fn get_unique_identifier_array(&self) -> Vec<i32> {
        VarMeta12::get_unique_identifier_array(self)
    }

/// Forwards to the base class implementation.
    ///
    /// Corresponds to Java: `AbstractVarMeta#getUniqueIdentifierSet()`
    fn get_unique_identifier_set(&self) -> BTreeSet<i32> {
        VarMeta12::get_unique_identifier_set(self)
    }

/// Forwards to the base class implementation.
    ///
    /// Corresponds to Java: `AbstractVarMeta#getOffset(Integer id, Integer type)`
    fn get_offset(&self, id: Option<i32>, type_: Option<i32>) -> Option<i32> {
        VarMeta12::get_offset(self, id, type_)
    }

/// Forwards to the base class implementation.
    ///
    /// Corresponds to Java: `AbstractVarMeta#getOffsets()`
    fn get_offsets(&self) -> &[i32] {
        VarMeta12::get_offsets(self)
    }

/// Forwards to the base class implementation.
    ///
    /// Corresponds to Java: `AbstractVarMeta#getTypes(Integer id)`
    fn get_types(&self, id: Option<i32>) -> BTreeSet<i32> {
        VarMeta12::get_types(self, id)
    }

/// Forwards to the base class implementation.
    ///
    /// Corresponds to Java: `AbstractVarMeta#containsKey(Integer key)`
    fn contains_key(&self, key: Option<i32>) -> bool {
        VarMeta12::contains_key(self, key)
    }

/// Forwards to the base class implementation.
    ///
    /// Corresponds to Java: `AbstractVarMeta#toString(FieldMap fieldMap)`
    fn to_string_with_field_map(&self, field_map: Option<&dyn VarDataKeyResolver>) -> String {
        VarMeta12::to_string_with_field_map(self, field_map)
    }
}

impl Display for VarMeta12 {
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

/// Assemble a VarMeta12 block: header + per record "unique ID(4) + offset(4) + type(2) + unknown(2)".
    fn var_meta12_bytes(magic: i32, items: &[(i32, i32, i32)], data_size: i32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&magic.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(items.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&data_size.to_le_bytes());

        for (unique_id, offset, type_) in items {
            data.extend_from_slice(&unique_id.to_le_bytes());
            data.extend_from_slice(&offset.to_le_bytes());
            data.extend_from_slice(&(*type_ as i16).to_le_bytes());
            data.extend_from_slice(&0i16.to_le_bytes());
        }

        data
    }

/// Parse the item count, index, and offset array.
    #[test]
    fn parses_items_offsets_and_types() {
        let bytes = var_meta12_bytes(MAGIC, &[(2, 40, 0x12), (1, 4, 0x34), (1, 8, 0x12)], 64);
        let meta = VarMeta12::new(&bytes).expect("valid block header");

        assert_eq!(meta.get_item_count(), 3);
        assert_eq!(meta.get_data_size(), 64);
        assert_eq!(meta.get_offsets(), [4, 8, 40]);
        assert_eq!(meta.get_unique_identifier_array(), vec![1, 2]);
        assert_eq!(meta.get_types(Some(1)), BTreeSet::from([0x12, 0x34]));
        assert_eq!(meta.get_offset(Some(1), Some(0x34)), Some(4));
        assert_eq!(meta.get_offset(Some(2), Some(0x12)), Some(40));
        assert!(meta.contains_key(Some(2)));

// Forwarding and Display
        assert_eq!(meta.get_meta().get_item_count(), 3);
        assert!(meta.to_string().ends_with("END: VarMeta\n\n"));
    }

/// A magic of 0 is also accepted (Java comment: some valid files are like this).
    #[test]
    fn accepts_zero_magic_number() {
        let bytes = var_meta12_bytes(0, &[(1, 4, 0x12)], 8);
        let meta = VarMeta12::new(&bytes).expect("magic 0 is valid");
        assert_eq!(meta.get_unique_identifier_array(), vec![1]);
    }

/// Any other magic value → error.
    #[test]
    fn rejects_bad_magic_number() {
        let bytes = var_meta12_bytes(0x01020304, &[], 0);
        let error = VarMeta12::new(&bytes).err().expect("magic mismatch should error");
        assert_eq!(error.message(), "Bad magic number: 16909060");
    }

/// Ends early without panicking when the remaining bytes are fewer than one record (12 bytes).
    #[test]
    fn stops_when_fewer_than_one_item_remains() {
// itemCount claims 3 records, but only 1 record + 6 leftover bytes are actually provided
        let mut bytes = var_meta12_bytes(MAGIC, &[(1, 4, 0x12), (2, 8, 0x12), (3, 12, 0x12)], 16);
        bytes.truncate(24 + 12 + 6);

        let meta = VarMeta12::new(&bytes).expect("valid block header");
        assert_eq!(meta.get_item_count(), 3);
        assert_eq!(meta.get_unique_identifier_array(), vec![1]);
        assert_eq!(meta.get_offsets(), [4]);
    }

/// Empty input does not panic.
    #[test]
    fn empty_input_does_not_panic() {
// Block header cannot be read → magic treated as 0 (valid), item count is 0
        let meta = VarMeta12::new(&[]).expect("magic 0 is valid");
        assert_eq!(meta.get_item_count(), 0);
        assert!(meta.get_unique_identifier_array().is_empty());
    }
}
