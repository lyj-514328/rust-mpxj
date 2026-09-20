//! 对应 Java: `org/mpxj/mpp/GanttBarCommonStyle.java`
//!
//! 甘特图条形样式（普通样式与单任务例外样式）**共有**的那部分属性：
//! 起点 / 中段 / 终点三段的形状、样式、颜色与图案，以及左上右下与条内文本字段。
//!
//! # 移植取舍
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `java.awt.Color` | [`crate::mpp::mpp_utility::Color`] | `java.awt.Color` 属 GUI 类型（跳过项），本工程已在 `MPPUtility` 内定义等价的最小颜色类型 |
//! | `FieldType`（接口，可为 null） | `Option<MppFieldType>` | `FieldType` 在 Rust 侧用 [`MppFieldType`] 表达（内置字段 / 自定义字段两种情形），Java 的 `null` 对应 `None` |
//! | 字段 `private X m_xxx;`（默认 null） | 字段 `Option<X>` | 同上，Java 的 0 / null 初值在 Rust 侧为 `None` |
//! | `getXxx()` 返回 `FieldType` 引用 | `get_xxx(&self) -> Option<MppFieldType>` | Rust 侧返回克隆值（[`MppFieldType`] 内部持有 `Rc`，克隆代价低），避免把借用绑在 `&self` 上 |
//! | `toString()` 里的 `PrintWriter` / `ByteArrayOutputStream` | `impl Display` | 输出内容逐行保持一致 |
//! | `java.awt.Color.toString()` 的 `java.awt.Color[r=..,g=..,b=..]` | 私有函数 `format_color` | 复刻 Java 的打印格式，使 `Display` 输出与原版一致 |
//!
//! 继承关系：Java 的 `GanttBarStyle` / `GanttBarStyleException` 均 `extends GanttBarCommonStyle`；
//! Rust 无继承，两个子类以**组合 + 转发**的方式持有本类型（见各自模块的文件头）。
//!
//! 跳过的 Java 成员：无（`java.io.PrintWriter` / `ByteArrayOutputStream` 属纯粹的输出管道，
//! 由 `Display` 取代）。

use std::fmt::{self, Display, Formatter};

use crate::common::mpp_field_type::MppFieldType;
use crate::mpp::chart_pattern::ChartPattern;
use crate::mpp::gantt_bar_middle_shape::GanttBarMiddleShape;
use crate::mpp::gantt_bar_start_end_shape::GanttBarStartEndShape;
use crate::mpp::gantt_bar_start_end_type::GanttBarStartEndType;
use crate::mpp::mpp_utility::{format_color, Color};

/// 甘特图条形样式（普通 / 例外）的共有部分。
///
/// 对应 Java: `public class GanttBarCommonStyle`
///
/// 说明：未派生 `Debug` —— 文本字段的类型 [`MppFieldType`] 未实现 `Debug`
/// （`Rc<RefCell<UserDefinedField>>` 一类内部结构），Java 侧该类也没有调试输出。
#[derive(Clone, Default)]
pub struct GanttBarCommonStyle {
    /// 对应 Java: `private GanttBarStartEndShape m_startShape`
    start_shape: Option<GanttBarStartEndShape>,
    /// 对应 Java: `private GanttBarStartEndType m_startType`
    start_type: Option<GanttBarStartEndType>,
    /// 对应 Java: `private Color m_startColor`
    start_color: Option<Color>,

    /// 对应 Java: `private GanttBarMiddleShape m_middleShape`
    middle_shape: Option<GanttBarMiddleShape>,
    /// 对应 Java: `private ChartPattern m_middlePattern`
    middle_pattern: Option<ChartPattern>,
    /// 对应 Java: `private Color m_middleColor`
    middle_color: Option<Color>,

    /// 对应 Java: `private GanttBarStartEndShape m_endShape`
    end_shape: Option<GanttBarStartEndShape>,
    /// 对应 Java: `private GanttBarStartEndType m_endType`
    end_type: Option<GanttBarStartEndType>,
    /// 对应 Java: `private Color m_endColor`
    end_color: Option<Color>,

    /// 对应 Java: `private FieldType m_leftText`
    left_text: Option<MppFieldType>,
    /// 对应 Java: `private FieldType m_rightText`
    right_text: Option<MppFieldType>,
    /// 对应 Java: `private FieldType m_topText`
    top_text: Option<MppFieldType>,
    /// 对应 Java: `private FieldType m_bottomText`
    bottom_text: Option<MppFieldType>,
    /// 对应 Java: `private FieldType m_insideText`
    inside_text: Option<MppFieldType>,
}

