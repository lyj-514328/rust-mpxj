//! 对应 Java: `org/mpxj/mpp/FontStyle.java`
//!
//! 在 [`crate::mpp::font_base::FontBase`]（字体名 + 字号）之上再叠加颜色、粗体、斜体、
//! 下划线等属性，用来表示 MPP 文件 `STYLE_DATA` 等数据块中的字体样式。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `public class FontStyle` | 结构体 [`FontStyle`] | 1:1 移植；字段顺序与 Java 一致 |
//! | 构造参数 `Color color` / `Color backgroundColor`（`java.awt.Color`） | `Option<Color>`，`Color` = [`crate::mpp::mpp_utility::Color`] | `java.awt.*` 属跳过项；Rust 侧统一复用 [`crate::mpp::mpp_utility::Color`]（`MPPUtility.get_color` 的返回值类型）。Java 的 `MPPUtility.getColor` 与 `ColorType.getColor()` 都可能返回 `null`（自动颜色），构造点也直接传 `null`，故用 `Option` |
//! | 构造参数 `FontBase fontBase` | `Option<FontBase>` | Java 调用点写成 `fontBases.get(index)`，字体表中没有该序号时会传入 `null`（见 `GanttChartView.getFontStyle`、`GroupReader`） |
//! | 构造参数 `BackgroundPattern backgroundPattern` | [`crate::mpp::background_pattern::BackgroundPattern`]（非 `Option`） | Java 的 4 处构造点都传入确定值（`BackgroundPattern.SOLID` 或 `BackgroundPattern.getInstance(...)`，后者不会返回 `null`） |
//! | `toString()` | `Display` | 输出格式与 Java 一致；`java.awt.Color` 部分按 `java.awt.Color[r=..,g=..,b=..]` 形式输出，`null` 输出 `null` |
//! | 未覆写 `equals` / `hashCode` | 派生 `PartialEq` / `Eq` | Rust 侧为便于比较与测试而派生（逐字段比较），语义比 Java 的引用相等更宽；因 [`crate::mpp::mpp_utility::Color`] 未实现 `Hash`，故**不**派生 `Hash` |
//! | 类名与模型层可能存在的同名类型 | 本类型位于 `crate::mpp::font_style::FontStyle` | 已核对本仓 Java 源码：`org.mpxj` 根包中**没有**同名类，因此这里不存在与模型层 `crate::FontStyle` 的冲突；模块路径不同，两者互不影响 |
//!
//! 未移植 / 跳过：无（JDBC / XML-JAXB / GUI / `javax.*` / `Serializable` 等机制本类未使用）。

use std::fmt::{self, Display, Formatter};

use crate::mpp::background_pattern::BackgroundPattern;
use crate::mpp::font_base::FontBase;
use crate::mpp::mpp_utility::{format_color, Color};

/// 字体样式（基础字体 + 颜色 / 字型属性）。
///
/// 对应 Java: `public class FontStyle`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontStyle {
    /// 基础字体。
    ///
    /// 对应 Java: `private final FontBase m_fontBase`
    font_base: Option<FontBase>,
    /// 斜体标志。
    ///
    /// 对应 Java: `private final boolean m_italic`
    italic: bool,
    /// 粗体标志。
    ///
    /// 对应 Java: `private final boolean m_bold`
    bold: bool,
    /// 下划线标志。
    ///
    /// 对应 Java: `private final boolean m_underline`
    underline: bool,
    /// 删除线标志。
    ///
    /// 对应 Java: `private final boolean m_strikethrough`
    strikethrough: bool,
    /// 字色。
    ///
    /// 对应 Java: `private final Color m_color`
    color: Option<Color>,
    /// 背景色。
    ///
    /// 对应 Java: `private final Color m_backgroundColor`
    background_color: Option<Color>,
    /// 背景图案。
    ///
    /// 对应 Java: `private final BackgroundPattern m_backgroundPattern`
    background_pattern: BackgroundPattern,
}

impl FontStyle {
    /// 构造函数。
    ///
    /// 对应 Java:
    /// `public FontStyle(FontBase fontBase, boolean italic, boolean bold, boolean underline, boolean strikethrough, Color color, Color backgroundColor, BackgroundPattern backgroundPattern)`
    pub fn new(
        font_base: Option<FontBase>,
        italic: bool,
        bold: bool,
        underline: bool,
        strikethrough: bool,
        color: Option<Color>,
        background_color: Option<Color>,
        background_pattern: BackgroundPattern,
    ) -> FontStyle {
        FontStyle {
            font_base,
            italic,
            bold,
            underline,
            strikethrough,
            color,
            background_color,
            background_pattern,
        }
    }

