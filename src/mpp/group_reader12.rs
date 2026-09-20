//! 对应 Java: `org/mpxj/mpp/GroupReader12.java`
//!
//! MPP12 的分组定义读取器：只声明本版本使用的 `Var2Data` 数据类型键（6），其余逻辑全部
//! 继承自 [`GroupReader`]。
//!
//! # 继承的还原（trait 实现）
//!
//! Java 的 `final class GroupReader12 extends GroupReader`。Rust 侧与
//! [`GroupReader9`](crate::mpp::group_reader9::GroupReader9) 同样的做法：实现
//! [`GroupReader`] trait；Java 继承来的公开方法 `process` 由 trait 的默认方法提供。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `private static final Integer GROUP_DATA = Integer.valueOf(6);` | [`GroupReader12::GROUP_DATA`]（`i32`） | 常量名沿用 Java；Java 的 `Integer` 恒非空 |
//! | `getVarDataType()` 返回 `Integer` | `get_var_data_type()` 返回 `i32` | 同上 |
//!
//! 跳过的 Java 成员：无。
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use crate::mpp::group_reader::GroupReader;

/// MPP12 分组定义读取器。
///
/// 对应 Java: `final class GroupReader12 extends GroupReader`
pub struct GroupReader12;

impl GroupReader12 {
    /// 本版本文件用于读取分组数据的 `Var2Data` 数据类型键。
    ///
    /// 对应 Java: `private static final Integer GROUP_DATA = Integer.valueOf(6);`
    pub const GROUP_DATA: i32 = 6;

    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法（Java 侧未显式声明构造方法）。
    pub fn new() -> Self {
        Self
    }
}

impl Default for GroupReader12 {
    /// 对应 Java: 隐式默认构造方法，等价于 [`GroupReader12::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl GroupReader for GroupReader12 {
    /// 对应 Java: `@Override protected Integer getVarDataType()`
    fn get_var_data_type(&self) -> i32 {
        Self::GROUP_DATA
    }
}

#[cfg(test)]
mod tests {
    use crate::mpp::group_reader::GroupReader;
    use crate::mpp::group_reader12::GroupReader12;

    /// 数据类型键与 Java 一致。
    #[test]
    fn constants_match_java() {
        assert_eq!(GroupReader12::GROUP_DATA, 6);
        assert_eq!(GroupReader12::new().get_var_data_type(), 6);
    }

    /// 默认构造与 `new` 等价。
    #[test]
    fn default_matches_new() {
        assert_eq!(
            GroupReader12::default().get_var_data_type(),
            GroupReader12::new().get_var_data_type()
        );
    }
}
