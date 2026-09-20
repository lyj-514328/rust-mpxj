//! 对应 Java: `org/mpxj/mpp/CustomFieldValueReader.java`
//!
//! 自定义字段**取值列表**读取的公共实现：从 `TBkndOutlCode` 的定长 / 变长数据块中读出
//! 取值项（unique ID、GUID、描述、未知数据块、父级 ID、取值类型、取值），按 unique ID 与
//! GUID 登记到自定义字段容器，并按查找表 GUID 挂到对应字段的查找表上。
//!
//! # 继承的还原（组合 + 转发）
//!
//! Java: `abstract class CustomFieldValueReader`，由 `CustomFieldValueReader12` /
//! `CustomFieldValueReader14` 继承（注意：`CustomFieldValueReader9` **不**继承本类，它是
//! 独立实现，另有自己的取值列表解析逻辑）。Rust 无继承，故：
//!
//! - 本类在 Rust 中是**具体结构体**：Java 的 `abstract` 只用于禁止直接实例化，本类没有任何
//!   抽象方法，Rust 侧不引入多余的类型；
//! - [`CustomFieldValueReader12`](crate::mpp::custom_field_value_reader12::CustomFieldValueReader12)
//!   与 `CustomFieldValueReader14` 各自**持有一个本结构体**（私有字段 `base`），并把全部公开
//!   方法逐一转发；构造时通过 [`CustomFieldValueReader::set_parent_offset`] /
//!   [`CustomFieldValueReader::set_type_offset`] /
//!   [`CustomFieldValueReader::set_field_offset`] 写入 Java 子类直接赋值的 `protected` 字段。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `CustomFieldValueReader(ProjectFile, Map<UUID, FieldType>, VarMeta, Var2Data, FixedData, FixedData)` | 同序构造，但第一个参数改为 `&Rc<RefCell<ProjectFile>>`，`Map<UUID, FieldType>` → `HashMap<Uuid, MppFieldType>` | `ProjectFile` 在本 crate 中以 `Rc<RefCell<..>>` 共享；`FieldType` 接口用 [`MppFieldType`] 表达 |
//! | `private final ProjectProperties m_properties`（构造时从 file 取） | 不保存引用，按需用 `ProjectFile::try_borrow` 借用 | `ProjectProperties` 由 `ProjectFile` 持有，Rust 无法取出稳定引用（与 `FieldMap` 的既有做法一致）；借用冲突时按「取不到属性」处理，绝不 panic |
//! | `private final CustomFieldContainer m_container` | `Rc<RefCell<CustomFieldContainer>>` | 与 Java 一样在构造时取 `file.getCustomFields()` |
//! | `protected int m_parentOffset / m_typeOffset / m_fieldOffset` | 私有字段 + `set_*` / `get_*` 访问器（`usize`） | Java 子类直接给 `protected` 字段赋值；Rust 子类在别的文件里，故用访问器。偏移语义是非负字节下标，用 `usize` 表达 |
//! | `protected Object getTypedValue(CustomFieldValueDataType, byte[])` | `pub fn get_typed_value(&self, Option<CustomFieldValueDataType>, &[u8]) -> Option<FieldValue>` | `Object` → [`FieldValue`]；`null` → `None`。取值类型为 `null`（Java 的 `getInstance` 未命中）时与 Java 一样按字符串猜测 |
//! | `private String valueAsString(byte[])` | 私有方法 `value_as_string`（返回 [`FieldValue::String`]） | Java 用 `try/catch` 兜底成 hexdump；Rust 的 `MPPUtility::get_unicode_string` 是全函数（畸形数据也返回字符串），该兜底分支不可达，见方法注释 |
//! | `ByteArrayHelper.getShort/getInt` 越界抛 `ArrayIndexOutOfBoundsException` | [`checked_short`] / [`checked_int`] 返回 `None` 并 `continue`（跳过该条记录） | 遵循「不 panic」约定 |
//! | `m_container.registerValue(item)` / `table.add(item)` / `table.set(index, item)` | `register_value(item.stored_item())` / `table.add(item.to_stored_item())` / `table.set(index, item.to_stored_item())` | Java 在容器与查找表里放**同一个实例**；Rust 的查找表按值存放（元素类型不是 `Rc`），故查找表侧是内容相同的副本，容器侧与本次构造的实例共享同一份存储（见 [`CustomFieldValueItem`]） |
//! | `table.stream().noneMatch(...)` | `table.as_slice().iter().all(...)` | Java 的 `Stream` 未移植（本 crate 用切片迭代替代） |
//!
//! 跳过的 Java 方法：无。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。
//!
//! # 已知功能缺口
//!
//! `m_lookupTableMap` 的键是查找表 GUID，值在 Java 中可能是 `null`（`populateLookupTableMap`
//! 在 `FieldTypeHelper` 无法识别字段号时会放入 `null`）。Rust 的 `HashMap` 值类型不可空，
//! 故调用方只放入能识别的字段；查不到时本类与 Java 一样跳过该取值项的查找表处理。

use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::mpp_field_type::MppFieldType;
use crate::custom_field_container::CustomFieldContainer;
use crate::custom_field_value_data_type::CustomFieldValueDataType;
use crate::field_value::FieldValue;
use crate::mpp::custom_field_value_item::CustomFieldValueItem;
use crate::mpp::fixed_data::FixedData;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_meta::VarMeta;
use crate::project_file::ProjectFile;
use crate::project_properties::ProjectProperties;

/// 取值项 unique ID 在定长数据块中的偏移。
///
/// 对应 Java: `public static final int UNJQUE_ID_OFFSET = 4;`
///（常量名沿用了 Java 源码里的拼写错误 `UNJQUE`）。
pub const UNJQUE_ID_OFFSET: usize = 4;

