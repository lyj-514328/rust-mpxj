//! Corresponds to Java: `org/mpxj/mpp/GanttBarStyleException.java`
//!
//! A **per-task exception style** for Gantt bar styles: a task does not use the default bar style but instead uses the style with the specified ID.
//!
//! # Porting trade-offs
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `GanttBarStyleException extends GanttBarCommonStyle` | field `common: GanttBarCommonStyle` + **forwarding methods** | Rust has no inheritance, so composition is used instead (same as `GanttBarStyle`) |
//! | `Integer m_ganttBarStyleID` | `Option<i32>` | Java's initial value of `null` |
//! | `int m_taskUniqueID` / `int m_barStyleIndex` | `i32` | Java's initial value of 0 |
//! | `toString()` | `impl Display` | the output format matches Java |
//!
//! **Java only provides `setBarStyleIndex`, with no corresponding getter** (this is how upstream MPXJ is),
//! and the Rust side preserves this 1:1, without adding a `get_bar_style_index` (the field can be observed through `Display`).
//!
//! Skipped Java members: none (`ByteArrayOutputStream` / `PrintWriter` are replaced by `Display`).

use std::fmt::{self, Display, Formatter};

use crate::common::mpp_field_type::MppFieldType;
use crate::mpp::chart_pattern::ChartPattern;
use crate::mpp::gantt_bar_common_style::GanttBarCommonStyle;
use crate::mpp::gantt_bar_middle_shape::GanttBarMiddleShape;
use crate::mpp::gantt_bar_start_end_shape::GanttBarStartEndShape;
use crate::mpp::gantt_bar_start_end_type::GanttBarStartEndType;
use crate::mpp::mpp_utility::Color;

/// A per-task exception style for Gantt bar styles.
///
/// Corresponds to Java: `public final class GanttBarStyleException extends GanttBarCommonStyle`
///
/// Note: `Debug` is not derived (for the same reason as [`GanttBarCommonStyle`]).
#[derive(Clone, Default)]
pub struct GanttBarStyleException {
    /// Corresponds to Java: `extends GanttBarCommonStyle` (composition expresses the inheritance)
    common: GanttBarCommonStyle,
    /// Corresponds to Java: `private int m_taskUniqueID`
    task_unique_id: i32,
    /// Corresponds to Java: `private int m_barStyleIndex`
    bar_style_index: i32,
    /// Corresponds to Java: `private Integer m_ganttBarStyleID`
    gantt_bar_style_id: Option<i32>,
}

impl GanttBarStyleException {
    /// Constructs an empty exception style.
    ///
    /// Corresponds to Java's implicit default constructor `GanttBarStyleException()`
    pub fn new() -> GanttBarStyleException {
        GanttBarStyleException::default()
    }

    /// Gets the inner common style instance.
    ///
    /// Rust-specific: Java exposes the base class members directly through inheritance, whereas composition requires obtaining them explicitly.
    pub fn get_common(&self) -> &GanttBarCommonStyle {
        &self.common
    }

    /// Gets the inner common style instance (mutable).
    ///
    /// Rust-specific, same as [`GanttBarStyleException::get_common`].
    pub fn get_common_mut(&mut self) -> &mut GanttBarCommonStyle {
        &mut self.common
    }

    /// Gets the task unique ID to which this exception style applies.
    ///
    /// Corresponds to Java: `public int getTaskUniqueID()`
    pub fn get_task_unique_id(&self) -> i32 {
        self.task_unique_id
    }

    /// Sets the task unique ID.
    ///
    /// Corresponds to Java: `public void setTaskUniqueID(int id)`
    pub fn set_task_unique_id(&mut self, id: i32) {
        self.task_unique_id = id;
    }

    /// Gets the Gantt bar style ID referenced by this exception style.
    ///
    /// Corresponds to Java: `public Integer getGanttBarStyleID()`
    pub fn get_gantt_bar_style_id(&self) -> Option<i32> {
        self.gantt_bar_style_id
    }

