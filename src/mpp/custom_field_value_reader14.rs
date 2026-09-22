//! Corresponds to Java: `org/mpxj/mpp/CustomFieldValueReader14.java`
//!
//! Value list reader for MPP14: selects between two offset layouts based on the project file version (before/after Project 2010).
//!
//! # Reproducing inheritance (composition + forwarding)
//!
//! Java: `class CustomFieldValueReader14 extends CustomFieldValueReader`. Rust has no inheritance, so by convention
//! it is changed to **composition + forwarding**:
//!
//! | Java | Rust |
//! |---|---|
//! | `extends CustomFieldValueReader` | The struct holds a private field `base: CustomFieldValueReader` |
//! | Inherited `process()` / `getTypedValue(...)` / `protected` fields | Forwarded one by one to `base` (see each method's docs) |
//! | The three offsets assigned in the constructor based on `m_properties.getApplicationVersion()` | Call `base.set_*_offset` after construction; the decision logic is the same as Java |
//! | `NumberHelper.getInt(m_properties.getApplicationVersion())` | [`ProjectProperties::get_application_version`] (`Option<i32>`) + `unwrap_or(0)`, consistent with Java treating `null` as 0 |
//! | `ApplicationVersion.PROJECT_2010` | [`crate::mpp::application_version::ApplicationVersion::PROJECT_2010`] | References this constant directly (during early parallel development a duplicate constant with the same value was defined in this file; it has since been removed) |
//!
//! Skipped Java methods: none (this class has only 1 constructor and no other methods).
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use uuid::Uuid;

use crate::common::mpp_field_type::MppFieldType;
use crate::custom_field_value_data_type::CustomFieldValueDataType;
use crate::field_value::FieldValue;
use crate::mpp::application_version::ApplicationVersion;
use crate::mpp::custom_field_value_reader::CustomFieldValueReader;
use crate::mpp::fixed_data::FixedData;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_meta::VarMeta;
use crate::project_file::ProjectFile;

/// MPP14 value list reader.
///
/// Corresponds to Java: `class CustomFieldValueReader14 extends CustomFieldValueReader`
pub struct CustomFieldValueReader14 {
/// The composed base class instance.
    ///
    /// Corresponds to Java: `extends CustomFieldValueReader`
    base: CustomFieldValueReader,
}