/// 取值项「取值」在变长数据块中的数据类型键。
///
/// 对应 Java: `public static final Integer VALUE_LIST_VALUE = Integer.valueOf(22);`
pub const VALUE_LIST_VALUE: i32 = 22;

/// 取值项「描述」在变长数据块中的数据类型键。
///
/// 对应 Java: `public static final Integer VALUE_LIST_DESCRIPTION = Integer.valueOf(8);`
pub const VALUE_LIST_DESCRIPTION: i32 = 8;

/// 取值项「未知数据块」在变长数据块中的数据类型键。
///
/// 对应 Java: `public static final Integer VALUE_LIST_UNKNOWN = Integer.valueOf(23);`
pub const VALUE_LIST_UNKNOWN: i32 = 23;

/// 自定义字段取值列表的公共读取逻辑。
///
/// 对应 Java: `abstract class CustomFieldValueReader`
pub struct CustomFieldValueReader {
    /// 项目文件。
    ///
    /// 对应 Java 构造方法中的 `m_properties = file.getProjectProperties()`
    ///（Rust 侧改为按需借用，见类型说明）。
    file: Rc<RefCell<ProjectFile>>,

    /// 自定义字段容器。
    ///
    /// 对应 Java 字段: `protected final CustomFieldContainer m_container;`
    ///（构造方法中 `m_container = file.getCustomFields();`）
    container: Rc<RefCell<CustomFieldContainer>>,

    /// 查找表 GUID → 字段。
    ///
    /// 对应 Java 字段: `protected final Map<UUID, FieldType> m_lookupTableMap;`
    lookup_table_map: HashMap<Uuid, MppFieldType>,

    /// 取值列表的变长数据块元数据。
    ///
    /// 对应 Java 字段: `protected final VarMeta m_outlineCodeVarMeta;`
    outline_code_var_meta: Rc<dyn VarMeta>,

    /// 取值列表的变长数据块。
    ///
    /// 对应 Java 字段: `protected final Var2Data m_outlineCodeVarData;`
    outline_code_var_data: Var2Data,

    /// 取值列表的定长数据块。
    ///
    /// 对应 Java 字段: `protected final FixedData m_outlineCodeFixedData;`
    outline_code_fixed_data: FixedData,

    /// 取值列表的第二个定长数据块（GUID 与取值类型）。
    ///
    /// 对应 Java 字段: `protected final FixedData m_outlineCodeFixedData2;`
    outline_code_fixed_data2: FixedData,

    /// 父级 unique ID 在定长数据块中的偏移。
    ///
    /// 对应 Java 字段: `protected int m_parentOffset;`（由子类构造方法赋值）
    parent_offset: usize,

    /// 取值类型在第二个定长数据块中的偏移。
    ///
    /// 对应 Java 字段: `protected int m_typeOffset;`（由子类构造方法赋值）
    type_offset: usize,

    /// 查找表 GUID 在第二个定长数据块中的偏移。
    ///
    /// 对应 Java 字段: `protected int m_fieldOffset;`（由子类构造方法赋值）
    field_offset: usize,
}

impl CustomFieldValueReader {
    /// 构造方法。
    ///
    /// 对应 Java: `public CustomFieldValueReader(ProjectFile file, Map<UUID, FieldType> lookupTableMap, VarMeta outlineCodeVarMeta, Var2Data outlineCodeVarData, FixedData outlineCodeFixedData, FixedData outlineCodeFixedData2)`
    ///
    /// 三个偏移（`m_parentOffset` / `m_typeOffset` / `m_fieldOffset`）在 Java 中由子类构造方法
    /// 直接赋值，Rust 侧构造后为 0，由子类（或调用方）用 `set_*_offset` 设置。
    pub fn new(
        file: &Rc<RefCell<ProjectFile>>,
        lookup_table_map: HashMap<Uuid, MppFieldType>,
        outline_code_var_meta: Rc<dyn VarMeta>,
        outline_code_var_data: Var2Data,
        outline_code_fixed_data: FixedData,
        outline_code_fixed_data2: FixedData,
    ) -> Self {
        // Java: m_container = file.getCustomFields();
        // 文件正被可变借用时（正常调用路径不会发生）退化为一枚独立容器，避免 panic。
        let container = match file.try_borrow() {
            Ok(file) => file.get_custom_fields(),
            Err(_) => CustomFieldContainer::new(),
        };

        Self {
            file: Rc::clone(file),
            container,
            lookup_table_map,
            outline_code_var_meta,
            outline_code_var_data,
            outline_code_fixed_data,
            outline_code_fixed_data2,
            parent_offset: 0,
            type_offset: 0,
            field_offset: 0,
        }
    }

