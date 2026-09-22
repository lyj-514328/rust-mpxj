//! Corresponds to Java: `org/mpxj/mpp/GanttBarStyle.java`
//!
//! The **default style** of a Gantt bar: which tasks it is displayed on, which two fields determine its start and end dates,
//! which row it is displayed on, and the shape / colour / text settings inherited from [`GanttBarCommonStyle`].
//!
//! # Porting trade-offs
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `GanttBarStyle extends GanttBarCommonStyle` | field `common: GanttBarCommonStyle` + **forwarding methods** | Rust has no inheritance, so composition is used instead: `get_xxx` / `set_xxx` forward method by method to `common`, and [`GanttBarStyle::get_common`] is provided to obtain the inner instance |
//! | `Integer m_id` / `String m_name` | `Option<i32>` / `Option<String>` | Java's initial `null` value |
//! | `FieldType m_fromField` / `m_toField` | `Option<MppFieldType>` | as above; on the Rust side `FieldType` is represented by [`MppFieldType`] |
//! | `Set<GanttBarShowForTasks> m_showForTasks = new TreeSet<>()` | `BTreeSet<GanttBarShowForTasks>` | Java's `TreeSet` uses the enum's natural order (declaration order); Rust enum discriminant values match declaration order, so `BTreeSet` has the same order |
//! | `toString()` | `impl Display` | prints its own properties first, then inlines the base class content, then closes, matching the Java output |
//!
//! Skipped Java members: none (`ByteArrayOutputStream` / `PrintWriter` are replaced by `Display`).

use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};

use crate::common::mpp_field_type::MppFieldType;
use crate::mpp::chart_pattern::ChartPattern;
use crate::mpp::gantt_bar_common_style::GanttBarCommonStyle;
use crate::mpp::gantt_bar_middle_shape::GanttBarMiddleShape;
use crate::mpp::gantt_bar_show_for_tasks::GanttBarShowForTasks;
use crate::mpp::gantt_bar_start_end_shape::GanttBarStartEndShape;
use crate::mpp::gantt_bar_start_end_type::GanttBarStartEndType;
use crate::mpp::mpp_utility::Color;

/// The default style of a Gantt bar.
///
/// Corresponds to Java: `public final class GanttBarStyle extends GanttBarCommonStyle`
///
/// Note: `Debug` is not derived (for the same reason as [`GanttBarCommonStyle`]).
#[derive(Clone, Default)]
pub struct GanttBarStyle {
/// Corresponds to Java: `extends GanttBarCommonStyle` (composition is used instead of inheritance)
    common: GanttBarCommonStyle,
    /// Corresponds to Java: `private Integer m_id`
    id: Option<i32>,
    /// Corresponds to Java: `private String m_name`
    name: Option<String>,
    /// Corresponds to Java: `private FieldType m_fromField`
    from_field: Option<MppFieldType>,
    /// Corresponds to Java: `private FieldType m_toField`
    to_field: Option<MppFieldType>,
    /// Corresponds to Java: `private int m_row`
    row: i32,
    /// Corresponds to Java: `private final Set<GanttBarShowForTasks> m_showForTasks = new TreeSet<>()`
    show_for_tasks: BTreeSet<GanttBarShowForTasks>,
}

