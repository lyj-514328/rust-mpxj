//! Corresponds to Java: `org/mpxj/TimephasedItem.java`
//!
//! Skipped/unported Java methods and reasons:
//!
//! | Java | Rust |
//! |---|---|
//! | Implicit default constructor | [`TimephasedItem::new`] |
//! | `public LocalDateTime getStart()` / `public void setStart(LocalDateTime start)` | [`TimephasedItem::get_start`] / [`TimephasedItem::set_start`] |
//! | `public T getAmountPerHour()` / `public void setAmountPerHour(T amountPerHour)` | [`TimephasedItem::get_amount_per_hour`] / [`TimephasedItem::set_amount_per_hour`] |
//! | `public T getTotalAmount()` / `public void setTotalAmount(T totalAmount)` | [`TimephasedItem::get_total_amount`] / [`TimephasedItem::set_total_amount`] |
//! | `public LocalDateTime getFinish()` / `public void setFinish(LocalDateTime finish)` | [`TimephasedItem::get_finish`] / [`TimephasedItem::set_finish`] |
//! | `@Override public String toString()` | [`Display`] |
//! | `@Override public boolean equals(Object o)` | [`PartialEq`] (semantics below) |
//! | `@Override public int hashCode()` | **Not ported** (see below) |
//!
//! Reflection / JDBC / XML (JAXB) / GUI / `javax.*` / `Serializable`: not used by this class, nothing skipped.
//!
//! # Differences from Java
//!
//! 1. **Generic payload**: Java's `TimephasedItem<T>` uses a boxed type `T` (`Duration` / `Number`),
//! while Rust's `T` is a value type (`Duration` / `f64`) and the value is `Option<T>` (Java's nullable reference).
//! The getter requires `T: Copy` (both instantiations in this crate, `Duration` and `f64`, satisfy it).
//! 2. **Semantics of `equals`**: Java's `equals` returns `true` only when **all four fields are non-null and equal**
//! (when both sides' fields are `null` it returns `false`). Rust's [`PartialEq`] implements the same rule,
//! so this "equality" relation **is not reflexive** and cannot implement Rust's [`Eq`] (`Eq` requires `a == a`
//! to hold always); `hashCode` therefore has no counterpart either (`Hash` cannot be used with `HashMap` without `Eq`).
//! MPXJ itself never uses `TimephasedItem` as a hash key.
//! 3. **`null` in `toString()`**: Java string concatenation renders `null` as `"null"`; Rust does the same.

use std::fmt::{self, Display, Formatter};

use chrono::NaiveDateTime;

/// A quantity (work or cost) spread over a period of time.
///
/// Corresponds to Java: `public abstract class TimephasedItem<T>`
///
/// # How inheritance is reproduced
///
/// In Java, [`crate::timephased_work::TimephasedWork`] and
/// [`crate::timephased_cost::TimephasedCost`] respectively `extends TimephasedItem<Duration>` /
/// `TimephasedItem<Number>`. Rust has no inheritance, so these two types carry the inheritance relation as **type aliases**
/// (`TimephasedItem<Duration>` / `TimephasedItem<f64>`), and their respective dedicated constructors are provided by additional
/// `impl` blocks (see the file header notes of the corresponding modules).
#[derive(Debug, Clone)]
pub struct TimephasedItem<T> {
/// Start time.
    ///
    /// Corresponds to Java: `private LocalDateTime m_start;`
    m_start: Option<NaiveDateTime>,

/// Total amount.
    ///
    /// Corresponds to Java: `private T m_totalAmount;`
    m_total_amount: Option<T>,

/// End time.
    ///
    /// Corresponds to Java: `private LocalDateTime m_finish;`
    m_finish: Option<NaiveDateTime>,

/// Amount per hour.
    ///
    /// Corresponds to Java: `private T m_amountPerHour;`
    m_amount_per_hour: Option<T>,
}

