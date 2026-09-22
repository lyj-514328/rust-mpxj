//! Corresponds to Java: `org/mpxj/mpp/CustomFieldReader12.java`
//!
//! Reads the field aliases of MPP12 (the `CUSTOM_FIELDS` block in `Props`): the block contains, in order,
//! "alias block → two unknown blocks → field definition block"; aliases are written together with field IDs into
//! [`CustomFieldContainer`](`CustomField::set_alias`).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `CustomFieldReader12(ProjectFile file, byte[] data)` | `new(&Rc<RefCell<ProjectFile>>, Option<&[u8]>)` | `ProjectFile` is shared as `Rc<RefCell<..>>`; Java's `byte[]` may be `null` (the first line of `process` checks for null), so the Rust side uses `Option<&[u8]>` (the "`null` → `Option<T>`" convention) and copies it into `Option<Vec<u8>>` at construction (Java stores the caller's array reference, while the Rust parameter is a borrow, so the data must be owned) |
//! | `CustomFieldContainer m_fields = file.getCustomFields()` | `Rc<RefCell<CustomFieldContainer>>` | same as Java |
//! | `Map<Integer, String> aliasMap` | `HashMap<i32, String>` | same as Java (field IDs that are not matched are handled by `FieldTypeHelper`, see below) |
//! | `ByteArrayHelper.getInt/getShort` out of bounds throws `ArrayIndexOutOfBoundsException` | [`checked_int`] / [`checked_short`] return `None` and abort parsing | follows the "do not panic" convention, annotated at each point |
//! | `offset += size` (`size` is an `int`; Java allows a negative value to move the offset backwards) | `advance_offset` returns `Option<usize>` and aborts parsing when the offset moves back past zero | Java likewise throws `ArrayIndexOutOfBoundsException` on the subsequent out-of-bounds read |
//! | `EnterpriseCustomFieldDataType.getDataTypeFromID(int)` | [`EnterpriseCustomFieldDataType::get_data_type_from_id`](crate::mpp::enterprise_custom_field_data_type::EnterpriseCustomFieldDataType::get_data_type_from_id) | the same as Java, delegates to that utility class (`crate::mpp::enterprise_custom_field_data_type` is already ported) |
//! | `m_fields.getOrCreate(FieldTypeHelper.getInstance(m_file, k)).setAlias(v)` | one by one `CustomFieldContainer::get_or_create(&container, key)` + `set_alias(Some(alias))` | When `FieldTypeHelper` does not recognize a field ID, Java passes `null` to `getOrCreate` (`HashMap` allows a null key, but `CustomField` then throws an NPE at `field.getFieldTypeClass()`); Rust's `FieldKey` has no "unknown" variant, so that field ID is skipped |
//! | `aliasMap.forEach(...)` | `for (field_id, alias) in alias_map` | the iteration order is not guaranteed, just like Java's `HashMap` (their internal orders differ) |
//!
//! Skipped Java methods: none (`process()` is translated line by line; the two commented-out debug statements keep their comments as-is).
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.
//! The `FieldTypeHelper` calls appearing in comments are reserved for future extension (the Java source file likewise only has comments).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::common::field_type_helper::FieldTypeHelper;
use crate::custom_field_container::CustomFieldContainer;
use crate::mpp::custom_field_value_reader::{checked_int, checked_short};
use crate::mpp::enterprise_custom_field_data_type::EnterpriseCustomFieldDataType;
use crate::mpp::mpp_utility::MPPUtility;
use crate::project_file::ProjectFile;

/// Reads the field aliases of MPP12.
///
/// Corresponds to Java: `class CustomFieldReader12`
pub struct CustomFieldReader12 {
/// The project file.
    ///
    /// Corresponds to Java field: `private final ProjectFile m_file;`
    file: Rc<RefCell<ProjectFile>>,

/// The custom field container.
    ///
    /// Corresponds to Java field: `private final CustomFieldContainer m_fields;`
    fields: Rc<RefCell<CustomFieldContainer>>,

/// The `CUSTOM_FIELDS` data block in `Props`.
    ///
    /// Corresponds to Java field: `private final byte[] m_data;`
    data: Option<Vec<u8>>,
}

