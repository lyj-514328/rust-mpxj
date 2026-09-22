//! Corresponds to Java: `org/mpxj/common/ObjectSequence.java`

use std::cell::Cell;
use std::fmt::{self, Debug, Formatter};

/// Simple sequence number generator.
///
/// Corresponds to Java: `public final class ObjectSequence`
///
/// # Differences from Java: mutable field changed to `Cell<i32>`
///
/// In Java, `m_id` is a mutable `int` field, and `sync` / `getNext` modify it in
/// place via instance methods. The Rust side uses `Cell<i32>` instead, so all
/// methods take `&self`, allowing it to be placed in `Rc<ObjectSequence>` and
/// shared between containers and entities (see
/// [`crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider`]),
/// with no extra `RefCell` wrapper.
///
/// ## Skipped Java code
///
/// None.
///
/// Unported Java methods: none.
pub struct ObjectSequence {
    /// The next ID to be allocated.
    ///
    /// Corresponds to Java: `private int m_id;`
    m_id: Cell<i32>,
}

impl ObjectSequence {
    /// Constructor.
    ///
    /// Corresponds to Java: `public ObjectSequence(int id)`
    pub fn new(id: i32) -> Self {
        Self {
            m_id: Cell::new(id),
        }
    }

    /// Sync with a value known to be in use: if the value is not less than the
    /// current sequence, advance the sequence to one past it.
    ///
    /// Corresponds to Java: `public void sync(Integer id)`
    ///
    /// In Java, `id` being `null` does nothing; the Rust side uses `Option<i32>`
    /// to express this.
    pub fn sync(&self, id: Option<i32>) {
        if let Some(id) = id {
            if id >= self.m_id.get() {
                self.m_id.set(id + 1);
            }
        }
    }

    /// Get the next value in the sequence.
    ///
    /// Corresponds to Java: `public Integer getNext()`
    pub fn get_next(&self) -> i32 {
        let result = self.m_id.get();
        self.m_id.set(result + 1);
        result
    }

    /// If `id` is not `null`, sync with it; otherwise generate a new value.
    ///
    /// Corresponds to Java: `public Integer syncOrGetNext(Integer id)`
    pub fn sync_or_get_next(&self, id: Option<i32>) -> i32 {
        match id {
            Some(id) => {
                self.sync(Some(id));
                id
            }
            None => self.get_next(),
        }
    }
}

impl Debug for ObjectSequence {
    /// Java's `ObjectSequence` does not override `toString()`; the Rust side,
    /// for easier testing and diagnostics, outputs the next value to be allocated.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObjectSequence")
            .field("m_id", &self.m_id.get())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_next_increments_from_initial_value() {
        let sequence = ObjectSequence::new(1);

        assert_eq!(sequence.get_next(), 1);
        assert_eq!(sequence.get_next(), 2);
        assert_eq!(sequence.get_next(), 3);
    }

    #[test]
    fn sync_moves_sequence_forward_only() {
        let sequence = ObjectSequence::new(1);

        // Smaller than the current value: no effect
        sequence.sync(Some(0));
        assert_eq!(sequence.get_next(), 1);

        // Greater than or equal to the current value: advance to one past it
        sequence.sync(Some(10));
        assert_eq!(sequence.get_next(), 11);

        // Equal: also advances
        sequence.sync(Some(11));
        assert_eq!(sequence.get_next(), 12);

        // null: no effect
        sequence.sync(None);
        assert_eq!(sequence.get_next(), 13);
    }

    #[test]
    fn sync_or_get_next_matches_java() {
        let sequence = ObjectSequence::new(1);

        // id non-null: sync and return as-is
        assert_eq!(sequence.sync_or_get_next(Some(4)), 4);
        assert_eq!(sequence.get_next(), 5);

        // id null: generate a new value
        assert_eq!(sequence.sync_or_get_next(None), 6);
        assert_eq!(sequence.sync_or_get_next(None), 7);
    }
}
