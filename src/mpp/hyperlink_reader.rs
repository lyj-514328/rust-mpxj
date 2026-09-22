//! Corresponds to Java: `org/mpxj/mpp/HyperlinkReader.java`
//!
//! Read the hyperlink data block common to all versions in an MPP file and write it into Task / Resource / ResourceAssignment
//! corresponding fields (hyperlink, address, subaddress, screen tip).
//!
//! Data block layout (identical to Java's implementation; all strings are UTF-16LE, NUL-terminated, each preceded by 12 bytes of
//! unknown data):
//!
//! ```text
//! offset 0  : 12 bytes of unknown data
//! offset 12 : 12 bytes of unknown data
//! offset 24 : hyperlink text (NUL-terminated)
//! +12 bytes of unknown data
//! : address (NUL-terminated)
//! +12 bytes of unknown data
//! : subaddress (NUL-terminated)
//! +12 bytes of unknown data
//! : screen tip (NUL-terminated; Project 98 has no such section)
//! ```
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `private static final Map<Class<?>, FieldType[]> FIELDS` + `FIELDS.get(container.getClass())` | Add a `FieldTypeClass` parameter, and [`fields`] looks up by it | Java uses reflection to derive the field family from the container class (`Task.class` / `Resource.class` / `ResourceAssignment.class`); Rust has no reflection, so the field family is given explicitly by the caller. When no lookup succeeds, Java throws `NullPointerException` from `fields[0]`, whereas the Rust side writes nothing (no panic) |
//! | `container.set(field, String)` | `container.set(field, Some(FieldValue::String(..)))` | When the screen tip is `null`, Java clears the field; the Rust side's `None` is equivalent |
//! | `read(FieldContainer container, byte[] data)` (`data` may be `null`) | `read(field_type_class, container, data: &[u8])` | Per project convention the `byte[]` argument becomes `&[u8]`; the check "`data == null` means do nothing" is handled by the caller when obtaining `Option<&[u8]>` |
//! | Offset out of bounds | Does not panic | Java relies on index arithmetic inside `MPPUtility.getUnicodeString` (which throws when out of bounds); the Rust method returns an empty string for out-of-bounds input |
//!
//! Skipped Java members: none.
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms
//! (the only reflective field-table lookup has been changed to an explicit parameter as per the table above).

use crate::assignment_field::AssignmentField;
use crate::field_container::FieldContainer;
use crate::field_key::FieldKey;
use crate::field_type_class::FieldTypeClass;
use crate::field_value::FieldValue;
use crate::mpp::mpp_utility::MPPUtility;
use crate::resource_field::ResourceField;
use crate::task_field::TaskField;

/// Number of unknown bytes skipped before each string.
///
/// Corresponds to Java: `offset += 12` (4 occurrences).
const SKIPPED_BYTES: usize = 12;

/// Get the 4 hyperlink fields for a field family (order: hyperlink, address, subaddress, screen tip).
///
/// Corresponds to Java: `FIELDS.get(container.getClass())` (Java looks up by class object; the Rust side uses the field family).
fn fields(field_type_class: FieldTypeClass) -> Option<[FieldKey; 4]> {
    match field_type_class {
        FieldTypeClass::Task => Some([
            FieldKey::Task(TaskField::Hyperlink),
            FieldKey::Task(TaskField::HyperlinkAddress),
            FieldKey::Task(TaskField::HyperlinkSubaddress),
            FieldKey::Task(TaskField::HyperlinkScreenTip),
        ]),
        FieldTypeClass::Resource => Some([
            FieldKey::Resource(ResourceField::Hyperlink),
            FieldKey::Resource(ResourceField::HyperlinkAddress),
            FieldKey::Resource(ResourceField::HyperlinkSubaddress),
            FieldKey::Resource(ResourceField::HyperlinkScreenTip),
        ]),
        FieldTypeClass::Assignment => Some([
            FieldKey::Assignment(AssignmentField::Hyperlink),
            FieldKey::Assignment(AssignmentField::HyperlinkAddress),
            FieldKey::Assignment(AssignmentField::HyperlinkSubaddress),
            FieldKey::Assignment(AssignmentField::HyperlinkScreenTip),
        ]),
// Java: other types are not in FIELDS -> getClass() lookup returns null -> fields[0] throws an NPE
        _ => None,
    }
}