impl GanttBarCommonStyle {
    /// 构造空的共有样式。
    ///
    /// 对应 Java 的隐式默认构造方法 `GanttBarCommonStyle()`
    pub fn new() -> GanttBarCommonStyle {
        GanttBarCommonStyle::default()
    }

    /// 取条形底部显示的文本字段。
    ///
    /// 对应 Java: `public FieldType getBottomText()`
    pub fn get_bottom_text(&self) -> Option<MppFieldType> {
        self.bottom_text.clone()
    }

    /// 设置条形底部显示的文本字段。
    ///
    /// 对应 Java: `public void setBottomText(FieldType field)`
    pub fn set_bottom_text(&mut self, field: Option<MppFieldType>) {
        self.bottom_text = field;
    }

    /// 取条形终点区域的颜色。
    ///
    /// 对应 Java: `public Color getEndColor()`
    pub fn get_end_color(&self) -> Option<Color> {
        self.end_color
    }

    /// 设置条形终点区域的颜色。
    ///
    /// 对应 Java: `public void setEndColor(Color color)`
    pub fn set_end_color(&mut self, color: Option<Color>) {
        self.end_color = color;
    }

    /// 取条形内部显示的文本字段。
    ///
    /// 对应 Java: `public FieldType getInsideText()`
    pub fn get_inside_text(&self) -> Option<MppFieldType> {
        self.inside_text.clone()
    }

    /// 设置条形内部显示的文本字段。
    ///
    /// 对应 Java: `public void setInsideText(FieldType field)`
    pub fn set_inside_text(&mut self, field: Option<MppFieldType>) {
        self.inside_text = field;
    }

    /// 取条形左侧显示的文本字段。
    ///
    /// 对应 Java: `public FieldType getLeftText()`
    pub fn get_left_text(&self) -> Option<MppFieldType> {
        self.left_text.clone()
    }

    /// 设置条形左侧显示的文本字段。
    ///
    /// 对应 Java: `public void setLeftText(FieldType field)`
    pub fn set_left_text(&mut self, field: Option<MppFieldType>) {
        self.left_text = field;
    }

    /// 取条形中段的颜色。
    ///
    /// 对应 Java: `public Color getMiddleColor()`
    pub fn get_middle_color(&self) -> Option<Color> {
        self.middle_color
    }

    /// 设置条形中段的颜色。
    ///
    /// 对应 Java: `public void setMiddleColor(Color color)`
    pub fn set_middle_color(&mut self, color: Option<Color>) {
        self.middle_color = color;
    }

    /// 取条形中段的填充图案。
    ///
    /// 对应 Java: `public ChartPattern getMiddlePattern()`
    pub fn get_middle_pattern(&self) -> Option<ChartPattern> {
        self.middle_pattern
    }

    /// 设置条形中段的填充图案。
    ///
    /// 对应 Java: `public void setMiddlePattern(ChartPattern pattern)`
    pub fn set_middle_pattern(&mut self, pattern: Option<ChartPattern>) {
        self.middle_pattern = pattern;
    }

    /// 取条形中段的形状。
    ///
    /// 对应 Java: `public GanttBarMiddleShape getMiddleShape()`
    pub fn get_middle_shape(&self) -> Option<GanttBarMiddleShape> {
        self.middle_shape
    }

    /// 设置条形中段的形状。
    ///
    /// 对应 Java: `public void setMiddleShape(GanttBarMiddleShape shape)`
    pub fn set_middle_shape(&mut self, shape: Option<GanttBarMiddleShape>) {
        self.middle_shape = shape;
    }

    /// 取条形右侧显示的文本字段。
    ///
    /// 对应 Java: `public FieldType getRightText()`
    pub fn get_right_text(&self) -> Option<MppFieldType> {
        self.right_text.clone()
    }

    /// 设置条形右侧显示的文本字段。
    ///
    /// 对应 Java: `public void setRightText(FieldType field)`
    pub fn set_right_text(&mut self, field: Option<MppFieldType>) {
        self.right_text = field;
    }

    /// 取条形起点区域的颜色。
    ///
    /// 对应 Java: `public Color getStartColor()`
    pub fn get_start_color(&self) -> Option<Color> {
        self.start_color
    }

