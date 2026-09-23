//! Corresponds to Java: `org/mpxj/mspdi/schema/Project.java`
//! (`Project.OutlineCodes`, `Project.OutlineCodes.OutlineCode`, `...OutlineCode.Masks`,
//! `...Masks.Mask`, `...OutlineCode.Values` and `...Values.Value`).
//!
//! The MSPDI `<OutlineCodes>` element holds one `<OutlineCode>` per outline code definition; each
//! definition may carry a `<Masks>` element (the code's mask per level) and a `<Values>` element
//! (the hierarchical list of allowed values).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | JAXB unmarshaller builds the six nested classes | the `from_node` constructors read the child elements straight out of a [`roxmltree::Node`] (see [`crate::mspdi::xml_node`]) | Rust has no JAXB; element matching is by local name, in the same order as the Java field declarations |
//! | `OutlineCode`, `Masks`, `Mask`, `Values` and `Value` are nested classes of `Project.OutlineCodes` | they are top level structs in this module | Rust has no nested types; the coordinator declares the module in `mod.rs` |
//! | `Project.outlineCodes` is a `Project.OutlineCodes` wrapper holding one `List` field | [`crate::mspdi::schema::Project::outline_codes`] is a `Vec<OutlineCode>` filled from the `OutlineCode` children of the `OutlineCodes` element | Java's `getOutlineCode()` returns an empty list only when the wrapper is non-null (the reader guards with `!= null`); the flattened Rust field is simply empty when the wrapper element is missing |
//! | `BigInteger` | `i64` | Parsed with a plain `i64` parse (Java's binding is the generic JAXB `xsd:integer` binding, so there is no entry in [`DatatypeConverter`]). A value that does not parse yields `None` instead of throwing the way JAXB would. Note that `Mask.Length` is a `BigInteger` here, unlike `Project.WBSMasks.WBSMask.Length`, which is a `String` |
//! | `UUID` via `Adapter32` (`Guid`) and `Adapter33` (`FieldGUID`) | [`uuid::Uuid`] via [`DatatypeConverter::parse_uuid`] | Same conversion method the adapters call |
//! | a present but empty element is handed to the JAXB adapter as `""` | [`crate::mspdi::xml_node::text`] returns `None` for an element with no character data, so the field stays `None` | Rust-specific branch; it keeps an empty element indistinguishable from an absent one instead of panicking |
//! | Java field `type` (elements `Type`) | `r#type` | `type` is a Rust keyword, so the raw identifier keeps the Java field name |
//! | `getXxx()` / `setXxx()` accessor pairs | public fields | This crate is a reader; the accessors carry no behaviour of their own |
//!
//! # Skipped Java members + reason
//!
//! None. Every field declared by `Project.OutlineCodes` (Java line 17624),
//! `Project.OutlineCodes.OutlineCode` (Java lines 17778-17791), `...OutlineCode.Masks` (Java line
//! 18204), `...Masks.Mask` (Java lines 18277-18280), `...OutlineCode.Values` (Java line 18443) and
//! `...Values.Value` (Java lines 18518-18524) is present. None of these classes has
//! `@XmlAttribute` or `@XmlValue` fields, so `from_node` only reads child elements.

use roxmltree::Node;
use uuid::Uuid;

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

/// Corresponds to Java: `public static class Project.OutlineCodes`.
///
/// The class only exists because of JAXB's element wrapper structure and is kept for parity with
/// Java; [`crate::mspdi::schema::Project`] flattens it to a `Vec<OutlineCode>`.
#[derive(Debug, Default, Clone)]
pub struct OutlineCodes {
    /// Corresponds to Java: `Project.OutlineCodes.outlineCode` (`@XmlElement(name = "OutlineCode")`, `List<Project.OutlineCodes.OutlineCode>`).
    pub outline_code: Vec<OutlineCode>,
}

impl OutlineCodes {
    /// Rust-specific: replaces the JAXB unmarshaller, so there is no Java method to correspond to.
    ///
    /// Reads the `<OutlineCode>` children of the `<OutlineCodes>` element.
    pub fn from_node(node: Node<'_, '_>) -> Self {
        Self {
            outline_code: xml_node::children(node, "OutlineCode")
                .map(OutlineCode::from_node)
                .collect(),
        }
    }
}

