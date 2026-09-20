//! 对应 Java: `org/mpxj/mpp/TableFontStyle.java`
//!
//! 在 [`FontStyle`]（基础字体 + 颜色 / 字型）之上再附加「表列字体样式」的信息：所属行、
//! 所属列字段，以及各项属性「是否被显式修改」的标志位。
//!
//! # 继承的还原（组合 + 转发）
//!
//! Java 的 `public class TableFontStyle extends FontStyle`。Rust 没有继承，故：
//!
//! - 本类**持有一个** [`FontStyle`]（私有字段 `font_style`），Java 继承来的 8 个 getter
//!   （`getFontBase` / `getBold` / `getColor` / `getItalic` / `getUnderline` /
//!   `getStrikethrough` / `getBackgroundColor` / `getBackgroundPattern`）逐一**转发**给该字段；
//! - Java 的 `super(fontBase, italic, bold, underline, strikethrough, color, backgroundColor,
//!   backgroundPattern)` 在 [`TableFontStyle::new`] 中对应一次 [`FontStyle::new`] 调用；
//! - 本类新增的字段（行 unique ID、列字段、8 个 changed 标志）作为本类自己的字段。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `Color color` / `Color backgroundColor`（`java.awt.Color`） | `Option<Color>`，`Color` = [`crate::mpp::mpp_utility::Color`] | 与 [`FontStyle`] 的取舍一致（`java.awt.*` 属跳过项；`MPPUtility.getColor` 与 `ColorType.getColor()` 都可能返回 `null`） |
//! | `FontBase fontBase` | `Option<FontBase>` | Java 调用点写成 `fontBases.get(index)`，字体表缺该序号时为 `null` |
//! | `FieldType fieldType` | `Option<MppFieldType>` | 本 crate 用 [`MppFieldType`] 表达 `FieldType`；字段号未识别时为 `None`（Java 的占位对象无法表达） |
//! | `BackgroundPattern backgroundPattern` | [`BackgroundPattern`]（非 `Option`） | Java 的调用点都传入确定值（`SOLID` / `TRANSPARENT` / `getInstance(...)`，后者不会返回 `null`） |
//! | `toString()` | [`Display`] | 逐段拼接顺序与 Java 一致；`java.awt.Color` 按 `java.awt.Color[r=..,g=..,b=..]` 输出，`null` 输出 `null` |
//! | 未覆写 `equals` / `hashCode` | 仅派生 `Debug` / `Clone` | `MppFieldType` 只实现了 `Clone`（未实现 `PartialEq`），因此本类无法派生 `PartialEq` / `Eq` / `Hash`（Java 侧本来也只有引用相等） |
//!
//! 跳过的 Java 成员：无。
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::fmt::{self, Display, Formatter};

use crate::common::mpp_field_type::MppFieldType;
use crate::mpp::background_pattern::BackgroundPattern;
use crate::mpp::font_base::FontBase;
use crate::mpp::font_style::FontStyle;
use crate::mpp::mpp_utility::{format_optional_display, Color};

/// 表格列 / 行 / 单元格的字体样式。
///
/// 对应 Java: `public class TableFontStyle extends FontStyle`
///
/// 说明：`MppFieldType` 未实现 `Debug`，故本类不派生 `Debug`，而是手工实现（复用
/// [`Display`]，与 `crate::group` / `crate::table` 等模型的既有做法一致）。
#[derive(Clone)]
pub struct TableFontStyle {
    /// 组合的基类实例（对应 Java 的 `extends FontStyle`）。
    font_style: FontStyle,

    /// 受影响行所属实体的 unique ID；`-1` 表示作用于所有行。
    ///
    /// 对应 Java: `private final int m_rowUniqueID;`
    row_unique_id: i32,

    /// 样式作用的表格列（字段）。
    ///
    /// 对应 Java: `private final FieldType m_fieldType;`
    field_type: Option<MppFieldType>,

    /// 斜体是否被显式修改。
    ///
    /// 对应 Java: `private final boolean m_italicChanged;`
    italic_changed: bool,

