//! 对应 Java: `org/mpxj/RecurringTask.java`
//!
//! 表示 MPX 文件里的「重复任务记录」（recurring task record）。
//!
//! # 与 Java 的差异：用 `Deref` 还原 `extends`
//!
//! Java 侧 `RecurringTask extends RecurringData`，且**不覆写**父类任何方法，
//! 只新增 `getDuration()` / `setDuration()` 并覆写 `toString()`。
//! Rust 无继承，本类型用**组合 + `Deref`/`DerefMut`** 还原：持有一个
//! [`RecurringData`]，通过 `Deref` 让父类的全部方法可以直接在 `RecurringTask`
//! 上调用（`task.get_start_date()` 即 `task.data.get_start_date()`），
//! 从而免去约 30 个纯转发的样板方法。
//!
//! 本文件是全 crate 唯一使用 `Deref` 还原继承的地方 —— 之所以可以这样，
//! 正是因为这里**没有任何方法覆写**。凡是子类需要覆写父类行为的场景，
//! 本 crate 其余位置一律采用显式「组合 + 转发」，以便逐方法对照 Java。

use std::fmt;

use crate::duration::Duration;
use crate::recurring_data::RecurringData;

/// 对应 Java: `public final class RecurringTask extends RecurringData`
pub struct RecurringTask {
    /// 对应 Java: 父类 `RecurringData` 的全部字段
    data: RecurringData,

    /// 对应 Java: `private Duration m_duration;`
    duration: Option<Duration>,
}

impl RecurringTask {
    /// 对应 Java: 默认构造方法（Java 由编译器隐式生成）
    pub fn new() -> Self {
        Self {
            data: RecurringData::new(),
            duration: None,
        }
    }

    /// 取重复任务的工期。
    ///
    /// 对应 Java: `public Duration getDuration()`
    pub fn get_duration(&self) -> Option<Duration> {
        self.duration
    }

    /// 设置重复任务的工期。
    ///
    /// 对应 Java: `public void setDuration(Duration duration)`
    pub fn set_duration(&mut self, duration: Option<Duration>) {
        self.duration = duration;
    }

    /// 取父类数据的只读引用（Rust 特有，便于需要显式访问 `RecurringData` 的场景）。
    pub fn get_recurring_data(&self) -> &RecurringData {
        &self.data
    }

    /// 取父类数据的可变引用（Rust 特有）。
    pub fn get_recurring_data_mut(&mut self) -> &mut RecurringData {
        &mut self.data
    }
}

impl Default for RecurringTask {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for RecurringTask {
    type Target = RecurringData;

    /// 对应 Java: 继承关系带来的「父类方法可直接调用」
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl std::ops::DerefMut for RecurringTask {
    /// 对应 Java: 继承关系带来的「父类方法可直接调用」
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl fmt::Display for RecurringTask {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 侧仅当 `m_duration != null` 时才追加 `super.toString()`，
    /// 且最后一行用 `println`（带换行）。这里逐项保留。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[RecurringTask")?;

        if self.duration.is_some() {
            write!(f, "{}", self.data)?;
        }

        writeln!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recurrence_type::RecurrenceType;
    use chrono::NaiveDate;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn duration_accessors_round_trip() {
        let mut task = RecurringTask::new();
        assert_eq!(task.get_duration(), None);

        task.set_duration(Some(Duration::get_instance_int(3, crate::time_unit::TimeUnit::Days)));

        assert!(task.get_duration().is_some());
        // Default 等价于隐式无参构造
        assert_eq!(RecurringTask::default().get_duration(), None);
    }

    #[test]
    fn parent_methods_are_reachable_through_deref() {
        // 对应 Java: 继承使父类方法可直接调用
        let mut task = RecurringTask::new();

        task.set_recurrence_type(Some(RecurrenceType::Daily));
        task.set_start_date(Some(date(2026, 9, 20)));
        task.set_occurrences(Some(2));

        assert_eq!(task.get_recurrence_type(), Some(RecurrenceType::Daily));
        assert_eq!(task.get_start_date(), Some(date(2026, 9, 20)));
        assert_eq!(task.get_occurrences(), Some(2));
    }

    #[test]
    fn display_omits_parent_when_duration_is_absent() {
        // 对应 Java: `m_duration == null` 时不打印 super.toString()
        let task = RecurringTask::new();
        assert_eq!(task.to_string(), "[RecurringTask]\n");
    }

    #[test]
    fn display_includes_parent_when_duration_is_present() {
        let mut task = RecurringTask::new();
        task.set_duration(Some(Duration::get_instance_int(1, crate::time_unit::TimeUnit::Days)));

        let text = task.to_string();
        assert!(text.starts_with("[RecurringTask"));
        assert!(text.ends_with("]\n"));
        // 有 duration 时会把父类的 Display 内容一并带上
        assert!(text.len() > "[RecurringTask]\n".len());
    }
}