/// Corresponds to Java: `public static class Project.OutlineCodes.OutlineCode`.
#[derive(Debug, Default, Clone)]
pub struct OutlineCode {
    /// Corresponds to Java: `OutlineCode.guid` (`@XmlElement(name = "Guid", required = true)`, `UUID` via `Adapter32`).
    pub guid: Option<Uuid>,
    /// Corresponds to Java: `OutlineCode.fieldID` (`@XmlElement(name = "FieldID")`, `String`).
    pub field_id: Option<String>,
    /// Corresponds to Java: `OutlineCode.fieldName` (`@XmlElement(name = "FieldName")`, `String`).
    pub field_name: Option<String>,
    /// Corresponds to Java: `OutlineCode.alias` (`@XmlElement(name = "Alias")`, `String`).
    pub alias: Option<String>,
    /// Corresponds to Java: `OutlineCode.phoneticAlias` (`@XmlElement(name = "PhoneticAlias")`, `String`).
    pub phonetic_alias: Option<String>,
    /// Corresponds to Java: `OutlineCode.values` (`@XmlElement(name = "Values")`, `Project.OutlineCodes.OutlineCode.Values`).
    pub values: Option<Values>,
    /// Corresponds to Java: `OutlineCode.enterprise` (`@XmlElement(name = "Enterprise")`, `Boolean` via `Adapter1`).
    pub enterprise: Option<bool>,
    /// Corresponds to Java: `OutlineCode.enterpriseOutlineCodeAlias` (`@XmlElement(name = "EnterpriseOutlineCodeAlias")`, `BigInteger`).
    pub enterprise_outline_code_alias: Option<i64>,
    /// Corresponds to Java: `OutlineCode.resourceSubstitutionEnabled` (`@XmlElement(name = "ResourceSubstitutionEnabled")`, `Boolean` via `Adapter1`).
    pub resource_substitution_enabled: Option<bool>,
    /// Corresponds to Java: `OutlineCode.leafOnly` (`@XmlElement(name = "LeafOnly")`, `Boolean` via `Adapter1`).
    pub leaf_only: Option<bool>,
    /// Corresponds to Java: `OutlineCode.allLevelsRequired` (`@XmlElement(name = "AllLevelsRequired")`, `Boolean` via `Adapter1`).
    pub all_levels_required: Option<bool>,
    /// Corresponds to Java: `OutlineCode.onlyTableValuesAllowed` (`@XmlElement(name = "OnlyTableValuesAllowed")`, `Boolean` via `Adapter1`).
    pub only_table_values_allowed: Option<bool>,
    /// Corresponds to Java: `OutlineCode.showIndent` (`@XmlElement(name = "ShowIndent")`, `Boolean` via `Adapter1`).
    pub show_indent: Option<bool>,
    /// Corresponds to Java: `OutlineCode.masks` (`@XmlElement(name = "Masks")`, `Project.OutlineCodes.OutlineCode.Masks`).
    pub masks: Option<Masks>,
}

impl OutlineCode {
    /// Rust-specific: replaces the JAXB unmarshaller, so there is no Java method to correspond to.
    ///
    /// Reads the child elements of an `<OutlineCode>` element by local name, in the same order as
    /// the Java field declarations.
    pub fn from_node(node: Node<'_, '_>) -> Self {
        Self {
            guid: xml_node::child_text(node, "Guid")
                .and_then(|value| DatatypeConverter::parse_uuid(&value)),
            field_id: xml_node::child_text(node, "FieldID").map(|value| value.into_owned()),
            field_name: xml_node::child_text(node, "FieldName").map(|value| value.into_owned()),
            alias: xml_node::child_text(node, "Alias").map(|value| value.into_owned()),
            phonetic_alias: xml_node::child_text(node, "PhoneticAlias")
                .map(|value| value.into_owned()),
            values: xml_node::child(node, "Values").map(Values::from_node),
            enterprise: xml_node::child_text(node, "Enterprise")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            enterprise_outline_code_alias: xml_node::child_text(node, "EnterpriseOutlineCodeAlias")
                .and_then(|value| parse_integer(&value)),
            resource_substitution_enabled: xml_node::child_text(
                node,
                "ResourceSubstitutionEnabled",
            )
            .map(|value| DatatypeConverter::parse_boolean(&value)),
            leaf_only: xml_node::child_text(node, "LeafOnly")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            all_levels_required: xml_node::child_text(node, "AllLevelsRequired")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            only_table_values_allowed: xml_node::child_text(node, "OnlyTableValuesAllowed")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            show_indent: xml_node::child_text(node, "ShowIndent")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            masks: xml_node::child(node, "Masks").map(Masks::from_node),
        }
    }
}

