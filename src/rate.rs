//! 对应 Java: `org/mpxj/Rate.java`

use crate::time_unit::TimeUnit;
use std::fmt::{self, Display, Formatter};

/// 单位时间（如 `$10/h`）的货币费率。
///
/// 对应 Java: `public final class Rate`
///
/// 另注：Java 的两个公开构造方法（`Number` / `double` 重载，Rust 无法重载）
/// 合并为 [`Rate::new`]。
#[derive(Debug, Clone, Copy)]
pub struct Rate {
    /// 金额。
    ///
    /// 对应 Java: `private final double m_amount;`
    m_amount: f64,
    /// 时间单位。
    ///
    /// 对应 Java: `private final TimeUnit m_units;`
    m_units: TimeUnit,
}

impl Rate {
    /// 构造方法。
    ///
    /// 对应 Java: `public Rate(double amount, TimeUnit time)`
    /// 及 `public Rate(Number amount, TimeUnit time)`（内部用 `NumberHelper.getDouble`，
    /// `null` 视为 0；Rust 侧由调用方传入已经取好的 `f64`）。
    pub const fn new(amount: f64, units: TimeUnit) -> Rate {
        Rate {
            m_amount: amount,
            m_units: units,
        }
    }

    /// 返回金额部分。
    ///
    /// 对应 Java: `public double getAmount()`
    pub fn get_amount(&self) -> f64 {
        self.m_amount
    }

    /// 返回时间单位部分。
    ///
    /// 对应 Java: `public TimeUnit getUnits()`
    pub fn get_units(&self) -> TimeUnit {
        self.m_units
    }

    /// 比较两个可能为空的费率（`null` 或金额为 0 都按「0 费率」处理）。
    ///
    /// 对应 Java: `public static boolean equals(Rate rate1, Rate rate2)`
    pub fn equals(rate1: Option<&Rate>, rate2: Option<&Rate>) -> bool {
        if rate1.is_none_or(|rate| rate.get_amount() == 0.0)
            && rate2.is_none_or(|rate| rate.get_amount() == 0.0)
        {
            return true;
        }

        match (rate1, rate2) {
            (Some(rate1), Some(rate2)) => rate1 == rate2,
            _ => false,
        }
    }

    /// 只比较金额部分（不考虑单位，带 `0.00001` 的误差容忍）。
    ///
    /// 对应 Java: `public boolean amountComponentEquals(Rate rhs)`
    pub fn amount_component_equals(&self, rhs: &Rate) -> bool {
        // Java: NumberHelper.equals(m_amount, rhs.m_amount, 0.00001)
        (self.m_amount - rhs.m_amount).abs() < 0.00001
    }

    /// 由金额与单位创建 `Rate`；金额为 `None` 时返回 `None`。
    ///
    /// 对应 Java: `public static Rate valueOf(Double value, TimeUnit units)`
    pub fn value_of(value: Option<f64>, units: TimeUnit) -> Option<Rate> {
        value.map(|value| Rate::new(value, units))
    }

    /// 0 费率（每小时）。
    ///
    /// 对应 Java: `public static final Rate ZERO`
    pub const ZERO: Rate = Rate::new(0.0, TimeUnit::Hours);
}

impl PartialEq for Rate {
    /// 金额（在误差范围内）与单位都相等时返回 `true`。
    ///
    /// 对应 Java: `@Override public boolean equals(Object obj)`
    fn eq(&self, other: &Self) -> bool {
        self.amount_component_equals(other) && self.m_units == other.m_units
    }
}

impl Display for Rate {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// 注意：Java 的 `double` 字符串化会保留 `.0`（如 `10.0`），Rust 的 `f64`
    /// `Display` 会输出 `10`，二者在本方法上存在细微差异。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.m_amount, self.m_units)
    }
}
