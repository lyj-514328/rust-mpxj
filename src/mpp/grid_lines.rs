//! Corresponds to Java: `org/mpxj/mpp/GridLines.java`
//!
//! The position and appearance attributes of a set of grid lines (normal line + interval line).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `public class GridLines` | Struct [`GridLines`] | 1:1 port; field order matches Java |
//! | Constructor parameters `Color normalLineColor` / `Color intervalLineColor` (`java.awt.Color`) | `Option<Color>`, `Color` = [`crate::mpp::mpp_utility::Color`] | `java.awt.*` is a skipped item; the Rust side reuses [`crate::mpp::mpp_utility::Color`]. Java obtains the values from `MPPUtility.getColor(...)` / `ColorType.getColor()`, both of which may return `null` (automatic color), hence `Option` |
//! | Constructor parameters `LineStyle normalLineStyle` / `LineStyle intervalLineStyle` | `Option<LineStyle>` | Java obtains the values from `LineStyle.getInstance(...)`, whose lookup table index 5 is `null` (see `crate::mpp::line_style::LineStyle`), hence `Option` |
//! | Constructor parameter `int intervalNumber` | `i32` | In Java this value comes directly from a **signed** `byte` in the file (e.g. `data[offset + 4]` in `GanttChartView9#getGridLines`), so it may be negative; the Rust side is likewise `i32`, with sign extension handled by the caller |
//! | `toString()` | `Display` | Output format matches Java; the `java.awt.Color` part is output in the form `java.awt.Color[r=..,g=..,b=..]`, and `null` is output as `null` |
//! | `equals` / `hashCode` not overridden | Derive `PartialEq` / `Eq` | Derived on the Rust side for easier comparison and testing (field-by-field comparison); since [`crate::mpp::mpp_utility::Color`] does not implement `Hash`, `Hash` is **not** derived |
//!
//! Unported / skipped: none (this class does not use mechanisms such as JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`).

use std::fmt::{self, Display, Formatter};

use crate::mpp::line_style::LineStyle;
use crate::mpp::mpp_utility::{format_optional_display, Color};

/// The attributes of a set of grid lines.
///
/// Corresponds to Java: `public class GridLines`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridLines {
/// Normal line color.
    ///
    /// Corresponds to Java: `private final Color m_normalLineColor`
    normal_line_color: Option<Color>,
/// Normal line style.
    ///
    /// Corresponds to Java: `private final LineStyle m_normalLineStyle`
    normal_line_style: Option<LineStyle>,
/// Interval line interval number.
    ///
    /// Corresponds to Java: `private final int m_intervalNumber`
    interval_number: i32,
/// Interval line style.
    ///
    /// Corresponds to Java: `private final LineStyle m_intervalLineStyle`
    interval_line_style: Option<LineStyle>,
/// Interval line color.
    ///
    /// Corresponds to Java: `private final Color m_intervalLineColor`
    interval_line_color: Option<Color>,
}

impl GridLines {
/// Constructor.
    ///
    /// Corresponds to Java:
    /// `public GridLines(Color normalLineColor, LineStyle normalLineStyle, int intervalNumber, LineStyle intervalLineStyle, Color intervalLineColor)`
    pub fn new(
        normal_line_color: Option<Color>,
        normal_line_style: Option<LineStyle>,
        interval_number: i32,
        interval_line_style: Option<LineStyle>,
        interval_line_color: Option<Color>,
    ) -> GridLines {
        GridLines {
            normal_line_color,
            normal_line_style,
            interval_number,
            interval_line_style,
            interval_line_color,
        }
    }

/// Get the interval line color.
    ///
    /// Corresponds to Java: `public Color getIntervalLineColor()`
    pub fn get_interval_line_color(&self) -> Option<Color> {
        self.interval_line_color
    }

/// Get the interval line style.
    ///
    /// Corresponds to Java: `public LineStyle getIntervalLineStyle()`
    pub fn get_interval_line_style(&self) -> Option<LineStyle> {
        self.interval_line_style
    }

/// Get the interval line interval number.
    ///
    /// Corresponds to Java: `public int getIntervalNumber()`
    pub fn get_interval_number(&self) -> i32 {
        self.interval_number
    }

/// Get the normal line color.
    ///
    /// Corresponds to Java: `public Color getNormalLineColor()`
    pub fn get_normal_line_color(&self) -> Option<Color> {
        self.normal_line_color
    }

/// Get the normal line style.
    ///
    /// Corresponds to Java: `public LineStyle getNormalLineStyle()`
    pub fn get_normal_line_style(&self) -> Option<LineStyle> {
        self.normal_line_style
    }
}

impl Display for GridLines {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Concatenate field by field, with an order fully consistent with Java.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[GridLines NormalLineColor={} NormalLineStyle={} IntervalNumber={} IntervalLineStyle={} IntervalLineColor={}]",
            format_optional_display(self.normal_line_color.as_ref()),
            format_optional_display(self.normal_line_style.as_ref()),
            self.interval_number,
            format_optional_display(self.interval_line_style.as_ref()),
            format_optional_display(self.interval_line_color.as_ref())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// The getters are consistent with Java.
    #[test]
    fn accessors_match_java() {
        let grid_lines = GridLines::new(
            Some(Color::new(1, 2, 3)),
            Some(LineStyle::Solid),
            2,
            Some(LineStyle::Dashed),
            Some(Color::new(4, 5, 6)),
        );

        assert_eq!(grid_lines.get_normal_line_color(), Some(Color::new(1, 2, 3)));
        assert_eq!(grid_lines.get_normal_line_style(), Some(LineStyle::Solid));
        assert_eq!(grid_lines.get_interval_number(), 2);
        assert_eq!(
            grid_lines.get_interval_line_style(),
            Some(LineStyle::Dashed)
        );
        assert_eq!(grid_lines.get_interval_line_color(), Some(Color::new(4, 5, 6)));
    }

/// Members that may be null in Java are `Option` on the Rust side; the interval number may be negative (signed byte).
    #[test]
    fn nullable_members_are_options() {
// Java: LineStyle.getInstance(5) returns null, and MPPUtility.getColor may also return null
        let grid_lines = GridLines::new(None, None, -1, None, None);
        assert_eq!(grid_lines.get_normal_line_color(), None);
        assert_eq!(grid_lines.get_normal_line_style(), None);
        assert_eq!(grid_lines.get_interval_line_style(), None);
        assert_eq!(grid_lines.get_interval_line_color(), None);
        assert_eq!(grid_lines.get_interval_number(), -1);
    }

/// The toString format is consistent with Java.
    #[test]
    fn to_string_matches_java() {
        let grid_lines = GridLines::new(
            Some(Color::new(255, 255, 255)),
            Some(LineStyle::Solid),
            2,
            Some(LineStyle::Dotted1),
            Some(Color::new(0, 0, 0)),
        );
        assert_eq!(
            grid_lines.to_string(),
            "[GridLines NormalLineColor=java.awt.Color[r=255,g=255,b=255] NormalLineStyle=Solid IntervalNumber=2 IntervalLineStyle=Dotted1 IntervalLineColor=java.awt.Color[r=0,g=0,b=0]]"
        );

        let grid_lines = GridLines::new(None, None, 0, None, None);
        assert_eq!(
            grid_lines.to_string(),
            "[GridLines NormalLineColor=null NormalLineStyle=null IntervalNumber=0 IntervalLineStyle=null IntervalLineColor=null]"
        );
    }
}
