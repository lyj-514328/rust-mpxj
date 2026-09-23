//! Corresponds to Java: `org/mpxj/mspdi` package (Microsoft Project XML / MSPDI reader)
//!
//! Java class → Rust module:
//!
//! | Java | Rust |
//! |---|---|
//! | `org.mpxj.mspdi.MSPDIReader` | [`mspdi_reader`] |
//! | `org.mpxj.mspdi.DatatypeConverter` | [`datatype_converter`] |
//! | `org.mpxj.mspdi.XsdDuration` | [`xsd_duration`] |
//! | `org.mpxj.mspdi.schema.*` (JAXB generated XML model) | [`schema`] |
//! | `org.mpxj.mspdi.NamespaceFilter` | not ported, see [`xml_node`] |
//! | `org.mpxj.mspdi.SaveVersion` | not ported (writer-only, `MSPDIWriter`) |
//! | `org.mpxj.mspdi.MSPDIWriter` | not ported (writer) |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | JAXB + SAX (`UnmarshalHelper`, `NamespaceFilter`, `JAXBContext`) | [`roxmltree`] + [`xml_node`] | Rust has no JAXB. JAXB ignores unknown elements and maps elements to fields by name, which is exactly what a read-only XML tree does; [`xml_node`] provides the access helpers. `NamespaceFilter` (which rewrites the namespace URI to `http://schemas.microsoft.com/project`) has no counterpart because [`xml_node`] matches on local names only |
//! | `MSPDIWriter`, `SaveVersion` | not ported | This crate is reader-only |
//!
//! # Skipped Java members + reason
//!
//! - `MSPDIWriter`: writer-only, and this crate is reader-only.
//! - `SaveVersion`: an enum used only by `MSPDIWriter`.
//! - `NamespaceFilter`: superseded by [`xml_node`], which matches on local names (see the table).
//!
//! Everything else in the package is ported: `MSPDIReader` → [`mspdi_reader`],
//! `DatatypeConverter` → [`datatype_converter`], `XsdDuration` → [`xsd_duration`], and the JAXB
//! `schema` package → [`schema`].

pub mod datatype_converter;
pub mod mspdi_reader;
pub mod schema;
pub mod xml_node;
pub mod xsd_duration;

// The MSPDI read sections, split by Java line range: each module holds one `impl ReaderContext`
// block (see the file header of [`mspdi_reader`]). They are crate-internal because
// [`mspdi_reader::ReaderContext`] is.
pub(crate) mod reader_assignments;
pub(crate) mod reader_calendars;
pub(crate) mod reader_custom_fields;
pub(crate) mod reader_project_properties;
pub(crate) mod reader_resources;
pub(crate) mod reader_tasks;
