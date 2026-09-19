//! 对应 Java: `org/mpxj/ProjectEntityWithMutableUniqueID.java`

use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;

/// 具有**可变** unique ID 的实体。
///
/// 对应 Java: `public interface ProjectEntityWithMutableUniqueID extends ProjectEntityWithUniqueID`
///
/// Rust 侧的接口继承用 supertrait 表达；实现本 trait 的类型必须同时实现
/// [`ProjectEntityWithUniqueID`]。
///
/// # 与 Java 的差异
///
/// Java 的 `setUniqueID(Integer id)` 接受可空值，Rust 侧为 `Option<i32>`，传 `None`
/// 等价于 Java 传 `null`。
///
/// 被跳过的 Java 机制：无。
///
/// 未移植的 Java 方法：无。
pub trait ProjectEntityWithMutableUniqueID: ProjectEntityWithUniqueID {
    /// 设置实体的 unique ID。
    ///
    /// 对应 Java: `void setUniqueID(Integer id)`
    fn set_unique_id(&mut self, id: Option<i32>);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用实体：unique ID 可读可写。
    struct Entity {
        unique_id: Option<i32>,
    }

    impl ProjectEntityWithUniqueID for Entity {
        fn get_unique_id(&self) -> Option<i32> {
            self.unique_id
        }
    }

    impl ProjectEntityWithMutableUniqueID for Entity {
        fn set_unique_id(&mut self, id: Option<i32>) {
            self.unique_id = id;
        }
    }

    #[test]
    fn unique_id_can_be_set_and_cleared() {
        let mut entity = Entity { unique_id: None };

        entity.set_unique_id(Some(3));
        assert_eq!(entity.get_unique_id(), Some(3));

        entity.set_unique_id(None);
        assert_eq!(entity.get_unique_id(), None);
    }
}