/// Read the hyperlink data common to all versions.
///
/// Corresponds to Java: `final class HyperlinkReader`
pub struct HyperlinkReader {
/// Whether the data block contains a screen tip.
    ///
    /// Corresponds to Java: `private boolean m_hasScreenTip = true`
    has_screen_tip: bool,
}

impl HyperlinkReader {
/// Constructor (screen tips are read by default, matching Java's initial field value).
    ///
/// Corresponds to Java: the implicit default constructor
    pub fn new() -> Self {
        Self {
            has_screen_tip: true,
        }
    }

/// Set whether to read the screen tip (only Project 98 lacks this section).
    ///
    /// Corresponds to Java: `public void setHasScreenTip(boolean hasScreenTip)`
    pub fn set_has_screen_tip(&mut self, has_screen_tip: bool) {
        self.has_screen_tip = has_screen_tip;
    }

/// Read the hyperlink data and write it into the given container's fields.
    ///
    /// Corresponds to Java: `public void read(FieldContainer container, byte[] data)`
    ///
/// `field_type_class` corresponds to Java's `container.getClass()` (see the differences table at the top of the file).
    pub fn read(
        &self,
        field_type_class: FieldTypeClass,
        container: &mut dyn FieldContainer,
        data: &[u8],
    ) {
        // Java: if (data != null)
        let Some(fields) = fields(field_type_class) else {
            return;
        };

// Java: int offset = 12; offset += 12; (a 24-byte prefix in total)
        let mut offset = SKIPPED_BYTES + SKIPPED_BYTES;

        let hyperlink = MPPUtility::get_unicode_string(data, offset);
        // Java: offset += ((hyperlink.length() + 1) * 2);
        offset += string_size(&hyperlink);

        offset += SKIPPED_BYTES;
        let address = MPPUtility::get_unicode_string(data, offset);
        offset += string_size(&address);

        offset += SKIPPED_BYTES;
        let subaddress = MPPUtility::get_unicode_string(data, offset);
        offset += string_size(&subaddress);

        offset += SKIPPED_BYTES;
        let screentip = if self.has_screen_tip {
            Some(MPPUtility::get_unicode_string(data, offset))
        } else {
            None
        };

        // Java: container.set(fields[0], hyperlink); ... container.set(fields[3], screentip);
        container.set(fields[0], Some(FieldValue::String(hyperlink)));
        container.set(fields[1], Some(FieldValue::String(address)));
        container.set(fields[2], Some(FieldValue::String(subaddress)));
        container.set(
            fields[3],
            screentip.map(FieldValue::String),
        );
    }
}

impl Default for HyperlinkReader {
/// Corresponds to Java: the implicit default constructor.
    fn default() -> Self {
        Self::new()
    }
}

/// Number of bytes a string occupies in a UTF-16LE data block (including the trailing NUL).
///
/// Corresponds to Java: `(value.length() + 1) * 2`; Java's `length()` is the number of UTF-16 code units,
/// which matches Rust's `encode_utf16().count()`.
fn string_size(value: &str) -> usize {
    (value.encode_utf16().count() + 1) * 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::listener::field_listener::FieldListener;
    use std::collections::HashMap;

/// Field container for tests (consistent with the test container in `crate::mpp::mpp_bit_flag`).
    #[derive(Default)]
    struct MockContainer {
        values: HashMap<FieldKey, FieldValue>,
    }

    impl FieldContainer for MockContainer {
        fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
            match value {
                Some(value) => self.values.insert(field, value),
                None => self.values.remove(&field),
            };
        }

        fn get(&self, field: FieldKey) -> Option<FieldValue> {
            self.values.get(&field).cloned()
        }

        fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
            self.values.get(&field)
        }

        fn add_field_listener(&mut self, _listener: Box<dyn FieldListener>) {}

        fn clear_field_listeners(&mut self) {}
    }

