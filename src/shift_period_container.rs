//! Corresponds to Java: `org/mpxj/ShiftPeriodContainer.java`
//!
//! Skipped Java methods and reasons: no methods custom to this class; all Java behavior comes from the base class
//! `ProjectEntityContainer<ShiftPeriod>` (whose parent is `ProjectEntityWithIDContainer` /
//! `ListWithCallbacks`); this crate does **not compose** these base classes (they are all in place but pending wiring, see
//! `TECH_DEBT.md` section six), following the same trade-off as
//! [`crate::unit_of_measure_container::UnitOfMeasureContainer`]:
//!
//! | Java (inherited from base class) | Rust |
//! |---|---|
//! | `add(E element)` | [`ShiftPeriodContainer::add`] |
//! | `get(int index)` / `size()` / `isEmpty()` / `iterator()` | [`ShiftPeriodContainer::get`] / [`ShiftPeriodContainer::len`] / [`ShiftPeriodContainer::is_empty`] / [`ShiftPeriodContainer::iter`] |
//! | `getByUniqueID(Integer id)` | [`ShiftPeriodContainer::get_by_unique_id`] |
//! | `updateUniqueID(element, oldID, newID)`, `fixUniqueIdClashes()`, `getSequenceProvider()`, the `ListWithCallbacks` callbacks | Not wired (both `ProjectEntityContainer` and `UniqueIdObjectSequenceProvider` are in place, see `TECH_DEBT.md` section six) |
//!
//! # Differences from Java
//!
//! - **Constructor**: Java's `ShiftPeriodContainer(UniqueIdObjectSequenceProvider sequenceProvider)`
//! passes the sequence provider to the base class and uses it only by base class methods (this crate does not compose the base class); the Rust constructor takes no parameters.
//! - **Element form**: Java stores `ShiftPeriod` references; the Rust side
//! [`crate::shift_period::ShiftPeriod`] is a value type (internally holding a `Weak<Shift>` back to the parent shift),
//! so the container stores `ShiftPeriod` directly, keeping the same value semantics as the
//! `BTreeSet<ShiftPeriod>` inside [`crate::shift::Shift`].
//! - **Lookup**: Java indexes unique IDs with `Map<Integer, ShiftPeriod>`; the Rust side searches linearly in order.
//! - `getByUniqueID(null)` finds no value in Java; the Rust side likewise returns `None`.

use crate::shift_period::ShiftPeriod;

/// The collection of shift periods available in the current project.
///
/// Corresponds to Java: `public class ShiftPeriodContainer extends ProjectEntityContainer<ShiftPeriod>`
#[derive(Debug, Clone, Default)]
pub struct ShiftPeriodContainer {
/// The shift periods in the container.
    ///
/// Corresponds to Java: the internal list storage of `ProjectEntityContainer`.
    items: Vec<ShiftPeriod>,
}

impl ShiftPeriodContainer {
/// Constructor.
    ///
    /// Corresponds to Java: `public ShiftPeriodContainer(UniqueIdObjectSequenceProvider sequenceProvider)`
    ///
/// Java's `sequenceProvider` parameter is used only by base class methods (this crate does not compose the base class); the Rust constructor takes no parameters
/// (see the file header).
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

/// Append a shift period.
    ///
/// Corresponds to Java: `ProjectEntityContainer` inherits from `ListWithCallbacks` / `ArrayList`
/// the `add(E element)` method.
    pub fn add(&mut self, period: ShiftPeriod) {
        self.items.push(period);
    }

/// Get a shift period by index; returns `None` if out of bounds.
    ///
    /// Corresponds to Java: `ArrayList.get(int index)`.
    pub fn get(&self, index: usize) -> Option<&ShiftPeriod> {
        self.items.get(index)
    }

/// Number of shift periods.
    ///
/// Corresponds to Java: `ProjectEntityContainer` inherits `size()` from `ArrayList`.
    pub fn len(&self) -> usize {
        self.items.len()
    }

/// Whether the container is empty.
    ///
/// Corresponds to Java: `ProjectEntityContainer` inherits `isEmpty()` from `ArrayList`.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

/// Iterate over the shift periods in the container.
    ///
/// Corresponds to Java: `ProjectEntityContainer` inherits `iterator()` from `ArrayList`.
    pub fn iter(&self) -> std::slice::Iter<'_, ShiftPeriod> {
        self.items.iter()
    }

/// Access the shift periods in the container as a slice.
    ///
/// Corresponds to Java: the list view of `ProjectEntityContainer`.
    pub fn as_slice(&self) -> &[ShiftPeriod] {
        self.items.as_slice()
    }

/// Find a shift period by unique ID; returns `None` if not found.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<&ShiftPeriod> {
        let id = id?;
        self.items
            .iter()
            .find(|period| period.get_unique_id() == Some(id))
    }

/// Clear the container.
    ///
/// Corresponds to Java: `ProjectEntityContainer` inherits `clear()` from `ArrayList`.
    ///
/// Note: Java's `ListWithCallbacks` fires callbacks on each individual removal; the Rust side does not compose that base class
/// (see the file header), so here we only clear the list.
    pub fn clear(&mut self) {
        self.items.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shift::Builder as ShiftBuilder;
    use crate::shift_period::Builder as ShiftPeriodBuilder;

    fn periods() -> (std::rc::Rc<crate::shift::Shift>, ShiftPeriod, ShiftPeriod) {
        let shift = ShiftBuilder::new().build();
        let morning = ShiftPeriodBuilder::new(&shift)
            .unique_id(Some(1))
            .start_from_hour(8)
            .build();
        let evening = ShiftPeriodBuilder::new(&shift)
            .unique_id(Some(2))
            .start_from_hour(20)
            .build();
        (shift, morning, evening)
    }

    #[test]
    fn container_basics() {
        let (_shift, morning, evening) = periods();
        let mut container = ShiftPeriodContainer::new();
        assert!(container.is_empty());

        container.add(morning);
        container.add(evening);

        assert_eq!(container.len(), 2);
        assert!(!container.is_empty());
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert_eq!(
            container.get(0).and_then(|period| period.get_unique_id()),
            Some(1)
        );
        assert!(container.get(2).is_none());
    }

    #[test]
    fn get_by_unique_id_returns_matching_period() {
        let (_shift, morning, _evening) = periods();
        let mut container = ShiftPeriodContainer::new();
        container.add(morning);

        assert_eq!(
            container
                .get_by_unique_id(Some(1))
                .and_then(|period| period.get_start()),
            chrono::NaiveTime::from_hms_opt(8, 0, 0)
        );
        assert!(container.get_by_unique_id(Some(8)).is_none());
// Java: getByUniqueID(null) finds no value
        assert!(container.get_by_unique_id(None).is_none());
    }

    #[test]
    fn clear_removes_all_periods() {
        let (_shift, morning, _evening) = periods();
        let mut container = ShiftPeriodContainer::new();
        container.add(morning);

        container.clear();
        assert!(container.is_empty());
        assert!(container.get_by_unique_id(Some(1)).is_none());
    }

    #[test]
    fn default_matches_new() {
        let container = ShiftPeriodContainer::default();
        assert!(container.is_empty());
    }
}
