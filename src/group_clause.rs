//! 对应 Java: `org/mpxj/GroupClause.java`
//!
//! # Java 的三处 import 与其对应关系
//!
//! ```java
//! import java.awt.Color;
//!
//! import org.mpxj.mpp.BackgroundPattern;
//! import org.mpxj.mpp.FontStyle;
//! ```
//!
//! 也就是说，本类虽然位于模型层（`org.mpxj`），其字体 / 颜色 / 图案三个成员却来自
//! **MPP 读取层**（`org.mpxj.mpp`）与 `java.awt`。Rust 侧照搬这一依赖方向（模型层依赖
//! `crate::mpp`），对应关系如下：
//!
//! | Java import | Rust |
//! |---|---|
//! | `java.awt.Color` | [`crate::mpp::mpp_utility::Color`]（`java.awt.*` 属跳过项，本 crate 复用 `MPPUtility.get_color` 的返回值类型，见该模块说明） |
//! | `org.mpxj.mpp.FontStyle` | [`crate::mpp::font_style::FontStyle`] |
//! | `org.mpxj.mpp.BackgroundPattern` | [`crate::mpp::background_pattern::BackgroundPattern`] |
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `private FieldType m_field` | [`GroupClause::get_field`] / [`GroupClause::set_field`]（`FieldType` → [`FieldKey`]） | 本 crate 对 `FieldType` 接口的运行时擦除 |
//! | `private FontStyle m_font` | `Option<FontStyle>` | Java 侧初始为 `null`，只有 MPP 读取器填过才有值 |
//! | `private Color m_cellBackgroundColor` | `Option<Color>` | 同上；`ColorType.AUTOMATIC` 的颜色在 Java 里也是 `null` |
//! | `private BackgroundPattern m_pattern` | `Option<BackgroundPattern>` | 同上 |
//! | `public Object getGroupInterval()` / `setGroupInterval(Object)` | [`GroupClause::get_group_interval`] / [`GroupClause::set_group_interval`]（`Object` → `Option<FieldValue>`，见下） | 实际取值为数值 / 字符串 / 工期等标量 |
//! | `@Override public String toString()` | [`Display`] | 八段格式与 Java 完全一致（含 Java 侧**漏写**结尾 `]` 的原样行为） |
//!
//! 未移植 / 跳过的 Java 成员：无（三个样式成员已随 `FontStyle` / `BackgroundPattern` /
//! `Color` 落地而补齐）。
//!
//! 反射 / JDBC / XML(JAXB) / `javax.*` / `Serializable`：本类未使用这些机制。
//!
//! 其余两点：
//!
//! - **`Object` 取值**：`m_startAt` / `m_groupInterval` 在 Java 中是 `Object`（实际取值为
//!   数值 / 字符串 / 工期等标量）。Rust 侧用本 crate 的标量 `Object` 载体
//!   [`FieldValue`] 表达。
//! - **`toString()` 的格式**：Java 输出 `field` / `ascending` / `font` / `color` /
//!   `pattern` / `groupOn` / `startAt` / `groupInterval` 各段（且**漏写**了结尾的 `]`，
//!   属 Java 侧原样保留的行为）。Rust 侧逐段照搬，其中 `color` 段按
//!   `java.awt.Color.toString()` 的形式渲染（见 [`format_color`]）。
//! - **`equals` / `hashCode`**：Java 未覆写（对象同一性）；Rust 侧不实现
//!   `PartialEq` / `Eq` / `Hash`。

use std::fmt::{self, Display, Formatter};

use crate::field_key::FieldKey;
use crate::field_value::FieldValue;
use crate::mpp::background_pattern::BackgroundPattern;
use crate::mpp::font_style::FontStyle;
use crate::mpp::mpp_utility::{format_color, Color};
use crate::test_operator::Operand;

/// 分组定义中的一条子句。
///
/// 对应 Java: `public final class GroupClause`
pub struct GroupClause {
    /// 分组字段。
    ///
    /// 对应 Java: `private FieldType m_field;`
    m_field: Option<FieldKey>,

    /// 是否升序分组。
    ///
    /// 对应 Java: `private boolean m_ascending;`
    m_ascending: bool,

    /// 字体样式。
    ///
    /// 对应 Java: `private FontStyle m_font;`
    m_font: Option<FontStyle>,

    /// 单元格背景色。
    ///
    /// 对应 Java: `private Color m_cellBackgroundColor;`
    m_cell_background_color: Option<Color>,

    /// 背景图案。
    ///
    /// 对应 Java: `private BackgroundPattern m_pattern;`
    m_pattern: Option<BackgroundPattern>,

    /// 分组方式。
    ///
    /// 对应 Java: `private int m_groupOn;`
    m_group_on: i32,

    /// 「起始于」取值。
    ///
    /// 对应 Java: `private Object m_startAt;`
    m_start_at: Option<FieldValue>,

    /// 分组间隔。
    ///
    /// 对应 Java: `private Object m_groupInterval;`
    m_group_interval: Option<FieldValue>,
}

