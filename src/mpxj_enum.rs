//! Corresponds to Java: `org/mpxj/MpxjEnum.java`

/// Common trait for all enums in MPXJ.
///
/// Corresponds to Java: `public interface MpxjEnum`
pub trait MpxjEnum {
    /// Returns the int value associated with this enum instance (not the ordinal).
    ///
    /// Corresponds to Java: `int getValue()`
    fn value(&self) -> i32;
}
