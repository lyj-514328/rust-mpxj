//! 对应 Java: `org/mpxj/common/MPPConstraintField.java`
//!
//! MS Project 二进制文件中表示「约束字段」的整数 → MPXJ 字段枚举的映射工具类。
//!
//! # 与 Java 的差异
//!
//! - Java 的 `getInstance` / `getID` 使用 `FieldType` 接口，Rust 侧用
//!   [`MppFieldType`]（内置 / 自定义两种情形）承载，见
//!   [`crate::common::mpp_field_type`] 的说明。约束字段没有「企业自定义字段」分支，
//!   因此这里的返回值恒为 [`MppFieldType::Builtin`]。
//! - Java 的 `null` 返回值 → `Option::None`；越界读表返回 `None`、越界读
//!   `ID_ARRAY` 返回 `-1` 而不抛 `ArrayIndexOutOfBoundsException`。
//! - Java 的 `getInstance(int value)` 没有 `project` 参数；Rust 侧按
//!   `MPPAssignmentField` / `MPPTaskField` 等模块的**统一签名**提供
//!   [`MPPConstraintField::get_instance`]（带 `project`，未使用），并保留与之等价的
//!   Java 原签名 [`MPPConstraintField::get_instance_from_value`]。
//! - Java 的 `ID_ARRAY` 由 `FIELD_ARRAY` 在编译期按 Java 的静态初始化块
//!   （先填 -1，再令 `ID_ARRAY[field.getValue()] = loop`）推导。
//!
//! 被跳过的 Java 代码：无。未移植的 Java 方法：无。

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::mpp_field_type::MppFieldType;
use crate::constraint_field::ConstraintField;
use crate::field_key::FieldKey;
use crate::mpxj_enum::MpxjEnum;
use crate::project_file::ProjectFile;

/// 对应 Java: `org.mpxj.common.MPPConstraintField`
pub struct MPPConstraintField;

impl MPPConstraintField {
    /// 企业自定义字段的字段号基数。
    ///
    /// 对应 Java: `public static final int CONSTRAINT_FIELD_BASE = 0xD400000;`
    pub const CONSTRAINT_FIELD_BASE: i32 = 0x0D40_0000;

    /// 按 MS Project 文件中的字段号取对应的字段类型。
    ///
    /// Java: `public static FieldType getInstance(int value)`（无 `project` 参数）。
    ///
    /// Rust 侧为与 `MPPAssignmentField` / `MPPTaskField` 等保持统一签名而保留
    /// `project` 参数，约束字段没有自定义字段分支，因此该参数未被使用。
    pub fn get_instance(
        project: &Rc<RefCell<ProjectFile>>,
        value: i32,
    ) -> Option<MppFieldType> {
        // Java 的 getInstance 不使用 project（约束字段不存在自定义字段）。
        let _ = project;
        Self::get_instance_from_value(value)
    }

    /// 按 MS Project 文件中的字段号取对应的字段类型。
    ///
    /// 对应 Java: `public static FieldType getInstance(int value)`
    ///
    /// 参考 Java 第 45-55 行。
    pub fn get_instance_from_value(value: i32) -> Option<MppFieldType> {
        // Java: `FieldType result = null; if (value >= 0 && value < FIELD_ARRAY.length) { result = FIELD_ARRAY[value]; }`
        if value >= 0 && (value as usize) < MAX_VALUE {
            return FIELD_ARRAY[value as usize]
                .map(|field| MppFieldType::Builtin(FieldKey::Constraint(field)));
        }

        None
    }

    /// 取字段在 MS Project 中使用的字段号。
    ///
    /// 对应 Java: `public static int getID(FieldType value)`
    ///
    /// 参考 Java 第 63-66 行：`ID_ARRAY[value.getValue()]`。
    pub fn get_id(value: &MppFieldType) -> i32 {
        match value {
            // 约束字段没有自定义字段分支，此分支仅为与 Java 的
            // `value instanceof UserDefinedField` 语义对齐（Java 同样返回 getValue()）；
            // 字段已被可变借用时返回 -1（Java 无此情形）。
            MppFieldType::Custom(field) => field
                .try_borrow()
                .map(|field| field.get_value())
                .unwrap_or(-1),
            // Java: `ID_ARRAY[value.getValue()]`，越界时抛
            // ArrayIndexOutOfBoundsException（传入非约束字段时可能发生）；
            // Rust 侧不 panic，退化为 ID_ARRAY 的默认值 -1。
            MppFieldType::Builtin(key) => usize::try_from(key.value())
                .ok()
                .and_then(|index| ID_ARRAY.get(index))
                .copied()
                .unwrap_or(-1),
        }
    }
}

/// 映射表长度（`FIELD_ARRAY` 的元素个数）。
///
/// 对应 Java: `private static final int MAX_VALUE = 10;`
const MAX_VALUE: usize = 10;

