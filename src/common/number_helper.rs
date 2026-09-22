//! Corresponds to Java: `org/mpxj/common/NumberHelper.java`

/// Utility methods for handling `Number` objects and numeric primitive types.
///
/// Corresponds to Java: `public final class NumberHelper`
///
/// Porting notes:
/// - Java's method overloads cannot share a name in Rust, so they are distinguished by a
///   suffix on the parameter type: `getInt(Number)` → [`NumberHelper::get_int`],
///   `getInt(String)` → [`NumberHelper::get_int_from_string`], and likewise for the other
///   overloads.
/// - Java's `Number` maps uniformly to `f64` on the Rust side; `BigInteger` is represented
///   by `i64` (no big-integer dependency is introduced).
/// - Java throws `NumberFormatException` on parse failure; the Rust side does not panic and
///   falls back to `0` or `None` as documented per method.
/// - Java's `intValue()` / `(int)` truncate to the low 32 bits on overflow, whereas Rust's
///   `as` saturates; they differ only in overflow scenarios.
///
/// Unported methods: none.
pub struct NumberHelper;

impl NumberHelper {
    /// Corresponds to Java: `public static final Double DOUBLE_ZERO`
    ///
    /// In Java a shared `Double` singleton is used to save memory; Rust's `f64` is unboxed,
    /// so it is just an ordinary constant.
    pub const DOUBLE_ZERO: f64 = 0.0;

    /// Corresponds to Java: `public static final Double DOUBLE_ONEHUNDRED`
    pub const DOUBLE_ONEHUNDRED: f64 = 100.0;

    /// Corresponds to Java: `public static final int getInt(Number value)`
    ///
    /// Corresponds to Java's `Number.intValue()` (truncation toward zero); returns 0 for null.
    pub fn get_int(value: Option<f64>) -> i32 {
        match value {
            Some(value) => value as i32,
            None => 0,
        }
    }

    /// Corresponds to Java: `public static final int getInt(String value)`
    ///
    /// Returns 0 for null or an empty string; Java throws `NumberFormatException` for invalid
    /// input, the Rust side falls back to 0.
    pub fn get_int_from_string(value: Option<&str>) -> i32 {
        match value {
            None => 0,
            Some(text) if text.is_empty() => 0,
            Some(text) => match text.parse::<i32>() {
                Ok(result) => result,
                Err(_) => 0,
            },
        }
    }

    /// Corresponds to Java: `public static final double getDouble(String value)`
    ///
    /// Returns 0 for null or an empty string; Java throws `NumberFormatException` for invalid
    /// input, the Rust side falls back to 0.
    pub fn get_double_from_string(value: Option<&str>) -> f64 {
        match value {
            None => 0.0,
            Some(text) if text.is_empty() => 0.0,
            Some(text) => match text.parse::<f64>() {
                Ok(result) => result,
                Err(_) => 0.0,
            },
        }
    }

    /// Corresponds to Java: `public static final Double getDoubleObject(String value)`
    ///
    /// Returns `None` for null or an empty string; Java throws `NumberFormatException` for
    /// invalid input, the Rust side returns `None`.
    pub fn get_double_object(value: Option<&str>) -> Option<f64> {
        match value {
            None => None,
            Some(text) if text.is_empty() => None,
            Some(text) => text.parse::<f64>().ok(),
        }
    }

    /// Corresponds to Java: `public static final Integer getInteger(Number value)`
    ///
    /// Java first checks `instanceof Integer`, otherwise uses `Math.round(doubleValue())`
    /// (.5 rounds toward positive infinity).
    pub fn get_integer(value: Option<f64>) -> Option<i32> {
        value.map(|value| java_math_round(value) as i32)
    }

    /// Corresponds to Java: `public static final Integer getInteger(String value)`
    ///
    /// Java silently ignores parse exceptions and returns null; the Rust side likewise
    /// returns `None`.
    pub fn get_integer_from_string(value: Option<&str>) -> Option<i32> {
        value.and_then(|text| text.parse::<i32>().ok())
    }

