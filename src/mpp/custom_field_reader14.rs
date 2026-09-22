//! Corresponds to Java: `org/mpxj/mpp/CustomFieldReader14.java`
//!
//! Reads MPP14 field aliases (the `CUSTOM_FIELDS` block in `Props`). The block structure is the same as MPP12
//! (alias block -> two unknown blocks -> field definition block); the difference is that the field definition block uses fixed 88-byte records,
//! and the Java side performs explicit bounds checks for every offset.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `CustomFieldReader14(ProjectFile file, byte[] data)` | `new(&Rc<RefCell<ProjectFile>>, Option<&[u8]>)` | Consistent with [`CustomFieldReader12`](crate::mpp::custom_field_reader12::CustomFieldReader12): `ProjectFile` is shared as `Rc<RefCell<..>>`, nullable `byte[]` -> `Option<&[u8]>` (copied into `Option<Vec<u8>>` at construction) |
//! | `Map<Integer, String> aliasMap` | `HashMap<i32, String>` | Same as Java |
//! | `if (offset + N > m_data.length) { return; }` | `if !has(data, offset, N) { return; }` | Same semantics, and avoids `offset + N` overflow |
//! | `offset += size` (`int`, may be negative) | [`advance_offset`] returns `Option<usize>`, and parsing is interrupted when it falls back to a negative offset | Java's explicit bounds check would likewise `return` afterwards |
//! | `EnterpriseCustomFieldDataType.getDataTypeFromID(int)` | [`EnterpriseCustomFieldDataType::get_data_type_from_id`](crate::mpp::enterprise_custom_field_data_type::EnterpriseCustomFieldDataType::get_data_type_from_id) | Delegates to that utility class as in Java (`crate::mpp::enterprise_custom_field_data_type` is already in place) |
//! | `m_fields.getOrCreate(FieldTypeHelper.getInstance(m_file, k)).setAlias(v)` | per-item `CustomFieldContainer::get_or_create` + `set_alias` | When `FieldTypeHelper` does not recognize the field number, Java passes `null` and throws an NPE; Rust's `FieldKey` has no "unknown" variant, so it is skipped |
//!
//! Skipped Java methods: none (`process()` is translated line by line; debug statements commented out in the source are kept as comments as-is).
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::common::field_type_helper::FieldTypeHelper;
use crate::custom_field_container::CustomFieldContainer;
use crate::mpp::custom_field_value_reader::{checked_int, checked_short};
use crate::mpp::enterprise_custom_field_data_type::EnterpriseCustomFieldDataType;
use crate::mpp::mpp_utility::MPPUtility;
use crate::project_file::ProjectFile;

/// Reads MPP14 field aliases.
///
/// Corresponds to Java: `class CustomFieldReader14`
pub struct CustomFieldReader14 {
/// Project file.
    ///
    /// Corresponds to Java field: `private final ProjectFile m_file;`
    file: Rc<RefCell<ProjectFile>>,

/// Custom field container.
    ///
    /// Corresponds to Java field: `private final CustomFieldContainer m_fields;`
    fields: Rc<RefCell<CustomFieldContainer>>,

/// The `CUSTOM_FIELDS` data block in `Props`.
    ///
    /// Corresponds to Java field: `private final byte[] m_data;`
    data: Option<Vec<u8>>,
}