    /// 读取全部取值项。
    ///
    /// 对应 Java: `public void process()`
    pub fn process(&self) {
        // Java: for (int loop = 0; loop < m_outlineCodeFixedData.getItemCount(); loop++)
        for loop_index in 0..self.outline_code_fixed_data.get_item_count() {
            // Java: byte[] fixedData = m_outlineCodeFixedData.getByteArrayValue(loop);
            //       if (fixedData == null) { continue; }
            let Some(fixed_data) = self
                .outline_code_fixed_data
                .get_byte_array_value(loop_index)
            else {
                continue;
            };

            // Java: Integer id = Integer.valueOf(ByteArrayHelper.getShort(fixedData, UNJQUE_ID_OFFSET));
            // Java 在数据不足时抛 ArrayIndexOutOfBoundsException；Rust 侧跳过该条记录。
            let Some(id) = checked_short(fixed_data, UNJQUE_ID_OFFSET) else {
                continue;
            };

            // Java: if (!m_outlineCodeVarMeta.containsKey(id)) { continue; }
            if !self.outline_code_var_meta.contains_key(Some(id)) {
                continue;
            }

            // Java: byte[] value = m_outlineCodeVarData.getByteArray(id, VALUE_LIST_VALUE);
            //       if (value == null) { continue; }
            let Some(value) = self
                .outline_code_var_data
                .get_byte_array(Some(id), Some(VALUE_LIST_VALUE))
            else {
                continue;
            };

            // Java: CustomFieldValueItem item = new CustomFieldValueItem(id);
            let item = CustomFieldValueItem::new(id);

            // Java: item.setDescription(m_outlineCodeVarData.getUnicodeString(id, VALUE_LIST_DESCRIPTION));
            item.set_description(
                self.outline_code_var_data
                    .get_unicode_string(Some(id), Some(VALUE_LIST_DESCRIPTION)),
            );

            // Java: item.setUnknown(m_outlineCodeVarData.getByteArray(id, VALUE_LIST_UNKNOWN));
            item.set_unknown(
                self.outline_code_var_data
                    .get_byte_array(Some(id), Some(VALUE_LIST_UNKNOWN)),
            );

            // Java: item.setParentUniqueID(Integer.valueOf(ByteArrayHelper.getShort(fixedData, m_parentOffset)));
            // Java 越界时抛异常；Rust 侧取不到时保持 null（`None`）。
            item.set_parent_unique_id(checked_short(fixed_data, self.parent_offset));

            // Java: byte[] fixedData2 = m_outlineCodeFixedData2.getByteArrayValue(loop);
            //       item.setGUID(MPPUtility.getGUID(fixedData2, 0));
            // Java 的 `getGUID` 自身对 null / 过短数据返回 null，故这里用空切片表达缺失。
            let fixed_data2 = self
                .outline_code_fixed_data2
                .get_byte_array_value(loop_index)
                .unwrap_or(&[]);
            item.set_guid(MPPUtility::get_guid(fixed_data2, 0));

            // Java: UUID lookupTableGuid = MPPUtility.getGUID(fixedData2, m_fieldOffset);
            let lookup_table_guid = MPPUtility::get_guid(fixed_data2, self.field_offset);

            // Java: item.setType(CustomFieldValueDataType.getInstance(ByteArrayHelper.getShort(fixedData2, m_typeOffset)));
            // `getInstance` 对未定义的取值下标返回 null，与 Rust 的 `and_then` 等价。
            item.set_type(
                checked_short(fixed_data2, self.type_offset)
                    .and_then(CustomFieldValueDataType::get_instance),
            );

            // Java: item.setValue(getTypedValue(item.getType(), value));
            item.set_value(self.get_typed_value(item.get_type(), &value));

            // Java: m_container.registerValue(item);
            self.container
                .borrow_mut()
                .register_value(item.stored_item());

            // Java: FieldType field = m_lookupTableMap.get(lookupTableGuid);
            //       if (field != null) { ... }
            // Java 的 `getGUID` 可能返回 null，而映射中不会有 null 键，Rust 侧同样跳过。
            let field = lookup_table_guid
                .and_then(|guid| self.lookup_table_map.get(&guid))
                .cloned();

            if let Some(field) = field {
                // Java: CustomFieldLookupTable table = m_container.getOrCreate(field).getLookupTable();
                let custom_field =
                    CustomFieldContainer::get_or_create(&self.container, field.to_field_key());
                let unique_id = item.get_unique_id();

                // Java 注释：检查是否重复，unique ID 足以判定重复。
                // Java: if (table.stream().noneMatch(i -> i.getUniqueID().equals(item.getUniqueID())))
                let is_new = custom_field
                    .borrow()
                    .get_lookup_table()
                    .as_slice()
                    .iter()
                    .all(|existing| existing.get_unique_id() != unique_id);

                if is_new {
                    // Java: table.add(item); table.setGUID(lookupTableGuid);
                    let mut custom_field = custom_field.borrow_mut();
                    let table = custom_field.get_lookup_table_mut();
                    table.add(item.to_stored_item());
                    table.set_guid(lookup_table_guid);
                } else {
                    // Java 注释：用新实例替换查找表中的旧实例，保证容器与查找表指向同一项。
                    // Rust 侧查找表按值存放，故写入内容相同的副本。
                    let index = custom_field
                        .borrow()
                        .get_lookup_table()
                        .as_slice()
                        .iter()
                        .position(|existing| existing.get_unique_id() == unique_id);

                    if let Some(index) = index {
                        custom_field
                            .borrow_mut()
                            .get_lookup_table_mut()
                            .set(index, item.to_stored_item());
                    }
                }
            }
        }
    }

