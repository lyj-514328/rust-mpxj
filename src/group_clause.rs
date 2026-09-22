//! Corresponds to Java: `org/mpxj/GroupClause.java`
//!
//! # Java's Three Imports and Their Correspondences
//!
//! ```java
//! import java.awt.Color;
//!
//! import org.mpxj.mpp.BackgroundPattern;
//! import org.mpxj.mpp.FontStyle;
//! ```
//!
//! That is, although this class lives in the model layer (`org.mpxj`), its font / color /
//! pattern members come from the **MPP read layer** (`org.mpxj.mpp`) and `java.awt`. The Rust
//! side copies this dependency direction (the model layer depends on `crate::mpp`); the
//! correspondences are as follows:
//!
//! | Java import | Rust |
//! |---|---|
//! | `java.awt.Color` | [`crate::mpp::mpp_utility::Color`] (`java.awt.*` is a skipped item; this crate reuses the return type of `MPPUtility.get_color`, see that module's docs) |
//! | `org.mpxj.mpp.FontStyle` | [`crate::mpp::font_style::FontStyle`] |
//! | `org.mpxj.mpp.BackgroundPattern` | [`crate::mpp::background_pattern::BackgroundPattern`] |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `private FieldType m_field` | [`GroupClause::get_field`] / [`GroupClause::set_field`] (`FieldType` → [`FieldKey`]) | This crate's runtime erasure of the `FieldType` interface |
//! | `private FontStyle m_font` | `Option<FontStyle>` | Initially `null` on the Java side; only has a value once an MPP reader has populated it |
//! | `private Color m_cellBackgroundColor` | `Option<Color>` | Same as above; the color for `ColorType.AUTOMATIC` is also `null` in Java |
//! | `private BackgroundPattern m_pattern` | `Option<BackgroundPattern>` | Same as above |
//! | `public Object getGroupInterval()` / `setGroupInterval(Object)` | [`GroupClause::get_group_interval`] / [`GroupClause::set_group_interval`] (`Object` → `Option<FieldValue>`, see below) | The actual values are scalars such as numbers / strings / durations |
//! | `@Override public String toString()` | [`Display`] | The eight-segment format is exactly consistent with Java (including Java's **omission** of the trailing `]`, preserved as-is) |
//!
//! Unported / skipped Java members: none (the three style members were completed along with
//! `FontStyle` / `BackgroundPattern` / `Color`).
//!
//! Reflection / JDBC / XML(JAXB) / `javax.*` / `Serializable`: this class does not use these mechanisms.
//!
//! Two further points:
//!
//! - **`Object` values**: `m_startAt` / `m_groupInterval` are `Object` in Java (the actual
//!   values are scalars such as numbers / strings / durations). The Rust side expresses them
//!   with this crate's scalar `Object` carrier [`FieldValue`].
//! - **`toString()` format**: Java outputs the `field` / `ascending` / `font` / `color` /
//!   `pattern` / `groupOn` / `startAt` / `groupInterval` segments (and **omits** the trailing
//!   `]`, a Java behavior preserved as-is). The Rust side copies each segment, rendering the
//!   `color` segment in the form of `java.awt.Color.toString()` (see [`format_color`]).
//! - **`equals` / `hashCode`**: not overridden in Java (object identity); the Rust side does
//!   not implement `PartialEq` / `Eq` / `Hash`.

use std::fmt::{self, Display, Formatter};

use crate::field_key::FieldKey;
use crate::field_value::FieldValue;
use crate::mpp::background_pattern::BackgroundPattern;
use crate::mpp::font_style::FontStyle;
use crate::mpp::mpp_utility::{format_color, Color};
use crate::test_operator::Operand;

/// A single clause in a group definition.
///
/// Corresponds to Java: `public final class GroupClause`
pub struct GroupClause {
    /// Group field.
    ///
    /// Corresponds to Java: `private FieldType m_field;`
    m_field: Option<FieldKey>,

