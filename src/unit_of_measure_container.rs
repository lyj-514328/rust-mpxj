//! Corresponds to Java: `org/mpxj/UnitOfMeasureContainer.java`

use crate::unit_of_measure::{Builder, UnitOfMeasure};

/// The collection of units of measure available to the current project.
///
/// Corresponds to Java: `public class UnitOfMeasureContainer extends ProjectEntityContainer<UnitOfMeasure>`
///
/// Porting notes:
/// - In Java it extends `ProjectEntityContainer<UnitOfMeasure>` (above which sit the
///   `ProjectEntityWithIDContainer` / `ListWithCallbacks` hierarchy); this repository does
///   **not compose** these base classes (they have all landed but are pending wiring, see
///   `TECH_DEBT.md` section 6), so the Rust side **is first implemented as a standalone simple
///   container**: an internal `Vec<UnitOfMeasure>` plus lookup by id/uniqueId; the inheritance
///   relationship will be aligned once wired (including callbacks, unique ID clash handling,
///   etc.).
/// - Java's constructor takes a `UniqueIdObjectSequenceProvider` used to allocate unique IDs;
///   that type has landed, but the Rust constructor takes no arguments, and the unique ID is
///   approximated inside [`UnitOfMeasureContainer::build_unit_of_measure`] as "current maximum
///   unique ID + 1" (on the Java side `ObjectSequence` allocates incrementally from 1).
///
/// Skipped Java code: methods inherited from `ProjectEntityContainer` (`getByUniqueID` is
/// mapped to [`UnitOfMeasureContainer::get_by_unique_id`]; `updateUniqueID`,
/// `fixUniqueIdClashes`, `getSequenceProvider`, the `ListWithCallbacks` callbacks, etc.),
/// because this repository does not compose these base classes (see `TECH_DEBT.md` section 6).
///
/// Unported Java methods: none.
#[derive(Debug, Clone, Default)]
pub struct UnitOfMeasureContainer {
    /// Units of measure in the container.
    ///
    /// Corresponds to Java: the list storage inside `ProjectEntityContainer`.
    items: Vec<UnitOfMeasure>,
}

impl UnitOfMeasureContainer {
    /// Constructor.
    ///
    /// Corresponds to Java: `public UnitOfMeasureContainer(UniqueIdObjectSequenceProvider sequenceProvider)`
    ///
    /// Java's `sequenceProvider` parameter is used to allocate unique IDs via the base class
    /// (this repository does not compose the base class and it is not yet wired;
    /// `UniqueIdObjectSequenceProvider` has landed), and the Rust constructor takes no arguments.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Append a unit of measure.
    ///
    /// Corresponds to Java: `add(E element)` inherited by `ProjectEntityContainer` from `ArrayList`.
    pub fn add(&mut self, item: UnitOfMeasure) {
        self.items.push(item);
    }

    /// Return the unit of measure at the given index; returns `None` when out of bounds.
    ///
    /// Corresponds to Java: `ArrayList.get(int index)`.
    pub fn get(&self, index: usize) -> Option<&UnitOfMeasure> {
        self.items.get(index)
    }

