//! Corresponds to Java: `org/mpxj/CustomFieldLookupTable.java`

use std::fmt::{self, Display, Formatter};
use uuid::Uuid;

use crate::custom_field_value_data_type::CustomFieldValueDataType;
use crate::field_value::FieldValue;
use crate::list_with_callbacks::ListWithCallbacks;

/// Lookup table of custom field values.
///
/// Corresponds to Java: `public class CustomFieldLookupTable extends ListWithCallbacks<CustomFieldValueItem>`
///
/// # How the inheritance hierarchy is restored
///
/// The Java side extends `ListWithCallbacks<CustomFieldValueItem>`; Rust has no inheritance, so **composition** is used instead:
/// it holds a [`ListWithCallbacks<CustomFieldValueItem>`] internally and forwards the list operations
/// ([`add`](CustomFieldLookupTable::add) / [`get`](CustomFieldLookupTable::get) /
/// [`remove`](CustomFieldLookupTable::remove), etc.).
///
/// Java's `CustomFieldLookupTable` does not override the `added` / `removed` / `replaced` hooks again,
/// so the three callbacks remain unregistered (equivalent to Java's empty implementation); the owner can use
/// [`ListWithCallbacks::set_added_callback`], etc., to register them itself.
///
/// # Element type: field carrier + the shared handle on the `mpp` side
///
/// Java's element type is `org.mpxj.mpp.CustomFieldValueItem`; on the Rust side this type is implemented in two halves:
///
/// - [`CustomFieldValueItem`] in this file: the **carrier** of the 8 Java fields (accessors take `&mut self`),
///   and also the element type of this list and of [`crate::custom_field_container::CustomFieldContainer`];
/// - [`crate::mpp::custom_field_value_item::CustomFieldValueItem`]: the **shared handle** for the same Java class
///   (internally holds `Rc<RefCell<CustomFieldValueItem>>`, accessors take `&self`), letting the MPP reader reproduce Java's
///   "the same value item enters both the container and the lookup table" (that module refers to this type via
///   `use crate::custom_field_lookup_table::CustomFieldValueItem as StoredValueItem`).
///
/// The two share the same fields (there is no duplicate state), so they cannot be merged into a single type: the list stores by value,
/// while the handle shares by index; both semantics are needed.
///
/// Java methods not ported: none.
#[derive(Debug)]
pub struct CustomFieldLookupTable {
    /// Lookup table contents.
    ///
    /// Corresponds to Java: the list storage inherited from `ListWithCallbacks<CustomFieldValueItem>`.
    entries: ListWithCallbacks<CustomFieldValueItem>,
    /// Lookup table GUID.
    ///
    /// Corresponds to Java: `private UUID m_guid;`
    m_guid: Option<Uuid>,
    /// Enterprise flag.
    ///
    /// Corresponds to Java: `private boolean m_enterprise;`
    m_enterprise: bool,
    /// Show indent flag (initial value `true` in Java).
    ///
    /// Corresponds to Java: `private boolean m_showIndent = true;`
    m_show_indent: bool,
    /// Resource substitution enabled flag.
    ///
    /// Corresponds to Java: `private boolean m_resourceSubstitutionEnabled;`
    m_resource_substitution_enabled: bool,
    /// Leaf nodes only flag.
    ///
    /// Corresponds to Java: `private boolean m_leafOnly;`
    m_leaf_only: bool,
    /// Required at all levels flag.
    ///
    /// Corresponds to Java: `private boolean m_allLevelsRequired;`
    m_all_levels_required: bool,
    /// Only table values allowed flag (initial value `true` in Java).
    ///
    /// Corresponds to Java: `private boolean m_onlyTableValuesAllowed = true;`
    m_only_table_values_allowed: bool,
}

impl CustomFieldLookupTable {
    /// Constructor.
    ///
    /// Corresponds to Java: `CustomFieldLookupTable()` (implicit default constructor; `m_showIndent` and
    /// `m_onlyTableValuesAllowed` are initially `true`).
    pub fn new() -> Self {
        Self {
            entries: ListWithCallbacks::new(),
            m_guid: None,
            m_enterprise: false,
            m_show_indent: true,
            m_resource_substitution_enabled: false,
            m_leaf_only: false,
            m_all_levels_required: false,
            m_only_table_values_allowed: true,
        }
    }

    /// Get the lookup table GUID.
    ///
    /// Corresponds to Java: `public UUID getGUID()`
    pub fn get_guid(&self) -> Option<Uuid> {
        self.m_guid
    }

