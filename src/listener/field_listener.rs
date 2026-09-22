//! Corresponds to Java: `org/mpxj/listener/FieldListener.java`

use crate::field_key::FieldKey;
use crate::field_value::FieldValue;

/// Interface for receiving notification of Task / Resource field changes.
///
/// Corresponds to Java: `public interface FieldListener`
///
/// # Differences from Java
///
/// The Java signature is
/// `void fieldChange(FieldContainer container, FieldType type, Object oldValue, Object newValue)`,
/// which passes the source container itself. On the Rust side, `set` holds a `&mut` borrow of
/// the container, so `&self` can no longer be handed to the listener (an aliasing borrow is not
/// allowed); therefore the `container` parameter is omitted.
/// The reader path never registers a field listener, so this difference does not affect read behavior.
pub trait FieldListener {
    /// Called when a field value changes.
    ///
    /// Corresponds to Java: `void fieldChange(FieldContainer container, FieldType type, Object oldValue, Object newValue)`
    fn field_change(
        &mut self,
        field: FieldKey,
        old_value: Option<&FieldValue>,
        new_value: Option<&FieldValue>,
    );
}