impl GanttBarStyle {
/// Construct an empty bar style.
    ///
/// Corresponds to Java's implicit default constructor `GanttBarStyle()`
    pub fn new() -> GanttBarStyle {
        GanttBarStyle::default()
    }

/// Get the inner common style instance.
    ///
/// Rust-specific: Java exposes base class members directly through inheritance, whereas composition requires obtaining them explicitly.
    pub fn get_common(&self) -> &GanttBarCommonStyle {
        &self.common
    }

/// Get the inner common style instance (mutable).
    ///
/// Rust-specific, same as [`GanttBarStyle::get_common`].
    pub fn get_common_mut(&mut self) -> &mut GanttBarCommonStyle {
        &mut self.common
    }

/// Get the field that determines this bar's start date.
    ///
    /// Corresponds to Java: `public FieldType getFromField()`
    pub fn get_from_field(&self) -> Option<MppFieldType> {
        self.from_field.clone()
    }

/// Set the field that determines this bar's start date.
    ///
    /// Corresponds to Java: `public void setFromField(FieldType field)`
    pub fn set_from_field(&mut self, field: Option<MppFieldType>) {
        self.from_field = field;
    }

/// Get the ID of this bar style.
    ///
    /// Corresponds to Java: `public Integer getID()`
    pub fn get_id(&self) -> Option<i32> {
        self.id
    }

/// Set the ID of this bar style.
    ///
    /// Corresponds to Java: `public void setID(Integer value)`
    pub fn set_id(&mut self, value: Option<i32>) {
        self.id = value;
    }

/// Get the style name.
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

/// Set the style name.
    ///
    /// Corresponds to Java: `public void setName(String name)`
    pub fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }

/// Get the row number of this bar.
    ///
    /// Corresponds to Java: `public int getRow()`
    pub fn get_row(&self) -> i32 {
        self.row
    }

/// Set the row number of this bar.
    ///
    /// Corresponds to Java: `public void setRow(int row)`
    pub fn set_row(&mut self, row: i32) {
        self.row = row;
    }

/// Get the field that determines this bar's end date.
    ///
    /// Corresponds to Java: `public FieldType getToField()`
    pub fn get_to_field(&self) -> Option<MppFieldType> {
        self.to_field.clone()
    }

/// Set the field that determines this bar's end date.
    ///
    /// Corresponds to Java: `public void setToField(FieldType field)`
    pub fn set_to_field(&mut self, field: Option<MppFieldType>) {
        self.to_field = field;
    }

/// Get the "show for" criteria set of this style.
    ///
    /// Corresponds to Java: `public Set<GanttBarShowForTasks> getShowForTasks()`
    pub fn get_show_for_tasks(&self) -> &BTreeSet<GanttBarShowForTasks> {
        &self.show_for_tasks
    }

/// Append a "show for" criterion.
    ///
    /// Corresponds to Java: `public void addShowForTasks(GanttBarShowForTasks tasks)`
    pub fn add_show_for_tasks(&mut self, tasks: GanttBarShowForTasks) {
        self.show_for_tasks.insert(tasks);
    }

// ---- The following forward members of `GanttBarCommonStyle` (Java's base class) ----

/// Forward [`GanttBarCommonStyle::get_bottom_text`].
    ///
/// Corresponds to Java: `public FieldType getBottomText()` inherited from `GanttBarCommonStyle`
    pub fn get_bottom_text(&self) -> Option<MppFieldType> {
        self.common.get_bottom_text()
    }

/// Forward [`GanttBarCommonStyle::set_bottom_text`].
    ///
/// Corresponds to Java: `public void setBottomText(FieldType field)` inherited from `GanttBarCommonStyle`
    pub fn set_bottom_text(&mut self, field: Option<MppFieldType>) {
        self.common.set_bottom_text(field);
    }

/// Forward [`GanttBarCommonStyle::get_end_color`].
    ///
/// Corresponds to Java: `public Color getEndColor()` inherited from `GanttBarCommonStyle`
    pub fn get_end_color(&self) -> Option<Color> {
        self.common.get_end_color()
    }

/// Forward [`GanttBarCommonStyle::set_end_color`].
    ///
/// Corresponds to Java: `public void setEndColor(Color color)` inherited from `GanttBarCommonStyle`
    pub fn set_end_color(&mut self, color: Option<Color>) {
        self.common.set_end_color(color);
    }

/// Forward [`GanttBarCommonStyle::get_inside_text`].
    ///
/// Corresponds to Java: `public FieldType getInsideText()` inherited from `GanttBarCommonStyle`
    pub fn get_inside_text(&self) -> Option<MppFieldType> {
        self.common.get_inside_text()
    }

/// Forward [`GanttBarCommonStyle::set_inside_text`].
    ///
