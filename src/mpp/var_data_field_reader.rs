//! 对应 Java: `org/mpxj/mpp/VarDataFieldReader.java`
//!
//! 从变长数据块（[`Var2Data`]）中按 `(唯一 ID, 数据类型)` 读取字段值的策略类：
//! 先看该数据项头两个字节是否为「取值列表」标志 —— 是则按列表项的 unique ID / GUID 到自定义
//! 字段容器里查实际取值；否则交给子类的 `readValue` 按类型解码。子类的 `coerceValue` 负责把
//! 取值收敛到目标类型（类型不符时返回 `null`）。
//!
//! # 继承 / 接口的还原
//!
//! Java: `abstract class VarDataFieldReader`（抽象类，一个具体方法 `getValue` + 两个抽象方法）。
//! Rust 侧按本批约定把它还原为 trait：
//!
//! | Java | Rust |
//! |---|---|
//! | `abstract class VarDataFieldReader` | [`VarDataFieldReader`] trait |
//! | `public Object getValue(Var2Data, Integer, Integer)`（具体方法） | trait 的**默认方法** [`VarDataFieldReader::get_value`]，方法体与 Java 逐行对应 |
//! | `protected abstract Object readValue(...)` | trait 的必需方法 [`VarDataFieldReader::read_value`]，由三个实现结构体提供 |
//! | `protected abstract Object coerceValue(Object)` | trait 的必需方法 [`VarDataFieldReader::coerce_value`]；Java 的 `Object`（可为 null）→ `Option<&FieldValue>` |
//! | `private final CustomFieldContainer m_customFields`（私有字段） | trait 的必需访问器 [`VarDataFieldReader::get_custom_fields`]（Rust 特有） |
//! | 子类通过继承访问 `m_customFields` | trait 默认方法不能访问实现结构体的字段，故声明访问器；三个实现结构体各自持有 `Rc<RefCell<CustomFieldContainer>>` 并转发 |
//! | `private Object getValueByID(byte[])`（私有方法） | 本文件的私有自由函数 `get_value_by_id`（需要 `&dyn VarDataFieldReader` 以取得容器与实现类的 `coerce_value`） |
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `Object` 返回值 | `Option<FieldValue>` | 本 crate 用 [`FieldValue`] 承载 Java 的 `Object` 字段值（可表达 `Double` / `String` / `LocalDateTime` 等）；`null` → `None` |
//! | `Integer id` / `Integer type` | `Option<i32>` | 「`null` → `Option<T>`」约定 |
//! | `byte[] data` | `Vec<u8>`（取回后按 `&[u8]` 使用） | Java 的 `byte[]` 入参在 Rust 侧为 `&[u8]` |
//! | `data.length`（`data` 为 null 时抛 NPE） | 取不到数据项时返回 `None` | 遵循「不 panic」约定；该分支实际不可达（标志位命中说明数据项存在且长度 ≥ 2） |
//! | `ByteArrayHelper.getInt(data, 2)` 越界抛 `ArrayIndexOutOfBoundsException` | 数据不足 6 字节时返回 `None` | 同上 |
//! | `UUID guid`（可为 null） | `Option<Uuid>` | [`MPPUtility::get_guid`] 返回 `None` 表示 Java 的 null；此时不做 GUID 查找（Java 的 `HashMap.get(null)` 同样查不到） |
//! | `LocalDateHelper.getLocalDateTime(...)` | [`LocalDateHelper::get_local_date_time`] | 返回 `Option<NaiveDateTime>`（`null` → `None`） |
//! | `m_customFields.getCustomFieldValueItemByGuid(...)` | 取容器前先 `try_borrow()` | `RefCell` 借用冲突时返回 `None` 而非 panic（见 [`get_value_by_id`] 注释） |
//!
//! 跳过的 Java 方法：无。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::local_date_helper::LocalDateHelper;
use crate::custom_field_container::CustomFieldContainer;
use crate::field_value::FieldValue;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::var2_data::Var2Data;

/// 「带 ID 的取值列表」标志。
///
/// 对应 Java: `private static final int VALUE_LIST_WITH_ID_MASK = 0x0701;`
const VALUE_LIST_WITH_ID_MASK: i32 = 0x0701;

