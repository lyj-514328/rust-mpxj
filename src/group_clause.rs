//! 对应 Java: `org/mpxj/GroupClause.java`
//!
//! 跳过/未移植的 Java 成员与原因：
//!
//! | Java | 原因 |
//! |---|---|
//! | `public Color getCellBackgroundColor()` / `setCellBackgroundColor(Color)` | `java.awt.Color` 属于 GUI（本 crate 不移植 GUI），且本项目未引入颜色类型 |
//! | `public FontStyle getFont()` / `setFont(FontStyle)` | `FontStyle`（`org.mpxj.mpp`）未在 `src/lib.rs` 声明，本 crate 无该类型 |
//! | `public BackgroundPattern getPattern()` / `setPattern(BackgroundPattern)` | `BackgroundPattern`（`org.mpxj.mpp`）未在 `src/lib.rs` 声明，本 crate 无该类型 |
//! | `private FieldType m_field` | [`GroupClause::get_field`] / [`GroupClause::set_field`]（`FieldType` → [`FieldKey`]） |
//! | `public boolean getAscending()` / `setAscending(boolean)` | [`GroupClause::get_ascending`] / [`GroupClause::set_ascending`] |
//! | `public Object getGroupInterval()` / `setGroupInterval(Object)` | [`GroupClause::get_group_interval`] / [`GroupClause::set_group_interval`]（`Object` → `Option<FieldValue>`，见下） |
//! | `public int getGroupOn()` / `setGroupOn(int)` | [`GroupClause::get_group_on`] / [`GroupClause::set_group_on`] |
//! | `public Object getStartAt()` / `setStartAt(Object)` | [`GroupClause::get_start_at`] / [`GroupClause::set_start_at`] |
//! | `@Override public String toString()` | [`Display`]（跳过被移除的三个成员对应的片段，见下） |
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用（颜色属于 GUI，
//! 已按上表跳过）。
//!
//! # 与 Java 的差异
//!
//! - **`Object` 取值**：`m_startAt` / `m_groupInterval` 在 Java 中是 `Object`（实际取值为
//!   数值 / 字符串 / 工期等标量）。Rust 侧用本 crate 的标量 `Object` 载体
//!   [`FieldValue`] 表达。
//! - **`toString()` 的格式**：Java 输出 `field` / `ascending` / `font` / `color` /
//!   `pattern` / `groupOn` / `startAt` / `groupInterval` 各段（且**漏写**了结尾的 `]`，
//!   属 Java 侧原样保留的行为）。由于 `font` / `color` / `pattern` 三个字段未移植，
//!   Rust 侧的输出省略这三段，其余格式与 Java 保持一致（同样不含结尾 `]`）。
//! - **`equals` / `hashCode`**：Java 未覆写（对象同一性）；Rust 侧不实现
//!   `PartialEq` / `Eq` / `Hash`。

use std::fmt::{self, Display, Formatter};

use crate::field_key::FieldKey;
use crate::field_value::FieldValue;
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

impl Display for GroupClause {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// 与 Java 一致地**不含**结尾的 `]`；省略未移植的 `font` / `color` / `pattern` 三段。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let field = match self.m_field {
            Some(field) => field.to_string(),
            None => "null".to_owned(),
        };

        write!(
            f,
            "[GroupClause field={} ascending={} groupOn={} startAt={} groupInterval={}",
            field,
            self.m_ascending,
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
    use crate::task_field::TaskField;
    use crate::time_unit::TimeUnit;

    #[test]
    fn accessors_round_trip() {
        let mut clause = GroupClause::new();

        assert!(!clause.get_ascending());
        assert_eq!(clause.get_field(), None);
        assert_eq!(clause.get_group_interval(), None);
        assert_eq!(clause.get_group_on(), 0);
        assert_eq!(clause.get_start_at(), None);

        clause.set_ascending(true);
        clause.set_field(Some(FieldKey::Task(TaskField::Name)));
        clause.set_group_on(2);
        clause.set_start_at(Some(FieldValue::String("A".to_owned())));
        clause.set_group_interval(Some(FieldValue::Integer(3)));

        assert!(clause.get_ascending());
        assert_eq!(clause.get_field(), Some(FieldKey::Task(TaskField::Name)));
        assert_eq!(clause.get_group_on(), 2);
        assert_eq!(clause.get_start_at(), Some(&FieldValue::String("A".to_owned())));
        assert_eq!(
            clause.get_group_interval(),
            Some(&FieldValue::Integer(3))
        );
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
            "[GroupClause field=Task Name ascending=true groupOn=1 startAt=A groupInterval=2"
        );
    }

    #[test]
    fn display_renders_nulls() {
        let clause = GroupClause::new();
        assert_eq!(
            clause.to_string(),
            "[GroupClause field=null ascending=false groupOn=0 startAt=null groupInterval=null"
        );
    }

    #[test]
    fn default_matches_new() {
        let clause = GroupClause::default();
        assert_eq!(clause.get_group_on(), 0);
    }
}
