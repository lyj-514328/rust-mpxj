//! 对应 Java: `org/mpxj/mpp/DoubleVarDataFieldReader.java`
//!
//! [`VarDataFieldReader`] 的实现：从变长数据块中读一个双精度浮点字段。
//!
//! # 继承的还原
//!
//! Java: `class DoubleVarDataFieldReader extends VarDataFieldReader`。
//! Rust 侧按本批约定：接口（抽象类）→ trait，实现类 → 独立结构体 + `impl`。
//!
//! | Java | Rust |
//! |---|---|
//! | `DoubleVarDataFieldReader extends VarDataFieldReader` | [`DoubleVarDataFieldReader`] 实现 [`VarDataFieldReader`] |
//! | `private final CustomFieldContainer m_customFields`（继承自基类） | 结构体自持 `custom_fields` 字段，经 [`VarDataFieldReader::get_custom_fields`] 暴露 |
//! | `public DoubleVarDataFieldReader(CustomFieldContainer)` | [`DoubleVarDataFieldReader::new`] |
//! | `protected Object readValue(...)` | [`VarDataFieldReader::read_value`] |
//! | `protected Object coerceValue(Object)` | [`VarDataFieldReader::coerce_value`] |
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `NumberHelper.getDouble(varData.getDouble(id, type))` | [`NumberHelper::get_double`] | Java 把基本类型 `double` 装箱成 `Double`；Rust 侧 `f64` 不装箱，故传 `Some(f64)`（恒非 `None`，与 Java 的自动装箱一致，结果恒为该值本身） |
//! | `Object` 返回值 | `Option<FieldValue>` | 读到的值恒为 `FieldValue::Double`；`null` → `None` |
//! | `value instanceof Number` | [`FieldValue::as_double`] | Java 的 `Number` 涵盖 `Integer` / `Short` / `Double`（以及本 crate 未承载的 `Long` / `Float` 等）；Rust 的 `FieldValue` 只有 `Double` / `Integer` / `Short` 三个数值变体，其余变体与 Java 一样返回 `null` → `None` |
//! | `NumberHelper.getDouble(((Number) value).doubleValue())` | [`FieldValue::as_double`] + `FieldValue::Double` | 语义等价：`Integer(3)` → `Double(3.0)` |
//!
//! 跳过的 Java 方法：无。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::number_helper::NumberHelper;
use crate::custom_field_container::CustomFieldContainer;
use crate::field_value::FieldValue;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_data_field_reader::VarDataFieldReader;

/// 从变长数据块中读双精度浮点字段。
///
/// 对应 Java: `class DoubleVarDataFieldReader extends VarDataFieldReader`
pub struct DoubleVarDataFieldReader {
    /// 自定义字段容器。
    ///
    /// 对应 Java 字段（继承自基类）: `private final CustomFieldContainer m_customFields`
    custom_fields: Rc<RefCell<CustomFieldContainer>>,
}

impl DoubleVarDataFieldReader {
    /// 构造方法。
    ///
    /// 对应 Java: `public DoubleVarDataFieldReader(CustomFieldContainer customFields)`
    pub fn new(custom_fields: Rc<RefCell<CustomFieldContainer>>) -> Self {
        Self { custom_fields }
    }
}

impl VarDataFieldReader for DoubleVarDataFieldReader {
    /// 对应 Java: `public CustomFieldContainer getCustomFields()`（Rust 特有访问器，见 trait 说明）
    fn get_custom_fields(&self) -> &Rc<RefCell<CustomFieldContainer>> {
        &self.custom_fields
    }

    /// 按双精度浮点读取数据项（缺数据项时为 0）。
    ///
    /// 对应 Java: `@Override protected Object readValue(Var2Data varData, Integer id, Integer type)`
    fn read_value(
        &self,
        var_data: &Var2Data,
        id: Option<i32>,
        type_: Option<i32>,
    ) -> Option<FieldValue> {
        // Java: return NumberHelper.getDouble(varData.getDouble(id, type));
        Some(FieldValue::Double(NumberHelper::get_double(Some(
            var_data.get_double(id, type_),
        ))))
    }

