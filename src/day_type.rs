//! 对应 Java: `org/mpxj/DayType.java`

use crate::mpxj_enum::MpxjEnum;

/// 项目日历中某一天的类型。
///
/// 对应 Java: `public enum DayType implements MpxjEnum`
///
/// Java 侧用 `EnumHelper.createTypeArray(DayType.class)` 生成查找表：数组长度为 3，
/// 取值 0..2 恰好填满，因此 `getInstance` 不会返回 `null`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DayType {
    /// 对应 Java: `NON_WORKING(0)`
    NonWorking = 0,
    /// 对应 Java: `WORKING(1)`
    Working = 1,
    /// 对应 Java: `DEFAULT(2)`
    Default = 2,
}

impl DayType {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `private static final DayType[] TYPE_VALUES`（去掉反射建表后）。
    pub const ALL: [DayType; 3] = [DayType::NonWorking, DayType::Working, DayType::Default];

    /// 根据 int 值取枚举实例，越界时回落到 `NON_WORKING`。
    ///
    /// 对应 Java: `public static DayType getInstance(int type)`
    pub fn get_instance(value: i32) -> DayType {
        match value {
            0 => DayType::NonWorking,
            1 => DayType::Working,
            2 => DayType::Default,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = NON_WORKING.getValue(); }`
            _ => DayType::NonWorking,
        }
    }

    /// 对应 Java: `public static DayType getInstance(Number type)`
    ///
    /// Java 侧 `null` 走 `getInstance(-1)`，最终回落到 `NON_WORKING`。
    pub fn get_instance_optional(value: Option<i32>) -> DayType {
        DayType::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for DayType {
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<DayType> for i32 {
    fn from(value: DayType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_type_round_trips() {
        for item in DayType::ALL {
            assert_eq!(DayType::get_instance(item.value()), item);
        }
    }

    #[test]
    fn day_type_falls_back_to_non_working() {
        assert_eq!(DayType::get_instance(-1), DayType::NonWorking);
        assert_eq!(DayType::get_instance(3), DayType::NonWorking);
        assert_eq!(DayType::get_instance_optional(None), DayType::NonWorking);
    }
}