/// Corresponds to Java: `public void setInsideText(FieldType field)` inherited from `GanttBarCommonStyle`
    pub fn set_inside_text(&mut self, field: Option<MppFieldType>) {
        self.common.set_inside_text(field);
    }

/// Forward [`GanttBarCommonStyle::get_left_text`].
    ///
/// Corresponds to Java: `public FieldType getLeftText()` inherited from `GanttBarCommonStyle`
    pub fn get_left_text(&self) -> Option<MppFieldType> {
        self.common.get_left_text()
    }

/// Forward [`GanttBarCommonStyle::set_left_text`].
    ///
/// Corresponds to Java: `public void setLeftText(FieldType field)` inherited from `GanttBarCommonStyle`
    pub fn set_left_text(&mut self, field: Option<MppFieldType>) {
        self.common.set_left_text(field);
    }

/// Forward [`GanttBarCommonStyle::get_middle_color`].
    ///
/// Corresponds to Java: `public Color getMiddleColor()` inherited from `GanttBarCommonStyle`
    pub fn get_middle_color(&self) -> Option<Color> {
        self.common.get_middle_color()
    }

/// Forward [`GanttBarCommonStyle::set_middle_color`].
    ///
/// Corresponds to Java: `public void setMiddleColor(Color color)` inherited from `GanttBarCommonStyle`
    pub fn set_middle_color(&mut self, color: Option<Color>) {
        self.common.set_middle_color(color);
    }

/// Forward [`GanttBarCommonStyle::get_middle_pattern`].
    ///
/// Corresponds to Java: `public ChartPattern getMiddlePattern()` inherited from `GanttBarCommonStyle`
    pub fn get_middle_pattern(&self) -> Option<ChartPattern> {
        self.common.get_middle_pattern()
    }

/// Forward [`GanttBarCommonStyle::set_middle_pattern`].
    ///
/// Corresponds to Java: `public void setMiddlePattern(ChartPattern pattern)` inherited from `GanttBarCommonStyle`
    pub fn set_middle_pattern(&mut self, pattern: Option<ChartPattern>) {
        self.common.set_middle_pattern(pattern);
    }

/// Forward [`GanttBarCommonStyle::get_middle_shape`].
    ///
/// Corresponds to Java: `public GanttBarMiddleShape getMiddleShape()` inherited from `GanttBarCommonStyle`
    pub fn get_middle_shape(&self) -> Option<GanttBarMiddleShape> {
        self.common.get_middle_shape()
    }

/// Forward [`GanttBarCommonStyle::set_middle_shape`].
    ///
/// Corresponds to Java: `public void setMiddleShape(GanttBarMiddleShape shape)` inherited from `GanttBarCommonStyle`
    pub fn set_middle_shape(&mut self, shape: Option<GanttBarMiddleShape>) {
        self.common.set_middle_shape(shape);
    }

/// Forward [`GanttBarCommonStyle::get_right_text`].
    ///
/// Corresponds to Java: `public FieldType getRightText()` inherited from `GanttBarCommonStyle`
    pub fn get_right_text(&self) -> Option<MppFieldType> {
        self.common.get_right_text()
    }

/// Forward [`GanttBarCommonStyle::set_right_text`].
    ///
/// Corresponds to Java: `public void setRightText(FieldType field)` inherited from `GanttBarCommonStyle`
    pub fn set_right_text(&mut self, field: Option<MppFieldType>) {
        self.common.set_right_text(field);
    }

/// Forward [`GanttBarCommonStyle::get_start_color`].
    ///
/// Corresponds to Java: `public Color getStartColor()` inherited from `GanttBarCommonStyle`
    pub fn get_start_color(&self) -> Option<Color> {
        self.common.get_start_color()
    }

/// Forward [`GanttBarCommonStyle::set_start_color`].
    ///
/// Corresponds to Java: `public void setStartColor(Color color)` inherited from `GanttBarCommonStyle`
    pub fn set_start_color(&mut self, color: Option<Color>) {
        self.common.set_start_color(color);
    }

/// Forward [`GanttBarCommonStyle::get_start_shape`].
    ///
