//! 对应 Java: `org/mpxj/mpp/GridLines.java`
//!
//! 一组网格线的位置与外观属性（常规线 + 间隔线）。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `public class GridLines` | 结构体 [`GridLines`] | 1:1 移植；字段顺序与 Java 一致 |
//! | 构造参数 `Color normalLineColor` / `Color intervalLineColor`（`java.awt.Color`） | `Option<Color>`，`Color` = [`crate::mpp::mpp_utility::Color`] | `java.awt.*` 属跳过项；Rust 侧复用 [`crate::mpp::mpp_utility::Color`]。Java 侧由 `MPPUtility.getColor(...)` / `ColorType.getColor()` 取值，两者都可能返回 `null`（自动颜色），故用 `Option` |
//! | 构造参数 `LineStyle normalLineStyle` / `LineStyle intervalLineStyle` | `Option<LineStyle>` | Java 侧由 `LineStyle.getInstance(...)` 取值，该方法的查找表下标 5 为 `null`（见 `crate::mpp::line_style::LineStyle`），故用 `Option` |
//! | 构造参数 `int intervalNumber` | `i32` | Java 侧该值直接来自文件中的**有符号** `byte`（如 `GanttChartView9#getGridLines` 的 `data[offset + 4]`），因此可能出现负数；Rust 侧同样是 `i32`，符号扩展由调用方负责 |
//! | `toString()` | `Display` | 输出格式与 Java 一致；`java.awt.Color` 部分按 `java.awt.Color[r=..,g=..,b=..]` 形式输出，`null` 输出 `null` |
//! | 未覆写 `equals` / `hashCode` | 派生 `PartialEq` / `Eq` | Rust 侧为便于比较与测试而派生（逐字段比较）；因 [`crate::mpp::mpp_utility::Color`] 未实现 `Hash`，故**不**派生 `Hash` |
//!
//! 未移植 / 跳过：无（JDBC / XML-JAXB / GUI / `javax.*` / `Serializable` 等机制本类未使用）。

use std::fmt::{self, Display, Formatter};

use crate::mpp::line_style::LineStyle;
use crate::mpp::mpp_utility::Color;

/// 一组网格线的属性。
///
/// 对应 Java: `public class GridLines`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridLines {
    /// 常规线颜色。
    ///
    /// 对应 Java: `private final Color m_normalLineColor`
    normal_line_color: Option<Color>,
    /// 常规线线型。
    ///
    /// 对应 Java: `private final LineStyle m_normalLineStyle`
    normal_line_style: Option<LineStyle>,
    /// 间隔线间隔数。
    ///
    /// 对应 Java: `private final int m_intervalNumber`
    interval_number: i32,
    /// 间隔线线型。
    ///
    /// 对应 Java: `private final LineStyle m_intervalLineStyle`
    interval_line_style: Option<LineStyle>,
    /// 间隔线颜色。
    ///
    /// 对应 Java: `private final Color m_intervalLineColor`
    interval_line_color: Option<Color>,
}

impl GridLines {
    /// 构造函数。
    ///
    /// 对应 Java:
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

    /// 取间隔线颜色。
    ///
    /// 对应 Java: `public Color getIntervalLineColor()`
    pub fn get_interval_line_color(&self) -> Option<Color> {
        self.interval_line_color
    }

    /// 取间隔线线型。
    ///
    /// 对应 Java: `public LineStyle getIntervalLineStyle()`
    pub fn get_interval_line_style(&self) -> Option<LineStyle> {
        self.interval_line_style
    }

    /// 取间隔线间隔数。
    ///
    /// 对应 Java: `public int getIntervalNumber()`
    pub fn get_interval_number(&self) -> i32 {
        self.interval_number
    }

    /// 取常规线颜色。
    ///
    /// 对应 Java: `public Color getNormalLineColor()`
    pub fn get_normal_line_color(&self) -> Option<Color> {
        self.normal_line_color
    }

    /// 取常规线线型。
    ///
    /// 对应 Java: `public LineStyle getNormalLineStyle()`
    pub fn get_normal_line_style(&self) -> Option<LineStyle> {
        self.normal_line_style
    }
}

impl Display for GridLines {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// 逐字段拼接，顺序与 Java 完全一致。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[GridLines NormalLineColor={} NormalLineStyle={} IntervalNumber={} IntervalLineStyle={} IntervalLineColor={}]",
            format_color(self.normal_line_color),
            format_optional_display(self.normal_line_style),
            self.interval_number,
            format_optional_display(self.interval_line_style),
            format_color(self.interval_line_color)
        )
    }
}

/// 把 `Option<Color>` 格式化成 Java 的 `java.awt.Color.toString()` 形式（`None` 输出 `null`）。
///
/// 说明：与 `crate::mpp::font_style` 中的同名辅助函数重复，原因是要保持
/// [`crate::mpp::mpp_utility::Color`]（已落地文件，不得修改）不含 `Display`，
/// 且本批不允许新增模块来放置公共辅助函数。
fn format_color(color: Option<Color>) -> String {
    match color {
        Some(color) => format!(
            "java.awt.Color[r={},g={},b={}]",
            color.get_red(),
            color.get_green(),
            color.get_blue()
        ),
        None => "null".to_owned(),
    }
}

/// 把可选对象按 Java 的字符串拼接语义输出（`None` 输出 `null`）。
fn format_optional_display<T: Display>(value: Option<T>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// getter 与 Java 一致。
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

    /// Java 中可能为 null 的成员在 Rust 侧是 `Option`；间隔数可为负（有符号 byte）。
    #[test]
    fn nullable_members_are_options() {
        // Java: LineStyle.getInstance(5) 返回 null，MPPUtility.getColor 也可能返回 null
        let grid_lines = GridLines::new(None, None, -1, None, None);
        assert_eq!(grid_lines.get_normal_line_color(), None);
        assert_eq!(grid_lines.get_normal_line_style(), None);
        assert_eq!(grid_lines.get_interval_line_style(), None);
        assert_eq!(grid_lines.get_interval_line_color(), None);
        assert_eq!(grid_lines.get_interval_number(), -1);
    }

    /// toString 格式与 Java 一致。
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