impl GroupClause {
    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法（Java 侧未显式声明构造方法）。
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

    /// 取「是否升序分组」标志。
    ///
    /// 对应 Java: `public boolean getAscending()`
    pub fn get_ascending(&self) -> bool {
        self.m_ascending
    }

    /// 设置「是否升序分组」标志。
    ///
    /// 对应 Java: `public void setAscending(boolean ascending)`
    pub fn set_ascending(&mut self, ascending: bool) {
        self.m_ascending = ascending;
    }

    /// 取单元格背景色。
    ///
    /// 对应 Java: `public Color getCellBackgroundColor()`
    pub fn get_cell_background_color(&self) -> Option<Color> {
        self.m_cell_background_color
    }

    /// 设置单元格背景色。
    ///
    /// 对应 Java: `public void setCellBackgroundColor(Color color)`
    pub fn set_cell_background_color(&mut self, color: Option<Color>) {
        self.m_cell_background_color = color;
    }

    /// 取分组字段。
    ///
    /// 对应 Java: `public FieldType getField()`
    pub fn get_field(&self) -> Option<FieldKey> {
        self.m_field
    }

    /// 设置分组字段。
    ///
    /// 对应 Java: `public void setField(FieldType field)`
    pub fn set_field(&mut self, field: Option<FieldKey>) {
        self.m_field = field;
    }

    /// 取字体样式。
    ///
    /// 对应 Java: `public FontStyle getFont()`
    pub fn get_font(&self) -> Option<&FontStyle> {
        self.m_font.as_ref()
    }

    /// 设置字体样式。
    ///
    /// 对应 Java: `public void setFont(FontStyle font)`
    pub fn set_font(&mut self, font: Option<FontStyle>) {
        self.m_font = font;
    }

    /// 取分组间隔。
    ///
    /// 对应 Java: `public Object getGroupInterval()`
    pub fn get_group_interval(&self) -> Option<&FieldValue> {
        self.m_group_interval.as_ref()
    }

    /// 设置分组间隔。
    ///
    /// 对应 Java: `public void setGroupInterval(Object groupInterval)`
    pub fn set_group_interval(&mut self, group_interval: Option<FieldValue>) {
        self.m_group_interval = group_interval;
    }

    /// 取分组方式。
    ///
    /// 对应 Java: `public int getGroupOn()`
    pub fn get_group_on(&self) -> i32 {
        self.m_group_on
    }

    /// 设置分组方式。
    ///
    /// 对应 Java: `public void setGroupOn(int groupOn)`
    pub fn set_group_on(&mut self, group_on: i32) {
        self.m_group_on = group_on;
    }

    /// 取背景图案。
    ///
    /// 对应 Java: `public BackgroundPattern getPattern()`
    pub fn get_pattern(&self) -> Option<BackgroundPattern> {
        self.m_pattern
    }

    /// 设置背景图案。
    ///
    /// 对应 Java: `public void setPattern(BackgroundPattern pattern)`
    pub fn set_pattern(&mut self, pattern: Option<BackgroundPattern>) {
        self.m_pattern = pattern;
    }

    /// 取「起始于」取值。
    ///
    /// 对应 Java: `public Object getStartAt()`
    pub fn get_start_at(&self) -> Option<&FieldValue> {
        self.m_start_at.as_ref()
    }

    /// 设置「起始于」取值。
    ///
    /// 对应 Java: `public void setStartAt(Object startAt)`
    pub fn set_start_at(&mut self, start_at: Option<FieldValue>) {
        self.m_start_at = start_at;
    }
}

impl Default for GroupClause {
    /// 对应 Java: 隐式默认构造方法，等价于 [`GroupClause::new`]。
    fn default() -> Self {
        Self::new()
    }
}

/// 按 Java 的 `String.valueOf` 风格渲染取值（`None` → `"null"`）。
fn render_value(value: Option<&FieldValue>) -> String {
    // 复用 [`Operand`] 的 Java 风格渲染，避免同一套渲染逻辑出现多份实现。
    match value {
        Some(value) => Operand::from(value.clone()).to_string(),
        None => "null".to_owned(),
    }
}

/// 把可选对象按 Java 的字符串拼接语义渲染（`None` → `"null"`）。
fn render_optional<T: Display>(value: Option<&T>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

impl Display for GroupClause {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// 与 Java 一致地**不含**结尾的 `]`。
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
    /// Java 未覆写调试输出；Rust 侧提供 `Debug`（复用 [`Display`]）。
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
        // Java 的 `setFont` / `setCellBackgroundColor` / `setPattern` 往返一致
        assert_eq!(clause.get_font(), Some(&font));
        assert_eq!(
            clause.get_cell_background_color(),
            Some(Color::new(255, 0, 0))
        );
        assert_eq!(clause.get_pattern(), Some(BackgroundPattern::Dotted));
    }

    /// 三个样式成员可以被重新置空（Java 侧同样允许传 `null`）。
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

    /// `font` / `color` / `pattern` 三段与 Java 的 `toString()` 逐字一致。
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