impl CustomFieldReader12 {
/// Constructor.
    ///
    /// Corresponds to Java: `public CustomFieldReader12(ProjectFile file, byte[] data)`
    pub fn new(file: &Rc<RefCell<ProjectFile>>, data: Option<&[u8]>) -> Self {
        // Java: m_fields = file.getCustomFields();
// When the file is already mutably borrowed (which never happens on the normal call path), degrade gracefully to a standalone container to avoid a panic.
        let fields = match file.try_borrow() {
            Ok(file) => file.get_custom_fields(),
            Err(_) => CustomFieldContainer::new(),
        };

        Self {
            file: Rc::clone(file),
            fields,
            data: data.map(<[u8]>::to_vec),
        }
    }

/// Processes the field aliases.
    ///
    /// Corresponds to Java: `public void process()`
    pub fn process(&self) {
        // Java: if (m_data == null) { return; }
        let Some(data) = self.data.as_deref() else {
            return;
        };

        // Java: Map<Integer, String> aliasMap = new HashMap<>();
        let mut alias_map: HashMap<i32, String> = HashMap::new();

        // Java: int index = 0; int offset = 0;
        let mut index = 0i32;
        let mut offset = 0usize;

        // Java: int aliasBlockSize = ByteArrayHelper.getInt(m_data, offset); offset += 8;
// (Java comment: the "length" appears twice, so skip 8 bytes)
        let Some(alias_block_size) = checked_int(data, offset) else {
            return;
        };
        offset += 8;

        // Java: int numberOfAliases = ByteArrayHelper.getInt(m_data, offset); offset += 4;
        let Some(number_of_aliases) = checked_int(data, offset) else {
            return;
        };
        offset += 4;

// Java uses `aliasBlockSize` as the loop bound: when negative, the loop body does not execute; the Rust side does the same.
        let alias_block_limit = usize::try_from(alias_block_size).unwrap_or(0);

        // Java: while (index < numberOfAliases && offset < aliasBlockSize)
        while index < number_of_aliases && offset < alias_block_limit {
            // Java: int fieldID = ByteArrayHelper.getInt(m_data, offset); offset += 4;
            let Some(field_id) = checked_int(data, offset) else {
                break;
            };
            offset += 4;

            // Java: int aliasOffset = ByteArrayHelper.getInt(m_data, offset) + 4; offset += 4;
// (Java comment: the alias offset needs +4 "for some reason")
            let Some(alias_offset) = checked_int(data, offset).map(|value| i64::from(value) + 4)
            else {
                break;
            };
            offset += 4;

            // Java: if (aliasOffset < m_data.length) { String alias = MPPUtility.getUnicodeString(m_data, aliasOffset); if (!alias.isEmpty()) { aliasMap.put(fieldID, alias); } }
            if let Ok(alias_offset) = usize::try_from(alias_offset) {
                if alias_offset < data.len() {
                    let alias = MPPUtility::get_unicode_string(data, alias_offset);
                    if !alias.is_empty() {
                        alias_map.insert(field_id, alias);
                    }
                }
            }

            // Java: index++;
            index += 1;
        }

// Java comment (kept verbatim):
        // NOTE: the blocks here all follow the same format.
        // 4 byte block size (excluding these 4 bytes)
        // 4 byte block size again
        // 4 byte item count
        // N bytes of data (if item count is non-zero)
        // The first block contains alias details
        // The last block may be enterprise custom field details (as per MPP14)
        // Not sure about the other blocks.

        // Java: offset = 4 + aliasBlockSize;
        let Some(mut offset) = advance_offset(4, alias_block_size) else {
            return;
        };

        // Java: int unknownBlock1Size = ByteArrayHelper.getInt(m_data, offset); offset += 4; offset += unknownBlock1Size;
        let Some(unknown_block1_size) = checked_int(data, offset) else {
            return;
        };
        let Some(next) = advance_offset(offset + 4, unknown_block1_size) else {
            return;
        };
        offset = next;

        // Java: int unknownBlock2Size = ByteArrayHelper.getInt(m_data, offset); offset += 4; offset += unknownBlock2Size;
        let Some(unknown_block2_size) = checked_int(data, offset) else {
            return;
        };
        let Some(next) = advance_offset(offset + 4, unknown_block2_size) else {
            return;
        };
        offset = next;

// Java comment: the field definition block; the length appears twice, so skip 8 bytes
        // Java: offset += 8;
        offset = offset.saturating_add(8);

        // Java: int numberOfDefinitions = ByteArrayHelper.getInt(m_data, offset); offset += 4;
        let Some(number_of_definitions) = checked_int(data, offset) else {
            return;
        };
        offset += 4;

        // Java: for (...) { // FieldType fieldType = ...; // CustomField customField = ...;
        //                    // System.out.println(...); offset += 8; }
        for _ in 0..number_of_definitions {
            offset = offset.saturating_add(8);
        }

// Java comment (kept verbatim): //System.out.println(ByteArrayHelper.hexdump(m_data, offset, m_data.length-offset, false));
        for _ in 0..number_of_definitions {
            // Java: if (offset + 2 > m_data.length) { break; }
            if offset.saturating_add(2) > data.len() {
                break;
            }

            // Java: int blockSize = ByteArrayHelper.getShort(m_data, offset);
            let Some(block_size) = checked_short(data, offset) else {
                break;
            };

            // Java: if (offset + blockSize > m_data.length) { break; }
            if offset.saturating_add(usize::try_from(block_size).unwrap_or(0)) > data.len() {
                break;
            }

            // Java: DataType customFieldDataType = EnterpriseCustomFieldDataType.getDataTypeFromID(ByteArrayHelper.getShort(m_data, offset + 12));
// Java throws when out of bounds; the Rust side treats an unavailable value as 0 (`getDataTypeFromID(0)` = CURRENCY)
            let custom_field_data_type = EnterpriseCustomFieldDataType::get_data_type_from_id(
                checked_short(data, offset + 12).unwrap_or(0),
            );

            // Java: FieldTypeHelper.getInstance(m_file, ByteArrayHelper.getInt(m_data, offset + 4), customFieldDataType);
// (Java only uses it for its side effect: ensuring the enterprise custom field is created; the return value is discarded)
            if let Some(field_id) = checked_int(data, offset + 4) {
                let _ = FieldTypeHelper::get_instance_with_data_type(
                    &self.file,
                    field_id,
                    custom_field_data_type,
                );
            }

            // Java: offset += blockSize;
            offset = offset.saturating_add(usize::try_from(block_size).unwrap_or(0));
        }

        // Java: aliasMap.forEach((k, v) -> m_fields.getOrCreate(FieldTypeHelper.getInstance(m_file, k.intValue())).setAlias(v));
        for (field_id, alias) in alias_map {
// Java passes `null` to `getOrCreate` (then throws an NPE); the Rust side skips unrecognized field IDs
            let Some(field) = FieldTypeHelper::get_instance(&self.file, field_id) else {
                continue;
            };

            let custom_field = CustomFieldContainer::get_or_create(&self.fields, field.to_field_key());
            custom_field.borrow_mut().set_alias(Some(alias));
        }
    }
}

