//! 对应 Java: `org/mpxj/TimephasedWork.java`
//!
//! 跳过/未移植的 Java 方法与原因：
//!
//! | Java | Rust |
//! |---|---|
//! | `public TimephasedWork()` | `TimephasedWork::new()`（即 [`TimephasedItem::new`]，见下） |
//! | `public TimephasedWork(TimephasedWork sourceItem)` | 由 `Clone` 承担（Java 的复制构造方法与按字段复制等价，Rust 侧 `TimephasedItem` 已派生 [`Clone`]） |
//! | `public TimephasedWork(TimephasedWork sourceItem, double factor)` | [`TimephasedItem::from_work`] |
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//!
//! # 继承关系的还原方式：类型别名
//!
//! Java 的 `public final class TimephasedWork extends TimephasedItem<Duration>` 在 Rust 侧
//! 以**类型别名**承载：`pub type TimephasedWork = TimephasedItem<Duration>;`。
//! 继承而来的方法（`getStart` / `setStart` / `getTotalAmount` / ... ）直接由
//! [`TimephasedItem`] 提供，本类特有的构造方法以额外的 `impl TimephasedItem<Duration>` 块实现
//! （Rust 允许在同一 crate 内为泛型实例写 inherent impl）。
//!
//! # 与 Java 的差异
//!
//! - **`T` 的类型**：Java 是 `TimephasedItem<Duration>`（包装类型），Rust 是
//!   `TimephasedItem<crate::duration::Duration>`（值类型），取值为 `Option<Duration>`。
//! - **`from_work` 的空值处理**：Java 的实现直接对 `getTotalAmount()` /
//!   `getAmountPerHour()` 解引用，为空时抛 `NullPointerException`；Rust 侧为空时保留为
//!   `None`（不 panic），非空时按 `factor` 等比缩放并保留原单位。
//! - **`equals` / `hashCode`**：继承自 [`TimephasedItem`]，其 Rust 侧取舍见该类型的文件头
//!   （按 Java 的「四字段全部非空且相等」语义实现 `PartialEq`，不实现 `Eq` / `Hash`）。

use crate::duration::Duration;
use crate::timephased_item::TimephasedItem;

/// 分摊在一段时间上的工时。
///
/// 对应 Java: `public final class TimephasedWork extends TimephasedItem<Duration>`
///
/// （Rust 侧为 `TimephasedItem<Duration>` 的类型别名，见文件头说明。）
pub type TimephasedWork = TimephasedItem<Duration>;

impl TimephasedItem<Duration> {
    /// 按比例复制一个工时项（缩放总量与每小时工时，起止时间原样复制）。
    ///
    /// 对应 Java: `public TimephasedWork(TimephasedWork sourceItem, double factor)`
    ///
    /// Java:
    /// ```text
    /// setStart(sourceItem.getStart());
    /// setFinish(sourceItem.getFinish());
    /// setTotalAmount(Duration.getInstance(sourceItem.getTotalAmount().getDuration() * factor, sourceItem.getTotalAmount().getUnits()));
    /// setAmountPerHour(Duration.getInstance(sourceItem.getAmountPerHour().getDuration() * factor, sourceItem.getAmountPerHour().getUnits()));
    /// ```
    pub fn from_work(source_item: &TimephasedWork, factor: f64) -> TimephasedWork {
        let mut result = TimephasedWork::new();
        result.set_start(source_item.get_start());
        result.set_finish(source_item.get_finish());
        result.set_total_amount(source_item.get_total_amount().map(|value| {
            Duration::get_instance(value.get_duration() * factor, value.get_units())
        }));
        result.set_amount_per_hour(source_item.get_amount_per_hour().map(|value| {
            Duration::get_instance(value.get_duration() * factor, value.get_units())
        }));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_unit::TimeUnit;
    use chrono::NaiveDateTime;

    fn date(year: i32, month: u32, day: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|value| value.and_hms_opt(0, 0, 0))
            .expect("合法日期")
    }

    fn work() -> TimephasedWork {
        let mut item = TimephasedWork::new();
        item.set_start(Some(date(2024, 5, 1)));
        item.set_finish(Some(date(2024, 5, 2)));
        item.set_total_amount(Some(Duration::get_instance(8.0, TimeUnit::Hours)));
        item.set_amount_per_hour(Some(Duration::get_instance(1.0, TimeUnit::Hours)));
        item
    }

    #[test]
    fn new_creates_empty_item() {
        let item = TimephasedWork::new();
        assert_eq!(item.get_start(), None);
        assert_eq!(item.get_finish(), None);
        assert_eq!(item.get_total_amount(), None);
        assert_eq!(item.get_amount_per_hour(), None);
    }

    #[test]
    fn clone_copies_source_item() {
        // Java: public TimephasedWork(TimephasedWork sourceItem)
        let copy = work().clone();

        assert_eq!(copy.get_start(), Some(date(2024, 5, 1)));
        assert_eq!(
            copy.get_total_amount().map(|value| value.get_duration()),
            Some(8.0)
        );
        assert_eq!(
            copy.get_amount_per_hour().map(|value| value.get_units()),
            Some(TimeUnit::Hours)
        );
    }

    #[test]
    fn from_work_scales_amounts_and_keeps_units() {
        let scaled = TimephasedWork::from_work(&work(), 0.5);

        assert_eq!(scaled.get_start(), Some(date(2024, 5, 1)));
        assert_eq!(scaled.get_finish(), Some(date(2024, 5, 2)));
        assert_eq!(
            scaled.get_total_amount().map(|value| value.get_duration()),
            Some(4.0)
        );
        assert_eq!(
            scaled
                .get_amount_per_hour()
                .map(|value| (value.get_duration(), value.get_units())),
            Some((0.5, TimeUnit::Hours))
        );
    }

    #[test]
    fn from_work_tolerates_missing_amounts() {
        let mut source = TimephasedWork::new();
        source.set_start(Some(date(2024, 5, 1)));

        let scaled = TimephasedWork::from_work(&source, 2.0);

        assert_eq!(scaled.get_total_amount(), None);
        assert_eq!(scaled.get_amount_per_hour(), None);
        assert_eq!(scaled.get_start(), Some(date(2024, 5, 1)));
    }
}
