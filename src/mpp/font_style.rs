//! Corresponds to Java: `org/mpxj/mpp/FontStyle.java`
//!
//! On top of [`crate::mpp::font_base::FontBase`] (font name + size), this adds color, bold, italic,
//! underline, and other attributes, representing the font style in MPP file data blocks such as `STYLE_DATA`.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `public class FontStyle` | struct [`FontStyle`] | 1:1 port; field order matches Java |
//! | constructor parameter `Color color` / `Color backgroundColor` (`java.awt.Color`) | `Option<Color>`, `Color` = [`crate::mpp::mpp_utility::Color`] | `java.awt.*` is a skipped item; the Rust side uniformly reuses [`crate::mpp::mpp_utility::Color`] (the return type of `MPPUtility.get_color`). Java's `MPPUtility.getColor` and `ColorType.getColor()` can both return `null` (automatic color), and the construction sites also pass `null` directly, hence `Option` |
//! | constructor parameter `FontBase fontBase` | `Option<FontBase>` | Java call sites are written as `fontBases.get(index)`; `null` is passed when the font table has no such index (see `GanttChartView.getFontStyle`, `GroupReader`) |
//! | constructor parameter `BackgroundPattern backgroundPattern` | [`crate::mpp::background_pattern::BackgroundPattern`] (not `Option`) | all 4 Java construction sites pass a definite value (`BackgroundPattern.SOLID` or `BackgroundPattern.getInstance(...)`, the latter never returns `null`) |
//! | `toString()` | `Display` | output format matches Java; the `java.awt.Color` part is output in the form `java.awt.Color[r=..,g=..,b=..]`, and `null` outputs `null` |
//! | does not override `equals` / `hashCode` | derives `PartialEq` / `Eq` | the Rust side derives these for ease of comparison and testing (field-by-field comparison), a broader semantics than Java's reference equality; because [`crate::mpp::mpp_utility::Color`] does not implement `Hash`, `Hash` is **not** derived |
//! | a same-named type that may exist in the model layer | this type lives in `crate::mpp::font_style::FontStyle` | the Java source in this repo was checked: there is **no** same-named class in the `org.mpxj` root package, so there is no conflict with the model layer's `crate::FontStyle`; the module paths differ, so the two do not affect each other |
//!
//! Unported / skipped: none (mechanisms such as JDBC / XML-JAXB / GUI / `javax.*` / `Serializable` are not used by this class).

use std::fmt::{self, Display, Formatter};

use crate::mpp::background_pattern::BackgroundPattern;
use crate::mpp::font_base::FontBase;
use crate::mpp::mpp_utility::{format_color, format_optional_display, Color};

/// Font style (base font + color / typeface attributes).
///
/// Corresponds to Java: `public class FontStyle`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontStyle {
/// Base font.
    ///
    /// Corresponds to Java: `private final FontBase m_fontBase`
    font_base: Option<FontBase>,
/// Italic flag.
    ///
    /// Corresponds to Java: `private final boolean m_italic`
    italic: bool,
/// Bold flag.
    ///
    /// Corresponds to Java: `private final boolean m_bold`
    bold: bool,
/// Underline flag.
    ///
    /// Corresponds to Java: `private final boolean m_underline`
    underline: bool,
/// Strikethrough flag.
    ///
    /// Corresponds to Java: `private final boolean m_strikethrough`
    strikethrough: bool,
/// Font color.
    ///
    /// Corresponds to Java: `private final Color m_color`
    color: Option<Color>,
/// Background color.
    ///
    /// Corresponds to Java: `private final Color m_backgroundColor`
    background_color: Option<Color>,
/// Background pattern.
    ///
    /// Corresponds to Java: `private final BackgroundPattern m_backgroundPattern`
    background_pattern: BackgroundPattern,
}

impl FontStyle {
/// Constructor.
    ///
    /// Corresponds to Java:
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

/// Get the base font.
    ///
    /// Corresponds to Java: `public FontBase getFontBase()`
    pub fn get_font_base(&self) -> Option<&FontBase> {
        self.font_base.as_ref()
    }

/// Get the bold flag.
    ///
    /// Corresponds to Java: `public boolean getBold()`
    pub fn get_bold(&self) -> bool {
        self.bold
    }

/// Get the font color.
    ///
    /// Corresponds to Java: `public Color getColor()`
    pub fn get_color(&self) -> Option<Color> {
        self.color
    }

/// Get the italic flag.
    ///
    /// Corresponds to Java: `public boolean getItalic()`
    pub fn get_italic(&self) -> bool {
        self.italic
    }

/// Get the underline flag.
    ///
    /// Corresponds to Java: `public boolean getUnderline()`
    pub fn get_underline(&self) -> bool {
        self.underline
    }

/// Get the strikethrough flag.
    ///
    /// Corresponds to Java: `public boolean getStrikethrough()`
    pub fn get_strikethrough(&self) -> bool {
        self.strikethrough
    }

/// Get the background color.
    ///
    /// Corresponds to Java: `public Color getBackgroundColor()`
    pub fn get_background_color(&self) -> Option<Color> {
        self.background_color
    }

/// Get the background pattern.
    ///
    /// Corresponds to Java: `public BackgroundPattern getBackgroundPattern()`
    pub fn get_background_pattern(&self) -> BackgroundPattern {
        self.background_pattern
    }
}

impl Display for FontStyle {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Concatenate field by field, in exactly the same order as Java.
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

#[cfg(test)]
mod tests {
    use super::*;

/// Construct a style for testing.
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

/// The getters match Java (including value retrieval independent of field order).
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

/// The three parameters that can be null in Java are `Option` on the Rust side.
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

/// The toString format matches Java (including the `null` branch).
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

/// The derived `PartialEq` compares field by field (the Java side only has reference equality).
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
