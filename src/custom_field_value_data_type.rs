//! 对应 Java: `org/mpxj/CustomFieldValueDataType.java`

use std::fmt::{self, Display, Formatter};

use crate::data_type::DataType;
use crate::mpxj_enum::MpxjEnum;

/// 自定义字段取值项（value item）的数据类型。
///
/// 对应 Java: `public enum CustomFieldValueDataType implements MpxjEnum`
///
/// Java 侧每个常量携带 `(int value, int maskValue, DataType type)` 三个字段：
///
/// | 常量 | `getValue()` | `getMaskValue()` | `getDataType()` |
/// |---|---|---|---|
/// | `DATE` | 4 | 4 | `DataType.DATE` |
/// | `DURATION` | 6 | 6 | `DataType.DURATION` |
/// | `COST` | 9 | 5 | `DataType.CURRENCY` |
/// | `NUMBER` | 15 | 7 | `DataType.NUMERIC` |
/// | `FLAG` | 17 | 17 | `DataType.BOOLEAN` |
/// | `TEXT` | 21 | 3 | `DataType.STRING` |
/// | `FINISH_DATE` | 27 | 9 | `DataType.DATE` |
///
/// Rust 侧 `value` 用显式判别值承载（[`MpxjEnum::value`] 直接返回它），
/// `maskValue` / `type` 由 [`CustomFieldValueDataType::get_mask_value`] 与
/// [`CustomFieldValueDataType::get_data_type`] 用 `match` 返回。
///
/// # 与 Java 的差异：反射建表改为 `match`
///
/// Java 侧用 `EnumHelper.createTypeArray` 反射生成两张表：
/// * `TYPE_VALUES`：长度 `7 + 21 = 28`，按 `getValue()` 作下标；
/// * `MASK_VALUES`：长度 28，按 `getMaskValue()` 作下标。
///
/// Rust 侧不使用反射，改为 [`CustomFieldValueDataType::ALL`] 加 `match`；
/// [`get_instance`](CustomFieldValueDataType::get_instance) 保留 Java 的「下标越界返回
/// `null`」语义（越界或该下标没有对应常量时返回 `None`）。
///
/// 被跳过的 Java 机制：反射（`EnumHelper.createTypeArray`，见上）。
///
/// 未移植的 Java 方法：无。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CustomFieldValueDataType {
    /// 对应 Java: `DATE(4, 4, DataType.DATE)`
    Date = 4,
    /// 对应 Java: `DURATION(6, 6, DataType.DURATION)`
    Duration = 6,
    /// 对应 Java: `COST(9, 5, DataType.CURRENCY)`
    Cost = 9,
    /// 对应 Java: `NUMBER(15, 7, DataType.NUMERIC)`
    Number = 15,
    /// 对应 Java: `FLAG(17, 17, DataType.BOOLEAN)`
    Flag = 17,
    /// 对应 Java: `TEXT(21, 3, DataType.STRING)`
    Text = 21,
    /// 对应 Java: `FINISH_DATE(27, 9, DataType.DATE)`
    FinishDate = 27,
}

impl CustomFieldValueDataType {
    /// Java 侧 `TYPE_VALUES` 数组长度：`EnumHelper.createTypeArray(class, 21)` 生成
    /// `常量个数(7) + 21 = 28` 个槽位。
    const TYPE_VALUES_LEN: i32 = 28;

    /// Java 侧 `MASK_VALUES` 数组长度。
    const MASK_VALUES_LEN: i32 = 28;

    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `values()`（`EnumHelper.createTypeArray` 与静态块都依赖它）。
    pub const ALL: [CustomFieldValueDataType; 7] = [
        CustomFieldValueDataType::Date,
        CustomFieldValueDataType::Duration,
        CustomFieldValueDataType::Cost,
        CustomFieldValueDataType::Number,
        CustomFieldValueDataType::Flag,
        CustomFieldValueDataType::Text,
        CustomFieldValueDataType::FinishDate,
    ];

    /// 根据 `getValue()` 的取值取枚举实例，取不到时返回 `None`。
    ///
    /// 对应 Java: `public static CustomFieldValueDataType getInstance(int type)`
    ///
    /// Java 侧为 `TYPE_VALUES` 数组按下标取值，越界返回 `null`；Rust 侧用 `match`
    /// 等价表达（`0..28` 之间没有对应常量的下标同样返回 `None`）。
    pub fn get_instance(value: i32) -> Option<CustomFieldValueDataType> {
        if !(0..Self::TYPE_VALUES_LEN).contains(&value) {
            return None;
        }

        match value {
            4 => Some(CustomFieldValueDataType::Date),
            6 => Some(CustomFieldValueDataType::Duration),
            9 => Some(CustomFieldValueDataType::Cost),
            15 => Some(CustomFieldValueDataType::Number),
            17 => Some(CustomFieldValueDataType::Flag),
            21 => Some(CustomFieldValueDataType::Text),
            27 => Some(CustomFieldValueDataType::FinishDate),
            _ => None,
        }
    }

    /// 根据 mask 值取枚举实例，取不到时返回 `None`。
    ///
    /// 对应 Java: `public static CustomFieldValueDataType getInstanceByMaskValue(int type)`
    ///
    /// Java 侧为 `MASK_VALUES` 数组按下标取值，越界返回 `null`。
    pub fn get_instance_by_mask_value(value: i32) -> Option<CustomFieldValueDataType> {
        if !(0..Self::MASK_VALUES_LEN).contains(&value) {
            return None;
        }

        match value {
            3 => Some(CustomFieldValueDataType::Text),
            4 => Some(CustomFieldValueDataType::Date),
            5 => Some(CustomFieldValueDataType::Cost),
            6 => Some(CustomFieldValueDataType::Duration),
            7 => Some(CustomFieldValueDataType::Number),
            9 => Some(CustomFieldValueDataType::FinishDate),
            17 => Some(CustomFieldValueDataType::Flag),
            _ => None,
        }
    }

