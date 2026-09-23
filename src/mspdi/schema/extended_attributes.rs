//! Corresponds to Java: `org/mpxj/mspdi/schema/Project.java`
//! (`Project.ExtendedAttributes`, `Project.ExtendedAttributes.ExtendedAttribute`,
//! `...ExtendedAttribute.ValueList` and `...ValueList.Value`).
//!
//! The MSPDI `<ExtendedAttributes>` element holds one `<ExtendedAttribute>` per custom field
//! definition; each definition may carry a `<ValueList>` of allowed values.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | JAXB unmarshaller builds the four nested classes | the `from_node` constructors read the child elements straight out of a [`roxmltree::Node`] (see [`crate::mspdi::xml_node`]) | Rust has no JAXB; element matching is by local name, in the same order as the Java field declarations |
//! | `ExtendedAttribute`, `ValueList` and `Value` are nested classes of `Project.ExtendedAttributes` | they are top level structs in this module | Rust has no nested types; the coordinator declares the module in `mod.rs` |
//! | `Project.extendedAttributes` is a `Project.ExtendedAttributes` wrapper holding one `List` field | [`crate::mspdi::schema::Project::extended_attributes`] is a `Vec<ExtendedAttribute>` filled from the `ExtendedAttribute` children of the `ExtendedAttributes` element | Java's `getExtendedAttribute()` returns an empty list only when the wrapper is non-null (the reader guards with `!= null`); the flattened Rust field is simply empty when the wrapper element is missing |
//! | `BigInteger` | `i64` | Parsed with a plain `i64` parse (Java's binding is the generic JAXB `xsd:integer` binding, so there is no entry in [`DatatypeConverter`]). A value that does not parse yields `None` instead of throwing the way JAXB would |
//! | `UUID` via `Adapter31` (`Ltuid`) | [`uuid::Uuid`] via [`DatatypeConverter::parse_uuid`] | Same conversion method the adapter calls |
//! | `String` fields `Guid`, `DefaultGuid`, `SecondaryGuid` | `Option<String>` | They are plain strings in Java as well: only `Ltuid` is bound to a `UUID` |
//! | a present but empty element is handed to the JAXB adapter as `""` | [`crate::mspdi::xml_node::text`] returns `None` for an element with no character data, so the field stays `None` | Rust-specific branch; it keeps an empty element indistinguishable from an absent one instead of panicking |
//! | Java field `_default` (element `Default`) | `r#default` | `default` is a Rust keyword, so the raw identifier keeps the Java field name |
//! | `getXxx()` / `setXxx()` accessor pairs | public fields | This crate is a reader; the accessors carry no behaviour of their own |
//!
//! # Skipped Java members + reason
//!
//! None. Every field declared by `Project.ExtendedAttributes` (Java line 16511),
//! `Project.ExtendedAttributes.ExtendedAttribute` (Java lines 16696-16717),
//! `...ExtendedAttribute.ValueList` (Java line 17329) and `...ValueList.Value` (Java lines
//! 17389-17392) is present. None of these classes has `@XmlAttribute` or `@XmlValue` fields, so
//! `from_node` only reads child elements.

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

/// Corresponds to Java: `public static class Project.ExtendedAttributes`.
///
/// The class only exists because of JAXB's element wrapper structure and is kept for parity with
/// Java; [`crate::mspdi::schema::Project`] flattens it to a `Vec<ExtendedAttribute>`.
#[derive(Debug, Default, Clone)]
pub struct ExtendedAttributes {
    /// Corresponds to Java: `Project.ExtendedAttributes.extendedAttribute` (`@XmlElement(name = "ExtendedAttribute")`, `List<Project.ExtendedAttributes.ExtendedAttribute>`).
    pub extended_attribute: Vec<ExtendedAttribute>,
}

impl ExtendedAttributes {
    /// Rust-specific: replaces the JAXB unmarshaller, so there is no Java method to correspond to.
    ///
    /// Reads the `<ExtendedAttribute>` children of the `<ExtendedAttributes>` element.
    pub fn from_node(node: Node<'_, '_>) -> Self {
        Self {
            extended_attribute: xml_node::children(node, "ExtendedAttribute")
                .map(ExtendedAttribute::from_node)
                .collect(),
        }
    }
}

