//! 对应 Java: `org/mpxj/mpp/CustomFieldValueReader12.java`
//!
//! MPP12 的取值列表读取器：只是把 [`CustomFieldValueReader`] 的三个偏移设成 MPP12 的布局。
//!
//! # 继承的还原（组合 + 转发）
//!
//! Java: `class CustomFieldValueReader12 extends CustomFieldValueReader`。Rust 无继承，按约定
//! 改为**组合 + 转发**：
//!
//! | Java | Rust |
//! |---|---|
//! | `extends CustomFieldValueReader` | 结构体持有私有字段 `base: CustomFieldValueReader` |
//! | 继承来的 `process()` / `getTypedValue(...)` / `protected` 字段 | 逐一转发到 `base`（见各方法说明） |
//! | 构造方法中 `m_parentOffset = 8; m_typeOffset = 48; m_fieldOffset = 32;` | 构造后调用 `base.set_*_offset` |
//!
//! 跳过的 Java 方法：无（本类只有 1 个构造方法，无其它方法）。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

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

/// MPP12 取值列表读取器。
///
/// 对应 Java: `class CustomFieldValueReader12 extends CustomFieldValueReader`
pub struct CustomFieldValueReader12 {
    /// 组合进来的基类实例。
    ///
    /// 对应 Java: `extends CustomFieldValueReader`
    base: CustomFieldValueReader,
}

impl CustomFieldValueReader12 {
    /// 构造方法。
    ///
    /// 对应 Java: `public CustomFieldValueReader12(ProjectFile file, Map<UUID, FieldType> lookupTableMap, VarMeta outlineCodeVarMeta, Var2Data outlineCodeVarData, FixedData outlineCodeFixedData, FixedData outlineCodeFixedData2)`
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

    /// 读取全部取值项。
    ///
    /// 对应 Java: 继承自 `CustomFieldValueReader.process()`
    pub fn process(&self) {
        self.base.process();
    }

    /// 把原始字节转换成 Java 类型的取值。
    ///
    /// 对应 Java: 继承自 `CustomFieldValueReader.getTypedValue(...)`（`protected`）
    pub fn get_typed_value(
        &self,
        type_: Option<CustomFieldValueDataType>,
        value: &[u8],
    ) -> Option<FieldValue> {
        self.base.get_typed_value(type_, value)
    }

    /// 取「父级 unique ID」的偏移。
    ///
    /// 对应 Java: 继承自 `CustomFieldValueReader.m_parentOffset`（`protected`）
    pub fn get_parent_offset(&self) -> usize {
        self.base.get_parent_offset()
    }

    /// 设置「父级 unique ID」的偏移。
    ///
    /// 对应 Java: 继承自 `CustomFieldValueReader.m_parentOffset`（`protected`）
    pub fn set_parent_offset(&mut self, offset: usize) {
        self.base.set_parent_offset(offset);
    }

    /// 取「取值类型」的偏移。
    ///
    /// 对应 Java: 继承自 `CustomFieldValueReader.m_typeOffset`（`protected`）
    pub fn get_type_offset(&self) -> usize {
        self.base.get_type_offset()
    }

    /// 设置「取值类型」的偏移。
    ///
    /// 对应 Java: 继承自 `CustomFieldValueReader.m_typeOffset`（`protected`）
    pub fn set_type_offset(&mut self, offset: usize) {
        self.base.set_type_offset(offset);
    }

    /// 取「查找表 GUID」的偏移。
    ///
    /// 对应 Java: 继承自 `CustomFieldValueReader.m_fieldOffset`（`protected`）
    pub fn get_field_offset(&self) -> usize {
        self.base.get_field_offset()
    }

    /// 设置「查找表 GUID」的偏移。
    ///
    /// 对应 Java: 继承自 `CustomFieldValueReader.m_fieldOffset`（`protected`）
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

    /// 构造一个空数据的读取器。
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

    /// 构造方法写入 MPP12 的三个偏移（Java 的 `m_parentOffset = 8` 等）。
    #[test]
    fn constructor_sets_mpp12_offsets() {
        let (reader, _file) = reader();

        assert_eq!(reader.get_parent_offset(), 8);
        assert_eq!(reader.get_type_offset(), 48);
        assert_eq!(reader.get_field_offset(), 32);
    }

    /// 转发的方法可用：空数据下 `process` 不 panic，`getTypedValue` 走基类实现。
    #[test]
    fn forwards_to_base_reader() {
        let (mut reader, file) = reader();

        reader.process();
        assert!(file.borrow().get_custom_fields().borrow().is_empty());

        // TEXT 分支：UTF-16LE 字符串
        assert_eq!(
            reader.get_typed_value(
                Some(CustomFieldValueDataType::Text),
                &[0x41, 0x00, 0x00, 0x00]
            ),
            Some(FieldValue::String("A".to_owned()))
        );

        // 转发的 setter 同样作用于基类
        reader.set_parent_offset(1);
        assert_eq!(reader.get_parent_offset(), 1);
        reader.set_type_offset(2);
        assert_eq!(reader.get_type_offset(), 2);
        reader.set_field_offset(3);
        assert_eq!(reader.get_field_offset(), 3);
    }
}
