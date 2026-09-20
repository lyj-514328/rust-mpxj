//! 对应 Java: `org/mpxj/TestOperator.java`
//!
//! 跳过/未移植的 Java 方法与原因：
//!
//! | Java | Rust |
//! |---|---|
//! | `TYPE_VALUES = EnumHelper.createTypeArray(TestOperator.class)` | [`TestOperator::ALL`] + [`TestOperator::get_instance`]（无反射：Java 侧该数组按下标 `getValue()` 建表，`0..=13` 连续，`match` 即可等价表达） |
//! | `public static TestOperator getInstance(int type)` | [`TestOperator::get_instance`] |
//! | `public static TestOperator getInstance(Number type)` | [`TestOperator::get_instance_optional`]（`null` → `-1` → 回落到 `IS_ANY_VALUE`，与 Java 一致） |
//! | `public int getValue()` | [`MpxjEnum::value`] |
//! | `public abstract boolean evaluate(Object lhs, Object rhs)` | [`TestOperator::evaluate`] |
//! | `protected Object getSingleOperand(Object operand)` | [`TestOperator::get_single_operand`] |
//! | `protected boolean evaluateWithin(Object lhs, Object rhs)` | [`TestOperator::evaluate_within`] |
//! | `protected int evaluateCompareTo(Object lhs, Object rhs)` | [`TestOperator::evaluate_compare_to`]（返回 `Option<Ordering>`，见下） |
//! | `protected boolean evaluateContains(Object lhs, Object rhs)` | [`TestOperator::evaluate_contains`] |
//! | `protected boolean evaluateContainsExactly(Object lhs, Object rhs)` | [`TestOperator::evaluate_contains_exactly`] |
//! | `@Override public String toString()`（枚举默认实现） | [`Display`]（输出常量名，如 `"EQUALS"`） |
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//!
//! # 与 Java 的差异
//!
//! 1. **`Object` 操作数**：Java 的 `evaluate` / `setRightValue` 用 `Object` 承载
//!    字符串 / 数值 / 日期 / 工期 / `FieldType` / `GenericCriteriaPrompt`。
//!    Rust 侧引入 [`Operand`] 作为这组取值（见其说明），`evaluate` 的签名为
//!    `evaluate(Option<&Operand>, &[Option<Operand>])`：
//!    * `lhs` 用 `Option` 表达 Java 的 `null`；
//!    * `rhs` 用切片表达 Java 的「单值 **或** `Object[]`」——
//!      `GenericCriteria` 侧恒为长度 2 的数组，长度 1 的切片对应 Java 的「非数组单值」
//!      （`IS_WITHIN` / `IS_NOT_WITHIN` 在非数组时结果为 `false`，与 Java 一致）。
//! 2. **`compareTo` 的失败语义**：Java 侧 `((Comparable) lhs).compareTo(rhs)` 在类型不兼容时抛
//!    `ClassCastException`；Rust 侧 [`TestOperator::evaluate_compare_to`] 返回 `Option<Ordering>`，
//!    不可比较时为 `None`（调用方按 `false` 处理，不 panic）。
//! 3. **`AND` / `OR`**：Java 侧 `evaluate` 直接抛 `UnsupportedOperationException`
//!    （这两个常量只作为 `GenericCriteria` 的逻辑组合子使用）；Rust 库代码不 panic，
//!    故返回 `false`（实际取值路径不会碰到，`GenericCriteria::evaluate` 会先行分流）。
//! 4. **大小写不敏感比较**：Java 用 `toUpperCase()`（Unicode），Rust 侧同样用
//!    [`str::to_uppercase`]（Unicode），行为一致。
//! 5. **二进制操作数**：Java `byte[]` 默认按对象地址字符串化，Rust 用 `Debug` 输出字节；
//!    筛选表达式不使用 OLE 自定义属性里的二进制值。

use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};

use crate::field_key::FieldKey;
use crate::field_value::FieldValue;
use crate::generic_criteria_prompt::GenericCriteriaPrompt;
use crate::mpxj_enum::MpxjEnum;

