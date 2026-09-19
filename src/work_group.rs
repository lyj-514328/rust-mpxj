//! 对应 Java: `org/mpxj/WorkGroup.java`

use crate::mpxj_enum::MpxjEnum;

/// 资源工作组类型的枚举。
///
/// 对应 Java: `public enum WorkGroup implements MpxjEnum`
///
/// Java 侧用 `EnumHelper.createTypeArray` 反射生成 int → 枚举的数组；
/// Rust 侧不使用反射，改为 [`WorkGroup::ALL`] 加 [`WorkGroup::get_instance`] 的 `match`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WorkGroup {
    /// 默认。
    ///
    /// 对应 Java: `DEFAULT(0)`
    Default = 0,
    /// 无。
    ///
    /// 对应 Java: `NONE(1)`
    None = 1,
    /// 电子邮件。
    ///
    /// 对应 Java: `EMAIL(2)`
    Email = 2,
    /// Web。
    ///
    /// 对应 Java: `WEB(3)`
    Web = 3,
}

impl WorkGroup {
    /// 全部取值，按 int 值升序排列。
    ///
    /// 对应 Java: `private static final WorkGroup[] TYPE_VALUES`（不含反射部分）。
    pub const ALL: [WorkGroup; 4] = [
        WorkGroup::Default,
        WorkGroup::None,
        WorkGroup::Email,
        WorkGroup::Web,
    ];

    /// 根据 int 值取枚举实例，越界时回落到 `NONE`。
    ///
    /// 对应 Java: `public static WorkGroup getInstance(int type)`
    pub fn get_instance(value: i32) -> WorkGroup {
        match value {
            0 => WorkGroup::Default,
            1 => WorkGroup::None,
            2 => WorkGroup::Email,
            3 => WorkGroup::Web,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = NONE.getValue(); }`
            _ => WorkGroup::None,
        }
    }

    /// 对应 Java: `public static WorkGroup getInstance(Number type)`
    ///
    /// Java 侧 `null` 走 `getInstance(-1)`，最终回落到 `NONE`。
    pub fn get_instance_optional(value: Option<i32>) -> WorkGroup {
        WorkGroup::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for WorkGroup {
    /// 返回该枚举关联的 int 值。
    ///
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<WorkGroup> for i32 {
    fn from(value: WorkGroup) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_group_round_trips() {
        for item in WorkGroup::ALL {
            assert_eq!(WorkGroup::get_instance(item.value()), item);
        }
    }

    #[test]
    fn work_group_falls_back_to_none() {
        assert_eq!(WorkGroup::get_instance(-1), WorkGroup::None);
        assert_eq!(WorkGroup::get_instance(4), WorkGroup::None);
        assert_eq!(WorkGroup::get_instance_optional(None), WorkGroup::None);
    }
}