/// Corresponds to Java: `public static class Project.OutlineCodes.OutlineCode.Masks`.
#[derive(Debug, Default, Clone)]
pub struct Masks {
    /// Corresponds to Java: `Masks.mask` (`@XmlElement(name = "Mask")`, `List<Project.OutlineCodes.OutlineCode.Masks.Mask>`).
    pub mask: Vec<Mask>,
}

impl Masks {
    /// Rust-specific: replaces the JAXB unmarshaller, so there is no Java method to correspond to.
    ///
    /// Reads the `<Mask>` children of the `<Masks>` element.
    pub fn from_node(node: Node<'_, '_>) -> Self {
        Self {
            mask: xml_node::children(node, "Mask")
                .map(Mask::from_node)
                .collect(),
        }
    }
}

/// Corresponds to Java: `public static class Project.OutlineCodes.OutlineCode.Masks.Mask`.
#[derive(Debug, Default, Clone)]
pub struct Mask {
    /// Corresponds to Java: `Mask.level` (`@XmlElement(name = "Level")`, `BigInteger`).
    pub level: Option<i64>,
    /// Corresponds to Java: `Mask.type` (`@XmlElement(name = "Type")`, `BigInteger`).
    pub r#type: Option<i64>,
    /// Corresponds to Java: `Mask.length` (`@XmlElement(name = "Length")`, `BigInteger`).
    pub length: Option<i64>,
    /// Corresponds to Java: `Mask.separator` (`@XmlElement(name = "Separator")`, `String`).
    pub separator: Option<String>,
}

impl Mask {
    /// Rust-specific: replaces the JAXB unmarshaller, so there is no Java method to correspond to.
    ///
    /// Reads the child elements of a `<Mask>` element by local name, in the same order as the Java
    /// field declarations.
    pub fn from_node(node: Node<'_, '_>) -> Self {
        Self {
            level: xml_node::child_text(node, "Level").and_then(|value| parse_integer(&value)),
            r#type: xml_node::child_text(node, "Type").and_then(|value| parse_integer(&value)),
            length: xml_node::child_text(node, "Length").and_then(|value| parse_integer(&value)),
            separator: xml_node::child_text(node, "Separator").map(|value| value.into_owned()),
        }
    }
}

/// Corresponds to Java: `public static class Project.OutlineCodes.OutlineCode.Values`.
#[derive(Debug, Default, Clone)]
pub struct Values {
    /// Corresponds to Java: `Values.value` (`@XmlElement(name = "Value")`, `List<Project.OutlineCodes.OutlineCode.Values.Value>`).
    pub value: Vec<Value>,
}

impl Values {
    /// Rust-specific: replaces the JAXB unmarshaller, so there is no Java method to correspond to.
    ///
    /// Reads the `<Value>` children of the `<Values>` element.
    pub fn from_node(node: Node<'_, '_>) -> Self {
        Self {
            value: xml_node::children(node, "Value")
                .map(Value::from_node)
                .collect(),
        }
    }
}

