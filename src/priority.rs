//! 对应 Java: `org/mpxj/Priority.java`

use std::fmt::{self, Display, Formatter};

/// 任务优先级。
///
/// 对应 Java: `public final class Priority`
///
/// Java 侧这是一个不可变值类：
/// * 私有构造方法把越界值收敛为 `MEDIUM`；
/// * `getInstance(int)` 对 `[LOWEST, DO_NOT_LEVEL]` 区间内 100 的整数倍返回
///   `VALUE` 数组中缓存的规范实例，其余值新建实例；
/// * `equals` / `hashCode` 未重写，`PriorityHelper` 等代码依赖「`getInstance`
///   返回缓存实例」带来的同一性比较。
///
/// Rust 侧映射说明：
/// * Java 的 `private final int m_value` → 私有字段 `m_value`；
/// * Java 的 10 个 `public static final int` 常量 → 同名的 `i32` 关联常量
///   （调用方式与 Java 一致，如 `Priority::get_instance(Priority::HIGH)`）；
/// * Java 的 `private static final Priority[] VALUE` → [`Priority::ALL`]；
/// * Rust 的值类型无法表达对象同一性，故按字段值派生 `PartialEq` / `Eq` /
///   `Hash`；对 `getInstance` 返回的规范实例而言与 Java 行为一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Priority {
    /// 优先级的内部表示。
    ///
    /// 对应 Java: `private final int m_value;`
    m_value: i32,
}

impl Priority {
    /// 最低优先级。
    ///
    /// 对应 Java: `public static final int LOWEST`
    pub const LOWEST: i32 = 100;

    /// 很低优先级。
    ///
    /// 对应 Java: `public static final int VERY_LOW`
    pub const VERY_LOW: i32 = 200;

    /// 较低优先级。
    ///
    /// 对应 Java: `public static final int LOWER`
    pub const LOWER: i32 = 300;

    /// 低优先级。
    ///
    /// 对应 Java: `public static final int LOW`
    pub const LOW: i32 = 400;

    /// 中优先级。
    ///
    /// 对应 Java: `public static final int MEDIUM`
    pub const MEDIUM: i32 = 500;

    /// 高优先级。
    ///
    /// 对应 Java: `public static final int HIGH`
    pub const HIGH: i32 = 600;

    /// 较高优先级。
    ///
    /// 对应 Java: `public static final int HIGHER`
    pub const HIGHER: i32 = 700;

    /// 很高优先级。
    ///
    /// 对应 Java: `public static final int VERY_HIGH`
    pub const VERY_HIGH: i32 = 800;

    /// 最高优先级。
    ///
    /// 对应 Java: `public static final int HIGHEST`
    pub const HIGHEST: i32 = 900;

    /// 「不参与调配（do not level）」优先级。
    ///
    /// 对应 Java: `public static final int DO_NOT_LEVEL`
    pub const DO_NOT_LEVEL: i32 = 1000;

    /// 全部规范实例，按 int 值升序排列。
    ///
    /// 对应 Java: `private static final Priority[] VALUE`
    pub const ALL: [Priority; 10] = [
        Priority {
            m_value: Priority::LOWEST,
        },
        Priority {
            m_value: Priority::VERY_LOW,
        },
        Priority {
            m_value: Priority::LOWER,
        },
        Priority {
            m_value: Priority::LOW,
        },
        Priority {
            m_value: Priority::MEDIUM,
        },
        Priority {
            m_value: Priority::HIGH,
        },
        Priority {
            m_value: Priority::HIGHER,
        },
        Priority {
            m_value: Priority::VERY_HIGH,
        },
        Priority {
            m_value: Priority::HIGHEST,
        },
        Priority {
            m_value: Priority::DO_NOT_LEVEL,
        },
    ];

    /// 私有构造方法：越界值（小于 0 或大于 `DO_NOT_LEVEL`）收敛为 `MEDIUM`。
    ///
    /// 对应 Java: `private Priority(int priority)`
    fn new(priority: i32) -> Priority {
        if priority < 0 || priority > Priority::DO_NOT_LEVEL {
            Priority {
                m_value: Priority::MEDIUM,
            }
        } else {
            Priority { m_value: priority }
        }
    }

    /// 根据 int 值取实例：区间内 100 的整数倍返回规范实例，其余值新建，
    /// 越界值收敛为 `MEDIUM`。
    ///
    /// 对应 Java: `public static Priority getInstance(int priority)`
    pub fn get_instance(priority: i32) -> Priority {
        if (Priority::LOWEST..=Priority::DO_NOT_LEVEL).contains(&priority) && priority % 100 == 0 {
            // Java: VALUE[(priority / 100) - 1]，此处下标必然落在 0..=9。
            Priority::ALL[(priority / 100 - 1) as usize]
        } else {
            Priority::new(priority)
        }
    }

    /// 返回优先级的数值表示。
    ///
    /// 对应 Java: `public int getValue()`
    pub fn get_value(&self) -> i32 {
        self.m_value
    }
}

impl Display for Priority {
    /// 对应 Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[Priority value={}]", self.m_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_round_trips_through_all() {
        for item in Priority::ALL {
            assert_eq!(Priority::get_instance(item.get_value()), item);
        }
    }

    #[test]
    fn priority_constants_match_java() {
        assert_eq!(Priority::LOWEST, 100);
        assert_eq!(Priority::VERY_LOW, 200);
        assert_eq!(Priority::LOWER, 300);
        assert_eq!(Priority::LOW, 400);
        assert_eq!(Priority::MEDIUM, 500);
        assert_eq!(Priority::HIGH, 600);
        assert_eq!(Priority::HIGHER, 700);
        assert_eq!(Priority::VERY_HIGH, 800);
        assert_eq!(Priority::HIGHEST, 900);
        assert_eq!(Priority::DO_NOT_LEVEL, 1000);
    }

    #[test]
    fn priority_keeps_non_canonical_values_in_range() {
        // Java: 非 100 的整数倍时新建实例，值保持不变。
        assert_eq!(Priority::get_instance(250).get_value(), 250);
        assert_eq!(Priority::get_instance(0).get_value(), 0);
        assert_eq!(
            Priority::get_instance(Priority::DO_NOT_LEVEL).get_value(),
            Priority::DO_NOT_LEVEL
        );
    }

    #[test]
    fn priority_clamps_out_of_range_values_to_medium() {
        // Java: 构造方法把越界值收敛为 MEDIUM。
        assert_eq!(Priority::get_instance(-1), Priority::get_instance(Priority::MEDIUM));
        assert_eq!(Priority::get_instance(1001), Priority::get_instance(Priority::MEDIUM));
        assert_eq!(Priority::get_instance(1001).get_value(), Priority::MEDIUM);
    }

    #[test]
    fn priority_display_matches_java_to_string() {
        assert_eq!(
            Priority::get_instance(Priority::HIGH).to_string(),
            "[Priority value=600]"
        );
    }
}
