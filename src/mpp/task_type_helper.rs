//! Corresponds to Java: `org/mpxj/mpp/TaskTypeHelper.java`
//!
//! Convert between "the task type integer in a Microsoft Project file" and `org.mpxj.TaskType`.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `public final class TaskTypeHelper` (private constructor) | Field-less struct [`TaskTypeHelper`] + associated functions | Rust expresses static methods with associated functions |
//! | `private static final Map<TaskType, Integer> TYPE_MAP` | `match` arms | Java's map is actually an "enum → integer" lookup, and writing it directly as arms is more intuitive on the Rust side; the table contents are preserved item by item (note that `FIXED_DURATION_AND_UNITS` and `FIXED_DURATION` take the same value 1) |
//! | The parameter of `getValue(TaskType type)` may be `null` | `Option<TaskType>` | Follows the "`null` → `Option<T>`" convention: `None` is equivalent to Java's `null` and takes the `FIXED_UNITS` value |
//!
//! Skipped Java members: none.
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use crate::task_type::TaskType;

/// Conversion of the integer representation of a task type.
///
/// Corresponds to Java: `public final class TaskTypeHelper`
pub struct TaskTypeHelper;

/// Java's "integer → task type" lookup table.
///
/// Corresponds to Java: `private static final TaskType[] TYPE_VALUES`
const TYPE_VALUES: [TaskType; 3] = [
    TaskType::FixedUnits,
    TaskType::FixedDuration,
    TaskType::FixedWork,
];

impl TaskTypeHelper {
/// Return the task type for an integer; out-of-bounds values (including negatives) return `FIXED_WORK`.
    ///
    /// Corresponds to Java: `public static TaskType getInstance(int type)`
    pub fn get_instance(r#type: i32) -> TaskType {
        // Java: if (type < 0 || type >= TYPE_VALUES.length) { return TaskType.FIXED_WORK; }
        let index = usize::try_from(r#type).ok();
        match index.and_then(|index| TYPE_VALUES.get(index)) {
            Some(value) => *value,
            None => TaskType::FixedWork,
        }
    }

/// Return the integer corresponding to a task type; `None` (Java's `null`) is handled as `FIXED_UNITS`.
    ///
    /// Corresponds to Java: `public static int getValue(TaskType type)`
    pub fn get_value(r#type: Option<TaskType>) -> i32 {
        // Java: if (type == null) { type = TaskType.FIXED_UNITS; }
        //       return TYPE_MAP.get(type).intValue();
        match r#type {
            // TYPE_MAP: FIXED_UNITS -> 0
            None | Some(TaskType::FixedUnits) => 0,
// TYPE_MAP: FIXED_DURATION -> 1, FIXED_DURATION_AND_UNITS -> 1 (both entries take the same value)
            Some(TaskType::FixedDuration) | Some(TaskType::FixedDurationAndUnits) => 1,
            // TYPE_MAP: FIXED_WORK -> 2
            Some(TaskType::FixedWork) => 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// The three valid values are mapped one by one; out-of-bounds values always fall back to `FIXED_WORK` (matching Java).
    #[test]
    fn get_instance_follows_java_type_values() {
        assert_eq!(TaskTypeHelper::get_instance(0), TaskType::FixedUnits);
        assert_eq!(TaskTypeHelper::get_instance(1), TaskType::FixedDuration);
        assert_eq!(TaskTypeHelper::get_instance(2), TaskType::FixedWork);

        // Java: type < 0 || type >= TYPE_VALUES.length -> FIXED_WORK
        assert_eq!(TaskTypeHelper::get_instance(3), TaskType::FixedWork);
        assert_eq!(TaskTypeHelper::get_instance(-1), TaskType::FixedWork);
        assert_eq!(TaskTypeHelper::get_instance(i32::MAX), TaskType::FixedWork);
        assert_eq!(TaskTypeHelper::get_instance(i32::MIN), TaskType::FixedWork);
    }

/// The values of `getValue` match Java's `TYPE_MAP`, and `null` is equivalent to `FIXED_UNITS`.
    #[test]
    fn get_value_follows_java_type_map() {
        assert_eq!(TaskTypeHelper::get_value(Some(TaskType::FixedUnits)), 0);
        assert_eq!(TaskTypeHelper::get_value(Some(TaskType::FixedDuration)), 1);
        assert_eq!(TaskTypeHelper::get_value(Some(TaskType::FixedWork)), 2);
        // Java: TYPE_MAP.put(TaskType.FIXED_DURATION_AND_UNITS, Integer.valueOf(1))
        assert_eq!(
            TaskTypeHelper::get_value(Some(TaskType::FixedDurationAndUnits)),
            1
        );
        // Java: type == null -> FIXED_UNITS
        assert_eq!(TaskTypeHelper::get_value(None), 0);
    }
}
