//! 对应 Java: `org/mpxj/GenericCriteriaPrompt.java`
//!
//! 未移植的 Java 方法：无（本类全部方法均已翻译）。
//!
//! 跳过/未移植的 Java 机制：反射、JDBC、XML/JAXB、GUI、`javax.*`、`Serializable` 均未使用。
//!
//! # 与 Java 的差异
//!
//! - **`equals` / `hashCode`**：Java 未覆写，即按对象同一性比较；但本类的实例被用作
//!   `Map<GenericCriteriaPrompt, Object>`（筛选提示的应答表）的键。Rust 侧改为派生
//!   [`PartialEq`] / [`Eq`] / [`Hash`]（按 `type` + `prompt` 值比较），从而可用作
//!   `HashMap` 的键。差异：Java 中「用新构造的同内容实例查表」查不到，Rust 中能查到
//!   —— 语义更宽松，不会漏掉应答。
//! - **`m_prompt`**：Java 允许传入 `null`，Rust 侧用 `Option<String>` 表达；
//!   [`Display`] 复刻 Java 字符串拼接把 `null` 渲染为 `"null"` 的行为。

use std::fmt::{self, Display, Formatter};

use crate::data_type::DataType;

/// 筛选条件中向用户提出的提示（prompt）。
///
/// 对应 Java: `public final class GenericCriteriaPrompt`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericCriteriaPrompt {
    /// 应答值的数据类型。
    ///
    /// 对应 Java: `private final DataType m_type;`
    m_type: DataType,
    /// 提示文本。
    ///
    /// 对应 Java: `private final String m_prompt;`
    m_prompt: Option<String>,
}

impl GenericCriteriaPrompt {
    /// 构造方法。
    ///
    /// 对应 Java: `public GenericCriteriaPrompt(DataType type, String prompt)`
    pub fn new(prompt_type: DataType, prompt: Option<String>) -> Self {
        Self {
            m_type: prompt_type,
            m_prompt: prompt,
        }
    }

    /// 取应答值的数据类型。
    ///
    /// 对应 Java: `public DataType getType()`
    pub fn get_type(&self) -> DataType {
        self.m_type
    }

    /// 取提示文本。
    ///
    /// 对应 Java: `public String getPrompt()`
    pub fn get_prompt(&self) -> Option<&str> {
        self.m_prompt.as_deref()
    }
}

impl Display for GenericCriteriaPrompt {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java: `"PROMPT(" + m_prompt + ")"`，`null` 渲染为 `"null"`。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "PROMPT({})", self.m_prompt.as_deref().unwrap_or("null"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn accessors_round_trip() {
        let prompt = GenericCriteriaPrompt::new(DataType::String, Some("Enter name".to_owned()));

        assert_eq!(prompt.get_type(), DataType::String);
        assert_eq!(prompt.get_prompt(), Some("Enter name"));
    }

    #[test]
    fn display_matches_java_format() {
        let prompt = GenericCriteriaPrompt::new(DataType::String, Some("Enter name".to_owned()));
        assert_eq!(prompt.to_string(), "PROMPT(Enter name)");

        // Java 字符串拼接把 null 渲染为 "null"
        let prompt = GenericCriteriaPrompt::new(DataType::Numeric, None);
        assert_eq!(prompt.to_string(), "PROMPT(null)");
    }

    #[test]
    fn usable_as_prompt_response_map_key() {
        let prompt = GenericCriteriaPrompt::new(DataType::String, Some("p".to_owned()));
        let mut responses: HashMap<GenericCriteriaPrompt, i32> = HashMap::new();
        responses.insert(prompt.clone(), 1);

        // 同内容的新实例也能查到（Java 按同一性查不到，Rust 按值查得到）
        let lookup = GenericCriteriaPrompt::new(DataType::String, Some("p".to_owned()));
        assert_eq!(responses.get(&lookup), Some(&1));
    }
}