    /// Set the lookup table GUID.
    ///
    /// Corresponds to Java: `public void setGUID(UUID guid)`
    pub fn set_guid(&mut self, guid: Option<Uuid>) {
        self.m_guid = guid;
    }

    /// Get the enterprise flag.
    ///
    /// Corresponds to Java: `public boolean getEnterprise()`
    pub fn get_enterprise(&self) -> bool {
        self.m_enterprise
    }

    /// Set the enterprise flag.
    ///
    /// Corresponds to Java: `public void setEnterprise(boolean enterprise)`
    pub fn set_enterprise(&mut self, enterprise: bool) {
        self.m_enterprise = enterprise;
    }

    /// Get the show indent flag.
    ///
    /// Corresponds to Java: `public boolean getShowIndent()`
    pub fn get_show_indent(&self) -> bool {
        self.m_show_indent
    }

    /// Set the show indent flag.
    ///
    /// Corresponds to Java: `public void setShowIndent(boolean showIndent)`
    pub fn set_show_indent(&mut self, show_indent: bool) {
        self.m_show_indent = show_indent;
    }

    /// Get the resource substitution enabled flag.
    ///
    /// Corresponds to Java: `public boolean getResourceSubstitutionEnabled()`
    pub fn get_resource_substitution_enabled(&self) -> bool {
        self.m_resource_substitution_enabled
    }

    /// Set the resource substitution enabled flag.
    ///
    /// Corresponds to Java: `public void setResourceSubstitutionEnabled(boolean resourceSubstitutionEnabled)`
    pub fn set_resource_substitution_enabled(&mut self, resource_substitution_enabled: bool) {
        self.m_resource_substitution_enabled = resource_substitution_enabled;
    }

    /// Get the leaf nodes only flag.
    ///
    /// Corresponds to Java: `public boolean getLeafOnly()`
    pub fn get_leaf_only(&self) -> bool {
        self.m_leaf_only
    }

    /// Set the leaf nodes only flag.
    ///
    /// Corresponds to Java: `public void setLeafOnly(boolean leafOnly)`
    pub fn set_leaf_only(&mut self, leaf_only: bool) {
        self.m_leaf_only = leaf_only;
    }

    /// Get the required at all levels flag.
    ///
    /// Corresponds to Java: `public boolean getAllLevelsRequired()`
    pub fn get_all_levels_required(&self) -> bool {
        self.m_all_levels_required
    }

    /// Set the required at all levels flag.
    ///
    /// Corresponds to Java: `public void setAllLevelsRequired(boolean allLevelsRequired)`
    pub fn set_all_levels_required(&mut self, all_levels_required: bool) {
        self.m_all_levels_required = all_levels_required;
    }

    /// Get the only table values allowed flag.
    ///
    /// Corresponds to Java: `public boolean getOnlyTableValuesAllowed()`
    pub fn get_only_table_values_allowed(&self) -> bool {
        self.m_only_table_values_allowed
    }

    /// Set the only table values allowed flag.
    ///
    /// Corresponds to Java: `public void setOnlyTableValuesAllowed(boolean onlyTableValuesAllowed)`
    pub fn set_only_table_values_allowed(&mut self, only_table_values_allowed: bool) {
        self.m_only_table_values_allowed = only_table_values_allowed;
    }

    /// Append a value item to the lookup table.
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(T e)`
    pub fn add(&mut self, item: CustomFieldValueItem) {
        self.entries.add(item);
    }

    /// Insert a value item at the specified index (returns `false` when out of bounds).
    ///
    /// Corresponds to Java: `ListWithCallbacks.add(int index, T element)`
    pub fn add_at(&mut self, index: usize, item: CustomFieldValueItem) -> bool {
        self.entries.add_at(index, item)
    }

    /// Get a value item by index.
    ///
    /// Corresponds to Java: `ListWithCallbacks.get(int index)`
    pub fn get(&self, index: usize) -> Option<&CustomFieldValueItem> {
        self.entries.get(index)
    }

    /// Replace the value item at the specified index and return the replaced old element (returns `None` when out of bounds).
    ///
    /// Corresponds to Java: `ListWithCallbacks.set(int index, T element)`
    pub fn set(&mut self, index: usize, item: CustomFieldValueItem) -> Option<CustomFieldValueItem> {
        self.entries.set(index, item)
    }

    /// Remove the value item at the specified index and return it (returns `None` when out of bounds).
    ///
    /// Corresponds to Java: `ListWithCallbacks.remove(int index)`
    pub fn remove(&mut self, index: usize) -> Option<CustomFieldValueItem> {
        self.entries.remove(index)
    }

    /// Clear the lookup table (does not fire the `removed` callback, consistent with Java).
    ///
    /// Corresponds to Java: `ListWithCallbacks.clear()`
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Number of value items.
    ///
    /// Corresponds to Java: `ListWithCallbacks.size()`
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the lookup table is empty.
    ///
    /// Corresponds to Java: `ListWithCallbacks.isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over the value items in the lookup table.
    ///
    /// Corresponds to Java: `ListWithCallbacks.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, CustomFieldValueItem> {
        self.entries.iter()
    }

    /// Access the lookup table contents as a slice.
    ///
    /// Corresponds to Java: the list view of `ListWithCallbacks`.
    pub fn as_slice(&self) -> &[CustomFieldValueItem] {
        self.entries.as_slice()
    }

    /// Find a value item by unique ID; returns `None` when not found.
    ///
    /// Convenience query added on the Rust side: Java's `CustomFieldLookupTable` provides only list semantics and has no
    /// lookup by unique ID; MPP reading needs to locate a value item by unique ID, so a linear search is provided here.
    pub fn get_by_unique_id(&self, unique_id: i32) -> Option<&CustomFieldValueItem> {
        self.entries
            .iter()
            .find(|item| item.get_unique_id() == unique_id)
    }
}