/// 对应 Java 中作为操作数传递的 `Object`。
///
/// `TestOperator.evaluate(Object lhs, Object rhs)` 与 `GenericCriteria` 的
/// `Object[] m_definedRightValues` / `Map<GenericCriteriaPrompt, Object>` 都用 `Object`
/// 承载取值；Rust 侧用本枚举统一表达：
///
/// | Java 取值 | Rust |
/// |---|---|
/// | `FieldType`（符号值） | [`Operand::Field`] |
/// | `GenericCriteriaPrompt`（提示值） | [`Operand::Prompt`] |
/// | 字符串 / 数值 / 布尔 / 日期 / 工期 / 费率 | [`Operand::Value`]（承载 [`FieldValue`]） |
/// | `null` | `Option::None` |
///
/// 符号值与提示值在 `GenericCriteria` 求值前会被解析为具体取值（见
/// [`crate::generic_criteria::GenericCriteria::evaluate`]）；若它们被直接送入
/// `evaluate`（Java 侧「只设置提示、未设置符号值」的路径），则与任何取值都不相等，
/// 与 Java 中 `String.equals(GenericCriteriaPrompt)` 的结果一致。
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    /// 对应 Java: 作为操作数的 `FieldType`。
    Field(FieldKey),
    /// 对应 Java: 作为操作数的 `GenericCriteriaPrompt`。
    Prompt(GenericCriteriaPrompt),
    /// 对应 Java: 作为操作数的标量取值（`String` / `Number` / `LocalDateTime` / `Duration` ...）。
    Value(FieldValue),
}

impl Operand {
    /// 取具体取值（非 [`Operand::Value`] 时返回 `None`）。
    pub fn as_value(&self) -> Option<&FieldValue> {
        match self {
            Operand::Value(value) => Some(value),
            _ => None,
        }
    }

    /// 取字符串取值（Java: `lhs instanceof String`）。
    pub fn as_string(&self) -> Option<&str> {
        self.as_value().and_then(FieldValue::as_string)
    }

    /// 复刻 Java `Comparable.compareTo` 的比较语义，不可比较时返回 `None`
    /// （Java 抛 `ClassCastException`）。
    ///
    /// 仅同样为具体取值的操作数之间可比较；数值类型不跨类型比较
    /// （Java 的 `Integer.compareTo(Double)` 会抛 `ClassCastException`）。
    pub fn compare_to(&self, other: &Operand) -> Option<Ordering> {
        match (self, other) {
            (Operand::Value(lhs), Operand::Value(rhs)) => compare_field_values(lhs, rhs),
            _ => None,
        }
    }
}

/// 比较两个字段取值，不可比较时返回 `None`。
fn compare_field_values(lhs: &FieldValue, rhs: &FieldValue) -> Option<Ordering> {
    match (lhs, rhs) {
        (FieldValue::Integer(lhs), FieldValue::Integer(rhs)) => Some(lhs.cmp(rhs)),
        (FieldValue::Short(lhs), FieldValue::Short(rhs)) => Some(lhs.cmp(rhs)),
        (FieldValue::Double(lhs), FieldValue::Double(rhs)) => lhs.partial_cmp(rhs),
        (FieldValue::Boolean(lhs), FieldValue::Boolean(rhs)) => Some(lhs.cmp(rhs)),
        (FieldValue::String(lhs), FieldValue::String(rhs)) => Some(lhs.cmp(rhs)),
        (FieldValue::Date(lhs), FieldValue::Date(rhs)) => Some(lhs.cmp(rhs)),
        (FieldValue::Time(lhs), FieldValue::Time(rhs)) => Some(lhs.cmp(rhs)),
        (FieldValue::DateTime(lhs), FieldValue::DateTime(rhs)) => Some(lhs.cmp(rhs)),
        (FieldValue::Duration(lhs), FieldValue::Duration(rhs)) => Some(lhs.cmp(rhs)),
        // Java 的 `Rate` 未实现 `Comparable`；此处按金额比较（不比较单位），供筛选使用。
        (FieldValue::Rate(lhs), FieldValue::Rate(rhs)) => {
            lhs.get_amount().partial_cmp(&rhs.get_amount())
        }
        _ => None,
    }
}

impl From<FieldValue> for Operand {
    /// 由具体取值构造操作数（Rust 特有便捷入口）。
    fn from(value: FieldValue) -> Self {
        Operand::Value(value)
    }
}

