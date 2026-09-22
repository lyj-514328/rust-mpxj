//! Corresponds to Java: `org/mpxj/ShiftPeriod.java`
//!
//! Skipped Java methods: none.
//!
//! # Differences from Java
//!
//! - **Unique ID allocation**: Java's constructor
//!   `UniqueIdObjectSequenceProvider#getUniqueIdObjectSequence(ShiftPeriod.class).syncOrGetNext(...)`
//! allocates/synchronizes the unique ID via `ObjectSequence` and `UniqueIdObjectSequenceProvider`, both of which are already in place, but this repo's
//! [`Builder`] is not yet wired up and still uses the value passed in by the caller through [`Builder::unique_id`];
//! `ShiftPeriod` also does not implement `ProjectEntityWithUniqueID` (that trait is already in place and is a pending wiring item,
//! see `TECH_DEBT.md` section six), but it still provides `get_unique_id()`.
//! - **Parent shift reference**: Java uses the strong reference `Shift m_parentShift` to point back to the parent shift (and also adds itself
//! to the parent shift's `TreeSet` during construction). The Rust side uses the [`Weak`] reference to express the back-reference (avoiding a
//! circular reference with `Shift.periods`); [`ShiftPeriod::get_parent_shift`]
//! returns `None` when the parent shift has been dropped.
//! - **Constructor**: Java's constructor is private and can only be created through [`Builder`]; the Rust side likewise
//! provides no public constructor, and `Builder::build` registers the newly created period into the parent shift per Java semantics.
//! - `equals` / `hashCode`: Java does not override them (reference equality), but `TreeSet` uses `compareTo`
//! to detect duplicates. The Rust side keeps `PartialEq` / `Hash` consistent with `Ord` (by start time),
//! so that `BTreeSet` matches Java's `TreeSet` deduplication behavior exactly.
//! - When `start` is `null`, Java throws an NPE from `getDuration()` / `compareTo()`,
//! while the Rust side degrades gracefully: duration is 0 hours and `null` is treated as larger for sorting (consistent with
//! the null semantics of `LocalTimeHelper.compare`).

use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::rc::{Rc, Weak};

use chrono::NaiveTime;

use crate::duration::Duration;
use crate::shift::Shift;
use crate::time_unit::TimeUnit;

/// A period within a resource shift.
///
/// Corresponds to Java: `public final class ShiftPeriod implements ProjectEntityWithUniqueID, Comparable<ShiftPeriod>`
#[derive(Debug, Clone)]
pub struct ShiftPeriod {
    /// Corresponds to Java field `private final Integer m_uniqueID`
    unique_id: Option<i32>,
/// Corresponds to Java field `private final Shift m_parentShift` (a weak reference on the Rust side)
    parent_shift: Weak<Shift>,
    /// Corresponds to Java field `private final LocalTime m_start`
    start: Option<NaiveTime>,
}

impl ShiftPeriod {
/// Get the parent shift (returns `None` when the parent shift has been dropped).
    ///
    /// Corresponds to Java: `public Shift getParentShift()`
    pub fn get_parent_shift(&self) -> Option<Rc<Shift>> {
        self.parent_shift.upgrade()
    }

/// Get the period's unique ID.
    ///
    /// Corresponds to Java: `@Override public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.unique_id
    }

/// Get the period's start time.
    ///
    /// Corresponds to Java: `public LocalTime getStart()`
    pub fn get_start(&self) -> Option<NaiveTime> {
        self.start
    }

/// Compute this period's duration.
    ///
    /// Corresponds to Java: `public Duration getDuration()`
    ///
/// Duration = the next period's start time - this period's start time; when there is no later period, midnight is the boundary
/// (`24 + difference` when crossing midnight), consistent with Java's `LocalTime.until(..., HOURS)`.
    pub fn get_duration(&self) -> Duration {
        let start = match self.start {
            Some(start) => start,
// Java: throws an NPE when start is null; the Rust side degrades gracefully to 0 hours
            None => return Duration::get_instance(0.0, TimeUnit::Hours),
        };

        // Java: m_parentShift.getPeriods().stream().filter(p -> p.m_start.isAfter(m_start))
        //       .findFirst().map(p -> p.m_start).orElse(LocalTime.MIDNIGHT)
// The collection is already ordered by start time; taking the smallest value "greater than this period's start time" is equivalent to findFirst.
        let next_start = self
            .parent_shift
            .upgrade()
            .map(|shift| shift.get_periods())
            .unwrap_or_default()
            .iter()
            .filter_map(|period| period.start)
            .filter(|period_start| *period_start > start)
            .min()
            .unwrap_or(NaiveTime::MIN);

        let mut hours = next_start.signed_duration_since(start).num_hours();
        if hours < 0 {
            hours += 24;
        }

        Duration::get_instance(hours as f64, TimeUnit::Hours)
    }
}

