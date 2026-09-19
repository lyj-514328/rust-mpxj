//! 对应 Java: `org/mpxj/RelationType.java`

use crate::mpxj_enum::MpxjEnum;
use std::fmt::{self, Display, Formatter};

/// 任务关系类型的枚举。
///
/// 对应 Java: `public enum RelationType implements MpxjEnum`
///
/// Java 侧用 `EnumHelper.createTypeArray` 反射生成 int → 枚举的数组；
/// Rust 侧不使用反射，改为 [`RelationType::ALL`] 加
/// [`RelationType::get_instance`] 的 `match`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RelationType {
    /// 完成-完成。
    ///
    /// 对应 Java: `FINISH_FINISH(0, "FF")`
    FinishFinish = 0,
    /// 完成-开始。
    ///
    /// 对应 Java: `FINISH_START(1, "FS")`
    FinishStart = 1,
    /// 开始-完成。
    ///
    /// 对应 Java: `START_FINISH(2, "SF")`
    StartFinish = 2,
    /// 开始-开始。
    ///
    /// 对应 Java: `START_START(3, "SS")`
    StartStart = 3,
}

impl RelationType {
    /// 全部取值，按 int 值升序排列。
    ///
    /// 对应 Java: `private static final RelationType[] TYPE_VALUES`（不含反射部分）。
    pub const ALL: [RelationType; 4] = [
        RelationType::FinishFinish,
        RelationType::FinishStart,
        RelationType::StartFinish,
        RelationType::StartStart,
    ];

    /// 根据 int 值取枚举实例，越界时回落到 `FINISH_START`。
    ///
    /// 对应 Java: `public static RelationType getInstance(int type)`
    pub fn get_instance(value: i32) -> RelationType {
        match value {
            0 => RelationType::FinishFinish,
            1 => RelationType::FinishStart,
            2 => RelationType::StartFinish,
            3 => RelationType::StartStart,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = FINISH_START.getValue(); }`
            _ => RelationType::FinishStart,
        }
    }

    /// 对应 Java: `public static RelationType getInstance(Number type)`
    ///
    /// Java 侧 `null` 走 `getInstance(-1)`，最终回落到 `FINISH_START`。
    pub fn get_instance_optional(value: Option<i32>) -> RelationType {
        RelationType::get_instance(value.unwrap_or(-1))
    }

    /// 返回该关系类型的名称（如 `"FS"`）。
    ///
    /// 对应 Java: `private final String m_name;` 经 `toString()` 暴露。
    pub fn get_name(&self) -> &'static str {
        match self {
            RelationType::FinishFinish => "FF",
            RelationType::FinishStart => "FS",
            RelationType::StartFinish => "SF",
            RelationType::StartStart => "SS",
        }
    }
}

impl MpxjEnum for RelationType {
    /// 返回该枚举关联的 int 值。
    ///
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for RelationType {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 返回字段 `m_name`。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl From<RelationType> for i32 {
    fn from(value: RelationType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relation_type_round_trips() {
        for item in RelationType::ALL {
            assert_eq!(RelationType::get_instance(item.value()), item);
        }
    }

    #[test]
    fn relation_type_falls_back_to_finish_start() {
        assert_eq!(RelationType::get_instance(-1), RelationType::FinishStart);
        assert_eq!(RelationType::get_instance(4), RelationType::FinishStart);
        assert_eq!(
            RelationType::get_instance_optional(None),
            RelationType::FinishStart
        );
    }

    #[test]
    fn relation_type_to_string_matches_java_names() {
        assert_eq!(RelationType::FinishFinish.to_string(), "FF");
        assert_eq!(RelationType::FinishStart.to_string(), "FS");
        assert_eq!(RelationType::StartFinish.to_string(), "SF");
        assert_eq!(RelationType::StartStart.to_string(), "SS");
    }
}
