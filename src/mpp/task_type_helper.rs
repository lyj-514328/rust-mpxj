//! 对应 Java: `org/mpxj/mpp/TaskTypeHelper.java`
//!
//! 在「Microsoft Project 文件里的任务类型整数」与 `org.mpxj.TaskType` 之间互相转换。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `public final class TaskTypeHelper`（私有构造方法） | 无字段结构体 [`TaskTypeHelper`] + 关联函数 | Rust 用关联函数表达静态方法 |
//! | `private static final Map<TaskType, Integer> TYPE_MAP` | `match` 分支 | Java 的映射表实际是「枚举 → 整数」的查表，Rust 侧直接写成分支更直观；表内容逐项保留（注意 `FIXED_DURATION_AND_UNITS` 与 `FIXED_DURATION` 取同一个值 1） |
//! | `getValue(TaskType type)` 的参数可为 `null` | `Option<TaskType>` | 遵循「`null` → `Option<T>`」约定：`None` 等价于 Java 的 `null`，按 `FIXED_UNITS` 取值 |
//!
//! 跳过的 Java 成员：无。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use crate::task_type::TaskType;

/// 任务类型的整数表示转换。
///
/// 对应 Java: `public final class TaskTypeHelper`
pub struct TaskTypeHelper;

/// Java 中「整数 → 任务类型」的查找表。
///
/// 对应 Java: `private static final TaskType[] TYPE_VALUES`
const TYPE_VALUES: [TaskType; 3] = [
    TaskType::FixedUnits,
    TaskType::FixedDuration,
    TaskType::FixedWork,
];

impl TaskTypeHelper {
    /// 按整数取任务类型；越界（含负数）时返回 `FIXED_WORK`。
    ///
    /// 对应 Java: `public static TaskType getInstance(int type)`
    pub fn get_instance(r#type: i32) -> TaskType {
        // Java: if (type < 0 || type >= TYPE_VALUES.length) { return TaskType.FIXED_WORK; }
        let index = usize::try_from(r#type).ok();
        match index.and_then(|index| TYPE_VALUES.get(index)) {
            Some(value) => *value,
            None => TaskType::FixedWork,
        }
    }

    /// 取任务类型对应的整数；`None`（Java 的 `null`）按 `FIXED_UNITS` 处理。
    ///
    /// 对应 Java: `public static int getValue(TaskType type)`
    pub fn get_value(r#type: Option<TaskType>) -> i32 {
        // Java: if (type == null) { type = TaskType.FIXED_UNITS; }
        //       return TYPE_MAP.get(type).intValue();
        match r#type {
            // TYPE_MAP: FIXED_UNITS -> 0
            None | Some(TaskType::FixedUnits) => 0,
            // TYPE_MAP: FIXED_DURATION -> 1，FIXED_DURATION_AND_UNITS -> 1（两条表项取同一个值）
            Some(TaskType::FixedDuration) | Some(TaskType::FixedDurationAndUnits) => 1,
            // TYPE_MAP: FIXED_WORK -> 2
            Some(TaskType::FixedWork) => 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 三个有效值逐一映射；越界值一律回落到 `FIXED_WORK`（与 Java 一致）。
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

    /// `getValue` 的取值与 Java 的 `TYPE_MAP` 一致，`null` 等价于 `FIXED_UNITS`。
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