/// Corresponds to Java: `public static class Project.OutlineCodes.OutlineCode.Values.Value`.
#[derive(Debug, Default, Clone)]
pub struct Value {
    /// Corresponds to Java: `Value.valueID` (`@XmlElement(name = "ValueID")`, `BigInteger`).
    pub value_id: Option<i64>,
    /// Corresponds to Java: `Value.fieldGUID` (`@XmlElement(name = "FieldGUID", required = true)`, `UUID` via `Adapter33`).
    pub field_guid: Option<Uuid>,
    /// Corresponds to Java: `Value.type` (`@XmlElement(name = "Type", required = true)`, `BigInteger`).
    pub r#type: Option<i64>,
    /// Corresponds to Java: `Value.isCollapsed` (`@XmlElement(name = "IsCollapsed")`, `Boolean` via `Adapter1`).
    pub is_collapsed: Option<bool>,
    /// Corresponds to Java: `Value.parentValueID` (`@XmlElement(name = "ParentValueID")`, `BigInteger`).
    pub parent_value_id: Option<i64>,
    /// Corresponds to Java: `Value.value` (`@XmlElement(name = "Value")`, `String`).
    pub value: Option<String>,
    /// Corresponds to Java: `Value.description` (`@XmlElement(name = "Description")`, `String`).
    pub description: Option<String>,
}

