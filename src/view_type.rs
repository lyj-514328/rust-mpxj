//! Corresponds to Java: `org/mpxj/ViewType.java`
//!
//! Skipped / unported Java methods and reasons:
//!
//! | Java | Rust |
//! |---|---|
//! | `TYPE_VALUES = EnumHelper.createTypeArray(ViewType.class, 1)` | [`ViewType::ALL`] + [`ViewType::get_instance`] (no reflection) |
//! | `public static ViewType getInstance(int type)` | [`ViewType::get_instance`] |
//! | `public static ViewType getInstance(Number type)` | [`ViewType::get_instance_optional`] (`null` → `-1` → falls back to `UNKNOWN`) |
//! | `@Override public int getValue()` | [`MpxjEnum::value`] |
//! | `public String getName()` | [`ViewType::get_name`] |
//! | `@Override public String toString()` | [`Display`] |
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: this class only uses reflection to build the table
//! (now changed to `match`, see above).
//!
//! # Differences from Java: equivalent expression of the reflection-built table
//!
//! In Java the length of `TYPE_VALUES` is `number of constants(15) + 1 = 16`, filled by the index `getValue()`.
//! Since no constant has `getValue() == 9`, `TYPE_VALUES[9]` is `null`, and Java falls back to
//! `UNKNOWN`; an out-of-range index (negative or ≥ 16) likewise first falls back to `UNKNOWN.getValue()` (i.e. 0).
//! The Rust side uses `match` to replicate these two fallback paths exactly.

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// View type.
///
/// Corresponds to Java: `public enum ViewType implements MpxjEnum`
///
/// The constant declaration order and `getValue()` values are fully consistent with Java.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewType {
    /// Corresponds to Java: `UNKNOWN(0, "UNKNOWN")`
    Unknown = 0,
    /// Corresponds to Java: `GANTT_CHART(1, "GANTT_CHART")`
    GanttChart = 1,
    /// Corresponds to Java: `NETWORK_DIAGRAM(2, "NETWORK_DIAGRAM")`
    NetworkDiagram = 2,
    /// Corresponds to Java: `RELATIONSHIP_DIAGRAM(3, "RELATIONSHIP_DIAGRAM")`
    RelationshipDiagram = 3,
    /// Corresponds to Java: `TASK_FORM(4, "TASK_FORM")`
    TaskForm = 4,
    /// Corresponds to Java: `TASK_SHEET(5, "TASK_SHEET")`
    TaskSheet = 5,
    /// Corresponds to Java: `RESOURCE_FORM(6, "RESOURCE_FORM")`
    ResourceForm = 6,
    /// Corresponds to Java: `RESOURCE_SHEET(7, "RESOURCE_SHEET")`
    ResourceSheet = 7,
    /// Corresponds to Java: `RESOURCE_GRAPH(8, "RESOURCE_GRAPH")`
    ResourceGraph = 8,
    /// Corresponds to Java: `TASK_DETAILS_FORM(10, "TASK_DETAILS_FORM")`
    TaskDetailsForm = 10,
    /// Corresponds to Java: `TASK_NAME_FORM(11, "TASK_NAME_FORM")`
    TaskNameForm = 11,
    /// Corresponds to Java: `RESOURCE_NAME_FORM(12, "RESOURCE_NAME_FORM")`
    ResourceNameForm = 12,
    /// Corresponds to Java: `CALENDAR(13, "CALENDAR")`
    Calendar = 13,
    /// Corresponds to Java: `TASK_USAGE(14, "TASK_USAGE")`
    TaskUsage = 14,
    /// Corresponds to Java: `RESOURCE_USAGE(15, "RESOURCE_USAGE")`
    ResourceUsage = 15,
}

impl ViewType {
/// All values, in the same order as the Java declarations.
    ///
    /// Corresponds to Java: `values()`
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

/// The length of the `TYPE_VALUES` array in Java: `EnumHelper.createTypeArray(ViewType.class, 1)`
/// generates `number of constants(15) + 1 = 16` slots.
    const TYPE_VALUES_LEN: i32 = 16;

/// Get an enum instance from an int value, falling back to `UNKNOWN` when out of range or when there is no constant at that index.
    ///
    /// Corresponds to Java: `public static ViewType getInstance(int type)`
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
// Corresponds to TYPE_VALUES[9] == null in Java
            10 => ViewType::TaskDetailsForm,
            11 => ViewType::TaskNameForm,
            12 => ViewType::ResourceNameForm,
            13 => ViewType::Calendar,
            14 => ViewType::TaskUsage,
            15 => ViewType::ResourceUsage,
            _ => ViewType::Unknown,
        }
    }

/// Get an enum instance from a nullable int value (`null` is equivalent to `-1`, falling back to `UNKNOWN`).
    ///
    /// Corresponds to Java: `public static ViewType getInstance(Number type)`
    pub fn get_instance_optional(value: Option<i32>) -> ViewType {
        ViewType::get_instance(value.unwrap_or(-1))
    }

/// Get the name of this enum.
    ///
    /// Corresponds to Java: `public String getName()`
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
/// Return the int representation of this view type.
    ///
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for ViewType {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java returns `m_name` (the same as the constant name).
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
// TYPE_VALUES[9] == null in Java
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
