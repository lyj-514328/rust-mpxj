//! Corresponds to Java: `org/mpxj/ProjectCalendarHours.java`
//!
//! Skipped Java methods: none.
//!
//! About methods inherited from `AbstractList` / `AbstractCollection`:
//! - Java's `addAll(Collection)` does not appear in the Java source file, but `ProjectCalendarWeek`
//! and `ProjectCalendarException` both call it, so this file provides [`ProjectCalendarHours::add_all`];
//! - The remaining inherited methods (`isEmpty`, `indexOf`, `contains`, `subList`, etc.) are not called by higher-level code,
//! and Rust's `Vec` / slices already express the same capability directly, so they are not rebuilt one by one.
//! - `equals` / `hashCode` are provided by `AbstractList` (compare by element, compute hash by element);
//! Rust expresses these via derived `PartialEq` / `Eq` / `Hash`, with the same semantics.

use std::fmt::{self, Display, Formatter};
use std::slice::Iter;

use crate::local_time_range::LocalTimeRange;

/// Represents the collection of working hours in a calendar (there can be multiple working periods in a day).
///
/// Corresponds to Java: `public class ProjectCalendarHours extends AbstractList<LocalTimeRange>`
///
/// Porting note: Java gains List semantics by inheriting `AbstractList`; Rust uses an internal
/// `Vec<LocalTimeRange>` and keeps the methods explicitly overridden in the Java source file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ProjectCalendarHours {
    /// Corresponds to Java field `private final List<LocalTimeRange> m_ranges`
    ranges: Vec<LocalTimeRange>,
}

impl ProjectCalendarHours {
/// Constructor.
    ///
/// Corresponds to Java: implicit no-arg constructor `ProjectCalendarHours()`
    pub fn new() -> Self {
        Self::default()
    }

/// Adds a working period and always returns `true` (consistent with Java `AbstractList.add`).
    ///
    /// Corresponds to Java: `@Override public boolean add(LocalTimeRange range)`
    pub fn add(&mut self, range: LocalTimeRange) -> bool {
        self.ranges.push(range);
        true
    }

/// Replaces the working period at the given index and returns the replaced old value.
    ///
    /// Corresponds to Java: `@Override public LocalTimeRange set(int index, LocalTimeRange value)`
    ///
/// Java throws `IndexOutOfBoundsException` when the index is out of bounds; Rust returns `None`.
/// Java's `int index` may be negative; Rust uses `usize` (negatives are not representable).
    pub fn set(&mut self, index: usize, value: LocalTimeRange) -> Option<LocalTimeRange> {
        match self.ranges.get_mut(index) {
            Some(slot) => Some(std::mem::replace(slot, value)),
            None => None,
        }
    }

/// Gets the working period at the given index.
    ///
    /// Corresponds to Java: `@Override public LocalTimeRange get(int index)`
    ///
/// Returns [`LocalTimeRange::EMPTY_RANGE`] when the index is out of bounds, matching Java;
/// on the Rust side the index is `usize`, so Java's "negative index returns an empty range" branch is not representable.
    pub fn get(&self, index: usize) -> LocalTimeRange {
        self.ranges
            .get(index)
            .copied()
            .unwrap_or(LocalTimeRange::EMPTY_RANGE)
    }

/// Iterates over the working periods (in insertion order).
    ///
    /// Corresponds to Java: `@Override public Iterator<LocalTimeRange> iterator()`
    pub fn iter(&self) -> Iter<'_, LocalTimeRange> {
        self.ranges.iter()
    }

/// Number of working periods.
    ///
    /// Corresponds to Java: `@Override public int size()`
    pub fn size(&self) -> usize {
        self.ranges.len()
    }

/// Clears all working periods.
    ///
    /// Corresponds to Java: `@Override public void clear()`
    pub fn clear(&mut self) {
        self.ranges.clear();
    }

/// Appends all periods from another working-hour collection.
    ///
/// Corresponds to Java: `AbstractCollection.addAll(Collection)` (provided by the base class;
/// `ProjectCalendarWeek#convertToRecurringExceptions` and
/// `ProjectCalendarException#getExpandedExceptions` call it).
    ///
