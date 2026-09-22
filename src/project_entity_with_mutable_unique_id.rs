//! Corresponds to Java: `org/mpxj/ProjectEntityWithMutableUniqueID.java`

use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;

/// An entity with a **mutable** unique ID.
///
/// Corresponds to Java: `public interface ProjectEntityWithMutableUniqueID extends ProjectEntityWithUniqueID`
///
/// Interface inheritance on the Rust side is expressed with supertraits; a type implementing this trait must also implement
/// [`ProjectEntityWithUniqueID`].
///
/// # Differences from Java
///
/// Java's `setUniqueID(Integer id)` accepts nullable values; the Rust side uses `Option<i32>`, and passing `None`
/// is equivalent to passing `null` in Java.
///
/// Skipped Java mechanisms: none.
///
/// Unported Java methods: none.
pub trait ProjectEntityWithMutableUniqueID: ProjectEntityWithUniqueID {
    /// Sets the entity unique ID.
    ///
    /// Corresponds to Java: `void setUniqueID(Integer id)`
    fn set_unique_id(&mut self, id: Option<i32>);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test entity: unique ID is readable and writable.
    struct Entity {
        unique_id: Option<i32>,
    }

    impl ProjectEntityWithUniqueID for Entity {
        fn get_unique_id(&self) -> Option<i32> {
            self.unique_id
        }
    }

    impl ProjectEntityWithMutableUniqueID for Entity {
        fn set_unique_id(&mut self, id: Option<i32>) {
            self.unique_id = id;
        }
    }

    #[test]
    fn unique_id_can_be_set_and_cleared() {
        let mut entity = Entity { unique_id: None };

        entity.set_unique_id(Some(3));
        assert_eq!(entity.get_unique_id(), Some(3));

        entity.set_unique_id(None);
        assert_eq!(entity.get_unique_id(), None);
    }
}
