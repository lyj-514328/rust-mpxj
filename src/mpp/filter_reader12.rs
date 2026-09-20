//! 对应 Java: `org/mpxj/mpp/FilterReader12.java`
//!
//! MPP12 的筛选器定义读取器：只声明本版本使用的 `Var2Data` 数据类型键（6）与条件读取器
//! （[`FilterCriteriaReader12`]），其余逻辑全部继承自 [`FilterReader`]。
//!
//! # 继承的还原（组合 + 转发）
//!
//! Java 的 `final class FilterReader12 extends FilterReader`。Rust 侧与
//! [`FilterReader9`](crate::mpp::filter_reader9::FilterReader9) 同样的做法：实现
//! [`FilterReader`] trait，并把 Java 的 `private final CriteriaReader m_criteraReader`
//! 还原为按值持有的具体字段 + `&dyn CriteriaReader` 转发。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `private static final Integer FILTER_DATA = Integer.valueOf(6);` | [`FilterReader12::FILTER_DATA`]（`i32`） | Java 的 `Integer` 恒非空 |
//! | `getVarDataType()` 返回 `Integer` | `get_var_data_type()` 返回 `i32` | 同上 |
//!
//! 跳过的 Java 成员：无。
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use crate::mpp::criteria_reader::CriteriaReader;
use crate::mpp::filter_criteria_reader12::FilterCriteriaReader12;
use crate::mpp::filter_reader::FilterReader;

/// MPP12 筛选器定义读取器。
///
/// 对应 Java: `final class FilterReader12 extends FilterReader`
pub struct FilterReader12 {
    /// 条件读取器。
    ///
    /// 对应 Java 字段: `private final CriteriaReader m_criteraReader = new FilterCriteriaReader12();`
    criteria_reader: FilterCriteriaReader12,
}

impl FilterReader12 {
    /// 本版本文件用于读取筛选器条件数据的 `Var2Data` 数据类型键。
    ///
    /// 对应 Java: `private static final Integer FILTER_DATA = Integer.valueOf(6);`
    pub const FILTER_DATA: i32 = 6;

    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法（Java 侧未显式声明构造方法）。
    pub fn new() -> Self {
        Self {
            criteria_reader: FilterCriteriaReader12::new(),
        }
    }
}

impl Default for FilterReader12 {
    /// 对应 Java: 隐式默认构造方法，等价于 [`FilterReader12::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl FilterReader for FilterReader12 {
    /// 对应 Java: `@Override protected Integer getVarDataType()`
    fn get_var_data_type(&self) -> i32 {
        Self::FILTER_DATA
    }

    /// 对应 Java: `@Override protected CriteriaReader getCriteriaReader()`
    fn get_criteria_reader(&self) -> &dyn CriteriaReader {
        &self.criteria_reader
    }
}

#[cfg(test)]
mod tests {
    use crate::mpp::filter_reader::FilterReader;
    use crate::mpp::filter_reader12::FilterReader12;

    /// 数据类型键与条件读取器与 Java 一致。
    #[test]
    fn constants_match_java() {
        let reader = FilterReader12::new();
        assert_eq!(FilterReader12::FILTER_DATA, 6);
        assert_eq!(reader.get_var_data_type(), 6);

        let criteria_reader = reader.get_criteria_reader();
        assert_eq!(criteria_reader.get_criteria_block_size(), 80);
        assert_eq!(criteria_reader.get_criteria_start_offset(), 20);
    }

    /// 默认构造与 `new` 等价。
    #[test]
    fn default_matches_new() {
        assert_eq!(
            FilterReader12::default().get_var_data_type(),
            FilterReader12::new().get_var_data_type()
        );
    }
}
