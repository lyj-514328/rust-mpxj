//! 对应 Java: `org/mpxj/common/MPPProjectField.java`
//!
//! MS Project 二进制文件中表示「项目字段」的整数 → MPXJ 字段枚举的映射工具类。
//!
//! # 与 Java 的差异
//!
//! - Java 的 `getInstance` 有 `(ProjectFile, int)` 与 `(ProjectFile, int, DataType)`
//!   两个重载，Rust 侧后者命名为 [`MPPProjectField::get_instance_with_data_type`]
//!   （Rust 无重载）。
//! - Java 的 `null` 返回值 → `Option::None`。与 `MPPResourceField` 不同，
//!   Java 对**非**企业自定义字段的取值一律返回 `null`（项目字段没有内置字段表），
//!   因此本模块只有企业自定义字段分支；`getID` 对内置字段一律返回 `-1`。
//! - Java 返回 `FieldType`，Rust 侧用 [`MppFieldType`] 承载，见
//!   [`crate::common::mpp_field_type`] 的说明。
//! - 企业自定义字段分支在 Java 中直接调用
//!   `project.getUserDefinedFields().getOrCreateProjectField(...)`，Rust 侧的容器接口
//!   需要 `&mut`，故改写成等价的两步（先查重、未命中再构建并登记），
//!   见 [`get_or_create_enterprise_field`]。
//!
//! 被跳过的 Java 代码：无。未移植的 Java 方法：无。

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::mpp_field_type::MppFieldType;
use crate::data_type::DataType;
use crate::field_type_class::FieldTypeClass;
use crate::project_file::ProjectFile;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;
use crate::user_defined_field::{Builder, UserDefinedField};

/// 对应 Java: `org.mpxj.common.MPPProjectField`
pub struct MPPProjectField;

impl MPPProjectField {
    /// 企业自定义字段的字段号基数。
    ///
    /// 对应 Java: `public static final int PROJECT_FIELD_BASE = 0x0B600000;`
    pub const PROJECT_FIELD_BASE: i32 = 0x0B60_0000;

    /// 按 MS Project 文件中的字段号取对应的字段类型。
    ///
    /// 对应 Java: `public static FieldType getInstance(ProjectFile project, int value)`
    ///
    /// 参考 Java 第 47-50 行：`return getInstance(project, value, DataType.CUSTOM);`
    pub fn get_instance(project: &Rc<RefCell<ProjectFile>>, value: i32) -> Option<MppFieldType> {
        Self::get_instance_with_data_type(project, value, DataType::Custom)
    }

    /// 按 MS Project 文件中的字段号取对应的字段类型，并指定自定义字段的数据类型。
    ///
    /// 对应 Java: `public static FieldType getInstance(ProjectFile project, int value, DataType customFieldDataType)`
    ///
    /// 参考 Java 第 61-78 行。
    pub fn get_instance_with_data_type(
        project: &Rc<RefCell<ProjectFile>>,
        value: i32,
        custom_field_data_type: DataType,
    ) -> Option<MppFieldType> {
        // Java: `if ((value & 0x8000) != 0) { return project.getUserDefinedFields().getOrCreateProjectField(...); }`
        if (value & 0x8000) != 0 {
            return get_or_create_enterprise_field(project, value, custom_field_data_type)
                .map(MppFieldType::Custom);
        }

        // Java: `return null;`（项目字段没有内置字段映射表）
        None
    }

    /// 取字段在 MS Project 中使用的字段号。
    ///
    /// 对应 Java: `public static int getID(FieldType value)`
    ///
    /// 参考 Java 第 86-98 行：只有 `UserDefinedField` 返回 `getValue()`，
    /// 其余一律返回 `-1`。
    pub fn get_id(value: &MppFieldType) -> i32 {
        match value {
            // Java: `if (value instanceof UserDefinedField) { result = value.getValue(); } else { result = -1; }`
            MppFieldType::Custom(field) => match field.try_borrow() {
                Ok(field) => field.get_value(),
                // Rust 特有：字段正被可变借用时无法读取（Java 无此情形）。
                Err(_) => -1,
            },
            MppFieldType::Builtin(_) => -1,
        }
    }
}