    /// 粗体是否被显式修改。
    ///
    /// 对应 Java: `private final boolean m_boldChanged;`
    bold_changed: bool,

    /// 下划线是否被显式修改。
    ///
    /// 对应 Java: `private final boolean m_underlineChanged;`
    underline_changed: bool,

    /// 删除线是否被显式修改。
    ///
    /// 对应 Java: `private final boolean m_strikethroughChanged;`
    strikethrough_changed: bool,

    /// 字色是否被显式修改。
    ///
    /// 对应 Java: `private final boolean m_colorChanged;`
    color_changed: bool,

    /// 字体是否被显式修改。
    ///
    /// 对应 Java: `private final boolean m_fontChanged;`
    font_changed: bool,

    /// 背景色是否被显式修改。
    ///
    /// 对应 Java: `private final boolean m_backgroundColorChanged;`
    background_color_changed: bool,

    /// 背景图案是否被显式修改。
    ///
    /// 对应 Java: `private final boolean m_backgroundPatternChanged;`
    background_pattern_changed: bool,
}

impl TableFontStyle {
    /// 构造方法。
    ///
    /// 对应 Java:
    /// `public TableFontStyle(int rowUniqueID, FieldType fieldType, FontBase fontBase, boolean italic,
    /// boolean bold, boolean underline, boolean strikethrough, Color color, Color backgroundColor,
    /// BackgroundPattern backgroundPattern, boolean italicChanged, boolean boldChanged,
    /// boolean underlineChanged, boolean strikethroughChanged, boolean colorChanged,
    /// boolean fontChanged, boolean backgroundColorChanged, boolean backgroundPatternChanged)`
    ///
    /// 参数顺序与 Java 完全一致（`italic` 在 `bold` 之前）。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        row_unique_id: i32,
        field_type: Option<MppFieldType>,
        font_base: Option<FontBase>,
        italic: bool,
        bold: bool,
        underline: bool,
        strikethrough: bool,
        color: Option<Color>,
        background_color: Option<Color>,
        background_pattern: BackgroundPattern,
        italic_changed: bool,
        bold_changed: bool,
        underline_changed: bool,
        strikethrough_changed: bool,
        color_changed: bool,
        font_changed: bool,
        background_color_changed: bool,
        background_pattern_changed: bool,
    ) -> Self {
        // Java: super(fontBase, italic, bold, underline, strikethrough, color, backgroundColor, backgroundPattern);
        let font_style = FontStyle::new(
            font_base,
            italic,
            bold,
            underline,
            strikethrough,
            color,
            background_color,
            background_pattern,
        );

        Self {
            font_style,
            row_unique_id,
            field_type,
            italic_changed,
            bold_changed,
            underline_changed,
            strikethrough_changed,
            color_changed,
            font_changed,
            background_color_changed,
            background_pattern_changed,
        }
    }

    /// 取受影响行的 unique ID（`-1` 表示作用于所有行）。
    ///
    /// 对应 Java: `public int getRowUniqueID()`
    pub fn get_row_unique_id(&self) -> i32 {
        self.row_unique_id
    }

    /// 取本样式作用的表格列（字段）。
    ///
    /// 对应 Java: `public FieldType getFieldType()`
    pub fn get_field_type(&self) -> Option<&MppFieldType> {
        self.field_type.as_ref()
    }

    /// 取粗体是否被显式修改。
    ///
    /// 对应 Java: `public boolean getBoldChanged()`
    pub fn get_bold_changed(&self) -> bool {
        self.bold_changed
    }

    /// 取字色是否被显式修改。
    ///
    /// 对应 Java: `public boolean getColorChanged()`
    pub fn get_color_changed(&self) -> bool {
        self.color_changed
    }

    /// 取斜体是否被显式修改。
    ///
    /// 对应 Java: `public boolean getItalicChanged()`
    pub fn get_italic_changed(&self) -> bool {
        self.italic_changed
    }

    /// 取下划线是否被显式修改。
    ///
    /// 对应 Java: `public boolean getUnderlineChanged()`
    pub fn get_underline_changed(&self) -> bool {
        self.underline_changed
    }

    /// 取删除线是否被显式修改。
    ///
    /// 对应 Java: `public boolean getStrikethroughChanged()`
    pub fn get_strikethrough_changed(&self) -> bool {
        self.strikethrough_changed
    }

    /// 取字体是否被显式修改。
    ///
    /// 对应 Java: `public boolean getFontChanged()`
    pub fn get_font_changed(&self) -> bool {
        self.font_changed
    }

    /// 取背景色是否被显式修改。
    ///
    /// 对应 Java: `public boolean getBackgroundColorChanged()`
    pub fn get_background_color_changed(&self) -> bool {
        self.background_color_changed
    }

    /// 取背景图案是否被显式修改。
    ///
    /// 对应 Java: `public boolean getBackgroundPatternChanged()`
    pub fn get_background_pattern_changed(&self) -> bool {
        self.background_pattern_changed
    }

    // -----------------------------------------------------------------
    // 以下为继承自 FontStyle 的成员（组合 + 转发）
    // -----------------------------------------------------------------

    /// 取基础字体。
    ///
    /// 对应 Java: `FontStyle.getFontBase()`
    pub fn get_font_base(&self) -> Option<&FontBase> {
        self.font_style.get_font_base()
    }

    /// 取粗体标志。
    ///
    /// 对应 Java: `FontStyle.getBold()`
    pub fn get_bold(&self) -> bool {
        self.font_style.get_bold()
    }

    /// 取字色。
    ///
    /// 对应 Java: `FontStyle.getColor()`
    pub fn get_color(&self) -> Option<Color> {
        self.font_style.get_color()
    }

    /// 取斜体标志。
    ///
    /// 对应 Java: `FontStyle.getItalic()`
    pub fn get_italic(&self) -> bool {
        self.font_style.get_italic()
    }

    /// 取下划线标志。
    ///
    /// 对应 Java: `FontStyle.getUnderline()`
    pub fn get_underline(&self) -> bool {
        self.font_style.get_underline()
    }

    /// 取删除线标志。
    ///
    /// 对应 Java: `FontStyle.getStrikethrough()`
    pub fn get_strikethrough(&self) -> bool {
        self.font_style.get_strikethrough()
    }

    /// 取背景色。
    ///
    /// 对应 Java: `FontStyle.getBackgroundColor()`
    pub fn get_background_color(&self) -> Option<Color> {
        self.font_style.get_background_color()
    }

    /// 取背景图案。
    ///
    /// 对应 Java: `FontStyle.getBackgroundPattern()`
    pub fn get_background_pattern(&self) -> BackgroundPattern {
        self.font_style.get_background_pattern()
    }

    /// 以只读方式取组合的基类实例（Rust 特有，便于整体转发 / 比较）。
    pub fn get_font_style(&self) -> &FontStyle {
        &self.font_style
    }
}

