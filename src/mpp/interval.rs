//! 对应 Java: `org/mpxj/mpp/Interval.java`
//!
//! 每日 / 每周 / 每月的时间间隔（进度线的重复间隔等）。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `public enum Interval implements MpxjEnum` | 同名 `enum` + `impl crate::MpxjEnum` | 常量按 Java 声明顺序排列，判别值即 Java 的 int 值 |
//! | `private static final Interval[] TYPE_VALUES = EnumHelper.createTypeArray(Interval.class)` | [`Interval::get_instance`] 的 `match` + [`Interval::ALL`] | 表长 3（偏移 0），取值 0..2 全部被填充，故 Java 的 `getInstance` 不会返回 `null` |
//! | `getInstance(Number type)` | [`Interval::get_instance_optional`] | Rust 无重载，加 `_optional` 后缀 |
//! | `getName()` / `toString()` | [`Interval::get_name`] / `Display` | 名称未本地化，与 Java 一致 |
//!
//! 跳过的 Java 成员：`EnumHelper.createTypeArray` 的反射建表（Rust 侧以 `match` + `ALL` 代替）。
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// 时间间隔。
///
/// 对应 Java: `public enum Interval`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Interval {
    /// 对应 Java: `DAILY(0, "Daily")`
    Daily = 0,
    /// 对应 Java: `WEEKLY(1, "Weekly")`
    Weekly = 1,
    /// 对应 Java: `MONTHLY(2, "Monthly")`
    Monthly = 2,
}

impl Interval {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `private static final Interval[] TYPE_VALUES`（去掉反射建表后）。
    pub const ALL: [Interval; 3] = [Interval::Daily, Interval::Weekly, Interval::Monthly];

    /// 按 int 值取枚举实例。
    ///
    /// 对应 Java: `public static Interval getInstance(int type)`
    ///
    /// 越界时 Java 先回落到 `DAILY.getValue()`（0）再取表，故不会返回 `null`。
    pub fn get_instance(value: i32) -> Interval {
        match value {
            0 => Interval::Daily,
            1 => Interval::Weekly,
            2 => Interval::Monthly,
            // Java: type < 0 || type >= TYPE_VALUES.length → type = DAILY.getValue()
            _ => Interval::Daily,
        }
    }

    /// 对应 Java: `public static Interval getInstance(Number type)`
    ///
    /// Java 侧 `null` 会走 `getInstance(-1)`，最终回落到 `DAILY`。
    pub fn get_instance_optional(value: Option<i32>) -> Interval {
        Interval::get_instance(value.unwrap_or(-1))
    }

    /// 取间隔名称（未做本地化）。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            Interval::Daily => "Daily",
            Interval::Weekly => "Weekly",
            Interval::Monthly => "Monthly",
        }
    }

    /// 对应 Java: `@Override public int getValue()`
    pub fn get_value(&self) -> i32 {
        *self as i32
    }
}

impl Display for Interval {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 返回 `getName()`，Rust 侧一致。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl MpxjEnum for Interval {
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.get_value()
    }
}

impl From<Interval> for i32 {
    /// Rust 特有的便捷转换，等价于 [`Interval::get_value`]。
    fn from(value: Interval) -> i32 {
        value.get_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 判别值即 Java 的 int 值，且 `getInstance` 往返一致。
    #[test]
    fn round_trips_through_int_values() {
        for (index, item) in Interval::ALL.iter().enumerate() {
            assert_eq!(item.get_value(), index as i32);
            assert_eq!(Interval::get_instance(index as i32), *item);
            assert_eq!(MpxjEnum::value(item), index as i32);
        }
    }

    /// 越界回落 DAILY（Java 不会返回 null）。
    #[test]
    fn out_of_range_falls_back_to_daily() {
        for value in [-1, -100, 3, 4, 1000, i32::MAX, i32::MIN] {
            assert_eq!(Interval::get_instance(value), Interval::Daily);
        }
        assert_eq!(Interval::get_instance_optional(None), Interval::Daily);
        assert_eq!(
            Interval::get_instance_optional(Some(2)),
            Interval::Monthly
        );
    }

    /// 名称与 `toString()` 与 Java 一致。
    #[test]
    fn names_and_to_string_match_java() {
        assert_eq!(Interval::Daily.get_name(), "Daily");
        assert_eq!(Interval::Weekly.to_string(), "Weekly");
        assert_eq!(Interval::Monthly.to_string(), "Monthly");
    }
}