/// 取（必要时创建）企业自定义字段对应的 `UserDefinedField`。
///
/// 对应 Java: `MPPProjectField.java` 第 65-74 行，即 `0x8000` 位置位时执行的
///
/// ```java
/// return project.getUserDefinedFields().getOrCreateProjectField(Integer.valueOf(value), (k) -> {
///    int id = (k.intValue() & 0xFFF) + 1;
///    return new UserDefinedField.Builder(project)
///       .uniqueID(Integer.valueOf(PROJECT_FIELD_BASE + k.intValue()))
///       .internalName("ENTERPRISE_CUSTOM_FIELD" + id)
///       .externalName("Enterprise Custom Field " + id)
///       .fieldTypeClass(FieldTypeClass.PROJECT)
///       .dataType(customFieldDataType)
///       .build();
/// });
/// ```
///
/// # 与 Java 的差异
///
/// - Java 的 `getOrCreateProjectField` 以 MPP 字段号 `value` 为键，命中时直接返回既有
///   字段、缺失时才调用创建函数；Rust 的
///   [`crate::user_defined_field_container::UserDefinedFieldContainer::get_or_create_project_field`]
///   需要 `&mut` 容器，而 `Builder::build()` 又要借同一个上下文取序号生成器，二者无法
///   在一次借用内完成。因此这里改写成两步：先按**等价且一一对应**的 unique ID
///   （`PROJECT_FIELD_BASE + value`）在 unique ID 索引中查重，未命中才构建并登记。
///   已存在时不构建，与 Java 一样不会多申请一次序号。
/// - Java 用 `project`（`ProjectFile`）作序号生成器提供者；Rust 侧直接交给
///   `ProjectContext`，因为 `impl UniqueIdObjectSequenceProvider for ProjectFile`
///   本来就转发给上下文（共享同一批序号生成器，见 `crate::project_file`）。
/// - 项目正在被可变借用等无法取到上下文的情形返回 `None`（Java 无此情形）。
fn get_or_create_enterprise_field(
    project: &Rc<RefCell<ProjectFile>>,
    value: i32,
    custom_field_data_type: DataType,
) -> Option<Rc<RefCell<UserDefinedField>>> {
    let context = project.try_borrow().ok()?.get_project_context();

    // Java: `PROJECT_FIELD_BASE + k.intValue()`
    let unique_id = MPPProjectField::PROJECT_FIELD_BASE + value;
    let existing = context
        .try_borrow()
        .ok()
        .and_then(|context| context.get_user_defined_fields().get_by_unique_id(Some(unique_id)));

    if let Some(existing) = existing {
        return Some(existing);
    }

    // Java: `int id = (k.intValue() & 0xFFF) + 1;`
    let id = (value & 0xFFF) + 1;
    let provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> = context.clone();
    let field = Builder::new(provider)
        .unique_id(Some(unique_id))
        .internal_name(Some(format!("ENTERPRISE_CUSTOM_FIELD{id}")))
        .external_name(Some(format!("Enterprise Custom Field {id}")))
        .field_type_class(FieldTypeClass::Project)
        .data_type(custom_field_data_type)
        .build();

    // Java: `getOrCreateProjectField(...)`（上文已确认该字段不存在，故这里是新建）。
    // 字段在取得可变借用之前构建完毕，避免与容器借用冲突。
    let created = context
        .try_borrow_mut()
        .ok()?
        .get_user_defined_fields_mut()
        .get_or_create_project_field(value, move |_| field);

    Some(created)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_key::FieldKey;
    use crate::project_field::ProjectField;
    use crate::resource_field::ResourceField;

    /// 新建一份项目文件（自定义字段分支需要项目上下文）。
    fn project() -> Rc<RefCell<ProjectFile>> {
        ProjectFile::new()
    }

    #[test]
    fn project_field_base_matches_java() {
        assert_eq!(MPPProjectField::PROJECT_FIELD_BASE, 0x0B60_0000);
    }

    #[test]
    fn builtin_project_fields_have_no_mpp_mapping() {
        let project = project();

        // Java: 非企业自定义字段一律 `return null;`
        for value in [0, 1, 2, 99, 181, 182, 1000] {
            assert!(
                MPPProjectField::get_instance(&project, value).is_none(),
                "字段号 {value} 在 Java 中返回 null"
            );
        }

        // 负数字段号：`i32::MIN` 的 `0x8000` 位为 0，走内置分支 → None；
        // 而 `-1`（位模式全 1）的 `0x8000` 位为 1，Java 与 Rust 都会走企业自定义字段分支。
        assert!(MPPProjectField::get_instance(&project, i32::MIN).is_none());
        assert!(MPPProjectField::get_instance(&project, -1).is_some_and(|field| field.is_custom()));
        // `0x8000` 位置位前的最大值仍走内置分支
        assert!(MPPProjectField::get_instance(&project, 0x7FFF).is_none());

        // Java: getID 对非 UserDefinedField 一律返回 -1（与字段类型无关）
        assert_eq!(
            MPPProjectField::get_id(&MppFieldType::Builtin(FieldKey::Project(
                ProjectField::Name
            ))),
            -1
        );
        assert_eq!(
            MPPProjectField::get_id(&MppFieldType::Builtin(FieldKey::Resource(
                ResourceField::Name
            ))),
            -1
        );
    }

    #[test]
    fn enterprise_custom_fields_are_created_once_and_reused() {
        let project = project();
        let value = 0x8000 | 3;

        let field = MPPProjectField::get_instance(&project, value).expect("应为企业自定义字段");
        let custom = field.as_custom().expect("应为自定义字段");
        assert!(field.is_custom());
        assert_eq!(
            custom.borrow().get_unique_id(),
            MPPProjectField::PROJECT_FIELD_BASE + value
        );
        assert_eq!(custom.borrow().name(), "ENTERPRISE_CUSTOM_FIELD4");
        assert_eq!(custom.borrow().get_name(), Some("Enterprise Custom Field 4"));
        assert_eq!(custom.borrow().get_field_type_class(), Some(FieldTypeClass::Project));
        assert_eq!(custom.borrow().get_data_type(), Some(DataType::Custom));
        assert_eq!(
            MPPProjectField::get_id(&field),
            MPPProjectField::PROJECT_FIELD_BASE + value
        );

        // Java 的 getOrCreateProjectField 复用既有字段（不重建）
        let again = MPPProjectField::get_instance(&project, value).expect("应为企业自定义字段");
        let again = again.as_custom().expect("应为自定义字段");
        assert!(Rc::ptr_eq(custom, again));
        assert_eq!(
            project
                .borrow()
                .get_user_defined_fields()
                .get_project_fields()
                .len(),
            1
        );
    }

    #[test]
    fn custom_field_uses_requested_data_type() {
        let project = project();
        let value = 0x8000 | 0x2A;

        let field =
            MPPProjectField::get_instance_with_data_type(&project, value, DataType::Numeric)
                .expect("应为企业自定义字段");
        let custom = field.as_custom().expect("应为自定义字段");
        assert_eq!(custom.borrow().get_data_type(), Some(DataType::Numeric));
        // Java: id = (value & 0xFFF) + 1 = 0x2B
        assert_eq!(
            custom.borrow().get_name(),
            Some("Enterprise Custom Field 43")
        );
    }
}
