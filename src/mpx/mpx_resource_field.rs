//! 对应 Java: `org/mpxj/mpx/MPXResourceField.java`
//!
//! MPX 文件中资源字段编号（MPX field type）与 MPXJ 的
//! [`ResourceField`] 之间的双向映射表。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `final class MPXResourceField`（私有构造器） | [`MPXResourceField`] | 空结构体作为命名空间，语义为无状态工具类 |
//! | `public static ResourceField getMpxjField(int value)` | [`MPXResourceField::get_mpxj_field`] | Java 越界/未映射返回 `null` → `None` |
//! | `public static int getMpxField(int value)` | [`MPXResourceField::get_mpx_field`] | 入参是 `ResourceField` 的 int 值（即枚举 ordinal），越界/未映射返回 `0` |
//! | `public static final int MAX_FIELDS` | [`MPXResourceField::MAX_FIELDS`] | |
//! | `private static final ResourceField[] MPX_MPXJ_ARRAY`（静态初始化块逐项赋值） | [`MPXResourceField::get_mpxj_field`] 内的 `match` | `match` 臂按 MPX 字段号升序排列，便于与 Java 静态块逐条比对 |
//! | `private static final int[] MPXJ_MPX_ARRAY`（静态初始化块按 MPX 数组回填） | [`MPXResourceField::get_mpx_field`] + 模块私有函数 [`mpx_number_of`] 内的 `match` | 臂顺序与 Java 静态块一致（按 Java 枚举常量名字母序） |
//! | `public static final List<FieldType> CUSTOM_FIELDS` | [`MPXResourceField::CUSTOM_FIELDS`] | Java 类型是 `List<FieldType>`（元素全是 `ResourceField`），Rust 侧用 `&'static [ResourceField]` |
//!
//! 几点取舍：
//!
//! - Java 的 `MPX_MPXJ_ARRAY` 是定长 52 的稀疏数组（大量 `null`），Rust 侧用
//!   `match` 表达稀疏映射，语义等价。
//! - `get_mpx_field` 用 [`ResourceField::get_instance`] 做边界检查：Java 的
//!   `MPXJ_MPX_ARRAY.length == ResourceField.MAX_VALUE`，与 `get_instance` 的合法区间一致，
//!   未命中的下标在 Java 中取数组默认值 `0`，Rust 侧同样返回 `0`。
//! - `CUSTOM_FIELDS`：Java 在静态块中用
//!   `Arrays.stream(MPX_MPXJ_ARRAY).filter(ResourceFieldLists.CUSTOM_FIELDS::contains)`
//!   生成。Rust 侧 `ResourceFieldLists`（`src/common/resource_field_lists.rs`）由其他模块
//!   并行翻译，为避免跨模块依赖，这里直接给出过滤结果（与 Java 完全一致，共 5 项）。
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//! 未移植的方法：无。

use crate::resource_field::ResourceField;

/// MPX 资源字段编号与 [`ResourceField`] 的映射工具。
///
/// 对应 Java: `final class MPXResourceField`
pub struct MPXResourceField;

impl MPXResourceField {
    /// MPX 文件中资源字段编号的最大个数（可用编号为 `0..MAX_FIELDS`）。
    ///
    /// 对应 Java: `public static final int MAX_FIELDS`
    pub const MAX_FIELDS: usize = 52;

    /// 根据 MPX 文件中读到的字段编号取 [`ResourceField`]。
    ///
    /// 对应 Java: `public static ResourceField getMpxjField(int value)`
    ///
    /// Java 对越界或未映射的编号返回 `null`，Rust 侧返回 `None`。
    pub fn get_mpxj_field(value: i32) -> Option<ResourceField> {
        match value {
            1 => Some(ResourceField::Name),
            2 => Some(ResourceField::Initials),
            3 => Some(ResourceField::Group),
            4 => Some(ResourceField::Code),
            5 => Some(ResourceField::Text1),
            6 => Some(ResourceField::Text2),
            7 => Some(ResourceField::Text3),
            8 => Some(ResourceField::Text4),
            9 => Some(ResourceField::Text5),
            11 => Some(ResourceField::EmailAddress),
            20 => Some(ResourceField::Work),
            21 => Some(ResourceField::BaselineWork),
            22 => Some(ResourceField::ActualWork),
            23 => Some(ResourceField::RemainingWork),
            24 => Some(ResourceField::OvertimeWork),
            25 => Some(ResourceField::WorkVariance),
            26 => Some(ResourceField::PercentWorkComplete),
            30 => Some(ResourceField::Cost),
            31 => Some(ResourceField::BaselineCost),
            32 => Some(ResourceField::ActualCost),
            33 => Some(ResourceField::RemainingCost),
            34 => Some(ResourceField::CostVariance),
            40 => Some(ResourceField::Id),
            41 => Some(ResourceField::MaxUnits),
            42 => Some(ResourceField::StandardRate),
            43 => Some(ResourceField::OvertimeRate),
            44 => Some(ResourceField::CostPerUse),
            45 => Some(ResourceField::AccrueAt),
            46 => Some(ResourceField::Overallocated),
            47 => Some(ResourceField::Peak),
            48 => Some(ResourceField::BaseCalendar),
            49 => Some(ResourceField::UniqueId),
            50 => Some(ResourceField::Objects),
            51 => Some(ResourceField::LinkedFields),
            _ => None,
        }
    }

