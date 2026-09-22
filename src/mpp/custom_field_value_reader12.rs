//! Corresponds to Java: `org/mpxj/mpp/CustomFieldValueReader12.java`
//!
//! MPP12 value list reader: it just sets the three offsets of [`CustomFieldValueReader`] to the MPP12 layout.
//!
//! # Restoring inheritance (composition + forwarding)
//!
//! Java: `class CustomFieldValueReader12 extends CustomFieldValueReader`. Rust has no inheritance, so by convention
//! it is changed to **composition + forwarding**:
//!
//! | Java | Rust |
//! |---|---|
//! | `extends CustomFieldValueReader` | the struct holds a private field `base: CustomFieldValueReader` |
//! | inherited `process()` / `getTypedValue(...)` / `protected` fields | forwarded one by one to `base` (see each method's documentation) |
//! | `m_parentOffset = 8; m_typeOffset = 48; m_fieldOffset = 32;` in the constructor | call `base.set_*_offset` after construction |
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
use crate::mpp::custom_field_value_reader::CustomFieldValueReader;
use crate::mpp::fixed_data::FixedData;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_meta::VarMeta;
use crate::project_file::ProjectFile;

/// MPP12 value list reader.
///
/// Corresponds to Java: `class CustomFieldValueReader12 extends CustomFieldValueReader`
pub struct CustomFieldValueReader12 {
/// The composed base class instance.
    ///
    /// Corresponds to Java: `extends CustomFieldValueReader`
    base: CustomFieldValueReader,
}

impl CustomFieldValueReader12 {
/// Constructor.
    ///
    /// Corresponds to Java: `public CustomFieldValueReader12(ProjectFile file, Map<UUID, FieldType> lookupTableMap, VarMeta outlineCodeVarMeta, Var2Data outlineCodeVarData, FixedData outlineCodeFixedData, FixedData outlineCodeFixedData2)`
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

        // Java: m_parentOffset = 8; m_typeOffset = 48; m_fieldOffset = 32;
        base.set_parent_offset(8);
        base.set_type_offset(48);
        base.set_field_offset(32);

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

/// Get the offset of the "parent unique ID".
    ///
/// Corresponds to Java: inherited from `CustomFieldValueReader.m_parentOffset` (`protected`)
    pub fn get_parent_offset(&self) -> usize {
        self.base.get_parent_offset()
    }

/// Set the offset of the "parent unique ID".
    ///
/// Corresponds to Java: inherited from `CustomFieldValueReader.m_parentOffset` (`protected`)
    pub fn set_parent_offset(&mut self, offset: usize) {
        self.base.set_parent_offset(offset);
    }

/// Get the offset of the "value type".
    ///
/// Corresponds to Java: inherited from `CustomFieldValueReader.m_typeOffset` (`protected`)
    pub fn get_type_offset(&self) -> usize {
        self.base.get_type_offset()
    }

/// Set the offset of the "value type".
    ///
/// Corresponds to Java: inherited from `CustomFieldValueReader.m_typeOffset` (`protected`)
    pub fn set_type_offset(&mut self, offset: usize) {
        self.base.set_type_offset(offset);
    }

/// Get the offset of the "lookup table GUID".
    ///
/// Corresponds to Java: inherited from `CustomFieldValueReader.m_fieldOffset` (`protected`)
    pub fn get_field_offset(&self) -> usize {
        self.base.get_field_offset()
    }

/// Set the offset of the "lookup table GUID".
    ///
/// Corresponds to Java: inherited from `CustomFieldValueReader.m_fieldOffset` (`protected`)
    pub fn set_field_offset(&mut self, offset: usize) {
        self.base.set_field_offset(offset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::custom_field_value_data_type::CustomFieldValueDataType;
    use crate::field_value::FieldValue;
    use crate::mpp::custom_field_value_reader::test_support::{
        empty_fixed_data, var_data_with_meta,
    };

/// Construct a reader with empty data.
    fn reader() -> (CustomFieldValueReader12, Rc<RefCell<ProjectFile>>) {
        let (meta, var_data) = var_data_with_meta(&[]);
        let file = ProjectFile::new();

        let reader = CustomFieldValueReader12::new(
            &file,
            HashMap::new(),
            meta,
            var_data,
            empty_fixed_data(),
            empty_fixed_data(),
        );
        (reader, file)
    }

/// The constructor writes MPP12's three offsets (Java's `m_parentOffset = 8`, etc.).
    #[test]
    fn constructor_sets_mpp12_offsets() {
        let (reader, _file) = reader();

        assert_eq!(reader.get_parent_offset(), 8);
        assert_eq!(reader.get_type_offset(), 48);
        assert_eq!(reader.get_field_offset(), 32);
    }

/// The forwarded methods work: with empty data `process` does not panic, and `getTypedValue` uses the base class implementation.
    #[test]
    fn forwards_to_base_reader() {
        let (mut reader, file) = reader();

        reader.process();
        assert!(file.borrow().get_custom_fields().borrow().is_empty());

// TEXT branch: UTF-16LE string
        assert_eq!(
            reader.get_typed_value(
                Some(CustomFieldValueDataType::Text),
                &[0x41, 0x00, 0x00, 0x00]
            ),
            Some(FieldValue::String("A".to_owned()))
        );

// the forwarded setters also act on the base class
        reader.set_parent_offset(1);
        assert_eq!(reader.get_parent_offset(), 1);
        reader.set_type_offset(2);
        assert_eq!(reader.get_type_offset(), 2);
        reader.set_field_offset(3);
        assert_eq!(reader.get_field_offset(), 3);
    }
}
