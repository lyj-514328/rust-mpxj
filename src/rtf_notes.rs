//! 对应 Java: `org/mpxj/RtfNotes.java`

use crate::common::rtf_helper::RtfHelper;
use crate::notes::Notes;
use std::fmt::{self, Display, Formatter};

/// 以 RTF 格式保存的笔记。
///
/// 对应 Java: `public class RtfNotes extends Notes`
///
/// # 继承关系如何还原
///
/// Java 中 `RtfNotes` 直接继承 `Notes`；Rust 无继承，改为独立 struct 实现
/// [`crate::notes::Notes`] trait。Java 通过 `super(RtfHelper.strip(rtf))` 存进基类的
/// `m_text` 直接作为本 struct 的字段；`m_rtf` 亦然。
///
/// 说明：任务描述里曾提到 `RtfNotes` 继承 `StructuredNotes`，但 Java 源码中
/// `RtfNotes extends Notes`（与 `StructuredNotes` 无继承关系），本移植以源码为准。
///
/// # 跳过的 Java 方法
///
/// 无。
pub struct RtfNotes {
    /// 去掉 RTF 标记后的纯文本，供 `Display` 使用。
    ///
    /// 对应 Java: `Notes.m_text`（由 `super(RtfHelper.strip(rtf))` 赋值）
    m_text: Option<String>,

    /// 原始 RTF 文档。
    ///
    /// 对应 Java: `private final String m_rtf;`
    m_rtf: Option<String>,
}

impl RtfNotes {
    /// 构造方法。
    ///
    /// 对应 Java: `public RtfNotes(String rtf)`
    ///
    /// Java 的 `String` 引用可为 `null`，故入参用 `Option`。
    pub fn new(rtf: Option<String>) -> RtfNotes {
        let m_text = RtfHelper::strip(rtf.as_deref());

        RtfNotes { m_text, m_rtf: rtf }
    }

    /// 返回原始 RTF 文档。
    ///
    /// 对应 Java: `public String getRtf()`
    pub fn get_rtf(&self) -> Option<&str> {
        self.m_rtf.as_deref()
    }
}

impl Display for RtfNotes {
    /// 返回去掉 RTF 标记后的纯文本。
    ///
    /// 对应 Java: 继承自 `Notes.toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.m_text {
            Some(text) => f.write_str(text),
            None => Ok(()),
        }
    }
}

impl Notes for RtfNotes {
    /// 对应 Java: 无（Rust 特有的子类型保真拷贝，见 [`Notes::to_box`]）
    fn to_box(&self) -> Box<dyn Notes> {
        Box::new(RtfNotes {
            m_text: self.m_text.clone(),
            m_rtf: self.m_rtf.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_rtf_is_preserved() {
        let notes = RtfNotes::new(Some(String::from("plain text")));
        assert_eq!(notes.get_rtf(), Some("plain text"));
    }

    #[test]
    fn display_uses_stripped_text() {
        let rtf = String::from("plain text");
        let notes = RtfNotes::new(Some(rtf.clone()));
        assert_eq!(notes.to_string(), RtfHelper::strip(Some(rtf.as_str())).unwrap_or_default());
        assert!(!notes.is_empty());
    }

    #[test]
    fn null_rtf_is_empty() {
        let notes = RtfNotes::new(None);
        assert_eq!(notes.get_rtf(), None);
        assert!(notes.is_empty());
    }
}
