//! 对应 Java: `org/mpxj/PercentCompleteType.java`

use crate::mpxj_enum::MpxjEnum;

/// 完成百分比的含义（按工期 / 按实际完成量 / 按资源单位 / 按范围）。
///
/// 对应 Java: `public enum PercentCompleteType`
///
/// Java 侧该枚举不实现 `MpxjEnum`，也没有 `getInstance`；Rust 侧按声明顺序提供
/// [`PercentCompleteType::ALL`]，并用「序号（`ordinal()`）」实现 [`MpxjEnum::value`]，
/// 使 int 与枚举可以互转（`get_instance` 在 Java 侧无对应方法）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PercentCompleteType {
    /// 对应 Java: `DURATION`
    Duration,
    /// 对应 Java: `PHYSICAL`
    Physical,
    /// 对应 Java: `UNITS`
    Units,
    /// 对应 Java: `SCOPE`
    Scope,
}

impl PercentCompleteType {
    /// 全部取值，顺序与 Java 声明顺序一致。
    pub const ALL: [PercentCompleteType; 4] = [
        PercentCompleteType::Duration,
        PercentCompleteType::Physical,
        PercentCompleteType::Units,
        PercentCompleteType::Scope,
    ];

    /// 按序号（Java 的 `ordinal()`）取枚举实例，越界返回 `None`。
    ///
    /// Java 侧无对应方法（Java 直接使用枚举常量或 `values()[n]`）。
    pub fn get_instance(value: i32) -> Option<PercentCompleteType> {
        match value {
            0 => Some(PercentCompleteType::Duration),
            1 => Some(PercentCompleteType::Physical),
            2 => Some(PercentCompleteType::Units),
            3 => Some(PercentCompleteType::Scope),
            _ => None,
        }
    }
}

impl MpxjEnum for PercentCompleteType {
    /// 返回声明顺序对应的序号。
    ///
    /// Java 侧无 `getValue()`，此处对应 `Enum.ordinal()`。
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<PercentCompleteType> for i32 {
    fn from(value: PercentCompleteType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_complete_type_round_trips() {
        for item in PercentCompleteType::ALL {
            assert_eq!(PercentCompleteType::get_instance(item.value()), Some(item));
        }
    }

    #[test]
    fn percent_complete_type_out_of_range() {
        assert_eq!(PercentCompleteType::get_instance(-1), None);
        assert_eq!(PercentCompleteType::get_instance(4), None);
    }
}
