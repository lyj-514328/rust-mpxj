//! 对应 Java: `org/mpxj/common/NumberHelper.java`

/// 处理 `Number` 对象与数值基本类型的工具方法。
///
/// 对应 Java: `public final class NumberHelper`
///
/// 移植说明：
/// - Java 的方法重载在 Rust 中不能同名，按参数类型加后缀区分：
///   `getInt(Number)` → [`NumberHelper::get_int`]、`getInt(String)` → [`NumberHelper::get_int_from_string`]，
///   其余重载同理。
/// - Java 的 `Number` 在 Rust 侧统一映射为 `f64`；`BigInteger` 用 `i64` 表示（未引入大整数依赖）。
/// - Java 解析失败会抛 `NumberFormatException`；Rust 侧不 panic，按各方法的注释回落到 `0` 或 `None`。
/// - Java 的 `intValue()` / `(int)` 在溢出时截断低 32 位，Rust 的 `as` 会饱和，仅溢出场景有差异。
///
/// 未移植的方法：无。
pub struct NumberHelper;

impl NumberHelper {
    /// 对应 Java: `public static final Double DOUBLE_ZERO`
    ///
    /// Java 侧用共享的 `Double` 单例节省内存；Rust 的 `f64` 不装箱，直接是普通常量。
    pub const DOUBLE_ZERO: f64 = 0.0;

    /// 对应 Java: `public static final Double DOUBLE_ONEHUNDRED`
    pub const DOUBLE_ONEHUNDRED: f64 = 100.0;

    /// 对应 Java: `public static final int getInt(Number value)`
    ///
    /// 对应 Java 的 `Number.intValue()`（向零截断），null 返回 0。
    pub fn get_int(value: Option<f64>) -> i32 {
        match value {
            Some(value) => value as i32,
            None => 0,
        }
    }

    /// 对应 Java: `public static final int getInt(String value)`
    ///
    /// null 或空串返回 0；Java 对非法输入抛 `NumberFormatException`，Rust 侧回落为 0。
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

    /// 对应 Java: `public static final double getDouble(String value)`
    ///
    /// null 或空串返回 0；Java 对非法输入抛 `NumberFormatException`，Rust 侧回落为 0。
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

    /// 对应 Java: `public static final Double getDoubleObject(String value)`
    ///
    /// null 或空串返回 `None`；Java 对非法输入抛 `NumberFormatException`，Rust 侧返回 `None`。
    pub fn get_double_object(value: Option<&str>) -> Option<f64> {
        match value {
            None => None,
            Some(text) if text.is_empty() => None,
            Some(text) => text.parse::<f64>().ok(),
        }
    }

    /// 对应 Java: `public static final Integer getInteger(Number value)`
    ///
    /// Java 先判断 `instanceof Integer`，否则取 `Math.round(doubleValue())`（.5 向正无穷取整）。
    pub fn get_integer(value: Option<f64>) -> Option<i32> {
        value.map(|value| java_math_round(value) as i32)
    }

    /// 对应 Java: `public static final Integer getInteger(String value)`
    ///
    /// Java 静默忽略解析异常并返回 null；Rust 侧同样返回 `None`。
    pub fn get_integer_from_string(value: Option<&str>) -> Option<i32> {
        value.and_then(|text| text.parse::<i32>().ok())
    }

    /// 对应 Java: `public static final BigInteger getBigInteger(Number value)`
    ///
    /// Java 返回 `BigInteger.valueOf(Math.round(...))`（无上限整数）；Rust 侧用 `i64` 表示。
    pub fn get_big_integer(value: Option<f64>) -> Option<i64> {
        value.map(|value| java_math_round(value) as i64)
    }

    /// 对应 Java: `public static final double getDouble(Number value)`
    pub fn get_double(value: Option<f64>) -> f64 {
        match value {
            Some(value) => value,
            None => 0.0,
        }
    }

    /// 对应 Java: `public static final Double getDouble(double value)`
    ///
    /// Java 用该重载复用 `DOUBLE_ZERO` 单例以节省内存；Rust 的 `f64` 不装箱，等价于直接返回入参。
    pub fn get_double_value(value: f64) -> f64 {
        value
    }

    /// 对应 Java: `public static final double round(double value, double precision)`
    ///
    /// Java 用 `Math.round`（.5 向正无穷取整），此处同样用 `floor(x + 0.5)` 复现。
    pub fn round(value: f64, precision: f64) -> f64 {
        let precision = 10f64.powf(precision);
        ((value * precision) + 0.5).floor() / precision
    }

    /// 对应 Java: `public static final Integer parseInteger(String value)`
    ///
    /// null 或空串返回 `None`；Java 对非法输入抛 `NumberFormatException`，Rust 侧返回 `None`。
    pub fn parse_integer(value: Option<&str>) -> Option<i32> {
        match value {
            None => None,
            Some(text) if text.is_empty() => None,
            Some(text) => text.parse::<i32>().ok(),
        }
    }

    /// 对应 Java: `public static boolean equals(Number lhs, Number rhs)`
    ///
    /// 两边都为 null 视为相等，单边为 null 视为不等。
    pub fn equals(lhs: Option<f64>, rhs: Option<f64>) -> bool {
        match (lhs, rhs) {
            (None, None) => true,
            (Some(lhs), Some(rhs)) => lhs == rhs,
            _ => false,
        }
    }

    /// 对应 Java: `public static int compare(Integer n1, Integer n2)`
    ///
    /// 处理 null：两边都为 null 返回 0，仅左边为 null 返回 1，仅右边为 null 返回 -1。
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

    /// 对应 Java: `public static boolean equals(double lhs, double rhs, double delta)`
    pub fn equals_with_delta(lhs: f64, rhs: f64, delta: f64) -> bool {
        (lhs - rhs).abs() < delta
    }

    /// 对应 Java: `public static Double sumAsDouble(Number... values)`
    ///
    /// Java 用 `DoubleSummaryStatistics` 做补偿求和（Neumaier 形式），Rust 侧按同一算法手工实现。
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

/// 对应 Java: `Math.round(double)`——返回最接近的整数，.5 向正无穷取整。
///
/// Java 该方法是库调用而非 `NumberHelper` 的成员，此处抽出私有函数供
/// [`NumberHelper::get_integer`] 与 [`NumberHelper::get_big_integer`] 共用。
fn java_math_round(value: f64) -> f64 {
    (value + 0.5).floor()
}
