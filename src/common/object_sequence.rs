//! 对应 Java: `org/mpxj/common/ObjectSequence.java`

use std::cell::Cell;
use std::fmt::{self, Debug, Formatter};

/// 简单的序号生成器。
///
/// 对应 Java: `public final class ObjectSequence`
///
/// # 与 Java 的差异：可变字段改为 `Cell<i32>`
///
/// Java 侧 `m_id` 是可变的 `int` 字段，`sync` / `getNext` 通过实例方法就地修改。
/// Rust 侧改用 `Cell<i32>`，所有方法接收 `&self`，从而可以放进 `Rc<ObjectSequence>`
/// 在容器与实体之间共享（见
/// [`crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider`]），
/// 无需再套一层 `RefCell`。
///
/// ## 被跳过的 Java 代码
///
/// 无。
///
/// 未移植的 Java 方法：无。
pub struct ObjectSequence {
    /// 下一个待分配的 ID。
    ///
    /// 对应 Java: `private int m_id;`
    m_id: Cell<i32>,
}

impl ObjectSequence {
    /// 构造方法。
    ///
    /// 对应 Java: `public ObjectSequence(int id)`
    pub fn new(id: i32) -> Self {
        Self {
            m_id: Cell::new(id),
        }
    }

    /// 与一个「已知在用」的值同步：如果该值不小于当前序号，则把序号推进到它的下一位。
    ///
    /// 对应 Java: `public void sync(Integer id)`
    ///
    /// Java 侧 `id` 为 `null` 时不做任何事；Rust 侧用 `Option<i32>` 表达。
    pub fn sync(&self, id: Option<i32>) {
        if let Some(id) = id {
            if id >= self.m_id.get() {
                self.m_id.set(id + 1);
            }
        }
    }

    /// 取序列中的下一个值。
    ///
    /// 对应 Java: `public Integer getNext()`
    pub fn get_next(&self) -> i32 {
        let result = self.m_id.get();
        self.m_id.set(result + 1);
        result
    }

    /// 若 `id` 非 `null` 则与它同步；否则生成一个新值。
    ///
    /// 对应 Java: `public Integer syncOrGetNext(Integer id)`
    pub fn sync_or_get_next(&self, id: Option<i32>) -> i32 {
        match id {
            Some(id) => {
                self.sync(Some(id));
                id
            }
            None => self.get_next(),
        }
    }
}

impl Debug for ObjectSequence {
    /// Java 侧 `ObjectSequence` 未覆写 `toString()`；Rust 侧为便于测试与诊断，
    /// 输出下一个待分配的值。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObjectSequence")
            .field("m_id", &self.m_id.get())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_next_increments_from_initial_value() {
        let sequence = ObjectSequence::new(1);

        assert_eq!(sequence.get_next(), 1);
        assert_eq!(sequence.get_next(), 2);
        assert_eq!(sequence.get_next(), 3);
    }

    #[test]
    fn sync_moves_sequence_forward_only() {
        let sequence = ObjectSequence::new(1);

        // 小于当前值：不生效
        sequence.sync(Some(0));
        assert_eq!(sequence.get_next(), 1);

        // 大于等于当前值：推进到其下一位
        sequence.sync(Some(10));
        assert_eq!(sequence.get_next(), 11);

        // 相等：同样推进
        sequence.sync(Some(11));
        assert_eq!(sequence.get_next(), 12);

        // null：不生效
        sequence.sync(None);
        assert_eq!(sequence.get_next(), 13);
    }

    #[test]
    fn sync_or_get_next_matches_java() {
        let sequence = ObjectSequence::new(1);

        // id 非空：同步并原样返回
        assert_eq!(sequence.sync_or_get_next(Some(4)), 4);
        assert_eq!(sequence.get_next(), 5);

        // id 为空：生成新值
        assert_eq!(sequence.sync_or_get_next(None), 6);
        assert_eq!(sequence.sync_or_get_next(None), 7);
    }
}
