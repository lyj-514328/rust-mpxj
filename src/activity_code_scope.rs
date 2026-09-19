//! 对应 Java: `org/mpxj/ActivityCodeScope.java`

use crate::mpxj_enum::MpxjEnum;

/// 活动编码（Activity Code）的作用范围。
///
/// 对应 Java: `public enum ActivityCodeScope`
///
/// Java 侧该枚举不实现 `MpxjEnum`，也没有 `getInstance`；Rust 侧按声明顺序提供
/// [`ActivityCodeScope::ALL`]，并用「序号（`ordinal()`）」实现 [`MpxjEnum::value`]，
/// 使 int 与枚举可以互转（`get_instance` 在 Java 侧无对应方法）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActivityCodeScope {
    /// 对应 Java: `GLOBAL`
    Global,
    /// 对应 Java: `EPS`
    Eps,
    /// 对应 Java: `PROJECT`
    Project,
}

impl ActivityCodeScope {
    /// 全部取值，顺序与 Java 声明顺序一致。
    pub const ALL: [ActivityCodeScope; 3] = [
        ActivityCodeScope::Global,
        ActivityCodeScope::Eps,
        ActivityCodeScope::Project,
    ];

    /// 按序号（Java 的 `ordinal()`）取枚举实例，越界返回 `None`。
    ///
    /// Java 侧无对应方法（Java 直接使用枚举常量或 `values()[n]`）。
    pub fn get_instance(value: i32) -> Option<ActivityCodeScope> {
        match value {
            0 => Some(ActivityCodeScope::Global),
            1 => Some(ActivityCodeScope::Eps),
            2 => Some(ActivityCodeScope::Project),
            _ => None,
        }
    }
}

impl MpxjEnum for ActivityCodeScope {
    /// 返回声明顺序对应的序号。
    ///
    /// Java 侧无 `getValue()`，此处对应 `Enum.ordinal()`。
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<ActivityCodeScope> for i32 {
    fn from(value: ActivityCodeScope) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_code_scope_round_trips() {
        for item in ActivityCodeScope::ALL {
            assert_eq!(ActivityCodeScope::get_instance(item.value()), Some(item));
        }
    }

    #[test]
    fn activity_code_scope_out_of_range() {
        assert_eq!(ActivityCodeScope::get_instance(-1), None);
        assert_eq!(ActivityCodeScope::get_instance(3), None);
    }
}
