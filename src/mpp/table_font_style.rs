//! Corresponds to Java: `org/mpxj/mpp/TableFontStyle.java`
//!
//! On top of [`FontStyle`] (base font + color / font style), this adds "table column font style" information: the owning row,
//! the owning column field, and flags for whether each attribute has been explicitly modified.
//!
//! # How inheritance is restored (composition + forwarding)
//!
//! Java's `public class TableFontStyle extends FontStyle`. Rust has no inheritance, so:
//!
//! - This class **holds a** [`FontStyle`] (private field `font_style`); the 8 getters inherited in Java
//!   (`getFontBase` / `getBold` / `getColor` / `getItalic` / `getUnderline` /
//! `getStrikethrough` / `getBackgroundColor` / `getBackgroundPattern`) are **forwarded** to that field one by one;
//! - Java's `super(fontBase, italic, bold, underline, strikethrough, color, backgroundColor,
//! backgroundPattern)` corresponds to one [`FontStyle::new`] call in [`TableFontStyle::new`];
//! - The fields newly added by this class (row unique ID, column field, 8 changed flags) are the class's own fields.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `Color color` / `Color backgroundColor` (`java.awt.Color`) | `Option<Color>`, `Color` = [`crate::mpp::mpp_utility::Color`] | Consistent with the trade-off in [`FontStyle`] (`java.awt.*` is a skipped item; both `MPPUtility.getColor` and `ColorType.getColor()` may return `null`) |
//! | `FontBase fontBase` | `Option<FontBase>` | The Java call site is written `fontBases.get(index)`; it is `null` when the font table lacks that index |
//! | `FieldType fieldType` | `Option<MppFieldType>` | This crate uses [`MppFieldType`] to represent `FieldType`; it is `None` when the field number is unrecognized (Java's placeholder object cannot express this) |
//! | `BackgroundPattern backgroundPattern` | [`BackgroundPattern`] (not `Option`) | All Java call sites pass a definite value (`SOLID` / `TRANSPARENT` / `getInstance(...)`, the latter never returns `null`) |
//! | `toString()` | [`Display`] | The segment-by-segment concatenation order matches Java; `java.awt.Color` is output as `java.awt.Color[r=..,g=..,b=..]`, and `null` outputs `null` |
//! | `equals` / `hashCode` not overridden | Only derive `Debug` / `Clone` | `MppFieldType` only implements `Clone` (not `PartialEq`), so this class cannot derive `PartialEq` / `Eq` / `Hash` (Java only has reference equality anyway) |
//!
//! Skipped Java members: none.
//!
//! Reflection / JDBC / XML (JAXB) / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::fmt::{self, Display, Formatter};

use crate::common::mpp_field_type::MppFieldType;
use crate::mpp::background_pattern::BackgroundPattern;
use crate::mpp::font_base::FontBase;
use crate::mpp::font_style::FontStyle;
use crate::mpp::mpp_utility::{format_optional_display, Color};

/// Font style of a table column / row / cell.
///
/// Corresponds to Java: `public class TableFontStyle extends FontStyle`
///
/// Note: `MppFieldType` does not implement `Debug`, so this class does not derive `Debug` but implements it manually (reusing
/// [`Display`], consistent with the existing approach of models such as `crate::group` / `crate::table`).
#[derive(Clone)]
pub struct TableFontStyle {
/// The composed base class instance (corresponding to Java's `extends FontStyle`).
    font_style: FontStyle,

/// Unique ID of the entity the affected row belongs to; `-1` means it applies to all rows.
    ///
    /// Corresponds to Java: `private final int m_rowUniqueID;`
    row_unique_id: i32,

/// The table column (field) the style applies to.
    ///
    /// Corresponds to Java: `private final FieldType m_fieldType;`
    field_type: Option<MppFieldType>,

/// Whether italic has been explicitly modified.
    ///
    /// Corresponds to Java: `private final boolean m_italicChanged;`
    italic_changed: bool,

/// Whether bold has been explicitly modified.
    ///
    /// Corresponds to Java: `private final boolean m_boldChanged;`
    bold_changed: bool,

/// Whether underline has been explicitly modified.
    ///
    /// Corresponds to Java: `private final boolean m_underlineChanged;`
    underline_changed: bool,

/// Whether strikethrough has been explicitly modified.
    ///
    /// Corresponds to Java: `private final boolean m_strikethroughChanged;`
    strikethrough_changed: bool,

/// Whether the font color has been explicitly modified.
    ///
    /// Corresponds to Java: `private final boolean m_colorChanged;`
    color_changed: bool,

/// Whether the font has been explicitly modified.
    ///
    /// Corresponds to Java: `private final boolean m_fontChanged;`
    font_changed: bool,

/// Whether the background color has been explicitly modified.
    ///
    /// Corresponds to Java: `private final boolean m_backgroundColorChanged;`
    background_color_changed: bool,

/// Whether the background pattern has been explicitly modified.
    ///
    /// Corresponds to Java: `private final boolean m_backgroundPatternChanged;`
    background_pattern_changed: bool,
}