    /// 把从 MPP 文件读到的原始字节转换成 Java 类型的取值。
    ///
    /// 对应 Java: `protected Object getTypedValue(CustomFieldValueDataType type, byte[] value)`
    ///
    /// Java 的 `default` 分支覆盖「未列举的取值类型」与「`type == null`」两种情况，Rust 侧
    /// `FLAG` 与 `None` 走同一分支。Java 的 `getTimestamp` / `getAdjustedDuration` 可能返回
    /// `null`，故返回 `Option<FieldValue>`。
    pub fn get_typed_value(
        &self,
        type_: Option<CustomFieldValueDataType>,
        value: &[u8],
    ) -> Option<FieldValue> {
        // Java: if (type == null) { result = valueAsString(value); }
        let Some(type_) = type_ else {
            return Some(self.value_as_string(value));
        };

        match type_ {
            // Java: case DATE: case FINISH_DATE: result = MPPUtility.getTimestamp(value, 0);
            CustomFieldValueDataType::Date | CustomFieldValueDataType::FinishDate => {
                MPPUtility::get_timestamp(value, 0).map(FieldValue::DateTime)
            }

            // Java:
            // case DURATION:
            //    TimeUnit units = MPPUtility.getDurationTimeUnits(ByteArrayHelper.getShort(value, 4), m_properties.getDefaultDurationUnits());
            //    result = MPPUtility.getAdjustedDuration(m_properties, ByteArrayHelper.getInt(value, 0), units);
            CustomFieldValueDataType::Duration => {
                // Java 直接读 `m_properties`；Rust 侧按需借用（借用冲突时退化为无默认单位）
                let properties = self.try_project_properties();
                let default_duration_units = properties
                    .as_deref()
                    .map(ProjectProperties::get_default_duration_units);
                // Java 越界时抛异常；Rust 侧取不到时按 0 处理（`getDurationTimeUnits(0)` = DAYS）
                let units = MPPUtility::get_duration_time_units_with_default(
                    checked_short(value, 4).unwrap_or(0),
                    default_duration_units,
                );
                let duration = checked_int(value, 0).unwrap_or(0);

                // Java 的 `getAdjustedDuration` 在 duration == -1 时返回 null；
                // 无法借用项目属性时同样返回 None（Java 不存在该情形）。
                properties
                    .as_deref()
                    .and_then(|properties| {
                        MPPUtility::get_adjusted_duration(properties, duration, units)
                    })
                    .map(FieldValue::Duration)
            }

            // Java: case COST: result = Double.valueOf(MPPUtility.getDouble(value, 0) / 100);
            CustomFieldValueDataType::Cost => Some(FieldValue::Double(
                MPPUtility::get_double(value, 0) / 100.0,
            )),

            // Java: case NUMBER: result = Double.valueOf(MPPUtility.getDouble(value, 0));
            CustomFieldValueDataType::Number => {
                Some(FieldValue::Double(MPPUtility::get_double(value, 0)))
            }

            // Java: case TEXT: result = MPPUtility.getUnicodeString(value, 0);
            CustomFieldValueDataType::Text => {
                Some(FieldValue::String(MPPUtility::get_unicode_string(value, 0)))
            }

            // Java: default: result = valueAsString(value);
            //（Java 的 switch 未列举 FLAG 且常量已枚举完毕，故只有 default 可达）
            CustomFieldValueDataType::Flag => Some(self.value_as_string(value)),
        }
    }

    /// 尽力把一段字节解释成字符串。
    ///
    /// 对应 Java: `private String valueAsString(byte[] value)`
    ///
    /// Java 在 `MPPUtility.getUnicodeString` 抛异常时回退到
    /// `ByteArrayHelper.hexdump(value, false)`；Rust 的
    /// [`MPPUtility::get_unicode_string`] 是「全函数」（越界与畸形数据都只返回字符串），
    /// 该 `catch` 分支不可达，故这里只保留解码结果。
    fn value_as_string(&self, value: &[u8]) -> FieldValue {
        // Java: try { result = MPPUtility.getUnicodeString(value, 0); }
        //       catch (Exception ex) { result = ByteArrayHelper.hexdump(value, false); }
        FieldValue::String(MPPUtility::get_unicode_string(value, 0))
    }

    /// 取「父级 unique ID」在定长数据块中的偏移。
    ///
    /// 对应 Java: `protected int m_parentOffset`（Rust 侧为访问器，见类型说明）
    pub fn get_parent_offset(&self) -> usize {
        self.parent_offset
    }

    /// 设置「父级 unique ID」的偏移。
    ///
    /// 对应 Java: 子类构造方法中的 `m_parentOffset = ...`
    pub fn set_parent_offset(&mut self, offset: usize) {
        self.parent_offset = offset;
    }

    /// 取「取值类型」在第二个定长数据块中的偏移。
    ///
    /// 对应 Java: `protected int m_typeOffset`（Rust 侧为访问器，见类型说明）
    pub fn get_type_offset(&self) -> usize {
        self.type_offset
    }

    /// 设置「取值类型」的偏移。
    ///
    /// 对应 Java: 子类构造方法中的 `m_typeOffset = ...`
    pub fn set_type_offset(&mut self, offset: usize) {
        self.type_offset = offset;
    }

    /// 取「查找表 GUID」在第二个定长数据块中的偏移。
    ///
    /// 对应 Java: `protected int m_fieldOffset`（Rust 侧为访问器，见类型说明）
    pub fn get_field_offset(&self) -> usize {
        self.field_offset
    }

    /// 设置「查找表 GUID」的偏移。
    ///
    /// 对应 Java: 子类构造方法中的 `m_fieldOffset = ...`
    pub fn set_field_offset(&mut self, offset: usize) {
        self.field_offset = offset;
    }

    /// 借用项目属性（Rust 特有：Java 在构造方法里保存属性对象的引用）。
    fn try_project_properties(&self) -> Option<Ref<'_, ProjectProperties>> {
        let file = self.file.try_borrow().ok()?;
        Some(Ref::map(file, |file| file.get_project_properties()))
    }
}

/// 读取两字节短整数；数据不足时返回 `None`。
///
/// 对应 Java: `ByteArrayHelper.getShort(byte[], int)`（越界时 Java 抛
/// `ArrayIndexOutOfBoundsException`，Rust 侧返回 `None`）。
pub(crate) fn checked_short(data: &[u8], offset: usize) -> Option<i32> {
    if offset.checked_add(2)? > data.len() {
        return None;
    }

    Some(ByteArrayHelper::get_short(data, offset))
}