    /// Corresponds to Java: `public static final BigInteger getBigInteger(Number value)`
    ///
    /// Java returns `BigInteger.valueOf(Math.round(...))` (an unbounded integer); the Rust
    /// side uses `i64`.
    pub fn get_big_integer(value: Option<f64>) -> Option<i64> {
        value.map(|value| java_math_round(value) as i64)
    }

    /// Corresponds to Java: `public static final double getDouble(Number value)`
    pub fn get_double(value: Option<f64>) -> f64 {
        match value {
            Some(value) => value,
            None => 0.0,
        }
    }

    /// Corresponds to Java: `public static final Double getDouble(double value)`
    ///
    /// Java uses this overload to reuse the `DOUBLE_ZERO` singleton to save memory; Rust's
    /// `f64` is unboxed, so this is equivalent to returning the argument directly.
    pub fn get_double_value(value: f64) -> f64 {
        value
    }

    /// Corresponds to Java: `public static final double round(double value, double precision)`
    ///
    /// Java uses `Math.round` (.5 rounds toward positive infinity), reproduced here with
    /// `floor(x + 0.5)`.
    pub fn round(value: f64, precision: f64) -> f64 {
        let precision = 10f64.powf(precision);
        ((value * precision) + 0.5).floor() / precision
    }

    /// Corresponds to Java: `public static final Integer parseInteger(String value)`
    ///
    /// Returns `None` for null or an empty string; Java throws `NumberFormatException` for
    /// invalid input, the Rust side returns `None`.
    pub fn parse_integer(value: Option<&str>) -> Option<i32> {
        match value {
            None => None,
            Some(text) if text.is_empty() => None,
            Some(text) => text.parse::<i32>().ok(),
        }
    }

    /// Corresponds to Java: `public static boolean equals(Number lhs, Number rhs)`
    ///
    /// Both sides null counts as equal; one side null counts as not equal.
    pub fn equals(lhs: Option<f64>, rhs: Option<f64>) -> bool {
        match (lhs, rhs) {
            (None, None) => true,
            (Some(lhs), Some(rhs)) => lhs == rhs,
            _ => false,
        }
    }

    /// Corresponds to Java: `public static int compare(Integer n1, Integer n2)`
    ///
    /// Handles null: both null returns 0, only left null returns 1, only right null returns -1.
    pub fn compare(n1: Option<i32>, n2: Option<i32>) -> i32 {
        match (n1, n2) {
            (None, None) => 0,
            (None, Some(_)) => 1,
            (Some(_), None) => -1,
            (Some(n1), Some(n2)) => match n1.cmp(&n2) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            },
        }
    }

    /// Corresponds to Java: `public static boolean equals(double lhs, double rhs, double delta)`
    pub fn equals_with_delta(lhs: f64, rhs: f64, delta: f64) -> bool {
        (lhs - rhs).abs() < delta
    }

    /// Corresponds to Java: `public static Double sumAsDouble(Number... values)`
    ///
    /// Java uses `DoubleSummaryStatistics` for compensated summation (Neumaier form); the
    /// Rust side implements the same algorithm by hand.
    pub fn sum_as_double(values: &[f64]) -> f64 {
        let mut sum = 0.0;
        let mut compensation = 0.0;

        for value in values {
            let tmp = sum + value;
            if sum.abs() >= value.abs() {
                compensation += (sum - tmp) + value;
            } else {
                compensation += (value - tmp) + sum;
            }
            sum = tmp;
        }

        sum + compensation
    }
}

/// Corresponds to Java: `Math.round(double)`—returns the nearest integer, rounding .5 toward
/// positive infinity.
///
/// In Java this is a library call rather than a member of `NumberHelper`; it is extracted here
/// as a private function shared by [`NumberHelper::get_integer`] and
/// [`NumberHelper::get_big_integer`].
fn java_math_round(value: f64) -> f64 {
    (value + 0.5).floor()
}
