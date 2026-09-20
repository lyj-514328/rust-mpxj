//! 对应 Java: `org/mpxj/mpp/GanttBarStyleException.java`
//!
//! 甘特图条形的**单任务例外样式**：某条任务不使用默认条形样式，而是改用指定 ID 的样式。
//!
//! # 移植取舍
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `GanttBarStyleException extends GanttBarCommonStyle` | 字段 `common: GanttBarCommonStyle` + **转发方法** | Rust 无继承，改用组合（同 `GanttBarStyle`） |
//! | `Integer m_ganttBarStyleID` | `Option<i32>` | Java 的 `null` 初值 |
//! | `int m_taskUniqueID` / `int m_barStyleIndex` | `i32` | Java 的 0 初值 |
//! | `toString()` | `impl Display` | 输出格式与 Java 一致 |
//!
//! **Java 只提供了 `setBarStyleIndex`，没有对应的 getter**（上游 MPXJ 如此），
//! Rust 侧 1:1 保留，未额外补充 `get_bar_style_index`（字段可通过 `Display` 观察）。
//!
//! 跳过的 Java 成员：无（`ByteArrayOutputStream` / `PrintWriter` 由 `Display` 取代）。

use std::fmt::{self, Display, Formatter};

use crate::common::mpp_field_type::MppFieldType;
use crate::mpp::chart_pattern::ChartPattern;
use crate::mpp::gantt_bar_common_style::GanttBarCommonStyle;
use crate::mpp::gantt_bar_middle_shape::GanttBarMiddleShape;
use crate::mpp::gantt_bar_start_end_shape::GanttBarStartEndShape;
use crate::mpp::gantt_bar_start_end_type::GanttBarStartEndType;
use crate::mpp::mpp_utility::Color;

/// 甘特图条形的单任务例外样式。
///
/// 对应 Java: `public final class GanttBarStyleException extends GanttBarCommonStyle`
///
/// 说明：未派生 `Debug`（原因同 [`GanttBarCommonStyle`]）。
#[derive(Clone, Default)]
pub struct GanttBarStyleException {
    /// 对应 Java: `extends GanttBarCommonStyle`（组合替代表达继承）
    common: GanttBarCommonStyle,
    /// 对应 Java: `private int m_taskUniqueID`
    task_unique_id: i32,
    /// 对应 Java: `private int m_barStyleIndex`
    bar_style_index: i32,
    /// 对应 Java: `private Integer m_ganttBarStyleID`
    gantt_bar_style_id: Option<i32>,
}

impl GanttBarStyleException {
    /// 构造空的例外样式。
    ///
    /// 对应 Java 的隐式默认构造方法 `GanttBarStyleException()`
    pub fn new() -> GanttBarStyleException {
        GanttBarStyleException::default()
    }

    /// 取内部共有样式实例。
    ///
    /// Rust 特有：Java 通过继承直接暴露父类成员，组合方式下需要显式取得。
    pub fn get_common(&self) -> &GanttBarCommonStyle {
        &self.common
    }

    /// 取内部共有样式实例（可变）。
    ///
    /// Rust 特有，同 [`GanttBarStyleException::get_common`]。
    pub fn get_common_mut(&mut self) -> &mut GanttBarCommonStyle {
        &mut self.common
    }

    /// 取本例外样式适用的任务唯一 ID。
    ///
    /// 对应 Java: `public int getTaskUniqueID()`
    pub fn get_task_unique_id(&self) -> i32 {
        self.task_unique_id
    }

    /// 设置任务唯一 ID。
    ///
    /// 对应 Java: `public void setTaskUniqueID(int id)`
    pub fn set_task_unique_id(&mut self, id: i32) {
        self.task_unique_id = id;
    }

    /// 取本例外样式所引用的甘特图条形样式 ID。
    ///
    /// 对应 Java: `public Integer getGanttBarStyleID()`
    pub fn get_gantt_bar_style_id(&self) -> Option<i32> {
        self.gantt_bar_style_id
    }

    /// 设置所引用的甘特图条形样式 ID。
    ///
    /// 对应 Java: `public void setGanttBarStyleID(Integer value)`
    pub fn set_gantt_bar_style_id(&mut self, value: Option<i32>) {
        self.gantt_bar_style_id = value;
    }

    /// 设置条形样式下标。
    ///
    /// 对应 Java: `public void setBarStyleIndex(int index)`
    pub fn set_bar_style_index(&mut self, index: i32) {
        self.bar_style_index = index;
    }

    // ---- 以下为 `GanttBarCommonStyle`（Java 的父类）成员的转发 ----

    /// 转发 [`GanttBarCommonStyle::get_bottom_text`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public FieldType getBottomText()`
    pub fn get_bottom_text(&self) -> Option<MppFieldType> {
        self.common.get_bottom_text()
    }

    /// 转发 [`GanttBarCommonStyle::set_bottom_text`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public void setBottomText(FieldType field)`
    pub fn set_bottom_text(&mut self, field: Option<MppFieldType>) {
        self.common.set_bottom_text(field);
    }

    /// 转发 [`GanttBarCommonStyle::get_end_color`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public Color getEndColor()`
    pub fn get_end_color(&self) -> Option<Color> {
        self.common.get_end_color()
    }

    /// 转发 [`GanttBarCommonStyle::set_end_color`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public void setEndColor(Color color)`
    pub fn set_end_color(&mut self, color: Option<Color>) {
        self.common.set_end_color(color);
    }

    /// 转发 [`GanttBarCommonStyle::get_inside_text`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public FieldType getInsideText()`
    pub fn get_inside_text(&self) -> Option<MppFieldType> {
        self.common.get_inside_text()
    }

