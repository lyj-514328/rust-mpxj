//! 对应 Java: `org/mpxj/TimephasedItem.java`
//!
//! 跳过/未移植的 Java 方法与原因：
//!
//! | Java | Rust |
//! |---|---|
//! | 隐式默认构造方法 | [`TimephasedItem::new`] |
//! | `public LocalDateTime getStart()` / `public void setStart(LocalDateTime start)` | [`TimephasedItem::get_start`] / [`TimephasedItem::set_start`] |
//! | `public T getAmountPerHour()` / `public void setAmountPerHour(T amountPerHour)` | [`TimephasedItem::get_amount_per_hour`] / [`TimephasedItem::set_amount_per_hour`] |
//! | `public T getTotalAmount()` / `public void setTotalAmount(T totalAmount)` | [`TimephasedItem::get_total_amount`] / [`TimephasedItem::set_total_amount`] |
//! | `public LocalDateTime getFinish()` / `public void setFinish(LocalDateTime finish)` | [`TimephasedItem::get_finish`] / [`TimephasedItem::set_finish`] |
//! | `@Override public String toString()` | [`Display`] |
//! | `@Override public boolean equals(Object o)` | [`PartialEq`]（语义见下） |
//! | `@Override public int hashCode()` | **未移植**（见下） |
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//!
//! # 与 Java 的差异
//!
//! 1. **泛型载荷**：Java 的 `TimephasedItem<T>` 用包装类型 `T`（`Duration` / `Number`），
//!    Rust 侧 `T` 是值类型（`Duration` / `f64`），取值为 `Option<T>`（Java 的可空引用）。
//!    getter 要求 `T: Copy`（本 crate 的两处实例化 `Duration` 与 `f64` 都满足）。
//! 2. **`equals` 的语义**：Java 的 `equals` 要求**四个字段全部非空且相等**才返回 `true`
//!    （两边的字段都为 `null` 时返回 `false`）。Rust 侧 [`PartialEq`] 按同样规则实现，
//!    因此该「相等」关系**不满足自反性**，无法实现 Rust 的 [`Eq`]（`Eq` 要求 `a == a`
//!    恒成立），`hashCode` 因此也没有配套实现（`Hash` 在没有 `Eq` 时不能用于 `HashMap`）。
//!    MPXJ 自身也没有把 `TimephasedItem` 当作哈希键使用的场景。
//! 3. **`toString()` 的 `null`**：Java 字符串拼接把 `null` 渲染为 `"null"`，Rust 侧同样处理。

use std::fmt::{self, Display, Formatter};

use chrono::NaiveDateTime;

/// 分摊在一段时间上的数量（工时或成本）。
///
/// 对应 Java: `public abstract class TimephasedItem<T>`
///
/// # 继承关系的还原方式
///
/// Java 中 [`crate::timephased_work::TimephasedWork`] 与
/// [`crate::timephased_cost::TimephasedCost`] 分别 `extends TimephasedItem<Duration>` /
/// `TimephasedItem<Number>`。Rust 没有继承，这两个类型以**类型别名**承载继承关系
/// （`TimephasedItem<Duration>` / `TimephasedItem<f64>`），各自的专属构造方法以额外的
/// `impl` 块提供（见对应模块的文件头说明）。
#[derive(Debug, Clone)]
pub struct TimephasedItem<T> {
    /// 起始时间。
    ///
    /// 对应 Java: `private LocalDateTime m_start;`
    m_start: Option<NaiveDateTime>,

    /// 总量。
    ///
    /// 对应 Java: `private T m_totalAmount;`
    m_total_amount: Option<T>,

    /// 结束时间。
    ///
    /// 对应 Java: `private LocalDateTime m_finish;`
    m_finish: Option<NaiveDateTime>,

    /// 每小时数量。
    ///
    /// 对应 Java: `private T m_amountPerHour;`
    m_amount_per_hour: Option<T>,
}

