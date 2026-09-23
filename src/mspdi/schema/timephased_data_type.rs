//! Corresponds to Java: `org/mpxj/mspdi/schema/TimephasedDataType.java`.
//!
//! `TimephasedDataType` is a top level JAXB generated class (not a nested class of `Project`): it is
//! the `<TimephasedData>` element that appears inside tasks, resources and assignments. Each
//! instance describes one time phased data block: a type, the owning UID, a start/finish range, a
//! unit and the value itself.
//!
//! # Where this class is referenced
//!
//! | Java | Rust |
//! |---|---|
//! | `Project.Tasks.Task.timephasedData` (`List<TimephasedDataType>`) | `crate::mspdi::schema::tasks::task::Task` |
//! | `Project.Resources.Resource.timephasedData` | `crate::mspdi::schema::resources::resource::Resource` |
//! | `Project.Assignments.Assignment.timephasedData` | `crate::mspdi::schema::assignments::assignment::Assignment` |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | JAXB unmarshaller builds `TimephasedDataType` | [`TimephasedDataType::from_node`] reads the child elements straight out of a [`roxmltree::Node`] (see [`crate::mspdi::xml_node`]) | Rust has no JAXB; element matching is by local name, in the same order as the Java field declarations |
//! | `BigInteger` fields `Type`, `UID` and `Unit` | `i64` | Parsed with a plain `i64` parse (Java's binding is the generic JAXB `xsd:integer` binding, so there is no entry in [`DatatypeConverter`]). A value that does not parse yields `None` instead of throwing the way JAXB would. The schema restricts `Type` and `Unit` to an enumeration, but Java keeps the raw `BigInteger` and interprets it in the reader, so the Rust model keeps the raw number too |
//! | `LocalDateTime` via `Adapter4` (`Start`, `Finish`) | [`chrono::NaiveDateTime`] via [`DatatypeConverter::parse_date_time`] | Same conversion method the adapter calls |
//! | a present but empty element is handed to the JAXB adapter as `""` | [`crate::mspdi::xml_node::text`] returns `None` for an element with no character data, so the field stays `None` | Rust-specific branch; it keeps an empty element indistinguishable from an absent one instead of panicking |
//! | Java field `type` (element `Type`) | `r#type` | `type` is a Rust keyword, so the raw identifier keeps the Java field name |
//! | `getXxx()` / `setXxx()` accessor pairs | public fields | This crate is a reader; the accessors carry no behaviour of their own |
//!
//! # Skipped Java members + reason
//!
//! None. Every field declared by `TimephasedDataType` (Java lines 145-150) is present, and the
//! getters/setters are replaced by the public fields. The class has no `@XmlAttribute` or
//! `@XmlValue` fields, so `from_node` only reads child elements.

use chrono::NaiveDateTime;
use roxmltree::Node;

use crate::mspdi::datatype_converter::DatatypeConverter;
use crate::mspdi::xml_node;

/// Rust-specific helper replacing the JAXB binding of `xsd:integer` (Java `BigInteger`).
///
/// XML Schema's whitespace facet for `integer` is "collapse", hence the `trim` before parsing. A
/// value that cannot be parsed yields `None` rather than the `NumberFormatException` JAXB would
/// propagate.
fn parse_integer(value: &str) -> Option<i64> {
    value.trim().parse::<i64>().ok()
}

/// Corresponds to Java: `public class TimephasedDataType`.
#[derive(Debug, Default, Clone)]
pub struct TimephasedDataType {
    /// Corresponds to Java: `TimephasedDataType.type` (`@XmlElement(name = "Type")`, `BigInteger`).
    pub r#type: Option<i64>,
    /// Corresponds to Java: `TimephasedDataType.uid` (`@XmlElement(name = "UID", required = true)`, `BigInteger`).
    pub uid: Option<i64>,
    /// Corresponds to Java: `TimephasedDataType.start` (`@XmlElement(name = "Start")`, `LocalDateTime` via `Adapter4`).
    pub start: Option<NaiveDateTime>,
    /// Corresponds to Java: `TimephasedDataType.finish` (`@XmlElement(name = "Finish")`, `LocalDateTime` via `Adapter4`).
    pub finish: Option<NaiveDateTime>,
    /// Corresponds to Java: `TimephasedDataType.unit` (`@XmlElement(name = "Unit")`, `BigInteger`).
    pub unit: Option<i64>,
    /// Corresponds to Java: `TimephasedDataType.value` (`@XmlElement(name = "Value")`, `String`).
    pub value: Option<String>,
}

impl TimephasedDataType {
    /// Rust-specific: replaces the JAXB unmarshaller, so there is no Java method to correspond to.
    ///
    /// Reads the child elements of a `<TimephasedData>` element by local name, in the same order as
    /// the Java field declarations.
    pub fn from_node(node: Node<'_, '_>) -> Self {
        Self {
            r#type: xml_node::child_text(node, "Type").and_then(|value| parse_integer(&value)),
            uid: xml_node::child_text(node, "UID").and_then(|value| parse_integer(&value)),
            start: xml_node::child_text(node, "Start")
                .and_then(|value| DatatypeConverter::parse_date_time(&value)),
            finish: xml_node::child_text(node, "Finish")
                .and_then(|value| DatatypeConverter::parse_date_time(&value)),
            unit: xml_node::child_text(node, "Unit").and_then(|value| parse_integer(&value)),
            value: xml_node::child_text(node, "Value").map(|value| value.into_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TimephasedDataType;
    use chrono::NaiveDate;
    use roxmltree::Document;

    /// Parses a document and returns the root element's `TimephasedDataType`.
    fn timephased_data(xml: &str) -> TimephasedDataType {
        let document = Document::parse(xml).unwrap();
        TimephasedDataType::from_node(document.root_element())
    }

    /// Every field is read from the element named by the Java `@XmlElement` annotation.
    #[test]
    fn reads_all_fields() {
        let data = timephased_data(
            r#"<TimephasedData>
                 <Type>1</Type>
                 <UID>3</UID>
                 <Start>2024-01-02T08:00:00</Start>
                 <Finish>2024-01-02T17:00:00</Finish>
                 <Unit>5</Unit>
                 <Value>8</Value>
               </TimephasedData>"#,
        );

        assert_eq!(data.r#type, Some(1));
        assert_eq!(data.uid, Some(3));
        assert_eq!(
            data.start,
            NaiveDate::from_ymd_opt(2024, 1, 2)
                .unwrap()
                .and_hms_opt(8, 0, 0)
        );
        assert_eq!(
            data.finish,
            NaiveDate::from_ymd_opt(2024, 1, 2)
                .unwrap()
                .and_hms_opt(17, 0, 0)
        );
        assert_eq!(data.unit, Some(5));
        assert_eq!(data.value.as_deref(), Some("8"));
    }

    /// Absent optional elements leave the fields as `None`.
    #[test]
    fn reads_absent_fields() {
        let data = timephased_data("<TimephasedData><Type>2</Type><UID>7</UID></TimephasedData>");

        assert_eq!(data.r#type, Some(2));
        assert_eq!(data.uid, Some(7));
        assert_eq!(data.start, None);
        assert_eq!(data.finish, None);
        assert_eq!(data.unit, None);
        assert_eq!(data.value, None);
    }
}