impl Display for TableFontStyle {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 只输出「被显式修改」的那些属性（其余段落省略）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut result = format!(
            "[ColumnFontStyle rowUniqueID={} fieldType={}",
            self.row_unique_id,
            format_optional_display(self.field_type.as_ref())
        );

        if self.italic_changed {
            result.push_str(&format!(" italic={}", self.get_italic()));
        }
        if self.bold_changed {
            result.push_str(&format!(" bold={}", self.get_bold()));
        }
        if self.underline_changed {
            result.push_str(&format!(" underline={}", self.get_underline()));
        }
        if self.strikethrough_changed {
            result.push_str(&format!(" strikethrough={}", self.get_strikethrough()));
        }
        if self.font_changed {
            result.push_str(&format!(
                " font={}",
                format_optional_display(self.get_font_base())
            ));
        }
        if self.color_changed {
            result.push_str(&format!(
                " color={}",
                format_optional_display(self.get_color().as_ref())
            ));
        }
        if self.background_color_changed {
            result.push_str(&format!(
                " backgroundColor={}",
                format_optional_display(self.get_background_color().as_ref())
            ));
        }
        if self.background_pattern_changed {
            result.push_str(&format!(
                " backgroundPattern={}",
                self.get_background_pattern()
            ));
        }

        result.push(']');
        f.write_str(&result)
    }
}