    /// Sets the referenced Gantt bar style ID.
    ///
    /// Corresponds to Java: `public void setGanttBarStyleID(Integer value)`
    pub fn set_gantt_bar_style_id(&mut self, value: Option<i32>) {
        self.gantt_bar_style_id = value;
    }

    /// Sets the bar style index.
    ///
    /// Corresponds to Java: `public void setBarStyleIndex(int index)`
    pub fn set_bar_style_index(&mut self, index: i32) {
        self.bar_style_index = index;
    }

    // ---- Below are forwarding methods for the members of `GanttBarCommonStyle` (Java's base class) ----

    /// Forwards [`GanttBarCommonStyle::get_bottom_text`].
    ///
    /// Corresponds to Java: the `public FieldType getBottomText()` inherited from `GanttBarCommonStyle`
    pub fn get_bottom_text(&self) -> Option<MppFieldType> {
        self.common.get_bottom_text()
    }

    /// Forwards [`GanttBarCommonStyle::set_bottom_text`].
    ///
    /// Corresponds to Java: the `public void setBottomText(FieldType field)` inherited from `GanttBarCommonStyle`
    pub fn set_bottom_text(&mut self, field: Option<MppFieldType>) {
        self.common.set_bottom_text(field);
    }

    /// Forwards [`GanttBarCommonStyle::get_end_color`].
    ///
    /// Corresponds to Java: the `public Color getEndColor()` inherited from `GanttBarCommonStyle`
    pub fn get_end_color(&self) -> Option<Color> {
        self.common.get_end_color()
    }

    /// Forwards [`GanttBarCommonStyle::set_end_color`].
    ///
    /// Corresponds to Java: the `public void setEndColor(Color color)` inherited from `GanttBarCommonStyle`
    pub fn set_end_color(&mut self, color: Option<Color>) {
        self.common.set_end_color(color);
    }

    /// Forwards [`GanttBarCommonStyle::get_inside_text`].
    ///
    /// Corresponds to Java: the `public FieldType getInsideText()` inherited from `GanttBarCommonStyle`
    pub fn get_inside_text(&self) -> Option<MppFieldType> {
        self.common.get_inside_text()
    }

    /// Forwards [`GanttBarCommonStyle::set_inside_text`].
    ///
    /// Corresponds to Java: the `public void setInsideText(FieldType field)` inherited from `GanttBarCommonStyle`
    pub fn set_inside_text(&mut self, field: Option<MppFieldType>) {
        self.common.set_inside_text(field);
    }

    /// Forwards [`GanttBarCommonStyle::get_left_text`].
    ///
    /// Corresponds to Java: the `public FieldType getLeftText()` inherited from `GanttBarCommonStyle`
    pub fn get_left_text(&self) -> Option<MppFieldType> {
        self.common.get_left_text()
    }

    /// Forwards [`GanttBarCommonStyle::set_left_text`].
    ///
    /// Corresponds to Java: the `public void setLeftText(FieldType field)` inherited from `GanttBarCommonStyle`
    pub fn set_left_text(&mut self, field: Option<MppFieldType>) {
        self.common.set_left_text(field);
    }

    /// Forwards [`GanttBarCommonStyle::get_middle_color`].
    ///
    /// Corresponds to Java: the `public Color getMiddleColor()` inherited from `GanttBarCommonStyle`
    pub fn get_middle_color(&self) -> Option<Color> {
        self.common.get_middle_color()
    }

    /// Forwards [`GanttBarCommonStyle::set_middle_color`].
    ///
    /// Corresponds to Java: the `public void setMiddleColor(Color color)` inherited from `GanttBarCommonStyle`
    pub fn set_middle_color(&mut self, color: Option<Color>) {
        self.common.set_middle_color(color);
    }

    /// Forwards [`GanttBarCommonStyle::get_middle_pattern`].
    ///
    /// Corresponds to Java: the `public ChartPattern getMiddlePattern()` inherited from `GanttBarCommonStyle`
    pub fn get_middle_pattern(&self) -> Option<ChartPattern> {
        self.common.get_middle_pattern()
    }