impl CustomFieldReader14 {
/// Constructor.
    ///
    /// Corresponds to Java: `public CustomFieldReader14(ProjectFile file, byte[] data)`
    pub fn new(file: &Rc<RefCell<ProjectFile>>, data: Option<&[u8]>) -> Self {
        // Java: m_fields = file.getCustomFields();
// When the file is already mutably borrowed (which does not happen on the normal call path), degrade gracefully to a standalone container to avoid panicking.
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

/// Process field aliases.
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
// (Java comment: "length" appears twice first, so skip 8 bytes)
        let Some(alias_block_size) = checked_int(data, offset) else {
            return;
        };
        offset += 8;

        // Java: int numberOfAliases = ByteArrayHelper.getInt(m_data, offset); offset += 4;
        let Some(number_of_aliases) = checked_int(data, offset) else {
            return;
        };
        offset += 4;

// Java uses `aliasBlockSize` as the loop upper bound: a negative value means the loop body does not execute; the Rust side does the same.
        let alias_block_limit = usize::try_from(alias_block_size).unwrap_or(0);

        // Java: while (index < numberOfAliases && offset < aliasBlockSize)
        while index < number_of_aliases && offset < alias_block_limit {
            // Java: int fieldID = ByteArrayHelper.getInt(m_data, offset); offset += 4;
            //       int aliasOffset = ByteArrayHelper.getInt(m_data, offset) + 4; offset += 4;
            let Some(field_id) = checked_int(data, offset) else {
                break;
            };
            offset += 4;

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

        // Java: offset = 4 + aliasBlockSize;
        let Some(mut offset) = advance_offset(4, alias_block_size) else {
            return;
        };

        // Java: if (offset + 4 > m_data.length) { return; }
        if !has(data, offset, 4) {
            return;
        }

        // Java: int unknownBlock1Size = ByteArrayHelper.getInt(m_data, offset); offset += 4; offset += unknownBlock1Size;
        let Some(unknown_block1_size) = checked_int(data, offset) else {
            return;
        };
        let Some(next) = advance_offset(offset + 4, unknown_block1_size) else {
            return;
        };
        offset = next;

        // Java: if (offset > m_data.length) { return; }
        if offset > data.len() {
            return;
        }

        // Java: if (offset + 4 > m_data.length) { return; }
        if !has(data, offset, 4) {
            return;
        }

        // Java: int unknownBlock2Size = ByteArrayHelper.getInt(m_data, offset); offset += 4; offset += unknownBlock2Size;
        let Some(unknown_block2_size) = checked_int(data, offset) else {
            return;
        };
        let Some(next) = advance_offset(offset + 4, unknown_block2_size) else {
            return;
        };
        offset = next;

        // Java: if (offset > m_data.length) { return; }
        if offset > data.len() {
            return;
        }

        // Java: if (offset + 8 > m_data.length) { return; }
        if !has(data, offset, 8) {
            return;
        }

        // Java: int numberOfDefinitions = ByteArrayHelper.getInt(m_data, offset); offset += 4;
        let Some(number_of_definitions) = checked_int(data, offset) else {
            return;
        };
        offset += 4;

// Java comment (kept verbatim): //int definitionsBlockSize = MPPUtility.getInt(m_data, offset);
        // Java: offset += 4;
        offset += 4;

// Java comment: 88 byte blocks
        for _ in 0..number_of_definitions {
            // Java: if (offset + 88 > m_data.length) { break; }
            if !has(data, offset, 88) {
                break;
            }

            // Java: DataType customFieldDataType = EnterpriseCustomFieldDataType.getDataTypeFromID(ByteArrayHelper.getShort(m_data, offset + 12));
            let custom_field_data_type = EnterpriseCustomFieldDataType::get_data_type_from_id(
                checked_short(data, offset + 12).unwrap_or(0),
            );

            // Java: FieldTypeHelper.getInstance(m_file, ByteArrayHelper.getInt(m_data, offset), customFieldDataType);
// (Java uses it only for its side effect: ensuring the enterprise custom field is created; the return value is discarded)
            if let Some(field_id) = checked_int(data, offset) {
                let _ = FieldTypeHelper::get_instance_with_data_type(
                    &self.file,
                    field_id,
                    custom_field_data_type,
                );
            }

            // Java: offset += 88;
            offset += 88;
        }

        // Java: aliasMap.forEach((k, v) -> m_fields.getOrCreate(FieldTypeHelper.getInstance(m_file, k.intValue())).setAlias(v));
        for (field_id, alias) in alias_map {
// Java passes `null` to `getOrCreate` (then throws an NPE); the Rust side skips unrecognized field numbers
            let Some(field) = FieldTypeHelper::get_instance(&self.file, field_id) else {
                continue;
            };

            let custom_field =
                CustomFieldContainer::get_or_create(&self.fields, field.to_field_key());
            custom_field.borrow_mut().set_alias(Some(alias));
        }
    }
}

/// Determine whether `size` bytes starting at `offset` fall within the data block.
///
/// Corresponds to Java: `offset + N > m_data.length` (the Rust side avoids addition overflow).
fn has(data: &[u8], offset: usize, size: usize) -> bool {
    offset
        .checked_add(size)
        .is_some_and(|end| end <= data.len())
}

/// Advance the offset as Java's `offset += size` does.
///
/// Corresponds to Java: `offset = 4 + aliasBlockSize` / `offset += unknownBlock1Size`, etc.
/// Returns `None` when the result is negative (Java would throw `ArrayIndexOutOfBoundsException` in the subsequent out-of-bounds read,
/// and the Rust side interrupts parsing).
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

/// Assemble the complete `CUSTOM_FIELDS` data block.
    ///
/// Difference from MPP12: the field definition block's length field and count are ordered "count first, block size second",
/// and the definition records are fixed at 88 bytes, with the field number at the start of the record.
    ///
    /// ```text
/// [0..4)        alias block size (counted from offset 4)
/// [4..8)        alias block size (repeated once)
/// [8..12)       alias count
/// [12..12+8n)   each alias: field number + alias offset (Java adds +4 when parsing)
/// then          alias text (UTF-16LE, NUL-terminated)
/// then          unknown block 1: 4-byte size + content
/// then          unknown block 2: 4-byte size (0)
/// then          definition count (4 bytes) + definition block size (4 bytes)
/// then          definition records: 88 bytes each, 4-byte field number at the start, +12 type ID
    /// ```
    fn custom_fields_block(aliases: &[(i32, &str)], definitions: &[(i32, i32)]) -> Vec<u8> {
        let count = aliases.len();

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
            data.extend_from_slice(&((text_offsets[index] as i32) - 4).to_le_bytes());
        }
        data.extend_from_slice(&texts);

// Unknown block 1: 4-byte size + content
        data.extend_from_slice(&4i32.to_le_bytes());
        data.extend_from_slice(&[0u8; 4]);
// Unknown block 2: 4-byte size (0)
        data.extend_from_slice(&0i32.to_le_bytes());

// Definition count + definition block size
        data.extend_from_slice(&(definitions.len() as i32).to_le_bytes());
        data.extend_from_slice(&(definitions.len() as i32 * 88).to_le_bytes());

// 88-byte definition records
        for (field_id, type_id) in definitions {
            let mut record = vec![0u8; 88];
            record[0..4].copy_from_slice(&field_id.to_le_bytes());
            record[12..14].copy_from_slice(&(*type_id as u16).to_le_bytes());
            data.extend_from_slice(&record);
        }

