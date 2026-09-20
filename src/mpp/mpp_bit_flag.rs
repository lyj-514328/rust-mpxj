//! 对应 Java: `org/mpxj/mpp/MppBitFlag.java`

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::field_container::FieldContainer;
use crate::field_key::FieldKey;
use crate::field_value::FieldValue;

/// 表示一块数据中某个位标志的字段类型与位置。
///
/// 对应 Java: `class MppBitFlag`（包级可见）
///
/// # 移植说明（取舍）
///
/// - Java 的 `FieldType type` 在 Rust 侧用 [`FieldKey`] 承载 —— `FieldKey` 就是
///   `FieldType` 接口在 Rust 里的具体化形式（见 `field_key` 模块头注释）。
/// - Java 的 `Object zeroValue` / `Object nonZeroValue` 在 Rust 侧用
///   [`FieldValue`] 承载并包成 `Option`：Java 允许传 `null`，此时 `set` 会清除该字段。
/// - Java 的 `setValue(FieldContainer, byte[])` 先判断 `data != null`；按项目翻译约定
///   `byte[]` 入参统一为 `&[u8]`（不可能为 null），因此该判空分支不再需要。
/// - Java 中 `ByteArrayHelper.getInt(data, m_offset)` 在 `m_offset` 越界时会抛
///   `ArrayIndexOutOfBoundsException`；Rust 侧把越界视为「表达式为 0」（即取
///   `zeroValue`）并注释，不 panic。
pub struct MppBitFlag {
    /// 对应 Java 字段 `m_type`
    m_type: FieldKey,
    /// 对应 Java 字段 `m_offset`
    m_offset: usize,
    /// 对应 Java 字段 `m_mask`
    m_mask: i32,
    /// 对应 Java 字段 `m_zeroValue`
    m_zero_value: Option<FieldValue>,
    /// 对应 Java 字段 `m_nonZeroValue`
    m_non_zero_value: Option<FieldValue>,
}

impl MppBitFlag {
    /// 构造方法。
    ///
    /// 对应 Java: `public MppBitFlag(FieldType type, int offset, int mask, Object zeroValue, Object nonZeroValue)`
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

    /// 从给定的字节数组中取出本位标志的值，并写入给定的字段容器。
    ///
    /// 对应 Java: `public void setValue(FieldContainer container, byte[] data)`
    pub fn set_value(&self, container: &mut dyn FieldContainer, data: &[u8]) {
        // Java: `(ByteArrayHelper.getInt(data, m_offset) & m_mask) == 0`
        // 越界时 Java 抛 ArrayIndexOutOfBoundsException，Rust 侧取 0（等价于命中 zeroValue 分支）。
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

    /// 测试用的字段容器。
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

    /// 构造一个「置位返回 true、清零返回 false」的位标志。
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
        // 小端读取偏移 0 处的 4 字节为 0x20
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
        // 0x20 位未置位，0x40 位置位
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
        // 偏移 1 起的 4 字节为 0x00000020
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
        // Java 侧此处抛 ArrayIndexOutOfBoundsException，Rust 侧回落为 zeroValue
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

        // Java 的 `container.set(type, null)` 即清除该字段
        assert_eq!(container.get(FieldKey::Task(TaskField::Milestone)), None);
    }
}