    /// 取 MPXJ 数据类型。
    ///
    /// 对应 Java: `public DataType getDataType()`
    pub fn get_data_type(&self) -> DataType {
        match self {
            CustomFieldValueDataType::Date => DataType::Date,
            CustomFieldValueDataType::Duration => DataType::Duration,
            CustomFieldValueDataType::Cost => DataType::Currency,
            CustomFieldValueDataType::Number => DataType::Numeric,
            CustomFieldValueDataType::Flag => DataType::Boolean,
            CustomFieldValueDataType::Text => DataType::String,
            CustomFieldValueDataType::FinishDate => DataType::Date,
        }
    }

    /// 取 mask 值。
    ///
    /// 对应 Java: `public int getMaskValue()`
    pub fn get_mask_value(&self) -> i32 {
        match self {
            CustomFieldValueDataType::Date => 4,
            CustomFieldValueDataType::Duration => 6,
            CustomFieldValueDataType::Cost => 5,
            CustomFieldValueDataType::Number => 7,
            CustomFieldValueDataType::Flag => 17,
            CustomFieldValueDataType::Text => 3,
            CustomFieldValueDataType::FinishDate => 9,
        }
    }
}

impl MpxjEnum for CustomFieldValueDataType {
    /// 返回该枚举关联的 int 值。
    ///
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for CustomFieldValueDataType {
    /// Java 枚举未覆写 `toString()`，默认返回常量名（如 `"DATE"`）；Rust 侧用
    /// `Display` 提供同样的输出，供 [`crate::custom_field_value_mask::CustomFieldValueMask`]
    /// 的 `toString()` 使用。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CustomFieldValueDataType::Date => "DATE",
            CustomFieldValueDataType::Duration => "DURATION",
            CustomFieldValueDataType::Cost => "COST",
            CustomFieldValueDataType::Number => "NUMBER",
            CustomFieldValueDataType::Flag => "FLAG",
            CustomFieldValueDataType::Text => "TEXT",
            CustomFieldValueDataType::FinishDate => "FINISH_DATE",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_values_round_trip_through_get_instance() {
        for item in CustomFieldValueDataType::ALL {
            assert_eq!(
                CustomFieldValueDataType::get_instance(item.value()),
                Some(item)
            );
        }
    }

    #[test]
    fn get_instance_returns_none_outside_type_values() {
        // 没有对应常量的下标
        assert_eq!(CustomFieldValueDataType::get_instance(0), None);
        assert_eq!(CustomFieldValueDataType::get_instance(5), None);
        assert_eq!(CustomFieldValueDataType::get_instance(26), None);

        // 数组越界
        assert_eq!(CustomFieldValueDataType::get_instance(-1), None);
        assert_eq!(CustomFieldValueDataType::get_instance(28), None);
    }

    #[test]
    fn get_instance_by_mask_value_matches_java_table() {
        assert_eq!(
            CustomFieldValueDataType::get_instance_by_mask_value(3),
            Some(CustomFieldValueDataType::Text)
        );
        assert_eq!(
            CustomFieldValueDataType::get_instance_by_mask_value(4),
            Some(CustomFieldValueDataType::Date)
        );
        assert_eq!(
            CustomFieldValueDataType::get_instance_by_mask_value(5),
            Some(CustomFieldValueDataType::Cost)
        );
        assert_eq!(
            CustomFieldValueDataType::get_instance_by_mask_value(6),
            Some(CustomFieldValueDataType::Duration)
        );
        assert_eq!(
            CustomFieldValueDataType::get_instance_by_mask_value(7),
            Some(CustomFieldValueDataType::Number)
        );
        assert_eq!(
            CustomFieldValueDataType::get_instance_by_mask_value(9),
            Some(CustomFieldValueDataType::FinishDate)
        );
        assert_eq!(
            CustomFieldValueDataType::get_instance_by_mask_value(17),
            Some(CustomFieldValueDataType::Flag)
        );

        assert_eq!(CustomFieldValueDataType::get_instance_by_mask_value(0), None);
        assert_eq!(CustomFieldValueDataType::get_instance_by_mask_value(28), None);
        assert_eq!(CustomFieldValueDataType::get_instance_by_mask_value(-1), None);
    }

    #[test]
    fn data_types_and_mask_values_match_java() {
        let expected = [
            (CustomFieldValueDataType::Date, DataType::Date, 4),
            (CustomFieldValueDataType::Duration, DataType::Duration, 6),
            (CustomFieldValueDataType::Cost, DataType::Currency, 5),
            (CustomFieldValueDataType::Number, DataType::Numeric, 7),
            (CustomFieldValueDataType::Flag, DataType::Boolean, 17),
            (CustomFieldValueDataType::Text, DataType::String, 3),
            (CustomFieldValueDataType::FinishDate, DataType::Date, 9),
        ];

        for (item, data_type, mask_value) in expected {
            assert_eq!(item.get_data_type(), data_type);
            assert_eq!(item.get_mask_value(), mask_value);
        }
    }

    #[test]
    fn display_matches_java_enum_names() {
        assert_eq!(CustomFieldValueDataType::Date.to_string(), "DATE");
        assert_eq!(CustomFieldValueDataType::Text.to_string(), "TEXT");
        assert_eq!(
            CustomFieldValueDataType::FinishDate.to_string(),
            "FINISH_DATE"
        );
    }
}
