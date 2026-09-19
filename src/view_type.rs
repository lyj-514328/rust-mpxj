//! 对应 Java: `org/mpxj/ViewType.java`
//!
//! 跳过/未移植的 Java 方法与原因：
//!
//! | Java | Rust |
//! |---|---|
//! | `TYPE_VALUES = EnumHelper.createTypeArray(ViewType.class, 1)` | [`ViewType::ALL`] + [`ViewType::get_instance`]（无反射） |
//! | `public static ViewType getInstance(int type)` | [`ViewType::get_instance`] |
//! | `public static ViewType getInstance(Number type)` | [`ViewType::get_instance_optional`]（`null` → `-1` → 回落到 `UNKNOWN`） |
//! | `@Override public int getValue()` | [`MpxjEnum::value`] |
//! | `public String getName()` | [`ViewType::get_name`] |
//! | `@Override public String toString()` | [`Display`] |
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类仅用到反射建表
//! （已改为 `match`，见上）。
//!
//! # 与 Java 的差异：反射建表的等价表达
//!
//! Java 侧 `TYPE_VALUES` 的长度为 `常量个数(15) + 1 = 16`，按下标 `getValue()` 填充。
//! 由于不存在 `getValue() == 9` 的常量，`TYPE_VALUES[9]` 为 `null`，Java 会回落到
//! `UNKNOWN`；下标越界（负数或 ≥ 16）同样先回落到 `UNKNOWN.getValue()`（即 0）。
//! Rust 侧用 `match` 精确复刻这两条回落路径。

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// 视图类型。
///
/// 对应 Java: `public enum ViewType implements MpxjEnum`
///
/// 常量声明顺序与 `getValue()` 取值与 Java 完全一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewType {
    /// 对应 Java: `UNKNOWN(0, "UNKNOWN")`
    Unknown = 0,
    /// 对应 Java: `GANTT_CHART(1, "GANTT_CHART")`
    GanttChart = 1,
    /// 对应 Java: `NETWORK_DIAGRAM(2, "NETWORK_DIAGRAM")`
    NetworkDiagram = 2,
    /// 对应 Java: `RELATIONSHIP_DIAGRAM(3, "RELATIONSHIP_DIAGRAM")`
    RelationshipDiagram = 3,
    /// 对应 Java: `TASK_FORM(4, "TASK_FORM")`
    TaskForm = 4,
    /// 对应 Java: `TASK_SHEET(5, "TASK_SHEET")`
    TaskSheet = 5,
    /// 对应 Java: `RESOURCE_FORM(6, "RESOURCE_FORM")`
    ResourceForm = 6,
    /// 对应 Java: `RESOURCE_SHEET(7, "RESOURCE_SHEET")`
    ResourceSheet = 7,
    /// 对应 Java: `RESOURCE_GRAPH(8, "RESOURCE_GRAPH")`
    ResourceGraph = 8,
    /// 对应 Java: `TASK_DETAILS_FORM(10, "TASK_DETAILS_FORM")`
    TaskDetailsForm = 10,
    /// 对应 Java: `TASK_NAME_FORM(11, "TASK_NAME_FORM")`
    TaskNameForm = 11,
    /// 对应 Java: `RESOURCE_NAME_FORM(12, "RESOURCE_NAME_FORM")`
    ResourceNameForm = 12,
    /// 对应 Java: `CALENDAR(13, "CALENDAR")`
    Calendar = 13,
    /// 对应 Java: `TASK_USAGE(14, "TASK_USAGE")`
    TaskUsage = 14,
    /// 对应 Java: `RESOURCE_USAGE(15, "RESOURCE_USAGE")`
    ResourceUsage = 15,
}

impl ViewType {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `values()`
    pub const ALL: [ViewType; 15] = [
        ViewType::Unknown,
        ViewType::GanttChart,
        ViewType::NetworkDiagram,
        ViewType::RelationshipDiagram,
        ViewType::TaskForm,
        ViewType::TaskSheet,
        ViewType::ResourceForm,
        ViewType::ResourceSheet,
        ViewType::ResourceGraph,
        ViewType::TaskDetailsForm,
        ViewType::TaskNameForm,
        ViewType::ResourceNameForm,
        ViewType::Calendar,
        ViewType::TaskUsage,
        ViewType::ResourceUsage,
    ];

