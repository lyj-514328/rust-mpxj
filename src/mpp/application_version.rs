//! 对应 Java: `org/mpxj/mpp/ApplicationVersion.java`
//!
//! 表示 Microsoft Project 各版本号的常量，供 MPP reader 判断文件由哪个版本写出
//! （例如「2013 及以后」的元数据布局与 2010 不同）。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `public class ApplicationVersion` + `public static final int` | 无字段结构体 [`ApplicationVersion`] + 关联常量 | Rust 没有静态字段，用关联常量表达；Java 的隐式公开构造方法不再提供（本类在 Java 里也只是一个常量容器，从不实例化） |
//!
//! 跳过的 Java 成员：无（类中只有 8 个常量，全部逐一翻译）。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

/// Microsoft Project 版本常量（取值为 MPP 文件中的应用版本号）。
///
/// 对应 Java: `public class ApplicationVersion`
pub struct ApplicationVersion;

impl ApplicationVersion {
    /// Project 2016。
    ///
    /// 对应 Java: `public static final int PROJECT_2016 = 16`
    pub const PROJECT_2016: i32 = 16;

    /// Project 2013。
    ///
    /// 对应 Java: `public static final int PROJECT_2013 = 15`
    pub const PROJECT_2013: i32 = 15;

    /// Project 2010。
    ///
    /// 对应 Java: `public static final int PROJECT_2010 = 14`
    pub const PROJECT_2010: i32 = 14;

    /// Project 2007。
    ///
    /// 对应 Java: `public static final int PROJECT_2007 = 12`
    pub const PROJECT_2007: i32 = 12;

    /// Project 2003。
    ///
    /// 对应 Java: `public static final int PROJECT_2003 = 11`
    pub const PROJECT_2003: i32 = 11;

    /// Project 2002。
    ///
    /// 对应 Java: `public static final int PROJECT_2002 = 10`
    pub const PROJECT_2002: i32 = 10;

    /// Project 2000。
    ///
    /// 对应 Java: `public static final int PROJECT_2000 = 9`
    pub const PROJECT_2000: i32 = 9;

    /// Project 98。
    ///
    /// 对应 Java: `public static final int PROJECT_98 = 8`
    pub const PROJECT_98: i32 = 8;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 常量取值与 Java 源码逐项一致（顺序也与 Java 的声明顺序一致）。
    #[test]
    fn constants_match_java_values() {
        assert_eq!(ApplicationVersion::PROJECT_2016, 16);
        assert_eq!(ApplicationVersion::PROJECT_2013, 15);
        assert_eq!(ApplicationVersion::PROJECT_2010, 14);
        assert_eq!(ApplicationVersion::PROJECT_2007, 12);
        assert_eq!(ApplicationVersion::PROJECT_2003, 11);
        assert_eq!(ApplicationVersion::PROJECT_2002, 10);
        assert_eq!(ApplicationVersion::PROJECT_2000, 9);
        assert_eq!(ApplicationVersion::PROJECT_98, 8);
    }

    /// 「比 2010 更新」的判定方式（reader 中 `> ApplicationVersion.PROJECT_2010`）。
    #[test]
    fn version_ordering_is_usable_for_comparisons() {
        assert!(ApplicationVersion::PROJECT_2013 > ApplicationVersion::PROJECT_2010);
        assert!(ApplicationVersion::PROJECT_2016 > ApplicationVersion::PROJECT_2013);
        assert!(!(ApplicationVersion::PROJECT_2010 > ApplicationVersion::PROJECT_2010));
    }
}
