//! 对应 Java: `org/mpxj/mpp/GroupReader9.java`
//!
//! MPP9 的分组定义读取器：只声明本版本使用的 `Var2Data` 数据类型键（1），其余逻辑全部
//! 继承自 [`GroupReader`]。
//!
//! # 继承的还原（trait 实现）
//!
//! Java 的 `final class GroupReader9 extends GroupReader`。Rust 没有继承，故本类是
//! **实现 [`GroupReader`] trait 的零尺寸结构体**：Java 的 `@Override protected` 方法
//! 对应 trait 的必需方法，Java 继承来的公开方法 `process` 由 trait 的默认方法提供。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `private static final Integer GROUP_DATA = Integer.valueOf(1);` | [`GroupReader9::GROUP_DATA`]（`i32`） | 常量名沿用 Java；Java 的 `Integer` 恒非空 |
//! | `getVarDataType()` 返回 `Integer` | `get_var_data_type()` 返回 `i32` | 同上 |
//!
//! 跳过的 Java 成员：无。
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use crate::mpp::group_reader::GroupReader;

/// MPP9 分组定义读取器。
///
/// 对应 Java: `final class GroupReader9 extends GroupReader`
pub struct GroupReader9;

impl GroupReader9 {
    /// 本版本文件用于读取分组数据的 `Var2Data` 数据类型键。
    ///
    /// 对应 Java: `private static final Integer GROUP_DATA = Integer.valueOf(1);`
    pub const GROUP_DATA: i32 = 1;

    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法（Java 侧未显式声明构造方法）。
    pub fn new() -> Self {
        Self
    }
}

impl Default for GroupReader9 {
    /// 对应 Java: 隐式默认构造方法，等价于 [`GroupReader9::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl GroupReader for GroupReader9 {
    /// 对应 Java: `@Override protected Integer getVarDataType()`
    fn get_var_data_type(&self) -> i32 {
        Self::GROUP_DATA
    }
}

#[cfg(test)]
mod tests {
    use crate::mpp::group_reader::GroupReader;
    use crate::mpp::group_reader9::GroupReader9;

    /// 数据类型键与 Java 一致。
    #[test]
    fn constants_match_java() {
        assert_eq!(GroupReader9::GROUP_DATA, 1);
        assert_eq!(GroupReader9::new().get_var_data_type(), 1);
    }

    /// 默认构造与 `new` 等价。
    #[test]
    fn default_matches_new() {
        assert_eq!(
            GroupReader9::default().get_var_data_type(),
            GroupReader9::new().get_var_data_type()
        );
    }
}
