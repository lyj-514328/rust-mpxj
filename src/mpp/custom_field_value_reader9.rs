//! 对应 Java: `org/mpxj/mpp/CustomFieldValueReader9.java`
//!
//! MPP9 的取值列表读取器。与 12 / 14 不同，本类**不继承**
//! [`CustomFieldValueReader`](crate::mpp::custom_field_value_reader::CustomFieldValueReader)，
//! 它自己解析两处数据：
//!
//! 1. `Props` 里的 `TASK_FIELD_ATTRIBUTES` 块 —— 「字段号 → 取值列表（值数组 + 描述数组）」；
//! 2. `TBkndOutlCode` 存储 —— 企业大纲代码的取 值/描述（定长块给字段号，变长块给文本）。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `CustomFieldValueReader9(DirectoryEntry projectDir, ProjectFile file, Props projectProps)` | `new(&DirectoryEntry, &Rc<RefCell<ProjectFile>>, &Props)` | 按约定 `ProjectFile` 共享为 `Rc<RefCell<..>>`；`Props` 为借用 |
//! | `private final Props m_projectProps`（保存引用，`processCustomFieldValues` 里取 `TASK_FIELD_ATTRIBUTES`） | 构造时取出 `TASK_FIELD_ATTRIBUTES` 一份副本存入 `task_field_attributes` | `Props` 未实现 `Clone`，且本 crate 的既有做法（见 `DocumentInputStreamFactory`）是「构造时把需要的取值取出、不持有 `Props`」；数据内容在构造与解析之间不会变化，语义等价 |
//! | `private final ProjectProperties m_properties` | 不保存引用，按需用 `ProjectFile::try_borrow` 借用 | 与 `CustomFieldValueReader` 同样的取舍 |
//! | `public void process() throws IOException` | `pub fn process(&mut self) -> MpxjResult<()>` | Java 里唯一会变化的状态是 unique ID 计数器，Rust 侧放在 `&mut self` 上；Java 的 `IOException` 对应 [`MpxjResult`]（`FixedMeta` / `VarMeta9` 解析失败） |
//! | `Map<FieldType, List<Pair<String, String>>>` | `HashMap<Option<MppFieldKey>, (Option<MppFieldType>, Vec<(Option<String>, Option<String>)>)>` | Java 的 `HashMap` 允许 `null` 键（`map.get(id)` 未命中的情形），Rust 侧用 `Option<MppFieldKey>` 表达；同时把「该键对应的字段」保存在值里，以便还原 `field == null ? "Unknown" : field.getName()`。`Pair` 用一个二元组表达（`Pair` 类型本身可用，但此处无需 `getFirst`/`getSecond` 之外的语义） |
//! | `List<Object> convertType(...)` | `Vec<Option<FieldValue>>` | Java 的列表元素可能为 `null`（如 `getTimestamp` / `getAdjustedDuration` 返回 null），Rust 侧用 `Option` 保持下标对齐 |
//! | `ByteArrayHelper.getInt/getShort` 越界抛异常 | [`checked_int`] / [`checked_short`] 返回 `None`（跳过 / 按 0 处理），`MPPUtility.getByteArray` 越界不做复制 | 遵循「不 panic」约定 |
//! | `UUID.nameUUIDFromBytes(byte[])` | 本文件的 [`name_uuid_from_bytes`]（自实现的 MD5 + 版本/变体位） | Java 的该方法生成基于 MD5 的「版本 3」UUID；`uuid` crate 的 `v3` feature 未启用，且本 crate 不允许新增依赖，故就地实现 RFC 1321 的 MD5 |
//! | `m_container.getOrCreate(null)`（`field` 为 null 时） | 跳过该字段 | Java 的 `FieldTypeHelper` 未识别字段号时会返回占位对象 / null；Rust 的 `FieldKey` 没有「未知」变体（见 `field_type_helper` 的说明），无法作为容器键 |
//! | `IOException` / `NullPointerException`（缺少 OLE 条目、数据越界） | 返回 `Ok(())` / 跳过该项 | 遵循「不 panic」约定，逐点注释 |
//!
//! 跳过的 Java 方法：无（`process` / `processCustomFieldValues` / `processOutlineCodeValues` /
//! 两个 `populateContainer` 重载 / `convertType` / `getDataType` 全部翻译；Rust 侧因不支持重载，
//! 第二个 `populateContainer` 改名为 `populate_container_from_outline_code`）。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::mpp_field_type::{MppFieldKey, MppFieldType};
use crate::custom_field_container::CustomFieldContainer;
use crate::custom_field_value_data_type::CustomFieldValueDataType;
use crate::data_type::DataType;
use crate::field_type::FieldType;
use crate::field_type_class::FieldTypeClass;
use crate::field_value::FieldValue;
use crate::mpp::custom_field_value_item::CustomFieldValueItem;
use crate::mpp::custom_field_value_reader::{checked_int, checked_short};
use crate::mpp::fixed_data::FixedData;
use crate::mpp::fixed_meta::FixedMeta;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::ole::DirectoryEntry;
use crate::mpp::props::Props;
use crate::mpp::props_key::PropsKey;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_meta::VarMeta;
use crate::mpp::var_meta9::VarMeta9;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;
use crate::project_properties::ProjectProperties;

/// 大纲代码「取值」在变长数据块中的数据类型键。
///
/// 对应 Java: `private static final Integer VALUE = Integer.valueOf(1);`
const VALUE: i32 = 1;

/// 大纲代码「描述」在变长数据块中的数据类型键。
///
/// 对应 Java: `private static final Integer DESCRIPTION = Integer.valueOf(2);`
const DESCRIPTION: i32 = 2;

/// MPP9 取值列表读取器。
///
/// 对应 Java: `class CustomFieldValueReader9`
pub struct CustomFieldValueReader9 {
    /// 项目存储（`TBkndOutlCode` 的父存储）。
    ///
    /// 对应 Java 字段: `private final DirectoryEntry m_projectDir;`
    project_dir: DirectoryEntry,

    /// 项目文件。
    ///
    /// 对应 Java 字段: `private final ProjectFile m_file;`
    file: Rc<RefCell<ProjectFile>>,

    /// `Props` 中的 `TASK_FIELD_ATTRIBUTES` 数据块。
    ///
    /// 对应 Java: `m_projectProps.getByteArray(PropsKey.TASK_FIELD_ATTRIBUTES)`
    task_field_attributes: Option<Vec<u8>>,

