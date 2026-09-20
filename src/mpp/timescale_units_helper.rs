//! 对应 Java: `org/mpxj/mpp/TimescaleUnitsHelper.java`
//!
//! 把 MS Project 文件中表示时间刻度单位（timescale units）的整数换算成
//! `org.mpxj.TimescaleUnits`。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `class TimescaleUnitsHelper`（包级可见） | `pub struct TimescaleUnitsHelper` + 关联函数 | Java 侧只有静态方法、无实例状态；Rust 侧用无字段结构体表达（本仓惯例） |
//! | `private static final TimescaleUnits[] TYPE_VALUES`（10 项） | [`crate::timescale_units::TimescaleUnits::ALL`] | Java 的表元素与 `TimescaleUnits` 的 10 个枚举常量一一对应且顺序相同，故 Rust 侧直接复用 `ALL`，不再复制一份表 |
//! | `type += 1; if (type < 0 \|\| type > TYPE_VALUES.length) return TimescaleUnits.NONE;` | 同样的判断，越界返回 `TimescaleUnits::None` | Java 的 `int` 加法会静默回绕，Rust 侧用 `wrapping_add` 保持一致 |
//! | `return TYPE_VALUES[type];` 在 `type == TYPE_VALUES.length`（即入参 9）时会抛 `ArrayIndexOutOfBoundsException`（Java 的判断写成了 `>` 而不是 `>=`，属既有缺陷） | 返回 `TimescaleUnits::None` | Rust 侧不 panic（本仓约定），按「越界回落到 NONE」处理并在此注明 |
//!
//! 未移植 / 跳过：无（JDBC / XML-JAXB / GUI / `javax.*` / `Serializable` 等机制本类未使用）。

use crate::timescale_units::TimescaleUnits;

/// 时间刻度单位的换算工具。
///
/// 对应 Java: `class TimescaleUnitsHelper`
pub struct TimescaleUnitsHelper;

impl TimescaleUnitsHelper {
    /// 按 MS Project 的整数值取时间刻度单位。
    ///
    /// 对应 Java: `public static TimescaleUnits getInstance(int type)`
    ///
    /// Java 的写法是先 `type += 1` 再查表，因此文件中的 0 对应 `MINUTES`、
    /// 8 对应 `YEARS`；越界（含负数）回落到 `NONE`。
    pub fn get_instance(type_value: i32) -> TimescaleUnits {
        // Java: type += 1（int 加法会回绕，Rust 侧显式用 wrapping_add 保持同样的行为）
        let index = type_value.wrapping_add(1);

        // Java: if (type < 0 || type > TYPE_VALUES.length) { return TimescaleUnits.NONE; }
        if index < 0 || index > TimescaleUnits::ALL.len() as i32 {
            return TimescaleUnits::None;
        }

        // Java: return TYPE_VALUES[type];
        // 其中 index == TYPE_VALUES.length（入参 9）在 Java 中会抛
        // ArrayIndexOutOfBoundsException（判断写成 `>` 而非 `>=` 的既有缺陷），
        // Rust 侧按「越界回落 NONE」处理，不 panic。
        TimescaleUnits::ALL
            .get(index as usize)
            .copied()
            .unwrap_or(TimescaleUnits::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 文件中的 0..8 依次映射到 MINUTES..YEARS（Java 侧先 `type += 1`）。
    #[test]
    fn maps_mpp_values_to_timescale_units() {
        let cases = [
            (0, TimescaleUnits::Minutes),
            (1, TimescaleUnits::Hours),
            (2, TimescaleUnits::Days),
            (3, TimescaleUnits::Weeks),
            (4, TimescaleUnits::ThirdsOfMonths),
            (5, TimescaleUnits::Months),
            (6, TimescaleUnits::Quarters),
            (7, TimescaleUnits::HalfYears),
            (8, TimescaleUnits::Years),
        ];

        for (value, expected) in cases {
            assert_eq!(
                TimescaleUnitsHelper::get_instance(value),
                expected,
                "MPP 单位值 {value}"
            );
        }
    }

    /// 越界（含负数）回落到 NONE。
    #[test]
    fn out_of_range_falls_back_to_none() {
        for value in [-1, -2, -100, 10, 11, 1000, i32::MAX, i32::MIN] {
            assert_eq!(
                TimescaleUnitsHelper::get_instance(value),
                TimescaleUnits::None,
                "越界值 {value}"
            );
        }
    }

    /// 入参 9 在 Java 中会抛 `ArrayIndexOutOfBoundsException`；Rust 侧返回 NONE。
    #[test]
    fn java_array_index_bug_does_not_panic() {
        assert_eq!(
            TimescaleUnitsHelper::get_instance(9),
            TimescaleUnits::None
        );
    }
}