/// 「不带 ID 的取值列表」标志。
///
/// 对应 Java: `private static final int VALUE_LIST_WITHOUT_ID_MASK = 0x0401;`
const VALUE_LIST_WITHOUT_ID_MASK: i32 = 0x0401;

/// 从变长数据块中按类型读取字段值的策略。
///
/// 对应 Java: `abstract class VarDataFieldReader`
///
/// [`get_value`](VarDataFieldReader::get_value) 是「模板方法」，其流程由本 trait 提供；
/// 具体类型的解码与收敛由实现结构体的
/// [`read_value`](VarDataFieldReader::read_value) /
/// [`coerce_value`](VarDataFieldReader::coerce_value) 决定。
pub trait VarDataFieldReader {
    /// 取自定义字段容器。
    ///
    /// 对应 Java 字段: `private final CustomFieldContainer m_customFields`
    ///
    /// Rust 特有：Java 的子类通过继承访问基类的私有字段，而 trait 的默认方法无法访问实现
    /// 结构体的字段，故以本访问器还原该访问路径。三个实现结构体都只是把持有的
    /// `Rc<RefCell<CustomFieldContainer>>` 返回。
    fn get_custom_fields(&self) -> &Rc<RefCell<CustomFieldContainer>>;

    /// 从变长数据块中取值。
    ///
    /// 对应 Java: `public Object getValue(Var2Data varData, Integer id, Integer type)`
    ///
    /// 数据项头两个字节为取值列表标志时，走「按列表项 unique ID / GUID 查自定义字段容器」的
    /// 分支（见私有函数 `get_value_by_id`）；否则交给实现类的 `read_value`。
    fn get_value(
        &self,
        var_data: &Var2Data,
        id: Option<i32>,
        type_: Option<i32>,
    ) -> Option<FieldValue> {
        // Java: int flag = varData.getShort(id, type);
        let flag = var_data.get_short(id, type_);

        if flag == VALUE_LIST_WITH_ID_MASK || flag == VALUE_LIST_WITHOUT_ID_MASK {
            // Java: byte[] data = varData.getByteArray(id, type);
            //       if (data.length == 4) ...
            // Java 对 data == null 会抛 NPE；Rust 侧返回 None（该分支实际不可达）。
            let data = var_data.get_byte_array(id, type_)?;

            if data.len() == 4 {
                // Java 注释：不确定这是否正确，手头没有「返回非空日期」的样本。
                // Java: result = LocalDateHelper.getLocalDateTime(MPPUtility.getDate(data, 2));
                LocalDateHelper::get_local_date_time(MPPUtility::get_date(&data, 2))
                    .map(FieldValue::DateTime)
            } else {
                // Java: result = getValueByID(data);
                get_value_by_id(self, &data)
            }
        } else {
            // Java: result = readValue(varData, id, type);
            self.read_value(var_data, id, type_)
        }
    }

    /// 由实现类覆写：按各自的目标类型读取数据项。
    ///
    /// 对应 Java: `protected abstract Object readValue(Var2Data varData, Integer id, Integer type)`
    fn read_value(
        &self,
        var_data: &Var2Data,
        id: Option<i32>,
        type_: Option<i32>,
    ) -> Option<FieldValue>;

    /// 由实现类覆写：把从取值列表项里取到的值收敛到目标类型。
    ///
    /// 对应 Java: `protected abstract Object coerceValue(Object value)`
    ///
    /// Java 的 `value` 可为 null（取值项未设置取值时）→ Rust 侧 `None`。
    fn coerce_value(&self, value: Option<&FieldValue>) -> Option<FieldValue>;
}

