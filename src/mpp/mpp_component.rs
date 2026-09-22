//! Corresponds to Java: `org/mpxj/mpp/MPPComponent.java`
//!
//! The **common base class** for the classes that read each section of an MPP file; it provides only 4 methods that sequentially read primitive types from a stream:
//! `readByte` / `readShort` / `readInt` / `readByteArray`.
//!
//! # How Java's inheritance relationship is handled on the Rust side
//!
//! Java: `class MPPComponent`, which `Props` / `AbstractVarMeta` / `FixDeferFix` / `CompObj` etc.
//! all `extends MPPComponent`. Rust has no inheritance, so this module degrades it to a **standalone utility type**:
//! Java's `readXxx(InputStream)` becomes [`MPPComponent::read_byte`] /
//! [`MPPComponent::read_short`] / [`MPPComponent::read_int`] /
//! [`MPPComponent::read_byte_array`]; the input source becomes a byte slice, and the cursor (read position) is held and advanced by the caller;
//! the semantics of each method are exactly the same as in Java.
//!
//! The read helpers of existing subclasses have been changed to **forward** to this module instead of each duplicating an equivalent implementation:
//!
//! | Module already in place | Forwarding method |
//! |---|---|
//! | `props.rs` | `InputStreamCursor`'s `read_int` / `read_short` / `read` forward to this module; its `available` / `skip` correspond to `InputStream.available()` / `InputStreamHelper.skip` (this module has no equivalent members) and remain in that file |
//! | `abstract_var_meta.rs` | the crate-internal free functions `read_byte` / `read_int` / `read_short` forward to this module |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `readByte(InputStream)` returns `int` | `Option<i32>` | value `0..=255`; when no byte can be read Java throws `EOFException`, Rust returns `None` |
//! | `readShort(InputStream)` returns `int` | `Option<i32>` | value `0..=65535`; same as above |
//! | `readInt(InputStream)` returns `int` | `Option<i32>` | may be negative (the highest byte forms the sign bit); same as above |
//! | `readByteArray(InputStream, int)` returns `byte[]` | `Option<&[u8]>` | borrows a sub-range of the input slice without copying; same as above |
//! | `protected MPPComponent()` | a fieldless unit struct | Java's protected constructor only exists to prevent external instantiation; the Rust side follows the same approach as `MPPUtility` |
//!
//! When fewer than the required bytes can be read, Rust always returns `None` **without moving the cursor** and never panics; the caller decides whether to
//! return `Err` or abort parsing.
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: not used by this class (Java only
//! `import java.io.IOException` / `java.io.InputStream`).
//!
//! Skipped Java members: none (the 4 methods correspond one by one; see the table above for the protected constructor).

use crate::common::byte_array_helper::ByteArrayHelper;

/// A little-endian read utility shared by the MPP sections.
///
/// Corresponds to Java: `class MPPComponent` (on the Java side it is inherited as a base class; on the Rust side it degrades to a standalone utility type)
pub struct MPPComponent;

impl MPPComponent {
/// Read a single byte.
    ///
    /// Corresponds to Java: `protected int readByte(InputStream is)`
    ///
/// Java implements this as `MPPUtility.getByte(InputStreamHelper.read(is, 1), 0)`, returning `0..=255`
/// as a positive integer; when fewer than 1 byte remains Java throws `EOFException`, Rust returns `None` (the cursor does not move).
    pub fn read_byte(data: &[u8], position: &mut usize) -> Option<i32> {
        let value = data.get(*position)?;
        let result = i32::from(*value);
        *position += 1;
        Some(result)
    }

/// Read a two-byte little-endian integer.
    ///
    /// Corresponds to Java: `protected int readShort(InputStream is)`
    ///
/// Returns `0..=65535`; when fewer than 2 bytes remain Java throws `EOFException`, Rust returns `None`.
    pub fn read_short(data: &[u8], position: &mut usize) -> Option<i32> {
        let end = position.checked_add(2)?;
        if end > data.len() {
            return None;
        }

        let result = ByteArrayHelper::get_short(data, *position);
        *position = end;
        Some(result)
    }

/// Read a four-byte little-endian integer.
    ///
    /// Corresponds to Java: `protected int readInt(InputStream is)`
    ///
/// The highest byte forms the sign bit, so the result may be negative (matching Java); when fewer than 4 bytes remain Java throws
/// `EOFException`; Rust returns `None` (the cursor does not move).
    pub fn read_int(data: &[u8], position: &mut usize) -> Option<i32> {
        let end = position.checked_add(4)?;
        if end > data.len() {
            return None;
        }

        let result = ByteArrayHelper::get_int(data, *position);
        *position = end;
        Some(result)
    }

/// Read a byte array of the specified length.
    ///
    /// Corresponds to Java: `protected byte[] readByteArray(InputStream is, int size)`
    ///
/// Java implements this as `InputStreamHelper.read(is, size)` (it throws if it cannot read enough); the Rust side returns
/// a borrowed sub-range of the input slice, and returns `None` when it cannot read enough (the cursor does not move).
    pub fn read_byte_array<'a>(
        data: &'a [u8],
        position: &mut usize,
        size: usize,
    ) -> Option<&'a [u8]> {
        let end = position.checked_add(size)?;
        if end > data.len() {
            return None;
        }

        let start = *position;
        *position = end;
        Some(&data[start..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_scalars_little_endian() {
        let data = [0xFFu8, 0x34, 0x12, 0x78, 0x56, 0x34, 0x12];
        let mut position = 0usize;

        assert_eq!(MPPComponent::read_byte(&data, &mut position), Some(255));
        assert_eq!(MPPComponent::read_short(&data, &mut position), Some(0x1234));
        assert_eq!(
            MPPComponent::read_int(&data, &mut position),
            Some(0x1234_5678)
        );
        assert_eq!(position, 7);
    }

    #[test]
    fn int_keeps_the_java_sign_semantics() {
// highest byte 0xFF => negative value (Java's int is likewise negative)
        let data = [0x00u8, 0x00, 0x00, 0xFF];
        let mut position = 0usize;

        assert_eq!(MPPComponent::read_int(&data, &mut position), Some(-16_777_216));
    }

    #[test]
    fn reads_byte_array_as_a_borrowed_slice() {
        let data = [0x01u8, 0x02, 0x03, 0x04, 0x05];
        let mut position = 1usize;

        assert_eq!(
            MPPComponent::read_byte_array(&data, &mut position, 3),
            Some(&[0x02, 0x03, 0x04][..])
        );
        assert_eq!(position, 4);
    }

    #[test]
    fn reads_past_the_end_return_none_without_moving_the_cursor() {
        let data = [0x01u8, 0x02, 0x03];
        let mut position = 2usize;

// Java throws EOFException in these cases
        assert_eq!(MPPComponent::read_byte(&data, &mut position), Some(0x03));
        assert_eq!(MPPComponent::read_byte(&data, &mut position), None);
        assert_eq!(MPPComponent::read_short(&data, &mut position), None);
        assert_eq!(MPPComponent::read_int(&data, &mut position), None);
        assert_eq!(MPPComponent::read_byte_array(&data, &mut position, 1), None);
        assert_eq!(position, 3, "failed read must not move the cursor");
    }

    #[test]
    fn empty_input_yields_none() {
        let mut position = 0usize;

        assert_eq!(MPPComponent::read_byte(&[], &mut position), None);
        assert_eq!(MPPComponent::read_short(&[], &mut position), None);
        assert_eq!(MPPComponent::read_int(&[], &mut position), None);
// Java: InputStreamHelper.read(is, 0) returns an empty array (does not throw)
        assert_eq!(MPPComponent::read_byte_array(&[], &mut position, 0), Some(&[][..]));
        assert_eq!(MPPComponent::read_byte_array(&[], &mut position, 1), None);
    }
}