/// Append a string to the buffer as UTF-16LE with a trailing NUL.
    fn push_unicode(buffer: &mut Vec<u8>, value: &str) {
        for unit in value.encode_utf16() {
            buffer.extend_from_slice(&unit.to_le_bytes());
        }
        buffer.extend_from_slice(&[0x00, 0x00]);
    }

/// Manually build a hyperlink data block: a 24-byte prefix at the start, then each string followed by 12 bytes of unknown data.
    fn hyperlink_data(has_screen_tip: bool) -> Vec<u8> {
        let mut data = vec![0u8; 24];
        push_unicode(&mut data, "https://example.com");
        data.extend_from_slice(&[0xBB; 12]);
        push_unicode(&mut data, "https://example.com/address");
        data.extend_from_slice(&[0xCC; 12]);
        push_unicode(&mut data, "subaddress");
        if has_screen_tip {
            data.extend_from_slice(&[0xDD; 12]);
            push_unicode(&mut data, "screen tip");
        }
        data
    }

/// Task container: reads all 4 fields.
    #[test]
    fn reads_all_task_hyperlink_fields() {
        let mut container = MockContainer::default();
        HyperlinkReader::new().read(FieldTypeClass::Task, &mut container, &hyperlink_data(true));

        assert_eq!(
            container.get(FieldKey::Task(TaskField::Hyperlink)),
            Some(FieldValue::String("https://example.com".to_owned()))
        );
        assert_eq!(
            container.get(FieldKey::Task(TaskField::HyperlinkAddress)),
            Some(FieldValue::String("https://example.com/address".to_owned()))
        );
        assert_eq!(
            container.get(FieldKey::Task(TaskField::HyperlinkSubaddress)),
            Some(FieldValue::String("subaddress".to_owned()))
        );
        assert_eq!(
            container.get(FieldKey::Task(TaskField::HyperlinkScreenTip)),
            Some(FieldValue::String("screen tip".to_owned()))
        );
    }

/// The resource and assignment containers use their own field families.
    #[test]
    fn reads_resource_and_assignment_fields() {
        let mut resource = MockContainer::default();
        HyperlinkReader::new().read(
            FieldTypeClass::Resource,
            &mut resource,
            &hyperlink_data(true),
        );
        assert_eq!(
            resource.get(FieldKey::Resource(ResourceField::Hyperlink)),
            Some(FieldValue::String("https://example.com".to_owned()))
        );
        assert_eq!(
            resource.get(FieldKey::Resource(ResourceField::HyperlinkScreenTip)),
            Some(FieldValue::String("screen tip".to_owned()))
        );

        let mut assignment = MockContainer::default();
        HyperlinkReader::new().read(
            FieldTypeClass::Assignment,
            &mut assignment,
            &hyperlink_data(true),
        );
        assert_eq!(
            assignment.get(FieldKey::Assignment(AssignmentField::HyperlinkAddress)),
            Some(FieldValue::String(
                "https://example.com/address".to_owned()
            ))
        );
    }

/// Project 98 has no screen tip: the caller must call `setHasScreenTip(false)` first for the field to be cleared.
    ///
