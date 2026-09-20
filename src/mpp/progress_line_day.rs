//! 对应 Java: `org/mpxj/mpp/ProgressLineDay.java`
//!
//! 进度线（progress line）绘制时机所用的「天」枚举。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `public enum ProgressLineDay implements MpxjEnum` | 同名 `enum` + `impl crate::MpxjEnum` | 常量按 Java 声明顺序排列，判别值即 Java 的 int 值 |
//! | 构造参数 `java.time.DayOfWeek day` / `DayOfWeek getDay()` | `Option<chrono::Weekday>` | `java.time.DayOfWeek` 在 Rust 侧统一用 [`chrono::Weekday`] 表示（与 `crate::common::day_of_week_helper::DayOfWeekHelper` 一致）；Java 中 `DAY` / `WORKINGDAY` / `NONWORKINGDAY` 对应的值是 `null`，故用 `Option` |
//! | `private static final ProgressLineDay[] TYPE_VALUES = EnumHelper.createTypeArray(ProgressLineDay.class, 1)` | [`ProgressLineDay::get_instance`] 的 `match` + [`ProgressLineDay::ALL`] | 表长为常量数 10 + 偏移 1 = 11：取值 1..10 被填充，**下标 0 保持 `null`**；且 Java 对越界值直接返回 `null`（不回落默认值） |
//! | `getInstance(int)` 可能返回 `null` | `Option<ProgressLineDay>` | 「`null` → `Option<T>`」约定 |
//! | `getNextDay()`（`value + 1`，超过 7 时回到 1，再查表） | [`ProgressLineDay::get_next_day`]，返回非可空的 `ProgressLineDay` | `value + 1` 超过 7 时一律回到 1，故结果总落在 1..=7（`SATURDAY` 以及 `DAY` / `WORKINGDAY` / `NONWORKINGDAY` 都得到 `SUNDAY`），查表必定命中，Java 侧不会返回 `null` |
//! | 未覆写 `toString()`（继承 `java.lang.Enum.toString()`，输出**常量名**） | `Display`，输出 `SUNDAY` / `MONDAY` / … / `NONWORKINGDAY` | Java 枚举即使用户未覆写也有 `toString()`，其值就是常量名；Rust 侧按此实现 `Display`（供 `GanttChartView` 的字符串表示等使用） |
//!
//! 注意：Java 侧本类**没有** `getInstance(Number)` 重载，因此 Rust 侧也不提供
//! `get_instance_optional`。
//!
//! 跳过的 Java 成员：`EnumHelper.createTypeArray` 的反射建表（Rust 侧以 `match` + `ALL` 代替）。
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::fmt::{self, Display, Formatter};

use chrono::Weekday;

use crate::mpxj_enum::MpxjEnum;

/// 进度线绘制时机所用的「天」。
///
/// 对应 Java: `public enum ProgressLineDay`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProgressLineDay {
    /// 对应 Java: `SUNDAY(1, DayOfWeek.SUNDAY)`
    Sunday = 1,
    /// 对应 Java: `MONDAY(2, DayOfWeek.MONDAY)`
    Monday = 2,
    /// 对应 Java: `TUESDAY(3, DayOfWeek.TUESDAY)`
    Tuesday = 3,
    /// 对应 Java: `WEDNESDAY(4, DayOfWeek.WEDNESDAY)`
    Wednesday = 4,
    /// 对应 Java: `THURSDAY(5, DayOfWeek.THURSDAY)`
    Thursday = 5,
    /// 对应 Java: `FRIDAY(6, DayOfWeek.FRIDAY)`
    Friday = 6,
    /// 对应 Java: `SATURDAY(7, DayOfWeek.SATURDAY)`
    Saturday = 7,
    /// 对应 Java: `DAY(8, null)`
    Day = 8,
    /// 对应 Java: `WORKINGDAY(9, null)`
    Workingday = 9,
    /// 对应 Java: `NONWORKINGDAY(10, null)`
    Nonworkingday = 10,
}