/// Corresponds to Java: `public static class Project.ExtendedAttributes.ExtendedAttribute`.
#[derive(Debug, Default, Clone)]
pub struct ExtendedAttribute {
    /// Corresponds to Java: `ExtendedAttribute.fieldID` (`@XmlElement(name = "FieldID")`, `String`).
    pub field_id: Option<String>,
    /// Corresponds to Java: `ExtendedAttribute.fieldName` (`@XmlElement(name = "FieldName")`, `String`).
    pub field_name: Option<String>,
    /// Corresponds to Java: `ExtendedAttribute.cfType` (`@XmlElement(name = "CFType")`, `BigInteger`).
    pub cf_type: Option<i64>,
    /// Corresponds to Java: `ExtendedAttribute.guid` (`@XmlElement(name = "Guid")`, `String`).
    pub guid: Option<String>,
    /// Corresponds to Java: `ExtendedAttribute.elemType` (`@XmlElement(name = "ElemType")`, `BigInteger`).
    pub elem_type: Option<i64>,
    /// Corresponds to Java: `ExtendedAttribute.maxMultiValues` (`@XmlElement(name = "MaxMultiValues")`, `BigInteger`).
    pub max_multi_values: Option<i64>,
    /// Corresponds to Java: `ExtendedAttribute.userDef` (`@XmlElement(name = "UserDef")`, `Boolean` via `Adapter1`).
    pub user_def: Option<bool>,
    /// Corresponds to Java: `ExtendedAttribute.alias` (`@XmlElement(name = "Alias")`, `String`).
    pub alias: Option<String>,
    /// Corresponds to Java: `ExtendedAttribute.secondaryPID` (`@XmlElement(name = "SecondaryPID")`, `String`).
    pub secondary_pid: Option<String>,
    /// Corresponds to Java: `ExtendedAttribute.autoRollDown` (`@XmlElement(name = "AutoRollDown")`, `Boolean` via `Adapter1`).
    pub auto_roll_down: Option<bool>,
    /// Corresponds to Java: `ExtendedAttribute.defaultGuid` (`@XmlElement(name = "DefaultGuid")`, `String`).
    pub default_guid: Option<String>,
    /// Corresponds to Java: `ExtendedAttribute.ltuid` (`@XmlElement(name = "Ltuid")`, `UUID` via `Adapter31`).
    pub ltuid: Option<Uuid>,
    /// Corresponds to Java: `ExtendedAttribute.secondaryGuid` (`@XmlElement(name = "SecondaryGuid")`, `String`).
    pub secondary_guid: Option<String>,
    /// Corresponds to Java: `ExtendedAttribute.phoneticAlias` (`@XmlElement(name = "PhoneticAlias")`, `String`).
    pub phonetic_alias: Option<String>,
    /// Corresponds to Java: `ExtendedAttribute.rollupType` (`@XmlElement(name = "RollupType")`, `BigInteger`).
    pub rollup_type: Option<i64>,
    /// Corresponds to Java: `ExtendedAttribute.calculationType` (`@XmlElement(name = "CalculationType")`, `BigInteger`).
    pub calculation_type: Option<i64>,
    /// Corresponds to Java: `ExtendedAttribute.formula` (`@XmlElement(name = "Formula")`, `String`).
    pub formula: Option<String>,
    /// Corresponds to Java: `ExtendedAttribute.restrictValues` (`@XmlElement(name = "RestrictValues")`, `Boolean` via `Adapter1`).
    pub restrict_values: Option<bool>,
    /// Corresponds to Java: `ExtendedAttribute.valuelistSortOrder` (`@XmlElement(name = "ValuelistSortOrder")`, `BigInteger`).
    pub valuelist_sort_order: Option<i64>,
    /// Corresponds to Java: `ExtendedAttribute.appendNewValues` (`@XmlElement(name = "AppendNewValues")`, `Boolean` via `Adapter1`).
    pub append_new_values: Option<bool>,
    /// Corresponds to Java: `ExtendedAttribute._default` (`@XmlElement(name = "Default")`, `String`).
    pub r#default: Option<String>,
    /// Corresponds to Java: `ExtendedAttribute.valueList` (`@XmlElement(name = "ValueList")`, `Project.ExtendedAttributes.ExtendedAttribute.ValueList`).
    pub value_list: Option<ValueList>,
}

