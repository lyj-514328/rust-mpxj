//! Corresponds to Java: `org/mpxj/mpp/GanttBarCommonStyle.java`
//!
//! The attributes **shared** by Gantt bar styles (normal styles and single-task exception styles):
//! the shape, style, color and pattern of the start / middle / end segments, as well as the left, top, right, bottom and in-bar text fields.
//!
//! # Porting trade-offs
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `java.awt.Color` | [`crate::mpp::mpp_utility::Color`] | `java.awt.Color` is a GUI type (a skipped item), and this project has already defined an equivalent minimal color type inside `MPPUtility` |
//! | `FieldType` (interface, may be null) | `Option<MppFieldType>` | `FieldType` is expressed on the Rust side as [`MppFieldType`] (two cases: built-in field / user-defined field); Java's `null` corresponds to `None` |
//! | Field `private X m_xxx;` (default null) | Field `Option<X>` | As above, Java's 0 / null initial value is `None` on the Rust side |
//! | `getXxx()` returns a `FieldType` reference | `get_xxx(&self) -> Option<MppFieldType>` | The Rust side returns a clone ([`MppFieldType`] holds an `Rc` internally, so cloning is cheap), avoiding tying the borrow to `&self` |
//! | `PrintWriter` / `ByteArrayOutputStream` in `toString()` | `impl Display` | The output content is kept line-by-line consistent |
//! | `java.awt.Color.toString()`'s `java.awt.Color[r=..,g=..,b=..]` | Private function `format_color` | Replicates Java's print format so that `Display` output matches the original |
//!
//! Inheritance: Java's `GanttBarStyle` / `GanttBarStyleException` both `extends GanttBarCommonStyle`;
//! Rust has no inheritance, so the two subclasses hold this type via **composition + forwarding** (see the file headers of their respective modules).
//!
//! Skipped Java members: none (`java.io.PrintWriter` / `ByteArrayOutputStream` are pure output plumbing,
//! replaced by `Display`).

use std::fmt::{self, Display, Formatter};

use crate::common::mpp_field_type::MppFieldType;
use crate::mpp::chart_pattern::ChartPattern;
use crate::mpp::gantt_bar_middle_shape::GanttBarMiddleShape;
use crate::mpp::gantt_bar_start_end_shape::GanttBarStartEndShape;
use crate::mpp::gantt_bar_start_end_type::GanttBarStartEndType;
use crate::mpp::mpp_utility::{format_color, format_optional_display, Color};

/// The shared portion of Gantt bar styles (normal / exception).
///
/// Corresponds to Java: `public class GanttBarCommonStyle`
///
/// Note: `Debug` is not derived -- the text field type [`MppFieldType`] does not implement `Debug`
/// (an internal structure like `Rc<RefCell<UserDefinedField>>`), and the Java class also has no debug output.
#[derive(Clone, Default)]
pub struct GanttBarCommonStyle {
    /// Corresponds to Java: `private GanttBarStartEndShape m_startShape`
    start_shape: Option<GanttBarStartEndShape>,
    /// Corresponds to Java: `private GanttBarStartEndType m_startType`
    start_type: Option<GanttBarStartEndType>,
    /// Corresponds to Java: `private Color m_startColor`
    start_color: Option<Color>,

    /// Corresponds to Java: `private GanttBarMiddleShape m_middleShape`
    middle_shape: Option<GanttBarMiddleShape>,
    /// Corresponds to Java: `private ChartPattern m_middlePattern`
    middle_pattern: Option<ChartPattern>,
    /// Corresponds to Java: `private Color m_middleColor`
    middle_color: Option<Color>,

    /// Corresponds to Java: `private GanttBarStartEndShape m_endShape`
    end_shape: Option<GanttBarStartEndShape>,
    /// Corresponds to Java: `private GanttBarStartEndType m_endType`
    end_type: Option<GanttBarStartEndType>,
    /// Corresponds to Java: `private Color m_endColor`
    end_color: Option<Color>,

    /// Corresponds to Java: `private FieldType m_leftText`
    left_text: Option<MppFieldType>,
    /// Corresponds to Java: `private FieldType m_rightText`
    right_text: Option<MppFieldType>,
    /// Corresponds to Java: `private FieldType m_topText`
    top_text: Option<MppFieldType>,
    /// Corresponds to Java: `private FieldType m_bottomText`
    bottom_text: Option<MppFieldType>,
    /// Corresponds to Java: `private FieldType m_insideText`
    inside_text: Option<MppFieldType>,
}

