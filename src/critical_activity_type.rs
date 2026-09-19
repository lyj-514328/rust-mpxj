//! 对应 Java: `org/mpxj/CriticalActivityType.java`

use crate::mpxj_enum::MpxjEnum;

/// 决定任务的关键路径标记如何判定。
///
/// 对应 Java: `public enum CriticalActivityType`
///
/// Java 侧该枚举不实现 `MpxjEnum`，也没有 `getInstance`；Rust 侧按声明顺序提供
/// [`CriticalActivityType::ALL`]，并用「序号（`ordinal()`）」实现 [`MpxjEnum::value`]，
/// 使 int 与枚举可以互转（`get_instance` 在 Java 侧无对应方法）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CriticalActivityType {
    /// 对应 Java: `TOTAL_FLOAT`
    TotalFloat,
    /// 对应 Java: `LONGEST_PATH`
    LongestPath,
}

impl CriticalActivityType {
    /// 全部取值，顺序与 Java 声明顺序一致。
    pub const ALL: [CriticalActivityType; 2] = [
        CriticalActivityType::TotalFloat,
        CriticalActivityType::LongestPath,
    ];

    /// 按序号（Java 的 `ordinal()`）取枚举实例，越界返回 `None`。
    ///
    /// Java 侧无对应方法（Java 直接使用枚举常量或 `values()[n]`）。
    pub fn get_instance(value: i32) -> Option<CriticalActivityType> {
        match value {
            0 => Some(CriticalActivityType::TotalFloat),
            1 => Some(CriticalActivityType::LongestPath),
            _ => None,
        }
    }
}

impl MpxjEnum for CriticalActivityType {
    /// 返回声明顺序对应的序号。
    ///
    /// Java 侧无 `getValue()`，此处对应 `Enum.ordinal()`。
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<CriticalActivityType> for i32 {
    fn from(value: CriticalActivityType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn critical_activity_type_round_trips() {
        for item in CriticalActivityType::ALL {
            assert_eq!(CriticalActivityType::get_instance(item.value()), Some(item));
        }
    }

    #[test]
    fn critical_activity_type_out_of_range() {
        assert_eq!(CriticalActivityType::get_instance(-1), None);
        assert_eq!(CriticalActivityType::get_instance(2), None);
    }
}