impl<T> TimephasedItem<T> {
/// Constructor.
    ///
/// Corresponds to Java: implicit default constructor (Java declares no explicit constructor).
    pub fn new() -> Self {
        Self {
            m_start: None,
            m_total_amount: None,
            m_finish: None,
            m_amount_per_hour: None,
        }
    }
}

impl<T: Copy> TimephasedItem<T> {
/// Get the start time.
    ///
    /// Corresponds to Java: `public LocalDateTime getStart()`
    pub fn get_start(&self) -> Option<NaiveDateTime> {
        self.m_start
    }

/// Set the start time.
    ///
    /// Corresponds to Java: `public void setStart(LocalDateTime start)`
    pub fn set_start(&mut self, start: Option<NaiveDateTime>) {
        self.m_start = start;
    }

/// Get the amount per hour.
    ///
    /// Corresponds to Java: `public T getAmountPerHour()`
    pub fn get_amount_per_hour(&self) -> Option<T> {
        self.m_amount_per_hour
    }

/// Set the amount per hour.
    ///
    /// Corresponds to Java: `public void setAmountPerHour(T amountPerHour)`
    pub fn set_amount_per_hour(&mut self, amount_per_hour: Option<T>) {
        self.m_amount_per_hour = amount_per_hour;
    }

/// Get the total amount.
    ///
    /// Corresponds to Java: `public T getTotalAmount()`
    pub fn get_total_amount(&self) -> Option<T> {
        self.m_total_amount
    }

/// Set the total amount.
    ///
    /// Corresponds to Java: `public void setTotalAmount(T totalAmount)`
    pub fn set_total_amount(&mut self, total_amount: Option<T>) {
        self.m_total_amount = total_amount;
    }

/// Get the end time.
    ///
    /// Corresponds to Java: `public LocalDateTime getFinish()`
    pub fn get_finish(&self) -> Option<NaiveDateTime> {
        self.m_finish
    }

/// Set the end time.
    ///
    /// Corresponds to Java: `public void setFinish(LocalDateTime finish)`
    pub fn set_finish(&mut self, finish: Option<NaiveDateTime>) {
        self.m_finish = finish;
    }
}

impl<T> Default for TimephasedItem<T> {
/// Corresponds to Java: implicit default constructor, equivalent to [`TimephasedItem::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl<T: PartialEq> PartialEq for TimephasedItem<T> {
    /// Corresponds to Java: `@SuppressWarnings("unchecked") @Override public boolean equals(Object o)`
    ///
/// Java's implementation returns `true` only when all four fields are **non-null and equal** (see the file header note:
/// the relation is not reflexive, so `Eq` is not implemented).
    fn eq(&self, other: &Self) -> bool {
        self.m_start.is_some()
            && self.m_start == other.m_start
            && self.m_finish.is_some()
            && self.m_finish == other.m_finish
            && self.m_total_amount.is_some()
            && self.m_total_amount == other.m_total_amount
            && self.m_amount_per_hour.is_some()
            && self.m_amount_per_hour == other.m_amount_per_hour
    }
}

impl<T: Display> Display for TimephasedItem<T> {
    /// Corresponds to Java: `@Override public String toString()`
    ///
    /// Java: `"[TimephasedItem start=" + m_start + " totalAmount=" + m_totalAmount + " finish=" + m_finish + " amountPerHour=" + m_amountPerHour + "]"`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[TimephasedItem start={} totalAmount={} finish={} amountPerHour={}]",
            render_date(self.m_start),
            render_value(self.m_total_amount.as_ref()),
            render_date(self.m_finish),
            render_value(self.m_amount_per_hour.as_ref())
        )
    }
}

/// Render a date-time (`None` renders as `"null"` in Java string concatenation).
fn render_date(value: Option<NaiveDateTime>) -> String {
    match value {
        Some(value) => value.format("%Y-%m-%dT%H:%M").to_string(),
        None => "null".to_owned(),
    }
}

