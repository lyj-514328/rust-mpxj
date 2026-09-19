//! 对应 Java: `org/mpxj/UniqueIdObjectSequenceProvider.java`

use std::any::TypeId;
use std::rc::Rc;

use crate::common::object_sequence::ObjectSequence;

/// 按类型提供 unique ID 序号生成器的接口。
///
/// 对应 Java: `public interface UniqueIdObjectSequenceProvider`
///
/// # 与 Java 的差异：`Class<?>` 键改为 `std::any::TypeId`
///
/// Java 侧以 `Class<?>` 作键（`ProjectContext` 内部实际用 `c.getName()` 字符串作键），
/// Rust 没有 class 字面量，改用 [`TypeId`]：
///
/// | Java | Rust |
/// |---|---|
/// | `provider.getUniqueIdObjectSequence(Task.class)` | `provider.get_unique_id_object_sequence(TypeId::of::<Task>())` |
/// | `provider.getUniqueIdObjectSequence(Task.class)`（泛型写法） | [`get_unique_id_object_sequence`] |
///
/// 之所以把 trait 方法定义为「接收 `TypeId` 参数」而不是泛型方法，是为了让本 trait
/// 仍然可以作 trait object 使用（容器持有
/// `Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>`）；泛型方法会破坏对象安全性。
/// 需要按类型书写时用 [`get_unique_id_object_sequence`] 这个泛型辅助函数。
///
/// # 与 Java 的差异：返回值用 `Rc<ObjectSequence>`
///
/// Java 侧每次返回**同一个** `ObjectSequence` 实例（`ProjectContext` 用
/// `computeIfAbsent` 缓存）。Rust 侧用 `Rc<ObjectSequence>` 表达这种共享；因为
/// [`ObjectSequence`] 内部用 `Cell<i32>`，共享时不需要额外的 `RefCell`。
pub trait UniqueIdObjectSequenceProvider {
    /// 取（必要时创建）指定类型的序号生成器。
    ///
    /// 对应 Java: `ObjectSequence getUniqueIdObjectSequence(Class<?> c)`
    ///
    /// Java 侧实现会在内部缓存新创建的 `ObjectSequence`，因此 Rust 侧接收 `&mut self`。
    fn get_unique_id_object_sequence(&mut self, type_id: TypeId) -> Rc<ObjectSequence>;
}

/// 按类型取序号生成器的泛型入口，对应 Java 的
/// `provider.getUniqueIdObjectSequence(T.class)`。
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

    /// 测试用实现：按 `TypeId` 缓存序号生成器，行为与 Java 的 `ProjectContext` 一致。
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

    /// 测试用「实体」类型，仅用于区分 `TypeId`。
    struct Alpha;

    /// 测试用「实体」类型，仅用于区分 `TypeId`。
    struct Beta;

    #[test]
    fn sequences_are_shared_for_the_same_type() {
        let mut provider = TestProvider::default();

        let alpha = get_unique_id_object_sequence::<Alpha>(&mut provider);
        let alpha_again = get_unique_id_object_sequence::<Alpha>(&mut provider);
        let beta = get_unique_id_object_sequence::<Beta>(&mut provider);

        // 同一类型共享同一个序列
        assert!(Rc::ptr_eq(&alpha, &alpha_again));
        assert_eq!(alpha.get_next(), 1);
        assert_eq!(alpha_again.get_next(), 2);

        // 不同类型各自独立
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