impl ProgressLineDay {
    /// 全部取值，顺序与 Java 声明顺序一致。
    ///
    /// 对应 Java: `private static final ProgressLineDay[] TYPE_VALUES`（去掉反射建表后）。
    pub const ALL: [ProgressLineDay; 10] = [
        ProgressLineDay::Sunday,
        ProgressLineDay::Monday,
        ProgressLineDay::Tuesday,
        ProgressLineDay::Wednesday,
        ProgressLineDay::Thursday,
        ProgressLineDay::Friday,
        ProgressLineDay::Saturday,
        ProgressLineDay::Day,
        ProgressLineDay::Workingday,
        ProgressLineDay::Nonworkingday,
    ];

    /// 按 int 值取枚举实例；越界（含 0，Java 的表中该下标为 `null`）返回 `None`。
    ///
    /// 对应 Java: `public static ProgressLineDay getInstance(int type)`
    ///
    /// 与其它枚举不同，Java 侧这里**不做**回落默认值的处理：越界直接返回 `null`。
    pub fn get_instance(value: i32) -> Option<ProgressLineDay> {
        match value {
            1 => Some(ProgressLineDay::Sunday),
            2 => Some(ProgressLineDay::Monday),
            3 => Some(ProgressLineDay::Tuesday),
            4 => Some(ProgressLineDay::Wednesday),
            5 => Some(ProgressLineDay::Thursday),
            6 => Some(ProgressLineDay::Friday),
            7 => Some(ProgressLineDay::Saturday),
            8 => Some(ProgressLineDay::Day),
            9 => Some(ProgressLineDay::Workingday),
            10 => Some(ProgressLineDay::Nonworkingday),
            // Java: type < 0 || type >= TYPE_VALUES.length（11）→ null；
            //       下标 0 未被填充，同样是 null
            _ => None,
        }
    }

    /// 对应 Java: `@Override public int getValue()`
    pub fn get_value(&self) -> i32 {
        *self as i32
    }

    /// 取与该取值等价的星期；`DAY` / `WORKINGDAY` / `NONWORKINGDAY` 返回 `None`。
    ///
    /// 对应 Java: `public DayOfWeek getDay()`
    pub fn get_day(&self) -> Option<Weekday> {
        match self {
            ProgressLineDay::Sunday => Some(Weekday::Sun),
            ProgressLineDay::Monday => Some(Weekday::Mon),
            ProgressLineDay::Tuesday => Some(Weekday::Tue),
            ProgressLineDay::Wednesday => Some(Weekday::Wed),
            ProgressLineDay::Thursday => Some(Weekday::Thu),
            ProgressLineDay::Friday => Some(Weekday::Fri),
            ProgressLineDay::Saturday => Some(Weekday::Sat),
            // Java: m_day == null
            ProgressLineDay::Day | ProgressLineDay::Workingday | ProgressLineDay::Nonworkingday => {
                None
            }
        }
    }

    /// 取序列中的下一天（周日 → 周一 会自动回绕）。
    ///
    /// 对应 Java: `public ProgressLineDay getNextDay()`
    ///
    /// Java 实现为 `value + 1`，超过 7 时回到 1，再交给 `getInstance`：因此
    /// 周内取值正常递增（`SUNDAY` → `MONDAY` …），而 `SATURDAY`(7)、`DAY`(8)、
    /// `WORKINGDAY`(9)、`NONWORKINGDAY`(10) 都回绕到 `SUNDAY`(1)。结果必定命中
    /// 查找表，与 Java 一样不会返回空值（`unwrap_or` 分支仅用于避免 `unwrap`，
    /// 实际不会命中）。
    pub fn get_next_day(&self) -> ProgressLineDay {
        let mut value = self.get_value() + 1;
        if value > 7 {
            value = 1;
        }
        ProgressLineDay::get_instance(value).unwrap_or(ProgressLineDay::Sunday)
    }
}

impl Display for ProgressLineDay {
    /// 对应 Java: 继承自 `java.lang.Enum` 的 `toString()`
    ///
    /// Java 的 `Enum.toString()` 返回常量名（如 `MONDAY`），本类未覆写，故 Rust 侧
    /// 输出同样的常量名。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ProgressLineDay::Sunday => "SUNDAY",
            ProgressLineDay::Monday => "MONDAY",
            ProgressLineDay::Tuesday => "TUESDAY",
            ProgressLineDay::Wednesday => "WEDNESDAY",
            ProgressLineDay::Thursday => "THURSDAY",
            ProgressLineDay::Friday => "FRIDAY",
            ProgressLineDay::Saturday => "SATURDAY",
            ProgressLineDay::Day => "DAY",
            ProgressLineDay::Workingday => "WORKINGDAY",
            ProgressLineDay::Nonworkingday => "NONWORKINGDAY",
        })
    }
}

