//! Corresponds to Java: `org/mpxj/mpp/CustomFieldValueItem.java`
//!
//! One entry in a custom field "value list": unique ID, GUID, value, description, unknown data block, parent ID,
//! value type (`CustomFieldValueDataType`), and collapsed flag.
//!
//! # Differences from Java (structural trade-offs)
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | 8 fields such as `private final Integer m_uniqueID` | **not stored again**; state lives in the shared [`StoredValueItem`] (i.e. `crate::custom_field_lookup_table::CustomFieldValueItem`) | That type is this class's existing Rust implementation (its fields map one-to-one to Java): `CustomFieldContainer::register_value`, `CustomFieldLookupTable::add`, and `VarDataFieldReader` all use it as their element type. If this class stored another copy of the fields, then "the value item in the container" and "the value item in the lookup table" would become two copies of the same logical item, and value list lookups (by unique ID / GUID) would fail to find values set through this class. So this class becomes a **shared handle**: it internally holds `Rc<RefCell<StoredValueItem>>`, and all accessors forward to that storage |
//! | `CustomFieldValueItem(Integer uniqueID)` | [`CustomFieldValueItem::new`], parameter is `i32` | Java's `Integer` is guaranteed non-null by the caller (all call sites use `Integer.valueOf(...)`) |
//! | `Object getValue()` | `Option<FieldValue>` (returns a clone) | `RefCell` cannot lend out the internal value by reference; this crate uniformly uses [`FieldValue`] to carry Java's `Object` field values |
//! | `byte[] getUnknown()` | `Option<Vec<u8>>` | "`null` → `Option<T>`" plus "`byte[]` → `Vec<u8>`" (Java returns an internal array reference; Rust returns a copy) |
//! | `String getDescription()` | `Option<String>` | same as above |
//! | `UUID` / `Integer` / enum fields | `Option<_>` | the "`null` → `Option<T>`" convention |
//! | `equals` / `hashCode` | **not implemented** | Java does not override them (identity is compared by reference); Rust likewise provides no `PartialEq`; to test "is this the same value item", use [`CustomFieldValueItem::stored_item`] with `Rc::ptr_eq` |
//! | `toString()` | [`Display`] | The output format is provided by [`StoredValueItem`]'s implementation of the same name (consistent with Java's `String.format` format) |
//! | — | [`CustomFieldValueItem::stored_item`] / [`CustomFieldValueItem::to_stored_item`] | Rust-specific: hand the "shared handle" / "value copy" to the container and lookup table |
//!
//! Skipped Java methods: none (Java has 1 constructor, 12 accessors, and `toString()`, all translated).
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class uses none of these mechanisms.

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use uuid::Uuid;

use crate::custom_field_lookup_table::CustomFieldValueItem as StoredValueItem;
use crate::custom_field_value_data_type::CustomFieldValueDataType;
use crate::field_value::FieldValue;

/// An entry in a value list.
///
/// Corresponds to Java: `public final class CustomFieldValueItem`
///
/// `Clone` is Rust-specific: it clones the **shared handle** (`Rc`), so both handles see the same item.
#[derive(Clone)]
pub struct CustomFieldValueItem {
/// Value item storage.
    ///
/// Corresponds to the 8 Java fields (`m_uniqueID` / `m_guid` / `m_value` / `m_description` /
/// `m_unknown` / `m_parentUniqueID` / `m_type` / `m_collapsed`); see the file header.
    stored: Rc<RefCell<StoredValueItem>>,
}

impl CustomFieldValueItem {
/// Constructor.
    ///
    /// Corresponds to Java: `public CustomFieldValueItem(Integer uniqueID)`
    pub fn new(unique_id: i32) -> Self {
        Self {
            stored: Rc::new(RefCell::new(StoredValueItem::new(unique_id))),
        }
    }

/// Get the unique ID.
    ///
    /// Corresponds to Java: `public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> i32 {
        self.stored.borrow().get_unique_id()
    }

/// Set the value.
    ///
    /// Corresponds to Java: `public void setValue(Object value)`
    pub fn set_value(&self, value: Option<FieldValue>) {
        self.stored.borrow_mut().set_value(value);
    }

/// Get the value.
    ///
    /// Corresponds to Java: `public Object getValue()`
    ///
/// Java returns an internal reference; on the Rust side `RefCell` cannot lend it out, so a copy is returned.
    pub fn get_value(&self) -> Option<FieldValue> {
        self.stored.borrow().get_value().cloned()
    }

/// Set the description.
    ///
    /// Corresponds to Java: `public void setDescription(String description)`
    pub fn set_description(&self, description: Option<String>) {
        self.stored.borrow_mut().set_description(description);
    }

/// Get the description.
    ///
    /// Corresponds to Java: `public String getDescription()`
    pub fn get_description(&self) -> Option<String> {
        self.stored.borrow().get_description().map(str::to_owned)
    }

/// Set the unknown data block.
    ///
    /// Corresponds to Java: `public void setUnknown(byte[] unknown)`
    pub fn set_unknown(&self, unknown: Option<Vec<u8>>) {
        self.stored.borrow_mut().set_unknown(unknown);
    }

/// Get the unknown data block.
    ///
    /// Corresponds to Java: `public byte[] getUnknown()`
    pub fn get_unknown(&self) -> Option<Vec<u8>> {
        self.stored.borrow().get_unknown().map(<[u8]>::to_vec)
    }

/// Set the parent unique ID.
    ///
    /// Corresponds to Java: `public void setParentUniqueID(Integer id)`
    pub fn set_parent_unique_id(&self, id: Option<i32>) {
        self.stored.borrow_mut().set_parent_unique_id(id);
    }

