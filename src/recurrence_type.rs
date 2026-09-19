//! 对应 Java: `org/mpxj/RecurrenceType.java`

use crate::mpxj_enum::MpxjEnum;
use std::fmt::{self, Display, Formatter};

/// 重复（周期）类型的枚举。
///
/// 对应 Java: `public enum RecurrenceType implements MpxjEnum`
///
/// Java 侧用 `EnumHelper.createTypeArray(RecurrenceType.class, 13)` 反射生成查找数组：
/// 枚举只有 4 个取值，数组长度却是 `4 + 13 = 17`，因此下标 2、3、5、6、7、9…15
/// 都是空洞。`getInstance(int)` 在 `type < 1 || type >= 17` 时回落到 `DAILY`，
/// 命中空洞时返回 `null`。
///
/// Rust 侧不使用反射，改为 [`RecurrenceType::ALL`] 加 [`RecurrenceType::get_instance`]
/// 的 `match`；为忠实表达 Java 可能返回 `null` 的行为，本方法返回 `Option<RecurrenceType>`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RecurrenceType {
    /// 每天。
    ///
    /// 对应 Java: `DAILY(1, "Daily")`
    Daily = 1,
    /// 每周。
    ///
    /// 对应 Java: `WEEKLY(4, "Weekly")`
    Weekly = 4,
    /// 每月。
    ///
    /// 对应 Java: `MONTHLY(8, "Monthly")`
    Monthly = 8,
    /// 每年。
    ///
    /// 对应 Java: `YEARLY(16, "Yearly")`
    Yearly = 16,
}

impl RecurrenceType {
    /// 全部取值，按 int 值升序排列。
    ///
    /// 对应 Java: `private static final RecurrenceType[] TYPE_VALUES`（不含反射部分与空洞）。
    pub const ALL: [RecurrenceType; 4] = [
        RecurrenceType::Daily,
        RecurrenceType::Weekly,
        RecurrenceType::Monthly,
        RecurrenceType::Yearly,
    ];

    /// Java 查找数组的长度：`EnumHelper.createTypeArray(RecurrenceType.class, 13)`
    /// = 枚举个数 4 + 偏移 13。
    const TYPE_VALUES_LENGTH: i32 = 17;

    /// 根据 int 值取枚举实例。
    ///
    /// 越界（`type < 1 || type >= 17`）回落到 `DAILY`；落在查找数组空洞中的值
    /// 对应 Java 的 `null`，此处返回 `None`。
    ///
    /// 对应 Java: `public static RecurrenceType getInstance(int type)`
    pub fn get_instance(value: i32) -> Option<RecurrenceType> {
        match value {
            1 => Some(RecurrenceType::Daily),
            4 => Some(RecurrenceType::Weekly),
            8 => Some(RecurrenceType::Monthly),
            16 => Some(RecurrenceType::Yearly),
            // Java: `if (type < 1 || type >= TYPE_VALUES.length) { type = DAILY.getValue(); }`
            _ if value < 1 || value >= RecurrenceType::TYPE_VALUES_LENGTH => {
                Some(RecurrenceType::Daily)
            }
            // Java: 查找数组中的空洞，返回 null。
            _ => None,
        }
    }

    /// 返回该重复类型对应的名称（如 `"Daily"`）。
    ///
    /// 对应 Java: `private final String m_name;` 经 `toString()` 暴露。
    pub fn get_name(&self) -> &'static str {
        match self {
            RecurrenceType::Daily => "Daily",
            RecurrenceType::Weekly => "Weekly",
            RecurrenceType::Monthly => "Monthly",
            RecurrenceType::Yearly => "Yearly",
        }
    }
}

impl MpxjEnum for RecurrenceType {
    /// 返回该枚举关联的 int 值。
    ///
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for RecurrenceType {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 返回字段 `m_name`。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl From<RecurrenceType> for i32 {
    fn from(value: RecurrenceType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recurrence_type_round_trips() {
        for item in RecurrenceType::ALL {
            assert_eq!(RecurrenceType::get_instance(item.value()), Some(item));
        }
    }

    #[test]
    fn recurrence_type_falls_back_to_daily() {
        assert_eq!(RecurrenceType::get_instance(0), Some(RecurrenceType::Daily));
        assert_eq!(RecurrenceType::get_instance(-1), Some(RecurrenceType::Daily));
        assert_eq!(RecurrenceType::get_instance(17), Some(RecurrenceType::Daily));
        assert_eq!(RecurrenceType::get_instance(100), Some(RecurrenceType::Daily));
    }

    #[test]
    fn recurrence_type_returns_none_for_lookup_array_holes() {
        // Java 侧 createTypeArray 的偏移 13 造成空洞，getInstance 返回 null。
        for value in [2, 3, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15] {
            assert_eq!(RecurrenceType::get_instance(value), None);
        }
    }

    #[test]
    fn recurrence_type_to_string_matches_java_names() {
        assert_eq!(RecurrenceType::Daily.to_string(), "Daily");
        assert_eq!(RecurrenceType::Weekly.to_string(), "Weekly");
        assert_eq!(RecurrenceType::Monthly.to_string(), "Monthly");
        assert_eq!(RecurrenceType::Yearly.to_string(), "Yearly");
    }
}
