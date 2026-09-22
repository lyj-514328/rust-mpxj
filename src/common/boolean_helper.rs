//! Corresponds to Java: `org/mpxj/common/BooleanHelper.java`

/// Utility methods for handling `Boolean` objects and the `boolean` primitive type.
///
/// Corresponds to Java: `public final class BooleanHelper`
///
/// Skipped Java methods: none.
pub struct BooleanHelper;

impl BooleanHelper {
    /// Corresponds to Java: `public static final boolean getBoolean(Boolean value)`
    ///
    /// Handles null: only an explicit `Some(true)` returns `true`.
    pub fn get_boolean(value: Option<bool>) -> bool {
        value == Some(true)
    }
}
