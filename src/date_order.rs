//! 对应 Java: `org/mpxj/DateOrder.java`

use crate::mpxj_enum::MpxjEnum;
use std::fmt::{self, Display, Formatter};

/// 日期书写顺序（月日年 / 日月年 / 年月日）。
///
/// 对应 Java: `public enum DateOrder implements MpxjEnum`
///
/// Java 侧用 `EnumHelper.createTypeArray(DateOrder.class)` 生成查找表：数组长度为 3，
/// 取值 0..2 恰好填满，因此 `getInstance` 不会返回 `null`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DateOrder {
    /// 对应 Java: `MDY(0)`
    Mdy = 0,
    /// 对应 Java: `DMY(1)`
    Dmy = 1,
    /// 对应 Java: `YMD(2)`
    Ymd = 2,
}

impl DateOrder {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `private static final DateOrder[] TYPE_VALUES`（去掉反射建表后）。
    pub const ALL: [DateOrder; 3] = [DateOrder::Mdy, DateOrder::Dmy, DateOrder::Ymd];

    /// 根据 int 值取枚举实例，越界时回落到 `MDY`。
    ///
    /// 对应 Java: `public static DateOrder getInstance(int type)`
    pub fn get_instance(value: i32) -> DateOrder {
        match value {
            0 => DateOrder::Mdy,
            1 => DateOrder::Dmy,
            2 => DateOrder::Ymd,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = MDY.getValue(); }`
            _ => DateOrder::Mdy,
        }
    }

    /// 对应 Java: `public static DateOrder getInstance(Number type)`
    ///
    /// Java 侧 `null` 走 `getInstance(-1)`，最终回落到 `MDY`。
    pub fn get_instance_optional(value: Option<i32>) -> DateOrder {
        DateOrder::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for DateOrder {
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for DateOrder {
    /// 对应 Java: `@Override public String toString()`，即 `Integer.toString(m_value)`。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

impl From<DateOrder> for i32 {
    fn from(value: DateOrder) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_order_round_trips() {
        for item in DateOrder::ALL {
            assert_eq!(DateOrder::get_instance(item.value()), item);
        }
    }

    #[test]
    fn date_order_falls_back_to_mdy() {
        assert_eq!(DateOrder::get_instance(-1), DateOrder::Mdy);
        assert_eq!(DateOrder::get_instance(3), DateOrder::Mdy);
        assert_eq!(DateOrder::get_instance_optional(None), DateOrder::Mdy);
    }

    #[test]
    fn date_order_to_string() {
        assert_eq!(DateOrder::Mdy.to_string(), "0");
        assert_eq!(DateOrder::Ymd.to_string(), "2");
    }
}
