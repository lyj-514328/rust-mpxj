//! 对应 Java: `org/mpxj/CurrencySymbolPosition.java`

use crate::mpxj_enum::MpxjEnum;
use std::fmt::{self, Display, Formatter};

/// 货币符号相对于金额的位置。
///
/// 对应 Java: `public enum CurrencySymbolPosition implements MpxjEnum`
///
/// Java 侧用 `EnumHelper.createTypeArray(CurrencySymbolPosition.class)` 生成查找表：
/// 数组长度为 4，取值 0..3 恰好填满，因此 `getInstance` 不会返回 `null`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CurrencySymbolPosition {
    /// 对应 Java: `AFTER(0)`
    After = 0,
    /// 对应 Java: `BEFORE(1)`
    Before = 1,
    /// 对应 Java: `AFTER_WITH_SPACE(2)`
    AfterWithSpace = 2,
    /// 对应 Java: `BEFORE_WITH_SPACE(3)`
    BeforeWithSpace = 3,
}

impl CurrencySymbolPosition {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `private static final CurrencySymbolPosition[] TYPE_VALUES`（去掉反射建表后）。
    pub const ALL: [CurrencySymbolPosition; 4] = [
        CurrencySymbolPosition::After,
        CurrencySymbolPosition::Before,
        CurrencySymbolPosition::AfterWithSpace,
        CurrencySymbolPosition::BeforeWithSpace,
    ];

    /// 根据 int 值取枚举实例，越界时回落到 `BEFORE`。
    ///
    /// 对应 Java: `public static CurrencySymbolPosition getInstance(int value)`
    ///
    /// Java 侧没有 `getInstance(Number)` 重载，故不提供 `get_instance_optional`。
    pub fn get_instance(value: i32) -> CurrencySymbolPosition {
        match value {
            0 => CurrencySymbolPosition::After,
            1 => CurrencySymbolPosition::Before,
            2 => CurrencySymbolPosition::AfterWithSpace,
            3 => CurrencySymbolPosition::BeforeWithSpace,
            // Java: `if (value < 0 || value >= TYPE_VALUES.length) { value = BEFORE.getValue(); }`
            _ => CurrencySymbolPosition::Before,
        }
    }
}

impl MpxjEnum for CurrencySymbolPosition {
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for CurrencySymbolPosition {
    /// 对应 Java: `@Override public String toString()`，即 `Integer.toString(m_value)`。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

impl From<CurrencySymbolPosition> for i32 {
    fn from(value: CurrencySymbolPosition) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn currency_symbol_position_round_trips() {
        for item in CurrencySymbolPosition::ALL {
            assert_eq!(CurrencySymbolPosition::get_instance(item.value()), item);
        }
    }

    #[test]
    fn currency_symbol_position_falls_back_to_before() {
        assert_eq!(
            CurrencySymbolPosition::get_instance(-1),
            CurrencySymbolPosition::Before
        );
        assert_eq!(
            CurrencySymbolPosition::get_instance(4),
            CurrencySymbolPosition::Before
        );
    }

    #[test]
    fn currency_symbol_position_to_string() {
        assert_eq!(CurrencySymbolPosition::After.to_string(), "0");
        assert_eq!(CurrencySymbolPosition::BeforeWithSpace.to_string(), "3");
    }
}
