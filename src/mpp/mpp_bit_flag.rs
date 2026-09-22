//! Corresponds to Java: `org/mpxj/mpp/MppBitFlag.java`

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::field_container::FieldContainer;
use crate::field_key::FieldKey;
use crate::field_value::FieldValue;

/// Represents the field type and position of a bit flag within a block of data.
///
/// Corresponds to Java: `class MppBitFlag` (package-private)
///
/// # Porting notes (trade-offs)
///
/// - Java's `FieldType type` is carried by [`FieldKey`] on the Rust side — `FieldKey` is
/// the concrete form of the `FieldType` interface in Rust (see the `field_key` module header comment).
/// - Java's `Object zeroValue` / `Object nonZeroValue` are carried on the Rust side by
/// [`FieldValue`] wrapped in `Option`: Java allows passing `null`, in which case `set` clears the field.
/// - Java's `setValue(FieldContainer, byte[])` first checks `data != null`; per project translation convention
/// a `byte[]` parameter is uniformly `&[u8]` (cannot be null), so the null check branch is no longer needed.
/// - In Java `ByteArrayHelper.getInt(data, m_offset)` throws
/// `ArrayIndexOutOfBoundsException` when `m_offset` is out of bounds; the Rust side treats out-of-bounds as "the expression is 0" (i.e. taking
/// `zeroValue`) and documents it, without panicking.
pub struct MppBitFlag {
    /// Corresponds to Java field `m_type`
    m_type: FieldKey,
    /// Corresponds to Java field `m_offset`
    m_offset: usize,
    /// Corresponds to Java field `m_mask`
    m_mask: i32,
    /// Corresponds to Java field `m_zeroValue`
    m_zero_value: Option<FieldValue>,
    /// Corresponds to Java field `m_nonZeroValue`
    m_non_zero_value: Option<FieldValue>,
}

impl MppBitFlag {
/// Constructor.
    ///
    /// Corresponds to Java: `public MppBitFlag(FieldType type, int offset, int mask, Object zeroValue, Object nonZeroValue)`
    pub fn new(
        field_type: FieldKey,
        offset: usize,
        mask: i32,
        zero_value: Option<FieldValue>,
        non_zero_value: Option<FieldValue>,
    ) -> Self {
        Self {
            m_type: field_type,
            m_offset: offset,
            m_mask: mask,
            m_zero_value: zero_value,
            m_non_zero_value: non_zero_value,
        }
    }

/// Read this bit flag's value from the given byte array and write it to the given field container.
    ///
    /// Corresponds to Java: `public void setValue(FieldContainer container, byte[] data)`
    pub fn set_value(&self, container: &mut dyn FieldContainer, data: &[u8]) {
        // Java: `(ByteArrayHelper.getInt(data, m_offset) & m_mask) == 0`
// Java throws ArrayIndexOutOfBoundsException when out of bounds; the Rust side takes 0 (equivalent to hitting the zeroValue branch).
        let expression = if self.m_offset + 4 <= data.len() {
            ByteArrayHelper::get_int(data, self.m_offset) & self.m_mask
        } else {
            0
        };

        let value = if expression == 0 {
            self.m_zero_value.clone()
        } else {
            self.m_non_zero_value.clone()
        };

        container.set(self.m_type, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::listener::field_listener::FieldListener;
    use crate::task_field::TaskField;
    use std::collections::HashMap;

/// Field container used for testing.
    #[derive(Default)]
    struct MockContainer {
        values: HashMap<FieldKey, FieldValue>,
    }

    impl FieldContainer for MockContainer {
        fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
            match value {
                Some(value) => self.values.insert(field, value),
                None => self.values.remove(&field),
            };
        }

        fn get(&self, field: FieldKey) -> Option<FieldValue> {
            self.values.get(&field).cloned()
        }

        fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
            self.values.get(&field)
        }

        fn add_field_listener(&mut self, _listener: Box<dyn FieldListener>) {}

        fn clear_field_listeners(&mut self) {}
    }

/// Construct a bit flag that "returns true when set and false when clear".
    fn flag(offset: usize, mask: i32) -> MppBitFlag {
        MppBitFlag::new(
            FieldKey::Task(TaskField::Milestone),
            offset,
            mask,
            Some(FieldValue::Boolean(false)),
            Some(FieldValue::Boolean(true)),
        )
    }

    #[test]
    fn set_value_extracts_a_set_bit() {
// Little-endian read of the 4 bytes at offset 0 gives 0x20
        let data = [0x20u8, 0x00, 0x00, 0x00];
        let mut container = MockContainer::default();
        flag(0, 0x20).set_value(&mut container, &data);

        assert_eq!(
            container.get(FieldKey::Task(TaskField::Milestone)),
            Some(FieldValue::Boolean(true))
        );
    }

    #[test]
    fn set_value_returns_zero_value_when_masked_out() {
// The 0x20 bit is clear, the 0x40 bit is set
        let data = [0x40u8, 0x00, 0x00, 0x00];
        let mut container = MockContainer::default();
        flag(0, 0x20).set_value(&mut container, &data);

        assert_eq!(
            container.get(FieldKey::Task(TaskField::Milestone)),
            Some(FieldValue::Boolean(false))
        );
    }

    #[test]
    fn set_value_reads_the_requested_offset() {
// The 4 bytes starting at offset 1 are 0x00000020
        let data = [0x00u8, 0x20, 0x00, 0x00, 0x00];
        let mut container = MockContainer::default();
        flag(1, 0x20).set_value(&mut container, &data);

        assert_eq!(
            container.get(FieldKey::Task(TaskField::Milestone)),
            Some(FieldValue::Boolean(true))
        );
    }

    #[test]
    fn set_value_does_not_panic_when_offset_is_out_of_range() {
// Java throws ArrayIndexOutOfBoundsException here; the Rust side falls back to zeroValue
        let data = [0x01u8, 0x02];
        let mut container = MockContainer::default();
        flag(5, 0x20).set_value(&mut container, &data);

        assert_eq!(
            container.get(FieldKey::Task(TaskField::Milestone)),
            Some(FieldValue::Boolean(false))
        );
    }

    #[test]
    fn null_values_remove_the_field() {
        let data = [0x20u8, 0x00, 0x00, 0x00];
        let mut container = MockContainer::default();
        let flag = MppBitFlag::new(
            FieldKey::Task(TaskField::Milestone),
            0,
            0x20,
            None,
            None,
        );
        flag.set_value(&mut container, &data);

// Java's `container.set(type, null)` clears the field
        assert_eq!(container.get(FieldKey::Task(TaskField::Milestone)), None);
    }
}