    /// Whether grouping is ascending.
    ///
    /// Corresponds to Java: `private boolean m_ascending;`
    m_ascending: bool,

    /// Font style.
    ///
    /// Corresponds to Java: `private FontStyle m_font;`
    m_font: Option<FontStyle>,

    /// Cell background color.
    ///
    /// Corresponds to Java: `private Color m_cellBackgroundColor;`
    m_cell_background_color: Option<Color>,

    /// Background pattern.
    ///
    /// Corresponds to Java: `private BackgroundPattern m_pattern;`
    m_pattern: Option<BackgroundPattern>,

    /// Group-on mode.
    ///
    /// Corresponds to Java: `private int m_groupOn;`
    m_group_on: i32,

    /// "Start at" value.
    ///
    /// Corresponds to Java: `private Object m_startAt;`
    m_start_at: Option<FieldValue>,

    /// Group interval.
    ///
    /// Corresponds to Java: `private Object m_groupInterval;`
    m_group_interval: Option<FieldValue>,
}

impl GroupClause {
    /// Constructor.
    ///
    /// Corresponds to Java: implicit default constructor (Java declares no explicit constructor).
    pub fn new() -> Self {
        Self {
            m_field: None,
            m_ascending: false,
            m_font: None,
            m_cell_background_color: None,
            m_pattern: None,
            m_group_on: 0,
            m_start_at: None,
            m_group_interval: None,
        }
    }

    /// Return the "whether grouping is ascending" flag.
    ///
    /// Corresponds to Java: `public boolean getAscending()`
    pub fn get_ascending(&self) -> bool {
        self.m_ascending
    }

    /// Set the "whether grouping is ascending" flag.
    ///
    /// Corresponds to Java: `public void setAscending(boolean ascending)`
    pub fn set_ascending(&mut self, ascending: bool) {
        self.m_ascending = ascending;
    }

    /// Return the cell background color.
    ///
    /// Corresponds to Java: `public Color getCellBackgroundColor()`
    pub fn get_cell_background_color(&self) -> Option<Color> {
        self.m_cell_background_color
    }

    /// Set the cell background color.
    ///
    /// Corresponds to Java: `public void setCellBackgroundColor(Color color)`
    pub fn set_cell_background_color(&mut self, color: Option<Color>) {
        self.m_cell_background_color = color;
    }

    /// Return the group field.
    ///
    /// Corresponds to Java: `public FieldType getField()`
    pub fn get_field(&self) -> Option<FieldKey> {
        self.m_field
    }

    /// Set the group field.
    ///
    /// Corresponds to Java: `public void setField(FieldType field)`
    pub fn set_field(&mut self, field: Option<FieldKey>) {
        self.m_field = field;
    }

    /// Return the font style.
    ///
    /// Corresponds to Java: `public FontStyle getFont()`
    pub fn get_font(&self) -> Option<&FontStyle> {
        self.m_font.as_ref()
    }

    /// Set the font style.
    ///
    /// Corresponds to Java: `public void setFont(FontStyle font)`
    pub fn set_font(&mut self, font: Option<FontStyle>) {
        self.m_font = font;
    }

    /// Return the group interval.
    ///
    /// Corresponds to Java: `public Object getGroupInterval()`
    pub fn get_group_interval(&self) -> Option<&FieldValue> {
        self.m_group_interval.as_ref()
    }

    /// Set the group interval.
    ///
    /// Corresponds to Java: `public void setGroupInterval(Object groupInterval)`
    pub fn set_group_interval(&mut self, group_interval: Option<FieldValue>) {
        self.m_group_interval = group_interval;
    }

    /// Return the group-on mode.
    ///
    /// Corresponds to Java: `public int getGroupOn()`
    pub fn get_group_on(&self) -> i32 {
        self.m_group_on
    }

    /// Set the group-on mode.
    ///
    /// Corresponds to Java: `public void setGroupOn(int groupOn)`
    pub fn set_group_on(&mut self, group_on: i32) {
        self.m_group_on = group_on;
    }

