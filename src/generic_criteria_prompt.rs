//! Corresponds to Java: `org/mpxj/GenericCriteriaPrompt.java`
//!
//! Unported Java methods: none (all methods of this class have been translated).
//!
//! Skipped/unported Java mechanisms: reflection, JDBC, XML/JAXB, GUI, `javax.*`, `Serializable` are all unused.
//!
//! # Differences from Java
//!
//! - **`equals` / `hashCode`**: Java does not override them, so comparison is by object identity; however, instances of this class are used as
//! keys of a `Map<GenericCriteriaPrompt, Object>` (the filter prompt response table). The Rust side instead derives
//! [`PartialEq`] / [`Eq`] / [`Hash`] (comparing by `type` + `prompt` values), so it can be used as
//! a `HashMap` key. Difference: in Java, "looking up the table with a newly constructed instance of the same content" finds nothing, whereas in Rust it is found
//! — the semantics are looser, so responses are not missed.
//! - **`m_prompt`**: Java allows passing `null`; the Rust side represents it with `Option<String>`;
//! [`Display`] replicates Java's string concatenation behavior of rendering `null` as `"null"`.

use std::fmt::{self, Display, Formatter};

use crate::data_type::DataType;

/// A prompt presented to the user as part of filter criteria.
///
/// Corresponds to Java: `public final class GenericCriteriaPrompt`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericCriteriaPrompt {
/// The data type of the response value.
    ///
    /// Corresponds to Java: `private final DataType m_type;`
    m_type: DataType,
/// The prompt text.
    ///
    /// Corresponds to Java: `private final String m_prompt;`
    m_prompt: Option<String>,
}

impl GenericCriteriaPrompt {
/// Constructor.
    ///
    /// Corresponds to Java: `public GenericCriteriaPrompt(DataType type, String prompt)`
    pub fn new(prompt_type: DataType, prompt: Option<String>) -> Self {
        Self {
            m_type: prompt_type,
            m_prompt: prompt,
        }
    }

/// Get the data type of the response value.
    ///
    /// Corresponds to Java: `public DataType getType()`
    pub fn get_type(&self) -> DataType {
        self.m_type
    }

/// Get the prompt text.
    ///
    /// Corresponds to Java: `public String getPrompt()`
    pub fn get_prompt(&self) -> Option<&str> {
        self.m_prompt.as_deref()
    }
}

impl Display for GenericCriteriaPrompt {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java: `"PROMPT(" + m_prompt + ")"`, rendering `null` as `"null"`.
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

// Java string concatenation renders null as "null"
        let prompt = GenericCriteriaPrompt::new(DataType::Numeric, None);
        assert_eq!(prompt.to_string(), "PROMPT(null)");
    }

    #[test]
    fn usable_as_prompt_response_map_key() {
        let prompt = GenericCriteriaPrompt::new(DataType::String, Some("p".to_owned()));
        let mut responses: HashMap<GenericCriteriaPrompt, i32> = HashMap::new();
        responses.insert(prompt.clone(), 1);

// A new instance with the same content is also found (Java finds nothing by identity, Rust finds it by value)
        let lookup = GenericCriteriaPrompt::new(DataType::String, Some("p".to_owned()));
        assert_eq!(responses.get(&lookup), Some(&1));
    }
}