impl CustomFieldValueReader14 {
/// Constructor.
    ///
    /// Corresponds to Java: `public CustomFieldValueReader14(ProjectFile file, Map<UUID, FieldType> lookupTableMap, VarMeta outlineCodeVarMeta, Var2Data outlineCodeVarData, FixedData outlineCodeFixedData, FixedData outlineCodeFixedData2)`
    pub fn new(
        file: &Rc<RefCell<ProjectFile>>,
        lookup_table_map: HashMap<Uuid, MppFieldType>,
        outline_code_var_meta: Rc<dyn VarMeta>,
        outline_code_var_data: Var2Data,
        outline_code_fixed_data: FixedData,
        outline_code_fixed_data2: FixedData,
    ) -> Self {
        // Java: super(file, lookupTableMap, outlineCodeVarMeta, outlineCodeVarData, outlineCodeFixedData, outlineCodeFixedData2);
        let mut base = CustomFieldValueReader::new(
            file,
            lookup_table_map,
            outline_code_var_meta,
            outline_code_var_data,
            outline_code_fixed_data,
            outline_code_fixed_data2,
        );

        // Java: if (NumberHelper.getInt(m_properties.getApplicationVersion()) > ApplicationVersion.PROJECT_2010)
// `NumberHelper.getInt(null)` returns 0, and the Rust side likewise uses `unwrap_or(0)`;
// when the project file is mutably borrowed it likewise degrades to 0 (this case does not exist in Java).
        let application_version = file
            .try_borrow()
            .ok()
            .and_then(|file| file.get_project_properties().get_application_version())
            .unwrap_or(0);

        if application_version > ApplicationVersion::PROJECT_2010 {
            // Java: m_typeOffset = 16; m_fieldOffset = 18; m_parentOffset = 10;
            base.set_type_offset(16);
            base.set_field_offset(18);
            base.set_parent_offset(10);
        } else {
            // Java: m_fieldOffset = 16; m_typeOffset = 32; m_parentOffset = 8;
            base.set_field_offset(16);
            base.set_type_offset(32);
            base.set_parent_offset(8);
        }

        Self { base }
    }

/// Read all value items.
    ///
/// Corresponds to Java: inherited from `CustomFieldValueReader.process()`
    pub fn process(&self) {
        self.base.process();
    }

/// Convert raw bytes into a Java-typed value.
    ///
/// Corresponds to Java: inherited from `CustomFieldValueReader.getTypedValue(...)` (`protected`)
    pub fn get_typed_value(
        &self,
        type_: Option<CustomFieldValueDataType>,
        value: &[u8],
    ) -> Option<FieldValue> {
        self.base.get_typed_value(type_, value)
    }

/// Get the "parent unique ID" offset.
    ///
/// Corresponds to Java: inherited from `CustomFieldValueReader.m_parentOffset` (`protected`)
    pub fn get_parent_offset(&self) -> usize {
        self.base.get_parent_offset()
    }

/// Set the "parent unique ID" offset.
    ///
/// Corresponds to Java: inherited from `CustomFieldValueReader.m_parentOffset` (`protected`)
    pub fn set_parent_offset(&mut self, offset: usize) {
        self.base.set_parent_offset(offset);
    }

/// Get the "value type" offset.
    ///
/// Corresponds to Java: inherited from `CustomFieldValueReader.m_typeOffset` (`protected`)
    pub fn get_type_offset(&self) -> usize {
        self.base.get_type_offset()
    }

/// Set the "value type" offset.
    ///
/// Corresponds to Java: inherited from `CustomFieldValueReader.m_typeOffset` (`protected`)
    pub fn set_type_offset(&mut self, offset: usize) {
        self.base.set_type_offset(offset);
    }

/// Get the "lookup table GUID" offset.
    ///
/// Corresponds to Java: inherited from `CustomFieldValueReader.m_fieldOffset` (`protected`)
    pub fn get_field_offset(&self) -> usize {
        self.base.get_field_offset()
    }

/// Set the "lookup table GUID" offset.
    ///
/// Corresponds to Java: inherited from `CustomFieldValueReader.m_fieldOffset` (`protected`)
    pub fn set_field_offset(&mut self, offset: usize) {
        self.base.set_field_offset(offset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpp::custom_field_value_reader::test_support::{
        empty_fixed_data, var_data_with_meta,
    };

/// Construct a reader for the specified application version.
    fn reader(application_version: Option<i32>) -> CustomFieldValueReader14 {
        let (meta, var_data) = var_data_with_meta(&[]);
        let file = ProjectFile::new();
        file.borrow_mut()
            .get_project_properties_mut()
            .set_application_version(application_version);

        CustomFieldValueReader14::new(
            &file,
            HashMap::new(),
            meta,
            var_data,
            empty_fixed_data(),
            empty_fixed_data(),
        )
    }

/// Before Project 2010 (including 2010 itself): `fieldOffset = 16 / typeOffset = 32 / parentOffset = 8`.
    #[test]
    fn constructor_uses_pre_2013_offsets() {
// The version number comes from Java's ApplicationVersion: 14 = Project 2010 (does not satisfy `> 14`)
        for version in [None, Some(0), Some(14)] {
            let reader = reader(version);
            assert_eq!(reader.get_field_offset(), 16);
            assert_eq!(reader.get_type_offset(), 32);
            assert_eq!(reader.get_parent_offset(), 8);
        }
    }

/// Project 2013 and later: `typeOffset = 16 / fieldOffset = 18 / parentOffset = 10`.
    #[test]
    fn constructor_uses_post_2010_offsets() {
        for version in [Some(15), Some(16)] {
            let reader = reader(version);
            assert_eq!(reader.get_type_offset(), 16);
            assert_eq!(reader.get_field_offset(), 18);
            assert_eq!(reader.get_parent_offset(), 10);
        }
    }

/// The offsets written by the constructor can be overwritten via the forwarding accessors, and `process` forwards to the base class.
    #[test]
    fn forwards_to_base_reader() {
        let mut reader = reader(Some(16));

        reader.set_parent_offset(1);
        reader.set_type_offset(2);
        reader.set_field_offset(3);
        assert_eq!(reader.get_parent_offset(), 1);
        assert_eq!(reader.get_type_offset(), 2);
        assert_eq!(reader.get_field_offset(), 3);

// Empty data: does not panic and produces no value items
        reader.process();

        assert_eq!(
            reader.get_typed_value(Some(CustomFieldValueDataType::Number), &1.5f64.to_le_bytes()),
            Some(FieldValue::Double(1.5))
        );
    }
}
