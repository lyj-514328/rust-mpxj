//! 对应 Java: `org/mpxj/CalendarType.java`

use crate::mpxj_enum::MpxjEnum;

/// P6 日历类型。
///
/// 对应 Java: `public enum CalendarType`
///
/// Java 侧该枚举不实现 `MpxjEnum`，也没有 `getInstance`；Rust 侧按声明顺序提供
/// [`CalendarType::ALL`]，并用「序号（`ordinal()`）」实现 [`MpxjEnum::value`]，
/// 使 int 与枚举可以互转（`get_instance` 在 Java 侧无对应方法）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CalendarType {
    /// 对应 Java: `GLOBAL`
    Global,
    /// 对应 Java: `PROJECT`
    Project,
    /// 对应 Java: `RESOURCE`
    Resource,
}

impl CalendarType {
    /// 全部取值，顺序与 Java 声明顺序一致。
    pub const ALL: [CalendarType; 3] = [
        CalendarType::Global,
        CalendarType::Project,
        CalendarType::Resource,
    ];

    /// 按序号（Java 的 `ordinal()`）取枚举实例，越界返回 `None`。
    ///
    /// Java 侧无对应方法（Java 直接使用枚举常量或 `values()[n]`）。
    pub fn get_instance(value: i32) -> Option<CalendarType> {
        match value {
            0 => Some(CalendarType::Global),
            1 => Some(CalendarType::Project),
            2 => Some(CalendarType::Resource),
            _ => None,
        }
    }
}

impl MpxjEnum for CalendarType {
    /// 返回声明顺序对应的序号。
    ///
    /// Java 侧无 `getValue()`，此处对应 `Enum.ordinal()`。
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<CalendarType> for i32 {
    fn from(value: CalendarType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_type_round_trips() {
        for item in CalendarType::ALL {
            assert_eq!(CalendarType::get_instance(item.value()), Some(item));
        }
    }

    #[test]
    fn calendar_type_out_of_range() {
        assert_eq!(CalendarType::get_instance(-1), None);
        assert_eq!(CalendarType::get_instance(3), None);
    }
}
