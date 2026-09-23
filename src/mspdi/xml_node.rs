//! Rust-specific XML access helpers.
//!
//! Corresponds to Java: `org.mpxj.mspdi.NamespaceFilter` plus the JAXB generated accessors of
//! `org.mpxj.mspdi.schema.Project` (there is no single Java class to point at: in Java the
//! generated bindings expose one `getXxx()` / `getXxxList()` accessor per `@XmlElement`, and the
//! JAXB runtime resolves names against the `http://schemas.microsoft.com/project` namespace).
//!
//! # Why this exists
//!
//! Rust has no JAXB, so [`crate::mspdi::schema`] reads values directly out of a
//! [`roxmltree::Node`]. These helpers are the equivalent of the generated accessors:
//!
//! | Java | Rust |
//! |---|---|
//! | `xml.getBaselineCost()` | [`child_text`]`(node, "BaselineCost")` |
//! | `xml.getBaselineList()` | [`children`]`(node, "Baseline")` |
//! | `xml.getProject()` | [`child`]`(node, "Project")` |
//! | `NamespaceFilter` rewriting the element namespace to `http://schemas.microsoft.com/project` | matching on the **local name** only |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `NamespaceFilter` (`XMLFilterImpl`) rewrites every element's namespace URI to `http://schemas.microsoft.com/project` so that JAXB can match the `@XmlSchema` in `package-info.java` | these helpers compare [`roxmltree::Node::tag_name`]'s local name only, ignoring the namespace | Equivalent for well-formed MSPDI (whose namespace is exactly that URI) and more tolerant of producers that emit a missing or different namespace, which is what `NamespaceFilter` was working around |
//! | JAXB collects an element's character data (including CDATA) into a single value | [`text`] concatenates every text child | `roxmltree`'s own `Node::text` returns the first text child only, which would silently drop data from `<Notes>a<![CDATA[b]]></Notes>` |
//! | Element accessors never allocate | [`text`] / [`child_text`] return [`std::borrow::Cow`] | Borrowed for the common single-text-child case, owned only when the character data is split into several text children |
//!
//! # Skipped Java members + reason
//!
//! None — this module has no Java counterpart (it is the replacement for the JAXB runtime accessors
//! plus `NamespaceFilter`, both of which are covered by the table above).

use std::borrow::Cow;

use roxmltree::Node;

/// Corresponds to Java: the generated accessor for a single child element, e.g.
/// `Project.Tasks.Task.getBaselineCost()`.
///
/// Returns the concatenated character data of the first child element with the given local name,
/// or `None` when there is no such child.
pub fn child_text<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Cow<'a, str>> {
    child(node, name).and_then(text)
}

/// Corresponds to Java: the generated accessor for a single child element, e.g.
/// `Project.Tasks.Task.getProject()`.
///
/// Returns the first child element with the given local name.
pub fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == name)
}

/// Corresponds to Java: the generated accessor for a repeated child element, e.g.
/// `Project.Tasks.Task.getPredecessorLinkList()`.
///
/// Returns every child element with the given local name, in document order. The element name must
/// be a literal, since it is captured by the returned iterator.
pub fn children<'a, 'input>(
    node: Node<'a, 'input>,
    name: &'static str,
) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children()
        .filter(move |child| child.is_element() && child.tag_name().name() == name)
}

/// Corresponds to Java: the character data JAXB collects for an element.
///
/// Returns the concatenation of all text children. JAXB exposes an empty element as an empty
/// string, so an element with no character data returns `Some("")`; a missing element is still
/// represented by [`child_text`] as `None`.
pub fn text<'a, 'input>(node: Node<'a, 'input>) -> Option<Cow<'a, str>> {
    let mut result: Option<Cow<'a, str>> = None;

    for child in node.children().filter(|child| child.is_text()) {
        if let Some(value) = child.text() {
            result = Some(match result {
                Some(existing) => Cow::Owned(format!("{existing}{value}")),
                None => Cow::Borrowed(value),
            });
        }
    }

    result.or_else(|| Some(Cow::Borrowed("")))
}

#[cfg(test)]
mod tests {
    use super::{child, child_text, children, text};
    use roxmltree::Document;

    fn document(xml: &str) -> Document<'_> {
        Document::parse(xml).unwrap()
    }

    /// A single text child is returned as borrowed data.
    #[test]
    fn child_text_reads_a_single_value() {
        let doc = document(r#"<Task><Name>Design</Name><PercentComplete>50</PercentComplete></Task>"#);
        let root = doc.root_element();
        assert_eq!(child_text(root, "Name").as_deref(), Some("Design"));
        assert_eq!(child_text(root, "PercentComplete").as_deref(), Some("50"));
    }

    /// A missing element yields `None` rather than an error (JAXB leaves the field null).
    #[test]
    fn child_text_missing_element() {
        let doc = document("<Task><Name>Design</Name></Task>");
        assert_eq!(child_text(doc.root_element(), "Notes"), None);
    }

    /// Entity references are decoded, matching what JAXB hands to `readTask`.
    #[test]
    fn child_text_decodes_entities() {
        let doc = document("<Task><Notes>a &amp; b &lt;c&gt;</Notes></Task>");
        assert_eq!(child_text(doc.root_element(), "Notes").as_deref(), Some("a & b <c>"));
    }

    /// Character data split by a CDATA section is concatenated.
    #[test]
    fn text_concatenates_cdata() {
        let doc = document("<Task><Notes>a<![CDATA[b]]>c</Notes></Task>");
        assert_eq!(child_text(doc.root_element(), "Notes").as_deref(), Some("abc"));
    }

    /// An empty element is exposed by JAXB as an empty string.
    #[test]
    fn text_of_empty_element() {
        let doc = document("<Task><Notes/></Task>");
        assert_eq!(text(child(doc.root_element(), "Notes").unwrap()).as_deref(), Some(""));
    }

    /// Repeated elements are returned in document order.
    #[test]
    fn children_preserve_document_order() {
        let doc = document(
            "<Task><Baseline><Number>1</Number></Baseline><Baseline><Number>2</Number></Baseline></Task>",
        );
        let numbers: Vec<String> = children(doc.root_element(), "Baseline")
            .map(|node| child_text(node, "Number").unwrap().into_owned())
            .collect();
        assert_eq!(numbers, ["1", "2"]);
    }

    /// Matching ignores the namespace, so a producer that omits it still parses.
    #[test]
    fn matching_ignores_namespace() {
        let with_namespace = document(r#"<Project xmlns="http://schemas.microsoft.com/project"><Title>T</Title></Project>"#);
        assert_eq!(child_text(with_namespace.root_element(), "Title").as_deref(), Some("T"));

        let without_namespace = document("<Project><Title>T</Title></Project>");
        assert_eq!(child_text(without_namespace.root_element(), "Title").as_deref(), Some("T"));
    }
}