impl Display for Operand {
    /// 复刻 Java 字符串拼接时的取值渲染（`String.valueOf`）；`Operand` 本身不是
    /// Java 类型，此为 Rust 侧格式化辅助。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Field(field) => write!(f, "{field}"),
            Operand::Prompt(prompt) => write!(f, "{prompt}"),
            Operand::Value(value) => f.write_str(&render_field_value(value)),
        }
    }
}

/// 按 Java 的 `toString()` 风格渲染字段取值。
fn render_field_value(value: &FieldValue) -> String {
    match value {
        FieldValue::Integer(value) => value.to_string(),
        FieldValue::Short(value) => value.to_string(),
        FieldValue::Double(value) => render_double(*value),
        FieldValue::Boolean(value) => value.to_string(),
        FieldValue::String(value) => value.clone(),
        FieldValue::Date(value) => value.to_string(),
        FieldValue::Time(value) => value.to_string(),
        FieldValue::DateTime(value) => value.to_string(),
        FieldValue::Duration(value) => value.to_string(),
        FieldValue::Rate(value) => value.to_string(),
        FieldValue::Uuid(value) => value.to_string(),
        FieldValue::TimeUnit(value) => value.to_string(),
        FieldValue::Notes(value) => value.to_string(),
        // Java 未覆写 `toString()` 的类型（`WorkContour` / 枚举）与集合 / 实体引用型取值：
        // 本仓约定改用 `Debug` 渲染（见各文件头「Java 未覆写 toString 的类改为 Debug」），
        // 它们不会出现在筛选表达式的操作数里。
        FieldValue::Bytes(_)
        | FieldValue::WorkContour(_)
        | FieldValue::RateSource(_)
        | FieldValue::ResourceRequestType(_)
        | FieldValue::Steps(_)
        | FieldValue::Relations(_)
        | FieldValue::Resource(_)
        | FieldValue::TimephasedWork(_)
        | FieldValue::TimephasedCost(_)
        | FieldValue::TimeRanges(_) => format!("{value:?}"),
    }
}

/// 复刻 Java `Double.toString` 的整数尾缀（`1.0` 而非 `1`）。
fn render_double(value: f64) -> String {
    let text = value.to_string();
    if text.contains('.') || text.contains('e') || text.contains('E') {
        text
    } else {
        format!("{text}.0")
    }
}

/// 用于两个操作数之间比较的运算符。
///
/// 对应 Java: `public enum TestOperator implements MpxjEnum`
///
/// 常量声明顺序与 `getValue()` 取值同 Java（`0..=13` 连续）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestOperator {
    /// 对应 Java: `IS_ANY_VALUE(0)`
    IsAnyValue = 0,
    /// 对应 Java: `IS_WITHIN(1)`
    IsWithin = 1,
    /// 对应 Java: `IS_GREATER_THAN(2)`
    IsGreaterThan = 2,
    /// 对应 Java: `IS_LESS_THAN(3)`
    IsLessThan = 3,
    /// 对应 Java: `IS_GREATER_THAN_OR_EQUAL_TO(4)`
    IsGreaterThanOrEqualTo = 4,
    /// 对应 Java: `IS_LESS_THAN_OR_EQUAL_TO(5)`
    IsLessThanOrEqualTo = 5,
    /// 对应 Java: `EQUALS(6)`
    Equals = 6,
    /// 对应 Java: `DOES_NOT_EQUAL(7)`
    DoesNotEqual = 7,
    /// 对应 Java: `CONTAINS(8)`
    Contains = 8,
    /// 对应 Java: `IS_NOT_WITHIN(9)`
    IsNotWithin = 9,
    /// 对应 Java: `DOES_NOT_CONTAIN(10)`
    DoesNotContain = 10,
    /// 对应 Java: `CONTAINS_EXACTLY(11)`
    ContainsExactly = 11,
    /// 对应 Java: `AND(12)`（MPXJ 扩展，非 MS Project 定义）
    And = 12,
    /// 对应 Java: `OR(13)`（MPXJ 扩展，非 MS Project 定义）
    Or = 13,
}

