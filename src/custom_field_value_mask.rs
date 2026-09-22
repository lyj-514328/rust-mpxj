//! Corresponds to Java: `org/mpxj/CustomFieldValueMask.java`

use std::fmt::{self, Display, Formatter};

use crate::custom_field_value_data_type::CustomFieldValueDataType;

/// One element of the mask used to define the structured content of a custom field.
///
/// Corresponds to Java: `public class CustomFieldValueMask`
///
/// # Differences from Java
///
/// - Java's `String m_separator` and `CustomFieldValueDataType m_type` may be `null`,
/// and the Rust side represents them with `Option<_>`.
/// - Java does not override `equals` / `hashCode` (it uses reference equality); the Rust side does not derive `PartialEq`,
/// preserving the same "no value comparison" semantics and deriving only `Debug` / `Clone`.
/// - Java implements `toString()` with `String.format`; the Rust side uses `Display`, and `null` fields follow Java's
/// `%s` rule and output `null`.
///
/// Skipped Java mechanisms: none.
///
/// Unported Java methods: none.
#[derive(Debug, Clone)]
pub struct CustomFieldValueMask {
/// Value length.
    ///
    /// Corresponds to Java: `private final int m_length;`
    m_length: i32,
/// Level.
    ///
    /// Corresponds to Java: `private final int m_level;`
    m_level: i32,
/// Separator.
    ///
    /// Corresponds to Java: `private final String m_separator;`
    m_separator: Option<String>,
/// Value type.
    ///
    /// Corresponds to Java: `private final CustomFieldValueDataType m_type;`
    m_type: Option<CustomFieldValueDataType>,
}

impl CustomFieldValueMask {
/// Constructor.
    ///
    /// Corresponds to Java: `public CustomFieldValueMask(int length, int level, String separator, CustomFieldValueDataType type)`
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

/// Get the value length.
    ///
    /// Corresponds to Java: `public int getLength()`
    pub fn get_length(&self) -> i32 {
        self.m_length
    }

/// Get the level.
    ///
    /// Corresponds to Java: `public int getLevel()`
    pub fn get_level(&self) -> i32 {
        self.m_level
    }

/// Get the separator.
    ///
    /// Corresponds to Java: `public String getSeparator()`
    pub fn get_separator(&self) -> Option<&str> {
        self.m_separator.as_deref()
    }

/// Get the value type.
    ///
    /// Corresponds to Java: `public CustomFieldValueDataType getType()`
    pub fn get_type(&self) -> Option<CustomFieldValueDataType> {
        self.m_type
    }
}

impl Display for CustomFieldValueMask {
    /// Corresponds to Java: `@Override public String toString()`
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
