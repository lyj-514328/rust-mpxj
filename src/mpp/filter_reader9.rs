//! 对应 Java: `org/mpxj/mpp/FilterReader9.java`
//!
//! MPP9 的筛选器定义读取器：只声明本版本使用的 `Var2Data` 数据类型键（1）与条件读取器
//! （[`FilterCriteriaReader9`]），其余逻辑全部继承自 [`FilterReader`]。
//!
//! # 继承的还原（组合 + 转发）
//!
//! Java 的 `final class FilterReader9 extends FilterReader`。Rust 没有继承，故：
//!
//! - 本类**实现 trait [`FilterReader`]**：Java 的 2 个 `@Override protected` 方法逐一对应
//!   trait 的必需方法，Java 继承来的公开方法 `process` 由 trait 的默认方法提供；
//! - Java 的 `private final CriteriaReader m_criteraReader = new FilterCriteriaReader9();`
//!   在 Rust 侧是**按值持有的具体字段** `criteria_reader`，由
//!   [`FilterReader9::get_criteria_reader`] 以 `&dyn CriteriaReader` 转发（与 Java 的
//!   向上转型等价）。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `private static final Integer FILTER_DATA = Integer.valueOf(1);` | [`FilterReader9::FILTER_DATA`]（`i32`） | 常量名沿用 Java；Java 的 `Integer` 恒非空 |
//! | `getVarDataType()` 返回 `Integer` | `get_var_data_type()` 返回 `i32` | 同上 |
//!
//! 跳过的 Java 成员：无。
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use crate::mpp::criteria_reader::CriteriaReader;
use crate::mpp::filter_criteria_reader9::FilterCriteriaReader9;
use crate::mpp::filter_reader::FilterReader;

/// MPP9 筛选器定义读取器。
///
/// 对应 Java: `final class FilterReader9 extends FilterReader`
pub struct FilterReader9 {
    /// 条件读取器。
    ///
    /// 对应 Java 字段: `private final CriteriaReader m_criteraReader = new FilterCriteriaReader9();`
    ///（字段名沿用了 Java 源码里的拼写错误 `m_criteraReader`）
    criteria_reader: FilterCriteriaReader9,
}

impl FilterReader9 {
    /// 本版本文件用于读取筛选器条件数据的 `Var2Data` 数据类型键。
    ///
    /// 对应 Java: `private static final Integer FILTER_DATA = Integer.valueOf(1);`
    pub const FILTER_DATA: i32 = 1;

    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法（Java 侧未显式声明构造方法）。
    pub fn new() -> Self {
        Self {
            criteria_reader: FilterCriteriaReader9::new(),
        }
    }
}

impl Default for FilterReader9 {
    /// 对应 Java: 隐式默认构造方法，等价于 [`FilterReader9::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl FilterReader for FilterReader9 {
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
    use crate::mpp::filter_reader9::FilterReader9;

    /// 数据类型键与条件读取器与 Java 一致。
    #[test]
    fn constants_match_java() {
        let reader = FilterReader9::new();
        assert_eq!(FilterReader9::FILTER_DATA, 1);
        assert_eq!(reader.get_var_data_type(), 1);

        // 条件读取器是 9 版布局：块长 80、起始偏移 20
        let criteria_reader = reader.get_criteria_reader();
        assert_eq!(criteria_reader.get_criteria_block_size(), 80);
        assert_eq!(criteria_reader.get_criteria_start_offset(), 20);
    }

    /// 默认构造与 `new` 等价。
    #[test]
    fn default_matches_new() {
        assert_eq!(
            FilterReader9::default().get_var_data_type(),
            FilterReader9::new().get_var_data_type()
        );
    }
}