    /// Return the background pattern.
    ///
    /// Corresponds to Java: `public BackgroundPattern getPattern()`
    pub fn get_pattern(&self) -> Option<BackgroundPattern> {
        self.m_pattern
    }

    /// Set the background pattern.
    ///
    /// Corresponds to Java: `public void setPattern(BackgroundPattern pattern)`
    pub fn set_pattern(&mut self, pattern: Option<BackgroundPattern>) {
        self.m_pattern = pattern;
    }

    /// Return the "start at" value.
    ///
    /// Corresponds to Java: `public Object getStartAt()`
    pub fn get_start_at(&self) -> Option<&FieldValue> {
        self.m_start_at.as_ref()
    }

    /// Set the "start at" value.
    ///
    /// Corresponds to Java: `public void setStartAt(Object startAt)`
    pub fn set_start_at(&mut self, start_at: Option<FieldValue>) {
        self.m_start_at = start_at;
    }
}

impl Default for GroupClause {
    /// Corresponds to Java: implicit default constructor, equivalent to [`GroupClause::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// Render the value in Java's `String.valueOf` style (`None` → `"null"`).
fn render_value(value: Option<&FieldValue>) -> String {
    // Reuse [`Operand`]'s Java-style rendering to avoid multiple implementations of the same rendering logic.
    match value {
        Some(value) => Operand::from(value.clone()).to_string(),
        None => "null".to_owned(),
    }
}

/// Render an optional object with Java's string-concatenation semantics (`None` → `"null"`).
fn render_optional<T: Display>(value: Option<&T>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

impl Display for GroupClause {
    /// Corresponds to Java: `@Override public String toString()`
    ///
    /// Consistent with Java, **does not** include the trailing `]`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let field = match self.m_field {
            Some(field) => field.to_string(),
            None => "null".to_owned(),
        };

        write!(
            f,
            "[GroupClause field={} ascending={} font={} color={} pattern={} groupOn={} startAt={} groupInterval={}",
            field,
            self.m_ascending,
            render_optional(self.m_font.as_ref()),
            format_color(self.m_cell_background_color),
            render_optional(self.m_pattern.as_ref()),
            self.m_group_on,
            render_value(self.m_start_at.as_ref()),
            render_value(self.m_group_interval.as_ref())
        )
    }
}

impl fmt::Debug for GroupClause {
    /// Java does not override debug output; the Rust side provides `Debug` (reusing [`Display`]).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpp::font_base::FontBase;
    use crate::task_field::TaskField;
    use crate::time_unit::TimeUnit;

    #[test]
    fn accessors_round_trip() {
        let mut clause = GroupClause::new();

        assert!(!clause.get_ascending());
        assert_eq!(clause.get_field(), None);
        assert_eq!(clause.get_cell_background_color(), None);
        assert_eq!(clause.get_font(), None);
        assert_eq!(clause.get_group_interval(), None);
        assert_eq!(clause.get_group_on(), 0);
        assert_eq!(clause.get_pattern(), None);
        assert_eq!(clause.get_start_at(), None);

        let font = FontStyle::new(
            Some(FontBase::new(Some(1), "Arial".to_owned(), 10)),
            true,
            false,
            true,
            false,
            Some(Color::new(0, 0, 0)),
            None,
            BackgroundPattern::Solid,
        );

        clause.set_ascending(true);
        clause.set_field(Some(FieldKey::Task(TaskField::Name)));
        clause.set_group_on(2);
        clause.set_start_at(Some(FieldValue::String("A".to_owned())));
        clause.set_group_interval(Some(FieldValue::Integer(3)));
        clause.set_font(Some(font.clone()));
        clause.set_cell_background_color(Some(Color::new(255, 0, 0)));
        clause.set_pattern(Some(BackgroundPattern::Dotted));

        assert!(clause.get_ascending());
        assert_eq!(clause.get_field(), Some(FieldKey::Task(TaskField::Name)));
        assert_eq!(clause.get_group_on(), 2);
        assert_eq!(clause.get_start_at(), Some(&FieldValue::String("A".to_owned())));
        assert_eq!(
            clause.get_group_interval(),
            Some(&FieldValue::Integer(3))
        );
        // Java's `setFont` / `setCellBackgroundColor` / `setPattern` round-trip consistently
        assert_eq!(clause.get_font(), Some(&font));
        assert_eq!(
            clause.get_cell_background_color(),
            Some(Color::new(255, 0, 0))
        );
        assert_eq!(clause.get_pattern(), Some(BackgroundPattern::Dotted));
    }

