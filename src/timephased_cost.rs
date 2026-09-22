//! Corresponds to Java: `org/mpxj/TimephasedCost.java`
//!
//! Skipped / unported Java methods and the reasons:
//!
//! | Java | Rust |
//! |---|---|
//! | (the Java side has no members of its own, only the comment `// Empty block`) | `TimephasedCost::new()` is provided by [`TimephasedItem::new`] via the type alias, and all other methods are inherited from [`TimephasedItem`] |
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: this class uses none of these, so nothing is skipped.
//!
//! # Reproducing the inheritance relationship: type alias
//!
//! Java's `public final class TimephasedCost extends TimephasedItem<Number>` is represented on the Rust side
//! by a **type alias**: `pub type TimephasedCost = TimephasedItem<f64>;`.
//! Since this class has no members or methods of its own on the Java side, the alias expresses it completely.
//!
//! # Differences from Java
//!
//! - **`Number` -> `f64`**: Java's payload type is `Number`, and the Rust side uniformly uses `f64`
//!   (consistent with how other modules in this crate handle `Number`), with the value being `Option<f64>`.
//! - **`equals` / `hashCode`**: inherited from [`TimephasedItem`]; the Rust-side trade-offs are described in that type's file header
//!   (implements `PartialEq` per Java's "all four fields non-null and equal" semantics, and does not implement `Eq` / `Hash`).

use crate::timephased_item::TimephasedItem;

/// A cost spread over a period of time.
///
/// Corresponds to Java: `public final class TimephasedCost extends TimephasedItem<Number>`
///
/// (On the Rust side this is a type alias for `TimephasedItem<f64>`; see the file header.)
pub type TimephasedCost = TimephasedItem<f64>;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    fn date(year: i32, month: u32, day: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|value| value.and_hms_opt(0, 0, 0))
            .expect("valid date")
    }

    #[test]
    fn new_creates_empty_item() {
        let item = TimephasedCost::new();
        assert_eq!(item.get_start(), None);
        assert_eq!(item.get_finish(), None);
        assert_eq!(item.get_total_amount(), None);
        assert_eq!(item.get_amount_per_hour(), None);
    }

    #[test]
    fn accessors_round_trip() {
        let mut item = TimephasedCost::new();
        item.set_start(Some(date(2024, 5, 1)));
        item.set_finish(Some(date(2024, 5, 2)));
        item.set_total_amount(Some(120.0));
        item.set_amount_per_hour(Some(15.0));

        assert_eq!(item.get_start(), Some(date(2024, 5, 1)));
        assert_eq!(item.get_finish(), Some(date(2024, 5, 2)));
        assert_eq!(item.get_total_amount(), Some(120.0));
        assert_eq!(item.get_amount_per_hour(), Some(15.0));
    }

    #[test]
    fn clone_copies_all_fields() {
        let mut source = TimephasedCost::new();
        source.set_total_amount(Some(50.0));

        let copy = source.clone();
        assert_eq!(copy.get_total_amount(), Some(50.0));
    }

    #[test]
    fn display_renders_amounts() {
        let mut item = TimephasedCost::new();
        item.set_total_amount(Some(120.0));

        assert_eq!(
            item.to_string(),
            "[TimephasedItem start=null totalAmount=120 finish=null amountPerHour=null]"
        );
    }

    #[test]
    fn equality_follows_java_all_fields_non_null_rule() {
        let mut first = TimephasedCost::new();
        first.set_start(Some(date(2024, 5, 1)));
        first.set_finish(Some(date(2024, 5, 2)));
        first.set_total_amount(Some(120.0));
        first.set_amount_per_hour(Some(15.0));

        let mut second = TimephasedCost::new();
        second.set_start(Some(date(2024, 5, 1)));
        second.set_finish(Some(date(2024, 5, 2)));
        second.set_total_amount(Some(120.0));
        second.set_amount_per_hour(Some(15.0));

        assert_eq!(first, second);

        // Any field being null => not equal
        second.set_amount_per_hour(None);
        assert_ne!(first, second);
    }
}