    /// Number of units of measure.
    ///
    /// Corresponds to Java: `size()` inherited by `ProjectEntityContainer` from `ArrayList`.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the container is empty.
    ///
    /// Corresponds to Java: `isEmpty()` inherited by `ProjectEntityContainer` from `ArrayList`.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Iterate over the units of measure in the container.
    ///
    /// Corresponds to Java: `iterator()` inherited by `ProjectEntityContainer` from `ArrayList`.
    pub fn iter(&self) -> std::slice::Iter<'_, UnitOfMeasure> {
        self.items.iter()
    }

    /// Access the units of measure in the container as a slice.
    ///
    /// Corresponds to Java: `ProjectEntityContainer`'s list view.
    pub fn as_slice(&self) -> &[UnitOfMeasure] {
        self.items.as_slice()
    }

    /// Find a unit of measure by unique ID; returns `None` when not found.
    ///
    /// Corresponds to Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    ///
    /// On the Java side a `HashMap` with a `null` key finds nothing; the Rust side likewise returns `None` for `None`.
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<&UnitOfMeasure> {
        let id = id?;
        self.items
            .iter()
            .find(|item| item.get_unique_id() == Some(id))
    }

    /// Find a unit of measure by abbreviation, creating one if not found.
    ///
    /// Corresponds to Java: `public UnitOfMeasure getOrCreateByAbbreviation(String abbreviation)`
    ///
    /// Returns `None` when the abbreviation is `null` or an empty string (corresponding to Java's `null`).
    pub fn get_or_create_by_abbreviation(
        &mut self,
        abbreviation: Option<&str>,
    ) -> Option<&UnitOfMeasure> {
        let abbreviation = match abbreviation {
            Some(abbreviation) if !abbreviation.is_empty() => abbreviation,
            _ => return None,
        };

        let existing = self
            .items
            .iter()
            .position(|item| item.get_abbreviation() == Some(abbreviation));

        let index = match existing {
            Some(index) => index,
            None => self.build_unit_of_measure(abbreviation),
        };

        self.items.get(index)
    }

    /// Create a unit of measure from a name, add it to the container, and return the new element's index in the container.
    ///
    /// Corresponds to Java: `private UnitOfMeasure buildUnitOfMeasure(String name)`
    ///
    /// The Java side returns the newly created `UnitOfMeasure` instance; to avoid panicking in
    /// library code, the Rust side returns an index instead, which the caller borrows via `get`.
    fn build_unit_of_measure(&mut self, name: &str) -> usize {
        // Java: UniqueIdObjectSequenceProvider allocates the unique ID (not wired in this repository; approximated here)
        let unique_id = self
            .items
            .iter()
            .filter_map(|item| item.get_unique_id())
            .max()
            .unwrap_or(0)
            + 1;

        // Java: stream().mapToInt(u -> u.getSequenceNumber().intValue()).max().orElse(0) + 1
        let sequence_number = self
            .items
            .iter()
            .map(|item| item.get_sequence_number().unwrap_or(0))
            .max()
            .unwrap_or(0)
            + 1;

        let uom = Builder::new()
            .unique_id(Some(unique_id))
            .name(Some(name.to_owned()))
            .abbreviation(Some(name.to_owned()))
            .sequence_number(Some(sequence_number))
            .build();

        self.items.push(uom);
        self.items.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_basics_and_unique_id_lookup() {
        let mut container = UnitOfMeasureContainer::new();
        assert!(container.is_empty());

        container.add(
            Builder::new()
                .unique_id(Some(7))
                .name(Some("Days".to_owned()))
                .abbreviation(Some("d".to_owned()))
                .sequence_number(Some(1))
                .build(),
        );

        assert_eq!(container.len(), 1);
        assert_eq!(container.iter().count(), 1);
        assert_eq!(container.as_slice().len(), 1);
        assert_eq!(
            container.get(0).and_then(|item| item.get_name()),
            Some("Days")
        );

        assert_eq!(
            container
                .get_by_unique_id(Some(7))
                .and_then(|item| item.get_abbreviation()),
            Some("d")
        );
        assert!(container.get_by_unique_id(Some(8)).is_none());
        assert!(container.get_by_unique_id(None).is_none());
    }

    #[test]
    fn get_or_create_returns_none_for_empty_abbreviation() {
        let mut container = UnitOfMeasureContainer::new();

        assert!(container.get_or_create_by_abbreviation(None).is_none());
        assert!(container.get_or_create_by_abbreviation(Some("")).is_none());
        assert!(container.is_empty());
    }

    #[test]
    fn get_or_create_creates_and_reuses_entries() {
        let mut container = UnitOfMeasureContainer::new();

        // First call creates a new element: name and abbreviation both take the abbreviation, and the sequence number and unique ID start from 1
        {
            let created = container.get_or_create_by_abbreviation(Some("days"));
            assert_eq!(created.and_then(|item| item.get_name()), Some("days"));
            assert_eq!(
                created.and_then(|item| item.get_sequence_number()),
                Some(1)
            );
            assert_eq!(created.and_then(|item| item.get_unique_id()), Some(1));
        }
        assert_eq!(container.len(), 1);

        // Subsequent call reuses the existing element without adding a new one
        {
            let existing = container.get_or_create_by_abbreviation(Some("days"));
            assert_eq!(existing.and_then(|item| item.get_unique_id()), Some(1));
        }
        assert_eq!(container.len(), 1);

        // A new abbreviation creates a second element, with the sequence number and unique ID incremented
        {
            let second = container.get_or_create_by_abbreviation(Some("hours"));
            assert_eq!(second.and_then(|item| item.get_sequence_number()), Some(2));
            assert_eq!(second.and_then(|item| item.get_unique_id()), Some(2));
        }
        assert_eq!(container.len(), 2);
    }
}
