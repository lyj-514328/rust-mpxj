//! Corresponds to Java: `org/mpxj/TestOperator.java`
//!
//! Skipped/unported Java methods and the reasons:
//!
//! | Java | Rust |
//! |---|---|
//! | `TYPE_VALUES = EnumHelper.createTypeArray(TestOperator.class)` | [`TestOperator::ALL`] + [`TestOperator::get_instance`] (no reflection: Java builds that array by index `getValue()`, which is contiguous `0..=13`, so a `match` expresses it equivalently) |
//! | `public static TestOperator getInstance(int type)` | [`TestOperator::get_instance`] |
//! | `public static TestOperator getInstance(Number type)` | [`TestOperator::get_instance_optional`] (`null` → `-1` → falls back to `IS_ANY_VALUE`, consistent with Java) |
//! | `public int getValue()` | [`MpxjEnum::value`] |
//! | `public abstract boolean evaluate(Object lhs, Object rhs)` | [`TestOperator::evaluate`] |
//! | `protected Object getSingleOperand(Object operand)` | [`TestOperator::get_single_operand`] |
//! | `protected boolean evaluateWithin(Object lhs, Object rhs)` | [`TestOperator::evaluate_within`] |
//! | `protected int evaluateCompareTo(Object lhs, Object rhs)` | [`TestOperator::evaluate_compare_to`] (returns `Option<Ordering>`, see below) |
//! | `protected boolean evaluateContains(Object lhs, Object rhs)` | [`TestOperator::evaluate_contains`] |
//! | `protected boolean evaluateContainsExactly(Object lhs, Object rhs)` | [`TestOperator::evaluate_contains_exactly`] |
//! | `@Override public String toString()` (default enum implementation) | [`Display`] (outputs the constant name, e.g. `"EQUALS"`) |
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: not used by this class, nothing skipped.
//!
//! # Differences from Java
//!
//! 1. **`Object` operands**: Java's `evaluate` / `setRightValue` use `Object` to carry
//! strings / numbers / dates / durations / `FieldType` / `GenericCriteriaPrompt`.
//! The Rust side introduces [`Operand`] for this set of values (see its documentation); the signature of `evaluate` is
//!    `evaluate(Option<&Operand>, &[Option<Operand>])`:
//! * `lhs` uses `Option` to express Java's `null`;
//! * `rhs` uses a slice to express Java's "single value **or** `Object[]`" --
//! on the `GenericCriteria` side it is always an array of length 2; a slice of length 1 corresponds to Java's "non-array single value"
//! (`IS_WITHIN` / `IS_NOT_WITHIN` yield `false` for a non-array, consistent with Java).
//! 2. **Failure semantics of `compareTo`**: Java's `((Comparable) lhs).compareTo(rhs)` throws
//! `ClassCastException` for incompatible types; the Rust side's [`TestOperator::evaluate_compare_to`] returns `Option<Ordering>`,
//! returning `None` when they are not comparable (the caller treats it as `false`, no panic).
//! 3. **`AND` / `OR`**: Java's `evaluate` directly throws `UnsupportedOperationException`
//! (these two constants are used only as logical combinators in `GenericCriteria`); Rust library code does not panic,
//! so it returns `false` (the actual value path never reaches it; `GenericCriteria::evaluate` branches earlier).
//! 4. **Case-insensitive comparison**: Java uses `toUpperCase()` (Unicode); the Rust side likewise uses
//! [`str::to_uppercase`] (Unicode), with identical behavior.
//! 5. **Binary operands**: Java's `byte[]` is stringified by object address by default; Rust uses `Debug` to output the bytes;
//! filter expressions do not use binary values from OLE custom properties.

use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};

use crate::field_key::FieldKey;
use crate::field_value::FieldValue;
use crate::generic_criteria_prompt::GenericCriteriaPrompt;
use crate::mpxj_enum::MpxjEnum;

