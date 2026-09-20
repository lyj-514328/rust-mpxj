//! 对应 Java: `org/mpxj/mpp/TimestampVarDataFieldReader.java`
//!
//! [`VarDataFieldReader`] 的实现：从变长数据块中读一个时间戳字段。
//!
//! # 继承的还原
//!
//! Java: `class TimestampVarDataFieldReader extends VarDataFieldReader`。
//! Rust 侧按约定：接口（抽象类）→ trait，实现类 → 独立结构体 + `impl`。
//!
//! | Java | Rust |
//! |---|---|
//! | `TimestampVarDataFieldReader extends VarDataFieldReader` | [`TimestampVarDataFieldReader`] 实现 [`VarDataFieldReader`] |
//! | `private final CustomFieldContainer m_customFields`（继承自基类） | 结构体自持 `custom_fields` 字段，经 [`VarDataFieldReader::get_custom_fields`] 暴露 |
//! | `public TimestampVarDataFieldReader(CustomFieldContainer)` | [`TimestampVarDataFieldReader::new`] |
//! | `protected Object readValue(...)` | [`VarDataFieldReader::read_value`] |
//! | `protected Object coerceValue(Object)` | [`VarDataFieldReader::coerce_value`] |
//! | `private Object getRawTimestampValue(Var2Data, Integer, Integer)` | 私有方法 `TimestampVarDataFieldReader::get_raw_timestamp_value` |
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `Object` 返回值 | `Option<FieldValue>` | Java 的 `getRawTimestampValue` 可能返回 `String`（512 字节分支）或 `LocalDateTime`；Rust 侧分别对应 `FieldValue::String` / `FieldValue::DateTime`，null → `None` |
//! | `MPPUtility.getTimestamp(data, 0)` | [`MPPUtility::get_timestamp`] | 返回 `Option<NaiveDateTime>`（天数为 NA 等情形 Java 返回 null） |
//! | `MPPUtility.getUnicodeString(data, 0)` | [`MPPUtility::get_unicode_string`] | 返回 `String`（Java 对空数据返回 `""`） |
//! | `value instanceof LocalDateTime` | 匹配 `FieldValue::DateTime` | 严格性与 Java 一致：`java.time.LocalDate`（Rust 的 `FieldValue::Date`）不通过 |
//!
//! 跳过的 Java 方法：无。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::cell::RefCell;
use std::rc::Rc;

use crate::custom_field_container::CustomFieldContainer;
use crate::field_value::FieldValue;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_data_field_reader::VarDataFieldReader;

/// 从变长数据块中读时间戳字段。
///
/// 对应 Java: `class TimestampVarDataFieldReader extends VarDataFieldReader`
pub struct TimestampVarDataFieldReader {
    /// 自定义字段容器。
    ///
    /// 对应 Java 字段（继承自基类）: `private final CustomFieldContainer m_customFields`
    custom_fields: Rc<RefCell<CustomFieldContainer>>,
}

impl TimestampVarDataFieldReader {
    /// 构造方法。
    ///
    /// 对应 Java: `public TimestampVarDataFieldReader(CustomFieldContainer customFields)`
    pub fn new(custom_fields: Rc<RefCell<CustomFieldContainer>>) -> Self {
        Self { custom_fields }
    }

    /// 从变长数据块中读取原始时间戳值。
    ///
    /// 对应 Java: `private Object getRawTimestampValue(Var2Data varData, Integer id, Integer type)`
    ///
    /// 512 字节的数据项按 UTF-16LE 字符串解码（Java 原样保留该分支），
    /// 其余数据项长度不小于 4 时按「时间 + 天数」组合时间戳解码。
    fn get_raw_timestamp_value(
        &self,
        var_data: &Var2Data,
        id: Option<i32>,
        type_: Option<i32>,
    ) -> Option<FieldValue> {
        // Java: byte[] data = varData.getByteArray(id, type);
        //       if (data != null) { ... }
        let data = var_data.get_byte_array(id, type_)?;

        if data.len() == 512 {
            // Java: result = MPPUtility.getUnicodeString(data, 0);
            Some(FieldValue::String(MPPUtility::get_unicode_string(&data, 0)))
        } else if data.len() >= 4 {
            // Java: result = MPPUtility.getTimestamp(data, 0);
            MPPUtility::get_timestamp(&data, 0).map(FieldValue::DateTime)
        } else {
            // Java: 长度不足 4 时 result 保持 null
            None
        }
    }
}

impl VarDataFieldReader for TimestampVarDataFieldReader {
    /// 对应 Java: `public CustomFieldContainer getCustomFields()`（Rust 特有访问器，见 trait 说明）
    fn get_custom_fields(&self) -> &Rc<RefCell<CustomFieldContainer>> {
        &self.custom_fields
    }

    /// 读取时间戳数据项。
    ///
    /// 对应 Java: `@Override protected Object readValue(Var2Data varData, Integer id, Integer type)`
    fn read_value(
        &self,
        var_data: &Var2Data,
        id: Option<i32>,
        type_: Option<i32>,
    ) -> Option<FieldValue> {
        // Java: return getRawTimestampValue(varData, id, type);
        self.get_raw_timestamp_value(var_data, id, type_)
    }