/// 处理「取值 = 整数 unique ID + GUID」形式的数据块。
///
/// 对应 Java: `private Object getValueByID(byte[] data)`
///
/// Java 里这是基类的私有实例方法；Rust 侧改为接收读取器的自由函数，
/// 因为它需要容器与实现类的 `coerce_value`。参数用 `?Sized` 泛型（而非 `&dyn`）是因为
/// trait 默认方法里的 `Self` 可能本身就是 `dyn VarDataFieldReader`，无大小的 `&Self`
/// 无法直接强制转换成 trait 对象。
///
/// 数据块共 26 字节：2 字节掩码 + 4 字节 unique ID + 16 字节 GUID + 4 字节未知。
fn get_value_by_id<T: VarDataFieldReader + ?Sized>(
    reader: &T,
    data: &[u8],
) -> Option<FieldValue> {
    // Java: int uniqueId = ByteArrayHelper.getInt(data, 2);
    // Java 在数据不足 6 字节时抛 ArrayIndexOutOfBoundsException，Rust 侧返回 None。
    if data.len() < 6 {
        return None;
    }
    let unique_id = ByteArrayHelper::get_int(data, 2);

    // Java: UUID guid = MPPUtility.getGUID(data, 6);
    // 数据不足 16 字节时 Rust 侧为 None（Java 返回 null）。
    let guid = MPPUtility::get_guid(data, 6);

    // Java 的容器方法只读，Rust 侧用 try_borrow 以免「容器正被可变借用」时 panic；
    // 借用冲突时按「查不到」处理（Java 无此情形）。
    let container = reader.get_custom_fields().try_borrow().ok()?;

    // Java: if (uniqueId == -1) { item = m_customFields.getCustomFieldValueItemByGuid(guid); }
    //       else { item = ...getCustomFieldValueItemByUniqueID(uniqueId);
    //              if (item == null) { item = ...getCustomFieldValueItemByGuid(guid); } }
    let item = if unique_id == -1 {
        guid.and_then(|guid| container.get_custom_field_value_item_by_guid(guid))
    } else {
        match container.get_custom_field_value_item_by_unique_id(unique_id) {
            Some(item) => Some(item),
            // Java 注释：unique ID 查不到时回退到 GUID
            None => guid.and_then(|guid| container.get_custom_field_value_item_by_guid(guid)),
        }
    };

    // Java: return item == null ? null : coerceValue(item.getValue());
    let item = item?;
    // 取值项自身为 RefCell：同样用 try_borrow 避免 panic
    let item = item.try_borrow().ok()?;
    reader.coerce_value(item.get_value())
}

/// 本文件的单元测试支撑：手工拼装 `Var2Data` 与其元数据。
///
/// 三个实现类文件的测试也复用这里的构造函数（`VarMeta9` 块的字节布局与
/// [`crate::mpp::var2_data`] 的测试一致）。
#[cfg(test)]
pub(crate) mod test_support {
    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::custom_field_container::CustomFieldContainer;
    use crate::custom_field_lookup_table::CustomFieldValueItem;
    use crate::mpp::var2_data::Var2Data;
    use crate::mpp::var_meta9::VarMeta9;
    use crate::project_file::ProjectFile;

    /// 手工拼装数据块：每项的形式与 MPP 一致（4 字节长度 + 数据本身）。
    ///
    /// 返回「数据块 + 每项的偏移」。
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
        // magic = 0xFADFADBA（小端）
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

    /// 按 `(唯一 ID, 数据类型, 数据)` 列表构造一个 `Var2Data`。
    pub(crate) fn var_data(entries: &[(i32, i32, Vec<u8>)]) -> Var2Data {
        let items: Vec<Vec<u8>> = entries.iter().map(|(_, _, data)| data.clone()).collect();
        let (data, offsets) = data_block(&items);
        let table: Vec<(i32, i32, usize)> = entries
            .iter()
            .enumerate()
            .map(|(index, (id, type_, _))| (*id, *type_, index))
            .collect();

        let meta = VarMeta9::new(&var_meta9_bytes(&table, &offsets, data.len() as i32))
            .expect("元数据合法");
        let file = ProjectFile::new();
        Var2Data::new(&file, Rc::new(meta), &data)
    }

    /// 构造一个已登记给定取值项的自定义字段容器。
    pub(crate) fn container_with_item(
        item: CustomFieldValueItem,
    ) -> Rc<RefCell<CustomFieldContainer>> {
        let container = CustomFieldContainer::new();
        container
            .borrow_mut()
            .register_value(Rc::new(RefCell::new(item)));
        container
    }

    /// 拼装「带 ID 的取值列表」数据块（掩码取 [`VALUE_LIST_WITH_ID_MASK`](super::VALUE_LIST_WITH_ID_MASK)）。
    pub(crate) fn value_list_data_with_id(unique_id: i32, guid: [u8; 16]) -> Vec<u8> {
        value_list_data(super::VALUE_LIST_WITH_ID_MASK, unique_id, guid)
    }