    /// 转发 [`GanttBarCommonStyle::set_inside_text`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public void setInsideText(FieldType field)`
    pub fn set_inside_text(&mut self, field: Option<MppFieldType>) {
        self.common.set_inside_text(field);
    }

    /// 转发 [`GanttBarCommonStyle::get_left_text`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public FieldType getLeftText()`
    pub fn get_left_text(&self) -> Option<MppFieldType> {
        self.common.get_left_text()
    }

    /// 转发 [`GanttBarCommonStyle::set_left_text`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public void setLeftText(FieldType field)`
    pub fn set_left_text(&mut self, field: Option<MppFieldType>) {
        self.common.set_left_text(field);
    }

    /// 转发 [`GanttBarCommonStyle::get_middle_color`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public Color getMiddleColor()`
    pub fn get_middle_color(&self) -> Option<Color> {
        self.common.get_middle_color()
    }

    /// 转发 [`GanttBarCommonStyle::set_middle_color`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public void setMiddleColor(Color color)`
    pub fn set_middle_color(&mut self, color: Option<Color>) {
        self.common.set_middle_color(color);
    }

    /// 转发 [`GanttBarCommonStyle::get_middle_pattern`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public ChartPattern getMiddlePattern()`
    pub fn get_middle_pattern(&self) -> Option<ChartPattern> {
        self.common.get_middle_pattern()
    }

    /// 转发 [`GanttBarCommonStyle::set_middle_pattern`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public void setMiddlePattern(ChartPattern pattern)`
    pub fn set_middle_pattern(&mut self, pattern: Option<ChartPattern>) {
        self.common.set_middle_pattern(pattern);
    }

    /// 转发 [`GanttBarCommonStyle::get_middle_shape`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public GanttBarMiddleShape getMiddleShape()`
    pub fn get_middle_shape(&self) -> Option<GanttBarMiddleShape> {
        self.common.get_middle_shape()
    }

    /// 转发 [`GanttBarCommonStyle::set_middle_shape`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public void setMiddleShape(GanttBarMiddleShape shape)`
    pub fn set_middle_shape(&mut self, shape: Option<GanttBarMiddleShape>) {
        self.common.set_middle_shape(shape);
    }

    /// 转发 [`GanttBarCommonStyle::get_right_text`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public FieldType getRightText()`
    pub fn get_right_text(&self) -> Option<MppFieldType> {
        self.common.get_right_text()
    }

    /// 转发 [`GanttBarCommonStyle::set_right_text`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public void setRightText(FieldType field)`
    pub fn set_right_text(&mut self, field: Option<MppFieldType>) {
        self.common.set_right_text(field);
    }

    /// 转发 [`GanttBarCommonStyle::get_start_color`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public Color getStartColor()`
    pub fn get_start_color(&self) -> Option<Color> {
        self.common.get_start_color()
    }

    /// 转发 [`GanttBarCommonStyle::set_start_color`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public void setStartColor(Color color)`
    pub fn set_start_color(&mut self, color: Option<Color>) {
        self.common.set_start_color(color);
    }

    /// 转发 [`GanttBarCommonStyle::get_start_shape`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public GanttBarStartEndShape getStartShape()`
    pub fn get_start_shape(&self) -> Option<GanttBarStartEndShape> {
        self.common.get_start_shape()
    }

    /// 转发 [`GanttBarCommonStyle::set_start_shape`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public void setStartShape(GanttBarStartEndShape shape)`
    pub fn set_start_shape(&mut self, shape: Option<GanttBarStartEndShape>) {
        self.common.set_start_shape(shape);
    }

    /// 转发 [`GanttBarCommonStyle::get_end_shape`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public GanttBarStartEndShape getEndShape()`
    pub fn get_end_shape(&self) -> Option<GanttBarStartEndShape> {
        self.common.get_end_shape()
    }

    /// 转发 [`GanttBarCommonStyle::set_end_shape`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public void setEndShape(GanttBarStartEndShape shape)`
    pub fn set_end_shape(&mut self, shape: Option<GanttBarStartEndShape>) {
        self.common.set_end_shape(shape);
    }

    /// 转发 [`GanttBarCommonStyle::get_start_type`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public GanttBarStartEndType getStartType()`
    pub fn get_start_type(&self) -> Option<GanttBarStartEndType> {
        self.common.get_start_type()
    }

    /// 转发 [`GanttBarCommonStyle::set_start_type`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public void setStartType(GanttBarStartEndType type)`
    pub fn set_start_type(&mut self, r#type: Option<GanttBarStartEndType>) {
        self.common.set_start_type(r#type);
    }

    /// 转发 [`GanttBarCommonStyle::get_end_type`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public GanttBarStartEndType getEndType()`
    pub fn get_end_type(&self) -> Option<GanttBarStartEndType> {
        self.common.get_end_type()
    }

    /// 转发 [`GanttBarCommonStyle::set_end_type`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public void setEndType(GanttBarStartEndType type)`
    pub fn set_end_type(&mut self, r#type: Option<GanttBarStartEndType>) {
        self.common.set_end_type(r#type);
    }

    /// 转发 [`GanttBarCommonStyle::get_top_text`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public FieldType getTopText()`
    pub fn get_top_text(&self) -> Option<MppFieldType> {
        self.common.get_top_text()
    }

    /// 转发 [`GanttBarCommonStyle::set_top_text`]。
    ///
    /// 对应 Java: 继承自 `GanttBarCommonStyle` 的 `public void setTopText(FieldType field)`
    pub fn set_top_text(&mut self, field: Option<MppFieldType>) {
        self.common.set_top_text(field);
    }
}

impl Display for GanttBarStyleException {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 输出：固定缩进的属性块，随后内联父类 `toString()`。
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
