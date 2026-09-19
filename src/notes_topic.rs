//! 对应 Java: `org/mpxj/NotesTopic.java`
//!
//! # 与 Java 的差异
//!
//! - **接口**：Java 的 `NotesTopic implements ProjectEntityWithUniqueID`，Rust 侧由
//!   `impl crate::project_entity_with_unique_id::ProjectEntityWithUniqueID for NotesTopic`
//!   表达（该 trait 的 `get_unique_id()` 返回 `Option<i32>`）。
//! - **唯一 ID 分配**：Java 构造方法通过
//!   `UniqueIdObjectSequenceProvider#getUniqueIdObjectSequence(NotesTopic.class).syncOrGetNext(...)`
//!   分配/同步唯一 ID。Rust 侧 [`Builder`] 暂不接收序列提供者（与
//!   `crate::shift::Builder`、`crate::unit_of_measure::Builder` 的处理方式一致），
//!   直接使用调用方通过 [`Builder::unique_id`] 传入的值。
//! - **常量 `DEFAULT`**：Java 的 `public static final NotesTopic DEFAULT` 无法在 Rust 中
//!   作为 `const` 构造（含 `String`），改为函数 [`NotesTopic::default_topic`]，
//!   并同时实现 [`Default`] 以便惯用法使用。

use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;

/// 用于组织 P6 笔记的主题。
///
/// 对应 Java: `public final class NotesTopic implements ProjectEntityWithUniqueID`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotesTopic {
    /// 对应 Java 字段 `private final Integer m_uniqueID`
    unique_id: Option<i32>,
    /// 对应 Java 字段 `private final Integer m_sequenceNumber`
    sequence_number: Option<i32>,
    /// 对应 Java 字段 `private final String m_name`
    name: Option<String>,
    /// 对应 Java 字段 `private final boolean m_availableForEPS`
    available_for_eps: bool,
    /// 对应 Java 字段 `private final boolean m_availableForProject`
    available_for_project: bool,
    /// 对应 Java 字段 `private final boolean m_availableForWBS`
    available_for_wbs: bool,
    /// 对应 Java 字段 `private final boolean m_availableForActivity`
    available_for_activity: bool,
}

impl NotesTopic {
    /// 取唯一 ID。
    ///
    /// 对应 Java: `@Override public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.unique_id
    }

    /// 取序号。
    ///
    /// 对应 Java: `public Integer getSequenceNumber()`
    pub fn get_sequence_number(&self) -> Option<i32> {
        self.sequence_number
    }

    /// 取名称。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// 取「可用于 EPS」标志。
    ///
    /// 对应 Java: `public boolean getAvailableForEPS()`
    pub fn get_available_for_eps(&self) -> bool {
        self.available_for_eps
    }

    /// 取「可用于 project」标志。
    ///
    /// 对应 Java: `public boolean getAvailableForProject()`
    pub fn get_available_for_project(&self) -> bool {
        self.available_for_project
    }

    /// 取「可用于 WBS」标志。
    ///
    /// 对应 Java: `public boolean getAvailableForWBS()`
    pub fn get_available_for_wbs(&self) -> bool {
        self.available_for_wbs
    }

    /// 取「可用于 activity」标志。
    ///
    /// 对应 Java: `public boolean getAvailableForActivity()`
    pub fn get_available_for_activity(&self) -> bool {
        self.available_for_activity
    }

    /// 构造 Java 常量 `NotesTopic.DEFAULT` 对应的实例。
    ///
    /// 对应 Java: `public static final NotesTopic DEFAULT`
    ///
    /// Java 用同一实例共享；Rust 侧每次返回新建的值（`NotesTopic` 是值类型）。
    pub fn default_topic() -> NotesTopic {
        Builder::new()
            .unique_id(Some(1))
            .sequence_number(Some(1))
            .name(Some("Notes".to_owned()))
            .available_for_eps(true)
            .available_for_project(true)
            .available_for_wbs(true)
            .available_for_activity(true)
            .build()
    }
}

impl Default for NotesTopic {
    /// 对应 Java: `public static final NotesTopic DEFAULT`
    fn default() -> Self {
        NotesTopic::default_topic()
    }
}

impl ProjectEntityWithUniqueID for NotesTopic {
    /// 取唯一 ID。
    ///
    /// 对应 Java: `@Override public Integer getUniqueID()`
    fn get_unique_id(&self) -> Option<i32> {
        self.unique_id
    }
}

/// `NotesTopic` 构造器。
///
/// 对应 Java: `public static class NotesTopic.Builder`
///
/// Java 的 `Builder` 方法原地修改并返回 `this`；Rust 侧采用与
/// `crate::shift::Builder`、`crate::unit_of_measure::Builder` 一致的
/// 「消耗自身并返回 `Self`」写法。
#[derive(Debug, Clone, Default)]
pub struct Builder {
    /// 对应 Java 字段 `private Integer m_uniqueID`
    unique_id: Option<i32>,
    /// 对应 Java 字段 `private Integer m_sequenceNumber`
    sequence_number: Option<i32>,
    /// 对应 Java 字段 `private String m_name`
    name: Option<String>,
    /// 对应 Java 字段 `private boolean m_availableForEPS`
    available_for_eps: bool,
    /// 对应 Java 字段 `private boolean m_availableForProject`
    available_for_project: bool,
    /// 对应 Java 字段 `private boolean m_availableForWBS`
    available_for_wbs: bool,
    /// 对应 Java 字段 `private boolean m_availableForActivity`
    available_for_activity: bool,
}