impl Default for CustomFieldLookupTable {
    /// Matches Java's implicit default constructor (note that `m_showIndent` and
    /// `m_onlyTableValuesAllowed` are `true`, so `derive(Default)` cannot be used).
    fn default() -> Self {
        Self::new()
    }
}

/// A candidate value for a custom field (the carrier of the 8 Java fields).
///
/// Corresponds to Java: `org.mpxj.mpp.CustomFieldValueItem`
///
/// This type is the **field carrier** of that Java class on the Rust side: [`CustomFieldLookupTable`] and
/// [`crate::custom_field_container::CustomFieldContainer`] both use it as their element type;
/// [`crate::mpp::custom_field_value_item::CustomFieldValueItem`] on the `mpp` side is the same Java class's
/// **shared handle**, internally holding `Rc<RefCell<CustomFieldValueItem>>` (see the "Element type" section in the file header).
///
/// # Differences from Java
///
/// - Java's `Object m_value` → `Option<FieldValue>` (matching the crate's unified representation of Java `Object` field values);
/// - Java's nullable reference fields (`UUID` / `String` / `byte[]` / `Integer` / enums) → `Option<_>`;
/// - Java does not override `equals` / `hashCode` (reference equality); the Rust side derives only `Debug` / `Clone`.
#[derive(Debug, Clone)]
pub struct CustomFieldValueItem {
    /// Value item unique ID.
    ///
    /// Corresponds to Java: `private final Integer m_uniqueID;`
    m_unique_id: i32,
    /// Value item GUID.
    ///
    /// Corresponds to Java: `private UUID m_guid;`
    m_guid: Option<Uuid>,
    /// Value item value.
    ///
    /// Corresponds to Java: `private Object m_value;`
    m_value: Option<FieldValue>,
    /// Value item description.
    ///
    /// Corresponds to Java: `private String m_description;`
    m_description: Option<String>,
    /// Unknown data block.
    ///
    /// Corresponds to Java: `private byte[] m_unknown;`
    m_unknown: Option<Vec<u8>>,
    /// Parent unique ID.
    ///
    /// Corresponds to Java: `private Integer m_parentUniqueID;`
    m_parent_unique_id: Option<i32>,
    /// Value type.
    ///
    /// Corresponds to Java: `private CustomFieldValueDataType m_type;`
    m_type: Option<CustomFieldValueDataType>,
    /// Collapsed flag.
    ///
    /// Corresponds to Java: `private boolean m_collapsed;`
    m_collapsed: bool,
}

impl CustomFieldValueItem {
    /// Constructor.
    ///
    /// Corresponds to Java: `public CustomFieldValueItem(Integer uniqueID)`
    pub fn new(unique_id: i32) -> Self {
        Self {
            m_unique_id: unique_id,
            m_guid: None,
            m_value: None,
            m_description: None,
            m_unknown: None,
            m_parent_unique_id: None,
            m_type: None,
            m_collapsed: false,
        }
    }