impl fmt::Debug for TableFontStyle {
    /// Java 未覆写调试输出；Rust 侧提供 `Debug`（复用 [`Display`]，因 `MppFieldType`
    /// 未实现 `Debug` 而无法派生）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_key::FieldKey;
    use crate::task_field::TaskField;

    /// 构造一个完整的样式。
    fn sample() -> TableFontStyle {
        TableFontStyle::new(
            42,
            Some(MppFieldType::Builtin(FieldKey::Task(TaskField::Name))),
            Some(FontBase::new(Some(1), "Arial".to_owned(), 10)),
            true,
            false,
            true,
            false,
            Some(Color::new(255, 0, 0)),
            Some(Color::new(0, 0, 255)),
            BackgroundPattern::Solid,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
        )
    }

    /// 自有 getter 与 Java 一致。
    #[test]
    fn own_accessors_match_java() {
        let style = sample();
        assert_eq!(style.get_row_unique_id(), 42);
        assert_eq!(
            style.get_field_type().map(|field| field.get_name()),
            Some("Task Name".to_owned())
        );
        assert!(style.get_bold_changed());
        assert!(style.get_color_changed());
        assert!(style.get_italic_changed());
        assert!(style.get_underline_changed());
        assert!(style.get_strikethrough_changed());
        assert!(style.get_font_changed());
        assert!(style.get_background_color_changed());
        assert!(style.get_background_pattern_changed());
    }

    /// 继承自 `FontStyle` 的 getter 被转发。
    #[test]
    fn inherited_accessors_are_forwarded() {
        let style = sample();
        assert_eq!(style.get_font_base().map(FontBase::get_name), Some("Arial"));
        assert!(style.get_italic());
        assert!(!style.get_bold());
        assert!(style.get_underline());
        assert!(!style.get_strikethrough());
        assert_eq!(style.get_color(), Some(Color::new(255, 0, 0)));
        assert_eq!(style.get_background_color(), Some(Color::new(0, 0, 255)));
        assert_eq!(style.get_background_pattern(), BackgroundPattern::Solid);

        // 组合的基类实例与转发结果一致
        assert_eq!(style.get_font_style().get_color(), style.get_color());
    }

    /// `toString` 只输出被修改的属性；未修改时只有前两段。
    #[test]
    fn display_matches_java_format() {
        let style = sample();
        assert_eq!(
            style.to_string(),
            "[ColumnFontStyle rowUniqueID=42 fieldType=Task Name italic=true bold=false underline=true strikethrough=false font=[FontBase name=Arial size=10] color=java.awt.Color[r=255,g=0,b=0] backgroundColor=java.awt.Color[r=0,g=0,b=255] backgroundPattern=Solid]"
        );

        let style = TableFontStyle::new(
            -1,
            None,
            None,
            false,
            false,
            false,
            false,
            None,
            None,
            BackgroundPattern::Transparent,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
        );
        assert_eq!(
            style.to_string(),
            "[ColumnFontStyle rowUniqueID=-1 fieldType=null]"
        );
    }

    /// 颜色 / 字体为 `null` 时按 Java 的拼接语义输出。
    #[test]
    fn display_renders_nulls() {
        let style = TableFontStyle::new(
            1,
            None,
            None,
            false,
            false,
            false,
            false,
            None,
            None,
            BackgroundPattern::Solid,
            false,
            false,
            false,
            false,
            true,
            true,
            false,
            false,
        );
        assert_eq!(
            style.to_string(),
            "[ColumnFontStyle rowUniqueID=1 fieldType=null font=null color=null]"
        );
    }
}