impl<T> TimephasedItem<T> {
    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法（Java 侧未显式声明构造方法）。
    pub fn new() -> Self {
        Self {
            m_start: None,
            m_total_amount: None,
            m_finish: None,
            m_amount_per_hour: None,
        }
    }
}

impl<T: Copy> TimephasedItem<T> {
    /// 取起始时间。
    ///
    /// 对应 Java: `public LocalDateTime getStart()`
    pub fn get_start(&self) -> Option<NaiveDateTime> {
        self.m_start
    }

    /// 设置起始时间。
    ///
    /// 对应 Java: `public void setStart(LocalDateTime start)`
    pub fn set_start(&mut self, start: Option<NaiveDateTime>) {
        self.m_start = start;
    }

    /// 取每小时数量。
    ///
    /// 对应 Java: `public T getAmountPerHour()`
    pub fn get_amount_per_hour(&self) -> Option<T> {
        self.m_amount_per_hour
    }

    /// 设置每小时数量。
    ///
    /// 对应 Java: `public void setAmountPerHour(T amountPerHour)`
    pub fn set_amount_per_hour(&mut self, amount_per_hour: Option<T>) {
        self.m_amount_per_hour = amount_per_hour;
    }

    /// 取总量。
    ///
    /// 对应 Java: `public T getTotalAmount()`
    pub fn get_total_amount(&self) -> Option<T> {
        self.m_total_amount
    }

    /// 设置总量。
    ///
    /// 对应 Java: `public void setTotalAmount(T totalAmount)`
    pub fn set_total_amount(&mut self, total_amount: Option<T>) {
        self.m_total_amount = total_amount;
    }

    /// 取结束时间。
    ///
    /// 对应 Java: `public LocalDateTime getFinish()`
    pub fn get_finish(&self) -> Option<NaiveDateTime> {
        self.m_finish
    }

    /// 设置结束时间。
    ///
    /// 对应 Java: `public void setFinish(LocalDateTime finish)`
    pub fn set_finish(&mut self, finish: Option<NaiveDateTime>) {
        self.m_finish = finish;
    }
}

impl<T> Default for TimephasedItem<T> {
    /// 对应 Java: 隐式默认构造方法，等价于 [`TimephasedItem::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl<T: PartialEq> PartialEq for TimephasedItem<T> {
    /// 对应 Java: `@SuppressWarnings("unchecked") @Override public boolean equals(Object o)`
    ///
    /// Java 的实现要求四个字段**全部非空且相等**才返回 `true`（见文件头说明：
    /// 该关系不满足自反性，因此不实现 `Eq`）。
    fn eq(&self, other: &Self) -> bool {
        self.m_start.is_some()
            && self.m_start == other.m_start
            && self.m_finish.is_some()
            && self.m_finish == other.m_finish
            && self.m_total_amount.is_some()
            && self.m_total_amount == other.m_total_amount
            && self.m_amount_per_hour.is_some()
            && self.m_amount_per_hour == other.m_amount_per_hour
    }
}

impl<T: Display> Display for TimephasedItem<T> {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java: `"[TimephasedItem start=" + m_start + " totalAmount=" + m_totalAmount + " finish=" + m_finish + " amountPerHour=" + m_amountPerHour + "]"`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[TimephasedItem start={} totalAmount={} finish={} amountPerHour={}]",
            render_date(self.m_start),
            render_value(self.m_total_amount.as_ref()),
            render_date(self.m_finish),
            render_value(self.m_amount_per_hour.as_ref())
        )
    }
}

/// 渲染日期时间（`None` 渲染为 Java 字符串拼接中的 `"null"`）。
fn render_date(value: Option<NaiveDateTime>) -> String {
    match value {
        Some(value) => value.format("%Y-%m-%dT%H:%M").to_string(),
        None => "null".to_owned(),
    }
}

