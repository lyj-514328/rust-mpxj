//! 对应 Java: `org/mpxj/Notes.java`

use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

/// 纯文本笔记的公共特征，调用 `Display`（等价于 Java 的 `toString()`）会得到
/// 该笔记的纯文本表示。
///
/// 对应 Java: `public class Notes`
///
/// # 继承关系如何还原
///
/// Java 里 `Notes` 是一个可实例化的基类，`ParentNotes` / `StructuredNotes` /
/// `RtfNotes` / `HtmlNotes` 都 `extends Notes`。Rust 没有继承，本移植把基类
/// `Notes` 还原为 `pub trait Notes`：各子类分别是独立 struct 并各自实现本 trait；
/// Java 中由基类字段承载的状态（`m_text`）直接放进各 struct。
///
/// Java 中 `new Notes(text)` 直接实例化基类表示纯文本笔记；基类被还原为 trait 后，
/// 该形态由 [`PlainNotes`] 承载，语义与 Java 的 `Notes` 完全一致。
///
/// # 跳过的 Java 方法
///
/// 无。
///
/// # 与 Java 的差异
///
/// Java 的字段表按引用共享 `Notes` 对象（`Task.setNotesObject(Notes)` 直接存入同一个
/// 实例），没有「复制」这一步。Rust 侧 [`crate::field_value::FieldValue::Notes`] 需要持有
/// `Rc<dyn Notes>`，而既有调用点交出的是 `&dyn Notes`（如 `mpp8_reader` 的
/// `task.set_notes_object(Some(&rtf_notes))`），故由各子类型实现 [`Notes::to_box`]
/// 提供一次**子类型保真**的浅拷贝（[`Notes::to_rc`] 基于它给出共享句柄）。
pub trait Notes: Display {
    /// 判断笔记文本是否为空。
    ///
    /// 对应 Java: `public boolean isEmpty()`
    ///
    /// Java 的实现是 `String text = toString(); return text == null || text.isEmpty();`。
    /// Rust 的 `Display` 不可能返回 `null`，因此只按「输出是否为空字符串」判断。
    fn is_empty(&self) -> bool {
        self.to_string().is_empty()
    }

    /// 复制为新的 `Box<dyn Notes>`（Rust 特有方法）。
    ///
    /// 对应 Java: 无（见 trait 文档的「与 Java 的差异」）。
    ///
    /// 各子类型需保证返回的对象与自身**具体子类型一致**，而不是退化后的纯文本，
    /// 否则 `RtfNotes` / `HtmlNotes` / `StructuredNotes` 的子类型信息会丢失。
    fn to_box(&self) -> Box<dyn Notes>;

    /// 复制为共享句柄 `Rc<dyn Notes>`（Rust 特有方法）。
    ///
    /// 对应 Java: 无（见 trait 文档的「与 Java 的差异」）。
    ///
    /// 默认实现基于 [`Notes::to_box`]，子类无需重复实现。
    fn to_rc(&self) -> Rc<dyn Notes> {
        Rc::from(self.to_box())
    }
}

/// 纯文本笔记。
///
/// 对应 Java: `public class Notes`（基类被直接实例化时的具体形态）
///
/// 说明：Java 的基类 `Notes` 本身既可被当作基类，也可通过 `new Notes(text)` 直接
/// 使用。Rust 侧 trait 无法被实例化，故用本类型承载「纯文本笔记」这一具体形态。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlainNotes {
    /// 笔记文本。
    ///
    /// 对应 Java: `private final String m_text;`
    m_text: Option<String>,
}

impl PlainNotes {
    /// 构造方法。
    ///
    /// 对应 Java: `public Notes(String text)`
    ///
    /// Java 允许传入 `null`（`ParentNotes` / `StructuredNotes` 就通过 `super(null)`
    /// 传入），故此处用 `Option` 表示。
    pub fn new(text: Option<String>) -> PlainNotes {
        PlainNotes { m_text: text }
    }
}

impl Display for PlainNotes {
    /// 返回纯文本内容。
    ///
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 在 `m_text` 为 `null` 时返回 `null`；Rust 的 `Display` 无法表达 `null`，
    /// 此处在 `None` 时输出空字符串。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.m_text {
            Some(text) => f.write_str(text),
            None => Ok(()),
        }
    }
}

impl Notes for PlainNotes {
    /// 对应 Java: 无（Rust 特有的子类型保真拷贝，见 [`Notes::to_box`]）
    fn to_box(&self) -> Box<dyn Notes> {
        Box::new(PlainNotes::new(self.m_text.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_notes_returns_text() {
        let notes = PlainNotes::new(Some(String::from("hello")));
        assert_eq!(notes.to_string(), "hello");
        assert!(!notes.is_empty());
    }

    #[test]
    fn empty_and_null_text_are_empty() {
        assert!(PlainNotes::new(Some(String::new())).is_empty());
        assert!(PlainNotes::new(None).is_empty());
        assert_eq!(PlainNotes::new(None).to_string(), "");
    }

    #[test]
    fn notes_trait_object_supports_empty_check() {
        let notes: Box<dyn Notes> = Box::new(PlainNotes::new(Some(String::from("x"))));
        assert!(!notes.is_empty());
        assert_eq!(notes.to_string(), "x");
    }
}