impl TestOperator {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `values()`（`EnumHelper.createTypeArray` 依赖它建表）。
    pub const ALL: [TestOperator; 14] = [
        TestOperator::IsAnyValue,
        TestOperator::IsWithin,
        TestOperator::IsGreaterThan,
        TestOperator::IsLessThan,
        TestOperator::IsGreaterThanOrEqualTo,
        TestOperator::IsLessThanOrEqualTo,
        TestOperator::Equals,
        TestOperator::DoesNotEqual,
        TestOperator::Contains,
        TestOperator::IsNotWithin,
        TestOperator::DoesNotContain,
        TestOperator::ContainsExactly,
        TestOperator::And,
        TestOperator::Or,
    ];

    /// Java 侧 `TYPE_VALUES` 数组长度（`EnumHelper.createTypeArray` 以最大 `getValue() + 1` 建表）。
    const TYPE_VALUES_LEN: i32 = 14;

    /// 根据 int 值取枚举实例，越界回落到 `IS_ANY_VALUE`。
    ///
    /// 对应 Java: `public static TestOperator getInstance(int type)`
    pub fn get_instance(value: i32) -> TestOperator {
        if !(0..Self::TYPE_VALUES_LEN).contains(&value) {
            return TestOperator::IsAnyValue;
        }

        match value {
            0 => TestOperator::IsAnyValue,
            1 => TestOperator::IsWithin,
            2 => TestOperator::IsGreaterThan,
            3 => TestOperator::IsLessThan,
            4 => TestOperator::IsGreaterThanOrEqualTo,
            5 => TestOperator::IsLessThanOrEqualTo,
            6 => TestOperator::Equals,
            7 => TestOperator::DoesNotEqual,
            8 => TestOperator::Contains,
            9 => TestOperator::IsNotWithin,
            10 => TestOperator::DoesNotContain,
            11 => TestOperator::ContainsExactly,
            12 => TestOperator::And,
            13 => TestOperator::Or,
            // Java 侧该数组下标都有对应常量；此处仅为穷尽匹配。
            _ => TestOperator::IsAnyValue,
        }
    }

    /// 根据可空的 int 值取枚举实例（`null` 等价于 `-1`，回落到 `IS_ANY_VALUE`）。
    ///
    /// 对应 Java: `public static TestOperator getInstance(Number type)`
    pub fn get_instance_optional(value: Option<i32>) -> TestOperator {
        TestOperator::get_instance(value.unwrap_or(-1))
    }

    /// 按本运算符比较操作数。
    ///
    /// 对应 Java: `public abstract boolean evaluate(Object lhs, Object rhs)`
    ///
    /// `rhs` 为切片：长度 1 表示 Java 的非数组单值，长度 ≥ 2 表示 Java 的 `Object[]`
    /// （`GenericCriteria` 恒传长度 2 的数组）。
    pub fn evaluate(&self, lhs: Option<&Operand>, rhs: &[Option<Operand>]) -> bool {
        match self {
            TestOperator::IsAnyValue => true,
            TestOperator::IsWithin => self.evaluate_within(lhs, rhs),
            TestOperator::IsGreaterThan => {
                self.compare_result(lhs, rhs) == Some(Ordering::Greater)
            }
            TestOperator::IsLessThan => self.compare_result(lhs, rhs) == Some(Ordering::Less),
            TestOperator::IsGreaterThanOrEqualTo => matches!(
                self.compare_result(lhs, rhs),
                Some(Ordering::Greater) | Some(Ordering::Equal)
            ),
            TestOperator::IsLessThanOrEqualTo => matches!(
                self.compare_result(lhs, rhs),
                Some(Ordering::Less) | Some(Ordering::Equal)
            ),
            TestOperator::Equals => match lhs {
                None => self.get_single_operand(rhs).is_none(),
                Some(lhs) => self.get_single_operand(rhs) == Some(lhs),
            },
            TestOperator::DoesNotEqual => match lhs {
                None => self.get_single_operand(rhs).is_some(),
                Some(lhs) => self.get_single_operand(rhs) != Some(lhs),
            },
            TestOperator::Contains => self.evaluate_contains(lhs, rhs),
            TestOperator::IsNotWithin => !self.evaluate_within(lhs, rhs),
            TestOperator::DoesNotContain => !self.evaluate_contains(lhs, rhs),
            TestOperator::ContainsExactly => self.evaluate_contains_exactly(lhs, rhs),
            // Java 抛 UnsupportedOperationException；Rust 不 panic，返回 false。
            TestOperator::And | TestOperator::Or => false,
        }
    }

