//! Corresponds to Java: `org/mpxj/mspdi/schema/Project.java`
//! (`Project.WBSMasks` and `Project.WBSMasks.WBSMask`).
//!
//! The MSPDI `<WBSMasks>` element carries the WBS code generation settings plus one `<WBSMask>`
//! element per outline level. As in Java, the wrapper class holds its own scalar fields
//! (`VerifyUniqueCodes`, `GenerateCodes`, `Prefix`) next to the `WBSMask` list.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | JAXB unmarshaller builds `Project.WBSMasks` / `Project.WBSMasks.WBSMask` | [`WBSMasks::from_node`] / [`WBSMask::from_node`] read the child elements straight out of a [`roxmltree::Node`] (see [`crate::mspdi::xml_node`]) | Rust has no JAXB; element matching is by local name, in the same order as the Java field declarations |
//! | `WBSMask` is a nested class of `WBSMasks` | `WBSMask` is a top level struct in this module | Rust has no nested types; the coordinator declares the module in `mod.rs` |
//! | `BigInteger` | `i64` | Parsed with a plain `i64` parse (Java's binding is the generic JAXB `xsd:integer` binding, so there is no entry in [`DatatypeConverter`]). A value that does not parse yields `None` instead of throwing the way JAXB would |
//! | a present but empty element is handed to the JAXB adapter as `""` | [`crate::mspdi::xml_node::text`] returns `None` for an element with no character data, so the field stays `None` | Rust-specific branch; it keeps an empty element indistinguishable from an absent one instead of panicking |
//! | `getWBSMask()` lazily creates and returns an empty list | [`WBSMasks::wbs_mask`] is a `Vec` that is empty by default | Same result; the Java reader does not read this element at all, but the data model is shared and therefore complete |
//! | `getXxx()` / `setXxx()` accessor pairs | public fields | This crate is a reader; the accessors carry no behaviour of their own |
//!
//! # Skipped Java members + reason
//!
//! None. Every field declared by `Project.WBSMasks` (Java lines 27665-27668) and
//! `Project.WBSMasks.WBSMask` (Java lines 27819-27822) is present. Neither class has
//! `@XmlAttribute` or `@XmlValue` fields, so `from_node` only reads child elements.

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

/// Corresponds to Java: `public static class Project.WBSMasks`.
#[derive(Debug, Default, Clone)]
pub struct WBSMasks {
    /// Corresponds to Java: `Project.WBSMasks.verifyUniqueCodes` (`@XmlElement(name = "VerifyUniqueCodes", defaultValue = "false")`, `Boolean` via `Adapter1`).
    pub verify_unique_codes: Option<bool>,
    /// Corresponds to Java: `Project.WBSMasks.generateCodes` (`@XmlElement(name = "GenerateCodes", defaultValue = "false")`, `Boolean` via `Adapter1`).
    pub generate_codes: Option<bool>,
    /// Corresponds to Java: `Project.WBSMasks.prefix` (`@XmlElement(name = "Prefix")`, `String`).
    pub prefix: Option<String>,
    /// Corresponds to Java: `Project.WBSMasks.wbsMask` (`@XmlElement(name = "WBSMask")`, `List<Project.WBSMasks.WBSMask>`).
    pub wbs_mask: Vec<WBSMask>,
}

impl WBSMasks {
    /// Rust-specific: replaces the JAXB unmarshaller, so there is no Java method to correspond to.
    ///
    /// Reads the child elements of the `<WBSMasks>` element by local name, in the same order as
    /// the Java field declarations.
    pub fn from_node(node: Node<'_, '_>) -> Self {
        Self {
            verify_unique_codes: xml_node::child_text(node, "VerifyUniqueCodes")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            generate_codes: xml_node::child_text(node, "GenerateCodes")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            prefix: xml_node::child_text(node, "Prefix").map(|value| value.into_owned()),
            wbs_mask: xml_node::children(node, "WBSMask")
                .map(WBSMask::from_node)
                .collect(),
        }
    }
}

