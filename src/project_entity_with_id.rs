//! 对应 Java: `org/mpxj/ProjectEntityWithID.java`

use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;

/// 可以用 ID（区别于 unique ID）标识的实体。
///
/// 对应 Java: `public interface ProjectEntityWithID extends ProjectEntityWithMutableUniqueID`
///
/// Rust 侧的接口继承用 supertrait 表达；实现本 trait 的类型必须同时实现
/// [`ProjectEntityWithMutableUniqueID`]（进而实现
/// [`crate::project_entity_with_unique_id::ProjectEntityWithUniqueID`]）。
///
/// # 与 Java 的差异
///
/// Java 的 `getID()` / `setID(Integer id)` 接受可空值，Rust 侧为 `Option<i32>`。
///
/// 被跳过的 Java 机制：无。
///
/// 未移植的 Java 方法：无。
pub trait ProjectEntityWithID: ProjectEntityWithMutableUniqueID {
    /// 取实体的 ID。
    ///
    /// 对应 Java: `Integer getID()`
    fn get_id(&self) -> Option<i32>;

    /// 设置实体的 ID。
    ///
    /// 对应 Java: `void setID(Integer id)`
    fn set_id(&mut self, id: Option<i32>);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;

    /// 测试用实体：同时具备 ID 与 unique ID。
    struct Entity {
        id: Option<i32>,
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

    impl ProjectEntityWithID for Entity {
        fn get_id(&self) -> Option<i32> {
            self.id
        }

        fn set_id(&mut self, id: Option<i32>) {
            self.id = id;
        }
    }

    #[test]
    fn id_and_unique_id_are_independent() {
        let mut entity = Entity {
            id: Some(1),
            unique_id: Some(10),
        };

        entity.set_id(Some(2));
        entity.set_unique_id(Some(20));

        assert_eq!(entity.get_id(), Some(2));
        assert_eq!(entity.get_unique_id(), Some(20));

        entity.set_id(None);
        assert_eq!(entity.get_id(), None);
    }
}