impl Value {
    /// Rust-specific: replaces the JAXB unmarshaller, so there is no Java method to correspond to.
    ///
    /// Reads the child elements of a `<Value>` element by local name, in the same order as the Java
    /// field declarations.
    pub fn from_node(node: Node<'_, '_>) -> Self {
        Self {
            value_id: xml_node::child_text(node, "ValueID").and_then(|value| parse_integer(&value)),
            field_guid: xml_node::child_text(node, "FieldGUID")
                .and_then(|value| DatatypeConverter::parse_uuid(&value)),
            r#type: xml_node::child_text(node, "Type").and_then(|value| parse_integer(&value)),
            is_collapsed: xml_node::child_text(node, "IsCollapsed")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            parent_value_id: xml_node::child_text(node, "ParentValueID")
                .and_then(|value| parse_integer(&value)),
            value: xml_node::child_text(node, "Value").map(|value| value.into_owned()),
            description: xml_node::child_text(node, "Description").map(|value| value.into_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Mask, OutlineCodes};
    use roxmltree::Document;
    use uuid::Uuid;

    /// Parses a document and returns the root element's `OutlineCodes`.
    fn outline_codes(xml: &str) -> OutlineCodes {
        let document = Document::parse(xml).unwrap();
        OutlineCodes::from_node(document.root_element())
    }

    /// The scalar fields, and the `Masks`/`Values` sub-elements, are read by local name.
    #[test]
    fn reads_outline_code_fields() {
        let codes = outline_codes(
            r#"<OutlineCodes>
                 <OutlineCode>
                   <Guid>1f2e3d4c-5b6a-4978-8a9b-0c1d2e3f4a5b</Guid>
                   <FieldID>188743731</FieldID>
                   <FieldName>Code</FieldName>
                   <Alias>Code</Alias>
                   <PhoneticAlias>Code</PhoneticAlias>
                   <Enterprise>0</Enterprise>
                   <EnterpriseOutlineCodeAlias>0</EnterpriseOutlineCodeAlias>
                   <ResourceSubstitutionEnabled>0</ResourceSubstitutionEnabled>
                   <LeafOnly>0</LeafOnly>
                   <AllLevelsRequired>0</AllLevelsRequired>
                   <OnlyTableValuesAllowed>0</OnlyTableValuesAllowed>
                   <ShowIndent>1</ShowIndent>
                   <Masks>
                     <Mask><Level>1</Level><Type>0</Type><Length>2</Length><Separator>.</Separator></Mask>
                     <Mask><Level>2</Level><Type>0</Type><Length>3</Length><Separator>/</Separator></Mask>
                   </Masks>
                   <Values>
                     <Value>
                       <ValueID>0</ValueID>
                       <FieldGUID>1f2e3d4c-5b6a-4978-8a9b-0c1d2e3f4a5b</FieldGUID>
                       <Type>0</Type>
                       <IsCollapsed>0</IsCollapsed>
                       <ParentValueID>1</ParentValueID>
                       <Value>Alpha</Value>
                       <Description>first</Description>
                     </Value>
                     <Value>
                       <ValueID>1</ValueID>
                       <FieldGUID>1f2e3d4c-5b6a-4978-8a9b-0c1d2e3f4a5c</FieldGUID>
                       <Type>0</Type>
                       <Value>Beta</Value>
                     </Value>
                   </Values>
                 </OutlineCode>
               </OutlineCodes>"#,
        );

        assert_eq!(codes.outline_code.len(), 1);
        let code = &codes.outline_code[0];
        assert_eq!(
            code.guid,
            Some(Uuid::parse_str("1f2e3d4c-5b6a-4978-8a9b-0c1d2e3f4a5b").unwrap())
        );
        assert_eq!(code.field_id.as_deref(), Some("188743731"));
        assert_eq!(code.field_name.as_deref(), Some("Code"));
        assert_eq!(code.alias.as_deref(), Some("Code"));
        assert_eq!(code.phonetic_alias.as_deref(), Some("Code"));
        assert_eq!(code.enterprise, Some(false));
        assert_eq!(code.enterprise_outline_code_alias, Some(0));
        assert_eq!(code.resource_substitution_enabled, Some(false));
        assert_eq!(code.leaf_only, Some(false));
        assert_eq!(code.all_levels_required, Some(false));
        assert_eq!(code.only_table_values_allowed, Some(false));
        assert_eq!(code.show_indent, Some(true));

        let masks = code.masks.as_ref().expect("Masks element is present");
        assert_eq!(masks.mask.len(), 2);
        assert_eq!(masks.mask[0].level, Some(1));
        assert_eq!(masks.mask[0].length, Some(2));
        assert_eq!(masks.mask[0].separator.as_deref(), Some("."));
        assert_eq!(masks.mask[1].level, Some(2));
        assert_eq!(masks.mask[1].length, Some(3));
        assert_eq!(masks.mask[1].separator.as_deref(), Some("/"));

        let values = code.values.as_ref().expect("Values element is present");
        assert_eq!(values.value.len(), 2);
        assert_eq!(values.value[0].value_id, Some(0));
        assert_eq!(
            values.value[0].field_guid,
            Some(Uuid::parse_str("1f2e3d4c-5b6a-4978-8a9b-0c1d2e3f4a5b").unwrap())
        );
        assert_eq!(values.value[0].r#type, Some(0));
        assert_eq!(values.value[0].is_collapsed, Some(false));
        assert_eq!(values.value[0].parent_value_id, Some(1));
        assert_eq!(values.value[0].value.as_deref(), Some("Alpha"));
        assert_eq!(values.value[0].description.as_deref(), Some("first"));
        assert_eq!(values.value[1].value_id, Some(1));
        assert_eq!(values.value[1].value.as_deref(), Some("Beta"));
        // Absent optional elements.
        assert_eq!(values.value[1].is_collapsed, None);
        assert_eq!(values.value[1].parent_value_id, None);
        assert_eq!(values.value[1].description, None);
    }

    /// Missing sub-elements leave the fields as `None`, and a missing wrapper leaves an empty `Vec`.
    #[test]
    fn reads_absent_elements() {
        let codes = outline_codes(
            "<OutlineCodes><OutlineCode><FieldName>Task</FieldName></OutlineCode></OutlineCodes>",
        );
        assert_eq!(codes.outline_code.len(), 1);
        let code = &codes.outline_code[0];
        assert_eq!(code.field_name.as_deref(), Some("Task"));
        assert_eq!(code.guid, None);
        assert_eq!(code.field_id, None);
        assert!(code.masks.is_none());
        assert!(code.values.is_none());

        assert!(outline_codes("<OutlineCodes/>").outline_code.is_empty());

        let document = Document::parse("<Mask><Type>1</Type></Mask>").unwrap();
        let mask = Mask::from_node(document.root_element());
        assert_eq!(mask.r#type, Some(1));
        assert_eq!(mask.level, None);
        assert_eq!(mask.length, None);
        assert_eq!(mask.separator, None);
    }
}
