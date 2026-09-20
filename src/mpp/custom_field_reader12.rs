//! 对应 Java: `org/mpxj/mpp/CustomFieldReader12.java`
//!
//! 读取 MPP12 的字段别名（`Props` 里的 `CUSTOM_FIELDS` 块）：块内依次是
//! 「别名块 → 两个未知块 → 字段定义块」，别名与字段号一起写进
//! [`CustomFieldContainer`]（`CustomField::set_alias`）。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `CustomFieldReader12(ProjectFile file, byte[] data)` | `new(&Rc<RefCell<ProjectFile>>, Option<&[u8]>)` | `ProjectFile` 共享为 `Rc<RefCell<..>>`；Java 的 `byte[]` 允许为 `null`（`process` 首行判空），故 Rust 侧用 `Option<&[u8]>`（「`null` → `Option<T>`」约定），构造时复制为 `Option<Vec<u8>>`（Java 保存的是调用方的数组引用，Rust 侧入参是借用，必须持有数据） |
//! | `CustomFieldContainer m_fields = file.getCustomFields()` | `Rc<RefCell<CustomFieldContainer>>` | 同 Java |
//! | `Map<Integer, String> aliasMap` | `HashMap<i32, String>` | 同 Java（未命中的字段号由 `FieldTypeHelper` 处理，见下） |
//! | `ByteArrayHelper.getInt/getShort` 越界抛 `ArrayIndexOutOfBoundsException` | [`checked_int`] / [`checked_short`] 返回 `None` 并中断解析 | 遵循「不 panic」约定，逐点注释 |
//! | `offset += size`（`size` 为 `int`，Java 允许负值把偏移回退） | `advance_offset` 返回 `Option<usize>`，回退到负偏移时中断解析 | Java 在随后的越界读取中同样会抛 `ArrayIndexOutOfBoundsException` |
//! | `EnterpriseCustomFieldDataType.getDataTypeFromID(int)` | [`EnterpriseCustomFieldDataType::get_data_type_from_id`](crate::mpp::enterprise_custom_field_data_type::EnterpriseCustomFieldDataType::get_data_type_from_id) | 与 Java 一样委托给该工具类（本批次同期落地的 `crate::mpp::enterprise_custom_field_data_type`） |
//! | `m_fields.getOrCreate(FieldTypeHelper.getInstance(m_file, k)).setAlias(v)` | 逐个 `CustomFieldContainer::get_or_create(&container, key)` + `set_alias(Some(alias))` | Java 在 `FieldTypeHelper` 未识别字段号时会把 `null` 传给 `getOrCreate`（`HashMap` 允许 null 键，但 `CustomField` 随后会因 `field.getFieldTypeClass()` 抛 NPE）；Rust 的 `FieldKey` 无「未知」变体，故跳过该字段号 |
//! | `aliasMap.forEach(...)` | `for (field_id, alias) in alias_map` | 遍历顺序与 Java 的 `HashMap` 一样不保证（各自内部顺序不同） |
//!
//! 跳过的 Java 方法：无（`process()` 逐行翻译，两个被注释掉的调试语句按原样保留注释）。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。
//! 注释里出现的 `FieldTypeHelper` 调用是为将来扩展预留（Java 源文件同样只有注释）。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::common::field_type_helper::FieldTypeHelper;
use crate::custom_field_container::CustomFieldContainer;
use crate::mpp::custom_field_value_reader::{checked_int, checked_short};
use crate::mpp::enterprise_custom_field_data_type::EnterpriseCustomFieldDataType;
use crate::mpp::mpp_utility::MPPUtility;
use crate::project_file::ProjectFile;

/// 读取 MPP12 的字段别名。
///
/// 对应 Java: `class CustomFieldReader12`
pub struct CustomFieldReader12 {
    /// 项目文件。
    ///
    /// 对应 Java 字段: `private final ProjectFile m_file;`
    file: Rc<RefCell<ProjectFile>>,

    /// 自定义字段容器。
    ///
    /// 对应 Java 字段: `private final CustomFieldContainer m_fields;`
    fields: Rc<RefCell<CustomFieldContainer>>,

    /// `Props` 中的 `CUSTOM_FIELDS` 数据块。
    ///
    /// 对应 Java 字段: `private final byte[] m_data;`
    data: Option<Vec<u8>>,
}

