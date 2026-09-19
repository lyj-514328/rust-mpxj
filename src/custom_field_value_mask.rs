//! 对应 Java: `org/mpxj/CustomFieldValueMask.java`

use std::fmt::{self, Display, Formatter};

use crate::custom_field_value_data_type::CustomFieldValueDataType;

/// 用于定义自定义字段结构化内容的 mask 中的一个元素。
///
/// 对应 Java: `public class CustomFieldValueMask`
///
/// # 与 Java 的差异
///
/// - Java 的 `String m_separator` 与 `CustomFieldValueDataType m_type` 允许为 `null`，
///   Rust 侧用 `Option<_>` 表达。
/// - Java 未覆写 `equals` / `hashCode`（使用引用相等）；Rust 侧不派生 `PartialEq`，
///   保持同样的「不按值比较」语义，只派生 `Debug` / `Clone`。
/// - Java 用 `String.format` 实现 `toString()`；Rust 侧用 `Display`，`null` 字段按 Java
///   的 `%s` 规则输出 `null`。
///
/// 被跳过的 Java 机制：无。
///
/// 未移植的 Java 方法：无。
#[derive(Debug, Clone)]
pub struct CustomFieldValueMask {
    /// 取值长度。
    ///
    /// 对应 Java: `private final int m_length;`
    m_length: i32,
    /// 层级。
    ///
    /// 对应 Java: `private final int m_level;`
    m_level: i32,
    /// 分隔符。
    ///
    /// 对应 Java: `private final String m_separator;`
    m_separator: Option<String>,
    /// 取值类型。
    ///
    /// 对应 Java: `private final CustomFieldValueDataType m_type;`
    m_type: Option<CustomFieldValueDataType>,
}

impl CustomFieldValueMask {
    /// 构造方法。
    ///
    /// 对应 Java: `public CustomFieldValueMask(int length, int level, String separator, CustomFieldValueDataType type)`
    pub fn new(
        length: i32,
        level: i32,
        separator: Option<String>,
        value_type: Option<CustomFieldValueDataType>,
    ) -> Self {
        Self {
            m_length: length,
            m_level: level,
            m_separator: separator,
            m_type: value_type,
        }
    }

    /// 取取值长度。
    ///
    /// 对应 Java: `public int getLength()`
    pub fn get_length(&self) -> i32 {
        self.m_length
    }

    /// 取层级。
    ///
    /// 对应 Java: `public int getLevel()`
    pub fn get_level(&self) -> i32 {
        self.m_level
    }

    /// 取分隔符。
    ///
    /// 对应 Java: `public String getSeparator()`
    pub fn get_separator(&self) -> Option<&str> {
        self.m_separator.as_deref()
    }

    /// 取取值类型。
    ///
    /// 对应 Java: `public CustomFieldValueDataType getType()`
    pub fn get_type(&self) -> Option<CustomFieldValueDataType> {
        self.m_type
    }
}

impl Display for CustomFieldValueMask {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java: `String.format("[CustomFieldValueMask length=%d level=%d separator=%s type=%s]", ...)`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[CustomFieldValueMask length={} level={} separator={} type={}]",
            self.m_length,
            self.m_level,
            self.m_separator.as_deref().unwrap_or("null"),
            self.m_type
                .map(|value_type| value_type.to_string())
                .unwrap_or_else(|| "null".to_owned())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessors_return_constructor_values() {
        let mask = CustomFieldValueMask::new(
            10,
            2,
            Some("-".to_owned()),
            Some(CustomFieldValueDataType::Text),
        );

        assert_eq!(mask.get_length(), 10);
        assert_eq!(mask.get_level(), 2);
        assert_eq!(mask.get_separator(), Some("-"));
        assert_eq!(mask.get_type(), Some(CustomFieldValueDataType::Text));
    }

    #[test]
    fn null_separator_and_type_are_preserved() {
        let mask = CustomFieldValueMask::new(0, 0, None, None);

        assert_eq!(mask.get_length(), 0);
        assert_eq!(mask.get_level(), 0);
        assert_eq!(mask.get_separator(), None);
        assert_eq!(mask.get_type(), None);
    }

    #[test]
    fn display_matches_java_format() {
        let mask = CustomFieldValueMask::new(
            5,
            1,
            Some(".".to_owned()),
            Some(CustomFieldValueDataType::Duration),
        );

        assert_eq!(
            mask.to_string(),
            "[CustomFieldValueMask length=5 level=1 separator=. type=DURATION]"
        );
    }

    #[test]
    fn display_renders_null_fields_as_java_does() {
        let mask = CustomFieldValueMask::new(3, 0, None, None);

        assert_eq!(
            mask.to_string(),
            "[CustomFieldValueMask length=3 level=0 separator=null type=null]"
        );
    }
}