impl GanttBarCommonStyle {
/// Construct an empty shared style.
    ///
/// Corresponds to Java's implicit default constructor `GanttBarCommonStyle()`
    pub fn new() -> GanttBarCommonStyle {
        GanttBarCommonStyle::default()
    }

/// Get the text field displayed at the bottom of the bar.
    ///
    /// Corresponds to Java: `public FieldType getBottomText()`
    pub fn get_bottom_text(&self) -> Option<MppFieldType> {
        self.bottom_text.clone()
    }

/// Set the text field displayed at the bottom of the bar.
    ///
    /// Corresponds to Java: `public void setBottomText(FieldType field)`
    pub fn set_bottom_text(&mut self, field: Option<MppFieldType>) {
        self.bottom_text = field;
    }

/// Get the color of the bar's end area.
    ///
    /// Corresponds to Java: `public Color getEndColor()`
    pub fn get_end_color(&self) -> Option<Color> {
        self.end_color
    }

/// Set the color of the bar's end area.
    ///
    /// Corresponds to Java: `public void setEndColor(Color color)`
    pub fn set_end_color(&mut self, color: Option<Color>) {
        self.end_color = color;
    }

/// Get the text field displayed inside the bar.
    ///
    /// Corresponds to Java: `public FieldType getInsideText()`
    pub fn get_inside_text(&self) -> Option<MppFieldType> {
        self.inside_text.clone()
    }

/// Set the text field displayed inside the bar.
    ///
    /// Corresponds to Java: `public void setInsideText(FieldType field)`
    pub fn set_inside_text(&mut self, field: Option<MppFieldType>) {
        self.inside_text = field;
    }

/// Get the text field displayed to the left of the bar.
    ///
    /// Corresponds to Java: `public FieldType getLeftText()`
    pub fn get_left_text(&self) -> Option<MppFieldType> {
        self.left_text.clone()
    }

/// Set the text field displayed to the left of the bar.
    ///
    /// Corresponds to Java: `public void setLeftText(FieldType field)`
    pub fn set_left_text(&mut self, field: Option<MppFieldType>) {
        self.left_text = field;
    }

/// Get the color of the bar's middle segment.
    ///
    /// Corresponds to Java: `public Color getMiddleColor()`
    pub fn get_middle_color(&self) -> Option<Color> {
        self.middle_color
    }

/// Set the color of the bar's middle segment.
    ///
    /// Corresponds to Java: `public void setMiddleColor(Color color)`
    pub fn set_middle_color(&mut self, color: Option<Color>) {
        self.middle_color = color;
    }

/// Get the fill pattern of the bar's middle segment.
    ///
    /// Corresponds to Java: `public ChartPattern getMiddlePattern()`
    pub fn get_middle_pattern(&self) -> Option<ChartPattern> {
        self.middle_pattern
    }

/// Set the fill pattern of the bar's middle segment.
    ///
    /// Corresponds to Java: `public void setMiddlePattern(ChartPattern pattern)`
    pub fn set_middle_pattern(&mut self, pattern: Option<ChartPattern>) {
        self.middle_pattern = pattern;
    }

/// Get the shape of the bar's middle segment.
    ///
    /// Corresponds to Java: `public GanttBarMiddleShape getMiddleShape()`
    pub fn get_middle_shape(&self) -> Option<GanttBarMiddleShape> {
        self.middle_shape
    }

/// Set the shape of the bar's middle segment.
    ///
    /// Corresponds to Java: `public void setMiddleShape(GanttBarMiddleShape shape)`
    pub fn set_middle_shape(&mut self, shape: Option<GanttBarMiddleShape>) {
        self.middle_shape = shape;
    }

/// Get the text field displayed to the right of the bar.
    ///
    /// Corresponds to Java: `public FieldType getRightText()`
    pub fn get_right_text(&self) -> Option<MppFieldType> {
        self.right_text.clone()
    }

/// Set the text field displayed to the right of the bar.
    ///
    /// Corresponds to Java: `public void setRightText(FieldType field)`
    pub fn set_right_text(&mut self, field: Option<MppFieldType>) {
        self.right_text = field;
    }

/// Get the color of the bar's start area.
    ///
    /// Corresponds to Java: `public Color getStartColor()`
    pub fn get_start_color(&self) -> Option<Color> {
        self.start_color
    }

/// Set the color of the bar's start area.
    ///
    /// Corresponds to Java: `public void setStartColor(Color color)`
    pub fn set_start_color(&mut self, color: Option<Color>) {
        self.start_color = color;
    }

/// Get the shape of the bar's start.
    ///
    /// Corresponds to Java: `public GanttBarStartEndShape getStartShape()`
    pub fn get_start_shape(&self) -> Option<GanttBarStartEndShape> {
        self.start_shape
    }

/// Set the shape of the bar's start.
    ///
    /// Corresponds to Java: `public void setStartShape(GanttBarStartEndShape shape)`
    pub fn set_start_shape(&mut self, shape: Option<GanttBarStartEndShape>) {
        self.start_shape = shape;
    }

/// Get the shape of the bar's end.
    ///
    /// Corresponds to Java: `public GanttBarStartEndShape getEndShape()`
    pub fn get_end_shape(&self) -> Option<GanttBarStartEndShape> {
        self.end_shape
    }

/// Set the shape of the bar's end.
    ///
    /// Corresponds to Java: `public void setEndShape(GanttBarStartEndShape shape)`
    pub fn set_end_shape(&mut self, shape: Option<GanttBarStartEndShape>) {
        self.end_shape = shape;
    }

/// Get the style of the bar's start.
    ///
    /// Corresponds to Java: `public GanttBarStartEndType getStartType()`
    pub fn get_start_type(&self) -> Option<GanttBarStartEndType> {
        self.start_type
    }

/// Set the style of the bar's start.
    ///
    /// Corresponds to Java: `public void setStartType(GanttBarStartEndType type)`
    pub fn set_start_type(&mut self, r#type: Option<GanttBarStartEndType>) {
        self.start_type = r#type;
    }

/// Get the style of the bar's end.
    ///
    /// Corresponds to Java: `public GanttBarStartEndType getEndType()`
    pub fn get_end_type(&self) -> Option<GanttBarStartEndType> {
        self.end_type
    }

/// Set the style of the bar's end.
    ///
    /// Corresponds to Java: `public void setEndType(GanttBarStartEndType type)`
    pub fn set_end_type(&mut self, r#type: Option<GanttBarStartEndType>) {
        self.end_type = r#type;
    }

/// Get the text field displayed above the bar.
    ///
    /// Corresponds to Java: `public FieldType getTopText()`
    pub fn get_top_text(&self) -> Option<MppFieldType> {
        self.top_text.clone()
    }

/// Set the text field displayed above the bar.
    ///
    /// Corresponds to Java: `public void setTopText(FieldType field)`
    pub fn set_top_text(&mut self, field: Option<MppFieldType>) {
        self.top_text = field;
    }
}