impl ExtendedAttribute {
    /// Rust-specific: replaces the JAXB unmarshaller, so there is no Java method to correspond to.
    ///
    /// Reads the child elements of an `<ExtendedAttribute>` element by local name, in the same
    /// order as the Java field declarations.
    pub fn from_node(node: Node<'_, '_>) -> Self {
        Self {
            field_id: xml_node::child_text(node, "FieldID").map(|value| value.into_owned()),
            field_name: xml_node::child_text(node, "FieldName").map(|value| value.into_owned()),
            cf_type: xml_node::child_text(node, "CFType").and_then(|value| parse_integer(&value)),
            guid: xml_node::child_text(node, "Guid").map(|value| value.into_owned()),
            elem_type: xml_node::child_text(node, "ElemType")
                .and_then(|value| parse_integer(&value)),
            max_multi_values: xml_node::child_text(node, "MaxMultiValues")
                .and_then(|value| parse_integer(&value)),
            user_def: xml_node::child_text(node, "UserDef")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            alias: xml_node::child_text(node, "Alias").map(|value| value.into_owned()),
            secondary_pid: xml_node::child_text(node, "SecondaryPID")
                .map(|value| value.into_owned()),
            auto_roll_down: xml_node::child_text(node, "AutoRollDown")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            default_guid: xml_node::child_text(node, "DefaultGuid").map(|value| value.into_owned()),
            ltuid: xml_node::child_text(node, "Ltuid")
                .and_then(|value| DatatypeConverter::parse_uuid(&value)),
            secondary_guid: xml_node::child_text(node, "SecondaryGuid")
                .map(|value| value.into_owned()),
            phonetic_alias: xml_node::child_text(node, "PhoneticAlias")
                .map(|value| value.into_owned()),
            rollup_type: xml_node::child_text(node, "RollupType")
                .and_then(|value| parse_integer(&value)),
            calculation_type: xml_node::child_text(node, "CalculationType")
                .and_then(|value| parse_integer(&value)),
            formula: xml_node::child_text(node, "Formula").map(|value| value.into_owned()),
            restrict_values: xml_node::child_text(node, "RestrictValues")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            valuelist_sort_order: xml_node::child_text(node, "ValuelistSortOrder")
                .and_then(|value| parse_integer(&value)),
            append_new_values: xml_node::child_text(node, "AppendNewValues")
                .map(|value| DatatypeConverter::parse_boolean(&value)),
            r#default: xml_node::child_text(node, "Default").map(|value| value.into_owned()),
            value_list: xml_node::child(node, "ValueList").map(ValueList::from_node),
        }
    }
}

/// Corresponds to Java: `public static class Project.ExtendedAttributes.ExtendedAttribute.ValueList`.
#[derive(Debug, Default, Clone)]
pub struct ValueList {
    /// Corresponds to Java: `ValueList.value` (`@XmlElement(name = "Value", required = true)`, `List<Project.ExtendedAttributes.ExtendedAttribute.ValueList.Value>`).
    pub value: Vec<Value>,
}

impl ValueList {
    /// Rust-specific: replaces the JAXB unmarshaller, so there is no Java method to correspond to.
    ///
    /// Reads the `<Value>` children of the `<ValueList>` element.
    pub fn from_node(node: Node<'_, '_>) -> Self {
        Self {
            value: xml_node::children(node, "Value")
                .map(Value::from_node)
                .collect(),
        }
    }
}

/// Corresponds to Java: `public static class Project.ExtendedAttributes.ExtendedAttribute.ValueList.Value`.
#[derive(Debug, Default, Clone)]
pub struct Value {
    /// Corresponds to Java: `Value.id` (`@XmlElement(name = "ID", required = true)`, `BigInteger`).
    pub id: Option<i64>,
    /// Corresponds to Java: `Value.value` (`@XmlElement(name = "Value")`, `String`).
    pub value: Option<String>,
    /// Corresponds to Java: `Value.description` (`@XmlElement(name = "Description")`, `String`).
    pub description: Option<String>,
    /// Corresponds to Java: `Value.phonetic` (`@XmlElement(name = "Phonetic")`, `String`).
    pub phonetic: Option<String>,
}