    /// 自定义字段容器。
    ///
    /// 对应 Java 字段: `private final CustomFieldContainer m_container;`
    container: Rc<RefCell<CustomFieldContainer>>,

    /// 取值项 unique ID 计数器。
    ///
    /// 对应 Java 字段: `private int m_valueUniqueIDCounter;`
    value_unique_id_counter: i32,
}

impl CustomFieldValueReader9 {
    /// 构造方法。
    ///
    /// 对应 Java: `public CustomFieldValueReader9(DirectoryEntry projectDir, ProjectFile file, Props projectProps)`
    pub fn new(
        project_dir: &DirectoryEntry,
        file: &Rc<RefCell<ProjectFile>>,
        project_props: &Props,
    ) -> Self {
        // Java: m_properties = file.getProjectProperties(); m_container = file.getCustomFields();
        // 文件正被可变借用时（正常调用路径不会发生）退化为一枚独立容器，避免 panic。
        let container = match file.try_borrow() {
            Ok(file) => file.get_custom_fields(),
            Err(_) => CustomFieldContainer::new(),
        };

        Self {
            project_dir: project_dir.clone(),
            file: Rc::clone(file),
            // Java 在 `processCustomFieldValues` 里才取该块；Rust 侧在构造时取出（见文件头说明）
            task_field_attributes: project_props
                .get_byte_array(PropsKey::TaskFieldAttributes)
                .map(<[u8]>::to_vec),
            container,
            value_unique_id_counter: 0,
        }
    }

    /// 读取全部取值列表。
    ///
    /// 对应 Java: `public void process() throws IOException`
    pub fn process(&mut self) -> MpxjResult<()> {
        // Java: processCustomFieldValues(); processOutlineCodeValues();
        self.process_custom_field_values();
        self.process_outline_code_values()
    }

    /// 读取非大纲代码的自定义字段取值列表。
    ///
    /// 对应 Java: `private void processCustomFieldValues()`
    fn process_custom_field_values(&mut self) {
        // Java: byte[] data = m_projectProps.getByteArray(PropsKey.TASK_FIELD_ATTRIBUTES);
        //       if (data != null) { ... }
        // Rust 侧复制一份局部数据：`populate_container` 需要 `&mut self`（unique ID 计数器），
        // 若直接借用 `self.task_field_attributes` 会与可变借用冲突。
        let Some(data) = self.task_field_attributes.clone() else {
            return;
        };

        // Java: int index = 0; int offset = 0;
        let mut index = 0i32;
        let mut offset = 0usize;

        // Java: int length = ByteArrayHelper.getInt(data, offset); offset += 4;
        // Java 越界时抛异常；Rust 侧直接返回（无取值列表可读）
        let Some(length) = checked_int(&data, offset) else {
            return;
        };
        offset += 4;

        // Java: int numberOfValueLists = ByteArrayHelper.getInt(data, offset); offset += 4;
        let Some(number_of_value_lists) = checked_int(&data, offset) else {
            return;
        };
        offset += 4;

        // Java: while (index < numberOfValueLists && offset < length)
        while index < number_of_value_lists && (offset as i64) < i64::from(length) {
            // Java: field = FieldTypeHelper.getInstance(m_file, ByteArrayHelper.getInt(data, offset));
            let Some(field_id) = checked_int(&data, offset) else {
                break;
            };
            offset += 4;
            let field = FieldTypeHelper::get_instance(&self.file, field_id);

            // Java: valueListOffset = ByteArrayHelper.getInt(data, offset); offset += 4;
            let Some(value_list_offset) = checked_int(&data, offset) else {
                break;
            };
            offset += 4;

            // Java: if (valueListOffset < data.length) { ... }
            //（Java 对负的 valueListOffset 会因负下标抛异常，Rust 侧跳过该条）
            if let Ok(value_list_offset) = usize::try_from(value_list_offset) {
                if value_list_offset < data.len() {
                    // Java: int tempOffset = valueListOffset; tempOffset += 8;
                    let base = value_list_offset as i64;
                    let mut temp_offset = value_list_offset.saturating_add(8);

                    // Java: int dataOffset = ByteArrayHelper.getInt(data, tempOffset) + valueListOffset; tempOffset += 4;
                    let data_offset = checked_int(&data, temp_offset).map(|value| i64::from(value) + base);
                    temp_offset = temp_offset.saturating_add(4);

                    // Java: int endDataOffset = ByteArrayHelper.getInt(data, tempOffset) + valueListOffset; tempOffset += 4;
                    let end_data_offset = checked_int(&data, temp_offset).map(|value| i64::from(value) + base);
                    temp_offset = temp_offset.saturating_add(4);

                    // Java: int endDescriptionOffset = ByteArrayHelper.getInt(data, tempOffset) + valueListOffset;
                    let end_description_offset =
                        checked_int(&data, temp_offset).map(|value| i64::from(value) + base);

                    // 三个偏移任一读不出来时 Java 会抛异常，Rust 侧跳过该条
                    if let (Some(data_offset), Some(end_data_offset), Some(end_description_offset)) =
                        (data_offset, end_data_offset, end_description_offset)
                    {
                        // Java: int valuesLength = endDataOffset - dataOffset;
                        //       byte[] values = new byte[valuesLength];
                        //       MPPUtility.getByteArray(data, dataOffset, valuesLength, values, 0);
                        let values = read_block(&data, data_offset, end_data_offset - data_offset);

                        // Java: int descriptionsLength = endDescriptionOffset - endDataOffset;
                        //       byte[] descriptions = new byte[descriptionsLength];
                        //       MPPUtility.getByteArray(data, endDataOffset, descriptionsLength, descriptions, 0);
                        let descriptions = read_block(
                            &data,
                            end_data_offset,
                            end_description_offset - end_data_offset,
                        );

                        // Java: populateContainer(field, values, descriptions);
                        self.populate_container(field.as_ref(), &values, &descriptions);
                    }
                }
            }

            // Java: index++;
            index += 1;
        }
    }

