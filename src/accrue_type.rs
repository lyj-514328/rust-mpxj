//! 对应 Java: `org/mpxj/AccrueType.java`

use crate::mpxj_enum::MpxjEnum;

/// 成本计提方式（accrue type）。
///
/// 对应 Java: `public enum AccrueType implements MpxjEnum`
///
/// Java 侧用 `EnumHelper.createTypeArray(AccrueType.class, 1)` 生成 int → 枚举的
/// 查找表：数组长度为 `3 + 1 = 4`，而取值 1/2/3 分别写入索引 1/2/3，索引 0 处保持
/// `null`。因此 Java 的 `getInstance(0)` 会返回 `null`，Rust 侧对应返回 `None`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccrueType {
    /// 对应 Java: `START(1)`
    Start = 1,
    /// 对应 Java: `END(2)`
    End = 2,
    /// 对应 Java: `PRORATED(3)`
    Prorated = 3,
}

impl AccrueType {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `private static final AccrueType[] TYPE_VALUES`（去掉反射建表后）。
    pub const ALL: [AccrueType; 3] = [AccrueType::Start, AccrueType::End, AccrueType::Prorated];

    /// 根据 int 值取枚举实例；越界回落到 `PRORATED`，索引 0（Java 中为 `null`）返回 `None`。
    ///
    /// 对应 Java: `public static AccrueType getInstance(int type)`
    pub fn get_instance(value: i32) -> Option<AccrueType> {
        match value {
            1 => Some(AccrueType::Start),
            2 => Some(AccrueType::End),
            3 => Some(AccrueType::Prorated),
            // Java: TYPE_VALUES 长度为 4，索引 0 处未被填充，取值结果为 null。
            0 => None,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = PRORATED.getValue(); }`
            _ => Some(AccrueType::Prorated),
        }
    }

    /// 对应 Java: `public static AccrueType getInstance(Number type)`
    ///
    /// Java 侧 `null` 走 `getInstance(-1)`，最终回落到 `PRORATED`。
    pub fn get_instance_optional(value: Option<i32>) -> Option<AccrueType> {
        AccrueType::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for AccrueType {
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<AccrueType> for i32 {
    fn from(value: AccrueType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accrue_type_round_trips() {
        for item in AccrueType::ALL {
            assert_eq!(AccrueType::get_instance(item.value()), Some(item));
        }
    }

    #[test]
    fn accrue_type_index_zero_is_none() {
        // Java: TYPE_VALUES[0] 为 null
        assert_eq!(AccrueType::get_instance(0), None);
    }

    #[test]
    fn accrue_type_falls_back_to_prorated() {
        assert_eq!(AccrueType::get_instance(-1), Some(AccrueType::Prorated));
        assert_eq!(AccrueType::get_instance(4), Some(AccrueType::Prorated));
        assert_eq!(
            AccrueType::get_instance_optional(None),
            Some(AccrueType::Prorated)
        );
    }
}
