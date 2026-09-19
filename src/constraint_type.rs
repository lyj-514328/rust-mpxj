//! 对应 Java: `org/mpxj/ConstraintType.java`

use crate::mpxj_enum::MpxjEnum;

/// 任务约束类型。
///
/// 对应 Java: `public enum ConstraintType implements MpxjEnum`
///
/// Java 侧用 `EnumHelper.createTypeArray(ConstraintType.class)` 生成查找表：数组长度
/// 为 10，取值 0..9 恰好填满，因此 `getInstance` 不会返回 `null`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstraintType {
    /// 对应 Java: `AS_SOON_AS_POSSIBLE(0)`
    AsSoonAsPossible = 0,
    /// 对应 Java: `AS_LATE_AS_POSSIBLE(1)`
    AsLateAsPossible = 1,
    /// 对应 Java: `MUST_START_ON(2)`
    MustStartOn = 2,
    /// 对应 Java: `MUST_FINISH_ON(3)`
    MustFinishOn = 3,
    /// 对应 Java: `START_NO_EARLIER_THAN(4)`
    StartNoEarlierThan = 4,
    /// 对应 Java: `START_NO_LATER_THAN(5)`
    StartNoLaterThan = 5,
    /// 对应 Java: `FINISH_NO_EARLIER_THAN(6)`
    FinishNoEarlierThan = 6,
    /// 对应 Java: `FINISH_NO_LATER_THAN(7)`
    FinishNoLaterThan = 7,
    /// 对应 Java: `START_ON(8)`
    StartOn = 8,
    /// 对应 Java: `FINISH_ON(9)`
    FinishOn = 9,
}

impl ConstraintType {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `private static final ConstraintType[] TYPE_VALUES`（去掉反射建表后）。
    pub const ALL: [ConstraintType; 10] = [
        ConstraintType::AsSoonAsPossible,
        ConstraintType::AsLateAsPossible,
        ConstraintType::MustStartOn,
        ConstraintType::MustFinishOn,
        ConstraintType::StartNoEarlierThan,
        ConstraintType::StartNoLaterThan,
        ConstraintType::FinishNoEarlierThan,
        ConstraintType::FinishNoLaterThan,
        ConstraintType::StartOn,
        ConstraintType::FinishOn,
    ];

    /// 根据 int 值取枚举实例，越界时回落到 `AS_SOON_AS_POSSIBLE`。
    ///
    /// 对应 Java: `public static ConstraintType getInstance(int type)`
    pub fn get_instance(value: i32) -> ConstraintType {
        match value {
            0 => ConstraintType::AsSoonAsPossible,
            1 => ConstraintType::AsLateAsPossible,
            2 => ConstraintType::MustStartOn,
            3 => ConstraintType::MustFinishOn,
            4 => ConstraintType::StartNoEarlierThan,
            5 => ConstraintType::StartNoLaterThan,
            6 => ConstraintType::FinishNoEarlierThan,
            7 => ConstraintType::FinishNoLaterThan,
            8 => ConstraintType::StartOn,
            9 => ConstraintType::FinishOn,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = AS_SOON_AS_POSSIBLE.getValue(); }`
            _ => ConstraintType::AsSoonAsPossible,
        }
    }

    /// 对应 Java: `public static ConstraintType getInstance(Number type)`
    ///
    /// Java 侧 `null` 走 `getInstance(-1)`，最终回落到 `AS_SOON_AS_POSSIBLE`。
    pub fn get_instance_optional(value: Option<i32>) -> ConstraintType {
        ConstraintType::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for ConstraintType {
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<ConstraintType> for i32 {
    fn from(value: ConstraintType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constraint_type_round_trips() {
        for item in ConstraintType::ALL {
            assert_eq!(ConstraintType::get_instance(item.value()), item);
        }
    }

    #[test]
    fn constraint_type_falls_back_to_as_soon_as_possible() {
        assert_eq!(
            ConstraintType::get_instance(-1),
            ConstraintType::AsSoonAsPossible
        );
        assert_eq!(
            ConstraintType::get_instance(10),
            ConstraintType::AsSoonAsPossible
        );
        assert_eq!(
            ConstraintType::get_instance_optional(None),
            ConstraintType::AsSoonAsPossible
        );
    }
}