    /// 读取大纲代码的取值列表。
    ///
    /// 对应 Java: `private void processOutlineCodeValues() throws IOException`
    fn process_outline_code_values(&mut self) -> MpxjResult<()> {
        // Java: DirectoryEntry outlineCodeDir = (DirectoryEntry) m_projectDir.getEntry("TBkndOutlCode");
        // POI 找不到条目时抛异常；Rust 侧返回 Ok(())（没有大纲代码可读）
        let Some(outline_code_dir) = self.project_dir.get_directory("TBkndOutlCode") else {
            return Ok(());
        };

        // Java: FixedMeta fm = new FixedMeta(new DocumentInputStream(((DocumentEntry) outlineCodeDir.getEntry("FixedMeta"))), 10);
        let Some(fixed_meta_document) = outline_code_dir.get_document("FixedMeta") else {
            return Ok(());
        };
        let fixed_meta = FixedMeta::with_item_size(&fixed_meta_document.get_data(), 10)?;

        // Java: FixedData fd = new FixedData(fm, new DocumentInputStream(((DocumentEntry) outlineCodeDir.getEntry("FixedData"))));
        let Some(fixed_data_document) = outline_code_dir.get_document("FixedData") else {
            return Ok(());
        };
        let fixed_data = FixedData::new(&fixed_meta, &fixed_data_document.get_data());

        // Java: Map<Integer, FieldType> map = new HashMap<>();
        let mut map: HashMap<i32, MppFieldType> = HashMap::new();

        // Java: int items = fm.getItemCount(); for (int loop = 0; loop < items; loop++) { ... }
        for loop_index in 0..fixed_meta.get_item_count() {
            // Java: byte[] data = fd.getByteArrayValue(loop); if (data.length < 18) { continue; }
            //（Java 对 null 会抛 NPE，Rust 侧跳过）
            let Some(data) = fixed_data.get_byte_array_value(loop_index) else {
                continue;
            };
            if data.len() < 18 {
                continue;
            }

            // Java: int fieldID = ByteArrayHelper.getInt(data, 12);
            let field_id = ByteArrayHelper::get_int(data, 12);

            // Java: FieldType fieldType = FieldTypeHelper.getInstance(m_file, fieldID);
            //       if (fieldType != null && fieldType.getFieldTypeClass() != FieldTypeClass.UNKNOWN)
            if let Some(field_type) = FieldTypeHelper::get_instance(&self.file, field_id) {
                if field_type.get_field_type_class() != FieldTypeClass::Unknown {
                    // Java: int index = ByteArrayHelper.getShort(data, 0);
                    //       map.put(Integer.valueOf(index), fieldType);
                    map.insert(ByteArrayHelper::get_short(data, 0), field_type);
                }
            }
        }

        // Java: VarMeta outlineCodeVarMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) outlineCodeDir.getEntry("VarMeta"))));
        let Some(var_meta_document) = outline_code_dir.get_document("VarMeta") else {
            return Ok(());
        };
        let outline_code_var_meta: Rc<dyn VarMeta> =
            Rc::new(VarMeta9::new(&var_meta_document.get_data())?);

        // Java: Var2Data outlineCodeVarData = new Var2Data(m_file, outlineCodeVarMeta, new DocumentInputStream(((DocumentEntry) outlineCodeDir.getEntry("Var2Data"))));
        let Some(var2_data_document) = outline_code_dir.get_document("Var2Data") else {
            return Ok(());
        };
        let outline_code_var_data = Var2Data::new(
            &self.file,
            Rc::clone(&outline_code_var_meta),
            &var2_data_document.get_data(),
        );

        // Java: Map<FieldType, List<Pair<String, String>>> valueMap = new HashMap<>();
        // Rust 侧：Java 允许 null 键（`map.get(id)` 未命中的情形），故键用 `Option<MppFieldKey>`；
        // 同时把字段本身存进值里，供 `populate_container_from_outline_code` 取名字。
        let mut value_map: HashMap<
            Option<MppFieldKey>,
            (Option<MppFieldType>, Vec<(Option<String>, Option<String>)>),
        > = HashMap::new();

        // Java: for (Integer id : outlineCodeVarMeta.getUniqueIdentifierArray())
        for id in outline_code_var_meta.get_unique_identifier_array() {
            // Java: FieldType fieldType = map.get(id);
            let field_type = map.get(&id).cloned();

            // Java: String value = outlineCodeVarData.getUnicodeString(id, VALUE);
            //       String description = outlineCodeVarData.getUnicodeString(id, DESCRIPTION);
            let value = outline_code_var_data.get_unicode_string(Some(id), Some(VALUE));
            let description = outline_code_var_data.get_unicode_string(Some(id), Some(DESCRIPTION));

            // Java: List<Pair<String, String>> list = valueMap.computeIfAbsent(fieldType, k -> new ArrayList<>());
            //       list.add(new Pair<>(value, description));
            let key = field_type.as_ref().map(MppFieldType::to_key);
            let entry = value_map
                .entry(key)
                .or_insert_with(|| (field_type, Vec::new()));
            entry.1.push((value, description));
        }

        // Java: for (Entry<FieldType, List<Pair<String, String>>> entry : valueMap.entrySet()) { populateContainer(entry.getKey(), entry.getValue()); }
        for (_, (field_type, items)) in value_map {
            self.populate_container_from_outline_code(field_type.as_ref(), &items);
        }

        Ok(())
    }