    /// Java 侧 `TYPE_VALUES` 数组长度：`EnumHelper.createTypeArray(ViewType.class, 1)`
    /// 生成 `常量个数(15) + 1 = 16` 个槽位。
    const TYPE_VALUES_LEN: i32 = 16;

    /// 根据 int 值取枚举实例，越界或该下标没有常量时回落到 `UNKNOWN`。
    ///
    /// 对应 Java: `public static ViewType getInstance(int type)`
    pub fn get_instance(value: i32) -> ViewType {
        if !(0..Self::TYPE_VALUES_LEN).contains(&value) {
            return ViewType::Unknown;
        }

        match value {
            0 => ViewType::Unknown,
            1 => ViewType::GanttChart,
            2 => ViewType::NetworkDiagram,
            3 => ViewType::RelationshipDiagram,
            4 => ViewType::TaskForm,
            5 => ViewType::TaskSheet,
            6 => ViewType::ResourceForm,
            7 => ViewType::ResourceSheet,
            8 => ViewType::ResourceGraph,
            // 对应 Java 中 TYPE_VALUES[9] == null
            10 => ViewType::TaskDetailsForm,
            11 => ViewType::TaskNameForm,
            12 => ViewType::ResourceNameForm,
            13 => ViewType::Calendar,
            14 => ViewType::TaskUsage,
            15 => ViewType::ResourceUsage,
            _ => ViewType::Unknown,
        }
    }

    /// 根据可空的 int 值取枚举实例（`null` 等价于 `-1`，回落到 `UNKNOWN`）。
    ///
    /// 对应 Java: `public static ViewType getInstance(Number type)`
    pub fn get_instance_optional(value: Option<i32>) -> ViewType {
        ViewType::get_instance(value.unwrap_or(-1))
    }

    /// 取该枚举的名称。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            ViewType::Unknown => "UNKNOWN",
            ViewType::GanttChart => "GANTT_CHART",
            ViewType::NetworkDiagram => "NETWORK_DIAGRAM",
            ViewType::RelationshipDiagram => "RELATIONSHIP_DIAGRAM",
            ViewType::TaskForm => "TASK_FORM",
            ViewType::TaskSheet => "TASK_SHEET",
            ViewType::ResourceForm => "RESOURCE_FORM",
            ViewType::ResourceSheet => "RESOURCE_SHEET",
            ViewType::ResourceGraph => "RESOURCE_GRAPH",
            ViewType::TaskDetailsForm => "TASK_DETAILS_FORM",
            ViewType::TaskNameForm => "TASK_NAME_FORM",
            ViewType::ResourceNameForm => "RESOURCE_NAME_FORM",
            ViewType::Calendar => "CALENDAR",
            ViewType::TaskUsage => "TASK_USAGE",
            ViewType::ResourceUsage => "RESOURCE_USAGE",
        }
    }
}

impl MpxjEnum for ViewType {
    /// 返回该视图类型的 int 表示。
    ///
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for ViewType {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 返回 `m_name`（与常量名相同）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_constants_round_trip_through_get_instance() {
        for view_type in ViewType::ALL {
            assert_eq!(ViewType::get_instance(view_type.value()), view_type);
        }
    }

    #[test]
    fn missing_table_entry_falls_back_to_unknown() {
        // Java 侧 TYPE_VALUES[9] == null
        assert_eq!(ViewType::get_instance(9), ViewType::Unknown);
    }

    #[test]
    fn out_of_range_falls_back_to_unknown() {
        assert_eq!(ViewType::get_instance(-1), ViewType::Unknown);
        assert_eq!(ViewType::get_instance(16), ViewType::Unknown);
        assert_eq!(ViewType::get_instance_optional(None), ViewType::Unknown);
        assert_eq!(
            ViewType::get_instance_optional(Some(1)),
            ViewType::GanttChart
        );
    }

    #[test]
    fn name_and_display_match_java() {
        assert_eq!(ViewType::GanttChart.get_name(), "GANTT_CHART");
        assert_eq!(ViewType::GanttChart.to_string(), "GANTT_CHART");
        assert_eq!(ViewType::Unknown.to_string(), "UNKNOWN");
    }
}
