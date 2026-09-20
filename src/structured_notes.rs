//! 对应 Java: `org/mpxj/StructuredNotes.java`

use crate::notes::Notes;
use crate::notes_topic::NotesTopic;
use crate::project_context::ProjectContext;
use std::fmt::{self, Display, Formatter};

/// 属于某个主题（topic）的结构化笔记。
///
/// 对应 Java: `public class StructuredNotes extends Notes`
///
/// # 继承关系如何还原
///
/// Java 中 `StructuredNotes` 直接继承 `Notes`；Rust 无继承，改为独立 struct 实现
/// [`crate::notes::Notes`] trait。Java 构造方法通过 `super(null)` 传给基类的
/// `m_text` 在本类中不参与逻辑（本类重写了 `toString()`），故不再保留；本类自身
/// 的 `m_uniqueID` / `m_topic` / `m_notes` 则原样放进 struct。
///
/// # 与 Java 的差异
///
/// - Java 用 `context.getUniqueIdObjectSequence(StructuredNotes.class)` 取唯一 ID
///   序列；Rust 没有 class 字面量，改为传入类名字符串（与 Java `Class.getName()`
///   作为 map key 的做法一致）。
/// - Java 的 `toString()` 在 `m_topic` 为 `null` 且笔记非空时会抛
///   `NullPointerException`；Rust 不 panic，主题名按空字符串处理。
///
/// # 跳过的 Java 方法
///
/// 无。
pub struct StructuredNotes {
    /// 唯一 ID。
    ///
    /// 对应 Java: `private final Integer m_uniqueID;`
    m_unique_id: Option<i32>,

    /// 笔记所属主题。
    ///
    /// 对应 Java: `private final NotesTopic m_topic;`
    m_topic: Option<NotesTopic>,

    /// 实际笔记内容。
    ///
    /// 对应 Java: `private final Notes m_notes;`
    m_notes: Option<Box<dyn Notes>>,
}

impl StructuredNotes {
    /// 构造方法。
    ///
    /// 对应 Java: `public StructuredNotes(ProjectContext context, Integer uniqueID, NotesTopic topic, Notes notes)`
    ///
    /// `unique_id` 为 `None` 时从上下文的序列里取下一个值；否则同步序列并返回该值
    /// （与 `ObjectSequence.syncOrGetNext` 语义一致）。
    pub fn new(
        context: &mut ProjectContext,
        unique_id: Option<i32>,
        topic: Option<NotesTopic>,
        notes: Option<Box<dyn Notes>>,
    ) -> StructuredNotes {
        let m_unique_id = crate::unique_id_object_sequence_provider::get_unique_id_object_sequence::<
            StructuredNotes,
        >(context)
        .sync_or_get_next(unique_id);

        StructuredNotes {
            m_unique_id: Some(m_unique_id),
            m_topic: topic,
            m_notes: notes,
        }
    }

    /// 返回本笔记的唯一 ID。
    ///
    /// 对应 Java: `public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.m_unique_id
    }

    /// 返回本笔记的主题 ID。
    ///
    /// 对应 Java: `public Integer getTopicID()`
    pub fn get_topic_id(&self) -> Option<i32> {
        self.m_topic
            .as_ref()
            .and_then(|topic| topic.get_unique_id())
    }

    /// 返回本笔记的主题。
    ///
    /// 对应 Java: `public NotesTopic getNotesTopic()`
    pub fn get_notes_topic(&self) -> Option<&NotesTopic> {
        self.m_topic.as_ref()
    }

    /// 返回笔记内容。
    ///
    /// 对应 Java: `public Notes getNotes()`
    pub fn get_notes(&self) -> Option<&dyn Notes> {
        self.m_notes.as_deref()
    }
}

impl Display for StructuredNotes {
    /// 返回「主题名 + 换行 + 笔记正文 + 换行」。
    ///
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 在正文为 `null` 或空串时返回 `null`；Rust 的 `Display` 无法表达 `null`，
    /// 此时输出空字符串（`is_empty` 的判定结果与 Java 保持一致）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Some(text) = self.m_notes.as_ref().map(|notes| notes.to_string()) else {
            return Ok(());
        };

        if text.is_empty() {
            return Ok(());
        }

        let name = self
            .m_topic
            .as_ref()
            .and_then(|topic| topic.get_name())
            .unwrap_or("");

        write!(f, "{}\n{}\n", name, text)
    }
}

impl Notes for StructuredNotes {
    /// 对应 Java: 无（Rust 特有的子类型保真拷贝，见 [`Notes::to_box`]）
    fn to_box(&self) -> Box<dyn Notes> {
        Box::new(StructuredNotes {
            m_unique_id: self.m_unique_id,
            m_topic: self.m_topic.clone(),
            m_notes: self.m_notes.as_ref().map(|notes| notes.to_box()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::PlainNotes;

    fn body(text: &str) -> Option<Box<dyn Notes>> {
        Some(Box::new(PlainNotes::new(Some(String::from(text)))))
    }

    #[test]
    fn unique_id_comes_from_the_context_sequence() {
        let mut context = ProjectContext::default();
        let notes = StructuredNotes::new(&mut context, None, None, body("body"));
        assert_eq!(notes.get_unique_id(), Some(1));

        let notes = StructuredNotes::new(&mut context, None, None, body("body"));
        assert_eq!(notes.get_unique_id(), Some(2));
    }

    #[test]
    fn supplied_unique_id_is_preserved_and_syncs_sequence() {
        let mut context = ProjectContext::default();
        let notes = StructuredNotes::new(&mut context, Some(42), None, body("body"));
        assert_eq!(notes.get_unique_id(), Some(42));

        // 序列被同步到 42 之后，下一个自动 ID 应为 43。
        let notes = StructuredNotes::new(&mut context, None, None, body("body"));
        assert_eq!(notes.get_unique_id(), Some(43));
    }

    #[test]
    fn accessors_reflect_constructor_arguments() {
        let mut context = ProjectContext::default();
        let notes = StructuredNotes::new(&mut context, None, None, body("body"));
        assert!(notes.get_notes_topic().is_none());
        assert_eq!(notes.get_topic_id(), None);
        assert_eq!(
            notes.get_notes().map(|notes| notes.to_string()),
            Some(String::from("body"))
        );
    }

    #[test]
    fn empty_body_renders_nothing() {
        let mut context = ProjectContext::default();
        let notes = StructuredNotes::new(&mut context, None, None, None);
        assert_eq!(notes.to_string(), "");
        assert!(notes.is_empty());

        let notes = StructuredNotes::new(&mut context, None, None, body(""));
        assert_eq!(notes.to_string(), "");
        assert!(notes.is_empty());
    }

    #[test]
    fn non_empty_body_without_topic_renders_body_only() {
        let mut context = ProjectContext::default();
        let notes = StructuredNotes::new(&mut context, None, None, body("body"));
        // Java 在 topic 为 null 时会抛 NPE，Rust 侧不 panic，主题名按空串处理。
        assert_eq!(notes.to_string(), "\nbody\n");
        assert!(!notes.is_empty());
    }
}