    /// 设置条形起点区域的颜色。
    ///
    /// 对应 Java: `public void setStartColor(Color color)`
    pub fn set_start_color(&mut self, color: Option<Color>) {
        self.start_color = color;
    }

    /// 取条形起点的形状。
    ///
    /// 对应 Java: `public GanttBarStartEndShape getStartShape()`
    pub fn get_start_shape(&self) -> Option<GanttBarStartEndShape> {
        self.start_shape
    }

    /// 设置条形起点的形状。
    ///
    /// 对应 Java: `public void setStartShape(GanttBarStartEndShape shape)`
    pub fn set_start_shape(&mut self, shape: Option<GanttBarStartEndShape>) {
        self.start_shape = shape;
    }

    /// 取条形终点的形状。
    ///
    /// 对应 Java: `public GanttBarStartEndShape getEndShape()`
    pub fn get_end_shape(&self) -> Option<GanttBarStartEndShape> {
        self.end_shape
    }

    /// 设置条形终点的形状。
    ///
    /// 对应 Java: `public void setEndShape(GanttBarStartEndShape shape)`
    pub fn set_end_shape(&mut self, shape: Option<GanttBarStartEndShape>) {
        self.end_shape = shape;
    }

    /// 取条形起点的样式。
    ///
    /// 对应 Java: `public GanttBarStartEndType getStartType()`
    pub fn get_start_type(&self) -> Option<GanttBarStartEndType> {
        self.start_type
    }

    /// 设置条形起点的样式。
    ///
    /// 对应 Java: `public void setStartType(GanttBarStartEndType type)`
    pub fn set_start_type(&mut self, r#type: Option<GanttBarStartEndType>) {
        self.start_type = r#type;
    }

    /// 取条形终点的样式。
    ///
    /// 对应 Java: `public GanttBarStartEndType getEndType()`
    pub fn get_end_type(&self) -> Option<GanttBarStartEndType> {
        self.end_type
    }

    /// 设置条形终点的样式。
    ///
    /// 对应 Java: `public void setEndType(GanttBarStartEndType type)`
    pub fn set_end_type(&mut self, r#type: Option<GanttBarStartEndType>) {
        self.end_type = r#type;
    }

    /// 取条形上方显示的文本字段。
    ///
    /// 对应 Java: `public FieldType getTopText()`
    pub fn get_top_text(&self) -> Option<MppFieldType> {
        self.top_text.clone()
    }

    /// 设置条形上方显示的文本字段。
    ///
    /// 对应 Java: `public void setTopText(FieldType field)`
    pub fn set_top_text(&mut self, field: Option<MppFieldType>) {
        self.top_text = field;
    }
}

impl Display for GanttBarCommonStyle {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// 逐行输出各属性，格式与 Java 的 `PrintWriter.println` 完全一致（含结尾换行）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "      StartShape={}", format_option(self.start_shape))?;
        writeln!(f, "      StartType={}", format_option(self.start_type))?;
        writeln!(f, "      StartColor={}", format_color(self.start_color))?;
        writeln!(f, "      MiddleShape={}", format_option(self.middle_shape))?;
        writeln!(f, "      MiddlePattern={}", format_option(self.middle_pattern))?;
        writeln!(f, "      MiddleColor={}", format_color(self.middle_color))?;
        writeln!(f, "      EndShape={}", format_option(self.end_shape))?;
        writeln!(f, "      EndType={}", format_option(self.end_type))?;
        writeln!(f, "      EndColor={}", format_color(self.end_color))?;
        writeln!(f, "      LeftText={}", format_field(self.left_text.as_ref()))?;
        writeln!(f, "      RightText={}", format_field(self.right_text.as_ref()))?;
        writeln!(f, "      TopText={}", format_field(self.top_text.as_ref()))?;
        writeln!(f, "      BottomText={}", format_field(self.bottom_text.as_ref()))?;
        writeln!(f, "      InsideText={}", format_field(self.inside_text.as_ref()))
    }
}

/// 打印可空的枚举 / 模式值，`None` 对应 Java 的 `null`。
fn format_option<T: Display>(value: Option<T>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

/// 打印可空的文本字段，`None` 对应 Java 的 `null`。
///
/// 说明：Java 打印的是枚举常量的 `name()`（如 `START`），Rust 的 [`MppFieldType`]
/// 以 [`Display`] 暴露显示名（如 `Start`），两者大小写风格不同。
fn format_field(value: Option<&MppFieldType>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
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