/// Get the parent unique ID.
    ///
    /// Corresponds to Java: `public Integer getParentUniqueID()`
    pub fn get_parent_unique_id(&self) -> Option<i32> {
        self.stored.borrow().get_parent_unique_id()
    }

/// Get the GUID.
    ///
    /// Corresponds to Java: `public UUID getGUID()`
    pub fn get_guid(&self) -> Option<Uuid> {
        self.stored.borrow().get_guid()
    }

/// Set the GUID.
    ///
    /// Corresponds to Java: `public void setGUID(UUID guid)`
    pub fn set_guid(&self, guid: Option<Uuid>) {
        self.stored.borrow_mut().set_guid(guid);
    }

/// Get the value type.
    ///
    /// Corresponds to Java: `public CustomFieldValueDataType getType()`
    pub fn get_type(&self) -> Option<CustomFieldValueDataType> {
        self.stored.borrow().get_type()
    }

/// Set the value type.
    ///
    /// Corresponds to Java: `public void setType(CustomFieldValueDataType type)`
    pub fn set_type(&self, type_: Option<CustomFieldValueDataType>) {
        self.stored.borrow_mut().set_type(type_);
    }

/// Get the collapsed flag.
    ///
    /// Corresponds to Java: `public boolean getCollapsed()`
    pub fn get_collapsed(&self) -> bool {
        self.stored.borrow().get_collapsed()
    }

/// Set the collapsed flag.
    ///
    /// Corresponds to Java: `public void setCollapsed(boolean collapsed)`
    pub fn set_collapsed(&self, collapsed: bool) {
        self.stored.borrow_mut().set_collapsed(collapsed);
    }

/// Get the shared storage handle (Rust-specific).
    ///
/// Used by `CustomFieldContainer::register_value`: after registering into the container, a container lookup by unique ID / GUID
/// finds this item (corresponding to Java's "the same `CustomFieldValueItem` instance goes into both the container and the lookup table").
    pub fn stored_item(&self) -> Rc<RefCell<StoredValueItem>> {
        Rc::clone(&self.stored)
    }

/// Get a value copy of the shared storage (Rust-specific).
    ///
/// Used by `CustomFieldLookupTable::add` / `set`: the lookup table stores elements by value (Java stores
/// an instance reference), so a storage value with identical content is cloned here.
    pub fn to_stored_item(&self) -> StoredValueItem {
        self.stored.borrow().clone()
    }
}

impl Display for CustomFieldValueItem {
    /// Corresponds to Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Java: String.format("[CustomFieldValueItem uniqueID=%d guid=%s parentId=%d value=%s description=%s]", ...)
        Display::fmt(&*self.stored.borrow(), f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// The 12 accessors map one-to-one to the Java fields and round-trip.
    #[test]
    fn accessors_round_trip() {
        let item = CustomFieldValueItem::new(4);
        let guid = Uuid::from_u128(42);

// Java initial values: all are null / false except uniqueID
        assert_eq!(item.get_unique_id(), 4);
        assert_eq!(item.get_guid(), None);
        assert_eq!(item.get_value(), None);
        assert_eq!(item.get_description(), None);
        assert_eq!(item.get_unknown(), None);
        assert_eq!(item.get_parent_unique_id(), None);
        assert_eq!(item.get_type(), None);
        assert!(!item.get_collapsed());

        item.set_guid(Some(guid));
        item.set_value(Some(FieldValue::Integer(1)));
        item.set_description(Some("desc".to_owned()));
        item.set_unknown(Some(vec![1, 2, 3]));
        item.set_parent_unique_id(Some(2));
        item.set_type(Some(CustomFieldValueDataType::Number));
        item.set_collapsed(true);

        assert_eq!(item.get_guid(), Some(guid));
        assert_eq!(item.get_value(), Some(FieldValue::Integer(1)));
        assert_eq!(item.get_description(), Some("desc".to_owned()));
        assert_eq!(item.get_unknown(), Some(vec![1, 2, 3]));
        assert_eq!(item.get_parent_unique_id(), Some(2));
        assert_eq!(item.get_type(), Some(CustomFieldValueDataType::Number));
        assert!(item.get_collapsed());
    }

/// The clone is another handle to the same item (corresponding to Java's reference semantics).
    #[test]
    fn clone_shares_the_same_item() {
        let item = CustomFieldValueItem::new(7);
        let copy = item.clone();

        assert!(Rc::ptr_eq(&item.stored_item(), &copy.stored_item()));

        copy.set_description(Some("shared".to_owned()));
        assert_eq!(item.get_description(), Some("shared".to_owned()));
    }

/// `to_stored_item` yields a value copy with identical content (decoupled from the shared handle).
    #[test]
    fn to_stored_item_copies_the_value() {
        let item = CustomFieldValueItem::new(9);
        item.set_value(Some(FieldValue::Double(1.5)));

        let stored = item.to_stored_item();
        assert_eq!(stored.get_unique_id(), 9);
        assert_eq!(stored.get_value(), Some(&FieldValue::Double(1.5)));

        item.set_value(None);
// the copy is unaffected
        assert_eq!(stored.get_value(), Some(&FieldValue::Double(1.5)));
    }

/// `toString` matches the Java format.
    #[test]
    fn display_matches_java_format() {
        let item = CustomFieldValueItem::new(4);
        item.set_description(Some("desc".to_owned()));
        item.set_parent_unique_id(Some(2));

        assert_eq!(
            item.to_string(),
            "[CustomFieldValueItem uniqueID=4 guid=null parentId=2 value=null description=desc]"
        );
    }
}