/// Corresponds to Java: `LocalTime.compareTo`, where `null` is treated as "greater".
fn compare_optional_time(lhs: Option<NaiveTime>, rhs: Option<NaiveTime>) -> Ordering {
    match (lhs, rhs) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(lhs), Some(rhs)) => lhs.cmp(&rhs),
    }
}

impl Ord for ShiftPeriod {
/// Corresponds to Java: `@Override public int compareTo(ShiftPeriod o)`: compare by start time.
    fn cmp(&self, other: &Self) -> Ordering {
        compare_optional_time(self.start, other.start)
    }
}

impl PartialOrd for ShiftPeriod {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ShiftPeriod {
/// Note: Java does not override `equals`, but `TreeSet` deduplicates with `compareTo`; the Rust side keeps it consistent with `Ord`
/// so that `BTreeSet`'s deduplication semantics match Java's.
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for ShiftPeriod {}

impl Hash for ShiftPeriod {
/// Keep consistent with [`ShiftPeriod::eq`]: use only the start time.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.start.hash(state);
    }
}

/// Builder for shift periods.
///
/// Corresponds to Java: `public static class ShiftPeriod.Builder`
///
/// Java's `Builder` methods modify in place and return `this`; the Rust side uses the same
/// "consume self and return `Self`" style as `crate::unit_of_measure::Builder`.
#[derive(Debug)]
pub struct Builder {
    /// Corresponds to Java field `private final Shift m_shift`
    shift: Rc<Shift>,
    /// Corresponds to Java field `private Integer m_uniqueID`
    unique_id: Option<i32>,
    /// Corresponds to Java field `private LocalTime m_start`
    start: Option<NaiveTime>,
}

impl Builder {
/// Constructor.
    ///
    /// Corresponds to Java: `public Builder(UniqueIdObjectSequenceProvider sequenceProvider, Shift shift)`
    ///
/// Java's sequence provider is used to allocate a unique ID via `ObjectSequence`; this repo has not yet wired it up
/// (`UniqueIdObjectSequenceProvider` is already in place), so this parameter is not accepted (see the file header).
    pub fn new(shift: &Rc<Shift>) -> Self {
        Self {
            shift: Rc::clone(shift),
            unique_id: None,
            start: None,
        }
    }

/// Set the unique ID.
    ///
    /// Corresponds to Java: `public Builder uniqueID(Integer value)`
    pub fn unique_id(mut self, value: Option<i32>) -> Self {
        self.unique_id = value;
        self
    }

/// Set the start time.
    ///
    /// Corresponds to Java: `public Builder start(LocalTime value)`
    pub fn start(mut self, value: Option<NaiveTime>) -> Self {
        self.start = value;
        self
    }

/// Set the start time as an hour interval (minutes and seconds are 0).
    ///
    /// Corresponds to Java: `public Builder start(Integer value)`(`LocalTime.of(value, 0)`)
    ///
/// Rust cannot overload, so the `_from_hour` suffix is added. Java throws
/// `DateTimeException` for an out-of-range hour; the Rust side degrades gracefully to no start time (`None`).
    pub fn start_from_hour(mut self, value: i32) -> Self {
        self.start = NaiveTime::from_hms_opt(value as u32, 0, 0);
        self
    }

/// Construct a [`ShiftPeriod`] instance and register the period into the parent shift.
    ///
    /// Corresponds to Java: `public ShiftPeriod build()`
    ///
/// The object Java returns is the same one registered into the parent shift's collection (reference semantics);
/// the Rust side's `ShiftPeriod` is a value type and returns a clone; the fields and environment are equivalent.
    pub fn build(self) -> ShiftPeriod {
        let period = ShiftPeriod {
            unique_id: self.unique_id,
            parent_shift: Rc::downgrade(&self.shift),
            start: self.start,
        };

// Java: m_parentShift.getPeriods().add(this) in the ShiftPeriod constructor
        self.shift.add_period(period.clone());

        period
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shift::Builder as ShiftBuilder;

    fn time(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).expect("valid time")
    }

