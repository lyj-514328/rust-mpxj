//! MPP 字段号 ↔ 统一字段类型 的映射结果（Rust 特有）。
//!
//! MPXJ 的 `org.mpxj.common.MPPTaskField` / `MPPResourceField` / `MPPAssignmentField` /
//! `MPPProjectField` / `MPPConstraintField` 都提供
//! `FieldType getInstance(ProjectFile project, int value)`：把 MPP 文件里的字段号
//! 映射成统一的 `FieldType`。
//!
//! Java 的 `FieldType` 既可能是**内置字段**（`TaskField` / `ResourceField` / …），
//! 也可能是**自定义字段** `UserDefinedField`（当字段号最高位为 1，即企业自定义字段）。
//! Rust 的 [`FieldKey`] 是本 crate 对 `FieldType` 的运行时擦除载体，但它要求
//! `Copy + Eq + Hash`，无法承载内部含 `String` 的 `UserDefinedField`。
//!
//! 因此本模块引入两个类型：
//! - [`MppFieldType`] —— 映射的返回值，表达「内置」与「自定义」两种情形；
//! - [`MppFieldKey`] —— 可作 `HashMap` 键的等价标识（自定义字段取其 unique ID）。

use std::cell::RefCell;
use std::rc::Rc;

use crate::data_type::DataType;
use crate::field_key::FieldKey;
use crate::field_type::FieldType;
use crate::field_type_class::FieldTypeClass;
use crate::user_defined_field::UserDefinedField;

/// 对应 Java: `MPPTaskField.getInstance(...)` 等方法的返回类型 `FieldType`。
#[derive(Clone)]
pub enum MppFieldType {
    /// 内置字段（`TaskField` / `ResourceField` / `AssignmentField` / `ProjectField` / `ConstraintField`）。
    Builtin(FieldKey),
    /// 项目自定义字段（`UserDefinedField`）。
    ///
    /// 对应 Java: `project.getUserDefinedFields().getOrCreateTaskField(...)` 的返回值。
    Custom(Rc<RefCell<UserDefinedField>>),
}

impl MppFieldType {
    /// 取可作 `HashMap` 键的等价标识。
    pub fn to_key(&self) -> MppFieldKey {
        match self {
            MppFieldType::Builtin(key) => MppFieldKey::Builtin(*key),
            MppFieldType::Custom(field) => MppFieldKey::Custom(field.borrow().get_unique_id()),
        }
    }

    /// 转成字段容器使用的键。
    ///
    /// 对应 Java: `container.set(item.getType(), value)` —— Java 直接把
    /// `FieldType` 当键；Rust 侧内置字段用其 [`FieldKey`]，自定义字段转成
    /// [`FieldKey::Custom`]（以 unique ID 为等价依据，与 Java
    /// `UserDefinedField.equals` 一致，见 [`CustomFieldKey`]）。
    pub fn to_field_key(&self) -> FieldKey {
        match self {
            MppFieldType::Builtin(key) => *key,
            MppFieldType::Custom(field) => {
                FieldKey::Custom(crate::field_key::CustomFieldKey::from_field(field))
            }
        }
    }

    /// 是否为内置字段。
    pub fn is_builtin(&self) -> bool {
        matches!(self, MppFieldType::Builtin(_))
    }

    /// 是否为自定义字段。
    pub fn is_custom(&self) -> bool {
        matches!(self, MppFieldType::Custom(_))
    }

    /// 取内置字段的 [`FieldKey`]；自定义字段返回 `None`。
    pub fn as_builtin(&self) -> Option<FieldKey> {
        match self {
            MppFieldType::Builtin(key) => Some(*key),
            MppFieldType::Custom(_) => None,
        }
    }

    /// 取自定义字段的共享句柄；内置字段返回 `None`。
    pub fn as_custom(&self) -> Option<&Rc<RefCell<UserDefinedField>>> {
        match self {
            MppFieldType::Builtin(_) => None,
            MppFieldType::Custom(field) => Some(field),
        }
    }

    /// 取该字段所属的实体类型。
    ///
    /// 对应 Java: `FieldType.getFieldTypeClass()`
    ///
    /// `UserDefinedField` 的对应方法返回 `Option`（Java 侧该字段可能未设置），
    /// 这里回落到 `UNKNOWN`（与 Java 返回 `null` 时各调用方的处理一致）。
    pub fn get_field_type_class(&self) -> FieldTypeClass {
        match self {
            MppFieldType::Builtin(key) => key.get_field_type_class(),
            MppFieldType::Custom(field) => field
                .borrow()
                .get_field_type_class()
                .unwrap_or(FieldTypeClass::Unknown),
        }
    }

    /// 取该字段的数据类型。
    ///
    /// 对应 Java: `FieldType.getDataType()`
    ///
    /// `UserDefinedField` 未设置数据类型时回落到 `CUSTOM`
    /// （对应 Java `MPPTaskField.getInstance(project, value)` 传入的默认值）。
    pub fn get_data_type(&self) -> DataType {
        match self {
            MppFieldType::Builtin(key) => key.get_data_type(),
            MppFieldType::Custom(field) => field.borrow().get_data_type().unwrap_or(DataType::Custom),
        }
    }

    /// 取该字段的显示名。
    ///
    /// 对应 Java: `FieldType.getName()`
    ///
    /// 自定义字段优先取外部名，未设置时回落到内部名。
    pub fn get_name(&self) -> String {
        match self {
            MppFieldType::Builtin(key) => key.get_name().to_owned(),
            MppFieldType::Custom(field) => {
                let field = field.borrow();
                field
                    .get_name()
                    .map(str::to_owned)
                    .unwrap_or_else(|| field.name().to_owned())
            }
        }
    }
}

impl From<FieldKey> for MppFieldType {
    fn from(value: FieldKey) -> Self {
        MppFieldType::Builtin(value)
    }
}

impl std::fmt::Display for MppFieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.get_name())
    }
}

/// [`MppFieldType`] 的可哈希等价标识，用作 `HashMap` 键。
///
/// 内置字段用其 [`FieldKey`]；自定义字段用其 unique ID（对应 Java
/// `UserDefinedField.getValue()`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MppFieldKey {
    /// 内置字段。
    Builtin(FieldKey),
    /// 自定义字段的 unique ID。
    Custom(i32),
}

impl std::fmt::Display for MppFieldKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MppFieldKey::Builtin(key) => write!(f, "{key}"),
            MppFieldKey::Custom(unique_id) => write!(f, "CustomField#{unique_id}"),
        }
    }
}
