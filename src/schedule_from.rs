//! 对应 Java: `org/mpxj/ScheduleFrom.java`

use crate::mpxj_enum::MpxjEnum;
use std::fmt::{self, Display, Formatter};

/// 「排程起始方向」的枚举。
///
/// 对应 Java: `public enum ScheduleFrom implements MpxjEnum`
///
/// Java 侧用 `EnumHelper.createTypeArray` 反射生成 int → 枚举的数组；
/// Rust 侧不使用反射，改为 [`ScheduleFrom::ALL`] 加
/// [`ScheduleFrom::get_instance`] 的 `match`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ScheduleFrom {
    /// 从开始日期排程。
    ///
    /// 对应 Java: `START(0)`
    Start = 0,
    /// 从完成日期排程。
    ///
    /// 对应 Java: `FINISH(1)`
    Finish = 1,
}

impl ScheduleFrom {
    /// 全部取值，按 int 值升序排列。
    ///
    /// 对应 Java: `private static final ScheduleFrom[] TYPE_VALUES`（不含反射部分）。
    pub const ALL: [ScheduleFrom; 2] = [ScheduleFrom::Start, ScheduleFrom::Finish];

    /// 根据 int 值取枚举实例，越界时回落到 `START`。
    ///
    /// 对应 Java: `public static ScheduleFrom getInstance(int type)`
    pub fn get_instance(value: i32) -> ScheduleFrom {
        match value {
            0 => ScheduleFrom::Start,
            1 => ScheduleFrom::Finish,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = START.getValue(); }`
            _ => ScheduleFrom::Start,
        }
    }

    /// 对应 Java: `public static ScheduleFrom getInstance(Number type)`
    ///
    /// Java 侧 `null` 走 `getInstance(-1)`，最终回落到 `START`。
    pub fn get_instance_optional(value: Option<i32>) -> ScheduleFrom {
        ScheduleFrom::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for ScheduleFrom {
    /// 返回该枚举关联的 int 值。
    ///
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for ScheduleFrom {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 返回 `Integer.toString(m_value)`，即 int 值的十进制字符串。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as i32)
    }
}

impl From<ScheduleFrom> for i32 {
    fn from(value: ScheduleFrom) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_from_round_trips() {
        for item in ScheduleFrom::ALL {
            assert_eq!(ScheduleFrom::get_instance(item.value()), item);
        }
    }

    #[test]
    fn schedule_from_falls_back_to_start() {
        assert_eq!(ScheduleFrom::get_instance(-1), ScheduleFrom::Start);
        assert_eq!(ScheduleFrom::get_instance(2), ScheduleFrom::Start);
        assert_eq!(
            ScheduleFrom::get_instance_optional(None),
            ScheduleFrom::Start
        );
    }

    #[test]
    fn schedule_from_to_string_is_decimal_value() {
        assert_eq!(ScheduleFrom::Start.to_string(), "0");
        assert_eq!(ScheduleFrom::Finish.to_string(), "1");
    }
}
