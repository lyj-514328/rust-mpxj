//! Corresponds to Java: `org/mpxj/UniqueIdObjectSequenceProvider.java`

use std::any::TypeId;
use std::rc::Rc;

use crate::common::object_sequence::ObjectSequence;

/// Interface that provides a unique ID sequence generator per type.
///
/// Corresponds to Java: `public interface UniqueIdObjectSequenceProvider`
///
/// # Differences from Java: `Class<?>` keys changed to `std::any::TypeId`
///
/// The Java side uses `Class<?>` as the key (internally `ProjectContext` actually uses the `c.getName()` string as the key),
/// Rust has no class literals, so [`TypeId`] is used instead:
///
/// | Java | Rust |
/// |---|---|
/// | `provider.getUniqueIdObjectSequence(Task.class)` | `provider.get_unique_id_object_sequence(TypeId::of::<Task>())` |
/// | `provider.getUniqueIdObjectSequence(Task.class)` (generic form) | [`get_unique_id_object_sequence`] |
///
/// The trait method is defined as "taking a `TypeId` parameter" rather than as a generic method so that this trait
/// can still be used as a trait object (the container holds
/// `Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>`); a generic method would break object safety.
/// When writing by type, use the generic helper function [`get_unique_id_object_sequence`].
///
/// # Differences from Java: the return value uses `Rc<ObjectSequence>`
///
/// The Java side returns **the same** `ObjectSequence` instance every time (`ProjectContext` uses
/// `computeIfAbsent` to cache it). The Rust side uses `Rc<ObjectSequence>` to express this sharing; because
/// [`ObjectSequence`] internally uses `Cell<i32>`, no extra `RefCell` is needed when sharing.
pub trait UniqueIdObjectSequenceProvider {
/// Get (creating if necessary) the sequence generator for the specified type.
    ///
    /// Corresponds to Java: `ObjectSequence getUniqueIdObjectSequence(Class<?> c)`
    ///
/// The Java implementation caches the newly created `ObjectSequence` internally, so the Rust side takes `&mut self`.
    fn get_unique_id_object_sequence(&mut self, type_id: TypeId) -> Rc<ObjectSequence>;
}

/// Generic entry point for getting the sequence generator by type, corresponding to Java's
/// `provider.getUniqueIdObjectSequence(T.class)`.
pub fn get_unique_id_object_sequence<T: 'static>(
    provider: &mut dyn UniqueIdObjectSequenceProvider,
) -> Rc<ObjectSequence> {
    provider.get_unique_id_object_sequence(TypeId::of::<T>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

/// Test implementation: caches sequence generators by `TypeId`, behaving like Java's `ProjectContext`.
    #[derive(Default)]
    struct TestProvider {
        sequences: HashMap<TypeId, Rc<ObjectSequence>>,
    }

    impl UniqueIdObjectSequenceProvider for TestProvider {
        fn get_unique_id_object_sequence(&mut self, type_id: TypeId) -> Rc<ObjectSequence> {
            Rc::clone(
                self.sequences
                    .entry(type_id)
                    .or_insert_with(|| Rc::new(ObjectSequence::new(1))),
            )
        }
    }

/// Test "entity" type, used only to distinguish `TypeId`s.
    struct Alpha;

/// Test "entity" type, used only to distinguish `TypeId`s.
    struct Beta;

    #[test]
    fn sequences_are_shared_for_the_same_type() {
        let mut provider = TestProvider::default();

        let alpha = get_unique_id_object_sequence::<Alpha>(&mut provider);
        let alpha_again = get_unique_id_object_sequence::<Alpha>(&mut provider);
        let beta = get_unique_id_object_sequence::<Beta>(&mut provider);

// The same type shares the same sequence
        assert!(Rc::ptr_eq(&alpha, &alpha_again));
        assert_eq!(alpha.get_next(), 1);
        assert_eq!(alpha_again.get_next(), 2);

// Different types are independent
        assert!(!Rc::ptr_eq(&alpha, &beta));
        assert_eq!(beta.get_next(), 1);
    }

    #[test]
    fn provider_can_be_shared_as_trait_object() {
        let provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            Rc::new(RefCell::new(TestProvider::default()));

        let sequence = provider
            .borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<Alpha>());

        assert_eq!(sequence.sync_or_get_next(None), 1);
        assert_eq!(sequence.sync_or_get_next(Some(10)), 10);
        assert_eq!(sequence.get_next(), 11);
    }
}