/// Corresponds to Java: `public GanttBarStartEndShape getStartShape()` inherited from `GanttBarCommonStyle`
    pub fn get_start_shape(&self) -> Option<GanttBarStartEndShape> {
        self.common.get_start_shape()
    }

/// Forward [`GanttBarCommonStyle::set_start_shape`].
    ///
/// Corresponds to Java: `public void setStartShape(GanttBarStartEndShape shape)` inherited from `GanttBarCommonStyle`
    pub fn set_start_shape(&mut self, shape: Option<GanttBarStartEndShape>) {
        self.common.set_start_shape(shape);
    }

/// Forward [`GanttBarCommonStyle::get_end_shape`].
    ///
/// Corresponds to Java: `public GanttBarStartEndShape getEndShape()` inherited from `GanttBarCommonStyle`
    pub fn get_end_shape(&self) -> Option<GanttBarStartEndShape> {
        self.common.get_end_shape()
    }

/// Forward [`GanttBarCommonStyle::set_end_shape`].
    ///
/// Corresponds to Java: `public void setEndShape(GanttBarStartEndShape shape)` inherited from `GanttBarCommonStyle`
    pub fn set_end_shape(&mut self, shape: Option<GanttBarStartEndShape>) {
        self.common.set_end_shape(shape);
    }

/// Forward [`GanttBarCommonStyle::get_start_type`].
    ///
/// Corresponds to Java: `public GanttBarStartEndType getStartType()` inherited from `GanttBarCommonStyle`
    pub fn get_start_type(&self) -> Option<GanttBarStartEndType> {
        self.common.get_start_type()
    }

/// Forward [`GanttBarCommonStyle::set_start_type`].
    ///
/// Corresponds to Java: `public void setStartType(GanttBarStartEndType type)` inherited from `GanttBarCommonStyle`
    pub fn set_start_type(&mut self, r#type: Option<GanttBarStartEndType>) {
        self.common.set_start_type(r#type);
    }

/// Forward [`GanttBarCommonStyle::get_end_type`].
    ///
/// Corresponds to Java: `public GanttBarStartEndType getEndType()` inherited from `GanttBarCommonStyle`
    pub fn get_end_type(&self) -> Option<GanttBarStartEndType> {
        self.common.get_end_type()
    }

/// Forward [`GanttBarCommonStyle::set_end_type`].
    ///
/// Corresponds to Java: `public void setEndType(GanttBarStartEndType type)` inherited from `GanttBarCommonStyle`
    pub fn set_end_type(&mut self, r#type: Option<GanttBarStartEndType>) {
        self.common.set_end_type(r#type);
    }

/// Forward [`GanttBarCommonStyle::get_top_text`].
    ///
/// Corresponds to Java: `public FieldType getTopText()` inherited from `GanttBarCommonStyle`
    pub fn get_top_text(&self) -> Option<MppFieldType> {
        self.common.get_top_text()
    }

/// Forward [`GanttBarCommonStyle::set_top_text`].
    ///
/// Corresponds to Java: `public void setTopText(FieldType field)` inherited from `GanttBarCommonStyle`
    pub fn set_top_text(&mut self, field: Option<MppFieldType>) {
        self.common.set_top_text(field);
    }
}

impl Display for GanttBarStyle {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java output: a fixed-indentation property block in which `ShowForTasks` uses `TreeSet.toString()`
/// (elements in declaration order, separated by `", "` and wrapped in square brackets), followed by the inlined base class `toString()`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "   [GanttBarStyle")?;
        writeln!(f, "      ID={}", format_option(self.id))?;
        writeln!(f, "      Name={}", format_option_str(self.name.as_deref()))?;
        writeln!(f, "      FromField={}", format_field(self.from_field.as_ref()))?;
        writeln!(f, "      ToField={}", format_field(self.to_field.as_ref()))?;
        writeln!(f, "      Row={}", self.row)?;
        writeln!(
            f,
            "      ShowForTasks={}",
            format_show_for_tasks(&self.show_for_tasks)
        )?;
        write!(f, "{}", self.common)?;
        writeln!(f, "   ]")
    }
}