    /// 取 [`ResourceField`] 在 MPX 文件中对应的字段编号。
    ///
    /// 对应 Java: `public static int getMpxField(int value)`
    ///
    /// `value` 是 `ResourceField` 的 int 值（`getValue()`，即本 crate 中的
    /// `*field as i32` / [`crate::mpxj_enum::MpxjEnum::value`]）；越界或未在
    /// MPX 映射表中出现的字段返回 `0`（与 Java 的数组默认值一致）。
    pub fn get_mpx_field(value: i32) -> i32 {
        match ResourceField::get_instance(value) {
            Some(field) => mpx_number_of(field),
            None => 0,
        }
    }

    /// MPX 映射表中属于「自定义字段」的字段列表，按 MPX 字段号升序。
    ///
    /// 对应 Java: `public static final List<FieldType> CUSTOM_FIELDS`
    ///
    /// 与 Java 的静态初始化块 `Arrays.stream(MPX_MPXJ_ARRAY).filter(
    /// ResourceFieldLists.CUSTOM_FIELDS::contains).forEach(CUSTOM_FIELDS::add)` 结果一致：
    /// MPX 映射表中只有 `TEXT1..TEXT5` 属于自定义字段。
    pub const CUSTOM_FIELDS: &'static [ResourceField] = &[
        ResourceField::Text1,
        ResourceField::Text2,
        ResourceField::Text3,
        ResourceField::Text4,
        ResourceField::Text5,
    ];
}