    /// 拼装「取值列表」数据块：2 字节掩码 + 4 字节 unique ID + 16 字节 GUID + 4 字节未知。
    pub(crate) fn value_list_data(mask: i32, unique_id: i32, guid: [u8; 16]) -> Vec<u8> {
        let mut data = Vec::new();
        // 掩码按 Java 的 getShort 语义写入（小端）
        data.extend_from_slice(&mask.to_le_bytes()[..2]);
        data.extend_from_slice(&unique_id.to_le_bytes());
        data.extend_from_slice(&guid);
        data.extend_from_slice(&[0u8; 4]);
        data
    }

    /// 按 [`MPPUtility::get_guid`](crate::mpp::mpp_utility::MPPUtility::get_guid)
    /// 的字节序构造 16 字节 GUID（前 4 / 2 / 2 字节倒序）。
    pub(crate) fn guid_bytes(value: u128) -> [u8; 16] {
        let uuid = uuid::Uuid::from_u128(value);
        let bytes = uuid.as_bytes();
        [
            bytes[3], bytes[2], bytes[1], bytes[0], bytes[5], bytes[4], bytes[7], bytes[6],
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{container_with_item, guid_bytes, value_list_data, var_data};
    use super::*;
    use chrono::NaiveDate;
    use crate::custom_field_lookup_table::CustomFieldValueItem;
    use crate::mpp::double_var_data_field_reader::DoubleVarDataFieldReader;
    use crate::mpp::string_var_data_field_reader::StringVarDataFieldReader;

    /// 数据项类型（测试里统一使用）。
    const TYPE: i32 = 0x12;

    /// 取值列表项命中（unique ID 非 -1）：返回值按实现类收敛为 `Double`。
    #[test]
    fn value_list_item_is_looked_up_by_unique_id() {
        let mut item = CustomFieldValueItem::new(11);
        // Java 的取值项取值可以是任意 Object，这里放 Integer 以验证 Double 读取器的收敛
        item.set_value(Some(FieldValue::Integer(3)));
        let container = container_with_item(item);

        let var_data = var_data(&[(
            1,
            TYPE,
            value_list_data(VALUE_LIST_WITH_ID_MASK, 11, guid_bytes(7)),
        )]);

        let reader = DoubleVarDataFieldReader::new(container);
        assert_eq!(
            reader.get_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::Double(3.0))
        );
    }

    /// unique ID 为 -1 时按 GUID 查取值项。
    #[test]
    fn value_list_item_is_looked_up_by_guid_when_unique_id_is_minus_one() {
        let mut item = CustomFieldValueItem::new(11);
        item.set_guid(Some(uuid::Uuid::from_u128(7)));
        item.set_value(Some(FieldValue::Integer(5)));
        let container = container_with_item(item);

        let var_data = var_data(&[(
            1,
            TYPE,
            value_list_data(VALUE_LIST_WITH_ID_MASK, -1, guid_bytes(7)),
        )]);

        let reader = DoubleVarDataFieldReader::new(container);
        assert_eq!(
            reader.get_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::Double(5.0))
        );
    }