    /// 用「值数组 + 描述数组」填充容器。
    ///
    /// 对应 Java: `private void populateContainer(FieldType field, byte[] values, byte[] descriptions)`
    fn populate_container(
        &mut self,
        field: Option<&MppFieldType>,
        values: &[u8],
        descriptions: &[u8],
    ) {
        // Java: CustomField config = m_container.getOrCreate(field);
        // Java 的 `field` 可能为 null（字段号未识别）：`HashMap` 允许 null 键，但紧随其后的
        // `config.getFieldType().getName()` 会抛 NPE。Rust 的 `FieldKey` 无「未知」变体，故跳过。
        let Some(field) = field else {
            return;
        };

        let custom_field = CustomFieldContainer::get_or_create(&self.container, field.to_field_key());

        // Java: String fieldTypeName = config.getFieldType().getName();
        let field_type_name = custom_field.borrow().get_field_type().get_name().to_owned();

        // Java: table.setGUID(UUID.nameUUIDFromBytes(fieldTypeName.getBytes()));
        //（Java 用平台默认编码取字节；字段名均为 ASCII，Rust 侧用 UTF-8 结果一致）
        custom_field
            .borrow_mut()
            .get_lookup_table_mut()
            .set_guid(Some(name_uuid_from_bytes(field_type_name.as_bytes())));

        // Java: List<Object> descriptionList = convertType(DataType.STRING, descriptions);
        let description_list = self.convert_type(DataType::String, descriptions);
        // Java: List<Object> valueList = convertType(field.getDataType(), values);
        let value_list = self.convert_type(field.get_data_type(), values);
        // Java: CustomFieldValueDataType itemType = getDataType(field);
        let item_type = Self::get_data_type(Some(field));

        // Java: for (int index = 0; index < descriptionList.size(); index++)
        for index in 0..description_list.len() {
            // Java: CustomFieldValueItem item = new CustomFieldValueItem(Integer.valueOf(++m_valueUniqueIDCounter));
            self.value_unique_id_counter += 1;
            let item = CustomFieldValueItem::new(self.value_unique_id_counter);

            // Java: item.setDescription((String) descriptionList.get(index));
            item.set_description(
                description_list[index]
                    .as_ref()
                    .and_then(|value| value.as_string().map(str::to_owned)),
            );
            // Java: item.setType(itemType);
            item.set_type(Some(item_type));
            // Java: item.setGUID(UUID.nameUUIDFromBytes((fieldTypeName + item.getUniqueID()).getBytes()));
            item.set_guid(Some(name_uuid_from_bytes(
                format!("{field_type_name}{}", item.get_unique_id()).as_bytes(),
            )));
            // Java: if (index < valueList.size()) { item.setValue(valueList.get(index)); }
            if index < value_list.len() {
                item.set_value(value_list[index].clone());
            }

            // Java: m_container.registerValue(item);
            self.container
                .borrow_mut()
                .register_value(item.stored_item());
            // Java: table.add(item);
            custom_field
                .borrow_mut()
                .get_lookup_table_mut()
                .add(item.to_stored_item());
        }
    }

    /// 用大纲代码数据填充容器。
    ///
    /// 对应 Java: `private void populateContainer(FieldType field, List<Pair<String, String>> items)`
    ///
    /// Rust 无重载，故该方法在 `populate_container` 之后加 `_from_outline_code` 后缀。
    /// 与 `populate_container` 不同：Java 这里只把取值项加进查找表，**不**登记到容器。
    fn populate_container_from_outline_code(
        &mut self,
        field: Option<&MppFieldType>,
        items: &[(Option<String>, Option<String>)],
    ) {
        // Java: CustomField config = m_container.getOrCreate(field);
        // `field` 为 null 时 Java 会用 "Unknown" 作为字段名，但仍需 `getOrCreate(null)`；
        // Rust 的 `FieldKey` 无法承载 null，故跳过（与 `populate_container` 同样的取舍）。
        let Some(field) = field else {
            return;
        };

        let custom_field = CustomFieldContainer::get_or_create(&self.container, field.to_field_key());

        // Java: String fieldTypeName = field == null ? "Unknown" : field.getName();
        //（`field` 为 None 的分支已在上面返回，故这里始终取字段名）
        let field_type_name = field.get_name();

        // Java: table.setGUID(UUID.nameUUIDFromBytes(fieldTypeName.getBytes()));
        custom_field
            .borrow_mut()
            .get_lookup_table_mut()
            .set_guid(Some(name_uuid_from_bytes(field_type_name.as_bytes())));

        // Java: CustomFieldValueDataType itemType = getDataType(field);
        let item_type = Self::get_data_type(Some(field));

        // Java: for (Pair<String, String> pair : items)
        for (value, description) in items {
            // Java: CustomFieldValueItem item = new CustomFieldValueItem(Integer.valueOf(++m_valueUniqueIDCounter));
            self.value_unique_id_counter += 1;
            let item = CustomFieldValueItem::new(self.value_unique_id_counter);

            // Java: item.setValue(pair.getFirst());
            item.set_value(value.clone().map(FieldValue::String));
            // Java: item.setDescription(pair.getSecond());
            item.set_description(description.clone());
            // Java: item.setType(itemType);
            item.set_type(Some(item_type));
            // Java: item.setGUID(UUID.nameUUIDFromBytes((fieldTypeName + item.getUniqueID()).getBytes()));
            item.set_guid(Some(name_uuid_from_bytes(
                format!("{field_type_name}{}", item.get_unique_id()).as_bytes(),
            )));

            // Java: table.add(item);
            custom_field
                .borrow_mut()
                .get_lookup_table_mut()
                .add(item.to_stored_item());
        }
    }