/// (If it still processes as the default "read screen tip", the Project 98 data block reads an empty string at that offset,
/// thereby setting the field to an empty string — this is consistent with Java's behavior.)
    #[test]
    fn project_98_has_no_screen_tip() {
        let mut container = MockContainer::default();
        container.set(
            FieldKey::Task(TaskField::HyperlinkScreenTip),
            Some(FieldValue::String("stale".to_owned())),
        );

        let mut reader = HyperlinkReader::new();
        reader.set_has_screen_tip(false);
        reader.read(FieldTypeClass::Task, &mut container, &hyperlink_data(false));

// Java: container.set(fields[3], null) -> clears the field
        assert_eq!(
            container.get(FieldKey::Task(TaskField::HyperlinkScreenTip)),
            None
        );
        assert_eq!(
            container.get(FieldKey::Task(TaskField::Hyperlink)),
            Some(FieldValue::String("https://example.com".to_owned()))
        );

// With the default (read screen tip), an empty string is read and the field is set to an empty string
        let mut container = MockContainer::default();
        HyperlinkReader::new().read(FieldTypeClass::Task, &mut container, &hyperlink_data(false));
        assert_eq!(
            container.get(FieldKey::Task(TaskField::HyperlinkScreenTip)),
            Some(FieldValue::String(String::new()))
        );
    }

/// With `setHasScreenTip(false)`, the screen tip is not read even if present in the data.
    #[test]
    fn set_has_screen_tip_false_clears_screen_tip() {
        let mut reader = HyperlinkReader::new();
        reader.set_has_screen_tip(false);

        let mut container = MockContainer::default();
        reader.read(FieldTypeClass::Task, &mut container, &hyperlink_data(true));

        assert_eq!(
            container.get(FieldKey::Task(TaskField::HyperlinkScreenTip)),
            None
        );
        assert_eq!(
            container.get(FieldKey::Task(TaskField::HyperlinkSubaddress)),
            Some(FieldValue::String("subaddress".to_owned()))
        );
    }

/// Neither insufficient data nor a field-family mismatch panics.
    #[test]
    fn short_data_and_unknown_field_class_do_not_panic() {
        let mut container = MockContainer::default();

// Empty data / very short data
        HyperlinkReader::new().read(FieldTypeClass::Task, &mut container, &[]);
        HyperlinkReader::new().read(FieldTypeClass::Task, &mut container, &[0u8; 5]);

// With empty data, every field is written as an empty string (Java's out-of-bounds exception does not occur)
        assert_eq!(
            container.get(FieldKey::Task(TaskField::Hyperlink)),
            Some(FieldValue::String(String::new()))
        );

// Java: there is no Project container in FIELDS -> fields is null -> NPE; the Rust side writes nothing
        let mut project = MockContainer::default();
        HyperlinkReader::new().read(
            FieldTypeClass::Project,
            &mut project,
            &hyperlink_data(true),
        );
        assert!(project.values.is_empty());
    }

/// Offset advancement remains correct for Chinese (multi-byte UTF-16) strings.
    #[test]
    fn handles_multi_byte_strings() {
        let mut data = vec![0u8; 24];
        push_unicode(&mut data, "中文链接");
        data.extend_from_slice(&[0x00; 12]);
        push_unicode(&mut data, "地址");
        data.extend_from_slice(&[0x00; 12]);
        push_unicode(&mut data, "子地址");
        data.extend_from_slice(&[0x00; 12]);
        push_unicode(&mut data, "提示");

        let mut container = MockContainer::default();
        HyperlinkReader::new().read(FieldTypeClass::Task, &mut container, &data);

        assert_eq!(
            container.get(FieldKey::Task(TaskField::Hyperlink)),
            Some(FieldValue::String("中文链接".to_owned()))
        );
        assert_eq!(
            container.get(FieldKey::Task(TaskField::HyperlinkAddress)),
            Some(FieldValue::String("地址".to_owned()))
        );
        assert_eq!(
            container.get(FieldKey::Task(TaskField::HyperlinkSubaddress)),
            Some(FieldValue::String("子地址".to_owned()))
        );
        assert_eq!(
            container.get(FieldKey::Task(TaskField::HyperlinkScreenTip)),
            Some(FieldValue::String("提示".to_owned()))
        );
    }
}