    /// Get the unique ID.
    ///
    /// Corresponds to Java: `public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> i32 {
        self.m_unique_id
    }

    /// Set the value.
    ///
    /// Corresponds to Java: `public void setValue(Object value)`
    pub fn set_value(&mut self, value: Option<FieldValue>) {
        self.m_value = value;
    }

    /// Get the value.
    ///
    /// Corresponds to Java: `public Object getValue()`
    pub fn get_value(&self) -> Option<&FieldValue> {
        self.m_value.as_ref()
    }

    /// Set the description.
    ///
    /// Corresponds to Java: `public void setDescription(String description)`
    pub fn set_description(&mut self, description: Option<String>) {
        self.m_description = description;
    }

    /// Get the description.
    ///
    /// Corresponds to Java: `public String getDescription()`
    pub fn get_description(&self) -> Option<&str> {
        self.m_description.as_deref()
    }

    /// Set the unknown data block.
    ///
    /// Corresponds to Java: `public void setUnknown(byte[] unknown)`
    pub fn set_unknown(&mut self, unknown: Option<Vec<u8>>) {
        self.m_unknown = unknown;
    }

    /// Get the unknown data block.
    ///
    /// Corresponds to Java: `public byte[] getUnknown()`
    pub fn get_unknown(&self) -> Option<&[u8]> {
        self.m_unknown.as_deref()
    }

    /// Set the parent unique ID.
    ///
    /// Corresponds to Java: `public void setParentUniqueID(Integer id)`
    pub fn set_parent_unique_id(&mut self, id: Option<i32>) {
        self.m_parent_unique_id = id;
    }

    /// Get the parent unique ID.
    ///
    /// Corresponds to Java: `public Integer getParentUniqueID()`
    pub fn get_parent_unique_id(&self) -> Option<i32> {
        self.m_parent_unique_id
    }

    /// Get the GUID.
    ///
    /// Corresponds to Java: `public UUID getGUID()`
    pub fn get_guid(&self) -> Option<Uuid> {
        self.m_guid
    }

    /// Set the GUID.
    ///
    /// Corresponds to Java: `public void setGUID(UUID guid)`
    pub fn set_guid(&mut self, guid: Option<Uuid>) {
        self.m_guid = guid;
    }

    /// Get the value type.
    ///
    /// Corresponds to Java: `public CustomFieldValueDataType getType()`
    pub fn get_type(&self) -> Option<CustomFieldValueDataType> {
        self.m_type
    }

    /// Set the value type.
    ///
    /// Corresponds to Java: `public void setType(CustomFieldValueDataType type)`
    pub fn set_type(&mut self, value_type: Option<CustomFieldValueDataType>) {
        self.m_type = value_type;
    }

    /// Get the collapsed flag.
    ///
    /// Corresponds to Java: `public boolean getCollapsed()`
    pub fn get_collapsed(&self) -> bool {
        self.m_collapsed
    }

    /// Set the collapsed flag.
    ///
    /// Corresponds to Java: `public void setCollapsed(boolean collapsed)`
    pub fn set_collapsed(&mut self, collapsed: bool) {
        self.m_collapsed = collapsed;
    }
}