    /// unique ID 查不到时回退到 GUID。
    #[test]
    fn value_list_item_falls_back_to_guid() {
        let mut item = CustomFieldValueItem::new(11);
        item.set_guid(Some(uuid::Uuid::from_u128(9)));
        item.set_value(Some(FieldValue::Integer(5)));
        let container = container_with_item(item);

        // unique ID 与登记的取值项不匹配
        let var_data = var_data(&[(
            1,
            TYPE,
            value_list_data(VALUE_LIST_WITH_ID_MASK, 42, guid_bytes(9)),
        )]);

        let reader = DoubleVarDataFieldReader::new(container);
        assert_eq!(
            reader.get_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::Double(5.0))
        );
    }

    /// unique ID 与 GUID 都查不到时返回 `None`（Java 返回 null）。
    #[test]
    fn unknown_value_list_item_returns_none() {
        let mut item = CustomFieldValueItem::new(11);
        item.set_value(Some(FieldValue::Integer(5)));
        let container = container_with_item(item);

        let unmatched_var_data = var_data(&[(
            1,
            TYPE,
            value_list_data(VALUE_LIST_WITHOUT_ID_MASK, 42, guid_bytes(1)),
        )]);

        let reader = DoubleVarDataFieldReader::new(container);
        assert_eq!(
            reader.get_value(&unmatched_var_data, Some(1), Some(TYPE)),
            None
        );

        // 取值项已登记但未设置取值（Java 的 getValue 返回 null）
        let empty_item = CustomFieldValueItem::new(7);
        let container = container_with_item(empty_item);
        let empty_item_var_data = var_data(&[(
            1,
            TYPE,
            value_list_data(VALUE_LIST_WITHOUT_ID_MASK, 7, guid_bytes(1)),
        )]);
        let reader = DoubleVarDataFieldReader::new(container);
        assert_eq!(
            reader.get_value(&empty_item_var_data, Some(1), Some(TYPE)),
            None
        );
    }

    /// 4 字节的取值列表数据块按日期解码（Java 的 `data.length == 4` 分支）。
    #[test]
    fn four_byte_value_list_is_decoded_as_date() {
        // 掩码 0x0701 + 天数 1 → 1984-01-01
        let data = vec![0x01, 0x07, 0x01, 0x00];
        let var_data = var_data(&[(1, TYPE, data)]);

        let reader = DoubleVarDataFieldReader::new(container_with_item(CustomFieldValueItem::new(
            1,
        )));

        assert_eq!(
            reader.get_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::DateTime(
                NaiveDate::from_ymd_opt(1984, 1, 1)
                    .expect("合法日期")
                    .and_hms_opt(0, 0, 0)
                    .expect("合法时间")
            ))
        );
    }

    /// 数据项缺失时标志位读出 0（不命中取值列表标志），因此交给实现类的 `read_value`。
    #[test]
    fn missing_entry_is_delegated_to_read_value() {
        let var_data = var_data(&[(1, TYPE, value_list_data(VALUE_LIST_WITH_ID_MASK, 1, [0; 16]))]);

        // 字符串读取器：数据项不存在 → Java 的 getUnicodeString 返回 null → None
        let reader = StringVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));
        assert_eq!(reader.get_value(&var_data, Some(999), Some(TYPE)), None);
        assert_eq!(reader.get_value(&var_data, None, None), None);

        // 双精度读取器：Java 的 getDouble 对缺失数据项返回 0，故这里不是 None
        let reader = DoubleVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));
        assert_eq!(
            reader.get_value(&var_data, Some(999), Some(TYPE)),
            Some(FieldValue::Double(0.0))
        );
    }

    /// 取值列表数据块过短（不足 6 字节）时返回 `None` 而不 panic。
    #[test]
    fn truncated_value_list_data_returns_none() {
        // 只有掩码，Java 的 ByteArrayHelper.getInt(data, 2) 会抛越界异常
        let mask_only_var_data = var_data(&[(1, TYPE, vec![0x01, 0x04])]);
        let reader = DoubleVarDataFieldReader::new(container_with_item(CustomFieldValueItem::new(
            1,
        )));
        assert_eq!(
            reader.get_value(&mask_only_var_data, Some(1), Some(TYPE)),
            None
        );

        // 5 字节（掩码 + 3 字节）同样不足以读出 4 字节 unique ID
        let five_byte_var_data = var_data(&[(1, TYPE, vec![0x01, 0x04, 0x00, 0x00, 0x00])]);
        let reader = DoubleVarDataFieldReader::new(container_with_item(CustomFieldValueItem::new(
            1,
        )));
        assert_eq!(
            reader.get_value(&five_byte_var_data, Some(1), Some(TYPE)),
            None
        );
    }

    /// 非取值列表标志时走 `read_value`（由实现类决定）。
    #[test]
    fn non_list_flag_uses_read_value() {
        let var_data = var_data(&[(1, TYPE, vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF8, 0x3F])]);
        let reader = DoubleVarDataFieldReader::new(container_with_item(CustomFieldValueItem::new(
            1,
        )));

        assert_eq!(
            reader.get_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::Double(1.5))
        );
    }
}
