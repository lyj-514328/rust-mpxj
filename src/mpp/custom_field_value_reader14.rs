//! 对应 Java: `org/mpxj/mpp/CustomFieldValueReader14.java`
//!
//! MPP14 的取值列表读取器：按项目文件版本（Project 2010 前后）选择两套偏移布局。
//!
//! # 继承的还原（组合 + 转发）
//!
//! Java: `class CustomFieldValueReader14 extends CustomFieldValueReader`。Rust 无继承，按本批
//! 约定改为**组合 + 转发**：
//!
//! | Java | Rust |
//! |---|---|
//! | `extends CustomFieldValueReader` | 结构体持有私有字段 `base: CustomFieldValueReader` |
//! | 继承来的 `process()` / `getTypedValue(...)` / `protected` 字段 | 逐一转发到 `base`（见各方法说明） |
//! | 构造方法中按 `m_properties.getApplicationVersion()` 分支赋值的三个偏移 | 构造后调用 `base.set_*_offset`，判定逻辑同 Java |
//! | `NumberHelper.getInt(m_properties.getApplicationVersion())` | [`ProjectProperties::get_application_version`]（`Option<i32>`）+ `unwrap_or(0)`，与 Java 把 `null` 视为 0 一致 |
//! | `ApplicationVersion.PROJECT_2010` | 本文件的私有常量 [`PROJECT_2010`] | Java 的 `ApplicationVersion` 是常量类，其 Rust 模块 [`crate::mpp::application_version`] **已落地**并提供同值的 `ApplicationVersion::PROJECT_2010`；本文件的私有常量是并行开发期留下的等价定义，尚未改为引用（取值相同，属待清理的重复定义） |
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

/// Microsoft Project 2010 的应用程序版本号。
///
/// 对应 Java: `ApplicationVersion.PROJECT_2010`
///
/// 说明：[`crate::mpp::application_version`] 已落地并提供同值常量，本私有常量是并行
/// 开发期留下的等价定义（待清理，见文件头差异表）。
const PROJECT_2010: i32 = 14;

/// MPP14 取值列表读取器。
///
/// 对应 Java: `class CustomFieldValueReader14 extends CustomFieldValueReader`
pub struct CustomFieldValueReader14 {
    /// 组合进来的基类实例。
    ///
    /// 对应 Java: `extends CustomFieldValueReader`
    base: CustomFieldValueReader,
}

impl CustomFieldValueReader14 {
    /// 构造方法。
    ///
    /// 对应 Java: `public CustomFieldValueReader14(ProjectFile file, Map<UUID, FieldType> lookupTableMap, VarMeta outlineCodeVarMeta, Var2Data outlineCodeVarData, FixedData outlineCodeFixedData, FixedData outlineCodeFixedData2)`
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
        // `NumberHelper.getInt(null)` 返回 0，Rust 侧同理用 `unwrap_or(0)`；
        // 项目文件正被可变借用时同样退化为 0（Java 不存在该情形）。
        let application_version = file
            .try_borrow()
            .ok()
            .and_then(|file| file.get_project_properties().get_application_version())
            .unwrap_or(0);

        if application_version > PROJECT_2010 {
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
    use crate::mpp::custom_field_value_reader::test_support::{
        empty_fixed_data, var_data_with_meta,
    };

    /// 构造一个指定应用程序版本的读取器。
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

    /// Project 2010 之前（含 2010 本身）：`fieldOffset = 16 / typeOffset = 32 / parentOffset = 8`。
    #[test]
    fn constructor_uses_pre_2013_offsets() {
        // 版本号取自 Java 的 ApplicationVersion：14 = Project 2010（不满足 `> 14`）
        for version in [None, Some(0), Some(14)] {
            let reader = reader(version);
            assert_eq!(reader.get_field_offset(), 16);
            assert_eq!(reader.get_type_offset(), 32);
            assert_eq!(reader.get_parent_offset(), 8);
        }
    }

    /// Project 2013 及之后：`typeOffset = 16 / fieldOffset = 18 / parentOffset = 10`。
    #[test]
    fn constructor_uses_post_2010_offsets() {
        for version in [Some(15), Some(16)] {
            let reader = reader(version);
            assert_eq!(reader.get_type_offset(), 16);
            assert_eq!(reader.get_field_offset(), 18);
            assert_eq!(reader.get_parent_offset(), 10);
        }
    }

    /// 构造方法写入的偏移可被转发访问器改写，`process` 转发到基类。
    #[test]
    fn forwards_to_base_reader() {
        let mut reader = reader(Some(16));

        reader.set_parent_offset(1);
        reader.set_type_offset(2);
        reader.set_field_offset(3);
        assert_eq!(reader.get_parent_offset(), 1);
        assert_eq!(reader.get_type_offset(), 2);
        assert_eq!(reader.get_field_offset(), 3);

        // 空数据：不 panic，也不产生取值项
        reader.process();

        assert_eq!(
            reader.get_typed_value(Some(CustomFieldValueDataType::Number), &1.5f64.to_le_bytes()),
            Some(FieldValue::Double(1.5))
        );
    }
}
