//! 对应 Java: `org/mpxj/common/DayOfWeekHelper.java`

use chrono::Weekday;

/// 处理 `DayOfWeek`（Rust 侧为 [`Weekday`]）的工具方法。
///
/// 对应 Java: `public final class DayOfWeekHelper`
///
/// 移植说明：
/// - Java 的 `getValue` 对 null 会抛 `NullPointerException`；Rust 侧参数为非可空的 [`Weekday`]，
///   其 `switch` 末尾的 `return 0` 在枚举穷举下不可达，Rust 的穷举 `match` 无需该分支。
/// - `getInstance(int)` 中 Java 的 `--type` 溢出时回绕，Rust 侧用 `wrapping_sub` 保持一致。
///
/// 未移植的方法：无。
pub struct DayOfWeekHelper;

impl DayOfWeekHelper {
    /// 对应 Java: `public static DayOfWeek getInstance(int type)`
    ///
    /// 1 表示周日，7 表示周六；越界返回 `None`。
    pub fn get_instance(r#type: i32) -> Option<Weekday> {
        let index = r#type.wrapping_sub(1);

        if index < 0 || index as usize >= Self::ORDERED_DAYS.len() {
            return None;
        }

        Some(Self::ORDERED_DAYS[index as usize])
    }

    /// 对应 Java: `public static int getValue(DayOfWeek day)`
    ///
    /// 周日返回 1，周六返回 7（与 `java.time.DayOfWeek` 自带的序号不同）。
    pub fn get_value(day: Weekday) -> i32 {
        match day {
            Weekday::Sun => 1,
            Weekday::Mon => 2,
            Weekday::Tue => 3,
            Weekday::Wed => 4,
            Weekday::Thu => 5,
            Weekday::Fri => 6,
            Weekday::Sat => 7,
        }
    }

    /// 对应 Java: `public static final DayOfWeek[] ORDERED_DAYS`
    ///
    /// int 值到枚举的映射表，下标 0 对应 int 值 1（周日）。
    pub const ORDERED_DAYS: [Weekday; 7] = [
        Weekday::Sun,
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
    ];
}