    /// 只接受 `LocalDateTime` 取值；其他类型返回 `None`（Java 返回 null）。
    ///
    /// 对应 Java: `@Override protected Object coerceValue(Object value)`
    fn coerce_value(&self, value: Option<&FieldValue>) -> Option<FieldValue> {
        // Java: if (value instanceof LocalDateTime) { return value; } return null;
        match value? {
            FieldValue::DateTime(value) => Some(FieldValue::DateTime(*value)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::var_data_field_reader::test_support::{
        container_with_item, guid_bytes, value_list_data_with_id, var_data,
    };
    use super::*;
    use crate::custom_field_lookup_table::CustomFieldValueItem;
    use chrono::{Days, NaiveDate, NaiveTime};

    /// 数据项类型（测试里统一使用）。
    const TYPE: i32 = 0x12;

    /// 时间戳数据项的期望值：1983-12-31 + 1000 天。
    fn expected_timestamp() -> chrono::NaiveDateTime {
        let date = NaiveDate::from_ymd_opt(1983, 12, 31)
            .and_then(|epoch| epoch.checked_add_days(Days::new(1000)))
            .expect("日期可计算");
        chrono::NaiveDateTime::new(date, NaiveTime::from_hms_opt(0, 0, 0).expect("合法时间"))
    }

    /// 「时间（十分之一分钟，2 字节）+ 天数（2 字节）」形式的时间戳数据项。
    fn timestamp_data() -> Vec<u8> {
        vec![0x00, 0x00, 0xE8, 0x03]
    }

    /// 读取「时间 + 天数」形式的时间戳数据项。
    #[test]
    fn reads_timestamp_value() {
        let var_data = var_data(&[(1, TYPE, timestamp_data())]);
        let reader = TimestampVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(
            reader.read_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::DateTime(expected_timestamp()))
        );
    }

    /// 512 字节的数据项按 UTF-16LE 字符串解码（Java 的特殊分支）。
    #[test]
    fn reads_512_byte_value_as_string() {
        let mut data = vec![0u8; 512];
        // "A" 的 UTF-16LE 表示，其余为 0（第一个双 0 字节对即字符串结尾）
        data[0] = 0x41;

        let var_data = var_data(&[(1, TYPE, data)]);
        let reader = TimestampVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(
            reader.read_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::String("A".to_owned()))
        );
    }

    /// 天数表示 NA（65535）时按 Java 规则返回 `None`（null）。
    #[test]
    fn na_day_count_returns_none() {
        // 时间 = 0，天数 = 65535
        let var_data = var_data(&[(1, TYPE, vec![0x00, 0x00, 0xFF, 0xFF])]);
        let reader = TimestampVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(reader.read_value(&var_data, Some(1), Some(TYPE)), None);
    }

    /// 数据项缺失或长度不足 4 字节时返回 `None`（Java 保持 null）。
    #[test]
    fn missing_or_short_data_returns_none() {
        let var_data = var_data(&[
            (1, TYPE, timestamp_data()),
            (2, TYPE, vec![0x00, 0x00, 0x00]),
            (3, TYPE, Vec::new()),
        ]);
        let reader = TimestampVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(reader.read_value(&var_data, Some(999), Some(TYPE)), None);
        assert_eq!(reader.read_value(&var_data, None, None), None);
        assert_eq!(reader.read_value(&var_data, Some(2), Some(TYPE)), None);
        assert_eq!(reader.read_value(&var_data, Some(3), Some(TYPE)), None);
    }

    /// 取值列表分支：取值项里的 `LocalDateTime` 通过收敛后原样返回。
    #[test]
    fn value_list_item_is_coerced_to_date_time() {
        let mut item = CustomFieldValueItem::new(11);
        item.set_value(Some(FieldValue::DateTime(expected_timestamp())));
        let container = container_with_item(item);

        let var_data = var_data(&[(
            1,
            TYPE,
            value_list_data_with_id(11, guid_bytes(3)),
        )]);

        let reader = TimestampVarDataFieldReader::new(container);
        assert_eq!(
            reader.get_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::DateTime(expected_timestamp()))
        );
    }

    /// `coerce_value`：仅 `LocalDateTime` 取值通过（`LocalDate` 不通过，与 Java 的 `instanceof` 一致）。
    #[test]
    fn coerce_value_accepts_date_time_only() {
        let reader = TimestampVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(
            reader.coerce_value(Some(&FieldValue::DateTime(expected_timestamp()))),
            Some(FieldValue::DateTime(expected_timestamp()))
        );
        assert_eq!(
            reader.coerce_value(Some(&FieldValue::Date(
                NaiveDate::from_ymd_opt(2024, 5, 1).expect("合法日期")
            ))),
            None
        );
        assert_eq!(
            reader.coerce_value(Some(&FieldValue::String("2024-05-01".to_owned()))),
            None
        );
        assert_eq!(reader.coerce_value(None), None);
    }
}
