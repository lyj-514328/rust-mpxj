//! Corresponds to Java: `org/mpxj/ProjectEntityWithID.java`

use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;

/// An entity that can be identified by ID (as distinct from unique ID).
///
/// Corresponds to Java: `public interface ProjectEntityWithID extends ProjectEntityWithMutableUniqueID`
///
/// Interface inheritance on the Rust side is expressed with supertraits; a type implementing this trait must also implement
/// [`ProjectEntityWithMutableUniqueID`] (and hence
/// [`crate::project_entity_with_unique_id::ProjectEntityWithUniqueID`]).
///
/// # Differences from Java
///
/// Java's `getID()` / `setID(Integer id)` accept nullable values, while the Rust side uses `Option<i32>`.
///
/// Skipped Java mechanisms: none.
///
/// Unported Java methods: none.
pub trait ProjectEntityWithID: ProjectEntityWithMutableUniqueID {
    /// Gets the entity ID.
    ///
    /// Corresponds to Java: `Integer getID()`
    fn get_id(&self) -> Option<i32>;

    /// Sets the entity ID.
    ///
    /// Corresponds to Java: `void setID(Integer id)`
    fn set_id(&mut self, id: Option<i32>);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;

    /// Test entity: has both an ID and a unique ID.
    struct Entity {
        id: Option<i32>,
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

    impl ProjectEntityWithID for Entity {
        fn get_id(&self) -> Option<i32> {
            self.id
        }

        fn set_id(&mut self, id: Option<i32>) {
            self.id = id;
        }
    }

    #[test]
    fn id_and_unique_id_are_independent() {
        let mut entity = Entity {
            id: Some(1),
            unique_id: Some(10),
        };

        entity.set_id(Some(2));
        entity.set_unique_id(Some(20));

        assert_eq!(entity.get_id(), Some(2));
        assert_eq!(entity.get_unique_id(), Some(20));

        entity.set_id(None);
        assert_eq!(entity.get_id(), None);
    }
}
