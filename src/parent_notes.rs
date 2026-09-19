//! 对应 Java: `org/mpxj/ParentNotes.java`

use crate::notes::Notes;
use std::fmt::{self, Display, Formatter};

/// 由一个或多个子笔记组合而成的笔记。
///
/// 对应 Java: `public class ParentNotes extends Notes`
///
/// # 继承关系如何还原
///
/// Java 中 `ParentNotes` 直接继承 `Notes`；Rust 无继承，改为独立 struct 实现
/// [`crate::notes::Notes`] trait。Java 构造方法通过 `super(null)` 传给基类的
/// `m_text` 在本类中不参与任何逻辑（本类重写了 `toString()`），故不再保留该字段。
///
/// # 跳过的 Java 方法
///
/// 无。
pub struct ParentNotes {
    /// 子笔记列表。
    ///
    /// 对应 Java: `private final List<Notes> m_childNotes;`
    m_child_notes: Vec<Box<dyn Notes>>,
}

impl ParentNotes {
    /// 构造方法。
    ///
    /// 对应 Java: `public ParentNotes(List<Notes> childNotes)`
    pub fn new(child_notes: Vec<Box<dyn Notes>>) -> ParentNotes {
        ParentNotes {
            m_child_notes: child_notes,
        }
    }

    /// 返回子笔记列表。
    ///
    /// 对应 Java: `public List<Notes> getChildNotes()`
    pub fn get_child_notes(&self) -> &[Box<dyn Notes>] {
        &self.m_child_notes
    }
}

impl Display for ParentNotes {
    /// 返回所有子笔记纯文本表示拼接后的结果。
    ///
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 先过滤掉 `null` 元素，再逐项 `toString()`，用 `\n` 连接后调用 `trim()`。
    /// Rust 的 `Vec<Box<dyn Notes>>` 中不存在 `null` 元素，故只需连接与去空白。
    ///
    /// 另注：Java 的 `String.trim()` 只去掉码点 `<= U+0020` 的首尾字符，Rust 的
    /// `str::trim()` 会去掉所有 Unicode 空白字符，极端输入下结果略有差异。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let text = self
            .m_child_notes
            .iter()
            .map(|notes| notes.to_string())
            .collect::<Vec<String>>()
            .join("\n");

        f.write_str(text.trim())
    }
}

impl Notes for ParentNotes {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::PlainNotes;

    fn plain(text: &str) -> Box<dyn Notes> {
        Box::new(PlainNotes::new(Some(String::from(text))))
    }

    #[test]
    fn child_notes_are_joined_with_newlines() {
        let notes = ParentNotes::new(vec![plain("a"), plain("b")]);
        assert_eq!(notes.get_child_notes().len(), 2);
        assert_eq!(notes.to_string(), "a\nb");
        assert!(!notes.is_empty());
    }

    #[test]
    fn empty_parent_notes_are_empty() {
        let notes = ParentNotes::new(Vec::new());
        assert_eq!(notes.to_string(), "");
        assert!(notes.is_empty());
    }

    #[test]
    fn surrounding_whitespace_is_trimmed() {
        let notes = ParentNotes::new(vec![plain("  a  "), plain("b")]);
        assert_eq!(notes.to_string(), "a  \nb");
    }
}