        data
    }

/// Field number of the built-in task field NAME.
    fn name_field_id() -> i32 {
        MPPTaskField::TASK_FIELD_BASE | 14
    }

/// Field number of the built-in task field ID.
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
        CustomFieldReader14::new(&file, Some(&block)).process();

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
    }

/// Enterprise custom fields (field numbers with the 0x8000 bit) are created while parsing definition records.
    #[test]
    fn process_creates_enterprise_custom_fields() {
// The 0x8000 bit indicates an enterprise custom field; type ID 7 = STRING
        let block = custom_fields_block(&[], &[(MPPTaskField::TASK_FIELD_BASE | 0x8000, 7)]);

        let file = ProjectFile::new();
        assert!(file.borrow().get_user_defined_fields().is_empty());

        CustomFieldReader14::new(&file, Some(&block)).process();

// Java uses `FieldTypeHelper.getInstance` only for its side effect: ensuring the enterprise custom field is created
        assert_eq!(file.borrow().get_user_defined_fields().get_task_fields().len(), 1);
// The alias map is empty, so there is no configuration in the custom field container
        assert_eq!(file.borrow().get_custom_fields().borrow().size(), 0);
    }

/// Does not panic when the data block is `None` / empty / truncated, and does not register aliases.
    #[test]
    fn process_handles_missing_and_truncated_data() {
        let file = ProjectFile::new();

// Java: m_data == null -> return directly
        CustomFieldReader14::new(&file, None).process();
        assert_eq!(file.borrow().get_custom_fields().borrow().size(), 0);

// Empty block
        CustomFieldReader14::new(&file, Some(&[])).process();
        assert_eq!(file.borrow().get_custom_fields().borrow().size(), 0);

// Only the alias block header: alias entries cannot be read (Java throws an out-of-bounds exception)
        let block = [
            32i32.to_le_bytes().as_slice(),
            32i32.to_le_bytes().as_slice(),
            1i32.to_le_bytes().as_slice(),
        ]
        .concat();
        CustomFieldReader14::new(&file, Some(&block)).process();
        assert_eq!(file.borrow().get_custom_fields().borrow().size(), 0);

// The unknown block size is out of bounds (Java's `if (offset > m_data.length) { return; }`)
        let block = [
            8i32.to_le_bytes().as_slice(),
            8i32.to_le_bytes().as_slice(),
            0i32.to_le_bytes().as_slice(),
// After the alias block: the size of unknown block 1 exceeds the data block
            0xFFFFi32.to_le_bytes().as_slice(),
        ]
        .concat();
        CustomFieldReader14::new(&file, Some(&block)).process();
        assert_eq!(file.borrow().get_custom_fields().borrow().size(), 0);
    }

/// An alias with an empty name is not registered (Java's `!alias.isEmpty()`).
    #[test]
    fn process_skips_empty_aliases() {
        let block = custom_fields_block(&[(name_field_id(), "")], &[]);

        let file = ProjectFile::new();
        CustomFieldReader14::new(&file, Some(&block)).process();

        assert_eq!(file.borrow().get_custom_fields().borrow().size(), 0);
    }

/// An unrecognized field number does not register an alias (Java passes null to `getOrCreate` and throws an NPE).
    #[test]
    fn process_skips_unknown_field_ids() {
        let block = custom_fields_block(&[(0x1234_0005, "Unknown Alias")], &[]);

        let file = ProjectFile::new();
        CustomFieldReader14::new(&file, Some(&block)).process();

        assert_eq!(file.borrow().get_custom_fields().borrow().size(), 0);
    }

/// `has` / `advance_offset` are consistent with Java's bounds checks.
    #[test]
    fn bounds_helpers_match_java() {
        let data = [0u8; 8];

        assert!(has(&data, 0, 8));
        assert!(has(&data, 4, 4));
        assert!(!has(&data, 4, 5));
        assert!(!has(&data, 9, 1));

        assert_eq!(advance_offset(4, 8), Some(12));
        assert_eq!(advance_offset(4, -4), Some(0));
// A negative size falls back to a negative offset -> None (Java throws an out-of-bounds exception)
        assert_eq!(advance_offset(4, -8), None);
    }
}
