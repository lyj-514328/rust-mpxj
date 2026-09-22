//! Corresponds to Java: `org/mpxj/Shift.java`
//!
//! Skipped Java methods: none.
//!
//! # Differences from Java
//!
//! - **Unique ID allocation**: Java's constructor uses
//!   `UniqueIdObjectSequenceProvider#getUniqueIdObjectSequence(Shift.class).syncOrGetNext(...)`
//! to allocate/synchronize unique IDs. `ObjectSequence` / `UniqueIdObjectSequenceProvider` /
//! `ProjectEntityWithUniqueID` have all been ported (see [`crate::common::object_sequence`] /
//!   [`crate::unique_id_object_sequence_provider`] / [`crate::project_entity_with_unique_id`];
//! [`crate::step::Step`] already uses the same mechanism), but this type is **not yet wired up**: [`Builder`] still does not accept a sequence
//! provider and directly uses the value passed by the caller via [`Builder::unique_id`]; `Shift` also does not
//! `impl ProjectEntityWithUniqueID`, only providing a same-named `get_unique_id()`.
//! - **Object graph**: In Java, `ShiftPeriod` holds a reference to its parent `Shift`, and `Shift` holds all shift
//! periods in a `TreeSet<ShiftPeriod>`. Rust uses `Rc<Shift>` + `Weak<Shift>`
//! to express this bidirectional relationship: `Builder::build()` returns `Rc<Shift>`, and shift periods point back to
//! the parent shift via a weak reference (avoiding a reference cycle that would leak memory). `Shift.periods` is therefore wrapped in
//! a [`RefCell`] so that `ShiftPeriod` can register itself with the parent shift when constructed.
//! - **Collection semantics**: Java's `Set<ShiftPeriod> m_periods = new TreeSet<>()` sorts by
//! `compareTo` (start time) and rejects periods whose `compareTo` is 0 as duplicates;
//! Rust uses `BTreeSet<ShiftPeriod>` (`Ord` likewise by start time, with `PartialEq` consistent with it),
//! giving fully identical semantics.
//! - `getPeriods()` in Java returns a collection that reflects changes live, while Rust returns a snapshot
//! `Vec<ShiftPeriod>` (value semantics; `ShiftPeriod` is cloneable).

use std::cell::RefCell;
use std::collections::BTreeSet;

use crate::shift_period::ShiftPeriod;

/// Resource shift.
///
/// Corresponds to Java: `public final class Shift implements ProjectEntityWithUniqueID`
#[derive(Debug)]
pub struct Shift {
    /// Corresponds to Java field `private final Integer m_uniqueID`
    unique_id: Option<i32>,
    /// Corresponds to Java field `private final String m_name`
    name: Option<String>,
    /// Corresponds to Java field `private final Set<ShiftPeriod> m_periods = new TreeSet<>()`
    periods: RefCell<BTreeSet<ShiftPeriod>>,
}

impl Shift {
/// Get the shift unique ID.
    ///
    /// Corresponds to Java: `@Override public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.unique_id
    }

/// Get the shift name.
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

/// Get all periods of this shift in ascending start-time order (Java returns a live collection; Rust returns a snapshot).
    ///
    /// Corresponds to Java: `public Collection<ShiftPeriod> getPeriods()`
    pub fn get_periods(&self) -> Vec<ShiftPeriod> {
        self.periods
            .try_borrow()
            .map(|periods| periods.iter().cloned().collect())
            .unwrap_or_default()
    }

/// Add a shift period to this shift.
    ///
/// Corresponds to Java: `m_parentShift.getPeriods().add(this)` in the `ShiftPeriod` constructor
/// (Java deduplicates via the collection itself; Rust deduplicates the same way via `BTreeSet`).
    ///
/// This method is only called inside [`crate::shift_period::Builder::build`],
/// where the call site does not overlap with other borrows, so `try_borrow_mut` does not fail.
    pub(crate) fn add_period(&self, period: ShiftPeriod) {
        if let Ok(mut periods) = self.periods.try_borrow_mut() {
            periods.insert(period);
        }
    }
}

/// Shift builder.
///
/// Corresponds to Java: `public static class Shift.Builder`
///
/// Java's `Builder` methods modify in place and return `this`; the Rust side uses the "consume self and return `Self`" style
/// consistent with `crate::unit_of_measure::Builder`.
#[derive(Debug)]
pub struct Builder {
    /// Corresponds to Java field `private Integer m_uniqueID`
    unique_id: Option<i32>,
    /// Corresponds to Java field `private String m_name`
    name: Option<String>,
}

impl Builder {
/// Constructor.
    ///
    /// Corresponds to Java: `public Builder(UniqueIdObjectSequenceProvider sequenceProvider)`
    ///
/// `UniqueIdObjectSequenceProvider` has been ported (see the file header), but this type does not yet accept that parameter,
/// so the Rust side does not accept a sequence provider and the caller passes the unique ID directly.
    pub fn new() -> Self {
        Self {
            unique_id: None,
            name: None,
        }
    }

/// Set the unique ID.
    ///
    /// Corresponds to Java: `public Builder uniqueID(Integer value)`
    pub fn unique_id(mut self, value: Option<i32>) -> Self {
        self.unique_id = value;
        self
    }

/// Set the name.
    ///
    /// Corresponds to Java: `public Builder name(String value)`
    pub fn name(mut self, value: Option<String>) -> Self {
        self.name = value;
        self
    }

/// Build a [`Shift`] instance.
    ///
    /// Corresponds to Java: `public Shift build()`
    ///
/// Java can return a plain reference, but on the Rust side shift periods need a weak reference back to the parent shift,
/// so it returns `Rc<Shift>` (see the file header note).
    pub fn build(self) -> std::rc::Rc<Shift> {
        std::rc::Rc::new(Shift {
            unique_id: self.unique_id,
            name: self.name,
            periods: RefCell::new(BTreeSet::new()),
        })
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_sets_unique_id_and_name() {
        let shift = Builder::new()
            .unique_id(Some(7))
            .name(Some("Morning shift".to_string()))
            .build();

        assert_eq!(shift.get_unique_id(), Some(7));
        assert_eq!(shift.get_name(), Some("Morning shift"));
        assert!(shift.get_periods().is_empty());
    }

    #[test]
    fn builder_defaults_are_empty() {
        let shift = Builder::new().build();
        assert_eq!(shift.get_unique_id(), None);
        assert_eq!(shift.get_name(), None);
    }

    #[test]
    fn add_period_keeps_periods_sorted_and_deduplicated() {
        use crate::shift_period::Builder as PeriodBuilder;

        let shift = Builder::new().build();

        PeriodBuilder::new(&shift)
            .unique_id(Some(2))
            .start_from_hour(20)
            .build();

        PeriodBuilder::new(&shift)
            .unique_id(Some(1))
            .start_from_hour(8)
            .build();

// Java: TreeSet sorts by start time
        let periods = shift.get_periods();
        assert_eq!(periods.len(), 2);
        assert_eq!(periods[0].get_unique_id(), Some(1));
        assert_eq!(periods[1].get_unique_id(), Some(2));

// Java: TreeSet uses compareTo to detect duplicates, so periods with the same start time are not added
        PeriodBuilder::new(&shift)
            .unique_id(Some(3))
            .start_from_hour(8)
            .build();
        assert_eq!(shift.get_periods().len(), 2);
    }
}