    /// 把取值列表项的值收敛为双精度浮点；不是数值时返回 `None`（Java 返回 null）。
    ///
    /// 对应 Java: `@Override protected Object coerceValue(Object value)`
    fn coerce_value(&self, value: Option<&FieldValue>) -> Option<FieldValue> {
        // Java: if (value instanceof Number) { return NumberHelper.getDouble(((Number) value).doubleValue()); }
        //       return null;
        value?.as_double().map(FieldValue::Double)
    }
}

#[cfg(test)]
mod tests {
    use super::super::var_data_field_reader::test_support::{container_with_item, var_data};
    use super::*;
    use crate::custom_field_lookup_table::CustomFieldValueItem;

    /// 数据项类型（测试里统一使用）。
    const TYPE: i32 = 0x12;

    /// 读取 8 字节浮点数据项。
    #[test]
    fn reads_double_value() {
        let var_data = var_data(&[(
            1,
            TYPE,
            vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF8, 0x3F],
        )]);
        let reader = DoubleVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(
            reader.read_value(&var_data, Some(1), Some(TYPE)),
            Some(FieldValue::Double(1.5))
        );
    }

    /// 数据项缺失时 `Var2Data.getDouble` 返回 0（Java 同样返回 0，而非 null）。
    #[test]
    fn missing_entry_reads_zero() {
        let var_data = var_data(&[(1, TYPE, vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF8, 0x3F])]);
        let reader = DoubleVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(
            reader.read_value(&var_data, Some(999), Some(TYPE)),
            Some(FieldValue::Double(0.0))
        );
    }

    /// 数据项不足 8 字节 / 位模式为 NaN 时按 0 处理（与 `Var2Data.getDouble` 一致）。
    #[test]
    fn short_or_nan_data_reads_zero() {
        // 4 字节：不足一个 double
        let four_byte_var_data = var_data(&[(1, TYPE, vec![0x00, 0x00, 0xF8, 0x3F])]);
        let reader = DoubleVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));
        assert_eq!(
            reader.read_value(&four_byte_var_data, Some(1), Some(TYPE)),
            Some(FieldValue::Double(0.0))
        );

        // NaN 位模式
        let nan_var_data = var_data(&[(1, TYPE, f64::NAN.to_bits().to_le_bytes().to_vec())]);
        let reader = DoubleVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));
        assert_eq!(
            reader.read_value(&nan_var_data, Some(1), Some(TYPE)),
            Some(FieldValue::Double(0.0))
        );
    }

    /// `coerce_value`：数值取值（含 `Integer` / `Short`）拓宽为 `Double`。
    #[test]
    fn coerce_value_widens_numbers() {
        let reader = DoubleVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(
            reader.coerce_value(Some(&FieldValue::Double(2.5))),
            Some(FieldValue::Double(2.5))
        );
        assert_eq!(
            reader.coerce_value(Some(&FieldValue::Integer(3))),
            Some(FieldValue::Double(3.0))
        );
        assert_eq!(
            reader.coerce_value(Some(&FieldValue::Short(4))),
            Some(FieldValue::Double(4.0))
        );
    }

    /// `coerce_value`：非数值取值与 null（`None`）返回 `None`。
    #[test]
    fn coerce_value_rejects_non_numbers() {
        let reader = DoubleVarDataFieldReader::new(container_with_item(
            CustomFieldValueItem::new(1),
        ));

        assert_eq!(reader.coerce_value(None), None);
        assert_eq!(
            reader.coerce_value(Some(&FieldValue::String("x".to_owned()))),
            None
        );
        assert_eq!(reader.coerce_value(Some(&FieldValue::Boolean(true))), None);
    }
}
