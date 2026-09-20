//! 对应 Java: `org/mpxj/mpp/FontBase.java`
//!
//! MS Project 的标准字体定义分成两部分：基础字体（名称 + 字号）与施加其上的样式
//! （颜色 / 粗体 / 斜体 …）。本类表示其中的**基础字体**信息。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `public class FontBase` | 结构体 [`FontBase`] | 1:1 移植 |
//! | `FontBase(Integer index, String name, int size)` | [`FontBase::new`]（`index: Option<i32>`、`name: String`、`size: i32`） | Java 的 `Integer` 可能为 `null` → `Option<i32>`；`name` 在本仓的三处构造点（`MPP9Reader` / `MPP12Reader` / `MPP14Reader` 的字体表解析）都传入非空串，故 Rust 侧按非可空的 `String` 处理（Java 侧理论上可传 `null`，但没有调用点这样做） |
//! | `String getName()` | `&str` | 借用返回，语义与 Java 相同 |
//! | `Integer getIndex()` | `Option<i32>` | 「`null` → `Option<T>`」约定；Java 侧它常被当作 `Map` 的 key 使用（`m_fontBases.put(fontBase.getIndex(), fontBase)`） |
//! | `toString()` | `Display` | 输出格式与 Java 完全一致：`[FontBase name=... size=...]` |
//!
//! 未移植 / 跳过：无（JDBC / XML-JAXB / GUI / `javax.*` / `Serializable` 等机制本类未使用）。

use std::fmt::{self, Display, Formatter};

/// 基础字体（名称 + 字号）。
///
/// 对应 Java: `public class FontBase`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontBase {
    /// 该字体在文件字体表中的序号。
    ///
    /// 对应 Java: `private final Integer m_index`
    index: Option<i32>,
    /// 字体名。
    ///
    /// 对应 Java: `private final String m_name`
    name: String,
    /// 字号。
    ///
    /// 对应 Java: `private final int m_size`
    size: i32,
}

impl FontBase {
    /// 构造函数。
    ///
    /// 对应 Java: `public FontBase(Integer index, String name, int size)`
    pub fn new(index: Option<i32>, name: String, size: i32) -> FontBase {
        FontBase { index, name, size }
    }

    /// 取字体名。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// 取字号。
    ///
    /// 对应 Java: `public int getSize()`
    pub fn get_size(&self) -> i32 {
        self.size
    }

    /// 取该基础字体在文件中的序号。
    ///
    /// 对应 Java: `public Integer getIndex()`
    pub fn get_index(&self) -> Option<i32> {
        self.index
    }
}

impl Display for FontBase {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 只输出名称与字号（不输出序号）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[FontBase name={} size={}]", self.name, self.size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// getter 与 Java 一致。
    #[test]
    fn accessors_match_java() {
        let font = FontBase::new(Some(3), "Arial".to_owned(), 12);
        assert_eq!(font.get_name(), "Arial");
        assert_eq!(font.get_size(), 12);
        assert_eq!(font.get_index(), Some(3));
    }

    /// `index` 可以为 `None`（Java 的 `Integer` 可为 null）。
    #[test]
    fn index_may_be_missing() {
        let font = FontBase::new(None, "Arial".to_owned(), 12);
        assert_eq!(font.get_index(), None);
    }

    /// toString 格式与 Java 一致。
    #[test]
    fn to_string_matches_java() {
        let font = FontBase::new(Some(0), "Arial".to_owned(), 10);
        assert_eq!(font.to_string(), "[FontBase name=Arial size=10]");
    }

    /// 空名称不 panic（Java 的 `getUnicodeString` 不会返回 null，但可能返回空串）。
    #[test]
    fn empty_name_is_allowed() {
        let font = FontBase::new(None, String::new(), 0);
        assert_eq!(font.get_name(), "");
        assert_eq!(font.to_string(), "[FontBase name= size=0]");
    }
}