impl Display for CustomFieldValueItem {
    /// Corresponds to Java: `@Override public String toString()`
    ///
    /// Java:
    /// `String.format("[CustomFieldValueItem uniqueID=%d guid=%s parentId=%d value=%s description=%s]", ...)`
    ///
    /// `null` fields are output as `null` following Java's `%s` rule (on the Java side `%d` throws an exception on `null`;
    /// the Rust side always outputs `null`).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[CustomFieldValueItem uniqueID={} guid={} parentId={} value={} description={}]",
            self.m_unique_id,
            self.m_guid
                .map(|guid| guid.to_string())
                .unwrap_or_else(|| "null".to_owned()),
            self.m_parent_unique_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "null".to_owned()),
            self.m_value
                .as_ref()
                .map(|value| format!("{value:?}"))
                .unwrap_or_else(|| "null".to_owned()),
            self.m_description.as_deref().unwrap_or("null")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_flags_match_java_initial_values() {
        let table = CustomFieldLookupTable::new();

        assert!(table.get_show_indent());
        assert!(table.get_only_table_values_allowed());
        assert!(!table.get_enterprise());
        assert!(!table.get_resource_substitution_enabled());
        assert!(!table.get_leaf_only());
        assert!(!table.get_all_levels_required());
        assert!(table.get_guid().is_none());
    }

    #[test]
    fn flags_are_settable() {
        let mut table = CustomFieldLookupTable::new();
        let guid = Uuid::from_u128(1);

        table.set_guid(Some(guid));
        table.set_enterprise(true);
        table.set_show_indent(false);
        table.set_resource_substitution_enabled(true);
        table.set_leaf_only(true);
        table.set_all_levels_required(true);
        table.set_only_table_values_allowed(false);

        assert_eq!(table.get_guid(), Some(guid));
        assert!(table.get_enterprise());
        assert!(!table.get_show_indent());
        assert!(table.get_resource_substitution_enabled());
        assert!(table.get_leaf_only());
        assert!(table.get_all_levels_required());
        assert!(!table.get_only_table_values_allowed());
    }

    #[test]
    fn list_operations_and_lookup_by_unique_id() {
        let mut table = CustomFieldLookupTable::new();
        assert!(table.is_empty());

        let mut first = CustomFieldValueItem::new(10);
        first.set_description(Some("A".to_owned()));
        first.set_value(Some(FieldValue::String("A".to_owned())));
        let second = CustomFieldValueItem::new(20);

        table.add(first);
        table.add_at(1, second);

        assert_eq!(table.len(), 2);
        assert_eq!(table.iter().count(), 2);
        assert_eq!(table.as_slice().len(), 2);
        assert_eq!(
            table.get_by_unique_id(10).and_then(|item| item.get_description()),
            Some("A")
        );
        assert!(table.get_by_unique_id(30).is_none());

        // add_at out of bounds
        assert!(!table.add_at(5, CustomFieldValueItem::new(30)));
        assert_eq!(table.len(), 2);

        // set / remove
        let replaced = table.set(0, CustomFieldValueItem::new(30)).expect("index exists");
        assert_eq!(replaced.get_unique_id(), 10);
        assert!(table.get_by_unique_id(30).is_some());

        let removed = table.remove(0).expect("index exists");
        assert_eq!(removed.get_unique_id(), 30);
        assert!(table.get_by_unique_id(30).is_none());

        // Out of bounds
        assert!(table.set(5, CustomFieldValueItem::new(40)).is_none());
        assert!(table.remove(5).is_none());

        table.clear();
        assert!(table.is_empty());
    }

    #[test]
    fn lookup_table_callbacks_can_be_registered() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let log = Rc::new(RefCell::new(Vec::new()));
        let mut table = CustomFieldLookupTable::new();

        let added = Rc::clone(&log);
        table
            .entries
            .set_added_callback(Some(Box::new(move |item: &CustomFieldValueItem| {
                added.borrow_mut().push(format!("added:{}", item.get_unique_id()));
            })));

        table.add(CustomFieldValueItem::new(7));

        assert_eq!(*log.borrow(), vec!["added:7".to_owned()]);
    }

    #[test]
    fn value_item_accessors_round_trip() {
        let mut item = CustomFieldValueItem::new(4);
        let guid = Uuid::from_u128(42);

        item.set_guid(Some(guid));
        item.set_value(Some(FieldValue::Integer(1)));
        item.set_description(Some("desc".to_owned()));
        item.set_unknown(Some(vec![1, 2, 3]));
        item.set_parent_unique_id(Some(2));
        item.set_type(Some(CustomFieldValueDataType::Number));
        item.set_collapsed(true);

        assert_eq!(item.get_unique_id(), 4);
        assert_eq!(item.get_guid(), Some(guid));
        assert_eq!(item.get_value(), Some(&FieldValue::Integer(1)));
        assert_eq!(item.get_description(), Some("desc"));
        assert_eq!(item.get_unknown(), Some(&[1, 2, 3][..]));
        assert_eq!(item.get_parent_unique_id(), Some(2));
        assert_eq!(item.get_type(), Some(CustomFieldValueDataType::Number));
        assert!(item.get_collapsed());
    }

    #[test]
    fn value_item_display_matches_java_format() {
        let mut item = CustomFieldValueItem::new(4);
        item.set_description(Some("desc".to_owned()));
        item.set_parent_unique_id(Some(2));

        assert_eq!(
            item.to_string(),
            "[CustomFieldValueItem uniqueID=4 guid=null parentId=2 value=null description=desc]"
        );
    }

    /// Deduplication regression: the shared handle on the `mpp` side and the local carrier read and write the **same fields** (there is no duplicate state).
    #[test]
    fn stored_item_and_mpp_handle_share_the_same_fields() {
        use crate::mpp::custom_field_value_item::CustomFieldValueItem as Handle;

        let handle = Handle::new(5);
        handle.set_description(Some("shared".to_owned()));
        handle.set_value(Some(FieldValue::Integer(9)));

        // Handle → carrier: values agree
        let stored = handle.to_stored_item();
        assert_eq!(stored.get_unique_id(), 5);
        assert_eq!(stored.get_description(), Some("shared"));
        assert_eq!(stored.get_value(), Some(&FieldValue::Integer(9)));

        // Carrier → handle: after modification through the shared handle, the handle side sees the same data
        let stored = handle.stored_item();
        stored
            .borrow_mut()
            .set_description(Some("updated".to_owned()));
        assert_eq!(handle.get_description(), Some("updated".to_owned()));
    }
}