impl CustomFieldReader12 {
    /// 构造方法。
    ///
    /// 对应 Java: `public CustomFieldReader12(ProjectFile file, byte[] data)`
    pub fn new(file: &Rc<RefCell<ProjectFile>>, data: Option<&[u8]>) -> Self {
        // Java: m_fields = file.getCustomFields();
        // 文件正被可变借用时（正常调用路径不会发生）退化为一枚独立容器，避免 panic。
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

    /// 处理字段别名。
    ///
    /// 对应 Java: `public void process()`
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
        //（Java 注释：先出现两次「长度」，故跳过 8 字节）
        let Some(alias_block_size) = checked_int(data, offset) else {
            return;
        };
        offset += 8;

        // Java: int numberOfAliases = ByteArrayHelper.getInt(m_data, offset); offset += 4;
        let Some(number_of_aliases) = checked_int(data, offset) else {
            return;
        };
        offset += 4;

        // Java 把 `aliasBlockSize` 当作循环上界使用：负值时循环体不执行，Rust 侧同样处理。
        let alias_block_limit = usize::try_from(alias_block_size).unwrap_or(0);

        // Java: while (index < numberOfAliases && offset < aliasBlockSize)
        while index < number_of_aliases && offset < alias_block_limit {
            // Java: int fieldID = ByteArrayHelper.getInt(m_data, offset); offset += 4;
            let Some(field_id) = checked_int(data, offset) else {
                break;
            };
            offset += 4;

            // Java: int aliasOffset = ByteArrayHelper.getInt(m_data, offset) + 4; offset += 4;
            //（Java 注释：别名偏移「不知为何」要 +4）
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

        // Java 注释（原文保留）：
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

        // Java 注释：字段定义块，长度出现两次，故跳过 8 字节
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

        // Java 注释（原文保留）：//System.out.println(ByteArrayHelper.hexdump(m_data, offset, m_data.length-offset, false));
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
            // Java 越界时抛异常；Rust 侧取不到时按 0 处理（`getDataTypeFromID(0)` = CURRENCY）
            let custom_field_data_type = EnterpriseCustomFieldDataType::get_data_type_from_id(
                checked_short(data, offset + 12).unwrap_or(0),
            );

            // Java: FieldTypeHelper.getInstance(m_file, ByteArrayHelper.getInt(m_data, offset + 4), customFieldDataType);
            //（Java 只用其副作用：确保企业自定义字段被创建；返回值被丢弃）
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
            // Java 会把 `null` 传给 `getOrCreate`（随后抛 NPE）；Rust 侧跳过未识别的字段号
            let Some(field) = FieldTypeHelper::get_instance(&self.file, field_id) else {
                continue;
            };

            let custom_field = CustomFieldContainer::get_or_create(&self.fields, field.to_field_key());
            custom_field.borrow_mut().set_alias(Some(alias));
        }
    }
}