/// Advances the offset like Java's `offset += size`.
///
/// Corresponds to Java: `offset = 4 + aliasBlockSize` / `offset += unknownBlock1Size`, etc.
/// Returns `None` when the result is negative (Java would throw `ArrayIndexOutOfBoundsException` on the subsequent out-of-bounds read,
/// while the Rust side aborts parsing).
fn advance_offset(offset: usize, size: i32) -> Option<usize> {
    let result = i64::try_from(offset).ok()? + i64::from(size);
    usize::try_from(result).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::mpp_task_field::MPPTaskField;
    use crate::field_key::FieldKey;
    use crate::field_type_class::FieldTypeClass;
    use crate::task_field::TaskField;

/// UTF-16LE (NUL-terminated) bytes.
    fn utf16(text: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        bytes.extend_from_slice(&[0x00, 0x00]);
        bytes
    }

/// Assembles the complete `CUSTOM_FIELDS` data block.
    ///
/// The layout corresponds one-to-one to Java's parse order:
    /// ```text
/// [0..4)        alias block size (block length counted from offset 4; Java uses offset = 4 + aliasBlockSize)
/// [4..8)        alias block size (repeated once)
/// [8..12)       number of aliases
/// [12..12+8n)   each alias: field ID + alias offset (Java adds 4 during parsing to get the absolute offset)
/// after that    alias text (UTF-16LE, NUL-terminated)
/// after that    unknown block 1: 4-byte size + content
/// after that    unknown block 2: 4-byte size + content
/// after that    field definition block: size + size + number of definitions + 8 bytes each
/// after that    definition record: the first 2 bytes are blockSize, +4 field ID, +12 type ID
    /// ```
    fn custom_fields_block(aliases: &[(i32, &str)], definitions: &[(i32, i32)]) -> Vec<u8> {
        let count = aliases.len();

// The alias text follows the alias entries; record its absolute offset
        let entries_end = 12 + 8 * count;
        let mut texts = Vec::new();
        let mut text_offsets = Vec::new();
        for (_, text) in aliases {
            text_offsets.push(entries_end + texts.len());
            texts.extend_from_slice(&utf16(text));
        }
        let alias_block_size = (entries_end + texts.len() - 4) as i32;

        let mut data = Vec::new();
        data.extend_from_slice(&alias_block_size.to_le_bytes());
        data.extend_from_slice(&alias_block_size.to_le_bytes());
        data.extend_from_slice(&(count as i32).to_le_bytes());

        for (index, (field_id, _)) in aliases.iter().enumerate() {
            data.extend_from_slice(&field_id.to_le_bytes());
            // Java: int aliasOffset = ByteArrayHelper.getInt(m_data, offset) + 4;
            data.extend_from_slice(&((text_offsets[index] as i32) - 4).to_le_bytes());
        }
        data.extend_from_slice(&texts);

// Unknown block 1: 4-byte size + content
        data.extend_from_slice(&4i32.to_le_bytes());
        data.extend_from_slice(&[0u8; 4]);
// Unknown block 2: 4-byte size (0) + no content
        data.extend_from_slice(&0i32.to_le_bytes());

// Field definition block: the length appears twice, followed by the count, then 8 bytes per entry
        data.extend_from_slice(&(definitions.len() as i32 * 8 + 4).to_le_bytes());
        data.extend_from_slice(&(definitions.len() as i32 * 8 + 4).to_le_bytes());
        data.extend_from_slice(&(definitions.len() as i32).to_le_bytes());
        for (field_id, _) in definitions {
            data.extend_from_slice(&field_id.to_le_bytes());
            data.extend_from_slice(&0i32.to_le_bytes());
        }

// Definition record: blockSize gives the length of this entry
        for (field_id, type_id) in definitions {
            let mut record = vec![0u8; 16];
            record[0..2].copy_from_slice(&16u16.to_le_bytes());
            record[4..8].copy_from_slice(&field_id.to_le_bytes());
            record[12..14].copy_from_slice(&(*type_id as u16).to_le_bytes());
            data.extend_from_slice(&record);
        }

        data
    }

/// Field ID of the built-in task field NAME.
    fn name_field_id() -> i32 {
        MPPTaskField::TASK_FIELD_BASE | 14
    }

/// Field ID of the built-in task field ID.
    fn id_field_id() -> i32 {
        MPPTaskField::TASK_FIELD_BASE | 23
    }

/// The alias is registered on the configuration of the corresponding field.
    #[test]
    fn process_registers_aliases() {
        let block = custom_fields_block(
            &[(name_field_id(), "My Alias"), (id_field_id(), "Id Alias")],
            &[(name_field_id(), 7)],
        );

        let file = ProjectFile::new();
        CustomFieldReader12::new(&file, Some(&block)).process();

        let container = file.borrow().get_custom_fields();
        let container = container.borrow();

        assert_eq!(
            container.get_field_type_by_alias(FieldTypeClass::Task, "My Alias"),
            Some(FieldKey::Task(TaskField::Name))
        );
        assert_eq!(
            container.get_field_type_by_alias(FieldTypeClass::Task, "Id Alias"),
            Some(FieldKey::Task(TaskField::Id))
        );
        assert_eq!(
            container
                .get(FieldKey::Task(TaskField::Name))
                .and_then(|field| field.borrow().get_alias().map(str::to_owned)),
            Some("My Alias".to_owned())
        );
        assert_eq!(
            container
                .get(FieldKey::Task(TaskField::Id))
                .and_then(|field| field.borrow().get_alias().map(str::to_owned)),
            Some("Id Alias".to_owned())
        );
    }

/// Does not panic and registers no aliases when the data block is `None` / empty / truncated.
    #[test]
    fn process_handles_missing_and_truncated_data() {
        let file = ProjectFile::new();

// Java: m_data == null → return immediately
        CustomFieldReader12::new(&file, None).process();
        assert_eq!(file.borrow().get_custom_fields().borrow().size(), 0);

// Empty block
        CustomFieldReader12::new(&file, Some(&[])).process();
        assert_eq!(file.borrow().get_custom_fields().borrow().size(), 0);

// Only the alias block header: the alias entries cannot be read (Java throws an out-of-bounds exception)
        let block = [
            32i32.to_le_bytes().as_slice(),
            32i32.to_le_bytes().as_slice(),
            1i32.to_le_bytes().as_slice(),
        ]
        .concat();
        CustomFieldReader12::new(&file, Some(&block)).process();
        assert_eq!(file.borrow().get_custom_fields().borrow().size(), 0);
    }

/// An alias with an empty name is not registered (Java's `!alias.isEmpty()`).
    #[test]
    fn process_skips_empty_aliases() {
        let block = custom_fields_block(&[(name_field_id(), "")], &[]);

        let file = ProjectFile::new();
        CustomFieldReader12::new(&file, Some(&block)).process();

        assert_eq!(file.borrow().get_custom_fields().borrow().size(), 0);
    }

/// An unrecognized field ID does not register an alias (Java passes null to `getOrCreate` and throws an NPE).
    #[test]
    fn process_skips_unknown_field_ids() {
        let block = custom_fields_block(&[(0x1234_0005, "Unknown Alias")], &[]);

        let file = ProjectFile::new();
        CustomFieldReader12::new(&file, Some(&block)).process();

        assert_eq!(file.borrow().get_custom_fields().borrow().size(), 0);
    }

/// Offset advancement helper (Java's `offset += size`).
    #[test]
    fn advance_offset_matches_java() {
        assert_eq!(advance_offset(4, 8), Some(12));
        assert_eq!(advance_offset(4, 0), Some(4));
// A negative advance moves the offset below zero → None (Java throws an out-of-bounds exception)
        assert_eq!(advance_offset(4, -8), None);
    }
}