/// Corresponds to the `Object` passed as an operand in Java.
///
/// `TestOperator.evaluate(Object lhs, Object rhs)` and `GenericCriteria`'s
/// `Object[] m_definedRightValues` / `Map<GenericCriteriaPrompt, Object>` all use `Object`
/// to carry values; the Rust side expresses them uniformly with this enum:
///
/// | Java value | Rust |
/// |---|---|
/// | `FieldType` (symbol value) | [`Operand::Field`] |
/// | `GenericCriteriaPrompt` (prompt value) | [`Operand::Prompt`] |
/// | String / number / boolean / date / duration / rate | [`Operand::Value`] (carries a [`FieldValue`]) |
/// | `null` | `Option::None` |
///
/// Symbol values and prompt values are resolved into concrete values before `GenericCriteria` evaluation (see
/// [`crate::generic_criteria::GenericCriteria::evaluate`]); if they are passed directly into
/// `evaluate` (the Java path where "only a prompt is set, no symbol value"), they are not equal to any value,
/// consistent with the result of `String.equals(GenericCriteriaPrompt)` in Java.
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
/// Corresponds to Java: `FieldType` used as an operand.
    Field(FieldKey),
/// Corresponds to Java: `GenericCriteriaPrompt` used as an operand.
    Prompt(GenericCriteriaPrompt),
/// Corresponds to Java: a scalar value used as an operand (`String` / `Number` / `LocalDateTime` / `Duration` ...).
    Value(FieldValue),
}

impl Operand {
/// Returns the concrete value (`None` when it is not [`Operand::Value`]).
    pub fn as_value(&self) -> Option<&FieldValue> {
        match self {
            Operand::Value(value) => Some(value),
            _ => None,
        }
    }

/// Returns the string value (Java: `lhs instanceof String`).
    pub fn as_string(&self) -> Option<&str> {
        self.as_value().and_then(FieldValue::as_string)
    }

/// Replicates the comparison semantics of Java's `Comparable.compareTo`, returning `None` when they are not comparable
/// (Java throws `ClassCastException`).
    ///
/// Only operands that are likewise concrete values can be compared; numeric types are not compared across types
/// (Java's `Integer.compareTo(Double)` throws `ClassCastException`).
    pub fn compare_to(&self, other: &Operand) -> Option<Ordering> {
        match (self, other) {
            (Operand::Value(lhs), Operand::Value(rhs)) => compare_field_values(lhs, rhs),
            _ => None,
        }
    }
}

/// Compares two field values, returning `None` when they are not comparable.
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
// Java's `Rate` does not implement `Comparable`; compare by amount here (ignoring units) for use in filtering.
        (FieldValue::Rate(lhs), FieldValue::Rate(rhs)) => {
            lhs.get_amount().partial_cmp(&rhs.get_amount())
        }
        _ => None,
    }
}

impl From<FieldValue> for Operand {
/// Constructs an operand from a concrete value (Rust-specific convenience entry point).
    fn from(value: FieldValue) -> Self {
        Operand::Value(value)
    }
}

impl Display for Operand {
/// Replicates Java's value rendering during string concatenation (`String.valueOf`); `Operand` itself is not a
/// Java type; this is a Rust-side formatting helper.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Field(field) => write!(f, "{field}"),
            Operand::Prompt(prompt) => write!(f, "{prompt}"),
            Operand::Value(value) => f.write_str(&render_field_value(value)),
        }
    }
}

/// Renders a field value in the style of Java's `toString()`.
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
// Types for which Java does not override `toString()` (`WorkContour` / enums) and collection / entity-reference values:
// this repository's convention is to render them with `Debug` (see "classes for which Java does not override toString use Debug" in each file header),
// they do not appear as operands in filter expressions.
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

/// Replicates the integer suffix of Java's `Double.toString` (`1.0` rather than `1`).
fn render_double(value: f64) -> String {
    let text = value.to_string();
    if text.contains('.') || text.contains('e') || text.contains('E') {
        text
    } else {
        format!("{text}.0")
    }
}