    /// 取 RHS 中的单个操作数（Java 的 `Object[]` 取下标 0）。
    ///
    /// 对应 Java: `protected Object getSingleOperand(Object operand)`
    pub fn get_single_operand<'a>(
        &self,
        operand: &'a [Option<Operand>],
    ) -> Option<&'a Operand> {
        // Java: 若 operand 是 Object[] 取 list[0]，否则取 operand 自身；
        // 由于非数组单值在 Rust 中以长度 1 的切片表达，两种情况都等价于取首元素。
        operand.first().and_then(Option::as_ref)
    }

    /// 判断 LHS 是否落在 RHS 给出的区间内。
    ///
    /// 对应 Java: `protected boolean evaluateWithin(Object lhs, Object rhs)`
    ///
    /// Project 也会尝试翻转后的区间，因此两个方向都接受；`rhs` 长度小于 2
    /// （Java 中「不是 `Object[]`」）时结果为 `false`。
    pub fn evaluate_within(&self, lhs: Option<&Operand>, rhs: &[Option<Operand>]) -> bool {
        if rhs.len() < 2 {
            return false;
        }

        let first = rhs[0].as_ref();
        let second = rhs[1].as_ref();

        let Some(lhs) = lhs else {
            // Project 也尊重 null 相等（例如 NA 日期）
            return first.is_none() || second.is_none();
        };

        let (Some(first), Some(second)) = (first, second) else {
            return false;
        };

        let (Some(first), Some(second)) = (lhs.compare_to(first), lhs.compare_to(second)) else {
            return false;
        };

        (first != Ordering::Less && second != Ordering::Greater)
            || (first != Ordering::Greater && second != Ordering::Less)
    }

    /// 复刻 Java 的 `compareTo` 结果（`lhs` 为 `null` 视为更大，与 Java 的
    /// `result = 1` 一致）。
    ///
    /// 对应 Java: `protected int evaluateCompareTo(Object lhs, Object rhs)`
    ///
    /// Rust 侧返回 `Option<Ordering>`：不可比较时返回 `None`（Java 抛
    /// `ClassCastException`）。
    pub fn evaluate_compare_to(
        &self,
        lhs: Option<&Operand>,
        rhs: &[Option<Operand>],
    ) -> Option<Ordering> {
        let rhs = self.get_single_operand(rhs);

        match (lhs, rhs) {
            (None, None) => Some(Ordering::Equal),
            (None, Some(_)) => Some(Ordering::Greater),
            (Some(_), None) => Some(Ordering::Less),
            (Some(lhs), Some(rhs)) => lhs.compare_to(rhs),
        }
    }

    /// 判断 RHS（字符串）是否被包含在 LHS（字符串）中，大小写不敏感。
    ///
    /// 对应 Java: `protected boolean evaluateContains(Object lhs, Object rhs)`
    pub fn evaluate_contains(&self, lhs: Option<&Operand>, rhs: &[Option<Operand>]) -> bool {
        self.contains_with(lhs, rhs, true)
    }

    /// 判断 RHS（字符串）是否被包含在 LHS（字符串）中，大小写敏感。
    ///
    /// 对应 Java: `protected boolean evaluateContainsExactly(Object lhs, Object rhs)`
    pub fn evaluate_contains_exactly(
        &self,
        lhs: Option<&Operand>,
        rhs: &[Option<Operand>],
    ) -> bool {
        self.contains_with(lhs, rhs, false)
    }

    /// `evaluateContains` / `evaluateContainsExactly` 的公共实现。
    fn contains_with(
        &self,
        lhs: Option<&Operand>,
        rhs: &[Option<Operand>],
        ignore_case: bool,
    ) -> bool {
        let rhs = self.get_single_operand(rhs);

        let (Some(lhs), Some(rhs)) = (lhs.and_then(Operand::as_string), rhs.and_then(Operand::as_string))
        else {
            return false;
        };

        if ignore_case {
            lhs.to_uppercase().contains(&rhs.to_uppercase())
        } else {
            lhs.contains(rhs)
        }
    }

    /// `evaluate_compare_to` 的 `Option` 版本（供 `evaluate` 内部使用）。
    fn compare_result(&self, lhs: Option<&Operand>, rhs: &[Option<Operand>]) -> Option<Ordering> {
        self.evaluate_compare_to(lhs, rhs)
    }
}

