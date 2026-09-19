//! 对应 Java: `org/mpxj/EarnedValueMethod.java`

use crate::mpxj_enum::MpxjEnum;

/// 挣值（Earned Value）计算方法。
///
/// 对应 Java: `public enum EarnedValueMethod implements MpxjEnum`
///
/// Java 侧用 `EnumHelper.createTypeArray(EarnedValueMethod.class)` 生成查找表：数组长度
/// 为 2，取值 0/1 恰好填满，因此 `getInstance` 不会返回 `null`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EarnedValueMethod {
    /// 对应 Java: `PERCENT_COMPLETE(0)`
    PercentComplete = 0,
    /// 对应 Java: `PHYSICAL_PERCENT_COMPLETE(1)`
    PhysicalPercentComplete = 1,
}

impl EarnedValueMethod {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `private static final EarnedValueMethod[] TYPE_VALUES`（去掉反射建表后）。
    pub const ALL: [EarnedValueMethod; 2] = [
        EarnedValueMethod::PercentComplete,
        EarnedValueMethod::PhysicalPercentComplete,
    ];

    /// 根据 int 值取枚举实例，越界时回落到 `PHYSICAL_PERCENT_COMPLETE`。
    ///
    /// 对应 Java: `public static EarnedValueMethod getInstance(int type)`
    pub fn get_instance(value: i32) -> EarnedValueMethod {
        match value {
            0 => EarnedValueMethod::PercentComplete,
            1 => EarnedValueMethod::PhysicalPercentComplete,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = PHYSICAL_PERCENT_COMPLETE.getValue(); }`
            _ => EarnedValueMethod::PhysicalPercentComplete,
        }
    }

    /// 对应 Java: `public static EarnedValueMethod getInstance(Number type)`
    ///
    /// Java 侧 `null` 走 `getInstance(-1)`，最终回落到 `PHYSICAL_PERCENT_COMPLETE`。
    pub fn get_instance_optional(value: Option<i32>) -> EarnedValueMethod {
        EarnedValueMethod::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for EarnedValueMethod {
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<EarnedValueMethod> for i32 {
    fn from(value: EarnedValueMethod) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earned_value_method_round_trips() {
        for item in EarnedValueMethod::ALL {
            assert_eq!(EarnedValueMethod::get_instance(item.value()), item);
        }
    }

    #[test]
    fn earned_value_method_falls_back_to_physical_percent_complete() {
        assert_eq!(
            EarnedValueMethod::get_instance(-1),
            EarnedValueMethod::PhysicalPercentComplete
        );
        assert_eq!(
            EarnedValueMethod::get_instance(2),
            EarnedValueMethod::PhysicalPercentComplete
        );
        assert_eq!(
            EarnedValueMethod::get_instance_optional(None),
            EarnedValueMethod::PhysicalPercentComplete
        );
    }
}