    /// 把一段原始数据按数据类型切分成取值列表。
    ///
    /// 对应 Java: `private List<Object> convertType(DataType type, byte[] data)`
    ///
    /// `Vec<Option<FieldValue>>` 对应 Java 的 `List<Object>`：元素可能为 `null`
    ///（`getTimestamp` / `getAdjustedDuration` 返回 null 时），用 `Option` 保持下标对齐。
    fn convert_type(&self, type_: DataType, data: &[u8]) -> Vec<Option<FieldValue>> {
        // Java: List<Object> result = new ArrayList<>(); int index = 0;
        let mut result = Vec::new();
        let mut index = 0usize;

        // Java: while (index < data.length)
        while index < data.len() {
            match type_ {
                // Java: case STRING: String value = MPPUtility.getUnicodeString(data, index);
                //       result.add(value); index += ((value.length() + 1) * 2);
                DataType::String => {
                    let value = MPPUtility::get_unicode_string(data, index);
                    // Java 的 `String.length()` 是 UTF-16 码元数
                    let units = value.encode_utf16().count() + 1;
                    result.push(Some(FieldValue::String(value)));
                    index = index.saturating_add(units * 2);
                }

                // Java: case CURRENCY: Double value = Double.valueOf(MPPUtility.getDouble(data, index) / 100);
                DataType::Currency => {
                    result.push(Some(FieldValue::Double(
                        MPPUtility::get_double(data, index) / 100.0,
                    )));
                    index = index.saturating_add(8);
                }

                // Java: case NUMERIC: Double value = Double.valueOf(MPPUtility.getDouble(data, index));
                DataType::Numeric => {
                    result.push(Some(FieldValue::Double(MPPUtility::get_double(data, index))));
                    index = index.saturating_add(8);
                }

                // Java: case DATE: LocalDateTime value = MPPUtility.getTimestamp(data, index);
                //       result.add(value); index += 4;
                DataType::Date => {
                    result.push(MPPUtility::get_timestamp(data, index).map(FieldValue::DateTime));
                    index = index.saturating_add(4);
                }

                // Java:
                // case DURATION:
                //    TimeUnit units = MPPUtility.getDurationTimeUnits(ByteArrayHelper.getShort(data, index + 4), m_properties.getDefaultDurationUnits());
                //    Duration value = MPPUtility.getAdjustedDuration(m_properties, ByteArrayHelper.getInt(data, index), units);
                //    result.add(value); index += 6;
                DataType::Duration => {
                    let properties = self.try_project_properties();
                    let default_duration_units = properties
                        .as_deref()
                        .map(ProjectProperties::get_default_duration_units);
                    // Java 越界时抛异常；Rust 侧取不到时按 0 处理（`getDurationTimeUnits(0)` = DAYS）
                    let units = MPPUtility::get_duration_time_units_with_default(
                        checked_short(data, index.saturating_add(4)).unwrap_or(0),
                        default_duration_units,
                    );
                    let duration = checked_int(data, index).unwrap_or(0);

                    result.push(
                        properties
                            .as_deref()
                            .and_then(|properties| {
                                MPPUtility::get_adjusted_duration(properties, duration, units)
                            })
                            .map(FieldValue::Duration),
                    );
                    index = index.saturating_add(6);
                }

                // Java: case BOOLEAN: Boolean value = Boolean.valueOf(ByteArrayHelper.getShort(data, index) == 1);
                //       result.add(value); index += 2;
                DataType::Boolean => {
                    result.push(Some(FieldValue::Boolean(
                        checked_short(data, index).unwrap_or(0) == 1,
                    )));
                    index = index.saturating_add(2);
                }

                // Java: default: index = data.length;（结束解析）
                _ => index = data.len(),
            }
        }

        result
    }

    /// 取自定义字段对应的取值项类型。
    ///
    /// 对应 Java: `private CustomFieldValueDataType getDataType(FieldType field)`
    ///
    /// Java 用静态 `Map<DataType, CustomFieldValueDataType> TYPE_MAP` + 「取不到则 TEXT」；
    /// Rust 侧用 `match` 等价表达（映射表是常量，无需运行期建表）。
    /// Java 的实例方法只用到静态表，故 Rust 侧实现为关联函数。
    fn get_data_type(field: Option<&MppFieldType>) -> CustomFieldValueDataType {
        // Java: if (field != null) { result = TYPE_MAP.get(field.getDataType()); }
        //       if (result == null) { result = CustomFieldValueDataType.TEXT; }
        match field.map(MppFieldType::get_data_type) {
            Some(DataType::String) => CustomFieldValueDataType::Text,
            Some(DataType::Currency) => CustomFieldValueDataType::Cost,
            Some(DataType::Numeric) => CustomFieldValueDataType::Number,
            Some(DataType::Date) => CustomFieldValueDataType::Date,
            Some(DataType::Duration) => CustomFieldValueDataType::Duration,
            Some(DataType::Boolean) => CustomFieldValueDataType::Flag,
            _ => CustomFieldValueDataType::Text,
        }
    }

    /// 借用项目属性（Rust 特有：Java 在构造方法里保存属性对象的引用）。
    fn try_project_properties(&self) -> Option<Ref<'_, ProjectProperties>> {
        let file = self.file.try_borrow().ok()?;
        Some(Ref::map(file, |file| file.get_project_properties()))
    }
}

/// 按 Java 语义复制一段数据。
///
/// 对应 Java: `new byte[size]` + `MPPUtility.getByteArray(data, offset, size, buffer, 0)`。
/// Java 在 `size` 为负时抛 `NegativeArraySizeException`、偏移越界时抛
/// `ArrayIndexOutOfBoundsException`；Rust 侧统一退化为「尽量复制，越界部分为 0」。
fn read_block(data: &[u8], offset: i64, size: i64) -> Vec<u8> {
    // Java: new byte[valuesLength]（负长度抛异常）
    let size = usize::try_from(size).unwrap_or(0);
    let mut result = vec![0u8; size];

    // Java: MPPUtility.getByteArray(data, dataOffset, valuesLength, values, 0);
    // （负偏移在 Java 会抛越界异常，Rust 侧退化为 0；`get_byte_array` 内部仍做边界检查）
    let offset = usize::try_from(offset.max(0)).unwrap_or(0);
    MPPUtility::get_byte_array(data, offset, size, &mut result, 0);

    result
}

/// 由任意字节生成「基于名字」的 UUID。
///
/// 对应 Java: `UUID.nameUUIDFromBytes(byte[] name)`
///
/// `java.util.UUID.nameUUIDFromBytes` 取 `MD5(name)` 的 16 字节，按 RFC 4122 改写版本位
/// （`0x30`）与变体位（`0x80`）后构造 UUID。`uuid` crate 的 `v3` feature 未启用（本 crate
/// 不允许新增依赖），故这里就地实现 MD5。
fn name_uuid_from_bytes(name: &[u8]) -> Uuid {
    let mut digest = md5(name);

    // Java: md5Bytes[6] &= 0x0f; md5Bytes[6] |= 0x30;
    //       md5Bytes[8] &= 0x3f; md5Bytes[8] |= 0x80;
    digest[6] = (digest[6] & 0x0F) | 0x30;
    digest[8] = (digest[8] & 0x3F) | 0x80;

    Uuid::from_bytes(digest)
}

/// MD5 每轮的左移位数。
///
/// 对应 RFC 1321 的 `s` 表。
const MD5_S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

/// MD5 每轮的加常量（`floor(2^32 * abs(sin(i + 1)))`）。
///
/// 对应 RFC 1321 的 `K` 表。
const MD5_K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

