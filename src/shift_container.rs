//! Corresponds to Java: `org/mpxj/ShiftContainer.java`
//!
//! Skipped Java methods and reasons: no methods custom to this class; all Java behavior comes from the base class
//! `ProjectEntityContainer<Shift>` (whose parent is `ProjectEntityWithIDContainer` /
//! `ListWithCallbacks`); this crate does **not compose** these base classes (they are all in place but pending wiring, see
//! `TECH_DEBT.md` section six), so we follow
//! the same trade-off as [`crate::unit_of_measure_container::UnitOfMeasureContainer`]:
//!
//! | Java (inherited from base class) | Rust |
//! |---|---|
//! | `add(E element)` | [`ShiftContainer::add`] |
//! | `get(int index)` / `size()` / `isEmpty()` / `iterator()` | [`ShiftContainer::get`] / [`ShiftContainer::len`] / [`ShiftContainer::is_empty`] / [`ShiftContainer::iter`] |
//! | `getByUniqueID(Integer id)` | [`ShiftContainer::get_by_unique_id`] |
//! | `updateUniqueID(element, oldID, newID)`, `fixUniqueIdClashes()`, `getSequenceProvider()`, the `ListWithCallbacks` callbacks | Not wired (both `ProjectEntityContainer` and `UniqueIdObjectSequenceProvider` are in place, see `TECH_DEBT.md` section six) |
//!
//! # Differences from Java
//!
//! - **Constructor**: Java's `ShiftContainer(UniqueIdObjectSequenceProvider sequenceProvider)`
//! passes the sequence provider to the base class and uses it only in `updateUniqueID` / `fixUniqueIdClashes`; these two methods
//! are not yet wired in this crate (same reason as above), and [`crate::shift::Builder`] likewise does not accept a sequence provider,
//! so the Rust constructor takes no parameters.
//! - **Element form**: Java stores `Shift` references; [`crate::shift::Builder::build`] returns
//! `Rc<Shift>` (shift periods need a weak reference back to the parent shift), so the container stores `Rc<Shift>`.
//! - **Lookup**: Java indexes unique IDs with `Map<Integer, Shift>`; the Rust side searches linearly in order,
//! avoiding a stale index after a `Shift`'s unique ID changes (Java maintains the index via `updateUniqueID`).
//! - `getByUniqueID(null)` finds no value in Java; the Rust side likewise returns `None`.

use std::rc::Rc;

use crate::shift::Shift;

/// The collection of shifts available in the current project.
///
/// Corresponds to Java: `public class ShiftContainer extends ProjectEntityContainer<Shift>`
#[derive(Debug, Clone, Default)]
pub struct ShiftContainer {
/// The shifts in the container.
    ///
/// Corresponds to Java: the internal list storage of `ProjectEntityContainer`.
    items: Vec<Rc<Shift>>,
}

impl ShiftContainer {
/// Constructor.
    ///
    /// Corresponds to Java: `public ShiftContainer(UniqueIdObjectSequenceProvider sequenceProvider)`
    ///
/// Java's `sequenceProvider` parameter is used only by base class methods (this crate does not compose the base class); the Rust constructor takes no parameters
/// (see the file header).
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

/// Append a shift.
    ///
/// Corresponds to Java: `ProjectEntityContainer` inherits from `ListWithCallbacks` / `ArrayList`
/// the `add(E element)` method.
    pub fn add(&mut self, shift: Rc<Shift>) {
        self.items.push(shift);
    }

/// Get a shift by index; returns `None` if out of bounds.
    ///
    /// Corresponds to Java: `ArrayList.get(int index)`.
    pub fn get(&self, index: usize) -> Option<&Rc<Shift>> {
        self.items.get(index)
    }

/// Number of shifts.
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

/// Iterate over the shifts in the container.
    ///
/// Corresponds to Java: `ProjectEntityContainer` inherits `iterator()` from `ArrayList`.
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<Shift>> {
        self.items.iter()
    }

/// Access the shifts in the container as a slice.
    ///
/// Corresponds to Java: the list view of `ProjectEntityContainer`.
    pub fn as_slice(&self) -> &[Rc<Shift>] {
        self.items.as_slice()
    }

/// Find a shift by unique ID; returns `None` if not found.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<&Rc<Shift>> {
        let id = id?;
        self.items
            .iter()
            .find(|shift| shift.get_unique_id() == Some(id))
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

    fn shift(unique_id: Option<i32>, name: &str) -> Rc<Shift> {
        ShiftBuilder::new()
            .unique_id(unique_id)
            .name(Some(name.to_owned()))
            .build()
    }

    #[test]
    fn container_basics() {
        let mut container = ShiftContainer::new();
        assert!(container.is_empty());

        container.add(shift(Some(1), "Morning shift"));
        container.add(shift(Some(2), "Evening shift"));

        assert_eq!(container.len(), 2);
        assert!(!container.is_empty());
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert_eq!(
            container.get(1).and_then(|shift| shift.get_name()),
            Some("Evening shift")
        );
        assert!(container.get(2).is_none());
    }

    #[test]
    fn get_by_unique_id_returns_matching_shift() {
        let mut container = ShiftContainer::new();
        container.add(shift(Some(7), "Morning shift"));

        assert_eq!(
            container
                .get_by_unique_id(Some(7))
                .and_then(|shift| shift.get_name()),
            Some("Morning shift")
        );
        assert!(container.get_by_unique_id(Some(8)).is_none());
// Java: getByUniqueID(null) finds no value
        assert!(container.get_by_unique_id(None).is_none());
    }

    #[test]
    fn clear_removes_all_shifts() {
        let mut container = ShiftContainer::new();
        container.add(shift(Some(1), "Morning shift"));

        container.clear();
        assert!(container.is_empty());
        assert!(container.get_by_unique_id(Some(1)).is_none());
    }

    #[test]
    fn default_matches_new() {
        let container = ShiftContainer::default();
        assert!(container.is_empty());
    }
}
