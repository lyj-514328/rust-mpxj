//! 对应 Java: `org/mpxj/mpp/EnterpriseCustomFieldDataType.java`
//!
//! 企业自定义字段（Enterprise Custom Field）的「整数类型 ID ↔ [`DataType`]」查找表。
//! ID 的含义取自 Microsoft Project 的 XML 数据交换格式
//! （`https://docs.microsoft.com/en-us/office-project/xml-data-interchange/cftype-element`，
//! 见 Java 源码中的注释）。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `public final class EnterpriseCustomFieldDataType` + 两个静态 `HashMap` | 单位结构体 [`EnterpriseCustomFieldDataType`] + 两个 `match` | Java 的 `DATA_TYPE_MAP` / `ID_MAP` 只是静态查找表，Rust 侧直接以 `match` 表达同样的映射（本 crate 的惯用做法，见 `crate::data_type::DataType::get_instance`），无需引入 `OnceLock` 等额外机制 |
//! | `Integer getIDFromDataType(DataType type)`（可能返回 `null`） | `Option<i32>` | 「`null` → `Option<T>`」约定 |
//! | `DATA_TYPE_MAP.getOrDefault(value, DataType.CUSTOM)` | `match` 的 `_` 分支回落 `DataType::Custom` | 越界 ID 一律回落 CUSTOM |
//! | `ID_MAP.get(type)`（`type` 为 `null` 时返回 `null`） | 形参是 `DataType`（值类型），无 `null` 情形 | `DataType` 在 Rust 侧是 `Copy` 枚举 |
//!
//! 跳过的 Java 方法：无（本类只有上述两个静态方法）。
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use crate::data_type::DataType;

/// 企业自定义字段数据类型的查找表。
///
/// 对应 Java: `public final class EnterpriseCustomFieldDataType`
pub struct EnterpriseCustomFieldDataType;

impl EnterpriseCustomFieldDataType {
    /// 按整数类型 ID 取企业自定义字段的数据类型。
    ///
    /// 对应 Java: `public static DataType getDataTypeFromID(int value)`
    ///
    /// 与 Java 一致：无法识别的 ID 回落 [`DataType::Custom`]。
    pub fn get_data_type_from_id(value: i32) -> DataType {
        match value {
            0 => DataType::Currency,
            1 => DataType::Date,
            2 => DataType::Duration,
            // Java 注释：3 = Finish
            3 => DataType::Date,
            4 => DataType::Boolean,
            5 => DataType::Numeric,
            // Java 注释：6 = Start
            6 => DataType::Date,
            7 => DataType::String,
            // Java: DATA_TYPE_MAP.getOrDefault(Integer.valueOf(value), DataType.CUSTOM)
            _ => DataType::Custom,
        }
    }

    /// 按数据类型取企业自定义字段的整数类型 ID。
    ///
    /// 对应 Java: `public static Integer getIDFromDataType(DataType type)`
    ///
    /// 与 Java 一致：表中没有的类型返回 `None`（Java 返回 `null`）。注意该映射不是
    /// 双向的——`DATE` 的 ID 为 1，而 `3` / `6`（Finish / Start）在反向表中没有对应项。
    pub fn get_id_from_data_type(type_: DataType) -> Option<i32> {
        match type_ {
            DataType::Currency => Some(0),
            DataType::Date => Some(1),
            DataType::Duration => Some(2),
            DataType::Boolean => Some(4),
            DataType::Numeric => Some(5),
            DataType::String => Some(7),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 正向表：8 个 ID 的映射与 Java 的 `DATA_TYPE_MAP` 一致。
    #[test]
    fn known_ids_map_to_the_java_data_types() {
        let cases = [
            (0, DataType::Currency),
            (1, DataType::Date),
            // Finish
            (2, DataType::Duration),
            (3, DataType::Date),
            (4, DataType::Boolean),
            (5, DataType::Numeric),
            // Start
            (6, DataType::Date),
            (7, DataType::String),
        ];

        for (id, data_type) in cases {
            assert_eq!(EnterpriseCustomFieldDataType::get_data_type_from_id(id), data_type);
        }
    }

    /// 越界 / 无法识别的 ID 回落 `CUSTOM`。
    #[test]
    fn unknown_ids_fall_back_to_custom() {
        for id in [-1, 8, 100, i32::MIN, i32::MAX] {
            assert_eq!(
                EnterpriseCustomFieldDataType::get_data_type_from_id(id),
                DataType::Custom
            );
        }
    }

    /// 反向表：6 个可反向的类型与 Java 的 `ID_MAP` 一致。
    #[test]
    fn reverse_lookup_matches_the_java_id_map() {
        let cases = [
            (DataType::Currency, 0),
            (DataType::Date, 1),
            (DataType::Duration, 2),
            (DataType::Boolean, 4),
            (DataType::Numeric, 5),
            (DataType::String, 7),
        ];

        for (data_type, id) in cases {
            assert_eq!(
                EnterpriseCustomFieldDataType::get_id_from_data_type(data_type),
                Some(id)
            );
        }
    }

    /// 表中没有的类型返回 `None`（Java 返回 `null`）。
    #[test]
    fn reverse_lookup_returns_none_for_unmapped_types() {
        for data_type in [
            DataType::Custom,
            DataType::Guid,
            DataType::Integer,
            DataType::Short,
            DataType::Work,
            DataType::Percentage,
            DataType::Units,
        ] {
            assert_eq!(
                EnterpriseCustomFieldDataType::get_id_from_data_type(data_type),
                None
            );
        }
    }

    /// 两张表不是严格互逆的：Finish（3）/ Start（6）都归一到 `DATE`，反向只得 ID 1。
    #[test]
    fn forward_and_reverse_are_not_exact_inverses() {
        for id in [1, 3, 6] {
            assert_eq!(
                EnterpriseCustomFieldDataType::get_id_from_data_type(
                    EnterpriseCustomFieldDataType::get_data_type_from_id(id)
                ),
                Some(1)
            );
        }
    }
}