/// Print a nullable integer; `None` corresponds to Java's `null`.
fn format_option(value: Option<i32>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

/// Print a nullable string; `None` corresponds to Java's `null`.
fn format_option_str(value: Option<&str>) -> String {
    match value {
        Some(value) => value.to_owned(),
        None => "null".to_owned(),
    }
}

/// Print a nullable field type; `None` corresponds to Java's `null`.
fn format_field(value: Option<&MppFieldType>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

/// Print the "show for" criteria set, replicating the format of Java's `TreeSet.toString()`.
fn format_show_for_tasks(tasks: &BTreeSet<GanttBarShowForTasks>) -> String {
    let mut result = String::from("[");
    for (index, task) in tasks.iter().enumerate() {
        if index != 0 {
            result.push_str(", ");
        }
        result.push_str(task.get_name());
    }
    result.push(']');
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_key::FieldKey;
    use crate::mpxj_enum::MpxjEnum;
    use crate::task_field::TaskField;

    #[test]
    fn defaults_match_java_null_fields() {
        let style = GanttBarStyle::new();

        assert_eq!(style.get_id(), None);
        assert_eq!(style.get_name(), None);
        assert!(style.get_from_field().is_none());
        assert!(style.get_to_field().is_none());
        assert_eq!(style.get_row(), 0);
        assert!(style.get_show_for_tasks().is_empty());
    }

    #[test]
    fn show_for_tasks_is_a_sorted_set() {
        let mut style = GanttBarStyle::new();

// Insertion order is shuffled; output still follows the Java enum declaration order
        style.add_show_for_tasks(GanttBarShowForTasks::Normal);
        style.add_show_for_tasks(GanttBarShowForTasks::Milestone);
        style.add_show_for_tasks(GanttBarShowForTasks::Normal);

        let tasks: Vec<i32> = style
            .get_show_for_tasks()
            .iter()
            .map(|task| task.value())
            .collect();
        assert_eq!(tasks, vec![0, 1]);
    }

    #[test]
    fn inherited_accessors_are_forwarded() {
        let mut style = GanttBarStyle::new();

        style.set_start_shape(Some(GanttBarStartEndShape::Diamond));
        style.set_middle_color(Some(Color::new(10, 20, 30)));
        style.set_left_text(Some(MppFieldType::Builtin(FieldKey::Task(TaskField::Name))));

        assert_eq!(
            style.get_common().get_start_shape(),
            Some(GanttBarStartEndShape::Diamond)
        );
        assert_eq!(style.get_middle_color(), Some(Color::new(10, 20, 30)));
        assert_eq!(
            style.get_left_text().and_then(|field| field.as_builtin()),
            Some(FieldKey::Task(TaskField::Name))
        );
    }

    #[test]
    fn to_string_matches_java_layout() {
        let mut style = GanttBarStyle::new();
        style.set_id(Some(1));
        style.set_name(Some("Task".to_owned()));
        style.set_row(2);
        style.add_show_for_tasks(GanttBarShowForTasks::Milestone);
        style.add_show_for_tasks(GanttBarShowForTasks::Normal);

        let text = style.to_string();
        assert!(text.starts_with("   [GanttBarStyle\n"));
        assert!(text.contains("      ID=1\n"));
        assert!(text.contains("      Name=Task\n"));
        assert!(text.contains("      FromField=null\n"));
        assert!(text.contains("      ToField=null\n"));
        assert!(text.contains("      Row=2\n"));
        assert!(text.contains("      ShowForTasks=[Normal, Milestone]\n"));
        assert!(text.contains("      StartShape=null\n"));
        assert!(text.ends_with("      InsideText=null\n   ]\n"));
    }

    #[test]
    fn to_string_prints_null_values_like_java() {
        let style = GanttBarStyle::new();
        let text = style.to_string();

        assert!(text.contains("      ID=null\n"));
        assert!(text.contains("      Name=null\n"));
        assert!(text.contains("      ShowForTasks=[]\n"));
    }
}