    fn start(hour: u32) -> Option<NaiveTime> {
        NaiveTime::from_hms_opt(hour, 0, 0)
    }

    fn hours(value: f64) -> Duration {
        Duration::get_instance(value, TimeUnit::Hours)
    }

    #[test]
    fn period_is_registered_in_parent_shift() {
        let shift = ShiftBuilder::new().name(Some("Morning shift".to_string())).build();
        let period = Builder::new(&shift).unique_id(Some(1)).start(start(8)).build();

        assert_eq!(period.get_unique_id(), Some(1));
        assert_eq!(period.get_start(), start(8));
        assert_eq!(shift.get_periods().len(), 1);
        assert_eq!(
            period.get_parent_shift().map(|shift| shift.get_name().map(str::to_string)),
            Some(Some("Morning shift".to_string()))
        );
    }

    #[test]
    fn duration_is_time_to_next_period() {
        let shift = ShiftBuilder::new().build();
        let morning = Builder::new(&shift).start(start(8)).build();
        let evening = Builder::new(&shift).start(start(20)).build();

        assert_eq!(morning.get_duration(), hours(12.0));
        assert_eq!(evening.get_duration(), hours(4.0));
    }

    #[test]
    fn duration_of_single_period_wraps_to_midnight() {
        let shift = ShiftBuilder::new().build();
        let period = Builder::new(&shift).start(start(8)).build();
        assert_eq!(period.get_duration(), hours(16.0));
    }

    #[test]
    fn duration_of_midnight_period_without_later_period_is_zero() {
        let shift = ShiftBuilder::new().build();
        let midnight = Builder::new(&shift).start(start(0)).build();
        // Java: m_start.until(LocalTime.MIDNIGHT, HOURS) == 0
        assert_eq!(midnight.get_duration(), hours(0.0));

        let next = Builder::new(&shift).start(start(0)).build();
// Same start time as the previous period, deduplicated by BTreeSet (matching Java TreeSet behavior)
        assert_eq!(next.get_duration(), hours(0.0));
        assert_eq!(shift.get_periods().len(), 1);
    }

    #[test]
    fn duration_uses_start_based_ordering() {
        let shift = ShiftBuilder::new().build();
        Builder::new(&shift).start(start(6)).build();
        Builder::new(&shift).start(start(14)).build();
        let night = Builder::new(&shift).start(start(22)).build();

// No later period after 22:00; crossing midnight to 00:00 the next day is 2 hours in total
        assert_eq!(night.get_duration(), hours(2.0));
    }

    #[test]
    fn null_start_degrades_gracefully() {
        let shift = ShiftBuilder::new().build();
        let period = Builder::new(&shift).build();

        assert_eq!(period.get_start(), None);
// Java: throws an NPE; the Rust side degrades gracefully to 0 hours
        assert_eq!(period.get_duration(), hours(0.0));
    }

    #[test]
    fn start_from_hour_rejects_invalid_hour() {
        let shift = ShiftBuilder::new().build();
        let valid = Builder::new(&shift).start_from_hour(9).build();
        assert_eq!(valid.get_start(), Some(time(9, 0)));

        let invalid = Builder::new(&shift).start_from_hour(25).build();
        assert_eq!(invalid.get_start(), None);
    }

    #[test]
    fn ordering_follows_start_time() {
        let shift = ShiftBuilder::new().build();
        let morning = Builder::new(&shift).start(start(8)).build();
        let afternoon = Builder::new(&shift).start(start(13)).build();
        let same_as_morning = Builder::new(&shift).start(start(8)).build();

        assert!(morning < afternoon);
        assert_eq!(morning.cmp(&same_as_morning), Ordering::Equal);
        assert_eq!(morning, same_as_morning);

        let periods = shift.get_periods();
        assert_eq!(periods.len(), 2);
        assert_eq!(periods[0].get_start(), start(8));
        assert_eq!(periods[1].get_start(), start(13));
    }
}