impl Value {
    /// Rust-specific: replaces the JAXB unmarshaller, so there is no Java method to correspond to.
    ///
    /// Reads the child elements of a `<Value>` element by local name, in the same order as the Java
    /// field declarations.
    pub fn from_node(node: Node<'_, '_>) -> Self {
        Self {
            id: xml_node::child_text(node, "ID").and_then(|value| parse_integer(&value)),
            value: xml_node::child_text(node, "Value").map(|value| value.into_owned()),
            description: xml_node::child_text(node, "Description").map(|value| value.into_owned()),
            phonetic: xml_node::child_text(node, "Phonetic").map(|value| value.into_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ExtendedAttributes;
    use roxmltree::Document;
    use uuid::Uuid;

    /// Parses a document and returns the root element's `ExtendedAttributes`.
    fn extended_attributes(xml: &str) -> ExtendedAttributes {
        let document = Document::parse(xml).unwrap();
        ExtendedAttributes::from_node(document.root_element())
    }

    /// The scalar fields are read from the elements named by the Java `@XmlElement` annotations.
    #[test]
    fn reads_extended_attribute_fields() {
        let attributes = extended_attributes(
            r#"<ExtendedAttributes>
                 <ExtendedAttribute>
                   <FieldID>188743731</FieldID>
                   <FieldName>Text1</FieldName>
                   <CFType>7</CFType>
                   <Guid>a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d</Guid>
                   <ElemType>0</ElemType>
                   <MaxMultiValues>5</MaxMultiValues>
                   <UserDef>1</UserDef>
                   <Alias>Cost Code</Alias>
                   <SecondaryPID>255885314</SecondaryPID>
                   <AutoRollDown>0</AutoRollDown>
                   <DefaultGuid>a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5e</DefaultGuid>
                   <Ltuid>a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5f</Ltuid>
                   <SecondaryGuid>a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c60</SecondaryGuid>
                   <PhoneticAlias>Cost</PhoneticAlias>
                   <RollupType>0</RollupType>
                   <CalculationType>0</CalculationType>
                   <Formula>1</Formula>
                   <RestrictValues>1</RestrictValues>
                   <ValuelistSortOrder>0</ValuelistSortOrder>
                   <AppendNewValues>1</AppendNewValues>
                   <Default>Unassigned</Default>
                 </ExtendedAttribute>
               </ExtendedAttributes>"#,
        );

        assert_eq!(attributes.extended_attribute.len(), 1);
        let attribute = &attributes.extended_attribute[0];
        assert_eq!(attribute.field_id.as_deref(), Some("188743731"));
        assert_eq!(attribute.field_name.as_deref(), Some("Text1"));
        assert_eq!(attribute.cf_type, Some(7));
        assert_eq!(
            attribute.guid.as_deref(),
            Some("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d")
        );
        assert_eq!(attribute.elem_type, Some(0));
        assert_eq!(attribute.max_multi_values, Some(5));
        assert_eq!(attribute.user_def, Some(true));
        assert_eq!(attribute.alias.as_deref(), Some("Cost Code"));
        assert_eq!(attribute.secondary_pid.as_deref(), Some("255885314"));
        assert_eq!(attribute.auto_roll_down, Some(false));
        assert_eq!(
            attribute.default_guid.as_deref(),
            Some("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5e")
        );
        assert_eq!(
            attribute.ltuid,
            Some(Uuid::parse_str("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5f").unwrap())
        );
        assert_eq!(
            attribute.secondary_guid.as_deref(),
            Some("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c60")
        );
        assert_eq!(attribute.phonetic_alias.as_deref(), Some("Cost"));
        assert_eq!(attribute.rollup_type, Some(0));
        assert_eq!(attribute.calculation_type, Some(0));
        assert_eq!(attribute.formula.as_deref(), Some("1"));
        assert_eq!(attribute.restrict_values, Some(true));
        assert_eq!(attribute.valuelist_sort_order, Some(0));
        assert_eq!(attribute.append_new_values, Some(true));
        assert_eq!(attribute.r#default.as_deref(), Some("Unassigned"));
        // No `ValueList` element, so the optional field stays `None`.
        assert!(attribute.value_list.is_none());
    }

    /// The `ValueList` values are read in document order, and absent elements yield `None`.
    #[test]
    fn reads_value_list_in_order() {
        let attributes = extended_attributes(
            r#"<ExtendedAttributes>
                 <ExtendedAttribute>
                   <FieldID>188743731</FieldID>
                   <ValueList>
                     <Value><ID>0</ID><Value>High</Value><Description>highest</Description><Phonetic>H</Phonetic></Value>
                     <Value><ID>1</ID><Value>Low</Value></Value>
                   </ValueList>
                 </ExtendedAttribute>
               </ExtendedAttributes>"#,
        );

        let attribute = &attributes.extended_attribute[0];
        assert_eq!(attribute.field_name, None);
        let value_list = attribute
            .value_list
            .as_ref()
            .expect("ValueList element is present");
        assert_eq!(value_list.value.len(), 2);
        assert_eq!(value_list.value[0].id, Some(0));
        assert_eq!(value_list.value[0].value.as_deref(), Some("High"));
        assert_eq!(value_list.value[0].description.as_deref(), Some("highest"));
        assert_eq!(value_list.value[0].phonetic.as_deref(), Some("H"));
        assert_eq!(value_list.value[1].id, Some(1));
        assert_eq!(value_list.value[1].value.as_deref(), Some("Low"));
        // Absent optional elements.
        assert_eq!(value_list.value[1].description, None);
        assert_eq!(value_list.value[1].phonetic, None);
    }
}