/// Returns whether a change occurred (same meaning as the Java `addAll` return value).
    pub fn add_all(&mut self, other: &ProjectCalendarHours) -> bool {
        if other.ranges.is_empty() {
            return false;
        }

        self.ranges.extend_from_slice(&other.ranges);
        true
    }
}

impl<'a> IntoIterator for &'a ProjectCalendarHours {
    type Item = &'a LocalTimeRange;
    type IntoIter = Iter<'a, LocalTimeRange>;

/// Corresponds to Java's enhanced for-loop semantics (`for (LocalTimeRange range : this)`).
    fn into_iter(self) -> Self::IntoIter {
        self.ranges.iter()
    }
}

impl Display for ProjectCalendarHours {
    /// Corresponds to Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[ProjectCalendarHours ")?;
        for range in &self.ranges {
            write!(f, "{}", range)?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    use chrono::NaiveTime;

    use super::*;

    fn time(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).expect("valid time")
    }

    fn range(from: u32, to: u32) -> LocalTimeRange {
        LocalTimeRange::new(Some(time(from, 0)), Some(time(to, 0)))
    }

    #[test]
    fn add_size_and_get() {
        let mut hours = ProjectCalendarHours::new();
        assert!(hours.add(range(8, 12)));
        assert!(hours.add(range(13, 17)));

        assert_eq!(hours.size(), 2);
        assert_eq!(hours.get(0), range(8, 12));
        assert_eq!(hours.get(1), range(13, 17));
// Java: out of bounds returns EMPTY_RANGE
        assert_eq!(hours.get(2), LocalTimeRange::EMPTY_RANGE);
        assert_eq!(hours.get(usize::MAX), LocalTimeRange::EMPTY_RANGE);
    }

    #[test]
    fn set_returns_previous_value() {
        let mut hours = ProjectCalendarHours::new();
        hours.add(range(8, 12));

// Java: returns the replaced old value
        assert_eq!(hours.set(0, range(9, 12)), Some(range(8, 12)));
        assert_eq!(hours.get(0), range(9, 12));
// Java: out of bounds throws IndexOutOfBoundsException; Rust returns None
        assert_eq!(hours.set(5, range(9, 12)), None);
    }

    #[test]
    fn clear_and_add_all() {
        let mut hours = ProjectCalendarHours::new();
        hours.add(range(8, 12));

        let mut other = ProjectCalendarHours::new();
        other.add(range(13, 17));
        other.add(range(18, 20));

        assert!(hours.add_all(&other));
        assert_eq!(hours.size(), 3);
        assert!(!hours.add_all(&ProjectCalendarHours::new()));

        hours.clear();
        assert_eq!(hours.size(), 0);
    }

    #[test]
    fn iteration_order() {
        let mut hours = ProjectCalendarHours::new();
        hours.add(range(8, 12));
        hours.add(range(13, 17));

        let collected: Vec<LocalTimeRange> = hours.iter().copied().collect();
        assert_eq!(collected, vec![range(8, 12), range(13, 17)]);

        let mut count = 0;
        for _ in &hours {
            count += 1;
        }
        assert_eq!(count, 2);
    }

    #[test]
    fn display_matches_java_format() {
        let mut hours = ProjectCalendarHours::new();
        hours.add(range(8, 12));
        assert_eq!(
            hours.to_string(),
            "[ProjectCalendarHours [LocalTimeRange start=08:00:00 end=12:00:00]]"
        );
        assert_eq!(ProjectCalendarHours::new().to_string(), "[ProjectCalendarHours ]");
    }

    #[test]
    fn equals_matches_abstract_list_semantics() {
        let mut lhs = ProjectCalendarHours::new();
        lhs.add(range(8, 12));

        let mut rhs = ProjectCalendarHours::new();
        rhs.add(range(8, 12));

        assert_eq!(lhs, rhs);

        rhs.add(range(13, 17));
        assert_ne!(lhs, rhs);

        let mut lhs_hash = DefaultHasher::new();
        lhs.hash(&mut lhs_hash);
        let mut rhs_hash = DefaultHasher::new();
        lhs.hash(&mut rhs_hash);
        assert_eq!(lhs_hash.finish(), rhs_hash.finish());
    }
}