/// 取 [`ResourceField`] 对应的 MPX 字段编号，未在 MPX 映射表中出现时返回 `0`。
///
/// 对应 Java: `private static final int[] MPXJ_MPX_ARRAY` 的静态初始化块
/// （按 `MPX_MPXJ_ARRAY` 下标升序回填）。
///
/// 臂的顺序与 Java 静态块中的书写顺序一致（按 Java 枚举常量名字母序）。
fn mpx_number_of(field: ResourceField) -> i32 {
    match field {
        ResourceField::AccrueAt => 45,
        ResourceField::ActualCost => 32,
        ResourceField::ActualWork => 22,
        ResourceField::BaseCalendar => 48,
        ResourceField::BaselineCost => 31,
        ResourceField::BaselineWork => 21,
        ResourceField::Code => 4,
        ResourceField::Cost => 30,
        ResourceField::CostPerUse => 44,
        ResourceField::CostVariance => 34,
        ResourceField::EmailAddress => 11,
        ResourceField::Group => 3,
        ResourceField::Id => 40,
        ResourceField::Initials => 2,
        ResourceField::LinkedFields => 51,
        ResourceField::MaxUnits => 41,
        ResourceField::Name => 1,
        ResourceField::Objects => 50,
        ResourceField::Overallocated => 46,
        ResourceField::OvertimeRate => 43,
        ResourceField::OvertimeWork => 24,
        ResourceField::Peak => 47,
        ResourceField::PercentWorkComplete => 26,
        ResourceField::RemainingCost => 33,
        ResourceField::RemainingWork => 23,
        ResourceField::StandardRate => 42,
        ResourceField::Text1 => 5,
        ResourceField::Text2 => 6,
        ResourceField::Text3 => 7,
        ResourceField::Text4 => 8,
        ResourceField::Text5 => 9,
        ResourceField::UniqueId => 49,
        ResourceField::Work => 20,
        ResourceField::WorkVariance => 25,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpxj_enum::MpxjEnum;
    use std::collections::HashMap;

    /// Java `MPXResourceField` 静态初始化块中 `MPX_MPXJ_ARRAY[...]` 的全部赋值
    /// （共 34 条），此处按 MPX 字段号升序排列。
    const JAVA_TABLE: [(i32, ResourceField); 34] = [
        (1, ResourceField::Name),
        (2, ResourceField::Initials),
        (3, ResourceField::Group),
        (4, ResourceField::Code),
        (5, ResourceField::Text1),
        (6, ResourceField::Text2),
        (7, ResourceField::Text3),
        (8, ResourceField::Text4),
        (9, ResourceField::Text5),
        (11, ResourceField::EmailAddress),
        (20, ResourceField::Work),
        (21, ResourceField::BaselineWork),
        (22, ResourceField::ActualWork),
        (23, ResourceField::RemainingWork),
        (24, ResourceField::OvertimeWork),
        (25, ResourceField::WorkVariance),
        (26, ResourceField::PercentWorkComplete),
        (30, ResourceField::Cost),
        (31, ResourceField::BaselineCost),
        (32, ResourceField::ActualCost),
        (33, ResourceField::RemainingCost),
        (34, ResourceField::CostVariance),
        (40, ResourceField::Id),
        (41, ResourceField::MaxUnits),
        (42, ResourceField::StandardRate),
        (43, ResourceField::OvertimeRate),
        (44, ResourceField::CostPerUse),
        (45, ResourceField::AccrueAt),
        (46, ResourceField::Overallocated),
        (47, ResourceField::Peak),
        (48, ResourceField::BaseCalendar),
        (49, ResourceField::UniqueId),
        (50, ResourceField::Objects),
        (51, ResourceField::LinkedFields),
    ];

    /// Java `Arrays.stream(MPX_MPXJ_ARRAY).filter(ResourceFieldLists.CUSTOM_FIELDS::contains)`
    /// 的过滤结果。
    const JAVA_CUSTOM_FIELDS: [ResourceField; 5] = [
        ResourceField::Text1,
        ResourceField::Text2,
        ResourceField::Text3,
        ResourceField::Text4,
        ResourceField::Text5,
    ];

    /// 逐一核对 MPX 字段号 → `ResourceField` 的映射（含全部未映射编号都返回 `None`）。
    #[test]
    fn get_mpxj_field_matches_java_table() {
        assert_eq!(MPXResourceField::MAX_FIELDS, 52);

        let mut expected = vec![None; MPXResourceField::MAX_FIELDS];
        for (number, field) in JAVA_TABLE {
            expected[number as usize] = Some(field);
        }

        for number in 0..MPXResourceField::MAX_FIELDS as i32 {
            assert_eq!(
                MPXResourceField::get_mpxj_field(number),
                expected[number as usize],
                "MPX 字段号 {number}"
            );
        }

        assert_eq!(MPXResourceField::get_mpxj_field(-1), None);
        assert_eq!(
            MPXResourceField::get_mpxj_field(MPXResourceField::MAX_FIELDS as i32),
            None
        );
    }

    /// 逐一核对 `ResourceField` → MPX 字段号的反向映射。
    #[test]
    fn get_mpx_field_matches_java_table() {
        let mut expected: HashMap<ResourceField, i32> = HashMap::new();
        for (number, field) in JAVA_TABLE {
            expected.insert(field, number);
        }

        for field in ResourceField::ALL {
            let want = expected.get(&field).copied().unwrap_or(0);
            assert_eq!(
                MPXResourceField::get_mpx_field(field.value()),
                want,
                "{field:?}"
            );
        }

        assert_eq!(MPXResourceField::get_mpx_field(-1), 0);
        assert_eq!(
            MPXResourceField::get_mpx_field(ResourceField::MAX_VALUE as i32),
            0
        );
    }

    /// 反向映射未覆盖的字段必须返回 `0`（Java 数组默认值）。
    #[test]
    fn unmapped_fields_return_zero() {
        for field in [
            ResourceField::Text6,
            ResourceField::Text10,
            ResourceField::Text11,
            ResourceField::Text30,
            ResourceField::Notes,
            ResourceField::RegularWork,
            ResourceField::Bcws,
            ResourceField::Sv,
            ResourceField::AvailableFrom,
            ResourceField::Number1,
            ResourceField::Flag1,
            ResourceField::Cost1,
            ResourceField::Duration1,
            ResourceField::Date1,
            ResourceField::Start1,
            ResourceField::Finish1,
            ResourceField::OutlineCode1,
            ResourceField::Baseline1Cost,
            ResourceField::EnterpriseText1,
        ] {
            assert_eq!(
                MPXResourceField::get_mpx_field(field.value()),
                0,
                "{field:?}"
            );
        }
    }

    /// 两个方向互相往返：有映射的字段必须回到自身，无映射的字段落到 `None`。
    #[test]
    fn mappings_round_trip() {
        for field in ResourceField::ALL {
            let number = MPXResourceField::get_mpx_field(field.value());
            let back = MPXResourceField::get_mpxj_field(number);
            if number == 0 {
                assert_eq!(back, None, "{field:?}");
            } else {
                assert_eq!(back, Some(field), "{field:?}");
            }
        }

        for (number, field) in JAVA_TABLE {
            assert_eq!(MPXResourceField::get_mpxj_field(number), Some(field));
            assert_eq!(
                MPXResourceField::get_mpxj_field(MPXResourceField::get_mpx_field(field.value())),
                Some(field),
                "MPX 字段号 {number}"
            );
        }
    }

    /// 自定义字段列表与 Java 的过滤结果一致，且按 MPX 字段号升序。
    #[test]
    fn custom_fields_match_java() {
        assert_eq!(MPXResourceField::CUSTOM_FIELDS, &JAVA_CUSTOM_FIELDS);

        let numbers: Vec<i32> = MPXResourceField::CUSTOM_FIELDS
            .iter()
            .map(|field| MPXResourceField::get_mpx_field(field.value()))
            .collect();
        assert!(numbers.iter().all(|number| *number > 0));
        assert!(numbers.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