impl MpxjEnum for ProgressLineDay {
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.get_value()
    }
}

impl From<ProgressLineDay> for i32 {
    /// Rust 特有的便捷转换，等价于 [`ProgressLineDay::get_value`]。
    fn from(value: ProgressLineDay) -> i32 {
        value.get_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 判别值即 Java 的 int 值，且 `getInstance` 往返一致。
    #[test]
    fn round_trips_through_int_values() {
        for (index, item) in ProgressLineDay::ALL.iter().enumerate() {
            assert_eq!(item.get_value(), index as i32 + 1);
            assert_eq!(
                ProgressLineDay::get_instance(item.get_value()),
                Some(*item)
            );
            assert_eq!(MpxjEnum::value(item), index as i32 + 1);
        }
    }

    /// Java 的查找表下标 0 为 null，越界也不回落默认值。
    #[test]
    fn missing_slots_return_none() {
        for value in [-1, 0, 11, 100, i32::MAX, i32::MIN] {
            assert_eq!(ProgressLineDay::get_instance(value), None, "下标 {value}");
        }
    }

    /// `getDay()` 映射与 `java.time.DayOfWeek` 一致，非星期取值返回 None。
    #[test]
    fn get_day_matches_java_day_of_week() {
        assert_eq!(ProgressLineDay::Sunday.get_day(), Some(Weekday::Sun));
        assert_eq!(ProgressLineDay::Monday.get_day(), Some(Weekday::Mon));
        assert_eq!(ProgressLineDay::Tuesday.get_day(), Some(Weekday::Tue));
        assert_eq!(ProgressLineDay::Wednesday.get_day(), Some(Weekday::Wed));
        assert_eq!(ProgressLineDay::Thursday.get_day(), Some(Weekday::Thu));
        assert_eq!(ProgressLineDay::Friday.get_day(), Some(Weekday::Fri));
        assert_eq!(ProgressLineDay::Saturday.get_day(), Some(Weekday::Sat));

        assert_eq!(ProgressLineDay::Day.get_day(), None);
        assert_eq!(ProgressLineDay::Workingday.get_day(), None);
        assert_eq!(ProgressLineDay::Nonworkingday.get_day(), None);
    }

    /// `getNextDay()` 的序列与回绕行为与 Java 一致。
    #[test]
    fn get_next_day_wraps_and_handles_special_values() {
        assert_eq!(
            ProgressLineDay::Sunday.get_next_day(),
            ProgressLineDay::Monday
        );
        assert_eq!(
            ProgressLineDay::Monday.get_next_day(),
            ProgressLineDay::Tuesday
        );
        // Java: 超过 7 时回到 1，因此 SATURDAY(7) 的下一天是 SUNDAY(1)
        assert_eq!(
            ProgressLineDay::Saturday.get_next_day(),
            ProgressLineDay::Sunday
        );
        // 值超过 7 时回到 1：DAY / WORKINGDAY / NONWORKINGDAY 都得到 SUNDAY
        for day in [
            ProgressLineDay::Day,
            ProgressLineDay::Workingday,
            ProgressLineDay::Nonworkingday,
        ] {
            assert_eq!(day.get_next_day(), ProgressLineDay::Sunday, "{day:?}");
        }
    }

    /// `toString()`（Java 的 `Enum.toString()`）输出常量名。
    #[test]
    fn to_string_outputs_the_java_constant_name() {
        assert_eq!(ProgressLineDay::Sunday.to_string(), "SUNDAY");
        assert_eq!(ProgressLineDay::Monday.to_string(), "MONDAY");
        assert_eq!(ProgressLineDay::Saturday.to_string(), "SATURDAY");
        assert_eq!(ProgressLineDay::Day.to_string(), "DAY");
        assert_eq!(ProgressLineDay::Workingday.to_string(), "WORKINGDAY");
        assert_eq!(ProgressLineDay::Nonworkingday.to_string(), "NONWORKINGDAY");
        assert_eq!(i32::from(ProgressLineDay::Nonworkingday), 10);
        assert_eq!(format!("{:?}", ProgressLineDay::Sunday), "Sunday");
    }
}