/// Render the payload (`None` renders as `"null"` in Java string concatenation).
fn render_value<T: Display>(value: Option<&T>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duration::Duration;
    use crate::time_unit::TimeUnit;

    fn date(year: i32, month: u32, day: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|value| value.and_hms_opt(0, 0, 0))
            .expect("valid date")
    }

    fn item() -> TimephasedItem<Duration> {
        let mut item = TimephasedItem::new();
        item.set_start(Some(date(2024, 5, 1)));
        item.set_finish(Some(date(2024, 5, 2)));
        item.set_total_amount(Some(Duration::get_instance(8.0, TimeUnit::Hours)));
        item.set_amount_per_hour(Some(Duration::get_instance(1.0, TimeUnit::Hours)));
        item
    }

    #[test]
    fn accessors_round_trip() {
        let mut item: TimephasedItem<Duration> = TimephasedItem::new();
        assert_eq!(item.get_start(), None);
        assert_eq!(item.get_finish(), None);
        assert_eq!(item.get_total_amount(), None);
        assert_eq!(item.get_amount_per_hour(), None);

        item.set_start(Some(date(2024, 5, 1)));
        item.set_finish(Some(date(2024, 5, 2)));
        item.set_total_amount(Some(Duration::get_instance(4.0, TimeUnit::Days)));
        item.set_amount_per_hour(Some(Duration::get_instance(2.0, TimeUnit::Hours)));

        assert_eq!(item.get_start(), Some(date(2024, 5, 1)));
        assert_eq!(item.get_finish(), Some(date(2024, 5, 2)));
        assert_eq!(
            item.get_total_amount().map(|value| value.get_duration()),
            Some(4.0)
        );
        assert_eq!(
            item.get_amount_per_hour().map(|value| value.get_units()),
            Some(TimeUnit::Hours)
        );
    }

    #[test]
    fn equality_follows_java_all_fields_non_null_rule() {
        assert_eq!(item(), item());

// Any field is null => not equal (Java's equals branch)
        let mut incomplete = item();
        incomplete.set_finish(None);
        assert_ne!(item(), incomplete);

// Two "empty" instances are also not equal (Java is not reflexive)
        let empty: TimephasedItem<Duration> = TimephasedItem::new();
        let other: TimephasedItem<Duration> = TimephasedItem::new();
        assert_ne!(empty, other);

// Different values => not equal
        let mut different = item();
        different.set_total_amount(Some(Duration::get_instance(9.0, TimeUnit::Hours)));
        assert_ne!(item(), different);
    }

    #[test]
    fn clone_copies_all_fields() {
        let source = item();
        let copy = source.clone();

        assert_eq!(copy.get_start(), source.get_start());
        assert_eq!(copy.get_finish(), source.get_finish());
        assert_eq!(
            copy.get_total_amount().map(|value| value.get_duration()),
            source.get_total_amount().map(|value| value.get_duration())
        );
    }

    #[test]
    fn display_matches_java_format() {
        assert_eq!(
            item().to_string(),
            "[TimephasedItem start=2024-05-01T00:00 totalAmount=8h finish=2024-05-02T00:00 amountPerHour=1h]"
        );

        let empty: TimephasedItem<Duration> = TimephasedItem::new();
        assert_eq!(
            empty.to_string(),
            "[TimephasedItem start=null totalAmount=null finish=null amountPerHour=null]"
        );
    }

    #[test]
    fn default_matches_new() {
        let item: TimephasedItem<Duration> = Default::default();
        assert_eq!(item.get_start(), None);
    }

    #[test]
    fn numeric_payload_is_supported() {
        let mut item: TimephasedItem<f64> = TimephasedItem::new();
        item.set_total_amount(Some(12.5));
        item.set_amount_per_hour(Some(2.5));

        assert_eq!(item.get_total_amount(), Some(12.5));
        assert_eq!(item.get_amount_per_hour(), Some(2.5));
    }
}