impl TableFontStyle {
/// Constructor.
    ///
    /// Corresponds to Java:
    /// `public TableFontStyle(int rowUniqueID, FieldType fieldType, FontBase fontBase, boolean italic,
    /// boolean bold, boolean underline, boolean strikethrough, Color color, Color backgroundColor,
    /// BackgroundPattern backgroundPattern, boolean italicChanged, boolean boldChanged,
    /// boolean underlineChanged, boolean strikethroughChanged, boolean colorChanged,
    /// boolean fontChanged, boolean backgroundColorChanged, boolean backgroundPatternChanged)`
    ///
/// The parameter order is exactly the same as Java (`italic` before `bold`).
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

/// Returns the unique ID of the affected row (`-1` means it applies to all rows).
    ///
    /// Corresponds to Java: `public int getRowUniqueID()`
    pub fn get_row_unique_id(&self) -> i32 {
        self.row_unique_id
    }

/// Returns the table column (field) this style applies to.
    ///
    /// Corresponds to Java: `public FieldType getFieldType()`
    pub fn get_field_type(&self) -> Option<&MppFieldType> {
        self.field_type.as_ref()
    }

/// Returns whether bold has been explicitly modified.
    ///
    /// Corresponds to Java: `public boolean getBoldChanged()`
    pub fn get_bold_changed(&self) -> bool {
        self.bold_changed
    }

/// Returns whether the font color has been explicitly modified.
    ///
    /// Corresponds to Java: `public boolean getColorChanged()`
    pub fn get_color_changed(&self) -> bool {
        self.color_changed
    }

/// Returns whether italic has been explicitly modified.
    ///
    /// Corresponds to Java: `public boolean getItalicChanged()`
    pub fn get_italic_changed(&self) -> bool {
        self.italic_changed
    }

/// Returns whether underline has been explicitly modified.
    ///
    /// Corresponds to Java: `public boolean getUnderlineChanged()`
    pub fn get_underline_changed(&self) -> bool {
        self.underline_changed
    }

/// Returns whether strikethrough has been explicitly modified.
    ///
    /// Corresponds to Java: `public boolean getStrikethroughChanged()`
    pub fn get_strikethrough_changed(&self) -> bool {
        self.strikethrough_changed
    }

/// Returns whether the font has been explicitly modified.
    ///
    /// Corresponds to Java: `public boolean getFontChanged()`
    pub fn get_font_changed(&self) -> bool {
        self.font_changed
    }

/// Returns whether the background color has been explicitly modified.
    ///
    /// Corresponds to Java: `public boolean getBackgroundColorChanged()`
    pub fn get_background_color_changed(&self) -> bool {
        self.background_color_changed
    }

/// Returns whether the background pattern has been explicitly modified.
    ///
    /// Corresponds to Java: `public boolean getBackgroundPatternChanged()`
    pub fn get_background_pattern_changed(&self) -> bool {
        self.background_pattern_changed
    }

    // -----------------------------------------------------------------
// The following are members inherited from FontStyle (composition + forwarding)
    // -----------------------------------------------------------------

/// Returns the base font.
    ///
    /// Corresponds to Java: `FontStyle.getFontBase()`
    pub fn get_font_base(&self) -> Option<&FontBase> {
        self.font_style.get_font_base()
    }

/// Returns the bold flag.
    ///
    /// Corresponds to Java: `FontStyle.getBold()`
    pub fn get_bold(&self) -> bool {
        self.font_style.get_bold()
    }

/// Returns the font color.
    ///
    /// Corresponds to Java: `FontStyle.getColor()`
    pub fn get_color(&self) -> Option<Color> {
        self.font_style.get_color()
    }

/// Returns the italic flag.
    ///
    /// Corresponds to Java: `FontStyle.getItalic()`
    pub fn get_italic(&self) -> bool {
        self.font_style.get_italic()
    }

/// Returns the underline flag.
    ///
    /// Corresponds to Java: `FontStyle.getUnderline()`
    pub fn get_underline(&self) -> bool {
        self.font_style.get_underline()
    }

/// Returns the strikethrough flag.
    ///
    /// Corresponds to Java: `FontStyle.getStrikethrough()`
    pub fn get_strikethrough(&self) -> bool {
        self.font_style.get_strikethrough()
    }

/// Returns the background color.
    ///
    /// Corresponds to Java: `FontStyle.getBackgroundColor()`
    pub fn get_background_color(&self) -> Option<Color> {
        self.font_style.get_background_color()
    }

/// Returns the background pattern.
    ///
    /// Corresponds to Java: `FontStyle.getBackgroundPattern()`
    pub fn get_background_pattern(&self) -> BackgroundPattern {
        self.font_style.get_background_pattern()
    }

/// Returns the composed base class instance by shared reference (Rust-specific, convenient for wholesale forwarding / comparison).
    pub fn get_font_style(&self) -> &FontStyle {
        &self.font_style
    }
}

impl Display for TableFontStyle {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java outputs only those attributes that have been explicitly modified (the other segments are omitted).
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
/// Java does not override debug output; the Rust side provides `Debug` (reusing [`Display`], because `MppFieldType`
/// does not implement `Debug` and so it cannot be derived).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_key::FieldKey;
    use crate::task_field::TaskField;

/// Builds a complete style.
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

/// The class's own getters match Java.
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

/// The getters inherited from `FontStyle` are forwarded.
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

// The composed base class instance is consistent with the forwarded results
        assert_eq!(style.get_font_style().get_color(), style.get_color());
    }

/// `toString` outputs only the modified attributes; when nothing is modified there are only the first two segments.
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

/// When the color / font is `null`, output follows Java's concatenation semantics.
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