impl Display for GanttBarCommonStyle {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Output each attribute line by line, in a format exactly matching Java's `PrintWriter.println` (including the trailing newline).
    ///
/// Note: for text fields (`leftText` etc.) Java prints the enum constant's `name()` (such as `START`),
/// while Rust's [`MppFieldType`] exposes the display name via [`Display`] (such as `Start`); the two differ in case style.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "      StartShape={}", format_optional_display(self.start_shape.as_ref()))?;
        writeln!(f, "      StartType={}", format_optional_display(self.start_type.as_ref()))?;
        writeln!(f, "      StartColor={}", format_color(self.start_color))?;
        writeln!(f, "      MiddleShape={}", format_optional_display(self.middle_shape.as_ref()))?;
        writeln!(f, "      MiddlePattern={}", format_optional_display(self.middle_pattern.as_ref()))?;
        writeln!(f, "      MiddleColor={}", format_color(self.middle_color))?;
        writeln!(f, "      EndShape={}", format_optional_display(self.end_shape.as_ref()))?;
        writeln!(f, "      EndType={}", format_optional_display(self.end_type.as_ref()))?;
        writeln!(f, "      EndColor={}", format_color(self.end_color))?;
        writeln!(f, "      LeftText={}", format_optional_display(self.left_text.as_ref()))?;
        writeln!(f, "      RightText={}", format_optional_display(self.right_text.as_ref()))?;
        writeln!(f, "      TopText={}", format_optional_display(self.top_text.as_ref()))?;
        writeln!(f, "      BottomText={}", format_optional_display(self.bottom_text.as_ref()))?;
        writeln!(f, "      InsideText={}", format_optional_display(self.inside_text.as_ref()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_key::FieldKey;
    use crate::task_field::TaskField;

    #[test]
    fn defaults_match_java_null_fields() {
        let style = GanttBarCommonStyle::new();

        assert!(style.get_start_shape().is_none());
        assert!(style.get_start_type().is_none());
        assert!(style.get_start_color().is_none());
        assert!(style.get_middle_shape().is_none());
        assert!(style.get_middle_pattern().is_none());
        assert!(style.get_middle_color().is_none());
        assert!(style.get_end_shape().is_none());
        assert!(style.get_end_type().is_none());
        assert!(style.get_end_color().is_none());
        assert!(style.get_left_text().is_none());
        assert!(style.get_right_text().is_none());
        assert!(style.get_top_text().is_none());
        assert!(style.get_bottom_text().is_none());
        assert!(style.get_inside_text().is_none());
    }

    #[test]
    fn setters_round_trip() {
        let mut style = GanttBarCommonStyle::new();

        style.set_start_shape(Some(GanttBarStartEndShape::Diamond));
        style.set_start_type(Some(GanttBarStartEndType::Framed));
        style.set_start_color(Some(Color::new(1, 2, 3)));
        style.set_middle_shape(Some(GanttBarMiddleShape::Rounded));
        style.set_middle_pattern(Some(ChartPattern::Solid));
        style.set_middle_color(Some(Color::new(4, 5, 6)));
        style.set_end_shape(Some(GanttBarStartEndShape::Star));
        style.set_end_type(Some(GanttBarStartEndType::Dashed));
        style.set_end_color(Some(Color::new(7, 8, 9)));
        style.set_left_text(Some(MppFieldType::Builtin(FieldKey::Task(TaskField::Name))));
        style.set_right_text(Some(MppFieldType::Builtin(FieldKey::Task(TaskField::Finish))));
        style.set_top_text(Some(MppFieldType::Builtin(FieldKey::Task(TaskField::Start))));
        style.set_bottom_text(Some(MppFieldType::Builtin(FieldKey::Task(TaskField::Duration))));
        style.set_inside_text(Some(MppFieldType::Builtin(FieldKey::Task(TaskField::Cost))));

        assert_eq!(style.get_start_shape(), Some(GanttBarStartEndShape::Diamond));
        assert_eq!(style.get_start_type(), Some(GanttBarStartEndType::Framed));
        assert_eq!(style.get_start_color(), Some(Color::new(1, 2, 3)));
        assert_eq!(style.get_middle_shape(), Some(GanttBarMiddleShape::Rounded));
        assert_eq!(style.get_middle_pattern(), Some(ChartPattern::Solid));
        assert_eq!(style.get_middle_color(), Some(Color::new(4, 5, 6)));
        assert_eq!(style.get_end_shape(), Some(GanttBarStartEndShape::Star));
        assert_eq!(style.get_end_type(), Some(GanttBarStartEndType::Dashed));
        assert_eq!(style.get_end_color(), Some(Color::new(7, 8, 9)));
        assert_eq!(
            style.get_left_text().and_then(|field| field.as_builtin()),
            Some(FieldKey::Task(TaskField::Name))
        );
        assert_eq!(
            style.get_inside_text().and_then(|field| field.as_builtin()),
            Some(FieldKey::Task(TaskField::Cost))
        );
    }

    #[test]
    fn to_string_matches_java_layout() {
        let style = GanttBarCommonStyle::new();
        let text = style.to_string();

        assert!(text.starts_with("      StartShape=null\n"));
        assert!(text.contains("      MiddlePattern=null\n"));
        assert!(text.contains("      EndColor=null\n"));
        assert!(text.ends_with("      InsideText=null\n"));

        let mut colored = GanttBarCommonStyle::new();
        colored.set_start_color(Some(Color::new(255, 128, 0)));
        assert!(colored
            .to_string()
            .contains("      StartColor=java.awt.Color[r=255,g=128,b=0]\n"));
    }
}