/// Corresponds to Java: `public static class Project.WBSMasks.WBSMask`.
///
/// Note that `Length` is a `String` in this class (the analogous `Project.OutlineCodes.OutlineCode.Masks.Mask.Length`
/// is a `BigInteger`); the schema keeps it as a string so the reader can decide what it means.
#[derive(Debug, Default, Clone)]
pub struct WBSMask {
    /// Corresponds to Java: `Project.WBSMasks.WBSMask.level` (`@XmlElement(name = "Level", required = true)`, `BigInteger`).
    pub level: Option<i64>,
    /// Corresponds to Java: `Project.WBSMasks.WBSMask.type` (`@XmlElement(name = "Type", required = true)`, `BigInteger`).
    pub r#type: Option<i64>,
    /// Corresponds to Java: `Project.WBSMasks.WBSMask.length` (`@XmlElement(name = "Length", required = true)`, `String`).
    pub length: Option<String>,
    /// Corresponds to Java: `Project.WBSMasks.WBSMask.separator` (`@XmlElement(name = "Separator", required = true)`, `String`).
    pub separator: Option<String>,
}

impl WBSMask {
    /// Rust-specific: replaces the JAXB unmarshaller, so there is no Java method to correspond to.
    ///
    /// Reads the child elements of a `<WBSMask>` element by local name, in the same order as the
    /// Java field declarations.
    pub fn from_node(node: Node<'_, '_>) -> Self {
        Self {
            level: xml_node::child_text(node, "Level").and_then(|value| parse_integer(&value)),
            r#type: xml_node::child_text(node, "Type").and_then(|value| parse_integer(&value)),
            length: xml_node::child_text(node, "Length").map(|value| value.into_owned()),
            separator: xml_node::child_text(node, "Separator").map(|value| value.into_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{WBSMask, WBSMasks};
    use roxmltree::Document;

    /// Parses a document and returns the root element's `WBSMasks`.
    fn wbs_masks(xml: &str) -> WBSMasks {
        let document = Document::parse(xml).unwrap();
        WBSMasks::from_node(document.root_element())
    }

    /// The scalar fields and the repeated `WBSMask` elements are read in document order.
    #[test]
    fn reads_fields_and_repeated_masks() {
        let masks = wbs_masks(
            r#"<WBSMasks>
                 <VerifyUniqueCodes>0</VerifyUniqueCodes>
                 <GenerateCodes>1</GenerateCodes>
                 <Prefix>WBS</Prefix>
                 <WBSMask><Level>1</Level><Type>0</Type><Length>3</Length><Separator>.</Separator></WBSMask>
                 <WBSMask><Level>2</Level><Type>0</Type><Length>3</Length><Separator>/</Separator></WBSMask>
               </WBSMasks>"#,
        );

        assert_eq!(masks.verify_unique_codes, Some(false));
        assert_eq!(masks.generate_codes, Some(true));
        assert_eq!(masks.prefix.as_deref(), Some("WBS"));
        assert_eq!(masks.wbs_mask.len(), 2);
        assert_eq!(masks.wbs_mask[0].level, Some(1));
        assert_eq!(masks.wbs_mask[0].separator.as_deref(), Some("."));
        assert_eq!(masks.wbs_mask[1].level, Some(2));
        assert_eq!(masks.wbs_mask[1].separator.as_deref(), Some("/"));
    }

    /// Absent elements leave the fields as `None`, and a missing `WBSMask` list yields an empty `Vec`.
    #[test]
    fn reads_absent_fields() {
        let masks = wbs_masks("<WBSMasks><Prefix>Code</Prefix></WBSMasks>");
        assert_eq!(masks.verify_unique_codes, None);
        assert_eq!(masks.generate_codes, None);
        assert_eq!(masks.prefix.as_deref(), Some("Code"));
        assert!(masks.wbs_mask.is_empty());

        let document = Document::parse("<WBSMask><Level>2</Level></WBSMask>").unwrap();
        let mask = WBSMask::from_node(document.root_element());
        assert_eq!(mask.level, Some(2));
        // `Type`, `Length` and `Separator` are absent, so they stay `None`.
        assert_eq!(mask.r#type, None);
        assert_eq!(mask.length, None);
        assert_eq!(mask.separator, None);
    }
}