impl MpxjEnum for TestOperator {
    /// 返回该运算符的 int 表示。
    ///
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for TestOperator {
    /// Java 枚举未覆写 `toString()`，输出常量名（如 `"EQUALS"`）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            TestOperator::IsAnyValue => "IS_ANY_VALUE",
            TestOperator::IsWithin => "IS_WITHIN",
            TestOperator::IsGreaterThan => "IS_GREATER_THAN",
            TestOperator::IsLessThan => "IS_LESS_THAN",
            TestOperator::IsGreaterThanOrEqualTo => "IS_GREATER_THAN_OR_EQUAL_TO",
            TestOperator::IsLessThanOrEqualTo => "IS_LESS_THAN_OR_EQUAL_TO",
            TestOperator::Equals => "EQUALS",
            TestOperator::DoesNotEqual => "DOES_NOT_EQUAL",
            TestOperator::Contains => "CONTAINS",
            TestOperator::IsNotWithin => "IS_NOT_WITHIN",
            TestOperator::DoesNotContain => "DOES_NOT_CONTAIN",
            TestOperator::ContainsExactly => "CONTAINS_EXACTLY",
            TestOperator::And => "AND",
            TestOperator::Or => "OR",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 便捷构造：字符串操作数。
    fn text(value: &str) -> Operand {
        Operand::Value(FieldValue::String(value.to_owned()))
    }

    /// 便捷构造：整数操作数。
    fn int(value: i32) -> Operand {
        Operand::Value(FieldValue::Integer(value))
    }

    #[test]
    fn all_constants_round_trip_through_get_instance() {
        for operator in TestOperator::ALL {
            assert_eq!(TestOperator::get_instance(operator.value()), operator);
        }
    }

    #[test]
    fn get_instance_falls_back_to_is_any_value() {
        assert_eq!(TestOperator::get_instance(-1), TestOperator::IsAnyValue);
        assert_eq!(TestOperator::get_instance(14), TestOperator::IsAnyValue);
        assert_eq!(
            TestOperator::get_instance_optional(None),
            TestOperator::IsAnyValue
        );
        assert_eq!(
            TestOperator::get_instance_optional(Some(6)),
            TestOperator::Equals
        );
    }

    #[test]
    fn is_any_value_always_matches() {
        assert!(TestOperator::IsAnyValue.evaluate(None, &[]));
        assert!(TestOperator::IsAnyValue.evaluate(Some(&int(1)), &[Some(text("x"))]));
    }

    #[test]
    fn equals_and_does_not_equal_handle_null() {
        let pair = [Some(text("abc")), None];

        assert!(TestOperator::Equals.evaluate(Some(&text("abc")), &pair));
        assert!(!TestOperator::Equals.evaluate(Some(&text("abd")), &pair));
        assert!(TestOperator::Equals.evaluate(None, &[None, None]));
        assert!(!TestOperator::Equals.evaluate(None, &pair));

        assert!(!TestOperator::DoesNotEqual.evaluate(Some(&text("abc")), &pair));
        assert!(TestOperator::DoesNotEqual.evaluate(Some(&text("abd")), &pair));
        assert!(TestOperator::DoesNotEqual.evaluate(None, &pair));
        assert!(!TestOperator::DoesNotEqual.evaluate(None, &[None, None]));
    }

    #[test]
    fn compare_operators_use_ordering() {
        let pair = [Some(int(10)), None];
        let lhs = int(5);

        assert!(TestOperator::IsLessThan.evaluate(Some(&lhs), &pair));
        assert!(!TestOperator::IsGreaterThan.evaluate(Some(&lhs), &pair));
        assert!(TestOperator::IsLessThanOrEqualTo.evaluate(Some(&lhs), &pair));
        assert!(!TestOperator::IsGreaterThanOrEqualTo.evaluate(Some(&lhs), &pair));

        let lhs = int(10);
        assert!(TestOperator::IsGreaterThanOrEqualTo.evaluate(Some(&lhs), &pair));
        assert!(TestOperator::IsLessThanOrEqualTo.evaluate(Some(&lhs), &pair));

        // lhs 为 null 时 Java 的 evaluateCompareTo 返回 1（视为更大）
        assert!(TestOperator::IsGreaterThan.evaluate(None, &pair));
    }

    #[test]
    fn compare_operators_return_false_for_incompatible_types() {
        // Java 抛 ClassCastException；Rust 按 false 处理
        let pair = [Some(text("abc")), None];
        assert!(!TestOperator::IsGreaterThan.evaluate(Some(&int(1)), &pair));

        // 数值不跨类型比较（Java 的 Integer.compareTo(Double) 抛 ClassCastException）
        let pair = [Some(Operand::Value(FieldValue::Double(1.0))), None];
        assert!(!TestOperator::IsGreaterThan.evaluate(Some(&int(2)), &pair));
    }

    #[test]
    fn within_accepts_both_directions_and_requires_pair() {
        let lhs = int(5);

        let pair = [Some(int(1)), Some(int(10))];
        assert!(TestOperator::IsWithin.evaluate(Some(&lhs), &pair));

        // Project 也尝试翻转后的区间
        let pair = [Some(int(10)), Some(int(1))];
        assert!(TestOperator::IsWithin.evaluate(Some(&lhs), &pair));

        // 越界
        let pair = [Some(int(6)), Some(int(10))];
        assert!(!TestOperator::IsWithin.evaluate(Some(&lhs), &pair));
        assert!(TestOperator::IsNotWithin.evaluate(Some(&lhs), &pair));

        // 非数组单值：结果为 false
        let pair = [Some(int(1))];
        assert!(!TestOperator::IsWithin.evaluate(Some(&lhs), &pair));

        // lhs 为 null 时按区间端点是否为 null 判定
        let pair = [Some(int(1)), None];
        assert!(TestOperator::IsWithin.evaluate(None, &pair));
        let pair = [Some(int(1)), Some(int(2))];
        assert!(!TestOperator::IsWithin.evaluate(None, &pair));
    }

    #[test]
    fn contains_is_case_insensitive() {
        let lhs = text("Hello World");
        let pair = [Some(text("hello")), None];

        assert!(TestOperator::Contains.evaluate(Some(&lhs), &pair));
        assert!(TestOperator::Contains.evaluate(Some(&lhs), &[Some(text("WORLD")), None]));
        assert!(!TestOperator::Contains.evaluate(Some(&lhs), &[Some(text("xyz")), None]));
        assert!(TestOperator::DoesNotContain.evaluate(Some(&lhs), &[Some(text("xyz")), None]));

        // 非字符串操作数不匹配
        assert!(!TestOperator::Contains.evaluate(Some(&int(1)), &pair));
    }

    #[test]
    fn contains_exactly_is_case_sensitive() {
        let lhs = text("Hello World");
        assert!(TestOperator::ContainsExactly.evaluate(Some(&lhs), &[Some(text("Hello")), None]));
        assert!(!TestOperator::ContainsExactly.evaluate(Some(&lhs), &[Some(text("hello")), None]));
    }

    #[test]
    fn and_or_do_not_panic() {
        // Java 抛 UnsupportedOperationException
        assert!(!TestOperator::And.evaluate(Some(&int(1)), &[]));
        assert!(!TestOperator::Or.evaluate(Some(&int(1)), &[]));
    }

    #[test]
    fn display_matches_java_enum_names() {
        assert_eq!(TestOperator::Equals.to_string(), "EQUALS");
        assert_eq!(
            TestOperator::IsGreaterThanOrEqualTo.to_string(),
            "IS_GREATER_THAN_OR_EQUAL_TO"
        );
    }

    #[test]
    fn operand_display_follows_java_string_value_of() {
        assert_eq!(text("x").to_string(), "x");
        assert_eq!(int(3).to_string(), "3");
        assert_eq!(
            Operand::Value(FieldValue::Double(1.0)).to_string(),
            "1.0"
        );
        assert_eq!(
            Operand::Field(FieldKey::Task(crate::task_field::TaskField::Name)).to_string(),
            "Task Name"
        );
    }
}