impl Builder {
    /// 构造方法。
    ///
    /// 对应 Java: `public Builder(UniqueIdObjectSequenceProvider sequenceProvider)`
    ///
    /// 序列提供者尚未移植，Rust 侧不接收该参数（见文件头说明）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 从已有 `NotesTopic` 初始化各字段。
    ///
    /// 对应 Java: `public Builder from(NotesTopic value)`
    pub fn from(mut self, value: &NotesTopic) -> Self {
        self.unique_id = value.unique_id;
        self.sequence_number = value.sequence_number;
        self.name = value.name.clone();
        self.available_for_eps = value.available_for_eps;
        self.available_for_project = value.available_for_project;
        self.available_for_wbs = value.available_for_wbs;
        self.available_for_activity = value.available_for_activity;
        self
    }

    /// 设置唯一 ID。
    ///
    /// 对应 Java: `public Builder uniqueID(Integer value)`
    pub fn unique_id(mut self, value: Option<i32>) -> Self {
        self.unique_id = value;
        self
    }

    /// 设置序号。
    ///
    /// 对应 Java: `public Builder sequenceNumber(Integer value)`
    pub fn sequence_number(mut self, value: Option<i32>) -> Self {
        self.sequence_number = value;
        self
    }

    /// 设置名称。
    ///
    /// 对应 Java: `public Builder name(String value)`
    pub fn name(mut self, value: Option<String>) -> Self {
        self.name = value;
        self
    }

    /// 设置「可用于 EPS」标志。
    ///
    /// 对应 Java: `public Builder availableForEPS(boolean value)`
    pub fn available_for_eps(mut self, value: bool) -> Self {
        self.available_for_eps = value;
        self
    }

    /// 设置「可用于 project」标志。
    ///
    /// 对应 Java: `public Builder availableForProject(boolean value)`
    pub fn available_for_project(mut self, value: bool) -> Self {
        self.available_for_project = value;
        self
    }

    /// 设置「可用于 WBS」标志。
    ///
    /// 对应 Java: `public Builder availableForWBS(boolean value)`
    pub fn available_for_wbs(mut self, value: bool) -> Self {
        self.available_for_wbs = value;
        self
    }

    /// 设置「可用于 activity」标志。
    ///
    /// 对应 Java: `public Builder availableForActivity(boolean value)`
    pub fn available_for_activity(mut self, value: bool) -> Self {
        self.available_for_activity = value;
        self
    }

    /// 构造 [`NotesTopic`] 实例。
    ///
    /// 对应 Java: `public NotesTopic build()`
    pub fn build(self) -> NotesTopic {
        NotesTopic {
            unique_id: self.unique_id,
            sequence_number: self.sequence_number,
            name: self.name,
            available_for_eps: self.available_for_eps,
            available_for_project: self.available_for_project,
            available_for_wbs: self.available_for_wbs,
            available_for_activity: self.available_for_activity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_sets_all_fields() {
        let topic = Builder::new()
            .unique_id(Some(7))
            .sequence_number(Some(3))
            .name(Some("主题".to_owned()))
            .available_for_eps(true)
            .available_for_project(false)
            .available_for_wbs(true)
            .available_for_activity(false)
            .build();

        assert_eq!(topic.get_unique_id(), Some(7));
        assert_eq!(topic.get_sequence_number(), Some(3));
        assert_eq!(topic.get_name(), Some("主题"));
        assert!(topic.get_available_for_eps());
        assert!(!topic.get_available_for_project());
        assert!(topic.get_available_for_wbs());
        assert!(!topic.get_available_for_activity());
    }

    #[test]
    fn builder_defaults_are_empty() {
        let topic = Builder::new().build();
        assert_eq!(topic.get_unique_id(), None);
        assert_eq!(topic.get_sequence_number(), None);
        assert_eq!(topic.get_name(), None);
        assert!(!topic.get_available_for_eps());
        assert!(!topic.get_available_for_project());
        assert!(!topic.get_available_for_wbs());
        assert!(!topic.get_available_for_activity());
    }

    #[test]
    fn builder_from_copies_existing_instance() {
        let source = Builder::new()
            .unique_id(Some(2))
            .sequence_number(Some(4))
            .name(Some("source".to_owned()))
            .available_for_activity(true)
            .build();

        let copy = Builder::new().from(&source).build();
        assert_eq!(copy, source);

        // Java 的 `from` 只复制字段，不改写原实例
        assert_eq!(source.get_name(), Some("source"));
    }

    #[test]
    fn default_topic_matches_java_default() {
        let topic = NotesTopic::default_topic();
        assert_eq!(topic.get_unique_id(), Some(1));
        assert_eq!(topic.get_sequence_number(), Some(1));
        assert_eq!(topic.get_name(), Some("Notes"));
        assert!(topic.get_available_for_eps());
        assert!(topic.get_available_for_project());
        assert!(topic.get_available_for_wbs());
        assert!(topic.get_available_for_activity());

        assert_eq!(NotesTopic::default(), topic);
    }

    #[test]
    fn implements_project_entity_with_unique_id() {
        fn unique_id_of<T: ProjectEntityWithUniqueID>(value: &T) -> Option<i32> {
            value.get_unique_id()
        }

        assert_eq!(unique_id_of(&NotesTopic::default_topic()), Some(1));
    }
}