/// Operators used to compare two operands.
///
/// Corresponds to Java: `public enum TestOperator implements MpxjEnum`
///
/// The constant declaration order and `getValue()` values match Java (contiguous `0..=13`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestOperator {
    /// Corresponds to Java: `IS_ANY_VALUE(0)`
    IsAnyValue = 0,
    /// Corresponds to Java: `IS_WITHIN(1)`
    IsWithin = 1,
    /// Corresponds to Java: `IS_GREATER_THAN(2)`
    IsGreaterThan = 2,
    /// Corresponds to Java: `IS_LESS_THAN(3)`
    IsLessThan = 3,
    /// Corresponds to Java: `IS_GREATER_THAN_OR_EQUAL_TO(4)`
    IsGreaterThanOrEqualTo = 4,
    /// Corresponds to Java: `IS_LESS_THAN_OR_EQUAL_TO(5)`
    IsLessThanOrEqualTo = 5,
    /// Corresponds to Java: `EQUALS(6)`
    Equals = 6,
    /// Corresponds to Java: `DOES_NOT_EQUAL(7)`
    DoesNotEqual = 7,
    /// Corresponds to Java: `CONTAINS(8)`
    Contains = 8,
    /// Corresponds to Java: `IS_NOT_WITHIN(9)`
    IsNotWithin = 9,
    /// Corresponds to Java: `DOES_NOT_CONTAIN(10)`
    DoesNotContain = 10,
    /// Corresponds to Java: `CONTAINS_EXACTLY(11)`
    ContainsExactly = 11,
/// Corresponds to Java: `AND(12)` (an MPXJ extension, not defined by MS Project)
    And = 12,
/// Corresponds to Java: `OR(13)` (an MPXJ extension, not defined by MS Project)
    Or = 13,
}

impl TestOperator {
/// All values, in the same order as the Java declaration order.
    ///
/// Corresponds to Java: `values()` (`EnumHelper.createTypeArray` relies on it to build the table).
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

/// The length of the `TYPE_VALUES` array on the Java side (`EnumHelper.createTypeArray` builds the table with the maximum `getValue() + 1`).
    const TYPE_VALUES_LEN: i32 = 14;

/// Returns the enum instance for an int value, falling back to `IS_ANY_VALUE` if out of bounds.
    ///
    /// Corresponds to Java: `public static TestOperator getInstance(int type)`
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
// On the Java side every index of that array has a corresponding constant; this arm exists only for exhaustive matching.
            _ => TestOperator::IsAnyValue,
        }
    }

/// Returns the enum instance for a nullable int value (`null` is equivalent to `-1`, falling back to `IS_ANY_VALUE`).
    ///
    /// Corresponds to Java: `public static TestOperator getInstance(Number type)`
    pub fn get_instance_optional(value: Option<i32>) -> TestOperator {
        TestOperator::get_instance(value.unwrap_or(-1))
    }

/// Compares operands using this operator.
    ///
    /// Corresponds to Java: `public abstract boolean evaluate(Object lhs, Object rhs)`
    ///
/// `rhs` is a slice: length 1 represents Java's non-array single value, length ≥ 2 represents Java's `Object[]`
/// (`GenericCriteria` always passes an array of length 2).
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
// Java throws UnsupportedOperationException; Rust does not panic and returns false.
            TestOperator::And | TestOperator::Or => false,
        }
    }

/// Returns the single operand in the RHS (Java's `Object[]` takes index 0).
    ///
    /// Corresponds to Java: `protected Object getSingleOperand(Object operand)`
    pub fn get_single_operand<'a>(
        &self,
        operand: &'a [Option<Operand>],
    ) -> Option<&'a Operand> {
// Java: if operand is an Object[] take list[0], otherwise take operand itself;
// since a non-array single value is expressed as a length-1 slice in Rust, both cases are equivalent to taking the first element.
        operand.first().and_then(Option::as_ref)
    }

/// Determines whether the LHS falls within the range given by the RHS.
    ///
    /// Corresponds to Java: `protected boolean evaluateWithin(Object lhs, Object rhs)`
    ///
