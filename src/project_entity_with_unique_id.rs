//! Corresponds to Java: `org/mpxj/ProjectEntityWithUniqueID.java`

/// Entity identified by a unique ID.
///
/// Corresponds to Java: `public interface ProjectEntityWithUniqueID`
///
/// Java's interface inheritance is expressed in Rust with trait supertraits:
///
/// ```text
/// ProjectEntityWithID            extends ProjectEntityWithMutableUniqueID
/// ProjectEntityWithMutableUniqueID extends ProjectEntityWithUniqueID
/// ```
///
/// Therefore an entity implementing [`crate::project_entity_with_id::ProjectEntityWithID`]
/// automatically satisfies [`crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID`]
/// and this trait.
///
/// # Differences from Java
///
/// Java's `getUniqueID()` returns a nullable `Integer`; the Rust side is `Option<i32>`.
///
/// Skipped Java mechanisms: none.
///
/// Unported Java methods: none.
pub trait ProjectEntityWithUniqueID {
    /// Return the entity's unique ID.
    ///
    /// Corresponds to Java: `Integer getUniqueID()`
    fn get_unique_id(&self) -> Option<i32>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test entity: implements only unique ID reading.
    struct Entity {
        unique_id: Option<i32>,
    }

    impl ProjectEntityWithUniqueID for Entity {
        fn get_unique_id(&self) -> Option<i32> {
            self.unique_id
        }
    }

    #[test]
    fn unique_id_is_exposed_as_option() {
        let entity = Entity {
            unique_id: Some(7),
        };
        assert_eq!(entity.get_unique_id(), Some(7));

        let entity = Entity { unique_id: None };
        assert_eq!(entity.get_unique_id(), None);
    }
}