/// 计算 MD5 摘要（RFC 1321）。
///
/// Rust 特有：Java 侧该能力由 `java.security.MessageDigest` 提供。
fn md5(input: &[u8]) -> [u8; 16] {
    // 填充：追加 0x80，补零至 56 mod 64，再接 8 字节小端的比特长度
    let mut message = input.to_vec();
    let bit_length = (input.len() as u64).wrapping_mul(8);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_le_bytes());

    let mut a0: u32 = 0x6745_2301;
    let mut b0: u32 = 0xefcd_ab89;
    let mut c0: u32 = 0x98ba_dcfe;
    let mut d0: u32 = 0x1032_5476;

    for chunk in message.chunks_exact(64) {
        let mut words = [0u32; 16];
        for (index, word) in words.iter_mut().enumerate() {
            let start = index * 4;
            *word = u32::from_le_bytes([
                chunk[start],
                chunk[start + 1],
                chunk[start + 2],
                chunk[start + 3],
            ]);
        }

        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for index in 0..64 {
            let (f, g) = match index / 16 {
                0 => ((b & c) | (!b & d), index),
                1 => ((d & b) | (!d & c), (5 * index + 1) % 16),
                2 => (b ^ c ^ d, (3 * index + 5) % 16),
                _ => (c ^ (b | !d), (7 * index) % 16),
            };

            let temp = d;
            d = c;
            c = b;
            b = b.wrapping_add(
                a.wrapping_add(f)
                    .wrapping_add(MD5_K[index])
                    .wrapping_add(words[g])
                    .rotate_left(MD5_S[index]),
            );
            a = temp;
        }

        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }

    let mut digest = [0u8; 16];
    digest[0..4].copy_from_slice(&a0.to_le_bytes());
    digest[4..8].copy_from_slice(&b0.to_le_bytes());
    digest[8..12].copy_from_slice(&c0.to_le_bytes());
    digest[12..16].copy_from_slice(&d0.to_le_bytes());
    digest
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use cfb::CompoundFile;

    use super::*;
    use crate::common::mpp_task_field::MPPTaskField;
    use crate::field_key::FieldKey;
    use crate::mpp::custom_field_value_reader::test_support::{data_block, var_meta9_bytes};
    use crate::mpp::ole::PoifsFileSystem;
    use crate::task_field::TaskField;

    /// 内置任务字段 NAME 的字段号。
    fn name_field_id() -> i32 {
        MPPTaskField::TASK_FIELD_BASE | 14
    }

    /// 内置任务字段 NAME。
    fn name_field() -> MppFieldType {
        MppFieldType::Builtin(FieldKey::Task(TaskField::Name))
    }

    /// UTF-16LE（NUL 结尾）字节。
    fn utf16(text: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        bytes.extend_from_slice(&[0x00, 0x00]);
        bytes
    }

    /// 构造一个把大纲代码流写入 `TBkndOutlCode` 存储的复合文档。
    ///
    /// `outline_code_records` 是定长数据块的记录（每条 20 字节：[0..2) = 索引，
    /// [12..16) = 字段号）；`var_data` 是变长数据块的内容。
    fn compound_file(outline_code_records: &[Vec<u8>], var_data: &[u8], var_meta: &[u8]) -> Vec<u8> {
        // FixedMeta：10 字节记录，偏移取自各条记录在数据体中的位置
        let mut fixed_meta = Vec::new();
        fixed_meta.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        fixed_meta.extend_from_slice(&0i32.to_le_bytes());
        fixed_meta.extend_from_slice(&(outline_code_records.len() as i32).to_le_bytes());
        fixed_meta.extend_from_slice(&0i32.to_le_bytes());

        let mut body = Vec::new();
        for record in outline_code_records {
            let mut meta_record = vec![0u8; 10];
            meta_record[4..8].copy_from_slice(&(body.len() as i32).to_le_bytes());
            fixed_meta.extend_from_slice(&meta_record);
            body.extend_from_slice(record);
        }

        let mut file = CompoundFile::create(Cursor::new(Vec::new())).expect("应能创建复合文档");
        file.create_storage("/TBkndOutlCode").expect("创建存储");
        file.create_stream("/TBkndOutlCode/FixedMeta")
            .expect("创建流")
            .write_all(&fixed_meta)
            .expect("写入 FixedMeta");
        file.create_stream("/TBkndOutlCode/FixedData")
            .expect("创建流")
            .write_all(&body)
            .expect("写入 FixedData");
        file.create_stream("/TBkndOutlCode/VarMeta")
            .expect("创建流")
            .write_all(var_meta)
            .expect("写入 VarMeta");
        file.create_stream("/TBkndOutlCode/Var2Data")
            .expect("创建流")
            .write_all(var_data)
            .expect("写入 Var2Data");

        file.into_inner().into_inner()
    }

    /// 打开复合文档并取根存储。
    ///
    /// 传入空切片时改用一个不含任何流的复合文档（对应「项目存储里没有 `TBkndOutlCode`」）。
    fn root_directory(bytes: Vec<u8>) -> DirectoryEntry {
        let bytes = if bytes.is_empty() {
            empty_compound_file()
        } else {
            bytes
        };

        PoifsFileSystem::new(Cursor::new(bytes))
            .expect("复合文档可解析")
            .get_root()
    }

    /// 构造一个不含任何流的复合文档（对应「项目存储里没有 `TBkndOutlCode`」）。
    fn empty_compound_file() -> Vec<u8> {
        CompoundFile::create(Cursor::new(Vec::new()))
            .expect("应能创建复合文档")
            .into_inner()
            .into_inner()
    }

    /// 大纲代码定长数据块的一条记录（20 字节）。
    fn outline_code_record(index: i32, field_id: i32) -> Vec<u8> {
        let mut record = vec![0u8; 20];
        record[0..2].copy_from_slice(&(index as u16).to_le_bytes());
        record[12..16].copy_from_slice(&field_id.to_le_bytes());
        record
    }

    /// MD5 与标准测试向量一致（RFC 1321 附录 A.5）。
    #[test]
    fn md5_matches_known_vectors() {
        assert_eq!(
            md5(b""),
            [
                0xd4, 0x1d, 0x8c, 0xd9, 0x8f, 0x00, 0xb2, 0x04, 0xe9, 0x80, 0x09, 0x98, 0xec, 0xf8,
                0x42, 0x7e
            ]
        );
        assert_eq!(
            md5(b"abc"),
            [
                0x90, 0x01, 0x50, 0x98, 0x3c, 0xd2, 0x4f, 0xb0, 0xd6, 0x96, 0x3f, 0x7d, 0x28, 0xe1,
                0x7f, 0x72
            ]
        );
    }

    /// `nameUUIDFromBytes` 与 Java 的「版本 3」UUID 一致。
    #[test]
    fn name_uuid_matches_java() {
        // Java: UUID.nameUUIDFromBytes("test".getBytes()) →
        //       098f6bcd-4621-3373-8ade-4e832627b4f6
        assert_eq!(
            name_uuid_from_bytes(b"test"),
            Uuid::parse_str("098f6bcd-4621-3373-8ade-4e832627b4f6").expect("合法 GUID")
        );
        // 版本位为 3、变体位为 RFC 4122
        let bytes = name_uuid_from_bytes(b"test").into_bytes();
        assert_eq!(bytes[6] >> 4, 3);
        assert_eq!(bytes[8] >> 6, 0b10);
    }

    /// `Props.TASK_FIELD_ATTRIBUTES` 里的取值列表被读成取值项（描述 / 取值 / 类型 / GUID）。
    #[test]
    fn process_reads_task_field_attributes() {
        // 数据块布局：
        //   [0..4)   总长度
        //   [4..8)   取值列表个数
        //   [8..12)  字段号  [12..16) 取值列表偏移（绝对）
        //   [32..40) 保留  [40..44) 数据偏移（相对）  [44..48) 数据结束  [48..52) 描述结束
        let mut block = vec![0u8; 66];
        block[0..4].copy_from_slice(&66i32.to_le_bytes());
        block[4..8].copy_from_slice(&1i32.to_le_bytes());
        block[8..12].copy_from_slice(&name_field_id().to_le_bytes());
        block[12..16].copy_from_slice(&32i32.to_le_bytes());
        // 值为 "AB"，描述为 "X"
        block[32 + 24..32 + 30].copy_from_slice(&utf16("AB"));
        block[32 + 30..32 + 34].copy_from_slice(&utf16("X"));
        block[32 + 8..32 + 12].copy_from_slice(&24i32.to_le_bytes());
        block[32 + 12..32 + 16].copy_from_slice(&30i32.to_le_bytes());
        block[32 + 16..32 + 20].copy_from_slice(&34i32.to_le_bytes());

        let mut props = Props::new();
        props.put(PropsKey::TaskFieldAttributes.get_value(), block);

        let file = ProjectFile::new();
        // 根存储下没有 TBkndOutlCode：大纲代码分支返回 Ok(())
        let the_root = root_directory(Vec::new());
        let mut reader = CustomFieldValueReader9::new(&the_root, &file, &props);
        reader.process().expect("解析不应失败");

        let container = file.borrow().get_custom_fields();
        let item = container
            .borrow()
            .get_custom_field_value_item_by_unique_id(1)
            .expect("取值项应已登记");
        let item = item.borrow();
        assert_eq!(item.get_description(), Some("X"));
        assert_eq!(item.get_value(), Some(&FieldValue::String("AB".to_owned())));
        assert_eq!(item.get_type(), Some(CustomFieldValueDataType::Text));
        // GUID = nameUUIDFromBytes(字段名 + uniqueID)
        let field_name = name_field().get_name();
        assert_eq!(
            item.get_guid(),
            Some(name_uuid_from_bytes(
                format!("{field_name}1").as_bytes()
            ))
        );

        // 查找表：GUID = nameUUIDFromBytes(字段名)
        let custom_field = container
            .borrow()
            .get(FieldKey::Task(TaskField::Name))
            .expect("字段配置应已创建");
        let custom_field = custom_field.borrow();
        assert_eq!(
            custom_field.get_lookup_table().get_guid(),
            Some(name_uuid_from_bytes(field_name.as_bytes()))
        );
        assert_eq!(custom_field.get_lookup_table().len(), 1);
    }

    /// `TASK_FIELD_ATTRIBUTES` 为空 / 截断时不 panic。
    #[test]
    fn process_handles_missing_task_field_attributes() {
        let file = ProjectFile::new();
        let the_root = root_directory(Vec::new());

        // 完全没有该属性
        let mut reader = CustomFieldValueReader9::new(&the_root, &file, &Props::new());
        reader.process().expect("解析不应失败");
        assert!(file.borrow().get_custom_fields().borrow().is_empty());

        // 只有 4 字节（读不出取值列表个数）
        let mut props = Props::new();
        props.put(PropsKey::TaskFieldAttributes.get_value(), vec![0u8; 4]);
        let mut reader = CustomFieldValueReader9::new(&the_root, &file, &props);
        reader.process().expect("解析不应失败");

        // 字段号指向未识别的值（FieldTypeHelper 返回 None）
        let mut block = vec![0u8; 20];
        block[0..4].copy_from_slice(&20i32.to_le_bytes());
        block[4..8].copy_from_slice(&1i32.to_le_bytes());
        block[8..12].copy_from_slice(&0x1234_0005i32.to_le_bytes());
        block[12..16].copy_from_slice(&16i32.to_le_bytes());
        let mut props = Props::new();
        props.put(PropsKey::TaskFieldAttributes.get_value(), block);
        let mut reader = CustomFieldValueReader9::new(&the_root, &file, &props);
        reader.process().expect("解析不应失败");
    }

    /// 大纲代码：字段号映射 + 变长数据块里的取值 / 描述被写进查找表。
    #[test]
    fn process_reads_outline_code_values() {
        // 变长数据块：id=1 与 id=2 各有取值（类型 1）与描述（类型 2）
        let items = vec![
            utf16("A"),
            utf16("desc A"),
            utf16("B"),
            utf16("desc B"),
        ];
        let (var_data, offsets) = data_block(&items);
        let var_meta = var_meta9_bytes(
            &[(1, VALUE, 0), (1, DESCRIPTION, 1), (2, VALUE, 2), (2, DESCRIPTION, 3)],
            &offsets,
            var_data.len() as i32,
        );

        // 三条定长记录：前两条映射到 NAME，第三条字段号无法识别
        let records = vec![
            outline_code_record(1, name_field_id()),
            outline_code_record(2, name_field_id()),
            outline_code_record(3, 0x1234_0005),
        ];
        let the_root = root_directory(compound_file(&records, &var_data, &var_meta));

        let file = ProjectFile::new();
        let mut reader = CustomFieldValueReader9::new(&the_root, &file, &Props::new());
        reader.process().expect("解析不应失败");

        let container = file.borrow().get_custom_fields();
        let custom_field = container
            .borrow()
            .get(FieldKey::Task(TaskField::Name))
            .expect("字段配置应已创建");
        let custom_field = custom_field.borrow();
        let table = custom_field.get_lookup_table();

        // 无法识别的字段号（第三条）不产生取值项
        assert_eq!(table.len(), 2);
        assert_eq!(table.get(0).and_then(|item| item.get_value()), Some(&FieldValue::String("A".to_owned())));
        assert_eq!(table.get(0).and_then(|item| item.get_description()), Some("desc A"));
        assert_eq!(table.get(1).and_then(|item| item.get_value()), Some(&FieldValue::String("B".to_owned())));
        assert_eq!(table.get(1).and_then(|item| item.get_description()), Some("desc B"));
        // unique ID 由计数器递增产生（Java 的 `++m_valueUniqueIDCounter`）
        assert_eq!(table.get(0).map(|item| item.get_unique_id()), Some(1));
        assert_eq!(table.get(1).map(|item| item.get_unique_id()), Some(2));
        // 取值类型来自字段的 DataType（STRING → TEXT）
        assert_eq!(table.get(0).and_then(|item| item.get_type()), Some(CustomFieldValueDataType::Text));
    }

    /// 大纲代码流缺失 / 畸形时不 panic。
    #[test]
    fn process_handles_broken_outline_code_streams() {
        let file = ProjectFile::new();

        // 存储存在但四个流都不存在
        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).expect("创建复合文档");
        compound.create_storage("/TBkndOutlCode").expect("创建存储");
        let the_root = root_directory(compound.into_inner().into_inner());
        let mut reader = CustomFieldValueReader9::new(&the_root, &file, &Props::new());
        reader.process().expect("解析不应失败");

        // FixedMeta 内容非法（magic 不对）→ 返回 Err
        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).expect("创建复合文档");
        compound.create_storage("/TBkndOutlCode").expect("创建存储");
        compound
            .create_stream("/TBkndOutlCode/FixedMeta")
            .expect("创建流")
            .write_all(&[0u8; 32])
            .expect("写入");
        compound
            .create_stream("/TBkndOutlCode/FixedData")
            .expect("创建流")
            .write_all(&[0u8; 32])
            .expect("写入");
        let the_root = root_directory(compound.into_inner().into_inner());
        let mut reader = CustomFieldValueReader9::new(&the_root, &file, &Props::new());
        assert!(reader.process().is_err());

        // FixedData 比元数据声明的短：读取时跳过，不 panic
        let var_meta = var_meta9_bytes(&[], &[], 0);
        let the_root = root_directory(compound_file(&[outline_code_record(1, name_field_id())], &[], &var_meta));
        let mut reader = CustomFieldValueReader9::new(&the_root, &file, &Props::new());
        reader.process().expect("解析不应失败");
    }

    /// 取值类型映射（`getDataType` 的静态映射表 + 兜底 TEXT）。
    #[test]
    fn data_type_mapping_matches_java() {
        let cases = [
            (DataType::String, CustomFieldValueDataType::Text),
            (DataType::Currency, CustomFieldValueDataType::Cost),
            (DataType::Numeric, CustomFieldValueDataType::Number),
            (DataType::Date, CustomFieldValueDataType::Date),
            (DataType::Duration, CustomFieldValueDataType::Duration),
            (DataType::Boolean, CustomFieldValueDataType::Flag),
            // 未在表中的类型 → TEXT
            (DataType::Custom, CustomFieldValueDataType::Text),
        ];

        for (data_type, expected) in cases {
            let key = FieldKey::Custom(crate::field_key::CustomFieldKey::new(
                1,
                FieldTypeClass::Task,
                data_type,
            ));
            let field = MppFieldType::Builtin(key);
            assert_eq!(
                CustomFieldValueReader9::get_data_type(Some(&field)),
                expected
            );
        }

        // Java: field == null → TEXT
        assert_eq!(
            CustomFieldValueReader9::get_data_type(None),
            CustomFieldValueDataType::Text
        );
    }

    /// `convertType` 的各分支（`Vec<Option<FieldValue>>` 保持 Java 的下标对齐）。
    #[test]
    fn convert_type_matches_java() {
        let file = ProjectFile::new();
        let the_root = root_directory(Vec::new());
        let reader = CustomFieldValueReader9::new(&the_root, &file, &Props::new());

        // STRING：UTF-16LE + NUL，按 (length + 1) * 2 前进
        let values = reader.convert_type(DataType::String, &[0x41, 0x00, 0x42, 0x00, 0x00, 0x00, 0x43, 0x00, 0x00, 0x00]);
        assert_eq!(
            values,
            vec![
                Some(FieldValue::String("AB".to_owned())),
                Some(FieldValue::String("C".to_owned()))
            ]
        );

        // CURRENCY / NUMERIC：8 字节双精度
        assert_eq!(
            reader.convert_type(DataType::Currency, &250.0f64.to_le_bytes()),
            vec![Some(FieldValue::Double(2.5))]
        );
        assert_eq!(
            reader.convert_type(DataType::Numeric, &1.5f64.to_le_bytes()),
            vec![Some(FieldValue::Double(1.5))]
        );

        // DATE：4 字节时间戳（天数非法时为 None，但下标仍前进一步）
        assert_eq!(
            reader.convert_type(DataType::Date, &[0x00, 0x00, 0x02, 0x00]),
            vec![Some(FieldValue::DateTime(
                chrono::NaiveDate::from_ymd_opt(1984, 1, 2)
                    .expect("合法日期")
                    .and_hms_opt(0, 0, 0)
                    .expect("合法时间")
            ))]
        );

        // DURATION：4 字节工期 + 2 字节单位
        let mut duration = 0i32.to_le_bytes().to_vec();
        duration.extend_from_slice(&7u16.to_le_bytes());
        let values = reader.convert_type(DataType::Duration, &duration);
        assert_eq!(values.len(), 1);
        assert!(matches!(values[0], Some(FieldValue::Duration(_))));

        // BOOLEAN：短整数 == 1
        assert_eq!(
            reader.convert_type(DataType::Boolean, &[0x01, 0x00, 0x00, 0x00]),
            vec![
                Some(FieldValue::Boolean(true)),
                Some(FieldValue::Boolean(false))
            ]
        );

        // 未支持的类型：直接结束解析
        assert!(reader.convert_type(DataType::Guid, &[0u8; 16]).is_empty());

        // 空数据
        assert!(reader.convert_type(DataType::String, &[]).is_empty());
    }
}