    /// 取基础字体。
    ///
    /// 对应 Java: `public FontBase getFontBase()`
    pub fn get_font_base(&self) -> Option<&FontBase> {
        self.font_base.as_ref()
    }

    /// 取粗体标志。
    ///
    /// 对应 Java: `public boolean getBold()`
    pub fn get_bold(&self) -> bool {
        self.bold
    }

    /// 取字色。
    ///
    /// 对应 Java: `public Color getColor()`
    pub fn get_color(&self) -> Option<Color> {
        self.color
    }

    /// 取斜体标志。
    ///
    /// 对应 Java: `public boolean getItalic()`
    pub fn get_italic(&self) -> bool {
        self.italic
    }

    /// 取下划线标志。
    ///
    /// 对应 Java: `public boolean getUnderline()`
    pub fn get_underline(&self) -> bool {
        self.underline
    }

    /// 取删除线标志。
    ///
    /// 对应 Java: `public boolean getStrikethrough()`
    pub fn get_strikethrough(&self) -> bool {
        self.strikethrough
    }

    /// 取背景色。
    ///
    /// 对应 Java: `public Color getBackgroundColor()`
    pub fn get_background_color(&self) -> Option<Color> {
        self.background_color
    }

    /// 取背景图案。
    ///
    /// 对应 Java: `public BackgroundPattern getBackgroundPattern()`
    pub fn get_background_pattern(&self) -> BackgroundPattern {
        self.background_pattern
    }
}

impl Display for FontStyle {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// 逐字段拼接，顺序与 Java 完全一致。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[FontStyle fontBase={} italic={} bold={} underline={} strikethrough={} color={} backgroundColor={} backgroundPattern={}]",
            format_optional_display(self.font_base.as_ref()),
            self.italic,
            self.bold,
            self.underline,
            self.strikethrough,
            format_color(self.color),
            format_color(self.background_color),
            self.background_pattern
        )
    }
}

/// 把可选对象按 Java 的字符串拼接语义输出（`None` 输出 `null`）。
fn format_optional_display<T: Display>(value: Option<&T>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个用于测试的样式。
    fn sample() -> FontStyle {
        FontStyle::new(
            Some(FontBase::new(Some(1), "Arial".to_owned(), 10)),
            true,
            false,
            true,
            false,
            Some(Color::new(255, 0, 0)),
            Some(Color::new(0, 0, 255)),
            BackgroundPattern::Solid,
        )
    }

    /// getter 与 Java 一致（含字段顺序无关的取值）。
    #[test]
    fn accessors_match_java() {
        let style = sample();
        assert_eq!(
            style.get_font_base().map(FontBase::get_name),
            Some("Arial")
        );
        assert!(style.get_italic());
        assert!(!style.get_bold());
        assert!(style.get_underline());
        assert!(!style.get_strikethrough());
        assert_eq!(style.get_color(), Some(Color::new(255, 0, 0)));
        assert_eq!(style.get_background_color(), Some(Color::new(0, 0, 255)));
        assert_eq!(style.get_background_pattern(), BackgroundPattern::Solid);
    }

    /// Java 中可为 null 的三个参数在 Rust 侧是 `Option`。
    #[test]
    fn nullable_members_are_options() {
        let style = FontStyle::new(
            None,
            false,
            false,
            false,
            false,
            None,
            None,
            BackgroundPattern::Transparent,
        );
        assert_eq!(style.get_font_base(), None);
        assert_eq!(style.get_color(), None);
        assert_eq!(style.get_background_color(), None);
    }

    /// toString 格式与 Java 一致（含 `null` 分支）。
    #[test]
    fn to_string_matches_java() {
        let style = sample();
        assert_eq!(
            style.to_string(),
            "[FontStyle fontBase=[FontBase name=Arial size=10] italic=true bold=false underline=true strikethrough=false color=java.awt.Color[r=255,g=0,b=0] backgroundColor=java.awt.Color[r=0,g=0,b=255] backgroundPattern=Solid]"
        );

        let style = FontStyle::new(
            None,
            false,
            true,
            false,
            false,
            None,
            None,
            BackgroundPattern::Transparent,
        );
        assert_eq!(
            style.to_string(),
            "[FontStyle fontBase=null italic=false bold=true underline=false strikethrough=false color=null backgroundColor=null backgroundPattern=Transparent]"
        );
    }

    /// 派生 `PartialEq` 逐字段比较（Java 侧只有引用相等）。
    #[test]
    fn equality_is_field_wise() {
        assert_eq!(sample(), sample());
        assert_ne!(
            sample(),
            FontStyle::new(
                None,
                false,
                false,
                false,
                false,
                None,
                None,
                BackgroundPattern::Solid
            )
        );
    }
}
