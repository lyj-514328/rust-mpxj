//! 对应 Java: `org/mpxj/BookingType.java`

use crate::mpxj_enum::MpxjEnum;

/// 资源预订类型。
///
/// 对应 Java: `public enum BookingType implements MpxjEnum`
///
/// Java 侧用 `EnumHelper.createTypeArray(BookingType.class)` 生成查找表：数组长度为
/// 常量个数 2，索引 0/1 分别被 `COMMITTED`/`PROPOSED` 填满，因此 `getInstance`
/// 不会返回 `null`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BookingType {
    /// 对应 Java: `COMMITTED(0)`
    Committed = 0,
    /// 对应 Java: `PROPOSED(1)`
    Proposed = 1,
}

impl BookingType {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `private static final BookingType[] TYPE_VALUES`（去掉反射建表后）。
    pub const ALL: [BookingType; 2] = [BookingType::Committed, BookingType::Proposed];

    /// 根据 int 值取枚举实例，越界时回落到 `PROPOSED`。
    ///
    /// 对应 Java: `public static BookingType getInstance(int type)`
    ///
    /// Java 侧没有 `getInstance(Number)` 重载，故不提供 `get_instance_optional`。
    pub fn get_instance(value: i32) -> BookingType {
        match value {
            0 => BookingType::Committed,
            1 => BookingType::Proposed,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = PROPOSED.getValue(); }`
            _ => BookingType::Proposed,
        }
    }
}

impl MpxjEnum for BookingType {
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<BookingType> for i32 {
    fn from(value: BookingType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn booking_type_round_trips() {
        for item in BookingType::ALL {
            assert_eq!(BookingType::get_instance(item.value()), item);
        }
    }

    #[test]
    fn booking_type_falls_back_to_proposed() {
        assert_eq!(BookingType::get_instance(-1), BookingType::Proposed);
        assert_eq!(BookingType::get_instance(2), BookingType::Proposed);
    }
}