/// 按 Java 的 `offset += size` 推进偏移。
///
/// 对应 Java: `offset = 4 + aliasBlockSize` / `offset += unknownBlock1Size` 等。
/// 结果为负时返回 `None`（Java 会在随后的越界读取中抛 `ArrayIndexOutOfBoundsException`，
/// Rust 侧中断解析）。
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

    /// UTF-16LE（NUL 结尾）字节。
    fn utf16(text: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        bytes.extend_from_slice(&[0x00, 0x00]);
        bytes
    }

    /// 拼装完整的 `CUSTOM_FIELDS` 数据块。
    ///
    /// 布局与 Java 的解析顺序一一对应：
    /// ```text
    /// [0..4)        别名块大小（自偏移 4 起算的块长度；Java 用 offset = 4 + aliasBlockSize）
    /// [4..8)        别名块大小（重复一次）
    /// [8..12)       别名条数
    /// [12..12+8n)   每条别名：字段号 + 别名偏移（Java 解析时 +4 才是绝对偏移）
    /// 其后          别名文本（UTF-16LE、NUL 结尾）
    /// 其后          未知块 1：4 字节大小 + 内容
    /// 其后          未知块 2：4 字节大小 + 内容
    /// 其后          字段定义块：大小 + 大小 + 定义条数 + 每条 8 字节
    /// 其后          定义记录：首 2 字节为 blockSize，+4 字段号，+12 类型 ID
    /// ```
    fn custom_fields_block(aliases: &[(i32, &str)], definitions: &[(i32, i32)]) -> Vec<u8> {
        let count = aliases.len();

        // 别名文本紧随别名条目之后，记录其绝对偏移
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

        // 未知块 1：4 字节大小 + 内容
        data.extend_from_slice(&4i32.to_le_bytes());
        data.extend_from_slice(&[0u8; 4]);
        // 未知块 2：4 字节大小（0）+ 无内容
        data.extend_from_slice(&0i32.to_le_bytes());

        // 字段定义块：长度出现两次，随后是条数，再是每条 8 字节
        data.extend_from_slice(&(definitions.len() as i32 * 8 + 4).to_le_bytes());
        data.extend_from_slice(&(definitions.len() as i32 * 8 + 4).to_le_bytes());
        data.extend_from_slice(&(definitions.len() as i32).to_le_bytes());
        for (field_id, _) in definitions {
            data.extend_from_slice(&field_id.to_le_bytes());
            data.extend_from_slice(&0i32.to_le_bytes());
        }

        // 定义记录：blockSize 指出本条长度
        for (field_id, type_id) in definitions {
            let mut record = vec![0u8; 16];
            record[0..2].copy_from_slice(&16u16.to_le_bytes());
            record[4..8].copy_from_slice(&field_id.to_le_bytes());
            record[12..14].copy_from_slice(&(*type_id as u16).to_le_bytes());
            data.extend_from_slice(&record);
        }

        data
    }

    /// 内置任务字段 NAME 的字段号。
    fn name_field_id() -> i32 {
        MPPTaskField::TASK_FIELD_BASE | 14
    }

    /// 内置任务字段 ID 的字段号。
    fn id_field_id() -> i32 {
        MPPTaskField::TASK_FIELD_BASE | 23
    }

    /// 别名被登记到对应字段的配置上。
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

    /// 数据块为 `None` / 空 / 被截断时不 panic，也不登记别名。
    #[test]
    fn process_handles_missing_and_truncated_data() {
        let file = ProjectFile::new();

        // Java: m_data == null → 直接返回
        CustomFieldReader12::new(&file, None).process();
        assert_eq!(file.borrow().get_custom_fields().borrow().size(), 0);

        // 空块
        CustomFieldReader12::new(&file, Some(&[])).process();
        assert_eq!(file.borrow().get_custom_fields().borrow().size(), 0);

        // 只有别名块头：别名条目读不出来（Java 抛越界异常）
        let block = [
            32i32.to_le_bytes().as_slice(),
            32i32.to_le_bytes().as_slice(),
            1i32.to_le_bytes().as_slice(),
        ]
        .concat();
        CustomFieldReader12::new(&file, Some(&block)).process();
        assert_eq!(file.borrow().get_custom_fields().borrow().size(), 0);
    }

    /// 名为空的别名不登记（Java 的 `!alias.isEmpty()`）。
    #[test]
    fn process_skips_empty_aliases() {
        let block = custom_fields_block(&[(name_field_id(), "")], &[]);

        let file = ProjectFile::new();
        CustomFieldReader12::new(&file, Some(&block)).process();

        assert_eq!(file.borrow().get_custom_fields().borrow().size(), 0);
    }

    /// 未识别的字段号不登记别名（Java 会把 null 传给 `getOrCreate` 并抛 NPE）。
    #[test]
    fn process_skips_unknown_field_ids() {
        let block = custom_fields_block(&[(0x1234_0005, "Unknown Alias")], &[]);

        let file = ProjectFile::new();
        CustomFieldReader12::new(&file, Some(&block)).process();

        assert_eq!(file.borrow().get_custom_fields().borrow().size(), 0);
    }

    /// 偏移推进辅助函数（Java 的 `offset += size`）。
    #[test]
    fn advance_offset_matches_java() {
        assert_eq!(advance_offset(4, 8), Some(12));
        assert_eq!(advance_offset(4, 0), Some(4));
        // 负数回退到负偏移 → None（Java 会抛越界异常）
        assert_eq!(advance_offset(4, -8), None);
    }
}