/// MS Project 字段号 → 约束字段。
///
/// 对应 Java: `MPPConstraintField.java` 第 70-77 行的 `FIELD_ARRAY`
/// （`private static final ConstraintField[] FIELD_ARRAY = new ConstraintField[MAX_VALUE]`
/// 及其 `static {}` 初始化块）。Java 中未被赋值的下标为 `null`，Rust 侧为 `None`；
/// 每个条目后的注释是 Java 中显式赋值的下标。
const FIELD_ARRAY: [Option<ConstraintField>; MAX_VALUE] = [
    None, // 0
    None, // 1
    None, // 2
    None, // 3
    None, // 4
    None, // 5
    Some(ConstraintField::Task1), // 6
    Some(ConstraintField::Task2), // 7
    None, // 8
    Some(ConstraintField::UniqueId), // 9
];

/// 字段 → MS Project 字段号的逆向映射表。
///
/// 对应 Java: `MPPConstraintField.java` 第 79-92 行的 `ID_ARRAY` 静态初始化块
/// （先调用 `Arrays.fill(ID_ARRAY, -1)`，再遍历 `FIELD_ARRAY` 令
/// `ID_ARRAY[field.getValue()] = loop`）。
const ID_ARRAY: [i32; ConstraintField::MAX_VALUE] = build_id_array(&FIELD_ARRAY);

/// 按 Java 的静态初始化块在编译期推导 `ID_ARRAY`。
const fn build_id_array(
    field_array: &[Option<ConstraintField>; MAX_VALUE],
) -> [i32; ConstraintField::MAX_VALUE] {
    let mut result = [-1; ConstraintField::MAX_VALUE];
    let mut loop_index = 0;
    while loop_index < field_array.len() {
        if let Some(field) = field_array[loop_index] {
            result[field as usize] = loop_index as i32;
        }
        loop_index += 1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assignment_field::AssignmentField;
    use crate::field_type::FieldType;

    /// 新建一份项目文件（Java 的 getInstance 不需要，仅为统一签名而传）。
    fn project() -> Rc<RefCell<ProjectFile>> {
        ProjectFile::new()
    }

    /// 取内置约束字段。
    fn builtin(value: i32) -> ConstraintField {
        match MPPConstraintField::get_instance(&project(), value) {
            Some(MppFieldType::Builtin(FieldKey::Constraint(field))) => field,
            _ => panic!("字段号 {value} 应为约束字段"),
        }
    }

    #[test]
    fn field_numbers_match_java() {
        // 与 Java `static {}` 块逐项对齐
        let field = builtin(6);
        assert_eq!(field, ConstraintField::Task1);
        assert_eq!(field.name(), "TASK1");

        let field = builtin(7);
        assert_eq!(field, ConstraintField::Task2);
        assert_eq!(field.name(), "TASK2");

        let field = builtin(9);
        assert_eq!(field, ConstraintField::UniqueId);
        assert_eq!(field.name(), "UNIQUE_ID");
    }

    #[test]
    fn unassigned_and_out_of_range_field_numbers_are_none() {
        let project = project();

        // Java 中这些下标未赋值（即 null）
        for value in [0, 1, 2, 3, 4, 5, 8] {
            assert!(
                MPPConstraintField::get_instance(&project, value).is_none(),
                "字段号 {value} 在 Java 中未赋值"
            );
        }

        // 越界（Java 返回 null）
        assert!(MPPConstraintField::get_instance(&project, -1).is_none());
        assert!(MPPConstraintField::get_instance(&project, MAX_VALUE as i32).is_none());
        assert!(MPPConstraintField::get_instance(&project, 100).is_none());
    }

    #[test]
    fn java_signature_overload_matches_contract_signature() {
        for value in -1..(MAX_VALUE as i32 + 1) {
            assert_eq!(
                MPPConstraintField::get_instance_from_value(value).map(|field| MPPConstraintField::get_id(&field)),
                MPPConstraintField::get_instance(&project(), value)
                    .map(|field| MPPConstraintField::get_id(&field)),
                "字段号 {value}"
            );
        }
    }

    #[test]
    fn id_round_trips_for_every_mapped_field_number() {
        let project = project();

        for value in 0..(MAX_VALUE as i32) {
            if let Some(field) = MPPConstraintField::get_instance(&project, value) {
                assert_eq!(MPPConstraintField::get_id(&field), value, "字段号 {value}");
            }
        }
    }

    #[test]
    fn id_array_matches_java_static_initialiser() {
        // Java: ID_ARRAY[UNIQUE_ID.getValue()] = 9 / TASK1 = 6 / TASK2 = 7
        assert_eq!(
            MPPConstraintField::get_id(&MppFieldType::Builtin(FieldKey::Constraint(
                ConstraintField::UniqueId
            ))),
            9
        );
        assert_eq!(
            MPPConstraintField::get_id(&MppFieldType::Builtin(FieldKey::Constraint(
                ConstraintField::Task1
            ))),
            6
        );
        assert_eq!(
            MPPConstraintField::get_id(&MppFieldType::Builtin(FieldKey::Constraint(
                ConstraintField::Task2
            ))),
            7
        );

        // 非约束字段（getValue() 超出 ID_ARRAY 长度）在 Java 中抛
        // ArrayIndexOutOfBoundsException，Rust 侧退化为 -1。
        assert_eq!(
            MPPConstraintField::get_id(&MppFieldType::Builtin(FieldKey::Assignment(
                AssignmentField::Finish
            ))),
            -1
        );
    }
}
