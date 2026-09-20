//! 对应 Java: `org/mpxj/HtmlNotes.java`

use crate::common::html_helper::HtmlHelper;
use crate::notes::Notes;
use std::fmt::{self, Display, Formatter};

/// 以 HTML 格式保存的笔记。
///
/// 对应 Java: `public class HtmlNotes extends Notes`
///
/// # 继承关系如何还原
///
/// Java 中 `HtmlNotes` 直接继承 `Notes`；Rust 无继承，改为独立 struct 实现
/// [`crate::notes::Notes`] trait。Java 通过 `super(HtmlHelper.strip(html))` 存进基类的
/// `m_text` 直接作为本 struct 的字段；`m_html` 亦然。
///
/// 说明：任务描述里曾提到 `HtmlNotes` 继承 `StructuredNotes`，但 Java 源码中
/// `HtmlNotes extends Notes`（与 `StructuredNotes` 无继承关系），本移植以源码为准。
///
/// # 跳过的 Java 方法
///
/// 无。
pub struct HtmlNotes {
    /// 去掉 HTML 标记后的纯文本，供 `Display` 使用。
    ///
    /// 对应 Java: `Notes.m_text`（由 `super(HtmlHelper.strip(html))` 赋值）
    m_text: Option<String>,

    /// 原始 HTML 文档。
    ///
    /// 对应 Java: `private final String m_html;`
    m_html: Option<String>,
}

impl HtmlNotes {
    /// 构造方法。
    ///
    /// 对应 Java: `public HtmlNotes(String html)`
    ///
    /// Java 的 `String` 引用可为 `null`，故入参用 `Option`。
    pub fn new(html: Option<String>) -> HtmlNotes {
        let m_text = HtmlHelper::strip(html.as_deref());

        HtmlNotes { m_text, m_html: html }
    }

    /// 返回原始 HTML 文档。
    ///
    /// 对应 Java: `public String getHtml()`
    pub fn get_html(&self) -> Option<&str> {
        self.m_html.as_deref()
    }
}

impl Display for HtmlNotes {
    /// 返回去掉 HTML 标记后的纯文本。
    ///
    /// 对应 Java: 继承自 `Notes.toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.m_text {
            Some(text) => f.write_str(text),
            None => Ok(()),
        }
    }
}

impl Notes for HtmlNotes {
    /// 对应 Java: 无（Rust 特有的子类型保真拷贝，见 [`Notes::to_box`]）
    fn to_box(&self) -> Box<dyn Notes> {
        Box::new(HtmlNotes {
            m_text: self.m_text.clone(),
            m_html: self.m_html.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_html_is_preserved() {
        let notes = HtmlNotes::new(Some(String::from("plain text")));
        assert_eq!(notes.get_html(), Some("plain text"));
    }

    #[test]
    fn display_uses_stripped_text() {
        let html = String::from("plain text");
        let notes = HtmlNotes::new(Some(html.clone()));
        assert_eq!(
            notes.to_string(),
            HtmlHelper::strip(Some(html.as_str())).unwrap_or_default()
        );
        assert!(!notes.is_empty());
    }

    #[test]
    fn null_html_is_empty() {
        let notes = HtmlNotes::new(None);
        assert_eq!(notes.get_html(), None);
        assert!(notes.is_empty());
    }
}