    /// The three style members can be cleared again (Java likewise allows passing `null`).
    #[test]
    fn style_members_can_be_cleared() {
        let mut clause = GroupClause::new();
        clause.set_font(Some(FontStyle::new(
            None,
            false,
            false,
            false,
            false,
            None,
            None,
            BackgroundPattern::Solid,
        )));
        clause.set_cell_background_color(Some(Color::new(1, 2, 3)));
        clause.set_pattern(Some(BackgroundPattern::Checkered));

        clause.set_font(None);
        clause.set_cell_background_color(None);
        clause.set_pattern(None);

        assert_eq!(clause.get_font(), None);
        assert_eq!(clause.get_cell_background_color(), None);
        assert_eq!(clause.get_pattern(), None);
    }

    #[test]
    fn values_may_be_durations() {
        let mut clause = GroupClause::new();
        clause.set_group_interval(Some(FieldValue::Duration(crate::duration::Duration::get_instance(
            1.0,
            TimeUnit::Weeks,
        ))));

        let interval = clause.get_group_interval().and_then(FieldValue::as_duration);
        assert_eq!(interval.map(|value| value.get_units()), Some(TimeUnit::Weeks));
    }

    #[test]
    fn display_matches_java_format_without_closing_bracket() {
        let mut clause = GroupClause::new();
        clause.set_field(Some(FieldKey::Task(TaskField::Name)));
        clause.set_ascending(true);
        clause.set_group_on(1);
        clause.set_start_at(Some(FieldValue::String("A".to_owned())));
        clause.set_group_interval(Some(FieldValue::Integer(2)));

        assert_eq!(
            clause.to_string(),
            "[GroupClause field=Task Name ascending=true font=null color=null pattern=null groupOn=1 startAt=A groupInterval=2"
        );
    }

    /// The `font` / `color` / `pattern` segments match Java's `toString()` verbatim.
    #[test]
    fn display_renders_font_color_and_pattern() {
        let mut clause = GroupClause::new();
        clause.set_font(Some(FontStyle::new(
            Some(FontBase::new(Some(1), "Arial".to_owned(), 10)),
            true,
            false,
            false,
            false,
            Some(Color::new(0, 0, 0)),
            None,
            BackgroundPattern::Solid,
        )));
        clause.set_cell_background_color(Some(Color::new(0, 0, 255)));
        clause.set_pattern(Some(BackgroundPattern::Dotted));

        assert_eq!(
            clause.to_string(),
            concat!(
                "[GroupClause field=null ascending=false ",
                "font=[FontStyle fontBase=[FontBase name=Arial size=10] italic=true bold=false ",
                "underline=false strikethrough=false color=java.awt.Color[r=0,g=0,b=0] ",
                "backgroundColor=null backgroundPattern=Solid] ",
                "color=java.awt.Color[r=0,g=0,b=255] pattern=Dotted ",
                "groupOn=0 startAt=null groupInterval=null"
            )
        );
    }

    #[test]
    fn display_renders_nulls() {
        let clause = GroupClause::new();
        assert_eq!(
            clause.to_string(),
            "[GroupClause field=null ascending=false font=null color=null pattern=null groupOn=0 startAt=null groupInterval=null"
        );
    }

    #[test]
    fn default_matches_new() {
        let clause = GroupClause::default();
        assert_eq!(clause.get_group_on(), 0);
    }
}