/// 渲染载荷（`None` 渲染为 Java 字符串拼接中的 `"null"`）。
fn render_value<T: Display>(value: Option<&T>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duration::Duration;
    use crate::time_unit::TimeUnit;

    fn date(year: i32, month: u32, day: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|value| value.and_hms_opt(0, 0, 0))
            .expect("合法日期")
    }

    fn item() -> TimephasedItem<Duration> {
        let mut item = TimephasedItem::new();
        item.set_start(Some(date(2024, 5, 1)));
        item.set_finish(Some(date(2024, 5, 2)));
        item.set_total_amount(Some(Duration::get_instance(8.0, TimeUnit::Hours)));
        item.set_amount_per_hour(Some(Duration::get_instance(1.0, TimeUnit::Hours)));
        item
    }

    #[test]
    fn accessors_round_trip() {
        let mut item: TimephasedItem<Duration> = TimephasedItem::new();
        assert_eq!(item.get_start(), None);
        assert_eq!(item.get_finish(), None);
        assert_eq!(item.get_total_amount(), None);
        assert_eq!(item.get_amount_per_hour(), None);

        item.set_start(Some(date(2024, 5, 1)));
        item.set_finish(Some(date(2024, 5, 2)));
        item.set_total_amount(Some(Duration::get_instance(4.0, TimeUnit::Days)));
        item.set_amount_per_hour(Some(Duration::get_instance(2.0, TimeUnit::Hours)));

        assert_eq!(item.get_start(), Some(date(2024, 5, 1)));
        assert_eq!(item.get_finish(), Some(date(2024, 5, 2)));
        assert_eq!(
            item.get_total_amount().map(|value| value.get_duration()),
            Some(4.0)
        );
        assert_eq!(
            item.get_amount_per_hour().map(|value| value.get_units()),
            Some(TimeUnit::Hours)
        );
    }

    #[test]
    fn equality_follows_java_all_fields_non_null_rule() {
        assert_eq!(item(), item());

        // 任一字段为 null ⇒ 不相等（Java 的 equals 分支）
        let mut incomplete = item();
        incomplete.set_finish(None);
        assert_ne!(item(), incomplete);

        // 两个「空」实例也不相等（Java 不满足自反性）
        let empty: TimephasedItem<Duration> = TimephasedItem::new();
        let other: TimephasedItem<Duration> = TimephasedItem::new();
        assert_ne!(empty, other);

        // 取值不同 ⇒ 不相等
        let mut different = item();
        different.set_total_amount(Some(Duration::get_instance(9.0, TimeUnit::Hours)));
        assert_ne!(item(), different);
    }

    #[test]
    fn clone_copies_all_fields() {
        let source = item();
        let copy = source.clone();

        assert_eq!(copy.get_start(), source.get_start());
        assert_eq!(copy.get_finish(), source.get_finish());
        assert_eq!(
            copy.get_total_amount().map(|value| value.get_duration()),
            source.get_total_amount().map(|value| value.get_duration())
        );
    }

    #[test]
    fn display_matches_java_format() {
        assert_eq!(
            item().to_string(),
            "[TimephasedItem start=2024-05-01T00:00 totalAmount=8h finish=2024-05-02T00:00 amountPerHour=1h]"
        );

        let empty: TimephasedItem<Duration> = TimephasedItem::new();
        assert_eq!(
            empty.to_string(),
            "[TimephasedItem start=null totalAmount=null finish=null amountPerHour=null]"
        );
    }

    #[test]
    fn default_matches_new() {
        let item: TimephasedItem<Duration> = Default::default();
        assert_eq!(item.get_start(), None);
    }

    #[test]
    fn numeric_payload_is_supported() {
        let mut item: TimephasedItem<f64> = TimephasedItem::new();
        item.set_total_amount(Some(12.5));
        item.set_amount_per_hour(Some(2.5));

        assert_eq!(item.get_total_amount(), Some(12.5));
        assert_eq!(item.get_amount_per_hour(), Some(2.5));
    }
}
