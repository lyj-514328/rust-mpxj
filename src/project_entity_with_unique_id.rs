//! 对应 Java: `org/mpxj/ProjectEntityWithUniqueID.java`

/// 具有 unique ID 标识的实体。
///
/// 对应 Java: `public interface ProjectEntityWithUniqueID`
///
/// Java 的接口继承关系在 Rust 中用 trait 的 supertrait 表达：
///
/// ```text
/// ProjectEntityWithID            extends ProjectEntityWithMutableUniqueID
/// ProjectEntityWithMutableUniqueID extends ProjectEntityWithUniqueID
/// ```
///
/// 因此实现 [`crate::project_entity_with_id::ProjectEntityWithID`] 的实体自动满足
/// [`crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID`] 与
/// 本 trait。
///
/// # 与 Java 的差异
///
/// Java 的 `getUniqueID()` 返回可空的 `Integer`，Rust 侧为 `Option<i32>`。
///
/// 被跳过的 Java 机制：无。
///
/// 未移植的 Java 方法：无。
pub trait ProjectEntityWithUniqueID {
    /// 取实体的 unique ID。
    ///
    /// 对应 Java: `Integer getUniqueID()`
    fn get_unique_id(&self) -> Option<i32>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用实体：只实现 unique ID 读取。
    struct Entity {
        unique_id: Option<i32>,
    }

    impl ProjectEntityWithUniqueID for Entity {
        fn get_unique_id(&self) -> Option<i32> {
            self.unique_id
        }
    }

    #[test]
    fn unique_id_is_exposed_as_option() {
        let entity = Entity {
            unique_id: Some(7),
        };
        assert_eq!(entity.get_unique_id(), Some(7));

        let entity = Entity { unique_id: None };
        assert_eq!(entity.get_unique_id(), None);
    }
}