/// 读取四字节整数；数据不足时返回 `None`。
///
/// 对应 Java: `ByteArrayHelper.getInt(byte[], int)`（越界时 Java 抛异常）。
pub(crate) fn checked_int(data: &[u8], offset: usize) -> Option<i32> {
    if offset.checked_add(4)? > data.len() {
        return None;
    }

    Some(ByteArrayHelper::get_int(data, offset))
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::collections::HashMap;
    use std::rc::Rc;

    use uuid::Uuid;

    use crate::common::mpp_field_type::MppFieldType;
    use crate::data_type::DataType;
    use crate::field_key::{CustomFieldKey, FieldKey};
    use crate::field_type_class::FieldTypeClass;
    use crate::mpp::fixed_data::FixedData;
    use crate::mpp::fixed_meta::FixedMeta;
    use crate::mpp::var2_data::Var2Data;
    use crate::mpp::var_meta::VarMeta;
    use crate::mpp::var_meta9::VarMeta9;
    use crate::project_file::ProjectFile;

    /// 构造一个 `FixedMeta` 块：16 字节块头 + 若干条 8 字节记录（第 4..8 字节是数据偏移）。
    pub(crate) fn fixed_meta_bytes(offsets: &[i32]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFADFADBAu32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(offsets.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());

        for offset in offsets {
            let mut item = vec![0xEE_u8; 8];
            item[4..8].copy_from_slice(&offset.to_le_bytes());
            data.extend_from_slice(&item);
        }

        data
    }

    /// 由若干条记录构造一个 `FixedData`（每条记录的长度由它在数据体中的实际字节数决定）。
    pub(crate) fn fixed_data(items: &[Vec<u8>]) -> FixedData {
        let mut offsets = Vec::new();
        let mut body = Vec::new();

        for item in items {
            offsets.push(body.len() as i32);
            body.extend_from_slice(item);
        }

        // 元数据记录固定 8 字节，偏移取自各条记录在数据体中的真实位置
        let meta =
            FixedMeta::with_item_size(&fixed_meta_bytes(&offsets), 8).expect("FixedMeta 合法");
        FixedData::new(&meta, &body)
    }

    /// 构造一个空的 `FixedData`（0 条记录）。
    pub(crate) fn empty_fixed_data() -> FixedData {
        fixed_data(&[])
    }

    /// 手工拼装 Var2Data 数据块，返回「数据块 + 每项偏移」。
    pub(crate) fn data_block(items: &[Vec<u8>]) -> (Vec<u8>, Vec<i32>) {
        let mut data = Vec::new();
        let mut offsets = Vec::new();

        for item in items {
            offsets.push(data.len() as i32);
            data.extend_from_slice(&(item.len() as i32).to_le_bytes());
            data.extend_from_slice(item);
        }

        (data, offsets)
    }

    /// 用「唯一 ID + 类型 + 数据项索引」拼装配套的 VarMeta9 块。
    pub(crate) fn var_meta9_bytes(
        entries: &[(i32, i32, usize)],
        offsets: &[i32],
        data_size: i32,
    ) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(entries.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&data_size.to_le_bytes());

        for (unique_id, type_, index) in entries {
            data.extend_from_slice(&unique_id.to_le_bytes()[..3]);
            data.push(*type_ as u8);
            data.extend_from_slice(&offsets[*index].to_le_bytes());
        }

        data
    }

    /// 构造一个 `Var2Data` 并同时返回其元数据句柄（读取器构造需要两者）。
    pub(crate) fn var_data_with_meta(
        entries: &[(i32, i32, Vec<u8>)],
    ) -> (Rc<dyn VarMeta>, Var2Data) {
        let items: Vec<Vec<u8>> = entries.iter().map(|(_, _, data)| data.clone()).collect();
        let (data, offsets) = data_block(&items);
        let table: Vec<(i32, i32, usize)> = entries
            .iter()
            .enumerate()
            .map(|(index, (id, type_, _))| (*id, *type_, index))
            .collect();

        let meta: Rc<dyn VarMeta> = Rc::new(
            VarMeta9::new(&var_meta9_bytes(&table, &offsets, data.len() as i32))
                .expect("元数据合法"),
        );
        let file = ProjectFile::new();
        let var_data = Var2Data::new(&file, Rc::clone(&meta), &data);
        (meta, var_data)
    }

    /// 按 [`crate::mpp::mpp_utility::MPPUtility::get_guid`] 的字节序构造 16 字节 GUID。
    pub(crate) fn guid_bytes(value: u128) -> [u8; 16] {
        let uuid = Uuid::from_u128(value);
        let bytes = uuid.as_bytes();
        [
            bytes[3], bytes[2], bytes[1], bytes[0], bytes[5], bytes[4], bytes[7], bytes[6],
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]
    }

    /// 构造「查找表 GUID → 字段」映射。
    pub(crate) fn lookup_table_map(entries: &[(u128, MppFieldType)]) -> HashMap<Uuid, MppFieldType> {
        entries
            .iter()
            .map(|(guid, field)| (Uuid::from_u128(*guid), field.clone()))
            .collect()
    }

    /// 构造一枚自定义字段键（测试用）。
    pub(crate) fn custom_field_key(unique_id: i32) -> FieldKey {
        FieldKey::Custom(CustomFieldKey::new(
            unique_id,
            FieldTypeClass::Task,
            DataType::String,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{
        custom_field_key, empty_fixed_data, fixed_data, guid_bytes, lookup_table_map,
        var_data_with_meta,
    };
    use super::*;
    use crate::duration::Duration;
    use crate::field_key::FieldKey;
    use crate::mpxj_enum::MpxjEnum;
    use crate::task_field::TaskField;
    use crate::time_unit::TimeUnit;
    use chrono::NaiveDate;

    /// 取值项 unique ID（测试统一使用）。
    const ID: i32 = 1;

    /// 定长数据块记录长度（reader12 的偏移为 8 / 32 / 48，故第二个块需要 50 字节以上）。
    const FIXED_ITEM_SIZE: usize = 20;
    const FIXED_DATA2_ITEM_SIZE: usize = 64;

    /// 内置任务字段 NAME。
    fn name_field() -> MppFieldType {
        MppFieldType::Builtin(FieldKey::Task(TaskField::Name))
    }

    /// 构造一条定长数据块记录。
    fn fixed_record(unique_id: i32, parent_unique_id: Option<i32>) -> Vec<u8> {
        let mut record = vec![0u8; FIXED_ITEM_SIZE];
        record[UNJQUE_ID_OFFSET..UNJQUE_ID_OFFSET + 2]
            .copy_from_slice(&(unique_id as u16).to_le_bytes());
        if let Some(parent_unique_id) = parent_unique_id {
            record[8..10].copy_from_slice(&(parent_unique_id as u16).to_le_bytes());
        }
        record
    }

    /// 构造一条第二个定长数据块的记录（取值项 GUID / 查找表 GUID / 取值类型）。
    fn fixed_record2(
        item_guid: Option<u128>,
        lookup_table_guid: Option<u128>,
        type_: Option<CustomFieldValueDataType>,
    ) -> Vec<u8> {
        let mut record = vec![0u8; FIXED_DATA2_ITEM_SIZE];
        if let Some(guid) = item_guid {
            record[0..16].copy_from_slice(&guid_bytes(guid));
        }
        if let Some(guid) = lookup_table_guid {
            record[32..48].copy_from_slice(&guid_bytes(guid));
        }
        if let Some(type_) = type_ {
            record[48..50].copy_from_slice(&(type_.value() as u16).to_le_bytes());
        }
        record
    }

    /// 构造一个读取器样本：定长块 1 条记录 + 第二个定长块 1 条记录 + 变长块 3 项。
    ///
    /// 定长块（20 字节）：[4..6) = ID，[8..10) = 父级 ID = 7
    /// 第二个定长块（64 字节）：[0..16) = 取值项 GUID，[32..48) = 查找表 GUID，
    /// [48..50) = 取值类型 = NUMBER(15)
    /// 变长块：22（取值）= 8 字节 double 1.5；8（描述）= "desc"；23（未知）= 3 字节
    fn sample_reader() -> (CustomFieldValueReader, Rc<RefCell<ProjectFile>>) {
        let mut utf16 = Vec::new();
        for unit in "desc".encode_utf16() {
            utf16.extend_from_slice(&unit.to_le_bytes());
        }
        utf16.extend_from_slice(&[0x00, 0x00]);

        let (meta, var_data) = var_data_with_meta(&[
            (ID, VALUE_LIST_VALUE, 1.5f64.to_le_bytes().to_vec()),
            (ID, VALUE_LIST_DESCRIPTION, utf16),
            (ID, VALUE_LIST_UNKNOWN, vec![1, 2, 3]),
        ]);

        let file = ProjectFile::new();
        let mut reader = CustomFieldValueReader::new(
            &file,
            lookup_table_map(&[(9, name_field())]),
            meta,
            var_data,
            fixed_data(&[fixed_record(ID, Some(7))]),
            fixed_data(&[fixed_record2(
                Some(5),
                Some(9),
                Some(CustomFieldValueDataType::Number),
            )]),
        );
        // 与 CustomFieldValueReader12 相同的偏移
        reader.set_parent_offset(8);
        reader.set_type_offset(48);
        reader.set_field_offset(32);

        (reader, file)
    }

    /// 取值项被登记进容器（按 unique ID 与 GUID），并挂到对应字段的查找表上。
    #[test]
    fn process_registers_items_and_populates_lookup_table() {
        let (reader, file) = sample_reader();
        reader.process();

        let container = file.borrow().get_custom_fields();

        // 按 unique ID / GUID 都能查到
        let by_id = container
            .borrow()
            .get_custom_field_value_item_by_unique_id(ID)
            .expect("按 unique ID 应能查到");
        let by_guid = container
            .borrow()
            .get_custom_field_value_item_by_guid(Uuid::from_u128(5))
            .expect("按 GUID 应能查到");
        assert!(Rc::ptr_eq(&by_id, &by_guid));

        let item = by_id.borrow();
        assert_eq!(item.get_unique_id(), ID);
        assert_eq!(item.get_parent_unique_id(), Some(7));
        assert_eq!(item.get_description(), Some("desc"));
        assert_eq!(item.get_unknown(), Some(&[1u8, 2, 3][..]));
        assert_eq!(item.get_type(), Some(CustomFieldValueDataType::Number));
        assert_eq!(item.get_value(), Some(&FieldValue::Double(1.5)));

        // 查找表：按查找表 GUID 找到字段 NAME，表里有一项且 GUID 已设置
        let custom_field = container
            .borrow()
            .get(FieldKey::Task(TaskField::Name))
            .expect("应已创建字段配置");
        let custom_field = custom_field.borrow();
        let table = custom_field.get_lookup_table();
        assert_eq!(table.len(), 1);
        assert_eq!(table.get_guid(), Some(Uuid::from_u128(9)));
        assert_eq!(table.get(0).map(|item| item.get_unique_id()), Some(ID));
        assert_eq!(
            table.get(0).and_then(|item| item.get_value()),
            Some(&FieldValue::Double(1.5))
        );
    }

    /// 重复 unique ID：第二次出现时替换查找表中的元素，而不是追加。
    #[test]
    fn process_replaces_duplicate_entries_in_lookup_table() {
        let (meta, var_data) =
            var_data_with_meta(&[(ID, VALUE_LIST_VALUE, 2.5f64.to_le_bytes().to_vec())]);

        let file = ProjectFile::new();
        let mut reader = CustomFieldValueReader::new(
            &file,
            lookup_table_map(&[(9, name_field())]),
            meta,
            var_data,
            fixed_data(&[fixed_record(ID, Some(7)), fixed_record(ID, Some(7))]),
            fixed_data(&[
                fixed_record2(None, Some(9), Some(CustomFieldValueDataType::Number)),
                fixed_record2(None, Some(9), Some(CustomFieldValueDataType::Number)),
            ]),
        );
        reader.set_parent_offset(8);
        reader.set_type_offset(48);
        reader.set_field_offset(32);
        reader.process();

        let container = file.borrow().get_custom_fields();
        let custom_field = container
            .borrow()
            .get(FieldKey::Task(TaskField::Name))
            .expect("应已创建字段配置");
        let custom_field = custom_field.borrow();
        // 仍然只有一项（Java 的「替换」分支）
        assert_eq!(custom_field.get_lookup_table().len(), 1);
        assert_eq!(
            custom_field
                .get_lookup_table()
                .get(0)
                .and_then(|item| item.get_value()),
            Some(&FieldValue::Double(2.5))
        );
    }

    /// 查找表 GUID 无法在映射中找到字段时，只登记到容器（Java 的 `field == null` 分支）。
    #[test]
    fn process_skips_unknown_lookup_table_guid() {
        let (meta, var_data) =
            var_data_with_meta(&[(ID, VALUE_LIST_VALUE, 1.5f64.to_le_bytes().to_vec())]);

        let file = ProjectFile::new();
        let mut reader = CustomFieldValueReader::new(
            &file,
            // 空映射：查不到字段
            HashMap::new(),
            meta,
            var_data,
            fixed_data(&[fixed_record(ID, None)]),
            fixed_data(&[fixed_record2(None, Some(9), None)]),
        );
        reader.set_parent_offset(8);
        reader.set_type_offset(48);
        reader.set_field_offset(32);
        reader.process();

        let container = file.borrow().get_custom_fields();
        assert!(container
            .borrow()
            .get_custom_field_value_item_by_unique_id(ID)
            .is_some());
        assert_eq!(container.borrow().size(), 0);
    }

    /// unique ID 不在变长数据元数据里时跳过（Java 的 `continue`）。
    #[test]
    fn process_skips_items_without_var_data() {
        let (meta, var_data) =
            var_data_with_meta(&[(99, VALUE_LIST_VALUE, 1.5f64.to_le_bytes().to_vec())]);

        let file = ProjectFile::new();
        let mut reader = CustomFieldValueReader::new(
            &file,
            lookup_table_map(&[(9, name_field())]),
            meta,
            var_data,
            fixed_data(&[fixed_record(ID, None)]),
            fixed_data(&[fixed_record2(None, Some(9), None)]),
        );
        reader.set_parent_offset(8);
        reader.set_type_offset(48);
        reader.set_field_offset(32);
        reader.process();

        let container = file.borrow().get_custom_fields();
        assert!(container
            .borrow()
            .get_custom_field_value_item_by_unique_id(ID)
            .is_none());
    }

    /// 定长块为空 / 记录过短时不会 panic。
    #[test]
    fn process_handles_empty_and_truncated_data() {
        // 定长块为空
        let (meta, var_data) =
            var_data_with_meta(&[(ID, VALUE_LIST_VALUE, 1.5f64.to_le_bytes().to_vec())]);
        let file = ProjectFile::new();
        let mut reader = CustomFieldValueReader::new(
            &file,
            lookup_table_map(&[(9, name_field())]),
            meta,
            var_data,
            empty_fixed_data(),
            empty_fixed_data(),
        );
        reader.set_parent_offset(8);
        reader.set_type_offset(48);
        reader.set_field_offset(32);
        reader.process();
        assert!(file
            .borrow()
            .get_custom_fields()
            .borrow()
            .get_custom_field_value_item_by_unique_id(ID)
            .is_none());

        // 记录只有 2 字节：读不出 unique ID（Java 抛越界异常）
        let (meta, var_data) =
            var_data_with_meta(&[(ID, VALUE_LIST_VALUE, 1.5f64.to_le_bytes().to_vec())]);
        let file = ProjectFile::new();
        let mut reader = CustomFieldValueReader::new(
            &file,
            lookup_table_map(&[(9, name_field())]),
            meta,
            var_data,
            fixed_data(&[vec![0x01, 0x00]]),
            fixed_data(&[vec![0x00, 0x00]]),
        );
        reader.set_parent_offset(8);
        reader.set_type_offset(48);
        reader.set_field_offset(32);
        reader.process();

        assert!(file
            .borrow()
            .get_custom_fields()
            .borrow()
            .get_custom_field_value_item_by_unique_id(ID)
            .is_none());
    }

    /// 取值类型映射（`getTypedValue` 的各分支）。
    #[test]
    fn typed_value_maps_each_type() {
        let (reader, file) = sample_reader();

        // 项目属性会影响工期换算
        {
            let mut file = file.borrow_mut();
            let properties = file.get_project_properties_mut();
            properties.set_minutes_per_day(Some(480));
            properties.set_days_per_month(Some(20));
        }

        // DATE：4 字节时间戳（低 2 字节为当日秒数、高 2 字节为天数）
        let date_bytes = [0x00, 0x00, 0x02, 0x00];
        assert_eq!(
            reader.get_typed_value(Some(CustomFieldValueDataType::Date), &date_bytes),
            Some(FieldValue::DateTime(
                NaiveDate::from_ymd_opt(1984, 1, 2)
                    .expect("合法日期")
                    .and_hms_opt(0, 0, 0)
                    .expect("合法时间")
            ))
        );
        // FINISH_DATE 与 DATE 走同一分支
        assert_eq!(
            reader.get_typed_value(Some(CustomFieldValueDataType::FinishDate), &date_bytes),
            reader.get_typed_value(Some(CustomFieldValueDataType::Date), &date_bytes)
        );

        // DURATION：4 字节工期 + 2 字节单位（7 = 天）
        let mut duration_bytes = 4800i32.to_le_bytes().to_vec();
        duration_bytes.extend_from_slice(&7u16.to_le_bytes());
        assert_eq!(
            reader.get_typed_value(Some(CustomFieldValueDataType::Duration), &duration_bytes),
            Some(FieldValue::Duration(Duration::get_instance(
                1.0,
                TimeUnit::Days
            )))
        );

        // COST：以「分」为单位的双精度
        assert_eq!(
            reader.get_typed_value(
                Some(CustomFieldValueDataType::Cost),
                &250.0f64.to_le_bytes()
            ),
            Some(FieldValue::Double(2.5))
        );

        // NUMBER：双精度
        assert_eq!(
            reader.get_typed_value(
                Some(CustomFieldValueDataType::Number),
                &1.5f64.to_le_bytes()
            ),
            Some(FieldValue::Double(1.5))
        );

        // TEXT：UTF-16LE
        assert_eq!(
            reader.get_typed_value(
                Some(CustomFieldValueDataType::Text),
                &[0x41, 0x00, 0x42, 0x00, 0x00, 0x00]
            ),
            Some(FieldValue::String("AB".to_owned()))
        );

        // FLAG 与「类型未知」都按字符串猜测（Java 的 default 分支）
        assert_eq!(
            reader.get_typed_value(
                Some(CustomFieldValueDataType::Flag),
                &[0x41, 0x00, 0x00, 0x00]
            ),
            Some(FieldValue::String("A".to_owned()))
        );
        assert_eq!(
            reader.get_typed_value(None, &[0x41, 0x00, 0x00, 0x00]),
            Some(FieldValue::String("A".to_owned()))
        );
    }

    /// 缺失 / 畸形数据不 panic，且返回 Java 的默认值。
    #[test]
    fn typed_value_handles_missing_data() {
        let (reader, _file) = sample_reader();

        // 空数据：日期时间戳读不出来 → None
        assert_eq!(
            reader.get_typed_value(Some(CustomFieldValueDataType::Date), &[]),
            None
        );
        // 工期：越界读按 0 处理 → 0 天（Java 的 `getInt` 越界会抛异常）
        assert_eq!(
            reader.get_typed_value(Some(CustomFieldValueDataType::Duration), &[]),
            Some(FieldValue::Duration(Duration::get_instance(
                0.0,
                TimeUnit::Days
            )))
        );
        // 双精度越界 → 0（与 Java 的 `getDouble` 一致）
        assert_eq!(
            reader.get_typed_value(Some(CustomFieldValueDataType::Cost), &[0x01]),
            Some(FieldValue::Double(0.0))
        );
        assert_eq!(
            reader.get_typed_value(Some(CustomFieldValueDataType::Number), &[]),
            Some(FieldValue::Double(0.0))
        );
        // 空字符串
        assert_eq!(
            reader.get_typed_value(Some(CustomFieldValueDataType::Text), &[]),
            Some(FieldValue::String(String::new()))
        );

        // duration == -1 → Java 的 `getAdjustedDuration` 返回 null
        let mut duration_bytes = (-1i32).to_le_bytes().to_vec();
        duration_bytes.extend_from_slice(&7u16.to_le_bytes());
        assert_eq!(
            reader.get_typed_value(Some(CustomFieldValueDataType::Duration), &duration_bytes),
            None
        );
    }

    /// 偏移访问器（Java 的 `protected` 字段）往返。
    #[test]
    fn offsets_round_trip() {
        let (mut reader, _file) = sample_reader();

        assert_eq!(reader.get_parent_offset(), 8);
        assert_eq!(reader.get_type_offset(), 48);
        assert_eq!(reader.get_field_offset(), 32);

        reader.set_parent_offset(10);
        reader.set_type_offset(16);
        reader.set_field_offset(18);
        assert_eq!(reader.get_parent_offset(), 10);
        assert_eq!(reader.get_type_offset(), 16);
        assert_eq!(reader.get_field_offset(), 18);
    }

    /// 自定义字段（`FieldKey::Custom`）也能挂上查找表。
    #[test]
    fn process_handles_custom_field_keys() {
        let (meta, var_data) =
            var_data_with_meta(&[(ID, VALUE_LIST_VALUE, vec![0x41, 0x00, 0x00, 0x00])]);

        let file = ProjectFile::new();
        let mut reader = CustomFieldValueReader::new(
            &file,
            lookup_table_map(&[(9, MppFieldType::Builtin(custom_field_key(3)))]),
            meta,
            var_data,
            fixed_data(&[fixed_record(ID, None)]),
            fixed_data(&[fixed_record2(
                None,
                Some(9),
                Some(CustomFieldValueDataType::Text),
            )]),
        );
        reader.set_parent_offset(8);
        reader.set_type_offset(48);
        reader.set_field_offset(32);
        reader.process();

        let container = file.borrow().get_custom_fields();
        let custom_field = container
            .borrow()
            .get(custom_field_key(3))
            .expect("自定义字段应已创建");
        assert_eq!(custom_field.borrow().get_lookup_table().len(), 1);
    }
}
