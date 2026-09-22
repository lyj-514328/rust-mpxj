//! Corresponds to Java: `org/mpxj/Rate.java`

use crate::time_unit::TimeUnit;
use std::fmt::{self, Display, Formatter};

/// A currency rate per unit of time (for example `$10/h`).
///
/// Corresponds to Java: `public final class Rate`
///
/// Also note: Java's two public constructors (`Number` / `double` overloads, which Rust cannot overload)
/// are merged into [`Rate::new`].
#[derive(Debug, Clone, Copy)]
pub struct Rate {
/// Amount.
    ///
    /// Corresponds to Java: `private final double m_amount;`
    m_amount: f64,
/// Time unit.
    ///
    /// Corresponds to Java: `private final TimeUnit m_units;`
    m_units: TimeUnit,
}

impl Rate {
/// Constructor.
    ///
    /// Corresponds to Java: `public Rate(double amount, TimeUnit time)`
/// and `public Rate(Number amount, TimeUnit time)` (internally uses `NumberHelper.getDouble`,
/// where `null` is treated as 0; on the Rust side the caller passes an already-resolved `f64`).
    pub const fn new(amount: f64, units: TimeUnit) -> Rate {
        Rate {
            m_amount: amount,
            m_units: units,
        }
    }

/// Return the amount part.
    ///
    /// Corresponds to Java: `public double getAmount()`
    pub fn get_amount(&self) -> f64 {
        self.m_amount
    }

/// Return the time unit part.
    ///
    /// Corresponds to Java: `public TimeUnit getUnits()`
    pub fn get_units(&self) -> TimeUnit {
        self.m_units
    }

/// Compare two possibly null rates (`null` or an amount of 0 is treated as a "zero rate").
    ///
    /// Corresponds to Java: `public static boolean equals(Rate rate1, Rate rate2)`
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

/// Compare only the amount part (ignoring the unit, with a tolerance of `0.00001`).
    ///
    /// Corresponds to Java: `public boolean amountComponentEquals(Rate rhs)`
    pub fn amount_component_equals(&self, rhs: &Rate) -> bool {
        // Java: NumberHelper.equals(m_amount, rhs.m_amount, 0.00001)
        (self.m_amount - rhs.m_amount).abs() < 0.00001
    }

/// Create a `Rate` from an amount and unit; return `None` when the amount is `None`.
    ///
    /// Corresponds to Java: `public static Rate valueOf(Double value, TimeUnit units)`
    pub fn value_of(value: Option<f64>, units: TimeUnit) -> Option<Rate> {
        value.map(|value| Rate::new(value, units))
    }

/// Zero rate (per hour).
    ///
    /// Corresponds to Java: `public static final Rate ZERO`
    pub const ZERO: Rate = Rate::new(0.0, TimeUnit::Hours);
}

impl PartialEq for Rate {
/// Return `true` when both the amount (within tolerance) and the unit are equal.
    ///
    /// Corresponds to Java: `@Override public boolean equals(Object obj)`
    fn eq(&self, other: &Self) -> bool {
        self.amount_component_equals(other) && self.m_units == other.m_units
    }
}

impl Display for Rate {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Note: Java's `double` string conversion keeps `.0` (for example `10.0`), while Rust's `f64`
/// `Display` outputs `10`; the two differ slightly in this method.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.m_amount, self.m_units)
    }
}