/// Project also tries the flipped range, so both directions are accepted; when `rhs` has length less than 2
/// (in Java, "not an `Object[]`") the result is `false`.
    pub fn evaluate_within(&self, lhs: Option<&Operand>, rhs: &[Option<Operand>]) -> bool {
        if rhs.len() < 2 {
            return false;
        }

        let first = rhs[0].as_ref();
        let second = rhs[1].as_ref();

        let Some(lhs) = lhs else {
// Project also honors null equality (e.g. NA dates)
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

/// Replicates the result of Java's `compareTo` (`lhs` being `null` is treated as greater, consistent with Java's
/// `result = 1`).
    ///
    /// Corresponds to Java: `protected int evaluateCompareTo(Object lhs, Object rhs)`
    ///
/// The Rust side returns `Option<Ordering>`: `None` when they are not comparable (Java throws
    /// `ClassCastException`).
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

/// Determines whether the RHS (string) is contained in the LHS (string), case-insensitively.
    ///
    /// Corresponds to Java: `protected boolean evaluateContains(Object lhs, Object rhs)`
    pub fn evaluate_contains(&self, lhs: Option<&Operand>, rhs: &[Option<Operand>]) -> bool {
        self.contains_with(lhs, rhs, true)
    }

/// Determines whether the RHS (string) is contained in the LHS (string), case-sensitively.
    ///
    /// Corresponds to Java: `protected boolean evaluateContainsExactly(Object lhs, Object rhs)`
    pub fn evaluate_contains_exactly(
        &self,
        lhs: Option<&Operand>,
        rhs: &[Option<Operand>],
    ) -> bool {
        self.contains_with(lhs, rhs, false)
    }

/// Common implementation of `evaluateContains` / `evaluateContainsExactly`.
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

/// The `Option` version of `evaluate_compare_to` (for internal use by `evaluate`).
    fn compare_result(&self, lhs: Option<&Operand>, rhs: &[Option<Operand>]) -> Option<Ordering> {
        self.evaluate_compare_to(lhs, rhs)
    }
}

impl MpxjEnum for TestOperator {
/// Returns the int representation of this operator.
    ///
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for TestOperator {
/// The Java enum does not override `toString()`; outputs the constant name (e.g. `"EQUALS"`).
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

/// Convenience constructor: string operand.
    fn text(value: &str) -> Operand {
        Operand::Value(FieldValue::String(value.to_owned()))
    }

/// Convenience constructor: integer operand.
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

// When lhs is null Java's evaluateCompareTo returns 1 (treated as greater)
        assert!(TestOperator::IsGreaterThan.evaluate(None, &pair));
    }

    #[test]
    fn compare_operators_return_false_for_incompatible_types() {
// Java throws ClassCastException; Rust treats it as false
        let pair = [Some(text("abc")), None];
        assert!(!TestOperator::IsGreaterThan.evaluate(Some(&int(1)), &pair));

// Numeric values are not compared across types (Java's Integer.compareTo(Double) throws ClassCastException)
        let pair = [Some(Operand::Value(FieldValue::Double(1.0))), None];
        assert!(!TestOperator::IsGreaterThan.evaluate(Some(&int(2)), &pair));
    }

    #[test]
    fn within_accepts_both_directions_and_requires_pair() {
        let lhs = int(5);

        let pair = [Some(int(1)), Some(int(10))];
        assert!(TestOperator::IsWithin.evaluate(Some(&lhs), &pair));

// Project also tries the flipped range
        let pair = [Some(int(10)), Some(int(1))];
        assert!(TestOperator::IsWithin.evaluate(Some(&lhs), &pair));

// Out of bounds
        let pair = [Some(int(6)), Some(int(10))];
        assert!(!TestOperator::IsWithin.evaluate(Some(&lhs), &pair));
        assert!(TestOperator::IsNotWithin.evaluate(Some(&lhs), &pair));

// Non-array single value: the result is false
        let pair = [Some(int(1))];
        assert!(!TestOperator::IsWithin.evaluate(Some(&lhs), &pair));

// When lhs is null, decide by whether the range endpoints are null
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

// Non-string operands do not match
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
// Java throws UnsupportedOperationException
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