    /// Forwards [`GanttBarCommonStyle::set_middle_pattern`].
    ///
    /// Corresponds to Java: the `public void setMiddlePattern(ChartPattern pattern)` inherited from `GanttBarCommonStyle`
    pub fn set_middle_pattern(&mut self, pattern: Option<ChartPattern>) {
        self.common.set_middle_pattern(pattern);
    }

    /// Forwards [`GanttBarCommonStyle::get_middle_shape`].
    ///
    /// Corresponds to Java: the `public GanttBarMiddleShape getMiddleShape()` inherited from `GanttBarCommonStyle`
    pub fn get_middle_shape(&self) -> Option<GanttBarMiddleShape> {
        self.common.get_middle_shape()
    }

    /// Forwards [`GanttBarCommonStyle::set_middle_shape`].
    ///
    /// Corresponds to Java: the `public void setMiddleShape(GanttBarMiddleShape shape)` inherited from `GanttBarCommonStyle`
    pub fn set_middle_shape(&mut self, shape: Option<GanttBarMiddleShape>) {
        self.common.set_middle_shape(shape);
    }

    /// Forwards [`GanttBarCommonStyle::get_right_text`].
    ///
    /// Corresponds to Java: the `public FieldType getRightText()` inherited from `GanttBarCommonStyle`
    pub fn get_right_text(&self) -> Option<MppFieldType> {
        self.common.get_right_text()
    }

    /// Forwards [`GanttBarCommonStyle::set_right_text`].
    ///
    /// Corresponds to Java: the `public void setRightText(FieldType field)` inherited from `GanttBarCommonStyle`
    pub fn set_right_text(&mut self, field: Option<MppFieldType>) {
        self.common.set_right_text(field);
    }

    /// Forwards [`GanttBarCommonStyle::get_start_color`].
    ///
    /// Corresponds to Java: the `public Color getStartColor()` inherited from `GanttBarCommonStyle`
    pub fn get_start_color(&self) -> Option<Color> {
        self.common.get_start_color()
    }

    /// Forwards [`GanttBarCommonStyle::set_start_color`].
    ///
    /// Corresponds to Java: the `public void setStartColor(Color color)` inherited from `GanttBarCommonStyle`
    pub fn set_start_color(&mut self, color: Option<Color>) {
        self.common.set_start_color(color);
    }

    /// Forwards [`GanttBarCommonStyle::get_start_shape`].
    ///
    /// Corresponds to Java: the `public GanttBarStartEndShape getStartShape()` inherited from `GanttBarCommonStyle`
    pub fn get_start_shape(&self) -> Option<GanttBarStartEndShape> {
        self.common.get_start_shape()
    }

    /// Forwards [`GanttBarCommonStyle::set_start_shape`].
    ///
    /// Corresponds to Java: the `public void setStartShape(GanttBarStartEndShape shape)` inherited from `GanttBarCommonStyle`
    pub fn set_start_shape(&mut self, shape: Option<GanttBarStartEndShape>) {
        self.common.set_start_shape(shape);
    }

    /// Forwards [`GanttBarCommonStyle::get_end_shape`].
    ///
    /// Corresponds to Java: the `public GanttBarStartEndShape getEndShape()` inherited from `GanttBarCommonStyle`
    pub fn get_end_shape(&self) -> Option<GanttBarStartEndShape> {
        self.common.get_end_shape()
    }

    /// Forwards [`GanttBarCommonStyle::set_end_shape`].
    ///
    /// Corresponds to Java: the `public void setEndShape(GanttBarStartEndShape shape)` inherited from `GanttBarCommonStyle`
    pub fn set_end_shape(&mut self, shape: Option<GanttBarStartEndShape>) {
        self.common.set_end_shape(shape);
    }

    /// Forwards [`GanttBarCommonStyle::get_start_type`].
    ///
    /// Corresponds to Java: the `public GanttBarStartEndType getStartType()` inherited from `GanttBarCommonStyle`
    pub fn get_start_type(&self) -> Option<GanttBarStartEndType> {
        self.common.get_start_type()
    }

