//! 对应 Java: `org/mpxj/mpp/GanttBarStyle.java`
//!
//! 甘特图条形的**默认样式**：在何种任务上显示、由哪两个字段决定起止日期、
//! 显示在第几行，以及从 [`GanttBarCommonStyle`] 继承来的形状 / 颜色 / 文本设置。
//!
//! # 移植取舍
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `GanttBarStyle extends GanttBarCommonStyle` | 字段 `common: GanttBarCommonStyle` + **转发方法** | Rust 无继承，改用组合：`get_xxx` / `set_xxx` 逐方法转发到 `common`，另外提供 [`GanttBarStyle::get_common`] 取内部实例 |
//! | `Integer m_id` / `String m_name` | `Option<i32>` / `Option<String>` | Java 的 `null` 初值 |
//! | `FieldType m_fromField` / `m_toField` | `Option<MppFieldType>` | 同上；`FieldType` 在 Rust 侧由 [`MppFieldType`] 表达 |
//! | `Set<GanttBarShowForTasks> m_showForTasks = new TreeSet<>()` | `BTreeSet<GanttBarShowForTasks>` | Java 的 `TreeSet` 按枚举自然序（声明序）；Rust 枚举判别值与声明序一致，`BTreeSet` 的序相同 |
//! | `toString()` | `impl Display` | 先打印自身属性，再内联父类内容，最后收尾，与 Java 输出一致 |
//!
//! 跳过的 Java 成员：无（`ByteArrayOutputStream` / `PrintWriter` 由 `Display` 取代）。

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

/// 甘特图条形的默认样式。
///
/// 对应 Java: `public final class GanttBarStyle extends GanttBarCommonStyle`
///
/// 说明：未派生 `Debug`（原因同 [`GanttBarCommonStyle`]）。
#[derive(Clone, Default)]
pub struct GanttBarStyle {
    /// 对应 Java: `extends GanttBarCommonStyle`（组合替代表达继承）
    common: GanttBarCommonStyle,
    /// 对应 Java: `private Integer m_id`
    id: Option<i32>,
    /// 对应 Java: `private String m_name`
    name: Option<String>,
    /// 对应 Java: `private FieldType m_fromField`
    from_field: Option<MppFieldType>,
    /// 对应 Java: `private FieldType m_toField`
    to_field: Option<MppFieldType>,
    /// 对应 Java: `private int m_row`
    row: i32,
    /// 对应 Java: `private final Set<GanttBarShowForTasks> m_showForTasks = new TreeSet<>()`
    show_for_tasks: BTreeSet<GanttBarShowForTasks>,
}

impl GanttBarStyle {
    /// 构造空的条形样式。
    ///
    /// 对应 Java 的隐式默认构造方法 `GanttBarStyle()`
    pub fn new() -> GanttBarStyle {
        GanttBarStyle::default()
    }

    /// 取内部共有样式实例。
    ///
    /// Rust 特有：Java 通过继承直接暴露父类成员，组合方式下需要显式取得。
    pub fn get_common(&self) -> &GanttBarCommonStyle {
        &self.common
    }

    /// 取内部共有样式实例（可变）。
    ///
    /// Rust 特有，同 [`GanttBarStyle::get_common`]。
    pub fn get_common_mut(&mut self) -> &mut GanttBarCommonStyle {
        &mut self.common
    }

    /// 取决定本条形起始日期的字段。
    ///
    /// 对应 Java: `public FieldType getFromField()`
    pub fn get_from_field(&self) -> Option<MppFieldType> {
        self.from_field.clone()
    }

    /// 设置决定本条形起始日期的字段。
    ///
    /// 对应 Java: `public void setFromField(FieldType field)`
    pub fn set_from_field(&mut self, field: Option<MppFieldType>) {
        self.from_field = field;
    }

    /// 取本条形样式的 ID。
    ///
    /// 对应 Java: `public Integer getID()`
    pub fn get_id(&self) -> Option<i32> {
        self.id
    }

    /// 设置本条形样式的 ID。
    ///
    /// 对应 Java: `public void setID(Integer value)`
    pub fn set_id(&mut self, value: Option<i32>) {
        self.id = value;
    }

    /// 取样式名称。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// 设置样式名称。
    ///
    /// 对应 Java: `public void setName(String name)`
    pub fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }

    /// 取本条形所在行号。
    ///
    /// 对应 Java: `public int getRow()`
    pub fn get_row(&self) -> i32 {
        self.row
    }

    /// 设置本条形所在行号。
    ///
    /// 对应 Java: `public void setRow(int row)`
    pub fn set_row(&mut self, row: i32) {
        self.row = row;
    }

    /// 取决定本条形结束日期的字段。
    ///
    /// 对应 Java: `public FieldType getToField()`
    pub fn get_to_field(&self) -> Option<MppFieldType> {
        self.to_field.clone()
    }

    /// 设置决定本条形结束日期的字段。
    ///
    /// 对应 Java: `public void setToField(FieldType field)`
    pub fn set_to_field(&mut self, field: Option<MppFieldType>) {
        self.to_field = field;
    }

    /// 取本样式的「显示于」判据集合。
    ///
    /// 对应 Java: `public Set<GanttBarShowForTasks> getShowForTasks()`
    pub fn get_show_for_tasks(&self) -> &BTreeSet<GanttBarShowForTasks> {
        &self.show_for_tasks
    }

    /// 追加一条「显示于」判据。
    ///
    /// 对应 Java: `public void addShowForTasks(GanttBarShowForTasks tasks)`
    pub fn add_show_for_tasks(&mut self, tasks: GanttBarShowForTasks) {
        self.show_for_tasks.insert(tasks);
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

impl Display for GanttBarStyle {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 输出：固定缩进的属性块，其中 `ShowForTasks` 用 `TreeSet.toString()`
    /// （元素按声明序、以 `", "` 分隔并加方括号），随后内联父类 `toString()`。
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

/// 打印可空的整数，`None` 对应 Java 的 `null`。
fn format_option(value: Option<i32>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

/// 打印可空的字符串，`None` 对应 Java 的 `null`。
fn format_option_str(value: Option<&str>) -> String {
    match value {
        Some(value) => value.to_owned(),
        None => "null".to_owned(),
    }
}

/// 打印可空的字段类型，`None` 对应 Java 的 `null`。
fn format_field(value: Option<&MppFieldType>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

/// 打印「显示于」判据集合，复刻 Java `TreeSet.toString()` 的格式。
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

        // 插入顺序打乱，输出仍按 Java 枚举声明序
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
