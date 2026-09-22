//! Corresponds to Java: `org/mpxj/TimephasedWork.java`
//!
//! Skipped/unported Java methods and the reasons:
//!
//! | Java | Rust |
//! |---|---|
//! | `public TimephasedWork()` | `TimephasedWork::new()` (i.e. [`TimephasedItem::new`], see below) |
//! | `public TimephasedWork(TimephasedWork sourceItem)` | handled by `Clone` (Java's copy constructor is equivalent to a field-by-field copy, and the Rust `TimephasedItem` already derives [`Clone`]) |
//! | `public TimephasedWork(TimephasedWork sourceItem, double factor)` | [`TimephasedItem::from_work`] |
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: this class does not use them, so nothing is skipped.
//!
//! # How the inheritance relationship is restored: type alias
//!
//! Java's `public final class TimephasedWork extends TimephasedItem<Duration>` is represented on the Rust side
//! by a **type alias**: `pub type TimephasedWork = TimephasedItem<Duration>;`.
//! The inherited methods (`getStart` / `setStart` / `getTotalAmount` / ...) are provided directly by
//! [`TimephasedItem`], and the constructors specific to this class are implemented in an extra `impl TimephasedItem<Duration>` block
//! (Rust allows writing an inherent impl for a generic instance within the same crate).
//!
//! # Differences from Java
//!
//! - **The type of `T`**: Java uses `TimephasedItem<Duration>` (boxed type), Rust uses
//! `TimephasedItem<crate::duration::Duration>` (value type), with the value being `Option<Duration>`.
//! - **Null handling in `from_work`**: the Java implementation directly dereferences `getTotalAmount()` /
//! `getAmountPerHour()` and throws `NullPointerException` when they are null; the Rust side keeps them as
//! `None` when null (without panicking), and scales proportionally by `factor` while preserving the original unit when non-null.
//! - **`equals` / `hashCode`**: inherited from [`TimephasedItem`]; see that type's file header for the Rust-side trade-offs
//! (`PartialEq` is implemented following Java's semantics of "all four fields non-null and equal"; `Eq` / `Hash` are not implemented).

use crate::duration::Duration;
use crate::timephased_item::TimephasedItem;

/// Work spread over a period of time.
///
/// Corresponds to Java: `public final class TimephasedWork extends TimephasedItem<Duration>`
///
/// (On the Rust side this is a type alias for `TimephasedItem<Duration>`, see the file header.)
pub type TimephasedWork = TimephasedItem<Duration>;

impl TimephasedItem<Duration> {
/// Copy a work item proportionally (scaling the total amount and the amount per hour, copying the start and end times as-is).
    ///
    /// Corresponds to Java: `public TimephasedWork(TimephasedWork sourceItem, double factor)`
    ///
    /// Java:
    /// ```text
    /// setStart(sourceItem.getStart());
    /// setFinish(sourceItem.getFinish());
    /// setTotalAmount(Duration.getInstance(sourceItem.getTotalAmount().getDuration() * factor, sourceItem.getTotalAmount().getUnits()));
    /// setAmountPerHour(Duration.getInstance(sourceItem.getAmountPerHour().getDuration() * factor, sourceItem.getAmountPerHour().getUnits()));
    /// ```
    pub fn from_work(source_item: &TimephasedWork, factor: f64) -> TimephasedWork {
        let mut result = TimephasedWork::new();
        result.set_start(source_item.get_start());
        result.set_finish(source_item.get_finish());
        result.set_total_amount(source_item.get_total_amount().map(|value| {
            Duration::get_instance(value.get_duration() * factor, value.get_units())
        }));
        result.set_amount_per_hour(source_item.get_amount_per_hour().map(|value| {
            Duration::get_instance(value.get_duration() * factor, value.get_units())
        }));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_unit::TimeUnit;
    use chrono::NaiveDateTime;

    fn date(year: i32, month: u32, day: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|value| value.and_hms_opt(0, 0, 0))
            .expect("valid date")
    }

    fn work() -> TimephasedWork {
        let mut item = TimephasedWork::new();
        item.set_start(Some(date(2024, 5, 1)));
        item.set_finish(Some(date(2024, 5, 2)));
        item.set_total_amount(Some(Duration::get_instance(8.0, TimeUnit::Hours)));
        item.set_amount_per_hour(Some(Duration::get_instance(1.0, TimeUnit::Hours)));
        item
    }

    #[test]
    fn new_creates_empty_item() {
        let item = TimephasedWork::new();
        assert_eq!(item.get_start(), None);
        assert_eq!(item.get_finish(), None);
        assert_eq!(item.get_total_amount(), None);
        assert_eq!(item.get_amount_per_hour(), None);
    }

    #[test]
    fn clone_copies_source_item() {
        // Java: public TimephasedWork(TimephasedWork sourceItem)
        let copy = work().clone();

        assert_eq!(copy.get_start(), Some(date(2024, 5, 1)));
        assert_eq!(
            copy.get_total_amount().map(|value| value.get_duration()),
            Some(8.0)
        );
        assert_eq!(
            copy.get_amount_per_hour().map(|value| value.get_units()),
            Some(TimeUnit::Hours)
        );
    }

    #[test]
    fn from_work_scales_amounts_and_keeps_units() {
        let scaled = TimephasedWork::from_work(&work(), 0.5);

        assert_eq!(scaled.get_start(), Some(date(2024, 5, 1)));
        assert_eq!(scaled.get_finish(), Some(date(2024, 5, 2)));
        assert_eq!(
            scaled.get_total_amount().map(|value| value.get_duration()),
            Some(4.0)
        );
        assert_eq!(
            scaled
                .get_amount_per_hour()
                .map(|value| (value.get_duration(), value.get_units())),
            Some((0.5, TimeUnit::Hours))
        );
    }

    #[test]
    fn from_work_tolerates_missing_amounts() {
        let mut source = TimephasedWork::new();
        source.set_start(Some(date(2024, 5, 1)));

        let scaled = TimephasedWork::from_work(&source, 2.0);

        assert_eq!(scaled.get_total_amount(), None);
        assert_eq!(scaled.get_amount_per_hour(), None);
        assert_eq!(scaled.get_start(), Some(date(2024, 5, 1)));
    }
}