    /// Forwards [`GanttBarCommonStyle::set_start_type`].
    ///
    /// Corresponds to Java: the `public void setStartType(GanttBarStartEndType type)` inherited from `GanttBarCommonStyle`
    pub fn set_start_type(&mut self, r#type: Option<GanttBarStartEndType>) {
        self.common.set_start_type(r#type);
    }

    /// Forwards [`GanttBarCommonStyle::get_end_type`].
    ///
    /// Corresponds to Java: the `public GanttBarStartEndType getEndType()` inherited from `GanttBarCommonStyle`
    pub fn get_end_type(&self) -> Option<GanttBarStartEndType> {
        self.common.get_end_type()
    }

    /// Forwards [`GanttBarCommonStyle::set_end_type`].
    ///
    /// Corresponds to Java: the `public void setEndType(GanttBarStartEndType type)` inherited from `GanttBarCommonStyle`
    pub fn set_end_type(&mut self, r#type: Option<GanttBarStartEndType>) {
        self.common.set_end_type(r#type);
    }

    /// Forwards [`GanttBarCommonStyle::get_top_text`].
    ///
    /// Corresponds to Java: the `public FieldType getTopText()` inherited from `GanttBarCommonStyle`
    pub fn get_top_text(&self) -> Option<MppFieldType> {
        self.common.get_top_text()
    }

    /// Forwards [`GanttBarCommonStyle::set_top_text`].
    ///
    /// Corresponds to Java: the `public void setTopText(FieldType field)` inherited from `GanttBarCommonStyle`
    pub fn set_top_text(&mut self, field: Option<MppFieldType>) {
        self.common.set_top_text(field);
    }
}

impl Display for GanttBarStyleException {
    /// Corresponds to Java: `@Override public String toString()`
    ///
    /// Java output: a fixed-indentation property block, followed by the inlined base class `toString()`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "   [GanttBarStyleException")?;
        writeln!(f, "      TaskID={}", self.task_unique_id)?;
        writeln!(
            f,
            "      BarStyleID={}",
            match self.gantt_bar_style_id {
                Some(value) => value.to_string(),
                None => "null".to_owned(),
            }
        )?;
        writeln!(f, "      BarStyleIndex={}", self.bar_style_index)?;
        write!(f, "{}", self.common)?;
        writeln!(f, "   ]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_java() {
        let style = GanttBarStyleException::new();

        assert_eq!(style.get_task_unique_id(), 0);
        assert_eq!(style.get_gantt_bar_style_id(), None);
        assert!(style.get_start_shape().is_none());
    }

    #[test]
    fn setters_round_trip() {
        let mut style = GanttBarStyleException::new();

        style.set_task_unique_id(42);
        style.set_gantt_bar_style_id(Some(7));
        style.set_bar_style_index(6);
        style.set_middle_shape(Some(GanttBarMiddleShape::Rounded));

        assert_eq!(style.get_task_unique_id(), 42);
        assert_eq!(style.get_gantt_bar_style_id(), Some(7));
        assert_eq!(
            style.get_common().get_middle_shape(),
            Some(GanttBarMiddleShape::Rounded)
        );
    }

    #[test]
    fn to_string_matches_java_layout() {
        let mut style = GanttBarStyleException::new();
        style.set_task_unique_id(42);
        style.set_gantt_bar_style_id(Some(7));
        style.set_bar_style_index(6);

        let text = style.to_string();
        assert!(text.starts_with("   [GanttBarStyleException\n"));
        assert!(text.contains("      TaskID=42\n"));
        assert!(text.contains("      BarStyleID=7\n"));
        assert!(text.contains("      BarStyleIndex=6\n"));
        assert!(text.ends_with("      InsideText=null\n   ]\n"));
    }

    #[test]
    fn to_string_prints_null_ids_like_java() {
        let style = GanttBarStyleException::new();
        assert!(style.to_string().contains("      BarStyleID=null\n"));
    }
}
